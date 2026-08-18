# BridgeExplosions / MetallicDebris Rules Offsets — Definitive Layout

**Status:** verified directly from `gamemd.exe` decompile + disassembly @ this
session. Resolves the layout discrepancy flagged in
`HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §11.4 vs §11.12 / §11.13 /
§12.11.

## 0. TL;DR

Both `MetallicDebris` and `BridgeExplosions` are stored in
`DynamicVectorClass<AnimTypeClass*>` instances embedded in `RulesClass`. The
DVC is the standard Westwood layout (`VectorClass` base + `DynamicVectorClass`
extension):

| Sub-field             | Offset from DVC base |
|-----------------------|----------------------|
| `vtable`              | `+0x00`              |
| `Vector` (data ptr)   | `+0x04`              |
| `VectorMax` (capacity)| `+0x08`              |
| `IsAllocated` (byte)  | `+0x0C` (sub-byte 0x0D inside the dword) |
| `ActiveCount` (count) | `+0x10`              |
| `GrowthStep`          | `+0x14`              |
| (trailing dword)      | `+0x18`              |

Total DVC instance size: 0x1C bytes.

Resolved table:

| INI key            | Rules DVC base | Data ptr | Count (ActiveCount) | GrowthStep | INI parser (ReadGeneral) | Live spawn reader (BlowUpBridge) |
|--------------------|----------------|----------|---------------------|------------|--------------------------|----------------------------------|
| `MetallicDebris`   | `Rules+0x13C`  | `+0x140` | `+0x14C`            | `+0x150`   | `0x0066DAA5` (string push) | `0x0047DD70` body (Anim #1 spawn) |
| `BridgeExplosions` | `Rules+0x158`  | `+0x15C` | `+0x168`            | `+0x16C`   | `0x0066DBA8` (string push) | `0x0047DD70` body (Anim #2 spawn) |

**Reconciliation:** the §11.4 / live-spawn-side offsets (`+0x140/+0x14C` for
MetallicDebris, `+0x15C/+0x168` for BridgeExplosions) are CORRECT for `data
ptr / count`. The §11.12-§11.13 / §12.11 doc claim that the trailing writes
`+0x14C/+0x150/+0x154` and `+0x168/+0x16C/+0x170` are `data/count/capacity` was
WRONG — those writes are the `ActiveCount / GrowthStep / trailing` fields of
the `DynamicVectorClass` extension, written AFTER `CopyFrom` has already filled
in `vtable + Vector + VectorMax`. The vtable bases are `+0x13C` (NOT `+0x148`)
and `+0x158` (NOT `+0x164`).

## 1. INI parser side — `RulesClass::ReadGeneral` @ `0x0066D530`

`[General]` section (verified via `PTR_s_General_007f0c9c` push at each
ReadString site). Both keys are parsed by the same boilerplate: ReadString →
tokenize on comma → `AnimTypeClass::FindOrAllocate` per token → push into a
local DVC → `DynamicVectorClass::CopyFrom(this=&Rules.field_DVC, &local_DVC)`
→ then 3 trailing `MOV [base+0x10/+0x14/+0x18], ...` writes.

### 1.1 `BridgeExplosions=` reader

String reference: `"BridgeExplosions"` @ `0x0083CEDC` (only xref:
`0x0066DBA8` in `RulesClass__ReadGeneral` [DATA]).

Key disassembly (from this session, ASM lines 468-528 of the function):

```
0066dba8: PUSH 0x83cedc          ; "BridgeExplosions"
0066dbb0: CALL 0x00528a10        ; CCINIClass::ReadString
...
0066dc4e: LEA  ECX,[ESP+0x34]    ; local source DVC
0066dc52: LEA  EBX,[ESI+0x158]   ; this  = &Rules.BridgeExplosions_DVC  → base = +0x158
0066dc59: MOV  ECX,EBX
0066dc5b: CALL 0x00525060        ; DynamicVectorClass::CopyFrom
0066dc60: MOV  EDX,[ESP+0x44]    ; local_94 (ActiveCount of src)
0066dc64: MOV  [EBX+0x10],EDX    ; Rules+0x168 = ActiveCount
0066dc67: MOV  EAX,[ESP+0x48]    ; local_90 (GrowthStep)
0066dc6b: MOV  [EBX+0x14],EAX    ; Rules+0x16C = GrowthStep
0066dc6e: MOV  ECX,[ESP+0x4c]    ; local_8c
0066dc??: MOV  [EBX+0x18],ECX    ; Rules+0x170 = trailing dword
```

(`ESI` holds `RulesClass*`; `EBX = ESI + 0x158`.)

### 1.2 `MetallicDebris=` reader

String reference: `"MetallicDebris"` @ `0x0083CEF0` (only xref: `0x0066DAA5`
in `RulesClass__ReadGeneral` [DATA]).

Key disassembly (ASM lines 399-461):

```
0066daa5: PUSH 0x83cef0          ; "MetallicDebris"
0066daad: CALL 0x00528a10        ; CCINIClass::ReadString
...
0066db4b: LEA  EBX,[ESI+0x13c]   ; this = &Rules.MetallicDebris_DVC  → base = +0x13C
0066db51: LEA  EAX,[ESP+0x34]
0066db56: MOV  ECX,EBX
0066db58: CALL 0x00525060        ; DynamicVectorClass::CopyFrom
0066db5d: MOV  ECX,[ESP+0x44]
0066db61: MOV  [EBX+0x10],ECX    ; Rules+0x14C = ActiveCount
0066db64: MOV  EDX,[ESP+0x48]
0066db68: MOV  [EBX+0x14],EDX    ; Rules+0x150 = GrowthStep
0066db6b: MOV  EAX,[ESP+0x4c]
0066db6f: MOV  [EBX+0x18],EAX    ; Rules+0x154 = trailing
```

### 1.3 DVC shape — confirmed via `DynamicVectorClass::CopyFrom` @ `0x00525060`

Pseudocode (this-pointer is `param_1`, source is `param_2`):

```
(**(*param_1 + 0xc))();        // virtual Clear() — touches vtable @ +0x00
param_1[2] = param_2[2];       // +0x08 ← +0x08    (VectorMax/capacity)
if (param_2[2] == 0) {
  param_1[1] = 0;              // +0x04 ← 0        (Vector/data*)
  *(byte*)(param_1 + 0xD) = 0; // +0x0D ← 0        (IsAllocated)
} else {
  param_1[1] = new[capacity*4]; // +0x04 ← heap    (Vector)
  *(byte*)(param_1 + 0xD) = 1;  // +0x0D ← 1       (IsAllocated)
  copy_loop: param_1[1][i] = param_2[1][i] for i in [0..capacity);
}
```

This confirms the `VectorClass` base layout (`vtable / Vector / VectorMax /
IsAllocated`). The `DynamicVectorClass` extension adds `ActiveCount / GrowthStep
/ ...` at `+0x10 / +0x14 / +0x18` — confirmed by the three post-CopyFrom MOVs
in `ReadGeneral`. After CopyFrom, capacity (`VectorMax @ +0x08`) and count
(`ActiveCount @ +0x10`) end up numerically equal (the local DVC was just-built
by token-by-token Add calls).

## 2. Live spawn side — `CellClass::BlowUpBridge` @ `0x0047DD70`

Verified callers (all live in YR — bridge collapse paths, no TS-only gates):
`SetBridgeDirection_NESW/NWSE`, `UpdateRamp_{NS,EW}_Collapse{A,B}_{High,Low}`,
`ProcessBridgeDamageStateMachine_{High,Low}`. None are gated behind
`SpecialFlags`, `FogOfWar`, or TS-era callers.

Decompiled spawn block (this session):

```c
if ((0 < *(int *)(g_RulesClass_Instance + 0x168)) &&            // BridgeExplosions count gate
    (Random__RandomRanged(0, 0x7ffffffe) * scale < 0.95)) {     // outer ~95% gate
  // ... position setup ...
  if ((Random__RandomRanged(0, 0x7ffffffe) * scale < 0.50) &&   // inner ~50% gate
      (pvVar5 = operator_new(0x1c8), pvVar5 != 0)) {
    iVar4 = Random__RandomRanged(0, *(int *)(g_RulesClass_Instance + 0x14c) + -1);
    AnimClass__Constructor(
        *(undefined4 *)(*(int *)(g_RulesClass_Instance + 0x140) + iVar4 * 4),   // MetallicDebris
        &iStack_c, 0, 1, 0x600, 0, 0);
  }
  pvVar5 = operator_new(0x1c8);
  if (pvVar5 != 0) {
    uVar6 = Random__RandomRanged(1, 5);                                          // anim frame variant
    iVar4 = Random__RandomRanged(0, *(int *)(g_RulesClass_Instance + 0x168) + -1);
    AnimClass__Constructor(
        *(undefined4 *)(*(int *)(g_RulesClass_Instance + 0x15c) + iVar4 * 4),   // BridgeExplosions
        &iStack_c, uVar6, 1, 0x600, 0, 0);
  }
}
```

### 2.1 MetallicDebris read sites + indexed offsets

- `*(g_RulesClass_Instance + 0x14C)` → MetallicDebris.ActiveCount (count gate)
- `*(g_RulesClass_Instance + 0x140)` → MetallicDebris.Vector (data ptr, indexed `[idx]*4` for `AnimTypeClass*` element)

Matches the parser side (base `+0x13C`, data at `+0x140 = base+4`, count at
`+0x14C = base+0x10`). VERIFIED.

### 2.2 BridgeExplosions read sites + indexed offsets

- `*(g_RulesClass_Instance + 0x168)` → BridgeExplosions.ActiveCount (outer gate + RNG bound)
- `*(g_RulesClass_Instance + 0x15C)` → BridgeExplosions.Vector (data ptr)

Matches the parser side (base `+0x158`, data at `+0x15C = base+4`, count at
`+0x168 = base+0x10`). VERIFIED.

## 3. Reconciliation table (final verdict)

| Claim | Source | Verdict |
|-------|--------|---------|
| MetallicDebris data ptr at `Rules+0x140` | §11.4 live reads, BlowUpBridge | ✓ CORRECT |
| MetallicDebris count at `Rules+0x14C`    | §11.4 live reads, BlowUpBridge | ✓ CORRECT (it is `ActiveCount` at DVC `+0x10`) |
| BridgeExplosions data ptr at `Rules+0x15C` | §11.4 live reads, BlowUpBridge | ✓ CORRECT |
| BridgeExplosions count at `Rules+0x168`    | §11.4 live reads, BlowUpBridge | ✓ CORRECT (`ActiveCount` at DVC `+0x10`) |
| MetallicDebris layout `+0x148/+0x14C/+0x150/+0x154` (vtable/data*/cap/count) | §11.13 + §12.11 | ✗ WRONG — vtable is at `+0x13C`, not `+0x148`; the 3 trailing fields are `ActiveCount/GrowthStep/trailing`, not `data*/cap/count` |
| BridgeExplosions layout `+0x164/+0x168/+0x16C/+0x170` (vtable/data*/cap/count) | §11.13 + §12.11 | ✗ WRONG — vtable is at `+0x158`, not `+0x164` |
| Both reads (§11.4) are "8 bytes lower than documented layout" | §12.11 puzzle | ✗ FALSE PUZZLE — the §12.11 layout was off by 0xC; §11.4 reads are correct |

## 4. RNG call sites

`Random__RandomRanged @ 0x0065C7E0` — verified by direct function lookup.
Called four times per `BlowUpBridge` invocation that passes the count gate:

1. Outer 95% gate (`R(0, 0x7FFFFFFE)` vs `_DAT_007e4f58 ≈ 0.95`)
2. X-offset jitter (`R(0, 0x7FFFFFFE)`)
3. Y-offset jitter (`R(0, 0x7FFFFFFE)`)
4. Inner 50% gate (`R(0, 0x7FFFFFFE)` vs `_DAT_007e1738 ≈ 0.50`) — guards MetallicDebris spawn
5. (conditional) `R(0, MetallicDebris.ActiveCount - 1)` for picking debris anim type
6. `R(1, 5)` — animation start-frame variant for BridgeExplosions
7. `R(0, BridgeExplosions.ActiveCount - 1)` for picking explosion anim type

All identical-RNG, lockstep-critical. Rust must mirror call order and ranges
exactly.

## 5. Active in YR — verdict

**ACTIVE.** All `BlowUpBridge` callers are bridge-collapse paths used during
regular YR play (ramp collapse, NESW/NWSE bridge stamping, both-tier damage
state machines). No `SpecialFlags`, no TS-only gates. Both `MetallicDebris=`
and `BridgeExplosions=` keys are parsed unconditionally from `[General]`.

No callers limited to TS-era code paths.

## 6. Open Questions

- **`+0x18` trailing field semantics.** The third post-CopyFrom write
  (`+0x154` for MetallicDebris, `+0x170` for BridgeExplosions) lands at DVC
  base `+0x18`. Likely a second growth-related field (Westwood DVC has
  `GrowthStep` and a "warn on growth" or "max growth" sibling). Not load-bearing
  for the spawn logic — both BlowUpBridge reads only touch `+0x04` (data) and
  `+0x10` (ActiveCount). Verifying the exact name would require studying
  `DynamicVectorClass::Add` / `Resize`. Out of scope for this report.

- **Live runtime read.** The static value of `g_RulesClass_Instance @
  0x008871E0` is `0x00000000` in the binary (heap-allocated at startup). A
  live debugger snapshot of the populated `Rules+0x13C..+0x158` and
  `+0x158..+0x174` bytes would be a third confirmation channel but is not
  required given the assembly is unambiguous.

## 7. Sources

- `RulesClass::ReadGeneral` @ `0x0066D530` — decompile + disassembly (this session).
  Body: `0x0066D530 - 0x00671E98`.
- String `"BridgeExplosions"` @ `0x0083CEDC` (single DATA xref from
  `0x0066DBA8`).
- String `"MetallicDebris"` @ `0x0083CEF0` (single DATA xref from `0x0066DAA5`).
- `DynamicVectorClass::CopyFrom` @ `0x00525060` — decompile (this session).
- `CellClass::BlowUpBridge` @ `0x0047DD70` — decompile (this session).
- `Random__RandomRanged` @ `0x0065C7E0` — function lookup (this session).
- `g_RulesClass_Instance` pointer @ `0x008871E0` (per `RULESCLASS_GHIDRA_REPORT.md`
  line 4) — static value null, runtime-populated.
- Parent docs reconciled: `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
  §11.4, §11.12, §11.13, §12.11; `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`
  Sources @ `0x66F1D9` (note: the user-supplied entry-point hint was off — the
  actual `ReadGeneral` entry is `0x0066D530`, not `0x0066F1D9`).
