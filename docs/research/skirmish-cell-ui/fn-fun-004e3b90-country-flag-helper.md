# FUN_004E3B90 — Country/Flag Combo Loader (All 8 Rows)

## Summary

Iterates all 8 player-slot rows (0–7) and populates each row's country combo
with items. For each row, it first calls `FUN_005E9B20(-1)` to check if the
current session matches `DAT_00AC11B4` (network/spectator guard). If it
matches, the combo is populated via a CB_ADDSTRING / CB_SETITEMDATA sequence
using string ID `0x135` and special item-data `-3`. Otherwise delegates to
`FUN_004E3A00` for the normal population path.

Active in YR: **Yes** — called from `FUN_006AE6E0` (dialog init, YR-active)
and `FUN_005C3F30` (out-of-scope YR function).
(verified via `get_function_callers 0x004E3B90`)

## Address

`0x004E3B90`
(verified via `decompile_function 0x004E3B90`)

## Signature / Parameters

```c
void __fastcall FUN_004e3b90(HWND param_1)  // dialog HWND
```

(verified via `decompile_function 0x004E3B90`)

## Callers

- `FUN_006AE6E0` @ `0x006AE6E0` — dialog init handler
- `FUN_005C3F30` @ `0x005C3F30` — out-of-scope YR caller

(verified via `get_function_callers 0x004E3B90`)

## Behavioral Analysis

### Main loop (rows 0–7)

```c
local_4 = 0;
do {
    iVar2 = FUN_005E9B20(0xFFFFFFFF);
    bool bVar1 = (DAT_00AC11B4 != 0) && (iVar2 == DAT_00AC11B4);
    // map local_4 → country combo ID (same table: 0→0x6A1, 1→0x510, ..., 7→0x521)
    if (bVar1) {
        // spectator/observer branch: add "Observer" string (ID 0x135, item-data -3)
        ...
    } else {
        FUN_004E3A00();  // normal country population
    }
    local_4++;
} while (local_4 < 8);
```

(verified via `decompile_function 0x004E3B90`)

### Spectator branch (when `iVar2 == DAT_00AC11B4`)

When the session check fires (network spectator mode), the function:

1. `GetDlgItem(param_1, comboId)` → hWnd
2. `ShowWindow(hWnd, SW_HIDE)` — hide before update
3. `SendMessageA(hWnd, 0x14B, ...)` — CB_RESETCONTENT (clears all items)
4. `SendMessageA(hWnd, 0x4DE, 0, 7)` — CB_SETMINVISIBLE; sets min visible items = 7
5. `StringTable__LoadString(s_GDlgSupp, 0x135)` → "Observer" string pointer
6. `SendMessageA(hWnd, 0x4C2, 0, lParam)` — CB_INSERTSTRING at position 0
7. `SendMessageA(hWnd, 0x151, wParam, -3)` — CB_SETITEMDATA; item-data = -3 (observer sentinel)
8. `SendMessageA(hWnd, 0x14E, 0, 0)` — CB_SETCURSEL; select item 0
9. `SendMessageA(hWnd, 0x4F1, 0, 1)` — CB_SETCUEBANNER or similar (extended combo msg)
10. Re-show via `ShowWindow(hWnd, SW_SHOW)` if it was visible before.

The item-data value `-3` matches the `PTR_s_obsi_pcx` ("Observer") case in
`FUN_004E3560` (the side→flag-PCX mapper).

(verified via `decompile_function 0x004E3B90`)

### Normal branch

`FUN_004E3A00()` — delegates full country combo population to an out-of-scope
helper. Not decoded in this task.

## Observed Globals

| Global | Address | Access | Role |
|---|---|---|---|
| `DAT_00AC11B4` | `0x00AC11B4` | READ | Network session ID for spectator-mode check |

## Out-of-scope refs

- `FUN_005E9B20` — session/network query helper; not in cell-UI scope
- `FUN_004E3A00` — normal country combo population; not in current scope
  (out-of-scope ref to be noted in manifest)
- `FUN_005C3F30` — caller outside cell scope

## Unverified (YELLOW)

- `FUN_005E9B20(-1)` semantics: inferred as "get current network session ID"
  from usage pattern (return compared against `DAT_00AC11B4`); not decompiled
  in this task.
- `FUN_004E3A00` normal population path: not decompiled. Its behavior is
  inferred from the "else" branch; it likely populates all country items with
  item-data 0–9 (matching `FUN_004E3560` range).
- Message code `0x4F1` (CB_SETCUEBANNER or similar extended combo message):
  not verified against Windows SDK documentation in this decode pass.
