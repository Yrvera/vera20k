# CellClass +0x122 — Semantic Identification

**Date:** 2026-05-18
**Scope:** Identify the semantic of the `cell + 0x122` byte and the gate that reads it during pathfinding.
**Trigger:** `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` §9 Open Question item 4 attributes the read
`*(char *)(cell + 0x122) == '\0' && param_7 != '\0'` to `UnitClass::Can_Enter_Cell @ 0x73F0A0`. This report
audits that claim against the live binary.

**Status:** COMPLETE.

---

## 1. Headline correction to the source doc

The hierarchy report is **wrong about the location** of the gate. The byte read described
(`cell + 0x122 == 0` paired with `param_7 != '\0'` causing skip) **does not occur in
`UnitClass::Can_Enter_Cell @ 0x73F0A0`**. A full disassembly walk of that function
(`0x73F0A0 .. 0x73FD43`, every `MOV` / `MOVSX` / `MOVZX` against the cell pointer in `ECX/EDX/EDI`)
reads the following CellClass byte offsets only: `0x14` (object flags via list iteration),
`0x44`, `0x54`, `0x58`, `0x11A`, `0x11B`, `0x124`, `0x128`, `0x140`, `0x21c`, `0xe4`, `0xe8`, `0xec`.
**`0x122` is never read in `UnitClass::Can_Enter_Cell`.**

The gate the source doc describes is real, but it lives in **`AStar_main_loop @ 0x429A90`**, at
instruction `0x429EB1` (`MOV CL, byte ptr [EBX + 0x122]`). The decompilation collapses to:

```c
// AStar_main_loop, expansion of neighbor `iVar16` (the neighbor CellClass*)
if (*(char *)(iVar16 + 0x122) == '\0' && param_7 != '\0')
    goto LAB_0042a1a1;   // ← skip this neighbor (loop-continue)
```

`param_7` of `AStar_main_loop` is the **hierarchical-A* enable flag**, propagated from
`AStar_pathfind_search`'s `param_8`. When `param_7 == 0` (regular A*), the gate is bypassed
unconditionally. The gate fires only in the hierarchical/zone-aware retry path.

---

## 2. What the +0x122 byte actually is

The 17 binary xrefs that read or write `[reg + 0x122]` (search pattern `8a 8? 22 01 00 00`)
all share an identical RMW signature: an 8-iteration loop that calls
`MapClass::Get_Cell_By_Coord @ 0x5657A0` for each of the 8 cardinal/diagonal neighbors of a
source cell, then performs `INC DL` or `DEC DL` on `byte[neighbor + 0x122]`. There is no
read site that reads `+0x122` as a condition **other than the AStar gate at `0x429EB1`**
and the bulk struct-copy in `MapClass::Resize` (which copies field 0x11D..0x122 byte-by-byte
between two CellClass instances).

So `+0x122` is a **per-cell 8-neighbor refcount of "an impassable / occupying object exists
in an adjacent cell."** The writers are:

| Writer site         | Function (entry)                                 | Op   | Trigger                                                |
|---------------------|--------------------------------------------------|------|--------------------------------------------------------|
| `0x5FC762`          | `OverlayClass::Mark @ 0x5FC570`                  | INC  | Wall overlay placed (gated on `OverlayType + 0x2a8`)   |
| `0x4809DD`          | `CellClass::PostDestructionWallCleanup @ 0x480630` | DEC  | Wall overlay removed (post-destruction cleanup)        |
| `0x481070`          | `CellClass::DestroyOverlay @ 0x480CB0`           | DEC  | Cell overlay destroyed                                 |
| `0x440CD9`          | `BuildingClass::Unlimbo @ 0x440580`              | INC  | Building placed; loops over its footprint              |
| `0x445D11`          | `BuildingClass::Limbo @ 0x445880`                | DEC  | Building removed; loops over its footprint             |
| `0x4D729A`          | `FootClass::Unlimbo @ 0x4D7170`                  | INC  | Unit placed on cell                                    |
| `0x4DB2D7`          | `FootClass::Limbo @ 0x4DB260`                    | DEC  | Unit removed from cell                                 |
| `0x4D86D8`, `0x4D8745` | `FootClass::PerCellProcess @ 0x4D85D0`        | DEC,INC | Per-cell tick (vacate old / claim new during step) |
| `0x4CEDA4`, `0x4CEE18`, `0x4CEE8B` | `FlyLocomotionClass::Descent_Step @ 0x4CE840` | DEC,INC,INC | Aircraft landing transitions             |
| `0x71C9A6`          | `TerrainClass::Limbo @ 0x71C930`                 | DEC  | Terrain object (tree, rock, …) removed                 |
| `0x71D085`          | `TerrainClass::Unlimbo @ 0x71D000`               | INC  | Terrain object placed                                  |

(`MapClass::Resize` sites `0x565FAF` and `0x5667C1` are field-by-field struct copies between
two CellClass instances, not semantic writes; they confirm the field lives at exactly `0x122`
between `+0x121 CachedFogEdgeFrame` and `+0x124 OccupationFlags`.)

The dominant pattern is: **placement of any blocking object (wall, building, terrain object,
unit, descending aircraft) increments `+0x122` on all 8 neighbor cells; removal decrements.**
The reader is exclusively the hierarchical A* expansion gate at `0x429EB1`.

### Correction to existing CellClass struct doc

`CELLCLASS_STRUCT_GHIDRA_REPORT.md` line 147 labels this byte `OreNeighborCount` (confidence
MED). That label is **incorrect**: ore-overlay placement does NOT increment `+0x122` — the
`OverlayClass::Mark` increment is gated by `OverlayType + 0x2a8` (IsWall), not by an ore
predicate. The increment runs for **walls**, not ore. Combined with the building/terrain/unit
writers, the correct label is closer to **`OccupiedNeighborCount`** or
**`BlockerNeighborCount`** — a generic refcount, not ore-specific.

---

## 3. What the gate does in hierarchical A*

In `AStar_main_loop`, when a candidate node's neighbor is being considered:

```c
if (*(char *)(neighbor_cell + 0x122) == '\0' && param_7 != '\0')
    continue;   // skip this neighbor entirely
```

In hierarchical mode (`param_7 != 0`), neighbor cells whose `+0x122` is zero — i.e., cells
that have **no blocker in any of their 8 surrounding cells** — are pruned from expansion.
Concretely: cells in the middle of open terrain (no walls/buildings/units/terrain near them)
are skipped, while cells adjacent to obstacles (within one cell of a wall, building, unit
column, or terrain feature) are expanded.

This is a hierarchical-pathfinding heuristic: long-range path search prefers cells near
structure (which carry route information from the obstacle topology) over cells in featureless
open space. The hierarchical retry loop iterates until `Zone_precheck` succeeds; this gate
narrows the expansion frontier on each retry.

The same gate does **not** appear in the per-step Can_Enter_Cell predicate — meaning the
heuristic is a pathfinding-frontier-pruning optimization, not a hard passability rule. Units
that already have a path through a `+0x122 == 0` cell are NOT blocked from entering it.

---

## 4. Active-in-YR determination

**Active in YR: Yes.**

- Writers are invoked by core gameplay paths: every building placement, every unit
  spawn/move, every wall placement, every aircraft landing, every terrain-object placement.
  These are unconditional and present on every map.
- The reader gate runs whenever `AStar_pathfind_search` is invoked with `param_8 != 0`. That
  flag is the hierarchical-A* mode used for long-range navigation across zone boundaries.
  Standard YR skirmishes hit this code path constantly — every long-range unit move command
  enters the hierarchical retry loop. The strings `"Hierarchical findpath failure"` and
  `"Regular findpath failure"` at `0x818820` / `0x8187C0` confirm the dual-mode dispatch.
- The gate has no `SpecialFlags` or `RulesClass` predicate gating it on/off; it runs whenever
  hierarchical mode is selected, which is whenever the source and destination zones differ
  (a common case).

No Tiberian Sun legacy markers around the gate — `param_7` is a plain bool, there's no
dormant feature flag involved.

---

## 5. Confidence

- **Content (the byte is an 8-neighbor blocker refcount):** HIGH. 17 xref sites, all
  unanimous INC/DEC RMW pattern, all loop 8 times via `MapClass::Get_Cell_By_Coord`. The
  field's writer set covers walls, buildings, units, terrain, aircraft — every kind of
  blocker the engine tracks.
- **Identity (the field is at CellClass offset 0x122):** HIGH. Confirmed by `MapClass::Resize`'s
  field-by-field copy loop at `0x565F73..0x565FBB`, which copies adjacent bytes `0x11D, 0x11E,
  0x11F, 0x120, 0x121, 0x122, 0x123` (the 0x123 byte is a padding follow-on touched by the
  same loop). Position bracketed by known fields `0x121 CachedFogEdgeFrame` and `0x124 OccupationFlags`.
- **Gate location (gate is in AStar_main_loop, NOT in UnitClass::Can_Enter_Cell):** HIGH.
  Verified by full disassembly of `UnitClass::Can_Enter_Cell @ 0x73F0A0` (no `0x122` literal
  in any instruction) and direct location of the gate at `AStar_main_loop + 0x421` (instruction
  `0x429EB1`).
- **Polarity (skip on zero in hierarchical mode):** HIGH. Decompilation shows
  `if (cell.+0x122 == 0 && param_7 != 0) goto skip` exactly as the prompt described, but the
  semantic is reversed from the prompt's hypothesis: the byte counts **blockers**, so
  `+0x122 == 0` means "this cell has no nearby obstacles" (an open-terrain cell), which is
  what gets skipped — not "non-water terrain" as the prompt suggested. The "amphibious gate"
  hypothesis is **refuted**.
- **YR-activity:** HIGH. Writers and reader are on the standard unit-movement and
  building-placement paths. No SpecialFlags or TS-only gating.

---

## 6. Cross-doc reconciliation

| Doc                                                | Claim about 0x122                                                                  | Status                                                                                                          |
|----------------------------------------------------|------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------|
| `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` §9.4 | Gate in `UnitClass::Can_Enter_Cell`; "non-water terrain gate for amphibious checks" | **WRONG** — gate is in `AStar_main_loop`; byte is a blocker-neighbor count, no amphibious relation              |
| `CELLCLASS_STRUCT_GHIDRA_REPORT.md` line 147       | `OreNeighborCount`, MED confidence                                                 | **WRONG label** — writers cover walls/buildings/units/terrain, not ore. Correct name: `OccupiedNeighborCount`   |
| `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`       | No mention of 0x122                                                                | Consistent — field is not bridge-related                                                                        |

Both stale claims trace back to insufficient writer-side verification: neither prior doc
followed the xrefs to the writer set. The hierarchy doc inferred location from a function
name; the struct doc inferred semantic from one writer (likely OverlayClass::Mark, mistakenly
generalized to "ore").

---

## 7. Implications for the Rust port

- **Closes** `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` §9 Open Question item 4.
- **No bridge or amphibious implication.** The byte plays no role in bridge passability,
  water/land transitions, or any visible naval/hover behavior.
- For port parity, this field is required only if the engine implements **hierarchical
  long-range A*** with the same frontier-pruning heuristic. If the Rust port uses a single
  uniform A* (no hierarchical retry loop), the field is unobservable and can be omitted.
  Player-visible effect of skipping the heuristic: minor pathfinding speed differences on
  long-range navigation in open-terrain maps; no change in final path correctness (because
  the regular-A* fallback runs when hierarchical fails, and the gate is bypassed there).
- If the port DOES implement hierarchical mode, this is a per-cell `u8` that increments on
  any blocker placed in any of 8 neighbors, decrements on removal, and is read only by the
  A* expansion loop. No struct-field alignment, packed-flag, or render-pipeline implications.
