# 2000m v3 design packet

Status: draft design packet. No v3 scorer, schema, campaign, or leaderboard semantics are implemented by this packet.

v3 is the proposed reset after the v1/v2 private pilot showed a serious benchmark gap: high mechanical conformance can coexist with poor game/artifact quality, and the current workflow track does not yet create a strong enough test of recovery, handoff, feedback routing, or governance.

## Core correction

2000m should not be one muddy composite. It should report four separate tracks:

1. **Mechanical correctness** — deterministic game-logic substrate.
2. **Product / visual artifact quality** — actual visible/playable artifact quality.
3. **Workflow resilience** — context loss, feedback routing, regression traps, stale/impossible requirements, and handoff.
4. **Evidence / recovery / governance quality** — reviewable work record, recovery anchors, decisions, claim boundaries, and public-safety hygiene.

The benchmark remains independent and workflow-agnostic. It may test failure modes that structured workflows are designed to handle, but it must not require Open Scaffold, `.osc` files, Open Scaffold CLI commands, or framework-specific schemas.

## Documents

- [`POSTMORTEM.md`](POSTMORTEM.md) — blunt postmortem of why the current benchmark result is not meaningful enough.
- [`DESIGN.md`](DESIGN.md) — v3 architecture and four-track scoring design.
- [`MANIFEST_AND_CAPTURE.md`](MANIFEST_AND_CAPTURE.md) — proposed v3 manifest, driver, capture, replay, and playable surface contract.
- [`WORKFLOW_RESILIENCE_PROTOCOL.md`](WORKFLOW_RESILIENCE_PROTOCOL.md) — context-wipe, reviewer packet, regression trap, impossible/stale requirement, scorer-feedback, and handoff protocol.
- [`WORKFLOW_GAUNTLET.md`](WORKFLOW_GAUNTLET.md) — frozen-candidate discrimination protocol for workflow, trajectory, token-cost, visual-blocker, and handoff claims.
- [`EVIDENCE_AND_CLAIMS.md`](EVIDENCE_AND_CLAIMS.md) — evidence/governance requirements, claim ladder, and anti-rigging guardrails.
- [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) — staged path from this design packet to v3 scorer/harness implementation.
- [`SPEC_GAPS_AND_DECISIONS.md`](SPEC_GAPS_AND_DECISIONS.md) — OMX advisory review gaps to resolve before executable v3 scorer work.
- [`2000m.driver.v3.md`](2000m.driver.v3.md) — draft exact v3 driver protocol.
- [`MECHANICAL_AC_SPEC.md`](MECHANICAL_AC_SPEC.md) — draft mechanical acceptance-criteria table.
- [`VISUAL_RUBRIC.md`](VISUAL_RUBRIC.md) — draft native capture and blind-review rubric.
- [`SCORING_MODEL.md`](SCORING_MODEL.md) — draft separated-track scoring and blocker model.
- [`SANDBOX_AND_RESOURCE_POLICY.md`](SANDBOX_AND_RESOURCE_POLICY.md) — draft scorer sandbox/resource policy.
- `*.schema.json` plus `examples/valid/`, `examples/invalid/`, `examples/workflow/`, and `examples/gauntlet/` — v3 schema foundation fixtures validated by `scripts/validate_v3_schemas.py`, `scripts/validate_v3_workflow_scenarios.py`, and `scripts/validate_v3_workflow_gauntlet.py`.

## Claim boundary

v3 is not a plan to make Open Scaffold win. It is a plan to make 2000m test structured-workflow claims generally while staying fair to naked, ad hoc, scripted, human-operated, Open Scaffold, and future workflow contenders. Open Scaffold is a possible campaign-specific lane, not a benchmark default.

Allowed future claim shape:

> Under the v3 scenario, entrant X scored Y on mechanical correctness, Z on visual artifact quality, A on workflow resilience, and B on evidence/governance.

Not allowed:

> Open Scaffold is better because the benchmark was written around Open Scaffold.

## Immediate use

Use this packet as the specification guide for a later v3 design/implementation PR. Do not mutate v2 scorer/campaign semantics based on the private pilot unless affected runs are labeled calibration-only.
