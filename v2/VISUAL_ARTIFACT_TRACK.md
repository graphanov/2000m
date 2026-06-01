# 2000m v2 visual/artifact-quality track

Status: protocol specification. This track defines what visual evidence must look like before a campaign may claim that one lane produced a more visually coherent game artifact.

## Why this exists

The v1 AC28 probe was useful as a pressure test, but it was not enough to prove visual quality. Driver-reported visual events and probe-only signals can catch some implementation behavior; they cannot establish that a game looks better to a human or that a workflow caused that improvement.

The visual/artifact-quality track is deliberately separate from mechanical score and workflow-resilience score.

## Required artifact package

For every valid paired run, each lane should produce a visual package with:

1. **Build ref** — public-safe repo ref, commit, or digest for the produced artifact.
2. **Fixed visual seeds** — the exact seeds used for replay/capture.
3. **Capture command** — command used to render screenshots, GIFs, or replay frames.
4. **Screenshots** — still images from predeclared ticks/windows.
5. **GIF or replay capture** — short motion artifact from the same fixed seeds.
6. **Replay log or frame metadata** — enough to regenerate or audit the capture.
7. **Rubric record** — machine-readable or markdown record of rubric judgments.
8. **Blind label map** — private or sealed mapping from anonymized artifact labels back to lanes; public result summaries should not reveal labels before review.

If screenshots/GIFs cannot be produced for a pilot, the campaign may still run as mechanical/workflow calibration, but it must not claim visual superiority.

## Capture protocol

A capture protocol must be predeclared before live results:

- scenario id and version;
- visual seed list;
- viewport/resolution;
- tick windows or frame ranges;
- screenshot count;
- GIF/replay duration and frame rate;
- capture command;
- accepted file formats;
- invalidation rules for crashes or nondeterministic captures.

Recommended pilot default:

```text
visual seeds: 3 fixed seeds per task seed
screenshots: 3 stills per visual seed at predeclared windows
gif/replay: 1 short capture per visual seed
resolution: fixed per produced artifact harness
review labels: randomized within each paired task seed
```

## Visual rubric

Use a rubric before any blinded comparison. Suggested 100-point split:

| Component | Weight | What it asks |
|---|---:|---|
| Field readability | 20 | Can the player, obstacles, and movement direction be understood at a glance? |
| Spatial composition | 20 | Are obstacles/targets distributed in a playable, visually coherent field? |
| Motion richness | 20 | Does the capture show more than static objects or minimal movement? |
| Event variety | 15 | Are collisions, avoidance, scoring, hazards, or other game events visible? |
| Game-shape coherence | 15 | Does the artifact read as an actual game loop rather than a driver stub? |
| Polish restraint | 10 | Are visual additions supportive rather than noisy, fake, or asset-infringing? |

Rubric scoring is artifact quality, not model intelligence and not adoption proof.

## Optional blinded preference

For paired pilots, add a blinded preference pass when practical:

1. Randomize labels within each paired seed, e.g. Artifact X and Artifact Y.
2. Hide lane/process labels from reviewers.
3. Show the same number and type of artifacts for each lane.
4. Ask for:
   - forced preference, if any;
   - confidence;
   - short rationale;
   - rubric scores.
5. Reveal lane mapping only after preference records are sealed.

If the owner is the only reviewer, label the result as owner taste and avoid claiming population preference. Multiple blinded reviewers are better for a real evidence campaign.

## Measurable visual features

Where practical, record diagnostic features alongside human/rubric review:

- obstacle/object density per frame;
- object type variety;
- visible motion/event count;
- approximate spread across the field;
- player-path clearance or collision density;
- animation/frame-change richness;
- screenshot entropy or contrast diagnostics.

These features are supporting diagnostics. They do not replace replay artifacts, rubric scoring, or human taste review.

## Rank blockers and invalidation

A visual/artifact-quality claim is blocked when:

- artifacts are missing for one lane in a valid pair;
- capture seeds or windows differ across lanes;
- a lane used extra human edits or hidden prompt help;
- output uses private/local paths as public evidence;
- screenshots/GIFs cannot be regenerated from the recorded refs;
- the artifact contains unapproved replica/proprietary visuals.

A blocked visual track does not automatically invalidate the mechanical or workflow-resilience score. It only prevents visual-superiority claims.

## Public wording

Allowed:

- "Lane B won the blinded visual preference in N/M paired seeds under this frozen campaign."
- "Visual/artifact quality was measured separately from mechanical score."
- "The pilot found a qualitative visual signal; sample size is too small for a proof claim."

Not allowed:

- "A single prettier output proves Open Scaffold wins."
- "Probe-only driver output proves visual quality."
- "Human taste changes the mechanical benchmark rank."
- "Visual preference proves adoption or general model intelligence."
