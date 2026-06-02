# 2000m v3 paired campaign protocol freeze

Status: frozen calibration protocol fixture. This runbook is for Gate 5/6 smokes and small private pilots only. It is not a public contender result, not public benchmark support, and not proof about any workflow system.

## Frozen comparison shape

- Lanes: A/B only. Lane C/controller behavior is not implemented and is not part of this protocol.
- Claim ceiling for the first private pilot smoke: private directional signal or calibration-only.
- Track reporting: mechanical, visual, workflow, and evidence tracks remain separate.
- Native capture is required before visual ranking; missing capture blocks visual claims without changing mechanical pass counts.
- Both lanes receive the full public rubric, the same exact scorer diagnostics, the same reviewer packet, the same visual seeds/windows, and the same context-wipe boundary.
- Raw private pilot artifacts stay outside the public benchmark repository. Public files may contain only scrubbed calibration fixtures and protocol definitions.

## Lane A prompt — unstructured baseline

```text
You are Lane A for the 2000m v3 paired calibration campaign. Build or repair the artifact using the full public v3 rubric and exact scorer feedback provided in this workspace. Do not use the campaign-specific structured workflow under test. Keep your own notes if useful, but no workflow framework or hidden lane-specific hints are required or scored. Produce a generic run record, mechanical result, visual capture package or visual blocker, feedback decision table, public-safety scan, and final handoff. Treat context wipe after the first scored generation as binding: after that point, continue only from repo state, public spec, run record, scorer feedback, visual status, and reviewer packet. Keep claims conservative: calibration/private directional signal only.
```

## Lane B prompt — pinned workflow lane

```text
You are Lane B for the 2000m v3 paired calibration campaign. You receive the same public v3 rubric, exact scorer feedback, reviewer packet, visual seeds/windows, and context-wipe boundary as Lane A. If the campaign-specific workflow-under-test is enabled in the frozen protocol, use only the pinned version and commit recorded there; otherwise behave as a generic workflow-system lane with no hidden information. Do not assume any hidden scorer data. Produce the same generic run record, mechanical result, visual capture package or visual blocker, feedback decision table, public-safety scan, and final handoff as Lane A. Keep claims conservative: calibration/private directional signal only.
```

## Frozen phase boundaries

1. Phase 0 protocol freeze before either lane runs.
2. Phase 1 first scored generation for both lanes.
3. Context wipe immediately after Phase 1 for both lanes.
4. Identical reviewer packet delivered after context wipe.
5. Regression-trap repair generation.
6. Stale/impossible/probe-only classification.
7. Final handoff from evidence only.

## Predeclared thresholds

- Do not report public support from this smoke or any one small private pilot.
- Directional signal requires no material mechanical loss, visual rank not blocked in both lanes, and better or tied workflow recovery/feedback/handoff on most pairs.
- If visual capture is missing in either lane, visual/product claims are blocked for that pair.
- If protocol, scorer, rubric, seeds, feedback packet, or context-wipe boundary changes after live outputs, affected rows are calibration-only.
