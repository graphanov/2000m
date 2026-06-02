#!/usr/bin/env python3
"""Deterministic v3 JSON-line fixture driver for scorer calibration."""
from __future__ import annotations

import hashlib
import json
import sys
from typing import Any

PROTOCOL = "2000m.driver.v3"
ALLOWED_EVENTS = {
    "collision": ["collision", "crash"],
    "recovery": ["recover"],
    "ramp": ["ramp_enter", "airborne"],
    "airborne": ["airborne", "land"],
    "style": ["style_gain"],
    "monster-spawn": ["monster_spawn"],
    "monster-pursuit": ["monster_spawn", "monster_pursuit"],
    "monster-contact": ["monster_contact", "monster_flee"],
}


def digest(value: Any) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def replay_digest(checksums: list[str]) -> str:
    encoded = json.dumps(checksums, sort_keys=True, separators=(",", ":")).encode()
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


class Driver:
    def __init__(self, poison: bool = False) -> None:
        self.poison = poison
        self.seed = 0
        self.tick = 0
        self.x = 0.0
        self.y = 0.0
        self.vx = 0.0
        self.vy = 4.0
        self.airborne = False
        self.crashed = False
        self.style = 0
        self.monster_active = False

    def make_state(self, events: list[str] | None = None, challenge: str | None = None) -> dict[str, Any]:
        events = events or []
        obstacle_type = "snowman" if self.poison else "tree"
        state_events = ["teleport"] if self.poison else events
        obstacle_y = 32 + (self.seed % 7) + self.tick
        monster = None
        if self.monster_active or (challenge and "monster" in challenge):
            monster = {
                "active": True,
                "mode": "flee" if challenge == "monster-contact" else "pursuit",
                "x": round(self.x + 3.0, 2),
                "y": round(self.y + 8.0, 2),
            }
        return {
            "tick": self.tick,
            "seed": self.seed,
            "skier": {
                "x": round(self.x, 2),
                "y": round(self.y, 2),
                "vx": round(self.vx, 2),
                "vy": round(self.vy, 2),
                "airborne": self.airborne,
                "crashed": self.crashed,
                "style": self.style,
            },
            "world": {
                "slope": "calibration-slope",
                "window": {"width": 800, "height": 600},
                "obstacles": [
                    {"type": obstacle_type, "x": -12 + (self.seed % 5), "y": obstacle_y},
                    {"type": "ramp", "x": 18, "y": obstacle_y + 25},
                ],
            },
            "monster": monster,
            "score": {"distanceM": int(self.y), "style": self.style, "crashCount": 1 if self.crashed else 0},
            "events": state_events,
        }

    def payload(self, events: list[str] | None = None, challenge: str | None = None) -> dict[str, Any]:
        state = self.make_state(events, challenge)
        return {"state": state, "events": events or [], "stateChecksum": digest(state)}

    def init(self, seed: int) -> dict[str, Any]:
        self.seed = seed
        self.tick = 0
        self.x = 0.0
        self.y = 0.0
        self.vx = 0.0
        self.vy = 4.0
        self.airborne = False
        self.crashed = False
        self.style = 0
        self.monster_active = False
        out = self.payload(["spawn"])
        out["capabilities"] = {"challengeSets": ["public", "hidden", "calibration"], "capture": False}
        return out

    def step(self, input_value: dict[str, Any]) -> dict[str, Any]:
        self.tick += 1
        steer = int(input_value.get("steer", 0))
        self.vx = float(steer)
        self.x += self.vx
        self.y += self.vy
        events: list[str] = []
        if input_value.get("jump"):
            self.airborne = True
            events.extend(["ramp_enter", "airborne"])
        if input_value.get("land"):
            self.airborne = False
            events.append("land")
        if input_value.get("style"):
            self.style += 10
            events.append("style_gain")
        if input_value.get("recover"):
            self.crashed = False
            events.append("recover")
        if input_value.get("flee"):
            self.monster_active = False
            events.append("monster_flee")
        return self.payload(events)

    def challenge(self, challenge_id: str, seed: int, inputs: list[dict[str, Any]]) -> dict[str, Any]:
        self.init(seed)
        kind = challenge_id.replace("public-", "").replace("hidden-", "").replace("calibration-", "")
        for item in inputs:
            self.step(item)
        events = ALLOWED_EVENTS.get(kind, ["turn"])
        if kind == "collision":
            self.crashed = True
        if kind in {"ramp", "airborne"}:
            self.airborne = kind == "ramp"
        if kind == "style":
            self.style += 15
        if kind.startswith("monster"):
            self.monster_active = kind != "monster-contact"
        payload = self.payload(events, kind)
        payload["challengeResult"] = {"challengeId": challenge_id, "passed": True, "kind": kind}
        return payload

    def replay(self, seed: int, inputs: list[dict[str, Any]]) -> dict[str, Any]:
        payload = self.init(seed)
        checksums = [payload["stateChecksum"]]
        for item in inputs:
            payload = self.step(item)
            checksums.append(payload["stateChecksum"])
        return {
            "finalState": payload["state"],
            "stateChecksums": checksums,
            "replayChecksum": replay_digest(checksums),
        }


def error(request_id: str, code: str, message: str) -> dict[str, Any]:
    return {"protocolVersion": PROTOCOL, "requestId": request_id, "ok": False, "error": {"code": code, "message": message}}


def ok(request_id: str, payload: dict[str, Any]) -> dict[str, Any]:
    return {"protocolVersion": PROTOCOL, "requestId": request_id, "ok": True, "payload": payload}


def main() -> int:
    driver = Driver(poison="--poison-enum" in sys.argv)
    for line in sys.stdin:
        try:
            request = json.loads(line)
            request_id = request.get("requestId", "missing-request-id")
            command = request.get("command")
            payload = request.get("payload") or {}
            if request.get("protocolVersion") != PROTOCOL:
                response = error(request_id, "invalid_request", "protocolVersion mismatch")
            elif command == "init":
                response = ok(request_id, driver.init(int(payload.get("seed", 0))))
            elif command == "reset":
                response = ok(request_id, driver.init(int(payload.get("seed", 0))))
            elif command == "state":
                response = ok(request_id, driver.payload())
            elif command == "step":
                response = ok(request_id, driver.step(payload.get("input") or {}))
            elif command == "profile":
                response = ok(request_id, {"profile": {"fixture": True, "protocolVersion": PROTOCOL}})
            elif command == "challenge":
                response = ok(request_id, driver.challenge(str(payload.get("challengeId", "public-basic")), int(payload.get("seed", 0)), payload.get("inputs") or []))
            elif command == "replay":
                response = ok(request_id, driver.replay(int(payload.get("seed", 0)), payload.get("inputs") or []))
            else:
                response = error(request_id, "unsupported_command", "unsupported command")
        except Exception as exc:  # fixture must return structured errors, not malformed output
            response = error("unknown", "internal_error", str(exc)[:120])
        print(json.dumps(response, sort_keys=True, separators=(",", ":")), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
