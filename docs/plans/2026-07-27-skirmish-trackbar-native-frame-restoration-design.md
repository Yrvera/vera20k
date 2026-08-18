# Skirmish Trackbar Native Frame Restoration Design

## Goal

Restore the complete retail trackbar frame around the slider and numeric plaque in the standard offline Skirmish screen, without changing slider values, input behavior, layout, or unrelated shell chrome.

## Architecture Context

The standard Skirmish renderer builds one shared chrome atlas in
`src/render/skirmish_shell_chrome.rs`. The app-layer owner-draw seam in
`src/app_skirmish_shell_render/controls.rs` resolves the atlas entries for a
`ControlPaint::Trackbar`, while `src/ui/skirmish_shell/layout.rs` owns the
verified `128x21` control rect and plaque/thumb geometry. The same app-layer
paint seam is also used by the active in-game Options trackbars, whose retail
descriptor uses the same `128`-wide control family.

The active YR callback at `0x0061D950` draws the plaque PCXs and `trakgrip.pcx`,
then issues two border-2 primitive frame calls at `0x0061E204` and
`0x0061E269`. Rust currently pre-renders only a thin horizontal primitive into
`skirmish_trackbar_rail` and emits it before the plaque and thumb. That is the
source of the visibly absent rectangular frame.

## Impact Analysis

- `src/render/skirmish_shell_chrome.rs`
  - Replace the thin rail approximation with one transparent atlas entry that
    contains both verified primitive frame calls.
  - Reuse the existing inclusive primitive-bevel raster and fixed bevel colors.
- `src/app_skirmish_shell_render/controls.rs`
  - Anchor the expanded frame entry two pixels above and left of the control.
  - Emit plaque, thumb, then primitive frames, matching the native paint stack.
  - Update focused draw-list tests.
- `src/app_skirmish_shell_render/in_game_options.rs`
  - Update only assertions/comments that describe the shared chrome entry.

No simulation, input, persisted settings, control rects, value mapping, text,
assets, or public data formats change. The main risk is an incorrect expanded
frame anchor or z-order causing a one-pixel crop or covering value text.

## Chosen Approach

Pre-render the two native primitive calls into one transparent atlas entry for
the verified `128x21` trackbar family, then composite that entry after the
plaque and thumb.

The entry uses a `132x25` canvas representing the control rect expanded by the
native two-pixel bevel border. Relative to that canvas:

- left frame input box: `[2, 2, 78, 21]`;
- numeric-side frame input box: `[82, 2, 48, 21]`;
- canvas draw anchor: `(control.x - 2, control.y - 2)`.

This keeps raster ownership in the atlas builder, follows the existing
asset/primitive composition pattern, uses one draw instance per trackbar, and
preserves the verified adjacent-frame overlap/divider.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING`: The standard Game Speed, Credits, and Unit Count
  controls need the complete rectangular frame; it is continuously visible in
  every ordinary offline Skirmish setup. The active callback issues two
  primitive calls, not one thin rail. `[GHIDRA 0x0061E204, 0x0061E269]`
- `MILESTONE-BLOCKING`: Both calls use border width `2` and the inclusive
  `FUN_006208F0` raster contract; using a generic outline would alter corners
  and ring colors. `[GHIDRA 0x006208F0]`
- `MILESTONE-BLOCKING`: The numeric plaque remains `trofl.pcx`,
  `trofm.pcx`, and `trofr.pcx`; the thumb remains `trakgrip.pcx`. The frame is
  primitive chrome and must not replace those assets.
  `[GHIDRA 0x0061DE9C..0x0061E0AD]`
- `MILESTONE-BLOCKING`: The frame expands two pixels outside its input boxes.
  The combined canvas and `(-2,-2)` anchor prevent clipping of the outer rings.
  `[GHIDRA 0x006208F0]`
- `MILESTONE-BLOCKING`: Native composition is plaque, thumb, two primitive
  frames, then value text. The frame must overlay the PCX edges but remain
  below the separately rendered text. `[GHIDRA 0x0061DE9C..0x0061E30A]`
- `COMPOUNDING`: The owner-draw paint seam is shared with active in-game
  Options trackbars. Keeping the correction in `ControlPaint::Trackbar`
  prevents the two screens from acquiring different chrome implementations.
- `EXACTIFICATION-RESIDUAL`: Final monitor RGB after the original 16-bit
  DirectDraw conversion is not claimed pixel-identical. The existing verified
  RGBA approximation for bevel colors remains unchanged.
- `EXACTIFICATION-RESIDUAL`: Disabled-trackbar alpha overlay is outside this
  correction; the three standard offline Skirmish trackbars are active.

## Design

### Components

1. Add a small atlas-builder helper that draws multiple primitive bevel boxes
   into one transparent `RenderedShellEntry`.
2. Build `skirmish_trackbar_rail` as the complete two-frame composition rather
   than the current thin rail.
3. Add a render-side anchor helper for the expanded frame canvas.
4. Reorder `ControlPaint::Trackbar` emission to plaque, thumb, frame.

### Interfaces / Contracts

- Keep the existing `ControlChrome::trackbar_rail` field to avoid broad API
  churn; its role becomes the complete native rail/base frame composition.
- The control state contract remains `rect + thumb_px`.
- The entry is emitted at native size and is never stretched.

### Data Flow

The atlas builder rasterizes both verified boxes once. `ControlPaint::Trackbar`
receives the existing layout rect and thumb offset, emits the existing PCX
pieces, and finally anchors the transparent frame entry around the rect. Shell
text remains a later render layer.

### Error Handling

The primitive entry is generated in memory and cannot be absent when atlas
construction succeeds. Existing optional chrome behavior remains intact for
test fixtures and partial-asset environments.

### Testing Strategy

- Pixel test the combined entry dimensions, outer corners, two rings, and
  central divider/overlap.
- Draw-list test plaque, caps, thumb, and frame order, positions, sizes, and
  depths at minimum/middle/maximum thumb offsets.
- Preserve the in-game Options assertion that only its two visible trackbars
  emit the shared frame entry.
- Run focused renderer tests, then `cargo check -q`.
- Runtime visual validation is performed by the user; no Windows app control.

## Architectural Decisions

The design follows the existing pre-rendered primitive-atlas pattern rather
than adding a second immediate-mode rectangle renderer. It keeps UI chrome
above simulation and leaves deterministic state untouched. The existing
`trackbar_rail` name is retained as a compatibility seam even though the entry
now includes both adjacent primitive frames; its doc comment will make that
role explicit.

## Alternatives Considered

1. **Dynamic edge sprites at runtime.** This could support arbitrary trackbar
   sizes, but would add many instances, duplicate the already verified raster
   helper, and introduce corner/z-order risk. The active scoped controls use
   the same fixed family.
2. **Add a second thin texture beside the current rail.** Rejected because
   Ghidra shows two full-height primitive boxes, not two horizontal strips.
3. **Use a generic rectangle outline.** Rejected because it would lose the
   two-ring color swap, inclusive endpoints, mixed corners, and native overlap.
