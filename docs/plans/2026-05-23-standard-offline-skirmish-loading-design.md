# Standard Offline Skirmish Loading Screen Design

## Goal

Replace the current one-frame egui loading placeholder with a pumpable standard
offline Skirmish loading path that renders the verified native gamemd.exe loading
background and progress surface while the map load advances.

## Architecture Context

Current Rust enters `GameScreen::Loading { map_name }` from the main-menu or
native Skirmish shell launch path, renders an egui panel in `ui::main_menu`, then
immediately calls `app_transitions::transition_to_in_game` after presenting that
frame. `transition_to_in_game` calls `app_init::load_map`, which synchronously
performs the whole load and hydrates `AppState` from `MapLoadResult`.

That shape is the core mismatch. It can show only one placeholder frame, so the
player never sees gamemd-style loading composition or milestone cadence.

The existing native shell renderers already provide the local pattern to reuse:
render-side modules load SHP/PCX/PAL assets through `AssetManager`, convert them
to RGBA, pack them into a `BatchTexture`, and app-level render functions draw the
result. The loading screen should follow that app/render pattern. It should not
route through egui for the standard Skirmish parity path, and it must not introduce
any dependency from `sim/` to render, UI, sidebar, audio, or net.

Relevant current surfaces:

- `src/ui/game_screen.rs`: `GameScreen::Loading { map_name }` currently carries
  only the requested map name.
- `src/app.rs`: launches loading from Skirmish, renders the loading screen, then
  synchronously transitions after one present.
- `src/app_transitions.rs`: owns post-load `AppState` hydration.
- `src/app_init.rs`: `load_map` owns the whole map-load pipeline and returns
  `MapLoadResult`.
- `src/render/skirmish_shell_chrome.rs` and sibling shell/sidebar render modules:
  established SHP/PAL atlas pattern for native UI chrome.
- `src/assets/pal_file.rs` and `src/assets/shp_file.rs`: current PAL conversion
  and SHP frame-to-RGBA conversion. The current PAL parser is not sufficient for
  gamemd UI/loading parity because it normalizes `63 -> 255`.

## Impact Analysis

Primary touched modules:

- `src/app.rs`: loading render branch, post-present transition hook, Skirmish
  launch state creation, and `AppState` fields.
- `src/app_transitions.rs`: split "hydrate app from `MapLoadResult`" from
  "synchronously call `load_map`".
- `src/app_init.rs`: split the synchronous load body into reusable phases or a
  pumpable loader object while preserving the existing final `MapLoadResult`.
- New app-level loading module, likely `src/app_loading.rs`: owns `LoadingSession`,
  loader phase state, milestone state, and phase pumping.
- New render module, likely `src/render/loading_screen_chrome.rs`: owns loading
  atlas entries and native loading draw data.
- `src/assets/pal_file.rs` or a small adjacent helper: add a gamemd-compatible
  UI/loading PAL conversion path.
- `src/ui/main_menu.rs`: keep the egui fallback only outside the standard Skirmish
  parity path, or remove the standard Skirmish use of `draw_loading_screen`.

Risk areas:

- Borrowing and ownership: `load_map` currently borrows GPU, batch renderer, and
  optional VXL compute renderer while creating render resources. A pumpable loader
  should own CPU intermediates and request GPU work during `pump(&mut AppState)`,
  not store long-lived borrows.
- Result hydration: the large `MapLoadResult -> AppState` assignment block should
  remain centralized so the new path does not silently diverge from the old path.
- Progress accuracy: do not invent evenly-spaced percentages for unverified phases.
  The loading state can support native milestone values, but only verified values
  should drive visible parity claims.
- Palette behavior: existing PAL conversion behavior is used by other render paths.
  Add a separate gamemd UI/loading conversion path rather than changing every
  current palette user at once.
- Asset lifetime: the new loading session should preserve the `AssetManager` for
  both loading UI assets and the final `MapLoadResult`.

No `sim/` dependency changes are required. This is an app/render/assets refactor.

## Chosen Approach

Use an app-level pumpable loading state.

`GameScreen::Loading` remains the user-visible mode, but `AppState` gains a
`LoadingSession` that owns the loading renderer state, native progress state, and
an incremental map-load job. Each frame:

1. Render the current native loading surface.
2. Present it.
3. Pump one or more bounded load phases.
4. If a phase emits a higher verified native milestone, update the loading progress
   state so the next frame repaints.
5. When the loader finishes, hydrate `AppState` from the returned `MapLoadResult`
   and enter `InGame` or `SpawnPick`.

This preserves the existing app/render boundary, keeps GPU resource creation on
the main thread, avoids worker-thread ownership issues, and creates a concrete
home for native milestone suppression.

## Tiny-Detail Ledger

- Standard offline Skirmish target is `g_GameMode == 5`; campaign loading metadata
  is not part of this design. Source:
  `LSLOADMESSAGE_SKIRMISH_LOADING_TEXT_SPLIT_GHIDRA_REPORT.md`.
- First verified standard Skirmish renderer is `0x00552D60`, not
  `WM_PAINT_Handler` mode 2. Source:
  `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`.
- First renderer draws `ls640/ls800<country>.shp` through `MPLS*.PAL` /
  `MPYLS.PAL`; do not use `PUDLGBG*` for this path. Source:
  `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`.
- Mode-2 `PUDLGBGN/A/S/Y.SHP` is a separate branch and is loaded-but-inactive for
  this first standard Skirmish loading renderer. Source:
  `PUDLGBG_LOADING_SCREEN_SHP_LIFECYCLE_GHIDRA_REPORT.md`.
- Progress setup derives side from the first session node country, with the native
  path mapping the first node country through house side data and storing it on
  ProgressClass. Source: `LOAD_PROGRESS_MANAGER_SETUP_GHIDRA_REPORT.md`.
- The first renderer occurs before `FUN_0069AE90(3)`, so the loading background
  must be available before progress milestone `3` is applied. Source:
  `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`.
- Standard Skirmish progress uses `PROGBARM.SHP`, frame `0`. Source:
  `PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md`.
- Progress fill width is clipped, not scaled:
  `ftol(frame0_width * lane_value / max_value)`. Source:
  `PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md`.
- Native progress placement uses the verified Skirmish origin/width model:
  default-width base `+0x0C,+0x100`, wider case `+0x10,+0x141`, and width override
  `0x146/0x196`, with helper insets layered after that. Source:
  `LOAD_PROGRESS_MANAGER_SETUP_GHIDRA_REPORT.md` and
  `PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md`.
- Duplicate or lower progress milestones do not repaint; only changed advancing
  values produce a visible update. Source:
  `PROGRESSCLASS_REPAINT_CADENCE_HWND_GHIDRA_REPORT.md`.
- Standard scenario load direct-draws because the ProgressClass HWND field is null;
  do not model this as a native `msctls_progress32` control for the Skirmish path.
  Source: `PROGRESSCLASS_REPAINT_CADENCE_HWND_GHIDRA_REPORT.md`.
- Skirmish loading must not render Rust-invented text: no "Mission deployment",
  no "Loading...", no `Map: ...`, and no explanatory status sentence. Source:
  `LSLOADMESSAGE_SKIRMISH_LOADING_TEXT_SPLIT_GHIDRA_REPORT.md` and current Rust
  `src/ui/main_menu.rs`.
- `LSLoadMessage`, `LSLoadBriefing`, `Briefing`, and `UIName` loading text are
  campaign-only for this scope and must be ignored for standard Skirmish. Source:
  `LSLOADMESSAGE_SKIRMISH_LOADING_TEXT_SPLIT_GHIDRA_REPORT.md`.
- UI/loading PAL conversion must use gamemd's component shift path:
  each component is `value << 2`, so raw `63` becomes `252`, not `255`. Source:
  `DIALOG_PALETTE_STARTUP_0072AA40_GHIDRA_REPORT.md`.
- PAL conversion itself should not assign alpha; SHP transparency handling belongs
  at SHP/frame conversion or render composition time. Source:
  `DIALOG_PALETTE_STARTUP_0072AA40_GHIDRA_REPORT.md`.
- Selected Skirmish map launch semantics should continue to use the selected map
  file token, not display label text. Source:
  `SKIRMISH_START_TO_LOADING_SCREEN_ACTIVATION_GHIDRA_REPORT.md` and current
  Rust launch fields.
- `mmpb.shp` marker overlay is known to be conditional after the LS background, but
  exact gates, frames, and interactions are blocked. Source:
  `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`; blocker tracked in the
  implementation contract.
- Localized text after the marker pass is known to exist in the renderer area, but
  exact displayed strings are blocked. Source:
  `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`; blocker tracked in the
  implementation contract.

## Design

### Components

#### `app_loading`

New app-level module that owns the pumpable state machine.

Core types:

```rust
pub struct LoadingSession {
    request: LoadingRequest,
    screen: NativeLoadingScreenState,
    job: LoadingJob,
}

pub struct LoadingRequest {
    requested_map: Option<String>,
    skirmish_launch_session: Option<SkirmishLaunchSession>,
    skirmish_settings: SkirmishSettings,
}

pub struct NativeLoadingScreenState {
    side: LoadingSide,
    progress: LoadingProgressState,
    atlas: Option<LoadingScreenAtlas>,
    first_renderer_ready: bool,
}

pub struct LoadingProgressState {
    max_value: u32,
    current_value: u32,
    last_drawn_value: Option<u32>,
}
```

`LoadingSession` must not borrow `AppState`; it receives mutable app state only
when pumped or rendered. This avoids storing references to GPU/batch/VXL resources.

`LoadingSide` is a small render/app enum that represents the loading-art side
selected by gamemd for this screen. It should be derived once from the first launch
session node country for native Skirmish. It should not read map theater, sidebar
theme, or display text as a fallback when a session exists.

#### `LoadingJob`

`LoadingJob` is the incremental form of `app_init::load_map`. It owns CPU
intermediate data between phases and eventually produces the same `MapLoadResult`
that `load_map` currently returns.

The implementation can be introduced by extracting the existing `load_map` body
into phase helpers before making it fully pumpable. The final design target is:

```rust
pub enum LoadingPump {
    Pending,
    RepaintRequested,
    Finished(MapLoadResult),
    Failed(anyhow::Error),
}
```

Representative phases:

- Create config and `AssetManager`.
- Build native loading atlas and render first loading background.
- Resolve requested map file and parse `MapFile`.
- Load theater data, rules, art, CSF, and infantry sequences.
- Resolve terrain, lighting, tile atlas, height maps, and bridge maps.
- Spawn simulation entities and apply Skirmish launch session.
- Build entity, overlay, bridge, sidebar, cursor, and font render resources.
- Build path grid and final transient runtime structures.
- Return `MapLoadResult`.

The phases should be chosen around existing data dependencies, not around made-up
progress percentages. Visible progress values should come from verified native
milestones only.

#### `render::loading_screen_chrome`

New render module for native loading assets.

Responsibilities:

- Load the correct `ls640/ls800<country>.shp` asset for the selected side/country.
- Load `MPLS*.PAL` / `MPYLS.PAL` with gamemd-compatible PAL conversion.
- Load `PROGBARM.SHP` frame `0`.
- Pack the background and progress image into a `BatchTexture`.
- Expose simple entries with pixel dimensions and UVs.
- Provide draw helpers that can render:
  - the full background at the verified anchor;
  - the clipped progress fill rect using runtime frame dimensions;
  - future extension slots for `mmpb.shp` and native text after their blockers close.

The renderer should follow the shell chrome atlas pattern: absent optional blocked
layers should log clearly and not substitute invented art.

#### Palette Conversion

Add a separate gamemd-compatible PAL conversion path, for example:

```rust
impl Palette {
    pub fn from_bytes_gamemd_ui(data: &[u8]) -> Result<Self, AssetError>;
}
```

This function maps `component << 2` and should not itself encode transparency
policy. If the existing `ShpFile::frame_to_rgba` requires alpha in the palette,
add a frame conversion helper for native UI SHP rendering that applies the SHP
transparent index/chroma handling at conversion time instead of baking that policy
into the PAL parser.

Do not change the existing normalized `Palette::from_bytes` globally in this pass.

#### `app_transitions`

Split current `transition_to_in_game` into:

- `apply_map_load_result(state: &mut AppState, result: MapLoadResult)`, containing
  the current result hydration block.
- A compatibility wrapper that can still perform the old synchronous load when
  needed by tests or non-native paths.

The pumpable path calls only the hydration function once `LoadingJob` finishes.

### Interfaces / Contracts

#### Starting Loading

Skirmish launch code should create:

```rust
LoadingRequest {
    requested_map: Some(selected_map_file),
    skirmish_launch_session: state.pending_skirmish_launch_session.clone(),
    skirmish_settings: state.skirmish_settings.clone(),
}
```

Then:

- store `LoadingSession` on `AppState`;
- set `GameScreen::Loading { map_name }` or replace the payload with a thinner
  marker once all callers are migrated;
- enter game window mode and reset zoom as today.

#### Rendering Loading

`GameScreen::Loading` render branch should call an app render helper, not egui:

```rust
app_loading::render_loading(state, &mut encoder, &view)?;
```

The helper draws the current native loading state. If the loading atlas has not
been built yet, it may clear to black for the first frame, then immediately pump
asset setup after present. It must not draw the current egui text panel for the
standard Skirmish path.

#### Pumping Loading

After `output.present()`, replace the unconditional synchronous transition with:

```rust
if matches!(state.screen, GameScreen::Loading { .. }) {
    app_loading::pump_loading_after_present(state);
}
```

`pump_loading_after_present` advances bounded work and returns control to the event
loop unless the job is complete. Completion calls `apply_map_load_result`.

The first implementation may pump more than one small CPU phase per frame if needed
for startup practicality, but it must never immediately consume the whole load in
the same frame unless explicitly running a fallback/test path. The player-visible
surface must have opportunities to repaint between changed native milestones.

#### Progress Events

Use an explicit method:

```rust
impl NativeLoadingScreenState {
    pub fn advance_progress(&mut self, value: u32) -> bool;
}
```

Rules:

- If `value <= current_value`, return `false` and do not request repaint.
- If `value > current_value`, update state and return `true`.
- The renderer clips `PROGBARM.SHP` frame `0` to the computed width on draw.
- Do not interpolate or animate between values.

### Data Flow

1. Native Skirmish shell start button produces a `SkirmishLaunchSession` and
   selected map filename, as it does today.
2. App creates a `LoadingSession`.
3. Loading session derives `LoadingSide` from the first launch node country.
4. First loading phase creates `AssetManager`, loads the native loading atlas, and
   marks the first renderer ready.
5. App renders native loading background.
6. Loader emits verified native milestone `3` after the first renderer point.
7. Later loader phases continue and emit only verified milestone values.
8. Duplicate/lower milestone requests are suppressed by `LoadingProgressState`.
9. When all phases finish, loader returns `MapLoadResult`.
10. App applies the result through the extracted hydration function and enters
    `InGame` or `SpawnPick`.

### Error Handling

Failure behavior should match the existing fallback spirit without drawing false
native parity:

- If loading assets are missing, log the exact missing asset and use a minimal black
  loading surface rather than egui text in the standard Skirmish parity path.
- If map loading fails, preserve the current behavior of returning a default/empty
  `MapLoadResult` only if the existing wrapper path still wants that fallback.
  The pumpable path should surface the error clearly in logs and transition to a
  controlled menu/fallback state rather than silently showing invented loading UI.
- Optional blocked layers (`mmpb.shp`, post-marker text) must remain absent until
  their gates are verified. Do not substitute setup-screen marker semantics.

### Testing Strategy

Unit-level tests:

- `pal_file_gamemd_shift_left_two_maps_63_to_252`: validates the new PAL path.
- `loading_progress_duplicate_milestones_do_not_redraw`: feeds `3, 3, 2, 8` and
  asserts only `3` and `8` request repaint.
- `loading_progress_skirmish_draws_progbarm_frame0_clipped_to_native_percent`:
  checks clipped width math from runtime frame dimensions.
- `loading_side_comes_from_first_launch_node_country`: builds a launch session with
  a known first node and checks `LoadingSide`.
- `skirmish_launch_uses_selected_record_filename_buffer`: preserves existing
  filename-not-display-label launch semantics.

Render/visual tests:

- Build a loading atlas from retail assets and assert the expected background and
  `PROGBARM.SHP` entries exist for Allied, Soviet, and Yuri-local variants.
- Capture first loading render before milestone `3`; assert no egui loading text is
  present in the standard Skirmish path.
- Capture progress at known values and compare clipped bar widths.

Flow tests:

- Launch standard offline Skirmish and assert the app remains in loading across
  more than one frame when the pumpable path is active.
- Feed a Skirmish map containing campaign `LSLoadMessage` / `LSLoadBriefing`
  metadata and assert those strings are ignored by this path.

Manual parity check after implementation:

- Run stock Skirmish starts for Allied, Soviet, and Yuri first-player countries.
- Compare first loading background and progress-bar placement against a gamemd.exe
  capture at the same resolution.
- Run a focused follow-up check after the `mmpb.shp` and post-marker text blockers
  are closed.

## Architectural Decisions

- Keep loading orchestration in app-level code. Map loading already belongs to the
  app layer and may depend on render, assets, rules, map, and sim. Moving this into
  `sim/` would violate project layering.
- Keep final `MapLoadResult` as the app hydration boundary. This limits churn and
  keeps the eventual in-game state identical to the current synchronous path.
- Add a native loading renderer rather than extending `ui::main_menu`. The target
  surface is retail SHP/PAL composition, not egui layout.
- Split gamemd UI PAL conversion from the existing general PAL parser. This avoids
  destabilizing unrelated render paths while closing the verified loading/UI gap.
- Do not fake unknown progress values, marker gates, or localized text. The design
  provides extension points, but the implementation must wait for verified evidence
  before drawing those layers.

Tech debt deliberately introduced:

- `GameScreen::Loading { map_name }` may temporarily keep `map_name` for legacy
  callers while `LoadingSession` carries the real request. Once all loading paths
  use `LoadingSession`, the enum payload should be reduced or renamed.
- The first pumpable extraction may preserve a synchronous compatibility wrapper.
  That wrapper should remain test/fallback-only and should not be used by the native
  Skirmish path.

## Alternatives Considered

### Background Loader Thread Plus Progress Channel

This would keep the event loop very responsive, but current loading creates GPU
resources throughout `load_map`. Splitting CPU work from GPU work would become a
larger architecture change than needed for this parity pass, and it would increase
the risk of hidden render-resource lifetime bugs.

### Native-Looking Facade Around Synchronous Load

This would draw the correct art once and then call the current synchronous
`transition_to_in_game`. It is not acceptable for parity because the player would
still not see gamemd-style milestone cadence, duplicate suppression, or real
progress-driven repaints.

### Implement Mode-2 `PUDLGBG*` First

Mode-2 loading background research is useful but not the first standard offline
Skirmish renderer. Implementing it first would spend effort on a separate branch
while leaving the visible Skirmish Start Game path wrong.
