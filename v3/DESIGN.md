# 2000m v3 design

Status: draft design packet. This document proposes v3 architecture only; it does not implement scorer semantics.

## Design goal

v3 should answer four separate questions:

1. **Mechanical correctness:** does the artifact obey the deterministic game-logic contract?
2. **Product / visual artifact quality:** does the artifact look and behave like a coherent playable SkiFree-inspired game?
3. **Workflow resilience:** can the entrant recover from context loss, feedback ambiguity, regressions, stale/impossible requirements, and handoff?
4. **Evidence / recovery / governance quality:** can a human or fresh worker audit what happened, recover the work, and trust the claim boundaries?

No single composite should hide these tracks. If a future leaderboard renders a composite for convenience, it must still expose each component and must not imply that workflow score is model intelligence.

## Non-goals

- Do not make a benchmark that requires Open Scaffold.
- Do not privilege `.osc` files, Open Scaffold CLI commands, or framework-specific schemas.
- Do not blend human taste into mechanical pass counts.
- Do not treat private/local evidence as public contender proof.
- Do not mutate scorer/campaign/rubric semantics after live result inspection unless affected runs are labeled calibration-only.

## Track 1 — Mechanical correctness

Purpose: keep the language-agnostic JSON-line driver as the deterministic substrate.

This track answers:

> Does the artifact obey the game logic contract?

It does not answer:

> Is this a good game?

### Required v3 mechanical checks

- deterministic `init`, `step`, `state`, `reset` behavior;
- seed-stable obstacle generation;
- collision correctness and recovery behavior;
- ramps, airborne/landing, style gain/loss;
- monster spawn, pursuit, contact, and flee behavior;
- replay consistency and checksum verification;
- regression stability across final reruns;
- hidden challenge seeds and hidden input streams;
- schema validation for every emitted state;
- no invalid enum/schema poison, including unsupported obstacle types or event labels;
- no scorer mutation by entrants.

### Mechanical result shape

Report at least:

```text
mechanical.ranked: true|false
mechanical.protocolVersion: 2000m.driver.v3
mechanical.passCount
mechanical.totalAcs
mechanical.compositeScore
mechanical.determinism.pass
mechanical.failedAcs[]
mechanical.hiddenChallengeSummary
mechanical.regressionSummary
mechanical.resultJsonRef
```

## Track 2 — Product / visual artifact quality

Purpose: require real visual/playable output instead of accepting only headless driver compliance.

This track answers:

> Did the entrant produce a coherent visible/playable game artifact?

### Required visual/product evidence

- fixed-seed screenshots;
- deterministic GIF or replay capture;
- frame metadata for each capture;
- visible skier, slope, obstacles, and monster when applicable;
- readable motion and obstacle spacing;
- coherent visual style;
- no absurd clutter or impossible-to-read object fields;
- replayable capture command;
- playable command or URL/path;
- blind human preference or rubric record for paired campaigns;
- optional simple visual metrics such as object spread, contrast, sprite variety, and frame continuity.

### Visual blocking rule

If an artifact lacks native capture and playable surfaces, it can still receive a mechanical result, but:

```text
visual.ranked = false
visual.blockReason = missing-native-capture-or-playable-surface
```

A neutral proxy renderer over JSON GameState may support private inspection, but it is not a replacement for submitted visual artifacts and must not unlock public visual-superiority claims by itself.

## Track 3 — Workflow resilience

Purpose: test workflow pain that strong one-shot coding ability does not necessarily solve.

This track answers:

> Can the entrant continue correctly under interruption, feedback, review ambiguity, regression risk, and handoff?

### Required workflow stressors

1. **Context wipe** — after a fixed phase, the continuing worker receives only allowed repo state and run records, not prior chat.
2. **Reviewer injection** — both lanes receive the same packet containing valid feedback, misleading feedback, and ambiguous/stale feedback.
3. **Regression trap** — at least one repair path can break an already-satisfied behavior; the entrant must preserve/retest prior evidence.
4. **Impossible/stale requirement** — at least one requirement is impossible under the frozen scorer or stale relative to the current contract.
5. **Exact scorer feedback integration** — the entrant must use exact diagnostics without inventing unsupported entities or editing the scorer.
6. **Handoff** — a fresh worker must recover artifact state, failed checks, feedback decisions, visual status, next commands, and claim boundaries from evidence only.

### Workflow result shape

Report at least:

```text
workflow.contextWipeRecoveryScore
workflow.feedbackDecisionScore
workflow.regressionProtectionScore
workflow.impossibleRequirementHandlingScore
workflow.handoffScore
workflow.finalRecommendation: continue|stop|redesign|inspect_scorer
workflow.rationaleRefs[]
```

## Track 4 — Evidence / recovery / governance quality

Purpose: ensure the work can be reviewed, recovered, replayed, and claimed safely.

This track answers:

> Can a human or future worker understand and verify what happened without trusting chat memory?

### Required evidence

- plan or equivalent work intent;
- prompt refs;
- final model response refs;
- stdout/stderr refs;
- files changed or artifact refs;
- build/test/score commands and results;
- mechanical scorer JSON refs;
- visual capture refs;
- scorer/reviewer feedback packets;
- accepted/rejected/deferred/inspect decisions;
- context-wipe handoff summary;
- final recommendation and rationale;
- public-safety scan;
- unsupported-claim scan;
- compact evidence summary;
- no private paths, owner identity, Discord IDs, unprocessed chat/log dumps, or local-only refs in public result records.

### Evidence result shape

Report at least:

```text
evidence.replayable: true|false
evidence.publicSafe: true|false
evidence.privateRefsBlocked: true|false
evidence.compactSummaryRef
evidence.requiredRefsMissing[]
evidence.claimBoundary
```

## Track separation rule

Track scores must be shown separately. A contender may be:

- mechanically strong but visually blocked;
- visually promising but mechanically incomplete;
- workflow-resilient but artifact-weak;
- evidence-clean but output-poor.

Those states are useful. v3 should preserve them instead of hiding them behind one number.

## Fair lane comparison

For paired comparisons:

- same model;
- same runtime;
- same task seed;
- same generation cap;
- same feedback packets;
- same prompt budget;
- same context-wipe timing;
- same visual seeds/windows;
- same reviewer budget;
- same freeze rules;
- no hidden human repair hints to one lane;
- no hand edits disguised as model output.

Lane A should be an unstructured/ad hoc baseline with full access to the benchmark rubric and feedback. It must not be deliberately made clueless. Lane B is campaign-specific: it may use Open Scaffold or another structured workflow depending on the campaign being tested. The intended variable is the workflow system, not hidden information.

## Support thresholds

A future campaign may claim **repeatable workflow-value support candidate** only if the structured workflow lane, under equal conditions:

- does not materially lose mechanical correctness;
- improves or ties visual/product quality in most valid pairs;
- recovers better after context wipe;
- handles reviewer/scorer feedback more correctly;
- avoids stale/impossible requirement traps;
- gives better fresh-worker handoff;
- wastes fewer generations after plateau;
- keeps public claims safer and evidence more replayable.

If only evidence paperwork improves while product quality remains poor, the result is not output-quality support.
