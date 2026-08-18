# Skirmish Choose Map YRO Display String Construction - Ghidra Research Report

**Address(es):** `0x00699980`, `0x0069A3B0`, `0x0069A980`, `0x006994F0`, `0x00529160`, `0x005E7BF0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The construction path that writes the scenario-record wide display string at record `+0x00` for PKT-style records, YRO-derived records, and loose custom YRM records consumed by Skirmish Choose Map.  
**Non-Scope:** modal list ordering, selected-record loader copies, preview rendering, PreviewPack decode, map file fallback/CD lookup, and full string-table internals outside the one description helper used here.  
**Confidence:** High for source branches, string keys/literals, suffix formatting, field offsets, truncation bounds, and YR activity; Medium for the exact human text produced by external CSF/string-table keys because that depends on loaded language resources beyond this binary slice.  
**Active in YR:** Yes. Evidence: the standard Skirmish scenario-list builder at `0x00699980` opens YR-specific `MISSIONSMD.PKT`, enumerates `*.PKT`, `*.YRO`, and `*.YRM`, appends `0x1BC` records to the list consumed by Choose Map, and sibling reports verify the live dialog path reads record `+0x00`.

## 1. Result

The display string that appears in Choose Map is not built in the modal. It is written into each scenario record at byte offset `+0x00` during global scenario-list construction.

For PKT-style entries from `MISSIONSMD.PKT`, loose `*.PKT`, and the embedded PKT inside `*.YRO`, constructor `0x0069A3B0` builds the base title:

- If the map entry section has `DescriptionText`, that value is read as text and converted to the record wide string at `+0x00`. Active in YR: Yes; evidence `0x0069A4DE..0x0069A52A`, key string `0x0083F648 = "DescriptionText"`.
- Otherwise it calls helper `0x00529160` with key `Description`; that helper resolves the value through the string-table load path and copies the result to the caller-supplied wide buffer. Active in YR: Yes; evidence `0x0069A531..0x0069A53A`, key string `0x0081B1A4 = "Description"`, and `0x00529310..0x0052932D`.
- The record path at `+0x58` for these entries is the MultiMaps entry value with `.MAP` appended. Active in YR: Yes; evidence `0x0069A46A..0x0069A4BF`, suffix string `0x0082DF18 = ".MAP"`.

For YRO-derived records, the base title above is immediately rewritten with a player-count suffix before append to the global list:

- If `MinPlayers == MaxPlayers`, it formats `(%d)` using wide format `0x0083F4CC = L"%c%d%c"` with characters `0x28` and `0x29`, so the suffix is `(n)`. Active in YR: Yes; evidence `0x00699E6B..0x00699EAD`.
- If `MinPlayers != MaxPlayers`, it formats `(%d-%d)` using wide format `0x0083F4DC = L"%c%d-%d%c"`, so the suffix is `(min-max)`. Active in YR: Yes; evidence `0x00699E6B..0x00699E93`.
- It copies the current record title to a stack wide buffer, appends one wide space from `0x0082083C = L" "`, appends the suffix, then copies the result back to record `+0x00` with a `0x2C` wide-slot bound and forces `word +0x56 = 0`. Active in YR: Yes; evidence `0x00699EB0..0x00699F1F`.

For loose custom `*.YRM` records, constructor `0x0069A980` writes the display string from the loose map's `[Basic] Name` path:

- The `*.YRM` branch reads `[Basic] Name` with default `No Name`, converts it to a wide string, and passes that pointer to `0x0069A980`. Active in YR: Yes; evidence `0x0069A056..0x0069A0AF`, strings `0x0082BF9C = "Basic"`, `0x00817854 = "Name"`, `0x00829284 = "No Name"`.
- `0x0069A980` copies that non-null title pointer into record `+0x00` with a `0x2C` wide-slot bound and forces `word +0x56 = 0`. Active in YR: Yes; evidence `0x0069A980` decompile and call setup `0x0069A112..0x0069A13C`.
- The constructor has a fallback for null title pointers: load string-table id `0xB1D`. Active in YR: Conditional; evidence `0x0069A980`, but the live `*.YRM` branch passes the converted `[Basic] Name` pointer, not null.

## 2. Source Branches

### `MISSIONSMD.PKT`

`0x00699980` opens `MISSIONSMD.PKT` and section `MultiMaps`, iterates entries by index, reads each entry value into a `0x40` byte stack buffer, allocates a `0x1BC` record, and calls `0x0069A3B0`. Active in YR: Yes; evidence `0x006999D9` string `MISSIONSMD.PKT`, `0x00699A08` string `MultiMaps`, constructor call `0x00699A68..0x00699A77`.

The MultiMaps entry value is not itself the visible title. It becomes the map-file stem used by `0x0069A3B0` to make record `+0x58 = entry + ".MAP"`. The visible title comes from that map section's `DescriptionText` or `Description`. Active in YR: Yes; evidence `0x0069A46A..0x0069A53A`.

### Loose `*.PKT`

The loose PKT branch repeats the same MultiMaps loop and calls the same `0x0069A3B0` constructor. Active in YR: Yes; evidence `0x00699AE1` pattern `*.PKT`, constructor call `0x00699BB9..0x00699BC8`.

Therefore loose PKT records use the same visible-title rules as `MISSIONSMD.PKT`: `DescriptionText` first, then translated `Description`; path field receives entry plus `.MAP`. Active in YR: Yes; evidence same constructor `0x0069A3B0`.

### YRO-derived records

The YRO branch enumerates `*.YRO`, checks/opens `MISSIONS.YRO`, constructs a matching embedded PKT name by replacing the `.YRO` extension with `PKT`, and opens that PKT data. Active in YR: Yes; evidence `0x00699C58` pattern `*.YRO`, `0x00699CB8` string `MISSIONS.YRO`, `0x00699D4C..0x00699DA8` suffix string `PKT`, and failure log `0x0083F498 = "Can't see .pkt inside .yro!"`.

Each embedded PKT MultiMaps entry is first constructed by `0x0069A3B0`, so its base record title is still `DescriptionText` or translated `Description`. Active in YR: Yes; evidence constructor call `0x00699E4D..0x00699E5C`.

Only after that constructor returns does the YRO branch rewrite the title with the player-count suffix. Active in YR: Yes; evidence the rewrite starts after non-null record check at `0x00699E63..0x00699E6B`.

### Loose `*.YRM` custom maps

The loose YRM branch enumerates only `*.YRM` in this slice, not `*.MMX`, `*.MPR`, or arbitrary `*.MAP`. Active in YR: Yes for `*.YRM`; evidence `0x0069A002` string `*.YRM`.

It reads the loose file's `[Basic] Name`, `[Digest] 1`, `[Basic] Official`, and first/last-file fragment helper outputs, then calls `0x0069A980`. Active in YR: Yes; evidence `0x0069A056..0x0069A13C`.

The visible title is the converted `[Basic] Name` value, defaulting to `No Name`. No YRO player-count suffix is applied to this loose YRM branch. Active in YR: Yes; evidence `0x0069A056..0x0069A13C` has no suffix-format block before the `0x0069A980` call, and the suffix-format block exists only in the `*.YRO` branch at `0x00699E6B..0x00699F1F`.

## 3. Bounds And Edge Details

- Record `+0x00` is a wide string field with a copy bound of `0x2C` wide slots in the relevant constructors/rewrite path. Active in YR: Yes; evidence `0x0069A505`, `0x00699F13`, `0x0069A980`.
- The explicit terminator write is `word [record + 0x56] = 0`, which corresponds to the last wide slot in the `0x58` byte title area. Active in YR: Yes; evidence `0x00699F1F` and `0x0069A980`.
- The YRO suffix append uses one explicit wide space before the parenthesized count. Active in YR: Yes; evidence `0x00699ED5` passes string `0x0082083C = L" "`.
- The YRO single-count suffix is used only when record `+0x180 == +0x184`; otherwise a range suffix is used. Active in YR: Yes; evidence compare at `0x00699E6B..0x00699E7B`.
- The dead-looking fallback at `0x00699F25` loads `MSG:NoDescription` via string-table id `0xB1D`, but the immediately preceding `LEA EDX,[ESP+...]` makes the tested pointer non-null in this path. Active in YR: No for normal YRO title rewrite; evidence `0x00699EFE..0x00699F25`. The same string-table id remains conditionally active in `0x0069A980` if passed a null title pointer.

## 4. Relationship To Prior Reports

This resolves the deferred `OQ-9` in `SKIRMISH_CHOOSE_MAP_RECORD_DECODE_FIELDS_GHIDRA_REPORT.md`. That report correctly identified record `+0x00` as the displayed title, but left the exact YRO construction source at medium confidence.

The refined contract is:

- PKT-style base title: map section `DescriptionText`, else translated `Description`.
- YRO final title: PKT-style base title plus space plus `(n)` or `(min-max)`, bounded back into record `+0x00`.
- Loose custom YRM title: `[Basic] Name`, default `No Name`, with no YRO suffix.

No contradiction was found with the selected-record loader: `0x005E7BF0` still copies final record `+0x00` to `DAT_00A8B322`. Active in YR: Yes; evidence parent report and loader `0x005E7BF0`.

## 5. Open Questions

- The exact localized player-facing text for a `Description` key depends on the loaded string table/CSF resources. The binary path to resolve it is verified, but this slot did not dump language-resource contents.
- The exact order of loose filesystem enumeration remains owned by the list-population report and is not part of display-string construction.
- Non-`*.YRM` loose custom-map extensions are not active in this `0x00699980` branch; if another YR path imports them, that is outside this slot.

## Sources

- Ghidra disassembly/decompile: `0x00699980`, `0x0069A3B0`, `0x0069A980`, `0x006994F0`, `0x00529160`, `0x005E7BF0`.
- Binary string evidence from `<ra2-install>/gamemd.exe`: `0x0083F520 = "MISSIONSMD.PKT"`, `0x0083F514 = "MultiMaps"`, `0x0083F50C = "*.PKT"`, `0x0083F504 = "*.YRO"`, `0x0083F4F4 = "MISSIONS.YRO"`, `0x0083F4F0 = "PKT"`, `0x0082DF18 = ".MAP"`, `0x0083F648 = "DescriptionText"`, `0x0081B1A4 = "Description"`, `0x0083F4CC = L"%c%d%c"`, `0x0083F4DC = L"%c%d-%d%c"`, `0x0082083C = L" "`, `0x0083F490 = "*.YRM"`, `0x00817854 = "Name"`, `0x00829284 = "No Name"`.
- Prior docs read: `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_RECORD_DECODE_FIELDS_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_RETURN_CONTRACT_GHIDRA_REPORT.md`.
