# string-IronCurtain-typename

## Identity

| Field | Value |
|---|---|
| String | `"IronCurtain"` |
| Address | `0x0081be54` |
| Usage | SuperWeaponTypeClass type-name enum (index 1) |
| INI Key | `Type=IronCurtain` in `[SuperWeapons]` section entries |
| Type | const char* in a pointer table |

## Verification

String address confirmed via `get_xrefs_to 0x0081be54` — returns 5 xrefs:
- `0x006ce57f` in `FUN_006ce570` [DATA] — type-name lookup function
- `0x008425c4` [DATA] — pointer table entry
- `0x006cec52` in `SuperWeaponTypeClass__ReadINI` [DATA]
- `0x006cecbd` in `SuperWeaponTypeClass__ReadINI` [DATA]
- `0x007e4ce4` [DATA] — likely vtable or static data

String content confirmed via `inspect_memory_content 0x0081be54`: `"IronCurtain"` (12 bytes including null terminator).

**Type-name table** at `PTR_s_MultiMissile_008425c0` (`0x008425c0`), confirmed via `decompile_function 0x006ce570`:

| Index | Pointer | String |
|---|---|---|
| 0 | `0x008425c0` → `0x008425f0` | `"MultiMissile"` |
| 1 | `0x008425c4` → `0x0081be54` | `"IronCurtain"` ← this string |
| 2 | `0x008425c8` → `0x0081be44` | `"LightningStorm"` |
| 3 | `0x008425cc` → `0x0081be34` | `"NoEnterTunnel"` |
| 4 | `0x008425d0` → `0x0081be28` | `"EnterTunnel"` |
| 5 | `0x008425d4` → `0x0081be1c` | `"NoTogglePower"` |
| 6 | `0x008425d8` → `0x0081be10` | `"NoGRepair"` |
| 7 | `0x008425dc` → `0x0081be04` | `"NoEnter"` |
| ... | ... | ... (12 entries total; table ends at `0x8425f0`) |

Table entries from `0x0081bcbc`: `"AmerParaDrop"`, `"Demolish"`, `"AttackMoveTar"`, `"AttackMoveNav"`, `"SelectBeacon"` — confirmed via `inspect_memory_content 0x0081bcbc`.

## Type-name lookup function

`FUN_006ce570` at `0x006ce570` — confirmed via `decompile_function 0x006ce570`:

```c
int FUN_006ce570(const char* name) {
    int index = 0;
    const char** table = &PTR_s_MultiMissile_008425c0;
    do {
        if (strcmp(*table, name) == 0) return index;
        table++;
        index++;
    } while ((int)table < 0x8425f0);
    return -1;   // not found
}
```

Returns the integer enum index for a given type name string. Returns -1 if not found. Called by `SuperWeaponTypeClass__ReadINI` (0x006cec52, 0x006cecbd) to resolve the `Type=` INI value.

## Semantics

`"IronCurtain"` is the type-name identifier for the Iron Curtain superweapon. When `SuperWeaponTypeClass__ReadINI` reads a `[SuperWeaponType]` entry and encounters `Type=IronCurtain`, it calls `FUN_006ce570` which returns index **1**. This integer (the enum value) is stored in the `SuperWeaponTypeClass` struct to record which superweapon type this is.

The enum index 1 is the runtime identity of the Iron Curtain type throughout the game — vtable dispatch, behavior dispatch in the fire/tick logic, and the `IsSWTypeActive` / `CanFire` checks all operate on this integer.

## Xref count: 5

| Address | Context |
|---|---|
| `0x008425c4` | Pointer table entry — DATA |
| `0x006ce57f` | Type-name lookup loop (`FUN_006ce570`) — DATA |
| `0x006cec52` | `SuperWeaponTypeClass__ReadINI` first reference — DATA |
| `0x006cecbd` | `SuperWeaponTypeClass__ReadINI` second reference — DATA |
| `0x007e4ce4` | Unknown DATA reference (likely vtable or static struct) |

## Active in YR: Yes

Iron Curtain is a standard YR superweapon. `SuperWeaponTypeClass__ReadINI` is the live YR INI read path — confirmed by the Ghidra label.

## Unverified

- Exact integer stored in `SuperWeaponTypeClass` for the enum index (field offset not decoded in this task).
- `0x007e4ce4` DATA xref purpose not investigated.
- Full enum list beyond index 0–7 is partial; 12 entries total inferred from table bounds but only first 8 confirmed.
