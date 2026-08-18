# Skirmish AI Row State Labels And Item Data - Ghidra Research Report

**Address(es):** `0x006AE6E0`, `0x006AE3F0`, `0x006ACEE0`, `0x006ADC20`, `0x006AE080`, `0x00687F10`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish dialog `0x102` AI row state combo labels, item data, tooltip strings, disabled sentinels, and Start Game packing/effect for row state values.  
**Non-Scope:** online/WOL player-state combo behavior except to distinguish `GUI:Closed`; country/color/start/team combo internals except disabled sentinels touched by AI row state; runtime CSF language-file text extraction beyond verified CSF keys.  
**Confidence:** High for binary labels, item data, tooltips, active-row predicates, Start writes, persisted snapshot conversion, and House difficulty handoff.  
**Active in YR:** Yes. `FUN_006AE3F0` routes dialog `0x102` init message `0x497` to `FUN_006AE6E0` and command `0x111` to `FUN_006ACEE0`; no TS-only gate is on this offline Skirmish route.

## 1. Overview

Offline Skirmish AI row state combos are controls `0x50B`, `0x50E`, `0x516`, `0x51A`, `0x51B`, `0x51C`, and `0x51D`. Each receives four entries in a fixed order: `GUI:None` with item data `-1`, `GUI:AIEasy` with item data `2`, `GUI:AINormal` with item data `1`, and `GUI:AIHard` with item data `0`.

The important parity detail is that the inactive offline Skirmish row is visibly `GUI:None`, not `GUI:Closed`. The row behaves like closed/inactive for enablement and Start packing, but the `GUI:Closed` key is loaded by netshare/WOL player-state code, not by the offline `0x102` combo population.

## 2. AI Row State Items

| Item data | Visible GUI key | Tooltip key while hovered | Row behavior | Active in YR / evidence |
|---:|---|---|---|---|
| `-1` | `GUI:None` | `STT:PlayerNone` | inactive row; disabled/sentinel sibling controls; excluded from active AI count and final AI arrays | Yes; inserted at `0x006AE7AA..0x006AE7D4`, tooltip at `0x006AE601..0x006AE61F`, inactive branch in `0x006ADC20` |
| `2` | `GUI:AIEasy` | `STT:PlayerDumbAI` | active Easy/Dumb AI row | Yes; inserted at `0x006AE7D8..0x006AE800`, tooltip at `0x006AE630..0x006AE64E`, Start accepts `2` |
| `1` | `GUI:AINormal` | `STT:PlayerSmartAI` | active Normal/Smart AI row | Yes; inserted at `0x006AE804..0x006AE82C`, tooltip at `0x006AE65F..0x006AE67D`, Start accepts `1` |
| `0` | `GUI:AIHard` | `STT:PlayerGeniusAI` | active Hard/Genius AI row | Yes; inserted at `0x006AE830..0x006AE858`, tooltip at `0x006AE68E..0x006AE6AB`, Start accepts `0` |

Insertion order matters for default selection: after adding all four entries, `FUN_006AE6E0` sends `CB_SETCURSEL` (`0x14E`) with index `0`, so the freshly reset combo selects the `GUI:None` / item data `-1` entry before persisted/default slot state is applied. Active in YR: Yes; evidence `0x006AE85A..0x006AE864`.

## 3. Closed, Observer, And Disabled Sentinels

`GUI:Closed` is not inserted by offline Skirmish `FUN_006AE6E0`. Ghidra xrefs for `GUI:Closed @ 0x00831C74` go to `FUN_005E87A0` and `FUN_005EB060`, which build/update netshare player-state records using `GUI:Open`, `GUI:Closed`, AI labels, `GUI:Waiting`, and `GUI:OpenObserver`. Active in YR: Conditional, but not active for standard offline `0x102` AI row combo population.

There is no observer item data in the offline AI row state combo. Observer is a country/side sentinel elsewhere (`-3`, e.g. `GUI:Observer`/observer flag paths), not a row-state item in `0x50B..0x51D`. Active in YR: Conditional outside this slice; not active in the scoped offline row-state combo. Evidence: `FUN_006AE6E0` inserts only item data `-1`, `2`, `1`, `0`; country observer was covered by prior side/country reports.

When the row state is not `0`, `1`, or `2`, `FUN_006ADC20` treats the row as inactive. It forces color, country, and start controls to item data `-2`, refreshes team to `-2` or Team D (`3`) depending on the selected mode team flag, then disables country/color/start/team windows. Active in YR: Yes; evidence `0x006ADC20`; the inactive branch calls `FUN_004E49A0(-2)`, `FUN_004E3F70(-2)`, and `FUN_004E5480(-2)`.

Init-time closed/inactive rows additionally call the team selector with `-2` before disabling siblings. Active in YR: Yes; evidence `0x006AEA01..0x006AEA83`.

Rows hidden by map player-count reduction are first set to item data `-1`, passed through `FUN_006ADC20`, and only then hidden. Active in YR: Yes; evidence `FUN_006AE080`.

## 4. Start Game Behavior

On Start Game or Back, `FUN_006ACEE0` scans the seven AI row-state combos and counts only selected item data `0`, `1`, or `2` into `DAT_00A8B274`. Item data `-1` is not counted. Active in YR: Yes; evidence `0x006AD043..0x006AD098`.

During final Start packing, only rows with item data `0`, `1`, or `2` write to the live AI arrays. Row state writes to `DAT_00A8B27C[slot]`, country to `DAT_00A8B29C[slot]`, color to `DAT_00A8B2BC[slot]`, start to `DAT_00A8B2DC[slot]`, and team to `DAT_00A8B2FC[slot]`. Active in YR: Yes; evidence `0x006AD453..0x006AD4E6`.

The saved/session snapshot converts selected row state to slot type codes: `-1 -> 1`, `0 -> 4`, `1 -> 5`, `2 -> 6`; other values fall through the binary expression to `0` or `6`. Active in YR: Yes; evidence `0x006AD5B0..0x006AD5E3`; readback mapping in `0x006AE931..0x006AE971` reverses `1 -> -1`, `4 -> 0`, `5 -> 1`, `6 -> 2`, else `-1`.

House creation later consumes `DAT_00A8B27C` as the AI difficulty index. `ScenarioClass__Create_Houses @ 0x00687F10` reads `piVar8[-8]`, optionally decrements it when `MultiPlayerAIDifficultyModifier` is active with more than one human and value is positive, then calls `HouseClass__SetDifficulty`. `HouseClass__SetDifficulty @ 0x004F6EC0` stores the argument at `HouseClass+0x184` and indexes difficulty tables with `param_2 * 0x50`. Active in YR: Yes; this is the standard scenario house creation path after Skirmish Start.

## 5. INI Keys

| INI key | YR value | Effect in this slice | Active in YR / evidence |
|---|---:|---|---|
| `[MultiplayerDialogSettings] AIDifficulty` | `0` | Parsed into `RulesClass+0x14A4`, but the offline row-state combo entries themselves are hardcoded by `FUN_006AE6E0`; persisted/default `Slot%02d` values choose selected row state. | Yes; `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`; `ini/rulesmd.ini:3027` |
| `[MultiplayerDialogSettings] AIPlayers` | `0` | Parsed into `RulesClass+0x14A8`; does not define the four visible row-state labels. | Yes; `0x00671EA0`; `ini/rulesmd.ini:3028` |
| `[Skirmish] Slot%02d` | profile-specific `type,country,color` | First value is persisted slot type code, converted to row item data during `FUN_006AE6E0` init. | Yes; `SessionClass__ReadSkirmishSettings @ 0x00697F10`; `FUN_00477440` |

## 6. Current Rust Implementation Status

`src/ui/skirmish_shell/state.rs` currently models opponents as `enabled: bool` plus country/color/start/team. It has no exact four-entry AI row-state combo, no item-data order `-1,2,1,0`, no per-row visible `GUI:None`/`GUI:AIEasy`/`GUI:AINormal`/`GUI:AIHard` labels, and no seven-slot `DAT_00A8B27C`-style difficulty array. `src/sim/game_options.rs` has one global `ai_difficulty` comment using `0=Easy, 1=Normal, 2=Hard`, which does not match the Skirmish combo's verified `0=Hard`, `1=Normal`, `2=Easy` item-data meaning.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline dialog `0x102` init and command dispatch | verified | `FUN_006AE3F0` | none |
| AI row-state combo IDs | verified | `FUN_006AE6E0`, `FUN_006ADC20`, `FUN_006ACEE0` | none |
| Visible GUI keys and item data | verified | assembly `0x006AE7AA..0x006AE858` | none |
| Hover tooltip keys per item | verified | assembly `0x006AE601..0x006AE6AB` | none |
| `GUI:Closed` exclusion from offline `0x102` row-state combo | verified | xrefs to `GUI:Closed @ 0x00831C74`; `FUN_005E87A0`, `FUN_005EB060`; no `FUN_006AE6E0` xref | none |
| Disabled/inactive sibling sentinel behavior | verified | `FUN_006ADC20`, `FUN_006AE080`, init branch `0x006AEA01..0x006AEA83` | exact visual disabled rendering is covered by owner-draw docs |
| Start active-row count and live arrays | verified | `FUN_006ACEE0` | none |
| Persisted slot type conversion | verified | `FUN_006ACEE0`, `FUN_006AE6E0`, `SessionClass__ReadSkirmishSettings`, `SessionClass__WriteSkirmishSettings` | profile file precedence outside standard `[Skirmish]` not expanded |
| House difficulty handoff | verified | `ScenarioClass__Create_Houses @ 0x00687F10`, `HouseClass__SetDifficulty @ 0x004F6EC0` | none |
| Retail CSF English values | deferred | keys verified, language file not extracted | optional asset-text pass if needed |

## 8. Open Questions - Final State

[RESOLVED] OQ-SCT-006 - Exact CSF display labels for AI row state item data `0`, `1`, `2`: `0 -> GUI:AIHard`, `1 -> GUI:AINormal`, `2 -> GUI:AIEasy`. The inactive sentinel is `-1 -> GUI:None`, not `GUI:Closed`. Evidence: `0x006AE7AA..0x006AE858`.

[RESOLVED] OQ-AIROW-001 - Which tooltip strings appear for the row-state items? `-1 -> STT:PlayerNone`, `2 -> STT:PlayerDumbAI`, `1 -> STT:PlayerSmartAI`, `0 -> STT:PlayerGeniusAI`. Evidence: `0x006AE601..0x006AE6AB`.

[RESOLVED] OQ-AIROW-002 - Does offline Skirmish use `GUI:Closed` in the AI row-state combo? No. `GUI:Closed` xrefs are netshare functions, while `FUN_006AE6E0` uses `GUI:None`. Evidence: xrefs to `0x00831C74`, disassembly `0x005E87C5`, `0x005EB2C8`, and `0x006AE7AC`.

[RESOLVED] OQ-AIROW-003 - What final Start values are live AI rows allowed to write? Only item data `0`, `1`, and `2` are counted and packed; `-1` is skipped. Evidence: `0x006AD043..0x006AD098`, `0x006AD453..0x006AD4E6`.

[RESOLVED] OQ-AIROW-004 - What does the persisted slot type store? It stores converted codes `-1 -> 1`, `0 -> 4`, `1 -> 5`, `2 -> 6`; readback maps those codes back to combo item data. Evidence: `0x006AD5B0..0x006AD5E3`, `0x006AE931..0x006AE971`.

[DEFERRED] OQ-AIROW-005 - Exact localized English text values inside retail `ra2md.csf`. Category: out-of-scope. Reason: this slice verified the CSF keys and binary use sites; extracting language-file values is an asset text pass, not needed to resolve item-data behavior.

## Sources

- Ghidra decompiled/read-only: `FUN_006AE3F0`, `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_006ADC20`, `FUN_006AE080`, `SessionClass__ReadSkirmishSettings @ 0x00697F10`, `SessionClass__WriteSkirmishSettings @ 0x00698F90`, `FUN_00477440`, `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`, `ScenarioClass__Create_Houses @ 0x00687F10`, `HouseClass__SetDifficulty @ 0x004F6EC0`.
- Ghidra assembly/read-only: `FUN_006AE6E0`, `FUN_006AE3F0`, `FUN_005E87A0`, `FUN_005EB060`.
- Ghidra strings: `GUI:None @ 0x0083FC88`, `GUI:AIEasy @ 0x00831C68`, `GUI:AINormal @ 0x00831C58`, `GUI:AIHard @ 0x00831C4C`, `GUI:Closed @ 0x00831C74`, `STT:PlayerNone @ 0x0083FC3C`, `STT:PlayerDumbAI @ 0x00831D0C`, `STT:PlayerSmartAI @ 0x00831CF8`, `STT:PlayerGeniusAI @ 0x00831CE4`, `STT:SkirmishComboAIPlayer @ 0x008353E4`.
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:3027..3028`; `rules.ini:2507..2508`.
- Prior reports: `SKIRMISH_PLAYER_AI_ROW_VISIBILITY_ENABLE_RULES_GHIDRA_REPORT.md`, `SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`, `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md`.
- Rust scan: `src/ui/skirmish_shell/state.rs`, `src/sim/game_options.rs`.
