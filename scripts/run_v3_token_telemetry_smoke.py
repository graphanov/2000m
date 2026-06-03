#!/usr/bin/env python3
"""Run public-safe smoke coverage for v3 token telemetry collection."""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def load_json(path: Path) -> Any:
    return json.loads(path.read_text())


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def run(cmd: list[str], cwd: Path = ROOT) -> None:
    print("$", " ".join(cmd))
    subprocess.run(cmd, cwd=cwd, check=True)


def run_record(seed: int, lane: str, generation: int, pass_count: int, ranked: bool) -> dict[str, Any]:
    data = load_json(ROOT / "v3" / "examples" / "valid" / "run-record.foundation.json")
    data["taskSeed"] = seed
    data["laneId"] = lane
    data["mechanical"]["ranked"] = ranked
    data["mechanical"]["passCount"] = pass_count
    data["mechanical"]["totalAcs"] = 24
    data["mechanical"]["failedAcs"] = [] if pass_count == 24 else ["M16"]
    data["mechanical"]["resultJsonRef"] = (
        f"records/pilot-seed-{seed}/lane-{lane}/generation-{generation:02d}/v3-result.json"
    )
    data["finalRecommendation"]["decision"] = "stop" if ranked else "continue"
    return data


def result_record(seed: int, lane: str, generation: int, pass_count: int, ranked: bool) -> dict[str, Any]:
    data = load_json(ROOT / "v3" / "examples" / "valid" / "result.foundation.json")
    data["taskSeed"] = seed
    data["laneId"] = lane
    data["mechanical"]["ranked"] = ranked
    data["mechanical"]["passCount"] = pass_count
    data["mechanical"]["totalAcs"] = 24
    data["mechanical"]["failedAcs"] = [] if pass_count == 24 else ["M16"]
    data["mechanical"]["compositeScore"] = 100 if pass_count == 24 else pass_count / 24 * 100
    data["mechanical"]["resultJsonRef"] = (
        f"records/pilot-seed-{seed}/lane-{lane}/generation-{generation:02d}/v3-result.json"
    )
    data["workflow"]["finalRecommendation"] = "stop" if ranked else "continue"
    return data


def write_generation(run_root: Path, seed: int, lane: str, generation: int, pass_count: int, ranked: bool, tokens: int) -> Path:
    gen_dir = run_root / "records" / f"pilot-seed-{seed}" / f"lane-{lane}" / f"generation-{generation:02d}"
    gen_dir.mkdir(parents=True, exist_ok=True)
    write_json(gen_dir / "run-record.json", run_record(seed, lane, generation, pass_count, ranked))
    write_json(gen_dir / "v3-result.json", result_record(seed, lane, generation, pass_count, ranked))
    (gen_dir / "codex-stderr.log").write_text(f"synthetic output\n\ntokens used\n{tokens:,}\n")
    return gen_dir


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="m2000-v3-token-telemetry-") as temp:
        run_root = Path(temp)
        write_generation(run_root, 101, "A", 1, 22, False, 1000)
        write_generation(run_root, 101, "A", 2, 24, True, 2000)
        write_generation(run_root, 101, "B", 1, 18, False, 3000)
        write_generation(run_root, 101, "B", 2, 24, True, 4000)
        summary_path = run_root / "token-telemetry-summary.json"
        markdown_path = run_root / "token-telemetry-summary.md"
        report_path = run_root / "final-private-report.json"
        run([
            sys.executable,
            "scripts/collect_v3_token_telemetry.py",
            str(run_root),
            "--write-records",
            "--json-out",
            str(summary_path),
            "--markdown-out",
            str(markdown_path),
        ])
        run([
            sys.executable,
            "scripts/render_v3_private_report.py",
            str(run_root),
            "--json-out",
            str(report_path),
        ])
        summary = load_json(summary_path)
        report = load_json(report_path)
        assert summary["aggregates"]["tokensByLane"] == {"A": 3000, "B": 7000}, summary
        assert summary["aggregates"]["laneDeltaFullRun"]["laneBMinusLaneA"] == 4000, summary
        assert summary["aggregates"]["tokensUntilFirstRankedMechanicalCeiling"]["byLane"] == {"A": 3000, "B": 7000}, summary
        assert report["finalPrivateVerdict"] == "no support", report
        assert report["visualProductTrack"]["blocked"] is True, report
        assert report["tokenTelemetry"]["aggregates"]["laneDeltaFullRun"]["laneBMinusLaneA"] == 4000, report
        meaningful = [
            row for row in summary["aggregates"]["tokensPerPassedAcImprovement"]
            if row["meaningful"]
        ]
        assert len(meaningful) == 2, meaningful
        for path in sorted(run_root.glob("records/pilot-seed-*/lane-*/generation-*/run-record.json")):
            run([sys.executable, "scripts/validate_v3_schemas.py", str(path)])
            record = load_json(path)
            assert record["runtimeTelemetry"]["tokens"]["input"] is None, record
            assert "input" in record["runtimeTelemetry"]["tokens"]["unavailableFields"], record
        assert "Token/cost telemetry is separate from mechanical correctness" in markdown_path.read_text()
    print("OK: v3 token telemetry smoke parsed totals, wrote run-record telemetry, and validated schemas")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
