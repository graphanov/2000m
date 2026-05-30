# 2000m Benchmark Rules

2000m is a benchmark for one narrow question: which LLM best drives the Open Scaffold evolve loop
to reproduce a SkiFree-inspired game in Rust. It is not a general intelligence benchmark.

## How a model enters

1. Create a separate produced-game repository for the model, for example `2000m-<model>`.
2. Build a Rust game/homage there. Do not put produced-game source into this benchmark repo.
3. Ship a headless driver that implements `protocol/2000m.driver.v0.md`.
4. Add a root `2000m.json` manifest that validates against `protocol/2000m.json.schema.json` and
   declares how to launch the driver.
5. Run the conformance suite from this repo against the produced-game directory.
6. Record each evolve generation's mechanical AC pass count and per-AC verdicts in `results/`.

The conformance suite is the scorer. It drives the produced game only by spawning the manifest's
subprocess driver and sending JSON-line protocol commands over stdin/stdout.

## Mechanical score

The ranked score is objective and reproducible:

- seed-determinism must hold for the run to be trusted;
- AC1–AC16 are checked mechanically through the driver protocol;
- the primary metric is the AC-pass trajectory across evolve generations;
- final pass count, generations-to-playable, generations-to-yeti, monotonicity, and generation
  efficiency may be reported as mechanical columns.

No wall-clock mechanics, networking, random OS state, or manual play are part of the mechanical
score.

## Human feel track

The owner/operator may play the final visual build and record a clearly labeled human-feel note.
That note is operator taste, not benchmark score. It is displayed next to the mechanical rank and
is never blended into ordering or pass counts.

## Asset and legal boundary

Do not include copyrighted SkiFree binaries, sprites, sounds, or extracted assets in this repo or
in produced-game submissions. Produced games should use original assets and should be framed as
SkiFree-inspired homages.

## Closing a run

A run record should include:

- model name and produced-game repository path;
- generation cap used;
- AC pass-count trajectory;
- final per-AC verdict JSON from the conformance suite;
- mechanical rank fields;
- optional separate human-feel note.
