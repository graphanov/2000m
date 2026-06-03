#!/usr/bin/env python3
"""Run synthetic v3 feedback-parity guard smoke coverage.

The smoke writes only temporary/public-safe fixtures. It proves the guard accepts
record/workspace feedback packets containing exact scorer diagnostics and fails
closed when scorer-feedback.md omits a required exact diagnostic line.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SEEDS = [101, 202, 303]
LANES = ["A", "B"]
EXACT_LINE = "M06 schema validity: state enum value 'teleport' is not allowed"


def result_payload() -> dict[str, Any]:
    return {
        "schemaVersion": "2000m.v3.result.v1",
        "campaignId": "v3-feedback-parity-smoke",
        "scenarioId": "v3-feedback-parity-smoke",
        "taskSeed": 101,
        "laneId": "A",
        "entrant": {
            "model": "fixture-driver",
            "runtime": "synthetic-smoke",
            "processType": "scripted-agent",
        },
        "protocolFreeze": {
            "changedAfterLiveResults": False,
            "scorerMutationObserved": False,
            "calibrationOnlyIfChanged": True,
        },
        "mechanical": {
            "ranked": False,
            "protocolVersion": "2000m.driver.v3",
            "passCount": 1,
            "totalAcs": 2,
            "compositeScore": 50.0,
            "determinism": {
                "pass": True,
                "details": "synthetic determinism details stable",
            },
            "failedAcs": ["M06"],
            "acs": [
                {
                    "id": "M01",
                    "name": "manifest/protocol validity",
                    "pass": True,
                    "detail": "manifest protocol is valid",
                },
                {
                    "id": "M06",
                    "name": "schema validity",
                    "pass": False,
                    "detail": "state enum value 'teleport' is not allowed",
                },
            ],
            "hiddenChallengeSummary": "0 hidden fixtures were attempted; see failedAcs",
            "regressionSummary": "synthetic regression details stable",
            "resultJsonRef": "records/pilot-seed-101/lane-A/generation-01/v3-result.json",
        },
        "visual": {
            "ranked": False,
            "blockReason": "missing-native-capture-or-playable-surface",
            "visualPackageRef": "",
            "captureDeterminism": "blocked",
            "rubricRecordRef": "",
        },
        "workflow": {
            "contextWipeRecoveryScore": 0.0,
            "feedbackDecisionScore": 0.0,
            "regressionProtectionScore": 0.0,
            "impossibleRequirementHandlingScore": 0.0,
            "handoffScore": 0.0,
            "finalRecommendation": "inspect_scorer",
            "rationaleRefs": ["v3/MECHANICAL_AC_SPEC.md"],
        },
        "evidence": {
            "replayable": True,
            "publicSafe": True,
            "privateRefsBlocked": False,
            "compactSummaryRef": "synthetic-v3-feedback-parity-smoke",
            "requiredRefsMissing": [],
            "claimBoundary": "calibration-only",
        },
        "claimBoundary": "calibration-only",
        "warnings": ["Synthetic v3 feedback parity smoke fixture."],
    }


def write_generation(run_root: Path, seed: int, lane: str, *, include_exact: bool) -> None:
    payload = result_payload()
    payload["taskSeed"] = seed
    payload["laneId"] = lane
    payload["mechanical"]["resultJsonRef"] = (
        f"records/pilot-seed-{seed}/lane-{lane}/generation-01/v3-result.json"
    )
    feedback_lines = [
        "# Scorer feedback",
        "",
        "## Exact scorer diagnostics",
        "",
    ]
    if include_exact:
        feedback_lines.append(EXACT_LINE)
    feedback = "\n".join(feedback_lines) + "\n"
    score_log = "synthetic v3 score log\n"
    manifest = {
        "schemaVersion": "2000m.v3.feedback-manifest.v1",
        "diagnosticSource": "v3-result.json",
        "feedbackRef": "scorer-feedback.md",
    }
    destinations = [
        run_root / "records" / f"pilot-seed-{seed}" / f"lane-{lane}" / "generation-01",
        run_root
        / "workspaces"
        / f"2000m-v3-private-pilot-seed-{seed}-lane-{lane}"
        / "trajectory"
        / "generation-01",
    ]
    for directory in destinations:
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "v3-result.json").write_text(json.dumps(payload, indent=2) + "\n")
        (directory / "score.log").write_text(score_log)
        (directory / "scorer-feedback.md").write_text(feedback)
        if "workspaces" in directory.parts:
            (directory / "feedback-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


def run_guard(run_root: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "scripts/check_v3_feedback_parity.py",
            str(run_root),
            "--generation-cap",
            "1",
            "--json-out",
            str(run_root / "records" / "feedback-parity-all.json"),
        ],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="m2000-v3-feedback-parity-") as temp:
        temp_root = Path(temp)
        pass_root = temp_root / "pass"
        fail_root = temp_root / "fail"
        for seed in SEEDS:
            for lane in LANES:
                write_generation(pass_root, seed, lane, include_exact=True)
                write_generation(fail_root, seed, lane, include_exact=True)
        write_generation(fail_root, 101, "A", include_exact=False)

        passing = run_guard(pass_root)
        if passing.returncode != 0:
            print(passing.stdout, end="")
            print(passing.stderr, end="", file=sys.stderr)
            raise AssertionError("expected v3 feedback parity guard to pass complete exact diagnostics")

        failing = run_guard(fail_root)
        if failing.returncode == 0:
            print(failing.stdout, end="")
            print(failing.stderr, end="", file=sys.stderr)
            raise AssertionError("expected v3 feedback parity guard to fail when exact diagnostics are missing")

    print("OK: v3 feedback parity smoke accepted exact diagnostics and rejected missing diagnostics")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
