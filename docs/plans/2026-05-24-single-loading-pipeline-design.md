# Single Loading Pipeline Design

## Goal

Migrate every production map-load entry point onto one `LoadingSession` / `LoadingJob` pipeline, so native Skirmish loading is not mixed with legacy synchronous transition state.

## Architecture Context

Current Rust has a partially pumpable loading path:

- `src/app_loading.rs` owns `LoadingSession`, `LoadingJob`, native loading progress, and the first `MapLoadInitial` phase.
- `src/app_init.rs` now splits the first boundary into `load_map_initial_with_assets(...)` and `load_map_from_initial(...)`.
- `src/app.rs` renders `GameScreen::Loading`, presents, then calls `app_loading::pump_loading_after_present`.
- `src/app_transitions.rs::apply_map_load_result` remains the single hydration point from `MapLoadResult` into `AppState`.

The remaining architectural problem is duplicate entry and truth sources:

- `start_selected_skirmish` calls `begin_legacy_loading`.
- `start_skirmish_session` calls `begin_skirmish_loading`.
- `RA2_QUICKPLAY` still initializes `GameScreen::Loading { map_name: "auto" }` directly, without a `LoadingSession`.
- `app_transitions::transition_to_in_game` and `app_init::load_map` still preserve the old synchronous wrapper shape.
- `GameScreen::Loading { map_name }`, `LoadingSession.request.selected_map_file`, and `pending_skirmish_launch_session` can overlap.

The target architecture is: `GameScreen::Loading` says only which screen is active; `LoadingSession` is the loading source of truth; `LoadingJob` owns the map-load state; and `apply_map_load_result` is the only final app hydration path.

This stays entirely above `sim/`. No `sim/` dependency on render, UI, sidebar, audio, or net is introduced.

## Impact Analysis

Touched modules:

- `src/app_loading.rs`: replace separate begin helpers with a single request/session constructor and loading mode enum; own all production loading jobs.
- `src/app.rs`: all load starts call `app_loading::begin_loading`; no caller sets `GameScreen::Loading` directly.
- `src/app_init.rs`: keep phase helpers; remove or quarantine production use of the synchronous `load_map` wrapper.
- `src/app_transitions.rs`: keep `apply_map_load_result`; remove production `transition_to_in_game` usage and eventually delete or test-gate it.
- `src/ui/game_screen.rs`: make `Loading` stop carrying authoritative map data, either immediately or after a small compatibility step.

Risk areas:

- `RA2_QUICKPLAY` currently has no `LoadingSession`; migrating it is required to avoid a stuck loading screen.
- Native Skirmish failure must not fall back to egui map-name text.
- Generic quickplay/dev loads may still use non-native visuals, but must use the same job and pump API.
- Removing `pending_skirmish_launch_session` from app-level ownership affects any code still reading it after loading begins.
- A full `load_map_from_initial` phase split remains the hardest follow-up; this design prevents caller drift before that split continues.

## Chosen Approach

Use one production `LoadingSession` pipeline for every map-load caller.

`LoadingSession` receives a `LoadingRequest` with:

- selected map token;
- optional `SkirmishLaunchSession`;
- legacy `SkirmishSettings` fallback data needed until all setup paths are native-session based;
- loading presentation mode.

Presentation mode is explicit:

- `NativeSelectedSkirmish`: verified standard offline Skirmish native LS background/progress path.
- `GenericMapLoad`: quickplay/dev/temporary non-native loading presentation, still pumped through the same `LoadingJob`.

`GameScreen::Loading` remains screen state only. It must not be used to decide which map to load or which launch session to apply.

`app_init::load_map` must not remain a production app API. If a synchronous run-to-completion helper is needed for tests or tools, it must be explicitly named as such and implemented over the same `LoadingJob` phases. The app must not keep a second loading implementation.

## Tiny-Detail Ledger

- Standard offline Skirmish reaches first renderer `0x00552D60`, then immediately milestone `3`; no progress draw occurs before the first LS composition. Source: `docs/research/LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`.
- First native Skirmish background is `ls640/ls800<country>.shp` using `MPLS*.PAL` / `MPYLS.PAL`, not `PUDLGBG*`. Source: `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`.
- `mmpb.shp` assigned-player marker overlay is after LS background and conditional; it is not the first step in this migration. Source: `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`.
- `PROGBARM.SHP` is configured before the first renderer, but first progress update is after it via milestone `3`. Source: `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`.
- Standard Skirmish progress is null-HWND direct draw; advancing milestones draw/blit before loader execution continues. Source: `docs/research/LOADING_DIRECT_DRAW_REPAINT_PRESENT_PATH_GHIDRA_REPORT.md`.
- Duplicate/lower progress milestones do not repaint. Source: `LOADING_DIRECT_DRAW_REPAINT_PRESENT_PATH_GHIDRA_REPORT.md`; `docs/research/LOADING_PROGRESSCLASS_VALUE_MAX_MAPPING_GHIDRA_REPORT.md`.
- `ProgressClass` max is `100.0`, standard Skirmish lane count is `1`, and milestone percent maps to lane value by `max * 0.01 * percent`. Source: `LOADING_PROGRESSCLASS_VALUE_MAX_MAPPING_GHIDRA_REPORT.md`.
- Progress fill width is clipped from `PROGBARM.SHP` frame `0` using `Math__ftol(frame0_width * lane_value / max)`, not scaled. Source: `LOADING_PROGRESSCLASS_VALUE_MAX_MAPPING_GHIDRA_REPORT.md`.
- Selected-map `Full_Init` visible milestone order includes nested helper milestones, not only direct outer calls. Source: `docs/research/LOADING_FULL_INIT_PROGRESS_SEQUENCE_AFTER_00552D60_GHIDRA_REPORT.md`.
- Visible standard selected-map milestones include `3, 8, 12, 25, 30, 31, 35, 45, 50, 55, 58, 60, 63, 65, 67, 68, 69, 70, 72, 74, 76, 78, 82, 86, 90, 93, 96, 98, 100`, with lower/duplicate raw calls suppressed. Source: `LOADING_FULL_INIT_PROGRESS_SEQUENCE_AFTER_00552D60_GHIDRA_REPORT.md`.
- Native Skirmish loading must not show Rust egui loading text, selected map label text, or explanatory status text. Source: `docs/research/LSLOADMESSAGE_SKIRMISH_LOADING_TEXT_SPLIT_GHIDRA_REPORT.md`; current Rust `src/ui/main_menu.rs`.
- Campaign loading metadata and text are not part of standard Skirmish native loading. Source: `LSLOADMESSAGE_SKIRMISH_LOADING_TEXT_SPLIT_GHIDRA_REPORT.md`.

## Design

### Components

#### `LoadingRequest`

Owns all data needed to start loading:

```rust
pub(crate) struct LoadingRequest {
    pub selected_map_file: String,
    pub launch: LoadingLaunch,
    pub presentation: LoadingPresentation,
    pub fallback_skirmish_settings: SkirmishSettings,
}
```

`LoadingLaunch`:

- `Skirmish(SkirmishLaunchSession)`
- `Generic`

`LoadingPresentation`:

- `NativeSelectedSkirmish`
- `GenericMapLoad`

The map token in this struct is authoritative. `GameScreen::Loading` does not own it.

`GenericMapLoad` is presentation only. It is not a second loader and must use the same `LoadingJob`, phase state, error path, and final `apply_map_load_result` hydration path as native Skirmish loading.

#### `LoadingSession`

Owns:

- `LoadingRequest`;
- optional `NativeLoadingScreenState`;
- `LoadingJob`;
- first-frame/presentation flags.

Only `app_loading` creates `LoadingSession`.

#### `LoadingJob`

Owns:

- RA2 install path;
- job-owned `AssetManager`;
- extracted map/load intermediates;
- current phase.

The native loading atlas must be built from the same job-owned `AssetManager` that later moves into `MapLoadResult.asset_manager`.

#### `GameScreen`

Recommended target:

```rust
pub enum GameScreen {
    MainMenu,
    Loading,
    SpawnPick,
    InGame,
    MissionResult { title: String, detail: String },
}
```

If this is too disruptive for one patch, keep `Loading { map_name }` temporarily but forbid reading it for loading decisions. The migration is not complete until the payload is gone or display-only.

Hard rule after migration: no production code constructs `GameScreen::Loading` directly. All loading starts go through `app_loading::begin_loading`.

### Interfaces / Contracts

Production entry point:

```rust
pub(crate) fn begin_loading(state: &mut AppState, request: LoadingRequest)
```

This is the only production entry point for starting a map load.

Pump entry point:

```rust
pub(crate) fn pump_loading_after_present(state: &mut AppState) -> LoadingPump
```

Render entry point:

```rust
pub(crate) fn render_loading_screen(...) -> LoadingRenderResult
```

`LoadingRenderResult` should distinguish:

- native rendered;
- generic fallback rendered;
- native required but failed.

This avoids using `bool` where native failure and generic fallback are different parity cases.

Hydration remains:

```rust
app_transitions::apply_map_load_result(state, result)
```

No other production path should mutate full map-loaded `AppState`.

### Data Flow

1. Main-menu start, native shell start, quickplay, and dev starts all construct `LoadingRequest`.
2. `begin_loading` creates `LoadingSession`, initializes `LoadingJob`, sets `GameScreen::Loading`, and resets loading visuals.
3. If presentation is `NativeSelectedSkirmish`, the first native atlas is built before first visible loading frame can be accepted.
4. The app render branch calls `render_loading_screen`.
5. After present, the app calls `pump_loading_after_present`.
6. Each pump phase may advance native progress. Advancing progress requests another redraw; duplicate/lower progress does not.
7. On finish, `apply_map_load_result` hydrates `AppState`, clears `LoadingSession`, and enters `InGame` or `SpawnPick`.
8. On native presentation failure, enter controlled loading failure state. Do not render egui loading text as a native fallback.

### Migration Steps

First implementation slice:

1. Add `LoadingRequest`, `LoadingLaunch`, and `LoadingPresentation`.
2. Replace `begin_legacy_loading` and `begin_skirmish_loading` with one `begin_loading`.
3. Migrate `start_selected_skirmish` to create a `GenericMapLoad` request.
4. Migrate native shell start to create a `NativeSelectedSkirmish` request.
5. Migrate `RA2_QUICKPLAY` initialization so it creates a `LoadingSession`; do not set `GameScreen::Loading` directly.
6. Remove production callers of `transition_to_in_game`.
7. Remove production use of `app_init::load_map`; if retained, rename it to make run-to-completion/test/tooling scope explicit and ensure it uses the same job phases.
8. Remove `pending_skirmish_launch_session` from loading startup and from `transition_to_in_game` fallback logic.
9. Add static/searchable tests or assertions that enforce the migration rules below.

Later phase-split slice:

10. Convert `GameScreen::Loading { map_name }` to `GameScreen::Loading` once no render code needs the display fallback.
11. Continue splitting `load_map_from_initial` into verified phases with milestone ownership.

### Error Handling

- `NativeSelectedSkirmish` required native assets and renderer readiness. Failure enters a controlled failure state.
- `GenericMapLoad` may use generic loading fallback presentation, but still uses the same job.
- Map-load errors should preserve `anyhow` context and surface through `LoadingPump::Failed`.
- Do not silently hydrate fallback empty maps for native selected Skirmish. That hides parity and asset failures.

### Testing Strategy

Focused unit tests:

- `quickplay_initializes_loading_session`
- `all_loading_entrypoints_create_loading_session`
- `native_skirmish_loading_request_is_authoritative`
- `game_screen_loading_payload_is_not_used_for_map_selection`
- `native_skirmish_loading_does_not_use_pending_launch_session`
- `native_skirmish_render_failure_does_not_draw_egui_fallback`
- `generic_map_load_uses_same_loading_job`
- `transition_to_in_game_has_no_production_callers`
- `game_screen_loading_has_no_direct_production_constructors`
- `pending_skirmish_launch_session_is_not_used_for_loading`
- `app_init_load_map_is_not_a_production_app_api`

Focused integration-style tests where feasible:

- pump selected-map native Skirmish through first phase and assert native atlas uses job-owned assets;
- pump quickplay through initial phase and assert it does not stall without a session;
- replay milestone ledger and assert suppressed values do not request presents.

Manual check:

- launch native shell, start standard Skirmish, confirm native LS art first frame;
- launch `RA2_QUICKPLAY`, confirm it enters loading pipeline and reaches game/failure state, not stuck loading.

## Architectural Decisions

- `LoadingSession` is authoritative for loading. `GameScreen` is display state only.
- Native and generic loading share one job pipeline. Presentation differs; loading ownership does not.
- `apply_map_load_result` remains centralized to avoid hydration drift.
- Synchronous load wrappers are not production architecture. If retained temporarily, they are test-only or clearly named run-to-completion adapters over the same job.
- Do not split every `load_map_from_initial` phase in the same patch as caller migration. First eliminate multiple production entry paths, then continue phase extraction.

## Acceptance Criteria

- Every production map-load start calls `app_loading::begin_loading`.
- No production code constructs `GameScreen::Loading` directly outside `app_loading`.
- `RA2_QUICKPLAY` creates a `LoadingSession` before the first loading render/pump.
- Native selected Skirmish loading reads map token and launch session only from `LoadingSession`.
- `pending_skirmish_launch_session` is not used as loading state.
- `app_transitions::transition_to_in_game` has no production caller.
- `app_init::load_map` is not used as a production app loading API; any retained sync helper is explicit run-to-completion/test/tooling code over the same job.
- `GenericMapLoad` and `NativeSelectedSkirmish` share the same `LoadingJob` and final `apply_map_load_result` hydration path.
- Native selected Skirmish cannot fall back to egui loading text.
- Focused loading tests and `cargo check` pass.

## Alternatives Considered

### Keep Legacy Sync Wrapper In Production

Rejected. It leaves two loading pipelines and allows future changes to fix one path while the other silently drifts.

### Big-Bang Full Phase Split Now

Rejected for immediate next step. It is the final parity direction, but doing caller migration, state cleanup, and full `load_map_from_initial` decomposition together creates too much blast radius.

### Native-Only Pipeline

Rejected. Quickplay/dev/generic loads still need a supported app path, but they should use the same `LoadingJob` with a generic presentation mode.
