#!/usr/bin/env python3
"""Validate the frozen v3 paired campaign protocol and campaign fixture."""
from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PROTOCOL = ROOT / "v3" / "examples" / "campaigns" / "v3-paired-private-pilot-protocol.json"
DEFAULT_CAMPAIGN = ROOT / "v3" / "examples" / "campaigns" / "v3-paired-private-pilot.campaign.json"
SCHEMA_VALIDATOR = ROOT / "scripts" / "validate_v3_schemas.py"
REQUIRED_WINDOWS = ["early-game", "mid-run-obstacle-field", "ramp-style-sequence", "monster-pressure", "post-feedback-rerun"]
RUNTIME_FIELDS = {"model", "codexCliVersion", "pythonVersion", "rustVersion", "cargoVersion", "networkPolicy"}


def load_validator():
    spec = importlib.util.spec_from_file_location("validate_v3_schemas", SCHEMA_VALIDATOR)
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load v3 schema validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_json(path: Path) -> Any:
    return json.loads(path.read_text())


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def validate_protocol(protocol: dict[str, Any], campaign: dict[str, Any]) -> None:
    require(protocol.get("schemaVersion") == "2000m.v3.paired-protocol.v1", "protocol schemaVersion mismatch")
    require(protocol.get("status") == "frozen", "protocol must be frozen")
    require(protocol.get("claimCeiling") == "private-directional-signal-or-calibration-only", "claim ceiling must remain private/calibration")
    require(set(protocol.get("runtime", {})) >= RUNTIME_FIELDS, "runtime/tool version fields are incomplete")
    lanes = {lane["laneId"]: lane for lane in protocol["lanes"]}
    require(set(lanes) == {"A", "B"}, "protocol must contain exactly Lane A and Lane B")
    require(lanes["A"].get("forbiddenCampaignSpecificWorkflow") is True, "Lane A must forbid the workflow under test")
    require(lanes["A"].get("receivesFullRubric") is True and lanes["B"].get("receivesFullRubric") is True, "both lanes must receive full rubric")
    require(lanes["A"].get("receivesSameFeedback") is True and lanes["B"].get("receivesSameFeedback") is True, "both lanes must receive same feedback")
    require(lanes["B"].get("receivesHiddenInfo") is False, "Lane B must receive no hidden info")
    require("Lane A" in protocol["prompts"]["laneA"], "Lane A prompt must be exact and present")
    require("Lane B" in protocol["prompts"]["laneB"], "Lane B prompt must be exact and present")
    require("same" in protocol["prompts"]["laneB"].lower(), "Lane B prompt must state same rubric/feedback boundary")
    require(protocol["contextWipeBoundary"] == campaign["controls"]["contextWipePhase"], "context-wipe boundary mismatch")
    require(protocol["captureWindows"] == REQUIRED_WINDOWS, "capture windows must be the frozen v3 standard windows")
    require(protocol["captureWindows"] == campaign["protocolFreeze"]["captureWindows"], "campaign capture windows differ from protocol")
    require(protocol["seeds"]["visualSeeds"] == campaign["protocolFreeze"]["visualSeeds"], "campaign visual seeds differ from protocol")
    require(campaign["controls"]["sameFeedbackPackets"] is True and campaign["controls"]["sameScorerDiagnostics"] is True, "feedback/scorer parity must be true")
    require(campaign["controls"]["samePromptBudget"] is True and campaign["controls"]["sameReviewerBudget"] is True, "budget parity must be true")
    thresholds = protocol["thresholds"]
    require(thresholds["predeclared"] is True, "thresholds must be predeclared")
    require(thresholds["publicSupportAllowed"] is False, "private pilot smoke must not allow public support claims")
    require(thresholds["visualClaimsRequireRankedCapture"] is True, "visual claims must require ranked capture")
    workflow = protocol["workflowUnderTest"]
    require(workflow["pinRequiredBeforeLiveRun"] is True, "workflow-under-test pin must be required before live run")
    require(workflow["receivesHiddenInfo"] is False, "workflow-under-test must receive no hidden info")
    if workflow["enabled"] and "open scaffold" in workflow["name"].lower():
        require(workflow["version"] not in {"", "not-enabled-for-calibration-smoke"}, "enabled Open Scaffold campaign requires version pin")
        require(workflow["commit"] not in {"", "not-enabled-for-calibration-smoke"}, "enabled Open Scaffold campaign requires commit pin")
    require(protocol["publicSafety"]["rawArtifactsInPublicRepo"] is False, "raw private artifacts must remain outside public repo")
    require(protocol["publicSafety"]["claimBoundary"] == "calibration-only", "protocol public-safety claim boundary mismatch")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate frozen v3 paired campaign protocol")
    parser.add_argument("--protocol", type=Path, default=DEFAULT_PROTOCOL)
    parser.add_argument("--campaign", type=Path, default=DEFAULT_CAMPAIGN)
    args = parser.parse_args()
    protocol_path = args.protocol if args.protocol.is_absolute() else ROOT / args.protocol
    campaign_path = args.campaign if args.campaign.is_absolute() else ROOT / args.campaign
    validator = load_validator()
    validator.validate_fixture(campaign_path)
    protocol = load_json(protocol_path)
    campaign = load_json(campaign_path)
    validator.check_public_safe_strings(protocol, allow_banned_claim_list=True)
    validator.check_unsupported_claims(protocol)
    validate_protocol(protocol, campaign)
    print(f"OK: v3 frozen campaign protocol {protocol_path.relative_to(ROOT)} + {campaign_path.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
