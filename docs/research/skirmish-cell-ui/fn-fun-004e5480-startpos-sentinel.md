# FUN_004E5480 — Start-Position Combo Selection Restore + Assignment Table Update

## Summary

Start-position equivalent of `FUN_004E49A0` (color sentinel, task #33). Takes
a dialog HWND, a start-position combo control ID, and a target start-position
item-data value. Clears the slot's prior start-position claim from
`DAT_008B3F38` (the start-position assignment table), scans combo items via
`CB_GETCOUNT`/`CB_GETITEMDATA` to find the target, calls `CB_SETCURSEL` when
found, and (unless item-data is `-2`) writes the slot index back into the
assignment table. Finishes by refreshing all 8 start-position combos. Called
from the row-enable state machine and dialog init.

## Address

`0x004E5480` (verified via `decompile_function 0x004E5480`)

## Active in YR

**Yes.** In-scope callers include `FUN_006ADC20` (0x006ADC20, row-enable state
machine, task #3) and `FUN_006AE6E0` (0x006AE6E0, dialog init, task #1). Also
called from `FUN_004E53D0` (task #40, in progress).

(confirmed via `get_function_callers 0x004E5480`)

## Signature / Parameters

```c
void __fastcall FUN_004e5480(HWND param_1, int param_2, int param_3)
// param_1 = dialog 0x102 HWND
// param_2 = start-position combo control ID (0x6A3..0x6A8, 0x6AA, 0x6AB)
// param_3 = target start-position item-data to find and select (-2 = sentinel)
```

(verified via `decompile_function 0x004E5480`)

## Start-Position Combo Control ID → Slot Index Map

Same IDs as returned by `FUN_004E4E60` (task #36):

| Control ID | Slot index |
|------------|-----------|
| 0x6A3      | 0         |
| 0x6A4      | 1         |
| 0x6A5      | 2         |
| 0x6A6      | 3         |
| 0x6A7      | 4         |
| 0x6A8      | 5         |
| 0x6AA      | 6         |
| 0x6AB      | 7         |

Note: 0x6A9 is absent (gap in the ID sequence, matching `FUN_004E4E60`).

(verified via `decompile_function 0x004E5480`)

## Behavioral Analysis

### Phase 1 — Clear prior start-position ownership

```c
hWnd = GetDlgItem(param_1, param_2);
BVar1 = ShowWindow(hWnd, SW_HIDE=0);
// map param_2 → row index iVar2
piVar5 = &DAT_008B3F38;
do {
    if (*piVar5 == iVar2) {
        if (iVar6 != -1) (&DAT_008B3F38)[iVar6 * 3] = 0xFFFFFFFF;
        break;
    }
    piVar5 += 3;
    iVar6++;
} while ((int)piVar5 < 0x8B3FA4);
```

Walks the start-position assignment table at `DAT_008B3F38` (stride 3 dwords,
9 entries, upper bound `0x8B3FA4`) and clears the entry owned by this slot.

Note: `DAT_008B3F38` is the availability/owner field within the same table
initialized by `FUN_004E4F50` (task #37). Field `[0]` of each entry (at
`DAT_008B3F30 + entry*12`) holds the swatch pointer; field `[+8]` (at
`DAT_008B3F38 + entry*12`) holds the owner slot index.

### Phase 2 — Find and select target item

```c
LVar3 = SendDlgItemMessageA(param_1, param_2, 0x146, 0, 0); // CB_GETCOUNT
wParam = 0;
if (0 < LVar3) {
    do {
        LVar4 = SendDlgItemMessageA(param_1, param_2, 0x150, wParam, 0); // CB_GETITEMDATA
        if (LVar4 == param_3) {
            SendDlgItemMessageA(param_1, param_2, 0x14E, wParam, 0); // CB_SETCURSEL
            if (LVar4 != -2) {
                (&DAT_008B3F38)[LVar4 * 3] = iVar2; // claim: [startposIndex * 3] = slotIndex
            }
            break;
        }
        wParam++;
    } while ((int)wParam < LVar3);
}
```

Sentinel `-2` suppresses the assignment write — no slot claims a start position.

### Phase 3 — Re-show and refresh all start-position combos

```c
if (BVar1 != 0) ShowWindow(hWnd, SW_SHOW=5);
iVar2 = 0;
do {
    if (spectator_mode_and_local_or_closed(iVar2))
        FUN_004e5260();    // start-pos sentinel loader
    else
        FUN_004e50c0();    // normal start-pos population
    iVar2++;
} while (iVar2 < 8);
```

Same dispatch pattern as `FUN_004E49A0` phase 3, using start-position helpers
`FUN_004E5260` and `FUN_004E50C0` instead of color helpers.

## Structural Parallel to FUN_004E49A0

`FUN_004E5480` is the start-position counterpart to `FUN_004E49A0` (color,
task #33). Both functions share the same three-phase structure, the same
`-2` sentinel semantics, and the same post-update refresh pattern. The only
differences are the combo control ID set, the assignment table base address
(`DAT_008B3F38` vs `DAT_008B4040`), and the refresh callees.

## Globals referenced

| Global | Address | Access | Role |
|--------|---------|--------|------|
| `DAT_008B3F38` | 0x008B3F38 | READ/WRITE | Start-pos assignment table owner field |
| `DAT_00A8DA90` | 0x00A8DA90 | READ | Per-slot session entry pointers |
| `DAT_00AC11B4` | 0x00AC11B4 | READ | Local player session entry pointer |
| `g_GameMode` | (symbolic) | READ | Game mode; 3=spectator, 4=observer |

## Struct field accesses

| Pointer | Offset | Unit | Usage |
|---------|--------|------|-------|
| `(&DAT_00A8DA90)[slot]` | `+0x6B` | int | Slot status; -1 = closed |
| `DAT_008B3F38[n * 3]` | `+0` | int | Slot owning start-pos n; 0xFFFF = unowned |

## Callers

| Address | Name | Role |
|---------|------|------|
| 0x0046FAC0 | FUN_0046fac0 | out-of-scope |
| 0x004E53D0 | FUN_004e53d0 | Start-pos helper (task #40) |
| 0x005E9E70 | FUN_005e9e70 | out-of-scope |
| 0x005EA3E0 | FUN_005ea3e0 | out-of-scope |
| 0x005EADA0 | FUN_005eada0 | out-of-scope |
| 0x005ED5A0 | FUN_005ed5a0 | out-of-scope |
| 0x006ADC20 | FUN_006adc20 | Row enable state machine (task #3) |
| 0x006AE6E0 | FUN_006ae6e0 | Dialog init (task #1) |

(confirmed via `get_function_callers 0x004E5480`)

## Callees

| Address | Name | Role |
|---------|------|------|
| 0x004E50C0 | FUN_004e50c0 | Normal start-pos combo population (task #39/40) |
| 0x004E5260 | FUN_004e5260 | Start-pos sentinel loader |
| Win32 | GetDlgItem | Get combo HWND |
| Win32 | ShowWindow | Hide/show combo |
| Win32 | SendDlgItemMessageA | CB_GETCOUNT, CB_GETITEMDATA, CB_SETCURSEL |

(confirmed via `get_function_callees 0x004E5480`)

## Out-of-scope refs

- `FUN_004E50C0` — normal start-pos population; task #39 in progress
- `FUN_004E5260` — start-pos sentinel loader; out of current task scope

## TS-filter

In-scope callers are the YR dialog functions. No TS-only gate.
**TS-legacy score: 0.0.**

## Unverified (YELLOW)

- `DAT_008B3F38` as the start-position assignment table owner field: `FUN_004E4F50`
  (task #37) clears this field to `0xFFFFFFFF` at init, and this function reads/writes
  it with slot indices — consistent with the same table, confirmed by matching address.
  The full table structure (3-dword stride, 9 entries) is the same as decoded in task #37.
- `FUN_004E5260` as start-pos sentinel loader: inferred from the spectator/closed
  dispatch pattern mirroring `FUN_004E4770` (color sentinel) in `FUN_004E49A0`; not
  independently decompiled in this task.
- `FUN_004E50C0` as normal start-pos population: inferred from the else-branch
  dispatch mirroring `FUN_004E45A0` (color population); not independently decompiled.
