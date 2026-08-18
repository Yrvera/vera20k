# Drive No-Active-Track Start Same-Tick Trace - 2026-05-27

## Scenario

Concrete trace only: stock `[MTNK]` Grizzly Battle Tank starts idle at cell `(40,40)`, facing east, on flat clear land, then receives a right-click move order to `(45,40)`.

Question: does active `gamemd.exe` use the no-active-track path `Process_Movement -> Process_Drive_Track(0)` in the same `DriveLocomotionClass::Process` tick, and does current Rust start the first DriveTrack/vector step in the equivalent same tick after the recent Drive speed/residual changes?

## Evidence Summary

- Stock YR `[MTNK]` is a normal DriveLocomotion vehicle: `Speed=7`, `ROT=5`, `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}`, `MovementZone=Normal`, `Accelerates=false` in `ini/rulesmd.ini:6603`, `:6618`, `:6624`, `:6636`, `:6638`, `:6643`.
- Active YR confirmation: read-only Ghidra decompile of `DriveLocomotionClass::Process @ 0x004B0500` shows standard Drive slope sampling and the no-active-track branch. This is the normal DriveLocomotion process path selected by the stock Drive locomotor CLSID, not a TS-only gated path.
- Existing research agrees: `docs/research/DRIVE_PROCESS_MOVEMENT_TICK_ORDER_GHIDRA_REPORT.md:53-57` states the no-active-track path calls `Process_Movement(...,1,0)` then `Process_Drive_Track(0)` at `0x004B0AAA`.
- Current Rust creates the initial `DriveTrackState` during move-command dispatch in `src/sim/movement/movement_commands.rs:585-599`, before the movement tick.
- Current Rust movement tick advances an already-present drive track through `advance_lepton_position` at `src/sim/movement/movement_tick.rs:1083-1095`; the active-track branch in `advance_lepton_position` begins at `src/sim/movement/movement_step.rs:409`.

## Pipeline

Right-click move order -> destination/path ownership -> first DriveLocomotion process tick -> path/track selection -> first track budget consumption -> visible body/subcell motion.

## Stage Verdicts

| Stage | gamemd for this scenario | Current Rust for this scenario | Verdict |
|---|---|---|---|
| 1. Stock unit data | `[MTNK]` uses active DriveLocomotion, Speed 7, ROT 5, Normal movement zone, `Accelerates=false`. | Rust rules source has the same data source; exact parsed runtime value was not re-executed in this trace. | PASS for source data; runtime parse UNCHECKED |
| 2. Active-YR entry path | `DriveLocomotionClass::Process @ 0x004B0500` is the live Drive tick. With no active track (`track_index == -1` or head-to valid byte clear), it enters the no-active branch. | Rust does not have a Drive-owned `Process` dispatcher with the same branch split; it runs the generic movement loop. | FAIL |
| 3. No-active same-tick order | In the no-active branch, after arrival/delay/NavCom gates, gamemd calls `Process_Movement(...,1,0)` and then, if not stopped/dead, calls `Process_Drive_Track(0)` in the same process call. | Rust starts the first drive track in `issue_move_command_with_layered` before the movement tick, not from a no-active `Process_Movement` branch inside the Drive tick. | FAIL |
| 4. First track selection state | `Process_Movement @ 0x004B2630` computes target speed fraction, writes Drive `+0x50`, sets track index/point index, then returns to `Process`, which calls `Process_Drive_Track(0)`. | `movement_commands.rs:574-583` initializes Drive speed fraction to `1.0` during command dispatch, and `:591-599` begins the drive track immediately. `movement_tick.rs:1008-1020` recomputes Drive current fraction during the later tick. | FAIL |
| 5. First movement tick track budget | `Process_Drive_Track(0)` updates current speed fraction, calls `GetCurrentSpeed`, computes `(speed + residual)`, consumes strict 7-unit track points, and stores residual. | `movement_step.rs:412-418` computes `fresh_budget = effective_speed * dt` and adds `drive.residual_budget`. The recent residual authority is closer, but literal equality to gamemd `GetCurrentSpeed` integer for MTNK Speed 7 was not computed here. | UNCHECKED |
| 6. Same-tick visible motion | gamemd can start the selected DriveTrack in the same Drive process tick after `Process_Movement`, without waiting for a second Drive tick. | Rust can also have a track active on the first movement tick, but because track creation happened during command dispatch, exact tick/order equivalence is unproven and mechanism differs. | UNCHECKED |

## Findings

### FAIL 1 - First DriveTrack owner is command dispatch, not Drive Process

Current Rust starts the first DriveTrack as part of issuing the move order:

- `src/sim/movement/movement_commands.rs:585-599` selects and begins a drive track immediately.
- Existing test `src/sim/movement/movement_tests.rs:420-455` asserts this behavior.

In gamemd, the order-command side sets destination/path state, then the active no-track Drive tick runs `Process_Movement` and immediately follows with `Process_Drive_Track(0)`. This is not just internal shape: arrival/delay/NavCom gates, speed target write, track index reset, and first budget consumption occur in one Drive-owned sequence.

Player-visible risk: first-frame start cadence can diverge when the command is issued relative to sim tick order, when movement delay/NavCom gates are active, or when the first path leg is immediately blocked/redirected.

### FAIL 2 - Rust first tick enters active-track advancement, not the no-active branch

Because command dispatch has already populated `entity.drive_track`, the later movement tick reaches:

- `src/sim/movement/movement_tick.rs:1083-1095` -> `advance_lepton_position`
- `src/sim/movement/movement_step.rs:409-424` active drive-track advancement

That is equivalent to "track already active" from the movement tick's point of view. Gamemd's concrete scenario starts the Drive process with no active track and uses `Process_Movement -> Process_Drive_Track(0)` in that same tick.

Player-visible risk: first straight east movement may appear close in a simple open-field sample, but any equality claim is unproven; the mechanism can shift first-frame subcell position, residual, facing/body cadence, and blocker response.

### UNCHECKED 1 - Literal first-tick position/residual equality

This trace did not compute both exact first-tick numeric outputs. The gamemd side needs the exact `FootClass::GetCurrentSpeed` integer for stock MTNK Speed 7 under the scenario's house/veterancy/runtime modifiers. The Rust side needs the exact `effective_speed * dt` budget used by the current movement tick. Without both numbers, this stage is UNCHECKED.

## Adjacent Findings

- Full `FootClass::GetCurrentSpeed` rounding and modifier chain remains a separate exact-speed trace/contract target.
- The active-track same-tick retry path recently added in Rust is adjacent, but this trace only covers the no-active-track start.
- Drive slope sampling at the top of `Process` occurs before movement in gamemd, but this flat-clear scenario does not trace slope transitions.

## Verdict Tally

PASS: 1 | FAIL: 3 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Sources

- Read-only Ghidra decompile: `DriveLocomotionClass::Process @ 0x004B0500`
- Read-only Ghidra decompile: `DriveLocomotionClass::Process_Movement @ 0x004B2630`
- `docs/research/DRIVE_PROCESS_MOVEMENT_TICK_ORDER_GHIDRA_REPORT.md`
- `docs/research/DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `src/sim/movement/movement_commands.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/movement/movement_step.rs`
