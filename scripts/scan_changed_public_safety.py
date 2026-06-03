#!/usr/bin/env python3
"""Scan changed public repo files for identity/path/transcript/claim leaks."""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parents[1]
SELF = Path(__file__).resolve()
TEXT_EXTENSIONS = {
    ".md", ".txt", ".json", ".py", ".rs", ".toml", ".lock", ".html", ".yml", ".yaml"
}
PRIVATE_MARKERS = ["/Users/", "\\Users\\", "file://", "~/", "..\\"]
OWNER_MARKERS = ["dan" + "imal"]
UNSUPPORTED_MARKERS = [
    "Open Scaffold is better",
    "makes the model smarter",
    "proves workflow superiority",
    "workflow superiority",
    "adoption proof",
    "model intelligence",
    "public benchmark support",
    "public support claim",
]
NEGATION_TOKENS = ("not ", "no ", "must not ", "does not ", "is not ", "never ")
TRANSCRIPT_MARKERS = [
    "<" + "transcript",
    "role" + ": " + "assistant",
    "role" + ": " + "user",
    "raw message dump",
]
EMAIL_RE = re.compile(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b")
DISCORD_ID_RE = re.compile(r"\b\d{17,20}\b")


def run_git(args: list[str]) -> list[str]:
    completed = subprocess.run(["git", *args], cwd=ROOT, check=True, text=True, stdout=subprocess.PIPE)
    return [line for line in completed.stdout.splitlines() if line.strip()]


def try_git(args: list[str]) -> list[str]:
    completed = subprocess.run(["git", *args], cwd=ROOT, check=False, text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    if completed.returncode != 0:
        return []
    return [line for line in completed.stdout.splitlines() if line.strip()]


def git_ref_exists(ref: str) -> bool:
    return subprocess.run(["git", "rev-parse", "--verify", ref], cwd=ROOT, check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0


def maybe_fetch_base_ref(base_ref: str) -> None:
    if not base_ref or git_ref_exists(f"origin/{base_ref}"):
        return
    subprocess.run(
        ["git", "fetch", "--quiet", "--depth=100", "origin", f"{base_ref}:refs/remotes/origin/{base_ref}"],
        cwd=ROOT,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def diff_base() -> str:
    explicit = os.environ.get("PUBLIC_SAFETY_BASE")
    if explicit:
        return explicit

    github_base = os.environ.get("GITHUB_BASE_REF", "")
    if github_base:
        maybe_fetch_base_ref(github_base)

    candidates: list[str] = []
    if github_base:
        candidates.extend([f"origin/{github_base}", github_base])
    candidates.extend(["origin/main", "main"])
    for candidate in candidates:
        if not git_ref_exists(candidate):
            continue
        base = try_git(["merge-base", candidate, "HEAD"])
        if base:
            return base[0]
    return "HEAD"


def changed_files() -> list[Path]:
    base = diff_base()
    tracked = run_git(["diff", "--name-only", "--diff-filter=ACMRTUXB", base, "HEAD", "--"])
    working = run_git(["diff", "--name-only", "--diff-filter=ACMRTUXB", "HEAD", "--"])
    untracked = run_git(["ls-files", "--others", "--exclude-standard"])
    out: list[Path] = []
    for raw in [*tracked, *working, *untracked]:
        path = ROOT / raw
        if path == SELF or not path.is_file():
            continue
        out.append(path)
    return sorted(set(out))


def iter_json_strings(value: Any, path: str = "$", *, skip_banned_claims: bool = True) -> Iterable[tuple[str, str]]:
    if isinstance(value, str):
        yield path, value
    elif isinstance(value, list):
        for idx, item in enumerate(value):
            yield from iter_json_strings(item, f"{path}[{idx}]", skip_banned_claims=skip_banned_claims)
    elif isinstance(value, dict):
        for key, item in value.items():
            if skip_banned_claims and key == "bannedClaims":
                continue
            yield from iter_json_strings(item, f"{path}.{key}", skip_banned_claims=skip_banned_claims)


def is_negated(text: str, marker_start: int) -> bool:
    prefix = text[max(0, marker_start - 40):marker_start].lower()
    return any(token in prefix for token in NEGATION_TOKENS)


def scan_text(path: Path, strings: Iterable[tuple[str, str]], *, strict_paths: bool) -> list[str]:
    failures: list[str] = []
    owner_home = "/" + "Users" + "/" + "dan" + "imal"
    private_home = "/" + "Users" + "/" + "private"
    win_owner_home = "\\" + "Users" + "\\" + "dan" + "imal"
    win_private_home = "\\" + "Users" + "\\" + "private"
    private_markers = PRIVATE_MARKERS if strict_paths else [owner_home, private_home, win_owner_home, win_private_home]
    for where, text in strings:
        lowered = text.lower()
        for marker in private_markers:
            if marker.lower() in lowered:
                failures.append(f"{path}:{where}: private/local path marker {marker!r}")
        for marker in OWNER_MARKERS:
            if marker in lowered:
                failures.append(f"{path}:{where}: owner identity marker {marker!r}")
        if EMAIL_RE.search(text):
            failures.append(f"{path}:{where}: email-like string")
        if DISCORD_ID_RE.search(text):
            failures.append(f"{path}:{where}: Discord-ID-like numeric string")
        for marker in TRANSCRIPT_MARKERS:
            if marker in lowered:
                failures.append(f"{path}:{where}: transcript-like marker {marker!r}")
        for marker in UNSUPPORTED_MARKERS:
            start = lowered.find(marker.lower())
            if start != -1 and not is_negated(text, start):
                failures.append(f"{path}:{where}: unsupported-claim marker {marker!r}")
    return failures


def strip_allowed_marker_declarations(relative: Path, text: str) -> str:
    """Avoid self-flagging validator marker declarations while scanning the rest."""
    if relative.as_posix() != "scripts/validate_v3_schemas.py":
        return text
    return re.sub(
        r"UNSUPPORTED_CLAIM_MARKERS\s*=\s*\(.*?\)\n",
        "UNSUPPORTED_CLAIM_MARKERS = (...redacted marker declarations...)\n",
        text,
        flags=re.DOTALL,
    )


def scan_file(path: Path) -> list[str]:
    if path.suffix not in TEXT_EXTENSIONS:
        return []
    try:
        text = path.read_text()
    except UnicodeDecodeError:
        return []
    relative = path.relative_to(ROOT)
    text = strip_allowed_marker_declarations(relative, text)
    if path.suffix == ".json":
        try:
            value = json.loads(text)
        except json.JSONDecodeError:
            return scan_text(relative, [("text", text)], strict_paths=True)
        return scan_text(relative, iter_json_strings(value), strict_paths=True)
    return scan_text(relative, [("text", text)], strict_paths=path.suffix in {".md", ".txt", ".html"})


def main() -> int:
    files = changed_files()
    failures: list[str] = []
    for path in files:
        failures.extend(scan_file(path))
    if failures:
        print("FAIL: changed-file public-safety scan found issues", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"OK: public-safety scan passed for {len(files)} changed public files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
