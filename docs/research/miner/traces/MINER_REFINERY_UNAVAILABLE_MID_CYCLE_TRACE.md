# Miner Refinery Unavailable Mid-Cycle Trace

Date: 2026-05-20

Scenario: a standard YR Chrono Miner with cargo is returning to, docking with, or unloading at its chosen owned refinery, and that refinery is destroyed or sold before unloading completes.

Scope: one concrete miner/refinery-loss scenario only. Adjacent queue, normal unload, ore search, and exit-track behavior are referenced only where they are directly consumed by this scenario.

## Summary

Verdict: PARTIAL.

The Rust implementation keeps cargo when the selected refinery disappears, but the fallback state is wrong in several player-visible cases. If the refinery was sold and removed from `EntityStore`, the dock/return resolver notices the missing refinery and sends the miner to `SearchOre`; a full Chrono Miner can then drive back toward ore instead of immediately finding another owned refinery. If the refinery was destroyed by combat but remains in `EntityStore` while its death animation plays (`dying = true`), the dock resolver still treats it as usable and can continue unloading/crediting against a destroyed refinery until the entity is physically removed.

gamemd evidence is active in standard YR:
- `UnitClass::Mission_Deploy_Building` at `0x0073D630` is the active harvester unload FSM for HARV/CMIN.
- Live read-only decompile on 2026-05-20 confirmed the state-3 missing-building branch: if `Look_up_building_in_cell()` returns null, it conditionally sends radio `3`, then calls `SetMission(10, 1)`.
- Existing verified docs confirm `BuildingClass::UndockUnit` at `0x004593A0` is called from `BuildingClass::ReceiveDamage` and `BuildingClass::Sell` when a unit is physically on the dock pad, clears both dock links, and preserves undumped storage on the harvester.

## Pipeline

1. Trigger: refinery is sold or destroyed while Chrono Miner has cargo and is returning/docking/unloading.
2. gamemd interrupt: if the unit is physically linked at the pad, `BuildingClass::UndockUnit` ejects it, clears both dock links, and sends BREAK(3).
3. gamemd unload fallback: next `Mission_Deploy_Building` state-3 tick cannot find a refinery at the dock lookup cell, sends conditional CLEAR_LINK, and sets Mission_Harvest.
4. Rust trigger: sell removes the building immediately; combat death marks it `dying` and leaves it in `EntityStore` through death animation.
5. Rust miner tick: `tick_miners` cleans reservations for non-alive ids, then `handle_return` or `handle_dock_sequence` resolves the reserved refinery id.
6. Rust fallback: missing reserved refinery changes miner to `SearchOre`; dying refinery still resolves and the normal dock/unload phases continue.
7. Player result: sold-refinery case keeps cargo but may seek ore while full; combat-destroyed case can visibly keep unloading and grant credits from a destroyed refinery.

## Stage Table

### Stage 1 - Trigger Entry Points

Rust:
- Sell command dispatches `production::sell_building` from `src/sim/world/world_commands.rs:624`.
- `sell_building` removes the structure with `sim.entities.remove(stable_id)` at `src/sim/production/production_sell.rs:563`.
- Combat-destroyed structures set `combat_result.structure_destroyed`; destroyed entities remain in `EntityStore` while `dying` until animation removal at `src/app_sim_tick.rs:284-299`.

gamemd:
- `BuildingClass::UndockUnit` report confirms callers: `BuildingClass::ReceiveDamage`, `BuildingClass::Sell`, and temporal wipe.
- `REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md` confirms ReceiveDamage call site at `0x004424EA`; `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md` confirms sell call.

Verdict: UNCHECKED. Entry points are identified, but no exact same-tick command-order equality was computed against gamemd for sell vs combat in this trace.

### Stage 2 - Reservation Release

Rust:
- `tick_miners` builds `alive_sids` from `!e.dying` entities and calls `dock_reservations.cleanup_dead(&alive_sids)` at `src/sim/miner/miner_system.rs:116-125`.
- `cleanup_dead` removes dead refineries from `occupied` and `queues` at `src/sim/miner/miner_dock.rs:89-93`.

gamemd:
- `UndockUnit` clears building+0x2E4 and unit+0x2E4 and sends BREAK(3); docs and live decompile show radio command `3`, not `7` or `0x19`.

Verdict: UNCHECKED. Both sides clear a reservation/contact concept, but the Rust `DockReservations` structure is not numerically comparable to gamemd Contacts/+0x2E4 fields in this trace.

### Stage 3 - Return-To-Refinery Fallback Before Dock

Rust:
- If `reserved_refinery` exists but `refinery_dock_for_sid` fails, `handle_return` clears `reserved_refinery` and sets `state = SearchOre` at `src/sim/miner/miner_system.rs:643-648`.
- `handle_search_ore` does not first check whether cargo is already full; it immediately uses archive/search paths at `src/sim/miner/miner_system.rs:299-360`.

gamemd:
- `UnitClass::Mission_Harvest` state 0 checks storage percentage first; if full, it transitions to state 2 (return to refinery). This is documented in `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md` and active in standard YR.

Scenario output:
- For a full Chrono Miner whose chosen refinery disappears before docking, gamemd remains in the return/refinery-selection path; Rust enters `SearchOre` and can path to ore even though capacity is already full.

Verdict: FAIL.

### Stage 4 - Dock/Unload Missing-Refinery Fallback

Rust:
- `handle_dock_sequence` calls `resolve_refinery_cells`; if the ref id no longer resolves, it clears `reserved_refinery`, sets dock phase `Approach`, clears `exit_cell`, and sets `state = SearchOre` at `src/sim/miner/miner_dock_sequence.rs:363-371`.
- This branch does not clear `dock_queued`, `movement_target`, or `display_type_override`.

gamemd:
- Live decompile of `UnitClass::Mission_Deploy_Building` at `0x0073D630` shows state 3 missing-building branch: if `Look_up_building_in_cell()` returns null, it conditionally sends radio `3`, then calls `SetMission(10, 1)`. Existing docs state remaining ore stays in the harvester's storage.

Scenario output:
- Cargo is kept in Rust, matching the storage-preservation part.
- Fallback state is not equivalent for a full cargo miner: Rust moves to `SearchOre` instead of immediately re-entering/refinding refinery behavior.
- Visual state can remain in the unloading model because `display_type_override` is set on link at `src/sim/miner/miner_dock_sequence.rs:454-457` and only cleared in normal deposit cooldown at `src/sim/miner/miner_dock_sequence.rs:624-632`.

Verdict: FAIL.

### Stage 5 - Combat Destruction While Death Animation Is Playing

Rust:
- Reservation cleanup treats a dying refinery as not alive at `src/sim/miner/miner_system.rs:116-125`.
- `resolve_refinery_cells` still accepts the same dying refinery because it only checks `sim.entities.get(ref_sid)?` and never checks `entity.dying` or `entity.is_alive()` at `src/sim/miner/miner_dock_sequence.rs:301-329`.
- `phase_unloading` then credits the refinery owner from that dying building at `src/sim/miner/miner_dock_sequence.rs:542-585`.

gamemd:
- `MINER_DOCK_GAPS_RESEARCH.md` says ReceiveDamage death case calls `UndockUnit` when the dock link exists; `UnitClass::Mission_Deploy_Building` then sees no building at the lookup cell and bails without crediting remaining storage.

Scenario output:
- Rust can keep unloading and award credits against a destroyed refinery until the corpse entity is removed.
- gamemd aborts/ejects once the refinery dies; no further ore slot is credited after the building is unavailable.

Verdict: FAIL.

### Stage 6 - Cargo Preservation

Rust:
- Missing-refinery branches in `handle_return` and `handle_dock_sequence` do not clear `snap.miner.cargo`.
- `phase_unloading` removes cargo only when a slot drain fires at `src/sim/miner/miner_dock_sequence.rs:542-551`.

gamemd:
- Verified docs state undumped storage remains on the harvester; live decompile confirms the missing-building branch skips the drain block and sets Mission_Harvest.

Verdict: PASS for the sold/missing-entity branch. UNCHECKED for combat destruction because Rust may still drain one or more slots while the dying refinery remains resolvable.

### Stage 7 - Next Refinery/Search/Wait Behavior

Rust:
- Missing reserved refinery before/during dock sets `SearchOre` at `src/sim/miner/miner_system.rs:643-648` and `src/sim/miner/miner_dock_sequence.rs:363-371`.
- If no ore exists, `SearchOre` eventually sets `WaitNoOre` at `src/sim/miner/miner_system.rs:364-366`.
- If another refinery exists, Rust does not immediately select it from the missing-refinery branch; it only does so if the miner later reaches `ReturnToRefinery` or `ForcedReturn`.

gamemd:
- `Mission_Harvest` state 0 full check sends full storage back to state 2; state 2 performs nearest dock search via `FootClass::Find_Docking_Bay`.

Scenario output:
- With full cargo and another owned refinery, Rust can visibly waste time trying to harvest again or waiting for ore before selecting the next refinery.

Verdict: FAIL.

### Stage 8 - Tests

No tests were run. The user constrained this trace to write exactly one file, and running `cargo test` would write build/test artifacts outside that file.

Verdict: UNCHECKED.

## Findings

1. FAIL - Full miner can seek ore after its chosen refinery disappears.
   - Rust: `handle_return` and `handle_dock_sequence` set `SearchOre` on missing ref.
   - Player-visible result: a full Chrono Miner may drive/retarget toward ore instead of finding another refinery.
   - gamemd evidence: `Mission_Harvest` state 0 checks full storage before ore search and transitions to return/refinery selection.

2. FAIL - Destroyed-but-not-despawned refinery remains usable to the miner dock FSM.
   - Rust: `resolve_refinery_cells` accepts entities without checking `dying`.
   - Player-visible result: miner can continue unloading and granting credits from a refinery already destroyed in combat.
   - gamemd evidence: `ReceiveDamage` death case calls `UndockUnit`; later state-3 lookup returns null and bails.

3. FAIL - Missing-refinery dock fallback does not clear unloading visual override.
   - Rust: `display_type_override` is set on link but only cleared by normal cooldown.
   - Player-visible result: miner can remain rendered as `CMON`/unloading model after refinery sale/destruction aborts the cycle.
   - gamemd evidence: interrupt undock clears dock links and the active dock/unload visual path exits; storage remains but unload visual does not continue as a valid docked state.

4. FAIL - Next-refinery behavior is delayed or skipped.
   - Rust: missing refinery sends the miner to `SearchOre`; another refinery is only found later via return logic.
   - Player-visible result: cargo-bearing miner may wait/search instead of immediately selecting another refinery.
   - gamemd evidence: state 2 `Find_Docking_Bay` is the return path for full storage.

## Adjacent Findings

- `sell_building` directly removes the building from `EntityStore` and does not use `Simulation::despawn_entity`; this trace did not investigate owner-count or occupancy side effects.
- Normal dock exit still has a known Force_Track 0x47 visual TODO; this trace did not audit normal exit parity.
- Queue promotion after a dying miner occupant is covered by an existing unit test, but dying refinery queue behavior is not directly tested.

## Verdict Tally

PASS: 1
FAIL: 4
UNCHECKED: 3
NOT-IMPLEMENTED: 0

## Status

PARTIAL - exact runtime equality was not computed with a live gamemd scenario, and no tests were run because the assignment allowed writing only this report file.
