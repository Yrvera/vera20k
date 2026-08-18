# Unit-to-Unit Collision and Re-Path Triggers — Ghidra Research Report

**Primary function:** `DriveLocomotionClass::Process_Movement @ 0x004b2630`
(confirmed, ~8.5 KB, body 0x004b2630–0x004b4766)
**Supporting functions:**
- `FootClass::Find_Path @ 0x004d3920`
- `FootClass::Find_Nearby_Passable_Cell @ 0x0056dc20`
- `TechnoClass__Is_Current_Cell_Obstacle_Free @ 0x00486ff0` (renamed; rejects cells
  containing Building (RTTI=6) or Terrain (RTTI=0x24/36) occupants)
- `FootClass::Can_Enter_Cell` (vtable +0x1AC)
- `CellClass::Scatter_Objects @ 0x00481670`

**Confidence:** HIGH (decompiled and verified in Ghidra 2026-04-05).
**Active in YR:** Yes — core ground-unit movement path.

## 1. Overview

This report consolidates two interleaved concerns in ground-unit movement:
1. How units behave when another unit blocks their next cell during pathing.
2. What conditions cause a unit to abandon its current path queue and recompute.

Both flows live inside `DriveLocomotionClass::Process_Movement`. It runs each frame a
unit is moving, picks the next direction from the 24-entry `path_queue` at FootClass
offset `+0x5E0`, validates the cell via `Can_Enter_Cell` (vtable `+0x1AC`), and
dispatches on the 0–7 return code.

This report **extends** existing docs rather than re-deriving them. See also:
- `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` — the 0–7 return-code taxonomy
- `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md` — broader locomotion flow
- `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` — A* cost table for blockers
- `SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md` — 6 scatter call sites
- `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md` — CellClass::Scatter_Objects
- `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md` — fallback destination search

This report focuses on **verifying the timer/urgency state machine and enumerating every
repath trigger** reachable from Process_Movement.

## 2. FootClass State Fields Used by Collision/Repath

FootClass is accessed in this function as `*(int *)(param_1 + 0xC)` (drive.this) or
`extraout_ECX`. All offsets below are **direct byte offsets into FootClass**.

| Offset  | Name (inferred)         | Type  | Purpose                                             |
|---------|-------------------------|-------|-----------------------------------------------------|
| `+0x5E0`| `path_queue[0]`         | u32[24]| 24-entry facing queue (8-dir encoded). `-1` = empty |
| `+0x638`| (queue slot 22 sentinel)| u32   | Last entry of the 24-step `path_queue` ring buffer. NOT a separate "previous direction" field — `DriveLocomotionClass::Process_Movement` shifts 22 ring slots left then writes `*(param_1+0x638) = 0xffffffff` as the queue end sentinel. Walk uses `+0x63C` (23-entry shift) instead. Verified via `decompile_function 0x004B2630` and `decompile_function 0x0075AEC0`. |
| `+0x640`| `movement_delay_start`  | i32   | Frame counter when movement delay started (`-1`=off)|
| `+0x644`| `movement_delay_facing` | u32   | Snapshot of pending direction                        |
| `+0x648`| `movement_delay_ticks`  | u32   | Duration of movement delay                           |
| `+0x64C`| `path_stuck_counter`    | i32   | Retry budget for "no path found" (reset to 10)      |
| `+0x668`| `blocked_delay_start`   | i32   | Frame counter when blocked-by-friendly began        |
| `+0x66C`| `blocked_delay_facing`  | u32   | Snapshot of pending direction                       |
| `+0x670`| `blocked_delay_ticks`   | u32   | Duration of blocked delay (= `Rules.BlockedDelay`)  |
| `+0x688`| `give_up_flag`          | u8    | Set when convoy/tether abandoned                    |
| `+0x68A`| `pending_blocked_sound` | u8    | Set to play blocked VO next tick                    |
| `+0x68B`| `path_direction_switched`| u8   | Bridge/elevation transition marker                  |
| `+0x6B7`| `path_blocked_flag`     | u8    | `1` once blocked by code-2 (friendly moving)        |

## 3. The Cell-Entry Return Code Dispatch

When `Process_Movement` reaches the direction at `path_queue[0]`, it calls
`Can_Enter_Cell` on the target cell. The return code drives the entire collision +
repath behavior.

**Confirmed from decompilation of 0x004b2630 (dispatch block near LAB_004b35e3):**

| Code | Meaning                          | Behavior in Process_Movement                                      |
|------|----------------------------------|-------------------------------------------------------------------|
| 0    | OK                               | Proceed with move, commit drive-track                             |
| 1    | Overlay blocking (civilian etc.) | `CellClass::Mark_Objects_Redraw`, clear head_to, recurse with `(0,0)` |
| 2    | Temporary block (friendly moving)| **Start/check blocked_delay, Find_Path with escalating urgency** — see §4 |
| 3    | Scatter-required (crushable)     | `Check_Crushable_Obstacle`, fall through to common clear+return   |
| 4/5  | Friendly wall / building         | If `is_retry` → clear path_queue, recurse with force_repath; else try crush or kick scatter |
| 6    | Blocked by allied stationary     | See §5 (close-enough stop vs. scatter)                            |
| 7    | Head-on deadlock / impassable    | If `is_retry` → clear path_queue, recurse; else clear head_to, Stop_Moving |

**Train exception (downgrade-all):** any unit with `TypeClass +0xC94` set (**`IsTrain`**, NOT Crusher) **downgrades codes <7 to 0** — trains pass through every soft blocker (civilian, friendly moving, scatter, friendly wall, friendly stationary, head-on) and only stop at code 7 (hard impassable). Verified via `decompile_function 0x004B2630` (5 reads of +0xC94 across the function — gates trains-vs-allied-blockers, trains skip code 6, and the universal `result < 7` downgrade; all consistent with the IsTrain flag confirmed by ReadINI in DRIVE_LOCOMOTION_CLASS.md §"Convoy / Formation Propagation"). **Real `Crusher` flag at `+0xD28`** has a narrower path: only downgrades codes 4 or 5 (friendly wall / building) AND only when the destination cell's overlay-type-index (+0x44) == 0. Prior doc revision labelled +0xC94 as "Crusher" — that was a semantic-effect-naming mistake; the binary reads IsTrain at this offset.

## 4. Code 2 (Temp-Block) — The Urgency Escalation State Machine

This is the central collision-wait mechanism. Verified from LAB_004b3607 (code-2 branch):

```
// On first entry per block (path_blocked_flag==0):
if (foot->path_blocked_flag == 0) {
    foot->path_blocked_flag        = 1;
    foot->blocked_delay_start      = g_CurrentFrameCounter;
    foot->blocked_delay_facing     = snapshot_of_pending_facing;
    foot->blocked_delay_ticks      = Rules->BlockedDelay;   // +0x1768
}

// Gate on movement_delay — if still active, don't even try to repath.
// (movement_delay = rate limiter on pathfinder calls)
if (movement_delay_start != -1) {
    elapsed = g_CurrentFrameCounter - movement_delay_start;
    if (elapsed < movement_delay_ticks) {
        // Movement delay still running: play blocked sound if pending, return
        goto LAB_004b3aa1;   // skips repath this tick
    }
}

// Movement delay expired — now decide urgency.
// urgency = 1 if blocked_delay still ticking, urgency = 2 if blocked_delay expired
blocked_delay_expired = FALSE;
if (foot->path_blocked_flag) {
    elapsed = g_CurrentFrameCounter - foot->blocked_delay_start;
    if (elapsed >= foot->blocked_delay_ticks) {
        blocked_delay_expired = TRUE;   // → urgency = 2 "destroyer mode"
    }
}

// Call Find_Path with urgency = blocked_delay_expired ? 2 : 1
success = FootClass::Find_Path(dest_cell, /*is_crusher*/0, urgency);

if (!success && !foot->is_dying) {
    (vtable +0x480)(0,1);   // Mark_Destination and bail this tick
    return 0;
}

// Success → restart movement_delay as a rate limiter for the next attempt
foot->movement_delay_start  = g_CurrentFrameCounter;
foot->movement_delay_facing = snapshot_of_pending_facing;
foot->movement_delay_ticks  = Math::ftol(...);  // variable-delay, typically small
return 1;
```

**Key insight:** there are TWO timers stacked:
- `movement_delay` (+0x640/+0x644/+0x648) — short rate-limiter between pathfinder calls.
  Prevents thrashing when blocked.
- `blocked_delay` (+0x668/+0x66C/+0x670) — longer patience timer. Duration = `Rules.BlockedDelay`.
  Controls **urgency escalation**: while running, urgency=1 (passive repath). Once expired,
  urgency=2 → Find_Path is told to treat friendly-moving blockers as **high-cost (1000.0
  instead of 4.0) in the A\* open-list**, effectively routing around them instead of
  queueing behind them.

**When is `path_blocked_flag` cleared?** Only when a movement step succeeds (code 0 from
Can_Enter_Cell — normal cell entry flow). Once a tick proceeds to LAB_004b460c (commit
drive-track), `pending_blocked_sound` at `+0x68A` is zeroed; the blocked flag itself gets
reset when a new Find_Path succeeds in the code-2 branch above (via the
`movement_delay_start = CurrentFrame` assignment and fresh path queue fill).

**`Rules.BlockedDelay` INI key:** read from `g_RulesClass_Instance + 0x1768`. This is
the `BlockagePathDelay=` field in rulesmd.ini `[General]`. Verified from the
`*(int *)(iVar5 + 0x670) = uVar8` assignment where `uVar8 = *(g_RulesClass_Instance + 0x1768)`.

## 5. Code 6 (Blocked by Allied Stationary) — Close-Enough vs. Scatter

When `Can_Enter_Cell` returns 6 (allied stationary unit/building blocks):

```
// Train override (not Crusher): if owner is IsTrain (TechnoTypeClass+0xC94 != 0),
// goto common-clear (LAB_004b3607) — trains skip code-6 entirely.
if (foot->type->IsTrain /* +0xC94 */) goto LAB_004b3607;

// Retry recursion: if is_retry flag set by caller, clear path head and recurse
if (is_retry) {
    foot->path_state[0x178] = -1;  // wipe direction state
    foot->movement_delay_start = g_CurrentFrameCounter;
    goto LAB_004b4541;  // self-recurse Process_Movement with (is_retry=0, force=0)
}

// Compute 3D distance from current pos to destination
dist = Sqrt_Approx((fx-dx)^2 + (fy-dy)^2 + (fz-dz)^2);

// "Close enough to give up"?
if (dist < Rules->CloseEnough) {         // +0x1718
    // Z-check: within 2 DriveHeightSteps vertically?
    if (abs(fz - dz) < 2 * DriveHeightStep) {
        // Final guard: destination cell scenario flag (+0xEC) != 10
        if (dest_cell->field_0xEC != 10) {
            // STOP here — treat as arrived
            clear head_to = NullCoord
            if (foot->in_mission == 0) (vtable +0x480)(0,1);  // Mark
            else { FootClass::Stop_Moving(); (vtable +0x484)(0,1); }
            goto LAB_004b3607;  // common clear
        }
    }
}

// Not close enough — try to shove the blocker via scatter
bridge_flag = (dest_cell->flags & 0x100) && abs(fz/DriveHeightStep - dest_cell->level) >= 3;
CellClass::Scatter_Objects(NullCoord, /*threat*/1, /*force*/1, bridge_flag);
goto LAB_004b3607;   // common clear, then continue next tick
```

**`Rules.CloseEnough` INI key:** read from `g_RulesClass_Instance + 0x1718`. This is
`CloseEnough=` in rulesmd.ini `[General]`, default **`0x240` (2.25 cells = 576 leptons)** (verified via `ini/rulesmd.ini`). Confirmed present in
`ini/rulesmd.ini` at `[General]` section.

**Semantic:** if your destination is within 2 cells AND at approximately the same
elevation AND the final cell isn't marked special, you **stop where you are** rather
than repath. Otherwise you request a scatter on the blocker and keep trying.

## 6. Path_Stuck_Counter — The Hard Give-Up Circuit Breaker

Verified from the first half of Process_Movement (uVar18 == 0xffffffff branch, the "no
valid next cell" path):

**Mechanics:**
- **Reset point:** LAB_004b3282 — set `foot->path_stuck_counter = 10` when the current
  direction maps to an unwalkable cell (out of playfield, Can_Enter_Cell returns !6, or
  allied scatter target invalid). This reset happens on *any* blocked-but-handleable tick.
- **Decrement point:** when `Find_Path` returns 0 (failed entirely) AND the unit is not
  dying, the "no-path" branch decrements: `if (foot->path_stuck_counter < 1) { give_up }
  else { foot->path_stuck_counter -= 1 }`.
- **Give-up action:** clear `head_to = NullCoord`, clear `drive_track_index = -1`,
  play `pending_blocked_sound` (if set), call `Stop_Moving` or `Mark` via vtable+0x480.

**Key nuance (correction to existing Rust impl assumption):** the counter is
**decremented only when Find_Path fully fails**, not on every blocked tick. A unit
stuck in traffic that can still compute *a* path (even if that path leads back into
the jam) will have its counter reset to 10 every frame. So this is
"consecutive failed pathfind attempts," not "consecutive blocked ticks."

## 7. Distance-Based Repath Trigger (mid-step)

Verified in the per-direction processing block (around LAB_004b3f7d / "uVar19 ==
0xffffffff" sub-branch):

```
// When path_queue[current] == -1 (no next cell) AND path_queue[1] is not set either:
compute 3D distance from (techno.pos) to (head_to)
if (dist > 0x200 leptons /* 512 = 2 cells */) {
    success = FootClass::Find_Path(head_to_cell, is_crusher, /*urgency*/0);
    // urgency=0 here because this is proactive continuation, not reaction to block
    if (!success && !is_dying) (vtable+0x480)(0,1);
}
```

**This fires when the 24-step path queue has drained but the unit is still >2 cells from
its destination.** This is the primary "continue pathing to a far destination" repath.

Urgency is **0** here (vs. 1/2 in code-2), meaning A* uses default costs — no
destroyer-mode penalty on friendly-moving blockers.

## 8. Complete Re-Path Trigger Enumeration

Every place Process_Movement (or its helpers) calls `FootClass::Find_Path`:

| # | Trigger                                | Urgency | Caller site              | Active in YR |
|---|----------------------------------------|---------|--------------------------|--------------|
| 1 | `path_queue[0] == -1` AND `movement_delay` expired AND head_to != NullCoord (new path needed) | 0 | LAB_004b281c                | Yes |
| 2 | Code 2 (friendly moving blocks) AND movement_delay expired: urgency depends on blocked_delay | 1 or 2 | LAB_004b3607 (code 2)      | Yes |
| 3 | Path queue drained mid-path AND dist to dest > 0x200 leptons | 0 | LAB_004b3f7d inner      | Yes |
| 4 | Recursive re-entry after `is_retry` clear (codes 4/5/6/7 with param_2=1) | (recurses) | LAB_004b4541                | Yes |
| 5 | Continuation from `extraout_ECX[0x1B2]` chained `Pathfinding_update_continued` loop (tether / linked paths) | — | inside Find_Path itself | Yes (rare) |

**No timer-based expiry exists.** A path does not "age out" — it's only replaced on
block, exhaustion, or unit-driven cancel. This matches the existing Rust
implementation (`movement_tick.rs` — "No repath expiry by age").

## 9. Scatter Path — How a Blocker Gets Nudged

When code 6 (or code 3) fires and we're not close-enough:

```
CellClass::Scatter_Objects(NullCoord, threat=1, force=1, bridge_flag)
 └─ gathers up to 10 occupants in cell
 └─ for each occupant, calls vtable+0x174 (TechnoClass::Scatter) if:
    • elite in cell,
    • force=1 (TRUE here),
    • Rules.PlayerScatter set,
    • HasWeaponAbility(3), or
    • owner's IQ ≥ Rules.IQ_Scatter
 └─ Scatter issues a 1-cell movement order to a free adjacent cell
```

**Important:** Scatter is asynchronous. The blocker receives a move command on a
later tick. Our unit's `Process_Movement` just moves on via LAB_004b3607 (clear
head/return), and will retry next tick, by which point the blocker may have moved.

If the blocker cannot scatter (cornered), our unit loops:
- code 6 → not close enough → scatter (ineffective) → retry next tick → code 6 again...
  — until either blocker eventually moves OR our unit moves within `CloseEnough` and stops.

## 10. INI Keys

| Key                  | Section    | Offset | Default | Purpose                              |
|----------------------|------------|--------|---------|--------------------------------------|
| `BlockagePathDelay=` | [General]  | +0x1768 | **60** (frames) | blocked_delay timer duration — verified via grep of `ini/rulesmd.ini` |
| `CloseEnough=`       | [General]  | +0x1718 | **0x240** (2.25 cells, 576 leptons) | stop distance for code 6 — verified via grep of `ini/rulesmd.ini` (`CloseEnough=2.25`) |
| `PathDelay=`         | [General]  | —       | (small) | movement_delay base duration         |
| `IQ.Scatter=`        | [General]  | —       | 2       | Min AI IQ for scatter participation  |
| `PlayerScatter=`     | [General]  | —       | **no**  | Master toggle for scatter dispatch — verified via grep of `ini/rulesmd.ini` (`PlayerScatter=no`). Player-owned units do NOT scatter from threats by default in standard YR.   |

Values above reflect `ini/rulesmd.ini` (YR master). Verified from Ghidra that offsets
`+0x1768` and `+0x1718` on g_RulesClass_Instance are read at the sites documented here.

## 11. Integration Points

**Callers:** `Process_Movement` is invoked from the unit/drive tick driver each frame
the unit has a non-null `drive_locomotor`. The sim tick-order position is **ground
movement** (per `sim/mod.rs` tick order — "commands → ground movement → ..."). It runs
before vision/turrets/combat.

**Callees of interest:**
- `Can_Enter_Cell` (vtable +0x1AC) — per-cell legality + block classification
- `Find_Path @ 0x4D3920` — entry to A* (calls Run_AStar)
- `Find_Nearby_Passable_Cell @ 0x56DC20` — fallback dest when final cell blocked
- `Scatter_Objects @ 0x481670` — dispatches blocker nudges
- `Check_Crushable_Obstacle` — kill/crush overlap
- `Rate_Timer` / `RateTimer::Current` — frame-based timer helpers

## 12. Current Rust Implementation Status

Based on the scan summary (see src/sim/movement_*):

| Feature                            | Status     | Rust location                              |
|------------------------------------|------------|--------------------------------------------|
| Cell-entry return code taxonomy    | Implemented | `cell_entry::classify_occupied_cell`      |
| Code 2 (temp block) wait           | Implemented | `movement_blocked.rs:handle_blocked_tick` |
| Code 6 stop-at-close-enough        | Partial    | (check for CloseEnough gate)              |
| Code 3 crushable handling          | Implemented | `bump_crush.rs`                           |
| Code 7 deadlock re-path            | Implemented | `movement_occupancy::handle_deferred_occupancy` |
| Dual timer (movement + blocked)    | Implemented | `MovementTarget::blocked_delay` field     |
| Urgency escalation (1 → 2)         | **Missing**? | Verify: does `try_repath_after_block` pass urgency param to A*? |
| Path_stuck_counter semantics       | Implemented but different | Current: increments on repath success. **Binary**: resets to 10 on any blocked tick where ≥1 dir was unwalkable; decrements only on Find_Path failure |
| Distance-based mid-path repath     | Implemented | `movement_tick::handle_path_exhaustion`   |
| Scatter dispatch                   | Implemented | `bump_crush::scatter_blocker`, `scatter.rs` |

## 13. Resolved Questions (follow-up pass)

### 13.1 A\* cost table and urgency=2 multiplier — VERIFIED

**Function:** `AStar_compute_edge_cost @ 0x00429830`.

**Base cost table** (8 floats at `DAT_0081870c`, indexed by Can_Enter_Cell return code):

| Code | Cost float | Bytes | Meaning |
|------|-----------:|-------|---------|
| 0    | 1.0        | `3f800000` | OK |
| 1    | 1000.0     | `447a0000` | Overlay/civilian blocker |
| 2    | 1.0 (baseline; overridden) | `3f800000` | Moving friendly — see dynamic logic below |
| 3    | 1.0        | `3f800000` | Crushable |
| 4    | 60.0       | `42700000` | Friendly wall |
| 5    | 20.0       | `41a00000` | Enemy |
| 6    | 8.0        | `41000000` | Allied stationary |
| 7    | 10000.0    | `461c4000` | Impassable |

**Code 2 dynamic cost** depends on `PathfinderClass->field_0x3c` (urgency stored at
entry to `AStar_pathfind_search @ 0x0042c900`, where it writes
`*(uint *)(this + 0x3C) = urgency;`):

```
if (incoming_code == 2) {
    piVar9 = cell->FirstObject;  // or AltObject for bridge cell
    if (urgency == 0) {
        // Walk forward up to 10 cells along each blocker's path_queue[0] chain
        for (i = 0; i < 10; i++) {
            if (piVar9 == NULL) {
                // Blocker chain ends within 10 steps → traffic will clear
                goto keep_baseline_1_0;
            }
            if (!(piVar9->flags & IsReallyMoving)) break;  // stationary blocker → penalize
            next_dir = (blocker.speed==0 ? blocker.path_queue[0] : rand_direction);
            piVar9 = next_cell->FirstObject;  // hop to next cell's occupant chain
        }
        // Loop fell through (10 iterations of moving-friendly chain) OR blocker idle
        cost = 4.0;  // traffic-jam penalty
    } else {
        cost = 4.0;  // urgency 1: immediately use traffic penalty, no look-ahead
    }
    if (urgency == 2) cost = 1000.0;  // escalate to route-around
    keep_baseline_1_0: ;
}
```

**Resolved semantics:**
| Urgency | Look-ahead result          | Cost     | Intent                          |
|---------|----------------------------|----------|---------------------------------|
| 0       | Chain clears within 10 steps| 1.0     | Queue optimistically            |
| 0       | Full 10-step jam or idle    | 4.0     | Prefer alternate routes         |
| 1       | (no look-ahead)             | 4.0     | Blocked once → prefer alternate |
| 2       | (no look-ahead)             | 1000.0  | Escalate → route AROUND blocker |

The existing `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` claim (4.0 normal, 1000.0
destroyer-mode) is **partially correct** — missing the 1.0-baseline case for
short-horizon clearable chains. Corrected here.

**Cost multipliers applied after the base cost:**
- Bridge cell: `cost *= 4.0` (`_DAT_007e37bc` = `40800000`)
- Diagonal move, non-bridge: `cost *= 10.0` (`_DAT_007e37b8` = `41200000`)
- Diagonal move, bridge target (neighbor also bridge): `cost *= 2.0` (`_DAT_007e37b4` = `40000000`)
- Diagonal move, bridge target (neighbor not bridge): `cost *= 1.0` (`_DAT_007e2ac8` = `3f800000`)

**A\* retry loop** (in `AStar_pathfind_search`): maximum retries computed as
`(-(param_6 != -1) & 0xfffffffc) + 5` → **5 retries** if no zone override, **1** if override.
Each failure calls `PathfinderClass__UpdateHierarchicalEdges` (the "broken zone edges"
mechanism), `PathfinderClass__Reset`, and re-tries `AStar_main_loop`.

### 13.2 `cell->field_0xEC` — VERIFIED

**`CellClass +0xEC` = `LandType` (int)**, per struct layout lookup (size 328, offset 236).

**`LandType == 10` → `Tunnel`** (the `cell->field_0xEC != 10` guard is confirmed by `decompile_function 0x004B2630`; the value `10` for Tunnel matches WW convention). **The previously-cited string table at `0x0081dbc0`–`0x0081dc1c` is NOT the LandType enum source** — those bytes are SpeedType / locomotor names (Wheel, Track, Foot, Weeds, Tunnel…). The LandType enum below is given by convention; the binary table backing it has not been independently located. Enum values used below:

| Idx | Name     | Idx | Name    |
|----:|----------|----:|---------|
| 0   | Clear    | 6   | Beach   |
| 1   | Road     | 7   | Rough   |
| 2   | Water    | 8   | Ice     |
| 3   | Rock     | 9   | Railroad|
| 4   | Wall     | **10**| **Tunnel** |
| 5   | Tiberium | 11  | Weeds   |

**Meaning of the close-enough guard** `if (cell->LandType != Tunnel)`: when the
destination cell is a tunnel entrance (used by TunnelLocomotion / subterranean APC),
the unit must **not** early-stop at `CloseEnough` range — it must complete the move
fully to trigger the tunnel entry transition. For any other land type, stopping within
`CloseEnough` + height constraint is permitted.

### 13.3 `path_blocked_flag` (+0x6B7) reset — VERIFIED (and surprising)

**All writes to FootClass +0x6B7** (byte-pattern scan on `b7 06 00 00` + analysis):

| Address       | Function                         | Op | Value |
|---------------|----------------------------------|----|------:|
| `0x004d3451`  | FootClass::Constructor           | `MOV [ESI+0x6B7],BL` | 0 (init) |
| `0x004b3663`  | DriveLocomotionClass::Process_Movement | `MOV imm`  | **1** |
| `0x00515d3a`  | HoverLocomotionClass::Process_Movement (`FUN_00514f70`) | `MOV imm` | **1** |
| `0x005b0bd4`  | JumpjetLocomotionClass::Process_Movement (`FUN_005b01c0`) | `MOV imm` | **1** |
| `0x006a2cb2`  | ShipLocomotionClass::Process_Movement | `MOV imm` | **1** |
| `0x0075b8b6`  | WalkLocomotionClass::ProcessMovement | `MOV imm` | **1** |
| `0x0075be11`  | WalkLocomotionClass::ProcessMovement | `MOV imm` | **0** (waypoint arrival) |
| `0x0075bfd1`  | WalkLocomotionClass::ProcessMovement | `MOV imm` | **0** (facing rotation branch) |

**Key finding:** **the flag is cleared ONLY by infantry (Walk locomotor).** Drive,
Ship, Hover, Jumpjet locomotors set it to 1 when code-2 blocking first occurs and
**never clear it within their own code**. `FootClass::Stop_Moving @ 0x4df0d0` does NOT
touch +0x6B7 either (body is just two pointer zeroes at +0x5A0/+0x5A4).

**Practical consequence for vehicles/ships:**
1. First code-2 block event → flag set, blocked_delay timer (= BlockagePathDelay) starts.
2. While blocked: urgency=1 until timer expires, then urgency=2.
3. Unit unblocks and continues moving. **Flag stays set. Timer stays expired.**
4. Any subsequent code-2 block event within this unit's lifetime: the timer is already
   stale and already expired → **unit immediately uses urgency=2** (route-around mode)
   from the very first blocked tick, skipping the grace period entirely.

**Infantry semantics differ:** the clear at `0x0075be11` fires inside the
"waypoint-reached" block (`if (dist_to_subcell < 0x11)` — 17 leptons): on every
successful sub-cell arrival, the flag is cleared, giving each new block event a fresh
grace period.

**This is NOT a bug — it is verified behavior of the retail binary.** It means the
game intentionally grows more impatient with vehicles as they accumulate blocking
events, while infantry's "impatience" resets at each cell step. This asymmetry should
be preserved in any faithful re-implementation.

### 13.4 Other open items (deferred)

- **`is_retry` external callers:** xref check (all confirmed self-recursion; no evidence
  of external invocation with is_retry=1).
- ~~**`FUN_00486ff0` type 0x24 semantic:** WhatAmI() enum lookup still pending.~~
  **RESOLVED:** `RTTIType::Terrain == 0x24 (36)` — verified via TerrainClass vtable at
  `0x007f5200`, WhatAmI at `0x0071d300`. `RTTIType::Building == 6` — verified via
  BuildingClass vtable at `0x007e3ebc`, WhatAmI at `0x00459ec0`. Function renamed to
  `TechnoClass__Is_Current_Cell_Obstacle_Free`.
- **Convoy/tether `Pathfinding_update_continued` loop:** out of scope for this report.

## 14. Revised Rust Implementation Delta (post-verification)

| Feature                              | Binary behavior                                    | Rust status |
|--------------------------------------|----------------------------------------------------|-------------|
| Code 2 cost lookup table             | 8-entry float table w/ code-2 dynamic override     | Verify      |
| Code 2 urgency-0 look-ahead (10 cells)| Walks blocker chain, returns 1.0 if it clears     | **Missing** |
| Urgency→cost mapping {0:dynamic, 1:4.0, 2:1000.0} | Exact | Verify A* cost function |
| CloseEnough `LandType != Tunnel` guard | Verified                                         | **Missing**? |
| path_blocked_flag: infantry clears on cell-arrival, vehicles never clear | Verified, asymmetric | **Likely missing** (Rust probably resets uniformly) |
| A* retry budget (5 retries, hierarchical edge updates) | Verified                        | Verify      |
| Bridge cell cost multiplier (4.0×)   | Verified                                           | Verify      |

## Sources

**Ghidra addresses decompiled (both passes):**
- 0x004b2630 `DriveLocomotionClass::Process_Movement` (full)
- 0x004d3920 `FootClass::Find_Path` (full)
- 0x00486ff0 `TechnoClass__Is_Current_Cell_Obstacle_Free` (renamed from `FUN_00486ff0`)
- 0x004db9b0 `FUN_004db9b0` (small vtable helper, noted)
- 0x0075aec0 `WalkLocomotionClass::ProcessMovement` (full, clear sites confirmed)
- 0x004df0d0 `FootClass::Stop_Moving` (tiny, confirms no flag touch)
- 0x004cbba0 `FootClass::Run_AStar` (thin wrapper, full)
- 0x0042c900 `AStar_pathfind_search` (full, urgency storage + retry loop)
- 0x00429830 `AStar_compute_edge_cost` (full, cost table + urgency dispatch)

**Data tables read:**
- 0x0081870c: A\* base cost table (8 floats, codes 0–7)
- 0x007e37b4/b8/bc, 0x007e2ac8: diagonal/bridge cost multipliers
- 0x0081dbc0–0x0081dc1c: **SpeedType / locomotor name strings** (Wheel, Track, Foot, Weeds, Tunnel…), NOT LandType — correction from prior audit revision
- `CellClass` struct layout (size 328 bytes, LandType at +0xEC)

**Docs cross-referenced:**
- `ra2-rust-game-docs/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`
- `ra2-rust-game-docs/SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md`
- `ra2-rust-game-docs/DRIVE_LOCOMOTION_CLASS.md`
- `ra2-rust-game-docs/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`

**INI files checked:**
- `ini/rulesmd.ini` [General] — BlockagePathDelay, CloseEnough, PathDelay, IQ.Scatter,
  PlayerScatter
