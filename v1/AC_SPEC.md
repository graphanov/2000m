# 2000m v1 Acceptance Criteria Specifications

All 28 ACs with test procedures, pass conditions, quality scoring rubrics, and stress test parameters.

## Scoring Model

Each AC returns:

```rust
struct AcVerdict {
    id: String,           // "AC6"
    name: String,         // "Collision Detection"
    pass: bool,           // binary pass/fail
    quality: u8,          // 0-100 quality score
    detail: String,       // human-readable result
    breakdown: QualityBreakdown,
}

struct QualityBreakdown {
    basic: u8,      // core functionality (0-100)
    precision: u8,  // accuracy/edge cases (0-100)
    performance: u8, // speed/efficiency (0-100)
    polish: u8,     // UX/refinement (0-100)
}
```

**Composite AC score**: `(basic × 0.4) + (precision × 0.2) + (performance × 0.2) + (polish × 0.2)`

**Pass condition**: `basic >= 80` (core functionality works)

**Quality score**: Full composite (0-100)

---

## Tier 1: Core Mechanics (AC1-AC16)

Existing ACs enhanced with quality scoring.

### AC1: Skier Entity with Position State

**Test procedure**:
```rust
let s = init(seed=1).state;
assert s.tick == 0;
assert s.skier.x.is_finite();
assert s.skier.y.is_finite();
assert s.skier.speed.is_finite();
assert matches!(s.skier.mode, "skiing");
```

**Pass condition**: Skier exists with valid position and mode

**Quality rubric**:
- `basic` (100): Skier exists with x, y, speed, mode
- `precision` (80): Position within reasonable bounds (|x| < 1000, |y| < 10)
- `performance` (70): State serialization < 1KB
- `polish` (60): Clean separation of concerns (skier struct, not inline fields)

**Expected quality**: 75-90

---

### AC2: Steering Moves Skier Deterministically

**Test procedure**:
```rust
// Right steering
let start = init(seed=2).state.skier.x;
for _ in 0..5 {
    step(steer=1, boost=false, jump=false);
}
let right_x = state().skier.x;
assert right_x > start + 0.1;

// Left steering
let start = init(seed=2).state.skier.x;
for _ in 0..5 {
    step(steer=-1, boost=false, jump=false);
}
let left_x = state().skier.x;
assert left_x < start - 0.1;
```

**Pass condition**: Steering changes skier.x deterministically

**Quality rubric**:
- `basic` (100): Steering moves skier
- `precision` (90): Symmetric response (left/right mirror within 5%)
- `performance` (80): Smooth acceleration curve (no instant teleport)
- `polish` (70): Input buffering (queue inputs during animation locks)

**Expected quality**: 80-95

---

### AC3: Slope Scrolls While Skiing

**Test procedure**:
```rust
let mut prev = init(seed=3).state.distanceM;
for i in 1..=10 {
    let s = step(steer=0, boost=false, jump=false).state;
    assert s.skier.mode == "skiing";
    assert s.distanceM > prev;
    prev = s.distanceM;
}
```

**Pass condition**: Distance strictly increases while skiing

**Quality rubric**:
- `basic` (100): Distance increases
- `precision` (90): Consistent increment (variance < 5%)
- `performance` (80): Sub-pixel rendering (no visible jitter)
- `polish` (70): Easing on speed changes (smooth acceleration)

**Expected quality**: 80-95

---

### AC4: Horizontal Wrap

**Test procedure**:
```rust
let mut prev = init(seed=4).state.skier.x;
for i in 1..=1200 {
    let x = step(steer=1, boost=false, jump=false).state.skier.x;
    if x < prev - 10.0 {
        return Ok("wrap observed");
    }
    prev = x;
}
return Err("no wrap after 1200 ticks");
```

**Pass condition**: Wrap discontinuity observed

**Quality rubric**:
- `basic` (100): Wrap works
- `precision` (90): Seamless transition (no pop-in, no gap)
- `performance` (80): Wrap calculation < 1μs
- `polish` (70): Visual continuity (obstacles wrap too, not just skier)

**Expected quality**: 75-90

---

### AC5: Seeded Obstacle Field

**Test procedure**:
```rust
let a = collect_obstacle_stream(seed=501, steps=60);
let b = collect_obstacle_stream(seed=501, steps=60);
let c = collect_obstacle_stream(seed=502, steps=60);

assert a.stream == b.stream;  // same seed → same stream
assert a.stream != c.stream;  // different seed → different stream
assert a.non_empty_count > 0; // obstacles exist
```

**Pass condition**: Deterministic obstacle generation

**Quality rubric**:
- `basic` (100): Deterministic stream
- `precision` (90): Diverse obstacle types (5+ types used)
- `performance` (80): Obstacle culling (only visible obstacles in state)
- `polish` (70): Natural distribution (not grid, not pure random — Perlin noise or similar)

**Expected quality**: 75-90

---

### AC6: Collision Detection

**Test procedure**:
```rust
let mut s = init(seed=6).state;
for _ in 0..2000 {
    let steer = steer_toward_obstacle(&s, &["tree", "bigtree", "stump", "rock"]);
    s = step(steer, boost=false, jump=false).state;
    if s.skier.mode == "crashed" {
        let crash_distance = s.distanceM;
        for _ in 0..5 {
            let after = step(steer=0).state;
            assert after.distanceM <= crash_distance + 0.001;
        }
        return Ok("crash observed");
    }
}
return Err("no crash after 2000 ticks");
```

**Pass condition**: Collision crashes skier and halts distance

**Quality rubric**:
- `basic` (100): Collision detected, crash state entered
- `precision` (95): Pixel-perfect detection (uses obstacle width/height if available)
- `performance` (90): Detection time < 1ms per frame, tunneling prevention
- `polish` (85): Edge cases (corner collisions, grazing hits), visual feedback

**Stress test**: High-speed tunneling (AC17)

**Expected quality**: 80-95

---

### AC7: Crash Recovery

**Test procedure**:
```rust
let mut crashed_at = None;
for _ in 0..2000 {
    let steer = if crashed_at.is_some() { 0 } else { steer_toward_crash() };
    s = step(steer).state;
    if s.skier.mode == "crashed" && crashed_at.is_none() {
        crashed_at = Some(s.tick);
    }
    if let Some(tick) = crashed_at {
        if s.tick > tick + 2 && s.skier.mode == "skiing" {
            return Ok("recovery observed");
        }
    }
}
```

**Pass condition**: Recovery from crash state

**Quality rubric**:
- `basic` (100): Recovery works
- `precision` (90): Reasonable recovery time (2-10 ticks)
- `performance` (80): Smooth animation (no instant teleport)
- `polish` (70): Clean state machine transitions

**Expected quality**: 80-95

---

### AC8: Speed Cap

**Test procedure**:
```rust
let start = init(seed=8).state.skier.speed;
let mut speeds = vec![];
for _ in 0..240 {
    let steer = steer_away_from_obstacle();
    s = step(steer, boost=false).state;
    if s.skier.mode == "skiing" {
        speeds.push(s.skier.speed);
    }
    if speeds.len() >= 120 { break; }
}

let early = speeds[20];
assert early > start + 0.25;  // acceleration

let tail = &speeds[speeds.len()-20..];
let tail_range = tail.max() - tail.min();
assert tail_range < 0.5;  // settled to cap
```

**Pass condition**: Speed accelerates then caps

**Quality rubric**:
- `basic` (100): Speed caps
- `precision` (90): Smooth acceleration curve (exponential or sigmoid)
- `performance` (80): Cap calculation < 1μs
- `polish` (70): Visual feedback (speed lines, camera FOV change)

**Expected quality**: 80-95

---

### AC9: Boost Exceeds Normal Cap

**Test procedure**:
```rust
let normal_max = max_speed_for(seed=9, boost=false, steps=140);
let boosted_max = max_speed_for(seed=9, boost=true, steps=80);
assert boosted_max > normal_max + 0.5;
```

**Pass condition**: Boost exceeds normal speed cap

**Quality rubric**:
- `basic` (100): Boost works
- `precision` (90): Smooth transition (no instant speed jump)
- `performance` (80): Boost state management < 1μs
- `polish` (70): Visual feedback (screen shake, motion blur, sound)

**Expected quality**: 80-95

---

### AC10: Ramp Airborne and Landing

**Test procedure**:
```rust
let mut airborne_at = None;
for _ in 0..2000 {
    let steer = if airborne_at.is_some() { 0 } else { steer_toward_ramp() };
    s = step(steer, boost=false, jump=true).state;
    if s.skier.mode == "airborne" && airborne_at.is_none() {
        airborne_at = Some(s.tick);
    }
    if let Some(tick) = airborne_at {
        if s.tick > tick && s.skier.mode == "skiing" {
            return Ok("airborne and landing observed");
        }
    }
}
```

**Pass condition**: Airborne state entered, then landing

**Quality rubric**:
- `basic` (100): Airborne + landing
- `precision` (90): Parabolic trajectory (realistic physics)
- `performance` (80): Physics calculation < 1μs
- `polish` (70): Animation blending, particle effects (dust on landing)

**Expected quality**: 75-90

---

### AC11: Style Scoring

**Test procedure**:
```rust
// Landing gain
let start_style = init(seed=11).state.style;
// ... navigate to ramp, go airborne, land ...
assert landing_style > start_style;

// Crash loss
let before_crash_style = ...;
// ... navigate to tree, crash ...
assert crash_style < before_crash_style;
```

**Pass condition**: Style changes on landing (gain) and crash (loss)

**Quality rubric**:
- `basic` (100): Style changes
- `precision` (90): Combo system (consecutive tricks multiply)
- `performance` (80): Style calculation < 1μs
- `polish` (70): Risk/reward balance, visual feedback (style meter, combo text)

**Expected quality**: 75-90

---

### AC12: Monster Spawns at 2000m

**Test procedure**:
```rust
init(seed=12);
for _ in 0..3000 {
    let s = step(steer=0).state;
    if s.monster.is_some() && s.distanceM < 2000.0 {
        return Err("monster spawned early");
    }
    if s.distanceM >= 2000.0 && s.monster.is_some() {
        return Ok("monster spawned at 2000m");
    }
}
```

**Pass condition**: Monster spawns at or after 2000m

**Quality rubric**:
- `basic` (100): Spawns at 2000m
- `precision` (90): Exact timing (within ±0.1m)
- `performance` (80): Spawn calculation < 1μs
- `polish` (70): Narrative moment (camera shake, sound cue, warning text)

**Expected quality**: 80-95

---

### AC13: Monster Pursues Skier

**Test procedure**:
```rust
// ... reach 2000m, monster spawns ...
let initial_distance = distance(monster, skier);
for _ in 0..100 {
    let s = step(steer=0).state;
    let current_distance = distance(s.monster, s.skier);
    assert current_distance < initial_distance;
}
```

**Pass condition**: Monster converges on skier

**Quality rubric**:
- `basic` (100): Monster pursues
- `precision` (90): Intelligent pathfinding (avoids obstacles)
- `performance` (80): Pursuit calculation < 1ms
- `polish` (70): Fair difficulty (skier can dodge with skill)

**Expected quality**: 75-90

---

### AC14: Monster Eats Skier

**Test procedure**:
```rust
// ... reach 2000m, monster spawns ...
for _ in 0..6000 {
    let s = step(steer=0).state;  // don't evade
    if s.skier.mode == "eaten" && s.gameOver {
        return Ok("skier eaten");
    }
}
```

**Pass condition**: Monster contact ends game

**Quality rubric**:
- `basic` (100): Contact ends game
- `precision` (90): Pixel-perfect contact detection
- `performance` (80): Contact check < 1μs
- `polish` (70): Death animation, game over screen

**Expected quality**: 80-95

---

### AC15: Monster Flees After Eating

**Test procedure**:
```rust
// ... monster eats skier ...
let mut flee_distance = None;
for _ in 0..6000 {
    let s = step(steer=0).state;
    if s.monster.mode == "fleeing" {
        let current = distance(s.monster, s.skier);
        if let Some(first) = flee_distance {
            if current > first + 1.0 {
                return Ok("monster fled");
            }
        } else {
            flee_distance = Some(current);
        }
    }
}
```

**Pass condition**: Monster enters fleeing mode and moves away

**Quality rubric**:
- `basic` (100): Flees after eating
- `precision` (90): Smooth transition (no teleport)
- `performance` (80): Flee calculation < 1μs
- `polish` (70): Narrative closure (monster leaves screen, fade to black)

**Expected quality**: 80-95

---

### AC16: Reset Reproducible

**Test procedure**:
```rust
init(seed=16);
// ... play for 60 ticks ...
let reset_state = reset(seed=16);
assert reset_state.tick == 0;
assert reset_state.distanceM == 0.0;
assert reset_state.style == 0.0;
assert reset_state.monster.is_none();
assert !reset_state.gameOver;

let reset_stream = collect_stream(steps=30);
let fresh_stream = fresh_init_stream(seed=16, steps=30);
assert reset_stream == fresh_stream;
```

**Pass condition**: Reset clears state and reproduces seed stream

**Quality rubric**:
- `basic` (100): Reset works
- `precision` (90): Complete state reset (no leaked state)
- `performance` (80): Reset time < 1ms
- `polish` (70): Clean resource cleanup (no memory leaks)

**Expected quality**: 85-100

---

## Tier 2: Edge Cases and Performance (AC17-AC22)

### AC17: High-Speed Tunneling Prevention

**Test procedure**:
```rust
// Challenge: start skier at max boost speed
init(seed=17);
challenge("high_speed", {"speed": 10.5});

// Steer toward thin obstacles (width < 2m)
let mut collisions = 0;
let mut tunneling = 0;
for _ in 0..1000 {
    let steer = steer_toward_thin_obstacle();
    let s = step(steer, boost=true).state;
    
    // Check if skier passed through obstacle without collision
    let obstacle_ahead = find_obstacle_ahead(&s, max_distance=5.0);
    if let Some(obs) = obstacle_ahead {
        if skier_passed_through(&s, &obs) && s.skier.mode != "crashed" {
            tunneling += 1;
        }
    }
    
    if s.skier.mode == "crashed" {
        collisions += 1;
    }
}

assert tunneling == 0;  // no tunneling
assert collisions > 0;  // collisions detected
```

**Pass condition**: No tunneling observed over 1000 high-speed collision attempts

**Quality rubric**:
- `basic` (100): No tunneling
- `precision` (95): Swept-volume collision detection (continuous collision detection)
- `performance` (90): Detection time < 2ms even at high speed
- `polish` (85): Visual feedback (impact particles, screen shake)

**Expected quality**: 70-90

**Failure modes**:
- Naive AABB collision misses high-speed objects
- Fixed timestep too large (skier moves > obstacle width per tick)

---

### AC18: Dense Obstacle Field Performance

**Test procedure**:
```rust
init(seed=18);
challenge("dense_field", {"obstacleCount": 100});

let mut frame_times = vec![];
for _ in 0..500 {
    let before = now();
    let s = step(steer=random()).state;
    let frame_time = now() - before;
    frame_times.push(frame_time);
    
    // Verify all mechanics still work
    assert s.obstacles.len() > 50;  // many obstacles visible
}

let avg = frame_times.mean();
let p95 = frame_times.percentile(95);
let p99 = frame_times.percentile(99);

assert avg < 16.6ms;  // 60fps
assert p95 < 20ms;
assert p99 < 30ms;
```

**Pass condition**: Maintain 60fps (16.6ms frame budget) with 100 obstacles

**Quality rubric**:
- `basic` (100): 60fps maintained
- `precision` (90): Spatial partitioning (quadtree, grid)
- `performance` (95): Culling (only update visible obstacles), object pooling
- `polish` (85): Profiling data included, optimization notes

**Expected quality**: 60-85

**Failure modes**:
- O(n²) collision detection
- No spatial partitioning
- Heap allocations per frame

---

### AC19: Monster Pursuit Under Evasion

**Test procedure**:
```rust
init(seed=19);
challenge("evasion_course", {"monsterDistance": 100});

let mut evasion_ticks = 0;
let mut monster_stuck = 0;
let mut monster_teleport = 0;
let mut last_monster_pos = None;

for _ in 0..200 {
    // Actively dodge: steer away from monster
    let steer = steer_away_from_monster();
    let s = step(steer, boost=true).state;
    
    if let Some(monster) = &s.monster {
        if let Some(last) = last_monster_pos {
            let distance_moved = distance(monster, last);
            if distance_moved > 20.0 {  // teleport threshold
                monster_teleport += 1;
            }
            if distance_moved < 0.1 {  // stuck threshold
                monster_stuck += 1;
            }
        }
        last_monster_pos = Some(monster.pos());
        
        // Check if skier successfully evaded
        let skier_monster_distance = distance(s.skier, monster);
        if skier_monster_distance > 50.0 {
            evasion_ticks += 1;
        }
    }
}

assert monster_stuck < 10;    // monster doesn't get stuck
assert monster_teleport == 0; // monster doesn't teleport
assert evasion_ticks > 50;    // skier can evade with skill
```

**Pass condition**: Monster doesn't get stuck or teleport, skier can evade

**Quality rubric**:
- `basic` (100): Pursuit works under evasion
- `precision` (90): Predictive pathfinding (anticipates skier movement)
- `performance` (85): Pathfinding < 2ms per tick
- `polish` (80): Fair difficulty (not too easy, not impossible)

**Expected quality**: 65-85

**Failure modes**:
- Monster gets stuck on obstacles
- Monster teleports when path blocked
- Pursuit too aggressive (no evasion possible)

---

### AC20: Determinism Over Long Runs

**Test procedure**:
```rust
// Process 1
let mut client1 = spawn_driver();
init(seed=20);
let mut stream1 = vec![];
for tick in 0..10000 {
    let steer = deterministic_input(tick);
    let s = step(steer).state;
    stream1.push(canonical_json(&s));
}

// Process 2 (fresh process)
let mut client2 = spawn_driver();
init(seed=20);
let mut stream2 = vec![];
for tick in 0..10000 {
    let steer = deterministic_input(tick);
    let s = step(steer).state;
    stream2.push(canonical_json(&s));
}

assert stream1 == stream2;  // identical streams

// Check for floating-point drift
let final_speed1 = stream1.last().skier.speed;
let final_speed2 = stream2.last().skier.speed;
assert (final_speed1 - final_speed2).abs() < 1e-10;
```

**Pass condition**: 10,000 tick simulation produces identical streams

**Quality rubric**:
- `basic` (100): Determinism over 10k ticks
- `precision` (95): Fixed-point or integer arithmetic (no floating-point drift)
- `performance` (90): < 50ms per tick even at 10k ticks
- `polish` (85): Seed management (no global random state), profiling data

**Expected quality**: 70-90

**Failure modes**:
- Floating-point accumulation error
- Global random state (not seeded per-system)
- Memory leaks causing performance degradation

---

### AC21: Crash Recovery Under Load

**Test procedure**:
```rust
init(seed=21);
challenge("crash_gauntlet", {"crashCount": 50});

let mut crashes = 0;
let mut recoveries = 0;
let mut state_corruption = 0;

for _ in 0..5000 {
    let steer = steer_toward_crash();
    let s = step(steer).state;
    
    if s.skier.mode == "crashed" {
        crashes += 1;
        // Check for state corruption
        if !s.skier.x.is_finite() || !s.skier.y.is_finite() {
            state_corruption += 1;
        }
        if s.obstacles.len() > 200 {  // memory leak?
            state_corruption += 1;
        }
    }
    
    if crashes > recoveries && s.skier.mode == "skiing" {
        recoveries += 1;
    }
}

assert crashes >= 50;           // 50 crashes occurred
assert recoveries == crashes;   // all recovered
assert state_corruption == 0;   // no corruption

// Check memory usage didn't grow
let memory_start = profile(window=100).metrics.peakMemoryBytes;
let memory_end = profile(window=100).metrics.peakMemoryBytes;
assert memory_end < memory_start * 1.1;  // < 10% growth
```

**Pass condition**: 50 crashes with no state corruption or memory leaks

**Quality rubric**:
- `basic` (100): 50 crashes + recoveries
- `precision` (90): Clean state machine transitions
- `performance` (85): Resource cleanup (particle effects, sound buffers)
- `polish` (80): Memory profiling data, no allocations during crash/recovery

**Expected quality**: 75-90

**Failure modes**:
- Particle effects not cleaned up
- Sound buffers not released
- State machine stuck in invalid state

---

### AC22: Monster Spawn Timing Precision

**Test procedure**:
```rust
init(seed=22);

let mut spawn_distance = None;
for _ in 0..3000 {
    let s = step(steer=0).state;
    if s.monster.is_some() && spawn_distance.is_none() {
        spawn_distance = Some(s.distanceM);
    }
}

let spawn = spawn_distance.unwrap();
assert (spawn - 2000.0).abs() < 0.1;  // within ±0.1m

// Check for visual transition quality
let before_spawn = state();
let after_spawn = step(steer=0).state;
let transition_smooth = (after_spawn.obstacles.len() - before_spawn.obstacles.len()).abs() < 5;
assert transition_smooth;  // no pop-in
```

**Pass condition**: Monster spawns at 2000m ± 0.1m

**Quality rubric**:
- `basic` (100): Spawns within tolerance
- `precision` (95): Exact distance tracking (no floating-point drift)
- `performance` (90): Spawn calculation < 1μs
- `polish` (85): Smooth transition (no pop-in), narrative moment (camera shake, sound)

**Expected quality**: 80-95

**Failure modes**:
- Floating-point drift in distance tracking
- Monster pops in without transition
- Spawn at 1999.5m or 2000.5m (outside tolerance)

---

## Tier 3: Polish and Optimization (AC23-AC28)

### AC23: Input Responsiveness

**Test procedure**:
```rust
init(seed=23);

// Measure latency from keypress to visual response
let mut latencies = vec![];
for _ in 0..100 {
    let before_input = now();
    let s_before = state();
    
    let s_after = step(steer=1).state;
    let after_response = now();
    
    let latency = after_response - before_input;
    let response_detected = s_after.skier.x != s_before.skier.x;
    
    if response_detected {
        latencies.push(latency);
    }
}

let avg_latency = latencies.mean();
let max_latency = latencies.max();

assert avg_latency < 50ms;
assert max_latency < 100ms;

// Check for input buffering
let mut buffered_inputs = 0;
for _ in 0..10 {
    // Send input during "animation lock" (crashed state)
    init(seed=23);
    // ... crash skier ...
    let s = step(steer=1).state;  // input during crash
    if s.events.contains("input_buffered") {
        buffered_inputs += 1;
    }
}

assert buffered_inputs > 0;  // input buffering works
```

**Pass condition**: Latency < 50ms average, < 100ms max

**Quality rubric**:
- `basic` (100): Latency < 50ms
- `precision` (90): Input buffering (queue during animation locks)
- `performance` (85): Predictive rendering (anticipate input)
- `polish` (80): Profiling data, latency breakdown by input type

**Expected quality**: 70-90

**Failure modes**:
- Blocking I/O in main loop
- No input buffering
- VSync adding latency

---

### AC24: Collision Forgiveness

**Test procedure**:
```rust
init(seed=24);

// Test near-miss detection
let mut near_misses = 0;
let mut unfair_collisions = 0;

for _ in 0..1000 {
    let steer = steer_near_obstacle(margin=0.5);  // pass within 0.5m
    let s = step(steer).state;
    
    if s.events.contains("near_miss") {
        near_misses += 1;
    }
    
    if s.skier.mode == "crashed" {
        // Check if collision was "unfair" (hitbox too large)
        let obstacle = find_nearest_obstacle(&s);
        let actual_distance = distance(s.skier, obstacle);
        if actual_distance > obstacle.width / 2 + 0.2 {  // 0.2m forgiveness
            unfair_collisions += 1;
        }
    }
}

assert near_misses > 50;          // near-miss detection works
assert unfair_collisions < 10;    // < 1% unfair collisions

// Check hitbox shrinking during grace frames
let mut grace_frame_forgiveness = 0;
for _ in 0..100 {
    init(seed=24);
    // ... crash skier ...
    // During recovery, test if hitbox shrinks
    let s = step(steer=0).state;
    if s.events.contains("grace_frames_active") {
        grace_frame_forgiveness += 1;
    }
}

assert grace_frame_forgiveness > 0;
```

**Pass condition**: Near-miss detection works, < 1% unfair collisions

**Quality rubric**:
- `basic` (100): Near-miss detection, < 1% unfair
- `precision` (90): Hitbox shrinking during grace frames
- `performance` (85): Forgiveness calculation < 1μs
- `polish` (80): Visual feedback (screen shake on near-miss), playtesting data

**Expected quality**: 65-85

**Failure modes**:
- Hitbox too large (unfair collisions)
- No near-miss detection
- No grace frames after crash

---

### AC25: Animation Smoothness

**Test procedure**:
```rust
init(seed=25);

let mut frame_times = vec![];
let mut jitter_events = 0;
let mut last_x = state().skier.x;

for _ in 0..1000 {
    let before = now();
    let s = step(steer=1).state;
    let frame_time = now() - before;
    frame_times.push(frame_time);
    
    // Check for jitter (sub-pixel rendering issues)
    let dx = (s.skier.x - last_x).abs();
    if dx < 0.01 && dx > 0.0 {  // sub-pixel movement
        // Should use interpolation, not snap to pixel
        // This is hard to detect without visual output, so we check frame time variance instead
    }
    
    last_x = s.skier.x;
}

let avg = frame_times.mean();
let variance = frame_times.variance();
let dropped_frames = frame_times.iter().filter(|t| *t > 33.3).count();  // > 2 frames

assert avg < 16.6ms;           // 60fps
assert variance < 4.0;         // low variance (no jitter)
assert dropped_frames < 10;    // < 1% dropped frames

// Check for easing functions
let mut speed_changes = vec![];
let mut last_speed = state().skier.speed;
for _ in 0..100 {
    let s = step(steer=0).state;
    let acceleration = s.skier.speed - last_speed;
    speed_changes.push(acceleration);
    last_speed = s.skier.speed;
}

// Smooth acceleration = low variance in acceleration
let acceleration_variance = speed_changes.variance();
assert acceleration_variance < 0.1;  // smooth easing
```

**Pass condition**: 60fps, low variance, no dropped frames

**Quality rubric**:
- `basic` (100): 60fps, < 1% dropped frames
- `precision` (90): Sub-pixel rendering (interpolation)
- `performance` (85): Easing functions (smooth acceleration/deceleration)
- `polish` (80): Motion blur on fast movement, frame pacing data

**Expected quality**: 70-90

**Failure modes**:
- VSync misalignment
- No interpolation (pixel snapping)
- Blocking operations in render loop

---

### AC26: Deterministic Replay Accuracy

**Test procedure**:
```rust
// Record replay
init(seed=26);
let replay = replay(ticks=1000);
let original_states = collect_states(ticks=1000);

// Replay in fresh process
let mut client2 = spawn_driver();
init(seed=replay.seed);
let mut replayed_states = vec![];
for input in decode_inputs(replay.inputSequence) {
    let s = step(input.steer, input.boost, input.jump).state;
    replayed_states.push(canonical_json(&s));
}

// Compare checksums
assert replay.stateChecksum == sha256(replayed_states.join("\n"));

// Compare states pixel-perfect (within 1 pixel tolerance)
for (original, replayed) in original_states.iter().zip(replayed_states.iter()) {
    let orig: GameState = parse(original);
    let repl: GameState = parse(replayed);
    assert (orig.skier.x - repl.skier.x).abs() < 0.01;  // 1 pixel = 0.01m
    assert (orig.skier.y - repl.skier.y).abs() < 0.01;
}

// Check replay file size
let replay_bytes = replay.inputSequence.len();
assert replay_bytes < 1000;  // < 1KB per 1000 ticks
```

**Pass condition**: Replay matches original within 1 pixel

**Quality rubric**:
- `basic` (100): Replay matches
- `precision` (95): Floating-point drift correction
- `performance` (90): Random seed management (per-system seeds)
- `polish` (85): Compact replay format (< 1KB per 1000 ticks)

**Expected quality**: 70-90

**Failure modes**:
- Floating-point drift over 1000 ticks
- Global random state (not per-system seeds)
- Replay file too large (storing full states instead of inputs)

---

### AC27: Performance Budget

**Test procedure**:
```rust
init(seed=27);
challenge("dense_field", {"obstacleCount": 100});

let mut frame_times = vec![];
let mut allocations = 0;
let mut memory_samples = vec![];

for _ in 0..1000 {
    let before = now();
    let s = step(steer=random()).state;
    let frame_time = now() - before;
    frame_times.push(frame_time);
    
    let profile = profile(window=1);
    allocations += profile.metrics.totalAllocations;
    memory_samples.push(profile.metrics.peakMemoryBytes);
}

let avg = frame_times.mean();
let p99 = frame_times.percentile(99);
let max_memory = memory_samples.max();

assert avg < 16.6ms;              // 60fps
assert p99 < 20ms;                // 99% < 20ms
assert allocations == 0;          // no heap allocations
assert max_memory < 50_000_000;   // < 50MB
```

**Pass condition**: 60fps, 99% < 20ms, zero allocations, < 50MB

**Quality rubric**:
- `basic` (100): 60fps, < 50MB
- `precision` (90): Object pooling (no allocations)
- `performance` (95): Parallel processing (rayon, SIMD)
- `polish` (85): Memory profiling data, optimization notes

**Expected quality**: 60-85

**Failure modes**:
- Heap allocations per frame
- No object pooling
- O(n²) algorithms

---

### AC28: Visual Polish

**Test procedure**:
```rust
init(seed=28);

// Check for particle effects
let mut particle_events = 0;
for _ in 0..100 {
    let s = step(steer=0).state;
    if s.events.contains("particle_spawn") {
        particle_events += 1;
    }
}
assert particle_events > 10;  // particles spawn on events

// Check for screen shake
let mut shake_events = 0;
// ... crash skier ...
let s = state();
if s.events.contains("screen_shake") {
    shake_events += 1;
}
assert shake_events > 0;

// Check for color grading
let mut color_grading = false;
// ... boost to high speed ...
let s = state();
if s.events.contains("color_grading_active") {
    color_grading = true;
}
assert color_grading;

// Check for accessibility features
let mut colorblind_mode = false;
let mut pause_on_focus_loss = false;
// These require config or special commands, so we check if driver supports them
// via the `challenge` command or similar
```

**Pass condition**: Particle effects, screen shake, visual feedback on major events

**Quality rubric**:
- `basic` (100): Particles, shake, feedback
- `precision` (90): Color grading (speed lines, style meter)
- `performance` (85): Particle system (pooling, culling)
- `polish` (80): Accessibility (colorblind mode, pause on focus loss)

**Expected quality**: 50-75

**Failure modes**:
- No particle system
- No screen shake
- No accessibility features

---

## Composite Scoring

```python
def calculate_composite_score(acs, generations, loc, avg_frame_time):
    # Pass rate (40%)
    pass_rate = sum(ac.pass for ac in acs) / len(acs)
    
    # Quality score (30%)
    quality_score = sum(ac.quality for ac in acs) / len(acs)
    
    # Efficiency (20%)
    # Lower is better: lines of code × frame time
    efficiency_raw = loc * avg_frame_time
    efficiency = min(100, 10000 / efficiency_raw)  # normalize to 0-100
    
    # Convergence speed (10%)
    # Fewer generations = better
    convergence = max(0, 100 - (generations * 10))
    
    return (
        pass_rate * 40 +
        quality_score * 30 +
        efficiency * 20 +
        convergence * 10
    )
```

**Example**: 28/28 ACs, avg quality 85, 5000 LOC, 2ms frame time, 6 generations

```
pass_rate = 1.0 → 40
quality = 85 → 25.5
efficiency = 10000 / (5000 × 2) = 1.0 → 20
convergence = 100 - (6 × 10) = 40 → 4

Total = 40 + 25.5 + 20 + 4 = 89.5
```

---

## Implementation Priority

**Phase 1 (AC1-AC16)**: Existing ACs with quality scoring (1-2 weeks)
**Phase 2 (AC17-AC22)**: Tier 2 stress tests (2-3 weeks)
**Phase 3 (AC23-AC28)**: Tier 3 polish (3-4 weeks)

Total: 6-9 weeks for full v1 implementation.
