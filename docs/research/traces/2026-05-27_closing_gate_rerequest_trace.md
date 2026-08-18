# Closing Gate Friendly Re-request Trace

**Date:** 2026-05-27  
**Trace slot:** 5  
**Scenario:** A friendly `Gate=yes` building is already closing when a friendly unit contacts/attempts the gate cell.  
**Scope limit:** Only the closing-gate friendly re-request branch, transition reversal/restart, and stable-open passability timing. Adjacent gate hold, enemy gate, infantry-only result-code mapping, rendering, and audio are not traced here.

## Sources Checked

- Read-only Ghidra spot-checks:
  - `MapClass__Check_Crushable_Obstacle @ 0x00578AD0`
  - allied opener/check `FUN_00452540 @ 0x00452540`
  - building mission `0x18` routine `FUN_0044E440 @ 0x0044E440`
  - transition reverse helper `FUN_004A5290 @ 0x004A5290`
  - transition start/open helpers `0x004A51F0`, `0x004A5240`
  - transition finalizer `0x004A5360`
  - `MissionClass__Assign_Mission @ 0x005B2FD0`
- Research docs:
  - `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md`
  - `docs/research/INFANTRY_GATE_CANGARRISON_RESULT_CONTRACT_GHIDRA_REPORT.md`
- Rust surfaces:
  - `src/sim/gate_runtime.rs`
  - `src/sim/movement/movement_occupancy.rs`
  - `src/sim/game_entity.rs`
  - `src/rules/object_type.rs`
  - `src/sim/world/mod.rs`

## Active-YR Confirmation

This path is active in standard Yuri's Revenge, conditional on a live `Gate=yes` building such as stock `[GAGATE_A]`.

- `MapClass__Check_Crushable_Obstacle @ 0x00578AD0` scans the live cell object list at `CellClass+0xE4`.
- When the object is a building (`WhatAmI()==6`) and its `BuildingType+0x16B7` `Gate=yes` byte is set, friendly ownership calls `FUN_00452540`.
- `FUN_00452540` assigns mission `0x18` when the gate is closing or closed-stable.
- Mission `0x18` dispatches to the building gate routine at `0x0044E440`.
- No TS-only feature gate was found in this branch. The path is normal building/pathing code, with player visibility depending on map-placed gate objects.

## Concrete Numeric Fixture

To make the reversal check concrete:

- Stock gate timing input: `DeployTime=.044`.
- Native transition duration consumed by helper writers: `trunc(.044 * 900) = 39` frames.
- Gate is already closing:
  - native helper active byte `+0x18 = 1`
  - native helper open-side byte `+0x19 = 0`
  - helper start frame `+0x08 = 100`
  - helper remaining/current field `+0x10 = 39`
  - helper total field `+0x14 = 39`
- Friendly unit contacts the gate at frame `110`, i.e. `10` frames into closing.

## Pipeline Trace

### Stage 1 - Live trigger finds friendly closing gate

**gamemd:** `0x00578AD0` walks `Cell+0xE4`, sees `Gate=yes`, checks ally, calls `0x00452540`.  
**Rust:** `handle_deferred_occupancy` calls `request_gate_open_for_cell` before classifying the occupied target cell (`src/sim/movement/movement_occupancy.rs:494`).  
**Concrete output:** both paths issue an opener/request against the friendly gate occupant in the checked cell.  
**Verdict:** PASS.

### Stage 2 - Same contact must remain blocked

**gamemd:** In `0x00452540`, closing state satisfies the reassign branch, calls clear/assign/commence, then returns `0` for this same obstacle check.  
**Rust:** `request_gate_open_for_cell` mutates the gate request state but does not add a skip-map entry; `build_live_building_entry_skip_map` only skips gates when `can_garrison_passable()` is true (`src/sim/movement/movement_occupancy.rs:320`).  
**Concrete output:** same contact is not passable in both engines.  
**Verdict:** PASS.

### Stage 3 - Mission reset/restart

**gamemd:** `0x00452540` calls mission assign with `0x18`; `MissionClass__Assign_Mission @ 0x005B2FD0` writes current mission `0x18` and resets local mission state `+0xBC = 0`.  
**Rust:** `request_open` sets `mission_18_active = true` and `mission_state = Setup` (`src/sim/gate_runtime.rs:119`).  
**Concrete output:** both reset the gate mission-local state to setup before the next gate mission step.  
**Verdict:** PASS.

### Stage 4 - Reversal helper field writes

**gamemd:** In state `0`, `0x0044E440` sees helper state `Closing` and calls `0x004A5290`. For the fixture:

- live remaining before reverse = `39 - (110 - 100) = 29`
- rewritten helper `+0x10 = +0x14 - live_remaining = 39 - 29 = 10`
- helper `+0x19` toggles from `0` to `1`
- helper start frame `+0x08` is not written by `0x004A5290`, so it remains `100`

**Rust:** `tick_gate` first calls `advance_transition`, then `reverse_transition`; for the same fixture it reaches `transition_ticks_remaining = 10`, toggles to `Opening`, but writes `transition_last_frame = 110` (`src/sim/gate_runtime.rs:56`, `src/sim/gate_runtime.rs:220`).  
**Concrete mismatch:** native start frame remains `100`; Rust transition baseline becomes `110`.  
**Verdict:** FAIL.

### Stage 5 - Reversed remaining amount

**gamemd:** fixture after reverse stores helper `+0x10 = 10`.  
**Rust:** fixture after reverse stores `transition_ticks_remaining = 10`.  
**Concrete output:** the remaining amount field matches for this sampled fixture.  
**Verdict:** PASS.

### Stage 6 - Stable-open timing after re-request

**gamemd:** Because `0x004A5290` leaves helper start frame at `100`, the progress helper on the next checked frame computes progress from the original start. At frame `111`, with rewritten `+0x10 = 10`, elapsed is `111 - 100 = 11`; the live remaining clamps to `0`, so the finalizer can stabilize open on that check.  
**Rust:** Because `transition_last_frame` was reset to `110`, frame `111` decrements `transition_ticks_remaining` from `10` to `9`; stable-open is not reached until frame `120`.  
**Player-visible difference:** after contacting a closing gate 10 frames into a 39-frame close, gamemd can make it passable on the next transition-finalizer check; Rust keeps it blocked for 9 extra frames in this fixture.  
**Verdict:** FAIL.

### Stage 7 - Hold timer after reversal

**gamemd:** state `0` seeds mission-local hold timer fields after starting/reversing toward open. Exact parser provenance for the duration fields was not rechecked in this trace.  
**Rust:** `tick_gate` seeds `hold_ticks_remaining` from `gate_close_delay_ticks` in setup (`src/sim/gate_runtime.rs:248`).  
**Concrete output:** not enough exact native field provenance was recomputed for a literal equality claim.  
**Verdict:** UNCHECKED.

### Stage 8 - Render/audio/dirty side effects

**gamemd:** mission state `0`/`3` dirty screen and play configured sounds around open/close starts; state `4` calls vtable `+0x484(0,1)` after stable-close.  
**Rust:** this trace did not inspect equivalent render/audio side effects; `gate_runtime.rs` is sim/passability-only.  
**Concrete output:** not computed for both engines in this run.  
**Verdict:** UNCHECKED.

## Findings

1. **FAIL - reversal timer baseline drift.** Native `0x004A5290` does not rewrite helper start frame `+0x08`; Rust resets `transition_last_frame` to the re-request frame in `reverse_transition`.
2. **FAIL - stable-open timing after re-request.** In the concrete `39`-frame close contacted at frame `110` after starting at `100`, gamemd's helper can be stable-open on the next finalizer check, while Rust remains opening until frame `120`.

## Adjacent Findings

- `GateCloseDelay` parser provenance to native `BuildingType+0x3C8/+0x3CC` remains a separate research uncertainty from the gate writer report. This trace only used the concrete duration already consumed by the helper.
- Render/audio side effects are adjacent to this passability trace and should be handled separately if pixel/audio parity is the target.

## Verdict Tally

PASS: 4  
FAIL: 2  
UNCHECKED: 2  
NOT-IMPLEMENTED: 0

## Status

COMPLETE
