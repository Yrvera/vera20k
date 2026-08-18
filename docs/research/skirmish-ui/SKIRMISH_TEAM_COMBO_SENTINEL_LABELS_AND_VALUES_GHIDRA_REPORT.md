# Skirmish Team Combo Sentinel Labels And Values - Ghidra Research Report

**Address(es):** `0x004E5AC0`, `0x004E5B60`, `0x004E5D60`, `0x004E5ED0`, `0x005D5DC0..0x005D5E08`, `0x006ADC20`, `0x006AE6E0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard offline Skirmish dialog `0x102` Team combo rows for controls `0x76D..0x774`: visible labels, item data, sentinel handling, and selected-mode gating for `None` vs explicit teams.
**Non-Scope:** House `+0x1605C` alliance consumer, full MPModes roster construction, WOL/network Team combo semantics, start-position combo rows, and post-launch alliance map implementation beyond Rust handoff.
**Confidence:** High for offline row labels/item-data and selected-mode gates; Medium for vtable `+0x34` caller coverage because the callback bytes and vtable binding are verified but the full indirect caller inventory was not expanded beyond this slice.
**Active in YR:** Yes for standard offline Skirmish. Conditional details are called out per finding.

## Working Notes

- Target question: Verify offline Skirmish Team combo labels, item-data/sentinel values, and selected-mode gating for Team None/Auto/numbered teams.
- Non-goals: Do not re-investigate House+0x1605C consumer, MPModes roster construction, launch alliance consumer, or unrelated combo geometry unless needed to prove source values.
- Evidence needed to mark COMPLETE: decompile/caller evidence for Team combo population path, label/string source, signed sentinel values, gating conditions, and current Rust surface scan with handoff.
- Stop conditions: zero-open questions for this narrow slice, or mark PARTIAL if Ghidra lacks a read-only boundary or any sentinel/gating claim cannot be proved.

## 1. Overview

The offline Skirmish Team combo is letter-based, not numbered in the visible text: optional `None`, then `A`, `B`, `C`, `D`. The stored item data is signed and compact: `None = -2`, `A = 0`, `B = 1`, `C = 2`, `D = 3`.

There is no offline Team combo `Auto` row. `-1` appears in the adjacent AI row-state combo as inactive `GUI:None`, and the verified Team validator rejects `-1`; do not expose or pack `-1` as a standard offline team choice.

## 2. Class Layout / Key Offsets

| Field / address | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `MultiplayerGameMode +0x3C` | `AlliesAllowed` byte; controls inactive-row team default / ally-allowed helper | vtable `+0x30` bytes `0x005D5DD0..0x005D5DDD`; direct read in `0x006ADC20` and `0x006AE6E0`; constructor source covered by MPModes report | Yes, conditional by selected mode |
| `MultiplayerGameMode +0x3F` | `MustAlly` byte; controls whether `None` is inserted | vtable `+0x2C` bytes `0x005D5DC0..0x005D5DCD`; MPModes constructor report | Yes, conditional by selected mode |
| vtable `+0x2C` | Team `None` availability: returns `-2` if `MustAlly == 0`, else `0` | Battle vtable memory `0x007EE184 + 0x2C -> 0x005D5DC0`; assembly `0x005D5DC0..0x005D5DCD`; caller `0x004E5B60` | Yes |
| vtable `+0x30` | Ally/default helper: returns `3` if `AlliesAllowed != 0`, else `-2` | Battle vtable memory `0x007EE184 + 0x30 -> 0x005D5DD0`; assembly `0x005D5DD0..0x005D5DDD` | Yes/Conditional |
| vtable `+0x34` | Team value validator: rejects `-2` when `MustAlly != 0`; accepts `0..3`; rejects `<0` other than allowed `-2` and rejects `>3` | Battle vtable memory `0x007EE184 + 0x34 -> 0x005D5DE0`; assembly `0x005D5DE0..0x005D5E08` | Yes as a mode callback; caller inventory not broadened |
| `DAT_008B3FC0..0x008B3FE4` | runtime pointers for Team `A..D` labels | initialized by `0x004E5AC0`; consumed in `0x004E5B60` loop with stride `0xC` | Yes |

## 3. Core Logic

### 3.1 Team label table initialization

`FUN_004E5AC0` loads the Team letters from `LETTER_A`, `LETTER_B`, `LETTER_C`, and `LETTER_D` into a 4-row table at `DAT_008B3FC0`, `DAT_008B3FCC`, `DAT_008B3FD8`, and `DAT_008B3FE4`, then writes a trailing pointer at `DAT_008B3FF0`. The order is A, B, C, D, even though the strings live in memory near reverse-ordered labels. Active in YR: Yes; `0x006AE6E0` calls `0x004E5AC0` during offline Skirmish initialization before Team combo population.

Evidence: decompile `0x004E5AC0`; assembly context `0x004E5AC0..0x004E5B2C`; string memory `0x00822BEC = LETTER_A`, `0x00822BE0 = LETTER_B`, `0x00822BD4 = LETTER_C`, `0x00822BC8 = LETTER_D`.

### 3.2 Team combo row population

`FUN_004E5B60(hwnd, control_id)` rebuilds one Team combo. It hides the control, clears it with `CB_RESETCONTENT` (`0x14B`), sets custom shell combo messages `0x4DD` and `0x4DE`, then evaluates the selected mode:

1. In standard offline Skirmish (`g_GameMode` not `3` or `4`), it calls `FUN_005E2F80()` to get the selected MPModes object.
2. It dispatches vtable `+0x2C`.
3. If the return is negative, it inserts `GUI:NoneAsSymbols` from string address `0x00822BF8` / string id `0x45F` and assigns item data `-2`.
4. It always appends A, B, C, D from the letter table and assigns item data `0`, `1`, `2`, `3` in loop order.
5. It selects row index `0` after rebuilding. Therefore Battle selects `None` by default when `None` exists; Team Game selects Team A by default because `None` is absent.

Active in YR: Yes. Evidence: decompile `0x004E5B60`; `0x004E5D60` calls it for all eight team controls in standard offline Skirmish; `0x006AE6E0` calls `0x004E5D60` during dialog setup and after selected-mode normalization.

### 3.3 Selected-mode gating

The concrete common MPModes callbacks are short byte-level helpers:

| Gate | Bytes / branch result | Effect | Active in YR |
|---|---|---|---|
| `+0x2C`, `0x005D5DC0` | reads byte `+0x3F`, returns `-2` when `MustAlly == 0`, else `0` | negative return makes `0x004E5B60` insert Team `None` | Yes; Battle/FFA/etc. allow `None`, Team Game suppresses it |
| `+0x30`, `0x005D5DD0` | reads byte `+0x3C`, returns `3` when `AlliesAllowed != 0`, else `-2` | same policy as direct `+0x3C` reads used for inactive-row team default | Yes/Conditional by selected mode |
| `+0x34`, `0x005D5DE0` | if `MustAlly != 0` and proposed value is `-2`, returns false; then accepts only signed `0..3` | `-1` is rejected; `4+` is rejected; `None` is invalid in Team Game | Yes as vtable-bound callback |

The signedness is load-bearing: the validator uses signed `JL` after `TEST EAX,EAX`, so `-1` is not an accepted Team value. Active in YR: Yes for the callback binding; evidence: assembly `0x005D5DF5..0x005D5E08` and Battle vtable memory `0x007EE184 + 0x34`.

### 3.4 Inactive AI row interaction

When an AI row is inactive, `FUN_006ADC20` disables the sibling side/color/start/team controls and forces side/color/start to `-2`. For the team control it chooses `-2` if no selected mode exists or selected mode `AlliesAllowed` is false, otherwise it selects Team D (`3`). The final post-init loop in `0x006AE6E0` repeats this same visible team-default policy for AI rows `1..7`: `AlliesAllowed == 0 -> -2`, else `3`.

Active in YR: Yes. Evidence: decompile `0x006ADC20`; decompile `0x006AE6E0` final loop after `DAT_00A8B23C = local_4`.

This is a default/selection policy, not an extra visible row. It does not create an `Auto` item.

## 4. INI Keys

| Key / source | Verified default/effect | Evidence | Active in YR |
|---|---|---|---|
| `MPModesMD.ini` rows | stock selected modes include Battle id `1`, Team Game id `9`, FFA id `2`, Coop id `3`, etc.; no stock Siege row | existing MPModes report; local `ini/mpmodesmd.ini` | Yes |
| `[MultiplayerDialogSettings] MustAlly` | constructor default false; Team Game override true; true suppresses Team `None` | MPModes report constructor `0x005D5CF7..0x005D5D11`; callback `0x005D5DC0` | Yes/Conditional |
| `[MultiplayerDialogSettings] AlliesAllowed` | constructor default true before override; FFA/Coop false; false makes inactive AI team default `None` instead of Team D | MPModes report constructor `0x005D5CDF..0x005D5CEA`; direct reads in `0x006ADC20` / `0x006AE6E0` | Yes/Conditional |
| base `rulesmd.ini` `AlliesAllowed=no` | not the selected MPModes object value for the Team combo | MPModes object report shows per-mode override reader; this slice observed selected object reads, not base RulesClass reads | Conditional; do not use as combo gate |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Dialog init | `0x006AE6E0` initializes label tables, calls `0x004E5D60`, normalizes selected mode, calls `0x004E5D60` again, then applies inactive-row team defaults | decompile `0x006AE6E0` | Yes |
| Team refresh fanout | `0x004E5D60` iterates eight row controls and calls `0x004E5B60` for standard offline Skirmish | decompile `0x004E5D60`; xrefs from `0x006AE6E0` and `0x006ACEE0` | Yes |
| Team combo setter | `0x004E5ED0` selects the item whose stored data equals requested value and does not write table ownership for `-2` | decompile `0x004E5ED0` | Yes |
| Start packing | `0x006ACEE0` reads current Team combo item data through `0x004E6030` and writes it into `DAT_00A8B2FC[row]`, later consumed by house creation | decompile `0x006ACEE0`; prior House `+0x1605C` report | Yes |

## 6. Current Rust Implementation Status

Rust now has a stock MPModes model with `allies_allowed` and `must_ally` in `src/skirmish_modes.rs:21..30`, and stock tests prove Team Game has `must_ally` while FFA disables allies. However, the active shell Team combo builder does not receive selected mode data: `src/ui/skirmish_shell/state.rs:824..872` always returns `[-2, 0, 1, 2, 3]` for every Team combo.

Launch packing preserves explicit `0..3` team values and `-2` as `LaunchTeam::None` through `src/ui/skirmish_shell/state.rs:1281..1352`, and `src/app_skirmish.rs:324..349` creates alliances for matching explicit teams only. That post-launch behavior is consistent with the prior `House+0x1605C` reports, but the visible combo and selected-mode validation are still incomplete.

Primary Rust delta: Team Game should omit `None` and reject/repair `-2`; FFA/Coop should keep `None` available but inactive AI rows/defaults should not be forced to Team D when `AlliesAllowed` is false. There is also no valid offline `Auto` Team row to add.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes / scope gate | verified | this report, Working Notes | none |
| `0x004E5AC0` Team label table | verified | decompile and assembly `0x004E5AC0..0x004E5B2C` | none |
| `0x004E5B60` offline Team row population | verified | decompile `0x004E5B60`; string `0x00822BF8` | none |
| `0x004E5D60` refresh fanout | verified | decompile `0x004E5D60`; xrefs from `0x006AE6E0` | none |
| `0x004E5ED0` select-by-item-data helper | verified | decompile `0x004E5ED0` | none |
| vtable `+0x2C` / `+0x30` / `+0x34` bytes | verified | vtable memory `0x007EE184`; assembly `0x005D5DC0..0x005D5E08` | none for semantics |
| vtable `+0x34` indirect caller inventory | touched-not-exhausted | callback bytes and vtable binding verified | full indirect caller census outside this narrow visible-row slice |
| `0x006ADC20` inactive-row team default | verified | decompile `0x006ADC20` | none |
| `0x006AE6E0` init/post-mode refresh order | verified | decompile `0x006AE6E0` | none |
| House `+0x1605C` alliance consumer | deferred | prior report covers it | out-of-scope; no contradiction found |
| WOL/network Team combo branch | deferred | `0x004E5B60` online branch observed | out-of-scope |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - What exact Team combo rows are inserted offline? -> Optional None first, then A-D always.` (evidence: `0x004E5B60`)
- `[RESOLVED] OQ-02 - What are the item-data values? -> None `-2`; A-D `0..3` in order.` (evidence: `0x004E5B60`, `0x004E5ED0`)
- `[RESOLVED] OQ-03 - Are labels numbered or lettered? -> Lettered via `LETTER_A..LETTER_D`, not visible numbered teams.` (evidence: `0x004E5AC0`, string memory `0x00822BEC..0x00822BC8`)
- `[RESOLVED] OQ-04 - Is there a visible Team Auto row in standard offline Skirmish? -> No; no `-1` item is inserted by `0x004E5B60`, and validator rejects `-1`.` (evidence: `0x004E5B60`, `0x005D5DF5..0x005D5E08`)
- `[RESOLVED] OQ-05 - What controls None insertion? -> selected mode vtable `+0x2C`, backed by `MustAlly +0x3F`.` (evidence: `0x004E5B60`, `0x005D5DC0..0x005D5DCD`)
- `[RESOLVED] OQ-06 - What controls inactive AI row team defaults? -> selected mode `AlliesAllowed +0x3C`; false selects `-2`, true selects `3`.` (evidence: `0x006ADC20`, `0x006AE6E0`)
- `[RESOLVED] OQ-07 - What exact values does the mode validator accept? -> `0..3`, plus `-2` only when `MustAlly == 0`; rejects `-1` and `>3`.` (evidence: `0x005D5DE0..0x005D5E08`)
- `[RESOLVED] OQ-08 - Is this active in standard YR offline Skirmish? -> Yes; `0x006AE6E0` and `0x006ACEE0` reach the helpers in dialog `0x102`.` (evidence: decompile `0x006AE6E0`, `0x006ACEE0`)
- `[RESOLVED] OQ-09 - Does Rust already gate the visible Team combo by selected mode? -> No; `combo_items` always returns `[-2,0,1,2,3]`.` (evidence: `src/ui/skirmish_shell/state.rs:824..872`)
- `[RESOLVED] OQ-10 - Does Rust preserve explicit same-team alliances after launch? -> Yes in current launch alliance map, for explicit `LaunchTeam::Team` only.` (evidence: `src/app_skirmish.rs:324..349`)
- `[DEFERRED] OQ-11 - Full WOL/network Team combo labels and disabled combo behavior.` (category: out-of-scope; reason: target is offline Skirmish; next-step-if-pursued: investigate `g_GameMode == 3 || 4` branch and `0x004E5CB0`)
- `[DEFERRED] OQ-12 - Complete indirect caller census for vtable `+0x34`.` (category: bounded-cost-too-high; reason: callback semantics and vtable binding are enough for this UI handoff; next-step-if-pursued: scan all indirect calls on MPModes object vtables)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Team Game suppresses visible Team `None`; Battle permits it | `0x004E5B60`, `0x005D5DC0..0x005D5DCD`, Team Game `MustAlly` from MPModes report | missing: `combo_items` always includes `-2` | `src/ui/skirmish_shell/state.rs::combo_items`, selected-mode plumbing | Team combo items must depend on selected mode `must_ally` | Select Battle: Team combo rows are `None,A,B,C,D`; select Team Game: rows are `A,B,C,D` and current `-2` selections are repaired/rejected | Do not model `None` as a static Team row; proposed test `team_game_must_ally_omits_team_none_combo_item` |
| No standard offline Team `Auto` row exists; `-1` is rejected by the Team validator | `0x004E5B60`, `0x005D5DF5..0x005D5E08`, AI row-state report for `-1` | partial risk: Rust has no Auto Team row now, but future UI text could confuse AI `None` with Team Auto | `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs::LaunchTeam` | Keep `-1` out of offline Team choices and reject it if passed as shell team value | Attempting to apply/select `Team(-1)` in offline shell is not accepted as a real team and is not packed into a launch session | Do not reuse AI-row inactive sentinel `-1` as Team Auto; proposed test `offline_team_combo_rejects_minus_one_auto_sentinel` |
| Inactive AI team default is selected-mode dependent: `AlliesAllowed=false -> -2`, true -> Team D (`3`) | `0x006ADC20`, `0x006AE6E0`, `0x005D5DD0..0x005D5DDD` | missing/unchecked: current opponent defaults initialize `team: -2` and do not refresh by selected mode | `src/ui/skirmish_shell/state.rs` row activation/mode-change logic | When selected mode or AI row state changes, inactive rows should select the native default for the selected mode | FFA/Coop inactive AI rows keep Team `None`; Battle/Team Game inactive rows select Team D when the row is disabled after mode refresh | Do not conflate row disabled state with launch alliance grouping; proposed test `inactive_ai_team_default_follows_allies_allowed` |

### Negative Facts / Do Not Do

- Do not add a visible offline Team `Auto` row. Active in YR: Yes; evidence `0x004E5B60` inserts only optional `-2` and `0..3`.
- Do not accept `-1` as an offline Team value. Active in YR: Yes for validator semantics; evidence signed range check `0x005D5DF5..0x005D5E08`.
- Do not label the explicit Team rows as `1..4` unless a localization layer maps `LETTER_A..D` that way. Active in YR: Yes; evidence `0x004E5AC0` loads `LETTER_A..LETTER_D`.
- Do not use base `rulesmd.ini` `AlliesAllowed=no` to decide Team combo rows. Active in YR: Conditional by mode object; evidence MPModes constructor report plus selected-object reads in `0x006ADC20`.
- Do not treat numeric StringTable IDs as globally unique without their source-string anchor: Team letters use `GDlgSupp.cpp` IDs `0x437..0x43A`, while Start validation uses `Skirmish.cpp` IDs with different visible text. Active in YR: Yes; evidence `0x004E5AC0` vs `0x006AD073..0x006AD0C6`.

### Stale Docs / Follow-up Docs

None found in the scoped seed reports. Existing `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md` and `SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md` already state A-D / `-2` correctly. This report adds the explicit "no offline Auto team row / reject `-1`" wording for future handoffs.

## Sources

- Ghidra decompiled: `0x004E5AC0`, `0x004E5B60`, `0x004E5CB0`, `0x004E5D60`, `0x004E5ED0`, `0x004E5940`, `0x004E6030`, `0x005E2F80`, `0x006ADC20`, `0x006ACEE0`, `0x006AE6E0`.
- Ghidra assembly/read memory: `0x005D5DC0..0x005D5E08`; vtable memory at `0x007EE184`, `0x007EE424`, `0x007EE27C`; strings at `0x00822BEC`, `0x00822BE0`, `0x00822BD4`, `0x00822BC8`, `0x00822BF8`.
- Docs referenced: `SKIRMISH_HOUSE_0X1605C_TEAM_ADJUNCT_CONSUMER_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`, `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`, `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`, `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`, `SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/skirmish_modes.rs`, `src/skirmish_launch.rs`, `src/app_skirmish.rs`, `src/app.rs`.
- INI checked: `ini/mpmodesmd.ini`, `ini/rulesmd.ini`, `ini/rules.ini`.
