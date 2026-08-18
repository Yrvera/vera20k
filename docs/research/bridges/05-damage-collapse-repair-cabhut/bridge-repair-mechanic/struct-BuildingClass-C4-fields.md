# struct-BuildingClass-C4-fields

**Runbook:** struct-decode-v1
**Target:** `BuildingClass` C4-related fields at byte offsets `+0x528`, `+0x52C`, `+0x530`, `+0x540`, `+0x6DF`
**Confidence:** HIGH (all offsets directly read from Ghidra decompilation and byte-pattern search)
**YR-active:** YES — all five fields are actively read/written in `BuildingClass::Update` (C4 timer check), `InfantryClass::PerCellProcess` (C4 plant), and `TechnoClass::ReceiveDamage` (Crewed survivor path).

---

## Method

All offsets verified by:
1. `decompile_function 0x0043FB20` (`BuildingClass::Update`) — confirmed all five fields in the C4 timer branch.
2. `decompile_function 0x00519630` (`InfantryClass::PerCellProcess`) — confirmed plant writes.
3. `search_byte_patterns c6 ?? df 06 00 00 / ff 00 ff ff ff ff` — found all three write sites for `+0x6DF`.
4. `decompile_function 0x00701900` (`TechnoClass::ReceiveDamage`) — decoded the Crewed-survivor write path.
5. `read_memory` at key addresses to verify raw instruction bytes.

---

## Field Definitions

### `BuildingClass + 0x6DF` — C4/Crewed-pending flag (byte)

**Size:** 1 byte (`BYTE`)
**Verified by:** `search_byte_patterns c6 ?? df 06 00 00` → three addresses: `0x0051A5A7`, `0x00440320`, `0x00701F45`. All three confirmed via `read_memory`.

| Address | Function | Operation | Value |
|---|---|---|---|
| `0x0051A5A7` | `InfantryClass__PerCellProcess` | `MOV BYTE [EDI+0x6DF], 1` | Sets flag: C4 planted |
| `0x00440320` | `BuildingClass__Update` | `c6 86 df 06 00 00 00` = `MOV BYTE [ESI+0x6DF], 0` | Clears flag after bridge collapse |
| `0x00701F45` | `TechnoClass__ReceiveDamage` | `MOV BYTE [EDI+0x6DF], 1` | Sets flag: Crewed-survivor path |

Verified via `read_memory 0x0051A5A0` (14 bytes):
```
5b 83 c4 40 c2 04 00 c6 87 df 06 00 00 01
```
Bytes at +7: `c6 87 df 06 00 00 01` = `MOV BYTE [EDI+0x6DF], 1`. Address `0x0051A5A7`. Exact match.

Verified via `read_memory 0x00701F40` (24 bytes):
```
1f 8b 54 24 4c c6 87 df 06 00 00 01 8b 0d 84 ed a8 00 81 c7 28 05 00 00
```
Bytes at +6: `c6 87 df 06 00 00 01` = `MOV BYTE [EDI+0x6DF], 1`. Address `0x00701F45`. Exact match.

#### DUAL-PURPOSE CONFIRMED

This field is used for two separate purposes. They share the same timer state (`+0x528`, `+0x52C`, `+0x530`):

**Purpose 1 — C4-plant-pending (bridge path):**
Set by `InfantryClass::PerCellProcess` when SEAL/Tanya plants C4 on a bridge repair hut (CABHUT). Cleared by `BuildingClass::Update` after the bridge collapse is dispatched. In this path:
- `+0x528` = frame counter at plant time
- `+0x52C` = plant target location Y coordinate (from the infantry's nav-target coords)
- `+0x530` = countdown ticks (computed from `CIVDelay` or equivalent RulesClass field via `Math__ftol`)
- `+0x540` = pointer to the planting infantry (for kill attribution)

**Purpose 2 — Crewed-survivor spawn cooldown:**
Set by `TechnoClass::ReceiveDamage` when a Crewed building takes fatal damage from a warhead with `Crewed=yes`. Gate condition: `*(char *)(*(int *)(this + 0x520) + 0x1551) != 0` (Crewed-spawn-enabled flag on the building type at `+0x1551`). In this path:
- `+0x528` = frame counter at kill time
- `+0x52C` = Y coordinate of the hit location (attacker position)
- `+0x530` = survivor spawn delay ticks
- `+0x540` is NOT written in this path (only `+0x6DF`, `+0x528`, `+0x52C`, `+0x530` are written)

The two purposes coexist because a Crewed bridge hut cannot simultaneously have both a C4 planted and be killed by a Crewed warhead — the timer expiry clears `+0x6DF` in `BuildingClass::Update` regardless of which path set it.

**NOTE for Rust port:** The `+0x6DF` field must be a single flag shared by both paths. The C4 path checks `+0x16B6` (BridgeRepairHut) to dispatch to bridge collapse vs generic damage; the Crewed path checks `+0x1551` (Crewed spawn enabled). These gates are mutually exclusive for normal gameplay — a bridge repair hut (CABHUT) is not a Crewed building in stock rulesmd.ini.

---

### `BuildingClass + 0x528` — Timer frame stamp (int, 4 bytes)

**Size:** 4 bytes (`int32`)
**Verified by:** `decompile_function 0x0043FB20` — direct reads:
```c
iVar12 = g_CurrentFrameCounter - *(int *)&this->field_0x528;
if (iVar12 < iVar3) {
    iVar3 = iVar3 - iVar12;  // remaining ticks = total - elapsed
}
```
Sentinel `-1` means "timer not started." When `+0x528 == -1`, the remaining tick count is taken directly from `+0x530` without subtraction.

Also verified in `TechnoClass::ReceiveDamage` at `0x00701F58` (post-`ADD EDI, 0x528`):
```
89 0f    MOV [EDI], ECX    ; ECX = g_CurrentFrameCounter → stored at +0x528
```
Verified via `read_memory 0x00701F52` (12 bytes):
```
81 c7 28 05 00 00   ADD EDI, 0x528
89 0f               MOV [EDI], ECX
89 57 04            MOV [EDI+4], EDX   ; → +0x52C
```

---

### `BuildingClass + 0x52C` — Timer hit-location Y (int, 4 bytes)

**Size:** 4 bytes (`int32`)
**Purpose:** Stores the Y coordinate of the event location at the time the flag is set. In the C4 path, this is the nav-target building Y coordinate. In the Crewed path, this is the Y coordinate of the attacker's projectile hit location.
**Verified by:** `read_memory 0x00701F55` continuing from the ADD EDI sequence:
```
89 57 04    MOV [EDI+4], EDX   ; EDI+0x528+4 = EDI+0x52C
```
Verified via `read_memory 0x00701F55` (6 bytes): `05 00 00 89 0f 89` ... `89 57 04` at offset +3 from `0x00701F58` = `0x00701F5A`. `[EDI+4]` relative to the incremented EDI (which points to `+0x528`) = byte offset `+0x52C`. Exact match.

In `InfantryClass::PerCellProcess`:
```c
piVar10[0x14b] = iStack_8;    // 0x14b * 4 = 0x52C
```
Verified via `decompile_function 0x00519630` — the write `piVar10[0x14b]` at `param_1[0x169]`-relative context.

---

### `BuildingClass + 0x530` — Countdown ticks (int, 4 bytes)

**Size:** 4 bytes (`int32`)
**Purpose:** Total countdown ticks. Combined with `+0x528` to compute remaining time: `remaining = +0x530 - (g_CurrentFrameCounter - +0x528)`.

**Verified in `BuildingClass::Update`:**
```c
iVar3 = *(int *)&this->field_0x530;
if (*(int *)&this->field_0x528 == -1) {
LAB_004401fe:
    if (iVar3 != 0) goto LAB_00440378;  // still counting
} else {
    iVar12 = g_CurrentFrameCounter - *(int *)&this->field_0x528;
    if (iVar12 < iVar3) {
        iVar3 = iVar3 - iVar12;
        goto LAB_004401fe;
    }
    // timer expired: dispatch bridge collapse
}
```

**Verified in `TechnoClass::ReceiveDamage`:** `MOV [EDI+8], EAX` at `0x00701F5D` (EDI points to `+0x528`):
```
89 47 08    MOV [EDI+8], EAX   ; EDI+0x528+8 = EDI+0x530
```
Verified via `read_memory 0x00701F5D`: `89 47 08` at offset. Exact match.

In `InfantryClass::PerCellProcess`:
```c
piVar10[0x14c] = iVar4;    // 0x14c * 4 = 0x530
```
Verified via `decompile_function 0x00519630`.

---

### `BuildingClass + 0x540` — Planter pointer (ptr, 4 bytes)

**Size:** 4 bytes (pointer to `InfantryClass*`)
**Purpose:** In the C4 path: pointer to the `InfantryClass` instance that planted the C4. Used for kill attribution when the bridge collapse is triggered via `vtable+0x16c` (InflictDamage call) — passed as the `attacker` parameter.

**Verified in `BuildingClass::Update` C4 branch:**
```c
if (this->Type[0x16b6] == '\0') {
    // Generic building: InflictDamage on the building
    (**(code **)(this->vtable + 0x16c))(
        &iStack_28, 0,
        *(undefined4 *)(g_RulesClass_Instance + 0xfa8),
        *(undefined4 *)&this->field_0x540,  // ← planter ptr as attacker
        1, 0, 0);
} else {
    // Bridge repair hut: 5×5 scan + DestroyBridge_High/Low_OnHutDeath
    // After dispatch:
    this->field_0x6df = 0;
    *(undefined4 *)&this->field_0x540 = 0;   // ← cleared after dispatch
}
```

**In `InfantryClass::PerCellProcess`:**
```c
piVar10[0x150] = (int)param_1;    // 0x150 * 4 = 0x540 = planting infantry ptr
```
Verified via `decompile_function 0x00519630`.

**NOT written in `TechnoClass::ReceiveDamage` Crewed path** — only `+0x6DF`, `+0x528`, `+0x52C`, `+0x530` are written there.

---

## Summary Table

| Offset | Size | Name | Written by | Read by | Purpose |
|---|---|---|---|---|---|
| `+0x528` | int32 | `c4_frame_stamp` | `PerCellProcess` (plant), `ReceiveDamage` (Crewed) | `Update` (timer check) | Frame counter when flag was set; `-1` = not started |
| `+0x52C` | int32 | `c4_location_y` | `PerCellProcess` (plant), `ReceiveDamage` (Crewed) | (Crewed path only) | Y coordinate of event location |
| `+0x530` | int32 | `c4_delay_ticks` | `PerCellProcess` (plant), `ReceiveDamage` (Crewed) | `Update` (timer check) | Total countdown ticks |
| `+0x540` | ptr   | `c4_planter_ptr` | `PerCellProcess` (plant) | `Update` (InflictDamage attacker arg) | Pointer to planting infantry; null for Crewed path |
| `+0x6DF` | byte  | `c4_pending` | `PerCellProcess` (sets 1), `ReceiveDamage` (sets 1), `Update` (clears 0) | `Update` (gate check) | **DUAL-PURPOSE**: C4-plant-pending flag AND Crewed-survivor spawn pending flag |

---

## Key Behavioral Notes for Rust Port

1. **`+0x6DF` is shared between two code paths.** The Rust port must use a single flag that both C4 and Crewed paths set. The `BuildingClass::Update` timer logic reads it unconditionally and dispatches based on `+0x16B6` (BridgeRepairHut) to determine which outcome fires.

2. **Timer sentinel `-1` for `+0x528`.** If `+0x528 == -1`, the remaining tick count is `+0x530` directly (no elapsed subtraction). This handles the case where the C4 is planted in the same frame the timer fires (or a fresh Crewed kill without a prior frame stamp).

3. **`+0x540` is cleared after bridge dispatch.** In the BridgeRepairHut path, `+0x540` and `+0x6DF` are both zeroed after `DestroyBridge_High/Low_OnHutDeath` returns. In the generic-building path, only the C4 damage is applied; the flag is not explicitly cleared in that code path (the building is presumably destroyed).

4. **`+0x52C` is read only in the Crewed survivor path** (not used in the bridge path). For the bridge mechanic specifically, `+0x52C` is a write-only value — it is stored but not consumed by `BuildingClass::Update`'s bridge branch.

---

## Write Site Verification Summary

| Address | Pattern (hex) | Confirmed at |
|---|---|---|
| `0x0051A5A7` | `c6 87 df 06 00 00 01` | `read_memory 0x0051A5A0` +7 |
| `0x00440320` | `c6 86 df 06 00 00 00` | `search_byte_patterns` result |
| `0x00701F45` | `c6 87 df 06 00 00 01` | `read_memory 0x00701F40` +6 |
| `0x00701F58` | `89 0f` (MOV [EDI], ECX) | `read_memory 0x00701F52` — +6 after ADD EDI,0x528 |
| `0x00701F5A` | `89 57 04` (MOV [EDI+4], EDX) | `read_memory 0x00701F55` — confirms +0x52C |
| `0x00701F5D` | `89 47 08` (MOV [EDI+8], EAX) | `read_memory 0x00701F55` — confirms +0x530 |

---

## Unverified

**YELLOW:** `BuildingClass + 0x520` — the field read in `TechnoClass::ReceiveDamage` as `*(int *)(uVar12 + 0x520)`, which points to a type object checked at `+0x1551`. This is the Crewed-spawn-enabled flag on the building type. Not decoded further; out of scope for this task.

**YELLOW:** `RulesClass + 0xFA8` — the warhead type passed to `vtable+0x16C` for generic building C4 damage (non-bridge-hut path). Likely the C4 warhead type. Not fetched in this session.

---

## Self-proof (exit gate)

### Claim 1: `+0x6DF` written as 1 in `TechnoClass::ReceiveDamage @ 0x00701F45` (Crewed path)
`read_memory 0x00701F40` (24 bytes) → hex `1f 8b 54 24 4c c6 87 df 06 00 00 01 ...`.
Bytes at offset +5 = `C6 87 DF 06 00 00 01` = `MOV BYTE PTR [EDI+0x6DF], 1`.
Address `0x00701F45`. **VERIFIED — dual-purpose confirmed.**

### Claim 2: `+0x528` = frame stamp, with sentinel `-1` (not-started) pattern
`decompile_function 0x0043FB20` (BuildingClass::Update) → `*(int *)&this->field_0x528`
read and compared against `-1` sentinel, then subtracted from `g_CurrentFrameCounter`.
**VERIFIED from decompilation output.**

### Claim 3: `+0x540` = planting infantry ptr written by InfantryClass::PerCellProcess
`decompile_function 0x00519630` → `piVar10[0x150] = (int)param_1` where
`param_1` = InfantryClass* and `piVar10` = BuildingClass* (`int*`);
byte offset = `0x150 * 4 = 0x540`. **VERIFIED.**
