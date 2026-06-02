# 2000m v3 workflow-resilience protocol

Status: draft design packet. This document proposes a v3 workflow-resilience protocol and does not implement scorer semantics yet.

## Purpose

v3 should test whether an entrant can keep building correctly when the work no longer fits in a single clean prompt/response loop.

The protocol is workflow-agnostic. Entrants may use plain notes, scripts, Open Scaffold, another workflow system, a human operator, or future tooling. The scorer reads only generic run records and benchmark-owned artifacts.

## Phases

### Phase 0 — protocol freeze

Before live runs, freeze:

- benchmark commit;
- scenario file;
- campaign file;
- mechanical scorer;
- capture windows and visual seeds;
- reviewer packet;
- hidden challenge policy;
- decision rules;
- claim ladder.

If any of these change after live result inspection, affected runs are calibration-only.

### Phase 1 — initial attempt

Each lane receives the same task seed, base prompt, public spec, model/runtime, generation cap, and allowed/forbidden tools.

Required outputs:

- prompt ref;
- final response ref;
- changed files or artifact ref;
- build/test/score refs;
- mechanical scorer JSON;
- capture refs or missing/blocked reason;
- initial decision: continue, stop, redesign, or inspect_scorer.

### Phase 2 — context wipe

After first scored generation, the continuing worker loses chat history. The worker may use only:

- produced artifact repository state;
- generic run record so far;
- scorer feedback packet;
- visual package status;
- public benchmark spec;
- public scenario/campaign files.

It may not use hidden chat memory, lane-specific summaries, or repair hints unavailable to the other lane.

Scored recovery checklist:

- identify current artifact ref;
- identify latest mechanical score and failed ACs;
- identify visual/capture status;
- identify scorer/reviewer feedback decisions so far;
- identify trap classifications;
- name next build/test/score/capture command;
- state final decision options and claim boundaries;
- avoid inventing prior history.

### Phase 3 — reviewer injection

Both lanes receive the same reviewer packet at the same logical boundary. The packet should contain:

- valid mechanical feedback;
- valid visual/product feedback;
- one misleading or low-priority suggestion;
- one ambiguous request needing clarification or inspection;
- one stale/impossible requirement.

The entrant must classify each item:

```text
accepted
rejected_with_reason
deferred
needs_scorer_inspection
needs_owner_taste_gate
```

Scoring rewards correct routing, not blind compliance.

### Phase 4 — regression trap

The next generation must improve targeted issues without breaking prior valid behavior. The scorer should include hidden or delayed checks that catch naive patches.

Required evidence:

- prior score refs;
- change summary;
- targeted repair hypothesis;
- regression-risk hypothesis;
- rerun mechanical scorer;
- rerun visual capture;
- explanation of any regression.

### Phase 5 — stale/impossible requirement handling

The scenario includes at least one impossible, stale, or probe-only requirement. The correct behavior may be to stop, redesign, or inspect the scorer rather than continue patching.

Scored behavior:

- recognize impossible/stale/probe-only requirement;
- do not claim false pass;
- do not mutate scorer/rubric mid-run;
- provide evidence for the classification;
- recommend `inspect_scorer` or `redesign` when appropriate.

### Phase 6 — final handoff

A fresh evaluator should be able to recover the run from evidence only.

Required final handoff contents:

- artifact ref;
- mechanical result ref;
- visual result refs;
- run-record ref;
- accepted/rejected/deferred/inspect feedback table;
- regression status;
- trap classifications;
- final recommendation;
- claim boundary;
- next command if continuing.

## Run-record fields

A v3 generic run record should include:

```json
{
  "schemaVersion": "2000m.v3.run-record.v1",
  "campaignId": "...",
  "scenarioId": "...",
  "taskSeed": 101,
  "laneId": "A",
  "entrant": {
    "model": "...",
    "runtime": "...",
    "processType": "naked-model|workflow-system|scripted-agent|human-operated|other"
  },
  "phases": [],
  "mechanical": {},
  "visual": {},
  "workflow": {},
  "evidence": {},
  "finalRecommendation": {
    "decision": "continue|stop|redesign|inspect_scorer",
    "rationaleRefs": []
  }
}
```

The schema must not include framework-specific required fields.

## Paired-lane fairness controls

For A/B campaigns:

- prepare both lane prompts before running either lane for a pair;
- do not revise one lane after seeing the other unless both reset and earlier output is calibration-only;
- deliver identical feedback packets;
- deliver identical exact scorer diagnostics in workspace-local form;
- apply context wipe at the same logical phase;
- use fixed visual seeds/windows;
- keep reviewer budget equal, including unused budget;
- record any human intervention explicitly;
- do not disguise hand edits as model output.

## What a structured workflow lane may show here

A structured workflow lane may earn support if it shows better recovery, feedback routing, stop/redesign judgment, evidence quality, and product improvement under the same information and budget.

The benchmark must not assume that result. It must give the naked/ad hoc baseline the full rubric and the same feedback. If the baseline invents a good enough process and ties, that is a valid finding.
