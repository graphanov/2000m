# 2000m

2000m is a Rust SkiFree reproduction benchmark. It measures which LLM best drives the Open
Scaffold evolve loop to rebuild a faithful, playable SkiFree-inspired homage **in Rust**. It does
not measure general intelligence.

This repository is the benchmark harness, not a game solution. It ships:

- a deterministic JSON-line subprocess driver protocol;
- a conformance suite that scores produced games through that protocol;
- a minimal Rust stub driver that proves the harness runs end to end while intentionally failing
  most ACs;
- leaderboard scaffolding for mechanical results plus a separate human-feel note.

## What is scored

The mechanical rank is the AC-pass trajectory across evolve generations. The scorer checks
seed-determinism and 16 mechanical acceptance criteria, from basic skier state through the 2000m
monster behavior. The conformance suite is deterministic: one protocol `step` equals one logic
tick, with no wall-clock dependence.

Human feel is a separate, clearly labeled track: `human feel (operator taste, not score)`. It may
be useful context after the owner plays a final visual build, but it is never blended into the
mechanical rank.

## What is not scored

- This is not a general LLM intelligence benchmark.
- This repo does not ship a full reference SkiFree game.
- Visual taste does not affect the mechanical score.
- No copyrighted SkiFree binaries, sprites, sounds, or extracted assets belong here. Produced
  games must use original homage assets only.

## Produced-game contract

A model enters by producing a Rust game in a separate repository/directory. That produced game
must include a root `2000m.json` manifest and a headless driver binary/process that implements
`protocol/2000m.driver.v0.md`.

Example manifest:

```json
{
  "protocolVersion": "2000m.driver.v0",
  "driver": { "command": "cargo", "args": ["run", "--quiet", "--bin", "driver"] },
  "language": "rust"
}
```

The scorer launches the command with the produced-game directory as the working directory and
drives it only through newline-delimited JSON on stdin/stdout.

## Run the conformance suite

Build the included smoke-test stub:

```bash
cd stub-driver
cargo build
cd ..
```

Run the scorer against any produced-game directory containing `2000m.json`:

```bash
cargo run --manifest-path conformance/Cargo.toml -- stub-driver
```

Write machine JSON to a file while printing the human summary:

```bash
cargo run --manifest-path conformance/Cargo.toml -- stub-driver --json-out /tmp/2000m-stub.json
```

The included `stub-driver/` is intentionally minimal. A healthy harness run should report a
partial pass count, not 16/16. If the stub ever passes all 16 ACs, the stub or suite is wrong.

## Repository map

- `protocol/2000m.driver.v0.md` — driver protocol spec.
- `protocol/2000m.json.schema.json` — produced-game manifest schema.
- `conformance/` — Rust conformance runner/scorer.
- `stub-driver/` — minimal Rust protocol stub for harness smoke tests.
- `results/` — JSON result format and rendered leaderboard scaffolding.
- `RULES.md` — model entry and scoring rules.
