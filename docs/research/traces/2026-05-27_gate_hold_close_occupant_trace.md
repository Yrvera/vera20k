# Gate Hold Close Occupant Trace - 2026-05-27

## Scope

Concrete scenario only: a friendly `Gate=yes` building is already in mission `0x18` stable-open state. A live unit occupies one cell of the gate foundation footprint. The gate should hold/reseed its close timer while occupied, then after the footprint clears it should close using `GateCloseDelay` for hold timing and `DeployTime` for the closing transition.

Non-scope: enemy gates, allied closed-gate open request, infantry result-code mapping, bunker/refinery pads, full gate art-frame composition beyond noting whether the close is rendered/audible.

## Evidence Sources

- `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md`
- `docs/research/GATE_RUNTIME_MINI_REINVESTIGATION_20260527.md`
- `docs/research/INFANTRY_GATE_CANGARRISON_RESULT_CONTRACT_GHIDRA_REPORT.md`
- Read-only Ghidra spot checks this run: `0x0044E440`, `0x0044E3A0`, `0x004A51F0`, `0x004A5240`, `0x004A5360`, `0x0045FE50`.
- INI: `ini/rulesmd.ini:[GAGATE_A]` and `[Unload]`.
- Rust: `src/rules/object_type.rs`, `src/sim/game_entity.rs`, `src/sim/gate_runtime.rs`, `src/sim/world/mod.rs`.

All gamemd references used here are active in standard YR when a live `BuildingTypeClass Gate=yes` building is present. The mission path is the normal `MissionClass` dispatch case `0x18` through the BuildingClass vtable slot, not a dormant TS-only path.

## Concrete Values

Stock `rulesmd.ini:[GAGATE_A]` has:

- `Gate=yes`
- `DeployTime=.044`
- `GateCloseDelay=.2`

Native conversion:

- `DeployTime`: `trunc(0.044 * 900.0) = 39` frames.
- `GateCloseDelay`: `trunc(0.2 * 900.0) = 180` frames.

Rust conversion:

- `ObjectType::from_ini_section` stores `deploy_time_ticks = 39`.
- `ObjectType::from_ini_section` stores `gate_close_delay_ticks = 180`.

`rulesmd.ini:[Unload]` has `Rate=.016`. Native mission `0x18` is the Unload mission entry, so the mission-handler epilogue returns `trunc(0.016 * 900.0) + RandomRanged(0,2) = 14..16` frames between handler dispatches unless a separate immediate path resets the timer.

## Pipeline

`Gate=yes` data -> stable-open mission `0x18` gate runtime -> state-2 footprint live object scan -> hold timer reseed while occupied -> no obstruction -> hold progress reaches 1.0 -> state 3 starts closing -> helper closing transition uses `DeployTime` -> stable closed -> render/audio updates.

## Stage Verdicts

| Stage | gamemd output | Rust output | Verdict |
|---|---|---|---|
| Stock rule values | `Gate=yes`, `DeployTime=.044`, `GateCloseDelay=.2` on `[GAGATE_A]`; `[Unload] Rate=.016` | Same INI values are read; gate timing parses to 39/180 | PASS |
| Gate passable predicate before close | Mission `0x18` plus helper stable-open bytes means passable/open | `mission_18_active && phase == OpenStable` | PASS |
| Occupant scan boolean for one live unit in a foundation cell | `FUN_0044E3A0` scans the building coordinate list, reads each cell object chain, ignores only the gate object, and returns obstruction present | `footprint_has_other_live_object` scans foundation cells in `OccupancyGrid` and returns true for any occupant whose id is not the gate | PASS for this concrete live-unit occupied-cell boolean |
| Hold reseed on an occupied mission dispatch | State 2 sees obstruction and reseeds the hold timer from `GateCloseDelay`: remaining/total = 180 at current frame | `tick_gate` in `OpenHold` with `obstructed=true` sets `hold_ticks_remaining = 180` and `hold_last_frame = binary_frame` | PASS for a dispatched/ticked occupied sample |
| Clear-to-close dispatch cadence | Native state 2 is only evaluated when mission `0x18` dispatch is due. Stock `[Unload] Rate=.016` gives 14..16 frame delay and consumes `RandomRanged(0,2)` in the epilogue. Close-start can occur on the first due mission dispatch at or after the timer reaches complete, not from a per-frame poll. | `World::tick` calls `tick_gate_runtimes` every frame/tick after ground movement; no mission timer gate and no `RandomRanged(0,2)` consumption. | FAIL |
| Closing transition duration after close starts | `StartClosing @ 0x004A5240` uses `DeployTime` and sets active/closing for 39 frames | `start_closing` uses `deploy_time_ticks = 39` and sets `phase = Closing` | PASS for relative transition duration after close-start |
| Stable-closed finalization tick | Native helper finalizer `0x004A5360` writes stable closed after the active/closing timer is complete; exact AI-update vs mission-order tick was not recomputed in this trace | `advance_transition` finalizes at the start of Rust gate tick when remaining reaches zero | UNCHECKED |
| Visible/audio close result | Native mission code plays configured gate sound and updates render helper/frame state while opening/closing; draw code consumes helper state | Rust gate runtime changes sim state only. No render/audio path reads `building_gate`, and no gate open/close sound is emitted from `gate_runtime.rs` | NOT-IMPLEMENTED |

## Findings

### FAIL - Mission polling cadence and RNG are not native

Native gate mission `0x18` is the BuildingClass Unload mission. The handler reaches the standard mission timer epilogue and returns the `[Unload] Rate` converted to frames plus `RandomRanged(0,2)`. With stock `Rate=.016`, that is `14..16` frames. Therefore the state-2 obstruction scan and clear-to-close decision are mission-dispatch gated, and they consume RNG.

Rust wires `tick_gate_runtimes` directly into `World::tick`, so the gate hold state is examined every frame/tick with no mission timer and no RNG consumption. Even though the hold duration value is correct at 180 frames, the close-start decision can be early/late relative to gamemd depending on where the clear event falls between native mission dispatches, and the global RNG stream differs.

Affected Rust surface:

- `src/sim/world/mod.rs:1267`
- `src/sim/gate_runtime.rs:251`

gamemd evidence:

- `BuildingClass mission 0x18 @ 0x0044E440` state 2 performs the obstruction check and then falls through the standard mission timer return path.
- `ini/rulesmd.ini:30553..30557` gives `[Unload] Rate=.016`.
- Existing mission timer docs verify `MissionClass::GetMissionTimerEntry` rate conversion and inclusive `RandomRanged(0,2)` behavior.

### NOT-IMPLEMENTED - Gate close is not rendered/audible

Native gate close is not just passability state. The mission code starts the close helper, plays a configured gate sound, updates a helper-derived frame byte, and BuildingClass draw uses the helper state for the visible gate body.

Current Rust has sim-side gate phase but no render/audio consumer. `rg` finds `building_gate` only in sim/world hash/spawn/movement code, not in render/app-render code. A player can get the passability change without the native visible close animation or sound.

Affected Rust surface:

- `src/sim/gate_runtime.rs:262`
- render/audio integration missing

gamemd evidence:

- `0x0044E440` state 3 calls `StartClosing @ 0x004A5240` and plays sound.
- Gate writer report documents draw-side helper consumption as a separate integration point.

## Adjacent Findings

- Rust's footprint scan uses `OccupancyGrid` and all occupants in a cell; native reads the `CellClass+0xE4` object chain. For the concrete live ground unit occupying a footprint cell, both return obstruction present. Layer/list exactness for bridge/air/stale entries was not traced in this slot.
- Exact post-close vtable `+0x484(0,1)` semantics remain out-of-scope.

## Verdict Tally

PASS: 5 | FAIL: 1 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Status

COMPLETE
