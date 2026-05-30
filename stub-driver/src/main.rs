use std::io::{self, BufRead, Write};

#[derive(Clone)]
struct State {
    x: f64,
    y: f64,
    speed: f64,
    mode: &'static str,
    distance_m: f64,
    style: f64,
    game_over: bool,
    tick: u64,
}

impl State {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            speed: 1.0,
            mode: "skiing",
            distance_m: 0.0,
            style: 0.0,
            game_over: false,
            tick: 0,
        }
    }

    fn step(&mut self, steer: i32) {
        self.tick += 1;
        if self.mode == "skiing" {
            self.x += steer as f64;
            self.y += self.speed;
            self.distance_m += self.speed;
        }
    }

    fn json(&self) -> String {
        format!(
            "{{\"skier\":{{\"x\":{},\"y\":{},\"speed\":{},\"mode\":\"{}\"}},\"distanceM\":{},\"style\":{},\"obstacles\":[],\"monster\":null,\"gameOver\":{},\"tick\":{}}}",
            json_number(self.x),
            json_number(self.y),
            json_number(self.speed),
            self.mode,
            json_number(self.distance_m),
            json_number(self.style),
            if self.game_over { "true" } else { "false" },
            self.tick
        )
    }
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut state = State::new();

    for line_result in stdin.lock().lines() {
        let Ok(line) = line_result else {
            break;
        };
        let trimmed = line.trim();
        let response = if has_cmd(trimmed, "init") {
            state = State::new();
            ok_state(&state)
        } else if has_cmd(trimmed, "reset") {
            state = State::new();
            ok_state(&state)
        } else if has_cmd(trimmed, "state") {
            ok_state(&state)
        } else if has_cmd(trimmed, "step") {
            let steer = parse_steer(trimmed);
            state.step(steer);
            ok_state(&state)
        } else {
            "{\"ok\":false,\"error\":\"unsupported command\"}".to_string()
        };

        if writeln!(stdout, "{}", response).is_err() {
            break;
        }
        if stdout.flush().is_err() {
            break;
        }
    }
}

fn has_cmd(line: &str, cmd: &str) -> bool {
    line.contains(&format!("\"cmd\":\"{}\"", cmd))
        || line.contains(&format!("\"cmd\": \"{}\"", cmd))
}

fn parse_steer(line: &str) -> i32 {
    if line.contains("\"steer\":-1") || line.contains("\"steer\": -1") {
        -1
    } else if line.contains("\"steer\":1") || line.contains("\"steer\": 1") {
        1
    } else {
        0
    }
}

fn ok_state(state: &State) -> String {
    format!("{{\"ok\":true,\"state\":{}}}", state.json())
}

fn json_number(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{:.0}", n)
    } else {
        n.to_string()
    }
}
