# Private pilot result-recording template

Status: Phase 2 template. This is not a completed result, not a contender row, and not a public proof claim.

Use this template for each `paired-private-pilot-v1` pair. Keep repo artifacts public-safe: no local absolute paths, personal identifiers, raw chat transcripts, or private account refs.

## Pair header

| Field | Value |
|---|---|
| Campaign | `paired-private-pilot-v1` |
| Pair id | `pilot-seed-<101|202|303>` |
| Task seed | `<101|202|303>` |
| Scenario | `v2/examples/workflow-resilience-pilot.scenario.json` |
| Scenario version | `1` |
| Enabled lanes | `A`, `B` |
| Lane C | Disabled; no controller evidence in this pilot |
| Model | `<same model for both lanes>` |
| Runtime | `<same runtime for both lanes>` |
| Generation cap | `3` |
| Scorer feedback budget | `1` |
| Reviewer feedback budget | `1` in the campaign fixture; record unused budget equally if no reviewer packet is used |
| Context wipe timing | After first scored generation and before final recommendation |
| Visual seeds | `1101`, `2202`, `3303` |
| Capture windows | early-game, mid-run obstacle field, post-feedback rerun |

## Fairness checklist

- [ ] Same model in both lanes.
- [ ] Same runtime in both lanes.
- [ ] Same task seed in both lanes.
- [ ] Same generation cap in both lanes.
- [ ] Same prompt budget in both lanes.
- [ ] Same scorer feedback budget in both lanes.
- [ ] Same reviewer feedback budget in both lanes, including equal unused budget if no reviewer packet is used.
- [ ] Same context-wipe timing.
- [ ] Same visual seeds and capture windows.
- [ ] No hidden human repair hints in one lane only.
- [ ] No post-hoc hand edits disguised as model output.
- [ ] No scorer/scenario/campaign/rubric mutation after live result inspection.
- [ ] Lane C stayed disabled.

## Lane A — naked model baseline

Process boundary: no `.osc` state, Open Scaffold plans, run packets, eval envelopes, evolve logs, compact evidence, or `osc evolve analyze`.

### Lane A generation log

| Gen | Prompt ref | Final response ref | stdout ref | stderr ref | Build/test refs | v1 conformance JSON | Score summary | Feedback decisions | Visual package | Handoff/context wipe | Recommendation |
|---:|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `<ref>` | `<ref>` | `<ref>` | `<ref>` | `<ref>` | `<ref>` | `<score>` | `<accepted/rejected/deferred/inspect>` | `<ref or missing reason>` | `<summary/ref>` | `<continue/stop/redesign/inspect_scorer>` |
| 2 | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<score or not run>` | `<decisions or not run>` | `<ref or missing reason>` | `<summary/ref>` | `<continue/stop/redesign/inspect_scorer>` |
| 3 | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<score or not run>` | `<decisions or not run>` | `<ref or missing reason>` | `<summary/ref>` | `<continue/stop/redesign/inspect_scorer>` |

### Lane A required refs

- Prompt transcript refs:
  - generation 1: `<ref>`
  - generation 2: `<ref or not run>`
  - generation 3: `<ref or not run>`
- Final response refs:
  - generation 1: `<ref>`
  - generation 2: `<ref or not run>`
  - generation 3: `<ref or not run>`
- Runtime stdout/stderr refs: `<refs>`
- Build/test/scorer output refs: `<refs>`
- v1 conformance JSON refs: `<refs>`
- Plain trajectory ref: `<ref>`
- Handoff summary ref: `<ref>`
- Visual package refs: `<refs or missing reason>`
- v2 run record ref: `<ref>`
- v2 result ref: `<ref>`

## Lane B — Open Scaffold ledger/analyze lane

Process boundary: Open Scaffold is allowed as a ledger/analyze loop only. Do not claim controller behavior.

### Lane B generation log

| Gen | Prompt ref | Final response ref | stdout ref | stderr ref | Build/test refs | v1 conformance JSON | Open Scaffold refs | Score summary | Feedback decisions | Visual package | Handoff/context wipe | Recommendation |
|---:|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `<ref>` | `<ref>` | `<ref>` | `<ref>` | `<ref>` | `<ref>` | `<run packet/eval/analyze/compact evidence refs>` | `<score>` | `<accepted/rejected/deferred/inspect>` | `<ref or missing reason>` | `<summary/ref>` | `<continue/stop/redesign/inspect_scorer>` |
| 2 | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<refs or not run>` | `<score or not run>` | `<decisions or not run>` | `<ref or missing reason>` | `<summary/ref>` | `<continue/stop/redesign/inspect_scorer>` |
| 3 | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<ref or not run>` | `<refs or not run>` | `<score or not run>` | `<decisions or not run>` | `<ref or missing reason>` | `<summary/ref>` | `<continue/stop/redesign/inspect_scorer>` |

### Lane B required refs

- Active/done plan refs: `<refs>`
- Run packet refs: `<refs>`
- Evaluation envelope refs: `<refs>`
- `osc evolve analyze` refs: `<refs>`
- Compact evidence refs: `<refs>`
- Prompt transcript refs: `<refs>`
- Final response refs: `<refs>`
- Runtime stdout/stderr refs: `<refs>`
- Build/test/scorer output refs: `<refs>`
- v1 conformance JSON refs: `<refs>`
- Handoff summary ref: `<ref>`
- Visual package refs: `<refs or missing reason>`
- v2 run record ref: `<ref>`
- v2 result ref: `<ref>`

## v1 conformance to v2 run-record mapping

Save v1 conformance JSON for every scored generation. The v2 scorer consumes only the single JSON named in `artifact.v1ConformanceJson`, so pin that field to the final selected comparison generation for the lane: latest scored generation at cap, or latest scored generation at an explicit early `stop`, `redesign`, or `inspect_scorer` recommendation. Apply the same rule to both lanes and keep non-pinned generation JSONs as evidence/trajectory refs.

| v1/scorer artifact | v2 run-record field |
|---|---|
| Produced artifact repo/ref | `artifact.repoOrPath`, phase output `artifact-ref` |
| Produced artifact commit/digest | `artifact.commitOrDigest` |
| Build command for pinned generation | `artifact.buildCommand`, phase output `build-command` |
| Score command for pinned generation | `artifact.scoreCommand`, phase output `score-command` |
| Pinned v1 conformance JSON ref | `artifact.v1ConformanceJson`, phase output `conformance-json`, evidence kind `conformance-json` |
| Other generation v1 conformance JSON refs | evidence refs labeled `conformance-json`; do not overwrite the pinned artifact input |
| Scorer feedback packet | `phases[].feedbackResponses[]` |
| Handoff summary | context-wipe phase output `handoff-summary` |
| Stop/continue/redesign/inspect decision | `finalRecommendation.decision`, final phase output `stop-recommendation` |

## v2 run-record skeleton

Fill this shape for each lane after the lane evidence is complete. Use public-safe refs. Do not use this fenced block as a committed JSON file without replacing placeholders and validating against [`../run-record.schema.json`](../run-record.schema.json).

```json
{
  "schemaVersion": "2000m.v2.run-record.v1",
  "scenarioId": "workflow-resilience-pilot",
  "scenarioVersion": 1,
  "entrant": {
    "label": "paired-private-pilot-<seed>-lane-<a-or-b>",
    "processType": "single-model",
    "notes": "Private pilot lane record; not a public contender result."
  },
  "artifact": {
    "repoOrPath": "<public-safe artifact ref>",
    "commitOrDigest": "<commit or digest>",
    "buildCommand": "<pinned generation build command>",
    "scoreCommand": "<pinned generation score command>",
    "v1ConformanceJson": "<public-safe pinned v1 conformance JSON ref>"
  },
  "phases": [
    {
      "phaseId": "initial-build",
      "outputs": {
        "artifact-ref": "<artifact ref>",
        "build-command": "<pinned generation build command>",
        "score-command": "<pinned generation score command>",
        "conformance-json": "<pinned v1 conformance JSON ref>"
      }
    },
    {
      "phaseId": "scorer-feedback",
      "outputs": {
        "feedback-response": "<summary ref>",
        "conformance-json": "<latest scored or pinned v1 conformance JSON ref>",
        "evidence-ref": "<feedback evidence ref>"
      },
      "feedbackResponses": [
        {
          "feedbackId": "<feedback id>",
          "decision": "<accepted|rejected_with_reason|needs_scorer_inspection|deferred>",
          "rationale": "<evidence-tied rationale>",
          "evidenceRef": "<evidence ref>"
        }
      ]
    },
    {
      "phaseId": "context-wipe",
      "outputs": {
        "handoff-summary": "<fresh-worker handoff summary>",
        "evidence-ref": "<handoff evidence ref>"
      }
    },
    {
      "phaseId": "trap-probe-only",
      "outputs": {
        "feedback-response": "<probe-only handling summary>",
        "stop-recommendation": "<continue|stop|redesign|inspect_scorer>",
        "evidence-ref": "<evidence ref>"
      }
    },
    {
      "phaseId": "trap-stale",
      "outputs": {
        "feedback-response": "<stale requirement handling summary>",
        "stop-recommendation": "<continue|stop|redesign|inspect_scorer>",
        "evidence-ref": "<evidence ref>"
      }
    },
    {
      "phaseId": "trap-impossible",
      "outputs": {
        "feedback-response": "<impossible requirement handling summary>",
        "stop-recommendation": "<continue|stop|redesign|inspect_scorer>",
        "evidence-ref": "<evidence ref>"
      }
    },
    {
      "phaseId": "final-decision",
      "outputs": {
        "stop-recommendation": "<continue|stop|redesign|inspect_scorer>",
        "evidence-ref": "<final decision evidence ref>"
      }
    }
  ],
  "finalRecommendation": {
    "decision": "<continue|stop|redesign|inspect_scorer>",
    "rationale": "<bounded evidence-tied rationale>"
  },
  "evidence": [
    {
      "label": "produced artifact",
      "ref": "<public-safe artifact ref>",
      "kind": "repo"
    },
    {
      "label": "v1 conformance",
      "ref": "<public-safe conformance JSON ref>",
      "kind": "conformance-json"
    },
    {
      "label": "handoff summary",
      "ref": "<public-safe handoff summary ref>",
      "kind": "summary"
    }
  ]
}
```

For Lane B, set `entrant.processType` to `workflow-system` or another schema-allowed value that accurately describes the actual process used. For Lane A, use `single-model` unless the actual execution process requires a different schema-allowed label.

## Visual package checklist

For each lane:

- [ ] Build ref recorded.
- [ ] Visual seeds recorded.
- [ ] Capture command recorded.
- [ ] Screenshots recorded for predeclared windows.
- [ ] GIF or replay capture recorded.
- [ ] Replay log or frame metadata recorded.
- [ ] Rubric record recorded.
- [ ] Blind-label map sealed before review.
- [ ] Missing or failed capture reason recorded, if applicable.

## Pair comparison

Keep tracks separate.

| Track | Lane A evidence | Lane B evidence | Pair-level finding | Claim boundary |
|---|---|---|---|---|
| Mechanical score | `<refs/scores>` | `<refs/scores>` | `<delta/tie/no valid pair>` | Mechanical score is not model intelligence or visual quality |
| Visual/artifact quality | `<refs/rubric/blind review>` | `<refs/rubric/blind review>` | `<delta/tie/blocked>` | Visual claim blocked if packages differ/missing |
| Trajectory quality | `<trajectory ref>` | `<trajectory/evolve analyze refs>` | `<delta/tie>` | Do not infer causality from one pair |
| Evidence/recovery/handoff | `<handoff ref>` | `<handoff/compact evidence refs>` | `<delta/tie>` | Fresh-agent recovery is separate from artifact score |

## Decision rule notes

Record a pair-level note here, but reserve the campaign-level decision until all valid pairs are complete. A single pair cannot establish repeatability.

Select exactly one preliminary pair note:

- [ ] No support in this pair.
- [ ] Directional pair-level signal; causality unproven.
- [ ] Possible workflow-value support signal requiring all-pair aggregation.
- [ ] Invalid/calibration only.

Rationale:

```text
<bounded rationale tied to mechanical, visual/artifact, trajectory, and evidence/recovery refs>
```

## Commit/local-private decision

Commit-ready after scan and owner approval:

- [ ] Sanitized v2 run record.
- [ ] Sanitized v2 scorer result.
- [ ] Sanitized pair comparison summary.
- [ ] Sanitized visual package metadata/rubric summary if public-safe.

Keep local/private unless separately approved and sanitized:

- [ ] Raw prompts.
- [ ] Raw stdout/stderr.
- [ ] Raw chat transcripts.
- [ ] Blind-label mapping before review is sealed.
- [ ] Local absolute paths.
- [ ] Produced-game implementation work records that belong in produced-output repos.
- [ ] Unreviewed screenshots/GIFs with private/proprietary content.
