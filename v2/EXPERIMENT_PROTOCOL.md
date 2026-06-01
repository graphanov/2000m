# 2000m v2 paired campaign protocol

Status: protocol-freeze candidate. Do not run live contender trials from this protocol until the protocol PR is merged or explicitly marked calibration-only.

## Purpose

The first v2 scorer and result spine make it possible to record workflow resilience, but they do not by themselves prove that any workflow caused a better artifact. The first 2000m v1 two-lane run produced a qualitative signal: the Open Scaffold lane looked more game-like even though the mechanical score tied. That signal is useful, but causality is unproven.

Possible explanations include:

- sample variance;
- better initial prompt or context framing;
- a workflow effect from persistent work records and analysis;
- one lucky generation.

This protocol freezes the next experiment shape before live results tempt the benchmark or narrative to move.

## Hypothesis

Under equal model/runtime/budget conditions, Open Scaffold's work-record/evolve loop produces more recoverable, visually coherent, and improvement-directed game artifacts than a naked model loop.

This is a workflow-value hypothesis. It is not a claim that Open Scaffold makes a model smarter, proves adoption, or wins because one output looked cooler.

## Experiment stages

### Stage 0 — protocol freeze

A PR that changes this protocol, the campaign schema, or the visual/artifact track must land before live paired trials are treated as evidence. If scorer or scenario changes are made after looking at live outcomes, every affected run must be labeled calibration and excluded from proof claims.

### Stage 1 — smoke / calibration

Run only enough neutral checks to prove the machinery is not broken:

- v2 scorer accepts known-good fixtures;
- rank blockers catch missing outputs and private/local references;
- result spine renders from machine-readable JSON;
- visual/replay packaging can be produced from fixed seeds;
- no Open Scaffold-specific field is required by the neutral v2 contracts.

Stage 1 can justify fixing the protocol. It cannot justify a workflow superiority claim.

### Stage 2 — paired private pilot

Run a small paired campaign across the same scenario/task seeds. The default pilot size is three paired seeds unless the campaign file declares a different cap before running.

Goal: decide whether there is a repeatable enough signal to justify a larger campaign. Stage 2 may produce a private directional finding, not a public proof claim.

### Stage 3 — real evidence campaign

Run multiple seeds/tasks and, if needed, multiple models. Pair every condition on the same task/seed so the comparison is about deltas under equal conditions, not vibes from unrelated runs.

## Lanes

Every enabled lane must receive the same base task, same public benchmark specification, same model/runtime, same generation cap, same scorer-feedback budget, and the same feedback packets at the same phase boundaries.

### Lane A — naked model baseline

Purpose: baseline model loop without Open Scaffold state or evolve protocol.

Allowed:

- the same model and runtime used by other enabled lanes;
- the same v1/v2 scorer outputs and reviewer/scorer feedback packets;
- a simple append-only trajectory file and per-generation notes;
- build/test/score commands needed to produce and evaluate the artifact.

Not allowed:

- `.osc` state, Open Scaffold plans, run packets, eval envelopes, evolve logs, or compact evidence commands;
- hidden extra feedback or human-written repair hypotheses unavailable to Lane B;
- post-hoc hand edits not credited as model/operator intervention.

### Lane B — Open Scaffold ledger/analyze lane

Purpose: test the current Open Scaffold tooling as a work-record and analysis loop, not as an autonomous controller.

Required:

- active plan listing the relevant 2000m task/scenario requirements;
- run packets for each generation;
- neutral scorer output imported into evaluation records;
- `osc evolve analyze` or equivalent current Open Scaffold analysis output;
- compact evidence for handoff/replay;
- explicit continue/stop/redesign/inspect-scorer recommendations;
- enough repository state for a fresh agent to resume without chat history.

Not allowed:

- different model, runtime, generation cap, scorer feedback, or budget from Lane A;
- claiming a controller effect unless a real controller is running;
- adding Open Scaffold-specific requirements to the neutral benchmark contract.

### Lane C — controller lane

Purpose: test a stronger claim only if a real bounded controller exists.

Default: disabled.

Lane C may only be enabled after the controller is implemented and reviewed. If disabled, the campaign must not imply that a controller existed or contributed to results.

## Controlled conditions

A campaign file must declare:

- model label and runtime label;
- generation cap;
- scorer-feedback budget;
- reviewer-feedback budget, if any;
- wall-clock policy or explicit statement that wall-clock is diagnostic only;
- scenario refs and task/seed pairs;
- exact benchmark/scorer commit or equivalent immutable reference;
- enabled lanes and lane-specific allowed/forbidden tools.

The default pilot budget is intentionally small:

- generation cap: 3;
- feedback rounds per generation: 1 scorer packet and, if used, 1 reviewer packet;
- no human taste edits during artifact generation;
- no scorer mutation after the first live result is inspected.

## Pairing and seeds

Each task seed creates one pair across enabled lanes. A paired comparison is valid only if:

- both lanes start from the same task seed and scenario version;
- both lanes use the same model/runtime budget;
- both lanes receive feedback at the same phase boundary;
- any context wipe is applied at the same logical point;
- artifact capture uses the same fixed visual seeds and capture windows.

If a lane fails to produce an artifact, keep the failed run in the campaign. Do not replace it with an easier rerun unless the whole pair is marked calibration.

## Evidence tracks

Keep the evidence tracks separate. Do not blend them into one narrative score.

### Mechanical score

The neutral 2000m v1/v2 scorer still matters. It answers whether required driver and workflow-resilience fields were satisfied under the benchmark contract.

Mechanical score alone is insufficient for a visual game-quality claim.

### Visual/artifact quality

Visual claims require replayable artifacts: screenshots, GIFs or deterministic replay captures, fixed visual seeds, and a rubric or blinded preference protocol. See [`VISUAL_ARTIFACT_TRACK.md`](VISUAL_ARTIFACT_TRACK.md).

### Trajectory quality

Track whether the lane:

- wastes fewer generations after plateau;
- states repair hypotheses that match current failures;
- distinguishes current-vs-frontier deltas;
- handles impossible/stale/probe-only requirements correctly;
- avoids regression churn.

### Evidence/recovery quality

Track whether:

- a fresh agent can resume from repo state;
- a human can understand what happened without chat history;
- stop/continue/redesign/inspect decisions are justified;
- artifacts are auditable from public-safe evidence refs.

## Freeze rules

Before the first live paired result is inspected, freeze:

- scenario JSON;
- scorer code and weights;
- campaign file;
- capture windows and visual seeds;
- decision rules;
- claim ladder.

Allowed after freeze:

- fixing infrastructure that failed before any live result was inspected;
- marking a run as calibration and excluding it from proof claims;
- opening a new protocol/scorer PR for a later campaign.

Not allowed after freeze:

- changing weights, rubric, seeds, scenario text, or lane rules because one lane looks better or worse;
- excluding a bad paired run unless the predeclared invalidation rule applies;
- turning a calibration run into proof after it looks favorable.

## Predeclared decision rules

For the small private pilot, use conservative language.

### No support

Say no output-quality advantage was found when:

- Lane B does not beat or tie Lane A mechanically in most valid pairs;
- or Lane B does not improve visual/artifact quality in most valid pairs;
- or Lane B's apparent advantage comes from unavailable feedback, budget drift, hand edits, or a scorer/protocol mutation;
- or only one run looks prettier.

Allowed claim: Open Scaffold value remains audit, handoff, and governance under this campaign until stronger evidence appears.

### Directional signal, causality unproven

Say signal exists but sample is too small when:

- Lane B looks better in one or more evidence tracks, but the effect is inconsistent;
- visual preference and mechanical score disagree;
- trajectory/recovery improves but artifact quality does not;
- the pilot has too few valid pairs to justify a broader claim.

Allowed claim: the pilot found a qualitative signal worth a larger campaign.

### Repeatable workflow-value support

For a pilot, this can only be a candidate finding. For a real evidence campaign, require predeclared thresholds such as:

- Lane B matches or beats Lane A mechanical score in most paired runs;
- and Lane B wins blinded visual preference or rubric score in most paired runs;
- and Lane B wastes fewer generations after plateau or makes better stop/redesign calls;
- and fresh-agent handoff/recovery is materially better;
- and no lane had hidden budget, prompt, feedback, or tooling advantages.

Allowed claim: evidence suggests Open Scaffold improves artifact quality or workflow trajectory under this benchmark design.

## Explicitly banned claims

Do not claim:

- Open Scaffold makes the model smarter;
- Open Scaffold wins because one output looked cooler;
- the old 2000m v1 result proves framework superiority;
- workflow score equals model ranking;
- adoption proof;
- public benchmark victory from calibration fixtures.

## Execution order after this protocol lands

1. Merge the protocol-freeze PR.
2. Run neutral Stage 1 smokes.
3. Create the private paired-pilot campaign branch/results workspace.
4. Run Lane A and Lane B paired trials under the frozen campaign file.
5. Package mechanical, visual, trajectory, and recovery evidence separately.
6. Decide using the predeclared claim ladder.
7. Only then write the claim, including a no-support or inconclusive claim if that is what the evidence says.
