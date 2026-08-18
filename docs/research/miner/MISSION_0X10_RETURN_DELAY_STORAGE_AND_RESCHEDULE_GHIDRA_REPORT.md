# Mission 0x10 Return Delay Storage And Reschedule - Ghidra Research Report

**Address(es):** `0x005B3060`, `0x005B3A00`, `0x0073D630`, `0x0073DF56..0x0073DFBC`, `0x0073E289..0x0073E2BE`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** how `MissionClass::Mission_Dispatch` stores and consumes return delays from `UnitClass::Mission_Deploy_Building` when current mission id is `0x10`, including the direct `return 5` facing-wait branch and the timer epilogue reached after accepted unload-start/state handoff.  
**Non-Scope:** dock radio `0x16`, PathType polarity beyond the already-gated `return 5` path, unload accumulator math (`+0xF8/+0x104/+0x110`), cargo credit arithmetic, and Rust implementation patches.  
**Confidence:** High for static storage/reschedule mechanism and branch return values; Medium for exact wall-clock replay frame in a full game because concrete frame ordering still depends on object iteration and `g_CurrentFrameCounter` sampling.  
**Active in YR:** Yes. Stock refinery `BuildingClass::Receive_Radio(0x15)` queues mission `0x10` for dock-unload units; `MissionClass::Mission_Dispatch @ 0x005B3060` dispatches mission id `0x10` through vtable slot `+0x23C` to `UnitClass::Mission_Deploy_Building @ 0x0073D630`.

## 1. Overview

`UnitClass::Mission_Deploy_Building` does not self-poll every tick after it returns. It returns an integer delay to `MissionClass::Mission_Dispatch`; dispatch writes the current frame to `MissionClass+0xC8` and the returned delay to `MissionClass+0xD0`, then later re-enters the mission only when `g_CurrentFrameCounter - +0xC8 >= +0xD0`.

For mission `0x10`, the facing-not-ready branch direct-returns `5` and consumes no random number. Accepted unload-start reaches the shared mission timer epilogue, which returns `ftol(MissionControl[0x10].Rate * 900.0) + RandomRanged(0,2)`. Stock `[Unload] Rate=.016` therefore schedules the next mission `0x10` dispatch after `14..16` frames.

## 2. Class Layout / Key Offsets

| Offset / data | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `MissionClass+0x90` | active/limbo byte gate; zero exits before timer check | `0x005B306C..0x005B3074` | Yes |
| `MissionClass+0x6C` | positive/alive gate before mission switch | `0x005B30A7..0x005B30AC` | Yes |
| `MissionClass+0xAC` | current mission id; `0x10` dispatches vtable slot `+0x23C` | `0x005B30B2`, `0x005B3260` | Yes |
| `MissionClass+0xC8` | mission timer start frame | read `0x005B307A`, store `0x005B3271` | Yes |
| `MissionClass+0xCC` | middle dispatch dword written from a caller stack local after handler return; not read by the due check | store `0x005B3273..0x005B3277`; due check reads only `+0xC8/+0xD0` | Yes, but scheduling role not observed |
| `MissionClass+0xD0` | mission timer duration, exactly the handler return value | read `0x005B3080`, store `0x005B327A` | Yes |
| `DAT_00A8ED84` | global current frame counter used by dispatcher | read `0x005B3091`, store source `0x005B326A` | Yes |
| `DAT_00A8E3A8` | mission-control table base, entry stride `0x20` | `0x005B3A00` | Yes |
| mission entry `+0x10` | `Rate` double consumed by timer epilogue | `0x0073E28C..0x0073E295` | Yes |
| `UnitClass+0x388` | primary facing `RateTimer` read by facing gate | `0x0073DF5A..0x0073DF65` | Yes |
| `UnitClass+0x6AF` | skip-Do_Turn byte in facing-wait branch | `0x0073DF7D..0x0073DF84` | Yes |
| `UnitClass+0x6D1` | dock/unload active latch | clear on no-path cleanup `0x0073DEF8`; set at unload-start `0x0073DFD6..0x0073DFE2` | Yes |

## 3. Core Logic

### 3.1 Dispatcher Due Check

`MissionClass::Mission_Dispatch @ 0x005B3060` first runs `ObjectClass::AI`, then tests `+0x90`. If active, it reads timer start `+0xC8` into `EDX` and duration `+0xD0` into `EAX`.

Assembly-confirmed gate:

```text
005b307a: MOV  EDX,dword ptr [EDI+0xc8]
005b3080: MOV  EAX,dword ptr [EDI+0xd0]
005b3086: LEA  ESI,[EDI+0xc8]
005b308c: CMP  EDX,-0x1
005b308f: JZ   0x005b309f
005b3091: MOV  ECX,dword ptr [0x00a8ed84]
005b3097: SUB  ECX,EDX
005b3099: CMP  ECX,EAX
005b309b: JGE  0x005b30a7
005b309d: SUB  EAX,ECX
005b309f: TEST EAX,EAX
005b30a1: JNZ  0x005b34e0
```

Mechanism:

```text
if start_frame != -1:
    elapsed = g_CurrentFrameCounter - start_frame
    if elapsed < duration:
        return without calling current mission
if duration != 0 after the check:
    return without calling current mission
call mission handler
```

The timer is passive. Nothing decrements `+0xD0` in this function. The re-entry boundary is inclusive: `elapsed >= duration` allows dispatch.

Active in YR: Yes. `TechnoClass::AI_Update @ 0x006F9E50` is the sole caller returned by Ghidra for `MissionClass::Mission_Dispatch`, and stock units run through this AI path.

### 3.2 Mission 0x10 Dispatch Storage

For mission id `0x10`, the jump table reaches `0x005B3260` and calls vtable slot `+0x23C`.

Assembly-confirmed storage:

```text
005b3260: MOV  EDX,dword ptr [EDI]
005b3262: MOV  ECX,EDI
005b3264: CALL dword ptr [EDX+0x23c]
005b326a: MOV  ECX,dword ptr [0x00a8ed84]
005b3271: MOV  dword ptr [ESI],ECX      ; +0xC8 = current frame
005b3273: MOV  ECX,dword ptr [ESP+0x8]
005b3277: MOV  dword ptr [ESI+0x4],ECX  ; +0xCC = stack local
005b327a: MOV  dword ptr [ESI+0x8],EAX  ; +0xD0 = handler return delay
```

Raw bytes for this site, read from Ghidra program memory:

```text
8b178bcfff923c0200008b0d84eda8005f890e8b4c2408894e048946085e83c40cc3
```

Therefore both direct `return 5` and timer-epilogue returns are stored identically: `+0xC8 = current frame`, `+0xD0 = returned delay`. There is no special immediate-reschedule path for mission `0x10`.

Active in YR: Yes. Mission id `0x10` is `[Unload]` in stock `rulesmd.ini`, and dock-unload refineries use this mission for harvesters/miners.

### 3.3 Direct Return 5 Branch

Inside `UnitClass::Mission_Deploy_Building`, after `PathType::Has_Valid_Steps()` succeeds, the function reads `RateTimer::Current(Unit+0x388)` and applies the accepted-facing window:

```text
accepted if (((current >> 7) + 1) & 0x1FE) == 0x80
```

If not accepted:

1. If `Unit+0x6AF == 0`, it writes stack word `0x4000` and calls active locomotor vtable slot `+0x4C`.
2. It returns literal `5`.
3. It does not call `MissionClass::GetMissionTimerEntry`.
4. It does not call `RandomRanged`.

The relevant byte range was read and disassembled at `0x0073DF56..0x0073DFBC`; key opcodes include:

```text
0073df56: LEA  EAX,[ESP+0x4c]
0073df5a: LEA  ECX,[ESI+0x388]
0073df60: PUSH EAX
0073df61: CALL 0x004c93d0        ; RateTimer::Current
...
0073df78: CMP  ECX,0x80
0073df7e: JZ   0x0073dfbd        ; accepted -> unload-start path
...
0073df9c: CALL dword ptr [EDX+0x4c]
0073dfb0: MOV  EAX,0x5
0073dfbc: RET
```

Active in YR: Yes. This is the live stock refinery unload `Mission_Deploy_Building` path for both HARV and CMIN once mission `0x10` has been queued and valid path steps exist.

### 3.4 Accepted Unload-Start Timer Epilogue

If the facing window is accepted and `Unit+0x6D1 == 0`, unload-start initializes the dock/unload latch and accumulator cluster, writes state `3`, then falls through to the common timer epilogue.

Key unload-start ordering at `0x0073DFC4..0x0073E09D`:

```text
MOV dword ptr [ESI+0xf8],0
MOV byte ptr [ESI+0x6d1],1
MOV EAX,dword ptr [0x00a8ed84]
LEA EDX,[ESI+0x100]
MOV dword ptr [ESI+0x10c],1
MOV dword ptr [EDX],EAX       ; +0x100 = current frame
MOV EAX,dword ptr [ESP+0x78]
MOV dword ptr [EDX+4],EAX     ; +0x104 = stack value, exact source non-scope
MOV dword ptr [EDX+8],1       ; +0x108 = 1
...
MOV dword ptr [ESI+0xbc],3    ; Unit mission local state = 3
```

The timer epilogue at `0x0073E289..0x0073E2BE` is:

```text
0073e289: MOV  ECX,ESI
0073e28b: CALL 0x005b3a00          ; MissionClass::GetMissionTimerEntry
0073e290: FLD  qword ptr [EAX+0x10] ; Rate
0073e293: FMUL qword ptr [0x007e27f8] ; 900.0
0073e299: CALL 0x007c5f00          ; Math::ftol
0073e29e: PUSH 0x2
0073e2a0: MOV  ESI,EAX
0073e2a1: PUSH 0x0
0073e2ad: CALL 0x0065c7e0          ; RandomRanged(0,2)
0073e2b2: ADD  EAX,ESI
0073e2be: RET
```

Raw bytes for the epilogue:

```text
8bcee87057e7ffdd4010dc0df8277e00e8627c08006a028bf06a008b0d30b2a80081c118020000e82be5f1ff03c65f5e5d5b83c470c3
```

`MissionClass::GetMissionTimerEntry @ 0x005B3A00` is:

```text
005b3a00: MOV EAX,dword ptr [ECX+0xac]
005b3a06: SHL EAX,0x5
005b3a09: ADD EAX,0x00a8e3a8
005b3a0e: RET
```

For mission id `0x10`, this uses mission-control entry `DAT_00A8E3A8 + 0x200`, field `+0x10`. Stock YR `[Unload] Rate=.016`, so `ftol(.016 * 900.0) = 14`, then `RandomRanged(0,2)` makes `14`, `15`, or `16`.

Active in YR: Yes. This epilogue is reached by live `Mission_Deploy_Building` paths after accepted unload-start and normal state-4 handoff.

### 3.5 When Mission 0x10 Can Run Again

After either branch returns to `Mission_Dispatch`:

| Branch | Handler return | RNG consumed by branch? | Stored `+0xC8` | Stored `+0xD0` | Earliest re-entry |
|---|---:|---|---|---:|---|
| Facing/rate not ready | `5` | No | current `g_CurrentFrameCounter` | `5` | first later dispatch where elapsed frames `>= 5` |
| Accepted unload-start timer epilogue | `14..16` stock | Yes, one `RandomRanged(0,2)` | current `g_CurrentFrameCounter` | `14..16` | first later dispatch where elapsed frames `>= returned delay` |
| No-valid-steps cleanup | `1` | No | current `g_CurrentFrameCounter` | `1` | next eligible frame after one-frame delay |
| State 3 positive drain / empty transition direct return | `1` | No | current `g_CurrentFrameCounter` | `1` | next eligible frame after one-frame delay |

The dispatcher due check runs before mission handler entry. A handler that returns `5` does not run again on the next AI pass unless the global frame counter has already advanced by at least five since the stored start frame.

## 4. INI Keys

| Key / section | Stock YR value | Binary reader/effect | Active in YR |
|---|---:|---|---|
| `[Unload] Rate` | `.016` | Mission-control entry `+0x10`, consumed by `Mission_Deploy_Building` timer epilogue through `GetMissionTimerEntry` | Yes |
| `[Unload] Recruitable` | `no` | Mission-control metadata, not part of return-delay scheduling in this slice | Yes, not scheduling-relevant here |
| `[Unload] Retaliate` | `no` | Mission-control metadata, not part of return-delay scheduling in this slice | Yes, not scheduling-relevant here |
| `[Unload] Scatter` | `no` | Mission-control metadata, not part of return-delay scheduling in this slice | Yes, not scheduling-relevant here |

YR `rulesmd.ini` has `[Unload] Rate=.016` at lines `30553..30557`. Base `rules.ini` matches.

## 5. Integration Points

| Function | Role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `TechnoClass::AI_Update @ 0x006F9E50` | sole static caller of `Mission_Dispatch` | Ghidra callers for `0x005B3060` | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | due check, vtable dispatch, return-delay storage | decompile plus assembly/bytes | Yes |
| `MissionClass::GetMissionTimerEntry @ 0x005B3A00` | computes `MissionControl[current_mission]` pointer | decompile plus bytes | Yes |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | mission `0x10` handler for stock refinery unload | decompile plus branch bytes | Yes |
| `RateTimer::Current @ 0x004C93D0` | facing gate input | callee in `Mission_Deploy_Building` | Yes |
| active locomotor vtable `+0x4C` | facing-wait retry calls `Do_Turn(0x4000)` | `0x0073DF90..0x0073DFA8` | Yes |
| `Random::RandomRanged @ 0x0065C7E0` | timer epilogue jitter `0..2`; not used by direct return `5` | `0x0073E29E..0x0073E2B2` | Yes |

## 6. Current Rust Implementation Status

Current Rust has a Mission Enter retry timer but no general mission `0x10` dispatch timer surface:

| Rust surface | Current status vs this slice |
|---|---|
| `src/sim/miner/mod.rs` | stores `dock_enter_retry_start_frame` and `dock_enter_retry_duration` for Mission Enter only; no analogous mission `0x10` start/duration field |
| `src/sim/miner/miner_dock_sequence.rs::schedule_enter_retry` | models passive `+0xC8/+0xD0` style for `Enter` retry with stock `14..16` jitter |
| `src/sim/miner/miner_dock_sequence.rs::phase_pivoting` | checks facing acceptance every Rust dock tick; does not currently store a direct-return `5` delay before rechecking mission `0x10` |
| `src/sim/miner/miner_dock_sequence.rs::start_unload_deploy` | starts unload immediately after Rust pivot acceptance; it also directly sets body facing and emits `DockDeploy`, both outside this report's scheduling proof |
| `src/sim/world/mod.rs` | advances `binary_frame` at the start of `Simulation::tick`; gamemd's global frame ordering remains a cross-system timing concern |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MissionClass::Mission_Dispatch` due check | verified | decompile `0x005B3060`, assembly `0x005B307A..0x005B30A1` | none for this slice |
| mission id `0x10` dispatch slot | verified | decompile `0x005B3060`, assembly `0x005B3260..0x005B3281` | none |
| delay storage in `+0xC8/+0xD0` | verified | assembly `0x005B326A..0x005B327A` | none |
| `+0xCC` semantic | touched-not-exhausted | store at `0x005B3273..0x005B3277`; not read by due check | broader MissionClass field audit if needed |
| `GetMissionTimerEntry` pointer math | verified | decompile/bytes `0x005B3A00..0x005B3A0E` | none |
| facing-not-ready direct `return 5` | verified | decompile `0x0073D630`; bytes/disassembly range `0x0073DF56..0x0073DFBC` | none |
| direct `return 5` RNG absence | verified | branch returns before `0x0073E289` epilogue; callee list shows RNG only in epilogue and other non-scope paths | none |
| accepted unload-start reaches timer epilogue | verified | decompile `0x0073DFBD..0x0073E09D`, epilogue `0x0073E289..0x0073E2BE` | exact `+0x104` source belongs to another swarm slot |
| stock `[Unload] Rate` value | verified | `ini/rulesmd.ini:30553..30557`, epilogue field `+0x10` | none |
| concrete replay frame for first re-entry | deferred | static dispatch proof complete | runtime trace needed for exact object-order/frame-counter sample |

## 8. Open Questions - Final State Of The Investigation Log

- `[RESOLVED] OQ-01 - Does mission 0x10 use normal MissionClass dispatch storage? -> Yes, vtable `+0x23C` return is stored into `+0xD0` with `+0xC8=current_frame`.` (evidence: `0x005B3260..0x005B327A`)
- `[RESOLVED] OQ-02 - Is direct return 5 a local per-tick retry? -> No, it is stored by the dispatcher like any other return delay.` (evidence: `0x0073DFB0..0x0073DFBC`, `0x005B326A..0x005B327A`)
- `[RESOLVED] OQ-03 - Does direct return 5 consume RNG? -> No, it returns before the timer epilogue and does not call `RandomRanged`.` (evidence: `0x0073DF56..0x0073DFBC`; `RandomRanged` call at `0x0073E2AD`)
- `[RESOLVED] OQ-04 - How is accepted unload-start rescheduled? -> It uses `GetMissionTimerEntry`, `Rate*900`, `ftol`, and `RandomRanged(0,2)`.` (evidence: `0x0073E289..0x0073E2BE`)
- `[RESOLVED] OQ-05 - Which mission entry does accepted unload-start use? -> The current mission id at `+0xAC`; for this slice `0x10`, so `[Unload]`.` (evidence: `0x005B3A00`, `rulesmd.ini:30553`)
- `[RESOLVED] OQ-06 - Is the due boundary `>` or `>=`? -> `>=`; dispatch jumps into the mission when `elapsed >= duration`.` (evidence: `0x005B3099..0x005B309B`)
- `[RESOLVED] OQ-07 - Does `+0xD0` decrement? -> No decrement observed in `Mission_Dispatch`; it compares passive elapsed against stored duration.` (evidence: `0x005B307A..0x005B30A1`)
- `[RESOLVED] OQ-08 - What calls `Mission_Dispatch`? -> `TechnoClass::AI_Update` is the Ghidra-reported static caller.` (evidence: Ghidra callers of `0x005B3060`)
- `[RESOLVED] OQ-09 - Does `+0xCC` affect this delay gate? -> Not in this function; due check reads only `+0xC8` and `+0xD0`.` (evidence: `0x005B307A..0x005B30A1`)
- `[RESOLVED] OQ-10 - Is stock `[Unload]` rate enough to compute accepted unload-start delay? -> Yes for stock YR; `.016*900` truncates to `14`, plus jitter `0..2`.` (evidence: `rulesmd.ini:30553..30557`, `0x0073E290..0x0073E2B2`)
- `[DEFERRED] OQ-11 - What is the full semantic name/consumer set for `MissionClass+0xCC`?` (category: `out-of-scope`; reason: it is written by dispatcher but not used by return-delay storage/eligibility; next-step-if-pursued: run a MissionClass field-use audit)
- `[DEFERRED] OQ-12 - What exact replay frame does a specific miner re-enter on after accepted unload-start?` (category: `needs-runtime-debugger`; reason: static mechanism is verified, but concrete frame depends on object AI ordering and current frame counter sample; next-step-if-pursued: runtime trace `+0xC8/+0xD0/+0xAC` around `0x005B3060`)
- `[DEFERRED] OQ-13 - What exact source value is copied into `Unit+0x104` at unload-start?` (category: `out-of-scope`; reason: this belongs to the separate unload accumulator swarm slot; next-step-if-pursued: audit `0x0073DFC4..0x0073E005` stack setup)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Mission `0x10` handler return delay is stored passively as `start_frame=current`, `duration=return_value`; next dispatch waits until `elapsed >= duration` | `0x005B307A..0x005B30A1`, `0x005B3260..0x005B327A` | missing for deploy/unload; only Enter has this shape | `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock_sequence.rs` or a future mission scheduler | add mission-0x10-equivalent pending delay before rechecking pivot/unload state | `mission_0x10_return_delay_blocks_reentry_until_elapsed_gte_duration` | Do not poll Mission_Deploy every Rust tick after a gamemd handler return |
| Facing-not-ready branch returns literal `5` and consumes no RNG | `0x0073DF56..0x0073DFBC` | Rust pivot phase rechecks every tick | `phase_pivoting`, future deploy mission scheduler | when facing window is not accepted, schedule next deploy check after 5 binary frames | `mission_deploy_facing_not_ready_reschedules_five_without_rng` | Do not attach the `5` delay to PathType false or spend jitter RNG here |
| Accepted unload-start epilogue consumes `[Unload] Rate*900` plus inclusive jitter `0..2`; stock gives `14..16` | `0x0073E289..0x0073E2BE`, `rulesmd.ini:30553..30557` | Rust starts unload phase and then uses its own unload timer model | `start_unload_deploy`, `phase_unloading`, future mission scheduler | after accepted unload-start, next mission-0x10/state-3 dispatch is delayed by 14..16 stock frames and consumes one RNG draw | `mission_deploy_unload_start_reschedules_14_to_16_and_consumes_one_rng` | Do not treat accepted unload-start as immediately entering the first dump-gate pass |

### Stale Docs / Follow-up Docs

- Replace any wording that says the facing-not-ready branch "waits/polls next tick" with: "The branch direct-returns `5`; `Mission_Dispatch` stores that in `MissionClass+0xD0`, so mission `0x10` is not eligible again until elapsed frames are at least five."
- Replace any wording that says the mission timer "decrements" with: "The timer is passive: `Mission_Dispatch` stores `+0xC8` start frame and `+0xD0` duration, then compares `g_CurrentFrameCounter - +0xC8 >= +0xD0`."
- Clarify that accepted unload-start's timer epilogue uses the current mission entry. For mission `0x10`, stock `[Unload] Rate=.016` gives `14..16` frames, not a hardcoded delay.

## 10. Negative Facts / Do Not Do

- Do not implement the direct `return 5` as a local spin inside `phase_pivoting`; gamemd stores it through `Mission_Dispatch`.
- Do not consume `RandomRanged(0,2)` for the facing-not-ready retry; the RNG call is only in the timer epilogue path.
- Do not model `+0xD0` as a decrementing counter; dispatch leaves it as a duration and recomputes elapsed from `g_CurrentFrameCounter`.
- Do not use `MissionClass+0xCC` as the mission delay. It is written after every handler return but is not the due-check duration.
- Do not claim exact concrete replay frame parity from this static report alone; it proves the storage/reschedule mechanism, not object-order runtime sampling.

## Sources

- Ghidra read-only decompile/disassembly/bytes: `0x005B3060`, `0x005B3A00`, `0x0073D630`, `0x0073DF56..0x0073DFBC`, `0x0073E289..0x0073E2BE`.
- Ghidra callers/callees: `MissionClass::Mission_Dispatch` caller `TechnoClass::AI_Update @ 0x006F9E50`; `UnitClass::Mission_Deploy_Building` callees include `MissionClass::GetMissionTimerEntry`, `Math::ftol`, `Random::RandomRanged`, `RateTimer::Current`.
- Existing docs read/reconciled: `docs/research/MISSIONENTER_RETRY_TIMER_STORAGE_AND_DISPATCH_GHIDRA_REPORT.md`, `docs/research/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, `docs/research/STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`, `docs/research/UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan only: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/world/mod.rs`.

**Status:** COMPLETE for mission `0x10` return-delay storage and reschedule. Remaining uncertainties are outside this slot: full `+0x104` unload accumulator source, broader `+0xCC` naming/consumer audit, and concrete runtime replay frame sampling.
