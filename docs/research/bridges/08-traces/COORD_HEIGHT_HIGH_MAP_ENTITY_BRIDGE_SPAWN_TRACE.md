# COORD_HEIGHT_HIGH_MAP_ENTITY_BRIDGE_SPAWN_TRACE

**Scenario:** A map-placed unit/object line marks the bridge/deck placement field as literal `High=yes` on a bridge/deck cell.

**Scope:** map entity parsing -> terrain bridge deck lookup -> spawn Z / bridge occupancy / `on_bridge` state -> cached screen coordinate -> first rendered unit sprite anchor and selected non-building status bracket/health bar anchor.

**Status:** COMPLETE. This trace did not edit Rust, INI, or in-repo docs. Ghidra MCP use was read-only.

## Pipeline

`[Units]` map line -> `MapEntity.high` -> `Simulation::spawn_from_map_with_resolved` -> `GameEntity::new` cached screen coords -> bridge occupancy / locomotor layer -> `OccupancyGrid::add` -> `build_unit_instances` -> `build_unit_status_bg_instances` / `build_unit_status_fill_instances`.

## Concrete Input

The concrete input is a `[Units]` placement equivalent to:

`0=Americans,MTNK,256,5,5,64,Guard,None,0,-1,yes,-1,false,false`

The target cell is assumed to be a resolved bridge/deck cell in our engine (`bridge_walkable=true`, `bridge_deck_level=N`). The literal field value under trace is `yes`, not numeric `1`.

## Stage Results

| Stage | Verdict | Our output | gamemd output | Evidence |
|---|---:|---|---|---|
| Map field parse | FAIL | `high=true` for `"yes"` | `High` token parsed by `atoi`; `"yes"` -> `0` | `src/map/entities.rs:288`, `src/map/entities.rs:293`; Ghidra `ScenarioClass__Read_Units_Section @ 0x00743270`, `CRT__atoi @ 0x007C9B72` |
| Bridge deck lookup | FAIL | Because `high=true`, Rust queries `resolved_terrain.cell(5,5)` and accepts `bridge_walkable` | Because parsed value is `0`, gamemd does not enter the High branch for `"yes"` | `src/sim/world/world_spawn.rs:49`, `src/sim/world/world_spawn.rs:52`; Ghidra `0x00743270` |
| Spawn Z | FAIL | `position.z = bridge_deck_level` | `local_8c` remains `0` before placement; no bridge height added for `"yes"` | `src/sim/world/world_spawn.rs:66`, `src/sim/world/world_spawn.rs:116`; Ghidra `0x00743270` |
| `on_bridge` / bridge occupancy | FAIL | `bridge_occupancy=Some`, `on_bridge=true`, locomotor layer Bridge | object `OnBridge` byte remains false for `"yes"` | `src/sim/world/world_spawn.rs:177`, `src/sim/world/world_spawn.rs:191`; Ghidra `0x00743270`, `CellClass__AddContent @ 0x0047E8A0` |
| Occupancy list layer | FAIL | Unit inserted into `MovementLayer::Bridge` | `CellClass::AddContent` receives false bridge-layer argument, so ground list (`+0xE4`) | `src/sim/world/world_spawn.rs:234`, `src/sim/world/world_spawn.rs:263`; Ghidra `0x0047E8A0` |
| Cached screen coordinate | FAIL | `GameEntity::new` calls `lepton_to_screen(..., z=bridge_deck_level)`; for `(5,5,z=N)`: `(0, 165 - 15N)` | no bridge height from `High=yes`; first screen point is ground-layer placement path | `src/sim/game_entity.rs:307`, `src/util/lepton.rs:136`; Ghidra `0x00743270` |
| First unit sprite anchor | FAIL | `build_unit_instances` uses cached raised `position.screen_x/screen_y` | gamemd draws from the non-raised object location for `"yes"` | `src/app_instances/units.rs:140`, `src/app_instances/units.rs:238`; Ghidra `TechnoClass__DrawHealthBar @ 0x006F64A0` confirms draw extras consume passed screen location |
| Selected unit bracket / health bar anchor | FAIL | PIPBRD/fill anchor uses raised `interpolated_screen_position_entity` | gamemd non-building selected bar uses the unraised `pLocation` for `"yes"` | `src/app_ui_overlays.rs:397`, `src/app_ui_overlays.rs:426`, `src/app_ui_overlays.rs:491`, `src/app_ui_overlays.rs:518`; Ghidra `0x006F64A0` |
| Numeric equality for `High=1` bridge deck | UNCHECKED | Rust would use `bridge_deck_level` and screen `165 - 15N` for `(5,5)` | gamemd would set `Z = CellClass::GetGroundHeight + DAT_00B1D0AC` | Exact same-cell gamemd screen pixels were not captured; adjacent finding only |

## Failures

### 1. Literal `High=yes` is parsed as true in Rust but false in gamemd

The player-visible result is a unit appearing on the bridge deck in Rust while gamemd keeps it on the ground layer for the literal token `yes`.

Root cause: Rust uses a permissive bool parser accepting `"yes"`, `"true"`, `"on"`, and `"1"`. `gamemd.exe` reads the map unit High field with `atoi`, so only numeric nonzero strings are true. `CRT__atoi` skips whitespace, optional sign, then consumes digits; a non-digit first character returns `0`.

Active in standard YR: Yes. `ScenarioClass__Read_Units_Section @ 0x00743270` is part of the standard scenario load path documented from `ScenarioClass__Full_Init`; this is not a dormant TS-only path.

### 2. The parse mismatch cascades into Z, bridge occupancy, object list layer, and first frame anchors

Because our parsed value is true, `spawn_from_map_with_resolved` resolves the bridge cell and writes:

- `position.z = bridge_deck_level`
- `bridge_occupancy = Some(BridgeOccupancy { deck_level })`
- `on_bridge = true`
- `LocomotorState.layer = MovementLayer::Bridge`
- occupancy grid entry on the Bridge layer

gamemd does none of that for literal `High=yes`; the High branch is skipped because the parsed integer is `0`.

## Adjacent Findings

- For numeric `High=1`, gamemd does set `object+0x8C OnBridge` and raises local Z by `CellClass::GetGroundHeight + DAT_00B1D0AC` before placement. I did not mark parity PASS because I did not capture same-cell gamemd pixels and Rust bridge deck levels use level units, not directly confirmed leptons.
- The Rust unit/infantry test data intentionally includes `true` in field 10/11. That is also nonnumeric and would parse false in gamemd. This is adjacent to the scenario but was not expanded into a test audit.

## Verdict Tally

PASS: 0 | FAIL: 8 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Evidence Index

- Rust parse: `src/map/entities.rs:183`, `src/map/entities.rs:288`, `src/map/entities.rs:293`
- Rust spawn: `src/sim/world/world_spawn.rs:49`, `src/sim/world/world_spawn.rs:66`, `src/sim/world/world_spawn.rs:177`, `src/sim/world/world_spawn.rs:191`, `src/sim/world/world_spawn.rs:234`, `src/sim/world/world_spawn.rs:263`
- Rust cached coords: `src/sim/game_entity.rs:307`, `src/util/lepton.rs:136`
- Rust first render anchors: `src/app_instances/units.rs:140`, `src/app_instances/units.rs:238`, `src/app_ui_overlays.rs:397`, `src/app_ui_overlays.rs:426`, `src/app_ui_overlays.rs:491`, `src/app_ui_overlays.rs:518`
- Ghidra read-only: `ScenarioClass__Read_Units_Section @ 0x00743270`, `CRT__atoi @ 0x007C9B72`, `CellClass__AddContent @ 0x0047E8A0`, `ObjectClass__Mark_Occupation @ 0x007441B0`, `ObjectClass__GetHeight @ 0x005F5F40`, `TechnoClass__DrawHealthBar @ 0x006F64A0`
