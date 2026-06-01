use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const SCENARIO_SCHEMA_VERSION: &str = "2000m.v2.scenario.v1";
const RUN_RECORD_SCHEMA_VERSION: &str = "2000m.v2.run-record.v1";
const RESULT_SCHEMA_VERSION: &str = "2000m.v2.result.v1";

type BoxError = Box<dyn Error + Send + Sync>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    #[serde(rename = "scenarioId")]
    scenario_id: String,
    #[serde(rename = "scenarioVersion")]
    scenario_version: u64,
    #[serde(rename = "title")]
    _title: String,
    #[serde(rename = "baseTrack")]
    base_track: String,
    phases: Vec<ScenarioPhase>,
    scoring: ScoringWeights,
    #[serde(default, rename = "neutralityRules")]
    neutrality_rules: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioPhase {
    #[serde(rename = "phaseId")]
    phase_id: String,
    kind: String,
    #[serde(rename = "prompt")]
    _prompt: String,
    #[serde(default, rename = "allowedInputs")]
    _allowed_inputs: Vec<String>,
    #[serde(rename = "requiredOutputs")]
    required_outputs: Vec<String>,
    #[serde(default, rename = "trapType")]
    trap_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScoringWeights {
    #[serde(rename = "artifactQualityWeight")]
    artifact_quality: f64,
    #[serde(rename = "feedbackIntegrationWeight")]
    feedback_integration: f64,
    #[serde(rename = "recoveryHandoffWeight")]
    recovery_handoff: f64,
    #[serde(rename = "stopConditionWeight")]
    stop_condition: f64,
    #[serde(rename = "evidenceReplayWeight")]
    evidence_replay: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunRecord {
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    #[serde(rename = "scenarioId")]
    scenario_id: String,
    #[serde(rename = "scenarioVersion")]
    scenario_version: u64,
    entrant: Entrant,
    artifact: Artifact,
    phases: Vec<RunPhase>,
    #[serde(rename = "finalRecommendation")]
    final_recommendation: FinalRecommendation,
    evidence: Vec<EvidenceRef>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entrant {
    label: String,
    #[serde(rename = "processType")]
    process_type: String,
    #[serde(default, rename = "notes")]
    _notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    #[serde(rename = "repoOrPath")]
    repo_or_path: String,
    #[serde(rename = "commitOrDigest")]
    commit_or_digest: String,
    #[serde(rename = "buildCommand")]
    build_command: String,
    #[serde(rename = "scoreCommand")]
    score_command: String,
    #[serde(default, rename = "v1ConformanceJson")]
    v1_conformance_json: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunPhase {
    #[serde(rename = "phaseId")]
    phase_id: String,
    outputs: BTreeMap<String, Value>,
    #[serde(default, rename = "feedbackResponses")]
    feedback_responses: Vec<FeedbackResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FeedbackResponse {
    #[serde(rename = "feedbackId")]
    feedback_id: String,
    decision: String,
    rationale: String,
    #[serde(default, rename = "evidenceRef")]
    _evidence_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FinalRecommendation {
    decision: String,
    rationale: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceRef {
    label: String,
    #[serde(rename = "ref")]
    reference: String,
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Serialize)]
struct V2Result {
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    #[serde(rename = "scenarioId")]
    scenario_id: String,
    #[serde(rename = "scenarioVersion")]
    scenario_version: u64,
    entrant: String,
    #[serde(rename = "processType")]
    process_type: String,
    ranked: bool,
    #[serde(rename = "compositeScore")]
    composite_score: f64,
    components: ComponentScores,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ComponentScores {
    #[serde(rename = "artifactQuality")]
    artifact_quality: ComponentScore,
    #[serde(rename = "feedbackIntegration")]
    feedback_integration: ComponentScore,
    #[serde(rename = "recoveryHandoff")]
    recovery_handoff: ComponentScore,
    #[serde(rename = "stopCondition")]
    stop_condition: ComponentScore,
    #[serde(rename = "evidenceReplay")]
    evidence_replay: ComponentScore,
}

#[derive(Debug, Serialize)]
struct ComponentScore {
    score: f64,
    detail: String,
}

fn main() -> Result<(), BoxError> {
    let args: Vec<String> = env::args().collect();
    let parsed = parse_args(&args)?;
    let scenario_path = PathBuf::from(parsed.scenario_path);
    let run_record_path = PathBuf::from(parsed.run_record_path);
    let scenario: Scenario = read_json(&scenario_path)?;
    let run_record: RunRecord = read_json(&run_record_path)?;
    let result = score_run(&scenario, &run_record, &run_record_path)?;
    let pretty = serde_json::to_string_pretty(&result)?;

    if let Some(out) = parsed.json_out {
        fs::write(out, format!("{}\n", pretty))?;
    }

    print_summary(&result);
    Ok(())
}

struct ParsedArgs {
    scenario_path: String,
    run_record_path: String,
    json_out: Option<String>,
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, BoxError> {
    if args.len() < 3 || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Err(usage().into());
    }

    let scenario_path = args[1].clone();
    let run_record_path = args[2].clone();
    let mut json_out = None;
    let mut idx = 3;
    while idx < args.len() {
        match args[idx].as_str() {
            "--json-out" => {
                let value = args.get(idx + 1).ok_or_else(usage)?;
                json_out = Some(value.clone());
                idx += 2;
            }
            other => return Err(format!("unknown argument `{}`\n{}", other, usage()).into()),
        }
    }

    Ok(ParsedArgs {
        scenario_path,
        run_record_path,
        json_out,
    })
}

fn usage() -> String {
    "usage: m2000-v2-conformance <scenario.json> <run-record.json> [--json-out <path>]".to_string()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, BoxError> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read `{}`: {}", path.display(), err))?;
    serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse `{}` as JSON contract: {}",
            path.display(),
            err
        )
        .into()
    })
}

fn score_run(
    scenario: &Scenario,
    run: &RunRecord,
    run_record_path: &Path,
) -> Result<V2Result, BoxError> {
    let mut warnings = Vec::new();
    validate_contract_identity(scenario, run, &mut warnings)?;
    validate_neutrality(scenario, &mut warnings);
    validate_required_outputs(scenario, run, &mut warnings);

    let artifact_quality = score_artifact_quality(run, run_record_path, &mut warnings);
    let feedback_integration = score_feedback_integration(scenario, run, &mut warnings);
    let recovery_handoff = score_recovery_handoff(scenario, run, &mut warnings);
    let stop_condition = score_stop_condition(scenario, run, &mut warnings);
    let evidence_replay = score_evidence_replay(run, &mut warnings);

    let weights = &scenario.scoring;
    let weight_sum = weights.artifact_quality
        + weights.feedback_integration
        + weights.recovery_handoff
        + weights.stop_condition
        + weights.evidence_replay;
    if (weight_sum - 1.0).abs() > 0.0001 {
        warnings.push(format!(
            "scenario scoring weights sum to {:.4}, expected 1.0; composite is reported but not rankable",
            weight_sum
        ));
    }

    let composite_score = round2(
        artifact_quality.score * weights.artifact_quality
            + feedback_integration.score * weights.feedback_integration
            + recovery_handoff.score * weights.recovery_handoff
            + stop_condition.score * weights.stop_condition
            + evidence_replay.score * weights.evidence_replay,
    );
    let ranked = warnings
        .iter()
        .all(|warning| !warning.starts_with("RANK-BLOCK:"))
        && (weight_sum - 1.0).abs() <= 0.0001;

    Ok(V2Result {
        schema_version: RESULT_SCHEMA_VERSION.to_string(),
        scenario_id: scenario.scenario_id.clone(),
        scenario_version: scenario.scenario_version,
        entrant: run.entrant.label.clone(),
        process_type: run.entrant.process_type.clone(),
        ranked,
        composite_score,
        components: ComponentScores {
            artifact_quality,
            feedback_integration,
            recovery_handoff,
            stop_condition,
            evidence_replay,
        },
        warnings,
    })
}

fn validate_contract_identity(
    scenario: &Scenario,
    run: &RunRecord,
    warnings: &mut Vec<String>,
) -> Result<(), BoxError> {
    if scenario.schema_version != SCENARIO_SCHEMA_VERSION {
        return Err(format!(
            "unsupported scenario schemaVersion `{}`; expected `{}`",
            scenario.schema_version, SCENARIO_SCHEMA_VERSION
        )
        .into());
    }
    if run.schema_version != RUN_RECORD_SCHEMA_VERSION {
        return Err(format!(
            "unsupported run-record schemaVersion `{}`; expected `{}`",
            run.schema_version, RUN_RECORD_SCHEMA_VERSION
        )
        .into());
    }
    if scenario.scenario_id != run.scenario_id {
        return Err(format!(
            "scenarioId mismatch: scenario `{}` vs run `{}`",
            scenario.scenario_id, run.scenario_id
        )
        .into());
    }
    if scenario.scenario_version != run.scenario_version {
        return Err(format!(
            "scenarioVersion mismatch: scenario `{}` vs run `{}`",
            scenario.scenario_version, run.scenario_version
        )
        .into());
    }
    if scenario.phases.is_empty() {
        warnings.push("RANK-BLOCK: scenario has no phases".to_string());
    }
    if scenario.base_track != "v1-artifact-score" {
        return Err(format!(
            "unsupported baseTrack `{}`; first v2 scorer supports `v1-artifact-score` only",
            scenario.base_track
        )
        .into());
    }
    validate_contract_values(scenario, run)?;
    Ok(())
}

fn validate_contract_values(scenario: &Scenario, run: &RunRecord) -> Result<(), BoxError> {
    const PHASE_KINDS: &[&str] = &[
        "initial-build",
        "scorer-feedback",
        "reviewer-feedback",
        "context-wipe-recovery",
        "requirement-trap",
        "stop-decision",
    ];
    const REQUIRED_OUTPUTS: &[&str] = &[
        "artifact-ref",
        "build-command",
        "score-command",
        "conformance-json",
        "feedback-response",
        "handoff-summary",
        "stop-recommendation",
        "evidence-ref",
    ];
    const TRAP_TYPES: &[&str] = &["none", "impossible", "stale", "probe-only"];
    const PROCESS_TYPES: &[&str] = &[
        "single-model",
        "model-plus-operator",
        "scripted-agent-loop",
        "workflow-system",
        "other",
    ];
    const FEEDBACK_DECISIONS: &[&str] = &[
        "accepted",
        "rejected_with_reason",
        "needs_scorer_inspection",
        "deferred",
    ];
    const FINAL_DECISIONS: &[&str] = &["continue", "stop", "redesign", "inspect_scorer"];
    const EVIDENCE_KINDS: &[&str] = &[
        "repo",
        "commit",
        "conformance-json",
        "log",
        "summary",
        "other",
    ];

    for phase in &scenario.phases {
        ensure_allowed("phase.kind", &phase.kind, PHASE_KINDS)?;
        if let Some(trap_type) = &phase.trap_type {
            ensure_allowed("phase.trapType", trap_type, TRAP_TYPES)?;
        }
        for output in &phase.required_outputs {
            ensure_allowed("phase.requiredOutputs", output, REQUIRED_OUTPUTS)?;
        }
    }
    ensure_allowed(
        "entrant.processType",
        &run.entrant.process_type,
        PROCESS_TYPES,
    )?;
    ensure_allowed(
        "finalRecommendation.decision",
        &run.final_recommendation.decision,
        FINAL_DECISIONS,
    )?;
    for phase in &run.phases {
        for response in &phase.feedback_responses {
            ensure_allowed(
                "feedbackResponses.decision",
                &response.decision,
                FEEDBACK_DECISIONS,
            )?;
        }
    }
    for evidence in &run.evidence {
        if let Some(kind) = &evidence.kind {
            ensure_allowed("evidence.kind", kind, EVIDENCE_KINDS)?;
        }
    }
    Ok(())
}

fn ensure_allowed(field: &str, value: &str, allowed: &[&str]) -> Result<(), BoxError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "unsupported {} value `{}`; allowed: {}",
            field,
            value,
            allowed.join(", ")
        )
        .into())
    }
}

fn validate_neutrality(scenario: &Scenario, warnings: &mut Vec<String>) {
    for rule in &scenario.neutrality_rules {
        let lower = rule.to_ascii_lowercase();
        if lower.contains("framework:") || lower.contains("requires framework") {
            warnings.push(format!(
                "RANK-BLOCK: scenario neutrality rule contains framework-specific wording: `{}`",
                rule
            ));
        }
    }
}

fn validate_required_outputs(scenario: &Scenario, run: &RunRecord, warnings: &mut Vec<String>) {
    for scenario_phase in &scenario.phases {
        let Some(run_phase) = run_phase(run, &scenario_phase.phase_id) else {
            for required in &scenario_phase.required_outputs {
                warnings.push(format!(
                    "RANK-BLOCK: phase `{}` missing required output `{}` because the phase is absent",
                    scenario_phase.phase_id, required
                ));
            }
            continue;
        };

        for required in &scenario_phase.required_outputs {
            if !required_output_satisfied(run_phase, required) {
                warnings.push(format!(
                    "RANK-BLOCK: phase `{}` missing required output `{}`",
                    scenario_phase.phase_id, required
                ));
            }
        }
    }
}

fn required_output_satisfied(phase: &RunPhase, required: &str) -> bool {
    match required {
        "feedback-response" => !phase.feedback_responses.is_empty(),
        _ => has_nonempty_output(phase, required),
    }
}

fn score_artifact_quality(
    run: &RunRecord,
    run_record_path: &Path,
    warnings: &mut Vec<String>,
) -> ComponentScore {
    let Some(ref_path) = &run.artifact.v1_conformance_json else {
        warnings.push(
            "RANK-BLOCK: artifact has no v1ConformanceJson for v1-artifact-score; artifact quality is zero"
                .to_string(),
        );
        return component(0.0, "missing v1 conformance JSON");
    };

    if looks_private_or_local(ref_path) {
        warnings.push(format!(
            "RANK-BLOCK: artifact v1ConformanceJson `{}` is private or local-only before read",
            ref_path
        ));
        return component(0.0, "private/local v1 conformance JSON");
    }

    let path = resolve_ref(run_record_path, ref_path);
    let Ok(text) = fs::read_to_string(&path) else {
        warnings.push(format!(
            "RANK-BLOCK: artifact conformance JSON `{}` could not be read; artifact quality is zero",
            ref_path
        ));
        return component(0.0, "unreadable v1 conformance JSON");
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        warnings.push(format!(
            "RANK-BLOCK: artifact conformance JSON `{}` could not be parsed; artifact quality is zero",
            ref_path
        ));
        return component(0.0, "invalid v1 conformance JSON");
    };

    let Some(score) = verified_v1_conformance_score(ref_path, &json, run, warnings) else {
        return component(0.0, "invalid v1 conformance result");
    };
    component(score, "from verified v1 conformance result")
}

fn verified_v1_conformance_score(
    ref_path: &str,
    json: &Value,
    run: &RunRecord,
    warnings: &mut Vec<String>,
) -> Option<f64> {
    if json.get("protocolVersion").and_then(Value::as_str) != Some("2000m.driver.v1") {
        rank_block_v1(ref_path, "missing or wrong protocolVersion", warnings);
        return None;
    }

    let Some(game_dir) = json.get("gameDir").and_then(Value::as_str) else {
        rank_block_v1(ref_path, "missing gameDir", warnings);
        return None;
    };
    if game_dir.trim().is_empty() {
        rank_block_v1(ref_path, "empty gameDir", warnings);
        return None;
    }
    if !v1_game_dir_matches_artifact(game_dir, &run.artifact.repo_or_path) {
        rank_block_v1(
            ref_path,
            "gameDir does not match artifact.repoOrPath",
            warnings,
        );
        return None;
    }

    if json
        .get("determinism")
        .and_then(|value| value.get("pass"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        rank_block_v1(ref_path, "determinism.pass is not true", warnings);
        return None;
    }

    let Some(pass_count) = json.get("passCount").and_then(Value::as_u64) else {
        rank_block_v1(ref_path, "missing numeric passCount", warnings);
        return None;
    };
    let Some(total_acs) = json.get("totalAcs").and_then(Value::as_u64) else {
        rank_block_v1(ref_path, "missing numeric totalAcs", warnings);
        return None;
    };
    if total_acs == 0 || pass_count > total_acs {
        rank_block_v1(ref_path, "invalid passCount/totalAcs", warnings);
        return None;
    }
    if total_acs != 28 {
        rank_block_v1(
            ref_path,
            "totalAcs is not the canonical v1 28-AC set",
            warnings,
        );
        return None;
    }

    let Some(composite_score) = json.get("compositeScore").and_then(Value::as_f64) else {
        rank_block_v1(ref_path, "missing numeric compositeScore", warnings);
        return None;
    };
    if !(0.0..=100.0).contains(&composite_score) {
        rank_block_v1(ref_path, "compositeScore outside 0..=100", warnings);
        return None;
    }

    let Some(acs) = json.get("acs").and_then(Value::as_array) else {
        rank_block_v1(ref_path, "missing acs array", warnings);
        return None;
    };
    if acs.len() != total_acs as usize {
        rank_block_v1(ref_path, "acs length does not match totalAcs", warnings);
        return None;
    }
    let mut computed_pass_count = 0_u64;
    let mut quality_sum = 0.0;
    for (index, ac) in acs.iter().enumerate() {
        let id = ac.get("id").and_then(Value::as_str);
        let id_missing = id.is_none_or(str::is_empty);
        let pass = ac.get("pass").and_then(Value::as_bool);
        let skipped = ac.get("skipped").and_then(Value::as_bool);
        let quality = ac.get("quality").and_then(Value::as_u64);
        let detail_missing = ac
            .get("detail")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty);
        if id_missing || pass.is_none() || skipped.is_none() || quality.is_none() || detail_missing
        {
            rank_block_v1(
                ref_path,
                &format!("acs[{}] is missing required verdict fields", index),
                warnings,
            );
            return None;
        }
        let pass = pass.expect("validated pass field");
        let skipped = skipped.expect("validated skipped field");
        let quality = quality.expect("validated quality field");
        let expected_id = format!("AC{}", index + 1);
        if id != Some(expected_id.as_str()) {
            rank_block_v1(
                ref_path,
                &format!("acs[{}] id is not canonical {}", index, expected_id),
                warnings,
            );
            return None;
        }
        if quality > 100 {
            rank_block_v1(
                ref_path,
                &format!("acs[{}] has quality outside 0..=100", index),
                warnings,
            );
            return None;
        }
        if pass && !skipped {
            computed_pass_count += 1;
        }
        if !skipped {
            quality_sum += quality as f64;
        }
    }
    if computed_pass_count != pass_count {
        rank_block_v1(ref_path, "passCount does not match AC verdicts", warnings);
        return None;
    }
    let computed_composite = (computed_pass_count as f64 / total_acs as f64) * 70.0
        + (quality_sum / total_acs as f64) * 0.3;
    if (computed_composite - composite_score).abs() > 0.0001 {
        rank_block_v1(
            ref_path,
            "compositeScore does not match v1 pass/quality formula",
            warnings,
        );
        return None;
    }

    Some(clamp_score(composite_score))
}

fn v1_game_dir_matches_artifact(game_dir: &str, repo_or_path: &str) -> bool {
    if game_dir == repo_or_path {
        return true;
    }
    if !looks_private_or_local(game_dir) {
        return false;
    }
    match (artifact_tail(game_dir), artifact_tail(repo_or_path)) {
        (Some(game_tail), Some(artifact_tail)) => game_tail == artifact_tail,
        _ => false,
    }
}

fn artifact_tail(value: &str) -> Option<&str> {
    let trimmed = value.trim().trim_end_matches('/').trim_end_matches(".git");
    trimmed
        .rsplit(['/', '\\', ':'])
        .find(|part| !part.is_empty())
}

fn rank_block_v1(ref_path: &str, detail: &str, warnings: &mut Vec<String>) {
    warnings.push(format!(
        "RANK-BLOCK: artifact conformance JSON `{}` is not a complete 2000m.driver.v1 result: {}",
        ref_path, detail
    ));
}

fn score_feedback_integration(
    scenario: &Scenario,
    run: &RunRecord,
    warnings: &mut Vec<String>,
) -> ComponentScore {
    let required = phases_requiring(scenario, |phase| {
        phase.kind == "scorer-feedback"
            || phase.kind == "reviewer-feedback"
            || phase
                .required_outputs
                .iter()
                .any(|output| output == "feedback-response")
    });
    if required.is_empty() {
        return component(100.0, "scenario has no feedback phase");
    }

    let mut covered = 0.0;
    for phase_id in &required {
        let Some(phase) = run_phase(run, phase_id) else {
            warnings.push(format!("missing feedback phase `{}`", phase_id));
            continue;
        };
        if phase.feedback_responses.is_empty() {
            warnings.push(format!(
                "feedback phase `{}` has no feedbackResponses",
                phase_id
            ));
            continue;
        }
        let valid = phase.feedback_responses.iter().all(|response| {
            !response.feedback_id.trim().is_empty()
                && !response.rationale.trim().is_empty()
                && matches!(
                    response.decision.as_str(),
                    "accepted" | "rejected_with_reason" | "needs_scorer_inspection" | "deferred"
                )
        });
        if valid {
            covered += 1.0;
        } else {
            warnings.push(format!(
                "feedback phase `{}` has incomplete feedbackResponses",
                phase_id
            ));
        }
    }

    component(
        round2((covered / required.len() as f64) * 100.0),
        format!(
            "{}/{} feedback phases covered",
            covered as usize,
            required.len()
        ),
    )
}

fn score_recovery_handoff(
    scenario: &Scenario,
    run: &RunRecord,
    warnings: &mut Vec<String>,
) -> ComponentScore {
    let required = phases_requiring(scenario, |phase| {
        phase.kind == "context-wipe-recovery"
            || phase
                .required_outputs
                .iter()
                .any(|output| output == "handoff-summary")
    });
    if required.is_empty() {
        return component(100.0, "scenario has no context-wipe recovery phase");
    }

    let mut covered = 0.0;
    for phase_id in &required {
        let Some(phase) = run_phase(run, phase_id) else {
            warnings.push(format!(
                "missing context-wipe recovery phase `{}`",
                phase_id
            ));
            continue;
        };
        if has_nonempty_output(phase, "handoff-summary") {
            covered += 1.0;
        } else {
            warnings.push(format!(
                "context-wipe recovery phase `{}` lacks non-empty handoff-summary",
                phase_id
            ));
        }
    }

    component(
        round2((covered / required.len() as f64) * 100.0),
        format!(
            "{}/{} recovery phases covered",
            covered as usize,
            required.len()
        ),
    )
}

fn score_stop_condition(
    scenario: &Scenario,
    run: &RunRecord,
    warnings: &mut Vec<String>,
) -> ComponentScore {
    if run.final_recommendation.rationale.trim().is_empty() {
        warnings.push("finalRecommendation has empty rationale".to_string());
        return component(0.0, "empty final recommendation rationale");
    }

    let trap_types: BTreeSet<String> = scenario
        .phases
        .iter()
        .filter_map(|phase| phase.trap_type.clone())
        .filter(|trap| trap != "none")
        .collect();

    if trap_types.is_empty() {
        return component(80.0, "no trap phases; recommendation has rationale");
    }

    match run.final_recommendation.decision.as_str() {
        "inspect_scorer" | "redesign" => component(
            100.0,
            format!(
                "correctly avoids blind continuation for trap types: {}",
                trap_types.into_iter().collect::<Vec<_>>().join(", ")
            ),
        ),
        "stop" => component(
            60.0,
            "stops, but trap phases usually require inspection/redesign",
        ),
        "continue" => {
            warnings.push("trap phase present but finalRecommendation is continue".to_string());
            component(0.0, "continued despite trap phase")
        }
        other => {
            warnings.push(format!("unknown finalRecommendation decision `{}`", other));
            component(0.0, "unknown stop decision")
        }
    }
}

fn score_evidence_replay(run: &RunRecord, warnings: &mut Vec<String>) -> ComponentScore {
    let mut score: f64 = 100.0;
    if run.artifact.build_command.trim().is_empty() || run.artifact.score_command.trim().is_empty()
    {
        warnings.push("RANK-BLOCK: artifact buildCommand or scoreCommand is empty".to_string());
        score = 0.0;
    }
    if run.artifact.commit_or_digest.trim().is_empty() {
        warnings.push("RANK-BLOCK: artifact commitOrDigest is empty".to_string());
        score = 0.0;
    }
    if run.evidence.is_empty() {
        warnings.push("RANK-BLOCK: run record has no evidence refs".to_string());
        score = 0.0;
    }

    let evidence_refs = run
        .evidence
        .iter()
        .map(|e| &e.reference)
        .chain(run.artifact.v1_conformance_json.iter());
    for value in std::iter::once(&run.artifact.repo_or_path).chain(evidence_refs) {
        if looks_private_or_local(value) {
            warnings.push(format!(
                "RANK-BLOCK: private or local-only evidence ref is not rankable: `{}`",
                value
            ));
            score = 0.0;
        }
    }

    let labeled = run
        .evidence
        .iter()
        .filter(|evidence| !evidence.label.trim().is_empty() && evidence.kind.is_some())
        .count();
    if !run.evidence.is_empty() && labeled < run.evidence.len() {
        warnings.push("some evidence refs lack a kind label".to_string());
        score = score.min(80.0);
    }

    component(
        score,
        "build, score, commit, and evidence replayability checks",
    )
}

fn phases_requiring<F>(scenario: &Scenario, predicate: F) -> Vec<String>
where
    F: Fn(&ScenarioPhase) -> bool,
{
    scenario
        .phases
        .iter()
        .filter(|phase| predicate(phase))
        .map(|phase| phase.phase_id.clone())
        .collect()
}

fn run_phase<'a>(run: &'a RunRecord, phase_id: &str) -> Option<&'a RunPhase> {
    run.phases.iter().find(|phase| phase.phase_id == phase_id)
}

fn has_nonempty_output(phase: &RunPhase, key: &str) -> bool {
    match phase.outputs.get(key) {
        Some(Value::String(value)) => !value.trim().is_empty(),
        Some(Value::Null) | None => false,
        Some(_) => true,
    }
}

fn resolve_ref(base_file: &Path, reference: &str) -> PathBuf {
    let path = PathBuf::from(reference);
    if path.is_absolute() {
        path
    } else {
        base_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    }
}

fn looks_private_or_local(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    trimmed.starts_with('/')
        || trimmed.starts_with('~')
        || lower.starts_with("file://")
        || lower.contains("/users/")
        || lower.contains("\\users\\")
        || has_windows_drive_prefix(&lower)
        || trimmed.starts_with("\\\\")
        || lower.contains("..")
}

fn has_windows_drive_prefix(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_lowercase() && bytes[1] == b':'
}

fn component(score: f64, detail: impl Into<String>) -> ComponentScore {
    ComponentScore {
        score: clamp_score(score),
        detail: detail.into(),
    }
}

fn clamp_score(score: f64) -> f64 {
    round2(score.clamp(0.0, 100.0))
}

fn round2(score: f64) -> f64 {
    (score * 100.0).round() / 100.0
}

fn print_summary(result: &V2Result) {
    println!(
        "2000m v2 conformance: {} v{} entrant={} ranked={} composite={:.2}",
        result.scenario_id,
        result.scenario_version,
        result.entrant,
        result.ranked,
        result.composite_score
    );
    println!(
        "  artifact={:.2} feedback={:.2} recovery={:.2} stop={:.2} replay={:.2}",
        result.components.artifact_quality.score,
        result.components.feedback_integration.score,
        result.components.recovery_handoff.score,
        result.components.stop_condition.score,
        result.components.evidence_replay.score
    );
    for warning in &result.warnings {
        println!("  WARN {}", warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn base_scenario() -> Scenario {
        Scenario {
            schema_version: SCENARIO_SCHEMA_VERSION.to_string(),
            scenario_id: "workflow-resilience-smoke".to_string(),
            scenario_version: 1,
            _title: "Workflow-resilience smoke scenario".to_string(),
            base_track: "v1-artifact-score".to_string(),
            neutrality_rules: vec!["No workflow framework is required or privileged.".to_string()],
            scoring: ScoringWeights {
                artifact_quality: 0.4,
                feedback_integration: 0.2,
                recovery_handoff: 0.15,
                stop_condition: 0.15,
                evidence_replay: 0.1,
            },
            phases: vec![
                ScenarioPhase {
                    phase_id: "build".to_string(),
                    kind: "initial-build".to_string(),
                    _prompt: "Build and score the artifact.".to_string(),
                    _allowed_inputs: vec![],
                    required_outputs: vec![
                        "artifact-ref".to_string(),
                        "conformance-json".to_string(),
                    ],
                    trap_type: Some("none".to_string()),
                },
                ScenarioPhase {
                    phase_id: "feedback".to_string(),
                    kind: "scorer-feedback".to_string(),
                    _prompt: "Respond to scorer feedback.".to_string(),
                    _allowed_inputs: vec![],
                    required_outputs: vec!["feedback-response".to_string()],
                    trap_type: Some("none".to_string()),
                },
                ScenarioPhase {
                    phase_id: "wipe".to_string(),
                    kind: "context-wipe-recovery".to_string(),
                    _prompt: "Resume after context wipe.".to_string(),
                    _allowed_inputs: vec![],
                    required_outputs: vec!["handoff-summary".to_string()],
                    trap_type: Some("none".to_string()),
                },
                ScenarioPhase {
                    phase_id: "trap".to_string(),
                    kind: "requirement-trap".to_string(),
                    _prompt: "Handle probe-only trap.".to_string(),
                    _allowed_inputs: vec![],
                    required_outputs: vec!["stop-recommendation".to_string()],
                    trap_type: Some("probe-only".to_string()),
                },
            ],
        }
    }

    fn base_run(v1_json: String) -> RunRecord {
        RunRecord {
            schema_version: RUN_RECORD_SCHEMA_VERSION.to_string(),
            scenario_id: "workflow-resilience-smoke".to_string(),
            scenario_version: 1,
            entrant: Entrant {
                label: "neutral-entrant".to_string(),
                process_type: "scripted-agent-loop".to_string(),
                _notes: None,
            },
            artifact: Artifact {
                repo_or_path: "https://github.com/example/2000m-entry".to_string(),
                commit_or_digest: "abc123".to_string(),
                build_command: "cargo build".to_string(),
                score_command: "cargo run --manifest-path v1/conformance/Cargo.toml".to_string(),
                v1_conformance_json: Some(v1_json),
            },
            phases: vec![
                RunPhase {
                    phase_id: "build".to_string(),
                    outputs: BTreeMap::from([
                        (
                            "artifact-ref".to_string(),
                            json!("https://github.com/example/2000m-entry"),
                        ),
                        ("conformance-json".to_string(), json!("v1-conformance.json")),
                    ]),
                    feedback_responses: vec![],
                },
                RunPhase {
                    phase_id: "feedback".to_string(),
                    outputs: BTreeMap::new(),
                    feedback_responses: vec![FeedbackResponse {
                        feedback_id: "score-ac28".to_string(),
                        decision: "needs_scorer_inspection".to_string(),
                        rationale:
                            "Probe-only signal should not be claimed as a ranked visual pass."
                                .to_string(),
                        _evidence_ref: None,
                    }],
                },
                RunPhase {
                    phase_id: "wipe".to_string(),
                    outputs: BTreeMap::from([(
                        "handoff-summary".to_string(),
                        json!("Current state, failing ACs, and next command are recorded."),
                    )]),
                    feedback_responses: vec![],
                },
                RunPhase {
                    phase_id: "trap".to_string(),
                    outputs: BTreeMap::from([(
                        "stop-recommendation".to_string(),
                        json!("inspect_scorer"),
                    )]),
                    feedback_responses: vec![],
                },
            ],
            final_recommendation: FinalRecommendation {
                decision: "inspect_scorer".to_string(),
                rationale: "The scenario contains a probe-only trap.".to_string(),
            },
            evidence: vec![EvidenceRef {
                label: "v1 conformance".to_string(),
                reference: "https://example.invalid/v1-conformance.json".to_string(),
                kind: Some("conformance-json".to_string()),
            }],
        }
    }

    fn write_temp_json(name: &str, value: Value) -> (PathBuf, PathBuf) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("m2000-v2-test-{}-{}", std::process::id(), nonce));
        fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join(name);
        fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).expect("write temp json");
        (dir, path)
    }

    fn v1_result(composite_score: f64) -> Value {
        let mut selected = None;
        for pass_count in 0..=28_usize {
            let pass_component = (pass_count as f64 / 28.0) * 70.0;
            let target_quality_sum = ((composite_score - pass_component) / 0.3 * 28.0).round();
            if !(0.0..=2800.0).contains(&target_quality_sum) {
                continue;
            }
            let quality_sum = target_quality_sum as usize;
            let actual = pass_component + (quality_sum as f64 / 28.0) * 0.3;
            if (actual - composite_score).abs() < 0.0001 {
                selected = Some((pass_count, quality_sum));
                break;
            }
        }
        let (pass_count, mut remaining_quality) =
            selected.expect("test score can be represented by v1 formula");
        let acs: Vec<Value> = (1..=28)
            .map(|idx| {
                let pass = idx <= pass_count;
                let quality = remaining_quality.min(100);
                remaining_quality -= quality;
                json!({
                    "id": format!("AC{}", idx),
                    "name": format!("Acceptance criterion {}", idx),
                    "pass": pass,
                    "skipped": false,
                    "quality": quality,
                    "detail": "sample v1 verdict",
                    "breakdown": {
                        "basic": quality,
                        "precision": quality,
                        "performance": quality,
                        "polish": quality
                    }
                })
            })
            .collect();
        json!({
            "protocolVersion": "2000m.driver.v1",
            "gameDir": "https://github.com/example/2000m-entry",
            "determinism": { "pass": true, "detail": "sample deterministic result" },
            "passCount": pass_count,
            "totalAcs": 28,
            "compositeScore": composite_score,
            "acs": acs
        })
    }

    #[test]
    fn consumes_v1_composite_score() {
        let (_dir, conformance_path) = write_temp_json("v1.json", v1_result(94.5));
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let run = base_run("v1.json".to_string());
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert_eq!(result.components.artifact_quality.score, 94.5);
        assert!(result.ranked);
    }

    #[test]
    fn fabricated_v1_composite_only_blocks_public_ranking() {
        let (_dir, conformance_path) =
            write_temp_json("v1.json", json!({ "compositeScore": 100.0 }));
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let run = base_run("v1.json".to_string());
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert_eq!(result.components.artifact_quality.score, 0.0);
        assert!(result.warnings.iter().any(|warning| {
            warning.starts_with("RANK-BLOCK:") && warning.contains("protocolVersion")
        }));
    }

    #[test]
    fn contradictory_v1_aggregate_fields_block_public_ranking() {
        let mut fake = v1_result(50.0);
        fake["compositeScore"] = json!(100.0);
        let (_dir, conformance_path) = write_temp_json("v1.json", fake);
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let run = base_run("v1.json".to_string());
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert_eq!(result.components.artifact_quality.score, 0.0);
        assert!(result.warnings.iter().any(|warning| {
            warning.starts_with("RANK-BLOCK:") && warning.contains("compositeScore")
        }));
    }

    #[test]
    fn noncanonical_v1_ac_set_blocks_public_ranking() {
        let mut fake = v1_result(100.0);
        fake["totalAcs"] = json!(1);
        fake["passCount"] = json!(1);
        fake["acs"] = json!([{
            "id": "AC1",
            "name": "Acceptance criterion 1",
            "pass": true,
            "skipped": false,
            "quality": 100,
            "detail": "too small",
            "breakdown": { "basic": 100, "precision": 100, "performance": 100, "polish": 100 }
        }]);
        let (_dir, conformance_path) = write_temp_json("v1.json", fake);
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let run = base_run("v1.json".to_string());
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert_eq!(result.components.artifact_quality.score, 0.0);
        assert!(result.warnings.iter().any(|warning| {
            warning.starts_with("RANK-BLOCK:") && warning.contains("canonical v1 28-AC set")
        }));
    }

    #[test]
    fn missing_handoff_summary_reduces_recovery_score() {
        let (_dir, conformance_path) = write_temp_json("v1.json", v1_result(80.0));
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let mut run = base_run("v1.json".to_string());
        run.phases.retain(|phase| phase.phase_id != "wipe");
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert_eq!(result.components.recovery_handoff.score, 0.0);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("missing context-wipe recovery phase")));
    }

    #[test]
    fn trap_phase_penalizes_blind_continuation() {
        let (_dir, conformance_path) = write_temp_json("v1.json", v1_result(80.0));
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let mut run = base_run("v1.json".to_string());
        run.final_recommendation.decision = "continue".to_string();
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert_eq!(result.components.stop_condition.score, 0.0);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("trap phase present")));
    }

    #[test]
    fn private_evidence_blocks_public_ranking() {
        let (_dir, conformance_path) = write_temp_json("v1.json", v1_result(80.0));
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let mut run = base_run("v1.json".to_string());
        run.evidence[0].reference = "/Users/private/run.json".to_string();
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert_eq!(result.components.evidence_replay.score, 0.0);
    }

    #[test]
    fn private_v1_conformance_path_blocks_public_ranking() {
        let (_dir, conformance_path) = write_temp_json("v1.json", v1_result(80.0));
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let run = base_run("/Users/private/v1.json".to_string());
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert!(result.warnings.iter().any(|warning| {
            warning.contains("private or local-only") && warning.contains("/Users/private/v1.json")
        }));
    }

    #[test]
    fn special_v1_conformance_path_is_rejected_before_reading() {
        let run_file = env::temp_dir().join("run-record.json");
        let scenario = base_scenario();
        let run = base_run("/dev/zero".to_string());
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert_eq!(result.components.artifact_quality.score, 0.0);
        assert!(result.warnings.iter().any(|warning| {
            warning.starts_with("RANK-BLOCK:") && warning.contains("before read")
        }));
    }

    #[test]
    fn windows_absolute_and_unc_paths_are_private_or_local() {
        assert!(looks_private_or_local(r"D:\tmp\2000m-entry"));
        assert!(looks_private_or_local(r"E:/tmp/2000m-entry"));
        assert!(looks_private_or_local(r"C:Users\alice\run.json"));
        assert!(looks_private_or_local(r"D:tmp\v1.json"));
        assert!(looks_private_or_local(r" C:tmp\entry"));
        assert!(looks_private_or_local(r"  \\server\share\2000m-entry"));
        assert!(looks_private_or_local(r"\\server\share\2000m-entry"));
    }

    #[test]
    fn mismatched_v1_game_dir_blocks_public_ranking() {
        let mut fake = v1_result(50.0);
        fake["gameDir"] = json!("https://github.com/example/other-entry");
        let (_dir, conformance_path) = write_temp_json("v1.json", fake);
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let run = base_run("v1.json".to_string());
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert_eq!(result.components.artifact_quality.score, 0.0);
        assert!(result
            .warnings
            .iter()
            .any(|warning| { warning.starts_with("RANK-BLOCK:") && warning.contains("gameDir") }));
    }

    #[test]
    fn public_v1_game_dir_with_only_matching_slug_blocks_public_ranking() {
        let mut fake = v1_result(50.0);
        fake["gameDir"] = json!("https://github.com/other-owner/2000m-entry");
        let (_dir, conformance_path) = write_temp_json("v1.json", fake);
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let run = base_run("v1.json".to_string());
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert_eq!(result.components.artifact_quality.score, 0.0);
        assert!(result
            .warnings
            .iter()
            .any(|warning| { warning.starts_with("RANK-BLOCK:") && warning.contains("gameDir") }));
    }

    #[test]
    fn local_v1_game_dir_with_matching_slug_is_accepted() {
        let mut fake = v1_result(50.0);
        fake["gameDir"] = json!("/tmp/2000m-entry");
        let (_dir, conformance_path) = write_temp_json("v1.json", fake);
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let run = base_run("v1.json".to_string());
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(result.ranked, "{:?}", result.warnings);
        assert_eq!(result.components.artifact_quality.score, 50.0);
    }

    #[test]
    fn missing_v1_conformance_json_blocks_public_ranking() {
        let run_file = env::temp_dir().join("run-record.json");
        let scenario = base_scenario();
        let mut run = base_run("v1.json".to_string());
        run.artifact.v1_conformance_json = None;
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert_eq!(result.components.artifact_quality.score, 0.0);
        assert!(result.warnings.iter().any(|warning| {
            warning.starts_with("RANK-BLOCK:") && warning.contains("v1ConformanceJson")
        }));
    }

    #[test]
    fn missing_scenario_required_outputs_is_rejected_during_parse() {
        let mut value: Value = serde_json::from_str(include_str!(
            "../../examples/workflow-resilience-smoke.scenario.json"
        ))
        .expect("example parses");
        value["phases"][0]
            .as_object_mut()
            .expect("phase object")
            .remove("requiredOutputs");
        let err = serde_json::from_value::<Scenario>(value).expect_err("missing field rejected");
        assert!(err.to_string().contains("requiredOutputs"));
    }

    #[test]
    fn missing_generic_required_outputs_block_public_ranking() {
        let (_dir, conformance_path) = write_temp_json("v1.json", v1_result(80.0));
        let run_file = conformance_path.with_file_name("run.json");
        let scenario = base_scenario();
        let mut run = base_run("v1.json".to_string());
        let trap = run
            .phases
            .iter_mut()
            .find(|phase| phase.phase_id == "trap")
            .expect("base run has trap phase");
        trap.outputs.clear();
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("phase `trap` missing required output")));
    }

    #[test]
    fn framework_specific_neutrality_words_block_ranking() {
        let (_dir, conformance_path) = write_temp_json("v1.json", v1_result(80.0));
        let run_file = conformance_path.with_file_name("run.json");
        let mut scenario = base_scenario();
        scenario
            .neutrality_rules
            .push("framework:specific evidence required".to_string());
        let run = base_run("v1.json".to_string());
        let result = score_run(&scenario, &run, &run_file).expect("score run");
        assert!(!result.ranked);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("framework-specific")));
    }

    #[test]
    fn unknown_run_record_fields_are_rejected() {
        let mut value: Value =
            serde_json::from_str(include_str!("../../examples/weak-run-record.json"))
                .expect("example parses");
        value.as_object_mut().expect("run record object").insert(
            "privateOperatorState".to_string(),
            json!({ "hidden": true }),
        );
        let err = serde_json::from_value::<RunRecord>(value).expect_err("unknown field rejected");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn invalid_required_outputs_are_rejected_before_scoring() {
        let (_dir, conformance_path) = write_temp_json("v1.json", v1_result(80.0));
        let run_file = conformance_path.with_file_name("run.json");
        let mut scenario = base_scenario();
        scenario.phases[0]
            .required_outputs
            .push("framework:specific".to_string());
        let run = base_run("v1.json".to_string());
        let err = score_run(&scenario, &run, &run_file).expect_err("invalid output rejected");
        assert!(err.to_string().contains("phase.requiredOutputs"));
    }

    #[test]
    fn v2_native_track_is_rejected_until_artifact_scoring_exists() {
        let (_dir, conformance_path) = write_temp_json("v1.json", v1_result(80.0));
        let run_file = conformance_path.with_file_name("run.json");
        let mut scenario = base_scenario();
        scenario.base_track = "v2-native-artifact-score".to_string();
        let run = base_run("v1.json".to_string());
        let err = score_run(&scenario, &run, &run_file).expect_err("native track rejected");
        assert!(err
            .to_string()
            .contains("first v2 scorer supports `v1-artifact-score` only"));
    }
}
