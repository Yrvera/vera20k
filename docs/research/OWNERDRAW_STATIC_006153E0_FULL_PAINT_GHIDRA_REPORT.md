# OwnerDraw_Static_006153E0 — Full Paint Procedure Ghidra Report

Date: 2026-05-19

Scope: End-to-end decompilation of `OwnerDraw_Static_006153E0 @ 0x006153E0`, the
subclassed window procedure for all Static controls on main-menu dialog `0xE2`.
Documents the complete WM_DRAWITEM-equivalent switch (routed through `WM_PAINT`),
all custom shell messages handled, and each of the four named Static controls'
distinct paint branches. No Rust code modified. No Ghidra annotations written.

Parent/sibling reports:

- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `MAIN_MENU_TITLE_TEXT_RENDER_GHIDRA_REPORT.md`
- `STATIC_0X71C_RUNTIME_VISIBILITY_GHIDRA_REPORT.md`
- `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`
- `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`

Active in YR: Yes (full function is the live static owner-draw proc for `0xE2`).

---

## 1. Function Overview

Address: `0x006153E0`
Body range: `0x006153E0 – 0x00616300`
Signature: `LRESULT OwnerDraw_Static_006153E0(HWND hwnd, UINT msg, WPARAM wParam, int lParam)`

This is a subclassed `WndProc` registered by `FUN_0060F9A0` for every child
control of class `"Static"` on dialog `0xE2` (and many other shell dialogs). It
intercepts a specific set of messages; all others fall through to
`CallWindowProcA(lpPrevWndFunc, ...)`.

Active in YR: Yes (subclassing happens unconditionally for `"Static"` class
children during `WM_INITDIALOG` via `FUN_00622B50 → FUN_0060F9A0`).

---

## 2. Per-Control Record (`piVar11`)

The proc begins by looking up the HWND in a hash table (bucket array at
`DAT_00ac1b00`, table size `1 << (byte)DAT_00ac1b0c`, hash via `(*DAT_00ac1b18)()`).
On a match it sets `piVar11 = piVar4 + 1` (the per-control state record, at the
HWND table entry + 1 dword). If the HWND is not found, the proc returns `0` immediately.

Key record fields used across all branches (dword-indexed, i.e. byte offset = index × 4):

| Field index | Byte offset | Meaning |
|---|---|---|
| `[4]` | `0x10` | BSurface* backing offscreen surface (allocated on first WM_PAINT) |
| `[5]` | `0x14` | Image/SHP pointer for kind-2 image type |
| `[0x16]` | `0x58` | VQMovie handle pointer (kind-4 movie) |
| `[0x17]` | `0x5C` | Movie loop flag (set by `0x4E3`) |
| `[0x18]` | `0x60` | Secondary movie / audio handle |
| `[0x19]` | `0x64` | wchar_t* text pointer (CSF-translated title) |
| `[0x1c]` | `0x70` | Kind/type: `0`=empty, `1`=text-anim, `2`=image, `3`=SHP-anim, `4`=movie |
| `[0x1d]` | `0x74` | SHP/image filename offset |
| `[0x1e]` | `0x78` | SHP/animation data pointer |
| `[0x1f]` | `0x7C` | SHP data ownership flag (1 = owned, must free) |
| `[0x20]` | `0x80` | Text-anim current char offset (scroll position) |
| `[0x21]` | `0x84` | Text-anim timer interval |
| `[0x22]` | `0x88` | Text-anim char scroll step |
| `[0x23]` | `0x8C` | Text-anim start offset |
| `[0x24]` | `0x90` | Sound id (played on anim start; `-1` = no sound) |
| `[0x25]` | `0x94` | SHP frame count |
| `[0x26]` | `0x98` | SHP current frame index |
| `[0x27]` | `0x9C` | Last tick (from `GetTickCount()`) |
| `[0x28]` | `0xA0` | Frame interval (ms between frames) |
| `[0x29]` | `0xA4` | Notify HWND for `0x4D8` messages (SHP loopback) |
| `[0x2a]` | `0xA8` | (char) Animation running flag |
| `[0x2b]` | `0xAC` | Text draw shadow/flags arg (initialized to `0x0C`) |
| `[0x2d]` | `0xB4` | (char) Background-fill active flag |
| `[0x2e]` | `0xB8` | Background-fill color (RGB packed) |
| `[0x2f]` | `0xBC` | (char) Suppress-paint flag |
| `[0x3b]` | `0xEC` | Text color (initialized to `DAT_00ac18a4` = `0xFFFF` → yellow) |

Active in YR: Yes.

---

## 3. Message Dispatch Overview

The proc handles messages in the following priority order:

```
if msg == 0x0F (WM_PAINT):       → paint branch (see §4)
if msg == 0x02 (WM_DESTROY):     → cleanup branch
if msg == 0x03 / 0x05:           → cleanup branch (also destroys surface)
if msg == 0x47:                   → destroy image surface, invalidate
if msg == 0x113 (WM_TIMER):      → movie tick or text-anim tick (see §5)
if msg == 0x497:                  → init/reset (see §6)
if 0x498..0x4D4 range:            → color-set, text-set, etc (see §7)
if 0x4D5..0x4F0 range:            → movie/SHP controls (see §8)
default:                          → CallWindowProcA(prevProc, ...)
```

Active in YR: Yes for all active message branches.

---

## 4. WM_PAINT (0x0F) — Full Paint Branch

This is the core owner-draw paint handler.

### 4.1 Guard and early exits

If `piVar11[0x2f]` (suppress-paint char) is non-zero → `ValidateRect` and return. This
allows external code to temporarily freeze repainting.

If `piVar11[0x16]` (movie handle) != 0 → `ValidateRect` and return (movie path uses
explicit `0x4F0` to paint; `WM_PAINT` is suppressed while a movie is live).

### 4.2 Surface allocation

If `piVar11[4]` (backing BSurface) is null, allocate:

```
1. FUN_00775690()             ; get window rect in client coords
2. GetClientRect(hwnd, &rc)
3. operator_new(0x20)         ; allocate XSurface struct
4. piVar4[1] = rc.right + 1  ; width
5. piVar4[2] = rc.bottom + 1 ; height
6. piVar4[3] = 0
7. *piVar4 = vtable__XSurface
8. piVar4[4] = 2             ; pixel depth flag
9. PixelBuffer_Init(piVar4+5, 0, h * w * 2)
10. *piVar4 = vtable__BSurface  ; promote to BSurface
piVar11[4] = piVar4
DAT_00ac48b4++               ; global surface counter
```

Then blit parent background into it:
```
(*vtable__BSurface[2])(surface, parent_surface, dest_rect, src_rect, 0, 1)
```

Active in YR: Yes.

### 4.3 Background-fill pre-pass

If `piVar11[0x2d]` (background-fill flag) is set, fill with the packed RGB from
`piVar11[0x2e]`, converting through the display-mode bit-shift globals
(`g_DD_BLoss/BShift`, `g_DD_GLoss/GShift`, `g_DD_RLoss/RShift`).

Active in YR: Conditional — only when a caller sent `0x4B1` to set the fill.
Not sent to any of the four main-menu statics on `0xE2`. So this path is
dormant for the main-menu but is part of the shared control infrastructure.

### 4.4 Kind dispatch

`iVar6 = piVar11[0x1c]` selects the paint path:

| Kind | Value | Paint mode |
|---|---|---|
| Empty | 0 | Text draw (§4.5) |
| Text-anim | 1 | Text draw with scroll ticker (§4.5) |
| Image (SHP) | 2 | Centered image blit (§4.6) |
| SHP-anim | 3 | Centered SHP frame blit (§4.7) |
| Movie | 4 | SHP frame blit with frame advance (§4.8) |

### 4.5 Kind 0/1 — Text draw path

Executed when `iVar6 == 0` or `iVar6 == 1`. Condition: `piVar11[10]` (the text
pointer alias, cross-check: this is index `10 = 0x0A`, byte offset `0x28`, but
a code path notes `piVar11[0x19]` at `0x64` as the stored wchar_t*; a possible
comment: `piVar11[10]` = `piVar11[0x0A]`, byte offset `0x28` — distinct from
`piVar11[0x19]` at offset `0x64`; the WM_PAINT text path reads `piVar11[10]` for
the length/content check and separately passes `piVar11[0x19]` to `FUN_00621040`).

Guard:
- `piVar11[10] != 0` (text buffer pointer non-null)
- `iVar6 == 0` always draws if text present
- `iVar6 == 1` (text-anim): draws only when `(char)piVar11[0x2a] != 0` (anim running)

Style bits determine alignment (`uVar10`):
```
uVar3 = GetWindowLongA(hwnd, GWL_STYLE)
uVar10 = 0x10 (default)
if (uVar3 & 1):     uVar10 = 0x11
else if (uVar3 & 2): uVar10 = 0x12
if (uVar3 & 0x8000000): color = DAT_00ac1cb4  ; disabled → dark-red #9F0000
else: color = piVar11[0x3b]                   ; normal → DAT_00ac18a4 = yellow
```

Text draw call:
```c
FUN_00621040(
  &clip_rect,        // ECX (fastcall): XSurface* (rect struct pointer used as source for FUN_00621040)
  piVar11[0x19],     // EDX: wchar_t* text
  piVar11[0x19],     // arg_1: RECT* (the backing surface coords)
  color,             // arg_2 (stack): 24-bit RGB color
  uVar10,            // arg_3: alignment flags (0x10/0x11/0x12 — see CORRECTION at §4.5.1)
  piVar11[0x2b],     // arg_4: 0x0C (dead arg, historical vestige)
  0,                 // arg_5: dead
  piVar11[0x20],     // arg_6: fade_count (typewriter scroll offset)
  piVar11[0x23]      // arg_7: fade_range (scroll window)
)
```

If `piVar11[0x24] != -1`: play sound via `VocClass__PlayAtPos(0x3f800000, 0)`.

For kind-1 text-anim, after draw:
```
iVar6 = FUN_007ca405(piVar11[10]) + 1 + piVar11[0x23]
if piVar11[0x20] < iVar6:
    piVar11[0x20] += piVar11[0x22]  ; advance scroll by step
    if iVar6 <= piVar11[0x20]:
        KillTimer(hwnd, 0)           ; anim complete
```

Active in YR: Yes — this is the live path for static `0x694` (title heading)
and `0x695` (tooltip), both kind 0, text-draw-only.

### 4.6 Kind 2 — Image (SHP) centered blit

Executed when `iVar6 == 2`. Requires `GetTickCount()` delta > frame interval
(`piVar11[0x28]`).

```
blit source rect = GetWindowRect(hwnd) in client coords
blit dest rect = GetClientRect(hwnd)
(*vtable__BSurface[2])(src_surface, backing_surface, dest_rect, src_rect, 0, 1)
piVar11 = piVar11[5]   ; image SHP object pointer
if piVar11 != 0:
    iVar6 = (*vtable[0x7c])()  ; get image width
    iVar7 = (*vtable[0x80])()  ; get image height
    compute centered rect (center within blit rect)
    FUN_006ba580(centered_rect, surface, piVar11, WHITE_color)
```

Color used: full white `(0xFF >> BLoss << BShift) | (0x00 >> GLoss << GShift) | (0xFF >> RLoss << RShift)`.

Note: `_g_DD_BLoss / _g_DD_GLoss / _g_DD_RLoss` (leading underscore variants) appear here instead of `g_DD_*`. These overlap with the same addresses, verified from the decompiler output.

Active in YR: Yes for other shell dialogs. Not used by any of the four main-menu
`0xE2` statics (none receive `0x4DA/0x4DB` messages that set kind to 2).

### 4.7 Kind 3 — SHP-anim centered blit

Executed when `iVar6 == 3` (or `bVar12 = iVar6==4` combined with kind-4 SHP branch).
Requires tick delta > `piVar11[0x28]`.

```
blit background from parent surface into backing surface
GetClientRect → blit rect
if piVar11[0x1e] != 0 (SHP data pointer):
    center frame within blit rect
    piVar4 = (*vtable__BSurface[0x78])(..array..)  ; get display surface info
    CC_Draw_Shape(shpData, frameIdx, destRect, srcSurface, 0x400, 0, 0, 0, 1000, 0,...)
```

`CC_Draw_Shape` flag `0x400` here is the same non-centering flag as used for
right-panel SHP drawing (parent doc's note).

Active in YR: Yes for other dialogs (e.g. score dialogs with SHP animations).
Not active for main-menu `0xE2` statics.

### 4.8 Kind 4 — Movie SHP frame path

Executed when `iVar6 == 4` and `piVar11[0x2a]` (anim running) is non-zero.
This is the SHP-frame sub-path within kind-4, not the Bink-direct path (Bink
is handled via timer-driven `0x4F0`).

```
center SHP frame within blit rect
CC_Draw_Shape(piVar11[0x1e], piVar11[0x26], destRect, surface, 0x400, ...)
if anim running:
    wParam = piVar11[0x26] + 1
    if wParam >= piVar11[0x25]: wParam = 0   ; wrap frame
    piVar11[0x26] = wParam
    if piVar11[0x29] != 0:
        SendMessageA(piVar11[0x29], 0x4D8, wParam, (LPARAM)hwnd)  ; notify owner
```

Active in YR: Yes for dialogs with SHP-animated statics (score, mission). Not
active for main-menu `0xE2` statics.

All WM_PAINT paths end with `ValidateRect(hwnd, NULL)`.

---

## 5. WM_TIMER (0x113)

Two distinct timer IDs are handled:

### 5.1 Timer ID 0x65 — Movie frame tick

Fires at `0x22` ms (≈ 34 fps, set by `0x4E4`/`0x4DF` cases).

Condition: `param_3 == 0x65`.

```
if piVar11[0x16] == 0: return 0
cVar2 = (*vtable__VQMovie[1])()    ; query if frame ready (vtable+0x04)
if cVar2 != 0: InvalidateRect(hwnd, NULL, 0)
cVar2 = (*vtable__VQMovie[5])()    ; query if playback complete (vtable+0x14)
if cVar2 == 0: return 0
// Playback complete:
if piVar11[0x17] != 0:             ; loop flag set
    (*vtable__VQMovie[7])(1)       ; vtable+0x1C: BinkGoto(frame=1, wait=1)
    Register_heap_pool("Looping movie")  ; heap pool log
    return 0
// Not looping: destroy movie handle
(vtable__VQMovie[0])(1)            ; vtable+0x00: destructor
piVar11[0x16] = 0
KillTimer(hwnd, 0x65)
if piVar11[0x18] != 0:
    destructor(piVar11[0x18])(1)
    piVar11[0x18] = 0
```

Active in YR: Yes — this is the live path for `0x71A` movie playback.

### 5.2 Timer ID 0 — Text-anim / SHP-anim tick

Fires at `piVar11[0x21]` ms (text-anim interval).

Dispatches on `piVar11[0x1c]` (kind):

- **Kind 1 (text-anim)**: `InvalidateRect(hwnd, NULL, 1)` to trigger repaint scroll step.
- **Kind 2 or 3 (image/SHP-anim)**: `InvalidateRect(hwnd, NULL, 1)` then `KillTimer(hwnd, 0)`.
- **Kind 4 (SHP-frame anim)**: check tick delta against interval; if exceeded, `InvalidateRect(hwnd, NULL, 1)`.
  Then on expiry: if `piVar11[0x25]` (frame count) is 0, kill timer; otherwise read
  next-frame interval from `*(short *)(piVar11[0x1e] + 6)` and restart timer.

Active in YR: Yes for kind-1 text-anim statics on other dialogs. Not fired for
any of the four main-menu `0xE2` statics (they are all static text, no animation).

---

## 6. Message 0x497 — Init / Reset

This is the "compute initial state" message, broadcast from `FUN_0060F9A0`
(`SendMessageA(hwnd, 0x497, 0, 0)` at `0x00610333`) for every child after
subclass installation.

```
// Look up dialog parent HWND
pHStack_74 = GetParent(hwnd)
piVar11[0x1c] = 0             ; kind = empty
GetDlgCtrlID(hwnd)            ; read control id (return value discarded in decompile but used by classifiers)
piVar11[0x2b] = 0x0C          ; text shadow/flags arg = 0x0C (dead in FUN_00621040, vestige)
piVar11[0x3b] = DAT_00ac18a4  ; text color = global default (0xFFFF = yellow)
```

Additionally, `FUN_00603240(parent_id, ctrl_id)` and related classifier functions
may be consulted by the orphan SHP-attach code path, but for all four `0xE2`
statics this lookup does nothing observable (see sibling doc
`STATIC_0X71C_RUNTIME_VISIBILITY_GHIDRA_REPORT.md`).

Active in YR: Yes for all four `0xE2` statics on every dialog open.

---

## 7. Messages 0x498–0x4D4 — Property-Set Messages

| Message | Hex | Behavior |
|---|---|---|
| `WM_SETTEXT` / text-set | `0x4B2` | Copy `lParam` (wchar_t*) into record text buffer via `FUN_00775690` helper, then `InvalidateRect`. Used by dialog proc to push tooltip and version text. |
| `0x4B4` | | Same as `0x4B2` — secondary text-set path that also triggers background blit update. |
| Color-set | `0x498` | `piVar11[0x3b] = lParam`; if changed from old, `InvalidateRect`. If `lParam == -1`, resets to `DAT_00ac18a4`. |
| Background fill | `0x4B1` | `piVar11[0x2d] = 1; piVar11[0x2e] = lParam` (RGB). |
| Anim-cycle msg | `0x4D3` | If kind == 4 and anim not running: start timer, begin SHP-anim cycle. Uses `FUN_006033f0()` as timer interval (returns `100` ms for `(0xE2, 0x71C)`). |
| Stop anim | `0x4D4` | If kind == 4 and anim running: KillTimer, clear anim flag. |
| Set frame idx | `0x4D5` | If kind == 4: `piVar11[0x26] = lParam`, `InvalidateRect`. |
| Get frame idx | `0x4D6` | If kind == 4: return `piVar11[0x26]`. |
| Set notify | `0x4D7` | If kind == 4: `piVar11[0x29] = lParam` (notify HWND). |

Active in YR: `0x4B2` Yes (used for `0x695` tooltip and `0x71D` version text).
`0x498` Yes (text-color override, not used on `0xE2` main-menu path).
`0x4D3/0x4D4/0x4D5/0x4D6/0x4D7` Yes for other dialogs; not for main-menu `0xE2` statics.

---

## 8. Custom Shell Messages 0x4DF–0x4F0 — Movie Control

| Message | Hex | Behavior |
|---|---|---|
| Load+play movie (path A) | `0x4DF` | Same as `0x4E4` except wParam bounds-checks against `DAT_00abf3a0`. Sets up `VQMovieHandle`. |
| Pause movie | `0x4E0` | Call `vtable__VQMovie[3]` (vtable+0x0C) with arg `1` (pause). |
| Resume movie | `0x4E1` | Call `vtable__VQMovie[3]` (vtable+0x0C) with arg `0` (unpause). |
| Stop/destroy movie | `0x4E2` | Destroy movie handle: call `vtable[0]` (destructor), set slot to 0, KillTimer `0x65`, destroy secondary handle `piVar11[0x18]`. |
| Set loop flag | `0x4E3` | `piVar11[0x17] = wParam`. Main menu passes `1` (loop). |
| Load+play movie (path B) | `0x4E4` | Destroy old movie, `FUN_00775690()`, construct `VQMovieHandle` for base-name string in `lParam`, `MoveWindow` to movie dims, `SetTimer(hwnd, 0x65, 0x22, NULL)`. |
| Start SHP-anim | `0x4EE` | If kind == 1 and anim not running: set running flag, `piVar11[0x20] = 1`, `SetTimer(hwnd, 0, piVar11[0x21], NULL)`, `InvalidateRect`. |
| Draw/copy frame | `0x4F0` | Call `vtable__VQMovie[10]` (vtable+0x28): explicit copy/draw current Bink frame. |

### 0x4E4 detail (main-menu `0x71A` path)

```
// Destroy any live movie
if piVar11[0x16] != 0:
    (piVar11[0x16]→vtable[0])(1)   ; destructor
    piVar11[0x16] = 0
KillTimer(hwnd, 0x65)
if piVar11[0x18] != 0:
    (piVar11[0x18]→vtable[0])(1)
    piVar11[0x18] = 0
FUN_00775690()   ; get window rect in client coords (side effect: primes local rect vars)
// Construct VQMovieHandle for base-name lParam (e.g. "Ra2ts_l")
piVar4 = VQMovieHandle__Constructor(0)  ; 0 = from base-name string
piVar11[0x16] = piVar4
if piVar4 != 0:
    (*piVar4→vtable[0x18])(pHStack_20, iStack_1c)  ; open/load movie (dims returned)
    MoveWindow(hwnd,
               iStack_28, iStack_24,            ; x,y from FUN_00775690 output
               *(piVar11[0x16]+8),               ; movie width
               *(piVar11[0x16]+0xC),             ; movie height
               0)
    SetTimer(hwnd, 0x65, 0x22, NULL)            ; 34 fps tick
    return 0
// Failure path:
KillTimer(hwnd, 0x65)
if piVar11[0x18] != 0:
    (piVar11[0x18]→vtable[0])(1)
    piVar11[0x18] = 0
```

Active in YR: Yes — `0x4E3` and `0x4E4` are both sent to `0x71A` from
`FUN_00531CC0` and `FUN_0052B9B0` on every main-menu open. `0x4F0` is sent by
dialog proc `0x00531F60` on every `WM_PAINT`. All three are the live Bink-playback
control path for the RA2TS movie panel.

---

## 9. WM_DESTROY (0x02) — Cleanup

```
if piVar11[4] != 0:
    (piVar11[4]→vtable[0])(1)    ; destroy BSurface
    piVar11[4] = 0
if piVar11[0x1f] != 0 (own SHP data):
    FUN_007c8b3d(piVar11[0x1e])  ; free SHP data
    piVar11[0x1e] = 0
if kind == 1 and anim running:
    KillTimer(hwnd, 0)
elif kind == 4:                ; no anim-running check for kind 4 — KillTimer unconditional
    KillTimer(hwnd, 0)
; (corrected 2026-05-29: original said "kind==1 or kind==4 and anim running" implying anim check
; applies to both; binary shows kind==4 bypasses the anim-running guard and kills timer unconditionally
; via a direct goto LAB_00615502. Only kind==1 checks (char)piVar11[0x2a] != 0 before killing.
; verified via decompile_function 0x006153E0 — OPERATOR_OR_ORDER_DRIFT)
if piVar11[0x16] != 0:
    (vtable[0])(1)               ; destroy movie handle
    piVar11[0x16] = 0
KillTimer(hwnd, 0x65)
if piVar11[0x18] != 0:
    (vtable[0])(1)
    piVar11[0x18] = 0
// fall through to CallWindowProcA default
```

Active in YR: Yes.

---

## 10. Per-Control Paint Behavior on Dialog 0xE2

### 10.1 Static `0x694` — Title heading "GUI:MainMenu"

- Kind: 0 (empty/text)
- Text: `GUI:MainMenu` resolved at init via `StringTable__LoadString`, stored
  at `piVar11[0x19]` (wchar_t*). Also cross-checked: the text check uses
  `piVar11[10]` for non-null guard; this is index `0xA` (byte `0x28`) which in
  `FUN_0060F9A0` is where the text pointer slot is written.
- Color: `DAT_00ac18a4 = 0xFFFF` = yellow `#FFFF00`.
- Alignment: low style bits `0x07` → bit 0 set → `uVar10 = 0x11` → h-center, top-anchored (NOT v-centered — bit 0x10 in the immediate is a no-op in `FUN_00621040`; see §11 correction).
  If `WS_DISABLED` (style bit `0x8000000`) → color overridden to `DAT_00ac1cb4 = 0x9F` (#9F0000 dark red).
  Not disabled on the `0xE2` path.
- Font: `g_GAME_FNT @ 0x0089C4D0` (17 px, GAME.FNT), resolved inside `FUN_00621040`.
- No shadow, no outline (single-pass glyph blit, verified in `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`).
- Shadow/flags arg `piVar11[0x2b] = 0x0C` but dead in `FUN_00621040`.
- No movie, no SHP, no anim timer.
- `ValidateRect` after text draw.
- Active in YR: Yes.

### 10.2 Static `0x695` — Tooltip / hover status line

- Kind: 0 (text) after `0x4B2` sends from the shell hit-test proc.
- Starts with title `GUI:Blank` (empty/blank CSF entry).
- Text updated dynamically: `FUN_00622B50`'s `0x84` hit-test path sends
  `SendMessageA(hwnd_695, 0x4B2, 0, (LPARAM)tooltip_wchar)`.
- `0x4B2` handler: copies text pointer into record, `InvalidateRect`, next
  `WM_PAINT` triggers kind-0 text draw.
- Color: `DAT_00ac18a4 = 0xFFFF` = yellow; no override.
- Alignment: same style-bit path as `0x694` (bit 0 set → `0x11` → h-center, top-anchored — NOT v-centered; see §11 correction).
- No movie, no SHP. Low-traffic invalidation: only on mouse hover changes.
- Active in YR: Yes.

### 10.3 Static `0x71D` — Bottom-right version / status line

- Kind: 0 (text).
- Starts with title `GUI:Blank`; populated by `MainMenuDialog0xE2_Proc_00531F60`
  case `0x497` via `SendMessageA(hwnd_71D, 0x4B2, 0, wide_version_string)`.
- Final text: `L"<GUI:Version label> <VERSION.TXT contents>"` (format `L"%s %s"`).
- Same paint path as `0x694` (kind 0, yellow, GAME.FNT, h-center, top-anchored — NOT v-centered; see §11 correction).
- Active in YR: Yes.

### 10.4 Static `0x71A` — RA2TS Bink movie panel

- Kind: 4 (movie) — set implicitly when `VQMovieHandle__Constructor` succeeds.
- Receive sequence on every main-menu open:
  1. `SendMessage(0x71A, 0x4E3, 1, 0)` → loop flag = 1.
  2. `SendMessage(0x71A, 0x4E4, 0, "Ra2ts_s"/"Ra2ts_l")` → construct VQMovie,
     resize window to movie dims, start timer `0x65` at 34 fps.
  3. Per `WM_PAINT` (fired from `WM_PAINT` of dialog proc): `SendMessage(0x71A, 0x4F0, 0, 0)` → explicit Bink frame copy.
- Timer `0x65` queries frame readiness and loops via `BinkGoto(1, 1)` at
  end-of-movie since `piVar11[0x17] = 1` (loop flag).
- On `WM_PAINT`: since `piVar11[0x16] != 0`, immediately `ValidateRect` and
  return — paint is suppressed; `0x4F0` is the real draw trigger.
- On `0x4E2` (stop movie): destroy VQMovie, kill timer, clear slot. This is
  sent by `FUN_00622B50` when record byte `+0xBE` is set.
- Active in YR: Yes.

### 10.5 Static `0x71C` — Blank/dead

- Kind: 0, no text, no SHP, no movie. Set by `0x497` init: `piVar11[0x1c] = 0`,
  `piVar11[0x3b] = DAT_00ac18a4`, `piVar11[0x2b] = 0x0C`.
- `WM_PAINT`: `piVar11[0x2f]` = 0, `piVar11[0x16]` = 0, surface allocated on
  first paint, background blit from parent, kind = 0, text check `piVar11[10]`
  = 0 (no text ever set), text block not entered → `ValidateRect` only.
- No message from `FUN_00531CC0`, `FUN_0052B9B0`, or `MainMenuDialog0xE2_Proc_00531F60`
  ever targets `0x71C` (verified in sibling doc).
- Visible output: none. The parent SHP-stack composite shows through.
- Active in YR: No visible output. Reposition by `ResizeShellChildControl_0060C0C0`
  happens but has no visual effect.

---

## 11. FUN_00621040 Call Convention (recap for Static path)

Verified calling convention — callsite at `0x006153E0` WM_PAINT branch:

```
__fastcall FUN_00621040(
  ECX: XSurface* surface (backing surface, &pHStack_50)
  EDX: wchar_t*  text    (piVar11[0x19])
  arg_1 (stack): u32  color  (iVar6 = piVar11[0x3b] or DAT_00ac1cb4)
  arg_2: u8   align flags — Static-side IMMEDIATE values are 0x10 / 0x11 / 0x12
              but bit 0x10 is a NO-OP inside FUN_00621040 (verified — only bits
              0x01 h-center, 0x02 h-right, 0x04 v-center are tested). Effective
              meanings: 0x10 = top-left, 0x11 = top + h-center, 0x12 = top + h-right.
              Static text is TOP-anchored, NOT v-centered. See §11 detailed
              correction notes.
  arg_3: u32  (always 0x0C from piVar11[0x2b])
  arg_4: u32  (always 0)
  arg_5: i32  fade_count (= piVar11[0x20] scroll offset; 0 for static text)
  arg_6: i32  fade_range (= piVar11[0x23]; 0 for static text)
)
```
; (corrected 2026-05-29: original listed `arg_1: RECT* clip_rect` and `arg_2: BitFont* font`
; as the first two stack args before color/align, totalling 10 params. The actual callsite
; `FUN_00621040(&pHStack_50, piVar11[0x19], iVar6, uVar10, piVar11[0x2b], 0, piVar11[0x20], piVar11[0x23])`
; shows 8 params total (ECX, EDX + 6 stack) with color as first stack arg, no separate clip_rect
; or BitFont* stack arguments visible. The callee's `param_3` Ghidra types as `int*` (used as rect)
; but receives the color int at the callsite — type confusion in Ghidra. Corrected to match callsite
; literal. verified via decompile_function 0x006153E0 — INFERENCE_HARDENED / PARAM1_TYPE_MISREAD)

The align flags `0x10/0x11/0x12` are the Static-proc variants (different from
Button's `0x05`). Internally `FUN_00621040` tests bit `0x04` for v-center and
passes the full byte to `FUN_00434CD0` which tests bit `0x01` for h-center.

Mapping:
- `0x10` → no h-center, no v-center (plain top-left)
- `0x11` → h-center + v-center (`0x01 | 0x10` — the internal test `& 0x04` is not
  set at 0x10, but passing `0x11` to DrawWithWrap sets h-center; the v-center
  is gated on `(param_6 & 4) != 0` in `FUN_00621040` which is `0x11 & 4 = 0` —
  so `0x11` = h-center only).
- `0x12` → h-right + (no v-center). Actually examining `FUN_00621040` body:
  `if (param_6 & 4)` tests bit 2; `0x10 & 4 = 0`, `0x11 & 4 = 0`, `0x12 & 4 = 0`.
  V-center only if bit 2 (`0x04`) is set. For `0x11` = `0b00010001` → bit 4 set,
  bit 0 set; `0x12` = `0b00010010` → bit 4 set, bit 1 set.
  `FUN_00434CD0` h-center branch tests `param_8 & 1` (= `0x11 & 1 = 1` → h-center).
  `FUN_00621040` v-center branch tests `param_6 & 4` (bit 2). So v-center requires
  the caller to pass a value with bit 2 set; `0x10/0x11/0x12` all have bit 2 clear.
  Conclusion: all three values from the Static path produce **no v-center** from
  `FUN_00621040`'s own pre-pass; v-center only happens via DrawWithWrap's handling.
  The Button path uses `0x05 = 0b00000101` (bit 2 + bit 0) → `FUN_00621040` v-centers
  then passes `0x05` to DrawWithWrap for h-center.
  The Static path passes `0x11` (bit 0 = h-center only to DrawWithWrap; no
  `FUN_00621040` v-center pre-pass). Text is drawn h-centered, top-aligned within
  the rect.

Active in YR: Yes for all four `0xE2` statics that draw text.

---

## 12. TS-vs-YR Filter

| Code path | Active in YR |
|---|---|
| Kind 0/1 text draw (`0x694`, `0x695`, `0x71D`) | Yes |
| Kind 4 movie path (`0x71A`) | Yes |
| Kind 2 image centered blit | Conditional (other dialogs); No for `0xE2` statics |
| Kind 3 SHP-anim blit | Conditional (other dialogs); No for `0xE2` statics |
| `0x4EE` text-scroll anim start | Conditional (other dialogs); No for `0xE2` statics |
| `0x4D3/0x4D4/0x4D5/0x4D6/0x4D7` SHP-anim control | Yes for score/other dialogs; No for `0xE2` main menu |
| `0x4DF` alternate movie-load (with bounds check) | Yes for some dialogs; not used on `0xE2` path |
| Static `0x71C` visible output | No |
| Orphan SHP-attach region `0x0060A338..` | No (no callers) |
| `bud_*` disabled PCX art family | No (dead, TS legacy; but this is Button not Static) |

---

## 13. Confidence Summary

| Claim | Confidence | Evidence |
|---|---|---|
| Full message switch decompiled | HIGH | Full decompile of `0x006153E0–0x00616300` in this session |
| Kind 0/1 text draw path | HIGH | Traced all branches; confirmed Yellow `#FFFF00`, GAME.FNT, `FUN_00621040` call shape |
| `0x4E3` sets loop flag at `piVar11[0x17]` | HIGH | Decompile case `0x4E3`: `piVar11[0x17] = param_3` |
| `0x4E4` constructs VQMovie, starts timer 0x65 at 34fps | HIGH | Decompile case `0x4E4`; timer interval `0x22` = 34 ms |
| `0x4F0` calls movie vtable+0x28 | HIGH | Decompile case `0x4F0`; `(*vtable[0x28])()` |
| Timer 0x65 loop via vtable+0x1C with `BinkGoto(1,1)` | HIGH | Decompile timer branch at `param_3 == 0x65` |
| Disabled text uses `DAT_00ac1cb4 = 0x9F` (#9F0000) | HIGH | Decompile + `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §Q1a` |
| `piVar11[0x2b] = 0x0C` dead arg | HIGH | `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §3`; init traced from `FUN_0060F9A0` |
| `0x71C` draws nothing | HIGH | `STATIC_0X71C_RUNTIME_VISIBILITY_GHIDRA_REPORT.md` full exhaustive trace |
| Align mode `0x11` → h-center, no v-center in `FUN_00621040` pre-pass | HIGH | `FUN_00621040` decompile; bit-test `param_6 & 4` = 0 for `0x11` |

---

## 14. Sources

Ghidra functions decompiled in this session (read-only):

- `OwnerDraw_Static_006153E0 @ 0x006153E0` — full body
- `FUN_00621040 @ 0x00621040` — full body (calling convention confirmation)
- `FUN_00775690 @ 0x00775690` — get-window-rect-in-client-coords helper

Prior reports referenced:

- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `MAIN_MENU_TITLE_TEXT_RENDER_GHIDRA_REPORT.md`
- `STATIC_0X71C_RUNTIME_VISIBILITY_GHIDRA_REPORT.md`
- `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`
- `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`
- `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`

INI files checked: none (owner-draw static paint has no INI surface).
