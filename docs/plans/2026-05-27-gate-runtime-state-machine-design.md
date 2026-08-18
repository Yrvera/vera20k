# Gate Runtime State Machine Design

## Goal
Implement the verified `Gate=yes` building runtime so closed allied gates open through mission `0x18`, hold open while occupied, and close using native timing.

## Architecture Context
Rules data enters through `src/rules/object_type.rs`, map/production spawning initializes `GameEntity`, movement runtime builds a live building skip map in `src/sim/movement/movement_occupancy.rs`, and `Simulation::advance_tick` owns deterministic system ordering. Existing partial gate support already parses `Gate=`, creates `BuildingGateRuntime`, and skips stable-open gates during movement classification.

## Impact Analysis
Touched surfaces are rules parsing, entity runtime state, state hashing, movement obstruction handling, and one simulation tick hook. The main risks are timing drift, accidentally treating opening gates as passable, and using static path-grid behavior instead of the live object-list scan.

## Chosen Approach
Keep the model local and explicit: extend `BuildingGateRuntime` with the mission local state, helper phase, and integer timers; add a small `sim::gate_runtime` tick module; call it after movement so an open request produced by movement can advance on subsequent ticks. Movement contact with a friendly closed/closing gate starts or restarts mission `0x18` but still lets the current cell-entry classification block.

## Tiny-Detail Ledger
- `GateCloseDelay=` parses to `BuildingType+0xE28` and is converted as `trunc(value * 900.0)` [doc: GATE_RUNTIME_MINI_REINVESTIGATION_20260527.md].
- `DeployTime=` at `TechnoType+0x3C8` drives both opening and closing transition duration, also `trunc(value * 900.0)` [doc: GATE_RUNTIME_MINI_REINVESTIGATION_20260527.md; TRANSPORT_DOOR_TIMING_RADIO_0X11_DEPLOY_TRACKER_GHIDRA_REPORT.md].
- Stock `[GAGATE_A]` gives `GateCloseDelay=.2 -> 180` frames and `DeployTime=.044 -> 39` frames [ini: rulesmd.ini; doc: GATE_RUNTIME_MINI_REINVESTIGATION_20260527.md].
- Stable-open passability is mission `0x18` plus helper state `active=0, open_side=1`; opening and closing remain blocked [doc: GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md].
- Allied contact assigns/starts mission `0x18` but the same entry check returns blocked [doc: GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md].
- State 0 starts opening from stable closed, reverses active closing, or moves directly to hold when already stable open [doc: GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md].
- State 2 scans live cell object chains over the gate coordinate list and ignores only the gate itself; any other object reseeds the hold timer [doc: GATE_RUNTIME_MINI_REINVESTIGATION_20260527.md].
- State 3 starts closing; state 4 waits until stable closed; state 5 is post-close idle [doc: GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md].

## Design

### Components
- `ObjectType`: add integer `deploy_time_ticks` and `gate_close_delay_ticks`.
- `BuildingGateRuntime`: add mission local state and transition/hold timers.
- `sim::gate_runtime`: pure sim tick and request helpers.
- `movement_occupancy`: request friendly gate opening during deferred blocker handling.

### Interfaces / Contracts
`request_gate_open_for_cell` mutates only friendly `Gate=yes` structure blockers in the contacted cell and returns no passability override. `tick_gate_runtimes` advances all gate buildings once per simulation tick using rules and live occupancy.

### Data Flow
Rules parse timings -> spawn initializes gate state -> movement contact starts mission/opening -> per-tick gate runtime transitions to stable open -> existing live skip map permits passability -> hold scan keeps gate open while occupied -> timer expires -> closing -> stable closed.

### Error Handling
Missing rules or type data leaves the existing gate state unchanged. Missing timing keys default to zero, matching native default fields.

### Testing Strategy
Focused unit tests cover timing conversion, same-check block on open request, stable-open passability only, hold reseed on live occupant, and close after clear hold expiration.

## Alternatives Considered
- Static skip-only gate flag: rejected, misses opening/hold/close timing.
- Fold gate runtime into movement only: rejected, auto-close and hold scan are building mission behavior, not pathfinding state.
