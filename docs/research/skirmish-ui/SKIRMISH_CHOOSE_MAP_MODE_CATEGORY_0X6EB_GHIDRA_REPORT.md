# Skirmish Choose Map Mode Category 0x6EB - Ghidra Research Report

**Address(es):** `0x005D6130`, `0x005D63E0`, `0x0069AE10`, `0x005D5E10`, `0x005D5F30`, `0x005E68A0`, `0x005E7160`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Choose Map modal control `0x6EB` construction, initial selection, item-data object relationship, and exact scenario/map `GameModes` filtering predicate for standard offline YR Skirmish.
**Non-Scope:** unrelated shell controls, full modal art/layout, full scenario source census, PreviewPack decode, non-offline WOL behavior, and Cooperative campaign payload internals.
**Confidence:** High for `0x6EB` population/selection/item-data and map predicate; Medium for runtime-only tournament/official gates because their branches are live but not default ordinary offline Skirmish.
**Active in YR:** Yes. `0x006ACEE0` calls `0x005E68A0` for the Skirmish Choose Map command; `0x005E68A0` creates the modal with callback `LAB_005E6920`; the callback calls `0x005D6130` for control `0x6EB` and the filtered-map loop calls `0x005D63E0`.

## 1. Overview

The Choose Map dialog's `0x6EB` control is the selected multiplayer mode/category list. It is populated from the global MPModes object vector built from `MPModesMD.ini`; each row stores the mode object pointer as item data, and the row matching the current selected mode id is selected during population.

The map-list filter does not match UI text. It passes the selected mode object into `0x005D63E0`; non-random maps match when the mode object's filter string at `mode+0x30` matches one of the scenario record's `GameModes` entries, with empty `GameModes` matching only `standard`.

## 2. Class Layout / Key Offsets

| Item | Type / value | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| control `0x6EB` | dialog child | MPModes mode/category combo/list control in Choose Map modal | `0x005E6ED1` pushes `0x6EB`; `0x005E6EDE` calls `0x005D6130`; `0x005E71F4..0x005E7219` reads selection/item data | Yes |
| control `0x553` | dialog child | filtered scenario/map list; item data is scenario-record pointer | `0x005E7163..0x005E71D2`, `0x005E70D0` | Yes |
| mode `+0x20` | string | display string used when adding a `0x6EB` row | `0x005D626E..0x005D627E` | Yes |
| mode `+0x28` / `mode[10]` | int | numeric mode id; selected id copied to `DAT_00A8B250` | `0x005D6282..0x005D6296`, `0x005E736D..0x005E7376` | Yes |
| mode `+0x30` / `param_1 + 0xC` | string/list block pointer base | selected mode map-filter string compared against scenario `GameModes` | `0x005D6419..0x005D641F`; prior MPModes loader report | Yes |
| mode `+0x34` | bool | random maps allowed; consulted through vtable `+0x3C` for `RandMap.Sed` | roster row field in `MPModesMD.ini`; predicate dispatch at `0x005D63E8..0x005D63FC` | Yes, conditional by selected mode |
| mode vtable `+0x40` | callback | offline Skirmish displayability gate before adding a `0x6EB` row | `0x005D625B..0x005D626C` | Yes for `g_GameMode == 5` |
| mode vtable `+0xBC` | callback | extra visibility/capability gate before `+0x40` in `0x005D6130`; value also used during accept validation | `0x005D6239..0x005D6259`, `0x005E722A..0x005E723C` | Conditional by mode/game context |
| record `+0x58` | string | map filename; compared to `RandMap.Sed` | `0x0069ADF0`; memory `0x0082BC30` = `RandMap.Sed` | Yes |
| record `+0x17C` | bool | official flag; can reject unofficial maps in one runtime mode | `0x005D63FF..0x005D6416` | Conditional |
| record `+0x1A8/+0x1B4` | list/count | map `GameModes` string vector and count | `0x0069AE10` | Yes |
| `DAT_00A8B250` | int | selected mode id used to repopulate/select `0x6EB` | `0x005E6EA6`, `0x005D5F30`, `0x005E7376` | Yes |
| `DAT_00A8B23C` | mode pointer | committed selected MPModes object | `0x005E71EA`, `0x005E7367` | Yes |
| `DAT_00A8B254` | int | selected scenario index in `DAT_00A8B8CC` | `0x005E7370`, `0x005E7382` | Yes |

## 3. Core Logic

### 3.1 Initial selected mode object

During modal initialization, the dialog attempts to resolve the current selected mode id:

1. The callback reads `DAT_00A8B250` and calls `0x005D5F30`.
2. `0x005D5F30` iterates the global MPModes vector `DAT_00ABFDA4[0..DAT_00ABFDB0)` and returns the first mode whose `mode+0x28` equals the requested id.
3. If lookup fails, the initialization path falls back to `0x005D5E10`.
4. `0x005D5E10` returns the first MPModes vector entry, or `0` if the vector is empty.

Active in YR: Yes. Evidence: `0x005E6EA6` modal-initialization path; `0x005D5F30` and `0x005D5E10` decompile; `0x005D7590`/`MPModesMD.ini` build the vector.

Tiny details:

- `0x005D5F30` compares mode ids, not labels, filenames, tooltips, or map filter strings.
- If no mode id matches, the fallback is the first vector item, not a hardcoded Battle object. In stock `MPModesMD.ini`, sorted insertion by id makes Battle id `1` first.
- Empty vector returns null from `0x005D5E10`; this is an edge case, not stock YR.

### 3.2 `0x6EB` population

`0x005D6130(HWND control, int selected_mode_id)` clears `0x6EB` with message `0x184`, then iterates the global MPModes vector.

For each mode object:

1. Skip WOL/tournament-disallowed entries only in `g_GameMode == 4` branches involving `mode+0x3D`, `mode+0x3E`, `FUN_0077D940`, and `FUN_0077D970`.
2. If `DAT_008316A4 == 0` and `*(DAT_00A8DA90 + 0x6B) == -1`, call mode vtable `+0xBC`; skip row if it returns false.
3. If `g_GameMode == 5`, call mode vtable `+0x40`; skip row if it returns false.
4. Convert `mode+0x20` display string with `0x007B7140`.
5. Add a row with message `0x4CD`.
6. If `mode+0x28 == selected_mode_id`, set current selection with `0x186`.
7. Store the mode object pointer as row item data with `0x19A`.

Active in YR: Yes for ordinary offline Skirmish. The `g_GameMode == 5` branch is the offline Skirmish display gate; the WOL branches are conditional and outside standard offline selection.

Evidence:

- Decompile `0x005D6130`.
- Assembly context `0x005D625B..0x005D62A0`: vtable `+0x40`, display pointer `mode+0x20`, add row `0x4CD`, select `0x186`, item data `0x19A`.
- Xref: `0x005E6EDE` calls `0x005D6130` after pushing `0x6EB`.

Tiny details:

- The row item data is the object pointer (`lParam`/`ESI`), not the mode id. The id is copied separately only when committed.
- Selection is made during population immediately after row insertion if the ids match; no post-population text scan is used.
- The function sets item data after optional selection. A selected row therefore still receives its mode pointer before the next control interaction.

### 3.3 Filtered scenario list predicate

The modal map list builder loops global scenario records in forward order and calls `0x005D63E0(selected_mode, record)`. This report only claims the predicate, not the full scenario source build.

`0x005D63E0`:

1. Calls `0x0069ADF0(record)`.
2. If `record+0x58` equals `RandMap.Sed`, returns selected mode vtable `+0x3C`.
3. Otherwise, if `record+0x17C == 0` and `FUN_0077D940()` returns true, returns false.
4. Otherwise calls `0x0069AE10(mode + 0x30, record)`.

`0x0069AE10`:

1. If the scenario record has zero `GameModes` entries (`record+0x1B4 == 0`), compare the selected mode filter to literal string `standard`.
2. If the record has entries, iterate `i = 0 .. count-1`, compare the selected mode filter against each `record+0x1A8[i]`, and accept on first match.
3. If no entries match, reject.

Active in YR: Yes. Evidence: decompile `0x005D63E0`, `0x0069AE10`; assembly context `0x005E6F17..0x005E6F3E` calls the predicate while iterating `DAT_00A8B8CC`; memory `0x0083F668` is `standard`; memory `0x0082BC30` is `RandMap.Sed`.

Tiny details:

- Filtering uses the selected mode filter string stored in the mode object, not the category section name (`Battle`, `ManBattle`, etc.) and not the UI label (`GUI:Battle`).
- Empty map `GameModes` is not "match everything"; it is exactly "match selected mode filter `standard`".
- `RandMap.Sed` bypasses map `GameModes` and asks the selected mode's random-map callback. Stock rows with `randomMapsAllowed=false` should not accept the random sentinel.
- The official-map rejection branch is live but conditional; ordinary offline Skirmish should not model it as a blanket rejection of custom maps without verifying the runtime `FUN_0077D940()` condition.

### 3.4 Accept/commit relationship

`0x005E7160` reads both controls on accept:

1. Read selected map from control `0x553` with `0x188`; if `-1`, fail.
2. Read the map row item data with `0x199`; scan `DAT_00A8B8CC` to find the matching scenario-record pointer; if absent, fail.
3. Read selected mode from control `0x6EB` with `0x188`; if not `-1`, read row item data with `0x199` into a mode pointer.
4. Temporarily assign `DAT_00A8B23C = selected_mode`, call selected mode callbacks, restore the previous selected mode while validation dialogs may run.
5. On accepted commit, if the selected mode changed, call old mode `+0x9C`, set `DAT_00A8B23C = selected_mode`, copy `selected_mode+0x28` to `DAT_00A8B250`, set `DAT_00A8B254 = selected_map_index`, then call new mode `+0x20`.
6. Always set `DAT_00A8B254 = selected_map_index` before post-commit calls and UI text updates.

Active in YR: Yes. Evidence: decompile `0x005E7160`; assembly context `0x005E71E5..0x005E7219` reads `0x6EB`; `0x005E734F..0x005E7382` commits pointer/id/index.

Tiny details:

- `0x6EB` selected item data may be null if no row is selected, but the normal path immediately dispatches through the pointer; stock population must therefore leave a valid current selection.
- The map list item data is pointer identity against the scenario record array, not display text.
- The selected mode object is temporarily written before validation and restored before rejection prompt handling.

## 4. INI Keys / Stock Data

`MPModesMD.ini` is the source for stock offline mode rows. `0x005D7590` opens string `MPModesMD.ini` at `0x00830A18`, parses registered categories, and inserts mode objects sorted by `mode+0x28`.

| Category | id | UI name | override file | map filter | random maps | Active in stock offline YR |
|---|---:|---|---|---|---:|---|
| Battle | 1 | `GUI:Battle` | `MPBattleMD.ini` | `standard` | true | Yes |
| Battle | 9 | `GUI:TeamGame` | `MPTeamMD.ini` | `teamgame` | false | Yes |
| ManBattle | 5 | `GUI:Megawealth` | `MPMWMD.ini` | `megawealth` | false | Yes |
| ManBattle | 6 | `GUI:Duel` | `MPDuelMD.ini` | `duel` | false | Yes |
| ManBattle | 7 | `GUI:MeatGrind` | `MPMeatMD.ini` | `meatgrind` | false | Yes |
| ManBattle | 8 | `GUI:NavalWar` | `MPNavalMD.ini` | `navalwar` | false | Yes |
| FreeForAll | 2 | `GUI:FreeForAll` | `MPFreeForAllMD.ini` | `standard` | true | Yes |
| Unholy | 4 | `GUI:UnholyAlliance` | `MPUnholyMD.ini` | `standard` | false | Yes |
| Cooperative | 3 | `GUI:Cooperative` | `MPCoopMD.ini` | `cooperative` | false | Yes |
| Siege | none | none | none | none | none | No stock selectable row |

Active in YR: Yes for the listed stock rows. Evidence: `ini/mpmodesmd.ini`; `0x005D7590` binary reader; `0x005D7CE0` category registration per prior MPModes report.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Modal creation | Choose Map opens dialog through `0x005E68A0`, callback `LAB_005E6920` | `0x005E68A0`; xref from `0x006AD947 in FUN_006ACEE0` | Yes |
| Initial mode resolution | selected id `DAT_00A8B250` resolves through `0x005D5F30`; fallback first mode via `0x005D5E10` | decompile `0x005D5F30`, `0x005D5E10`; callback context `0x005E6EA6` | Yes |
| `0x6EB` population | add display string, set current row by id, store mode pointer as item data | `0x005D6130`; call at `0x005E6EDE` | Yes |
| map-list filtering | scenario records filtered by selected mode object against record `GameModes` | `0x005D63E0`, `0x0069AE10`; calls at `0x005E6AA0`, `0x005E6F27` | Yes |
| accept commit | selected map pointer and selected mode pointer committed to globals | `0x005E7160` | Yes |

This slice is synchronous shell/modal code. It has no simulation tick-cycle integration.

## 6. Current Rust Implementation Status

| Area | Rust status | Evidence |
|---|---|---|
| Choose Map button | action exists, but `apply_action` cycles `selected_map_idx` in place instead of opening a modal | `src/ui/skirmish_shell/state.rs` |
| selected map identity | stores `selected_map_idx` only | `src/ui/skirmish_shell/state.rs` |
| MPModes model | no parsed mode roster / selected-mode object; launch mode is hardcoded Battle | `src/skirmish_launch.rs`, `src/ui/skirmish_shell/state.rs` |
| map list | scans loose files and sorts by display name; no PKT source order or mode filter | `src/app_list_maps.rs` |
| map record data | `MapMenuEntry` lacks mode `GameModes`, source-order, official, random-sentinel, and mode-filter fields | `src/app_init.rs`, `src/app_list_maps.rs` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x6EB` population function `0x005D6130` | verified | decompile; assembly context `0x005D625B..0x005D62A0`; xref `0x005E6EDE` | none for offline visible rows |
| selected id lookup `0x005D5F30` | verified | decompile | none |
| first-mode fallback `0x005D5E10` | verified | decompile | empty-vector behavior is edge-only |
| map predicate `0x005D63E0` | verified | decompile; xrefs from modal list rebuild | none |
| `GameModes` comparison `0x0069AE10` | verified | decompile; memory `0x0083F668 = standard` | none |
| random sentinel check `0x0069ADF0` | verified | decompile; memory `0x0082BC30 = RandMap.Sed` | exact vtable `+0x3C` per concrete mode covered by MPModes report |
| accept commit `0x005E7160` selected mode item data | verified | decompile; assembly context `0x005E71E5..0x005E7382` | full accept/cancel parent restoration covered by sibling report |
| WOL/tournament gates in `0x005D6130` | touched-not-exhausted | decompile branches against `g_GameMode == 4` | out of standard offline Skirmish scope |
| official-map runtime gate | touched-not-exhausted | `0x005D63FF..0x005D6416` | exact `FUN_0077D940()` runtime mode defaults outside this slice |
| modal visual layout of `0x6EB` | not-touched | none | separate visual/control-layout swarm slot |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is control 0x6EB populated in the Choose Map modal path? -> Yes, modal callback calls 0x005D6130 for child 0x6EB.` (evidence: `0x005E6ED1..0x005E6EDE`, `0x005E68A0`)
- `[RESOLVED] OQ-02 - What object source populates 0x6EB? -> The global MPModes vector built from MPModesMD.ini rows.` (evidence: `0x005D6130`, `0x005D7590`, `ini/mpmodesmd.ini`)
- `[RESOLVED] OQ-03 - What selects the initial 0x6EB row? -> `mode+0x28 == DAT_00A8B250`; if lookup fails, first mode is used before population.` (evidence: `0x005D5F30`, `0x005D5E10`, `0x005D6282..0x005D6296`)
- `[RESOLVED] OQ-04 - What is 0x6EB row item data? -> The MPModes object pointer, not the id or label.` (evidence: `0x005D6298..0x005D62A0`, `0x005E71F4..0x005E7219`)
- `[RESOLVED] OQ-05 - Does filtering use category section names like Battle/ManBattle? -> No; it uses the selected mode object's filter string at `mode+0x30`.` (evidence: `0x005D6419..0x005D641F`, `0x0069AE10`)
- `[RESOLVED] OQ-06 - Does filtering use display text? -> No; display text is only `mode+0x20` for row insertion.` (evidence: `0x005D626E..0x005D627E`, `0x005D6419`)
- `[RESOLVED] OQ-07 - What happens to maps with no GameModes entries? -> They match only selected filter `standard`.` (evidence: `0x0069AE15..0x0069AE30`, memory `0x0083F668`)
- `[RESOLVED] OQ-08 - What happens to RandMap.Sed? -> It bypasses normal GameModes comparison and returns selected mode vtable `+0x3C`.` (evidence: `0x005D63E8..0x005D63FC`, `0x0069ADF0`, memory `0x0082BC30`)
- `[RESOLVED] OQ-09 - Is the 0x6EB selected object committed on accept? -> Yes; `0x005E7160` reads item data, commits pointer/id, and then runs mode callbacks.` (evidence: `0x005E71E5..0x005E7382`)
- `[RESOLVED] OQ-10 - Is standard offline Skirmish the active path? -> Yes for `g_GameMode == 5` display gate and Choose Map command path; WOL branches are conditional out-of-scope.` (evidence: `0x005D625B..0x005D626C`, xref `0x006AD947`)
- `[RESOLVED] OQ-11 - Does current Rust match the modal/filter model? -> No; it cycles selected_map_idx and lacks MPModes/filter fields.` (evidence: `src/ui/skirmish_shell/state.rs`, `src/app_list_maps.rs`)
- `[DEFERRED] OQ-12 - Exact concrete implementation of every mode vtable +0x3C random-map callback.` (category: out-of-scope; reason: existing MPModes report covers row randomMapsAllowed enough for chooser design; next-step-if-pursued: dedicated callback-vtable audit)
- `[DEFERRED] OQ-13 - Exact default truth of `FUN_0077D940()` for official-map filtering in every launch/network mode.` (category: out-of-scope; reason: target is standard offline chooser; next-step-if-pursued: runtime mode/global investigation for tournament/custom-map filtering)
- `[DEFERRED] OQ-14 - Visual geometry and owner-draw behavior for control 0x6EB.` (category: out-of-scope; reason: this slot is data/filter contract only; next-step-if-pursued: modal visual/control-layout slot)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x6EB` rows are MPModes objects from `MPModesMD.ini`; row item data is the mode object pointer and selection is by mode id `+0x28`. | `0x005D6130`, `0x005D5F30`, `0x005D7590`, `ini/mpmodesmd.ini` | missing | needed MPModes loader/model; `src/ui/skirmish_shell/state.rs`; `src/skirmish_launch.rs` | Model selected mode as data-driven id/display/filter/random/flags, not a display string. | `choose_map_mode_combo_populates_stock_mpmodes_and_selects_by_id`: stock rows include Battle id 1, Team Game id 9, no Siege; selecting id 9 stores Team Game mode data. | Do not hardcode Battle-only launch or infer category from UI text. |
| Map filtering compares selected `mode+0x30` filter string against record `GameModes`; empty record `GameModes` matches only `standard`. | `0x005D63E0`, `0x0069AE10`, memory `0x0083F668` | missing | `src/app_list_maps.rs`, `MapMenuEntry`, chooser-modal state | Carry parsed map `GameModes` and filter the chooser list by selected mode filter. | `choose_map_filters_by_selected_mpmode_game_modes`: Team Game shows only `teamgame` maps; Battle/FFA show `standard`; empty `GameModes` maps are hidden for `duel`. | Do not treat missing `GameModes` as match-all. |
| `RandMap.Sed` is accepted through selected mode random-map callback, separate from `GameModes`. | `0x005D63E8..0x005D63FC`, `0x0069ADF0`, `ini/mpmodesmd.ini` random flag column | missing | map source/list model and chooser filter | Represent random sentinel separately and include it only for modes with random maps allowed. | `choose_map_random_sentinel_respects_mode_random_allowed`: Battle and FreeForAll include random; Team Game/Duel/Naval/Unholy/Coop do not. | Do not add Random Map as a generic first row for every mode. |
| Accept commits the selected mode pointer/id and selected map pointer/index together. | `0x005E7160`, `0x005E734F..0x005E7382` | missing | chooser modal accept/cancel state; preview invalidation path | Commit selected map and mode as one accepted transaction; cancel must leave both restored. | `choose_map_accept_commits_mode_and_map_cancel_restores_both`: change mode/map in modal, cancel restores previous preview/filter; accept updates preview and launch mode. | Do not commit mode changes while the modal is still open unless the parent path is intentionally modeling native globals and restoration. |

## Negative Facts / Do Not Do

- Do not implement Choose Map as in-place cycling; retail opens a modal and uses `0x6EB` plus `0x553`.
- Do not filter maps by displayed mode name (`GUI:Battle`) or category section (`Battle`/`ManBattle`); filter by mode `mapFilter` string (`standard`, `teamgame`, etc.).
- Do not treat empty map `GameModes` as universal; native treats it as `standard` only.
- Do not expose stock Siege in offline Skirmish from binary category registration alone; there is no stock `[Siege]` row in `MPModesMD.ini`.
- Do not sort the modal list by display name when claiming native parity; this slice confirms predicate semantics, while list-order parity remains from the source-order report.
- Do not reject custom maps solely because `Official=no`; the official gate is conditional and not proven as a blanket standard offline restriction.

## Remaining Uncertainty

- The exact concrete `+0x3C` random-map callback bodies were not re-decompiled here; the stock `randomMapsAllowed` row field and prior MPModes report are sufficient for chooser design but a callback-only audit would strengthen this detail.
- The runtime meaning/default of `FUN_0077D940()` for the official-map gate remains outside this target. The branch is live, but ordinary offline behavior should not be inferred from the branch alone.
- Control `0x6EB` visual geometry and owner-draw details were not investigated in this data-contract slot.

## Stale Docs / Follow-up Docs

No stale-doc correction is required for the two reconciled reports. This report narrows and strengthens their wording:

> Choose Map control `0x6EB` stores the selected MPModes object pointer as row item data. Initial selection is by numeric mode id at object `+0x28`, and map filtering uses the selected object's map-filter string at `+0x30` against each scenario record's `GameModes` list. The category section name and visible UI label are not the filter key.

## Sources

- Ghidra read-only decompile: `0x005D6130`, `0x005D63E0`, `0x0069AE10`, `0x005D5E10`, `0x005D5F30`, `0x005E68A0`, `0x005E70D0`, `0x005E7160`, `0x0069ADF0`, `0x005D7590`.
- Ghidra read-only xrefs/context: `0x005D6130` xref from `0x005E6EDE`; `0x005D63E0` xrefs from `0x005E6AA0`, `0x005E6F27`, `0x006AEB62`; `0x005E68A0` xref from `0x006AD947`; `0x005E7160` xrefs from `0x005E6B2F`, `0x005E6B67`; assembly context `0x005D625B..0x005D62A0`, `0x005E71E5..0x005E7382`, `0x005D6419..0x0069AE30`.
- Ghidra memory/string evidence: `0x0082BC30 = RandMap.Sed`, `0x0083F668 = standard`, `0x00830A18 = MPModesMD.ini`.
- Docs reconciled: `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_RETURN_CONTRACT_GHIDRA_REPORT.md`.
- INI/data checked: `C:/Users/enok/Documents/ra2-rust-game/ini/mpmodesmd.ini`.
- Rust scan: `src/ui/skirmish_shell/state.rs`, `src/app_list_maps.rs`, `src/app_init.rs`, `src/skirmish_launch.rs`.
