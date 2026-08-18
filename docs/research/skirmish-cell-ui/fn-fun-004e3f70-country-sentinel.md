# FUN_004E3F70 — Country Combo Selection Setter (Explicit Value)

## Summary

Takes a dialog HWND, a country-combo control ID, and an explicit target item-data
value `param_3`. Hides the combo, scans all items for one whose CB_GETITEMDATA
equals `param_3`, selects it via CB_SETCURSEL, then re-shows the combo. After the
scan-and-select, reverse-maps the control ID to (row-index, flag-static ID),
reads the current selection's item-data, and if out of the valid range −3..9
calls vtable slot `+0x28` on the selected-mode object. Finally calls
`FUN_004E3560` (flag PCX loader) and `FUN_00603D30` (owner-draw invalidation).

Differs from `FUN_004E3CE0` (task #24) in that it takes the target value as an
explicit `param_3` rather than reading it from globals `DAT_00A8B3BC`/`DAT_00A8B3C0`.

## Address

`0x004E3F70` (verified via `decompile_function 0x004E3F70`)

## Active in YR

**Yes.** In-scope callers include `FUN_006ADC20` (0x006ADC20, row-enable state machine,
YR-active anchor) and `FUN_006AE6E0` (0x006AE6E0, dialog init, YR-active anchor).
(Callers confirmed via `get_function_callers 0x004E3F70`)

## Signature / Parameters

```c
void __fastcall FUN_004e3f70(
    HWND  param_1,   // dialog 0x102 HWND
    int   param_2,   // country-combo control ID (e.g. 0x6A1, 0x510..0x521)
    int   param_3    // target item-data value to select
)
```

(verified via `decompile_function 0x004E3F70`)

## Behavioral Analysis

### Step 1 — Hide-scan-select-show loop

```c
hWnd = GetDlgItem(param_1, param_2);
BVar1 = ShowWindow(hWnd, 0);           // hide; save return (was-visible bool)
LVar2 = SendDlgItemMessageA(param_1, param_2, 0x146, 0, 0);  // CB_GETCOUNT
WVar6 = 0;
if (0 < LVar2) {
    do {
        LVar3 = SendDlgItemMessageA(param_1, param_2, 0x150, WVar6, 0); // CB_GETITEMDATA
        if (LVar3 == param_3) {
            SendDlgItemMessageA(param_1, param_2, 0x14e, WVar6, 0);     // CB_SETCURSEL
            break;
        }
        WVar6++;
    } while ((int)WVar6 < LVar2);
}
if (BVar1 != 0) ShowWindow(hWnd, 5);  // SW_SHOW if was visible
```

The hide-before-scan pattern avoids a visible flash when programmatically changing
the selection. If no item matches `param_3`, the combo stays at its prior selection.
(verified via `decompile_function 0x004E3F70`)

### Step 2 — Reverse-map control ID to (row-index, flag-static ID)

```c
if      (param_2 == 0x6a1) { iVar5 = 0; iVar4 = 0x6da; }
else if (param_2 == 0x510) { iVar5 = 1; iVar4 = 0x6db; }
else if (param_2 == 0x513) { iVar5 = 2; iVar4 = 0x6dc; }
else if (param_2 == 0x51e) { iVar5 = 3; iVar4 = 0x6dd; }
else if (param_2 == 0x514) { iVar5 = 4; iVar4 = 0x6de; }
else if (param_2 == 0x51f) { iVar5 = 5; iVar4 = 0x6df; }
else if (param_2 == 0x520) { iVar5 = 6; iVar4 = 0x6e0; }
else if (param_2 == 0x521) { iVar5 = 7; iVar4 = 0x6e1; }
else                        { iVar5 = -1; iVar4 = -1;   }
GetDlgItem(param_1, iVar4);   // result unused — artifact or implicit side-effect
```

The result of `GetDlgItem(param_1, iVar4)` is not stored — the same pattern as in
`FUN_004E3CE0` and `FUN_004E3690`. Likely a Ghidra decompile artifact.
(verified via `decompile_function 0x004E3F70`)

### Step 3 — Out-of-range item-data check and vtable call

After the reverse-map, the function re-maps `iVar5` back to a country-combo ID
(identical chain as `FUN_004E37D0`, task #21) and reads the current selection:

```c
WVar6 = SendDlgItemMessageA(param_1, iVar4, 0x147, 0, 0); // CB_GETCURSEL
LVar2 = SendDlgItemMessageA(param_1, iVar4, 0x150, WVar6, 0); // CB_GETITEMDATA
if (((LVar2 < -3) || (9 < LVar2)) && (DAT_00a8b23c != (int *)0x0)) {
    (**(code **)(*DAT_00a8b23c + 0x28))();
}
```

Valid item-data range −3..9 matches the observer (−3), random (−2), and 0–9 country
indices documented in `FUN_004E3560` (task #19). Out-of-range triggers vtable `+0x28`
on `DAT_00A8B23C` (selected-mode object).
(verified via `decompile_function 0x004E3F70`)

### Step 4 — Flag update and owner-draw refresh

```c
FUN_004e3560();    // side/flag PCX loader (task #19)
FUN_00603d30();    // owner-draw combo invalidation (out of scope)
```

## Control-ID to Row/Flag-Static Mapping

| Country combo ID (param_2) | Row index | Flag static ID |
|---|---|---|
| 0x6A1 | 0 | 0x6DA |
| 0x510 | 1 | 0x6DB |
| 0x513 | 2 | 0x6DC |
| 0x51E | 3 | 0x6DD |
| 0x514 | 4 | 0x6DE |
| 0x51F | 5 | 0x6DF |
| 0x520 | 6 | 0x6E0 |
| 0x521 | 7 | 0x6E1 |
| other | −1 | −1 |

## Globals Accessed

| Global | Address | Usage |
|---|---|---|
| `DAT_00A8B23C` | `0x00A8B23C` | Selected-mode pointer; vtable `+0x28` called on out-of-range item-data |

(confirmed via `decompile_function 0x004E3F70`)

## Callers

In-scope YR-active callers:
- `FUN_006ADC20` @ `0x006ADC20` — row-enable state machine (anchor task #3)
- `FUN_006AE6E0` @ `0x006AE6E0` — dialog init (anchor task #1)

Out-of-scope callers: `FUN_0046F6D0`, `FUN_005C2500`, `FUN_005E9E70`,
`FUN_005EA3E0`, `FUN_005EC3A0`, `FUN_005ED5A0`.

(confirmed via `get_function_callers 0x004E3F70`)

## Callees

- `FUN_004E3560` (0x004E3560) — flag PCX lookup + loader (task #19, decoded)
- `FUN_00603D30` (0x00603D30) — owner-draw combo invalidation (out of scope)
- `GetDlgItem`, `ShowWindow`, `SendDlgItemMessageA` — Win32

## Relationship to FUN_004E3CE0 (task #24)

`FUN_004E3CE0` and `FUN_004E3F70` are near-identical; the only structural difference
is how the target country value is obtained:

| | `FUN_004E3CE0` | `FUN_004E3F70` |
|---|---|---|
| Target value source | Reads `DAT_00A8B3BC` / `DAT_00A8B3C0` globals | Explicit `param_3` |
| param_2 meaning | 0-based row index | Country-combo control ID directly |

`FUN_004E3CE0` maps its row-index `param_2` to a control ID first; `FUN_004E3F70`
takes the control ID directly as `param_2` and uses it both for the CB operations
and the reverse-map lookup.

## Out-of-scope refs

- `FUN_00603D30` — owner-draw infrastructure; not in cell-UI scope
- Vtable `+0x28` on `DAT_00A8B23C` — selected-mode vtable; covered by task #68

## Unverified (YELLOW)

- `GetDlgItem(param_1, iVar4)` result unused in decompile: same pattern as tasks #20
  and #24; likely a Ghidra artifact. Not re-verified in this session.
- Vtable call `*DAT_00A8B23C + 0x28` purpose: inferred as random-start or
  conflict-resolver from usage pattern; actual vtable content not read.
