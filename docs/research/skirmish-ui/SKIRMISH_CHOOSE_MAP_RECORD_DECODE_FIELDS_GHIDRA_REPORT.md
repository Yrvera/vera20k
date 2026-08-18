# Skirmish Choose Map Record Decode Fields - Ghidra Research Report

**Address(es):** `0x005E7BF0`, `0x0069A3B0`, `0x0069A980`, `0x0069AD80`, `0x0069ADF0`, `0x005E7160`, `0x006ACEE0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The selected-map record fields used by the Choose Map modal and parent Skirmish setup for display text, filesystem path/name, digest, min/max players, official flag, selected-map globals, and post-accept text-control update.  
**Non-Scope:** PreviewPack decode, preview object lifecycle, random terrain generation, exact static-control subclass thunk internals, category vtable identity, and full scenario-list source ordering already covered by sibling reports.  
**Confidence:** High for field offsets, selected-map global copies, active Skirmish call chain, and PKT/YRO/YRM display string construction after follow-up correction.  
**Active in YR:** Yes. Evidence: standard Skirmish dialog `0x102` command route reaches `FUN_006ACEE0`; the `0x5AA` Choose Map branch calls `0x005E68A0`, accept reaches `0x005E7160`, and parent accept/cancel refresh calls the selected-record loader `0x005E7BF0`.

## 1. Overview

Choose Map records are fixed-size `0x1BC` byte objects stored in `DAT_00A8B8CC`. The list modal uses record pointers as item data; the visible map name is the record's wide string at byte offset `+0x00`, while the file/path string used for loading is the ASCII string at `+0x58`.

After a map is accepted, `0x005E7160` commits the selected index and category object, then the parent Skirmish setup calls `0x005E7BF0(DAT_00A8B254)`. That loader copies selected record fields into globals used by the setup labels, preview loader, scenario class path, and start/player rules.

## 2. Record Fields And Selected Globals

| Field / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| record size `0x1BC` | scenario/map record allocation size | `operator_new(0x1bc)` in `0x00699980`; constructors `0x0069A3B0`, `0x0069A980` | Yes - records populate the Choose Map list |
| record `+0x00` | wide display title/name used by list/static text paths | `0x0069A3B0` initializes/copies via `0x007CA489`/`0x007CA422`; `0x005E7BF0` copies it to `DAT_00A8B322` | Yes |
| record `+0x58` | ASCII map file/path string | `0x0069A3B0` writes `param_1+0x2c` as wide-pointer math = byte `+0x58`; `0x0069A980` `_strncpy(record+0x58, ..., 0x104)`; `0x005E7BF0` opens/copies it | Yes |
| record `+0x15B` | explicit terminator for `+0x58` path buffer | `0x0069A3B0`, `0x0069A980` set byte `+0x15B = 0` after bounded copy | Yes |
| record `+0x15C` | digest string, 0x20-byte bounded source | `0x0069AD80`; `0x0069A980`; `0x005E7BF0` copies to `DAT_00A8BAE2` | Yes |
| record `+0x17B` | explicit terminator for digest | `0x0069AD80`, `0x0069A980` set byte `+0x17B = 0` after `_strncpy(..., 0x20)` | Yes |
| record `+0x17C` | `[Basic] Official` bool / official flag | `0x006994F0` reads `[Basic] Official`; `0x0069A3B0` default sets it to `1`; `0x005E7BF0` copies to `DAT_00A8BB08` | Yes, with optional official-filter gate in sibling report |
| record `+0x180` | `[Basic] MinPlayers`, default `2` | `0x0069A3B0` default; `0x006994F0` reads `[Basic] MinPlayers`; `0x0069A980` stores param value | Yes |
| record `+0x184` | `[Basic] MaxPlayers`, default `4` | `0x0069A3B0` default; `0x006994F0` reads `[Basic] MaxPlayers`; `0x0069A980` stores param value | Yes |
| record `+0x188..+0x19C` | CD/source availability integer list | constructor setup in `0x0069A3B0`/`0x0069A980`; checked by `0x0069AC30` | Conditional - active when selected file open fails and CD/source fallback is needed |
| record `+0x1A4..+0x1B8` | map `GameModes` list | constructor setup and population in `0x0069A3B0`/`0x0069A980`; matched by `0x0069AE10` in sibling report | Yes for modal filtering |
| `DAT_00A8B322` | selected map display-name/global label buffer | `0x005E7BF0` copies record `+0x00`; `0x005E2F60` sends it to control `0x5A8` | Yes |
| `DAT_00A8B8E0` | selected map file/path global | `0x005E7BF0` copies record `+0x58`; preview docs show normal preview loader opens this path | Yes |
| `ScenarioClass+0x125C` | selected map path mirrored into scenario state | `0x005E7BF0` copies `DAT_00A8B8E0` into `g_ScenarioClass_Instance + 0x125C` | Yes |
| `DAT_00A8BAE2` | selected digest global | `0x005E7BF0` copies record `+0x15C` | Yes |
| `DAT_00A8BB08` | selected official flag global | `0x005E7BF0` copies record `+0x17C` | Yes |
| `DAT_00A8BB0C` | selected player-count cap/mask derived from record/session/mode | `0x005E7BF0` writes after applying session vtable `+0x98` and `g_GameMode == 4` gates | Conditional - network branches only in game mode `4`; offline Skirmish uses the base/session-clamped value |
| `DAT_00A8BB04` | selected map file vtable `+0x2C` result | `0x005E7BF0` opens `DAT_00A8B8E0` and stores vtable `+0x2C` result twice | Yes |

## 3. Core Logic

### Record construction defaults

`0x0069A3B0` constructs records produced from PKT-style sources. Active in YR: Yes, because `0x00699980` builds `MISSIONSMD.PKT`, loose `*.PKT`, and `*.YRO`/embedded PKT records for the standard scenario list.

Important defaults:

- display name at `+0x00` starts as empty, then is filled from source data or a string-table fallback. Active in YR: Yes; evidence `0x0069A3B0`.
- digest at `+0x15C` defaults to `No Digest`. Active in YR: Yes; evidence `0x0069A3B0`, `0x0069AD80`.
- official flag at `+0x17C` defaults to `1`. Active in YR: Yes; evidence `0x0069A3B0`.
- min/max players default to `2` and `4`. Active in YR: Yes; evidence `0x0069A3B0`.
- path writes into byte `+0x58`; bounded copies force a terminator at byte `+0x15B`. Active in YR: Yes; evidence `0x0069A3B0`, `0x0069A980`.

`0x0069A980` constructs direct loose map records such as loose `*.YRM`. Active in YR: Yes; evidence `0x00699980` loose `*.YRM` branch calls `0x0069A980`.

Important direct-map behavior:

- null file/path argument writes `No File Name` to `+0x58`; otherwise `_strncpy(..., 0x104)` and byte `+0x15B = 0`. Active in YR: Yes; evidence `0x0069A980`.
- null display-title argument loads string-table id `0xB1D`; otherwise copies up to `0x2C` wide chars and sets word `+0x56 = 0`. Active in YR: Yes; evidence `0x0069A980`.
- null digest writes `No Digest`; otherwise `_strncpy(..., 0x20)` and byte `+0x17B = 0`. Active in YR: Yes; evidence `0x0069A980`.
- min players, official flag, and max players are stored directly into `+0x180`, `+0x17C`, and `+0x184`. Active in YR: Yes; evidence `0x0069A980`.

### Selected-record loader `0x005E7BF0`

`0x005E7BF0(param_1)` is the loader for the selected scenario index. Active in YR: Yes; evidence `0x006ACEE0` calls it after Choose Map accept and in cancel/refresh paths.

If `param_1 == -1`, the loader clears selected-map globals and returns failure-like `0`: `DAT_00A8BB04`, `DAT_00A8BB08`, `DAT_00A8BB0C`, first word/byte of `DAT_00A8B322`, first byte of `DAT_00A8B8E0`, and first byte of `DAT_00A8BAE2`. Active in YR: Conditional - only if caller passes no selected scenario; evidence `0x005E7BF0`.

For a normal index, it:

1. Opens record `+0x58` through a `CCFileClass`-style file object. Active in YR: Yes; evidence `0x005E7BF0`.
2. If open fails, attempts source/CD fallback through `0x0069AC30` and `0x0069ACC0`; if those fail, returns `0`. Active in YR: Conditional - only for missing/inaccessible selected file; evidence `0x005E7BF0`, `0x0069AC30`, `0x0069ACC0`.
3. Copies record `+0x00` to `DAT_00A8B322`. Active in YR: Yes; evidence `0x005E7BF0` call to `0x007CA489(&DAT_00A8B322, record)`.
4. Copies record `+0x15C` to `DAT_00A8BAE2`. Active in YR: Yes; evidence `0x005E7BF0`.
5. Copies record `+0x58` to `DAT_00A8B8E0`, then mirrors `DAT_00A8B8E0` to `g_ScenarioClass_Instance + 0x125C`. Active in YR: Yes; evidence `0x005E7BF0`.
6. Copies record `+0x17C` to `DAT_00A8BB08`. Active in YR: Yes; evidence `0x005E7BF0`.
7. Derives `DAT_00A8BB0C` from a helper result, optionally clamped by current category/session vtable `+0x98`, and conditionally modified when `g_GameMode == 4`. Active in YR: Yes for base/session clamp; Conditional for `g_GameMode == 4` network overrides.
8. Opens `DAT_00A8B8E0` and stores vtable `+0x2C` result into `DAT_00A8BB04`; the same open/vtable write is repeated immediately. Active in YR: Yes; evidence `0x005E7BF0`.

The loader does not parse PreviewPack. Active in YR: Yes for separation of responsibilities; evidence `0x005E7BF0` only copies record/path globals, while preview reports verify `[PreviewPack]` decode in `0x00641B00`.

### Accept and text-control refresh

`0x005E7160` commits the selected row from listbox `0x553` by matching list item data against `DAT_00A8B8CC[i]`. Active in YR: Yes; evidence `0x005E7160`.

After resolving the selected record index, it writes:

- `DAT_00A8B23C` is set to the selected `MPModes` mode/category object, not the selected scenario/map record. Active in YR: Yes; evidence `0x005E7160` and `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`.
- `DAT_00A8B250 = selected_object[10]` when category changes. Active in YR: Yes; evidence `0x005E7160`.
- `DAT_00A8B254 = selected record index`, written in both category-change and unconditional paths. Active in YR: Yes; evidence `0x005E7160`.

The parent `0x006ACEE0` accept branch then calls `0x005E7BF0(DAT_00A8B254)`. If it returns `0`, it restores the old selected index/token and returns without text refresh. Active in YR: Conditional - only on load failure; evidence `0x006ACEE0`.

On success, the parent refreshes:

- control `0x6EC` via `0x005E2EF0`, which sends message `0x4B2` with current mode/category wide text from `0x007B7140`. Active in YR: Yes; evidence `0x005E2EF0`, `0x007B7140`.
- control `0x5A8` via `0x005E2F60`, which sends message `0x4B2` with `DAT_00A8B322`. Active in YR: Yes; evidence `0x005E2F60`.

`0x005E7160` also updates chooser/parent text controls before/around modal close by sending `0x4B2` to `0x6EC` with category text and to `0x5A8` with the selected record pointer. Active in YR: Yes; evidence `0x005E7160`. The static-control thunk that interprets `0x4B2` is resolved by `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`: it copies incoming wide text to per-control record storage before owner-draw dispatch.

## 4. INI Keys

| INI path | Record field / effect | Default in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `[Basic] MinPlayers` | record `+0x180` | constructor default `2` before read | `0x006994F0`; `0x0069A3B0` | Yes |
| `[Basic] MaxPlayers` | record `+0x184` | constructor default `4` before read | `0x006994F0`; `0x0069A3B0` | Yes |
| `[Basic] GameMode` / map mode strings | category/filter list source, not copied by selected loader | none in loader | `0x006994F0`; sibling `0x0069AE10` report | Yes |
| `[Basic] Official` | record `+0x17C`, copied to `DAT_00A8BB08` | PKT constructor default `1`; direct parser default may be supplied by caller | `0x006994F0`; `0x0069A3B0`; `0x005E7BF0` | Yes |
| `[Digest]` string | record `+0x15C`, copied to `DAT_00A8BAE2` | `No Digest` | `0x006994F0`; `0x0069AD80`; `0x005E7BF0` | Yes |

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Global list build | `0x00699980` allocates `0x1BC` records and appends to the scenario list | `0x00699980` | Yes |
| Modal selection | map list item data is a record pointer; commit scans `DAT_00A8B8CC` for that pointer | `0x005E7160`; sibling list report | Yes |
| Parent accept branch | calls selected-record loader after modal accept, then text refresh helpers | `0x006ACEE0` | Yes |
| Parent failure branch | if loader returns `0`, restores old `DAT_00A8B254` and `DAT_00A8B250` | `0x006ACEE0` | Conditional - selected file/load failure |
| Setup map label | `0x005E2F60` sends `DAT_00A8B322` to static control `0x5A8` | `0x005E2F60` | Yes |
| Setup mode label | `0x005E2EF0` sends session/category text to static control `0x6EC` | `0x005E2EF0`, `0x007B7140` | Yes |
| Preview path consumer | normal preview loader uses `DAT_00A8B8E0`, but decode is separate | preview lifecycle reports; `0x005E7BF0` path copy | Yes |

## 6. Current Rust Implementation Status

Rust currently has a lightweight map-entry model and not the retail record/global contract.

| Area | Rust status | Evidence |
|---|---|---|
| map list source | scans loose files with extensions including `mmx`, `yro`, `map`, `mpr`, `yrm`; does not build PKT `MultiMaps` records in retail order | `src/app_list_maps.rs:21`, `src/app_list_maps.rs:40` |
| list ordering | sorts by lowercase display name | `src/app_list_maps.rs:46` |
| display name | reads `[Basic] Name` or falls back to file stem | `src/app_list_maps.rs:71` |
| selected state | stores `selected_map_idx`, not record pointer item-data identity plus selected globals | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs:35` |
| Choose Map action | currently handled without a retail modal record commit path | `src/app.rs:557`; sibling action trace |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| record `0x1BC` allocation | verified | `0x00699980` | none |
| PKT-style record constructor | verified | `0x0069A3B0`; follow-up `SKIRMISH_CHOOSE_MAP_YRO_DISPLAY_STRING_CONSTRUCTION_GHIDRA_REPORT.md` | none for title/path rules |
| direct loose-map record constructor | verified | `0x0069A980` | none for fields in this scope |
| digest setter/default | verified | `0x0069AD80`, `0x0069A980` | none |
| random-map sentinel test | verified | `0x0069ADF0` compares record `+0x58` to `RandMap.Sed` | random generation internals out of scope |
| selected-record loader success path | verified | `0x005E7BF0` | none for field copies |
| selected-record loader failure path | verified | `0x005E7BF0`, `0x0069AC30`, `0x0069ACC0` | exact user-facing error text for `DAT_0081C1D0 == -2` is not decoded |
| selected globals written by modal accept | verified | `0x005E7160` | none for selected index/token |
| parent accept/cancel refresh | verified | `0x006ACEE0` | preview object replacement out of scope |
| `0x6EC` / `0x5A8` update helpers | verified | `0x005E2EF0`, `0x005E2F60`, `0x007B7140`; thunk resolved by `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md` | none for static text copy plumbing |
| PreviewPack decode | deferred | user hard constraint | owned by preview reports |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1` - Which record field is the map display label? Record byte `+0x00`, a wide string copied by `0x007CA489`/`0x007CA422` and later copied to `DAT_00A8B322`. Evidence: `0x0069A3B0`, `0x0069A980`, `0x005E7BF0`. Active in YR: Yes.
- `[RESOLVED] OQ-2` - Which record field is the file/path? Record byte `+0x58`, ASCII, bounded by byte `+0x15B`; loader opens and copies it to `DAT_00A8B8E0`. Evidence: `0x0069A3B0`, `0x0069A980`, `0x005E7BF0`. Active in YR: Yes.
- `[RESOLVED] OQ-3` - Which field becomes the selected digest? Record byte `+0x15C`, bounded by byte `+0x17B`, copied to `DAT_00A8BAE2`. Evidence: `0x0069AD80`, `0x005E7BF0`. Active in YR: Yes.
- `[RESOLVED] OQ-4` - Which fields hold min/max players? Record `+0x180` and `+0x184`; default `2/4`, read from `[Basic] MinPlayers/MaxPlayers`. Evidence: `0x0069A3B0`, `0x006994F0`, `0x0069A980`. Active in YR: Yes.
- `[RESOLVED] OQ-5` - Does `0x005E7BF0` parse preview data? No; it copies record/path globals and leaves preview parsing to other functions. Evidence: `0x005E7BF0`; preview reports for `0x00641B00`. Active in YR: Yes.
- `[RESOLVED] OQ-6` - What happens when selected index is `-1`? Selected-map globals are cleared and the function returns `0`. Evidence: `0x005E7BF0`. Active in YR: Conditional.
- `[RESOLVED] OQ-7` - How does the parent handle selected-loader failure after accept? It restores previous `DAT_00A8B254` and `DAT_00A8B250` and returns before label refresh. Evidence: `0x006ACEE0`. Active in YR: Conditional.
- `[RESOLVED] OQ-8` - How exactly does the static subclass consume message `0x4B2` and record-pointer lParams? `0x00610CA0` copies incoming wide text into the per-control record at `+0x28` before dispatching to the owner proc; Skirmish controls `0x6EC` and `0x5A8` are active users. Evidence: `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`.
- `[RESOLVED] OQ-9` - Exact YRO display string construction in the decompiler-elided call cluster. PKT-style records use `DescriptionText` first, then translated `Description`; YRO records append ` (n)` or ` (min-max)` from min/max players; loose YRM records use `[Basic] Name` or `No Name`. Evidence: `SKIRMISH_CHOOSE_MAP_YRO_DISPLAY_STRING_CONSTRUCTION_GHIDRA_REPORT.md`.

## Sources

- Ghidra decompile: `0x005E7BF0`, `0x0069A3B0`, `0x0069A980`, `0x0069AD80`, `0x0069ADF0`, `0x0069AC30`, `0x0069ACC0`, `0x006994F0`, `0x00699980`, `0x005E7160`, `0x006ACEE0`, `0x005E2EF0`, `0x005E2F60`, `0x007B7140`, `0x007CA489`, `0x007CA422`, `0x005E6520`.
- Prior reports read: `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_RETURN_CONTRACT_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`; preview lifecycle/decode reports in the same folder for non-scope separation.
- Rust scan: `src/app_list_maps.rs`, `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`.
