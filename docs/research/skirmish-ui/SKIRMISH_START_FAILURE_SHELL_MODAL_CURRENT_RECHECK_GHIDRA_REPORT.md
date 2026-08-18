# Skirmish Start Failure Shell Modal Current Recheck - Ghidra Research Report

**Address(es):** `FUN_006ACEE0 @ 0x006ACEE0`, `FUN_005D3490 @ 0x005D3490`, `FUN_006AE2C0 @ 0x006AE2C0`, `FUN_006AE3F0 @ 0x006AE3F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** current Rust handling of offline Skirmish `Start Game` command `0x617` validation failures versus native shell modal/re-enable behavior for map capacity, fewer than two players, same explicit team, and selected-mode false output only when the output dword equals `0x617`.  
**Non-Scope:** successful launch packing, post-map spawn generation, Choose Map behavior, random-map generation, and full selected-mode object construction.  
**Confidence:** High for native failure branch behavior and current Rust source status; Medium for exact modal visual parity because this recheck did not render a screenshot.  
**Active in YR:** Yes for ordinary offline Skirmish failures; Conditional for selected-mode rejection by selected mode method and output dword.

## 0. Working Notes

- Target question: Does current Rust reproduce native Start `0x617` validation failure behavior: shell modal, no launch packing, Start re-enabled, and selected-mode rejection only when output dword equals `0x617`?
- Non-goals: Do not investigate successful launch packing, post-map spawn generation, map preview, Choose Map modal, or final player/session serialization.
- Evidence needed to mark COMPLETE: current Rust source scan for validation/result/modal paths; decompile plus assembly address ranges for each native failure branch; modal helper evidence; selected-mode `0x617` output gate evidence; stale-doc wording for current Rust status.
- Stop conditions: stop after each scoped native branch and each corresponding current Rust surface is classified; do not chase gameplay startup or non-stock/custom MPModes beyond the `0x617` output condition.

## 1. Overview

Native `gamemd.exe` handles ordinary Start failures inside the active offline Skirmish dialog. It disables Start, validates the selected map/player/team state, shows `FUN_005D3490` as a shell modal for blocking failures, re-enables Start, and returns before packing the launch session/result code.

Current Rust now has data-level validation for the three ordinary failure categories and sets a `validation_modal` state on failure. However, in the current source snapshot the render path only builds/draws the Choose Map modal; no validation modal sprite or text draw is wired into `src/app_skirmish_shell_render.rs`, so the visible player-facing native modal is still incomplete.

## 2. Key Offsets And Controls

| Item | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| dialog `0x102` | offline Skirmish setup shell | `0x006AE2C0` creates/pumps until `0x617` or `0x5C0` | Yes |
| command `0x617` | Start Game button | `0x006ACF8C..0x006ACF9E`, `0x006AE2C0` loop result | Yes |
| command `0x5C0` | Back button | `0x006AE2C0` loop result | Yes |
| child `0x5B0` | first message text control in `FUN_005D3490` modal | decompile `0x005D3490` | Yes |
| child `0x5AE` | second message/control text in `FUN_005D3490` modal | decompile `0x005D3490` | Yes |
| child `2`, `0x5AF` | optional button-label controls used only when non-empty params are passed | decompile `0x005D3490` | Conditional; not used by scoped Start failures |
| `DAT_00A8B274` | active AI row count written before validation | decompile `0x006ACEE0`; assembly `0x006AD052` | Yes |
| `DAT_00A8B23C` | selected MPModes object used for vtable `+0x14` acceptance | decompile `0x006ACEE0`; assembly `0x006AD2BA..0x006AD2D2` | Yes |

## 3. Native Core Logic

For `WM_COMMAND`, the parent dialog proc `0x006AE3F0` routes to `0x006ACEE0`. The Start/Back branch ignores nonzero notifications; Start with notification `0` first calls `EnableWindow(GetDlgItem(hwnd, 0x617), 0)`. Active in YR: Yes. Evidence: decompile `0x006AE3F0`; decompile `0x006ACEE0`; assembly `0x006ACF7B..0x006ACF9E`.

Active AI rows are counted by reading seven row type combos and counting item data `0`, `1`, or `2`. That count is stored in `DAT_00A8B274`, and total players are `active_ai_rows + 1`. Active in YR: Yes. Evidence: decompile `0x006ACEE0`; assembly `0x006AD040..0x006AD052`.

### Map Capacity Failure

If selected-map capacity is less than `active_ai_rows + 1`, native loads/formats `TXT_SCENARIO_TOO_SMALL` (`0x437`) with the capacity, loads `TXT_OK` (`0x438`), calls `FUN_005D3490(message, ok, 0, 0)`, re-enables Start, and returns before launch packing/result write. Active in YR: Yes. Evidence: decompile `0x006ACEE0`; assembly `0x006AD05B..0x006AD0DA`; text mapping from `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`.

### Fewer Than Two Players

If `active_ai_rows + 1 < 2`, native loads `TXT_NEED_AT_LEAST_TWO_PLAYERS` (`0x43F`), loads `TXT_OK` (`0x440`), calls `FUN_005D3490(message, ok, 0, 0)`, re-enables Start, and returns before packing/result write. Active in YR: Yes. Evidence: decompile `0x006ACEE0`; assembly `0x006AD0ED..0x006AD159`; text mapping from the modal text report.

### Same Explicit Team

The same-team branch runs only if the local team control returns a nonnegative explicit team. If every active AI team equals the local explicit team, native loads `TXT_CANNOT_ALLY` (`0x457`), loads `TXT_OK` (`0x458`), calls `FUN_005D3490(message, ok, 0, 0)`, re-enables Start, and returns before packing/result write. Local Team None/negative skips this branch. Active in YR: Yes. Evidence: decompile `0x006ACEE0`; assembly `0x006AD16C..0x006AD2A7`; helpers `0x004E5940` and `0x004E6030` verified by prior Start failure report.

### Selected-Mode False Output Gate

After ordinary validations, native initializes a local output object, dispatches selected mode vtable `+0x14`, and only shows the generic blocking modal when the method returns false and the output dword equals `0x617`. That modal uses mode output text as the body and `TXT_OK` (`0x469`) as the OK/control text, re-enables Start, frees the output object, and returns. If false returns with any other output dword, native calls `0x005D5E10` and falls through to packing. Active in YR: Conditional. Evidence: decompile `0x006ACEE0`; assembly `0x006AD2BA..0x006AD34B`; decompile `0x007B7100`; `0x005D6310` returns true for stock Battle/ManBattle; `0x005CB400` false path leaves output zero; `0x005CA720..0x005CA7B6` plus `0x007B6880` show Siege-style false paths write allocated string pointers, not literal `0x617`.

## 4. Modal Helper

`FUN_005D3490` is the native shell modal helper for this slice. It creates a modal shell dialog, stores a local result pointer at window long offset `8`, writes non-empty `param_1` to child `0x5B0`, writes non-empty `param_2` to child `0x5AE`, optionally writes `param_3` to child `2` and `param_4` to child `0x5AF`, pumps until a modal result changes, then tears the modal down and restores display-chain state. Active in YR: Yes. Evidence: decompile `0x005D3490`; disassembly `0x005D3490..0x005D3539`.

The scoped Start failures pass zeros for the optional button-label params; their visible button/control text comes from the second argument (`TXT_OK`) and modal template behavior, not from custom `param_3/4`. Active in YR: Yes. Evidence: failure branch assembly ranges `0x006AD0A7..0x006AD0C6`, `0x006AD126..0x006AD145`, `0x006AD274..0x006AD293`, `0x006AD2E3..0x006AD316`.

## 5. INI Keys

No INI key is directly read by the scoped Start failure modal branches. Selected map capacity comes from the selected scenario/map record path, and player/team state comes from dialog controls. Active in YR: Yes for the absence in this slice. Evidence: decompile `0x006ACEE0` failure branches and prior Choose Map/current map-record reports for selected-map capacity source.

Relevant data source outside this slice: `ini/mpmodesmd.ini` determines which stock/local MPModes rows are selectable. Prior modal text/mode report verified no stock/local selectable mode writes output dword `0x617`. Active in YR: Yes for local retail data. Evidence: `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`.

## 6. Current Rust Implementation Status

Current Rust data validation is partially caught up:

| Rust surface | Current status | Delta vs native |
|---|---|---|
| `src/ui/skirmish_shell/state.rs:1375` `launch_session` | validates selected map, counts active opponents, checks map capacity, no enabled opponent, same explicit team, color, and start position | ordinary failure categories now exist; mode `+0x14` output contract is still not modeled |
| `src/skirmish_launch.rs:209` `LaunchValidationError` | has `NoEnabledOpponent`, `MapCapacityExceeded`, `SameExplicitTeam`, plus unrelated selected-map/color/start errors | enough enum shape for ordinary native categories; lacks explicit native text/category metadata and mode-rejection output code |
| `src/app.rs:613` `skirmish_validation_modal` | maps ordinary validation errors to CSF keys and `TXT_OK`; capacity currently formats fallback as `"{base} ({requested_players}/{capacity})"` | capacity formatting does not match native `%d` substitution exactly; fallback English strings differ from local retail text |
| `src/app.rs:671` Start failure branch | logs warning and sets `state.skirmish_shell_state.validation_modal` | better than log-only, but still logs; native visibly blocks without a warning-log concept |
| `src/app.rs:834` modal click handler | consumes clicks while modal is present and clears it on OK rect | input side is partially wired |
| `src/app_skirmish_shell_render.rs:1271` and `:2130..2144` | builds shell and Choose Map modal instances/text only | validation modal is not drawn in the current render path; player-visible modal remains missing |
| `src/ui/skirmish_shell/layout.rs:560` | computes a validation modal layout | layout exists but no renderer uses it yet |
| `src/ui/skirmish_shell/state.rs:2117..2175` tests | tests data-level missing map/bad color/capacity/no-opponent/same-team | no acceptance test currently proves visible shell modal text, render, OK dismissal, or no loading transition |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes and scope gate | verified | section 0 | none |
| `0x006AE2C0` shell loop | verified | decompile `0x006AE2C0` | none |
| `0x006AE3F0` command dispatch | verified | decompile `0x006AE3F0` | none |
| Start notification gate and disable | verified | decompile `0x006ACEE0`; assembly `0x006ACF7B..0x006ACF9E` | none |
| active AI count | verified | decompile `0x006ACEE0`; assembly `0x006AD040..0x006AD052` | none |
| capacity failure modal/re-enable/return | verified | decompile `0x006ACEE0`; assembly `0x006AD05B..0x006AD0DA` | none |
| fewer-than-two modal/re-enable/return | verified | decompile `0x006ACEE0`; assembly `0x006AD0ED..0x006AD159` | none |
| same explicit team modal/re-enable/return | verified | decompile `0x006ACEE0`; assembly `0x006AD16C..0x006AD2A7` | none |
| selected-mode false output `0x617` gate | verified | decompile `0x006ACEE0`; assembly `0x006AD2BA..0x006AD34B`; mode helper evidence | custom/modded mode objects remain outside scope |
| current Rust validation enum/data checks | verified | `src/ui/skirmish_shell/state.rs:1375`, `src/skirmish_launch.rs:209`, tests `state.rs:2117..2175` | none for ordinary data categories |
| current Rust visible validation modal render | verified missing | `rg validation_modal`; no render use in `src/app_skirmish_shell_render.rs`; render only calls Choose Map modal draw at `:1388` and `:2144` | implement/render/verify modal |
| current Rust selected-mode false output modeling | verified missing | `SkirmishLaunchMode` only `Battle` at `src/skirmish_launch.rs:14`; no mode acceptance result type | future MPModes acceptance model |
| successful packing and spawn | not-touched | out of scope | use existing packing/spawn reports |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the scoped Start failure path live in standard YR? -> Yes; `0x006AE2C0` pumps offline Skirmish dialog `0x102`, and `0x006AE3F0` routes `WM_COMMAND` to `0x006ACEE0`.` (evidence: `0x006AE2C0`, `0x006AE3F0`)
- `[RESOLVED] OQ-02 - Does Start disable before validation? -> Yes, Start `0x617` notification zero calls `EnableWindow(...,0)` before count/validation.` (evidence: `0x006ACF7B..0x006ACF9E`)
- `[RESOLVED] OQ-03 - Which ordinary failures show modals? -> map capacity, total players < 2, and all active players on same explicit team.` (evidence: `0x006AD05B..0x006AD2A7`)
- `[RESOLVED] OQ-04 - Does native re-enable Start on ordinary failure? -> Yes after each modal and before return.` (evidence: `0x006AD0CB..0x006AD0DA`, `0x006AD14A..0x006AD159`, `0x006AD298..0x006AD2A7`)
- `[RESOLVED] OQ-05 - Does native pack launch data after ordinary failure? -> No; each ordinary failure returns before the packing block at `0x006AD34B`.` (evidence: decompile `0x006ACEE0`; assembly failure returns before `0x006AD34B`)
- `[RESOLVED] OQ-06 - Is same-team validation start-position collision? -> No; it reads team controls and local explicit team, and local Team None skips the branch.` (evidence: `0x006AD16C..0x006AD2A7`)
- `[RESOLVED] OQ-07 - Does selected-mode false always block? -> No; it blocks only when local output dword equals `0x617`.` (evidence: `0x006AD2D5..0x006AD346`)
- `[RESOLVED] OQ-08 - Do stock/local modes reach the generic `0x617` output modal? -> No evidence for stock/local selectable modes; Battle/ManBattle accept, Unholy false leaves output zero, Siege is not stock-local and writes string pointers.` (evidence: `0x005D6310`, `0x005CB400..0x005CB421`, `0x005CA720..0x005CA7B6`, `0x007B6880`; prior modal text report)
- `[RESOLVED] OQ-09 - Does current Rust validate ordinary categories? -> Yes for data-level `MapCapacityExceeded`, `NoEnabledOpponent`, and `SameExplicitTeam`.` (evidence: `src/ui/skirmish_shell/state.rs:1375..1412`; tests `state.rs:2137..2175`)
- `[RESOLVED] OQ-10 - Does current Rust show a visible native-style modal? -> Partial only: state/input exist, but render path does not draw validation modal instances or text.` (evidence: `src/app.rs:613..680`, `src/app.rs:834..851`, `src/app_skirmish_shell_render.rs:1271..1391`, `src/app_skirmish_shell_render.rs:2130..2144`)
- `[RESOLVED] OQ-11 - What happens to Start pressed/disabled state in current Rust failure? -> Current Rust clears pressed button on mouse-up before action; it does not model native transient `EnableWindow(0/1)` but the button remains usable after modal dismissal.` (evidence: `src/app.rs:904..923`; native `0x006ACF92..0x006AD32A`)
- `[DEFERRED] OQ-12 - Exact pixel/modal-template parity for validation modal render.` (category: out-of-scope; reason: current renderer lacks the validation modal, and this slot did not implement or screenshot it; next-step-if-pursued: implement render then run 800x600 visual check)
- `[DEFERRED] OQ-13 - Modded/custom mode object that intentionally writes output dword `0x617`.` (category: out-of-scope; reason: stock/local retail modes were already checked; next-step-if-pursued: investigate custom MPModes extension path separately)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Capacity failure shows `TXT_SCENARIO_TOO_SMALL` formatted with only map capacity, `TXT_OK`, re-enables Start, and stays in shell | `0x006AD05B..0x006AD0DA`; modal text report | partial: validation and modal state exist, but format/render incomplete | `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs` | render a blocking validation shell modal with native text and OK dismissal; no loading transition | selected capacity `2`, local+2 AIs: visible modal body says map has `2 player max`, OK closes it, Start can be clicked again | Do not append non-native `(requested/capacity)` text; proposed test `skirmish_start_capacity_failure_renders_native_modal_and_stays_in_shell` |
| No active AI rows show `TXT_NEED_AT_LEAST_TWO_PLAYERS` + `TXT_OK`, re-enable Start, return before packing | `0x006AD0ED..0x006AD159`; modal text report | partial: `NoEnabledOpponent` maps to modal state but render missing | same | make the player see the min-player shell modal and preserve shell state | all AI rows None: no launch session, visible min-player text, OK dismisses, button usable | Do not leave this as log-only or route to selected-map error; proposed test `skirmish_start_no_opponent_failure_renders_native_modal` |
| Same explicit team failure blocks only when local explicit team and all active AI teams match; Team None skips | `0x006AD16C..0x006AD2A7`; `0x004E5940/0x004E6030` prior helper evidence | partial: data validation exists; visible render missing | `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs` | preserve data check and render `TXT_CANNOT_ALLY` modal | local team A, active AIs team A: blocked; local Team None with AIs team A: not blocked by this check | Do not treat this as start-position collision; proposed test `skirmish_start_same_explicit_team_failure_renders_native_modal` |
| Selected-mode false blocks only if output dword is exactly `0x617`; stock/local selectable modes do not hit it | `0x006AD2D5..0x006AD346`; `0x005D6310`; `0x005CB400`; `0x007B6880` | missing mode-acceptance model; no immediate stock Battle delta | future MPModes model and `launch_session` validation | when mode acceptance is added, distinguish false+`0x617` modal from other false returns | synthetic mode acceptance false with output `0x617` blocks with output body + OK; false with output zero does not use generic blocking modal | Do not make every mode false return show the generic `TXT_OK` modal; proposed test `skirmish_mode_rejection_blocks_only_when_output_is_start_code` |

## 10. Negative Facts / Do Not Do

- Do not say Start validation is absent in current Rust. Active in YR comparison: Yes; current Rust now has data-level capacity/no-opponent/same-team validation. Evidence: `src/ui/skirmish_shell/state.rs:1375..1412`, tests `state.rs:2137..2175`.
- Do not say current Rust fully matches the native shell modal. Active in YR comparison: Yes; render path currently omits validation modal drawing. Evidence: `src/app_skirmish_shell_render.rs:1271..1391`, `:2130..2144`; no `validation_modal` render references.
- Do not launch or pack after ordinary validation failures. Active in YR: Yes. Evidence: failure branches return before `0x006AD34B`.
- Do not leave Start unusable after a blocking failure. Active in YR: Yes. Evidence: re-enable calls at `0x006AD0CB..0x006AD0DA`, `0x006AD14A..0x006AD159`, `0x006AD298..0x006AD2A7`, `0x006AD31B..0x006AD32A`.
- Do not implement same-team validation as start-position collision. Active in YR: Yes. Evidence: `0x006AD16C..0x006AD2A7` reads team controls.
- Do not make every selected-mode false return a blocking modal. Active in YR: Conditional. Evidence: `0x006AD2D9 CMP [ESP+0x1C],0x617` followed by fallthrough call `0x005D5E10` when not equal.
- Do not expose or rely on Siege as a stock local Skirmish path just to exercise this modal. Active in YR: No for stock/local roster; evidence inherited from `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`.

## 11. Remaining Uncertainty

- Exact validation-modal pixel/art parity remains unverified because current Rust does not render the validation modal yet.
- Custom/modded MPModes objects that deliberately write output dword `0x617` remain outside this stock/local recheck.
- Current Rust changed during this swarm window; this report reflects the source snapshot that contains `src/app.rs:613..680` modal-state mapping and no `validation_modal` rendering in `src/app_skirmish_shell_render.rs`.

## Stale Docs / Replacement Wording

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`: replace the Current Rust table wording "logs `LaunchValidationError` and leaves the shell visible; missing native shell modal; no structured visible error state" with "Current Rust now has `LaunchValidationError::MapCapacityExceeded`, `NoEnabledOpponent`, and `SameExplicitTeam`, maps failures to `SkirmishValidationModalState` in `src/app.rs`, and consumes OK clicks, but the current render path does not draw validation modal instances/text; capacity text formatting also does not exactly match native `%d` substitution."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`: replace "validation exists, modal text surface missing" with "validation exists and app-level modal state is partially wired, but validation modal rendering is still missing from `src/app_skirmish_shell_render.rs`; keep this as a visible-modal/render parity gap, not a data-validation gap."
- `C:/Users/enok/Documents/ra2-rust-game-docs/.swarm-claims.md`: append this report's claim line that current Rust is partial: data checks and modal state exist, visible validation modal render is still missing.

## Sources

- Ghidra read-only decompile: `0x006ACEE0`, `0x005D3490`, `0x006AE2C0`, `0x006AE3F0`, `0x005D6310`, `0x007B6880`, `0x007B7100`.
- Ghidra read-only assembly/disassembly: `0x006ACF7B..0x006AD34B`, `0x005D3490..0x005D3539`, `0x005CB400..0x005CB421`, `0x005CA720..0x005CA7B6`, `0x006AD2D5..0x006AD346`.
- Prior reports: `SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`, `SKIRMISH_PLAYER_AI_ROW_VISIBILITY_ENABLE_RULES_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`.
