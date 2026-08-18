# Phase 0 Chrono Delay Formula — Ghidra Report

**Target:** TeleportLocomotionClass Phase 0 self-teleport delay formula  
**Function:** TeleportLocomotionClass::StateMachineTick at 0x007192F0  
**Scope:** Exact delay calculation for chrono miner / chrono legionnaire self-warp  
**Date:** 2026-05-19  
**Status:** VERIFIED from live Ghidra decompilation + assembly trace

---

## 1. Executive Summary

The Phase 0 delay formula for self-teleport (chrono miner / chrono legionnaire) is:

```
distance_leptons = (int)sqrt(dx*dx + dy*dy + dz*dz)   // 3D Euclidean, ftol truncation

Step 1 — Raw delay:
  if ChronoTrigger (Rules+0xBF8, bool):
    raw_delay = distance_leptons / ChronoDistanceFactor   // signed integer IDIV, truncating
  else:
    raw_delay = 0

Step 2 — Remaining check:
  elapsed   = g_CurrentFrameCounter - timer.StartFrame   // timer was just set
  remaining = (elapsed >= raw_delay) ? 0 : raw_delay - elapsed
  // NOTE: since timer was set in the same call, elapsed ≈ 0, so remaining ≈ raw_delay

Step 3 — Minimum clamp:
  if remaining <= ChronoMinimumDelay (Rules+0xBFC):
    timer.StartFrame = g_CurrentFrameCounter
    timer.Duration   = ChronoMinimumDelay

Step 4 — Range clamp:
  if distance_leptons < ChronoRangeMinimum (Rules+0xC00):
    timer.StartFrame = g_CurrentFrameCounter
    timer.Duration   = ChronoMinimumDelay
```

**The result is stored in `locomotor.Timer.Duration` (TeleportLocomotionClass+0x44),
NOT in TechnoClass+0x284.** TechnoClass+0x284 (ChronoLockDuration) is used by the
Chronosphere path (Phase 3/5), not by self-teleport Phase 0.

**Active in YR: Yes.** ChronoTrigger defaults to `yes` in rulesmd.ini. The formula
fires every time a chrono miner or chrono legionnaire self-teleports.

---

## 2. INI Values (from ini/rulesmd.ini, both rules.ini and rulesmd.ini agree)

| INI Key | Rules Offset | Default | Type |
|---------|-------------|---------|------|
| ChronoDelay | +0xBEC | 60 | int (frames) |
| ChronoReinfDelay | +0xBF0 | 180 | int (frames) |
| ChronoDistanceFactor | +0xBF4 | 48 | int (leptons per frame-delay) |
| ChronoTrigger | +0xBF8 | yes | bool |
| ChronoMinimumDelay | +0xBFC | 16 | int (frames) |
| ChronoRangeMinimum | +0xC00 | 0 | int (leptons) |

The INI comment in rulesmd.ini states "default = 32" for ChronoDistanceFactor, but the
actual value in the file is 48. The 48 value is what is loaded at runtime.

Formula interpretation from INI comment: `256/ChronoDistanceFactor` frames per cell.
With factor=48: ≈5.33 frames per cell (256/48). With factor=32: 8 frames per cell.

---

## 3. Verified Assembly Trace

All addresses are in gamemd.exe (YR 1.001). EDI points to `param_1 + 0x38`
= TeleportLocomotionClass base+0x38 = the CDTimerClass triple (StartFrame/field4/Duration).

### Distance Calculation
```asm
; ... ftol result (distance in leptons) in EAX at this point
007194a4: ADD ESP, 0x8         ; clean sqrt args
007194a7: CALL 0x007C5F00      ; Math::ftol — truncate double→int
007194ac: LEA EDI, [ESI+0x38]  ; EDI = &locomotor.Timer.StartFrame
007194af: MOV EDX, EAX         ; EDX = distance (leptons, int)
```

### Default Timer Set (Duration=0 before ChronoTrigger check)
```asm
007194b1: MOV EAX, [0x00A8ED84]   ; EAX = g_CurrentFrameCounter
007194b6: MOV EBX, EDI            ; EBX = &timer
007194b8: XOR ECX, ECX            ; ECX = 0 (default Duration)
007194ba: MOV [ESP+0x34], EDX     ; save distance to stack
007194be: MOV [EBX], EAX          ; timer.StartFrame = CurrentFrame
007194c0: MOV EAX, [ESP+0x24]     ; reload stale middle-field value
007194c4: MOV [EBX+0x4], EAX      ; timer.field_4 = stale (vestigial)
007194c7: MOV [EBX+0x8], ECX      ; timer.Duration = 0 (default)
```

### ChronoTrigger Branch
```asm
007194ca: MOV EBX, [0x008871E0]   ; EBX = g_RulesClass_Instance
007194d0: MOV AL, [EBX+0xBF8]     ; AL = Rules->ChronoTrigger (bool byte)
007194d6: TEST AL, AL
007194d8: JZ 0x007194FD           ; if ChronoTrigger==0, skip division (Duration stays 0)

; ChronoTrigger==1 path:
007194da: MOV EAX, EDX            ; EAX = distance (leptons)
007194dc: MOV ECX, [0x00A8ED84]   ; ECX = g_CurrentFrameCounter
007194e2: CDQ                      ; sign-extend EAX into EDX:EAX
007194e3: IDIV [EBX+0xBF4]        ; signed divide by Rules->ChronoDistanceFactor
                                   ; quotient → EAX, remainder → EDX (discarded)
007194e9: MOV EDX, EDI            ; EDX = &timer (overwrites IDIV remainder)
007194eb: MOV [EDX], ECX          ; timer.StartFrame = g_CurrentFrameCounter (reset)
007194ed: MOV ECX, [ESP+0x24]     ; ECX = stale stack value
007194f1: MOV [EDX+0x4], ECX      ; timer.field_4 = stale (vestigial)
007194f4: MOV [EDX+0x8], EAX      ; timer.Duration = quotient = distance / ChronoDistanceFactor
```

**IDIV confirmed:** signed integer division, truncating (floor toward zero). No rounding.

### Remaining Computation (both branches converge here at 0x7194FD)
```asm
007194f7: MOV EBX, [0x008871E0]   ; EBX = g_RulesClass_Instance
007194fd: MOV EDX, [EDI]          ; EDX = timer.StartFrame
007194ff: MOV EAX, [EDI+0x8]      ; EAX = timer.Duration
00719502: CMP EDX, -1
00719505: JZ 0x00719519           ; if StartFrame==-1, skip (treat remaining=Duration)
00719507: MOV ECX, [0x00A8ED84]   ; ECX = g_CurrentFrameCounter
0071950d: SUB ECX, EDX            ; ECX = elapsed = CurrentFrame - StartFrame
0071950f: CMP ECX, EAX            ; elapsed vs Duration
00719511: JGE 0x00719517          ; if elapsed >= Duration: remaining=0
00719513: SUB EAX, ECX            ; remaining = Duration - elapsed
00719515: JMP 0x00719519
00719517: XOR EAX, EAX            ; remaining = 0
```
(Since StartFrame was just set to CurrentFrame, elapsed=0, remaining=Duration.)

### Minimum Delay Clamp
```asm
00719519: MOV EBX, [EBX+0xBFC]    ; EBX = Rules->ChronoMinimumDelay
0071951f: CMP EAX, EBX            ; remaining vs ChronoMinimumDelay
00719521: JLE 0x00719527          ; if remaining <= minimum → apply minimum
00719523: MOV EAX, EDI            ; remaining > minimum: keep existing timer (no change)
00719525: JMP 0x00719539

; "Apply minimum" branch (0x719527):
00719527: MOV EDX, [0x00A8ED84]   ; EDX = g_CurrentFrameCounter
0071952d: MOV [ESP+0x1C], EBX     ; push ChronoMinimumDelay as new Duration to stack
00719531: MOV [ESP+0x14], EDX     ; push CurrentFrame as new StartFrame to stack

; Common path (0x719539) — copy from source to timer:
00719539: MOV EDX, [EAX]          ; load StartFrame from chosen source
0071953b: MOV ECX, EDI
0071953d: MOV [ECX], EDX          ; timer.StartFrame = chosen StartFrame
0071953f: MOV EDX, [EAX+0x4]      ; load field_4
00719542: MOV [ECX+0x4], EDX      ; timer.field_4 = copied
00719545: MOV EAX, [EAX+0x8]      ; load Duration from chosen source
0071954c: MOV [ECX+0x8], EAX      ; timer.Duration = chosen Duration
```

The `JLE` is **signed**. Since remaining and ChronoMinimumDelay are both non-negative in
normal play, this is equivalent to `<=`. Any negative remaining would also apply the minimum.

### ChronoRangeMinimum Check (runs AFTER the above, distance is re-loaded)
```asm
00719548: MOV EDX, [ESP+0x34]     ; EDX = distance (saved earlier at 0x7194ba)
0071954f: MOV ECX, [0x008871E0]   ; ECX = g_RulesClass_Instance
00719555: CMP EDX, [ECX+0xC00]    ; distance vs Rules->ChronoRangeMinimum
0071955b: JGE 0x00719576          ; if distance >= ChronoRangeMinimum, skip override

; "Force minimum" branch (distance < ChronoRangeMinimum):
0071955d: MOV EAX, [0x00A8ED84]   ; EAX = g_CurrentFrameCounter
00719562: MOV ECX, [ECX+0xBFC]    ; ECX = Rules->ChronoMinimumDelay
00719568: MOV EDX, EDI
0071956a: MOV [EDX], EAX          ; timer.StartFrame = CurrentFrame
0071956c: MOV EAX, [ESP+0x24]     ; (stale middle field)
00719570: MOV [EDX+0x4], EAX      ; timer.field_4 = stale
00719573: MOV [EDX+0x8], ECX      ; timer.Duration = ChronoMinimumDelay
```

### BeingWarped Set (immediately after timer is finalized)
```asm
00719576: MOV ECX, [ESI+0x8]      ; ECX = LinkedTo (TechnoClass*)
00719579: MOV byte ptr [ECX+0x271], 0x1   ; TechnoClass->BeingWarped = 1
```

---

## 4. Edge Cases

### ChronoTrigger = false (Rules+0xBF8 == 0)
**Active in YR: Conditional (ChronoTrigger defaults to yes in rulesmd.ini)**

When ChronoTrigger is false: `JZ 0x7194FD` is taken → timer.Duration stays 0.
Then remaining=0 at the clamp check. `CMP 0, ChronoMinimumDelay` → `JLE` taken
(since 0 <= 16). Result: `timer.Duration = ChronoMinimumDelay` (default 16 frames).

**ChronoTrigger=false does NOT produce zero delay.** It produces ChronoMinimumDelay.
The INI comment "this value will also be used if the ChronoTrigger flag is turned off"
is correct and verified by the binary.

### Zero Distance
Distance = 0 leptons (teleporting to exact current location — the "no movement needed"
check at the top of Phase 0 should prevent this, but if it reaches the formula):
- `0 / ChronoDistanceFactor = 0` → remaining=0 → ChronoMinimumDelay applied.
- Result: 16 frames delay (with defaults).

### Very Large Distance / No Clamping
No maximum cap on delay is implemented. The only bounds check is the minimum. If
ChronoDistanceFactor=1 and distance=1,000,000 leptons, delay = 1,000,000 frames.
No overflow protection beyond the 32-bit int limit of IDIV.

### ChronoRangeMinimum = 0 (default)
When ChronoRangeMinimum=0 (default), the check `distance < 0` is never true for a
non-negative distance. The ChronoRangeMinimum override path is dead by default in YR.

### Bridge / Water Modifiers
No bridge or water modifier exists in the delay formula. The cell bridge flag affects
only `TechnoClass->IsOnBridge (+0x8C)` written at 0x71968A/0x71967C (after the timer
is set). The timer duration is unaffected by terrain type.

---

## 5. Where the Result is Stored

**Self-teleport (Phase 0):** The delay is stored in the **locomotor's own CDTimerClass**:
- `TeleportLocomotionClass+0x3C` = timer.StartFrame
- `TeleportLocomotionClass+0x44` = timer.Duration = calculated delay

The unit remains at `WarpPhase=0` with `BeingWarped=1`. Every subsequent tick hits the
pre-phase check (BeingWarped && phase==0 && PendingWarpPhase==0) and calls TimerCheck
(0x719BF0), which counts down and clears BeingWarped when expired.

**TechnoClass+0x284 is NOT used by self-teleport Phase 0.**
- +0x284 is set in Chronosphere Phase 3 (0x719983): written from `Rules->ChronoDelay`
- +0x284 is used in Chronosphere Phase 5 as the lockdown timer duration
- In self-teleport, +0x284 is never read or written during the Phase 0 delay sequence

---

## 6. Self-Teleport vs Chronosphere Path Distinction

| Aspect | Self-Teleport (Phase 0) | Chronosphere (Phases 0-7) |
|--------|------------------------|--------------------------|
| Trigger | IsMoving==1, ChronoInTransit==0, PendingWarpPhase==0 | ChronoInTransit==1 or PendingWarpPhase==3 |
| Delay stored | Locomotor Timer+0x44 | TechnoClass+0x284 initially ChronoReinfDelay, then overwritten with ChronoDelay |
| Delay formula | distance / ChronoDistanceFactor, clamped to ≥ChronoMinimumDelay | ChronoDelay (flat, from Rules+0xBEC) |
| Phase after warp | Stays at 0, TimerCheck each tick | Advances through phases 5→6→7 |
| WarpPhase at end | 0 (never incremented) | 0 (cleared in phase 7) |

---

## 7. Confidence Summary

| Claim | Evidence | Confidence |
|-------|---------|-----------|
| Formula: `IDIV [EBX+0xBF4]` (integer division) | Assembly at 0x7194e3 | 100% |
| ChronoTrigger read from Rules+0xBF8 | Assembly at 0x7194d0 | 100% |
| ChronoMinimumDelay read from Rules+0xBFC | Assembly at 0x719519, 0x719562 | 100% |
| ChronoRangeMinimum read from Rules+0xC00 | Assembly at 0x719555 | 100% |
| ChronoTrigger=false → ChronoMinimumDelay (not zero) | JZ path + JLE logic | 100% |
| Result in locomotor+0x44, NOT TechnoClass+0x284 | Assembly at 0x7194f4/0x719573; +0x284 writer is at 0x719983 (Phase 3) | 100% |
| No bridge/water modifier on delay | Full Phase 0 assembly traced | 100% |
| No upper clamp on delay | Full assembly traced | 100% |

---

## 8. Open Questions

1. **TechnoClass+0x284 for self-teleport**: Is +0x284 ever read during self-teleport's
   BeingWarped countdown (the TimerCheck at 0x719BF0)? The TimerCheck uses the locomotor
   timer, not +0x284. But confirming that TimerCheck never touches +0x284 would be clean.

2. **Infantry chrono-kill interaction**: When chrono-kill fires (infantry + ChronoKillInfantry),
   timer.Duration is set to 0 and BeingWarped is cleared to 0. But the minimum clamp block
   has already run by that point in Phase 0. Does the infantry kill code unconditionally
   override the timer, or does the minimum clamp re-fire? (The kill branch at 0x719588
   writes Duration=0 AFTER the clamp block — so yes, it unconditionally overrides it.)

3. **Subterranean/tunnel**: Out of scope per project memory (TS legacy, skip).

---

## 9. Rust Implementation Note

The existing overview doc (CHRONO_MINER_SYSTEM_OVERVIEW.md lines 194-258) correctly
describes the formula but underspecifies two points now verified:

1. "clamp to max(ChronoMinimumDelay, delay)" — The actual comparison is `remaining <= minimum`
   using `JLE` (signed), meaning even if remaining somehow went negative, the minimum is
   applied. In practice with non-negative inputs this is equivalent to `max(minimum, remaining)`.

2. "if distance < ChronoRangeMinimum: force minimum" — Runs AFTER the minimum clamp step,
   using the original pre-division distance, and overwrites whatever the clamp step produced.
   The two clamps are independent sequential checks, not a single conditional.
