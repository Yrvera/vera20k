# Two-Miner Refinery Dock-Unload Handoff Order Design

## Goal

Preserve stock YR refinery handoff order when one miner finishes unloading while another waits: release only frees the first miner's contact, and the waiting miner enters only through its own later `Mission_Enter -> CAN_DOCK(0x0E)` retry.

## Architecture Context

The implementation belongs entirely under `src/sim/miner/`.

`miner_system.rs` ticks miners through a two-phase snapshot pattern: collect all miner snapshots from `EntityStore`, process them in stable-id order, then write miner components back. Shared dock state in `sim.production.dock_reservations` is mutated during snapshot processing, so later miners in the same tick can observe release/contact changes made by earlier miners.

`miner_dock_sequence.rs` owns the active refinery docking FSM:

- `Approach`: Mission_Harvest-style `HELLO(0x02)` contact request.
- `MissionEnter`: `CAN_DOCK(0x0E)` admission and accepted-cell movement.
- `AwaitingAcceptedCell`: waits for the accepted-cell move to finish.
- `Linked` / `Pivoting` / `Unloading`: pad arrival, facing pivot, and cargo dump gates.
- `Departing`: stock zero-link state-4 cleanup.

`miner_dock.rs` stores refinery contact state:

- `contacts`: equivalent to refinery `Contacts[]` populated by `HELLO`.
- `waiting_retry_queue`: deterministic retry order for denied miners.
- `contact_entered`: the stock `0x18` / `+0x418`-style entered flag.
- `on_pad`: physical stock refinery pad occupancy.

The current module shape is a good fit for the verified behavior because admission is already miner-owned: a waiter calls `hello_or_wait` during its own phase processing, rather than being mutated directly by the refinery release code.

## Impact Analysis

Primary files:

- `src/sim/miner/miner_system.rs`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/miner/miner_dock.rs`
- `src/sim/miner/miner_tests.rs`

Risk areas:

- Tick ordering: stable-id processing determines whether a waiter can enter on the same sim tick as the previous miner's state-4 release.
- Snapshot writeback: if a waiter snapshot was processed before release, it must not be retroactively promoted by `release_contact`.
- Contact vs pad state: `contacts`, `contact_entered`, and `on_pad` must remain separate.
- Movement target handling: wait staging uses `QueueingCell`, but accepted dock admission uses refinery NW `+(3,1)`.
- Regression risk around old helper APIs such as `try_reserve` / `release`, which can imply direct promotion if used carelessly.

No cross-layer dependencies are needed. `sim/` remains independent of render, UI, sidebar, audio, and net.

## Chosen Approach

Use **retry-owned handoff with the current FSM tightened**.

State-4 release should only clear the finishing miner's contact, entered flag, pad state, visual override, movement/facing state, and miner dock bookkeeping. It must not select, promote, or mutate the next waiting miner.

The waiting miner remains responsible for entering:

1. It stays in `Approach` or `MissionEnter`, with `dock_queued = true`.
2. On its own later snapshot processing, it calls `hello_or_wait`.
3. If it is at the front of `waiting_retry_queue` and capacity is free, it enters `contacts`.
4. `MissionEnter` then sends the `CAN_DOCK` equivalent, moves to accepted cell `NW+(3,1)` if needed, and only starts the entered/pivot handoff on a later already-there pass.

This preserves gamemd.exe's observable ordering without introducing a new radio event queue or direct refinery-side promotion.

## Tiny-Detail Ledger

- Stock target scenario is active YR `HARV/CMIN -> GAREFN/NAREFN`, with stock refinery `DockUnload=yes` and harvester `Dock=` links. Source: `AUDIT_LOG.md:229`, `AUDIT_LOG.md:230`.
- CMIN close return sends only `HELLO/radio 0x02`; it does not send `CAN_DOCK(0x0E)` in the same branch. Source: `AUDIT_LOG.md:229`, `UnitClass::Mission_Harvest @ 0x0073E5E0`.
- CMIN close threshold is inclusive `<= ChronoHarvTooFarDistance * 0x100`; stock value is `50 * 256`. Source: `AUDIT_LOG.md:229`.
- Close success writes harvest substate 3, then `Queue_Mission(7,0)` defers first `Mission_Enter` dispatch to a later mission-dispatch pass. Source: `AUDIT_LOG.md:229`.
- `Mission_Enter` is the first standard sender of `CAN_DOCK(0x0E)`. Source: `AUDIT_LOG.md:229`; parent spot-check `FootClass::Mission_Enter @ 0x004D9290`.
- Accepted dock cell is refinery NW `+(3,1)`, not art `QueueingCell=4,1`. Source: `AUDIT_LOG.md:230`.
- `0x12` only proceeds to `0x18/0x16` if the miner is already at the accepted cell; otherwise it commands movement to that cell. Source: `AUDIT_LOG.md:230`.
- A's zero-link state-4 release clears unload-active state, queues Harvest `0x0A`, gates `BREAK(3)` on live contacts, and contains no B-side promotion callback. Source: `AUDIT_LOG.md:230`.
- `BREAK(3)` clears sender-side contact first, then receiver-side contact. Source: `AUDIT_LOG.md:230`.
- B admission comes only from B's own later `Mission_Enter -> CAN_DOCK(0x0E)` retry, gated by mission timer and live-object order. Source: `AUDIT_LOG.md:230`.
- Same-tick B admission is conditional: B can enter on the same tick only if B is processed after A's release and is mission-timer eligible. Source: `AUDIT_LOG.md:230`.
- If B is processed before A's release, B remains queued until its next eligible dispatch; A's release must not retroactively alter B's snapshot. Source: `AUDIT_LOG.md:230`.
- Stock healthy state-4 exit must not call `ReleaseDockedHarvester` or start `Force_Track(0x47)`. Source: `AUDIT_LOG.md:231`.
- Empty-slot unload handoff must not add another post-empty `DepositCooldown`. Source: `AUDIT_LOG.md:227`.

## Design

### Components

#### `RefineryDockContacts`

Keep `release_contact(refinery_sid, miner_sid)` as a cleanup operation for the specified miner only:

- Remove `miner_sid` from `contacts`.
- Clear `contact_entered` only if it belongs to `miner_sid`.
- Remove `miner_sid` from `waiting_retry_queue`.
- Do not inspect the next waiter for promotion.
- Do not return a promoted miner id.

Keep `hello_or_wait(refinery_sid, miner_sid, capacity)` as the only normal path that moves a waiting miner into `contacts`. Its FIFO behavior should remain deterministic:

- If `contacts` is full, enqueue the sender if absent and return `Waiting`.
- If contacts are free but another waiter is ahead, enqueue/keep this sender and return `Waiting`.
- If this sender is at the front, pop it and insert it into `contacts`.
- If this sender already has contact, remove it from the waiting queue and return `Accepted`.

#### `phase_departing`

Keep state-4 stock cleanup one-way and local to the finishing miner:

- Release pad state for A.
- Release contact/entered state for A.
- Clear A's unload visual override, movement target, drive tracks, forced track, and facing target.
- Clear A's dock bookkeeping and return A to `SearchOre`.
- Do not touch any other miner component.
- Do not start `Force_Track(0x47)`.
- Do not seed a queue-cell exit move.

#### `phase_mission_enter`

Keep `CAN_DOCK` admission owned by the current miner snapshot:

- Call `hello_or_wait` for the current miner.
- If not accepted and not already entered, stay queued and stage at wait `QueueingCell` if needed.
- If accepted and pad is clear, issue movement to accepted cell `NW+(3,1)` unless already there.
- If already at accepted cell and not moving, set `contact_entered` and transition to `Linked`.
- If movement just completed, go back to `MissionEnter` so the already-there pass owns `0x18/0x16`.

The implementation should not add a shortcut that links a miner immediately after another miner releases.

#### `try_begin_chrono_close_return_radio`

Keep the CMIN close-return split:

- Near CMIN sends HELLO only.
- Accepted HELLO sets `MinerState::Dock` and `RefineryDockPhase::MissionEnter`.
- Refused HELLO sets `MinerState::Dock`, `RefineryDockPhase::Approach`, and stages at `QueueingCell`.
- Do not combine HELLO success with accepted-cell movement.

### Interfaces / Contracts

No new public API is required.

Internal contract to document in code/tests:

- `release_contact` never promotes waiters.
- `hello_or_wait` is the only normal waiter-to-contact transition.
- `phase_mission_enter` is the only normal `CAN_DOCK` path.
- `phase_departing` owns stock state-4 cleanup for healthy stock refinery unload.

If a helper is useful, it should be a test helper, not a production abstraction, unless implementation shows real duplication.

### Data Flow

Same-tick order where A has lower stable id than B:

1. A snapshot processes first in `Departing`.
2. A releases contact/pad and returns to search.
3. B snapshot processes later in `MissionEnter`.
4. B calls `hello_or_wait`, becomes accepted, and proceeds through the normal accepted-cell path.

Next-tick order where B has lower stable id than A:

1. B snapshot processes first while A still occupies contact/pad.
2. B remains queued.
3. A snapshot processes later and releases contact/pad.
4. No promotion happens during A's release.
5. B can only enter on B's next eligible snapshot processing.

This mirrors the audited live-object order dependency without needing to model a separate global radio queue.

### Error Handling

Invalid or dying refinery paths should keep using existing abort and cleanup behavior:

- Clear this miner's contact/pad state.
- Preserve cargo unless a verified unload drain occurred.
- Return full miners to refinery selection or search behavior according to existing abort helpers.

This design does not resolve the runtime-only visual duration for mid-unload refinery loss; that remains a trace target, not part of this handoff implementation.

### Testing Strategy

Add or adjust focused unit tests in `miner_tests.rs`.

Required tests:

- `two_miners_waiter_after_releaser_same_tick_claims_on_own_mission_enter`: A lower stable id releases, B higher stable id runs later and claims through B's own `MissionEnter`.
- `two_miners_waiter_before_releaser_not_retroactively_promoted`: B lower stable id runs before A, remains queued; A releases later; B does not become accepted until the next tick.
- `release_contact_does_not_promote_waiter`: direct `RefineryDockContacts` test that release clears only the released miner and leaves the front waiter in `waiting_retry_queue`.
- `waiter_moves_from_queueingcell_to_accepted_cell_before_entered`: B staged at `QueueingCell=4,1` must move to accepted cell `NW+(3,1)` before `contact_entered`/`Linked`.
- `cmin_close_hello_success_defers_can_dock_to_mission_enter`: close CMIN HELLO success does not issue movement in the same tick.
- `cmin_refused_close_return_stages_at_queueingcell_then_can_dock_uses_accepted_cell`: refused close CMIN waits at `QueueingCell`; later acceptance uses `NW+(3,1)`.

Regression tests to keep passing:

- Existing no post-empty cooldown behavior.
- Existing stock no `Force_Track(0x47)` behavior.
- Existing occupied `CAN_DOCK` deferral behavior.

Recommended command:

```powershell
cargo test miner::miner_tests::two_miners_waiter -- --nocapture
cargo test miner::miner_tests::cmin_close -- --nocapture
cargo test miner::miner_tests::release_contact_does_not_promote_waiter -- --nocapture
```

If exact names differ after implementation, run the full miner test module.

## Architectural Decisions

- Preserve the current stable-id snapshot processing model. This already matches the evidence that same-frame handoff depends on live-object order.
- Keep the dock contact manager passive on release. This avoids the parity drift of refinery-side promotion.
- Keep `QueueingCell` and accepted cell as separate concepts. `QueueingCell` is waiting/fallback staging; accepted admission uses `NW+(3,1)`.
- Do not introduce a global radio event queue for this target. It would be broader than required and would increase tick-order blast radius.
- Do not model stock healthy exit through conditional reciprocal-link helpers.

Tech debt:

- `RefineryDockContacts` still has compatibility helpers whose names can suggest direct reservation/promotion semantics. The implementation should avoid extending those semantics and may add comments/tests to keep them from being used as the stock handoff model.

## Alternatives Considered

### Global Radio Event Queue

This would model `HELLO`, `BREAK`, `CAN_DOCK`, `0x12`, `0x18`, `0x15`, and `0x16` as explicit queued events. It has good long-term fidelity potential, but it is too broad for this verified gap and would touch more tick systems than necessary.

### Direct Waiter Promotion In `release_contact`

Rejected. It is the opposite of the GREEN audit result: A's state-4 release contains no B promotion callback, and B admission is owned by B's later `Mission_Enter` retry.

### Leave Existing Broad Tests Only

Rejected. Current tests cover broad takeover behavior, but the audits require explicit pins for object-order split cases and for the difference between `QueueingCell` and accepted `NW+(3,1)` movement.
