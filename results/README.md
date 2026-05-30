# Results format

`results.json` stores benchmark rows. `leaderboard.md` is the rendered table for humans. The
mechanical columns determine rank; the human-feel column is operator taste and never affects rank.

Required row fields:

- `model`: display name for the contender.
- `producedRepo`: produced-game repository or path, not vendored source.
- `generationCap`: maximum evolve generations used for the run.
- `trajectory`: array of AC pass counts per generation.
- `finalPassCount`: final mechanical AC pass count out of 16.
- `generationsToPlayable`: first generation passing AC1–AC8, or `null`.
- `generationsToYeti`: first generation passing AC12–AC14, or `null`.
- `monotonic`: whether the trajectory never regressed.
- `humanFeelOperatorTasteNotScore`: optional labeled human verdict, not used for ranking.
- `evidence`: path to the final conformance JSON or release evidence note.
