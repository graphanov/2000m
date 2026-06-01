# 2000m v2: Workflow-Resilience Benchmark Design

## Status

Initial implementation. This document defines the benchmark-v2 contract and the first scorer/harness implementation. It does not launch models and does not claim that any entrant wins.

## Why v2 exists

v0 and v1 answer a narrow artifact question: did the submitted Rust game satisfy deterministic SkiFree-inspired mechanics through the driver protocol?

That remains valuable, but it misses a second question that showed up during multi-generation runs:

> Can an entrant use evidence, feedback, interruption recovery, and stop decisions to build better without overclaiming or looping blindly?

v2 is the benchmark track for that second question. It still scores a 2000m produced-game task, but it adds a workflow-resilience layer around the build process.

## Neutrality rule

The benchmark must not require or privilege any workflow framework.

An entrant may be:

- a single model with a prompt;
- a model plus a human operator;
- a scripted agent loop;
- a repo-native workflow system;
- another future process that can produce the required generic run record.

The v2 harness should only read benchmark-owned inputs and generic entrant outputs. Framework-specific logs can be attached as evidence, but the scorer must not depend on framework-specific files, commands, labels, or schemas.

## What v2 measures

v2 should measure workflow value without turning into a raw model-intelligence ranking. The score should be track-labeled and based on observable evidence.

### 1. Artifact quality

The produced game still has to satisfy the 2000m task. v2 may reuse v1 scorer output as the artifact-quality input until v2-specific game ACs exist.

Required signals:

- deterministic conformance output;
- per-AC verdicts;
- quality/composite fields with provenance labels;
- reproducible commands for rerunning the artifact score.

### 2. Feedback integration

The entrant receives scorer and/or reviewer feedback between phases. v2 measures whether later artifacts address valid findings without deleting evidence, gaming the scorer, or claiming unsupported passes.

Required signals:

- feedback item IDs;
- entrant response per item: `accepted`, `rejected_with_reason`, `needs_scorer_inspection`, or `deferred`;
- linked artifact changes or rationale;
- rerun scorer output after changes.

### 3. Context-wipe recovery

A v2 scenario can force a handoff after one or more attempts. The next worker/process receives only the allowed repo state and generic run record, not hidden chat history.

Scored behavior:

- identifies the current artifact state;
- identifies the current failing ACs and highest-priority next action;
- preserves useful evidence instead of starting from scratch unnecessarily;
- does not invent unverified history.

### 4. Stop-condition correctness

The entrant must recommend one of:

- `continue` — another attempt is likely to improve the score;
- `stop` — the score has reached the track ceiling or further attempts add noise;
- `redesign` — the entrant strategy/task setup is wrong;
- `inspect_scorer` — a scorer ambiguity, probe-only field, impossible AC, or stale requirement should be investigated before more attempts.

v2 should reward correct stop/redesign/inspect decisions, not just more generations.

### 5. Impossible, stale, and probe-only requirement handling

Scenarios may include requirements that are intentionally impossible, stale relative to the current scorer, or probe-only rather than ranking proof.

Scored behavior:

- detects the requirement type;
- avoids claiming a false pass;
- routes to scorer inspection or requirement redesign when appropriate;
- keeps final result claims bounded to measured fields.

### 6. Replayability and evidence hygiene

A v2 result should be replayable by another evaluator.

Required signals:

- exact repository/ref for the produced artifact;
- scenario ID and version;
- commands used to build and score;
- machine-readable scorer outputs;
- run record with phase history;
- public-safe evidence paths or URLs;
- no local private paths or hidden operator-only state in public result records.

## Scenario model

A v2 scenario is a benchmark-owned file matching [`scenario.schema.json`](scenario.schema.json). It defines:

- `scenarioId` and `scenarioVersion`;
- base game task and protocol track;
- phase sequence;
- allowed inputs per phase;
- feedback injections;
- context-wipe points;
- stale/impossible/probe-only traps;
- expected output artifacts;
- scoring weights.

The first v2 implementation should start with one public scenario and keep hidden scenario variants for later anti-overfit work.

## Run-record model

An entrant submits a generic run record matching [`run-record.schema.json`](run-record.schema.json). It should describe:

- entrant label and declared process type;
- scenario ID/version;
- produced artifact ref;
- phase outputs;
- scorer outputs;
- feedback responses;
- context-wipe recovery summaries;
- stop recommendation;
- evidence refs.

The record can point to additional logs, but the scorer must be able to evaluate core v2 fields without knowing the entrant's internal framework.

## Scoring sketch

Initial v2 composite should remain simple and explainable:

```text
artifact_quality_score      40%
feedback_integration_score  20%
recovery_handoff_score      15%
stop_condition_score        15%
evidence_replay_score       10%
```

Rules:

- each component must be reported separately;
- skipped/unavailable required evidence counts as zero for that component;
- probe-only evidence can support diagnostics but must not satisfy a ranked pass alone;
- a result with private/unreplayable evidence cannot be ranked as public proof;
- v2 scores are not comparable to v0 or v1 scores.

## Public result claims

Allowed:

- "Entrant X scored Y on 2000m v2 scenario Z."
- "The run record shows correct/incorrect recovery, feedback use, and stop decisions under this scenario."
- "Artifact score and workflow-resilience score are separate components."

Not allowed:

- "This proves general model intelligence."
- "This proves a framework wins before real contender evidence exists."
- "This v2 workflow score is directly comparable to v0/v1 mechanical score."
- "Probe-only or driver-reported evidence proves visual quality or portable performance by itself."

## Implementation status

Implemented in the first scorer/harness slice:

1. v2 Rust crate for scenario loading, run-record loading, and scoring.
2. Public starter scenario and weak sample run record.
3. Scoring functions for the five v2 components with unit tests.
4. v1 conformance JSON as the first artifact-quality input.
5. CLI command to score a v2 run record against a scenario.
6. CI-capable smoke that validates the weak sample scores below ceiling.

Still future work:

- hidden/private scenario variants;
- model/runtime launcher integration, if ever needed;
- richer reviewer packet fixtures;
- results leaderboard rendering for v2 rows.

## Acceptance criteria for the first implementation slice

- v2 scorer rejects missing required phase outputs.
- v2 scorer rejects framework-specific required fields in the generic contract.
- v2 scorer treats missing replay evidence as zero for replayability.
- v2 scorer separates artifact score from workflow-resilience components.
- v2 scorer can consume v1 conformance JSON as artifact-quality input.
- v2 scorer emits machine-readable JSON and a compact human summary.
- CI proves the included weak sample scores below ceiling.
