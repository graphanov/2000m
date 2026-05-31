# 2000m v1: Neutral Benchmark Hardening Design

## Status

This document describes the v1 benchmark design and the credibility boundaries that keep the judge neutral. It is **not** evidence that any workflow, scaffold, runtime, model, or prompt style wins. Comparative claims require real contender runs, committed evidence, and the same judge applied to every entrant.

## Problem Statement

The v0 benchmark established a deterministic 16-AC harness for a SkiFree-inspired Rust game. It is a useful baseline, but it leaves limited room to distinguish:

1. a minimal implementation from a robust implementation;
2. mechanics that work only on the happy path from mechanics that survive stress cases;
3. short-run determinism from long-run replay stability;
4. mechanical pass/fail from diagnostic quality signals.

v1 expands the suite so a contender has more room to improve across generations. The goal is a harder and more transparent benchmark, not a preloaded conclusion about any development method.

## Design Principles for v1

### 1. Quality gradients, not only binary checks

Every AC may return both:

- `pass`: whether the mechanical requirement passed;
- `quality`: a diagnostic 0-100 score attached to that AC.

Quality is useful for comparing two implementations that pass the same binary ACs, but the provenance of each quality subscore matters. A measured geometry/replay result is stronger than a constant default or a driver-reported telemetry field.

### 2. Stress tests and edge cases

v1 adds checks for harder conditions:

- high-speed collision tunneling;
- dense obstacle fields;
- monster pursuit while the skier actively evades;
- crash/recovery repetition;
- 10,000-tick determinism;
- deterministic replay round-trips;
- visual-polish telemetry as probe-only evidence.

### 3. Reproducibility before polish

The scorer should make deterministic, protocol-observable results easy to trust. Any host-bound, driver-reported, or probe-only result must be labeled as such in evidence. v1 currently contains a mix of sources; see [Performance and quality provenance](#performance-and-quality-provenance).

### 4. Track-labeled scoring

v0 and v1 are separate tracks:

- v0: 16 mechanical ACs and AC-pass trajectory across generations;
- v1: 28 ACs, per-AC quality, and standalone composite score from the v1 scorer.

Results must say which track produced the score. A v1 score is not comparable to a v0 score.

## Acceptance Criteria Structure

### Tier 1: Core mechanics, enhanced

AC1-AC16 preserve the v0 mechanic coverage and add quality/detail fields where the v1 scorer can observe stronger behavior:

- skier entity and state;
- steering;
- slope scrolling;
- horizontal wrap;
- seeded obstacle field;
- collision and crash halt;
- crash recovery;
- speed cap;
- boost;
- ramps;
- style scoring;
- monster spawn/pursuit/contact/flee behavior;
- reset reproducibility.

### Tier 2: Stress and long-run behavior

AC17-AC22 extend the core with stress conditions:

- high-speed tunneling prevention;
- dense obstacle performance;
- monster pursuit under evasion;
- long-run determinism;
- crash recovery under load;
- monster spawn timing precision.

### Tier 3: Replay, responsiveness, and polish probes

AC23-AC28 cover later-stage robustness and polish signals:

- input responsiveness;
- collision forgiveness;
- animation smoothness;
- deterministic replay accuracy;
- performance budget;
- visual-polish event telemetry.

AC28 is intentionally probe-only for mechanical pass/fail: headless event strings may inform quality/context, but they do not prove a renderer or human-feel outcome.

## Protocol Changes

### `profile`

Returns driver-collected metrics for a requested tick window:

```json
{"cmd":"profile","window":100}
```

Example response:

```json
{
  "ok": true,
  "metrics": {
    "windowTicks": 100,
    "avgTickNanos": 2300000,
    "maxTickNanos": 8100000,
    "p95TickNanos": 4200000,
    "p99TickNanos": 6800000,
    "totalAllocations": 0,
    "peakMemoryBytes": 45000000,
    "collisionChecksTotal": 1523,
    "collisionHitsTotal": 42
  }
}
```

`profile` fields are driver-reported. They are useful for reproducible telemetry when reported honestly, but they are not independently verified in the same way AC26 replay is. Evidence that relies on these fields must say so.

### `replay`

Returns a compact deterministic replay log for the last N ticks:

```json
{"cmd":"replay","ticks":1000}
```

The scorer replays the returned input sequence in a fresh process and recomputes the checksum. This makes AC26 stronger than simple driver self-attestation.

### `challenge`

Configures stress-test initial conditions that would be impractical to reach only through normal play, such as dense obstacle fields or crash gauntlets.

Challenges are optional. Unsupported challenges remain visible in AC details and skipped ACs count as zero in ranked composite denominators.

## Standalone Composite Score

The canonical v1 standalone composite is the formula implemented in `v1/conformance/src/main.rs`:

```text
pass_rate = passed_non_skipped_acs / total_acs
quality_avg = sum(ac.quality for all acs, including skipped as zero) / total_acs
composite_score = pass_rate * 70.0 + quality_avg * 0.3
```

Important boundaries:

- skipped/untestable ACs stay in the denominator;
- no LOC term is included;
- no convergence-speed bonus is included;
- no external result-repo trajectory field is included;
- no human-feel note is included.

Multi-generation result repositories may report trajectory, generation count, monotonicity, or separate efficiency columns, but those are separate fields layered on top of the standalone scorer output. They are not part of `composite_score` unless a future protocol version explicitly changes the scorer.

## Performance and Quality Provenance

v1 currently uses several evidence sources. They should be reported truthfully:

| Source | Examples | Trust level | How to describe it |
| --- | --- | --- | --- |
| Suite-recomputed deterministic evidence | determinism preflight, AC20, AC26 replay round-trip | strongest | independently recomputed by the scorer |
| Protocol-observed game state | positions, modes, obstacle streams, monster state | strong when deterministic | observed through JSON-line driver state |
| Host wall-clock probe | AC18, AC23, AC25 elapsed `step` timing | host-bound | measures the scorer host plus JSON/IPC overhead; useful diagnostic, not portable proof |
| Driver-reported profile telemetry | AC27 `avgTickNanos`, `p99TickNanos`, allocations, memory | self-reported | reproducible if honest, but not independently recomputed |
| Event-string / visual telemetry | AC28 particles/shake/style events | probe-only | can inform quality/context, not mechanical pass |
| Constant defaults | fixed `performance`/`polish` values in some AC rubrics | weak | rubric placeholders, not measured polish |

The current v1 scorer still contains host-bound and self-reported performance signals. That does not make the suite useless, but public results must not overclaim these fields as portable, independently verified performance proof. A future v1 hardening pass should either replace these with suite-recomputed signals or make them non-ranking diagnostics.

## Neutrality Rules

A neutral judge repo must not pre-register the desired winner. Therefore:

- no mock trajectory should be presented as expected evidence;
- no self-run row should be described as adoption or independent validation;
- no method should be described as winning until real contender evidence exists;
- owner-supervised reference baselines must be labeled as reference baselines;
- public rankings must link to produced-game repos and conformance JSON.

## Implementation Priorities

1. Keep the v1 scorer and docs aligned on the standalone composite formula.
2. Label host-bound and self-reported performance fields in human output and result evidence.
3. Prefer suite-recomputed checks, following the AC26 replay pattern, when a future AC depends on driver-reported claims.
4. Keep visual and human-feel evidence separate from mechanical rank.
5. Require real produced-game evidence before making any comparative claims.

## Success Criteria for v1 Credibility

The v1 benchmark is credible when:

1. docs and scorer publish one scoring formula;
2. timing/performance ACs state their evidence source honestly;
3. skipped ACs cannot inflate rank;
4. visual-polish telemetry remains probe-only unless independently validated;
5. produced-game entries include reproducible conformance JSON;
6. the leaderboard distinguishes reference baselines, owner-supervised runs, and independent submissions.

## Conclusion

v1 turns 2000m from a narrow binary baseline into a harder benchmark with stress checks, replay checks, and quality diagnostics. Its value depends on neutrality: the repo should define the mountain, not the winner. Claims about any workflow, model, runtime, prompt style, or contender strategy belong in separately labeled result evidence after real runs exist.
