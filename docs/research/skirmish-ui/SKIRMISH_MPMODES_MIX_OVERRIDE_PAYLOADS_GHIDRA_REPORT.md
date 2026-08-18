# Skirmish MPModes MIX Override Payloads - Ghidra Research Report

**Address(es):** `0x005D5B60`, `0x005D7590`, `0x005D7CE0`, `0x005D6130`, `0x005E7160`, `0x00671EA0`, `0x0069AE10`, `0x004E4170`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Which `MP*.ini` override payload fields referenced by stock `MPModesMD.ini` affect offline Skirmish setup/session mode behavior for Battle, Team Game, Free For All, Unholy Alliance, and Cooperative; constructor defaults; MIX-backed load path; and Rust parse/do-not-hardcode handoff.  
**Non-Scope:** online/WOL lobby behavior, multiplayer netcode, Choose Map rediscovery beyond selected-mode/filter evidence, player-name/status behavior, full Cooperative campaign internals, and full rules-override application for non-common payload sections.  
**Confidence:** High for constructor defaults, common-object field readers, row fields, active offline path, and relevant stock payload values; Medium for exact hashed MIX entry attribution because this report used visible retail `ra2md.mix` text payloads and binary readers, not a dedicated MIX directory-name extraction pass.  
**Active in YR:** Yes for the stock offline Skirmish rows listed in `MPModesMD.ini`; Conditional for per-mode field effects; No for stock offline Siege selection.

## 0. Working Notes

- Target question: Which `MP*.ini` override payload fields loaded from MPModes affect offline Skirmish setup/session behavior, with defaults and stock payload values for Battle/Team/FFA/Unholy/Coop?
- Non-goals: online lobby/network protocol, Choose Map rediscovery, player-name/status behavior, broad session launch validation unrelated to MPModes payloads.
- Evidence needed to mark COMPLETE: constructor defaults, MIX-backed override load path, per-key binary reads/defaults, stock payload values from retail INI/MIX source, current Rust deltas, and negative facts for fields not affecting offline Skirmish.
- Stop conditions: all scoped override payload keys and entry points resolved or explicitly deferred; one zero-add verification pass over primary function/callees; report written only to the requested path, with shared claims optionally updated.

## 1. Overview

Offline Skirmish does not get its setup mode behavior from one hardcoded Battle path. `MPModesMD.ini` is loaded by `0x005D7590`, categories are registered by `0x005D7CE0`, and each row supplies a mode id, display key, tooltip key, override filename, map filter, and random-map flag. Active in YR: Yes; `0x005D6130` then populates the Skirmish mode control when `g_GameMode == 5`.

The common mode-object constructor `0x005D5B60` opens the row's override filename through `CCFileClass`/`CDFileClass`, so archive-backed `MP*.ini` payloads are live. From `[MultiplayerDialogSettings]` it reads exactly four common mode-object booleans: `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`, and `MustAlly`. Active in YR: Yes; these fields drive setup/session behavior through mode listing, team/alliance helpers, selected-mode state, and Start/session handoff.

Most other `[MultiplayerDialogSettings]` names in `MP*.ini` or `rulesmd.ini` are not common MPModes object fields. `Money`, `UnitCount`, `TechLevel`, `GameSpeed`, `AllyChangeAllowed`, `ShortGame`, `BuildOffAlly`, `FogOfWar`, and similar keys are read by `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`, not by `0x005D5B60`. Active in YR: Yes as dialog/rules settings, but not as the selected mode object's common fields.

## 2. Class Layout / Key Offsets

| Field / source | Purpose | Verified behavior | Evidence | Active in YR |
|---|---|---|---|---|
| mode `+0x28` | numeric mode id | copied from MPModes row key and committed to selected-mode id mirror | row reader `0x005D7590`; selection commit `0x005E734F..0x005E737D` | Yes |
| mode `+0x2C` | override filename | `0x005D5B60` builds a file object from this string and loads it before reading override keys | decompile `0x005D5B60`; assembly `0x005D5C2D..0x005D5C57` | Yes |
| mode `+0x30` | map filter string | compared against map `GameModes`; empty map list matches only `standard` | `0x005D6419`; `0x0069AE10`; `MPModesMD.ini` row field | Yes |
| mode `+0x34` | random maps allowed | copied from fifth MPModes row token; random-map sentinel is mode-gated | row reader `0x005D7590`; `MPModesMD.ini` | Yes |
| mode `+0x3C` | `AlliesAllowed` common byte | constructor default true, then override read; downstream ally helper returns `3` when true and `-2` when false | defaults `0x005D5BF2`; reader `0x005D5CDF`; helper `0x005D5DD0` | Conditional by selected mode |
| mode `+0x3D` | `WonlineTournamentAllowed` | constructor default true, then override read; used by online/tournament gates, still constructed in offline object | defaults `0x005D5BEA`; reader `0x005D5CA7`; listing gate `0x005D6130` | Conditional; not an offline-only behavior |
| mode `+0x3E` | `WonlineClanTournamentAllowed` | constructor default true, then override read; used by online/tournament gates, still constructed in offline object | defaults `0x005D5BEE`; reader `0x005D5CC4`; listing gate `0x005D6130` | Conditional; not an offline-only behavior |
| mode `+0x3F` | `MustAlly` common byte | constructor default false, then override read; if true while `AlliesAllowed` is false, constructor clears it | defaults `0x005D5BF6`; reader/clamp `0x005D5CF7..0x005D5D11`; team helper `0x005D5DC0` | Yes for Team Game |
| `DAT_00A8B23C` | selected mode object | mode control `0x6EB` item data is temporarily tested, then committed with selected map index | `0x005E7160`; `0x005E71E5..0x005E7382` | Yes |

## 3. Core Logic

### 3.1 MPModes row load

`0x005D7CE0` registers categories in order: `Battle`, `ManBattle`, `Siege`, `Unholy`, `FreeForAll`, `Cooperative`. It calls `0x005D7590` for each category. Active in YR: Yes; assembly pushes and calls appear at `0x005D7D3C..0x005D7E36`.

`0x005D7590` opens `MPModesMD.ini` (`0x005D759E` pushes string `0x00830A18`), enumerates each registered section, parses five comma-separated row tokens, calls the category factory, and inserts mode objects sorted by `mode+0x28`. Active in YR: Yes; stock offline rows are ids `1..9`, with no stock `[Siege]` row.

### 3.2 MIX-backed override load and constructor defaults

`0x005D5B60` receives the MPModes row fields and initializes the common object before reading any override payload:

| Field | Native default | Evidence | Active in YR |
|---|---:|---|---|
| `WonlineTournamentAllowed` `+0x3D` | true | `MOV byte ptr [ESI + 0x3d],0x1` at `0x005D5BEA` | Yes, conditional use |
| `WonlineClanTournamentAllowed` `+0x3E` | true | `MOV byte ptr [ESI + 0x3e],0x1` at `0x005D5BEE` | Yes, conditional use |
| `AlliesAllowed` `+0x3C` | true | `MOV byte ptr [ESI + 0x3c],0x1` at `0x005D5BF2` | Yes |
| `MustAlly` `+0x3F` | false | `MOV byte ptr [ESI + 0x3f],BL` at `0x005D5BF6`, with `BL=0` in this path | Yes |

The constructor then builds a file object from `mode+0x2C`, calls the file-load path, and uses the loaded INI object to query `[MultiplayerDialogSettings]`. Active in YR: Yes; decompile `0x005D5B60`, assembly `0x005D5C2D..0x005D5C57`. This is the binary evidence that stock or modded `MP*.ini` override files should be loaded through the asset/MIX lookup path, not approximated by filename switches.

The common field read order is fixed:

1. `WonlineTournamentAllowed` at `0x005D5CA7`
2. `WonlineClanTournamentAllowed` at `0x005D5CC4`
3. `AlliesAllowed` at `0x005D5CDF`
4. `MustAlly` at `0x005D5CF7`

After reading `MustAlly`, native enforces `MustAlly=false` when `AlliesAllowed=false`: `CMP AL,BL`, write `+0x3F`, test `+0x3C`, then `MOV byte ptr [ESI + 0x3f],BL` at `0x005D5D05..0x005D5D11`. Active in YR: Yes; this affects Team Game style team restrictions and prevents contradictory payload values from surviving.

### 3.3 Offline setup/session consumers

Mode list population `0x005D6130` clears control `0x6EB`, iterates the constructed mode vector, applies tournament gates only in online mode (`g_GameMode == 4`), applies mode callbacks, and for offline Skirmish (`g_GameMode == 5`) calls vtable `+0x40` before adding visible rows. Active in YR: Yes for offline row population; evidence `0x005D6130`.

Map filtering is row-field driven, not override-payload driven. `mode+0x30` is compared against scenario `GameModes`; records with no parsed `GameModes` match only literal `standard`. Active in YR: Yes; evidence `0x005D6419` caller and `0x0069AE10`.

Team and alliance setup are common-payload-field driven:

| Native helper | Input field | Behavior | Evidence | Active in YR |
|---|---|---|---|---|
| vtable `+0x2C`, base `0x005D5DC0` | `MustAlly` `+0x3F` | returns `-2` when `MustAlly=0`, else `0`; this is the Team `None` availability/default source | assembly `0x005D5DC0..0x005D5DCD` | Yes |
| vtable `+0x30`, base `0x005D5DD0` | `AlliesAllowed` `+0x3C` | returns `3` when allies are allowed, else `-2` | assembly `0x005D5DD0..0x005D5DDD` | Yes/Conditional |
| vtable `+0x34`, base `0x005D5DE0` | `MustAlly` `+0x3F` plus proposed team value | rejects `-2` when `MustAlly=1`; accepts `0..3`; rejects other values | assembly `0x005D5DE0..0x005D5E08` | Yes |
| country/default helper `0x004E4170` | selected mode vtable `+0x28` fallback | if combo item data is outside `[-3,9]`, calls selected mode fallback; no selected mode falls back to `-2` | decompile `0x004E4170` | Yes |

Selection commit `0x005E7160` reads mode control `0x6EB` item data, temporarily writes `DAT_00A8B23C`, validates through selected-mode callbacks, restores the old pointer for rejection, then commits selected mode pointer/id and selected map index. Active in YR: Yes; evidence `0x005E71E5..0x005E7382`.

## 4. INI Keys And Stock Payload Values

### 4.1 Stock rows in `MPModesMD.ini`

| Stock mode | id | category | override file | map filter | random maps | Active in YR |
|---|---:|---|---|---|---:|---|
| Battle | 1 | `Battle` | `MPBattleMD.ini` | `standard` | true | Yes |
| Team Game | 9 | `Battle` | `MPTeamMD.ini` | `teamgame` | false | Yes |
| Free For All | 2 | `FreeForAll` | `MPFreeForAllMD.ini` | `standard` | true | Yes |
| Unholy Alliance | 4 | `Unholy` | `MPUnholyMD.ini` | `standard` | false | Yes |
| Cooperative | 3 | `Cooperative` | `MPCoopMD.ini` | `cooperative` | false | Yes |

Evidence: `ini/mpmodesmd.ini`; binary row reader `0x005D7590`. `ManBattle` modes also exist in stock data, but this report only uses them as context and does not claim their gameplay-specific payload behavior. `Siege` is registered by the binary but has no stock offline row. Active in YR: No for stock offline Siege selection.

### 4.2 Common-object fields read from `MP*.ini`

These are the only payload fields from `[MultiplayerDialogSettings]` that `0x005D5B60` reads into the common selected-mode object:

| Key | Native default if absent | Field | Reader evidence | Offline setup/session effect | Active in YR |
|---|---:|---|---|---|---|
| `WonlineTournamentAllowed` | true | `+0x3D` | `0x005D5CA7`; xref only in `0x005D5B60` | constructed on every mode object; offline behavior not the main consumer | Conditional |
| `WonlineClanTournamentAllowed` | true | `+0x3E` | `0x005D5CC4`; xref only in `0x005D5B60` | constructed on every mode object; offline behavior not the main consumer | Conditional |
| `AlliesAllowed` | true | `+0x3C` | `0x005D5CDF`; also rules-reader xref `0x00672148` for separate dialog defaults | controls ally/team helper behavior; also clears contradictory `MustAlly` | Yes/Conditional |
| `MustAlly` | false | `+0x3F` | `0x005D5CF7`; xref only in `0x005D5B60` | suppresses Team `None` and rejects `-2` team value | Yes for Team Game |

### 4.3 Relevant stock payload values from retail `ra2md.mix`

Retail `ra2md.mix` contains readable override payload text. The rows below cite the visible text lines from `rg -a` plus the binary reader addresses above. Active in YR: Yes for the stock selectable rows because `0x005D5B60` opens the row's override filename through the normal file/archive path.

| Mode / override | Payload values relevant to this slice | Resulting common object state | Evidence | Active in YR |
|---|---|---|---|---|
| Battle / `MPBattleMD.ini` | `AlliesAllowed=yes` | `AlliesAllowed=true`, `MustAlly=false`, tournament flags remain defaults | retail `ra2md.mix` text `110946..110954`; reader `0x005D5CDF`; defaults `0x005D5BEA..0x005D5BF6` | Yes |
| Team Game / `MPTeamMD.ini` | `AlliesAllowed=yes`, `AllyChangeAllowed=no`, `MustAlly=yes` | `AlliesAllowed=true`, `MustAlly=true`; `AllyChangeAllowed` is not a common mode-object field | retail text `111631..111641`; readers `0x005D5CDF`, `0x005D5CF7`; `AllyChangeAllowed` xref only at `0x00672168` | Yes |
| Free For All / `MPFreeForAllMD.ini` | `AlliesAllowed=no`, `WonlineClanTournamentAllowed=no` | `AlliesAllowed=false`, `MustAlly=false`, clan tournament flag false | retail text `111050..111061`; readers `0x005D5CC4`, `0x005D5CDF`; clamp `0x005D5D05..0x005D5D11` | Yes |
| Unholy Alliance / `MPUnholyMD.ini` | `AlliesAllowed=yes` | `AlliesAllowed=true`, `MustAlly=false`, tournament flags remain defaults | retail text `111568..111581`; reader `0x005D5CDF` | Yes |
| Cooperative / `MPCoopMD.ini` | `AlliesAllowed=no`, `WonlineTournamentAllowed=no` | `AlliesAllowed=false`, `MustAlly=false`, tournament flag false | retail text `111622..111627`; readers `0x005D5CA7`, `0x005D5CDF`; clamp `0x005D5D05..0x005D5D11` | Yes |

### 4.4 Adjacent keys not part of the common MPModes object

`RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` reads `MinMoney`, `Money`, `MaxMoney`, `MoneyIncrement`, `MinUnitCount`, `UnitCount`, `MaxUnitCount`, `TechLevel`, `GameSpeed`, `AIDifficulty`, `AIPlayers`, `BridgeDestruction`, `ShadowGrow`, `Shroud`, `Bases`, `TiberiumGrows`, `Crates`, `CaptureTheFlag`, `HarvesterTruce`, `MultiEngineer`, `AlliesAllowed`, `AllyChangeAllowed`, `ShortGame`, `SuperWeaponsAllowed`, `BuildOffAlly`, `FogOfWar`, and `MCVRedeploys`. Active in YR: Yes as rules/dialog settings. Evidence: decompile `0x00671EA0`, `rulesmd.ini:[MultiplayerDialogSettings]` lines around `3017..3042`.

Negative common-object fact: these keys are not read by `0x005D5B60` except `AlliesAllowed`, and the `AlliesAllowed` xrefs prove two different consumers: common MPModes object at `0x005D5CDF`, and RulesClass dialog defaults at `0x00672148`. Active in YR: Yes; implementers must keep these surfaces separate.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Stock mode-object creation | `MPModesMD.ini` rows create sorted mode objects | `0x005D7590`; `MPModesMD.ini` | Yes |
| MIX-backed override payload load | constructor uses row override filename and file/archive load path before INI reads | `0x005D5B60`; assembly `0x005D5C2D..0x005D5C57`; retail `ra2md.mix` payload text | Yes |
| Offline mode control | control `0x6EB` stores mode object pointers as item data | `0x005D6130`; `0x005D625B..0x005D62A0` from prior report, rechecked by decompile | Yes |
| Choose Map mode/map commit | selected mode pointer/id and selected map index are committed together | `0x005E7160`; `0x005E734F..0x005E7382` | Yes |
| Map filtering | selected `mode+0x30` filter string controls map list; empty map `GameModes` means `standard` only | `0x005D6419`; `0x0069AE10` | Yes |
| Team/alliance setup | `MustAlly` and `AlliesAllowed` callbacks control Team `None`, `-2` acceptance, and ally helper defaults | `0x005D5DC0..0x005D5E08`; `0x004E4170` | Yes/Conditional |
| Rules/dialog defaults | broader `[MultiplayerDialogSettings]` fields are read by RulesClass, not the common MPModes object | `0x00671EA0` | Yes, separate surface |

No simulation tick-cycle path is claimed. This report covers shell/setup/session model data before and during launch handoff.

## 6. Current Rust Implementation Status

Current Rust partially models the native shape:

- `src/skirmish_modes.rs:21` defines `SkirmishGameMode` with id, UI key, tooltip, override file, map filter, random-map flag, `allies_allowed`, and `must_ally`.
- `src/skirmish_modes.rs:63` uses `apply_known_stock_dialog_defaults`, a filename switch that hardcodes the stock common values instead of loading each `MP*.ini` override payload through assets/MIX.
- `src/skirmish_scenarios.rs:197` matches native map filtering for row fields: random sentinel is mode-gated, empty map `GameModes` matches only `standard`, otherwise the selected filter string must match.
- `src/ui/skirmish_shell/state.rs:168..312` tracks selected mode in the Choose Map modal and refreshes map rows from mode filters.
- `src/ui/skirmish_shell/state.rs:1303` still exposes static team values `[-2,0,1,2,3]` regardless of selected mode.
- `src/skirmish_launch.rs:14` still has only `SkirmishLaunchMode::Battle`, and `src/ui/skirmish_shell/state.rs:1995` still packs launch sessions as Battle.
- `src/assets/asset_manager.rs:270..299` already has archive-backed `get`, `get_ref`, `get_with_source`, and `get_with_source_ref` lookup surfaces that can support a MIX-backed override loader.

Rust delta: stock UI values currently match the common fields for the five scoped stock modes, but the implementation is not data-driven. It will fail modded/custom MPModes and can drift when more `MP*.ini` fields are needed, because the native path opens the override filename and reads available keys with constructor defaults.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Required working notes | verified | Section 0 | none |
| Category registration | verified | `0x005D7CE0`; assembly `0x005D7D3C..0x005D7E36` | none |
| `MPModesMD.ini` row load | verified | `0x005D7590`; `MPModesMD.ini` | none |
| Common constructor defaults | verified | `0x005D5BEA..0x005D5BF6` | none |
| MIX/file-backed override load path | verified | `0x005D5C2D..0x005D5C57`; retail `ra2md.mix` text | exact hashed MIX entry attribution |
| Common constructor key set | verified | `0x005D5CA7`, `0x005D5CC4`, `0x005D5CDF`, `0x005D5CF7`; string xrefs | none |
| `MustAlly && !AlliesAllowed` clamp | verified | `0x005D5D05..0x005D5D11` | none |
| Battle/Team/FFA/Unholy/Coop common payload values | verified | retail text `110946..111641` plus binary readers | exact archive entry-name mapping only |
| Map filter/random row fields | verified | `0x005D7590`; `0x0069AE10`; `MPModesMD.ini` | none |
| Team/alliance callback effects | verified | `0x005D5DC0..0x005D5E08`; `0x004E4170` | full row rebuild behavior owned by sibling target |
| Broader dialog/rules settings | verified as separate surface | `0x00671EA0`; rules INI lines | full rules-override application is out-of-scope |
| Current Rust surface | verified | codegraph plus `src/skirmish_modes.rs`, `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/assets/asset_manager.rs` | no Rust edits in this slot |
| Online/WOL lobby behavior | deferred | user non-goal | separate online investigation |
| Full Cooperative object internals | deferred | user non-goal | separate Cooperative mode report |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which MPModes entry point constructs offline mode rows? -> `0x005D7590` opens `MPModesMD.ini`, enumerates registered sections, and creates sorted mode objects.` (evidence: `0x005D7590`; `0x005D759E`)
- `[RESOLVED] OQ-02 - Which categories are registered by the binary? -> Battle, ManBattle, Siege, Unholy, FreeForAll, Cooperative.` (evidence: `0x005D7D3C..0x005D7E36`)
- `[RESOLVED] OQ-03 - Which scoped stock rows exist? -> Battle id 1, Team Game id 9, Free For All id 2, Unholy id 4, Cooperative id 3.` (evidence: `MPModesMD.ini`; reader `0x005D7590`)
- `[RESOLVED] OQ-04 - Is stock Siege selectable offline? -> No; the binary registers the category but stock `MPModesMD.ini` has no `[Siege]` row.` (evidence: `0x005D7DA0`; `MPModesMD.ini`)
- `[RESOLVED] OQ-05 - What are constructor defaults? -> tournament flags true, `AlliesAllowed=true`, `MustAlly=false`.` (evidence: `0x005D5BEA..0x005D5BF6`)
- `[RESOLVED] OQ-06 - Does the constructor load the row override filename? -> Yes, it builds/opens the file from `mode+0x2C` before reading `[MultiplayerDialogSettings]`.` (evidence: `0x005D5B60`; assembly `0x005D5C2D..0x005D5C57`)
- `[RESOLVED] OQ-07 - Which common object keys are read? -> `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`, and `MustAlly` only.` (evidence: `0x005D5CA7..0x005D5D00`; string xrefs)
- `[RESOLVED] OQ-08 - What happens to contradictory `MustAlly=yes` and `AlliesAllowed=no`? -> `MustAlly` is cleared false.` (evidence: `0x005D5D05..0x005D5D11`)
- `[RESOLVED] OQ-09 - What stock Battle payload fields affect the common object? -> `AlliesAllowed=yes`; all other common fields default.` (evidence: retail text `110946..110954`; reader `0x005D5CDF`)
- `[RESOLVED] OQ-10 - What stock Team Game payload fields affect the common object? -> `AlliesAllowed=yes` and `MustAlly=yes`; `AllyChangeAllowed=no` is not a common object field.` (evidence: retail text `111631..111641`; readers `0x005D5CDF`, `0x005D5CF7`; xref `0x00672168`)
- `[RESOLVED] OQ-11 - What stock FFA payload fields affect the common object? -> `AlliesAllowed=no` and `WonlineClanTournamentAllowed=no`.` (evidence: retail text `111050..111061`; readers `0x005D5CC4`, `0x005D5CDF`)
- `[RESOLVED] OQ-12 - What stock Unholy payload fields affect the common object? -> `AlliesAllowed=yes`; all other common fields default.` (evidence: retail text `111568..111581`; reader `0x005D5CDF`)
- `[RESOLVED] OQ-13 - What stock Cooperative payload fields affect the common object? -> `AlliesAllowed=no` and `WonlineTournamentAllowed=no`.` (evidence: retail text `111622..111627`; readers `0x005D5CA7`, `0x005D5CDF`)
- `[RESOLVED] OQ-14 - Are map filter and random-map eligibility override payload fields? -> No; they are MPModes row fields.` (evidence: `MPModesMD.ini`; `0x005D7590`; `0x0069AE10`)
- `[RESOLVED] OQ-15 - Are money/unit/tech/game-speed common mode-object fields? -> No; their reader is `RulesClass__ReadMultiplayerDialogSettings`, not `0x005D5B60`.` (evidence: `0x00671EA0`)
- `[RESOLVED] OQ-16 - Is `AllyChangeAllowed` a common selected-mode object field? -> No; Team Game contains the key, but its xref is `0x00672168` in RulesClass, not `0x005D5B60`.` (evidence: string xrefs)
- `[RESOLVED] OQ-17 - What Rust currently hardcodes? -> `apply_known_stock_dialog_defaults` maps override filenames to stock `allies_allowed`/`must_ally`.` (evidence: `src/skirmish_modes.rs:63..80`)
- `[RESOLVED] OQ-18 - Is Rust launch/session still Battle-only? -> Yes, `SkirmishLaunchMode` only has `Battle` and launch packs Battle.` (evidence: `src/skirmish_launch.rs:14`; `src/ui/skirmish_shell/state.rs:1995`)
- `[DEFERRED] OQ-19 - Exact hashed MIX entry-to-filename attribution for each contiguous payload.` (category: bounded-cost-too-high; reason: common values are verified by visible retail payload text plus binary readers, but directory/hash attribution needs a dedicated MIX-index extraction pass; next-step-if-pursued: use asset manager or an external read-only MIX lister to map entry ids to filenames)
- `[DEFERRED] OQ-20 - Full application of non-common payload sections during scenario/rules mutation.` (category: out-of-scope; reason: target is offline setup/session MPModes payload fields, not full rules override semantics; next-step-if-pursued: trace selected override filename into rules load/start scenario flow)
- `[DEFERRED] OQ-21 - Full Cooperative `0x344` object internals.` (category: out-of-scope; reason: common constructor fields and setup/session effects are resolved; next-step-if-pursued: separate Cooperative object investigation)

Zero-add pass result: re-read `0x005D5B60`, xrefs for `MustAlly`/`AlliesAllowed`/tournament keys, `0x00671EA0`, `0x005D6130`, and `0x005E7160` after drafting; no new scoped common-object fields or offline setup consumers were found.

Adversarial corner cases answered:

- Missing override file: constructor defaults remain true/true/true/false because reads only occur after successful file/section load. Evidence: defaults at `0x005D5BEA..0x005D5BF6` before load/read branch.
- Missing key inside present override: `CCINIClass__ReadBool` is called with the current field as default, so the constructor default or prior field value is preserved. Evidence: each read pushes/uses the existing field byte immediately before key pointer, `0x005D5C9D..0x005D5CF7`.
- `MustAlly=yes` with `AlliesAllowed=no`: native clears `MustAlly`. Evidence: `0x005D5D05..0x005D5D11`.
- Stock Team Game and FFA both use `standard`-family category? No; Team Game row's map filter is `teamgame`, while FFA is `standard`; filters are row fields, not payload booleans. Evidence: `MPModesMD.ini`, `0x0069AE10`.
- `AllyChangeAllowed=no` in Team Game should change `SkirmishGameMode`? No for this common object; its reader is `0x00671EA0`, separate from `0x005D5B60`. Evidence: xrefs to `0x0083CFB4`.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native loads each MPModes row override filename through the file/archive path and reads common fields with constructor defaults. | `0x005D5B60`; assembly `0x005D5C2D..0x005D5D11`; retail `ra2md.mix` payload text | stock values are hardcoded by filename | `src/skirmish_modes.rs`; asset-backed INI loading via `src/assets/asset_manager.rs:270..299` | Replace `apply_known_stock_dialog_defaults` with an override merge that looks up `mode.override_file` through the same archive search used for other retail assets, parses `[MultiplayerDialogSettings]`, applies defaults, and clamps `MustAlly && !AlliesAllowed`. | With stock assets, ids 1/4 have `allies_allowed=true,must_ally=false`; id 9 has `true,true`; ids 2/3 have `false,false`; removing/omitting an override key preserves defaults. | Do not add more filename cases; native is data-driven and modded `MPModesMD.ini` rows must work. |
| Only four keys are common selected-mode object fields: `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`, `MustAlly`. | `0x005D5CA7`, `0x005D5CC4`, `0x005D5CDF`, `0x005D5CF7`; string xrefs | Rust stores only `allies_allowed` and `must_ally`; tournament flags are not represented | `SkirmishGameMode` or adjacent setup-mode model | Parse the four common fields if the model needs parity with listing/tournament gates; for offline setup behavior, `AlliesAllowed` and `MustAlly` are the visible/session-critical fields. | Unit test proves Team Game suppresses Team `None`; FFA/Coop default ally helper behavior differs from Battle; tournament flags can be ignored only in an explicitly offline-only surface. | Do not store every `[MultiplayerDialogSettings]` key on `SkirmishGameMode`. |
| `AllyChangeAllowed`, money/unit/tech/game-speed, checkboxes, and `MCVRedeploys` are RulesClass/dialog settings, not common MPModes object fields. | `0x00671EA0`; `AllyChangeAllowed` xref `0x00672168`; absence from `0x005D5B60` | launch options are separate, but future mode parser could accidentally conflate the section | `src/skirmish_modes.rs`; `src/skirmish_launch.rs`; future rules override loader | Keep the common mode-object override parser narrow; trace full rules override application separately before changing launch option defaults from `MP*.ini`. | Selecting Team Game changes team constraints, not credits/unit count by this common-object parser. | Do not parse `AllyChangeAllowed=no` into `SkirmishGameMode::allies_allowed`; it is a different key and reader. |
| Map filter and random-map eligibility are row fields, not override payload fields. | `MPModesMD.ini`; row reader `0x005D7590`; filter consumer `0x0069AE10` | mostly implemented | `src/skirmish_scenarios.rs`; Choose Map modal state | Keep selected-mode filtering tied to row `map_filter` and `random_maps_allowed`. | Battle and FFA expose random map sentinel; Team/Unholy/Coop do not; Team filters to `teamgame`, Coop to `cooperative`. | Do not infer filter/random capability from override filename or UI string. |
| Team `None` and `-2` team acceptance are selected-mode dependent through `MustAlly`; `AlliesAllowed` also affects ally helper default. | `0x005D5DC0..0x005D5E08`; `0x004E4170`; Team payload lines `111638..111641` | static team combo rows and Battle-only launch | `src/ui/skirmish_shell/state.rs:1303`; `src/skirmish_launch.rs`; launch validation | Build team choices and launch validation from selected mode fields: Team Game omits/rejects `None`, Battle permits it, FFA/Coop must not behave like Battle for ally defaults. | Selecting Team Game removes Team `None` and prevents packing `LaunchTeam::None`; switching back to Battle restores `None`. | Do not leave `[-2,0,1,2,3]` static for all modes. |
| Selected mode id/object is committed and later used for session behavior; Rust still packs Battle. | `0x005E7160`; `src/skirmish_launch.rs:14`; `src/ui/skirmish_shell/state.rs:1995` | mismatch | `SkirmishLaunchSession`; shell launch construction | Carry selected mode id/data into launch session so payload-derived setup behavior reaches session creation. | Selecting FFA/Team/Coop and pressing Start produces a launch session with that selected mode id and matching team constraints. | Do not treat correct map filtering alone as selected-mode launch parity. |

## Negative Facts / Do Not Do

- Do not expose Siege in stock offline Skirmish from binary category registration alone. Active in YR: No for stock offline selection; evidence `0x005D7DA0` registers the category but `MPModesMD.ini` has no stock `[Siege]` section.
- Do not keep stock-only hardcoded filename branches as the long-term parser. Active in YR: Yes, native opens the row override filename through the file/archive path at `0x005D5C2D..0x005D5C57`.
- Do not treat base `rulesmd.ini:[MultiplayerDialogSettings] AlliesAllowed=no` as every selected mode object's value. Active in YR: Yes, `0x005D5B60` defaults `AlliesAllowed=true` and then reads each mode override.
- Do not let `MustAlly=yes` survive when `AlliesAllowed=no`. Active in YR: Yes, constructor clears it at `0x005D5D05..0x005D5D11`.
- Do not model `AllyChangeAllowed` as `SkirmishGameMode::allies_allowed` or as a common mode-object field. Active in YR: Yes as RulesClass/dialog setting only; evidence xref `0x00672168`.
- Do not move money, unit count, tech level, game speed, checkbox defaults, `FogOfWar`, or `MCVRedeploys` into the common MPModes object without a separate rules-override trace. Active in YR: Yes as `0x00671EA0` rules/dialog settings, not common object fields.
- Do not infer map filters or random-map capability from override payloads. Active in YR: Yes, both come from `MPModesMD.ini` row tokens.
- Do not claim launch/session parity while `SkirmishLaunchMode` is Battle-only and team rows ignore selected-mode `MustAlly`.

## Remaining Uncertainty

- Exact hashed MIX directory entry-to-filename attribution for the visible override payloads was not extracted. This does not change the common-field findings because native reads by row filename, retail payload comments/values are visible in `ra2md.mix`, and the reader addresses are verified.
- Full application of non-common `MP*.ini` payload sections to rules/scenario state remains outside this target. This report only separates those fields from the common MPModes object and names the separate `0x00671EA0` reader.
- Cooperative-specific large object fields beyond the common constructor are outside this target.

## Sources

- Ghidra read-only decompile/recheck: `0x005D5B60`, `0x005D7590`, `0x005D7CE0`, `0x005D6130`, `0x005E7160`, `0x00671EA0`, `0x0069AE10`, `0x004E4170`, `0x005D6310`.
- Ghidra assembly/xrefs: defaults and reads `0x005D5BEA..0x005D5D11`; team helpers `0x005D5DC0..0x005D5E08`; file load path `0x005D5C2D..0x005D5C57`; category registration `0x005D7D3C..0x005D7E36`; selected-mode commit `0x005E71E5..0x005E7382`; string xrefs `MustAlly` `0x008308A0`, `AlliesAllowed` `0x008308AC`, `WonlineClanTournamentAllowed` `0x008308BC`, `WonlineTournamentAllowed` `0x008308DC`, `AllyChangeAllowed` `0x0083CFB4`.
- Retail/INI data checked: `ini/mpmodesmd.ini`; `ini/rulesmd.ini`; `ini/rules.ini`; text scan of `<ra2-install>/ra2md.mix` lines around `110946..111641`.
- Prior reports used for gap targeting: `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_OVERRIDE_PAYLOAD_VALUES_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_RETAIL_VALUES_AUDIT_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_SESSION_PACKING_BROAD_RECHECK_GHIDRA_REPORT.md`.
- Current Rust scan: `src/skirmish_modes.rs`, `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/assets/asset_manager.rs`.
