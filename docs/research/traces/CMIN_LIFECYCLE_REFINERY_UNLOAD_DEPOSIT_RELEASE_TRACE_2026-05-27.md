# CMIN Lifecycle Trace - Refinery Unload Deposit Release

Date: 2026-05-27
Trace slot: Chrono Miner mining lifecycle - refinery unload deposit release
Scenario: one full stock `CMIN` with full ore cargo arrives at one stock `GAREFN`, performs contact admission, dock handoff, unload deposit, release, and handoff back to harvest scheduling.
Verdict tally: PASS: 4 | FAIL: 1 | UNCHECKED: 6 | NOT-IMPLEMENTED: 1
Status: COMPLETE

## Scope

This trace covers only the healthy stock zero-link `CMIN -> GAREFN` unload path. It does not trace close/far return selection, destroyed refinery aborts, two-miner queue takeover, War Miner, Slave Miner, Yuri refinery, modded refinery `ProductionAnim`, or non-stock reciprocal `+0x2E4` release helpers.

Assumption fixed for concrete value checks: full ore CMIN cargo, not gems or mixed cargo. Stock `CMIN` has `Storage=20`, `UnloadingClass=CMON`, `Harvester=yes`, `Dock=NAREFN,GAREFN`, and `Teleporter=yes`; stock `GAREFN` has `DockUnload=yes`, `Refinery=yes`, `SpecialAnim=GAREFNOR`, and no active `ProductionAnim`.

## Pipeline

`Mission_Harvest/Enter arrival` -> `HELLO contact admission` -> `CAN_DOCK accepted-cell/contact flag` -> `0x15 queues mission 0x10` -> `Mission_Deploy_Building facing gate` -> `unload-active state 3` -> `slot dump gates` -> `credits + SpecialAnim` -> `empty-slot state 4` -> `release/contact cleanup` -> `SearchOre/Harvest scheduling`.

## Stage Verdicts

| Stage | Rust surface | gamemd evidence | Concrete check | Verdict |
|---|---|---|---|---|
| 1. Stock activation and data | `rulesmd.ini`, `artmd.ini`, `MinerConfig` | Stock INI plus `UnitClass::Mission_Deploy_Building @ 0x0073D630` active for `Harvester=yes` | `CMIN Storage=20`, full ore value `20*25=500`; `GAREFN DockUnload=yes`; no active `ProductionAnim` | PASS |
| 2. HELLO/contact admission | `RefineryDockContacts::hello_or_wait` at `src/sim/miner/miner_dock.rs:42` | Contacts array `RadioClass +0xE4/+0xE8` owns stock admission | Both should accept one miner into one empty refinery contact slot, but exact frame and capacity payload were not computed side by side | UNCHECKED |
| 3. CAN_DOCK/contact flag | `phase_mission_enter`, `mark_contact_entered` at `src/sim/miner/miner_dock_sequence.rs:794` and `:850` | Building `0x0E` sends `0x18` then `0x16`; Techno `+0x418` is mirrored by `0x18` | Rust has a contact-entered map, but exact mirrored endpoint bytes and second-call frame ordering were not computed | UNCHECKED |
| 4. `0x15` queue boundary | `MissionQueued` at `src/sim/miner/mod.rs:108`; `phase_mission_queued` at `src/sim/miner/miner_dock_sequence.rs:921` | `BuildingClass::Receive_Radio(0x15)` stock branch only queues sender mission `0x10` and returns `1` | Rust produces no cargo drain, no display override, no DockDeploy sound, and no pad link in the queued phase | PASS |
| 5. Mission `0x10` pivot/facing gate | `phase_pivoting` at `src/sim/miner/miner_dock_sequence.rs:973` | `0x0073DF56..0x0073DFBC` gates unload start on rate/facing; not-ready returns `5` | Rust samples a `FacingClass` and schedules a delay, but exact frame equality for the concrete CMIN arrival was not computed | UNCHECKED |
| 6. Unload-active / display class | `start_unload_deploy` at `src/sim/miner/miner_dock_sequence.rs:950`; renderer uses `display_type_override` at `src/app_instances/units.rs:167` | gamemd sets `Unit+0x6D1=1`, timer cluster, state `+0xBC=3`; draw uses `UnloadingClass=CMON` while latch is live | Rust sets `unload_active=true` and `CMON`, but pixel equality and exact first rendered frame were not captured | UNCHECKED |
| 7. Refinery start-unload anim slot | no observed Rust equivalent in `start_unload_deploy`; deposit-only events at `src/sim/miner/miner_dock_sequence.rs:1094` | gamemd start-unload init calls `BuildingClass::SetAnimSlotImage(slot=7, ...)` if adjacent refinery is found | Rust only emits `BaleDepositEvent` on real slot drains for slot-10/SpecialAnim; no start-unload slot-7 event was found | NOT-IMPLEMENTED |
| 8. First dump gate timing | accumulator at `src/sim/miner/miner_dock_sequence.rs:136` and `phase_unloading` at `:1027` | state 3 compares `HarvesterDumpRate * 900.0 = 14.4` against `Unit+0xF8` | Rust threshold is `unload_accumulator * 10 >= 144`; exact first drain frame versus gamemd was not runtime-computed | UNCHECKED |
| 9. Full ore credit deposit | `phase_unloading` at `src/sim/miner/miner_dock_sequence.rs:1031` and `:1072` | gamemd drains first non-empty `StorageClass` slot, credits refinery owner, then resets `Unit+0xF8` | Full ore CMIN drains one ore slot worth `20*25=500` credits to refinery owner in Rust and gamemd | PASS |
| 10. Deposit visual event | `BaleDepositEvent` at `src/sim/miner/miner_dock_sequence.rs:1096`; app consumes at `src/app_building_anim.rs:343` | gamemd requests/uses refinery SpecialAnim slot 10 per real slot drain | Rust emits one event per slot, but exact same-frame anim start, frame index, YSort, and particle count were not computed | UNCHECKED |
| 11. Empty-slot handoff | `phase_unloading` at `src/sim/miner/miner_dock_sequence.rs:1106` | empty `FindFirstNonEmptySlot == -1` writes state 4; stock `GAREFN` has no active slot-8 `ProductionAnim` wait | Rust sets `deposit_cooldown_ticks=0` and advances directly to `Departing`; no extra post-empty dump interval | PASS |
| 12. State-4 release/contact cleanup | `phase_departing` and `release_contact` at `src/sim/miner/miner_dock_sequence.rs:1129`, `src/sim/miner/miner_dock.rs:124` | DockUnload state 4 clears `Unit+0x6D1` and queues harvest; `+0x418` is cleared by later `0x19`/break cleanup, not by state 4 itself | Rust clears contact and `contact_entered` during state-4 handoff, so the modeled `+0x418`-like state is cleared too early/directly | FAIL |

## Findings

1. NOT-IMPLEMENTED - Start-unload refinery anim slot 7 is absent. Player-visible difference: the refinery may miss the binary's immediate unload-start building animation cue before the first deposit pulse. Rust evidence: `start_unload_deploy` sets miner display/latch only at `src/sim/miner/miner_dock_sequence.rs:950`; deposit events begin later at `:1094`. gamemd evidence: `UnitClass::Mission_Deploy_Building @ 0x0073E013..0x0073E08E` calls `BuildingClass::SetAnimSlotImage(slot=7, ...)` when the adjacent refinery is found.

2. FAIL - State-4 release clears contact-entered too early/directly. Player-visible difference: the refinery can become available to a waiting miner on Rust's state-4 handoff rather than through gamemd's later radio cleanup timing, so handoff order can differ in normal two-miner play. Rust evidence: `phase_departing` calls `release_contact` at `src/sim/miner/miner_dock_sequence.rs:1146`; `release_contact` clears `contact_entered` at `src/sim/miner/miner_dock.rs:129`. gamemd evidence: `UnitClass::Mission_Deploy_Building @ 0x0073D630` state 4 clears `Unit+0x6D1` and schedules harvest but does not clear `Techno+0x418`; `TechnoClass::Receive_Radio(0x19)` owns that clear.

## Adjacent Findings

- Rust's `on_pad` map is not used by the healthy stock unload start in current source, which matches the verified zero-`+0x2E4` stock path for this scenario. Older docs/tests still mention physical pad occupancy, but no production `link_on_pad` call was found in the healthy path.
- Exact two-miner takeover frame order remains a separate trace target. This slot only identified the state-4 contact-clear mismatch that can affect that takeover.
- Exact pixel equality for `CMON` draw, `GAREFNOR` SpecialAnim frame start, particle/smoke details, and layer/YSort remains UNCHECKED.

## Sources

- `docs/research/REFINERY_PAD_LINK_OCCUPANCY_LIFECYCLE_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`
- `docs/research/RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`
- `docs/research/RADIO_0X18_CONTACT_FLAG_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/miner/EMPTY_SLOT_UNLOAD_GATE_TO_STATE4_RELEASE_TIMING_GHIDRA_REPORT.md`
- `docs/research/miner/STOCK_REFINERY_RADIO_0X08_GLOBAL_SENDERS_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`, `ini/artmd.ini`
- `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/mod.rs`, `src/app_building_anim.rs`, `src/app_instances/units.rs`
