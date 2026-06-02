# 2000m v3 sandbox and resource policy draft

Status: executable-spec draft. This policy defines expected scorer/harness execution boundaries for future v3 implementation. It does not run a live pilot.

## Purpose

v3 needs deterministic, fair, and public-safe execution. Resource policy is part of the frozen campaign contract so entrants cannot gain hidden advantage through environment differences.

## Draft default policy

| Surface | Default |
|---|---|
| Mechanical driver timeout | 30 seconds per scorer command batch unless campaign overrides before freeze. |
| Capture timeout | 60 seconds per seed/window capture unless campaign overrides before freeze. |
| Build timeout | 5 minutes per artifact build in calibration smokes. |
| Network during scoring | Disabled. |
| Network during pre-run setup | Only allowlisted package installation before protocol freeze. |
| Filesystem | Scorer reads artifact root and benchmark fixtures; public records must use repo-relative refs. |
| Build cache | Allowed only if cache policy is identical across lanes and declared. |
| Stdout/stderr limits | Capture compact refs; raw logs remain private unless scanned and explicitly allowed. |
| Secrets | No secrets required for scoring or capture. |

## Command failure handling

Every failed command record must include:

- command label;
- public-safe command string or argv;
- exit code or timeout marker;
- stdout/stderr refs or compact redacted summary;
- whether the failure blocks mechanical, visual, workflow, or evidence track ranking.

A command failure must not be silently converted into a pass, and it must not trigger scorer/rubric changes after live output inspection unless the affected run becomes calibration-only.

## Path policy

Public records must reject:

- local absolute paths;
- home-directory paths;
- Windows drive/UNC local paths;
- `file://` URLs;
- traversal refs such as `../`;
- private repo paths;
- owner identity or private Discord IDs.

Use repo-relative refs or public URLs only.

## Lane fairness

For paired campaigns, both lanes receive the same resource policy:

- same model/runtime label;
- same generation cap;
- same prompt and feedback budget;
- same exact scorer diagnostics;
- same context-wipe phase;
- same visual seeds/windows;
- same reviewer budget;
- same network/cache policy;
- explicit disclosure of any human intervention.

## Future implementation notes

The first v3 scorer implementation should start with sandbox policy validation and fixture-based smokes. It should not run a live contender campaign until schemas, scenarios, visual package validation, and separated result rendering are in place.
