# Chrono Miner Close Return Scheduler Frame Trace

Date: 2026-05-22

Scenario: stock YR `CMIN` with full cargo, close enough to a stock `GAREFN` or
`NAREFN`, tracing only the scheduler boundary that was left deferred by
`CHRONO_MINER_FULL_CARGO_CLOSE_RETURN_MISSION_DISPATCH_TIMING_TRACE.md`.

Scope: answer when queued mission `7` (`Mission_Enter`) and queued mission
`0x10` (`Mission_Deploy_Building` / DockUnload) first become dispatchable after
the radio handoff. No Rust files were edited.

## Sources Checked

- Ghidra read-only decompile:
  - `UnitClass::AI @ 0x007360C0`
  - `FootClass::AI @ 0x004DA530`
  - `TechnoClass::AI_Update @ 0x006F9E50`
  - `MissionClass::Mission_Dispatch @ 0x005B3060`
  - `MissionClass::Queue_Mission @ 0x005B35E0`
  - `MissionClass::Commence @ 0x005B3570`
  - `MissionClass::Assign_Mission @ 0x005B2FD0`
  - `MissionClass::GetMissionTimerEntry @ 0x005B3A00`
  - `UnitClass::Mission_Harvest @ 0x0073E5E0`
  - `FootClass::Mission_Enter @ 0x004D9290`
- Existing docs:
  - `miner/traces/CHRONO_MINER_FULL_CARGO_CLOSE_RETURN_MISSION_DISPATCH_TIMING_TRACE.md`
  - `UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`
  - `UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_00739EC0_NAVCOM_GHIDRA_REPORT.md`
  - `miner/BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`
  - `miner/TWO_CMIN_ONE_REFINERY_TAKEOVER_TIMING_GHIDRA_REPORT.md`
- Rust comparison surface:
  - `src/sim/miner/miner_system.rs`
  - `src/sim/miner/miner_dock_sequence.rs`

## Scheduler Order

One live unit tick runs through `UnitClass::AI @ 0x007360C0`.

Relevant order inside that function:

1. Early `ReadyToCommence` / `Commence` gate:
   `if (vtable+0x200()) vtable+0x1EC()`.
2. `FootClass::AI @ 0x004DA530`.
3. Inside `FootClass::AI`, the first call is `TechnoClass::AI_Update`.
4. Inside `TechnoClass::AI_Update`, `MissionClass::Mission_Dispatch` runs once.
5. `FootClass::AI` then continues into locomotor processing and per-cell effects.
6. Back in `UnitClass::AI`, a late `ReadyToCommence` / `Commence` gate runs again:
   `if (vtable+0x200()) vtable+0x1EC()`.

`MissionClass::Queue_Mission @ 0x005B35E0` with flag `0` writes the queued mission
field (`+0xB4`) and clears the commenced byte, but does not call `Commence`.
`MissionClass::Commence @ 0x005B3570` copies queued mission `+0xB4` into current
mission `+0xAC`, clears `+0xB4`, clears the mission timer duration, and resets the
timer frame fields. The default `ReadyToCommence @ 0x004E0140` returns `1`, so
UnitClass normally takes both commence gates when a queued mission exists.

## Resolved Frame Questions

Let frame `F` be a unit's `UnitClass::AI` call where `Mission_Dispatch` runs
`Mission_Harvest` substate 3 and queues mission `7`.

1. During frame `F`, `Mission_Dispatch` is already inside the current
   `Mission_Harvest` handler. State 3 calls `Queue_Mission(7, 0)`.
2. The mission handler returns to `Mission_Dispatch`; there is no second
   mission-dispatch loop in the same `TechnoClass::AI_Update`.
3. `FootClass::AI` finishes, then the late `UnitClass::AI` commence gate promotes
   queued mission `7` to current mission `+0xAC` in the same frame `F`.
4. First `FootClass::Mission_Enter` dispatch is therefore no earlier than the
   next live `Mission_Dispatch` for that unit, normally frame `F+1` if the unit is
   still active in the next global logic frame.

Let frame `A` be a unit's `Mission_Enter` dispatch where the unit is already at
the accepted refinery cell and the building sends `0x18`/`0x16`.

1. `0x18` and `0x16` are synchronous inside the `BuildingClass::Receive_Radio(0x0E)`
   call that happens during `Mission_Dispatch`.
2. `Mission_Dispatch` then returns; it does not immediately dispatch the mission
   that may later be queued by pad arrival.
3. Later in the same `FootClass::AI` call, locomotor/per-cell processing can run
   `UnitClass::PerCellProcess`, send radio `0x15`, and cause the building to call
   `Queue_Mission(0x10, 0)` on the sender.
4. After `FootClass::AI` returns, the late `UnitClass::AI` commence gate promotes
   queued mission `0x10` to current mission in the same frame `A`.
5. First `UnitClass::Mission_Deploy_Building` dispatch is therefore no earlier
   than the next live `Mission_Dispatch` for that unit, normally frame `A+1`.

## Stage Verdicts

| Stage | gamemd output | Current Rust surface | Verdict |
|---|---|---|---|
| State-3 queue mission `7` | Queued during `Mission_Harvest`; promoted to current later in the same `UnitClass::AI`; first Mission Enter handler dispatch next unit tick/frame. | `phase_approach` immediately sets `MissionEnter` phase; the next Rust miner tick runs `phase_mission_enter`. | PASS-ish for one-frame phase separation after HELLO if early HELLO is moved into return state; current Rust still sends HELLO too late. |
| `Mission_Enter` already-there `0x18/0x16` | Synchronous with the `0x0E` dispatch when `0x12` returns already-there. | `phase_mission_enter` marks entered and `phase_linked` starts pivot on a later Rust tick. | FAIL for same-dispatch ordering. |
| `0x15` queues mission `0x10` | Can occur later in the same unit AI frame after Mission Enter dispatch, during locomotor/per-cell processing. | Rust has no explicit `0x15`; `phase_linked` directly marks on-pad and starts pivot. | FAIL for event identity; broad visual result may still converge. |
| First unload mission dispatch | Queued `0x10` is promoted same AI frame as `0x15`, but first `Mission_Deploy_Building` dispatch is next unit tick/frame. | Rust starts pivot/unload phase after `Linked`/`Pivoting`, not through a queued mission boundary. | UNCHECKED/FAIL for exact frame parity. |

## Implementation Implications

- Do not model `Queue_Mission(7, 0)` or `Queue_Mission(0x10, 0)` as same-dispatch
  handler calls. Promotion to current mission can happen later in the same
  `UnitClass::AI` call, but the newly current mission's handler waits for the
  next `Mission_Dispatch`.
- Moving Rust's close-return HELLO earlier is still the first required fix.
  After a successful early HELLO, the Rust phase model should preserve a one-tick
  boundary before `MissionEnter` logic rather than collapsing HELLO and `0x0E`
  in the same miner tick.
- A later exact-frame parity pass should split "already at accepted cell" from
  "pad arrival / `0x15`" if the current `Linked` phase visibly starts pivot one
  frame too late.

## Remaining Uncertainty

- The exact runtime mission timer table value and `RandomRanged(0,2)` result for
  the state-2/state-3 harvest epilogue were not read from a running process.
  This trace resolves the queue/commence boundary once a state-3 dispatch has
  happened; it does not prove the number of frames from state-2 HELLO success to
  state-3 dispatch.
- First rendered locomotor pixel delta after the accepted-cell `Set_Destination`
  still needs runtime coordinate logging.

