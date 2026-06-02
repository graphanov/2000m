# 2000m v2

Status: first scorer/harness implementation. It validates generic v2 scenario/run-record files, consumes v1 conformance JSON as the initial artifact-quality input, and reports workflow-resilience component scores.

2000m v2 is the proposed workflow-resilience track for the 2000m benchmark. It keeps the judge independent: the benchmark defines scenarios, inputs, outputs, scoring, and evidence requirements; entrants may use any model, workflow, framework, operator process, or no framework at all.

v2 exists because v0 and v1 mostly measure the final produced game. They are useful, but they do not fully measure whether an entrant can recover after interruption, use scorer feedback well, avoid stale or impossible requirements, and stop or redesign instead of burning more generations.

## Files

- [`DESIGN.md`](DESIGN.md) — design brief, neutrality rules, scenario phases, scoring model, and implementation plan.
- [`EXPERIMENT_PROTOCOL.md`](EXPERIMENT_PROTOCOL.md) — frozen paired-campaign protocol, lane definitions, controls, freeze rules, and claim ladder.
- [`VISUAL_ARTIFACT_TRACK.md`](VISUAL_ARTIFACT_TRACK.md) — screenshot/GIF/replay, fixed-seed, rubric, blind-review, and visual-feature requirements.
- [`PRIVATE_PILOT_AB_RUNBOOK.md`](PRIVATE_PILOT_AB_RUNBOOK.md) — Phase 1/Phase 2 private A/B pilot runbook, lane rules, evidence recording plan, and Phase 2 bootstrap template.
- [`scenario.schema.json`](scenario.schema.json) — draft machine-readable scenario contract for v2 fixtures.
- [`run-record.schema.json`](run-record.schema.json) — draft entrant-submitted run record contract for v2 scoring.
- [`result.schema.json`](result.schema.json) — machine-readable scorer result contract for rendered result rows.
- [`campaign.schema.json`](campaign.schema.json) — machine-readable paired-campaign contract for controlled pilots.
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

## Stage 1 neutral smokes

Before any live paired pilot, run the neutral Stage 1 smoke pack:

```bash
python3 scripts/run_v2_stage1_smokes.py
```

This checks the v0 stub discrimination band, the v2 smoke fixture, every pilot
calibration fixture, rank blockers for missing outputs and private/local scorer
inputs, result-spine rendering, campaign validation, neutral-contract wording,
and a deterministic visual/replay package generated from the fixed campaign
seeds. It produces no contender row and no workflow-value claim.

To inspect the generated visual package, write it to an empty directory:

```bash
python3 scripts/run_v2_stage1_smokes.py --visual-out <empty-output-dir>
```

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

## Paired campaign protocol

The next step after calibration is a frozen paired campaign, not a single public
rerun. The protocol is:

1. freeze the scenario, scorer, campaign config, visual capture plan, and
   decision rules before inspecting live results;
2. run neutral smokes/calibration;
3. run paired Lane A / Lane B private pilots under equal model/runtime/budget
   conditions;
4. run `python3 scripts/check_v2_feedback_parity.py <private-run-root> --generations <generation-number>` after scoring each generation and before later generations can consume scorer feedback; by default this checks the frozen `101`/`202`/`303` × `A`/`B` pilot matrix;
5. compare mechanical score, visual/artifact quality, trajectory quality, and
   evidence/recovery quality separately;
6. write a conservative claim, including no-support or inconclusive outcomes.

The public campaign example is
[`examples/paired-private-pilot.campaign.json`](examples/paired-private-pilot.campaign.json).
Validate campaign files with:

```bash
python3 scripts/validate_v2_campaigns.py
```

Lane C is disabled in campaign schema v1 until a real controller exists and the
schema/validator grow immutable controller implementation/evidence refs. Do not
treat the current Open Scaffold ledger/analyze lane as a controller lane.

## Boundary

- This repo owns benchmark-v2 design, scenarios, scorer, harness, result schema, and benchmark truth.
- Produced-game or contender repos own their implementation work records and generated artifacts.
- Any workflow system can participate only through the generic v2 contracts. No workflow framework is required or privileged.
- v2 results are benchmark-specific evidence, not general model-intelligence claims.

## Non-goals

- No runner that launches models or workflow tools.
- No claims that any entrant, framework, model, or process wins.
