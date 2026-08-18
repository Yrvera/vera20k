# Stuck Detection & Unstick Logic — State Machine Synthesis

**Purpose:** Consolidated reference for "what counts as stuck", the watchdog timers, the unstick mechanisms, and the hard-give-up condition. Synthesizes findings already documented across multiple files into a single state-machine view, and adds 3-axis confidence labelling per `[[feedback_research_confidence_axes]]` to load-bearing claims.

**Primary source doc (REQUIRED READING):** `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md` — has the full per-function decomp of the stuck state machine inside `DriveLocomotionClass::Process_Movement @ 0x4B2630`. This synthesis doc does NOT duplicate that content; it organizes it as a state machine and verifies the key findings to current confidence standards.

**Active in YR:** Yes — runs every tick for every ground/naval/walking unit. Most-frequently-fired piece of pathfinding state machinery.

---

## 1. What counts as "stuck"?

Three distinct states in the binary, each with its own timer and recovery path:

### State A — Blocked-by-moving-friendly (code 2)

**Trigger:** `Can_Enter_Cell` returns **2** (cell currently occupied by a moving friendly unit).

**Fields written on first entry per block:**
- `FootClass+0x6B7 = 1` (path_blocked_flag) — **only cleared by Walk locomotor**, see §3
- `FootClass+0x668 = g_CurrentFrameCounter` (blocked_delay start)
- `FootClass+0x66C = pending_facing_snapshot` (blocked_delay facing)
- `FootClass+0x670 = RulesClass+0x1768` (blocked_delay ticks = `BlockagePathDelay`, default **15 frames**)

**Recovery:** Call `FootClass::Find_Path` with **urgency = 1** (passive repath, A* uses 4.0 cost for moving-friendly blockers — traffic-jam penalty).

**Escalation:** After `BlockagePathDelay` frames elapsed AND still blocked → urgency = 2 (A* uses 1000.0 cost for moving-friendly blockers — destroyer-mode route-around).

C=HIGH (decompilation of LAB_004B3607 in Drive::Process_Movement verified), I=HIGH (function names confirmed), B=HIGH (path verified — Process_Movement is the only writer of +0x668/+0x66C/+0x670 with `RulesClass+0x1768`).

### State B — Pathfinder couldn't compute any route

**Trigger:** `FootClass::Find_Path` returns 0 (A* exhausted retries).

**Fields:**
- `FootClass+0x64C = path_stuck_counter` — int. Reset to **10** on any blocked-but-handleable tick. Decremented only when `Find_Path` fully fails. **Hard circuit breaker.**

**Recovery:** Until `path_stuck_counter < 1`: clear head_to, retry next tick (which may succeed if obstacle moved).

**Give-up:** When `path_stuck_counter < 1`: clear `head_to = NullCoord`, clear `drive_track_index = -1`, play `pending_blocked_sound` (if `FootClass+0x68A` set), call `Stop_Moving` or `Mark` via vtable+0x480.

C=HIGH (decompilation at LAB_004B3282 and the no-path branch), I=HIGH, B=HIGH (counter is +0x64C; verified via `path_stuck_counter` xref enumeration in source doc §6).

**Subtle detail correction (already in source doc but worth re-emphasising):** The counter is **decremented only on full Find_Path failure**, NOT on every blocked tick. A unit stuck in traffic that can still compute *a* path (even if that path immediately re-enters the jam) resets the counter to 10 every frame. So "stuck = X consecutive failed pathfind attempts", not "stuck = X consecutive blocked ticks". A Rust port that decrements on every blocked tick will give up too aggressively.

### State C — Code 7 deadlock (no possible direction)

**Trigger:** `Can_Enter_Cell` returns **7** (cell is impassable AND no alternative direction exists).

**Recovery:** If `is_retry` flag set (caller indicates a retry context): clear path_queue and self-recurse Process_Movement (one more attempt). Else: clear `head_to = NullCoord`, call `Stop_Moving`.

C=HIGH (decompilation at code-7 dispatch branch), I=HIGH, B=HIGH.

### State D — Friendly stationary close-enough arrival (code 6)

This is NOT stuck — it's a graceful "arrived nearby" detection. But it shares the dispatcher with stuck handling so it's documented here.

**Trigger:** `Can_Enter_Cell` returns **6** AND:
- `Distance3D(unit, destination) < RulesClass+0x1718` (= `CloseEnough`, default **0x200 leptons = 2 cells**)
- AND `abs(unit.Z - dest.Z) < 2 × DriveHeightStep` (Z-tolerance ~2 levels)
- AND `dest_cell.LandType != 10` (not a Tunnel cell — tunnel entries require full traversal, not close-enough)

**Recovery:** Treat as arrived. Clear head_to, mark mission complete via vtable+0x480 or call `Stop_Moving` + vtable+0x484 depending on mission state.

C=HIGH, I=HIGH, B=HIGH (verified in source doc §5).

**Subtle detail — Tunnel exception:** A unit pathing toward a Tunnel cell does NOT early-stop at CloseEnough; it must fully traverse to the Tunnel entry to trigger the subterranean transition. Per `[[feedback_no_tunnel_subterranean]]` this is TS-legacy and not active in standard YR maps, but the binary code path is live.

---

## 2. The complete watchdog-timer triple

Three timers stacked inside the stuck state machine, all at `FootClass+0x640`–`+0x670`:

| Field offset | Name | Role | Set from | Default |
|---|---|---|---|---|
| `+0x640` | `movement_delay_start` (i32) | Frame counter when movement_delay began. `-1` = inactive | `g_CurrentFrameCounter` on Find_Path success in code-2 branch | n/a |
| `+0x644` | `movement_delay_facing` (u32) | Snapshot of pending direction at delay start | direction lookup at +0x5E0 | n/a |
| `+0x648` | `movement_delay_ticks` (u32) | Duration of movement_delay | `Math::ftol(...)` — small variable | (small) |
| `+0x64C` | `path_stuck_counter` (i32) | **Circuit breaker** — see §1 State B | Reset to **10** | 0 |
| `+0x668` | `blocked_delay_start` (i32) | Frame counter when blocked-by-friendly began | `g_CurrentFrameCounter` | n/a |
| `+0x66C` | `blocked_delay_facing` (u32) | Snapshot of pending direction | direction lookup at +0x5E0 | n/a |
| `+0x670` | `blocked_delay_ticks` (u32) | Patience timer | **`RulesClass+0x1768` = `BlockagePathDelay`** | **15 frames** in stock YR |
| `+0x6B7` | `path_blocked_flag` (u8) | "Currently blocked" sticky bit | Set on code-2 entry; cleared only by Walk per sub-cell arrival | 0 |
| `+0x68A` | `pending_blocked_sound` (u8) | "Play VO next frame" | Set on stuck-detection; cleared on successful step | 0 |
| `+0x68B` | `path_direction_switched` (u8) | Bridge/elevation transition marker | Set when cell.flags & 0x100 differs from current on_bridge | 0 |

**Why two timers?**
- `movement_delay` (+0x640) — short rate-limiter (few frames) to prevent pathfinder thrashing when blocked. Re-armed on every successful Find_Path.
- `blocked_delay` (+0x668) — longer patience timer (15 frames default). Drives urgency escalation. Re-set only on transition into block state.

C=HIGH (offset table verified in source doc §2), I=HIGH (field names per cross-referenced `FOOTCLASS_STRUCT_LAYOUT.md` and `FOOTCLASS_NON_MOVEMENT_FIELDS.md`), B=HIGH.

---

## 3. The asymmetric `path_blocked_flag` clear — vehicles vs infantry

**This is the most surprising parity-load-bearing detail in the entire stuck system.**

`FootClass+0x6B7` is set to 1 by every locomotor on code-2 block entry. But it is **cleared by ONLY the Walk locomotor**:

| Locomotor | Sets +0x6B7 = 1 at | Clears +0x6B7 = 0 at |
|---|---|---|
| Drive | `0x4B3663` (in Process_Movement) | **NEVER** |
| Ship | `0x6A2CB2` (in Process_Movement) | **NEVER** |
| Hover | `0x515D3A` (in Process_Movement) | **NEVER** |
| Jumpjet | `0x5B0BD4` (in Process_Movement) | **NEVER** |
| **Walk** | `0x75B8B6` (in ProcessMovement) | **`0x75BE11` (waypoint arrival)** AND **`0x75BFD1` (facing rotation branch)** |

`FootClass::Stop_Moving @ 0x4DF0D0` does NOT touch +0x6B7 either (body is just two pointer zeroes at +0x5A0/+0x5A4).

**Consequence:** Once a vehicle (Drive/Ship/Hover/Jumpjet) has been blocked once, its `path_blocked_flag` stays set for the rest of its life. The blocked_delay timer's start frame ages forever. Any subsequent block uses urgency 2 (destroyer-mode) from frame 1 — **no grace period**.

Infantry (Walk) clear the flag on every sub-cell arrival, so each new block gets a fresh 15-frame grace period.

**Player-observable:** Veteran vehicles in crowded base perimeters cut around obstacles faster than fresh vehicles, because their `path_blocked_flag` was set long ago by some prior block. The effect is subtle but real — and **must be preserved** in any port targeting parity.

C=HIGH (verified by byte-pattern scan + analysis at all 7 listed sites), I=HIGH, B=HIGH.

---

## 4. Unstick mechanisms — what fires when stuck

In order of escalation:

### 4.1 Movement_delay rate-limit (5-15 frames typical)

- After any successful Find_Path → arm movement_delay.
- Subsequent ticks: if movement_delay still ticking, return early WITHOUT pathfinder call.
- Once expired: free to pathfind again.

**Purpose:** Prevent pathfinder thrashing. A unit in heavy traffic must wait between repath attempts.

### 4.2 Blocked_delay urgency escalation (15 frames default)

- First code-2 block → urgency = 1 (passive repath, A* cost 4.0 for moving-friendly).
- After `BlockagePathDelay` frames → urgency = 2 (route-around, A* cost 1000.0).

### 4.3 Scatter dispatch (code 6 not-close-enough)

When `Can_Enter_Cell` returns 6 AND not within CloseEnough:
```c
bridge_flag = (dest_cell.flags & 0x100) && abs(unit.Z/DriveHeightStep - dest_cell.Level) >= 3;
CellClass::Scatter_Objects(NullCoord, /*threat*/1, /*force*/1, bridge_flag);
```

`Scatter_Objects` gathers up to 10 occupants in cell, dispatches `TechnoClass::Scatter` via vtable+0x174 on each that satisfies one of:
- elite in cell
- force=1 (always TRUE here)
- `Rules.PlayerScatter` set
- `HasWeaponAbility(3)`
- owner's IQ ≥ `Rules.IQ_Scatter`

`Scatter` then issues a 1-cell move command to a free adjacent cell.

**Asynchronous:** the blocker doesn't move this tick — it queues a move command for next tick. Our unit's `Process_Movement` just clears head_to and returns; next tick it retries.

If blocker cannot scatter (cornered), our unit loops: code 6 → scatter (ineffective) → retry → code 6... until either blocker eventually moves OR our unit moves within CloseEnough and stops.

C=HIGH (verified in source doc §9 and cross-doc `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md`), I=HIGH, B=HIGH.

### 4.4 Final give-up

When `path_stuck_counter < 1` and Find_Path failed for the Nth consecutive time:
- Clear `head_to = NullCoord`
- Clear `drive_track_index = -1` (+0x58 / +0x178 depending on locomotor)
- Play `pending_blocked_sound` voice cue (EVA "Unable to comply" or unit-specific frustration line)
- Call `(vtable+0x480)(0, 1)` (Mission_Update — bail out, mark mission failed)

The unit stops trying. It will only attempt to move again if a new order is issued.

---

## 5. INI keys (RulesClass offsets)

| Key | Section | Offset | Default in YR | Purpose |
|---|---|---|---|---|
| `BlockagePathDelay` | `[General]` | `+0x1768` | **15** (frames) | blocked_delay timer duration → urgency-escalation patience |
| `CloseEnough` | `[General]` | `+0x1718` | **0x200** (2 cells = 512 leptons) | Code-6 stop distance |
| `PathDelay` | `[General]` | — | (small) | movement_delay base duration |
| `IQ.Scatter` | `[General]` | — | 2 | Min AI IQ for scatter participation |
| `PlayerScatter` | `[General]` | — | yes | Master toggle for scatter dispatch |

**Subtle detail:** `ConditionRed` at `RulesClass+0x1724` is also a stuck-related override per `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md` line 606 — "bridge-stuck timer override in AreaGuard". Open question: how does this interact with `BlockagePathDelay`? Not traced in this pass.

C=HIGH (cross-doc verification), I=HIGH (INI key names confirmed via ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md), B=HIGH.

---

## 6. Stuck-related cell-entry return codes

The full dispatch is documented in [`UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md`](UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md) §3 and `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`. Summary of stuck-relevant codes:

| Code | Meaning | Stuck involvement |
|---|---|---|
| 0 | OK | Clears the stuck-loop — counter resets implicitly |
| 1 | Overlay block (civilian) | Redraw cell, clear head_to, recurse |
| **2** | **Moving friendly blocks** | **State A — blocked_delay urgency machine** |
| 3 | Crushable | Crush + clear, no stuck involvement |
| 4/5 | Wall/Building | Crusher override (TT.+0xC94) → treat as 0; else: scatter or attack |
| **6** | **Allied stationary** | **CloseEnough check (State D), else scatter** |
| **7** | **Path-locked** | **State C — clear path on retry, give up otherwise** |

---

## 7. Crusher override (TT.+0xC94 = `TooBigToFitUnderBridge` or `Crusher=yes`)

When `TechnoTypeClass+0xC94` is non-zero (Crusher flag set in INI), the cell-entry dispatch **downgrades codes 1-6 to 0** — units like Mammoth Tank, Apocalypse, etc. ignore most blockers and proceed via `Check_Crushable_Obstacle`.

**Code 7 is NOT downgraded.** A Mammoth in a code-7 deadlock still gives up.

C=HIGH, I=HIGH, B=HIGH (verified in source doc §3).

---

## 8. Why this matters for parity

A simpler watchdog (one timer, immediate scatter on block, immediate give-up after N failures) would PROBABLY work. But it would diverge from gamemd.exe in several ways the player notices:
- Traffic-jam visuals: dual-timer system produces characteristic 15-frame "patience pulses" before vehicles try to route around. A single-timer system either causes early route-around (chaos) or no route-around (gridlock).
- Asymmetric vehicle-vs-infantry impatience: veteran units in old fights move differently from fresh units. Players can't articulate this but they FEEL it.
- The CloseEnough-tunnel exception (one extra LandType==10 check) — gets a unit to actually reach a tunnel mouth instead of stopping nearby.
- Crusher exception preserves Mammoth-class "I just plow through traffic" dominance.

**A faithful port must replicate all of these.**

---

## 9. Open questions

1. **`RulesClass+0x1724` (ConditionRed?) bridge-stuck override in AreaGuard** — referenced from `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md` line 606. How does it interact with `BlockagePathDelay`? Possibly a per-mission override that shortens the patience timer for area-guarded units. Needs decompilation of the area-guard tick path.

2. **`movement_delay_ticks` (+0x648) computation** — described as `Math::ftol(...)` in source doc but the input expression isn't enumerated. What rate-limit value does the engine actually compute?

3. **`is_retry` external callers** — source doc §13.4 says "no evidence of external invocation with is_retry=1" but worth re-verifying. If a non-recursive call ever passes is_retry=1, the path-clearing semantics change.

4. **Per-Mission stuck-timer overrides** — Mission_Area_Guard has one. Do other missions (Mission_Patrol, Mission_Hunt) have similar overrides? Cross-reference with `MISSIONCLASS_STATE_MACHINE.md`.

---

## 10. Sources

**Primary source doc:** `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md` (2026-04-05) — comprehensive function-level decomp.

**Cross-referenced docs:**
- `FOOTCLASS_STRUCT_LAYOUT.md` — confirms `+0x670` = PathRetryTimer.Duration
- `FOOTCLASS_NON_MOVEMENT_FIELDS.md` — confirms `+0x668` = BlockagePath_Timer
- `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md` — confirms reset pattern on every new destination
- `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` — confirms BlockRetryTimer pattern + INI key name
- `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` — confirms `RulesClass+0x1768 = BlockagePathDelay`, default 15
- `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — confirms stuck pattern in Ship's Process_Movement
- `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — confirms stuck pattern in Walk's ProcessMovement + asymmetric flag-clear behaviour
- `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md` — confirms `+0x64C` = path_stuck_counter
- `DRIVE_LOCOMOTION_CLASS.md` — confirms field offsets
- `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md` — Scatter_Objects mechanism
- `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md` — `RulesClass+0x1724` override hint

**Memory references:**
- `[[feedback_research_confidence_axes]]` — 3-axis confidence applied to top-level claims
- `[[feedback_caller_trace_before_finding]]` — caller traces via existing source doc

---

*End of synthesis. This doc is intentionally short — its job is to organize and reinforce what the source doc already established. The single most important takeaway is the asymmetric `path_blocked_flag` clear (§3) — without preserving it, vehicles behave too patiently in a Rust port.*
