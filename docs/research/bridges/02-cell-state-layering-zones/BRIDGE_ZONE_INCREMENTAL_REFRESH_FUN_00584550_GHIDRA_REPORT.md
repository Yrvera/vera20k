# Bridge Zone Incremental Refresh FUN_00584550 - Ghidra Research Report

**Address(es):** `0x00584550` primary; related `0x00586990`, `0x0056C510`, `0x00582D70`, `0x00581F90`, `0x005824A0`, `0x0042C1C0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Exact `FUN_00584550` localized zone patch contract, per-level patch radius, relation to coord-list refresh `0x00586990`, bridge/tube edge reinjection, and caller-side split from full `UpdateBridgeZonesHelper`.
**Non-Scope:** Broad bridge collapse parity, full A* search, exact vector allocator internals, full `UpdateBridgeZonesHelper` rebuild internals beyond contrast points, tactical dirty-rectangle parity.
**Confidence:** High
**Active in YR:** Yes. Direct callers include standard overlay/building/terrain mutation paths; bridge collapse/destruction paths reach it through `0x00586990`.

## Target Question

What exactly does `FUN_00584550` patch at every level, what input does it require, how does it relate to `FUN_00586990`, and when do callers choose this localized patch path versus full `UpdateBridgeZonesHelper` / full Rust `PathGrid` and `ZoneGrid` rebuilds?

## Non-Goals

- Do not re-investigate broad bridge collapse, repair, or A* behavior.
- Do not rename or modify Ghidra state.
- Do not implement Rust.
- Do not claim that gamemd has a Rust-style `PathGrid`; use Rust's `PathGrid` only in implementation handoff comparisons.

## Evidence Needed To Mark COMPLETE

- Decompile `0x00584550` and identify all material loops, constants, callees, and early outs.
- Trace direct callers of `0x00584550` and callers of `0x00586990`.
- Verify bridge caller split against `UpdateBridgeZonesHelper @ 0x0056C510`.
- Compare current Rust surfaces that perform bridge zone/path refresh.
- State Active in YR for every load-bearing claim.

## Stop Conditions

- Stop if a function boundary is missing and cannot be inspected read-only.
- Stop if material bridge behavior depends on runtime-only state not visible in static decompilation.
- Stop before any Rust, INI, in-repo doc, or Ghidra mutation.

## 1. Overview

`FUN_00584550` is a localized hierarchical zone-graph patcher for one changed cell coordinate. It does not recompute the whole map. For each zone hierarchy level `2, 1, 0`, it aligns the changed coordinate to a fixed block, removes old zone nodes and reciprocal edges for that block, flood-fills replacement zones inside the block, reinjects active bridge/tube edges that touch the block, emits final bidirectional edges, then refreshes downstream pathfinder scratch arrays.

`FUN_00586990` is the coord-list wrapper that bridge/destruction code commonly calls. It clears the changed cells' level-0 zone slots, recalculates `CellClass` attributes for each changed coordinate, then calls `FUN_00584550` only for cells whose level-0 slot is still zero after recalculation.

## 2. Class Layout / Key Offsets

| Struct / object | Offset | Type | Purpose | Active in YR |
|---|---:|---|---|---|
| `MapClass` | `+0x54` | ptr | `BridgeRecord` array base, 16-byte records. | Yes |
| `MapClass` | `+0x60` | int | Bridge record count. | Yes |
| `MapClass` | `+0x68` | ptr | Per-cell 4-byte zone-cell data: type byte, height byte, base cluster id. | Yes |
| `MapClass` | `+0x6C` | int | Zone-cell count / clamp upper bound. | Yes |
| `MapClass` | `+0x70` | ptr | Per-cell 10-byte hierarchical zone cache; level zone IDs live at `+0,+2,+4`, parent/base link at `+6`. | Yes |
| `MapClass` | `+0x74 + level*4` | int | Next zone id / zone-block count for level. | Yes |
| `MapClass` | `+0x80 + level*4` | ptr | 256 temporary 12-byte edge hash buckets for the level. | Yes |
| `MapClass` | `+0x90 + level*0x18` | vector header | Final zone-block array for level; block stride is `0x24`. | Yes |
| `BridgeRecord` | `+0x00/+0x04` | packed coords | Endpoint A/B used for bridge/tube edge injection. | Yes |
| `BridgeRecord` | `+0x08` | byte | Active/intact flag; `0x00584550` calls bridge edge insertion only when nonzero. | Yes |
| `BridgeRecord` | `+0x0C` | int | Bridge kind (`0` high, `1` low/tube). `0x00584550` itself does not filter this; callee handles high vs low/tube. | Yes |
| `DynamicVector<CellCoord>` | `+0x04/+0x10` | ptr/count | Input list consumed by `0x00586990`; entries are 4-byte packed `i16 x, i16 y`. | Yes |

## 3. Core Logic

### 3.1 Input contract

| Function | Input contract | Verified behavior | Active in YR |
|---|---|---|---|
| `FUN_00584550 @ 0x00584550` | `MapClass* this` in `ECX`, `short* coord` argument. Coord is X at `+0`, Y at `+2`. | Immediately calls `MapClass__Is_Cell_In_Playfield(coord, 1)` and returns with no writes if false. | Yes |
| `MapClass__RecalcCellsAndRebuildZones @ 0x00586990` | `MapClass* this`, dynamic vector of packed coords. | Iterates the list backward twice; filters each coord through `Is_Cell_In_Playfield(coord, 1)`. | Yes |

`0x00584550` assumes the caller already changed cell state and, when necessary, cleared the affected zone slot. It is not a mutator of bridge overlays or terrain. It patches the zone graph for a coordinate whose zone assignment may need regeneration.

### 3.2 Per-level behavior of `0x00584550`

The outer loop starts at `level = 2` and decrements through `1` and `0`.

| Level | Block size expression | Patch block size | Alignment | Active in YR |
|---:|---|---:|---|---|
| 2 | `1 << (level + 1)` | 8x8 cells | `x - x % 8`, `y - y % 8` | Yes |
| 1 | `1 << (level + 1)` | 4x4 cells | `x - x % 4`, `y - y % 4` | Yes |
| 0 | `1 << (level + 1)` | 2x2 cells | `x - x % 2`, `y - y % 2` | Yes |

For each level the function:

1. Clears all temporary edge buckets for that level (`MapClass+0x80+level*4`) using vector vtable `+0x0C`.
2. Scans every cell in the aligned patch block.
3. Collects distinct old nonzero zone IDs for this level into a scratch vector.
4. Clears each cell's zone ID for this level to zero.
5. Copies the base cell cluster id from `MapClass+0x68 + cell_index*4 + 2` into the 10-byte zone cache at `+6`.
6. For each old zone ID, removes reciprocal final edges from neighbor zone blocks and clears the old zone block with vtable `+0x0C`.
7. Flood-fills replacement zones only inside the aligned block, skipping out-of-playfield cells and zone type `7` sentinel cells.
8. Writes each new zone block's parent link, zone type, constant `+0x14 = 0x10`, and representative tile bucket value.
9. Updates the level's zone count at `MapClass+0x74+level*4`.
10. Reinjects active bridge/tube edges for records whose endpoint A or endpoint B lies inside the same aligned block.
11. Converts temporary 12-byte hash-bucket edges into final bidirectional 8-byte zone-block edge entries.

The representative bucket formula written to zone-block `+0x20` is:

```text
((x + sign_adjust_3) >> 2) + 0x83 + (((y + sign_adjust_3) >> 2) * 0x82)
```

This is integer truncation with a signed-negative adjustment before shifting. For standard playfield coordinates it behaves as `x / 4 + 0x83 + (y / 4) * 0x82`.

### 3.3 Final 8x8 parent-link sync and scratch refresh

After levels `2, 1, 0` have been patched, `0x00584550` aligns the original coord to an 8x8 block using the `& 0x80000007` signed-modulo sequence and scans that 8x8 block. For each in-playfield cell it copies the next-level zone id into the zone-block `+0x18` link field for two lower levels. It then calls `FUN_0042C1C0`.

`FUN_0042C1C0 @ 0x0042C1C0` frees and reallocates zeroed arrays for three level-related scratch groups, using sizes read from the `DAT_0087F810` region. This is not a whole-map flood-fill. It is a post-patch scratch/cache refresh for pathfinder/zone consumers.

Active in YR: Yes. Evidence: direct call at the end of `0x00584550`; `0x0042C1C0` decompiled and shows three-iteration free/new/zero loops.

### 3.4 Bridge/tube edge reinjection inside the local patch

Within each level, `0x00584550` iterates all bridge records backward:

```text
for record in MapClass+0x54 records:
  if endpoint_a inside aligned block OR endpoint_b inside aligned block:
    if record+0x08 != 0:
      FUN_00582D70(record, level)
```

`FUN_00582D70 @ 0x00582D70` computes three connection pairs and inserts temporary bucket entries for high bridge or low/tube records. It tests bridge tile identity first (`CellClass__IsBridge` / `IsWoodBridge`); otherwise it uses `GetTubeAtCell` and tube direction data. It inserts pairs using zone IDs from the current level and writes temp edge flag low byte `0`.

Active in YR: Yes. Evidence: decompile of `0x00584550` block endpoint test and `record+0x08` gate; decompile of `0x00582D70`.

### 3.5 `0x00586990` relationship

`MapClass__RecalcCellsAndRebuildZones @ 0x00586990` is a two-pass list helper:

1. Backward pass over coords:
   - in-playfield check
   - clamp linear index from `(MapClass+0xF8 + 1 + MapClass+0xF4) * y + x`
   - clear level-0 zone slot at `MapClass+0x70 + index*10`
   - fetch `CellClass` from `g_CellArray_Base[y*0x200 + x]` or fallback cell
   - call `CellClass__RecalcAttributes @ 0x0047D2B0`
2. Backward pass over coords:
   - in-playfield check
   - recompute/clamp same linear index
   - if level-0 zone slot is still zero, call `FUN_00584550(coord)`

This proves `0x00586990` is not the local patch itself; it is the changed-cell list driver that prepares cells and delegates to `0x00584550` only when the cell remains unassigned after recalc.

Active in YR: Yes. Evidence: decompile of `0x00586990`; callers include standard bridge damage/destruction functions and region helpers.

### 3.6 Full rebuild split

Full `UpdateBridgeZonesHelper @ 0x0056C510` is caller-side and separate:

- `ProcessBridgeDamageStateMachine_High @ 0x00576BA0` and `_Low @ 0x00571490` call `InvalidateBridgeZones`; only if that returns nonzero do they call `UpdateBridgeZonesHelper`. Their full-collapse branches can also build a changed-cell vector and call `0x00586990` afterward.
- `ProcessBridgeDestruction_High @ 0x00573540` and `_Low @ 0x00570050` call `ValidateBridgeZones`; if validation reports a bridge-record connectivity change, they call `UpdateBridgeZonesHelper`. Separately, if their local changed-cell vector has entries, they call `0x00586990`.
- `MapClass__BridgePavementSpanWalker @ 0x00569760` and high twin `0x00568E40` OR together `ValidateBridgeZones` results into a byte. If nonzero, they call `UpdateBridgeZonesHelper`; if their accumulated coord vector is nonempty, they call `0x00586990`.

So the vanilla ordering is not "bridge changed -> always full rebuild". It is:

```text
cell/overlay changes -> coord-list recalc -> localized 0x00584550 patch as needed
bridge record active/intact connectivity changed -> caller may run full 0x0056C510 rebuild
```

Active in YR: Yes. Evidence: decompile of the four bridge state/destruction callers plus `0x00569760` and `0x00568E40`.

## 4. INI Keys

No INI key is read directly by `0x00584550` or `0x00586990`.

| Key / data source | Role | Default / evidence | Active in YR |
|---|---|---|---|
| `[CombatDamage] DestroyableBridges=` | Upstream bridge damage/destruction gate only. | `rulesmd.ini`: `DestroyableBridges=yes`. Not read by scoped helpers. | Yes upstream |
| `[CombatDamage] BridgeStrength=` | Upstream damage RNG gate only. | `rulesmd.ini`: `BridgeStrength=1500`. Not read by scoped helpers. | Yes upstream |
| Theater bridge tile globals | Consumed by bridge callers and `0x00582D70`, not by `0x00584550` directly. | Prior bridge docs and decompile of `0x00582D70`. | Yes |

## 5. Integration Points

| Function | Relationship | Evidence | Active in YR |
|---|---|---|---|
| `CellClass__DestroyOverlay @ 0x00480CB0` | Directly calls `AssignOrphanedCellZone`, then `0x00584550`, after wall overlay removal and `RecalcAttributes`. | Decompile direct call. | Yes |
| `OverlayClass__Mark @ 0x005FC570` | For non-editor wall overlay placement, calls `MergeAdjacentCellZone`, then `0x00584550`. Bridge overlay marking itself is handled by bridge-direction setup, not this wall path. | Decompile direct call. | Yes |
| `BuildingClass__Place_OccupyMap @ 0x00441F60` | For each foundation cell, resets overlay/occupancy, recalcs attributes, assigns orphaned zone, then calls `0x00584550`. | Decompile direct call. | Yes |
| `AnimClass__Middle @ 0x00424CE0` | Tiberium/vein-like cell mutation path calls `RecalcAttributes`, `AssignOrphanedCellZone`, then `0x00584550`. | Decompile direct call. | Conditional, based on anim type flag |
| `MapClass__RecalcCellsAndRebuildZones @ 0x00586990` | Coord-list wrapper; calls `0x00584550` only when slot 0 remains zero after recalc. | Decompile direct call. | Yes |
| `MapClass__BridgePavementSpanWalker @ 0x00569760` | Bridge-specific changed-cell producer; calls `0x00586990` when accumulated list is nonempty and separately calls full rebuild on bridge-record validation change. | Decompile direct calls. | Yes |
| `ProcessBridgeDamageStateMachine_{High,Low}` | Collapse branches can call full rebuild after invalidate and also call `0x00586990` for local recalc lists. | Decompile direct calls. | Yes |
| `ProcessBridgeDestruction_{High,Low}` | Direct destruction paths can call full rebuild after validate and also call `0x00586990` for local recalc lists. | Decompile direct calls. | Yes |
| `UpdateBridgeZonesHelper @ 0x0056C510` | Full-zone rebuild contrast point, not called by `0x00584550` or `0x00586990`. | Callers and decompile. | Yes |

Tick-cycle position is caller-dependent. On bridge damage/destruction, these functions run synchronously in the damage/collapse call path before later movement/path decisions consume the zone graph.

## 6. Current Rust Implementation Status

| Binary behavior | Rust surface | Status |
|---|---|---|
| One-cell localized hierarchical block patch at levels 2/1/0 | `src/sim/pathfinding/zone_incremental.rs::try_incremental_update` | Partial but structurally different. Rust updates by affected-zone clearing/reflood inside a padded bbox and falls back if resolved terrain is present. |
| Terrain-aware dynamic changes use localized patch in gamemd | `src/sim/pathfinding/zone_incremental.rs:45` | Missing for resolved-terrain mode. The code currently returns `false` immediately when `resolved_terrain.is_some()`, forcing full rebuild. |
| Bridge connectivity full rebuild is caller-side and conditional | `src/sim/world/bridge_orchestrator.rs::refresh_bridge_zones_if_dirty` | Conservative full rebuild. It builds `PathGrid::from_resolved_terrain_with_bridges` and calls `Simulation::rebuild_zone_grid`. |
| Coord-list recalc before local patch | `src/sim/overlay_grid.rs::take_dirty_cells`, `recalc_overlay_passability`; `src/app_sim_tick.rs` dirty overlay handling | Similar concept exists for overlays, but bridge state paths mostly signal `bridge_state_changed` / `zones_dirty` rather than feeding a shared localized map-refresh service. |
| Full zone-grid rebuild | `src/sim/world/mod.rs::rebuild_zone_grid`; `src/sim/pathfinding/zone_map.rs::build_with_terrain` | Present and safer than gamemd's local patch, but less faithful in update breadth and timing. |

Important Rust delta: Rust does not expose gamemd's three-level block patch. It often does a full dynamic `PathGrid` plus `ZoneGrid` rebuild after bridge changes. This is behaviorally safe for connectivity if done before consumers read pathing, but it is not the original engine's incremental contract.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00584550` input and playfield early return | verified | Decompile `0x00584550` | none |
| `FUN_00584550` per-level block sizes 8/4/2 | verified | Decompile `0x00584550`, `1 << (level+1)` with levels 2..0 | none |
| Old-zone collection/removal | verified | Decompile `0x00584550`, scratch vector and zone-block edge removal loops | exact vector allocator internals not needed |
| Replacement flood-fill inside block | verified | Decompile `0x00584550`; callee `0x005824A0` | none |
| Bridge/tube edge reinjection | verified | Decompile `0x00584550`; `0x00582D70` | none |
| Final bidirectional edge emission | verified | Decompile `0x00584550`; contrast `0x00581F90` | none |
| Final 8x8 link sync | verified | Decompile `0x00584550` tail | exact semantic of all consumers deferred to zone-precheck docs |
| `FUN_0042C1C0` scratch refresh | verified | Decompile `0x0042C1C0` | array names remain inferred |
| `FUN_00586990` two-pass coord-list contract | verified | Decompile `0x00586990`; callers/callees | none |
| Bridge full rebuild split | verified | Decompile callers `0x00569760`, `0x00568E40`, `0x00576BA0`, `0x00571490`, `0x00573540`, `0x00570050`; callers of `0x0056C510` | none for this slice |
| Rust comparison | verified high-level | Codegraph plus reads of `zone_incremental.rs`, `zone_map.rs`, `bridge_orchestrator.rs`, `world/mod.rs`, `app_sim_tick.rs` | no Rust tests run because this is research-only |
| Full A* path behavior after patch | deferred | out-of-scope | trace concrete path scenario separately |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is `0x00584550` active in YR? -> Yes; direct callers include overlay destruction/placement and building placement, and bridge paths reach it through `0x00586990`.` (evidence: `get_function_callers 0x00584550`; decompile callers `0x00480CB0`, `0x005FC570`, `0x00441F60`, `0x00586990`)
- `[RESOLVED] OQ-02 - Does `0x00584550` take a changed-cell list? -> No; it takes one coord. The list wrapper is `0x00586990`.` (evidence: decompile `0x00584550`, `0x00586990`)
- `[RESOLVED] OQ-03 - What per-level blocks are patched? -> Level 2 patches 8x8, level 1 patches 4x4, level 0 patches 2x2, each aligned down by coordinate modulus.` (evidence: `0x00584550`)
- `[RESOLVED] OQ-04 - Does it rebuild the whole map? -> No; it touches aligned local blocks and then an 8x8 link-sync block around the changed coordinate.` (evidence: `0x00584550`)
- `[RESOLVED] OQ-05 - How are old zones removed? -> Distinct old IDs in the block are collected; reciprocal final edges to those IDs are removed from neighbor blocks; then old zone blocks are cleared.` (evidence: `0x00584550`)
- `[RESOLVED] OQ-06 - How are replacement zones created? -> It flood-fills unassigned non-sentinel cells inside the block with `ZoneMap__FloodFillScanline`.` (evidence: `0x00584550`, `0x005824A0`)
- `[RESOLVED] OQ-07 - Are bridge/tube edges considered in the local patch? -> Yes; active records with either endpoint inside the patch block call `0x00582D70` for every level.` (evidence: `0x00584550`, `0x00582D70`)
- `[RESOLVED] OQ-08 - Does local bridge edge injection filter high vs low records in `0x00584550`? -> No. `0x00584550` tests only endpoint-in-block and active byte; `0x00582D70` handles bridge vs tube detail.` (evidence: `0x00584550`, `0x00582D70`)
- `[RESOLVED] OQ-09 - What does `0x00586990` do before calling `0x00584550`? -> It clears level-0 slot and calls `CellClass__RecalcAttributes` for each listed in-playfield coord, then calls `0x00584550` only if the level-0 slot remains zero.` (evidence: `0x00586990`)
- `[RESOLVED] OQ-10 - Does `0x00586990` call full `UpdateBridgeZonesHelper`? -> No.` (evidence: callees of `0x00586990`; decompile body)
- `[RESOLVED] OQ-11 - Where is full bridge-zone rebuild chosen? -> Bridge callers choose it around `ValidateBridgeZones` / `InvalidateBridgeZones` return values, separate from changed-cell local refresh lists.` (evidence: decompile `0x00569760`, `0x00568E40`, `0x00576BA0`, `0x00571490`, `0x00573540`, `0x00570050`)
- `[RESOLVED] OQ-12 - Does `0x00584550` read INI keys? -> No direct INI reads.` (evidence: decompile `0x00584550`; INI search only finds upstream bridge keys)
- `[RESOLVED] OQ-13 - Is this TS-only or gated off by SpecialFlags? -> No TS-only gate found in the scoped functions; callers are standard YR map/overlay/building/bridge paths.` (evidence: caller graph and decompile)
- `[RESOLVED] OQ-14 - What is the final `0x0042C1C0` call? -> A three-group free/new/zero scratch-array refresh keyed from `DAT_0087F810`, not a whole-map flood-fill.` (evidence: decompile `0x0042C1C0`)
- `[RESOLVED] OQ-15 - Does Rust currently mirror this exact local patch contract? -> No. Rust has a terrain-disabled incremental updater and otherwise uses full dynamic path/zone rebuilds for bridge changes.` (evidence: `src/sim/pathfinding/zone_incremental.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/world/mod.rs`)
- `[DEFERRED] OQ-16 - Exact names of the scratch arrays reallocated by `0x0042C1C0`.` (category: requires-different-system-context; reason: not needed to answer local patch vs full rebuild contract; next-step-if-pursued: trace consumers of the arrays around `DAT_0087F810`)
- `[DEFERRED] OQ-17 - Player-visible A* route delta caused by local patch timing versus full rebuild timing.` (category: out-of-scope; reason: requires a concrete pathing trace, not this helper slice; next-step-if-pursued: run `/trace-action low bridge route before and after collapse`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x00584550` patches aligned 8x8/4x4/2x2 hierarchy blocks for one changed coord, not the whole map. | `0x00584550` decompile, levels 2..0 and `1 << (level+1)`. | Missing exact contract; Rust terrain-aware path falls back to full rebuild. | `src/sim/pathfinding/zone_incremental.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/world/mod.rs`. | If optimizing for parity/perf, add a localized terrain-aware zone patch that updates only affected hierarchy-equivalent regions or document why full rebuild is accepted. | Collapse or repair one bridge cell and verify only local zone refresh work is scheduled while reachability matches full rebuild. | Do not call every changed bridge cell a full map rebuild requirement. Proposed test: `bridge_zone_incremental_patches_aligned_blocks_only`. |
| `0x00586990` clears listed cells' level-0 slot and recalculates `CellClass` attributes before deciding whether to call `0x00584550`. | `0x00586990` decompile. | Rust has overlay dirty recalc and bridge `zones_dirty`, but no shared changed-cell recalc -> local-zone-patch service for bridge changes. | `src/sim/overlay_grid.rs`, `src/app_sim_tick.rs`, `src/sim/world/bridge_orchestrator.rs`. | Introduce or preserve a deterministic changed-cell refresh stage before bridge/path zone consumers read connectivity. | A final bridge walker emits a coord list; attributes/passability update happens before zone reachability is queried. | Do not skip local recalc because a later full rebuild exists. Proposed test: `bridge_changed_cell_recalc_precedes_zone_patch`. |
| Full `UpdateBridgeZonesHelper` is caller-side and conditional on bridge record validate/invalidate changes, while coord-list local patch remains separate. | Caller decompile of `0x00569760`, `0x00568E40`, `0x00576BA0`, `0x00571490`, `0x00573540`, `0x00570050`. | Rust currently aggregates bridge outcomes into `zones_dirty` and rebuilds `PathGrid`/`ZoneGrid` broadly. | `src/sim/world/bridge_orchestrator.rs::refresh_bridge_zones_if_dirty`; `src/sim/world_orders.rs`. | Keep two signals: local changed cells for recalc/patch and bridge-record connectivity dirty for full rebuild/fallback. | Damaged visual-state repair that changes cells but not endpoint connectivity should not be forced through the same signal as full span collapse. | Do not conflate "visual cell changed" with "bridge endpoint connectivity changed." Proposed test: `bridge_visual_refresh_without_bridge_record_full_rebuild`. |
| Local patch reinjects active high and low/tube bridge records when either endpoint is inside the patch block. | `0x00584550` record loop; `0x00582D70` high/tube branches. | Rust bridge adjacency injection exists in full `ZoneGrid::build_with_terrain`; incremental terrain path returns false before doing equivalent local reinjection. | `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_incremental.rs`. | Any future incremental bridge patch must refresh bridge adjacency for records touching the local patch block, including low/tube records where applicable. | Low bridge/tube and high bridge records both retain reachability when a nearby cell is locally patched. | Do not filter low records out of local reinjection. Proposed test: `zone_incremental_reinjects_active_high_and_low_bridge_records`. |

## Negative Facts / Do Not Do

- Do not describe `0x00584550` as a full map rebuild. It is a local hierarchical patch.
- Do not describe `0x00586990` as the patch algorithm. It is the coord-list recalc wrapper that may call the patcher.
- Do not make `sim/` depend on render/UI to model tactical dirty rectangles; this slice is path/zone state only.
- Do not filter bridge records by `BridgeRecord+0x0C == 0` inside the local patch. `0x00584550` does not.
- Do not assume vanilla always rebuilds full bridge zones after a bridge visual cell changes. Full rebuild is separately gated by validate/invalidate return values.
- Do not remove the conservative Rust full rebuild until an incremental path can reinject bridge adjacency and preserve reachability for all movement-zone consumers.

## Remaining Uncertainty

- Exact semantic names for scratch arrays refreshed by `0x0042C1C0`.
- Concrete player-visible route choice delta between vanilla local patch timing and Rust full rebuild timing remains untraced.

## Stale Docs / Follow-up Docs

`BRIDGE_CELLLIST_DIRTY_ZONE_HELPERS_00569760_00586990_GHIDRA_REPORT.md` OQ-10 can be closed with:

> `FUN_00584550 @ 0x00584550` is the one-coordinate localized hierarchical zone patcher. For levels 2, 1, and 0 it patches aligned 8x8, 4x4, and 2x2 blocks respectively, removes old zone-block edges, flood-fills replacement zones, reinjects active bridge/tube edges for records whose endpoints touch the block, syncs lower-level parent links across the enclosing 8x8 block, and refreshes zone/path scratch arrays via `0x0042C1C0`. It is separate from full `UpdateBridgeZonesHelper @ 0x0056C510`, which callers invoke only when bridge-record validate/invalidate logic says full connectivity changed.

## Sources

- Ghidra decompiled: `FUN_00584550 @ 0x00584550`
- Ghidra decompiled: `MapClass__RecalcCellsAndRebuildZones @ 0x00586990`
- Ghidra callers/callees: `0x00584550`, `0x00586990`, `0x0056C510`
- Ghidra decompiled: `FUN_00582D70 @ 0x00582D70`
- Ghidra decompiled: `ZoneMap__BuildZoneLevel @ 0x00581F90`
- Ghidra decompiled: `ZoneMap__FloodFillScanline @ 0x005824A0`
- Ghidra decompiled: `FUN_0042C1C0 @ 0x0042C1C0`
- Ghidra decompiled callers: `0x00569760`, `0x00568E40`, `0x00576BA0`, `0x00571490`, `0x00573540`, `0x00570050`, `0x00480CB0`, `0x005FC570`, `0x00424CE0`, `0x00441F60`
- Prior docs: `BRIDGE_CELLLIST_DIRTY_ZONE_HELPERS_00569760_00586990_GHIDRA_REPORT.md`, `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`, `BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md`, `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md`
- Rust scanned: `src/sim/pathfinding/zone_incremental.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/world/mod.rs`, `src/sim/overlay_grid.rs`, `src/app_sim_tick.rs`
