# 2000m v3 spec gaps and decision backlog

Status: draft design packet. This file records advisory review gaps to address before v3 becomes an executable benchmark contract.

## Verdict from advisory review

The v3 packet is a strong specification guide / PRD, but not yet an executable benchmark spec. It needs schemas, exact protocol fields, scoring semantics, validators, fixtures, visual-rubric rules, sandbox/resource policy, and result-renderer support before any new pilot can be treated as evidence.

## Must preserve

- Four separate tracks; do not collapse them into a flattering composite.
- Mechanical correctness is the substrate only; high AC score does not mean good game.
- Native capture/playable output is required for visual/product claims.
- The benchmark remains workflow-agnostic: no `.osc`, framework command names, or framework-specific required schemas.
- Claim ladder stays conservative: calibration-only, no support, directional signal, support candidate.
- Feedback parity and freeze rules are hard controls.
- Public-safety scan blocks private refs and unsupported claims.

## Missing executable spec pieces

### 1. Exact v3 driver protocol

Needed before scorer implementation:

- request/response schemas for each command;
- GameState fields and units;
- allowed enum values;
- error semantics;
- replay checksum rules;
- protocol versioning;
- how hidden/public challenge sets are declared and isolated.

### 2. Mechanical AC table

Needed before Track 1 scorer:

- AC IDs;
- pass/fail thresholds;
- quality subfields if any;
- hidden/public fixture split;
- denominator and skip rules;
- composite formula;
- regression-stability checks;
- invalid enum/schema poison handling.

### 3. Visual scoring rubric

Needed before Track 2 can rank:

- viewport and resolution;
- FPS and capture length;
- standard windows and window definitions;
- reviewer count;
- blind-rating form;
- weights/tie handling;
- how owner taste differs from public human preference;
- how diagnostic visual metrics feed the score.

### 4. Capture trust model

Needed to avoid fake screenshots:

- artifact digest recorded before capture;
- scorer-controlled seeds and input streams;
- replay checksum linking GameState to frames;
- repeated capture determinism check;
- frame checksum;
- failure handling and rank blockers.

### 5. Workflow scoring semantics

Needed before Track 3 scorer:

- point values for context wipe, reviewer injection, regression trap, stale/impossible requirement handling, and handoff;
- automatic vs human-judged boundaries;
- negative/golden fixtures;
- anti-paperwork rules so evidence volume alone does not score.

### 6. Campaign/statistics rules

Needed before private pilots:

- minimum sample sizes;
- tie handling;
- invalidation rules;
- effect thresholds;
- claim ceiling by campaign size;
- how missing visual packages affect pair validity.

### 7. Sandbox/resource policy

Needed before live scorer/harness runs:

- command timeouts;
- network policy;
- filesystem isolation;
- build cache policy;
- allowed tools;
- stdout/stderr size limits;
- command failure handling.

### 8. Asset/legal validation

Needed before public results:

- manifest attestation for original homage assets;
- practical checks/blockers for copied/extracted SkiFree assets;
- public wording that avoids replica/asset-reproduction framing.

## Wording risks to avoid

- Do not say v3 is designed for Open Scaffold.
- Say it tests structured workflow claims generally.
- Keep the fair-rerun conclusion visible: Open Scaffold did not prove output superiority.
- Treat Lane B/Open Scaffold as campaign-specific when that campaign is testing Open Scaffold, not as a benchmark default.
- Do not say workflow-value support unless visual/product quality also ties or improves under frozen equal conditions.
- If only the owner rated visuals, call it owner taste, not general human preference.
- Do not call private/proxy visual renders public proof.

## Next design actions

The foundation slice adds draft versions of these artifacts plus schema fixtures. Before scorer behavior changes, refine them against review feedback and keep the protocol frozen for any campaign that uses them:

1. `v3/2000m.driver.v3.md` — exact driver protocol.
2. `v3/MECHANICAL_AC_SPEC.md` — concrete mechanical AC table.
3. `v3/VISUAL_RUBRIC.md` — native capture and blind review rubric.
4. `v3/SCORING_MODEL.md` — separated track scoring and blockers.
5. `v3/SANDBOX_AND_RESOURCE_POLICY.md` — scorer execution policy.
6. v3 schemas and validators.
7. v3 golden/negative fixtures.

Only after those exist should a v3 implementation PR start changing scorer behavior.
