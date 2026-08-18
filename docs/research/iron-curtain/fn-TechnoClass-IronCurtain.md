# TechnoClass::IronCurtain — Function Decode

**Address:** `0x0070e2b0`
**Kind:** function (`__thiscall`)
**Runbook:** function-decode-v1
**Verified via:** `decompile_function 0x0070e2b0`, `disassemble_function 0x0070e2b0`, `get_function_callers 0x0070e2b0`, `get_function_callees 0x0070e2b0`

---

## Summary

`TechnoClass::IronCurtain` is the base-class method that stamps Iron Curtain (or Force
Shield) state onto any `TechnoClass` instance. It records when the effect started, how
long it lasts, and whether it is a Force Shield rather than a standard Iron Curtain. It is
a leaf function — no callees — and is always reached through a subclass override
(`BuildingClass::IronCurtain`) or the IC dispatch function (`TechnoClass__StartFidget`
at `0x004deae4`, which is misnamed and acts as the per-techno IC application dispatcher).

---

## Active in YR

**Yes.** Both callers (`0x00457c90` and `0x004deae4`) are reachable from the standard
super-weapon firing path. Force Shield shares this exact code path via the `is_force_shield`
parameter.

---

## Signature

```c
void __thiscall TechnoClass__IronCurtain(
    void *this,
    int   duration,       // [ESP+4]  at call site before frame adjust
    int   source_house,   // [ESP+8]  received but NOT stored in TechnoClass fields
    int   is_force_shield // [ESP+C]  0 = Iron Curtain, non-0 = Force Shield
);
```

---

## Decompilation (Ghidra C-like, from `decompile_function 0x0070e2b0`)

```c
void __thiscall TechnoClass__IronCurtain(
    void *this, int duration, int source_house, int is_force_shield)
{
    undefined4 local_8;   // COMPILER ARTIFACT — see assembly note below

    *(undefined4 *)((int)this + 0x18c) = g_CurrentFrameCounter;
    *(undefined4 *)((int)this + 400)   = local_8;     // ← ARTIFACT: see §Assembly below
    *(undefined4 *)((int)this + 0x1a4) = 0;
    *(int *)       ((int)this + 0x194) = duration;
    if ((char)is_force_shield != '\0') {
        *(undefined4 *)((int)this + 0x1c4) = 1;
        return;
    }
    *(undefined4 *)((int)this + 0x1c4) = 0;
    return;
}
```

> `400 decimal = 0x190`. The `local_8` write is a Ghidra decompiler artifact — see
> §Assembly Analysis for the verified behavior.

---

## Assembly Analysis (`disassemble_function 0x0070e2b0`)

```asm
0070e2b0: MOV  EAX, [0x00a8ed84]      ; EAX = g_CurrentFrameCounter
0070e2b5: SUB  ESP, 0xc               ; allocate 12-byte local frame
0070e2b8: MOV  EDX, [ESP+0x10]        ; EDX = duration  (arg1, verified: (ESP_entry-0xC)+0x10 = ESP_entry+4)
0070e2bc: PUSH ESI                    ; ESP -= 4 (total frame shift now 0x10)
0070e2bd: LEA  ESI, [ECX+0x18c]       ; ESI = &this->+0x18c
0070e2c3: MOV  [ECX+0x18c], EAX       ; this->+0x18c = g_CurrentFrameCounter
0070e2c9: MOV  EAX, [ESP+0x8]         ; EAX = local stack slot (source_house NOT stored — see note)
0070e2cd: MOV  [ESI+0x4], EAX         ; this->+0x190 = local stack (uninitialized / garbage)
0070e2d0: XOR  EAX, EAX
0070e2d2: MOV  [ECX+0x1a4], EAX       ; this->+0x1a4 = 0
0070e2d8: MOV  [ESI+0x8], EDX         ; this->+0x194 = duration
0070e2db: MOV  DL,  [ESP+0x1c]        ; DL = is_force_shield ([ESP+0x1c] = original [ESP_entry+0xC] ✓)
0070e2df: CMP  DL, AL                 ; compare with 0
0070e2e1: POP  ESI
0070e2e2: JZ   0x0070e2f4             ; if is_force_shield == 0 → branch
0070e2e4: MOV  [ECX+0x1c4], 0x1       ; this->+0x1c4 = 1 (Force Shield)
0070e2ee: ADD  ESP, 0xc
0070e2f1: RET  0xc
0070e2f4: MOV  [ECX+0x1c4], EAX       ; this->+0x1c4 = 0 (Iron Curtain)
0070e2fa: ADD  ESP, 0xc
0070e2fd: RET  0xc
```

**Frame verification (entry ESP = X):**
After `SUB ESP,0xC`: ESP = X−0xC.
- `[ESP+0x10]` = `[X+4]` = arg1 = **duration** ✓

After `PUSH ESI`: ESP = X−0x10.
- `[ESP+0x1C]` = `[X+0xC]` = arg3 = **is_force_shield** ✓
- `[ESP+0x8]`  = `[X−0x8]`  = **below entry ESP** — this is unused local stack space, NOT `source_house` (which would be at `[X+8]` = `[ESP+0x18]`).

**Conclusion on `+0x190`:** The write `[ECX+0x190] = [ESP+0x8]` stores garbage from the
callee's own local frame — this slot is never explicitly initialized. This appears to be a
dead write (probably a vestigial `source_house` storage that was removed or never
connected). The field `+0x190` is written but carries no defined value from this function.

The `source_house` parameter (`[X+8]`) is **received but never stored** by this function.

---

## Struct Field Accesses

All offsets are direct byte offsets from `this` (param is `void *` cast to `(int)this`).
Verified via `disassemble_function 0x0070e2b0`.

| Offset | Size | Access | Value | Semantic |
|--------|------|--------|-------|----------|
| `+0x18c` | 4 | write | `g_CurrentFrameCounter` | IC effect start frame (from `MOV [0x00a8ed84]`) |
| `+0x190` | 4 | write | **local stack garbage** | Vestigial/dead write — NOT `source_house` (see assembly analysis) |
| `+0x194` | 4 | write | `duration` (arg1) | IC effect duration in frames |
| `+0x1a4` | 4 | write | `0` | Cleared; purpose unknown from this function alone |
| `+0x1c4` | 4 | write | `0` or `1` | `1` = Force Shield active, `0` = Iron Curtain |

**Reference frame:** all offsets are from the `TechnoClass` instance pointer (NW — no
coordinate semantics here; these are state fields, not spatial offsets).

---

## Globals Referenced

| Address | Name | Access |
|---------|------|--------|
| `0x00a8ed84` | `g_CurrentFrameCounter` | read (via `MOV EAX,[0x00a8ed84]` at `0x0070e2b0`) |

---

## Callees

None. This is a leaf function (verified: `get_function_callees 0x0070e2b0` returned no callees).

---

## Callers (from `get_function_callers 0x0070e2b0`)

| Address | Name | Role |
|---------|------|------|
| `0x00457c90` | `BuildingClass__IronCurtain` | Building-specific override; handles building-internal IC state then delegates here |
| `0x004deae4` | `TechnoClass__StartFidget` (misnamed) | Actually the IC application dispatcher — applies IC to a single techno; calls this after pre-writing WarpAttach fields at `+0x6A0..+0x6A8` |

Both callers are live in standard YR play (part of the super-weapon fire chain).

---

## Behavioral Analysis

**What this function does (observable behavior):**
1. Stamps the current frame number as the IC start time onto the unit (`+0x18c`).
2. Records how many frames the effect lasts (`+0x194`).
3. Clears `+0x1a4` (exact role requires struct context — out-of-scope for this decode).
4. Sets `+0x1c4` to `1` for Force Shield, `0` for Iron Curtain.

**Force Shield vs Iron Curtain:**
The same function body handles both. The `is_force_shield` flag at `+0x1c4` distinguishes
them. The observable difference (gold vs silver shimmer, damage immunity source) is
driven by downstream consumers of `+0x1c4` — not in this function.

**`source_house` handling:**
The `source_house` parameter is passed through but not stored by `TechnoClass::IronCurtain`.
The misnamed dispatcher (`TechnoClass__StartFidget`) writes its own copy to `+0x6A4`
(= `param_1[0x1a9]` where `param_1` is `int *`, so `0x1a9*4 = 0x6A4`) before calling here.

---

## TS-vs-YR Filter

**Active in YR: Yes.** Both callers are reached from the standard Iron Curtain
super-weapon firing path in a normal YR skirmish. Force Shield (Yuri's secondary)
also routes here. No TS-only gate detected.

---

## Out-of-Scope Refs

These symbols are referenced but not in current scope — passed to scope-explorer:

| Symbol | Address | Reason seen |
|--------|---------|-------------|
| `g_CurrentFrameCounter` | `0x00a8ed84` | Read by this function; global decode task #14 covers it |
| `TechnoClass::+0x1a4` field | — | Cleared here; semantic unknown without struct context; task #10 covers TechnoClass IC fields |
| `TechnoClass::+0x190` field | — | Dead write; struct task #10 should confirm purpose |
| `WarpAttachClass__Detach` | (seen in `0x004deae4`) | Called before IC application in StartFidget; scope-explorer should evaluate |
| `TechnoClass__StartFidget` `+0x6A4` write | — | `source_house` stored there by caller before calling here |

---

## Unverified Claims

> **YELLOW — not mixed into verified body above**

- The preflight note stated `+0x190 = source_house`. **Assembly analysis refutes this.**
  The write at `0x0070e2cd` uses `[ESP+0x8]` which, given the frame analysis, is below
  the entry ESP and is NOT the `source_house` argument (`[X+8]`). The field receives
  local stack garbage. Confidence: HIGH (assembly frame analysis is definitive). The
  preflight note is incorrect for this offset.
