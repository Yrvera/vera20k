# Loading Progress-Row Closure Design

## Goal

Make the standard offline Skirmish loading progress row retail-convincing by
removing the opaque magenta country-insignia background, restoring the native
row text/status handoff, and deriving the complete row geometry from the actual
progress asset and GAME.FNT metrics without disturbing the verified progress
cadence.

The representative scenario is an ordinary selected-map Skirmish at the retail
640x480 and 800x600 loading sizes, with America, Russia, and Yuri covering the
three side families. Random-map loading must retain the same row contract, but
random-map preview composition remains outside this slice.

## Architecture Context

`src/assets/pcx_file.rs` owns PCX decoding. It preserves the embedded VGA
palette for retail one-plane PCXs and already exposes
`PcxFile::to_rgba_with_color_key`, which works for both palettized and
three-plane direct-RGB files.

`src/render/loading_screen_chrome.rs` owns the loading atlas. It resolves the
verified country-to-PCX mapping through `LoadingArtVariant::side_icon_pcx`,
packs the decoded insignia beside the LS background, `PROGBARM.SHP`, optional
preview/markers, and the solid texel, then exposes atlas entries to the app
renderer. Its current loading-specific PCX path treats palette index `0` as
transparent. That conflicts with the verified owner-draw flag contract, where
RGB magenta is the transparent key, and produces the user-observed opaque
magenta rectangle.

`src/app_loading.rs` owns loading-session orchestration, progress cadence,
per-milestone synchronous presentation, bar geometry, sprite construction, and
GAME.FNT text emission. `NativeLoadingScreenState` already receives the launch
session before the first loading frame. `SkirmishLaunchSession` owns
`player_name`, so the progress row does not need simulation state or a
post-start house lookup.

`src/app_loading_composition.rs` owns the selected-map first-render snapshot:
preview, markers, four localized post-marker text layers, and their ordered
composition. The progress row is not selected-map-only, so its identity and
geometry must not be stored exclusively in this snapshot.

The active native row contract is:

1. fill the full `PROGBARM.SHP` frame-0 rect with the player scheme backing;
2. draw the clipped, player-remapped frame-0 progress span;
3. draw the country PCX at `base_x + W + 0x15`, vertically centered;
4. invoke the row text/status helper after the icon branch.

Sources:

- `docs/research/PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md`
  sections 5, 6, 8, and 12;
- `docs/research/LOAD_PROGRESS_MANAGER_SETUP_GHIDRA_REPORT.md` verified facts
  and implementation handoff;
- `docs/research/skirmish-ui/SKIRMISH_LOWER_PCX_DECODE_PALETTE_KEY_PATH_FOR_FLAG_STATICS_GHIDRA_REPORT.md`
  sections 3.1-3.4 and 9;
- `docs/research/skirmish-ui/SKIRMISH_FLAG_PCX_PALETTE_AND_NATIVE_CLIP_GHIDRA_REPORT.md`
  sections 3-7;
- the user-provided production screenshot dated 2026-07-27.

The bounded read-only follow-up now proves that standard one-lane Skirmish uses
the first session-node display name, which maps to
`SkirmishLaunchSession::player_name` in the Rust launch snapshot. The helper
draws that value through GAME.FNT, left aligned, without a backing rectangle.
Its x position is ten pixels after the country icon, its y position uses the
actual GAME.FNT height and native integer truncation, and its right edge is
`base_x + width_override - 3`. The same follow-up proves that the loading
country PCX uses an RGB-magenta `(255,0,255)` transparency key.

## Impact Analysis

### Expected production surfaces

- `src/app_loading_progress_row.rs` (new): pure progress-row snapshot and layout
  contract, kept outside the already-large loading orchestrator.
- `src/lib.rs`: private module declaration.
- `src/app_loading.rs`: create the snapshot from the launch request, use the
  actual font metric, compose row sprites/text in both normal and synchronous
  presentation paths.
- `src/app_loading_composition.rs`: extend the existing player-visible layer
  ledger with the terminal progress-row label; the selected-map snapshot still
  does not own row identity or geometry.
- `src/render/loading_screen_chrome.rs`: decode country insignias with the
  loading row's verified/observed RGB color-key contract.
- Focused unit and retail-asset tests adjacent to those modules.

`src/assets/pcx_file.rs` should not need production changes because the required
RGB-key API already exists. A focused test may be added there only if current
coverage does not prove the non-magenta-index-zero case clearly enough.

### Dependencies and consumers

- The row snapshot depends on immutable launch-session identity only.
- Row layout depends on render width, `PROGBARM.SHP` frame-0 dimensions, optional
  side-icon dimensions, GAME.FNT cell height, and the verified native constants.
- The loading atlas remains the owner of retail asset decoding and GPU texture
  packing.
- Both `render_native_loading_frame` and `present_native_loading` must consume
  the same CPU row plan so the ordinary frame and per-milestone repaint cannot
  diverge.

### Blast radius

This is presentation-only. It does not touch `sim/`, RNG, tick ordering,
commands, save state, replay state, or deterministic hashes. The main risks are
incorrect alpha keying, a label drawn from the wrong native source, disagreement
between the regular and synchronous draw paths, and depth/scissor ordering that
places the label behind the icon or bar.

The working tree currently contains other loading-composition work. A later
implementation must preserve those changes and avoid broad rewrites of
`src/app_loading.rs` or `src/render/loading_screen_chrome.rs`.

## Chosen Approach

Use a scoped app-owned progress-row snapshot and pure layout module.

The snapshot captures stable row identity before the first frame. The layout
function receives live render/asset/font dimensions and returns the complete
bar, icon, and label geometry. The loading atlas continues to own PCX decoding;
the app renderer continues to own draw submission. This follows the existing
division between `app_loading_composition` CPU snapshots,
`loading_screen_chrome` GPU assets, and `app_loading` presentation orchestration.

The implementation is staged:

1. implement RGB-magenta keyed loading insignias;
2. introduce the pure row snapshot/layout and actual GAME.FNT height;
3. emit the verified first-player-name label after the icon;
4. validate fixed progress states and representative factions.

The research prerequisite is complete and recorded in
`PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md`. No label content or
geometry is inferred by the implementation.

## Player-Experience Detail Ledger

- **MILESTONE-BLOCKING — insignia transparency:** the opaque magenta rectangle
  is visible on every standard loading row using a retail country PCX. The
  retail flag path keys converted RGB magenta, not palette index `0`.
  `[doc: SKIRMISH_LOWER_PCX_DECODE_PALETTE_KEY_PATH_FOR_FLAG_STATICS_GHIDRA_REPORT.md sections 3.1-3.3]`
  Loading-specific exact 16-bit mechanism remains unproven, so the delivery fix
  is retail-convincing but not an exact-mechanism parity claim.
- **MILESTONE-BLOCKING — row text/status:** the native direct-draw path invokes
  the row helper after the icon branch and substitutes the first session node
  when the setup text pointer is null. Rust currently ends after the icon.
  `[doc: PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md section 6]`
  `[doc: LOAD_PROGRESS_MANAGER_SETUP_GHIDRA_REPORT.md verified facts]`
- **MILESTONE-BLOCKING — exact row-label content:** verified as the first
  session-node display name, mapped to the immutable Rust launch-session
  `player_name`. No map-name, `LSLoadMessage`, or hardcoded fallback is allowed.
- **MILESTONE-BLOCKING — exact label rectangle/flags/backing:** verified as
  GAME.FNT, left aligned, no backing, ten pixels after the icon, vertically
  centered with native integer truncation, and clipped at
  `base_x + width_override - 3`.
- **MILESTONE-BLOCKING — draw order:** backing/fill precede the country icon,
  and the row label/status follows it. The Rust font-atlas draw must have a
  depth/order that leaves it visible above the row sprites without disturbing
  the four earlier loading-copy layers.
  `[doc: PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md sections 6 and 8]`
- **MILESTONE-BLOCKING — row height:** native uses
  `max(icon_h, bar_h + 6, font_h) + 4`. Rust currently substitutes `bar_h` for
  `font_h`; the available `BitFont::cell_height()` should supply the real metric.
  `[doc: PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md section 6]`
- **MILESTONE-BLOCKING — progress contract preservation:** keep frame 0,
  full-rect scheme backing, the 16-shade player remap, positive-domain `ftol`
  clipped width, monotonic milestone gating, and synchronous presentation on
  advancing values. `[doc: PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md sections 5, 7, and 12]`
- **MILESTONE-BLOCKING — source identity:** side art comes from the resolved
  local country; backing/remap comes from the selected player color; label input
  comes from the verified session-node contract. Do not infer any of these from
  theater, map filename, briefing text, or country description.
- **MILESTONE-BLOCKING — ordinary variants:** verify at least America, Russia,
  and Yuri at 800x600; verify the 640x480 layout branch; verify 3%, 55%, 93%,
  and 100% row states.
- **EXACTIFICATION-RESIDUAL — pixel format:** Rust keys embedded-palette RGB
  before RGBA upload, while native compares converted 16-bit pixels. This is
  equivalent for the retail magenta entries but is not an exhaustive proof over
  palette entries that quantize to the same 16-bit value.
- **EXACTIFICATION-RESIDUAL — malformed/custom PCX:** unusual headers,
  multi-plane files, or non-retail near-magenta colors are outside the stock
  Skirmish delivery matrix.
- **EXACTIFICATION-RESIDUAL — final pixel certification:** exact glyph pixels,
  alpha/blend quantization, exact 3% mask width, milestone dwell, and
  HWND-versus-direct repaint mechanics remain `UNCHECKED`.

## Design

### Components

#### `app_loading_progress_row`

Own a small, CPU-only row model:

- stable text/status input captured from the verified launch/session source;
- `LoadingProgressRowLayout` containing the bar, optional icon, and optional
  label rectangles;
- a pure layout function using render width, frame-0 dimensions, icon
  dimensions, GAME.FNT cell height, and the native width override/constants.

This module must not depend on GPU types, `sim/`, map parsing, or selected-map
preview state. It may depend on small UI geometry types already used by
`app_loading_composition`.

#### `loading_screen_chrome`

Keep country-to-PCX resolution and atlas packing here. The side-icon decode path
uses the PCX embedded palette plus RGB-magenta color key. Non-flag PCXs and SHPs
must not inherit this rule.

Missing or malformed insignia art remains non-fatal: the atlas omits the icon,
and the row layout follows the verified no-icon branch.

#### `app_loading`

`NativeLoadingScreenState` owns the row snapshot so selected-map and random-map
loads share the same row identity. Before every normal or synchronous draw,
`app_loading` constructs one row draw plan from:

- current progress;
- atlas dimensions;
- actual `BitFont::cell_height()`;
- render width;
- the immutable row snapshot.

The same plan feeds both presentation paths. The plan emits row sprites and a
separate GAME.FNT draw for the label/status with an explicit scissor and depth
after the icon.

### Interfaces / Contracts

- `LoadingProgressRowSnapshot::from_launch_session(...)` copies
  `SkirmishLaunchSession::player_name` before the first displayed loading frame.
- `layout_standard_skirmish_progress_row(...)` is pure and returns integer or
  exactly representable pixel geometry. It preserves native truncation and
  centering rules; no scale-to-fit behavior is introduced.
- `render_loading_side_icon(...)` applies RGB-magenta keying only to the
  verified country-insignia PCX family.
- `build_native_loading_row_draw(...)` returns the sprite/text plan consumed by
  both regular and synchronous presentations.
- Empty verified text yields no label draw. No placeholder, map name,
  `LSLoadMessage`, or hardcoded `"Player"` is substituted.

### Data Flow

1. Skirmish launch creates `SkirmishLaunchSession`.
2. `LoadingSession::from_request` captures the native loading variant, player
   color, progress cadence, and progress-row snapshot.
3. The atlas loads the LS background, remapped progress frame, and
   RGB-magenta-keyed country insignia.
4. At draw time, the app supplies atlas dimensions and GAME.FNT cell height to
   the pure row-layout function.
5. The renderer draws the already-established first-render composition.
6. It draws progress backing, clipped progress frame, and country insignia.
7. It draws the verified row label/status through the font atlas with its own
   scissor/depth.
8. Advancing progress milestones rebuild and submit the same row plan; duplicate
   or lower milestones remain suppressed.

### Error Handling

- Missing/malformed PCX: log once during atlas construction, omit the icon, and
  continue loading.
- Missing/empty verified node text: omit the label without inventing fallback
  prose.
- Invalid or zero layout dimensions: omit only the affected row element and
  retain the background/loading flow; unit tests should make this unreachable
  for retail assets.
- GPU upload or surface-acquire failure: preserve the existing non-fatal loading
  presentation handling.

### Testing Strategy

Pure tests:

- RGB magenta becomes alpha zero even when it is not palette index `0`.
- Non-magenta palette index `0` remains opaque.
- Direct-RGB PCX magenta follows the same key rule.
- Row height uses the maximum of icon, `bar_h + 6`, and actual font height,
  followed by `+4`.
- 640 and 800 row origins, icon anchors, label bounds, centering, and truncation
  match the verified formulas.
- Missing-icon and empty-label cases follow the verified branches.
- Composition order is progress backing, progress frame, icon, then row label.
- The normal frame and synchronous presenter receive identical row plans.

Progress regression tests:

- Preserve existing 0/25/50/100 fill-width tests.
- Add representative 3/55/93/100 draw-plan fixtures.
- Preserve duplicate/lower-milestone suppression and terminal-100 presentation.

Retail-asset tests:

- Decode all standard country insignia PCXs and assert that their magenta
  background pixels are transparent while visible pixels remain.
- Confirm `PROGBARM.SHP` frame 0 and GAME.FNT metrics produce nonempty retail
  layouts.
- Keep retail-install-dependent tests ignored or explicitly gated in the same
  manner as existing retail loading tests.

Production visual matrix:

- 800x600 America, Russia, and Yuri selected-map loads at 3%, 55%, 93%, and
  100%;
- 640x480 one representative faction;
- compare icon transparency, row label content, anchor, clipping, color, and
  draw order against a retail capture;
- record exact pixel parity as `UNCHECKED` unless a native-derived executable
  image oracle is added.

## Architectural Decisions

- Keep row identity at app/loading scope rather than in `sim/` or the
  selected-map-only composition snapshot.
- Reuse the existing PCX color-key API instead of adding another decoder or
  applying an SHP palette.
- Add a focused row module because `src/app_loading.rs` is already far beyond
  the preferred module size and the geometry is independently testable.
- Share a CPU row plan between the two presentation paths to prevent hidden
  cadence-dependent visual drift.
- Do not create a generic cross-shell flag/progress widget. Loading-row
  placement and shell-static clipping are different native contracts.
- Do not claim exact mechanism or pixel parity from the retail-convincing
  correction.

No simulation or deterministic-state debt is introduced.

## Alternatives Considered

### Direct in-place patch

Change the loading PCX call, thread `player_name` into the current text builder,
and pass font height into existing geometry functions. This has a smaller
initial diff but keeps a complete native row contract scattered through the
already-large orchestrator and makes the normal/synchronous paths easier to
diverge.

### Generic shared flag/progress component

Centralize loading and shell flags in one reusable render component. Rejected
because shell flag statics use control-centered native-size clipping while the
loading insignia is anchored by ProgressClass row geometry. Combining them
would hide two distinct behavior contracts behind a convenience abstraction.

### Transparency-only correction

Fix only the magenta rectangle. Rejected for the approved scope because it
would knowingly leave the ordinary progress row without its native
text/status handoff and retain the font-height stand-in.
