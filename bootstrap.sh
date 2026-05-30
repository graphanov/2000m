#!/usr/bin/env bash
# open-scaffold bootstrap for a downstream project
# Tested on macOS system bash (3.2). Avoids GNU-only date flags.
# Idempotent: safe to run any number of times.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
TODAY="$(date +%Y-%m-%d)"
MISSION="$ROOT/MISSION.md"

stamp_changelog() {
  if [ ! -f "$MISSION" ]; then
    return 0
  fi
  STAMP="$TODAY: bootstrap run"
  if ! grep -Fq "$STAMP" "$MISSION"; then
    ANCHOR='<!-- append YYYY-MM-DD entries below this line -->'
    if grep -Fq "$ANCHOR" "$MISSION"; then
      TMPFILE=$(mktemp)
      while IFS= read -r line || [ -n "$line" ]; do
        printf '%s\n' "$line"
        if printf '%s' "$line" | grep -Fq "$ANCHOR"; then
          printf -- '- %s\n' "$STAMP"
        fi
      done < "$MISSION" > "$TMPFILE"
      mv "$TMPFILE" "$MISSION"
    else
      printf -- '- %s\n' "$STAMP" >> "$MISSION"
    fi
  fi
}

mkdir -p "$ROOT/.osc/research"
mkdir -p "$ROOT/.osc/state"
mkdir -p "$ROOT/.osc/plans/active"
mkdir -p "$ROOT/.osc/plans/backlog"
mkdir -p "$ROOT/.osc/plans/done"
mkdir -p "$ROOT/.osc/plans/blocked"
mkdir -p "$ROOT/.osc/releases"

if [ -f "$MISSION" ] && grep -Fq 'mission:unset' "$MISSION"; then
  if [ -t 0 ]; then
    printf '\n'
    printf '=== open-scaffold: Define Your Mission ===\n'
    printf '\n'
    printf 'What is this project? (one sentence)\n> '
    read -r USER_MISSION
    printf '\n'
    printf 'What should it achieve? (main outcomes — separate multiple with semicolons)\n> '
    read -r USER_GOALS
    printf '\n'
    printf 'What should this project NOT do? (separate multiple with semicolons)\n> '
    read -r USER_NONGOALS
    printf '\n'

    if [ -n "$USER_MISSION" ]; then
      GOALS_LIST=""
      if [ -n "$USER_GOALS" ]; then
        GOALS_LIST=$(printf '%s' "$USER_GOALS" | tr ',;' '\n\n' | while read -r item; do
          trimmed=$(printf '%s' "$item" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
          if [ -n "$trimmed" ]; then
            printf '- %s\n' "$trimmed"
          fi
        done)
      fi
      if [ -z "$GOALS_LIST" ]; then
        GOALS_LIST="- TODO: define your project's goals"
      fi

      NONGOALS_LIST=""
      if [ -n "$USER_NONGOALS" ]; then
        NONGOALS_LIST=$(printf '%s' "$USER_NONGOALS" | tr ',;' '\n\n' | while read -r item; do
          trimmed=$(printf '%s' "$item" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
          if [ -n "$trimmed" ]; then
            printf '- %s\n' "$trimmed"
          fi
        done)
      fi
      if [ -z "$NONGOALS_LIST" ]; then
        NONGOALS_LIST="- TODO: define your project's non-goals"
      fi

      cat > "$MISSION" << MISSION_EOF
# Mission

$USER_MISSION

## Goals

$GOALS_LIST

## Non-Goals

$NONGOALS_LIST

## Changelog

One-line dated entries for every scope pivot. Format: `YYYY-MM-DD: <one-line pivot description + link to amendment file if applicable>`. Append entries in chronological order. Never rewrite history here.

<!-- append YYYY-MM-DD entries below this line -->
MISSION_EOF

      stamp_changelog
      printf 'Mission defined! Your MISSION.md has been updated.\n'
    else
      printf 'No mission entered. You can edit MISSION.md manually later.\n'
      stamp_changelog
    fi
  else
    stamp_changelog
  fi
else
  stamp_changelog
fi

if [ -x "$ROOT/verify.sh" ]; then
  printf '\n'
  "$ROOT/verify.sh" --quick || true
fi

if [ -f "$ROOT/docs/WORKFLOW.md" ]; then
  NEXT_READ="$ROOT/docs/WORKFLOW.md"
elif [ -f "$ROOT/.osc/plans/WORKFLOW.md" ]; then
  NEXT_READ="$ROOT/.osc/plans/WORKFLOW.md"
else
  NEXT_READ="$ROOT/.osc/RULES.md"
fi
printf '\nBootstrap complete.\nRead: %s\n' "$NEXT_READ"
