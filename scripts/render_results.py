#!/usr/bin/env python3
"""Render and validate the 2000m results spine.

The script intentionally has no third-party dependencies. It keeps v0/v1-style
mechanical rows separate from v2 workflow-resilience result rows so v2 does not
turn into a raw model-intelligence leaderboard by accident.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
RESULTS_JSON = ROOT / "results" / "results.json"
LEADERBOARD = ROOT / "results" / "leaderboard.md"

PRIVATE_MARKERS = (
    "/Users/",
    "\\Users\\",
    "file://",
    "~/",
    "../",
    "..\\",
)


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


def fmt_bool(value: bool) -> str:
    return "yes" if value else "no"


def fmt_optional(value: Any) -> str:
    return "—" if value is None else str(value)


def fmt_score(value: Any) -> str:
    if isinstance(value, (int, float)):
        return f"{float(value):.2f}".rstrip("0").rstrip(".")
    return str(value)


def link(path: str) -> str:
    return f"[`{path}`](../{path})"


def check_public_ref(value: str, field: str) -> None:
    lowered = value.lower().strip()
    require(not lowered.startswith("/"), f"{field} contains absolute local path: {value}")
    require(not lowered.startswith("~"), f"{field} contains tilde path: {value}")
    require(not (len(lowered) >= 2 and lowered[0].isalpha() and lowered[1] == ":"), f"{field} contains Windows local path: {value}")
    for marker in PRIVATE_MARKERS:
        require(marker.lower() not in lowered, f"{field} contains private/local marker `{marker}`: {value}")


def validate_v0_row(row: dict[str, Any]) -> None:
    required = [
        "rank",
        "model",
        "producedRepo",
        "generationCap",
        "trajectory",
        "finalPassCount",
        "totalAcs",
        "generationsToPlayable",
        "generationsToYeti",
        "monotonic",
        "humanFeelOperatorTasteNotScore",
        "evidence",
    ]
    for key in required:
        require(key in row, f"v0 row missing `{key}`")
    check_public_ref(row["producedRepo"], "v0 producedRepo")
    check_public_ref(row["evidence"], "v0 evidence")


def validate_v2_row(row: dict[str, Any]) -> dict[str, Any]:
    required = ["track", "scenario", "runRecord", "resultJson", "claimBoundary"]
    for key in required:
        require(key in row, f"v2 row missing `{key}`")
        require(isinstance(row[key], str) and row[key].strip(), f"v2 row `{key}` must be a non-empty string")
        check_public_ref(row[key], f"v2 {key}")
    require(row["track"] == "v2-workflow-resilience", f"unsupported v2 track `{row['track']}`")

    scenario_path = ROOT / row["scenario"]
    run_record_path = ROOT / row["runRecord"]
    result_path = ROOT / row["resultJson"]
    require(scenario_path.exists(), f"v2 scenario missing: {row['scenario']}")
    require(run_record_path.exists(), f"v2 run record missing: {row['runRecord']}")

    scenario = load_json(scenario_path)
    run_record = load_json(run_record_path)
    result = load_json(result_path)
    require(scenario.get("schemaVersion") == "2000m.v2.scenario.v1", f"{row['scenario']} has wrong schemaVersion")
    require(run_record.get("schemaVersion") == "2000m.v2.run-record.v1", f"{row['runRecord']} has wrong schemaVersion")
    require(result.get("schemaVersion") == "2000m.v2.result.v1", f"{row['resultJson']} has wrong schemaVersion")
    require(result.get("scenarioId") == scenario.get("scenarioId"), f"{row['resultJson']} scenarioId does not match {row['scenario']}")
    require(result.get("scenarioVersion") == scenario.get("scenarioVersion"), f"{row['resultJson']} scenarioVersion does not match {row['scenario']}")
    require(result.get("scenarioId") == run_record.get("scenarioId"), f"{row['resultJson']} scenarioId does not match {row['runRecord']}")
    require(result.get("scenarioVersion") == run_record.get("scenarioVersion"), f"{row['resultJson']} scenarioVersion does not match {row['runRecord']}")
    require(result.get("entrant") == run_record.get("entrant", {}).get("label"), f"{row['resultJson']} entrant does not match {row['runRecord']}")
    require(result.get("processType") == run_record.get("entrant", {}).get("processType"), f"{row['resultJson']} processType does not match {row['runRecord']}")
    require(isinstance(result.get("ranked"), bool), f"{row['resultJson']} missing ranked bool")
    require(isinstance(result.get("compositeScore"), (int, float)), f"{row['resultJson']} missing compositeScore")
    for component in ["artifactQuality", "feedbackIntegration", "recoveryHandoff", "stopCondition", "evidenceReplay"]:
        value = result.get("components", {}).get(component, {})
        require(isinstance(value.get("score"), (int, float)), f"{row['resultJson']} missing {component}.score")
    return result


def render(data: dict[str, Any]) -> str:
    require(data.get("schemaVersion") == "2000m.results.v1", "results.json schemaVersion must be 2000m.results.v1")
    v0_rows = data.get("rows", [])
    v2_rows = data.get("v2Rows", [])
    require(isinstance(v0_rows, list), "rows must be a list")
    require(isinstance(v2_rows, list), "v2Rows must be a list")

    lines: list[str] = [
        "# 2000m Leaderboard",
        "",
        "This is the rendered scoreboard for the 2000m judge. Mechanical columns determine",
        "rank from the track-labeled AC-pass trajectory; the human-feel column is",
        "explicitly **operator taste, not score**, and is never blended into the",
        "mechanical rank.",
        "",
        "## v0 mechanical leaderboard",
        "",
        "| Mechanical rank | Model | Produced repo | AC trajectory | Final ACs | Gen-to-playable (AC1–AC8) | Gen-to-yeti (AC12–AC14) | Monotonic | Human feel (operator taste, not score) | Evidence |",
        "|---:|---|---|---:|---:|---:|---:|:---:|---|---|",
    ]

    for row in v0_rows:
        validate_v0_row(row)
        trajectory = "→".join(str(x) for x in row["trajectory"])
        lines.append(
            f"| {row['rank']} | {row['model']} | `{row['producedRepo']}` | {trajectory} | "
            f"{row['finalPassCount']}/{row['totalAcs']} | {fmt_optional(row['generationsToPlayable'])} | "
            f"{fmt_optional(row['generationsToYeti'])} | {fmt_bool(row['monotonic'])} | "
            f"{row['humanFeelOperatorTasteNotScore']} | `{row['evidence']}` |"
        )

    lines.extend([
        "",
        "## v2 workflow-resilience result spine",
        "",
        "These rows exercise the v2 result schema and scorer behavior. Calibration",
        "fixtures are not contender results, not model rankings, and not evidence that",
        "any workflow framework is superior.",
        "",
        "| Scenario | Entrant | Process | Ranked? | Composite | Artifact | Feedback | Recovery | Stop | Replay | Result | Run record | Claim boundary |",
        "|---|---|---|:---:|---:|---:|---:|---:|---:|---:|---|---|---|",
    ])

    for row in v2_rows:
        result = validate_v2_row(row)
        components = result["components"]
        lines.append(
            f"| {result['scenarioId']} v{result['scenarioVersion']} | {result['entrant']} | "
            f"{result['processType']} | {fmt_bool(result['ranked'])} | {fmt_score(result['compositeScore'])} | "
            f"{fmt_score(components['artifactQuality']['score'])} | "
            f"{fmt_score(components['feedbackIntegration']['score'])} | "
            f"{fmt_score(components['recoveryHandoff']['score'])} | "
            f"{fmt_score(components['stopCondition']['score'])} | "
            f"{fmt_score(components['evidenceReplay']['score'])} | "
            f"{link(row['resultJson'])} | {link(row['runRecord'])} | {row['claimBoundary']} |"
        )

    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if results/leaderboard.md is not up to date")
    parser.add_argument("--write", action="store_true", help="write results/leaderboard.md")
    args = parser.parse_args()
    data = load_json(RESULTS_JSON)
    rendered = render(data)
    if args.check:
        current = LEADERBOARD.read_text() if LEADERBOARD.exists() else ""
        if current != rendered:
            print("results/leaderboard.md is not up to date; run python3 scripts/render_results.py --write", file=sys.stderr)
            return 1
        print("OK: results/leaderboard.md is up to date")
        return 0
    if args.write:
        LEADERBOARD.write_text(rendered)
        print(f"wrote {LEADERBOARD.relative_to(ROOT)}")
        return 0
    print(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
