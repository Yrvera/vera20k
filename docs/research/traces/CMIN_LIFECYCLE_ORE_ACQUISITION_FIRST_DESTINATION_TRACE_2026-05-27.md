# CMIN Lifecycle Ore Acquisition First Destination Trace

**Date:** 2026-05-27  
**Trace-swarm slot:** 1  
**Mechanic:** Chrono Miner mining lifecycle - ore acquisition first destination  
**Scenario scope:** Fresh Allied Chrono Miner (`CMIN`) at a stock Allied refinery (`GAREFN`) starts `Mission_Harvest` state 0 / Rust `SearchOre`. Trace only the state-0 ore scan, selected first ore cell, destination assignment, and first visible movement mode.

## Concrete Scenario

- Standard YR/skirmish path: shroud harvestability gate skipped by `g_GameMode != 0`.
- `GAREFN` NW foundation anchor: `(40,40)`.
- Stock accepted refinery dock cell: NW `+(3,1)` = `(43,41)`.
- Fresh `CMIN` current cell: `(43,41)`, empty cargo, no archive/ghost cell, no active destination, no active teleport state.
- Reachability/passability: all listed resource cells are in-playfield, same Crusher zone, and enterable for the miner; no blockers.
- Stock scan radius: `TiberiumLongScan=48`.
- Fixed nearby Riparius layout, represented as real map overlay density bytes and Rust overlay-seeded resource stock:

| Cell | Ring from `(43,41)` | Type | gamemd `OverlayData` | gamemd value | Rust `ResourceNode.remaining` | Rust scan score |
|---|---:|---|---:|---:|---:|---:|
| `(42,39)` | 2 | Riparius/Ore | 3 | `25 * (3+1) = 100` | `120 * (3+1) = 480` | `25 * (480+1) = 12025` |
| `(43,43)` | 2 | Riparius/Ore | 7 | `25 * (7+1) = 200` | `120 * (7+1) = 960` | `25 * (960+1) = 24025` |
| `(45,42)` | 2 | Riparius/Ore | 5 | `25 * (5+1) = 150` | `120 * (5+1) = 720` | `25 * (720+1) = 18025` |

Expected first destination for this exact layout: `(43,43)`.

## Pipeline

gamemd: `Mission_Harvest state 0 -> full-cargo gate -> archive gate -> clear harvesting flag -> TiberiumLongScan -> FootClass::Scan_For_Tiberium -> Search_For_Tiberium_And_Move -> Set_Destination -> ground drive toward selected ore`

Rust: `tick_miners -> handle_search_ore -> build_scan_filter -> search_local_ore -> target_ore_cell=Some(cell), state=MoveToOre -> handle_move_to_ore -> MovementTarget ground move`

## Stage Results

### Stage 1 - Stock Active-YR Data

**gamemd/YR evidence:** `[CMIN]` has `Harvester=yes`, `Storage=20`, `Teleporter=yes`, teleport locomotor, and `MovementZone=Crusher` in `ini/rulesmd.ini:7351-7400`. `[GAREFN]` has `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, and `FreeUnit=CMIN` in `ini/rulesmd.ini:11722-11737`. `[GAREFN]` art has `Foundation=4x3` in `ini/artmd.ini:1763-1766`. Riparius `Value=25` is stock active ore/tiberium data in `ini/rulesmd.ini:30372-30396`.

**Rust evidence:** `miner_kind_for_object` maps `Harvester=yes + Teleporter=yes` to `MinerKind::Chrono` in `src/sim/miner/mod.rs:426-443`; `MinerConfig` stock scan defaults include `long_scan_radius=48` in `src/sim/miner/mod.rs:189-213`.

**Verdict:** PASS. Stock scenario inputs are active standard YR data and Rust has corresponding CMIN/miner configuration.

### Stage 2 - Starting Cell And Refinery Anchor

**gamemd evidence:** The accepted stock refinery `CAN_DOCK` cell is computed from the building NW cell as `slot_x+3, slot_y+1`; the report gives `(10,10)->(13,11)` and confirms the path is active for standard YR `GAREFN/NAREFN` and `CMIN/HARV` (`docs/research/BUILDINGCLASS_GETCELLLOCATION_VTABLE_0X1B8_ANCHOR_GHIDRA_REPORT.md:83-106`).

**Concrete value:** For NW `(40,40)`, fresh miner-at-refinery anchor is `(43,41)`.

**Rust evidence:** `refinery_dock_cell` delegates to the stock queue/dock helper in `src/sim/miner/miner_system.rs:1430-1448` and the docking sequence uses the same `NW+(3,1)` stock cell in prior verified dock reports.

**Verdict:** PASS for the concrete starting coordinate used by this trace.

### Stage 3 - State-0 Scan Radius And Ring Search

**gamemd evidence:** `UnitClass::Mission_Harvest` state 0 is active for CMIN/HARV; it reads `TiberiumLongScan`, not short scan, and calls `FootClass::Scan_For_Tiberium` (`docs/research/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md:1-8`, `95-113`, `187-188`). The scan checks ring 0 first, then rings `1..47`, exits after the first ring with harvestable ore, and chooses the highest `CellClass::Get_Tiberium_Value` inside that ring (`same report:130-167`).

**Rust evidence:** `handle_search_ore` calls `search_local_ore(... config.long_scan_radius ...)` and writes `target_ore_cell` plus `MoveToOre` on success (`src/sim/miner/miner_system.rs:365-385`). `search_local_ore` scans ring `1..radius`, exits after `best_in_ring`, and returns the selected cell (`src/sim/miner/miner_system.rs:1376-1427`).

**Concrete output:** Ring 1 has no resource. Ring 2 has three harvestable Riparius cells. Both algorithms stop at ring 2.

**Verdict:** PASS for radius, ring order, ring stop, and selected ring.

### Stage 4 - Per-Cell Value Mechanism

**gamemd evidence:** `CellClass::Get_Tiberium_Value` returns `TiberiumClass.Value * (CellClass+0x11E + 1)` (`docs/research/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md:238-250`). With Riparius `Value=25`, the three concrete values are `100`, `200`, and `150`.

**Rust evidence:** Live overlay seeding converts overlay frame to `ResourceNode.remaining = base_stock * (frame+1)` (`src/sim/production/production_queue.rs:155-170`). `search_local_ore` then scores with `bale_value * (node.remaining + 1)` (`src/sim/miner/miner_system.rs:1359-1365`), producing `12025`, `24025`, and `18025`.

**Concrete output:** The chosen cell remains `(43,43)` in this all-Riparius layout, but the consumed score values are not numerically equal: gamemd best score `200`, Rust best score `24025`.

**Verdict:** FAIL. Mechanism and consumed score bytes differ even though this particular all-Riparius ordering still selects the same cell.

### Stage 5 - Selected First Ore Cell

**gamemd concrete computation:** Ring 2 candidates: `(42,39)=100`, `(43,43)=200`, `(45,42)=150`; highest value in the nearest non-empty ring is `(43,43)`.

**Rust concrete computation:** Ring 2 candidates: `(42,39)=12025`, `(43,43)=24025`, `(45,42)=18025`; highest score in the nearest non-empty ring is `(43,43)`.

**Concrete output:** selected first ore cell `(43,43)` on both sides.

**Verdict:** PASS for this exact layout's destination cell.

### Stage 6 - Destination Assignment And Harvest State Transition

**gamemd evidence:** When search succeeds, state 0 sets `UnitClass+0x6D2=1`, initializes the step timer fields to `2`, sets harvest substate to `1`, and returns `1` (`docs/research/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md:253-260`). The successful search path assigns the selected destination through `Search_For_Tiberium_And_Move`.

**Rust evidence:** `handle_search_ore` writes `snap.miner.target_ore_cell = Some(cell)` and `snap.miner.state = MinerState::MoveToOre`, then returns (`src/sim/miner/miner_system.rs:375-385`).

**Concrete output:** gamemd destination `(43,43)` and Rust `target_ore_cell=Some((43,43))`.

**Verdict:** PASS for first destination assignment. Exact gamemd timer-cluster bytes are not represented by Rust's split `SearchOre -> MoveToOre` state shape, but the scoped first destination cell is numerically equal.

### Stage 7 - First Visible Movement Mode

**gamemd evidence:** The verified state-0 report confirms no chrono-specific scan function; CMIN differs only by a CLSID check that cancels an in-progress warp before scanning (`docs/research/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md:95-113`). Prior ore-acquisition trace evidence found outbound acquisition drives rather than warps.

**Rust evidence:** `handle_move_to_ore` only waits if `teleport_state` already exists, then issues ground movement through `issue_direct_move` for adjacent targets or `issue_move_if_idle`/`issue_move_command` for non-adjacent targets (`src/sim/miner/miner_system.rs:428-439`, `491-510`; `src/sim/movement/movement_commands.rs:60-90`, `193-240`). The chrono teleport call path appears in return logic, not ore acquisition (`src/sim/miner/miner_system.rs:1002-1020`).

**Concrete output:** For `(43,41)->(43,43)`, Rust issues a ground `MovementTarget`; no `TeleportState`, no chrono world effects, no `ChronoTeleport` sound. gamemd expected movement mode is ground drive, not chrono warp.

**Verdict:** PASS for first visible movement mode: `drive_move=1`, `teleport_move=0`, immediate chrono effect/sound count `0`.

### Stage 8 - Exact First Path Cell, Facing, And Tick Cadence

**gamemd evidence:** Outbound drive path, facing, lepton-per-tick advance, and arrival timing are DriveLocomotion/pathfinding details after the first destination is assigned.

**Rust evidence:** Rust uses path-backed `MovementTarget` for non-adjacent ore and fixed-point speed derived from `Speed=4` (`src/sim/miner/miner_system.rs:108-124`, `498-510`).

**Concrete output:** Not computed for both engines in this slot.

**Verdict:** UNCHECKED. This trace proves the first destination and movement mode, not exact DriveLocomotion path/facing/tick parity.

## Failures

1. **Stage 4 - scan score mechanism drift.** Rust scores cells from `ResourceNode.remaining`, which is overlay-seeded stock, while gamemd scores from `OverlayData + 1`. In this all-Riparius layout the selected destination remains `(43,43)`, but numerical score equality fails (`200` vs `24025`). This can become player-visible as a different first destination when same-ring cells contain different tiberium types/densities whose gamemd values tie or cross differently.

## Not Implemented

None in this scoped first-destination trace.

## Unchecked

- Exact first path cell, facing, lepton movement per tick, and arrival tick after the first destination assignment.
- Tie handling should be spot-checked before a mixed same-ring ore/gem trace: the verified report text says "last-updated winner" while its quoted condition `old < new` implies equal values should not update.

## Adjacent Findings

- The score mismatch belongs to first-destination selection because it is consumed inside the scan, but a separate mixed ore/gem trace should demonstrate the player-visible wrong-cell case if prioritizing fixes.
- Harvest extraction, ore depletion retarget, full-cargo return, refinery docking/unload, and deposit/release are adjacent lifecycle stages and were not traced here.

## Verdict Tally

PASS: 6 | FAIL: 1 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Sources

- `docs/research/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_GETCELLLOCATION_VTABLE_0X1B8_ANCHOR_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `src/sim/miner/miner_system.rs`
- `src/sim/miner/mod.rs`
- `src/sim/production/production_queue.rs`
- `src/sim/movement/movement_commands.rs`
