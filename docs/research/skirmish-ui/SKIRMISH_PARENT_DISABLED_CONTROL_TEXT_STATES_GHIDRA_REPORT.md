# Skirmish Parent Disabled Control Text States - Ghidra Research Report

**Address(es):** Prior verified sites `FUN_00621040 @ 0x00621040`, `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`, `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `OwnerDraw_Button_00612B70 @ 0x00612B70`, row sibling update `FUN_006ADC20`, mode row update `FUN_006ACD60`.
**Investigation Mode:** coverage-map with a narrow Rust disparity pass.
**Claimed Scope:** Disabled text/color behavior for parent Skirmish shell controls: disabled collapsed combo faces, option checkbox labels, trackbar labels/value text, and parent buttons only where shared disabled text branches apply.
**Non-Scope:** Validation-modal disabled Start underlay timing, modal `0x005D3490` visual composition, Choose Map modal controls, player-name edit `0x6A0`, and fresh runtime screenshot RGB capture.
**Confidence:** Medium-high for behavior already backed by prior Ghidra reports; Medium for Rust deltas; Partial overall because this session has no Ghidra MCP exposed for fresh read-only decompile.
**Active in YR:** Yes for normal parent shell controls; Conditional for disabled branches when `WS_DISABLED` is set; Yes for inactive opponent row sibling combos because standard row-state code disables those controls.

## 0. Working Notes

- Target question: Which parent Skirmish shell controls use disabled text/color branches, which branches are active in standard YR, and where does current Rust diverge?
- Non-goals: Do not investigate validation-modal disabled Start underlay timing, modal template/paint, Choose Map, player-name edit, or exact post-DirectDraw screenshot RGB.
- Evidence needed to mark COMPLETE: Fresh decompile plus assembly/xref proof for each disabled branch, row disable liveness, and current Rust comparison. This report has prior verified Ghidra evidence but no fresh Ghidra MCP access, so final status is PARTIAL.
- Stop conditions: Write exactly this report and update the shared swarm claims file; do not edit Rust, INI, or other docs.

## 1. Overview

The shared shell text wrapper uses normal shell yellow `DAT_00AC18A4 = 0x0000FFFF` for ordinary labels and disabled packed color `DAT_00AC1CB4 = 0x9F` where owner-draw callers test `WS_DISABLED`. The important live parent-shell disabled surface is not the three setup trackbars and not ordinary option checkboxes; it is inactive opponent row sibling controls. Native disables those row sibling combo controls, so their collapsed combo text should follow the disabled/grey combo text path.

Current Rust has most controls drawn in normal yellow and has chrome-level disabled handling for inactive combo arrows, but it does not carry a disabled text color/state into combo-face text drawing. It also draws trackbar numeric values with the button-specific dark `RGB(5,12,0)` constant even though prior Ghidra reports verify trackbar value text uses normal shell yellow unless the trackbar HWND is disabled.

## 2. Verified Binary Findings

| Finding | Active in YR | Evidence |
|---|---|---|
| Normal shell text source is `DAT_00AC18A4 = 0x0000FFFF`, byte order `0x00BBGGRR`, so source RGB is `(255,255,0)`. | Yes | `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `FUN_00621040 @ 0x00621040`, assembly `0x00621054..0x006210B1`. |
| Disabled text source used by static/checkbox/trackbar owner-draw branches is `DAT_00AC1CB4 = 0x9F`. | Conditional | Static report cites `OwnerDraw_Static_006153E0`, checkbox assembly `0x00616651..0x00616655`, trackbar assembly `0x0061E2A5..0x0061E2B6`. |
| Checkbox labels use normal yellow when enabled and switch to `DAT_00AC1CB4` only if the checkbox HWND has `WS_DISABLED`. | Conditional; standard option checkboxes are normally enabled | Prior decompile `OwnerDraw_Checkbox_006163A0`; call site `0x0061663E..0x00616674`. |
| Trackbar value text uses normal yellow when enabled and switches to `DAT_00AC1CB4` only if `WS_DISABLED` is set. Standard offline `0x102` does not disable trackbars `0x529`, `0x511`, or `0x50C`. | Yes for enabled values; Conditional disabled branch not normally reached | `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`, `OwnerDraw_Trackbar_0061D950`, `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_006AE3F0`. |
| Collapsed combo selected text has a disabled/grey color branch before calling `FUN_00621040`. | Conditional | `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`, `OwnerDraw_ComboBox_00617250`, color mode evidence `0x00617A39..0x00617B2E`. |
| Inactive AI rows disable row sibling country/color/start/team controls through standard parent update paths, so disabled combo-face visuals are a live standard Skirmish surface. | Yes when an opponent row is inactive/None | `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md` row-sibling contrast; `FUN_006ADC20`, `FUN_006ACD60`, `FUN_006ADDF0`, `FUN_006AE080`, `FUN_006ADF00`. |
| Parent Start disabled timing belongs to the validation modal slot; this report only relies on the shared fact that `OwnerDraw_Button_00612B70` has a disabled style path if painted while `WS_DISABLED` is set. | Conditional | `VALIDATION_MODAL_0X005D3490_DISABLED_START_UNDERLAY_TIMING_GHIDRA_REPORT.md`; disabled paint style test around `0x006135FD..0x0061361B`. |

## 3. Current Rust Implementation Status

| Surface | Current Rust | Status vs verified behavior |
|---|---|---|
| Normal labels / checkbox labels / trackbar labels | `push_label_draw` uses `SHELL_LABEL_TEXT_RGB` in `src/app_skirmish_shell_render/text.rs`. | Matches enabled yellow for normal parent labels. No disabled text branch exists, but no normal standard flow disables option checkboxes or trackbar labels. |
| Trackbar value text | `text.rs:459..467` draws values with `SHELL_BUTTON_TEXT_RGB_00000C05`. | Mismatch for normal standard `0x102`; should use normal shell yellow unless a real disabled trackbar state is introduced. |
| Inactive opponent row combo chrome | `controls.rs:416..450` passes `sibling_disabled` to combo faces, greying arrows and hiding inactive color swatches. | Partially matches disabled chrome. |
| Inactive opponent row combo text | `text.rs:518..532` draws side/start/team labels only inside `if opponent.is_active()`. | Mismatch/risk: native disabled combo controls still have an owner-draw disabled/grey text path; Rust suppresses inactive sibling combo text entirely. |
| Parent buttons | `push_button_label_draw` has no disabled argument; current Start modal disabled-underlay handling is outside this scope. | No normal parent-button disabled text delta proven here; validation modal slot owns timing/visibility. |
| Forced disabled checkbox/trackbar harness | No disabled color constant/state path exposed. | Optional shared owner-draw completeness only; not required for normal offline `0x102` unless a reachable disabled state is proven. |

## 4. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Required working-note lines | verified | Section 0 | none |
| Shared shell text normal/disabled source colors | verified-from-prior-doc | `FUN_00621040`, static text report | fresh decompile unavailable this session |
| Checkbox disabled text branch | verified-from-prior-doc | `0x0061663E..0x00616674` | standard runtime writer for option checkboxes not freshly rechecked |
| Trackbar disabled branch and non-reachability | verified-from-prior-doc | trackbar disabled runtime flow report | none for normal offline trackbars |
| Combo disabled text branch | touched-not-exhausted | `0x00617A39..0x00617B2E` prior text contract | exact final disabled color source name for combo branch should be freshly decompiled if strict |
| Inactive row sibling disabled liveness | verified-from-prior-doc | `FUN_006ADC20`, `FUN_006ACD60` reports | exact selected text content in disabled sibling combo should be captured/rechecked |
| Parent button disabled branch | deferred | validation modal timing report | modal slot owns exact Start underlay timing and pixels |
| Current Rust comparison | verified | source scan `text.rs`, `controls.rs`, `app_skirmish_shell_render.rs` | no Rust edits performed |

## 5. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which mode applies? -> coverage-map, because fresh Ghidra MCP is unavailable and combo disabled color was not freshly drained.` (evidence: tool search found no Ghidra tools)
- `[RESOLVED] OQ-02 - What is the normal shell text source? -> `DAT_00AC18A4 = 0x0000FFFF`, source RGB yellow.` (evidence: `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `0x00621054..0x006210B1`)
- `[RESOLVED] OQ-03 - What is the disabled text source for checkbox/trackbar/static branches? -> `DAT_00AC1CB4 = 0x9F`.` (evidence: `0x00616651..0x00616655`, `0x0061E2A5..0x0061E2B6`)
- `[RESOLVED] OQ-04 - Are standard option trackbars disabled by normal Skirmish flow? -> No; they remain enabled through init, mode/map changes, validation failures, and readback.` (evidence: `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-05 - Is inactive row sibling disabled state live? -> Yes; parent update paths disable/enable row country/color/start/team controls.` (evidence: `FUN_006ADC20`, `FUN_006ACD60`)
- `[RESOLVED] OQ-06 - Does Rust model inactive combo text disabled color? -> No; it suppresses inactive row side/start/team text rather than drawing disabled-color selected text.` (evidence: `src/app_skirmish_shell_render/text.rs:518..532`)
- `[RESOLVED] OQ-07 - Does Rust model normal trackbar value color? -> No; values use button dark color instead of normal shell yellow.` (evidence: `src/app_skirmish_shell_render/text.rs:459..467`)
- `[DEFERRED] OQ-08 - Exact final disabled combo text source and final display RGB.` (category: `needs-runtime-debugger`; reason: prior docs prove a disabled/grey branch but final display RGB and exact combo grey source need fresh Ghidra/runtime capture; next-step-if-pursued: decompile `OwnerDraw_ComboBox_00617250` color block and capture retail inactive row combo)
- `[DEFERRED] OQ-09 - Parent button disabled text under validation modal.` (category: `requires-different-system-context`; reason: explicitly owned by `VALIDATION_MODAL_0X005D3490_DISABLED_START_UNDERLAY_TIMING`; next-step-if-pursued: use that modal slot and button paint-composition slot)
- `[DEFERRED] OQ-10 - Forced disabled checkbox visual harness.` (category: `out-of-scope`; reason: no standard option-checkbox disable writer was proven for normal parent Skirmish; next-step-if-pursued: search `EnableWindow` writers for checkbox IDs)

## 6. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard trackbar value text uses normal shell yellow while enabled; disabled `0x9F` exists only if `WS_DISABLED`, and standard offline trackbars are not disabled. | `0x0061E296..0x0061E30A`; trackbar disabled runtime flow report | mismatch: values use `SHELL_BUTTON_TEXT_RGB_00000C05` | `src/app_skirmish_shell_render/text.rs` trackbar value draw | Draw Game Speed/Credits/Unit Count values with normal label yellow in normal setup; do not add speculative disabled gating | Default skirmish setup shows trackbar numeric values in the same yellow family as labels; changing AI rows/map does not dim them | Proposed test `skirmish_trackbar_value_text_uses_normal_shell_yellow_when_enabled`; risk: button text color slot is separate and should not drive trackbar color |
| Inactive opponent row sibling combos are disabled controls, and combo owner-draw has a disabled/grey text branch. | `OwnerDraw_ComboBox_00617250` color block `0x00617A39..0x00617B2E`; row sibling disable paths `FUN_006ADC20`, `FUN_006ACD60` | mismatch/risk: Rust greys arrows but skips inactive sibling combo text | `src/app_skirmish_shell_render/controls.rs`, `src/app_skirmish_shell_render/text.rs` combo face rendering | Carry disabled state into combo-face label drawing and render disabled-color selected text for inactive row side/start/team as native does | Set opponent row to `None`: side/start/team combo faces remain visibly disabled and still show their selected/default text in disabled color | Proposed test `skirmish_inactive_opponent_combo_faces_draw_disabled_text`; risk: do not re-enable interaction or draw normal yellow text |
| Checkbox disabled text exists in shared owner-draw, but standard option checkboxes are normally enabled in this scoped parent setup. | `OwnerDraw_Checkbox_006163A0` `0x0061663E..0x00616674`; standard setup docs | no normal-flow delta; optional harness branch missing | checkbox text renderer if future disabled state is introduced | Leave normal option checkbox labels yellow; only add disabled color if a real disabled checkbox path or explicit harness state exists | Standard options remain readable/clickable yellow through map/mode changes | Proposed test `skirmish_option_checkboxes_remain_enabled_normal_yellow`; risk: do not dim option labels based on unrelated selected mode or inactive AI row state |

## 7. Negative Facts / Do Not Do

- Do not use validation-modal Start disabled timing as evidence here. Active in YR: Conditional; owned by `VALIDATION_MODAL_0X005D3490_DISABLED_START_UNDERLAY_TIMING_GHIDRA_REPORT.md`.
- Do not disable Game Speed, Credits, or Unit Count in standard offline Skirmish due to AI row state, selected mode, selected map, or validation failure. Active in YR: No for those trackbar disable flows.
- Do not draw trackbar numeric values with the button-specific dark `RGB(5,12,0)` in normal setup; verified trackbar value text uses normal shell text color while enabled.
- Do not suppress inactive row sibling combo text entirely; native has a disabled/grey collapsed combo text path when the HWND is disabled.
- Do not infer exact disabled final screenshot RGB from packed constants alone; DirectDraw conversion/capture still needs runtime sampling for pixel-perfect final color.

## 8. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`: replace "trackbar value draw: rect/color normal present; disabled color unchecked" with "trackbar value rect is present, but current Rust draws normal enabled values with `SHELL_BUTTON_TEXT_RGB_00000C05`; verified native enabled value text uses normal shell yellow `DAT_00AC18A4`, while disabled `DAT_00AC1CB4` is conditional and not normally reached for standard offline trackbars."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`: replace "combo/dropdown/checkbox text surfaces are not yet implemented" with "combo/dropdown/checkbox text surfaces are implemented in current split renderer, but inactive opponent row combo-face text is still missing a disabled-state color path and standard combo/dropdown long-string pre-truncation remains a parity risk."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`: keep the verified non-reachability claim; add "This does not excuse using button text color for normal enabled trackbar values; enabled value text still uses `DAT_00AC18A4`."

## Sources

- Prior Ghidra reports: `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`, `VALIDATION_MODAL_0X005D3490_DISABLED_START_UNDERLAY_TIMING_GHIDRA_REPORT.md`.
- Prior Ghidra evidence cited by those reports: `FUN_00621040 @ 0x00621040`, `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`, `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `OwnerDraw_Button_00612B70 @ 0x00612B70`, row update helpers `FUN_006ADC20` and `FUN_006ACD60`.
- Rust scanned read-only: `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render/text.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render/controls.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`.
