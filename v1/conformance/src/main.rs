// 2000m Conformance Suite v1
// 28 acceptance criteria with quality scoring across 3 tiers
//
// Scoring model:
// - Deterministic AC pass rate (70%): skipped ACs count as zero
// - Quality average (30%): 0-100 per AC, including skipped ACs as zero
//
// Standalone v1 scoring intentionally excludes host wall-clock timing,
// LOC efficiency, random OS state, and trajectory/convergence bonuses.

#![allow(clippy::vec_init_then_push)]

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

type BoxError = Box<dyn Error + Send + Sync>;
type CheckResult = Result<AcVerdict, BoxError>;

const PROTOCOL_VERSION: &str = "2000m.driver.v1";
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

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
    #[serde(default)]
    events: Vec<String>,
    #[serde(default)]
    quality: Option<QualityMetrics>,
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
    #[serde(default)]
    width: Option<f64>,
    #[serde(default)]
    height: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Monster {
    x: f64,
    y: f64,
    mode: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct QualityMetrics {
    #[serde(rename = "tickNanos")]
    tick_nanos: Option<i64>,
    #[serde(rename = "collisionChecks")]
    collision_checks: Option<i64>,
    #[serde(rename = "collisionHits")]
    collision_hits: Option<i64>,
    #[serde(rename = "activeObjects")]
    active_objects: Option<i64>,
    #[serde(rename = "memoryBytes")]
    memory_bytes: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ProfileMetrics {
    #[serde(rename = "windowTicks")]
    window_ticks: u64,
    #[serde(rename = "avgTickNanos")]
    avg_tick_nanos: i64,
    #[serde(rename = "maxTickNanos")]
    max_tick_nanos: i64,
    #[serde(rename = "p95TickNanos")]
    p95_tick_nanos: i64,
    #[serde(rename = "p99TickNanos")]
    p99_tick_nanos: i64,
    #[serde(rename = "totalAllocations")]
    total_allocations: i64,
    #[serde(rename = "peakMemoryBytes")]
    peak_memory_bytes: i64,
    #[serde(rename = "collisionChecksTotal")]
    collision_checks_total: i64,
    #[serde(rename = "collisionHitsTotal")]
    collision_hits_total: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ReplayData {
    seed: i64,
    #[serde(rename = "startTick")]
    start_tick: u64,
    #[serde(rename = "endTick")]
    end_tick: u64,
    #[serde(rename = "inputSequence")]
    input_sequence: String,
    #[serde(rename = "stateChecksum")]
    state_checksum: String,
}

#[derive(Debug)]
struct ProtocolState {
    state: GameState,
    state_value: Value,
    canonical: String,
}

fn canonical_state_value(mut value: Value) -> Value {
    if let Some(obj) = value.as_object_mut() {
        // `quality` is optional telemetry. It can include timings, allocation
        // counters, or other runtime observations that are useful for scoring
        // but should not affect gameplay determinism/replay canonicalization.
        obj.remove("quality");
    }
    value
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
    #[serde(rename = "compositeScore")]
    composite_score: f64,
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
    skipped: bool,
    quality: u8,
    detail: String,
    breakdown: QualityBreakdown,
}

#[derive(Debug, Clone, Serialize)]
struct QualityBreakdown {
    basic: u8,
    precision: u8,
    performance: u8,
    polish: u8,
}

impl QualityBreakdown {
    fn composite(&self) -> u8 {
        let score = (self.basic as f64 * 0.4)
            + (self.precision as f64 * 0.2)
            + (self.performance as f64 * 0.2)
            + (self.polish as f64 * 0.2);
        score.round() as u8
    }
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
        let canonical = serde_json::to_string(&canonical_state_value(state_value.clone()))?;
        Ok(ProtocolState {
            state,
            state_value,
            canonical,
        })
    }

    fn send_raw(&mut self, command: Value) -> Result<Value, BoxError> {
        let line = serde_json::to_string(&command)?;
        writeln!(self.stdin, "{}", line)?;
        self.stdin.flush()?;

        let response_line = match self.stdout_rx.recv_timeout(RESPONSE_TIMEOUT) {
            Ok(Ok(line)) => line,
            Ok(Err(err)) => return Err(format!("driver stdout read error: {}", err).into()),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err("timed out waiting for driver response".into());
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("driver exited before responding".into());
            }
        };

        let response: Value = serde_json::from_str(&response_line)
            .map_err(|err| format!("driver emitted invalid JSON: {}", err))?;
        if response.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(format!("driver returned non-ok response: {}", response_line).into());
        }
        Ok(response)
    }

    fn try_stderr(&mut self) -> String {
        let mut out = String::new();
        while let Ok(chunk) = self.stderr_rx.try_recv() {
            out.push_str(&chunk);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Protocol commands
// ---------------------------------------------------------------------------

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

fn challenge(
    client: &mut DriverClient,
    name: &str,
    params: &Value,
) -> Result<ProtocolState, BoxError> {
    client.send(json!({
        "cmd": "challenge",
        "name": name,
        "params": params
    }))
}

fn profile(client: &mut DriverClient, window: Option<u64>) -> Result<ProfileMetrics, BoxError> {
    let mut cmd = json!({ "cmd": "profile" });
    if let Some(w) = window {
        cmd["window"] = json!(w);
    }
    let response = client.send_raw(cmd)?;
    let metrics = response
        .get("metrics")
        .ok_or("profile response missing metrics")?;
    let parsed: ProfileMetrics = serde_json::from_value(metrics.clone())?;
    Ok(parsed)
}

fn replay(client: &mut DriverClient, ticks: u64) -> Result<ReplayData, BoxError> {
    let response = client.send_raw(json!({ "cmd": "replay", "ticks": ticks }))?;
    let replay = response
        .get("replay")
        .ok_or("replay response missing replay field")?;
    let parsed: ReplayData = serde_json::from_value(replay.clone())?;
    Ok(parsed)
}

fn decode_replay_inputs(encoded: &str) -> Result<Vec<(i64, bool, bool)>, BoxError> {
    let bytes = BASE64
        .decode(encoded)
        .map_err(|err| format!("replay inputSequence was not valid base64: {}", err))?;
    let mut inputs = Vec::with_capacity(bytes.len());
    for (idx, byte) in bytes.into_iter().enumerate() {
        if byte & !0b1111 != 0 {
            return Err(format!(
                "replay input byte {} used reserved bits: 0x{:02x}",
                idx, byte
            )
            .into());
        }
        let steer = match byte & 0b11 {
            0 => -1,
            1 => 0,
            2 => 1,
            other => {
                return Err(format!(
                    "replay input byte {} used invalid steer bits: {}",
                    idx, other
                )
                .into())
            }
        };
        let boost = byte & 0b0100 != 0;
        let jump = byte & 0b1000 != 0;
        inputs.push((steer, boost, jump));
    }
    Ok(inputs)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const CRASH_OBSTACLES: [&str; 4] = ["tree", "bigtree", "stump", "rock"];

fn default_obstacle_width(kind: &str) -> f64 {
    match kind {
        "tree" => 2.0,
        "bigtree" => 4.0,
        "stump" => 1.5,
        "rock" => 3.0,
        "mogul" => 2.0,
        "ramp" => 4.0,
        _ => 2.0,
    }
}

fn obstacle_width(obs: &Obstacle) -> f64 {
    obs.width
        .unwrap_or_else(|| default_obstacle_width(&obs.kind))
}

fn steer_toward_obstacle(state: &GameState, types: &[&str]) -> i64 {
    let target = state
        .obstacles
        .iter()
        .filter(|o| types.contains(&o.kind.as_str()) && o.y >= state.skier.y)
        .min_by(|a, b| {
            (a.y - state.skier.y)
                .partial_cmp(&(b.y - state.skier.y))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    match target {
        Some(o) => {
            let dx = o.x - state.skier.x;
            if dx > 0.25 {
                1
            } else if dx < -0.25 {
                -1
            } else {
                0
            }
        }
        None => 0,
    }
}

fn steer_near_obstacle_margin(state: &GameState, types: &[&str]) -> i64 {
    let target = state
        .obstacles
        .iter()
        .filter(|o| types.contains(&o.kind.as_str()) && o.y >= state.skier.y)
        .min_by(|a, b| {
            (a.y - state.skier.y)
                .partial_cmp(&(b.y - state.skier.y))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    match target {
        Some(o) => {
            let half_width = obstacle_width(o) / 2.0;
            let clearance = half_width + 0.5;
            let side = if state.skier.x <= o.x { -1.0 } else { 1.0 };
            let target_x = o.x + side * clearance;
            let dx = target_x - state.skier.x;
            if dx > 0.25 {
                1
            } else if dx < -0.25 {
                -1
            } else {
                0
            }
        }
        None => 0,
    }
}

fn steer_away_from_obstacle(state: &GameState) -> i64 {
    let threat = state
        .obstacles
        .iter()
        .filter(|o| {
            o.y >= state.skier.y && o.y - state.skier.y < 16.0 && (o.x - state.skier.x).abs() < 4.0
        })
        .min_by(|a, b| {
            (a.y - state.skier.y)
                .partial_cmp(&(b.y - state.skier.y))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    match threat {
        Some(o) => {
            if o.x >= state.skier.x {
                -1
            } else {
                1
            }
        }
        None => 0,
    }
}

fn steer_away_from_monster(state: &GameState) -> i64 {
    match &state.monster {
        Some(m) => {
            let dx = m.x - state.skier.x;
            if dx > 0.0 {
                -1
            } else if dx < 0.0 {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

fn point_distance(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
}

fn value_between(value: f64, start: f64, end: f64) -> bool {
    let lo = start.min(end);
    let hi = start.max(end);
    value > lo && value < hi
}

fn matching_next_obstacle<'a>(next: &'a GameState, obs: &Obstacle) -> Option<&'a Obstacle> {
    next.obstacles
        .iter()
        .filter(|candidate| candidate.kind == obs.kind && (candidate.x - obs.x).abs() < 5.0)
        .min_by(|a, b| {
            point_distance(obs.x, obs.y, a.x, a.y)
                .partial_cmp(&point_distance(obs.x, obs.y, b.x, b.y))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn obstacle_crossed_skier_path(previous: &GameState, next: &GameState, obs: &Obstacle) -> bool {
    let skier_dy = next.skier.y - previous.skier.y;

    if skier_dy.abs() > 1e-6 {
        return value_between(obs.y, previous.skier.y, next.skier.y);
    }

    // Stationary screen-space skier: compare the obstacle's own screen-space
    // motion against the stationary skier y coordinate. This avoids mixing
    // cumulative distanceM with screen-relative obstacle y values.
    if let Some(next_obs) = matching_next_obstacle(next, obs) {
        if value_between(previous.skier.y, obs.y, next_obs.y) {
            return true;
        }
    }

    // Fallback for drivers that keep skier.y fixed but report obstacle y in
    // world/downhill coordinates rather than screen-relative coordinates.
    value_between(
        obs.y,
        previous.distance_m + previous.skier.y,
        next.distance_m + next.skier.y,
    )
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

fn max_speed_for(
    harness: &Harness,
    seed: i64,
    boost: bool,
    steps_count: usize,
) -> Result<f64, BoxError> {
    let mut client = DriverClient::spawn(harness)?;
    let mut s = init(&mut client, seed, None)?.state;
    let mut max_speed = f64::NEG_INFINITY;
    for _ in 0..steps_count {
        let steer = steer_away_from_obstacle(&s);
        s = step(&mut client, steer, boost, false)?.state;
        if s.skier.mode == "skiing" {
            max_speed = max_speed.max(s.skier.speed);
        }
    }
    if !max_speed.is_finite() {
        return Err("max speed was not finite".into());
    }
    Ok(max_speed)
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
    if trimmed.len() > 500 {
        format!("{}...", &trimmed[..500])
    } else {
        trimmed.to_string()
    }
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

struct ObstacleStream {
    stream: Vec<String>,
    non_empty_count: usize,
}

fn canonical_obstacles(state_value: &Value) -> Result<String, BoxError> {
    let obstacles = state_value
        .get("obstacles")
        .ok_or("state missing obstacles field")?;
    Ok(serde_json::to_string(obstacles)?)
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

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
    eprintln!("Scores a produced Rust game via its 2000m.json subprocess driver manifest (v1).");
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
                "unsupported language `{}`; 2000m v1 produced games must declare `rust`",
                manifest.language
            )
            .into());
        }
        Ok(Self { game_dir, manifest })
    }
}

// ---------------------------------------------------------------------------
// Suite runner
// ---------------------------------------------------------------------------

fn run_suite(harness: &Harness) -> SuiteResult {
    let determinism = match check_determinism(harness) {
        Ok(detail) => Verdict { pass: true, detail },
        Err(err) => Verdict {
            pass: false,
            detail: err.to_string(),
        },
    };

    let mut acs = Vec::new();

    // Tier 1: Core Mechanics (AC1-AC16)
    acs.push(run_ac(
        harness,
        "AC1",
        "skier entity with position state",
        ac1_skier_exists,
    ));
    acs.push(run_ac(
        harness,
        "AC2",
        "steering moves skier deterministically",
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
        "horizontal wrap",
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
        "collision detection",
        ac6_collision_crashes,
    ));
    acs.push(run_ac(harness, "AC7", "crash recovery", ac7_crash_recovery));
    acs.push(run_ac(harness, "AC8", "speed cap", ac8_speed_cap));
    acs.push(run_ac(
        harness,
        "AC9",
        "boost exceeds normal cap",
        ac9_boost_exceeds_cap,
    ));
    acs.push(run_ac(
        harness,
        "AC10",
        "ramp airborne and landing",
        ac10_ramp_airborne_land,
    ));
    acs.push(run_ac(harness, "AC11", "style scoring", ac11_style_scoring));
    acs.push(run_ac(
        harness,
        "AC12",
        "monster spawns at 2000m",
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
        "monster eats skier",
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
        "reset reproducible",
        ac16_reset_reproducible,
    ));

    // Tier 2: Edge Cases and Performance (AC17-AC22)
    acs.push(run_ac(
        harness,
        "AC17",
        "high-speed tunneling prevention",
        ac17_tunneling_prevention,
    ));
    acs.push(run_ac(
        harness,
        "AC18",
        "dense field performance",
        ac18_dense_field_performance,
    ));
    acs.push(run_ac(
        harness,
        "AC19",
        "monster pursuit under evasion",
        ac19_monster_evasion,
    ));
    acs.push(run_ac(
        harness,
        "AC20",
        "determinism over long runs",
        ac20_long_determinism,
    ));
    acs.push(run_ac(
        harness,
        "AC21",
        "crash recovery under load",
        ac21_crash_under_load,
    ));
    acs.push(run_ac(
        harness,
        "AC22",
        "monster spawn timing precision",
        ac22_spawn_precision,
    ));

    // Tier 3: Polish and Optimization (AC23-AC28)
    acs.push(run_ac(
        harness,
        "AC23",
        "input responsiveness",
        ac23_input_responsiveness,
    ));
    acs.push(run_ac(
        harness,
        "AC24",
        "collision forgiveness",
        ac24_collision_forgiveness,
    ));
    acs.push(run_ac(
        harness,
        "AC25",
        "animation smoothness",
        ac25_animation_smoothness,
    ));
    acs.push(run_ac(
        harness,
        "AC26",
        "deterministic replay accuracy",
        ac26_replay_accuracy,
    ));
    acs.push(run_ac(
        harness,
        "AC27",
        "performance budget",
        ac27_performance_budget,
    ));
    acs.push(run_ac(harness, "AC28", "visual polish", ac28_visual_polish));

    let pass_count = acs.iter().filter(|ac| ac.pass && !ac.skipped).count();
    let total = acs.len();

    // Rank/composite scoring deliberately treats skipped ACs as zero-score ACs.
    // Unsupported challenges are useful diagnostics, but they must not inflate
    // a contender's ranked denominator by opting out of hard checks.
    let quality_avg = if total == 0 {
        0.0
    } else {
        acs.iter()
            .map(|ac| if ac.skipped { 0.0 } else { ac.quality as f64 })
            .sum::<f64>()
            / total as f64
    };

    let pass_rate = if total == 0 {
        0.0
    } else {
        pass_count as f64 / total as f64
    };

    // Standalone v1 composite has no separate LOC, OS-state, external timing,
    // efficiency, or trajectory/convergence term. Multi-generation result
    // repositories may layer separate trajectory fields on top. AC-level
    // pass/quality can still be based on labeled host-bound probes or
    // driver-reported telemetry, so result evidence must keep those sources
    // explicit.
    let composite_score = pass_rate * 70.0 + quality_avg * 0.3;

    SuiteResult {
        protocol_version: PROTOCOL_VERSION.to_string(),
        game_dir: harness.game_dir.display().to_string(),
        determinism,
        pass_count,
        total_acs: total,
        composite_score,
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
        Ok(verdict) => verdict,
        Err(err) => AcVerdict {
            id: id.to_string(),
            name: name.to_string(),
            pass: false,
            skipped: false,
            quality: 0,
            detail: err.to_string(),
            breakdown: QualityBreakdown {
                basic: 0,
                precision: 0,
                performance: 0,
                polish: 0,
            },
        },
    }
}

fn render_human_summary(result: &SuiteResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("2000m conformance v1: {}\n", result.game_dir));
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
    out.push_str(&format!("Composite Score: {:.1}\n", result.composite_score));
    for ac in &result.acs {
        let pass_str = if ac.skipped {
            "SKIP"
        } else if ac.pass {
            "PASS"
        } else {
            "FAIL"
        };
        out.push_str(&format!(
            "  {} {:4} (Q:{:3}) — {}: {}\n",
            pass_str, ac.id, ac.quality, ac.name, ac.detail
        ));
    }
    out.trim_end().to_string()
}

// ---------------------------------------------------------------------------
// Determinism check (pre-AC)
// ---------------------------------------------------------------------------

fn check_determinism(harness: &Harness) -> Result<String, BoxError> {
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
            2..=4 => 0,
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

// ===========================================================================
// TIER 1: Core Mechanics (AC1-AC16)
// ===========================================================================

fn ac1_skier_exists(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let s = init(&mut client, 1, None)?.state;
    if s.tick != 0 {
        return Err(format!("init tick was {}, expected 0", s.tick).into());
    }

    let basic = 100u8;
    let precision = if s.skier.x.abs() < 1000.0 && s.skier.y.abs() < 10.0 {
        90
    } else {
        50
    };
    let performance = 70;
    let polish = 60;
    let breakdown = QualityBreakdown {
        basic,
        precision,
        performance,
        polish,
    };

    Ok(AcVerdict {
        id: "AC1".to_string(),
        name: "skier entity with position state".to_string(),
        pass: true,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "init returned skier x={} y={} speed={} mode={}",
            s.skier.x, s.skier.y, s.skier.speed, s.skier.mode
        ),
        breakdown,
    })
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

    let right_delta = right_x - start;
    let left_delta = left_start - left_x;

    if right_delta <= 0.1 {
        return Err(format!(
            "right steering did not increase x enough: start={}, end={}",
            start, right_x
        )
        .into());
    }
    if left_delta <= 0.1 {
        return Err(format!(
            "left steering did not decrease x enough: start={}, end={}",
            left_start, left_x
        )
        .into());
    }

    // Quality: symmetry check (left/right mirror within 10%)
    let symmetry = if right_delta > 0.0 && left_delta > 0.0 {
        let ratio = (right_delta / left_delta).max(left_delta / right_delta);
        if ratio < 1.1 {
            95
        } else if ratio < 1.5 {
            80
        } else {
            60
        }
    } else {
        50
    };

    let breakdown = QualityBreakdown {
        basic: 100,
        precision: symmetry,
        performance: 80,
        polish: 70,
    };

    Ok(AcVerdict {
        id: "AC2".to_string(),
        name: "steering moves skier deterministically".to_string(),
        pass: true,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "right x {}→{} (+{:.1}), left x {}→{} (-{:.1})",
            start, right_x, right_delta, left_start, left_x, left_delta
        ),
        breakdown,
    })
}

fn ac3_slope_scrolls(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let mut prev = init(&mut client, 3, None)?.state.distance_m;
    let mut distances = Vec::new();

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
        distances.push(s.distance_m - prev);
        prev = s.distance_m;
    }

    // Quality: consistent increment variance
    let mean_inc = distances.iter().sum::<f64>() / distances.len() as f64;
    let variance = distances
        .iter()
        .map(|d| (d - mean_inc).powi(2))
        .sum::<f64>()
        / distances.len() as f64;
    let precision = if variance < 0.01 {
        95
    } else if variance < 0.1 {
        85
    } else {
        70
    };

    let breakdown = QualityBreakdown {
        basic: 100,
        precision,
        performance: 80,
        polish: 70,
    };

    Ok(AcVerdict {
        id: "AC3".to_string(),
        name: "slope scrolls while skiing".to_string(),
        pass: true,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "distance increased to {} after 10 neutral steps, variance={:.4}",
            prev, variance
        ),
        breakdown,
    })
}

fn ac4_horizontal_wrap(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let mut prev = init(&mut client, 4, None)?.state.skier.x;
    let mut wrap_tick = None;

    for idx in 1..=1200 {
        let x = step(&mut client, 1, false, false)?.state.skier.x;
        if x < prev - 10.0 {
            wrap_tick = Some(idx);
            break;
        }
        if !x.is_finite() || x.abs() > 10_000.0 {
            return Err(format!("x escaped finite playable range at step {}: {}", idx, x).into());
        }
        prev = x;
    }

    match wrap_tick {
        Some(tick) => {
            let breakdown = QualityBreakdown {
                basic: 100,
                precision: 90,
                performance: 80,
                polish: 70,
            };
            Ok(AcVerdict {
                id: "AC4".to_string(),
                name: "horizontal wrap".to_string(),
                pass: true,
                skipped: false,
                quality: breakdown.composite(),
                detail: format!("wrap observed at step {}", tick),
                breakdown,
            })
        }
        None => {
            Err("no horizontal wrap discontinuity observed after 1200 right-steer ticks".into())
        }
    }
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

    // Quality: count obstacle types used
    let breakdown = QualityBreakdown {
        basic: 100,
        precision: 85,
        performance: 80,
        polish: 75,
    };

    Ok(AcVerdict {
        id: "AC5".to_string(),
        name: "seeded obstacle field".to_string(),
        pass: true,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "same seed matched; different seed differed; {} snapshots had obstacles",
            a.non_empty_count
        ),
        breakdown,
    })
}

fn ac6_collision_crashes(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let mut s = init(&mut client, 6, None)?.state;
    for _ in 0..2000 {
        let steer = steer_toward_obstacle(&s, &CRASH_OBSTACLES);
        s = step(&mut client, steer, false, false)?.state;
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

            // Quality: check if events field reports collision
            let has_event = s.events.iter().any(|e| e == "collision" || e == "crash");
            let precision = if has_event { 95 } else { 80 };

            let breakdown = QualityBreakdown {
                basic: 100,
                precision,
                performance: 85,
                polish: if has_event { 85 } else { 60 },
            };

            return Ok(AcVerdict {
                id: "AC6".to_string(),
                name: "collision detection".to_string(),
                pass: true,
                skipped: false,
                quality: breakdown.composite(),
                detail: format!(
                    "navigated into obstacle, crashed at tick {} distance halted at {}, events={}",
                    crash_tick, crash_distance, has_event
                ),
                breakdown,
            });
        }
    }
    Err("no tree/stump crash observed after navigating into obstacles for 2000 ticks".into())
}

fn ac7_crash_recovery(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let mut s = init(&mut client, 7, None)?.state;
    let mut crashed_at: Option<u64> = None;
    let mut recovery_ticks: Vec<u64> = Vec::new();

    for _ in 0..2000 {
        let steer = if crashed_at.is_some() {
            0
        } else {
            steer_toward_obstacle(&s, &CRASH_OBSTACLES)
        };
        s = step(&mut client, steer, false, false)?.state;
        if s.skier.mode == "crashed" && crashed_at.is_none() {
            crashed_at = Some(s.tick);
        }
        if let Some(tick) = crashed_at {
            if s.tick > tick + 2 && s.skier.mode == "skiing" {
                recovery_ticks.push(s.tick - tick);
                // Found first recovery
                let recovery_time = s.tick - tick;
                let precision = if recovery_time >= 2 && recovery_time <= 10 {
                    95
                } else if recovery_time <= 20 {
                    80
                } else {
                    60
                };
                let breakdown = QualityBreakdown {
                    basic: 100,
                    precision,
                    performance: 80,
                    polish: 75,
                };
                return Ok(AcVerdict {
                    id: "AC7".to_string(),
                    name: "crash recovery".to_string(),
                    pass: true,
                    skipped: false,
                    quality: breakdown.composite(),
                    detail: format!(
                        "crashed at tick {}, recovered by tick {} ({} ticks)",
                        tick, s.tick, recovery_time
                    ),
                    breakdown,
                });
            }
        }
    }
    match crashed_at {
        Some(tick) => Err(format!(
            "crashed at tick {} but did not recover within 2000 ticks",
            tick
        )
        .into()),
        None => Err(
            "no crash observed while navigating obstacles, so recovery could not be verified"
                .into(),
        ),
    }
}

fn ac8_speed_cap(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let mut s = init(&mut client, 8, None)?.state;
    let start = s.skier.speed;
    let mut speeds = Vec::new();
    for _ in 0..240 {
        let steer = steer_away_from_obstacle(&s);
        s = step(&mut client, steer, false, false)?.state;
        if s.skier.mode == "skiing" {
            speeds.push(s.skier.speed);
        }
        if speeds.len() >= 120 {
            break;
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

    // Quality: smooth acceleration curve check
    let mut accels = Vec::new();
    for w in speeds.windows(2) {
        accels.push(w[1] - w[0]);
    }
    let accel_variance = if accels.len() > 1 {
        let mean = accels.iter().sum::<f64>() / accels.len() as f64;
        accels.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / accels.len() as f64
    } else {
        1.0
    };
    let precision = if accel_variance < 0.1 {
        95
    } else if accel_variance < 0.5 {
        85
    } else {
        70
    };

    let breakdown = QualityBreakdown {
        basic: 100,
        precision,
        performance: 80,
        polish: 70,
    };

    Ok(AcVerdict {
        id: "AC8".to_string(),
        name: "speed cap".to_string(),
        pass: true,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "speed rose from {} to cap near {}, accel variance={:.4}",
            start, max_speed, accel_variance
        ),
        breakdown,
    })
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

    let excess = boosted - normal;
    let precision = if excess > 2.0 {
        95
    } else if excess > 1.0 {
        85
    } else {
        75
    };

    let breakdown = QualityBreakdown {
        basic: 100,
        precision,
        performance: 80,
        polish: 70,
    };

    Ok(AcVerdict {
        id: "AC9".to_string(),
        name: "boost exceeds normal cap".to_string(),
        pass: true,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "boosted max {} exceeded normal max {} (excess={:.1})",
            boosted, normal, excess
        ),
        breakdown,
    })
}

fn ac10_ramp_airborne_land(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    let mut s = init(&mut client, 10, None)?.state;
    let mut airborne_at: Option<u64> = None;
    let mut airborne_speeds = Vec::new();

    for _ in 0..2000 {
        let steer = if airborne_at.is_some() {
            0
        } else {
            steer_toward_obstacle(&s, &["ramp"])
        };
        s = step(&mut client, steer, false, true)?.state;
        if s.skier.mode == "airborne" && airborne_at.is_none() {
            airborne_at = Some(s.tick);
        }
        if s.skier.mode == "airborne" {
            airborne_speeds.push(s.skier.speed);
        }
        if let Some(tick) = airborne_at {
            if s.tick > tick && s.skier.mode == "skiing" {
                // Quality: check for speed continuity (parabolic trajectory)
                let precision = if !airborne_speeds.is_empty() { 90 } else { 70 };
                let breakdown = QualityBreakdown {
                    basic: 100,
                    precision,
                    performance: 80,
                    polish: 75,
                };
                return Ok(AcVerdict {
                    id: "AC10".to_string(),
                    name: "ramp airborne and landing".to_string(),
                    pass: true,
                    skipped: false,
                    quality: breakdown.composite(),
                    detail: format!(
                        "navigated onto ramp, airborne at tick {}, landed by tick {}, {} airborne samples",
                        tick,
                        s.tick,
                        airborne_speeds.len()
                    ),
                    breakdown,
                });
            }
        }
    }
    match airborne_at {
        Some(tick) => Err(format!(
            "airborne at tick {} but did not land within 2000 ticks",
            tick
        )
        .into()),
        None => Err("no airborne state observed after navigating onto ramps".into()),
    }
}

fn ac11_style_scoring(harness: &Harness) -> CheckResult {
    let mut ramp = DriverClient::spawn(harness)?;
    let mut rs = init(&mut ramp, 11, None)?.state;
    let start_style = rs.style;
    let mut airborne_seen = false;
    let mut landing_gain = false;
    let mut max_style = start_style;
    for _ in 0..2000 {
        let steer = if airborne_seen {
            0
        } else {
            steer_toward_obstacle(&rs, &["ramp"])
        };
        rs = step(&mut ramp, steer, false, true)?.state;
        max_style = max_style.max(rs.style);
        if rs.skier.mode == "airborne" {
            airborne_seen = true;
        }
        if airborne_seen && rs.skier.mode == "skiing" && rs.style > start_style {
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
    let mut cs = init(&mut crash, 11, None)?.state;
    let mut before_crash_style = cs.style;
    for _ in 0..2000 {
        let steer = steer_toward_obstacle(&cs, &CRASH_OBSTACLES);
        cs = step(&mut crash, steer, false, false)?.state;
        if cs.skier.mode != "crashed" {
            before_crash_style = cs.style;
        } else if cs.style < before_crash_style {
            let landing_delta = max_style - start_style;
            let crash_delta = before_crash_style - cs.style;

            // Quality: risk/reward balance — landing gain should be meaningful
            let precision = if landing_delta > 5.0 && crash_delta > 2.0 {
                95
            } else if landing_delta > 1.0 {
                80
            } else {
                65
            };

            let breakdown = QualityBreakdown {
                basic: 100,
                precision,
                performance: 80,
                polish: 75,
            };

            return Ok(AcVerdict {
                id: "AC11".to_string(),
                name: "style scoring".to_string(),
                pass: true,
                skipped: false,
                quality: breakdown.composite(),
                detail: format!(
                    "landing increased style +{:.1}; crash deducted -{:.1}",
                    landing_delta, crash_delta
                ),
                breakdown,
            });
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
                let spawn_distance = s.distance_m;
                let precision = if (spawn_distance - 2000.0).abs() < 1.0 {
                    95
                } else if (spawn_distance - 2000.0).abs() < 5.0 {
                    85
                } else {
                    70
                };
                let breakdown = QualityBreakdown {
                    basic: 100,
                    precision,
                    performance: 80,
                    polish: 70,
                };
                return Ok(AcVerdict {
                    id: "AC12".to_string(),
                    name: "monster spawns at 2000m".to_string(),
                    pass: true,
                    skipped: false,
                    quality: breakdown.composite(),
                    detail: format!(
                        "monster appeared at distance {:.1} tick {}",
                        spawn_distance, s.tick
                    ),
                    breakdown,
                });
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

    let convergence = initial_distance - best_distance;
    let precision = if convergence > initial_distance * 0.8 {
        95
    } else if convergence > initial_distance * 0.5 {
        85
    } else {
        70
    };

    let breakdown = QualityBreakdown {
        basic: 100,
        precision,
        performance: 80,
        polish: 70,
    };

    Ok(AcVerdict {
        id: "AC13".to_string(),
        name: "monster pursues skier".to_string(),
        pass: true,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "monster converged from distance {:.1} to {:.1} ({:.0}%)",
            initial_distance,
            best_distance,
            convergence / initial_distance * 100.0
        ),
        breakdown,
    })
}

fn ac14_monster_eats(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 14, None)?;
    let mut spawned = false;
    for _ in 0..6000 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.monster.is_some() && s.distance_m >= 2000.0 {
            spawned = true;
        }
        if spawned && s.skier.mode == "eaten" && s.game_over {
            let has_event = s
                .events
                .iter()
                .any(|e| e.contains("eat") || e.contains("monster"));
            let breakdown = QualityBreakdown {
                basic: 100,
                precision: 90,
                performance: 80,
                polish: if has_event { 85 } else { 60 },
            };
            return Ok(AcVerdict {
                id: "AC14".to_string(),
                name: "monster eats skier".to_string(),
                pass: true,
                skipped: false,
                quality: breakdown.composite(),
                detail: format!(
                    "skier eaten and gameOver=true at tick {}, events={}",
                    s.tick, has_event
                ),
                breakdown,
            });
        }
    }
    if spawned {
        Err("monster spawned but did not eat the non-evading skier within 6000 ticks".into())
    } else {
        Err("did not reach 2000m to spawn the monster within 6000 neutral ticks".into())
    }
}

fn ac15_monster_flees(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 15, None)?;
    let mut eaten_seen = false;
    let mut first_flee_distance: Option<f64> = None;
    for _ in 0..6000 {
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
                            let flee_delta = distance - first;
                            let precision = if flee_delta > 5.0 {
                                95
                            } else if flee_delta > 2.0 {
                                85
                            } else {
                                75
                            };
                            let breakdown = QualityBreakdown {
                                basic: 100,
                                precision,
                                performance: 80,
                                polish: 75,
                            };
                            return Ok(AcVerdict {
                                id: "AC15".to_string(),
                                name: "monster flees after eating".to_string(),
                                pass: true,
                                skipped: false,
                                quality: breakdown.composite(),
                                detail: format!(
                                    "fleeing monster moved away: distance {:.1}→{:.1} (+{:.1})",
                                    first, distance, flee_delta
                                ),
                                breakdown,
                            });
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

    // Collect post-reset stream
    let mut reset_stream = Vec::new();
    let mut non_empty = 0;
    for tick in 0..30 {
        let steer = if tick % 2 == 0 { 1 } else { -1 };
        let p = step(&mut client, steer, false, false)?;
        if !p.state.obstacles.is_empty() {
            non_empty += 1;
        }
        reset_stream.push(p.canonical.clone());
    }

    // Collect fresh stream
    let mut fresh_client = DriverClient::spawn(harness)?;
    init(&mut fresh_client, 16, None)?;
    let mut fresh_stream = Vec::new();
    for tick in 0..30 {
        let steer = if tick % 2 == 0 { 1 } else { -1 };
        let p = step(&mut fresh_client, steer, false, false)?;
        fresh_stream.push(p.canonical.clone());
    }

    if reset_stream != fresh_stream {
        return Err("post-reset stream differed from fresh init stream for same seed".into());
    }
    let breakdown = QualityBreakdown {
        basic: 100,
        precision: if non_empty > 0 { 95 } else { 80 },
        performance: 85,
        polish: 80,
    };

    Ok(AcVerdict {
        id: "AC16".to_string(),
        name: "reset reproducible".to_string(),
        pass: true,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "reset cleared state and matched fresh seeded stream over {} snapshots ({} with obstacles)",
            reset_stream.len(),
            non_empty
        ),
        breakdown,
    })
}

// ===========================================================================
// TIER 2: Edge Cases and Performance (AC17-AC22)
// ===========================================================================

fn ac17_tunneling_prevention(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 17, None)?;

    // Try challenge for high speed. If unsupported and normal boosted play
    // cannot produce a swept collision/tunneling attempt, this stress AC is
    // untestable rather than failed.
    let challenge_ok = challenge(&mut client, "high_speed", &json!({"speed": 10.5})).is_ok();

    let mut tunneling_count = 0;
    let mut collision_count = 0;
    let mut s = state(&mut client)?.state;

    for _ in 0..1000 {
        let steer = steer_toward_obstacle(&s, &["tree", "rock"]);
        let next = step(&mut client, steer, true, false)?.state;

        // Detect tunneling: obstacle crossed the skier's swept path but no crash.
        for obs in &s.obstacles {
            if obstacle_crossed_skier_path(&s, &next, obs) {
                let skier_passed_near = (s.skier.x - obs.x).abs() < 3.0;
                if skier_passed_near && next.skier.mode != "crashed" {
                    tunneling_count += 1;
                }
            }
        }

        if next.skier.mode == "crashed" {
            collision_count += 1;
        }

        s = next;
    }

    if !challenge_ok && tunneling_count == 0 && collision_count == 0 {
        let breakdown = QualityBreakdown {
            basic: 0,
            precision: 0,
            performance: 0,
            polish: 0,
        };
        return Ok(AcVerdict {
            id: "AC17".to_string(),
            name: "high-speed tunneling prevention".to_string(),
            pass: false,
            skipped: true,
            quality: 0,
            detail: "skipped: high_speed challenge unsupported and fallback produced no collision/tunneling attempts".to_string(),
            breakdown,
        });
    }

    let pass = tunneling_count == 0 && collision_count > 0;
    let precision = if tunneling_count == 0 && collision_count > 0 {
        95
    } else if tunneling_count < 5 {
        60
    } else {
        20
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 0 },
        precision,
        performance: 85,
        polish: 80,
    };

    Ok(AcVerdict {
        id: "AC17".to_string(),
        name: "high-speed tunneling prevention".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "tunneling events={}, collisions={} high_speed_challenge={} over 1000 boosted ticks",
            tunneling_count, collision_count, challenge_ok
        ),
        breakdown,
    })
}

fn ac18_dense_field_performance(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 18, None)?;

    // Try challenge for dense field; if unsupported, test with normal field
    let challenge_ok =
        challenge(&mut client, "dense_field", &json!({"obstacleCount": 100})).is_ok();

    let mut frame_times_ns: Vec<f64> = Vec::new();
    let mut max_obstacles = 0usize;

    for _ in 0..500 {
        let start = Instant::now();
        let p = step(&mut client, 0, false, false)?;
        let elapsed_ns = start.elapsed().as_nanos() as f64;
        frame_times_ns.push(elapsed_ns);
        max_obstacles = max_obstacles.max(p.state.obstacles.len());
    }

    let n = frame_times_ns.len() as f64;
    let avg_ns = frame_times_ns.iter().sum::<f64>() / n;
    let avg_ms = avg_ns / 1_000_000.0;

    // Sort for percentile calculation
    let mut sorted = frame_times_ns.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95_idx = (n * 0.95) as usize;
    let p99_idx = (n * 0.99) as usize;
    let p95_ms = sorted.get(p95_idx).copied().unwrap_or(0.0) / 1_000_000.0;
    let p99_ms = sorted.get(p99_idx).copied().unwrap_or(0.0) / 1_000_000.0;

    // Check if driver reported quality metrics
    let driver_quality = {
        let mut check_client = DriverClient::spawn(harness)?;
        init(&mut check_client, 18, None)?.state.quality.is_some()
    };

    if !challenge_ok && max_obstacles < 50 {
        let breakdown = QualityBreakdown {
            basic: 0,
            precision: 0,
            performance: 0,
            polish: 0,
        };
        return Ok(AcVerdict {
            id: "AC18".to_string(),
            name: "dense field performance".to_string(),
            pass: false,
            skipped: true,
            quality: 0,
            detail: format!(
                "skipped: dense_field challenge unsupported and fallback reached only {} visible obstacles",
                max_obstacles
            ),
            breakdown,
        });
    }

    let pass = avg_ms < 16.6 && p95_ms < 20.0 && p99_ms < 30.0 && max_obstacles >= 50;
    let perf_score = if avg_ms < 5.0 {
        95
    } else if avg_ms < 10.0 {
        85
    } else if avg_ms < 16.6 {
        70
    } else {
        30
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 0 },
        precision: if challenge_ok && max_obstacles >= 50 {
            90
        } else {
            70
        },
        performance: perf_score,
        polish: if driver_quality { 85 } else { 60 },
    };

    Ok(AcVerdict {
        id: "AC18".to_string(),
        name: "dense field performance".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "timing_source=host_wall_clock avg={:.2}ms p95={:.2}ms p99={:.2}ms max_obstacles={} challenge={}",
            avg_ms, p95_ms, p99_ms, max_obstacles, challenge_ok
        ),
        breakdown,
    })
}

fn ac19_monster_evasion(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 19, None)?;

    // Try early monster spawn challenge
    let challenge_ok = challenge(
        &mut client,
        "evasion_course",
        &json!({"monsterDistance": 100}),
    )
    .is_ok();

    let mut monster_spawned = false;
    let mut evasion_ticks = 0;
    let mut monster_stuck_ticks = 0;
    let mut monster_teleport_ticks = 0;
    let mut last_monster_pos: Option<(f64, f64)> = None;

    // First, reach monster spawn if challenge didn't work
    for _ in 0..6000 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.monster.is_some() {
            monster_spawned = true;
            break;
        }
    }

    if !monster_spawned && !challenge_ok {
        return Err("monster did not spawn for evasion test".into());
    }

    // Now test evasion
    for _ in 0..200 {
        let s = state(&mut client)?.state;
        let steer = steer_away_from_monster(&s);
        let next = step(&mut client, steer, true, false)?.state;

        if let Some(m) = &next.monster {
            if let Some(last) = last_monster_pos {
                let moved = point_distance(m.x, m.y, last.0, last.1);
                if moved > 20.0 {
                    monster_teleport_ticks += 1;
                }
                if moved < 0.05 {
                    monster_stuck_ticks += 1;
                }
            }
            last_monster_pos = Some((m.x, m.y));

            let dist = point_distance(next.skier.x, next.skier.y, m.x, m.y);
            if dist > 50.0 {
                evasion_ticks += 1;
            }

            if next.skier.mode == "eaten" {
                // Eaten during evasion — still measure what we have
                break;
            }
        }
    }

    let pass = monster_stuck_ticks < 10 && monster_teleport_ticks == 0 && evasion_ticks > 50;

    let precision = if evasion_ticks > 100 {
        95
    } else if evasion_ticks > 50 {
        80
    } else {
        60
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 0 },
        precision,
        performance: 80,
        polish: if challenge_ok { 85 } else { 65 },
    };

    Ok(AcVerdict {
        id: "AC19".to_string(),
        name: "monster pursuit under evasion".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "evasion_ticks={}, stuck_ticks={}, teleport_ticks={}",
            evasion_ticks, monster_stuck_ticks, monster_teleport_ticks
        ),
        breakdown,
    })
}

fn ac20_long_determinism(harness: &Harness) -> CheckResult {
    // Run 10,000 ticks in two separate processes and compare
    let stream1 = collect_long_stream(harness, 20, 10_000)?;
    let stream2 = collect_long_stream(harness, 20, 10_000)?;

    if stream1.len() != stream2.len() {
        return Err(format!(
            "stream length mismatch: {} vs {}",
            stream1.len(),
            stream2.len()
        )
        .into());
    }

    let mut first_mismatch = None;
    for (i, (a, b)) in stream1.iter().zip(stream2.iter()).enumerate() {
        if a != b {
            first_mismatch = Some(i);
            break;
        }
    }

    match first_mismatch {
        Some(idx) => Err(format!(
            "determinism failure at tick {} of 10000; first={}, second={}",
            idx,
            &stream1[idx][..stream1[idx].len().min(100)],
            &stream2[idx][..stream2[idx].len().min(100)]
        )
        .into()),
        None => {
            // Check for floating-point drift by parsing final states
            let last1: GameState = serde_json::from_str(&stream1[stream1.len() - 1])?;
            let last2: GameState = serde_json::from_str(&stream2[stream2.len() - 1])?;
            let speed_drift = (last1.skier.speed - last2.skier.speed).abs();
            let distance_drift = (last1.distance_m - last2.distance_m).abs();

            let precision = if speed_drift < 1e-10 && distance_drift < 1e-10 {
                95
            } else if speed_drift < 1e-6 && distance_drift < 1e-6 {
                85
            } else {
                60
            };

            let breakdown = QualityBreakdown {
                basic: 100,
                precision,
                performance: 85,
                polish: 80,
            };

            Ok(AcVerdict {
                id: "AC20".to_string(),
                name: "determinism over long runs".to_string(),
                pass: true,
                skipped: false,
                quality: breakdown.composite(),
                detail: format!(
                    "{} ticks matched; speed_drift={:.2e}, distance_drift={:.2e}",
                    stream1.len(),
                    speed_drift,
                    distance_drift
                ),
                breakdown,
            })
        }
    }
}

fn collect_long_stream(
    harness: &Harness,
    seed: i64,
    ticks: usize,
) -> Result<Vec<String>, BoxError> {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, seed, None)?;
    let mut stream = Vec::with_capacity(ticks);
    for tick in 0..ticks {
        let steer = match tick % 7 {
            0 | 1 => -1,
            2..=4 => 0,
            _ => 1,
        };
        let boost = tick % 23 == 0;
        let jump = tick % 31 == 0;
        let p = step(&mut client, steer, boost, jump)?;
        stream.push(p.canonical);
    }
    Ok(stream)
}

fn ac21_crash_under_load(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 21, None)?;

    // Try crash gauntlet challenge. If unsupported and normal play cannot
    // generate enough crash cycles, this AC is untestable rather than failed.
    let challenge_ok = challenge(&mut client, "crash_gauntlet", &json!({"crashCount": 50})).is_ok();

    let mut crashes = 0u32;
    let mut recoveries = 0u32;
    let mut state_corruption = 0u32;
    let mut first_memory: Option<i64> = None;
    let mut max_memory: Option<i64> = None;
    let mut last_memory: Option<i64> = None;

    for _ in 0..5000 {
        let s = state(&mut client)?.state;
        let steer = if s.skier.mode == "crashed" {
            0
        } else {
            steer_toward_obstacle(&s, &CRASH_OBSTACLES)
        };
        let next = step(&mut client, steer, false, false)?;
        let ns = &next.state;

        if ns.skier.mode == "crashed" && s.skier.mode != "crashed" {
            crashes += 1;
            // Check for state corruption
            if !ns.skier.x.is_finite() || !ns.skier.y.is_finite() {
                state_corruption += 1;
            }
        }

        if crashes > recoveries && ns.skier.mode == "skiing" {
            recoveries += 1;
        }

        // Track memory if quality metrics are available. Negative values mean
        // unavailable and cannot prove absence of leaks.
        if let Some(q) = &ns.quality {
            if let Some(mem) = q.memory_bytes {
                if mem >= 0 {
                    if first_memory.is_none() {
                        first_memory = Some(mem);
                    }
                    max_memory = Some(max_memory.map_or(mem, |max| max.max(mem)));
                    last_memory = Some(mem);
                }
            }
        }
    }

    let memory_ok = match (first_memory, max_memory) {
        (Some(first), Some(max)) if first > 0 => max <= first + (first / 10),
        (Some(_), Some(max)) => max < 50_000_000,
        // Per-tick quality metrics are optional. Absence of memory telemetry does
        // not prove a leak, but reported growth beyond the budget fails the AC.
        _ => true,
    };

    if !challenge_ok && crashes < 50 {
        let breakdown = QualityBreakdown {
            basic: 0,
            precision: 0,
            performance: 0,
            polish: 0,
        };
        return Ok(AcVerdict {
            id: "AC21".to_string(),
            name: "crash recovery under load".to_string(),
            pass: false,
            skipped: true,
            quality: 0,
            detail: format!(
                "skipped: crash_gauntlet challenge unsupported and fallback produced only {} crash cycles",
                crashes
            ),
            breakdown,
        });
    }

    let pass = crashes >= 50 && recoveries >= crashes && state_corruption == 0 && memory_ok;

    let precision = if recoveries == crashes {
        95
    } else if recoveries >= crashes * 9 / 10 {
        80
    } else {
        50
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 0 },
        precision,
        performance: if state_corruption == 0 { 90 } else { 30 },
        polish: if last_memory.is_some() { 80 } else { 60 },
    };

    Ok(AcVerdict {
        id: "AC21".to_string(),
        name: "crash recovery under load".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "crashes={}, recoveries={}, corruptions={}, crash_gauntlet={}, first_memory={:?}, max_memory={:?}, final_memory={:?}, memory_ok={}",
            crashes,
            recoveries,
            state_corruption,
            challenge_ok,
            first_memory,
            max_memory,
            last_memory,
            memory_ok
        ),
        breakdown,
    })
}

fn ac22_spawn_precision(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 22, None)?;

    let mut spawn_distance = None;
    for _ in 0..3000 {
        let s = step(&mut client, 0, false, false)?.state;
        if s.monster.is_some() && spawn_distance.is_none() {
            spawn_distance = Some(s.distance_m);
            break;
        }
    }

    let spawn = match spawn_distance {
        Some(d) => d,
        None => return Err("monster did not spawn within 3000 ticks".into()),
    };

    let deviation = (spawn - 2000.0).abs();
    let pass = deviation <= 0.1;

    let precision = if deviation < 0.1 {
        100
    } else if deviation < 1.0 {
        90
    } else if deviation < 5.0 {
        70
    } else {
        30
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 0 },
        precision,
        performance: 85,
        polish: 75,
    };

    Ok(AcVerdict {
        id: "AC22".to_string(),
        name: "monster spawn timing precision".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "monster spawned at {:.3}m (deviation={:.3}m from 2000m)",
            spawn, deviation
        ),
        breakdown,
    })
}

// ===========================================================================
// TIER 3: Polish and Optimization (AC23-AC28)
// ===========================================================================

fn ac23_input_responsiveness(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 23, None)?;

    let mut latencies_ns: Vec<f64> = Vec::new();
    let mut responses_detected = 0u32;

    for _ in 0..100 {
        let s_before = state(&mut client)?.state;
        let before = Instant::now();
        let next = step(&mut client, 1, false, false)?;
        let latency = before.elapsed().as_nanos() as f64;
        latencies_ns.push(latency);

        let s_after = &next.state;
        if s_after.skier.x != s_before.skier.x {
            responses_detected += 1;
        }
    }

    if latencies_ns.is_empty() {
        return Err("no latency samples collected".into());
    }

    let n = latencies_ns.len() as f64;
    let avg_ns = latencies_ns.iter().sum::<f64>() / n;
    let avg_ms = avg_ns / 1_000_000.0;
    let max_ms = latencies_ns
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        / 1_000_000.0;

    let pass = avg_ms < 50.0 && max_ms < 100.0 && responses_detected > 80;

    let precision = if avg_ms < 10.0 {
        95
    } else if avg_ms < 25.0 {
        85
    } else if avg_ms < 50.0 {
        70
    } else {
        40
    };

    let perf = if max_ms < 20.0 {
        95
    } else if max_ms < 50.0 {
        80
    } else if max_ms < 100.0 {
        65
    } else {
        40
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 0 },
        precision,
        performance: perf,
        polish: 70,
    };

    Ok(AcVerdict {
        id: "AC23".to_string(),
        name: "input responsiveness".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "timing_source=host_wall_clock avg={:.2}ms max={:.2}ms responses={}/100",
            avg_ms, max_ms, responses_detected
        ),
        breakdown,
    })
}

fn ac24_collision_forgiveness(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 24, None)?;

    let mut near_misses = 0u32;
    let mut total_crashes = 0u32;
    let mut unfair_collisions = 0u32;
    let mut near_margin_passes = 0u32;

    for _ in 0..2000 {
        let s = state(&mut client)?.state;
        // Steer to a close non-colliding margin around obstacles.
        let steer = steer_near_obstacle_margin(&s, &CRASH_OBSTACLES);
        let next = step(&mut client, steer, false, false)?.state;

        let reported_near_miss = next.events.iter().any(|e| e == "near_miss");
        let mut near_margin_this_tick = false;

        // Track near-margin passes (skier passed very close without crashing)
        for obs in &s.obstacles {
            if obstacle_crossed_skier_path(&s, &next, obs) {
                let margin = (next.skier.x - obs.x).abs();
                let obs_half_width = obstacle_width(obs) / 2.0;
                if margin > obs_half_width && margin < obs_half_width + 1.0 {
                    near_margin_this_tick = true;
                    near_margin_passes += 1;
                }
            }
        }

        // Count near_miss events only when the reported event is geometrically
        // supported by an actual near-margin pass on this tick.
        if reported_near_miss && near_margin_this_tick {
            near_misses += 1;
        }

        if next.skier.mode == "crashed" {
            total_crashes += 1;
            let nearest = next
                .obstacles
                .iter()
                .filter(|obs| CRASH_OBSTACLES.contains(&obs.kind.as_str()))
                .min_by(|a, b| {
                    point_distance(next.skier.x, next.skier.y, a.x, a.y)
                        .partial_cmp(&point_distance(next.skier.x, next.skier.y, b.x, b.y))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            if let Some(obs) = nearest {
                let actual_distance = point_distance(next.skier.x, next.skier.y, obs.x, obs.y);
                let obs_half_width = obstacle_width(obs) / 2.0;
                if actual_distance > obs_half_width + 0.2 {
                    unfair_collisions += 1;
                }
            } else {
                // A crash with no nearby crashable obstacle is also unfair.
                unfair_collisions += 1;
            }
        }
    }

    // Pass requires actual near-miss events; inferred near-margin passes are
    // diagnostic only because AC24 validates near-miss detection itself.
    let has_near_miss_detection = near_misses > 50;
    let unfair_collision_rate = unfair_collisions as f64 / 2000.0;
    let pass = has_near_miss_detection && unfair_collision_rate < 0.01;

    let precision = if near_misses > 20 {
        95
    } else if near_misses > 5 || near_margin_passes > 20 {
        80
    } else if has_near_miss_detection {
        65
    } else {
        30
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 50 },
        precision,
        performance: 75,
        polish: if near_misses > 0 { 85 } else { 50 },
    };

    Ok(AcVerdict {
        id: "AC24".to_string(),
        name: "collision forgiveness".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "near_miss_events={}, near_margin_passes={}, crashes={}, unfair_collisions={} ({:.2}%)",
            near_misses,
            near_margin_passes,
            total_crashes,
            unfair_collisions,
            unfair_collision_rate * 100.0
        ),
        breakdown,
    })
}

fn ac25_animation_smoothness(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 25, None)?;

    let mut frame_times_ns: Vec<f64> = Vec::new();
    let mut speeds: Vec<f64> = Vec::new();

    for _ in 0..1000 {
        let before = Instant::now();
        let p = step(&mut client, 1, false, false)?;
        let elapsed = before.elapsed().as_nanos() as f64;
        frame_times_ns.push(elapsed);
        speeds.push(p.state.skier.speed);
    }

    let n = frame_times_ns.len() as f64;
    let avg_ns = frame_times_ns.iter().sum::<f64>() / n;
    let avg_ms = avg_ns / 1_000_000.0;

    // Variance in frame times (jitter), expressed in milliseconds squared.
    let ft_variance_ms2 = frame_times_ns
        .iter()
        .map(|t| ((t - avg_ns) / 1_000_000.0).powi(2))
        .sum::<f64>()
        / n;

    // Dropped frames (> 33.3ms = more than 2 frames at 60fps)
    let dropped_frames = frame_times_ns.iter().filter(|t| **t > 33_300_000.0).count();

    // Acceleration smoothness
    let mut accels: Vec<f64> = Vec::new();
    for w in speeds.windows(2) {
        accels.push(w[1] - w[0]);
    }
    let accel_variance = if accels.len() > 1 {
        let mean = accels.iter().sum::<f64>() / accels.len() as f64;
        accels.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / accels.len() as f64
    } else {
        1.0
    };

    let pass = avg_ms < 16.6 && ft_variance_ms2 < 4.0 && dropped_frames < 10;

    let precision = if ft_variance_ms2 < 1.0 {
        95
    } else if ft_variance_ms2 < 4.0 {
        80
    } else {
        60
    };

    let perf = if dropped_frames == 0 {
        95
    } else if dropped_frames < 5 {
        85
    } else if dropped_frames < 10 {
        70
    } else {
        40
    };

    let polish = if accel_variance < 0.01 {
        90
    } else if accel_variance < 0.1 {
        75
    } else {
        55
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 0 },
        precision,
        performance: perf,
        polish,
    };

    Ok(AcVerdict {
        id: "AC25".to_string(),
        name: "animation smoothness".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "timing_source=host_wall_clock avg={:.2}ms variance_ms2={:.2} dropped={} accel_var={:.4}",
            avg_ms, ft_variance_ms2, dropped_frames, accel_variance
        ),
        breakdown,
    })
}

fn ac26_replay_accuracy(harness: &Harness) -> CheckResult {
    // Run 1000 ticks, get replay data, then replay in fresh process
    let mut client1 = DriverClient::spawn(harness)?;
    init(&mut client1, 26, None)?;

    let mut inputs: Vec<(i64, bool, bool)> = Vec::new();
    let mut checksum_states: Vec<String> = Vec::new();

    for tick in 0..1000 {
        let steer = match tick % 5 {
            0 | 1 => -1i64,
            2 => 0,
            _ => 1,
        };
        let boost = tick % 11 == 0;
        let jump = tick % 13 == 0;
        inputs.push((steer, boost, jump));
        let p = step(&mut client1, steer, boost, jump)?;
        checksum_states.push(p.canonical);
    }

    // Try to get replay data from driver
    let replay_data = replay(&mut client1, 1000).ok();

    // Compute expected checksum
    let combined = checksum_states.join("\n");
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let expected_checksum = format!("sha256:{:x}", hasher.finalize());

    // Replay in fresh process
    let mut client2 = DriverClient::spawn(harness)?;
    init(&mut client2, 26, None)?;
    let mut replayed_states: Vec<String> = Vec::new();

    for (steer, boost, jump) in &inputs {
        let p = step(&mut client2, *steer, *boost, *jump)?;
        replayed_states.push(p.canonical);
    }

    let replayed_combined = replayed_states.join("\n");
    let mut hasher2 = Sha256::new();
    hasher2.update(replayed_combined.as_bytes());
    let replayed_checksum = format!("sha256:{:x}", hasher2.finalize());

    let checksums_match = expected_checksum == replayed_checksum;

    // If driver provided replay data, also check its checksum
    let driver_replay_match = replay_data
        .as_ref()
        .map(|r| r.state_checksum == expected_checksum || r.state_checksum == replayed_checksum);

    let driver_replay_payload_present = replay_data
        .as_ref()
        .map(|r| !r.input_sequence.is_empty() && r.end_tick >= r.start_tick)
        .unwrap_or(false);

    let driver_replay_roundtrip = if let Some(replay) = &replay_data {
        let decoded_inputs = decode_replay_inputs(&replay.input_sequence)?;
        let declared_ticks = replay.end_tick.saturating_sub(replay.start_tick) as usize;
        if decoded_inputs.is_empty() || declared_ticks != decoded_inputs.len() {
            false
        } else {
            let mut replay_client = DriverClient::spawn(harness)?;
            init(&mut replay_client, replay.seed, None)?;
            let mut replay_states = Vec::with_capacity(decoded_inputs.len());
            for (steer, boost, jump) in decoded_inputs {
                let p = step(&mut replay_client, steer, boost, jump)?;
                replay_states.push(p.canonical);
            }
            let replay_combined = replay_states.join("\n");
            let mut replay_hasher = Sha256::new();
            replay_hasher.update(replay_combined.as_bytes());
            let replay_checksum = format!("sha256:{:x}", replay_hasher.finalize());
            replay_checksum == replay.state_checksum
        }
    } else {
        false
    };

    let pass = checksums_match
        && driver_replay_match == Some(true)
        && driver_replay_payload_present
        && driver_replay_roundtrip;

    let precision = if checksums_match {
        if driver_replay_match == Some(true) {
            100
        } else {
            90
        }
    } else {
        20
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 0 },
        precision,
        performance: 85,
        polish: if replay_data.is_some() { 90 } else { 60 },
    };

    Ok(AcVerdict {
        id: "AC26".to_string(),
        name: "deterministic replay accuracy".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "checksum_match={} driver_replay={} over 1000 ticks",
            checksums_match,
            replay_data.is_some()
        ),
        breakdown,
    })
}

fn ac27_performance_budget(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 27, None)?;

    // Try dense field challenge. If unsupported and normal fallback cannot
    // create dense conditions, this AC is untestable and should be skipped.
    let dense_challenge_ok =
        challenge(&mut client, "dense_field", &json!({"obstacleCount": 100})).is_ok();

    let mut frame_times_ns: Vec<f64> = Vec::new();
    let mut total_allocations: i64 = 0;
    let mut peak_memory: i64 = 0;
    let mut max_obstacles = 0usize;
    let mut has_quality = false;
    let mut profile_supported = true;
    let mut profile_samples = 0usize;
    let mut allocations_available = false;
    let mut allocations_unavailable = false;
    let mut memory_available = false;
    let mut memory_unavailable = false;
    let mut reported_avg_ns: Option<i64> = None;
    let mut reported_p99_ns: Option<i64> = None;
    let mut reported_profile_ticks: Option<u64> = None;
    let mut reported_tick_nanos_samples: Vec<i64> = Vec::new();

    for _ in 0..1000 {
        let before = Instant::now();
        let p = step(&mut client, 0, false, false)?;
        let elapsed = before.elapsed().as_nanos() as f64;
        frame_times_ns.push(elapsed);
        max_obstacles = max_obstacles.max(p.state.obstacles.len());

        if let Some(q) = &p.state.quality {
            has_quality = true;
            if let Some(allocs) = q.active_objects {
                // active_objects used as proxy when total_allocations not available
                let _ = allocs;
            }
            if let Some(mem) = q.memory_bytes {
                if mem >= 0 {
                    memory_available = true;
                    peak_memory = peak_memory.max(mem);
                } else {
                    memory_unavailable = true;
                }
            }
        }

        // Sample allocation/memory telemetry across the whole dense-field run,
        // not just the tail window. Per protocol, -1 means unavailable.
        if profile_supported {
            match profile(&mut client, Some(1)) {
                Ok(pm) => {
                    profile_samples += 1;
                    let mut sample_ns = None;
                    if pm.avg_tick_nanos >= 0 {
                        sample_ns = Some(pm.avg_tick_nanos);
                    }
                    if pm.p99_tick_nanos >= 0 {
                        sample_ns = Some(
                            sample_ns
                                .map_or(pm.p99_tick_nanos, |ns: i64| ns.max(pm.p99_tick_nanos)),
                        );
                    }
                    if let Some(ns) = sample_ns {
                        reported_tick_nanos_samples.push(ns);
                    }
                    if pm.total_allocations >= 0 {
                        allocations_available = true;
                        total_allocations += pm.total_allocations;
                    } else {
                        allocations_unavailable = true;
                    }
                    if pm.peak_memory_bytes >= 0 {
                        memory_available = true;
                        peak_memory = peak_memory.max(pm.peak_memory_bytes);
                    } else {
                        memory_unavailable = true;
                    }
                }
                Err(_) => {
                    profile_supported = false;
                    allocations_unavailable = true;
                    memory_unavailable = true;
                }
            }
        }
    }

    if profile_supported {
        match profile(&mut client, Some(1000)) {
            Ok(pm) => {
                profile_samples += 1;
                let full_window_profile = pm.window_ticks >= 1000;
                reported_profile_ticks = Some(pm.window_ticks);
                if full_window_profile && pm.avg_tick_nanos >= 0 {
                    reported_avg_ns = Some(pm.avg_tick_nanos);
                }
                if full_window_profile && pm.p99_tick_nanos >= 0 {
                    reported_p99_ns = Some(pm.p99_tick_nanos);
                }
                if pm.total_allocations >= 0 {
                    allocations_available = true;
                    total_allocations += pm.total_allocations;
                }
                if pm.peak_memory_bytes >= 0 {
                    memory_available = true;
                    peak_memory = peak_memory.max(pm.peak_memory_bytes);
                }
            }
            Err(_) => {
                allocations_unavailable = true;
                memory_unavailable = true;
            }
        }
    }

    if !reported_tick_nanos_samples.is_empty()
        && (reported_avg_ns.is_none() || reported_p99_ns.is_none())
    {
        let sample_count = reported_tick_nanos_samples.len() as i64;
        if reported_avg_ns.is_none() {
            let sum: i64 = reported_tick_nanos_samples.iter().sum();
            reported_avg_ns = Some(sum / sample_count);
        }
        if reported_p99_ns.is_none() {
            let mut sorted_reported = reported_tick_nanos_samples.clone();
            sorted_reported.sort_unstable();
            let p99_idx = ((sorted_reported.len() as f64) * 0.99) as usize;
            reported_p99_ns = sorted_reported
                .get(p99_idx.min(sorted_reported.len().saturating_sub(1)))
                .copied();
        }
    }

    let n = frame_times_ns.len() as f64;
    let avg_ns = frame_times_ns.iter().sum::<f64>() / n;
    let avg_ms = avg_ns / 1_000_000.0;

    let mut sorted = frame_times_ns.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p99_idx = (n * 0.99) as usize;
    let p99_ms = sorted.get(p99_idx).copied().unwrap_or(0.0) / 1_000_000.0;

    if !dense_challenge_ok && max_obstacles < 50 {
        let breakdown = QualityBreakdown {
            basic: 0,
            precision: 0,
            performance: 0,
            polish: 0,
        };
        return Ok(AcVerdict {
            id: "AC27".to_string(),
            name: "performance budget".to_string(),
            pass: false,
            skipped: true,
            quality: 0,
            detail: format!(
                "skipped: dense_field challenge unsupported and fallback reached only {} visible obstacles",
                max_obstacles
            ),
            breakdown,
        });
    }

    // Profile telemetry is optional. Absence/unavailability should not fail the
    // AC, but any available allocation or memory evidence must respect budget.
    let allocations_ok = !allocations_available || total_allocations == 0;
    let memory_ok = !memory_available || (peak_memory > 0 && peak_memory < 50_000_000);

    // External wall-clock sampling is not the ranked AC27 pass gate. The pass
    // gate uses full-window driver-reported profile telemetry so the conformance
    // result is not tied to the scorer host. The external probe may still feed
    // the labeled quality breakdown/composite as host-bound diagnostic evidence.
    // Driver timing values are self-reported and must be labeled as such.
    let reported_full_window_ok = matches!(reported_profile_ticks, Some(ticks) if ticks >= 1000);
    let reported_perf_ok = reported_full_window_ok
        && matches!(reported_avg_ns, Some(ns) if ns < 16_600_000)
        && matches!(reported_p99_ns, Some(ns) if ns < 20_000_000);
    let pass = max_obstacles >= 50 && allocations_ok && memory_ok && reported_perf_ok;

    let precision = if allocations_available && total_allocations == 0 && has_quality {
        100
    } else if allocations_ok {
        80
    } else {
        50
    };

    let perf = if p99_ms < 16.6 {
        95
    } else if p99_ms < 20.0 {
        85
    } else if p99_ms < 30.0 {
        70
    } else {
        40
    };

    let polish = if memory_available && peak_memory > 0 && peak_memory < 50_000_000 {
        90
    } else if memory_available && peak_memory > 0 {
        60
    } else if has_quality || profile_samples > 0 {
        70
    } else {
        50
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 0 },
        precision,
        performance: perf,
        polish,
    };

    Ok(AcVerdict {
        id: "AC27".to_string(),
        name: "performance budget".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "ranked_timing_source=driver_reported_profile diagnostic_timing_source=host_wall_clock reported_avg_ns={:?} reported_p99_ns={:?} reported_profile_ticks={:?} reported_full_window={} external_probe_avg={:.2}ms external_probe_p99={:.2}ms max_obstacles={} dense_challenge={} allocs={} peak_mem={}MB quality={} profile_samples={} allocations_unavailable={} memory_unavailable={}",
            reported_avg_ns,
            reported_p99_ns,
            reported_profile_ticks,
            reported_full_window_ok,
            avg_ms,
            p99_ms,
            max_obstacles,
            dense_challenge_ok,
            total_allocations,
            peak_memory / 1_000_000,
            has_quality,
            profile_samples,
            allocations_unavailable,
            memory_unavailable
        ),
        breakdown,
    })
}

fn ac28_visual_polish(harness: &Harness) -> CheckResult {
    let mut client = DriverClient::spawn(harness)?;
    init(&mut client, 28, None)?;

    let mut particle_events = 0u32;
    let mut shake_events = 0u32;
    let mut style_events = 0u32;
    let mut landing_events = 0u32;
    let mut crash_events = 0u32;
    let mut near_miss_events = 0u32;
    let mut color_grading_events = 0u32;
    let mut total_events_seen = 0u32;
    let mut event_types_seen: Vec<String> = Vec::new();

    // Play through various scenarios
    for _ in 0..2000 {
        let s = state(&mut client)?.state;
        let steer = if s.skier.mode == "crashed" {
            0
        } else {
            steer_toward_obstacle(&s, &["ramp"])
        };
        let next = step(&mut client, steer, false, true)?;

        for event in &next.state.events {
            if !event_types_seen.contains(event) {
                event_types_seen.push(event.clone());
            }
            total_events_seen += 1;
            match event.as_str() {
                "particle_spawn" => particle_events += 1,
                "screen_shake" => shake_events += 1,
                "style_gain" | "style_loss" => style_events += 1,
                "landing" => landing_events += 1,
                "crash" | "collision" => crash_events += 1,
                "near_miss" => near_miss_events += 1,
                "color_grading_active" => color_grading_events += 1,
                _ => {}
            }
        }
    }

    let total_event_types = event_types_seen.len();
    let has_rich_events = total_event_types >= 4;
    let visual_feedback_events =
        particle_events + shake_events + style_events + near_miss_events + color_grading_events;
    // Headless drivers can self-report event strings, but visual polish is a
    // human/renderer judgment. Keep event richness as quality telemetry only;
    // do not award a mechanical PASS from spoofable strings alone.
    let pass = false;

    let precision = if total_event_types >= 6 {
        95
    } else if total_event_types >= 4 {
        80
    } else if total_event_types >= 2 {
        65
    } else if total_events_seen > 0 {
        50
    } else {
        20
    };

    let polish = if particle_events > 0 && shake_events > 0 {
        90
    } else if particle_events > 0 || shake_events > 0 {
        75
    } else if has_rich_events {
        60
    } else {
        40
    };

    let breakdown = QualityBreakdown {
        basic: if pass { 100 } else { 30 },
        precision,
        performance: 75,
        polish,
    };

    Ok(AcVerdict {
        id: "AC28".to_string(),
        name: "visual polish".to_string(),
        pass,
        skipped: false,
        quality: breakdown.composite(),
        detail: format!(
            "probe_only=true event_types={} total_events={} visual_feedback_events={} [particles={}, shake={}, style={}, landing={}, crash={}, near_miss={}, color_grading={}] types={:?}",
            total_event_types,
            total_events_seen,
            visual_feedback_events,
            particle_events,
            shake_events,
            style_events,
            landing_events,
            crash_events,
            near_miss_events,
            color_grading_events,
            event_types_seen
        ),
        breakdown,
    })
}
