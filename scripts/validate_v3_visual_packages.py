#!/usr/bin/env python3
"""Validate v3 visual packages after deterministic capture generation."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_VALIDATOR = ROOT / "scripts" / "validate_v3_schemas.py"


def load_schema_validator():
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


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()


def sha256_pair(first: Path, second: Path) -> str:
    h = hashlib.sha256()
    h.update(first.read_bytes())
    h.update(second.read_bytes())
    return "sha256:" + h.hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def package_path(raw: Path) -> Path:
    path = raw if raw.is_absolute() else ROOT / raw
    if path.is_dir():
        for candidate in (path / "visual-package.json", path / "visual-package" / "visual-package.json"):
            if candidate.exists():
                return candidate
        raise SystemExit(f"could not find visual-package.json under {path}")
    return path


def resolve_ref(base: Path, ref: str) -> Path:
    require(not ref.startswith("/"), f"visual package ref is absolute: {ref}")
    require(not ref.startswith("~"), f"visual package ref uses home marker: {ref}")
    require(not re.match(r"^[A-Za-z]:", ref), f"visual package ref uses drive path: {ref}")
    require("../" not in ref and "..\\" not in ref, f"visual package ref uses traversal: {ref}")
    require("file://" not in ref.lower(), f"visual package ref uses file URL: {ref}")
    return base / ref


def validate_package(path: Path) -> None:
    validator = load_schema_validator()
    try:
        validator.validate_fixture(path)
    except Exception as exc:  # validator raises its own V3ValidationError
        raise SystemExit(f"schema validation failed for {path}: {exc}") from exc

    data = load_json(path)
    base = path.parent
    artifacts = data["artifacts"]
    for key in ["blindContactSheetRef", "blindLabelMapRef", "screenshotsDirRef", "replaysDirRef", "framesDirRef"]:
        target = resolve_ref(base, artifacts[key])
        require(target.exists(), f"{key} target does not exist: {artifacts[key]}")
    label_map = load_json(resolve_ref(base, artifacts["blindLabelMapRef"]))
    require(label_map.get("sealed") is True, "blind label map must be sealed before review")
    require("laneNames" not in label_map, "sealed blind label map must not expose lane names before rating")

    for item in data["windows"]:
        screenshot = resolve_ref(base, item["screenshotRef"])
        replay = resolve_ref(base, item["replayRef"])
        frames_path = resolve_ref(base, item["frameMetadataRef"])
        rubric_path = resolve_ref(base, item["rubricMetadataRef"])
        for target in (screenshot, replay, frames_path, rubric_path):
            require(target.exists(), f"capture artifact missing: {target}")
            require(target.stat().st_size > 0, f"capture artifact is empty: {target}")
        frames = load_json(frames_path)
        require(frames.get("seed") == item["seed"], f"frames seed mismatch for {frames_path}")
        require(frames.get("window") == item["window"], f"frames window mismatch for {frames_path}")
        require(frames.get("frameCount") == item["frameCount"], f"frames frameCount mismatch for {frames_path}")
        require(frames.get("fps") == item["fps"], f"frames fps mismatch for {frames_path}")
        require(frames.get("stateChecksum") == item["stateChecksum"], f"stateChecksum mismatch for {frames_path}")
        require(frames.get("frameChecksum") == item["frameChecksum"], f"frameChecksum mismatch for {frames_path}")
        require(sha256_pair(screenshot, replay) == item["frameChecksum"], f"frameChecksum does not match screenshot+replay bytes for {frames_path}")
        require(sha256_file(rubric_path).startswith("sha256:"), "rubric metadata checksum computation failed")
    print(f"OK: v3 visual package {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate generated v3 visual package(s)")
    parser.add_argument("paths", nargs="+", help="visual-package.json files or directories containing them")
    args = parser.parse_args()
    for raw in args.paths:
        validate_package(package_path(Path(raw)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
