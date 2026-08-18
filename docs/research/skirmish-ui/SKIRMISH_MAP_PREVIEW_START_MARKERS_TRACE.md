# Skirmish Map Preview Start Markers Trace

Scenario: open offline Skirmish dialog `0x102` on a normal skirmish map with
start waypoints. Concrete map instance used for local values:
`<ra2-install>/Dustbowl.map`.

Correction 2026-05-21: this trace originally treated loose `Dustbowl.map` as a
live `STARTBUT.SHP` overlay case. Newer Ghidra reports show that is stale.
Loose Dustbowl lacks `[Header]` preview start metadata, so the active selected-map
preview path resets `ScenarioClass+0x113C` to `-1`; `DrawStartPositions` therefore
skips live `STARTBUT.SHP` overlays. Any visible Dustbowl start marks come from
baked red pixels inside `[PreviewPack]`, not from the live overlay path.

Scope: preview surface rendering, `STARTBUT.SHP` marker placement, numeric labels, clipping, and marker offsets only.

## Verdict Summary

PASS: 1 | FAIL: 0 | UNCHECKED: 6 | NOT-IMPLEMENTED: 5

Overall status: the standard YR `DrawStartPositions` path actively draws the
preview surface, then conditionally draws `STARTBUT.SHP` frame `0` and labels
`1..N` only when `ScenarioClass+0x113C` is in `1..8`. For loose Dustbowl, that
count is `-1`, so gamemd draws the preview surface but no live `STARTBUT.SHP`
overlay. The Rust Skirmish shell still lacks the real preview surface, so it also
misses Dustbowl's baked `[PreviewPack]` red start pixels.

## Evidence Used

- `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`: dialog `0x102`, map preview child `0x468`, and `DrawStartPositions @ 0x00640710`.
- `SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`: active offline Skirmish `WM_PAINT` order and `DrawStartPositions` sequence.
- `SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`: start waypoints `0..7`, contiguous scan, and start-marker UX.
- `SCENARIO_PREVIEW_HEADER_DEFAULTS_AND_DUSTBOWL_SOURCE_PATH_GHIDRA_REPORT.md`:
  loose Dustbowl lacks `[Header]`, leaving live overlay count `-1`.
- `SKIRMISH_MAP_PREVIEW_MARKER_ASSET_LAYOUT_GHIDRA_REPORT.md`: live marker
  overlays are `STARTBUT.SHP` frame `0`, separate from baked red preview pixels.
- Read-only Ghidra spot-check: `DrawStartPositions @ 0x00640710` and offline Skirmish window proc `0x006AE3F0`.
- Rust files: `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/app_list_maps.rs`, `src/map/preview.rs`, `src/map/waypoints.rs`.

Active in standard YR: yes for the `DrawStartPositions` paint path. Live
`STARTBUT.SHP` markers are conditional on `0 < ScenarioClass+0x113C < 9`.
The offline Skirmish `WM_PAINT` branch at `0x006AE3F0` calls
`DrawStartPositions @ 0x00640710` when the preview state exists and the preview
child `0x468` is available. `mmpb.shp` is active elsewhere, but not in this
standard offline `0x102` numbered-marker path.

## Concrete Local Inputs

`Dustbowl.map` has:

- `[Preview] Size=0,0,138,75`
- `[PreviewPack]` present and non-empty.
- `[Map] LocalSize=2,8,65,62`
- `[Waypoints] 0=116070`, `[Waypoints] 1=34079`
- no `[Header] NumberStartingPoints` / `[Header] WaypointN` preview overlay
  metadata in the verified loose retail file.

For this loose-map path, `DrawStartPositions` can blit the decoded preview
surface, but it skips live `STARTBUT.SHP` overlays because
`ScenarioClass+0x113C == -1`. The `[Waypoints]` entries remain gameplay start
data and can also correspond to baked preview pixels; they are not the live
overlay count source for loose Dustbowl.

Rust waypoint parsing for `NewINIFormat >= 4` uses `rx = value % 1000`, `ry = value / 1000`, so local parsed starts are:

- start `0`: `(rx=70, ry=116)`
- start `1`: `(rx=79, ry=34)`

These starts are exposed through `MapMenuEntry.multiplayer_start_waypoints`, but the active Skirmish shell renderer does not consume that field.

## Pipeline

1. Open offline Skirmish dialog `0x102`.
2. WM_PAINT reaches the preview-marker branch.
3. Locate preview child `0x468`.
4. Compute preview child/backbuffer rect and preview source bounds.
5. Blit decoded map preview surface.
6. Read live overlay count from `ScenarioClass+0x113C`, only accepting `1..8`.
7. If the count is valid, project `[Header]` preview start cells to preview
   pixels.
8. If projected, draw `STARTBUT.SHP` frame `0` at projected point plus
   `(-9,-6)`.
9. If projected, draw numeric label `i + 1` after the marker shape.

For loose Dustbowl, steps 7-9 are skipped; baked red preview pixels are already
part of the decoded `[PreviewPack]` image.

## Stage Results

### Stage 1 - Dialog Preview Child

gamemd: dialog `0x102` contains preview child ID `0x468` at dialog rect `(429,23,96,69)`, and `DrawStartPositions` looks up `GetDlgItem(hwnd, 0x468)`.

Rust: `compute_layout` defines `MapPreview0x468` and maps it to `layout.map_preview`; at `800x600` tests expect `RectPx(644,37,144,112)`.

Verdict: UNCHECKED. The child identity matches, but literal Win32 client/backbuffer pixel equality for the active runtime rectangle was not computed from both engines in this trace.

### Stage 2 - Preview Surface Availability

gamemd: `DrawStartPositions` locks/clips `DAT_00887310` and blits the decoded preview surface before markers.

Rust: `real_preview_surface_available()` returns `false`, so `build_skirmish_shell_instances` does not render a preview surface and marker helpers return early.

Verdict: NOT-IMPLEMENTED. Player sees no real map thumbnail backing in the Rust Skirmish shell.

### Stage 3 - PreviewPack Decode

gamemd: the preview object supplies source bounds and a drawable preview surface through its vtable.

Rust: `PreviewSection` records metadata and `has_packed_preview`, but `[PreviewPack]` image decode and surface upload are not present in the Skirmish shell path. `parse_preview_size` also records only the first two comma fields, so `Dustbowl.map` becomes `Some((0,0))` instead of the drawable preview dimensions `(138,75)`.

Verdict: NOT-IMPLEMENTED. The renderer has no decoded preview pixels to draw.

### Stage 4 - Source Bounds

gamemd: marker projection reads source bounds from `ScenarioClass+0x112C`
through `ScenarioClass+0x1138`, but loose Dustbowl leaves these preview-overlay
fields reset to `-1` on the active selected-map path.

Rust: `preview_source_bounds_from_verified_source` always returns `None`; the renderer receives no source-bounds value.

Verdict: NOT-IMPLEMENTED. Rust cannot compute the same map-to-thumbnail projection.

### Stage 5 - Live Overlay Count

gamemd: `DrawStartPositions` reads `ScenarioClass+0x113C` and draws live
`STARTBUT.SHP` overlays only when `0 < count < 9`. For loose Dustbowl, newer
evidence shows the active selected-map preview path resets this field to `-1`
because the loose file lacks `[Header] NumberStartingPoints`.

Rust: `multiplayer_start_waypoints` filters and sorts waypoints `0..=7`; for `Dustbowl.map` it finds two starts.

Verdict: UNCHECKED for parity and corrected from the earlier PASS. Rust finds two
gameplay waypoints, but that is not the same source as gamemd's live overlay
count for loose Dustbowl. Drawing `STARTBUT.SHP` from `[Waypoints]` for this map
would be wrong; only the baked `[PreviewPack]` red pixels are expected.

### Stage 6 - Coordinate Projection

gamemd: when `ScenarioClass+0x113C` is valid, it computes integer scaled
coordinates from source bounds to preview dimensions using `*1000` integer math
and truncating divisions. Loose Dustbowl does not enter this loop.

Rust: no active projection function feeds the Skirmish shell; `push_start_marker_sprites` and `push_start_marker_labels` are called with `&[]`.

Verdict: NOT-IMPLEMENTED. Player-visible marker positions are absent.

### Stage 7 - STARTBUT Asset

gamemd: lazily loads `STARTBUT.SHP` and draws frame `0` only on maps/paths with
valid live overlay count `1..8`.

Rust: `build_skirmish_shell_chrome_atlas` loads `STARTBUT.SHP` frame `0` into `atlas.start_marker`.

Verdict: UNCHECKED. File and frame selection match, but pixel/palette equality was not compared because the active draw path is gated off.

### Stage 8 - Marker Offset

gamemd: marker shape draw applies X offset `-9` and Y offset `-6` when the live
overlay loop runs. Loose Dustbowl skips the loop.

Rust: `START_MARKER_OFFSET_X = -9`, `START_MARKER_OFFSET_Y = -6`, and `push_start_marker_sprites` applies those constants before drawing.

Verdict: PASS for the dormant helper math. It is not player-visible yet because the helper returns early and receives no positions.

### Stage 9 - Marker Clipping

gamemd: clips drawing through the target surface lock/clip rect derived from the preview child.

Rust: marker helper only tests the unoffset anchor point with `preview_rect.contains(x, y)`; there is no active preview-surface clip because the helper is gated off.

Verdict: UNCHECKED. The active output is absent; if enabled, clipping semantics still need exact comparison for partially overlapping `STARTBUT.SHP` pixels near edges.

### Stage 10 - Numeric Labels

gamemd: after each live marker shape, draws numeric label `i + 1`. Loose
Dustbowl skips this label path because the overlay count is `-1`.

Rust: `push_start_marker_labels` would generate `(idx + 1).to_string()`, but it is called with an empty position list and returns early while `real_preview_surface_available()` is false.

Verdict: NOT-IMPLEMENTED for the active scenario. The dormant string generation matches the label numbering rule, but no label is visible.

### Stage 11 - Draw Order

gamemd: preview surface first; if live overlay count is valid, `STARTBUT.SHP`
marker second and numeric label third. Loose Dustbowl only uses the preview
surface path for visible start marks, via baked red pixels in `[PreviewPack]`.

Rust: semantic draw order records the same relative order only when `real_preview_surface_available` is passed as true, but the active function returns false.

Verdict: UNCHECKED. Intended order is represented in tests, but active runtime output has no preview, markers, or labels.

### Stage 12 - Timing

gamemd: draw happens in the dialog `WM_PAINT` pass after common shell paint and before `ValidateRect`.

Rust: Skirmish shell rendering happens in the app render pass, not Win32 `WM_PAINT`.

Verdict: UNCHECKED. No literal frame/tick timing comparison was computed; visible output is currently missing regardless of timing.

## Player-Visible Findings

1. NOT-IMPLEMENTED - Preview surface: Rust displays no decoded map thumbnail in
   the Skirmish dialog; gamemd blits the map preview surface. For loose Dustbowl,
   this surface includes baked red start pixels from `[PreviewPack]`.
2. CONDITIONAL - Live `STARTBUT.SHP` marker coordinates: Rust passes an empty
   position list, so no live overlays appear. That is not a Dustbowl divergence,
   because loose Dustbowl also skips live overlays. It remains unimplemented for
   maps/generated paths that populate `ScenarioClass+0x113C` in `1..8`.
3. CONDITIONAL - Numeric labels: Rust draws no live marker labels. Loose Dustbowl
   also draws none; labels remain required for valid live overlay paths.
4. CONDITIONAL - Source bounds: Rust never populates verified preview source
   bounds; gamemd uses those bounds only when the live overlay count is valid.
5. NOT-IMPLEMENTED - PreviewPack decode: Rust records metadata only; gamemd uses a real preview surface object.

## Adjacent Findings

- `parse_preview_size` currently treats `[Preview] Size=0,0,138,75` as `(0,0)`. That is likely wrong for preview image dimensions, but a separate preview decode trace should verify the full `[Preview]` field semantics before changing it.
- `mmpb.shp` is intentionally not part of this offline available-start marker path. It belongs to a separate assigned-player/house marker context.
- Do not draw live `STARTBUT.SHP` overlays for loose Dustbowl from `[Waypoints]`.
  The verified loose-map path expects no live overlay because `[Header]` preview
  metadata is absent; baked red pixels in `[PreviewPack]` are the relevant visible
  start marks.
