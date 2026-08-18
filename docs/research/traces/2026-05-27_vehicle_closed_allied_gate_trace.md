# Vehicle Closed Allied Gate Trace

Date: 2026-05-27
Slot: trace-swarm slot 1
Mechanic: Vehicle closed allied gate passability and open request
Scenario: Allied tank attempts to enter a closed friendly `Gate=yes` building cell. The first runtime check should request mission `0x18` but remain blocked; after stable-open the live building skip should permit entry.

## Scope

This trace covers one concrete movement/passability scenario only. It does not trace enemy gates, infantry-specific gate behavior, gate rendering, gate sounds, or closing/hold behavior except where those affect the first contact and stable-open passability result.

Ghidra MCP was used read-only only. A live `batch_decompile` spot-check for `0x00452540`, `0x004525F0`, `0x00578AD0`, and `0x0044E440` returned `Function not found`, so active-gamemd evidence below is from already verified Ghidra research reports, not fresh decompilation in this run.

## Evidence Sources

- `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md:17-21`: gate helper states and stable-open passability.
- `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md:77-102`: allied live entry assigns mission `0x18` and returns blocked on the same call.
- `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md:104-120`: stock `Gate=`, `DeployTime=`, `GateCloseDelay=` data and active YR integration points.
- `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md:145-159`: resolved active-YR questions including same-call false return.
- `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md:167-173`: implementation handoff for gate passability and mission state.
- `docs/research/GARRISON_SYSTEM_GHIDRA_REPORT.md:364-378`: `BuildingClass::CanGarrison` is the gate passability predicate.
- `docs/research/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md:417-430`: UnitClass building branch maps friendly blocked gate/building to result code `3`.
- `ini/rulesmd.ini:17186-17206`: `[GAGATE_A] Gate=yes`, `DeployTime=.044`, `GateCloseDelay=.2`.

## Pipeline

Tank movement attempts target gate cell -> runtime deferred occupancy check -> friendly gate open request -> same occupancy classification remains blocked -> gate mission/runtime ticks -> helper reaches stable-open -> live building skip ignores gate occupant -> vehicle can enter if no later blocker exists.

## Stage Verdicts

1. Stock gate flag and scenario data
   - Rust: `GAGATE_A` parses `Gate=yes` from `src/rules/object_type.rs:968`.
   - Rust value: `gate = true`.
   - gamemd: `BuildingTypeClass+0x16B7 Gate=yes` is consumed by `0x00452540`, `0x004525F0`, and `0x00578AD0`; stock `[GAGATE_A] Gate=yes` is documented active in YR.
   - Verdict: PASS.

2. Stock timing conversion
   - Rust: `DeployTime=.044` -> `trunc(.044 * 900) = 39`; `GateCloseDelay=.2` -> `trunc(.2 * 900) = 180` in `src/rules/object_type.rs:797-802` and `src/rules/object_type.rs:969-974`.
   - gamemd: writer duration fields are passed to gate helper writers, but the exact parser-to-field source for `DeployTime` / `GateCloseDelay` was deferred in the verified report.
   - Verdict: UNCHECKED.

3. Closed friendly gate runtime initial state
   - Rust: structure spawns for `Gate=yes` install `BuildingGateRuntime::default()` at `src/sim/world/world_spawn.rs:173-175`, `src/sim/world/world_spawn.rs:376-377`, and `src/sim/world/world_spawn.rs:538-539`; default is closed/stable in `src/sim/game_entity.rs:102-110`.
   - gamemd: the helper has verified `ClosedStable` bytes `+0x18=0,+0x19=0`, but this run did not compute map-spawn initial bytes from a live runtime fixture.
   - Verdict: UNCHECKED.

4. First friendly gate contact requests open mission
   - Rust: `handle_deferred_occupancy` calls `request_gate_open_for_cell` before classification at `src/sim/movement/movement_occupancy.rs:493-505`; `request_open` sets `mission_18_active=true` and `mission_state=Setup` at `src/sim/gate_runtime.rs:119-130`.
   - gamemd: allied `MapClass__Check_Crushable_Obstacle` calls `0x00452540`; the opener clears/retargets via vtable `+0x1F0(-1)`, assigns mission `0x18`, calls vtable `+0x1EC()`, then returns false.
   - Difference: Rust models the mission assignment intent but does not model the native clear/retarget and commence/reset side effects.
   - Verdict: NOT-IMPLEMENTED.

5. Same runtime check remains blocked with native result code
   - Rust: after the request, the same classifier sees the structure as a normal friendly stationary blocker and returns `FriendlyStationary`, YR code `6`, via `src/sim/pathfinding/cell_entry.rs:625-635`.
   - gamemd: the allied opener path returns false on the same call; UnitClass building processing for friendly blocked gate/building raises result code `3` (`ScatterRequired`) when `CanGarrison` is false.
   - Difference: both block movement, but the result code and downstream response are not byte/mechanism-equivalent. Code `6` can dispatch generic friendly-stationary behavior instead of the native allied-building/scatter result.
   - Verdict: FAIL.

6. Opening transition reaches stable-open
   - Rust: state `Setup` starts `Opening`, counts down `deploy_time_ticks`, and finalizes to `OpenStable` in `src/sim/gate_runtime.rs:220-249`.
   - Rust value for stock `GAGATE_A`: transition lasts 39 binary frames if the parsed `DeployTime=.044` value is used.
   - gamemd: `StartOpening @ 0x004A51F0` writes active/opening; `FinishTransition @ 0x004A5360` finalizes active/opening to stable open. Exact tick ordering relative to the movement contact was not computed in this run.
   - Verdict: UNCHECKED.

7. Stable-open passability predicate
   - Rust: `can_garrison_passable()` is true only when `mission_18_active && phase == OpenStable` at `src/sim/game_entity.rs:117-120`.
   - gamemd: `BuildingClass::CanGarrison @ 0x004525F0` returns true for gates only when current mission is `0x18` and helper predicate `0x004A51B0` confirms stable-open (`+0x18=0,+0x19=1`).
   - Verdict: PASS.

8. Stable-open live building skip permits entry
   - Rust: `build_live_building_entry_skip_map` adds gate foundation cells for unit/infantry movers only when `obj.gate && building_gate.can_garrison_passable()` at `src/sim/movement/movement_occupancy.rs:303-324` and `src/sim/movement/movement_occupancy.rs:377-386`; the classifier ignores skipped blockers at `src/sim/pathfinding/cell_entry.rs:601-609` and returns clear if no other blocker remains at `src/sim/pathfinding/cell_entry.rs:571-578`.
   - gamemd: verified handoff requires closed/opening/closing to block and stable-open to permit branch continuation.
   - Scenario result: with only the stable-open gate occupant in the target cell, both outputs are passable/clear.
   - Verdict: PASS.

9. Tick/update ordering from contact to open gate
   - Rust: gate runtimes tick after ground movement in `src/sim/world/mod.rs:1245-1275`.
   - gamemd: mission dispatch and TechnoClass AI transition finalization are verified active, but this run did not compute their exact order relative to the movement contact in a live fixture.
   - Verdict: UNCHECKED.

## Findings

### F1 - First contact uses the wrong blocked result code

Stage: 5
Player-visible difference: an allied tank blocked by a closed friendly gate may follow Rust's generic friendly-stationary response instead of gamemd's allied-building/scatter result. This can affect waiting, scatter/repath behavior, and debug/result-code parity even though the vehicle does not pass through immediately.

Rust evidence: `src/sim/pathfinding/cell_entry.rs:625-635` maps any friendly stationary blocker to code `6`; `src/sim/movement/movement_occupancy.rs:493-505` requests gate open before that generic classification.

gamemd evidence: `GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md:77-102` proves allied contact assigns mission `0x18` but returns false on the same call; `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md:417-430` documents friendly building blocked result code `3`.

Verdict: FAIL.

### N1 - Native opener side effects are not modeled

Stage: 4
Player-visible difference: usually subtle, but parity-relevant: gamemd clears/retargets and commences/resets the gate mission through vtable calls when the allied opener runs. Rust currently only sets a compact mission flag/state and waits for the world gate runtime tick.

Rust evidence: `src/sim/gate_runtime.rs:119-130`.

gamemd evidence: `GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md:84-96` and resolved question `OQ-009` at `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md:155`.

Verdict: NOT-IMPLEMENTED.

## Adjacent Findings

- Gate render frame and sound parity were not traced in this slot.
- Enemy/neutral gate handling was not traced in this slot.
- Closing/re-request behavior was not traced in this slot.
- The exact parser origin for native `DeployTime` / `GateCloseDelay` offsets remains deferred by the verified gate report.

## Verdict Tally

PASS: 3
FAIL: 1
UNCHECKED: 4
NOT-IMPLEMENTED: 1

## Status

COMPLETE. Report is complete for the requested slot, with UNCHECKED stages where literal Rust-vs-gamemd equality was not computed.
