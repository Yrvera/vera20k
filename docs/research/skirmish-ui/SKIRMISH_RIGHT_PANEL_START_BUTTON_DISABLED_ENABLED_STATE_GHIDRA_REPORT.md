# Skirmish Right-Panel Start Button Disabled / Enabled State - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x006AE6E0`, `0x006AE3F0`, `0x006ADC20`, `0x006ACD60`, `0x00612B70`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** active standard YR offline Skirmish dialog `0x102` Start Game button `0x617` enabled/disabled lifecycle before launch, the conditions that change that state, disabled draw/input consequence, and current Rust handoff.  
**Non-Scope:** Start/Choose/Back rect geometry, full validation modal pixel/template parity, successful launch/session packing after the Start boundary, Choose Map modal, random map generation, and gameplay spawn behavior.  
**Confidence:** High for standard offline `0x102` lifecycle, transient disable/re-enable conditions, ownerdraw disabled visual/input consequences, and current Rust status; Medium for exact behind-modal disabled pixels because no runtime retail screenshot was captured.  
**Active in YR:** Yes for the standard offline shell path; disabled Start is Conditional during the synchronous Start-click validation/modal window, not a persistent preflight state.

## 0. Working Notes

- Target question: Is right-panel Start Game `0x617` ever disabled/greyed before launch, what conditions change it, how does disabled draw/input differ, and should Rust pre-disable Start or keep click-to-validate?
- Non-goals: do not redo button pixel geometry except disabled effect; do not re-investigate validation text except lifecycle boundaries; do not modify Rust/INI/in-repo docs.
- Evidence needed to mark COMPLETE: active `0x102` init/command path, all standard `EnableWindow` writers touching or plausibly touching Start, disabled ownerdraw branch, current Rust render/input state, negative facts, and implementation handoff.
- Stop conditions: stop once standard `0x102` init, row/mode/map update helpers, Start command branch, and ownerdraw disabled branch are classified; arbitrary external Win32 mutation and online dialogs are out of scope.

## 1. Overview

Standard offline YR Skirmish does **not** grey or suppress Start before the player clicks it, even when the current setup will fail validation. Start is an enabled ownerdraw Button in the normal shell; the `0x617` command branch disables it only after a Start click with notification `0`, validates the setup, and re-enables it after any blocking validation modal returns. Active in YR: Yes. Evidence: `0x006AE2C0`, `0x006AE3F0`, `0x006ACF7B..0x006AD34B`.

The only verified disabled Start visual in this slice is transient: while the native blocking modal is active, the underlying Start HWND has `WS_DISABLED`. The ownerdraw button callback renders disabled buttons with released art plus alpha overlay `0x80`, suppresses pressed/down art, and suppresses the pressed-paint `GenericClick` transition. Active in YR: Conditional. Evidence: `0x00612B70` decompile, especially `0x00613254..0x00613262` and `0x006135F3..0x0061361B` from prior ownerdraw assembly evidence.

## 2. Class Layout / Key Offsets

| Field / control | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Dialog `0x102` | offline Skirmish setup dialog pumped by `FUN_006AE2C0` | `0x006AE2C0`, `0x006AE3F0` | Yes |
| Button `0x617` | right-panel Start Game command button | `0x006ACEE0`, resource/button reports | Yes |
| `WS_DISABLED 0x08000000` | Win32 disabled style read by ownerdraw callback | `GetWindowLongA(GWL_STYLE)` in `0x00612B70` | Conditional |
| Ownerdraw state bit low `& 1` | pressed/down state source for ownerdraw paint | `0x00612B70` | Yes |
| owner byte `+0xBC` / decompile `piVar17[0x2f]` | suppresses mouse sound/paint for that control state, separate from Win32 disabled | `0x00612B70` | Conditional |
| `DAT_00833684` | global last button paint state char for pressed-transition sound | `0x00612B70` | Yes |

## 3. Core Logic

### 3.1 Normal shell leaves Start enabled

The active shell launcher creates/pumps dialog `0x102`; `WM_COMMAND` is routed through `0x006AE3F0` into `0x006ACEE0`. Init `0x006AE6E0` populates player rows, selected mode/map, sliders, checkboxes, preview, and row sibling enable states. No init branch disables Start `0x617`.

Active in YR: Yes. Evidence: `0x006AE2C0` shell loop; `0x006AE3F0` command route; `0x006AE6E0` decompile has `GetDlgItem`/`EnableWindow` calls for inactive row sibling controls and team support, plus slider/checkbox setup, but no `GetDlgItem(param_1,0x617)` / `EnableWindow(...,0)` init path.

### 3.2 Row and mode changes do not pre-disable Start

AI row changes route through `0x006ADC20`, which enables/disables that row's country/color/start/team sibling controls based on AI row item data `0/1/2` vs inactive. Selected-mode team support routes through `0x006ACD60`, which enables/disables Team controls (`0x76D` and row team controls) based on selected mode team support and inactive rows. Neither helper targets Start `0x617`.

Active in YR: Yes. Evidence: `0x006ACEE0` row-type cases call `FUN_006ADC20(); FUN_006ACD60();`; decompile `0x006ADC20` calls `EnableWindow` only on IDs returned by side/color/start/team helper functions; decompile `0x006ACD60` calls `GetDlgItem(param_1,0x76D)` and row team control IDs, never `0x617`.

### 3.3 Start click disables before validation

`0x006ACEE0` accepts Start/Back only when notification high word is zero. For Start `0x617`, before any map capacity, min-player, same-team, or selected-mode acceptance validation, it calls:

1. `GetDlgItem(hwnd, 0x617)`
2. `EnableWindow(start_hwnd, 0)`

Then it counts active AI rows, stores the count to `DAT_00A8B274`, and runs the ordinary validation sequence.

Active in YR: Yes. Evidence: decompile `0x006ACEE0`; branch `param_2 == 0x617` immediately calls `GetDlgItem(param_1,0x617)` and `EnableWindow(...,0)` after the notification gate, before `0x006ACFBD` row counting.

### 3.4 Blocking failures re-enable Start after the modal

Every ordinary blocking failure calls `FUN_005D3490`, then re-enables Start with `EnableWindow(GetDlgItem(hwnd,0x617),1)`, then returns before launch packing:

| Failure | Re-enable evidence | Active in YR |
|---|---|---|
| map capacity too small | `0x006AD05B..0x006AD0DA`; re-enable at `0x006AD0CB..0x006AD0DA` | Yes |
| fewer than two total players | `0x006AD0ED..0x006AD159`; re-enable at `0x006AD14A..0x006AD159` | Yes |
| all active players on same explicit team | `0x006AD16C..0x006AD2A7`; re-enable at `0x006AD298..0x006AD2A7` | Yes |
| selected-mode false plus output dword `0x617` | `0x006AD2BA..0x006AD34B`; re-enable at `0x006AD31B..0x006AD32A` | Conditional |

Successful Start does not re-enable Start before packing/result write because the shell is transitioning out. Active in YR: Yes. Evidence: after validation, `0x006AD34B` begins session/global packing and later writes the dialog result pointer; no intervening `EnableWindow(0x617,1)` exists in the success path.

### 3.5 Disabled draw/input consequence

Ownerdraw paint reads `GWL_STYLE`. If `WS_DISABLED` is set, the callback forces state char back to released `'u'`; the pressed/down art is not selected. The disabled path still draws the enabled/released `bue_*30.pcx` family and label, then applies `AlphaBlendRect(..., 0x80)` over the button rect. It also suppresses the enabled pressed-paint transition sound because the `GenericClick` branch is gated by `(style & WS_DISABLED) == 0`.

Active in YR: Conditional, when `0x617` is disabled by the Start branch or some external/nonstandard code. Evidence: `0x00612B70` decompile; prior ownerdraw assembly ranges `0x00613254..0x00613262`, `0x00613264..0x0061329B`, `0x006135F3..0x0061361B`.

## 4. INI Keys

No INI key was found that gates Start `0x617` enabled/disabled state in standard offline Skirmish. The validation inputs come from selected map capacity, row combo item data, team controls, and selected mode object acceptance, not an INI-driven "Start enabled" flag.

Active in YR: Yes for absence in the scoped flow. Evidence: `0x006ACEE0` and related helper decompiles; prior MPModes reports for selected mode acceptance; no relevant `rulesmd.ini`/`artmd.ini` Start-button enable key.

## 5. Integration Points

| Function | Role | Status | Active in YR |
|---|---|---|---|
| `0x006AE2C0` | creates/pumps offline Skirmish dialog `0x102`, returns true only for result `0x617` | verified | Yes |
| `0x006AE3F0` | common dialog proc; routes `WM_COMMAND` to `0x006ACEE0` and handles status overrides | verified | Yes |
| `0x006AE6E0` | init/update population of row controls, mode/map, sliders/checks/preview | verified for no Start disable | Yes |
| `0x006ACEE0` | command handler; only verified standard Start disable/re-enable writer | verified | Yes |
| `0x006ADC20` | AI row change sibling enable/disable helper | verified negative for Start | Yes |
| `0x006ACD60` | team-control enable/disable helper | verified negative for Start | Yes |
| `0x00612B70` | shared ownerdraw button callback; disabled visual/input consequence | verified | Yes / Conditional |

## 6. Current Rust Implementation Status

Current Rust keeps Start enabled in the normal shell and validates on click. That matches the verified native pre-click behavior. Start is hit-tested first in `src/ui/skirmish_shell/state.rs`, `apply_action` returns `StartGame`, and `src/app.rs::handle_skirmish_shell_action` runs `launch_session` only after release-inside.

Rust now maps ordinary validation errors to `SkirmishValidationModalState`, renders validation modal instances/text, clears pressed Start state, and consumes modal mouse input. That is no longer the older log-only mismatch. The remaining scoped delta is not "pre-disable Start"; it is only the native transient behind-modal disabled style/alpha if pixel parity requires the underlying Start button to draw disabled while `validation_modal` is open.

| Rust surface | Current status | Delta vs binary |
|---|---|---|
| `src/ui/skirmish_shell/state.rs::hit_test_owner_draw_button` / `hit_test` | Start remains clickable whenever no modal owns input | matches normal enabled state |
| `src/app.rs::handle_skirmish_shell_action` | validates on Start click; shows modal for ordinary errors | matches click-to-validate contract for ordinary failures |
| `src/app.rs::show_skirmish_validation_modal` | clears pressed button/dropdowns/drag and blurs edit | matches no-double-fire intent, not literal Win32 disabled style |
| `src/app.rs::handle_validation_modal_mouse_down/up` | consumes modal input; OK clears modal | matches practical modal input suppression |
| `src/app_skirmish_shell_render.rs::push_right_panel_button_shp` call sites | Start render passes pressed state only; no disabled Start flag tied to modal-open state | optional pixel delta: native underlying Start would be disabled while modal is active |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target / non-goals / stop conditions | verified | section 0 | none |
| Standard `0x102` liveness | verified | `0x006AE2C0`, `0x006AE3F0` | none |
| Init path Start enabled state | verified | `0x006AE6E0` no `0x617` disable | none |
| Row change helper | verified | `0x006ADC20` | none for Start |
| Mode/team helper | verified | `0x006ACD60` | none for Start |
| Start notification gate | verified | `0x006ACEE0` | none |
| Initial Start disable on click | verified | `0x006ACF92..0x006ACF9E` | none |
| Ordinary failure re-enable | verified | `0x006AD0CB..0x006AD32A` | none |
| Success path no re-enable before transition | verified | `0x006AD34B` onwards | detailed packing out of scope |
| Disabled ownerdraw visual/input | verified | `0x00612B70`; prior assembly ranges | final display pixels require runtime capture |
| Current Rust pre-click behavior | verified | `src/ui/skirmish_shell/state.rs`, `src/app.rs` | none |
| Current Rust behind-modal disabled tint | verified missing/optional | `src/app_skirmish_shell_render.rs` call sites | implement only if targeting exact native modal-underlay pixels |
| Online/multiplayer lobby Start buttons | deferred | out of scope | separate investigation if needed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x617` active in standard YR offline Skirmish? -> Yes; `0x006AE2C0` creates/pumps `0x102` and returns true for result `0x617`.` (evidence: `0x006AE2C0`)
- `[RESOLVED] OQ-02 - Is Start pre-disabled at init based on invalid current setup? -> No; `0x006AE6E0` initializes other controls and row siblings but has no Start disable branch.` (evidence: `0x006AE6E0`)
- `[RESOLVED] OQ-03 - Do AI row changes disable Start? -> No; row changes disable/enable only that row's sibling controls.` (evidence: `0x006ADC20`)
- `[RESOLVED] OQ-04 - Do selected mode/team changes disable Start? -> No; `0x006ACD60` targets team controls only.` (evidence: `0x006ACD60`)
- `[RESOLVED] OQ-05 - What condition disables Start? -> A Start `0x617` command with notification high word `0` disables it before validation.` (evidence: `0x006ACF7B..0x006ACF9E`)
- `[RESOLVED] OQ-06 - Is disabled Start persistent after ordinary validation failure? -> No; each blocking modal branch re-enables Start before returning.` (evidence: `0x006AD0CB..0x006AD32A`)
- `[RESOLVED] OQ-07 - Does successful Start re-enable Start before launch? -> No; it falls through to packing/result write and shell transition.` (evidence: `0x006AD34B` onward)
- `[RESOLVED] OQ-08 - How does disabled draw differ? -> released art, no pressed art, alpha overlay `0x80`, and pressed-paint sound suppression.` (evidence: `0x00612B70`)
- `[RESOLVED] OQ-09 - Does disabled Start suppress click input? -> Yes through Win32 disabled HWND/modal behavior during native failure handling; current Rust consumes input while validation modal is present.` (evidence: `0x006ACEE0`, `src/app.rs` modal handlers)
- `[RESOLVED] OQ-10 - Should Rust grey Start before invalid setups? -> No; native validates on click and only transiently disables after the click.` (evidence: `0x006ACF92..0x006AD32A`)
- `[RESOLVED] OQ-11 - What is the current Rust delta? -> normal click-to-validate and modal input suppression are implemented; optional behind-modal disabled Start render is not wired.` (evidence: source scan)
- `[DEFERRED] OQ-12 - Exact retail disabled Start pixels under the modal.` (category: needs-runtime-debugger; reason: binary proves alpha value/path, but final DirectDraw display-format pixels require retail capture; next-step-if-pursued: capture a native validation modal frame with underlying Start visible)
- `[DEFERRED] OQ-13 - Online host/guest lobby Start enable policy.` (category: out-of-scope; reason: this report is standard offline `0x102`; next-step-if-pursued: separate WOL/host lobby investigation)
- `[DEFERRED] OQ-14 - Arbitrary external code forcing `0x617` disabled outside Start click.` (category: out-of-scope; reason: no standard offline writer found and arbitrary Win32 mutation is not stock shell behavior; next-step-if-pursued: debugger watchpoint on Start style)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Start is enabled in the normal shell even when the current setup will fail validation; validation happens on click. | `0x006AE6E0`, `0x006ACEE0` | none observed for normal behavior | `src/ui/skirmish_shell/state.rs`, `src/app.rs` | keep Start clickable whenever no modal owns input; do not precompute invalid setup to grey/suppress Start | all AI rows disabled: Start still draws/clicks normally, click opens min-player modal | Do not grey Start based on no opponents, map capacity, same team, or missing future mode checks |
| Start disables only after `0x617` click notification `0`, before validation, then re-enables after blocking modal branches. | `0x006ACF92..0x006ACF9E`, `0x006AD0CB..0x006AD32A` | practical modal suppression implemented; literal transient disabled flag not modeled | `src/app.rs`, `src/app_skirmish_shell_render.rs` | continue clearing pressed state and consuming modal input; optional pixel work can render underlying Start disabled while validation modal is open | double-click invalid Start creates one modal, no launch; OK dismisses, second Start click repeats modal | Do not leave Start permanently disabled after OK |
| Disabled ownerdraw Start uses released art plus alpha overlay, never `bud_*` or down art, and suppresses pressed-paint sound. | `0x00612B70`, prior `0x00613254..0x0061361B` | optional missing behind-modal disabled visual | `src/app_skirmish_shell_render.rs::push_right_panel_button_shp`, button sound transition state | if implementing exact native underlay, feed `disabled = validation_modal.is_some()` only for Start during blocking modal, using released art + alpha | force capacity failure: modal appears; underlying Start, if visible, is released/dimmed and emits no additional button click sound | Do not introduce pre-click disabled visual; do not use `bud_*30.pcx` |

Proposed Rust test names:

- `skirmish_start_remains_enabled_until_clicked_even_when_invalid`
- `skirmish_start_validation_modal_consumes_double_click_and_reenables_after_ok`
- `skirmish_start_modal_underlay_uses_disabled_released_button_visual`

## 10. Negative Facts / Do Not Do

- Do not pre-disable/grey Start for invalid map capacity, no enabled opponents, same explicit team, or selected-mode rejection. Active in YR: No; evidence: no standard pre-click writer in `0x006AE6E0`, `0x006ADC20`, or `0x006ACD60`; validation happens after `0x617` click in `0x006ACEE0`.
- Do not suppress Start clicks in the normal shell because validation would fail. Active in YR: No; evidence: native click path starts by disabling then validating.
- Do not leave Start disabled after a blocking validation failure. Active in YR: Yes; evidence: re-enable calls after each modal branch.
- Do not switch disabled Start to `bud_*30.pcx` or pressed art. Active in YR: No; evidence: ownerdraw disabled path forces released `'u'` and applies alpha overlay.
- Do not treat row sibling disabled state or Team control disabled state as Start disabled state. Active in YR: No for Start; evidence: `0x006ADC20` and `0x006ACD60` target different controls.

## 11. Remaining Uncertainty

- Exact display-format pixels for the transient disabled Start under the native modal were not runtime-captured; binary evidence is sufficient for behavior and alpha path, but not final screenshot RGB.
- Online host/guest lobby Start enable policies are not covered.
- Arbitrary debugger/modded Win32 mutation of the Start HWND is not covered; no stock offline `0x102` writer beyond the Start command branch was found.

## Stale Docs / Follow-up Docs

- Replace stale wording that says current Rust Start validation is "log-only" with: "Current Rust maps ordinary Start validation errors to visible `SkirmishValidationModalState`, renders the modal, clears pressed button state, and consumes modal input; remaining scoped delta is native modal visual parity and optional behind-modal disabled Start tint."
- Replace broad handoff wording "disabled Start visual state remains unchecked/missing" with: "Disabled Start is not a pre-click shell state; it is transient during native Start validation/modal handling. Rust should keep Start enabled before click and only model disabled visual under the validation modal if exact underlay pixel parity is pursued."

## Sources

- Ghidra read-only decompile: `0x006ACEE0`, `0x006AE3F0`, `0x006AE6E0`, `0x006AE2C0`, `0x00612B70`, `0x0060F9A0`, `0x006ADC20`, `0x006ACD60`.
- Prior reports referenced: `SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_ASSEMBLY_RIGHT_PANEL_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_PUSH_BUTTON_SOUNDS_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_SESSION_PACKING_BROAD_RECHECK_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs`.
