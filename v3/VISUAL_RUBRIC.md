# 2000m v3 visual/product rubric draft

Status: executable-spec draft. This rubric defines visual/product evidence expectations and blockers; it does not create public proof or run a pilot.

## Visual track boundary

Visual/product quality answers whether the entrant produced a coherent visible/playable SkiFree-inspired artifact. It is separate from mechanical correctness and workflow resilience.

A missing native capture or playable surface sets:

```text
visual.ranked = false
visual.blockReason = missing-native-capture-or-playable-surface
```

A neutral proxy renderer may support private inspection but cannot substitute for submitted native capture in public visual ranking.

## Required package

Every ranked visual package must validate against `v3/visual-package.schema.json` and include:

- blind contact sheet;
- sealed blind label map;
- fixed-seed screenshots;
- deterministic GIF or WebM/replay artifacts;
- per-window frame metadata;
- rubric record;
- artifact digest captured before rendering;
- capture command and checksums for each window.

## Capture windows

| Window | Required evidence | Purpose |
|---|---|---|
| `early-game` | screenshot + replay + frame metadata | Basic skier/slope/readability. |
| `mid-run-obstacle-field` | screenshot + replay + frame metadata | Obstacle density without unreadable clutter. |
| `ramp-style-sequence` | screenshot + replay + frame metadata | Ramps, airborne/landing, style readability. |
| `monster-pressure` | screenshot + replay + frame metadata | Monster appearance and threat readability. |
| `post-feedback-rerun` | screenshot + replay + frame metadata | Visual regression/improvement after feedback. |

Seeds and windows must be frozen in the campaign before live outputs are seen.

## Draft rubric dimensions

Each dimension is scored 0-5 by reviewer or deterministic diagnostic where stated. Do not use these numbers to rescue a mechanically invalid artifact.

| Dimension | Weight | Evidence | 0 means | 5 means |
|---|---:|---|---|---|
| Skier readability | 20% | screenshots/replay | skier cannot be found | skier is consistently clear |
| Slope and motion clarity | 15% | replay/frames | motion unreadable or static | motion reads as downhill skiing |
| Obstacle readability | 15% | screenshots/frames | clutter/objects impossible to parse | obstacles are distinct and navigable |
| Ramp/style communication | 10% | ramp window replay | ramp/style invisible | state changes are visually obvious |
| Monster pressure | 10% | monster window | monster absent/unreadable | monster reads as active threat |
| Visual coherence | 15% | contact sheet | inconsistent/noisy/non-game look | coherent homage style with original assets |
| Capture determinism | 10% | checksums | repeated capture differs | repeated capture matches |
| Playable surface | 5% | playable command/ref | no playable surface | surface launches from documented command/ref |

## Blocking rules

The visual track is rank-blocked when:

- capture command is missing or fails without valid rerun;
- screenshot/replay/frame metadata refs are missing;
- frame metadata omits seed, window, frame count, FPS, input ref, state checksum, or frame checksum;
- fixed seeds/windows changed after live result inspection;
- blind label map was opened before rating;
- package/public record contains private local refs;
- asset attestation fails or copied/extracted SkiFree assets are found.

## Owner taste vs public preference

Owner taste may be recorded as a separate note. It is not a public human-preference score unless the campaign predeclares reviewer count, blinding, form, weights, and tie handling.
