#!/usr/bin/env python3
"""Run a small v3 private A/B pilot smoke using the frozen paired protocol.

This runner writes raw smoke artifacts to the caller-provided external run root.
The public repository receives only this reusable runner and frozen protocol
fixtures; no unredacted chat dumps or private run artifacts are written into the repo.
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PROTOCOL = ROOT / "v3" / "examples" / "campaigns" / "v3-paired-private-pilot-protocol.json"
DEFAULT_CAMPAIGN = ROOT / "v3" / "examples" / "campaigns" / "v3-paired-private-pilot.campaign.json"
VALID_ARTIFACT = ROOT / "v3" / "fixtures" / "mechanical" / "valid-artifact"


def run(cmd: list[str], *, cwd: Path = ROOT) -> None:
    print("$", " ".join(cmd))
    subprocess.run(cmd, cwd=cwd, check=True)


def load_json(path: Path) -> Any:
    return json.loads(path.read_text())


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def rel_or_abs(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def lane_smoke(lane_id: str, run_dir: Path, protocol: dict[str, Any]) -> dict[str, Any]:
    lane_dir = run_dir / f"lane-{lane_id.lower()}"
    if lane_dir.exists():
        shutil.rmtree(lane_dir)
    lane_dir.mkdir(parents=True)
    mechanical_result = lane_dir / "mechanical-result.json"
    visual_out = lane_dir / "visual"
    run(["cargo", "run", "-p", "m2000-v3-conformance", "--quiet", "--", rel_or_abs(VALID_ARTIFACT), "--json-out", rel_or_abs(mechanical_result)])
    run([sys.executable, "scripts/validate_v3_schemas.py", rel_or_abs(mechanical_result)])
    run([sys.executable, "scripts/run_v3_visual_smokes.py", "--visual-out", rel_or_abs(visual_out)])
    mechanical = load_json(mechanical_result)
    visual_package = load_json(visual_out / "visual-package.json")
    lane_record = {
        "laneId": lane_id,
        "role": "unstructured-baseline" if lane_id == "A" else "pinned-workflow-lane-smoke",
        "protocolId": protocol["protocolId"],
        "claimBoundary": "calibration-only",
        "mechanicalResultRef": str(mechanical_result),
        "visualPackageRef": str(visual_out / "visual-package.json"),
        "mechanicalRanked": mechanical["mechanical"]["ranked"],
        "mechanicalPassCount": mechanical["mechanical"]["passCount"],
        "visualWindowCount": len(visual_package["windows"]),
        "contextWipeBoundary": protocol["contextWipeBoundary"],
        "feedbackPacketRefs": [packet["sourceRef"] for packet in protocol["feedbackPackets"]],
        "rawArtifactPolicy": "external-run-root-only",
        "publicClaim": "none"
    }
    write_json(lane_dir / "lane-smoke-record.json", lane_record)
    return lane_record


def main() -> int:
    parser = argparse.ArgumentParser(description="Run v3 private paired pilot smoke into an external run root")
    parser.add_argument("--run-root", required=True, type=Path, help="external directory for private raw smoke artifacts")
    parser.add_argument("--protocol", type=Path, default=DEFAULT_PROTOCOL)
    parser.add_argument("--campaign", type=Path, default=DEFAULT_CAMPAIGN)
    args = parser.parse_args()
    run_root = args.run_root
    if not run_root.is_absolute():
        raise SystemExit("--run-root must be an absolute external path")
    if ROOT in run_root.parents or run_root == ROOT:
        raise SystemExit("--run-root must be outside the public benchmark repo")

    protocol_path = args.protocol if args.protocol.is_absolute() else ROOT / args.protocol
    campaign_path = args.campaign if args.campaign.is_absolute() else ROOT / args.campaign
    run([sys.executable, "scripts/validate_v3_campaign_protocol.py", "--protocol", rel_or_abs(protocol_path), "--campaign", rel_or_abs(campaign_path)])
    protocol = load_json(protocol_path)
    campaign = load_json(campaign_path)
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    run_dir = run_root / f"{timestamp}-v3-private-pilot-smoke"
    run_dir.mkdir(parents=True, exist_ok=False)
    shutil.copy2(protocol_path, run_dir / "frozen-protocol.json")
    shutil.copy2(campaign_path, run_dir / "frozen-campaign.json")

    lane_records = [lane_smoke("A", run_dir, protocol), lane_smoke("B", run_dir, protocol)]
    summary = {
        "schemaVersion": "2000m.v3.private-pilot-smoke.v1",
        "runKind": "private-pilot-smoke",
        "runRoot": str(run_dir),
        "campaignId": campaign["campaignId"],
        "protocolId": protocol["protocolId"],
        "lanes": lane_records,
        "claimCeiling": "private directional signal only / calibration-only",
        "publicSupportClaim": False,
        "rawArtifactsPublicRepo": False,
        "notes": "Smoke uses deterministic fixtures to exercise the frozen protocol runner; it is not a live contender campaign."
    }
    write_json(run_dir / "private-pilot-smoke-summary.json", summary)
    print(f"OK: v3 private pilot smoke wrote {run_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
