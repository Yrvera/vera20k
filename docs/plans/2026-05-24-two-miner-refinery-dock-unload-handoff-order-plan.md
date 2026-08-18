# Two-Miner Refinery Dock-Unload Handoff Order Implementation Plan

> For Codex: Execute this plan task-by-task. Keep the implementation inside `src/sim/miner/`. Do not replace the miner dock FSM with a generic radio event system in this plan.

**Goal:** Pin and, if needed, correct stock YR two-miner refinery handoff ordering so a finishing miner's state-4 release only frees its own contact/pad state, while the waiting miner enters only through its own later `Mission_Enter -> CAN_DOCK(0x0E)` retry. Preserve CMIN close-return deferral because it shares the same `HELLO -> MissionEnter -> CAN_DOCK` ordering contract.

**Design Doc:** [docs/plans/2026-05-23-two-miner-refinery-dock-unload-handoff-order-design.md](2026-05-23-two-miner-refinery-dock-unload-handoff-order-design.md)

---

## Grounding Summary

- `TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_FRAME_ORDER_GHIDRA_REPORT.md` audited GREEN on 2026-05-23: stock state-4 clears the finishing miner's unload/contact state, gates `BREAK(3)` on live contacts, and has no waiter promotion callback.
- `CMIN_STATE2_CLOSE_FAR_RETURN_TO_MISSION_ENTER_DISPATCH_GHIDRA_REPORT.md` audited GREEN on 2026-05-23: close CMIN state 2 sends only `HELLO/radio 0x02`; `Mission_Enter` and `CAN_DOCK(0x0E)` happen later.
- `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_STATE3_STATE4_TIMING_GHIDRA_REPORT.md` audited YELLOW, but its core state-3/state-4 timing facts were confirmed: no extra post-empty unload cooldown, state 4 clears unload-active state, and stock refineries have no active state-4 `ProductionAnim` wait.
- `CMIN_FORCE_TRACK_0X47_EXIT_FIRST_VISIBLE_MOVEMENT_AND_FACING_GHIDRA_REPORT.md` audited YELLOW, but its core stock-exit fact was confirmed: healthy stock zero-link unload completion does not call `ReleaseDockedHarvester` or `Force_Track(0x47)`.
- Current Rust already has the right broad shape: stable-id miner snapshots in `miner_system.rs`, dock FSM in `miner_dock_sequence.rs`, and shared contact state in `miner_dock.rs`.
- The main remaining implementation risk is not a large missing subsystem; it is insufficiently pinned behavior around release-vs-retry order, accepted-cell movement, and CMIN close-return deferral.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/miner/miner_dock.rs` | Document and test that release is cleanup-only and never promotes waiters. |
| Modify | `src/sim/miner/miner_dock_sequence.rs` | Preserve retry-owned `MissionEnter` admission and stock state-4 cleanup; make minimal corrections only if tests expose drift. |
| Modify | `src/sim/miner/miner_system.rs` | Preserve CMIN close-return `HELLO` deferral; make minimal corrections only if tests expose drift. |
| Modify | `src/sim/miner/miner_tests.rs` | Add focused parity pins for stable-id order, accepted-cell movement, and CMIN deferral. |

## Parity-Critical Items

| Item | Implementation Home | Verification |
|---|---|---|
| `release_contact` never promotes or mutates a waiter | `miner_dock.rs` | Unit test on `RefineryDockContacts`. |
| A lower stable id releases before B runs; B may claim on B's own same-tick `MissionEnter` when B is mission-dispatch eligible | `miner_dock_sequence.rs`, `miner_tests.rs` | Stable-id order integration test. |
| A lower stable id releases before B runs while B is still in `Approach`; B may claim `HELLO` contact but must not collapse into `CAN_DOCK` movement | `miner_dock_sequence.rs`, `miner_tests.rs` | Stable-id order integration test. |
| B lower stable id runs before A releases; B is not retroactively promoted by A's release | `miner_dock_sequence.rs`, `miner_tests.rs` | Stable-id order integration test. |
| Waiting/staging cell remains distinct from accepted dock cell | `miner_dock_sequence.rs`, `miner_tests.rs` | QueueingCell-to-accepted-cell test. |
| `HELLO` success does not issue accepted-cell movement in the same tick | `miner_system.rs`, `miner_dock_sequence.rs` | CMIN/war miner `HELLO` deferral tests. |
| Accepted-cell arrival requires a later already-there `MissionEnter` pass before entered/link | `miner_dock_sequence.rs` | Existing and focused accepted-cell tests. |
| Empty-slot gate does not seed post-empty `DepositCooldown` | `miner_dock_sequence.rs` | Existing regression test. |
| Stock healthy departure does not call `Force_Track(0x47)` | `miner_dock_sequence.rs` | Existing regression test. |

---

## Tasks

### Task 1: Add Direct Contact-Manager Regression Tests

**Why:** The lowest-level invariant is simple and load-bearing: releasing A must not promote B. Pin this before touching the FSM.

**Files:**

- `src/sim/miner/miner_dock.rs`
- `src/sim/miner/miner_tests.rs` only if local module tests are not practical.

**Steps:**

1. Add a focused unit test for `RefineryDockContacts` with one refinery, occupied contact A, and waiting B.
2. Call `release_contact(refinery, A)`.
3. Assert:
   - A is absent from `contacts`.
   - A's `contact_entered` is cleared.
   - B is still in `waiting_retry_queue`.
   - B is not in `contacts`.
   - `on_pad` is untouched unless `release_on_pad` is explicitly called.
4. Add a second test if useful for `release_on_pad` + `release_contact` matching state-4 cleanup order.
5. If existing helper `release(refinery)` auto-promotes through the older compatibility path, do not use it in stock state-4 tests; either document it as compatibility-only or narrow its use.

**Acceptance:**

- `release_contact` is proven cleanup-only.
- No production behavior changes unless the test exposes an existing direct-promotion bug.

**Run:**

```powershell
cargo test -q release_contact --lib
```

### Task 2: Pin Same-Tick Handoff When Releaser Runs Before Waiter

**Why:** GREEN audit permits same-tick B admission only when B's own processing happens after A's release and B is mission-dispatch eligible. This is the common stable-id ordering case when A has the lower id.

**Files:**

- `src/sim/miner/miner_tests.rs`
- `src/sim/miner/miner_dock_sequence.rs` only if the test fails.

**Steps:**

1. Build a test with:
   - A stable id lower than B.
   - A in `Dock/Departing`, with contact, entered flag, and pad occupancy.
   - B in `Dock/MissionEnter`, waiting in `waiting_retry_queue`, cargo present, same refinery reserved. Treat this setup as the modeled mission-timer-eligible case for this slice; do not claim broader native timer parity from this test alone.
   - B positioned at accepted cell `(refinery_rx + 3, refinery_ry + 1)` to isolate handoff order from movement travel.
2. Tick miners once.
3. Assert A released and returned to search.
4. Assert B entered through its own processing:
   - B has contact after the tick.
   - B has `contact_entered`.
   - B is exactly `Linked` for the accepted-cell already-there path.
   - B is no longer queued.
   - B is not yet marked `on_pad` unless this same test also drives the later `Linked`/pad-arrival handoff.
5. Assert no `Force_Track(0x47)` or queue-cell exit move was created for A.

**Acceptance:**

- The test demonstrates same-tick takeover is B-owned, not release-owned.
- The test demonstrates the mission-dispatch-eligible case, not all native mission timer values.
- If current Rust already passes, leave production code unchanged.

**Run:**

```powershell
cargo test -q two_miners_waiter_after_releaser --lib
```

### Task 3: Pin Same-Tick HELLO-Only Case When Releaser Runs Before Approach Waiter

**Why:** A waiter may still be in `Approach` after a denied `HELLO`. If A releases before that waiter runs, the waiter may acquire contact in the same tick, but it must not also issue `CAN_DOCK` movement or set entered state during that `HELLO` pass.

**Files:**

- `src/sim/miner/miner_tests.rs`
- `src/sim/miner/miner_dock_sequence.rs` only if the test fails.

**Steps:**

1. Build a test with:
   - A stable id lower than B.
   - A in `Dock/Departing`, with contact, entered flag, and pad occupancy.
   - B in `Dock/Approach`, waiting in `waiting_retry_queue`, cargo present, same refinery reserved.
   - B at or near the stock wait `QueueingCell` so no accepted-cell movement is already satisfied.
2. Tick miners once.
3. Assert:
   - A released and returned to search.
   - B gained `contacts` through B's own `HELLO` retry.
   - B moved to `Dock/MissionEnter`.
   - B did not receive accepted-cell movement in that same tick.
   - B did not set `contact_entered`.
   - B is not `Linked` and is not on pad.
4. Tick the next miner pass.
5. Assert `MissionEnter` now owns `CAN_DOCK` movement or already-there admission.

**Acceptance:**

- `Approach` remains HELLO-only even when a release happened earlier in the same tick.
- The plan preserves the audited split between `HELLO` and `CAN_DOCK`.

**Run:**

```powershell
cargo test -q two_miners_waiter_after_releaser_approach_hello_only --lib
```

### Task 4: Pin Next-Tick Handoff When Waiter Runs Before Releaser

**Why:** This is the gap most likely to be hidden by current broad tests. If B's snapshot already ran before A releases, A's state-4 cleanup must not retroactively promote B.

**Files:**

- `src/sim/miner/miner_tests.rs`
- `src/sim/miner/miner_dock_sequence.rs` or `src/sim/miner/miner_dock.rs` only if the test fails.

**Steps:**

1. Build a test with:
   - B stable id lower than A.
   - B in `Dock/MissionEnter`, waiting on A's occupied refinery.
   - A in `Dock/Departing`, with contact, entered flag, and pad occupancy.
2. Tick miners once.
3. Assert:
   - B remains not accepted during that tick because B processed before A.
   - A releases later in the same tick.
   - B remains queued after A's release.
   - B is not `Linked`, has no `contact_entered`, and is not on pad.
4. Tick miners a second time.
5. Assert B can now claim through B's own `MissionEnter` pass.

**Acceptance:**

- No release-side retroactive mutation of B.
- Stable-id order visibly controls whether handoff is same-tick or next-tick.

**Run:**

```powershell
cargo test -q two_miners_waiter_before_releaser --lib
```

### Task 5: Pin QueueingCell vs Accepted Cell Transition

**Why:** The audited docs distinguish wait staging at art `QueueingCell=4,1` from accepted dock admission at refinery NW `+(3,1)`. A waiter must move from staging to accepted cell before entered/link.

**Files:**

- `src/sim/miner/miner_tests.rs`
- `src/sim/miner/miner_dock_sequence.rs` only if the test fails.

**Steps:**

1. Put B at the stock wait `QueueingCell` for a 4x3 refinery at `(10,10)`: `(14,11)`.
2. Give B a free contact/admission opportunity through `MissionEnter`.
3. Tick once.
4. Assert:
   - B does not set `contact_entered` while still at `(14,11)`.
   - B receives movement toward accepted cell `(13,11)`.
   - B transitions through `AwaitingAcceptedCell`, not directly to `Linked`.
5. Complete or simulate the accepted-cell move in one explicit way:
   - preferred: drive real movement ticks until the entity reaches `(13,11)` and `movement_target` is `None`;
   - acceptable for a focused FSM test: manually set the entity position to `(13,11)` and clear `movement_target = None` in the setup block, with a comment that this skips movement physics and tests only the radio/FSM recheck.
6. Tick again and assert B returns to `MissionEnter`.
7. Tick the already-there pass and assert `contact_entered` / `Linked`.

**Acceptance:**

- `QueueingCell` and accepted cell remain separate concepts.
- `0x12` movement and already-there handshake ordering are pinned.

**Run:**

```powershell
cargo test -q waiter_moves_from_queueingcell_to_accepted_cell --lib
```

### Task 6: Pin CMIN Close-Return Deferral

**Why:** The two-miner handoff depends on the same broader contract: `HELLO` and `CAN_DOCK` are separate dispatches. CMIN is the most visible route for this because close returns are normal play.

**Files:**

- `src/sim/miner/miner_tests.rs`
- `src/sim/miner/miner_system.rs` only if the test fails.
- `src/sim/miner/miner_dock_sequence.rs` only if the follow-up admission path fails.

**Steps:**

1. Add or strengthen a CMIN close-return test:
   - CMIN full cargo.
   - Distance to refinery at or below `ChronoHarvTooFarDistance * 0x100`.
   - No active contact at refinery.
2. Tick once through return selection.
3. Assert:
   - `HELLO` created contact.
   - Miner moved to `Dock/MissionEnter`.
   - No accepted-cell movement was issued in the same tick.
   - No `contact_entered` flag was set.
4. Tick the next miner pass.
5. Assert `MissionEnter` now issues accepted-cell movement or enters only if already at the accepted cell.
6. Add a refused/busy CMIN close-return case:
   - Occupant holds contact/pad.
   - CMIN waiter stages at `QueueingCell`.
   - Later acceptance uses accepted cell `(rx + 3, ry + 1)`, not `QueueingCell`.

**Acceptance:**

- CMIN close return does not collapse `HELLO` and `CAN_DOCK`.
- Refused CMIN waits at `QueueingCell`, then uses accepted cell after admission.

**Run:**

```powershell
cargo test -q cmin_close --lib
```

### Task 7: Make Minimal Production Corrections If Tests Expose Drift

**Why:** The design expects current Rust to be close. Keep code changes narrow and test-driven.

**Files:**

- `src/sim/miner/miner_dock.rs`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/miner/miner_system.rs`

**Allowed corrections:**

1. If `release_contact` or a state-4 path promotes a waiter, remove that promotion from stock state-4 cleanup.
2. If `phase_departing` mutates any miner except the departing miner, move that behavior into the waiter's own `MissionEnter` processing or remove it.
3. If `phase_mission_enter` links a miner immediately from wait staging without accepted-cell already-there recheck, restore the `AwaitingAcceptedCell -> MissionEnter -> Linked` split.
4. If close CMIN accepted `HELLO` also issues movement, split it so movement starts only when `MissionEnter` processes.
5. If busy/refused close return uses accepted cell as staging, restore `QueueingCell` staging and keep accepted cell for `CAN_DOCK`.

**Disallowed corrections in this plan:**

- Do not add a global radio event queue.
- Do not implement `ReleaseDockedHarvester` for healthy stock unload completion.
- Do not start `Force_Track(0x47)` for healthy stock completion.
- Do not implement runtime visual duration for mid-unload refinery loss.
- Do not broaden this into full contact saturation or queue eviction parity.

**Acceptance:**

- All tests from Tasks 1-6 pass.
- Existing stock no-`Force_Track`, no-post-empty-cooldown, and accepted-cell tests still pass.

**Run:**

```powershell
cargo test -q miner::miner_tests:: --lib
```

If that selector is unsupported, run:

```powershell
cargo test -q miner_tests --lib
```

### Task 8: Focused Regression Sweep

**Why:** This area is tick-order sensitive. Run the narrow tests first, then the relevant broader module checks.

**Files:** none unless failures reveal needed fixes.

**Steps:**

1. Run the direct new tests by name.
2. Run existing nearby tests:
   - `hello_before_mission_enter_then_can_dock_move`
   - `accepted_cell_arrival_rechecks_can_dock_before_entered_flag`
   - `occupied_can_dock_defers_without_clearing_waiting_miner_target`
   - `empty_unload_gate_releases_dock_on_next_stock_state4_handoff`
   - `queued_miner_takes_over_immediately_after_empty_gate_handoff`
   - `stock_departing_does_not_start_force_track_0x47`
3. Run the full miner test module if time allows.
4. If unrelated dirty-worktree failures appear, report them without fixing unrelated files.

**Run:**

```powershell
cargo test -q hello_before_mission_enter_then_can_dock_move --lib
cargo test -q accepted_cell_arrival_rechecks_can_dock_before_entered_flag --lib
cargo test -q occupied_can_dock_defers_without_clearing_waiting_miner_target --lib
cargo test -q empty_unload_gate_releases_dock_on_next_stock_state4_handoff --lib
cargo test -q queued_miner_takes_over_immediately_after_empty_gate_handoff --lib
cargo test -q stock_departing_does_not_start_force_track_0x47 --lib
```

## Stop Conditions

- Stop and reassess if a focused test requires changing world tick order outside `src/sim/miner/`.
- Stop and request new research if implementation needs exact runtime first rendered movement or visual stale-frame duration.
- Stop if a fix would require changing `sim/` dependencies toward render/UI/sidebar/audio/net.
- Stop if existing unrelated dirty changes in the worktree make miner behavior impossible to isolate.

## Expected Outcome

After this plan, the miner dock handoff has explicit tests proving:

- Release is cleanup-only.
- Waiters enter through their own retry.
- Same-tick vs next-tick admission depends on stable-id processing order.
- CMIN close return preserves HELLO-before-CAN_DOCK ordering.
- Queue staging and accepted-cell movement remain distinct.

If production code already matches the verified behavior, this plan may land mostly as tests and comments. That is acceptable; the value is locking down parity-critical ordering before later miner/refinery changes.
