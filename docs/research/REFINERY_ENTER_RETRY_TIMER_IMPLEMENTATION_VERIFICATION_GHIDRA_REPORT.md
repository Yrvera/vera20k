# Refinery Enter Retry Timer Implementation Verification - Ghidra Research Report

**Address(es):** `0x004D9290`, `0x005B3060`, `0x005B3A00`, `0x005B3760`, `0x0065C7E0`, `0x007C5F00`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** current Rust implementation of the refinery `Mission_Enter` retry timer against active YR `gamemd.exe`: `[Enter] Rate * 900`, `Math::ftol`, `RandomRanged(0,2)`, `+0xC8/+0xD0` storage, dispatch due condition, and per-dispatch jitter consumption.  
**Non-Scope:** accepted-cell coordinate, `0x18`/`0x16` semantics outside timer scheduling, radio `0x15` side effects, `Mission_Deploy_Building` unload side effects, pad/link occupancy, and Rust edits.  
**Confidence:** High for stock binary timer behavior and current Rust deltas; Medium for exact long-runtime frame-wrap behavior because no runtime wrap trace was sampled.  
**Active in YR:** Yes. Standard CMIN/HARV refinery docking uses mission id `7` (`Enter`) and stock `[Enter] Rate=.016`.

## 0. Working Notes

**Target question:** Does Rust's refinery `Mission_Enter` retry timer match active `gamemd.exe` timing, rounding, RNG, storage, dispatch condition, and per-path scheduling?  
**Non-goals:** No deploy side effects, no `0x15`/pad/unload re-verification, no Rust edits.  
**Evidence needed to mark COMPLETE:** Binary/decompile plus address proof for timer computation/storage/dispatch and caller paths, INI/default source for `[Enter] Rate`, and current Rust comparison.  
**Stop conditions:** Every seeded timer/Rust/INI question is resolved or explicitly deferred; zero-add pass over the primary timer path adds no new material timer questions.

## 1. Overview

Native `Mission_Enter` computes its retry delay at the end of every dispatch as `Math::ftol(MissionControl[mission].Rate * 900.0) + RandomRanged(0,2)`. `MissionClass::Mission_Dispatch` stores the current frame at `this+0xC8` and the returned duration at `this+0xD0`, then later dispatches when `current_frame - start >= duration`.

Current Rust is much closer than the older immediate-retry model: it stores a start frame and duration, waits on `elapsed >= duration`, and does not let accepted-cell arrival bypass the timer. Two timer mismatches remain: the base `14` is hardcoded instead of read from `[Enter] Rate`, and the successful later `FaceSync -> MissionQueued` handoff misses the native `RandomRanged(0,2)` draw that every `Mission_Enter` dispatch consumes before returning.

## 2. Class Layout / Key Offsets

| Offset / data | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `MissionClass+0xAC` | current mission id; `7` selects `Enter` | `MissionClass::Mission_Dispatch @ 0x005B3060`, case 7 calls vtable `+0x240`; mission table index 7 is `Enter` per prior timer report | Yes |
| `MissionClass+0xC8` | mission timer start frame | store at `0x005B311D`; load at `0x005B307A` | Yes |
| `MissionClass+0xD0` | mission timer duration | store at `0x005B3126`; load at `0x005B3080` | Yes |
| `MissionControl entry +0x10` | `Rate` double | `FootClass::Mission_Enter @ 0x004D9473`; `MissionClass::Read_INI @ 0x005B3760` | Yes |
| `MissionControl entry +0x18` | `AARate` double; zero copies `Rate` | `MissionClass::Read_INI @ 0x005B3760` | Yes |
| `DAT_00A8ED84` | global current frame counter | load at `0x005B3091`, store source at `0x005B3116` | Yes |

## 3. Core Logic

### 3.1 Native Delay Formula

`FootClass::Mission_Enter @ 0x004D9290` always falls through to the timer epilogue after its target/radio handling. The relevant assembly is:

```text
004d946e: CALL 0x005b3a00              ; current MissionControl entry
004d9473: FLD double ptr [EAX + 0x10]  ; Rate
004d9476: FMUL double ptr [0x007e27f8] ; 900.0
004d947c: CALL 0x007c5f00              ; Math__ftol
004d9481: MOV ESI,EAX                  ; base frames
004d9488: PUSH 0x2
004d948a: PUSH 0x0
004d9492: CALL 0x0065c7e0              ; RandomRanged(0,2)
004d9497: ADD EAX,ESI
004d949b: RET
```

Material findings:

| Finding | Evidence | Active in YR |
|---|---|---|
| The base is not a literal `14`; it is `ftol(Rate * 900.0)`. | `0x004D9473..0x004D947C` | Yes |
| The jitter is inclusive `0..2` and is added after the base. | `0x004D9488..0x004D9497`; `Random__RandomRanged @ 0x0065C7E0` | Yes |
| `Math__ftol @ 0x007C5F00` uses x87 `FISTP` after setting/checking the stored FPU control word. For positive stock `14.4`, the verified result is `14` under the project's existing `Math__ftol` control-word proof. | `0x007C5F00..0x007C5F3C`; prior `ADD_TIBERIUM_CREDITS_PURIFIER_VIRTUAL_PURIFIERS_GHIDRA_REPORT.md` control-word evidence | Yes |
| Every static `FootClass::Mission_Enter` branch observed reaches this epilogue; there is no early return before the delay calculation in the decompile. | decompile `0x004D9290` | Yes |

For stock YR, `ini/rulesmd.ini:[Enter] Rate=.016`, so the stock delay is `ftol(0.016 * 900.0) + RandomRanged(0,2)` = `14..16` frames.

### 3.2 Native Storage And Due Check

`MissionClass::Mission_Dispatch @ 0x005B3060` implements a passive start/duration timer, not a countdown:

```text
005b307a: MOV EDX,dword ptr [EDI + 0xc8]  ; start
005b3080: MOV EAX,dword ptr [EDI + 0xd0]  ; duration
005b3091: MOV ECX,dword ptr [0x00a8ed84]  ; current frame
005b3097: SUB ECX,EDX                     ; elapsed
005b3099: CMP ECX,EAX
005b309b: JGE 0x005b30a7                  ; dispatch when elapsed >= duration
005b309d: SUB EAX,ECX
005b309f: TEST EAX,EAX
005b30a1: JNZ 0x005b34e0                  ; otherwise return
```

For mission id `7`, the handler call and stores are:

```text
005b3110: CALL dword ptr [EDX + 0x240]    ; Mission_Enter
005b3116: MOV ECX,dword ptr [0x00a8ed84]
005b311d: MOV dword ptr [ESI],ECX         ; +0xC8 = current frame
005b3126: MOV dword ptr [ESI + 0x8],EAX   ; +0xD0 = returned duration
```

Material findings:

| Finding | Evidence | Active in YR |
|---|---|---|
| Dispatch condition is inclusive: `elapsed >= duration`. | `0x005B3099..0x005B309B` | Yes |
| The timer is stored as start frame plus duration. | `0x005B311D`, `0x005B3126` | Yes |
| Handler return storage happens after `Mission_Enter` has already consumed its RNG draw. | `0x004D9492` before return; `0x005B311D..0x005B3126` after vtable call | Yes |
| The native fields are dwords, not bytes. | dword loads/stores at `0x005B307A`, `0x005B3080`, `0x005B311D`, `0x005B3126` | Yes |

### 3.3 `[Enter] Rate` Reader

`MissionClass::Read_INI @ 0x005B3760` reads mission-section `Rate` into entry `+0x10` and `AARate` into `+0x18`; if `AARate` is zero, it copies `Rate`. `RulesClass::ReadTypeData @ 0x00679C94..0x00679CAD` iterates mission entries at `DAT_00A8E3A8` in `0x20`-byte strides and calls this reader for each mission.

Stock INI evidence:

```text
ini/rulesmd.ini:[Enter]
Retaliate=no
Recruitable=no
Rate=.016
```

Material finding: `[Enter] Rate` is live in standard YR and should be parser-driven for mod parity. Hardcoding `14` is equivalent only for stock `.016`.

## 4. INI Keys

| Key | Stock YR value | Binary effect | Current Rust status | Active in YR |
|---|---:|---|---|---|
| `[Enter] Rate` | `.016` | `ftol(Rate * 900.0)` base retry frames | not parsed for miner Enter retry; Rust hardcodes `14` | Yes |
| `[Enter] AARate` | absent/zero | read, then zero falls back to `Rate`; not used by this ground `Mission_Enter` return formula | not represented | Yes, but not material to this refinery ground path |

## 5. Integration Points

| Function / file | Role | Evidence | Active in YR |
|---|---|---|---|
| `FootClass::Mission_Enter @ 0x004D9290` | sends one `0x0E` attempt per dispatch and returns timer+jitter | decompile + assembly `0x004D946E..0x004D949B` | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | timer gate and storage | decompile + assembly `0x005B307A..0x005B3126` | Yes |
| `MissionClass::GetMissionTimerEntry @ 0x005B3A00` | indexes mission entry from current mission id | decompile | Yes |
| `MissionClass::Read_INI @ 0x005B3760` | reads mission `Rate`/`AARate` | decompile | Yes |
| `Random__RandomRanged @ 0x0065C7E0` | inclusive jitter helper | decompile | Yes |
| `src/sim/miner/mod.rs` | Rust stores `dock_enter_retry_start_frame` and `dock_enter_retry_duration` | source lines 291..297 | Rust active |
| `src/sim/miner/miner_dock_sequence.rs` | Rust computes/checks/schedules retry and transitions `MissionEnter`/`FaceSync` | source lines 49..100, 655..768 | Rust active |

## 6. Current Rust Implementation Status

Rust matches several core mechanics:

- It stores a start frame and duration (`dock_enter_retry_start_frame`, `dock_enter_retry_duration`) rather than a decrementing countdown.
- It checks `elapsed >= duration` in `enter_retry_due`.
- It uses inclusive `next_range_u32_inclusive(0,2)`.
- It schedules after denied/busy `MissionEnter`, after accepted-cell move dispatch, and after the first already-there handshake dispatch.
- It does not let accepted-cell movement completion bypass the timer; `AwaitingAcceptedCell` returns to `MissionEnter`, and `MissionEnter` first checks `enter_retry_due`.

Rust mismatches:

| Rust surface | Current behavior | Binary delta | Severity |
|---|---|---|---|
| `miner_dock_sequence.rs:49..85` | base is hardcoded `ENTER_RETRY_BASE_FRAMES = 14` | binary uses parsed `[Enter] Rate * 900` through `Math__ftol` | DRIFT for modded/non-stock Rate; stock default equivalent |
| `miner/mod.rs:295..297` | duration field is `u8` | native `+0xD0` is a dword handler return | DRIFT for any future/data-driven Rate above 255 frames |
| `miner_dock_sequence.rs:764..766` | successful due `FaceSync` clears timer and enters `MissionQueued` without scheduling/consuming jitter | native later `Mission_Enter` dispatch still consumes `RandomRanged(0,2)` and returns a duration before dispatch stores it | DRIFT in RNG stream and mission timer state |
| `miner_dock_sequence.rs:90..92` | uses `saturating_sub` for elapsed | native x86 `SUB` wraps before signed `JGE` compare | edge DRIFT at frame-counter wrap |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Mission_Enter` delay formula | verified | `0x004D946E..0x004D949B` | none |
| `RandomRanged(0,2)` inclusivity | verified | `0x0065C7E0` decompile | none |
| `Mission_Dispatch` due check | verified | `0x005B307A..0x005B30A1` | frame-wrap runtime edge not sampled |
| `+0xC8/+0xD0` storage | verified | `0x005B311D`, `0x005B3126` | none |
| `[Enter] Rate` source | verified | `0x005B3760`, `0x00679C94..0x00679CAD`, `ini/rulesmd.ini` | Rust parser/handoff remains |
| Rust start/duration fields | verified | `src/sim/miner/mod.rs:291..297` | field width parity decision |
| Rust schedule function | verified | `src/sim/miner/miner_dock_sequence.rs:49..85` | replace hardcoded base with data-driven mission rate |
| Rust denied/busy scheduling | verified | `src/sim/miner/miner_dock_sequence.rs:672..686` | none observed for this slice |
| Rust accepted-move scheduling | verified | `src/sim/miner/miner_dock_sequence.rs:711..718` | none observed for this slice |
| Rust first already-there handshake scheduling | verified | `src/sim/miner/miner_dock_sequence.rs:699..708` | none observed for this slice |
| Rust successful later FaceSync scheduling | verified-mismatch | `src/sim/miner/miner_dock_sequence.rs:759..768` | consume/schedule native jitter before mission queue handoff |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What exact timer formula does native Mission_Enter return? -> `ftol(Rate * 900.0) + RandomRanged(0,2)`.` (evidence: `0x004D946E..0x004D949B`; Active in YR: Yes)
- `[RESOLVED] OQ-02 - Is stock base 14 or data-driven? -> Data-driven; stock `.016` produces 14 only after `Rate * 900` and `Math__ftol`.` (evidence: `0x004D9473..0x004D947C`, `ini/rulesmd.ini:[Enter]`; Active in YR: Yes)
- `[RESOLVED] OQ-03 - Is `RandomRanged(0,2)` inclusive? -> Yes, inclusive helper returns low plus accepted sample within span.` (evidence: `0x0065C7E0`; Active in YR: Yes)
- `[RESOLVED] OQ-04 - Are `+0xC8/+0xD0` start/duration or countdown? -> Start frame and duration.` (evidence: `0x005B307A..0x005B30A1`, `0x005B311D..0x005B3126`; Active in YR: Yes)
- `[RESOLVED] OQ-05 - What is the dispatch boundary? -> dispatch when elapsed is greater than or equal to duration.` (evidence: `0x005B3099..0x005B309B`; Active in YR: Yes)
- `[RESOLVED] OQ-06 - Does every observed Mission_Enter branch consume/schedule jitter? -> Yes; decompile falls through to the common epilogue with `RandomRanged(0,2)` and return.` (evidence: `0x004D9290`, `0x004D946E..0x004D949B`; Active in YR: Yes)
- `[RESOLVED] OQ-07 - Does Rust use start/duration storage? -> Yes, fields exist and `enter_retry_due` compares elapsed against duration.` (evidence: `src/sim/miner/mod.rs:291..297`, `src/sim/miner/miner_dock_sequence.rs:88..92`; Active in Rust: Yes)
- `[RESOLVED] OQ-08 - Does Rust parse `[Enter] Rate` for this timer? -> No evidence found; code hardcodes `ENTER_RETRY_BASE_FRAMES = 14`.` (evidence: `src/sim/miner/miner_dock_sequence.rs:49..85`; Active in Rust: Yes)
- `[RESOLVED] OQ-09 - Does Rust schedule after denied/busy Enter dispatches? -> Yes, the admission-denied path calls `schedule_enter_retry`.` (evidence: `src/sim/miner/miner_dock_sequence.rs:672..686`; Active in Rust: Yes)
- `[RESOLVED] OQ-10 - Does Rust schedule after accepted movement-order dispatches? -> Yes, it issues direct move if idle and calls `schedule_enter_retry`.` (evidence: `src/sim/miner/miner_dock_sequence.rs:711..718`; Active in Rust: Yes)
- `[RESOLVED] OQ-11 - Does Rust schedule after first already-there handshake? -> Yes, it marks contact/syncs facing and then calls `schedule_enter_retry`.` (evidence: `src/sim/miner/miner_dock_sequence.rs:699..708`; Active in Rust: Yes)
- `[RESOLVED] OQ-12 - Does Rust consume jitter on successful later FaceSync/0x15 handoff? -> No; it clears the timer and enters `MissionQueued` without `schedule_enter_retry`.` (evidence: `src/sim/miner/miner_dock_sequence.rs:759..768`; Active in Rust: Yes)
- `[RESOLVED] OQ-13 - Does accepted-cell arrival bypass the timer in current Rust? -> No; arrival returns to `MissionEnter`, whose first step is `enter_retry_due`.` (evidence: `src/sim/miner/miner_dock_sequence.rs:721..738`, `655..657`; Active in Rust: Yes)
- `[RESOLVED] OQ-14 - Is Rust's inclusive RNG helper shape compatible with native `RandomRanged` for 0..2? -> Yes for inclusive bounds and rejection shape at ordinary spans.` (evidence: `src/sim/rng.rs:130..148`, `0x0065C7E0`; Active in Rust/YR: Yes)
- `[DEFERRED] OQ-15 - Does Rust's `saturating_sub` create an observable frame-wrap drift in realistic sessions?` (category: `needs-runtime-debugger`; reason: native wrap boundary requires very long-frame/runtime sampling; next-step-if-pursued: add a deterministic wrap-unit test or runtime trace around frame counter wrap)
- `[DEFERRED] OQ-16 - Does Rust stable-id tick order match native live object vector for every same-frame takeover replay?` (category: `requires-different-system-context`; reason: object-vector identity is outside this timer implementation slice; next-step-if-pursued: use the live-object-vector report plus runtime logging)

Zero-add pass result: Re-reading `0x004D9290`, `0x005B3060`, and current Rust timer paths added no new material timer questions after OQ-12/OQ-15/OQ-16 were recorded.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Enter retry base is `Math::ftol([Enter] Rate * 900.0)`, not a literal. | `0x004D9473..0x004D947C`, `0x005B3760`, `ini/rulesmd.ini:[Enter]` | mismatch for non-stock/modded Rate; stock `.016` equivalent | `src/sim/miner/miner_dock_sequence.rs::schedule_enter_retry`; rules mission parser surface | Use parsed mission `[Enter] Rate` and native ftol semantics to compute base frames. | Override `[Enter] Rate=.020`; retry base should become `18`, not remain `14`. | Do not hardcode `14` as a general parity rule. |
| Native duration is stored as a dword handler return. | dword stores at `0x005B3126`; loads at `0x005B3080` | Rust stores `dock_enter_retry_duration: u8` | `src/sim/miner/mod.rs` miner timer fields | Use a width that can represent the native handler return once data-driven Rate exists. | Large modded `[Enter] Rate` above 255 frames should not wrap or saturate to byte range. | Do not keep byte storage if mission-rate parsing is added. |
| Every `Mission_Enter` dispatch consumes `RandomRanged(0,2)` before returning, including successful later handoff. | common epilogue `0x004D946E..0x004D949B` | mismatch on `FaceSync` accepted branch: no jitter draw/schedule | `src/sim/miner/miner_dock_sequence.rs::phase_face_sync` | When due `FaceSync` represents a later `Mission_Enter` dispatch that queues `0x15`, consume/schedule the native retry result before/while transitioning to mission `0x10`. | RNG stream after a successful second `0x16 -> 0x15` handoff should match a model that consumed one `RandomRanged(0,2)` draw. | Do not treat `0x15` success as skipping the `Mission_Enter` epilogue. |
| Accepted-cell arrival does not bypass the Enter timer. | `Mission_Dispatch @ 0x005B3060`; prior PerCellProcess negative report | none observed in current Rust | `phase_awaiting_accepted_cell`, `phase_mission_enter` | Preserve current wait-on-due behavior after movement completion. | Miner arrives at accepted cell with 10 frames remaining; no `0x18/0x16` handshake until due. | Do not add an arrival callback that retries CAN_DOCK immediately. |
| Dispatch uses `elapsed >= duration` against start frame and duration. | `0x005B3091..0x005B309B` | mostly matched; wrap edge differs through `saturating_sub` | `enter_retry_due` | Preserve inclusive due boundary; decide whether wrap parity matters for the sim frame counter. | At exactly duration frames elapsed, retry dispatches; one frame before, it does not. | Do not change to `>` or countdown-only behavior. |

## 10. Negative Facts / Do Not Do

- Do not describe the current hardcoded `14` as full parity; it is stock-default equivalent only.
- Do not drop the `RandomRanged(0,2)` draw on a successful later handoff to `MissionQueued`.
- Do not let accepted-cell arrival itself run the `0x18/0x16` handshake; the due check must remain first.
- Do not convert the native start-frame/duration timer into an every-tick retry.
- Do not use `AARate` for this ground refinery Enter retry unless a separate binary path proves it.
- Do not narrow native mission duration to `u8` once parsed mission rates are supported.

## Sources

- Fresh read-only Ghidra decompile: `FootClass::Mission_Enter @ 0x004D9290`.
- Fresh read-only Ghidra assembly context: `0x004D946E..0x004D949B`.
- Fresh read-only Ghidra decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`.
- Fresh read-only Ghidra assembly context: `0x005B307A..0x005B3126`.
- Fresh read-only Ghidra decompile: `MissionClass::GetMissionTimerEntry @ 0x005B3A00`.
- Fresh read-only Ghidra decompile: `MissionClass::Read_INI @ 0x005B3760`.
- Fresh read-only Ghidra assembly context: `RulesClass::ReadTypeData @ 0x00679C94..0x00679CAD`.
- Fresh read-only Ghidra decompile: `Random__RandomRanged @ 0x0065C7E0`.
- Fresh read-only Ghidra decompile/assembly context: `Math__ftol @ 0x007C5F00`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Current Rust scanned: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/rng.rs`, `src/sim/miner/miner_tests.rs`.
- Prior corroborating report: `docs/research/MISSIONENTER_RETRY_TIMER_STORAGE_AND_DISPATCH_GHIDRA_REPORT.md`.
- Prior `Math__ftol` control-word report: `docs/research/ADD_TIBERIUM_CREDITS_PURIFIER_VIRTUAL_PURIFIERS_GHIDRA_REPORT.md`.
