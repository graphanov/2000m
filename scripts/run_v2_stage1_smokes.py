#!/usr/bin/env python3
"""Run the v2 Stage 1 neutral smoke/calibration checks.

Stage 1 proves the public benchmark machinery is wired correctly before any live
paired pilot results are inspected. It intentionally produces no contender row
and no workflow-superiority claim.
"""

from __future__ import annotations

import argparse
import difflib
import json
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]

PILOT_EXPECTED: dict[str, tuple[bool, float]] = {
    "good": (True, 80.0),
    "weak-ranked": (True, 72.0),
    "wrong-stop": (True, 65.0),
    "missing-output": (False, 65.0),
    "private-path": (False, 50.0),
}

NEUTRAL_CONTRACT_FILES = [
    "v2/scenario.schema.json",
    "v2/run-record.schema.json",
    "v2/result.schema.json",
    "v2/examples/workflow-resilience-smoke.scenario.json",
    "v2/examples/workflow-resilience-pilot.scenario.json",
]

PRIVATE_MARKERS = (
    "/Users/",
    "\\Users\\",
    "file://",
    "~/",
    "../",
    "..\\",
)


def run(command: list[str], *, cwd: Path = ROOT) -> subprocess.CompletedProcess[str]:
    print("$ " + " ".join(command), flush=True)
    return subprocess.run(command, cwd=cwd, check=True, text=True)


def load_json(path: Path) -> Any:
    return json.loads(path.read_text())


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def unified_diff(expected: Path, actual: Path) -> str:
    expected_lines = expected.read_text().splitlines(keepends=True)
    actual_lines = actual.read_text().splitlines(keepends=True)
    return "".join(
        difflib.unified_diff(
            expected_lines,
            actual_lines,
            fromfile=str(expected.relative_to(ROOT)),
            tofile=str(actual),
        )
    )


def assert_result(path: Path, *, ranked: bool, composite: float) -> dict[str, Any]:
    data = load_json(path)
    require(data["ranked"] is ranked, f"{path} ranked={data['ranked']} expected {ranked}")
    require(data["compositeScore"] == composite, f"{path} composite={data['compositeScore']} expected {composite}")
    return data


def smoke_v0_stub(tmp: Path) -> None:
    out = tmp / "v0-stub.json"
    run(["cargo", "run", "--quiet", "-p", "m2000-conformance", "--", "stub-driver", "--json-out", str(out)])
    data = load_json(out)
    require(data["determinism"]["pass"] is True, "v0 stub determinism must pass")
    pass_count = data["passCount"]
    require(1 <= pass_count <= 5, f"v0 stub passCount {pass_count} outside discrimination band 1..5")
    print(f"OK: v0 stub determinism pass, {pass_count}/{data['totalAcs']}")


def smoke_v2_scorer(tmp: Path) -> None:
    out = tmp / "v2-smoke.json"
    run([
        "cargo",
        "run",
        "--quiet",
        "-p",
        "m2000-v2-conformance",
        "--",
        "v2/examples/workflow-resilience-smoke.scenario.json",
        "v2/examples/weak-run-record.json",
        "--json-out",
        str(out),
    ])
    data = assert_result(out, ranked=True, composite=80.0)
    require(data["entrant"] == "weak-sample", "v2 smoke should score the weak-sample fixture")
    require(data["processType"] == "scripted-agent-loop", "v2 smoke uses a neutral scripted-agent-loop process type")
    print("OK: v2 smoke fixture ranked below ceiling at composite 80.0")


def smoke_pilot_fixtures(tmp: Path) -> None:
    generated_dir = tmp / "pilot-results"
    generated_dir.mkdir()
    for name, (ranked, composite) in PILOT_EXPECTED.items():
        out = generated_dir / f"pilot-{name}-result.json"
        run([
            "cargo",
            "run",
            "--quiet",
            "-p",
            "m2000-v2-conformance",
            "--",
            "v2/examples/workflow-resilience-pilot.scenario.json",
            f"v2/examples/pilot-{name}-run-record.json",
            "--json-out",
            str(out),
        ])
        committed = ROOT / "v2" / "examples" / "results" / f"pilot-{name}-result.json"
        if committed.read_text() != out.read_text():
            raise AssertionError("stale committed v2 result fixture:\n" + unified_diff(committed, out))
        data = assert_result(out, ranked=ranked, composite=composite)
        if not ranked:
            require(
                any(str(warning).startswith("RANK-BLOCK:") for warning in data["warnings"]),
                f"{name} should include a rank-block warning",
            )
    missing = load_json(generated_dir / "pilot-missing-output-result.json")
    require(
        any("missing required output" in warning for warning in missing["warnings"]),
        "missing-output fixture must prove generic required output rank blocker",
    )
    private = load_json(generated_dir / "pilot-private-path-result.json")
    require(
        any("private or local-only" in warning for warning in private["warnings"]),
        "private-path fixture must prove nested scorer-input private ref blocker",
    )
    wrong_stop = load_json(generated_dir / "pilot-wrong-stop-result.json")
    require(wrong_stop["ranked"] is True, "wrong-stop fixture should remain ranked but penalized")
    require(wrong_stop["components"]["stopCondition"]["score"] == 0.0, "wrong-stop fixture must expose stop-condition failure")
    print("OK: v2 pilot fixtures regenerate exactly and exercise ranked/rank-blocked outcomes")


def smoke_result_spine() -> None:
    run([sys.executable, "scripts/render_results.py", "--check"])
    print("OK: result spine renders from machine-readable JSON")


def smoke_campaign() -> None:
    run([sys.executable, "scripts/validate_v2_campaigns.py"])
    print("OK: paired campaign fixture validates with Lane C disabled")


def smoke_neutral_contracts() -> None:
    forbidden = [
        "open scaffold",
        ".osc",
        "osc evolve",
        "osc evidence",
        "requires framework",
        "framework:",
    ]
    findings: list[str] = []
    for rel in NEUTRAL_CONTRACT_FILES:
        path = ROOT / rel
        text = path.read_text().lower()
        for marker in forbidden:
            if marker in text:
                findings.append(f"{rel}: contains neutral-contract forbidden marker `{marker}`")
    if findings:
        raise AssertionError("neutral v2 contracts require or privilege workflow-specific fields:\n" + "\n".join(findings))
    print("OK: neutral v2 contracts contain no Open Scaffold-specific required fields")


def png_chunk(kind: bytes, payload: bytes) -> bytes:
    return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)


def png_bytes(seed: int, window_index: int, width: int = 64, height: int = 36) -> bytes:
    rows = bytearray()
    for y in range(height):
        rows.append(0)  # no filter
        for x in range(width):
            r = (seed + x * 7 + y * 3 + window_index * 17) % 256
            g = (seed // 3 + x * 5 + y * 11 + window_index * 23) % 256
            b = (seed // 7 + x * 13 + y * 2 + window_index * 31) % 256
            rows.extend([r, g, b])
    return b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)) + png_chunk(b"IDAT", zlib.compress(bytes(rows))) + png_chunk(b"IEND", b"")


def single_pixel_gif() -> bytes:
    return (
        b"GIF89a"
        b"\x01\x00\x01\x00\x80\x00\x00"
        b"\x00\x00\x00\xff\xff\xff"
        b"!\xf9\x04\x00\x00\x00\x00\x00"
        b",\x00\x00\x00\x00\x01\x00\x01\x00\x00"
        b"\x02\x02D\x01\x00;"
    )


def relative_public_ref(path: Path, base: Path) -> str:
    ref = path.relative_to(base).as_posix()
    lowered = ref.lower().strip()
    require(not lowered.startswith("/"), f"visual ref is absolute: {ref}")
    for marker in PRIVATE_MARKERS:
        require(marker.lower() not in lowered, f"visual ref contains private marker `{marker}`: {ref}")
    return ref


def build_visual_package(out_dir: Path) -> None:
    campaign = load_json(ROOT / "v2/examples/paired-private-pilot.campaign.json")
    visual = campaign["visualTrack"]
    scenario_ref = campaign["freezeRules"]["frozenRefs"]["scenario"]
    seeds = visual["fixedSeeds"]
    windows = visual["captureWindows"]

    screenshots_dir = out_dir / "screenshots"
    replay_dir = out_dir / "replay"
    screenshots_dir.mkdir(parents=True, exist_ok=True)
    replay_dir.mkdir(parents=True, exist_ok=True)

    screenshots: list[dict[str, Any]] = []
    replay_events: list[dict[str, Any]] = []
    for seed in seeds:
        for index, window in enumerate(windows, start=1):
            filename = f"seed-{seed}-window-{index:02d}.png"
            path = screenshots_dir / filename
            path.write_bytes(png_bytes(seed, index))
            screenshots.append({"seed": seed, "window": window, "ref": relative_public_ref(path, out_dir)})
            replay_events.append({"seed": seed, "window": window, "tickStart": index * 100, "tickEnd": index * 100 + 24, "eventCount": (seed + index) % 7 + 1})

    gif_path = out_dir / "capture-smoke.gif"
    gif_path.write_bytes(single_pixel_gif())

    replay_log = replay_dir / "replay-log.jsonl"
    replay_log.write_text("".join(json.dumps(event, sort_keys=True) + "\n" for event in replay_events))

    frame_metadata = replay_dir / "frame-metadata.json"
    frame_metadata.write_text(json.dumps({"schemaVersion": "2000m.v2.frame-metadata.v1", "frames": screenshots}, indent=2, sort_keys=True) + "\n")

    rubric_record = out_dir / "rubric-record.json"
    rubric_record.write_text(json.dumps({
        "schemaVersion": "2000m.v2.visual-rubric-record.v1",
        "stage1SmokeOnly": True,
        "rubricRef": visual["rubricRef"],
        "scores": [],
        "claimBoundary": "packaging smoke only; no visual quality judgment",
    }, indent=2, sort_keys=True) + "\n")

    blind_label_map = out_dir / "blind-label-map.json"
    blind_label_map.write_text(json.dumps({
        "schemaVersion": "2000m.v2.blind-label-map.v1",
        "stage1SmokeOnly": True,
        "labels": {
            "artifact-x": "neutral-smoke-a",
            "artifact-y": "neutral-smoke-b",
        },
        "claimBoundary": "dummy map for packaging smoke; not a live paired campaign",
    }, indent=2, sort_keys=True) + "\n")

    manifest = {
        "schemaVersion": "2000m.v2.visual-package.v1",
        "packageId": "stage1-neutral-visual-smoke",
        "stage": "stage-1-smoke-calibration",
        "scenario": scenario_ref,
        "campaign": "v2/examples/paired-private-pilot.campaign.json",
        "fixedSeeds": seeds,
        "captureWindows": windows,
        "requiredArtifacts": visual["requiredArtifacts"],
        "artifacts": {
            "screenshot": [item["ref"] for item in screenshots],
            "gif": relative_public_ref(gif_path, out_dir),
            "replay-log": relative_public_ref(replay_log, out_dir),
            "frame-metadata": relative_public_ref(frame_metadata, out_dir),
            "rubric-record": relative_public_ref(rubric_record, out_dir),
            "blind-label-map": relative_public_ref(blind_label_map, out_dir),
        },
        "claimBoundary": "Stage 1 packaging smoke only; no live result and no visual superiority claim.",
    }
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


def validate_visual_package(out_dir: Path) -> None:
    manifest = load_json(out_dir / "manifest.json")
    campaign = load_json(ROOT / "v2/examples/paired-private-pilot.campaign.json")
    visual = campaign["visualTrack"]
    artifacts = manifest["artifacts"]
    for required in visual["requiredArtifacts"]:
        require(required in artifacts, f"visual package missing required artifact kind `{required}`")
    expected_screenshot_count = len(visual["fixedSeeds"]) * len(visual["captureWindows"])
    require(len(artifacts["screenshot"]) == expected_screenshot_count, "visual package screenshot count must cover every fixed seed/window")
    refs: list[str] = []
    for value in artifacts.values():
        if isinstance(value, list):
            refs.extend(value)
        else:
            refs.append(value)
    for ref in refs:
        path = out_dir / ref
        require(path.exists(), f"visual artifact ref missing: {ref}")
        require(path.stat().st_size > 0, f"visual artifact ref is empty: {ref}")
        relative_public_ref(path, out_dir)
    require((out_dir / artifacts["gif"]).read_bytes().startswith(b"GIF89a"), "visual GIF smoke artifact must be a GIF")
    png_refs = artifacts["screenshot"]
    require((out_dir / png_refs[0]).read_bytes().startswith(b"\x89PNG\r\n\x1a\n"), "visual screenshot smoke artifact must be a PNG")
    print(f"OK: visual/replay package produced from fixed seeds at {out_dir}")


def smoke_visual_package(tmp: Path, visual_out: str | None) -> None:
    if visual_out:
        out_dir = Path(visual_out).resolve()
        if out_dir.exists():
            require(out_dir.is_dir(), f"--visual-out exists and is not a directory: {out_dir}")
            require(not any(out_dir.iterdir()), f"--visual-out must be empty so existing evidence is not overwritten: {out_dir}")
        else:
            out_dir.mkdir(parents=True)
    else:
        out_dir = tmp / "visual-package"
        out_dir.mkdir()
    build_visual_package(out_dir)
    validate_visual_package(out_dir)


def main() -> int:
    parser = argparse.ArgumentParser(description="Run 2000m v2 Stage 1 neutral smoke/calibration checks")
    parser.add_argument("--visual-out", help="optional directory where the generated visual smoke package should be written")
    args = parser.parse_args()

    with tempfile.TemporaryDirectory(prefix="2000m-v2-stage1-") as tmp_name:
        tmp = Path(tmp_name)
        smoke_v0_stub(tmp)
        smoke_v2_scorer(tmp)
        smoke_pilot_fixtures(tmp)
        smoke_result_spine()
        smoke_campaign()
        smoke_neutral_contracts()
        smoke_visual_package(tmp, args.visual_out)

    print("OK: Stage 1 neutral smokes passed; no live pilot result or workflow-value claim was produced")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
