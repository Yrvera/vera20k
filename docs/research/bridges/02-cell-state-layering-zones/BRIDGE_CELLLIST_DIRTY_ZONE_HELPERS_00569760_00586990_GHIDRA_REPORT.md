# Bridge Cell-List Dirty/Zone Helpers 0x00569760 / 0x00586990 - Ghidra Research Report

**Address(es):** `0x00569760`, `0x00586990`
**Investigation Mode:** exhaustive-slice, downgraded to evidence-synthesis extension because this session exposed no live Ghidra MCP tools
**Claimed Scope:** Classification of `FUN_00569760` and `FUN_00586990`: cell-list batching, dirty-rect output, zone-marker patch/update behavior, active callers, and whether these should be treated as a reusable engine service rather than bridge-only logic.
**Non-Scope:** Re-decoding bridge damage state machines, bridge repair walkers, `CellClass::RecalcAttributes`, `FUN_00584550`, the hierarchical zone graph internals, and exact overlay type names for `g_OverlayTypeClass_Array[+0x3B4/+0x3B8]`.
**Confidence:** Medium overall. The function-body facts are high-confidence prior Ghidra findings from `BRIDGE_PAVEMENT_WALKER_AND_CELLLIST_DISPATCH_GHIDRA_REPORT.md`; this pass could not perform fresh live decompilation because no Ghidra MCP tool was available in the session.
**Active in YR:** Yes for both functions on the observed bridge/destruction/damage paths; conditional for individual branches as stated below.

## 1. Overview

`FUN_00569760` is not a reusable map-update helper. It is a bridge-specific low-bridge pavement/destruction walker that scans up to 30 cells along one bridge axis, mutates bridge pavement/tile state, accumulates some touched cells into a stack `DynamicVector`, computes an optional tactical dirty rectangle, and tail-dispatches the accumulated list to `FUN_00586990`.

`FUN_00586990` is the reusable piece. It consumes a dynamic vector of packed cell coordinates and performs a generic deferred cell refresh: clear the level-0 zone slot, run `CellClass::RecalcAttributes`, then patch the zone graph around cells whose slot remains zero. Current verified callers are bridge/damage/rectangle update paths, but the body itself is not bridge-specialized.

## 2. Class Layout / Key Offsets

| Struct / object | Offset | Type | Purpose | Active in YR |
|-----------------|--------|------|---------|--------------|
| `CellClass` | `+0x24` | packed coord | Walker seed/current coord, also used when building `DynamicVector` entries. Evidence: prior body decode of `FUN_00569760`. | Yes; live bridge destruction/repair-adjacent paths. |
| `CellClass` | `+0x38` | tile id | Compared against theater-loaded bridge/ramp bucket globals. | Yes; gates bridge-specific branches. |
| `CellClass` | `+0x11A` | byte | Bridge/ramp sub-state byte tested for values such as 2, 4, and >4. | Yes; bridge/ramp cells. |
| `CellClass` | `+0x11B` | byte | Height/level adjustment byte; `FUN_00569760` bumps it by 4 on selected ramp-body siblings. | Yes; conditional on the 5-variant ramp branch. |
| `CellClass` | `+0x140` | flags dword | Not directly read/written by either function; modified indirectly through `ToggleBridgePavement` or `RecalcAttributes`. | Yes indirectly. |
| `DynamicVector<CellCoord>` | `+0x04` | pointer | Buffer of packed `i16 x, i16 y` coords consumed by `FUN_00586990`. | Yes. |
| `DynamicVector<CellCoord>` | `+0x10` | int | Count; both passes in `FUN_00586990` iterate `count - 1` down to 0. | Yes. |
| `MapClass` | `+0x6C` | int | Zone cell count, used as upper clamp bound for zone-cache linear index. | Yes. |
| `MapClass` | `+0x70` | pointer | Zone speed/cache table; `FUN_00586990` clears slot 0 for each listed cell. | Yes. |
| `MapClass` | `+0xF4/+0xF8` | int | Map width/height used in the linear zone-cache index formula. | Yes. |

## 3. Core Logic

### `FUN_00569760`

Verified behavior from prior Ghidra body decode:

| Finding | Evidence | Confidence | Active in YR |
|---------|----------|------------|--------------|
| Walks a linear bridge span, not a flood-fill. Phase A advances by `g_DirectionOffsets[param_2 & 7]` and stops at `i == 30` (`0x1E`). | `BRIDGE_PAVEMENT_WALKER_AND_CELLLIST_DISPATCH_GHIDRA_REPORT.md` Section 1.2, Ghidra `0x00569760`. | High from prior Ghidra; medium fresh-session confidence. | Yes; all known callers pass directions 2 or 4. |
| `param_2` is direction/orientation, not damage tier. Known live values are 2 and 4. Other values have no known active caller and fall through to the `dir=4` overlay-spawn arm. | Prior report Section 1.1 and caller graph Section 1.5. | High prior. | Conditional; 2/4 active, other values not active. |
| The stack `DynamicVector` is populated only for the 5-variant ramp `+4` branch, by accumulating a 2x5 cell rectangle around the ramp transition. | Prior report Section 1.4. | High prior. | Conditional; active when a standard YR bridge/ramp reaches that branch. |
| The function computes an output screen rect only when `param_3 != NULL`. It uses tactical world-to-screen projection, level correction, and padding; the dirty-rect call itself is the caller's job. | Prior report Section 1.4 plus audit-log correction for `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`. | High prior. | Conditional; only callers that pass a rect pointer consume it. |
| Zone rebuild is split: local `ValidateBridgeZones` results may trigger full `UpdateBridgeZonesHelper`, while accumulated rectangle cells are sent to `FUN_00586990` for localized recalc/patch. | Prior report Sections 1.3-1.4; `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` for validate/helper semantics. | High prior. | Yes. |
| The function is bridge-specific: it compares against theater bridge bucket globals, calls `ToggleBridgePavement` / `SetOverlayAndPropagate`, spawns bridge destroyed-marker `OverlayClass`es, and uses bridge ramp sub-state bytes. | Prior report Sections 1.2-1.4. | High prior. | Yes. |

Negative classification: `FUN_00569760` should not be modeled as a generic dirty-list helper. The only reusable subpart is its tail dispatch to `FUN_00586990`; the rest is bridge/ramp visual and passability orchestration.

### `FUN_00586990`

Verified behavior from prior Ghidra body decode:

| Finding | Evidence | Confidence | Active in YR |
|---------|----------|------------|--------------|
| Two backward passes over the same coord vector: pass 1 clears zone-cache slot 0 and calls `CellClass::RecalcAttributes`; pass 2 calls `FUN_00584550(coord)` only if slot 0 remains zero. | Prior report Section 2.2, Ghidra `0x00586990`. | High prior. | Yes; called by live bridge/damage paths. |
| Per-cell linear index is `(map_width + map_height + 1) * coord.y + coord.x`, clamped to `[0, zone_cell_count - 1]`. | Prior report Section 2.2. | High prior. | Yes. |
| In-bounds filtering uses `MapClass::Is_Cell_In_Playfield(coord, 1)` before either pass touches cache/cell state. | Prior report Section 2.2. | High prior. | Yes. |
| It does not call `DirtyScreenRect`, `RadarClass::MarkTerrainDirty`, `ValidateBridgeZones`, or `UpdateBridgeZonesHelper`. | Prior report Sections 2.3-2.4. | High prior. | Yes. |
| It does not directly read/write `CellClass+0x140`; `RecalcAttributes` owns the resulting cell flag refresh. | Prior report Section 3. | High prior. | Yes indirectly through `RecalcAttributes`. |
| The body is generic with respect to cell content: it takes `MapClass*` plus a coord list and contains no bridge overlay/ramp constants. | Prior report Section 2; caller graph Section 2.5. | High prior for body, medium for "generic service" classification. | Yes, but observed callers are still bridge/damage-region surfaces. |

Conclusion: `FUN_00586990` is best classified as a generic map cell-list refresh/zone-patch helper that bridge code happens to use heavily. It is reusable in the engine sense, but the observed active caller set is not a broad arbitrary overlay-edit API.

## 4. INI Keys

No INI key is read directly by either function.

| Key / data source | Role | Default / source checked | Active in YR |
|-------------------|------|--------------------------|--------------|
| Theater bridge tile buckets such as `DAT_00ABAD1C`, `DAT_00ABAD30`, `DAT_00AA1028`, `DAT_00ABC1E8` | Runtime globals populated from theater bridge data; `FUN_00569760` compares relative tile ids against them. | Prior report cites `Read_Theater_TileSets_INI @ 0x00545B88` xrefs. | Yes, for stock YR theaters with bridges. |
| `[CombatDamage] DestroyableBridges=` | Does not gate either helper directly, but gates upstream bridge damage/destruction paths. | `ini/rulesmd.ini` line 804 and `ini/rules.ini` line 664: `yes`. | Yes upstream; default enabled. |
| `[CombatDamage] BridgeStrength=` | Does not gate either helper directly; upstream bridge damage RNG. | `ini/rulesmd.ini` line 816 and `ini/rules.ini` line 676: `1500`. | Yes upstream. |
| `BridgeExplosions=` / `BridgeVoxelMax=` | Not consumed here. Included only to rule out dirty-zone helper involvement in debris presentation. | `ini/rulesmd.ini` lines 419 and 529. | Yes elsewhere, no direct effect here. |

## 5. Integration Points

| Caller / callee | Relationship | Evidence | Active in YR |
|-----------------|--------------|----------|--------------|
| `ProcessBridgeDestruction_Low` -> `FUN_00569760` | Three verified low-bridge calls with direction 2 or 4. | Prior report Section 1.5, xrefs `0x00570771`, `0x005707F8`, `0x00570A4E`. | Yes. |
| Unregistered region near `0x0056A080` -> `FUN_00569760` | Four mirror-shape calls, likely high-bridge twin/cousin; function creation was forbidden in the prior read-only session. | Prior report Section 1.5. | Conditional; call sites exist, identity remained medium-confidence. |
| `FUN_00569760` -> `FUN_00586990` | Tail call when accumulated coord vector count is nonzero. | Prior report Sections 1.4 and 2.5, xref `0x0056A048`. | Conditional on the 5-variant ramp branch. |
| `FUN_00568E40` -> `FUN_00586990` | High pavement walker twin tail call. | Prior report Section 2.5, xref `0x00569722`. | Yes, conditional on accumulated cells. |
| `FUN_005868A0` -> `FUN_00586990` | Rectangle-region driver builds a coord list for `[x, x+w) x [y, y+h)` and calls the helper. | Prior report Section 2.5. | Yes. |
| `ProcessBridgeDestruction_{Low,High}` -> `FUN_00586990` | Tail calls after local cell-list accumulation. | Prior report Section 2.5, xrefs `0x00570AA4`, `0x00573FC7`. | Yes. |
| `ProcessBridgeDamageStateMachine_{Low,High}` -> `FUN_00586990` | Four damage-state emit sites. | Prior report Section 2.5. | Yes. |
| `FUN_00586990` -> `CellClass::RecalcAttributes` | Per-cell attribute refresh in pass 1. | Prior report Section 2.2; `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md` separately identifies `0x0047D2B0` as single-cell recalc. | Yes. |
| `FUN_00586990` -> `FUN_00584550` | Incremental zone-graph patch for cells still uncolored after recalc. | Prior report Section 2.2. | Yes. |

Tick-cycle integration: upstream bridge damage/repair/destruction paths mutate bridge/tile state, then run local recalc/zone patch and sometimes full bridge-zone rebuild before the next pathfinding decision. Rust currently approximates this by setting `zones_dirty` and rebuilding `PathGrid`/`ZoneGrid` in bridge orchestrator code, plus a separate overlay dirty-cell pipeline in `app_sim_tick.rs`.

## 6. Current Rust Implementation Status

| Binary feature | Rust surface | Status |
|----------------|--------------|--------|
| Bridge-specific walker state mutation (`FUN_00569760`) | `src/sim/bridge_state/walker.rs`, `src/sim/bridge_state/mod.rs` | Partially represented through dedicated bridge runtime walkers and repair outcomes. Current shape is intentionally bridge/path/render dirty, not a direct copy of the cell-list helper. |
| Accumulated cell-list service (`FUN_00586990`) | No exact shared service. Related surfaces: `src/sim/overlay_grid.rs::take_dirty_cells`, `recalc_overlay_passability`; `src/sim/world/bridge_orchestrator.rs::refresh_bridge_zones_if_dirty`; `src/sim/world/mod.rs::rebuild_zone_grid`. | Missing as a shared map-update abstraction. Rust has separate overlay dirty handling and bridge full-rebuild handling, but no common "cell list -> recalc attributes -> localized zone patch" service. |
| Dirty screen rect output from `FUN_00569760` | No bridge-specific render dirty rectangle channel found; `world_orders.rs` comments note no render-side bridge dirty-cell API yet. | Missing/unchecked. Rust likely redraws through broader app/render state rather than tactical dirty rectangles. |
| Localized zone patch after bridge cell-list | `zone_incremental::try_incremental_update` exists but falls back to full rebuild whenever `resolved_terrain` is present. `bridge_orchestrator::refresh_bridge_zones_if_dirty` builds a new `PathGrid` and calls `rebuild_zone_grid`. | Functionally conservative but not equivalent. It is full-rebuild-oriented for terrain-aware dynamic changes rather than gamemd's per-list patch behavior. |
| Damaged-variant clear during repair | `BridgeRuntimeState::apply_damaged_variant_flood_fill`, `body_cell_repair_state`, walker repair paths. | Implemented in principle; this report does not re-audit exact caller parity. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| `FUN_00569760` bridge-specific classification | verified | Prior Ghidra report Sections 1.1-1.5 | Fresh live Ghidra recheck unavailable this session. |
| `FUN_00569760` screen-rect output | verified | Prior report Section 1.4; audit-log dirty-rect correction for `LAT_RETRIGGER...` | Exact caller-side dirty rect dimensions should be rechecked if implementing tactical dirty rectangles. |
| `FUN_00569760` non-2/4 direction behavior | verified as inactive | Prior report Section 1.1 and 4 | None for standard YR; do not add Rust public API for this theoretical path. |
| `FUN_00586990` two-pass cell-list refresh | verified | Prior report Section 2.2 | Fresh live Ghidra recheck unavailable. |
| `FUN_00586990` caller graph | touched-not-exhausted | Prior report Section 2.5 | Caller graph could have changed if Ghidra function discovery improves around `0x0056A080`; no live MCP to re-run xrefs. |
| `FUN_00584550` zone patch internals | deferred | Prior report explicitly deferred; `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` covers related zone helpers but not this function in this slot. | Dedicated zone-patch helper investigation. |
| Rust bridge walker surfaces | touched-not-exhausted | Codegraph and reads of `src/sim/bridge_state/*`, `src/sim/world/bridge_orchestrator.rs` | This slot did not run tests or audit full bridge-state parity. |
| Rust shared map-update service | verified missing at high level | Codegraph search and reads of `overlay_grid.rs`, `zone_incremental.rs`, `bridge_orchestrator.rs`, `world/mod.rs` | Future design/implementation should decide whether to add a shared service or keep conservative full rebuilds. |
| INI direct reads | verified none | Prior Ghidra report plus INI search in `rules*.ini` / `art*.ini` | None. |
| TS legacy filtering | verified for scoped functions | Prior report Section 4 says no TS-only gates found; current caller set is live bridge code. | Fresh live recheck unavailable. |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Are `0x00569760` and `0x00586990` both bridge-only helpers? -> No. `0x00569760` is bridge-specific; `0x00586990` is a generic cell-list recalc/zone-patch worker used by bridge/damage paths.` (evidence: `BRIDGE_PAVEMENT_WALKER_AND_CELLLIST_DISPATCH_GHIDRA_REPORT.md` Sections 1-2)
- `[RESOLVED] OQ-02 - Does `FUN_00569760` itself dirty the screen? -> It computes an optional screen rect, but caller performs `DirtyScreenRect`; `FUN_00586990` does no render dirty work.` (evidence: prior report Section 1.4 and Section 2.4)
- `[RESOLVED] OQ-03 - Is the list batching a whole-span mechanism? -> Not always. The stack vector is populated only for selected 5-variant ramp branches; otherwise `FUN_00569760` may mutate/dirty without a `FUN_00586990` tail list.` (evidence: prior report Section 1.4)
- `[RESOLVED] OQ-04 - Does `FUN_00586990` perform a full bridge-zone rebuild? -> No. It clears per-cell zone slot 0, runs `RecalcAttributes`, then invokes `FUN_00584550` for still-zero cells. Full `UpdateBridgeZonesHelper` is separate caller-side behavior.` (evidence: prior report Section 2.2; `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-05 - Is `CellClass+0x140` mutated directly here? -> No direct mutation in either helper; mutation is indirect via `ToggleBridgePavement` or `RecalcAttributes`.` (evidence: prior report Section 3)
- `[RESOLVED] OQ-06 - Are direction codes other than 2/4 active in YR? -> No known active caller. All known call sites pass 2 or 4.` (evidence: prior report Sections 1.1 and 4)
- `[RESOLVED] OQ-07 - Is this gated by TS-only fog/subterranean flags? -> No such gate found in prior body/caller decode; caller paths are standard YR bridge damage/repair/destruction code.` (evidence: prior report Section 4)
- `[RESOLVED] OQ-08 - What Rust surface currently owns a comparable dirty-list flow? -> Overlay mutations use `OverlayGrid::take_dirty_cells` plus `recalc_overlay_passability`; bridge mutations use `zones_dirty` plus full path/zone rebuild. No shared cell-list map-refresh service exists.` (evidence: `src/sim/overlay_grid.rs`, `src/app_sim_tick.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/world/mod.rs`)
- `[RESOLVED] OQ-09 - Are INI keys read directly by these helpers? -> No direct reads; relevant bridge tile buckets are theater-loaded globals, while `DestroyableBridges`/`BridgeStrength` gate upstream damage.` (evidence: prior report Section 1.3; local INI search)
- `[DEFERRED] OQ-10 - What exactly does `FUN_00584550` patch at every level?` (category: out-of-scope; reason: target is the cell-list dispatcher, and prior report explicitly treated `FUN_00584550` internals as separate zone-graph work; next-step-if-pursued: run `/re-investigate FUN_00584550 incremental zone patch`)
- `[DEFERRED] OQ-11 - Are the four `0x0056A080` region callers definitively high-bridge destruction?` (category: needs-runtime-debugger; reason: prior read-only Ghidra session could not create a function boundary; next-step-if-pursued: create/define function in a writable Ghidra session and rerun callers)
- `[DEFERRED] OQ-12 - Exact overlay names behind `g_OverlayTypeClass_Array[+0x3B4/+0x3B8]`.` (category: out-of-scope; reason: visual marker identity is not needed to classify the dirty-zone helper; next-step-if-pursued: trace overlay registry indices through theater/overlay type load)
- `[DEFERRED] OQ-13 - Runtime pause/replay/save interaction.` (category: requires-different-system-context; reason: helper bodies are synchronous map mutation helpers; save/replay scheduling requires broader tick/save investigation; next-step-if-pursued: trace callers through save/load and replay command timing)

Remaining uncertainty: the main unresolved condition is the exact breadth of `FUN_00586990` outside bridge-labeled call sites. Its body is generic, and the rectangle wrapper strengthens the service classification, but the verified active caller list in the prior report remains bridge/damage-oriented.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|-------------------|----------|--------------------|-----------------------|--------------------------------|---------------------|------------------|
| A batch of changed cells should be processed as "clear zone slot, recalc attributes, patch/rebuild zones", separate from bridge-specific walker logic. | `FUN_00586990 @ 0x00586990`, prior report Section 2.2. | Missing as a shared service; Rust has overlay dirty handling and bridge full rebuilds separately. | Needed sim-level map-update service, likely near `src/sim/pathfinding/zone_incremental.rs` / `src/sim/world/mod.rs`; must not depend on render/UI. | Introduce or design a shared cell-list map-refresh service that accepts changed coords and owns passability/zone refresh decisions; bridge code should feed changed cells rather than inventing bridge-only zone plumbing. | `test_name: bridge_cell_list_refresh_recalc_then_zone_patch_order` - mutate bridge/overlay cells, assert recalc happens before zone connectivity is queried and only listed cells drive the localized path when supported. | Do not put this in render/app code, and do not hardwire it to bridge overlay IDs; `FUN_00586990` is generic by body. |
| `FUN_00569760` dirty rectangle is render-facing output distinct from `FUN_00586990` zone refresh. | `0x00569760` prior report Section 1.4; `0x00586990` prior report Section 2.4. | Bridge render dirty channel is missing/unchecked; comments note no bridge-specific dirty-cell API. | `src/sim/bridge_state/*` for deterministic changed-cell output; app/render layer for dirty rect consumption. | Keep deterministic sim changed-cell output separate from optional render dirty rectangles. Render can choose broader redraws, but sim must not call render. | `test_name: bridge_pavement_walker_outputs_changed_cells_without_render_dependency` - bridge state mutation records changed cells while compiling without render dependency from `sim`. | Do not make `sim` depend on `render`, `ui`, or tactical screen projection to mimic gamemd dirty rect internals. |
| `FUN_00569760` is bridge-specific and should stay out of a generic map-update API. | `0x00569760` prior report Sections 1.2-1.4. | Current Rust mostly follows this split with bridge-specific walkers, but it lacks a generic cell-list sink. | `src/sim/bridge_state/walker.rs`, `src/sim/bridge_state/mod.rs`. | Bridge walkers should produce bridge-specific mutations and a list of changed cells; shared service should consume the list afterward. | `test_name: bridge_walker_marks_only_touched_cells_for_map_refresh` - final-stage bridge walker returns the exact changed strip/rectangle needed by the refresh service. | Do not collapse bridge damaged-variant flood fill, overlay marker spawn, and zone recalc into one opaque bridge-only rebuild call. |
| `FUN_00586990` does not call full `UpdateBridgeZonesHelper`; full rebuild remains caller-side and conditional. | Prior report Sections 1.4, 2.2, 2.4. | Rust bridge orchestrator currently uses `zones_dirty` to rebuild full `PathGrid`/`ZoneGrid`; terrain-aware incremental updates fall back to full rebuild. | `src/sim/world/bridge_orchestrator.rs::refresh_bridge_zones_if_dirty`; `src/sim/world/mod.rs::rebuild_zone_grid`; `src/sim/pathfinding/zone_incremental.rs`. | Preserve correctness first, but if optimizing, mirror the split: localized cell-list recalc first, full rebuild only when bridge connectivity validation requires it. | `test_name: bridge_repair_local_recalc_does_not_force_full_zone_rebuild_when_connectivity_unchanged` - repair a damaged-but-still-connected visual cell and assert path connectivity remains valid without unnecessary full rebuild when incremental support exists. | Do not assume every visual/pavement dirty cell is a bridge connectivity break; gamemd separates localized patching from full bridge-zone validation. |

### Negative Facts / Do Not Do

- Do not treat `FUN_00569760` as the reusable dirty-zone helper. It is bridge-specific walker/orchestration code.
- Do not implement `FUN_00586990` as "bridge zone rebuild." It is a generic coord-list refresh and localized zone patch; bridge full rebuild is separate.
- Do not wire tactical dirty rectangles into `sim/`. The binary dirty rect is render-facing, while Rust's architecture requires `sim/` to stay below render/app.
- Do not use direction codes other than 2 and 4 as a Rust feature surface. No active YR caller was found.
- Do not skip `RecalcAttributes`-equivalent refresh for cells whose bridge visual state changed just because a full zone rebuild will happen later; gamemd does both local attribute refresh and conditional zone work.

### Remaining Uncertainty

- Live Ghidra MCP was not available in this session, so this report could not cold-spot-check the prior slot-3 decompilation.
- `FUN_00584550` remains outside this slot. Its exact patch radius and data writes should be investigated before implementing a high-fidelity localized zone patch.
- The `0x0056A080` function-region identity remains medium-confidence because read-only Ghidra constraints prevented function creation in the prior report.
- Exact render dirty-rect parity is not required for deterministic sim, but a future renderer-dirty optimization should re-read caller-side `DirtyScreenRect` dimensions before implementation.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_PAVEMENT_WALKER_AND_CELLLIST_DISPATCH_GHIDRA_REPORT.md` - primary prior Ghidra body decode for `0x00569760` and `0x00586990`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` - bridge zone lifecycle, `ValidateBridgeZones`, `UpdateBridgeZonesHelper`, and full-zone rebuild context.
- `C:/Users/enok/Documents/ra2-rust-game-docs/FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md` - current swarm finding that `0x0047D2B0` is single-cell `RecalcAttributes`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md` and `AUDIT_LOG.md` - dirty-rect correction and damaged-variant context.
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/walker.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/zone_incremental.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/overlay_grid.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_sim_tick.rs`
- INI search over `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
