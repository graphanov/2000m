# 2000m v3 scoring model draft

Status: executable-spec draft. This model defines separated tracks, blockers, and claim boundaries for future implementation. It does not run a new evidence campaign.

## Track separation

v3 reports four tracks separately:

1. `mechanical` — deterministic game-logic correctness.
2. `visual` — native visual/playable artifact quality.
3. `workflow` — recovery, feedback routing, regression protection, impossible/stale handling, and handoff.
4. `evidence` — replayability, public safety, compact record quality, and claim-boundary hygiene.

No single composite may hide these tracks. If a UI later renders a convenience summary, it must preserve every component and state that workflow score is not model intelligence.

Token/cost telemetry is a separate trajectory-efficiency input, not a
mechanical score component. A campaign may report total tokens by
seed/lane/generation, tokens until first ranked mechanical ceiling, and token
delta between lanes only when those fields are source-labeled and kept out of
mechanical pass counts.

## Discrimination backlog

Future frozen protocols should add neutral tracks that make ties less likely
without coupling the benchmark to any workflow product:

- trajectory efficiency: generations until first valid ceiling, regressions,
  and avoidable no-op generations;
- token/cost efficiency: source-labeled total tokens and cost only when a
  versioned cost source exists;
- context-wipe/handoff: fresh-agent recovery from compact records without chat
  history;
- plateau detection: stop/redesign recommendations when attempts stop moving;
- impossible/stale AC detection: scorer or requirement inspection instead of
  blind retries;
- reviewer feedback resilience: exact-diagnostic parity plus correct handling
  of reviewer corrections and regression traps;
- visual/product track: native capture validity as a prerequisite before any
  visual ranking;
- hidden/regression traps: public-safe trap classes that are independent of
  Open Scaffold, lane names, directory shape, or command vocabulary.

## Draft track fields

### Mechanical

- `mechanical.ranked`
- `mechanical.protocolVersion`
- `mechanical.passCount`
- `mechanical.totalAcs`
- `mechanical.compositeScore`
- `mechanical.determinism.pass`
- `mechanical.failedAcs[]`
- `mechanical.hiddenChallengeSummary`
- `mechanical.regressionSummary`
- `mechanical.resultJsonRef`

### Visual

- `visual.ranked`
- `visual.blockReason`
- `visual.visualPackageRef`
- `visual.captureDeterminism`
- `visual.rubricRecordRef`
- optional `visual.score`

### Workflow

- `workflow.contextWipeRecoveryScore`
- `workflow.feedbackDecisionScore`
- `workflow.regressionProtectionScore`
- `workflow.impossibleRequirementHandlingScore`
- `workflow.handoffScore`
- `workflow.finalRecommendation`
- `workflow.rationaleRefs[]`

### Evidence

- `evidence.replayable`
- `evidence.publicSafe`
- `evidence.privateRefsBlocked`
- `evidence.compactSummaryRef`
- `evidence.requiredRefsMissing[]`
- `evidence.claimBoundary`

### Runtime telemetry

- `runtimeTelemetry.source.kind`
- `runtimeTelemetry.source.ref`
- `runtimeTelemetry.tokens.total`
- nullable `runtimeTelemetry.tokens.input/output/cachedInput/reasoning`
- `runtimeTelemetry.tokens.unavailableFields[]`
- nullable `runtimeTelemetry.cost.estimatedUsd`
- `runtimeTelemetry.cost.unavailableReason`

## Blockers

| Blocker | Mechanical | Visual | Workflow | Evidence/claim impact |
|---|---|---|---|---|
| Missing native capture | unaffected | rank-blocked | unaffected unless scenario requires handling it | visual claims forbidden |
| Invalid driver enum/schema | failed/rank-blocked | unaffected | may affect feedback handling | exact reason required |
| Private/local path in public record | unaffected unless manifest depends on it | rank-blocked if visual ref | evidence fails | public claims blocked |
| Frozen protocol changed after live results | affected run calibration-only | affected run calibration-only | affected run calibration-only | public support blocked |
| Unsupported claim text | unaffected | unaffected | unaffected | evidence fails; public claims blocked |
| Evidence volume without decision quality | unaffected | unaffected | no score credit | evidence may still pass only if compact and useful |
| Higher token/cost use | unaffected | unaffected unless the workflow track predeclares efficiency scoring | must be reported separately | cannot rescue tied or worse output |

## Claim ceiling

| `claimBoundary` | Meaning |
|---|---|
| `calibration-only` | Fixture, smoke, or changed-protocol result; not contender evidence. |
| `no-support` | Valid campaign found no support for the tested claim. |
| `directional-signal` | Pattern worth more study; causality unproven. |
| `repeatable-workflow-value-support-candidate` | Predeclared thresholds met across valid pairs. |
| `public-benchmark-support` | Reserved for larger public-safe campaigns with independent review. |

Foundation fixtures in this PR must stay `calibration-only`.

## Anti-paperwork rule

Workflow and evidence tracks must not award score for evidence volume, framework names, command names, or directory shape. Score only generic recoverability, correctness of decisions, feedback parity, rerun evidence, and public-safe replayability.
