# FUN_004E4770 — Color Combo Sentinel Loader (Spectator/Closed Slot)

## Summary

`FUN_004E4770` resets a single color combo control to a one-item "Random Color"
sentinel state. It takes a dialog HWND and a color combo control ID. It hides
the combo, clears all items (CB_RESETCONTENT), sets item height and min-visible
count, inserts a single string from string table ID `0x237` ("Random Color" /
closed-slot label), assigns item-data `-2` (the sentinel for a closed or
spectator row), selects that item at index 0, and re-shows the combo if it was
visible before. This is the per-combo counterpart to `FUN_004E49A0` (which
additionally walks a color assignment table). Called when a row should display
the "no real color assigned" state — network/spectator mode or when a row's
color choice must be reset.

## Address

`0x004E4770` (verified via `decompile_function 0x004E4770`)

## Active in YR

**Yes.** Called from `FUN_004E4C20` (0x004E4C20), which is itself called from
`FUN_006ACEE0` (the YR WM_COMMAND dispatcher) — confirmed via
`get_function_callers 0x004E4C20`. Also called from `FUN_004E4820` (task #31)
and `FUN_004E49A0` (task #33). No TS-only gate present.

(confirmed via `get_function_callers 0x004E4770`)

## Signature / Parameters

```c
void __fastcall FUN_004e4770(HWND param_1, int param_2)
// param_1 = dialog 0x102 HWND
// param_2 = color combo control ID (0x6A2..0x528 per FUN_004E41D0)
```

(verified via `decompile_function 0x004E4770`)

## Behavioral Analysis

### Message sequence

```c
hWnd = GetDlgItem(param_1, param_2);
BVar1 = ShowWindow(hWnd, SW_HIDE=0);       // hide; save prior visibility
SendMessageA(hWnd, 0x14B, 0, 0);           // CB_RESETCONTENT — clear all items
SendMessageA(hWnd, 0x4DD, 0, 1);           // CB_SETITEMHEIGHT — item height = 1
SendMessageA(hWnd, 0x4DE, 0, 9);           // CB_SETMINVISIBLE — min visible = 9
lParam = StringTable__LoadString(          // load string ID 0x237 → "Random Color" label
    s_GDlgSupp, 0x237);
wParam = SendMessageA(hWnd, 0x4C2, 0, lParam); // CB_INSERTSTRING at pos 0
SendMessageA(hWnd, 0x151, wParam, -2);     // CB_SETITEMDATA; item-data = -2 (sentinel)
SendMessageA(hWnd, 0x498, wParam, -1);     // custom ownerdraw swatch msg; color index = -1
SendMessageA(hWnd, 0x14E, 0, 0);           // CB_SETCURSEL; select item 0
SendMessageA(hWnd, 0x4F1, 0, 1);           // CB_SETCUEBANNER or extended combo msg
if (BVar1 != 0) ShowWindow(hWnd, SW_SHOW=5); // re-show if was visible
```

(verified via `decompile_function 0x004E4770`)

### Key sentinel value: item-data `-2`

Item-data `-2` (0xFFFFFFFE) is the sentinel written to color combos for
closed or spectator rows. The color selection restore path in
`FUN_006AE6E0` (init) writes this sentinel when a slot's type-code is
`-1` (Closed). `FUN_004E49A0` (color-sentinel task #33) also reads
item-data to test for `-2` to skip color-table writes.

### Message 0x498 — owner-draw color swatch

`SendMessageA(hWnd, 0x498, wParam, -1)` passes color index `-1` to the
color combo. Message `0x498` is a custom application-defined message in
the owner-draw combo infrastructure — it carries the color swatch index
for painting the colored square. Index `-1` signals "no color / blank
swatch" for the sentinel row entry.

### String 0x237

`StringTable__LoadString(s_GDlgSupp, 0x237)` — string ID `0x237` (decimal 567).
This is the "Random Color" or closed-slot display string shown in the color combo
when a row is in spectator/closed state. The exact localized text is in
`GDlgSupp.csf`.

### Callers and usage context

`FUN_004E4820` (task #31) loops all 8 rows and calls this function when
`g_GameMode` is 3 or 4 (spectator/observer game modes) or when the session
entry's `+0x6B` field is `-1` (slot closed). `FUN_004E49A0` (task #33)
calls it after resetting the color assignment table when restoring a specific
color selection. `FUN_004E4C20` (task #34) is the color selection handler
that calls it on mode change.

## Struct field accesses

None directly — all Win32 control messages with immediate integer arguments.

## Globals referenced

None directly (string table pointer `s_GDlgSupp` is a code segment reference
used by `StringTable__LoadString`).

## Callers

| Address | Name | Role |
|---------|------|------|
| 0x005E3D10 | CDFileClass__Constructor (mislabeled) | out-of-scope |
| 0x004E4820 | FUN_004e4820 | Color combo loop (task #31) |
| 0x004E49A0 | FUN_004e49a0 | Color sentinel restore (task #33) |
| 0x004E4C20 | FUN_004e4c20 | Color selection handler (task #34) |
| 0x005DBF50 | FUN_005dbf50 | out-of-scope |
| 0x005E8EA0 | FUN_005e8ea0 | out-of-scope |
| 0x005EC3A0 | FUN_005ec3a0 | out-of-scope |
| 0x005EC9F0 | FUN_005ec9f0 | out-of-scope |
| 0x0077ED00 | FUN_0077ed00 | out-of-scope |

(confirmed via `get_function_callers 0x004E4770`)

## Callees

| Address | Name | Role |
|---------|------|------|
| Win32 | GetDlgItem | Get combo HWND |
| Win32 | ShowWindow | Hide/show combo |
| Win32 | SendMessageA | CB_RESETCONTENT, CB_SETITEMHEIGHT, CB_SETMINVISIBLE, CB_INSERTSTRING, CB_SETITEMDATA, CB_SETCURSEL, 0x498, 0x4F1 |
| 0x00734E60 | StringTable__LoadString | Load string ID 0x237 |

(confirmed via `get_function_callees 0x004E4770`)

## Out-of-scope refs

- `FUN_005E3D10` (mislabeled `CDFileClass__Constructor`) — out of cell-UI scope
- `FUN_005DBF50`, `FUN_005E8EA0`, `FUN_005EC3A0`, `FUN_005EC9F0`, `FUN_0077ED00` — out-of-scope callers

## TS-filter

All in-scope callers are reachable from the YR offline Skirmish dialog path.
No TS-only gate. **TS-legacy score: 0.0.**

## Unverified (YELLOW)

- String ID `0x237` text: inferred as "Random Color" / closed-slot label from
  context; the actual localized string in `GDlgSupp.csf` was not read in this
  session.
- Message `0x498` semantics: inferred as "set owner-draw color swatch index" from
  usage pattern (integer argument matches color index convention; value -1 = no
  color); not independently verified against the owner-draw combo handler at
  `0x00617250`.
- Message `0x4F1` semantics: same as in other combo loaders — inferred as
  `CB_SETCUEBANNER` or an extended combo message; not verified against Windows
  SDK in this session.
- `CDFileClass__Constructor` at `0x005E3D10` label: Ghidra label is mislabeled
  (noted in manifest); actual role of this caller is unknown and out of scope.
