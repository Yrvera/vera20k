# Skirmish Selected Map Token Load Consumer - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x006AE6E0`, `0x005E7BF0`, `0x00683AB0`, `0x00684620`, `0x00686730`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Offline Skirmish selected-map token/index mirrors `DAT_00A8B3C4/3C8` and live selected globals `DAT_00A8B250/254` from Start Game commit through the scenario/map load handoff.  
**Non-Scope:** Preview rendering, map-list sorting/population, random terrain generation internals, and online/WOL launch variants except when needed to distinguish inactive branches.  
**Confidence:** High for the offline Skirmish handoff and first loader consumer; Medium for exact UI-facing meaning of some fallback CD/source checks.  
**Active in YR:** Yes. Evidence: standard offline Skirmish uses game mode `5`, `Main_Game @ 0x0052E6F8..0x0052E745` takes the non-campaign filename-buffer path into `ScenarioClass__Start_Scenario @ 0x00683AB0`, and the setup dialog path reaches `0x006ACEE0` / `0x006AE6E0`.

## 1. Overview

The Start Game branch mirrors the selected mode token and selected scenario index into `DAT_00A8B3C4` and `DAT_00A8B3C8`, but those mirrors are not the direct post-shell map filename consumer. The actual filename opened after shell exit is the selected record path already copied by `0x005E7BF0` from `DAT_00A8B8CC[DAT_00A8B254]+0x58` into both `DAT_00A8B8E0` and `ScenarioClass+0x125C`.

After the shell exits, `Main_Game` calls `ScenarioClass__Start_Scenario` with `ECX = ScenarioClass+0x125C` and `param_2 = -1` for the non-campaign Skirmish path. `ScenarioClass__Start_Scenario` is therefore the first scenario-load consumer of the committed selected map path; it opens that filename via `CCFileClass__Constructor(param_1)` before calling `ScenarioClass__Read_Scenario`.

## 2. Key Offsets And Globals

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8B250` | live selected mode/category token | `0x005E7160`, `0x006ACEE0`, `0x006AE6E0` | Yes |
| `DAT_00A8B254` | live selected scenario record index into `DAT_00A8B8CC` | `0x005E7160`, `0x005E7BF0`, `0x00683B97` | Yes |
| `DAT_00A8B3C4` | persisted/launch mirror of selected mode token | Start write `0x006AD35E`; setup-init reads/writes `0x006AEAC5..0x006AEB07` | Yes |
| `DAT_00A8B3C8` | persisted/launch mirror of selected scenario index | Start write/clamp `0x006AD364..0x006AD36B`; setup-init consume `0x006AEB22..0x006AEB55` | Yes |
| `DAT_00A8B8CC` | scenario record pointer array | `0x005E7BF0`, `0x005E7160`, prior record decode report | Yes |
| record `+0x58` | ASCII map file/path string | `0x005E7BF0` copies and opens it | Yes |
| `DAT_00A8B8E0` | selected map path display/preview global | `0x005E7BF0`, `0x006ACEE0` save-before-chooser | Yes |
| `ScenarioClass+0x125C` | scenario filename buffer used by scenario start/load | `0x005E7BF0`; `0x0052E73F..0x0052E745`; `0x00683AB0` | Yes |
| `ScenarioClass+0x1254` | selected map index copied for scenario state | `0x005E7460` writes from `DAT_00A8B254` | Yes |

## 3. Core Logic

### Choose Map commit and restore

`0x005E7160` commits an accepted Choose Map selection by matching listbox `0x553` item data back to `DAT_00A8B8CC[i]`. It writes `DAT_00A8B254 = i`, and when the selected mode/category changes it writes `DAT_00A8B250 = selected_mode[10]`. Active in YR: Yes; evidence `0x005E7160`.

The parent `0x006ACEE0` `0x5AA` branch saves the prior `DAT_00A8B250`, `DAT_00A8B254`, and current `DAT_00A8B8E0` before opening the modal. On cancel/result `2`, it restores `DAT_00A8B250/254` and reloads the selected record/preview state. On accept, it calls `0x005E7BF0(DAT_00A8B254)`; if that selected-record load fails, it restores the saved live token/index and returns. Active in YR: Yes for the branch; conditional for cancel/load-failure subpaths. Evidence `0x006AD8E7..0x006ADB52`, especially calls at `0x006AD967` and `0x006ADA7D`.

### Selected-record loader

`0x005E7BF0(index)` is the selected-record loader that turns the committed index into path globals. For normal `index >= 0`, it opens record `+0x58`, copies display text to `DAT_00A8B322`, copies digest to `DAT_00A8BAE2`, copies record `+0x58` to `DAT_00A8B8E0`, then copies `DAT_00A8B8E0` into `ScenarioClass+0x125C`. Active in YR: Yes; evidence `0x005E7D87..0x005E7D9E` for `DAT_00A8B8E0`, and `0x005E7DA0..0x005E7DCA` for the second copy into `ScenarioClass+0x125C`.

If `index == -1`, `0x005E7BF0` clears the selected-map globals and returns `0`. Active in YR: Conditional; evidence `0x005E7BF8..0x005E7C26`.

The loader also computes selected player/source metadata and opens `DAT_00A8B8E0` twice to store a file vtable `+0x2C` result in `DAT_00A8BB04`. Those side effects are active, but they are not the post-shell filename selection mechanism. Active in YR: Yes; evidence `0x005E7E21..0x005E7E75`.

### Start Game mirror write

The Start Game branch mirrors the live selected state:

- `DAT_00A8B3C4 = DAT_00A8B250`.
- `DAT_00A8B3C8 = DAT_00A8B254`.
- If `DAT_00A8B254 >= DAT_00A8B8D8`, it forces `DAT_00A8B3C8 = 0`.

Active in YR: Yes; evidence `0x006AD34B..0x006AD36B`. This mirror happens after Start acceptance and before row/session packing. It does not call `0x005E7BF0` and does not copy a filename.

### Setup init consumes mirrors, not scenario load

`0x006AE6E0` is the main setup dialog initializer and the only non-Start function with xrefs to `DAT_00A8B3C4/3C8` found in this slot. It validates `DAT_00A8B3C4`, resolves the mode object through `FUN_005E2F80`, then rewrites `DAT_00A8B3C4` and `DAT_00A8B250` from the resolved mode object's token. It validates `DAT_00A8B3C8` against `DAT_00A8B8D8`, calls `0x005E7BF0`, and assigns `DAT_00A8B254 = load_success ? DAT_00A8B3C8 : 0`. Active in YR: Yes; evidence `0x006AEAC5..0x006AEB55`.

If the record selected by `DAT_00A8B3C8` is not accepted by the selected mode filter (`0x005D63E0`), setup init falls back to the selected mode's default map index via vtable `+0x44`; if that returns `-1`, it falls back to the first mode object and asks its `+0x44`. It then copies that record's path into `DAT_00A8B8E0`, and writes both `DAT_00A8B254` and `DAT_00A8B3C8` to the fallback index. Active in YR: Conditional; evidence `0x006AEB62..0x006AEBD7`.

This establishes the role of `DAT_00A8B3C4/3C8`: they persist the selected setup state for the next dialog initialization. They are not the first post-shell scenario file opener.

### Scenario start and first loader consumer

For non-campaign Skirmish, `Main_Game` calls `ScenarioClass__Start_Scenario @ 0x00683AB0` with `ECX = ScenarioClass+0x125C` and stack argument `-1`. Active in YR: Yes for standard Skirmish; evidence `0x0052E6F8` branches away from the campaign index path when `g_GameMode != 0`, and `0x0052E737..0x0052E745` loads `DAT_00A8B230`, pushes `EBP` (`-1`), sets `ECX` to `ScenarioClass+0x125C`, and calls `0x00683AB0`.

`ScenarioClass__Start_Scenario` copies `param_1` into `ScenarioClass+0x125C` again, normalizes the path with `0x007DCFC4`, opens it with `CCFileClass__Constructor(param_1)`, reads intro/briefing metadata if available, and then calls `ScenarioClass__Read_Scenario @ 0x00684620`. Active in YR: Yes; evidence `0x00683AB0` decompile and call at `0x00683D21`.

`ScenarioClass__Read_Scenario` copies the same filename into a local buffer, detects random-map sentinel state, and for non-random maps calls `ScenarioClass__Read_Scenario_INI @ 0x00686730`. `ScenarioClass__Read_Scenario_INI` constructs a file object from the filename, opens it through `SHAPipe__Constructor`, copies the filename into `ScenarioClass+0x125C`, and calls `ScenarioClass__Full_Init @ 0x00686B20`. Active in YR: Yes; evidence `0x00684620`, `0x00686730`, xref `0x006849C9`.

`ScenarioClass__Start_Scenario` still reads `DAT_00A8B254` on the non-campaign path, but that read is for selected-record source/CD availability fallback, not filename selection. It indexes `DAT_00A8B8CC[DAT_00A8B254]`, calls `0x0069AC30`, and may call `0x0069ACC0`. Active in YR: Conditional on `DAT_00A8B254 != -1` and source availability; evidence `0x00683B97..0x00683BC7`.

## 4. INI Keys

No INI key directly controls the handoff between `DAT_00A8B250/254`, `DAT_00A8B3C4/3C8`, `DAT_00A8B8E0`, and `ScenarioClass+0x125C`. Map-list records and their file paths are populated elsewhere from PKT/map metadata; this report intentionally does not re-investigate sorting or record construction. Active in YR: Yes as a negative finding for this slice; evidence: scoped functions above do not call INI readers for the selected-token handoff.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Choose Map accept | `0x005E7160` writes live selected token/index; parent calls `0x005E7BF0` | `0x005E7160`, `0x006ADA7D` | Yes |
| Choose Map cancel | parent restores saved live `DAT_00A8B250/254` | `0x006AD95B..0x006AD967` | Conditional |
| Setup initialization | `0x006AE6E0` consumes `DAT_00A8B3C4/3C8` and reloads selected record/path | `0x006AEAC5..0x006AEB55` | Yes |
| Start Game accept | mirrors live selected token/index to `DAT_00A8B3C4/3C8` | `0x006AD34B..0x006AD36B` | Yes |
| Scenario launch | `Main_Game` passes `ScenarioClass+0x125C` to `ScenarioClass__Start_Scenario` for Skirmish | `0x0052E737..0x0052E745` | Yes |
| First file open | `ScenarioClass__Start_Scenario` opens `param_1` via `CCFileClass__Constructor` | `0x00683AB0` | Yes |

## 6. Current Rust Implementation Status

Rust currently has a simpler selected-map index path:

| Area | Current Rust status | Evidence |
|---|---|---|
| setup state | stores only `SkirmishShellState.selected_map_idx` | `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs:36` |
| Choose Map action | cycles `selected_map_idx` in-place, no modal accept/cancel restore | `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs:165` |
| Start launch | reads `skirmish_settings.selected_map_idx`, gets `available_maps[index].file_name`, then enters `GameScreen::Loading { map_name }` | `C:/Users/enok/Documents/ra2-rust-game/src/app.rs:411` |

Rust does choose a map filename by index, which matches the high-level outcome when the index is valid. It does not yet model the retail record loader contract: live token/index, launch mirrors, selected record path copy into a scenario filename buffer, loader-failure restore, or setup-init fallback to mode default map.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Start Game selected mirror writes | verified | `0x006AD34B..0x006AD36B` | none |
| xrefs to `DAT_00A8B3C4/3C8` | verified | `get_xrefs_to 0x00A8B3C4/0x00A8B3C8` | none for standard Skirmish; nonstandard online paths out of scope |
| setup-init mirror consumption | verified | `0x006AE6E0`, `0x006AEB3C` call to `0x005E7BF0` | none |
| selected-record path copy | verified | `0x005E7BF0`, `0x005E7D87..0x005E7DCA` | none |
| Choose Map accept/restore relation | verified | `0x005E7160`, `0x006ACEE0` | none |
| Main_Game Skirmish launch call | verified | `0x0052E737..0x0052E745` | exact shell loop return state outside scope |
| first scenario file opener | verified | `0x00683AB0`; `0x00684620`; `0x00686730` | none |
| random-map generation path | deferred | `0x00684620` random branch to `0x00597A10` | out-of-scope for selected stock/custom file handoff |
| online/IPX/WOL variants | deferred | extra callers of `0x005E7BF0` and `0x00683AB0` exist | out-of-scope; standard offline Skirmish path verified |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Which field is mirrored at Start? -> DAT_00A8B3C4 receives DAT_00A8B250 and DAT_00A8B3C8 receives DAT_00A8B254, with B3C8 clamped to 0 if the live index is >= DAT_00A8B8D8.` (evidence: `0x006AD34B..0x006AD36B`)
- `[RESOLVED] OQ-2 - Do B3C4/B3C8 directly feed the scenario file open after shell exit? -> No; xrefs in this slot show Start writes and setup-init reads/writes, while scenario start receives ScenarioClass+0x125C as filename.` (evidence: `get_xrefs_to 0x00A8B3C4/0x00A8B3C8`, `0x0052E737..0x0052E745`)
- `[RESOLVED] OQ-3 - What turns the selected index into a filename? -> 0x005E7BF0(index) copies record +0x58 into DAT_00A8B8E0 and ScenarioClass+0x125C.` (evidence: `0x005E7BF0`)
- `[RESOLVED] OQ-4 - Which committed field actually selects the file? -> DAT_00A8B254 selects DAT_00A8B8CC[index] for 0x005E7BF0; the file string is record +0x58. After shell exit, ScenarioClass+0x125C is the file argument.` (evidence: `0x005E7BF0`, `0x0052E745`, `0x00683AB0`)
- `[RESOLVED] OQ-5 - How does Choose Map cancel relate? -> The parent saves live B250/B254 before modal open and restores them on result 2, then reloads selected-record/preview state.` (evidence: `0x006AD8E7..0x006AD967`)
- `[RESOLVED] OQ-6 - How does Choose Map accept relate? -> 0x005E7160 writes live B250/B254; parent calls 0x005E7BF0(DAT_00A8B254), and restores old B250/B254 if that loader fails.` (evidence: `0x005E7160`, `0x006ADA7D..0x006ADB52`)
- `[RESOLVED] OQ-7 - What is the first loader consumer? -> ScenarioClass__Start_Scenario opens param_1, which is ScenarioClass+0x125C for standard Skirmish, before ScenarioClass__Read_Scenario and ScenarioClass__Read_Scenario_INI continue parsing.` (evidence: `0x0052E745`, `0x00683AB0`, `0x00684620`, `0x00686730`)
- `[RESOLVED] OQ-8 - Does ScenarioClass__Start_Scenario read DAT_00A8B254? -> Yes, but only for non-campaign selected-record/source availability checks, not to derive the filename argument.` (evidence: `0x00683B97..0x00683BC7`)
- `[DEFERRED] OQ-9 - What exactly does the random-map branch do after 0x00684620 detects IsRandom?` (category: out-of-scope; reason: user scope is selected token/index mirrors to file load, not random terrain generation; next-step-if-pursued: trace `0x00597A10` and generated scenario filename/state)
- `[DEFERRED] OQ-10 - Do online/IPX/WOL launch variants use the same mirrors differently?` (category: out-of-scope; reason: this slot is standard offline Skirmish; next-step-if-pursued: separate WOL/IPX selected-map launch report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Live selected index is loaded into a record path before Start; Start itself mirrors but does not derive the filename. | `0x005E7BF0`, `0x006AD34B..0x006AD36B` | missing retail contract | `src/ui/skirmish_shell/state.rs`, future map-record/session model | Keep selected record identity/path separate from launch mirrors; Start should preserve current accepted map, not silently cycle or re-resolve ad hoc. | Select a map in the modal, press Start, and load exactly that record path. | Do not treat `DAT_00A8B3C8` as the direct file-open index. |
| Scenario start consumes `ScenarioClass+0x125C` as the filename in Skirmish. | `0x0052E737..0x0052E745`, `0x00683AB0` | partially matched by `map_name` string but lacks scenario buffer semantics | `src/app.rs:411`, loading/scenario init path | Launch should receive the selected record file/path copied by the accepted selected-record loader. | A custom loose map with display name differing from filename loads by filename/path, not display title. | Do not load by UI label text. |
| Choose Map cancel restores prior live token/index and path-backed state. | `0x006AD8E7..0x006AD967` | missing | future Choose Map modal state | Cancel must leave the previous selected map and launch path unchanged. | Open Choose Map, highlight another map, cancel, press Start; original map loads. | Do not update global selected index on highlight or on button click alone. |
| Setup init consumes B3C4/B3C8 and falls back when invalid/not accepted by selected mode. | `0x006AEAC5..0x006AEBD7` | missing | shell init/session restore | Persisted selected index should be validated against map count and mode filter, then fallback to mode default/first-mode default. | Persisted index out of range starts setup on the retail fallback map, not a missing path. | Do not modulo-wrap invalid indices. |

## Stale Docs / Follow-up Docs

- `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md` can sharpen its wording: replace "selected map token/index mirrors ... scenario/map load selection path" with "Start mirrors selected token/index for later setup/session state; the post-shell filename is `ScenarioClass+0x125C`, populated earlier by `0x005E7BF0(DAT_00A8B254)` from selected record `+0x58`."

## Sources

- Ghidra read-only decompile / assembly context: `0x006ACEE0`, `0x006AE6E0`, `0x005E7160`, `0x005E7BF0`, `0x005E7460`, `0x00683AB0`, `0x00684620`, `0x00686730`, `0x00686B20`, `0x0069AC30`, `0x0069ACC0`.
- Ghidra xrefs: `DAT_00A8B3C4`, `DAT_00A8B3C8`, `DAT_00A8B250`, `DAT_00A8B254`, `DAT_00A8B8E0`; function xrefs to `ScenarioClass__Start_Scenario`, `ScenarioClass__Read_Scenario`, and `0x005E7BF0`.
- Prior docs referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHOOSE_MAP_RECORD_DECODE_FIELDS_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md`, `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`.
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`.
