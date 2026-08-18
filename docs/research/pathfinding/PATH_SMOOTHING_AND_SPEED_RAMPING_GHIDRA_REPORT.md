# Path Smoothing & Speed Ramping — Ghidra Research Report

Source: Ghidra MCP (gamemd.exe), live decompilation. All addresses verified.
Confidence: **HIGH** — all functions fully decompiled and traced.

---

## 1. Path Smoothing Pipeline

After A* completes and `AStar_reconstruct_path` (0x42aa90) produces the raw direction
array, two smoothing passes are applied in sequence:

```
AStar_main_loop (0x429a90)
  └─ on success:
       1. AStar_reconstruct_path (0x42aa90)    → raw direction array
       2. Path_smooth_corners (0x42b210)        → remove 90-degree zigzags
       3. Path_optimize_straight_segments (0x42b7f0) → straighten drifting runs
```

Both operate on a **direction array** (not cell coordinates). Each entry is an int:
- 0-7: compass direction (in iso cell-space)
- 8: bridge/tunnel crossing
- -1 (0xFFFFFFFF): end sentinel
- -2 (0xFFFFFFFE): deleted/skip marker (used during optimization, compacted at end)

### Direction Encoding (smoothing convention; source corrected 2026-06-01)

```
Direction 0 = S   (cell dy=+1, dx=0)
Direction 1 = SW  (cell dy=+1, dx=-1)
Direction 2 = W   (cell dy=0,  dx=-1)
Direction 3 = NW  (cell dy=-1, dx=-1)
Direction 4 = N   (cell dy=-1, dx=0)
Direction 5 = NE  (cell dy=-1, dx=+1)
Direction 6 = E   (cell dy=0,  dx=+1)
Direction 7 = SE  (cell dy=+1, dx=+1)
Direction 8 = bridge/tunnel jump (special, non-adjacent cells)
```

**SOURCE CORRECTION (2026-06-01):** The `0x00818760` table is not the dx/dy source for
this mapping. Static bytes there begin `ff ff ff ff 06 00 00 00 ...`, matching the
earlier warning that it is a remap/index table, not coordinate deltas. The smoothing
helpers step cells through `g_DirectionOffsets @ 0x0089F688` (`MapCoord_Step_By_Direction`
and direct reads in `Path_Reroute_Straight_Line`), while the A* neighbor table at
`0x007E3774` is a separate CellClass-pointer offset table. Treat the mapping above as
the smoothing/Drive direction convention, not as a fact proven by `0x00818760`
(corrected 2026-06-01: was "verified from lookup table at 0x818760"; binary shows the
smoothing step source is `g_DirectionOffsets @ 0x0089F688` via `decompile_function
0x0042D490` and `read_memory 0x00818760` - OFFSET_RETYPED_WRONG).

---

## 2. Pass 1: Path_smooth_corners (0x42b210)

**Purpose:** Replace 90-degree zigzag pairs with diagonal shortcuts.

**Algorithm:**

Iterates through the direction array tracking runs of identical directions. When it
encounters a direction change of exactly +/-2 (modulo 8) — i.e., a 90-degree turn —
it groups consecutive steps of the new direction and calls `Path_smooth_single_segment`
to attempt replacement.

**Key logic (pseudocode):**
```
prev_dir = NONE
run_start = 0
run_length = 0
zigzag_length = 0
in_zigzag = false

for each step in direction_array:
    if in_zigzag:
        if step == zigzag_dir:
            zigzag_length++     // extend the zigzag run
        else:
            // End of zigzag — try to smooth it
            Path_smooth_single_segment(
                unit, &directions[run_start], &heights[run_start],
                run_length, zigzag_length, &current_pos
            )
            in_zigzag = false
    else:
        diff = (step - prev_dir) & 7
        if step == prev_dir:
            run_length++        // same direction, extend run
        else if diff == 2 or diff == 6:
            // 90-degree turn detected — start zigzag tracking
            if prev_dir is diagonal (odd):
                in_zigzag = true
                zigzag_length = 1
                zigzag_dir = step
        else:
            // Direction change > 90 degrees — reset
            run_length = 1
            prev_dir = step (only if step is diagonal)
```

**Critical detail:** Only diagonal (odd-numbered) directions trigger zigzag detection.
Cardinal directions (even) are never the "anchor" of a zigzag. This means the engine
only smooths zigzags like "NE then N" or "SE then E" — not "N then E" directly. The
anchor must be a diagonal.

### Path_smooth_single_segment (0x42b420)

Called when a zigzag is detected. Attempts to replace the zigzag pattern with the
intermediate diagonal direction.

**Parameters:**
- `unit` (FootClass*): for passability checks
- `directions`: pointer into direction array at the run start
- `heights`: parallel height array
- `run_length`: number of steps in the first direction
- `zigzag_length`: number of steps in the second direction
- `current_pos`: pointer to current cell coordinate (updated in-place)

**Algorithm:**
1. Compute the midpoint direction: `mid = (dir_a + dir_b) / 2` — this is the diagonal
   between the two 90-degree-apart directions
2. Determine how many steps can be replaced: `min(run_length, zigzag_length)`
3. For each replacement step, walk the midpoint direction and validate:
   - Call `Can_Enter_Cell` (vtable+0x1AC) to check passability
   - Check cell flags for cliff ramps (`cell+0x140 & 0x40000`)
   - Check slope compatibility using the slope lookup at FootClass+0x87*4 (the
     height-step slope table): `FUN_0056bcd0(&pos, slope_table)`
   - If slope factor exceeds 1.0 (via `FUN_004dc760` slope multiplier), reject
4. If ANY cell along the shortcut is blocked or too steep, the smoothing is rejected
   for this segment (falls through to keep the original zigzag)
5. On success: replaces the direction entries with the midpoint direction, overwriting
   both the original run and zigzag portions. Updates `current_pos` accordingly.
6. Direction 8 (bridge crossings) are NEVER smoothed — the function immediately skips
   to walking the original directions when either direction is 8.

---

## 3. Pass 2: Path_optimize_straight_segments (0x42b7f0)

**Purpose:** Detect segments that drift from a straight line, then replace them with
a true straight-line decomposition (cardinal + diagonal interleave).

**Maximum scope:** Operates on up to 20 steps (`0x13 < iVar13` check at top of loop).

**Algorithm:**

Tracks cumulative displacement vectors as it walks the direction array:
- `cum_offset` = total displacement from segment start (cell-delta vector)
- `best_offset` = displacement at the peak Chebyshev distance from start
- Maintains a "segment start" position and an "anchor" position

**Drift detection:**
```
For each step:
    new_offset = cum_offset + direction_delta[step]
    chebyshev = max(abs(new_offset.x), abs(new_offset.y))

    if chebyshev < previous_best:
        // We've moved CLOSER to start than our furthest point
        // This means the path is curving back — drift detected!
        Record anchor point, start tracking new segment
```

When drift is detected or at the end of the 20-step window, two helper functions
are called:

### FUN_0042bca0 — Find optimal split point

Walks backward from the current position to the segment anchor, tracking the
displacement vector. Finds the point of maximum Chebyshev distance from anchor,
which becomes the split point for rerouting. Computes the rotated direction
(offset by 4, i.e., 180 degrees) for the replacement path direction.

### FUN_0042be20 — Attempt straight-line rerouting

Given a segment to replace, decomposes the ideal displacement into:
- `diag_steps = min(abs_dx, abs_dy)` — number of diagonal steps
- `card_steps = abs(abs_dx - abs_dy)` — number of cardinal steps
- Selects appropriate diagonal direction (one of 4 diagonals based on sign)
- Selects appropriate cardinal direction (N/S/E/W based on longer axis)

Then walks the proposed straight path, validating EVERY cell:
```
For each step in proposed path:
    cell = MapClass__Get_CellClass(&pos)
    move_result = unit->Can_Enter_Cell(cell, direction, height, 0, 1)

    // Reject if blocked
    if (move_result != 0) OR (cell_flags & 0x40000 != 0):
        blocked = true

    // Slope check: count steep slope transitions
    slope_factor = FUN_0056bcd0(&pos, slope_table)
    if slope_factor * slope_multiplier >= threshold:
        steep_count++

    // Mid-scan reroutes allow 0 steep cells; final end-of-window reroute allows up to 3
    if (steep_count >= 4) OR (!is_end_of_scan_window AND steep_count >= 1):
        blocked = true
```

(corrected 2026-06-01: was "up to 3 steep slopes normally, 0 if at path end"; binary
shows `param_7 == 0` mid-scan calls block on the first steep cell, while `param_7 == 1`
final-window calls allow `steep_count < 4`, via `decompile_function 0x0042BE20` and
`decompile_function 0x0042B7F0` - OPERATOR_OR_ORDER_DRIFT)

**On success:** Replaces the segment's direction entries with the straight-line
decomposition (diagonal steps first, then cardinal steps). Excess entries are marked
-2 (0xFFFFFFFE).

**Compaction:** After all optimization passes, the function does a final scan of the
direction array, removing all -2 entries and compacting the array. Updates the path
length accordingly.

**Tries both orderings:** If diagonal-first fails passability, the function swaps
the cardinal and diagonal directions and tries the opposite order (cardinal-first).
Two attempts total (`local_10` counter, exits when `local_10 >= 2`).

---

## 4. Comparison: Our Rust Implementation vs Original

### Pass 1 (Zigzag smoothing)

| Aspect | Original (gamemd.exe) | Our Rust (path_smooth.rs) |
|--------|----------------------|--------------------------|
| Input format | Direction array (ints) | Cell coordinate array |
| Zigzag detection | Direction diff == 2 mod 8 | Same: `dir_diff(d0,d1) == 2` |
| Anchor constraint | Only diagonal dirs anchor zigzags | None — any direction can anchor |
| Passability check | `Can_Enter_Cell` + slope check + cliff check | Simple `walkable(x,y)` closure |
| Bridge handling | Direction 8 skipped entirely | Layer transitions block smoothing |
| Slope validation | Height-step slope table lookup | Not implemented |

### Pass 2 (Drift correction / straight-line optimization)

| Aspect | Original (gamemd.exe) | Our Rust (path_smooth.rs) |
|--------|----------------------|--------------------------|
| Trigger | Chebyshev distance decreasing (path curves back) | Cross-product drift threshold |
| Max steps | 20 steps hard limit | `MAX_OPTIMIZE_STEPS = 20` (matches) |
| Rerouting | Cardinal+diagonal decomposition, tries both orderings | Similar decomposition, one ordering |
| Validation | `Can_Enter_Cell` + cliff + slope (0 steep cells mid-scan; up to 3 only in final end-of-window reroute; corrected 2026-06-01 via `decompile_function 0x0042BE20` - OPERATOR_OR_ORDER_DRIFT) | Simple `walkable(x,y)` closure |
| Unused entries | Marked -2, compacted at end | Splice-based removal |

**Key differences to address:**
1. Our zigzag smoothing allows cardinal anchors; original only allows diagonal anchors
2. Original has slope-aware validation during smoothing (steep terrain blocks shortcuts)
3. Original tries both diagonal-first and cardinal-first orderings for rerouting
4. Original drift detection uses Chebyshev regression (distance shrinks) rather than
   cross-product threshold

---

## 5. Speed Ramping in DriveLocomotionClass

### TechnoTypeClass Speed Fields

| Offset | Type | INI Key | Purpose |
|--------|------|---------|---------|
| +0x2F8 | int | `SlowdownDistance` | Distance (leptons) at which braking begins |
| +0x300 | double | `DeaccelerationFactor` | Deceleration rate multiplier |
| +0x308 | double | `AccelerationFactor` | Acceleration rate multiplier |
| +0x370 | double | `Weight` | (nearby field, not directly speed-related) |
| +0xDBD | bool | `Accelerates` | Whether the unit uses speed ramping at all |

### DriveLocomotionClass Speed Fields

| Offset | Type | Init | Field |
|--------|------|------|-------|
| +0x50 | double | 0.0 | `target_speed_fraction` — DriveLocomotionClass target/baseline fraction (0.0 to 1.0 range), copied directly to Techno+0x578 for non-accelerating types and used as the clamp target during ramping (corrected 2026-06-01: was `current_speed`; binary uses Techno+0x578 as the mutable current fraction and +0x50 as the ramp target via `decompile_function 0x004B0F20` - PARAM1_TYPE_MISREAD) |
| +0x4C | int | 0 | `residual_ticks` — leftover movement budget from last tick |

### FootClass/TechnoClass Speed Fields

| Offset | Type | Field |
|--------|------|-------|
| +0x578 (word +0x15E) | double | `speed_percentage` — current speed fraction (0.0 to 1.0), set by SetSpeedPercentage (vtable+0x544) |
| +0x3CD | byte | `is_decelerating` flag |
| +0x6B5 | byte | `is_braking` flag (crawl speed mode) |

**Speed field relationship:**
- `loco+0x50` (`target_speed_fraction`) = the DriveLocomotionClass's target/baseline
  speed fraction; non-accelerating types copy it directly to `techno+0x578`, while
  accelerating types ramp `techno+0x578` toward it (corrected 2026-06-01: was described
  as local current speed; binary shows +0x50 is the clamp target via `decompile_function
  0x004B0F20` - PARAM1_TYPE_MISREAD)
- `techno+0x578` (`speed_percentage`) = the TechnoClass speed fraction (0.0-1.0) that
  is mutated by `SetSpeedPercentage`, read each tick by `GetCurrentSpeed` (vtable+0x538),
  and converted to a lepton-based movement budget
- `SetSpeedPercentage` (vtable+0x544, at 0x4D3710) writes directly to +0x578, clamping
  to [0.0, 1.0]
- `GetCurrentSpeed` (vtable+0x538, at 0x4DB1A0) reads +0x578, multiplies by the type's
  base speed and terrain factors, returns leptons-per-tick

### Speed State Machine (Process_Drive_Track, 0x4B0F20)

The speed computation runs when `track_step < 0x40` (i.e., not a special track).
It computes the 3D distance from the unit's current position to its destination,
then applies one of several speed adjustment paths:

```
distance = 3D_distance(unit_pos, destination)
current_speed = techno.speed_percentage   // +0x578 (double at +0x15E word offset)
target_speed = loco.target_speed_fraction // +0x50
decel_steps = techno->GetDecelSteps()     // vtable+0x38C

// BRANCH 1: Within braking distance
if distance < technoType.SlowdownDistance:           // +0x2F8
    new_speed = current_speed - decel_steps * technoType.DeaccelerationFactor  // +0x300
    new_speed = max(new_speed, 0.3)                  // floor at 0.3

// BRANCH 2: is_decelerating flag set
else if techno.is_decelerating:                      // +0x3CD
    new_speed = current_speed - decel_steps * 0.0015 // hardcoded alt decel rate
    new_speed = max(new_speed, 0.1)                  // floor at 0.1

// After computing new_speed, apply based on current state:

if techno.is_braking:                                // +0x6B5
    // Crawl mode: cap at 0.2
    new_speed = min(0.2, target_speed)
    loco.target_speed_fraction = new_speed
    techno.SetSpeed(new_speed)

else if decelerating:
    // Within braking distance or is_decelerating flag — apply computed target
    techno.SetSpeed(new_speed)

else if current_speed < target_speed:
    // BELOW target speed — accelerate by AccelerationFactor, clamped to target
    new_speed = current_speed + technoType.AccelerationFactor  // +0x308
    if target_speed < new_speed:
        new_speed = target_speed
    techno.SetSpeed(new_speed)

else if target_speed < current_speed:
    // ABOVE target speed — decelerate by DeaccelerationFactor, clamped to target
    new_speed = current_speed - decel_steps * technoType.DeaccelerationFactor
    if new_speed < target_speed:
        new_speed = target_speed
    techno.SetSpeed(new_speed)
```

(corrected 2026-06-01: branch comments had the above/below-target cases inverted and
treated Techno+0x578 as max speed; binary shows Techno+0x578 is the mutable current
fraction and DriveLocomotion+0x50 is the target clamp via `decompile_function 0x004B0F20`
and `disassemble_function 0x004DB1A0` - PARAM1_TYPE_MISREAD)

### Speed Constants (verified from memory dumps)

| Address | Type | Value | Purpose |
|---------|------|-------|---------|
| 0x7e6240 | double | 0.3 | Minimum speed during braking (Branch 1 floor) |
| 0x7e6248 | double | 0.1 | Minimum speed during decel flag (Branch 2 floor) |
| 0x7e6250 | double | 0.0015 | Alternative deceleration rate (Branch 2) |
| 0x7e3548 | double | 0.2 | Crawl speed (is_braking mode cap) |

### Movement Budget Per Tick

```c
speed_value = techno->GetCurrentSpeed()    // vtable+0x538, at 0x4DB1A0
if (is_retry_tick):
    budget = 0 + loco.residual_ticks      // no speed contribution on retry
else:
    budget = speed_value + loco.residual_ticks
```

The budget is consumed as the unit steps through drive track deltas. Any leftover
is stored back in `loco.residual_ticks` for the next tick.

---

## 6. DriveLocomotionClass::Process State Machine (0x4B0500)

The Process function is the top-level tick handler, called every frame from
`FootClass::AI`. It has two major branches:

### Branch A: Active drive state (`loco+0x58` [track_index] `!= -1` AND byte `loco+0x63` [is_on_track] `!= 0`)

```
1. Process_Drive_Track(0)              // Execute current track
2. If still alive and the active marker has cleared (`loco+0x58 == -1`) but movement/path
   checks still require another segment:
   a. Process_Movement(&result, 1, 0)  // Select next track
   b. Process_Drive_Track(1)           // Execute with retry flag
```

(corrected 2026-07-12: the 2026-06-01 correction treated Process()'s raw `ESI`-relative
reads (`ESI+0x54`, `ESI+0x5F`) as fields distinct from `track_index`/`is_on_track`
(`Process_Drive_Track`'s `param_1+0x58`/`+0x63`). They are the SAME two fields: the call
site `MOV ECX,EDI` / `CALL 0x004b0f20` at 0x004b0574 passes `EDI = ESI-4` as
`Process_Drive_Track`'s "this" (`LEA EDI,[ESI+-0x4]` at 0x004b0518), so `param_1+0x58`
== `ESI+0x54` and `param_1+0x63` == `ESI+0x5F` byte-for-byte — not four distinct fields.
Restored to the doc's own `track_index`/`is_on_track` convention (already used in the Key
State Fields table below) via `disassemble_function 0x004B0500` - PARAM1_TYPE_MISREAD)

### Branch B: No active drive state (`loco+0x58 == -1` OR byte `loco+0x63 == 0`)

```
1. Check various abort conditions:
   - NavTarget mission == 0xB (entering building)
   - Current mission == 5 (Move) with destination == current pos
   - Drive delay timer still active
2. If delay expired:
   - Clear delay flag
   - Set mission to 0 (Idle)
3. Try Is_Moving check
4. Check reachability (vtable+0x2CC)
5. Process_Movement(&result, 1, 0)     // Pathfind and select track
6. If movement succeeded and still alive:
   Process_Drive_Track(continuation)    // Execute selected track
```

### Phase 1: Slope Detection

Every tick, reads the cell's SlopeIndex (`CellClass+0x11C`) and compares against
the cached current slope at `loco+0x1C`. On change:
- Copies old `loco+0x1C` to previous-slope `loco+0x20`
- Writes the new SlopeIndex to `loco+0x1C`
- Starts a 3-frame slope transition timer stored at `loco+0x24..0x30` (CDTimerClass)
- `Draw_Matrix` interpolates the visual tilt over 3 frames

(corrected 2026-07-12: the 2026-06-01 pass read these fields via Process()'s raw
`ESI`-relative offsets (+0x18/+0x1C/+0x20..0x2C) without applying the same 4-byte "this"
adjustment documented in the Branch A/B correction above (`EDI = ESI-4` is what
`Process_Drive_Track` actually receives as `param_1`). Converted to the doc's own
`param_1`-relative convention (matching `track_index`/`is_on_track`/etc.): current-slope
write at `[EDI+0x1c]`, previous-slope write at `[EDI+0x20]`, CDTimerClass fields at
`[EDI+0x24]..[EDI+0x30]`, verified via `disassemble_function 0x004B0500`
(instructions 0x004b0518-0x004b0557) - PARAM1_TYPE_MISREAD)

### Key State Fields

| Field | Offset | Purpose |
|-------|--------|---------|
| `track_index` | loco+0x58 | Current TurnTrack index (-1 = none, 0-71 = active) |
| `point_index` | loco+0x5C | Current step within the track data |
| `is_on_track` | loco+0x63 | Whether actively following a track (bool) |
| `is_reversed` | loco+0x60 | Using short/reverse variant of track |
| `target_speed_fraction` | loco+0x50 | DriveLocomotionClass ramp target/baseline fraction (corrected 2026-06-01: was `current_speed`; binary ramps Techno+0x578 toward this field via `decompile_function 0x004B0F20` - PARAM1_TYPE_MISREAD) |
| `residual_ticks` | loco+0x4C | Leftover movement budget |
| `destination` | loco+0x34 | Target coordinate (X,Y,Z) |
| `head_to` | loco+0x40 | Next waypoint coordinate (X,Y,Z) |

---

## 7. Summary of Key Findings

### Path Smoothing
1. Two passes applied after A* search, before the path enters the 24-entry queue
2. Pass 1 (zigzag smoothing) only anchors on diagonal directions; uses Can_Enter_Cell +
   slope validation
3. Pass 2 (straight-line optimization) detects drift by Chebyshev regression, reroutes
   with cardinal+diagonal decomposition, tries both orderings, validates with
   Can_Enter_Cell + slope + cliff checks (0 steep cells allowed in mid-scan reroutes,
   up to 3 only in the final end-of-window reroute; corrected 2026-06-01 via
   `decompile_function 0x0042BE20` - OPERATOR_OR_ORDER_DRIFT)
4. Maximum 20 steps analyzed in pass 2
5. Bridge crossings (direction 8) are NEVER smoothed in either pass

### Speed Ramping
1. Speed is a double in range 0.0 to 1.0 (fraction of max speed)
2. Three speed zones based on distance-to-destination:
   - **Far:** Ramp Techno+0x578 toward DriveLocomotion+0x50 (AccelerationFactor upward,
     DeaccelerationFactor downward)
   - **Near (< SlowdownDistance):** Brake using DeaccelerationFactor, floor at 0.3
   - **Decel flag:** Gentle decel at 0.0015, floor at 0.1
3. Crawl mode (is_braking flag): cap at 0.2
4. Speed is NOT a state machine with discrete states — it's continuous adjustment
   each tick based on distance and flags. The "7-state machine" referenced in
   WalkLocomotionClass is a different system (WalkLocomotionClass +0x50).

### DriveLocomotionClass does NOT have a 7-state walk machine
The DriveLocomotionClass uses a simpler two-branch Process model:
- Has active track → Process_Drive_Track (continuous speed adjustment)
- No active track → pathfind, select track, begin movement

The 7-state machine concept comes from **WalkLocomotionClass** (infantry), not
DriveLocomotionClass (vehicles). Our `GroundMovePhase` enum in `locomotor.rs`
documents this as applying to WalkLocomotionClass +0x50.

---

## 8. Verification Addendum (2026-04-05)

Re-verified the two smoothing passes and their helpers in Ghidra. Findings below
either **correct** or **extend** §2 and §3 above.

### 8.0 Ghidra function labels added this pass

These addresses are now named in the saved Ghidra database:

| Address | Name | What it does |
|---------|------|--------------|
| `0x0042be20` | `Path_Reroute_Straight_Line` | §3's "FUN_0042be20" — the cardinal+diagonal decomposer with 2-ordering retry (`local_10 < 2` loop). Validates each step via Can_Enter_Cell + cliff flag (0x40000) + steep-slope count. |
| `0x0042bca0` | `Path_Find_Split_Anchor` | §3's "FUN_0042bca0" — walks the direction array backward using reversed directions, returns the Chebyshev-peak inflection point as split anchor for rerouting. |
| `0x0056bcd0` | `MapClass__Get_Slope_Cost_At_Cell` | §8.4's `FUN_0056bcd0` — reads per-SpeedType coarse slope cost map at 4-cell resolution, indexed as `(y/4)*0x82 + (x/4)`, offset `0x59F0` into the per-unit table at `FootClass+0x21C`. |
| `0x004dc760` | `FootClass__Get_Slope_Speed_Factor` | §8.4's `FUN_004dc760` — returns `*(double *)(foot + 0x530)` normally, overrides to 1.0 if tether partner's type has flag `+0xF2`. Multiplied against slope cost in steep-slope check. |
| `0x0042d490` | `MapCoord_Step_By_Direction` | Advances a MapCoord by one cell in the given 8-direction. Handles direction 8 (tube jump) via `g_TubeArray` lookup at `0x008b413c`. |
| `0x0042d510` | `MapCoord_Add` | Trivial (x,y) vector add of two MapCoord structs. |
| `0x0042d470` | `MapCoord_Set` | Trivial (x,y) setter into short[2] buffer. |

Names reference binary behavior verified in this pass. Any future pointer to "the
cardinal+diagonal rerouter" etc. in this report refers to these Ghidra symbols.

### 8.1 Call chain confirmed

`AStar_main_loop @ 0x00429a90` calls the smoothing pipeline on A* success at
`0x0042a40b`–`0x0042a41e`:

```
AStar_reconstruct_path(piStack_48, param_5)   →  raw direction array
Path_smooth_corners(result, piVar2)           →  Pass 1: zigzag removal
Path_optimize_straight_segments(result, piVar2)  →  Pass 2: drift correction
if (pathfinder->urgency != 0)
    PathfinderClass__UpdateBridgePassability(foot)
return result
```

Both passes are **unconditional** for any successful A* result (no gate flag).
Active in all YR skirmishes.

### 8.2 Pass 1 anchor-reset rule — VERIFIED

At `Path_smooth_corners @ 0x0042b210`, the "only diagonals anchor zigzags" rule
is enforced by:

```c
// When current step doesn't match prev and isn't ±90°:
iVar8 = 1;
uVar7 = uVar2;             // uVar7 = new prev_dir
if ((uVar2 & 1) == 0) {    // cardinal direction (even)
    uVar7 = 0xffffffff;    // reset anchor to "none"
}
```

So cardinal steps (0/2/4/6) blank the anchor state. Only diagonal (odd) directions
persist as candidate zigzag anchors. This matches the existing §2 claim.

### 8.3 Direction-8 tube-jump handling — CORRECTED 2026-06-01

Direction 8 represents a bridge/tunnel cell-to-cell jump through a `TubeClass` entry.
When encountered by the generic coordinate stepper or Pass 1, the engine looks up the
exit coordinate:

```c
cell = MapClass::Get_CellClass(&current_pos)
tube_index = cell->field_0x116    // short, -1 if no tube
if (tube_index == -1) {
    current_pos = (0,0)            // defensive fallback (Pass 1)
} else {
    current_pos = g_TubeArray[tube_index]->field_0x28   // exit coord
}
```

The tube array is at `DAT_008b413c`. Direction 8 is **never smoothed**, but Pass 2 does
**not** perform this TubeArray lookup. `Path_optimize_straight_segments` detects
direction 8, advances using the first `g_DirectionOffsets` entry, zeroes its cumulative
drift state, and restarts the scan window; no `MapClass__Get_CellClass`/`g_TubeArray`
read appears in that branch (corrected 2026-06-01: was "Pass 2 resets from the jump
exit"; binary shows the Pass-2 direction-8 branch at `0x0042B882..0x0042B904` reads
`0x0089F688` and resets counters, via `disassemble_function 0x0042B7F0` - INFERENCE_HARDENED).

### 8.4 Slope validation mechanics — VERIFIED

Both passes call the same slope check but **with different thresholds**.

**Slope cost lookup — `FUN_0056bcd0(&pos, slope_table)` at `0x0056bcd0`:**

```c
cell = g_CellArray[y*0x200 + x]
if (cell == NULL || coord out of range)
    cell = &g_DummyCell    // DAT_00abdc50
// Read cell's map coordinate (field_0x24)
// Index the coarse slope/zone table at 4-cell resolution:
table_y = (cell.MapCoord.y + (cell.MapCoord.y >> 31 & 3)) >> 2   // y/4, signed-floor
table_x = (cell.MapCoord.x + (cell.MapCoord.x >> 31 & 3)) >> 2   // x/4, signed-floor
return *(int *)(slope_table + 0x59F0 + (table_y*0x82 + table_x)*4)
```

`slope_table` = `foot[0x87]` = **`FootClass+0x21C`** (pointer). This points to a
precomputed per-SpeedType zone map. The 0x59F0 offset is into that structure;
the stride `0x82 * 4` = 520 bytes per row means 130 columns at 4-cell
granularity — enough for the maximum 512-cell map width / 4.

**Unit speed multiplier — `FUN_004dc760(foot)` at `0x004dc760`:**

```c
if (foot->field_0x5D4 != 0 &&   // tether/tow link active
    foot->tether_obj->type->field_0xF2 != 0) {
    return 1.0
}
return *(double *)(foot + 0x530)
```

`FootClass+0x530` (double) is the unit's current speed factor relative to its
max. Tethered units with type flag `+0xF2` set override to 1.0.

**Steep-slope threshold (PASSES DIFFER):**

| Pass | Threshold constant | Value  | Location |
|------|-------------------:|-------:|----------|
| 1 (`Path_smooth_single_segment`) | `_g_Const_1_0` | **1.0**  | compiler global |
| 2 (`FUN_0042be20`) | `_DAT_007e3808` | **0.01** | 0x007e3808 (double) |

> Verified via `read_memory 0x007e3808 16` → bytes `7b 14 ae 47 e1 7a 84 3f` = IEEE-754 double `0.01` exactly. (Prior value of `1.01` was wrong — significant: with 0.01, any nonzero slope cost counts toward the 4-steep-cell cap, which changes which paths are rejected on hilly terrain.)

**Steep test:** `(slope_cost * speed_multiplier) >= threshold`.

Pass 2 also has an **enable gate**: `if (speed_mult > _DAT_007e3810)` where
`_DAT_007e3810 = 1e-5`. Essentially slope checks are always on unless the
unit's speed factor is below 1e-5 (effectively zero).

> Verified via `read_memory 0x007e3810 8` → bytes `f1 68 e3 88 b5 f8 e4 3e` = IEEE-754 double `1e-5` exactly. Prior value `6.3e-10` was wrong; both decode to "near-zero" so behavioral impact is small, but bit-exact parity requires `1e-5`.

### 8.5 Pass-2 steep-slope tolerance — EXISTING DOC IS INVERTED

**CORRECTION TO §3.** The existing document states:

> "Allow up to 3 steep slopes normally, 0 if at path end"

The actual behavior, verified from `FUN_0042be20` at `0x0042be20`:

```c
// Decision gate (simplified):
bVar3 = false;   // false = NOT blocked, path step is accepted
if (can_enter_cell_result != 0 || (cell.flags & 0x40000) || steep_count >= 4 ||
    (param_7 == 0 && steep_count >= 1)) {
    bVar3 = true;   // blocked, abort this path ordering
}
```

**`param_7` is the `is_end_of_scan_window` flag**, not "is_path_end":
- `param_7 = 0` → mid-scan reroute (inside the 20-step drift loop) →
  **0 steep cells allowed** (the `&& steep_count >= 1` kicks in)
- `param_7 = 1` → end-of-scan reroute (called once after the 20-step window
  closes) → **up to 3 steep cells allowed** (only `steep_count < 4` enforced)

Caller sites in `Path_optimize_straight_segments @ 0x0042b7f0`:
- Line `0x0042bb7a` area: inside main loop, passes **0**
- Line `0x0042bc50` area: final sweep after loop, passes **1**

**Why backwards from intuition:** mid-window drift corrections are strict because
the A* path has plenty of room to route around steep terrain. The tail-end
correction that finalizes remaining displacement after the 20-step scan is
lenient because it's the last chance to fix accumulated drift before path
delivery; rejecting it would leave the path unoptimized.

### 8.6 Pass-2 cardinal/diagonal decomposition — VERIFIED

`FUN_0042be20` picks the directions by the signs of the target displacement
`(dx, dy) = *param_4`:

**Diagonal direction (`local_28`) — by sign of dx, dy:**

| `dx`         | `dy`         | diag dir (YR encoding) |
|--------------|--------------|:-----------------------:|
| dx < 0       | dy ≥ 0       | **5** (NE)              |
| dx < 0       | dy < 0       | **7** (SE)              |
| dx ≥ 0       | dy ≥ 0       | **3** (NW)              |
| dx ≥ 0       | dy < 0       | **1** (SW)              |

**Cardinal direction (`local_24`) — by sign and magnitude comparison:**
Selects one of 0/2/4/6 (S/W/N/E) based on whether |dx| > |dy| and the signs.

**Step counts:**
- `local_34 = min(|dx|, |dy|)` — diagonal steps (consumed first)
- `local_30 = ||dx| - |dy||` — cardinal steps (consumed second)

**Two orderings tried** via `local_10` counter (0 then 1, exit at `>= 2`):
- Iteration 0: diagonal-first then cardinal
- Iteration 1: swap (`local_28 ↔ local_24`, `local_34 ↔ local_30`): cardinal-first
  then diagonal

If either ordering passes the per-cell validator, the direction entries
`param_1[0 .. local_34+local_30-1]` are overwritten and the excess
`param_1[local_34+local_30 .. param_2-1]` is marked `0xFFFFFFFE`. If both
orderings fail, the function returns 0 and the original zigzag stays.

### 8.7 FUN_0042bca0 split-point finder — VERIFIED

Walks the direction array **backward** from `param_2` (end-of-scan index) to
`param_3` (segment start index), tracking cumulative displacement **using
reversed directions** (`dir XOR 4`, i.e. dir + 4 mod 8):

```c
For each step walking backward:
    skip if dir == -2 (already deleted)
    reversed_dir = (dir - 4) & 7
    accumulate position using reversed_dir's delta
    chebyshev = max(|accum.x|, |accum.y|)
    if chebyshev > peak_chebyshev:
        peak_chebyshev = chebyshev
        if we already saw a stall (bVar1):
            return this position as split anchor,
            and (reversed_dir-4)&7 rotated direction as replacement start
    else:
        bVar1 = true   // record that we hit a non-increasing step
```

The algorithm finds the **peak Chebyshev displacement after the first plateau**,
which corresponds to the "inflection point" where the path started curving back.
That point becomes the split anchor for the replacement straight-line segment.

### 8.8 Pass-2 bridge-level adjustment — VERIFIED (undocumented detail)

After each validated step in `FUN_0042be20`, the code maintains a **running
bridge-level state** (`iVar1` = effective cell level):

```c
cell_level = cell->field_0x11B   // byte, cell base level
if ((current_level - cell_level) == 4 && (cell.flags & 0x100)) {
    current_level = cell_level + 4   // stay on bridge top
} else {
    current_level = cell_level       // drop to ground
}
```

This is passed to `Can_Enter_Cell` as the height argument. It models the
bridge-top overlay: when stepping between two bridge-elevated cells, maintain
the +4-level offset; otherwise fall to ground level. Flag `0x100` on cell
flags = `IsBridge` marker.

### 8.9 Revised Rust Implementation Delta

| Aspect                               | Binary behavior                | Rust impact |
|--------------------------------------|--------------------------------|-------------|
| Pass 2 steep threshold               | 0.01 (not 1.0)                 | Adjust if porting |
| Pass 2 slope-check enable gate       | `speed_mult > 1e-5`            | Effectively always on |
| Mid-loop vs end-loop steep tolerance | **0 mid, 3 end** (inverted from existing doc) | **Rust matches existing doc — verify** |
| Two-ordering retry (diagonal-first, cardinal-first) | `local_10 < 2` loop | Existing doc correct |
| Direction-8 never smoothed           | Pass 1/generic stepper use TubeArray; Pass 2 only resets scan state and does not TubeArray-lookup (corrected 2026-06-01 via `disassemble_function 0x0042B7F0` - INFERENCE_HARDENED) | Rust layer transition blocks — verify equivalence |
| Bridge-top level tracking            | `(curr - cell.level == 4 && IsBridge)` → stay elevated | Rust needs same logic for bridge passability |
| Slope cost = per-SpeedType coarse zone map at 4-cell res | Precomputed pointer at FootClass+0x21C | Rust has per-cell passability, may differ |
| Unit speed factor = FootClass+0x530, tether override via type.+0xF2 | Verified | Not yet modeled |

### Addendum Sources

**Ghidra addresses re-decompiled (2026-04-05):**
- `0x0042b210` `Path_smooth_corners` (full)
- `0x0042b420` `Path_smooth_single_segment` (full)
- `0x0042b7f0` `Path_optimize_straight_segments` (full)
- `0x0042bca0` `FUN_0042bca0` split-point finder (full)
- `0x0042be20` `FUN_0042be20` straight-line rerouter (full)
- `0x0056bcd0` `FUN_0056bcd0` slope cost lookup (full)
- `0x004dc760` `FUN_004dc760` unit speed multiplier (full)
- `0x00429a90` `AStar_main_loop` (call-chain verification)

**Constants read:**
- `0x007e3808`: 0.01 (double) — Pass-2 steep threshold (verified via `read_memory 0x007e3808 16` → bytes `7b 14 ae 47 e1 7a 84 3f`)
- `0x007e3810`: 1e-5 (double) — Pass-2 slope-check enable gate (verified via same read → next 8 bytes `f1 68 e3 88 b5 f8 e4 3e`)
- `0x008b413c`: TubeArray base pointer (direction-8 lookup table)

**Cell flag 0x40000:** "cliff/impassable edge" — set by map init on cells flagged
as cliff boundaries. Used by both smoothing passes as an unconditional blocker.
**Cell flag 0x100:** "IsBridge" — used by the level-tracking logic in Pass 2.
