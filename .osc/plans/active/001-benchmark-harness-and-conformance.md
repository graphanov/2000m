# Plan: 001-benchmark-harness-and-conformance

## Status

active

## Context

2000m is a Rust SkiFree reproduction benchmark. Before any model can compete, the benchmark
itself must exist: a deterministic, language-agnostic way to drive a produced Rust game
headlessly, and a mechanical conformance suite that scores it against 16 acceptance criteria.
This plan builds that harness. It deliberately ships NO full reference game — only a minimal
stub driver to prove the harness runs — so the suite cannot be shaped to pass one reference
implementation. Full games come only from contending models later, through `osc evolve`.

Grounding sources (studied, not copied): ski.ihoc.net (original author, confirms horizontal
slope wrap-around and the 2000m monster), en.wikipedia.org/wiki/SkiFree (modes, HUD, F-key,
monster at 2000m), github.com/basicallydan/skifree.js (MIT, confirms feasibility + seeded tree
fields, collision reactions, F-key boost, ramps, monster eats-then-flees).

## Goal

Build the 2000m benchmark harness and conformance suite in this repository:

1. A versioned JSON-line **driver protocol** spec (`2000m.driver.v0`): a produced Rust game
   ships a headless driver binary that reads newline-delimited JSON commands on stdin and writes
   newline-delimited JSON responses on stdout. Commands: `init` (seed), `step` (input), `state`,
   `reset` (seed). Responses carry a `GameState` (skier x/y/speed/mode, distanceM, style,
   obstacles[], monster|null, gameOver, tick). A `2000m.json` manifest at a produced game's root
   declares how to launch its driver.
2. A **conformance runner** (the scorer) that, given a produced game directory, reads its
   `2000m.json`, spawns the Rust driver as a subprocess, and asserts: (a) seed-determinism —
   same seed + same input sequence yields byte-identical GameState streams across two separate
   driver processes; (b) each of the 16 acceptance criteria below. It emits a machine-readable
   result (pass count, per-AC verdicts) and a human summary.
3. A **minimal stub Rust driver** under `stub-driver/` that implements the protocol just well
   enough to pass roughly AC1–AC3, proving the harness runs end to end. It is NOT a full game.
4. **Rules**, an honest-scope **README**, and **results/** leaderboard scaffolding (small JSON +
   rendered markdown table) with a separate, clearly-labeled human-feel column that never blends
   into the mechanical rank.

### The 16 acceptance criteria the conformance suite must check
- AC1: skier entity with position state (x/y/speed/mode) exists after `init`.
- AC2: steering (-1/+1) moves skier.x deterministically.
- AC3: slope scrolls — distanceM strictly increases while skiing.
- AC4: slope wraps horizontally — skier crossing an edge re-enters the opposite side.
- AC5: seeded obstacle field — same seed ⇒ identical obstacle stream; different seed ⇒ different.
- AC6: tree/stump collision ⇒ skier.mode='crashed', forward motion halts.
- AC7: crash recovery — after N ticks skier.mode returns to 'skiing'.
- AC8: speed increases going straight downhill, up to a cap.
- AC9: F-key boost (boost:true) exceeds the AC8 cap.
- AC10: ramp ⇒ skier.mode='airborne' then back to 'skiing' on landing.
- AC11: style score accrues on good landings/tricks; crash deducts.
- AC12: monster becomes non-null at distanceM >= 2000.
- AC13: monster pursues — monster x/y converges toward skier over ticks.
- AC14: monster contact ⇒ skier.mode='eaten', gameOver=true.
- AC15: monster flees after eating — post-eat monster.mode='fleeing', moves away.
- AC16: restart/reset resets distance/style/monster; seed reproducible.

## Constraints / Out of scope

- Produced games are **Rust**; the conformance runner may be Rust or TypeScript but must spawn
  the produced driver as a subprocess (language-agnostic at the process boundary).
- Ship NO full reference SkiFree game. Stub driver only.
- No copyrighted SkiFree assets (binaries, sprites, sounds). Original homage only.
- Human "feel" must never enter the mechanical score; leaderboard keeps it in a separate column.
- The mechanical core judged by the suite must be fixed-tick deterministic (1 `step` = 1 logic
  tick, no wall-clock dependence).
- No network access during conformance/scoring runs.
- Honest scope in the README: measures LLM-driving-Open-Scaffold on Rust SkiFree reproduction,
  not general intelligence.

## Files to touch

- `protocol/2000m.driver.v0.md` — the driver protocol spec.
- `protocol/2000m.json.schema.json` — manifest schema for produced games.
- `conformance/` — the conformance runner (scorer) + the 16 AC checks + determinism check.
- `stub-driver/` — minimal Rust driver proving the harness runs (passes ~AC1–AC3).
- `results/` — leaderboard scaffolding (JSON + rendered markdown, separate human-feel column).
- `RULES.md` — how a model enters: produce a Rust game with a conforming driver + manifest.
- `README.md` — honest-scope overview, how to run the conformance suite, what is/ isn't proven.

## Acceptance criteria

- [ ] The driver protocol spec exists and fully defines `init`/`step`/`state`/`reset` and the GameState shape.
- [ ] The conformance runner spawns a driver subprocess from a `2000m.json` manifest and runs all 16 AC checks plus the determinism check.
- [ ] The minimal stub Rust driver builds (`cargo build`) and the conformance runner reports a real pass count for it (expected: passes the lowest few ACs, fails the rest — proving the suite discriminates).
- [ ] Running the suite twice on the same stub yields identical results (reproducible).
- [ ] RULES.md and README.md state the honest scope and the entry process; no copyrighted assets are present.
- [ ] results/ contains leaderboard scaffolding with a mechanical-rank table and a separate, clearly-labeled human-feel column.

## Verification steps

1. `cargo build` in `stub-driver/` succeeds.
2. Run the conformance runner against the stub driver; confirm it emits per-AC verdicts and a pass count, and that the count is partial (not all 16) — proving the suite actually discriminates.
3. Run the conformance runner twice; confirm identical output (determinism/reproducibility).
4. `./verify.sh --standard` passes (Open Scaffold work-record integrity).
5. Grep the repo for any copyrighted SkiFree asset filenames; confirm none present.

## Open questions

- None outstanding. Conformance runner language (Rust vs TypeScript) is left to the builder, as
  long as it spawns the produced driver as a subprocess and stays deterministic.
