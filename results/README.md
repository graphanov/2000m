# Results format

This is the scoreboard format checkpoint for 2000m. [`results.json`](results.json)
stores benchmark rows, and [`leaderboard.md`](leaderboard.md) is the rendered
table for humans. Run `python3 scripts/render_results.py --check` to verify that
the rendered table matches the machine-readable result spine.

Mechanical columns determine rank through the track-labeled AC-pass trajectory
and related fields. Human feel is operator taste, not score, and never affects
rank.

Required row fields for the current v0-style table:

- `model`: display name for the contender.
- `producedRepo`: produced-game repository or path, not vendored source.
- `generationCap`: maximum attempts/generations used for the run.
- `trajectory`: array of AC pass counts per generation.
- `finalPassCount`: final mechanical AC pass count out of 16.
- `generationsToPlayable`: first generation passing AC1–AC8, or `null`.
- `generationsToYeti`: first generation passing AC12–AC14, or `null`.
- `monotonic`: whether the trajectory never regressed.
- `humanFeelOperatorTasteNotScore`: optional labeled human verdict, not used for
  ranking.
- `evidence`: path to the final conformance JSON or release evidence note.

For v1 runs, keep the row explicitly track-labeled and include quality/composite
fields in evidence until the rendered leaderboard schema is expanded. Evidence
for v1 timing or polish fields must label whether each field is suite-recomputed,
host-bound, driver-reported, probe-only, or a constant rubric default.

For v2 runs, keep artifact quality and workflow-resilience components separate.
A v2 row points to the generic v2 run record, scenario version, and scorer result
JSON; it must not require or privilege any particular workflow framework.
Calibration fixtures may live in the result spine only when their claim boundary
says they are not contender results.

Required row fields for `v2Rows`:

- `track`: currently `v2-workflow-resilience`.
- `scenario`: public scenario JSON path.
- `runRecord`: public generic run-record JSON path.
- `resultJson`: scorer output matching `v2/result.schema.json`.
- `claimBoundary`: plain-language limit such as “calibration fixture, not a
  contender result.”
