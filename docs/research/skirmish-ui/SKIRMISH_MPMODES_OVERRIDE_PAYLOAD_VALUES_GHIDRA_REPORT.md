# Skirmish MPModes Override Payload Values - Ghidra Research Report

**Address(es):** `0x005D5B60`, `0x005D7590`, `0x005D7CE0`, `0x005D6130`, `0x005E7160`, `0x00671EA0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock offline Skirmish `MPModesMD.ini` rows, the retail override payload values that feed the common selected-mode object, and the binary readers/defaults for setup-visible `MustAlly`, `AlliesAllowed`, tournament flags, map filter, and random-map eligibility.  
**Non-Scope:** mode object construction beyond confirming reader inputs, post-start spawn internals, full Cooperative campaign payloads, full rules-override application, and online/WOL lobby behavior.  
**Confidence:** High for reader addresses, defaults, row fields, and common-object payload effects; Medium for exact MIX filename-to-payload attribution because the retail payloads are visible in `ra2md.mix` text but hashed MIX directory names were not extracted in this slice.  
**Active in YR:** Yes for stock offline Skirmish modes listed in `MPModesMD.ini`; Conditional for per-mode effects; No for stock offline Siege selection because no stock `[Siege]` roster row exists.

## 0. Working Notes

- Target question: Which stock offline Skirmish MPModes override payload fields are present, what exact selected-mode object fields read them, and which values should replace Rust's hardcoded stock defaults?
- Non-goals: Do not re-investigate full mode construction, post-start spawn internals, full rules application, or implement Rust.
- Evidence needed to mark COMPLETE: stock roster source, retail override payload source, common selected-mode reader address/defaults, map-filter/random-map reader path, and Rust surface scan.
- Stop conditions: Stop once common selected-mode object values and Rust-facing deltas are pinned; defer only MIX hash-name attribution and broader rules/session readers outside the selected-mode object.

## 1. Overview

`MPModesMD.ini` supplies the visible offline mode rows. Each row contributes a numeric id, UI label, tooltip, override filename, map-filter string, and random-map boolean. Active in YR: Yes; evidence `0x005D7590` opens `MPModesMD.ini`, `0x005D7CE0` registers the stock categories, and repo `ini/mpmodesmd.ini` matches the visible stock roster.

The common selected-mode constructor `0x005D5B60` reads only four `[MultiplayerDialogSettings]` booleans from each override payload into the mode object: `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`, and `MustAlly`. Active in YR: Yes; evidence decompile `0x005D5B60` plus key xrefs at `0x005D5CA7`, `0x005D5CC4`, `0x005D5CDF`, and `0x005D5CF7`.

Credits, unit count, tech level, game speed, and broader checkbox defaults are read by `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`, not by the selected-mode object constructor. Active in YR: Yes as rules/dialog settings, but not as MPModes object fields; evidence xrefs for those keys land in `0x00671EA0`, while `0x005D5B60` only references the four common-object keys above.

## 2. Class Layout / Key Offsets

| Field/global | Purpose | Verified behavior | Evidence | Active in YR |
|---|---|---|---|---|
| mode `+0x28` | numeric id | copied from roster row key and mirrored to `DAT_00A8B250` on selection | `0x005D7590`, `0x005E734F..0x005E737D` | Yes |
| mode `+0x2C` | override filename | used by constructor to open the mode's override INI | `0x005D5C2D..0x005D5C57` | Yes |
| mode `+0x30` | map filter | compared against map `GameModes`; empty map list accepts only `standard` | `0x005D6419`, `0x0069AE10` | Yes |
| mode `+0x34` | random maps allowed | copied from roster fifth token; random-map sentinel is mode-gated | `0x005D7590`; Rust already mirrors in `skirmish_scenarios.rs` | Yes |
| mode `+0x3C` | `AlliesAllowed` | default `1`, then override read | `0x005D5BF2`, `0x005D5CDF` | Yes / Conditional by selected mode |
| mode `+0x3D` | `WonlineTournamentAllowed` | default `1`, then override read | `0x005D5BEA`, `0x005D5CA7` | Conditional online flag, still constructed |
| mode `+0x3E` | `WonlineClanTournamentAllowed` | default `1`, then override read | `0x005D5BEE`, `0x005D5CC4` | Conditional online flag, still constructed |
| mode `+0x3F` | `MustAlly` | default `0`, then override read; cleared if `AlliesAllowed=0` | `0x005D5BF6`, `0x005D5CF7..0x005D5D11` | Yes for Team Game |
| `DAT_00A8B23C` | selected mode object | combo `0x6EB` item-data pointer is validated then committed | `0x005E71E5..0x005E7382` | Yes |

## 3. Core Logic

`0x005D7CE0` registers categories in this order: `Battle`, `ManBattle`, `Siege`, `Unholy`, `FreeForAll`, `Cooperative`. `0x005D7590` enumerates rows from `MPModesMD.ini`, parses five comma-separated row fields, constructs mode objects through the category factory, and keeps the object vector sorted by numeric id. Active in YR: Yes; evidence category xrefs `0x005D7D3C..0x005D7E24`, file string xref `0x005D759E`, and row factory call inside `0x005D7590`.

`0x005D5B60` initializes common selected-mode fields before reading override data: `AlliesAllowed=1`, `WonlineTournamentAllowed=1`, `WonlineClanTournamentAllowed=1`, and `MustAlly=0`. It then opens the row's override filename and reads `[MultiplayerDialogSettings]` in this order: `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`, `MustAlly`. If the final object has `MustAlly=1` and `AlliesAllowed=0`, it writes `MustAlly=0`. Active in YR: Yes; evidence decompile `0x005D5B60` and xrefs listed above.

Map filtering is not derived from the override payload. The row's fourth token is copied into the mode object and later compared against map records. Empty map `GameModes` vectors only match selected filter `standard`; otherwise the selected filter must equal one map `GameModes` entry. Active in YR: Yes; evidence `0x005D6419` and `0x0069AE10`.

Random-map eligibility is also row data, not override payload data. Stock Battle id `1` and Free For All id `2` have `randomMapsAllowed=true`; the other stock rows have false. Active in YR: Yes; evidence `ini/mpmodesmd.ini`, row reader `0x005D7590`, and Rust surface `src/skirmish_scenarios.rs:194`.

## 4. INI Keys

### Stock Exposed Roster

| Category | id | override file | map filter | random maps | Active in YR |
|---|---:|---|---|---|---|
| Battle | 1 | `MPBattleMD.ini` | `standard` | true | Yes |
| Battle | 9 | `MPTeamMD.ini` | `teamgame` | false | Yes |
| ManBattle | 5 | `MPMWMD.ini` | `megawealth` | false | Yes |
| ManBattle | 6 | `MPDuelMD.ini` | `duel` | false | Yes |
| ManBattle | 7 | `MPMeatMD.ini` | `meatgrind` | false | Yes |
| ManBattle | 8 | `MPNavalMD.ini` | `navalwar` | false | Yes |
| FreeForAll | 2 | `MPFreeForAllMD.ini` | `standard` | true | Yes |
| Unholy | 4 | `MPUnholyMD.ini` | `standard` | false | Yes |
| Cooperative | 3 | `MPCoopMD.ini` | `cooperative` | false | Yes |
| Siege | none | stock payload text exists | no stock row | none | No for stock offline selection |

Evidence: `ini/mpmodesmd.ini`, retail `ra2md.mix` text around payload comments, and binary reader `0x005D7590`. Active in YR: Yes for listed rows because offline Skirmish population uses these objects when `g_GameMode == 5` in `0x005D6130`.

### Stock Override Payload Values Read By `0x005D5B60`

| Payload comment / row | Common-object values | Resulting selected-mode object effect | Evidence | Active in YR |
|---|---|---|---|---|
| Battle / `MPBattleMD.ini` | `AlliesAllowed=yes` | `AlliesAllowed=1`, `MustAlly=0` | retail text line `110954`; reader `0x005D5CDF` | Yes |
| Team Game / `MPTeamMD.ini` | `AlliesAllowed=yes`, `AllyChangeAllowed=no`, `MustAlly=yes` | `AlliesAllowed=1`, `MustAlly=1`; `AllyChangeAllowed` is not a common-object field | retail text lines `111639..111641`; readers `0x005D5CDF`, `0x005D5CF7` | Yes |
| Mega Wealth / `MPMWMD.ini` | `AlliesAllowed=yes` | `AlliesAllowed=1`, `MustAlly=0` | retail text line `111312`; reader `0x005D5CDF` | Yes |
| Duel / `MPDuelMD.ini` | `AlliesAllowed=yes` | `AlliesAllowed=1`, `MustAlly=0` | retail text line `110979`; reader `0x005D5CDF` | Yes |
| Meat-Grinder / `MPMeatMD.ini` | `AlliesAllowed=yes` | `AlliesAllowed=1`, `MustAlly=0` | retail text line `111087`; reader `0x005D5CDF` | Yes |
| Naval / `MPNavalMD.ini` | `AlliesAllowed=yes` | `AlliesAllowed=1`, `MustAlly=0` | retail text line `111444`; reader `0x005D5CDF` | Yes |
| Free For All / `MPFreeForAllMD.ini` | `AlliesAllowed=no`, `WonlineClanTournamentAllowed=no` | `AlliesAllowed=0`, `MustAlly=0`, clan tournament flag false | retail text lines `111060..111061`; readers `0x005D5CC4`, `0x005D5CDF` | Yes |
| Unholy Alliance / `MPUnholyMD.ini` | `AlliesAllowed=yes` | `AlliesAllowed=1`, `MustAlly=0` | retail text line `111581`; reader `0x005D5CDF` | Yes |
| Cooperative / `MPCoopMD.ini` | `AlliesAllowed=no`, `WonlineTournamentAllowed=no` | `AlliesAllowed=0`, `MustAlly=0`, tournament flag false | retail text lines `111626..111627`; readers `0x005D5CA7`, `0x005D5CDF` | Yes |
| Siege payload | no common-object `[MultiplayerDialogSettings]` values found in the visible text scan before next payload boundary | no stock selectable object because no stock row | retail text lines `111559..111568`; no stock `[Siege]` row | No for stock offline selection |

### Adjacent Dialog/Rules Keys Not In The Selected-Mode Object

`RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` reads `MinMoney`, `Money`, `MaxMoney`, `MoneyIncrement`, `MinUnitCount`, `UnitCount`, `MaxUnitCount`, `TechLevel`, `GameSpeed`, `AIDifficulty`, `AIPlayers`, `BridgeDestruction`, `Shroud`, `Bases`, `Crates`, `HarvesterTruce`, `MultiEngineer`, `AlliesAllowed`, `AllyChangeAllowed`, `ShortGame`, `SuperWeaponsAllowed`, `BuildOffAlly`, `FogOfWar`, and `MCVRedeploys`. Active in YR: Yes as rules/dialog defaults; evidence decompile `0x00671EA0` and `rulesmd.ini:[MultiplayerDialogSettings]`.

This reader does not prove these keys are common selected-mode object fields. Active in YR: Yes as a negative object-layout fact; evidence `0x005D5B60` has no xrefs to the money/unit/speed/checklist strings, while xrefs for those strings resolve to `0x00671EA0` and related rules/session readers.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Mode roster load | `MPModesMD.ini` rows create mode objects sorted by id | `0x005D7590` | Yes |
| Offline mode combo | `0x005D6130` adds mode objects to control `0x6EB` when `g_GameMode == 5` and mode callbacks allow display | `0x005D6130` | Yes |
| Selection commit | `0x005E7160` reads combo `0x6EB` item data, validates, then commits `DAT_00A8B23C`, `DAT_00A8B250`, and `DAT_00A8B254` | `0x005E7160` | Yes |
| Map filtering | selected mode `+0x30` compared to map `GameModes` | `0x005D6419`, `0x0069AE10` | Yes |
| Team UI | `MustAlly` controls Team `None` and `-2` acceptance; `AlliesAllowed` controls ally helper | `0x005D5DC0..0x005D5E08`; prior team reports | Yes / Conditional |
| Start handoff | Start calls selected mode vtable `+0x14`; Battle/ManBattle accept, other categories can add side effects or validation | `0x006AD2BA..0x006AD34B`; sibling Start report | Yes / Conditional |

No simulation tick-cycle path is in this slice. These are shell/setup and launch-handoff values.

## 6. Current Rust Implementation Status

Rust now parses the stock roster and stores selected mode ids:

- `src/skirmish_modes.rs:21` defines `SkirmishGameMode` with id, label, override filename, map filter, random-map flag, `allies_allowed`, and `must_ally`.
- `src/skirmish_modes.rs:63` applies `apply_known_stock_dialog_defaults`, hardcoding the verified stock common-object values instead of reading MIX-backed override payloads.
- `src/skirmish_scenarios.rs:183` and `src/skirmish_scenarios.rs:194` filter maps and random-map sentinel visibility from selected mode fields.
- `src/ui/skirmish_shell/state.rs:127` and `src/ui/skirmish_shell/state.rs:503` keep selected mode id state.
- `src/ui/skirmish_shell/state.rs:1363` still launches `SkirmishLaunchMode::Battle`, and `src/skirmish_launch.rs:14` has only `Battle`.

Rust delta: roster/filter/random-map behavior is partially implemented; MIX-backed override payload parsing is missing; launch/session mode is still Battle-only; the Team Game `MustAlly` launch validation/UI restriction is not fully represented in launch mode/session.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MPModesMD.ini` row load | verified | `0x005D7590`, `ini/mpmodesmd.ini` | none |
| category registration | verified | `0x005D7CE0`, xrefs `0x005D7D3C..0x005D7E24` | none |
| common object defaults | verified | `0x005D5B60` | none |
| common object override reads | verified | `0x005D5CA7..0x005D5D11` | none |
| `MustAlly && !AlliesAllowed` clamp | verified | `0x005D5D05..0x005D5D11` | none |
| stock override common-object values | verified | retail `ra2md.mix` text lines `110954..111641`; readers in `0x005D5B60` | exact hashed MIX filename attribution |
| map filter behavior | verified | `0x005D6419`, `0x0069AE10` | none |
| random-map row field | verified | `0x005D7590`; `ini/mpmodesmd.ini` fifth token | none |
| general dialog/rules keys | touched-not-exhausted | `0x00671EA0` | full rules-override application outside selected-mode object |
| Start/spawn application of full override rules | deferred | out of target scope | separate launch/rules override investigation |
| Siege payload | verified for stock offline non-exposure | no `[Siege]` row; binary category and payload text exist | exact custom-data activation if modded row supplies Siege |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which stock override files are referenced by visible offline modes? -> `MPBattleMD.ini`, `MPTeamMD.ini`, `MPMWMD.ini`, `MPDuelMD.ini`, `MPMeatMD.ini`, `MPNavalMD.ini`, `MPFreeForAllMD.ini`, `MPUnholyMD.ini`, and `MPCoopMD.ini`.` (evidence: `ini/mpmodesmd.ini`; reader `0x005D7590`)
- `[RESOLVED] OQ-02 - Does stock offline Skirmish reference a Siege override row? -> No stock `[Siege]` roster row exists, although binary category support and payload text exist.` (evidence: `ini/mpmodesmd.ini`; category registration `0x005D7DA0`)
- `[RESOLVED] OQ-03 - What selected-mode object defaults exist before override? -> `AlliesAllowed=1`, `WonlineTournamentAllowed=1`, `WonlineClanTournamentAllowed=1`, `MustAlly=0`.` (evidence: `0x005D5BEA..0x005D5BF6`)
- `[RESOLVED] OQ-04 - Which override keys does the common selected-mode constructor read? -> Only `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`, and `MustAlly`.` (evidence: `0x005D5CA7..0x005D5D00`)
- `[RESOLVED] OQ-05 - What happens if payload says `MustAlly=yes` and `AlliesAllowed=no`? -> Constructor clears `MustAlly` back to false.` (evidence: `0x005D5D05..0x005D5D11`)
- `[RESOLVED] OQ-06 - Which stock selectable payload sets `MustAlly=yes`? -> Team Game only.` (evidence: retail text `111639..111641`; reader `0x005D5CF7`)
- `[RESOLVED] OQ-07 - Which stock selectable payloads disable common-object allies? -> Free For All and Cooperative set `AlliesAllowed=no`.` (evidence: retail text `111060`, `111626`; reader `0x005D5CDF`)
- `[RESOLVED] OQ-08 - Which stock selectable payloads disable tournament flags? -> FFA disables `WonlineClanTournamentAllowed`; Cooperative disables `WonlineTournamentAllowed`.` (evidence: retail text `111061`, `111627`; readers `0x005D5CC4`, `0x005D5CA7`)
- `[RESOLVED] OQ-09 - Are map filter and random-map eligibility payload override values? -> No; they come from roster row fields.` (evidence: `ini/mpmodesmd.ini`; reader `0x005D7590`; filter consumer `0x0069AE10`)
- `[RESOLVED] OQ-10 - Are Money/UnitCount/TechLevel/GameSpeed fields in the selected-mode object? -> No evidence in `0x005D5B60`; their key xrefs resolve to rules/session readers such as `0x00671EA0`.` (evidence: Ghidra key xrefs and decompile `0x00671EA0`)
- `[RESOLVED] OQ-11 - What Rust surface currently hardcodes this? -> `apply_known_stock_dialog_defaults` hardcodes common-object values.` (evidence: `src/skirmish_modes.rs:63`)
- `[RESOLVED] OQ-12 - Is Rust launch still Battle-only? -> Yes; `SkirmishLaunchMode` has only `Battle` and shell launch writes Battle.` (evidence: `src/skirmish_launch.rs:14`; `src/ui/skirmish_shell/state.rs:1363`)
- `[DEFERRED] OQ-13 - Exact MIX hash directory filename attribution for each visible payload.` (category: bounded-cost-too-high; reason: requires a dedicated MIX directory/name extraction pass; next-step-if-pursued: use the asset manager or an external MIX lister read-only to map payload offsets to hashed entries)
- `[DEFERRED] OQ-14 - Full application path for rules override payloads outside the selected-mode object.` (category: out-of-scope; reason: target only asks starting/options keys if reader paths are in selected-mode object; next-step-if-pursued: trace selected mode override filename through rules load/start scenario)
- `[DEFERRED] OQ-15 - Full Cooperative campaign payload fields in the large Cooperative object.` (category: out-of-scope; reason: common setup-visible object fields are resolved; next-step-if-pursued: Cooperative-specific mode investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Common selected-mode values come from each referenced override payload, not hardcoded filename branches: Team Game sets `MustAlly=yes`; FFA/Coop set `AlliesAllowed=no`; Coop/FFA also set tournament flags. | retail text `110954..111641`; binary readers `0x005D5CA7..0x005D5D11` | partial hardcoded stock values, no payload parser | `src/skirmish_modes.rs` and asset/MIX-backed INI loading surface | Load override INI payloads and merge only the common object fields with constructor defaults and the `MustAlly && !AlliesAllowed` clamp. | Opening stock assets builds 9 modes where Team Game has `must_ally=true`, FFA/Coop have `allies_allowed=false`, and other stock selectable modes preserve `allies_allowed=true`. Proposed test: `parses_stock_mpmodes_override_payload_values_from_mix`. | Do not keep expanding `apply_known_stock_dialog_defaults`; it hides modded/custom override behavior. |
| Map filter and random-map eligibility are row fields from `MPModesMD.ini`, not override payload fields. | `0x005D7590`; `0x005D6419`; `0x0069AE10`; `ini/mpmodesmd.ini` | mostly present | `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs` | Keep map filtering and random-map sentinel gating tied to selected `SkirmishGameMode` row fields. | Team Game filters to `teamgame` maps and hides Random Map; Battle/FFA allow Random Map. Proposed test: `selected_mode_row_fields_drive_filter_and_random_map_button`. | Do not infer filters from UI labels or override filenames. |
| Credits/unit count/tech/game-speed and most checkbox defaults are not common selected-mode object fields; they are read by `RulesClass__ReadMultiplayerDialogSettings`. | `0x00671EA0`; key xrefs for `Money`, `UnitCount`, `TechLevel`, `GameSpeed`, etc.; absence from `0x005D5B60` | current launch options use separate defaults; relation to mode override rules unchecked | `src/skirmish_launch.rs`, `src/sim/game_options.rs`, future rules-override loader | Keep mode object fields separate from global rules/dialog defaults; investigate full rules override application before using mode payloads to change launch option defaults. | Selecting Team Game changes team constraints/map filter but does not by itself rewrite credits/unit count unless a verified rules-reader path applies a payload override. Proposed test: `mode_common_payload_does_not_mutate_launch_option_defaults`. | Do not store all `[MultiplayerDialogSettings]` keys on `SkirmishGameMode` just because they share the section name. |

### Negative Facts / Do Not Do

- Do not expose Siege in stock offline Skirmish from binary category registration alone. Evidence: `0x005D7CE0` registers `Siege`, but stock `ini/mpmodesmd.ini` has no `[Siege]` row. Active in YR: No for stock offline selection.
- Do not let `MustAlly=yes` survive with `AlliesAllowed=no`. Evidence: constructor clears `MustAlly` at `0x005D5D05..0x005D5D11`. Active in YR: Yes.
- Do not treat base `rulesmd.ini:[MultiplayerDialogSettings] AlliesAllowed=no` as every selected mode's common-object value. Evidence: `0x005D5B60` defaults object `AlliesAllowed=1` and then reads per-mode override payloads; most stock selectable overrides set yes. Active in YR: Yes.
- Do not model `AllyChangeAllowed` as a common selected-mode object field. Evidence: Team Game payload contains it, but `0x005D5B60` does not read it; `AllyChangeAllowed` string xref is in `0x00671EA0`. Active in YR: Yes as rules/dialog setting, not this object field.
- Do not move money/unit/tech/speed defaults into `SkirmishGameMode` without a separate rules-override trace. Evidence: their key xrefs resolve to rules/session readers, not selected-mode constructor `0x005D5B60`. Active in YR: Yes as adjacent settings.

## Remaining Uncertainty

- Exact hashed MIX entry-to-filename attribution for the visible payload offsets remains Medium. The payload comments and `MPModesMD.ini` filenames line up with stock modes, but this slice did not extract the MIX directory.
- Full rules override application for non-common payload content, including unit `TechLevel` changes in Megawealth/Naval/etc., is outside this target.
- Cooperative-specific large-object fields beyond the common selected-mode payload were not re-investigated.

## Stale Docs / Follow-up Docs

Replace in `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_MPMODES_RETAIL_VALUES_AUDIT_GHIDRA_REPORT.md` the wording:

> Medium for exact stock override-file values because the local repo and plain retail install expose `mpmodesmd.ini` but not `MPBattleMD.ini`, `MPTeamMD.ini`, etc.

with:

> High for the common selected-mode object fields read by `0x005D5B60`: retail `ra2md.mix` contains visible stock override payloads showing Battle/Duel/Meat-Grinder/Mega Wealth/Naval/Unholy set `AlliesAllowed=yes`, Free For All sets `AlliesAllowed=no` and `WonlineClanTournamentAllowed=no`, Cooperative sets `AlliesAllowed=no` and `WonlineTournamentAllowed=no`, and Team Game sets `AlliesAllowed=yes`, `AllyChangeAllowed=no`, and `MustAlly=yes`. Exact MIX hash filename attribution remains a separate extraction concern.

## Sources

- Ghidra read-only decompile/xrefs: `0x005D5B60`, `0x005D7590`, `0x005D7CE0`, `0x005D6130`, `0x005E7160`, `0x0069AE10`, `0x00671EA0`, `0x004E4170`.
- Ghidra xrefs: `MustAlly` at `0x005D5CF7`; `AlliesAllowed` at `0x005D5CDF` and `0x00672148`; `WonlineClanTournamentAllowed` at `0x005D5CC4`; `WonlineTournamentAllowed` at `0x005D5CA7`; `MPModesMD.ini` at `0x005D759E`.
- INI/data checked: `C:/Users/enok/Documents/ra2-rust-game/ini/mpmodesmd.ini`; `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`; retail text scan of `C:/Users/enok/Documents/Command and Conquer Red Alert II/ra2md.mix`.
- Prior reports used for context: `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_RETAIL_VALUES_AUDIT_GHIDRA_REPORT.md`, `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`.
- Rust scan: `src/skirmish_modes.rs`, `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`.
