# Skirmish Trackbar Disabled Runtime Enable Flow - Ghidra Research Report

**Address(es):** `FUN_006AE2C0 @ 0x006AE2C0`, `FUN_006AE3F0 @ 0x006AE3F0`, `FUN_006AE6E0 @ 0x006AE6E0`, `FUN_006ACEE0 @ 0x006ACEE0`, `FUN_006ACD60 @ 0x006ACD60`, `FUN_006ADC20 @ 0x006ADC20`, `FUN_006ADDF0 @ 0x006ADDF0`, `FUN_006AE080 @ 0x006AE080`, `FUN_006ADF00 @ 0x006ADF00`, `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Runtime flow that disables/enables standard offline Skirmish `0x102` trackbars `0x529` Game Speed, `0x511` Credits, and `0x50C` Unit Count, including `EnableWindow`/style writers, parent command/update branches, selected mode/map side effects, and liveness in normal standard YR setup.
**Non-Scope:** Trackbar geometry, value mapping, changed-value sound, combo/listbox disabled visuals, row-sibling control behavior except where needed to prove trackbar non-targeting, online host/guest dialogs except as negative ID-search context.
**Confidence:** High for standard offline `0x102` non-reachability of disabled trackbar states; High for existence of conditional disabled paint inside the shared trackbar callback; Medium for global binary-wide "no writer anywhere" because this pass exhausts standard `0x102` paths and ID/search evidence, not every non-Skirmish dialog.
**Active in YR:** Yes for the standard offline `0x102` paths and enabled trackbars; Conditional for disabled trackbar paint only if some nonstandard caller disables the HWND.

## 0. Working Notes

- Target question: Do standard offline Skirmish trackbars `0x529`, `0x511`, and `0x50C` have any reachable runtime disable/enable flow?
- Non-goals: Do not re-audit trackbar geometry, value-change sound, notification order, unrelated controls, online host/guest sliders, or Rust implementation patches.
- Evidence needed to mark COMPLETE: binary evidence for `EnableWindow`/style writers targeting these IDs, parent command/update branches, selected mode/map side effects, and normal-YR liveness/default setup.
- Stop conditions: Stop after every material standard `0x102` writer candidate is resolved or explicitly deferred; write only this report; make no Rust/INI/in-repo-doc edits.

## 1. Overview

The standard offline Skirmish setup creates and initializes all three trackbars as enabled controls, then reads their values on Start/Back. The standard runtime update paths that disable or re-enable controls target AI-row sibling combos/statics/buttons and the Start button, not `0x529`, `0x511`, or `0x50C`.

Active in YR: Yes. Evidence: `FUN_006AE2C0` opens the offline Skirmish dialog loop; `FUN_006AE3F0` dispatches init/command/paint; `FUN_006AE6E0 @ 0x006AECB3..0x006AEDA0` initializes the three scoped trackbars; `FUN_006ACEE0 @ 0x006AD709..0x006AD77C` reads them before launch/back application.

## 2. Key Controls And State

| Control | Role | Standard init | Runtime writer found | Active in YR |
|---:|---|---|---|---|
| `0x529` | Game Speed trackbar | `TBM_SETRANGE 0..6`, `TBM_SETPOS 6 - DAT_00A8B268` | no `EnableWindow`/style writer in standard `0x102`; value read on apply | Yes; enabled |
| `0x511` | Credits trackbar | `TBM_SETRANGE Rules +0x1480..+0x1488`, `TBM_SETPOS DAT_00A8B25C`, `0x4AB` step from Rules `+0x148C` | no `EnableWindow`/style writer in standard `0x102`; value read on apply | Yes; enabled |
| `0x50C` | Unit Count trackbar | `TBM_SETRANGE Rules +0x1490..+0x1498`, `TBM_SETPOS DAT_00A8B270` | no `EnableWindow`/style writer in standard `0x102`; value read on apply | Yes; enabled |
| `WS_DISABLED 0x08000000` | Win32 disabled style bit read by trackbar paint | not set by standard init for these controls | paint reads only current style via `GetWindowLongA(GWL_STYLE)` | Conditional |
| state `+0x108` | trackbar sound suppression byte | default zero for standard init | custom message `0x4AE`; not disabled-window state | Conditional; not a disable flow |

## 3. Core Logic

### 3.1 Dialog dispatch does not route `WM_HSCROLL` to a trackbar disable branch

`FUN_006AE3F0` first calls common shell handling `FUN_00622B50`, then handles only:

- `0x497`: calls `FUN_006AE6E0` init.
- `0x0F`: paint/preview path.
- `0x111`: command path, calls `FUN_006ACEE0`.
- `0x4E9`: tooltip/update text path for hovered controls.

There is no `0x114 WM_HSCROLL` branch in this dialog proc. Trackbar `OwnerDraw_Trackbar_0061D950` still sends parent `WM_HSCROLL`, but this standard Skirmish parent does not use it to enable or disable other controls.

Evidence: decompile `FUN_006AE3F0`; prior sound report assembly `0x0061E692..0x0061E6AF` for the trackbar send. Active in YR: Yes.

### 3.2 Init seeds values and ranges only

`FUN_006AE6E0` obtains each trackbar by `GetDlgItem`, null-checks it, then sends only setup messages:

- `0x529`: `0x406` range with `0x60000`, `0x405` position with `6 - DAT_00A8B268`.
- `0x511`: `0x406` range from Rules money min/max, `0x405` position from `DAT_00A8B25C`, `0x4AB` step from Rules money increment.
- `0x50C`: `0x406` range from Rules unit min/max, `0x405` position from `DAT_00A8B270`.

No `EnableWindow` call or style write occurs in these three trackbar blocks. The `EnableWindow` calls in init are for row sibling controls after inactive row selection, not for these trackbar IDs.

Evidence: decompile `FUN_006AE6E0`; assembly contexts `0x006AECB3`, `0x006AECFF`, `0x006AED5B`, `0x006AED85`. Active in YR: Yes.

### 3.3 Start/Back reads values and does not change enabled state

On Start `0x617` and Back `0x5C0`, `FUN_006ACEE0` reads the three trackbars late in the successful path:

- `0x529` `0x400` current position -> stored as `6 - visual`.
- `0x511` `0x400` current position -> stored as credits.
- `0x50C` `0x400` current position -> stored as unit count.

The nearby `EnableWindow` calls in this same function only disable/re-enable Start `0x617` around validation failures. They do not target `0x529`, `0x511`, or `0x50C`.

Evidence: decompile `FUN_006ACEE0`; assembly `0x006AD709..0x006AD77C` reads the trackbars, while `0x006ACF92..0x006ACF9E`, `0x006AD0CB..0x006AD0DA`, `0x006AD14A..0x006AD159`, `0x006AD298..0x006AD2A7`, and `0x006AD31B..0x006AD32A` target Start `0x617`. Active in YR: Yes.

### 3.4 AI row and mode/map side effects do disable controls, but only row siblings

The standard runtime disable/enable flow is real, but it belongs to AI-row sibling controls:

- `FUN_006ADC20` reacts to AI row-state changes and calls `EnableWindow` on row country/color/start/team controls returned by `FUN_004E41D0`, `FUN_004E37D0`, `FUN_004E4E60`, and `FUN_004E5940`.
- `FUN_006ACD60` updates start/team-row enabled state from the selected mode's team flag at `DAT_00A8B23C + 0x3C`.
- `FUN_006ADDF0`, `FUN_006AE080`, and `FUN_006ADF00` hide/show excess AI rows and then call the row-sibling update path.
- The Choose Map `0x5AA` success path in `FUN_006ACEE0` rebuilds map/mode state and calls `FUN_006ACD60`; it does not target trackbar controls.

No decompiled row/mode/map side-effect function obtains `GetDlgItem(..., 0x529/0x511/0x50C)` for `EnableWindow`.

Evidence: decompile `FUN_006ADC20`, `FUN_006ACD60`, `FUN_006ADDF0`, `FUN_006AE080`, `FUN_006ADF00`, and `FUN_006ACEE0` Choose Map branch. Active in YR: Yes for row-sibling controls; No for scoped trackbar disablement.

### 3.5 Disabled paint exists but is not reached by standard Skirmish flow

`OwnerDraw_Trackbar_0061D950` reads `GetWindowLongA(hwnd, GWL_STYLE)` during paint and masks `0x08000000`. If the bit is set, the thumb receives an alpha overlay, the rail primitive color switches to `DAT_00AC1CA8`, and the value text color switches to `DAT_00AC1CB4`. This is a valid shared owner-draw branch, but this investigation found no standard offline Skirmish runtime path that sets the bit for the three scoped controls.

Evidence: decompile `OwnerDraw_Trackbar_0061D950`; prior geometry report cites assembly `0x0061E0B0..0x0061E1B9` and `0x0061E2A1..0x0061E2B6`. Active in YR: Conditional; not reached by normal standard `0x102` setup for these trackbars.

## 4. INI Keys

| Key | Stock YR value | Binary use in this slice | Disables trackbar? | Active in YR |
|---|---:|---|---|---|
| `[MultiplayerDialogSettings] GameSpeed` | `1` in `rulesmd.ini` | default stored speed, visual initialized as `6 - stored` | No | Yes |
| `[MultiplayerDialogSettings] MinMoney` | `5000` | credits range minimum | No | Yes |
| `[MultiplayerDialogSettings] Money` | `10000` | credits default | No | Yes |
| `[MultiplayerDialogSettings] MaxMoney` | `10000` | credits range maximum | No | Yes |
| `[MultiplayerDialogSettings] MoneyIncrement` | `100` | credits step via message `0x4AB` | No | Yes |
| `[MultiplayerDialogSettings] MinUnitCount` | `0` | unit-count range minimum | No | Yes |
| `[MultiplayerDialogSettings] UnitCount` | `10` | unit-count default | No | Yes |
| `[MultiplayerDialogSettings] MaxUnitCount` | `10` | unit-count range maximum | No | Yes |

No INI key was found that gates enabled/disabled state for these three standard offline trackbars.

Evidence: `ini/rulesmd.ini:3017..3026`, `ini/rules.ini:2497..2506`, `FUN_006AE6E0`, prior `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` reports. Active in YR: Yes.

## 5. Integration Points

| Function / point | Behavior relevant to disabled flow | Active in YR |
|---|---|---|
| `FUN_006AE2C0` | Opens/pumps offline Skirmish dialog; local result waits for `0x617` or `0x5C0` | Yes |
| `FUN_006AE3F0` | Dispatches init, command, paint, tooltip; no `WM_HSCROLL` disable branch | Yes |
| `FUN_00622B50` | Common shell dialog handling/subclass setup and tooltip routing; no scoped trackbar disable writer found | Yes |
| `FUN_006AE6E0` | Initializes trackbar ranges/positions; disables inactive row sibling controls only | Yes |
| `FUN_006ACEE0` | Handles commands, Choose Map side effects, Start validation, and final value reads; Start button is the only button disabled/re-enabled in this slice | Yes |
| `FUN_006ADC20` | AI row state update; disables/enables row sibling controls | Yes, but No for trackbars |
| `FUN_006ACD60` | Mode/team flag update; enables/disables start/team sibling controls | Yes, but No for trackbars |
| `OwnerDraw_Trackbar_0061D950` | Paint/input/value logic; disabled branch is style-read-only | Yes / Conditional |

## 6. Current Rust Implementation Status

Current Rust has the three trackbars as always-interactive standard setup controls:

- `src/ui/skirmish_shell/layout.rs` defines `SkirmishTrackbarId::{GameSpeed0x529, Credits0x511, UnitCount0x50c}`.
- `src/ui/skirmish_shell/state.rs` routes mouse down/move/up through `trackbar_ids()`, `handle_option_mouse_down`, `handle_option_mouse_move`, and `set_trackbar_visual_value`.
- `src/app_skirmish_shell_render.rs` renders all three trackbars in `push_trackbar_instances` and draws text in the right-side value rect.

There is no current disabled state flag for the three standard trackbars. Given this report, that is not a parity gap for normal standard offline Skirmish setup. A harness-only disabled visual branch may still be useful later for shared owner-draw completeness, but normal `0x102` gameplay should not disable these controls.

Evidence: source scan `rg` over `src/ui/skirmish_shell`, `src/app_skirmish_shell_render.rs`, `src/app.rs`; Codegraph context for `SkirmishTrackbarId` and `SkirmishShellState`. Active in YR: not applicable to Rust.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target question / notes | verified | Section 0 | none |
| Standard dialog creation/liveness | verified | `FUN_006AE2C0`, `FUN_006AE3F0`; prior AI-row report | none |
| `0x529/0x511/0x50C` init | verified | `FUN_006AE6E0`; assembly `0x006AECB3`, `0x006AECFF`, `0x006AED5B` | none |
| Trackbar apply/readback | verified | `FUN_006ACEE0`; assembly `0x006AD709..0x006AD77C` | none |
| Start button disable/re-enable contrast | verified | `FUN_006ACEE0`; assembly `0x006ACF92..0x006AD32A` | none |
| AI row sibling `EnableWindow` flow | verified | `FUN_006ADC20`, `FUN_006ACD60`, `FUN_006AE080`, `FUN_006ADF00` | none for trackbar scope |
| Choose Map selected-map side effects | verified | `FUN_006ACEE0`, `FUN_006ACD60` | no trackbar disable writer found |
| `WM_HSCROLL` parent branch | verified-negative | `FUN_006AE3F0`; `OwnerDraw_Trackbar_0061D950` sends `0x114` | none |
| Trackbar disabled paint branch | verified | `OwnerDraw_Trackbar_0061D950`; prior assembly `0x0061E0B0..0x0061E2B6` | runtime screenshot only if forced disabled state is tested |
| Binary-wide nonstandard dialogs using same numeric IDs | touched-not-exhausted | byte-pattern search hits in options/host/guest/help text | out-of-scope; they are not standard offline `0x102` trackbars |
| Current Rust standard trackbar state/render/input | verified | source scan + Codegraph context | none for normal enabled state |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which mode is this investigation? -> exhaustive-slice for standard offline Skirmish 0x102 disabled/enabled flow for 0x529/0x511/0x50C.` (evidence: user scope; Section 0)
- `[RESOLVED] OQ-02 - What creates the standard dialog? -> FUN_006AE2C0 opens and pumps the offline Skirmish dialog; FUN_006AE3F0 dispatches its messages.` (evidence: `0x006AE2C0`, `0x006AE3F0`)
- `[RESOLVED] OQ-03 - Are the three trackbars live in standard YR? -> Yes, FUN_006AE6E0 initializes all three and FUN_006ACEE0 reads all three on apply.` (evidence: `0x006AECB3..0x006AEDA0`, `0x006AD709..0x006AD77C`)
- `[RESOLVED] OQ-04 - Does init disable any of these three trackbars? -> No; init sends range/position/step messages only to these controls.` (evidence: `FUN_006AE6E0`)
- `[RESOLVED] OQ-05 - Does Start/Back disable or re-enable these three trackbars? -> No; Start validation disables/re-enables Start 0x617, then reads trackbar values only on successful packing.` (evidence: `FUN_006ACEE0`, `0x006ACF92..0x006AD32A`, `0x006AD709..0x006AD77C`)
- `[RESOLVED] OQ-06 - Does parent WM_HSCROLL trigger a disable branch? -> No standard 0x102 branch handles 0x114; common handler and dialog proc do not target trackbar disablement.` (evidence: `FUN_006AE3F0`, `FUN_00622B50`)
- `[RESOLVED] OQ-07 - Do AI row changes disable controls? -> Yes, but only row sibling country/color/start/team controls, not the scoped trackbars.` (evidence: `FUN_006ADC20`)
- `[RESOLVED] OQ-08 - Do selected mode/team flags disable controls? -> Yes, team/start sibling controls are disabled when the selected mode lacks team support or the row is inactive; no trackbar target.` (evidence: `FUN_006ACD60`, `FUN_006ADDF0`)
- `[RESOLVED] OQ-09 - Do selected map side effects disable trackbars? -> No; Choose Map success rebuilds map/mode state and calls row/mode helpers, with no trackbar EnableWindow/style write.` (evidence: `FUN_006ACEE0`, `FUN_006ACD60`)
- `[RESOLVED] OQ-10 - Does the owner-draw callback support disabled paint? -> Yes, it reads WS_DISABLED and changes thumb overlay, rail color, and value text color.` (evidence: `OwnerDraw_Trackbar_0061D950`, prior `0x0061E0B0..0x0061E2B6`)
- `[RESOLVED] OQ-11 - Is disabled paint reachable in normal standard setup? -> No writer found in standard 0x102 init/command/update paths; active only if some external/nonstandard code disables the HWND.` (evidence: `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_006ADC20`, `FUN_006ACD60`)
- `[RESOLVED] OQ-12 - Do INI defaults gate enabled state? -> No; MultiplayerDialogSettings keys provide values/ranges/step only.` (evidence: `ini/rulesmd.ini:3017..3026`, `FUN_006AE6E0`)
- `[RESOLVED] OQ-13 - Is this TS legacy? -> No for the enabled standard trackbar path; disabled paint is shared owner-draw infrastructure but conditionally unreachable in normal offline Skirmish.` (evidence: `FUN_006AE2C0`, `FUN_006AE6E0`, `OwnerDraw_Trackbar_0061D950`)
- `[RESOLVED] OQ-14 - What Rust surfaces would need disabled state if it were reachable? -> state/input/render surfaces around `SkirmishTrackbarId`, `handle_option_mouse_*`, and `push_trackbar_instances`; normal path needs no disabled flag.` (evidence: source scan)
- `[RESOLVED] OQ-15 - Edge cases: null HWND? -> Each init/apply block null-checks `GetDlgItem` before sending/reading; missing controls silently skip.` (evidence: `FUN_006AE6E0`, `FUN_006ACEE0`)
- `[RESOLVED] OQ-16 - Edge cases: zero/max values? -> Values are governed by already-verified range/value logic; no disable state depends on zero/max.` (evidence: prior trackbar reports; `OwnerDraw_Trackbar_0061D950`)
- `[RESOLVED] OQ-17 - Edge cases: first tick / first paint? -> First paint can render enabled state after init; disabled branch requires `WS_DISABLED`, which standard init does not set.` (evidence: `FUN_006AE3F0`, `FUN_006AE6E0`)
- `[RESOLVED] OQ-18 - Edge cases: paused/replay/save restore? -> Not applicable to setup dialog runtime; setup loop is shell UI before launch, and Start reads final values.` (evidence: `FUN_006AE2C0`, `FUN_006ACEE0`)
- `[DEFERRED] OQ-19 - Can a modded resource or external dialog-control mutation force these HWNDs disabled?` (category: `out-of-scope`; reason: standard retail `0x102` flow is exhausted, but arbitrary external Win32 mutation is not a stock path; next-step-if-pursued: runtime debugger force-disable screenshot harness)
- `[DEFERRED] OQ-20 - Do online host/guest slider IDs have their own disabled flow?` (category: `out-of-scope`; reason: user scope is standard offline `0x102`, and host/guest use different credit/unit IDs; next-step-if-pursued: separate WOL/host setup investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard offline `0x102` never disables `0x529`, `0x511`, or `0x50C`; they stay enabled from init through Start/Back readback | `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_006AE3F0` | none observed for normal setup | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Keep standard trackbars interactive/rendered enabled in normal offline setup | `skirmish_standard_trackbars_remain_enabled_after_ai_row_mode_and_map_changes`: change AI rows, choose map, start validation failure; trackbars still accept input | Do not add speculative disabling tied to map, mode, player count, or row state |
| Runtime disabled/enabled flows in this area are row-sibling or Start-button flows, not global option-trackbar flows | `FUN_006ADC20`, `FUN_006ACD60`, `FUN_006ACEE0 @ 0x006ACF92..0x006AD32A` | row sibling disabled state exists separately; trackbar delta none | row/combo/button surfaces, not trackbar state | Keep row sibling disabled logic separate from the three setup trackbars | `skirmish_ai_none_disables_row_siblings_not_global_trackbars`: selecting None disables row country/color/start/team only | Do not reuse opponent row `enabled` flags to gate Game Speed/Credits/Unit Count |
| Trackbar disabled paint is real but only conditional on `WS_DISABLED`; normal standard `0x102` does not reach it | `OwnerDraw_Trackbar_0061D950`, prior `0x0061E0B0..0x0061E2B6` | optional harness-only visual branch missing/unchecked | possible future shared owner-draw renderer state | No required normal-game implementation; only add disabled visual if a caller can prove a reachable disabled trackbar | `skirmish_forced_disabled_trackbar_uses_disabled_visual_colors` as a harness-only future test if a forced-disabled state is introduced | Do not treat disabled paint support as proof of a standard runtime disable flow |

## 10. Negative Facts / Do Not Do

- Do not disable Game Speed `0x529` when AI rows are set to `None`; `FUN_006ADC20` disables only row sibling controls. Active in YR: No for trackbar disablement.
- Do not disable Credits `0x511` or Unit Count `0x50C` after Choose Map; map selection refreshes map/mode state and row helpers but does not target those trackbars. Active in YR: No.
- Do not implement a parent `WM_HSCROLL` Skirmish branch that disables controls; the standard parent proc does not handle `0x114` for this purpose. Active in YR: No.
- Do not conflate trackbar custom sound-suppression message `0x4AE` / state `+0x108` with Win32 disabled state. Active in YR: No for disabled visual/input.
- Do not use host/guest or in-game-options uses of numeric ID `0x529` as evidence for offline `0x102` Game Speed disablement. Active in YR: No for this slice.

## 11. Stale Docs / Follow-up Docs

- Replace `SKIRMISH_TRACKBAR_CHANGED_VALUE_SOUND_GHIDRA_REPORT.md` coverage row wording "disabled-window input behavior: deferred ... runtime/user-flow that disables these trackbars, if any" with: "disabled-window behavior: standard offline `0x102` runtime flow does not disable `0x529`, `0x511`, or `0x50C`; disabled paint remains a conditional owner-draw branch only if an external/nonstandard caller sets `WS_DISABLED`."
- Replace broad trace wording that says "Disabled trackbar visual is UNCHECKED" for standard setup with: "Disabled trackbar visual is verified in the shared callback but not normally reachable for standard offline `0x102`; normal Skirmish should render these three trackbars enabled."

## Sources

- Ghidra decompiled/read-only: `FUN_006AE2C0 @ 0x006AE2C0`
- Ghidra decompiled/read-only: `FUN_006AE3F0 @ 0x006AE3F0`
- Ghidra decompiled/read-only: `FUN_006AE6E0 @ 0x006AE6E0`
- Ghidra decompiled/read-only: `FUN_006ACEE0 @ 0x006ACEE0`
- Ghidra decompiled/read-only: `FUN_006ACD60 @ 0x006ACD60`
- Ghidra decompiled/read-only: `FUN_006ADC20 @ 0x006ADC20`
- Ghidra decompiled/read-only: `FUN_006ADDF0 @ 0x006ADDF0`
- Ghidra decompiled/read-only: `FUN_006AE080 @ 0x006AE080`
- Ghidra decompiled/read-only: `FUN_006ADF00 @ 0x006ADF00`
- Ghidra decompiled/read-only: `FUN_00622B50 @ 0x00622B50`
- Ghidra decompiled/read-only: `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`
- Ghidra byte-pattern/read-only: scoped ID hits for `0x529`, `0x511`, `0x50C`; `EnableWindow` import-call hits checked for standard `0x102` functions
- Prior docs checked: `SKIRMISH_TRACKBAR_CHANGED_VALUE_SOUND_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`, `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
- Rust scanned: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`
