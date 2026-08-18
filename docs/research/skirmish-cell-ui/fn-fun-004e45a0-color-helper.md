# FUN_004E45A0 — Color Combo Population Helper

## Summary

Populates a single color owner-draw combo box for a given row of dialog 0x102.
Hides the combo, sends custom owner-draw setup messages, adds the localized
`GUI:RandomAsSymbols` entry, then iterates the global color table (`DAT_008B4040..0x8B40A0`,
8 entries, stride 12 bytes) adding items for every color that either belongs to
the current row slot (item-data == row index) or is unclaimed (item-data == –1).
Selects the entry matching the row's slot and sends a post-populate message.
Re-shows the combo at end.

## Address

`0x004E45A0` (verified via `decompile_function 0x004E45A0`)

## Active in YR

**Yes.** Called by `FUN_004E4820` (task #31) and `FUN_004E4C20` (task #34, color
selection handler), both in-scope YR-active. Also called by `FUN_004E49A0` (task #33).
(Callers confirmed via `get_function_callers 0x004E45A0`)

## Signature / Parameters

```c
void __fastcall FUN_004e45a0(
    HWND param_1,   // dialog 0x102 HWND
    int  param_2    // color combo control ID (0x6A2, 0x522..0x528)
)
```

## Control-ID → Row-Index Mapping

| Control ID (param_2) | Row index (local_c) |
|---|---|
| 0x6A2 | 0 (human-player slot) |
| 0x522 | 1 |
| 0x523 | 2 |
| 0x524 | 3 |
| 0x525 | 4 |
| 0x526 | 5 |
| 0x527 | 6 |
| 0x528 | 7 |

(verified via `decompile_function 0x004E45A0`)

## Behavioral Analysis

### Setup messages

```c
hWnd = GetDlgItem(param_1, param_2);
BVar2 = ShowWindow(hWnd, 0);                   // hide; saves visibility
SendMessageA(hWnd, 0x14B, 0, 0);               // CB_RESETCONTENT
SendMessageA(hWnd, 0x4DD, 0, 1);               // custom message — owner-draw min height?
SendMessageA(hWnd, 0x4DE, 0, 9);               // custom message — set item count?
```

Messages 0x4DD and 0x4DE are custom WM messages above 0x400 (WM_USER) — these are
owner-draw combo setup messages specific to the game's custom combo class.
(verified via `decompile_function 0x004E45A0`)

### Random/empty item

```c
lParam = StringTable__LoadString(..., "GUI:RandomAsSymbols", ..., 0x20A);
WVar3 = SendMessageA(hWnd, 0x4C2, 0, lParam);   // custom add-item
SendMessageA(hWnd, 0x151, WVar3, -2);            // CB_SETITEMDATA → -2 (random)
SendMessageA(hWnd, 0x498, WVar3, -1);            // custom owner-draw data → -1
```

Assembly at `0x004E45EF` loads string-key pointer `0x00822B7C`, whose bytes are
`GUI:RandomAsSymbols`. The immediate `0x20A` is the `GDlgSupp.cpp` source-line
argument to the string loader, not a string-table ID. This corrects the earlier
decompiler-signature interpretation.
Item-data –2 = random sentinel (same as country sentinel –2 in `FUN_006ADC20`).

### Color table iteration

```c
piVar4 = &DAT_008B4040;   // color table base (runtime-populated)
local_4 = 0;              // selected item index
local_8 = 0;              // color entry index
do {
    iVar1 = *piVar4;      // color table entry[i].slot_owner (stride 3 ints, +0 offset)
    if (iVar1 == local_c || iVar1 == -1) {
        WVar3 = SendMessageA(hWnd, 0x4C2, 0, 0x822B78);   // add-item with name ptr
        SendMessageA(hWnd, 0x498, WVar3, piVar4[-1]);       // owner-draw data (stride-1 = color RGB?)
        SendMessageA(hWnd, 0x151, WVar3, local_8);          // CB_SETITEMDATA = color index
        if (iVar1 == local_c) local_4 = WVar3;             // remember selection
    }
    piVar4 += 3;   // stride = 3 ints = 12 bytes
    local_8++;
} while ((int)piVar4 < 0x8B40A0);
```

Loop range: `0x8B4040` to `0x8B40A0` exclusive, stride 12 bytes →
`(0x8B40A0 - 0x8B4040) / 12 = 0x60 / 12 = 8` iterations.
Combined with the random entry, the combo holds up to 9 color choices.

`piVar4[-1]` (int at `base + entry_offset - 4`) carries the owner-draw data for
the color swatch (likely the color's RGB or palette index).
The fixed pointer `0x822B78` in the add-item call is the color name string pointer
(a global string label for the color, populated by `FUN_004E43C0`, task #28).

Color table structure at `0x008B4040` (stride 3 ints):
- `[+0]`: slot_owner — row index that "owns" this color, or –1 if unclaimed
- `[-4]` (the int before the triplet base): owner-draw / swatch data
- Remaining field: unknown; not read in this function

(Table base address confirmed via `read_memory 0x008B4040`; table is zero at load
time, populated at runtime by `FUN_004E43C0` / `FUN_004E4C20`.)

### Finalization

```c
SendMessageA(hWnd, 0x14E, local_4, 0);   // CB_SETCURSEL — select matched entry
SendMessageA(hWnd, 0x4F1, 0, 0);          // custom post-populate message
if (BVar2 != 0) ShowWindow(hWnd, SW_SHOW=5);
```

## Globals Accessed

| Global | Address | Usage |
|---|---|---|
| `DAT_008B4040` | `0x008B4040` | Color table base — stride-3-int entries |
| `0x008B40A0` | (loop end bound) | Color table end (exclusive) |
| `0x00822B78` | Color name string ptr | Passed to add-item call |

(verified via `decompile_function 0x004E45A0` and `read_memory 0x008B4040`)

## Callees

Confirmed via `get_function_callees 0x004E45A0`:
- `StringTable__LoadString` — loads `GUI:RandomAsSymbols`; `0x20A` is source-line metadata
- `GetDlgItem`, `SendMessageA`, `ShowWindow` — Win32

## Out-of-scope refs

- `0x822B78` — color name string pointer; resolved by `FUN_004E43C0` (task #28)
- Color table struct layout — full decode in task #66 (decode-struct-colortableentry)
- Custom WM messages 0x4C2, 0x4DD, 0x4DE, 0x498, 0x4F1 — owner-draw combo protocol;
  not decoded here

## Unverified (YELLOW)

- The exact meaning of `piVar4[-1]` (the int before the triplet base at the first
  entry stride) is inferred as owner-draw/swatch data from its use in the
  `0x498` custom message call. The full color table struct layout is deferred to
  task #66.
- The purpose of custom WM messages 0x4DD and 0x4DE (sent during setup) is not
  confirmed — they appear to be game-specific owner-draw combo initialization.
