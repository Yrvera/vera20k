# Skirmish Validation Modal Native Pixel Rects - Ghidra Research Report

**Address(es):** `0x005D3490`, `0x00622650`, `0x00622B50`, `0x00777060`, `0x00777080`, `0x0060C7D0`, RT_DIALOG `0xCE` resource bytes at file offset `0x4F9A3C`  
**Investigation Mode:** exhaustive-slice for static binary/resource geometry; final screenshot-grade pixels are partial because runtime `CreateDialogIndirectParamA`/font metrics are required.  
**Claimed Scope:** ordinary offline Skirmish Start validation popup created through `FUN_005D3490 -> RT_DIALOG 0xCE`; static template rects, native centering formula, and what can/cannot be proven for 800x600 and 1024x768 without runtime capture.  
**Non-Scope:** unrelated dialogs, optional `0x120/0x121` helper variants except as negative context, full `PUDLGBG*` inventory, OK button state frames, text wrapping internals beyond native control rectangles, and Rust code changes.  
**Confidence:** High for dialog activation, resource DLU rects, and native centering formula; Medium/Partial for final pixel rects because the exact `GetClientRect` size after Win32 dialog creation was not captured.  
**Active in YR:** Yes for ordinary offline Skirmish Start validation failures.

## 0. Working Notes

- Target question: What final native on-screen pixel geometry does the ordinary offline Skirmish Start validation popup use, and which parts are binary/resource-proven versus runtime-only?
- Non-goals: Do not inventory unrelated mode-2 dialogs; do not re-investigate Start validation activation except where needed to prove this exact popup; do not modify Rust or Ghidra state.
- Evidence needed to mark COMPLETE: resource bytes for `0xCE`; decompile plus assembly/disassembly ranges proving `CreateDialogIndirectParamA`, `WM_INITDIALOG`, and centering; final `GetClientRect`/`GetWindowRect` or screenshot/debugger capture at 800x600 and 1024x768.
- Stop conditions: Stop after the `0xCE` static template, RA2 centering formula, current Rust delta, and runtime capture boundary are documented. Downgrade final pixel certainty if no native screenshot/debugger capture exists.
- Result: Static geometry and centering are verified. Exact screenshot-grade pixel rects are deferred to runtime capture.

## 1. Overview

The validation modal is a real native Win32 child dialog, not a freehand engine rectangle. `FUN_005D3490` creates RT_DIALOG `0xCE` through `CreateDialogIndirectParamA`; the dialog template provides a `300x200` dialog-unit client template with body static `0x5B0` and OK button `0x5AE`.

The missing static piece in older reports was centering. This report verifies that RA2/YR centers the created child dialog during `WM_INITDIALOG` by calling `FUN_00777060 -> CenterChildWindow`, before `ShowWindow`. The exact final pixels still depend on the client width/height Windows produced from the dialog units and `8pt MS Sans Serif`, so a native `GetClientRect`/screenshot capture is still required for screenshot-grade parity.

## 2. Static Resource Geometry

Resource parse was rechecked directly from `<ra2-install>/gamemd.exe` at file offset `0x4F9A3C`.

| Item | Value | Evidence | Active in YR |
|---|---:|---|---|
| Resource id | `0xCE` | `0x005D3490` ordinary optional-text path plus RT_DIALOG load in `0x00622650` | Yes |
| Template style | `0x40000040` | resource bytes at `0x4F9A3C` | Yes |
| Extended style | `0x00000000` | resource bytes | Yes |
| Child count | `2` | resource bytes | Yes |
| Template rect | `x=0 y=0 cx=300 cy=200` dialog units | resource bytes | Yes |
| Font | `8pt MS Sans Serif` | resource bytes | Yes |
| Body control | id `0x5B0`, static ordinal `0x82`, style `0x50000000`, rect `40,40,220,50` DLUs, title `GUI:Blank` | resource bytes plus text write at `0x005D3573..0x005D3588` | Yes |
| OK control | id `0x5AE`, button ordinal `0x80`, style `0x5000000B`, rect `207,175,83,15` DLUs, title `GUI:OK` | resource bytes plus text write at `0x005D3592..0x005D35A9` | Yes |

Tiny details that matter:

- `x=0,y=0` in the resource is not the final screen position. It is overwritten by RA2's centering helper after the dialog exists.
- The template itself has no caption text and no extra item data for either control.
- The one-button ordinary modal has no child controls `2` or `0x5AF`; those belong to optional helper variants outside this target.
- The template dimensions are dialog units, not pixels. Treating `300x200` as pixels is wrong.

## 3. Creation And Centering Path

| Step | Function / address | Behavior | Evidence | Active in YR |
|---:|---|---|---|---|
| 1 | `0x005D3490` | Ordinary two-text call creates dialog through `FUN_00622650` and later calls `FUN_00622800` to show it. | decompile `0x005D3490`; prior Start caller proof | Yes |
| 2 | `0x00622650` | Loads RT_DIALOG template via `FUN_004A3B40`, passes `g_hWnd` as parent, calls `CreateDialogIndirectParamA(hInstance, template, g_hWnd, proc, local_8)`. | decompile `0x00622650`; disassembly range `0x00622650..0x0062270F` checked | Yes |
| 3 | `0x00622B50` | On `WM_INITDIALOG (0x110)` with non-null lParam, subclasses/setup children and parent, then calls `FUN_00777060(param_1)`. | decompile `0x00622B50`; disassembly range `0x00622F80..0x0062309F` checked | Yes |
| 4 | `0x00777060` | Reads parent with `GetWindowLongA(hwnd, GWL_HWNDPARENT=-8)` and calls `CenterChildWindow` if parent exists. | decompile `0x00777060`; disassembly range `0x00777060..0x0077714F` checked | Yes |
| 5 | `0x00777080` | Computes centered child position from parent/client and child/client sizes; calls `SetWindowPos(hwnd, 0, X, Y, -1, -1, 5)`. | decompile `CenterChildWindow @ 0x00777080`; disassembly range `0x00777060..0x0077714F` checked | Yes |
| 6 | `0x00622800` | Only after creation/init/centering, `ShowWindow(hwnd, 1)` and `SetForegroundWindow(hwnd)` make the dialog visible. | decompile `0x00622800` | Yes |

`SetWindowPos` uses flag `5`, which is `SWP_NOSIZE | SWP_NOZORDER`. The centering helper moves the dialog but does not resize it. Therefore the final width/height must be whatever Windows produced from the RT_DIALOG template before the helper ran.

## 4. Centering Formula

`CenterChildWindow @ 0x00777080` computes the parent size and child size from client rectangles. If the parent HWND is `g_hWnd`, it overwrites the parent client size with `g_ScreenWidth` and `g_ScreenHeight`.

For the target modal, `0x00622650` creates the dialog with parent `g_hWnd`, so the active centering formula is:

```text
parent_w = g_ScreenWidth
parent_h = g_ScreenHeight
child_w  = GetClientRect(dialog).right
child_h  = GetClientRect(dialog).bottom

x = max(0, ((parent_w - child_w) + 1) / 2)
y = max(0, ((parent_h - child_h) + 1) / 2)

SetWindowPos(dialog, 0, x, y, -1, -1, SWP_NOSIZE | SWP_NOZORDER)
```

Active in YR: Yes. Evidence: `0x00622650` parent argument is `g_hWnd`; `0x00622B50` calls `0x00777060`; `0x00777060` gates on nonzero parent; `0x00777080` implements the formula above.

Tiny details that matter:

- The `+1` before integer division changes odd-size centering by one pixel.
- Negative coordinates are clamped to `0`.
- The child is centered by client size, not by asset size and not by the original DLU `x/y`.
- The helper does not use `800x600` as a hardcoded base. It uses the current `g_ScreenWidth/g_ScreenHeight`; for a 1024x768 game window, the same formula centers in 1024x768.

## 5. What Is Statically Possible For 800x600 / 1024x768

The following are proven:

- Native dialog local template: `300x200` DLUs.
- Native control local template rects: body `40,40,220,50` DLUs; OK `207,175,83,15` DLUs.
- Native screen-position formula after Windows creates the dialog.

The following are not proven without runtime capture:

- Exact `child_w/child_h` returned by `GetClientRect(dialog)` after `CreateDialogIndirectParamA`.
- Exact pixel conversion of every DLU rect under the running OS font metrics.
- Whether the visible `PUDLGBG*` frame is clipped by the child client rect. The decoded SHP frame is larger than the common `6x13` DLU-conversion candidate, so this is not a safe assumption.

### Candidate Only: If Runtime Dialog Base Units Are 6x13

This table is useful as a likely Win32 candidate, not as a verified final pixel capture. It assumes `MapDialogRect`-style `MulDiv` conversion with base units `x=6`, `y=13`.

| Rect | Local pixels | 800x600 absolute | 1024x768 absolute | Confidence |
|---|---:|---:|---:|---|
| Dialog client | `450x325` | `x=175 y=138 w=450 h=325` | `x=287 y=222 w=450 h=325` | Medium; formula proven, child size assumed |
| Body `0x5B0` | `x=60 y=65 w=330 h=81` | `x=235 y=203 w=330 h=81` | `x=347 y=287 w=330 h=81` | Medium; DLU conversion assumed |
| OK `0x5AE` | `x=311 y=284 w=125 h=24` | `x=486 y=422 w=125 h=24` | `x=598 y=506 w=125 h=24` | Medium; DLU conversion assumed |

Do not promote this candidate table to a parity contract until a native `GetClientRect`/`GetWindowRect` or screenshot confirms the actual child size and control rects.

## 6. Visual/UI Composition Ledger

This ledger is scoped to geometry and positioning, not full paint inventory.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `0x00622650` | ordinary `0xCE` selected by `0x005D3490` | RT_DIALOG `0xCE` | template `0,0,300,200` DLUs | n/a | Yes | creates geometry owner |
| 2 | Windows `CreateDialogIndirectParamA` | parent `g_hWnd`, font `8pt MS Sans Serif` | native Win32 dialog/control windows | runtime DLU-to-pixel conversion | n/a | Yes | produces client/control pixel rects |
| 3 | `0x00622B50 WM_INITDIALOG` | non-null lParam from `0x00622650` | ownerdraw records | child/parent setup | n/a | Yes | prepares custom paint and centering |
| 4 | `0x00777060 -> 0x00777080` | parent exists from `GWL_HWNDPARENT` | no asset | `x=max(0,((screen_w-child_w)+1)/2)`, `y=max(0,((screen_h-child_h)+1)/2)` | n/a | Yes | final dialog screen position |
| 5 | `0x00621E90` mode-2 paint | `0x0060C7D0` marks dialog `0xCE` mode 2 | `PUDLGBGN.SHP` frame 0 in shell/no-game native path | local `0,0` clipped to dialog client | `DIALOGN.PAL` | Yes natively; Rust currently overrides to Soviet | background fill subject to client clipping |
| 6 | child ownerdraw/static paint | child controls from RT_DIALOG | body text and `MNBTTN` OK art | child rects from DLU conversion | common shell text / `MAINBTTN.PAL` | Yes | visible body and OK |

## 7. Asset Role Matrix

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `PUDLGBGN.SHP` | Yes | Yes natively | Yes for neutral shell/no-game validation | No | Yes | No | No | No | paint report `0x00621E90`; active shell branch |
| `PUDLGBGS.SHP` | Yes | Conditional | No for native shell/no-game validation; yes only as current Rust override | No | Yes | No | No | Native inactive for this target | paint report side-1 branch; current Rust scan |
| `DIALOGN.PAL` | Yes | Palette input | Yes natively | No | Palette | No | No | No | paint report `0x00621E90` |
| `DIALOG.PAL` | Yes | Conditional palette input | No natively for shell/no-game validation; yes for current Soviet override | No | Palette | No | No | Native inactive for this target | paint report side branch; current Rust scan |
| `MNBTTN.SHP` | Yes | Yes for OK | Yes | No | Button art | No | No | No | prior `MNBTTN_MAINBTTN` report; Rust scan |
| `MAINBTTN.PAL` | Yes | Palette input | Yes | No | Palette | No | No | No | prior `MNBTTN_MAINBTTN` report; Rust scan |

## 8. Current Rust Implementation Status

Current Rust has moved closer to native control roles, but its final geometry remains approximate:

- `src/ui/skirmish_shell/layout.rs` currently defines `VALIDATION_MODAL_W=451`, `VALIDATION_MODAL_H=326`.
- `compute_validation_modal_layout` uses `dlu_rect(40,40,220,50)` and `dlu_rect(207,175,83,15)` for body/OK local rects.
- `dlu_rect` uses `BASE_X=6`, `BASE_Y=13` and a custom round-to-nearest helper.
- `centered_shell_dialog` centers in an 800x600 shell base plus screen offset, but uses `(base - w) / 2` without the native `+1` term found in `CenterChildWindow`.
- `src/app_skirmish_shell_render/modals.rs` currently draws `validation_modal_background_pudlgbgs`, which is the user-requested Soviet override, not the native neutral shell/no-game asset.

Current Rust delta:

- Geometry: partially aligned to resource control DLUs, but final dialog size and centering differ from the binary-proven formula unless runtime evidence proves `451x326` and the no-`+1` centering happen to match.
- Theme: intentionally non-native for this target because Soviet background is a requested override.
- Runtime verification: missing native screenshot/debugger capture for `GetClientRect`, `GetWindowRect`, and child rects.

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes / scope | verified | section 0 | none |
| RT_DIALOG `0xCE` resource bytes | verified | direct binary parse at file offset `0x4F9A3C` | none for DLUs |
| `0x005D3490` ordinary helper path | verified | existing activation report plus decompile | none for target activation |
| `0x00622650` dialog creation and parent HWND | verified | decompile; disassembly range `0x00622650..0x0062270F` | none |
| `0x00622B50` init path reaches centering | verified | decompile; disassembly range `0x00622F80..0x0062309F` | none |
| `0x00777060/0x00777080` centering formula | verified | decompile; disassembly range `0x00777060..0x0077714F` | none |
| `0x0060C7D0` mode-2 mark for `0xCE` | verified | decompile; disassembly range `0x0060C7D0..0x0060C8EF` | full paint belongs slot 2 |
| 800x600 final pixel rect | touched-not-exhausted | formula proven; candidate table assumes `6x13` base units | runtime `GetClientRect`/screenshot |
| 1024x768 final pixel rect | touched-not-exhausted | formula proven; candidate table assumes `6x13` base units | runtime `GetClientRect`/screenshot |
| Current Rust geometry delta | verified | `rg` and file reads in `src/ui/skirmish_shell/layout.rs`, render files | no code changes in this slot |

## 10. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the ordinary validation popup definitely RT_DIALOG 0xCE? -> Yes; ordinary Start failures call `0x005D3490` with no optional texts, selecting `0xCE`.` (evidence: activation report; `0x005D3490`; resource parse)
- `[RESOLVED] OQ-02 - What is the static dialog template size? -> `300x200` dialog units, `8pt MS Sans Serif`.` (evidence: resource bytes at file offset `0x4F9A3C`)
- `[RESOLVED] OQ-03 - What are native child DLU rects? -> body `0x5B0` at `40,40,220,50`; OK `0x5AE` at `207,175,83,15`.` (evidence: resource bytes at file offset `0x4F9A3C`)
- `[RESOLVED] OQ-04 - Does the engine center the dialog, or does resource `0,0` place it at top-left? -> It centers the child dialog during init through `0x00777060 -> 0x00777080`.` (evidence: `0x00622B50`, `0x00777060`, `0x00777080`)
- `[RESOLVED] OQ-05 - What exact centering formula is binary-proven? -> `max(0,((parent_dim-child_dim)+1)/2)` for x/y, then `SetWindowPos(..., flags=5)` with no resize.` (evidence: `CenterChildWindow @ 0x00777080`)
- `[RESOLVED] OQ-06 - Does centering use 800x600 constants? -> No; if parent is `g_hWnd`, it uses `g_ScreenWidth/g_ScreenHeight`.` (evidence: `0x00777080`)
- `[RESOLVED] OQ-07 - Is the parent `g_hWnd` for this target? -> Yes; `0x00622650` passes `g_hWnd` into `CreateDialogIndirectParamA`.` (evidence: `0x00622650`)
- `[RESOLVED] OQ-08 - Does the RA2 helper resize the dialog after creation? -> No resize was found on the target path; `SetWindowPos` uses `SWP_NOSIZE` and `ShowWindow` only makes it visible.` (evidence: `0x00777080`, `0x00622800`)
- `[RESOLVED] OQ-09 - Is `PUDLGBGS` native for this exact shell popup? -> No; native shell/no-game branch is neutral, while current Rust uses Soviet by request.` (evidence: paint report `0x00621E90`; Rust scan)
- `[DEFERRED] OQ-10 - What exact `GetClientRect(dialog)` width/height does Windows produce for `0xCE`?` (category: needs-runtime-debugger; reason: `CreateDialogIndirectParamA` performs font/DLU conversion at runtime; next-step-if-pursued: instrument native `GetClientRect` immediately after `CreateDialogIndirectParamA` or capture screenshot)
- `[DEFERRED] OQ-11 - What exact absolute child pixel rects are returned after creation?` (category: needs-runtime-debugger; reason: child rects use same runtime DLU conversion and parent move; next-step-if-pursued: native `GetWindowRect`/`ScreenToClient` for `0x5B0` and `0x5AE`)
- `[DEFERRED] OQ-12 - Is `PUDLGBGN` frame 0 clipped by the dialog client rect?` (category: needs-runtime-debugger; reason: decoded SHP dimensions do not by themselves prove created client size; next-step-if-pursued: screenshot/pixel compare or native client-size capture)
- `[DEFERRED] OQ-13 - Exact OK button art-to-control clipping and pressed offset.` (category: out-of-scope; reason: slot 3 owns state frames; next-step-if-pursued: consume OK button state-frame report)

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native `0xCE` dialog is centered by `CenterChildWindow` after Win32 creates it: `x=max(0,((screen_w-child_w)+1)/2)`, `y=max(0,((screen_h-child_h)+1)/2)`, no resize | `0x00622650`, `0x00622B50`, `0x00777060`, `0x00777080` | mismatch risk: Rust centers with `(base-w)/2` and fixed `451x326`; no native runtime capture | `src/ui/skirmish_shell/layout.rs::compute_validation_modal_layout` | preserve native DLU child rects but base final dialog position on captured/proven client size and native `+1` centering formula | invalid Start at 800x600 and 1024x768 places modal at native top-left for the captured client size | Do not treat resource `x=0,y=0` as top-left; do not drop the `+1` centering term without screenshot proof |
| Resource `0xCE` body and OK local DLU rects are `40,40,220,50` and `207,175,83,15` | resource bytes at `0x4F9A3C`; writes at `0x005D3573..0x005D35A9` | mostly implemented: Rust uses the same DLU rects with `BASE_X=6`, `BASE_Y=13`; exact conversion still unproven | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/text.rs`, `src/app.rs` hit tests | continue deriving body/OK positions from the resource rects, but verify base units against native capture | body text and OK hit area align with native child windows | Do not tune body/OK rects by eye while leaving a stale DLU model |
| Native geometry uses the child client size produced by Windows, not decoded SHP dimensions | `0x00777080` uses `GetClientRect(dialog)` and `SetWindowPos(..., SWP_NOSIZE)`; paint report says background draws clipped to client | Rust uses fixed `451x326` while decoded `PUDLGBG*` frame is larger; final native clipping unknown | layout plus modal background draw path | capture native client size before deciding whether to use DLU-converted size, SHP size, or a deliberate override | screenshot/pixel test shows background edges and OK rect match native | Do not infer dialog size from `PUDLGBGN/PUDLGBGS` frame dimensions alone |
| Current Soviet background is not native for this exact shell/no-game popup, even though geometry path is still the same dialog | paint report `0x00621E90`; current Rust scan | intentional user-requested override | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render/modals.rs` | document and gate it as an override if parity mode is needed later | neutral parity mode shows `PUDLGBGN`; user-style mode can show `PUDLGBGS` | Do not call Soviet background parity-correct for ordinary offline Skirmish validation |

## 12. Sources

- Ghidra read-only decompile: `0x005D3490`, `0x00622650`, `0x00622B50`, `0x00622800`, `0x0060C7D0`, `0x00777060`, `0x00777080`.
- Ghidra read-only disassembly ranges checked: `0x00622650..0x0062270F`, `0x00622F80..0x0062309F`, `0x0060C7D0..0x0060C8EF`, `0x00777060..0x0077714F`.
- Resource bytes parsed directly from `<ra2-install>/gamemd.exe` at file offset `0x4F9A3C`.
- Prior docs read:
  - `docs/research/skirmish-ui/VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_ACTIVATION_RECHECK_GHIDRA_REPORT.md`
- Current Rust surfaces scanned: `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/modals.rs`, `src/app_skirmish_shell_render/chrome.rs`, `src/app_skirmish_shell_render/text.rs`, `src/render/skirmish_shell_chrome.rs`, `src/app.rs`.
