# Skirmish PreviewPack Thumbnail Design

## Goal

Decode `[PreviewPack]` thumbnails from retail skirmish maps and render the selected map preview in the existing Skirmish setup shell.

## Architecture Context

The current Skirmish setup screen already has a lightweight map metadata path:

`list_available_maps` scans retail map files, `read_map_menu_entry` extracts INI metadata, `MapMenuEntry` stores the menu-facing result, and `app_skirmish_shell_render` renders the shell from `SkirmishShellState.selected_map_idx`.

Today this path only records preview metadata. `src/map/preview.rs` parses `[Preview] Size=` and records whether `[PreviewPack]` contains non-empty values. `src/app_skirmish_shell_render.rs` still has `real_preview_surface_available() -> false`, so `PreviewSurface`, `StartMarker`, and `StartMarkerLabel` draw roles are gated off.

The repo already has most of the low-level decode pieces needed for the real thumbnail:

- `src/util/base64.rs` decodes map-style base64 text while skipping whitespace.
- `src/util/lzo.rs` decompresses chunked LZO streams with the same `[u16 src_len][u16 dst_len][compressed bytes]` layout used elsewhere in map binary sections.
- `src/render/batch.rs` already supports uploaded RGBA textures and sprite instances.

The decode belongs in `map/preview` because it is map-file data. GPU texture upload and drawing belong in the app/render layer. `sim/` must not participate.

## Impact Analysis

Primary touched files:

- `src/map/preview.rs`: parse `Size=` correctly, collect `[PreviewPack]` text, decode/decompress/validate pixels, expose decoded preview data.
- `src/app_init.rs`: extend `MapMenuEntry`/preview data shape if needed.
- `src/app_list_maps.rs`: keep using the existing metadata extraction path and populate decoded preview data there.
- `src/app_skirmish_shell_render.rs` and adjacent app state/render glue: upload and draw the selected map preview texture.

Likely tests:

- `src/map/preview.rs` unit tests for two-field and four-field `Size=`.
- Unit test for empty/missing `[PreviewPack]`.
- Unit test for exact decoded byte-count validation using a small synthetic chunk.
- Render/state test or visual check that a selected map with decoded data enables `PreviewSurface` without enabling start-marker overlays.

Risk areas:

- Channel order is not yet proven as RGB vs BGR. The design must isolate this in one conversion point and require visual verification against gamemd.exe before claiming final parity.
- Up-front decode during map listing may add startup cost. Retail thumbnails are small, so this is acceptable for the first slice. If it becomes visible, a later cache/lazy path can be introduced without changing the decode contract.
- MIX-wrapped map handling must stay in the existing `read_map_ini_for_metadata` path. Do not duplicate map file probing in the renderer.

## Chosen Approach

Approach A: decode CPU preview pixels during map metadata load, then upload the selected map's RGBA pixels as a skirmish shell texture.

This follows the existing map-menu data flow, keeps map decoding out of render code, and keeps GPU resource ownership above the map layer. It also leaves the modal Choose Map behavior out of scope, as requested.

Rejected alternatives:

- Lazy-decode only when selection changes: lower up-front cost but duplicates file/INI/MIX loading concerns outside the metadata reader.
- Dedicated preview cache service keyed by map filename: probably useful for the future modal map chooser, but too much infrastructure for rendering one selected thumbnail in the current shell.

## Tiny-Detail Ledger

- `[Preview] Size=0,0,138,75` is a four-integer rectangle; preview dimensions are fields 3 and 4. Existing two-field parsing remains accepted for tests/custom maps. Source: `SKIRMISH_MAP_PREVIEW_SOURCE_BOUNDS_AND_PREVIEWPACK_GHIDRA_REPORT.md`.
- `[PreviewPack]` is base64-style INI binary data over LZO chunks, not raw pixels. Source: `SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md`.
- The decompressed payload length must equal `width * height * 3`; Dustbowl verifies `138 * 75 * 3 = 31050`. Source: `SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md`.
- Pixel storage is row-major, with height as the outer loop, width as the inner loop, and 3 bytes per pixel. Source: `SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md`.
- Serialized channel order is still unresolved. Source: `SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md`.
- Normal selected-map preview loading is active in YR skirmish through the selected `.map` preview loader path. Source: `SKIRMISH_PREVIEW_OBJECT_LIFECYCLE_DAT_00AC1154_GHIDRA_REPORT.md`.
- Stock maps without verified `[Header]` source bounds should not draw separate start-marker overlays. Their visible starts are baked into the preview pixels. Source: `SKIRMISH_PREVIEW_SCENARIO_BOUNDS_STOCK_MAP_POPULATION_GHIDRA_REPORT.md`.
- `GenerateTerrainPreview` bakes red 4x4 start markers for generated/saved preview surfaces, but this first slice targets decoded stock-map `[PreviewPack]` thumbnails only. Source: `SKIRMISH_GENERATE_TERRAIN_PREVIEW_BAKED_START_MARKERS_GHIDRA_REPORT.md`.

## Design

### Components

`DecodedPreview`

- New map-layer data type.
- Holds `width: u32`, `height: u32`, and `rgba: Vec<u8>`.
- The RGBA vector length must be `width * height * 4`.
- Owns CPU pixels only; no GPU resource handles.

`PreviewSection`

- Continue exposing `size` and `has_packed_preview`.
- Add `decoded: Option<DecodedPreview>` or equivalent.
- Keep missing/invalid decode non-fatal for menu listing, but preserve enough logging/error context to diagnose broken maps.

`PreviewChannelOrder`

- A tiny internal enum or conversion helper in `map/preview`.
- Initial default should be whichever current visual verification supports.
- Because RGB/BGR is unresolved in the research, all channel-order handling must live in one function so a one-line change can flip it after visual comparison.

`SkirmishPreviewTexture`

- App/render-layer state, not map-layer state.
- Caches the uploaded `BatchTexture` for the selected map index.
- Rebuilds only when `selected_map_idx` changes or when the selected decoded preview changes.

### Interfaces / Contracts

`parse_preview_section(ini: &IniFile) -> PreviewSection`

- Parse `[Preview] Size=`.
- Accept both two-part `w,h` and four-part `x,y,w,h` forms.
- Detect non-empty `[PreviewPack]`.
- If size and pack data exist, attempt decode.
- If decode fails, return metadata with `decoded: None` and log a warning from the caller or via a structured error-return helper. Map listing must not fail because one thumbnail is corrupt.

Possible lower-level helper:

`decode_preview_pack(ini: &IniFile, width: u32, height: u32) -> Result<DecodedPreview, PreviewDecodeError>`

- Collect all values in `[PreviewPack]` in numeric key order where possible.
- Concatenate values without injecting separators.
- Base64-decode with the existing utility.
- LZO-decompress with the existing chunk utility.
- Validate exact byte count.
- Convert 3-byte row-major pixels to RGBA with alpha 255.

Numeric key order matters because INI sections store pack data split across numbered lines. If the underlying INI section preserves insertion order, tests should still enforce numeric order so future parser changes do not silently scramble thumbnails.

### Data Flow

1. `list_available_maps` finds `.mmx`, `.yro`, `.map`, `.mpr`, and `.yrm` files.
2. `read_map_ini_for_metadata` extracts the real map INI, including MIX-wrapped map entries.
3. `read_map_menu_entry_from_ini` calls `parse_preview_section`.
4. `parse_preview_section` returns metadata plus decoded CPU RGBA pixels when possible.
5. Skirmish shell app/render state checks the currently selected `MapMenuEntry`.
6. If decoded preview data exists and selected index changed, upload RGBA pixels as a `BatchTexture`.
7. `build_skirmish_shell_instances` or adjacent render glue emits a `PreviewSurface` sprite fitted to `layout.map_preview`.
8. Start-marker sprites remain disabled unless `preview_source_bounds` is later populated from verified `[Header]` source fields.

### Error Handling

Decode failures should not prevent the menu from opening. The user-visible fallback is the current no-preview state.

Error categories worth distinguishing:

- Missing `[Preview] Size=`.
- Missing or empty `[PreviewPack]`.
- Invalid base64.
- LZO chunk decode failure.
- Decompressed byte count mismatch.
- Integer overflow in `width * height * 3` or `width * height * 4`.

Use a small `PreviewDecodeError` with `thiserror` if the map module already follows that pattern nearby; otherwise keep an internal error enum and convert to warning text at the app boundary. Do not use `anyhow` inside low-level map parsing APIs.

### Testing Strategy

Unit tests in `src/map/preview.rs`:

- `Size=80,50` returns `(80, 50)`.
- `Size=0,0,138,75` returns `(138, 75)`.
- `Size=` with fewer than two fields returns `None`.
- Empty `[PreviewPack]` keeps `has_packed_preview == false`.
- Non-empty `[PreviewPack]` with no valid size sets presence but no decoded preview.
- Synthetic encoded chunk decodes to the expected RGBA bytes and validates alpha 255.
- Wrong decompressed length returns a decode error and does not produce `DecodedPreview`.

Integration/visual checks:

- Run the skirmish shell with a known stock map such as Dustbowl.
- Confirm the preview rectangle shows the real map thumbnail instead of remaining blank.
- Compare the thumbnail against gamemd.exe or a captured reference to settle RGB vs BGR.
- Confirm no separate start-marker sprites appear for stock maps without verified `[Header]` preview source bounds.

### Determinism

No simulation state changes. Decode happens in menu/app setup only and does not affect lockstep game logic.

## Architectural Decisions

- Decode in `map/preview`, not render code, because `[Preview]` and `[PreviewPack]` are map-file data.
- Store CPU RGBA in menu metadata, not GPU textures, because `MapMenuEntry` is data and should remain independent of `wgpu`/render resource lifetimes.
- Upload selected preview texture in app/render state, following the existing `BatchTexture` pattern.
- Keep start-marker overlay disabled for this slice because the stock decoded preview already contains the visible start markers and the live overlay source bounds are not populated for stock maps.
- Keep channel order isolated in one conversion helper because the binary research did not resolve RGB vs BGR.

Tech debt:

- Up-front decode may be replaced by lazy decode/cache once the real modal Choose Map dialog is implemented.
- RGB/BGR must be visually verified before the task is considered final parity.

## Alternatives Considered

Lazy selected-map decode:

- Avoids decoding all map previews up front.
- Rejected for this slice because it scatters file/INI/MIX loading responsibility into render-time code and complicates state invalidation.

Dedicated preview cache service:

- Better long-term shape for a full map chooser with many visible thumbnails.
- Rejected for this slice because the current shell only needs the selected preview, and the existing metadata flow is already sufficient.

Render overlay start markers from `[Waypoints]`:

- Rejected for this slice because the latest research shows stock maps without verified `[Header]` source bounds should not draw separate `STARTBUT.SHP` overlays. Their starts are baked into the decoded preview image.
