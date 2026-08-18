# Skirmish Choose Map List Population Order - Ghidra Research Report

**Address(es):** `0x005E68A0`, `0x005E6920`, `0x005D6130`, `0x005D63E0`, `0x005E70D0`, `0x005E7160`, `0x00699980`, `0x006994F0`, `0x0069A3B0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Offline Skirmish Choose Map modal list population: source map records/files, valid map filtering, display order, selected item data/state, and initial selection.
**Non-Scope:** PreviewPack decode, preview object lifecycle, parent Skirmish preview invalidation, full modal return contract, random terrain generation internals.
**Confidence:** High for modal list population and selection state; Medium for loose-file enumeration order because Win32 `FindFirstFileA` ordering is filesystem-provided.
**Active in YR:** Yes. Evidence: `0x006ACEE0` live Skirmish `0x5AA` branch calls `0x005E68A0`; `0x005E68A0` creates dialog resource `0x6B` with callback `0x005E6920`; the callback initializes and commits list state without any TS-only gate.

## 1. Overview

The Choose Map modal does not independently scan files when it opens. It displays records already built in the global scenario list at `DAT_00A8B8CC` with count `DAT_00A8B8D8`; the modal filters that record array by the selected game-mode/category list item and fills listbox control `0x553` in original record order.

Initial selection is pointer-based, not display-text-based. The modal list item data is the scenario-record pointer, and selection helpers search listbox `0x553` by comparing `LB_GETITEMDATA` (`0x199`) against `DAT_00A8B8CC[index]`.

## 2. Key Offsets / Globals

| Item | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8B8CC` | global array of scenario-record pointers | `0x005E6A90`, `0x005E6F17`, `0x005E7160`, `0x00699980` | Yes - read by modal and parent |
| `DAT_00A8B8D8` | scenario-record count | `0x005E6A6C`, `0x005E6EF6`, `0x005E7160` | Yes |
| record `+0x58` | map file path/name; compared to `RandMap.Sed` and copied to selected path | `0x0069ADF0`, `0x005E7BF0` | Yes |
| record `+0x15C` | digest copied into selected digest global | `0x005E7BF0`, `0x0069AD80` | Yes |
| record `+0x17C` | `Official` bool from `[Basic] Official` | `0x006994F0`, `0x0069A980`, `0x005D63FF` | Yes; extra gate only when `FUN_0077D940()` is true |
| record `+0x180/+0x184` | min/max players from `[Basic] MinPlayers/MaxPlayers` | `0x006994F0`, `0x0069A980`, `0x005E7BF0` | Yes |
| record `+0x1B4` under map mode-list block | count of map `GameModes` entries | `0x0069AE15`, `0x0069AE33` | Yes |
| control `0x553` | map listbox in Choose Map dialog | `0x005E6A2F`, `0x005E70D0`, `0x005E7160` | Yes |
| control `0x6EB` | game-mode/category listbox; item data is mode pointer | `0x005D6130`, `0x005E6ED1`, `0x005E7160` | Yes |
| `DAT_00A8B254` | selected scenario index in `DAT_00A8B8CC` | `0x005E7160`, `0x005E7BF0`, `0x006ACEE0` | Yes |
| `DAT_00A8B250` | selected game-mode/category id | `0x005E6EA6`, `0x005D6130`, `0x005E7160` | Yes |

## 3. Source Collections / File Order

`0x00699980` builds the global scenario-record list before the modal consumes it. The modal itself only reads `DAT_00A8B8CC`.

Verified build order:

1. Clear/reuse the scenario list at caller object `+0x690`, then open `MISSIONSMD.PKT` (`0x006999D9` pushes string `0x0083F520`; memory at `0x0083F520` = `MISSIONSMD.PKT`).
2. Read section `MultiMaps` (`0x0083F514`) and iterate its entries by count/index (`0x00699A08..0x00699ABD`); each non-empty entry creates a `0x1BC` scenario record via `0x0069A3B0` and appends it.
3. Enumerate loose `*.PKT` files (`FindFirstFileA` at `0x00699AE1`, pattern `0x0083F50C` = `*.PKT`) and process their `MultiMaps` entries the same way.
4. Enumerate `*.YRO` files (`0x00699C5D`, pattern `0x0083F504` = `*.YRO`), open archive-side `MISSIONS.YRO` / embedded `PKT` data, and append produced scenario records.
5. Enumerate loose `*.YRM` files (`0x0083F490` = `*.YRM`) and create direct map records via `0x0069A980`.

Active in YR: Yes. `MISSIONSMD.PKT`, `*.YRO`, and `*.YRM` are Yuri's Revenge-specific source names in the live scenario-list builder; no TS-only branch gates the builder.

## 4. Record Validity / Filtering

The Choose Map list is rebuilt in the dialog callback, not by file scanning:

1. Custom message `0x497` enters initialization at `0x005E6EA6`.
2. It sets `DAT_00AC11C8 = 1`, finds the selected category by `DAT_00A8B250` using `0x005D5F30`, or falls back to the first category via `0x005D5E10`.
3. It populates category listbox `0x6EB` by calling `0x005D6130(hwnd, selected_category_id)`.
4. It allocates a temporary pointer vector (`0x005EF0B0`) sized to `DAT_00A8B8D8`.
5. It loops `EBX = 0 .. DAT_00A8B8D8-1`, loads `record = DAT_00A8B8CC[EBX]`, and calls `0x005D63E0(selected_mode, record)`.
6. If the predicate returns true, it appends that record pointer to the temporary vector with `0x005EEE40`.
7. It passes the temporary vector into the listbox/custom list backing object at `0x005E6F47..0x005E6F5B`.

`0x005D63E0` predicate:

- If `record+0x58` equals `RandMap.Sed`, return the selected mode object's vtable `+0x3C` result. Active in YR: Conditional - only for the random-map sentinel; evidence `0x005D63E8..0x005D63FC`, `0x0069ADF0`.
- Else, if `record+0x17C == 0` and `FUN_0077D940()` returns true, reject the record. Active in YR: Conditional - the branch is live, but depends on the `FUN_0077D940()` runtime mode; evidence `0x005D63FF..0x005D6416`.
- Else, compare the selected mode's string/list block at `mode+0x30` with the record's `GameModes` list by `0x0069AE10`. Active in YR: Yes; evidence `0x005D6419..0x005D641F`.

`0x0069AE10` mode comparison:

- If the map record has zero `GameModes` entries, it matches only selected mode string `"standard"` (`0x0083F668`). Active in YR: Yes; evidence `0x0069AE15..0x0069AE30`.
- If it has entries, the selected mode is compared against each map `GameModes` entry in insertion order; first match accepts the record. Active in YR: Yes; evidence `0x0069AE33..0x0069AE65`.

## 5. Displayed Text Order

The visible map-list order is the append order of the filtered `DAT_00A8B8CC` records:

- The modal loop increments `EBX` from `0` upward and never sorts (`0x005E6F17..0x005E6F45`; same pattern at `0x005E6A90..0x005E6ABA` for another rebuild path).
- The temporary vector append helper `0x005EEE40` writes the accepted pointer at current count and increments the count by one; no comparison or reorder exists.
- Therefore stock `MISSIONSMD.PKT` entries appear before loose `*.PKT`, `*.YRO`-derived, and loose `*.YRM` records, subject to Win32 filesystem enumeration order for loose file groups.

Displayed label text is carried by the scenario record, not recomputed by the modal. `0x0069A3B0` and `0x0069A980` construct records from `MultiMaps`/`[Basic]` metadata; modal population passes record pointers to the listbox backing object and list item data rather than adding literal strings in the modal callback.

Active in YR: Yes. The loop and append order are in the live Choose Map dialog callback.

## 6. Initial Selection and Commit State

Initial/rebuild selection in custom message `0x497`:

1. `0x005E6EA6` marks population-in-progress with `DAT_00AC11C8 = 1`.
2. After rebuilding listbox `0x553`, if `DAT_00A8B254 != -1` and `< DAT_00A8B8D8`, the callback loads `DAT_00A8B8CC[DAT_00A8B254]` and scans listbox `0x553` item data for that pointer (`0x005E6F94..0x005E701B`).
3. It sets selection on the matching row with `LB_SETCURSEL` (`0x186`) and stores `DAT_00AC10E0 = DAT_00A8B254`; then clears `DAT_00AC0D30` to `-1` and `DAT_00AC11C8` to `0` (`0x005E701D..0x005E7031`).
4. Helper `0x005E70D0(hwnd, record_ptr)` performs the same pointer-to-list-row selection by iterating listbox count (`0x18B`), reading each item data (`0x199`), and setting current selection (`0x186`) when item data matches.

Accept/commit in `0x005E7160`:

1. Reads current map list selection from `0x553` with `LB_GETCURSEL` (`0x188`); if it returns `-1`, commit fails.
2. Reads selected row item data with `LB_GETITEMDATA` (`0x199`), then scans `DAT_00A8B8CC` until the pointer matches; if no match, commit fails.
3. Reads selected category from `0x6EB` and its item data into `DAT_00A8B23C`.
4. On accepted map/category change, writes `DAT_00A8B250 = selected_mode[10]`, `DAT_00A8B254 = matched_index`, calls selected-mode hooks, updates display controls `0x6EC` and `0x5A8`, and closes the dialog through `0x007757E0`.

Active in YR: Yes. This is the modal path called from the live `0x006ACEE0` Choose Map command.

## 7. Current Rust Implementation Status

Rust currently does not model this modal list population contract.

| Area | Rust status | Evidence |
|---|---|---|
| Map source order | Scans the RA2 directory only, includes loose `mmx/yro/map/mpr/yrm`, then sorts by lowercase display name | `src/app_list_maps.rs:23`, `src/app_list_maps.rs:42` |
| PKT `MultiMaps` source | Not implemented in the current menu list | `src/app_list_maps.rs:23` |
| Filter by selected game mode/category | Not implemented | `src/app_list_maps.rs:23`, `src/ui/skirmish_shell/state.rs:159` |
| Choose Map modal | Not implemented; current `ChooseMap` action cycles `selected_map_idx` in-place | `src/ui/skirmish_shell/state.rs:164` |
| Selection state | Stores only `selected_map_idx`, not record-pointer/item-data identity | `src/ui/skirmish_shell/state.rs:35` |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Parent opens modal | verified | `0x006ACEE0 -> 0x005E68A0`; prior report `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md` | none for list slice |
| Modal create/pump | verified | `0x005E68A0` creates resource `0x6B`, callback `0x005E6920` | none |
| Scenario source collection build | verified | `0x00699980`, strings `MISSIONSMD.PKT`, `MultiMaps`, `*.PKT`, `*.YRO`, `*.YRM` | exact retail file contents not dumped |
| Loose file enumeration order | touched-not-exhausted | `FindFirstFileA` calls at `0x00699AE1`, `0x00699C5D`, `*.YRM` branch | filesystem-specific order should be runtime-observed if exact loose-file order matters |
| Modal category list | verified | `0x005D6130`, `0x005E6EA6` | category definitions/vtable names out of scope |
| Map predicate | verified | `0x005D63E0`, `0x0069AE10`, `0x0069ADF0` | none for list membership |
| Map list append order | verified | `0x005E6F17..0x005E6F45`, `0x005EEE40` | none |
| Initial selection | verified | `0x005E6F94..0x005E7031`, `0x005E70D0` | none |
| Accepted item commit | verified | `0x005E7160` | full accept/cancel return contract covered by sibling report |
| Preview refresh after commit | deferred | out of scope by swarm constraint | sibling preview-invalidation/lifecycle slots |

## 9. Open Questions - Final State

- [RESOLVED] OQ-1 - Does the modal scan files directly? No; it consumes `DAT_00A8B8CC/DAT_00A8B8D8`. Evidence: `0x005E6F17`, `0x005E7160`.
- [RESOLVED] OQ-2 - Which files seed the global list? `MISSIONSMD.PKT`, loose `*.PKT`, `*.YRO`/embedded PKT, loose `*.YRM`. Evidence: `0x00699980`, strings at `0x0083F520`, `0x0083F514`, `0x0083F50C`, `0x0083F504`, `0x0083F490`.
- [RESOLVED] OQ-3 - Are records sorted in the modal? No; accepted records are appended while iterating the global array forward. Evidence: `0x005E6F17..0x005E6F45`, `0x005EEE40`.
- [RESOLVED] OQ-4 - What makes a record valid for the current category? Random sentinel asks selected mode vtable `+0x3C`; otherwise optional official gate, then `GameModes` match or default `"standard"` match. Evidence: `0x005D63E0`, `0x0069AE10`.
- [RESOLVED] OQ-5 - What is list item data? Scenario-record pointer. Evidence: `0x005E70D0` and `0x005E7160` compare listbox item data against `DAT_00A8B8CC[i]`.
- [RESOLVED] OQ-6 - How is initial selection set? By finding list item data equal to `DAT_00A8B8CC[DAT_00A8B254]`, not by matching text. Evidence: `0x005E6F94..0x005E701B`, `0x005E70D0`.
- [DEFERRED] OQ-7 - Exact category display strings and vtable method names. Category: out-of-scope; list membership only needs the item-data mode pointer and string comparison path.
- [DEFERRED] OQ-8 - Exact PreviewPack/list preview side effects after selection. Category: out-of-scope by swarm constraint.

## Sources

- Ghidra decompile/disassembly: `0x005E68A0`, `0x005E6920`, `0x005D6130`, `0x005D63E0`, `0x005E70D0`, `0x005E7160`, `0x005E7BF0`, `0x005EEE40`, `0x005EF0B0`, `0x00699980`, `0x006994F0`, `0x0069A3B0`, `0x0069A980`, `0x0069ADF0`, `0x0069AE10`.
- String/data evidence: `0x0083F520` `MISSIONSMD.PKT`, `0x0083F514` `MultiMaps`, `0x0083F50C` `*.PKT`, `0x0083F504` `*.YRO`, `0x0083F490` `*.YRM`, `0x0082BC30` `RandMap.Sed`, `0x0083F668` `standard`, `0x0083F3F0` `MaxPlayers`, `0x0083F3FC` `MinPlayers`.
- Prior related docs: `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_RETURN_CONTRACT_GHIDRA_REPORT.md`, `traces/SKIRMISH_CHOOSE_MAP_ACTION_TRACE.md`.
- Rust scan: `src/app_list_maps.rs`, `src/ui/skirmish_shell/state.rs`.
