# High Bridge Body Asset Resolution - Ghidra Research Report

**Date:** 2026-05-16
**Scope:** Narrow follow-up on whether the Rust bridge body atlas loads/parses the same high bridge body SHPs as `gamemd.exe`.
**Primary addresses:** `0x005F92D0`, `0x005F9070`, `0x005FEDE0`, `0x0047F6A0`, `0x0069E740`, `0x0069E7E0`, `0x0069E900`, `0x004AED70`, `0x005B4030`, `0x005FD2E0`.
**Confidence:** HIGH for `Image=`/`Theater=` resolution and draw-time SHP pointer use; HIGH for SHP header/frame-header offsets; MEDIUM for exact SHP codec semantics because the previous asset report still has codec-level open work inside the blitters.
**Active in YR:** Yes. The draw path is reached through `Tactical_layer_smudges -> Cell_ContentRendering -> CellClass__DrawOverlay_Body` in the normal Tactical draw path, and map overlay stamping is reached from `ScenarioClass__Full_Init -> ReadMapOverlayPacks`.

## 1. Summary verdict

For retail high bridge body art, Rust's current filename resolution is aligned on the important question: `BRIDGE1` and `BRIDGE2` resolve through `Image=BRIDGE`; `BRIDGEB1` and `BRIDGEB2` resolve through `Image=BRIDGB`; `[BRIDGE]` and `[BRIDGB]` have `Theater=yes`, so the theater SHP candidate is `BRIDGE.<theater>` or `BRIDGB.<theater>`.

That means the earlier statement "probably structurally OK for body art" can be strengthened:

- **Verified match:** Rust uses the same INI indirection that `gamemd.exe` uses for the stock high bridge body SHP names.
- **Verified match:** Rust parses the same SHP(TS) header fields needed by the binary frame accessors: file width, file height, frame count, per-frame rect, and data offset.
- **Verified non-match:** Rust adds fallback candidates that `gamemd.exe` does not use for this path (`.SHP` after a `Theater=yes` lookup and numeric-suffix decrement aliases).
- **Verified non-match:** Rust clamps/falls back when a requested bridge frame is missing or empty; the binary frame accessors do not clamp to the nearest valid bridge frame.
- **Verified non-match outside pure parsing:** Rust pre-renders bridge SHP frames into RGBA atlas textures and gives the bridge atlas an all-zero depth texture. `gamemd.exe` keeps indexed SHP data and sends it through `CC_Draw_Shape`/SHP blitters with Z/alpha-buffer behavior.

So: **the body asset name resolution is correct for stock YR, but this is not end-to-end bridge rendering parity.** The remaining body-rendering risk is not mainly "wrong SHP file"; it is fallback behavior, missing-frame behavior, indexed-pixel blitter semantics, and Z/depth/shadow substrate differences after the SHP is loaded.

## 2. Class layout / key offsets

### ObjectTypeClass / OverlayTypeClass fields

| Offset | Owner | Verified use | Evidence |
|---:|---|---|---|
| `+0x24` | `AbstractTypeClass` | Type ID / section name. Used as INI section key. | `ObjectTypeClass__ReadINI @ 0x005F92D0`, `OverlayTypeClass__FindOrCreate @ 0x005FEC70` |
| `+0x1F8` | `ObjectTypeClass` | Image base name. Defaults from type ID, then overwritten by `Image=`. | `ObjectTypeClass__Constructor @ 0x005F7090`, `ObjectTypeClass__ReadINI @ 0x005F92D0` |
| `+0x213` | `ObjectTypeClass` | `AlphaImage=` base name, loaded separately when non-empty. Not bridge body art. | `ObjectTypeClass__ReadINI @ 0x005F92D0` |
| `+0x22C` | `ObjectTypeClass` | `Theater=` boolean. When true, filename extension is the current theater extension. | `ObjectTypeClass__ReadINI @ 0x005F92D0`, `FUN_005F9070` |
| `+0x237` | `ObjectTypeClass` | `NewTheater=` boolean. Enables second-letter theater substitution for GA/GT/NA/NT/CA/CT-like names when not using `Theater=`. | `ObjectTypeClass__ReadINI @ 0x005F92D0`, `FUN_005F9070` |
| `+0xA4` | `ObjectTypeClass` | Loaded SHP pointer / descriptor pointer returned by the vtable entry used by bridge draw. | `FUN_005F9070`, `OverlayTypeClass__GetRadarColor @ 0x005FEDE0` |
| `+0x2AF` | `OverlayTypeClass` | Demand-load flag checked by the vtable getter. Default false in constructor. | `OverlayTypeClass__Constructor @ 0x005FE250`, `OverlayTypeClass__GetRadarColor @ 0x005FEDE0` |

### CellClass fields used after asset resolution

| Offset | Purpose | Evidence |
|---:|---|---|
| `+0x44` | Overlay type index into `g_OverlayTypeClass_Array`. | `CellClass__DrawOverlay_Body @ 0x0047F6A0` |
| `+0x11E` | Overlay data / high bridge state byte. Used as body frame selector. | `CellClass__DrawOverlay_Body @ 0x0047F6A0` |
| `+0x140 & 0x80` | High bridge flag. Selects the high bridge body special path. | `CellClass__DrawOverlay_Body @ 0x0047F6A0`, `CellClass__Get_Draw_Offset @ 0x00480110` |
| `+0x11B` | Signed cell level. Used in Z/depth expression. | `CellClass__DrawOverlay_Body @ 0x0047F6A0` |

## 3. Binary asset resolution path

### 3.1 `Image=` is the source of the body SHP base name

`ObjectTypeClass__ReadINI @ 0x005F92D0` first copies the current image buffer (`+0x1F8`) into a local default, then reads:

```text
0x1F8 = ReadString(section, "Image", default = old +0x1F8, max = 0x19)
```

For stock YR:

| Overlay type | `rulesmd.ini` value | Final image base |
|---|---|---|
| `[BRIDGE1]` | `Image=BRIDGE` | `BRIDGE` |
| `[BRIDGE2]` | `Image=BRIDGE` | `BRIDGE` |
| `[BRIDGEB1]` | `Image=BRIDGB` | `BRIDGB` |
| `[BRIDGEB2]` | `Image=BRIDGB` | `BRIDGB` |

**Evidence:** `ObjectTypeClass__ReadINI @ 0x005F92D0`; `ini/rulesmd.ini` lines `29869..29894`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### 3.2 `Theater=yes` changes extension, not the image base

`ObjectTypeClass__ReadINI @ 0x005F92D0` reads:

```text
+0x22C = ReadBool("Theater", old +0x22C)
+0x237 = ReadBool("NewTheater", old +0x237)
```

`FUN_005F9070` then constructs the filename from `+0x1F8`:

```text
if Theater == false:
    filename = image + ".SHP"
    optionally mutate second character for NewTheater
else:
    filename = image + "." + current_theater_extension
```

For stock bridge body art, `[BRIDGE] Theater=yes` and `[BRIDGB] Theater=yes`, so the normal candidates are:

```text
BRIDGE.TEM / BRIDGE.SNO / BRIDGE.URB / ...
BRIDGB.TEM / BRIDGB.SNO / BRIDGB.URB / ...
```

depending on the loaded theater.

**Evidence:** `FUN_005F9070`; `ini/artmd.ini` lines `13111..13115`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### 3.3 The binary fallback is not Rust's fallback

If `LoadFileFromMIX(filename)` fails in `FUN_005F9070`, the binary does this:

```text
local_203 = 0x47  // ASCII 'G', second character of the candidate path/name buffer
retry LoadFileFromMIX()
```

This is the old "generic theater letter" fallback used by many `NewTheater`-style art names. For `BRIDGE`/`BRIDGB`, it would mutate the second character of the constructed name. It is not a numeric-suffix fallback.

Rust, in `bridge_atlas.rs`, adds extra candidates with `decrement_numeric_suffix(key.name)`. For bridge bodies, this means a missing `BRIDGE.TEM` could eventually try names derived from `BRIDGE1 -> BRIDGE0`, even though the binary body path resolves through `Image=BRIDGE` and does not do that.

**Evidence:** `FUN_005F9070`; Rust `src/render/bridge_atlas.rs:174`, `src/render/bridge_atlas.rs:401`.
**Confidence:** HIGH.
**Active in YR:** Yes, but only observable if the normal asset is missing or a mod relies on fallback behavior.

### 3.4 `LoadFileFromMIX` uppercases before hashing

`LoadFileFromMIX @ 0x005B4030` copies the requested name, uppercases the local copy, computes a CRC, then looks in the global asset cache/tree. If no cached node exists, it opens through `CCFileClass` and stores either loaded bytes or a file descriptor depending on extension/flag.

Rust's `AssetManager` lookup is also case-insensitive at the MIX hash level, because `mix_hash(name)` normalizes the requested name. Therefore uppercase/lowercase candidate spelling is not a parity risk for these bridge body files.

**Evidence:** `LoadFileFromMIX @ 0x005B4030`; Rust `src/assets/asset_manager.rs:441`.
**Confidence:** HIGH.
**Active in YR:** Yes.

## 4. Draw-time use of the loaded SHP

### 4.1 Draw code uses the type's loaded SHP pointer

`CellClass__DrawOverlay_Body @ 0x0047F6A0` does:

```text
overlay_type = g_OverlayTypeClass_Array[cell.overlay_index]
shape = overlay_type->vtable[0x9C]()
...
CC_Draw_Shape(shape, frame, position, clip, 0x4E00, ...)
```

The function at vtable `+0x9C` is currently labelled `OverlayTypeClass__GetRadarColor @ 0x005FEDE0` in Ghidra, but the decompile shows it is actually the image getter/demand-loader:

```text
image = this +0xA4
if image == 0 and demand_load_flag(+0x2AF) != 0:
    build image filename from +0x1F8 and theater flags
    load SHP bytes
    store +0xA4
return +0xA4
```

So draw-time bridge body rendering does not re-resolve names from the overlay ID. It consumes the `OverlayTypeClass`'s already-resolved image pointer.

**Evidence:** `CellClass__DrawOverlay_Body @ 0x0047F6A0`; `OverlayTypeClass__GetRadarColor @ 0x005FEDE0`; vtable memory at `0x007EF600 + 0x9C`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### 4.2 Map load validates that overlay art exists before stamping

`ReadMapOverlayPacks @ 0x005FD2E0` reads each overlay byte from `[OverlayPack]`, then calls the same vtable `+0x9C` image getter before constructing the overlay:

```text
shape = overlay_type->vtable[0x9C]()
if shape != 0 or overlay_type.CellAnim != 0:
    new OverlayClass(...)
```

This means missing bridge body SHP art can prevent the overlay from being stamped from the pack at all. The preservation special case for high bridge overlay data then runs only after the overlay constructor path.

**Evidence:** `ReadMapOverlayPacks @ 0x005FD2E0`; xref from `ScenarioClass__Full_Init @ 0x00687A34`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### 4.3 Body frame selection is still state-byte driven after asset load

Once the SHP pointer exists, the body frame is selected from `cell+0x11E`:

```text
state = cell+0x11E
if state == 0 or state == 9:
    state += g_OverlayVarietyLatinSquare[((cell.y & 3) << 2) | (cell.x & 3)]
frame = state
```

This is independent of the resolved filename. `BRIDGE1` and `BRIDGE2` can both point at the same `BRIDGE.<theater>` SHP because the cell state byte selects the frame.

**Evidence:** `CellClass__DrawOverlay_Body @ 0x0047F6A0`.
**Confidence:** HIGH.
**Active in YR:** Yes.

## 5. SHP file-format facts relevant to body parsing

### 5.1 Header and frame table offsets

The binary frame accessors verify this layout:

```text
SHP header:
  +0x00 i16 magic/descriptor marker
  +0x02 i16 width
  +0x04 i16 height
  +0x06 i16 frame_count

Frame header:
  base = shp + 0x08 + frame_index * 0x18
  +0x00 i16 frame_x
  +0x02 i16 frame_y
  +0x04 i16 frame_width
  +0x06 i16 frame_height
  +0x08 u8  frame flags / format bits
  +0x14 u32 data_offset
```

`SHP_frame_rect_getter @ 0x0069E7E0` reads `frame_x/y/w/h` as signed 16-bit values and returns a global default rect if the frame index is out of range.

`SHP_frame_data_getter @ 0x0069E740` reads `data_offset` at `+0x14`; if the frame index is out of range or `data_offset == 0`, it returns null.

**Evidence:** `0x0069E7E0`, `0x0069E740`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### 5.2 The frame flag byte chooses standard vs extended SHP blitter

`SHP_frame_flag_check @ 0x0069E900` returns:

```text
(frame_header[+0x08] & 0x02) >> 1
```

`CC_Draw_Shape @ 0x004AED70` uses this bit to choose:

```text
0 -> Standard_SHP_blitter
1 -> Extended_SHP_blitter
```

This is not just a parser concern. Bridge body frames with bit `0x02` set enter a different SHP blitter family.

**Evidence:** `SHP_frame_flag_check @ 0x0069E900`; `CC_Draw_Shape @ 0x004AED70`.
**Confidence:** HIGH for the bit check and dispatch; MEDIUM for exact codec operation inside the blitters.
**Active in YR:** Yes.

### 5.3 Binary does not clamp missing bridge frames to a nearby valid frame

The binary frame helpers check `frame_index < frame_count`. If the requested frame is outside the SHP frame count, `SHP_frame_rect_getter` returns a default rect and `SHP_frame_data_getter` returns null. If the frame header exists but `data_offset == 0`, data access also returns null.

Rust's `bridge_atlas.rs` instead clamps the requested frame to the selected body/shadow window and, if that frame is empty, falls back to the first non-empty frame in the same window.

This is not expected to matter for stock `BRIDGE`/`BRIDGB` if all requested frames exist and are non-empty. It is a verified behavioral difference for malformed or modded SHPs.

**Evidence:** `SHP_frame_data_getter @ 0x0069E740`; `SHP_frame_rect_getter @ 0x0069E7E0`; Rust `src/render/bridge_atlas.rs:203`, `src/render/bridge_atlas.rs:213`.
**Confidence:** HIGH.
**Active in YR:** Yes when a requested frame is missing/empty.

## 6. Current Rust implementation status

### 6.1 Matches binary for stock body image indirection

Rust:

- filters high bridge body overlay names in `src/render/bridge_atlas.rs:78`;
- resolves the effective `Image=` through `ArtRegistry::resolve_overlay_image_id` in `src/render/bridge_atlas.rs:166`;
- uses `overlay_shp_candidates` in `src/render/bridge_atlas.rs:167`;
- `overlay_shp_candidates` honors `Theater=yes` through `overlay_convention_flags` in `src/rules/art_data.rs:557` and `src/rules/art_data.rs:754`.

For stock YR, this produces the same important body image base names:

```text
BRIDGE1/BRIDGE2   -> Image=BRIDGE -> BRIDGE.<theater>
BRIDGEB1/BRIDGEB2 -> Image=BRIDGB -> BRIDGB.<theater>
```

**Rust state:** MATCH for stock body asset name resolution.

### 6.2 Structurally matches verified SHP header/frame fields

Rust parses:

- file width at header `+0x02`;
- file height at header `+0x04`;
- frame count at header `+0x06`;
- frame rect at per-frame offsets `+0x00..+0x06`;
- frame format byte at `+0x08`;
- data offset at `+0x14`.

These align with the binary accessors.

**Rust state:** MATCH for the body SHP fields used by frame lookup and atlas placement.

### 6.3 Important Rust differences

| Area | Binary behavior | Rust behavior | Parity impact |
|---|---|---|---|
| `Theater=yes` fallback | Normally tries `<image>.<theater>`, then mutates second character to `G` and retries. | Tries `<image>.<theater>`, then `<image>.SHP`, plus lower-case/hash-equivalent and numeric-decrement fallbacks. | No stock impact if retail `BRIDGE.<theater>` exists; mod/missing-asset behavior differs. |
| Numeric suffix fallback | Not used for high bridge body `Image=BRIDGE/BRIDGB`. | Adds `BRIDGE1 -> BRIDGE0`-style candidates after normal image candidates. | Not stock-visible, but not binary behavior. |
| Missing/empty frame | Helpers return default/null; no nearest-frame fallback in the binary accessors. | Clamps to the selected window and falls back to first non-empty frame. | Only visible with malformed/modded bridge SHPs or if our parser misclassifies a valid frame as empty. |
| Frame x/y signedness | Binary reads frame x/y as signed shorts. | Rust stores `frame_x/frame_y` as `u16` and casts to `u32` for full-canvas blit. | Needs retail bridge SHP dump to prove no negative body frame offsets. Likely safe for stock body frames, but not binary-identical. |
| Indexed SHP substrate | Binary keeps indexed pixels and blits through `CC_Draw_Shape`, A-buffer, and Z-capable SHP blitters. | Rust pre-renders indexed pixels to RGBA in the bridge atlas. | Correct loaded art can still render differently under palette/A-buffer/Z interactions. |
| Bridge depth atlas | Binary SHP blitter receives Z parameters and uses SHP blitter depth behavior. | Bridge atlas creates an all-zero R8 depth texture at `src/render/bridge_atlas.rs:342`. | Body art can be correct while bridge depth/occlusion remains non-parity. |

## 7. Answer to the specific question

The statement:

> Bridge body asset parsing is probably structurally OK for body art.

should now be replaced with:

> **Verified for stock YR filename resolution and core SHP header parsing, but not sufficient for end-to-end body rendering parity.**

More concretely:

1. Rust does resolve the correct stock high bridge body image bases (`BRIDGE` and `BRIDGB`) through the same `Image=` indirection as the binary.
2. Rust does honor `Theater=yes`, so the primary retail candidate is the correct theater SHP.
3. Rust parses the SHP header/frame table fields at the same offsets the binary uses.
4. Rust is not binary-equivalent for fallback selection, missing-frame behavior, signed frame offsets, or the indexed SHP blitter substrate.
5. Therefore, if a visual diff still shows wrong bridge body pixels, the first suspects should be render substrate/depth/palette/frame-offset handling, not the `BRIDGE1 -> BRIDGE` filename mapping.

## 8. Open questions

1. **Retail bridge SHP frame-offset signedness.** The binary reads frame x/y as signed shorts. Rust stores them as `u16`. Need a retail dump of `BRIDGE.<theater>` and `BRIDGB.<theater>` frame headers to prove all bridge body/shadow frame x/y offsets are non-negative.
2. **Exact SHP codec parity for bridge frames.** Previous asset research left codec-level details at MEDIUM confidence. The parser likely works for current assets, but a byte-for-byte decoded-frame comparison against a trusted decoder or live binary buffer would close this.
3. **Whether bridge body SHPs ever depend on non-theater `.SHP` fallback in standard YR.** Stock INI says `Theater=yes`; no evidence found that standard YR uses the `.SHP` fallback for these body assets.
4. **Palette/A-buffer equivalence for bridge body pixels.** This pass verified asset resolution and SHP structure, not final palette lookup through `CC_Draw_Shape`, A-buffer, and lighting tables.
5. **Bridge depth texture semantics.** Rust's bridge atlas currently uses an all-zero R8 depth texture; this pass did not resolve whether a bridge-body-specific depth raster is required or whether the current uniform depth approximation is visually acceptable.

## Sources

### Ghidra

- `0x005F92D0` `ObjectTypeClass__ReadINI`
- `0x005F9070` object/overlay image filename construction and load helper
- `0x005FEDE0` vtable `+0x9C` image getter/demand-loader, currently mislabeled `OverlayTypeClass__GetRadarColor`
- `0x0047F6A0` `CellClass__DrawOverlay_Body`
- `0x005FD2E0` `ReadMapOverlayPacks`
- `0x00687A34` `ScenarioClass__Full_Init` xref to overlay pack load
- `0x006D3290` `Tactical_layer_smudges`
- `0x006D7001` xref site in `Cell_ContentRendering`
- `0x0069E740` `SHP_frame_data_getter`
- `0x0069E7E0` `SHP_frame_rect_getter`
- `0x0069E900` `SHP_frame_flag_check`
- `0x004AED70` `CC_Draw_Shape`
- `0x005B4030` `LoadFileFromMIX`

### INI

- `ini/rulesmd.ini` `[BRIDGE1]`, `[BRIDGE2]`, `[BRIDGEB1]`, `[BRIDGEB2]`
- `ini/artmd.ini` `[BRIDGE]`, `[BRIDGB]`

### Rust files read

- `src/render/bridge_atlas.rs`
- `src/assets/shp_file.rs`
- `src/assets/shp_decode.rs`
- `src/assets/asset_manager.rs`
- `src/rules/art_data.rs`
- `src/map/overlay_types.rs`

### Prior reports checked

- `docs/research/ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_RENDERING_REMAINING_CASES_GHIDRA_REPORT.md`
- `docs/research/ZBUFFER_DEPTH_SYSTEM.md`
