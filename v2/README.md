# 2000m v2

Status: design draft. No v2 scorer or harness is implemented in this directory yet.

2000m v2 is the proposed workflow-resilience track for the 2000m benchmark. It keeps the judge independent: the benchmark defines scenarios, inputs, outputs, scoring, and evidence requirements; entrants may use any model, workflow, framework, operator process, or no framework at all.

v2 exists because v0 and v1 mostly measure the final produced game. They are useful, but they do not fully measure whether an entrant can recover after interruption, use scorer feedback well, avoid stale or impossible requirements, and stop or redesign instead of burning more generations.

## Files

- [`DESIGN.md`](DESIGN.md) — design brief, neutrality rules, scenario phases, scoring model, and implementation plan.
- [`scenario.schema.json`](scenario.schema.json) — draft machine-readable scenario contract for v2 fixtures.
- [`run-record.schema.json`](run-record.schema.json) — draft entrant-submitted run record contract for v2 scoring.

## Boundary

- This repo owns benchmark-v2 design, scenarios, scorer, harness, result schema, and benchmark truth.
- Produced-game or contender repos own their implementation work records and generated artifacts.
- Any workflow system can participate only through the generic v2 contracts. No workflow framework is required or privileged.
- v2 results are benchmark-specific evidence, not general model-intelligence claims.

## Non-goals for this design slice

- No executable v2 scorer.
- No hidden-seed harness.
- No runner that launches models or workflow tools.
- No claims that any entrant, framework, model, or process wins.
