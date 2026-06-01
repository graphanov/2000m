# 2000m v2 examples

These files are public calibration fixtures for the v2 workflow-resilience scorer.
They are not contender results and do not prove that any model or workflow is
superior.

## Scenario

- `workflow-resilience-pilot.scenario.json` — pilot scenario that combines a
  v1 artifact score with scorer feedback, context-wipe recovery, probe-only,
  stale, impossible, and final stop-decision phases.
- `workflow-resilience-smoke.scenario.json` — smaller smoke fixture for the first
  scorer implementation.

## Run-record fixtures

- `pilot-good-run-record.json` — ranked fixture with complete generic outputs,
  replayable public refs, correct trap handling, and intentionally weak artifact
  quality imported from the sample v1 scorer JSON.
- `pilot-weak-ranked-run-record.json` — ranked fixture with lower stop/replay
  scores while still satisfying the public contract.
- `pilot-wrong-stop-run-record.json` — ranked fixture that keeps required outputs
  but makes the wrong final continue decision under trap phases.
- `pilot-missing-output-run-record.json` — rank-block fixture missing a required
  context-wipe handoff output.
- `pilot-private-path-run-record.json` — rank-block fixture with a private/local
  v1 scorer input ref.

## Generate the result fixtures

```bash
mkdir -p v2/examples/results
for name in good weak-ranked missing-output private-path wrong-stop; do
  cargo run --quiet -p m2000-v2-conformance --     v2/examples/workflow-resilience-pilot.scenario.json     v2/examples/pilot-${name}-run-record.json     --json-out v2/examples/results/pilot-${name}-result.json
done
python3 scripts/render_results.py --check
```

A valid calibration pack should include both ranked and rank-blocked examples so
future scorer changes cannot collapse v2 into score-only artifact deltas.
