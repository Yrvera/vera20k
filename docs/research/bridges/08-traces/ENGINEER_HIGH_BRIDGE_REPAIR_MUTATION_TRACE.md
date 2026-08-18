# Engineer High Bridge Repair Mutation Trace

Date: 2026-05-20

Scenario: Engineer enters `CABHUT`; nearby high bridge main-deck cells are `Destroyed`; verify 5x5 scan, repaired overlay/state bytes, RNG variant use, `bridge_state_changed` / zone rebuild trigger, and engineer consumption.

Scope guard: This trace covers only the high bridge repair mutation path after the engineer repair-hut branch is eligible. Cursor/order gating and sound/EVA ordering are adjacent swarm slots.

## Verdict Summary

PASS: 5 | FAIL: 2 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

Top findings:

1. FAIL - Exact repaired overlay variant is not gamemd-parity: gamemd uses `FUN_00598030` (`Random_Next` + `Math_ftol` rejection loop, limit 3); Rust uses `SimRng::next_range_u32(4)` with xorshift64* modulo. Player-visible repaired high bridge tile variant and replay RNG stream can diverge. Rust: `src/sim/bridge_state/walker.rs:339-342`, `src/sim/rng.rs:29-50`. Gamemd evidence: `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md:1400-1414`, `REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md` section 2.
2. FAIL - Rust resets its bridge damage state on repair, while gamemd repair walkers write only `CellClass+0x44` overlay and leave `+0x11E` stale. Immediate visuals may still look repaired, but repaired-then-redamaged bridge durability/progression can differ. Rust: `src/sim/bridge_state/walker.rs:353-354`. Gamemd evidence: `REPAIRBRIDGEWALKER_FIELD_11E_FOLLOWUP.md`, `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md:946-950`.
3. NOT-IMPLEMENTED - Destroyed-to-healthy repair produces radar dirty cells, but Rust drops `outcome.radar_cells` without a render/minimap propagation channel. The minimap can miss or delay the repaired bridge update. Rust: `src/sim/world/world_orders.rs:354-358`. Gamemd evidence: `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md:1310-1315`, `1438-1452`.
4. UNCHECKED - Exact engineer arrival timing is not proven equal in this slot. Gamemd fires from `InfantryClass::PerCellProcess` when the engineer enters/steps into the hut cell; Rust fires for an adjacent engineer with `capture_target` set. This belongs to the action-gate slot, but mutation timing depends on it. Rust: `src/sim/world/world_orders.rs:307-328`. Gamemd evidence: `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md:356-383`.

## Evidence Used

Primary gamemd evidence came from verified research docs, not live Ghidra mutation. The cited docs state these paths are active in standard YR: `InfantryClass::PerCellProcess @ 0x519630` reaches the engineer `BridgeRepairHut` branch in missions `8`, `0xB`, `0x19`, dispatches high repair through `ProcessBridgeDestruction_High @ 0x573540`, then `RepairBridge_High @ 0x57F440`, then `RepairBridgeWalker_NS_High @ 0x5800D0` / `EW_High @ 0x580600`. `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md:56-62`, `107-121`, `1118-1120`, `1503-1518`.

Relevant Rust path:

- `Simulation::advance_tick` runs `tick_bridge_repair_orders` before capture and ORs the result into `TickResult.bridge_state_changed`: `src/sim/world/mod.rs:1243-1251`.
- `tick_bridge_repair_orders` filters engineers with `capture_target`, checks `BridgeRepairHut=yes`, checks adjacency, emits repair sound, runs a 5x5 scan, calls bridge repair, and despawns the engineer: `src/sim/world/world_orders.rs:261-365`.
- `cells_in_5x5_scan` enumerates inclusive `[-2..=2]` offsets with `dy` outer and `dx` inner: `src/sim/bridge_state/mod.rs:1484-1497`.
- `repair_bridge_from_engineer_scan` chooses low if any scanned low overlay/wood tile exists; otherwise it scans high overlays and calls the high walker: `src/sim/bridge_state/walker.rs:62-115`.
- High repair walkers traverse the high overlay band and write a 3-cell strip via `apply_repair_to_strip_cell`: `src/sim/bridge_state/walker.rs:138-156`, `237-313`, `315-374`.

## Stage Results

| Stage | Rust output for this scenario | gamemd output | Verdict |
|---|---|---|---|
| Active YR path | Bridge repair exists in sim and is triggered before generic capture. | Engineer repair-hut branch is active in standard YR via `PerCellProcess` missions `8`, `0xB`, `0x19`; not dormant TS legacy. | PASS |
| Arrival/tick trigger | Adjacent engineer with `capture_target=hut` fires on current sim tick. | `PerCellProcess` fires when infantry enters/steps into the hut cell and finds a `BridgeRepairHut`. | UNCHECKED |
| 5x5 scan | Interior center produces exactly 25 cells, inclusive `[-2..=2]`, `dy` outer, `dx` inner. | Same inclusive 25-cell scan around engineer coord. | PASS |
| Low/high dispatch | With no low bridge overlay/wood tile in scan, dispatches high family. | Outer scan sets low only for low ramp tile or `0x4A..=0x65`; otherwise calls high dispatcher. | PASS |
| High destroyed main-deck mutation set | For high NS destroyed `0xE7`, one walker iteration rewrites `(x,y)`, `(x,y-1)`, `(x,y+1)`; Rust test fixture repairs 3 cells. | High NS destroyed `0xE7` maps to `0xCD + RNG(0..3)` and writes the same overlay to the 3-cell perpendicular strip. | PASS |
| Exact overlay variant / RNG | Default Rust seed first repair offsets are xorshift modulo values `1,0,2,...` if no earlier RNG draw occurs; new overlays begin `0xCE,0xCD,0xCF,...`. | Uses gamemd `Random_Next` + float conversion/rejection loop, limit `3`; exact sequence not Rust xorshift modulo. | FAIL |
| State-byte semantics | Rust writes `DamageState::Healthy { variant }` to every touched cell and syncs span state. | Repair walker writes only overlay `+0x44`; follow-up refuted any `+0x11E` reset through `RecalcAttributes` for bridge overlays. | FAIL |
| Zone rebuild trigger | RandomHealthy repair sets `outcome.zones_dirty=true`; `tick_bridge_repair_orders` returns true; `advance_tick` sets `bridge_state_changed`; app refresh path observes that flag. | `bVar1=true` for high `0xD1..0xD5`, `0xDA..0xDE`, `0xE7`, `0xE8`; walker calls `UpdateBridgeZonesHelper` after traversal. | PASS |
| Engineer consumption | Engineer is despawned after repair. | Engineer disposal calls `vtable[0xF8]` after callbacks. | PASS |
| Radar dirty propagation | `radar_cells` are collected for destroyed anchors but ignored at world-order boundary. | Destroyed-anchor repair marks terrain dirty for the 3 written cells. | NOT-IMPLEMENTED |

## Concrete Rust Scenario Check

Existing test `engineer_enters_cabhut_repairs_bridge` sets:

- CABHUT at `(9,10)`.
- Engineer at `(10,10)` with `capture_target=hut`.
- High NS bridge cells `(10,9)..(10,13)` seeded `DamageState::Destroyed`, overlay `0xE7`.

Observed by test:

- `TickResult.bridge_state_changed == true`.
- Engineer is removed from the entity store.
- Repaired strip cells `(10,9)`, `(10,10)`, `(10,11)` are `Healthy { variant <= 3 }` and bridge-walkable.

Verification run:

- `cargo test -q engineer_enters_cabhut_repairs_bridge` passed on 2026-05-20.

This test covers the Rust mutation path but does not prove gamemd numerical equality for RNG output or stale `+0x11E` behavior.

## Adjacent Findings

- Sound/EVA is intentionally not scored here. Rust emits `SimSoundEvent::BridgeRepaired` before repair mutation; gamemd has EVA and sound before the 5x5 dispatch. That belongs to the swarm sound/EVA slot.
- The Rust trigger currently consumes an adjacent engineer rather than proving the same cell-entry timing as `PerCellProcess`. That belongs to the action-gate slot.
- Rust comments still describe minimap refresh as happening after PathGrid rebuild, but the repair-specific `radar_cells` are not connected to a per-cell radar dirty API.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/REPAIRBRIDGEWALKER_FIELD_11E_FOLLOWUP.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_orders.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/walker.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/rng.rs`
