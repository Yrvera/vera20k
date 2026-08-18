# Validation Modal 0x005D3490 Paint Composition - Ghidra Research Report

**Address(es):** `0x005D3490`, `0x005D36A0`, `0x00622650`, `0x00622B50`, `0x00621E90`, `0x0060A330`, `0x00609E20`, `0x00612B70`, `0x006153E0`, `0x0072AA40`, `0x0072B050`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** native visual composition and draw ordering for the ordinary Start validation modal created by `0x005D3490` when Skirmish Start validation fails.  
**Non-Scope:** exact child/dialog rectangles, Enter/Escape/default-button behavior, failure trigger/text rediscovery, successful launch, screenshot RGB capture, and generic non-Start callers except where needed to classify paint roles.  
**Confidence:** High for dialog/template selection, mode-2 background draw, OK-button owner-draw classification, asset/palette binding, and current Rust deltas; Medium for final perceived pixels because no runtime screenshot was captured.  
**Active in YR:** Yes for ordinary offline Skirmish Start validation failures; conditional branches are noted per finding.

## 0. Working Notes

- Target question: What native visual composition/draw order does the `0x005D3490` Start validation modal use?
- Non-goals: Do not extract exact child rects, keyboard/default behavior, or re-prove failure conditions/text.
- Evidence needed to mark COMPLETE: decompile plus assembly/resource evidence for dialog creation, parent paint path, owner-draw button/static paths, asset/palette names, current Rust render delta, and a Rust-facing handoff.
- Stop conditions: Stop after the ordinary `0xCE` one-button path has a closed ordered composition ledger; record optional `0x120/0x121` and screenshot/runtime questions as uncertainty.

## 1. Overview

The ordinary Skirmish Start validation failure uses the generic shell message helper `0x005D3490`, but visually it is not a simple solid panel. The helper selects RT_DIALOG `0xCE`, creates it with `CreateDialogIndirectParamA`, runs the common shell dialog setup, then relies on the common mode-2 `WM_PAINT` path to draw a clipped `PUDLGBG*.SHP` frame-0 background before child controls paint.

For standard offline shell state, the modal background branch is the no-game/menu mode: `PUDLGBGN.SHP` with `DIALOGN.PAL`. The OK control `0x5AE` is a Button-class child reclassified by `FUN_0060A330/FUN_00609E20` into owner-draw type `3`, so it draws `MNBTTN.SHP` through `MAINBTTN.PAL`, not the Skirmish `bue_*30.pcx` / `bde_*30.pcx` PCX button pieces.

## 2. Core Verified Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| Ordinary Start validation calls pass body text and `TXT_OK` but no optional third/fourth button texts, so `0x005D3490` selects dialog id `0xCE`. | `0x005D351D..0x005D353A` keeps default `ECX=0xCE` unless optional args are non-empty; Start call ranges in current contract report pass zero optional args. | Yes |
| The helper creates a Win32 RT_DIALOG resource, not an in-engine ad hoc rectangle. | `0x00622665..0x006226CB`: `FUN_004A3B40(id, RT_DIALOG=5)` then `CreateDialogIndirectParamA(hInstance, template, g_hWnd, proc, lParam)`. | Yes |
| The ordinary resource has static body `0x5B0` and Button OK `0x5AE`; exact rect ownership belongs to slot 1. | Slot-1 resource report: resource `0xCE` has static `0x5B0` and button `0x5AE`; helper writes `0x5B0` at `0x005D3573..0x005D3588` and `0x5AE` at `0x005D3592..0x005D35A9`. | Yes |
| Dialog ids `0xCE`, `0x120`, and `0x121` are in the common mode-2 shell paint allow-list. | `0x00622AAF..0x00622B1F` compares the dialog id and writes record mode `+0xB0 = 2`. | Yes for `0xCE`; conditional for optional helper variants |
| Parent `WM_PAINT` for mode `2` draws a single `PUDLGBG*.SHP` frame `0` through a DIALOG-family palette, clipped to the dialog client. | `0x00622C4D..0x00622CBF` calls `WM_PAINT_Handler @ 0x00621E90`; mode-2 branch selects SHP/palette then calls `CC_Draw_Shape` at `0x006222A8` per mode-2 report. | Yes, conditional on mode field `2` |
| In normal shell/no-game state the mode-2 background is `PUDLGBGN.SHP` plus `DIALOGN.PAL`; in-game branches use Allied/Soviet/Yuri variants. | `0x00621E90` calls `FUN_0069BBE0`; no-game branch calls `0x0072B030` and keeps `DAT_00B0FC80`; strings verified from exe pointer table: `0x00844C44 -> PUDLGBGN.SHP`. | Yes for shell/no-game; conditional otherwise |
| `0x5AE` is reclassified to owner-draw button type `3`, binding it to `MNBTTN.SHP` and `MAINBTTN.PAL`. | `FUN_00609E20` returns true for parent id `0xCE`, child `0x5AE`; `FUN_0060A330` writes record `+0xB0=3`; `OwnerDraw_Button_00612B70` type `3` calls `FUN_0072B050` and uses `DAT_00B0FACC`; `0x0072AA40` maps `0x00844C54 -> MNBTTN.SHP`, `0x00844BA8 -> MAINBTTN.PAL`. | Yes for ordinary OK button |
| Button SHP frame selection is state-driven: enabled/unpressed frame `0`, disabled frame `1`, pressed/timer frame `2`. | `OwnerDraw_Button_00612B70`: type `3` initializes frame `0`, disabled style branch sets frame `1`, pressed byte branch sets frame `2` before `CC_Draw_Shape`. | Yes |
| Static/body text and OK text are drawn after the background/control art through the common owner-draw text path. | `0x4B2` text copy report verifies record text; static owner proc `0x006153E0` invalidates and later calls `FUN_00621040`; button proc draws label after SHP art when record text exists. | Yes |
| Default Win32 erase/chrome is suppressed or bypassed for the visible shell composition. | Common proc handles `WM_ERASEBKGND (0x14)` as `1`; `WM_CTLCOLOR* 0x132..0x138` returns stock object `4`; parent body is custom mode-2 DirectDraw/SHP paint. | Yes |

## 3. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 0 | `0x005D3490 -> 0x0052FEC0` | helper entry before dialog create | current display/backbuffer copy | full current display surface | current display format | Yes | preserves/synchronizes underlying shell surface before modal |
| 1 | `0x00622650` | id `0xCE`, proc `0x005D36A0`, RT_DIALOG type `5` | Win32 dialog resource `0xCE` | resource template; exact rects in slot 1 | n/a | Yes | creates modal HWND and child controls |
| 2 | `0x00622B50 WM_INITDIALOG` + `0x0060F9A0` | non-null lParam path; subclasses children and parent | owner-draw records | child HWNDs | shell owner-draw state | Yes | enables custom static/button paint |
| 3 | `0x0060A330` | parent id `0xCE`, child `0x5AE` via `0x00609E20` | button type field `3` | OK child only | later `MAINBTTN.PAL` | Yes | classifies OK as SHP button |
| 4 | `0x00621E90` mode-2 parent paint | record mode `+0xB0 = 2` for `0xCE` | `PUDLGBGN.SHP`, frame `0` in normal shell/no-game state | `{0,0}` clipped to dialog client | `DIALOGN.PAL` via `0x0072B030` | Yes, normal shell state | modal body/background chrome |
| 5 | `OwnerDraw_Static_006153E0` | static `0x5B0`, text copied via `0x4B2` | no separate bitmap proven | static child rect/clip | `GAME.FNT` via `FUN_00621040`, shell text color | Yes | body/message text |
| 6 | `OwnerDraw_Button_00612B70` | button `0x5AE`, owner-draw type `3` | `MNBTTN.SHP`, frame `0/1/2` by state | OK child rect; exact rect in slot 1 | `MAINBTTN.PAL` via `0x0072B050` | Yes | OK button art |
| 7 | `OwnerDraw_Button_00612B70 -> FUN_00621040` | record text exists after `0x4B2` | no separate bitmap | button text rect internal to button proc | `GAME.FNT`, shell button text color | Yes | OK label text |

## 4. Asset Role Matrix

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `PUDLGBGN.SHP` | Yes | Yes in shell/no-game mode | Yes for ordinary shell modal | No | Yes | No | No | No | `0x0072AA40` pointer `0x00844C44`; mode-2 no-game branch |
| `PUDLGBGA.SHP` | Yes | Conditional | No for normal shell/no-game modal | No | Yes | No | No | Inactive unless in-game side 0 | `0x00844C48`; mode-2 side branch |
| `PUDLGBGS.SHP` | Yes | Conditional | No for normal shell/no-game modal | No | Yes | No | No | Inactive unless in-game side 1 | `0x00844C4C`; mode-2 side branch |
| `PUDLGBGY.SHP` | Yes | Conditional | No for normal shell/no-game modal | No | Yes | No | No | Inactive unless in-game non-0/1 side | `0x00844C50`; mode-2 side branch |
| `DIALOGN.PAL` | Yes | Palette input | Yes for normal shell/no-game modal | No | Palette | No | No | No | `0x0072B030`, `0x0062225F..0x0062226A` |
| `DIALOG.PAL` | Yes | Conditional palette input | No for normal shell/no-game modal | No | Palette | No | No | Inactive in no-game branch | `0x0072AFF0` side 0/1 branches |
| `DIALOGY.PAL` | Yes | Conditional palette input | No for normal shell/no-game modal | No | Palette | No | No | Inactive in no-game branch | `0x0072B010` Yuri branch |
| `MNBTTN.SHP` | Yes | Yes | Yes | No | Button art | No | No | No | `0x0072AA40` pointer `0x00844C54`; button type `3` |
| `MAINBTTN.PAL` | Yes | Palette input | Yes for OK button | No | Button palette | No | No | No | `0x0072B050`; `OwnerDraw_Button_00612B70` type `3` |
| `bue_*30.pcx` / `bde_*30.pcx` | Yes elsewhere | No for `0x5AE` | No | No | No for this control | No | No | Inactive for OK `0x5AE` | `0x0060A330` sets type `3`, bypassing button type `0` PCX path |

## 5. Current Rust Implementation Status

Current Rust implements functional modal display, but its visual composition is not native:

- `src/ui/skirmish_shell/layout.rs:35..36` uses hand-authored `360x122` constants; `compute_validation_modal_layout` at `:849` creates a centered primitive panel with guessed message and OK rects.
- `src/app_skirmish_shell_render/modals.rs:167..191` draws a solid modal panel, outline, and `push_button_30` PCX-style button.
- `src/app_skirmish_shell_render/text.rs:722..746` center-aligns body text and OK text in Rust-computed rects.
- `src/app_skirmish_shell_render/draw_order.rs:135..139` models the modal as only `ValidationModal` then `ValidationModalButton`, with no PUDLGBG/MNBTTN/palette role.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x005D3490` dialog id selection | verified | decompile plus assembly `0x005D351D..0x005D353A` | none |
| RT_DIALOG creation | verified | `0x00622665..0x006226CB` | none |
| Ordinary resource roles | verified-by-slot-1 | `VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md` | exact rects remain slot-1-owned |
| Common mode-2 parent paint | verified | `0x00622AAF..0x00622B1F`, `0x00621E90`, mode-2 report | no screenshot RGB |
| PUDLGBG/DIALOG asset names | verified | exe pointer extraction plus `0x0072AA40` and mode-2 report | none |
| OK button owner-draw type `3` | verified | `0x00609E20`, `0x0060A330`, `0x00612B70` | none |
| `MNBTTN.SHP` / `MAINBTTN.PAL` OK binding | verified | `0x0072AA40`, `0x0072B050`, exe pointer extraction | exact SHP dimensions not captured |
| Static/body text paint | touched-not-exhausted | text thunk/static reports, `0x006153E0` | exact wrapped text pixels/screenshots |
| Current Rust visual delta | verified | Rust file scans listed above | implementation work |
| Keyboard/default dismissal | deferred | slot 3 owns it | follow slot 3 |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the ordinary modal a native RT_DIALOG resource? -> Yes, id `0xCE` is loaded as RT_DIALOG type `5` and created with `CreateDialogIndirectParamA`.` (evidence: `0x00622665..0x006226CB`)
- `[RESOLVED] OQ-02 - Is the visible body a solid/generic panel? -> No; parent paint uses common mode-2 SHP composition.` (evidence: `0x00622AAF..0x00622B1F`, `0x00621E90`)
- `[RESOLVED] OQ-03 - Which background asset is active in the normal shell state? -> `PUDLGBGN.SHP` with `DIALOGN.PAL`.` (evidence: `0x0062225F..0x0062226A`, pointer `0x00844C44`)
- `[RESOLVED] OQ-04 - Is OK `0x5AE` a PCX `bue/bde` button? -> No; it is owner-draw type `3` using `MNBTTN.SHP` and `MAINBTTN.PAL`.` (evidence: `0x00609E20`, `0x0060A330`, `0x00612B70`, `0x0072AA40`)
- `[RESOLVED] OQ-05 - What draws text? -> `0x4B2` copies text into owner records; static/button owner procs draw labels through `FUN_00621040`.` (evidence: text thunk report; `0x006153E0`; `0x00612B70`)
- `[RESOLVED] OQ-06 - Is this path TS-only? -> No; Start validation reaches `0x005D3490` from standard offline YR Skirmish, and all paint helpers are common shell paths active in YR.` (evidence: current contract report; `0x006ACEE0`; `0x005D3490`)
- `[DEFERRED] OQ-07 - Exact RGB pixels after DirectDraw conversion and clipping.` (category: needs-runtime-debugger; reason: no retail screenshot/pixel capture in this slot; next-step-if-pursued: capture native modal at 800x600 and compare PUDLGBGN/MNBTTN decoded output)
- `[DEFERRED] OQ-08 - Exact child rectangles and DLU conversion.` (category: out-of-scope; reason: slot 1 owns rect extraction; next-step-if-pursued: consume slot-1 final rects)
- `[DEFERRED] OQ-09 - Enter/Escape/default-button behavior.` (category: out-of-scope; reason: slot 3 owns keyboard/default behavior; next-step-if-pursued: consume slot-3 report)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary Start validation modal body is RT_DIALOG `0xCE` with mode-2 `PUDLGBGN.SHP`/`DIALOGN.PAL` background in shell/no-game state | `0x005D3490`; `0x00622650`; `0x00621E90`; pointer `0x00844C44` | mismatch: solid panel/outline | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/modals.rs`, chrome/asset atlas | Add a native validation-modal background role using the resource-derived dialog size and clipped PUDLGBGN frame-0 with DIALOGN palette semantics | no-opponent Start failure at 800x600 shows native SHP-backed modal body, not a flat solid panel | Do not tune a flat color to look close; proposed test `skirmish_validation_modal_uses_pudlgbgn_dialogn_background`; risk: needs screenshot/asset decode fixture |
| OK button `0x5AE` uses owner-draw type `3`: `MNBTTN.SHP` plus `MAINBTTN.PAL`, frame `0/1/2` by state | `0x00609E20`; `0x0060A330`; `0x00612B70`; `0x0072AA40`; `0x0072B050` | mismatch: Rust calls `push_button_30` PCX path | `src/app_skirmish_shell_render/modals.rs`, `src/render/skirmish_shell_chrome.rs` | Load/pack `MNBTTN.SHP` and `MAINBTTN.PAL` for validation OK button; select normal/pressed/disabled frames | click and hold OK on validation modal changes to native pressed frame and releases to dismiss | Do not use `bue_*30.pcx`/`bde_*30.pcx` for this OK button; proposed test `skirmish_validation_ok_uses_mnbttn_frames`; risk: exact frame dimensions not captured here |
| Body text static `0x5B0` and OK label paint after modal background/button art through common shell text path | `0x005D3573..0x005D35A9`; text thunk report; `0x006153E0`; `0x00612B70` | partial mismatch: Rust centers body text in guessed rect and draws OK label over PCX button | `src/app_skirmish_shell_render/text.rs`, `src/ui/skirmish_shell/layout.rs` | Use slot-1 resource rects and native static/button text alignment/clip semantics for body and OK label | capacity failure wraps/clips body text in the native `0x5B0` static area and OK label sits inside `0x5AE` | Do not globally center/wrap by guessed panel dimensions; proposed test `skirmish_validation_modal_text_uses_resource_control_roles`; risk: exact wrapping requires screenshot or BitFont parity |

## 9. Negative Facts / Do Not Do

- Do not render the ordinary validation modal as a solid color rectangle with bevel only. Active in YR: Yes; evidence `0x00621E90` mode-2 SHP draw for `0xCE`.
- Do not use Skirmish 30-family PCX button art for OK `0x5AE`. Active in YR: Yes; evidence `0x00609E20 -> 0x0060A330` sets owner-draw type `3`, which uses `MNBTTN.SHP`/`MAINBTTN.PAL`.
- Do not treat `0x5AE` as a second static text field. Active in YR: Yes; evidence slot-1 resource report plus `0x005D36A0` command mapping.
- Do not add controls `2` or `0x5AF` to ordinary Start validation failures. Active in YR: Yes; evidence ordinary helper path selects `0xCE`, while `0x120/0x121` require optional button text.
- Do not assume final modal pixels are screenshot-proven. Evidence here is binary paint path plus asset names; final RGB capture remains deferred.

## 10. Stale Docs / Replacement Wording

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`: replace "Modal helper `0x005D3490` writes message/body text to child `0x5B0`, second text to child `0x5AE`, optional button text to controls `2` and `0x5AF` only when non-empty" with: "Modal helper `0x005D3490` selects RT_DIALOG `0xCE/0x120/0x121` from optional button-text presence. Ordinary Start validation uses `0xCE`: static `0x5B0` receives body text and owner-draw button `0x5AE` receives `TXT_OK`; optional button controls `2` and `0x5AF` exist only in `0x120/0x121`."
- `docs/contracts/2026-05-23-skirmish-ui-shell-implementation-contract.md`: replace the validation-modal visual implication "render helpers for `validation_modal` exist" with: "current render helpers are functional but visually wrong for native parity: native `0xCE` uses a mode-2 `PUDLGBGN.SHP`/`DIALOGN.PAL` background and `MNBTTN.SHP`/`MAINBTTN.PAL` OK button, not a flat panel with `push_button_30`."

## 11. Remaining Uncertainty

- Exact final RGB/pixel screenshot comparison remains unperformed.
- Exact child/control pixel rectangles are owned by slot 1 and should be consumed from its report.
- Enter/Escape/default-button behavior is owned by slot 3.
- Exact `MNBTTN.SHP` frame dimensions were not dumped here; only binding and frame indices were verified.

## Sources

- Ghidra read-only decompile/assembly: `0x005D3490`, `0x005D36A0`, `0x00622650`, `0x00622B50`, `0x00621E90`, `0x0060A330`, `0x00609E20`, `0x00612B70`, `0x006153E0`, `0x0072AA40`, `0x0072B050`.
- Resource/asset evidence: slot-1 report `VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md`; local PE pointer extraction from `gamemd.exe` for `PUDLGBGN/A/S/Y.SHP`, `MNBTTN.SHP`, `DIALOGN.PAL`, `MAINBTTN.PAL`.
- Prior docs: `LOADING_SCREEN_WM_PAINT_MODE2_COMPOSITION_GHIDRA_REPORT.md`, `DIALOG_PALETTE_STARTUP_0072AA40_GHIDRA_REPORT.md`, `PUDLGBG_LOADING_SCREEN_SHP_LIFECYCLE_GHIDRA_REPORT.md`, `SHELL_SUBCLASS_THUNK_00610CA0_TEXT_UPDATE_PLUMBING_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`.
- Current Rust scan: `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/modals.rs`, `src/app_skirmish_shell_render/text.rs`, `src/app_skirmish_shell_render/draw_order.rs`.

**Status:** COMPLETE
