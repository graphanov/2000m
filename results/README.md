# Results format

This is the scoreboard format checkpoint for 2000m. [`results.json`](results.json)
stores benchmark rows, and [`leaderboard.md`](leaderboard.md) is the rendered
table for humans.

Mechanical columns determine rank through the track-labeled AC-pass trajectory
and related fields. Human feel is operator taste, not score, and never affects
rank.

Required row fields for the current v0-style table:

- `model`: display name for the contender.
- `producedRepo`: produced-game repository or path, not vendored source.
- `generationCap`: maximum evolve generations used for the run.
- `trajectory`: array of AC pass counts per generation.
- `finalPassCount`: final mechanical AC pass count out of 16.
- `generationsToPlayable`: first generation passing AC1–AC8, or `null`.
- `generationsToYeti`: first generation passing AC12–AC14, or `null`.
- `monotonic`: whether the trajectory never regressed.
- `humanFeelOperatorTasteNotScore`: optional labeled human verdict, not used for
  ranking.
- `evidence`: path to the final conformance JSON or release evidence note.

For v1 runs, keep the row explicitly track-labeled and include quality/composite
fields in evidence until the rendered leaderboard schema is expanded.
