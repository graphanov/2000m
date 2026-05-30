# Project README

TODO: replace this with your project overview.

## How this repo uses Open Scaffold

This repository uses Open Scaffold to keep AI-assisted work traceable in files instead of lost in chat history.

The basic loop is:

1. Define the mission in `MISSION.md`.
2. Create the current slice with `npx open-scaffold plan new <slug> --stage active` (or `osc plan new <slug> --stage active` if installed locally), then fill every TODO prompt. For a shaped starting point, use `--from-template bug-fix`; validate before execution with `npx open-scaffold plan validate <slug>`. Shell fallback: `cp .osc/plans/handoff-template.md .osc/plans/active/<slug>.md`.
3. Do the work and run the project checks.
4. Record evidence with `npx open-scaffold evidence new <slug>` (or `osc evidence new <slug>` if installed locally), then replace every TODO with real commands and results. Shell fallback: create the note manually under `.osc/releases/`.
5. If scope changes, run `npx open-scaffold amend <slug> --message "<what changed>"` (or `osc amend <slug> --message "<what changed>"`) and fill the amendment TODOs. When verified, close with `npx open-scaffold close <slug> --message "<what shipped>"` (or `osc close <slug> --message "<what shipped>"`). Shell fallback: `./amend.sh <slug>` and `./close.sh <slug> --message "<what shipped>"`.

## First useful commands

```bash
./bootstrap.sh
npx open-scaffold plan new my-first-task --stage active
# shell fallback: cp .osc/plans/handoff-template.md .osc/plans/active/my-first-task.md
./verify.sh --quick
./verify.sh --standard
npx open-scaffold evidence new my-first-task
npx open-scaffold amend my-first-task --message "scope changed"
npx open-scaffold close my-first-task --message "verified first task"
```

## Optional Dev Container

This standard scaffold includes `.devcontainer/` for teams that want the same Node.js, npm, git, and `osc` setup everywhere.
Open the repo in VS Code Dev Containers or Codespaces and run `osc status` after setup completes.
Containers are optional: the normal `npx open-scaffold ...` and shell fallback commands above still work on the host.

## Project status

- Mission: fill in `MISSION.md` before substantial work.
- Current work: keep one active plan under `.osc/plans/active/`.
- Evidence: add concise notes under `.osc/releases/` when meaningful slices close.

Keep this README about the downstream project. Link to Open Scaffold docs only when the methodology itself needs explanation.
