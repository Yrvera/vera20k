# FUN_004E3690 — Cell Command Handler (Country Combo CBN_SELCHANGE)

## Summary

`FUN_004E3690` handles a CBN_SELCHANGE notification on a country combo control for one of the 8 player-slot rows. It takes the dialog HWND and a slot index (0-7). It maps the slot index to a flag-image static control ID (0x6DA..0x6E1) and a country combo control ID (0x6A1 or 0x510..0x521), then reads the current country selection's item-data via `CB_GETCURSEL`/`CB_GETITEMDATA`. If the item-data is out of the valid range (-3..9), it calls a vtable method on the selected-mode object at `vtable+0x28` (inferred: random-start trigger or country-conflict resolver). Finally calls `FUN_004E3560` (side item-data → flag PCX lookup/update) and `FUN_00603D30` (owner-draw combo invalidation). The only caller is `FUN_006ACEE0` (WM_COMMAND dispatcher).

## Active in YR

**Yes.** Sole caller is `FUN_006ACEE0` (verified via `get_function_callers 0x004E3690`), which is the WM_COMMAND dispatcher for dialog 0x102, reachable from the standard YR offline Skirmish flow. No TS-only gate present.

## Decompilation excerpt (verified via `decompile_function 0x004E3690`)

```c
void FUN_004e3690(HWND param_1, int param_2)  // param_2 = slot index 0-7
{
    // Map slot → flag static control ID (0x6DA..0x6E1)
    int flagID;
    if      (param_2 == 0) flagID = 0x6da;
    else if (param_2 == 1) flagID = 0x6db;
    else if (param_2 == 2) flagID = 0x6dc;
    else if (param_2 == 3) flagID = 0x6dd;
    else if (param_2 == 4) flagID = 0x6de;
    else if (param_2 == 5) flagID = 0x6df;
    else if (param_2 == 6) flagID = 0x6e0;
    else                   flagID = 0x6e1;  // slot 7
    GetDlgItem(param_1, flagID);  // retrieve flag static HWND (result unused — triggers side effect?)

    // Map slot → country combo control ID (same as FUN_004e37d0)
    int countryID;
    if      (param_2 == 0) countryID = 0x6a1;
    else if (param_2 == 1) countryID = 0x510;
    else if (param_2 == 2) countryID = 0x513;
    else if (param_2 == 3) countryID = 0x51e;
    else if (param_2 == 4) countryID = 0x514;
    else if (param_2 == 5) countryID = 0x51f;
    else if (param_2 == 6) countryID = 0x520;
    else                   countryID = 0x521; // slot 7

    // Read current selection's item-data
    WPARAM wParam = SendDlgItemMessageA(param_1, countryID, 0x147, 0, 0); // CB_GETCURSEL
    LRESULT LVar2 = SendDlgItemMessageA(param_1, countryID, 0x150, wParam, 0); // CB_GETITEMDATA

    // If item-data is outside valid country range and selected-mode is set:
    // call vtable+0x28 on selected-mode object (random-start or conflict resolve)
    if (((LVar2 < -3) || (9 < LVar2)) && (DAT_00a8b23c != (int *)0x0)) {
        (**(code **)(*DAT_00a8b23c + 0x28))();
    }

    FUN_004e3560();   // update flag PCX for this country selection
    FUN_00603d30();   // invalidate owner-draw combo for redraw
}
```

## Behavioral analysis

### Two-phase slot mapping

The function duplicates the slot→ID mapping twice:
1. First maps slot → flag static control ID (0x6DA..0x6E1)
2. Then maps slot → country combo control ID (same as `FUN_004E37D0`)

The flag static control displays a small flag PCX image for the selected country. The `GetDlgItem` call for the flag control retrieves its HWND but the result is not stored — this may be for a side effect in certain configurations, or the result is implicitly used by `FUN_004E3560` which updates the flag image.

### Flag static control IDs

| Slot | Flag static ID |
|------|----------------|
| 0    | 0x6DA          |
| 1    | 0x6DB          |
| 2    | 0x6DC          |
| 3    | 0x6DD          |
| 4    | 0x6DE          |
| 5    | 0x6DF          |
| 6    | 0x6E0          |
| 7    | 0x6E1          |

### Country combo control IDs

Same mapping as `FUN_004E37D0` (verified via `decompile_function 0x004E3690`):

| Slot | Country combo ID |
|------|-----------------|
| 0    | 0x6A1           |
| 1    | 0x510           |
| 2    | 0x513           |
| 3    | 0x51E           |
| 4    | 0x514           |
| 5    | 0x51F           |
| 6    | 0x520           |
| 7    | 0x521           |

### Item-data validity check

The country combo item-data represents a country/side index. Valid range is -3 to 9. Values outside this range trigger a vtable call on the selected-mode object at `vtable+0x28`. The comment in the task manifest notes that country items include special values for "Random" or similar entries — the out-of-range check guards against invalid selections.

### Vtable call on selected-mode object (YELLOW)

When country item-data is outside -3..9, `(**(code **)(*DAT_00a8b23c + 0x28))()` is called — a vtable method at slot offset 0x28 on the selected-mode object. This is inferred to handle random-assignment or selection-conflict resolution for the selected game mode. The vtable content at `+0x28` is not verified in this decode.

### FUN_004E3560 — flag image update

`FUN_004E3560` (in scope as task #19) is called with the slot/dialog context to update the flag PCX image in the flag static control based on the newly selected country. This produces the visible flag-image update observable in the dialog.

### FUN_00603D30 — owner-draw combo invalidation

`FUN_00603D30` (verified via `decompile_function 0x00603D30`) looks up the owner-draw combo registration hash table using `param_1` (the dialog HWND) and `param_2` (combo item index). Sets `piVar2[0x1D] = 2` (state flag) and `piVar2[6] = param_2` (current item), then calls `InvalidateRect` to trigger a repaint of the combo control. This produces the observable owner-draw combo swatch refresh.

## Struct field accesses

| Pointer | Offset | Unit | Usage | Frame |
|---------|--------|------|-------|-------|
| `DAT_00A8B23C` (selected-mode ptr) | vtable `+0x28` | vtable slot | Called when item-data out of range | internal vtable |
| owner-draw combo entry (from hash table) | `+0x74` (`[0x1D]`) | int | State flag set to 2 | internal combo struct |
| owner-draw combo entry | `+0x18` (`[6]`) | int | Current item index | internal combo struct |

## Globals referenced

| Global | Address | Role |
|--------|---------|------|
| `DAT_00A8B23C` | 0x00A8B23C | Selected-mode object pointer — vtable+0x28 called on out-of-range country |
| `DAT_00AC1B04` | 0x00AC1B04 | Owner-draw registration table validity flag (inside FUN_00603D30) |
| `DAT_00AC1B00` | 0x00AC1B00 | Owner-draw combo hash table base (inside FUN_00603D30) |

## Callers

- `FUN_006ACEE0` @ 0x006ACEE0 — WM_COMMAND dispatcher (confirmed via `get_function_callers 0x004E3690`)

## Callees

| Address | Name | Role |
|---------|------|------|
| 0x004E3560 | FUN_004e3560 | Flag PCX update for selected country |
| 0x00603D30 | FUN_00603d30 | Owner-draw combo invalidation / redraw trigger |
| Win32 | GetDlgItem | Get flag static HWND |
| Win32 | SendDlgItemMessageA | CB_GETCURSEL (0x147), CB_GETITEMDATA (0x150) |

## Out-of-scope refs

- `FUN_00603D30` owner-draw invalidation — shared owner-draw infrastructure, out of cell combo scope for independent decode; behavior documented here inline.
- Vtable call `*DAT_00a8b23c + 0x28` — selected-mode vtable, covered by selected-mode system.

## TS-filter

Sole caller is the YR dialog WM_COMMAND path. No TS-only gating. **TS-legacy score: 0.0.**

## Unverified claims (YELLOW)

- Vtable call `*DAT_00a8b23c + 0x28` semantic — inferred as "random-start or conflict resolver"; actual vtable content at offset 0x28 in the selected-mode class was not independently read via `read_memory`. This is the only load-bearing unverified claim.
- Country item-data valid range `-3..9` — the boundary check in the decompile is `(LVar2 < -3) || (9 < LVar2)`. The meaning of values -3..9 (country indices + special values) is inferred from context; the full country list enumeration is in task #23 (FUN_004e3b90).
- `GetDlgItem(param_1, flagID)` result unused in Ghidra decompile — observed in decompilation; the returned HWND is not stored to any local. This may be a Ghidra decompile artifact or genuinely unused.
