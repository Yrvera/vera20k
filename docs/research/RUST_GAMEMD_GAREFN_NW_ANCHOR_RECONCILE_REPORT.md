# RUST_GAMEMD_GAREFN_NW_ANCHOR_RECONCILE - Research Report

**Address(es):** no new Ghidra decompile in this slot; binary evidence comes from existing verified reports: `BuildingClass::Receive_Radio @ 0x0043C2D0`, `BuildingClass::GetDockCellForObject @ 0x0044EFB0`.
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Compare current Rust `GameEntity.position.rx/ry` for a placed `GAREFN` against the gamemd NW/foundation-origin anchor used by refinery dock helper reports.
**Non-Scope:** Full refinery radio state machine, queued-miner handoff, post-unload exit reachability, and whether every Rust call site uses the correct dock-vs-wait cell.
**Confidence:** High for anchor equivalence from current Rust source plus existing binary reports; Medium for absence of a GAREFN-specific Rust test asserting the placed building entity's own `rx/ry`.
**Active in YR:** Yes for the gamemd dock helper path; standard `GAREFN` is `Refinery=yes`, `DockUnload=yes`, and `NumberOfDocks=1` in `ini/rulesmd.ini`.

## 1. Overview

The current Rust representation uses `GameEntity.position.rx/ry` for building placement as the top-left/NW foundation cell, not the visual center or dock pad. Existing binary reports describe the gamemd refinery dock helper anchor as `GetMapCell()` / `building.MapCell` / vtable `+0x1B8`, also documented as the top-left foundation cell. Therefore, for a placed `GAREFN`, Rust `position.rx/ry` corresponds to the gamemd NW/foundation origin anchor used by the verified refinery dock helper reports.

The historical trace warning that Rust might be one cell east was blocked on this anchor proof. With the anchor reconciled, any difference between `QueueingCell=4,1` and gamemd `Receive_Radio` case `0x0E` `+3,+1` is a dock-helper/call-site question, not an anchor-definition question.

## 2. Class Layout / Key Offsets

| Field / concept | Rust / gamemd representation | Evidence | Active in YR |
|---|---|---|---|
| Rust building anchor | `GameEntity.position.rx/ry` passed from placement/spawn | `src/sim/production/production_placement.rs:180-248`; `src/sim/world/world_spawn.rs:280-337`; `src/sim/game_entity.rs:281-319` | Yes; used for all spawned entities, including buildings |
| Rust footprint origin | footprint tests use `e.position.rx..rx+width`, `e.position.ry..ry+height` | `src/sim/production/production_placement.rs:429-440` | Yes; current placement validation |
| gamemd refinery helper anchor | `GetMapCell()` / vtable `+0x1B8` top-left foundation cell, then `+3,+1` | `BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md:252-255`; `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md:205-209` | Yes; report says case `0x0E` fires when harvesters dock |
| gamemd foundation perimeter anchor | `building.MapCell = top-left` in `GetDockCellForObject` scan | `FIND_NEAREST_VARIANTS_SPIRAL_COMPARISON_GHIDRA_REPORT.md:300-320` | Conditional; active for production/hospital exit helper, not stock harvester dock |
| GAREFN retail foundation | `Foundation=4x3` | `ini/artmd.ini:1763-1773` | Yes; YR art data |
| GAREFN stock dock type flags | `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `FreeUnit=CMIN` | `ini/rulesmd.ini:11721-11736` | Yes; standard Allied refinery |

## 3. Core Logic

### Rust placement anchor

Current Rust placement passes the requested cell unchanged through `place_ready_building(..., rx, ry, ...)` into `sim.spawn_object(type_id, owner, rx, ry, ...)`. `spawn_object` reads height at exactly `(rx, ry)`, then `spawn_object_at_height` calls `GameEntity::new` with the same `rx, ry`. `GameEntity::new` stores those values directly in `Position { rx, ry, ... }`.

Active in YR: Yes as Rust implementation status for all placed buildings. Evidence: `src/sim/production/production_placement.rs:180-248`, `src/sim/world/world_spawn.rs:280-337`, `src/sim/game_entity.rs:281-319`.

Placement preview confirms the semantic intent: it names `(rx, ry)` as the reference/top-left foundation cell, walks the foundation with `dx/dy` from that origin, and structure overlap tests use `e.position.rx/ry` as the rectangle minimum. Active in YR: Yes as Rust implementation status. Evidence: `src/sim/production/production_placement.rs:40-55`, `src/sim/production/production_placement.rs:429-440`.

### gamemd dock helper anchor

The existing binary reports say the refinery/weeder branch in `BuildingClass::Receive_Radio` case `0x0E` gets the building top-left/foundation cell via vtable `+0x1B8`, computes `queue = { tl.x + 3, tl.y + 1 }`, and gets the `CellClass` for that queue cell. A newer full-switch report restates the same behavior as standard refinery path `anchor+(X+3, Y+1)` and explicitly says `QueueingCell=` is stored but not read by the BuildingClass switch.

Active in YR: Yes. Evidence: `BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md:252-255`; `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md:191-209`, `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md:438-439`, `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md:506-507`.

### Reconciliation

For a placed `GAREFN` at Rust `(rx, ry)`, Rust `GameEntity.position.rx/ry` is the NW/top-left foundation cell. The gamemd refinery helper's `GetMapCell()` / vtable `+0x1B8` anchor is also documented as the top-left foundation cell. The anchors are the same concept for this slice.

Active in YR: Yes for stock `GAREFN` dock requests, because `rulesmd.ini` marks `GAREFN` as `Refinery=yes`, `DockUnload=yes`, and `NumberOfDocks=1`. Evidence: `ini/rulesmd.ini:11721-11736` plus dock report evidence above.

## 4. INI Keys

| Key | Retail value | Effect in this slice | Active in YR |
|---|---:|---|---|
| `artmd.ini [GAREFN] Foundation` | `4x3` | Defines footprint dimensions measured from the NW/top-left origin | Yes; `ini/artmd.ini:1763-1773` |
| `artmd.ini [GAREFN] QueueingCell` | `4,1` | Parsed/merged by Rust and used by some Rust queue/exit helpers, but existing binary docs say BuildingClass case `0x0E` does not read it | Conditional; stored in binary, not read by verified case `0x0E` |
| `rulesmd.ini [GAREFN] DockUnload` | `yes` | Enables refinery unload behavior in binary reports | Yes; `ini/rulesmd.ini:11721-11727` |
| `rulesmd.ini [GAREFN] Refinery` | `yes` | Classifies GAREFN as refinery | Yes; `ini/rulesmd.ini:11721-11727` |
| `rulesmd.ini [GAREFN] NumberOfDocks` | `1` | Single dock capacity | Yes; `ini/rulesmd.ini:11728-11730` |
| `rulesmd.ini [GAREFN] FreeUnit` | `CMIN` | Free chrono miner on placement | Yes; `ini/rulesmd.ini:11735-11736` |

## 5. Integration Points

- `placement_preview_for_owner` treats `(rx, ry)` as top-left reference height and walks foundation cells from that origin.
- `place_ready_building` passes `(rx, ry)` unchanged to `spawn_object`.
- `spawn_object_at_height` passes `(rx, ry)` unchanged to `GameEntity::new`.
- `refinery_geometry_for_entity` reads `entity.position.rx/ry` and derives refinery cells from that anchor; current source includes `refinery_can_dock_queue_cell(rx, ry) = (rx+3, ry+1)`.
- `BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x0E`, handles standard refinery docking and uses building top-left/NW anchor plus `(+3,+1)`.
- `GetDockCellForObject @ 0x0044EFB0` also uses `building.MapCell` as top-left, but that report identifies it as a production/hospital exit helper rather than the stock harvester dock path.

## 6. Current Rust Implementation Status

Implemented and aligned for the anchor:

- `Position.rx/ry` are the authoritative cell coordinates stored on every `GameEntity` (`src/sim/components.rs:22-49`).
- Placed building `rx/ry` are stored unchanged on the entity (`src/sim/production/production_placement.rs:180-248`; `src/sim/world/world_spawn.rs:280-337`; `src/sim/game_entity.rs:281-319`).
- Footprint overlap treats `position.rx/ry` as the rectangle minimum / NW foundation origin (`src/sim/production/production_placement.rs:429-440`).
- Current refinery geometry has a gamemd-shaped CAN_DOCK helper `rx+3, ry+1` (`src/sim/miner/miner_dock_sequence.rs:100-106`, `src/sim/miner/miner_dock_sequence.rs:340-352`).

Not proven by a direct test:

- The existing GAREFN placement test checks that a free harvester spawns and does not land inside the footprint, but does not directly assert that the placed `GAREFN` entity itself has `position.rx == 20` and `position.ry == 20`. A generic building placement test makes that assertion for `GACNST`. Evidence: `src/sim/production/production_placement_tests.rs:156-198`, `src/sim/production/production_placement_tests.rs:201-245`.

Open but out of this slice:

- Some Rust helpers still parse and use `QueueingCell=4,1` for queue/exit/far-return contexts (`src/rules/art_data.rs:100-102`, `src/rules/ruleset.rs:1707-1770`, `src/sim/miner/miner_dock_sequence.rs:82-98`, `src/sim/miner/miner_dock_sequence.rs:164-185`). This report only proves the anchor, not every call-site's correct target cell.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Rust `Position.rx/ry` meaning | verified | `src/sim/components.rs:22-49` | none |
| Rust placed-building storage path | verified | `src/sim/production/production_placement.rs:180-248`; `src/sim/world/world_spawn.rs:280-337`; `src/sim/game_entity.rs:281-319` | no GAREFN-specific assert |
| Rust footprint origin | verified | `src/sim/production/production_placement.rs:40-55`; `src/sim/production/production_placement.rs:429-440` | none |
| Rust refinery CAN_DOCK cell helper | verified | `src/sim/miner/miner_dock_sequence.rs:100-106`, `src/sim/miner/miner_dock_sequence.rs:340-352` | call-site behavior beyond anchor not audited |
| gamemd `Receive_Radio` case `0x0E` anchor | verified-via-existing-report | `BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md:252-255`; `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md:191-209` | no new Ghidra spot-check because docs did not conflict |
| gamemd `QueueingCell=` non-read in case `0x0E` | verified-via-existing-report | `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md:438-439`, `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md:506-507` | whether other binary paths use it is out-of-scope |
| GAREFN retail dimensions/type flags | verified | `ini/artmd.ini:1763-1773`; `ini/rulesmd.ini:11721-11736` | none |
| Historical trace anchor uncertainty | resolved for anchor only | `miner/traces/MINER_DOCK_QUEUE_TWO_MINERS_ONE_REFINERY_TRACE.md:80-88`, this report's Rust/doc reconciliation | queue-cell call-site parity remains separate |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does current Rust store placed building `rx/ry` unchanged on `GameEntity.position`? Yes. Evidence: `src/sim/production/production_placement.rs:180-248`; `src/sim/world/world_spawn.rs:280-337`; `src/sim/game_entity.rs:281-319`.

[RESOLVED] OQ-2 - Does current Rust treat building `position.rx/ry` as a foundation NW/top-left cell rather than center? Yes. Evidence: placement preview reference height at `(rx, ry)` and footprint walk from that origin, plus overlap rectangle minimum at `e.position.rx/ry`; `src/sim/production/production_placement.rs:40-55`, `src/sim/production/production_placement.rs:429-440`.

[RESOLVED] OQ-3 - Does gamemd's refinery dock helper use a NW/top-left/foundation anchor? Yes per existing binary reports: vtable `+0x1B8` / `GetMapCell()` top-left, then `+3,+1`. Evidence: `BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md:252-255`; `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md:205-209`.

[RESOLVED] OQ-4 - Is the gamemd path active in standard YR for `GAREFN`? Yes. Existing report says case `0x0E` fires every match when a harvester tries to dock, and stock `GAREFN` has `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`. Evidence: `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md:191-209`; `ini/rulesmd.ini:11721-11736`.

[RESOLVED] OQ-5 - Does current Rust still parse `QueueingCell=4,1` for `GAREFN`? Yes. Evidence: `ini/artmd.ini:1763-1773`; `src/rules/art_data.rs:100-102`, `src/rules/art_data.rs:324-329`; `src/rules/ruleset.rs:1707-1770`.

[DEFERRED] OQ-6 - Should every Rust `QueueingCell=4,1` use for refinery queue/exit/far-return be replaced or split from the verified CAN_DOCK `+3,+1` path? Category: out-of-scope. This target only reconciles the anchor, not every dock-state target-cell call site.

## Sources

- `src/sim/components.rs`
- `src/sim/game_entity.rs`
- `src/sim/world/world_spawn.rs`
- `src/sim/production/production_placement.rs`
- `src/sim/production/production_placement_tests.rs`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/rules/art_data.rs`
- `src/rules/ruleset.rs`
- `ini/artmd.ini`
- `ini/rulesmd.ini`
- `docs/research/BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`
- `docs/research/FIND_NEAREST_VARIANTS_SPIRAL_COMPARISON_GHIDRA_REPORT.md`
- `docs/research/miner/traces/MINER_DOCK_QUEUE_TWO_MINERS_ONE_REFINERY_TRACE.md`
