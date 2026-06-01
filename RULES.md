# 2000m Benchmark Rules

This is the rules checkpoint for the 2000m trail: how a produced-game entry
joins the run, what the judge measures, and what stays off the scoreboard.

2000m is a benchmark for one narrow task family: producing and evaluating a
SkiFree-inspired Rust game through deterministic driver protocols. It is not a
general intelligence scorecard. The benchmark repository remains the neutral
harness/judge, not a finished game, game solution, or workflow framework.

## How a model enters

1. Create a separate produced-game repository for the model, for example
   `2000m-<model>`.
2. The produced-game repository or work directory owns the entrant's work
   record: prompts, attempts/generations, logs, evidence, and artifact changes
   live there or in a separate results record. This benchmark repository stays
   neutral and ships no framework apparatus.
3. Build a Rust game/homage there. Do not put produced-game source into this
   benchmark repo.
4. Ship a headless driver that implements the selected protocol:
   - v0: `protocol/2000m.driver.v0.md`
   - v1: `v1/2000m.driver.v1.md`
5. Add a root `2000m.json` manifest that validates against
   `protocol/2000m.json.schema.json` and declares how to launch the driver.
6. Run the matching conformance suite from this repo against the produced-game
   directory.
7. Record each attempt/generation's AC pass count, quality fields when present,
   and per-AC verdicts in `results/`.

The conformance suite is the scorer. It drives the produced game only by
spawning the manifest's subprocess driver and sending JSON-line protocol
commands over stdin/stdout.

## No private shortcuts (brutal mode)

The suite gives the produced game no private setup hints, shortcut keys, or
model-specific flags. It sends an empty `config` on every `init`.

For v0, the suite reaches each mechanic the way a human would: it reads the
seeded obstacle and monster coordinates the game itself reports and steers the
skier into trees, onto ramps, and into the yeti by ordinary protocol input.

For v1, the public protocol also defines standardized `challenge` scenarios for
stress/probe checks such as dense fields and high-speed tunneling. Those
scenarios are benchmark inputs, not secret answer keys: a produced game still
must expose real protocol state and cannot pass by special-casing private flags
or undocumented setup paths.

## Mechanical score

The ranked score is track-labeled and must state its evidence source:

- seed-determinism must hold for the run to be trusted;
- v0 checks AC1–AC16 mechanically through the driver protocol;
- v1 checks AC1–AC28 and adds quality scoring across three tiers;
- v1 skipped/untestable ACs count as zero in ranked composite denominators;
- the primary v0 metric is the AC-pass trajectory across attempts/generations;
- the v1 standalone composite is the scorer output from AC pass rate plus
  per-AC quality over all 28 ACs; it does not include LOC, convergence speed,
  human feel, or external result-repo trajectory fields;
- v1 currently includes explicitly labeled host-bound timing probes and
  driver-reported profile telemetry, so timing fields must not be described as
  portable, independently verified performance proof;
- final pass count, generations-to-playable, generations-to-yeti, monotonicity,
  and generation efficiency may be reported as separate mechanical columns.

Networking, random OS state, manual play, self-attested visual polish, and human
feel are not part of the mechanical rank. Host wall-clock timing and
driver-reported profile telemetry may appear in v1 evidence only when labeled by
source.

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

A v0/v1 run record should include:

- model name and produced-game repository path;
- benchmark track and protocol version;
- generation cap used;
- AC pass-count trajectory;
- final per-AC verdict JSON from the conformance suite;
- quality/composite fields when using v1;
- mechanical rank fields;
- optional separate human-feel note.

A v2 run record should additionally include:

- scenario ID and version;
- generic phase outputs for feedback, context-wipe recovery, trap handling, and
  final stop decision;
- the scorer result JSON matching `v2/result.schema.json`;
- separate artifact-quality and workflow-resilience component scores;
- a claim boundary stating whether the row is a calibration fixture or a real
  contender result.

A v2 paired campaign should additionally freeze before live results:

- campaign JSON matching `v2/campaign.schema.json`;
- Lane A / Lane B definitions and Lane C disabled unless a real controller exists;
- same model/runtime/budget/feedback controls across enabled lanes;
- seed/task pairing rules;
- visual artifact package requirements and fixed capture seeds;
- predeclared decision rules and banned claims.
