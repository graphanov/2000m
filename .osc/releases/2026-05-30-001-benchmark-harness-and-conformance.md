# Release / Evidence Note: 001-benchmark-harness-and-conformance

## Summary

Built the 2000m benchmark harness: the `2000m.driver.v0` JSON-line driver protocol spec, a Rust
conformance scorer that drives a produced game purely through its `2000m.json` subprocess driver
and checks seed-determinism plus 16 acceptance criteria, a minimal Rust stub driver, entry RULES,
an honest-scope README, and results/leaderboard scaffolding with a separate human-feel column.
The harness was built autonomously by OMX (`omx --madmax --xhigh`) on branch
`build/harness-and-conformance`; Hermes reviewed and verified every gate independently before
closeout.

## Traceability

- Plan: `.osc/plans/done/001-benchmark-harness-and-conformance.md` (after closeout).
- Branch / PR: branch `build/harness-and-conformance`; PR pending owner gate.
- Builder: OMX (Codex / GPT-5.5 harness) full-autonomy on branch; Hermes postflight + gates.
- Design support: `.osc/research/2026-05-30-design-spike.md`.

## Verification

Run by Hermes independently of the builder's self-report:

- `cargo build` in `stub-driver/` — PASS (clean).
- `cargo build` in `conformance/` — PASS (clean).
- Conformance scorer vs stub driver — **determinism PASS** (107 canonical GameState snapshots
  matched byte-for-byte across two independent driver processes) and **3/16 ACs passed**
  (AC1/AC2/AC3 pass; AC4–AC16 fail with specific mechanical reasons). The partial pass count
  proves the suite discriminates rather than rubber-stamping.
- **Brutal-mode passability proof** — a throwaway correct reference Rust driver (built in `/tmp`,
  never committed, never a contender) scored **16/16 with determinism PASS** on the hint-free
  suite, reaching every mechanic through real navigation (e.g. "navigated into a tree/stump and
  crashed", "navigated onto a ramp", "monster converged from distance 26.5 to 1.5", "skier
  eaten"). This proves the no-hint suite is hard but fair, not impossible. Reference torn down
  after the proof.
- Reproducibility — running the scorer twice produced byte-identical machine JSON.
- Minimality — stub `src/main.rs` always emits `"monster":null` with no collision/ramp/monster
  logic; the real engineering is the conformance scorer. OMX did NOT build a full reference game.
- No copyrighted assets — repo scan for `.png/.bmp/.wav/.mp3/.gif/.exe/.ico` found none.
- README honest scope — states "does not measure general intelligence," Rust target, separate
  human-feel track, and no copyrighted assets.
- `./verify.sh --standard` — 6 pass / 0 fail / 0 warn.
- Boundary respect — OMX did not modify MISSION/committed plans, did not close/commit/push.

## Owner-directed brutal-mode hardening (no hints)

OMX's first build exposed isolated mechanics to the suite via `init.config.scenario` hints
(`collision-tree`, `ramp`, `monster-contact`) and a `slopeWidthM` wrap hint. The owner rejected
any hinting: no version of the benchmark should tell a model how to reach a mechanic. Hermes
removed all config hints and rewrote the 7 hint-using checks (AC4 wrap, AC6/AC7 collision,
AC10/AC11 ramp, AC14/AC15 monster contact) to reach each mechanic through ordinary play — the
suite reads the seeded obstacle/monster coordinates the game reports and navigates the skier into
them. The protocol spec now documents an explicit coordinate convention plus a "brutal-mode
scoring (no hints)" contract; RULES.md adds a "No hints (brutal mode)" section. Discrimination is
preserved (correct reference 16/16, stub 3/16) and determinism still holds.

## Outcome

Shipped the 2000m benchmark harness: protocol spec, Rust conformance scorer (the judge), minimal
Rust stub driver, RULES, honest-scope README, and results scaffolding with a labeled human-feel
column never blended into the rank. The suite verifiably discriminates (3/16 on the stub) and is
deterministic and reproducible. Pending owner review and merge gate; no game solution and no
copyrighted assets shipped.

## Follow-up

- Owner gate: review and merge the PR for `build/harness-and-conformance`.
- After merge: create `graphanov/2000m-codex-gpt55`, seed the 2000m task into `osc evolve`, run
  Codex/GPT-5.5 for up to 8 generations, capture the real Gen1→Gen8 AC-pass trajectory.
