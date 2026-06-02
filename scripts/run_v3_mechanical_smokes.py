#!/usr/bin/env python3
"""Run v3 mechanical scorer scaffold smoke fixtures.

The smoke proves the dedicated v3 Rust harness can score a valid driver fixture,
rank-block invalid enum/schema poison, emit v3 separated-track result JSON, and
leave visual/workflow/evidence tracks separate from mechanical ACs.
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUT = ROOT / "v3" / "fixtures" / "mechanical" / "out"
VALID_ARTIFACT = ROOT / "v3" / "fixtures" / "mechanical" / "valid-artifact"
INVALID_ENUM_ARTIFACT = ROOT / "v3" / "fixtures" / "mechanical" / "invalid-enum-artifact"
TIMEOUT_ARTIFACT = ROOT / "v3" / "fixtures" / "mechanical" / "timeout-artifact"
STDERR_ARTIFACT = ROOT / "v3" / "fixtures" / "mechanical" / "stderr-artifact"
MANIFEST_FIXTURES = [VALID_ARTIFACT, INVALID_ENUM_ARTIFACT, TIMEOUT_ARTIFACT, STDERR_ARTIFACT]


def run(cmd: list[str]) -> None:
    print("$", " ".join(cmd))
    subprocess.run(cmd, cwd=ROOT, check=True)


def load(path: Path) -> dict:
    return json.loads(path.read_text())


def main() -> int:
    parser = argparse.ArgumentParser(description="Run v3 mechanical conformance smoke fixtures")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT, help="repo-relative or absolute output directory for ignored smoke JSON")
    args = parser.parse_args()
    out = args.out if args.out.is_absolute() else ROOT / args.out
    for fixture in MANIFEST_FIXTURES:
        run([sys.executable, "scripts/validate_v3_schemas.py", str((fixture / "2000m.v3.json").relative_to(ROOT))])
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)

    valid_result = out / "valid-result.json"
    invalid_result = out / "invalid-enum-result.json"
    run(["cargo", "run", "-p", "m2000-v3-conformance", "--quiet", "--", str(VALID_ARTIFACT.relative_to(ROOT)), "--json-out", str(valid_result.relative_to(ROOT))])
    run([sys.executable, "scripts/validate_v3_schemas.py", str(valid_result.relative_to(ROOT))])
    run(["cargo", "run", "-p", "m2000-v3-conformance", "--quiet", "--", str(INVALID_ENUM_ARTIFACT.relative_to(ROOT)), "--json-out", str(invalid_result.relative_to(ROOT))])
    run([sys.executable, "scripts/validate_v3_schemas.py", str(invalid_result.relative_to(ROOT))])

    valid = load(valid_result)
    invalid = load(invalid_result)
    assert valid["mechanical"]["ranked"] is True, valid["mechanical"]
    assert valid["mechanical"]["passCount"] == valid["mechanical"]["totalAcs"] == 24, valid["mechanical"]
    assert valid["visual"]["ranked"] is False, valid["visual"]
    assert invalid["mechanical"]["ranked"] is False, invalid["mechanical"]
    assert "M06" in invalid["mechanical"]["failedAcs"], invalid["mechanical"]
    assert invalid["mechanical"]["protocolVersion"] == "2000m.driver.v3"
    print(f"OK: v3 mechanical smokes wrote {valid_result.relative_to(ROOT)} and {invalid_result.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
