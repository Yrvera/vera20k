# FUN_004E3CE0 — Country/Flag Combo Initialization Helper

## Summary

Per-row country/flag combo setup function. Takes a dialog HWND and a 0-based row
index. Briefly hides the country combo, scans all its items via `CB_GETITEMDATA`
to find the item matching the current country selection (from globals), selects it
via `CB_SETCURSEL`, re-shows the combo if it was visible. Then maps the row to
its corresponding flag-static control ID, fetches the flag-static HWND (result
unused), reads the current country-combo item-data to detect if it's out of range, and
if so calls a vtable method on `DAT_00A8B23C` (`+0x28` slot). Finally calls
`FUN_004E3560` (PCX flag loader) and `FUN_00603D30` (unknown — likely redraws the
flag static).

## Address

`0x004E3CE0` (verified via `decompile_function 0x004E3CE0`)

## Active in YR

**Yes.** Called by `FUN_006AE6E0` (0x006AE6E0, dialog init, YR-active anchor).
(Callers confirmed via `get_function_callers 0x004E3CE0`)

## Signature / Parameters

```c
void __fastcall FUN_004e3ce0(
    HWND param_1,  // dialog 0x102 HWND
    int  param_2   // 0-based row index (0–7)
)
```

## Behavioral Analysis

### Step 1 — Determine target country value

```c
local_4 = DAT_00A8B3BC;       // default: some global country selection
if (DAT_00A8B3C0 == -2) local_4 = -2;   // random
if (DAT_00A8B3C0 == -3) local_4 = -3;   // observer
```

`DAT_00A8B3BC` (0x00A8B3BC) and `DAT_00A8B3C0` (0x00A8B3C0) are session globals
holding the saved country and its mode override (random = –2, observer = –3).
(verified via `decompile_function 0x004E3CE0`)

### Step 2 — Map row index to country-combo control ID

Same mapping as `FUN_004E37D0` (task #21):
- Row 0 → 0x6A1, row 1 → 0x510, ..., row 7 → 0x521

### Step 3 — Scan-and-select the matching item

```c
hWnd = GetDlgItem(param_1, iVar4);
BVar1 = ShowWindow(hWnd, 0);          // hide combo; saves visibility state
LVar2 = SendDlgItemMessageA(param_1, iVar4, CB_GETCOUNT=0x146, 0, 0);
for (WVar6 = 0; WVar6 < LVar2; WVar6++) {
    LVar3 = SendDlgItemMessageA(param_1, iVar4, CB_GETITEMDATA=0x150, WVar6, 0);
    if (LVar3 == local_4) {
        SendDlgItemMessageA(param_1, iVar4, CB_SETCURSEL=0x14E, WVar6, 0);
        break;
    }
}
if (BVar1 != 0) ShowWindow(hWnd, SW_SHOW=5);   // re-show if was visible
```

The hide-scan-show pattern avoids a flash when programmatically setting the
selection.
(verified via `decompile_function 0x004E3CE0`)

### Step 4 — Map to flag-static control ID

After the combo scan, the function maps the country-combo control ID to the
corresponding flag-static control ID (same mapping as `FUN_004E3320`, task #18):

| Country combo ID | Row index (iVar5) | Flag static ID |
|---|---|---|
| 0x6A1 | 0 | 0x6DA |
| 0x510 | 1 | 0x6DB |
| 0x513 | 2 | 0x6DC |
| 0x51E | 3 | 0x6DD |
| 0x514 | 4 | 0x6DE |
| 0x51F | 5 | 0x6DF |
| 0x520 | 6 | 0x6E0 |
| 0x521 | 7 | 0x6E1 |
| other | -1 | -1 |

`GetDlgItem(param_1, iVar4)` is called on the flag-static ID but the result is
not used — this is likely a no-op artifact or the HWND is consumed indirectly.

### Step 5 — Out-of-range country-combo check

After the flag-static ID mapping, the function re-runs the row-index → country-combo
ID mapping a second time (using the derived `iVar5` row index). Then reads
`CB_GETCURSEL` + `CB_GETITEMDATA` on that **country combo** (not the AI-type combo)
and range-checks the result:

```c
// Second iVar4 reassignment: iVar5 (row index) → country-combo control ID
// (same mapping as Step 2; iVar5 in 0..7 → iVar4 in {0x6A1,0x510,...,0x521})
// For iVar5 outside 0..7: (-(uint)(iVar5!=7) & 0xFFFFFADE) + 0x521
//   iVar5==7:  -(uint)(false) = 0; 0 & 0xFFFFFADE = 0; 0 + 0x521 = 0x521  ✓
//   iVar5!=7:  -(uint)(true) = 0xFFFFFFFF; 0xFFFFFFFF & 0xFFFFFADE = 0xFFFFFADE;
//              0xFFFFFADE + 0x521 = 0xFFFFFFFF = -1  (out-of-range → iVar4=-1)
WVar6 = SendDlgItemMessageA(param_1, iVar4, CB_GETCURSEL=0x147, 0, 0);
LVar2 = SendDlgItemMessageA(param_1, iVar4, CB_GETITEMDATA=0x150, WVar6, 0);
if ((LVar2 < -3 || LVar2 > 9) && DAT_00A8B23C != NULL) {
    (**(code **)(*DAT_00A8B23C + 0x28))();   // vtable slot +0x28 on SelectedMode
}
```

If the current country-combo item-data is outside the valid range [–3..9], calls vtable
slot `+0x28` on the SelectedMode object. Byte offset `+0x28` on an `int*`-typed vtable
pointer = slot index `0x28/4 = 0xA` (10th slot). Purpose unclear — possibly resets
or validates the selection.
(verified via `decompile_function 0x004E3CE0`)

### Step 6 — Flag PCX update

```c
FUN_004E3560();    // load flag PCX for current country selection (task #19)
FUN_00603D30();    // unknown — likely refreshes/redraws the flag static
```

`FUN_004E3560` (task #19, decoded) maps the country item-data to its PCX filename
and calls the loader. `FUN_00603D30` (0x00603D30) is out of scope.

## Globals Accessed

| Global | Address | Usage |
|---|---|---|
| `DAT_00A8B3BC` | `0x00A8B3BC` | Default country selection |
| `DAT_00A8B3C0` | `0x00A8B3C0` | Country override (-2=random, -3=observer) |
| `DAT_00A8B23C` | `0x00A8B23C` | SelectedMode pointer (vtable call target) |

(All confirmed from decompile body; `decompile_function 0x004E3CE0`)

## Callees

Confirmed via `get_function_callees 0x004E3CE0`:
- `FUN_004E3560` (0x004E3560) — side/flag PCX lookup + loader (task #19, decoded)
- `FUN_00603D30` (0x00603D30) — unknown, likely flag-static redraw; out of scope
- `GetDlgItem`, `SendDlgItemMessageA`, `ShowWindow` — Win32

## Out-of-scope refs

- `FUN_00603D30` (0x00603D30) — flag display/redraw helper; not in scope
- `DAT_00A8B3BC`, `DAT_00A8B3C0` — session country globals; covered under
  decode-struct-sessionclass-slots-slice (task #65)
- vtable slot `+0x28` on SelectedMode — covered under task #68

## Unverified (YELLOW)

- The purpose of `(**(code **)(*DAT_00A8B23C + 0x28))()` (vtable slot 0xA on
  SelectedMode) is unknown — decompile shows a vtable call but no evidence of
  which method this is or what observable effect it produces. Marked out-of-scope
  for task #68.
