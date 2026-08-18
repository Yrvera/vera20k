# SessionClass__ReadSkirmishSettings — Skirmish Settings INI Reader

## Summary

Reads saved skirmish settings from RA2MD.INI `[Skirmish]` section and populates the
`SessionClass` struct pointed to by `param_1`. Reads global options (game mode, scenario
index, game speed, credits, unit count, booleans) then iterates slots 1–7 reading per-slot
triples `(type, country, color)` via `FUN_00477440` (task #53, INI 3-int tokenizer).

Slot 0 is the human player row and is treated specially: its defaults come from `param_5`
while slots 2–7 use `param_4` as the default type. All slot triples are stored in
`param_1` as a flat array with stride 3 starting at dword-index 10 for slot 1.

## Address

`0x00697F10` (verified via `decompile_function 0x00697F10`)

## Active in YR

Yes. Called from `CDFileClass__Constructor @ 0x006980C0` (mislabeled; actually the
YR skirmish session loader). That function is called from `Main_Game @ 0x0052D9A0`,
`OptionsClass__ShowLauncherDialog @ 0x0055FC80`, and
`SimpleWonlineDialogControl__Constructor @ 0x007864C0`.
(Confirmed via `get_function_callers 0x00697F10` and `get_function_callers 0x006980C0`)

## Signature / Parameters

```c
void __thiscall SessionClass__ReadSkirmishSettings(
    undefined4 *param_1,   // this: SessionClass* (output struct)
    undefined4  param_2,   // unused at call-site; overwritten in loop
    undefined4  param_3,   // INI section handle (CCINIClass* or section ptr)
    undefined4  param_4,   // default slot-type for slots 2–7
    undefined4  param_5    // default slot-type for slot 1 (human row)
)
```

(verified via `decompile_function 0x00697F10`)

## Behavioral Analysis

### Step 1 — Clear INI cache and get INI instance

```c
INIClass__ClearSectionCache();
iVar4 = FUN_005d5e10();   // returns CCINIClass* for RA2MD.INI / skirmish INI
```

`FUN_005d5e10` is a lazy-init singleton accessor; returns 0 if the INI object is not
yet loaded. The result is not null-checked before ReadInt/ReadBool calls — callers
must ensure the INI is loaded.
(verified via `decompile_function 0x00697F10`, `decompile_function 0x005D5E10`)

### Step 2 — Global option fields

All reads use `CCINIClass__ReadInt` / `CCINIClass__ReadBool` with defaults drawn
from `g_RulesClass_Instance` where present:

| Field offset (dwords) | byte offset | INI key | Default source |
|---|---|---|---|
| `param_1[0]` | +0x00 | `GameMode` | `iVar4 + 0x28` (INI instance field) |
| `param_1[1]` | +0x04 | `ScenIndex` | `0` |
| `param_1[2]` | +0x08 | `GameSpeed` | `g_RulesClass_Instance + 0x1A0` × 4 = `+0x14A0` |
| `param_1[3]` | +0x0C | `Credits` | `g_RulesClass_Instance + 0x1484` |
| `param_1[4]` | +0x10 | `UnitCount` | `g_RulesClass_Instance + 0x1494` |
| `*(param_1+5)` (bool) | +0x14 | `ShortGame` | `g_RulesClass_Instance + 0x14B6` |
| byte at +0x15 | +0x15 | `SuperWeaponsAllowed` | `g_RulesClass_Instance + 0x14B9` |
| byte at +0x16 | +0x16 | `BuildOffAlly` | `g_RulesClass_Instance + 0x14BA` |
| byte at +0x17 | +0x17 | `MCVRepacks` | `g_RulesClass_Instance + 0x14B8` |
| `*(param_1+6)` (bool) | +0x18 | `CratesAppear` | `g_RulesClass_Instance + 0x14B1` |

(verified via `decompile_function 0x00697F10`)

### Step 3 — Per-slot triple loop (slots 1–7)

```c
iVar4 = 1;
do {
    FUN_007c8ef4(local_10, s_Slot_02d, iVar4);   // sprintf(local_10, "Slot%02d", iVar4)
    param_2  = (iVar4 == 1) ? param_5 : param_4; // default slot-type
    param_3  = 0xFFFFFFFE;                        // default country (0xFFFFFFFE = random)
    local_14 = 0xFFFFFFFE;                        // default color   (0xFFFFFFFE = random)
    FUN_00477440(uVar2, local_10, &param_2, &param_3, &local_14);
    param_1[iVar4 * 3 + 7] = param_2;            // slot type
    param_1[iVar4 * 3 + 8] = param_3;            // country
    param_1[iVar4 * 3 + 9] = local_14;           // color
    iVar4++;
} while (iVar4 < 8);
```

Loop runs for `iVar4 = 1..7` (7 iterations). Slot 0 (the human player) is not read here —
it is handled separately by the caller.
(verified via `decompile_function 0x00697F10`)

### Slot triple layout in `param_1`

For slot `n` (1 ≤ n ≤ 7):

| dword index | byte offset | Content |
|---|---|---|
| `n*3 + 7` | `(n*3+7)*4` | Slot type (AI type index, or sentinel) |
| `n*3 + 8` | `(n*3+8)*4` | Country index (0xFFFFFFFE = random) |
| `n*3 + 9` | `(n*3+9)*4` | Color index  (0xFFFFFFFE = random) |

Slot 1 lands at dword indices 10, 11, 12 (byte offsets 0x28, 0x2C, 0x30).
Slot 7 lands at dword indices 28, 29, 30 (byte offsets 0x70, 0x74, 0x78).

### INI key format

`FUN_007c8ef4` formats slot section keys as `"Slot%02d"` — producing `Slot01`, `Slot02`,
… `Slot07`. These are section names (or sub-keys) under the `[Skirmish]` INI section.
`FUN_00477440` reads each as a single comma-separated value.
(verified via `decompile_function 0x007C8EF4`)

## Globals Referenced

| Global | Role | Access |
|---|---|---|
| `g_RulesClass_Instance` | RulesClass singleton; provides INI defaults | READ |
| (INI singleton via `FUN_005D5E10`) | RA2MD.INI / skirmish session INI object | READ |

## Callees

Confirmed via `get_function_callees 0x00697F10`:
- `INIClass__ClearSectionCache @ 0x00526B00` — clears section lookup cache
- `FUN_005D5E10 @ 0x005D5E10` — returns CCINIClass* (RA2MD.INI singleton accessor)
- `CCINIClass__ReadInt @ 0x005276D0` — reads integer INI value with default
- `CCINIClass__ReadBool @ 0x005295F0` — reads boolean INI value with default
- `FUN_007C8EF4 @ 0x007C8EF4` — sprintf wrapper (formats "Slot%02d")
- `FUN_00477440 @ 0x00477440` — INI 3-int tokenizer (task #53)

## Callers

- `CDFileClass__Constructor @ 0x006980C0` (mislabeled; actually YR skirmish session loader)

(Confirmed via `get_function_callers 0x00697F10`)

## Out-of-scope refs

- `FUN_005D5E10` — CCINIClass singleton accessor; not in cell-UI scope
- `g_RulesClass_Instance` — RulesClass global; layout decoded in separate RE work
- `INI key defaults` from `g_RulesClass_Instance` — field offsets `0x14A0`, `0x1484`, etc.
  belong to RulesClass struct, out of cell-UI scope

## Unverified (YELLOW)

- `param_3` type: Ghidra shows `undefined4`; likely a `CCINIClass*` section handle or
  the INI object directly — not confirmed against `CCINIClass__ReadInt` signature.
- `param_4` / `param_5` exact semantics: inferred as "default AI slot type" from their
  use as the initial `param_2` value in the loop and the slot-1 special case; not
  confirmed by tracing the caller `0x006980C0`.
- `FUN_005D5E10` return: inferred as a CCINIClass singleton from the subsequent `ReadInt`
  call using its return value `iVar4` as the INI context at `iVar4 + 0x28`; the
  singleton identity (RA2MD.INI vs another INI) is not confirmed by decompile.
- Slot 0 handling: the loop starts at `iVar4 = 1`, so slot 0 (human player) is absent.
  How slot 0 is populated is handled by the caller — not traced in this session.
