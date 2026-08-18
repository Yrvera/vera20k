# Dock 0x16 RateTimer / Mission_Deploy Proof Gates - Reinvestigation Report

**Address(es):** `0x00737430` (`UnitClass::Receive_Radio` case `0x16`), `0x004B0EF0` (`DriveLocomotionClass::Do_Turn`), `0x004C9220` (`RateTimer::Set`), `0x004C93D0` (`RateTimer::Current`), `0x004D9290` (`FootClass::Mission_Enter`), `0x0073D630` (`UnitClass::Mission_Deploy_Building`), `0x0065AE30` (`PathType::Has_Valid_Steps`)
**Investigation Mode:** coverage-map follow-up
**Claimed Scope:** proof-gate status for implementing a full coupled stock refinery dock-facing / unload-start path after the Chrono Miner locomotor ownership bridge.
**Non-Scope:** fresh live Ghidra decompilation, runtime debugger traces, complete MissionClass scheduler storage/decrement, full building anim/audio composition, and Rust patches.
**Confidence:** Medium overall. High for facts inherited from existing high-confidence Ghidra reports; Medium for this consolidation because the local Ghidra MCP had no running instance in this session.
**Active in YR:** Yes for stock `HARV/CMIN -> GAREFN/NAREFN` refinery docking/unload.

## 1. Overview

This pass checked whether the design's hard proof gates are closed enough to implement the full byte-field dock/unload slice immediately.

Result: the `0x16 -> DriveLocomotion +0x4C -> RateTimer::Set(owner+0x388, 0x4000)` mechanism is already strongly verified. `Mission_Deploy_Building` path validity, facing-window polarity, not-ready return `5`, and normal stock zero-link state-3/state-4 branching are also strongly verified.

The blocker is narrower than before but still real: a byte-field implementation of the `+0xF8..+0x110` periodic accumulator cannot honestly claim full parity until the `+0x110` increment-step default/writer contract and the `+0x104` stack/Z value source are rechecked in one live binary slice. Existing docs identify their roles, but one load-bearing sentence still says `field_0x110 = 1` is not yet verified.

## 2. Class Layout / Key Offsets

| Offset / field | Owner | Verified role in this slice | Evidence | Status |
|---|---|---|---|---|
| `+0x388` | Unit/Foot | Facing/RateTimer sampled by radio `0x16` and Mission_Deploy facing gate | `DRIVELOCOMOTION_VTABLE_0X4C_TIMING_SYNC_METHOD_GHIDRA_REPORT.md`; `0x007376CE`, `0x0073DF56` | verified |
| `+0x674` | Unit/Foot | Active locomotor pointer used for `+0x4C(0x4000)` and `Is_Moving` | `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md` | verified |
| `+0x6AF` | Unit/Foot | Chrono/teleporting gate; when clear, ordinary dock sync calls locomotor turn | `0x007376BF`, `0x0073DF7A` in cited reports | verified |
| `+0xF8` | Unit | Bale/dump elapsed accumulator, reset at unload-start | `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md`; `0x0073DFD0` | verified |
| `+0xFC` | Unit | Timer fired flag set by `TechnoClass::AI_Update` | `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md` | touched |
| `+0x100` | Unit | Start frame for periodic accumulator | same; `0x0073DFE0..0x0073DFF3` | verified |
| `+0x104` | Unit | Secondary/Z storage copied from a stack value and updated by AI timer code | same | touched-not-exhausted |
| `+0x108` | Unit | Duration, set to `1` for dump/unload periodic accumulator | same; `0x0073DFFC` | verified |
| `+0x10C` | Unit | Timer-active/repeat flag, set to `1` at unload-start | same; `0x0073DFED` | verified |
| `+0x110` | Unit | Increment step added to `+0xF8` when periodic timer fires | `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md` | touched-not-exhausted |
| `+0x6D1` | Unit | Unload-active latch used by draw/unload path | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` | verified |
| `+0xBC` | Unit/Mission | Mission substate; unload-start writes `3`, empty transition writes `4` | `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md` | verified |
| `+0x2E4` | Unit/Building | Reciprocal dock-link branch selector; stock refinery normal path is zero-link | same | verified |

## 3. Core Logic

### 3.1 First ordinary radio `0x16`

For stock refinery docking, `BuildingClass::Receive_Radio(0x0E)` can send `0x18` and then `0x16` after `0x12` replies already-there (`0x14`). `UnitClass::Receive_Radio(0x16)` first runs the base chain, then when `+0x6AF == 0` it samples `RateTimer::Current(Unit+0x388)`.

If the current low word is not `0x4000`, it calls the active locomotor vtable slot `+0x4C` with target low word `0x4000` and returns `1` immediately. It does not send `0x15` in that first unsynchronized path.

Evidence: `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md`; `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_SCHEDULING_GHIDRA_REPORT.md`; `DRIVELOCOMOTION_VTABLE_0X4C_TIMING_SYNC_METHOD_GHIDRA_REPORT.md`.

### 3.2 DriveLocomotion vtable `+0x4C`

For active DriveLocomotion, vtable slot `+0x4C` resolves to `0x004B0EF0`. It reads `DriveLocomotion+0x08` for the owner, computes `owner+0x388`, and calls `RateTimer::Set` with the caller's target dword. It does not touch path speed, track state, refinery fields, or movement destination.

This proves that dock `0x16` is not a body-facing write in place. It is a RateTimer retarget through the active Drive locomotor.

Evidence: `DRIVELOCOMOTION_VTABLE_0X4C_TIMING_SYNC_METHOD_GHIDRA_REPORT.md`.

### 3.3 RateTimer storage and interpolation

`RateTimer::Set` / `RateTimer::Current` are sufficiently proven for this dock-facing design:

- target/current packed value at `timer+0x00`;
- source/interpolation baseline at `timer+0x04`;
- start frame at `timer+0x08`;
- duration at `timer+0x10`;
- rate at `timer+0x14`;
- retarget snapshots current interpolated value before writing the new target;
- duration is `abs(new_low - baseline_low) / rate`, integer division;
- `Current` returns target when `rate < 1`, remaining is zero, or step count is below one;
- low 16 bits interpolate; high 16 bits copy from target;
- expiration is `elapsed == duration`.

Evidence: `RATETIMER_CURRENT_FRAME_COUNTER_HELPERS_GHIDRA_REPORT.md`, `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`, and `DRIVELOCOMOTION_VTABLE_0X4C_TIMING_SYNC_METHOD_GHIDRA_REPORT.md`.

### 3.4 Mission_Deploy path/facing gates

`Mission_Deploy_Building @ 0x0073D630` in the stock zero-link harvester branch first checks `PathType::Has_Valid_Steps @ 0x0065AE30`.

The branch polarity is settled:

- `Has_Valid_Steps() != 0` jumps to the RateTimer/facing gate.
- `Has_Valid_Steps() == 0` takes cleanup, clears `+0x6D1`, optionally queues/stops, and direct-returns `1`.

The direct return `5` belongs to the facing-not-ready branch, not to the PathType false branch. The facing accept condition is:

```text
((RateTimerCurrent >> 7) + 1) & 0x1FE == 0x80
```

The accepted low-word window is `0x3F80..0x407F` inclusive. If not ready and `+0x6AF == 0`, Mission_Deploy calls locomotor `+0x4C(0x4000)` again and returns delay `5`.

Evidence: `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`; `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`; `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`.

### 3.5 Unload-start field writes

Once path and facing gates accept and `+0x6D1` is not already set, the verified write order is:

1. `+0xF8 = 0`;
2. `+0x6D1 = 1`;
3. read `g_CurrentFrameCounter`;
4. `+0x10C = 1`;
5. `+0x100 = current frame`;
6. `+0x104 = stack value`;
7. `+0x108 = 1`;
8. optional refinery slot-7 animation lookup/call;
9. `+0xBC = 3`;
10. timer epilogue return: mission timer entry `Rate * 900.0`, `ftol`, plus `RandomRanged(0,2)`.

No explicit body-facing write is observed in this unload-start block. The binary requires the facing timer to already be inside the east window; it does not snap the unit's facing to East at this point.

Evidence: `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`.

### 3.6 Periodic accumulator and dump gate

The `+0xF8..+0x110` cluster is not a plain CDTimer. It is a periodic accumulator:

- `+0xF8` counts upward;
- `+0x100/+0x108/+0x10C` define an active one-frame repeating timer in unload state;
- `TechnoClass::AI_Update` increments `+0xF8` by `+0x110` when the timer fires, resets `+0x100`, updates `+0x104`, and reloads `+0x108` from `+0x10C`;
- `Mission_Deploy_Building` state 3 deposits when `HarvesterDumpRate * 900.0 <= +0xF8`.

Existing docs identify `+0x104` as Z/secondary storage and `+0x110` as increment step. However, the current evidence set still has one explicit uncertainty: `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md` says `field_0x110 = 1` is assumed/not yet verified for the dump path. That is a blocker for claiming byte-field parity for the coupled path.

Evidence: `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md`.

### 3.7 State 3 / state 4 exit

Stock `HARV/CMIN -> GAREFN/NAREFN` completion is the zero `+0x2E4` path. The nonzero reciprocal-link path calls `BuildingClass::ReleaseDockedHarvester` and can force track `0x47`, but that is not normal stock refinery completion.

State 3 empty-cargo transition requests slot 8, writes state 4, clears slot 10 if occupied, and direct-returns `1`. Normal state 4 later clears `+0x6D1`, sets mission `0x0A`, optionally sends radio `3` if valid steps exist, queues/commences, and then reaches the timer epilogue.

Evidence: `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`; `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`.

## 4. INI Keys

| INI key | Stock value | Effect in this slice | Evidence | Status |
|---|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | refinery candidates | `ini/rulesmd.ini` | active |
| `[CMIN] Harvester` | `yes` | reaches harvester unload branch | `ini/rulesmd.ini`; `0x0073D678` docs | active |
| `[CMIN] Teleporter` | `yes` | chrono identity; not a Mission_Deploy unload gate | `ini/rulesmd.ini` | active elsewhere |
| `[HARV] Dock` | `NAREFN,GAREFN` | refinery candidates | `ini/rulesmd.ini` | active |
| `[HARV] Harvester` | `yes` | reaches same unload branch as CMIN | `ini/rulesmd.ini`; `0x0073D678` docs | active |
| `[GAREFN]/[NAREFN] DockUnload` | `yes` | radio `0x15` queues sender mission `0x10` | `ini/rulesmd.ini`; radio docs | active |
| `[GAREFN]/[NAREFN] Refinery` | `yes` | state-4 refinery checks/animations | `ini/rulesmd.ini` | active |
| `[Enter] Rate` | `.016` | Mission Enter retry delay: `ftol(.016 * 900) + RandomRanged(0,2)` = `14..16` frames | `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_SCHEDULING_GHIDRA_REPORT.md` | active |
| `[General] HarvesterDumpRate` | default `0.016` | state-3 dump threshold: `0.016 * 900.0 = 14.4` | `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md` | active |
| `QueueingCell=4,1` | stock refineries | queue/far-staging context; not the `0x0E` accepted cell and not read by Mission_Deploy state 3/4 | Mission Enter / reachability docs | active elsewhere |

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Status |
|---|---|---|---|
| `Mission_Enter -> Building 0x0E` | one `CAN_DOCK(0x0E)` per mission dispatch | `0x004D9290` reports | verified |
| Building `0x0E -> Unit 0x12` | sends accepted cell, waits for already-there `0x14` | second-call scheduling report | verified |
| Building `0x0E -> 0x18 -> 0x16` | sends `0x18`, then `0x16` after already-there | same | verified |
| First unit `0x16` | can only retarget RateTimer and return `1` if not already at `0x4000` | timing report | verified |
| Later/aligned unit `0x16` | can send `0x15` when idle, has building destination, destination flag, mission `7` | timing report | verified |
| Building `0x15` | queues sender mission `0x10`, does not start unload directly | unload-start verification report | verified |
| Mission `0x10` | path/facing gates and unload-start field writes | unload-start / reachability reports | verified |
| `TechnoClass::AI_Update` | increments `+0xF8` using the periodic accumulator cluster | bale cadence report | touched |
| `Main_Tick` frame counter | gamemd increments near end of tick | RateTimer helper report | verified from prior docs |

## 6. Current Rust Implementation Status

Rust already has a bridge-shaped dock FSM:

- `src/sim/miner/mod.rs` has `FaceSync`, `MissionQueued`, and `Pivoting` phases that describe the split between `0x16`, `0x15`, and mission `0x10`.
- `src/sim/miner/miner_dock_sequence.rs::sync_dock_facing` uses `FacingClass`, target `0x4000`, and the same east-window accept shape.
- `start_unload_deploy` still writes Rust-specific side effects that are not the gamemd field order: `link_on_pad`, `display_type_override`, forced `entity.facing = DOCK_FACING_EAST`, `DockDeploy` sound event, and a local countdown.
- `phase_pivoting` polls `sync_dock_facing` each Rust FSM tick. gamemd's Mission_Deploy not-ready branch returns delay `5`.
- `src/sim/movement/facing_class.rs` is the closest existing Rust primitive to `RateTimer::Set/Current`, but Rust `binary_frame` is currently updated at the start of `advance_tick()` while gamemd increments `g_CurrentFrameCounter` near the end of `Main_Tick` according to the timing docs.

This means a bridge patch can remove the forced facing snap and per-tick polling drift. A full byte-field path still needs the accumulator proof gate closed before implementing `+0xF8..+0x110` as exact state.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Ghidra availability this session | deferred | MCP `list_instances` returned no running instances | start Ghidra MCP for fresh decompile if needed |
| `UnitClass::Receive_Radio(0x16)` first-turn branch | verified | `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md` | none |
| later/aligned `0x16` cascade to `0x15` | verified | same | none for dock-facing design |
| DriveLocomotion `+0x4C` resolution | verified | `DRIVELOCOMOTION_VTABLE_0X4C_TIMING_SYNC_METHOD_GHIDRA_REPORT.md` | none |
| RateTimer Set/Current storage/interpolation | verified | `RATETIMER_CURRENT_FRAME_COUNTER_HELPERS_GHIDRA_REPORT.md`; timing docs | fresh decompile optional |
| Mission_Deploy PathType polarity | verified | `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md` | none |
| Mission_Deploy facing window and return `5` | verified | same; unload-start verification | none |
| unload-start write order through `+0xBC=3` | verified | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` | source of `+0x104` stack value |
| `+0xF8..+0x110` accumulator identity | touched-not-exhausted | `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md` | verify `+0x110` writer/default and `+0x104` source in one slice |
| accepted-path mission timer epilogue formula | verified | unload-start and PathType/state4 docs | exact runtime mission-table storage/decrement broader than this report |
| Mission Enter retry cadence | verified for stock `[Enter]` formula | second-call scheduling report | runtime table value check optional |
| current Rust dock FSM scan | touched | Codegraph + file reads | implementation separate |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is dock 0x16 a forced East body-facing write? -> No. Ordinary first 0x16 calls active Drive locomotor +0x4C with target 0x4000, which wraps RateTimer::Set(owner+0x388).` (evidence: `DRIVELOCOMOTION_VTABLE_0X4C_TIMING_SYNC_METHOD_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-02 - Is RateTimer storage/progression known enough for the dock facing gate? -> Yes: target/source/start/duration/rate fields and Current/Set semantics are documented.` (evidence: `RATETIMER_CURRENT_FRAME_COUNTER_HELPERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-03 - Does Mission_Deploy require exact target 0x4000? -> No, it accepts the quantized window `0x3F80..0x407F`.` (evidence: `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-04 - Which branch returns delay 5? -> The valid-path but facing-not-ready branch.` (evidence: `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-05 - What is PathType::Has_Valid_Steps polarity? -> True proceeds to RateTimer/state dispatch; false cleanup direct-returns 1.` (evidence: `0x0065AE30`, `0x0073DEE2..0x0073DEE9` reports)
- `[RESOLVED] OQ-06 - Does unload-start snap body facing to East? -> No explicit facing write appears in the verified unload-start block.` (evidence: `0x0073DF56..0x0073E09D` in unload-start verification)
- `[RESOLVED] OQ-07 - Is normal stock exit ReleaseDockedHarvester / Force_Track(0x47)? -> No. That is the nonzero reciprocal-link branch, not normal stock zero-link completion.` (evidence: PathType/state4 and reachability reports)
- `[RESOLVED] OQ-08 - Does Mission Enter supply a later 0x16? -> Yes, through later `Mission_Enter -> 0x0E` dispatch after `0x12 == 0x14`; 0x16 does not self-schedule.` (evidence: second-call scheduling report)
- `[RESOLVED] OQ-09 - Is the accepted-path epilogue formula known? -> Yes where it is used: mission timer entry rate times 900, `ftol`, plus `RandomRanged(0,2)`.` (evidence: unload-start and Mission Enter reports)
- `[DEFERRED] OQ-10 - What exact writer/default guarantees `+0x110 == 1` for unload-start periodic accumulation?` (category: `needs-runtime-debugger`; reason: existing report explicitly leaves this as not yet verified; next-step-if-pursued: live Ghidra xref/write audit for UnitClass `+0x110` around constructors, Unlimbo, HarvestBrain_Idle, Harvest_Ore_Tick, and Mission_Deploy)
- `[DEFERRED] OQ-11 - What exact source value is copied into `+0x104` at unload-start?` (category: `needs-runtime-debugger`; reason: existing docs identify Z/secondary storage but not the stack value's full source path in this slice; next-step-if-pursued: live decompile/assembly around `0x0073DFE0..0x0073DFFC` plus caller/local setup)
- `[DEFERRED] OQ-12 - Is Rust `binary_frame` start-of-tick ordering acceptable for this exact dock timer?` (category: `requires-different-system-context`; reason: global tick-counter realignment affects more systems than refinery docking; next-step-if-pursued: timing API design or trace-action against a same-tick start/read case)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| First ordinary `0x16` retargets active Drive RateTimer and returns before `0x15` if not already at `0x4000` | DriveLocomotion and 0x16 reports | Rust has `FaceSync`, but still compresses some side effects into later phases | `src/sim/miner/miner_dock_sequence.rs::phase_face_sync`, `sync_dock_facing` | keep `0x16` as timer sync only: no unload, no sound, no pad link, no body snap | first unsynced accepted-cell pass starts turn and cannot start unload in same step | Do not treat return `1` from first `0x16` as unload-start |
| Mission_Deploy not-ready facing returns delay `5` and calls `+0x4C(0x4000)` | unload-start verification; PathType/state4 report | `phase_pivoting` currently polls every FSM tick | `phase_pivoting`, mission retry/timer fields | introduce a mission-10 retry delay or prove scheduler equivalence before polling again | slow-turn miner cannot start unload during the 5-frame not-ready delay | Do not poll the deploy-facing gate every frame |
| Mission_Deploy accepted start writes unload latch/timer fields and does not snap body facing | unload-start verification | `start_unload_deploy` forces `entity.facing = DOCK_FACING_EAST` | `start_unload_deploy` | preserve current RateTimer-derived facing value; remove/avoid explicit body-facing snap in parity path | facing byte before and after unload-start does not jump except by normal timer progression | Do not hide dock drift with a direct facing assignment |
| PathType false cleanup direct-returns `1`; true proceeds to facing/state dispatch | PathType/state4 report | exact path guard missing | future mission deploy helper | if implementing full mission, preserve true/false polarity and direct return | no-valid-steps fixture clears unload latch and never initializes state 3 | Do not copy older inverted PathType wording |
| Full byte-field unload needs periodic accumulator `+0xF8..+0x110` | bale cadence report | Rust uses local `unload_timer` countdown | `Miner` state, `phase_unloading`, possible shared timer helper | only implement exact byte-field accumulator after `+0x110` and `+0x104` are closed | first deposit threshold follows `HarvesterDumpRate * 900.0 <= +0xF8` with exact step/default | Do not call a countdown bridge byte-field parity |
| Normal stock exit is zero-link state 4, not ReleaseDockedHarvester | reachability reports | Rust direction mostly matches | `phase_departing`, `RefineryDockContacts` | keep normal stock exit independent from reciprocal `+0x2E4` release/track behavior | full CMIN/HARV unload exits without `Force_Track(0x47)` | Do not reuse reciprocal release helper for normal stock unload |

### Stale Docs / Follow-up Docs

- Any design wording that says Approach C is implementation-ready should be narrowed to: RateTimer, PathType, facing-window, and stock zero-link state-4 gates are ready; exact byte-field accumulator parity remains blocked by `+0x110` default/writer and `+0x104` source proof.
- Any doc wording that says dock `0x16` pivots/snaps body facing should be replaced with: first ordinary `0x16` retargets owner `+0x388` through active Drive locomotor `+0x4C(0x4000)` and returns; Mission_Deploy later samples the same timer window.
- Any doc wording that says PathType valid steps return `5` should be replaced with: valid steps proceed to facing/state dispatch; facing-not-ready returns `5`; no-valid-steps cleanup returns `1`.

## Sources

- `docs/research/DRIVELOCOMOTION_VTABLE_0X4C_TIMING_SYNC_METHOD_GHIDRA_REPORT.md`
- `docs/research/RATETIMER_CURRENT_FRAME_COUNTER_HELPERS_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_SCHEDULING_GHIDRA_REPORT.md`
- `docs/research/UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`
- `docs/research/STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`
- `docs/research/miner/STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`
- `docs/research/UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md`
- `docs/research/UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
- Rust scan: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/movement/facing_class.rs`
