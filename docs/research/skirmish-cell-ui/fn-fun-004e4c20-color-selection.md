# FUN_004E4C20 — Color Combo CBN_SELCHANGE Handler

## Summary

`FUN_004E4C20` is the color combo CBN_SELCHANGE handler — called when the player
changes the color selection for a slot. It maps the combo control ID to a slot
index, clears that slot's prior color ownership from `DAT_008B4040` (the color
assignment table), reads the newly selected item's item-data via
`CB_GETCURSEL`/`CB_GETITEMDATA`, writes the new ownership into the color table
(unless item-data is the sentinel `-2`), and then refreshes all 8 color combos
to reflect the updated availability. Called from the WM_COMMAND dispatcher
`FUN_006ACEE0`.

## Address

`0x004E4C20` (verified via `decompile_function 0x004E4C20`)

## Active in YR

**Yes.** Called from `FUN_006ACEE0` (0x006ACEE0), the YR WM_COMMAND dispatcher
for dialog 0x102.

(confirmed via `get_function_callers 0x004E4C20`)

## Signature / Parameters

```c
void __fastcall FUN_004e4c20(HWND param_1, int param_2)
// param_1 = dialog 0x102 HWND
// param_2 = color combo control ID that fired CBN_SELCHANGE
//           (0x6A2=slot0, 0x522..0x528=slots 1-7)
```

(verified via `decompile_function 0x004E4C20`)

## Behavioral Analysis

### Color combo control ID → slot index

Same mapping as `FUN_004E49A0` (task #33):

| Control ID | Slot index |
|------------|-----------|
| 0x6A2      | 0         |
| 0x522      | 1         |
| 0x523      | 2         |
| 0x524      | 3         |
| 0x525      | 4         |
| 0x526      | 5         |
| 0x527      | 6         |
| 0x528      | 7         |

(verified via `decompile_function 0x004E4C20`)

### Phase 1 — Clear prior color ownership

Walk `DAT_008B4040` (stride 3 dwords) to find the entry whose `[0]` field equals
the current slot index `iVar1`. When found and `iVar4 != -1`, clear it:

```c
(&DAT_008B4040)[iVar4 * 3] = 0xFFFFFFFF;  // mark color unowned
```

This releases the color this slot previously claimed so other slots can take it.

### Phase 2 — Read new selection

```c
wParam = SendDlgItemMessageA(param_1, param_2, 0x147, 0, 0);  // CB_GETCURSEL
LVar2  = SendDlgItemMessageA(param_1, param_2, 0x150, wParam, 0); // CB_GETITEMDATA
```

`LVar2` is the color index (0..N) for a real color, or `-2` for the closed/sentinel entry.

### Phase 3 — Write new ownership

```c
if (LVar2 != -2) {
    (&DAT_008B4040)[LVar2 * 3] = iVar1;  // [colorIndex * 3] = slotIndex
}
```

When item-data is `-2` (sentinel), no ownership is written — the slot claims no color.

### Phase 4 — Refresh all 8 color combos

Same refresh loop as `FUN_004E4820` and `FUN_004E49A0`:

```c
iVar1 = 0;
do {
    if (spectator_mode_and_local_or_closed(iVar1))
        FUN_004e4770();   // reload sentinel
    else
        FUN_004e45a0();   // reload full color list
    iVar1++;
} while (iVar1 < 8);
```

This causes all other slots' color combos to update their available-colors list,
removing the newly claimed color from their drop-downs.

## Difference from FUN_004E49A0

`FUN_004E49A0` (task #33) is the *restore* path: it takes an explicit target
color index, iterates CB_GETITEMDATA until it finds a match, then calls
CB_SETCURSEL. `FUN_004E4C20` is the *live* path: it reads the current selection
(which the user just changed via mouse/keyboard) and updates ownership without
needing to search.

## Globals referenced

| Global | Address | Access | Role |
|--------|---------|--------|------|
| `DAT_008B4040` | 0x008B4040 | READ/WRITE | Color assignment table; `[colorIndex * 3]` = slot index |
| `DAT_00A8DA90` | 0x00A8DA90 | READ | Per-slot session entry pointers |
| `DAT_00AC11B4` | 0x00AC11B4 | READ | Local player session entry pointer |
| `g_GameMode` | (symbolic) | READ | Game mode; 3=spectator, 4=observer |

## Struct field accesses

| Pointer | Offset | Unit | Usage |
|---------|--------|------|-------|
| `(&DAT_00A8DA90)[slot]` | `+0x6B` | int | Slot status; -1 = closed |
| `DAT_008B4040[n * 3]` | `+0` | int | Slot owning color n; 0xFFFFFFFF = unowned |

## Callers

| Address | Name | Role |
|---------|------|------|
| 0x005E9C00 | FUN_005e9c00 | out-of-scope |
| 0x005ED5A0 | FUN_005ed5a0 | out-of-scope |
| 0x006ACEE0 | FUN_006acee0 | WM_COMMAND dispatcher (task #2) |

(confirmed via `get_function_callers 0x004E4C20`)

## Callees

| Address | Name | Role |
|---------|------|------|
| 0x004E4770 | FUN_004e4770 | Color sentinel loader (task #30) |
| 0x004E45A0 | FUN_004e45a0 | Normal color combo population (task #29) |
| Win32 | SendDlgItemMessageA | CB_GETCURSEL (0x147), CB_GETITEMDATA (0x150) |

(confirmed via `get_function_callees 0x004E4C20`)

## Out-of-scope refs

- `FUN_004E45A0` — full color population; task #29 in progress by another decoder
- `FUN_005E9C00`, `FUN_005ED5A0` — out-of-scope callers

## TS-filter

Primary in-scope caller is `FUN_006ACEE0` (YR WM_COMMAND). No TS-only gate.
**TS-legacy score: 0.0.**

## Unverified (YELLOW)

- `DAT_008B4040` stride-3 layout and semantics: same inference as task #33 —
  consistent with loop stride and write `[colorIndex * 3] = slotIndex`, not
  independently verified against a struct layout decode.
- `g_GameMode == 3/4` enum values: same inference as tasks #31–33.
- Table upper bound `0x8B40AC` in the clear loop: `0x8B40AC - 0x8B4040 = 0x6C`
  bytes = 27 dwords = 9 entries of 3 dwords; implies 9 color entries in the table.
  Not independently verified against a color table size constant.
