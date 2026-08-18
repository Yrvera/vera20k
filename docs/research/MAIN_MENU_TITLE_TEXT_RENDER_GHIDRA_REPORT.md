# Main Menu Title Text Render — Ghidra Report

Date: 2026-05-19

## 2026-07-26 exactification correction

This section supersedes the affected `0x694` alignment, placement, color-state,
and paint-lifecycle claims below. Function labels remain navigation hints; the
load-bearing result is grounded in the cited instruction ranges plus guarded,
active-YR executable frames.

- The `0x694` draw mode is `0x11`: bit `0x01` horizontally centers, bit
  `0x04` would vertically center, and bit `0x10` is not a vertical-centering
  request. Because `0x04` is absent, the title is top-aligned.
- At the enrolled 800x600 breakpoint the active external visible rect is
  exactly `[635,9,798,27)`, or `163x18`. The dialog resource contributes a
  `162x16` nominal rect; the enrolled same-size `0x71D` compatibility family
  establishes a `163x17` pre-finalizer control, and the verified
  `FUN_0060B950` `0xE2`/`0x694` branch adds `+7` to y and `+1` to height.
  The internal source of the one-pixel compatibility width remains
  unlocalized; it is not evidence of a second `FUN_0060B950` resize.
- The title uses the active kind-1 static reveal. With the stock English
  `GUI:MainMenu` value, UTF-16 length `9`, range `8`, step `1`, and interval
  `0x1e`, the target is `18`. The final invalidated paint displays count
  `17`; its post-paint update advances the internal count to `18`, kills the
  timer, and does not invalidate again. The persistent player-visible state
  therefore remains count `17`.
- Path-A color is content-agnostic: at displayed count `17`, UTF-16 unit 9
  receives encoded RGB `(255,255,30)`. The active RGB565 path stores
  `(31,63,3)`, and the verified presentation codebook produces BGRA
  `(25,255,255,255)`. The guarded native title has 214 pure-yellow foreground
  pixels and 29 such tinted foreground pixels; those 29 pixels happen to form
  the final stock-English `u`, but no implementation may hardcode that glyph.
- Guard SHA-256:
  `fe32f218137b76a91dc3bac07bc96372a61c22e48ea083519f1ecbdbd97d601c`.
  Native active-YR frame SHA-256:
  `69a15fd903831ea6e82f56b0d717eb80d27e626af92121116c9274e75239b0f1`.
  Two independent hidden no-input Rust captures,
  `e2-title-static-green-a-20260726-114632` and
  `e2-title-static-green-b-20260726-114656`, both produced the same full-frame
  SHA-256
  `688e259ece407df446e29a3c3b030cc9dc958997b8c7a0bd921bc201095d6e7b`
  and exact native title-region SHA-256
  `f8a87d35f9225a3d9c8e1d313ac42684eec788e49470884eaa1564ae3e613f6b`.
  Each comparison reported `9376/9376` presentation pixels equal for the
  `163x18` title rect; the Single Player, Options, and Exit Game preservation
  crops also matched exactly.

The executable differential above verifies this title checkpoint at guarded
800x600. The derived 640x480 and 1024x768 layouts remain geometry-regression
evidence, not native pixel certification at those breakpoints.

## Scope

What text — if any — is drawn at the top of dialog `0xE2` (the standard
Yuri's Revenge initial main menu): identify CSF key(s), draw function, font,
color, position relative to the dialog client, and confirm YR-active state.
Button labels are out of scope (covered by
`MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`). The body
of the quit-confirm modal `0x120` is out of scope (covered by
`QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md`).

Parent reports:

- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` (controls,
  layout, paint order)
- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md` (PCX
  button text path)
- `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` (GAME.FNT + `FUN_00621040` draw stack)

No Rust code or Ghidra annotations were modified.

## Executive Summary

Dialog `0xE2` draws **two text strings** at top-of-dialog / overlay positions
(other than the six right-side button labels):

1. **Top-of-right-panel heading** — owner-draw static `0x694`, CSF key
   `GUI:MainMenu` (verified-by-resource-parse in parent report; the literal
   ASCII string `GUI:MainMenu` is NOT present in `.rdata`/`.text` because it
   lives in the `RT_DIALOG` resource where Ghidra's string scanner does not
   index it). Font GAME.FNT, color `0x0000FFFF` interpreted as RGB `#FFFF00`
   (yellow), horizontal-center + top alignment, drawn through
   `OwnerDraw_Static_006153E0` → `FUN_00621040`.
2. **Bottom-right version/status line** — owner-draw static `0x71D`, CSF key
   `GUI:Version` (verified ASCII at `0x0082696c`), concatenated with the
   contents of `VERSION.TXT` (or a numeric fallback `"%d.%3.3dTUC"`) using the
   wide format `"%s %s"` at `0x00826960`. Same GAME.FNT, same yellow
   `#FFFF00`, same static draw path.

There is **no separate title-bar widget, no copyright/Westwood overlay, no
build-number splash** drawn by either the dialog proc
`MainMenuDialog0xE2_Proc_00531F60` or the parent `WM_PAINT_Handler @
0x00621E90` on the standard `0xE2` path. Confirmed verified-negative:

- No `GUI:MainMenu` ASCII reference in code/data (only in dialog resource).
- No `GUI:Title`, `GUI:Copyright`, `GUI:MainMenuTitle`, `GUI:RedAlertTitle`,
  `TXT_MAIN_MENU`, `TXT_VERSION`, `© 2000`, `Westwood Studios` strings of any
  kind dropped on the `0xE2` paint path.
- `Main_Game @ 0x0052D9A0` (the shell driver that calls `FUN_00531CC0`) does
  not draw any text before/around the dialog — it only kicks the `INTRO`
  music theme and enters the dialog modal.
- `GUI:TradeMarkTop @ 0x00826858` and `GUI:TradeMarkBottom @ 0x00826844` are
  CSF keys present in `gamemd.exe` but no xref reaches them from the `0xE2`
  paint path. They belong to other shell screens.

Active in YR: Yes (both static `0x694` and `0x71D` populate every time the
initial main menu is shown).

## 1. Title heading — static `0x694`

### Source CSF key

`GUI:MainMenu` (per `RT_DIALOG 0xE2` template's child-control title; parsed
in `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` table at
the top of that doc). The text is loaded at subclass time, NOT at paint time.

### CSF key → wide string conversion

`FUN_0060F9A0` (owner-draw subclass installer, called once per child during
`WM_INITDIALOG` via `FUN_00622B50`) reads the Win32 dialog template title
through `CallWindowProcA(prev_proc, hwnd, WM_GETTEXT, 0x800, buffer)` at
`0x006102f5`. If the buffer is non-empty, it calls:

```text
0x00610310  XOR  EDX, EDX                ; out-id-ptr = NULL
0x00610304  PUSH 0x1ce0                  ; __LINE__ = 7392
0x00610309  PUSH 0x00833730              ; __FILE__ = "ownrdraw.cpp"
0x00610310  LEA  ECX, [ESP + 0x2c8]      ; key = the template title buffer
0x00610317  CALL 0x00734e60              ; StringTable__LoadString
```

`StringTable__LoadString @ 0x00734E60` looks up the key in the loaded CSF
table (`DAT_00b1cf74` head, `DAT_00b1cf78` value-array) and returns a
`wchar_t*` to the localized translation. That pointer is then stored in the
owner-draw record at `record[+0x2c] / piVar14[0xb]` via `FUN_00623560`.

If the CSF key is missing, `StringTable__LoadString` returns a heap-allocated
`MISSING: <key>` placeholder (no crash).

### Paint path

`OwnerDraw_Static_006153E0` on `WM_PAINT` (`param_2 == 0x0f`) at the text
draw site `~0x00615b00`:

```text
FUN_00621040(&clip_rect,
             piVar11[0x19],  ; text source pointer (wchar_t* from CSF)
             iVar6,          ; color = piVar11[0x3b] (default DAT_00ac18a4)
             uVar10,         ; align mode = 0x10 / 0x11 / 0x12 from style bits
             piVar11[0x2b],  ; outline/shadow flags = 0x0c (set on init)
             0,
             piVar11[0x20],
             piVar11[0x23]);
```

Static style for `0x694` from the resource is `0x50000007` (style low byte
`7` per `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`).
For the owner-draw static, the verified draw mode is `0x11`. Bit `0x01`
horizontally centers the glyph span. Vertical centering is bit `0x04`, which
is absent; bit `0x10` does not vertically center the text. The `0x694` title
is therefore horizontally centered and top-aligned.

`FUN_00621040` (per `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`):

- defaults font to global `g_GAME_FNT @ 0x0089C4D0` (GAME.FNT, 17-px line
  height, `fonT`-magic format),
- converts the 24-bit RGB color to 16-bit display format,
- sets the BitFont clip rect to the supplied rect,
- dispatches into `FUN_00434CD0` for the actual per-glyph rasterization.

### Font

**GAME.FNT** (cell height 17 px, line gap 1 px, 1 bit/pixel glyphs, MSB-first;
loaded once into the global `g_GAME_FNT @ 0x0089C4D0`). Confirmed by
`BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` and by `FUN_00621040` resolving the
font from the same global when the static draw passes
`piVar11[0x19] = <text-ptr>` (the font argument resolves to `g_GAME_FNT`
inside the wrapper, not from a per-control field).

### Color

**`#FFFF00` yellow.** Source: `DAT_00AC18A4`, initialized to `0xFFFF` by
`FUN_0060F9A0 @ 0x0060FA3F` (`MOV dword ptr [0x00ac18a4], 0xFFFF`) before any
control subclass runs. On `WM_USER`-class `0x497`, `OwnerDraw_Static_006153E0`
stores `DAT_00AC18A4` into the per-control color slot `piVar11[0x3b]`. There
is no `0x498` recolor override on the `0xE2` path. `FUN_00621040`
interprets the 24-bit value as packed RGB (low byte = red, middle = green,
high = blue) → `(R=0x00, G=0xFF, B=0xFF)` would map to cyan, but
`BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` notes the wrapper actually treats
the value as `0x00RRGGBB` with byte-order convention giving the final visible
result of yellow `#FFFF00` for the constant `0xFFFF`. The composition doc
matches this interpretation ("yellow `#FFFF00`") and it is consistent with
the button-label text using the same global and rendering yellow.

### Position relative to dialog client

`RT_DIALOG 0xE2` template rect for control `0x694`: DLU `425,1,108,10`.
Its nominal 800x600 mapping is `162x16`. Guarded same-family runtime evidence
establishes a `163x17` compatibility rect before the title-specific finalizer.

`FUN_0060B950` (called from the `FUN_0060C4A0 → ResizeShellChildControl`
enumeration during `WM_INITDIALOG`) applies a dialog-`0xE2`-specific nudge:
top moves down by **+7 px**, height increases by **+1 px**, leaving x and
width unchanged. Verified path: `iVar10 == 0xe2` AND `iVar5 == 0x694`
satisfy the upper guarded branch; `0xe2` is NOT in the inner `+1`
allow-list (`0xbc / 0xbd / 0x102 / 0xc2 / 0xc9 / 0xbc6 / 0x105 / 0x6b /
0x113`), so the path falls to `iVar9 = iVar8 + 7; local_34 = iVar3 + 1`
and `MoveWindow` is issued.

Final active external visible rect (800x600 shell): exactly
`[635,9,798,27)`, or `(635,9,163,18)`.

At larger shell resolutions the right-panel block recenters (see
`RightPanel__ComputeLayoutRects` rules in the composition doc) but the
nudge constants `+7` top and `+1` height stay fixed.

## 2. Bottom-right version line — static `0x71D`

### Initial state

Template title is `GUI:Blank` (resolves to an empty-ish localized blank by
the same CSF path described in section 1). Initial paint produces no visible
text.

### Live update trigger

Custom message `0x497` ("init/refresh sequence") posted to the main-menu
dialog ends up in `MainMenuDialog0xE2_Proc_00531F60` case `0x497` at
`0x00531fbb`. The dispatch is verified by disassembly:

```text
0x00531fbb  PUSH 0x71d                ; control id
0x00531fc0  PUSH ESI                  ; hDlg
0x00531fc1  CALL [GetDlgItem]
0x00531fce  CALL 0x0074fae0           ; FUN_0074fae0 (load VERSION.TXT)
0x00531fd5  CALL 0x00735120           ; FUN_00735120 (ASCII→UTF16 ring)
0x00531fe7  MOV  ECX, 0x82696c        ; CSF key "GUI:Version"
0x00531fe5  XOR  EDX, EDX             ; out-id-ptr = NULL
0x00531fe0  PUSH 0x825fb8             ; __FILE__ = "Init.CPP"
0x00531fdb  PUSH 0x1757               ; __LINE__ = 5975
0x00531fec  CALL 0x00734e60           ; StringTable__LoadString
0x00531ff6  PUSH 0x00826960           ; wide format L"%s %s"
0x00531ffc  CALL 0x007ca564           ; wide vsprintf
0x0053200b  PUSH 0x4b2                ; static "set text" message
0x00532011  CALL [SendMessageA]       ; send to 0x71D
```

### Source CSF key

`GUI:Version` — ASCII at `0x0082696c`. Verified by Ghidra string scan and by
the `MOV ECX, 0x82696c` immediate above.

### Wide format

`L"%s %s"` at `0x00826960` (the only xref into this address from the
`0xE2` dialog proc is from `MainMenuDialog0xE2_Proc_00531F60` at `0x531ff6`,
confirmed via `get_xrefs_to(0x00826960)`).

### Value composition

`FUN_0074fae0` reads `VERSION.TXT` through a `RawFileClass` (string
`"VERSION.TXT"` at `0x0084635c`), trims trailing `\r`, caches the contents
in `DAT_00a8ed0a..` plus a per-caller copy. If the file is missing,
`FUN_007c8ef4` formats the numeric fallback using `"%d.%3.3dTUC"` at
`0x00846368`.

`FUN_00735120` converts the ASCII buffer to a UTF-16 buffer in a global
8-slot ring at `DAT_00b13f24 + slot * 0x800`. The wide pointer returned
becomes the second `%s` in the format.

The first `%s` is the localized `GUI:Version` wide string.

Final string sent to control `0x71D`: `L"<localized GUI:Version label> <VERSION.TXT contents>"`.

### Send mechanism

`SendMessageA(hwnd_71d, 0x4b2, 0, (LPARAM)wide_buffer)`.
`OwnerDraw_Static_006153E0` case `0x4b2` calls `FUN_00775690` and copies the
new text into the owner-draw record, then `InvalidateRect`.

### Paint, font, color, position

Same `FUN_00621040` path as section 1.

- Font: GAME.FNT.
- Color: yellow `#FFFF00` from `DAT_00AC18A4 = 0xFFFF` (no recolor in the
  `0xE2` path).
- Alignment: low style bit `1` was interpreted as horizontal-center +
  vertical-center in the original pass. The 2026-07-26 correction
  reclassifies `0x694`, not this bottom-line control; `0x71D` vertical
  alignment remains `UNCHECKED` here.
- Resource DLU rect: `425,357,108,10`, pixel-mapped `(638,580,162,16)`.
- `FUN_0060B610` applies the right-panel bottom-cap inset using
  `DAT_007F5BF8 = 168` and the right-panel-bottom slot `DAT_00B0FC28`. With
  control width approximately `162` px the inset is `(168-162)/2 = 3` px,
  so the final 800×600 placement is approximately `(635, bottom_cap_y - 16, 162, 16)`.

### Trigger frequency

`0x497` is broadcast by `FUN_0060F9A0` at the end of each subclass install
(`SendMessageA(param_1, 0x497, 0, 0)` at `0x00610333`). Therefore every
re-entry into the dialog (every first-show, every dialog re-init) populates
`0x71D` with the version string. This is the dialog's "(re)compute initial
state" message — same role as a `WM_INITDIALOG` callback for owner-draw
fields. The text remains visible for the lifetime of the dialog.

## 3. Verified-negative: no other title text

A directed scan of the `0xE2` paint path turned up no additional text draws
at top-of-dialog positions:

- `MainMenuDialog0xE2_Proc_00531F60` handles only three messages: `WM_PAINT`
  (forwards a movie-copy to `0x71A`), `WM_COMMAND` (button → return code),
  and `0x497` (sets `0x71D` text). No `0x4b2` is sent to `0x694`, `0x71A`,
  `0x71C`, `0x695`, or to any unknown top-area static from this proc.
- `WM_PAINT_Handler @ 0x00621E90` (parent paint) draws the right-panel SHP
  stack and the parent shell background. No text draw at all happens in
  this handler — it composites images only, then `BltAt`s to the screen
  surface. The owner-draw text controls then paint themselves via their own
  `WM_PAINT` handler when invalidated.
- `RightPanel__Draw @ 0x0072E450` draws SHPs only (`SDTP`, `SDBTNBKGD`,
  `SDBTNANM`, `SDBTM`, `LWSCRNS/L`). No text.
- `Main_Game @ 0x0052D9A0` enters the dialog modal via `FUN_00531CC0` and
  does no text rendering of its own around the dialog.
- The `0x694` heading rect (DLU `425,1,108,10`, pixel `638,9..26`) is
  the only top-of-right-panel widget present in the dialog resource. No
  other Static at y < 50 px is defined.

Specifically searched and absent (no string in `.rdata`, no xref from the
`0xE2` path):

| Search term | Result |
|---|---|
| `GUI:MainMenu` (ASCII or UTF-16) | absent in .rdata; lives only in `RT_DIALOG 0xE2` resource title for child `0x694`. |
| `GUI:Title` | absent. |
| `GUI:Copyright` | absent. |
| `GUI:MainMenuTitle` | absent. |
| `GUI:RedAlertTitle` | absent. |
| `TXT_MAIN_MENU` | absent. |
| `TXT_VERSION` | absent. |
| `Westwood Studios` literal | only in legacy WOL URLs and registry paths, no xref from `0xE2`. |
| `© 2000` (any encoding) | absent. |
| `Red Alert 2` / `Yuri's Revenge` (decorative) | only in WOL registry/URL strings, no xref from `0xE2`. |
| `GUI:WWBrand @ 0x0082687c` | exists but no xref from `0xE2` paint path. |
| `GUI:TradeMarkTop / Bottom @ 0x00826858 / 0x00826844` | exist but no xref from `0xE2`. |

The "© 2000 Westwood Studios" Westwood corporate text is **not** drawn by
the in-game shell on the standard YR initial menu. It is rendered earlier
by the Westwood-logo + Mammoth-tank intro movies (`WWLOGO.VQA`,
`mptitlee.bik`), which are part of the asset preroll and not part of dialog
`0xE2` itself.

## 4. Active-in-YR confirmation

- `FUN_00531CC0` is the standard shell driver invoked from `Main_Game` case
  `0x12`, which is the default game-mode-0 entry — i.e., the unmodified YR
  startup path with no recording / no map editor / no save-load.
- The subclass install path `FUN_00622B50 → FUN_0060F9A0` runs for every
  child of `0xE2` on every `WM_INITDIALOG`, including `0x694` and `0x71D`,
  with no `SpecialFlags` gate.
- `MainMenuDialog0xE2_Proc_00531F60` is the dialog proc literally
  pointed-to by the dialog creation site at `FUN_00531CC0` (verified by
  `0x00531CC0`'s call into `FUN_00622650(0)`, the shell dialog factory, and
  cross-referenced in the parent composition doc).
- The `0x497`-driven `GUI:Version` populate broadcast in `FUN_0060F9A0`
  always fires post-init, no flag gate.

No Tiberian Sun gating, no `SpecialFlags & 0x1000`, no
`g_FogOfWar`-style toggle, no `DAT_00a8ed5d` recording-only branch on either
text path.

## 5. Implementation implications (Rust port)

1. Render two CSF-translated text strings at top and bottom of the
   right-panel block: `GUI:MainMenu` centered in the top-of-panel slot,
   `GUI:Version + " " + VERSION.TXT-contents` centered in the bottom-cap
   slot.
2. Use GAME.FNT and yellow `#FFFF00`. The `0x694` title is horizontally
   centered and top-aligned in the exact `163x18` active rect, with the
   dialog-specific `+7` top / `+1` height finalizer. Preserve the separately
   owned `(168 - width) / 2` inset for `0x71D`; do not infer its vertical
   alignment from `0x694`.
3. Model the `0x694` kind-1 reveal as paint-committed state: timer ticks
   invalidate, a successfully presented dirty paint advances internal state,
   and the terminal displayed count remains retained after the non-invalidating
   completion update. Count UTF-16 units and apply the verified integer Path-A
   gradient; do not hardcode the stock-English final glyph.
4. The version-line population is a one-shot CSF-format-concat. Read
   `VERSION.TXT` once at startup, trim trailing `\r`, cache, fall back to
   `"%d.%3.3dTUC"` when the file is missing.
5. Do NOT render any additional title text, copyright, build banner, or
   Westwood logo over dialog `0xE2`. None of those exist on the verified
   `0xE2` paint path.

## 6. Confidence

| Claim | Confidence | Evidence |
|---|---|---|
| `0x694` title is `GUI:MainMenu` (in dialog resource) | HIGH | Composition doc resource-parse table; consistent with the only top-of-right-panel static in the dialog template. Direct string scan of `.rdata` is verified-negative, matching the expectation that the key lives in `RT_DIALOG`. |
| `0x71D` text = `GUI:Version + " " + VERSION.TXT` | HIGH | Disassembly traced step-by-step at `0x00531fbb..0x00532017`. CSF key string `GUI:Version` confirmed at `0x0082696c`. Wide format `"%s %s"` at `0x00826960` confirmed by xref. |
| Font is GAME.FNT | HIGH | `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`, verified xref through `FUN_00621040` defaulting font from `g_GAME_FNT @ 0x0089C4D0`. |
| Color/reveal terminal state | HIGH | `DAT_00AC18A4 = 0xFFFF`, verified Path-A instruction sequence, guarded active-YR native frame, and two deterministic Rust capture differentials. The exact title region matches all `9376/9376` presentation pixels. |
| Heading rect and nudge for `0xE2` `0x694` | HIGH at guarded 800x600 | `FUN_0060B950` branch fully traced; enrolled external rect `[635,9,798,27)`; native/Rust title-region differential exact. Other breakpoints remain geometry-regression evidence. |
| No other title/copyright/version text on `0xE2` | HIGH | All paint-path functions enumerated; all `0xE2` proc messages accounted for; targeted string scan turned up no candidate keys with `0xE2`-side xrefs; `Main_Game` does no surrounding text draw. |
| Active in YR | HIGH | Standard `Main_Game` case `0x12` path, no SpecialFlags or recording gate. |

## 7. Sources

Verified in this pass:

- `MainMenuDialog0xE2_Proc_00531F60` (`0x00531F60`) — decompile + disassembly.
- `FUN_00531CC0` (`0x00531CC0`) — decompile (shell driver).
- `OwnerDraw_Static_006153E0` (`0x006153E0`) — decompile (WM_PAINT and
  `0x4b2` text-set handlers).
- `FUN_0060F9A0` (`0x0060F9A0`) — decompile + disassembly of the WM_GETTEXT
  + StringTable__LoadString tail (`0x006102d6..0x00610327`) and the
  `0x497` self-broadcast (`0x00610333`).
- `FUN_0060B950` (`0x0060B950`) — decompile (heading reposition for `0xE2`
  `0x694`).
- `FUN_0074FAE0` (`0x0074FAE0`) — decompile (VERSION.TXT reader + numeric
  fallback).
- `FUN_00735120` (`0x00735120`) — decompile (ASCII→UTF-16 ring buffer).
- `StringTable__LoadString @ 0x00734E60` — decompile (CSF lookup
  signature: ECX=key, EDX=out-id-ptr, stack=__FILE__,__LINE__).
- `Main_Game @ 0x0052D9A0` — decompile (no surrounding text draw).

String scan evidence (read-only, no Ghidra edits):

- `GUI:Version @ 0x0082696c` ASCII (the only verified CSF key drawn on the
  `0xE2` text path beyond the resource-embedded `0x694` title).
- `L"%s %s" @ 0x00826960` UTF-16 format string.
- `"VERSION.TXT" @ 0x0084635c` ASCII.
- `"%d.%3.3dTUC" @ 0x00846368` ASCII numeric fallback.
- `D:\ra2mdpost\Init.CPP @ 0x00825fb8` (proc `__FILE__`).
- `D:\ra2mdpost\ownrdraw.cpp @ 0x00833730` (subclass `__FILE__`).
- Verified-absent (full negative scan): `GUI:MainMenu` ASCII or UTF-16,
  `GUI:Title`, `GUI:Copyright`, `GUI:MainMenuTitle`, `GUI:RedAlertTitle`,
  `TXT_MAIN_MENU`, `TXT_VERSION`, `© 2000`, `MainMenu` (any encoding).

Prior reports relied upon:

- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`
- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`

## 8. Open question

The visible RGB byte-order in `FUN_00621040`'s conversion of
`DAT_00AC18A4 = 0xFFFF` to a 16-bit display color is documented as yellow
by the composition doc and by `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`'s
convention, but a literal `0x0000FFFF` reads naturally as cyan under a
strict `0x00RRGGBB` interpretation. A final-pixel screenshot or a runtime
trace of `FUN_00621040`'s color-byte permutation would tighten the
"yellow vs cyan" choice to HIGH. The parent composition doc names it
yellow and matches the in-game appearance of the right-side button text
(which uses the same constant), so the working assumption is yellow
`#FFFF00`. This is the only residual ambiguity in this report.
