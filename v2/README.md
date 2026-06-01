# 2000m v2

Status: first scorer/harness implementation. It validates generic v2 scenario/run-record files, consumes v1 conformance JSON as the initial artifact-quality input, and reports workflow-resilience component scores.

2000m v2 is the proposed workflow-resilience track for the 2000m benchmark. It keeps the judge independent: the benchmark defines scenarios, inputs, outputs, scoring, and evidence requirements; entrants may use any model, workflow, framework, operator process, or no framework at all.

v2 exists because v0 and v1 mostly measure the final produced game. They are useful, but they do not fully measure whether an entrant can recover after interruption, use scorer feedback well, avoid stale or impossible requirements, and stop or redesign instead of burning more generations.

## Files

- [`DESIGN.md`](DESIGN.md) — design brief, neutrality rules, scenario phases, scoring model, and implementation plan.
- [`scenario.schema.json`](scenario.schema.json) — draft machine-readable scenario contract for v2 fixtures.
- [`run-record.schema.json`](run-record.schema.json) — draft entrant-submitted run record contract for v2 scoring.
- [`result.schema.json`](result.schema.json) — machine-readable scorer result contract for rendered result rows.
- [`conformance/`](conformance/) — first v2 scorer/harness crate.
- [`examples/`](examples/) — public smoke scenario, pilot scenario, golden run-record fixtures, sample v1 conformance input, and generated scorer-result fixtures.

## Run the v2 scorer

```bash
cargo run --manifest-path v2/conformance/Cargo.toml -- \
  v2/examples/workflow-resilience-smoke.scenario.json \
  v2/examples/weak-run-record.json \
  --json-out /tmp/2000m-v2-smoke.json
```

The included weak sample should score below ceiling. It is a scorer smoke fixture, not a contender result.

## Pilot calibration pack

The [`examples/`](examples/) directory also contains a `workflow-resilience-pilot`
scenario and five golden run records:

- a ranked complete workflow fixture;
- a ranked but weaker fixture;
- a wrong-stop-decision fixture;
- a rank-blocked missing-output fixture;
- a rank-blocked private-path fixture.

Generate or check their result rows with:

```bash
mkdir -p v2/examples/results
for name in good weak-ranked missing-output private-path wrong-stop; do
  cargo run --quiet -p m2000-v2-conformance -- \
    v2/examples/workflow-resilience-pilot.scenario.json \
    v2/examples/pilot-${name}-run-record.json \
    --json-out v2/examples/results/pilot-${name}-result.json
done
python3 scripts/render_results.py --check
```

Those rows exercise the result spine. They are calibration fixtures only, not a
public model/workflow leaderboard.

## Boundary

- This repo owns benchmark-v2 design, scenarios, scorer, harness, result schema, and benchmark truth.
- Produced-game or contender repos own their implementation work records and generated artifacts.
- Any workflow system can participate only through the generic v2 contracts. No workflow framework is required or privileged.
- v2 results are benchmark-specific evidence, not general model-intelligence claims.

## Non-goals

- No runner that launches models or workflow tools.
- No claims that any entrant, framework, model, or process wins.
