# Bridge Repair Sound / EVA Ordering Trace

**Scenario:** Engineer bridge repair fires `RepairBridgeSound=BridgeRepaired` and
`EVA_BridgeRepaired`; verify timing, owner/local-player gating, spatial sound
position, event order relative to repair mutation, and whether sound fires when no
cells mutate.

**Scope:** One mechanic only: the audio/EVA side effects of an engineer entering a
`BridgeRepairHut=yes` building. Bridge-cell repair walker correctness is cited only
as the mutation boundary.

**Report path:** `docs/research/traces/BRIDGE_REPAIR_SOUND_EVA_ORDERING_TRACE.md`

## Summary

The original YR path is active in standard gameplay: `InfantryClass::PerCellProcess`
reaches the `BridgeRepairHut` branch for an engineer entering a CABHUT. In that
branch, gamemd raises the bridge-repair radar/EVA request first, plays the
`RepairBridgeSound` SFX at the CABHUT/building location second, and only then runs
the 5x5 bridge-cell scan and repair dispatch.

The Rust sim queues `SimSoundEvent::BridgeRepaired` before running the 5x5 repair
scan, so the sim-side event production order matches the intended mutation
boundary. The app layer resolves `RepairBridgeSound` and even computes an
`eva_sound_id`, but the playback drain treats `GameSoundEvent::BridgeRepaired` as a
normal spatial SFX and never plays the stored EVA ID. Player-visible result: the
bridge-repair SFX can play, but the "Bridge repaired." EVA line is silent.

## Evidence

### Stock INI Data

- `ini/rulesmd.ini:721`: `RepairBridgeSound= BridgeRepaired`
- `ini/soundmd.ini:5355-5359`: `[BridgeRepaired]`, `Sounds=urepair`,
  `Type=global`, `MinVol=55`, `Volume=55`
- `ini/evamd.ini:49`: dialog list entry `46=EVA_BridgeRepaired`
- `ini/evamd.ini:982-987`: `[EVA_BridgeRepaired]`, text "Bridge repaired.",
  `Russian=csof046`, `Allied=ceva046`, `Yuri=cyur046`

### gamemd Evidence

- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md:383-405` documents the live
  `BridgeRepairHut` branch order: local-human EVA/radar, global
  `RepairBridgeSound`, then 5x5 scan.
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md:900-906` states the observable
  rule: `EVA_BridgeRepaired` is local-human/engineer-owner gated,
  `RepairBridgeSound` plays at the building location, and audio fires before the
  bridge mutation.
- `VOXCLASS_PLAYEVA_FFFFFFFF_SENTINEL_GHIDRA_REPORT.md:117-134` verifies the
  caller loads the literal name `EVA_BridgeRepaired`, calls
  `VoxClass::PlayEVA`, and separately triggers `RepairBridgeSound`.
- `VOXCLASS_PLAYEVA_FFFFFFFF_SENTINEL_GHIDRA_REPORT.md:136-150` confirms this is
  YR-active standard bridge repair, not dormant TS legacy.
- `GLOBAL_SOUNDS_GHIDRA_REPORT.md:255` maps `RepairBridgeSound` to
  `RulesClass+0x248`, stock default `BridgeRepaired`.

### Rust Evidence

- `src/sim/world/world_orders.rs:250-261`: bridge repair order contract says emit
  sound, run 5x5 repair, despawn engineer.
- `src/sim/world/world_orders.rs:330-345`: `SimSoundEvent::BridgeRepaired` is
  pushed with the CABHUT cell and engineer owner before
  `repair_bridge_from_engineer_scan`.
- `src/sim/world/world_orders.rs:350-361`: bridge-state dirty flag is derived after
  the repair outcome, then engineer is consumed.
- `src/sim/world/mod.rs:176-182`: sim event is documented as building-cell SFX plus
  local-human EVA gating in the app layer.
- `src/app_sim_tick.rs:530-571`: app conversion resolves stock repair SFX, computes
  screen position from the CABHUT cell, and resolves `EVA_BridgeRepaired` if the
  engineer owner matches the local owner.
- `src/audio/events.rs:143-158`: `GameSoundEvent::BridgeRepaired` carries both
  `sound_id` and `eva_sound_id`.
- `src/app_building_anim.rs:580-630`: playback ignores `eva_sound_id`; the
  `BridgeRepaired` event falls through to generic spatial SFX playback and only
  calls `event.sound_id()`.
- `src/sim/world/world_orders_bridge_repair_tests.rs:448-470`: existing test
  confirms a bridge-repair event is emitted and the engineer is consumed even when
  the bridge is already intact, with no bridge-state change.
- `src/sim/world/world_orders_bridge_repair_tests.rs:504-521`: existing test
  confirms a bridge-repair event is emitted and the engineer is consumed even when
  the scan finds no bridge cells, with no bridge-state change.

## Stage Results

| Stage | Boundary Checked | gamemd Output | Rust Output | Verdict |
|---|---|---|---|---|
| 1 | Stock data: `RepairBridgeSound` | Sound index for `BridgeRepaired` (`RulesClass+0x248`, stock slot 0x92) | `BridgeRules.repair_sound = Some("BRIDGEREPAIRED")`, case-insensitive lookup | UNCHECKED |
| 2 | Trigger branch is YR-active | Engineer + CABHUT branch in `InfantryClass::PerCellProcess` | `tick_bridge_repair_orders` handles engineer `capture_target` pointing at `BridgeRepairHut=yes` | UNCHECKED |
| 3 | Sim event order vs repair mutation | EVA/radar, SFX, then 5x5 scan/walker | `SimSoundEvent::BridgeRepaired` queued before `repair_bridge_from_engineer_scan` | PASS |
| 4 | Actual playback order vs repair mutation | `PlayEVA`/`VocClass::PlayAt` called before scan/walker mutation | app converts/drains sim sound events after the sim tick has already mutated bridge state | FAIL |
| 5 | SFX ID and emission | One `RepairBridgeSound=BridgeRepaired` SFX request when Rules sound is not `-1` | One `GameSoundEvent::BridgeRepaired { sound_id: "BRIDGEREPAIRED" }` when rules sound is set | PASS |
| 6 | SFX spatial anchor | Building/CABHUT location, not engineer cell | CABHUT cell `(brx,bry)` converted through `iso_to_screen(rx, ry, 0)` | UNCHECKED |
| 7 | EVA request identity | `PlayEVA("EVA_BridgeRepaired", -1, -1)` | `eva_registry.get("EVA_BridgeRepaired", faction)` computed into `eva_sound_id` | PASS |
| 8 | EVA local-owner gate | `HouseClass::IsHumanPlayer()` and successful `CreateRadarEvent`; engineer owning house | local owner name equals engineer owner; no radar-event success gate | UNCHECKED |
| 9 | EVA playback | Voice queue receives `EVA_BridgeRepaired` (`ceva046`/`csof046`/`cyur046`) | `eva_sound_id` is never read by playback; no EVA plays | NOT-IMPLEMENTED |
| 10 | No-cell-mutation case | Audio happens before scan, so branch-triggered audio still fires if scan mutates 0 cells | event is queued before scan; tests cover intact and empty-scan no-mutation cases | PASS |

## Findings

### NOT-IMPLEMENTED: `EVA_BridgeRepaired` is resolved but never played

Player-visible difference: the player does not hear "Bridge repaired." after a
successful engineer bridge repair. gamemd calls
`PlayEVA("EVA_BridgeRepaired", -1, -1)` on the YR-active bridge-repair branch.
Rust computes `eva_sound_id` in `src/app_sim_tick.rs:549-560`, stores it in
`GameSoundEvent::BridgeRepaired`, but `src/app_building_anim.rs:580-630` never
matches that field or calls the voice/EVA playback path for it.

Severity: high for engineer bridge repair, because this is the primary global
feedback line for the action.

### FAIL: actual audio playback is after the sim mutation, not before it

gamemd's call order is EVA/radar, SFX, then scan/walker. Rust queues the sim event
before the repair scan, but app-level conversion and playback occur only after the
sim tick returns, when bridge state has already been mutated. This is not literal
ordering parity. It may be visually hard to perceive in one frame, but it matters
for exact event sequencing and any future audio/radar side effects tied to the
mutation boundary.

### UNCHECKED: exact spatial position and attenuation are not numerically verified

Rust anchors the SFX to the CABHUT cell, matching the documented semantic anchor.
However, this trace did not compute gamemd's final sound pan/volume or compare it
numerically against Rust's `iso_to_screen(rx, ry, 0)` plus
`calc_spatial_volume`. Exact spatial parity remains unchecked.

### UNCHECKED: local-player gating is not identical at the predicate level

gamemd gates EVA on `HouseClass::IsHumanPlayer()` plus successful radar-event
creation. Rust gates on `local_owner_name == engineer_owner` and resolves the
faction key from the house roster. Because EVA playback is not implemented for this
event, this predicate difference is currently masked, but it must be rechecked when
the EVA arm is wired.

## Adjacent Findings

- `soundmd.ini` uses `MinVol=55` on `[BridgeRepaired]`, while the Rust parser reads
  `MinVolume`. This trace did not verify whether gamemd accepts `MinVol` as an
  alias or falls back to the default, so no verdict is assigned here.
- Bridge repair radar-event type 14 is documented in
  `RADAR_EVENT_CLASS_GHIDRA_REPORT.md`, but this trace did not inspect Rust radar
  event emission for bridge repair.

## Verdict Tally

PASS: 4 | FAIL: 1 | UNCHECKED: 4 | NOT-IMPLEMENTED: 1

## Status

COMPLETE
