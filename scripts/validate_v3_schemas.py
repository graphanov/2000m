#!/usr/bin/env python3
"""Validate 2000m v3 schema scaffolding and fixtures.

The v3 foundation intentionally avoids third-party JSON Schema dependencies.
The JSON Schema files are the public contract; this script enforces enough of
Draft 2020-12 for the committed fixtures plus the v3-specific safety invariants
that schema syntax alone cannot express.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
V3 = ROOT / "v3"
SCHEMAS = {
    "2000m.v3.manifest.v1": V3 / "manifest.schema.json",
    "2000m.v3.campaign.v1": V3 / "campaign.schema.json",
    "2000m.v3.run-record.v1": V3 / "run-record.schema.json",
    "2000m.v3.result.v1": V3 / "result.schema.json",
    "2000m.v3.visual-package.v1": V3 / "visual-package.schema.json",
}
REQUIRED_SPEC_FILES = [
    V3 / "manifest.schema.json",
    V3 / "campaign.schema.json",
    V3 / "run-record.schema.json",
    V3 / "result.schema.json",
    V3 / "visual-package.schema.json",
    V3 / "2000m.driver.v3.md",
    V3 / "MECHANICAL_AC_SPEC.md",
    V3 / "VISUAL_RUBRIC.md",
    V3 / "SCORING_MODEL.md",
    V3 / "SANDBOX_AND_RESOURCE_POLICY.md",
]
VALID_FIXTURES = sorted((V3 / "examples" / "valid").glob("*.json"))
INVALID_FIXTURES = {
    V3 / "examples" / "invalid" / "manifest-private-path.json": "private/local ref",
    V3 / "examples" / "invalid" / "campaign-framework-specific-required-field.json": "additional property",
    V3 / "examples" / "invalid" / "campaign-boolean-seed.json": "integer",
    V3 / "examples" / "invalid" / "visual-package-missing-capture-metadata.json": "required",
    V3 / "examples" / "invalid" / "result-unsupported-claim.json": "unsupported claim",
    V3 / "examples" / "invalid" / "run-record-invalid-enum.json": "enum",
    V3 / "examples" / "invalid" / "result-mutated-frozen-protocol.json": "frozen protocol mutation",
    V3 / "examples" / "invalid" / "result-ranked-visual-missing-evidence.json": "ranked visual result",
    V3 / "examples" / "invalid" / "run-record-ranked-visual-missing-evidence.json": "ranked run-record visual status",
}

PRIVATE_MARKERS = (
    "/Users/",
    "\\Users\\",
    "file://",
    "~/",
    "../",
    "..\\",
)
OWNER_IDENTITY_MARKERS: tuple[str, ...] = ()
UNSUPPORTED_CLAIM_MARKERS = (
    "makes the model smarter",
    "proves workflow superiority",
    "workflow superiority",
    "adoption proof",
    "public contender result from a fixture",
    "high mechanical score means a good game",
    "score equals model ranking",
)
FRAMEWORK_REQUIRED_MARKERS = (
    ".osc",
    "openscaffold",
    "open-scaffold",
    "open scaffold",
    "frameworkrequired",
    "framework-specific",
    "command-name credit",
)


class V3ValidationError(ValueError):
    pass


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        raise V3ValidationError(f"missing JSON file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise V3ValidationError(f"invalid JSON in {path}: {exc}") from exc


def require(condition: bool, message: str) -> None:
    if not condition:
        raise V3ValidationError(message)


def is_plain_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def is_number(value: Any) -> bool:
    return (isinstance(value, int) or isinstance(value, float)) and not isinstance(value, bool)


def resolve_ref(schema: dict[str, Any], root: dict[str, Any]) -> dict[str, Any]:
    ref = schema.get("$ref")
    if not isinstance(ref, str):
        return schema
    require(ref.startswith("#/$defs/"), f"only local $defs refs are supported by validator, got {ref}")
    name = ref.rsplit("/", 1)[-1]
    defs = root.get("$defs", {})
    require(name in defs, f"unresolved $ref {ref}")
    target = defs[name]
    require(isinstance(target, dict), f"$ref target {ref} is not a schema object")
    return target


def validate_json_schema(instance: Any, schema: dict[str, Any], *, root: dict[str, Any] | None = None, path: str = "$", seen_refs: int = 0) -> None:
    root = root or schema
    if "$ref" in schema:
        require(seen_refs < 20, f"{path}: excessive $ref recursion")
        return validate_json_schema(instance, resolve_ref(schema, root), root=root, path=path, seen_refs=seen_refs + 1)

    if "const" in schema:
        require(instance == schema["const"], f"{path}: expected const {schema['const']!r}, got {instance!r}")
    if "enum" in schema:
        require(instance in schema["enum"], f"{path}: value {instance!r} not in enum {schema['enum']!r}")

    schema_type = schema.get("type")
    if isinstance(schema_type, list):
        if any(_matches_type(instance, item) for item in schema_type):
            pass
        else:
            raise V3ValidationError(f"{path}: expected one of types {schema_type!r}, got {type(instance).__name__}")
    elif isinstance(schema_type, str):
        require(_matches_type(instance, schema_type), f"{path}: expected type {schema_type}, got {type(instance).__name__}")

    if isinstance(instance, str):
        if "minLength" in schema:
            require(len(instance) >= schema["minLength"], f"{path}: string shorter than minLength {schema['minLength']}")
        if "pattern" in schema:
            require(re.search(schema["pattern"], instance), f"{path}: string {instance!r} does not match pattern {schema['pattern']!r}")

    if is_number(instance):
        if "minimum" in schema:
            require(instance >= schema["minimum"], f"{path}: value {instance!r} below minimum {schema['minimum']!r}")
        if "maximum" in schema:
            require(instance <= schema["maximum"], f"{path}: value {instance!r} above maximum {schema['maximum']!r}")

    if isinstance(instance, list):
        if "minItems" in schema:
            require(len(instance) >= schema["minItems"], f"{path}: array has fewer than minItems {schema['minItems']}")
        if "maxItems" in schema:
            require(len(instance) <= schema["maxItems"], f"{path}: array has more than maxItems {schema['maxItems']}")
        if schema.get("uniqueItems"):
            encoded = [json.dumps(item, sort_keys=True) for item in instance]
            require(len(encoded) == len(set(encoded)), f"{path}: array items are not unique")
        if "items" in schema:
            items_schema = schema["items"]
            require(isinstance(items_schema, dict), f"{path}: validator supports only object items schemas")
            for idx, item in enumerate(instance):
                validate_json_schema(item, items_schema, root=root, path=f"{path}[{idx}]", seen_refs=seen_refs)

    if isinstance(instance, dict):
        required = schema.get("required", [])
        require(isinstance(required, list), f"{path}: required must be a list")
        for key in required:
            require(key in instance, f"{path}: missing required property {key!r}")
        properties = schema.get("properties", {})
        require(isinstance(properties, dict), f"{path}: properties must be object")
        if schema.get("additionalProperties") is False:
            allowed = set(properties)
            extra = sorted(set(instance) - allowed)
            if extra:
                raise V3ValidationError(f"{path}: additional property {extra[0]!r} is not allowed")
        for key, value in instance.items():
            if key in properties:
                prop_schema = properties[key]
                require(isinstance(prop_schema, dict), f"{path}.{key}: property schema must be object")
                validate_json_schema(value, prop_schema, root=root, path=f"{path}.{key}", seen_refs=seen_refs)


def _matches_type(value: Any, schema_type: str) -> bool:
    if schema_type == "object":
        return isinstance(value, dict)
    if schema_type == "array":
        return isinstance(value, list)
    if schema_type == "string":
        return isinstance(value, str)
    if schema_type == "integer":
        return is_plain_int(value)
    if schema_type == "number":
        return is_number(value)
    if schema_type == "boolean":
        return isinstance(value, bool)
    if schema_type == "null":
        return value is None
    raise V3ValidationError(f"unsupported schema type {schema_type!r}")


def iter_strings(value: Any, path: str = "$", *, include_keys: bool = False):
    if isinstance(value, str):
        yield path, value
    elif isinstance(value, list):
        for idx, item in enumerate(value):
            yield from iter_strings(item, f"{path}[{idx}]", include_keys=include_keys)
    elif isinstance(value, dict):
        for key, item in value.items():
            if include_keys:
                yield f"{path}.{key}.__key__", str(key)
            yield from iter_strings(item, f"{path}.{key}", include_keys=include_keys)


def check_public_safe_strings(value: Any, *, allow_banned_claim_list: bool = False) -> None:
    for path, text in iter_strings(value):
        stripped = text.strip()
        lowered = stripped.lower()
        if path.endswith(".bannedClaims") or ".bannedClaims[" in path:
            if allow_banned_claim_list:
                continue
        require(not stripped.startswith("/"), f"{path}: private/local ref starts with absolute slash: {text!r}")
        require(not stripped.startswith("~"), f"{path}: private/local ref starts with home marker: {text!r}")
        require(not re.match(r"^[A-Za-z]:", stripped), f"{path}: private/local ref uses Windows drive path: {text!r}")
        require(not stripped.startswith("\\\\"), f"{path}: private/local ref uses UNC path: {text!r}")
        for marker in PRIVATE_MARKERS:
            require(marker.lower() not in lowered, f"{path}: private/local ref marker {marker!r}: {text!r}")
        for marker in OWNER_IDENTITY_MARKERS:
            require(marker.lower() not in lowered, f"{path}: owner identity marker {marker!r}: {text!r}")
        # Discord snowflakes are typically 17-20 digits. Keep a broad guard for public records.
        require(not bool(re.search(r"\b\d{17,20}\b", stripped)), f"{path}: possible Discord ID: {text!r}")


def check_unsupported_claims(value: Any) -> None:
    for path, text in iter_strings(value):
        if path.endswith(".bannedClaims") or ".bannedClaims[" in path:
            continue
        lowered = text.lower()
        for marker in UNSUPPORTED_CLAIM_MARKERS:
            require(marker not in lowered, f"{path}: unsupported claim marker {marker!r}")


def check_schema_neutrality(schema: dict[str, Any], path: Path) -> None:
    for string_path, text in iter_strings(schema, include_keys=True):
        lowered = text.lower()
        if string_path.endswith(".__key__") or ".required[" in string_path:
            for marker in FRAMEWORK_REQUIRED_MARKERS:
                require(marker not in lowered, f"{path}:{string_path}: framework-specific schema field marker {marker!r}")
        # Schema files must not contain native controller/Lane C implementation fields.
        require("controllerstatus" not in lowered, f"{path}:{string_path}: controllerStatus belongs to v2, not v3 foundation")


def validate_semantics(data: dict[str, Any], schema_version: str) -> None:
    check_public_safe_strings(data, allow_banned_claim_list=True)
    check_unsupported_claims(data)

    if schema_version == "2000m.v3.campaign.v1":
        freeze = data["protocolFreeze"]
        require(freeze["freezeBeforeLiveResults"] is True, "campaign protocol must freeze before live results")
        require(freeze["noProtocolMutationAfterLiveResults"] is True, "campaign must forbid protocol mutation after live results")
        for seed in freeze["visualSeeds"]:
            require(is_plain_int(seed), "campaign visualSeeds must contain integers, not booleans")
        seen_lanes: set[str] = set()
        enabled_lanes = {lane["laneId"] for lane in data["lanes"] if lane["enabled"]}
        for lane in data["lanes"]:
            lane_id = lane["laneId"]
            require(lane_id not in seen_lanes, f"duplicate laneId {lane_id}")
            seen_lanes.add(lane_id)
            require(lane_id != "C", "Lane C/controller behavior is out of scope for the v3 foundation slice")
            require("controller" not in json.dumps(lane).lower(), "controller behavior is out of scope for the v3 foundation slice")
            require(lane["processType"] in {"naked-model", "workflow-system", "scripted-agent", "human-operated", "other"}, "unsupported processType")
        for pair in data["pairs"]:
            require(is_plain_int(pair["taskSeed"]), "campaign pair taskSeed must be integer, not boolean")
            require(set(pair["enabledLanes"]).issubset(enabled_lanes), f"pair {pair['pairId']} references disabled or missing lane")
    elif schema_version == "2000m.v3.run-record.v1":
        freeze = data["protocolFreeze"]
        require(freeze["frozen"] is True, "run record must point at a frozen campaign/protocol")
        require(freeze["changedAfterLiveResults"] is False, "frozen protocol mutation is calibration-only and invalid in foundation fixtures")
        require(freeze["scorerMutationObserved"] is False, "scorer mutation is invalid in foundation fixtures")
        require(data["entrant"]["processType"] in {"naked-model", "workflow-system", "scripted-agent", "human-operated", "other"}, "unsupported processType")
        visual = data["visual"]
        if visual["ranked"] is True or visual["blockReason"] == "none":
            require(visual["ranked"] is True, "ranked run-record visual status must set visual.ranked=true when blockReason is none")
            require(visual["blockReason"] == "none", "ranked run-record visual status must use blockReason none")
            require(bool(visual["visualPackageRef"].strip()), "ranked run-record visual status requires non-empty visualPackageRef")
            require(bool(visual.get("captureCommandResultRef", "").strip()), "ranked run-record visual status requires non-empty captureCommandResultRef")
    elif schema_version == "2000m.v3.result.v1":
        freeze = data["protocolFreeze"]
        require(freeze["changedAfterLiveResults"] is False, "frozen protocol mutation is calibration-only and invalid in foundation fixtures")
        require(freeze["scorerMutationObserved"] is False, "frozen protocol mutation is calibration-only and invalid in foundation fixtures")
        require(freeze["calibrationOnlyIfChanged"] is True, "changed protocol must force calibration-only handling")
        require(data["claimBoundary"] != "public-benchmark-support", "foundation fixtures must not claim public benchmark support")
        require(data["evidence"]["claimBoundary"] == data["claimBoundary"], "evidence claimBoundary must match result claimBoundary")
        mechanical = data["mechanical"]
        acs = mechanical.get("acs")
        if acs is not None:
            for ac in acs:
                require(bool(ac["detail"].strip()), f"mechanical.acs {ac['id']} detail must be non-empty")
            failed_ids = [ac["id"] for ac in acs if ac["pass"] is False]
            pass_count = sum(1 for ac in acs if ac["pass"] is True)
            require(mechanical["failedAcs"] == failed_ids, "mechanical.failedAcs must match failed mechanical.acs item ids")
            require(mechanical["totalAcs"] == len(acs), "mechanical.totalAcs must equal mechanical.acs length")
            require(mechanical["passCount"] == pass_count, "mechanical.passCount must equal passing mechanical.acs item count")
        visual = data["visual"]
        if visual["ranked"] is True or visual["blockReason"] == "none":
            require(visual["ranked"] is True, "ranked visual result must set visual.ranked=true when blockReason is none")
            require(visual["blockReason"] == "none", "ranked visual result must use blockReason none")
            require(bool(visual["visualPackageRef"].strip()), "ranked visual result requires non-empty visualPackageRef")
            require(bool(visual["rubricRecordRef"].strip()), "ranked visual result requires non-empty rubricRecordRef")
            require(visual["captureDeterminism"] == "passed", "ranked visual result requires captureDeterminism passed")
    elif schema_version == "2000m.v3.visual-package.v1":
        require(data["anonymized"] is True, "visual package must be anonymized before blind review")
        require(data["mappingSealedBeforeReview"] is True, "blind label map must be sealed before review")
        for window in data["windows"]:
            for key in ("seed", "captureCommand", "screenshotRef", "replayRef", "frameMetadataRef", "rubricMetadataRef", "fps", "frameCount", "inputSequenceRef", "stateChecksum", "frameChecksum"):
                require(key in window, f"visual package missing capture metadata {key}")
            require(is_plain_int(window["seed"]), "capture seed must be integer, not boolean")
    elif schema_version == "2000m.v3.manifest.v1":
        require(data["protocolVersion"] == "2000m.driver.v3", "manifest protocolVersion must be 2000m.driver.v3")
        if "capture" not in data or "playable" not in data:
            # Allowed for mechanical-only manifests, but visual/product rank must be blocked elsewhere.
            pass


def validate_fixture(path: Path) -> None:
    data = load_json(path)
    require(isinstance(data, dict), f"{path}: top-level JSON must be object")
    schema_version = data.get("schemaVersion")
    require(isinstance(schema_version, str), f"{path}: missing schemaVersion")
    require(schema_version in SCHEMAS, f"{path}: unsupported v3 schemaVersion {schema_version!r}")
    schema = load_json(SCHEMAS[schema_version])
    require(isinstance(schema, dict), f"{SCHEMAS[schema_version]}: schema must be object")
    validate_json_schema(data, schema)
    validate_semantics(data, schema_version)


def validate_all() -> None:
    for path in REQUIRED_SPEC_FILES:
        require(path.exists(), f"missing required v3 foundation file: {path.relative_to(ROOT)}")
    for path in SCHEMAS.values():
        schema = load_json(path)
        require(isinstance(schema, dict), f"{path}: schema must be object")
        require(schema.get("$schema") == "https://json-schema.org/draft/2020-12/schema", f"{path}: schema draft marker must be 2020-12")
        check_schema_neutrality(schema, path.relative_to(ROOT))
    require(bool(VALID_FIXTURES), "no v3 valid fixtures found")
    for path in VALID_FIXTURES:
        validate_fixture(path)
        print(f"OK valid: {path.relative_to(ROOT)}")
    for path, expected in INVALID_FIXTURES.items():
        try:
            validate_fixture(path)
        except V3ValidationError as exc:
            message = str(exc).lower()
            require(expected.lower() in message, f"{path.relative_to(ROOT)} failed for unexpected reason: {exc}; expected marker {expected!r}")
            print(f"OK invalid: {path.relative_to(ROOT)} -> {exc}")
        else:
            raise V3ValidationError(f"invalid fixture unexpectedly passed: {path.relative_to(ROOT)}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate v3 schemas, examples, and semantic guard fixtures")
    parser.add_argument("paths", nargs="*", help="Specific v3 JSON fixture paths to validate; defaults to full suite")
    args = parser.parse_args()
    try:
        if args.paths:
            for raw in args.paths:
                path = Path(raw)
                validate_fixture(path if path.is_absolute() else ROOT / path)
                print(f"OK: {raw}")
        else:
            validate_all()
    except V3ValidationError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
