# 2000m v3 mechanical AC table draft

Status: executable-spec draft. This table defines the first concrete Track 1 acceptance-criteria shape for future scorer implementation. It is not yet a scorer and does not change v0/v1/v2 results.

## Scoring boundary

Mechanical correctness answers only whether the artifact obeys the deterministic game-logic driver contract. It does not decide whether the game looks good, is fun, or demonstrates workflow value.

Draft denominator: 24 binary ACs plus diagnostic quality fields. Hidden challenges may be used only if their policy is frozen before live results.

## AC table

| ID | Area | Public fixture | Hidden fixture | Pass threshold | Notes |
|---|---|---:|---:|---|---|
| M01 | Manifest/protocol | yes | no | manifest validates as `2000m.v3.manifest.v1`; driver declares `2000m.driver.v3` | Capture/playable may block visual rank but not mechanical. |
| M02 | Init determinism | yes | yes | same seed/config emits byte-stable canonical state checksum | Booleans are invalid seeds. |
| M03 | Reset determinism | yes | yes | reset returns to the same state as init for the same seed/config | |
| M04 | Step determinism | yes | yes | same input stream emits identical state/event/checksum sequence | |
| M05 | State idempotence | yes | no | repeated `state` calls do not advance ticks | |
| M06 | Schema validity | yes | yes | every emitted state validates; unsupported enum values fail | Covers obstacle and event poison. |
| M07 | Obstacle generation | yes | yes | fixed seeds produce stable obstacle fields with allowed types | |
| M08 | Skier movement | yes | no | left/right/neutral input changes position and velocity within bounds | |
| M09 | Collision correctness | yes | yes | collisions with solid obstacles crash or recover as specified | |
| M10 | Recovery behavior | yes | no | crashed skier can recover through legal steps | |
| M11 | Ramp entry | yes | yes | ramp collision enters airborne state rather than ordinary crash | |
| M12 | Airborne/landing | yes | yes | airborne duration and landing transitions are deterministic | |
| M13 | Style scoring | yes | no | style gain/loss events update score consistently | |
| M14 | Monster spawn | yes | yes | monster appears under frozen trigger conditions | |
| M15 | Monster pursuit | yes | yes | monster moves toward skier under pressure window | |
| M16 | Monster contact/flee | yes | yes | contact/flee events are emitted and deterministic | |
| M17 | Replay checksum | yes | yes | `replay` reproduces final state and checksum list | |
| M18 | Public challenge | yes | no | public challenge set can be executed by ID | |
| M19 | Hidden challenge isolation | no | yes | hidden challenge inputs are not required in entrant manifest | Policy must be predeclared. |
| M20 | Regression stability | yes | yes | final artifact reruns pass the same AC set twice | |
| M21 | Error semantics | yes | no | invalid requests return structured errors, not malformed output | |
| M22 | No scorer mutation | yes | yes | result record shows frozen scorer/protocol were not mutated | Schema-level guard only in this PR. |
| M23 | No private setup hints | yes | yes | driver passes with public config only | |
| M24 | Mechanical/visual separation | yes | no | missing native capture does not alter mechanical pass count | Visual rank blocker is recorded separately. |

## Draft composite

If a mechanical convenience composite is rendered later, it must be labeled `mechanical.compositeScore` and computed only from mechanical ACs. It must not include visual taste, workflow resilience, evidence volume, generations-to-finish, or framework usage.

Draft formula for future implementation:

```text
mechanical.compositeScore = 100 * passed_mechanical_acs / total_ranked_mechanical_acs
```

Skipped or untestable ranked ACs count as zero unless the frozen campaign declares the AC as probe-only before live results.

## Invalid data handling

Unsupported obstacle/event enums, malformed state, boolean seeds, private setup dependencies, and scorer/protocol mutation are hard mechanical failures or rank blockers. The future scorer must report the exact reason rather than silently repairing entrant output.
