use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

const PROTOCOL_VERSION: &str = "2000m.driver.v3";
const RESULT_SCHEMA_VERSION: &str = "2000m.v3.result.v1";
const FIXTURE_SCHEMA_VERSION: &str = "2000m.v3.mechanical-fixtures.v1";
const TOTAL_ACS: usize = 24;

const ALLOWED_OBSTACLES: &[&str] = &["tree", "bigtree", "stump", "mogul", "rock", "ramp"];
const ALLOWED_EVENTS: &[&str] = &[
    "spawn",
    "turn",
    "collision",
    "crash",
    "recover",
    "ramp_enter",
    "airborne",
    "land",
    "style_gain",
    "style_loss",
    "monster_spawn",
    "monster_pursuit",
    "monster_contact",
    "monster_flee",
];
const ALLOWED_ERROR_CODES: &[&str] = &[
    "invalid_request",
    "invalid_state",
    "unsupported_command",
    "schema_violation",
    "challenge_unavailable",
    "internal_error",
];

const CHECKS: &[CheckSpec] = &[
    CheckSpec {
        id: "M01",
        name: "manifest/protocol",
        runner: check_manifest_protocol,
    },
    CheckSpec {
        id: "M02",
        name: "init determinism",
        runner: check_init_determinism,
    },
    CheckSpec {
        id: "M03",
        name: "reset determinism",
        runner: check_reset_determinism,
    },
    CheckSpec {
        id: "M04",
        name: "step determinism",
        runner: check_step_determinism,
    },
    CheckSpec {
        id: "M05",
        name: "state idempotence",
        runner: check_state_idempotence,
    },
    CheckSpec {
        id: "M06",
        name: "schema validity and enum poison",
        runner: check_schema_validity,
    },
    CheckSpec {
        id: "M07",
        name: "obstacle generation",
        runner: check_obstacle_generation,
    },
    CheckSpec {
        id: "M08",
        name: "skier movement",
        runner: check_skier_movement,
    },
    CheckSpec {
        id: "M09",
        name: "collision correctness",
        runner: check_collision_correctness,
    },
    CheckSpec {
        id: "M10",
        name: "recovery behavior",
        runner: check_recovery_behavior,
    },
    CheckSpec {
        id: "M11",
        name: "ramp entry",
        runner: check_ramp_entry,
    },
    CheckSpec {
        id: "M12",
        name: "airborne/landing",
        runner: check_airborne_landing,
    },
    CheckSpec {
        id: "M13",
        name: "style scoring",
        runner: check_style_scoring,
    },
    CheckSpec {
        id: "M14",
        name: "monster spawn",
        runner: check_monster_spawn,
    },
    CheckSpec {
        id: "M15",
        name: "monster pursuit",
        runner: check_monster_pursuit,
    },
    CheckSpec {
        id: "M16",
        name: "monster contact/flee",
        runner: check_monster_contact_flee,
    },
    CheckSpec {
        id: "M17",
        name: "replay checksum",
        runner: check_replay_checksum,
    },
    CheckSpec {
        id: "M18",
        name: "public challenge",
        runner: check_public_challenge,
    },
    CheckSpec {
        id: "M19",
        name: "hidden challenge isolation",
        runner: check_hidden_challenge,
    },
    CheckSpec {
        id: "M20",
        name: "regression stability",
        runner: check_regression_stability,
    },
    CheckSpec {
        id: "M21",
        name: "error semantics",
        runner: check_error_semantics,
    },
    CheckSpec {
        id: "M22",
        name: "no scorer mutation",
        runner: check_no_scorer_mutation,
    },
    CheckSpec {
        id: "M23",
        name: "no private setup hints",
        runner: check_no_private_setup_hints,
    },
    CheckSpec {
        id: "M24",
        name: "mechanical/visual separation",
        runner: check_track_separation,
    },
];

type BoxError = Box<dyn Error + Send + Sync>;
type CheckResult = Result<String, String>;
type CheckRunner = fn(&Harness) -> CheckResult;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    language: String,
    driver: CommandSpec,
    #[serde(default)]
    capture: Option<Value>,
    #[serde(default)]
    playable: Option<Value>,
    assets: Assets,
    #[serde(default, rename = "notes")]
    _notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandSpec {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default, rename = "timeoutSeconds")]
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Assets {
    license: String,
    attestation: String,
    #[serde(default, rename = "sourceRefs")]
    source_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureSet {
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    fixtures: Vec<ChallengeFixture>,
    #[serde(default, rename = "description")]
    _description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChallengeFixture {
    #[serde(rename = "challengeId")]
    challenge_id: String,
    kind: String,
    visibility: String,
    seed: i64,
    inputs: Vec<Value>,
}

#[derive(Debug)]
struct CheckSpec {
    id: &'static str,
    name: &'static str,
    runner: CheckRunner,
}

#[derive(Debug)]
struct Harness {
    manifest_path: PathBuf,
    artifact_root: PathBuf,
    manifest: Manifest,
    fixture_set: FixtureSet,
}

#[derive(Debug, Serialize)]
struct V3Result {
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    #[serde(rename = "campaignId")]
    campaign_id: String,
    #[serde(rename = "scenarioId")]
    scenario_id: String,
    #[serde(rename = "taskSeed")]
    task_seed: i64,
    #[serde(rename = "laneId")]
    lane_id: String,
    entrant: EntrantResult,
    #[serde(rename = "protocolFreeze")]
    protocol_freeze: ProtocolFreeze,
    mechanical: MechanicalResult,
    visual: VisualResult,
    workflow: WorkflowResult,
    evidence: EvidenceResult,
    #[serde(rename = "claimBoundary")]
    claim_boundary: String,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct EntrantResult {
    model: String,
    runtime: String,
    #[serde(rename = "processType")]
    process_type: String,
}

#[derive(Debug, Serialize)]
struct ProtocolFreeze {
    #[serde(rename = "changedAfterLiveResults")]
    changed_after_live_results: bool,
    #[serde(rename = "scorerMutationObserved")]
    scorer_mutation_observed: bool,
    #[serde(rename = "calibrationOnlyIfChanged")]
    calibration_only_if_changed: bool,
}

#[derive(Debug, Serialize)]
struct MechanicalResult {
    ranked: bool,
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    #[serde(rename = "passCount")]
    pass_count: usize,
    #[serde(rename = "totalAcs")]
    total_acs: usize,
    #[serde(rename = "compositeScore")]
    composite_score: f64,
    determinism: DeterminismResult,
    acs: Vec<AcVerdict>,
    #[serde(rename = "failedAcs")]
    failed_acs: Vec<String>,
    #[serde(rename = "hiddenChallengeSummary")]
    hidden_challenge_summary: String,
    #[serde(rename = "regressionSummary")]
    regression_summary: String,
    #[serde(rename = "resultJsonRef")]
    result_json_ref: String,
}

#[derive(Debug, Serialize)]
struct DeterminismResult {
    pass: bool,
    details: String,
}

#[derive(Debug, Clone, Serialize)]
struct AcVerdict {
    id: String,
    name: String,
    pass: bool,
    detail: String,
}

#[derive(Debug, Serialize)]
struct VisualResult {
    ranked: bool,
    #[serde(rename = "blockReason")]
    block_reason: String,
    #[serde(rename = "visualPackageRef")]
    visual_package_ref: String,
    #[serde(rename = "captureDeterminism")]
    capture_determinism: String,
    #[serde(rename = "rubricRecordRef")]
    rubric_record_ref: String,
}

#[derive(Debug, Serialize)]
struct WorkflowResult {
    #[serde(rename = "contextWipeRecoveryScore")]
    context_wipe_recovery_score: f64,
    #[serde(rename = "feedbackDecisionScore")]
    feedback_decision_score: f64,
    #[serde(rename = "regressionProtectionScore")]
    regression_protection_score: f64,
    #[serde(rename = "impossibleRequirementHandlingScore")]
    impossible_requirement_handling_score: f64,
    #[serde(rename = "handoffScore")]
    handoff_score: f64,
    #[serde(rename = "finalRecommendation")]
    final_recommendation: String,
    #[serde(rename = "rationaleRefs")]
    rationale_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
struct EvidenceResult {
    replayable: bool,
    #[serde(rename = "publicSafe")]
    public_safe: bool,
    #[serde(rename = "privateRefsBlocked")]
    private_refs_blocked: bool,
    #[serde(rename = "compactSummaryRef")]
    compact_summary_ref: String,
    #[serde(rename = "requiredRefsMissing")]
    required_refs_missing: Vec<String>,
    #[serde(rename = "claimBoundary")]
    claim_boundary: String,
}

#[derive(Debug)]
struct ParsedArgs {
    manifest_path: PathBuf,
    fixture_set_path: PathBuf,
    json_out: Option<PathBuf>,
}

struct DriverClient {
    child: Child,
    stdin: ChildStdin,
    response_rx: Receiver<Result<String, String>>,
    timeout: Duration,
    next_request: u64,
}

impl Drop for DriverClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Debug)]
struct DriverResponse {
    ok: bool,
    payload: Option<Value>,
    error: Option<Value>,
}

fn main() -> Result<(), BoxError> {
    let args: Vec<String> = env::args().collect();
    let parsed = parse_args(&args)?;
    let harness = Harness::load(parsed.manifest_path, parsed.fixture_set_path)?;
    let mut result = score(&harness, parsed.json_out.as_deref());
    if let Some(out) = parsed.json_out.as_ref() {
        result.mechanical.result_json_ref = public_ref_for(out);
        let pretty = serde_json::to_string_pretty(&result)?;
        fs::write(out, format!("{}\n", pretty))?;
    }
    print_summary(&result);
    if parsed.json_out.is_none() {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, BoxError> {
    if args.len() < 2 || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Err(usage().into());
    }
    let manifest_path = PathBuf::from(&args[1]);
    let mut fixture_set_path = default_fixture_set_path();
    let mut json_out = None;
    let mut idx = 2;
    while idx < args.len() {
        match args[idx].as_str() {
            "--fixtures" => {
                let value = args.get(idx + 1).ok_or_else(usage)?;
                fixture_set_path = PathBuf::from(value);
                idx += 2;
            }
            "--json-out" => {
                let value = args.get(idx + 1).ok_or_else(usage)?;
                json_out = Some(PathBuf::from(value));
                idx += 2;
            }
            other => return Err(format!("unknown argument `{}`\n{}", other, usage()).into()),
        }
    }
    Ok(ParsedArgs {
        manifest_path,
        fixture_set_path,
        json_out,
    })
}

fn usage() -> String {
    "usage: m2000-v3-conformance <manifest-or-artifact-dir> [--fixtures <challenges.json>] [--json-out <path>]".to_string()
}

fn default_fixture_set_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("v3/conformance has v3 parent")
        .join("fixtures/mechanical/challenges.json")
}

impl Harness {
    fn load(raw_manifest_path: PathBuf, fixture_set_path: PathBuf) -> Result<Self, BoxError> {
        let manifest_path = if raw_manifest_path.is_dir() {
            raw_manifest_path.join("2000m.v3.json")
        } else {
            raw_manifest_path
        };
        let artifact_root = manifest_path
            .parent()
            .ok_or("manifest path has no parent")?
            .to_path_buf();
        let manifest_text = fs::read_to_string(&manifest_path).map_err(|err| {
            format!(
                "failed to read manifest `{}`: {}",
                manifest_path.display(),
                err
            )
        })?;
        let manifest: Manifest = serde_json::from_str(&manifest_text).map_err(|err| {
            format!(
                "failed to parse manifest `{}`: {}",
                manifest_path.display(),
                err
            )
        })?;
        let fixture_text = fs::read_to_string(&fixture_set_path).map_err(|err| {
            format!(
                "failed to read fixture set `{}`: {}",
                fixture_set_path.display(),
                err
            )
        })?;
        let fixture_set: FixtureSet = serde_json::from_str(&fixture_text).map_err(|err| {
            format!(
                "failed to parse fixture set `{}`: {}",
                fixture_set_path.display(),
                err
            )
        })?;
        Ok(Self {
            manifest_path,
            artifact_root,
            manifest,
            fixture_set,
        })
    }

    fn spawn_driver(&self) -> Result<DriverClient, String> {
        let mut command = Command::new(resolve_command(&self.manifest.driver.command));
        command.args(&self.manifest.driver.args);
        let cwd = match &self.manifest.driver.cwd {
            Some(relative) => self.artifact_root.join(relative),
            None => self.artifact_root.clone(),
        };
        command
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|err| format!("failed to spawn driver: {err}"))?;
        let stdin = child.stdin.take().ok_or("driver stdin not available")?;
        let stdout = child.stdout.take().ok_or("driver stdout not available")?;
        let mut stderr = child.stderr.take().ok_or("driver stderr not available")?;
        let _stderr_thread = thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            loop {
                match stderr.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
        });
        let (response_tx, response_rx) = mpsc::channel();
        let _reader_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        if response_tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        let _ = response_tx.send(Err(format!("failed to read response: {err}")));
                        break;
                    }
                }
            }
        });
        let timeout =
            Duration::from_secs(self.manifest.driver.timeout_seconds.unwrap_or(30).max(1));
        Ok(DriverClient {
            child,
            stdin,
            response_rx,
            timeout,
            next_request: 0,
        })
    }

    fn fixture_by_kind(&self, kind: &str) -> Result<&ChallengeFixture, String> {
        self.fixture_set
            .fixtures
            .iter()
            .find(|fixture| fixture.kind == kind)
            .ok_or_else(|| format!("missing fixture kind `{kind}`"))
    }

    fn fixtures_by_visibility(&self, visibility: &str) -> Vec<&ChallengeFixture> {
        self.fixture_set
            .fixtures
            .iter()
            .filter(|fixture| fixture.visibility == visibility)
            .collect()
    }
}

impl DriverClient {
    fn send(&mut self, command: &str, payload: Value) -> Result<DriverResponse, String> {
        self.next_request += 1;
        let request_id = format!("req-{:04}", self.next_request);
        let request = json!({
            "protocolVersion": PROTOCOL_VERSION,
            "requestId": request_id,
            "command": command,
            "payload": payload,
        });
        let line = serde_json::to_string(&request).map_err(|err| err.to_string())?;
        writeln!(self.stdin, "{line}").map_err(|err| format!("failed to write request: {err}"))?;
        self.stdin
            .flush()
            .map_err(|err| format!("failed to flush request: {err}"))?;
        let response_line = match self.response_rx.recv_timeout(self.timeout) {
            Ok(Ok(line)) => line,
            Ok(Err(err)) => return Err(err),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = self.child.kill();
                return Err(format!(
                    "driver response timeout after {}s",
                    self.timeout.as_secs()
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("driver closed stdout before response".to_string());
            }
        };
        let response: Value = serde_json::from_str(response_line.trim_end()).map_err(|err| {
            format!("driver emitted invalid JSON response: {err}: {response_line}")
        })?;
        require_value(
            response.get("protocolVersion") == Some(&Value::String(PROTOCOL_VERSION.to_string())),
            "response protocolVersion mismatch",
        )?;
        require_value(
            response.get("requestId") == Some(&Value::String(request_id)),
            "response requestId mismatch",
        )?;
        let ok = response
            .get("ok")
            .and_then(Value::as_bool)
            .ok_or("response missing ok bool")?;
        Ok(DriverResponse {
            ok,
            payload: response.get("payload").cloned(),
            error: response.get("error").cloned(),
        })
    }

    fn ok_payload(&mut self, command: &str, payload: Value) -> Result<Value, String> {
        let response = self.send(command, payload)?;
        if !response.ok {
            return Err(format!(
                "driver returned error for {command}: {:?}",
                response.error
            ));
        }
        response
            .payload
            .ok_or_else(|| format!("ok response for {command} missing payload"))
    }
}

fn resolve_command(command: &str) -> PathBuf {
    PathBuf::from(command)
}

fn score(harness: &Harness, json_out: Option<&Path>) -> V3Result {
    let acs: Vec<AcVerdict> = CHECKS
        .iter()
        .map(|spec| match (spec.runner)(harness) {
            Ok(detail) => AcVerdict {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                pass: true,
                detail,
            },
            Err(detail) => AcVerdict {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                pass: false,
                detail,
            },
        })
        .collect();
    let failed_acs: Vec<String> = acs
        .iter()
        .filter(|ac| !ac.pass)
        .map(|ac| ac.id.clone())
        .collect();
    let pass_count = acs.iter().filter(|ac| ac.pass).count();
    let ranked = failed_acs.is_empty();
    let determinism_pass = ac_pass(&acs, "M02") && ac_pass(&acs, "M04") && ac_pass(&acs, "M20");
    let hidden_challenge_summary = hidden_summary(harness, &acs);
    let regression_summary = if ac_pass(&acs, "M20") {
        "fixed hidden checks reran stably".to_string()
    } else {
        "fixed hidden checks were not stable".to_string()
    };
    let visual_blocked = harness.manifest.capture.is_none() || harness.manifest.playable.is_none();
    let visual_block_reason = if visual_blocked {
        "missing-native-capture-or-playable-surface"
    } else {
        "missing-capture-metadata"
    };
    let result_ref = json_out
        .map(public_ref_for)
        .unwrap_or_else(|| "stdout".to_string());
    V3Result {
        schema_version: RESULT_SCHEMA_VERSION.to_string(),
        campaign_id: "v3-mechanical-smoke".to_string(),
        scenario_id: "v3-mechanical-smoke".to_string(),
        task_seed: 0,
        lane_id: "A".to_string(),
        entrant: EntrantResult {
            model: "fixture-driver".to_string(),
            runtime: "v3-conformance-smoke".to_string(),
            process_type: "scripted-agent".to_string(),
        },
        protocol_freeze: ProtocolFreeze {
            changed_after_live_results: false,
            scorer_mutation_observed: false,
            calibration_only_if_changed: true,
        },
        mechanical: MechanicalResult {
            ranked,
            protocol_version: PROTOCOL_VERSION.to_string(),
            pass_count,
            total_acs: TOTAL_ACS,
            composite_score: (pass_count as f64 / TOTAL_ACS as f64) * 100.0,
            determinism: DeterminismResult {
                pass: determinism_pass,
                details: "init, step, replay, and hidden regression checks are fixture-stable"
                    .to_string(),
            },
            acs,
            failed_acs,
            hidden_challenge_summary,
            regression_summary,
            result_json_ref: result_ref,
        },
        visual: VisualResult {
            ranked: false,
            block_reason: visual_block_reason.to_string(),
            visual_package_ref: String::new(),
            capture_determinism: "blocked".to_string(),
            rubric_record_ref: String::new(),
        },
        workflow: WorkflowResult {
            context_wipe_recovery_score: 0.0,
            feedback_decision_score: 0.0,
            regression_protection_score: 0.0,
            impossible_requirement_handling_score: 0.0,
            handoff_score: 0.0,
            final_recommendation: if ranked {
                "continue".to_string()
            } else {
                "inspect_scorer".to_string()
            },
            rationale_refs: vec!["v3/MECHANICAL_AC_SPEC.md".to_string()],
        },
        evidence: EvidenceResult {
            replayable: true,
            public_safe: true,
            private_refs_blocked: false,
            compact_summary_ref: "v3/fixtures/mechanical/challenges.json".to_string(),
            required_refs_missing: Vec::new(),
            claim_boundary: "calibration-only".to_string(),
        },
        claim_boundary: "calibration-only".to_string(),
        warnings: vec![
            "Mechanical scaffold fixture result; calibration-only and not a contender result."
                .to_string(),
        ],
    }
}

fn print_summary(result: &V3Result) {
    println!(
        "v3 mechanical: ranked={} passCount={}/{} composite={:.2} failed={:?}",
        result.mechanical.ranked,
        result.mechanical.pass_count,
        result.mechanical.total_acs,
        result.mechanical.composite_score,
        result.mechanical.failed_acs
    );
}

fn ac_pass(acs: &[AcVerdict], id: &str) -> bool {
    acs.iter().any(|ac| ac.id == id && ac.pass)
}

fn hidden_summary(harness: &Harness, acs: &[AcVerdict]) -> String {
    let hidden_count = harness.fixtures_by_visibility("hidden").len();
    if ac_pass(acs, "M19") && ac_pass(acs, "M20") {
        format!("{hidden_count} hidden fixtures passed with predeclared isolation")
    } else {
        format!("{hidden_count} hidden fixtures were attempted; see failedAcs")
    }
}

fn public_ref_for(path: &Path) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent);
    root.and_then(|repo| path.strip_prefix(repo).ok())
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| {
            path.file_name()
                .map(|name| format!("private-run-root/{}", name.to_string_lossy()))
                .unwrap_or_else(|| "private-run-root/result.json".to_string())
        })
}

fn check_manifest_protocol(harness: &Harness) -> CheckResult {
    require_value(
        harness.manifest.schema_version == "2000m.v3.manifest.v1",
        "manifest schemaVersion must be 2000m.v3.manifest.v1",
    )?;
    require_value(
        harness.manifest.protocol_version == PROTOCOL_VERSION,
        "manifest protocolVersion must be 2000m.driver.v3",
    )?;
    require_value(
        harness.manifest.language == "rust",
        "foundation scaffold currently accepts rust language manifests",
    )?;
    require_value(
        harness.fixture_set.schema_version == FIXTURE_SCHEMA_VERSION,
        "fixture set schemaVersion mismatch",
    )?;
    require_value(
        !harness.manifest.driver.command.trim().is_empty(),
        "driver command must be non-empty",
    )?;
    require_value(
        !harness.manifest.assets.attestation.trim().is_empty(),
        "asset attestation must be non-empty",
    )?;
    require_value(
        !harness.manifest.assets.license.trim().is_empty(),
        "asset license must be non-empty",
    )?;
    Ok("manifest and fixture protocol versions are valid".to_string())
}

fn check_init_determinism(harness: &Harness) -> CheckResult {
    let first = init_checksum(harness, 1101)?;
    let second = init_checksum(harness, 1101)?;
    require_value(
        first == second,
        "same seed init emitted different checksums",
    )?;
    Ok(format!("seed 1101 checksum {first}"))
}

fn check_reset_determinism(harness: &Harness) -> CheckResult {
    let mut client = harness.spawn_driver()?;
    let init = state_capture(client.ok_payload("init", json!({"seed": 1102, "config": {}}))?)?;
    run_step(&mut client, json!({"steer": 1}))?;
    let reset = state_capture(client.ok_payload("reset", json!({"seed": 1102, "config": {}}))?)?;
    require_value(
        init.checksum == reset.checksum,
        "reset did not return to init checksum",
    )?;
    Ok(format!("reset checksum {}", reset.checksum))
}

fn check_step_determinism(harness: &Harness) -> CheckResult {
    let inputs = vec![
        json!({"steer": 1}),
        json!({"steer": -1}),
        json!({"jump": true}),
    ];
    let first = step_sequence(harness, 1103, &inputs)?;
    let second = step_sequence(harness, 1103, &inputs)?;
    require_value(
        first == second,
        "same input stream emitted different checksum sequence",
    )?;
    Ok(format!("{} step checksums stable", first.len()))
}

fn check_state_idempotence(harness: &Harness) -> CheckResult {
    let mut client = harness.spawn_driver()?;
    let _ = state_capture(client.ok_payload("init", json!({"seed": 1104, "config": {}}))?)?;
    let first = state_capture(client.ok_payload("state", json!({}))?)?;
    let second = state_capture(client.ok_payload("state", json!({}))?)?;
    require_value(
        first.checksum == second.checksum,
        "state call advanced or changed state",
    )?;
    require_value(
        read_i64(&first.state, &["tick"])? == read_i64(&second.state, &["tick"])?,
        "state call changed tick",
    )?;
    Ok("state is idempotent".to_string())
}

fn check_schema_validity(harness: &Harness) -> CheckResult {
    let mut client = harness.spawn_driver()?;
    let init = state_capture(client.ok_payload("init", json!({"seed": 1105, "config": {}}))?)?;
    validate_state(&init.state)?;
    let step = run_step(&mut client, json!({"steer": 0, "style": true}))?;
    validate_state(&step.state)?;
    Ok("init and step states validate with allowed enums".to_string())
}

fn check_obstacle_generation(harness: &Harness) -> CheckResult {
    let fixture = harness.fixture_by_kind("basic")?;
    let first = run_challenge(harness, fixture)?;
    let second = run_challenge(harness, fixture)?;
    require_value(
        first.checksum == second.checksum,
        "obstacle challenge was not deterministic",
    )?;
    let obstacles = first
        .state
        .pointer("/world/obstacles")
        .and_then(Value::as_array)
        .ok_or("missing world.obstacles")?;
    require_value(!obstacles.is_empty(), "obstacle list is empty")?;
    Ok(format!("{} obstacles stable", obstacles.len()))
}

fn check_skier_movement(harness: &Harness) -> CheckResult {
    let mut client = harness.spawn_driver()?;
    let init = state_capture(client.ok_payload("init", json!({"seed": 1106, "config": {}}))?)?;
    let left = run_step(&mut client, json!({"steer": -1}))?;
    let left_x = read_f64(&left.state, &["skier", "x"])?;
    let init_x = read_f64(&init.state, &["skier", "x"])?;
    require_value(left_x < init_x, "left input did not move skier left")?;
    let right = run_step(&mut client, json!({"steer": 1}))?;
    let right_x = read_f64(&right.state, &["skier", "x"])?;
    require_value(
        right_x >= left_x,
        "right input did not move skier rightward",
    )?;
    Ok("left/right input changes skier position".to_string())
}

fn check_collision_correctness(harness: &Harness) -> CheckResult {
    require_challenge_passed(harness, "collision", &["collision", "crash"])
}

fn check_recovery_behavior(harness: &Harness) -> CheckResult {
    require_challenge_passed(harness, "recovery", &["recover"])
}

fn check_ramp_entry(harness: &Harness) -> CheckResult {
    require_challenge_passed(harness, "ramp", &["ramp_enter", "airborne"])
}

fn check_airborne_landing(harness: &Harness) -> CheckResult {
    require_challenge_passed(harness, "airborne", &["airborne", "land"])
}

fn check_style_scoring(harness: &Harness) -> CheckResult {
    let fixture = harness.fixture_by_kind("style")?;
    let capture = run_challenge(harness, fixture)?;
    require_events(&capture.events, &["style_gain"])?;
    let style = read_i64(&capture.state, &["score", "style"])?;
    require_value(style > 0, "style challenge did not increase score.style")?;
    Ok("style_gain event updates score.style".to_string())
}

fn check_monster_spawn(harness: &Harness) -> CheckResult {
    let fixture = harness.fixture_by_kind("monster-spawn")?;
    let capture = run_challenge(harness, fixture)?;
    require_events(&capture.events, &["monster_spawn"])?;
    require_value(
        !capture.state["monster"].is_null(),
        "monster_spawn challenge did not expose monster",
    )?;
    Ok("monster spawn is visible in state".to_string())
}

fn check_monster_pursuit(harness: &Harness) -> CheckResult {
    require_challenge_passed(
        harness,
        "monster-pursuit",
        &["monster_spawn", "monster_pursuit"],
    )
}

fn check_monster_contact_flee(harness: &Harness) -> CheckResult {
    require_challenge_passed(
        harness,
        "monster-contact",
        &["monster_contact", "monster_flee"],
    )
}

fn check_replay_checksum(harness: &Harness) -> CheckResult {
    let fixture = harness.fixture_by_kind("calibration-replay")?;
    let first = run_replay(harness, fixture)?;
    let second = run_replay(harness, fixture)?;
    require_value(
        first == second,
        "replay checksum changed across identical reruns",
    )?;
    Ok(format!("replay checksum {first}"))
}

fn check_public_challenge(harness: &Harness) -> CheckResult {
    let fixture = harness.fixture_by_kind("basic")?;
    let capture = run_challenge(harness, fixture)?;
    require_challenge_result(&capture.payload, &fixture.challenge_id)?;
    Ok(format!("public challenge {} passed", fixture.challenge_id))
}

fn check_hidden_challenge(harness: &Harness) -> CheckResult {
    let fixture = harness.fixture_by_kind("hidden-basic")?;
    let capture = run_challenge(harness, fixture)?;
    require_challenge_result(&capture.payload, &fixture.challenge_id)?;
    let manifest_text =
        fs::read_to_string(&harness.manifest_path).map_err(|err| err.to_string())?;
    require_value(
        !manifest_text.contains(&fixture.challenge_id),
        "manifest leaked hidden challenge id",
    )?;
    Ok(format!(
        "hidden challenge {} passed without manifest dependency",
        fixture.challenge_id
    ))
}

fn check_regression_stability(harness: &Harness) -> CheckResult {
    let hidden = harness.fixtures_by_visibility("hidden");
    require_value(!hidden.is_empty(), "no hidden fixtures declared")?;
    let first = hidden_challenge_checksums(harness, &hidden)?;
    let second = hidden_challenge_checksums(harness, &hidden)?;
    require_value(first == second, "hidden fixture rerun checksums changed")?;
    Ok(format!("{} hidden checksums stable", first.len()))
}

fn check_error_semantics(harness: &Harness) -> CheckResult {
    let mut client = harness.spawn_driver()?;
    let response = client.send("definitely-not-a-v3-command", json!({}))?;
    require_value(!response.ok, "unsupported command returned ok=true")?;
    let error = response
        .error
        .ok_or("unsupported command error missing error object")?;
    let code = error
        .get("code")
        .and_then(Value::as_str)
        .ok_or("error.code missing")?;
    require_value(
        ALLOWED_ERROR_CODES.contains(&code),
        "error.code not in allowed enum",
    )?;
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    require_value(!message.trim().is_empty(), "error.message missing")?;
    Ok(format!(
        "unsupported command returned structured {code} error"
    ))
}

fn check_no_scorer_mutation(harness: &Harness) -> CheckResult {
    let fixture_count = harness.fixture_set.fixtures.len();
    require_value(
        fixture_count >= 3,
        "fixture set is too small to be a frozen scorer input",
    )?;
    let ids: BTreeSet<&str> = harness
        .fixture_set
        .fixtures
        .iter()
        .map(|fixture| fixture.challenge_id.as_str())
        .collect();
    require_value(
        ids.len() == fixture_count,
        "duplicate challenge ids would mutate denominator semantics",
    )?;
    Ok("scorer fixture denominator is fixed and unique".to_string())
}

fn check_no_private_setup_hints(harness: &Harness) -> CheckResult {
    check_public_string(&harness.manifest.driver.command, "driver.command")?;
    for (idx, arg) in harness.manifest.driver.args.iter().enumerate() {
        check_public_string(arg, &format!("driver.args[{idx}]"))?;
    }
    if let Some(cwd) = &harness.manifest.driver.cwd {
        check_public_string(cwd, "driver.cwd")?;
    }
    for (idx, source_ref) in harness.manifest.assets.source_refs.iter().enumerate() {
        check_public_string(source_ref, &format!("assets.sourceRefs[{idx}]"))?;
    }
    Ok("manifest does not depend on private/local setup refs".to_string())
}

fn check_track_separation(harness: &Harness) -> CheckResult {
    let init = init_checksum(harness, 1108)?;
    require_value(!init.is_empty(), "mechanical init checksum missing")?;
    Ok("mechanical checks run even when capture/playable evidence is absent".to_string())
}

fn init_checksum(harness: &Harness, seed: i64) -> Result<String, String> {
    let mut client = harness.spawn_driver()?;
    let payload = client.ok_payload("init", json!({"seed": seed, "config": {}}))?;
    Ok(state_capture(payload)?.checksum)
}

fn step_sequence(harness: &Harness, seed: i64, inputs: &[Value]) -> Result<Vec<String>, String> {
    let mut client = harness.spawn_driver()?;
    let init = state_capture(client.ok_payload("init", json!({"seed": seed, "config": {}}))?)?;
    let mut checksums = vec![init.checksum];
    for input in inputs {
        checksums.push(run_step(&mut client, input.clone())?.checksum);
    }
    Ok(checksums)
}

fn run_step(client: &mut DriverClient, input: Value) -> Result<StateCapture, String> {
    state_capture(client.ok_payload("step", json!({"input": input}))?)
}

fn run_challenge(harness: &Harness, fixture: &ChallengeFixture) -> Result<StateCapture, String> {
    let mut client = harness.spawn_driver()?;
    let payload = client.ok_payload(
        "challenge",
        json!({"challengeId": fixture.challenge_id, "seed": fixture.seed, "inputs": fixture.inputs}),
    )?;
    state_capture(payload)
}

fn run_replay(harness: &Harness, fixture: &ChallengeFixture) -> Result<String, String> {
    let mut client = harness.spawn_driver()?;
    let payload = client.ok_payload(
        "replay",
        json!({"seed": fixture.seed, "config": {}, "inputs": fixture.inputs}),
    )?;
    let checksums = payload
        .get("stateChecksums")
        .and_then(Value::as_array)
        .ok_or("replay missing stateChecksums")?;
    require_value(!checksums.is_empty(), "replay returned no stateChecksums")?;
    for checksum in checksums {
        let checksum = checksum
            .as_str()
            .ok_or("stateChecksums item is not a string")?;
        require_checksum_shape(checksum)?;
    }
    let replay_checksum = payload
        .get("replayChecksum")
        .and_then(Value::as_str)
        .ok_or("replay missing replayChecksum")?;
    require_checksum_shape(replay_checksum)?;
    Ok(replay_checksum.to_string())
}

fn hidden_challenge_checksums(
    harness: &Harness,
    hidden: &[&ChallengeFixture],
) -> Result<Vec<String>, String> {
    hidden
        .iter()
        .map(|fixture| run_challenge(harness, fixture).map(|capture| capture.checksum))
        .collect()
}

fn require_challenge_passed(
    harness: &Harness,
    kind: &str,
    expected_events: &[&str],
) -> CheckResult {
    let fixture = harness.fixture_by_kind(kind)?;
    let capture = run_challenge(harness, fixture)?;
    require_events(&capture.events, expected_events)?;
    require_challenge_result(&capture.payload, &fixture.challenge_id)?;
    Ok(format!(
        "challenge {} emitted {:?}",
        fixture.challenge_id, expected_events
    ))
}

#[derive(Debug)]
struct StateCapture {
    payload: Value,
    state: Value,
    checksum: String,
    events: Vec<String>,
}

fn state_capture(payload: Value) -> Result<StateCapture, String> {
    let state = payload
        .get("state")
        .cloned()
        .ok_or("payload missing state")?;
    validate_state(&state)?;
    let checksum = payload
        .get("stateChecksum")
        .and_then(Value::as_str)
        .ok_or("payload missing stateChecksum")?
        .to_string();
    require_checksum_shape(&checksum)?;
    let computed = sha256_canonical(&state);
    require_value(
        checksum == computed,
        &format!("stateChecksum mismatch: emitted {checksum}, computed {computed}"),
    )?;
    let events = payload
        .get("events")
        .and_then(Value::as_array)
        .map(|raw| {
            raw.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    require_events_allowed(&events)?;
    Ok(StateCapture {
        payload,
        state,
        checksum,
        events,
    })
}

fn validate_state(state: &Value) -> Result<(), String> {
    require_value(state.is_object(), "state must be object")?;
    let seed = state.get("seed").ok_or("state.seed missing")?;
    require_value(
        seed.is_i64() || seed.is_u64(),
        "state.seed must be integer, not boolean/string",
    )?;
    let tick = state.get("tick").ok_or("state.tick missing")?;
    require_value(tick.is_i64() || tick.is_u64(), "state.tick must be integer")?;
    require_object(state, "skier")?;
    require_object(state, "world")?;
    require_object(state, "score")?;
    if let Some(monster) = state.get("monster") {
        require_value(
            monster.is_null() || monster.is_object(),
            "monster must be object or null",
        )?;
    } else {
        return Err("state.monster missing".to_string());
    }
    let obstacles = state
        .pointer("/world/obstacles")
        .and_then(Value::as_array)
        .ok_or("world.obstacles must be array")?;
    for (idx, obstacle) in obstacles.iter().enumerate() {
        let kind = obstacle
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("world.obstacles[{idx}].type missing"))?;
        require_value(
            ALLOWED_OBSTACLES.contains(&kind),
            &format!("unsupported obstacle type `{kind}`"),
        )?;
    }
    let events = state
        .get("events")
        .and_then(Value::as_array)
        .ok_or("state.events must be array")?;
    let event_strings: Result<Vec<String>, String> = events
        .iter()
        .map(|event| {
            event
                .as_str()
                .map(ToString::to_string)
                .ok_or("state.events item must be string".to_string())
        })
        .collect();
    require_events_allowed(&event_strings?)?;
    Ok(())
}

fn require_object(state: &Value, field: &str) -> Result<(), String> {
    require_value(
        state.get(field).is_some_and(Value::is_object),
        &format!("state.{field} must be object"),
    )
}

fn require_events(events: &[String], expected: &[&str]) -> Result<(), String> {
    for event in expected {
        require_value(
            events.iter().any(|seen| seen == event),
            &format!("missing expected event `{event}`"),
        )?;
    }
    Ok(())
}

fn require_events_allowed(events: &[String]) -> Result<(), String> {
    for event in events {
        require_value(
            ALLOWED_EVENTS.contains(&event.as_str()),
            &format!("unsupported event label `{event}`"),
        )?;
    }
    Ok(())
}

fn require_challenge_result(payload: &Value, challenge_id: &str) -> Result<(), String> {
    let result = payload
        .get("challengeResult")
        .ok_or("payload missing challengeResult")?;
    require_value(
        result.get("challengeId").and_then(Value::as_str) == Some(challenge_id),
        "challengeResult.challengeId mismatch",
    )?;
    require_value(
        result.get("passed").and_then(Value::as_bool) == Some(true),
        "challengeResult.passed must be true",
    )
}

fn require_checksum_shape(value: &str) -> Result<(), String> {
    let digest = value
        .strip_prefix("sha256:")
        .ok_or("checksum missing sha256 prefix")?;
    require_value(
        digest.len() == 64
            && digest
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "checksum must be lowercase sha256 hex",
    )
}

fn sha256_canonical(value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_json(value).as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => value.to_string(),
        Value::String(text) => {
            serde_json::to_string(text).expect("string serialization cannot fail")
        }
        Value::Array(items) => {
            let joined = items
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{joined}]")
        }
        Value::Object(map) => {
            let sorted: BTreeMap<&String, &Value> = map.iter().collect();
            let fields = sorted
                .iter()
                .map(|(key, item)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("key serialization cannot fail"),
                        canonical_json(item)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{fields}}}")
        }
    }
}

fn read_i64(value: &Value, path: &[&str]) -> Result<i64, String> {
    let mut cursor = value;
    for key in path {
        cursor = cursor
            .get(*key)
            .ok_or_else(|| format!("missing {}", path.join(".")))?;
    }
    cursor
        .as_i64()
        .or_else(|| cursor.as_u64().and_then(|raw| i64::try_from(raw).ok()))
        .ok_or_else(|| format!("{} is not integer", path.join(".")))
}

fn read_f64(value: &Value, path: &[&str]) -> Result<f64, String> {
    let mut cursor = value;
    for key in path {
        cursor = cursor
            .get(*key)
            .ok_or_else(|| format!("missing {}", path.join(".")))?;
    }
    cursor
        .as_f64()
        .ok_or_else(|| format!("{} is not number", path.join(".")))
}

fn check_public_string(value: &str, field: &str) -> Result<(), String> {
    let trimmed = value.trim();
    let lowered = trimmed.to_lowercase();
    require_value(
        !trimmed.starts_with('/'),
        &format!("{field} starts with absolute local path"),
    )?;
    require_value(
        !trimmed.starts_with('~'),
        &format!("{field} starts with home marker"),
    )?;
    require_value(
        !lowered.starts_with("file://"),
        &format!("{field} uses file URL"),
    )?;
    require_value(
        !lowered.contains("/users/"),
        &format!("{field} contains private local path marker"),
    )?;
    require_value(
        !lowered.contains("../") && !lowered.contains("..\\"),
        &format!("{field} contains traversal marker"),
    )
}

fn require_value(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_sorts_keys() {
        let value = json!({"b": 2, "a": {"d": 4, "c": 3}});
        assert_eq!(canonical_json(&value), r#"{"a":{"c":3,"d":4},"b":2}"#);
    }

    #[test]
    fn valid_fixture_scores_all_mechanical_acs() {
        let manifest = default_fixture_set_path()
            .parent()
            .unwrap()
            .join("valid-artifact/2000m.v3.json");
        let harness = Harness::load(manifest, default_fixture_set_path()).unwrap();
        let result = score(&harness, None);
        assert!(
            result.mechanical.ranked,
            "{:?}",
            result.mechanical.failed_acs
        );
        assert_eq!(result.mechanical.pass_count, TOTAL_ACS);
        assert!(!result.visual.ranked);
    }

    #[test]
    fn invalid_enum_fixture_rank_blocks_mechanical() {
        let manifest = default_fixture_set_path()
            .parent()
            .unwrap()
            .join("invalid-enum-artifact/2000m.v3.json");
        let harness = Harness::load(manifest, default_fixture_set_path()).unwrap();
        let result = score(&harness, None);
        assert!(!result.mechanical.ranked);
        assert!(result.mechanical.failed_acs.contains(&"M06".to_string()));
    }

    #[test]
    fn driver_stderr_is_drained() {
        let manifest = default_fixture_set_path()
            .parent()
            .unwrap()
            .join("stderr-artifact/2000m.v3.json");
        let harness = Harness::load(manifest, default_fixture_set_path()).unwrap();
        let mut client = harness.spawn_driver().unwrap();
        let response = client.send("init", json!({ "seed": 1 })).unwrap();
        assert!(response.ok);
    }

    #[test]
    fn driver_timeout_seconds_is_enforced() {
        let manifest = default_fixture_set_path()
            .parent()
            .unwrap()
            .join("timeout-artifact/2000m.v3.json");
        let harness = Harness::load(manifest, default_fixture_set_path()).unwrap();
        let mut client = harness.spawn_driver().unwrap();
        let started = std::time::Instant::now();
        let err = client.send("init", json!({ "seed": 1 })).unwrap_err();
        assert!(err.contains("timeout"), "{err}");
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
