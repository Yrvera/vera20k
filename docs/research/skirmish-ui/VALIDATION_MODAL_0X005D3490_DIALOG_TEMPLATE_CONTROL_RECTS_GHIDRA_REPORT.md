# Validation Modal 0x005D3490 Dialog Template Control Rects - Ghidra Research Report

**Address(es):** `0x005D3490`, `0x00622650`, `0x004A3B40`, `0x005D36A0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact Win32 dialog-resource template selection and dialog-unit control rectangles for the modal helper used by Skirmish Start validation failures, including resource ids `0xCE`, `0x120`, `0x121` and child ids `0x5B0`, `0x5AE`, `2`, `0x5AF`.  
**Non-Scope:** modal paint/chrome asset composition, final screenshot pixels after Windows dialog-unit conversion, keyboard/default-button behavior, Start disabled underlay timing, Choose Map, random map, and gameplay spawn.  
**Confidence:** High for resource ids, child ids, classes, styles, and template dialog-unit rectangles; Medium for Rust visual delta because final runtime pixel conversion was not captured.  
**Active in YR:** Yes for `0xCE` ordinary Skirmish Start validation failures; Conditional for `0x120`/`0x121` generic helper variants when callers supply third/fourth text arguments.

## 0. Working Notes

- Target question: What native dialog resource/template geometry and child-control ids/rects does `0x005D3490` use for the Start validation modal?
- Non-goals: Do not investigate random map, Choose Map, gameplay spawn, broad Skirmish shell layout, modal paint assets, keyboard/default dismissal, or Start disabled underlay timing.
- Evidence needed to mark COMPLETE: decompile plus assembly proving template selection and control id writes; resource bytes parsed from `gamemd.exe` proving dialog/control rects; caller evidence proving ordinary Skirmish failures use the one-button resource.
- Stop conditions: Stop after resource ids `0xCE/0x120/0x121`, controls `0x5B0/0x5AE/2/0x5AF`, their template rects/classes/styles, and current Rust layout implications are recorded.

## 1. Overview

`0x005D3490` is a generic shell message dialog helper. It selects one of three RT_DIALOG resources based only on whether the third and fourth string parameters are non-empty, creates the dialog with `CreateDialogIndirectParamA`, writes caller-supplied strings into known child controls, shows/pumps the dialog, then returns the result written by the dialog proc.

Ordinary Skirmish Start validation failures pass only the message body and `TXT_OK`, with both optional text arguments zero. Active in YR: Yes. Evidence: call-site assembly around `0x006AD0A7..0x006AD0C6`, `0x006AD126..0x006AD145`, and `0x006AD274..0x006AD293`; helper assembly `0x005D351D..0x005D3541`.

## 2. Native Template Selection

| Condition in `0x005D3490` | Dialog resource id | Meaning | Active in YR |
|---|---:|---|---|
| `param_4` non-null and first UTF-16 code unit nonzero | `0x121` | four-control / three-button variant | Conditional; active only for callers with fourth text |
| else `param_3` non-null and first UTF-16 code unit nonzero | `0x120` | three-control / two-button variant | Conditional; active only for callers with third text |
| else | `0xCE` | one message static plus one OK button | Yes for ordinary Skirmish Start validation failures |

Evidence: decompile `0x005D3490`; assembly `0x005D351D..0x005D353A` sets `ECX=0xCE`, overrides to `0x120` if `param_3` is non-empty, or `0x121` if `param_4` is non-empty, then calls `0x00622650` at `0x005D3541`. `0x00622650` calls `0x004A3B40` with `EDX=5`, and `0x004A3B40` passes that type to `FindResourceA`; resource type `5` is RT_DIALOG. Active in YR: Yes.

## 3. Resource Template Geometry

All three templates are standard `DLGTEMPLATE` resources, not extended templates. Units below are native dialog units from `gamemd.exe` resources; final pixels require the Win32 dialog-unit conversion for 8pt `MS Sans Serif` and are intentionally not claimed here.

| Resource | File evidence | Style | Dialog rect | Font | Child count | Active in YR |
|---|---|---:|---|---|---:|---|
| `0xCE` | `.rsrc` RVA `0x7F5A3C`, file offset `0x4F9A3C`, size `138` | `0x40000040` | `x=0 y=0 cx=300 cy=200` | 8pt `MS Sans Serif` | 2 | Yes for ordinary Start validation failures |
| `0x120` | `.rsrc` RVA `0x800A68`, file offset `0x504A68`, size `186` | `0x40000040` | `x=0 y=0 cx=300 cy=200` | 8pt `MS Sans Serif` | 3 | Conditional |
| `0x121` | `.rsrc` RVA `0x800B24`, file offset `0x504B24`, size `232` | `0x40000040` | `x=0 y=0 cx=300 cy=200` | 8pt `MS Sans Serif` | 4 | Conditional |

Resource parse source: `<ra2-install>/gamemd.exe`, PE resource directory `.rsrc` at RVA `0x77A000`. Active in YR: Yes for the loaded retail binary resources.

### Resource `0xCE` - Ordinary Start Validation Variant

| Index | Control id | Class | Title in template | Style | Rect in dialog units | Text set by `0x005D3490` | Active in YR |
|---:|---:|---|---|---:|---|---|---|
| 0 | `0x5B0` (`1456`) | static, ordinal `0x82` | `GUI:Blank` | `0x50000000` | `x=40 y=40 cx=220 cy=50` | `param_1` message body | Yes |
| 1 | `0x5AE` (`1454`) | button, ordinal `0x80` | `GUI:OK` | `0x5000000B` | `x=207 y=175 cx=83 cy=15` | `param_2`, `TXT_OK` for Start failures | Yes |

Evidence: resource bytes at RVA `0x7F5A3C`; helper writes `param_1` to `GetDlgItem(hwnd,0x5B0)` at `0x005D3573..0x005D3588` and `param_2` to `GetDlgItem(hwnd,0x5AE)` at `0x005D3592..0x005D35A9`. Dialog proc maps a `WM_COMMAND` from `0x5AE` with notification high word `0` to result `0`; evidence `0x005D36A0` decompile. Active in YR: Yes.

### Resource `0x120` - Optional Second Button Variant

| Index | Control id | Class | Title in template | Style | Rect in dialog units | Text set by `0x005D3490` | Active in YR |
|---:|---:|---|---|---:|---|---|---|
| 0 | `2` | button, ordinal `0x80` | `GUI:Cancel` | `0x5000000B` | `x=207 y=175 cx=83 cy=15` | `param_3` when non-empty | Conditional |
| 1 | `0x5B0` (`1456`) | static, ordinal `0x82` | `GUI:Blank` | `0x50000000` | `x=40 y=40 cx=220 cy=50` | `param_1` message body | Conditional |
| 2 | `0x5AE` (`1454`) | button, ordinal `0x80` | `GUI:OK` | `0x5000000B` | `x=207 y=155 cx=83 cy=15` | `param_2` | Conditional |

Evidence: resource bytes at RVA `0x800A68`; optional `param_3` write to control `2` at `0x005D35B6..0x005D35CA`. Dialog proc maps id `1` or `2` with notification `0` to result `1`; evidence `0x005D36A0`. Active in YR: Conditional.

### Resource `0x121` - Optional Third Button Variant

| Index | Control id | Class | Title in template | Style | Rect in dialog units | Text set by `0x005D3490` | Active in YR |
|---:|---:|---|---|---:|---|---|---|
| 0 | `2` | button, ordinal `0x80` | `GUI:Cancel` | `0x5000000B` | `x=207 y=175 cx=83 cy=15` | `param_3` when non-empty | Conditional |
| 1 | `0x5B0` (`1456`) | static, ordinal `0x82` | `GUI:Blank` | `0x50000000` | `x=40 y=40 cx=220 cy=50` | `param_1` message body | Conditional |
| 2 | `0x5AE` (`1454`) | button, ordinal `0x80` | `GUI:OK` | `0x5000000B` | `x=207 y=135 cx=83 cy=15` | `param_2` | Conditional |
| 3 | `0x5AF` (`1455`) | button, ordinal `0x80` | `GUI:Blank` | `0x5000000B` | `x=207 y=155 cx=83 cy=15` | `param_4` when non-empty | Conditional |

Evidence: resource bytes at RVA `0x800B24`; optional `param_4` write to control `0x5AF` at `0x005D35D7..0x005D35EE`. Dialog proc maps id `0x5AF` with notification `0` to result `2`; evidence `0x005D36A0`. Active in YR: Conditional.

## 4. Start Validation Caller Proof

| Failure branch | `0x005D3490` optional args | Template selected | Evidence | Active in YR |
|---|---|---|---|---|
| map capacity too small | `param_3=0`, `param_4=0` | `0xCE` | pushes three zeros before `TXT_OK`/body call at `0x006AD0A7..0x006AD0C6` | Yes |
| fewer than two players | `param_3=0`, `param_4=0` | `0xCE` | pushes three zeros before `TXT_OK`/body call at `0x006AD126..0x006AD145` | Yes |
| same explicit team | `param_3=0`, `param_4=0` | `0xCE` | pushes three zeros before `TXT_OK`/body call at `0x006AD274..0x006AD293` | Yes |
| selected-mode rejection with output dword `0x617` | `param_3=0`, `param_4=0` | `0xCE` | setup begins at `0x006AD2E3`; final call continues after `0x006AD311` | Conditional |

## 5. Current Rust Implementation Status

Current Rust uses a hand-built pixel layout, not the native dialog-resource geometry. Evidence: `src/ui/skirmish_shell/layout.rs` defines `VALIDATION_MODAL_W=360`, `VALIDATION_MODAL_H=122`, message rect `(24,24,w-48,42)`, and centered OK rect `82x24` at `VALIDATION_MODAL_H-40`. Rendering uses a solid panel/outline and `push_button_30`; evidence `src/app_skirmish_shell_render/modals.rs`.

Rust therefore matches the functional one-button modal shape but does not yet model the native resource id `0xCE`, dialog-unit `300x200` template, `0x5B0` static rect, or `0x5AE` ownerdraw button rect. Active in YR comparison: Yes.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes and scope | verified | section 0 | none |
| `0x005D3490` template selection | verified | decompile and assembly `0x005D351D..0x005D3541` | none |
| resource loader uses RT_DIALOG | verified | `0x00622665..0x006226CB`, `0x004A3B40` | none |
| resource `0xCE` geometry | verified | `.rsrc` RVA `0x7F5A3C`, file offset `0x4F9A3C` | final pixel conversion deferred |
| resource `0x120` geometry | verified | `.rsrc` RVA `0x800A68`, file offset `0x504A68` | final pixel conversion deferred |
| resource `0x121` geometry | verified | `.rsrc` RVA `0x800B24`, file offset `0x504B24` | final pixel conversion deferred |
| dialog proc id-to-result mapping | touched-not-exhausted | `0x005D36A0` | keyboard/default behavior belongs to slot 3 |
| ordinary Skirmish Start validation call sites | verified | `0x006AD0A7..0x006AD293` | selected-mode final call tail only touched |
| current Rust primitive layout | verified | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/modals.rs` | native visual implementation |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which helper owns the modal template? -> `0x005D3490` selects and creates the dialog.` (evidence: `0x005D3490`)
- `[RESOLVED] OQ-02 - Does the helper load Win32 dialog resources or custom assets? -> It calls `0x00622650`, which calls `0x004A3B40` with type `5` and then `CreateDialogIndirectParamA`.` (evidence: `0x00622665..0x006226CB`, `0x004A3B40`)
- `[RESOLVED] OQ-03 - Which template does ordinary Start validation use? -> Resource `0xCE`.` (evidence: zero optional args at `0x006AD0A7..0x006AD293`; selection at `0x005D351D..0x005D353A`)
- `[RESOLVED] OQ-04 - What is the resource `0xCE` dialog rect? -> `x=0 y=0 cx=300 cy=200` dialog units, 8pt `MS Sans Serif`.` (evidence: `.rsrc` RVA `0x7F5A3C`)
- `[RESOLVED] OQ-05 - What is control `0x5B0`? -> Static ordinal `0x82`, `x=40 y=40 cx=220 cy=50`, receives body text.` (evidence: resource `0xCE`; write at `0x005D3573..0x005D3588`)
- `[RESOLVED] OQ-06 - What is control `0x5AE` in Start validation? -> Ownerdraw button ordinal `0x80`, `x=207 y=175 cx=83 cy=15`, receives `TXT_OK`, command result `0`.` (evidence: resource `0xCE`; write at `0x005D3592..0x005D35A9`; proc `0x005D36A0`)
- `[RESOLVED] OQ-07 - Are controls `2` and `0x5AF` present in the ordinary Start validation resource? -> No; they are present only in `0x120`/`0x121`.` (evidence: resource tables above)
- `[RESOLVED] OQ-08 - What are `0x120` and `0x121` rects? -> Same `300x200` dialog; buttons stack at `y=155/175` for two-button and `y=135/155/175` for three-button variants.` (evidence: resource RVAs `0x800A68`, `0x800B24`)
- `[RESOLVED] OQ-09 - Is `0x5AE` a second static text control? -> No; the resource class is button ordinal `0x80` in all three templates.` (evidence: resource tables)
- `[DEFERRED] OQ-10 - What are final on-screen pixel rects after DLU conversion?` (category: requires-different-system-context; reason: static resource gives dialog units, but exact pixels require runtime font/base-unit conversion or screenshot capture; next-step-if-pursued: capture native dialog or instrument `GetWindowRect`/`MapDialogRect`)
- `[DEFERRED] OQ-11 - What exact chrome/assets paint these ownerdraw buttons and panel?` (category: out-of-scope; reason: slot 2 owns paint composition; next-step-if-pursued: trace ownerdraw paint path)
- `[DEFERRED] OQ-12 - Does Enter/Escape activate `0x5AE`, `2`, or cancel?` (category: out-of-scope; reason: slot 3 owns keyboard/default behavior; next-step-if-pursued: drain `IsDialogMessage`/dialog proc key path)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary Start validation failures use RT_DIALOG resource `0xCE`, a `300x200` dialog-unit template with static `0x5B0` at `40,40,220,50` and OK ownerdraw button `0x5AE` at `207,175,83,15` | `0x005D3490`, `0x00622650`, resource RVA `0x7F5A3C`; call sites `0x006AD0A7..0x006AD293` | missing; Rust uses `360x122` hand-built pixel modal and centered `82x24` OK | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/modals.rs`, `src/app_skirmish_shell_render/text.rs` | replace/augment validation layout with native resource-derived dialog-unit geometry, then convert/capture to stable pixel rects for 800x600 | no-opponent Start failure at 800x600 shows one native-shaped dialog with body in `0x5B0` rect and OK at native `0x5AE` position | Do not center the OK button by guesswork; proposed test `skirmish_validation_modal_uses_resource_0xce_control_rects`; risk: DLU-to-pixel conversion must be verified before asserting exact pixels |
| `0x5AE` is the OK button and command result `0`; controls `2` and `0x5AF` are absent from the ordinary one-button Start template | resource `0xCE`; dialog proc `0x005D36A0` | Rust has one OK button but no native id model | app modal input and layout hit-test | keep one actionable OK target for ordinary Start validation; optional controls must not be created for this path | capacity/no-opponent/same-team failure produces exactly one visible button and no Cancel/third button | Do not add `IDOK=1`, `IDCANCEL=2`, or `0x5AF` to the ordinary Start modal; proposed test `skirmish_start_validation_modal_has_only_native_ok_button`; risk: keyboard behavior still separate |
| `0x120`/`0x121` are generic helper variants only when optional third/fourth text exists, with button ids `2` and `0x5AF` stacked above/below `0x5AE` | resource RVAs `0x800A68`, `0x800B24`; selection at `0x005D351D..0x005D353A` | unchecked for future non-Start callers | future generic shell modal abstraction, not ordinary Start-only state | if Rust generalizes the helper, select template by optional button-text count, not by caller name | synthetic helper call with third text uses `0x120` geometry; fourth text uses `0x121` geometry | Do not apply `0x120/0x121` button set to ordinary Start failures; proposed test `shell_message_dialog_selects_template_by_optional_button_texts`; risk: not needed until generic modal helper exists |

## 9. Negative Facts / Do Not Do

- Do not call `0x5AE` a static text field. Active in YR: Yes for Start validation; evidence: resource `0xCE` class ordinal `0x80` button and proc `0x005D36A0`.
- Do not add controls `2` or `0x5AF` to ordinary Start validation failures. Active in YR: Yes for absence; evidence: resource `0xCE` has only controls `0x5B0` and `0x5AE`; Start failures pass zero optional args.
- Do not treat the current Rust `360x122` pixel modal as native-template verified. Active in YR comparison: Yes; evidence: native resource is `300x200` dialog units at `.rsrc` RVA `0x7F5A3C`.
- Do not choose dialog variant by message type or failure type. Active in YR: Yes; evidence: `0x005D351D..0x005D353A` chooses solely from non-empty `param_3`/`param_4`.
- Do not infer final pixel bounds from the resource rects without a DLU conversion/capture step. Active in YR: Conditional on runtime font metrics; evidence: standard `DLGTEMPLATE` with 8pt `MS Sans Serif`.

## 10. Remaining Uncertainty

- Exact final screen pixel bounds after Win32 dialog-unit conversion and any shell scaling are not proven in this slot.
- Exact ownerdraw/chrome composition for the dialog and buttons is not proven in this slot.
- Enter/Escape/default-button behavior is not proven in this slot.

## 11. Stale Docs / Replacement Wording

- `docs/research/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`: replace "Modal helper `0x005D3490` writes message/body text to child `0x5B0`, second text to child `0x5AE`, optional button text to controls `2` and `0x5AF` only when non-empty" with "Modal helper `0x005D3490` selects RT_DIALOG `0xCE/0x120/0x121` from optional button-text presence. Ordinary Start validation uses `0xCE`: static `0x5B0` receives body text and ownerdraw button `0x5AE` receives `TXT_OK`; optional button controls `2` and `0x5AF` exist only in `0x120/0x121`."
- `docs/research/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`: replace "exact native modal pixels/resource rectangles" deferred wording with "resource/template rectangles are verified in `VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md`; final runtime pixel conversion and paint composition remain deferred."

## Sources

- Ghidra read-only decompile/assembly: `0x005D3490`, `0x00622650`, `0x004A3B40`, `0x005D36A0`, Start call-site assembly around `0x006AD0A7`, `0x006AD126`, `0x006AD274`, `0x006AD2E3`.
- Retail binary resource parse: `<ra2-install>/gamemd.exe`, `.rsrc` RVA `0x77A000`; RT_DIALOG ids `0xCE`, `0x120`, `0x121`.
- Prior doc: `docs/research/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/modals.rs`, `src/app.rs`.
