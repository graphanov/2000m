# 2000m v1: The Benchmark That Proves Scaffolding Wins

## Problem Statement

The v0 benchmark was 1-shotted by GPT-5.5 (16/16 on generation 1) because:

1. **All ACs were binary existence checks**: "Does collision work?" → pass/fail
2. **No quality gradients**: A minimal implementation scored the same as a polished one
3. **No interacting constraints**: Each AC was independent, so no trade-offs emerged
4. **No stress tests**: Edge cases, performance, and determinism under load weren't checked
5. **No room for iteration**: Once all 16 passed, there was nowhere to go

This made Open Scaffold's `compare` and `evolve` irrelevant — there was nothing to compare, nothing to evolve toward.

## Design Principles for v1

### 1. Quality Gradients, Not Binary Checks

Every AC should have a **quality score** (0-100) in addition to pass/fail. A basic implementation passes, but a refined one scores higher.

Example:
- **v0**: "Collision detection works" → pass if any collision detected
- **v1**: "Collision detection quality" → score based on precision, edge cases, performance

### 2. Stress Tests and Edge Cases

Add ACs that only pass under specific stress conditions:
- High-speed tunneling (skier moves so fast it could skip obstacles)
- Dense obstacle fields (50+ obstacles visible simultaneously)
- Monster pursuit under evasion (yeti vs. actively dodging skier)
- Determinism over 10,000 ticks (not just 80)

### 3. Interacting Constraints

Create ACs that conflict with each other, forcing trade-offs:
- **AC-17: Smooth 60fps animation** vs **AC-20: Detailed particle effects**
- **AC-18: Deterministic replay** vs **AC-21: Performance budget**
- **AC-19: Responsive controls** vs **AC-22: Realistic physics simulation**

These force models to iterate: first attempt will optimize one at the expense of another.

### 4. Tiered Difficulty

Structure ACs into tiers that unlock progressively:
- **Tier 1 (Gen 1-2)**: Core mechanics — should pass easily
- **Tier 2 (Gen 3-5)**: Edge cases and performance — where 1-shot fails
- **Tier 3 (Gen 6-8)**: Polish and optimization — where `evolve` shines

### 5. Multi-Dimensional Scoring

Replace single `passCount` with composite score:

```
Final Score = (Pass Rate × 0.4) + (Quality Score × 0.3) + (Efficiency Score × 0.2) + (Convergence Speed × 0.1)
```

Where:
- **Pass Rate**: % of ACs passed (binary)
- **Quality Score**: Average quality score across all ACs (0-100 each)
- **Efficiency Score**: Lines of code / performance ratio
- **Convergence Speed**: Generations needed to reach 80% pass rate (fewer = better)

## New AC Structure

### Tier 1: Core Mechanics (Existing, Enhanced)

**AC1: Skier Entity** (unchanged)
- Pass: Skier exists with position
- Quality: Bonus for clean separation of concerns, readable code

**AC2: Steering** (enhanced)
- Pass: Steering moves skier deterministically
- Quality: Bonus for smooth acceleration curves, responsive feel

**AC3: Slope Scrolling** (enhanced)
- Pass: Distance increases while skiing
- Quality: Bonus for sub-pixel rendering, no visible jitter

**AC4: Horizontal Wrap** (enhanced)
- Pass: Wrap observed
- Quality: Bonus for seamless visual transition, no pop-in

**AC5: Seeded Obstacles** (enhanced)
- Pass: Deterministic obstacle stream
- Quality: Bonus for diverse obstacle types, natural distribution

**AC6: Collision** (enhanced with stress test)
- Pass: Collision crashes skier
- Quality: Bonus for:
  - Pixel-perfect detection (not just bounding box)
  - High-speed tunneling prevention (skier at max boost doesn't skip obstacles)
  - Edge case: corner collisions, grazing hits
  - Performance: detection time < 1ms per frame

**AC7: Crash Recovery** (enhanced)
- Pass: Recovery works
- Quality: Bonus for smooth animation, reasonable recovery time

**AC8: Speed Cap** (enhanced)
- Pass: Speed caps
- Quality: Bonus for smooth acceleration curve, realistic feel

**AC9: Boost** (enhanced)
- Pass: Boost exceeds cap
- Quality: Bonus for visual feedback, smooth transition

**AC10: Ramps** (enhanced)
- Pass: Airborne + landing
- Quality: Bonus for parabolic trajectory, realistic physics

**AC11: Style Scoring** (enhanced)
- Pass: Style changes
- Quality: Bonus for combo system, risk/reward balance

**AC12: Monster Spawn** (unchanged)
- Pass: Spawns at 2000m

**AC13: Monster Pursuit** (enhanced with stress test)
- Pass: Monster converges
- Quality: Bonus for:
  - Intelligent pathfinding (avoids obstacles while chasing)
  - Evasion difficulty (skier can dodge with skill, not just luck)
  - Performance: pursuit calculation < 1ms per frame

**AC14: Monster Eats** (unchanged)
- Pass: Contact ends game

**AC15: Monster Flees** (unchanged)
- Pass: Flees after eating

**AC16: Reset** (enhanced)
- Pass: Reset reproducible
- Quality: Bonus for clean state management, no memory leaks

### Tier 2: Edge Cases and Performance (New)

**AC17: High-Speed Tunneling Prevention**
- Test: Skier at max boost speed (10.5 m/tick) vs. thin obstacles (width < 2m)
- Pass: No tunneling observed over 1000 high-speed collision attempts
- Quality: Bonus for swept-volume collision detection, continuous collision detection
- Stress: 50 consecutive high-speed runs through dense obstacle field

**AC18: Dense Obstacle Field Performance**
- Test: 100 obstacles visible simultaneously
- Pass: Maintain 60fps (16.6ms frame budget) with all mechanics active
- Quality: Bonus for:
  - Spatial partitioning (quadtree, grid)
  - Culling (only update visible obstacles)
  - Performance profiling data included

**AC19: Monster Pursuit Under Evasion**
- Test: Skier actively dodging monster for 200 ticks
- Pass: Monster doesn't get stuck, doesn't teleport
- Quality: Bonus for:
  - Predictive pathfinding (anticipates skier movement)
  - Fair difficulty (skier can escape with skill)
  - Monster doesn't clip through obstacles

**AC20: Determinism Over Long Runs**
- Test: 10,000 tick simulation with random inputs
- Pass: Two independent processes produce identical state streams
- Quality: Bonus for:
  - Deterministic floating-point arithmetic (fixed-point or integer)
  - Seed management (no global random state)
  - Performance: < 50ms per tick even at 10k ticks

**AC21: Crash Recovery Under Load**
- Test: 50 crashes in rapid succession (crash, recover, crash immediately)
- Pass: No state corruption, no memory leaks
- Quality: Bonus for:
  - Clean state machine transitions
  - Resource cleanup (particle effects, sound buffers)
  - Performance: recovery time < 100ms

**AC22: Monster Spawn Timing Precision**
- Test: Monster must spawn at exactly 2000m ± 0.1m
- Pass: Spawn within tolerance
- Quality: Bonus for:
  - Exact distance tracking (no floating-point drift)
  - Smooth transition (no pop-in)
  - Narrative moment (camera shake, sound cue)

### Tier 3: Polish and Optimization (New)

**AC23: Input Responsiveness**
- Test: Measure latency from keypress to visual response
- Pass: Latency < 50ms
- Quality: Bonus for:
  - Input buffering (queue inputs during animation locks)
  - Predictive rendering (anticipate input based on pattern)
  - Profiling data included

**AC24: Collision Forgiveness**
- Test: Near-miss detection with configurable tolerance
- Pass: Player feels "I barely made it" not "that was unfair"
- Quality: Bonus for:
  - Hitbox shrinking during grace frames
  - Visual feedback (screen shake on near-miss)
  - Playtesting data (AI playtest with 1000 runs)

**AC25: Animation Smoothness**
- Test: No visible jitter during camera movement
- Pass: Sub-pixel rendering, proper interpolation
- Quality: Bonus for:
  - Motion blur on fast movement
  - Easing functions (smooth acceleration/deceleration)
  - 60fps stability (no dropped frames)

**AC26: Deterministic Replay Accuracy**
- Test: Replay must match original within 1 pixel
- Pass: Replay matches over 1000 tick simulation
- Quality: Bonus for:
  - Floating-point drift correction
  - Random seed management (per-system seeds)
  - Replay file size < 1KB per 1000 ticks

**AC27: Performance Budget**
- Test: Maintain 60fps with 100+ active objects
- Pass: 99% of frames < 16.6ms
- Quality: Bonus for:
  - Object pooling (no allocations during gameplay)
  - Parallel processing (rayon, SIMD)
  - Memory usage < 50MB

**AC28: Visual Polish**
- Test: Particle effects, screen shake, juice
- Pass: Visual feedback on all major events
- Quality: Bonus for:
  - Particle system (crashes, jumps, trails)
  - Screen shake (impact, near-miss)
  - Color grading (speed lines, style meter)
  - Accessibility (colorblind mode, pause on focus loss)

## Protocol Changes

### New Command: `profile`

Returns performance metrics for the last N ticks:

```json
{"cmd":"profile","ticks":100}
```

Response:
```json
{
  "ok": true,
  "metrics": {
    "avgTickMs": 2.3,
    "maxTickMs": 8.1,
    "p95TickMs": 4.2,
    "allocations": 0,
    "memoryBytes": 45000000
  }
}
```

### New Command: `replay`

Returns deterministic replay data for the last N ticks:

```json
{"cmd":"replay","ticks":1000}
```

Response:
```json
{
  "ok": true,
  "replay": {
    "seed": 42,
    "commands": [...],
    "states": [...],
    "checksum": "abc123"
  }
}
```

### Enhanced GameState

Add optional quality metrics:

```json
{
  "skier": { ... },
  "distanceM": 1234.5,
  "style": 42,
  "obstacles": [...],
  "monster": { ... },
  "gameOver": false,
  "tick": 1234,
  "quality": {
    "frameTime": 2.3,
    "collisionsChecked": 15,
    "collisionsDetected": 2,
    "memoryBytes": 45000000
  }
}
```

## Scoring System

### Per-AC Scoring

Each AC returns:
```json
{
  "id": "AC6",
  "name": "Collision Detection",
  "pass": true,
  "quality": 85,
  "detail": "Pixel-perfect detection, tunneling prevented, 0.8ms avg",
  "breakdown": {
    "basic": 100,
    "precision": 90,
    "edgeCases": 80,
    "performance": 70
  }
}
```

### Composite Score

```python
def calculate_score(acs, generations):
    pass_rate = sum(ac.pass for ac in acs) / len(acs)
    quality_score = sum(ac.quality for ac in acs) / len(acs)
    
    # Efficiency: lines of code / performance
    loc = count_lines_of_code()
    perf = average_frame_time()
    efficiency = min(100, 1000 / (loc * perf))
    
    # Convergence: fewer generations = better
    convergence = 100 - (generations * 10)
    
    return (
        pass_rate * 0.4 +
        quality_score * 0.3 +
        efficiency * 0.2 +
        convergence * 0.1
    )
```

## Expected Trajectory

### Without Open Scaffold (Raw LLM)

```
Gen 1: 16/28 ACs passed, quality 45, score 52
Gen 2: 20/28 ACs passed, quality 50, score 58
Gen 3: 22/28 ACs passed, quality 52, score 60
Gen 4: 22/28 ACs passed, quality 51, score 59  ← stagnation
Gen 5: 23/28 ACs passed, quality 53, score 61
Gen 6: 23/28 ACs passed, quality 52, score 60  ← stagnation
Gen 7: 24/28 ACs passed, quality 54, score 62
Gen 8: 24/28 ACs passed, quality 53, score 61  ← stagnation
```

Raw model stalls because it can't identify which ACs to prioritize or how to resolve trade-offs.

### With Open Scaffold (Scaffolded LLM)

```
Gen 1: 16/28 ACs passed, quality 45, score 52
  → compare: "AC17, AC18, AC19 failed due to performance"
  → evolve: "Add spatial partitioning to fix AC18"

Gen 2: 20/28 ACs passed, quality 55, score 65
  → compare: "AC20 failed due to floating-point drift"
  → evolve: "Switch to fixed-point arithmetic"

Gen 3: 24/28 ACs passed, quality 65, score 75
  → compare: "AC23, AC24 failed due to input latency"
  → evolve: "Add input buffering and predictive rendering"

Gen 4: 26/28 ACs passed, quality 72, score 82
  → compare: "AC25 failed due to animation jitter"
  → evolve: "Add sub-pixel rendering and easing functions"

Gen 5: 27/28 ACs passed, quality 78, score 87
  → compare: "AC26 failed due to replay drift"
  → evolve: "Implement deterministic random seed management"

Gen 6: 28/28 ACs passed, quality 85, score 92
  → compare: "All ACs passed, optimize quality scores"
  → evolve: "Add particle effects and screen shake"

Gen 7: 28/28 ACs passed, quality 92, score 95
  → compare: "Optimize efficiency score"
  → evolve: "Implement object pooling and SIMD"

Gen 8: 28/28 ACs passed, quality 95, score 98  ← ceiling
```

Scaffolded model reaches 98 vs raw model's 61 because `compare` identifies bottlenecks and `evolve` generates targeted fixes.

## Implementation Plan

### Phase 1: Protocol Extension
- Add `profile` and `replay` commands
- Extend `GameState` with optional `quality` field
- Update driver protocol documentation

### Phase 2: Tier 2 ACs
- Implement AC17-AC22 with stress tests
- Add quality scoring infrastructure
- Create performance profiling harness

### Phase 3: Tier 3 ACs
- Implement AC23-AC28 with polish checks
- Add visual analysis (screenshot comparison)
- Create playtesting harness (AI plays 1000 games)

### Phase 4: Scoring System
- Implement composite score calculation
- Add leaderboard with multi-dimensional ranking
- Create trajectory visualization

### Phase 5: Open Scaffold Integration
- Document `compare` and `evolve` workflows
- Create example trajectories
- Publish results showing scaffolded vs raw performance

## Success Metrics

The v1 benchmark succeeds if:

1. **No 1-shot**: No model reaches 28/28 on generation 1
2. **Iteration required**: Models need 4+ generations to reach 80% pass rate
3. **Scaffolding wins**: Scaffolded models outperform raw models by 30+ points
4. **Quality matters**: Quality score differentiates implementations beyond pass/fail
5. **Popular adoption**: 10+ models tested within 6 months of release

## Conclusion

The v1 benchmark transforms 2000m from a "does it work?" check into a "how well does it work?" evaluation that:

- **Punishes premature convergence**: Binary checks become quality gradients
- **Rewards iterative exploration**: Stress tests and edge cases require refinement
- **Demonstrates scaffolding value**: `compare` identifies bottlenecks, `evolve` generates fixes
- **Creates compelling narratives**: Trajectory charts show scaffolded vs raw performance

This positions Open Scaffold as essential infrastructure for AI-assisted development, not just a nice-to-have.
