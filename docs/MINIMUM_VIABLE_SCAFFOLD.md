# Minimum Viable Scaffold

This repo has the minimum Open Scaffold loop needed for a human and an agent to answer:

In plain terms: this is the minimum viable scaffold for this project.

- What are we building?
- What is the current slice?
- How will we verify it?
- What evidence proves it closed?

## Five-step loop

1. Define the project mission in `MISSION.md`.
2. Create one active plan with `npx open-scaffold plan new <slug> --stage active` (or `osc plan new <slug> --stage active` if installed locally), then fill every TODO prompt with real acceptance criteria and verification. Shell fallback: copy `.osc/plans/handoff-template.md` into `.osc/plans/active/<slug>.md`.
3. Execute the work and run the project checks.
4. Run `./verify.sh --standard` before calling the slice done.
5. Record evidence with `npx open-scaffold evidence new <slug>` (or `osc evidence new <slug>`), replace every TODO with real results, then close the plan with `npx open-scaffold close <slug> --message "<what shipped>"` (or `osc close <slug> --message "<what shipped>"`). If scope changes, use `npx open-scaffold amend <slug> --message "<what changed>"` (or `osc amend <slug> --message "<what changed>"`). Shell fallback: `./amend.sh <slug>` and `./close.sh <slug> --message "<what shipped>"`.

## First-user checklist

- [ ] `MISSION.md` describes this project.
- [ ] `.osc/plans/active/` has one current plan before first verification.
- [ ] `.osc/plans/done/` contains only this project's completed work.
- [ ] `.osc/releases/` contains only this project's evidence notes.
- [ ] `./verify.sh --standard` passes.
