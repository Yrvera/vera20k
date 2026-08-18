# MissionClass Verb API — Guards, Override, and Restore Semantics

**Date:** 2026-06-02
**Target:** `Assign_Mission`, `Queue_Mission`, `Commence`, `Override_Mission`, `Restore_Mission` —
exact field-write semantics and interrupt guards.
**Trigger:** Prior swarm found 4 factual errors in `MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md`
§5.1.6. These verb claims are the direct input to the Rust scheduler spec and must be
re-verified before implementation.
**Confidence axes used:** (1) content — correct pseudocode; (2) identity — correct address;
(3) binding — vtable slot confirmed. All three required for HIGH.

---

## Investigation Plan (pre-flight)

**Target question:** Do the five verb functions match the §5.1.6 design-doc claims exactly,
including the "discard-current-when-queued" Override subtlety?

**Non-goals:** Dispatch switch decoding; ReadyToCommence subclass overrides (settled in
READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md); individual mission
handlers.

**Evidence needed to mark COMPLETE:**
- `decompile_function` for all five addresses.
- `get_assembly_context` confirming exact field offsets and CMP immediates from asm.
- `read_memory` on vtable+0x1E8..+0x1F8 confirming each slot→address binding.

**Stop conditions:** All five verbs VERIFIED or explicitly WRONG/UNVERIFIABLE with inline
Ghidra evidence.

---

## Vtable Binding Verification

`read_memory 0x007EDEA8` (= vtable base 0x007EDCC0 + 0x1E8), length 24 bytes:

```
e0 35 5b 00  70 35 5b 00  d0 2f 5b 00  50 36 5b 00  b0 36 5b 00  10 3a 5b 00
```

| vtable offset | slot address | Resolved (LE) | Expected | Match |
|---|---|---|---|---|
| +0x1E8 | 0x007EDEA8 | **0x005B35E0** | Queue_Mission | ✓ |
| +0x1EC | 0x007EDEAC | **0x005B3570** | Commence | ✓ |
| +0x1F0 | 0x007EDEB0 | **0x005B2FD0** | Assign_Mission | ✓ |
| +0x1F4 | 0x007EDEB4 | **0x005B3650** | Override_Mission | ✓ |
| +0x1F8 | 0x007EDEB8 | **0x005B36B0** | Restore_Mission | ✓ |

All five vtable bindings confirmed via `read_memory 0x007EDEA8` length=24.

---

## Per-Verb Analysis

### 1. `Assign_Mission` @ 0x005B2FD0 — vtable+0x1F0

**Design-doc claim (§5.1.6):** force `+0xAC=m`, clear queued/`+0xB8`/substate, reset timer
(`+0xC0/+0xC8 = now`, `+0xD0 = 0`). Guarded by Deliberate(0x1C)+Guard(5) only.

**Decompile evidence** (`decompile_function 0x005B2FD0`):

```c
void Assign_Mission(int param_1, int param_2) {
    if ((*(int*)(param_1+0xAC) != 0x1C) || (param_2 != 5)) {
        *(int*)(param_1+0xAC) = param_2;           // CurrentMission = m
        *(param_1+0xB4)       = 0xFFFFFFFF;        // QueuedMission = -1
        *(param_1+0xB8)       = 0;                 // IsCommenced = 0
        *(param_1+0xBC)       = 0;                 // MissionState = 0
        *(param_1+0xC0)       = g_CurrentFrame;    // MissionTimer.Start = now
        *(param_1+0xC4)       = 0;                 // MissionTickCounter = 0
        *(param_1+0xC8)       = g_CurrentFrame;    // DispatchTimer.Start = now
        *(param_1+0xCC)       = local_8;           // scratch (uninitialized)
        *(param_1+0xD0)       = 0;                 // DispatchTimer.Rate = 0
    }
}
```

**Assembly confirmation** (`get_assembly_context 0x005B2FD0`):
```
005b2fd9: CMP EAX,0x1c          ; CurrentMission == 0x1C?
005b2fe2: CMP EAX,0x5           ; param_2 == 5?
005b2fe9: MOV [ECX+0xac],EAX    ; +0xAC = m
005b2fef: MOV [ECX+0xb4],0xffffffff ; +0xB4 = -1
005b2ff9: MOV [ECX+0xb8],DL     ; +0xB8 = 0
005b2fff: MOV [ECX+0xbc],EDX    ; +0xBC = 0
005b3005: MOV EAX,[0xa8ed84]    ; load g_CurrentFrame
005b300a: MOV [ECX+0xc0],EAX    ; +0xC0 = now
005b3010: MOV [ECX+0xc4],EDX    ; +0xC4 = 0
005b3016: MOV EAX,[0xa8ed84]    ; load g_CurrentFrame again
005b301b: ADD ECX,0xc8
005b3021: MOV [ECX],EAX         ; +0xC8 = now
005b3023: MOV EAX,[ESP+0x4]     ; load local_8 (uninitialized scratch)
005b3027: MOV [ECX+0x4],EAX     ; +0xCC = local_8
005b302a: MOV [ECX+0x8],EDX     ; +0xD0 = 0
```

**Findings:**

| Claim | Result | Evidence |
|---|---|---|
| Guard: current==0x1C && m==5 → skip | **VERIFIED** | CMP EAX,0x1c at 0x005b2fd9; CMP EAX,0x5 at 0x005b2fe2 |
| No Selling(0x13) guard | **VERIFIED** | No 0x13 CMP in Assign_Mission; only Queue/Override have it |
| +0xAC = m | **VERIFIED** | MOV [ECX+0xac],EAX at 0x005b2fe9 |
| +0xB4 = -1 (clear queued) | **VERIFIED** | MOV [ECX+0xb4],0xffffffff at 0x005b2fef |
| +0xB8 = 0 | **VERIFIED** | MOV [ECX+0xb8],DL at 0x005b2ff9 |
| +0xBC = 0 (substate) | **VERIFIED** | MOV [ECX+0xbc],EDX at 0x005b2fff |
| +0xC0 = now | **VERIFIED** | MOV [ECX+0xc0],EAX (g_CurrentFrame) at 0x005b300a |
| +0xC4 = 0 (tick counter) | **VERIFIED** | MOV [ECX+0xc4],EDX at 0x005b3010 |
| +0xC8 = now | **VERIFIED** | MOV [ECX],EAX after ADD ECX,0xc8 at 0x005b3021 |
| +0xCC = local_8 (scratch, uninitialized) | **VERIFIED** | MOV [ECX+0x4],EAX with ESP+0x4 at 0x005b3027 |
| +0xD0 = 0 | **VERIFIED** | MOV [ECX+0x8],EDX at 0x005b302a |
| SuspendedMission (+0xB0) NOT written | **VERIFIED** | No MOV to ECX+0xB0 in body |

**Design-doc §5.1.6 claim for Assign_Mission:** FULLY VERIFIED.

**IMPORTANT NOTE on +0xCC:** Assign_Mission writes an uninitialized stack value to +0xCC.
This matches the design doc's `+0xCC scratch (uninitialized; dead)` annotation. The Rust
`MissionCom` struct may safely omit +0xCC or zero-initialize it.

---

### 2. `Queue_Mission` @ 0x005B35E0 — vtable+0x1E8

**Design-doc claim (§5.1.6):** write `+0xB4=m` (iff m!=-1 and not redundant), clear `+0xB8`;
if commence, call ReadyToCommence(+0x200) then Commence(). Reject if (current==0x1C && m==5)
or current==0x13.

**Decompile evidence** (`decompile_function 0x005B35E0`):

```c
void Queue_Mission(int* param_1, int param_2, char param_3) {
    int iVar1 = param_1[0x2b];  // +0xAC = current mission (0x2B * 4 = 0xAC)
    if (((iVar1 != 0x1C) || (param_2 != 5)) && (iVar1 != 0x13)) {
        if ((param_2 != -1) &&
            ((iVar1 != param_2) || ((param_1[0x2d] != param_2) && (param_1[0x2d] != -1)))) {
            // 0x2d * 4 = 0xB4; 0x2e * 4 = 0xB8 (as byte)
            param_1[0x2d] = param_2;   // +0xB4 = m
            *(byte*)(param_1+0x2e) = 0; // +0xB8 = 0
        }
        if (param_3 != 0) {
            cVar2 = (*vtable[+0x200])();   // ReadyToCommence
            if (cVar2 != 0) {
                (*vtable[+0x1EC])();       // Commence
            }
        }
    }
}
```

**Assembly confirmation** (`get_assembly_context 0x005B35E0`):
```
005b35e7: MOV ECX,[ESI+0xac]     ; load CurrentMission
005b35ed: CMP ECX,0x1c           ; guard: current == 0x1C?
005b35f2: CMP EAX,0x5            ; guard: m == 5?
005b35f7: CMP ECX,0x13           ; guard: current == 0x13?
005b35fc: CMP EAX,-0x1           ; m == -1? skip write
005b3601: CMP ECX,EAX            ; current == m?
005b3605: MOV ECX,[ESI+0xb4]     ; load QueuedMission
005b360b: CMP ECX,EAX            ; queued == m?
005b360f: CMP ECX,-0x1           ; queued == -1?
005b3614: MOV [ESI+0xb4],EAX     ; +0xB4 = m
005b361a: MOV [ESI+0xb8],0x0     ; +0xB8 = 0
005b362d: CALL [EAX+0x200]       ; vtable ReadyToCommence
005b363b: CALL [EDX+0x1ec]       ; vtable Commence
```

**Findings:**

| Claim | Result | Evidence |
|---|---|---|
| Guard: current==0x1C && m==5 → skip | **VERIFIED** | CMP ECX,0x1c at 0x005b35ed; CMP EAX,0x5 at 0x005b35f2 |
| Guard: current==0x13 → skip | **VERIFIED** | CMP ECX,0x13 at 0x005b35f7 |
| Skip write if m==-1 | **VERIFIED** | CMP EAX,-0x1 at 0x005b35fc |
| Skip write if current==m AND (queued==m OR queued==-1) | **VERIFIED** | CMPs at 0x005b3601, 0x005b360b, 0x005b360f |
| +0xB4 = m | **VERIFIED** | MOV [ESI+0xb4],EAX at 0x005b3614 |
| +0xB8 = 0 | **VERIFIED** | MOV [ESI+0xb8],0x0 at 0x005b361a |
| if commence: ReadyToCommence → Commence | **VERIFIED** | CALL [EAX+0x200] at 0x005b362d; CALL [EDX+0x1ec] at 0x005b363b |
| ReadyToCommence called via vtable (+0x200) | **VERIFIED** | Indirect call [EAX+0x200] |
| Commence called via vtable (+0x1EC) | **VERIFIED** | Indirect call [EDX+0x1ec] |

**Design-doc §5.1.6 claim for Queue_Mission:** FULLY VERIFIED.

**IMPORTANT NOTE on redundancy check:** The skip condition is:
`m != -1 AND NOT (current==m AND (queued==m OR queued==-1))`
Written out: write `+0xB4=m` unless m is already active AND (already queued OR queue is clear).
The design doc says "not redundant" — this matches exactly.

---

### 3. `Commence` @ 0x005B3570 — vtable+0x1EC

**Design-doc claim (§5.1.6):** if `+0xB4!=-1`: `+0xAC=+0xB4`, `+0xB4=-1`, reset substate/
`+0xB8`/timers, `+0xD0=0`, return true.

**Decompile evidence** (`decompile_function 0x005B3570`):

```c
undefined4 Commence(int param_1) {
    if (*(int*)(param_1+0xB4) != -1) {
        *(int*)(param_1+0xAC) = *(int*)(param_1+0xB4);  // current = queued
        *(param_1+0xB4)       = 0xFFFFFFFF;              // queued = -1
        *(param_1+0xBC)       = 0;                       // substate = 0
        *(param_1+0xC8)       = g_CurrentFrame;          // dispatch start = now
        *(param_1+0xCC)       = local_8;                 // scratch (uninit)
        *(param_1+0xD0)       = 0;                       // dispatch rate = 0
        *(param_1+0xC0)       = g_CurrentFrame;          // mission start = now
        *(param_1+0xC4)       = 0;                       // tick counter = 0
        *(param_1+0xB8)       = 0;                       // IsCommenced = 0
        return true;   // low byte = 1
    }
    return false;      // 0xFFFFFF00 (low byte = 0)
}
```

**Assembly confirmation** (`get_assembly_context 0x005B3570`):
```
005b3579: CMP EAX,-0x1          ; +0xB4 == -1?
005b357c: JZ  0x005b35cc        ; if so, jump to false return
005b357f: MOV [ECX+0xac],EAX    ; +0xAC = +0xB4 value
005b3585: MOV [ECX+0xb4],0xffffffff ; +0xB4 = -1
005b358f: MOV EAX,[0xa8ed84]    ; g_CurrentFrame
005b3594: LEA ESI,[ECX+0xc8]
005b359a: XOR EDX,EDX
005b359c: MOV [ECX+0xbc],EDX    ; +0xBC = 0
005b35a2: MOV [ESI],EAX         ; +0xC8 = now
005b35a4: MOV EAX,[ESP+0x8]     ; local_8 (uninit scratch)
005b35a8: MOV [ESI+0x4],EAX     ; +0xCC = local_8
005b35ab: MOV [ESI+0x8],EDX     ; +0xD0 = 0
005b35ae: MOV EAX,[0xa8ed84]    ; g_CurrentFrame again
005b35b3: MOV [ECX+0xc0],EAX    ; +0xC0 = now
005b35b9: MOV [ECX+0xc4],EDX    ; +0xC4 = 0
005b35bf: MOV [ECX+0xb8],DL     ; +0xB8 = 0
005b35c5: MOV AL,0x1            ; return true
005b35cc: XOR AL,AL             ; (false path) return false
```

**Findings:**

| Claim | Result | Evidence |
|---|---|---|
| Guard: +0xB4==-1 → return false | **VERIFIED** | CMP EAX,-0x1 + JZ at 0x005b3579/357c |
| +0xAC = +0xB4 | **VERIFIED** | MOV [ECX+0xac],EAX at 0x005b357f |
| +0xB4 = -1 | **VERIFIED** | MOV [ECX+0xb4],0xffffffff at 0x005b3585 |
| +0xBC = 0 (substate) | **VERIFIED** | MOV [ECX+0xbc],EDX at 0x005b359c |
| +0xC8 = now | **VERIFIED** | MOV [ESI],EAX (ESI=ECX+0xC8) at 0x005b35a2 |
| +0xD0 = 0 | **VERIFIED** | MOV [ESI+0x8],EDX at 0x005b35ab |
| +0xC0 = now | **VERIFIED** | MOV [ECX+0xc0],EAX at 0x005b35b3 |
| +0xC4 = 0 | **VERIFIED** | MOV [ECX+0xc4],EDX at 0x005b35b9 |
| +0xB8 = 0 | **VERIFIED** | MOV [ECX+0xb8],DL at 0x005b35bf |
| return true on success | **VERIFIED** | MOV AL,0x1 at 0x005b35c5 |
| return false if +0xB4==-1 | **VERIFIED** | XOR AL,AL at 0x005b35cc |
| SuspendedMission (+0xB0) NOT written | **VERIFIED** | No MOV to ECX+0xB0 in body |

**Design-doc §5.1.6 claim for Commence:** FULLY VERIFIED.

**IMPORTANT NOTE on return semantics:** `return 0xffffff00` with low byte 0 = false; high
bytes are garbage (from `uVar1` loaded with g_CurrentFrame earlier). Callers test only the
low byte (char/bool return convention). Rust must return `bool`.

---

### 4. `Override_Mission` @ 0x005B3650 — vtable+0x1F4

**Design-doc claim (§5.1.6):** if `+0xB4!=-1` → `+0xAC=m`, `+0xB0=+0xB4` (saves QUEUED,
prior current DISCARDED, `+0xB4` NOT cleared); else → `+0xB0=+0xAC`, `+0xAC=m`. Clear
`+0xB8`. Guards: current==0x1C && m==5 → skip; current==0x13 → skip.

**Decompile evidence** (`decompile_function 0x005B3650`):

```c
void Override_Mission(int param_1, int param_2) {
    int iVar1 = *(int*)(param_1+0xAC);   // load CurrentMission
    if (((iVar1 != 0x1C) || (param_2 != 5)) && (iVar1 != 0x13)) {
        if (*(int*)(param_1+0xB4) != -1) {
            // Branch A: queued mission exists
            *(int*)(param_1+0xAC) = param_2;              // current = m
            *(int*)(param_1+0xB0) = *(int*)(param_1+0xB4); // suspended = queued
            *(param_1+0xB8) = 0;                           // IsCommenced = 0
            return;
            // NOTE: +0xB4 is NOT cleared here
        }
        // Branch B: no queued mission
        *(int*)(param_1+0xB0) = iVar1;    // suspended = current
        *(int*)(param_1+0xAC) = param_2;  // current = m
        *(param_1+0xB8) = 0;              // IsCommenced = 0
    }
}
```

**Assembly confirmation** (`get_assembly_context 0x005B3650`):
```
005b365b: CMP EAX,0x1c          ; current == 0x1C?
005b3660: CMP ESI,0x5           ; m == 5?
005b3663: JZ  0x005b369f        ; skip (both conditions met)
005b3665: CMP EAX,0x13          ; current == 0x13?
005b3668: JZ  0x005b369f        ; skip
005b366a: MOV EDX,[ECX+0xb4]    ; load QueuedMission
005b3670: CMP EDX,-0x1          ; queued == -1?
005b3673: JZ  0x005b368c        ; branch B if no queued
; Branch A (queued != -1):
005b3675: MOV [ECX+0xac],ESI    ; +0xAC = m
005b367b: MOV [ECX+0xb0],EDX    ; +0xB0 = queued (+0xB4 value)
005b3681: MOV [ECX+0xb8],0x0    ; +0xB8 = 0
005b3688: POP ESI
005b3689: RET 0xc               ; returns; +0xB4 NOT touched
; Branch B (queued == -1):
005b368c: MOV [ECX+0xb0],EAX    ; +0xB0 = current (iVar1)
005b3692: MOV [ECX+0xac],ESI    ; +0xAC = m
005b3698: MOV [ECX+0xb8],0x0    ; +0xB8 = 0
```

**Findings:**

| Claim | Result | Evidence |
|---|---|---|
| Guard: current==0x1C && m==5 | **VERIFIED** | CMP EAX,0x1c at 0x005b365b; CMP ESI,0x5 at 0x005b3660 |
| Guard: current==0x13 → skip | **VERIFIED** | CMP EAX,0x13 at 0x005b3665 |
| Branch A: +0xB4!=-1 → +0xAC=m | **VERIFIED** | MOV [ECX+0xac],ESI at 0x005b3675 |
| Branch A: +0xB0=+0xB4 (saves queued) | **VERIFIED** | MOV [ECX+0xb0],EDX (EDX=prior +0xB4) at 0x005b367b |
| Branch A: prior current (+0xAC) DISCARDED | **VERIFIED** | No write to preserve iVar1; overwritten at 0x005b3675 |
| Branch A: +0xB4 NOT cleared | **VERIFIED** | RET 0xc at 0x005b3689 with no MOV to ECX+0xB4 in branch A |
| Branch B: +0xB4==-1 → +0xB0=+0xAC | **VERIFIED** | MOV [ECX+0xb0],EAX (EAX=iVar1=CurrentMission) at 0x005b368c |
| Branch B: +0xAC=m | **VERIFIED** | MOV [ECX+0xac],ESI at 0x005b3692 |
| +0xB8 = 0 in both branches | **VERIFIED** | MOV [ECX+0xb8],0x0 at 0x005b3681 and 0x005b3698 |
| No timer reset in Override | **VERIFIED** | No writes to +0xC0/+0xC4/+0xC8/+0xCC/+0xD0 |

**Design-doc §5.1.6 claim for Override_Mission:** FULLY VERIFIED.

**THE LOAD-BEARING SUBTLETY CONFIRMED:** When a queued mission exists (+0xB4 != -1),
Override saves the *queued* mission into suspended (+0xB0) and *discards* the current
mission (+0xAC overwritten, old value lost). Additionally, +0xB4 (queued) is **not
cleared** — so after Override returns with a queued mission, **both** +0xAC (new m) and
+0xB4 (old queued) are live. The Rust implementation must reproduce this exactly.

---

### 5. `Restore_Mission` @ 0x005B36B0 — vtable+0x1F8

**Design-doc claim (§5.1.6):** if `+0xB0!=-1`: `+0xAC=+0xB0`, `+0xB0=-1`, return true.

**Decompile evidence** (`decompile_function 0x005B36B0`):

```c
undefined4 Restore_Mission(int param_1) {
    int iVar1 = *(int*)(param_1+0xB0);   // SuspendedMission
    if (iVar1 != -1) {
        *(int*)(param_1+0xAC) = iVar1;   // current = suspended
        *(param_1+0xB0) = 0xFFFFFFFF;    // suspended = -1
        *(param_1+0xB8) = 0;             // IsCommenced = 0
        return true;   // AL = 1
    }
    return false;      // AL = 0
}
```

**Assembly confirmation** (`get_assembly_context 0x005B36B0`):
```
005b36b6: CMP EAX,-0x1           ; +0xB0 == -1?
005b36b9: JZ  0x005b36d5         ; jump to false return
005b36bb: MOV [ECX+0xac],EAX     ; +0xAC = +0xB0 value
005b36c1: MOV [ECX+0xb0],0xffffffff ; +0xB0 = -1
005b36cb: MOV [ECX+0xb8],0x0     ; +0xB8 = 0
005b36d2: MOV AL,0x1             ; return true
005b36d4: RET
005b36d5: XOR AL,AL              ; return false
005b36d7: RET
```

**Findings:**

| Claim | Result | Evidence |
|---|---|---|
| Guard: +0xB0==-1 → return false | **VERIFIED** | CMP EAX,-0x1 + JZ at 0x005b36b6/36b9 |
| +0xAC = +0xB0 | **VERIFIED** | MOV [ECX+0xac],EAX at 0x005b36bb |
| +0xB0 = -1 | **VERIFIED** | MOV [ECX+0xb0],0xffffffff at 0x005b36c1 |
| +0xB8 = 0 | **VERIFIED** | MOV [ECX+0xb8],0x0 at 0x005b36cb |
| return true on success | **VERIFIED** | MOV AL,0x1 at 0x005b36d2 |
| return false if no suspended | **VERIFIED** | XOR AL,AL at 0x005b36d5 |
| No timer reset | **VERIFIED** | No writes to +0xC0/+0xC4/+0xC8/+0xCC/+0xD0 |
| No guard on mission type | **VERIFIED** | No CMP before the +0xB0 check |

**Design-doc §5.1.6 claim for Restore_Mission:** FULLY VERIFIED with one addition.

**ADDITION NOT IN DESIGN DOC:** Restore_Mission also writes `+0xB8 = 0`. The design doc
§5.1.6 does not mention this write. The assembly at 0x005b36cb is unambiguous. The Rust
implementation must include this write.

---

## Per-Claim Summary Table

| Verb | Design-doc claim | Verdict | Evidence |
|---|---|---|---|
| `Assign_Mission` guard | current==0x1C && m==5 only | **VERIFIED** | CMP 0x1c/0x5 at 0x005b2fd9/2fe2 |
| `Assign_Mission` fields | +0xAC/B4/B8/BC/C0/C4/C8/CC/D0 | **VERIFIED** | asm 0x005b2fe9..302a |
| `Queue_Mission` guards | 0x1C+5 and 0x13 | **VERIFIED** | CMP 0x1c/0x5/0x13 at 0x005b35ed/f2/f7 |
| `Queue_Mission` redundancy | skip if m==-1 or already-current+queued | **VERIFIED** | CMP chain at 0x005b35fc..360f |
| `Queue_Mission` field writes | +0xB4=m, +0xB8=0 | **VERIFIED** | MOV at 0x005b3614/361a |
| `Queue_Mission` ReadyToCommence | vtable[+0x200] | **VERIFIED** | CALL [EAX+0x200] at 0x005b362d |
| `Queue_Mission` Commence | vtable[+0x1EC] | **VERIFIED** | CALL [EDX+0x1ec] at 0x005b363b |
| `Commence` guard | +0xB4==-1 → return false | **VERIFIED** | CMP/JZ at 0x005b3579/357c |
| `Commence` field writes | +0xAC/B4/BC/B8/C0/C4/C8/CC/D0 | **VERIFIED** | asm 0x005b357f..35bf |
| `Override_Mission` guards | 0x1C+5 and 0x13 | **VERIFIED** | CMP at 0x005b365b/3660/3665 |
| `Override_Mission` Branch A | queued→suspended, prior-current discarded, +0xB4 NOT cleared | **VERIFIED** | MOV at 0x005b3675/367b, RET 0x005b3689 |
| `Override_Mission` Branch B | current→suspended, +0xAC=m | **VERIFIED** | MOV at 0x005b368c/3692 |
| `Override_Mission` no timer reset | no +0xC0/C4/C8/CC/D0 writes | **VERIFIED** | full asm scan |
| `Restore_Mission` guard | +0xB0==-1 → return false | **VERIFIED** | CMP/JZ at 0x005b36b6/36b9 |
| `Restore_Mission` fields | +0xAC=+0xB0, +0xB0=-1, **+0xB8=0** | **VERIFIED** (doc missing +0xB8) | MOV at 0x005b36bb/36c1/36cb |
| `Restore_Mission` no timer reset | no +0xC0/C4/C8/CC/D0 writes | **VERIFIED** | full asm scan |

---

## One Discrepancy with Design-Doc §5.1.6

**`Restore_Mission` — +0xB8 write omitted from the design doc:**

The design doc §5.1.6 says: `if +0xB0!=-1: +0xAC=+0xB0, +0xB0=-1, return true.`

The binary also writes `+0xB8 = 0` (asm 0x005b36cb: `MOV byte ptr [ECX+0xb8],0x0`).

This is a minor omission in the doc, not a behavioral error — the Rust `restore_mission`
function must include `com.is_commenced = false` (i.e. set +0xB8 to 0) alongside the
field moves.

---

## Implementation Handoff

### Handoff Chain 1 — Override_Mission discard-current-when-queued

- **Verified verb semantics:** Branch A of Override (queued != -1): write `+0xAC = m`,
  write `+0xB0 = +0xB4` (old queued), do NOT clear +0xB4, do NOT save old +0xAC.
  Confirmed at asm 0x005b3675/367b; RET at 0x005b3689 without touching +0xB4.
- **Rust verb fn delta:** `override_mission` in `sim/mission/mod.rs` must implement both
  branches. Branch A: `com.suspended = com.queued; com.current = m; /* queued stays */`.
  Branch B: `com.suspended = com.current; com.current = m`. Never `com.queued = None` in
  Override.
- **Affected surface:** Any caller that overrides and then restores will get back the queued
  mission (not the prior current). If Rust collapses this to "save current always", it will
  restore the wrong mission when a queued mission was in flight at override time.
- **Acceptance scenario:** Unit A has current=Guard(5), queued=Attack(1). `Override(Move(2))`
  fires. Expected post-state: current=Move(2), suspended=Attack(1), queued=Attack(1).
  `Restore()` → current=Attack(1). Misimplementation restores Guard(5).
- **Proposed test name:** `test_override_mission_saves_queued_not_current`
- **Risk:** HIGH — wrong Restore target. Any unit override during mid-transition (command
  queued but not yet commenced) restores to the wrong mission.

### Handoff Chain 2 — Restore_Mission must clear +0xB8

- **Verified verb semantics:** Restore writes +0xB8=0 (asm 0x005b36cb), not mentioned in
  design doc §5.1.6.
- **Rust verb fn delta:** `restore_mission` must include `com.is_commenced = false`.
- **Affected surface:** If omitted, `+0xB8` retains whatever state the overriding mission
  left. In the Rust model this means `MissionCom.is_commenced` stays `true` after restore,
  which could cause the handler to skip its re-init branch if it checks that flag.
- **Proposed test name:** `test_restore_mission_clears_is_commenced`
- **Risk:** MEDIUM — behavioral only if a mission handler inspects `+0xB8` on re-entry
  after an override+restore cycle.

### Handoff Chain 3 — +0xCC scratch write (Assign + Commence)

- **Verified verb semantics:** Both Assign_Mission and Commence write an uninitialized
  stack value to +0xCC (asm 0x005b3027 and 0x005b35a8). This is the `DispatchTimer_??`
  field in the anchor doc, noted as "uninitialized; dead".
- **Rust verb fn delta:** `MissionCom` does not need to model +0xCC. The write is an
  artifact of the CDTimer struct being copied block-wise (3 × DWORD at ECX+0xC8);
  +0xCC is the middle DWORD and gamemd never reads it. Omitting it from `MissionCom`
  is correct.
- **Proposed test name:** no test needed — confirmed dead field.
- **Risk:** NONE if omitted from Rust struct.

---

## Negative Facts / Do Not Do

1. **Do NOT implement a Selling(0x13) guard in Assign_Mission.** Only Queue_Mission and
   Override_Mission check for 0x13. Assign_Mission has only the 0x1C+Guard(5) guard.
   (Confirmed: no CMP 0x13 in Assign_Mission body; asm 0x005b2fd9 has only CMP EAX,0x1c.)

2. **Do NOT clear +0xB4 (QueuedMission) in Override_Mission Branch A.** The queued mission
   is intentionally preserved after Override when a queued mission was live. (Confirmed:
   RET at 0x005b3689 without MOV to ECX+0xB4 in Branch A.)

3. **Do NOT reset timers (+0xC0/+0xC4/+0xC8/+0xD0) in Override_Mission or Restore_Mission.**
   Neither function touches timer fields. (Confirmed: full asm scan of both functions shows
   no writes to those offsets.)

4. **Do NOT omit the +0xB8 write from Restore_Mission.** The design doc §5.1.6 omits it,
   but the binary writes `MOV byte ptr [ECX+0xb8],0x0` at 0x005b36cb.

5. **Do NOT give Restore_Mission any interrupt guard on mission type.** Unlike Queue and
   Override, Restore has no CMP before the +0xB0 check — it restores unconditionally.
   (Confirmed: first instruction after entry is `CMP EAX,-0x1` on +0xB0 only.)

---

## Remaining Uncertainty

None for the five verb functions verified here. All offsets, guards, and return values are
confirmed by direct assembly inspection.

The following are **out of scope** for this report (already settled or deferred):
- ReadyToCommence subclass overrides: settled in READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md.
- Per-mission handlers: out of scope per investigation plan.
- +0xCC exact semantics: confirmed dead/uninitialized; no action needed.

---

## Active in YR

All five verbs are called on every active techno every time a mission transition occurs.
`TechnoClass::AI_Update @ 0x006F9E50` is the sole caller of `Mission_Dispatch`; the verb
functions are called from callers throughout the codebase (attack commands, dock sequences,
retaliation, etc.). **Active: Yes** for all five.

---

## Sources

- `decompile_function 0x005B2FD0` (Assign_Mission)
- `decompile_function 0x005B35E0` (Queue_Mission)
- `decompile_function 0x005B3570` (Commence)
- `decompile_function 0x005B3650` (Override_Mission)
- `decompile_function 0x005B36B0` (Restore_Mission)
- `get_assembly_context 0x005B2FD0` — field-write and guard asm confirmation
- `get_assembly_context 0x005B35E0` — Queue_Mission full asm confirmation
- `get_assembly_context 0x005B3570` — Commence full asm confirmation
- `get_assembly_context 0x005B3650` — Override_Mission full asm confirmation (from context_after)
- `get_assembly_context 0x005B36B0` — Restore_Mission full asm confirmation
- `read_memory 0x007EDEA8` length=24 — vtable slots +0x1E8..+0x1F8 binding confirmation
- Anchor doc: `docs/research/MISSIONCLASS_STATE_MACHINE.md`
- Design doc: `docs/research/MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md` §5.1.6
