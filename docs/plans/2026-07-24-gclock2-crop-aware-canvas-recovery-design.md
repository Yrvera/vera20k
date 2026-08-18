# GCLOCK2 Crop-Aware Canvas Recovery Design

## Goal

Replace the unique useful part of `stash@{1}` while fixing the root geometry
problem: the build-progress clock must occupy its own full SHP canvas/slot, and a
base cameo that was alpha-cropped in the atlas must retain enough metadata to be
placed at its original canvas offset.

## Evidence and current boundary

- Retail `GCLOCK2.SHP` has 55 frames. Frame 0 is empty; visible frames use the
  full 60 by 48 canvas with zero offsets.
- Native composition draws the cameo first and GCLOCK2 afterward with the
  sidebar palette.
- `render/sidebar_chrome.rs` already builds a separate GCLOCK2 texture and
  preserves full-frame dimensions.
- The UI draw path already batches GCLOCK2 separately after cameo sprites and
  applies camera cancellation correctly.
- `render/sidebar_cameo_atlas.rs` alpha-crops base cameos but currently discards
  the original canvas dimensions and crop origin.
- `app_sidebar_build.rs` consequently reuses the cropped base-cameo rectangle
  for GCLOCK2. The old stash changes only that last symptom.

## Chosen design

1. Extend the rendered-cameo and atlas-entry metadata with original canvas size
   and visible crop origin.
2. Keep the atlas texture alpha-cropped for packing efficiency.
3. Compute one unrounded full-canvas-to-slot transform for the base cameo.
4. Place the cropped base rectangle at its original canvas offset, rounding
   shared edges rather than independently rounding position and size.
5. Compute GCLOCK2 placement independently from its own full-frame dimensions,
   anchored to/filling the full sidebar slot.
6. Extract the geometry calculation into a small pure helper so fractional scale
   and camera behavior can be tested without constructing application state.
7. Leave the already-correct texture separation, batching, draw order, palette,
   and progress-frame timing path unchanged.

## Rejected alternatives

- Reapply the three-line stash patch only: it fixes the clock rectangle but
  leaves alpha-cropped base cameos shifted/scaled relative to their original
  60 by 48 canvas.
- Stop cropping cameo textures: correct but wastes atlas space and broadens the
  rendering change unnecessarily.
- Reuse base cameo pixel size for GCLOCK2: the overlay owns an independent SHP
  canvas and must not inherit crop geometry.

## Validation

- A synthetic nonzero alpha crop retains its crop origin and full canvas size.
- A 60 by 48 canvas with a nonzero crop origin maps the visible base pixels to
  the correct slot-relative rectangle.
- GCLOCK2 maps to the full slot even when the base cameo is cropped.
- Fractional UI scale and nonzero camera offsets have no one-pixel seam or
  double-camera subtraction.
- Existing sidebar/cameo tests pass, followed by `cargo check -q -p vera20k`.

## Stop condition

The crop-aware placement and full-slot GCLOCK2 overlay pass focused validation,
unrelated render behavior remains unchanged, and `stash@{1}` remains available
until the replacement is committed.
