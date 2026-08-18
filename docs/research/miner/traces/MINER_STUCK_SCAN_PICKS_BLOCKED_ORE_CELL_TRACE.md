# Miner Stuck — Ore-Cell Target Selection: Does Scan_For_Tiberium Pick Blocked Cells?

**Scenario:** Chrono Miner (CMIN) at cell (90, 185). Ore at cell (92, 187) with a
TerrainObject overlay (tree) on top. Does `FootClass::Scan_For_Tiberium` at 0x004DD0A0
return (92, 187) as the target, or skip it?

**Date:** 2026-05-20
**Scope:** `Scan_For_Tiberium` per-cell filter only. Caller chain: `UnitClass::Mission_Harvest`
case 0 → `FootClass::Search_For_Tiberium_And_Move` → `FootClass::Scan_For_Tiberium` →
`FootClass::Is_Cell_Harvestable`.

---

## Stage Table

| # | Stage | gamemd behavior | Our behavior | Verdict |
|---|-------|----------------|-------------|---------|
| A | Per-cell gate in `Scan_For_Tiberium` | `FootClass__Is_Cell_Harvestable` (0x004DCE80) — see detail below | `search_local_ore` + `build_reachable_filter` — see detail below | PASS (both naive about occupants) |
| B | Playfield bound check | `MapClass__Is_Cell_In_Playfield(cell)` — rejects out-of-bounds | Implicit: `nx < 0 || ny < 0 || nx > u16::MAX` guard in ring loop | PASS |
| C | Zone reachability | `MapClass__Can_Reach_Zone(miner_pos, ore_cell, zone_id, ...)` — compares flood-fill ZoneIDs; rejects if different zone | `ore_reachable()` via `ZoneGrid::can_reach()` — same concept, different impl | UNCHECKED (semantics equivalent in common case; corner cases not verified) |
| D | LandType check | `*(int*)(cell+0xEC) == 5` — must be Tiberium land type | `resource_nodes.contains_key(cell)` — nodes only exist for cells with ore overlay; LandType==Tiberium ↔ ore overlay for standard maps | PASS (equivalent for standard maps) |
| E | TerrainObject / occupier check | ABSENT — `Is_Cell_Harvestable` has no `Cell_Occupier()` call, no `IsImpassable` check | ABSENT — `search_local_ore` has no occupant check | PASS (both skip; matching behavior) |
| F | Locomotor passability check | `FootClass::vtable+0x1AC` → `FootClass__LocomotorPassabilityCheck` (0x004D9C10): calls attached locomotor's passability vtable slot with cell. For DriveLocomotion on ground unit: checks speed-type vs LandType table — LandType==Tiberium is passable for Tracked/Wheel → returns 0 → harvestable | `search_local_ore` has no equivalent locomotor passability check | FAIL (missing check; see note below) |
| G | Post-scan destination assignment | `Search_For_Tiberium_And_Move` (0x004DCFE0) → `(**(code**)(vtable+0x480))(cell)` = `Set_Destination(cell)` directly — NO `Find_Nearby_Passable_Cell` call in state 0 path | `snap.miner.target_ore_cell = Some(cell)` then drive issued | PASS (direct destination, no post-filter) |
| H | Ring early-exit (nearest wins) | `if (iVar6 != -1) break;` after each ring — first ring with any harvestable cell wins; no global best | Same: `if let Some((_, cell)) = best_in_ring { return Some(cell); }` | PASS |
| I | Center cell fast path | `*(int*)(cell+0xEC) == 5` check on center cell before ring loop; no `Is_Cell_Harvestable` call for center | Ring 0: `nodes.get(&center)` only — no zone/passability check for center | UNCHECKED (minor: center fast-path skips zone check in both; parity unclear for edge case where center is in different zone) |

---

## Key Finding: The Scan IS Naive About TerrainObject Occupants

**gamemd verdict:** `Scan_For_Tiberium` WILL return (92, 187) as the target even if a
TerrainObject (tree) sits on that cell, as long as:
1. The cell is in the playfield.
2. The cell shares the same zone ID as the miner (flood-fill connectivity).
3. `cell+0xEC == 5` (LandType is Tiberium — tree overlay does NOT change this).
4. The locomotor's passability check passes (LandType==Tiberium is passable for tracked units).

`Is_Cell_Harvestable` (0x004DCE80) has no `Cell_Occupier()` call, no building footprint
check, and no `IsImpassable` flag check. The occupant check at vtable+0x1AC is
`LocomotorPassabilityCheck` — it tests speed-type against LandType, not against object occupants.

**Conclusion:** The miner-stuck bug (if caused by targeting a blocked cell) is NOT introduced
by `Scan_For_Tiberium`. gamemd would also target that cell. The stuck behavior after targeting
is what diverges — the miner tries to pathfind into a cell occupied by a TerrainObject and
either spins or idles. This is a separate pathfinding/locomotor issue, not a scan filter gap.

---

## Stage F — LocomotorPassabilityCheck: Missing from Rust

**What gamemd does:** After the zone check and LandType==Tiberium check, `Is_Cell_Harvestable`
calls `FootClass::vtable+0x1AC` = `FootClass__LocomotorPassabilityCheck` (0x004D9C10) with
`(cellptr, -1, -1, 0, 1)`. That function reads the attached locomotor object at
`FootClass+0x674` and calls its passability vtable slot on the cell.

For DriveLocomotionClass on a ground unit with Tracked speed type:
- LandType==Tiberium → passable → returns 0 → `Is_Cell_Harvestable` returns 1 (harvestable).

For edge cases where the cell's LandType is NOT Tiberium but has a Tiberium overlay (unusual):
- The LandType gate (`cell+0xEC == 5`) already rejects it — locomotor check never fires.

For CMIN with teleport locomotor piggybacking DriveLocomotion:
- The piggybacked DriveLocomotion is at `FootClass+0x674`. Its passability check applies.

**Player-visible consequence of missing this check:** Negligible in standard scenarios.
The LandType==Tiberium gate is the primary filter; the locomotor check only fires for cells
that already passed it. An ore cell with LandType==Tiberium is passable for all harvester
speed types. The locomotor check would only reject if speed-type is NONE or the cell is
otherwise locomotor-specific impassable — not a real-world scenario for standard ore cells.

**Frequency:** Fires every ore-scan cycle (every tick in state 0). Impact: effectively zero
for standard terrain. Low player-visible severity.

---

## What Causes the Miner Stuck Bug (Adjacent Finding)

The scan correctly identifies ore cells regardless of TerrainObject occupants — matching
gamemd. The miner stuck symptom arises downstream:

1. Scan returns a cell with a tree on it.
2. `Set_Destination(cell)` is issued.
3. The locomotor tries to path to that cell.
4. The pathfinder's per-cell walkability check (separate from `Is_Cell_Harvestable`) sees
   the TerrainObject and marks the cell impassable.
5. The path fails; the miner stalls or loops.

This is NOT a scan-filter gap — it is a pathfinding/destination-assignment gap. The fix
(if needed) would be at the destination assignment level: if the chosen ore cell is
path-grid impassable, find the nearest walkable adjacent cell before calling
`Set_Destination`. gamemd does this only in the state 2 (return-to-refinery) path via
`Find_Nearby_Passable_Cell` — NOT in state 0 (ore scan). So both gamemd and our impl are
equally exposed to this. It is not a parity bug.

---

## Verified Binary Evidence

| Claim | Evidence |
|-------|----------|
| `Scan_For_Tiberium` at 0x004DD0A0 calls `Is_Cell_Harvestable` per candidate | Decompiled 0x004DD0A0 — `cVar3 = FootClass__Is_Cell_Harvestable(...)` in ring loop |
| `Is_Cell_Harvestable` at 0x004DCE80 | `search_functions("Is_Cell_Harvestable")` → 0x004DCE80; decompiled |
| No `Cell_Occupier` call in `Is_Cell_Harvestable` | Full decompile of 0x004DCE80 — no such call site found |
| `LandType==5` check at `cell+0xEC` | `if (*(int *)(uVar1 + 0xec) == 5)` in `Is_Cell_Harvestable` decompile |
| Zone check via `MapClass__Can_Reach_Zone` | `uVar1 = MapClass__Can_Reach_Zone(...)` in `Is_Cell_Harvestable` decompile |
| vtable+0x1AC = `FootClass__LocomotorPassabilityCheck` (0x004D9C10) | FootClass vtable base at 0x007E8C94 (confirmed via RTTI: `.?AVFootClass@@`); vtable+0x1AC = 0x007E8E40 → 0x004D9C10; `search_functions("LocomotorPassabilityCheck")` confirms |
| FootClass vtable+0x338 = `Scan_For_Tiberium` | 0x007E8C94+0x338 = 0x007E8FCC → read_memory → 0x004DD0A0 |
| `Search_For_Tiberium_And_Move` uses `Set_Destination` directly (vtable+0x480) | Decompiled 0x004DCFE0 — `(**(code**)(iVar3+0x480))(uVar4,uVar6)` |
| No `Find_Nearby_Passable_Cell` in state 0 path | Decompiled `UnitClass__Mission_Harvest` (0x0073E5E0) case 0 — `Find_Nearby_Passable_Cell` only called in case 2 (return path) |

---

## Verdict Tally

PASS: 5 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

---

## Top Findings (FAIL / NOT-IMPLEMENTED)

1. **Stage F — LocomotorPassabilityCheck missing** | Player sees: no observable difference in standard play (LandType==Tiberium is always passable for harvest units); only edge-case non-parity if a non-standard speed type is used | `src/sim/miner/miner_system.rs` (`search_local_ore` / `build_reachable_filter`, no passability call) | gamemd evidence: `Is_Cell_Harvestable` 0x004DCE80 decompile, vtable+0x1AC → 0x004D9C10.

---

## Status

COMPLETE
