# Garrison Abandoned EVA Playback Postfix Trace

Date: 2026-05-27

Scenario: a player-owned occupied `CanBeOccupied` civilian building has its last occupant leave through the normal garrison unload/abandon path. This trace checks only whether Rust emits and app/audio consumes the abandoned-structure cue with the same owner/order/timing as active YR `gamemd.exe` after the recent playback fix.

Status: COMPLETE for the scoped cue path. The previous "not audible" gap is fixed, but native EVA queue semantics remain a mismatch.

## Pipeline

`OrderIntent::Unloading` on occupied player-owned UC building -> Rust live-object-order building pass unloads one occupant -> cargo count `1 -> 0` -> same building pass reconciles empty civilian garrison -> push `SimSoundEvent::StructureAbandoned { owner: pre_revert_owner }` -> write civilian owner -> app tick drains sim sounds -> local-owner gate -> resolve `EVA_StructureAbandoned` to faction sound id -> queue `GameSoundEvent::StructureAbandoned` -> app audio drain calls `SfxPlayer::play_voice_sound`.

Native path: building mission slot 26 ejects occupants when count is positive -> later in the same active `BuildingClass::Update @ 0x0043FB20`, `CanBeOccupied` guard calls `BuildingClass::CheckAutoSellOrCivilian @ 0x00458200` -> empty/non-civilian branch checks human owner and plays abandoned sound/radar/EVA before `ChangeOwner(civilian_house, 0)`.

## Active-YR Evidence

- Stock YR garrisonable civilian structure exists: `[CAGAS01]` is a gas station with `CanBeOccupied=yes` and `MaxNumberOccupants=10` at `ini/rulesmd.ini:19302..19323`.
- `BuildingClass::Update @ 0x0043FB20` is active and calls `BuildingClass__CheckAutoSellOrCivilian()` when `this->Type[0x157b] != 0`.
- `BuildingClass::CheckAutoSellOrCivilian @ 0x00458200` decompile shows the empty branch condition as `GetOccupantCount() == 0 && Owner != civilian_house`; inside that branch it calls `HouseClass__IsHumanPlayer`, `VocClass__PlayAtPos`, `CreateRadarEvent`, `VoxClass__PlayEVA`, then `ChangeOwner(civilian_house, 0)`.
- String anchor check for `EVA_StructureAbandoned` at `0x0081926c` found exactly one referencing function, documented as `BuildingClass__CheckAutoSellOrCivilian`.
- `ini/evamd.ini:1425..1431` defines `[EVA_StructureAbandoned]`: `Allied=ceva108`, `Russian=csof108`, `Yuri=cyur108`, `Type= QUEUE`, `Priority= NORMAL`.

## Stage Verdicts

| Stage | gamemd result | Rust post-fix result | Verdict |
|---|---|---|---|
| 1. Last-occupant unload before empty reconciliation | Building mission slot 26 calls `SellBuilding` when occupant count is positive; later `BuildingClass::Update` reaches the `CanBeOccupied` reconciliation call. For one occupant, the reconciliation reads count `0`. | `tick_passenger_system` takes a live-object order snapshot, and for a `CanBeOccupied` unloading building calls `process_unloading_transport` before `reconcile_civilian_garrison_owner_for_building` in the same entity turn (`src/sim/passenger.rs:266..303`). | PASS |
| 2. Abandoned event owner/order | In `CheckAutoSellOrCivilian`, the human/audio/EVA branch runs before `ChangeOwner(civilian_house, 0)`, so the cue is owned by the pre-revert player. | Rust pushes `SimSoundEvent::StructureAbandoned { owner: current_owner }` before writing `building.owner = civilian_owner` (`src/sim/passenger.rs:514..521`). For input owner `Americans`, the event owner is `Americans`. | PASS |
| 3. Duplicate suppression | After native `ChangeOwner(civilian_house, 0)`, the empty/non-civilian condition is false on the next reconciliation. | Rust emits only when `cargo_empty && !is_civilian_garrison_owner(current_owner)`; the unit test verifies one same-tick event and no second event on the next passenger pass (`src/sim/passenger.rs:1681..1697`). | PASS |
| 4. Local-human gate | Native checks `HouseClass::IsHumanPlayer` before abandoned audio/EVA and before owner revert. | App conversion resolves the event owner, compares it to `local_owner_name`, and continues only on mismatch (`src/app_sim_tick.rs:481..488`). For local `Americans`, the event survives the gate. | PASS |
| 5. Faction sound id | Native cue is `EVA_StructureAbandoned`; for Allied/Americans, `evamd.ini` maps it to `ceva108`. | `eva_faction_key("Americans", roster)` falls through to `Allied` unless the roster maps to Soviet/Yuri (`src/app_building_anim.rs:544..562`), then `app_sim_tick.rs:493..498` looks up `EVA_StructureAbandoned` and falls back to `ceva108`. Concrete Allied output: `ceva108`. | PASS |
| 6. App event drain timing | Native calls abandoned sound/radar/EVA inside the building update before `ChangeOwner`. Exact downstream `VoxClass` audio-start timing relative to frame/audio mixer was not computed here. | Rust drains sim sound events during `advance_fixed_simulation` (`src/app_sim_tick.rs:326`), then calls `drain_sound_events` immediately after the fixed-sim advance in the same app update (`src/app_sim_tick.rs:176..183`). Exact audio-device start timing relative to native was not measured. | UNCHECKED |
| 7. Audible consumption after recent fix | Native abandoned EVA is player-audible for the human pre-revert owner. | `drain_sound_events` now groups `GameSoundEvent::StructureAbandoned` with EVA cues and calls `sfx.play_voice_sound(event.sound_id(), ...)` (`src/app_building_anim.rs:598..607`). This fixes the old silent arm. | PASS |
| 8. Native EVA queue/order semantics | Native `EVA_StructureAbandoned` has `Type= QUEUE` and `Priority= NORMAL` (`ini/evamd.ini:1430..1431`), so the cue participates in EVA queue/priority behavior. | Rust routes it to `SfxPlayer::play_voice_sound`, whose implementation stops any existing voice immediately before appending the new source (`src/audio/sfx.rs:257..285`). No EVA queue/priority model is used. | FAIL |
| 9. Full abandoned cue set | Native branch calls positional sound, radar event, and EVA before owner change. | This trace only verifies EVA playback. Rust still represents the scoped path as one `StructureAbandoned` sound event; exact radar/positional cue parity was not retraced here. | UNCHECKED |

Verdict tally: PASS: 6 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Top Player-Visible Findings

1. Stage 8 - EVA queue/order semantics differ: native `EVA_StructureAbandoned` is `Type=QUEUE`/`Priority=NORMAL`, while Rust cuts off the current voice immediately. Rust: `src/audio/sfx.rs:257..285`. gamemd evidence: active `CheckAutoSellOrCivilian @ 0x00458200` -> `VoxClass__PlayEVA`; `ini/evamd.ini:1430..1431`.

## Adjacent Findings

- This trace does not re-check exit-cell selection, occupant order, scatter, or `Can_Enter_Cell`; those are separate changed-target traces.
- Exact native audio mixer start time and complete `VoxClass` queue internals were not computed. The observable old gap, no audible abandoned EVA at all, is fixed by the current app drain path.
- Native radar/positional abandoned cue behavior remains outside this single EVA playback scenario.

## Sources

- Read-only Ghidra: `BuildingClass::Update @ 0x0043FB20`, `BuildingClass::CheckAutoSellOrCivilian @ 0x00458200`, `BuildingClass` mission slot 26 body `0x0044D880`, string anchor `EVA_StructureAbandoned @ 0x0081926c`.
- Research/docs: `docs/research/CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`, `docs/research/traces/GARRISON_UNLOAD_REVERT_SAME_TICK_POSTFIX_TRACE.md`, `docs/plans/2026-05-04-garrison-sound-plan.md`.
- INI: `ini/rulesmd.ini`, `ini/evamd.ini`.
- Rust source: `src/sim/passenger.rs`, `src/app_sim_tick.rs`, `src/app_building_anim.rs`, `src/audio/sfx.rs`, `src/rules/sound_ini.rs`.
- No Ghidra mutation was performed. No Cargo tests were run because this slot was constrained to one report write.
