# 2000m benchmark postmortem — why v3 is needed

Status: draft design packet. This postmortem summarizes benchmark-design failures surfaced by recent private A/B pilots. It is not a public contender result and does not make a workflow-superiority claim.

## Executive verdict

The current 2000m benchmark is useful as a deterministic mechanical harness, but it is not yet a meaningful proof surface for product quality or workflow value.

The hard failure is this:

> A lane can score very high mechanically while producing an artifact that is visually and experientially poor.

That means the benchmark currently over-rewards scorer-shaped headless driver compliance and under-specifies the actual game/product surface.

## What current v0/v1/v2 measure

### v0 / v1

v0 and v1 mostly answer:

> Does the produced artifact implement a deterministic JSON-line game-logic driver that satisfies seeded acceptance checks?

That is a valid mechanical question. It is not the same as:

> Did the model produce a good game?

v1 added more ACs and quality scoring, but the outputs can still be headless protocol artifacts instead of coherent games. The result is a mechanical mountain that strong models can climb while the produced artifact remains weak as a product.

### v2

v2 added a workflow-resilience scorer/harness. It correctly separates workflow evidence from mechanical score and keeps the benchmark independent of any workflow framework.

But the private pilot exposed two remaining problems:

1. feedback parity can decide the result if one lane receives better workspace-local diagnostics than another;
2. even after feedback parity is corrected, visual/product quality remains underdefined and poor.

The corrected fair-feedback private rerun produced the conservative decision language: **no support** for a repeatable output-quality advantage by the tested Lane B/Open Scaffold configuration.

## What went wrong

### 1. Mechanical score is too dominant

A high mechanical score currently looks impressive even when the product is not. This creates false confidence. Mechanical correctness should be the substrate track, not the whole benchmark.

### 2. Visual quality is not a first-class contract

The benchmark can accept a produced artifact with no native screenshot, GIF, replay capture, playable surface, or visual rubric. A later neutral renderer can inspect JSON GameState, but that is a private proxy, not a submitted visual artifact.

This is why a lane can score well and still produce something the owner rejects visually.

### 3. Human taste was treated as a side note instead of a separate track

Human feel should not change mechanical rank, but if the benchmark is meant to evaluate game reproduction, product quality cannot be omitted. The answer is not to blend taste into mechanical score. The answer is to create a separate visual/product track with fixed captures, blind review, and explicit claim boundaries.

### 4. Workflow value was tested too weakly

A structured-workflow claim is not raw model intelligence. The claim is that work can become more recoverable, reviewable, governable, and handoffable when AI work spans attempts, feedback, context loss, and human review.

The current pilot only partially pressures those dimensions. It does not yet force enough context loss, reviewer ambiguity, regression traps, impossible/stale requirement handling, or fresh-worker recovery to discriminate structured workflow from ad hoc model self-organization.

### 5. Feedback locality was a confound

The first private run made Lane A artificially blind to exact scorer diagnostics while Lane B had richer local evidence. That measured feedback plumbing, not lane superiority. The fair rerun fixed this and changed the verdict to no support.

Future campaigns need hard parity: every lane gets the same class of exact feedback at the same logical phase boundary in a form it can actually use.

### 6. The benchmark risks testing the wrong workflow claim

A one-shot or short-loop coding puzzle asks:

> Can this model implement the target?

A workflow-resilience benchmark needs to ask:

> Can the work survive interruption, feedback, review, ambiguity, stale requirements, and later recovery without false claims?

Those are different questions.

## What was actually learned

- The deterministic driver protocol is still valuable.
- The visual/product surface is under-specified and must become rankable as a separate track.
- Workflow resilience must be tested with deliberate workflow pain, not inferred from better-looking paperwork.
- Feedback parity is a hard experimental control.
- The tested Lane B/Open Scaffold configuration did not prove output superiority in the fair rerun.
- The benchmark should be redesigned before any larger public evidence campaign.

## What remains unproven

The following claims remain unproven and must not be asserted from current runs:

- Do not claim Open Scaffold makes a model smarter.
- Do not claim Open Scaffold produces better games by default.
- Do not claim workflow score equals model ranking.
- Do not claim a high mechanical score means a good game.
- Do not claim private calibration/pilot runs are public contender results.
- Do not claim a prettier private proxy render proves causal workflow superiority.

## Postmortem conclusion

2000m should become a multi-track benchmark where mechanical correctness, product quality, workflow resilience, and evidence/recovery/governance are scored separately. The benchmark should stay workflow-agnostic, but it should deliberately stress failure modes that structured workflows claim to handle.

If a structured workflow has real value, v3 should let it show up through recovery, handoff, decision quality, and artifact improvement under equal conditions — not through benchmark coupling or hidden feedback advantages.
