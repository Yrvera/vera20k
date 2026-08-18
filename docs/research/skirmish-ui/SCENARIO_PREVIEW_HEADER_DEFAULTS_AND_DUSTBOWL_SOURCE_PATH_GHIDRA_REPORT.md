# Scenario Preview Header Defaults And Dustbowl Source Path - Ghidra Research Report

**Date:** 2026-05-21  
**Address(es):** `0x00689D30`, `0x00689E90`, `0x00641EE0`, `0x00640710`, `0x006874F8`, `0x00687853`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Resolve the active default chain for `ScenarioClass+0x112C..0x113C` when `[Header]` preview keys are absent, and resolve the concrete loose `Dustbowl.map` start-marker source path in the offline Skirmish preview.  
**Non-Scope:** Full MIX archive map census, random-map preview generation beyond its known separation, editor save UI, and full Skirmish dialog UX.  
**Confidence:** High for the active binary paths and loose retail Dustbowl data checked here. Medium for broader stock-map corpus conclusions because MIX-contained variants were not extracted in this pass.  
**Active in YR:** Yes for offline Skirmish selected-map preview and standard scenario init. Conditional for generated/saved map header writer paths.

## 1. Overview

The apparent doc conflict comes from two different functions. `ScenarioClass__Read_INI_Basic @ 0x00689E90` does preserve current `ScenarioClass+0x112C..0x113C` values when `[Header]` keys are missing, but the active selected-map preview helper `FUN_00689D30` resets those same fields to `-1` immediately before reading `[Header]`. The active full scenario init path also calls `FUN_00689D30` before the later `Read_INI_Basic` call, so missing keys preserve the just-reset `-1` values rather than arbitrary old values.

For the loose retail `Dustbowl.map`, there is no `[Header]` section. Its `[PreviewPack]` can contain baked red start pixels, but `DrawStartPositions @ 0x00640710` does not draw `STARTBUT.SHP` overlays for this loose-map path because `ScenarioClass+0x113C` remains `-1` and the overlay guard requires `1..8`.

## 2. Class Layout / Key Offsets

| Offset | Type | Purpose in this slice | Missing-`[Header]` value on active preview helper |
|---|---:|---|---:|
| `ScenarioClass+0x112C` | `i32` | preview source `StartX` | `-1` |
| `ScenarioClass+0x1130` | `i32` | preview source `StartY` | `-1` |
| `ScenarioClass+0x1134` | `i32` | preview source `Width` divisor | `-1` |
| `ScenarioClass+0x1138` | `i32` | preview source `Height` divisor | `-1` |
| `ScenarioClass+0x113C` | `i32` | `NumberStartingPoints` overlay count | `-1` |
| `ScenarioClass+0x1140 + i*8` | `i32` | `[Header] Waypoint%d` X | zeroed before reads |
| `ScenarioClass+0x1144 + i*8` | `i32` | `[Header] Waypoint%d` Y | zeroed before reads |

Tiny but load-bearing details:

- `FUN_00689D30` copies `ECX` into `ESI` at `0x00689D3B`; the destination object is the `ScenarioClass` pointer, not the temporary INI object.
- `FUN_00689D30` sets `EAX = 0xFFFFFFFF` at `0x00689D33` and writes that same value to all five scalar preview fields at `0x00689D46..0x00689D5E`.
- It zeroes exactly eight coordinate pairs starting at `+0x1140` before reading any `[Header] Waypoint%d` keys.
- The waypoint-read loop is gated only by `0 < NumberStartingPoints`; this parser does not apply the later draw-time `< 9` guard.
- The formatted keys are `Waypoint1`, `Waypoint2`, etc.; the counter is incremented before formatting. These are `[Header]` keys, not `[Waypoints] 0=...` gameplay entries.

## 3. Core Logic

### Active selected-map preview helper: `FUN_00689D30`

Pseudocode for the verified part:

```text
scenario.StartX = -1
scenario.StartY = -1
scenario.Width = -1
scenario.Height = -1
scenario.NumberStartingPoints = -1
for i in 0..8:
    scenario.HeaderWaypoint[i] = (0, 0)

scenario.StartX = ReadInt("Header", "StartX", scenario.StartX)
scenario.StartY = ReadInt("Header", "StartY", scenario.StartY)
scenario.Width = ReadInt("Header", "Width", scenario.Width)
scenario.Height = ReadInt("Header", "Height", scenario.Height)
scenario.NumberStartingPoints =
    ReadInt("Header", "NumberStartingPoints", scenario.NumberStartingPoints)

if scenario.NumberStartingPoints > 0:
    for n in 1..=scenario.NumberStartingPoints:
        scenario.HeaderWaypoint[n - 1] =
            ReadMinMax("Header", format("Waypoint%d", n), current_pair)
```

Evidence:

- Reset and destination: `0x00689D33..0x00689D5E`, assembly shows `MOV ESI,ECX`, `LEA EDI,[ESI+0x1140]`, then `MOV [ESI+0x112C..0x113C],EAX`.
- String addresses: `Header @ 0x0083DE68`, `StartX @ 0x0083DE70`, `StartY @ 0x0083DE60`, `NumberStartingPoints @ 0x0083DE48`.
- Count gate: `0x00689F64..0x00689F70` tests `+0x113C` and skips the waypoint loop when it is `<= 0`.
- Active selected-map call: `0x00641F64` loads `ECX = [0x00A8B230]`, `0x00641F6A` loads `EDX` with the temporary INI, `0x00641F71` pushes the INI, and `0x00641F72` calls `0x00689D30`.

### `ScenarioClass__Read_INI_Basic @ 0x00689E90`

This function does not reset the preview fields. It reads the same `[Header]` scalar keys using the current field values as defaults:

- `0x00689EB0..0x00689EC8`: current `+0x112C` is pushed as default for `[Header] StartX`.
- `0x00689F28..0x00689F40`: current `+0x113C` is pushed as default for `[Header] NumberStartingPoints`.
- `0x00689F64..0x00689F70`: the waypoint loop only runs when the resulting count is positive.

This means the narrow statement "`Read_INI_Basic` preserves current values" is true in isolation. The implementation-relevant statement for active menu/full-init paths is different: those paths reset first through `FUN_00689D30`, so a missing `[Header]` preserves `-1`.

### Full scenario init ordering

`ScenarioClass__Full_Init` also runs the reset helper before the later full basic parser:

- `0x006874E7`: loads `ECX = [0x00A8B230]`.
- `0x006874F8`: calls `0x00689D30`.
- `0x00687502..0x00687528`: reads `[Map] LocalSize` and passes it toward radar/map bounds handling, after the preview reset/read helper.
- `0x0068784C`: reloads `ECX = [0x00A8B230]`.
- `0x00687853`: calls `ScenarioClass__Read_INI_Basic @ 0x00689E90`.

Therefore full init does not provide a `[Map] LocalSize -> ScenarioClass+0x112C..0x113C` fallback. `LocalSize` is adjacent in the sequence, but it feeds the radar/map-bounds path, not the preview source fields.

### `DrawStartPositions @ 0x00640710`

The overlay consumer reads `ScenarioClass+0x113C` and draws markers only when `0 < count < 9`. If count is `-1`, zero, or `>= 9`, the `STARTBUT.SHP` loop is skipped.

When it does draw, it projects each `[Header] Waypoint%d` pair by subtracting `+0x112C/+0x1130`, dividing by `+0x1134/+0x1138`, scaling into the fitted preview rectangle, then applying `STARTBUT.SHP` offsets `-9,-6`.

## 4. INI Keys

| Section | Key | Read by | Effect | Missing-key behavior on active selected preview |
|---|---|---|---|---|
| `[Header]` | `StartX` | `0x00689D30`, `0x00689E90` | preview source X origin | `-1` after `0x00689D30` reset |
| `[Header]` | `StartY` | `0x00689D30`, `0x00689E90` | preview source Y origin | `-1` |
| `[Header]` | `Width` | `0x00689D30`, `0x00689E90` | preview source width divisor | `-1` |
| `[Header]` | `Height` | `0x00689D30`, `0x00689E90` | preview source height divisor | `-1` |
| `[Header]` | `NumberStartingPoints` | `0x00689D30`, `0x00689E90` | overlay count gate | `-1`; no overlays |
| `[Header]` | `Waypoint%d` | `0x00689D30`, `0x00689E90` | overlay coordinates | loop skipped when count is `-1` |
| `[Map]` | `LocalSize` | full init map/radar path | radar/map bounds | no verified write to preview fields |
| `[Waypoints]` | `0=`, `1=`, etc. | gameplay start-waypoint paths | gameplay starts / generated preview baking | not consumed by `DrawStartPositions` overlay projection in this loose-map path |
| `[Preview]` | `Size` | preview surface loader | preview image dimensions | independent of overlay metadata |
| `[PreviewPack]` | numbered chunks | preview image loader | compressed preview pixels | can already include baked red start pixels |

## 5. Dustbowl Source Path

The local loose retail file exists at:

`C:/Users/enok/Documents/Command and Conquer Red Alert II/Dustbowl.map`

Relevant data found with `rg -a`:

- `Dustbowl.map:2` lists `1=Dustbowl` in `[MultiMaps]`.
- `Dustbowl.map:251..254` contains `[Preview]`, `Size=0,0,138,75`, and `[PreviewPack]`.
- `Dustbowl.map:531..534` contains `[Map]`, `Size=0,0,70,76`, and `LocalSize=2,8,65,62`.
- `Dustbowl.map:536..539` contains `[Waypoints]`, including `0=116070`, `1=34079`, `8=97093`.
- No `[Header]` line or `NumberStartingPoints=` line was found in this loose file.

This matters because the selected-map preview loader `0x00641EE0` reads the preview/header portion before `[Map]`, calls `FUN_00689D30`, and later parses the preview surface from `[Preview]` / `[PreviewPack]`. The gameplay `[Waypoints]` section is after `[Map]`, and in any case is not the `[Header] Waypoint%d` source used by `DrawStartPositions`.

Resolved player-visible behavior for the loose Dustbowl preview:

- The preview image can show baked red `4x4` start pixels from `[PreviewPack]`.
- `STARTBUT.SHP` numbered overlays are not drawn by `DrawStartPositions` for this path because `NumberStartingPoints` remains `-1`.
- The earlier trace claim that standard loose Dustbowl draws live `STARTBUT.SHP` overlays is stale for this path. That claim remains possible only for a different source variant containing `[Header]`, or for generated/cached paths that explicitly populate the preview fields.

## 6. Integration Points

| Function / path | Role | Status |
|---|---|---|
| `0x00641EE0` | selected-map preview load helper; builds temp INI and calls `FUN_00689D30` with `g_ScenarioClass_Instance` | verified active in Skirmish preview load family |
| `0x00689D30` | reset-then-read `[Header]` preview metadata helper | verified |
| `0x00689E90` | full basic scenario parser; preserve-current `[Header]` reads | verified, but not sufficient alone for active defaults |
| `0x006874F8` | full init call to reset/read helper before LocalSize handling | verified |
| `0x00687853` | later full init call to `Read_INI_Basic` | verified |
| `0x00640710` | preview blit plus optional `STARTBUT.SHP` overlays and labels | verified |
| `0x0068AD70` | generated/saved map `[Header]` writer from map bounds/waypoints | touched; conditional non-menu-load path |

## 7. Current Rust Implementation Status

Current Rust is conservative in the right direction for this resolved slice:

- `src/app_list_maps.rs` still leaves `preview_source_bounds` as `None`; this matches the verified "do not synthesize from `[Map] LocalSize`" rule for missing `[Header]`.
- `src/map/preview.rs` parses `[Preview] Size=0,0,w,h` as rectangle dimensions and detects `[PreviewPack]`.
- `src/app_skirmish_shell_render.rs` still skips `STARTBUT.SHP` marker sprites unless a real preview surface and verified source bounds are available.

Implementation implication: for stock loose maps like Dustbowl, rendering the decoded `[PreviewPack]` is enough to show baked red preview markers; do not add `STARTBUT.SHP` overlays from `[Waypoints]` or `[Map] LocalSize` unless `[Header]` or a verified generated/cache path supplies `ScenarioClass+0x112C..0x113C` semantics.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00689D30` reset behavior | verified | decompile and assembly at `0x00689D33..0x00689D5E` | none |
| `FUN_00689D30` selected-preview call binding | verified | `0x00641F64..0x00641F72` assembly | none |
| `Read_INI_Basic @ 0x00689E90` preserve-current reads | verified | decompile and assembly at `0x00689EB0`, `0x00689F28..0x00689F40` | none |
| Full init ordering: reset helper before `Read_INI_Basic` | verified | `0x006874F8`, `0x00687853` | none for this slice |
| `[Map] LocalSize` fallback to preview fields | verified absent in traced active paths | `0x00687502..0x00687528`; prior reports; no write to `+0x112C..0x113C` | program-wide raw offset census not redone here |
| Loose `Dustbowl.map` `[Header]` presence | verified absent | `rg -a` data check | MIX-contained variants not extracted |
| Loose Dustbowl baked markers | verified by prior report, incorporated | `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md` | broader map census |
| Loose Dustbowl `STARTBUT.SHP` overlay | verified no for this path | no `[Header]`; `+0x113C = -1`; `DrawStartPositions` guard | runtime screenshot not captured in this pass |
| Generated/saved map header writer `0x0068AD70` | touched-not-exhausted | decompile shows it writes `[Header]` from playfield/waypoints | editor/save trigger details |

## 9. Open Questions - Final State

[RESOLVED] OQ-1 - Does missing `[Header]` preserve arbitrary prior/current values on the active selected-map preview path? No. `Read_INI_Basic` preserves current values in isolation, but active selected-map preview first calls `FUN_00689D30`, which resets the fields to `-1`. Evidence: `0x00641F64..0x00641F72`, `0x00689D33..0x00689D5E`.

[RESOLVED] OQ-2 - Does full scenario init also reset before the later `Read_INI_Basic` preserve-current reads? Yes. `0x006874F8` calls `FUN_00689D30`; `0x00687853` later calls `0x00689E90`. Evidence: assembly contexts for both calls.

[RESOLVED] OQ-3 - Does `[Map] LocalSize` backfill preview source bounds/count? No verified active write exists in the traced paths. In full init, `LocalSize` is read after `FUN_00689D30` and before `Read_INI_Basic`, but the traced consumer is radar/map bounds, not `ScenarioClass+0x112C..0x113C`. Evidence: `0x00687502..0x00687528` and prior LocalSize reports.

[RESOLVED] OQ-4 - Does loose retail `Dustbowl.map` contain `[Header] NumberStartingPoints` or `[Header] Waypoint%d` overlay metadata? No. The loose file has `[Preview]`, `[PreviewPack]`, `[Map]`, and `[Waypoints]`, but no `[Header]` or `NumberStartingPoints=` line. Evidence: `rg -a` on `C:/Users/enok/Documents/Command and Conquer Red Alert II/Dustbowl.map`.

[RESOLVED] OQ-5 - Should loose Dustbowl show `STARTBUT.SHP` overlays in the offline Skirmish preview? No for the verified loose-map path. The preview can contain baked red pixels, but live `STARTBUT.SHP` overlays require `0 < ScenarioClass+0x113C < 9`, and the missing-header path leaves `+0x113C = -1`. Evidence: `FUN_00689D30`, `DrawStartPositions @ 0x00640710`, and Dustbowl data.

[DEFERRED] OQ-6 - Do MIX-contained or patched Dustbowl variants include `[Header]` metadata and therefore draw overlays? Category: out-of-scope. This pass checked the local loose retail file and active binary path, not every possible archive variant.

## Sources

- Ghidra decompilation/assembly: `0x00689D30`, `0x00689E90`, `0x00641EE0`, `0x00640710`, `0x006874F8`, `0x00687853`.
- Ghidra memory strings: `Header @ 0x0083DE68`, `StartX @ 0x0083DE70`, `StartY @ 0x0083DE60`, `NumberStartingPoints @ 0x0083DE48`.
- Retail data: `C:/Users/enok/Documents/Command and Conquer Red Alert II/Dustbowl.map`.
- Prior docs reconciled:
  - `SKIRMISH_PREVIEW_SCENARIO_BOUNDS_STOCK_MAP_POPULATION_GHIDRA_REPORT.md`
  - `SCENARIO_PREVIEW_BOUNDS_STOCK_MAP_POPULATION_GHIDRA_REPORT.md`
  - `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md`
  - `traces/SKIRMISH_MAP_PREVIEW_START_MARKERS_TRACE.md`

