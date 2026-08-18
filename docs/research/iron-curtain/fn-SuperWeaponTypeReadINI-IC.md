# SuperWeaponTypeClass__ReadINI — Iron Curtain type-name decode

**Address:** `0x006cea20` (function start); IronCurtain string refs at `0x006cec52`, `0x006cecbd`  
**Class:** SuperWeaponTypeClass  
**Runbook:** function-decode-v1  
**Decoded:** 2026-05-24

---

## Summary

`SuperWeaponTypeClass__ReadINI` reads all INI fields for a super weapon type.
Within it, the `Type=` INI key is read and matched against a 12-entry string
table to assign an enum index stored at `SuperWeaponTypeClass + 0xb4`. The
string "IronCurtain" is at index **1** in that table. This enum index is the
discriminant that wires an INI-declared super weapon to the IC apply dispatch
path (`TechnoClass__StartFidget` misnamed dispatch at `0x004deae4`).

**Active in YR: Yes.** IronCurtain is a standard Soviet super weapon in all
YR/RA2 configurations.

---

## Decompilation excerpt (IC-relevant section)

```c
// from decompile_function 0x006cea20
// ... (other fields) ...

// Type= INI key read — stores result in local_3a8
CCINIClass__ReadString(iVar6, &DAT_00824314, &DAT_00889f64, local_3a8, 0x28);

// strlen check: if string non-empty
iVar3 = -1;
pcVar9 = local_3a8;
do { ... } while (cVar1 != '\0');

if (iVar3 != -2) {  // string was non-empty
    local_3f0 = 0;
    ppuVar8 = &PTR_s_MultiMissile_008425c0;  // start of SW type name table
    do {
        iVar3 = FUN_007c8d20(*ppuVar8, ...);  // strcmp(table_entry, local_3a8)
        if (iVar3 == 0) {
            if (local_3f0 != -1) {
                *(int *)(param_1 + 0xb4) = local_3f0;  // store enum index
            }
            break;
        }
        ppuVar8 = ppuVar8 + 1;
        local_3f0 = local_3f0 + 1;
    } while ((int)ppuVar8 < 0x8425f0);  // loop bound
}
```

`DAT_00824314` = "Type" (verified via `read_memory 0x00824314`).  
`FUN_007c8d20` = strcmp wrapper (unlabeled; verified via `get_function_by_address 0x007c8d20`).  
`param_1` is `int` (direct byte offsets).

The same table is also iterated for the `PreDependent=` INI key, storing the
result at `SuperWeaponTypeClass + 0xf0` (from decompile of the same function).

---

## SW type name table

Table starts at `0x008425c0`, iterates while `ppuVar2 < 0x8425f0` (12 entries,
indices 0–11). Verified via `read_memory 0x008425c0` (52 bytes).

| Index | String pointer | String value | Verified |
|-------|---------------|--------------|---------|
| 0 | `0x008425f0` | "MultiMissile" | `read_memory 0x008425c0` → ptr `0x008425f0`; Ghidra label `PTR_s_MultiMissile_008425c0` |
| **1** | `0x0081be54` | **"IronCurtain"** | `read_memory 0x008425c4` → ptr `0x0081be54`; `read_memory 0x0081be54` → "IronCurtain\0" |
| 2 | `0x0081be44` | "LightningStorm" | `read_memory 0x008425c8`; `read_memory 0x0081be44` |
| 3 | `0x0081be34` | "ChronoSphere" | `read_memory 0x008425cc`; `read_memory 0x0081be34` |
| 4 | `0x0081be28` | "ChronoWarp" | `read_memory 0x008425d0`; `read_memory 0x0081be28` |
| 5 | `0x0081be1c` | "ParaDrop" | `read_memory 0x008425d4`; `read_memory 0x0081be1c` |
| 6 | `0x0081bcbc` | "...ticConverter" (partial read) | `read_memory 0x008425d8` |
| 7 | `0x0081bca8` | "SpyPlane" | `read_memory 0x008425dc`; `read_memory 0x0081bc8c+28` |
| 8 | `0x0081bc9c` | "PsychicDominator" | `read_memory 0x008425e0`; `read_memory 0x0081bc8c+16` |
| 9 | `0x0081bc88` | "AmerParaDrop" | `read_memory 0x008425e4`; `read_memory 0x0081bc8c` |
| 10 | `0x0081bc7c` | (unread) | `read_memory 0x008425e8` — pointer only |
| 11 | `0x0081bc5c` | (unread) | `read_memory 0x008425ec` — pointer only |

**IronCurtain enum index = 1** (confirmed). This value is stored at
`SuperWeaponTypeClass + 0xb4` when `Type=IronCurtain` appears in the INI section.

---

## Separate helper function: FUN_006ce570

A standalone lookup function at `0x006ce570` uses the same table and returns
the index for a given type name string. It is the name→index resolver called
externally. Verified via `decompile_function 0x006ce570` (same loop, same
table bounds, returns index or -1 on not-found).

Xref: `0x006ce57f` in `FUN_006ce570` references "IronCurtain" at `0x0081be54`
(from `get_xrefs_to 0x0081be54`).

---

## Struct field accesses (SuperWeaponTypeClass)

`param_1` is `int` — all offsets are direct byte offsets.

| Offset | Size | INI key | Default | Semantic |
|--------|------|---------|---------|----------|
| `+0xb4` | 4 bytes (i32) | `Type=` | -1 (not found) | SW type enum index. `1` = IronCurtain. |
| `+0xf0` | 4 bytes (i32) | `PreDependent=` | -1 | Prerequisite SW type index. |
| `+0x9c` | 4 bytes (ptr) | `WeaponType=` | prior value | Weapon type ptr. |
| `+0xbc` | 4 bytes (enum) | `Action=` | prior value | Cursor action type. |
| `+0xe6` | 1 byte (bool) | `IsPowered=` | prior | Requires power. |
| `+0xe7` | 1 byte (bool) | `DisableableFromShell=` | prior | Can disable from options. |
| `+0xe8` | 4 bytes (i32) | `FlashSidebarTabFrames=` | prior | Sidebar flash on ready. |
| `+0xec` | 1 byte (bool) | `AIDefendAgainst=` | prior | AI defense flag. |
| `+0xed` | 1 byte (bool) | `PreClick=` | prior | Requires first click. |
| `+0xee` | 1 byte (bool) | `PostClick=` | prior | Requires second click. |
| `+0xf4` | 1 byte (bool) | `ShowTimer=` | prior | Show countdown timer. |
| `+0xc0` | 4 bytes (int) | `SpecialSound=` | prior | Sound cue index on use. |
| `+0xc4` | 4 bytes (int) | `StartSound=` | prior | Sound on charge start. |
| `+0xf8` | 4 bytes (float) | `Range=` | prior | Area-effect radius (cells). |
| `+0xfc` | 4 bytes (i32) | `LineMultiplier=` | prior | Multiplier for line SW. |
| `+0xb0` | 4 bytes (i32) | `RechargeTime=` | prior | Recharge duration (frames). |
| `+0xcc` | 25 bytes (char[]) | `SidebarImage=` | prior | Cameo image name. |
| `+0xb8` | 4 bytes (ptr) | (SHPFile handle) | — | Loaded sidebar sprite. |
| `+0xe5` | 1 byte (bool) | `UseChargeDrain=` | prior | Drain charge on fire. |
| `+0xf5` | 1 byte (bool) | `ManualControl=` | prior | Manual target selection. |
| `+0xc8` | (ptr) | `AuxBuilding=` | prior | Required aux building type. |

All offsets verified via `decompile_function 0x006cea20` — `param_1` is `int`.

---

## Dispatch table at 0x007e4ce4

A separate 8-entry table at `0x007e4ce4` also references the "IronCurtain"
string pointer (via `get_xrefs_to 0x0081be54`). This table has string pointers
starting from index "IronCurtain". It is distinct from the type-name enum
table above and its exact role (handler function table, action table, or visual
config) is **unverified** in this session. Scope-explorer should evaluate.

Verified via `read_memory 0x007e4ce0`: adjacent entries are "LightningStorm",
"ChronoSphere", "ChronoWarp" strings — same order as the type enum table but
starting at IronCurtain (not MultiMissile). Likely a subset dispatch table for
super weapons that have area-effect behavior.

---

## Handler dispatch

When an IC super weapon fires, the dispatch path is:
1. Player confirms target → some SW manager reads `+0xb4` == 1 (IronCurtain)
2. For each unit in the target area → calls `TechnoClass__StartFidget` misnamed
   dispatch at `0x004deae4` per unit (the actual IC apply function, decoded
   separately in `fn-StartFidget-IronCurtain-Dispatch.md`)

The `FUN_006ce570` function (`0x006ce570`) provides an external name→index
resolver (used by unknown callers — `get_function_callers` returns null, likely
vtable-dispatched). Verified via `decompile_function 0x006ce570`.

---

## INI keys

| INI key | INI section | Role |
|---------|------------|------|
| `Type=` | `[<SW section>]` | Assigns the SW type enum index (`+0xb4`). `Type=IronCurtain` → index 1. |

---

## Globals referenced

| Symbol | Address | Role |
|--------|---------|------|
| `PTR_s_MultiMissile_008425c0` | `0x008425c0` | Base pointer for SW type name table. |

---

## Out-of-scope refs

| Symbol | Address/location | Reason |
|--------|-----------------|--------|
| `FUN_007c8d20` | `0x007c8d20` | Strcmp wrapper; unlabeled. Not IC-specific. |
| `FUN_006ce570` | `0x006ce570` | SW type name→index lookup; callers not resolved. Scope-explorer should evaluate dispatch chain. |
| table at `0x007e4ce4` | `0x007e4ce4` | 8-entry string pointer table starting at "IronCurtain"; purpose unresolved. Scope-explorer should evaluate. |
| Indices 6–11 in SW type table | `0x008425d8`–`0x008425ec` | String values at those indices not fully read. Out of IC scope. |

---

## Unverified (YELLOW)

- **SW enum indices 6–11:** string values not read in this session (only pointers verified via `read_memory 0x008425c0`). Index assignment for IronCurtain = 1 is fully verified.
- **Dispatch chain from `+0xb4` to handler:** the SW manager that reads `+0xb4` and routes to `StartFidget` dispatch was not traced in this session. The `fn-StartFidget-IronCurtain-Dispatch.md` doc covers the per-unit handler but the connecting caller is unverified here.
- **Table at `0x007e4ce4`:** 8 entries, contains "IronCurtain" at index 0. Role is not confirmed — could be area-effect dispatch table or visual config table.
