# HARV Ore Scan Occupancy Filter -- Ghidra Research Report

**Address(es):** `0x004DD0A0` (`FootClass::Scan_For_Tiberium`), `0x004DCE80` (`FootClass::Is_Cell_Harvestable`), `0x0073F0A0` (`UnitClass::Can_Enter_Cell`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** War Miner ore-scan candidate filtering for occupied ore cells in `Mission_Harvest` state 0/1 scan calls, with focus on ring 0 versus rings 1+ and HARV-on-ore contention.
**Non-Scope:** Full `UnitClass::Can_Enter_Cell` priority tree, terrain-object/tree blocked ore behavior, pathfinder blocked-destination fallback, refinery return/dock behavior, Slave Miner scan policy.
**Confidence:** High for the claimed slice.
**Active in YR:** Yes. This is the live `UnitClass::Mission_Harvest` path for standard HARV (`Harvester=yes`, `Weeder=no`, `Teleporter=no`) in Yuri's Revenge.

## Working Notes

**Target question:** Does stock YR's War Miner ore scan exclude ore cells occupied by allied/enemy vehicles, and is ring 0 handled differently from rings 1+?

**Non-goals:** Do not re-investigate all movement/pathfinding, refinery docking, ore extraction, terrain-object overlays, or Slave Miner behavior. Do not edit Rust/INI files.

**Evidence needed to mark COMPLETE:** binary proof for scan ring bounds, ring-0 return handling, `Is_Cell_Harvestable -> Can_Enter_Cell` dispatch/argument order, occupied-vehicle return handling, and liveness from `UnitClass::Mission_Harvest` for HARV.

**Stop conditions:** stop after `Scan_For_Tiberium`, `Is_Cell_Harvestable`, `Search_For_Tiberium_And_Move`, and the relevant occupied-vehicle branches of `UnitClass::Can_Enter_Cell` are traced; defer unrelated `Can_Enter_Cell` terrain/bridge/tunnel branches if they do not alter occupied ore-cell filtering.

## 1. Overview

Stock YR treats the miner's current cell as a special fast path: if the War Miner is already standing on ore, `Scan_For_Tiberium` returns that cell immediately without `Is_Cell_Harvestable` or `Can_Enter_Cell`. Every outer-ring candidate goes through `Is_Cell_Harvestable`; for a HARV object that dynamic `vtable+0x1AC` call resolves to `UnitClass::Can_Enter_Cell`, and non-zero return codes reject the cell before it can become the scan winner.

For the player, this prevents a second War Miner from selecting an ore cell already occupied by another miner when the cell is in ring 1+. It does not prevent the miner from harvesting the cell it is already on.

## 2. Class Layout / Key Offsets

| Object | Offset | Meaning for this slice | Evidence |
|--------|--------|------------------------|----------|
| `UnitClass` | `+0xBC` (`param_1[0x2F]`) | `Mission_Harvest` substate; state 0 and state 1 call ore scan paths | `0x0073E5E0` |
| `UnitClass` | `+0x5A4` (`param_1[0x169]`) | destination; `Search_For_Tiberium_And_Move` does not scan if destination is nonzero | `0x004DCFE0` |
| `UnitClass` | `+0x6C4` (`param_1[0x1B1]`) | unit type pointer; HARV flags live here | `0x0073E5E0` |
| `UnitTypeClass` | `+0xCD4` | `Teleporter`; HARV is false | `0x0073E5E0`, INI `[HARV]` |
| `UnitTypeClass` | `+0xE0E` | `Harvester`; HARV is true | `0x0073E5E0`, `rulesmd.ini [HARV] Harvester=yes` |
| `UnitTypeClass` | `+0xE0F` | `Weeder`; HARV is false | `0x0073E5E0` |
| `CellClass` | `+0xE4` / `+0xE8` | object lists traversed by `UnitClass::Can_Enter_Cell`; list choice depends on stack flag from caller | `0x0073F0A0` |
| `CellClass` | `+0xEC` | land type; `5` is Tiberium/ore | `0x004DD0A0`, `0x004DCE80` |
| `CellClass` | `+0x11E` | ore density used later by `Get_Tiberium_Value`, not the occupancy gate | prior `0x00485020` reports |

## 3. Core Logic

### 3.1 Liveness From `Mission_Harvest`

`UnitClass::Mission_Harvest` at `0x0073E5E0` is the live standard HARV mission handler:

- The normal harvester path is active when `UnitTypeClass+0xE0E != 0` and `+0xE0F == 0`.
- State 0 calls `FootClass::Search_For_Tiberium_And_Move` with `RulesClass+0x177C` (`TiberiumLongScan`) converted from leptons to cells by the signed `(value + (value >> 31 & 0xFF)) >> 8` pattern.
- State 1 continuation scans call the same `Search_For_Tiberium_And_Move` for harvesters using `RulesClass+0x1778` (`TiberiumShortScan`), after a failed harvest tick and while not full.
- The scan result is used directly as the ore destination; no post-scan "ore target reservation" or "someone is heading there" filter was found in the state 0 path.

Active in YR: Yes for `[HARV]`.

### 3.2 `Search_For_Tiberium_And_Move`

`FootClass::Search_For_Tiberium_And_Move` at `0x004DCFE0`:

1. Reads current destination `this+0x5A4`.
2. If destination is nonzero, it does not call the scan function in this pass.
3. If destination is zero, dispatches `vtable+0x338`, which resolves to `FootClass::Scan_For_Tiberium` for the HARV object path.
4. If the returned cell is not the invalid sentinel and not the current cell, calls `vtable+0x480` (`Set_Destination`) on the returned `CellClass`.
5. If the returned cell is the current cell, returns success without setting a destination.

Important negative fact: no additional occupancy or reservation pass occurs after the scan result.

### 3.3 `Scan_For_Tiberium` Ring Behavior

`FootClass::Scan_For_Tiberium` at `0x004DD0A0`:

1. Gets the unit coordinates through `vtable+0x48`.
2. Converts leptons to cell coordinates with signed bias: `(coord + (coord >> 31 & 0xFF)) >> 8`.
3. Checks the center cell (`ring 0`) by calling `MapClass::Get_CellClass` and reading `CellClass+0xEC`.
4. If `LandType == 5`, writes the center cell to the return slot and returns immediately.
5. Otherwise scans rings while `ring < radius`. With default `TiberiumLongScan=48`, rings `1..47` are scanned.
6. For each ring, column offset runs from `-ring` through `+ring` inclusive.
7. For each column, four diamond-arm candidates are evaluated. Corners are visited more than once; there is no deduplication.
8. Each candidate calls `FootClass::Is_Cell_Harvestable`.
9. Only harvestable candidates call `CellClass::Get_Tiberium_Value`.
10. The best cell in the current ring is updated only when `old_value < new_value`; ties keep the first-seen candidate.
11. When any candidate in the current ring is accepted (`best_value != -1`), scanning stops before the next ring.

Ring-0 conclusion: current-cell ore bypasses playfield, shroud, zone, occupancy, and locomotor checks. That bypass is deliberate in the binary.

Outer-ring conclusion: rings 1+ are filtered through `Is_Cell_Harvestable`, so occupied vehicle cells can be rejected before ranking.

### 3.4 `Is_Cell_Harvestable -> UnitClass::Can_Enter_Cell`

`FootClass::Is_Cell_Harvestable` at `0x004DCE80` checks, in order:

1. `MapClass::Is_Cell_In_Playfield(cell)`. If false, return false.
2. Campaign shroud branch only when `g_GameMode == 0` and byte `this+0x41A != 0`. If shrouded, return false.
3. Unit zone lookup via `vtable+0xBC`, `vtable+0x84`, then `MapClass::Can_Reach_Zone(...)`. If false, return false.
4. `MapClass::Get_CellClass(cell)` and `CellClass+0xEC == 5`. If false, return false.
5. Dynamic call `(**(vtable+0x1AC))(cellClassPtr, 0xFFFFFFFF, 0xFFFFFFFF, 0, 1)`.
6. If that call returns `0`, return `1` (harvestable); otherwise return `0`.

For a War Miner, `vtable+0x1AC` is `UnitClass::Can_Enter_Cell` at `0x0073F0A0`, not the base `FootClass::LocomotorPassabilityCheck` path. `UnitClass::Can_Enter_Cell` itself calls `FootClass::LocomotorPassabilityCheck` internally, but vehicle occupancy is also evaluated by the UnitClass override.

Argument order for this call, as recovered from the HARV path:

| Position | Value from `Is_Cell_Harvestable` | Meaning for this slice |
|----------|----------------------------------|------------------------|
| `this` | HARV/UnitClass object | dynamic dispatch uses UnitClass override |
| arg1 | `CellClass*` returned by `MapClass::Get_CellClass` | candidate ore cell |
| arg2 | `-1` | no explicit facing/direction constraint |
| arg3 | `-1` | no explicit level/subposition constraint |
| arg4 | `0` | default object-list/layer selector path |
| arg5 | `1` | enables the normal locomotor/passability branch used by this predicate |

Return handling is strict: only return code `0` is accepted as harvestable. Return codes `2`, `5`, `6`, or `7` all reject the ore cell.

### 3.5 Occupied Vehicle Return Handling

In `UnitClass::Can_Enter_Cell` at `0x0073F0A0`, after terrain/tube/locomotor gates, the function walks the selected cell object list and handles blockers:

- If the listed object is `param_1` itself, self is not treated as a blocker in that iteration.
- Enemy vehicle/object blockers can raise the result to `5` (`EnemyBlock`) or return `7` in hard-block branches.
- Allied stationary non-building blockers can raise the result to `6` (`FriendlyStationary`).
- Allied moving blockers can raise the result to `2` (`TemporaryBlock`) after the moving/contact checks.
- These codes are all non-zero, so `Is_Cell_Harvestable` rejects the cell.

Standard HARV-on-ore contention: Miner A sitting on ore at ring 1+ relative to Miner B is a non-self object in the candidate cell list. If allied and stationary, the scan predicate rejects it via code `6`; if allied and moving, via code `2`; if enemy, via code `5` or a stricter block. Miner B therefore should not choose Miner A's occupied ore cell while scanning outer rings.

## 4. INI Keys

| INI key / section | YR value | Binary use in this slice | Active in YR |
|-------------------|----------|--------------------------|--------------|
| `[General] TiberiumShortScan` | `6` | state 1 continuation scan radius, converted to cells | Yes |
| `[General] TiberiumLongScan` | `48` | state 0 scan radius, converted to cells | Yes |
| `[HARV] Harvester` | `yes` | selects normal harvester scan path | Yes |
| `[HARV] Teleporter` | absent/false | avoids chrono-specific teleport branch for HARV | Yes |
| `[HARV] Weeder` | absent/false | avoids weeder/no-zone scan path | Yes |
| `[HARV] Primary` | `20mmRapid` | unrelated to ore scan filtering | Yes, out-of-scope |

## 5. Integration Points

Call chain for the verified slice:

`UnitClass::Mission_Harvest` (`0x0073E5E0`) -> `FootClass::Search_For_Tiberium_And_Move` (`0x004DCFE0`) -> `FootClass::Scan_For_Tiberium` (`0x004DD0A0`) -> `FootClass::Is_Cell_Harvestable` (`0x004DCE80`) -> `UnitClass::Can_Enter_Cell` (`0x0073F0A0`).

Tick-cycle integration:

- State 0 runs when the miner is seeking a new ore patch. If a ring-1+ occupied cell is rejected and another cell in the same ring is harvestable, the other cell can win. If no candidate exists in that ring, the scan expands outward.
- State 1 continuation scans reuse the same predicate after the current cell is depleted and the miner is not full.
- The scan writes only a chosen cell result. Movement/pathing handles the chosen destination later; there is no scan-time target reservation system.

## 6. Current Rust Implementation Status

Current Rust surfaces inspected:

- `src/sim/miner/miner_system.rs::build_scan_filter`
- `src/sim/miner/miner_system.rs::is_cell_path_clear_for_scan`
- `src/sim/miner/miner_system.rs::search_local_ore`
- `src/sim/pathfinding/cell_entry.rs::{check_terrain_with_layers, classify_occupied_cell_with_layers}`
- `src/sim/miner/miner_tests.rs` scan-filter tests

Current status against this exact slice:

- `search_local_ore` keeps ring 0 unfiltered: if `nodes.get(center)` has remaining ore, it returns center before applying the filter. This matches `0x004DD0A0`.
- Rings 1+ call the optional filter after confirming a non-empty resource node. This matches the binary placement of `Is_Cell_Harvestable` for outer-ring candidates.
- `build_scan_filter` combines reachability and occupancy/path clearance before a candidate can win. For vehicle occupancy, this now matches the HARV-on-ore contention requirement.
- `scan_skips_cell_occupied_by_other_miner` and `scan_ring_0_allows_harvesters_own_cell` are the right acceptance-test shapes for this report.

Important nuance: the current Rust filter also rejects `PathGrid::is_walkable == false` cells. That broader static-blocker behavior is not claimed by this report; it belongs to the separate terrain-object/pathfinding slice and has had conflicting prior wording.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| HARV liveness through `UnitClass::Mission_Harvest` | verified | `0x0073E5E0`, `[HARV] Harvester=yes` | none for this slice |
| State 0 long scan call | verified | `0x0073E5E0` reads Rules `+0x177C` and calls `0x004DCFE0` | none |
| State 1 continuation scan call | verified | `0x0073E5E0` reads Rules `+0x1778` and calls `0x004DCFE0` for harvesters | none |
| `Search_For_Tiberium_And_Move` return/use behavior | verified | `0x004DCFE0` | none |
| Ring 0 fast path | verified | `0x004DD0A0`, `CellClass+0xEC == 5` immediate return | none |
| Ring bounds and early exit | verified | `0x004DD0A0`, `ring < radius`, `col <= ring`, `best_value != -1` break | none |
| Candidate harvestability gate for rings 1+ | verified | `0x004DD0A0` calls `0x004DCE80` before value ranking | none |
| `Can_Enter_Cell` argument order | verified | `0x004DCE80` dynamic call with `(cell, -1, -1, 0, 1)` | exact semantic names for args 2-5 deferred outside this slice |
| HARV vtable override at `+0x1AC` | verified | `UnitClass::Can_Enter_Cell @ 0x0073F0A0`; dynamic call from UnitClass object | no live debugger vtable memory read available, but function identity and prior reports agree |
| Allied/enemy vehicle blocker return handling | verified | `0x0073F0A0` occupied object-list branches produce non-zero block codes | full priority tree deferred |
| Ore target reservation check | verified negative | no such post-scan filter in `0x0073E5E0`/`0x004DCFE0` | none for this slice |
| Terrain object / tree blocked ore behavior | deferred | separate prior trace has conflicting wording | follow separate terrain-object scan/pathing audit |
| Shroud campaign gate | touched-not-exhausted | `0x004DCE80` checks `g_GameMode == 0` | campaign-only details out-of-scope |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 -- Is this a live YR War Miner path? -> Yes, HARV uses `UnitClass::Mission_Harvest` with `Harvester=yes`, `Weeder=no`, `Teleporter=no`.` (evidence: `0x0073E5E0`, `rulesmd.ini [HARV]`)
- `[RESOLVED] OQ-02 -- Does ring 0 call `Is_Cell_Harvestable`? -> No. It checks only `CellClass+0xEC == 5` and returns immediately.` (evidence: `0x004DD0A0`)
- `[RESOLVED] OQ-03 -- What ring bounds are scanned? -> Outer loop scans while `ring < radius`; default long radius 48 scans rings 1..47, plus the ring-0 fast path.` (evidence: `0x004DD0A0`, `rulesmd.ini TiberiumLongScan=48`)
- `[RESOLVED] OQ-04 -- Do rings 1+ call a harvestability predicate? -> Yes, every outer-ring candidate calls `FootClass::Is_Cell_Harvestable` before value scoring.` (evidence: `0x004DD0A0`)
- `[RESOLVED] OQ-05 -- What checks happen before `Can_Enter_Cell`? -> playfield, campaign shroud, zone reachability, and `LandType == 5`.` (evidence: `0x004DCE80`)
- `[RESOLVED] OQ-06 -- What arguments does `Is_Cell_Harvestable` pass to `vtable+0x1AC`? -> `(CellClass*, -1, -1, 0, 1)` after the land-type check.` (evidence: `0x004DCE80`)
- `[RESOLVED] OQ-07 -- What return value is accepted? -> Only `0`; any non-zero result rejects the candidate.` (evidence: `0x004DCE80`)
- `[RESOLVED] OQ-08 -- Does HARV use `UnitClass::Can_Enter_Cell` rather than only base `FootClass::LocomotorPassabilityCheck`? -> Yes. The dynamic slot for UnitClass is the override at `0x0073F0A0`; that override calls the base locomotor check internally but also handles object blockers.` (evidence: `0x0073F0A0`, `0x004D9C10`)
- `[RESOLVED] OQ-09 -- What happens for another allied vehicle on the candidate ore cell? -> Non-self allied blockers produce non-zero codes such as `6` stationary or `2` moving, so the cell is rejected.` (evidence: `0x0073F0A0`, `0x004DCE80`)
- `[RESOLVED] OQ-10 -- What happens for an enemy vehicle on the candidate ore cell? -> Enemy blockers produce non-zero block handling, commonly code `5` or stricter hard-block branches, so the cell is rejected.` (evidence: `0x0073F0A0`, `0x004DCE80`)
- `[RESOLVED] OQ-11 -- Is there a fake ore-target reservation filter after scan? -> No post-scan reservation filter was found in state 0 or `Search_For_Tiberium_And_Move`; scan result is sent to destination handling directly.` (evidence: `0x0073E5E0`, `0x004DCFE0`)
- `[RESOLVED] OQ-12 -- Does selection continue to farther rings after finding an occupied rejected cell? -> Rejected cells do not set `best_value`; if no accepted cell exists in that ring, the loop continues outward.` (evidence: `0x004DD0A0`)
- `[RESOLVED] OQ-13 -- How are equal ore values in one ring handled? -> Strict `old < new` update means first accepted candidate wins ties.` (evidence: `0x004DD0A0`)
- `[RESOLVED] OQ-14 -- Does current Rust filter ring 0? -> No, `search_local_ore` returns center before invoking the filter.` (evidence: `src/sim/miner/miner_system.rs`)
- `[RESOLVED] OQ-15 -- Does current Rust filter rings 1+ for occupied cells? -> Yes in current checkout: rings 1+ call `filter`, and `build_scan_filter` checks occupancy/path clearance.` (evidence: `src/sim/miner/miner_system.rs`)
- `[DEFERRED] OQ-16 -- Exact priority ordering for every `UnitClass::Can_Enter_Cell` branch.` (category: out-of-scope; reason: this report only needs occupied vehicle non-zero rejection for ore scan; next-step-if-pursued: run a dedicated `UnitClass::Can_Enter_Cell` full priority-tree investigation)
- `[DEFERRED] OQ-17 -- TerrainObject/tree-on-ore scan behavior.` (category: requires-different-system-context; reason: prior docs conflict over whether static terrain blockers belong to scan predicate or downstream pathing; next-step-if-pursued: audit TerrainClass/object-list representation in `UnitClass::Can_Enter_Cell` with a concrete tree-on-ore scenario)
- `[DEFERRED] OQ-18 -- Campaign shroud gate exact player/control flag meaning at `this+0x41A`.` (category: out-of-scope; reason: standard YR skirmish War Miner occupancy filtering does not depend on this branch; next-step-if-pursued: campaign shroud scan audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|-------------------|----------|--------------------|-----------------------|--------------------------------|---------------------|------------------|
| Ring 0 returns current ore cell before any occupancy/passability filter | `0x004DD0A0` | none observed | `src/sim/miner/miner_system.rs::search_local_ore` | Keep center-cell fast path unfiltered | `scan_ring_0_allows_harvesters_own_cell` | Do not reject the miner's own cell because occupancy contains itself |
| Rings 1+ call `Is_Cell_Harvestable` and accept only if `UnitClass::Can_Enter_Cell` returns `0` | `0x004DD0A0`, `0x004DCE80`, `0x0073F0A0` | none observed for vehicle occupancy | `build_scan_filter`, `is_cell_path_clear_for_scan`, `search_local_ore` | Reject ring-1+ ore cells occupied by non-self ground vehicles/structures | `scan_skips_cell_occupied_by_other_miner`; add `scan_skips_enemy_miner_occupied_cell` | Do not implement this as a fake ore-target reservation; it is a live cell-entry predicate |
| No post-scan ore target reservation exists | `0x0073E5E0`, `0x004DCFE0` | none observed | miner target selection / movement order boundary | Use the chosen scan cell directly; rely on scan predicate for occupied-cell exclusion | two miners selecting nearby ore should diverge only when one cell is currently occupied | Do not add "someone else is heading there" suppression unless separately verified |
| Nearest accepted ring wins; best value only within that ring | `0x004DD0A0` | none observed | `search_local_ore` | Continue to stop after first ring with an accepted candidate | high-density farther ore loses to any accepted nearer-ring ore | Do not globally sort all ore by density/value |
| Ties within one ring keep the first accepted candidate | `0x004DD0A0` strict `old < new` update | check comments/tests; current code uses `value <= cur` to keep first | `search_local_ore` | Preserve first-seen tie behavior in diamond-arm order | add `scan_equal_value_ring_tie_keeps_first_candidate` | Do not change to last-seen tie updates |

### Rust Test-Name Proposals

- `scan_skips_enemy_miner_occupied_cell`
- `scan_skips_moving_friendly_miner_occupied_cell`
- `scan_ring_1_occupied_cell_falls_through_to_farther_ring`
- `scan_ring_0_ignores_self_occupancy_even_when_filter_would_reject`
- `scan_equal_value_ring_tie_keeps_first_candidate`

### Negative Facts / Do Not Do

- Do not add a fake ore-target reservation system for miners "heading toward" an ore cell; no such post-scan check was found.
- Do not run `Can_Enter_Cell` or occupancy filtering on ring 0; the current cell fast path intentionally bypasses it.
- Do not treat the base `FootClass::LocomotorPassabilityCheck` as the whole HARV predicate. HARV reaches the `UnitClass::Can_Enter_Cell` override at `0x0073F0A0`.
- Do not choose the globally richest ore cell inside the scan radius. The binary chooses the richest accepted cell only within the nearest accepted ring.
- Do not convert non-zero `Can_Enter_Cell` codes into "still harvestable with higher cost" for the scan predicate; `Is_Cell_Harvestable` requires exactly `0`.

### Stale Docs / Follow-up Docs

- `MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md` should replace "strict less-than, so ties are broken by the last-updated winner" with: "strict `old < new` update; equal values do not update, so the first accepted candidate in scan order wins ties."
- `MINER_STUCK_SCAN_PICKS_BLOCKED_ORE_CELL_TRACE.md` should narrow its "occupier check absent" wording. Replacement: "For HARV/UnitClass candidates, `Is_Cell_Harvestable` dispatches `vtable+0x1AC` to `UnitClass::Can_Enter_Cell`; live vehicle occupants are checked there and reject rings 1+. TerrainObject/tree behavior is a separate static-blocker/pathing question and is not proven by absence of a direct `Cell_Occupier` call in `Is_Cell_Harvestable`."
- `MINER_STUCK_MULTI_MINER_CELL_CONTENTION_TRACE.md` remains directionally correct for vehicle occupancy, but its Rust disparity note is stale for the current checkout: current `miner_system.rs` now has a ring-1+ scan filter that checks occupancy.

## Sources

- Ghidra decompiled functions, read-only:
  - `UnitClass::Mission_Harvest` @ `0x0073E5E0`
  - `FootClass::Search_For_Tiberium_And_Move` @ `0x004DCFE0`
  - `FootClass::Scan_For_Tiberium` @ `0x004DD0A0`
  - `FootClass::Is_Cell_Harvestable` @ `0x004DCE80`
  - `UnitClass::Can_Enter_Cell` @ `0x0073F0A0`
  - `FootClass::LocomotorPassabilityCheck` @ `0x004D9C10`
- Prior docs consulted:
  - `docs/research/miner/traces/MINER_STUCK_MULTI_MINER_CELL_CONTENTION_TRACE.md`
  - `docs/research/miner/traces/MINER_STUCK_SCAN_PICKS_BLOCKED_ORE_CELL_TRACE.md`
  - `docs/research/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md`
  - `docs/research/miner/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`
- INI checked:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
- Rust inspected only, not edited:
  - `src/sim/miner/miner_system.rs`
  - `src/sim/pathfinding/cell_entry.rs`
  - `src/sim/miner/miner_tests.rs`

**Status:** COMPLETE for HARV occupied-vehicle ore-scan candidate filtering.
