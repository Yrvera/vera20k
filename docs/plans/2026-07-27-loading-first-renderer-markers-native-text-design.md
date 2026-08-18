# Loading First-Renderer Markers and Native Text — Design

**Date:** 2026-07-27  
**Status:** approved for a bounded verified-fix swarm  
**System:** GSI-03.09 / LOOP-001  
**Delivery target:** retail-convincing selected-map loading composition

## Outcome

The first displayed selected-map loading frame will contain the retail map
preview, black start indicators, assigned-color `mmpb.shp` markers, the four
verified localized text layers, and the already-implemented 3% progress chrome.
The composition is prepared before the first submitted frame and reused by all
later synchronous progress presentations.

This closes the ordinary-play omission without claiming final-pixel parity.
Exact glyph blending, the complete native auto-start randomization algorithm,
and exhaustive unusual-resolution behavior remain explicit residuals.

## Evidence

The design is grounded in:

- `docs/research/LOADING_FIRST_RENDERER_CORRECTED_COMPOSITION_DATA_READINESS_GHIDRA_REPORT.md`
- `docs/research/LOADING_MMPB_EXACT_MARKER_ASSIGNMENT_COMPOSITION_GHIDRA_REPORT.md`
- `docs/research/LOADING_POST_MARKER_TEXT_MODE5_CONTENT_LAYOUT_GHIDRA_REPORT.md`
- `docs/research/LOADING_FIRST_RENDERER_COMPOSITION_SYSTEM_MODEL_SYNTHESIS.md`

The verified selected-map order is:

1. loading background
2. selected map preview with black 4×4 start indicators
3. assigned-house `mmpb.shp` markers
4. localized country name
5. localized country-special-unit line
6. localized `LoadBrief:<country>` paragraph
7. localized `GUI:LoadingEx`
8. progress backing/bar/side icon
9. hidden-to-primary synchronous blit

The selected-map compositor returns before the raw 3 callback. Therefore the
first confirmed displayed frame contains the whole static composition plus 3%.
No claim is made about native `Present`/`Flip` or a visible pre-3 frame.

## Player-experience ledger

| Detail | Classification | Design response |
|---|---|---|
| Map preview absent during selected-map loading | MILESTONE-BLOCKING | Decode and display the selected map's own `PreviewPack` |
| Assigned start markers absent | MILESTONE-BLOCKING | Draw `mmpb.shp` frame 0 using the actual Rust launch assignments and colors |
| Four native loading text layers absent | MILESTONE-BLOCKING | Resolve verified CSF keys and draw in verified order/rectangles |
| Current marker-region field order and `>=` selector | COMPOUNDING | Replace with conventional `(x,y,width,height)` and exact width equality |
| Native auto-start random/distance selection differs from Rust | DRIFT | Do not broaden loading UI into simulation/RNG authority; share Rust's actual assignment core |
| Sparse waypoint holes, malformed maps, unusual widths | EXACTIFICATION-RESIDUAL | Preserve verified prefix behavior where safe; guard invalid divisors |
| Exact glyph, alpha, palette, and final framebuffer pixels | UNCHECKED | Preserve verified content/order/layout; do not claim pixel parity |

## Considered approaches

### A. Snapshot current shell preview state

Reuse `SkirmishScenarioRecord` or the shell preview texture before teardown.

Rejected. Normal scenario discovery intentionally leaves `PreviewPack` lazy,
the shell marker projection uses `[Header] PreviewSourceBounds` rather than the
verified loading projection, the GPU texture does not retain editable CPU RGBA,
and final launch assignments do not exist there. It could display plausible but
incorrect markers.

### B. Transfer the torn-down shell GPU texture

Move the shell's preview texture and marker state into the loading renderer.

Rejected. The shell uses setup-specific visuals and projection, lacks the
assignment table and black source rectangles, and would couple game startup to
the lifetime of a UI screen that is deliberately torn down.

### C. Prepare an authoritative composition before first display

Parse the selected map's initial load phase before the first submitted loading
frame, build an immutable snapshot from the parsed map plus launch session, and
reuse it through later progress callbacks.

Selected. This matches the verified native order, avoids duplicate map parsing,
uses the map actually being loaded, and lets marker presentation share the
assignments used by current Rust gameplay.

## Architecture

### New composition module

Add `src/app_loading_composition.rs` for pure/loading-specific preparation:

- verified marker region selection and projection math
- black start-indicator rasterization into a cloned preview RGBA buffer
- participant/start/color marker records
- CSF key mapping and localized text snapshot
- verified text rectangles and layer ordering
- tests for signed arithmetic, sparse prefixes, layout, and assignment identity

This keeps `src/app_loading.rs` focused on lifecycle and GPU orchestration and
prevents the already-large file from absorbing another cohesive subsystem.
`src/lib.rs` declares the module.

### Selected-map pre-first-frame phase

For selected-map cadence only:

1. create the loading job
2. run `MapLoadInitial` once with a deferred/non-presenting progress sink
3. retain the resulting initial map state in the job
4. build `LoadingCompositionSnapshot`
5. submit the first loading frame at 3%
6. visibly hand off the deferred 8% milestone
7. continue the existing phase machine

Random-map cadence keeps its current pre-generation first frame and does not use
the selected-map marker path. This preserves the verified visible sequence:
selected map begins at 3%, random map begins at 1%.

`MapLoadInitial` exposes only the read-only map data required to create the
snapshot. It remains the owner transferred into the later load phase.

### Preview decoding

Full map parsing decodes the selected map's `PreviewPack` into
`MapFile.preview.decoded`. Menu discovery remains lazy and unchanged.
Preview-decode failure is non-fatal: log it and omit preview/marker layers while
retaining background, text, and progress.

The CPU preview clone is modified before GPU upload:

- valid numeric start waypoints receive verified black 4×4 source rectangles
- writes are clipped to the image bounds
- zero/invalid projected extents omit markers instead of panicking

### Start assignment authority

Refactor the existing launch assignment into:

- an original-waypoint assignment phase: explicit starts first, then current
  Rust Auto first-free behavior
- the existing terrain fallback phase for deficient start pools

Both loading composition and simulation startup consume the same original
assignment result. Terrain-generated fallback starts have no original preview
coordinate and therefore receive no loading marker.

This intentionally does not implement the native randomized/distance Auto
algorithm in a loading-screen patch. That is a separate deterministic gameplay
change. The loading UI must describe where current Rust will actually spawn the
participants.

### Marker projection and assets

Use the verified marker regions:

- width exactly 800: `(499,379,216,166)`
- width exactly 1024: `(570,424,300,260)`
- every other width: `(385,270,200,200)`

The type is named conventionally as `(x, y, width, height)`. No `>=`
breakpoints remain.

Projection preserves signed native arithmetic:

- waypoint cells are sign-extended from 16 bits
- cell center is `cell * 256 + 128`
- isometric projected coordinates divide by 60 and 30
- aspect fit uses scale 1000 and truncate-toward-zero
- fractions use the verified two-stage 1,000,000 normalization
- marker offset is `(-3,-2)`

The loading atlas packs:

- existing background/progress assets
- the prepared preview image
- one remapped `mmpb.shp` frame-0 image for each distinct assigned house color

Color comes from each assigned participant's launch color through the existing
scheme-priority and house-ramp conversion. Marker records retain the explicit
start-index-to-participant mapping; they are never produced by zipping two
independently filtered lists.

Missing `mmpb.shp` or a missing color conversion omits only the affected marker.
No marker pixels are hardcoded.

### Localized text

Resolve strings once into the immutable snapshot:

1. country name key
2. country special-unit key, uppercased after localization
3. `LoadBrief:<country>` key, including retail's `LoadBrief:Lybia` spelling
4. `GUI:LoadingEx`

The keys are table-driven by `LaunchCountry`. `LSLoadMessage`,
`LSLoadBriefing`, and map `[Briefing]` are not used. A missing CSF value logs a
warning and omits only that layer; English is not invented.

Text color uses the named `[Colors]` scheme:

- Allied local side: `AlliedLoad`
- Soviet or Yuri local side: `SovietLoad`

The existing integer HSV-to-RGB conversion supplies the draw color.

At width 640 use the verified 640 rectangles. At width 800 and above, use the
verified 800 rectangles plus a centered 800×600 viewport offset. Non-640 widths
below 800 keep the native 800-base rectangles without centering. Cooperative
mode changes only the 800-base briefing Y coordinate and is identified by the
existing `MPCoopMD.ini` mode contract.

Country, briefing, and Loading text receive a measured black backing at alpha
`0x9F`; the special-unit line is black with no backing. `GAME.FNT` is drawn
through the existing `BitFont` and `shell_text::draw_in_rect` machinery.

### Ordered rendering

Create an explicit ordered list of draw commands, each carrying its texture,
instance buffer, and scissor:

1. base loading atlas: background, preview, markers
2. country backing
3. country font
4. special-unit font
5. briefing backing
6. briefing font
7. Loading backing
8. Loading font
9. loading atlas: progress chrome

Submit them in one render pass, switching bind groups and scissors as the
existing shell renderer already does. The synchronous presenter reuses the same
command builder, so callback milestones cannot omit the static composition.

## Failure behavior

- selected map parse failure: preserve existing load failure
- missing/invalid preview: omit preview and markers
- missing `mmpb.shp`: retain preview and text
- missing localized string: omit that text layer
- missing named text color: fall back to white and log
- zero projected extent or invalid marker arithmetic: omit markers
- unsupported window width: use the verified native fallback marker region

No failure path mutates INI files.

## Validation

Focused automated checks:

- marker-region equality semantics for 640/800/1024/1920
- signed projection fixture and truncate-toward-zero behavior
- 4×4 black rectangle clipping
- sparse valid-waypoint prefix behavior
- assignment mapping preserves participant color identity
- text key table for all countries, including `Lybia`
- 640, 800, 1024, and cooperative briefing rectangles
- layer-command order
- selected-map visible milestones remain 3 then 8
- random-map first visible milestone remains 1
- every later progress callback rebuilds the complete composition

Retail-asset integration checks:

- decoded selected-map preview is present
- `mmpb.shp` frame 0 decodes as the retail asset rather than embedded pixels
- localized loading keys resolve from the active CSF

Then run the focused loading suite serially and one final `cargo check -q`,
subject to Cargo ownership coordination.

## Adversarial review

**The extra preparse can delay the first visible frame.** Native also performs
selected-map preparation before composing the first confirmed frame. The map is
parsed once and transferred forward, so this buys correctness without duplicate
work.

**Auto markers still do not match native random assignment.** Correct: that is
recorded DRIFT. They will match current Rust gameplay exactly, which is more
honest and safer than changing deterministic spawn/RNG authority inside a UI
slice.

**Multiple textures complicate the presenter.** The renderer already has a
proven one-pass bind-group/scissor pattern in the shell. A typed ordered command
list contains that complexity and makes the native layer order testable.

**Malformed custom maps could divide by zero or write outside preview memory.**
The snapshot builder validates extents and clips CPU writes. This is a deliberate
Rust safety guard, not an exact malformed-input claim.

**The implementation could make `app_loading.rs` unmaintainable.** Pure
composition, layout, and projection live in the new module; orchestration and
GPU presentation remain in `app_loading.rs`; retail asset packing remains in
`render/loading_screen_chrome.rs`.

## Stop condition

Stop when selected-map first and later loading frames consistently include the
prepared preview, assigned markers, four localized text layers, and progress
chrome in verified order; focused tests and final check pass; no unrelated files
or INIs changed; and remaining exactness gaps are documented without an exact
pixel-parity claim.
