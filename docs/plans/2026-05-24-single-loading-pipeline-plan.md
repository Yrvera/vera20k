# Single Loading Pipeline Implementation Plan

> Execute only after user approval. This is an implementation plan, not the implementation.

**Goal:** migrate every production map-load start onto one `LoadingSession` /
`LoadingJob` pipeline so standard offline Skirmish, quickplay, and generic dev loads
do not drift through separate synchronous loading paths.

**Design Doc:** `docs/plans/2026-05-24-single-loading-pipeline-design.md`

## Grounding Summary

- Current Rust has a pumpable loading path in `src/app_loading.rs`, but production
  starts still split across `begin_legacy_loading`, `begin_skirmish_loading`,
  `RA2_QUICKPLAY` direct `GameScreen::Loading`, and the old synchronous
  `app_transitions::transition_to_in_game` wrapper.
- `src/app_init.rs` already has the first load boundary split into
  `load_map_initial_with_assets(...)` and `load_map_from_initial(...)`.
- `LoadingJob` now owns the `AssetManager` used for native loading atlas setup and
  later `MapLoadResult.asset_manager`.
- Native selected Skirmish failure must not fall back to egui loading text.
- `RA2_QUICKPLAY` currently sets `GameScreen::Loading` without creating a
  `LoadingSession`, which can leave the app on a loading screen with no job to
  pump.
- `pending_skirmish_launch_session` and `GameScreen::Loading { map_name }` are
  still overlapping loading truth sources and should not survive as production
  loading state.

## Scope

This plan covers the caller migration and ownership cleanup only.

In scope:

- one production loading start API;
- request/session typing for native and generic presentations;
- migration of main-menu Skirmish start, native shell Skirmish start, and
  `RA2_QUICKPLAY`;
- removal or quarantine of the synchronous production wrapper;
- removal of `pending_skirmish_launch_session` from loading ownership;
- verification that no production code constructs loading screens directly.

Out of scope:

- full decomposition of `load_map_from_initial(...)` into every verified native
  progress phase;
- `mmpb.shp` player marker and post-marker loading text;
- campaign loading UI;
- random-map or multiplayer wait/resend loading parity;
- any new `sim/` dependency on render, UI, sidebar, audio, or net.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/app_loading.rs` | Define `LoadingRequest`, launch/presentation modes, unified `begin_loading`, typed loading render result, and request/session tests. |
| Modify | `src/app.rs` | Route every app load start through `begin_loading`, migrate `RA2_QUICKPLAY`, and handle native failure without egui fallback. |
| Modify | `src/app_init.rs` | Remove or rename production `load_map` wrapper after callers are gone; keep phase helpers. |
| Modify | `src/app_transitions.rs` | Keep `apply_map_load_result`; delete or test-gate `transition_to_in_game`. |
| Modify | `src/ui/game_screen.rs` | Convert `GameScreen::Loading { map_name }` to screen-only `GameScreen::Loading`, or complete the equivalent no-payload migration. |
| Modify as needed | tests near touched modules | Add focused unit tests for request/session construction and regression checks. |

## Design Constraints

- `LoadingSession` is the authoritative loading state.
- `GameScreen::Loading` is only the active screen marker.
- Native and generic loading presentations share the same `LoadingJob` and the same
  final `apply_map_load_result` hydration path.
- `GenericMapLoad` is a presentation mode, not a second loader.
- `NativeSelectedSkirmish` cannot draw egui loading text when native rendering fails.
- Synchronous run-to-completion loading must not remain a production app API.
- `pending_skirmish_launch_session` must not be used as loading state.

## Tasks

### Task 1: Add First-Class Loading Request Types

**Why:** One start API needs to carry both the selected map token and the kind of
presentation/launch data without borrowing from `GameScreen` or app scratch fields.

**Files:**

- `src/app_loading.rs`

**Steps:**

1. Add `LoadingRequest` with:
   - `selected_map_file: String`;
   - `launch: LoadingLaunch`;
   - `presentation: LoadingPresentation`;
   - `fallback_skirmish_settings: SkirmishSettings`.
2. Add `LoadingLaunch`:
   - `Skirmish(SkirmishLaunchSession)`;
   - `Generic`.
3. Add `LoadingPresentation`:
   - `NativeSelectedSkirmish`;
   - `GenericMapLoad`.
4. Add constructors that make invalid pairings hard to express:
   - native selected Skirmish requires `LoadingLaunch::Skirmish`;
   - generic map load uses `LoadingLaunch::Generic` unless a future caller proves a
     need for something else.
5. Keep `LoadingRequest` fields private outside `app_loading`; callers must use
   smart constructors such as `LoadingRequest::native_selected_skirmish(...)` and
   `LoadingRequest::generic_map_load(...)`.
6. Move `LoadingSession` construction behind `LoadingSession::from_request(...)`.
7. Preserve side derivation from the Skirmish launch session for native selected
   Skirmish.

**Tests:**

- native selected Skirmish request preserves selected map filename and launch
  session;
- generic map load creates no native loading state;
- native selected Skirmish request derives loading side from the launch session;
- invalid native/generic request pairings are rejected or impossible through public
  constructors;
- no module outside `app_loading` can manually assemble a `LoadingRequest` with
  mismatched launch and presentation modes.

### Task 2: Replace Split Begin Helpers With One `begin_loading`

**Why:** The current `begin_legacy_loading` and `begin_skirmish_loading` helpers
encode two ownership models. The app should have one production entry point.

**Files:**

- `src/app_loading.rs`
- `src/app.rs`

**Steps:**

1. Add `pub(crate) fn begin_loading(state: &mut AppState, request: LoadingRequest)`.
2. In `begin_loading`, reset prior loading state, create the `LoadingSession`, set
   `GameScreen::Loading`, and clear any stale loading visuals.
3. Remove `begin_legacy_loading` and `begin_skirmish_loading`, or leave them only
   during the patch while immediately migrating all callers.
4. Ensure `begin_loading` does not write
   `state.pending_skirmish_launch_session`.
5. Ensure `begin_loading` is the only production function that constructs
   `GameScreen::Loading`.

**Tests:**

- `begin_loading` creates a `LoadingSession`;
- `begin_loading` sets screen state to loading;
- `begin_loading` does not populate pending launch fallback state;
- native and generic requests both create a pumpable `LoadingJob`.

### Task 3: Migrate All Production Callers

**Why:** This is the core of the migration. Leaving even one direct loading start
behind keeps two pipelines alive.

**Files:**

- `src/app.rs`

**Steps:**

1. Update `start_selected_skirmish` to construct a `GenericMapLoad` request and
   call `app_loading::begin_loading`.
2. Update native shell Skirmish start to construct a `NativeSelectedSkirmish`
   request with `LoadingLaunch::Skirmish(session)` and call
   `app_loading::begin_loading`.
3. Migrate `RA2_QUICKPLAY` initialization so it creates a `LoadingSession` before
   the first loading render/pump:
   - build `AppState` with a neutral initial screen and `loading_session: None`;
   - clone the needed fallback settings;
   - construct `LoadingRequest::generic_map_load("auto", cloned_settings)`;
   - call `app_loading::begin_loading(&mut state, request)` before returning the
     initialized state.
4. Keep the existing quickplay window/game-mode setup intact.
5. Confirm no app caller sets `GameScreen::Loading` directly.

**Tests:**

- quickplay initialization creates a loading session;
- quickplay does not stall in loading with no job;
- selected Skirmish start and native shell Skirmish start both enter the same
  loading session path.

### Task 4: Make Loading Presentation Result Typed

**Why:** The render path currently uses a bool-like result where native failure and
generic fallback are easy to conflate. They are different parity cases.

**Files:**

- `src/app_loading.rs`
- `src/app.rs`

**Steps:**

1. Replace the current loading render boolean with a typed result, for example:
   - `NativeRendered`;
   - `GenericFallbackRendered`;
   - `NoPresentation`;
   - `NativeFailed(anyhow::Error)` or equivalent error context.
2. In the `GameScreen::Loading` app branch, allow generic presentation fallback only
   for `GenericMapLoad`.
3. For `NativeSelectedSkirmish`, transition to a controlled loading failure state
   if native rendering cannot be prepared.
4. Do not draw `ui::main_menu::draw_loading_screen` for native selected Skirmish.

**Tests:**

- native selected Skirmish render failure does not call the egui fallback path;
- generic map loading can still use a generic fallback presentation;
- native and generic presentation results share the same pump/final hydration path.

### Task 5: Remove Loading Payload From `GameScreen`

**Why:** `GameScreen::Loading { map_name }` looks like a source of truth and has
already caused map/request state to be duplicated.

**Files:**

- `src/ui/game_screen.rs`
- `src/app.rs`
- `src/app_loading.rs`
- `src/app_transitions.rs`

**Steps:**

1. Change `GameScreen::Loading { map_name }` to `GameScreen::Loading`.
2. Update all matches and constructors.
3. If generic fallback UI needs a label, read it from `LoadingSession.request` via a
   small `app_loading` helper instead of from `GameScreen`.
4. Ensure `GameScreen` no longer carries map selection data.

**Tests:**

- loading render gets display/request data from `LoadingSession`;
- no map selection logic can read a loading payload from `GameScreen`.

### Task 6: Delete Or Quarantine Legacy Synchronous Transition

**Why:** `transition_to_in_game` preserves the old `load_map(...)` production shape.
As long as it remains callable, future changes can accidentally revive the sync
loader.

**Files:**

- `src/app_transitions.rs`
- `src/app_init.rs`

**Steps:**

1. Confirm all production callers now use `pump_loading_after_present`.
2. Delete `app_transitions::transition_to_in_game`.
3. If a run-to-completion helper is still needed, make it test/tooling scoped and
   name it explicitly, for example `load_map_run_to_completion_for_tests`; do not
   keep a crate-callable `pub(crate) fn transition_to_in_game`.
4. Remove production use of `app_init::load_map`.
5. Keep `app_transitions::apply_map_load_result` as the single final hydration
   boundary.
6. Keep `load_map_initial(...)`, `load_map_initial_with_assets(...)`, and
   `load_map_from_initial(...)` phase helpers.

**Tests:**

- no `pub(crate) fn transition_to_in_game` remains;
- no production caller references any run-to-completion helper;
- no production app path calls `app_init::load_map`;
- pump completion still hydrates through `apply_map_load_result`.

### Task 7: Remove `pending_skirmish_launch_session` As Loading State

**Why:** Loading launch ownership belongs inside `LoadingRequest`. Keeping a second
app-level optional launch session invites mismatches between what is rendered and
what is loaded.

**Files:**

- `src/app.rs`
- `src/app_loading.rs`
- `src/app_transitions.rs`

**Steps:**

1. Remove writes to `state.pending_skirmish_launch_session` during loading startup.
2. Remove reads of `pending_skirmish_launch_session` from legacy transition fallback
   logic.
3. If no non-loading owner remains, delete the `AppState` field.
4. If a non-loading flow still needs deferred launch data, rename the field to that
   specific purpose and keep it out of loading startup.

**Tests:**

- native selected Skirmish loads from `LoadingSession.request.launch`;
- no loading path reads `pending_skirmish_launch_session`;
- deleting or renaming the field does not change final `SkirmishLaunchSession`
  application.

### Task 8: Tighten Verification And Search Checks

**Why:** This migration is as much about removing old entry points as adding new
types. Source searches should be part of the acceptance checks.

**Files:**

- touched tests near `app_loading`, `app`, and `app_init`

**Steps:**

1. Add focused unit tests for request/session behavior.
2. Add quickplay initialization coverage if an app-init test harness can construct
   state without requiring a full retail asset run.
3. Run static searches after implementation:
   - `rg -n "begin_legacy_loading|begin_skirmish_loading" src`
   - `rg -n "GameScreen::Loading" src`
   - `rg -n "transition_to_in_game|pending_skirmish_launch_session|pub fn load_map\\(" src`
4. For `GameScreen::Loading`, the only acceptable production matches are the enum
   definition, match arms, and the single constructor inside `app_loading`; no other
   module may start loading by assigning the screen directly.
5. Any remaining result must be test-only, docs-only, or explicitly justified in
   code.

## Suggested Execution Order

1. Add request/session enums and constructors.
2. Add unified `begin_loading`.
3. Migrate normal, native shell, and `RA2_QUICKPLAY` callers.
4. Type the loading render result and enforce native failure behavior.
5. Convert `GameScreen::Loading` to payload-free screen state.
6. Remove or quarantine `transition_to_in_game` and `app_init::load_map`.
7. Delete `pending_skirmish_launch_session` if no non-loading use remains.
8. Run focused tests, `cargo check`, and source-search acceptance checks.

## Acceptance Criteria

- Every production map-load start calls `app_loading::begin_loading`.
- No production code constructs `GameScreen::Loading` directly outside
  `app_loading`.
- `GameScreen::Loading` no longer carries a map token.
- `RA2_QUICKPLAY` creates a `LoadingSession` before the first loading render/pump.
- Native selected Skirmish reads selected map and launch session only from
  `LoadingSession`.
- `pending_skirmish_launch_session` is not used as loading state.
- `transition_to_in_game` no longer exists as a crate-callable production helper.
- `app_init::load_map` is not a production loading API.
- `GenericMapLoad` and `NativeSelectedSkirmish` share the same `LoadingJob` and
  final `apply_map_load_result` hydration path.
- Native selected Skirmish cannot fall back to egui loading text.
- The implementation does not claim full native progress cadence beyond the phases
  currently split.
- Focused loading tests and `cargo check` pass.

## Verification Commands

Run these after implementation:

```powershell
cargo fmt
cargo test loading_session --lib
cargo test loading_side --lib
cargo test loading_progress --lib
cargo test loading_art --lib
cargo test app_init --lib
cargo check -q
rg -n "begin_legacy_loading|begin_skirmish_loading" src
rg -n "GameScreen::Loading" src
rg -n "transition_to_in_game|pending_skirmish_launch_session|pub fn load_map\(" src
```

Expected search outcome:

- no `begin_legacy_loading` or `begin_skirmish_loading`;
- no `GameScreen::Loading { ... }`;
- no direct `GameScreen::Loading` constructor outside `app_loading`;
- no crate-callable `transition_to_in_game`;
- no production `pending_skirmish_launch_session`;
- no production `pub fn load_map(...)` app loading wrapper.

## Do Not Do In This Plan

- Do not split all remaining `load_map_from_initial(...)` phases in this patch.
- Do not implement or guess `mmpb.shp` marker behavior.
- Do not add campaign loading UI.
- Do not let native selected Skirmish use egui fallback text.
- Do not keep a second production synchronous loader.
- Do not move loading orchestration into `sim/`.
- Do not hardcode new gameplay or visual constants where verified assets/INI data
  should drive them.
