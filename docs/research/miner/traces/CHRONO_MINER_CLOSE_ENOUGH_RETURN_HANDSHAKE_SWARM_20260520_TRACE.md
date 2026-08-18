# Chrono Miner CloseEnough Return Handshake Swarm Trace

Scenario: loaded Allied Chrono Miner (`CMIN`) returning to a `GAREFN` dock target is at `(88,183)`, goal/dock cell `(88,181)`, and has stopped with no active movement target while inside `CloseEnough` but not docked.

Date: 2026-05-20

Scope guard: one mechanic only, the CloseEnough return handoff near the refinery. Ghidra use was read-only decompile only. No Rust, INI, or in-repo docs were modified.

> **Repo-status supersession 2026-05-25:** Any adjacent note below saying Rust
> still uses a hardcoded 2-cell chrono inbound threshold is stale. Current Rust
> reads `ChronoHarvTooFarDistance` for the close/far split. The exact
> CloseEnough scenario verdict can still be read, but do not use the old
> threshold note as current repo evidence.

## Verdict

PASS: 5 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

Player-visible answer: current Rust now matches the important next-tick behavior for this exact no-contention scenario. `gamemd.exe` does not unload and does not teleport. It re-enters the refinery enter/dock movement path and targets `(88,181)`. Current Rust also does not unload, does not teleport, leaves the CMIN in return state, and reissues movement to `(88,181)`.

## Concrete Values

- `rulesmd.ini:58`: `CloseEnough=2.25`, equivalent to `576` leptons.
- `rulesmd.ini:294`: `ChronoHarvTooFarDistance=50`, equivalent to `12800` leptons.
- `rulesmd.ini:7351` `[CMIN]`; active standard YR Allied Chrono Miner data includes `Harvester=yes`, `Dock=NAREFN,GAREFN`, `Teleporter=yes`, and the teleport locomotor.
- `rulesmd.ini:11722` `[GAREFN]`; active standard YR Allied refinery data includes `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`.
- `artmd.ini:1763` `[GAREFN]`; `Foundation=4x3`, `QueueingCell=4,1`.
- Scenario distance to dock cell: `(88,183)` -> `(88,181)` gives `dx=0`, `dy=2`.
- Rust CloseEnough helper value: `(0 + 2) * 256 = 512`; `512 < 576`.
- Rust chrono warp gate: `cell_dist_sq((88,183), (88,181)) = 4`; threshold square is `2 * 2 = 4`; Rust uses `>`, so `4 > 4` is false and no warp is issued.
- gamemd chrono return distance to refinery origin `(85,180)`: `dx=3*256=768`, `dy=3*256=768`, `floor(sqrt(768^2 + 768^2)) = 1086`; `1086 <= 12800`, so the within-distance dock branch is taken.
- gamemd accepted `CAN_DOCK` target for GAREFN anchor `(85,180)` is `(85 + 3, 180 + 1) = (88,181)`.

## Pipeline

Stopped within CloseEnough with no movement target -> return/harvest mission tick -> chrono near/far branch -> refinery enter/dock movement handshake -> dock cell movement target -> later pad arrival/unload path.

## Stage Results

### Stage 1 - Data Loading

Our data: current INI parsing exposes `CloseEnough=576`, `ChronoHarvTooFarDistance=50`, CMIN harvester/teleporter flags, and GAREFN dock/refinery flags.

gamemd evidence: `UnitClass__Mission_Harvest` reads the active CMIN harvester/teleporter type bytes and the `RulesClass` chrono harvester distance field; `DriveLocomotionClass__Process_Movement` reads active `CloseEnough`; `BuildingClass__Receive_Radio` case `0x0E` handles active refinery `CAN_DOCK`.

Verdict: PASS.

### Stage 2 - CloseEnough Stopped State

Our output for the given state is `movement_target=None` and position `(88,183)`, matching the scenario precondition. Rust's blocked movement helper uses `(dx + dy) * 256`; for `(88,183)` to `(88,181)`, that is `512 < 576`.

gamemd evidence: `DriveLocomotionClass__Process_Movement` has the active-YR non-`Mission_Enter` CloseEnough abort path. For this axis-aligned two-cell case the distance is `512` leptons, also below `576`, so a close stop is plausible and active, not TS legacy.

Verdict: PASS.

### Stage 3 - Next Return Tick: Teleport vs Re-Enter Movement

Our output: `try_issue_chrono_return_teleport` returns false because `cell_dist_sq=4` is not greater than `2^2=4`; no `teleport_state` is created. Since CMIN contact is gated to exact dock-cell arrival, Rust does not enter `MinerState::Dock`. It calls `issue_move_if_idle(..., target=(88,181))`.

gamemd output: `UnitClass__Mission_Harvest` state 2 sees a teleporter harvester within `ChronoHarvTooFarDistance` (`1086 <= 12800`), sends radio code `2` to the refinery, sets harvest substate `3` on acceptance, and state 3 sets mission `7` (`Mission_Enter`). `FootClass__Mission_Enter` then sends `CAN_DOCK(0x0E)`.

Compared player-visible boundary: no teleport, no unload, next movement target `(88,181)`.

Verdict: PASS.

### Stage 4 - Dock Target Cell

Our output: `refinery_dock_for_sid` uses `refinery_can_dock_queue_cell`, documented in code as the `BuildingClass::Receive_Radio` case `0x0E` target; for GAREFN `(85,180)` this is `(88,181)`.

gamemd output: `BuildingClass__Receive_Radio` case `0x0E` computes building anchor + `(3,1)`, also `(88,181)`, and returns that target through the dock handshake.

Verdict: PASS.

### Stage 5 - Unload Trigger

Our output on the next return tick is not unload: miner state remains `ReturnToRefinery`, `dock_phase` is not advanced to linked/unloading, and the regression test `chrono_return_close_enough_does_not_enter_dock` asserts no dock occupancy from CloseEnough alone.

gamemd output: state 2/3 enters `Mission_Enter`/`CAN_DOCK`; the miner is not at the dock coordinate yet (`(88,183) != (88,181)`), so the unload/radio dock-now path is not the next behavior.

Verdict: PASS.

### Stage 6 - Audio/Visual Teleport Effects

Our output: no `ChronoTeleport` sound/effect is emitted because the teleport branch returns false before `spawn_warp_effects` and `issue_teleport_command`.

gamemd output: the active within-distance branch never arms `TeleportLocomotionClass__HeadToCoord` for this case, so the next behavior has no chrono out/in audiovisuals.

Verdict: PASS.

### Stage 7 - Exact Tick Timing

Our tick ordering is clear from `tick_miners`: the reissued `movement_target` is produced on the miner tick that observes no movement target. gamemd decompile confirms branch order from harvest state 2 to state 3 to `Mission_Enter`, but this trace did not run an instrumented live session to count exact frame delays and mission timer randomization.

Verdict: UNCHECKED.

### Stage 8 - Reservation/Contention Semantics

gamemd sends refinery radio before the `CAN_DOCK` movement retarget. Rust reissues movement directly while still in `ReturnToRefinery` and does not enter dock reservation until exact dock-cell contact. In this exact single-miner/no-contention scenario the player-visible next movement target is still identical, but queue contention could expose a difference.

Verdict: UNCHECKED for this scenario; adjacent for contention.

## Findings

No player-visible FAIL or NOT-IMPLEMENTED finding was confirmed for this exact `(88,183)` -> `(88,181)` CloseEnough return handoff in current Rust.

## Active-YR Confirmation

- `UnitClass__Mission_Harvest`: active standard YR harvester mission path; CMIN reaches the teleporter harvester branch through retail `Harvester=yes` / `Teleporter=yes` data.
- `DriveLocomotionClass__Process_Movement`: active standard YR ground movement path; CloseEnough abort is gated out for `Mission_Enter` but active before the enter mission takes over.
- `FootClass__Mission_Enter`: active mission 7 handler used after harvest state 3.
- `BuildingClass__Receive_Radio` case `0x0E`: active YR refinery `CAN_DOCK` reply path; computes target `(anchor + 3, anchor + 1)`.

## Adjacent Findings

- The older `chrono_miner_close_enough_return_loop_TRACE.md` is stale for current Rust at this exact point: CMIN CloseEnough no longer promotes directly to dock, and the warp check now compares against the dock cell so `dist_sq=4` does not warp.
- Rust still uses a hardcoded 2-cell chrono inbound warp threshold rather than `ChronoHarvTooFarDistance=50`. That is adjacent here because the exact two-cell boundary avoids the warp, but it remains player-visible for other nearby return starts.
- Rust does not model the full gamemd radio/state sequence before moving from `(88,183)` to `(88,181)`. In a no-contention scenario the next target matches; refinery contention should be traced separately.

## References

- `src/sim/miner/miner_system.rs:39`
- `src/sim/miner/miner_system.rs:637`
- `src/sim/miner/miner_system.rs:641`
- `src/sim/miner/miner_system.rs:645`
- `src/sim/miner/miner_system.rs:646`
- `src/sim/miner/miner_system.rs:662`
- `src/sim/miner/miner_system.rs:852`
- `src/sim/miner/miner_system.rs:867`
- `src/sim/miner/miner_system.rs:868`
- `src/sim/miner/miner_system.rs:878`
- `src/sim/miner/miner_system.rs:885`
- `src/sim/miner/miner_system.rs:1009`
- `src/sim/miner/miner_system.rs:1269`
- `src/sim/miner/miner_system.rs:1275`
- `src/sim/miner/miner_dock_sequence.rs:84`
- `src/sim/miner/miner_dock_sequence.rs:88`
- `src/sim/movement/movement_blocked.rs:87`
- `src/sim/movement/movement_tick.rs:1093`
- `src/sim/miner/miner_tests.rs:537`
- `src/sim/miner/miner_tests.rs:560`
- `src/sim/miner/miner_tests.rs:564`
- `src/sim/miner/miner_tests.rs:570`
- `src/sim/miner/miner_tests.rs:572`
- `src/sim/miner/miner_tests.rs:576`
- `src/sim/miner/miner_tests.rs:580`
- `ini/rulesmd.ini:58`
- `ini/rulesmd.ini:294`
- `ini/rulesmd.ini:7351`
- `ini/rulesmd.ini:11722`
- `ini/artmd.ini:1763`
- Ghidra read-only decompile: `UnitClass__Mission_Harvest`
- Ghidra read-only decompile: `DriveLocomotionClass__Process_Movement`
- Ghidra read-only decompile: `FootClass__Mission_Enter`
- Ghidra read-only decompile: `BuildingClass__Receive_Radio`
