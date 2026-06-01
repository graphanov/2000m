# 2000m 🎿

A tiny retro-slope benchmark for Rust game reproduction.

This repository is the neutral benchmark harness and judge — not a finished
game, not a game solution, and not a general intelligence scorecard. Build the
skier elsewhere; this repo runs the mountain.

The skier can be playful. The scoreboard is not: rank comes from deterministic
protocol checks. The repo now has three course markers: **v0 / 16 mechanical ACs**
for the baseline harness, **v1 / 28 ACs with quality scoring** in [`v1/`](v1/),
and a **v2 workflow-resilience design draft** in [`v2/`](v2/).

## The run, in one screen

| Trail stop | What happens |
| --- | --- |
| Model | Works in a separate produced-game repo and records each generation. |
| Rust game | Produces a SkiFree-inspired homage with original assets. |
| `2000m.json` | Tells the judge how to launch the headless driver. |
| Driver protocol | Implements deterministic JSON-line [`v0`](protocol/2000m.driver.v0.md) or [`v1`](v1/2000m.driver.v1.md). |
| Conformance suite | Runs seeded checks and records AC verdicts. |
| Leaderboard | Ranks by track-labeled results; human feel stays separate. |

## What the mountain scores

The baseline v0 rank is the **AC-pass trajectory across attempts/generations**.
Before those ACs are trusted, the conformance suite checks seed determinism. The
v0 suite evaluates **16 mechanical** acceptance criteria, from basic skier state
through 2000m monster behavior.

The v1 course, under [`v1/`](v1/), expands the climb to **28 ACs** across core
mechanics, challenge depth, polish, and optimization. It adds quality scoring so
"barely passes" and stronger implementations can be reported with more nuance,
but each v1 result must keep measured, host-bound, driver-reported, and
probe-only signals clearly labeled.

The v2 design draft, under [`v2/`](v2/), keeps the benchmark independent while
adding a workflow-resilience track around interruption recovery, feedback use,
stop-condition correctness, and replayable evidence. v2 is not implemented yet
and does not privilege any workflow framework.

One protocol `step` is one deterministic logic tick. The stable scorecard keeps
manual play, networking, random OS state, and human visual taste out of
mechanical rank. v1 currently contains some host wall-clock timing probes and
full-window driver-reported performance telemetry; those fields are evidence
sources to label, not portable independent performance proof.
A produced game can be charming; the rank still comes from reproducible protocol
checks.

## Off the scoreboard

Keep these limits in view:

- 2000m is not a general intelligence scorecard or broad model ranking.
- This repo is the harness/judge, not a finished game, game solution, or
  reference game.
- Human feel is **operator taste, not score**; it never changes mechanical rank.
- Host wall-clock timing, driver-reported performance telemetry, and visual
  event strings may appear in v1 evidence only when explicitly labeled by
  source; they are not portable independent proof by themselves.
- Manual play, network access, and random OS state are not scored.
- No copyrighted SkiFree binaries, sprites, sounds, extracted assets, or replica
  visuals belong in this repo or produced-game submissions.
- Produced games should use original homage assets only.

## Entering a run

A model enters by producing a Rust game in a separate repository or directory.
That produced game must include:

1. a root [`2000m.json`](protocol/2000m.json.schema.json) manifest;
2. a headless driver command declared by that manifest;
3. a driver process that implements the selected protocol version;
4. original SkiFree-inspired homage assets, not copied or extracted SkiFree
   materials.

The scorer launches the manifest command with the produced-game directory as the
working directory and communicates only through newline-delimited JSON on
stdin/stdout.

Example v0 manifest:

```json
{
  "protocolVersion": "2000m.driver.v0",
  "driver": { "command": "cargo", "args": ["run", "--quiet", "--bin", "driver"] },
  "language": "rust"
}
```

For v1 entries, use `"protocolVersion": "2000m.driver.v1"` and implement the
[`v1` driver protocol](v1/2000m.driver.v1.md).

Use [`RULES.md`](RULES.md) as the rules checkpoint for entry, brutal-mode
constraints, scoring, human-feel notes, and asset boundaries.

## Run the judge

Build the included v0 smoke-test stub:

```bash
cd stub-driver
cargo build
cd ..
```

Run the v0 scorer against any produced-game directory containing `2000m.json`:

```bash
cargo run --manifest-path conformance/Cargo.toml -- stub-driver
```

Write machine JSON to a file while printing the human summary:

```bash
cargo run --manifest-path conformance/Cargo.toml -- stub-driver --json-out /tmp/2000m-stub.json
```

The included [`stub-driver/`](stub-driver/) is intentionally minimal. A healthy
harness run should report a partial pass count, not 16/16. If the stub ever
passes all 16 ACs, the stub or suite is wrong.

Build the v1 scorer:

```bash
cargo build --manifest-path v1/conformance/Cargo.toml
```

Run it against a v1 produced-game directory:

```bash
cargo run --manifest-path v1/conformance/Cargo.toml -- path/to/produced-game
```

## Trail map

| Path | Checkpoint |
| --- | --- |
| [`RULES.md`](RULES.md) | Rules checkpoint for entry, scoring, human feel, and asset boundaries. |
| [`protocol/2000m.driver.v0.md`](protocol/2000m.driver.v0.md) | Baseline v0 JSON-line driver protocol. |
| [`protocol/2000m.json.schema.json`](protocol/2000m.json.schema.json) | Manifest checkpoint for produced-game launch metadata. |
| [`conformance/`](conformance/) | v0 judge/scorer with 16 mechanical ACs. |
| [`v1/`](v1/) | v1 protocol, 28-AC spec, quality-scored conformance suite, and weak v1 stub smoke. |
| [`v2/`](v2/) | Design draft for a workflow-resilience track; no v2 scorer or harness yet. |
| [`stub-driver/`](stub-driver/) | Smoke-test skier that shows the v0 harness path works while failing most ACs. |
| [`results/README.md`](results/README.md) | Scorecard format for benchmark rows. |
| [`results/leaderboard.md`](results/leaderboard.md) | Rendered mechanical scoreboard. |

## Scoreboard and human feel

Results live in [`results/`](results/): [`results/README.md`](results/README.md)
defines the row format, and [`results/leaderboard.md`](results/leaderboard.md)
renders the public scoreboard.

Mechanical columns determine rank for their stated track. The human-feel column
is useful context after the operator plays a final visual build, but it is
**operator taste, not score** and is never blended into ordering, pass counts,
quality scoring, or AC-pass trajectory.

## Asset boundary

2000m is SkiFree-inspired, but submissions must be original homages. Do not
include copyrighted SkiFree binaries, sprites, sounds, extracted assets, or
replica graphics in this repository or in produced-game submissions.

Markdown-native trail markers are welcome. Copied game assets are not.
