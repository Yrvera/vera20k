# Skirmish Map Preview Source Bounds and PreviewPack Decode

Date: 2026-05-20
Mode: `/re-investigate` coverage-map
Scope: offline Skirmish/map-selection preview source bounds, start markers, and `[PreviewPack]` format/decode boundaries.

## Summary

The Skirmish preview path has two distinct marker mechanisms:

1. The preview image itself can contain small start markers. `GenerateTerrainPreview @ 0x00641140` paints map terrain into a low-resolution preview surface and then paints small start-position pixels from the first valid waypoints.
2. `DrawStartPositions @ 0x00640710` can overlay `STARTBUT.SHP` frame 0 plus numeric labels over an existing preview surface. This overlay is driven by `ScenarioClass+0x112C..+0x1144` fields.

`[Preview] Size=0,0,138,75` is a four-int rectangle/size record. For menu image decode, the drawable dimensions are the third and fourth values (`138x75` in `Dustbowl.map`), not the first two zeros. The current Rust parser records `(0,0)` for this case.

The exact source-bound population has a split answer:

- Verified consumer: `DrawStartPositions @ 0x00640710` subtracts `ScenarioClass+0x112C/+0x1130`, divides by `+0x1134/+0x1138`, then scales into the preview destination.
- Verified `[Header]` reader: `FUN_00689D30` and `ScenarioClass__Read_INI_Basic @ 0x00689E90` read `[Header] StartX`, `StartY`, `Width`, `Height`, `NumberStartingPoints`, and `Waypoint%d` into those fields.
- Verified `[Header]` writer/generator: `FUN_0068AD70` computes source bounds by iterating the map playfield, projecting playable cells through the same cell-to-preview transform used by preview generation, then writes `[Header]` fields. It also writes `Waypoint1..Waypoint8` from the first eight gameplay waypoints after projection.
- Not fully proven for standard bundled `.map` menu load: `Dustbowl.map` has `[Map] LocalSize=2,8,65,62`, `[Waypoints]`, `[Preview]`, and `[PreviewPack]`, but no `[Header]`. Therefore `[Header]` is a verified path, but it is not sufficient as the only explanation for stock Skirmish map previews.

## Active YR Paths

### `DrawStartPositions @ 0x00640710`

Active in standard YR offline Skirmish paint when a preview surface exists. The function:

- calls `ValidateRect`;
- finds child control `0x468`;
- queries the preview surface source rectangle/size through vtable `+0x78`;
- aspect-fits the preview into the child destination;
- blits the preview surface first;
- lazy-loads `STARTBUT.SHP`;
- reads `ScenarioClass+0x113C`;
- draws overlays only when `0 < count < 9`;
- reads marker coordinate pairs from `ScenarioClass+0x1140+i*8` and `+0x1144+i*8`;
- applies draw offsets `x - 9`, `y - 6`;
- draws shape frame 0, then draws the numeric label.

Projection inputs:

| Field | Meaning in this path |
| --- | --- |
| `ScenarioClass+0x112C` | source origin X |
| `ScenarioClass+0x1130` | source origin Y |
| `ScenarioClass+0x1134` | source width |
| `ScenarioClass+0x1138` | source height |
| `ScenarioClass+0x113C` | start overlay count |
| `ScenarioClass+0x1140+i*8` | start overlay X |
| `ScenarioClass+0x1144+i*8` | start overlay Y |

The decompiler expression is integer math with `*1000` scale factors before division. Do not replace it with floating-point UI math if implementing parity-sensitive placement.

### `GenerateTerrainPreview @ 0x00641140`

This function creates the low-resolution map preview surface. It:

- iterates `MapClass` cells;
- keeps only cells passing `MapClass__Is_Cell_In_Playfield`;
- converts cell coordinates through the cell preview projection;
- divides projected X by `0x3C` and projected Y by `0x1E`;
- tracks min/max projected bounds over the playable cells;
- allocates a `DSurface` with width `(max_x - min_x) * 2` and height `(max_y - min_y)`;
- draws two horizontally adjacent pixels per cell;
- uses terrain/overlay/building radar colors, with building type RGB at object type offsets `+0x29C/+0x29D/+0x29E`;
- logs and substitutes grey if a black pixel is produced;
- loops waypoints `0..7`, validates each with `FUN_0068BD80`, reads it with `FUN_0068BCC0`, projects it, and paints a small 4x4 marker into the generated image.

This means a decoded `[PreviewPack]` can already contain start markers before `STARTBUT.SHP` overlays are considered.

### `FUN_006418B0` / `00641A78` Preview Storage

This path writes preview data to INI sections:

- if no preview surface exists, it calls `GenerateTerrainPreview`;
- writes `[Preview]` metadata;
- walks the preview surface by height and width;
- reads each pixel;
- writes exactly 3 bytes per pixel into an LZO pipe;
- stores the compressed payload in `[PreviewPack]`.

The confirmed payload model is:

```text
[Preview] Size = left,top,width,height
[PreviewPack] = INI-binary text block containing LZO-compressed RGB triples
decoded byte count target = width * height * 3
pixel order = row-major, 3 bytes per pixel
```

The exact byte channel order should still be verified by decoding a known map and comparing the resulting pixels against retail. The writer path passes three bytes per pixel from the surface color read; the decompile does not by itself prove RGB vs BGR at the map file boundary.

### Generic INI Binary Read Path

`Pipe__Constructor` is a misnamed generic INI binary reader/writer helper in the current Ghidra labels. For a section name such as `PreviewPack`, it:

- finds the INI section;
- iterates its keyed lines;
- trims each text value;
- passes each line through a binary text decoder (`FUN_0042DDB0`);
- feeds a pipe/buffer chain.

This supports the interpretation that `[PreviewPack]` is not plain base64 image data directly; it is INI-encoded compressed bytes, then decompressed by the preview pipe chain.

## Header Field Paths

### Reader: `FUN_00689D30`

This function initializes the header fields to `-1` and zeroes eight waypoint pairs, then reads:

- `[Header] StartX` -> `+0x112C`
- `[Header] StartY` -> `+0x1130`
- `[Header] Width` -> `+0x1134`
- `[Header] Height` -> `+0x1138`
- `[Header] NumberStartingPoints` -> `+0x113C`
- `[Header] NumCoopHumanStartSpots` -> `+0x11E4`
- `[Header] Waypoint%d`, with `i` starting at 1 in the decompiled loop, into `+0x1140`.

### Full Scenario Parser: `ScenarioClass__Read_INI_Basic @ 0x00689E90`

The full scenario parser repeats the same `[Header]` reads as part of broader scenario INI parsing. This verifies the field mapping independently, but it does not prove stock multiplayer maps carry the section.

### Writer/Generator: `FUN_0068AD70`

This function writes `[Header]` metadata. Important verified behavior:

- Iterates playable cells and projects them through the map preview transform.
- Writes `StartX`, `StartY`, `Width`, and `Height` from projected min/max bounds, not directly from `[Map] LocalSize`.
- Counts valid gameplay waypoints among indices `0..7`.
- Writes `NumberStartingPoints`.
- Writes `Waypoint1..Waypoint8` from projected coordinates of the first valid gameplay waypoint cells.

This function is the missing link between gameplay waypoints/playfield bounds and the `[Header]` overlay metadata. It is a generator/writer path, not proof that every shipped map file contains `[Header]`.

## Stock Map Data Check

Retail install sample:

```text
C:/Users/enok/Documents/Command and Conquer Red Alert II/Dustbowl.map
[Preview]
Size=0,0,138,75
[PreviewPack]
...
[Map]
Size=0,0,70,76
LocalSize=2,8,65,62
[Waypoints]
0=116070
1=34079
...
```

No `[Header]` section was found in this file. This matters because `DrawStartPositions` overlay fields cannot be populated from `[Header]` for this specific file unless a separate runtime generator runs before paint. The preview image can still show small baked-in markers because `GenerateTerrainPreview` paints them into the preview surface before `[PreviewPack]` is stored.

## Rust Status

Current Rust state:

- `src/map/preview.rs` parses `[Preview] Size` as the first two comma fields. This is wrong for `Size=0,0,138,75`; it should treat the four-field form as a rect and use width/height from fields 3 and 4.
- `PreviewSection` only records `has_packed_preview`; it does not decode `[PreviewPack]`.
- `src/app_list_maps.rs` intentionally leaves `preview_source_bounds_from_verified_source` as `None`, because earlier research had not proven the source. This doc verifies the `[Header]` path and the generator path, but still does not prove a direct `[Map] LocalSize -> DrawStartPositions fields` load path for stock `.map` menu display.
- `src/app_skirmish_shell_render.rs` gates `STARTBUT.SHP` marker sprites and labels behind `real_preview_surface_available() == false`, so overlays are currently disabled.

Implementation implication:

1. Fix `[Preview] Size` parsing first.
2. Decode `[PreviewPack]` into a real preview surface before enabling overlays.
3. Treat start markers as two layers:
   - baked preview pixels from `[PreviewPack]`;
   - optional `STARTBUT.SHP` overlays only when verified header/source-bound fields are available.
4. Do not use `[Map] LocalSize` as a drop-in replacement for `DrawStartPositions` source bounds. The verified generator computes bounds by projecting playable cells and taking min/max.

## Coverage Ledger

| Item | Status | Evidence |
| --- | --- | --- |
| Offline Skirmish preview paint calls `DrawStartPositions` | Verified | Existing live reports plus `0x006AE3F0` path |
| `DrawStartPositions` field offsets and draw order | Verified | Fresh decompile `0x00640710` |
| Header field reader | Verified | Fresh decompile `0x00689D30` |
| Full scenario parser header reads | Verified | Fresh decompile `0x00689E90` |
| Header writer/generator | Verified | Fresh decompile `0x0068AD70` |
| Gameplay `[Waypoints]` read format | Verified | Fresh decompile `0x0068BDC0` |
| Gameplay `[Waypoints]` write format | Verified | Fresh decompile `0x0068BE90` |
| Preview surface generation | Verified | Fresh decompile `0x00641140` |
| PreviewPack storage format | Verified to format boundary | Fresh decompile `0x006418B0` / `0x00641A78` |
| PreviewPack channel order | Open | Need decoded-pixel comparison |
| Stock `.map` header absence | Verified sample | `Dustbowl.map` inspection |
| Direct stock menu runtime population of `+0x112C..+0x113C` without `[Header]` | Open | Need live trace/call chain |

## Open Questions

1. Does the stock Skirmish menu generate `[Header]`-equivalent fields in memory before painting bundled multiplayer maps that lack `[Header]`? This needs a live call-chain or memory watch around map selection.
2. What exact byte channel order does `[PreviewPack]` use at the serialized boundary? The writer confirms 3 bytes/pixel, but a decode-vs-retail pixel sample should settle RGB/BGR.
3. Are `STARTBUT.SHP` numbered overlays normally visible for stock maps without `[Header]`, or are those maps relying only on baked preview markers? This should be verified with a retail screenshot or memory inspection of `ScenarioClass+0x113C` after choosing Dustbowl.

