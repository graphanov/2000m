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
