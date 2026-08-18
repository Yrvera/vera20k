# CellClass Substrate Rust Caller Inventory - Source Report

**Target question:** What current Rust call sites read or write the CellClass-like substrate (`OccupancyGrid`, layer selection, passability/placement helpers) for a native CellClass migration?
**Non-goals:** Do not rediscover binary facts; do not edit Rust; do not prove exact `gamemd.exe` `Can_Enter_Cell`, `CellRect::CheckPassability`, or object-list writer contracts beyond labeling parity sensitivity from existing docs.
**Evidence needed to mark COMPLETE:** line-cited inventory covering movement, production spawn, placement, scatter, bridges, AI/miner scans, combat/AOE bridge-layer consumers, save/load rebuild, existing tests, ownership boundaries, and at least one implementation handoff item.
**Stop conditions:** Stop once each required surface has at least one exact source citation and unresolved breadth/threading gaps are listed as uncertainty instead of expanding into new binary or implementation work.

**Investigation mode:** source inventory; no Rust edits; no Ghidra calls.
**Active in YR:** Rust source inventory is not itself a native behavior claim. Parity sensitivity labels cite existing verified research docs.
**Confidence:** High for listed Rust call sites found by focused scans; Medium for full transitive completeness because macros/future feature-gated paths were not exhaustively compiled under all configurations.

## 1. Substrate Owner

Current Rust's CellClass-like dynamic object-list substrate is `src/sim/occupancy.rs`.

`OccupancyGrid` stores a `BTreeMap<(u16,u16), CellOccupancy>` and exposes layer-tagged occupants in per-cell list order (`src/sim/occupancy.rs:45`, `src/sim/occupancy.rs:54`, `src/sim/occupancy.rs:97`, `src/sim/occupancy.rs:98`). It maps structures to append and non-structures to prepend (`src/sim/occupancy.rs:30`, `src/sim/occupancy.rs:36`, `src/sim/occupancy.rs:145`, `src/sim/occupancy.rs:160`). It provides the write API for add/remove/move/subcell update (`src/sim/occupancy.rs:145`, `src/sim/occupancy.rs:182`, `src/sim/occupancy.rs:192`, `src/sim/occupancy.rs:208`) and the read API for `get`, layer emptiness, counts, and contains checks (`src/sim/occupancy.rs:217`, `src/sim/occupancy.rs:222`, `src/sim/occupancy.rs:229`, `src/sim/occupancy.rs:236`).

Parity label: existing object-list research says native `CellClass` has separate ground `+0xE4` and bridge `+0xE8` lists, buildings append, non-buildings prepend, and list order is player-visible in entry, A*, scatter, AoE, nearest-object ties, building lookup, and bridge collapse (`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:12`, `:18`, `:19`, `:25`, `:27`, `:31`, `:43`-`:51`).

The list-layer source is `GameEntity::occupancy_list_layer`, which intentionally uses `on_bridge` rather than locomotor layer and returns no occupancy layer for `Air` or `Underground` (`src/sim/game_entity.rs:576`, `src/sim/game_entity.rs:582`, `src/sim/game_entity.rs:589`, `src/sim/game_entity.rs:596`). `EntityStore` remains the deterministic owning store and is `BTreeMap<u64, GameEntity>` (`src/sim/entity_store.rs:21`, `src/sim/entity_store.rs:23`, `src/sim/entity_store.rs:33`, `src/sim/entity_store.rs:35`).

## 2. Writer Inventory

Map/scenario spawn writes occupancy after inserting and revealing the entity. Structures register all foundation cells; other entities register their origin cell (`src/sim/world/world_spawn.rs:259`, `src/sim/world/world_spawn.rs:260`, `src/sim/world/world_spawn.rs:263`, `src/sim/world/world_spawn.rs:264`, `src/sim/world/world_spawn.rs:270`). Bridge map-spawn also sets `bridge_occupancy` and `on_bridge` before occupancy registration (`src/sim/world/world_spawn.rs:198`, `src/sim/world/world_spawn.rs:199`, `src/sim/world/world_spawn.rs:200`, `src/sim/world/world_spawn.rs:201`).

Normal movement checks runtime entry, then moves the occupancy record on cell crossing and updates infantry subcells after reservation (`src/sim/movement/movement_step.rs:949`, `src/sim/movement/movement_step.rs:1135`, `src/sim/movement/movement_step.rs:1144`, `src/sim/movement/movement_step.rs:1192`, `src/sim/movement/movement_step.rs:1196`, `src/sim/movement/movement_step.rs:1227`). Legacy/alternate movement tick paths still contain equivalent move/update logic (`src/sim/movement/movement_tick.rs:1246`, `src/sim/movement/movement_tick.rs:1247`, `src/sim/movement/movement_tick.rs:1272`, `src/sim/movement/movement_tick.rs:1273`).

Special locomotors also write occupancy: teleport relocation calls `move_entity` (`src/sim/movement/teleport_movement.rs:285`, `src/sim/movement/teleport_movement.rs:309`); low-bridge tube movement relayers by inferred bridge landing layer (`src/sim/movement/tube_movement.rs:219`, `src/sim/movement/tube_movement.rs:317`, `src/sim/movement/tube_movement.rs:318`); tunnel movement removes on dig-in and re-adds on dig-out (`src/sim/movement/tunnel_movement.rs:206`, `src/sim/movement/tunnel_movement.rs:210`, `src/sim/movement/tunnel_movement.rs:266`, `src/sim/movement/tunnel_movement.rs:274`).

Death/despawn writes are split: combat immediate deaths remove from occupancy before entity removal (`src/sim/combat/mod.rs:1003`, `src/sim/combat/mod.rs:1006`, `src/sim/combat/mod.rs:1009`); crush removes victims before deferred entity removal (`src/sim/movement/movement_tick.rs:725`, `src/sim/movement/movement_tick.rs:727`, `src/sim/movement/movement_tick.rs:1565`, `src/sim/movement/movement_tick.rs:1566`); `Simulation::uninit` removes the origin cell only and warns multi-cell structures require caller-side foundation cleanup first (`src/sim/world/mod.rs:813`, `src/sim/world/mod.rs:817`, `src/sim/world/mod.rs:832`, `src/sim/world/mod.rs:835`).

Runtime unlimbo/spawn writers include paradrop passenger placement (`src/sim/aircraft/drop_payload.rs:176`, `src/sim/aircraft/drop_payload.rs:215`, `src/sim/aircraft/drop_payload.rs:227`) and other passenger ejection paths found by `sim.occupancy.add` in `src/sim/passenger.rs:871`, `src/sim/passenger.rs:1025`, and `src/sim/passenger.rs:2161`.

Save/load rebuild is explicit: skipped cache fields are restored in `rebuild_caches_after_load`, then `self.occupancy = OccupancyGrid::rebuild(&self.entities)` (`src/sim/world/mod.rs:944`, `src/sim/world/mod.rs:971`, `src/sim/world/mod.rs:973`). Debug checks can rebuild and compare after a tick when `OCCUPANCY_DEBUG=1` (`src/sim/world/mod.rs:1485`, `src/sim/world/mod.rs:1491`, `src/sim/world/mod.rs:1492`, `src/sim/world/mod.rs:1493`). Native active-object load/reveal order is separately sensitive and not a sorted-ID fallback (`docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md:22`, `:24`, `:40`-`:50`, `:67`-`:73`).

## 3. Reader Inventory By Required Surface

### Movement / `Can_Enter_Cell`

`cell_entry` owns Rust's public entry codes and split phase checks. It now has `CanEnterLayerContext { terrain_layer, object_list_layer, occupancy_bits_layer }` (`src/sim/pathfinding/cell_entry.rs:190`, `src/sim/pathfinding/cell_entry.rs:195`), phase-1 terrain/occupancy checks (`src/sim/pathfinding/cell_entry.rs:386`, `src/sim/pathfinding/cell_entry.rs:412`, `src/sim/pathfinding/cell_entry.rs:415`, `src/sim/pathfinding/cell_entry.rs:427`), and phase-2 blocker classification that iterates the selected layer (`src/sim/pathfinding/cell_entry.rs:520`, `src/sim/pathfinding/cell_entry.rs:535`, `src/sim/pathfinding/cell_entry.rs:555`, `src/sim/pathfinding/cell_entry.rs:594`, `src/sim/pathfinding/cell_entry.rs:595`).

Runtime movement builds `RuntimeCanEnterCellArgs` from current cell, target cell, `on_bridge`, and effective height (`src/sim/movement/movement_occupancy.rs:113`, `src/sim/movement/movement_occupancy.rs:120`, `src/sim/movement/movement_occupancy.rs:123`). It evaluates bridge traversal, chooses object-list layer, then asks `can_enter_layer_context` for split layers (`src/sim/movement/movement_occupancy.rs:127`, `:151`, `:159`, `:176`, `:180`). `check_bridge_traversal` and `can_enter_layer_context` live in `pathfinding/core.rs` (`src/sim/pathfinding/core.rs:506`, `src/sim/pathfinding/core.rs:522`, `src/sim/pathfinding/core.rs:553`, `src/sim/pathfinding/core.rs:594`, `src/sim/pathfinding/core.rs:600`).

Parity label: current runtime argument shape is high sensitivity because bridge research says runtime differs from A* parent sourcing and two-pass list/occupancy split is live in runtime movement (`docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md:260`, `:264`-`:273`, `:300`-`:305`).

### Production Spawn

Production completion selects a spawn cell through `find_spawn_selection_for_owner`, threading `&sim.occupancy` into exact `ExitCoord`, preferred exits, and ring fallback (`src/sim/production/production_spawn.rs:101`, `:104`, `:111`, `:123`, `:128`, `:135`, `:201`, `:208`, `:261`, `:268`, `:311`, `:316`). `cell_available_for_spawn` reads occupancy: infantry need a free subcell; other categories need no ground blockers (`src/sim/production/production_spawn.rs:385`, `:389`, `:406`, `:408`, `:412`, `:414`).

Parity label: existing CellRect validator research explicitly says production spawn has the relevant surface but does not expose native `Find_Nearby` candidate flags or `CheckPassability`/`CheckOccupancy` contracts (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:123`, `:130`).

### Placement

Building placement does not currently read `OccupancyGrid`; it scans `EntityStore` for structure overlap and separately checks terrain/build blockers (`src/sim/production/production_placement.rs:298`, `:302`, `:312`, `:363`, `:372`, `:401`, `:412`, `:416`, `:420`, `:427`). AI placement delegates to production placement by searching for a valid cell and queuing `PlaceReadyBuilding` (`src/sim/ai.rs:78`, `src/sim/ai.rs:118`, `src/sim/ai.rs:350`, `src/sim/ai.rs:382`, `src/sim/ai.rs:394`).

Parity label: CellRect research classifies placement as a separate predicate family, not the same as `CheckPassability`/`CheckOccupancy` (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:129`). Refinery-dock research warns not to treat hidden `AddOccupy`/`RemoveOccupy` as real movement occupancy (`docs/research/REFINERY_DOCK_PAD_CAN_ENTER_CELL_STACK_GHIDRA_REPORT.md:84`-`:92`).

### Scatter

Idle scatter uses occupancy to detect shared cells and candidate availability (`src/sim/movement/scatter.rs:71`, `:76`, `:111`, `:112`, `:137`, `:140`, `:143`). Commanded scatter from a cell uses occupancy in spiral candidate selection (`src/sim/movement/scatter.rs:203`, `:210`, `:255`, `:261`, `:312`, `:318`, `:340`, `:343`, `:345`). Bump/crush scatter of a blocker also checks neighboring occupancy before issuing movement (`src/sim/movement/bump_crush.rs:694`, `:711`, `:715`, `:739`, `:760`, `:775`).

Parity label: object-list ordering research says native `CellClass::Scatter_Objects` is list-order sensitive (`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:47`), while current Rust scatter often uses split `blockers()`/`infantry()` or availability booleans, not a general combined list scan (`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:63`).

### Bridges

Bridge collapse/drop-in relayers bridge deck occupants to ground, clears bridge state, and calls `sim.occupancy.move_entity` with `MovementLayer::Ground` (`src/sim/world/bridge_orchestrator.rs:1344`, `:1364`, `:1374`, `:1380`, `:1391`, `:1397`). Existing bridge tests cover snap-to-ground, over-water survival, under-bridge routing, and bridge snapshot roundtrip (`src/sim/world/world_tests.rs:943`, `:996`, `:1055`, `:1108`, `:1599`, `:1710`, `:1753`).

Bridge pathing data is separate from occupancy: `PathGrid` stores ground/bridge walkability and bridge deck metadata (`src/sim/pathfinding/core.rs:1460`, `:1560`, `:1622`, `:1644`, `:1828`, `:1936`). Do not collapse bridge path-grid state and live cell object-list state into one structure.

### AI / Miner Scans

Miner ore scan composes zone reachability with path-grid and occupancy clearance (`src/sim/miner/miner_system.rs:277`, `:286`, `:299`, `:301`, `:304`, `:308`). The clearance helper rejects non-walkable path cells and non-self ground blockers, but allows infantry and ring-0 self occupancy (`src/sim/miner/miner_system.rs:312`, `:319`, `:325`, `:330`, `:331`, `:336`). Slave miner scan correction reuses the same clearance helper (`src/sim/slave_miner.rs:112`, `:120`, `:122`, `:679`, `:693`, `:710`).

AI's direct dependency is placement/path-grid/production command generation, not occupancy. It passes `path_grid` into building placement and production decisions (`src/sim/ai.rs:64`, `:68`, `:79`, `:118`, `:351`, `:355`), so substrate migration should preserve placement/spawn behavior at the production boundary rather than teaching AI about occupancy internals.

### Combat / AOE Bridge Layer Consumers

`combat_aoe` owns the layer-aware AoE primitive. `AoELayerContext` carries optional occupancy, terrain, and impact Z (`src/sim/combat/combat_aoe.rs:33`, `:35`), and `apply_aoe_damage` selects one object layer when occupancy and terrain are present, then iterates selected-layer occupants (`src/sim/combat/combat_aoe.rs:68`, `:89`, `:90`, `:102`, `:105`). Direct combat/death AoE and superweapon callers thread this context (`src/sim/combat/mod.rs:1041`, `:1050`, `src/sim/combat/mod.rs:1919`, `:1928`, `src/sim/superweapon/lightning_storm.rs:244`, `:254`, `src/sim/superweapon/genetic_converter.rs:88`, `:98`).

Parity label: bridge AoE research says the primitive should select exactly one object list per detonation from impact cell/Z, and current Rust has the pieces but must preserve context threading (`docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md:253`-`:259`, `:269`-`:276`; `docs/research/bridges/05-damage-collapse-repair-cabhut/SUPERWEAPON_BRIDGE_AOE_IMPACT_Z_THREADING_GHIDRA_REPORT.md:140`-`:147`, `:180`-`:182`).

## 4. Existing Test Inventory

Substrate/unit ordering tests exist in `src/sim/occupancy.rs`: `layer_filtering`, `non_buildings_prepend_on_same_layer`, `buildings_append_on_same_layer`, `move_entity_reinserts_with_requested_order`, and `rebuild_uses_category_insertion` (`src/sim/occupancy.rs:359`, `:530`, `:558`, `:682`, `:720`).

Entry/layer tests exist in `cell_entry`: `find_primary_blocker_follows_layer_order`, `split_context_uses_occupancy_bits_layer_for_presence`, `split_context_uses_object_list_layer_for_selected_blockers`, and `split_context_scans_object_list_layer_for_primary_blocker` (`src/sim/pathfinding/cell_entry.rs:1053`, `:1133`, `:1192`, `:1224`). Bridge traversal tests exist in `core_tests` for null-parent reconstruction and diff-4 bridge-list forcing (`src/sim/pathfinding/core_tests.rs:293`, `:368`, `:3121`).

Production/placement tests cover spawn exit behavior and bridge placement rejections (`src/sim/production/production_tests.rs:675`, `:709`, `:765`, `:816`, `:852`; `src/sim/production/production_placement_tests.rs:773`, `:821`, `:924`, `:1118`, `:1200`). Miner scan tests cover tree/path blockers, other-miner occupancy, and ring-0 self allowance (`src/sim/miner/miner_tests.rs:5301`, `:5348`, `:5407`).

Bridge/AoE tests cover bridge-layer damage and superweapon layer threading (`src/sim/combat/combat_aoe.rs:399`, `:422`, `:445`; `src/sim/superweapon/lightning_storm.rs:379`; `src/sim/superweapon/genetic_converter.rs:262`; `src/sim/world/world_tests.rs:943`, `:996`, `:1599`).

## 5. Ownership Boundaries For Migration

`sim/occupancy.rs` is the natural Rust-native substrate owner. It already obeys sim-layer dependency rules and must not depend on render/ui/sidebar/audio/net (`src/sim/occupancy.rs:10`-`:12`). `EntityStore` remains the owner of entity storage; the substrate should index/store live cell membership, not replace `EntityStore` (`src/sim/entity_store.rs:21`-`:35`).

Callers should be migrated by API surface, not by C++ class shape:

1. Writer API: spawn/unlimbo, movement relayer, tunnel/teleport/tube, bridge drop-in, death/despawn, passenger/paradrop, save/load rebuild.
2. Read API: selected-layer iteration, phase-1 occupancy presence, phase-2 primary blocker, spawn availability, scatter availability, miner scan clearance, AoE selected-layer iteration.
3. Non-occupancy placement remains separate until a verified `CellRect`/placement contract says otherwise.

## 6. Implementation Handoff

| Order | Handoff item | Source evidence | Required effect | Acceptance scenario / proposed Rust test |
|---:|---|---|---|---|
| 1 | Introduce a narrow `CellSubstrate`/`CellObjectLists` facade around `OccupancyGrid` without changing storage or callers all at once. | `OccupancyGrid` API at `src/sim/occupancy.rs:145`, `:182`, `:192`, `:217`; list-order parity at `docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:25`-`:35` | Preserve current add/remove/move/rebuild ordering, selected-layer iteration, and `occupancy_list_layer()` semantics. | `cell_substrate_preserves_building_append_nonbuilding_prepend_across_relayer` |
| 2 | Move `cell_entry` and runtime movement to the facade first, because they are the highest-frequency entry consumer and already have split layer contexts. | movement calls at `src/sim/movement/movement_step.rs:949`, `:1144`; split context at `src/sim/pathfinding/cell_entry.rs:195`; runtime args at `src/sim/movement/movement_occupancy.rs:113`-`:124` | No behavior change: `detect_deferred_cell_check`, `classify_occupied_cell_with_layers_and_ignored`, bridge traversal, crush, and subcell allocation keep identical inputs. | `runtime_cell_substrate_split_layers_match_existing_cell_entry_codes` |
| 3 | Migrate secondary read consumers after movement: production spawn, scatter, miner/slave scans, and AoE, while leaving placement's structure scan separate. | spawn `src/sim/production/production_spawn.rs:406`-`:417`; scatter `src/sim/movement/scatter.rs:111`-`:146`; miner `src/sim/miner/miner_system.rs:319`-`:336`; AoE `src/sim/combat/combat_aoe.rs:89`-`:105`; placement separate at `src/sim/production/production_placement.rs:420`-`:440` | Provide equivalent helper methods for selected-layer occupants, ground blockers except self, infantry subcell availability, and spawn availability; do not route placement overlap through dynamic occupancy without verified placement contract. | `secondary_substrate_consumers_keep_spawn_scatter_scan_and_aoe_results` |

## 7. Negative Facts / Do Not Do

- Do not add dependencies from `sim/occupancy.rs` or the new substrate into render/ui/sidebar/audio/net; current module docs explicitly forbid this (`src/sim/occupancy.rs:10`-`:12`; `src/sim/entity_store.rs:13`-`:15`; `src/sim/combat/combat_aoe.rs:12`-`:14`).
- Do not replace `EntityStore` with an ECS or non-deterministic map; `EntityStore` is `BTreeMap<u64, GameEntity>` with deterministic sorted iteration (`src/sim/entity_store.rs:21`-`:35`).
- Do not merge path-grid bridge walkability and live object-list occupancy into one structure; `PathGrid` owns terrain/bridge walkability metadata (`src/sim/pathfinding/core.rs:1460`, `:1560`, `:1828`), while `OccupancyGrid` owns live occupants (`src/sim/occupancy.rs:92`-`:99`).
- Do not treat `AddOccupy`/`RemoveOccupy` hidden occupancy as real movement/building foundation occupancy; refinery dock research says Rust should keep base foundation cells distinct from hidden modifiers (`docs/research/REFINERY_DOCK_PAD_CAN_ENTER_CELL_STACK_GHIDRA_REPORT.md:84`-`:92`).
- Do not generalize `blockers()` then `infantry()` split iteration into a CellClass list-order scan; existing research flags that these helpers may be correct only for boolean/category semantics (`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:61`-`:63`).

## 8. Stale Docs / Replacement Wording Found

`docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md:281`-`:294` says Rust cannot model runtime split list/occupancy layers and points to older single-layer line numbers. Replacement wording:

> Current Rust now has the split-layer substrate needed for this boundary: `movement_occupancy::runtime_can_enter_cell_args` builds runtime target/direction/effective-height inputs, `evaluate_runtime_can_enter_cell` threads `CheckBridgeTraversal`-style results into `CanEnterLayerContext`, and `cell_entry` consumes distinct `terrain_layer`, `object_list_layer`, and `occupancy_bits_layer`. Remaining risk is caller completeness and exact parity of every runtime callsite, not absence of a split-layer Rust representation.

`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:59` cites stale `GameEntity::occupancy_list_layer` line numbers. Replacement wording:

> Current Rust models the normal list-layer source through `GameEntity::occupancy_list_layer`, using `on_bridge` rather than locomotor/path layer for ground/bridge list membership (`src/sim/game_entity.rs:576`, `src/sim/game_entity.rs:582`, `src/sim/game_entity.rs:596`). `OccupancyGrid::rebuild` calls that method and then applies `CellListInsertion::from_category` (`src/sim/occupancy.rs:110`, `src/sim/occupancy.rs:117`, `src/sim/occupancy.rs:128`).

`docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md:256` says superweapon callers that lack verified impact-Z construction still use default context. Replacement wording should reference the follow-up correction:

> Lightning Storm and Genetic Converter now compute `bridge_adjusted_impact_z` and pass occupancy, terrain, and `impact_z` through `AoELayerContext`; Psychic Dominator and Nuclear/MultiMissile damaging paths remain future/unimplemented or bounded follow-ups.

## 9. Remaining Uncertainty

- Full transitive certainty over every feature-gated test/helper path was not proven; inventory was based on focused `rg` scans over `src/sim`.
- `Simulation::uninit` removes only the origin cell and relies on callers to handle multi-cell structures; this is documented in source but still a migration risk until all structure-destruction paths are routed through one lifecycle helper.
- Threading/concurrency was not investigated. Current sim code appears single-threaded around `Simulation`, but this slot did not audit app-level parallelism or save/load threading.
- `ParticleSystem`/gas comments mention future `OccupancyGrid` cell-iteration helpers, but no active substrate consumer was inventoried there in this slot.

## 10. Status

COMPLETE for current Rust caller inventory and first migration handoff. No Rust, INI, or non-research docs were edited.
