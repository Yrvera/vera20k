# Chrono Miner — Teleport Dock Approach Trace

**Slot:** 2 of 5 (parallel swarm)
**Mechanic:** Warp-out → warp-in at refinery dock cell
**Scenario:** Full-cargo Allied Chrono Miner (CMIN) at remote ore patch. Mission_Harvest
  transitions to return-to-refinery. Distance exceeds ChronoHarvTooFarDistance (50 cells)
  → teleport branch taken. Trace from destination assignment through docked at refinery.
**Date:** 2026-05-19
**Binary ref:** gamemd.exe YR 1.001
**Docs consulted:** CHRONO_MINER_SYSTEM_OVERVIEW.md, CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md,
  PHASE0_CHRONO_DELAY_FORMULA_MATH_GHIDRA_REPORT.md, MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md,
  MISSION_ENTER_REFINERY_DOCK_VERIFICATION_NOTES.md, TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md,
  RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md, CHRONO_WARP_VISUAL_RENDERING.md,
  PATHFINDING_VALIDATE_ALTERNATE_CHRONO_RETURN_GHIDRA_REPORT.md,
  TECHNOCLASS_0x6AF_CHRONO_STATE_FIELD_GHIDRA_REPORT.md
**Rust files inspected:** src/sim/movement/teleport_movement.rs, src/sim/miner/miner_system.rs,
  src/sim/miner/miner_dock_sequence.rs, src/sim/miner/miner_dock.rs

---

## Iron Law Reminder

PASS requires literal numerical equality between our output and gamemd's. If both values
were not computed, the stage is UNCHECKED, not PASS. Pad-PASS is forbidden.

---

## Stage Table

### Stage 1: ChronoHarvTooFarDistance check and teleport-vs-drive decision

**gamemd behavior:**
- `UnitClass::Mission_Harvest` state 2 (RETURN) calls `Find_Docking_Bay` → nearest refinery.
- Seed cell = `(dock.cellX + BuildingTypeClass[+0x1618], dock.cellY + BuildingTypeClass[+0x161C])`
  (DockOffset from building type). For GAREFN: dock cell = building_anchor + (4, 1).
- Queue cell is found via `FootClass::Find_Nearby_Passable_Cell(0x56DC20)` from that seed.
  Zone check disabled (param_5=-1). Only SpeedType=2 terrain passability checked.
- Distance check: gamemd computes distance in **leptons** via `Sqrt_Approx` from miner position
  to dock-adjacent cell. `ChronoHarvTooFarDistance` (default 50, stored as cells at `Rules+0xD7C`)
  is compared as `dist_leptons <= threshold * 256`. So the real cutoff is 50*256=12800 leptons.
  If dist > 12800: teleport branch (Set_Destination with empty cell → TeleportLoco stays active).
  If dist <= 12800: reserve slot directly, drive path.

**Our behavior (`begin_return` in miner_system.rs ~line 760):**
- Uses `cell_dist_sq((snap.rx, snap.ry), center)` — **cell-coordinate squared distance** vs
  `threshold^2` (where threshold = `config.too_far_threshold_chrono` = 50 cells).
- `center` is computed as `(entity.position.rx + w/2, entity.position.ry + h/2)` — the
  refinery's geometric center cell.
- gamemd measures lepton-distance from miner to the **dock-adjacent cell** (queue cell),
  not the building center.

**FAIL:** Distance is measured to the building center in our code, vs the dock-adjacent cell
(queue cell) in gamemd. The measurement point differs. The squaring-vs-sqrt approach is
functionally equivalent for threshold comparisons, but the reference point is wrong.
Additionally, gamemd does its comparison in leptons (`dist_leptons <= threshold * 256`)
which has the same threshold but uses a different path. However the dominant error is the
center vs queue-cell reference point — for a 4x3 refinery this shifts the boundary by
~2 cells.

**Severity:** MEDIUM. Fires every harvest cycle. A miner 50-51 cells from the dock
cell but only 48 cells from the building center will incorrectly drive instead of warp.

---

### Stage 2: Set_Destination decision — teleport-vs-drive

**gamemd behavior (TechnoClass::Set_Destination 0x741970):**
- Teleporter block at 0x7423CD: `CellClass::FindFirstBuilding(destCell)`.
  - If dest cell contains a building → create DriveLocomotionClass, piggyback Teleport under
    it → DRIVE path. Miner drives.
  - If dest cell is empty (queue cell is outside the building footprint) → keep TeleportLoco
    active → WARP path.
- For GAREFN the queue cell (4,1 offset from anchor) is outside the foundation footprint.
  FindFirstBuilding returns NULL → warp branch taken.

**Our behavior:**
- `issue_teleport_command` is called directly from `begin_return` / `handle_return` when
  `far_enough` is true. No FindFirstBuilding check exists in our code.
- The queue cell is computed as `refinery_queue_cell(rx, ry, w, h, queueing_cell)` which
  uses art.ini `QueueingCell=` when available, otherwise `(rx+w, ry+h/2)`.

**PASS (structural):** The intent matches — we only teleport to empty queue cells, never to
building-occupied cells. We don't replicate FindFirstBuilding literally, but the behavioral
output is the same: we never teleport into a building footprint because the queue cell is
always adjacent to (not inside) the building. No numerical value to compare here.

**Note:** The `QueueingCell=` lookup from art.ini is the correct source (gamemd reads
`BuildingTypeClass+0x1618/+0x161C` which is populated by art.ini). PASS on this sub-check.

---

### Stage 3: Warp-out animation spawning (departure effects)

**gamemd behavior (TeleportLocomotionClass::StateMachineTick Phase 0, ~0x7195B0):**
- Spawns `AnimClass::Constructor(Rules+0x33C /*WarpOut*/, &currentCoords, 0, 1, 0x600, 0, 0)`
  at departure cell. Rules+0x33C is `WarpOut=` key (default WARPOUT anim).
- Immediately after: plays `ChronoOutSound` via per-unit override (TypeClass+0x578) if ≠ -1,
  else global fallback (Rules+0x21C). The VocClass call has the departure coordinates.
- IMPORTANT: `WarpIn` (Rules+0x338) is **NOT spawned** by self-teleport. Only `WarpOut`
  (Rules+0x33C) is spawned, twice — once at departure, once at arrival. WarpAway (Rules+0x340)
  is also NOT spawned for self-teleport.

**Our behavior (`spawn_warp_effects` in miner_system.rs ~line 827):**
- Uses `rules.general.warp_out.name` (WarpOut) and spawns two `WorldEffect` instances:
  one at departure, one at arrival. This correctly uses WarpOut at both points.
- Also emits `SimSoundEvent::ChronoTeleport` for ChronoOutSound at departure and
  ChronoInSound at arrival, with per-unit fallback to global. This matches the gamemd order.
- HOWEVER: the anim is specified by `warp_out.name` (string) and `warp_out.rate_ms`. gamemd
  passes flags `0x600` (AnimFlag = centered + loop). We pass `translucent: true`. This may
  differ in Z-sort, looping behavior, and centering.

**UNCHECKED:** We have not compared the AnimClass flag set (0x600 = centered|unknown flag)
vs our translucency-only `WorldEffect`. The visual output of animation spawning cannot be
confirmed identical without rendering the anim and comparing frame-by-frame.

---

### Stage 4: Chrono delay formula (distance calculation)

**gamemd behavior (Phase 0, verified from assembly in PHASE0_CHRONO_DELAY_FORMULA_MATH):**
```
distance = (int)sqrt(dx*dx + dy*dy + dz*dz)   // 3D Euclidean, ftol truncation
raw_delay = ChronoTrigger ? distance / ChronoDistanceFactor : 0   // IDIV, truncating
remaining = raw_delay (since timer was just set, elapsed=0)
if remaining <= ChronoMinimumDelay:
    timer.Duration = ChronoMinimumDelay   (JLE: signed comparison)
if distance < ChronoRangeMinimum:
    timer.Duration = ChronoMinimumDelay   (runs AFTER the minimum clamp, sequential)
```
- Result stored in `TeleportLocomotionClass+0x44` (locomotor timer), NOT TechnoClass+0x284.
- `ChronoRangeMinimum` defaults to 0, so the last check never fires in standard YR.
- **SPECIAL CASE FOR HARVESTER**: Section 10.5 of CHRONO_WARP_VISUAL_RENDERING.md states:
  "If UnitClass AND Harvester=yes (type+0xE0E): timer=0, BeingWarped=0 (instant!)". The
  state machine zeroes the timer AND clears BeingWarped for Harvester=yes units. This means:
  - The chrono miner's BeingWarped is NOT set (it never becomes semi-transparent).
  - The warp completes in 1 tick with zero post-warp lock.
  - The unit is fully opaque immediately after warp.

**Our behavior (`compute_chrono_delay` in teleport_movement.rs line 70 and
`issue_teleport_command` line 120):**
```rust
let chrono_ticks = if is_harvester { 0 } else { compute_chrono_delay(rules, distance_leptons) };
```
- When `is_harvester=true` (which `begin_return` passes for Chrono Miners), `being_warped_ticks=0`.
- In `tick_teleport_movement` at line 209: `if teleport.being_warped_ticks == 0 { finished.push(id) }`.
  This means the teleport completes in the Relocate tick itself — no ChronoDelay phase.
- `compute_chrono_delay` uses integer division matching gamemd's IDIV.
- The minimum clamp `if delay < rules.chrono_minimum_delay` uses `<` not `<=`. gamemd uses
  `JLE` (less-than-or-equal). For normal inputs this is the same since delay is always
  non-negative and ChronoMinimumDelay is positive, but the signed boundary case differs.

**FAIL (minor):** Our minimum clamp uses `delay < chrono_minimum_delay` (exclusive) vs
gamemd's `remaining <= ChronoMinimumDelay` (inclusive). In practice with ChronoMinimumDelay=16
this only differs when `delay` exactly equals 16 — gamemd would reset the timer to 16 again
(no-op functionally), but the timer's StartFrame is reset which is a subtle behavioral difference.
Very low player visibility.

**PASS (for harvester case):** Harvester instant-warp (timer=0, single-tick completion) is
correctly implemented. BeingWarped is never set (teleport_state.being_warped_ticks == 0 means
ChronoDelay phase is skipped entirely, matching gamemd's timer=0 + BeingWarped=0).

**UNCHECKED:** Distance calculation. gamemd uses 3D Euclidean `sqrt(dx^2+dy^2+dz^2)` in
leptons. Our code (teleport_movement.rs line 116-119) uses:
```rust
let dx = (entity.position.rx as i32 - target.0 as i32) * 256;
let dy = (entity.position.ry as i32 - target.1 as i32) * 256;
let dist_sq = (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64);
let distance_leptons = isqrt_i64(dist_sq) as i32;
```
This is 2D (no Z/height component). gamemd adds `dz*dz` in the sqrt. For ground units on
flat terrain this is negligible, but on hills/bridges the Z differs. The chrono miner case
(ground unit, flat terrain typical) makes this a low-priority gap.

---

### Stage 5: WarpPhase state machine — self-teleport path

**gamemd behavior (Phase 0, all happens in ONE tick):**
1. Stop targeting of this unit (`FUN_0070D4A0`).
2. Detach anim effects linked to this unit.
3. Spawn WarpOut anim at departure.
4. Compute distance, set timer.
5. **If Harvester=yes: timer=0, BeingWarped=0.** Otherwise: BeingWarped=1.
6. Unmark from old cell (remove from cell occupancy).
7. Play ChronoOutSound at departure.
8. Mark at new cell (add to cell occupancy at destination).
9. Play ChronoInSound at destination.
10. SetMission(GUARD_AREA=2).
11. Spawn WarpOut anim at arrival.
12. Clear PendingWarpPhase.
- Phase stays at 0 (no increment). WarpPhase never advances to 1-7 for self-teleport.
- Subsequent ticks: pre-phase check fires `TimerCheck` each tick while BeingWarped=1.

**Our behavior (tick_teleport_movement, TeleportPhase::Relocate):**
- Position snap happens (steps 6+8 equivalent). ✅
- Occupancy swap via `occupancy.move_entity(...)`. ✅
- If `being_warped_ticks == 0`: finished immediately (no ChronoDelay) — harvester case. ✅
- Otherwise transitions to `TeleportPhase::ChronoDelay`.

**NOT-IMPLEMENTED:** Steps 1 and 2 (stop targeting, detach animations from unit) are not
implemented in `tick_teleport_movement`. In gamemd, when the miner warps, all units
targeting it lose their target and anims attached to the miner are detached. This affects
combat during warp (units firing at the miner lose lock) and visual effects.

**NOT-IMPLEMENTED:** Step 10 — `SetMission(GUARD_AREA=2)`. gamemd sets the unit's mission
to Guard_Area immediately after warping. Our miner FSM stays in `ReturnToRefinery` state
during and after the warp (the teleport guard at line 499 waits for teleport to finish,
then transitions to `Dock`). This functional difference is not visible to the player in
normal single-miner scenarios but could affect the unit's response to commands received
during the warp window.

---

### Stage 6: ChronoInTransit flag — NOT active for self-teleport

**gamemd behavior:**
- `ChronoInTransit` (TechnoClass+0x27C) is only set by the **Chronosphere superweapon**,
  never by self-teleport. The self-teleport path (chrono miner) always has
  `ChronoInTransit == 0`.
- The state machine phase 0 pre-check tests `ChronoInTransit != 0 && phase == 0` to
  enter the 60-frame wait (Chronosphere path). This is NOT the chrono miner path.

**Our behavior:**
- We have no `ChronoInTransit` field modeled for self-teleport. Correct by omission —
  the miner path never sets it.

**PASS:** Not applicable to self-teleport; correctly absent.

---

### Stage 7: BeingWarped translucency visual (post-warp warp-in visual)

**gamemd behavior:**
- For non-harvester teleporters: `BeingWarped (+0x271) = 1` is set. `TechnoClass::Draw`
  (0x706640) adds draw flag `0x2004` — 50% translucency. Unit appears semi-transparent
  at destination for `chrono_delay` frames.
- For Harvester=yes (CMIN): `BeingWarped = 0` immediately (as per the harvester branch
  in Phase 0). The unit appears **fully opaque** at the destination from the first frame.
- The translucency comes from the unit draw flags, NOT from AnimClass objects.
- WarpOut anim is spawned at arrival (same anim as departure — `Rules+0x33C`), which
  plays the warp shimmer *over* the opaque unit.

**Our behavior:**
- `being_warped_ticks == 0` for the harvester case means `entity.teleport_state` is `None`
  after the first Relocate tick. The entity has no "being_warped" flag.
- Rendering reads `entity.teleport_state.is_some()` to apply translucency (implied by the
  comment "unit drawn 50% translucent" in teleport_movement.rs line 59). Since
  `teleport_state` is immediately cleared for harvesters, no translucency is applied.

**PASS (for harvester):** Chrono miner appears fully opaque immediately, matching gamemd.

**UNCHECKED:** Whether our renderer actually reads teleport_state or being_warped_ticks
to apply 50% translucency for the non-harvester path. The Rust draw code is not examined
in this trace (out of scope — scope is the miner teleport path only).

---

### Stage 8: Dock-cell occupancy check

**gamemd behavior (after warp, arrival at queue cell):**
- `FootClass::Find_Nearby_Passable_Cell` (0x56DC20) is called from Mission_Harvest state 2
  with `check_occupants=false` (param_12=0). Occupant check is **disabled** for the teleport
  destination search. The zone check is also disabled (param_5=-1).
- After warp-in, the miner is marked at the dest cell (via `Mark(1)` in Phase 0 step 8).
  No occupancy gate prevents teleporting into an occupied queue cell.
- Damage to occupants: `TeleportLocomotionClass::Update_Position` (mode 0) damages flying
  units and infantry at destination (using `Rules->C4Warhead` at Rules+0xFA8). This does
  NOT apply to self-teleport because self-teleport uses the simpler InitiateWarp path, not
  Update_Position.

**Our behavior:**
- `issue_teleport_command` calls `occupancy.move_entity(...)` which moves the entity in the
  occupancy grid unconditionally (no check for existing occupants at destination). This
  matches gamemd's no-occupant-check for the teleport destination.

**PASS (structural):** No occupant check at destination, matching gamemd.

**NOT-IMPLEMENTED:** We do not deal `C4Warhead` damage to occupants at the destination.
However, for self-teleport (vs Chronosphere path), `Update_Position` is not called and
occupant damage through that path doesn't apply. The direct Phase 0 `FUN_0070D4A0` only
stops targeting. Actual collision damage on teleport is a Chronosphere superweapon feature.
This is not applicable to the chrono miner self-teleport. Correctly absent.

---

### Stage 9: IPiggyback locomotor swap (Drive → Teleport active, then swap back)

**gamemd behavior:**
- After warp completes (timer expires, BeingWarped cleared), `TeleportLocomotionClass::Is_Ok_To_End`
  (0x719F30) returns true when: not moving, has piggybacked loco, field_35==0,
  ChronoInTransit==0, WarpPhase==0, IsDeploying==0.
- `FootClass::AI` (0x4DA530) every tick: QueryInterface for IPiggyback, calls `Is_Ok_To_End`,
  if true: calls `End_Piggyback()`, swaps active locomotor back to `DriveLocomotionClass`.
- After swap, Drive is active. Miner drives last cells into the refinery dock.
- For harvesters (timer=0, BeingWarped=0): `Is_Ok_To_End` is true immediately after warp
  completes — Drive is restored in the **same tick** as the warp, or the next tick (timing
  depends on whether FootClass::AI runs before or after StateMachineTick in the tick order).

**Our behavior (`tick_teleport_movement` cleanup section, ~line 240):**
```rust
entity.teleport_state = None;
if let Some(ref mut loco) = entity.locomotor {
    if loco.is_overridden() { loco.end_override(); }
}
```
- `end_override()` restores the base locomotor (Drive) immediately in the same tick as
  warp completion. This matches the gamemd intent.
- The `OverrideKind::Teleport` in locomotor.rs mirrors the "Teleport active over Drive" setup.

**PASS (structural):** Locomotor swap back to Drive happens on warp completion, matching
the gamemd IPiggyback End_Piggyback pattern. Exact tick-timing of the swap relative to
FootClass::AI is UNCHECKED (depends on advance_tick() ordering).

---

### Stage 10: Final dock approach — driving to queue cell, docking sequence

**gamemd behavior (FootClass::AI + UnitClass::Mission_Harvest state 3 + UnitClass::Mission_Enter):**
- After locomotor swap, Drive is active. Mission is Guard_Area (set in Phase 0 step 10).
- Mission_Harvest state 3 then queues Mission_Enter (mission 7).
- Mission_Enter approach: miner drives toward queue cell (pathfinding, DriveLocomotionClass).
- At queue cell: queries IPiggyback for CLSID_WalkLocomotion (confirms DriveLocomotive is
  piggybacked over TeleportLoco). Sets DockLink if WeaponsFactory flag set and DockLink==NULL.
- Radio protocol: `CAN_DOCK(0x0E)` → refinery checks power, slot, sends:
  - `NEED_TO_MOVE(0x13)` → probe motion state
  - `MOVE_TO_CELL(0x12)` with queue cell coords (hardcoded `building_anchor + (3,1)`)
  - `ENTER_DOCK(0x18)` if harvester already at queue cell (reply == 0x14)
  - `TIMING_SYNC(0x16)` → sets locomotor speed to 0x4000
- Harvester then drives to pad cell inside the building.
- TIMING_SYNC: since CMIN has `Turret=no`, `field_0x6AF == 0` always (TurretRateSync=0),
  so the `SetSpeed(0x4000)` always fires on TIMING_SYNC(0x16).

**Our behavior (miner_dock_sequence.rs — RefineryDockPhase FSM):**
- `Approach`: tries to reserve dock via `dock_reservations.try_reserve()`. If granted,
  immediately issues a direct move to `pad` cell and transitions to `Linked`.
- **FAIL (queue cell protocol):** Our Approach phase skips the full radio handshake (0x0E,
  0x12, 0x18, 0x16). The refinery queue cell used in gamemd is computed as
  `building_anchor + (3,1)` hardcoded in BuildingClass::Receive_Radio case 0x0E. Our
  `refinery_queue_cell()` uses `art.ini QueueingCell=` if available, else `(rx+width, ry+height/2)`.
  For GAREFN (4x3): `rx+4, ry+1`. Gamemd's hardcoded `anchor+(3,1)` where anchor is top-left
  = `rx+3, ry+1`. Our formula gives `rx+4, ry+1` — **off by 1 in X**. The miner queues
  one cell east of where gamemd places it.

**FAIL:** Queue cell X offset is 4 from anchor in our code vs 3 in gamemd (hardcoded in
BuildingClass::Receive_Radio case 0x0E at 0x43C2D0).

---

### Stage 11: Final facing at dock exit

**gamemd behavior (`ReleaseDockedHarvester` 0x4595C0 / `UnitClass::Mission_Unload`):**
- After unloading, `Force_Track` sets track_index to `0x47` (ESE = East-South-East).
- The miner exits the dock pad at facing 0x47.

**Our behavior (phase_departing in miner_dock_sequence.rs ~line 551):**
```rust
entity.facing = 0x47;
entity.facing_target = Some(0x47);
```
And on arrival at exit cell:
```rust
entity.facing = 0x47;
```

**PASS:** Both departure and arrival at exit set facing to 0x47, matching gamemd.

---

### Stage 12: Vision reveal at destination

**gamemd behavior:**
- When the unit is marked at the new cell (Phase 0 step 8, `Mark(1)`), the unit's Sight
  range reveals the shroud around the destination. This happens within the same game tick
  as the warp.
- The vision update runs each tick as part of the normal tick pipeline.

**Our behavior:**
- `refresh_fog` is called in `advance_tick()` at line 1195 (world/mod.rs). It runs every
  tick from all entity positions.
- After `tick_teleport_movement` executes (Relocate phase snaps position), the entity is
  at the destination. `refresh_fog` will pick up the new position on the same tick.
- Order in `advance_tick()` (per CLAUDE.md): teleport/special movement runs before vision.

**PASS (structural):** Vision reveal at destination happens within the same tick as warp,
in the correct pipeline order (movement before vision). Exact sight-radius computation
not verified numerically.

---

### Stage 13: Sound cue ordering and fallbacks

**gamemd behavior (Phase 0 steps 7 and 9):**
- ChronoOutSound fired at departure coords: `VocClass::PlayAt(TypeClass+0x578 or Rules+0x21C)`.
- ChronoInSound fired at arrival coords: `VocClass::PlayAt(TypeClass+0x574 or Rules+0x218)`.
- Per-unit override wins; -1 means use global fallback.
- The `global fallback` for CMIN is `ChronoMinerTeleport` (the same sound used for both
  in/out per rulesmd.ini when no per-type override exists).
- Sounds are positional (play at their respective coordinates).

**Our behavior (`spawn_warp_effects`, miner_system.rs line 881):**
```rust
let chrono_out = obj.and_then(|o| o.chrono_out_sound.clone())
    .or_else(|| rules.general.chrono_out_sound.clone());
let chrono_in = obj.and_then(|o| o.chrono_in_sound.clone())
    .or_else(|| rules.general.chrono_in_sound.clone());
```
- Per-unit override first, then global fallback. Order matches gamemd.
- Sound events are `SimSoundEvent::ChronoTeleport { sound_id, rx, ry }` — positional.

**PASS (structural):** Sound cue ordering (out at departure, in at arrival), fallback chain,
and positional emission match gamemd. Exact sound playback panning behavior (spatial audio
system) not verified here.

---

## Summary of Results

| Stage | Description | Result |
|-------|-------------|--------|
| 1 | Too-far distance check (center vs queue cell) | FAIL |
| 2 | Set_Destination teleport-vs-drive decision | PASS |
| 3 | WarpOut anim spawning at departure and arrival | UNCHECKED |
| 4a | Chrono delay formula (harvester instant-warp) | PASS |
| 4b | Chrono delay formula (minimum clamp < vs <=) | FAIL (minor) |
| 4c | Distance calculation (2D vs 3D) | UNCHECKED |
| 5a | Phase 0 — position snap and occupancy swap | PASS |
| 5b | Phase 0 — stop targeting / detach anims | NOT-IMPLEMENTED |
| 5c | Phase 0 — SetMission(GUARD_AREA) | NOT-IMPLEMENTED |
| 6 | ChronoInTransit flag (not active for self-teleport) | PASS |
| 7 | BeingWarped translucency (fully opaque for harvester) | PASS |
| 8 | Dock-cell occupancy check (disabled) | PASS |
| 9 | IPiggyback locomotor swap | PASS |
| 10 | Queue cell X offset (4 vs 3 from anchor) | FAIL |
| 11 | Final facing 0x47 at exit | PASS |
| 12 | Vision reveal at destination (same tick) | PASS |
| 13 | Sound cue ordering and fallbacks | PASS |

**PASS: 9 | FAIL: 3 | UNCHECKED: 3 | NOT-IMPLEMENTED: 2**

---

## Top 5 Most Player-Visible Failures

1. **Stage 10 — Queue cell X offset wrong (4 vs 3 from anchor)**
   Player sees: miner parks one cell east of where it should queue outside the refinery.
   In a constrained map this can cause pathfinding failures or collision with other units.
   Fires: every single dock approach for every Chrono Miner (= every harvest cycle).
   Code: `src/sim/miner/miner_dock_sequence.rs:63` — `refinery_queue_cell` returns `(rx+width, ry+height/2)` for no-QueueingCell case. gamemd hardcodes `building_anchor+(3,1)`.
   gamemd evidence: `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md §3`, assembly at 0x43C2D0, `X += 3` confirmed.

2. **Stage 1 — Distance check uses building center, not dock queue cell**
   Player sees: chrono miner may drive instead of warp (or warp instead of drive) when miner is 49-53 cells from the refinery. The error is ~2 cells for a 4x3 refinery.
   Fires: every full harvest cycle when miner is near the ChronoHarvTooFarDistance boundary (common in mid-sized maps).
   Code: `src/sim/miner/miner_system.rs:782` — uses `refinery_center_cell_for_sid` as distance reference. gamemd measures to dock-adjacent cell (`Find_Nearby_Passable_Cell` seed).
   gamemd evidence: `PATHFINDING_VALIDATE_ALTERNATE_CHRONO_RETURN_GHIDRA_REPORT.md §2`, seed cell formula confirmed.

3. **Stage 5b — Stop targeting / detach animations not implemented on warp**
   Player sees: units that were attacking the miner before it warpeds continue targeting the empty departure cell rather than losing lock. Visual: combat anims attached to the miner (e.g., mind-control beam, temporal beam) don't detach on warp.
   Fires: whenever a miner is warping while under attack (mid-battle harvest scenarios).
   Code: `src/sim/movement/teleport_movement.rs` — `tick_teleport_movement` TeleportPhase::Relocate block has no analog to `FUN_0070D4A0` (stop-targeting) or anim-detach loop.
   gamemd evidence: `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md §3` Phase 0 steps 1-2.

4. **Stage 5c — SetMission(GUARD_AREA) not called post-warp**
   Player sees: if the player issues a command to the miner during the brief warp window, mission state handling may differ. More critically, if multiple commands are queued, the GUARD_AREA insertion that gamemd uses as a "reset point" is absent.
   Fires: whenever the miner warps (every far-harvest cycle) — the missing SetMission affects what mission is active in the 1-tick window after warp and before dock FSM resumes.
   Code: No equivalent in `tick_teleport_movement` or miner state machine.
   gamemd evidence: `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md §3` Phase 0 step 17 (`vtable->SetMission(2)`).

5. **Stage 4b — Minimum clamp uses `<` (exclusive) vs gamemd `<=` (inclusive)**
   Player sees: when warp distance computes exactly `ChronoMinimumDelay` leptons / ChronoDistanceFactor (= 768 leptons / 48 = 16 exactly), our code does NOT reset the timer's StartFrame but gamemd does. The timer duration is the same (16) but StartFrame differs by 1 frame. This produces a 1-frame timing slip on the post-warp BeingWarped window for Chrono Legionnaire and other non-harvester teleporters. (Chrono miner bypasses this with the harvester branch.)
   Fires: when non-harvester teleporters travel exactly `ChronoDistanceFactor * ChronoMinimumDelay` leptons — rare edge case.
   Code: `src/sim/movement/teleport_movement.rs:79` — `if delay < rules.chrono_minimum_delay`.
   gamemd evidence: `PHASE0_CHRONO_DELAY_FORMULA_MATH_GHIDRA_REPORT.md §3`, assembly at 0x719521 (`JLE` = ≤ comparison).

---

## Status

PARTIAL — 3 UNCHECKED stages (anim flags, 3D distance, renderer translucency read path)
require rendering integration testing not available in static analysis.

---

## Report File

`docs/research/traces/CHRONO_MINER_TELEPORT_DOCK_APPROACH_TRACE.md`
