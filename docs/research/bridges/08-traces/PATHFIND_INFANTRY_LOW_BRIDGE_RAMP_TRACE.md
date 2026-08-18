# Trace: Infantry Low Bridge Ramp Crossing

**Mechanic:** Infantry crosses a low bridge ramp (ground → bridge layer transition)
**Date:** 2026-05-20
**Slot:** 2 of /trace-swarm batch
**Scenario:** Conscript at ground cell 2S of south ramp; destination 2N of north ramp on an EW low bridge.

---

## Scenario Cell Layout

```
(col, row):  C-2 = ground south start
             C-1 = first ground cell approaching
             C+0 = south bridgehead ramp (transition cell: 0x100 | 0x200)
             C+1 = bridge deck / body cells
             C+2 = north bridgehead ramp (transition cell: 0x100 | 0x200)
             C+3 = first north ground cell
             C+4 = destination (2 cells north of north ramp)
```

Low bridge deck height: same level as ground (Level = 0, BridgeDirection::Low).
This is the key distinction from high bridges — no 4-level height offset.

---

## Stage Pipeline

### Stage 1: Zone Pre-check (Zone_precheck / zone_search.rs)

**gamemd.exe behavior (docs, HIGH confidence):**
- `Zone_precheck @ 0x42C290` runs 3-tier Dijkstra at levels 2→1→0.
- Uses `g_ZoneBaseCostByLandType` with bridge edge penalty 0.001 (essentially a tiebreaker).
- For low bridge cells: LandType = 10 (Tunnel/LowBridge from `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md §2.1`).
- LandType 10 = land-type-cost index [7] = 1.0 in `g_ZoneBaseCostByLandType`.
- Bridge-edge adjacency is encoded in the zone graph; infantry infantry zone (`Infantry` MovementZone) uses the passability matrix.
- Low bridge cells ARE passable for infantry. Infantry zone connects ground → bridge zone → ground.
- Zone_precheck returns 1 (reachable) → A* proceeds.

**Our code (`zone_search.rs:find_layered_path_zoned`):**
- Checks `zg.can_reach(mz, start, start_layer, goal, MovementLayer::Ground)`.
- For Infantry MovementZone, bridge cells are given ground-layer zone membership in the ZoneGrid builder (bridge cells redirect to ground endpoint zones per the comment at line 329: "bridge cells redirect to ground endpoint zones via `zone_at(Bridge)`, so a single ground-layer reachability check covers cross-bridge paths").
- If zone connectivity correctly includes the low bridge path, zone pre-check passes.
- If zone builder marks low bridge cells as a distinct zone not connected to the source/dest zones, this fails.

**Status: UNCHECKED** — the zone pre-check logic in our code checks ground-layer reachability, and our zone builder's handling of low bridge (LandType 10) cells is not verified against the binary. If low bridge cells are zoned as tunnel/underground (not ground), infantry cannot pathfind across them at the zone level, causing a false rejection before A* even runs. Cannot confirm numerical equality without running both.

---

### Stage 2: A* Goal Height Setup

**gamemd.exe behavior (HIGH confidence, BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md §2):**
- `AStar_pathfind_search @ 0x42C900` sets `pathfinder.+0x34` (dest height).
- For GROUND-layer destinations, dest height = `goal.Level`.
- For low bridge: low bridge cells have the SAME Level as ground (BridgeDirection::Low has no height offset).
- Locomotor type for infantry = Walk (CLSID `4A582744`). Walk locomotor kind != 2, so bridge height adjustment applies.
- BUT: low bridge cells have no 4-level height offset. Deck level == ground level for low bridges.
- Therefore `goal_height = goal_cell.Level` (same as ground).
- `start_height = start_cell.Level` (ground cell, same height).
- Both heights are 0 (flat terrain scenario).

**Our code (`core.rs:astar_search`, lines 535-541):**
```rust
let goal_height = if !options.is_infantry && goal_bridge_ok {
    goal_cell.bridge_deck_level
} else {
    goal_cell.ground_level
};
```
For infantry (`is_infantry = true`), goal always uses `ground_level`. For low bridge cells, `ground_level == bridge_deck_level` (both 0). So the code produces the correct goal height = 0.

**Status: PASS** (low bridge specific — both engines target ground_level = 0 for infantry crossing a low bridge destination).

---

### Stage 3: A* Expansion — Ground → South Bridgehead

**gamemd.exe behavior (HIGH confidence, BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md §1):**
- From cell C-1 (ground, Level=0) expanding to C+0 (south bridgehead, has flags 0x100|0x200, Level=0 for low bridge).
- Height diff = `p5_height - p1_level = 0 - 0 = 0` (diff_abs == 0).
- Low bridge has NO 4-level height difference. The diff-4 bridge entry case does NOT fire for low bridges.
- `CheckBridgeTraversal` step 3a (diff==0): checks if all of `{candidate.bridge, candidate.bridgehead, parent.bridge}` are set, OR `path_height == -1`, OR `path_height == candidate_level`.
  - Candidate (bridgehead) has 0x100 (bridge) = YES, 0x200 (bridgehead) = YES.
  - Parent (ground cell before ramp) has NO 0x100 flag → set is NOT complete.
  - `path_height == 0 == candidate_level` → condition passes.
- Move is ALLOWED. `*param_4` (bridge_entered) is NOT set (diff==4 ascending case only).
- Path_height remains 0. Layer selection: `is_at_bridge_level(0, bridgehead)` — bridgehead.bridge_walkable && abs(0 - bridgehead.ground_level=0) >= 2? → abs(0) = 0 < 2 → **FALSE**. So bridgehead is traversed on GROUND layer.
- Cost: base 1.0 (code-0, open path).

**Our code (`core.rs:check_bridge_traversal`):**
- For low bridge bridgehead (transition=true, bridge_walkable=true, ground_level=0):
- `needs_bridge_traversal_for_edge` fires (bridgehead has `has_bridgehead_transition()`).
- `check_bridge_traversal` with path_height=0, candidate_level=0, diff=0.
- `all_bridge_transition = candidate.has_structural_bridge() && candidate.has_bridgehead_transition() && parent.has_structural_bridge()`.
- Low bridge bridgehead in our model: `bridge_structural` is set if `bridge_walkable && !transition` (from test helper) BUT in our production code (PathCell construction from ResolvedTerrainCell) the mapping needs checking.

**Status: UNCHECKED** — the exact mapping of `bridge_structural` for low bridge bridgehead cells in PathCell construction from ResolvedTerrainCell is not verified. If low bridge bridgehead has `bridge_structural=false` but the diff==0 path requires `all_bridge_transition` (which needs `parent.has_structural_bridge()` which is also false), then the fallback `path_height == candidate_level` (0 == 0) allows it. This is the correct path. However, cannot confirm without running both engines.

---

### Stage 4: A* Expansion — Bridgehead to Deck / Body Cells

**gamemd.exe behavior (HIGH confidence):**
- From C+0 (bridgehead, Level=0, flags 0x100|0x200) to C+1 (deck, Level=0, flags 0x100, NO 0x200).
- Height diff == 0. Path_height = 0 = candidate.Level.
- `CheckBridgeTraversal` step 3a (diff==0): `path_height == 0 == candidate.Level` → ALLOWED.
- Layer: `is_at_bridge_level(0, deck_cell)` — deck.bridge_walkable && abs(0-0) >= 2 → FALSE → GROUND layer.
- Low bridge deck cells are traversed on the GROUND closed list (same as ground).
- Path cost: base 1.0 per cell.

**Our code:**
- `is_at_bridge_level(path_height=0, neighbor_cell)` checks `cell.bridge_walkable && 0.abs_diff(0) >= 2` → 0 < 2 → returns FALSE.
- Low bridge deck cells get `neighbor_use_bridge = false` → ground closed list.
- This is CORRECT for low bridges: the binary also keeps low bridge traversal on ground layer (no height offset means the layer-height-threshold never triggers bridge-list selection).

**Status: PASS** — both engines traverse low bridge deck on ground layer for flat (Level=0) low bridges.

---

### Stage 5: on_bridge Flag at Bridgehead Crossing (South Ramp Entry)

**gamemd.exe behavior (BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md, HIGH confidence):**
`WalkLocomotionClass::ProcessMovement @ 0x0075AEC0` applies the transition predicate at each cell boundary:

```c
if ((int8)dst->Level == (int8)src->Level - 4) {
    if (dst->Flags & 0x100) { object->OnBridge = 1; goto after; }
    if (src->Flags & 0x100) { object->OnBridge = 0; }
} else {
    if ((dst->Flags & 0x100) == 0) {
        if (src->Flags & 0x100) { object->OnBridge = 0; }
    }
}
```

For LOW bridge (Level delta = 0, NOT -4):
- Entering south bridgehead from ground: `dst.Level(0) == src.Level(0) - 4 = -4`? → NO. Falls to else branch.
- Else: `dst.Flags & 0x100 != 0` (bridgehead IS bridge cell) → condition `(dst.Flags & 0x100) == 0` is FALSE → neither set nor clear executes.
- **on_bridge stays 0 throughout entire low bridge crossing.**

For comparison, high bridge: `dst.Level(0) == src.Level(4) - 4 = 0` → YES → set on_bridge=1. 

**Our code (`movement_bridge.rs:compute_bridge_transition`):**
```rust
let entry = dst_h == src_h.wrapping_sub(4) && dst.has_structural_bridge();
let exit = !dst.has_structural_bridge() && src.has_structural_bridge();
```
For low bridge (dst_h=0, src_h=0): entry = `0 == 0.wrapping_sub(4) = -4 as u8 = 252`? → NO.
Exit = `!dst.bridge_walkable && src.bridge_walkable`. If src is ground (not bridge_walkable), exit = false.
Result: `NoChange` → on_bridge stays false.

**Status: PASS** — both engines leave on_bridge = 0 throughout a low bridge crossing (no 4-level height transition to trigger the set). This is the correct low bridge behavior.

---

### Stage 6: Occupancy Layer Selection (Bridge vs Ground List)

**gamemd.exe behavior (HIGH confidence):**
- `CellClass::AddContent @ 0x0047E8A0` reads `object->OnBridge` (byte at +0x8C) to select list.
- Since on_bridge stays 0 for low bridges, ALL cells (ground approach, bridgehead, deck, north bridgehead, ground exit) use ground list `cell+0xE4`.
- `CellClass::RemoveContent @ 0x0047EA90` likewise uses ground list throughout.
- Infantry occupies ground layer for entire low bridge crossing.

**Our code:**
- `on_bridge` stays false (Stage 5 PASS).
- Occupancy grid uses `on_bridge ? Bridge : Ground` layer selector.
- All occupancy operations during low bridge crossing use Ground layer.
- Correct behavior.

**Status: PASS** — both engines use ground occupancy list for the full low bridge crossing.

---

### Stage 7: Walk Locomotor Per-Step Processing — Ramp Walk

**gamemd.exe behavior (`BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md §2.9.1`, HIGH confidence):**
- `WalkLocomotionClass::ProcessMovement @ 0x75AEC0` handles per-tick movement.
- Bridge-state-mismatch detector at ~0x75B567: if `(cell.flags >> 8) & 1 != object.on_bridge`, sets `FootClass+0x68B = 1` (mismatch flag).
- For low bridge cells: cell.flags has bit 0x100 set (bridge structural), but on_bridge = 0.
- `(cell.flags >> 8) & 1 = 1` != `on_bridge = 0` → **mismatch flag fires on every low bridge deck cell**.
- This sets `FootClass+0x68B = 1` each tick the infantry is on a low bridge cell.
- Purpose of this flag: not fully traced by prior RE, likely triggers path re-evaluation.

**Our code (`movement_bridge.rs`):**
- `FootClass+0x68B` mismatch flag does not exist in our entity model.
- This gamemd mechanism exists specifically for walk locomotor — NOT for drive locomotor.

**Status: NOT-IMPLEMENTED** — the walk-locomotor bridge-state-mismatch detector (`FootClass+0x68B`) fires on every tick of low bridge traversal. Trigger: every cell step while on any bridge cell that has flags & 0x100 but on_bridge = 0. For low bridges this fires on EVERY deck step. Severity: MEDIUM — the mismatch flag's consumer is unknown; if it triggers path-replanning, infantry on low bridges may stutter in gamemd. Without knowing the consumer, cannot assess player-visibility exactly.

---

### Stage 8: LandType 10 Exemption in Walk ProcessMovement

**gamemd.exe behavior (`BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md §2.9.4`, HIGH confidence):**
- In ProcessMovement, when path is blocked AND Z-diff is within 2×height_step, an exemption exists for `LandType == 10` (Tunnel/LowBridge).
- This exemption allows infantry to NOT fail their path when blocked on a LandType-10 cell.
- Low bridge cells have LandType = 10 (`YR_CELL_LAND_TUNNEL` in our code).
- This is a walk-specific gate: Drive locomotor doesn't have this.

**Our code:**
- This specific soft-block exemption for LandType 10 doesn't appear to be modeled in our movement code.
- We have `YR_CELL_LAND_TUNNEL: u8 = 10` defined in `resolved_terrain.rs` but the exemption in soft-block cases is not in `movement_bridge.rs` or visible in `core.rs` pathfinding.

**Status: NOT-IMPLEMENTED** — the LandType-10 soft-block exemption in WalkLocomotionClass::ProcessMovement is not ported. Trigger frequency: fires only when the infantry's path is simultaneously blocked AND the unit is on a LandType-10 (low bridge) cell — a narrow corner case in normal play.

---

### Stage 9: North Bridgehead Exit / on_bridge Flag

**gamemd.exe behavior (PASS from Stage 5 analysis):**
- Exiting north bridgehead to ground: `dst.Level == src.Level` (both 0), dst is NOT bridge cell → else branch → `(dst.Flags & 0x100) == 0` is TRUE → but `src.Flags & 0x100` is TRUE (bridgehead is bridge) → `on_bridge = 0`... but it was already 0. No-op.
- on_bridge remains 0 upon exit. Correct.

**Our code:**
- `compute_bridge_transition`: src=bridgehead (bridge_walkable), dst=ground (not bridge_walkable). `exit = !dst.has_structural_bridge() && src.has_structural_bridge()` → might fire if bridgehead has bridge_structural=true.
- For low bridge bridgehead: `bridge_structural` in PathCell is mapped from `has_bridge_deck` field in ResolvedTerrainCell — needs verification.
- If bridge_structural is true for bridgehead (wrong), Exit fires and sets on_bridge=false (no-op since it's already false, benign).
- If bridge_structural is false for bridgehead (correct), no-op.
- Either way, result is on_bridge=false at exit. **Functionally correct.**

**Status: PASS** — on_bridge stays 0 at north bridgehead exit regardless of bridge_structural mapping detail.

---

### Stage 10: Rendering Layer at Each Step

**gamemd.exe behavior (BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md, HIGH confidence):**
- `ObjectClass::GetHeight @ 0x005F5F40`: when `OnBridge != 0`, subtracts bridge Z offset.
- For low bridge: on_bridge = 0 throughout → no Z adjustment → unit renders at ground level Z.
- Render layer: unit stays in the ground/tactical layer, NOT the bridge-deck render layer.
- Player sees the unit rendered at ground height while crossing the low bridge.

**Our code:**
- `on_bridge = false` → `bridge_occupancy = None` throughout.
- Render uses `position.z = dst_cell.ground_level` for all steps.
- Unit renders at ground Z (correct for low bridge).

**Status: PASS** — unit renders at ground Z throughout low bridge crossing for both engines.

---

### Stage 11: Path Cost Computation

**gamemd.exe behavior (BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md, HIGH confidence):**
- `AStar_compute_edge_cost @ 0x429830`: uses `g_AStar_EdgeCost_BaseTable`.
- For low bridge cells (open, code 0): base cost = 1.0.
- Diagonal-bridge cost multiplier (§3.5): gated on `param_4 != 0` (entering BRIDGE layer) AND `pathfinder.+1 != 0`.
- For low bridge: since `is_at_bridge_level` returns false (height diff < 2), we never select BRIDGE layer. `param_4 = 0`. Diagonal-bridge cost does NOT apply.
- BridgeApproach 4× multiplier (§3.4): gated on `dest_cell.flags & 0x40000`. This is set transiently by `UpdateBridgePassability` — not a permanent cell property.
- Net per-cell cost for low bridge deck: **1.0 × STEP_COST** (uniform).

**Our code (`core.rs`):**
- `terrain_cost = 100` for bridge layer, but since we don't enter bridge layer, we use terrain_costs grid.
- For low bridge cells on ground layer: terrain cost from the cost grid.
- Diagonal-bridge cost (2.0/1.0/10.0 multipliers): not implemented in our code (noted as missing in `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md §10`).
- For low bridge: diagonal-bridge cost doesn't apply (no BRIDGE layer entry), so the missing diagonal-bridge cost doesn't cause a divergence here.
- Path cost per step = STEP_COST = 1000.

**Status: PASS** (for the specific low bridge ramp cardinal case — diagonal-bridge cost diverges only on BRIDGE-layer entry, which doesn't happen for low bridges).

---

### Stage 12: Arrival at Destination

**gamemd.exe behavior:**
- Unit arrives at destination (2N of north ramp).
- on_bridge = 0, position.z = ground level.
- `FootClass::Find_Path` goal check: `(cx, cy) == goal && current.height == goal_height`.
- goal_height = 0 (ground), current.height = 0. Match → path reconstruction.

**Our code:**
- Goal check: `(cx, cy) == goal && current.height == goal_height` (line 617).
- goal_height = 0 (infantry uses ground_level), current.height = 0. Match.
- Path reconstructed via `reconstruct_path_dual`.

**Status: PASS** — goal detection fires correctly for both engines.

---

## Summary Table

| Stage | Description | gamemd | Ours | Status |
|-------|-------------|--------|------|--------|
| 1 | Zone pre-check reachability | Infantry zone includes low bridge | Zone builder LandType-10 handling unverified | UNCHECKED |
| 2 | A* goal height (infantry/low bridge) | ground_level=0 | ground_level=0 | PASS |
| 3 | Ground→bridgehead expansion | diff==0, path_height match, ALLOWED | diff==0 fallback via path_height==level | UNCHECKED |
| 4 | Bridgehead→deck traversal | diff==0, GROUND layer | diff==0, GROUND layer | PASS |
| 5 | on_bridge flag at south ramp entry | stays 0 (no Level-4 delta) | stays false (no delta-4) | PASS |
| 6 | Occupancy list layer selection | Ground list throughout | Ground list throughout | PASS |
| 7 | Walk mismatch detector `FootClass+0x68B` | fires every deck step | not implemented | NOT-IMPLEMENTED |
| 8 | LandType-10 soft-block exemption in Walk | exempts from path-fail on LT10 | not implemented | NOT-IMPLEMENTED |
| 9 | North bridgehead exit / on_bridge | stays 0, no-op | stays false, no-op | PASS |
| 10 | Render layer (GetHeight Z) | ground Z throughout | ground Z throughout | PASS |
| 11 | Path cost per step | 1.0 × STEP_COST, no diagonal mult | 1.0 × STEP_COST, no diagonal mult | PASS |
| 12 | Arrival / goal detection | height match → path done | height match → path done | PASS |

---

## Adjacent Findings (do NOT trace this run)

1. **High bridge on_bridge timing gap** — The `movement_step.rs` / `movement_tick.rs` occupancy-layer sequencing bug described in `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md §Current Rust Status`: removal observes pre-transition layer but insertion may observe wrong layer on high bridge ramp steps. Affects high bridges, not low bridges (where on_bridge stays 0).

2. **Diagonal-bridge cost not implemented** — Missing `AStar_compute_edge_cost` diagonal-bridge multipliers (2.0/1.0/10.0). Fires on any diagonal A* step entering BRIDGE layer. Does not affect low bridge (never enters bridge layer) but does affect high bridge diagonal approaches.

3. **Walk Z-bump rounding direction** — Walk uses `ftol(x - 0.5)` (round-half-down) vs Drive's `ftol(x + 0.5)` (round-half-up) for bridge dest-Z. 1-lepton discrepancy at boundary values. Affects high bridge infantry dest-Z, not low bridge.

4. **resolved_terrain.rs bridge derivation method diverges from gamemd** — `BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md` documents that gamemd stamps bridge flags via `SetBridgeDirection` from overlay anchors, not a global inference/gap-fill pass. Our resolved_terrain.rs uses a global bridge normalization approach. All downstream systems inherit this structural divergence.

5. **InfantryClass `path_height > cell.Level + 4` shortcut** — Infantry-specific A* shortcut at `0x51C055` allows cells where path_height exceeds deck by more than 4. Not in our pathfinding. Triggers on bridge-collapse / unusual height combos, not normal crossing.

---

## Evidence Sources

- `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md` — on_bridge field timing, writer sites, WalkLocomotionClass::ProcessMovement @ 0x0075AEC0 (assembly at 0x0075c16e-0x0075c19a)
- `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` — CheckBridgeTraversal @ 0x4D9C60 full decomp, diff-cases 0/1/4, cell flag semantics
- `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` — edge cost table @ 0x81870C, diagonal-bridge cost, Zone_precheck @ 0x42C290
- `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` — InfantryClass::Can_Enter_Cell @ 0x51BF90, vtable+0x1AC/0x1B0 layout
- `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` — WalkLocomotionClass::ProcessMovement @ 0x75AEC0 (5 bridge sites including mismatch detector @ ~0x75B567 and LandType-10 exemption @ ~0x75B81F)
- `BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md` — map-load bridge flag derivation, cell field table, flag bit semantics
- `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md` — PathfinderClass struct layout, height selection, dual closed-list mechanics
- Our code: `src/sim/pathfinding/core.rs`, `src/sim/movement/movement_bridge.rs`, `src/sim/pathfinding/zone_search.rs`
