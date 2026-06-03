#!/usr/bin/env python3
"""Run deterministic v3 capture smokes and build a blind visual package."""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUT = ROOT / "v3" / "fixtures" / "visual" / "out"
VALID_MANIFEST = ROOT / "v3" / "fixtures" / "visual" / "valid-artifact" / "2000m.v3.json"
MISSING_CAPTURE_MANIFEST = ROOT / "v3" / "fixtures" / "visual" / "missing-capture-artifact" / "2000m.v3.json"
WINDOWS = ["early-game", "mid-run-obstacle-field", "ramp-style-sequence", "monster-pressure", "post-feedback-rerun"]
SEEDS = [1101, 2202]


def sha256_bytes(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def display(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def run(cmd: list[str], cwd: Path) -> None:
    print("$", " ".join(cmd))
    subprocess.run(cmd, cwd=cwd, check=True)


def materialize_args(args: list[str], seed: int, window: str, out: Path) -> list[str]:
    return [item.replace("{seed}", str(seed)).replace("{window}", window).replace("{out}", str(out)) for item in args]


def build_blocked_result(out: Path) -> Path:
    result = {
        "schemaVersion": "2000m.v3.result.v1",
        "campaignId": "v3-visual-smoke",
        "scenarioId": "v3-visual-smoke",
        "taskSeed": 0,
        "laneId": "A",
        "entrant": {"model": "fixture-driver", "runtime": "v3-visual-smoke", "processType": "scripted-agent"},
        "protocolFreeze": {"changedAfterLiveResults": False, "scorerMutationObserved": False, "calibrationOnlyIfChanged": True},
        "mechanical": {
            "ranked": True,
            "protocolVersion": "2000m.driver.v3",
            "passCount": 24,
            "totalAcs": 24,
            "compositeScore": 100,
            "determinism": {"pass": True, "details": "mechanical fixture only"},
            "failedAcs": [],
            "hiddenChallengeSummary": "not run by visual smoke",
            "regressionSummary": "not run by visual smoke",
            "resultJsonRef": "v3/fixtures/mechanical/out/valid-result.json"
        },
        "visual": {
            "ranked": False,
            "blockReason": "missing-native-capture-or-playable-surface",
            "visualPackageRef": "",
            "captureDeterminism": "blocked",
            "rubricRecordRef": ""
        },
        "workflow": {
            "contextWipeRecoveryScore": 0,
            "feedbackDecisionScore": 0,
            "regressionProtectionScore": 0,
            "impossibleRequirementHandlingScore": 0,
            "handoffScore": 0,
            "finalRecommendation": "continue",
            "rationaleRefs": ["v3/VISUAL_RUBRIC.md"]
        },
        "evidence": {
            "replayable": True,
            "publicSafe": True,
            "privateRefsBlocked": False,
            "compactSummaryRef": "v3/VISUAL_RUBRIC.md",
            "requiredRefsMissing": ["visualPackageRef"],
            "claimBoundary": "calibration-only"
        },
        "claimBoundary": "calibration-only",
        "warnings": ["Visual missing-capture fixture; calibration-only and not a contender result."]
    }
    path = out / "missing-capture-result.json"
    write_json(path, result)
    return path


def main() -> int:
    parser = argparse.ArgumentParser(description="Run v3 visual capture smokes")
    parser.add_argument("--visual-out", type=Path, default=DEFAULT_OUT, help="empty output directory for generated visual package")
    args = parser.parse_args()
    out = args.visual_out if args.visual_out.is_absolute() else ROOT / args.visual_out
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)

    manifest = load_json(VALID_MANIFEST)
    capture = manifest.get("capture")
    if not capture:
        raise SystemExit("valid visual fixture missing capture block")
    command = capture["command"]
    raw_args = capture["args"]
    artifact_root = VALID_MANIFEST.parent
    package_root = out / "visual-package"
    first_runs = out / "capture-first"
    second_runs = out / "capture-second"
    windows_meta: list[dict[str, Any]] = []
    labels: dict[str, dict[str, Any]] = {}

    for seed in SEEDS:
        for window in WINDOWS:
            label = f"artifact-01-{seed}-{window}"
            first = first_runs / str(seed) / window
            second = second_runs / str(seed) / window
            run([command, *materialize_args(raw_args, seed, window, first)], cwd=artifact_root)
            run([command, *materialize_args(raw_args, seed, window, second)], cwd=artifact_root)
            expected = ["screenshot.png", "replay.gif", "frames.json", "rubric-metadata.json"]
            for name in expected:
                a = first / name
                b = second / name
                if not a.exists() or not b.exists():
                    raise SystemExit(f"capture missing {name} for seed={seed} window={window}")
                if sha256_file(a) != sha256_file(b):
                    raise SystemExit(f"capture is non-deterministic for {name} seed={seed} window={window}")
            frames = load_json(first / "frames.json")
            screenshot_ref = f"visual-package/screenshots/{label}.png"
            replay_ref = f"visual-package/gifs/{label}.gif"
            frame_ref = f"visual-package/frames/{label}.frames.json"
            rubric_ref = f"visual-package/frames/{label}.rubric-metadata.json"
            (out / screenshot_ref).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(first / "screenshot.png", out / screenshot_ref)
            (out / replay_ref).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(first / "replay.gif", out / replay_ref)
            (out / frame_ref).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(first / "frames.json", out / frame_ref)
            shutil.copy2(first / "rubric-metadata.json", out / rubric_ref)
            windows_meta.append({
                "window": window,
                "seed": seed,
                "captureCommand": "python3 capture.py --seed {seed} --window {window} --out {out}",
                "screenshotRef": screenshot_ref,
                "screenshotChecksum": sha256_file(out / screenshot_ref),
                "replayRef": replay_ref,
                "replayChecksum": sha256_file(out / replay_ref),
                "frameMetadataRef": frame_ref,
                "rubricMetadataRef": rubric_ref,
                "fps": frames["fps"],
                "frameCount": frames["frameCount"],
                "inputSequenceRef": "visual-package/inputs/empty-inputs.json",
                "stateChecksum": frames["stateChecksum"],
                "frameChecksum": frames["frameChecksum"],
                "warnings": []
            })
            labels[label] = {"seed": seed, "window": window, "sealedLaneKey": "artifact-01"}

    write_json(package_root / "inputs" / "empty-inputs.json", {"inputs": []})
    (package_root / "blind-contact-sheet.png").write_bytes(b"v3 visual blind contact sheet placeholder\n" + json.dumps(sorted(labels), sort_keys=True).encode())
    write_json(package_root / "blind-label-map.json", {"sealed": True, "labels": labels, "openAfterRatings": True})
    write_json(package_root / "rubric-record.json", {"schemaVersion": "2000m.v3.rubric-record.v1", "status": "unrated-calibration-smoke", "ownerTasteSeparated": True})

    package = {
        "schemaVersion": "2000m.v3.visual-package.v1",
        "packageId": "v3-visual-smoke-package",
        "campaignId": "v3-visual-smoke",
        "scenarioId": "v3-visual-smoke",
        "taskSeed": 0,
        "anonymized": True,
        "mappingSealedBeforeReview": True,
        "artifactDigest": sha256_bytes(VALID_MANIFEST.read_bytes()),
        "windows": windows_meta,
        "artifacts": {
            "blindContactSheetRef": "visual-package/blind-contact-sheet.png",
            "blindLabelMapRef": "visual-package/blind-label-map.json",
            "screenshotsDirRef": "visual-package/screenshots",
            "replaysDirRef": "visual-package/gifs",
            "framesDirRef": "visual-package/frames"
        },
        "rubric": {
            "rubricRef": "v3/VISUAL_RUBRIC.md",
            "rubricRecordRef": "visual-package/rubric-record.json",
            "reviewerCount": 1,
            "ownerTasteSeparated": True
        }
    }
    package_json = out / "visual-package.json"
    write_json(package_json, package)
    blocked_result = build_blocked_result(out)
    run([sys.executable, "scripts/validate_v3_visual_packages.py", display(package_json)], cwd=ROOT)
    run([sys.executable, "scripts/validate_v3_schemas.py", display(blocked_result), str(MISSING_CAPTURE_MANIFEST.relative_to(ROOT))], cwd=ROOT)
    print(f"OK: v3 visual smokes wrote {display(package_json)} and {display(blocked_result)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
