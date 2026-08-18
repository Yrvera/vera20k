# Skirmish Side/Country/Team Final Writes - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x006AE6E0`, `0x006ACEE0`, `0x004E3A00`, `0x004E4170`, `0x004E5AC0`, `0x004E5B60`, `0x004E6030`, `0x0069B760`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard offline Skirmish dialog `0x102` final Start Game writes for local and AI side/country/team selections.  
**Non-Scope:** color combo writes except adjacency disambiguation, start-position semantics except where needed to avoid confusing neighboring arrays, online/WOL lobby behavior beyond gates visible in these helpers.  
**Confidence:** High for control IDs, item data, final write destinations, and validation gates in the scoped path.  
**Active in YR:** Yes. `0x006AE31C..0x006AE328` installs dialog proc `0x006AE3F0` for dialog id `0x102`; `0x006AE3F0` routes `WM_COMMAND` to `0x006ACEE0`.

## 1. Overview

Standard offline Skirmish does not write a separate final "side" enum from dialog `0x102`. The country/side combo item data is the country index/sentinel (`-3`, `-2`, `0..9`), and later systems derive side from the selected country type. Team is a separate combo family (`0x76D..0x774`) whose item data is `-2` or `0..3`.

On Start Game (`0x617`, notification `0`), `FUN_006ACEE0` rereads current combo item data and writes local selection into the session object at `0x00A8B238`, local globals/new node fields, and AI selections into five parallel arrays. Color and start are adjacent in those arrays; this report identifies them only to prevent team/country misassignment.

## 2. Control IDs And Item Data

| Row slot | Player/AI row state | Country/side combo | Team combo | Active in YR |
|---:|---:|---:|---:|---|
| local `0` | none in final AI loop | `0x6A1` | `0x76D` | Yes, standard offline |
| AI `1` | `0x50B` | `0x510` | `0x76E` | Yes, standard offline |
| AI `2` | `0x50E` | `0x513` | `0x76F` | Yes, standard offline |
| AI `3` | `0x516` | `0x51E` | `0x770` | Yes, standard offline |
| AI `4` | `0x51A` | `0x514` | `0x771` | Yes, standard offline |
| AI `5` | `0x51B` | `0x51F` | `0x772` | Yes, standard offline |
| AI `6` | `0x51C` | `0x520` | `0x773` | Yes, standard offline |
| AI `7` | `0x51D` | `0x521` | `0x774` | Yes, standard offline |

Evidence: control mappers `FUN_004E37D0` (country), `FUN_004E5940` (team), and `FUN_006ACEE0` apply-loop assembly `0x006AD3C1..0x006AD4E6`. Active in YR: Yes.

Country/side combo item data:

| Item data | Meaning | Evidence | Active in YR |
|---:|---|---|---|
| `-3` | observer/special row; flag maps to `obsi.pcx` | `FUN_004E3B00`, `FUN_004E3560`; observer path gated by row/local observer state | Conditional, not the normal standard offline human/AI selection |
| `-2` | Random country; flag maps to `rani.pcx` | `FUN_004E3A00`, `FUN_004E3560`, `FUN_0069B760` random handling | Yes |
| `0..9` | country indices from `[Countries]` | `FUN_004E3A00` inserts `CountryTypeClass+0xB8`; `ini/rulesmd.ini:959..971` | Yes |

Stock YR country item data:

| Item data | Country | Side group from `[Sides]` | Evidence | Active in YR |
|---:|---|---|---|---|
| `0` | `Americans` | GDI/Allied | `ini/rulesmd.ini:959..986` | Yes |
| `1` | `Alliance` | GDI/Allied | `ini/rulesmd.ini:960..986` | Yes |
| `2` | `French` | GDI/Allied | `ini/rulesmd.ini:962..986` | Yes |
| `3` | `Germans` | GDI/Allied | `ini/rulesmd.ini:963..986` | Yes |
| `4` | `British` | GDI/Allied | `ini/rulesmd.ini:964..986` | Yes |
| `5` | `Africans` | Nod/Soviet | `ini/rulesmd.ini:966..987` | Yes |
| `6` | `Arabs` | Nod/Soviet | `ini/rulesmd.ini:967..987` | Yes |
| `7` | `Confederation` | Nod/Soviet | `ini/rulesmd.ini:968..987` | Yes |
| `8` | `Russians` | Nod/Soviet | `ini/rulesmd.ini:969..987` | Yes |
| `9` | `YuriCountry` | ThirdSide/Yuri | `ini/rulesmd.ini:971..987` | Yes |

Team combo item data:

| Item data | Meaning | Evidence | Active in YR |
|---:|---|---|---|
| `-2` | None/no team | `FUN_004E5B60` inserts optional `GUI:None...` row with item data `-2`; validation treats local `<0` as no team gate | Conditional; inserted when selected mode `MustAlly` is false via vtable `+0x2C`, resolved by `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md` |
| `0` | Team A | `FUN_004E5AC0` / `FUN_004E5B60` A-D table and item data loop | Yes |
| `1` | Team B | same | Yes |
| `2` | Team C | same | Yes |
| `3` | Team D | same | Yes |

Player/AI row state item data:

| Item data | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|
| `-1` | inactive row, visible `GUI:None` / `STT:PlayerNone`; excluded from active AI count/final live arrays | `FUN_006AE6E0` inserts row state `-1`; `0x006AD453..0x006AD475` accepts only `0..2`; labels resolved by `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md` | Yes |
| `0` | active AI row, visible `GUI:AIHard` / `STT:PlayerGeniusAI` | `FUN_006AE6E0` inserts item data `0`; `FUN_006ACEE0` counts and writes it | Yes |
| `1` | active AI row, visible `GUI:AINormal` / `STT:PlayerSmartAI` | `FUN_006AE6E0` inserts item data `1`; `FUN_006ACEE0` counts and writes it | Yes |
| `2` | active AI row, visible `GUI:AIEasy` / `STT:PlayerDumbAI` | `FUN_006AE6E0` inserts item data `2`; `FUN_006ACEE0` counts and writes it | Yes |

## 3. Local Player Final Writes

| Selection | Source control | Destination | Exact behavior | Evidence | Active in YR |
|---|---:|---|---|---|---|
| Country/side item data | `0x6A1` | session object `0x00A8B238` via `FUN_0069B760` | selected item data is read by `FUN_004E4170`; if `-2`, randomize flag `1` makes `FUN_0069B760` choose random country `0..9`, store chosen value at `+0x184`, store sentinel `-2` at `+0x188`, and mirror to `+0x174/+0x178`; non-random stores selected value and `-1` sentinel | call site `0x006AD3A4..0x006AD3BA`; `FUN_0069B760` | Yes |
| Team item data | `0x76D` | `DAT_00A8B3A4`, then new node field `+0x63` | `FUN_004E6030` returns the selected team item data; Start Game writes it to `DAT_00A8B3A4`, then copies it into the newly allocated node at `+0x63` | `0x006AD61C..0x006AD641`, `0x006AD694..0x006AD699` | Yes |

Adjacent local fields, not in scope but disambiguated: color control `0x6A2` goes through `FUN_0069B7E0`; start control `0x6A3` writes `DAT_00A8B39C` and node `+0x5B`. Evidence: `0x006AD5FE..0x006AD63B`, `0x006AD68B..0x006AD691`. Active in YR: Yes.

## 4. AI Row Final Writes

The AI apply loop uses local loop index `0..6`, stores into slot `index+1`, and skips rows whose player/AI combo item data is not `0`, `1`, or `2`. Active in YR: Yes; evidence `0x006AD3C1..0x006AD4F2`.

| Destination | Source | Meaning | Evidence | Active in YR |
|---|---|---|---|---|
| `DAT_00A8B27C[slot]` where slot `1..7` | row state `0x50B,0x50E,0x516,0x51A..0x51D` | active AI row state/difficulty item data | `0x006AD453..0x006AD49E` | Yes |
| `DAT_00A8B29C[slot]` where slot `1..7` | country controls `0x510,0x513,0x51E,0x514,0x51F,0x520,0x521` | AI country/side item data (`-3`, `-2`, `0..9`) | `0x006AD498..0x006AD4AC` | Yes |
| `DAT_00A8B2FC[slot]` where slot `1..7` | team controls `0x76E..0x774` | AI team item data (`-2`, `0..3`) | `0x006AD4DB..0x006AD4E6` | Yes |

Adjacent AI fields, not in scope but disambiguated: `DAT_00A8B2BC[slot]` is color from `0x522..0x528`; `DAT_00A8B2DC[slot]` is start position from `0x6A4..0x6AB`. Evidence: `0x006AD4B3..0x006AD4D4` plus mappers `FUN_004E41D0` and `FUN_004E4E60`. Active in YR: Yes.

The persisted/snapshot AI row triplet at `DAT_00A8B3EC..` records row state class, country, and color only:

| Destination | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8B3EC + row*12` | row state converted: `-1 -> 1`, `0 -> 4`, `1 -> 5`, `2 -> 6`, other -> `0 or 6` via the binary expression | `0x006AD5B0..0x006AD5E3` | Yes |
| `DAT_00A8B3F0 + row*12` | country/side item data from `FUN_004E4170` | `0x006AD594..0x006AD5E6` | Yes |
| `DAT_00A8B3F4 + row*12` | color item data, adjacent/out-of-scope | `0x006AD5A5..0x006AD5E8` | Yes |

No team item data is written to that `DAT_00A8B3EC` triplet in this Start Game apply block. Active in YR: Yes; evidence `0x006AD5E3..0x006AD5E8`.

## 5. Validation And Fallbacks

| Case | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Too many active AI rows for selected map start count | Start Game shows message string IDs `0x437/0x438`, re-enables button `0x617`, and returns before final writes | `FUN_006ACEE0` after active AI count, before apply block | Yes |
| Fewer than two total players | Start Game shows string IDs `0x43F/0x440`, re-enables `0x617`, and returns | `FUN_006ACEE0` active count check | Yes |
| Local team set and every active AI has same team | Start Game shows string IDs `0x457/0x458`, re-enables `0x617`, and returns before final writes | `0x006AD16C..0x006AD236`; local `0x76D` compared against each active AI `0x76E..0x774` | Yes |
| Local team is `-2` or otherwise `<0` | Same-team validation is skipped | `0x006AD16C..0x006AD17C` jumps past validation when local team `<0` | Yes |
| Country item data outside `-3..9` | `FUN_004E4170` falls back to selected `MPModes` mode/category vtable `+0x28` if `DAT_00A8B23C` exists; otherwise returns `-2` | `FUN_004E4170`; `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md` | Yes |
| Random local country | `FUN_0069B760` with randomize flag `1` resolves `-2` to random `0..9` and stores `-2` as the sentinel field | `FUN_0069B760`; call site pushes `1` at `0x006AD3B7` | Yes |

## 6. Current Rust Implementation Status

Rust currently models one player country and one AI country in `SkirmishSettings`, plus a dev shell state with a single enabled opponent by default. It has no stock seven-row AI state array, no `-2` Random / `-3` Observer sentinels, and no A-D/None team final-write model matching the binary.

Evidence: `src/ui/main_menu.rs:18..143`, `src/ui/skirmish_shell/state.rs:25..87`, `src/app_skirmish.rs:45..57`. Active in YR: not applicable to binary; this is current Rust status.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Dialog `0x102` activation | verified | `0x006AE31C..0x006AE328` | none |
| Start Game command path | verified | `FUN_006ACEE0`, button `0x617`, notification `0` | none |
| Country control IDs and item data | verified | `FUN_004E37D0`, `FUN_004E3A00`, `FUN_004E4170` | none for country/team final-write naming |
| Team control IDs and item data | verified | `FUN_004E5940`, `FUN_004E5AC0`, `FUN_004E5B60`, `FUN_004E6030`; `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md` | none for offline Team None condition |
| Local country final write | verified | `0x006AD3A4..0x006AD3BA`, `FUN_0069B760` | none |
| Local team final write | verified | `0x006AD61C..0x006AD641`, `0x006AD694..0x006AD699` | none |
| AI country final write | verified | `0x006AD498..0x006AD4AC` | none |
| AI team final write | verified | `0x006AD4DB..0x006AD4E6` | none |
| Team validation | verified | `0x006AD16C..0x006AD236` | none |
| Color writes | deferred | prior color report | out-of-scope |
| Online/WOL observer/closed row behavior | deferred | gates in `FUN_004E3B90`, `FUN_004E5D60` | out-of-scope for standard offline Skirmish |

## 8. Open Questions - Final State

[RESOLVED] OQ-SCT-001 - Does final Start Game write a separate side enum? No; it writes country item data via `FUN_004E4170`/`FUN_0069B760`, and side is derived from country data later. Evidence: `0x006AD3A4..0x006AD3BA`, `0x006AD498..0x006AD4AC`, `COUNTRY_SIDE_TYPE_CLASSES.md`, `ini/rulesmd.ini:981..987`.

[RESOLVED] OQ-SCT-002 - Which controls are team controls? `0x76D..0x774`, with item data `-2`/`0..3`. Evidence: `FUN_004E5940`, `FUN_004E5AC0`, `FUN_004E5B60`, final read at `0x006AD61C..0x006AD627`.

[RESOLVED] OQ-SCT-003 - Where is local team written? `DAT_00A8B3A4`, then new node `+0x63`. Evidence: `0x006AD627..0x006AD641`, `0x006AD694..0x006AD699`.

[RESOLVED] OQ-SCT-004 - Where is AI team written? `DAT_00A8B2FC[slot]` for slots `1..7`. Evidence: `0x006AD4DB..0x006AD4E6`.

[RESOLVED] OQ-SCT-005 - Is a team written into the AI persisted row triplet `DAT_00A8B3EC..`? No; that triplet stores row-state class, country, and color only. Evidence: `0x006AD5E3..0x006AD5E8`.

[RESOLVED] OQ-SCT-006 - Exact CSF display labels for AI row state item data `0`, `1`, `2`: `0 -> GUI:AIHard / STT:PlayerGeniusAI`, `1 -> GUI:AINormal / STT:PlayerSmartAI`, `2 -> GUI:AIEasy / STT:PlayerDumbAI`. The inactive row is `-1 -> GUI:None / STT:PlayerNone`; `GUI:Closed` is online/WOL, not offline dialog `0x102`. Evidence: `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`.

[RESOLVED] OQ-SCT-007 - Exact selected-mode vtable `+0x2C` meaning controlling whether Team None is inserted. `FUN_004E5B60` calls the selected `MPModes` mode object method; the concrete method at `0x005D5DC0` returns `-2` when `MustAlly` is false and `0` when true. Team `None` is inserted only for the negative return. Evidence: `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md`.

## Sources

- Ghidra decompiled/rechecked: `FUN_006AE2C0`, `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_004E37D0`, `FUN_004E3A00`, `FUN_004E3B00`, `FUN_004E3B90`, `FUN_004E4170`, `FUN_004E5940`, `FUN_004E5AC0`, `FUN_004E5B60`, `FUN_004E5D60`, `FUN_004E6030`, `FUN_0069B760`.
- Ghidra assembly context: `0x006AE31C..0x006AE328`, `0x006AD16C..0x006AD236`, `0x006AD3A4..0x006AD4E6`, `0x006AD5E3..0x006AD641`, `0x006AD68B..0x006AD699`.
- INI: `ini/rulesmd.ini:959..989`.
- Prior docs cross-checked: `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`, `COUNTRY_SIDE_TYPE_CLASSES.md`, `COUNTRY_ICON_SHP_SELECTOR_GHIDRA_REPORT.md`, `traces/SKIRMISH_PLAYER_AI_COMBOS_FLAGS_TRACE.md`.
