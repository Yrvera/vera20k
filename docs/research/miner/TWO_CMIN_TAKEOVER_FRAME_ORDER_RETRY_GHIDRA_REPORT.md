# Two CMIN Takeover Frame Order Retry - Ghidra Research Report

**Address(es):** `0x0073D630`, `0x0065A970`, `0x0065A820`, `0x0055AFB0`, `0x005B3060`, `0x004D9290`, `0x0043C2D0`, `0x004D8FB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard YR stock healthy zero-link two full Chrono Miners / one stock refinery takeover order from miner A state-4 release through miner B's first eligible `Mission_Enter` / `CAN_DOCK(0x0E)` retry, accepted-cell movement, entered/pivot handoff condition, and static limits on first player-visible movement proof.  
**Non-Scope:** runtime capture of a concrete replay's live-object vector indices, B's exact mission timer/jitter value, rendered pixel displacement, refinery sold/destroyed interruption, nonzero reciprocal `+0x2E4` release, slave miners, service depots, multi-dock modded refineries.  
**Confidence:** High for static ordering and conditions; Medium for any concrete replay frame because runtime object order and B timer values are not recoverable from static Ghidra alone.  
**Active in YR:** Yes. Stock `[CMIN]` has `Harvester=yes`, `Teleporter=yes`, `Dock=NAREFN,GAREFN`, and stock `[GAREFN]/[NAREFN]` have `Refinery=yes`, `DockUnload=yes`, `NumberOfDocks=1`.

## 1. Overview

The timed-out target is resolved for static behavior. A stock healthy zero-link refinery unload completion does not promote a waiting miner. Miner A's state-4 release clears its unload-active byte, queues Harvest, and sends `BREAK(3)` only to release the existing contact. That radio clear is synchronous on A and the refinery, but it does not scan waiters or call miner B.

Miner B can claim the freed refinery only when B's own object AI reaches `MissionClass::Mission_Dispatch`, its mission timer is eligible, and mission id `7` dispatches `FootClass::Mission_Enter`. Same-frame admission is therefore possible but conditional: B must be processed after A in the live-object vector and have an expired mission timer. If B already ran earlier in the frame, or its timer is still blocked, there is no retroactive promotion.

`CAN_DOCK(0x0E)` acceptance is also not the final visible dock pivot unless B is already on the accepted cell. The refinery sends radio `0x12` with building anchor `+(3,1)`; if B is at the stock QueueingCell `+(4,1)`, the first accepted retry commands movement to `+(3,1)` and returns before `0x18/0x16`.

## 2. Key Fields / Constants

| Field / value | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass+0xBC` | deploy-building substate; `4` is stock state-4 release | `UnitClass::Mission_Deploy_Building @ 0x0073D630` | Yes |
| `UnitClass+0x6D1` | unload-active/display byte cleared in state 4 before release | `0x0073D630`, state-4 branch | Yes |
| `RadioClass+0xE4/+0xE8` | Contacts array and capacity | `0x0065A970`, `0x0065A820`; `NumberOfDocks=1` | Yes |
| mission id `7` | `Mission_Enter`; dispatched through vtable `+0x240` | `MissionClass::Mission_Dispatch @ 0x005B3060` | Yes |
| `MissionClass+0xC8/+0xD0` | mission timer start and delay gate | `0x005B3060` | Yes |
| live-object vector order | increasing index call to object vtable `+0x5C` | `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` | Yes |
| `CAN_DOCK(0x0E)` | refinery admission query from B's `Mission_Enter` | `0x004D9290`, `0x0043C2D0` | Yes |
| accepted cell | refinery NW cell plus `(3,1)`, not `QueueingCell=4,1` | `0x0043C2D0`; `artmd.ini` QueueingCell | Yes |
| radio `0x12` reply `0x14` | already-there condition before `0x18/0x16` | `FootClass::Receive_Radio @ 0x004D8FB0` | Yes |

## 3. Core Ordering

### 3.1 Miner A state-4 release

In `UnitClass::Mission_Deploy_Building @ 0x0073D630`, stock state 4 checks for a stock refinery slot-8 `ProductionAnim` wait first. Stock `GAREFN/NAREFN` do not define active `ProductionAnim`, so the healthy state-4 release can continue.

The release order is:

1. Clear `UnitClass+0x6D1`.
2. Queue/set Harvest mission `0x0A`.
3. If a valid radio contact exists, send `BREAK(3)`.
4. Fall through mission timer epilogue.

`RadioClass::Transmit_Radio_Impl @ 0x0065A970` handles outgoing `BREAK(3)` by nulling every sender Contacts slot equal to the target before forwarding the message. `RadioClass::Receive_Radio @ 0x0065A820` handles incoming `BREAK(3)` by finding the sender in the receiver Contacts array, calling the base radio side effect, nulling that slot, and returning `1`.

No branch in the verified A release path scans waiting miners, calls a refinery FIFO callback, or dispatches B's mission.

### 3.2 Miner B retry owner

`FootClass::Mission_Enter @ 0x004D9290` owns the retry. It gets the current destination/refinery and sends radio `0x0E` via the unit transmit vtable. If the reply is `1`, it continues the enter path; if the reply is not `1` and the unit is not already in the alternate entered state, it sends `BREAK(3)` and clears the destination.

Therefore B can observe A's release only when B's own `Mission_Enter` runs after A's `BREAK(3)` has cleared the refinery contact.

### 3.3 Mission timer gate

`MissionClass::Mission_Dispatch @ 0x005B3060` calls `ObjectClass::AI`, then returns if the object is inactive. It then gates mission dispatch with `+0xC8/+0xD0`: if the start frame is not `-1` and elapsed time is below the delay, it returns without calling mission id `7`.

When mission id `7` is eligible, the dispatcher calls vtable `+0x240`, then writes `+0xC8 = g_CurrentFrameCounter` and `+0xD0 = returned delay`. `FootClass::Mission_Enter` returns `Math__ftol(MissionClass::GetMissionTimerEntry()) + Random__RandomRanged(0,2)`, so a natural replay's next retry frame depends on the actual stored timer and jitter.

### 3.4 Live-object order

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` updates the main object vector by increasing index and invokes each object's vtable `+0x5C`. Consequences for the release frame:

- If A runs first and B runs later, and B's mission timer is eligible, B may claim the freed refinery in the same frame.
- If B already ran before A, A's later contact clear cannot retroactively promote B.
- If B runs later but the mission timer has remaining delay, B waits until a later eligible dispatch.

Static Ghidra proves the rule but not a particular replay's vector indices or B's timer value.

### 3.5 Accepted-cell and entered/pivot handoff

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` accepts stock refinery dock queries by sending radio `0x12` with the cell at building anchor `+(3,1)`. This is separate from stock `artmd.ini` `QueueingCell=4,1`.

`FootClass::Receive_Radio @ 0x004D8FB0` case `0x12` returns `0x14` only when the unit is already at the payload cell. Otherwise it issues a movement destination to that cell, writes mission timer start to the current frame, clears delay to zero, and returns `1`.

Only after an already-there `0x14` reply does the refinery send `0x18`, then `0x16`. Thus a waiting CMIN at QueueingCell first receives movement to the accepted cell. Static code cannot prove the first rendered displacement frame; it proves only command issue and state mutation order.

## 4. Current Rust Status

Focused scan only; no Rust edits.

- `src/sim/miner/miner_system.rs::tick_miners` snapshots miners in deterministic stable-id order and mutates shared dock state during the pass. This can model the binary rule: later processed waiter can see an earlier releaser's contact clear; earlier processed waiter cannot be retroactively promoted.
- `src/sim/miner/miner_dock_sequence.rs::phase_departing` releases contact/pad state and does not call a refinery-side waiter promotion callback.
- `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter` owns retry/admission and distinguishes accepted-cell movement from entered/pad handoff.
- `src/sim/miner/miner_dock.rs::RefineryDockContacts` stores contacts and waiting state; this is a Rust abstraction, but current tests assert B enters only through B's own retry.
- `src/sim/miner/miner_tests.rs` already has targeted tests including `two_miners_waiter_after_releaser_same_tick_claims_on_own_mission_enter`, `two_miners_waiter_before_releaser_not_retroactively_promoted`, and `waiter_moves_from_queueingcell_to_accepted_cell_before_entered`. These mostly exercise the shared refinery path with War Miners; add CMIN-specific fixtures if the chrono return/refinery boundary is touched.

Rust appears aligned for this static ordering slice. The caveat is representation: stable-id order is not gamemd's live-object vector identity/order. The correct parity statement is order-dependent behavior, not "always same tick" or "always next tick."

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| A state-4 healthy zero-link release | verified | `0x0073D630` | none for static order |
| A `BREAK(3)` sender-side clear | verified | `0x0065A970` | none |
| refinery receiver-side `BREAK(3)` clear | verified | `0x0065A820` | none |
| absence of direct waiter promotion | verified | `0x0073D630`; B owner `0x004D9290` | none for static path |
| B `Mission_Enter` retry owner | verified | `0x004D9290` | none |
| mission timer eligibility | verified | `0x005B3060` | concrete replay timer value requires runtime logging |
| live-object vector iteration order | verified | `0x0055AFB0` | concrete replay vector indices require runtime logging |
| accepted cell vs QueueingCell | verified | `0x0043C2D0`, `0x004D8FB0`, `artmd.ini` | none |
| first rendered B displacement | deferred | static code proves command issue, not render pixels | runtime capture |
| current Rust static-order tests | verified by source scan | `src/sim/miner/miner_tests.rs` | run focused tests if implementation touches this area |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does A directly promote a waiting miner? -> No.` (evidence: `0x0073D630`, `0x004D9290`)
- `[RESOLVED] OQ-02 - Does A release the refinery contact synchronously? -> Yes, via outgoing and incoming `BREAK(3)` contact clears.` (evidence: `0x0065A970`, `0x0065A820`)
- `[RESOLVED] OQ-03 - What admits B? -> B's own `FootClass::Mission_Enter` sends `CAN_DOCK(0x0E)`.` (evidence: `0x004D9290`)
- `[RESOLVED] OQ-04 - Can B enter in the same frame? -> Yes only if B is processed later than A and B's mission timer is eligible.` (evidence: `0x0055AFB0`, `0x005B3060`)
- `[RESOLVED] OQ-05 - What if B was processed earlier? -> No retroactive promotion; B waits for a later eligible dispatch.` (evidence: `0x0055AFB0`, no B callback in `0x0073D630`)
- `[RESOLVED] OQ-06 - Does first accepted `CAN_DOCK` from QueueingCell immediately pivot/enter? -> No, not unless already on accepted cell `+(3,1)`.` (evidence: `0x0043C2D0`, `0x004D8FB0`)
- `[RESOLVED] OQ-07 - Does current Rust encode the rule as order-dependent? -> Yes in focused tests and current dock phases.` (evidence: `src/sim/miner/miner_tests.rs`, `miner_dock_sequence.rs`)
- `[DEFERRED] OQ-08 - What are A and B's concrete live-object vector indices in a retail replay?` (category: `needs-runtime-debugger`; reason: static Ghidra proves iteration order but not this replay's object positions; next-step-if-pursued: runtime trace object pointers/indices around `0x0055AFB0`)
- `[DEFERRED] OQ-09 - What is B's concrete mission timer/jitter value at A's release frame?` (category: `needs-runtime-debugger`; reason: static code proves timer gate and retry delay formula but not replay state; next-step-if-pursued: trace `+0xC8/+0xD0` and `Mission_Enter` return for B)
- `[DEFERRED] OQ-10 - What is the first rendered displacement/pixel frame after accepted-cell movement?` (category: `needs-runtime-debugger`; reason: static code proves movement command issue only; next-step-if-pursued: capture B coords/render frame after `0x004D8FB0` case `0x12`)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| A release frees contacts by `BREAK(3)` and never promotes B directly. | `0x0073D630`, `0x0065A970`, `0x0065A820` | none observed | `src/sim/miner/miner_dock_sequence.rs::phase_departing`; `src/sim/miner/miner_dock.rs::RefineryDockContacts` | Release only current contact/pad state; leave B admission to B's own retry. | two full CMINs, A releasing, B already waiting, no callback before B tick | `two_cmin_waiter_before_releaser_not_retroactively_promoted`; do not add refinery FIFO promotion. |
| Same-frame takeover is order/timer dependent. | `0x0055AFB0`, `0x005B3060`, `0x004D9290` | none observed for current tests; runtime vector identity not modeled literally | `src/sim/miner/miner_system.rs::tick_miners`; miner tests | Later eligible waiter may claim in same tick; earlier or timer-blocked waiter must not. | one CMIN test with waiter processed after releaser, one with waiter before releaser | `two_cmin_waiter_after_releaser_same_tick_claims_on_own_mission_enter`; avoid universal same-tick or universal next-tick claims. |
| `CAN_DOCK` from QueueingCell first commands accepted-cell movement; `0x18/0x16` waits for already-there. | `0x0043C2D0`, `0x004D8FB0` | none observed | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`; `src/sim/miner/miner_tests.rs` | Keep QueueingCell `+(4,1)` staging separate from accepted `+(3,1)` and from entered/pad state. | CMIN B at QueueingCell gets movement to accepted cell before entered flag/pad handoff | `two_cmin_waiter_moves_from_queueingcell_to_accepted_cell_before_entered`; do not collapse QueueingCell and accepted cell. |

## 8. Negative Facts / Do Not Do

- Do not implement a refinery-side waiter promotion callback. Evidence: no callback in A state-4 release `0x0073D630`; B retry owner is `0x004D9290`.
- Do not claim same-frame takeover is impossible. Evidence: live-object loop at `0x0055AFB0` can process B after A in the same frame if B's timer is eligible.
- Do not claim same-frame takeover is guaranteed. Evidence: B may already have run, or `MissionClass::Mission_Dispatch @ 0x005B3060` may return before mission id `7`.
- Do not start entered/pivot state from QueueingCell acceptance alone. Evidence: `0x004D8FB0` returns `1` and issues movement unless already at the `0x12` payload cell; `0x0043C2D0` sends `0x18/0x16` only after `0x14`.
- Do not use healthy stock `ReleaseDockedHarvester` / `Force_Track(0x47)` for this zero-link handoff. Evidence: stock state-4 path in `0x0073D630` uses Harvest mission and optional `BREAK(3)` instead.

## 9. Remaining Uncertainty

- Concrete retail replay frame outcome requires runtime logging of A/B live-object vector indices and B's `+0xC8/+0xD0` mission timer values on the release frame.
- First player-visible movement/pixel frame requires runtime capture after B receives radio `0x12`; static Ghidra proves only the command/state order.

## 10. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_FRAME_ORDER_GHIDRA_REPORT.md`: replace partial same-frame wording with: "Static binary evidence proves same-frame admission is conditional on live-object order plus B mission timer eligibility; only concrete replay timer/vector values and first rendered displacement remain runtime-only."
- Any wording that says "the refinery promotes a queued miner" should be replaced with: "A's release frees the contact slot via `BREAK(3)`; the waiting miner is admitted only during that miner's own later `Mission_Enter` / `CAN_DOCK(0x0E)` retry, which may occur in the same frame if object order and mission timer permit."

## Sources

- Ghidra read-only decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Ghidra read-only decompile: `RadioClass::Transmit_Radio_Impl @ 0x0065A970`.
- Ghidra read-only decompile: `RadioClass::Receive_Radio @ 0x0065A820`.
- Ghidra read-only decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`.
- Ghidra read-only decompile: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.
- Ghidra read-only decompile: `FootClass::Mission_Enter @ 0x004D9290`.
- Ghidra read-only decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Ghidra read-only decompile: `FootClass::Receive_Radio @ 0x004D8FB0`.
- Prior reports checked: `miner/MISSION_ENTER_CANDOCK_RETRY_SAME_FRAME_ORDER_GHIDRA_REPORT.md`, `miner/EMPTY_SLOT_UNLOAD_GATE_TO_STATE4_RELEASE_TIMING_GHIDRA_REPORT.md`, `miner/CMIN_CLOSE_FAR_RETURN_SPLIT_CHRONOHARVTOOFARDISTANCE_GHIDRA_REPORT.md`, `miner/REFINERY_SOLD_DESTROYED_MID_UNLOAD_RUNTIME_EFFECTS_GHIDRA_REPORT.md`.
- Rust scanned: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`.

## Status

COMPLETE for static two-CMIN takeover ordering. Runtime logging remains required only for concrete replay object-order/timer values and first rendered movement.
