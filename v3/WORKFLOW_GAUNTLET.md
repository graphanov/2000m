# 2000m v3 workflow gauntlet

Status: frozen-candidate protocol. This is not live evidence and does not support any public result claim.

## Purpose

The gauntlet is the next discrimination layer after the private pilot exposed a weak test: final mechanical ceiling ties can hide visual blockers, wasted retries, and token overhead.

This protocol tests whether a workflow process creates measurable control under equal information. It stays product-neutral. The benchmark reads generic lane records, runtime telemetry, visual packages, scorer output, reviewer decisions, and handoff evidence.

## Lanes

The gauntlet has three required lane roles:

- `unstructured-baseline` gets the full rubric and same feedback, with generic notes only.
- `minimal-checklist-control` gets the full rubric and same feedback, plus a short plain-text checklist.
- `structured-workflow-candidate` gets the full rubric and same feedback, plus a pinned structured workflow selected before live results.

The minimal checklist lane matters. If a structured workflow only beats chaos but not a cheap checklist, the result is weak.

No lane receives hidden hints, richer diagnostics, extra reviewer budget, or post-hoc scorer edits. Lane ID `C` is forbidden in the v1 fixture.

## Scenario Families

Every frozen gauntlet must include these stressors:

- context wipe;
- reviewer feedback resilience;
- regression trap;
- stale or impossible AC handling;
- token pressure;
- handoff resume;
- visual native-capture prerequisite;
- plateau or no-op retry detection.

Each family must have public calibration fixtures and may have hidden regression checks. Hidden checks test general behavior, not command names, directory layout, or framework vocabulary.

## Tracks

Report tracks separately:

- mechanical;
- visual;
- workflow;
- trajectory;
- token-cost;
- evidence.

Mechanical correctness is a gate, not a place to hide other failures. Token cost is a predeclared token-cost track, not a mechanical score component. Evidence quality scores compact, replayable, decision-linked records; file count and raw volume give no credit.

## Stop Rules

The gauntlet requires a stop, inspect, or redesign recommendation when:

- two consecutive generations show no accepted metric improvement;
- a stale or impossible AC is supported by scorer evidence;
- visual package validation remains invalid after bounded repair attempts;
- token spend rises without a recorded repair hypothesis and expected measurable gain.

Continuing anyway is evidence against workflow control.

## Honest Win Condition

A structured workflow can earn a support-candidate result only if it:

- avoids material mechanical loss versus both controls;
- beats both controls on the predeclared workflow aggregate;
- beats at least one control on trajectory or token-cost without worse final output;
- keeps visual claims blocked until native capture is valid;
- passes feedback parity, scorer diagnostic parity, budget parity, and public-safety checks;
- reports the minimal checklist comparison and unavailable telemetry fields.

Anything less is no support or calibration-only, depending on the failure.

## Fixtures and smoke runner

The public frozen-candidate protocol fixture is:

```text
v3/examples/gauntlet/v3-workflow-gauntlet-v1.json
```

Validate it with:

```bash
python3 scripts/validate_v3_workflow_gauntlet.py
```

The public calibration-only smoke pack is:

```text
v3/examples/gauntlet/smoke/v3-workflow-gauntlet-smoke.fixture.json
```

Run it with:

```bash
python3 scripts/run_v3_workflow_gauntlet_smoke.py
```

The smoke pack is synthetic protocol calibration, not contender evidence. It
loads generic v3 run-record/result fixtures for lanes `U`, `M`, and `W`, covers
all required scenario families, and emits a deterministic no-support-style
summary where final mechanical outcome ties, visual ranking is blocked by invalid
native capture metadata, the structured candidate does not beat the minimal
checklist control, and token cost remains a separate track.
