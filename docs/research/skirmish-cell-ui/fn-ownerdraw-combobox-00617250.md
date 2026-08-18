# OwnerDraw_ComboBox_00617250 — Custom Combo WndProc (Subclassed Owner-Draw)

## Summary

Subclassed window procedure for the owner-draw combo controls in dialog 0x102
(and other YR dialogs). Registered via `SetWindowLongA` / subclass mechanism —
no direct callers in Ghidra. On each message, looks up the combo's registration
entry in two hash tables (`DAT_00AC1B00` = owner-draw combo table,
`DAT_00AC1B48` = prior-WndProc table) and either handles the message directly
or delegates to `CallWindowProcA(pWVar28, ...)` for the original procedure.
Handles WM_PAINT (0x0F), WM_LBUTTONDOWN (0x201), WM_LBUTTONDBLCLK (0x203),
CB_GETCURSEL (0x147), CB_SETCURSEL (0x14E), CB_GETITEMDATA (0x150/0x151),
and a set of custom application messages (0x497–0x4F1). Performs dropdown
creation/destruction, custom text painting with `BitFont`, alpha-blend
background rendering, and color swatch placement.

## Address

`0x00617250` (verified via `decompile_function 0x00617250`)

## Active in YR

**Yes.** Registered as the window procedure for owner-draw combo controls
via the subclass hash table at `DAT_00AC1B00`. All in-scope combo controls
in dialog 0x102 are owner-draw; this function handles their painting and
input. No TS-only gate.

No Ghidra callers — registered as WndProc via the subclass infrastructure.
(confirmed via `get_function_callers 0x00617250`)

## Signature

```c
WPARAM OwnerDraw_ComboBox_00617250(void)
// Parameters arrive via stack (x86 WndProc convention):
// in_stack_00002b84 = HWND hWnd
// in_stack_00002b88 = UINT uMsg
// in_stack_00002b8c = WPARAM wParam
// in_stack_00002b90 = LPARAM lParam
```

(verified via `decompile_function 0x00617250`)

## Hash Table Lookup

On every call, the function performs two lookups:

1. **Prior WndProc table** (`DAT_00AC1B48`): maps HWND → original WndProc
   pointer for `CallWindowProcA` fallback.
2. **Owner-draw combo table** (`DAT_00AC1B00`): maps HWND → combo
   registration struct (`piStack00000018`). If not found, immediately
   delegates to `CallWindowProcA` and returns.

Both tables are hash maps keyed by HWND, using hash functions at
`DAT_00AC1B18` / `DAT_00AC1B60`.

(verified via `decompile_function 0x00617250`)

## Key Message Handlers

### WM_PAINT (0x0F)

Custom painting path: uses `BSurface` / `AlphaBlendRect` to draw the
collapsed combo background, then:
- Calls `FUN_006208F0` / `FUN_00620720` to blit the color swatch area
- Reads the current selection via `CallWindowProcA(..., 0x147, ...)` (CB_GETCURSEL)
- Reads item-data via `CallWindowProcA(..., 0x199, ...)` then falls back to 0x150
- Clips item text via `BitFont__GetTextWidth` + truncation loop
- Calls `FUN_00621040` to draw the truncated text
- Calls `ValidateRect` to suppress further WM_PAINT

### WM_LBUTTONDOWN (0x201) / WM_LBUTTONDBLCLK (0x203)

Arrow-click detection: checks if X coordinate > `in_stack_000000d8 - 0x14`
(the arrow button region on the right). If inside, sends `CB_GETDROPPEDSTATE`
(0x157), then `PostMessageA(..., 0x14F, ...)` to toggle dropdown.

### CB_SETCURSEL (0x14E)

Stores the new selection index in `piStack00000018[0x3E]`, then dispatches
to `SendMessageA(..., 0x4B4, ...)` with item text for text-area update.
Calls `InvalidateRect` to repaint. Delegates to the original WndProc at end.

### CB_GETCURSEL (0x147)

Returns `piStack00000018[0x3E]` directly from the registration struct
(cached current selection) without forwarding.

### CB_GETITEMDATA (0x150) / CB_SETITEMDATA (0x151)

Calls `CallWindowProcA(pWVar28, ..., 0x150, ...)` to get the item-data from
the underlying combo, then:
- For 0x150/0x199: returns `*(WPARAM*)(result + 4)` — item-data field at +4
- For 0x151/0x19A: writes `in_stack_00002b90` to `*(char**)(result + 4)`

### Message 0x497 (custom init)

Resets `piStack00000018[0x3E]` (current selection) to -1 and clears the
50-entry (`0x32`) swatch color array at `piStack00000018[0x44]` to -1.
Adjusts item height via `CB_SETITEMHEIGHT` (0x153) based on the font metrics.

### Message 0x498 (custom swatch store)

Stores `in_stack_00002b90` (color swatch LRESULT) at
`piStack00000018[in_stack_00002b8c + 0x44]` when `wParam < 0x33`.
This is how per-item color swatch data is written for owner-draw painting.

### Message 0x4F1

Sets `*(bool*)((int)piStack00000018 + 0xCD)` from `lParam == 1`.

### Dropdown creation (msg 0x14F, wParam=1)

Creates a "ComboDropWin" child window via `CreateWindowExA`, builds a new
owner-draw registration struct, registers it in `DAT_00AC1B00`, and shows
it via `ShowWindow` + `SetCapture`.

### Dropdown destruction (msg 0x14F, wParam=0)

Destroys the dropdown window, unregisters it from `DAT_00AC1B00`, and
sends `0x4A9` (dropdown-closed notification) to the parent.

## Registration Struct at `piStack00000018`

Key offsets accessed (all 4-byte ints, `piStack00000018[n]` = offset `n*4`):

| Index | Offset | Usage |
|-------|--------|-------|
| `[4]` | 0x10 | BSurface pointer (background surface, lazily created) |
| `[7]` | 0x1C | Item height initialized flag |
| `[0xD]` | 0x34 | Linked list of string allocation nodes |
| `[0x19]` | 0x64 | Font metrics struct pointer |
| `[0x1D]` | 0x74 | State flag (set to 2 by FUN_00603D30) |
| `[0x33]` | 0xCC | bool: show item icon/swatch flag |
| `[0x34]` | 0xD0 | CB_SETMINVISIBLE stored value |
| `[0x3D]` | 0xF4 | Dropdown child HWND (0 = closed) |
| `[0x3E]` | 0xF8 | Current selection index (-1 = none) |
| `[0x44..0x75]` | 0x110..0x1D4 | Per-item swatch color data (50 entries) |

(verified via `decompile_function 0x00617250`)

## Globals referenced

| Global | Address | Role |
|--------|---------|------|
| `DAT_00AC1B00` | 0x00AC1B00 | Owner-draw combo hash table base |
| `DAT_00AC1B04` | 0x00AC1B04 | Owner-draw combo hash table count |
| `DAT_00AC1B08` | 0x00AC1B08 | Hash table bucket count |
| `DAT_00AC1B0C` | 0x00AC1B0C | Hash table bucket shift |
| `DAT_00AC1B18` | 0x00AC1B18 | Hash function pointer |
| `DAT_00AC1B48` | 0x00AC1B48 | Prior-WndProc hash table base |
| `g_GAME_FNT` | (symbolic) | Game font resource |

## Callers

None — registered as WndProc via subclass mechanism.

(confirmed via `get_function_callers 0x00617250`)

## Callees (selected)

| Address | Name | Role |
|---------|------|------|
| Win32 | CallWindowProcA | Delegate to prior WndProc |
| Win32 | CreateWindowExA | Create dropdown child ("ComboDropWin") |
| Win32 | DestroyWindow | Destroy dropdown |
| 0x006208F0 | FUN_006208f0 | Blit color swatch area |
| 0x00621040 | FUN_00621040 | Draw text with BitFont |
| 0x00621B80 | AlphaBlendRect | Alpha-blend background |
| 0x00433ED0 | BitFont__GetTextWidth | Measure text for truncation |
| 0x00750920 | VocClass__PlayAtPos | Play click sound |

(confirmed via `get_function_callees 0x00617250`)

## Out-of-scope refs

- `FUN_006208F0`, `FUN_00620720`, `FUN_00621040` — swatch-blit and text-draw
  helpers; out of current cell-UI decode scope
- `BSurface`, `XSurface`, `AlphaBlendRect` — rendering infrastructure; not in scope

## TS-filter

Active in YR — all in-scope combo controls use this WndProc. No TS-only gate.
**TS-legacy score: 0.0.**

## Unverified (YELLOW)

- Registration struct field names/purposes: derived from access patterns in the
  decompile; the actual struct definition is not in a decoded header. Field
  semantics (e.g., `[0x19]` = font metrics, `[0x34]` = MINVISIBLE) are inferred
  from the context of use.
- "50-entry swatch array" at `[0x44]`: loop `for (iVar7 = 0x32; iVar7 != 0; ...)
  *piVar9 = -1;` initializes 0x32 = 50 dwords at `[0x44]`; the actual per-item
  count interpretation (max 50 items) is an inference.
- `WM_PAINT` path internal pipeline: several rendering helpers (`FUN_006208F0`,
  `FUN_00620720`) were not independently decompiled; their roles are inferred
  from names, context, and adjacent `AlphaBlendRect` calls.
