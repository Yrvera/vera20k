# FUN_004E49A0 --- Color Combo Sentinel Writer and Color Table Updater

## Summary

Three-phase function that (1) clears the current row's color table ownership,
(2) selects a target color item in the combo and writes its row ownership back
to the color table, then (3) refreshes all 8 rows' color combos. The net
effect is an atomic color re-assignment: the old color is released, the new
color is claimed by this row, and all combo lists are repopulated to reflect
the new state.

param_3 is the target color item-data (color index or -2 for random). If param_3
is -2 (random sentinel), step 2 still selects that item but skips the color-table
ownership write.

## Address

`0x004E49A0` (verified via `decompile_function 0x004E49A0`)

## Active in YR

**Yes.** Called by `FUN_006ADC20` (0x006ADC20, row enable state machine, task #3,
YR-active) and `FUN_006AE6E0` (0x006AE6E0, dialog init, task #1, YR-active).
Also called by `FUN_004E48E0` (0x004E48E0, task #32, in-scope) and several
out-of-scope callers.
(Confirmed via `get_function_callers 0x004E49A0`)

## Signature / Parameters

```c
void __fastcall FUN_004e49a0(
    HWND param_1,   // dialog 0x102 HWND
    int  param_2,   // color combo control ID (0x6A2, 0x522..0x528)
    int  param_3    // target color item-data (color index 0-7, or -2 = random)
)
```

(verified via `decompile_function 0x004E49A0`)

## Control-ID to Row-Index Mapping

Same mapping as `FUN_004E45A0` (task #29):

| Control ID (param_2) | Row index |
|---|---|
| 0x6A2 | 0 |
| 0x522 | 1 |
| 0x523 | 2 |
| 0x524 | 3 |
| 0x525 | 4 |
| 0x526 | 5 |
| 0x527 | 6 |
| 0x528 | 7 |

## Behavioral Analysis

### Phase 1 -- Release old color ownership

```c
hWnd = GetDlgItem(param_1, param_2);
BVar1 = ShowWindow(hWnd, 0);   // hide combo
// map param_2 -> row index iVar2
piVar5 = &DAT_008b4040;
iVar6 = 0;
do {
    if (*piVar5 == iVar2) {             // color entry currently owned by this row?
        if (iVar6 != -1) {
            (&DAT_008b4040)[iVar6 * 3] = 0xffffffff;   // release: slot_owner = -1
        }
        break;
    }
    piVar5 += 3;
    iVar6++;
} while ((int)piVar5 < 0x8b40ac);
```

Walks the color table (8 entries, stride 3 ints at `0x8B4040`) to find the
entry whose slot_owner field (`[+0]`) equals this row index. When found, resets
it to `0xFFFFFFFF` (-1 = unclaimed), unless `iVar6 == -1` (would mean an invalid
index reached before finding an owned entry).

Loop upper bound here is `0x8B40AC` (not `0x8B40A0` as in FUN_004E45A0); the
extra 12 bytes covers one additional safety entry. Functionally the same 8-entry
range.

(verified via `decompile_function 0x004E49A0`)

### Phase 2 -- Select target item and claim new color

```c
LVar3 = SendDlgItemMessageA(param_1, param_2, 0x146, 0, 0);  // CB_GETCOUNT
wParam = 0;
if (0 < LVar3) {
    do {
        LVar4 = SendDlgItemMessageA(param_1, param_2, 0x150, wParam, 0);  // CB_GETITEMDATA
        if (LVar4 == param_3) {
            SendDlgItemMessageA(param_1, param_2, 0x14e, wParam, 0);  // CB_SETCURSEL
            if (LVar4 != -2) {
                // map param_2 -> row index iVar2 (same as phase 1)
                (&DAT_008b4040)[LVar4 * 3] = iVar2;  // claim: slot_owner = row
            }
            break;
        }
        wParam++;
    } while ((int)wParam < LVar3);
}
```

Scans all combo items looking for the one whose item-data matches `param_3`.
When found:
- Selects it via `CB_SETCURSEL`
- If item-data is NOT -2 (random): writes the row index into `color_table[color_idx].slot_owner`
- If item-data IS -2 (random): skips the color table write; no color is owned

(verified via `decompile_function 0x004E49A0`)

### Phase 3 -- Re-show and refresh all color combos

```c
if (BVar1 != 0) ShowWindow(hWnd, 5);  // re-show if was visible
// loop all 8 rows: same dispatch as FUN_004E4820
iVar2 = 0;
do {
    if (((g_GameMode == 3) || (g_GameMode == 4)) &&
        ((&DAT_00a8da90)[iVar2] == DAT_00ac11b4 ||
         *(int *)((&DAT_00a8da90)[iVar2] + 0x6b) == -1))
    {
        FUN_004e4770();    // sentinel loader
    }
    else {
        FUN_004e45a0();    // normal color population
    }
    iVar2++;
} while (iVar2 < 8);
```

Identical dispatch loop to `FUN_004E4820` (task #31). Repopulates all 8 color
combos so that colors released in phase 1 and claimed in phase 2 are correctly
reflected across every row.

(verified via `decompile_function 0x004E49A0`)

## Color Table Structure

The color table at `DAT_008B4040` (8 entries, stride 3 ints = 12 bytes):
- Entry `[i*3 + 0]` = slot_owner (row index 0-7 that owns this color, or -1 = unclaimed)
- Entry `[i*3 + (-1)]` (int before the triplet base) = owner-draw/swatch data

This function reads and writes only `[i*3 + 0]` (slot_owner). The swatch data
at stride-1 is read by `FUN_004E45A0` but not accessed here.

## Globals Accessed

| Global | Address | Usage |
|---|---|---|
| `DAT_008B4040` | `0x008B4040` | Color table base; slot_owner read/written |
| `g_GameMode` | (Ghidra label) | Mode gate for phase 3 dispatch |
| `DAT_00A8DA90` | `0x00A8DA90` | Player-slot pointer array (phase 3) |
| `DAT_00AC11B4` | `0x00AC11B4` | Null/absent slot sentinel (phase 3) |

## Callees

Confirmed via `get_function_callees 0x004E49A0`:
- `FUN_004E45A0` (0x004E45A0) — normal color combo population (task #29)
- `FUN_004E4770` (0x004E4770) — sentinel color loader (task #30)
- `GetDlgItem`, `SendDlgItemMessageA`, `ShowWindow` -- Win32

## Callers (in scope)

- `FUN_006ADC20` (0x006ADC20) -- row enable state machine (task #3)
- `FUN_006AE6E0` (0x006AE6E0) -- dialog init (task #1)
- `FUN_004E48E0` (0x004E48E0) -- color-combo-helper-b (task #32)

Out-of-scope callers: `CDFileClass__Constructor` (mislabeled) at 0x005E3D10,
`FUN_005DBF50`, `FUN_005E8EA0`, `FUN_005E9E70`, `FUN_005EA3E0`,
`FUN_005EAEE0`, `FUN_005EC3A0`, `FUN_005EC9F0`, `FUN_005ED5A0`,
`FUN_0077ED00`.
(Confirmed via `get_function_callers 0x004E49A0`)

## Out-of-scope refs

- Color table struct full layout -- deferred to task #66 (decode-struct-colortableentry)
- `g_GameMode` values 3/4 -- covered by task #52/#65
- Player-slot struct offset `+0x6B` -- covered by task #65

## Unverified (YELLOW)

- Loop upper bound `0x8B40AC` (vs `0x8B40A0` in FUN_004E45A0): the extra 12
  bytes suggest a slightly wider guard; functionally this does not change the
  8-entry iteration but the exact reason for the discrepancy is not verified.
- Color table entry structure (stride-1 swatch data, stride-0 slot_owner) --
  confirmed consistent with task #29 findings but full struct decode deferred
  to task #66.
- `iVar6 != -1` guard in phase 1: the only way `iVar6` reaches -1 would be if
  the loop starts with an already-invalid index; this appears to be a defensive
  check. The exact triggering condition is not verified.
