# Validation Modal Current Rust Visual Delta Recheck - Ghidra Research Report

**Address(es):** prior verified reports only: `0x006ACEE0`, `0x005D3490`, `0x006AE2C0`, `0x006AE3F0`, `0x00612B70`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** current Rust/source and existing-doc delta audit for the Skirmish Start validation modal, focused on visual/layout/input parity handoffs after ordinary failure functionality already exists.  
**Non-Scope:** new binary deep dive, exact native dialog template rectangles, native paint composition, Enter/Escape/default-button proof, retail screenshot capture, Random Map, Choose Map `0x6B`, successful launch/spawn, or Rust/doc implementation.  
**Confidence:** High for current Rust source status and stale-doc classification; Medium for native visual/input implications because slots 1-4 own the fresh binary/template/paint/keyboard/underlay facts.  
**Active in YR:** Rust-only for current-source findings; Yes/Conditional only where inherited from cited verified reports.

## 0. Working Notes

- Target question: What exact current Rust surfaces, tests, stale docs, and minimal handoffs remain for validation-modal visual/layout/input parity?
- Non-goals: Do not rediscover ordinary Start validation functional branches; do not investigate native template/paint/keyboard deeply; do not modify Rust or in-repo docs.
- Evidence needed to mark COMPLETE: exact Rust file:line evidence for modal state/mapping/layout/render/input/tests; exact doc-path evidence for stale claims; handoff items with acceptance scenarios and test-name proposals.
- Stop conditions: stop after source/docs delta is mapped; defer native template/paint/keyboard/underlay facts to slots 1-4 unless a direct contradiction appears.

## 1. Overview

Current Rust is not log-only for the three ordinary native Start validation failures. Active in YR comparison: Rust-only source state compared to active YR behavior. Evidence: `src/app.rs:657..668` calls `launch_session`, maps recognized errors, and calls `show_skirmish_validation_modal`; `src/app.rs:700..735` maps capacity/no-opponent/same-team to CSF-backed modal text plus OK; `src/app.rs:738..749` stores `validation_modal` and clears transient shell interaction state.

The remaining Rust-facing gaps are now narrower: the modal uses a hand-built centered primitive layout and solid panel/button renderer, app/render tests are sparse, and validation-modal Enter/Escape dismissal is missing. Sibling slots now refine the native target: ordinary Start validation uses RT_DIALOG `0xCE`, static `0x5B0`, ownerdraw OK `0x5AE`, mode-2 `PUDLGBGN.SHP`/`DIALOGN.PAL` background, and `MNBTTN.SHP`/`MAINBTTN.PAL` OK art; Rust's `push_button_30` flat panel is therefore functional but visually wrong. Active in YR comparison: Yes for native modal facts, Rust-only for current primitive status. Evidence: `VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md:123..124`; `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md:106..108`; Rust layout/render evidence below.

## 2. Current Rust Surfaces

| Surface | Current source state | Active in YR | Evidence |
|---|---|---|---|
| Error categories | `LaunchValidationError` has `NoEnabledOpponent`, `MapCapacityExceeded`, `SameExplicitTeam`, plus non-native/internal errors. | Rust-only | `src/skirmish_launch.rs:232..252` |
| Data validation | `launch_session` rejects capacity overflow, no active opponent, and same explicit team before session construction. | Rust-only source counterpart to active YR ordinary failures | `src/ui/skirmish_shell/state/launch.rs:87..114`; YR baseline `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md:38..40` |
| Modal state | `SkirmishValidationModalState` carries only visible body text and OK text. | Rust-only; compatible with active YR two-text ordinary calls, not template/control-id-complete | `src/ui/skirmish_shell/state.rs:156..168`; YR control-id baseline `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md:83` |
| App mapping | Start errors recognized by `skirmish_validation_modal_for_error` become `validation_modal`; unknown/internal errors still log. | Rust-only; ordinary mapped errors match active YR categories | `src/app.rs:657..671`, `src/app.rs:700..735` |
| Show modal side effects | Opening validation modal clears pressed ownerdraw button, dropdowns, scroll/drag, last painted pressed button, and player-name focus. | Rust-only approximation of native modal ownership/Start disable window | `src/app.rs:738..749` |
| Mouse input | Any mouse-down is consumed while modal exists; mouse-up clears only if cursor is inside the primitive OK rect. | Rust-only current behavior; native exact keyboard/default result deferred | `src/app.rs:1154..1172` |
| Parent input suppression | Parent mouse-down/up, hover/status, wheel, and shell Escape-close path are blocked while validation modal exists. | Rust-only current behavior; consistent with modal ownership | `src/app.rs:1175..1228`, `src/app.rs:1261..1291`, `src/app.rs:1510..1516`; `src/ui/skirmish_shell/state/hit_test.rs:40..49` |
| Primitive layout | `VALIDATION_MODAL_W/H` are fixed at `360x122`; message rect is inset `(24,24,312,42)`; OK is `(82x24)` centered at bottom. | Rust-only; not native-verified | `src/ui/skirmish_shell/layout.rs:35..36`, `src/ui/skirmish_shell/layout.rs:849..867` |
| Primitive render | Renderer draws Start disabled when modal exists, then draws a solid modal panel, outline, `push_button_30`, and centered text. Native slot-2 says this is visually wrong for `0xCE`. | Rust-only current state; active YR visual target differs | `src/app_skirmish_shell_render.rs:253..260`, `:308..310`, `:437..438`; `src/app_skirmish_shell_render/modals.rs:167..194`; slot-2 `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md:106..108` |
| Tests | Data-level failures and primitive layout/draw-order tests exist; app-level modal mapping/render/input tests are missing. | Rust-only | `src/ui/skirmish_shell/state/tests.rs:1100..1138`; `src/ui/skirmish_shell/layout.rs:1257..1264`; `src/app_skirmish_shell_render.rs:1093..1104` |

## 3. Existing Docs And Stale Claims

| Doc claim | Status | Evidence | Replacement / action |
|---|---|---|---|
| In-repo contract says Start validation errors "currently log instead of assigning `validation_modal`" and `rg` finds no setter. | Stale | `docs/contracts/2026-05-23-skirmish-ui-shell-implementation-contract.md:35..36`, `:53`; current setter at `src/app.rs:742` and constructor at `src/app.rs:735` | Replace with wording in section 9. |
| Older Start failure recheck says render path omits validation modal drawing. | Stale for current Rust snapshot | `SKIRMISH_START_FAILURE_SHELL_MODAL_CURRENT_RECHECK_GHIDRA_REPORT.md:21`, `:99`, `:131..142`; current render at `src/app_skirmish_shell_render.rs:308..310`, `:437..438` | Treat as old snapshot only; do not use for current implementation queue. |
| Current contract recheck says current Rust maps and renders primitive modal, but native pixels/keyboard are deferred. | Partly superseded by sibling slots | `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md:21`, `:89..93`, `:117..118`; slot-1/2/3 reports now resolve template, paint, and keyboard dismissal | Keep the current-Rust rows; replace the deferred native visual/keyboard wording. |
| Right-panel Start report says Start is not pre-disabled; only transient disabled under modal is optional pixel delta. | Current baseline | `SKIRMISH_RIGHT_PANEL_START_BUTTON_DISABLED_ENABLED_STATE_GHIDRA_REPORT.md:19`, `:100`, `:150..151`, `:175..176` | Preserve; do not pre-grey invalid setups. |

## 4. Coverage Ledger

| Area / branch | Status | Evidence | What remains |
|---|---|---|---|
| Rust ordinary validation categories | verified | `src/skirmish_launch.rs:232..252`; `src/ui/skirmish_shell/state/launch.rs:87..114` | none for data-level ordinary categories |
| App modal mapping | verified | `src/app.rs:657..735` | add app-level tests |
| Modal input blocking and OK mouse dismissal | verified | `src/app.rs:1154..1172`, `:1175..1228`, `:1261..1291`, `:1510..1516` | add tests; native keyboard/default result deferred |
| Primitive validation layout | verified Rust-only | `src/ui/skirmish_shell/layout.rs:35..36`, `:849..867`, `:1257..1264` | replace/adjust after slot-1 native template rects |
| Primitive validation render | verified Rust-only | `src/app_skirmish_shell_render.rs:253..260`, `:308..310`, `:437..438`; modal/text helpers | replace/adjust after slot-2 paint composition |
| Start disabled underlay | touched-not-exhausted | Rust passes disabled to Start when modal exists at `src/app_skirmish_shell_render.rs:253..260`; native transient baseline in Start-disabled report | slot-4 owns exact native timing/pixels |
| Keyboard/default dismissal | verified mismatch | current Escape suppression at `src/app.rs:1510..1516`; no Enter path found; native Enter/Escape dismissal proof in `VALIDATION_MODAL_0X005D3490_KEYBOARD_DEFAULT_DISMISSAL_GHIDRA_REPORT.md:110..111` | implement/test modal dismissal on Enter and Escape without closing parent |
| In-repo implementation contract | verified stale | `docs/contracts/2026-05-23-skirmish-ui-shell-implementation-contract.md:35..36`, `:53` | patch only when parent/user asks |

## 5. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does current Rust set validation modal state for ordinary native failures? -> Yes.` (evidence: `src/app.rs:657..668`, `src/app.rs:700..735`, `src/app.rs:742`)
- `[RESOLVED] OQ-02 - Does current Rust render validation modal instances/text? -> Yes, through primitive modal instance/text helpers.` (evidence: `src/app_skirmish_shell_render.rs:308..310`, `:437..438`; `src/app_skirmish_shell_render/modals.rs:167..194`; `src/app_skirmish_shell_render/text.rs:722..748`)
- `[RESOLVED] OQ-03 - Are current validation modal dimensions native-verified? -> No; they are hard-coded primitive Rust constants.` (evidence: `src/ui/skirmish_shell/layout.rs:35..36`, `:849..867`; native-rect deferral in baseline report `:92`)
- `[RESOLVED] OQ-04 - Does current Rust consume parent shell input while modal is open? -> Yes for mouse down/up, wheel, hover/status, and shell Escape-close.` (evidence: `src/app.rs:1154..1172`, `:1269..1291`, `:1510..1516`)
- `[RESOLVED] OQ-05 - What current tests exist? -> Data-level validation, primitive layout centering, and semantic overlay order only.` (evidence: `src/ui/skirmish_shell/state/tests.rs:1100..1138`; `src/ui/skirmish_shell/layout.rs:1257..1264`; `src/app_skirmish_shell_render.rs:1093..1104`)
- `[RESOLVED] OQ-06 - Which doc is materially stale for current Rust? -> The in-repo implementation contract still says log-only/no setter.` (evidence: `docs/contracts/2026-05-23-skirmish-ui-shell-implementation-contract.md:35..36`, `:53`)
- `[RESOLVED] OQ-07 - Exact native modal template/control rectangles? -> Sibling slot 1 resolves ordinary Start modal as RT_DIALOG `0xCE`, static `0x5B0`, OK `0x5AE`; optional controls `2`/`0x5AF` are not in this path.` (evidence: `VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md:123..124`)
- `[RESOLVED] OQ-08 - Exact native modal paint composition/assets/palette? -> Sibling slot 2 resolves the main visual target as `PUDLGBGN.SHP`/`DIALOGN.PAL` plus `MNBTTN.SHP`/`MAINBTTN.PAL` OK art.` (evidence: `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md:106..108`)
- `[RESOLVED] OQ-09 - Enter/Escape/default-button behavior inside `0x005D3490`? -> Sibling slot 3 resolves native Enter and Escape dismissal through `IsDialogMessageA` translated commands; current Rust is missing this.` (evidence: `VALIDATION_MODAL_0X005D3490_KEYBOARD_DEFAULT_DISMISSAL_GHIDRA_REPORT.md:110..111`)
- `[RESOLVED] OQ-10 - Disabled Start timing? -> Sibling slot 4 resolves that Start is disabled before validation modal and re-enabled after return; exact visible underlay remains runtime-only.` (evidence: `VALIDATION_MODAL_0X005D3490_DISABLED_START_UNDERLAY_TIMING_GHIDRA_REPORT.md:164..166`)

## 6. Implementation Handoff

| Verified behavior / source state | Evidence | Rust delta | Affected Rust surface | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary validation failures already create a modal state with CSF/localized body and OK text. Active in YR: Yes by prior reports; current source Rust-only. | `src/app.rs:657..735`; baseline native text `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md:38..40` | Missing app-level regression tests proving no launch and visible modal mapping. | `src/app.rs`; app/skirmish shell test harness if available | Capacity 2 with local+2 AIs stays in shell, modal body contains `2 player max`, OK text is `OK`, no launch transition. | `skirmish_start_capacity_error_sets_validation_modal_text_and_stays_in_shell` | Do not resurrect log-only behavior or append non-native requested/capacity suffix. |
| Modal currently blocks parent mouse/wheel/status/Escape-close and OK mouse-up clears only inside OK rect. Active in YR comparison: modal ownership Yes, but native Enter/Escape also dismiss. | `src/app.rs:1154..1172`, `:1269..1291`, `:1510..1516`; native `VALIDATION_MODAL_0X005D3490_KEYBOARD_DEFAULT_DISMISSAL_GHIDRA_REPORT.md:110..111` | Missing Enter/Escape dismissal and focused input tests. | `src/app.rs`; `src/ui/skirmish_shell/state/hit_test.rs` | Invalid setup modal open: Enter or Escape clears only the modal, does not close parent shell or launch; OK mouse click still clears. | `skirmish_validation_modal_enter_escape_dismiss_without_parent_close` | Do not route Enter to Start behind the modal; do not keep current Escape consume-without-dismiss behavior. |
| Native modal layout/render is RT_DIALOG `0xCE`, static `0x5B0`, OK `0x5AE`, `PUDLGBGN.SHP`/`DIALOGN.PAL` background, and `MNBTTN.SHP`/`MAINBTTN.PAL` OK art; current Rust is primitive `360x122`, solid panel/outline, generic 30px button. Active in YR: Yes for native target; Rust-only current mismatch. | slot-1 `VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md:123..124`; slot-2 `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md:106..108`; Rust `src/ui/skirmish_shell/layout.rs:35..36`, `:849..867`; `src/app_skirmish_shell_render/modals.rs:167..194` | Replace/parameterize layout and paint; pack/load the native SHP/PAL roles when available. | `src/ui/skirmish_shell/layout.rs`; `src/app_skirmish_shell_render/modals.rs`; `src/app_skirmish_shell_render/text.rs`; chrome atlas/draw-order tests | No-opponent modal at 800x600 uses resource-derived control roles and native modal assets, not a flat panel or `push_button_30`. | `skirmish_validation_modal_uses_resource_0xce_and_native_assets` | Do not tune modal pixels by eye; do not use Skirmish 30-family PCX button art for OK `0x5AE`. |

## 7. Negative Facts / Do Not Do

- Do not say current Rust ordinary Start validation is log-only. Active in YR comparison: Rust-only current source; evidence: `src/app.rs:657..668`, `src/app.rs:735`, `src/app.rs:742`.
- Do not use `SKIRMISH_START_FAILURE_SHELL_MODAL_CURRENT_RECHECK_GHIDRA_REPORT.md` as current Rust render evidence where it says validation modal drawing is missing. Active in YR comparison: Rust-only stale-doc issue; evidence: stale report `:99`, current render `src/app_skirmish_shell_render.rs:308..310`, `:437..438`.
- Do not pre-disable/grey Start for invalid setups before click. Active in YR: No for standard offline shell; evidence: `SKIRMISH_RIGHT_PANEL_START_BUTTON_DISABLED_ENABLED_STATE_GHIDRA_REPORT.md:19`, `:175..176`.
- Do not make every selected-mode false return show the generic validation modal. Active in YR: Conditional; evidence: current baseline report `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md:40`, `:103`.
- Do not claim primitive validation modal layout/panel/button art is native-pixel verified. Active in YR comparison: Rust-only mismatch; evidence: `src/ui/skirmish_shell/layout.rs:35..36`, `:849..867`; native slot-1/2 evidence `VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md:123..124`, `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md:106..108`.
- Do not keep Escape as "consume but leave modal open" for validation modal parity. Active in YR: Yes for dismissal; evidence: `VALIDATION_MODAL_0X005D3490_KEYBOARD_DEFAULT_DISMISSAL_GHIDRA_REPORT.md:110..111`.

## 8. Remaining Uncertainty

- Runtime screenshot/RGB verification remains open for the native `PUDLGBGN.SHP`/`MNBTTN.SHP` modal composition.
- Exact DLU-to-final-pixel conversion/capture should be verified before asserting screenshot-perfect modal coordinates.
- Exact visible disabled Start underlay pixels remain runtime-only; slot 4 verifies timing but not whether the player sees underlay pixels.

## 9. Stale Docs / Replacement Wording

- `docs/contracts/2026-05-23-skirmish-ui-shell-implementation-contract.md:36`: replace "Current action routing: Choose Map opens modal; Start validation errors currently log instead of assigning `validation_modal`." with "Current action routing: Choose Map opens modal; ordinary Start validation errors map to `SkirmishValidationModalState` and request a redraw, while unrecognized/internal launch errors still log."
- `docs/contracts/2026-05-23-skirmish-ui-shell-implementation-contract.md:53`: replace the `Current Rust behavior` sentence "Data checks exist in `launch_session`; render helpers for `validation_modal` exist. Current app action still only logs `LaunchValidationError`, and no source setter for `validation_modal` exists." with "Data checks exist in `launch_session`; current app code maps `MapCapacityExceeded`, `NoEnabledOpponent`, and `SameExplicitTeam` to `SkirmishValidationModalState`; the renderer draws a primitive validation modal and Start underlay disabled state while it is open. Remaining deltas are native template/paint/keyboard parity and focused app/render/input tests."
- `docs/research/skirmish-ui/SKIRMISH_START_FAILURE_SHELL_MODAL_CURRENT_RECHECK_GHIDRA_REPORT.md`: replace any current-Rust handoff wording that says validation modal rendering is missing with "Current Rust now renders validation modal instances/text through primitive helpers; this older report reflects a prior source snapshot and remains useful for native failure branch/text evidence, not current renderer status."
- `docs/research/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`: replace "Native keyboard behavior for validation modal Enter/Escape/default buttons is not fully proven by this slot." with "Native validation-modal keyboard dismissal is now verified: `0x005D3490` registers the modal for `IsDialogMessageA`; translated `IDOK (1)` and `IDCANCEL (2)` commands dismiss the modal, and Rust should close `validation_modal` on Enter/Escape while keeping the parent shell open."

## Sources

- Rust scanned: `src/app.rs`; `src/skirmish_launch.rs`; `src/ui/skirmish_shell/state.rs`; `src/ui/skirmish_shell/state/launch.rs`; `src/ui/skirmish_shell/state/tests.rs`; `src/ui/skirmish_shell/state/hit_test.rs`; `src/ui/skirmish_shell/layout.rs`; `src/app_skirmish_shell_render.rs`; `src/app_skirmish_shell_render/modals.rs`; `src/app_skirmish_shell_render/text.rs`; `src/app_skirmish_shell_render/draw_order.rs`.
- Reports/docs referenced: `docs/research/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_RIGHT_PANEL_START_BUTTON_DISABLED_ENABLED_STATE_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_START_FAILURE_SHELL_MODAL_CURRENT_RECHECK_GHIDRA_REPORT.md`; `docs/contracts/2026-05-23-skirmish-ui-shell-implementation-contract.md`.
