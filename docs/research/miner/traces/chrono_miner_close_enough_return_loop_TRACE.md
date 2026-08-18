# Chrono Miner CloseEnough Return Loop Trace

Scenario: loaded Allied Chrono Miner (`CMIN`) returning to a GAREFN dock target. Concrete observed state: unit cell `(88,183)`, goal/dock cell `(88,181)`, no active movement target after movement stopped inside `CloseEnough`.

Scope guard: one mechanic only, the CloseEnough interaction during ReturnToRefinery. Ghidra MCP use was read-only: decompile/search only.

> **Repo-status supersession 2026-05-25:** This trace is stale for current Rust
> implementation guidance where it attributes the loop to a hardcoded 2-cell
> chrono-return gate. Current Rust reads `ChronoHarvTooFarDistance` for the
> close/far split. Re-run this scenario before using its FAIL tally.

## Verdict

PASS: 2 | FAIL: 4 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

Player-visible answer: gamemd.exe does not unload just because the miner is within `CloseEnough`, and it does not teleport for this near-refinery case. The active YR path re-enters the harvester dock/enter handshake and reissues movement through `Mission_Enter`/`CAN_DOCK`. Our current CMIN path teleports to the pad first because a hardcoded 2-cell chrono-return gate runs before the CloseEnough docking fallback.

## Concrete Values

- `rulesmd.ini:58`: `CloseEnough=2.25`, parsed in Rust as 576 leptons (`2.25 * 256`).
- Scenario distance `(88,183)` to `(88,181)`: `dx=0`, `dy=2`, Rust miner helper distance `(0 + 2) * 256 = 512`, so `512 < 576` is true.
- GAREFN anchor implied by the existing regression fixture: `(85,180)`, radio dock target `(88,181)`, foundation `4x3`.
- Rust chrono-return center for GAREFN: `(85 + 4/2, 180 + 3/2) = (87,181)`.
- Rust chrono-return threshold: `CHRONO_INBOUND_WARP_THRESHOLD_CELLS = 2`; from `(88,183)` to center `(87,181)`, `dist_sq = 1^2 + 2^2 = 5`, threshold square is `4`, so Rust considers the miner far enough and initiates teleport.
- gamemd chrono harvester threshold: `ChronoHarvTooFarDistance=50`, used in `UnitClass__Mission_Harvest @ 0073e5e0` as `RulesClass+0xD7C * 0x100 = 12800` leptons. The near-refinery distance is about `512` to `572` leptons, so it is in the non-teleport dock handshake branch.

## Pipeline

1. Movement close-enough stop.
2. Return-to-refinery state tick after movement target is gone.
3. Chrono-specific near/far decision.
4. Dock admission and movement reissue.
5. Pad arrival and unload trigger.
6. Player-visible audio/visual result.

## Stage Results

### Stage 1 - Data Loading

Our values: `CloseEnough=576`, `CMIN` is `Harvester=yes`, `Dock=NAREFN,GAREFN`, `GAREFN` has `DockUnload=yes` and `NumberOfDocks=1`.

gamemd evidence: active YR decompile reads `TechnoType+0xE0E` for Harvester and `RulesClass+0x1718` for CloseEnough in `DriveLocomotionClass__Process_Movement @ 004b2630`; `UnitClass__Mission_Harvest @ 0073e5e0` reads `RulesClass+0xD7C` for chrono harvester range.

Verdict: PASS for the loaded constants used by this trace.

### Stage 2 - Movement Stops Within CloseEnough

Our path: movement blocked logic computes `dist = (dx + dy) * 256`; for `(88,183)` to `(88,181)`, `512 < 576`, pushes the entity into finished movement, and `finalize_finished_entities` clears `movement_target`.

gamemd evidence: `DriveLocomotionClass__Process_Movement @ 004b2630` compares distance against `RulesClass+0x1718` and clears destination/stops when the path cannot continue and the object is close enough. The close-enough stop path is active YR ground locomotion, not TS-only code.

Verdict: PASS for the stop condition in the pre-Enter/non-Mission_Enter movement stage.

### Stage 3 - Return Tick After Stop

Our path: `handle_return` checks `!moving && try_issue_chrono_return_teleport(...)` before `stopped_close_enough`. With this concrete GAREFN geometry, `dist_sq=5 > 4`, so Rust issues a harvester teleport to `(88,181)`, emits chrono effects/sounds, and does not take the CloseEnough-to-Dock branch on that tick.

gamemd evidence: `UnitClass__Mission_Harvest @ 0073e5e0` state 2 checks the chrono harvester branch against `ChronoHarvTooFarDistance * 256`; near-refinery distance is under `12800`, so it sends radio `2` to the refinery, sets state 3 on acceptance, and state 3 sets mission `7` (`Mission_Enter`). No self-teleport is taken for this near case.

Verdict: FAIL. Player-visible difference: our CMIN blinks/warps two cells onto the pad; gamemd keeps the ordinary dock-enter flow.

### Stage 4 - CloseEnough As Dock Trigger

Our path: if the teleport branch were not taken, `handle_return` treats `movement_target.is_none()` plus `is_within_close_enough(...)` as sufficient to set `MinerState::Dock` and `RefineryDockPhase::Approach`.

gamemd evidence: after `Mission_Harvest` transfers to `FootClass__Mission_Enter @ 004d9290`, the unit sends `CAN_DOCK(0x0E)` to the destination. `DriveLocomotionClass__Process_Movement @ 004b2630` explicitly gates the generic CloseEnough abort behind `!FootClass__Is_Mission_Enter`, so CloseEnough is not itself the Mission_Enter docking trigger. The building replies through active YR `BuildingClass__Receive_Radio @ 0043c2d0` case `0x0E`, returning cell `anchor + (3,1)` and sending `0x18`/`0x16`.

Verdict: FAIL. Player-visible difference: our state machine can promote a close stopped returner into dock approach without the same active-YR Mission_Enter/CAN_DOCK reissue point.

### Stage 5 - Unload Entry

Our path: because the concrete CMIN teleports to `(88,181)`, later ticks can proceed from the pad into `Linked`, pivot, and unload without the missing two-cell drive.

gamemd evidence: `UnitClass__PerCellProcess @ 00739ec0` only sends dock-now radio `0x15` after the unit's current cell equals the building's dock coordinate. At `(88,183)` vs dock `(88,181)`, that equality is false, so gamemd does not enter unload from the CloseEnough stop cell.

Verdict: FAIL. Player-visible difference: our miner can appear on the dock pad and begin the unload sequence earlier; gamemd must still satisfy the dock-cell arrival test.

### Stage 6 - Audio/Visual Effects

Our path: `try_issue_chrono_return_teleport` calls `spawn_warp_effects` and `issue_teleport_command(..., is_harvester=true)`, so the near-refinery CMIN produces ChronoOut/ChronoIn sound events and warp effects.

gamemd evidence: the active `Mission_Harvest` near branch enters radio dock flow; the self-teleport fallback is for the too-far/passable-cell branch, not this 2-cell case.

Verdict: FAIL. Player-visible difference: our near-refinery return can play chrono teleport audio/visuals where gamemd would show continued docking movement.

### Stage 7 - Exact Tick Timing

Our timing: Rust issue point is the next miner tick after movement_target is absent; harvester teleport relocation completes in one teleport tick after it is issued.

gamemd timing: decompile confirms branch order and state changes, but this trace did not run an instrumented gamemd session to count exact frame numbers from the CloseEnough stop to the next CAN_DOCK and dock-cell arrival.

Verdict: UNCHECKED.

### Stage 8 - Queue Contention Variant

The no-active-target state is the only traced scenario. Queue contention, occupied pad, and blocked queue retry behavior are adjacent because they would change the outcome of `CAN_DOCK`/contact admission.

Verdict: UNCHECKED.

## Failures

1. Stage 3: Rust teleports a CMIN from `(88,183)` to `(88,181)` because `dist_sq=5 > 2^2`; gamemd near-refinery branch uses `ChronoHarvTooFarDistance=50` and enters radio dock flow instead.
2. Stage 4: Rust has a CloseEnough-to-Dock promotion path; gamemd's `Mission_Enter` path bypasses generic CloseEnough abort and reissues docking movement through `CAN_DOCK`.
3. Stage 5: Rust can reach pad/unload after a near-range teleport; gamemd only sends dock-now once the current cell equals the dock coordinate.
4. Stage 6: Rust emits chrono teleport visuals/sounds for this near case; gamemd does not.

## Active-YR Confirmation

- `UnitClass__Mission_Harvest @ 0073e5e0`: active for standard YR harvesters; gates CMIN through `TechnoType+0xE0E`.
- `DriveLocomotionClass__Process_Movement @ 004b2630`: active ground locomotion path for the returning unit; CloseEnough is read from `RulesClass+0x1718`.
- `FootClass__Mission_Enter @ 004d9290`: active mission 7 handler used after harvester return state 3.
- `BuildingClass__Receive_Radio @ 0043c2d0` case `0x0E`: active YR refinery CAN_DOCK reply path.
- `UnitClass__PerCellProcess @ 00739ec0`: active per-cell hook that sends dock-now only on exact dock-cell arrival.

## Adjacent Findings

- Existing Rust regression `return_close_enough_to_refinery_enters_dock` covers a War Miner in this geometry, not a CMIN. For CMIN the earlier hardcoded chrono threshold changes the outcome.
- The separate known issue that Rust uses a 2-cell chrono-return threshold instead of `ChronoHarvTooFarDistance=50` is the dominant cause here, but this trace did not expand into a full chrono return threshold audit.

## References

- `src/sim/miner/miner_system.rs:40`, `:580`, `:634`, `:638`, `:643`, `:844`, `:856`, `:1251`
- `src/sim/miner/miner_dock_sequence.rs:441`, `:490`
- `src/sim/movement/movement_blocked.rs:87`
- `src/sim/movement/movement_tick.rs:1022`
- `src/sim/movement/teleport_movement.rs:103`, `:120`, `:134`, `:184`
- `src/sim/miner/miner_tests.rs:460`
- `ini/rulesmd.ini:58`, `:294`, `:7351`, `:7361`, `:7364`, `:11722`, `:11726`, `:11729`
- `ini/artmd.ini:1763`, `:1766`, `:1773`
- Ghidra read-only decompile: `UnitClass__Mission_Harvest @ 0073e5e0`
- Ghidra read-only decompile: `DriveLocomotionClass__Process_Movement @ 004b2630`
- Ghidra read-only decompile: `FootClass__Mission_Enter @ 004d9290`
- Ghidra read-only decompile: `BuildingClass__Receive_Radio @ 0043c2d0`
- Ghidra read-only decompile: `UnitClass__PerCellProcess @ 00739ec0`
