# AGENTS.md

This repository uses Open Scaffold. It is not the Open Scaffold product repository.

## Project facts

- `MISSION.md` is the source of truth for what this project is trying to do.
- `.osc/plans/` stores immutable plans in `active/`, `backlog/`, `done/`, and `blocked/`.
- `.osc/RULES.md` is the compact rule sheet. Re-read it before structural changes.
- `.osc/plans/WORKFLOW.md` explains how plan files move between stages.
- `.osc/releases/` stores concise evidence/release notes for meaningful closed slices.
- `verify.sh` checks scaffold health before claiming work is done.

## Operating rules

1. Read `MISSION.md` before meaningful work. If the mission is still unset, help the owner define it first.
2. Check `.osc/plans/active/` before starting new work. Continue active work unless the owner says otherwise.
3. Do not silently rewrite committed plans. New information becomes an amendment, follow-up plan, or evidence note.
4. Keep work small enough to review. One plan should map to one clear slice.
5. Verify against acceptance criteria, then record evidence before closing a plan.
6. Treat chat and agent transcripts as working context, not durable truth. Promote important facts into repo files.

## First task bootstrap

If this is a new repo, ask the owner:

1. What is this project in one sentence?
2. What should it achieve first?
3. What should it explicitly not do yet?

Then update `MISSION.md` and create the first active plan with `npx open-scaffold plan new <slug> --stage active`, `osc plan new <slug> --stage active`, or by copying `.osc/plans/handoff-template.md`.
