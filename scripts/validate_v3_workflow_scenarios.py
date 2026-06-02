#!/usr/bin/env python3
"""Validate v3 workflow-resilience calibration scenarios and golden fixtures."""
from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SCENARIO = ROOT / "v3" / "examples" / "workflow" / "scenarios" / "v3-workflow-calibration.scenario.json"
SCENARIO_SCHEMA = ROOT / "v3" / "workflow-scenario.schema.json"
SCHEMA_VALIDATOR = ROOT / "scripts" / "validate_v3_schemas.py"
REQUIRED_STRESSORS = {
    "context-wipe",
    "reviewer-feedback-valid-misleading-stale",
    "regression-trap",
    "impossible-or-probe-only-requirement",
    "scorer-feedback-integration",
    "final-handoff",
}
REQUIRED_KINDS = {
    "complete-ranked",
    "weak-ranked",
    "missing-visual-rank-blocked",
    "wrong-feedback-routing",
    "wrong-stop-decision",
    "private-path-rank-blocked",
    "scorer-mutated-invalid",
}


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


def resolve(ref: str) -> Path:
    require(ref, "fixture ref is empty")
    require(not ref.startswith("/"), f"fixture ref is absolute: {ref}")
    require("../" not in ref and "..\\" not in ref, f"fixture ref contains traversal: {ref}")
    return ROOT / ref


def ref_path(ref: str) -> Path:
    return resolve(ref.split("#", 1)[0])


def require_public_ref_exists(ref: str, label: str, *, allow_empty: bool = False) -> None:
    if not ref:
        require(allow_empty, f"{label} is empty")
        return
    path = ref_path(ref)
    require(path.exists(), f"{label} points at missing committed file: {ref}")


def validate_result_refs(result: dict[str, Any], fixture_kind: str) -> None:
    evidence = result["evidence"]
    missing = set(evidence.get("requiredRefsMissing", []))
    if evidence.get("publicSafe") is True:
        require_public_ref_exists(evidence["compactSummaryRef"], f"{fixture_kind} compactSummaryRef")

    visual = result["visual"]
    if visual.get("ranked") is True:
        require_public_ref_exists(visual["visualPackageRef"], f"{fixture_kind} visualPackageRef")
        require_public_ref_exists(visual["rubricRecordRef"], f"{fixture_kind} rubricRecordRef")
    else:
        if visual.get("visualPackageRef"):
            require_public_ref_exists(visual["visualPackageRef"], f"{fixture_kind} visualPackageRef")
        else:
            require("visualPackageRef" in missing, f"{fixture_kind} must list missing visualPackageRef when visual package is absent")
        if visual.get("rubricRecordRef"):
            require_public_ref_exists(visual["rubricRecordRef"], f"{fixture_kind} rubricRecordRef")


def validate_run_record_refs(run_record: dict[str, Any], fixture_kind: str) -> None:
    for phase in run_record.get("phases", []):
        for item in phase.get("evidenceRefs", []):
            require_public_ref_exists(item["ref"], f"{fixture_kind} phase evidence ref")
    visual = run_record["visual"]
    if visual.get("captureCommandResultRef"):
        require_public_ref_exists(visual["captureCommandResultRef"], f"{fixture_kind} captureCommandResultRef")
    if visual.get("ranked") is True:
        require_public_ref_exists(visual["visualPackageRef"], f"{fixture_kind} run-record visualPackageRef")


def validate_scenario(path: Path, validator: Any) -> dict[str, Any]:
    scenario = load_json(path)
    schema = load_json(SCENARIO_SCHEMA)
    validator.validate_json_schema(scenario, schema)
    validator.check_public_safe_strings(scenario, allow_banned_claim_list=True)
    validator.check_unsupported_claims(scenario)
    require(set(scenario["stressors"]) == REQUIRED_STRESSORS, "scenario stressors must cover every Gate 4 stressor exactly")
    kinds = {fixture["kind"] for fixture in scenario["goldenFixtures"]}
    require(kinds == REQUIRED_KINDS, f"scenario golden fixture kinds mismatch: {sorted(kinds)}")
    feedback_kinds = {item["kind"] for item in scenario["reviewerPacket"]["items"]}
    for expected in {"valid-mechanical", "valid-visual", "misleading", "ambiguous", "stale-impossible", "scorer-diagnostic"}:
        require(expected in feedback_kinds, f"reviewer packet missing {expected}")
    return scenario


def validate_result_tracks(result: dict[str, Any], fixture_kind: str) -> None:
    require("compositeScore" not in result, "v3 result must not expose a top-level compositeScore")
    require(result["claimBoundary"] == "calibration-only", f"{fixture_kind} must be calibration-only")
    require(result["evidence"]["claimBoundary"] == result["claimBoundary"], f"{fixture_kind} evidence claimBoundary mismatch")
    for track in ("mechanical", "visual", "workflow", "evidence"):
        require(track in result, f"{fixture_kind} missing {track} track")
    workflow = result["workflow"]
    for field in ["contextWipeRecoveryScore", "feedbackDecisionScore", "regressionProtectionScore", "impossibleRequirementHandlingScore", "handoffScore"]:
        require(isinstance(workflow[field], (int, float)), f"{fixture_kind} {field} must remain separate numeric field")
    if fixture_kind == "missing-visual-rank-blocked":
        require(result["mechanical"]["ranked"] is True, "missing visual fixture must preserve mechanical ranking")
        require(result["visual"]["ranked"] is False, "missing visual fixture must block visual ranking")
        require(result["visual"]["blockReason"] == "missing-native-capture-or-playable-surface", "missing visual fixture has wrong block reason")
    if fixture_kind == "wrong-feedback-routing":
        require(result["workflow"]["feedbackDecisionScore"] == 0, "wrong feedback fixture must isolate feedback routing failure")
    if fixture_kind == "wrong-stop-decision":
        require(result["workflow"]["finalRecommendation"] == "continue", "wrong stop fixture must encode wrong continue decision")
        require(result["workflow"]["impossibleRequirementHandlingScore"] < 50, "wrong stop fixture must lower impossible/stale handling score")
    validate_result_refs(result, fixture_kind)


def validate_run_record(run_record: dict[str, Any], fixture_kind: str) -> None:
    require(run_record["evidence"]["claimBoundary"] == "calibration-only", f"{fixture_kind} run record must be calibration-only")
    phase_kinds = {phase["kind"] for phase in run_record["phases"]}
    for expected in ["context-wipe-recovery", "reviewer-injection", "regression-trap", "stale-or-impossible-requirement", "final-handoff"]:
        require(expected in phase_kinds, f"{fixture_kind} run record missing phase {expected}")
    if fixture_kind == "wrong-feedback-routing":
        reviewer = next(phase for phase in run_record["phases"] if phase["kind"] == "reviewer-injection")
        decisions = {item["decision"] for item in reviewer.get("feedbackDecisions", [])}
        require(decisions == {"accepted"}, "wrong feedback fixture should blindly accept every reviewer item")
    validate_run_record_refs(run_record, fixture_kind)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate v3 workflow-resilience scenario pack")
    parser.add_argument("paths", nargs="*", help="scenario JSON files; defaults to the public calibration scenario")
    args = parser.parse_args()
    validator = load_validator()
    paths = [Path(p) if Path(p).is_absolute() else ROOT / p for p in args.paths] if args.paths else [DEFAULT_SCENARIO]
    for path in paths:
        scenario = validate_scenario(path, validator)
        for fixture in scenario["goldenFixtures"]:
            kind = fixture["kind"]
            if kind == "private-path-rank-blocked":
                result_path = resolve(fixture["resultRef"])
                validator.validate_fixture(result_path)
                result = load_json(result_path)
                require(result["visual"]["blockReason"] == "private-local-ref-in-public-record", "private-path fixture must rank-block visual/evidence")
                require("PRIVATE_LOCAL_PATH_SENTINEL" in result["evidence"]["compactSummaryRef"], "private-path fixture must use a synthetic sentinel, not a real local path")
                transcript_marker = "raw" + "-" + "transcript"
                require(transcript_marker not in result["evidence"]["compactSummaryRef"], "private-path fixture must not use transcript-like public refs")
                print(f"OK invalid: {fixture['resultRef']} -> synthetic private-local sentinel rank-blocked")
                continue
            if kind == "scorer-mutated-invalid":
                result_path = resolve(fixture["resultRef"])
                try:
                    validator.validate_fixture(result_path)
                except Exception as exc:
                    message = str(exc).lower()
                    require("frozen protocol mutation" in message, f"{kind} failed for unexpected reason: {exc}")
                    print(f"OK invalid: {fixture['resultRef']} -> {exc}")
                else:
                    raise SystemExit(f"invalid fixture unexpectedly passed: {fixture['resultRef']}")
                continue
            run_record_path = resolve(fixture["runRecordRef"])
            result_path = resolve(fixture["resultRef"])
            validator.validate_fixture(run_record_path)
            validator.validate_fixture(result_path)
            run_record = load_json(run_record_path)
            result = load_json(result_path)
            validate_run_record(run_record, kind)
            validate_result_tracks(result, kind)
            print(f"OK valid: {fixture['runRecordRef']} + {fixture['resultRef']}")
        print(f"OK: v3 workflow scenario {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
