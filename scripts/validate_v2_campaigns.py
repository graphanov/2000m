#!/usr/bin/env python3
"""Validate 2000m v2 paired-campaign files.

This intentionally avoids third-party JSON Schema dependencies. The JSON Schema is
the public contract; this script enforces the invariants that matter most for not
overclaiming a workflow experiment.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CAMPAIGNS = sorted((ROOT / "v2" / "examples").glob("*.campaign.json"))
PRIVATE_MARKERS = (
    "/Users/",
    "\\Users\\",
    "file://",
    "~/",
    "../",
    "..\\",
)
BANNED_CLAIM_MARKERS = [
    "makes the model smarter",
    "one output looked cooler",
    "v1 result proves framework superiority",
    "workflow score equals model ranking",
    "adoption proof",
    "calibration fixtures are public contender results",
]


class CampaignError(ValueError):
    pass


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        raise CampaignError(f"missing JSON file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise CampaignError(f"invalid JSON in {path}: {exc}") from exc


def require(condition: bool, message: str) -> None:
    if not condition:
        raise CampaignError(message)


def is_plain_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def require_string(obj: dict[str, Any], key: str) -> str:
    value = obj.get(key)
    require(isinstance(value, str) and bool(value.strip()), f"`{key}` must be a non-empty string")
    assert isinstance(value, str)
    return value


def check_public_ref(value: str, field: str) -> None:
    lowered = value.lower().strip()
    require(not lowered.startswith("/"), f"{field} contains absolute local path: {value}")
    require(not lowered.startswith("~"), f"{field} contains tilde path: {value}")
    require(not (len(lowered) >= 2 and lowered[0].isalpha() and lowered[1] == ":"), f"{field} contains Windows local path: {value}")
    for marker in PRIVATE_MARKERS:
        require(marker.lower() not in lowered, f"{field} contains private/local marker `{marker}`: {value}")


def validate_refs(campaign: dict[str, Any]) -> None:
    frozen_refs = campaign["freezeRules"]["frozenRefs"]
    for key, ref in frozen_refs.items():
        require_string(frozen_refs, key)
        check_public_ref(ref, f"freezeRules.frozenRefs.{key}")
        path = ref.split("#", 1)[0]
        if path.endswith((".json", ".md", ".rs")):
            require((ROOT / path).exists(), f"frozen ref does not exist: {ref}")

    visual = campaign["visualTrack"]
    for key in ("specRef", "rubricRef"):
        ref = require_string(visual, key)
        check_public_ref(ref, f"visualTrack.{key}")
        path = ref.split("#", 1)[0]
        require((ROOT / path).exists(), f"visual ref does not exist: {ref}")

    for pair in campaign["pairs"]:
        scenario = require_string(pair, "scenario")
        check_public_ref(scenario, f"pairs.{pair.get('pairId', '<unknown>')}.scenario")
        require((ROOT / scenario).exists(), f"pair scenario does not exist: {scenario}")


def validate_lanes(campaign: dict[str, Any]) -> None:
    lanes = campaign["lanes"]
    require(isinstance(lanes, list), "lanes must be a list")
    by_id: dict[str, dict[str, Any]] = {}
    for lane in lanes:
        lane_id = require_string(lane, "laneId")
        require(lane_id in {"A", "B", "C"}, f"unsupported laneId `{lane_id}` in campaign schema v1")
        require(lane_id not in by_id, f"duplicate laneId `{lane_id}`")
        by_id[lane_id] = lane
        require(lane.get("role") in {"naked-model-baseline", "open-scaffold-ledger-analyze", "controller"}, f"lane {lane_id} has unsupported role")
        require(lane.get("controllerStatus") in {"not-implemented", "not-applicable"}, f"lane {lane_id} has unsupported controllerStatus")
        require(isinstance(lane.get("enabled"), bool), f"lane {lane_id} enabled must be bool")
        require(isinstance(lane.get("allowedTools"), list), f"lane {lane_id} allowedTools must be list")
        require(isinstance(lane.get("forbiddenAdvantages"), list), f"lane {lane_id} forbiddenAdvantages must be list")
        require(isinstance(lane.get("requiredEvidence"), list), f"lane {lane_id} requiredEvidence must be list")

    require({"A", "B"}.issubset(by_id), "campaign must include Lane A and Lane B")
    require(by_id["A"]["role"] == "naked-model-baseline", "Lane A must be naked-model-baseline")
    require(by_id["B"]["role"] == "open-scaffold-ledger-analyze", "Lane B must be open-scaffold-ledger-analyze")
    require(by_id["A"]["enabled"] is True, "Lane A must be enabled for paired pilot")
    require(by_id["B"]["enabled"] is True, "Lane B must be enabled for paired pilot")

    if "C" in by_id:
        lane_c = by_id["C"]
        require(lane_c["role"] == "controller", "Lane C must use controller role")
        require(lane_c["enabled"] is False, "Lane C is disabled in campaign schema v1 until a reviewed controller implementation and evidence-ref field exist")
        require(lane_c.get("controllerStatus") == "not-implemented", "Lane C controllerStatus must remain not-implemented in campaign schema v1")

    lane_a_forbidden = " ".join(by_id["A"].get("forbiddenAdvantages", [])).lower()
    require("open scaffold" in lane_a_forbidden or ".osc" in lane_a_forbidden, "Lane A must explicitly forbid Open Scaffold state/tools")
    lane_b_tools = " ".join(by_id["B"].get("allowedTools", [])).lower()
    require("evolve analyze" in lane_b_tools, "Lane B must require evolve/analyze evidence")
    require("compact evidence" in lane_b_tools, "Lane B must require compact evidence")

    enabled = {lane_id for lane_id, lane in by_id.items() if lane.get("enabled")}
    seen_pair_ids: set[str] = set()
    seen_task_seeds: set[int] = set()
    for pair in campaign["pairs"]:
        pair_id = require_string(pair, "pairId")
        require(pair_id not in seen_pair_ids, f"duplicate pairId `{pair_id}`")
        seen_pair_ids.add(pair_id)
        task_seed = pair.get("taskSeed")
        require(is_plain_int(task_seed), f"pair {pair_id} taskSeed must be integer")
        require(task_seed not in seen_task_seeds, f"duplicate taskSeed `{task_seed}` would inflate paired campaign size")
        seen_task_seeds.add(task_seed)
        pair_lanes = pair.get("enabledLanes")
        require(isinstance(pair_lanes, list) and len(pair_lanes) > 0, f"pair {pair_id} must list enabledLanes")
        assert isinstance(pair_lanes, list)
        require(set(pair_lanes).issubset(enabled), f"pair {pair_id} references disabled lane")
        require(len(pair_lanes) == 2, f"pair {pair_id} must list exactly two enabled lanes in campaign schema v1")
        require(set(pair_lanes) == {"A", "B"}, f"pair {pair_id} must be exactly the A/B pair in campaign schema v1")


def validate_controls(campaign: dict[str, Any]) -> None:
    controls = campaign["controls"]
    generation_cap = controls.get("generationCap")
    scorer_budget = controls.get("scorerFeedbackBudget")
    reviewer_budget = controls.get("reviewerFeedbackBudget")
    require(is_plain_int(generation_cap) and generation_cap >= 1, "generationCap must be integer >= 1")
    require(controls.get("samePromptBudget") is True, "samePromptBudget must be true")
    require(controls.get("sameFeedbackPackets") is True, "sameFeedbackPackets must be true")
    require(is_plain_int(scorer_budget) and scorer_budget >= 0, "scorerFeedbackBudget must be integer >= 0")
    require(is_plain_int(reviewer_budget) and reviewer_budget >= 0, "reviewerFeedbackBudget must be integer >= 0")


def validate_freeze_and_claims(campaign: dict[str, Any]) -> None:
    freeze = campaign["freezeRules"]
    require(freeze.get("freezeBeforeLiveResults") is True, "freezeBeforeLiveResults must be true")
    require(freeze.get("noScorerMutationAfterLiveResults") is True, "noScorerMutationAfterLiveResults must be true")
    require("calibration" in freeze.get("calibrationMutationPolicy", "").lower(), "calibrationMutationPolicy must define calibration handling")

    decision = campaign["decisionRules"]
    require(decision.get("predeclared") is True, "decisionRules.predeclared must be true")
    for key in ("noSupport", "directionalSignal", "repeatableSupport"):
        require_string(decision, key)

    banned = campaign.get("bannedClaims")
    require(isinstance(banned, list), "bannedClaims must be list")
    lowered = "\n".join(str(item).lower() for item in banned)
    for marker in BANNED_CLAIM_MARKERS:
        require(marker in lowered, f"bannedClaims missing marker `{marker}`")


def validate_visual_track(campaign: dict[str, Any]) -> None:
    visual = campaign["visualTrack"]
    require(visual.get("enabled") is True, "visualTrack.enabled must be true for paired pilot")
    seeds = visual.get("fixedSeeds")
    require(isinstance(seeds, list) and len(seeds) >= 1, "visualTrack.fixedSeeds must be non-empty")
    require(all(is_plain_int(seed) for seed in seeds), "visualTrack.fixedSeeds must contain integers, not booleans")
    require(len(seeds) == len(set(seeds)), "visualTrack.fixedSeeds must be unique")
    raw_artifacts = visual.get("requiredArtifacts")
    require(isinstance(raw_artifacts, list), "visualTrack.requiredArtifacts must be list")
    artifacts = set(raw_artifacts)
    for item in ("screenshot", "replay-log", "frame-metadata", "rubric-record", "blind-label-map"):
        require(item in artifacts, f"visualTrack.requiredArtifacts missing `{item}`")
    require("gif" in artifacts or "replay-log" in artifacts, "visualTrack must require GIF or replay-log")
    blind = visual.get("blindReview", {})
    require(blind.get("anonymized") is True, "blindReview.anonymized must be true")
    require(blind.get("mappingSealedBeforeReview") is True, "blindReview.mappingSealedBeforeReview must be true")
    require(blind.get("ownerTasteIsNotMechanicalScore") is True, "blindReview.ownerTasteIsNotMechanicalScore must be true")


def validate_campaign(path: Path) -> None:
    campaign = load_json(path)
    require(campaign.get("schemaVersion") == "2000m.v2.campaign.v1", f"{path} has wrong schemaVersion")
    require_string(campaign, "campaignId")
    require_string(campaign, "title")
    require(campaign.get("status") in {"draft", "frozen", "calibration", "pilot", "complete"}, f"{path} has invalid status")
    hypothesis = require_string(campaign, "hypothesis").lower()
    require("equal model/runtime/budget" in hypothesis, "hypothesis must preserve equal model/runtime/budget condition")
    require("recoverable" in hypothesis and "visually coherent" in hypothesis, "hypothesis must include recovery and visual coherence")

    for key in ("freezeRules", "controls", "lanes", "pairs", "visualTrack", "decisionRules"):
        require(key in campaign, f"{path} missing `{key}`")

    validate_refs(campaign)
    validate_controls(campaign)
    validate_lanes(campaign)
    validate_freeze_and_claims(campaign)
    validate_visual_track(campaign)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="*", help="campaign JSON files; defaults to v2/examples/*.campaign.json")
    args = parser.parse_args()
    paths = [Path(p) for p in args.paths] if args.paths else DEFAULT_CAMPAIGNS
    if not paths:
        print("no v2 campaign files found", file=sys.stderr)
        return 1
    failed = False
    for path in paths:
        try:
            validate_campaign(path if path.is_absolute() else ROOT / path)
            print(f"OK: {path}")
        except CampaignError as exc:
            failed = True
            print(f"FAIL: {path}: {exc}", file=sys.stderr)
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
