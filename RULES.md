# 2000m Benchmark Rules

This is the rules checkpoint for the 2000m trail: how a produced-game entry
joins the run, what the judge measures, and what stays off the scoreboard.

2000m is a benchmark for one narrow question: which LLM best drives the Open
Scaffold evolve loop to reproduce a SkiFree-inspired game in Rust. It is not a
general intelligence scorecard. The benchmark repository remains the neutral
harness/judge, not a finished game or game solution.

## How a model enters

1. Create a separate produced-game repository for the model, for example
   `2000m-<model>`.
2. That produced-game repository should itself be an Open Scaffold work record:
   the whole point of 2000m is to measure how well a model drives the Open
   Scaffold evolve loop, so the model's plans, generations, and evidence live
   there. This benchmark repository stays neutral and ships no framework
   apparatus.
3. Build a Rust game/homage there. Do not put produced-game source into this
   benchmark repo.
4. Ship a headless driver that implements the selected protocol:
   - v0: `protocol/2000m.driver.v0.md`
   - v1: `v1/2000m.driver.v1.md`
5. Add a root `2000m.json` manifest that validates against
   `protocol/2000m.json.schema.json` and declares how to launch the driver.
6. Run the matching conformance suite from this repo against the produced-game
   directory.
7. Record each evolve generation's AC pass count, quality fields when present,
   and per-AC verdicts in `results/`.

The conformance suite is the scorer. It drives the produced game only by
spawning the manifest's subprocess driver and sending JSON-line protocol
commands over stdin/stdout.

## No hints (brutal mode)

The suite gives the produced game no setup hints, scenario flags, or shortcut
keys. It sends an empty `config` on every `init` and reaches each mechanic the
way a human would: it reads the seeded obstacle and monster coordinates the game
itself reports and steers the skier into trees, onto ramps, and into the yeti by
ordinary protocol input. Collisions, ramps, and monster contact must genuinely
work — a produced game cannot pass a mechanic by special-casing a flag, because
no such flag exists.

## Mechanical score

The ranked score is objective and reproducible per track:

- seed-determinism must hold for the run to be trusted;
- v0 checks AC1–AC16 mechanically through the driver protocol;
- v1 checks AC1–AC28 and adds quality scoring across three tiers;
- the primary v0 metric is the AC-pass trajectory across evolve generations;
- v1 may additionally report quality average, efficiency, convergence, and a
  composite score;
- final pass count, generations-to-playable, generations-to-yeti, monotonicity,
  and generation efficiency may be reported as mechanical columns.

No wall-clock mechanics, networking, random OS state, or manual play are part of
the mechanical score.

## Human feel track

The owner/operator may play the final visual build and record a clearly labeled
human-feel note. That note is operator taste, not score. It is displayed next to
the mechanical rank and is never blended into ordering, pass counts, or quality
scores.

## Asset and legal boundary

Do not include copyrighted SkiFree binaries, sprites, sounds, extracted assets,
or replica visuals in this repo or in produced-game submissions. Produced games
should use original homage assets, not copied or extracted SkiFree materials.

## Closing a run

A run record should include:

- model name and produced-game repository path;
- benchmark track and protocol version;
- generation cap used;
- AC pass-count trajectory;
- final per-AC verdict JSON from the conformance suite;
- quality/composite fields when using v1;
- mechanical rank fields;
- optional separate human-feel note.
