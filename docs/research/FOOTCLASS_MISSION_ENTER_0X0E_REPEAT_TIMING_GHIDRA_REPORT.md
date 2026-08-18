# FootClass Mission Enter 0x0E Repeat Timing - Ghidra Research Report

**Address(es):** `FootClass__Mission_Enter @ 0x004D9290`, `MissionClass__Mission_Dispatch @ 0x005B3060`, `BuildingClass__Receive_Radio @ 0x0043C2D0`, `FootClass__Receive_Radio @ 0x004D8FB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard YR harvester/refinery `Mission Enter` repeat timing around `CAN_DOCK(0x0E)`, receiver `MOVE_TO_CELL(0x12)` return `1` vs `0x14`, mission timer storage, and whether `Mission_Enter` can create a next-tick second `0x16`.  
**Non-Scope:** Full `UnitClass::Receive_Radio(0x16)` timer cascade internals, PerCellProcess scheduling, full locomotor movement completion, and non-refinery enter targets.  
**Confidence:** High for the static branch/timer behavior; Medium for which 0x15 source wins first globally, because sibling slots must close tick-order and locomotor timing.  
**Active in YR:** Yes. Standard `[CMIN]`/`[HARV]` use Mission `7` / Enter toward stock `[GAREFN]`/`[NAREFN]` with `DockUnload=yes`.

## 0. Working Notes Contract

**Target question:** Determine exactly when `FootClass::Mission_Enter @ 0x004D9290` sends or re-sends `CAN_DOCK(0x0E)` after the refinery's `0x12` accepted-cell response, and whether it can cause a second `0x16` in the next tick before PerCellProcess can fire.

**Non-goals:** Do not reclassify refinery dock cells, re-decode `0x16`, implement Rust, or investigate all non-refinery enter paths.

**Evidence needed to mark COMPLETE:** Live Ghidra decompile plus assembly context for `0x004D9290`, `0x005B3060`, and the building `0x12 -> 0x18 -> 0x16` gate; INI/default proof for `[Enter] Rate`; Rust scan for the affected miner surfaces.

**Stop conditions:** Stop once Mission Enter's repeat cadence, `0x12` return consequences, mission timer writes, next-tick `0x16` possibility, and Rust handoff are proven for stock refinery docking.

## 1. Overview

`FootClass::Mission_Enter` sends `CAN_DOCK(0x0E)` only when `MissionClass::Mission_Dispatch` calls mission id `7`. It does not spin in a local retry loop and it does not schedule a next-tick resend. After every scoped path, including the already-there path that makes the building send `0x18` then `0x16`, the function reaches the normal `[Enter]` mission delay epilogue.

The stock delay is `ftol([Enter].Rate * 900.0) + RandomRanged(0,2)`. With stock YR `[Enter] Rate=.016`, the return is `14..16` frames. `Mission_Dispatch` then writes this return value into `this+0xD0`, so a second `Mission_Enter` / `CAN_DOCK` / building `0x16` pass cannot be caused by this function on the next tick.

## 2. Class Layout / Key Offsets

| Offset / value | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| MissionClass `+0xAC` | current mission id; mission `7` dispatches Enter via vtable `+0x240` | `MissionClass__Mission_Dispatch @ 0x005B3060` decompile; vtable xrefs `0x007E8ED4/0x007EB298/0x007F5EB0` from prior slot | Yes |
| MissionClass `+0xC8` | mission timer start frame | `0x005B3138..0x005B313F` | Yes |
| MissionClass `+0xCC` | middle timer/scratch dword written from stack by dispatch | `0x005B3141..0x005B3145` | Yes |
| MissionClass `+0xD0` | handler-return delay/duration | `0x005B3148` | Yes |
| Unit/Foot byte `+0x418` | already-entered preserve flag for non-ROGER `CAN_DOCK` | `0x004D92C4..0x004D92CC` | Yes |
| Foot `+0x5A4` | NavCom/destination pointer checked after accepted/preserved `CAN_DOCK` | `0x004D92ED..0x004D9303` | Yes |
| Mission-control entry `+0x10` | `Rate` double used by Enter delay epilogue | `0x004D9473`, `0x005B3760` | Yes |

## 3. Core Logic

### 3.1 Mission Dispatch Is The Repeat Gate

`MissionClass__Mission_Dispatch @ 0x005B3060` calls `ObjectClass::AI`, then checks the mission timer before any mission handler call. If `+0xC8 != -1`, it computes `elapsed = g_CurrentFrameCounter - +0xC8`; if `elapsed < +0xD0`, it returns without calling mission id `7`. If eligible, mission id `7` calls vtable slot `+0x240` and then stores the handler return:

- `0x005B3091..0x005B30A1`: timer comparison; dispatch proceeds only when the remaining duration reaches zero.
- `0x005B30B2..0x005B30C1`: current mission switch.
- `MissionClass__Mission_Dispatch @ 0x005B3060` decompile: case `7` calls the mission handler vtable slot `+0x240`, matching prior vtable xrefs to `FootClass__Mission_Enter @ 0x004D9290`.
- `0x005B3138..0x005B3148`: writes `+0xC8 = current frame`, `+0xCC = stack local`, `+0xD0 = handler return`.

**Active in YR:** Yes. Mission id `7` is the standard Enter mission for stock harvester/refinery entry.

### 3.2 Mission Enter Sends `0x0E` Once Per Dispatch

Inside `FootClass__Mission_Enter @ 0x004D9290`, the target path sends one `CAN_DOCK(0x0E)` through the unit radio vtable:

- `0x004D92B2..0x004D92B9`: push target and `0x0E`, call vtable `+0x278`.
- `0x004D92BF..0x004D92CC`: if reply is `1`, or if byte `+0x418` is nonzero, continue the enter path.
- `0x004D92CE..0x004D92E8`: if reply is not `1` and `+0x418 == 0`, send `BREAK(3)`, call vtable `+0x484(0,1)`, then jump to the same timer epilogue.

There is no loop back to send `0x0E` again inside this function. The next `0x0E` depends on the next eligible `Mission_Dispatch`.

**Active in YR:** Yes for standard stock miner/refinery Enter.

### 3.3 `0x12` Return `1` Versus `0x14`

The `0x12` return is owned by the receiver path, but its consequence is decisive for repeat timing:

- Building `0x0E` sends `0x12` at `0x0043CAB4..0x0043CAB8`.
- It compares the reply to `0x14` at `0x0043CABE`.
- If the reply is not `0x14`, it returns without sending `0x18` or `0x16`.
- If the reply is `0x14`, it sends `0x18` at `0x0043CACA..0x0043CACE`, then `0x16` at `0x0043CAD7..0x0043CADB`.

Sibling report `RADIO_0X12_MOVE_TO_CELL_PAYLOAD_AND_TIMESTAMPS_GHIDRA_REPORT.md` verifies `FootClass::Receive_Radio(0x12)` returns `0x14` only when current cell already equals the payload cell; otherwise it sets destination, writes timer fields, and returns `1`.

Crucial ordering: even when `0x12` returns `0x14` and the building sends `0x16` during this same `Mission_Enter` dispatch, `Mission_Enter` then continues to the delay epilogue at `0x004D946C`.

**Active in YR:** Yes for stock refinery `CAN_DOCK` admission.

### 3.4 Mission Enter Return Delay

The epilogue is common to the scoped branches:

- `0x004D946C..0x004D9476`: call `MissionClass__GetMissionTimerEntry`, load entry `+0x10`, multiply by `900.0`.
- `0x004D947C..0x004D9481`: `Math__ftol`, store base delay in `ESI`.
- `0x004D9488..0x004D9492`: call `Random__RandomRanged(0,2)`.
- `0x004D9497..0x004D949B`: add jitter to base and return.

`MissionClass__Read_INI @ 0x005B3760` reads `Rate` into entry `+0x10`; `rulesmd.ini:[Enter] Rate=.016` is present at `ini/rulesmd.ini:30507..30510`. `Random__RandomRanged @ 0x0065C7E0` is inclusive for `0..2`. The stock return is therefore `ftol(.016 * 900.0) + 0..2 = 14..16`.

**Active in YR:** Yes. This is the active stock mission-control entry used by Mission Enter.

### 3.5 Answer To The Next-Tick `0x16` Question

`Mission_Enter` can cause the building to send `0x16` only during an already-there `0x12` response in the same synchronous radio call chain. It cannot itself cause another `0x16` on the next tick:

1. The only `0x16` in this slice is building-side and gated by `0x12 == 0x14`.
2. `Mission_Enter` has no internal resend loop.
3. After the call chain, `Mission_Enter` returns `14..16`.
4. `Mission_Dispatch` writes that return into `+0xD0`.
5. Until `elapsed >= +0xD0`, mission id `7` is not called again.

So the remaining "which `0x15` source wins" question should not assume a next-tick second `0x16` from `Mission_Enter`. If a later `0x16` fires sooner, it must come from a different scheduling path than a normal next-frame Mission Enter repeat.

## 4. INI Keys

| Key | Stock YR value / location | Effect | Active in YR |
|---|---|---|---|
| `[Enter] Rate` | `.016`, `ini/rulesmd.ini:30510`; base RA2 same at `ini/rules.ini:22661` | Mission Enter base delay: `ftol(Rate * 900.0)` | Yes |
| `[Enter] AARate` | absent for YR `[Enter]`; binary copies `Rate` when `AARate` read is zero | no separate AA cadence here | Yes |
| `[CMIN] Dock` / `Harvester` | `NAREFN,GAREFN` / `yes` | activates standard chrono miner refinery entry | Yes |
| `[HARV] Dock` / `Harvester` | `NAREFN,GAREFN` / `yes` | activates standard war miner refinery entry | Yes |
| `[GAREFN]` / `[NAREFN] DockUnload` | `yes` | building receiver path that sends `0x12`, then `0x18`/`0x16` on already-there | Yes |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `FootClass__Mission_Enter @ 0x004D9290` | sends one `CAN_DOCK(0x0E)` per dispatch and returns Enter delay | decompile; `0x004D92B2..0x004D92E8`, `0x004D946C..0x004D949B` | Yes |
| `MissionClass__Mission_Dispatch @ 0x005B3060` | gates repeat by `+0xC8/+0xD0` and stores handler return | decompile; `0x005B3091..0x005B3148` | Yes |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` | sends `0x12`; sends `0x18`/`0x16` only if reply is `0x14` | decompile; `0x0043CAB4..0x0043CADB` | Yes |
| `FootClass__Receive_Radio @ 0x004D8FB0` | `0x12` returns `0x14` already-there or `1` after destination set | sibling report plus prior decompile | Yes |
| `Random__RandomRanged @ 0x0065C7E0` | inclusive jitter `0..2` | decompile | Yes |

## 6. Current Rust Implementation Status

Current Rust already separates accepted-cell movement from the entered handshake in `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter` / `phase_awaiting_accepted_cell`, and tests such as `accepted_cell_arrival_rechecks_can_dock_before_entered_flag` preserve the `0x12 == 1` vs `0x12 == 0x14` distinction.

The likely remaining Rust delta for this slot is timing: `phase_awaiting_accepted_cell` returns to `MissionEnter` immediately after movement completes, and `phase_mission_enter` can re-run on the next sim tick. The binary `Mission_Enter` path returns `14..16` frames and `Mission_Dispatch` should gate the next `0x0E` dispatch unless another verified path bypasses it.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Mission_Enter` `0x0E` send | verified | `0x004D92B2..0x004D92B9` | none |
| `0x0E` reply preserve/abort gate | verified | `0x004D92BF..0x004D92E8` | non-refinery consumers out of scope |
| building `0x12 -> 0x18 -> 0x16` already-there gate | verified | `0x0043CAB4..0x0043CADB` | none for stock refinery |
| Mission Enter delay epilogue | verified | `0x004D946C..0x004D949B`; `[Enter] Rate` INI | none |
| Mission dispatch timer store/check | verified | `0x005B3091..0x005B3148` | none |
| Next-tick second `0x16` from Mission Enter | verified negative | no internal loop; `+0xD0=14..16` after return | sibling slots still need global first-`0x15` winner |
| Current Rust timing parity | touched-not-exhausted | source scan | implementation pass should add timer/jitter tests |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-001 - Does `Mission_Enter` send `0x0E` more than once per invocation? -> No; one vtable `+0x278` call with `0x0E`, no internal resend loop.` (evidence: `0x004D92B2..0x004D92E8`)
- `[RESOLVED] OQ-002 - What happens when building `0x12` returns `1`? -> Building does not send `0x18`/`0x16`; `Mission_Enter` returns normal `14..16` delay after the call chain.` (evidence: `0x0043CABE..0x0043CAC1`, `0x004D946C..0x004D949B`)
- `[RESOLVED] OQ-003 - What happens when building `0x12` returns `0x14`? -> Building sends `0x18` then `0x16` synchronously inside the same `Mission_Enter` radio chain; `Mission_Enter` still returns normal delay.` (evidence: `0x0043CACA..0x0043CADB`, `0x004D946C..0x004D949B`)
- `[RESOLVED] OQ-004 - What is the stock repeat delay? -> `ftol(.016 * 900.0) + RandomRanged(0,2)` = `14..16` frames.` (evidence: `0x004D9473..0x004D9497`, `ini/rulesmd.ini:30507..30510`, `0x0065C7E0`)
- `[RESOLVED] OQ-005 - Can this function cause a second `0x16` next tick before PerCellProcess? -> No; normal repeat is mission-timer gated to `14..16` frames, and `0x16` is only sent during the synchronous already-there building call chain.` (evidence: `0x005B3091..0x005B3148`, `0x0043CAB4..0x0043CADB`)
- `[RESOLVED] OQ-006 - Is this active in standard YR? -> Yes for stock CMIN/HARV entering GAREFN/NAREFN.` (evidence: mission dispatch `0x005B3060`; stock INI dock/refinery keys)
- `[DEFERRED] OQ-007 - Which `0x15` source wins first globally?` (category: `requires-different-system-context`; reason: this slot proves Mission Enter repeat timing only; sibling slots must reconcile `0x16`, PerCellProcess, and locomotor order.)
- `[DEFERRED] OQ-008 - Does movement completion or locomotor callback alter mission timer independently?` (category: `requires-different-system-context`; reason: out of this slot's `Mission_Enter` function scope; DriveLocomotor slot should resolve visibility order.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x12` return `1` assigns accepted-cell movement but does not let building send `0x18`/`0x16`; next `Mission_Enter` repeat is `14..16` frames. | `0x0043CABE..0x0043CAC1`, `0x004D946C..0x004D949B`, `0x005B3138..0x005B3148` | Rust likely rechecks immediately after accepted-cell movement completes | `src/sim/miner/miner_dock_sequence.rs::phase_awaiting_accepted_cell`, `phase_mission_enter` | Add/retain a Mission Enter dispatch timer before the already-there recheck unless another verified path bypasses it. | Miner reaches accepted cell after `0x12==1`; no entered flag until Enter timer expires, then already-there pass can set it. Proposed test: `mission_enter_accepted_cell_recheck_waits_enter_rate_delay` | Do not make accepted-cell arrival itself trigger the already-there handshake next tick. |
| Already-there `0x12==0x14` sends `0x18`/`0x16` synchronously, then Mission Enter still returns the normal delay. | `0x0043CACA..0x0043CADB`, `0x004D946C..0x004D949B` | Rust has `Linked` handoff but no explicit post-handoff Enter timer | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter` | Preserve synchronous entered/contact transition, but do not schedule another immediate `CAN_DOCK` from Mission Enter. | Miner starts MissionEnter already at `(13,11)`; entered flag set once, no second admission retry next tick. Proposed test: `mission_enter_already_there_sends_entered_once_then_delays_retry` | Do not use MissionEnter as the source of repeated `0x16` pulses. |
| Busy/non-ROGER `CAN_DOCK` without `+0x418` sends `BREAK(3)`/clear path and still returns normal Enter delay; with `+0x418` it preserves. | `0x004D92BF..0x004D92E8` | Rust has a busy defer path that preserves target for waiters | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`, `src/sim/miner/miner_dock.rs` | Split ordinary busy wait from already-entered preserve and apply the same Mission Enter timer cadence. | Busy refinery waiter cannot retry every tick; already-entered miner is preserved. Proposed test: `mission_enter_busy_retry_uses_enter_rate_delay_unless_entered` | Do not poll `CAN_DOCK` every tick while waiting. |

## 10. Negative Facts / Do Not Do

- Do not model `Mission_Enter` as a per-tick `CAN_DOCK` loop; repeat is gated by `Mission_Dispatch` and the returned `14..16` frame delay. Evidence: `0x005B3091..0x005B3148`, `0x004D946C..0x004D949B`.
- Do not treat `0x12` return `1` as permission to send `0x18`/`0x16`; building requires `0x14`. Evidence: `0x0043CABE..0x0043CADB`.
- Do not attribute a next-tick second `0x16` to `Mission_Enter`; no such path exists in the scoped function. Evidence: one `0x0E` call at `0x004D92B2..0x004D92B9`; epilogue delay at `0x004D946C..0x004D949B`.
- Do not drop the `RandomRanged(0,2)` jitter. Evidence: `0x004D9488..0x004D9497`, `0x0065C7E0`.
- Do not collapse accepted `CAN_DOCK` target timing with `GetDockCoord` PerCellProcess timing; this slot only proves the accepted-cell Mission Enter repeat cadence.

## 11. Remaining Uncertainty

- The first winning `0x15` source remains unresolved by this slot alone; it must be reconciled with the `0x16` receiver, PerCellProcess caller order, and locomotor arrival visibility slots.
- Whether a locomotor/movement completion path independently makes Mission Enter eligible sooner was not investigated here; this report proves only that `Mission_Enter` itself and normal `Mission_Dispatch` do not do that.

## 12. Stale Docs / Follow-up Docs

- `docs/research/miner/MISSION_ENTER_CANDOCK_RETRY_SAME_FRAME_ORDER_GHIDRA_REPORT.md`: replace "whatever the mission timer entry produces plus a random `0..2` jitter" with "stock `[Enter] Rate=.016` makes `FootClass::Mission_Enter` return `ftol(.016 * 900.0) + RandomRanged(0,2) = 14..16` frames."
- Any refinery-dock doc or test comment implying "accepted-cell arrival rechecks CAN_DOCK on the next tick" should be narrowed to: "accepted-cell arrival must re-enter `CAN_DOCK` on a later Mission Enter dispatch; stock `Mission_Enter` returns a `14..16` frame delay unless another separately verified path bypasses the mission timer."

## Sources

- Ghidra read-only decompile: `FootClass__Mission_Enter @ 0x004D9290`.
- Ghidra read-only assembly contexts: `0x004D92BF`, `0x004D92D4`, `0x004D946C`, `0x004D9488`, `0x004D9497`.
- Ghidra read-only decompile/assembly: `MissionClass__Mission_Dispatch @ 0x005B3060`, especially `0x005B3091..0x005B3148`.
- Ghidra read-only decompile/assembly: `BuildingClass__Receive_Radio @ 0x0043C2D0`, especially `0x0043CAB4..0x0043CADB`.
- Ghidra read-only decompile: `MissionClass__Read_INI @ 0x005B3760`, `MissionClass__GetMissionTimerEntry @ 0x005B3A00`, `Random__RandomRanged @ 0x0065C7E0`.
- Existing docs read: `RADIO_0X12_MOVE_TO_CELL_PAYLOAD_AND_TIMESTAMPS_GHIDRA_REPORT.md`, `UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`, `miner/WAITING_MINER_MISSION_TIMER_AFTER_BUSY_CANDOCK_GHIDRA_REPORT.md`, `REFINERY_DOCK_0X16_BRIDGE_VERIFICATION_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `rules.ini`.
- Rust scanned: `src/sim/miner/miner_dock_sequence.rs`, `miner_tests.rs`.

## Status

COMPLETE for the scoped `Mission_Enter` / `CAN_DOCK` repeat timing slice. The global first-`0x15` winner remains for sibling slots.
