#!/usr/bin/env python3
"""Render a conservative v3 private campaign report from run records."""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

from collect_v3_token_telemetry import collect as collect_token_telemetry

GENERATION_RE = re.compile(r"records/pilot-seed-(\d+)/lane-([^/]+)/generation-(\d+)$")


def load_json(path: Path) -> Any:
    return json.loads(path.read_text())


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def rel(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return str(path)


def generation_dirs(run_root: Path) -> list[Path]:
    return sorted(path for path in (run_root / "records").glob("pilot-seed-*/lane-*/generation-*") if path.is_dir())


def generation_identity(path: Path, run_root: Path) -> tuple[int, str, int]:
    match = GENERATION_RE.fullmatch(rel(path, run_root))
    if not match:
        raise SystemExit(f"unexpected generation path: {path}")
    return int(match.group(1)), match.group(2).upper(), int(match.group(3))


def read_generation(path: Path, run_root: Path) -> dict[str, Any]:
    seed, lane, generation = generation_identity(path, run_root)
    result = load_json(path / "v3-result.json")
    run_record = load_json(path / "run-record.json")
    visual_status_path = path / "visual-status.json"
    visual_status = load_json(visual_status_path) if visual_status_path.exists() else result.get("visual", {})
    return {
        "taskSeed": seed,
        "laneId": lane,
        "generation": generation,
        "generationRef": rel(path, run_root),
        "resultRef": rel(path / "v3-result.json", run_root),
        "runRecordRef": rel(path / "run-record.json", run_root),
        "mechanical": result["mechanical"],
        "visual": visual_status,
        "workflow": run_record.get("workflow", {}),
    }


def selected_generations(entries: list[dict[str, Any]]) -> dict[str, dict[str, dict[str, Any]]]:
    grouped: dict[tuple[int, str], list[dict[str, Any]]] = {}
    for entry in entries:
        grouped.setdefault((entry["taskSeed"], entry["laneId"]), []).append(entry)
    selected: dict[str, dict[str, dict[str, Any]]] = {}
    for (seed, lane), rows in sorted(grouped.items()):
        def key(row: dict[str, Any]) -> tuple[int, float, int]:
            mechanical = row["mechanical"]
            return (
                int(mechanical.get("passCount") or 0),
                float(mechanical.get("compositeScore") or 0),
                int(row["generation"]),
            )
        best = max(rows, key=key)
        selected.setdefault(str(seed), {})[lane] = best
    return selected


def trajectory(entries: list[dict[str, Any]]) -> dict[str, dict[str, str]]:
    grouped: dict[tuple[int, str], list[dict[str, Any]]] = {}
    for entry in entries:
        grouped.setdefault((entry["taskSeed"], entry["laneId"]), []).append(entry)
    out: dict[str, dict[str, str]] = {}
    for (seed, lane), rows in sorted(grouped.items()):
        parts = []
        for row in sorted(rows, key=lambda item: item["generation"]):
            mechanical = row["mechanical"]
            parts.append(f"{mechanical.get('passCount')}/{mechanical.get('totalAcs')}")
        out.setdefault(str(seed), {})[lane] = " -> ".join(parts)
    return out


def selected_summary(selected: dict[str, dict[str, dict[str, Any]]]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for seed, lanes in selected.items():
        out[seed] = {}
        for lane, row in lanes.items():
            mechanical = row["mechanical"]
            visual = row["visual"]
            workflow = row["workflow"]
            out[seed][lane] = {
                "generation": row["generation"],
                "passCount": mechanical.get("passCount"),
                "totalAcs": mechanical.get("totalAcs"),
                "ranked": mechanical.get("ranked"),
                "resultRef": row["resultRef"],
                "visualRanked": visual.get("ranked"),
                "visualBlockReason": visual.get("blockReason"),
                "workflow": workflow,
            }
    return out


def feedback_parity(run_root: Path) -> dict[str, Any]:
    reports = sorted((run_root / "feedback-parity").glob("generation-*.json"))
    if not reports:
        return {"pass": None, "latestReportRef": None}
    latest = reports[-1]
    data = load_json(latest)
    return {"pass": data.get("pass"), "latestReportRef": rel(latest, run_root)}


def visual_blocked(selected: dict[str, dict[str, dict[str, Any]]]) -> bool:
    for lanes in selected.values():
        for row in lanes.values():
            if row["visual"].get("ranked") is not True:
                return True
    return False


def final_mechanical_tie(selected: dict[str, dict[str, dict[str, Any]]]) -> bool:
    for lanes in selected.values():
        if not {"A", "B"}.issubset(lanes):
            return False
        a = lanes["A"]["mechanical"]
        b = lanes["B"]["mechanical"]
        if a.get("passCount") != b.get("passCount") or a.get("totalAcs") != b.get("totalAcs"):
            return False
    return True


def verdict_rationale(selected: dict[str, dict[str, dict[str, Any]]], token_summary: dict[str, Any], parity: dict[str, Any]) -> list[str]:
    reasons: list[str] = []
    if final_mechanical_tie(selected):
        reasons.append("Final selected mechanical outcomes tied across Lane A and Lane B.")
    else:
        reasons.append("Final selected mechanical outcomes did not establish a predeclared Lane B advantage.")
    if visual_blocked(selected):
        reasons.append("At least one selected visual/product record is rank-blocked, so no visual/product claim is valid.")
    reasons.append("Workflow status is reported separately and is not independent output-quality proof.")
    if parity.get("pass") is True:
        reasons.append("Feedback parity passed for the latest guard report.")
    else:
        reasons.append("Feedback parity is missing or failed, which blocks support claims.")
    delta = token_summary["aggregates"]["laneDeltaFullRun"]
    if delta.get("laneBMinusLaneA") is not None:
        reasons.append(
            f"Lane B used {delta['laneBMinusLaneA']} more full-run tokens than Lane A ({delta['laneBPercentDeltaVsLaneA']}%)."
        )
    return reasons


def render(run_root: Path) -> dict[str, Any]:
    entries = [read_generation(path, run_root) for path in generation_dirs(run_root)]
    selected = selected_generations(entries)
    token_summary = collect_token_telemetry(run_root, write_records=False)
    parity = feedback_parity(run_root)
    return {
        "schemaVersion": "2000m.v3.private-final-report.v2",
        "runRoot": str(run_root),
        "claimBoundary": "private-evidence-only",
        "finalPrivateVerdict": "no support",
        "trajectorySummary": trajectory(entries),
        "selectedGenerations": selected_summary(selected),
        "visualProductTrack": {
            "blocked": visual_blocked(selected),
            "note": "Native capture validity is a prerequisite for visual/product ranking.",
        },
        "workflowTrack": {
            "note": "Workflow status is reported separately and is not a mechanical score component or output-quality proof.",
        },
        "feedbackParity": parity,
        "tokenTelemetry": {
            "trackBoundary": token_summary["trackBoundary"],
            "aggregates": token_summary["aggregates"],
        },
        "verdictRationale": verdict_rationale(selected, token_summary, parity),
        "ownerGates": [
            "commit",
            "push",
            "pull-request",
            "merge",
            "release",
            "publish",
            "new public claim",
        ],
    }


def render_markdown(report: dict[str, Any]) -> str:
    token_full = report["tokenTelemetry"]["aggregates"]["laneDeltaFullRun"]
    token_first = report["tokenTelemetry"]["aggregates"]["tokensUntilFirstRankedMechanicalCeiling"]["laneDelta"]
    lines = [
        "# 2000m v3 private report",
        "",
        f"Verdict: {report['finalPrivateVerdict']}",
        f"Claim boundary: {report['claimBoundary']}",
        "",
        "## Mechanical trajectory",
        "",
    ]
    for seed, lanes in report["trajectorySummary"].items():
        for lane, text in lanes.items():
            selected = report["selectedGenerations"][seed][lane]
            lines.append(
                f"- seed {seed} lane {lane}: {text}; selected gen {selected['generation']} = {selected['passCount']}/{selected['totalAcs']} ranked={selected['ranked']}"
            )
    lines.extend([
        "",
        "## Visual/product",
        "",
        f"- blocked: {report['visualProductTrack']['blocked']}",
        f"- note: {report['visualProductTrack']['note']}",
        "",
        "## Token/cost telemetry",
        "",
        f"- full-run Lane A tokens: {token_full['laneA']}",
        f"- full-run Lane B tokens: {token_full['laneB']}",
        f"- Lane B minus Lane A: {token_full['laneBMinusLaneA']} ({token_full['laneBPercentDeltaVsLaneA']}%)",
        f"- until first mechanical ceiling Lane A tokens: {token_first['laneA']}",
        f"- until first mechanical ceiling Lane B tokens: {token_first['laneB']}",
        f"- until-first-ceiling Lane B minus Lane A: {token_first['laneBMinusLaneA']} ({token_first['laneBPercentDeltaVsLaneA']}%)",
        "",
        "## Feedback parity",
        "",
        f"- pass: {report['feedbackParity']['pass']}",
        f"- latest report: {report['feedbackParity']['latestReportRef']}",
        "",
        "## Verdict rationale",
        "",
        *[f"- {reason}" for reason in report["verdictRationale"]],
        "",
    ])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Render v3 private campaign report")
    parser.add_argument("run_root", type=Path)
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--markdown-out", type=Path)
    args = parser.parse_args()

    run_root = args.run_root.resolve()
    report = render(run_root)
    if args.json_out:
        write_json(args.json_out if args.json_out.is_absolute() else Path.cwd() / args.json_out, report)
    if args.markdown_out:
        out = args.markdown_out if args.markdown_out.is_absolute() else Path.cwd() / args.markdown_out
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(render_markdown(report))
    if not args.json_out and not args.markdown_out:
        print(render_markdown(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
