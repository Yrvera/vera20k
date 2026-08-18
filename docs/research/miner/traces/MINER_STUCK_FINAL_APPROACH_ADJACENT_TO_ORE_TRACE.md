# Miner-Stuck Bug — Final-Approach Issuance: Adjacent-to-Ore Cell

**Mechanic traced:** When the Chrono Miner is at cell (91,186) with target ore cell
(92,187) — diagonally adjacent (dx=1, dy=1) — what does gamemd do on the next tick
to step the miner onto (92,187)? Does it bypass FindPath or not? Does it bypass the
cell-passability check?

**Scenario:** CMIN at (91,186), `target_ore_cell=(92,187)`, in `Mission_Harvest`
State 1 (or transitioning from State 0). Adjacent diagonal.

**Date:** 2026-05-20
**Authored by:** trace-swarm slot 2
**Iron Law applied:** PASS requires literal numerical equality. Anything less is FAIL
or UNCHECKED.

> **Disputed status 2026-05-25:** This trace's conclusion that CMIN always
> warps to ore conflicts with newer ore-acquisition and drive-model docs plus
> current Rust comments. Do not implement an ore-approach warp from this trace
> alone. Required follow-up before changing code: `/re-investigate chrono miner
> ore approach teleport-vs-drive`.

---

## Summary of gamemd Behavior (Verified)

### gamemd does NOT issue a drive-path for ore approach — it teleports

`UnitClass::Mission_Harvest` State 0 calls `FootClass::Search_For_Tiberium_And_Move`
(0x4DCFE0), which calls `vtable+0x480 = TechnoClass::Set_Destination` (0x741970) with
the ore cell's `CellClass*`.

`Set_Destination` → `FootClass::Set_Destination_Internal` (0x4D94B0) → writes the
destination to `FootClass+0x5A4` (NavCom) → calls `ILocomotion::Head_To_Coord`
(vtable+0x44) on `FootClass+0x674` (the **active locomotor**).

For CMIN: the active locomotor is `TeleportLocomotionClass`. `Head_To_Coord` on
TeleportLoco arms the warp state machine (sets `IsMoving=1`). On the next tick,
`TeleportLocomotionClass::Process` fires Phase 0 of the warp sequence: the miner
teleports to the ore cell in one tick.

**There is no "adjacent-to-ore drive path" in gamemd.** The CMIN always warps to
ore cells regardless of distance — adjacent or 48 cells away. Drive locomotor is
only activated when `Set_Destination` is called with a BUILDING-containing cell
(the refinery dock approach: `FindFirstBuilding != NULL` in `Set_Destination`).

Evidence: `CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md` §Stage 2 (verified in
this trace session via decompilation of `TechnoClass::Set_Destination @ 0x741970`);
`HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md` §4.3.

### FootClass::Find_Path is never called for ore approach

`FootClass::Find_Path` (0x4D3920) is only called from
`DriveLocomotionClass::Process_Movement` (0x4B2630) when `FootClass+0x5E0 == -1`
(path queue exhausted). Because the CMIN warp never uses the Drive locomotor for
ore approach, `Find_Path` is never invoked. The warp bypasses the entire A* / path
queue / drive-track system.

Evidence: decompiled `DriveLocomotionClass::Process_Movement` comment header and
code body at `LAB_004b281c`: `FootClass__Find_Path` is only reached when
`uVar18 == 0xffffffff` (path queue front = -1) and a drive destination exists.

### Can_Enter_Cell does NOT block Tiberium for Track units

`UnitClass::Can_Enter_Cell` (0x73F0A0) ultimately checks
`g_SpeedType_LandType_Table[SpeedType + LandType*9]` (base 0x89EA40, verified in
`SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` §3.1). For CMIN:

- SpeedType = Track = col 1
- LandType = Tiberium = row 5
- Table value: `[Tiberium] Track=70%` = **0.7** (non-zero)

`Can_Enter_Cell` returns 7 (Impassable) for terrain only when speed == 0.0. Track on
Tiberium = 0.7 → **passable**. The miner enters the ore cell at 70% base speed via
the drive locomotor. No special bypass is needed; `Can_Enter_Cell` returns 0 (OK)
for a Track unit on Tiberium.

Evidence: `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` §4 (table row 5 Tiberium,
column 1 Track = 0.7); `UnitClass::Can_Enter_Cell` decompiled at 0x73F0A0 — the
`return 7` impassable branch fires only when the float from the table `== FLOAT_007e1748
(0.0)`.

---

## Stage Table

| # | Stage | gamemd behavior | Our behavior | Verdict |
|---|-------|----------------|--------------|---------|
| A-1 | Move issuance mechanism for ore approach | `Set_Destination` → TeleportLoco → Phase 0 warp (all distances, including adjacent) | `dx≤1,dy≤1` branch: `issue_direct_move` (Drive MovementTarget, 2-cell path `[start,target]`) | **FAIL** |
| A-2 | Does gamemd call `Find_Path` for adjacent ore? | No — TeleportLoco bypasses drive/path entirely | N/A (direct move also bypasses A*) | PASS (structural: neither uses FindPath for this case) |
| A-3 | Is there a special gamemd "adjacent" branch in Mission_Harvest? | No. `Search_For_Tiberium_And_Move` issues `Set_Destination` uniformly — no distance-based branch | Our `dx≤1,dy≤1` branch is an engine invention, not a gamemd mechanism | **FAIL** |
| A-4 | Path shape produced | 0-cell "path" — warp is instantaneous; no path object | 2-cell path `[start, target]` via `issue_direct_move`; path traversed over multiple ticks at `speed` lep/sec | **FAIL** |
| A-5 | `Can_Enter_Cell` on Tiberium for Track unit | Returns 0 (OK) — Track speed on Tiberium = 0.7 (non-zero) → not blocked | PASSABILITY_MATRIX[Crusher=1][Tiberium=5] = `PASS_BLOCKED (2)` — the grid marks ore cell BLOCKED | **FAIL** |
| A-6 | `bypass_grid` flag on the direct move | N/A (gamemd doesn't use the concept) | `issue_direct_move` uses `bypass_grid: false` (default) — respects path_grid walkability | FAIL (path_grid marks Tiberium as blocked, causing the direct move to be halted at the ore cell boundary) |

---

## Root Cause of Miner Stuck (Verdict)

Two independent bugs compound:

**Bug 1 (PASSABILITY_MATRIX):** `PASSABILITY_MATRIX[row 1 Crusher][col 5 Tiberium] = 2`
(BLOCKED). In gamemd, `SpeedType_LandType_Table[Track][Tiberium] = 0.7` — Tiberium
is passable for Track/Crusher units. The passability matrix incorrectly marks Tiberium
as blocked for ALL non-amphibious/non-fly movement zones. This makes the path grid
report the ore cell as unwalkable for the Chrono Miner.

**Bug 2 (Architecture mismatch):** Our engine drives to ore; gamemd warps. The
`dx≤1,dy≤1` `issue_direct_move` branch was added to work around Bug 1 (a blocked
ore cell can't be A*-pathed to), but `issue_direct_move` uses `bypass_grid: false`
by default, so the movement tick still refuses to cross the Tiberium cell because
the path grid says it's blocked. The direct move issues correctly but then the miner
stalls on the last step.

The immediate fix for the stuck bug is either:
- Set `bypass_grid: true` on the `issue_direct_move` call for ore cells (workaround,
  doesn't fix the underlying architecture), OR
- Fix `PASSABILITY_MATRIX` to mark Tiberium as passable for Crusher/Track/Foot rows
  (correct the underlying model to match gamemd)

The correct fix per gamemd is the passability matrix fix. Tiberium row should have
`PASS_OK (1)` for Crusher (row 1), Destroyer (row 2), Infantry (row 7),
InfantryDestroyer (row 8), and CrusherAll (row 12) — matching `SpeedType_LandType_Table`
where Foot=0.9, Track=0.7, Wheel=0.5 (all non-zero).

---

## Key Findings (FAIL)

### F1 — A-1/A-3/A-4: gamemd warps to ore; we drive
**Stage:** A-1, A-3, A-4 (ore approach mechanism, path shape)
**Player sees:** Chrono Miner drives slowly to ore (seconds) instead of teleporting
in 1 tick. No WarpAway animation at departure or arrival. No translucency at arrival.
**File:line:** `src/sim/miner/miner_system.rs:396-401` (the `dx<=1,dy<=1` branch)
and `src/sim/miner/miner_system.rs:407-415` (the A* fallback)
**gamemd evidence:** `TechnoClass::Set_Destination @ 0x741970` → TeleportLoco active
for ore cells (FindFirstBuilding == NULL) → `Head_To_Coord` arms warp Phase 0.
`HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md` §4.3; `CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md` §S2.

### F2 — A-5/A-6: Tiberium cells wrongly blocked in passability matrix (root cause of stuck)
**Stage:** A-5, A-6 (Can_Enter_Cell / path grid blocking ore cell)
**Player sees:** Chrono Miner stops one cell away from the ore cell and never
reaches it. The miner oscillates or idles with `movement_target` set but unable to
step onto the ore. Fires every harvest cycle (once every ~30 seconds in normal play).
**File:line:** `src/sim/pathfinding/passability.rs:122-123` — Crusher row and
Destroyer row both have `2` (BLOCKED) in column 5 (Tiberium). Should be `1` (OK)
for row 1 (Crusher), row 2 (Destroyer), row 7 (Infantry), row 8 (InfantryDestroyer),
row 12 (CrusherAll). Wheel (row 0 equivalent in original) = 0.5 still non-zero, so
Wall-and-Wheel should also be OK. See correction table below.
**gamemd evidence:** `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` §4:
`[Tiberium] Foot=0.9, Track=0.7, Wheel=0.5` — all non-zero = passable.
`UnitClass::Can_Enter_Cell @ 0x73F0A0` — returns 7 (Impassable) only when speed == 0.0.

### F3 — A-3: Unnecessary `dx<=1,dy<=1` branch in our engine
**Stage:** A-3 (special adjacent branch)
**Player sees:** Indirect — this branch is the workaround that almost works but fails
due to F2. Gamemd has no such branch; the engine comment at line 387-390 incorrectly
claims Track units cannot path onto Tiberium.
**File:line:** `src/sim/miner/miner_system.rs:387-401` — comment at 387-390 says
"passability matrix blocks Tiberium terrain for Track-type units"; this is wrong
per gamemd's `SpeedType_LandType_Table[Track][Tiberium] = 0.7`.
**gamemd evidence:** Table verified in `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` §4.

---

## Passability Matrix Correction (Required)

gamemd `SpeedType_LandType_Table[SpeedType][Tiberium=5]`:

| SpeedType | Value | Our zone row | Current matrix[row][Tib=5] | Correct |
|-----------|-------|--------------|---------------------------|---------|
| Foot | 0.9 | Row 2 (Destroyer/Track/Foot) | 2 BLOCKED | → 1 OK |
| Track | 0.7 | Row 1 (Crusher) | 2 BLOCKED | → 1 OK |
| Wheel | 0.5 | Row 0 (Normal) | 2 BLOCKED | → 1 OK |
| Hover | 0.5 | (separate) | varies | → 1 OK |
| Float | 0.0 | Row 10 (Water) | 2 BLOCKED | KEEP (Float cannot cross land) |
| Amphibious | 0.5 | Rows 3,4,5 | 2 BLOCKED | → 1 OK |

File: `src/sim/pathfinding/passability.rs:118-146`, rows 0 (Normal), 1 (Crusher),
2 (Destroyer), 3 (AmphibiousDestroyer), 4 (AmphibiousCrusher), 5 (Amphibious),
7 (Infantry), 8 (InfantryDestroyer), 12 (CrusherAll) — column 5 should be 1 (OK).

---

## Verdict Tally

PASS: 1 | FAIL: 5 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

---

## Top 5 Player-Visible Failures

1. **A-1/A-3/A-4 — Chrono Miner drives to ore instead of warping**
   Player sees: miner slowly drives to ore field (seconds) instead of teleporting
   in 1 tick. Every harvest cycle. No warp animation. No translucency.
   Code: `src/sim/miner/miner_system.rs:396-415` (handle_move_to_ore)
   gamemd: TechnoClass::Set_Destination 0x741970 → TeleportLoco → warp Phase 0
   (CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md §S2)

2. **A-5/A-6 — Passability matrix blocks Tiberium for Crusher/Track (miner stuck)**
   Player sees: Chrono Miner stops adjacent to ore cell and never reaches it. Miner
   idles permanently on that harvest cycle until reassigned.
   Code: `src/sim/pathfinding/passability.rs:122-123` — Crusher/Destroyer rows have
   col 5 (Tiberium) = 2 (BLOCKED).
   gamemd: SpeedType_LandType_Table Track/Tiberium = 0.7 → Can_Enter_Cell OK.
   (SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md §4)

3. **A-1 — Missing WarpAway animation on ore approach departure**
   Player sees: no shimmer effect when miner leaves its current cell to warp to ore.
   Every harvest cycle.
   Code: `src/sim/miner/miner_system.rs:396-415` — no anim spawn for ore approach.
   gamemd: TeleportLocomotionClass Phase 0 spawns WarpAway anim at departure cell.

4. **A-1 — Missing translucency at ore cell arrival (chrono lock timer)**
   Player sees: miner appears fully opaque at ore cell immediately. Should be 50%
   translucent for ChronoDelay ticks after warp-in.
   Code: `issue_direct_move` (drive-based) bypasses teleport visual path entirely.
   gamemd: `TechnoClass::Draw` adds flag 0x2004 when `BeingWarped(+0x271)=1`.

5. **A-3 — Wrong comment in source (incorrect claim about Track passability)**
   Player sees: indirectly — comment motivates the adjacent-branch workaround which
   silently fails due to the blocked passability. Misleads future readers.
   Code: `src/sim/miner/miner_system.rs:387-390` comment text.
   gamemd: Track/Tiberium = 0.7 passable (SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md §4).

---

## Sources

- `UnitClass::Mission_Harvest @ 0x73E5E0` — MISSION_HARVEST_GHIDRA_REPORT.md,
  HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md
- `FootClass::Search_For_Tiberium_And_Move @ 0x4DCFE0` — decompiled this session
- `TechnoClass::Set_Destination @ 0x741970` — decompiled this session
- `FootClass::Set_Destination_Internal @ 0x4D94B0` — decompiled this session
- `DriveLocomotionClass::Set_Destination @ 0x4AFD40` — decompiled this session
- `DriveLocomotionClass::Process @ 0x4B0500` — decompiled this session
- `DriveLocomotionClass::Process_Movement @ 0x4B2630` — decompiled this session
  (Find_Path called when path queue == -1, i.e., never for warp-to-ore)
- `FootClass::Find_Path @ 0x4D3920` — decompiled this session
- `UnitClass::Can_Enter_Cell @ 0x73F0A0` — decompiled this session
- `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` — table verified: Track/Tiberium=0.7
- `CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md` — §S2 confirms warp-to-ore
- `src/sim/miner/miner_system.rs:337-415` — handle_move_to_ore
- `src/sim/movement/movement_commands.rs:98-148` — issue_direct_move
- `src/sim/pathfinding/passability.rs:118-146` — PASSABILITY_MATRIX

---

## Status

COMPLETE
