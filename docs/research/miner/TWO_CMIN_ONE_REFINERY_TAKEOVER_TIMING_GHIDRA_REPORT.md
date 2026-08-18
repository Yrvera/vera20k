# Two CMIN One Refinery Takeover Timing - Ghidra Research Report

**Address(es):** `0x0073D630`, `0x004D9290`, `0x0043C2D0`, `0x004D8FB0`, `0x0065A970`, `0x0065A820`, `0x005B3060`, `0x0055AFB0`  
**Investigation Mode:** exhaustive-slice downgraded to partial for pixel-motion frame, because static Ghidra verifies the radio/mission ordering but not the first rendered locomotor displacement without runtime observation.  
**Claimed Scope:** two standard full `CMIN` units targeting one stock `GAREFN`/`NAREFN`, miner A completing standard zero-link unload state 4 while miner B is waiting/retrying, through B's next `CAN_DOCK` admission and accepted-cell movement command.  
**Non-Scope:** full radio protocol, far-return selection, ore economics before the cargo-empty gate, `ReleaseDockedHarvester`, `Force_Track(0x47)`, non-stock refineries, non-CMIN harvesters except where the shared HARV/CMIN path proves activity.  
**Confidence:** High for contact release/admission command ordering; Medium for same-frame vs next-frame admission because it depends on object-vector order and Mission Enter timer eligibility; Low for first rendered pixel/cell displacement without runtime debugger.  
**Active in YR:** Yes for standard YR CMIN/HARV -> stock GAREFN/NAREFN zero-link refinery docking.

## 1. Target Question

When miner A finishes standard zero-link unload and miner B is queued/waiting for the same one-dock refinery, what exact tick/frame order frees the contact/pad, lets B retry/admit, and starts B's movement from wait cell `(NW+4,NW+1)` to accepted/pad cell `(NW+3,NW+1)`?

## 2. Non-Goals

- Do not re-prove zero-link vs `ReleaseDockedHarvester`.
- Do not re-prove far staging or `QueueingCell=4,1` except as the wait cell.
- Do not implement or edit Rust.
- Do not claim first rendered pixel movement without runtime locomotor instrumentation.

## 3. Evidence Needed to Mark COMPLETE

Static binary evidence is enough to mark the radio/mission ordering complete. Runtime debugger evidence is still needed for a fully complete frame trace: log `g_CurrentFrameCounter`, object-vector index/order, A state-4 dispatch, A radio `3`, B `Mission_Enter`, B `0x12`, and B locomotor first coordinate delta in one replay.

## 4. Stop Conditions

Stop if the live path leaves standard `DockUnload=yes`/`Refinery=yes` GAREFN/NAREFN, if `unit+0x2E4` becomes nonzero, if B is not in mission `7`/Mission Enter with the refinery contact or fallback target, or if function boundaries are missing and mutation would be required. No such blocking mutation was needed.

## 5. Verified Tick / Frame Sequence

Let frame `F` be the frame where miner A's `UnitClass::Mission_Deploy_Building @ 0x0073D630` runs state 4 and `building+0x57C == 0`.

1. A state 4 re-finds the refinery from A's current cell plus `g_refinery_unload_adjacent_lookup_dx`. It first checks `Refinery=yes` and `building+0x57C`; if slot 8 is nonzero it returns immediately, so no release occurs on that frame.  
   **Active in YR:** Yes. Evidence: `0x0073D630`, state-4 branch; stock GAREFN/NAREFN are `Refinery=yes`. Conditional delay only if `building+0x57C != 0`.

2. On the release frame, before any BREAK, A clears byte `unit+0x6D1 = 0`. This is A's unload-active flag, not the refinery contact slot.  
   **Active in YR:** Yes. Evidence: `Mission_Deploy_Building @ 0x0073D630`, state 4.

3. A sets its mission to Harvest (`0x0A`, decimal `10`) through vtable `+0x1E8` before sending radio `3`.  
   **Active in YR:** Yes. Evidence: `0x0073D630` state-4 normal exit sequence.

4. If the normal path/contact is still valid, A sends radio `3` through vtable `+0x274`. `RadioClass::Transmit_Radio_Impl @ 0x0065A970` removes the target refinery from A's `Contacts[]` first, then forwards BREAK to the refinery. `RadioClass::Receive_Radio @ 0x0065A820` on the refinery removes A from the refinery `Contacts[]`.  
   **Active in YR:** Yes for normal stock release with valid radio/path state. Evidence: `0x0073D630`, `0x0065A970`, `0x0065A820`.

5. There is no immediate building-side promotion or FIFO callback for B in A's state-4 code. A's release only frees the radio contact; B must run its own `FootClass::Mission_Enter @ 0x004D9290` again and send `CAN_DOCK(0x0E)`.  
   **Active in YR:** Yes. Evidence: no B iteration/callback in `0x0073D630`; `0x004D9290` sends `0x0E` on each Mission Enter dispatch.

6. B retries on its own mission dispatch. `MissionClass::Mission_Dispatch @ 0x005B3060` calls mission ID `7` through vtable `+0x240`, then stores the returned delay into `+0xD0`. `FootClass::Mission_Enter` returns `ftol(MissionTimerEntry[current]+rate * 900.0) + RandomRanged(0,2)`.  
   **Active in YR:** Yes. Evidence: `0x005B3060`, `0x004D9290`, `0x005B3A00`.

7. Therefore B can be admitted on frame `F` only if B's object AI has not yet run for frame `F` and B's Mission Enter timer is eligible when its turn arrives. If B already ran earlier in the same object-vector pass, or its timer is not eligible, the earliest admission is B's next eligible Mission Enter dispatch.  
   **Active in YR:** Conditional. Evidence: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` iterates live objects in vector order and calls vtable `+0x5C`; `Mission_Dispatch` is per-object timer-gated.

8. On B's admitted retry, the refinery's `BuildingClass::Receive_Radio @ 0x0043C2D0` has an empty contact slot, sends HELLO/contact if needed, then sends `0x13`, then computes the accepted DockUnload payload as refinery NW `+(3,1)`, not art `QueueingCell`.  
   **Active in YR:** Yes. Evidence: `0x0043C2D0`; stock GAREFN/NAREFN `DockUnload=yes`, `NumberOfDocks=1`.

9. If B is at the wait cell `(NW+4,NW+1)`, `FootClass::Receive_Radio @ 0x004D8FB0` case `0x12` compares B's current cell against the payload cell `(NW+3,NW+1)`, sees it is not already there, calls `Set_Destination(payload, 1)`, writes mission timer start to `g_CurrentFrameCounter`, clears mission timer duration to `0`, and returns `1`. The refinery does not send `0x18`/`0x16` yet.  
   **Active in YR:** Yes. Evidence: `0x004D8FB0` case `0x12`; `0x0043C2D0` only sends `0x18/0x16` when `0x12` returns `0x14`.

10. The first accepted-cell movement command is synchronous with B's `0x12` handling on B's admitted retry frame. The first rendered locomotor displacement is not statically proven; it should be measured as the first coordinate delta after that `Set_Destination`.  
    **Active in YR:** Yes for command; runtime needed for rendered displacement. Evidence: `0x004D8FB0`, `TechnoClass::Set_Destination @ 0x00741970`.

## 6. Key Offsets / Fields

| Field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| Unit `+0x6D1` | unload FSM initialized/active byte; cleared by A before release radio | `0x0073D630` | Yes |
| Building `+0x57C` | slot-8 ProductionAnim pointer; state-4 delay guard | `0x0073D630`; `BUILDINGCLASS_0X57C...` | Conditional delay; stock usually zero |
| Radio `Contacts[] +0xE4/+0xE8` | contact roster and capacity; one slot for stock refineries | `0x0065A970`, `0x0065A820`; `NumberOfDocks=1` | Yes |
| Unit `+0x418` | dock-entered flag set by `0x18`; preserves Mission Enter on non-ROGER | `0x006F4AB0`, `0x004D9290` | Yes |
| Unit `+0xC8/+0xD0` | mission timer start/duration; `0x12` resets for movement retry | `0x004D8FB0`, `0x005B3060` | Yes |
| BuildingType `+0x16B3/+0x16BB` | `DockUnload=yes` / `Refinery=yes` | `0x0043C2D0`, `0x0073D630`; INI | Yes |

## 7. Current Rust Implementation Status

Current Rust has relevant surfaces in `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, and tests in `src/sim/miner/miner_tests.rs`. The model has explicit `contacts`, `waiting_retry_queue`, `contact_entered`, and `on_pad`. Binary evidence does not prove a binary FIFO callback; the Rust FIFO is an implementation device for deterministic retry, and must behave like B's own timer/order-gated Mission Enter retry.

Existing tests cover pieces such as `queued_miner_enters_after_contact_and_pad_are_released`, but the full two-CMIN, full-cargo, one-refinery release-to-first-accepted-cell-move frame trace remains the needed acceptance test.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| A zero-link state-4 release | verified | `0x0073D630` | none for release order |
| A contact freeing via BREAK(3) | verified | `0x0073D630`, `0x0065A970`, `0x0065A820` | none |
| Absence of automatic B promotion in A release | verified | `0x0073D630` | none |
| B retry ownership by Mission Enter | verified | `0x004D9290`, `0x005B3060` | runtime mission timer value |
| Same-frame vs next-frame condition | touched-not-exhausted | `0x0055AFB0`, `0x005B3060` | runtime object vector order for concrete replay |
| B accepted-cell command from wait cell | verified | `0x0043C2D0`, `0x004D8FB0` | none for command issue |
| First rendered locomotor displacement | deferred | static decompile only | runtime coordinate logging |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does A free the contact in state 4? -> Yes, normal state 4 clears `+0x6D1`, sets mission Harvest, then sends BREAK(3) when the radio/path gate is valid; BREAK removes both sender and receiver contact slots.` (evidence: `0x0073D630`, `0x0065A970`, `0x0065A820`)
- `[RESOLVED] OQ-2 - Does A directly admit/promote B? -> No; A state 4 has no B callback or queue scan.` (evidence: `0x0073D630`)
- `[RESOLVED] OQ-3 - What admits B? -> B's own Mission Enter dispatch sends `CAN_DOCK(0x0E)` again.` (evidence: `0x004D9290`, `0x005B3060`)
- `[RESOLVED] OQ-4 - Can B be admitted on the same frame as A release? -> Conditional: yes only if B's object update is later in the same live-vector pass and B's mission timer is eligible; otherwise next eligible Mission Enter dispatch.` (evidence: `0x0055AFB0`, `0x005B3060`)
- `[RESOLVED] OQ-5 - What is B's first accepted movement command? -> refinery `0x0E` sends `0x12` with NW+(3,1); B at NW+(4,1) calls `Set_Destination` to that cell and returns `1`, without `0x18/0x16`.` (evidence: `0x0043C2D0`, `0x004D8FB0`)
- `[DEFERRED] OQ-6 - Which exact rendered frame contains B's first pixel/cell delta?` (category: `needs-runtime-debugger`; reason: locomotor update after `Set_Destination` needs runtime frame logging; next-step-if-pursued: log B coordinates before/after `0x004D8FB0` and subsequent locomotor update)
- `[DEFERRED] OQ-7 - Exact mission timer table value for mission 7 in this scenario.` (category: `needs-runtime-debugger`; reason: formula and jitter are verified, but table value was not read from a running process; next-step-if-pursued: break after `0x005B3A00` during B Mission Enter)

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| A's zero-link state-4 frees contacts only by BREAK(3), after clearing `+0x6D1` and setting mission Harvest; it does not promote B. | `0x0073D630`, `0x0065A970`, `0x0065A820` | possible mismatch if Rust release immediately promotes FIFO in the same A phase | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_dock.rs` | Release contact/pad, but require B to retry through its own dock-enter phase before accepted-cell movement/unload. | `test_two_chrono_miners_one_refinery_waiter_enters_after_zero_link_release` | Do not implement a building-side instant promotion callback on A release. |
| B's takeover timing is object-order/timer-gated, not a universal same-tick event. | `0x0055AFB0`, `0x005B3060`, `0x004D9290` | unchecked against Rust tick loop, which may process miners in deterministic stable-id order every tick | `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs` | If B is processed later and eligible, admission may occur same sim tick; if already processed, wait until next retry tick. | Same setup with A stable-id lower than B and reversed order should show different admission frame if Rust models object order. | Do not assert a fixed `release_tick + 1` or same-tick takeover independent of update order. |
| B at `QueueingCell=4,1` receives accepted movement to NW+(3,1); `0x18/0x16` wait until B is already there. | `0x0043C2D0`, `0x004D8FB0` | mostly matched by separate wait/accepted cells; full end-to-end test missing | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs` | On admitted retry, issue accepted-cell move first; only a later already-there retry sets entered/on-pad/unload. | `test_two_chrono_miners_one_refinery_waiter_enters_after_zero_link_release` asserts B movement target `(13,11)` from `(14,11)` before entered/on-pad. | Do not collapse wait cell and accepted pad cell; do not set entered/on-pad on the same `0x12` that first commands movement from `(14,11)`. |

## 11. Negative Facts / Do Not Do

- Do not use `ReleaseDockedHarvester` or `Force_Track(0x47)` for this stock zero-link release.
- Do not model A's release as directly assigning B to the dock; B must retry through Mission Enter.
- Do not treat art `QueueingCell=4,1` as the accepted `CAN_DOCK` cell; accepted cell is hardcoded NW+(3,1).
- Do not send `0x18/0x16` while B is still at `(NW+4,NW+1)`; `0x12` must return `0x14` first.
- Do not claim exact first rendered movement frame from static decompile alone.

## 12. Stale Docs / Follow-up Wording

- `miner/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md`: replace any remaining implication of a refinery dock-queue processor or automatic promotion with: "Stock refinery zero-link release frees RadioClass contacts via BREAK(3). The next miner is admitted only when that miner's Mission Enter dispatch retries `CAN_DOCK`; no separate refinery queue processor or release-time promotion callback was verified."
- Older `HARVESTER_DOCK_UNLOAD*.md`: replace any statement that normal stock unload uses `ReleaseDockedHarvester` with: "Normal stock CMIN/HARV refinery unload completes through the zero-`unit+0x2E4` state-4 path in `UnitClass::Mission_Deploy_Building`; `ReleaseDockedHarvester` belongs to nonzero-link/interrupt contexts."
- `miner/traces/TWO_CHRONO_MINERS_SAME_REFINERY_FULL_CARGO_QUEUE_TAKEOVER_TRACE.md`: refine the unchecked takeover wording to: "B's first accepted-cell movement is commanded on B's first eligible Mission Enter dispatch after A's BREAK(3) contact release; this can be the same frame only if B has not yet run in the live-object pass and its mission timer is due."

## 13. Remaining Uncertainty

- Runtime object-vector order for a concrete two-CMIN replay was not observed; static evidence shows the condition but not which miner index wins in every setup.
- Mission ID 7's runtime timer table value was not decoded here; the verified formula includes `RandomRanged(0,2)` jitter.
- First rendered locomotor displacement after `Set_Destination` needs runtime coordinate logging.

## Sources

- Starting trace: `docs/research/miner/traces/TWO_CHRONO_MINERS_SAME_REFINERY_FULL_CARGO_QUEUE_TAKEOVER_TRACE.md`
- Ghidra read-only decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Ghidra read-only decompile: `FootClass::Mission_Enter @ 0x004D9290`
- Ghidra read-only decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`
- Ghidra read-only decompile: `FootClass::Receive_Radio @ 0x004D8FB0`
- Ghidra read-only decompile: `RadioClass::Transmit_Radio_Impl @ 0x0065A970`
- Ghidra read-only decompile: `RadioClass::Receive_Radio @ 0x0065A820`
- Ghidra read-only decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`
- Ghidra read-only decompile: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`
- Prior docs: `UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`, `CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`, `MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`, `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`, `BUILDING_RECEIVE_RADIO_REFINERY_0X0E_NON_ACCEPTED_PATHS_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`
- Rust surfaces scanned: `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_tests.rs`
