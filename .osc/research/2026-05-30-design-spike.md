# 2000m — Benchmark Design Spike (decision-support, not yet built)

Status: DRAFT for owner review. Nothing public created yet. No evolve run started.
Author: Hermes (for graphanov). Date: 2026-05-30.

This document exists to argue the design — especially the scoring oracle — ON PAPER
before any repo is created or any model run is launched. Premature commitment is the
known failure mode; this spike is the guardrail against it.

---

## 1. What 2000m is (and is not)

**2000m** is a reproducibility benchmark. It measures how well a given LLM, driving the
Open Scaffold closed evolutionary loop (`osc evolve`), can iteratively build a faithful,
playable **Rust** homage of the 1991 game **SkiFree** (Chris Pirih, Microsoft) across multiple
generations.

The name is the distance at which SkiFree's Abominable Snow Monster spawns — the game's
single most iconic, most testable moment.

### Language: Rust (CONFIRMED pending owner, 2026-05-30)
Supersedes the earlier "any language" idea. Every produced game must be written in **Rust**.
Rationale: Rust is a harder target, so it discriminates between models (a benchmark everyone
trivially passes is worthless); integer/fixed-point math makes seed-determinism easier than JS;
and Rust→WASM (e.g. via `macroquad`) gives a clean in-browser visual demo with no separate
stack. Honest scope: this is a **Rust SkiFree reproduction** benchmark, named as such. The
JSON-line driver protocol (§3) still applies — the driver is simply a Rust binary speaking it.

### It measures
- **Iterative improvement through the evolve loop**: the trajectory of acceptance-criteria
  passes across generations, per model. Gen 1 might pass 3/15 ACs; Gen 6 might pass 14/15.
  That climb IS the metric.

### It explicitly does NOT measure
- "Which LLM is smarter" in general. Every model runs *through* Open Scaffold, so the
  harness is in the loop by design. The honest framing is:
  **"Which LLM best drives the Open Scaffold evolve loop to reproduce SkiFree."**
  We name this confound openly; we never market 2000m as a general LLM benchmark.
- Visual taste. Fidelity is graded by a mechanical AC suite, not by how pretty it looks.

---

## 2. Source grounding (authoritative)

- Original author page: https://ski.ihoc.net/ (Chris Pirih). Confirms: endless slope with
  **horizontal wrap-around** (restored in v1.04 — a defining mechanic), VGA sprite rendering,
  the Snow Monster + sounds, v1.0 canonical behavior.
- Wikipedia: https://en.wikipedia.org/wiki/SkiFree. Confirms: three modes (slalom, tree
  slalom, freestyle), HUD (time/meters/speed/style), F-key accelerate, **monster at 2000m**,
  endless-runner course loop.
- Reference reproduction (MIT): https://github.com/basicallydan/skifree.js (549*). Confirms
  feasibility and authentic details: seeded random tree fields, collision reactions, F-key
  boost, ramps/jumps, and the monster that **eats you then flees because it's full**. Has a
  real test suite (vitest + Playwright) — a model for our conformance approach.

**Asset/legal boundary:** 2000m ships NO copyrighted SkiFree binaries, sprites, or sounds.
Original assets only. Framed as a SkiFree-*inspired* homage. skifree.js is studied (MIT) for
mechanics, not copied.

---

## 3. The oracle (THE critical design decision) — LANGUAGE-AGNOSTIC

A benchmark is only as credible as its judge. 2000m's judge must be mechanical, deterministic,
and reproducible — never taste. This is achieved by requiring every produced game to expose a
**deterministic, seeded, headless logic core** separated from rendering.

OWNER DECISION (2026-05-30): models may pick ANY language. Therefore the oracle CANNOT be an
in-process module import (which would force one language). Instead the contract is a
**process-level stdin/stdout JSON-line driver protocol**. The benchmark ships the protocol spec
+ a conformance runner that spawns the game's headless driver as a subprocess; the model
implements the driver in whatever language it chose.

### Harness contract v0 — JSON-line driver protocol (language-agnostic)

Every produced game must ship a **headless driver executable** (e.g. `./2000m-driver`, or a
documented run command like `node driver.js` / `python driver.py` / `./target/release/driver`)
declared in a small `2000m.json` manifest at the game repo root:

```json
{ "protocolVersion": "2000m.driver.v0",
  "driver": { "command": "node", "args": ["dist/driver.js"] },
  "language": "typescript" }
```

The driver reads newline-delimited JSON commands on stdin and writes newline-delimited JSON
responses on stdout (one response per command). Commands:

```
→ {"cmd":"init","seed":42,"config":{}}        ← {"ok":true,"state":<GameState>}
→ {"cmd":"step","input":{"steer":1,"boost":false,"jump":false}}
                                              ← {"ok":true,"state":<GameState>}
→ {"cmd":"state"}                             ← {"ok":true,"state":<GameState>}
→ {"cmd":"reset","seed":42}                   ← {"ok":true,"state":<GameState>}
```

`Input` = `{ steer: -1|0|1, boost: bool, jump: bool }`.

`GameState` (the scored shape, language-neutral JSON):

```json
{ "skier":   { "x": 0, "y": 0, "speed": 0, "mode": "skiing|crashed|airborne|eaten" },
  "distanceM": 0,
  "style": 0,
  "obstacles": [ { "type": "tree|bigtree|stump|mogul|rock|ramp", "x": 0, "y": 0 } ],
  "monster":  null,
  "gameOver": false,
  "tick": 0 }
```

`monster` when present: `{ "x":0, "y":0, "mode":"chasing|eating|fleeing" }`.

**Determinism requirement:** same `seed` + same input sequence ⇒ byte-identical `GameState`
stream across two separate driver processes. The conformance runner asserts this directly
(spawn twice, compare). This is what makes runs comparable and the visual a by-product, not the
judge. Non-determinism is itself an AC failure.

### Separation from `osc compare` (important)

2000m's SCORE is produced by 2000m's own conformance suite (mechanical AC tests against the
driver protocol). `osc compare` is NOT the judge and is not used to score — per its own
contract it does not benchmark models or auto-score. Open Scaffold's role here is the
`osc evolve` LOOP and LEDGER: it records each generation, the AC verdicts fed in from the 2000m
suite, evidence, and the promote/reject/retry decision. 2000m judges; osc evolve remembers.

### Two graded surfaces
1. **Logic core** — graded by the AC ladder below via the conformance suite. This is the score.
2. **Render layer** — smoke-tested only (Playwright: "boots, fills canvas, draws pixels across
   ticks, keyboard input reaches the core, no crash"). Pass/fail, not ranked. Visual fidelity
   is a clearly-labeled secondary screenshot, never a number that affects ranking.

---

## 4. Acceptance-criteria ladder (v0, grounded)

Ordered easy → hard. Each AC is a headless, deterministic assertion against the contract.
Exact assertions to be finalized in `acceptance-criteria/` in the repo.

| AC | Criterion | Mechanical test |
|----|-----------|-----------------|
| AC1 | Skier entity with position state exists | `getState().skier` has x/y/speed/mode |
| AC2 | Steering moves the skier | steer -1/+1 changes skier.x deterministically |
| AC3 | Slope scrolls; meters increment | distanceM strictly increases while skiing |
| AC4 | Slope wraps horizontally | skier crossing edge re-enters opposite side (v1.04 mechanic) |
| AC5 | Seeded obstacle field | same seed ⇒ identical obstacle stream; different seed ⇒ different |
| AC6 | Tree/stump collision crashes skier | overlap ⇒ skier.mode='crashed', forward motion halts |
| AC7 | Crash recovery | after N ticks skier.mode returns to 'skiing' |
| AC8 | Speed increases going straight | steer 0 downhill ⇒ speed rises to a cap |
| AC9 | F-key boost exceeds normal cap | boost:true ⇒ speed exceeds the AC8 cap |
| AC10 | Ramp → airborne → land | hitting a ramp ⇒ mode='airborne' then back to 'skiing' |
| AC11 | Style scoring on tricks/landings | good landing adds style; crash deducts |
| AC12 | **Monster spawns at distanceM ≥ 2000** | monster becomes non-null at/after 2000m |
| AC13 | **Monster pursues the skier** | monster.x/y converges toward skier over ticks |
| AC14 | **Monster catches ⇒ eaten ⇒ game over** | contact ⇒ skier.mode='eaten', gameOver=true |
| AC15 | Monster flees after eating (authentic) | post-eat monster.mode='fleeing', moves away |
| AC16 | Restart resets state | new game ⇒ distance/style/monster reset; seed reproducible |

Trajectory metric = count of ACs passing at each generation, per model.

---

## 5. Scoring & metrics — TWO TRACKS (never blended)

The credibility spine: the rank is mechanical; human "feel" is reported alongside, never mixed
into the number. Blending taste into the score is the known failure mode and is forbidden.

### Track 1 — Mechanical (THE rank, objective, reproducible)
- **AC-pass trajectory**: `[gen1, gen2, ... gen8]` pass counts — the headline metric.
- **Final AC pass count** (out of 16).
- **Generations-to-playable** (first gen passing AC1–AC8).
- **Generations-to-yeti** (first gen passing AC12–AC14).
- **Monotonicity**: did it climb steadily or regress between gens?
- **Generations used** (efficiency, cap 8).
This track protects the floor: real mechanics, no crashes, performance, seed-determinism.

### Track 2 — Human feel (labeled judgment, NOT part of the rank)
After the run, the owner (Daniel) plays the final WASM demo of each model's game and records a
subjective verdict — e.g. `feel: 7/10 — "monster chase tense, turning mushy"`. This is reported
NEXT TO the mechanical rank, clearly labeled "operator taste, not benchmark score." It never
changes the AC-based ranking. This mirrors the established game-dev rule: automated ACs protect
crashes/perf/mechanics; the owner is the taste gate. It is the honest way to answer "does it
feel good?" without corrupting objectivity.

Secondary (labeled, non-ranking): final-build screenshot/GIF, qualitative notes, token/cost if
captured. These inform; they do not move the rank.

Leaderboard lives in `2000m/results/` as small JSON + a rendered markdown table, referencing
each produced-game repo. No produced game source lives in the benchmark repo. The table shows
the mechanical rank and, in a separate clearly-labeled column, the owner feel verdict.

---

## 6. Run protocol (per model)

1. Fresh produced-game repo `graphanov/2000m-<model>` initialized with Open Scaffold.
2. Seed the same task spec (the 2000m brief + harness contract + AC ladder) into `osc evolve`.
3. Run N generations. Each generation: the model attempts to raise AC passes; the conformance
   suite scores it; evidence + AC verdicts recorded via the evolve loop.
4. Capture the trajectory. Stop at a fixed generation cap or at full AC pass, whichever first.
5. Record results to `2000m/results/`. Produced game stays in its own repo.

Fixed seed set for scoring ⇒ apples-to-apples obstacle fields. No network/deps during scoring.

---

## 7. Guardrails

- **Mechanical oracle only.** If an AC can't be asserted headlessly, it doesn't gate the score.
- **Deterministic + seeded** logic core, or it isn't a benchmark.
- **No network, no external runtime deps** during scoring runs (reproducibility + safety).
- **No copyrighted assets.** Original-only homage.
- **Honest scope** stated in the README: measures LLM-driving-Open-Scaffold on SkiFree
  reproduction, not general intelligence.
- **Sober benchmark name, fun game names.** 2000m stays sober; produced games can be playful.
- **Owner gate** before any public repo creation, before any real evolve run, before publishing
  any leaderboard.

---

## 8. First run

- First (and so far only) contender: **Codex / GPT-5.5**.
- Recommendation: prove the trajectory mechanism end-to-end with ONE model before expanding the
  lineup. Get a real Gen1→GenN AC-pass curve for Codex, validate the conformance suite actually
  discriminates, then add a second model for a real head-to-head.

---

## 9. Owner decisions (CONFIRMED 2026-05-30)

1. **Harness language**: ANY language — models pick freely. → Oracle is the language-agnostic
   JSON-line driver protocol (§3), NOT a TS module import.
2. **Generation cap**: **8** evolve generations (evolutionary attempts) per model.
3. **Produced-game repos**: one repo per model (`graphanov/2000m-<model>`). [confirmed default]
4. **AC count for v0**: **all 16 ACs** in the first run.
5. **Who builds the 2000m benchmark itself** (harness contract + conformance suite + rules):
   built by **running OMX in `--madmax --xhigh` mode** (canonical autonomous worker launch:
   `omx --madmax --xhigh`), as an Open Scaffold work record. Hermes orchestrates/reviews; OMX
   does the build.
6. **Public from day 1**: yes. `graphanov/2000m` public from creation.

### Consequence to confirm understanding
- The BENCHMARK (`2000m`) = built by OMX --madmax --xhigh, owns the driver protocol + 16-AC
  conformance suite + rules + leaderboard. Public day 1. Contains no game solution.
- The PRODUCED GAMES (`2000m-<model>`) = built by each contending model through `osc evolve`
  (8-gen cap), scored by 2000m's suite. First contender: Codex / GPT-5.5.
- 2000m judges (mechanical AC suite). osc evolve remembers (loop + ledger + trajectory).
  `osc compare` is not the scorer.

---

## 10. Next concrete step

Sections 3 (oracle) + 4 (AC ladder) confirmed by owner. Build order:

1. Create public `graphanov/2000m`, initialize as an Open Scaffold work record (MISSION + plan).
2. Launch **`omx --madmax --xhigh`** against the 2000m build plan: driver protocol spec, the
   16-AC conformance suite (spawns driver subprocess, asserts determinism + each AC), rules,
   results/leaderboard scaffolding, README with honest scope. Hermes reviews output, runs gates.
3. Once 2000m exists and its conformance suite is real: create `graphanov/2000m-codex-gpt55`,
   seed the 2000m task into `osc evolve`, run Codex/GPT-5.5 for up to 8 generations, capture the
   real Gen1→Gen8 AC-pass trajectory.

Open sub-questions surfaced for §11 below before launching OMX.

---

## 11. Sub-question answers (CONFIRMED 2026-05-30)

1. **Determinism vs. real-time + "feel".** The SCORE judges the **fixed-tick deterministic logic
   core** (1 `step` = 1 logic tick, no wall-clock) — this is mechanically verifiable and
   reproducible. The "does it feel good?" question is real and wanted, but is handled as **Track 2
   human-feel judgment** (§5): owner plays the final WASM demo and records a labeled verdict that
   sits NEXT TO the rank and never blends into it. So: logic is auto-scored; feel is owner-judged
   and reported, not ranked. Verifiable without corrupting objectivity.

2. **OMX must NOT grade its own homework.** OMX (`--madmax --xhigh`) builds the benchmark: driver
   protocol spec, the 16-AC conformance suite, rules, leaderboard scaffolding, README. It also
   writes a deliberately **minimal STUB Rust driver** that passes only the lowest few ACs (e.g.
   AC1–AC3) purely to prove the harness runs end-to-end. OMX does NOT write a full reference game.
   Full games come only from the contending models later (owner-confirmed). This prevents the
   suite being unconsciously shaped to pass one full reference implementation.

3. **Visual demo location.** The playable WASM visual lives in each **produced-game repo**
   (`graphanov/2000m-<model>`), not in the benchmark repo. `graphanov/2000m` stays headless:
   protocol + conformance suite + rules + leaderboard, plus the minimal stub driver. Owner-confirmed.

4. **OMX autonomy leash.** OMX runs with **full autonomy on a branch** in the fresh public repo.
   Hermes reviews the output and runs all gates (build, conformance suite, determinism, verify);
   the owner sits at the merge/closeout gate. Same discipline as PR #158: full-send build, hard
   review before anything is called done. Owner-confirmed.

### Net build definition for OMX
Build `graphanov/2000m` (Rust-targeted SkiFree reproduction benchmark) containing:
- `2000m.driver.v0` JSON-line protocol spec (language is Rust for produced games).
- A conformance runner that spawns a produced game's Rust driver binary as a subprocess and
  asserts seed-determinism + each of the 16 ACs.
- A minimal stub Rust driver (passes ~AC1–AC3 only) proving the harness runs.
- Rules, honest-scope README (Rust SkiFree reproduction; LLM-driving-osc-evolve, not general IQ;
  no copyrighted assets), and `results/` leaderboard scaffolding with a separate labeled
  human-feel column.
All as an Open Scaffold work record (MISSION + plan + evidence + close), full autonomy on a
branch, Hermes-reviewed, owner merge gate.
