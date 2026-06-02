# 2000m v2 private A/B pilot runbook

Status: Phase 1 preparation document. This file defines the private A/B pilot setup and Phase 2 execution path. It is not a completed pilot result and does not authorize live Lane A or Lane B execution.

## Purpose

The private A/B pilot tests workflow value under controlled conditions, not raw model intelligence.

It follows the Stage 1 neutral smoke runner:

```bash
python3 scripts/run_v2_stage1_smokes.py
```

A passing Stage 1 smoke run verifies that the scorer, result spine, campaign validator, rank blockers, and visual/replay packaging can run without producing live pilot results or workflow-value claims. The private pilot may produce directional internal findings only. It is not a public proof claim.

## Hypothesis and claim boundary

Hypothesis:

> Under equal model/runtime/budget conditions, Open Scaffold's work-record/evolve loop may produce more recoverable, visually coherent, and improvement-directed artifacts than a naked model loop.

This is a workflow-value hypothesis. It does not claim Open Scaffold makes the model smarter, and it does not claim controller behavior.

Banned claims for this pilot:

- Do not claim: "Open Scaffold makes the model smarter."
- Do not claim: "Open Scaffold won because one output looked cooler."
- Do not claim: "The old 2000m v1 result proves framework superiority."
- Do not claim: "Workflow score equals model ranking."
- Do not claim: "Adoption proof."
- Do not claim: "Calibration fixtures are public contender results."

Acceptable output language after Phase 2 is limited to predeclared decision rules: no support, directional signal with causality unproven, repeatable workflow-value support candidate, or invalid/calibration. The protocol claim ladder is in [`EXPERIMENT_PROTOCOL.md`](EXPERIMENT_PROTOCOL.md#predeclared-decision-rules); this runbook adds the pilot-specific invalidation/calibration gates below.

## Source files for Phase 2

Read these before executing the private pilot:

- [`EXPERIMENT_PROTOCOL.md`](EXPERIMENT_PROTOCOL.md)
- [`VISUAL_ARTIFACT_TRACK.md`](VISUAL_ARTIFACT_TRACK.md)
- [`campaign.schema.json`](campaign.schema.json)
- [`scenario.schema.json`](scenario.schema.json)
- [`run-record.schema.json`](run-record.schema.json)
- [`result.schema.json`](result.schema.json)
- [`examples/paired-private-pilot.campaign.json`](examples/paired-private-pilot.campaign.json)
- [`examples/workflow-resilience-pilot.scenario.json`](examples/workflow-resilience-pilot.scenario.json)
- [`examples/private-pilot-result-template.md`](examples/private-pilot-result-template.md)

## Pilot scope

Use the existing campaign fixture unless a fresh protocol PR changes it before any live result is inspected:

- campaign id: `paired-private-pilot-v1`
- scenario: `v2/examples/workflow-resilience-pilot.scenario.json`
- task seeds: `101`, `202`, `303`
- enabled lanes: `A`, `B`
- Lane C: present in the design but disabled
- generation cap: `3`
- scorer feedback budget: `1` packet per generation boundary
- reviewer feedback budget: `1` packet per generation boundary in the campaign fixture; if no reviewer packet is used, record the unused budget equally for both lanes
- wall-clock policy: diagnostic only

Do not run, inspect, or generate live pilot outcomes during Phase 1. Do not mutate scorer, scenario, campaign, visual seeds, visual windows, lane rules, or decision rules after any live result is inspected unless the affected run is marked calibration-only.

## Lane definitions

Every enabled lane receives the same base task, model, runtime, task seed, prompt budget, generation cap, scorer feedback packet, reviewer feedback packet if used, context-wipe timing, and visual capture seeds/windows.

### Lane A — naked model baseline

Purpose: baseline the same model/runtime without Open Scaffold state or evolve protocol.

Allowed:

- same model/runtime as Lane B;
- same benchmark specification, task seed, prompt budget, and generation cap;
- same build/test/score commands;
- same scorer feedback and reviewer feedback packets at the same phase boundaries;
- plain append-only trajectory notes;
- plain evidence sufficient to compare the run.

Forbidden:

- `.osc` state;
- Open Scaffold plans;
- Open Scaffold run packets;
- Open Scaffold eval envelopes;
- Open Scaffold evolve logs;
- Open Scaffold compact evidence;
- `osc evolve analyze`;
- extra human repair hints unavailable to Lane B;
- post-hoc hand edits disguised as model output.

Required evidence:

- generation prompt and final model response refs;
- stdout/stderr refs from the model runtime;
- build/test/score command refs and outputs;
- v1 conformance JSON for every scored generation;
- plain trajectory/evidence notes;
- v2 run record draft/final refs;
- visual package refs;
- context-wipe handoff summary;
- final continue/stop/redesign/inspect-scorer recommendation.

### Lane B — Open Scaffold ledger/analyze lane

Purpose: test Open Scaffold as a work-record and analysis loop, not as an autonomous controller.

Allowed/required:

- same model/runtime as Lane A;
- same benchmark specification, task seed, prompt budget, and generation cap;
- same scorer feedback and reviewer feedback packets at the same phase boundaries;
- Open Scaffold active plan for the run;
- Open Scaffold run packets for each generation;
- scorer import/evaluation envelope from neutral v1/v2 scorer output;
- `osc evolve analyze` output;
- compact evidence;
- explicit continue/stop/redesign/inspect-scorer recommendation;
- enough repository state for fresh-agent handoff.

Forbidden:

- different model/runtime from Lane A;
- extra scorer feedback or reviewer feedback;
- extra generation budget;
- hidden human repair hints unavailable to Lane A;
- claiming controller behavior;
- adding Open Scaffold-specific requirements to the neutral benchmark contract;
- post-hoc hand edits disguised as model output.

Required evidence:

- active/done plan refs;
- run packet refs;
- evaluation record refs;
- evolve analysis refs;
- compact evidence refs;
- generation prompt and final model response refs;
- stdout/stderr refs from the model runtime;
- build/test/score command refs and outputs;
- v1 conformance JSON for every scored generation;
- v2 run record draft/final refs;
- visual package refs;
- context-wipe handoff summary;
- final continue/stop/redesign/inspect-scorer recommendation.

### Lane C — disabled controller lane

Lane C is explicitly disabled for this private pilot. It exists only as a future controller lane.

Do not enable Lane C, implement a controller, claim controller behavior, or route Lane B as a controller lane. Enabling Lane C requires a separate implementation/review slice plus schema and validator updates before any campaign can use it.

## Fairness controls

The pilot is invalid or calibration-only if these controls drift:

- same model;
- same runtime;
- same generation cap;
- same scorer feedback budget;
- same reviewer feedback budget if reviewer feedback is used;
- same task seeds;
- same prompt budget;
- same context-wipe timing;
- same visual capture seeds/windows;
- same allowed benchmark/scorer inputs;
- same feedback packet content at the same logical phase boundaries;
- no hidden human repair hints in one lane only;
- no post-hoc hand edits disguised as model output;
- no scorer/scenario/campaign/rubric mutation after live result inspection.

If a lane fails to produce an artifact, keep the failed run in the pair. Do not replace only that lane with an easier rerun. If the pair must be rerun because of infrastructure failure, mark the original pair calibration and record why.

## Prompt and feedback boundaries

Before each paired seed:

1. Prepare one base task seed packet.
2. Prepare one Lane A prompt and one Lane B prompt with identical task requirements, model/runtime budget, and phase boundaries.
3. The only intended variable is process state:
   - Lane A has no Open Scaffold state/tools.
   - Lane B uses Open Scaffold ledger/analyze artifacts.
4. Save both prompts before running either lane.
5. Do not revise one lane's prompt after seeing the other lane's output unless both prompts are reset before execution and the earlier output is marked calibration.

Feedback packets:

- Scorer feedback packets come from the same scorer output shape for both lanes.
- Reviewer feedback packets, if used, must be identical in budget and comparable in content.
- Feedback must not contain lane-specific hidden repair hints.
- Every accepted/rejected/deferred/inspect decision is recorded in the v2 run record.

## Context-wipe timing and handoff test

Apply the context wipe at the campaign-declared phase:

```text
after first scored generation and before final recommendation
```

At the wipe point, the continuing worker may use only recorded repository state and public-safe evidence refs, not previous chat history.

Evaluate handoff quality by checking whether a fresh worker can recover:

- current artifact ref;
- latest v1 conformance score and failed ACs;
- v2 run record state so far;
- scorer feedback decisions;
- visual package status;
- trap classifications;
- next build/test/score command;
- final decision options and claim boundaries.

Lane B may use Open Scaffold compact evidence and evolve analysis for handoff. Lane A must still preserve a plain handoff summary sufficient for comparison.

## Evidence tracks

Keep tracks separate in result analysis.

### Mechanical score

Record v1 conformance JSON for every scored generation and v2 scorer result for every completed run record. Mechanical score does not prove visual quality or model intelligence.

### Visual/artifact quality

Use [`VISUAL_ARTIFACT_TRACK.md`](VISUAL_ARTIFACT_TRACK.md). Each lane should produce the same package shape:

- build ref;
- fixed visual seeds;
- capture command;
- screenshots;
- GIF or replay capture;
- replay log or frame metadata;
- rubric record;
- blind label map.

For the current campaign fixture, visual seeds are `1101`, `2202`, and `3303`; capture windows are early-game, mid-run obstacle field, and post-feedback rerun windows. If visual packages are missing or cannot be regenerated, block visual-superiority claims without necessarily invalidating mechanical/workflow-resilience tracks.

### Trajectory quality

Record whether each lane:

- improves, plateaus, or regresses across generations;
- uses scorer feedback correctly;
- states repair hypotheses tied to current failures;
- distinguishes current-vs-frontier state;
- detects stale, impossible, or probe-only traps;
- avoids wasteful retries after plateau.

### Evidence/recovery/handoff quality

Record whether:

- a fresh worker can resume from repo state;
- a human can audit what happened without chat history;
- evidence refs are public-safe and replayable;
- continue/stop/redesign/inspect decisions are justified;
- raw/private material stays out of public artifacts.

## Recording layout for Phase 2

Use repo-relative refs in committed artifacts. Use placeholder labels such as `<lane>`, `<seed>`, `<generation>`, and `<run-id>` in private workspaces. Do not commit local absolute paths.

Recommended per-lane run workspace shape:

```text
private-pilot/<campaign-id>/<pair-id>/
  lane-a/
    generation-01/
      prompt.md
      final-response.md
      runtime-stdout.log
      runtime-stderr.log
      build.log
      test.log
      score.log
      v1-conformance.json
      trajectory.md
      visual-package/
      handoff-summary.md
    generation-02/
    generation-03/
    v2-run-record.json
    v2-result.json
    lane-summary.md
  lane-b/
    generation-01/
      prompt.md
      final-response.md
      runtime-stdout.log
      runtime-stderr.log
      build.log
      test.log
      score.log
      v1-conformance.json
      open-scaffold-run-packet.json
      evaluation.json
      evolve-analyze.md
      compact-evidence.md
      visual-package/
      handoff-summary.md
    generation-02/
    generation-03/
    v2-run-record.json
    v2-result.json
    lane-summary.md
  pair-comparison.md
```

This layout is descriptive, not a new benchmark contract. The v2 scorer contract remains [`run-record.schema.json`](run-record.schema.json) plus [`scenario.schema.json`](scenario.schema.json).

## Per-generation record checklist

For every generation in every lane, record:

- pair id and task seed;
- lane id;
- generation number;
- model/runtime label;
- prompt ref;
- final response ref;
- stdout/stderr refs;
- files changed or artifact ref;
- build command and result;
- test command and result;
- v1 score command and v1 conformance JSON ref;
- scorer feedback packet ref;
- accepted/rejected/deferred/inspect decisions;
- visual package refs or missing-artifact reason;
- context-wipe/handoff status when applicable;
- recommendation: `continue`, `stop`, `redesign`, or `inspect_scorer`;
- rationale tied to evidence refs.

Use [`examples/private-pilot-result-template.md`](examples/private-pilot-result-template.md) as the Phase 2 recording template.

## How v1 conformance feeds v2 run records

For every scored generation:

1. Run the neutral v1 scorer and save machine JSON.
2. Reference that JSON in the v2 run record under `artifact.v1ConformanceJson`.
3. Copy the exact score command into `artifact.scoreCommand` and phase output `score-command`.
4. Copy the conformance JSON ref into phase output `conformance-json`.
5. Record scorer feedback decisions under `feedbackResponses`.
6. Include the conformance JSON as an evidence ref with kind `conformance-json`.
7. Run the v2 scorer against the completed run record to produce `v2-result.json`.

Do not rewrite v1 conformance JSON to improve a narrative. If a scorer input is private/local-only, the v2 scorer should rank-block it rather than silently accepting it.

## Decision rules and invalidation gates

Apply these at the campaign level after all valid pairs are recorded. Pair-level notes may describe deltas, but a single pair cannot establish repeatability.

### No support

Use when Lane B does not match or beat Lane A in the relevant evidence tracks, when visual/artifact quality does not improve, when any apparent advantage comes from unequal budget/feedback/tooling, or when only one run looks better.

### Directional signal; causality unproven

Use when Lane B improves one or more tracks but the pilot is too small, results disagree across tracks, or valid pairs are too few for a causal claim.

### Repeatable workflow-value support candidate

Use only as a candidate finding for the private pilot, and only if Lane B matches or beats mechanical score in most valid pairs, improves visual/artifact quality in most valid pairs, handles plateau/traps better, improves fresh-agent handoff, and has no hidden budget, prompt, feedback, or tooling advantage.

### Invalid/calibration only

Use when fairness controls drift, scorer/scenario/campaign/rubric semantics changed after live-result inspection, live output influenced protocol edits, one lane received hidden hints or post-hoc hand edits, visual capture windows/seeds differed, required evidence cannot be replayed, or a lane rerun replaced only one side of a pair. Calibration-only runs can inform protocol repair, but they cannot support proof claims.

## Commit vs local/private handling

Commit only public-safe, durable benchmark artifacts after review:

- protocol docs;
- sanitized campaign/scenario/run-record/result files if owner-approved for the repo;
- sanitized summaries and comparison docs;
- renderer/validator updates if separately justified before outcome-looking.

Keep local/private unless explicitly approved and sanitized:

- raw prompt transcripts;
- raw stdout/stderr logs;
- private blind-label mapping before review is sealed;
- local absolute paths;
- produced-game implementation work records that belong in produced-output repos;
- raw chat transcripts;
- personal identity, account paths, or Discord IDs;
- unreviewed screenshots/GIFs that include private or proprietary material.

The benchmark repo owns neutral protocol/scorer/result truth. Produced artifact repos own implementation work records and generated game files.

## Phase 2 execution order

1. Confirm the Phase 1 prep PR has landed or is otherwise approved.
2. Fetch current main for the canonical 2000m repository and verify the exact starting commit.
3. Run `python3 scripts/run_v2_stage1_smokes.py` again.
4. Read the source files listed above.
5. Freeze the campaign/scenario/scorer/visual/decision refs before live execution.
6. For each task seed, prepare Lane A and Lane B prompts before executing either lane.
7. Run Lane A and Lane B under the same model/runtime/budget.
8. Save per-generation evidence and v1 conformance JSON.
9. Apply the context-wipe/handoff test at the same logical point.
10. Capture visual packages with the same seeds/windows.
11. Produce v2 run records and v2 scorer results.
12. Compare mechanical, visual/artifact, trajectory, and evidence/recovery tracks separately.
13. Apply decision rules without overclaiming.
14. Stop before public proof claims, publish, release, or merge unless separately approved.

## Phase 2 bootstrap prompt template

Use this only after Phase 1 is approved. Replace bracketed values with the exact current repo state.

```text
MODE: Bounded execution. Phase 2 private A/B pilot only. No publish/release. No merge without owner approval. Do not change scorer/scenario/campaign/rubric semantics after inspecting any live result unless the affected run is marked calibration-only.

TARGET: canonical 2000m working tree at [local checkout], starting from main commit [exact commit] after Phase 1 prep PR [PR URL/status].

SOURCE OF TRUTH:
- Live canonical 2000m main at [exact commit].
- v2/PRIVATE_PILOT_AB_RUNBOOK.md
- v2/EXPERIMENT_PROTOCOL.md
- v2/VISUAL_ARTIFACT_TRACK.md
- v2/campaign.schema.json
- v2/scenario.schema.json
- v2/run-record.schema.json
- v2/result.schema.json
- v2/examples/paired-private-pilot.campaign.json
- v2/examples/workflow-resilience-pilot.scenario.json
- v2/examples/private-pilot-result-template.md

FIRST STEPS:
1. Load relevant skills for 2000m contender runs, Open Scaffold, GitHub PR workflow, and Codex review if opening a PR.
2. Fetch/prune origin, verify clean main, log the exact commit, and list open PRs.
3. Run: python3 scripts/run_v2_stage1_smokes.py
4. Read all Phase 2 source-of-truth files above before executing live lanes.

TASK:
Run the paired private pilot for campaign paired-private-pilot-v1 using seeds 101, 202, and 303. Run only enabled lanes A and B. Lane C is disabled.

LANE RULES:
- Lane A: naked model baseline, same model/runtime/budget/feedback, no .osc state, no Open Scaffold plans/run packets/eval envelopes/evolve logs/compact evidence, no osc evolve analyze. Preserve plain trajectory/evidence for comparison.
- Lane B: Open Scaffold ledger/analyze lane, same model/runtime/budget/feedback, use active plan, run packets, scorer import/evaluation envelope, osc evolve analyze, compact evidence, explicit continue/stop/redesign/inspect-scorer recommendations. Do not claim controller behavior.

FAIRNESS CONTROLS:
Use the same model, runtime, generation cap, scorer feedback budget, reviewer feedback budget if used, task seeds, prompt budget, context-wipe timing, visual capture seeds/windows, and feedback packets. Do not provide hidden human repair hints to only one lane. Do not disguise hand edits as model output.

EVIDENCE REQUIREMENTS:
For each generation, record prompt, final response, stdout, stderr, build/test/scorer outputs, v1 conformance JSON, visual package refs, feedback decisions, context-wipe/handoff status, and continue/stop/redesign/inspect-scorer recommendation. Feed v1 conformance JSON into v2 run records, then score v2 run records.

OUTPUT:
Produce private pilot run records/results and a comparison summary that separates mechanical score, visual/artifact quality, trajectory quality, and evidence/recovery quality. Use only conservative decision-rule language: no support, directional signal with causality unproven, repeatable workflow-value support candidate, or invalid/calibration.

STOP CONDITIONS:
- Stop if Stage 1 smokes fail.
- Stop if the working tree is dirty in unrelated ways.
- Stop before changing scorer/scenario/campaign/rubric semantics after live result inspection.
- Stop before enabling Lane C or implementing a controller.
- Stop before public proof claims, publish, release, or merge.
```
