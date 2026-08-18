# Two Miner One Refinery Zero-Link Handoff Frame Order - Ghidra Research Report

**Address(es):** `0x0073D630`, `0x004D9290`, `0x005B3060`, `0x0055AFB0`, `0x0043C2D0`, `0x004D8FB0`, `0x0065A970`, `0x0065A820`  
**Investigation Mode:** exhaustive-slice, downgraded to PARTIAL for first rendered displacement because static Ghidra cannot prove a concrete replay's object-vector order or locomotor-pixel frame.  
**Claimed Scope:** Normal healthy stock `HARV/CMIN -> GAREFN/NAREFN` contention where miner A completes stock zero-link unload and miner B is already waiting/retrying for the same one-dock refinery.  
**Non-Scope:** generic pathfinding, destroyed/sold refinery, reciprocal `+0x2E4` release, Force_Track visuals, slave miner, service depots, aircraft docks, and non-stock refinery rules.  
**Confidence:** High for static release/admission order; Medium for same-frame admission condition; Low for first rendered displacement without runtime debugger logging.  
**Active in YR:** Yes for stock healthy `HARV`/`CMIN` and `GAREFN`/`NAREFN`; exact same-frame admission is conditional on object order and mission timer eligibility.

## 0. Working Notes Required Before Investigation

**Target question:** In normal two-miner one-refinery stock zero-link unload, exactly when does A clear contact, when can B retry/admit, when is B's first movement command issued, and what should Rust tests assert about handoff order?  
**Non-goals:** Do not rediscover generic pathfinding, destroyed refinery, reciprocal-link `ReleaseDockedHarvester`, Force_Track visuals, or broad miner economics.  
**Evidence needed to mark COMPLETE:** decompile plus assembly-address evidence for A state-4 cleanup, radio contact clearing, B Mission Enter retry, global object-tick order, B `0x12` accepted-cell behavior, and runtime frame evidence for first rendered B displacement.  
**Stop conditions:** Stop if scope leaves stock healthy DockUnload/Refinery contention, if nonzero `unit+0x2E4` is required, if function boundaries require mutation, or if first rendered displacement cannot be proven without runtime observation.

## 1. Overview

Miner A's stock unload completion does not promote miner B directly. A's state-4 handler clears its unload-active byte, sets mission Harvest, then sends radio `BREAK(3)` when the contact-present guard is true; that radio synchronously removes A from both A and refinery contact arrays.

Miner B can only take over when B's own `Mission_Enter` dispatch runs again and sends `CAN_DOCK(0x0E)`. That retry can happen in the same game frame only if B has not yet run in the live-object update pass and its mission timer is eligible; otherwise it waits for B's next eligible Mission Enter dispatch. The first accepted-cell movement command is synchronous with B's accepted `0x12` reply, but the first rendered coordinate delta remains runtime-only.

## 2. Key Offsets / Values

| Field / value | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `unit+0xBC` | `Mission_Deploy_Building` substate; state `4` is stock exit | `0x0073D630` | Yes |
| `unit+0x6D1` | unload-active / initialized byte; A clears before BREAK | `0x0073E1F6` | Yes |
| `unit/building+0xE4/+0xE8` | radio `Contacts[]` and capacity | `0x0065A970`, `0x0065A820` | Yes |
| stock `NumberOfDocks=1` | one refinery contact slot for stock GAREFN/NAREFN | `rulesmd.ini:[GAREFN]/[NAREFN]` | Yes |
| `unit+0x418` | radio entered flag set by `0x18`, cleared by `0x19` cascade | `0x006F4AB0` via prior handoff report | Yes |
| `building+0x57C` | slot-8/ProductionAnim wait guard before state-4 release | `0x0073E1D5..0x0073E1EA` | Conditional; stock normally no active slot-8 wait |
| accepted cell `NW+(3,1)` | B's `CAN_DOCK` target from refinery top-left | `0x0043C2D0` | Yes |
| wait cell `QueueingCell=4,1` | stock art staging/wait point, not the accepted cell | `artmd.ini:[GAREFN]/[NAREFN]`; no read in `0x0043C2D0` | Yes as data; not used by this receiver path |
| mission timer `+0xC8/+0xD0` | dispatch start/duration; gates B retry | `0x005B3060`, `0x004D8FB0` | Yes |

## 3. Verified Frame-Order Chain

Let frame `F` be the frame where miner A's `UnitClass::Mission_Deploy_Building @ 0x0073D630` runs state 4 with `building+0x57C == 0`.

1. A state 4 first re-finds the refinery from A's current cell west-neighbor lookup and checks `Refinery=yes` plus `building+0x57C`. If slot 8 is still non-null, it returns `1`; no unload-active clear and no contact release happen on that frame.  
   **Evidence:** `0x0073E1D5..0x0073E1EA`. **Active in YR:** Conditional; stock healthy refinery usually has no slot-8 wait.

2. On the actual release frame, A clears `unit+0x6D1 = 0` before mission handoff and before radio `BREAK(3)`.  
   **Evidence:** `0x0073E1F6`, followed later by `0x0073E24F..0x0073E279`. **Active in YR:** Yes.

3. A sets mission Harvest (`0x0A`) before sending the break. This means A is logically back in harvest scheduling before the refinery contact slot is cleared.  
   **Evidence:** state-4 normal exit in `0x0073D630`, `0x0073E24F..0x0073E254`. **Active in YR:** Yes.

4. A sends `BREAK(3)` only through the radio/contact path when vtable `+0x200` succeeds and `PathType__Has_Valid_Steps @ 0x0065AE30` reports a non-null contact slot.  
   **Evidence:** `0x0073E25E..0x0073E279`; `0x0065AE30` prior decompile. **Active in YR:** Yes for normal valid-contact unload completion.

5. `RadioClass::Transmit_Radio_Impl @ 0x0065A970` removes the refinery from A's sender-side `Contacts[]` before forwarding BREAK to the refinery. `RadioClass::Receive_Radio @ 0x0065A820` then removes A from the refinery-side `Contacts[]`.  
   **Evidence:** `0x0065A970` BREAK branch, `0x0065A9C9` forward; `0x0065A820` BREAK branch. **Active in YR:** Yes.

6. A's release function contains no scan, callback, or promotion for B. B is admitted only by B's own later `FootClass::Mission_Enter @ 0x004D9290`, which sends `CAN_DOCK(0x0E)` to its destination/refinery.  
   **Evidence:** no B-side callback in `0x0073D630`; `0x004D9290` calls vtable `+0x278` with `0x0E`. **Active in YR:** Yes.

7. `MissionClass::Mission_Dispatch @ 0x005B3060` gates mission execution by `g_CurrentFrameCounter`, `+0xC8`, and `+0xD0`, then calls mission ID `7` through vtable `+0x240`. Therefore B can be admitted on frame `F` only if B's object AI is processed after A in `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` and B's mission timer is due. If B already ran earlier in frame `F`, or its timer is still nonzero, the earliest admission is B's next eligible dispatch.  
   **Evidence:** `0x005B3060`; live-object vector loop at `0x0055B4D7..0x0055B6xx` calls each object's vtable `+0x5C` in vector order. **Active in YR:** Conditional.

8. On B's accepted retry, `BuildingClass::Receive_Radio(0x0E) @ 0x0043C2D0` can first install B into the contact slot via HELLO if the slot is free, then sends `0x12` with refinery top-left `+(3,1)`. If B is not already on that cell, `FootClass::Receive_Radio(0x12) @ 0x004D8FB0` issues `Set_Destination(payload, 1)`, stamps mission timer start to `g_CurrentFrameCounter`, clears duration to `0`, and returns `1`. The refinery therefore does not send `0x18`/`0x16` yet.  
   **Evidence:** `0x0043C2D0` DockUnload case, `0x004D8FB0` case `0x12`. **Active in YR:** Yes.

9. If B is already on the accepted `NW+(3,1)` cell when `0x12` arrives, `FootClass::Receive_Radio(0x12)` returns `0x14`; the refinery then sends `0x18` and `0x16`, moving B into the entered/pivot handshake.  
   **Evidence:** `0x004D8FB0` case `0x12`; `0x0043C2D0` subsequent `0x18`/`0x16` calls. **Active in YR:** Yes.

10. First rendered displacement of B is not proven by static decompile. Static evidence proves the `Set_Destination` command frame, not the later draw-visible coordinate delta.  
    **Evidence:** static Ghidra only; runtime coordinate logging still required. **Active in YR:** Yes for the command; rendered displacement unresolved.

## 4. Current Rust Implementation Status

Relevant Rust surfaces scanned:

- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock.rs`: `RefineryDockContacts` has `contacts`, deterministic `waiting_retry_queue`, `contact_entered`, and `on_pad`. The older `DockReservations` FIFO remains as compatibility/test surface.
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs`: `phase_departing` releases `on_pad` and contact immediately for stock state-4 handoff; `phase_mission_enter` retries admission and gates entry on `pad_clear_or_self`.
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_system.rs`: `tick_miners` snapshots miners sorted by stable_id and processes each snapshot in order, while writes to shared dock reservations are visible during the same processing pass.
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_tests.rs`: useful focused tests exist, especially `occupied_can_dock_defers_without_clearing_waiting_miner_target`, `queued_miner_enters_after_contact_and_pad_are_released`, `empty_unload_gate_releases_dock_on_next_stock_state4_handoff`, and `queued_miner_takes_over_immediately_after_empty_gate_handoff`.

Rust status: coverage is good for contact release and immediate deterministic retry, but not enough for the full binary acceptance surface. The current tests do not cover reversed stable-id/object-order admission, and the handoff tests often place B directly on the accepted cell rather than proving B first receives movement from art wait cell `NW+(4,1)` to accepted cell `NW+(3,1)` before `0x18/0x16`.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| A state-4 slot-8 wait before release | verified | `0x0073E1D5..0x0073E1EA` | none for static order |
| A `+0x6D1` clear before BREAK | verified | `0x0073E1F6`, `0x0073E24F..0x0073E279` | none |
| A mission Harvest before BREAK | verified | `0x0073E24F..0x0073E254` | none |
| A BREAK sender-side contact clear | verified | `0x0065A970` | none |
| refinery receiver-side contact clear | verified | `0x0065A820` | none |
| No release-time promotion of B | verified | `0x0073D630` | none |
| B retry owner is Mission Enter | verified | `0x004D9290`, `0x005B3060` | runtime timer value for exact replay |
| same-frame admission condition | touched-not-exhausted | `0x0055AFB0`, `0x005B3060` | runtime object-vector order |
| B `0x12` accepted-cell command | verified | `0x0043C2D0`, `0x004D8FB0` | none for command issue |
| first rendered B displacement | deferred | static decompile only | runtime coordinate/render frame trace |
| current Rust handoff tests | touched-not-exhausted | Rust scan above | add object-order and wait-cell-to-accepted-cell tests |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does A clear contact in stock zero-link state 4? -> Yes, after `+0x6D1` clear and Harvest mission assignment, by optional radio `BREAK(3)` when a contact is present.` (evidence: `0x0073D630`, `0x0065A970`, `0x0065A820`)
- `[RESOLVED] OQ-2 - Does A promote B directly? -> No; A release has no B callback or queue scan.` (evidence: `0x0073D630`)
- `[RESOLVED] OQ-3 - What admits B? -> B's own `Mission_Enter` dispatch sends `CAN_DOCK(0x0E)` again.` (evidence: `0x004D9290`, `0x005B3060`)
- `[RESOLVED] OQ-4 - Can B admit same frame as A release? -> Conditional: only when B has not yet run in the frame's live-object pass and B's mission timer is due.` (evidence: `0x0055AFB0`, `0x005B3060`)
- `[RESOLVED] OQ-5 - What happens if B is at the wait cell instead of accepted cell? -> B receives `0x12` to `NW+(3,1)`, issues `Set_Destination`, and does not get `0x18/0x16` yet.` (evidence: `0x0043C2D0`, `0x004D8FB0`)
- `[RESOLVED] OQ-6 - Is the stock accepted cell art `QueueingCell=4,1`? -> No; `0x0E` hardcodes top-left `+(3,1)`.` (evidence: `0x0043C2D0`; art INI only for QueueingCell data)
- `[RESOLVED] OQ-7 - Does Rust have enough focused tests for contact release? -> Mostly yes for release/admission pieces, but not for reversed order or wait-cell movement.` (evidence: Rust scan)
- `[DEFERRED] OQ-8 - Which exact concrete replay frame contains B's first rendered displacement?` (category: `needs-runtime-debugger`; reason: requires coordinate logging after `Set_Destination`; next-step-if-pursued: log B coords before/after `0x004D8FB0` and after the next locomotor update)
- `[DEFERRED] OQ-9 - What is B's exact mission timer value in a natural replay immediately before A release?` (category: `needs-runtime-debugger`; reason: dispatch formula is verified but concrete timer/jitter state is runtime; next-step-if-pursued: break at `0x005B3060` for B)
- `[DEFERRED] OQ-10 - Does the draw frame ever show B overlapping A's pad cell during same-frame admission?` (category: `needs-runtime-debugger`; reason: static evidence proves contact availability before physical displacement, not render composition; next-step-if-pursued: collect object cell/coord/render frame trace)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| A's stock state-4 release frees contacts by `BREAK(3)` after `+0x6D1` clear and Harvest mission assignment; it does not promote B. | `0x0073D630`, `0x0065A970`, `0x0065A820` | mostly matched; Rust deterministic queue can look like promotion if not tested carefully | `miner_dock_sequence.rs::phase_departing`, `miner_dock.rs::RefineryDockContacts` | release A contact/pad, but require B to enter through its own retry phase | `test_two_miners_zero_link_release_does_not_directly_promote_waiter` | Do not implement a building-side instant promotion callback. |
| B can admit on the same sim frame only if B is processed after A and its mission timer is due; otherwise next eligible Mission Enter dispatch. | `0x0055AFB0`, `0x005B3060`, `0x004D9290` | Rust processes miner snapshots by stable_id and shared contacts mutate during the pass; reversed-order coverage missing | `miner_system.rs::tick_miners`, `miner_tests.rs` | admission timing should vary with processing order/timer eligibility, not be a fixed `release + 1` rule | `test_two_miners_zero_link_waiter_processed_after_release_can_admit_same_tick`; `test_two_miners_zero_link_waiter_processed_before_release_waits_next_dispatch` | Do not hardcode universal same-tick or next-tick takeover independent of order. |
| B at `QueueingCell=NW+(4,1)` first receives movement to accepted cell `NW+(3,1)`; `0x18/0x16` wait until a later already-there retry. | `0x0043C2D0`, `0x004D8FB0`; art INI `QueueingCell=4,1` | partial: helper exists, but full end-to-end handoff tests place B at accepted cell in several cases | `miner_dock_sequence.rs::phase_mission_enter`, `miner_tests.rs` | issue accepted-cell movement before entered/on-pad/unload when B is still at the wait cell | `test_two_miners_zero_link_waiter_moves_from_queueing_cell_to_accepted_cell_before_entered` | Do not collapse QueueingCell and accepted cell; do not set entered/on-pad on the same `0x12` that first commands movement. |

## 8. Negative Facts / Do Not Do

- Do not use `ReleaseDockedHarvester` or `Force_Track(0x47)` for this stock healthy zero-link handoff.
- Do not model A's state-4 release as a refinery-side FIFO promotion callback.
- Do not allow two stock miners to occupy the refinery `Contacts[]` simultaneously; stock `NumberOfDocks=1` means B waits until A's slot is null.
- Do not treat art `QueueingCell=4,1` as the `CAN_DOCK` accepted cell; accepted cell is hardcoded `NW+(3,1)`.
- Do not claim first rendered displacement or overlap/no-overlap from static decompile alone.

## 9. Remaining Uncertainty

- Runtime object-vector order for a natural two-CMIN replay was not observed in this slot.
- B's exact mission timer/jitter state at A's release frame was not observed.
- First rendered B displacement after `Set_Destination` remains runtime-only.

## 10. Stale Docs / Follow-up Wording

- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md`: replace any refinery queue/promoter wording with: "Stock refinery zero-link release frees RadioClass contacts via `BREAK(3)`; the next miner is admitted only when that miner's own `Mission_Enter` retry sends `CAN_DOCK(0x0E)`. No separate release-time refinery promotion callback is verified."
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/TWO_CHRONO_MINERS_SAME_REFINERY_FULL_CARGO_QUEUE_TAKEOVER_TRACE.md`: replace unchecked takeover wording with: "B's first accepted-cell movement is commanded on B's first eligible `Mission_Enter` dispatch after A's `BREAK(3)` contact release; this can be the same frame only if B has not yet run in the live-object pass and its mission timer is due."
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/HARVESTER_DOCK_UNLOAD.md` and `C:/Users/enok/Documents/ra2-rust-game-docs/miner/HARVESTER_DOCK_UNLOAD_SEQUENCE.md`: replace normal stock unload release wording with: "Normal stock `CMIN/HARV` refinery unload completes through zero-link `UnitClass::Mission_Deploy_Building` state 4; `ReleaseDockedHarvester` is for nonzero-link/interrupt contexts, not this healthy handoff."

## Sources

- Ghidra read-only decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Ghidra read-only decompile: `FootClass::Mission_Enter @ 0x004D9290`.
- Ghidra read-only decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`.
- Ghidra read-only decompile: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.
- Ghidra read-only decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Ghidra read-only decompile: `FootClass::Receive_Radio @ 0x004D8FB0`.
- Ghidra read-only decompile: `RadioClass::Transmit_Radio_Impl @ 0x0065A970`.
- Ghidra read-only decompile: `RadioClass::Receive_Radio @ 0x0065A820`.
- Prior reports: `TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_TIMING_GHIDRA_REPORT.md`, `TWO_CMIN_ONE_REFINERY_TAKEOVER_TIMING_GHIDRA_REPORT.md`, `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`, `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/art.ini`.
- Rust scanned: `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_tests.rs`.

## Status

PARTIAL: static binary evidence proves contact clearing, B retry ownership, same-frame admission preconditions, and B's first accepted-cell command, but exact first rendered displacement and concrete natural replay order require runtime debugger/frame logging.
