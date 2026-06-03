#!/usr/bin/env python3
"""Run the v3 workflow-gauntlet calibration smoke fixture.

This script is intentionally fixture-only. It validates the neutral gauntlet
protocol, loads generic v3 run-record/result fixtures, computes separated-track
summaries, and emits an honest no-support-style calibration report when the
structured candidate fails to beat the controls.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path
from typing import Any, Mapping

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURE = ROOT / "v3" / "examples" / "gauntlet" / "smoke" / "v3-workflow-gauntlet-smoke.fixture.json"
SCHEMA_VALIDATOR = ROOT / "scripts" / "validate_v3_schemas.py"
GAUNTLET_VALIDATOR = ROOT / "scripts" / "validate_v3_workflow_gauntlet.py"

REQUIRED_LANE_IDS = {"U", "M", "W"}
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
WORKFLOW_SCORE_FIELDS = [
    "contextWipeRecoveryScore",
    "feedbackDecisionScore",
    "regressionProtectionScore",
    "impossibleRequirementHandlingScore",
    "handoffScore",
]


class GauntletSmokeError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise GauntletSmokeError(message)


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise GauntletSmokeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        raise GauntletSmokeError(f"missing JSON file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise GauntletSmokeError(f"invalid JSON in {path}: {exc}") from exc


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def rel(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def resolve_ref(ref: str, label: str) -> Path:
    require(bool(ref.strip()), f"{label} is empty")
    raw = ref.split("#", 1)[0]
    require(not raw.startswith("/"), f"{label} is absolute: {ref}")
    require("../" not in raw and "..\\" not in raw, f"{label} contains traversal: {ref}")
    path = ROOT / raw
    require(path.exists(), f"{label} points at missing file: {ref}")
    return path


def workflow_score(result: dict[str, Any]) -> float:
    workflow = result["workflow"]
    return round(sum(float(workflow[field]) for field in WORKFLOW_SCORE_FIELDS) / len(WORKFLOW_SCORE_FIELDS), 6)


def token_total(run_record: dict[str, Any]) -> int | None:
    telemetry = run_record.get("runtimeTelemetry")
    if telemetry is None:
        return None
    return telemetry["tokens"]["total"]


def feedback_ids(run_record: dict[str, Any]) -> list[str]:
    ids: list[str] = []
    for phase in run_record["phases"]:
        if phase["kind"] != "reviewer-injection":
            continue
        ids.extend(item["feedbackId"] for item in phase.get("feedbackDecisions", []))
    return sorted(ids)


def has_repair_hypothesis(run_record: dict[str, Any]) -> bool:
    for phase in run_record["phases"]:
        outputs = phase.get("outputs", {})
        hypothesis = outputs.get("repairHypothesis")
        if isinstance(hypothesis, str) and hypothesis.strip():
            return True
    return False


def validate_fixture_shape(fixture: dict[str, Any], schema_validator: Any) -> None:
    require(fixture.get("schemaVersion") == "2000m.v3.workflow-gauntlet-smoke-fixture.v1", "smoke fixture schemaVersion mismatch")
    require(fixture.get("claimBoundary") == "calibration-only", "smoke fixture must remain calibration-only")
    require(fixture.get("fixtureKind") == "protocol-smoke", "smoke fixture must be protocol-smoke")
    schema_validator.check_public_safe_strings(fixture, allow_banned_claim_list=True)
    schema_validator.check_unsupported_claims(fixture)
    claims = fixture.get("declaredClaims", {})
    require(claims.get("visualProductClaim") is False, "fixture must not declare a visual product claim")
    require(claims.get("workflowValueClaim") is False, "fixture must not declare a workflow-value claim")


def validate_protocol(gauntlet_path: Path, gauntlet_validator: Any, schema_validator: Any) -> dict[str, Any]:
    gauntlet_validator.validate_gauntlet(gauntlet_path, schema_validator)
    gauntlet = load_json(gauntlet_path)
    lane_ids = {lane["laneId"] for lane in gauntlet["lanes"]}
    require(
        lane_ids == REQUIRED_LANE_IDS and len(gauntlet["lanes"]) == len(REQUIRED_LANE_IDS),
        f"v1 gauntlet lanes must be exactly {sorted(REQUIRED_LANE_IDS)}, got {sorted(lane_ids)} from {len(gauntlet['lanes'])} lane records",
    )
    stressors = {family["stressor"] for family in gauntlet["scenarioFamilies"]}
    require(stressors == REQUIRED_STRESSORS, f"gauntlet stressors mismatch: {sorted(stressors)}")
    report_fields = set(gauntlet["reporting"]["reportMustShow"])
    require(report_fields == REQUIRED_REPORT_FIELDS, "gauntlet report fields must expose every required final field")
    return gauntlet


def validate_scenario_pack(fixture: dict[str, Any], gauntlet: dict[str, Any]) -> None:
    families = {item["familyId"]: item for item in gauntlet["scenarioFamilies"]}
    pack = fixture.get("scenarioPack", [])
    require(isinstance(pack, list) and bool(pack), "smoke fixture must define scenarioPack")
    seen_stressors = set()
    for item in pack:
        family_id = item.get("familyId")
        require(family_id in families, f"scenario pack family is not in gauntlet: {family_id}")
        stressor = item.get("stressor")
        require(stressor == families[family_id]["stressor"], f"scenario pack stressor mismatch for {family_id}")
        seen_stressors.add(stressor)
        require(item.get("calibrationOnly") is True, f"{family_id} must be marked calibration-only")
        resolve_ref(item["fixtureRef"], f"{family_id} fixtureRef")
    require(seen_stressors == REQUIRED_STRESSORS, f"scenario pack stressors mismatch: {sorted(seen_stressors)}")


def load_records(fixture: dict[str, Any], schema_validator: Any) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for item in fixture.get("records", []):
        lane_id = item.get("laneId")
        require(lane_id in REQUIRED_LANE_IDS, f"record laneId must be one of U/M/W: {lane_id}")
        require(lane_id != "C", "record must not use Lane C")
        generation = item.get("generation")
        require(isinstance(generation, int) and not isinstance(generation, bool) and generation >= 1, "record generation must be a positive integer")
        task_seed = item.get("taskSeed")
        require(isinstance(task_seed, int) and not isinstance(task_seed, bool), "record taskSeed must be an integer")
        run_record_path = resolve_ref(item["runRecordRef"], f"{lane_id} generation {generation} runRecordRef")
        result_path = resolve_ref(item["resultRef"], f"{lane_id} generation {generation} resultRef")
        schema_validator.validate_fixture(run_record_path)
        schema_validator.validate_fixture(result_path)
        run_record = load_json(run_record_path)
        result = load_json(result_path)
        for field in ["campaignId", "scenarioId", "taskSeed", "laneId"]:
            require(run_record[field] == result[field], f"{rel(run_record_path)} and {rel(result_path)} mismatch on {field}")
        require(run_record["laneId"] == lane_id, f"{rel(run_record_path)} laneId mismatch")
        require(run_record["taskSeed"] == task_seed, f"{rel(run_record_path)} taskSeed mismatch")
        require(result["claimBoundary"] == "calibration-only", f"{rel(result_path)} must be calibration-only")
        require(result["evidence"]["claimBoundary"] == "calibration-only", f"{rel(result_path)} evidence must be calibration-only")
        mechanical_text = json.dumps(result["mechanical"], sort_keys=True).lower()
        require("token" not in mechanical_text and "cost" not in mechanical_text, f"{rel(result_path)} mechanical track must not contain token/cost fields")
        require(token_total(run_record) is not None, f"{rel(run_record_path)} must include source-labeled total token telemetry")
        records.append({
            "laneId": lane_id,
            "taskSeed": task_seed,
            "generation": generation,
            "runRecordRef": rel(run_record_path),
            "resultRef": rel(result_path),
            "runRecord": run_record,
            "result": result,
            "workflowScore": workflow_score(result),
            "tokens": token_total(run_record),
        })
    lanes = {record["laneId"] for record in records}
    require(lanes == REQUIRED_LANE_IDS, f"smoke records must include exactly lanes U/M/W, got {sorted(lanes)}")
    return sorted(records, key=lambda record: (record["taskSeed"], record["laneId"], record["generation"]))


def selected_by_lane(records: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    by_lane: dict[str, list[dict[str, Any]]] = {}
    for record in records:
        by_lane.setdefault(record["laneId"], []).append(record)
    selected: dict[str, dict[str, Any]] = {}
    for lane, rows in by_lane.items():
        selected[lane] = max(
            rows,
            key=lambda row: (
                int(row["result"]["mechanical"].get("passCount") or 0),
                float(row["result"]["mechanical"].get("compositeScore") or 0),
                int(row["generation"]),
            ),
        )
    return selected


def mechanical_report(selected: dict[str, dict[str, Any]]) -> dict[str, Any]:
    by_lane: dict[str, Any] = {}
    signatures = set()
    gated = False
    for lane, record in sorted(selected.items()):
        mechanical = record["result"]["mechanical"]
        signature = (mechanical["ranked"], mechanical["passCount"], mechanical["totalAcs"])
        signatures.add(signature)
        gated = gated or mechanical["ranked"] is not True
        by_lane[lane] = {
            "generation": record["generation"],
            "ranked": mechanical["ranked"],
            "passCount": mechanical["passCount"],
            "totalAcs": mechanical["totalAcs"],
            "failedAcs": mechanical.get("failedAcs", []),
            "resultRef": record["resultRef"],
        }
    return {
        "status": "gated" if gated else "tied" if len(signatures) == 1 else "unequal",
        "byLane": by_lane,
    }


def visual_report(selected: dict[str, dict[str, Any]], fixture: dict[str, Any]) -> dict[str, Any]:
    by_lane: dict[str, Any] = {}
    blocked = False
    for lane, record in sorted(selected.items()):
        visual = record["result"]["visual"]
        is_blocked = visual["ranked"] is not True
        blocked = blocked or is_blocked
        by_lane[lane] = {
            "generation": record["generation"],
            "ranked": visual["ranked"],
            "blockReason": visual["blockReason"],
            "captureDeterminism": visual["captureDeterminism"],
            "visualPackageRef": visual["visualPackageRef"],
        }
    claims = fixture.get("declaredClaims", {})
    require(not (blocked and claims.get("visualProductClaim") is True), "visual/product claim is blocked by invalid native capture")
    return {
        "blocked": blocked,
        "nativeCaptureRequired": True,
        "claimBlocked": blocked,
        "byLane": by_lane,
    }


def workflow_report(selected: dict[str, dict[str, Any]]) -> dict[str, Any]:
    scores = {lane: record["workflowScore"] for lane, record in selected.items()}
    candidate = scores["W"]
    return {
        "scoreByLane": {lane: scores[lane] for lane in sorted(scores)},
        "candidateMinusUnstructured": round(candidate - scores["U"], 6),
        "candidateMinusMinimalControl": round(candidate - scores["M"], 6),
        "candidateBeatsBothControls": candidate > scores["U"] and candidate > scores["M"],
    }


def trajectory_report(records: list[dict[str, Any]]) -> dict[str, Any]:
    by_lane: dict[str, list[dict[str, Any]]] = {}
    for record in records:
        by_lane.setdefault(record["laneId"], []).append(record)
    summaries: dict[str, Any] = {}
    missing_hypotheses: list[dict[str, Any]] = []
    for lane, rows in sorted(by_lane.items()):
        rows = sorted(rows, key=lambda row: row["generation"])
        previous_pass: int | None = None
        previous_workflow: float | None = None
        no_op_count = 0
        consecutive_no_op = 0
        plateau = False
        first_ceiling_generation: int | None = None
        trajectory = []
        for row in rows:
            mechanical = row["result"]["mechanical"]
            current_pass = mechanical["passCount"]
            total_acs = mechanical["totalAcs"]
            if mechanical["ranked"] is True and current_pass == total_acs and first_ceiling_generation is None:
                first_ceiling_generation = row["generation"]
            no_improvement = False
            if previous_pass is not None and previous_workflow is not None:
                no_improvement = current_pass <= previous_pass and row["workflowScore"] <= previous_workflow
                if no_improvement:
                    no_op_count += 1
                    consecutive_no_op += 1
                    plateau = plateau or consecutive_no_op >= 2
                else:
                    consecutive_no_op = 0
            decision = row["runRecord"]["finalRecommendation"]["decision"]
            needs_repair_hypothesis = row["generation"] > 1 and (no_improvement or decision == "continue")
            has_hypothesis = has_repair_hypothesis(row["runRecord"])
            if needs_repair_hypothesis and not has_hypothesis:
                missing_hypotheses.append({
                    "laneId": lane,
                    "generation": row["generation"],
                    "runRecordRef": row["runRecordRef"],
                    "reason": "retry-or-continue generation lacks repairHypothesis",
                })
            trajectory.append({
                "generation": row["generation"],
                "passCount": current_pass,
                "totalAcs": total_acs,
                "workflowScore": row["workflowScore"],
                "noAcceptedMetricImprovement": no_improvement,
                "finalRecommendation": decision,
                "hasRepairHypothesis": has_hypothesis,
            })
            previous_pass = current_pass
            previous_workflow = row["workflowScore"]
        summaries[lane] = {
            "firstRankedMechanicalCeilingGeneration": first_ceiling_generation,
            "noOpRetryCount": no_op_count,
            "plateauDetected": plateau,
            "trajectory": trajectory,
        }
    return {
        "byLane": summaries,
        "missingRepairHypotheses": missing_hypotheses,
        "plateauDetected": any(summary["plateauDetected"] for summary in summaries.values()),
    }


def candidate_delta(by_lane: Mapping[str, int | None]) -> dict[str, int | None]:
    candidate = by_lane.get("W")

    def delta(base_lane: str) -> int | None:
        base = by_lane.get(base_lane)
        if candidate is None or base is None:
            return None
        return candidate - base

    return {
        "candidateMinusUnstructured": delta("U"),
        "candidateMinusMinimalControl": delta("M"),
    }


def token_report(records: list[dict[str, Any]]) -> dict[str, Any]:
    full_run_totals: dict[str, int] = {lane: 0 for lane in REQUIRED_LANE_IDS}
    tokens_until_ceiling: dict[str, int | None] = {lane: None for lane in REQUIRED_LANE_IDS}
    post_ceiling_waste: dict[str, int] = {lane: 0 for lane in REQUIRED_LANE_IDS}
    unavailable: set[str] = set()
    cost_unavailable: set[str] = set()
    by_lane: dict[str, list[dict[str, Any]]] = {lane: [] for lane in REQUIRED_LANE_IDS}
    for record in records:
        by_lane[record["laneId"]].append(record)
        telemetry = record["runRecord"].get("runtimeTelemetry", {})
        tokens = telemetry.get("tokens", {})
        unavailable.update(tokens.get("unavailableFields", []))
        reason = telemetry.get("cost", {}).get("unavailableReason")
        if reason:
            cost_unavailable.add(reason)

    for lane, rows in sorted(by_lane.items()):
        running_until_ceiling = 0
        for record in sorted(rows, key=lambda row: row["generation"]):
            total = record["tokens"]
            if not isinstance(total, int) or isinstance(total, bool):
                continue
            full_run_totals[lane] += total
            if tokens_until_ceiling[lane] is None:
                running_until_ceiling += total
            else:
                post_ceiling_waste[lane] += total

            mechanical = record["result"]["mechanical"]
            mechanical_ceiling = mechanical["ranked"] is True and mechanical["passCount"] == mechanical["totalAcs"]
            if mechanical_ceiling and tokens_until_ceiling[lane] is None:
                tokens_until_ceiling[lane] = running_until_ceiling

    full_run_delta = candidate_delta(full_run_totals)
    first_ceiling_delta = candidate_delta(tokens_until_ceiling)
    candidate = full_run_totals["W"]
    return {
        "trackBoundary": "Token/cost telemetry is separate from mechanical correctness and is not a mechanical score component.",
        "tokensByLane": {lane: full_run_totals[lane] for lane in sorted(full_run_totals)},
        "fullRunTokensByLane": {lane: full_run_totals[lane] for lane in sorted(full_run_totals)},
        "candidateMinusUnstructured": full_run_delta["candidateMinusUnstructured"],
        "candidateMinusMinimalControl": full_run_delta["candidateMinusMinimalControl"],
        "fullRunCandidateDelta": full_run_delta,
        "tokensUntilFirstRankedMechanicalCeilingByLane": {lane: tokens_until_ceiling[lane] for lane in sorted(tokens_until_ceiling)},
        "tokensUntilFirstRankedMechanicalCeilingCandidateDelta": first_ceiling_delta,
        "postCeilingWasteTokensByLane": {lane: post_ceiling_waste[lane] for lane in sorted(post_ceiling_waste)},
        "candidateTokenCostWorseThanMinimalControl": candidate > full_run_totals["M"],
        "unavailableTokenFields": sorted(unavailable),
        "unavailableCostReasons": sorted(cost_unavailable),
    }


def feedback_parity_report(records: list[dict[str, Any]]) -> dict[str, Any]:
    groups: dict[tuple[int, int], dict[str, list[str]]] = {}
    for record in records:
        key = (record["taskSeed"], record["generation"])
        groups.setdefault(key, {})[record["laneId"]] = feedback_ids(record["runRecord"])
    failures: list[dict[str, Any]] = []
    checked = 0
    for (seed, generation), lanes in sorted(groups.items()):
        if set(lanes) != REQUIRED_LANE_IDS:
            failures.append({"taskSeed": seed, "generation": generation, "reason": "missing lane for parity check", "lanes": sorted(lanes)})
            continue
        checked += 1
        baseline = lanes["U"]
        for lane in ["M", "W"]:
            if lanes[lane] != baseline:
                failures.append({"taskSeed": seed, "generation": generation, "laneId": lane, "expected": baseline, "actual": lanes[lane]})
    return {
        "pass": not failures,
        "checkedGenerationGroups": checked,
        "failures": failures,
    }


def evidence_report(selected: dict[str, dict[str, Any]]) -> dict[str, Any]:
    by_lane = {}
    for lane, record in sorted(selected.items()):
        evidence = record["result"]["evidence"]
        by_lane[lane] = {
            "publicSafe": evidence["publicSafe"],
            "privateRefsBlocked": evidence["privateRefsBlocked"],
            "replayable": evidence["replayable"],
            "requiredRefsMissing": evidence["requiredRefsMissing"],
            "compactSummaryRef": evidence["compactSummaryRef"],
        }
    return {
        "scoreBasis": "Compact, replayable, decision-linked evidence only; raw volume and file count give no credit.",
        "byLane": by_lane,
    }


def minimal_control_report(mechanical: dict[str, Any], workflow: dict[str, Any], token: dict[str, Any]) -> dict[str, Any]:
    w = mechanical["byLane"]["W"]
    m = mechanical["byLane"]["M"]
    mechanical_tie = (w["ranked"], w["passCount"], w["totalAcs"]) == (m["ranked"], m["passCount"], m["totalAcs"])
    return {
        "mechanicalTieWithMinimalControl": mechanical_tie,
        "workflowDeltaVsMinimalControl": workflow["candidateMinusMinimalControl"],
        "tokenDeltaVsMinimalControl": token["candidateMinusMinimalControl"],
        "structuredCandidateBeatsMinimalControl": (
            workflow["candidateMinusMinimalControl"] > 0 and token["candidateMinusMinimalControl"] <= 0 and not token["candidateTokenCostWorseThanMinimalControl"]
        ),
    }


def final_verdict(mechanical: dict[str, Any], visual: dict[str, Any], workflow: dict[str, Any], token: dict[str, Any], trajectory: dict[str, Any], parity: dict[str, Any]) -> tuple[str, list[str]]:
    reasons: list[str] = []
    if mechanical["status"] in {"tied", "gated"}:
        reasons.append(f"Final mechanical outcome is {mechanical['status']} across selected lane records.")
    else:
        reasons.append("Final mechanical outcome is unequal and does not establish a clean structured-candidate win.")
    if visual["blocked"]:
        reasons.append("Visual/product ranking is blocked because native capture validity is not established.")
    if workflow["candidateMinusMinimalControl"] <= 0:
        reasons.append("Structured candidate does not beat the minimal checklist control on the workflow aggregate.")
    if token["candidateTokenCostWorseThanMinimalControl"]:
        reasons.append("Structured candidate uses more source-labeled total tokens than the minimal checklist control.")
    if trajectory["missingRepairHypotheses"]:
        reasons.append("At least one retry/continue generation is missing a repair hypothesis.")
    if trajectory["plateauDetected"]:
        reasons.append("At least one lane shows plateau/no-op retry behavior.")
    if parity["pass"] is not True:
        reasons.append("Feedback parity is missing or failed.")
    status = "no-support-calibration-smoke"
    return status, reasons


def build_report(fixture_path: Path) -> dict[str, Any]:
    schema_validator = load_module(SCHEMA_VALIDATOR, "validate_v3_schemas")
    gauntlet_validator = load_module(GAUNTLET_VALIDATOR, "validate_v3_workflow_gauntlet")
    fixture = load_json(fixture_path)
    validate_fixture_shape(fixture, schema_validator)
    gauntlet_path = resolve_ref(fixture["gauntletRef"], "gauntletRef")
    gauntlet = validate_protocol(gauntlet_path, gauntlet_validator, schema_validator)
    validate_scenario_pack(fixture, gauntlet)
    records = load_records(fixture, schema_validator)
    selected = selected_by_lane(records)
    mechanical = mechanical_report(selected)
    visual = visual_report(selected, fixture)
    workflow = workflow_report(selected)
    trajectory = trajectory_report(records)
    token = token_report(records)
    parity = feedback_parity_report(records)
    evidence = evidence_report(selected)
    minimal = minimal_control_report(mechanical, workflow, token)
    verdict, rationale = final_verdict(mechanical, visual, workflow, token, trajectory, parity)
    report_fields = {
        "final-mechanical-outcome": mechanical,
        "visual-blocker-status": visual,
        "workflow-track-deltas": workflow,
        "trajectory-efficiency": trajectory,
        "token-cost-deltas": token,
        "feedback-parity": parity,
        "minimal-control-comparison": minimal,
        "claim-boundary": {
            "fixtureClaimBoundary": fixture["claimBoundary"],
            "finalVerdict": verdict,
            "reportIsCalibrationOnly": True,
        },
        "unavailable-telemetry-fields": {
            "tokenFields": token["unavailableTokenFields"],
            "costReasons": token["unavailableCostReasons"],
        },
    }
    require(set(report_fields) == REQUIRED_REPORT_FIELDS, "final report fields mismatch")
    return {
        "schemaVersion": "2000m.v3.workflow-gauntlet-smoke-report.v1",
        "fixtureRef": rel(fixture_path),
        "gauntletRef": rel(gauntlet_path),
        "claimBoundary": "calibration-only",
        "finalVerdict": verdict,
        "reportFields": report_fields,
        "evidence-track-summary": evidence,
        "recordCount": len(records),
        "scenarioFamilyCoverage": sorted(item["stressor"] for item in fixture["scenarioPack"]),
        "supportCandidateWouldNeed": fixture["supportCandidateWouldNeed"],
        "verdictRationale": rationale,
    }


def render_markdown(report: dict[str, Any]) -> str:
    fields = report["reportFields"]
    mechanical = fields["final-mechanical-outcome"]
    visual = fields["visual-blocker-status"]
    workflow = fields["workflow-track-deltas"]
    token = fields["token-cost-deltas"]
    trajectory = fields["trajectory-efficiency"]
    parity = fields["feedback-parity"]
    minimal = fields["minimal-control-comparison"]
    lines = [
        "# v3 workflow gauntlet smoke summary",
        "",
        f"Verdict: {report['finalVerdict']}",
        f"Claim boundary: {report['claimBoundary']}",
        f"Fixture: {report['fixtureRef']}",
        "",
        "## Required final report fields",
        "",
        f"- final mechanical outcome: {mechanical['status']} ({mechanical['byLane']})",
        f"- visual blocker status: blocked={visual['blocked']} claimBlocked={visual['claimBlocked']}",
        f"- workflow deltas: W-U={workflow['candidateMinusUnstructured']} W-M={workflow['candidateMinusMinimalControl']}",
        f"- trajectory efficiency: plateauDetected={trajectory['plateauDetected']} missingRepairHypotheses={len(trajectory['missingRepairHypotheses'])}",
        f"- token-cost full-run deltas: W-U={token['candidateMinusUnstructured']} W-M={token['candidateMinusMinimalControl']}",
        f"- token efficiency: tokensUntilFirstRankedMechanicalCeilingByLane={token['tokensUntilFirstRankedMechanicalCeilingByLane']} candidateDelta={token['tokensUntilFirstRankedMechanicalCeilingCandidateDelta']}",
        f"- post-ceiling waste tokens: {token['postCeilingWasteTokensByLane']}",
        f"- feedback parity: pass={parity['pass']} checkedGroups={parity['checkedGenerationGroups']}",
        f"- minimal-control comparison: {minimal}",
        f"- unavailable telemetry fields: {fields['unavailable-telemetry-fields']}",
        "",
        "## Verdict rationale",
        "",
        *[f"- {reason}" for reason in report["verdictRationale"]],
        "",
        "## Future support-candidate shape",
        "",
        *[f"- {item}" for item in report["supportCandidateWouldNeed"]],
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Run the v3 workflow-gauntlet calibration smoke")
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--json-out", type=Path, help="write the deterministic smoke report JSON")
    parser.add_argument("--markdown-out", type=Path, help="write the deterministic smoke report markdown")
    args = parser.parse_args()
    fixture_path = args.fixture if args.fixture.is_absolute() else ROOT / args.fixture
    try:
        report = build_report(fixture_path)
    except GauntletSmokeError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    if args.json_out:
        out = args.json_out if args.json_out.is_absolute() else ROOT / args.json_out
        write_json(out, report)
    markdown = render_markdown(report)
    if args.markdown_out:
        out = args.markdown_out if args.markdown_out.is_absolute() else ROOT / args.markdown_out
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(markdown)
    if not args.json_out and not args.markdown_out:
        print(markdown)
    elif args.json_out:
        print(f"OK: v3 workflow gauntlet smoke -> {rel(args.json_out if args.json_out.is_absolute() else ROOT / args.json_out)}")
    else:
        print(f"OK: v3 workflow gauntlet smoke -> {rel(args.markdown_out if args.markdown_out.is_absolute() else ROOT / args.markdown_out)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
