# Skirmish PreviewPack Thumbnail Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Decode `[PreviewPack]` thumbnails from retail skirmish maps and render the selected map preview in the existing Skirmish setup shell.

**Architecture:** Map-file decode stays in `src/map/preview.rs`; app/render code owns GPU texture upload and drawing. The skirmish shell renders the decoded selected-map preview as a separate `BatchTexture` draw, while keeping start-marker overlays disabled unless verified source bounds exist.

**Design Doc:** `docs/plans/2026-05-20-skirmish-previewpack-thumbnail-design.md`

---

## Grounding Summary

The research docs already verify the major behavioral facts for this slice. `[Preview] Size=0,0,138,75` is a four-field rect whose third and fourth values are drawable thumbnail dimensions, and current Rust incorrectly reads the first two values as `(0,0)`. `[PreviewPack]` is INI-binary text containing LZO-compressed 3-byte pixels; Dustbowl verifies the raw byte count as `138 * 75 * 3 = 31050`. The writer loop is row-major, height outer loop and width inner loop.

`SKIRMISH_PREVIEW_OBJECT_LIFECYCLE_DAT_00AC1154_GHIDRA_REPORT.md` verifies the live selected-map preview loader path through `0x005E74E0 -> 0x00641EE0 -> 0x00641B00`. `SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md` verifies the packed format but leaves RGB/BGR channel order unresolved. `SKIRMISH_PREVIEW_SCENARIO_BOUNDS_STOCK_MAP_POPULATION_GHIDRA_REPORT.md` verifies that stock maps such as Dustbowl lack `[Header]`, so separate `STARTBUT.SHP` overlays should not be enabled just because a decoded preview exists. `SKIRMISH_GENERATE_TERRAIN_PREVIEW_BAKED_START_MARKERS_GHIDRA_REPORT.md` verifies that generated/saved previews can contain baked red markers, which decoding will naturally display as pixels.

Live Ghidra re-decompile was attempted in this session for `0x005E74E0`, `0x00641B00`, `0x00641EE0`, `0x00640710`, `0x00641140`, `0x006418B0`, and `0x006AE3F0`, but the MCP session did not resolve those function addresses. No contradiction was found; this plan relies on the verified research reports above for binary grounding.

Codegraph confirms the current Rust state: `PreviewSection` only has `size` and `has_packed_preview`, `parse_preview_section` only parses metadata, and `BatchRenderer::create_texture` already uploads RGBA data for sprite drawing. `IniSection::get_values()` already returns numbered INI values sorted by numeric key, which is the right pattern for `[PreviewPack]`.

No `rules.ini`, `rulesmd.ini`, `art.ini`, or `artmd.ini` constants drive this feature. The input is per-map INI data: `[Preview] Size=` and numbered `[PreviewPack]` values.

Still unknown after grounding: PreviewPack serialized channel order, RGB vs BGR. The implementation must isolate this in one helper and visually verify Dustbowl against gamemd.exe before claiming final parity.

## Key Technical Decisions

- Decode during map metadata load and store CPU RGBA in `PreviewSection` — **Confidence:** high
  - **Source:** design doc; repo pattern `read_map_menu_entry_from_ini -> parse_preview_section`; Codegraph current-state check.
- Use `IniSection::get_values()` to collect `[PreviewPack]` lines in numeric order — **Confidence:** high
  - **Source:** repo pattern `src/rules/ini_parser.rs`; PreviewPack report says pack data is split across numbered INI keys.
- Validate decompressed byte count as `width * height * 3` before converting to RGBA — **Confidence:** high
  - **Source:** `SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md`.
- Default initial conversion to RGB byte order but keep all channel ordering in one helper — **Confidence:** low
  - **Source:** PreviewPack report verifies 3-byte pixels but explicitly leaves RGB/BGR unresolved. Flag for `/review-plan` and visual verification.
- Draw decoded preview surface without enabling `STARTBUT.SHP` overlays — **Confidence:** high
  - **Source:** `SKIRMISH_PREVIEW_SCENARIO_BOUNDS_STOCK_MAP_POPULATION_GHIDRA_REPORT.md`; design scope excludes overlay source-bound work.

## Open Questions

### Resolved During Planning

- Should the first slice include the modal Choose Map behavior? Resolution: no; user explicitly chose preview decode/render only.
- Should stock decoded previews automatically draw separate start-marker overlays? Resolution: no; stock maps without verified `[Header]` source bounds rely on baked preview pixels for this slice.
- Are there rules/art INI constants to parse? Resolution: no; this feature uses per-map `[Preview]` and `[PreviewPack]` sections only.

### Deferred to Implementation

- PreviewPack channel order: implement one conversion helper with an RGB default, run visual comparison on Dustbowl, then flip that helper if the colors are visibly swapped.
- Exact stretch/blit parity inside the preview child: live `DrawStartPositions @ 0x00640710` aspect-fits the preview surface into child `0x468`. This plan must fit the decoded thumbnail inside `layout.map_preview` while preserving aspect ratio, then use visual comparison to confirm the integer rounding.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/map/preview.rs` | Parse preview dimensions, decode `[PreviewPack]`, expose CPU RGBA preview pixels, and unit-test pure decode logic. |
| Modify | `src/app_init.rs` | Keep `MapMenuEntry` carrying the expanded `PreviewSection`. |
| Modify | `src/app_list_maps.rs` | Continue populating `MapMenuEntry.preview` through the metadata load path; no duplicate map probing. |
| Modify | `src/app.rs` | Expose `available_maps` to the app render module and store a selected-preview GPU texture cache. |
| Modify | `src/app_skirmish_shell_render.rs` | Upload the selected decoded preview, draw it as a separate texture, and decouple preview-surface draw order from start-marker overlays. |

## Interface Changes

- `PreviewSection` gains `decoded: Option<DecodedPreview>`.
- New `DecodedPreview` struct in `src/map/preview.rs` contains `width`, `height`, and `rgba`.
- New `PreviewDecodeError` in `src/map/preview.rs` reports decode failures for tests/logging.
- New helper `decode_preview_pack(ini, width, height)` in `src/map/preview.rs`.
- `AppState.available_maps` becomes `pub(crate)` so `app_skirmish_shell_render` can read the selected map's decoded preview.
- `AppState` gains `skirmish_preview_texture: Option<SkirmishPreviewTexture>`.
- `skirmish_shell_semantic_draw_order` splits preview availability from start-marker overlay availability.

## Sim Checklist

Not applicable. This plan does not touch `sim/`.

## Risk Areas

- Channel order can produce obviously wrong colors if RGB/BGR is guessed wrong.
- The current draw-order helper couples `PreviewSurface`, `StartMarker`, and `StartMarkerLabel`; enabling the preview without splitting that gate would add incorrect overlays.
- Holding a `BatchTexture` cache inside `AppState` must be invalidated when `selected_map_idx` changes.
- `available_maps` currently is private to `app.rs`; render integration must either expose it `pub(crate)` or add a getter. This plan uses `pub(crate)` to match the surrounding `AppState` field style.
- Render-pass integration must avoid drawing the preview with the chrome atlas texture. The preview is its own `BatchTexture` and needs its own draw call.
- Preview pass ordering must be explicit: `draw_with_buffer_passthrough` uses a depth-compare-always pipeline, so `SHELL_PREVIEW_SURFACE_DEPTH` does not order the preview relative to separate atlas/text draw calls.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 2 | Four-field `[Preview] Size=` uses fields 3 and 4 | Stock maps such as Dustbowl otherwise decode as `0x0` and never render a real thumbnail | Unit test `Size=0,0,138,75`; compare decoded Dustbowl dimensions |
| Task 4 | PreviewPack LZO/base64 decode and exact byte-count validation | Wrong decode produces blank or scrambled thumbnails in normal Skirmish setup | Unit test with synthetic chunk; run Dustbowl and confirm nonblank preview |
| Task 4 | Row-major 3-byte pixel conversion | Swapped row/stride logic visibly scrambles the thumbnail | Unit test two-pixel row and Dustbowl visual check |
| Task 4 | RGB/BGR isolation | Wrong channel order visibly shifts map colors | Visual comparison against gamemd.exe; one helper controls channel order |
| Task 8 | PreviewSurface does not enable `STARTBUT.SHP` overlays | Stock maps without `[Header]` would show duplicate or misplaced start markers | Semantic draw-order test: preview true, marker overlay false |
| Task 9 | Preview rendered from its own texture aspect-fitted inside `layout.map_preview` | Stretching Dustbowl `138x75` to the full `144x112` child would visibly distort the thumbnail versus gamemd.exe | In-game visual check in the dev skirmish shell |

---

## Tasks

### Task 1: Add Decoded Preview Types

**Why:** Establish the map-layer data contract before decode and render code consume it.

**Files:**

- Modify: `src/map/preview.rs`

**Pattern:** Existing map data structs in `src/map/*`; low-level map types own data only and do not hold GPU resources.

**Step 1: Add imports**

```rust
use std::fmt;

use crate::util::lzo::LzoError;
```

**Step 2: Add `DecodedPreview`**

```rust
/// CPU RGBA thumbnail decoded from `[PreviewPack]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPreview {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
```

**Step 3: Extend `PreviewSection`**

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviewSection {
    /// Size metadata from `[Preview] Size=` when present.
    pub size: Option<(u32, u32)>,
    /// True if `[PreviewPack]` exists and contains data.
    pub has_packed_preview: bool,
    /// Decoded CPU-side preview thumbnail when the map carries a valid pack.
    pub decoded: Option<DecodedPreview>,
}
```

**Step 4: Add `PreviewDecodeError`**

Use a local enum so `map/preview` does not depend on `anyhow`.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewDecodeError {
    MissingPreviewPack,
    InvalidBase64(String),
    Lzo(String),
    PixelByteCount { expected: usize, actual: usize },
    PixelBufferTooLarge,
}

impl fmt::Display for PreviewDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPreviewPack => write!(f, "missing or empty [PreviewPack]"),
            Self::InvalidBase64(err) => write!(f, "invalid PreviewPack base64: {err}"),
            Self::Lzo(err) => write!(f, "invalid PreviewPack LZO stream: {err}"),
            Self::PixelByteCount { expected, actual } => {
                write!(f, "PreviewPack byte count {actual} did not match expected {expected}")
            }
            Self::PixelBufferTooLarge => write!(f, "PreviewPack dimensions overflowed"),
        }
    }
}

impl std::error::Error for PreviewDecodeError {}

impl From<LzoError> for PreviewDecodeError {
    fn from(value: LzoError) -> Self {
        Self::Lzo(value.to_string())
    }
}
```

**Step 5: Update existing struct literals**

Update the `PreviewSection` literal in `parse_preview_section` to include `decoded: None` until Task 5 wires decoding.

**Step 6: Verify**

Run:

```powershell
cargo test parse_preview_metadata empty_preview_pack_is_not_counted -- --nocapture
```

Expected: both existing tests pass after adding the new `decoded` field.

### Task 2: Fix Preview Size Parsing

**Why:** Four-field stock-map sizes currently parse as `(0,0)`, which prevents correct decode dimensions.

**Files:**

- Modify: `src/map/preview.rs`

**Pattern:** Pure helper function plus same-file unit tests.

**Step 1: Replace `parse_preview_size`**

```rust
fn parse_preview_size(value: &str) -> Option<(u32, u32)> {
    let parts: Vec<u32> = value
        .split(',')
        .map(str::trim)
        .map(str::parse::<u32>)
        .collect::<Result<Vec<u32>, _>>()
        .ok()?;

    match parts.as_slice() {
        [width, height] => Some((*width, *height)),
        [_, _, width, height, ..] => Some((*width, *height)),
        _ => None,
    }
}
```

**Step 2: Update `parse_preview_metadata`**

Add `assert_eq!(preview.decoded, None);` so the existing metadata test confirms decode remains absent for invalid synthetic data.

**Step 3: Add a four-field test**

```rust
#[test]
fn parse_preview_size_uses_rect_dimensions() {
    let ini = IniFile::from_str("[Preview]\nSize=0,0,138,75\n");
    let preview = parse_preview_section(&ini);
    assert_eq!(preview.size, Some((138, 75)));
}
```

**Step 4: Add a malformed-size test**

```rust
#[test]
fn parse_preview_size_rejects_single_value() {
    let ini = IniFile::from_str("[Preview]\nSize=138\n");
    let preview = parse_preview_section(&ini);
    assert_eq!(preview.size, None);
}
```

**Step 5: Verify**

Run:

```powershell
cargo test preview_size -- --nocapture
```

Expected: the two-field, four-field, and malformed size cases pass.

### Task 3: Collect PreviewPack Lines in Numeric Order

**Why:** PreviewPack data is split across numbered INI keys, and scrambled order corrupts the byte stream.

**Files:**

- Modify: `src/map/preview.rs`

**Pattern:** Reuse `IniSection::get_values()`, which already sorts numeric keys for RA2 numbered sections.

**Step 1: Add helper**

```rust
fn preview_pack_text(ini: &IniFile) -> Option<String> {
    let section = ini.section("PreviewPack")?;
    let values = section.get_values();
    if values.iter().all(|value| value.trim().is_empty()) {
        return None;
    }
    Some(values.concat())
}
```

**Step 2: Use helper for presence**

In `parse_preview_section`, replace the current `has_packed_preview` scan with:

```rust
let pack_text = preview_pack_text(ini);
let has_packed_preview = pack_text.is_some();
```

Keep `pack_text` local for Task 5.

**Step 3: Add numeric ordering test**

```rust
#[test]
fn preview_pack_text_uses_numeric_key_order() {
    let ini = IniFile::from_str("[PreviewPack]\n2=BBB\n10=CCC\n1=AAA\n");
    assert_eq!(preview_pack_text(&ini).as_deref(), Some("AAABBBCCC"));
}
```

**Step 4: Add empty numeric pack test**

```rust
#[test]
fn preview_pack_text_rejects_empty_numbered_values() {
    let ini = IniFile::from_str("[PreviewPack]\n1=\n2=\n");
    assert_eq!(preview_pack_text(&ini), None);
}
```

**Step 5: Verify**

Run:

```powershell
cargo test preview_pack_text -- --nocapture
```

Expected: numeric ordering and empty-pack tests pass.

### Task 4: Implement PreviewPack Decode

**Why:** This is the pure data transform from map INI payload to CPU RGBA pixels.

**Files:**

- Modify: `src/map/preview.rs`

**Pattern:** Low-level map parser delegates reusable binary transforms to `util/base64` and `util/lzo`.

**Step 1: Add imports**

```rust
use crate::util::base64::base64_decode;
use crate::util::lzo::decompress_chunks;
```

**Step 2: Add channel order helper**

Keep channel ordering centralized. The default is RGB until the visual check proves otherwise.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewChannelOrder {
    Rgb,
    Bgr,
}

const PREVIEW_CHANNEL_ORDER: PreviewChannelOrder = PreviewChannelOrder::Rgb;

fn push_rgba_from_preview_pixel(out: &mut Vec<u8>, pixel: &[u8]) {
    match PREVIEW_CHANNEL_ORDER {
        PreviewChannelOrder::Rgb => out.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]),
        PreviewChannelOrder::Bgr => out.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]),
    }
}
```

**Step 3: Add byte-count helper**

```rust
fn expected_preview_rgb_len(width: u32, height: u32) -> Result<usize, PreviewDecodeError> {
    let pixels = width
        .checked_mul(height)
        .ok_or(PreviewDecodeError::PixelBufferTooLarge)?;
    let bytes = pixels
        .checked_mul(3)
        .ok_or(PreviewDecodeError::PixelBufferTooLarge)?;
    usize::try_from(bytes).map_err(|_| PreviewDecodeError::PixelBufferTooLarge)
}
```

**Step 4: Add decode function**

```rust
pub fn decode_preview_pack(
    pack_text: &str,
    width: u32,
    height: u32,
) -> Result<DecodedPreview, PreviewDecodeError> {
    let encoded = pack_text.trim();
    if encoded.is_empty() {
        return Err(PreviewDecodeError::MissingPreviewPack);
    }

    let compressed = base64_decode(encoded).map_err(PreviewDecodeError::InvalidBase64)?;
    let rgb = decompress_chunks(&compressed)?;
    let expected = expected_preview_rgb_len(width, height)?;
    if rgb.len() != expected {
        return Err(PreviewDecodeError::PixelByteCount {
            expected,
            actual: rgb.len(),
        });
    }

    let rgba_len = usize::try_from(
        width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(PreviewDecodeError::PixelBufferTooLarge)?,
    )
    .map_err(|_| PreviewDecodeError::PixelBufferTooLarge)?;
    let mut rgba = Vec::with_capacity(rgba_len);
    for pixel in rgb.chunks_exact(3) {
        push_rgba_from_preview_pixel(&mut rgba, pixel);
    }

    Ok(DecodedPreview {
        width,
        height,
        rgba,
    })
}
```

**Step 5: Add synthetic decode test**

The base64 string below is one chunk with a literal-only LZO stream for two RGB pixels: `[1,2,3]` and `[4,5,6]`.

```rust
#[test]
fn decode_preview_pack_literal_chunk_to_rgba() {
    let preview = decode_preview_pack("CgAGABcBAgMEBQYRAAA=", 2, 1).expect("valid preview");
    assert_eq!(preview.width, 2);
    assert_eq!(preview.height, 1);
    assert_eq!(preview.rgba, vec![1, 2, 3, 255, 4, 5, 6, 255]);
}
```

**Step 6: Add byte-count mismatch test**

```rust
#[test]
fn decode_preview_pack_rejects_wrong_byte_count() {
    let err = decode_preview_pack("CgAGABcBAgMEBQYRAAA=", 1, 1).unwrap_err();
    assert_eq!(
        err,
        PreviewDecodeError::PixelByteCount {
            expected: 3,
            actual: 6,
        }
    );
}
```

**Step 7: Verify**

Run:

```powershell
cargo test decode_preview_pack -- --nocapture
```

Expected: valid synthetic chunk converts to RGBA with alpha 255; wrong dimensions return `PixelByteCount`.

### Task 5: Wire Decode into `parse_preview_section`

**Why:** `MapMenuEntry.preview` must carry decoded CPU pixels through the existing metadata flow.

**Files:**

- Modify: `src/map/preview.rs`
- Verify only: `src/app_list_maps.rs`

**Pattern:** `read_map_menu_entry_from_ini` already calls `parse_preview_section`; keep that interface and avoid duplicating file loading.

**Step 1: Update `parse_preview_section` locals**

```rust
pub fn parse_preview_section(ini: &IniFile) -> PreviewSection {
    let size = ini
        .section("Preview")
        .and_then(|section| section.get("Size"))
        .and_then(parse_preview_size);

    let pack_text = preview_pack_text(ini);
    let has_packed_preview = pack_text.is_some();
    let decoded = match (size, pack_text.as_deref()) {
        (Some((width, height)), Some(text)) => match decode_preview_pack(text, width, height) {
            Ok(preview) => Some(preview),
            Err(err) => {
                log::warn!("Failed to decode map PreviewPack {width}x{height}: {err}");
                None
            }
        },
        _ => None,
    };

    PreviewSection {
        size,
        has_packed_preview,
        decoded,
    }
}
```

**Step 2: Add parse integration test**

```rust
#[test]
fn parse_preview_section_decodes_valid_pack() {
    let ini = IniFile::from_str(
        "[Preview]\nSize=0,0,2,1\n[PreviewPack]\n1=CgAGABcBAgMEBQYRAAA=\n",
    );
    let preview = parse_preview_section(&ini);
    let decoded = preview.decoded.expect("decoded preview");
    assert_eq!(decoded.width, 2);
    assert_eq!(decoded.height, 1);
    assert_eq!(decoded.rgba, vec![1, 2, 3, 255, 4, 5, 6, 255]);
}
```

**Step 3: Add non-fatal invalid-pack test**

```rust
#[test]
fn parse_preview_section_keeps_invalid_pack_nonfatal() {
    let ini = IniFile::from_str("[Preview]\nSize=2,1\n[PreviewPack]\n1=not valid base64!\n");
    let preview = parse_preview_section(&ini);
    assert_eq!(preview.size, Some((2, 1)));
    assert!(preview.has_packed_preview);
    assert_eq!(preview.decoded, None);
}
```

**Step 4: Verify `app_list_maps` needs no decode-specific change**

Read `src/app_list_maps.rs` and confirm `read_map_menu_entry_from_ini` still assigns:

```rust
preview: crate::map::preview::parse_preview_section(&ini),
```

No additional map-file probing should be added there.

**Step 5: Verify**

Run:

```powershell
cargo test parse_preview_section -- --nocapture
```

Expected: metadata, valid decode, and invalid non-fatal cases pass.

### Task 6: Add Selected Preview Texture Cache to App State

**Why:** The renderer needs a GPU texture for the selected map, but map metadata must stay CPU-only.

**Files:**

- Modify: `src/app.rs`
- Modify: `src/app_skirmish_shell_render.rs`

**Pattern:** `AppState` already owns render resources such as `skirmish_shell_chrome`, `main_menu_movie`, and `minimap`.

**Step 1: Define cache type**

In `src/app_skirmish_shell_render.rs`, add near the top-level helper types:

```rust
pub(crate) struct SkirmishPreviewTexture {
    pub selected_map_idx: usize,
    pub texture: crate::render::batch::BatchTexture,
    pub width: u32,
    pub height: u32,
}
```

**Step 2: Expose available maps and add cache field**

In `src/app.rs`, change:

```rust
available_maps: Vec<MapMenuEntry>,
```

to:

```rust
pub(crate) available_maps: Vec<MapMenuEntry>,
```

Then add this field near `skirmish_shell_chrome`:

```rust
pub(crate) skirmish_preview_texture:
    Option<crate::app_skirmish_shell_render::SkirmishPreviewTexture>,
```

**Step 3: Initialize cache**

In `AppState::new`, initialize:

```rust
skirmish_preview_texture: None,
```

place it next to `skirmish_shell_chrome`.

**Step 4: Verify**

Run:

```powershell
cargo check
```

Expected: only missing-use warnings or no warnings; no privacy or initialization errors.

### Task 7: Upload Selected Decoded Preview Texture

**Why:** The selected map's CPU RGBA data must become a `BatchTexture` before drawing.

**Files:**

- Modify: `src/app_skirmish_shell_render.rs`

**Pattern:** Existing render resource creation uses `BatchRenderer::create_texture(&state.gpu, rgba, width, height)`.

**Step 1: Add helper to detect cached selected texture**

```rust
fn selected_preview_texture_is_current(state: &AppState, selected_map_idx: usize) -> bool {
    state
        .skirmish_preview_texture
        .as_ref()
        .is_some_and(|cached| cached.selected_map_idx == selected_map_idx)
}
```

**Step 2: Add texture refresh helper**

```rust
fn ensure_selected_preview_texture(state: &mut AppState) {
    let selected_map_idx = state.skirmish_shell_state.selected_map_idx;
    if selected_preview_texture_is_current(state, selected_map_idx) {
        return;
    }

    let decoded = state
        .available_maps
        .get(selected_map_idx)
        .and_then(|entry| entry.preview.decoded.as_ref());

    let Some(decoded) = decoded else {
        state.skirmish_preview_texture = None;
        return;
    };

    let texture = state.batch_renderer.create_texture(
        &state.gpu,
        &decoded.rgba,
        decoded.width,
        decoded.height,
    );
    state.skirmish_preview_texture = Some(SkirmishPreviewTexture {
        selected_map_idx,
        texture,
        width: decoded.width,
        height: decoded.height,
    });
}
```

**Step 3: Call helper before instance-buffer creation**

In `render_skirmish_shell_with_atlas`, after computing `layout` and after confirming `atlas` exists, call:

```rust
ensure_selected_preview_texture(state);
```

**Step 4: Add a narrow cache unit test only if construction can avoid GPU**

Do not create fake `GpuContext`. This helper is GPU-bound, so rely on `cargo check` and the visual verification task for runtime behavior.

**Step 5: Verify**

Run:

```powershell
cargo check
```

Expected: no borrow checker errors around `state.available_maps`, `state.batch_renderer`, or `state.skirmish_preview_texture`.

### Task 8: Split Preview Surface and Start-Marker Draw Gates

**Why:** The decoded preview should become visible, but separate `STARTBUT.SHP` markers must remain disabled for stock maps without verified source bounds.

**Files:**

- Modify: `src/app_skirmish_shell_render.rs`

**Pattern:** Existing `skirmish_shell_semantic_draw_order` tests encode recovered shell draw ordering.

**Step 1: Remove or stop using `real_preview_surface_available()`**

Do not keep a hardcoded `false` gate for the preview surface.

**Step 2: Change draw-order signature**

Change:

```rust
pub fn skirmish_shell_semantic_draw_order(
    layout: &SkirmishShellLayout,
    overlay_frame10_active: bool,
    real_preview_surface_available: bool,
    flag_count: usize,
) -> Vec<SkirmishShellDrawRole>
```

to:

```rust
pub fn skirmish_shell_semantic_draw_order(
    layout: &SkirmishShellLayout,
    overlay_frame10_active: bool,
    preview_surface_available: bool,
    start_marker_overlay_available: bool,
    flag_count: usize,
) -> Vec<SkirmishShellDrawRole>
```

**Step 3: Split role insertion**

```rust
if preview_surface_available {
    roles.push(SkirmishShellDrawRole::PreviewSurface);
}
if start_marker_overlay_available {
    roles.push(SkirmishShellDrawRole::StartMarker);
    roles.push(SkirmishShellDrawRole::StartMarkerLabel);
}
```

**Step 4: Keep marker sprite emission disabled**

In `build_skirmish_shell_instances`, keep:

```rust
push_start_marker_sprites(
    &mut instances,
    atlas,
    layout.map_preview,
    &[],
    false,
    0.00056,
);
```

This preserves the design decision that PreviewPack decode alone does not imply overlay markers.

**Step 5: Update tests**

Update existing call sites in tests to pass the new marker flag.

Add this test:

```rust
#[test]
fn decoded_preview_surface_does_not_imply_start_marker_overlays() {
    let layout = compute_layout(800, 600);
    let order = skirmish_shell_semantic_draw_order(&layout, false, true, false, 0);
    assert!(order.contains(&SkirmishShellDrawRole::PreviewSurface));
    assert!(!order.contains(&SkirmishShellDrawRole::StartMarker));
    assert!(!order.contains(&SkirmishShellDrawRole::StartMarkerLabel));
}
```

**Step 6: Verify**

Run:

```powershell
cargo test skirmish_shell_semantic_draw_order decoded_preview_surface -- --nocapture
```

Expected: semantic order tests pass and preview/marker gates are independent.

### Task 9: Draw the Preview Texture in the Skirmish Shell Pass

**Why:** The player-visible result is the selected map thumbnail aspect-fitted inside `layout.map_preview`.

**Files:**

- Modify: `src/app_skirmish_shell_render.rs`

**Pattern:** Draw a separate `BatchTexture` with its own `SpriteInstance` buffer, matching existing batch renderer usage.

**Step 1: Add preview depth constant**

Place near shell depth constants:

```rust
const SHELL_PREVIEW_SURFACE_DEPTH: f32 = 0.00058;
```

This is only the instance depth used inside the preview texture draw. The current shell draw path uses `draw_with_buffer_passthrough`, whose pipeline bypasses depth ordering, so future `STARTBUT.SHP` overlays must be drawn in a later draw call if they are enabled.

**Step 2: Add aspect-fit helper and preview instance builder**

```rust
fn aspect_fit_rect(dst: RectPx, src_w: u32, src_h: u32) -> RectPx {
    if dst.w <= 0 || dst.h <= 0 || src_w == 0 || src_h == 0 {
        return RectPx::new(dst.x, dst.y, 0, 0);
    }

    let scale_w = dst.w as f32 / src_w as f32;
    let scale_h = dst.h as f32 / src_h as f32;
    let scale = scale_w.min(scale_h);
    let fitted_w = (src_w as f32 * scale).round() as i32;
    let fitted_h = (src_h as f32 * scale).round() as i32;
    RectPx::new(
        dst.x + ((dst.w - fitted_w) / 2),
        dst.y + ((dst.h - fitted_h) / 2),
        fitted_w,
        fitted_h,
    )
}

fn build_preview_surface_instance(
    layout: &SkirmishShellLayout,
    preview_width: u32,
    preview_height: u32,
) -> Option<SpriteInstance> {
    let fitted = aspect_fit_rect(layout.map_preview, preview_width, preview_height);
    if fitted.w <= 0 || fitted.h <= 0 {
        return None;
    }

    Some(SpriteInstance {
        position: [fitted.x as f32, fitted.y as f32],
        size: [fitted.w as f32, fitted.h as f32],
        uv_origin: [0.0, 0.0],
        uv_size: [1.0, 1.0],
        depth: SHELL_PREVIEW_SURFACE_DEPTH,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    })
}
```

Add a unit test for Dustbowl's known `138x75` preview in the current `800x600` shell layout. The fitted rect must preserve aspect ratio and remain centered inside `layout.map_preview`; visual verification remains responsible for confirming exact gamemd.exe rounding.

**Step 3: Create preview instance buffer**

In `render_skirmish_shell_with_atlas`, after `ensure_selected_preview_texture(state);`, build:

```rust
let preview_instance = state
    .skirmish_preview_texture
    .as_ref()
    .and_then(|preview| build_preview_surface_instance(&layout, preview.width, preview.height));
let preview_buffer = preview_instance
    .as_ref()
    .and_then(|instance| state.batch_renderer.create_instance_buffer(&state.gpu, &[*instance]));
```

**Step 4: Draw the preview texture after chrome**

After drawing `atlas.texture` and before text drawing, add:

```rust
if let (Some(preview), Some((buffer, count))) =
    (state.skirmish_preview_texture.as_ref(), preview_buffer.as_ref())
{
    state.batch_renderer.draw_with_buffer_passthrough(
        &mut pass,
        &preview.texture,
        buffer,
        *count,
    );
}
```

Keep text drawing after this block as currently ordered, so text remains visible. Do not rely on `SHELL_PREVIEW_SURFACE_DEPTH` to order this draw relative to the chrome or future marker overlays; the passthrough pipeline draws in call order.

**Step 5: Preserve blank fallback**

If `skirmish_preview_texture` is `None`, skip the extra draw call. Do not create a placeholder texture.

**Step 6: Verify**

Run:

```powershell
cargo check
```

Expected: no borrow checker errors in render pass setup or draw ordering.

### Task 10: Full Test and Visual Verification

**Why:** Unit tests prove pure decode behavior; visual verification catches channel order, stretch, and render integration issues.

**Files:**

- Verify only.

**Pattern:** Existing project uses unit tests plus visual checks for shell parity.

**Step 1: Run focused tests**

```powershell
cargo test preview -- --nocapture
```

Expected: all `src/map/preview.rs` tests pass.

**Step 2: Run skirmish shell tests**

```powershell
cargo test skirmish_shell -- --nocapture
```

Expected: layout, hit-test, and semantic draw-order tests pass.

**Step 3: Run full check**

```powershell
cargo check
```

Expected: the crate checks successfully.

**Step 4: Run the development skirmish shell**

Use the existing dev shell path:

```powershell
$env:RA2_DEV_SKIRMISH_SHELL='1'; cargo run
```

Expected visible result: the Skirmish setup shell shows the selected map's real thumbnail in the map preview rectangle instead of a blank area.

**Step 5: Verify Dustbowl channel order**

Select or cycle to Dustbowl if present. Compare against a gamemd.exe screenshot or known retail preview capture:

- If terrain colors look correct, leave `PREVIEW_CHANNEL_ORDER` as `Rgb`.
- If red/blue channels are swapped, change only `PREVIEW_CHANNEL_ORDER` to `PreviewChannelOrder::Bgr`, rerun `cargo test preview -- --nocapture`, and rerun the visual check.

**Step 6: Verify overlays remain disabled**

On a stock map without `[Header]`, confirm no separate `STARTBUT.SHP` numbered markers appear on top of the decoded thumbnail. Baked red pixels inside the preview image may be visible; those are part of the decoded thumbnail and should remain.

## Sources & References

- **Design doc:** `docs/plans/2026-05-20-skirmish-previewpack-thumbnail-design.md`
- **Ghidra reports:**
  - `docs/research/SKIRMISH_MAP_PREVIEW_SOURCE_BOUNDS_AND_PREVIEWPACK_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_PREVIEW_OBJECT_LIFECYCLE_DAT_00AC1154_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_PREVIEW_SCENARIO_BOUNDS_STOCK_MAP_POPULATION_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_GENERATE_TERRAIN_PREVIEW_BAKED_START_MARKERS_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`
  - `docs/research/traces/SKIRMISH_MAP_PREVIEW_START_MARKERS_TRACE.md`
- **gamemd.exe addresses cited by reports:** `0x005E74E0`, `0x00641B00`, `0x00641EE0`, `0x00640710`, `0x00641140`, `0x006418B0`, `0x006AE3F0`, `0x006ACEE0`.
- **Map INI sections:** `[Preview] Size=left,top,width,height`; numbered `[PreviewPack]` values.
- **Related code:**
  - `src/map/preview.rs`
  - `src/app_init.rs`
  - `src/app_list_maps.rs`
  - `src/app.rs`
  - `src/app_skirmish_shell_render.rs`
  - `src/rules/ini_parser.rs`
  - `src/util/base64.rs`
  - `src/util/lzo.rs`
  - `src/render/batch.rs`
