# Skirmish Preview Scenario Bounds Stock-Map Population - Ghidra Research Report

**Date:** 2026-05-20  
**Address(es):** `0x00641EE0`, `0x00689D30`, `0x006ACEE0`, `0x006AE3F0`, `0x0068AD70`, `0x00687D80`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Whether offline Skirmish map preview/menu paint populates `ScenarioClass+0x112C..0x113C` and `+0x1140` marker pairs for stock maps that lack `[Header]`.  
**Non-Scope:** PreviewPack decode byte/channel order, exact preview object vtable semantics, and full map chooser UX.  
**Confidence:** High for the static offline-menu call chain; Medium for loose retail-map corpus coverage because maps inside MIX archives were not extracted in this slot.  
**Active in YR:** Yes for the offline Skirmish preview load/paint path; Conditional for header writer/generator paths, which are save/editor/random-preview generation paths rather than normal stock-menu load.

## 1. Overview

For stock Skirmish maps that lack `[Header]`, the offline Skirmish menu does not synthesize `ScenarioClass+0x112C..0x113C` or the `+0x1140` marker pairs before the preview paint. The active selected-map preview loader still calls the `[Header]` reader, but that reader initializes the bounds/count fields to `-1` and only replaces them from `[Header]` keys. With no `[Header]`, `NumberStartingPoints` remains `-1`, and the `DrawStartPositions` overlay guard fails.

The practical player-visible result is that stock maps without `[Header]` can still show baked start pixels inside the decoded preview image, but they do not get `STARTBUT.SHP` numbered overlays from `DrawStartPositions` unless some `[Header]` metadata exists or a separate, untraced external map-generation/save path created it beforehand.

## 2. Key Offsets

| Offset | Meaning in this slice | Population for no-`[Header]` menu load | Evidence | Active in YR |
|---|---|---:|---|---|
| `ScenarioClass+0x112C` | preview/source `StartX` | `-1` | `FUN_00689D30` initialization/read | Yes, menu preview load calls it |
| `ScenarioClass+0x1130` | preview/source `StartY` | `-1` | `FUN_00689D30` | Yes |
| `ScenarioClass+0x1134` | preview/source `Width` | `-1` | `FUN_00689D30` | Yes |
| `ScenarioClass+0x1138` | preview/source `Height` | `-1` | `FUN_00689D30` | Yes |
| `ScenarioClass+0x113C` | overlay start count | `-1` | `FUN_00689D30`; consumer guard at `DrawStartPositions @ 0x00640710` | Yes |
| `ScenarioClass+0x1140 + i*8` | overlay marker X | zeroed before reads | `FUN_00689D30` | Yes |
| `ScenarioClass+0x1144 + i*8` | overlay marker Y | zeroed before reads | `FUN_00689D30` | Yes |

## 3. Core Logic

### Active offline menu load path

`FUN_006ACEE0` is the Skirmish dialog command handler. In the map-selection paths, it destroys any existing `DAT_00AC1154` preview object, creates a new one with `FUN_006406E0`, loads the selected map into it, and invalidates the window. `FUN_006AE3F0` is the Skirmish window proc/paint path: on `WM_PAINT` (`0x0F`), if `DAT_00AC1154 != 0`, it reaches `DrawStartPositions @ 0x00640710`.

**Active in YR:** Yes. Evidence: `FUN_006AE3F0` handles the Skirmish dialog paint path and calls `DrawStartPositions`; `FUN_006ACEE0` handles Skirmish command messages and refreshes `DAT_00AC1154`.

### Selected-map preview loader

`0x00641EE0` is the load helper reached from Skirmish/network map-preview paths. Its material sequence is:

1. open/read selected map file;
2. construct a temporary `CCINIClass`;
3. parse the early map preview/header portion through a `SHAPipe`;
4. call `FUN_00689D30` with `ECX = g_ScenarioClass_Instance` and `EDX = temporary INI`;
5. read chunks until it finds the literal string `[Map]`;
6. truncate at `[Map]`;
7. parse the pre-`[Map]` data into the preview object.

**Active in YR:** Yes. Evidence: xrefs to `0x00641EE0` from active map-preview callers at `0x005535A7`, `0x005B8BEB`, and `0x005E78CB`; `0x006ACEE0` and Skirmish init flow create `DAT_00AC1154` and call into the same preview-load family.

### Header reader behavior

`FUN_00689D30` is the only verified reader that populates these exact fields in this slice. It first writes `-1` to `+0x112C`, `+0x1130`, `+0x1134`, `+0x1138`, and `+0x113C`; then it zeroes eight marker pairs starting at `+0x1140`; then it reads:

| INI key | Destination |
|---|---|
| `[Header] StartX` | `+0x112C` |
| `[Header] StartY` | `+0x1130` |
| `[Header] Width` | `+0x1134` |
| `[Header] Height` | `+0x1138` |
| `[Header] NumberStartingPoints` | `+0x113C` |
| `[Header] Waypoint%d` | `+0x1140` pairs, loop index starts at 1 |

If `[Header]` is absent, the default values survive because the `ReadInt` defaults are the just-written `-1` values, and the waypoint loop is skipped because `0 < NumberStartingPoints` is false.

**Active in YR:** Yes. Evidence: direct call from `0x00641F72` inside selected-map preview load, plus full scenario init call at `0x006874F8`.

### Consumer consequence

`DrawStartPositions @ 0x00640710` reads `ScenarioClass+0x113C` and draws overlays only for `0 < count < 9`. With no `[Header]`, the active menu load leaves count `-1`, so the overlay branch is not entered. The preview surface itself may still contain baked start pixels from `[PreviewPack]`; that is separate from the `STARTBUT.SHP` overlay branch.

**Active in YR:** Yes. Evidence: `FUN_006AE3F0` paint path calls `DrawStartPositions`; count guard verified at `0x00640710`.

## 4. Header Generator Is Not The Stock Menu Writer

`FUN_0068AD70` writes/generates `[Header]` metadata from playable-cell projected bounds and gameplay waypoints. Its only verified caller in this slice is the scenario/map write function around `0x00687D80`, which then writes map sections and preview data. This is a save/editor/generated-map output path, not a normal stock-map menu load path.

**Active in YR:** Conditional. Evidence: xref to `FUN_0068AD70` from `0x00687DCE`; decompile at `0x00687D80` shows map/INI write sequence, `g_IsMapEditor` handling, preview generation/write, and `Write_Map_Section_And_IsoMapPack5`. It is active when maps are written/generated, not when a stock loose map without `[Header]` is merely selected in the offline Skirmish menu.

## 5. Stock Map Data Check

Loose retail sample `Dustbowl.map` has `[Preview]`, `[PreviewPack]`, `[Map] Size`, `[Map] LocalSize`, and `[Waypoints]`, but no `[Header]`:

- `C:/Users/enok/Documents/Command and Conquer Red Alert II/Dustbowl.map:251` `[Preview]`
- `.../Dustbowl.map:252` `Size=0,0,138,75`
- `.../Dustbowl.map:254` `[PreviewPack]`
- `.../Dustbowl.map:531` `[Map]`
- `.../Dustbowl.map:533` `Size=0,0,70,76`
- `.../Dustbowl.map:534` `LocalSize=2,8,65,62`
- `.../Dustbowl.map:536` `[Waypoints]`

Loose map-like file scan found 54 `.map/.mpr/.yrm/.mmx/.yro` files and 9 `[Header]` matches, all in `.yro` files; this supports the specific target premise that many stock multiplayer maps lack `[Header]`, but it does not prove the contents of maps packed inside MIX archives.

**Active in YR:** Yes as local retail data consumed by the active menu path; corpus coverage is partial.

## 6. Current Rust Implementation Status

Rust currently keeps `preview_source_bounds` empty in `src/app_list_maps.rs` because `[Map] LocalSize` had not been proven as the menu-preview source. This investigation supports keeping that guard: for stock maps without `[Header]`, gamemd does not populate the overlay fields from `[Map] LocalSize` during offline menu preview load. Rust should not enable `STARTBUT.SHP` overlays by substituting `LocalSize` for `ScenarioClass+0x112C..0x1138`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Skirmish paint consumer | verified | `FUN_006AE3F0`, `DrawStartPositions @ 0x00640710` | none for this slice |
| Selected-map preview loader | verified | `0x00641EE0`; assembly context at `0x00641F72` | concrete symbol name still mislabeled in Ghidra |
| Header reader defaults/missing-section behavior | verified | `FUN_00689D30` | none |
| Full scenario init header read | verified | xref `0x006874F8` to `FUN_00689D30` | outside offline menu after game start |
| Header generator/writer | verified as non-menu-load path | `FUN_0068AD70`; caller `0x00687D80` | exact editor UI trigger out of scope |
| Direct `[Map] LocalSize -> +0x112C..+0x113C` menu writer | verified absent in traced offline menu path | `0x00641EE0`, `0x006ACEE0`, xref set to `FUN_00689D30`/`FUN_0068AD70` | program-wide raw offset scan not available through current MCP |
| Loose retail map header presence | touched-not-exhausted | PowerShell/rg scan of loose files | MIX-contained map corpus not extracted |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does the active offline menu loader call a runtime stock-map generator for `+0x112C..+0x113C` when `[Header]` is missing? No. The active loader calls `FUN_00689D30`; if `[Header]` is absent, defaults remain `-1` and zeroed pairs. Evidence: `0x00641EE0`, `0x00641F72`, `FUN_00689D30`.

[RESOLVED] OQ-2 - Is `FUN_0068AD70` the missing stock-menu writer? No for normal stock selection. It is reached from a map/INI write path, not the menu preview load path. Evidence: xref `0x00687DCE`, caller decompile `0x00687D80`.

[RESOLVED] OQ-3 - What happens to overlays when `NumberStartingPoints` remains `-1`? They do not draw because `DrawStartPositions` requires `0 < count < 9`. Evidence: `0x00640710`.

[DEFERRED] OQ-4 - Do MIX-contained maps have `[Header]` metadata at different rates than loose maps? Category: out-of-scope. This slot did not extract MIX archives.

## Sources

- Ghidra: `0x00641EE0`, `0x00641F72`, `0x00689D30`, `0x006ACEE0`, `0x006AE3F0`, `0x00640710`, `0x0068AD70`, `0x00687D80`, `0x006874F8`.
- Retail data: `C:/Users/enok/Documents/Command and Conquer Red Alert II/Dustbowl.map`.
- Parent report: `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_MAP_PREVIEW_SOURCE_BOUNDS_AND_PREVIEWPACK_GHIDRA_REPORT.md`.
