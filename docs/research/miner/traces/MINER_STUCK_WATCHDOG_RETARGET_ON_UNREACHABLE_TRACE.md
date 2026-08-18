# Miner Stuck Watchdog / Re-target on Unreachable Ore Cell

**Scenario:** Chrono Miner with `target_ore_cell = (92,187)` issued `Set_Destination((92,187))`
N ticks ago. The miner has not moved — either the path is blocked, the cell is occupied by
another unit, or it is flat-out unreachable. Does gamemd un-stick the miner?

**Date:** 2026-05-20
**Sources verified:** `UnitClass::Mission_Harvest` decompiled at `0x0073E5E0` (this session);
`DriveLocomotionClass::Process_Movement` at `0x4B2630` (this session);
`DriveLocomotionClass::Process` at `0x4B0500` (DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md);
`FootClass::CanReachDestination` at `0x4D3810` (this session);
existing docs: MISSION_HARVEST_GHIDRA_REPORT.md, DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md.

> **Audit status 2026-05-25:** RED/stale for implementation guidance. Newer
> source audits and `0x0073E5E0` rechecks contradict this trace's core
> state-0/in-transit loop framing: current Rust already has a `MoveToOre`
> per-tick rescan path, and W10 conflates blocked-delay urgency with
> `SetMission(0)` effects that require narrower verification. Do not implement
> a miner watchdog fix from this trace without using the newer blocked-delay and
> ore-acquisition docs.
>
> **Re-verified 2026-07-12:** confirmed directly — `handle_move_to_ore` in
> `src/sim/miner/miner_system.rs` (currently lines 460-522) now runs a per-tick rescan
> (`search_local_ore` call at lines 503-514, explicitly commented as modelling gamemd's
> Mission_Harvest state-0 rescan) and retargets/clears movement when a different cell is
> selected. This directly contradicts F1/F2 below and Top-5 items #1/#2 ("MoveToOre never
> re-scans" / "re-issues same blocked cell forever") as currently worded — those sections
> describe an already-superseded implementation. Left unedited below per swarm scope (Rust
> narrative, not a binary-claim correction); flagged here with exact current line numbers so
> the staleness this banner already asserts is verifiable at a glance.
>
> **Corrected 2026-07-18:** a deeper independent binary check this session found the doc's
> gamemd-side thesis was ALSO wrong on its own terms, separate from the Rust-side staleness
> above: `FootClass::Search_For_Tiberium_And_Move` @ 0x004dcfe0 was never itself
> decompiled/disassembled in the 2026-05-20 or 2026-07-12 passes. It is gated on
> `has_destination` (offset +0x5a4) and is a complete no-op — no scan, no `Set_Destination` —
> whenever a destination is already assigned; state 1 (Harvest) is entered only when the unit
> is discovered ALREADY sitting on the found ore cell, not "when scan+move succeeds and the
> unit begins moving" as W1/W3/F1/Watchdog-point-1 previously claimed. See the inline
> corrections in the Stage Table (W1, W3, W7) and the F1/F2/Watchdog-Summary addenda below,
> all verified via `disassemble_function 0x004dcfe0` this session. This does not change the
> Rust-staleness verdict above (still do not implement from this trace) but replaces the
> "no external watchdog needed, state 0 rescans every tick" mental model with: the rescan is
> entirely dependent on an external destination-clear (locomotor abort or arrival), matching
> the W4/W5/W6 mechanism already corrected 2026-07-12.
> Rust line numbers have also drifted further since 2026-07-12 (`handle_move_to_ore` is now
> at miner_system.rs:463, `search_local_ore` call at line 510) — not re-edited, same
> out-of-scope rationale as the 2026-07-12 banner.

---

## Stage Table

| # | Check | gamemd behavior | Our behavior | Verdict |
|---|-------|----------------|--------------|---------|
| W1 | State 0 re-scan every tick | State 0 calls `Search_For_Tiberium_And_Move` every tick, but the wrapper itself gates on `has_destination` (corrected 2026-07-18: was "every tick regardless of whether unit already has a destination" — `FootClass::Search_For_Tiberium_And_Move` @ 0x004dcfe0 opens with `MOV EAX,[ESI+0x5a4] / TEST EAX,EAX / JNZ 0x004dd08c` on `this[0x169]` (has_destination) and returns `AL=0` immediately with NO scan and NO `Set_Destination` call when a destination is already set; verified via disassemble_function 0x004dcfe0 — INFERENCE_HARDENED) | `handle_search_ore` is only called in `SearchOre` state; once transitioned to `MoveToOre`, scanning stops until arrival or depletion | FAIL |
| W2 | Destination cleared before re-scan (Chrono Miner) | State 0: if Teleporter=yes AND `Destination != 0` → `Set_Destination(0, force_move=1)` clears it before calling scan | No destination clear in `handle_search_ore`; but `MoveToOre` never calls scan at all | FAIL (moot — W1 is the root cause) |
| W3 | `Search_For_Tiberium_And_Move` re-issues path every tick | (corrected 2026-07-18: was "Wrapper calls Scan_For_Tiberium then Set_Destination(ore_cell) on every state-0 tick; path is refreshed even if ore cell is the same" — WRONG; disassemble_function 0x004dcfe0 shows the entry block `MOV EAX,[ESI+0x5a4] / TEST EAX,EAX / JNZ 0x004dd08c` returns immediately with no scan and no `Set_Destination` call whenever a destination is already assigned — the scan+move body only executes when `has_destination == 0`; the path is never refreshed while already assigned — INFERENCE_HARDENED) | `handle_move_to_ore` re-issues movement only when `!has_movement` (line 407). If `has_movement` stays true (locomotor stuck retrying), no re-issue occurs | FAIL |
| W4 | Locomotor unreachable-destination detect (`CanReachDestination`) | `DriveLocomotionClass::Process_Movement` (called from `Process`): `can_reach = techno->CanReachDestination(&loco.destination)` (vtable+0x2CC) — if returns 0: calls `Set_Destination(0,1)` directly (vtable+0x480) and returns 0 immediately. No `Scatter` call occurs on this branch (corrected 2026-07-12: was "calls Scatter(0,1), falls to IDLE_TAIL"; verified via decompile_function 0x4b2630 — the `CanReachDestination`-fail branch reads `(**(code**)(*piVar6+0x480))(0,1)` with no intervening Scatter call — OFFSET_RETYPED_WRONG) | Movement system has `path_stuck_counter` (init=10); on exhaustion calls `finalize_finished_entities` which sets `movement_target = None` | PASS (equivalent signal) |
| W5 | `path_stuck_counter` exhaustion → `movement_target = None` | Locomotor zone-map check (`CanReachDestination`, vtable+0x2CC) fires per-tick; on failure the locomotor calls `Set_Destination(0,1)` directly (vtable+0x480, confirmed `TechnoClass__Set_Destination` @ 0x00741970) — NOT `Scatter`. The distinct 2-arg `Scatter(0,1)`-style call used elsewhere in `Process`/`Process_Movement` lives at vtable+0x484 (Ghidra label `UnitClass__Scatter_Force` @ 0x00738970 — label unverified against RTTI this session, treat as hint only), not +0x480 (corrected 2026-07-12: was "vtable+0x480 on the linked harvester, which is TechnoClass::Scatter"; verified via read_memory on vtable__UnitClass @ 0x007f5c70 [+0x480 → 0x00741970, +0x484 → 0x00738970] and decompile_function 0x4b2630 — OFFSET_RETYPED_WRONG) | `movement_blocked.rs:144` — on `path_stuck_counter == 0`: `finished_entities.push(entity_id)`; `finalize_finished_entities` sets `movement_target = None` | PASS (signal itself is equivalent) |
| W6 | After `Scatter(0,1)` — does gamemd return to state 0 or requeue Mission_Harvest? | `TechnoClass::Scatter` with `param_2=0, param_3=1` issues `Queue_Mission(Guard, force=0)` or similar re-dispatch (the 2-arg Scatter-style method is at vtable+0x484, not +0x480 — vtable+0x480 is confirmed `Set_Destination`; corrected 2026-07-12, see W5, OFFSET_RETYPED_WRONG). The harvester goes to Guard briefly, then AI/player re-issues harvest. BUT inside Mission_Harvest state 0, the reachability check is inside the locomotor, not Mission_Harvest itself — Mission_Harvest state 0 simply calls `Search_For_Tiberium_And_Move` every tick; if the locomotor clears the destination, the next state-0 tick calls scan+move again with no destination, so a fresh ore cell is selected | After `movement_target = None`, `handle_move_to_ore` next tick sees `!has_movement` and calls `issue_move_if_idle` again — to the **same** `target_ore_cell`, not a new one | FAIL |
| W7 | Re-scan triggered on locomotor abort | When the locomotor aborts (corrected 2026-07-18: was "Scatter" — the call is `Set_Destination(0,1)` at vtable+0x480, not a Scatter call; verified via decompile_function 0x4b2630 and read_memory on vtable__UnitClass@0x7f5c70 — OFFSET_RETYPED_WRONG, same root cause as W4/W5/W6, missed by the 2026-07-12 pass), gamemd's next scheduled `Mission_Harvest` state-0 tick will call `Search_For_Tiberium_And_Move` with `destination == 0` (the locomotor cleared it), which re-enters the scan body (see corrected W1/W3) and picks a fresh ore cell | Our `handle_move_to_ore` re-issues movement to the same blocked cell forever (as long as `target_ore_cell` remains set and has ore); never re-enters `handle_search_ore` to pick a different cell | FAIL |
| W8 | Tick counter / `MissionTimer` watchdog in Mission_Harvest | No tick-counter watchdog in Mission_Harvest. No `MissionTimer` gate on re-scan. State 0 is called every tick. State 4 returns `0x69` (105 ticks) only when no ore found at all | Not applicable (no equivalent exists in gamemd) | N/A |
| W9 | `Frame & 0xF` pattern at top of Mission_Harvest | Not present. Mission_Harvest state 0 runs every tick without a frame-parity guard | Not applicable | N/A |
| W10 | DriveLocomotion `was_waiting_flag` + timer at `techno+0x388` | When `Process_Movement` returns no valid path segment, `CDTimerClass::Init` sets a blocked-delay timer at `techno+0x388`. While active, `loco+0x62` (was_waiting_flag) is set and Process returns without pathfinding. On timer expiry, `SetMission(None)` re-evaluates the unit's mission | `blocked_delay` timer in `MovementTarget` (from `BlockagePathDelay=`); on expiry urgency escalates but mission is not reset — only `movement_target = None` on stuck_counter exhaustion | UNCHECKED (mechanism is present but SetMission(None) vs movement_target=None have different downstream effects on the miner state machine) |

---

## Key Findings

### F1 — W1/W3/W6/W7: MoveToOre never re-scans; re-issues same blocked cell forever

**Root cause:** gamemd's Mission_Harvest stays in **state 0** while driving to ore. State 0 calls
`Search_For_Tiberium_And_Move` every tick. For the Chrono Miner specifically, if a destination is
already set, it is **explicitly cleared** (`Set_Destination(0, force_move=1)`) before the scan call.
This means:

1. Every tick, the locomotor re-selects the best-reachable ore cell from the current position.
2. If the previously targeted cell became unreachable (occupied, zone-split, destroyed),
   the scan finds a different cell automatically — no external watchdog needed.
3. The transition to state 1 (Harvest) only happens when `Search_For_Tiberium_And_Move` returns
   true AND the unit begins moving. State 0 ≠ "searching"; it is BOTH search AND in-transit.

> **Corrected 2026-07-18:** Points 1–3 above are WRONG as written. `FootClass::Search_For_Tiberium_And_Move`
> (0x004dcfe0) is gated on `this[0x169]` (has_destination, offset +0x5a4): `disassemble_function 0x004dcfe0`
> shows `MOV EAX,[ESI+0x5a4] / TEST EAX,EAX / JNZ 0x004dd08c` at entry — when a destination is already set,
> the function returns `AL=0` immediately with **no scan and no re-selection**. Re-selection only happens on
> a tick where `has_destination == 0` (freshly cleared — either the initial state-0 entry, the Chrono-Miner
> CLSID-gated clear, or the locomotor's `CanReachDestination`-fail `Set_Destination(0,1)` call, see W4/W5).
> "No external watchdog needed" (point 2) is backwards: without that external clear (the locomotor's
> zone-reachability check), a regular non-Chrono miner stuck against a locally-blocked-but-zone-reachable
> cell has no other path back into the scan body. Point 3 is also WRONG: `cVar1`/AL is 1 **only** when the
> unit's current position already equals the found ore cell (`CMP word ptr [ESP+0x1c],CX` /
> `CMP word ptr [ESP+0x1e],AX` both matching at 0x004dd058-0x004dd064 → `MOV AL,0x1`); when a NEW
> destination is issued via `Set_Destination` (vtable+0x480 at 0x004dd086), the function falls through to
> `XOR AL,AL` (0x004dd08d) and returns **false** — state 1 (Harvest) is entered on the tick the unit is
> discovered already sitting on ore, not on the tick movement is issued. Verified via
> `disassemble_function 0x004dcfe0` this session. ROOT_CAUSE: INFERENCE_HARDENED (the wrapper's own body
> was never decompiled/disassembled in the 2026-05-20 or 2026-07-12 passes).

Our impl splits these into `SearchOre` (scan once, pick cell) → `MoveToOre` (drive to it).
In `MoveToOre` we never re-scan. If the locomotor is stuck (path_blocked, retrying),
`has_movement` stays true, and `handle_move_to_ore` never calls `issue_move_if_idle`.
When `path_stuck_counter` eventually exhausts and `movement_target = None`, we re-issue
movement to the **same** `target_ore_cell` (line 407-415 of miner_system.rs), not a fresh scan.

**Player sees:** Chrono Miner issued to a cell that is blocked by a friendly unit parks
permanently adjacent to the blockage. In gamemd, within one stuck-counter cycle (~10 ticks
after blockage at urgency=2), the miner re-scans and drives to the next-best ore cell.

**File:line:** `src/sim/miner/miner_system.rs:337` — `handle_move_to_ore` function entirely.
No call to `handle_search_ore` or any scan inside `MoveToOre`.

**gamemd evidence:** `UnitClass::Mission_Harvest` decompiled at `0x0073E5E0`, case 0:
```
// CLSID match + has destination → clear destination
if (bVar11 [TeleportLoco match] && param_1[0x169] != 0) {
    (**(code **)(*param_1 + 0x480))(0,1);  // Set_Destination(null, force)
}
cVar1 = FootClass__Search_For_Tiberium_And_Move(TiberiumLongScan, zone);  // scan + move
if (cVar1) { state = 1; return 1; }
// no ore and no destination → state 4
// no ore but has destination → clear first-time flag, stay state 0
```

State 0 only transitions to state 1 when `cVar1 != '\0'` (scan found ore and issued movement).
Until then it stays in state 0 and calls scan+move every tick.

**Frequency:** Every match where a miner targets an ore cell that becomes blocked before
arrival. With 2–4 miners this is a regular occurrence. Each blocked miner stays stuck for
the full match duration instead of ~10 ticks (one stuck-counter cycle).

---

### F2 — W6: On stuck-counter exhaustion, re-issue goes to same cell, not fresh scan

**Root cause:** `finalize_finished_entities` sets `movement_target = None`. Next
`handle_move_to_ore` tick sees `!has_movement`, finds ore still at `target_ore_cell`,
and calls `issue_move_if_idle` to the same cell again. Loop repeats indefinitely.

**Fix needed:** On locomotor abort (stuck-counter exhaustion), transition miner state to
`SearchOre` and clear `target_ore_cell`, so the next tick picks a fresh ore cell.

**File:line:** `src/sim/miner/miner_system.rs:407-415` — `if !has_movement && let Some(grid) =
path_grid { issue_move_if_idle(...) }` — no abort check before re-issuing.

**gamemd evidence:** When the locomotor clears the destination (corrected 2026-07-18: was "via Scatter or
timer expiry" — the confirmed clear path is `Set_Destination(0,1)` at vtable+0x480 called directly from
`DriveLocomotionClass::Process_Movement`'s `CanReachDestination`-fail branch, not `Scatter`; the separate
blocked-delay-timer/`SetMission(0)` path remains UNCHECKED per W10/F3 — verified via
decompile_function 0x4b2630, OFFSET_RETYPED_WRONG),
state 0 sees `param_1[0x169] == 0` on the next scheduled `Mission_Harvest` tick and the
`Search_For_Tiberium_And_Move` call re-enters its scan body (see corrected W1/W3, F1 addendum) and picks
a new best-reachable ore cell. `target_ore_cell` is only refreshed once the destination has been cleared —
the wrapper is a complete no-op while a destination remains set (corrected 2026-07-18, INFERENCE_HARDENED).

---

### F3 — W10: `SetMission(None)` re-evaluation path not modeled

**Confidence:** UNCHECKED. The `was_waiting_flag` + timer-at-`techno+0x388` mechanism
(DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md Step 3D/3E) calls `techno->SetMission(0)` when the
drive delay timer expires. For a harvester this may reset the mission dispatch back to
Mission_Harvest case 0. Our `movement_target = None` path does not call any equivalent.

**Frequency:** Fires every time a miner's blocked_delay timer expires while still blocked —
which is every few ticks during a sustained block. Potentially a compounding difference.

---

## gamemd Watchdog Summary

gamemd has NO explicit tick-counter watchdog in Mission_Harvest. Instead, un-sticking is
inherent in the design:

1. **State 0 calls the rescan wrapper every tick, but the wrapper only re-scans when
   `has_destination == 0`.** (corrected 2026-07-18: was "State 0 re-scans every tick" —
   `FootClass::Search_For_Tiberium_And_Move` @ 0x004dcfe0 opens with a `this[0x169]`/+0x5a4
   has-destination check (`TEST EAX,EAX / JNZ 0x004dd08c`) and is a complete no-op — no scan,
   no `Set_Destination` — whenever a destination is already assigned; verified via
   disassemble_function 0x004dcfe0 — INFERENCE_HARDENED.) As long as the miner is in state 0
   (searching OR moving to ore), `Search_For_Tiberium_And_Move` is called every tick, but it
   only does work on the tick(s) where the destination has just been cleared.
2. **For Chrono Miner: destination is cleared before re-scan.** The piggybacked locomotion
   CLSID check (`0x0073E818–0x0073E82A`) detects the teleport locomotor and clears the
   destination so the scan starts fresh.
3. **Locomotor `CanReachDestination` provides a fast-path abort.** `DriveLocomotionClass::Process`
   (via `Process_Movement`) checks zone-map reachability; if unreachable, calls `Set_Destination(0,1)`
   directly (vtable+0x480) — clearing the destination itself, not a separate `Scatter` call
   (corrected 2026-07-12: was "calls Scatter(0,1) which triggers mission re-evaluation";
   verified via decompile_function 0x4b2630 — OFFSET_RETYPED_WRONG). This is still the per-tick
   stuck signal — it fires on the first tick the zone map shows the destination is unreachable.
4. **`Process_Movement` calls `Set_Destination(0,1)` on `Find_Path` failure when zone is unreachable.**
   In the decompiled `Process_Movement` (0x4b2630): if `Find_Path` returns 0 AND
   `CanReachDestination` (vtable+0x2CC) returns 0 → `(**(code**)(*techno+0x480))(0,1)`, i.e.
   `Set_Destination(0,1)`, then returns 0. This fires on every failed-path tick where zone
   connectivity confirms the destination is unreachable (corrected 2026-07-12: was
   "Scatter(0,1)" with an unverifiable "line 143-145" pseudocode-line citation; verified via
   decompile_function 0x4b2630 this session — OFFSET_RETYPED_WRONG).

**Period:** For an unreachable destination (zone-split), `Set_Destination(0,1)` fires on the
very first tick `DriveLocomotionClass::Process`/`Process_Movement` runs and the zone check
fails — typically within 1 tick of the path failure, NOT 10 (stuck-counter cycles) (corrected
2026-07-12: was "`Scatter` fires" — see point 3/4 above, OFFSET_RETYPED_WRONG). The stuck-counter is only relevant for
cells that ARE zone-reachable but have a local path blockage (e.g., surrounded by friendly
units). For flat-out unreachable cells, the zone check un-sticks in 1 tick.

---

## Verdict Tally

PASS: 2 | FAIL: 4 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

---

## Top 5 Player-Visible Failures

1. **MoveToOre never re-scans on block** — miner stuck permanently at blocked ore cell instead
   of finding next best cell within ~1 tick (zone-unreachable) or ~10 ticks (occupied).
   `src/sim/miner/miner_system.rs:337` (`handle_move_to_ore` function); gamemd evidence:
   `0x0073E5E0` case 0 — scan wrapper called every tick in state 0, but only re-scans on ticks
   where the destination has just been cleared (corrected 2026-07-18: see W1/W3/F1 addendum;
   `Search_For_Tiberium_And_Move` @ 0x004dcfe0 is a no-op while a destination is already set —
   INFERENCE_HARDENED).

2. **On stuck-counter exhaustion, re-issues same cell** — miner loops forever: stuck → abort →
   reissue same cell → stuck. gamemd re-scans for new cell.
   `src/sim/miner/miner_system.rs:407-415`; gamemd: `param_1[0x169]==0` → fresh scan next tick.

3. **Chrono Miner destination NOT cleared before re-scan** — if our MoveToOre ever did re-scan,
   it would scan with the old destination still set, whereas gamemd clears it first for
   Teleporter=yes units.
   `src/sim/miner/miner_system.rs:337`; gamemd: `0x0073E818` clears dest before scan call.

4. **Zone-unreachable cells clear the destination via `Set_Destination(0,1)` in gamemd in 1 tick**
   (corrected 2026-07-12: was "fire Scatter(0,1)" — verified via decompile_function 0x4b2630,
   the call is vtable+0x480 not +0x484 — OFFSET_RETYPED_WRONG) — our stuck-counter takes
   up to 10 urgency=2 repath failures (each after a `blockage_path_delay` grace period) — 
   potentially tens of ticks before abort vs. 1 tick in gamemd for zone-unreachable destinations.
   `src/sim/movement/movement_blocked.rs:143`; gamemd: `DriveLocomotionClass::Process` Step 3G.

5. **`SetMission(None)` call on drive-delay-timer expiry not modeled** (UNCHECKED) — gamemd's
   locomotor calls `techno->SetMission(0)` when the blocked-delay timer expires, which may
   trigger immediate mission re-evaluation (re-enter state 0). Our movement clears
   `movement_target` but does not trigger miner-state re-evaluation.
   `src/sim/movement/movement_tick.rs:1022` (`finalize_finished_entities`);
   gamemd: `DriveLocomotionClass::Process` Step 3D (0x4B08D1).

---

## Status

COMPLETE
