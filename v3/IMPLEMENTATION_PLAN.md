# 2000m v3 implementation plan

Status: draft design packet. This is a staged implementation guide, not an executed plan.

## Gate 0 — design packet review

Artifacts:

- `v3/POSTMORTEM.md`
- `v3/DESIGN.md`
- `v3/MANIFEST_AND_CAPTURE.md`
- `v3/WORKFLOW_RESILIENCE_PROTOCOL.md`
- `v3/EVIDENCE_AND_CLAIMS.md`
- `v3/IMPLEMENTATION_PLAN.md`
- `v3/SPEC_GAPS_AND_DECISIONS.md`

Acceptance criteria:

- The postmortem names the current benchmark failure without spinning it into a win.
- v3 separates mechanical, visual/product, workflow, and evidence/recovery/governance tracks.
- The design remains workflow-agnostic and does not require Open Scaffold.
- Missing native visual capture blocks visual claims without invalidating mechanical scoring.
- Fairness controls include feedback parity, context-wipe parity, and fixed visual seeds/windows.
- Claim ladder uses conservative language.
- Public-safety scan finds no private identity/path leaks or unsupported superiority claims.

## Gate 1 — schema design PR

Add draft schemas only, no scorer yet:

```text
v3/manifest.schema.json
v3/run-record.schema.json
v3/result.schema.json
v3/campaign.schema.json
v3/visual-package.schema.json
```

Acceptance criteria:

- Manifest schema supports `driver`, `capture`, `playable`, and asset-boundary metadata.
- Run-record schema is generic and has no framework-specific required fields.
- Result schema exposes four separate tracks.
- Campaign schema freezes seeds, windows, feedback packets, model/runtime budget, and context-wipe phase.
- Booleans are rejected for integer seed/budget fields.
- Lane arrays support workflow-agnostic process labels.
- Missing visual capture can block visual track while allowing mechanical scoring.

Verification:

```bash
python3 scripts/validate_v3_schemas.py
cargo build --workspace --quiet
cargo test --workspace --quiet
```

## Gate 2 — mechanical v3 scorer scaffold

Implement only Track 1 first.

Acceptance criteria:

- v3 driver protocol loads from v3 manifest.
- deterministic checks run against public and hidden seed/input fixtures.
- invalid enum/schema poison blocks or fails the mechanical track.
- regression stability reruns the final artifact under fixed hidden checks.
- v3 result JSON reports mechanical fields separately from other tracks.
- v0/v1/v2 smokes remain passing.

## Gate 3 — visual/package harness

Implement capture validation before scoring human preference.

Acceptance criteria:

- capture command runs for frozen windows/seeds;
- screenshot/GIF/frame metadata refs are produced;
- capture determinism is checked across repeated runs;
- missing/failed capture produces `visual.ranked=false` with reason;
- blind contact sheet and sealed label map can be generated;
- copied/extracted asset boundary remains documented and checked where possible.

Verification:

```bash
python3 scripts/run_v3_visual_smokes.py --visual-out <empty-output-dir>
python3 scripts/validate_v3_visual_packages.py <fixture-output>
```

## Gate 4 — workflow-resilience scenario pack

Implement public calibration scenarios, not live contender results.

Scenarios should cover:

- context wipe;
- reviewer packet with valid/misleading/stale feedback;
- regression trap;
- impossible/probe-only requirement;
- scorer feedback integration;
- final handoff.

Golden fixtures:

- complete ranked fixture;
- weak but ranked fixture;
- missing visual rank-blocked fixture;
- wrong feedback-routing fixture;
- wrong stop-decision fixture;
- private-path rank-blocked fixture;
- scorer-mutated invalid fixture.

Acceptance criteria:

- Fixtures exercise every decision path.
- Calibration rows are clearly not contender results.
- Result renderer keeps all tracks separate.
- Public docs reject workflow-superiority claims from fixtures.

## Gate 5 — paired campaign protocol

Only after v3 smokes pass, freeze a private pilot protocol.

Acceptance criteria:

- Lane A gets the full rubric and same exact feedback, without the campaign-specific structured workflow under test.
- Lane B is explicitly campaign-specific; if the campaign tests Open Scaffold, it may use Open Scaffold but receives no hidden information.
- Both lane prompts are prepared before either runs.
- Feedback packets and context wipe occur at identical phase boundaries.
- Native visual packages are required for visual track ranking.
- Missing visual packages block visual claims.
- Decision thresholds are predeclared.
- Runtime token telemetry is parsed from source-labeled logs or receipts and reported separately from mechanical scoring.

## Gate 6 — private pilot

Run a small private A/B only after the protocol is frozen.

Recommended initial pilot if the campaign specifically tests Open Scaffold:

```text
seeds: 3 to 5 task seeds
lanes: A unstructured baseline, B campaign-specific Open Scaffold workflow lane
generation cap: 3
context wipe: after first scored generation
reviewer packet: one per pair, same phase
visual windows: fixed before run
claim ceiling: private directional signal only
token telemetry: source-labeled total tokens when reliable; split/cost fields nullable unless supplied by the runtime
```

## Gate 7 — larger evidence campaign

Only if the private pilot shows a useful signal.

Requirements:

- more seeds;
- possibly more models;
- public-safe evidence;
- independent review of visual packages and run records;
- no post-hoc scorer/rubric changes;
- conservative public claim language.

## Parallel Open Scaffold follow-up

This benchmark repo should not implement Open Scaffold features. But findings can feed generic Open Scaffold improvements in that repo:

- stronger `osc evolve analyze` plateau/impossible requirement detection;
- better context-wipe/handoff summaries;
- compact evidence mode;
- generic scorer-feedback import hygiene;
- clearer stop/redesign/inspect-scorer recommendations.

Keep those improvements benchmark-agnostic.
