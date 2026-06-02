#!/usr/bin/env python3
"""Check workspace-local scorer feedback parity for private v2 A/B runs.

The checker is intentionally runner-layout aware but scorer-neutral: it does not
change scoring semantics and it does not interpret a lane outcome. It only proves
that every enabled lane received the same class of detailed scorer evidence before
future generations could use it.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

REQUIRED_RECORD_FILES = ("v1-conformance.json", "score.log", "scorer-feedback.md")
REQUIRED_WORKSPACE_FILES = REQUIRED_RECORD_FILES + ("feedback-manifest.json",)


class FeedbackParityError(ValueError):
    pass


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        raise FeedbackParityError(f"missing JSON file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise FeedbackParityError(f"invalid JSON in {path}: {exc}") from exc


def lane_key(value: str) -> str:
    value = value.strip()
    if not value:
        raise argparse.ArgumentTypeError("lane id cannot be empty")
    return value.lower()


def discover_seeds(run_root: Path) -> list[int]:
    records = run_root / "records"
    seeds: list[int] = []
    for path in sorted(records.glob("pilot-seed-*")):
        match = re.fullmatch(r"pilot-seed-(\d+)", path.name)
        if match:
            seeds.append(int(match.group(1)))
    return seeds


def discover_lanes(run_root: Path, seeds: list[int]) -> list[str]:
    lanes: set[str] = set()
    for seed in seeds:
        pair = run_root / "records" / f"pilot-seed-{seed}"
        for path in sorted(pair.glob("lane-*")):
            match = re.fullmatch(r"lane-([A-Za-z0-9_-]+)", path.name)
            if match:
                lanes.add(match.group(1).lower())
    return sorted(lanes)


def scorer_diagnostic_lines(conformance_path: Path) -> list[str]:
    data = load_json(conformance_path)
    lines: list[str] = []
    det = data.get("determinism")
    if isinstance(det, dict) and det.get("detail"):
        lines.append(f"determinism: {det.get('detail')}")
    for ac in data.get("acs", []):
        if ac.get("pass"):
            continue
        detail = ac.get("detail") or ac.get("message") or "no detail supplied"
        lines.append(f"{ac.get('id', '?')} {ac.get('name', '')}: {detail}")
    seen: set[str] = set()
    unique: list[str] = []
    for line in lines:
        if line not in seen:
            seen.add(line)
            unique.append(line)
    return unique


def relative_or_str(path: Path, base: Path) -> str:
    try:
        return path.relative_to(base).as_posix()
    except ValueError:
        return str(path)


def same_file_content(left: Path, right: Path) -> bool:
    if not left.exists() or not right.exists():
        return False
    try:
        return left.read_bytes() == right.read_bytes()
    except OSError:
        return False


def check_generation(run_root: Path, seed: int, lane: str, generation: int, require_workspace_copy: bool) -> dict[str, Any]:
    lane_dir = run_root / "records" / f"pilot-seed-{seed}" / f"lane-{lane}" / f"generation-{generation:02d}"
    workspace_dir = run_root / "workspaces" / f"2000m-private-pilot-seed-{seed}-lane-{lane}" / "trajectory" / f"generation-{generation:02d}"

    checks: dict[str, bool] = {}
    missing: list[str] = []
    for filename in REQUIRED_RECORD_FILES:
        key = f"records/{filename}"
        exists = (lane_dir / filename).exists()
        checks[key] = exists
        if not exists:
            missing.append(relative_or_str(lane_dir / filename, run_root))

    if require_workspace_copy:
        for filename in REQUIRED_WORKSPACE_FILES:
            key = f"workspace/{filename}"
            exists = (workspace_dir / filename).exists()
            checks[key] = exists
            if not exists:
                missing.append(relative_or_str(workspace_dir / filename, run_root))
        for filename in REQUIRED_RECORD_FILES:
            record_path = lane_dir / filename
            workspace_path = workspace_dir / filename
            checks[f"workspace_matches_record/{filename}"] = same_file_content(record_path, workspace_path)

    feedback_path = workspace_dir / "scorer-feedback.md" if require_workspace_copy else lane_dir / "scorer-feedback.md"
    conformance_path = workspace_dir / "v1-conformance.json" if require_workspace_copy else lane_dir / "v1-conformance.json"
    feedback_text = feedback_path.read_text(errors="replace") if feedback_path.exists() else ""
    exact_section = "## Exact scorer diagnostics" in feedback_text
    diagnostic_lines: list[str] = []
    diagnostic_visibility = False
    if conformance_path.exists():
        diagnostic_lines = scorer_diagnostic_lines(conformance_path)
        diagnostic_visibility = all(line in feedback_text for line in diagnostic_lines) if diagnostic_lines else exact_section

    checks["feedback_has_exact_diagnostics_section"] = exact_section
    checks["feedback_contains_exact_diagnostics"] = diagnostic_visibility

    passed = all(checks.values())
    return {
        "pairId": f"pilot-seed-{seed}",
        "taskSeed": seed,
        "lane": lane.upper(),
        "generation": generation,
        "recordFeedbackDir": relative_or_str(lane_dir, run_root),
        "workspaceFeedbackDir": relative_or_str(workspace_dir, run_root),
        "checks": checks,
        "missing": missing,
        "diagnosticCount": len(diagnostic_lines),
        "pass": passed,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Check v2 private-pilot feedback parity before future generations use scorer feedback.")
    parser.add_argument("run_root", help="Private pilot run root containing records/ and workspaces/.")
    parser.add_argument("--seeds", nargs="*", type=int, help="Task seeds to check. Defaults to records/pilot-seed-* discovery.")
    parser.add_argument("--lanes", nargs="*", type=lane_key, help="Lane IDs to check. Defaults to lane-* discovery for discovered seeds.")
    parser.add_argument("--generations", nargs="*", type=int, help="Generation numbers to check. Defaults to 1..generation-cap.")
    parser.add_argument("--generation-cap", type=int, default=3, help="Generation cap used when --generations is omitted. Default: 3.")
    parser.add_argument("--records-only", action="store_true", help="Check generation records only. Default requires workspace-local feedback copies too.")
    parser.add_argument("--json-out", help="Write the full parity report JSON to this path.")
    args = parser.parse_args()

    run_root = Path(args.run_root).expanduser().resolve()
    if not run_root.exists():
        print(f"FAIL: run root does not exist: {run_root}", file=sys.stderr)
        return 1
    seeds = args.seeds or discover_seeds(run_root)
    if not seeds:
        print("FAIL: no seeds supplied or discovered under records/pilot-seed-*", file=sys.stderr)
        return 1
    lanes = args.lanes or discover_lanes(run_root, seeds)
    if not lanes:
        print("FAIL: no lanes supplied or discovered under records/pilot-seed-*/lane-*", file=sys.stderr)
        return 1
    generations = args.generations or list(range(1, args.generation_cap + 1))
    if any(gen < 1 for gen in generations):
        print("FAIL: generation numbers must be >= 1", file=sys.stderr)
        return 1

    entries: list[dict[str, Any]] = []
    for generation in generations:
        for seed in seeds:
            for lane in lanes:
                entries.append(check_generation(run_root, seed, lane, generation, require_workspace_copy=not args.records_only))

    report = {
        "schemaVersion": "2000m.v2.feedback-parity-report.v1",
        "runRoot": str(run_root),
        "seeds": seeds,
        "lanes": [lane.upper() for lane in lanes],
        "generations": generations,
        "requiredRecordFiles": list(REQUIRED_RECORD_FILES),
        "requiredWorkspaceFiles": [] if args.records_only else list(REQUIRED_WORKSPACE_FILES),
        "pass": all(entry["pass"] for entry in entries),
        "entries": entries,
    }

    if args.json_out:
        out = Path(args.json_out).expanduser()
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(report, indent=2) + "\n")

    if report["pass"]:
        print(f"OK: feedback parity passed for {len(entries)} lane-generation records")
        return 0

    print("FAIL: feedback parity failed", file=sys.stderr)
    for entry in entries:
        if not entry["pass"]:
            print(
                f"- {entry['pairId']} lane {entry['lane']} gen {entry['generation']}: "
                f"missing={entry['missing']} checks={entry['checks']}",
                file=sys.stderr,
            )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
