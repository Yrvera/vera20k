# FUN_006ADC20 — Per-Row Enable State Machine

## Summary

Per-row enable/disable state machine for dialog 0x102 player-slot rows 0–6. Called
whenever an AI-type combo changes. Reads the combo's current selection item-data:
if 0/1/2 (Easy/Medium/Hard AI active), the row's four sibling controls
(country, color, start-position, team) are enabled. Otherwise the row is inactive:
three sentinel writers force country/color/start combos to –2, team sentinel is set
to 3 or –2 based on AlliesAllowed, and all four sibling controls are disabled.
Also propagates team-sentinel values across all rows in range when called from the
reveal/hide callers.

## Address

`0x006ADC20` — body `0x006ADC20..0x006ADDE4`
(verified via `decompile_function 0x006ADC20`)

## Active in YR

**Yes.** Called by three in-scope YR-active functions
(verified via `get_function_callers 0x006ADC20` and `get_xrefs_to 0x006ADC20`):
- `FUN_006ACEE0` @ `0x006ACF35` — WM_COMMAND dispatcher (when AI-type combo changes)
- `FUN_006ADF00` @ `0x006AE056` — reveal newly-valid AI rows
- `FUN_006AE080` @ `0x006AE146` — hide AI rows beyond map start count

No TS-gating flag. Fires on every AI-type combo change in a standard YR skirmish
session, and once per row during map-change row adjustment.

## Signature / Parameters

```c
void __fastcall FUN_006adc20(
    HWND param_1,   // dialog 0x102 HWND
    int  param_2    // AI-type combo control ID for this row (0x50B..0x51D)
)
```

## Control-ID to Row-Index Mapping

The function converts `param_2` (combo control ID) to a 0-based row index `iVar3`,
then derives 1-based `iVar1 = iVar3 + 1`:

| param_2 | row index (iVar3) | iVar1 |
|---|---|---|
| 0x50B | 0 | 1 |
| 0x50E | 1 | 2 |
| 0x516 | 2 | 3 |
| 0x51A | 3 | 4 |
| 0x51B | 4 | 5 |
| 0x51C | 5 | 6 |
| 0x51D | 6 | 7 |
| other | 6+something | overflows to 7 |

(verified via `decompile_function 0x006ADC20`)

## Sibling Control IDs

Four helper calls resolve the sibling control IDs for the row before the item-data
test:
```c
nIDDlgItem    = FUN_004E41D0()   // country/flag control ID
nIDDlgItem_00 = FUN_004E37D0()   // color control ID (or vice versa)
nIDDlgItem_01 = FUN_004E4E60()   // start-position control ID
nIDDlgItem_02 = FUN_004E5940()   // team control ID
```

These per-row helpers (tasks #27, #21, #36, #44) return control IDs based on the
currently-indexed row. The exact row-index argument passing is implicit — they read
a shared row-index global or use the combo ID as the index.

## Team-Sentinel Propagation Loop (AlliesAllowed gate)

Before the item-data test, the function runs a team-sentinel propagation loop when
`param_1 != NULL` and `DAT_00A8B23C != 0` (SelectedMode pointer is valid):

```c
iVar5 = 8;
iVar2 = 1;
if (0 < iVar1 && iVar1 < 8) {
    iVar5 = iVar3 + 2;   // end = row_index + 2
    iVar2 = iVar1;        // start = row_index + 1
}
for (; iVar2 < iVar5; iVar2++) {
    FUN_004E5940();       // set team combo row context
    if (DAT_00A8B23C == 0 || *(char *)(DAT_00A8B23C + 0x3C) == '\0') {
        uVar6 = 0xFFFFFFFE;   // sentinel -2: AlliesAllowed == false
    } else {
        uVar6 = 3;             // value 3: AlliesAllowed == true
    }
    FUN_004E5ED0(uVar6);  // write team sentinel for this row
}
```

Key: `*(char *)(DAT_00A8B23C + 0x3C)` — reads byte at offset `+0x3C` from the
SelectedMode object pointer. This is the AlliesAllowed flag (boolean byte).
`DAT_00A8B23C` is the SelectedMode pointer global
(confirmed via `get_xrefs_to 0x00A8B23C` — read in `FUN_006ADC20` and
`FUN_006ACEE0`; manifest entry: "Selected mode pointer; +0x3C byte is AlliesAllowed").

When called from `FUN_006ADF00` / `FUN_006AE080` (row reveal/hide), `iVar1` may be
outside the 1–7 range — the guard `0 < iVar1 < 8` falls through, setting
`iVar2=1, iVar5=8` and thus propagating team sentinels across all 7 rows.

## Item-Data Test and Enable / Disable Logic

```c
wParam = SendDlgItemMessageA(param_1, param_2, CB_GETCURSEL=0x147, 0, 0);
LVar4  = SendDlgItemMessageA(param_1, param_2, CB_GETITEMDATA=0x150, wParam, 0);
if (LVar4 == 0 || LVar4 == 1 || LVar4 == 2) {
    local_8 = TRUE;    // row active (Easy/Medium/Hard AI)
} else {
    local_8 = FALSE;   // row inactive
    FUN_004E49A0(0xFFFFFFFE);   // color sentinel -2
    FUN_004E3F70(0xFFFFFFFE);   // country sentinel -2
    FUN_004E5480(0xFFFFFFFE);   // start-position sentinel -2
    // team sentinel already handled in AlliesAllowed loop above
}
hWnd    = GetDlgItem(param_1, nIDDlgItem);     // country control
hWnd_00 = GetDlgItem(param_1, nIDDlgItem_00);  // color control
hWnd_01 = GetDlgItem(param_1, nIDDlgItem_01);  // start-pos control
hWnd_02 = GetDlgItem(param_1, nIDDlgItem_02);  // team control
EnableWindow(hWnd,    local_8);
EnableWindow(hWnd_00, local_8);
EnableWindow(hWnd_01, local_8);
EnableWindow(hWnd_02, local_8);
```

(verified via `decompile_function 0x006ADC20`)

**Sentinel writers called on inactive row:**
| Function | Sentinel written | Target combo |
|---|---|---|
| `FUN_004E49A0` (task #33) | –2 (`0xFFFFFFFE`) | Color |
| `FUN_004E3F70` (task #25) | –2 (`0xFFFFFFFE`) | Country |
| `FUN_004E5480` (task #41) | –2 (`0xFFFFFFFE`) | Start-position |
| `FUN_004E5ED0` (task #48) | –2 or 3 | Team (AlliesAllowed-dependent) |

Note: team sentinel is written in the AlliesAllowed loop, not in the inactive branch
directly — it runs unconditionally before the item-data test.

## Callees

Confirmed via `get_function_callees 0x006ADC20`:
- `FUN_004E41D0` (0x004E41D0) — row helper, returns country control ID
- `FUN_004E37D0` (0x004E37D0) — row helper, returns color/flag control ID
- `FUN_004E4E60` (0x004E4E60) — row helper, returns start-pos control ID
- `FUN_004E5940` (0x004E5940) — team row helper / context setter
- `FUN_004E3F70` (0x004E3F70) — country sentinel writer
- `FUN_004E49A0` (0x004E49A0) — color sentinel writer
- `FUN_004E5480` (0x004E5480) — start-position sentinel writer
- `FUN_004E5ED0` (0x004E5ED0) — team sentinel writer
- `SendDlgItemMessageA`, `GetDlgItem`, `EnableWindow` — Win32

## Out-of-scope refs

- `FUN_004E41D0`, `FUN_004E37D0`, `FUN_004E4E60`, `FUN_004E5940` — per-row
  control-ID helpers; individual decode tasks #27, #21, #36, #44 in scope.
- `FUN_004E3F70`, `FUN_004E49A0`, `FUN_004E5480`, `FUN_004E5ED0` — sentinel
  writers; decode tasks #25, #33, #41, #48 in scope.
- `DAT_00A8B23C + 0x3C` AlliesAllowed byte — parent object layout belongs to
  decode-global-selectedmode-alliesallowed-ptr (task #68).

## Unverified (YELLOW)

- The exact per-row-index argument-passing convention for `FUN_004E41D0`,
  `FUN_004E37D0`, `FUN_004E4E60`, `FUN_004E5940` (how they know which row
  to return IDs for) is not confirmed here — those functions take their row
  context from implicit globals or the `param_2` combo ID; to be confirmed
  by decoders for tasks #27, #21, #36, #44.
