# BUNKER_0X2E4_LIFECYCLE_EXIT_CLEAR_PATH_GHIDRA_REPORT

Date: 2026-05-23
Target: YR `BuildingClass+0x2E4` bunker lifecycle and active clear/exit paths.

## Working Notes

Target question: How does active YR bunker single-slot `+0x2E4` lifecycle enter, install/hide, exit/clear, and play up/down sounds?
Non-goals: Civilian `CanBeOccupied` garrison occupant vector, ordinary garrison fire/kill-credit behavior, full refinery dock lifecycle, and row-helper pathing consequences beyond identifying bunker occupancy separation.
Evidence needed to mark COMPLETE: Exact parser/default offsets for `Bunker` and `Bunkerable`; decompile plus assembly/xref evidence for bunker radio handoff and `MissionRepairAndProduce` caller; decompile plus assembly/xref evidence for install, release, sell/destruction clear helpers; stock INI evidence for `NATBNK`/`NABNKR`; Rust-facing handoff deltas.
Stop conditions: Stop once these are covered, or record unresolved helper/path liveness in Remaining Uncertainty; do not expand into `PenetratesBunker`, civilian garrison, or refinery parity.

## Executive Result

Status: COMPLETE.

The active YR bunker is a separate, single contained-unit link, not the civilian garrison occupant vector. The link is stored both ways: `BuildingClass+0x2E4` points at the contained unit and the contained unit's `TechnoClass/FootClass+0x2E4` points back at the building. Entry is driven through `BuildingClass::Receive_Radio` bunker branches and then `BuildingClass::MissionRepairAndProduce`, whose only xref into the bunker state machine is `0x0044B7A3 -> 0x00458E50`. Install writes both links, hides/limbos the unit, raises the bunker, and plays `BunkerWallsUpSound`. Normal exit from a bunkered unit's deploy mission uses a fuller release helper that un-hides/repositions the unit; sell/destruction/temporal/super clear paths use related helpers that clear reciprocal links and/or undock without the same passable-cell placement step.

Stock retail data in the checked `rulesmd.ini` activates this exact path for `NATBNK` (`Bunker=yes`). `NABNKR` is live as a listed Soviet defense, but the checked stock section does not set `Bunker=yes`; therefore it is not proven to reach the `BuildingTypeClass+0x16AB` bunker state machine without an override.

## Verified Binary Findings

### Type Flags And Defaults

- Active in YR: Conditional. `BuildingTypeClass::ReadINI` reads `Bunker` from INI string `0x0081AADC` and stores the result at `BuildingTypeClass+0x16AB`; assembly `0x00460941..0x00460954` pushes `"Bunker"`, calls the bool reader, and writes `AL` to `[EBP+0x16AB]`.
- Active in YR: Yes. `TechnoTypeClass::ReadINI` reads `Bunkerable` from string `0x0084371C` and stores it at `TechnoTypeClass+0xD2E`; assembly `0x00715003..0x0071501E` uses the prior `[EBP+0xD2E]` as default, pushes `"Bunkerable"`, calls the bool reader, and writes `AL` back to `[EBP+0xD2E]`.
- Active in YR: Yes. Unit types default `Bunkerable=true`: `UnitTypeClass__Constructor` writes `1` to `[ESI+0xD2E]` at `0x007472AA`. Aircraft, infantry, and building type constructors write or retain false at the same offset; this matches stock INI comments such as `rulesmd.ini:7008` saying units default yes and others no.
- Active in YR: Conditional. `TechnoClass__CanAutoDeployHere @ 0x0070FB50` gates bunker entry on `TechnoTypeClass+0xD2E != 0`, turret presence, deploy compatibility, and movement-zone/other runtime checks. Its relevant xrefs include `BuildingClass__Receive_Radio` at `0x0043C512` and `0x0043C86A`.

### Stock INI Activity

- Active in YR: Yes for `NATBNK`. `rulesmd.ini:13722` defines `[NATBNK]`, and `rulesmd.ini:13732` sets `Bunker=yes`; `artmd.ini:5019..5053` defines the 2x2 tank bunker and its up/down special animation slots.
- Active in YR: No for the checked stock `NABNKR` bunker state-machine path. `rulesmd.ini:12979` defines `[NABNKR]`, and `rulesmd.ini:3085` includes it in Soviet base defenses, but the checked section has no `Bunker=yes`; no binary path reaches `0x00458E50` unless `BuildingTypeClass+0x16AB` is set.
- Active in YR: Yes. Global sounds are parsed in `RulesClass__ReadAudioVisual`: `BunkerWallsUpSound` at `RulesClass+0x240` from `0x00669E87`, and `BunkerWallsDownSound` at `RulesClass+0x244` from `0x00669EC8`. Stock values are `rulesmd.ini:719` `TankBunkerUp` and `rulesmd.ini:720` `TankBunkerDown`.

### Entry And Install

- Active in YR: Conditional. `BuildingClass__Receive_Radio` case `0x0F` has a `Bunker=yes` branch that calls `TechnoClass__CanAutoDeployHere` at `0x0043C512`; if false it returns reject code `10`, and if a `vtable+0x278(0x23, sender)` contact check returns `1` it also returns `10`; otherwise it returns `1`.
- Active in YR: Conditional. `BuildingClass__Receive_Radio` case `0x15` sets a bunker building runtime flag and calls `MissionSet(0x14, 0)` for `Bunker=yes` buildings, handing execution to `BuildingClass__MissionRepairAndProduce`; this is the active handoff into the install state machine.
- Active in YR: Conditional. `BuildingClass__MissionRepairAndProduce @ 0x0044B7A3` is the only xref to the state machine at `0x00458E50`; the caller is gated by `BuildingTypeClass+0x16AB`.
- Active in YR: Conditional. State machine `0x00458E50` uses `BuildingClass+0x2E4` as the contained-unit slot. If no candidate exists, it calls `FootClass__GetDestination(0)`; if the candidate is missing or not `WhatAmI()==1` (unit), it resets `BuildingClass+0x718` and missions the building back to `5`.
- Active in YR: Conditional. Install state writes both reciprocal links: at `0x00459301` `building+0x2E4 = unit`, and at `0x0045930F` `unit+0x2E4 = building`. It also writes `unit+0x214 = -1`, calls the unit `vtable+0x150` hide/limbo-style method, sets `BuildingClass+0x718 = 6`, missions the contained unit with `MissionSet(5,1)`, and plays `RulesClass+0x240` if not `-1` via `VocClass__PlayAt` at `0x0045933D..0x0045936F`.

### Exit, Sell, Destruction, And Clear

- Active in YR: Conditional. A bunkered unit exits through `UnitClass__Mission_Deploy_Building @ 0x0073D630` when `unit+0x2E4 != 0`: it looks up the building in the current cell and calls `BuildingClass__ReleaseDockedHarvester @ 0x004595C0` at `0x0073D66D`.
- Active in YR: Conditional. `BuildingClass__ReleaseDockedHarvester @ 0x004595C0` is the full release path for reciprocal `+0x2E4` links. It clears anim slots 10 and 11, plays `RulesClass+0x244` if set at `0x004595D0..0x00459612`, creates down/exit animations from `BuildingTypeClass+0x127C/+0x128C` and `+0x12C0/+0x12D0`, clears `unit+0x2E4`, calls the unit locomotor `+0x58`, calls locomotor `+0x70` with facing/frame argument `0x47` and building offset `(-0x80,+0x80)`, applies unit `vtable+0x544(0, 0x3ff00000)`, finds a nearby passable cell from the building foundation anchor `(-1,+1)`, places the unit with `vtable+0x480(cell,1)`, missions it to `2`, then clears `building+0x2E4`, clears `building+0x718`, missions the building to `5`, and sends radio `3`.
- Active in YR: Conditional. `BuildingClass__UndockUnit @ 0x004593A0` is used by sell/destruction/temporal building paths. Xrefs are `BuildingClass__Sell @ 0x0044AAB0`, `BuildingClass__ReceiveDamage @ 0x004424EA`, and `TemporalClass__Update @ 0x0071AA15`. It un-stops/re-heads the unit through locomotor `+0x58/+0x70`, applies `vtable+0x544`, clears both `+0x2E4` links, and sends building radio `3`, but does not run the full nearby-passable-cell ejection sequence.
- Active in YR: Conditional. `BuildingClass__Sell @ 0x0044AA00` checks `[EBP+0x2E4]`, and if nonzero calls `0x004593A0`; assembly `0x0044AAA4..0x0044AAB0` is `MOV EAX,[EBP+0x2E4]`, `TEST`, conditional skip, then `CALL 0x004593A0`.
- Active in YR: Conditional. `BuildingClass__ReceiveDamage` destruction case `4` checks `field_0x2E4` and calls `BuildingClass__UndockUnit`; assembly xref `0x004424EA -> 0x004593A0`.
- Active in YR: Conditional. `FUN_00459470` is a clear-only bunker/dock-link helper reached from `SuperClass__Launch @ 0x006CC955`, `TemporalClass__Update @ 0x0071AA90`, and `UnitClass__ReceiveDamage @ 0x00737D97`. It clears anim slots, plays `RulesClass+0x244` when occupied, creates down/exit animations, sends radio `3`, clears `unit+0x2E4`, clears `building+0x2E4`, clears `building+0x718`, and missions the building to `5`; it does not unhide/place/mission the contained unit.

## Separation From Civilian Garrison

- Active in YR: Yes. This target is not the civilian `CanBeOccupied` occupant-vector system. The bunker path is a single reciprocal `+0x2E4` link and requires `BuildingTypeClass+0x16AB Bunker=yes` plus `TechnoTypeClass+0xD2E Bunkerable`; ordinary civilian garrison uses a separate building occupant collection and different entry/fire/ejection logic.

## Current Rust Surface

- Active in Rust now: partial. `ObjectType.bunker` exists and is parsed from `Bunker` in `src/rules/object_type.rs:653` and `src/rules/object_type.rs:1077`, with a test at `src/rules/object_type.rs:1738`.
- Active in Rust now: partial. `GameEntity.bunker_occupant` exists at `src/sim/game_entity.rs:276` and feeds current movement/pathing gates in `src/sim/movement/movement_occupancy.rs:191` and `src/sim/movement/bump_crush.rs:147`.
- Active in Rust now: partial. `RulesGeneral.bunker_walls_down_sound` exists at `src/rules/ruleset.rs:267` and is parsed at `src/rules/ruleset.rs:902`; `BunkerWallsUpSound` and a separate bunker up/down sound event surface are missing. Current `RefineryExitSfx` naming/use around `src/app_sim_tick.rs:521` is semantically stale for bunker down events.
- Active in Rust now: No. No `Bunkerable` parser/default surface, no radio entry handoff/state machine, no install hide/link write, no normal exit release, and no sell/destruction reciprocal clear path were found.

## Implementation Handoff

1. Verified behavior: Only `Bunker=yes` buildings with `Bunkerable` unit candidates enter the single-slot `+0x2E4` state machine; stock checked `NATBNK` qualifies and checked `NABNKR` does not. Rust delta: add `Bunkerable` to techno/unit type data with unit default true and aircraft/infantry/building default false, and gate bunker entry on it plus current `ObjectType.bunker`. Affected surface: rules parser, object/type model, bunker entry planner. Acceptance scenario: a tank with default `Bunkerable` can request entry to `NATBNK`, an infantry candidate and a unit overridden `Bunkerable=no` cannot, and `NABNKR` does not enter unless modded with `Bunker=yes`. Proposed test name: `bunker_entry_requires_bunkerable_unit_and_stock_natbnk_bunker_flag`. Risk: medium; wrong defaults will make many units incorrectly enter or fail to enter bunkers.

2. Verified behavior: Install writes both reciprocal links, hides/limbos the unit, sets bunker state, and plays `BunkerWallsUpSound`; normal unit deploy exit uses the full release helper, plays `BunkerWallsDownSound`, clears both links, places the unit on a nearby passable cell, and missions it out. Rust delta: implement an explicit bunker lifecycle state or direct equivalent with reciprocal link invariants and separate up/down sound events. Affected surface: sim entity state, mission/radio handling, movement placement, sound event mapping. Acceptance scenario: entering `NATBNK` sets `building.bunker_occupant` and unit back-reference, removes the unit from normal map blocking/targeting, emits up sound once; deploying/exiting clears both links, restores map presence at a nearby passable cell, and emits down sound once. Proposed test name: `bunker_install_and_exit_clear_reciprocal_links_and_emit_wall_sounds`. Risk: high; one-sided link clearing or sound reuse will cause stuck/ghost units and audible parity errors.

3. Verified behavior: Sell and destruction use `BuildingClass__UndockUnit @ 0x004593A0`, while super/temporal/unit-damage edge clear uses `FUN_00459470`; neither should be modeled as civilian garrison ejection. Rust delta: add lifecycle clear paths for building sell/destruction and damage/temporal edge cases that clear both bunker links and state, using full release only where the binary does. Affected surface: building sell/destruction, damage, temporal/super interactions, deterministic world cleanup. Acceptance scenario: selling or destroying an occupied `NATBNK` clears both links and bunker state without leaving the unit referenced as an occupant; clear-only edge cases do not run the normal nearby-passable-cell exit placement. Proposed test name: `occupied_bunker_sell_and_destroy_clear_reciprocal_links_without_garrison_vector`. Risk: high; conflating release and clear-only paths can duplicate units, lose units, or leave stale references.

## Negative Facts / Do Not Do

- Do not implement tank bunkers as civilian `CanBeOccupied` garrisons. Evidence: bunker install writes `building+0x2E4` and `unit+0x2E4` at `0x00459301/0x0045930F`, while civilian garrison uses a separate occupant collection.
- Do not assume `NABNKR` uses the tank-bunker state machine in stock YR. Evidence: checked `rulesmd.ini:12979` has `[NABNKR]`, but only `rulesmd.ini:13732` under `[NATBNK]` sets `Bunker=yes`.
- Do not treat `Bunkerable` as an all-techno default true. Evidence: `UnitTypeClass__Constructor` writes true at `0x007472AA`; aircraft/infantry/building defaults are false and `TechnoTypeClass__ReadINI @ 0x00715003..0x0071501E` preserves the class default.
- Do not model `BunkerWallsDownSound` as a normal refinery unload sound. Evidence: stock zero-link refinery flow does not set the reciprocal `+0x2E4` link; bunker down sounds are in `0x004595C0` and `0x00459470` clear/release paths keyed by `RulesClass+0x244`.
- Do not clear only the building-side occupant pointer. Evidence: install writes both sides at `0x00459301/0x0045930F`; release/clear helpers clear the unit side and building side before resetting building state.

## Remaining Uncertainty

None for this target. Exact method names for vtable calls such as unit `+0x150`, unit `+0x544`, and locomotor `+0x58/+0x70` are inferred from call shape and surrounding behavior; the report relies on observed lifecycle effects and call addresses, not names, for implementation handoff.

## Stale-Doc Replacement Wording Found

- `docs/research/BUNKER_SYSTEM_GHIDRA_REPORT.md`: Replace any Rust status wording claiming Rust does not parse the bunker flag or lacks a runtime bunker occupant field with: "Current Rust parses `Bunker` into `ObjectType.bunker` and stores `GameEntity.bunker_occupant`; remaining gaps are `Bunkerable` parsing/defaults, radio entry handoff, install hide/link semantics, full release/clear lifecycle, `BunkerWallsUpSound`, and separating bunker down sound from refinery exit naming."
- `docs/research/BUNKER_SYSTEM_GHIDRA_REPORT.md`: Replace wording implying both `NATBNK` and `NABNKR` are active stock `Bunker=yes` tank-bunker state-machine users with: "Checked stock `rulesmd.ini` sets `Bunker=yes` on `[NATBNK]`; `[NABNKR]` is live as a listed Soviet defense but does not set `Bunker=yes` in the checked section, so it is not proven to reach the `BuildingTypeClass+0x16AB` bunker lifecycle without an override."
- `src/audio/events.rs`: Replace the `RefineryExitSfx` comment wording "Fires every refinery dock cycle" with: "Currently used for the verified stock zero-link refinery non-event test and for provisional bunker down playback; bunker down should move to a dedicated bunker wall sound event when the bunker lifecycle is implemented."

