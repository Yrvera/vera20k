# Current Rust Validation Modal Visual Delta After Soviet Override - Rust/Doc Delta Report

**Date:** 2026-05-24  
**Investigation Mode:** coverage-map delta scan  
**Claimed Scope:** current Rust validation-modal visual path after the user-requested Soviet background override, compared against the latest verified Skirmish Start validation modal reports.  
**Non-Scope:** fresh Ghidra decompilation, native screenshot capture, global `PUDLGBG*` dialog inventory, Choose Map, gameplay launch/spawn, or Rust edits.  
**Confidence:** High for current Rust/source deltas and documented binary target facts; Medium for final perceived pixel deltas because native screenshot comparison is still absent.  
**Active in YR:** Yes for ordinary offline Skirmish Start validation failures; the Soviet background on this shell popup is an intentional Rust override, not native YR behavior for this target.

## 0. Working Notes

- Target question: What does current Rust now match, intentionally override, still mismatch, or leave untested for the Skirmish Start validation modal visual path?
- Non-goals: Do not edit Rust, do not run broad gameplay review, do not re-investigate all dialogs, and do not prove native pixels beyond the existing reports.
- Evidence needed to mark COMPLETE: verified docs for native target behavior plus focused current Rust scan covering asset loading, modal instance construction, layout, text draw, input state, and tests.
- Stop conditions: Stop after classifying current Rust into `matches native`, `intentional Soviet override`, `mismatched`, and `untested`, with a concrete implementation handoff.

## 1. Overview

Current Rust has moved meaningfully closer to the verified native modal path for the OK button and resource-style layout. It now loads `MNBTTN.SHP` through `MAINBTTN.PAL`, selects frame `0` for normal and frame `2` for pressed, and uses the resource-derived `0x5B0` and `0x5AE` dialog-unit rectangles for message and OK hit/text regions.

The background is deliberately not native parity for ordinary offline Skirmish validation. Verified docs say the native shell/no-game branch uses `PUDLGBGN.SHP + DIALOGN.PAL`; current Rust loads and draws `PUDLGBGS.SHP + DIALOG.PAL` because the user requested the Soviet theme. That is valid as a style override, but should stay labeled as an override.

## 2. Native Target From Verified Reports

| Native fact | Evidence | Active in YR |
|---|---|---|
| Invalid offline Skirmish Start failures call the generic message helper and select RT_DIALOG `0xCE` for ordinary one-button failures. | `SKIRMISH_START_VALIDATION_MODAL_ACTIVATION_RECHECK_GHIDRA_REPORT.md`; `VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md` | Yes |
| Resource `0xCE` has dialog-unit rect `300x200`, body static `0x5B0` at `40,40,220,50`, and OK button `0x5AE` at `207,175,83,15`. | `VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md` | Yes |
| Native shell/no-game mode-2 paint draws `PUDLGBGN.SHP` frame `0` with `DIALOGN.PAL`; side-themed `PUDLGBGA/S/Y` branches are in-game conditional variants. | `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md`; activation recheck report | Yes for neutral; conditional for side variants |
| OK control `0x5AE` is owner-draw type `3`, using `MNBTTN.SHP + MAINBTTN.PAL`; normal and pressed paths use frames `0` and `2`. | `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md`; `MNBTTN_MAINBTTN_MODAL_BUTTON_ART_GHIDRA_REPORT.md` | Yes |
| Frame `1`, final RGB pixels, final Win32 DLU pixel conversion, and exact text wrapping still need runtime/screenshot confirmation. | same reports' deferred/open sections | Deferred |

## 3. Current Rust Evidence

| Surface | Current Rust behavior | Source evidence |
|---|---|---|
| Modal creation on launch validation failure | `StartGame` validation errors are mapped to `SkirmishValidationModalState` with CSF-backed message and `TXT_OK`. | `src/app.rs:662`, `src/app.rs:705` |
| Modal blocking / dismissal | Validation modal blocks parent mouse path, tracks OK pressed state, dismisses on OK mouse-up and Enter/NumpadEnter/Escape. | `src/app.rs:1159`, `src/app.rs:1178`, `src/app.rs:1195`, `src/app.rs:1557`; `src/ui/skirmish_shell/state/hit_test.rs:47` |
| Resource-style layout | `VALIDATION_MODAL_W/H` are `451x326`; message and OK rects are derived from `dlu_rect(40,40,220,50)` and `dlu_rect(207,175,83,15)`. | `src/ui/skirmish_shell/layout.rs:35`, `src/ui/skirmish_shell/layout.rs:234`, `src/ui/skirmish_shell/layout.rs:849` |
| Soviet background override | Atlas loads `PUDLGBGS.SHP` frame `0` using `DIALOG.PAL`; modal renderer draws `validation_modal_background_pudlgbgs`. | `src/render/skirmish_shell_chrome.rs:117`, `src/render/skirmish_shell_chrome.rs:189`, `src/render/skirmish_shell_chrome.rs:352`; `src/app_skirmish_shell_render/modals.rs:173` |
| Native OK art | Atlas loads `MNBTTN.SHP` frames `0/1/2` using `MAINBTTN.PAL`; renderer uses the modal-button helper instead of `push_button_30` for validation OK. | `src/render/skirmish_shell_chrome.rs:116`, `src/render/skirmish_shell_chrome.rs:169`, `src/render/skirmish_shell_chrome.rs:359`; `src/app_skirmish_shell_render/modals.rs:197` |
| OK frame selection | `pressed=false -> frame 0`; `pressed=true -> frame 2`; frame `1` is loaded but not currently selected. | `src/app_skirmish_shell_render/chrome.rs:321`, `src/app_skirmish_shell_render/chrome.rs:337` |
| OK fallback | If MNBTTN entry is missing, helper falls back to generic `push_button_30`. | `src/app_skirmish_shell_render/chrome.rs:347` |
| Text drawing | Message and OK text are centered in current Rust rects after modal sprite construction. | `src/app_skirmish_shell_render/text.rs:726`; `src/app_skirmish_shell_render.rs:441` |
| Draw role model | Semantic order models only `ValidationModal` and `ValidationModalButton`; text is outside that role vector. | `src/app_skirmish_shell_render/draw_order.rs:135`; `src/app_skirmish_shell_render.rs:430` |

## 4. What Currently Matches Native Parity

| Area | Match status | Evidence |
|---|---|---|
| One-button ordinary validation concept | Matches the verified native `0xCE` shape at a functional level: one body message and one OK button. | Rust `SkirmishValidationModalState` only stores `message`, `ok_button`, and pressed state; native report says ordinary `0xCE` has static `0x5B0` and OK `0x5AE`. |
| CSF-backed OK/body text for scoped errors | Matches the documented native text families for map capacity, too-few players, same-team, and OK. | `src/app.rs:709` through `src/app.rs:734`; activation/resource reports. |
| Resource-derived message and OK rect intent | Current Rust now uses the verified `0x5B0` and `0x5AE` dialog-unit rect constants rather than arbitrary centered panel guesses. | `src/ui/skirmish_shell/layout.rs:857`; resource report. |
| OK button native art | Normal/pressed OK now uses `MNBTTN.SHP + MAINBTTN.PAL`, the verified owner-draw type-3 art path. | `src/render/skirmish_shell_chrome.rs:169`; `src/app_skirmish_shell_render/chrome.rs:321`; MNBTTN report. |
| Keyboard dismissal | Enter/NumpadEnter/Escape dismiss the modal before parent Escape closes the shell, matching the prior keyboard report's implementation target. | `src/app.rs:1159`; `src/app.rs:1557`; keyboard report referenced from prior swarm claims. |

## 5. Intentional User-Requested Override

| Area | Current Rust | Native parity target | Classification |
|---|---|---|---|
| Modal background theme | `PUDLGBGS.SHP + DIALOG.PAL` | `PUDLGBGN.SHP + DIALOGN.PAL` for ordinary shell/no-game validation | Intentional Soviet style override |

This is the main non-parity choice. The Soviet asset is real and live in `gamemd.exe`, but the verified reports place it on the in-game side-1 mode-2 branch, not the ordinary offline Skirmish shell validation popup.

## 6. Remaining Mismatches / Untested Visual Risks

| Area | Status | Player-visible risk | Evidence |
|---|---|---|---|
| Native neutral background unavailable in current path | Mismatch if parity is the goal; current atlas has only `validation_modal_background_pudlgbgs` for this modal. | Shell validation popup shows Soviet image instead of neutral menu image. | `src/render/skirmish_shell_chrome.rs:49`; `src/app_skirmish_shell_render/modals.rs:173`; paint-composition report. |
| Background art size vs layout/hit rect | Untested; current Rust draws the SHP at native atlas size, while layout uses `451x326`. Prior asset preview observed the neutral background around `454x328`; this slot did not re-dump Soviet dimensions. | Art could extend a few pixels beyond text/hit layout or differ from native DLU/window bounds. | `push_entry_native` at `src/app_skirmish_shell_render/modals.rs:173`; layout constants at `src/ui/skirmish_shell/layout.rs:35`; prior asset preview evidence. |
| Text alignment/wrapping | Partially matched by control rects, but exact native `FUN_00621040` flags/wrapping for static `0x5B0` and OK text are not proven here. | Long capacity/error text may wrap or clip differently. | `src/app_skirmish_shell_render/text.rs:734`; text-layout swarm slot is separate. |
| Child paint order vs Rust text pass | Untested; Rust builds modal/button sprites first and text draws after, while native child paint order is handled by Win32/owner-draw controls. | Usually low if text does not overlap, but clipping/order mismatches could show under edge strings. | `src/app_skirmish_shell_render.rs:308`, `src/app_skirmish_shell_render.rs:441`; paint-composition report. |
| Button frame `1` | Loaded but not used. Native report says frame `1` is selected by a record-flag/alternate state path; exact disabled/default/focus UX mapping remains deferred. | Disabled/focus/default-button state may be visually wrong if it becomes observable. | `src/render/skirmish_shell_chrome.rs:170`; `src/app_skirmish_shell_render/chrome.rs:321`; MNBTTN report. |
| Start button disabled underlay | Current scan did not find a visual Start-disabled state while the modal is open. Prior native report says Start is disabled while the helper runs, but whether the disabled underlay is visible before/during the modal remains runtime conditional. | Underlying Start button may remain visually enabled behind/near the modal. | `src/app.rs:743`; prior disabled-underlay report via swarm claims. |
| Rust-only random selection validation | Current modal can show `"Random side/color selection is not available yet."`, which is not a verified native Start validation message. | A Rust-only failure path uses the modal chrome; useful for development but not native parity. | `src/app.rs:735`; `src/ui/skirmish_shell/state/launch.rs:20`. |
| Pixel/screenshot parity | Untested. No native 800x600 invalid-Start screenshot was compared against current Rust after the Soviet override. | Exact modal position, palette conversion, clipping, and text pixels remain unverified. | no screenshot artifact or test found in scan. |

## 7. Test Coverage Scan

| Test / check | What it covers | Gap |
|---|---|---|
| `validation_modal_layout_centers_ok_button` | Current 800x600 computed dialog/message/OK rects. | Does not prove Win32 DLU pixel parity or background SHP size. |
| `validation_modal_button_uses_mnbttn_normal_and_pressed_frames` | Frame `0/2` mapping for normal/pressed. | Does not cover frame `1`, disabled/default focus, or asset load failure behavior. |
| `validation_modal_button_draws_mnbttn_at_native_size_centered_on_control` | MNBTTN placement for a sample control rect and `126x25` entry. | Does not prove retail decode unless paired with ignored asset test. |
| `retail_shell_shp_dimensions_match_research` | Ignored test can confirm `MNBTTN` frames are `126x25` with retail assets. | Ignored; does not cover `PUDLGBGS/PUDLGBGN` dimensions. |
| `validation_modal_dismissal_keys_match_dialog_translation` | Enter/NumpadEnter/Escape dismissal key list. | Does not verify full event ordering or mouse/capture edge cases. |
| `validation_modal_semantic_draw_order_is_blocking_overlay` | Modal/button role ordering excludes normal owner-draw buttons. | Does not model text roles or Soviet-vs-neutral background identity. |

No tests were run in this slot. The swarm prompt forbids modifying generated target files, and running Rust tests would update `target/`.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Verified native activation/template facts | verified-from-docs | activation recheck and template rect reports | fresh Ghidra not needed for this current-Rust delta slot |
| Verified native paint/background facts | verified-from-docs | paint-composition report | native screenshot capture remains separate |
| Verified native MNBTTN button facts | verified-from-docs | MNBTTN report | exact frame-1 UX mapping remains deferred |
| Rust asset atlas scan | verified | `src/render/skirmish_shell_chrome.rs:116`, `:169`, `:189`, `:352`, `:359` | add neutral atlas path if returning to parity |
| Rust modal sprite construction | verified | `src/app_skirmish_shell_render/modals.rs:167` | no pixel screenshot test |
| Rust layout scan | verified | `src/ui/skirmish_shell/layout.rs:35`, `:234`, `:849` | final Win32/runtime pixel proof |
| Rust text scan | touched-not-exhausted | `src/app_skirmish_shell_render/text.rs:726` | exact native text flags/wrapping |
| Rust input/dismiss scan | verified | `src/app.rs:1159`, `:1178`, `:1195`, `:1557` | mouse capture/off-control drag edge cases not exhaustively tested |
| Start-disabled underlay | touched-not-exhausted | `src/app.rs:743`; prior disabled-underlay report | runtime visibility and Rust disabled-state rendering |
| Current screenshots/pixel diffs | not-touched | none | capture native and Rust side-by-side |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does current Rust still use generic PCX art for the validation OK button? -> No for the primary path; it now uses `push_modal_button_mnbttn`, with `push_button_30` only as a missing-asset fallback.` (evidence: `src/app_skirmish_shell_render/modals.rs:197`, `src/app_skirmish_shell_render/chrome.rs:347`)
- `[RESOLVED] OQ-02 - Does current Rust implement the requested Soviet background? -> Yes; it loads `PUDLGBGS.SHP` with `DIALOG.PAL` and draws it for the validation modal.` (evidence: `src/render/skirmish_shell_chrome.rs:117`, `src/render/skirmish_shell_chrome.rs:189`, `src/app_skirmish_shell_render/modals.rs:173`)
- `[RESOLVED] OQ-03 - Is that Soviet background native parity for ordinary offline Skirmish validation? -> No; verified docs say shell/no-game validation uses `PUDLGBGN.SHP + DIALOGN.PAL`.` (evidence: paint-composition and activation recheck reports)
- `[RESOLVED] OQ-04 - Does current Rust use verified `0x5B0/0x5AE` dialog-unit rect constants? -> Yes, through `dlu_rect(40,40,220,50)` and `dlu_rect(207,175,83,15)`.` (evidence: `src/ui/skirmish_shell/layout.rs:857`)
- `[RESOLVED] OQ-05 - Does current Rust have test coverage for normal/pressed MNBTTN frame selection? -> Yes.` (evidence: `src/app_skirmish_shell_render.rs:786`)
- `[DEFERRED] OQ-06 - Exact native-vs-Rust screenshot pixels after the Soviet override.` (category: `needs-runtime-debugger`; reason: no native screenshot or current Rust screenshot diff was produced in this slot; next-step-if-pursued: capture native neutral target plus Rust Soviet override and compare intentionally)
- `[DEFERRED] OQ-07 - Exact text wrapping/alignment and OK label pressed offset parity.` (category: `requires-different-system-context`; reason: this slot only scanned current Rust and existing docs; next-step-if-pursued: consume dedicated text-layout slot or run a focused text paint investigation)
- `[DEFERRED] OQ-08 - Frame `1` disabled/default/focus mapping.` (category: `requires-different-system-context`; reason: existing MNBTTN report defers exact UX mapping; next-step-if-pursued: native screenshot or message-state trace for disabled/default OK)
- `[DEFERRED] OQ-09 - Whether Start disabled underlay is visible under the modal in current Rust vs native.` (category: `needs-runtime-debugger`; reason: prior binary report says Start is disabled but repaint visibility is runtime conditional; next-step-if-pursued: screenshot invalid Start with modal open)

## 10. Visual/UI Composition Ledger

| Order | Current Rust function / surface | Asset / frame | Rect / anchor | Palette / convert | Native classification |
|---:|---|---|---|---|---|
| 1 | `push_validation_modal_instances` | `PUDLGBGS.SHP#0` | `layout.dialog.x/y`, native atlas size | `DIALOG.PAL` | Intentional Soviet override; native shell target is `PUDLGBGN.SHP#0 + DIALOGN.PAL` |
| 2 | `push_modal_button_mnbttn` | `MNBTTN.SHP#0` or `#2` | centered on `layout.ok_button` | `MAINBTTN.PAL` | Matches native normal/pressed art path |
| 3 | `push_validation_modal_text_draws` | text only | `layout.message`, `button_text_rect(layout.ok_button, pressed)` | current shell text color/font path | Partially matched; exact native flags/wrapping unverified |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `PUDLGBGS.SHP` | Yes | Yes | Yes in current Rust | No | Yes | No | No | Not native for shell target | `src/render/skirmish_shell_chrome.rs:192`; `src/app_skirmish_shell_render/modals.rs:173` |
| `DIALOG.PAL` | Yes | Palette input | Yes in current Rust | No | Palette | No | No | Not native palette for shell target | `src/render/skirmish_shell_chrome.rs:117` |
| `PUDLGBGN.SHP` | No for this modal path | No | No | No | Native target chrome | No | No | Missing from current validation path | `rg` source scan; paint-composition report |
| `DIALOGN.PAL` | No source hit in current validation path | No | No | No | Native target palette | No | No | Missing from current validation path | `rg` source scan; paint-composition report |
| `MNBTTN.SHP` | Yes | Yes | Yes | No | OK button art | No | No | No | `src/render/skirmish_shell_chrome.rs:169`; `src/app_skirmish_shell_render/chrome.rs:337` |
| `MAINBTTN.PAL` | Yes | Palette input | Yes | No | OK button palette | No | No | No | `src/render/skirmish_shell_chrome.rs:116` |
| `bue/bde_*30.pcx` | Yes elsewhere | Fallback only for validation OK | No in primary validation OK path | No | Generic button fallback | No | No | Inactive unless MNBTTN missing | `src/app_skirmish_shell_render/chrome.rs:347` |

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary shell validation uses neutral `PUDLGBGN.SHP + DIALOGN.PAL`, not Soviet, in native YR. | `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md`; activation recheck report | intentional mismatch: Rust uses `PUDLGBGS.SHP + DIALOG.PAL` | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render/modals.rs` | Keep the Soviet path only if the product decision is override; otherwise load/draw neutral background for parity | Invalid Start in Skirmish setup shows neutral shell modal when parity mode is selected | Do not document the Soviet Skirmish popup as native parity; proposed test `skirmish_validation_modal_background_theme_is_explicit_override`; risk: user preference vs parity target conflict |
| OK `0x5AE` uses `MNBTTN.SHP + MAINBTTN.PAL`, frames `0/2` for normal/pressed primary path. | MNBTTN report; paint-composition report | mostly matched; frame `1` and exact disabled/default state remain unmodeled | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render/chrome.rs`, `src/app.rs` mouse handlers | Preserve MNBTTN primary path and add only verified frame-1/default behavior when researched | Click-hold OK on validation popup shows MNBTTN pressed frame and release dismisses | Do not reintroduce `push_button_30` as the primary OK art; proposed test `skirmish_validation_ok_keeps_mnbttn_primary_path`; risk: missing asset fallback can hide parity failures |
| Native template gives body and OK child rects in dialog units, but final pixels require runtime conversion/capture. | template rect report | partial match: Rust uses fixed base units and `451x326`, no screenshot proof | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/text.rs` | Verify or adjust final modal/window/control pixel positions against native capture | 800x600 invalid Start screenshot aligns body text, OK art, and modal bounds with target/override policy | Do not treat the current `451x326` constants as final screenshot proof; proposed test `skirmish_validation_modal_pixel_rects_match_reference_capture`; risk: off-by-few-pixel drift |
| Body/static and OK label text are child-control paint, not a generic center-everything overlay. | paint-composition report; text-layout report pending | unverified: Rust centers both in computed rects and draws text after sprites | `src/app_skirmish_shell_render/text.rs` | Reconcile exact alignment/wrap/clip/pressed-label offset after text slot completes | Long capacity error wraps/clips like native in `0x5B0`; OK label sits like native in `0x5AE` | Do not tune strings visually without text-path evidence; proposed test `skirmish_validation_modal_text_matches_native_control_flags`; risk: font/wrap drift |

## 12. Negative Facts / Do Not Do

- Do not call the current Soviet validation background native parity for ordinary offline Skirmish. Evidence: verified docs select `PUDLGBGN.SHP + DIALOGN.PAL` in shell/no-game state.
- Do not replace MNBTTN primary OK rendering with Skirmish 30-family PCX button pieces. Evidence: current Rust now matches the owner-draw type-3 handoff for frame `0/2`; native reports reject `push_button_30` for `0xCE/0x5AE`.
- Do not assert pixel parity from the current source scan. Evidence: no native screenshot/pixel diff exists, and final DLU conversion remains a deferred item in the verified reports.
- Do not use the Rust-only random-selection validation message as native proof. Evidence: `RandomSelectionUnverified` is a current Rust implementation guard, not one of the verified native ordinary Start failure branches.
- Do not hide missing MNBTTN assets behind fallback in parity tests. Evidence: `push_modal_button_mnbttn` falls back to `push_button_30` if the atlas entry is absent.

## 13. Stale Docs / Follow-Up Docs

No existing verified report needs replacement from this Rust delta scan. Suggested future wording for implementation notes only:

> Current Rust after the Soviet override intentionally renders the validation modal background with `PUDLGBGS.SHP + DIALOG.PAL`; this is a user-requested style override. Native ordinary offline Skirmish validation remains `PUDLGBGN.SHP + DIALOGN.PAL`.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_ACTIVATION_RECHECK_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/MNBTTN_MAINBTTN_MODAL_BUTTON_ART_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render/modals.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render/chrome.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render/text.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render/draw_order.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`

**Status:** COMPLETE for current Rust/doc delta scan; native pixel proof remains deferred to screenshot/runtime work.
