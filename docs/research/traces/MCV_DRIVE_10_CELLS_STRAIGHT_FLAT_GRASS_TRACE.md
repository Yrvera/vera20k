# MCV Drive — 10 Cells Straight Flat Grass — Trace Report

**Scenario:** Allied AMCV at cell (50,50), facing North (0x00). Right-click moves to (60,50) — pure east,
10-cell straight line, flat clear grass, no obstacles, no other units.

**Trace covers:** command dispatch → pathfinding → initial facing turn → throttle ramp → per-tick
position interpolation → arrival deceleration → final stop at (60,50).

**Sources:** `rulesmd.ini [AMCV]`, `src/sim/movement/movement_commands.rs`,
`src/sim/movement/movement_tick.rs`, `src/sim/movement/movement_step.rs`,
`src/sim/movement/turret.rs`, `src/util/fixed_math.rs`, `src/sim/world/world_commands.rs`,
Ghidra decompile of `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` (fresh),
`FootClass::GetCurrentSpeed @ 0x004DB1A0` (fresh).

**Confidence:** High for INI parsing and Rust code behavior; High for gamemd speed-budget
mechanism (fresh decompile); Medium for exact gamemd tick-count arithmetic (not run live).

---

## INI Reference (rulesmd.ini, merged YR values)

| Key | Value | Source |
|---|---|---|
| `[AMCV] Speed` | `4` | `rulesmd.ini:6980` |
| `[AMCV] ROT` | `5` | `rulesmd.ini:6986` |
| `[AMCV] Locomotor` | `{4A582741-9839-11d1-B709-00A024DDAFD1}` = Drive | `rulesmd.ini:6998` |
| `[AMCV] MovementZone` | `Normal` | `rulesmd.ini:7000` |
| `[AMCV] SpeedType` | *(not set)* — defaults to `Track` via ObjectType parser | `src/rules/object_type.rs` |
| `[AMCV] DeploysInto` | `GACNST` | `rulesmd.ini:6977` |
| `[AMCV] Accelerates` | *(not set)* — defaults `true` per gamemd `TechnoTypeClass::Constructor` | `src/rules/object_type.rs` (no parse yet) |
| `AccelerationFactor` default | `0.03` | `src/rules/object_type.rs:851` |
| `DeaccelerationFactor` default | `0.002` | `src/rules/object_type.rs:855` |
| `SlowdownDistance` default | `500` leptons | `src/rules/object_type.rs:856` |

---

## Stage-by-Stage Trace Table

| # | Stage | gamemd output | Rust output | Verdict |
|---|---|---|---|---|
| 1 | Path length | 11 nodes (10 cells), A* straight east | Same (11 nodes) | UNCHECKED — not run live vs binary |
| 2 | Initial facing byte | 0x00 North | 0x00 North (facing_target set to 0x40) | PASS (facing_target correctly set) |
| 3 | Target facing byte | 0x40 East | 0x40 East | PASS |
| 4 | Turn ticks (ROT=5, 15Hz gamemd) | ~22 binary frames (~1467ms) | ~32 ticks × 22ms = 704ms (45Hz Rust) | DRIFT — different turn duration in wall time; see §ROT below |
| 5 | Speed value used per tick | ~10 leptons/binary-frame | 9.9 leptons/Rust-tick BUT with 3× DEBUG multiplier → 29.7 lep/tick | DRIFT — DEBUG 3× MCV multiplier active in world_commands.rs:73 |
| 6 | Throttle ramp at start | Ramp enabled (Accelerates=true default); starts at 0, ramps via AccelerationFactor | Starts at full `speed` immediately (`current_speed = speed` in initial MovementTarget); accel_factor stamped *after* issue_move_command | DRIFT — Rust starts at full speed on tick 0; gamemd ramps from 0 |
| 7 | Per-tick lepton advancement (tick 0) | 0 (still turning; then ramp starts) | Rust: full speed during turn too (rotation blocks movement for vehicles, so 0 while turning) | PASS for turning phase; DRIFT for ramp start |
| 8 | Deceleration before stop | Yes: within SlowdownDistance=500 leptons (~2 cells), decel by 0.002×max/tick, floor at 30% | Yes: implemented in movement_tick.rs:651–658; MIN_BRAKE_FRACTION=0.3 | UNCHECKED — formula matches but not numerically verified |
| 9 | Final facing byte at stop | 0x40 East (retained from last drive heading) | 0x40 East (facing set during drive, not cleared at arrival) | PASS |
| 10 | Total tick count (move to stop) | ~278 binary frames at 15Hz (~18.5 sec wall) | ~290 Rust ticks at 45Hz for gamemd-accurate speed; ~103 ticks with DEBUG 3× MCV speed | DRIFT — 3× debug multiplier halves travel time; ramp absent |
| 11 | Sound at move command | `VoiceMove=MCVAlliedMove` + `MoveSound=MCVMoveStart` | Not traced (audio layer outside sim/) | UNCHECKED — out of sim scope |
| 12 | Sound at arrival | (none verified for standard drive) | Not traced | UNCHECKED |

---

## ROT Turn Detail

### gamemd mechanism (verified `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`)

The decompile shows `FacingClass__UpdateFacing()` is called at `0x004B1AC1` for each consumed
drive-track point. The heading byte from the track point is shifted left 8 bits:
`sStack_28 = (ushort)(byte)heading << 8` → this is a 16-bit facing delta, not the 8-bit body
facing. Body rotation via `DriveLocomotionClass__Update_Facing_From_Type @ 0x004B04D0` reads
`TechnoTypeClass+0x11C` (the ROT field) and calls `RateTimer__Set`. Process @ 0x004B0500 reads
ROT at tick start and starts a 3-tick CDTimer if it changes.

### Rust mechanism (`src/sim/movement/turret.rs:rot_to_facing_delta`)

```rust
// ROT degrees/frame * 15 frames/sec = degrees/sec
// degrees/sec * tick_ms/1000 = degrees this tick
// degrees * 256/360 = facing units this tick
let numerator: u64 = rot as u64 * 256 * 15 * tick_ms as u64;
let denominator: u64 = 360 * 1000;
let delta: u64 = numerator.div_ceil(denominator);
```

At 45 Hz (tick_ms = 22), ROT=5:
- numerator = 5 × 256 × 15 × 22 = 422,400
- denominator = 360,000
- delta = ceil(1.173) = **2 facing units/tick**

Turn from 0x00 to 0x40 = **64 facing units** → ceil(64/2) = **32 ticks = 704ms**

### gamemd equivalent

gamemd runs at 15 binary frames/sec. ROT=5 produces approximately:
- `5 × 256 / 360 ≈ 3.56` → truncated to **3 facing units/binary-frame**
- Turn from 0x00 to 0x40: ceil(64/3) = **22 frames = 1467ms**

**DRIFT:** The turn duration is 704ms in Rust vs ~1467ms in gamemd. Roughly 2× faster in Rust.
Root cause: Rust's 45Hz tick rate is 3× faster than gamemd's 15Hz frame rate, but the
`rot_to_facing_delta` formula scales by 15fps explicitly, giving `5×15 = 75 deg/sec` regardless
of tick rate. The player observes the MCV turning faster than in the original game.

Verdict: **DRIFT (MEDIUM priority)** — visible every match when a player moves an MCV from a
non-east heading.

---

## Speed Detail

### gamemd mechanism (verified `FootClass::GetCurrentSpeed @ 0x004DB1A0`)

```
speed_budget_per_tick = GetTypeSpeed() * SpeedFraction * HouseSpeedBonus
                      + GetCurrentSpeed terrain/health factors
```

`GetTypeSpeed` reads `TechnoTypeClass+0x678`. The INI `Speed=4` value is stored there.
From the verified formula:
- `leptons_per_tick = floor(Speed_raw × 256 / 100)` = `floor(4×256/100)` = `floor(10.24)` = **10**
- This is the raw budget per binary frame (15Hz gamemd).

### Rust mechanism (`src/util/fixed_math.rs:ra2_speed_to_leptons_per_second`)

```rust
let capped = raw_speed.min(100);                         // 4
let leptons_per_tick = (capped * 256 / 100).min(255);   // 10
SimFixed::from_num(leptons_per_tick * 15)               // 150 leptons/second
```

The formula correctly converts Speed=4 to 150 leptons/second (equivalent to 10 lep/frame at 15Hz).
At 45Hz ticks (22ms), this advances `150 × 0.022 = 3.3 leptons/tick`.

### Critical DRIFT: DEBUG 3× MCV speed multiplier (`src/sim/world/world_commands.rs:73`)

```rust
// DEBUG: 3x speed boost for MCVs during development.
let speed_mult = obj.map_or(1, |o| if o.deploys_into.is_some() { 3 } else { 1 });
let base_speed = obj.map(|o| ra2_speed_to_leptons_per_second(o.speed * speed_mult))
```

Because `[AMCV] DeploysInto=GACNST` is set, `deploys_into.is_some()` is `true`.
Effective raw speed = 4 × 3 = **12** → `ra2_speed_to_leptons_per_second(12)` = 450 lep/sec.

Per-tick advancement at 45Hz: `450 × 0.022 = 9.9 leptons/tick`.

**DRIFT (HIGH priority, fires every match when a player moves an MCV):** MCV travels at 3× gamemd
speed. All other drive units are unaffected. This is a known development stub that must be removed
before any parity-correctness claim on MCV movement.

---

## Throttle Ramp / Acceleration Detail

### gamemd mechanism

`[AMCV]` has no `Accelerates=false`. `TechnoTypeClass::Constructor` defaults `+0xDBD = 1` (true).
`DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` takes the ramp/brake branch:
- current < target: `current += AccelerationFactor × tick` (ramps up)
- within SlowdownDistance=500: `current -= DeaccelerationFactor × speed × tick` (ramps down, floor 30%)

AMCV does not have INI `AccelerationFactor=` or `DeaccelerationFactor=`, so defaults apply:
- AccelerationFactor default in gamemd: not directly verified in this slot; prior docs suggest ~0.03
  per binary frame (ramp to full in ~2 sec)
- DeaccelerationFactor default: ~0.002 per binary frame

### Rust mechanism

`issue_move_command_with_layered` creates `MovementTarget` with `current_speed: speed` (full speed
on tick 0). Then `world_commands.rs:267-269` stamps `accel_factor=0.03`, `decel_factor=0.002`,
`slowdown_distance=500` *after* the target is created. But `current_speed` is already `= speed`,
not `= SIM_ZERO`.

The `movement_tick.rs:626` ramp runs only if `accel_factor > 0 || decel_factor > 0`, and
`target.current_speed < target.speed` must be true to accelerate. Since `current_speed == speed`
from the start, the acceleration branch is never entered on tick 0 — the unit moves at full speed
immediately.

**DRIFT (MEDIUM priority):** Rust MCV starts at full speed; gamemd ramps from zero. The
acceleration phase (gamemd: ~22 binary frames / ~1.5 sec for AccelerationFactor=0.03) is missing.
This is observable: in gamemd the MCV visibly accelerates from a standing start; in Rust it leaps
to full speed instantly. Affects every movement command issued from rest.

Note: `Accelerates=false` is not yet parsed in Rust (`src/rules/object_type.rs` has no
`accelerates` field, confirmed by `GRIZZLY_ACCELERATES_FALSE_SEMANTICS_GHIDRA_REPORT.md`), but
since AMCV defaults to `Accelerates=true`, the missing parse is not the cause here — the bug is
that `current_speed` starts at `speed` rather than `SIM_ZERO`.

---

## Per-Tick Lepton Position (Sample Points)

Computed for Rust *without* DEBUG multiplier (i.e., corrected speed 150 lep/sec, 3.3 lep/tick),
starting from cell (50,50) center = lepton position (12928, 12928) + (128, 128) = (13056, 12928)
(east movement: sub_x advances, sub_y constant at center).

Turn phase: ticks 0–31, sub-position at (128, 128), cell stays (50,50).
Movement phase starts at tick 32.

| Tick | Lepton sub_x (approx) | Cell x | Notes |
|---|---|---|---|
| 0 | 128 | 50 | At cell center, turning |
| 5 | 128 | 50 | Still turning |
| 31 | 128 | 50 | Last turn tick |
| 32 | 131 | 50 | Movement starts; sub_x += 3.3 |
| 37 | 148 | 50 | 5 movement ticks |
| ~57 | 256 | 50→51 | Cell boundary crossed (128+~128 = 256); rx becomes 51, sub_x resets to ~0 |
| ~135 | ~128 | 55 | Midpoint — ~77 ticks / 5 cells |
| ~212 | ~128 | 59 | 9 cells done; 1 cell remaining |
| ~233 | ~48 | 60 | Within SlowdownDistance=500 lep from center of cell 60 |
| ~257 | ~128 | 60 | Arrival; movement_target cleared |

*(These are rough estimates from 3.3 lep/tick × ticks; exact values depend on SimFixed Q16.16
rounding. Not verified against live binary output — UNCHECKED.)*

**With DEBUG 3× multiplier (9.9 lep/tick):** same milestones reached in ~1/3 the ticks:
- Cell boundary crossed at tick ~45 (not ~57)
- Total travel: ~80 ticks (not ~225)

---

## Drive Track vs Straight-Line Movement

**gamemd:** Uses drive-track curves (precomputed curve table at `0x007E7A28`). Budget per
binary-frame = `GetCurrentSpeed()` ≈ 10 units. Each track point costs 7 budget units → ~1.4
points/frame for straight movement. The curve table has straight-line tracks for cardinal
directions (dx=+1, dy=0 = East).

**Rust:** `advance_lepton_position` in `movement_step.rs:267` checks for `drive_track_state`.
Drive tracks are selected in `configure_motion_after_transition` (`movement_step.rs:93`) when
`new_face != *facing` AND the locomotor is `LocomotorKind::Drive`. For straight east (no turns),
`new_face == *facing` after the initial turn, so **no drive track is selected** — Rust uses
straight-line vector advancement (`lepton_step = effective_speed × dt`).

gamemd also effectively advances straight for cardinal movement (the curve is degenerate, moving
directly to the target cell with points at equal sub-positions). The observable behavior (cell
boundary crossing timing, visible pixel position) should match for the straight-line case.
However, the budget mechanism differs: gamemd's 7-budget/residual system provides fractional
interpolation between track points; Rust uses direct lepton accumulation.

Verdict for straight case: **UNCHECKED** — the two mechanisms are algebraically different but
may produce near-identical output for cardinal movement. Not verified with side-by-side pixel
comparison.

---

## Verdict Tally

| Status | Count |
|---|---|
| DRIFT | 4 |
| PASS | 3 |
| UNCHECKED | 5 |

---

## Top 5 Player-Visible Failures

1. **DEBUG 3× MCV speed (HIGH, fires every match):** MCV moves at 3× gamemd speed. `world_commands.rs:73`. Remove `speed_mult` MCV special-case before shipping.

2. **Missing acceleration ramp (MEDIUM, fires every move from rest):** MCV starts at full speed in Rust; gamemd ramps from zero over ~1.5 sec. `current_speed = speed` in initial `MovementTarget` should be `SIM_ZERO` (with `Accelerates=true` and nonzero `accel_factor`).

3. **Turn duration 2× too fast (MEDIUM, fires every match when not already facing east):** ROT=5 at 45Hz Rust produces 704ms turn vs ~1467ms in gamemd. Root cause: gamemd's ROT formula is per binary-frame (15fps), but Rust runs at 45Hz. The `rot_to_facing_delta` formula scales by `×15` but ticks fire 3× more often.

4. **Accelerates=false not parsed (LOW, affects Grizzly/MTNK and others — not AMCV):** AMCV defaults `Accelerates=true` so this doesn't affect this trace, but `src/rules/object_type.rs` has no `accelerates` field (confirmed by `GRIZZLY_ACCELERATES_FALSE_SEMANTICS_GHIDRA_REPORT.md`). Adjacent finding, listed for completeness.

5. **Initial `current_speed` stamping is order-dependent (LOW, structural):** `accel_factor`/`decel_factor` are stamped onto `MovementTarget` *after* `issue_move_command_with_layered` creates it with `current_speed = speed`. Any caller that reads `current_speed` before the stamp sees full speed. `world_commands.rs:262-273`.

---

## Adjacent Findings (Out of Trace Scope)

- **AMCV has no turret** (`Turret=` not set in INI), so body ROT applies to the entire hull.
  The turret rotation system in `turret.rs` correctly skips non-turret units.
- **Crusher=yes on AMCV** (`rulesmd.ini:6988`): AMCV can crush infantry. Relevant to
  occupancy checks during movement but out of scope for this flat-grass trace.
- **OmniCrushResistant=yes** (`rulesmd.ini:7009`): AMCV cannot be crushed by OmniCrushers.
  Out of scope.

---

## Summary Status

The AMCV Drive locomotor pipeline is structurally complete in Rust (pathfinding → turn → velocity
advance → cell crossings → deceleration → arrival). Three parity gaps are active and
player-visible:

1. `world_commands.rs` DEBUG 3× speed multiplier — must be removed.
2. Initial `current_speed = speed` instead of `SIM_ZERO` — breaks acceleration ramp.
3. ROT turn duration at 45Hz is 2× shorter than at 15Hz gamemd — formula interaction with tick rate.

Remaining stages (deceleration formula, final cell snap, sound cues) are UNCHECKED and require
side-by-side binary comparison to complete.
