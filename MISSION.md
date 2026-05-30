# Mission

**2000m** is a Rust SkiFree reproduction benchmark. It measures how well an LLM, driving the
Open Scaffold closed evolutionary loop (`osc evolve`), can iteratively rebuild a faithful,
playable homage of the 1991 game SkiFree (Chris Pirih, Microsoft) — graded by a mechanical
acceptance-criteria trajectory across generations.

The name is the distance at which SkiFree's Abominable Snow Monster spawns: the game's most
iconic and most testable moment.

## Goals

- Define a deterministic, seeded, **language-agnostic JSON-line driver protocol** that a Rust
  game binary implements, so the benchmark can drive any produced game headlessly.
- Ship a mechanical **conformance suite** that spawns a produced game's driver as a subprocess
  and asserts seed-determinism plus 16 acceptance criteria, easy → hard, up to the yeti.
- Score the **iterative AC-pass trajectory** across evolve generations — the rank is objective
  and reproducible.
- Keep human "feel" as a **separate, clearly-labeled track** (owner plays the final WASM demo),
  reported next to the rank but never blended into it.
- Stay an honest benchmark: this measures *which LLM best drives Open Scaffold to reproduce
  SkiFree in Rust*, not general intelligence.

## Non-Goals

- Not a general LLM intelligence benchmark.
- The benchmark repo ships no full reference game and no produced-game solutions — only the
  protocol, conformance suite, rules, a minimal stub driver, and the leaderboard.
- No copyrighted SkiFree binaries, sprites, or sounds. Original assets only; this is a
  SkiFree-*inspired* homage.
- Human taste never enters the mechanical score.

## Changelog

<!-- append YYYY-MM-DD entries below this line -->
- 2026-05-30: closed 001-benchmark-harness-and-conformance — Build 2000m benchmark harness, conformance suite, and stub driver
