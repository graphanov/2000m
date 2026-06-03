#!/usr/bin/env python3
"""Collect v3 private-campaign token telemetry from Codex stderr logs.

This is runner-layout aware and scorer-neutral. It records runtime/cost
telemetry separately from mechanical, visual, workflow, and evidence tracks.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

TOKEN_MARKER_RE = re.compile(r"^\s*tokens used\s*$", re.IGNORECASE)
TOKEN_VALUE_RE = re.compile(r"^\s*([0-9][0-9,]*)\s*$")
TOKEN_INLINE_RE = re.compile(r"\b([0-9][0-9,]*)\s+tokens used\b", re.IGNORECASE)
GENERATION_RE = re.compile(r"records/pilot-seed-(\d+)/lane-([^/]+)/generation-(\d+)$")


def load_json(path: Path) -> Any:
    return json.loads(path.read_text())


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def rel(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return str(path)


def parse_codex_tokens_used(stderr_path: Path) -> int | None:
    if not stderr_path.exists():
        return None
    lines = stderr_path.read_text(errors="replace").splitlines()
    values: list[int] = []
    for index, line in enumerate(lines):
        for match in TOKEN_INLINE_RE.finditer(line):
            values.append(int(match.group(1).replace(",", "")))
        if TOKEN_MARKER_RE.match(line):
            for next_line in lines[index + 1:index + 5]:
                value_match = TOKEN_VALUE_RE.match(next_line)
                if value_match:
                    values.append(int(value_match.group(1).replace(",", "")))
                    break
    return values[-1] if values else None


def generation_dirs(run_root: Path) -> list[Path]:
    records = run_root / "records"
    return sorted(path for path in records.glob("pilot-seed-*/lane-*/generation-*") if path.is_dir())


def generation_identity(path: Path, run_root: Path) -> tuple[int, str, int]:
    relative = rel(path, run_root)
    match = GENERATION_RE.fullmatch(relative)
    require(match is not None, f"unexpected generation path: {relative}")
    assert match is not None
    return int(match.group(1)), match.group(2).upper(), int(match.group(3))


def mechanical_status(path: Path, run_root: Path) -> dict[str, Any]:
    for filename in ("run-record.json", "v3-result.json"):
        candidate = path / filename
        if candidate.exists():
            data = load_json(candidate)
            mechanical = data.get("mechanical", {}) if isinstance(data, dict) else {}
            if isinstance(mechanical, dict):
                return {
                    "ranked": mechanical.get("ranked"),
                    "passCount": mechanical.get("passCount"),
                    "totalAcs": mechanical.get("totalAcs"),
                    "failedAcs": mechanical.get("failedAcs", []),
                    "sourceRef": rel(candidate, run_root),
                }
    return {"ranked": None, "passCount": None, "totalAcs": None, "failedAcs": [], "sourceRef": ""}


def runtime_telemetry(stderr_ref: str, total_tokens: int | None) -> dict[str, Any]:
    unavailable = ["input", "output", "cachedInput", "reasoning"]
    if total_tokens is None:
        unavailable = ["total", *unavailable]
    return {
        "source": {
            "kind": "codex-stderr",
            "ref": stderr_ref,
            "parser": "codex-cli-tokens-used-line-v1",
        },
        "tokens": {
            "total": total_tokens,
            "input": None,
            "output": None,
            "cachedInput": None,
            "reasoning": None,
            "unavailableFields": unavailable,
        },
        "cost": {
            "estimatedUsd": None,
            "currency": None,
            "unavailableReason": "Codex stderr exposes a reliable total token count only; no versioned pricing table or token split was applied.",
        },
    }


def update_run_record(generation_dir: Path, telemetry: dict[str, Any]) -> None:
    path = generation_dir / "run-record.json"
    if not path.exists():
        return
    data = load_json(path)
    require(isinstance(data, dict), f"run-record must be a JSON object: {path}")
    data["runtimeTelemetry"] = telemetry
    write_json(path, data)


def add_nested_total(target: dict[str, Any], seed: int, lane: str, tokens: int | None) -> None:
    seed_key = str(seed)
    target.setdefault(seed_key, {})
    current = target[seed_key].get(lane)
    if tokens is None:
        target[seed_key][lane] = current
    else:
        target[seed_key][lane] = (current or 0) + tokens


def percent_delta(delta: int | None, baseline: int | None) -> float | None:
    if delta is None or not baseline:
        return None
    return round((delta / baseline) * 100, 6)


def delta_summary(by_lane: dict[str, int]) -> dict[str, Any]:
    a = by_lane.get("A")
    b = by_lane.get("B")
    delta = b - a if a is not None and b is not None else None
    return {
        "laneA": a,
        "laneB": b,
        "laneBMinusLaneA": delta,
        "laneBPercentDeltaVsLaneA": percent_delta(delta, a),
    }


def aggregate(entries: list[dict[str, Any]]) -> dict[str, Any]:
    by_seed_lane: dict[str, Any] = {}
    by_lane: dict[str, int] = {}
    missing_token_entries = 0
    for entry in entries:
        tokens = entry["tokens"]["total"]
        if tokens is None:
            missing_token_entries += 1
            continue
        add_nested_total(by_seed_lane, entry["taskSeed"], entry["laneId"], tokens)
        by_lane[entry["laneId"]] = by_lane.get(entry["laneId"], 0) + tokens

    first_ceiling_by_seed_lane: dict[str, Any] = {}
    by_pair: dict[tuple[int, str], list[dict[str, Any]]] = {}
    for entry in entries:
        by_pair.setdefault((entry["taskSeed"], entry["laneId"]), []).append(entry)
    for (seed, lane), rows in sorted(by_pair.items()):
        running = 0
        found: dict[str, Any] | None = None
        for row in sorted(rows, key=lambda item: item["generation"]):
            tokens = row["tokens"]["total"]
            if tokens is not None:
                running += tokens
            mechanical = row["mechanical"]
            if mechanical["ranked"] is True and mechanical["passCount"] == mechanical["totalAcs"] and mechanical["totalAcs"]:
                found = {
                    "generation": row["generation"],
                    "tokens": running,
                    "passCount": mechanical["passCount"],
                    "totalAcs": mechanical["totalAcs"],
                }
                break
        first_ceiling_by_seed_lane.setdefault(str(seed), {})[lane] = found

    first_ceiling_by_lane: dict[str, int] = {}
    for lanes in first_ceiling_by_seed_lane.values():
        for lane, value in lanes.items():
            if isinstance(value, dict) and isinstance(value.get("tokens"), int):
                first_ceiling_by_lane[lane] = first_ceiling_by_lane.get(lane, 0) + value["tokens"]

    improvement_rows: list[dict[str, Any]] = []
    for (seed, lane), rows in sorted(by_pair.items()):
        previous_pass: int | None = None
        for row in sorted(rows, key=lambda item: item["generation"]):
            current_pass = row["mechanical"]["passCount"]
            tokens = row["tokens"]["total"]
            if previous_pass is None or not isinstance(current_pass, int):
                improvement_rows.append({
                    "taskSeed": seed,
                    "laneId": lane,
                    "generation": row["generation"],
                    "deltaPassCount": None,
                    "tokens": tokens,
                    "tokensPerPassedAcImprovement": None,
                    "meaningful": False,
                    "reason": "no previous generation baseline",
                })
            else:
                delta = current_pass - previous_pass
                improvement_rows.append({
                    "taskSeed": seed,
                    "laneId": lane,
                    "generation": row["generation"],
                    "deltaPassCount": delta,
                    "tokens": tokens,
                    "tokensPerPassedAcImprovement": round(tokens / delta, 6) if isinstance(tokens, int) and delta > 0 else None,
                    "meaningful": delta > 0 and isinstance(tokens, int),
                    "reason": "pass-count improved" if delta > 0 else "no pass-count improvement",
                })
            previous_pass = current_pass if isinstance(current_pass, int) else previous_pass

    return {
        "tokensBySeedLane": by_seed_lane,
        "tokensByLane": by_lane,
        "laneDeltaFullRun": delta_summary(by_lane),
        "tokensUntilFirstRankedMechanicalCeiling": {
            "bySeedLane": first_ceiling_by_seed_lane,
            "byLane": first_ceiling_by_lane,
            "laneDelta": delta_summary(first_ceiling_by_lane),
        },
        "tokensPerPassedAcImprovement": improvement_rows,
        "missingTokenEntryCount": missing_token_entries,
    }


def collect(run_root: Path, *, write_records: bool) -> dict[str, Any]:
    entries: list[dict[str, Any]] = []
    for gen_dir in generation_dirs(run_root):
        seed, lane, generation = generation_identity(gen_dir, run_root)
        stderr_path = gen_dir / "codex-stderr.log"
        stderr_ref = rel(stderr_path, run_root)
        tokens = parse_codex_tokens_used(stderr_path)
        telemetry = runtime_telemetry(stderr_ref, tokens)
        mechanical = mechanical_status(gen_dir, run_root)
        token_record = {
            "schemaVersion": "2000m.v3.runtime-telemetry.v1",
            "taskSeed": seed,
            "laneId": lane,
            "generation": generation,
            "runtimeTelemetry": telemetry,
        }
        if write_records:
            write_json(gen_dir / "token-telemetry.json", token_record)
            update_run_record(gen_dir, telemetry)
        entries.append({
            "taskSeed": seed,
            "laneId": lane,
            "generation": generation,
            "generationRef": rel(gen_dir, run_root),
            "stderrRef": stderr_ref,
            "tokens": telemetry["tokens"],
            "cost": telemetry["cost"],
            "mechanical": mechanical,
        })
    return {
        "schemaVersion": "2000m.v3.token-telemetry-summary.v1",
        "runRoot": str(run_root),
        "parser": "codex-cli-tokens-used-line-v1",
        "trackBoundary": "runtime token/cost telemetry is separate from mechanical correctness and is not a mechanical score component",
        "entries": entries,
        "aggregates": aggregate(entries),
    }


def render_markdown(summary: dict[str, Any]) -> str:
    full = summary["aggregates"]["laneDeltaFullRun"]
    first = summary["aggregates"]["tokensUntilFirstRankedMechanicalCeiling"]["laneDelta"]
    lines = [
        "# v3 token telemetry summary",
        "",
        "Token/cost telemetry is separate from mechanical correctness. It is not a score component.",
        "",
        "## Lane totals",
        "",
        f"- Full run Lane A: {full['laneA']}",
        f"- Full run Lane B: {full['laneB']}",
        f"- Lane B minus Lane A: {full['laneBMinusLaneA']} ({full['laneBPercentDeltaVsLaneA']}%)",
        "",
        "## Until first ranked mechanical ceiling",
        "",
        f"- Lane A: {first['laneA']}",
        f"- Lane B: {first['laneB']}",
        f"- Lane B minus Lane A: {first['laneBMinusLaneA']} ({first['laneBPercentDeltaVsLaneA']}%)",
        "",
        "## Unavailable fields",
        "",
        "- Codex stderr exposed total tokens only.",
        "- Input/output/cached/reasoning token splits are null.",
        "- Estimated cost is null because no versioned pricing table was applied.",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Collect v3 private campaign token telemetry")
    parser.add_argument("run_root", type=Path)
    parser.add_argument("--write-records", action="store_true", help="write token-telemetry.json files and update run-record.json runtimeTelemetry fields")
    parser.add_argument("--json-out", type=Path, help="write aggregate telemetry JSON")
    parser.add_argument("--markdown-out", type=Path, help="write aggregate telemetry markdown")
    args = parser.parse_args()

    run_root = args.run_root.resolve()
    require(run_root.exists() and run_root.is_dir(), f"run root does not exist: {run_root}")
    summary = collect(run_root, write_records=args.write_records)
    if args.json_out:
        write_json(args.json_out if args.json_out.is_absolute() else Path.cwd() / args.json_out, summary)
    if args.markdown_out:
        out = args.markdown_out if args.markdown_out.is_absolute() else Path.cwd() / args.markdown_out
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(render_markdown(summary))
    if not args.json_out and not args.markdown_out:
        json.dump(summary, sys.stdout, indent=2, sort_keys=True)
        sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
