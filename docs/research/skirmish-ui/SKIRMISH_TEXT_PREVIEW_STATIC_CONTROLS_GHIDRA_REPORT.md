# Skirmish Text/Preview Static Controls - Ghidra Research Report

**Address(es):** `0x006153E0`, `0x0060F9A0`, `0x0060A5B0`, `0x00602490`, `0x006AE3F0`, `0x00640710`, `0x005E2EF0`, `0x005E2F60`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Skirmish dialog `0x102` static controls `0x694`, `0x468`, `0x6EC`, and `0x5A8`: owner-draw setup, text/no-text paint paths, preview paint delegation, and invalidation/update messages.  
**Non-Scope:** Skirmish flag statics `0x6DA..0x6E1`, main-menu `0x71C` except contrast, button/combo/checkbox visual rendering.  
**Confidence:** High for active control classification and paint/update paths; Medium for exact final CSF text values because CSF string contents were not runtime-read in this pass.  
**Active in YR:** Yes for standard offline Skirmish dialog `0x102`; preview drawing is conditional on live preview object `DAT_00AC1154`.

## 1. Overview

The Skirmish right-panel statics split into two behaviors. `0x694`, `0x6EC`, and `0x5A8` are subclassed Static controls that become kind-1 shell text-animation labels; they draw through `OwnerDraw_Static_006153E0` only while their animation-running byte is set. `0x468` is also subclassed, but it remains a no-text placeholder; the actual map preview and start markers are drawn by the parent Skirmish dialog `WM_PAINT` branch, using the child HWND only as a coordinate anchor.

## 2. Control Inventory

| Control | Resource/layout role | Owner-draw classification | Active in YR | Evidence |
|---|---|---|---|---|
| `0x694` | Skirmish title, resource title `GUI:SkirmishGame` | Static subclass; kind 1 text animation | Yes | PE dialog `0x102` in prior layout report; `FUN_0060F9A0`; `FUN_00602490` returns true for dialog `0x102`, id `0x694`; `FUN_0060A5B0` sets kind `1` |
| `0x468` | map preview child/static placeholder | Static subclass; remains kind 0/no text for this slice | Yes, but visible preview is parent-drawn | PE dialog `0x102`; `FUN_0060F9A0`; `FUN_006AE3F0` gets child `0x468`; `DrawStartPositions @ 0x00640710` |
| `0x6EC` | game-type text, initial title `GUI:None` | Static subclass; kind 1 text animation | Yes | `FUN_00602490` returns true for dialog `0x102`, id `0x6EC`; `FUN_005E2EF0` sends `0x4B2` to child `0x6EC` |
| `0x5A8` | scenario/map label, initial title `GUI:None` | Static subclass; kind 1 text animation | Yes | `FUN_00602490` returns true for dialog `0x102`, id `0x5A8`; `FUN_005E2F60` sends `0x4B2` to child `0x5A8` |

## 3. Shell Owner-Draw Setup

Active in YR: Yes. `FUN_00622B50` handles `WM_INITDIALOG` (`0x110`) for shell dialogs and calls `EnumChildWindows(param_1, FUN_0060F9A0, 0)` plus `FUN_0060F9A0(param_1, 0)` on the parent path. `FUN_0060F9A0` reads each child class name and maps `"Static"` to `OwnerDraw_Static_006153E0`, then installs the common subclass thunk via `SetWindowLongA(hwnd, GWL_WNDPROC, 0x00610CA0)`, records the real owner-draw function, snapshots the original text with `WM_GETTEXT` (`0x0D`), translates non-empty CSF keys through `StringTable__LoadString`, stores the copied wide text via `FUN_00623560`, and sends init message `0x497`.

Active in YR: Yes. `OwnerDraw_Static_006153E0` handles `0x497` by setting kind `piVar11[0x1C] = 0`, shadow/flags arg `piVar11[0x2B] = 0x0C`, and default color `piVar11[0x3B] = DAT_00AC18A4` (`0xFFFF`, yellow). The later child enumeration `FUN_0060A5B0` reclassifies `0x694`, `0x6EC`, and `0x5A8` to kind `1` because `FUN_00602490` returns true for those IDs on dialog `0x102`.

Active in YR: Yes. `FUN_0060A5B0` sets kind-1 text animation fields for these labels: running byte `piVar8[0x2A] = 0`, reveal count `piVar8[0x20] = 1`, interval from `FUN_00600CA0` (`0x1E` for `0x694/0x6EC/0x5A8`), step from `FUN_006015E0` (`1` for these labels), and fade/reveal range from `FUN_00601D20` (`8` for these labels). It also sets `piVar8[0x24] = -1` for no sound, except score-dialog special cases outside this scope.

## 4. Static WM_PAINT Behavior

Active in YR: Yes. `OwnerDraw_Static_006153E0` `WM_PAINT` first suppresses drawing if `piVar11[0x2F]` is nonzero or if a movie handle exists at `piVar11[0x16]`; both paths call `ValidateRect(hwnd, NULL)` and return. These guards are active infrastructure, but no evidence in this slice sets them for the three Skirmish text labels.

Active in YR: Yes. On first paint, if the per-control backing surface `piVar11[4]` is null, the static proc allocates a `BSurface` sized `client.right + 1` by `client.bottom + 1`, copies the parent/backbuffer area into it, increments `DAT_00AC48B4`, then continues. Later paints reuse that backing surface unless message `0x47`, `WM_SIZE`, `WM_MOVE`, or destroy cleanup resets it.

Active in YR: Yes. For kind `0` or kind `1`, text draws only if `piVar11[10]` is non-null and either kind is `0` or kind is `1` with running byte `piVar11[0x2A] != 0`. The three Skirmish labels are kind `1`, so initial setup alone is not enough; the text appears through the animation-start path.

Active in YR: Yes. Text alignment comes directly from static style low bits: default `0x10`, style bit `1` -> `0x11` horizontal center, style bit `2` -> `0x12` right align. Disabled style bit `0x08000000` swaps color to `DAT_00AC1CB4` (`0x9F`, dark red). Otherwise color is `piVar11[0x3B]`, initialized to yellow.

Active in YR: Yes. The draw call is `FUN_00621040(..., text, rect, color, align_flags, 0x0C, 0, reveal_count, reveal_range)`. `FUN_00621040` converts RGB to the active 16-bit display format, clips to the passed rect, vertically centers only if flag bit `0x04` is set, and dispatches to `BitFont` wrap/draw. The static label flags `0x10/0x11/0x12` do not set bit `0x04`, so these labels are top-anchored, not vertically centered.

Active in YR: Yes. During kind-1 animation, after each draw the proc computes `text_len + 1 + reveal_range`; if current reveal count is below that, it adds the configured step (`1`) and kills timer `0` once the target is reached.

## 5. Text Start And Update Messages

Active in YR: Conditional. The actual animation start message is `0x4EE`. `FUN_00622B50` handles broadcast message `0x4EC` by `EnumChildWindows(parent, FUN_0060AA60, 0)`, and `FUN_0060AA60` sends `0x4EE` to children for which `FUN_00602490` is true. `OwnerDraw_Static_006153E0` handles `0x4EE` by setting running byte `piVar11[0x2A] = 1`, resetting reveal count to `1`, starting timer `0` at `piVar11[0x21]`, and invalidating the child. `FUN_006071E0` sends `0x4EC` only in the nonzero-`DL` transition path; the common `WM_PAINT` deferred caller passes `DL=0` and sends `0x4ED`, which does not start reveal for standard `0x102`.

Active in YR: Yes. `FUN_005E2EF0` updates game-type label `0x6EC`: if its second argument is nonzero, it gets `GetDlgItem(parent, 0x6EC)`, obtains the current wide text via `FUN_007B7140()`, and sends `SendMessageA(child, 0x4B2, 0, lParam)`.

Active in YR: Yes. `FUN_005E2F60` updates scenario/map label `0x5A8`: it gets `GetDlgItem(parent, 0x5A8)` and sends `SendMessageA(child, 0x4B2, 0, 0x00A8B322)`, where `0x00A8B322` is the global map-name/scenario string buffer populated in the Skirmish setup path.

Active in YR: Yes. `OwnerDraw_Static_006153E0` handles `0x4B2`/`0x4B4` by refreshing from the screen/backbuffer into an existing backing surface and invalidating, rather than taking the full default Win32 paint path. The text-copy side is handled by shared owner-draw state setup/update infrastructure; this pass verified the message reaches the static labels and triggers invalidation, but did not fully drain the common thunk at `0x00610CA0`.

## 6. Map Preview Static `0x468`

Active in YR: Yes, conditional on preview object. `FUN_006AE3F0`, the Skirmish dialog proc, delegates `WM_PAINT` first to common shell `FUN_00622B50`; if common paint returns `0`, it checks `DAT_00AC1154 != 0`, gets child `0x468`, calls `FUN_006067A0`, and if that returns false calls `DrawStartPositions @ 0x00640710`, then validates the parent dialog.

Active in YR: Yes. `FUN_006067A0` looks up the child in owner-draw metadata and returns the per-control byte at the same record position used as the static suppress-paint byte. Thus the parent preview marker draw is skipped if the preview child is in suppress/no-draw state; standard observed path is not suppressed.

Active in YR: Yes. `DrawStartPositions @ 0x00640710` validates the parent at entry, requires a non-null preview object, looks up `GetDlgItem(parent, 0x468)`, converts that child HWND rect to shell/backbuffer coordinates through `FUN_00775690`, aspect-fits the preview surface using integer `*1000` scale math, blits the preview to `DAT_00887310`, lazily loads `STARTBUT.SHP`, then draws start markers only when `ScenarioClass+0x113C` is in `1..8`.

Active in YR: Yes. For each start marker, `DrawStartPositions` reads X/Y from `ScenarioClass+0x1140 + i*8` and `+0x1144 + i*8`, subtracts visible-map origin fields `+0x112C/+0x1130`, divides by visible-map size fields `+0x1134/+0x1138`, draws `STARTBUT.SHP` frame `0` at offset `-9,-6`, then draws numeric label `i + 1`. This is the visible preview overlay; not the `0x468` static proc.

Active in YR: Yes. On Choose Map (`0x5AA`) success in `FUN_006ACEE0`, the dialog hides, opens the choose-map flow, rebuilds/loads preview state when random-map mode applies, and calls `InvalidateRect(parent, NULL, 0)` after preview replacement. This invalidates the parent, not just child `0x468`, matching the parent-owned preview paint path.

## 7. Contrast With Main-Menu `0x71C`

Active in YR: Yes as contrast only. `0x71C` is classified by the same common helpers as an animated/static special in many shell dialogs, but it is not part of the Skirmish `0x102` text/preview set. Unlike Skirmish `0x468`, main-menu `0x71C` has no parent Skirmish preview draw branch and no map-marker semantics. No further `0x71C` investigation was performed.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Static subclass setup | verified | `FUN_0060F9A0`, `FUN_00622B50` | none |
| `OwnerDraw_Static_006153E0` text/no-draw path | verified | `0x006153E0` decompile | common thunk text-copy internals not fully drained |
| `0x694` title classification | verified | `FUN_00602490`, `FUN_0060A5B0` | runtime CSF value not read |
| `0x6EC` game-type classification/update | verified | `FUN_00602490`, `FUN_005E2EF0` | exact source text from `FUN_007B7140` not decoded |
| `0x5A8` map-label classification/update | verified | `FUN_00602490`, `FUN_005E2F60`, global `0x00A8B322` | exact buffer contents not runtime-read |
| `0x468` static no-text/preview delegation | verified | `FUN_006AE3F0`, `FUN_006067A0`, `DrawStartPositions @ 0x00640710` | preview object construction internals deferred |
| Flag statics `0x6DA..0x6E1` | not-touched | user non-scope | investigate only in separate slot |

## 9. Open Questions - Final State

[RESOLVED] OQ1 - Are `0x694`, `0x6EC`, and `0x5A8` ordinary static text controls? No; they are reclassified to kind `1` text animation by `FUN_0060A5B0` after `FUN_00602490` returns true. Evidence: `FUN_00602490`, `FUN_0060A5B0`.

[RESOLVED] OQ2 - Does `0x468` draw its own map preview from `OwnerDraw_Static_006153E0`? No; the static proc remains a placeholder/no-text path in this slice, and the parent Skirmish `WM_PAINT` calls `DrawStartPositions`. Evidence: `FUN_006AE3F0`, `0x00640710`.

[RESOLVED] OQ3 - What starts text animation? Broadcast `0x4EC` through `FUN_00622B50` enumerates children and `FUN_0060AA60` sends `0x4EE`; `OwnerDraw_Static_006153E0` starts timer `0` and invalidates. `FUN_006071E0` sends `0x4EC` only in the nonzero-`DL` transition path; the common `WM_PAINT` deferred caller passes `DL=0` and sends `0x4ED`, which does not start reveal for standard `0x102`. Evidence: `FUN_00622B50`, `FUN_0060AA60`, `OwnerDraw_Static_006153E0`, `SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md`.

[RESOLVED] OQ4 - Are text label updates active for `0x6EC` and `0x5A8`? Yes; `FUN_005E2EF0` and `FUN_005E2F60` send `0x4B2` to those child HWNDs. Evidence: `0x005E2EF0`, `0x005E2F60`.

[DEFERRED] OQ5 - Where exactly does the common subclass thunk at `0x00610CA0` copy dynamic `0x4B2` text into the record? Ghidra does not currently have that thunk as a function and creating it is a mutating action prohibited for this slot. Category: needs-readonly-function-boundary-or-runtime-debugger.

## Sources

- Ghidra decompiled: `OwnerDraw_Static_006153E0 @ 0x006153E0`, `FUN_0060F9A0`, `FUN_00622B50`, `FUN_0060A5B0`, `FUN_00602490`, `FUN_00600CA0`, `FUN_006015E0`, `FUN_00601D20`, `FUN_0060AA60`, `FUN_006071E0`, `FUN_006AE3F0`, `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_006067A0`, `DrawStartPositions @ 0x00640710`, `FUN_005E2EF0`, `FUN_005E2F60`, `FUN_00621040`, `FUN_00775690`.
- Prior docs cross-checked: `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`, `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`.
- INI files checked: none; these controls are shell/resource/CSF and runtime-dialog behavior, not INI-driven.
