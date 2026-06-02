#!/usr/bin/env python3
"""Deterministic native-capture fixture for v3 visual harness smokes."""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
from pathlib import Path

# Valid 1x1 transparent PNG and 1x1 GIF. Metadata carries seed/window identity.
PNG_BYTES = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII="
)
GIF_BYTES = base64.b64decode("R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==")


def sha256_bytes(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def sha256_json(value: object) -> str:
    return sha256_bytes(json.dumps(value, sort_keys=True, separators=(",", ":")).encode())


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--seed", required=True, type=int)
    parser.add_argument("--window", required=True)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    # Keep binary artifacts deterministic while frame metadata records the frozen inputs.
    screenshot = PNG_BYTES + f"\n# seed={args.seed} window={args.window}\n".encode()
    replay = GIF_BYTES + f"\n# seed={args.seed} window={args.window}\n".encode()
    state = {"seed": args.seed, "window": args.window, "fixture": "visual-capture", "tick": 120}
    frame_checksum = sha256_bytes(screenshot + replay)
    frames = {
        "schemaVersion": "2000m.capture.frames.v1",
        "seed": args.seed,
        "window": args.window,
        "frameCount": 120,
        "fps": 30,
        "inputSequenceRef": "inputs.json",
        "stateChecksum": sha256_json(state),
        "frameChecksum": frame_checksum,
        "events": ["spawn", "turn"],
        "warnings": []
    }
    rubric_metadata = {
        "schemaVersion": "2000m.capture.rubric-metadata.v1",
        "seed": args.seed,
        "window": args.window,
        "artifactDigest": sha256_json({"fixture": "visual-artifact", "seed": args.seed, "window": args.window}),
        "assetAttestation": "generated-placeholder-only"
    }
    (args.out / "screenshot.png").write_bytes(screenshot)
    (args.out / "replay.gif").write_bytes(replay)
    (args.out / "frames.json").write_text(json.dumps(frames, indent=2, sort_keys=True) + "\n")
    (args.out / "rubric-metadata.json").write_text(json.dumps(rubric_metadata, indent=2, sort_keys=True) + "\n")
    (args.out / "inputs.json").write_text(json.dumps({"seed": args.seed, "window": args.window, "inputs": []}, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
