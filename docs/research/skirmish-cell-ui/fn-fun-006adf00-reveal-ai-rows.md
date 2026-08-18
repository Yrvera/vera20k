# FUN_006ADF00 — Reveal Newly-Valid AI Rows

## Summary

Reveals the AI player row controls for a range of row slots in dialog 0x102.
Takes a starting row index and a count; for each row in `[param_2, param_2+param_3)`,
looks up all six control IDs for that row (AI-type combo, slot-0 flag/picture control,
country, color, start-position, team), calls `ShowWindow(hwnd, SW_SHOW=5)` on each,
then calls `FUN_006ADC20` to run the row's enable state machine. Called when the
selected map's start-count grew (more slots available than before).

## Address

`0x006ADF00` (verified via `decompile_function 0x006ADF00`)

## Active in YR

**Yes.** Only caller is `FUN_006ADDF0` (0x006ADDF0) — the row show/hide adjuster
(confirmed via `get_function_callers 0x006ADF00`). `FUN_006ADDF0` itself is called
from the YR-active WM_COMMAND dispatcher and dialog init. No TS-gating flag.

## Signature / Parameters

```c
void __fastcall FUN_006adf00(
    HWND param_1,  // dialog 0x102 HWND
    int  param_2,  // starting row index (0-based, inclusive)
    int  param_3   // count of rows to reveal
)
```

The loop runs while `iVar3 < iVar1` where `iVar1 = param_3 + param_2` (exclusive
upper bound) and `iVar3` starts at `param_2 - 1` (0-based row index).

## Behavioral Analysis

### Row-index to AI-type combo control ID

Same mapping as `FUN_006ADC20` and `FUN_006ACEE0`:

| Row index (iVar3) | AI-type combo control ID |
|---|---|
| 0 | 0x50B |
| 1 | 0x50E |
| 2 | 0x516 |
| 3 | 0x51A |
| 4 | 0x51B |
| 5 | 0x51C |
| 6 | 0x51D |

(verified via `decompile_function 0x006ADF00`)

### Per-row reveal loop

For each row index `iVar3` from `param_2 - 1` up to `param_2 + param_3 - 1`:

```c
param_3         = <AI-type combo ID for this row>
iVar2           = FUN_004E3320()    // slot-0 picture/flag control ID
nIDDlgItem      = FUN_004E41D0()    // country/flag control ID
nIDDlgItem_00   = FUN_004E37D0()    // color/flag control ID
nIDDlgItem_01   = FUN_004E4E60()    // start-position control ID
nIDDlgItem_02   = FUN_004E5940()    // team control ID

hWnd    = GetDlgItem(param_1, param_3);         // AI-type combo
hWnd_00 = GetDlgItem(param_1, iVar2);           // slot-0 picture
hWnd_01 = GetDlgItem(param_1, nIDDlgItem);      // country
hWnd_02 = GetDlgItem(param_1, nIDDlgItem_00);   // color
hWnd_03 = GetDlgItem(param_1, nIDDlgItem_01);   // start-position
hWnd_04 = GetDlgItem(param_1, nIDDlgItem_02);   // team

ShowWindow(hWnd,    SW_SHOW=5);
ShowWindow(hWnd_00, SW_SHOW=5);
ShowWindow(hWnd_01, SW_SHOW=5);
ShowWindow(hWnd_02, SW_SHOW=5);
ShowWindow(hWnd_03, SW_SHOW=5);
ShowWindow(hWnd_04, SW_SHOW=5);

FUN_006ADC20(param_1, param_3);   // run row enable state machine
```

After making controls visible, `FUN_006ADC20` determines whether to enable or
disable the country/color/start/team controls based on the current AI-type combo
item-data (if the row is inactive, controls are shown but immediately disabled).

(verified via `decompile_function 0x006ADF00`)

## Callees

Confirmed via `get_function_callees 0x006ADF00`:
- `FUN_004E3320` (0x004E3320) — slot-0 picture/flag control ID helper (task #18)
- `FUN_004E41D0` (0x004E41D0) — country control ID helper (task #27)
- `FUN_004E37D0` (0x004E37D0) — color/flag control ID helper (task #21)
- `FUN_004E4E60` (0x004E4E60) — start-position control ID helper (task #36)
- `FUN_004E5940` (0x004E5940) — team control ID helper (task #44)
- `FUN_006ADC20` (0x006ADC20) — per-row enable state machine (task #3, decoded)
- `GetDlgItem`, `ShowWindow` — Win32

## Out-of-scope refs

- `FUN_004E3320` — slot-0 picture helper; task #18 in scope
- `FUN_004E41D0`, `FUN_004E37D0`, `FUN_004E4E60`, `FUN_004E5940` — control-ID
  helpers; tasks #27, #21, #36, #44 in scope

## Unverified (YELLOW)

- The exact row-index argument passing into `FUN_004E3320`, `FUN_004E41D0`, etc.
  (how they know which row's control ID to return when called without explicit
  row-index argument in the decompile) — implicit global or combo-ID register
  argument not visible in the decompile. Confirmed to be in scope for those tasks.
