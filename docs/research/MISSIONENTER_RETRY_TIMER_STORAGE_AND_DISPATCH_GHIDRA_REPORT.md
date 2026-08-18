# MissionEnter Retry Timer Storage And Dispatch - Ghidra Research Report

**Address(es):** `0x004D9290`, `0x005B3060`, `0x005B3A00`, `0x005B3700`, `0x005B3760`, `0x0065C7E0`, `0x00739EC0`, `0x006F9E50`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR miner/refinery mission `7` (`Enter`) retry timing: where `FootClass::Mission_Enter`'s `14..16` frame return is produced, where that return is stored, how later dispatch eligibility is computed, and whether accepted-cell arrival creates an immediate next-tick retry.  
**Non-Scope:** full `BuildingClass::Receive_Radio(0x0E)` switch, exact drive-locomotor pixel arrival frame, concrete replay RNG/object-index logging, non-refinery Enter targets, and Rust implementation patches.  
**Confidence:** High for static timer storage/dispatch and stock `[Enter]` cadence; Medium for a concrete replay's exact retry frame because that still requires runtime sampling of object order, RNG draw, `+0xC8`, and `+0xD0`.  
**Active in YR:** Yes. Mission id `7` dispatches through `MissionClass::Mission_Dispatch` to vtable slot `+0x240`, whose Foot/Unit vtables point to `FootClass::Mission_Enter @ 0x004D9290`; stock harvesters use `Dock=NAREFN,GAREFN`, and stock refineries use `DockUnload=yes`.

## 0. Investigation Gate

**Target question:** Where is the `FootClass::Mission_Enter` return delay stored, how is it tested on later ticks, and does standard refinery accepted-cell arrival bypass that timer?

**Non-goals:** Do not re-decode accepted `NW+(3,1)` vs `GetDockCoord NW+(2,1)`, do not implement Rust, do not cover all Enter consumers, and do not claim a concrete replay frame without runtime logging.

**Evidence needed to mark COMPLETE:** decompile plus assembly context for `Mission_Enter` epilogue; decompile plus assembly context for `MissionClass::Mission_Dispatch`; binary reader/default proof for `[Enter] Rate`; xref/dispatch proof for mission id `7`; static check that `UnitClass::PerCellProcess`/arrival does not call `Mission_Enter` or write the mission dispatch timer; Rust surface scan for handoff.

**Stop conditions:** stop after the dispatch/storage mechanism has no open static questions; defer only concrete replay sampling that requires a runtime debugger; do not edit Rust or in-repo docs.

## 1. Overview

`Mission_Enter` does not schedule itself and does not run every tick after a refinery accepted-cell arrival. It returns a delay to `MissionClass::Mission_Dispatch`; dispatch stores the current frame at MissionClass `+0xC8` and the returned delay at `+0xD0`, then later calls the mission again only when `g_CurrentFrameCounter - +0xC8 >= +0xD0`.

For stock `[Enter] Rate=.016`, `FootClass::Mission_Enter` returns `ftol(.016 * 900.0) + RandomRanged(0,2)`, i.e. `14`, `15`, or `16` frames. Accepted-cell arrival can make the miner stopped and eligible for a later `0x12 == 0x14`/`0x16` handshake, but it does not itself create an immediate next-tick Mission Enter retry.

## 2. Class Layout / Key Offsets

| Offset / data | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `MissionClass+0x6C` | health/alive gate checked before mission switch | `MissionClass::Mission_Dispatch @ 0x005B30A7` | Yes |
| `MissionClass+0x90` | active/in-limbo gate; zero returns before timer check | `0x005B306C..0x005B3074` | Yes |
| `MissionClass+0xAC` | current mission id; `7` = Enter | `0x005B30B2`, `0x005B3A00`; mission name table index 7 points to `"Enter"` | Yes |
| `MissionClass+0xC8` | mission timer start frame | `0x005B307A`, store at `0x005B311D` for mission 7 | Yes |
| `MissionClass+0xCC` | dispatch scratch/middle dword written after handler return; not used by the timer gate in this slice | `0x005B311F..0x005B3123` | Yes |
| `MissionClass+0xD0` | mission timer duration from handler return | load `0x005B3080`, store `0x005B3126` for mission 7 | Yes |
| `DAT_00A8ED84` | global current frame counter | read `0x005B3091`, store source `0x005B3116` | Yes |
| `DAT_00A8E3A8` | mission-control table base, 32 entries x 32 bytes | `0x005B3A00`, `0x00679C94..0x00679CAD` | Yes |
| mission entry `+0x10` | `Rate` double used by mission handler return formula | `0x004D9473`, `0x005B3760` | Yes |
| mission entry `+0x18` | `AARate` double, copied from `Rate` if INI read returns zero | `0x005B3760` | Yes |

## 3. Core Logic

### 3.1 Dispatch Gate And Storage

Verified decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`.

Assembly context:

```text
005b3067: CALL 0x005f3e70              ; ObjectClass::AI
005b306c: MOV  AL,byte ptr [EDI+0x90]
005b3074: JZ   0x005b34e0              ; inactive/limbo early return
005b307a: MOV  EDX,dword ptr [EDI+0xc8]
005b3080: MOV  EAX,dword ptr [EDI+0xd0]
005b3086: LEA  ESI,[EDI+0xc8]
005b308c: CMP  EDX,-0x1
005b308f: JZ   0x005b309f
005b3091: MOV  ECX,dword ptr [0x00a8ed84]
005b3097: SUB  ECX,EDX                 ; elapsed = current_frame - start
005b3099: CMP  ECX,EAX
005b309b: JGE  0x005b30a7              ; dispatch when elapsed >= duration
005b309d: SUB  EAX,ECX
005b309f: TEST EAX,EAX
005b30a1: JNZ  0x005b34e0              ; return while not due
```

Mission id `7` dispatches through the jump table entry `0x005B310C`:

```text
005b310c: MOV  EDX,dword ptr [EDI]
005b310e: MOV  ECX,EDI
005b3110: CALL dword ptr [EDX+0x240]   ; mission 7 -> Mission_Enter
005b3116: MOV  ECX,dword ptr [0x00a8ed84]
005b311d: MOV  dword ptr [ESI],ECX     ; +0xC8 = current frame
005b311f: MOV  ECX,dword ptr [ESP+0x8]
005b3123: MOV  dword ptr [ESI+0x4],ECX ; +0xCC = stack local
005b3126: MOV  dword ptr [ESI+0x8],EAX ; +0xD0 = handler return
```

There is no decrementing counter field. The native timer is passive: dispatch computes elapsed frames from a stored start frame and stored duration. The inclusive boundary is `elapsed >= duration`.

### 3.2 Mission Enter Return Formula

Verified decompile: `FootClass::Mission_Enter @ 0x004D9290`.

The mission timer epilogue:

```text
004d946c: MOV  ECX,ESI
004d946e: CALL 0x005b3a00              ; &MissionControl[current mission]
004d9473: FLD  double ptr [EAX+0x10]   ; Rate
004d9476: FMUL double ptr [0x007e27f8] ; 900.0
004d947c: CALL 0x007c5f00              ; Math__ftol
004d9481: MOV  ESI,EAX
004d9488: PUSH 0x2
004d948a: PUSH 0x0
004d9492: CALL 0x0065c7e0              ; RandomRanged(0,2)
004d9497: ADD  EAX,ESI
004d949b: RET
```

`MissionClass::GetMissionTimerEntry @ 0x005B3A00` returns:

```text
&DAT_00A8E3A8 + *(this+0xAC) * 0x20
```

`Random__RandomRanged @ 0x0065C7E0` is inclusive for `(0,2)`: if min and max differ, it normalizes ordering, computes `max - min`, samples until the masked result is `<= max-min`, and returns `min + sample`.

### 3.3 `[Enter] Rate` Source

`RulesClass::ReadTypeData @ 0x00679C94..0x00679CAD` initializes/read-fills 32 mission entries:

```text
00679c94: MOV EDI,0xa8e3a8
00679c99: PUSH ESI
00679c9a: MOV ECX,EDI
00679c9c: MOV dword ptr [EDI],EBX      ; entry id
00679c9e: CALL 0x005b3760              ; MissionClass::Read_INI
00679ca3: ADD EDI,0x20
00679ca6: INC EBX
00679ca7: CMP EDI,0xa8e7a8
00679cad: JL  0x00679c99
```

`MissionClass::Read_INI @ 0x005B3760` reads the mission name from `g_MissionNameTable`, reads `Rate` into entry `+0x10`, reads `AARate` into entry `+0x18`, and if `AARate` is `0.0`, copies `Rate` into `AARate`.

Mission name table evidence: pointer at `0x00816CC8` (index `7`) is `0x00816E34`; bytes at `0x00816E34` are `45 6E 74 65 72 00`, i.e. `"Enter"`.

Stock INI evidence: `ini/rulesmd.ini:[Enter]` has `Rate=.016` and no `AARate`. Therefore the stock Enter mission base is:

```text
ftol(0.016 * 900.0) = ftol(14.4) = 14
14 + RandomRanged(0,2) = 14..16 frames
```

The constructor at `0x005B3700` also defaults each mission entry to `Rate=AARate=0.016` before INI overrides (`0x3F90624D:D2F1A9FC` double).

### 3.4 Accepted-Cell Arrival Does Not Bypass The Timer

Verified decompile: `TechnoClass::AI_Update @ 0x006F9E50` calls `MissionClass::Mission_Dispatch` during the normal object AI update. `Mission_Dispatch` itself performs the due check above before it calls vtable slot `+0x240`.

Verified decompile/callees: `UnitClass::PerCellProcess @ 0x00739EC0` can send radio `0x15`, call `FootClass::PerCellProcess`, process cell actions, stop movement, and queue other missions in later error/fallback branches, but it does not call `MissionClass::Mission_Dispatch`, does not call `FootClass::Mission_Enter`, and does not write the dispatch timer fields `+0xC8/+0xD0`.

Therefore a stock miner that reaches the accepted refinery cell after a `0x12 == 1` movement order waits until its next ordinary `Mission_Dispatch` eligibility. Accepted-cell arrival itself is not an immediate next-tick `CAN_DOCK` retry source. A same-frame or next-frame retry can occur only if the stored timer is already due when that unit's AI pass reaches `Mission_Dispatch`.

## 4. INI Keys

| Key | Stock YR value | Binary reader/effect | Active in YR |
|---|---:|---|---|
| `[Enter] Rate` | `.016` | `MissionClass::Read_INI @ 0x005B3760` stores to mission entry `+0x10`; used by `0x004D9473` | Yes |
| `[Enter] AARate` | absent/zero | read to `+0x18`; zero causes copy from `Rate` | Yes |
| `[CMIN] Dock` | `NAREFN,GAREFN` | stock Chrono Miner targets refineries for Enter/dock path | Yes |
| `[HARV] Dock` | `NAREFN,GAREFN` | stock War Miner targets refineries for Enter/dock path | Yes |
| `[GAREFN]/[NAREFN] DockUnload` | `yes` | building-side receiver participates in refinery unload handoff | Yes |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `TechnoClass::AI_Update @ 0x006F9E50` | Calls `MissionClass::Mission_Dispatch` during object AI | decompile/caller evidence | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | due check, mission handler call, `+0xC8/+0xD0` storage | decompile + assembly context | Yes |
| `FootClass::Mission_Enter @ 0x004D9290` | sends one `CAN_DOCK(0x0E)` per dispatch, returns timer+jitter | decompile + assembly context | Yes |
| `MissionClass::GetMissionTimerEntry @ 0x005B3A00` | current mission entry lookup | decompile | Yes |
| `MissionClass::Read_INI @ 0x005B3760` | reads `[Enter] Rate`/`AARate` | decompile + INI | Yes |
| `UnitClass::PerCellProcess @ 0x00739EC0` | arrival/cell hook; not a Mission Enter dispatch source | decompile + callees | Yes |

## 6. Current Rust Implementation Status

Current Rust surfaces scanned:

- `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter` runs CAN_DOCK-like admission whenever the Rust phase is processed. It has accepted-cell and already-entered checks, but no native `14..16` frame Mission Enter retry timer.
- `src/sim/miner/miner_dock_sequence.rs::phase_awaiting_accepted_cell` sets `dock_phase = MissionEnter` immediately after movement stops. That models "next pass retries" rather than gamemd's stored mission timer gate.
- `src/sim/miner/miner_dock_sequence.rs::phase_linked` still snaps `snap.rx/snap.ry` to `pad` and links on pad; that belongs to the separate NW+2/NW+3 FSM correction, not this timer slice, but it is the same handoff area.
- `src/sim/miner/miner_system.rs::tick_miners` processes miners in stable-id order. That can model the order-dependent nature of same-frame takeover, but it does not model gamemd's live object vector identity or per-mission timer state.
- `src/sim/miner/miner_tests.rs::dock_sequence_progresses_through_phases` and related dock tests cover phase progress and accepted/queue cells, but they do not assert `14..16` frame Mission Enter retry delay or "arrival alone does not retry next tick."

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MissionClass::Mission_Dispatch` timer load/test | verified | `0x005B3060`, assembly `0x005B307A..0x005B30A1` | none |
| mission id `7` dispatch to vtable `+0x240` | verified | jump table index 7 -> `0x005B310C`; assembly `0x005B3110` | none |
| handler return storage | verified | assembly `0x005B3116..0x005B3126` | none |
| passive elapsed-frame semantics | verified | `0x005B3091..0x005B309D` | none |
| `Mission_Enter` timer epilogue | verified | `0x004D946C..0x004D9497` | none |
| mission table stride and Rate offset | verified | `0x005B3A00`, `0x005B3760`, `0x00679C94..0x00679CAD` | none |
| mission name index 7 = `Enter` | verified | `0x00816CC8 -> 0x00816E34`, bytes `"Enter\0"` | none |
| stock `[Enter] Rate=.016` | verified | `ini/rulesmd.ini`, reader `0x005B3760` | none |
| `RandomRanged(0,2)` inclusive | verified | `0x0065C7E0` | none |
| accepted-cell arrival as immediate retry source | verified negative | `UnitClass::PerCellProcess @ 0x00739EC0` callees; `Mission_Dispatch @ 0x005B3060` gate | none for static mechanism |
| concrete replay retry frame | deferred | requires runtime state/RNG/object order | runtime debugger logging |
| Rust exact delta | touched-not-exhausted | source scan lines around `phase_mission_enter`/`phase_awaiting_accepted_cell` | implementation pass should add tests and patch |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What exact slice is being claimed? -> Mission Enter retry timer storage/dispatch only for standard YR miner/refinery mission 7.` (evidence: target prompt; scoped Ghidra addresses above)
- `[RESOLVED] OQ-02 - Where does Mission Enter compute `14..16`? -> `0x004D946C..0x004D9497` loads mission Rate `+0x10`, multiplies by `900.0`, converts through `Math__ftol`, then adds inclusive `RandomRanged(0,2)`.` (evidence: `0x004D946C..0x004D9497`, `0x0065C7E0`)
- `[RESOLVED] OQ-03 - Where is the returned delay stored? -> `MissionClass::Mission_Dispatch` stores handler return `EAX` to `this+0xD0` and current frame to `this+0xC8`.` (evidence: `0x005B3116..0x005B3126`)
- `[RESOLVED] OQ-04 - How is it decremented? -> It is not decremented; dispatch recomputes `elapsed = current_frame - +0xC8` and compares against `+0xD0`.` (evidence: `0x005B307A..0x005B30A1`)
- `[RESOLVED] OQ-05 - What is the due boundary? -> dispatch when `elapsed >= duration`; return while `elapsed < duration`.` (evidence: `CMP ECX,EAX` then `JGE 0x005B30A7`)
- `[RESOLVED] OQ-06 - Is mission id 7 definitely `[Enter]`? -> Yes; mission name table index 7 points to string `"Enter"`, and jump table index 7 calls vtable `+0x240`.` (evidence: `0x00816CC8`, `0x00816E34`, `0x005B310C`)
- `[RESOLVED] OQ-07 - Is stock `[Enter] Rate` known statically? -> Yes, `rulesmd.ini:[Enter] Rate=.016`, read by `MissionClass::Read_INI`.` (evidence: `ini/rulesmd.ini`, `0x005B3760`)
- `[RESOLVED] OQ-08 - Does accepted-cell arrival write the mission timer or call Mission Enter? -> No evidence of that in `UnitClass::PerCellProcess`; the ordinary AI dispatch gate remains the caller.` (evidence: `0x00739EC0` decompile/callees, `0x006F9E50 -> 0x005B3060`)
- `[RESOLVED] OQ-09 - Can same-frame takeover still happen? -> Yes, but only if the waiter is processed later and its stored mission timer is already due.` (evidence: `0x005B3060` gate; prior same-frame-order reports)
- `[RESOLVED] OQ-10 - Does Rust currently model this timer? -> No first-class `14..16` MissionEnter retry timer was found in the scanned dock phases.` (evidence: `src/sim/miner/miner_dock_sequence.rs:613..698`)
- `[DEFERRED] OQ-11 - What exact frame will a specific retail replay retry on?` (category: `needs-runtime-debugger`; reason: requires sampling that miner's object index, `+0xC8`, `+0xD0`, current frame, and RNG draw; next-step-if-pursued: set runtime logging around `0x005B3060` and `0x004D9290`)
- `[DEFERRED] OQ-12 - Does stable-id order exactly match gamemd's live object vector order in all Rust scenarios?` (category: `requires-different-system-context`; reason: this slot only verifies timer dispatch, not global object-vector identity; next-step-if-pursued: compare Rust stable-id iteration with `g_CurrentObjects_Data` insertion/limbo semantics)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Mission Enter retry is timer-gated by `+0xC8/+0xD0`; there is no per-tick retry. | `0x005B307A..0x005B30A1`, `0x005B3116..0x005B3126` | missing | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`; miner component state | Store an Enter retry timer/due frame after each MissionEnter dispatch and skip CAN_DOCK until due. | `miner_mission_enter_retry_waits_until_14_16_frame_timer_due` | Do not let `phase_awaiting_accepted_cell` immediately re-run CAN_DOCK next tick just because movement stopped. |
| Stock `[Enter]` delay is `14..16` frames: `ftol(.016 * 900.0) + inclusive 0..2`. | `0x004D946C..0x004D9497`, `0x005B3760`, `rulesmd.ini:[Enter]`, `0x0065C7E0` | missing | `src/sim/miner` timer/RNG surface | Use the same mission-rate formula and deterministic RNG consumption if/when exact timing parity is implemented. | `miner_mission_enter_retry_jitter_consumes_rng_and_returns_14_15_or_16` | Do not hardcode a fixed 15-frame delay or drop RNG consumption. |
| Accepted-cell arrival does not bypass the stored Mission Enter timer. | `UnitClass::PerCellProcess @ 0x00739EC0` no MissionEnter/timer write; `TechnoClass::AI_Update @ 0x006F9E50` ordinary dispatch path | mismatch likely | `phase_awaiting_accepted_cell` | After accepted movement stops, keep the mission Enter state but wait for the stored due frame before sending the next CAN_DOCK. | `accepted_cell_arrival_does_not_retry_can_dock_next_tick_when_timer_not_due` | Do not convert movement arrival into an immediate `0x12==0x14 -> 0x18/0x16` handoff. |
| Same-frame refinery takeover is conditional on waiter order plus waiter timer already due. | `0x005B3060`; prior `MISSION_ENTER_CANDOCK_RETRY_SAME_FRAME_ORDER` reports | partially represented by order tests, timer missing | `src/sim/miner/miner_system.rs::tick_miners`; `miner_tests.rs` | Keep order-dependent behavior but add timer-gated variants. | `waiter_after_releaser_claims_same_tick_only_when_enter_timer_due`; `waiter_after_releaser_does_not_claim_same_tick_when_enter_timer_not_due` | Do not promote a waiting miner solely from refinery release; the waiter must run its own eligible MissionEnter pass. |

## 10. Negative Facts / Do Not Do

- Do not model Mission Enter retry as a decrementing field. The binary stores start frame plus duration and computes elapsed.
- Do not retry `CAN_DOCK` every tick after accepted-cell arrival.
- Do not start the `0x18/0x16` handoff solely because movement to accepted `NW+(3,1)` completed.
- Do not drop `RandomRanged(0,2)` from `[Enter]` cadence.
- Do not claim same-frame takeover is always allowed or always impossible; it depends on live object order and the waiter's timer due state.
- Do not conflate the mission dispatch timer with the `0x16` facing/RateTimer at unit `+0x388`; they are separate timing mechanisms.

## 11. Remaining Uncertainty

- Concrete replay frame: exact stored `+0xC8/+0xD0`, RNG draw, and live-object index for a given miner must be runtime-logged.
- Rust order identity: stable-id iteration may or may not match every gamemd live-object-vector case; this timer report does not settle global object-order parity.

## 12. Concrete Rust Test Names

- `miner_mission_enter_retry_waits_until_14_16_frame_timer_due`
- `miner_mission_enter_retry_jitter_consumes_rng_and_returns_14_15_or_16`
- `accepted_cell_arrival_does_not_retry_can_dock_next_tick_when_timer_not_due`
- `accepted_cell_arrival_retries_can_dock_when_enter_timer_due`
- `waiter_after_releaser_claims_same_tick_only_when_enter_timer_due`
- `waiter_after_releaser_does_not_claim_same_tick_when_enter_timer_not_due`

## Sources

- Ghidra read-only decompile: `FootClass::Mission_Enter @ 0x004D9290`.
- Ghidra read-only assembly context: `0x004D946C..0x004D9497`.
- Ghidra read-only decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`.
- Ghidra read-only assembly context: `0x005B307A..0x005B30A1`, `0x005B310C..0x005B3126`.
- Ghidra read-only decompile: `MissionClass::GetMissionTimerEntry @ 0x005B3A00`.
- Ghidra read-only decompile: `MissionClass::Read_INI @ 0x005B3760`.
- Ghidra read-only assembly context: `RulesClass::ReadTypeData @ 0x00679C94..0x00679CAD`.
- Ghidra read-only decompile: `Random__RandomRanged @ 0x0065C7E0`.
- Ghidra read-only decompile/callees: `UnitClass::PerCellProcess @ 0x00739EC0`.
- Ghidra read-only decompile: `TechnoClass::AI_Update @ 0x006F9E50`.
- Ghidra read-only memory: `0x00816CC8 -> 0x00816E34`, bytes `"Enter\0"`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `[Enter] Rate=.016`.
- Rust scanned: `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs`, `miner_system.rs`, `miner_tests.rs`.
