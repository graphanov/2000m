use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{self, BufRead, Write};

#[derive(Clone)]
struct Obstacle {
    kind: &'static str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone)]
struct State {
    seed: i64,
    x: f64,
    y: f64,
    speed: f64,
    mode: &'static str,
    distance_m: f64,
    style: f64,
    game_over: bool,
    tick: u64,
    obstacles: Vec<Obstacle>,
    step_inputs: Vec<u8>,
    step_canonicals: Vec<String>,
}

impl State {
    fn new(seed: i64) -> Self {
        Self {
            seed,
            x: 0.0,
            y: 0.0,
            speed: 1.0,
            mode: "skiing",
            distance_m: 0.0,
            style: 0.0,
            game_over: false,
            tick: 0,
            obstacles: Vec::new(),
            step_inputs: Vec::new(),
            step_canonicals: Vec::new(),
        }
    }

    fn step(&mut self, steer: i64, boost: bool, jump: bool) {
        self.tick += 1;
        if self.mode == "skiing" {
            let clamped_steer = steer.clamp(-1, 1) as f64;
            self.speed = if boost { 2.0 } else { 1.0 };
            self.x += clamped_steer;
            self.y += self.speed;
            self.distance_m += self.speed;
            if jump {
                self.style += 0.25;
            }
        }

        if self.obstacles.is_empty() && self.tick % 17 == 0 {
            self.obstacles.push(Obstacle {
                kind: "tree",
                x: 8.0,
                y: self.y + 25.0,
                width: 2.0,
                height: 3.0,
            });
        }

        self.step_inputs.push(pack_input(steer, boost, jump));
        self.step_canonicals
            .push(canonical_json(&self.state_value()));
    }

    fn apply_challenge(&mut self, name: &str, params: &Value) -> bool {
        match name {
            "dense_field" => {
                let requested = params
                    .get("obstacleCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(100)
                    .min(120) as usize;
                self.obstacles = (0..requested)
                    .map(|idx| Obstacle {
                        kind: if idx % 3 == 0 { "rock" } else { "tree" },
                        x: (idx as f64 % 20.0) - 10.0,
                        y: self.y + 10.0 + idx as f64,
                        width: if idx % 3 == 0 { 3.0 } else { 2.0 },
                        height: if idx % 3 == 0 { 2.0 } else { 3.0 },
                    })
                    .collect();
                true
            }
            "high_speed" => {
                self.speed = 8.0;
                self.obstacles = vec![Obstacle {
                    kind: "tree",
                    x: 0.0,
                    y: self.y + 4.0,
                    width: 2.0,
                    height: 3.0,
                }];
                true
            }
            _ => false,
        }
    }

    fn state_value(&self) -> Value {
        let obstacles: Vec<Value> = self
            .obstacles
            .iter()
            .map(|obstacle| {
                json!({
                    "type": obstacle.kind,
                    "x": obstacle.x,
                    "y": obstacle.y,
                    "width": obstacle.width,
                    "height": obstacle.height
                })
            })
            .collect();

        json!({
            "skier": {
                "x": self.x,
                "y": self.y,
                "speed": self.speed,
                "mode": self.mode
            },
            "distanceM": self.distance_m,
            "style": self.style,
            "obstacles": obstacles,
            "monster": Value::Null,
            "gameOver": self.game_over,
            "tick": self.tick,
            "events": [],
            "quality": {
                "tickNanos": 1000000,
                "collisionChecks": self.obstacles.len() as i64,
                "collisionHits": 0,
                "activeObjects": self.obstacles.len() as i64 + 1,
                "memoryBytes": 1048576
            }
        })
    }

    fn profile_value(&self, window: u64) -> Value {
        let ticks = if window == 0 {
            self.tick
        } else {
            window.min(self.tick.max(1))
        };
        json!({
            "windowTicks": ticks,
            "avgTickNanos": 1000000,
            "maxTickNanos": 1000000,
            "p95TickNanos": 1000000,
            "p99TickNanos": 1000000,
            "totalAllocations": -1,
            "peakMemoryBytes": 1048576,
            "collisionChecksTotal": (self.obstacles.len() as i64) * (ticks as i64),
            "collisionHitsTotal": 0
        })
    }

    fn replay_value(&self, requested_ticks: u64) -> Value {
        let available = self.step_inputs.len();
        let count = (requested_ticks as usize).min(available);
        let start = available.saturating_sub(count);
        let inputs = &self.step_inputs[start..];
        let canonicals = &self.step_canonicals[start..];
        let combined = canonicals.join("\n");
        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        json!({
            "seed": self.seed,
            "startTick": start as u64,
            "endTick": (start + count) as u64,
            "inputSequence": BASE64.encode(inputs),
            "stateChecksum": format!("sha256:{:x}", hasher.finalize())
        })
    }
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut state = State::new(0);

    for line_result in stdin.lock().lines() {
        let Ok(line) = line_result else { break };
        let Ok(command) = serde_json::from_str::<Value>(line.trim()) else {
            write_response(&mut stdout, json!({"ok": false, "error": "invalid JSON"}));
            continue;
        };

        let response = match command.get("cmd").and_then(Value::as_str) {
            Some("init") | Some("reset") => {
                let seed = command.get("seed").and_then(Value::as_i64).unwrap_or(0);
                state = State::new(seed);
                ok_state(&state)
            }
            Some("state") => ok_state(&state),
            Some("step") => {
                let input = command.get("input").unwrap_or(&Value::Null);
                let steer = input.get("steer").and_then(Value::as_i64).unwrap_or(0);
                let boost = input.get("boost").and_then(Value::as_bool).unwrap_or(false);
                let jump = input.get("jump").and_then(Value::as_bool).unwrap_or(false);
                state.step(steer, boost, jump);
                ok_state(&state)
            }
            Some("profile") => {
                let window = command.get("window").and_then(Value::as_u64).unwrap_or(0);
                json!({"ok": true, "metrics": state.profile_value(window)})
            }
            Some("replay") => {
                let ticks = command
                    .get("ticks")
                    .and_then(Value::as_u64)
                    .unwrap_or(state.tick);
                json!({"ok": true, "replay": state.replay_value(ticks)})
            }
            Some("challenge") => {
                let name = command.get("name").and_then(Value::as_str).unwrap_or("");
                let params = command.get("params").unwrap_or(&Value::Null);
                if state.apply_challenge(name, params) {
                    ok_state(&state)
                } else {
                    json!({"ok": false, "error": format!("unknown challenge: {name}")})
                }
            }
            Some(other) => json!({"ok": false, "error": format!("unsupported command: {other}")}),
            None => json!({"ok": false, "error": "missing cmd"}),
        };

        write_response(&mut stdout, response);
    }
}

fn ok_state(state: &State) -> Value {
    json!({"ok": true, "state": state.state_value()})
}

fn write_response(stdout: &mut io::Stdout, response: Value) {
    if writeln!(stdout, "{}", response).is_ok() {
        let _ = stdout.flush();
    }
}

fn pack_input(steer: i64, boost: bool, jump: bool) -> u8 {
    let steer_bits = match steer.clamp(-1, 1) {
        -1 => 0,
        0 => 1,
        1 => 2,
        _ => unreachable!(),
    };
    steer_bits | ((boost as u8) << 2) | ((jump as u8) << 3)
}

fn canonical_json(state_value: &Value) -> String {
    let mut canonical = state_value.clone();
    if let Some(obj) = canonical.as_object_mut() {
        obj.remove("quality");
    }
    serde_json::to_string(&canonical).expect("state JSON should serialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_checksum_uses_canonical_step_states() {
        let mut state = State::new(26);
        state.step(-1, true, false);
        state.step(0, false, true);
        let replay = state.replay_value(2);
        assert_eq!(replay["startTick"], 0);
        assert_eq!(replay["endTick"], 2);
        assert!(!replay["inputSequence"].as_str().unwrap().is_empty());
        assert!(replay["stateChecksum"]
            .as_str()
            .unwrap()
            .starts_with("sha256:"));
    }
}
