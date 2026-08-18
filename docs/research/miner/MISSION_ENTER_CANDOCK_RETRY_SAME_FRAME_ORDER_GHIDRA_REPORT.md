# Mission_Enter CAN_DOCK Retry Same-Frame Order - Ghidra Research Report

**Address(es):** `0x004D9290`, `0x005B3060`, `0x0055AFB0`, `0x0043C2D0`, `0x004D8FB0`, `0x0073D630`, `0x0065A970`, `0x0065A820`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard YR `HARV/CMIN -> GAREFN/NAREFN` stock healthy zero-link contention where miner A finishes refinery unload and miner B is waiting on the same one-dock refinery; exact `Mission_Enter` / `CAN_DOCK(0x0E)` retry ordering, same-frame eligibility, object iteration dependency, mission timer dependency, contacts, and whether the refinery directly promotes waiters.  
**Non-Scope:** first rendered pixel displacement, destroyed/sold refinery interruption, nonzero reciprocal `+0x2E4` release, service depots, slave miner, and non-stock multi-dock refinery rules.  
**Confidence:** High for static binary order and same-frame eligibility rule; Medium for any concrete replay's observed same-frame outcome because that depends on live-object vector order and runtime mission timer state.  
**Active in YR:** Yes. Stock `[CMIN]`/`[HARV]` dock at `[GAREFN]`/`[NAREFN]`, which have `DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1` in `rulesmd.ini`.

## 1. Overview

The refinery does not directly promote a waiting miner when the current miner releases. Miner A's state-4 unload exit clears its own unload-active byte, sets Harvest mission, and sends radio `BREAK(3)` if a contact is present; that synchronous radio clears both A's sender contact and the refinery-side contact slot.

Miner B can be admitted only when B's own `FootClass::Mission_Enter` runs and sends `CAN_DOCK(0x0E)` to the refinery. Therefore same-frame admission is conditional: it can happen in the same frame as A's release only if B is processed later in the live-object update pass and B's mission timer is eligible for dispatch on that frame. If B already ran earlier, or its timer has not expired, it waits until its next eligible `Mission_Enter` dispatch.

## 2. Class Layout / Key Offsets

| Offset / value | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| unit `+0xBC` | `Mission_Deploy_Building` substate; state `4` is stock zero-link release | `0x0073D630` | Yes |
| unit byte `+0x6D1` | unload-active/init byte cleared before contact release | `0x0073E1F6` | Yes |
| unit/building `+0xE4` | `RadioClass` Contacts[] pointer | `0x0065A970`, `0x0065A820` | Yes |
| unit/building `+0xE8` | Contacts[] capacity | `0x0065A970`, `0x0065A820`; stock `NumberOfDocks=1` | Yes |
| unit byte `+0x418` | entered-dock flag set by radio `0x18`; not a release promoter | prior dock-arrival report, `0x006F4AB0` | Yes |
| MissionClass `+0xC8` | mission timer start frame, `-1` special value | `0x005B3060` | Yes |
| MissionClass `+0xD0` | mission timer duration / remaining gate | `0x005B3060` | Yes |
| mission id `7` | `Mission_Enter`, dispatched through vtable `+0x240` | `0x005B3060` | Yes |
| accepted cell | refinery NW cell plus `(3,1)` | `0x0043C2D0` case `0x0E` | Yes |
| radio `0x12` reply | "move/need-to-move" payload; returns `0x14` only when already at cell | `0x004D8FB0` | Yes |

## 3. Core Logic

### 3.1 A releases the contact synchronously, but does not call B

In `UnitClass::Mission_Deploy_Building @ 0x0073D630`, state 4 first re-finds the adjacent refinery and checks the `Refinery=yes` type flag plus `building+0x57C`. If slot 8 is non-null, it returns `1` and does not release that frame.

On the actual release path:

1. `unit+0x6D1 = 0` at `0x0073E1F6`.
2. If the normal zero-link branch applies, the unit sets mission `0x0A` / Harvest at `0x0073E24F..0x0073E254`.
3. It checks whether a radio contact exists (`vtable +0x200`, then `PathType__Has_Valid_Steps @ 0x0065AE30`).
4. It sends radio `BREAK(3)` at `0x0073E275..0x0073E279` only if that contact check succeeds.
5. It finalizes radio cleanup through `vtable +0x1EC` at `0x0073E27F..0x0073E283`.

`RadioClass::Transmit_Radio_Impl @ 0x0065A970` handles outgoing `BREAK(3)` by scanning the sender's Contacts[] and nulling every slot equal to the target before forwarding to the target receiver. `RadioClass::Receive_Radio @ 0x0065A820` handles incoming `BREAK(3)` by scanning the refinery Contacts[] for the sender, calling `ObjectClass::Receive_Radio`, nulling that slot, and returning `1`.

No decompiled branch in A's state-4 release scans waiting miners, calls a queue callback, or invokes another miner's `Mission_Enter`.

### 3.2 B's retry owner is `Mission_Enter`

`FootClass::Mission_Enter @ 0x004D9290` gets the destination/refinery and then calls the target via vtable `+0x278` with message `0x0E`:

```text
target = FootClass::GetDestination()
if target exists:
    reply = this->Transmit_Radio(0x0E, target)
```

If the reply is `ROGER(1)`, the function continues the enter path. If the reply is not `1` and the unit is not already in the alternate entered state, it sends `BREAK(3)` and clears its destination. This means a waiting miner can only retry when its own mission dispatch reaches `Mission_Enter`.

### 3.3 Mission timer gate controls whether B runs on the release frame

`MissionClass::Mission_Dispatch @ 0x005B3060` gates every mission call:

1. It calls `ObjectClass::AI`.
2. It returns if the object is not active (`byte +0x90 == 0`).
3. It reads timer start `+0xC8` and duration `+0xD0`.
4. If start is not `-1`, it compares `g_CurrentFrameCounter - start` against duration.
5. If remaining duration is nonzero, it returns without calling the mission.
6. If eligible, mission id `7` calls vtable `+0x240`, then writes `+0xC8 = g_CurrentFrameCounter` and `+0xD0 = returned delay`.

`FootClass::Mission_Enter` returns `MissionClass::GetMissionTimerEntry()` converted by `Math__ftol()` plus `Random__RandomRanged(0,2)`. The exact next retry delay is therefore not a fixed one-tick rule; it is whatever the mission timer entry produces plus a random `0..2` jitter.

### 3.4 Live-object update order decides same-frame eligibility

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` updates the main live object vector by increasing index:

```text
ESI = 0
while ESI < live_count:
    object = live_vector[ESI]
    object->AI()        // vtable +0x5C
    ESI += 1
```

Assembly context at `0x0055B608..0x0055B619` shows it reading `live_vector[ESI]`, calling `vtable +0x5C`, incrementing `ESI`, and looping while `ESI < count`.

Consequences for frame `F`, where A releases:

- If A's object AI runs before B's object AI in frame `F`, A's `BREAK(3)` clears the refinery contact before B reaches `Mission_Enter`. B can be admitted in frame `F` if B's mission timer is due.
- If B's object AI already ran earlier in frame `F`, A's later contact clear does not retroactively call B. B waits until B's next eligible object AI / `Mission_Enter` dispatch.
- If B is later in object order but its mission timer has remaining duration, B also waits.

### 3.5 `CAN_DOCK(0x0E)` does not equal final dock entry

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` is the receiver-side admission path. For stock `DockUnload=yes` / `Refinery=yes` refineries:

1. It calls base `TechnoClass::Receive_Radio`.
2. If unpowered, it returns `10`.
3. It may install the sender into Contacts[] by sending HELLO (`0x02`) when a free slot exists.
4. It sends `0x12` with the hardcoded accepted cell: building NW cell plus `(3,1)`.
5. Only if the unit replies `0x14` already-there does the refinery send `0x18` and then `0x16`.

`FootClass::Receive_Radio @ 0x004D8FB0` case `0x12` returns `0x14` only when the unit's current packed cell equals the payload cell. Otherwise it issues `Set_Destination(payload, 1)`, writes mission timer start `+0xC8 = g_CurrentFrameCounter`, clears duration `+0xD0 = 0`, and returns `1`.

Therefore a waiting miner at art `QueueingCell=NW+(4,1)` can claim the contact on its `Mission_Enter` retry, but it first receives movement to the accepted cell `NW+(3,1)` and does not yet receive the `0x18`/`0x16` entered handshake.

## 4. INI Keys

| INI key | Stock value | Effect in this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[CMIN] Dock` | `NAREFN,GAREFN` | CMIN targets stock refineries | Yes |
| `rulesmd.ini:[CMIN] Harvester` | `yes` | CMIN uses harvester/refinery logic | Yes |
| `rulesmd.ini:[HARV] Dock` | `NAREFN,GAREFN` | War Miner targets stock refineries | Yes |
| `rulesmd.ini:[HARV] Harvester` | `yes` | War Miner uses harvester/refinery logic | Yes |
| `rulesmd.ini:[GAREFN] DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | one-contact stock Allied refinery | Yes |
| `rulesmd.ini:[NAREFN] DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | one-contact stock Soviet refinery | Yes |
| `artmd.ini:[GAREFN]/[NAREFN] QueueingCell` | `4,1` | waiting/staging cell; not the `CAN_DOCK` accepted cell | Yes as data |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | A's stock zero-link unload state 4 cleanup and `BREAK(3)` | decompiled, assembly `0x0073E1F6`, `0x0073E24F`, `0x0073E279` | Yes |
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970` | sender-side `BREAK(3)` contact clear before forwarding | decompiled | Yes |
| `RadioClass::Receive_Radio @ 0x0065A820` | receiver-side `BREAK(3)` contact clear | decompiled | Yes |
| `FootClass::Mission_Enter @ 0x004D9290` | B retry owner; sends `CAN_DOCK(0x0E)` | decompiled | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | mission timer eligibility and mission id dispatch | decompiled, assembly `0x005B307A..0x005B30C1` | Yes |
| `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` | object vector order; increasing-index vtable `+0x5C` calls | decompiled, assembly `0x0055B608..0x0055B619` | Yes |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | stock refinery `0x0E` contact/accepted-cell/entered handshake | decompiled | Yes |
| `FootClass::Receive_Radio @ 0x004D8FB0` | `0x12` already-there vs movement command | decompiled | Yes |

## 6. Current Rust Implementation Status

Current Rust surfaces scanned:

- `src/sim/miner/miner_system.rs`: `tick_miners` snapshots miners sorted by stable id and processes each snapshot in order. Shared dock state mutates during this pass, so a later waiter can observe an earlier releaser's contact clear in the same Rust tick.
- `src/sim/miner/miner_dock.rs`: `RefineryDockContacts` stores contacts, a deterministic waiting retry queue, contact-entered state, and pad occupancy. `hello_or_wait` accepts only when capacity is free and the waiting miner is at the queue front.
- `src/sim/miner/miner_dock_sequence.rs`: `phase_departing` releases pad/contact and does not call a promotion callback; `phase_mission_enter` owns retry/admission and distinguishes accepted-cell movement from entered/pad handoff.
- `src/sim/miner/miner_tests.rs`: current tests now cover waiter-after-releaser same-tick admission, waiter-before-releaser non-promotion, approach HELLO-only behavior, and QueueingCell-to-accepted-cell movement before entered state.

Rust appears to match the core static ordering better than the earlier partial report described. Remaining implementation caution: Rust uses stable-id order, not gamemd's live-object vector identity/order. That is deterministic and acceptable internally, but parity tests should phrase expectations as order-dependent rather than universal same-tick or universal next-tick takeover.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| A state-4 slot-8 wait before release | verified | `0x0073E1D5..0x0073E1EA` | exact stock art slot-8 liveness covered by separate reports |
| A `+0x6D1` clear before contact release | verified | `0x0073E1F6` | none |
| A Harvest mission assignment before `BREAK(3)` | verified | `0x0073E24F..0x0073E254` | none |
| A sender-side `BREAK(3)` contact clear | verified | `0x0065A970` | none |
| refinery receiver-side `BREAK(3)` contact clear | verified | `0x0065A820` | none |
| no direct refinery promotion callback | verified | `0x0073D630`; B retry path at `0x004D9290` | none for static path |
| B `Mission_Enter` retry path | verified | `0x004D9290` | none |
| mission timer eligibility | verified | `0x005B3060` | concrete replay timer values require runtime logging |
| live-object vector order | verified | `0x0055AFB0`; assembly `0x0055B608..0x0055B619` | concrete object order in a replay requires runtime logging |
| B accepted-cell command before entered state | verified | `0x0043C2D0`, `0x004D8FB0` | none |
| first rendered displacement | deferred | static Ghidra cannot prove rendered pixel/frame | runtime capture if needed |
| current Rust handoff tests | verified by source scan | `miner_tests.rs` focused tests | run focused tests in implementation pass |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-001 - Does A's zero-link state-4 release directly promote a waiting miner? -> No. It clears its own contact and sends `BREAK(3)`; no B scan/callback exists in the release path.` (evidence: `0x0073D630`, `0x0065A970`, `0x0065A820`)
- `[RESOLVED] OQ-002 - When is the refinery contact slot cleared? -> After `+0x6D1` clear and Harvest mission assignment, synchronously during outgoing/incoming `BREAK(3)`.` (evidence: `0x0073E1F6`, `0x0073E24F`, `0x0073E279`, `0x0065A970`, `0x0065A820`)
- `[RESOLVED] OQ-003 - What admits B after A releases? -> B's own `FootClass::Mission_Enter` sends `CAN_DOCK(0x0E)`.` (evidence: `0x004D9290`)
- `[RESOLVED] OQ-004 - Can B be admitted in the same frame as A release? -> Yes, conditionally: B must run later in the live-object vector on that frame and its mission timer must be due.` (evidence: `0x0055AFB0`, `0x005B3060`)
- `[RESOLVED] OQ-005 - What if B already ran earlier in the frame? -> No retroactive promotion occurs; earliest retry is B's next eligible dispatch.` (evidence: increasing-index live-object loop `0x0055B608..0x0055B619`; no callback in `0x0073D630`)
- `[RESOLVED] OQ-006 - What if B's timer is not due? -> `MissionClass::Mission_Dispatch` returns before mission id `7`, so B waits even if the contact slot is free.` (evidence: `0x005B3060`)
- `[RESOLVED] OQ-007 - Does accepted `CAN_DOCK` immediately start entered/pivot state? -> Only if `0x12` returns `0x14` already-there; otherwise it only commands movement to `NW+(3,1)`.` (evidence: `0x0043C2D0`, `0x004D8FB0`)
- `[RESOLVED] OQ-008 - Is `QueueingCell=4,1` the accepted cell? -> No. The accepted cell is hardcoded refinery NW plus `(3,1)`.` (evidence: `0x0043C2D0`; `artmd.ini` QueueingCell data)
- `[RESOLVED] OQ-009 - Does stock incoming HELLO evict the current refinery contact? -> No. Receiver-side full Contacts[] returns `10`; sender-side HELLO eviction is a different path on the sender's own contact list.` (evidence: `0x0065A820`, `0x0065A970`)
- `[RESOLVED] OQ-010 - Does Rust currently model this as a direct promotion callback? -> No in current scanned code; release mutates contacts, retry occurs in `phase_mission_enter`, and tests cover both object-order directions.` (evidence: Rust scan paths in Section 6)
- `[DEFERRED] OQ-011 - What exact frame/pixel is B's first rendered displacement in a natural retail replay?` (category: `needs-runtime-debugger`; reason: static decompile proves command issue timing, not rendered coordinates; next-step-if-pursued: log B object coords before/after `0x004D8FB0` and render frame)
- `[DEFERRED] OQ-012 - What is B's natural mission timer value in a specific replay just before A releases?` (category: `needs-runtime-debugger`; reason: the timer formula is verified but concrete jitter/runtime value is replay-specific; next-step-if-pursued: break at `0x005B3060` for B)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| A release frees contacts by `BREAK(3)` and never promotes B directly. | `0x0073D630`, `0x0065A970`, `0x0065A820` | mostly matched | `src/sim/miner/miner_dock_sequence.rs::phase_departing`; `src/sim/miner/miner_dock.rs::RefineryDockContacts` | release A contact/pad, but leave B admission to B's own retry phase | `two_miners_waiter_before_releaser_not_retroactively_promoted` | Do not add a building/refinery FIFO promotion callback. |
| Same-frame takeover is order/timer dependent, not universal. | `0x0055AFB0`, `0x005B3060`, `0x004D9290` | mostly matched through stable-id order tests; timer jitter is abstracted | `src/sim/miner/miner_system.rs::tick_miners`; miner tests | if waiter is processed after release and eligible, same-tick claim may happen; if processed before release, wait until next retry | `two_miners_waiter_after_releaser_same_tick_claims_on_own_mission_enter`; `two_miners_waiter_before_releaser_not_retroactively_promoted` | Do not hardcode universal same-tick or universal next-tick takeover. |
| `CAN_DOCK` from QueueingCell first commands movement to accepted cell; entered/pivot waits for already-there retry. | `0x0043C2D0`, `0x004D8FB0` | matched by current tests | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`; miner tests | keep QueueingCell staging distinct from accepted cell and entered state | `waiter_moves_from_queueingcell_to_accepted_cell_before_entered` | Do not collapse `QueueingCell=NW+(4,1)` and accepted cell `NW+(3,1)`. |

## 10. Negative Facts / Do Not Do

- Do not implement a refinery-side "promote next waiter" callback on A release.
- Do not treat same-frame admission as impossible; it is possible when B runs later and is timer-eligible.
- Do not treat same-frame admission as guaranteed; B may already have run or may be mission-timer gated.
- Do not evict A from the receiver-side busy refinery contact list when B says HELLO; incoming full Contacts[] returns `10`.
- Do not start `0x18`/`0x16` entered state on the first `0x12` reply unless B is already on the accepted cell.
- Do not use `ReleaseDockedHarvester` / `Force_Track(0x47)` for this stock healthy zero-link handoff.

## 11. Remaining Uncertainty

- A concrete retail replay's actual same-frame outcome still requires runtime logging of live-object vector order and B's mission timer state. The static rule is resolved; the per-replay instance is not.
- First rendered displacement / pixel overlap remains outside this report because static Ghidra proves command ordering, not render-frame coordinates.

## 12. Stale Docs / Follow-up Docs

- `TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_FRAME_ORDER_GHIDRA_REPORT.md` can be upgraded from "PARTIAL for same-frame admission condition" to: "Static binary evidence proves same-frame admission is conditional on live-object order plus B mission timer eligibility; only concrete replay timer/vector values and first rendered displacement remain runtime-only."
- Any wording that says "queued miner is promoted by the refinery" should be replaced with: "A's release frees the contact slot via `BREAK(3)`; the waiting miner is admitted only during that miner's own later `Mission_Enter` / `CAN_DOCK(0x0E)` retry, which may occur in the same frame if object order and mission timer permit."

## Sources

- Ghidra read-only decompile: `FootClass::Mission_Enter @ 0x004D9290`.
- Ghidra read-only decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`.
- Ghidra read-only decompile: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.
- Ghidra read-only decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Ghidra read-only decompile: `FootClass::Receive_Radio @ 0x004D8FB0`.
- Ghidra read-only decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Ghidra read-only decompile: `RadioClass::Transmit_Radio_Impl @ 0x0065A970`.
- Ghidra read-only decompile: `RadioClass::Receive_Radio @ 0x0065A820`.
- Ghidra assembly context: `0x0055B608..0x0055B619`, `0x005B307A..0x005B30C1`, `0x0073E1F6`, `0x0073E24F`, `0x0073E279`.
- Prior docs: `BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`, `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`, `TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_FRAME_ORDER_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `rules.ini`, `artmd.ini`, `art.ini`.
- Rust scanned: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`.

## Status

COMPLETE for the static `Mission_Enter` / `CAN_DOCK` retry ordering slice. Runtime logging is still required only for a concrete replay's object-vector/timer values and first rendered displacement.
