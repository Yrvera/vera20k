# Loading Progress-Row Closure Implementation Plan

**Goal:** Make the standard offline Skirmish loading progress row
retail-convincing by removing the opaque magenta country-insignia rectangle,
restoring the first-player-name label, and deriving the bar/icon/label geometry
from the actual GAME.FNT height without changing the verified progress cadence.

**Approved design:**
`docs/plans/2026-07-27-loading-progress-row-closure-design.md`.

**Architecture:** Add one small CPU-only `app_loading_progress_row` module that
owns the immutable row label and integer layout contract. Keep PCX decoding and
atlas packing in `render/loading_screen_chrome`; keep draw orchestration in
`app_loading`. Build one row layout per presentation and feed it to both the
ordinary loading-frame path and the synchronous per-milestone presenter.

**Truth boundary:** This closes the common-path visible row contract. It does
not claim native/Rust pixel parity, exact DirectDraw presentation mechanics,
exact 3% pixel equivalence, or exact milestone dwell. Those remain
`UNCHECKED`/`UNVERIFIED`.

## Task contract

- **Representative player scenario:** Start an ordinary stock Skirmish as
  America, Russia, or Yuri at 640x480 or 800x600. From the first visible
  progress state through 100%, the country insignia has no magenta rectangle,
  the local player name appears to its right, and all row elements remain
  aligned and unobscured.
- **Necessary scope:** `src/app_loading_progress_row.rs` (new), `src/lib.rs`,
  `src/app_loading.rs`, `src/render/loading_screen_chrome.rs`,
  `src/assets/pcx_file.rs` tests, and the progress-row order entry/test in
  `src/app_loading_composition.rs`.
- **Non-deferrable constraints:** use RGB magenta rather than a palette-index
  key; source the label from the immutable launch-session `player_name`; use
  GAME.FNT height; preserve native integer truncation and the existing discrete
  progress gate; share one draw plan across both presentation paths; keep
  `sim/` untouched.
- **Smallest production validation:** focused unit tests, the loading-related
  test slice, one serial `cargo check -q`, and a visual matrix covering
  America/Russia/Yuri plus selected/random map paths at the two width
  breakpoints.
- **Residual risks:** exact pixels and timing remain unverified; missing retail
  icon art follows the verified no-icon layout but is not a normal stock
  scenario.
- **Stop condition:** all focused tests and `cargo check -q` pass, no unrelated
  diff appears, and the visual matrix shows no opaque magenta background,
  missing/incorrect player name, overlap, clipping, or divergence between the
  first frame and later progress repaints.

## Verified evidence and current anchors

- `docs/research/PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md`
  records the load-bearing contract:
  - country PCX transparent key is RGB `(255,0,255)`;
  - row height is `max(icon_h, bar_h + 6, GAME_FNT_h) + 4`;
  - bar pixels start at `base_x + 8` and the verified centered y;
  - icon x is `base_x + bar_w + 0x15`;
  - with an icon, label x is `icon_x + icon_w + 10`;
  - without an icon, label x is the would-be icon x;
  - label y is `row_y + (row_h - GAME_FNT_h) / 2`;
  - label right edge is `base_x + width_override - 3`;
  - the label is the first session-node display name, left aligned, with no
    backing.
- Live read-only Ghidra evidence used by the design:
  `0x00643AE0`, `0x00643720`, `0x00643670`, `0x004355D0`,
  `0x004A60D0`, and `0x004A61C0`.
- Current Rust anchors:
  - `PcxFile::to_rgba_with_color_key` already provides the required decoder;
  - `LoadingArtVariant::side_icon_pcx` already owns the verified country mapping;
  - `BitFont::cell_height()` already exposes the loaded GAME.FNT metric;
  - `SkirmishLaunchSession::player_name` is available before the first loading
    frame;
  - `build_native_loading_instances` and
    `build_native_loading_text_draws` currently split row construction;
  - `present_native_loading` is used by both synchronous progress routes.

## Execution preflight

1. Re-read the approved design and the research report sections 5, 6, 11, and
   12.
2. Record `git rev-parse HEAD`, `git status --short`, and
   `git diff --stat`. The planning observation was `dev@41493066` with active
   uncommitted loading work, including an untracked
   `src/app_loading_composition.rs`; treat every pre-existing hunk as
   user/other-session work.
3. Re-run:

   ```powershell
   rg -n "SIDE_ICON_TRANSPARENT|standard_skirmish_row_height|build_native_loading_instances|build_native_loading_text_draws|present_native_loading|RenderingProgressSink|ProgressSideIcon" src
   ```

   Re-anchor the edits if another session changed these seams.
4. Check build ownership:

   ```powershell
   Get-Process cargo,rustc -ErrorAction SilentlyContinue |
       Select-Object ProcessName,Id,CPU
   ```

   Do not start Cargo while another task owns it. All Cargo commands in this
   plan are serial.

## Task 1: Lock the RGB-magenta PCX contract with a decoder regression

**Files:**

- Modify: `src/assets/pcx_file.rs`
- Modify: `src/render/loading_screen_chrome.rs`

### Step 1: Strengthen the palette-decoder test

Replace
`rgba_color_key_applies_after_embedded_palette_conversion` with this exact
fixture so palette index `0` is explicitly proven opaque when its RGB value is
not magenta:

```rust
#[test]
fn rgba_color_key_uses_rgb_not_palette_index() {
    let pcx = PcxFile {
        width: 3,
        height: 1,
        pixels: vec![1, 0, 3],
        palette: {
            let mut palette = [[0u8; 3]; 256];
            palette[0] = [0, 0, 0];
            palette[1] = [255, 0, 255];
            palette[3] = [255, 0, 255];
            palette
        },
        direct_rgb: false,
    };

    assert_eq!(
        pcx.to_rgba_with_color_key([255, 0, 255]),
        vec![
            255, 0, 255, 0, //
            0, 0, 0, 255, //
            255, 0, 255, 0,
        ]
    );
}
```

Keep `direct_rgb_color_key_applies_after_rgb_conversion`; it covers the
three-plane PCX path.

### Step 2: Run the focused test and require the expected pre-fix behavior

```powershell
cargo test -q rgba_color_key_uses_rgb_not_palette_index
```

The decoder test should pass before the production call-site change because the
RGB-key API already exists. Its role is to prevent a future regression back to
palette-index semantics.

### Step 3: Change only the loading country-icon decode call

In `src/render/loading_screen_chrome.rs`, replace the palette-index constant and
decode:

```rust
/// Native converts this RGB key to the active DirectDraw format before blitting
/// the standard Skirmish country insignia.
const SIDE_ICON_TRANSPARENT_RGB: [u8; 3] = [255, 0, 255];
```

```rust
/// Decode a country-insignia PCX into an atlas entry using the native
/// RGB-magenta transparency key.
fn render_pcx_entry(assets: &AssetManager, file_name: &str) -> Option<RenderedLoadingEntry> {
    let bytes = assets.get_ref(file_name)?;
    let pcx = PcxFile::from_bytes(bytes)
        .map_err(|err| {
            log::warn!("Could not parse standard Skirmish loading side icon {file_name}: {err:#}");
            err
        })
        .ok()?;
    let rgba = pcx.to_rgba_with_color_key(SIDE_ICON_TRANSPARENT_RGB);
    Some(RenderedLoadingEntry {
        label: file_name.to_ascii_lowercase(),
        width: pcx.width as u32,
        height: pcx.height as u32,
        rgba,
    })
}
```

Do not change generic PCX decoding, SHP transparency, selected-map preview
pixels, or MMPB marker decoding.

### Step 4: Extend the configured-retail manifest test

Inside the existing variant loop in
`loading_art_manifest_assets_resolve_and_decode_from_configured_install_when_available`,
after the loading background assertion, add:

```rust
let icon = render_pcx_entry(&assets, variant.side_icon_pcx())
    .unwrap_or_else(|| panic!("{variant:?} country insignia should decode"));
assert!(
    icon.rgba.chunks_exact(4).any(|pixel| pixel[3] == 0),
    "{variant:?} country insignia should contain transparent keyed pixels"
);
```

This remains a configured-retail integration test; the synthetic decoder test
is the always-available semantic oracle.

## Task 2: Add the pure progress-row snapshot and integer layout

**Files:**

- Create: `src/app_loading_progress_row.rs`
- Modify: `src/lib.rs`

### Step 1: Add the module declaration

Immediately after `pub mod app_loading;` in `src/lib.rs`, add:

```rust
pub mod app_loading_progress_row;
```

### Step 2: Create the CPU-only module

Create `src/app_loading_progress_row.rs` with:

```rust
//! CPU-side standard-Skirmish loading progress-row identity and geometry.
//!
//! Depends only on immutable launch data and integer shell geometry. GPU asset
//! decoding remains in `render::loading_screen_chrome`; draw submission remains
//! in `app_loading`.

use crate::skirmish_launch::SkirmishLaunchSession;
use crate::ui::shell::geom::RectPx;

const WIDE_BREAKPOINT: u32 = 800;
const NARROW_BASE_X: i32 = 0x0C;
const NARROW_BASE_Y: i32 = 0x100;
const NARROW_ROW_WIDTH: i32 = 0x146;
const WIDE_BASE_X: i32 = 0x10;
const WIDE_BASE_Y: i32 = 0x141;
const WIDE_ROW_WIDTH: i32 = 0x196;
const BAR_X_HELPER_INSET: i32 = 5 + 3;
const BAR_Y_INSET: i32 = 3;
const BAR_HEIGHT_BAND: i32 = 6;
const ROW_PADDING: i32 = 4;
const SIDE_ICON_GAP: i32 = 0x15;
const LABEL_GAP_AFTER_ICON: i32 = 10;
const LABEL_RIGHT_INSET: i32 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoadingProgressRowSnapshot {
    pub label: String,
}

impl LoadingProgressRowSnapshot {
    pub fn from_launch_session(session: &SkirmishLaunchSession) -> Self {
        Self {
            label: session.player_name.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LoadingProgressRowLayout {
    pub row_height: i32,
    pub bar_origin: [i32; 2],
    pub icon_origin: Option<[i32; 2]>,
    pub label_rect: RectPx,
}

pub(crate) fn layout_standard_skirmish_progress_row(
    render_width: u32,
    bar_size: [i32; 2],
    side_icon_size: Option<[i32; 2]>,
    font_height: i32,
) -> LoadingProgressRowLayout {
    let [base_x, base_y, row_width] = if render_width >= WIDE_BREAKPOINT {
        [WIDE_BASE_X, WIDE_BASE_Y, WIDE_ROW_WIDTH]
    } else {
        [NARROW_BASE_X, NARROW_BASE_Y, NARROW_ROW_WIDTH]
    };
    let bar_width = bar_size[0].max(0);
    let bar_height = bar_size[1].max(0);
    let font_height = font_height.max(0);
    let side_icon_size =
        side_icon_size.filter(|size| size[0] > 0 && size[1] > 0);
    let icon_height = side_icon_size.map_or(0, |size| size[1]);
    let row_height = icon_height
        .max(bar_height + BAR_HEIGHT_BAND)
        .max(font_height)
        + ROW_PADDING;

    let bar_origin = [
        base_x + BAR_X_HELPER_INSET,
        base_y + (row_height - (bar_height + BAR_HEIGHT_BAND)) / 2 + BAR_Y_INSET,
    ];
    let icon_x = base_x + bar_width + SIDE_ICON_GAP;
    let icon_origin = side_icon_size.map(|size| {
        [
            icon_x,
            base_y + (row_height - size[1]) / 2,
        ]
    });
    let label_x = side_icon_size.map_or(icon_x, |size| {
        icon_x + size[0] + LABEL_GAP_AFTER_ICON
    });
    let label_y = base_y + (row_height - font_height) / 2;
    let label_right = base_x + row_width - LABEL_RIGHT_INSET;
    let label_rect = RectPx::new(
        label_x,
        label_y,
        (label_right - label_x).max(0),
        font_height,
    );

    LoadingProgressRowLayout {
        row_height,
        bar_origin,
        icon_origin,
        label_rect,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_progress_row_layout_matches_native_640_fixture() {
        let layout =
            layout_standard_skirmish_progress_row(640, [80, 5], Some([47, 23]), 12);

        assert_eq!(layout.row_height, 27);
        assert_eq!(layout.bar_origin, [20, 267]);
        assert_eq!(layout.icon_origin, Some([113, 258]));
        assert_eq!(layout.label_rect, RectPx::new(170, 263, 165, 12));
    }

    #[test]
    fn loading_progress_row_layout_matches_native_800_fixture() {
        let layout =
            layout_standard_skirmish_progress_row(800, [80, 5], Some([47, 23]), 12);

        assert_eq!(layout.row_height, 27);
        assert_eq!(layout.bar_origin, [24, 332]);
        assert_eq!(layout.icon_origin, Some([117, 323]));
        assert_eq!(layout.label_rect, RectPx::new(174, 328, 245, 12));
    }

    #[test]
    fn missing_icon_uses_would_be_icon_anchor_for_label() {
        let layout = layout_standard_skirmish_progress_row(640, [80, 5], None, 12);

        assert_eq!(layout.row_height, 16);
        assert_eq!(layout.bar_origin, [20, 261]);
        assert_eq!(layout.icon_origin, None);
        assert_eq!(layout.label_rect, RectPx::new(113, 258, 222, 12));
    }

    #[test]
    fn actual_font_height_can_dominate_the_row() {
        let layout =
            layout_standard_skirmish_progress_row(640, [80, 5], Some([20, 10]), 30);

        assert_eq!(layout.row_height, 34);
        assert_eq!(layout.label_rect.y, 258);
        assert_eq!(layout.label_rect.h, 30);
    }
}
```

The arithmetic intentionally stays integral. Native division truncates, so do
not convert to `f32` until building `SpriteInstance` positions.

### Step 3: Run the pure layout slice

```powershell
cargo test -q loading_progress_row_layout
cargo test -q missing_icon_uses_would_be_icon_anchor_for_label
cargo test -q actual_font_height_can_dominate_the_row
```

Read and record each literal `test result:` line.

## Task 3: Snapshot the player name before the first frame

**Files:**

- Modify: `src/app_loading.rs`

### Step 1: Import and store the row snapshot

Add:

```rust
use crate::app_loading_progress_row::{
    LoadingProgressRowLayout, LoadingProgressRowSnapshot,
    layout_standard_skirmish_progress_row,
};
```

Add this field to `NativeLoadingScreenState`:

```rust
pub progress_row: LoadingProgressRowSnapshot,
```

Change `NativeLoadingScreenState::standard_skirmish` to accept
`progress_row: LoadingProgressRowSnapshot` and assign it:

```rust
fn standard_skirmish(
    variant: LoadingArtVariant,
    local_side_index: u8,
    color_index: HouseColorIndex,
    progress_row: LoadingProgressRowSnapshot,
    progress_cadence: NativeLoadingProgressCadence,
) -> Self {
    Self {
        variant,
        local_side_index,
        color_index,
        backing_rgb: FALLBACK_BACKING_RGB,
        text_rgb: [1.0, 1.0, 1.0],
        progress_ramp: FALLBACK_PROGRESS_RAMP,
        progress: LoadingProgressState::standard_skirmish(),
        progress_row,
        atlas: None,
        composition: None,
        first_renderer_ready: false,
        runtime_color_scheme_count: 0,
        progress_cadence,
    }
}
```

At the `LoadingSession::from_request` call site, pass:

```rust
LoadingProgressRowSnapshot::from_launch_session(skirmish_launch_session),
```

between `color_index` and `progress_cadence`.

Update the direct test construction of `NativeLoadingScreenState` with:

```rust
LoadingProgressRowSnapshot {
    label: "Player".to_owned(),
},
```

### Step 2: Add the identity regression

Beside `loading_side_comes_from_first_launch_node_country`, add:

```rust
#[test]
fn loading_progress_row_snapshots_the_launch_player_name() {
    let mut launch = test_launch_session(LaunchCountry::America);
    launch.player_name = "Commander".to_owned();
    let session = LoadingSession::from_request(
        LoadingRequest::unverified_legacy_skirmish(
            launch,
            unverified_seed(22),
            SkirmishSettings::default(),
        ),
    );

    assert_eq!(
        session
            .native
            .as_ref()
            .map(|native| native.progress_row.label.as_str()),
        Some("Commander"),
    );
}
```

Run:

```powershell
cargo test -q loading_progress_row_snapshots_the_launch_player_name
```

Do not retrieve the name from simulation state, map metadata, localization, or
the selected-map composition snapshot.

## Task 4: Build one shared row plan for both presentation paths

**Files:**

- Modify: `src/app_loading.rs`

### Step 1: Replace the floating-point geometry helpers

Delete `standard_skirmish_row_origin`,
`standard_skirmish_row_height`,
`standard_skirmish_progress_position`, and
`standard_skirmish_side_icon_position`, along with the five now-moved geometry
constants. Keep the progress formula/cadence constants in `app_loading`.

Add:

```rust
const ROW_LABEL_DEPTH: f32 = 0.05;

struct NativeLoadingFramePlan {
    instances: Vec<SpriteInstance>,
    text_draws: Vec<NativeLoadingTextDraw>,
}

fn native_loading_row_layout(
    font: &BitFont,
    atlas: &LoadingScreenAtlas,
    progress: &LoadingProgressState,
    render_width: u32,
) -> Option<LoadingProgressRowLayout> {
    if progress.current_value() == 0.0 {
        return None;
    }
    Some(layout_standard_skirmish_progress_row(
        render_width,
        [
            atlas.progress_frame0.pixel_size[0] as i32,
            atlas.progress_frame0.pixel_size[1] as i32,
        ],
        atlas.side_icon.map(|icon| {
            [icon.pixel_size[0] as i32, icon.pixel_size[1] as i32]
        }),
        font.cell_height() as i32,
    ))
}
```

The asset and font dimensions are decoded integer pixel sizes represented as
`f32` by the render structs; convert them once at this boundary.

### Step 2: Make sprite construction consume the shared layout

Change the final argument of `build_native_loading_instances` from
`render_width: u32` to:

```rust
row_layout: Option<&LoadingProgressRowLayout>,
```

Replace the current row-height/origin calculation with:

```rust
let Some(row_layout) = row_layout else {
    return instances;
};
let bar_w = atlas.progress_frame0.pixel_size[0];
let bar_h = atlas.progress_frame0.pixel_size[1];
let bar_origin = [
    row_layout.bar_origin[0] as f32,
    row_layout.bar_origin[1] as f32,
];
```

Keep the current full backing fill and clipped frame-0 progress draw unchanged.
Replace the side-icon tail with:

```rust
// Country insignia follows the progress span. The atlas has already applied
// the verified RGB-magenta key.
if let (Some(icon), Some(icon_origin)) = (atlas.side_icon, row_layout.icon_origin) {
    push_entry(
        &mut instances,
        icon,
        [icon_origin[0] as f32, icon_origin[1] as f32],
        SIDE_ICON_DEPTH,
    );
}
```

Delete the stale comment claiming the Skirmish text pointer means no label.

### Step 3: Allow text draws to carry their intended depth

Add a `depth: f32` argument to `build_native_loading_text_draw` immediately
after `with_backing: bool`, and replace the hardcoded `TEXT_DEPTH` argument to
`draw_in_rect` with `depth`.

Pass `TEXT_DEPTH` from each of the four existing selected-map copy calls.

Change `build_native_loading_text_draws` to accept:

```rust
row: &LoadingProgressRowSnapshot,
row_layout: Option<&LoadingProgressRowLayout>,
row_rgb: [f32; 3],
```

in addition to its current arguments. Remove the early return when
`composition` is `None`; selected-map copy remains conditional, but the progress
row must also render for random maps. Its structure should be:

```rust
fn build_native_loading_text_draws(
    font: &BitFont,
    atlas: &LoadingScreenAtlas,
    composition: Option<&LoadingCompositionSnapshot>,
    row: &LoadingProgressRowSnapshot,
    row_layout: Option<&LoadingProgressRowLayout>,
    text_rgb: [f32; 3],
    row_rgb: [f32; 3],
) -> Vec<NativeLoadingTextDraw> {
    let mut draws = Vec::with_capacity(5);

    if let Some(composition) = composition {
        if let Some(text) = composition.text.country_name.as_deref() {
            draws.push(build_native_loading_text_draw(
                font,
                atlas,
                text,
                composition.text_rects.country_name,
                text_rgb,
                ShellAlign::H_RIGHT,
                true,
                TEXT_DEPTH,
            ));
        }
        if let Some(text) = composition.text.special_unit.as_deref() {
            draws.push(build_native_loading_text_draw(
                font,
                atlas,
                text,
                composition.text_rects.special_unit,
                [0.0, 0.0, 0.0],
                ShellAlign::NONE,
                false,
                TEXT_DEPTH,
            ));
        }
        if let Some(text) = composition.text.load_brief.as_deref() {
            draws.push(build_native_loading_text_draw(
                font,
                atlas,
                text,
                composition.text_rects.load_brief,
                text_rgb,
                ShellAlign::NONE,
                true,
                TEXT_DEPTH,
            ));
        }
        if let Some(text) = composition.text.loading.as_deref() {
            draws.push(build_native_loading_text_draw(
                font,
                atlas,
                text,
                composition.text_rects.loading,
                text_rgb,
                ShellAlign::NONE,
                true,
                TEXT_DEPTH,
            ));
        }
    }

    if let Some(layout) = row_layout
        && !row.label.is_empty()
        && layout.label_rect.w > 0
        && layout.label_rect.h > 0
    {
        draws.push(build_native_loading_text_draw(
            font,
            atlas,
            &row.label,
            layout.label_rect,
            row_rgb,
            ShellAlign::NONE,
            false,
            ROW_LABEL_DEPTH,
        ));
    }

    draws
}
```

`row_rgb` is the local player's resolved `[Colors]` RGB, the same native scheme
used by the row. Do not use the AlliedLoad/SovietLoad copy color for this label.
The no-backing flag is mandatory.

### Step 4: Add the shared frame-plan constructor

```rust
#[allow(clippy::too_many_arguments)]
fn build_native_loading_frame_plan(
    font: &BitFont,
    atlas: &LoadingScreenAtlas,
    composition: Option<&LoadingCompositionSnapshot>,
    progress_row: &LoadingProgressRowSnapshot,
    progress: &LoadingProgressState,
    backing_rgb: [f32; 3],
    text_rgb: [f32; 3],
    render_width: u32,
) -> NativeLoadingFramePlan {
    let row_layout =
        native_loading_row_layout(font, atlas, progress, render_width);
    let instances = build_native_loading_instances(
        atlas,
        composition,
        progress,
        backing_rgb,
        row_layout.as_ref(),
    );
    let text_draws = build_native_loading_text_draws(
        font,
        atlas,
        composition,
        progress_row,
        row_layout.as_ref(),
        text_rgb,
        backing_rgb,
    );
    NativeLoadingFramePlan {
        instances,
        text_draws,
    }
}
```

At the ordinary render path, replace the two independent builders with one
call:

```rust
let frame_plan = build_native_loading_frame_plan(
    &state.bit_font,
    atlas,
    native.composition.as_ref(),
    &native.progress_row,
    &native.progress,
    native.backing_rgb,
    native.text_rgb,
    state.gpu.config.width,
);
let instances = frame_plan.instances;
let text_draws = frame_plan.text_draws;
```

### Step 5: Route both synchronous presenters through the same plan

Add `progress_row: &LoadingProgressRowSnapshot` to
`present_native_loading`. Inside it, replace the two independent builders with:

```rust
let frame_plan = build_native_loading_frame_plan(
    font,
    atlas,
    composition,
    progress_row,
    progress,
    backing_rgb,
    text_rgb,
    render_width,
);
let instances = frame_plan.instances;
let text_draws = frame_plan.text_draws;
```

Pass `&native.progress_row` from `advance_and_present_native_progress`.

Add this field to `RenderingProgressSink`:

```rust
progress_row: &'a LoadingProgressRowSnapshot,
```

Initialize it from `&native.progress_row`, and pass `self.progress_row` to
`present_native_loading`.

Do not change `LoadingProgressState`, cadence selection, milestone emitters, the
3%/100% presentation gates, or `src/app_init.rs`.

## Task 5: Record the complete progress-row composition order

**Files:**

- Modify: `src/app_loading_composition.rs`

Add the final enum variant:

```rust
ProgressLabel,
```

Immediately after:

```rust
layers.push(LoadingCompositionLayer::ProgressSideIcon);
```

add:

```rust
layers.push(LoadingCompositionLayer::ProgressLabel);
```

Extend `ordered_layers_keep_text_between_markers_and_progress` so its expected
tail is:

```rust
LoadingCompositionLayer::ProgressBacking,
LoadingCompositionLayer::ProgressBar,
LoadingCompositionLayer::ProgressSideIcon,
LoadingCompositionLayer::ProgressLabel,
```

The snapshot still does not own the row label or geometry; this enum records
the complete player-visible order only.

Run:

```powershell
cargo test -q ordered_layers_keep_text_between_markers_and_progress
```

## Task 6: Remove superseded tests and run the focused suite

**Files:**

- Modify: `src/app_loading.rs` tests

Delete these tests because the new pure module replaces their floating-point
helpers:

- `bar_origin_uses_helper_offset_and_row_centering`
- `row_height_is_dominated_by_tallest_of_band_icon_font_plus_padding`
- `side_icon_sits_one_gap_right_of_bar_and_is_vertically_centered`

Do not delete progress cadence, fill-width, duplicate-milestone, atlas, or
player-ramp tests.

Format only edited Rust files:

```powershell
rustfmt --edition 2024 src/app_loading_progress_row.rs src/app_loading.rs src/render/loading_screen_chrome.rs src/assets/pcx_file.rs src/app_loading_composition.rs src/lib.rs
```

Inspect the diff immediately:

```powershell
git diff --check
git diff -- src/app_loading_progress_row.rs src/app_loading.rs src/render/loading_screen_chrome.rs src/assets/pcx_file.rs src/app_loading_composition.rs src/lib.rs
```

Run focused tests serially:

```powershell
cargo test -q rgba_color_key_uses_rgb_not_palette_index
cargo test -q direct_rgb_color_key_applies_after_rgb_conversion
cargo test -q loading_progress_row_layout
cargo test -q loading_progress_row_snapshots_the_launch_player_name
cargo test -q ordered_layers_keep_text_between_markers_and_progress
cargo test -q loading_progress
cargo test -q loading_art_manifest_assets_resolve_and_decode_from_configured_install_when_available
```

Then run:

```powershell
cargo check -q
```

For every test invocation, report the literal `test result:` line. Do not infer
success from process completion.

## Task 7: End-to-end visual validation

Use the production application, not a mocked renderer. Run only after the
focused tests pass.

### Required matrix

1. 640x480, selected stock map, America.
2. 640x480, selected stock map, Russia.
3. 640x480, selected stock map, Yuri.
4. 800x600, selected stock map, America.
5. 800x600, selected stock map, Russia.
6. 800x600, selected stock map, Yuri.
7. 640x480, random map, America.
8. 800x600, random map, America.

Use a distinctive non-empty player name such as `Commander` so a stale
hardcoded `Player` value is obvious.

### Acceptance checks

At the first visible progress value, one middle advancing milestone, and 100%:

- no opaque magenta country-icon background;
- the icon retains its intended opaque colored pixels;
- the label text is exactly the configured local player name;
- label is left aligned, has no backing rectangle, and uses the local player's
  selected color;
- bar, icon, and label remain vertically aligned;
- label begins ten pixels after a present icon and does not overlap it;
- label clips at the native row right bound without spilling;
- selected-map copy/preview/markers remain unchanged;
- random-map loading shows the same progress-row contract even though it has no
  selected-map composition snapshot;
- duplicate/lower progress callbacks do not cause new visible redraw states;
- terminal 100% still presents before gameplay.

Capture at least one 640 and one 800 screenshot for handoff. Treat them as
production evidence, not exact native pixel or timing oracles.

## Post-implementation review checklist

- Re-read `git status --short` and compare it to the preflight baseline.
- Confirm no `src/app_init.rs`, `src/sim/`, INI, map, or unrelated render change
  was introduced.
- Confirm every existing pre-task hunk was preserved.
- Search for stale palette-index and missing-label claims:

  ```powershell
  rg -n "SIDE_ICON_TRANSPARENT_INDEX|index 0 as transparent|No label is drawn|font_h = bar_height" src
  ```

  Require no active production matches.
- Confirm both ordinary and synchronous paths call
  `build_native_loading_frame_plan`.
- Confirm row label construction is outside the selected-map-only composition
  branch.
- Confirm `ROW_LABEL_DEPTH < SIDE_ICON_DEPTH`.
- Confirm the implementation uses `backing_rgb` for the row label and
  `text_rgb` only for existing AlliedLoad/SovietLoad copy.
- Confirm every exactness statement remains honest: retail-convincing closure,
  not native/Rust pixel parity.

## Handoff

Report:

- exact files changed;
- focused test literal result lines;
- `cargo check -q` exit status;
- visual matrix results and screenshot paths;
- any residual or skipped matrix row;
- explicit statement that exact pixel/timing parity remains
  `UNCHECKED`/`UNVERIFIED`.

Do not commit or push unless the user separately requests it.
