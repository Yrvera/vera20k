# Skirmish Player/AI Row Visibility And Enable Rules - Ghidra Research Report

**Address(es):** `0x006AE6E0` primary init, `0x006ADDF0` row show/hide adjuster, `0x006ADC20` player-type row enable adjuster, `0x006ACD60` team-control enable refresh, `0x006ACEE0` Start/Back apply validation  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard offline Skirmish dialog `0x102` local player row and seven AI/opponent row visibility/enabled rules. Covers controls shown/hidden/enabled/disabled from selected map start count, AI player row type, persisted/default slot values, and apply-time active-player count validation.  
**Non-Scope:** combo owner-draw rendering internals, dropdown visuals, map chooser internals beyond the selected-map count refresh already needed by row visibility, online/WOL/observer variants except where a helper branch is explicitly not standard offline Skirmish.  
**Confidence:** High for offline `0x102` control IDs, row show/hide, enabled/disabled paths, Start validation gates, and AI row labels after follow-up correction.  
**Active in YR:** Yes. Evidence: `FUN_006AE2C0` creates/pumps dialog `0x102`, `FUN_006AE3F0` routes custom init `0x497` to `FUN_006AE6E0` and `WM_COMMAND` to `FUN_006ACEE0`; no TS-only gate found on this offline Skirmish route.

## 1. Overview

Offline Skirmish has eight row slots for side/country, color, start, team, and flag controls: slot `0` is the local human player and slots `1..7` are AI/opponent rows. Only slots `1..7` have a player-type combo (`0x50B..0x51D`). This is why the local row is never closed by the player-type logic; the AI rows can be visible but disabled when their type is `GUI:None` (`item data -1`). Follow-up `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md` confirms that offline dialog `0x102` does not use `GUI:Closed`; that label belongs to online/WOL paths.

Selected map start count controls row visibility for AI rows only. If the selected map supports `N` players/starts, the local player row stays visible and AI rows after `N - 1` are hidden. Within a visible AI row, selecting a live AI difficulty (`0=Hard`, `1=Normal`, `2=Easy`) enables country/color/start/team controls; selecting `GUI:None` (`item data -1`) disables those sibling controls and programmatically selects sentinel values.

## 2. Control Layout / Key IDs

| Slot | Meaning | Type combo | Flag/static | Country/side | Color | Start | Team | Active in YR / evidence |
|---:|---|---:|---:|---:|---:|---:|---:|---|
| 0 | local player | none | `0x6DA` | `0x6A1` | `0x6A2` | `0x6A3` | `0x76D` | Yes; `FUN_004E3320`, `FUN_004E37D0`, `FUN_004E41D0`, `FUN_004E4E60`, `FUN_004E5940`; layout doc |
| 1 | AI/opponent 1 | `0x50B` | `0x6DB` | `0x510` | `0x522` | `0x6A4` | `0x76E` | Yes; same helpers plus `FUN_006AE6E0` type loop |
| 2 | AI/opponent 2 | `0x50E` | `0x6DC` | `0x513` | `0x523` | `0x6A5` | `0x76F` | Yes; same |
| 3 | AI/opponent 3 | `0x516` | `0x6DD` | `0x51E` | `0x524` | `0x6A6` | `0x770` | Yes; same |
| 4 | AI/opponent 4 | `0x51A` | `0x6DE` | `0x514` | `0x525` | `0x6A7` | `0x771` | Yes; same |
| 5 | AI/opponent 5 | `0x51B` | `0x6DF` | `0x51F` | `0x526` | `0x6A8` | `0x772` | Yes; same |
| 6 | AI/opponent 6 | `0x51C` | `0x6E0` | `0x520` | `0x527` | `0x6AA` | `0x773` | Yes; same |
| 7 | AI/opponent 7 | `0x51D` | `0x6E1` | `0x521` | `0x528` | `0x6AB` | `0x774` | Yes; same |

`FUN_006ADDF0`, `FUN_006ADF00`, and `FUN_006AE080` also include the flag/static control from `FUN_004E3320` when showing/hiding an AI row. The local player's edit/name control `0x6A0` is initialized separately in `FUN_006AE6E0`; it is not part of the AI show/hide loops. Active in YR: Yes; evidence `0x006AE6E0`, `FUN_004E3320`, `FUN_006ADDF0`, `FUN_006ADF00`, `FUN_006AE080`.

## 3. Core Logic

### Dialog entry and initialization

`FUN_006AE2C0` is the standard offline Skirmish launcher. It creates/pumps dialog `0x102` through proc `FUN_006AE3F0`. `FUN_006AE3F0` calls `FUN_006AE6E0` on custom message `0x497`.

Active in YR: Yes. Evidence: `FUN_006AE2C0`, `FUN_006AE3F0`.

### AI type combo population

`FUN_006AE6E0` loops exactly seven AI type controls: `0x50B`, `0x50E`, `0x516`, `0x51A`, `0x51B`, `0x51C`, `0x51D`. Each combo is reset (`0x14B`), receives four rows, and stores item data in this order: `-1`, `2`, `1`, `0`, corresponding to `GUI:None`, `GUI:AIEasy`, `GUI:AINormal`, and `GUI:AIHard`. The combo is initially selected to row `0`, which is the `-1` inactive sentinel, then persisted/default slot state may select one of the real difficulty item-data values.

Active in YR: Yes. Evidence: `FUN_006AE6E0` AI-type setup loop; `FUN_006AE3F0` tooltip branch later treats `-1`, `2`, `1`, and `0` as the four recognized type states.

### Persisted/default slot state

AI slot triples are persisted/read as type, country/side, and color values. `SessionClass__ReadSkirmishSettings @ 0x00697F10` reads `Slot%02d` strings through `FUN_00477440`, which parses up to three comma-separated integers. If slot entries are absent, the caller's defaults are used. The standard `[Skirmish]` read site in the session/options load path passes different defaults for Slot01 versus later slots, matching the rules comment that Skirmish always has at least one AI even though `rulesmd.ini [MultiplayerDialogSettings] AIPlayers=0`.

Active in YR: Yes. Evidence: `SessionClass__ReadSkirmishSettings @ 0x00697F10`, `FUN_00477440`, session/options load path decompile around `0x00698100`, `rulesmd.ini:3000..3028`.

### Init-time closed-row handling

During `FUN_006AE6E0`, each saved AI type code is mapped back to type-combo item data:

| Saved type code | Type combo item data | Row interpretation | Active in YR / evidence |
|---:|---:|---|---|
| `1` | `-1` | closed/inactive row | Yes; `FUN_006AE6E0` mapping before `local_14` selection |
| `4` | `0` | active AI difficulty/state | Yes; same |
| `5` | `1` | active AI difficulty/state | Yes; same |
| `6` | `2` | active AI difficulty/state | Yes; same |
| other | `-1` | closed fallback | Yes; same |

If the mapped type item data is `-1`, init programmatically selects sentinel `-2` for the row's country, color, start, and team-like controls, then calls `EnableWindow(..., 0)` on country, color, start, and team. The type combo itself remains enabled; the row may still be visible depending on selected map start count.

Active in YR: Yes. Evidence: `FUN_006AE6E0` branch after selecting type item data, calls to `FUN_004E3F70(-2)`, `FUN_004E49A0(-2)`, `FUN_004E5480(-2)`, `FUN_004E5ED0(-2)`, then `EnableWindow` false on the four sibling controls.

### Player-type change handling

`FUN_006ACEE0` routes type combo commands `0x50B`, `0x50E`, `0x516`, `0x51A`, `0x51B`, `0x51C`, `0x51D` to `FUN_006ADC20`, then refreshes dependent team enable state through `FUN_006ACD60`.

`FUN_006ADC20` reads the selected type combo item data. If it is `0`, `1`, or `2`, the row is active and the country/color/start/team controls are enabled. Otherwise the row is inactive: it forces country, color, and start controls to sentinel `-2`, and disables country/color/start/team. Before final enabling, the helper also refreshes the row's team selection to either `3` or `-2` depending on selected mode `AlliesAllowed` at `DAT_00A8B23C + 0x3C`.

Active in YR: Yes. Evidence: `FUN_006ACEE0` switch cases; `FUN_006ADC20`; `FUN_006ACD60`.

### Map start count and row visibility

The selected-map player/start count comes from `FUN_005E6520(index)`: it opens the selected map path, counts present `[Waypoints]` keys `0..7`, and if none exist falls back to `[RandomMap] NumPlayers`; if that fallback is zero it returns `8`. `FUN_006ADDF0` compares old and new selected-map counts and calls:

- `FUN_006ADF00` when the new count is larger, showing the newly valid AI rows.
- `FUN_006AE080` when the new count is smaller, closing and hiding rows beyond the new count.

`FUN_006AE080(count)` first selects closed (`-1`) in every AI type combo from row `count` through row `7`, calls `FUN_006ADC20` so the sibling controls become disabled/sentinel, then hides the row's type combo, flag/static, country, color, start, and team controls. It never hides the local slot `0`.

Active in YR: Yes. Evidence: `FUN_005E6520`, `FUN_006ADDF0`, `FUN_006ADF00`, `FUN_006AE080`; Choose Map branch in `FUN_006ACEE0` calls the selected-map rebuild and then `FUN_006ADDF0`.

### Team controls have an additional selected-mode enable gate

`FUN_006ACD60` enables team control `0x76D` for the local row only when `DAT_00A8B23C` is non-null and byte `+0x3C` is nonzero. Follow-up `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md` resolves this byte as selected mode `AlliesAllowed`. For AI rows, the same team flag must be true and the row type must not be inactive (`type item data != -1`); otherwise the team control is disabled.

Active in YR: Conditional. The helper runs in standard YR Skirmish, but team controls are enabled only if the selected mode allows allies. Evidence: `FUN_006ACD60`; `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md`.

### Apply-time validation

On Start Game (`0x617`) or Back (`0x5C0`), `FUN_006ACEE0` scans the seven AI type combos and counts only rows whose type item data is `0`, `1`, or `2`. It stores that count in `DAT_00A8B274`. On Start Game, it validates:

- selected map start count must be at least `active_ai_count + 1`;
- `active_ai_count + 1` must be at least `2`;
- if the local player selected an explicit team, active AI rows cannot all have that same team.

The Start button is disabled while the Start path validates and re-enabled on validation failure. This is apply-time validation; the row controls can be visible/disabled before Start based on the rules above.

Active in YR: Yes. Evidence: `FUN_006ACEE0` Start/Back branch, type combo scan, `DAT_00A8B274` write, `FUN_005E6520` count compare, popup branches, and `EnableWindow(0x617, 0/1)`.

## 4. INI Keys

| INI key | YR value | Effect in this slice | Active in YR / evidence |
|---|---:|---|---|
| `rulesmd.ini [MultiplayerDialogSettings] AIDifficulty` | `0` | Parsed into `RulesClass+0x14A4`; not directly enough to decide row visibility. Slot type defaults/persisted `Slot%02d` state feed the row type combos. | Yes; `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`, `rulesmd.ini:3027` |
| `rulesmd.ini [MultiplayerDialogSettings] AIPlayers` | `0` | Parsed into `RulesClass+0x14A8`; the standard Skirmish session defaults still enforce at least one AI row when no slot override exists. | Yes; `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`, `rulesmd.ini:3028`, `SessionClass__ReadSkirmishSettings @ 0x00697F10` |
| `[Skirmish] Slot%02d` in user/session INI | absent or `type,side,color` | Persists row type/side/color triples; absent values fall back to caller defaults. | Yes; `SessionClass__ReadSkirmishSettings @ 0x00697F10`, `FUN_00477440`; written by `SessionClass__WriteSkirmishSettings @ 0x00698F90` |
| selected map `[Waypoints] 0..7` | map-specific | Count of present waypoint starts becomes selected-map player/start capacity for row visibility and Start validation. | Yes; `FUN_005E6520` |
| selected map `[RandomMap] NumPlayers` | map-specific fallback | Used only if no start waypoints are found; zero falls back to `8`. | Conditional; `FUN_005E6520` |

## 5. Integration Points

| Integration | Behavior | Active in YR / evidence |
|---|---|---|
| `FUN_006AE2C0` | launches/pumps offline Skirmish dialog `0x102` until Start `0x617` or Back `0x5C0` | Yes |
| `FUN_006AE3F0` | dispatches init `0x497`, paint, `WM_COMMAND`, and tooltips | Yes |
| `FUN_006AE6E0` | initializes type/country/color/start/team controls, selected map, default state, and dependent enabled state | Yes |
| `FUN_006ACEE0` type branch | user changes AI row type; calls `FUN_006ADC20` and `FUN_006ACD60` | Yes |
| `FUN_006ACEE0` Choose Map branch | after accepted map selection, recomputes start count and calls `FUN_006ADDF0` to hide/show rows | Yes |
| `FUN_006ACEE0` Start branch | counts active AI rows and validates against selected map start count | Yes |

## 6. Current Rust Implementation Status

Not investigated in this slot beyond existing trace context. The current request prohibited modifying Rust files and asked only for binary row visibility/enable rules. Existing trace `traces/SKIRMISH_PLAYER_AI_COMBOS_FLAGS_TRACE.md` already records that the shell lacks full player/AI combo interaction; this report should be used as the binary behavior spec for implementing those row controls.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline dialog entry `0x102` | verified | `FUN_006AE2C0`, `FUN_006AE3F0` | none |
| AI type combo IDs and item data | verified | `FUN_006AE6E0`, `FUN_006AE3F0` tooltip branch; labels resolved by `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md` | none for offline row labels |
| Row slot control ID mapping | verified | `FUN_004E3320`, `FUN_004E37D0`, `FUN_004E41D0`, `FUN_004E4E60`, `FUN_004E5940`; layout doc | none |
| Init-time closed-row disable | verified | `FUN_006AE6E0` | none |
| Runtime type-change enable/disable | verified | `FUN_006ACEE0`, `FUN_006ADC20`, `FUN_006ACD60` | none |
| Map count source | verified | `FUN_005E6520` | exact map parser internals beyond waypoint/NumPlayers count out of scope |
| Map count row show/hide | verified | `FUN_006ADDF0`, `FUN_006ADF00`, `FUN_006AE080` | none |
| Team flag extra enable gate | verified | `FUN_006ACD60`, `DAT_00A8B23C+0x3C`; semantic name resolved by `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md` | none for offline team enable gate |
| Default/persisted slot read | verified | `SessionClass__ReadSkirmishSettings @ 0x00697F10`, `FUN_00477440`, session/options load path around `0x00698100` | exact precedence of all RA2MD.INI profile sections outside standard Skirmish deferred |
| Start apply validation | verified | `FUN_006ACEE0`, `FUN_005E6520`, `DAT_00A8B274` | exact popup localized strings out of scope |
| Combo owner-draw visuals | deferred | user non-scope | separate owner-draw reports cover rendering |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does the local player row have a type combo? No. Slot `0` has country/color/start/team/flag controls but no `0x50B..0x51D` equivalent; only seven AI rows have type combos. Evidence: `FUN_006AE6E0`, helper control maps.

[RESOLVED] OQ-2 - What hides AI rows? Selected-map player/start count changes. `FUN_006ADDF0` uses `FUN_006ADF00` to show newly valid rows and `FUN_006AE080` to close and hide rows beyond the count. Evidence: `FUN_005E6520`, `FUN_006ADDF0`, `FUN_006ADF00`, `FUN_006AE080`.

[RESOLVED] OQ-3 - What disables a visible AI row's sibling controls? Type item data not in `{0,1,2}`. Offline `GUI:None` item data `-1` forces sentinel selections and disables country/color/start/team. Evidence: `FUN_006AE6E0`, `FUN_006ADC20`, `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`.

[RESOLVED] OQ-4 - Are hidden rows also closed first? Yes. `FUN_006AE080` selects type item data `-1`, calls `FUN_006ADC20`, then hides the controls. Evidence: `FUN_006AE080`.

[RESOLVED] OQ-5 - Does Start Game trust visible rows alone? No. It rescans type combo item data and counts only active AI item data `{0,1,2}` into `DAT_00A8B274`, then validates against selected-map start count. Evidence: `FUN_006ACEE0`.

[RESOLVED] OQ-6 - Exact localized/string IDs for the four AI type combo rows. Offline Skirmish uses `-1 -> GUI:None / STT:PlayerNone`, `2 -> GUI:AIEasy / STT:PlayerDumbAI`, `1 -> GUI:AINormal / STT:PlayerSmartAI`, and `0 -> GUI:AIHard / STT:PlayerGeniusAI`. Evidence: `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`.

[RESOLVED] OQ-7 - Exact semantic name/source of selected mode byte `DAT_00A8B23C+0x3C`. It is the selected `MPModes` mode object's `AlliesAllowed` flag. It gates team-control enablement, and if false it also clears `MustAlly` during mode construction. Evidence: `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md`.

## Sources

- Ghidra decompiled/read-only: `FUN_006AE2C0`, `FUN_006AE3F0`, `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_006ACD60`, `FUN_006ADC20`, `FUN_006ADDF0`, `FUN_006ADF00`, `FUN_006AE080`, `FUN_005E6520`, `FUN_004E3320`, `FUN_004E37D0`, `FUN_004E41D0`, `FUN_004E4E60`, `FUN_004E5940`, `FUN_004E3F70`, `FUN_004E49A0`, `FUN_004E5480`, `FUN_004E5ED0`, `SessionClass__ReadSkirmishSettings @ 0x00697F10`, `SessionClass__WriteSkirmishSettings @ 0x00698F90`, `FUN_00477440`, `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`.
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` `[MultiplayerDialogSettings] AIDifficulty=0`, `AIPlayers=0`; selected maps' `[Waypoints]` and `[RandomMap] NumPlayers` are read by `FUN_005E6520`.
- Prior reports cross-checked: `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`, `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_FLAG_STATICS_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`, `traces/SKIRMISH_PLAYER_AI_COMBOS_FLAGS_TRACE.md`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`.
