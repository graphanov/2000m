#!/usr/bin/env python3
"""Validate the v3 workflow gauntlet protocol fixture."""
from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_GAUNTLET = ROOT / "v3" / "examples" / "gauntlet" / "v3-workflow-gauntlet-v1.json"
GAUNTLET_SCHEMA = ROOT / "v3" / "workflow-gauntlet.schema.json"
SCHEMA_VALIDATOR = ROOT / "scripts" / "validate_v3_schemas.py"

REQUIRED_TRACKS = {
    "mechanical",
    "visual",
    "workflow",
    "trajectory",
    "token-cost",
    "evidence",
}
REQUIRED_ROLES = {
    "unstructured-baseline",
    "minimal-checklist-control",
    "structured-workflow-candidate",
}
REQUIRED_STRESSORS = {
    "context-wipe",
    "reviewer-feedback-resilience",
    "regression-trap",
    "stale-or-impossible-ac",
    "token-pressure",
    "handoff-resume",
    "visual-native-capture-prerequisite",
    "plateau-or-noop-retry",
}
REQUIRED_METRICS = {
    "final-mechanical-correctness-gate",
    "mechanical-ac-improvement-trajectory",
    "visual-native-capture-validity-gate",
    "visual-product-quality",
    "context-wipe-recovery",
    "reviewer-feedback-routing",
    "stale-or-impossible-ac-handling",
    "handoff-resume-quality",
    "no-op-retry-count",
    "plateau-stop-redesign-quality",
    "tokens-to-first-ranked-mechanical-ceiling",
    "token-delta-versus-unstructured",
    "tokens-per-accepted-ac-improvement",
    "compact-evidence-quality",
    "false-claim-prevention",
}
REQUIRED_REPORT_FIELDS = {
    "final-mechanical-outcome",
    "visual-blocker-status",
    "workflow-track-deltas",
    "trajectory-efficiency",
    "token-cost-deltas",
    "feedback-parity",
    "minimal-control-comparison",
    "claim-boundary",
    "unavailable-telemetry-fields",
}
FRAMEWORK_MARKERS = (
    ".osc",
    "openscaffold",
    "open-scaffold",
    "open scaffold",
)


def load_validator():
    spec = importlib.util.spec_from_file_location("validate_v3_schemas", SCHEMA_VALIDATOR)
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load v3 schema validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        raise SystemExit(f"missing JSON file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise SystemExit(f"invalid JSON in {path}: {exc}") from exc


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def validate_no_framework_coupling(gauntlet: dict[str, Any], validator: Any) -> None:
    for path, text in validator.iter_strings(gauntlet, include_keys=True):
        lowered = text.lower()
        for marker in FRAMEWORK_MARKERS:
            require(marker not in lowered, f"{path}: gauntlet must not contain framework-specific marker {marker!r}")


def validate_lanes(gauntlet: dict[str, Any]) -> None:
    lanes = gauntlet["lanes"]
    roles = {lane["role"] for lane in lanes}
    lane_ids = {lane["laneId"] for lane in lanes}
    require(roles == REQUIRED_ROLES, f"lane roles mismatch: {sorted(roles)}")
    require("C" not in lane_ids, "gauntlet must not define Lane C")
    for lane in lanes:
        require(lane["receivesSameRubric"] is True, f"{lane['laneId']} must receive the same rubric")
        require(lane["receivesSameFeedback"] is True, f"{lane['laneId']} must receive the same feedback")
        require(lane["receivesHiddenInfo"] is False, f"{lane['laneId']} must receive no hidden information")


def validate_metrics(gauntlet: dict[str, Any]) -> None:
    tracks = set(gauntlet["tracks"])
    require(tracks == REQUIRED_TRACKS, f"tracks mismatch: {sorted(tracks)}")
    metrics = {metric["metricId"]: metric for metric in gauntlet["metrics"]}
    require(set(metrics) == REQUIRED_METRICS, f"metrics mismatch: {sorted(metrics)}")

    token_metrics = [metric for metric in metrics.values() if metric["track"] == "token-cost"]
    require(bool(token_metrics), "gauntlet must include token-cost metrics")
    for metric in token_metrics:
        require(metric["valueSource"] == "runtime-telemetry", f"{metric['metricId']} must source token values from runtime telemetry")
        require(metric["mechanicalScoreCoupling"] == "none", f"{metric['metricId']} must not couple token cost to mechanical score")
        require(metric["scoreUse"] in {"track-score", "diagnostic-only"}, f"{metric['metricId']} must stay in token-cost or diagnostics")

    mechanical_gate = metrics["final-mechanical-correctness-gate"]
    require(mechanical_gate["scoreUse"] == "gate-only", "mechanical correctness must be a gate in this gauntlet")
    require(mechanical_gate["mechanicalScoreCoupling"] == "gate-only", "mechanical gate coupling must be explicit")

    visual_gate = metrics["visual-native-capture-validity-gate"]
    require(visual_gate["scoreUse"] == "gate-only", "native visual capture validity must be a gate")
    require(visual_gate["valueSource"] == "visual-package-validator", "visual gate must use visual package validation")

    evidence_metric = metrics["compact-evidence-quality"]
    require("file count" in evidence_metric["scoreBasis"].lower(), "evidence metric must explicitly reject file-count credit")
    require("no credit" in evidence_metric["scoreBasis"].lower(), "evidence metric must explicitly reject volume credit")


def validate_decision_rules(gauntlet: dict[str, Any]) -> None:
    rules = gauntlet["decisionRules"]
    require(rules["minimalControlRequired"] is True, "minimal checklist control is required")
    require("C" in set(rules["forbiddenLaneIds"]), "Lane C must be explicitly forbidden")
    require(rules["visualRankingRequiresNativeCaptureValid"] is True, "visual ranking must require native capture validity")
    require(rules["tokenMetricsSeparateFromMechanical"] is True, "token metrics must stay separate from mechanical score")
    require(rules["evidenceVolumeScoreCredit"] is False, "evidence volume must not score")
    require(rules["minTaskSeeds"] >= 12, "gauntlet needs enough seeds to be more discriminating than the private pilot")
    require(rules["maxGenerations"] <= 3, "generation cap must preserve pressure and avoid unbounded retries")
    support_text = " ".join(rules["supportCandidateRequires"]).lower()
    require("both controls" in support_text, "support criteria must compare against both controls")
    require("minimal checklist" in support_text, "support criteria must report the minimal checklist control")
    no_support_text = " ".join(rules["noSupportTriggers"]).lower()
    require("token" in no_support_text, "no-support triggers must include token waste")
    require("visual" in no_support_text, "no-support triggers must include visual blocker misuse")


def validate_scenarios(gauntlet: dict[str, Any]) -> None:
    stressors = {family["stressor"] for family in gauntlet["scenarioFamilies"]}
    require(stressors == REQUIRED_STRESSORS, f"stressor mismatch: {sorted(stressors)}")
    for family in gauntlet["scenarioFamilies"]:
        require(family["requiredForFreeze"] is True, f"{family['familyId']} must be required for freeze")


def validate_reporting(gauntlet: dict[str, Any]) -> None:
    reporting = gauntlet["reporting"]
    require(set(reporting["reportMustShow"]) == REQUIRED_REPORT_FIELDS, "reporting fields must expose every separated track and parity item")
    require(reporting["claimCeiling"] == "no-public-result-until-frozen-and-run", "fixture must not allow public result claims")
    prereq_text = " ".join(reporting["publicResultPrerequisites"]).lower()
    require("native capture" in prereq_text, "public result prerequisites must include native capture validity")
    require("token telemetry" in prereq_text, "public result prerequisites must include token telemetry source labeling")


def validate_gauntlet(path: Path, validator: Any) -> None:
    gauntlet = load_json(path)
    schema = load_json(GAUNTLET_SCHEMA)
    validator.validate_json_schema(gauntlet, schema)
    validator.check_public_safe_strings(gauntlet, allow_banned_claim_list=True)
    validator.check_unsupported_claims(gauntlet)
    validate_no_framework_coupling(gauntlet, validator)
    require(gauntlet["status"] == "frozen-candidate", "default gauntlet fixture should be a frozen candidate, not a live result")
    require(gauntlet["claimBoundary"] == "predeclared-future-protocol", "gauntlet fixture is a protocol, not evidence")
    validate_lanes(gauntlet)
    validate_scenarios(gauntlet)
    validate_metrics(gauntlet)
    validate_decision_rules(gauntlet)
    validate_reporting(gauntlet)
    print(f"OK: v3 workflow gauntlet {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate v3 workflow gauntlet protocol fixture")
    parser.add_argument("paths", nargs="*", help="gauntlet JSON files; defaults to the public v1 fixture")
    args = parser.parse_args()
    validator = load_validator()
    paths = [Path(p) if Path(p).is_absolute() else ROOT / p for p in args.paths] if args.paths else [DEFAULT_GAUNTLET]
    for path in paths:
        validate_gauntlet(path, validator)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
