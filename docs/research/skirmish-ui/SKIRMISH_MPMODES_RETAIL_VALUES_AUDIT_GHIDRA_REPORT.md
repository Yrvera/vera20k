# Skirmish MPModes Retail Values Audit - Ghidra Research Report

**Address(es):** `0x005D7590`, `0x005D7CE0`, `0x005D6130`, `0x005D5DC0`, `0x005D5DD0`, `0x005D5DE0`, `0x0069AE10`, `0x005D6419`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** Retail/repo INI data that creates offline Skirmish multiplayer mode objects, plus binary-confirmed keys already tied to Team `None`, allies/team controls, Start acceptance, and map `GameModes` filtering.  
**Non-Scope:** Gameplay spawn placement, full WOL/online team behavior, map preview decoding, and complete extraction of per-mode override files not present as plain local INI files.  
**Confidence:** High for binary key names, mode roster, defaults, effects of `MustAlly` / `AlliesAllowed`, and the common selected-mode object fields read by `0x005D5B60`. `SKIRMISH_MPMODES_OVERRIDE_PAYLOAD_VALUES_GHIDRA_REPORT.md` verifies visible stock override payload values for the common fields: Battle/Duel/Meat-Grinder/Mega Wealth/Naval/Unholy set `AlliesAllowed=yes`; Free For All sets `AlliesAllowed=no` and `WonlineClanTournamentAllowed=no`; Cooperative sets `AlliesAllowed=no` and `WonlineTournamentAllowed=no`; Team Game sets `AlliesAllowed=yes`, `AllyChangeAllowed=no`, and `MustAlly=yes`. Exact hashed MIX entry-to-filename attribution remains a separate extraction concern.  
**Active in YR:** Yes / Conditional. `MPModesMD.ini` is loaded by the YR binary path at `0x005D7590`, and offline Skirmish category population checks `g_GameMode == 5` at `0x005D6130`; individual mode behaviors are conditional on selected mode id/category.

## 1. Overview

The retail MPModes data visible in this workspace is `ini/mpmodesmd.ini`. It defines nine selectable mode ids grouped under six binary-known categories: `Battle`, `ManBattle`, `FreeForAll`, `Unholy`, `Cooperative`, and the binary-known but no-local-entry `Siege` category.

`MustAlly` and `AlliesAllowed` are not present in the exposed `mpmodesmd.ini` roster file or in `[MultiplayerDialogSettings]`; they are binary-read mode/rules keys with constructor defaults. Missing `MustAlly` defaults false, making Team `None` available through vtable `+0x2C`; missing `AlliesAllowed` defaults true inside the mode object unless an override INI says otherwise.

## 2. Class Layout / Key Offsets

| Item | Purpose | Evidence | Active in YR |
|---|---|---|---|
| mode object `+0x30` | selected mode's map-filter/category string used against map `GameModes` | `0x005D6419` adds `0x30`, then calls `0x0069AE10`; `mpmodesmd.ini` 4th column | Yes |
| mode object `+0x3C` | `AlliesAllowed` byte, also used by vtable `+0x34` style helper at `0x005D5DD0` | string `0x008308AC`; `0x005D5DD0` reads `[ECX+0x3C]` | Conditional by mode data/default |
| mode object `+0x3F` | `MustAlly` byte; Team `None` suppression and team-value validation helper input | string `0x008308A0`; `0x005D5DC0` / `0x005D5DE0` read `[ECX+0x3F]` | Conditional by mode data/default |
| map record `+0x1A8`, `+0x1B4` | parsed map `GameModes` string-vector pointer/count | `0x0069AE10` loop over `record+0x1A8`, count `record+0x1B4` | Yes for Choose Map filtering |

## 3. Core Logic

### Mode Roster Loading

`0x005D7CE0` registers factory/category strings in this order: `Battle`, `ManBattle`, `Siege`, `Unholy`, `FreeForAll`, `Cooperative`. Each registration uses the category string and a factory vtable before invoking the shared loader path.

Active in YR: Yes. Evidence: category pushes at `0x005D7D3C` (`Battle`), `0x005D7D74` (`ManBattle`), `0x005D7DA0` (`Siege`), `0x005D7DCC` (`Unholy`), `0x005D7DF8` (`FreeForAll`), `0x005D7E24` (`Cooperative`), and loader string `MPModesMD.ini` at `0x005D759E`.

### Team `None` Availability

The selected-mode vtable helper at `0x005D5DC0` returns `-2` when `MustAlly` at `+0x3F` is zero and `0` when nonzero. `FUN_004E5B60` inserts Team `None` only on the negative return.

Active in YR: Yes for standard offline Skirmish team combo rebuild; the visible `None` row is conditional by selected mode object's `MustAlly`. Evidence: `0x005D5DC0..0x005D5DCD`, plus prior team-control report verifying `FUN_004E5B60` use.

### AlliesAllowed And Team Value Acceptance

`0x005D5DD0` reads `AlliesAllowed` at `+0x3C` and returns `-2` when false or `3` when true. `0x005D5DE0` validates a proposed team item-data value against `MustAlly`: when `MustAlly` is true, `-2` is rejected; otherwise values `0..3` are accepted and other negatives/out-of-range values reject.

Active in YR: Conditional by selected mode data/default. Evidence: `0x005D5DD0..0x005D5DDD` and `0x005D5DE0..0x005D5E08`.

### Map GameModes Matching

When a map record has zero parsed `GameModes`, `0x0069AE10` accepts it only for selected mode string `standard`. When the map record has one or more entries, the selected mode's string at `mode+0x30` is compared against each map string in insertion order.

Active in YR: Yes for Choose Map filtering. Evidence: `0x005D6419..0x005D641F`; `0x0069AE15` zero-count branch compares against string `standard`; `0x0069AE33..0x0069AE65` loops `record+0x1A8`.

## 4. INI Keys

| File / section / key | Observed value or default | Effect | Evidence | Active in YR |
|---|---|---|---|---|
| `ini/mpmodesmd.ini:[Battle]` id `1` | `GUI:Battle`, `STT:ModeBattle`, `MPBattleMD.ini`, `standard`, `true` | Battle category, standard map filter, random maps allowed by data | `ini/mpmodesmd.ini:7..8`; binary category `Battle` | Yes |
| `ini/mpmodesmd.ini:[Battle]` id `9` | `GUI:TeamGame`, `STT:ModeTeamGame`, `MPTeamMD.ini`, `teamgame`, `false` | Battle-family team mode, teamgame map filter | `ini/mpmodesmd.ini:9` | Yes, if selected |
| `ini/mpmodesmd.ini:[ManBattle]` ids `5..8` | Megawealth/Duel/MeatGrind/NavalWar override files, filters `megawealth`, `duel`, `meatgrind`, `navalwar`, all random false | ManBattle category modes; Start acceptance uses ManBattle concrete method unless subclass overrides elsewhere | `ini/mpmodesmd.ini:12..16`; binary category `ManBattle` | Conditional |
| `ini/mpmodesmd.ini:[FreeForAll]` id `2` | `MPFreeForAllMD.ini`, `standard`, `true` | FFA category, standard map filter, random maps allowed | `ini/mpmodesmd.ini:19..20`; `0x005C5D40` acceptance side effect in sibling report | Conditional |
| `ini/mpmodesmd.ini:[Unholy]` id `4` | `MPUnholyMD.ini`, `standard`, `false` | Unholy category; Start can reject when global enable byte is unset | `ini/mpmodesmd.ini:22..23`; `0x005CB400` sibling report | Conditional |
| `ini/mpmodesmd.ini:[Cooperative]` id `3` | `MPCoopMD.ini`, `cooperative`, `false` | Cooperative category and map filter; two-node pre-call path | `ini/mpmodesmd.ini:26..27`; `0x005C1D80` sibling report | Conditional |
| `Siege` category | no entry in exposed `ini/mpmodesmd.ini` | Binary supports category and Start validation, but this local retail/repo roster does not list a selectable Siege mode | `0x005D7DA0`; no `[Siege]` lines in `ini/mpmodesmd.ini` | No for exposed stock local roster; binary path conditional if data supplies it |
| `MustAlly` | constructor default false if absent | false allows Team `None`; true suppresses `None` and rejects proposed `-2` team value | string `0x008308A0`; `0x005D5DC0`, `0x005D5DE0` | Conditional |
| `AlliesAllowed` | constructor default true in mode object; `[MultiplayerDialogSettings]` has `AlliesAllowed=no` as dialog/session default | false clears `MustAlly` during construction per sibling report and makes helper return `-2`; dialog default disables allied starts unless overridden | string `0x008308AC`; `0x005D5DD0`; `rulesmd.ini:3011,3038` | Conditional |
| `[MultiplayerDialogSettings] ShortGame/Crates/MCVRedeploys` | `yes` in base and YR rules | dialog defaults adjacent to Skirmish setup, not MPModes object fields | `rules.ini:2488..2521`; `rulesmd.ini:3007..3041` | Yes for dialog defaults, not mode selection |

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| MPModes file load | Opens `MPModesMD.ini` through INI parser before mode enumeration | `0x005D759E` pushes `0x00830A18` | Yes |
| Offline Skirmish category population | Compares `g_GameMode` to `5` and uses mode object list for dialog category/mode controls | `0x005D6130..0x005D6156` | Yes for local Skirmish |
| Team controls | Team combo `None` presence comes from selected mode vtable `+0x2C` and `MustAlly` | `0x005D5DC0`; sibling `SKIRMISH_TEAM_NONE_INSERTION...` | Conditional |
| Start acceptance | Start Game dispatches selected mode vtable `+0x14`; concrete behavior differs by category | sibling `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE...` | Conditional by selected category |
| Choose Map filtering | selected mode filter string compared against map record `GameModes`; empty map list means `standard` only | `0x005D6419`; `0x0069AE10` | Yes |

## 6. Current Rust Implementation Status

No direct Rust implementation of `MPModesMD.ini` mode-object loading, mode filter strings, `MustAlly`, or `AlliesAllowed`-driven Team `None` suppression was found by scoped search for `MPModesMD`, `MPBattleMD`, `MustAlly`, and `AlliesAllowed` under `src/`.

Active in YR: This is a Rust implementation status note, not binary behavior.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MPModesMD.ini` mode roster | verified | `ini/mpmodesmd.ini:7..27`; `0x005D759E` | none for exposed roster |
| binary category registrations | verified | `0x005D7D3C..0x005D7E24` | none |
| Team `None` rule from `MustAlly` | verified | `0x005D5DC0..0x005D5DCD` | exact override-file values if files are later extracted |
| `AlliesAllowed` helper | verified | `0x005D5DD0..0x005D5DDD` | downstream online/team use outside scope |
| team value validation helper | verified | `0x005D5DE0..0x005D5E08` | exact callers outside Team UI not exhausted |
| map `GameModes` filter matching | verified | `0x005D6419`, `0x0069AE10` | map corpus audit out of scope |
| per-mode override INI payloads | touched-not-exhausted | filenames in `ini/mpmodesmd.ini`; no plain local files found under retail root; repo `ini/` only has `mpmodesmd.ini` for this family | extract/identify `MPBattleMD.ini`, `MPTeamMD.ini`, etc. from archives with a dedicated content tool |

## 8. Open Questions - Final State

[RESOLVED] OQ-MP-001 - Which MPModes file is active in YR? `MPModesMD.ini`; evidence string `0x00830A18` pushed at `0x005D759E`.

[RESOLVED] OQ-MP-002 - Which categories are binary-known? `Battle`, `ManBattle`, `Siege`, `Unholy`, `FreeForAll`, `Cooperative`; evidence `0x005D7D3C..0x005D7E24`.

[RESOLVED] OQ-MP-003 - Which categories are present in exposed repo retail data? `Battle`, `ManBattle`, `FreeForAll`, `Unholy`, `Cooperative`; no `[Siege]` section in `ini/mpmodesmd.ini:7..27`.

[RESOLVED] OQ-MP-004 - What is the default impact of missing `MustAlly`? Constructor/default path leaves it false; vtable `+0x2C` then returns `-2`, enabling Team `None`. Evidence: string `0x008308A0`, `0x005D5DC0`.

[RESOLVED] OQ-MP-005 - How are mode strings used for map filtering? `mode+0x30` is compared against map record `GameModes`; empty map list accepts only `standard`. Evidence: `0x005D6419`, `0x0069AE10`.

[DEFERRED] OQ-MP-006 - Exact values inside `MPBattleMD.ini`, `MPTeamMD.ini`, `MPFreeForAllMD.ini`, `MPUnholyMD.ini`, `MPCoopMD.ini`, `MPMWMD.ini`, `MPDuelMD.ini`, `MPMeatMD.ini`, and `MPNavalMD.ini`. Category: bounded-cost-too-high. Reason: not present as plain local retail or repo INI files in this slot; needs a dedicated archive content extraction/listing tool that does not modify repo files.

## Sources

- Ghidra read-only string search: `0x008308A0` `MustAlly`, `0x008308AC` `AlliesAllowed`, `0x00830A18` `MPModesMD.ini`, category strings `0x00830BCC..0x00830C00`.
- Ghidra assembly context: `0x005D7590`, `0x005D7CE0`, `0x005D7D3C`, `0x005D7DA0`, `0x005D7DF0`, `0x005D7E18`, `0x005D6130`, `0x005D5DC0`, `0x005D5DD0`, `0x005D5DE0`, `0x005D6419`, `0x0069AE10`.
- INI files checked: `C:/Users/enok/Documents/ra2-rust-game/ini/mpmodesmd.ini`, `rules.ini`, `rulesmd.ini`.
- Retail root checked for plain override INIs: `C:/Users/enok/Documents/Command and Conquer Red Alert II/` had no top-level `MP*.ini` files.
- Prior reports: `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md`, `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`.
