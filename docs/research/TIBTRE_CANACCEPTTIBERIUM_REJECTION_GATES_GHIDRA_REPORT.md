# TIBTRE CanAcceptTiberium Rejection Gates - Ghidra Research Report

**Address(es):** `0x00483780` (`CellClass::SpreadTiberium`), `0x004838E0` (`CellClass::CanPlaceTiberium`), `0x00487190` (`CellClass::PlaceTiberium`), `0x0071C730` (`TerrainClass::AI`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** TIBTRE terrain-object ore spawn target acceptance/rejection gates from `TerrainClass::AI -> CellClass::SpreadTiberium(force=1) -> CellClass::CanPlaceTiberium`, compared to current Rust `src/sim/terrain_spawn.rs`.
**Non-Scope:** TIBTRE probability/timing, exact ore type selection, overlay frame randomization beyond whether existing ore is accepted, natural TiberiumClass queue timing, `PlaceTiberium` full side effects, and renderer/minimap behavior.
**Confidence:** High for the claimed rejection-gate slice.
**Active in YR:** Yes. `TerrainClass::AI` calls `SpreadTiberium(1)` for terrain types with `SpawnsTiberium=yes` and `IsAnimated=yes`; stock YR `TIBTRE01/02/03` set both keys in `ini/rulesmd.ini`.

## Working Notes Required By Slot

Target question: What exact `CanPlaceTiberium` rejection gates apply to adjacent cells chosen by TIBTRE `SpreadTiberium(force=1)`, and where does current Rust `can_accept_tiberium` differ?

Non-goals: Do not reinvestigate TIBTRE animation timing, ore type/defaulting, `PlaceTiberium` overlay-frame side effects, natural ore queues, or terrain `Light*` behavior.

Evidence needed to mark COMPLETE: live caller proof from `TerrainClass::AI`, direct `SpreadTiberium` call-site proof that each candidate is prefiltered through `CanPlaceTiberium`, direct `CanPlaceTiberium` gate proof with assembly/decompile context, current Rust scan with line references, and an implementation handoff with concrete test names.

Stop conditions: Stop after all target rejection gates and the existing-ore-vs-empty-cell question are resolved or explicitly deferred; do not modify Rust, INI, or repo docs.

## 1. Overview

TIBTRE tree ore spawning does not use a loose "walkable adjacent cell" predicate. The live YR path chooses a random adjacent start direction, checks up to eight neighbors, and accepts only the first neighbor that passes `CellClass::CanPlaceTiberium`.

The most important Rust-facing correction is that the TIBTRE spread caller does not intentionally grow existing ore. `PlaceTiberium` has an additive/grow branch, but `SpreadTiberium(force=1)` calls `CanPlaceTiberium` first; that helper requires `CellClass+0x44 == -1`, so any existing overlay, including existing ore or gems, rejects the neighbor before `PlaceTiberium(type, 3)` is called.

## 2. Class Layout / Key Offsets

| Owner | Offset | Type / role | Verified use | Active in YR |
|---|---:|---|---|---|
| `TerrainTypeClass` | `+0x2B1` | `SpawnsTiberium` byte | TIBTRE midpoint spawn gate and terrain-object rejection gate | Yes; stock `TIBTRE01/02/03` set `SpawnsTiberium=yes` |
| `TerrainTypeClass` | `+0x2B3` | `IsAnimated` byte | TIBTRE AI activation gate | Yes; stock `TIBTRE01/02/03` set `IsAnimated=yes` |
| `CellClass` | `+0x24` | map coordinate | Passed to playfield test and neighbor lookup | Yes |
| `CellClass` | `+0x38` | `IsoTileTypeClass` index | Final `AllowTiberium` lookup at tile `+0x306` | Yes |
| `CellClass` | `+0x44` | overlay type index | Must equal `-1`; any overlay rejects placement | Yes |
| `CellClass` | `+0xE4` | object-list head | Scanned for live buildings and `SpawnsTiberium` terrain objects when game is active | Yes during normal gameplay |
| `CellClass` | `+0xEC` | YR land type | Indexes land-type Buildable table at `0x0089EA60` | Yes |
| `CellClass` | `+0x11C` | slope index | Must be `0`; sloped targets reject | Yes |
| `CellClass` | `+0x140` | cell flags | Mask `0x500` rejects bridge/rail structural cells | Yes |
| `BuildingClass` | `+0x6C` | health | Building must be alive (`>0`) to block | Yes |
| `BuildingClass` | `+0x520` | `BuildingTypeClass*` | Source for type exception bytes | Yes |
| `BuildingTypeClass` | `+0xC9A`, `+0x1701` | visibility/exception bytes | If either is nonzero, the live building does not reject tiberium placement | Conditional; only matters for buildings on the candidate cell |
| `TerrainClass` | `+0xC8` | `TerrainTypeClass*` | Reads terrain type `SpawnsTiberium` | Yes |
| `IsoTileTypeClass` | `+0x306` | `AllowTiberium` byte | In-range tile must set this byte | Yes; theater INIs contain `AllowTiberium=true` on selected tilesets |

## 3. Core Logic

### 3.1 TIBTRE reaches `SpreadTiberium(force=1)`

`TerrainClass::AI` at `0x0071C730` is the live owner for TIBTRE ore spawn. After the animated TIBTRE reaches the frame midpoint, the code resets the animation state, gets the terrain object's current cell, and calls `CellClass::SpreadTiberium(1)`.

Active in YR: Yes. Stock `rulesmd.ini` has `[TIBTRE01]`, `[TIBTRE02]`, and `[TIBTRE03]` with `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, and `AnimationProbability=.003`.

### 3.2 `force=1` bypasses source spread gates but not target gates

With `param_2 == 1`, `SpreadTiberium` skips the non-force source checks such as the `TiberiumSpreads` bit, source density threshold, source slope, and source object-list emptiness. It still selects a tiberium type, rolls `RandomRanged(0,7)`, visits neighbor indices `(start + i) & 7` for `i=0..7`, fetches the neighbor cell, and requires `CellClass::CanPlaceTiberium` before calling `CellClass::PlaceTiberium(tib_type, 3)`.

Evidence: decompile `0x00483780`; assembly context `0x0048389A..0x004838C5` shows the neighbor coordinate/local stack argument pushed before `CALL 0x004838E0`; the tiberium type/index is pushed later for the successful `PlaceTiberium(type, 3)` call.

Active in YR: Yes. This is the direct target-selection path reached by TIBTRE `SpreadTiberium(1)`.

### 3.3 `CanPlaceTiberium` rejection gates for TIBTRE candidates

All gates below must pass. Any failure rejects that neighbor and `SpreadTiberium` continues to the next wrapped direction until all eight fail or one succeeds.

| Gate | Binary behavior | Evidence | Active in YR |
|---|---|---|---|
| Playfield bounds | `MapClass::Is_Cell_In_Playfield(&cell->coord, 1)` must return true | decompile `0x004838E0`; assembly `0x004838E4..0x004838F6` | Yes |
| Cell flags | `(CellClass+0x140 & 0x500) == 0`; mask is implemented as `TEST AH,0x5` after loading flags | assembly `0x004838FC..0x00483905` | Yes |
| Live visible building | If game is active, scan object list. RTTI `6` with health `>0` rejects unless type byte `+0xC9A` or `+0x1701` is nonzero | assembly `0x00483918..0x0048395A` | Yes during normal gameplay; conditional on a building occupying the candidate |
| Spawning terrain object | If game is active, scan object list. RTTI `0x24` rejects when `TerrainTypeClass+0x2B1` is nonzero | assembly `0x00483969..0x0048399A` | Yes during normal gameplay; this blocks TIBTRE cells and other `SpawnsTiberium` terrain objects |
| Land-type Buildable | `0x0089EA60 + land_type * 0x24` must be nonzero | assembly `0x0048399C..0x004839AE` | Yes |
| No existing overlay | `CellClass+0x44` must equal `-1` | assembly `0x004839B0..0x004839B4` | Yes |
| Flat cell | `CellClass+0x11C` must equal `0` | assembly `0x004839B6..0x004839BE` | Yes |
| Tile `AllowTiberium` | If `IsoTileTypeIndex` is in range, `IsoTileTypeClass+0x306` must be nonzero. Out-of-range tile indices pass this final fallback | assembly `0x004839C0..0x004839E3` | Yes; theater INIs have explicit `AllowTiberium=true` entries |

### 3.4 Existing ore is rejected by TIBTRE target selection

The existing-ore question has two different answers depending on the entry point:

- TIBTRE `SpreadTiberium(force=1)`: existing ore is rejected as a target because existing ore is an overlay and `CanPlaceTiberium` requires `CellClass+0x44 == -1`.
- Direct `PlaceTiberium(tib_type, density)`: if called without the `SpreadTiberium` prefilter and `CanPlaceTiberium` returns false, the function has a grow-existing branch for matching tiberium under additional gates.

Active in YR: Yes for both code paths, but only the first bullet applies to TIBTRE target selection. This corrects older wording that implied TIBTRE can grow existing adjacent ore.

### 3.5 Relationship to `PathGrid::is_walkable`

Earlier Rust `can_accept_tiberium` used `PathGrid::is_walkable` as the main terrain predicate, rejected cells with another terrain spawner, rejected gems, and allowed existing ore. As of the 2026-05-24 TIBTRE implementation pass, the terrain-spawn path uses `ResolvedTerrainGrid` validation for flat slope, base-buildable terrain, bridge/rail flags, and `AllowTiberium`, and it rejects existing resources/overlays when the overlay grid is available. It is still not a full `CanPlaceTiberium` equivalent because the binary also checks live object types, especially the type-aware building exception and any live terrain object whose type has `SpawnsTiberium=yes`.

Active in YR: Yes. These are live target gates on the TIBTRE path, not TS legacy.

## 4. INI Keys

| Key | Location | Stock YR value | Binary effect | Active in YR |
|---|---|---|---|---|
| `SpawnsTiberium` | `[TIBTRE01/02/03]` in `rulesmd.ini` | `yes` | Enables TIBTRE midpoint spawn and makes any such terrain object reject target placement on its own cell | Yes |
| `IsAnimated` | `[TIBTRE01/02/03]` in `rulesmd.ini` | `yes` | Required for the TIBTRE AI spawn path | Yes |
| `AnimationRate` | `[TIBTRE01/02/03]` in `rulesmd.ini` | `3` | Timing only; not part of this rejection-gate slice | Yes |
| `AnimationProbability` | `[TIBTRE01/02/03]` in `rulesmd.ini` | `.003` | Spawn-attempt probability only; not part of this rejection-gate slice | Yes |
| `AllowTiberium` | theater `[TileSetNNNN]` entries | present as `true` on selected tilesets | In-range tile must have `IsoTileTypeClass+0x306 != 0` to accept placement | Yes |

## 5. Integration Points

| Point | Evidence | Contract | Active in YR |
|---|---|---|---|
| TIBTRE AI caller | `TerrainClass::AI @ 0x0071C730` | Calls `SpreadTiberium(1)` at animation midpoint for `SpawnsTiberium && IsAnimated` terrain | Yes |
| Neighbor loop | `CellClass::SpreadTiberium @ 0x00483780`; assembly `0x00483823..0x004838C5` | Random start, wrapped 8-neighbor scan, first `CanPlaceTiberium` success wins | Yes |
| Target validation | `CellClass::CanPlaceTiberium @ 0x004838E0` | Eight-gate target predicate, including no existing overlay | Yes |
| Placement after validation | `CellClass::PlaceTiberium @ 0x00487190` | Called only after target validation succeeds in this path | Yes |
| Current Rust TIBTRE tick | `src/sim/world/mod.rs:1556` | Runs `tick_terrain_spawners_stateful` after `tick_ore_growth` | Current Rust behavior |
| Current Rust target helper | `src/sim/terrain_spawn.rs` | Uses resource/overlay rejection plus resolved-terrain gates for flat/base-buildable/bridge/AllowTiberium; still lacks live object-list gates | Current Rust behavior |

## 6. Current Rust Implementation Status

As of the 2026-05-24 TIBTRE implementation pass, current Rust `src/sim/terrain_spawn.rs` covers several previously missing target gates but is still narrower than GameMD's full `CanPlaceTiberium` object-list scan:

- `try_spawn_ore` checks up to eight adjacent cells and calls `can_accept_tiberium` before placement.
- `can_accept_tiberium` rejects other known terrain spawner cells.
- It rejects existing resource nodes.
- It rejects any existing overlay when `OverlayGrid` is present.
- It uses resolved terrain to reject non-flat cells, base-build-blocked cells, bridge/rail flagged cells, and tiles whose final resolved tile lacks `AllowTiberium`.
- `PathGrid` is now only a fallback bounds check when resolved terrain is not available.

Rust deltas against verified YR target gates:

- Missing exact playfield dimensions when neither `ResolvedTerrainGrid` nor `PathGrid` is passed.
- `CellClass+0x140 & 0x500` is approximated through resolved bridge flags; the exact bit semantics were not rederived in this slot.
- Missing live-building gate and the two binary exception bytes.
- Spawner terrain rejection is partially present through `terrain_spawners`, but only for seeded `SpawnsTiberium && IsAnimated` spawners, not a general object-list `SpawnsTiberium` terrain-object query.
- Land-type Buildable is approximated with `ResolvedTerrainCell::base_build_blocked`; this should be kept under test against any future exact land table model.
- Existing ore/resource and overlay occupancy are rejected in the current overlay-grid path.
- Flat-slope gate is implemented through `slope_type == 0`.
- Theater `AllowTiberium` is parsed/exposed and checked through `ResolvedTerrainCell::allows_tiberium`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TerrainClass::AI` TIBTRE caller | verified | Ghidra `0x0071C730`; `rulesmd.ini [TIBTRE01/02/03]` | none for liveness |
| `SpreadTiberium(force=1)` source-gate bypass | verified | Ghidra `0x00483780` | none for target gate slice |
| `SpreadTiberium` target prefilter before placement | verified | assembly `0x0048389A..0x004838C5` | exact direction labels remain out of scope |
| `CanPlaceTiberium` playfield gate | verified | `0x004838E0`, assembly `0x004838E4..0x004838F6` | none |
| `CanPlaceTiberium` cell flags mask `0x500` | verified | `0x004838E0`, assembly `0x004838FC..0x00483905` | semantic names for each bit not reverified here |
| `CanPlaceTiberium` building gate | verified | `0x004838E0`, assembly `0x00483918..0x0048395A` | exception byte identities and stock users covered by `TIBTRE_BUILDING_EXCEPTION_BYTES_0XC9A_0X1701_GHIDRA_REPORT.md` |
| `CanPlaceTiberium` `SpawnsTiberium` terrain-object gate | verified | `0x004838E0`, assembly `0x00483969..0x0048399A` | none |
| `CanPlaceTiberium` land Buildable gate | verified | `0x004838E0`, assembly `0x0048399C..0x004839AE` | exact land-type table initialization not reverified here |
| `CanPlaceTiberium` no-overlay gate | verified | `0x004838E0`, assembly `0x004839B0..0x004839B4` | none |
| `CanPlaceTiberium` slope gate | verified | `0x004838E0`, assembly `0x004839B6..0x004839BE` | none |
| `CanPlaceTiberium` tile `AllowTiberium` gate | verified | `0x004838E0`, assembly `0x004839C0..0x004839E3`; theater INI grep | implemented in current resolved-terrain validation |
| `PlaceTiberium` grow-existing branch | touched-not-exhausted | Ghidra `0x00487190` | full grow branch side effects out of scope |
| Current Rust `can_accept_tiberium` | verified-source-scan | `src/sim/terrain_spawn.rs` | live object-list gates remain |
| Current Rust placement helper | verified-source-scan | `src/sim/terrain_spawn.rs` | native variant/queue side effects belong to the PlaceTiberium report |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - What is the investigation mode? -> exhaustive-slice for TIBTRE target rejection gates only` (evidence: user target and report header)
- `[RESOLVED] OQ2 - Is `CanPlaceTiberium` on the live TIBTRE YR path? -> yes, `TerrainClass::AI` calls `SpreadTiberium(1)`, and each neighbor candidate calls `CanPlaceTiberium` before placement` (evidence: `0x0071C730`, `0x00483780`, assembly `0x0048389A..0x004838C5`)
- `[RESOLVED] OQ3 - Does force bypass target validation? -> no; it bypasses non-force source spread gates only` (evidence: `0x00483780`)
- `[RESOLVED] OQ4 - What are the playfield/bounds semantics? -> candidate must pass `MapClass::Is_Cell_In_Playfield(coord, 1)`; out-of-playfield rejects` (evidence: `0x004838E0`, assembly `0x004838E4..0x004838F6`)
- `[RESOLVED] OQ5 - Which cell flags reject? -> `CellClass+0x140 & 0x500` rejects` (evidence: `0x004838E0`, assembly `0x004838FC..0x00483905`)
- `[RESOLVED] OQ6 - Do buildings reject? -> live RTTI 6 building with health > 0 rejects unless type byte `+0xC9A` or `+0x1701` is nonzero` (evidence: `0x004838E0`, assembly `0x00483918..0x0048395A`)
- `[RESOLVED] OQ7 - Do terrain objects reject? -> RTTI `0x24` terrain object rejects only when its type has `SpawnsTiberium` byte nonzero` (evidence: `0x004838E0`, assembly `0x00483969..0x0048399A`)
- `[RESOLVED] OQ8 - Does land type matter? -> yes, land-type Buildable table byte at `0x0089EA60 + land_type*0x24` must be nonzero` (evidence: `0x004838E0`, assembly `0x0048399C..0x004839AE`)
- `[RESOLVED] OQ9 - Does existing overlay matter? -> yes, `CellClass+0x44` must be `-1`, so any overlay rejects` (evidence: `0x004838E0`, assembly `0x004839B0..0x004839B4`)
- `[RESOLVED] OQ10 - Can TIBTRE grow existing adjacent ore? -> no through this caller; existing ore has an overlay and fails `CanPlaceTiberium` before `PlaceTiberium(type,3)` is called` (evidence: `0x00483780`, `0x004838E0`)
- `[RESOLVED] OQ11 - Can direct `PlaceTiberium` grow existing ore? -> yes conditionally through its separate false-`CanPlaceTiberium` branch, but that is not selected by TIBTRE `SpreadTiberium` target filtering` (evidence: `0x00487190`)
- `[RESOLVED] OQ12 - Do slopes reject? -> yes, `CellClass+0x11C` must be zero` (evidence: `0x004838E0`, assembly `0x004839B6..0x004839BE`)
- `[RESOLVED] OQ13 - Does theater tile data reject? -> yes, in-range `IsoTileTypeClass+0x306 AllowTiberium` must be nonzero; invalid/out-of-range tile indices pass fallback` (evidence: `0x004838E0`, assembly `0x004839C0..0x004839E3`; theater INI `AllowTiberium=true` grep)
- `[RESOLVED] OQ14 - Does current Rust match existing overlay behavior? -> current Rust now rejects existing resources and, when `OverlayGrid` is present, any existing overlay before TIBTRE placement.` (evidence: `src/sim/terrain_spawn.rs`)
- `[RESOLVED] OQ15 - Does current Rust parse `AllowTiberium`? -> yes as of the 2026-05-24 implementation pass; it is exposed through resolved terrain and checked during terrain-spawn validation.` (evidence: `src/map/theater.rs`, `src/map/resolved_terrain.rs`, `src/sim/terrain_spawn.rs`)
- `[RESOLVED] OQ16 - Is `PathGrid::is_walkable` equivalent to the binary gate chain? -> no; the current implementation no longer treats it as equivalent and uses resolved terrain for the available tile gates, but live object-list gates still remain.` (evidence: `src/sim/terrain_spawn.rs`, `0x004838E0`)
- `[DEFERRED] OQ17 - What exact gameplay labels correspond to the two bits in mask `0x500`?` (category: bounded-cost-too-high; reason: not required to prove the mask rejects target cells; next-step-if-pursued: trace writers/readers of `CellClass+0x140` bits `0x100` and `0x400`)
- `[DEFERRED] OQ18 - Do any stock YR buildings set the two exception bytes?` (category: out-of-scope; reason: this target is gate behavior, not a full building-type census; next-step-if-pursued: map `BuildingTypeClass+0xC9A/+0x1701` to INI readers and scan stock rules)
- `[DEFERRED] OQ19 - Which land-type table initialization path sets `0x0089EA60` for every land type?` (category: out-of-scope; reason: prior validation docs cover the table mapping and this slot verified the consumer; next-step-if-pursued: inspect `RulesClass::ReadSpeedTypeLandTypeTable`)

Adversarial corner-case answers:

- If all eight adjacent cells contain ore, TIBTRE places nothing because all eight fail the no-overlay gate.
- If a candidate cell is ground-walkable but sloped, TIBTRE rejects it.
- If a candidate cell is otherwise clear but its tile's `AllowTiberium` byte is false, TIBTRE rejects it.
- If a candidate cell has a live visible building, TIBTRE rejects it; if one of the two type exception bytes is set, that specific building gate does not reject.
- If a candidate cell contains a non-spawning terrain object, this helper does not reject it on the terrain-object gate; only `SpawnsTiberium` terrain objects reject by that branch, though other cell attributes may still reject.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| TIBTRE target cells are filtered by `CanPlaceTiberium`, including no existing overlay | `0x00483780`, `0x004838E0`, assembly `0x0048389A..0x004838C5` and `0x004839B0..0x004839B4` | implemented for current resource/overlay-grid placement path | `src/sim/terrain_spawn.rs` | Preserve existing overlay/resource rejection for this caller; TIBTRE should only place on empty accepted cells | `tibtre_spread_rejects_existing_ore_neighbors` | Do not treat `PlaceTiberium`'s grow branch as reachable through TIBTRE target selection |
| Cell flags mask `0x500` rejects candidates | `0x004838E0`, assembly `0x004838FC..0x00483905` | approximated through resolved bridge flags | terrain spawn validation surface plus resolved terrain/bridge data | Keep checking equivalent bridge/rail structural-cell rejection before placement | `tibtre_spread_rejects_bridge_flagged_cell_even_if_path_walkable` | Do not assume walkability captures cell flag semantics |
| Live visible buildings reject candidates, with two binary exception bytes | `0x004838E0`, assembly `0x00483918..0x0048395A` | unchecked by `can_accept_tiberium`; may be indirectly blocked by path grid only | terrain spawn validation plus occupancy/building type metadata | Reject live building-occupied cells, preserving exception semantics if those bytes are modeled | `tibtre_spread_rejects_live_building_cell` | Do not silently use all occupancy as exact parity if invisible/exception buildings become modeled |
| `SpawnsTiberium` terrain objects reject candidates | `0x004838E0`, assembly `0x00483969..0x0048399A` | partial: Rust rejects only cells in `terrain_spawners`, seeded by `SpawnsTiberium && IsAnimated` | `ProductionState::terrain_spawners`, terrain object state/index | Reject any live terrain object whose type has `SpawnsTiberium=yes`, independent of animation seeding policy | `tibtre_spread_rejects_any_spawns_tiberium_terrain_object_cell` | Do not key the rejection solely to the active spawner map if non-animated spawners or future lifecycle states exist |
| Land-type Buildable and flat slope are separate target gates | `0x004838E0`, assembly `0x0048399C..0x004839BE` | implemented/approximated through resolved terrain (`base_build_blocked`, `slope_type == 0`) | resolved terrain cell data | Preserve binary-equivalent land Buildable and flat-slope rejection | `tibtre_spread_rejects_rough_impassable_or_slope_even_when_not_resource_occupied`; `tibtre_spread_accepts_flat_buildable_clear_candidate` | Do not infer this only from movement passability |
| Tile `AllowTiberium` is a final in-range gate | `0x004838E0`, assembly `0x004839C0..0x004839E3`; theater INI grep | implemented through theater parser and resolved terrain | `src/map/theater.rs`, `ResolvedTerrainCell`, terrain spawn validation | Preserve per-tile `AllowTiberium` and reject false in-range tiles | `tibtre_spread_rejects_tile_without_allow_tiberium` | Do not collapse `AllowTiberium` into land type; binary checks both |
| Existing gems, walls, crates, and any other overlay reject as overlays, not just as resource types | `0x004838E0`, no-overlay gate at `0x004839B0..0x004839B4` | implemented when `OverlayGrid` is present | `OverlayGrid` plus terrain spawn validation | Candidate acceptance should consult actual overlay presence, not just `ResourceNode` type | `tibtre_spread_rejects_wall_or_crate_overlay_candidate` | Do not special-case only ore/gems; the binary checks `OverlayTypeIndex == -1` |

### Negative Facts / Do Not Do

- Do not implement TIBTRE acceptance as `PathGrid::is_walkable` plus resource-type checks. The binary uses a distinct `CanPlaceTiberium` gate chain.
- Do not allow TIBTRE to add density to existing adjacent ore. The selected target must have no overlay.
- Do not treat all terrain objects as blockers for this specific gate; the verified terrain-object branch rejects `SpawnsTiberium` terrain objects.
- Do not ignore theater `AllowTiberium`; it is checked after land type and slope.
- Do not rely on `PlaceTiberium` grow-existing behavior to describe TIBTRE spread target selection. That branch exists but is not reached by a neighbor that fails `CanPlaceTiberium` in `SpreadTiberium`.

## 10. Remaining Uncertainty

- Exact semantic names for `CellClass+0x140` bits `0x100` and `0x400` were not rederived in this slot; the rejecting mask `0x500` is verified.
- Full stock-YR census of the two building exception bytes was not performed. The gate behavior is verified; exact content use remains a follow-up if implementation wants more than occupancy approximation.
- The land-type Buildable table consumer is verified here, but the complete table initialization path was not rechecked.
- Exact direction-table cardinal label order remains out of scope; the wrapped eight-neighbor index scan is verified.

## Sources

- Ghidra decompile: `TerrainClass::AI @ 0x0071C730`
- Ghidra decompile: `CellClass::SpreadTiberium @ 0x00483780`
- Ghidra decompile: `CellClass::CanPlaceTiberium @ 0x004838E0`
- Ghidra decompile: `CellClass::PlaceTiberium @ 0x00487190`
- Ghidra assembly contexts: `0x0048389A..0x004838C5`, `0x004838E4..0x004839E3`
- Prior docs: `TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md`, `CELL_VALIDATION_TIBERIUM_PLACEMENT_REPORT.md`, `PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md`, `ORE_TIBERIUM_RNG_CLASSIFICATION_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini`, theater INIs containing `AllowTiberium`
- Rust scanned: `src/sim/terrain_spawn.rs`, `src/sim/pathfinding/core.rs`, `src/map/resolved_terrain.rs`, `src/sim/world/mod.rs`
