# 2000m v3 evidence and claim rules

Status: draft design packet. This document proposes evidence/recovery/governance and claim-boundary rules for v3.

## Evidence principle

A v3 result should be understandable without chat history. A fresh evaluator should be able to reproduce the main claims from public-safe artifacts and machine-readable records.

Evidence/recovery/governance quality is not a consolation prize for weak output. It is a separate track.

## Required evidence refs

For every scored generation in every lane, record:

- pair/campaign ID;
- task seed;
- lane ID;
- generation number;
- model/runtime label;
- prompt ref;
- final model response ref;
- stdout/stderr refs;
- runtime token telemetry when the runtime exposes reliable usage lines;
- files changed or artifact refs;
- build command and result;
- test command and result;
- mechanical score command and JSON ref;
- capture command and visual refs;
- scorer/reviewer feedback packet refs;
- feedback decisions;
- context-wipe/handoff status when applicable;
- final recommendation: continue, stop, redesign, inspect_scorer;
- rationale tied to evidence refs.

## Public-safety rules

Public result records must not include:

- owner personal identity;
- local user paths;
- private repo paths;
- Discord IDs or thread IDs;
- unprocessed chat/log dumps;
- unredacted logs with secrets;
- private/local evidence refs;
- unsupported score-win, workflow-superiority, adoption, or model-intelligence claims.

If evidence is private or local-only, it may support private analysis but must block public ranking/proof claims for the affected track.

## Claim ladder

### Invalid / calibration-only

Use when:

- scorer/campaign/rubric changed after live result inspection;
- feedback parity failed;
- visual seeds/windows changed after looking;
- hidden human repair hints favored one lane;
- artifacts were hand-edited without disclosure;
- required evidence is missing for the claimed track.

Allowed wording:

> Calibration-only; not valid paired evidence.

### No support

Use when:

- structured workflow does not materially beat/tie on the relevant tracks;
- visual/product quality is broadly failed;
- evidence quality improved but output quality did not;
- mechanical and visual results disagree without enough samples;
- sample is too small or noisy.

Allowed wording:

> No support for the tested output-quality/workflow-value claim under this campaign.

### Directional signal / causality unproven

Use when:

- one lane looks better on some tracks but the effect is inconsistent;
- sample size is small;
- human visual preference and mechanical score disagree;
- a private proxy artifact suggests a pattern but native capture is missing;
- confounds cannot be fully excluded.

Allowed wording:

> Directional signal worth a larger frozen campaign; causality unproven.

### Repeatable workflow-value support candidate

Use only when predeclared thresholds are met across valid pairs:

- no material mechanical loss;
- better or tied visual/product track;
- better context-wipe recovery;
- better feedback routing;
- better stale/impossible requirement handling;
- better handoff quality;
- equal information and budget;
- public-safe replayable evidence.

Allowed wording:

> Repeatable workflow-value support candidate under this v3 scenario.

### Externally reviewed support

Reserve for larger campaigns with multiple seeds, frozen scenarios, public-safe evidence, and independent review. Do not use for private pilots.

## Anti-rigging guardrails

The benchmark may stress workflow failure modes, but it must not be Open Scaffold-specific.

Forbidden:

- requiring `.osc` files;
- awarding points for Open Scaffold command names;
- giving Lane B richer feedback;
- hiding the rubric from Lane A;
- requiring framework-specific schemas;
- designing reviewer packets that only Open Scaffold terminology can parse;
- declaring workflow victory from evidence volume alone;
- calling generated paperwork a product-quality improvement.

Required:

- same public spec for all lanes;
- same exact scorer diagnostics;
- same reviewer packet;
- same context-wipe timing;
- same visual seeds/windows;
- generic run-record schema;
- track-separated scoring;
- explicit missing-evidence blockers;
- frozen decision rules.

## Evidence compaction

v3 should avoid raw log dumps. Required public records should use compact summaries with stable refs to raw evidence where allowed.

A compact evidence summary should include:

```text
what changed
what passed
what failed
what was accepted/rejected/deferred
what remains blocked
what can be rerun
what cannot be claimed
```

Raw logs can remain private/local unless a public result specifically requires them and they pass safety scanning.

## Final report shape

A v3 final report should always separate:

```text
mechanical correctness
product / visual artifact quality
workflow resilience
trajectory quality
token / cost efficiency
evidence / recovery / governance quality
claim boundary
```

Do not collapse these into a single feel-good narrative.

When token/cost telemetry is present, the report should show:

```text
tokens per generation
total tokens by seed/lane
total tokens by lane
tokens until first ranked mechanical ceiling
tokens per passed-AC improvement where meaningful
Lane B vs Lane A token delta
unavailable token/cost subfields
```

If the runtime exposes only total tokens, record only total tokens. Do not infer
input/output/cached/reasoning splits or dollar cost from logs that do not carry
those fields.
