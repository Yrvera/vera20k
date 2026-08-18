# House Color Scheme To Radar Dot Packed Color - Ghidra Research Report

**Date:** 2026-05-27  
**Slot:** `/re-swarm` radar-sidebar-producer-pixel-followup slot 4  
**Target:** `HOUSE_COLORSCHEME_TO_RADAR_DOT_PACKED_COLOR`  
**Investigation Mode:** exhaustive-slice for the in-game radar object-dot color source and packing chain  
**Primary Addresses:** `0x00655C50`, `0x00656150`, `0x0050B840`, `0x00500DF2`, `0x004FCE00`, `0x00687F10`, `0x0069A310`, `0x00474A90`, `0x004BA900`  
**Confidence:** High for dirty-pixel and full-overlay radar dot color mechanics; Medium for the no-owner fallback because standard tracked objects normally have owners and no runtime sample hit that branch.  
**Active in YR:** Yes for standard in-game radar dots. Conditional for `RenderAllCells`, which is the no-window/full-overlay path already separated by radar mode-selector reports.

## Summary

Native in-game radar object dots do not use Rust-style generated RGB ramps from color names. The live dirty-pixel path resolves the object's owner or disguise house, reads the already-initialized house RGB bytes at `HouseClass+0x56F9..0x56FB`, and packs those bytes into the current DirectDraw 16-bit format with `g_DD_*Shift` and `_g_DD_*Loss`.

Those `House+0x56F9..0x56FB` bytes are themselves derived earlier from the house color scheme's converted palette data. House creation/session setup maps lobby color priority through `SessionClass::PriorityToColorScheme`; scenario load can read `Color=` by color-scheme name; both flows then extract a pixel at `ColorScheme+0x330` from `ColorScheme+0x30C` and unpack it back into RGB bytes for later consumers such as radar dots, target lines, and selection visuals.

## Target and Non-Scope

In scope:

- How a house color selection becomes `House+0x16054`.
- How `House+0x16054` becomes `House+0x56F9..0x56FB`.
- How `RadarClass::RenderCellPixel` packs those RGB bytes into the primary radar surface.
- How `RadarClass::RenderAllCells` differs from the dirty-pixel path.
- Owner/disguise/local-player flash interaction when directly in the dot color path.

Non-scope:

- Object eligibility, tracker priority, and click priority except where needed to identify the color source.
- Full color-scheme class construction from `[Colors]`.
- Live RGB555-vs-RGB565 runtime descriptor sampling.
- Radar event, spy-satellite, terrain, shroud, and fog color shapes.
- Rust implementation edits.

## Verified Binary Findings

### 1. Dirty-pixel object dots use owner or disguise house RGB bytes

`RadarClass::RenderCellPixel @ 0x00655C50` resolves the tracker entry object and then chooses a color-source house:

1. Start from `object+0x21C` through the decompiler's `piVar1[0x87]`.
2. Call object vtable `+0xC4`; when true, call vtable `+0xD0(0)` and replace the color source with the returned house pointer.
3. If the color-source house is non-null, read `+0x56F9`, `+0x56FA`, `+0x56FB`.
4. Pack those bytes through the current display-format globals:

```text
packed =
  ((house[0x56FB] >> g_DD_BLoss) << g_DD_BShift) |
  ((house[0x56FA] >> g_DD_GLoss) << g_DD_GShift) |
  ((house[0x56F9] >> g_DD_RLoss) << g_DD_RShift)
```

Evidence: decompile `0x00655F48..0x00655FE2`.

Load-bearing detail: the dirty path does **not** index directly into `g_ColorSchemeArray` for ordinary owner-colored objects. It uses the cached house RGB bytes and repacks them every object pixel.

### 2. Local-owner status does not change the base dot color; it only affects ordering and flash inversion

The base color is still the owner/disguise house color. Local-player state affects the dot path in two separate ways:

- tracker insertion/winner order, covered by `RADAR_OBJECT_DOT_PRIORITY_VISIBILITY_GATES_GHIDRA_REPORT.md`;
- flashing inversion inside `RenderCellPixel`.

The inversion branch reads `object+0x174` and `object+0x17C` in decompiler indices (`piVar1[0x5D]`, `piVar1[0x5F]`). If the remaining flash duration is positive, it computes:

```text
phase = ((remaining - 1) / RulesClass[0x88])
if phase is odd and object.owner == g_PlayerPtr:
    write ~packed
else:
    write packed
```

If `object+0x174 == -1`, the code uses `object+0x17C` directly as the remaining duration. Otherwise it subtracts elapsed frames from `object+0x17C` before the same phase test.

Evidence: `0x00655FEA..0x0065604B`; `RulesClass+0x88` is the same flash-frame timing source cited by prior radar/minimap reports.

Negative detail: no `LocalRadarColor=` key is read in this object-dot color block. Local ownership does not force green owner dots in this path.

### 3. House creation maps lobby/session color priority through a 9-byte table before house color extraction

`SessionClass::PriorityToColorScheme @ 0x0069A310` maps a lobby/session color priority to a `ColorSchemeArray` index:

```text
if priority == -2:
    return DAT_0083ED1C
if priority < 9:
    return signed_byte DAT_0083ED14[priority]
return priority
```

Evidence: decompile `0x0069A310`. Existing verified lobby doc records the `DAT_0083ED14` bytes as `03 0B 15 1D 0D 19 11 0F 05`.

`ScenarioClass::Create_Houses @ 0x00687F10` calls `HouseClass::Set_Credits_And_Color @ 0x004FCE00`, then immediately overwrites `House+0x16054` with `SessionClass::PriorityToColorScheme`, then calls `HouseClass::InitColor @ 0x0050B840`.

Evidence: `0x006880C0..0x006880E4` for human houses and `0x006881C5..0x006881E4` for AI houses. `HouseClass::Set_Credits_And_Color @ 0x004FCE00` writes the transient raw priority to both `CountryType+0xC0` and `House+0x16054`; the latter is not the final render color in normal skirmish creation.

### 4. Scenario `Color=` can also update `House+0x16054` and cached RGB bytes

`HouseClass::Read_Scenario_INI @ 0x00500DF2` calls `FUN_00474A90(section, "Color", current_color_scheme)` and stores the result into `House+0x16054`. It then validates the color scheme, forces index `5` if the index is negative or the scheme pointer is null, and extracts `House+0x56F9..0x56FB` from the selected color scheme's converted pixel data.

`FUN_00474A90 @ 0x00474A90` reads the color name string and searches `g_ColorSchemeArray` for matching `ColorScheme+0x304` with `ColorScheme+0x310 != 1`. If no match is found, it returns the previous/default color scheme index.

Evidence: `0x00500DF2` decompile around the `Color` read and extraction block; `0x00474A90` decompile.

Load-bearing detail: scenario `Color=` names are resolved to color-scheme indices, not to the simplified Rust `HouseColorIndex` names or generated RGB bases.

### 5. `HouseClass::InitColor` extracts display-format RGB from the color scheme's converted pixel

`HouseClass::InitColor @ 0x0050B840`:

1. If `House+0x16054 < 0`, force it to `5`.
2. Load `scheme = g_ColorSchemeArray[House+0x16054]`.
3. If `scheme == null`, print a forcing message, set `House+0x16054 = 5`, and use `g_ColorSchemeArray[5]`.
4. Load `convert = scheme+0x30C`.
5. Load pixel data pointer from `convert+0x174`.
6. If `convert+0x4 == 1`, read one byte at `pixel_data + scheme+0x330`; otherwise read one 16-bit word at `pixel_data + scheme+0x330 * 2`.
7. Unpack that pixel through display-format globals and write:
   - `House+0x56F9 = R`
   - `House+0x56FA = G`
   - `House+0x56FB = B`

Evidence: decompile `0x0050B840`; equivalent scenario-load extraction seen in `HouseClass::Read_Scenario_INI @ 0x00500DF2`.

Tiny but important detail: `House+0x56F9..0x56FB` are not arbitrary 8-bit INI RGB values. They are the result of reading the converted pixel for the scheme's remap index and expanding it back to 8-bit-ish channels via `_g_DD_*Loss`.

### 6. `RenderAllCells` uses a different color read path than `RenderCellPixel`

`RadarClass::RenderAllCells @ 0x00656150` does not repack `House+0x56F9..0x56FB`. For each first-visited tracker pixel it:

1. Starts from `g_ColorSchemeArray[object.owner->0x16054]`.
2. Applies the same disguise replacement if object vtable `+0xC4` returns true.
3. Uses fallback helper `FUN_0068CA50` only if the disguise house lookup returns null.
4. Reads `convert = scheme+0x30C`, `pixel_data = convert+0x174`, and `index = scheme+0x330`.
5. If `convert+0x4 == 1`, reads one byte; otherwise reads one 16-bit word.
6. Writes that pixel value directly to the primary radar surface.

Evidence: decompile `0x00656150`, especially `0x006561D2..0x00656270`.

Load-bearing detail: `RenderAllCells` and dirty `RenderCellPixel` should normally land on the same display-format color for valid houses, but their mechanisms differ. Exact parity should model the two native paths separately because one goes through cached RGB repacking and the other directly samples converted color-scheme data.

### 7. DirectDraw masks decide both extraction and final packing

`DSurface::Constructor @ 0x004BA900` derives `g_DD_R/G/BShift` and `_g_DD_R/G/BLoss` from the primary surface descriptor masks. `HouseClass::InitColor` uses those globals to unpack the color-scheme pixel into RGB bytes; `RenderCellPixel` later uses the same globals to repack the house RGB bytes into the radar surface pixel.

Evidence: `0x004BA900`; `DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS_GHIDRA_REPORT.md`.

Implication: RGB555 and RGB565 remain separate runtime possibilities. The dot color contract is display-format-parametric until a live runtime descriptor sample proves the actual mode for a given environment.

### 8. No-owner fallback exists but is not proven common for standard tracked objects

In `RenderCellPixel`, if the selected color-source house pointer is null, the code calls `FUN_0068CA50` and reads `returned_scheme+0x330` directly as the pixel value. In `RenderAllCells`, fallback after a null disguise-house lookup calls the same helper but then samples `scheme+0x30C/+0x330` through converted pixel data.

Evidence: `0x00655FCF..0x00655FE5`, `0x00656245..0x00656270`, `FUN_0068CA50 @ 0x0068CA50`.

Status: mechanism touched, but standard-YR liveness is not proven. Ordinary radar-tracked technos/buildings have owner houses. Treat this as a defensive branch to preserve, not as the expected normal color path.

## Active in Standard YR?

Yes for ordinary in-game radar dots:

- `RadarClass::RenderCellPixel @ 0x00655C50` is called by live radar update dirty-pixel handling, already established in recent minimap reports.
- `TechnoClass::RegisterOnRadar @ 0x0070CC90` and `BuildingClass::RegisterOnRadar @ 0x00456580` populate the tracker used by this color path.
- `ScenarioClass::Create_Houses @ 0x00687F10` is live for standard non-campaign/skirmish house creation and calls the color initialization path.
- `HouseClass::Read_Scenario_INI @ 0x00500DF2` is live for scenario-defined houses and can update `Color=`.

Conditional:

- `RenderAllCells @ 0x00656150` is active only for the no-window/full-overlay path separated by `RENDERALLCELLS_MODE_SELECTOR_GHIDRA_REPORT.md`; it is not the ordinary dirty-pixel path.
- Local flash inversion requires a positive object flash timer phase and `object.owner == g_PlayerPtr`.
- Disguise-house color replacement requires the object vtable `+0xC4` predicate to return true.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Dirty radar object dots pack `House+0x56F9..0x56FB` through DD shift/loss globals | `0x00655F7C..0x00655FE2` | mismatch: Rust `owner_dot_color` uses generated RGBA ramp shade 0 | `src/render/minimap_helpers.rs::owner_dot_color`, `src/render/minimap.rs` | Store native house radar RGB bytes derived from color schemes and pack through a display-format fixture before writing minimap pixels | Soviet/DarkRed house dot matches native packed value under RGB555 and RGB565 fixtures | Do not use generated HSV/RGB ramps as native dot colors |
| House creation maps session color priority via `DAT_0083ED14` before `InitColor` | `0x0069A310`, `0x00687F10`; lobby doc bytes `03 0B 15 1D 0D 19 11 0F 05` | mismatch risk: Rust maps color names directly to compact 0..8 enum | `src/map/houses.rs`, `src/app_skirmish.rs`, `src/sim/house_state.rs`, `src/rules/house_colors.rs` | Preserve lobby priority -> color-scheme index mapping and the post-creation `House+0x16054` value | Skirmish color slot 0 resolves through native table to the same color-scheme entry before extracting dot color | Do not assume lobby color index equals color-scheme array index |
| Scenario `Color=` resolves by color-scheme name with `ColorScheme+0x310 != 1` and fallback to prior index | `0x00474A90`, `0x00500DF2` | partial: Rust parses color names to local enum aliases | `src/map/houses.rs`, `src/rules/house_colors.rs` | Resolve scenario color names against loaded native color schemes, preserving default on miss | House with invalid `Color=` retains previous/native default scheme instead of falling back to Rust Gold by name heuristic | Do not use substring aliases as the parity mechanism |
| `HouseClass::InitColor` extracts RGB from `ColorScheme+0x30C` pixel data at `ColorScheme+0x330` | `0x0050B840`; `0x00500DF2` | missing: Rust ramps are generated constants, not read from converted palette data | `src/rules/house_colors.rs`, palette/convert helpers, future native display-format helper | Build house radar RGB from converted color scheme data using the same 8-bit/16-bit branch and display-format unpacking | For each stock `[Colors]` MP color, extracted house RGB equals native `House+0x56F9..0x56FB` bytes | Do not treat `[Colors]` HSV triples as final radar RGB |
| `RenderAllCells` samples `ColorScheme+0x30C/+0x330` directly while dirty path repacks cached RGB | `0x00656150`, `0x00655C50` | missing: Rust has one RGBA dot path | `src/render/minimap.rs` | Keep separate dirty-pixel and full-overlay mechanisms once native retained radar surface is implemented | Forced `RenderAllCells` fixture writes the same pixel via color-scheme convert path, while ordinary dirty update writes via cached RGB pack path | Do not collapse mechanisms unless exhaustive RGB555/RGB565 equivalence is proven |
| Local flashing writes bitwise `~packed` only for local-owned object and odd phase | `0x00655FEA..0x0065604B` | likely missing: Rust minimap dots do not model native flash inversion cadence | `src/render/minimap.rs`, future radar tracker state | Apply inversion after native packed color selection and before primary-surface write | Local selected/flashing object alternates between packed color and bitwise inverse at `Rules+0x88` cadence; enemy object does not invert | Do not invert RGBA channels after GPU conversion and call it equivalent |

## Negative Facts / Do Not Do

- Do not color live radar object dots from `SIDEBAR.PAL`, `CAMEO.PAL`, `OBSERVER.PAL`, or `RADAR.SHP`. This is generated-surface pixel data.
- Do not use Rust's generated `house_color_ramp(index)[0]` as a parity substitute for `House+0x56F9..0x56FB`.
- Do not assume lobby color index equals native color-scheme index; `PriorityToColorScheme` uses `DAT_0083ED14`.
- Do not treat `Color=` names as arbitrary substring aliases for exact parity; native searches color-scheme names and excludes `ColorScheme+0x310 == 1` in `FUN_00474A90`.
- Do not use `LocalRadarColor=` for base object-dot color. The verified dot color block does not read it.
- Do not apply local flash inversion before display-format packing; native inverts the final packed pixel value with `~packed`.
- Do not merge `RenderCellPixel` and `RenderAllCells` color source mechanisms without proof; one repacks cached house RGB bytes, the other samples color-scheme converted data directly.

## Remaining Uncertainty

- The no-owner fallback branch in `RenderCellPixel` is mechanically visible, but standard tracked-object liveness was not proven with a runtime sample. Ordinary standard objects have owner houses.
- Full `ColorSchemeClass` construction from `[Colors]` was not redrained in this slot; this report relies on the verified read/consumer offsets plus prior color-system docs for table bytes and scheme semantics.
- Live DirectDraw descriptor sampling remains deferred, so the final packed values are parametric over RGB555/RGB565 until runtime proves the active mask.
- Observer-specific radar-dot color policy was not separately traced beyond the direct color path: if observer-visible tracked objects use a house pointer, this report's mechanism applies; broader observer visibility ownership rules remain outside scope.

## Stale-Doc Replacement Wording

Use this wording where older docs say object dots use "house color ramp", "owner palette", or "default color" without separating mechanisms:

> Live dirty-pixel radar object dots resolve the object's owner or disguise house, read the cached house RGB bytes at `HouseClass+0x56F9..0x56FB`, and pack those bytes through the current DirectDraw shift/loss globals. Those cached bytes are initialized from the selected `ColorSchemeClass`: house creation maps lobby color priority through `SessionClass::PriorityToColorScheme`, scenario `Color=` resolves by color-scheme name, and `HouseClass::InitColor` samples `ColorScheme+0x30C` pixel data at `ColorScheme+0x330` before unpacking to RGB. The `RenderAllCells` bulk path is distinct: it samples `ColorScheme+0x30C/+0x330` directly instead of repacking cached house RGB bytes. Local ownership affects tracker priority and optional bitwise inversion of the final packed pixel; it does not replace the base color with `LocalRadarColor=`.

## Status

COMPLETE for `HOUSE_COLORSCHEME_TO_RADAR_DOT_PACKED_COLOR` as a bounded radar-dot color-source and packing slice.

Partial only for full color-scheme construction from `[Colors]`, no-owner fallback liveness, observer visibility policy outside the direct color path, and live DirectDraw RGB555/RGB565 runtime sampling.

## Sources

- Ghidra read-only decompile: `RadarClass::RenderCellPixel @ 0x00655C50`.
- Ghidra read-only decompile: `RadarClass::RenderAllCells @ 0x00656150`.
- Ghidra read-only decompile: `HouseClass::InitColor @ 0x0050B840`.
- Ghidra read-only decompile: `HouseClass::Read_Scenario_INI @ 0x00500DF2`.
- Ghidra read-only decompile: `HouseClass::Set_Credits_And_Color @ 0x004FCE00`.
- Ghidra read-only decompile: `ScenarioClass::Create_Houses @ 0x00687F10`.
- Ghidra read-only decompile: `SessionClass::PriorityToColorScheme @ 0x0069A310`.
- Ghidra read-only decompile: `FUN_00474A90 @ 0x00474A90`.
- Ghidra read-only decompile: `FUN_0068CA50 @ 0x0068CA50`.
- Ghidra read-only decompile: `DSurface::Constructor @ 0x004BA900`.
- Prior docs: `RADAR_OBJECT_DOT_PRIORITY_VISIBILITY_GATES_GHIDRA_REPORT.md`, `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS_GHIDRA_REPORT.md`, `HOUSE_CREATION_COLOR_SYSTEM.md`, `LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: `src/render/minimap.rs`, `src/render/minimap_helpers.rs`, `src/rules/house_colors.rs`, `src/map/houses.rs`, `src/sim/house_state.rs`.
