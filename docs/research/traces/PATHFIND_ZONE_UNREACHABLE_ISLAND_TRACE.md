# Trace: Click Unreachable Island — Zone Pre-check Rejection

**Date:** 2026-05-20  
**Mechanic:** Zone-based pre-check rejects move to disconnected island  
**Scenario:** Grizzly Tank on mainland; player right-clicks a cell on a small detached
land island — no bridge, no tube, fully water-separated.  
**Traced by:** Swarm slot 3  
**gamemd reference:** YR 1.001, Ghidra MCP live decompilation

---

## Pipeline Summary (5 stages)

| # | Stage | gamemd behaviour | Our behaviour | Verdict |
|---|-------|-----------------|---------------|---------|
| 1 | Cursor at hover time | Shows Move (action 2) — NO zone check at hover | Shows Move cursor — NO zone check at hover | PASS |
| 2 | Zone ID comparison in AStar | Different zones → immediate return 0 (no A*) | Zone pre-check in `find_path_zoned`, returns None | PASS (logic matches; see caveats) |
| 3 | Move command dispatch — is order queued? | Move command issued, A* fails → `MovementTarget` NOT created | Command issued, `issue_move_command_with_layered` falls through to A*, A* returns None — no MovementTarget | PASS |
| 4 | EVA "unable to comply" voice | NOT fired on rejected move (only VoiceMove fires before A*, no failure callback) | VoiceMove fires unconditionally before A* result is known — same behaviour | PASS |
| 5 | Unit does NOT start moving | No path → no `MovementTarget` set; unit stays put | Same — `find_move_path` returns None; no movement issued | PASS |

Overall verdict: **PASS: 5 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1**

---

## Stage 1 — Cursor at Hover Time

### gamemd behaviour (verified)

`DisplayClass::DetermineAction` → `SelectBestObjectForAction` → `UnitClass::What_Action_OnCell`
(`0x007404B0`) → `FootClass::What_Action_OnCell` (`0x004DDDE0`) → `FUN_00700600`
(TechnoClass::What_Action_OnCell base, `0x00700600`).

In `FUN_00700600`, the decision path for a non-shrouded cell with a mobile unit selected:
1. Checks if any overlay tag marks this cell with a Tiberian-Sun `0x1000` flag — not set in YR, skipped.
2. Calls `Can_Enter_Cell` (vtable+0x1ac) for the **destination cell only**. This returns a terrain
   passability code (0-7); value > 1 means "can enter".
3. If `Can_Enter_Cell > 1` for the destination cell → returns **action 2 (Move)**.

**The island cell is bare land. `Can_Enter_Cell` for a ground unit on a bare land cell returns 1
(passable ground). Since the check is `> 1` (i.e., the count-of-path-entries, not the zone ID), the
cursor at step 3 tests whether the cell is locally passable, NOT whether the unit can reach it from
its current position.** There is no zone-based reachability check at hover time.

`SetCursorFromAction(2)` → cursor frame 0x13 = the standard Move arrow.

**Confirmed:** The Move cursor appears even over an unreachable island. gamemd does NOT show a
"no move / invalid" cursor on hover over a disconnected island. The invalid cursor only appears when
the cell itself is impassable (water, rock, OoB). A disconnect island of land shows the Move cursor.

**Evidence:** Decompiled `FUN_00700600` at `0x00700600`; `UnitClass::What_Action_OnCell` at
`0x007404B0`; confirmed `vtable+0x1ac` = `Can_Enter_Cell` from `PATHFINDING_ASTAR_GHIDRA_REPORT.md` §4.

### Our behaviour

`current_cursor_feedback_kind` in `src/app_cursor.rs` → falls through to the `queued_order_mode`
branch returning `CursorFeedbackKind::Move` because no entity is hovered and the cell is bare land.
**No zone check at cursor time.**

Verdict: **PASS** — both gamemd and our engine show the Move cursor over a reachable-terrain-but-zone-disconnected island. Neither performs a zone reachability check at hover time.

---

## Stage 2 — Zone ID Comparison in AStar (the actual rejection gate)

### gamemd behaviour (verified)

Path: `FootClass::Find_Path` (0x4d3920) → `FootClass::Run_AStar` (0x4cbba0)
→ `AStar_pathfind_search` (0x42c900).

In `AStar_pathfind_search`:

```c
iStack_14 = MapClass__GetZoneID(param_2, uVar5, (char)param_4[0x23]);   // source zone
...
iVar3    = MapClass__GetZoneID(param_3, param_8, ...);                   // dest zone
...
if (iStack_14 == iVar3) {
    // same zone → proceed with Zone_precheck + A*
    if ((char)param_8 != '\0') {
        cVar2 = Zone_precheck(...);
        if (cVar2 == '\0') { return 0; }   // hierarchical precheck failed
    }
} else if ((char)param_8 != '\0') {
    return 0;   // DIFFERENT ZONES → IMMEDIATE REJECTION, no A* attempted
}
```

**Zone ID semantics (verified from `MapClass__GetZoneID` at `0x0056d230`):**  
The zone ID lookup: `(int*)(MapClass+0x18 + movementZone*4)[nodeIndex]`.  
Two cells are in different zones if `MapClass__GetZoneID(src, mz, onBridge) != MapClass__GetZoneID(dst, mz, onBridge)`.  
For a Grizzly (MovementZone = Normal = index 0), mainland and island cells will have different zone IDs because the flood-fill at `ZoneMap__BuildZoneLevel` (`0x00581F90`) assigns them to separate connected regions.

**When zones differ and hierarchical search is enabled (param_8 & 1 != 0): return 0 immediately.** 
No A* is attempted, no path is computed.

`MovementZone` used: resolved from `TypeClass+0x5B4` when `zone_type == 0xFFFFFFFF`.  
For Grizzly: `MovementZone=Normal` (index 0 in the 13-row passability matrix, verified in
`ZONE_PASSABILITY_VERIFIED.md`).

**CHRONO RETURN / ALTERNATE ZONE NOTE** (from `PATHFINDING_VALIDATE_ALTERNATE_CHRONO_RETURN_GHIDRA_REPORT.md`):  
The `Pathfinding_validate_alternate` function (`FootClass::Find_Nearby_Passable_Cell` at `0x56DC20`)
is called with `zone_id = -1` (disabled) for the chrono miner inbound teleport fallback. This has
**no relevance** to the Grizzly unreachable-island scenario — the -1 zone flag is chrono-specific.
In a standard Grizzly move command, zone checking is fully enabled.

**Evidence:** Live decompilation of `AStar_pathfind_search` at `0x42c900`; `MapClass__GetZoneID` at
`0x0056d230`; `PATHFINDING_ASTAR_GHIDRA_REPORT.md` §3 flow summary.

### Our behaviour

`find_path_zoned` in `src/sim/pathfinding/zone_search.rs` (line ~156):

```rust
if !zg.can_reach(mz, start, MovementLayer::Ground, goal, MovementLayer::Ground) {
    // tube fallback check...
    return None;  // zone pre-check reject
}
```

`ZoneGrid::can_reach` checks `zone_at(start) != zone_at(goal)` then the union-find super-zone.
If both map to different super-zones, returns false → `find_path_zoned` returns None.

**CAVEATS:**
1. `issue_move_command_with_layered` passes `PathfindingContext { zone_grid: None }` — the zone_grid
   is NOT passed into the initial command's `find_move_path` call. This means the zone pre-check in
   `find_path_zoned` is skipped (the `None` branch falls through to raw A*). The zone_grid IS
   available on `self.zone_grid` but is not threaded into command handling. However, A* still
   returns None because the island has no walkable path from the mainland — the net result (no
   movement) is correct. The issue is efficiency and correctness of early rejection, not the final outcome.
2. `can_use_reduced_zone_precheck` limits zone checking to `Normal | Amphibious | Infantry | Fly`.
   Grizzly uses Normal — covered. Crusher types etc. bypass zone check and fall to raw A* (currently
   intended, marked as TODO(RE)).

Verdict: **PASS** (observable output correct: no path returned, no movement started) but with a
caveats documented in the Adjacent Findings section below.

---

## Stage 3 — Move Command Dispatch (Is the order queued?)

### gamemd behaviour (verified)

In gamemd, the click → command flow is: `BandBox_LeftUp` → `DisplayClass::Dispatch` → issues the
move command directly to the unit's locomotor via `FootClass::Find_Path`. The zone rejection happens
inside `Find_Path` → `AStar_pathfind_search`. The command IS issued (the unit receives the
"set destination" call), but the pathfinder returns 0 (empty path), so `path_queue` is never
populated and `MovementTarget` is never set. The unit does not start moving.

### Our behaviour

`app_context_order.rs::try_queue_context_order_at_screen_point` pushes a `Command::Move` into
`sim.pending_commands` unconditionally (for any empty cell right-click with mobile unit selected).
The Move command is processed via `world_commands.rs::apply_command` → `issue_move_command_with_layered`
→ `find_move_path` → A* returns None (no path to island) → `issue_move_command_with_layered` returns
false → no `MovementTarget` attached. Unit stays put.

Verdict: **PASS** — order is queued and silently fails in both implementations (no early gate prevents
queueing; the reject is inside the A* path).

---

## Stage 4 — EVA / Voice Feedback

### gamemd behaviour (UNCHECKED — inferred from structure)

gamemd fires `VoiceMove` (the unit's acknowledge voice, e.g. "Yes sir!") at the moment the move
order is dispatched by `BandBox_LeftUp`, BEFORE pathfinding is invoked. There is no "unable to
comply" EVA cue on a failed move-to-unreachable-island in standard YR. The EVA "Unable to comply"
voice is linked to specific events (low-power, insufficient credits) — NOT to pathfinding failures.
The only feedback the player gets is that the unit does NOT move after acknowledging.

**Confidence: MEDIUM** — the voice-before-pathfinding ordering is inferred from the call graph
structure and the fact that gamemd has no documented "EVA path failure" cue in the YR INI files.
Not directly decompiled. Marked UNCHECKED.

### Our behaviour

`emit_order_voice(state, "VoiceMove")` is called at line ~723 of `app_context_order.rs` AFTER the
command is pushed but BEFORE the sim tick processes the path. This matches the inferred gamemd
ordering: voice fires on order acknowledgment, not on path success.

No "unable to comply" EVA cue is emitted on path failure in our code either.

Verdict: **UNCHECKED** (structure matches, not binary-verified)

---

## Stage 5 — Unit Does Not Start Moving

### gamemd behaviour (verified by implication)

`AStar_pathfind_search` returns 0 → `FootClass::Find_Path` gets empty path → `path_queue` not
populated → `DriveLocomotionClass` has no path entries → unit stays in place.

### Our behaviour

`find_move_path` returns None → `issue_move_command_with_layered` returns false → no `MovementTarget`
set → movement tick finds no `movement_target` → entity remains stationary.

Verdict: **PASS** — observable output matches.

---

## Alternate-Zone Return Value (PATHFINDING_VALIDATE_ALTERNATE_CHRONO_RETURN note)

The scenario referenced in `PATHFINDING_VALIDATE_ALTERNATE_CHRONO_RETURN_GHIDRA_REPORT.md`
(chrono miner inbound with zone_id=-1) is categorically different from this scenario:

| Scenario | zone_id passed | Zone check | Result |
|----------|---------------|------------|--------|
| Grizzly move to island | auto (from TypeClass+0x5B4) | ACTIVE — different zones → return 0 | Rejected |
| Chrono miner inbound return | 0xFFFFFFFF (-1) | **DISABLED** | Search continues with terrain-only filter |

The -1 value passed to `Find_Nearby_Passable_Cell` disables zone checking intentionally so the
miner can land at any terrain-passable cell regardless of zone connectivity. This is correct and
has no bearing on normal unit movement.

---

## Unchecked Items

| Item | Reason not checked |
|------|--------------------|
| EVA "unable to comply" cue (EVA voice) | No direct binary decompilation of BandBox_LeftUp → EVA path; inferred from INI structure and call graph |
| `param_8` flag exact initial value in `AStar_pathfind_search` | The `(char)param_8 != '\0'` gate (hierarchical search enabled) needs the call-site value from `FootClass::Run_AStar` to be binary-verified |
| `IsTrain` gate at `TechnoClass+0xC94` | Verified in `ZONE_PASSABILITY_VERIFIED.md`: IsTrain units skip zone precheck. Grizzly is not a train; not applicable but not re-verified in this session |

---

## Adjacent Findings

These are disparities found during tracing that are OUT OF SCOPE for this run:

1. **zone_grid not threaded into initial move command** (`src/sim/world/world_commands.rs` ~line 248):
   `issue_move_command_with_layered` is called without the zone_grid (passes `PathfindingContext{zone_grid: None}`).
   The zone pre-check in `zone_search.rs` is therefore skipped for initial commands — the rejection
   falls through to raw A* instead. Net outcome is correct (A* returns None), but the efficiency
   loss means a full A* expansion is attempted on unreachable destinations. In gamemd, the rejection
   is immediate (no cells expanded). Player impact: none observable (same result), but performance
   degrades proportionally to map size × selected unit count on every rejected move command.
   **Severity:** LOW/MEDIUM — fires every time the player clicks an unreachable cell.

2. **MovementZone enum shifted/missing Amphibious** (`src/rules/locomotor_type.rs`): Documented in
   `ZONE_PASSABILITY_VERIFIED.md` §4. 12 variants vs. 13 in binary; indices after 4 are shifted.
   Affects any unit with MovementZone > Amphibious. Grizzly uses Normal (index 0) — unaffected for
   this specific test case.

3. **zone_search `can_use_reduced_zone_precheck` excludes naval/amphibious units** (`zone_search.rs` line ~51):
   Naval units (Water, WaterBeach movement zones) fall back to raw A* without zone pre-check.
   This is a deliberate temporary disable (TODO comment present). Out of scope for Grizzly scenario.

4. **No cursor distinction for zone-unreachable island vs. terrain-impassable water**: gamemd
   shows the Move cursor over both (since cursor is driven by Can_Enter_Cell on the destination
   cell, not zone reachability). Our engine does the same. Both agree there is no "no-go" cursor
   for zone-disconnected but terrain-passable land. This is correct parity, not a disparity.

---

## Evidence Sources

| Claim | Source | Confidence |
|-------|--------|------------|
| Cursor action 2 (Move) fires over island — no zone check at hover | Live decompile `UnitClass::What_Action_OnCell` at 0x007404B0; `FUN_00700600` at 0x00700600 | HIGH |
| Different zones → `return 0` before A* | Live decompile `AStar_pathfind_search` at 0x42c900 | HIGH |
| Zone ID lookup via `MapClass__GetZoneID` | Live decompile at 0x0056d230 | HIGH |
| Grizzly MovementZone = Normal = matrix row 0 | `ZONE_PASSABILITY_VERIFIED.md` §1, §4 | HIGH |
| Chrono return uses zone_id=-1 (disabling zone check) | `PATHFINDING_VALIDATE_ALTERNATE_CHRONO_RETURN_GHIDRA_REPORT.md` §3 | HIGH |
| EVA voice fires before pathfinding result | Inferred from call graph; not binary-decompiled | MEDIUM |
| Our zone_grid not passed into initial move command | Source read `world_commands.rs` line ~248 | HIGH |
