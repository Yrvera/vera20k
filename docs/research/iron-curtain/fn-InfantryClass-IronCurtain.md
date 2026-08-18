# fn-InfantryClass-IronCurtain

## Identity

| Field | Value |
|---|---|
| Address | `0x00522600` |
| Name | `InfantryClass__IronCurtain` |
| Signature | `void __thiscall InfantryClass__IronCurtain(int* param_1, undefined4 param_2, undefined4 param_3)` |
| Vtable slot | Byte offset `0x154` from vtable__InfantryClass base `0x007eb058` (slot 85). Verified: DATA xref from `0x007eb1ac` (bytes `00 26 52 00` = `0x00522600`); read_memory 0x007eb1ac. |
| Active in YR | Yes — InfantryClass vtable slot dispatched from IronCurtain super-weapon apply path. |
| Body range | `0x00522600` – `0x0052263b` |

Vtable base confirmed: `get_assembly_context 0x00517a50` shows instruction at `0x00517acc`: `MOV dword ptr [ESI], 0x7eb058` — the constructor stores `0x007eb058` as `*this` (the vtable pointer).

Verified via `decompile_function 0x00522600` and `get_function_by_address 0x00522600`.

## CRITICAL FINDING: Infantry are instakilled by Iron Curtain

This function does NOT make infantry invulnerable. It kills them outright by calling a damage dispatch function with the unit's full strength as damage, using the `C4Warhead` warhead. This matches observable gameplay: infantry die instantly when targeted by Iron Curtain.

## Decompiled body (verbatim)

```c
void __thiscall InfantryClass__IronCurtain(int *param_1, undefined4 param_2, undefined4 param_3)
{
  undefined4 local_4;

  local_4 = *(undefined4 *)(param_1[0x1b0] + 0xa0);
  (**(code **)(*param_1 + 0x16c))(
      &local_4,
      0,
      *(undefined4 *)(g_RulesClass_Instance + 0xfa8),
      0,
      1,
      0,
      param_3);
  return;
}
```

Verified via `decompile_function 0x00522600`.

**param_1 pointer-arithmetic note**: param_1 is `int*`. `param_1[0x1b0]` = byte offset `0x1b0 × 4 = 0x6c0`. This is the InfantryTypeClass pointer slot (confirmed from constructor: `InfantryClass__Constructor` at `0x00517a50` stores `param_2` at `param_1[0x1b0]` = `[ESI + 0x6c0]`).

## Step-by-step decode

### Step 1: Load damage value from TypeClass

```c
local_4 = *(undefined4 *)(param_1[0x1b0] + 0xa0);
```

- `param_1[0x1b0]` = `*(param_1 + 0x6c0)` = pointer to `InfantryTypeClass` instance for this unit.
- `+ 0xa0` = field at byte offset `0xa0` within `InfantryTypeClass` = **`Strength`** (the unit's max hit points / base HP). This is the standard TechnoTypeClass field for unit strength, well-established across RA2 class hierarchy.
- `local_4` therefore holds the unit's own `Strength` value — used as the damage amount.

**YELLOW — Unverified**: That `InfantryTypeClass + 0xa0` = Strength. This is inferred from the RA2 TechnoTypeClass layout convention. The struct-decode task for TechnoTypeClass IC fields (task #13) should confirm the exact offset.

### Step 2: Identify the warhead

```c
*(undefined4 *)(g_RulesClass_Instance + 0xfa8)
```

Verified via `decompile_function 0x0066bbb0` (`RulesClass__ReadCombatDamage`): the field at `RulesClass + 0xfa8` is written by:
```c
pcStack_e0 = s_C4Warhead_0083b1d4;   // INI key "C4Warhead"
uVar3 = WarheadTypeClass__FindOrAllocate();
*(undefined4 *)(param_1 + 0xfa8) = uVar3;
```

**`RulesClass + 0xfa8` = `C4Warhead` warhead pointer** (a `WarheadTypeClass*`). INI key `C4Warhead=` under `[CombatDamage]`. In stock YR `rulesmd.ini`: `C4Warhead=C4`.

This is NOT a large damage integer constant — it is a warhead pointer. The task preflight note "likely a huge damage constant or IronCurtainDamage INI key" was incorrect; verified from `RulesClass__ReadCombatDamage` decompile.

### Step 3: Identify the called function

```c
(**(code **)(*param_1 + 0x16c))(...)
```

- `*param_1` = vtable pointer = `0x007eb058` (vtable__InfantryClass base).
- `0x007eb058 + 0x16c` = `0x007eb1c4` — vtable slot at byte offset `0x16c`.
- `read_memory 0x007eb1c4` → bytes `a0 7f 51 00` = function pointer `0x00517fa0`.
- `get_xrefs_to 0x00517fa0` returns one DATA xref: `0x007eb1c4` — confirming this is the only reference.

The function at `0x00517fa0` is unnamed in Ghidra. It is NOT `InfantryClass__ReceiveDamage` (which is at `0x005227f0`). Its first bytes are `81 EC C0 00 00 00` = `SUB ESP, 0xC0` (large stack frame = complex function). Based on call argument layout and position in the vtable inheritance chain, this is a parent-class ReceiveDamage dispatch (FootClass or TechnoClass level). Exact identity is unverified without decompiling `0x00517fa0` fully.

**YELLOW — Unverified**: The exact name/identity of `0x00517fa0`. It is the function at InfantryClass vtable byte offset `0x16c`, accepting `(int* damage_ref, 0, WarheadTypeClass*, 0, 1, 0, source_house)`. Recommend scope-explorer add this to manifest as `FUN_00517fa0_ReceiveDamage_ancestor`.

### Step 4: Full call reconstruction

```
FUN_00517fa0(
    &local_4,          // pointer to damage value = unit's Strength (instakill)
    0,                 // unknown (likely: no forcing)
    C4Warhead_ptr,     // warhead = RulesClass.C4Warhead
    0,                 // unknown
    1,                 // unknown flag (possibly "instant kill override")
    0,                 // unknown
    param_3            // source_house (forwarded from IronCurtain dispatch)
)
```

This matches the YR observable behavior: infantry die the frame Iron Curtain is applied.

### Step 5: Parameters not forwarded

`param_2` (duration) and `param_3` (is_force_shield / source_house) from the vtable call:
- `param_2` (duration) is **ignored entirely**. Infantry don't receive an IC duration — they are killed.
- `param_3` is forwarded to the damage call as source_house (last argument).

## Callers

`get_function_callers 0x00522600` returned empty (vtable-dispatched; known MCP flakiness on vtable-dispatched functions per team-lead manifest note).

`get_xrefs_to 0x00522600` returns: DATA xref from `0x007eb1ac` — the InfantryClass primary vtable slot at offset `0x154` from vtable base `0x007eb058` = slot 85.

This function is called exclusively through the vtable dispatch from the IronCurtain super-weapon apply path.

## Out-of-scope references

- `InfantryTypeClass + 0xa0` (Strength field) — covered by task #13 (decode-struct-TechnoTypeClass_IC_immune)
- `g_RulesClass_Instance` — covered by task #15
- `RulesClass + 0xfa8` (C4Warhead) — covered by task #12 (decode-struct-RulesClass_IC_config)
- `FUN_00517fa0` at `0x00517fa0` — unnamed parent-class ReceiveDamage. Should be added to manifest via scope-explorer for full tracing.

## Observable behavior summary

Iron Curtain applied to an infantry unit → unit receives damage equal to its own `Strength` using the `C4Warhead` warhead → unit dies instantly. No invulnerability state is set. No duration stored. The IC effect on infantry is purely destructive, not protective.

## Active in YR: Yes

Vtable slot populated, directly reachable from the IC super-weapon apply path. No TS-legacy gate.

## TS-legacy assessment: Not TS-legacy

The function is compact, in an active vtable slot, and exhibits observable behavior (infantry instakill) that occurs in standard YR gameplay. No TS-only flags.
