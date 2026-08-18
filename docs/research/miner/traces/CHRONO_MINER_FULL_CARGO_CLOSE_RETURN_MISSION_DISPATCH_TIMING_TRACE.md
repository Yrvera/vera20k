# Chrono Miner Full Cargo Close Return Mission Dispatch Timing Trace

Date: 2026-05-22

> **Follow-up 2026-05-22:** The deferred queue/commence frame boundary is now
> resolved in `CHRONO_MINER_CLOSE_RETURN_SCHEDULER_FRAME_TRACE.md`. A mission
> queued by `Queue_Mission(..., 0)` is promoted by the late `UnitClass::AI`
> commence gate in the same unit AI frame, but the newly current mission handler
> is not dispatched until the next `MissionClass::Mission_Dispatch` for that
> unit. This resolves DQ-1 and DQ-2 at the scheduler boundary; runtime logging is
> still needed only for exact timer-table/jitter values and first rendered
> locomotor displacement.

Scenario: stock YR `CMIN` with full cargo, within the close-return branch for a stock `GAREFN` or `NAREFN`, so `UnitClass::Mission_Harvest` state 2 uses the refinery radio/contact path instead of the far `QueueingCell` fallback.

Scope: trace the ordering from state 2 `HELLO(0x02)` through state 3, mission `7` dispatch, `Mission_Enter`, `0x0E`, `0x12`, `0x18/0x16`, accepted-cell/pad arrival, `0x15`, and mission `0x10` unload entry. Current Rust comparison is limited to `src/sim/miner/*`. No Rust implementation files were edited.

## Sources Checked

- Binary read-only Ghidra: `UnitClass::Mission_Harvest @ 0x0073E5E0`, `MissionClass::Mission_Dispatch @ 0x005B3060`, `MissionClass::Queue_Mission @ 0x005B35E0`, `MissionClass::Commence @ 0x005B3570`, `MissionClass::GetCurrentMission @ 0x005B3040`, `FootClass::Mission_Enter @ 0x004D9290`, `FootClass::Receive_Radio @ 0x004D8FB0`, `BuildingClass::Receive_Radio @ 0x0043C2D0`, `UnitClass::PerCellProcess @ 0x00739EC0`, `UnitClass::Mission_Deploy_Building @ 0x0073D630`, `TechnoClass::AI_Update @ 0x006F9E50`.
- Existing research: `miner/MISSION_HARVEST_STATE2_CLOSE_RETURN_RADIO_TIMING_GHIDRA_REPORT.md`, `miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`, `FOOTCLASS_RECEIVE_RADIO_0X12_MOVE_FIELDS_NAVCOM_GHIDRA_REPORT.md`, `UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_00739EC0_NAVCOM_GHIDRA_REPORT.md`, `miner/STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`, `TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md`.
- INI data: `rulesmd.ini` `[General]`, `[CMIN]`, `[GAREFN]`, `[NAREFN]`; `artmd.ini` `[GAREFN]`, `[NAREFN]`.
- Rust surfaces only: `src/sim/miner/mod.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`.

## Concrete Stock Values

| Value | Stock source | Concrete value |
|---|---|---:|
| CMIN close-return threshold | `rulesmd.ini:294` | `ChronoHarvTooFarDistance=50`, compared as `50 * 0x100 = 12800` leptons, inclusive |
| CMIN dock targets | `rulesmd.ini:7361` | `Dock=NAREFN,GAREFN` |
| CMIN full cargo | `rulesmd.ini:7374` | `Storage=20` bales |
| CMIN unloading voxel | `rulesmd.ini:7384` | `UnloadingClass=CMON` |
| CMIN chrono type | `rulesmd.ini:7396..7398` | `Teleporter=yes`, teleport locomotor |
| Stock refinery admission | `rulesmd.ini:11726..11729`, `12519..12521` | `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1` |
| Accepted `0x12` cell for refinery at NW `(rx,ry)` | binary `0x0043C2D0`; art confirmation | `(rx+3, ry+1)` |
| Waiting/fallback queue cell | `artmd.ini:1716`, `1773` | `(rx+4, ry+1)`, not the accepted `0x12` cell |
| Open physical pad cell | `artmd.ini:1760`, `1795` | `(rx+3, ry+1)` is made passable by stock refinery art |
| Dump interval | binary `0x0073E35B..0x0073E374`; rules default | `HarvesterDumpRate * 900 = 0.016 * 900 = 14.4` frames per storage-slot gate |

## Pipeline Summary

`Mission_Harvest state 2` -> `radio 0x02 HELLO` -> `state 3` -> `Queue_Mission(7, 0)` -> mission scheduler/commence -> `Mission_Enter` -> `radio 0x0E CAN_DOCK` -> refinery sends `0x12 MOVE_TO_CELL(rx+3,ry+1)` -> if not already there, NavCom is written and movement runs -> if already there, refinery sends `0x18` then `0x16` -> physical accepted-cell arrival sends `0x15` -> refinery queues sender mission `0x10` -> mission scheduler/commence -> `Mission_Deploy_Building` state 0/3 unload init -> dump gates every 14.4 frames -> empty-slot gate sets state 4 -> state 4 hands back to harvest scheduling.

## Stage Table

| Stage | gamemd evidence and timing | Current Rust surface | Verdict |
|---|---|---|---|
| 1. Full cargo enters return branch | `Mission_Harvest` state 1 detects full storage, writes state 2. CMIN capacity is 20 bales. | `handle_harvest` extends cargo and calls `begin_return` when `is_full()`; capacity comes from `Storage=20` or config default 20. | PASS for full-cargo trigger surface. |
| 2. Close-return threshold | State 2 compares 3D object distance to `ChronoHarvTooFarDistance * 0x100`; stock threshold is `12800` leptons and the compare is inclusive. | `try_issue_chrono_far_return_teleport` uses `cell_dist_sq > threshold^2` against refinery anchor; it is cell-squared, not binary 3D lepton distance. | FAIL for exact boundary math; likely low player visibility except at the 50-cell boundary. |
| 3. State 2 `HELLO(0x02)` | At `0x0073EE54..0x0073EE68`, close branch pushes refinery, pushes `0x02`, calls directed radio, and writes substate `3` only on reply `1`. No `0x0E` yet. This happens inside the state-2 dispatch tick. | `handle_return` moves a chrono miner to the accepted dock cell first; only after `contact` does `Dock/Approach` call `hello_or_wait`. | FAIL for exact radio/tick ordering. Player-visible under refinery contention; single unblocked miner usually still reaches the same cell. |
| 4. State 3 queues mission 7 | At `0x0073EE8D..0x0073EE93`, state 3 pushes `0`, pushes `7`, calls vtable `+0x1E8`, and returns `1`. Static evidence verifies the queue/write and 1-frame dispatch timer return. | Rust uses `DockPhase::MissionEnter` on the next miner tick after `Approach`; it does not model the `MissionClass` queue directly. | UNCHECKED for exact frame parity; Rust intentionally uses one sim tick between phases. |
| 5. Mission scheduler boundary after state 3 | `Mission_Dispatch @ 0x005B3060` is called once per `TechnoClass::AI_Update` tick, reads `+0xC8/+0xD0`, and writes the handler return as the next delay. `Queue_Mission @ 0x005B35E0` with force byte `0` does not call `Commence` through its `param_3 != 0` block. `Commence @ 0x005B3570` copies `+0xB4` to current mission and zeroes dispatch duration. Exact runtime placement of the commence call for this queued mission was not proven in this trace. | No MissionClass equivalent; phase progression is controlled by `tick_miners` once per sim tick. | UNCHECKED/DEFERRED. Needs runtime debugger/watchpoint or a complete caller proof for `Commence` around the same object tick. |
| 6. `Mission_Enter` sends `0x0E` | `FootClass::Mission_Enter @ 0x004D9290`, `0x004D92B4..0x004D92BF`, sends directed `0x0E` to the target and accepts reply `1` or `+0x418`. | `phase_mission_enter` calls `hello_or_wait` again, checks pad/contact state, then issues direct movement to accepted cell. | PASS for single-miner accepted target progression; FAIL for raw radio shape. |
| 7. Refinery accepted cell | `BuildingClass::Receive_Radio(0x0E)` computes accepted payload as building NW `+(3,1)`. `QueueingCell=4,1` is not read in this accepted branch. | `refinery_can_dock_queue_cell(rx,ry)` returns `(rx+3, ry+1)`; `refinery_queue_cell` remains separate for waiting/staging. | PASS. |
| 8. `0x12` NavCom write | If unit is not already on `(rx+3,ry+1)`, `FootClass::Receive_Radio(0x12)` calls `Set_Destination_Internal`, writes `+0xC8 = g_CurrentFrameCounter`, writes `+0xCC = target Y local`, then writes `+0xD0 = 0`. | Rust issues `movement::issue_direct_move` to the accepted cell; it does not have raw NavCom/timer triplet fields. | PASS for movement target surface; UNCHECKED for retry/timer field parity. |
| 9. `0x18`/`0x16` gate | `0x18` then `0x16` are sent only after `0x12` returns `0x14` already-there. `0x18` sets `+0x418`; `0x16` starts the face/timing sync. | When at accepted cell and not moving, `phase_mission_enter` marks `contact_entered` and advances to `Linked`; `phase_linked` starts pivot on a later tick. | FAIL/UNCHECKED for exact same-dispatch ordering; PASS for eventual entered/pivot state in an uncontested case. |
| 10. Physical arrival and `0x15` | `UnitClass::PerCellProcess @ 0x00739EC0` requires mission `7`/`0x19`, destination building, current cell equals dock coord, locomotor gate, then calls `FootClass::PerCellProcess(2)`, sends `0x15`, then locomotor slot `+0x5C`. | Rust has no distinct `0x15` event; `phase_linked` directly marks `on_pad`, sets unloading display, emits `DockDeploy`, and starts pivot. | FAIL for event identity and likely tick placement; PASS for visible "miner is docked and starts pivot/unload" at broad level. |
| 11. `0x15` queues mission `0x10` | Building receiver case `0x15` queues sender mission `0x10` for `DockUnload=yes`; this is a mission id, not radio `0x10`. | Rust enters `Unloading` phase after pivot convergence instead of queuing mission `0x10`. | PASS for broad unload entry; UNCHECKED for exact mission-dispatch frame. |
| 12. Unload first gate | `Mission_Deploy_Building` first waits for facing/rate window; if not ready and `+0x6AF` false, it asks locomotor to face `0x4000` and returns `5`. | `phase_pivoting` uses a `FacingClass` timer and only enters `Unloading` once the accepted east-facing window is reached. | PASS at behavioral surface; UNCHECKED for exact number of frames from arbitrary incoming facing. |
| 13. Slot drain cadence | State 3 threshold is `14.4` frames. Each threshold drains one whole storage slot; stock full ore CMIN is one ore slot worth 20 bales, so visible credits arrive in one ore pulse, not 20 per-bale pulses. | `phase_unloading` drains all bales of one resource type per gate and stores interval as `144` tenths, decrementing by `10` per tick. | PASS for cadence model and slot granularity. |
| 14. Empty-slot gate and mission `0x10` exit | After the last real slot drain, the next 14.4-frame gate finds no non-empty slot, sets state 4, clears slot 10 if needed, and direct-returns `1`. State 4 then clears unload-active state and schedules harvest/guard handoff; stock zero-link path does not call `ReleaseDockedHarvester` or `Force_Track(0x47)`. | `phase_unloading` goes to `Departing` only on the empty-slot gate; `phase_departing` clears dock/contact bookkeeping and returns to `SearchOre` without `Force_Track(0x47)`. | PASS for stock zero-link behavior; UNCHECKED for the exact one-tick state-4 scheduler boundary. |

## Frame/Timing Values Computed

- Close-return threshold for CMIN: `50 * 256 = 12800` leptons, inclusive in gamemd.
- Accepted refinery cell for a refinery at NW `(10,10)`: `(13,11)`.
- Waiting/fallback queue cell for the same refinery: `(14,11)`.
- Dump gate: `0.016 * 900 = 14.4` frames.
- Full stock CMIN pure ore cargo: 20 bales in one ore storage slot. First real deposit occurs at the first state-3 dump gate after unload entry; empty-slot transition occurs at the next dump gate, another 14.4 frames later.
- Mission handler returns verified statically: state 2/3 close-return handlers return `1`; `Mission_Deploy_Building` facing-not-ready path returns `5`; state-3 deposit/empty paths direct-return `1`.

Exact frame values still not computed:

- The exact runtime frame on which mission `7` first dispatches after state 3 calls vtable `+0x1E8`.
- The exact runtime frame on which mission `0x10` first dispatches after `0x15` queues it.
- The exact frame from accepted-cell movement completion to `PerCellProcess(2)`/`0x15`, because movement, per-cell processing, and `Mission_Dispatch` ordering need a live or fully reconstructed tick-order trace.
- The exact pivot duration from arbitrary incoming facing to the `0x4000` accepted east-facing window; binary rate-timer and current facing are verified, but the scenario did not specify incoming facing.

## Player-Visible Risks

1. Busy refinery timing can differ because Rust currently delays the `HELLO(0x02)` contact until the chrono miner reaches the accepted dock cell. In gamemd, the close-return branch contacts the refinery before the accepted-cell move.
2. A miner waiting near a busy refinery may move or queue at a different frame because Rust collapses raw radio/NavCom/mission queue fields into dock phases.
3. The accepted cell is correct, so single-miner happy-path movement usually looks close, but frame-perfect docking, pivot start, and first unload-entry timing remain unproven.
4. Boundary returns near exactly 50 cells can diverge because Rust uses cell-squared distance to the refinery anchor, while gamemd uses inclusive 3D object-coordinate distance scaled in leptons.

## Deferred Timing Questions

- DQ-1: After `Mission_Harvest` state 3 calls vtable `+0x1E8` with `(7,0)` at `0x0073EE8D..0x0073EE93`, does `MissionClass::Commence` run later in the same object update, on the next `TechnoClass::AI_Update`, or through another caller before the next visible frame? Static evidence verifies the queue write and dispatch timers, but not this runtime placement.
- DQ-2: After `BuildingClass::Receive_Radio(0x15)` queues sender mission `0x10`, which exact frame first enters `UnitClass::Mission_Deploy_Building` state 0/state 3? Same scheduler uncertainty as DQ-1.
- DQ-3: On the already-at-accepted-cell variant, does `0x18/0x16` and the successful `0x15` pad-arrival handoff occur in the same logic frame or across adjacent frames? Static code proves the synchronous `0x18/0x16` gate and the `PerCellProcess` order, but not the full global tick interleave.
- DQ-4: For a specified incoming facing, how many frames until the `RateTimer` expression `(((timer >> 7) + 1) & 0x1FE) == 0x80` accepts the east-facing unload window?

## Trace Verdict

Overall status: FAIL/UNCHECKED for exact frame parity, PASS for several broad single-miner surfaces.

The binary ordering is clear through each local function: close branch sends `0x02`, state 3 schedules mission `7`, `Mission_Enter` sends `0x0E`, the building sends `0x12`, `0x18/0x16` are already-there gated, physical arrival sends `0x15`, and the building queues mission `0x10`. The exact scheduler frame for mission `7` and mission `0x10` still needs runtime debugger evidence or a complete proof of every `Commence` caller in the object tick.

The current Rust miner surface is not frame-equivalent because close-return contact is modeled after reaching the accepted cell rather than during `Mission_Harvest` state 2. Do not patch from this trace alone; first answer the deferred scheduler questions if frame-perfect parity is required.
