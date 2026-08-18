# Chrono Miner — Drive-Phase Locomotion Trace

**Scenario:** Empty Allied Chrono Miner spawns at War Factory exit cell. Player
right-clicks an ore patch ~10 cells away (within drive range — close enough that
gamemd does NOT teleport, uses ground driving via piggybacked DriveLocomotionClass).
Trace: click to arrival at ore patch.

**Scope:** Drive phase only. Teleport branch is slot 2 of the swarm — not traced here.

**Date:** 2026-05-19
**Authored by:** trace-swarm slot 1
**Iron Law applied:** PASS requires literal numerical equality. Anything less is FAIL or UNCHECKED.

> **Disputed status 2026-05-25:** This trace's older ore-approach/locomotor
> conclusions conflict with newer ore-acquisition and drive-model docs plus
> current Rust comments. Do not implement an ore-approach warp from this trace
> alone. Required follow-up before changing code: `/re-investigate chrono miner
> ore approach teleport-vs-drive`.

---

## Key INI Values (from ini/rulesmd.ini [CMIN])

| Key | Value | Parsed as |
|-----|-------|-----------|
| Speed | 4 | leptons/tick = 4×256/100 = 10; leptons/sec = 10×15 = 150 |
| ROT | 5 (overridden to 10 by `Harvester=yes`) | 10 facing-units per game frame |
| Locomotor | `{4A582747}` TeleportLocomotionClass | Primary locomotor; Drive piggybacked |
| Teleporter | yes | TechnoTypeClass+0xCD4 = 1 |
| Harvester | yes | Triggers ROT=10 override at INI parse time |
| MovementZone | Crusher | Used by pathfinder |
| Crusher | yes | AutoCrush=yes, Crusher=yes |
| ChronoHarvTooFarDistance | 50 cells | Drive threshold for ore-to-refinery |
| TiberiumLongScan | 48 cells | State 0 scan radius |
| HarvesterLoadRate | 2 | Frames per StepTimer step |

---

## Pipeline Stages

### STAGE 0 — Locomotor State at Spawn (Active = Drive or Teleport?)

**gamemd behavior (verified, CHRONO_MINER_SYSTEM_OVERVIEW.md §2, §3):**
- At spawn, the unit's primary locomotor is TeleportLocomotionClass at FootClass+0x674.
- DriveLocomotionClass is piggybacked inside TeleportLoco at TeleportLoco+0x48.
- Active locomotor at spawn = TeleportLocomotionClass (ready to warp).
- Drive is dormant (piggybacked), NOT the active locomotor.

**Our-side behavior (src/sim/movement/locomotor.rs:LocomotorState::from_object_type):**
- CMIN has `Locomotor={4A582747}` → `LocomotorKind::Teleport`.
- `from_object_type` maps Teleport → `(MovementLayer::Ground, speed_multiplier=1.0)`.
- The `override_state: None` field means no piggyback override is currently active.
- There is no separate DriveLocomotionClass object instantiated at spawn.
- The `override_state` field stores a saved base locomotor ONLY when a teleport is
  active (teleport is the override of Drive), not the other way.
  
**Critical architectural gap:** In gamemd the primary stored locomotor is Teleport
and Drive is piggybacked inside it. In our engine Teleport IS the sole locomotor kind
(`loco.kind = Teleport`) and there is no real Drive locomotor object. The piggyback
swap concept (FootClass::AI calls Is_Ok_To_End each tick, swaps active locomotor)
does not exist in our codebase — there is no equivalent of `FootClass::AI` checking
`Is_Ok_To_End()` and restoring the Drive locomotor. Instead, the `override_state`
mechanism inverts the relationship: Drive is "base", Teleport is "override".

**Verdict: UNCHECKED at spawn** (no literal number to compare; architecture differs —
see Adjacent Findings §A).

---

### STAGE 1 — Ore Scan: SearchOre State (TiberiumLongScan)

**gamemd (CHRONO_MINER_SYSTEM_OVERVIEW.md §4, WAR_MINER_LOCOMOTION_INTEGRATION §4 State 0):**
- Mission_Harvest State 0: scan radius = TiberiumLongScan = 48 cells.
- Chrono-miner-specific: if active loco's CLSID == TeleportLocomotion AND nav_target != null,
  call Set_Destination(NULL, 1) to cancel any in-progress teleport destination before scanning.
- Then calls Search_For_Tiberium_And_Move with LongScan radius.

**Our-side (src/sim/miner/miner_system.rs:handle_search_ore):**
- `config.long_scan_radius` = 48 (read from `general.tiberium_long_scan`, default 48). PASS on radius.
- Ghost-cell archive check runs first (correct — matches "scan TiberiumShortScan for ghost cell"
  on full-storage transition to State 2).
- No equivalent of the Chrono-specific "cancel in-progress teleport destination before scanning"
  branch — the `if active_loco CLSID == TeleportLocomotion AND nav_target != null` cancel call
  from gamemd is absent. This fires when a chrono miner receives a warp destination while already
  in State 0 (edge case); for a fresh spawn it doesn't fire. Low player-visibility impact
  for a fresh miner, but the branch is MISSING.
- Zone-reachability filter: present (build_reachable_filter). Not present in original engine;
  this is an engine enhancement. Does not affect the first-tick scan for a miner with no
  zone data, but could suppress an ore cell the original would have found. UNCHECKED for parity.

**Verdict: UNCHECKED** (scan radius matches, but cancel-on-scan branch missing for chrono edge case)

---

### STAGE 2 — Set_Destination Decision: Drive vs Teleport for Ore Cell

**gamemd (TechnoClass::Set_Destination 0x741970, CHRONO_MINER_SYSTEM_OVERVIEW.md §3):**
- Set_Destination is called with the ore cell.
- Teleporter check: TechnoTypeClass+0xCD4 == 1 → enter teleporter block.
- CellClass::FindFirstBuilding (0x47EBA0) on ore cell.
- Ore cell is an empty field cell — FindFirstBuilding returns NULL.
- NULL result → keep TeleportLocomotionClass active → unit WARPS.
- Wait: **10 cells away is within drive range** — the scenario states we DON'T warp.
  Re-reading §4: "Ore cell is empty → but drive piggyback is created (need to pathfind
  to ore) Unit DRIVES to ore." This contradicts §3's "empty cell → WARPS."
  
  Correct resolution (CHRONO_MINER_SYSTEM_OVERVIEW.md §4 State 0 detail):
  The FindFirstBuilding check determines BUILDING vs EMPTY. For **ore** cells
  (empty terrain with tiberium overlay), FindFirstBuilding returns NULL → stays
  TeleportLoco → normally WARPS. However, gamemd's chrono miner DOES drive to ore
  patches — this is because: the chrono miner drives to ore first, warping is only
  triggered when the miner actively wants to move via a non-NULL IsMoving flag.
  
  Actual gamemd behavior for ore patch at 10 cells:
  - Set_Destination with ore cell → FindFirstBuilding = NULL → TeleportLoco stays active.
  - FootClass::Assign_Destination → TeleportLoco::Head_To_Coord (0x718100).
  - Head_To_Coord validates destination, sets IsMoving=1 → triggers warp Phase 0 on next tick.
  - **Conclusion:** The chrono miner WARPS to ore cells, it does NOT drive to them
    using the drive locomotor in the ore-seek phase.
  - The drive locomotor is only active when the miner is approaching a BUILDING
    (refinery dock) — FindFirstBuilding finds the building → creates DriveLocomotionClass
    + piggyback Teleport under it.
  
  **So the scenario premise "within drive range — close enough that gamemd does NOT
  teleport, but uses ground driving" is INCORRECT for the ore-seek phase.** The miner
  ALWAYS warps to ore. Drive phase only occurs for building approach (dock).

**Actual drive-phase trigger in gamemd:** Drive is active ONLY when:
- Set_Destination is called with a BUILDING-containing cell (FindFirstBuilding != NULL), OR
- FootClass::AI swaps locomotor after a warp completes (Is_Ok_To_End → End_Piggyback).

For a 10-cell ore approach, gamemd warps. For dock approach, gamemd drives.

**Our-side (src/sim/miner/miner_system.rs:handle_move_to_ore):**
- Arrival check: if (snap.rx, snap.ry) == target → state = Harvest. No issue.
- Adjacent (dx ≤ 1, dy ≤ 1): `issue_direct_move` to ore cell if not already moving.
- Otherwise: `issue_move_command` via A* to ore cell (regular drive pathfinding).
- NO teleport is issued for ore approach. The miner always drives using MovementTarget/A*.
- This is a FUNDAMENTAL divergence: gamemd warps to ore; we drive to ore.

**Verdict for ore approach: FAIL**
- gamemd: TeleportLocomotionClass::Head_To_Coord → IsMoving=1 → warp Phase 0 → unit teleports
  instantly to ore cell (or within 1 cell if piggybacking drive for building approach).
- ours: A* pathfinding → MovementTarget → ground drive (10 cells, ~1700ms at Speed=4).
- Player sees: unit drives slowly to ore instead of warping. Timing is dramatically different
  (10 cells × 256 leptons ÷ 150 lep/sec ≈ 17 seconds driven vs ~1 tick warped).
- Source: `src/sim/miner/miner_system.rs:362-392`

---

### STAGE 3 — Move Command Dispatch (Drive Path)

*This stage traces our engine's drive path as actually implemented (even though gamemd warps).*

**Our-side trigger (miner_system.rs handle_move_to_ore):**
- `issue_move_command` → PathGrid A* from current cell to ore cell.
- Speed set to `ra2_speed_to_leptons_per_second(4)` = 150 leptons/sec.
- MovementTarget attached with: path (A* steps), speed=150 lep/sec, accel/decel factors.

**gamemd drive path (only for building approach, NOT ore):**
- DriveLocomotionClass::Set_Destination (0x4AFD40) stores coord at loco+0x34.
- Process reads loco+0x34, computes next waypoint, writes to head_to at loco+0x40.
- Path computed by FootClass::Find_Path (A* with MovementZone=Crusher).

**gamemd accel/decel (PROCESS_DRIVE_TRACK_DECOMPILATION.md §Phase 2):**
- decel_threshold at TechnoTypeClass+0x2F8 (verified).
- Speed target = techno->max_speed (from TypeClass+0x15E as word-index).
- Decel formula: `if distance < decel_threshold: target_speed = distance * max_speed / decel_threshold`.
- Accel: `current_speed += accel_step` per-track-step.

**Our-side accel/decel (movement_tick.rs:537-586):**
- Uses `target.accel_factor` and `target.decel_factor` fields on MovementTarget.
- Slowdown triggers at `dist < target.slowdown_distance` (our: Euclidean 2D, gamemd: 3D Pythagorean).
- Decel floor at 30% of max speed (`MIN_BRAKE_FRACTION`). gamemd uses a linear slope to 0.
- 3D vs 2D distance: we use Euclidean 2D (`distance_to_goal_leptons`), gamemd uses
  3D `Sqrt_Approx(dx²+dy²+dz²)`. On flat terrain dz=0 so results are identical.

**Verdict for drive dispatch: UNCHECKED**
- We don't have verified numerical equality between our accel_factor/decel_factor and
  gamemd's equivalent computed constants. The formulas match conceptually but the
  per-tick step sizes haven't been compared with literal numbers.

---

### STAGE 4 — Speed per Tick

**gamemd (PROCESS_DRIVE_TRACK_DECOMPILATION.md §2 + WAR_MINER_LOCOMOTION_INTEGRATION §7):**
- Speed=4: `leptons_per_tick = 4 * 256 / 100 = 10` (integer division, truncate toward zero).
- At 15 fps logical rate: `10` leptons per game frame (one sim tick = one frame).
- Maximum: 10 leptons per tick at flat terrain, no slope.

**Our-side (fixed_math.rs:ra2_speed_to_leptons_per_second):**
```
capped = 4 (≤100)
leptons_per_tick = 4 * 256 / 100 = 10 (Rust integer division = truncate, same as C)
leptons_per_second = 10 * 15 = 150
```
- Our engine uses leptons/second; per-tick advancement = `speed * dt` where
  `dt = tick_ms / 1000.0` (SimFixed). At 66.67ms tick (15fps): `150 * 0.06667 ≈ 10` leptons.
- Formula is **identical to gamemd**. Test `test_lepton_speed_harvester` verifies: 150 lep/sec.

**Verdict: PASS** — 10 leptons/tick at Speed=4, 15fps, integer truncation matches.

---

### STAGE 5 — Facing Rotation Before Movement (ROT)

**gamemd (WAR_MINER_LOCOMOTION_INTEGRATION §7, object_type.rs:861-864):**
- `Harvester=yes` → ROT forced to 10 at INI parse time (UnitTypeClass::ReadINI, 0x747620,
  writes 10 to TypeClass+0x398 after standard ReadInt).
- CMIN has `ROT=5` in INI but effective ROT = 10.
- Drive locomotor: unit rotates in place before moving. Rate = 10 facing-units per game frame.
- 1 full rotation = 256 facing units. ROT=10 → full circle in 25.6 ticks = ~1.7 seconds.
- Per-tick turn: 10 facing units.

**Our-side (rules/object_type.rs:857-864):**
```rust
turret_rot: if section.get_bool("Harvester").unwrap_or(false) { 10 } else { section.get_i32("ROT")... }
```
- ROT=10 IS applied at parse time for Harvester=yes. PASS on value.

**Our-side rotation tick (movement_step.rs:handle_vehicle_rotation):**
- `rot_to_facing_delta(rot=10, tick_ms)` → facing delta per tick.
- Rotation happens BEFORE lepton advancement: if `facing_target` is set and not reached, 
  skip lepton advancement this tick (movement_tick.rs:495-511). Matches gamemd's drive
  locomotor behavior: rotate first, then move.
- **`rot_to_facing_delta` formula:** need to verify the exact conversion from gamemd.
  
**gamemd rotation formula (drive track, verified from DRIVE_TRACK_SYSTEM.md):**
- facing-change is computed by `Transform_Track_Coords` which applies turn transforms
  per track-step per tick. The ROT value gates how many track-steps are consumed per tick.
- The turn-track system handles rotation implicitly: a vehicle in a turn-track curve
  rotates gradually. The in-place rotation (pre-movement) is only done in `Process_Movement`
  when no track is assigned and the vehicle needs to face toward the next cell.
- Exact per-tick facing delta for in-place rotation was NOT verified numerically from
  binary; `rot_to_facing_delta` implementation not cross-checked against gamemd.

**Verdict: UNCHECKED** (ROT=10 value correct, but per-tick facing delta formula unverified vs gamemd)

---

### STAGE 6 — Drive Track Curves (Turn Smoothness)

**gamemd (DRIVE_TRACK_SYSTEM.md):**
- When direction changes between cells, a TurnTrack entry (72-entry table at 0x7e7b28)
  selects a RawTrack curve (16 tracks at 0x7e7a28).
- Unit follows the curve's pre-computed TrackPoint array — smooth arc through the cell.
- No stop-rotate-go: facing changes gradually along the curve.
- Call chain: `DriveLocomotionClass::Process → if track_index != -1 → Process_Drive_Track`.
- TRACK_STEP_COST = 7 per step. Budget = `current_speed × scale_factor` per tick.

**Our-side (drive_track.rs, movement_step.rs:configure_motion_after_transition):**
- `select_drive_track(from_facing, to_facing, false)` → TurnTrack lookup.
- `begin_drive_track(raw_track_index, flags, dx, dy, target_facing)` → DriveTrackState.
- In `movement_tick.rs`: `advance_lepton_position` returns `DriveTrackActive` / `DriveTrackCellJump`
  / `DriveTrackChainReady` based on track progress.
- Track data extracted from the original engine binary (drive_track.rs doc comment).

**Parity concern — track advancement budget:**
- gamemd uses TRACK_STEP_COST = 7 per step; budget = `leptons_per_tick / something`.
- Our engine advances track by `speed * dt` in lepton-space each tick.
- The budget system in gamemd accumulates fractional ticks via `residual_ticks` (loco+0x4C).
- Our residual handling: `DriveTrackState` has a lepton-progress model, different from
  gamemd's step-budget + residual. Not numerically verified.

**Verdict: UNCHECKED** (track table data extracted from binary is correct; per-tick budget
math not verified against gamemd's Process_Drive_Track SPEED_COMPUTE phase)

---

### STAGE 7 — Pathfinding (A* with MovementZone=Crusher)

**gamemd:**
- FootClass::Find_Path uses MovementZone=Crusher: can drive over tiberium cells.
- Path limited to 24 steps (path_queue[24] at FootClass+0x5E0).
- On exhaustion: auto-repath picks next 24-step segment.
- Re-sync each tick: Process re-reads nav_target, calls Set_Destination again if not on track
  (WAR_MINER_LOCOMOTION_INTEGRATION §3.4 "re-sync head_to from nav target").

**Our-side (movement_commands.rs:issue_move_command, movement_tick.rs:handle_path_exhaustion):**
- A* pathfinding with MovementZone=Crusher (verified via `snap.movement_zone`).
- Path stored in `MovementTarget.path` as a Vec of cells (not limited to 24 — this is an
  engine scale enhancement, acceptable per CLAUDE.md scale exceptions).
- Segment repath on exhaustion: `handle_path_exhaustion` calls `find_move_path` for next segment.
- No per-tick re-sync from nav_target: once the path is set, it's followed. Missing the
  gamemd behavior where Process re-reads the nav_target's current coord each tick. For static
  ore cells this produces identical output; for a moving target (not relevant here) it would differ.
- `ignore_terrain_cost = true` set on the movement after issue — prevents blocking at ore cells
  along the path. Not in gamemd (harvesters drive normally through ore fields via MovementZone=Crusher).

**Verdict: UNCHECKED** (functional parity for static ore target; minor behavioral difference on
path re-sync and terrain cost exemption flag not verified to produce identical output)

---

### STAGE 8 — Cell Boundary Crossing + Occupancy Update

**gamemd:**
- Process_Drive_Track §6: when track crosses cell boundary (jump_index reached),
  calls Can_Enter_Cell, updates occupancy, transfers unit to new cell.
- Occupancy marks unit at new cell, removes from old cell atomically.
- Path stuck counter: increments on Can_Enter_Cell failure; 10 consecutive → abandon path.

**Our-side:**
- `process_cell_crossings` → `handle_deferred_occupancy` for blocked cells.
- `occupancy.move_entity(old, new, id, layer, sub_cell, ...)` — atomically updates.
- `path_stuck_counter` (MovementTarget): initialized to `PATH_STUCK_INIT`, escalates on block.
- Not verified: gamemd's exact 10-failure threshold vs our implementation's threshold.

**Verdict: UNCHECKED** (mechanisms present; threshold not numerically verified)

---

### STAGE 9 — Arrival Detection

**gamemd (WAR_MINER_LOCOMOTION_INTEGRATION §3.2):**
- For a non-building nav target: `if *(offset_AC) == 5 AND destination != NULL AND
  owner.Location == destination → clear destination, auto-arrive`.
- Ore cell has RTTI != 0xB (not a building), so the building-arrival branch doesn't fire.
- The generic arrival check: `owner.Location == this.destination`.

**Our-side (miner_system.rs handle_move_to_ore:350-356):**
- `if (snap.rx, snap.ry) == target { state = MinerState::Harvest; }` — checked each miner tick.
- Adjacent final step: `issue_direct_move` when dx≤1, dy≤1 and no movement active.
- Movement finalization: `finalize_finished_entities` sets `movement_target = None` when
  `next_index >= path.len()` AND `at_final_goal`.
- Missing the `*(offset_AC) == 5` gate — but this appears to be a mission-state guard
  that equals Mission_Guard (5). Our engine doesn't have the equivalent mission state field,
  but the observable result (arrival detection) is achieved by position check.

**Verdict: UNCHECKED** (arrival triggers; exact tick of detection not compared to gamemd)

---

### STAGE 10 — Locomotor Phase After Arrival (Swap Back to TeleportLoco)

**gamemd (CHRONO_MINER_SYSTEM_OVERVIEW.md §2 Locomotor Swap Lifecycle):**
- After drive phase completes, FootClass::AI (0x4DA530) checks Is_Ok_To_End each tick.
- Is_Ok_To_End (0x719F30): true when: not moving, has piggybacked loco, field_35==0,
  ChronoInTransit==0, WarpPhase==0, field_6AD==0.
- When true: End_Piggyback() retrieves saved locomotor → swap active back to TeleportLoco.
- The unit returns to "idle with TeleportLoco active, ready for next warp."

**Our-side (teleport_movement.rs:tick_teleport_movement):**
- When `TeleportState` is finished (being_warped_ticks=0): `end_override()` is called
  (locomotor.rs:end_override), restoring `saved` base locomotor state.
- But our "drive phase" doesn't use TeleportState — it uses MovementTarget (standard ground drive).
- After MovementTarget is consumed (path exhausted): `finalize_finished_entities` → `loco.phase = Idle`.
- There is NO swap-back logic. Once MovementTarget finishes, loco.kind stays at `Drive` (if that
  was set) or `Teleport` (if override was never used). For a unit that has `loco.kind = Teleport`
  and no override active, the "drive" happens through the normal A* pipeline which doesn't check
  loco.kind at all for ground movement. So functionally it "drives" but the internal state
  doesn't reflect the gamemd Active=Drive ↔ Active=Teleport swap.
- Is_Ok_To_End logic: **not implemented**. The 6-condition gate doesn't exist.

**Verdict: NOT-IMPLEMENTED** (Is_Ok_To_End + locomotor swap-back not present; observable impact
is that the Chrono Miner's readiness state for the NEXT warp after driving may not be correct)

---

### STAGE 11 — Animation (Foot vs Tracks During Drive)

**gamemd:**
- SHP sequences for CMIN: the unit has a CMIN.SHP with multiple sequence groups.
- While driving (DriveLocomotionClass active), the body rotation + driving animation plays.
- No foot animation — CMIN is a tracked vehicle (uses CMIN.VXL).
- Orientation sequence updated each tick based on current facing (0..31 voxel frames).

**Our-side:**
- VoxelAnimation component drives the voxel body rendering.
- `voxel_animation.playing = is_harvesting` (Phase 4 of tick_miners). Drive phase has no
  special animation override. The driving animation comes from the facing direction applied
  to the VXL render.
- UNCHECKED: exact frame selection for facing not compared to gamemd's facing-to-sequence mapping.

**Verdict: UNCHECKED** (structure present; exact facing-to-frame mapping not verified)

---

## Disparity Summary Table

| Stage | Description | Verdict | Player Impact |
|-------|-------------|---------|---------------|
| S0 | Locomotor state at spawn (Active=Teleport vs Drive) | UNCHECKED | Low (architecture differs, output TBD) |
| S1 | Ore scan, radius=48 | UNCHECKED | Low (radius correct; cancel-on-scan branch missing) |
| S2 | **Set_Destination → warp vs drive for ore cell** | **FAIL** | **HIGH** — miner drives to ore, gamemd warps |
| S3 | Drive command dispatch (accel/decel formula) | UNCHECKED | Medium |
| S4 | Speed per tick: 10 leptons/tick at Speed=4 | **PASS** | — |
| S5 | ROT=10 override applied at parse time | UNCHECKED (value PASS, formula unverified) | Medium |
| S6 | Drive track curves | UNCHECKED | Medium (visual smoothness) |
| S7 | A* pathfinding with MovementZone=Crusher | UNCHECKED | Low (functional) |
| S8 | Cell crossing + occupancy | UNCHECKED | Low |
| S9 | Arrival detection | UNCHECKED | Low |
| S10 | **Locomotor swap-back (Is_Ok_To_End)** | **NOT-IMPLEMENTED** | Medium (next-warp readiness) |
| S11 | Drive animation | UNCHECKED | Low |

**Tally: PASS: 1 | FAIL: 1 | UNCHECKED: 9 | NOT-IMPLEMENTED: 1**

---

## Top 5 Player-Visible Failures

1. **S2 — Ore approach: drive instead of warp**
   - Player sees: Chrono Miner drives slowly to ore field (17 seconds) instead of
     teleporting instantly (1 tick). Massive timing difference. No WarpAway anim at
     departure/arrival. Missing 50% translucency at ore cell during chrono delay.
   - Code: `src/sim/miner/miner_system.rs:362-392` (`handle_move_to_ore`)
   - gamemd: TechnoClass::Set_Destination 0x741970 → TeleportLoco::Head_To_Coord 0x718100
     → IsMoving=1 → Phase 0 warp on next tick (CHRONO_MINER_SYSTEM_OVERVIEW.md §3, §5).

2. **S10 — Missing locomotor swap-back (Is_Ok_To_End)**
   - Player sees: after any drive sequence completes, the Chrono Miner's internal locomotor
     state is wrong. The next warp may not trigger correctly or may fire when it should
     drive. Fires on every dock approach (very common, every harvest cycle).
   - Code: no implementation of `Is_Ok_To_End` or `End_Piggyback` swap in any file.
   - gamemd: FootClass::AI 0x4DA530 → Is_Ok_To_End 0x719F30 → End_Piggyback 0x719EE0,
     every tick (CHRONO_MINER_SYSTEM_OVERVIEW.md §2).

3. **S2 (secondary) — Missing WarpAway animation on ore approach**
   - Player sees: no shimmering warp effect at either departure or arrival when miner
     moves to ore. The WarpAway (WARPAWAY) anim should spawn at both cells.
   - Code: `src/sim/miner/miner_system.rs:handle_move_to_ore` — no anim spawn.
   - gamemd: Phase 0 spawns `AnimClass(Rules+0x33C, departure, ...)` and
     `AnimClass(Rules+0x340, arrival, ...)` (CHRONO_MINER_SYSTEM_OVERVIEW.md §5 Phase 0 steps 3,15).

4. **S2 (tertiary) — Missing 50% translucency at arrival**
   - Player sees: miner appears fully opaque at ore cell immediately. Should be 50%
     translucent for ChronoDelay ticks after warp-in (chrono lock visual).
   - Code: `src/sim/movement/teleport_movement.rs:issue_teleport_command` IS called for
     refinery return (far case), but NOT for ore approach (handle_move_to_ore drives instead).
   - gamemd: TechnoClass::Draw 0x706640 adds flag 0x2004 when BeingWarped(+0x271)=1
     (CHRONO_MINER_SYSTEM_OVERVIEW.md §6).

5. **S5 — ROT per-tick formula unverified**
   - Player sees: body rotation speed during dock approach and exit may not match original.
     If our `rot_to_facing_delta(10, tick_ms)` formula differs from gamemd's, the miner
     rotates too fast or too slow before each move step. Visible in every drive segment.
   - Code: `src/sim/movement/movement_step.rs:handle_vehicle_rotation` → `rot_to_facing_delta`.
   - gamemd: DriveLocomotionClass::Update_Facing_From_Type 0x4B04D0, reads ROT=10, applies
     per-track-step (WAR_MINER_LOCOMOTION_INTEGRATION §7).

---

## Adjacent Findings

**A. Chrono Miner always warps to ore — critical architecture finding:**
The entire premise of this swarm slot ("drive-phase to ore") is that gamemd sometimes drives
to ore. Based on CHRONO_MINER_SYSTEM_OVERVIEW.md §3 and §4, this is wrong. The miner
teleports to ore (FindFirstBuilding = NULL → TeleportLoco stays active → warp). The Drive
locomotor is only activated for BUILDING approach (FindFirstBuilding != NULL). The "drive
phase" in the chrono miner only occurs during the final dock approach to the refinery.
Recommend the swarm investigate whether there is a close-range drive-to-ore case at all.

**B. `too_far_threshold_chrono` hardcoded default (10 cells) differs from INI (50 cells):**
`MinerConfig::default()` has `too_far_threshold_chrono: 10` but the INI value is
`ChronoHarvTooFarDistance=50`. When `MinerConfig::from_general_rules` is called it reads
the INI correctly (50). If the game ever uses `MinerConfig::default()` without calling
`from_general_rules` (e.g., tests, fallback paths), the wrong threshold fires — miner
drives when it should warp at 11-50 cells. The test comment at miner_tests.rs:391 says
"Must be > ChronoHarvTooFarDistance (50 cells)" confirming the correct value is known.

**C. `issue_teleport_command` skips chrono delay for harvesters:**
`is_harvester=true` forces `being_warped_ticks=0` — the warp resolves in a single tick
with no translucency phase. This is inconsistent with gamemd where the chrono miner does
show brief translucency at the destination during the lock timer. The lock timer for
self-teleport at 10 cells = 10×256/48 = ~53 leptons/48 ≈ 0; minimum delay = 16 ticks.
So even at close range, gamemd shows 16 ticks of translucency. By forcing 0 our miner
is immediately opaque. (Chrono delay formula: CHRONO_MINER_SYSTEM_OVERVIEW.md §5 Phase 0
steps 4-5: distance=10 cells=2560 leptons; delay=2560/48=53; clamp to max(16,53)=53 ticks.)

**D. Tiberium spill animation (Blue Tiberium overlay) not implemented:**
WAR_MINER_LOCOMOTION_INTEGRATION §3.5: spawn TiberiumSpill anim every 10 frames when
driving over Blue Tiberium overlay. No equivalent in our engine. Minor visual gap.

---

## Sources

- `CHRONO_MINER_SYSTEM_OVERVIEW.md` — locomotor architecture, Set_Destination decision,
  warp sequence, TechnoClass offsets (verified from gamemd).
- `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md` — DriveLocomotionClass::Process,
  Set_Destination_Internal, footclass offsets, ROT override, speed/ROT application.
- `PROCESS_DRIVE_TRACK_DECOMPILATION.md` — Process_Drive_Track speed compute, budget system.
- `DRIVE_TRACK_SYSTEM.md` — TurnTrack/RawTrack tables, call chain.
- `ILOCOMOTION_COM_PROTOCOL_SPEC.md` — vtable addresses, IPiggyback support matrix.
- `ini/rulesmd.ini [CMIN]` — Speed=4, ROT=5 (effective 10), Storage=20, Teleporter=yes.
- `src/sim/miner/miner_system.rs` — ore approach, movement dispatch, drive vs teleport.
- `src/sim/movement/teleport_movement.rs` — issue_teleport_command, TeleportState lifecycle.
- `src/sim/movement/locomotor.rs` — LocomotorState, OverrideKind, begin/end_override.
- `src/sim/movement/movement_tick.rs` — speed ramp, drive track, cell crossings.
- `src/sim/movement/movement_step.rs` — handle_vehicle_rotation, configure_motion_after_transition.
- `src/util/fixed_math.rs` — ra2_speed_to_leptons_per_second (Speed=4 → 150 lep/sec, tested).
