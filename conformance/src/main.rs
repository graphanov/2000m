use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

type BoxError = Box<dyn Error + Send + Sync>;
type CheckResult = Result<String, BoxError>;

const PROTOCOL_VERSION: &str = "2000m.driver.v0";
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    driver: DriverManifest,
    language: String,
}

#[derive(Debug, Deserialize)]
struct DriverManifest {
    command: String,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GameState {
    skier: Skier,
    #[serde(rename = "distanceM")]
    distance_m: f64,
    style: f64,
    #[serde(default)]
    obstacles: Vec<Obstacle>,
    monster: Option<Monster>,
    #[serde(rename = "gameOver")]
    game_over: bool,
    tick: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Skier {
    x: f64,
    y: f64,
    speed: f64,
    mode: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Obstacle {
    #[serde(rename = "type")]
    kind: String,
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Monster {
    x: f64,
    y: f64,
    mode: String,
}

#[derive(Debug)]
struct ProtocolState {
    state: GameState,
    state_value: Value,
    canonical: String,
}

#[derive(Debug, Serialize)]
struct SuiteResult {
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    #[serde(rename = "gameDir")]
    game_dir: String,
    determinism: Verdict,
    #[serde(rename = "passCount")]
    pass_count: usize,
    #[serde(rename = "totalAcs")]
    total_acs: usize,
    acs: Vec<AcVerdict>,
}

#[derive(Debug, Clone, Serialize)]
struct Verdict {
    pass: bool,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct AcVerdict {
    id: String,
    name: String,
    pass: bool,
    detail: String,
}

struct Harness {
    game_dir: PathBuf,
    manifest: Manifest,
}

struct DriverClient {
    child: Child,
    stdin: ChildStdin,
    stdout_rx: Receiver<Result<String, String>>,
    stderr_rx: Receiver<String>,
}

impl Drop for DriverClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl DriverClient {
    fn spawn(harness: &Harness) -> Result<Self, BoxError> {
        let executable =
            resolve_driver_command(&harness.game_dir, &harness.manifest.driver.command);
        let mut command = Command::new(executable);
        command
            .args(&harness.manifest.driver.args)
            .current_dir(&harness.game_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|err| {
            format!(
                "failed to spawn driver `{}` in `{}`: {}",
                harness.manifest.driver.command,
                harness.game_dir.display(),
                err
            )
        })?;

        let stdin = child.stdin.take().ok_or("driver stdin was not piped")?;
        let stdout = child.stdout.take().ok_or("driver stdout was not piped")?;
        let stderr = child.stderr.take().ok_or("driver stderr was not piped")?;

        let (stdout_tx, stdout_rx) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if stdout_tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        let _ = stdout_tx.send(Err(err.to_string()));
                        break;
                    }
                }
            }
        });

        let (stderr_tx, stderr_rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut buf = String::new();
            let _ = reader.read_to_string(&mut buf);
            let _ = stderr_tx.send(buf);
        });

        Ok(Self {
            child,
            stdin,
            stdout_rx,
            stderr_rx,
        })
    }

    fn send(&mut self, command: Value) -> Result<ProtocolState, BoxError> {
        let line = serde_json::to_string(&command)?;
        writeln!(self.stdin, "{}", line)?;
        self.stdin.flush()?;

        let response_line = match self.stdout_rx.recv_timeout(RESPONSE_TIMEOUT) {
            Ok(Ok(line)) => line,
            Ok(Err(err)) => return Err(format!("driver stdout read error: {}", err).into()),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let stderr = self.try_stderr();
                return Err(format!(
                    "timed out waiting for driver response to {}; stderr: {}",
                    line,
                    trim_for_error(&stderr)
                )
                .into());
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let stderr = self.try_stderr();
                return Err(format!(
                    "driver exited before responding to {}; stderr: {}",
                    line,
                    trim_for_error(&stderr)
                )
                .into());
            }
        };

        let response: Value = serde_json::from_str(&response_line).map_err(|err| {
            format!(
                "driver emitted invalid JSON response `{}`: {}",
                response_line, err
            )
        })?;
        if response.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(format!("driver returned non-ok response: {}", response_line).into());
        }
        let state_value = response
            .get("state")
            .cloned()
            .ok_or_else(|| format!("ok response missing state: {}", response_line))?;
        let state: GameState = serde_json::from_value(state_value.clone()).map_err(|err| {
            format!(
                "response state does not match GameState shape: {}; state={}",
                err, state_value
            )
        })?;
        validate_state(&state)?;
        let canonical = serde_json::to_string(&state_value)?;
        Ok(ProtocolState {
            state,
            state_value,
            canonical,
        })
    }

    fn try_stderr(&mut self) -> String {
        let mut out = String::new();
        while let Ok(chunk) = self.stderr_rx.try_recv() {
            out.push_str(&chunk);
        }
        out
    }
}

fn main() -> Result<(), BoxError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage(&args[0]);
        return Ok(());
    }

    let mut game_dir: Option<PathBuf> = None;
    let mut json_out: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json-out" => {
                i += 1;
                let path = args.get(i).ok_or("--json-out requires a path")?;
                json_out = Some(PathBuf::from(path));
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag `{}`", other).into());
            }
            path => {
                if game_dir.is_some() {
                    return Err("only one produced-game directory may be supplied".into());
                }
                game_dir = Some(PathBuf::from(path));
            }
        }
        i += 1;
    }

    let game_dir = game_dir.ok_or("missing produced-game directory")?;
    let harness = Harness::load(&game_dir)?;
    let result = run_suite(&harness);
    let human = render_human_summary(&result);
    let machine = serde_json::to_string_pretty(&result)?;

    if let Some(path) = json_out {
        fs::write(&path, format!("{}\n", machine))?;
        println!("{}", human);
        println!("Machine JSON written to {}", path.display());
    } else {
        eprintln!("{}", human);
        println!("{}", machine);
    }

    if !result.determinism.pass {
        std::process::exit(1);
    }
    Ok(())
}

fn print_usage(bin: &str) {
    eprintln!(
        "Usage: {} <produced-game-dir> [--json-out result.json]",
        bin
    );
    eprintln!("Scores a produced Rust game via its 2000m.json subprocess driver manifest.");
}

impl Harness {
    fn load(game_dir: &Path) -> Result<Self, BoxError> {
        let game_dir = game_dir.canonicalize().map_err(|err| {
            format!(
                "failed to resolve produced-game directory `{}`: {}",
                game_dir.display(),
                err
            )
        })?;
        let manifest_path = game_dir.join("2000m.json");
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
        if manifest.protocol_version != PROTOCOL_VERSION {
            return Err(format!(
                "unsupported protocolVersion `{}`; expected `{}`",
                manifest.protocol_version, PROTOCOL_VERSION
            )
            .into());
        }
        if manifest.language != "rust" {
            return Err(format!(
                "unsupported language `{}`; 2000m v0 produced games must declare `rust`",
                manifest.language
            )
            .into());
        }
        Ok(Self { game_dir, manifest })
    }
}

fn run_suite(harness: &Harness) -> SuiteResult {
    let determinism = match check_determinism(harness) {
        Ok(detail) => Verdict { pass: true, detail },
        Err(err) => Verdict {
            pass: false,
            detail: err.to_string(),
        },
    };

    let mut acs = Vec::new();
    acs.push(run_ac(
        harness,
        "AC1",
        "skier entity with position state",
        ac1_skier_exists,
    ));
    acs.push(run_ac(
        harness,
        "AC2",
        "steering moves skier.x deterministically",
        ac2_steering_moves,
    ));
    acs.push(run_ac(
        harness,
        "AC3",
        "slope scrolls while skiing",
        ac3_slope_scrolls,
    ));
    acs.push(run_ac(
        harness,
        "AC4",
        "slope wraps horizontally",
        ac4_horizontal_wrap,
    ));
    acs.push(run_ac(
        harness,
        "AC5",
        "seeded obstacle field",
        ac5_seeded_obstacles,
    ));
    acs.push(run_ac(
        harness,
        "AC6",
        "tree/stump collision crashes and halts",
        ac6_collision_crashes,
    ));
    acs.push(run_ac(
        harness,
        "AC7",
        "crash recovery returns to skiing",
        ac7_crash_recovery,
    ));
    acs.push(run_ac(
        harness,
        "AC8",
        "straight downhill acceleration to a cap",
        ac8_speed_cap,
    ));
    acs.push(run_ac(
        harness,
        "AC9",
        "boost exceeds normal speed cap",
        ac9_boost_exceeds_cap,
    ));
    acs.push(run_ac(
        harness,
        "AC10",
        "ramp causes airborne then landing",
        ac10_ramp_airborne_land,
    ));
    acs.push(run_ac(
        harness,
        "AC11",
        "style changes on landing and crash",
        ac11_style_scoring,
    ));
    acs.push(run_ac(
        harness,
        "AC12",
        "monster spawns at distance >= 2000m",
        ac12_monster_spawns,
    ));
    acs.push(run_ac(
        harness,
        "AC13",
        "monster pursues skier",
        ac13_monster_pursues,
    ));
    acs.push(run_ac(
        harness,
        "AC14",
        "monster contact eats skier and ends game",
        ac14_monster_eats,
    ));
    acs.push(run_ac(
        harness,
        "AC15",
        "monster flees after eating",
        ac15_monster_flees,
    ));
    acs.push(run_ac(
        harness,
        "AC16",
        "reset restores state and seed stream",
        ac16_reset_reproducible,
    ));

    let pass_count = acs.iter().filter(|ac| ac.pass).count();
    SuiteResult {
        protocol_version: PROTOCOL_VERSION.to_string(),
        game_dir: harness.game_dir.display().to_string(),
        determinism,
        pass_count,
        total_acs: acs.len(),
        acs,
    }
}

fn run_ac(
    harness: &Harness,
    id: &str,
    name: &str,
    check: fn(&Harness) -> CheckResult,
) -> AcVerdict {
    match check(harness) {
        Ok(detail) => AcVerdict {
            id: id.to_string(),
            name: name.to_string(),
            pass: true,
            detail,
        },
        Err(err) => AcVerdict {
            id: id.to_string(),
            name: name.to_string(),
            pass: false,
            detail: err.to_string(),
        },
    }
}

fn render_human_summary(result: &SuiteResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("2000m conformance: {}\n", result.game_dir));
    out.push_str(&format!(
        "Determinism: {} — {}\n",
        if result.determinism.pass {
            "PASS"
        } else {
            "FAIL"
        },
        result.determinism.detail
    ));
    out.push_str(&format!(
        "Mechanical ACs: {}/{} passed\n",
        result.pass_count, result.total_acs
    ));
    for ac in &result.acs {
        out.push_str(&format!(
            "  {} {:4} — {}: {}\n",
            if ac.pass { "PASS" } else { "FAIL" },
            ac.id,
            ac.name,
            ac.detail
        ));
    }
    out.trim_end().to_string()
}

fn check_determinism(harness: &Harness) -> CheckResult {
    let first = collect_determinism_stream(harness)?;
    let second = collect_determinism_stream(harness)?;
    if first == second {
        Ok(format!(
            "{} canonical GameState snapshots matched across two processes",
            first.len()
        ))
    } else {
        let idx = first
            .iter()
            .zip(second.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| first.len().min(second.len()));
        Err(format!(
            "stream mismatch at snapshot {}; first={}, second={}",
            idx,
            first
                .get(idx)
                .cloned()
                .unwrap_or_else(|| "<missing>".to_string()),
            second
                .get(idx)
                .cloned()
                .unwrap_or_else(|| "<missing>".to_string())
        )
        .into())
    }
}

fn collect_determinism_stream(harness: &Harness) -> Result<Vec<String>, BoxError> {
    let mut client = DriverClient::spawn(harness)?;
    let mut stream = Vec::new();
    stream.push(init(&mut client, 4242, None)?.canonical);
    for tick in 0..80 {
        let steer = match tick % 9 {
            0 | 1 => -1,
            2 | 3 | 4 => 0,
            _ => 1,
        };
        let boost = tick % 13 == 0;
        let jump = tick % 17 == 0;
        stream.push(step(&mut client, steer, boost, jump)?.canonical);
        if tick % 19 == 0 {
            stream.push(state(&mut client)?.canonical);
        }
    }
    stream.push(reset(&mut client, 4242)?.canonical);
    for tick in 0..20 {
        let steer = if tick % 2 == 0 { 1 } else { -1 };
        stream.push(step(&mut client, steer, false, false)?.canonical);
    }
    Ok(stream)
}

fn ac1_skier_exists(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let s = init(&mut client, 1, None)?.state;
    if s.tick != 0 {
        return Err(format!("init tick was {}, expected 0", s.tick).into());
    }
    Ok(format!(
        "init returned skier x={} y={} speed={} mode={}",
        s.skier.x, s.skier.y, s.skier.speed, s.skier.mode
    ))
}

fn ac2_steering_moves(harness: &Harness) -> CheckResult {
    let mut right = DriverClient::spawn(harness)?;
    let start = init(&mut right, 2, None)?.state.skier.x;
    let mut right_x = start;
    for _ in 0..5 {
        right_x = step(&mut right, 1, false, false)?.state.skier.x;
    }

    let mut left = DriverClient::spawn(harness)?;
    let left_start = init(&mut left, 2, None)?.state.skier.x;
    let mut left_x = left_start;
    for _ in 0..5 {
        left_x = step(&mut left, -1, false, false)?.state.skier.x;
    }

    if right_x <= start + 0.1 {
        return Err(format!(
            "right steering did not increase x enough: start={}, end={}",
            start, right_x
        )
        .into());
    }
    if left_x >= left_start - 0.1 {
        return Err(format!(
            "left steering did not decrease x enough: start={}, end={}",
            left_start, left_x
        )
        .into());
    }
    Ok(format!(
        "right x {}→{}, left x {}→{}",
        start, right_x, left_start, left_x
    ))
}

fn ac3_slope_scrolls(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let mut prev = init(&mut client, 3, None)?.state.distance_m;
    for idx in 1..=10 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.skier.mode != "skiing" {
            return Err(format!(
                "mode became `{}` at tick {}; expected skiing",
                s.skier.mode, s.tick
            )
            .into());
        }
        if s.distance_m <= prev {
            return Err(format!(
                "distance did not strictly increase at step {}: {} <= {}",
                idx, s.distance_m, prev
            )
            .into());
        }
        prev = s.distance_m;
    }
    Ok(format!(
        "distance increased to {} after 10 neutral steps",
        prev
    ))
}

fn ac4_horizontal_wrap(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let mut prev = init(&mut client, 4, Some(json!({ "slopeWidthM": 100.0 })))?
        .state
        .skier
        .x;
    for idx in 1..=240 {
        let x = step(&mut client, 1, false, false)?.state.skier.x;
        if x < prev - 10.0 {
            return Ok(format!(
                "observed right-edge wrap at step {}: x {}→{}",
                idx, prev, x
            ));
        }
        if !x.is_finite() || x.abs() > 10_000.0 {
            return Err(format!("x escaped finite playable range at step {}: {}", idx, x).into());
        }
        prev = x;
    }
    Err("no horizontal wrap discontinuity observed after 240 right-steer ticks".into())
}

fn ac5_seeded_obstacles(harness: &Harness) -> CheckResult {
    let a = collect_obstacle_stream(harness, 501, 60)?;
    let b = collect_obstacle_stream(harness, 501, 60)?;
    let c = collect_obstacle_stream(harness, 502, 60)?;
    if a.non_empty_count == 0 {
        return Err("same-seed obstacle stream contained no obstacles".into());
    }
    if a.stream != b.stream {
        return Err("same seed produced different obstacle streams".into());
    }
    if a.stream == c.stream {
        return Err("different seeds produced identical obstacle streams".into());
    }
    Ok(format!(
        "same seed matched; different seed differed; {} snapshots had obstacles",
        a.non_empty_count
    ))
}

fn ac6_collision_crashes(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(
        &mut client,
        6,
        Some(json!({ "scenario": "collision-tree" })),
    )?;
    for _ in 0..100 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.skier.mode == "crashed" {
            let crash_distance = s.distance_m;
            let crash_tick = s.tick;
            for _ in 0..5 {
                let after = step(&mut client, 0, false, false)?.state;
                if after.distance_m > crash_distance + 0.001 {
                    return Err(format!(
                        "distance advanced while crashed: crash distance {}, later {}",
                        crash_distance, after.distance_m
                    )
                    .into());
                }
            }
            return Ok(format!(
                "crashed at tick {} and distance halted at {}",
                crash_tick, crash_distance
            ));
        }
    }
    Err("no tree/stump crash observed in collision-tree scenario within 100 ticks".into())
}

fn ac7_crash_recovery(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(
        &mut client,
        7,
        Some(json!({ "scenario": "collision-tree" })),
    )?;
    let mut crashed_at: Option<u64> = None;
    for _ in 0..240 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.skier.mode == "crashed" && crashed_at.is_none() {
            crashed_at = Some(s.tick);
        }
        if let Some(tick) = crashed_at {
            if s.tick > tick + 2 && s.skier.mode == "skiing" {
                return Ok(format!(
                    "crashed at tick {}, recovered by tick {}",
                    tick, s.tick
                ));
            }
        }
    }
    match crashed_at {
        Some(tick) => Err(format!(
            "crashed at tick {} but did not recover within 240 ticks",
            tick
        )
        .into()),
        None => Err("no crash observed, so recovery could not be verified".into()),
    }
}

fn ac8_speed_cap(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let start = init(&mut client, 8, None)?.state.skier.speed;
    let mut speeds = Vec::new();
    for _ in 0..120 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.skier.mode == "skiing" {
            speeds.push(s.skier.speed);
        }
    }
    if speeds.len() < 60 {
        return Err(format!("only {} skiing speed samples collected", speeds.len()).into());
    }
    let early = speeds[20];
    if early <= start + 0.25 {
        return Err(format!(
            "speed did not increase enough: start={}, sample20={}",
            start, early
        )
        .into());
    }
    let max_speed = speeds.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let tail = &speeds[speeds.len() - 20..];
    let tail_min = tail.iter().copied().fold(f64::INFINITY, f64::min);
    let tail_max = tail.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if tail_max - tail_min > 0.5 {
        return Err(format!(
            "speed did not settle to a cap; last-20 range was {}..{}",
            tail_min, tail_max
        )
        .into());
    }
    Ok(format!(
        "speed rose from {} to cap near {}",
        start, max_speed
    ))
}

fn ac9_boost_exceeds_cap(harness: &Harness) -> CheckResult {
    let normal = max_speed_for(harness, 9, false, 140)?;
    let boosted = max_speed_for(harness, 9, true, 80)?;
    if boosted <= normal + 0.5 {
        return Err(format!(
            "boosted max {} did not exceed normal max {} by >0.5",
            boosted, normal
        )
        .into());
    }
    Ok(format!(
        "boosted max {} exceeded normal max {}",
        boosted, normal
    ))
}

fn ac10_ramp_airborne_land(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 10, Some(json!({ "scenario": "ramp" })))?;
    let mut airborne_at: Option<u64> = None;
    for _ in 0..180 {
        let s = step(&mut client, 0, false, true)?.state;
        if s.skier.mode == "airborne" && airborne_at.is_none() {
            airborne_at = Some(s.tick);
        }
        if let Some(tick) = airborne_at {
            if s.tick > tick && s.skier.mode == "skiing" {
                return Ok(format!(
                    "airborne at tick {}, landed by tick {}",
                    tick, s.tick
                ));
            }
        }
    }
    match airborne_at {
        Some(tick) => Err(format!(
            "airborne at tick {} but did not land within 180 ticks",
            tick
        )
        .into()),
        None => Err("no airborne state observed in ramp scenario".into()),
    }
}

fn ac11_style_scoring(harness: &Harness) -> CheckResult {
    let mut ramp = DriverClient::spawn(harness)?;
    let start_style = init(&mut ramp, 11, Some(json!({ "scenario": "ramp" })))?
        .state
        .style;
    let mut airborne_seen = false;
    let mut landing_gain = false;
    let mut max_style = start_style;
    for _ in 0..180 {
        let s = step(&mut ramp, 0, false, true)?.state;
        max_style = max_style.max(s.style);
        if s.skier.mode == "airborne" {
            airborne_seen = true;
        }
        if airborne_seen && s.skier.mode == "skiing" && s.style > start_style {
            landing_gain = true;
            break;
        }
    }
    if !landing_gain {
        return Err(format!(
            "style did not increase on ramp landing; start={}, max={}",
            start_style, max_style
        )
        .into());
    }

    let mut crash = DriverClient::spawn(harness)?;
    let mut before_crash_style = init(
        &mut crash,
        11,
        Some(json!({ "scenario": "collision-tree" })),
    )?
    .state
    .style;
    for _ in 0..120 {
        let s = step(&mut crash, 0, false, false)?.state;
        if s.skier.mode != "crashed" {
            before_crash_style = s.style;
        } else if s.style < before_crash_style {
            return Ok(format!(
                "landing increased style above {}; crash deducted {}→{}",
                start_style, before_crash_style, s.style
            ));
        }
    }
    Err("style increased on landing, but no crash style deduction was observed".into())
}

fn ac12_monster_spawns(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 12, None)?;
    let mut reached_2000 = false;
    for _ in 0..3000 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.monster.is_some() && s.distance_m < 2000.0 {
            return Err(format!("monster appeared early at distance {}", s.distance_m).into());
        }
        if s.distance_m >= 2000.0 {
            reached_2000 = true;
            if s.monster.is_some() {
                return Ok(format!(
                    "monster appeared at distance {} tick {}",
                    s.distance_m, s.tick
                ));
            }
        }
    }
    if reached_2000 {
        Err("distance reached 2000m but monster remained null".into())
    } else {
        Err("did not reach 2000m within 3000 neutral ticks".into())
    }
}

fn ac13_monster_pursues(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 13, None)?;
    let mut spawn_state: Option<GameState> = None;
    for _ in 0..3000 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.monster.is_some() && s.distance_m >= 2000.0 {
            spawn_state = Some(s);
            break;
        }
    }
    let spawn = spawn_state.ok_or("monster did not spawn by 3000 ticks")?;
    let monster = spawn
        .monster
        .as_ref()
        .ok_or("spawn state missing monster")?;
    let initial_distance = point_distance(monster.x, monster.y, spawn.skier.x, spawn.skier.y);
    let mut best_distance = initial_distance;
    let mut chasing_seen = monster.mode == "chasing";
    for _ in 0..100 {
        let s = step(&mut client, 0, false, false)?.state;
        if let Some(m) = &s.monster {
            chasing_seen |= m.mode == "chasing";
            best_distance = best_distance.min(point_distance(m.x, m.y, s.skier.x, s.skier.y));
        }
    }
    if !chasing_seen {
        return Err("monster was present but never in chasing mode".into());
    }
    if best_distance >= initial_distance - 1.0 {
        return Err(format!(
            "monster did not converge: initial distance {}, best {}",
            initial_distance, best_distance
        )
        .into());
    }
    Ok(format!(
        "monster converged from distance {} to {}",
        initial_distance, best_distance
    ))
}

fn ac14_monster_eats(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(
        &mut client,
        14,
        Some(json!({ "scenario": "monster-contact" })),
    )?;
    for _ in 0..240 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.skier.mode == "eaten" && s.game_over {
            return Ok(format!("skier eaten and gameOver=true at tick {}", s.tick));
        }
    }
    Err("monster-contact scenario did not produce eaten/gameOver within 240 ticks".into())
}

fn ac15_monster_flees(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(
        &mut client,
        15,
        Some(json!({ "scenario": "monster-contact" })),
    )?;
    let mut eaten_seen = false;
    let mut first_flee_distance: Option<f64> = None;
    for _ in 0..320 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.skier.mode == "eaten" || s.game_over {
            eaten_seen = true;
        }
        if eaten_seen {
            if let Some(monster) = &s.monster {
                if monster.mode == "fleeing" {
                    let distance = point_distance(monster.x, monster.y, s.skier.x, s.skier.y);
                    if let Some(first) = first_flee_distance {
                        if distance > first + 1.0 {
                            return Ok(format!(
                                "fleeing monster moved away: distance {}→{}",
                                first, distance
                            ));
                        }
                    } else {
                        first_flee_distance = Some(distance);
                    }
                }
            }
        }
    }
    if !eaten_seen {
        Err("skier was never eaten, so flee behavior could not be checked".into())
    } else {
        Err("monster did not enter fleeing mode and move away after eating".into())
    }
}

fn ac16_reset_reproducible(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 16, None)?;
    for tick in 0..60 {
        let steer = if tick % 3 == 0 {
            1
        } else if tick % 3 == 1 {
            -1
        } else {
            0
        };
        step(&mut client, steer, tick % 11 == 0, tick % 17 == 0)?;
    }
    let reset_state = reset(&mut client, 16)?.state;
    if reset_state.tick != 0 {
        return Err(format!("reset tick was {}, expected 0", reset_state.tick).into());
    }
    if reset_state.distance_m.abs() > 0.001 {
        return Err(format!("reset distance was {}, expected 0", reset_state.distance_m).into());
    }
    if reset_state.style.abs() > 0.001 {
        return Err(format!("reset style was {}, expected 0", reset_state.style).into());
    }
    if reset_state.monster.is_some() {
        return Err("reset monster was non-null".into());
    }
    if reset_state.game_over {
        return Err("reset gameOver was true".into());
    }

    let reset_stream = collect_after_existing_reset_stream(&mut client, 30)?;
    let fresh_stream = collect_fresh_stream(harness, 16, 30)?;
    if reset_stream.stream != fresh_stream.stream {
        return Err("post-reset stream differed from fresh init stream for same seed".into());
    }
    if reset_stream.non_empty_obstacle_snapshots == 0 {
        return Err("post-reset reproducibility stream contained no seeded obstacles".into());
    }
    Ok(format!(
        "reset cleared state and matched fresh seeded stream over {} snapshots",
        reset_stream.stream.len()
    ))
}

struct ObstacleStream {
    stream: Vec<String>,
    non_empty_count: usize,
}

fn collect_obstacle_stream(
    harness: &Harness,
    seed: i64,
    steps: usize,
) -> Result<ObstacleStream, BoxError> {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, seed, None)?;
    let mut stream = Vec::with_capacity(steps);
    let mut non_empty_count = 0;
    for _ in 0..steps {
        let p = step(&mut client, 0, false, false)?;
        if !p.state.obstacles.is_empty() {
            non_empty_count += 1;
        }
        stream.push(canonical_obstacles(&p.state_value)?);
    }
    Ok(ObstacleStream {
        stream,
        non_empty_count,
    })
}

struct StreamWithObstacleCount {
    stream: Vec<String>,
    non_empty_obstacle_snapshots: usize,
}

fn collect_after_existing_reset_stream(
    client: &mut DriverClient,
    steps: usize,
) -> Result<StreamWithObstacleCount, BoxError> {
    let mut stream = Vec::new();
    let mut non_empty = 0;
    for tick in 0..steps {
        let steer = if tick % 2 == 0 { 1 } else { -1 };
        let p = step(client, steer, false, false)?;
        if !p.state.obstacles.is_empty() {
            non_empty += 1;
        }
        stream.push(p.canonical);
    }
    Ok(StreamWithObstacleCount {
        stream,
        non_empty_obstacle_snapshots: non_empty,
    })
}

fn collect_fresh_stream(
    harness: &Harness,
    seed: i64,
    steps: usize,
) -> Result<StreamWithObstacleCount, BoxError> {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, seed, None)?;
    collect_after_existing_reset_stream(&mut client, steps)
}

fn max_speed_for(
    harness: &Harness,
    seed: i64,
    boost: bool,
    steps_count: usize,
) -> Result<f64, BoxError> {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, seed, None)?;
    let mut max_speed = f64::NEG_INFINITY;
    for _ in 0..steps_count {
        let s = step(&mut client, 0, boost, false)?.state;
        max_speed = max_speed.max(s.skier.speed);
    }
    if !max_speed.is_finite() {
        return Err("max speed was not finite".into());
    }
    Ok(max_speed)
}

fn init(
    client: &mut DriverClient,
    seed: i64,
    config: Option<Value>,
) -> Result<ProtocolState, BoxError> {
    let mut command = json!({ "cmd": "init", "seed": seed });
    if let Some(config) = config {
        command
            .as_object_mut()
            .ok_or("internal init command was not an object")?
            .insert("config".to_string(), config);
    }
    client.send(command)
}

fn step(
    client: &mut DriverClient,
    steer: i64,
    boost: bool,
    jump: bool,
) -> Result<ProtocolState, BoxError> {
    client.send(json!({
        "cmd": "step",
        "input": { "steer": steer, "boost": boost, "jump": jump }
    }))
}

fn state(client: &mut DriverClient) -> Result<ProtocolState, BoxError> {
    client.send(json!({ "cmd": "state" }))
}

fn reset(client: &mut DriverClient, seed: i64) -> Result<ProtocolState, BoxError> {
    client.send(json!({ "cmd": "reset", "seed": seed }))
}

fn validate_state(s: &GameState) -> Result<(), BoxError> {
    if !s.skier.x.is_finite()
        || !s.skier.y.is_finite()
        || !s.skier.speed.is_finite()
        || !s.distance_m.is_finite()
        || !s.style.is_finite()
    {
        return Err("GameState contains non-finite skier/distance/style number".into());
    }
    if !matches!(
        s.skier.mode.as_str(),
        "skiing" | "crashed" | "airborne" | "eaten"
    ) {
        return Err(format!("invalid skier.mode `{}`", s.skier.mode).into());
    }
    for obstacle in &s.obstacles {
        if !matches!(
            obstacle.kind.as_str(),
            "tree" | "bigtree" | "stump" | "mogul" | "rock" | "ramp"
        ) {
            return Err(format!("invalid obstacle.type `{}`", obstacle.kind).into());
        }
        if !obstacle.x.is_finite() || !obstacle.y.is_finite() {
            return Err("obstacle contains non-finite coordinate".into());
        }
    }
    if let Some(monster) = &s.monster {
        if !matches!(monster.mode.as_str(), "chasing" | "eating" | "fleeing") {
            return Err(format!("invalid monster.mode `{}`", monster.mode).into());
        }
        if !monster.x.is_finite() || !monster.y.is_finite() {
            return Err("monster contains non-finite coordinate".into());
        }
    }
    Ok(())
}

fn canonical_obstacles(state_value: &Value) -> Result<String, BoxError> {
    let obstacles = state_value
        .get("obstacles")
        .ok_or("state missing obstacles field")?;
    Ok(serde_json::to_string(obstacles)?)
}

fn point_distance(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
}

fn resolve_driver_command(game_dir: &Path, command: &str) -> PathBuf {
    let path = Path::new(command);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    if command.contains('/') || command.contains('\\') {
        game_dir.join(path)
    } else {
        PathBuf::from(command)
    }
}

fn trim_for_error(stderr: &str) -> String {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    const LIMIT: usize = 600;
    if trimmed.len() > LIMIT {
        format!("{}…", &trimmed[..LIMIT])
    } else {
        trimmed.to_string()
    }
}
