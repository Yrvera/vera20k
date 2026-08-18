# Current Rust Two-Miner Tests vs Binary Rule - Ghidra/Rust Audit

**Target:** `CURRENT_RUST_TWO_MINER_TESTS_VS_BINARY_RULE`  
**Investigation Mode:** exhaustive-slice for current Rust tests/implementation against the already-verified `Mission_Enter` / `CAN_DOCK` binary rule.  
**Primary binary source:** `miner/MISSION_ENTER_CANDOCK_RETRY_SAME_FRAME_ORDER_GHIDRA_REPORT.md`.  
**Rust source scanned:** `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`, `src/sim/production/production_types.rs`.  
**Non-Scope:** implementing fixes, runtime replay capture, first rendered displacement, and validating the timed-out full two-CMIN runtime slot.  
**Confidence:** High for source/test audit; Medium for exact runtime replay outcome because binary evidence still defers object-vector/timer values to runtime logging.

## Summary

Current Rust refinery dock tests mostly encode the verified binary rule correctly:

- No refinery-side waiter promotion is asserted in the active refinery contact path.
- Same-tick takeover is tested as order-dependent, not universal.
- Waiter-before-releaser remains queued until its next own `MissionEnter` pass.
- `QueueingCell=NW+(4,1)` and accepted `CAN_DOCK` cell `NW+(3,1)` are kept distinct.
- Approach/HELLO acceptance does not collapse into immediate `CAN_DOCK`.

No Rust source was edited. No test run was performed for this slot; this is a read-only source/evidence audit.

The only caution is naming/history: `miner_dock.rs` still contains a generic `DockReservations` helper with `release_promotes_next` tests. That helper is not the active refinery reservation type in `ProductionState`; refineries use `RefineryDockContacts`. Treat those old helper tests as `INTENTIONAL_INTERNAL_DIFFERENCE` / non-refinery compatibility coverage, not as evidence for refinery-side promotion.

## Binary Rule Baseline

| Binary rule | Evidence | Status for this audit |
|---|---|---|
| Miner A state-4 release does not scan/promote waiting miners. | `MISSION_ENTER...` lines 35-47; `UnitClass::Mission_Deploy_Building @ 0x0073D630`; `RadioClass` `BREAK(3)` clear at `0x0065A970` / `0x0065A820`. | Baseline |
| Miner B is admitted only by B's own `FootClass::Mission_Enter` sending `CAN_DOCK(0x0E)`. | `MISSION_ENTER...` lines 49-59; `0x004D9290`. | Baseline |
| Same-frame admission is conditional on live-object order and mission timer eligibility. | `MISSION_ENTER...` lines 63-91; `0x005B3060`, `0x0055AFB0`. | Baseline |
| `QueueingCell=4,1` is not the accepted cell. Accepted `CAN_DOCK` sends NW `+(3,1)` and only already-there returns start entered handoff. | `MISSION_ENTER...` lines 96-106; `0x0043C2D0`, `0x004D8FB0`; art `QueueingCell=4,1`. | Baseline |

## Current Rust Behavior Matrix

| Rust behavior / test | Label | Evidence | Notes |
|---|---|---|---|
| Active refinery production state uses `RefineryDockContacts`, not generic `DockReservations`. | PASS | `src/sim/production/production_types.rs:205-206`; default at `:236`. | This keeps refinery semantics on the contact/radio model. |
| `release_contact` removes A's contact/entered marker and does not grant B contact. | PASS | `src/sim/miner/miner_dock.rs:124-131`; test `release_contact_does_not_promote_waiter` at `:366-390`. | Matches no refinery-side promotion. |
| `phase_departing` releases pad/contact and does not call a waiter promotion callback. | PASS | `src/sim/miner/miner_dock_sequence.rs:898-930`. | B can only observe this during B's later own processing. |
| Miner tick order is deterministic stable-id order and shared dock state mutates during the pass. | INTENTIONAL_INTERNAL_DIFFERENCE | `src/sim/miner/miner_system.rs:94-150`. | gamemd uses live-object vector order. Rust stable-id order can model the same dependency, but concrete retail vector order remains runtime-only. |
| Waiter processed after releaser can claim in the same Rust tick through its own `MissionEnter`. | PASS | `two_miners_waiter_after_releaser_same_tick_claims_on_own_mission_enter`, `src/sim/miner/miner_tests.rs:3076-3142`. | Correctly scoped to waiter-after-releaser and already MissionEnter/eligible. |
| Waiter in Approach after same-tick release performs HELLO only, then CAN_DOCK later. | PASS | `two_miners_waiter_after_releaser_approach_hello_only`, `src/sim/miner/miner_tests.rs:3144-3220`; `phase_approach` at `miner_dock_sequence.rs:575-608`. | Prevents HELLO collapsing into accepted-cell movement. |
| Waiter processed before releaser is not retroactively promoted. | PASS | `two_miners_waiter_before_releaser_not_retroactively_promoted`, `src/sim/miner/miner_tests.rs:3222-3295`. | Directly covers the binary's no-callback implication. |
| Busy `CAN_DOCK` defers without clearing target or entered state. | PASS | `occupied_can_dock_defers_without_clearing_waiting_miner_target`, `src/sim/miner/miner_tests.rs:2965-3021`; `phase_mission_enter` at `miner_dock_sequence.rs:611-647`. | Matches receiver-side busy/non-ROGER behavior for Rust's current abstraction. |
| Accepted-cell movement and entered handoff are split across passes. | PASS | `accepted_cell_arrival_rechecks_can_dock_before_entered_flag`, `src/sim/miner/miner_tests.rs:2835-2885`; `waiter_moves_from_queueingcell_to_accepted_cell_before_entered`, `:2887-2963`. | Matches `0x12` move versus already-there entered handshake. |
| Queueing cell helper and accepted-cell helper are distinct. | PASS | `refinery_queue_cell` and `refinery_can_dock_queue_cell`, `src/sim/miner/miner_dock_sequence.rs:82-105`; test `refinery_pad_and_conditional_release_cells`, `src/sim/miner/miner_tests.rs:1970-2011`. | Correctly keeps art `QueueingCell` separate from hardcoded receiver target. |
| Test `queued_miner_takes_over_immediately_after_empty_gate_handoff` expects next-tick takeover after empty gate. | PASS WITH WORDING CAUTION | `src/sim/miner/miner_tests.rs:4584-4651`. | In this setup occupant id `1` releases before waiter id `3`, so the "immediate next tick" is order-dependent and valid. Avoid generalizing name/body beyond that fixture. |
| Generic `DockReservations::release`/`release_promotes_next` promotes from queue. | INTENTIONAL_INTERNAL_DIFFERENCE | `src/sim/miner/miner_dock.rs:202-306`; tests at `:327-356`; production refinery state uses `RefineryDockContacts`, not this type. | This helper is still used for depots and old compatibility-style tests. It should not be cited for refinery parity. |

## Verified Facts

1. **PASS - Active refinery state uses `RefineryDockContacts`.**  
   Evidence: `ProductionState.dock_reservations` is `RefineryDockContacts` at `src/sim/production/production_types.rs:205-206`, defaulting to `RefineryDockContacts::default()` at `:236`.

2. **PASS - Current refinery contact release does not promote a waiter.**  
   Evidence: `RefineryDockContacts::release_contact` at `src/sim/miner/miner_dock.rs:124-131` only removes the released miner's contact/entered/waiter records. Test `release_contact_does_not_promote_waiter` asserts the front waiter remains queued and contactless at `src/sim/miner/miner_dock.rs:366-390`. Binary source: no A release callback in `UnitClass::Mission_Deploy_Building @ 0x0073D630`; B retry owner is `FootClass::Mission_Enter @ 0x004D9290`.

3. **PASS - Same-frame admission is represented as order-dependent.**  
   Evidence: Rust processes miner snapshots in stable-id order at `src/sim/miner/miner_system.rs:94-150`. Test `two_miners_waiter_after_releaser_same_tick_claims_on_own_mission_enter` uses occupant id `1`, waiter id `3`, and expects same-tick claim only during waiter's own later `MissionEnter` pass at `src/sim/miner/miner_tests.rs:3076-3142`. Test `two_miners_waiter_before_releaser_not_retroactively_promoted` uses waiter id `1`, occupant id `3`, and asserts no retroactive promotion at `:3222-3295`. Binary source: live-object order and mission timer gates at `0x0055AFB0` and `0x005B3060`.

4. **PASS - `QueueingCell` and accepted `CAN_DOCK` cell are distinct in helpers and tests.**  
   Evidence: `refinery_queue_cell` consumes art `QueueingCell` at `src/sim/miner/miner_dock_sequence.rs:82-97`; `refinery_can_dock_queue_cell` hardcodes NW `+(3,1)` at `:100-105`. Tests assert `QueueingCell` `(14,11)` is not accepted cell `(13,11)` at `src/sim/miner/miner_tests.rs:1970-2011`, `:2887-2963`, and `:1310-1399`.

5. **UNCHECKED - Rust does not currently model gamemd's exact mission timer jitter for this handoff.**  
   Evidence: binary report states `FootClass::Mission_Enter` returns mission timer entry plus `RandomRanged(0,2)` and same-frame claim requires timer eligibility (`MISSION_ENTER...` lines 63-72). Current tests set miners directly into eligible phases and do not prove natural jitter values. This is acceptable for unit tests of the rule, but concrete retail replay timing remains runtime-only.

## Implementation Handoffs

1. **Keep the current refinery contact model; do not replace it with generic FIFO promotion.**  
   Affected surfaces: `src/sim/miner/miner_dock.rs::RefineryDockContacts`, `src/sim/miner/miner_dock_sequence.rs::phase_departing`, `src/sim/miner/miner_system.rs::tick_miners`.  
   Acceptance: `release_contact_does_not_promote_waiter`, `two_miners_waiter_before_releaser_not_retroactively_promoted`, and `two_miners_waiter_after_releaser_same_tick_claims_on_own_mission_enter` remain true.

2. **Preserve order-dependent test phrasing.**  
   Affected tests: `two_miners_waiter_after_releaser_same_tick_claims_on_own_mission_enter`, `two_miners_waiter_before_releaser_not_retroactively_promoted`, `queued_miner_takes_over_immediately_after_empty_gate_handoff`.  
   Acceptance: no test should claim universal same-tick takeover or universal next-tick takeover without specifying processing order and eligibility.

3. **If runtime parity for a concrete replay is needed, add instrumentation rather than more static code audit.**  
   Needed evidence: gamemd live-object vector order for A/refinery/B, B mission timer start/duration at the release frame, and first rendered movement frame.  
   Current status: source tests cover the static rule, not the concrete replay instance.

## Negative Facts / Do Not Do

- Do not cite `DockReservations::release_promotes_next` as refinery parity evidence; active refinery state is `RefineryDockContacts`.
- Do not add a refinery-side waiter promotion callback to `phase_departing`.
- Do not assert same-frame takeover is guaranteed.
- Do not assert same-frame takeover is impossible.
- Do not collapse Approach/HELLO into immediate `CAN_DOCK` movement.
- Do not collapse art `QueueingCell=4,1` and accepted `CAN_DOCK` target `+(3,1)`.

## Remaining Uncertainty

- Concrete retail two-CMIN takeover frame order still needs runtime logging of live-object vector order and B's mission timer state. This audit confirms Rust tests encode the static binary rule, not that a natural retail replay picks the same frame as a given Rust fixture.
- Exact first rendered displacement/pixel overlap remains outside this report.
- Generic `DockReservations` remains in `miner_dock.rs` for depot/compatibility surfaces; a future cleanup could reduce naming confusion, but it is not a current refinery behavior mismatch.

## Stale-Doc Wording

- Replace any remaining "Rust promotes queued miner when refinery releases" wording with: "Current refinery Rust uses `RefineryDockContacts`; release frees A's contact/pad, and B claims only on B's own later `MissionEnter`/`CAN_DOCK` processing. Generic `DockReservations` promotion tests are not the refinery path."
- Narrow any "queued miner takes over immediately" wording to include fixture order/eligibility: "same-tick or next-tick takeover depends on whether the waiter is processed after release and is MissionEnter-eligible."

## Status

COMPLETE
