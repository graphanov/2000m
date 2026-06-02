# 2000m Leaderboard

This is the rendered scoreboard for the 2000m judge. Mechanical columns determine
rank from the track-labeled AC-pass trajectory; the human-feel column is
explicitly **operator taste, not score**, and is never blended into the
mechanical rank.

## v0 mechanical leaderboard

| Mechanical rank | Model | Produced repo | AC trajectory | Final ACs | Gen-to-playable (AC1–AC8) | Gen-to-yeti (AC12–AC14) | Monotonic | Human feel (operator taste, not score) | Evidence |
|---:|---|---|---:|---:|---:|---:|:---:|---|---|
| 1 | placeholder-model | `graphanov/2000m-placeholder-model` | 2→4→7→9 | 9/16 | 3 | — | yes | not evaluated — placeholder row | `results/example-placeholder-final.json` |

## v2 workflow-resilience result spine

These rows exercise the v2 result schema and scorer behavior. Calibration
fixtures are not contender results, not model rankings, and not evidence that
any workflow framework is superior.

| Scenario | Entrant | Process | Ranked? | Composite | Artifact | Feedback | Recovery | Stop | Replay | Result | Run record | Claim boundary |
|---|---|---|:---:|---:|---:|---:|---:|---:|---:|---|---|---|
| workflow-resilience-pilot v1 | calibration-good-record | scripted-agent-loop | yes | 80 | 50 | 100 | 100 | 100 | 100 | [`v2/examples/results/pilot-good-result.json`](../v2/examples/results/pilot-good-result.json) | [`v2/examples/pilot-good-run-record.json`](../v2/examples/pilot-good-run-record.json) | calibration fixture, not a contender result |
| workflow-resilience-pilot v1 | calibration-weak-ranked-record | scripted-agent-loop | yes | 72 | 50 | 100 | 100 | 60 | 80 | [`v2/examples/results/pilot-weak-ranked-result.json`](../v2/examples/results/pilot-weak-ranked-result.json) | [`v2/examples/pilot-weak-ranked-run-record.json`](../v2/examples/pilot-weak-ranked-run-record.json) | calibration fixture, ranked but intentionally weak |
| workflow-resilience-pilot v1 | calibration-wrong-stop-record | scripted-agent-loop | yes | 65 | 50 | 100 | 100 | 0 | 100 | [`v2/examples/results/pilot-wrong-stop-result.json`](../v2/examples/results/pilot-wrong-stop-result.json) | [`v2/examples/pilot-wrong-stop-run-record.json`](../v2/examples/pilot-wrong-stop-run-record.json) | calibration fixture with wrong stop decision |
| workflow-resilience-pilot v1 | calibration-missing-output-record | scripted-agent-loop | no | 65 | 50 | 100 | 0 | 100 | 100 | [`v2/examples/results/pilot-missing-output-result.json`](../v2/examples/results/pilot-missing-output-result.json) | [`v2/examples/pilot-missing-output-run-record.json`](../v2/examples/pilot-missing-output-run-record.json) | rank-block fixture: missing required output |
| workflow-resilience-pilot v1 | calibration-private-path-record | scripted-agent-loop | no | 50 | 0 | 100 | 100 | 100 | 0 | [`v2/examples/results/pilot-private-path-result.json`](../v2/examples/results/pilot-private-path-result.json) | [`v2/examples/pilot-private-path-run-record.json`](../v2/examples/pilot-private-path-run-record.json) | rank-block fixture: private/local scorer input |

## v3 separated-track calibration spine

These rows exercise v3 fixture handling only. They are calibration/non-contender
rows, not public benchmark support, not model rankings, and not proof
about any workflow system. Mechanical, visual, workflow, and evidence fields remain
separate so no composite can hide blocked or weak tracks.

| Scenario | Lane | Process | Mechanical | Visual | Workflow avg | Evidence safe? | Result | Run record | Claim boundary |
|---|---|---|---:|---|---:|:---:|---|---|---|
| v3-workflow-calibration | A | scripted-agent | 24/24 | ranked | 100 | yes | [`v3/examples/workflow/results/complete-ranked.result.json`](../v3/examples/workflow/results/complete-ranked.result.json) | [`v3/examples/workflow/run-records/complete-ranked.run-record.json`](../v3/examples/workflow/run-records/complete-ranked.run-record.json) | calibration fixture, non-contender result |
| v3-workflow-calibration | A | scripted-agent | 24/24 | ranked | 57 | yes | [`v3/examples/workflow/results/weak-ranked.result.json`](../v3/examples/workflow/results/weak-ranked.result.json) | [`v3/examples/workflow/run-records/weak-ranked.run-record.json`](../v3/examples/workflow/run-records/weak-ranked.run-record.json) | calibration fixture, ranked but intentionally weak |
| v3-workflow-calibration | A | scripted-agent | 24/24 | blocked: missing-native-capture-or-playable-surface | 90 | yes | [`v3/examples/workflow/results/missing-visual-rank-blocked.result.json`](../v3/examples/workflow/results/missing-visual-rank-blocked.result.json) | [`v3/examples/workflow/run-records/missing-visual-rank-blocked.run-record.json`](../v3/examples/workflow/run-records/missing-visual-rank-blocked.run-record.json) | calibration fixture, visual rank-blocked |
| v3-workflow-calibration | A | scripted-agent | 24/24 | ranked | 56 | yes | [`v3/examples/workflow/results/wrong-feedback-routing.result.json`](../v3/examples/workflow/results/wrong-feedback-routing.result.json) | [`v3/examples/workflow/run-records/wrong-feedback-routing.run-record.json`](../v3/examples/workflow/run-records/wrong-feedback-routing.run-record.json) | calibration fixture, wrong feedback routing |
| v3-workflow-calibration | A | scripted-agent | 24/24 | ranked | 76 | yes | [`v3/examples/workflow/results/wrong-stop-decision.result.json`](../v3/examples/workflow/results/wrong-stop-decision.result.json) | [`v3/examples/workflow/run-records/wrong-stop-decision.run-record.json`](../v3/examples/workflow/run-records/wrong-stop-decision.run-record.json) | calibration fixture, wrong stop decision |
