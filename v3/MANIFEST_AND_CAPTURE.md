# 2000m v3 manifest and capture contract

Status: draft design packet. This document proposes a v3 manifest/capture contract and does not implement schema validation yet.

## Problem

Earlier 2000m versions allowed high-scoring headless drivers with no native visual capture or playable product surface. That made mechanical conformance look stronger than the actual game artifact.

v3 should require the produced artifact to declare three surfaces:

1. a deterministic headless driver for mechanical scoring;
2. deterministic visual/replay capture for visual/product scoring;
3. a playable surface for owner/human inspection.

## Proposed manifest shape

```json
{
  "protocolVersion": "2000m.driver.v3",
  "language": "rust",
  "driver": {
    "command": "cargo",
    "args": ["run", "--quiet", "--bin", "driver"]
  },
  "capture": {
    "command": "cargo",
    "args": ["run", "--quiet", "--bin", "capture", "--", "--seed", "{seed}", "--window", "{window}", "--out", "{out}"],
    "outputs": {
      "screenshot": "{out}/screenshot.png",
      "gif": "{out}/replay.gif",
      "frames": "{out}/frames.json"
    }
  },
  "playable": {
    "command": "cargo",
    "args": ["run", "--quiet", "--bin", "game"],
    "urlOrPath": "target/2000m/index.html"
  },
  "assets": {
    "license": "original-homage-assets-only",
    "notes": "No copied or extracted SkiFree assets."
  }
}
```

## Manifest rules

- `driver` is required for mechanical scoring.
- `capture` is required for ranked visual/product scoring.
- `playable` is required for ranked product/human-feel review.
- `capture.args` may use placeholders: `{seed}`, `{window}`, `{out}`.
- Capture outputs must be deterministic for the same artifact, seed, window, and command.
- A missing `capture` block does not block mechanical scoring, but it blocks ranked visual/product claims.
- A missing `playable` block does not block mechanical scoring, but it blocks playable-product claims.
- Public result records must not include local private absolute paths.

## Driver surface

The v3 driver keeps the JSON-line command pattern:

- `init`
- `step`
- `state`
- `reset`
- `profile`
- `replay`
- `challenge`

v3 may extend `challenge` with hidden/public challenge sets, but emitted GameState must stay schema-validated and deterministic.

## Capture surface

The capture command should produce a directory with:

```text
screenshot.png
replay.gif or replay.webm
frames.json
rubric-metadata.json
```

Required `frames.json` fields:

```json
{
  "schemaVersion": "2000m.capture.frames.v1",
  "seed": 1101,
  "window": "early-game",
  "frameCount": 120,
  "fps": 30,
  "inputSequenceRef": "inputs.json",
  "stateChecksum": "sha256:...",
  "frameChecksum": "sha256:...",
  "events": ["..."],
  "warnings": []
}
```

Capture commands must be allowed to fail honestly. If a capture fails, the result record must include the command, stdout/stderr refs, and failure reason.

## Standard capture windows

A v3 campaign should freeze the capture windows before live results:

| Window | Purpose |
|---|---|
| `early-game` | Is the basic skier/slope/obstacle presentation readable? |
| `mid-run-obstacle-field` | Does the game handle obstacle density without unreadable clutter? |
| `ramp-style-sequence` | Are ramps, airborne/landing, and style events visible? |
| `monster-pressure` | Does the monster appear and read as a threat? |
| `post-feedback-rerun` | Did the final attempt improve or regress visible quality after feedback? |

Default visual seeds should be fixed in the campaign file, not chosen after seeing outputs.

## Visual metrics

Human/blind review remains important, but simple metrics can catch extreme failures. Candidate metrics:

- object spread across horizontal buckets;
- visible obstacle type variety;
- skier visibility / contrast against background;
- frame-to-frame continuity checksum;
- clutter density near the skier;
- monster visibility when expected;
- capture determinism across two runs.

These metrics should be diagnostic or component inputs, not a substitute for human product judgment.

## Blind review package

For paired campaigns, the benchmark should generate:

```text
visual-package/
  blind-contact-sheet.png
  blind-label-map.json
  screenshots/
  gifs/
  frames/
  rubric.md
  rubric-record.json
```

`blind-label-map.json` must be sealed until after ratings. The final result should record both the blind ratings and the decoded lane mapping.

## Blocking conditions

Visual/product track is rank-blocked when:

- capture command is missing;
- capture command fails and no valid rerun exists;
- screenshots/GIFs are missing or non-deterministic;
- fixed seeds/windows were changed after viewing live results;
- blind label map was inspected before rating;
- capture artifacts include private/local refs in public records;
- copied/extracted SkiFree assets are used.

## Relationship to mechanical scoring

Mechanical scoring must not depend on screenshots or human taste. Visual scoring must not rescue a mechanically invalid artifact. They are separate tracks.
