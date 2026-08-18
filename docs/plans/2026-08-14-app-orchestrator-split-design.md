# App Orchestrator Split Design

## Goal

Split `src/app.rs` into private, responsibility-oriented child modules without changing runtime behavior, public paths, state ownership, or frame/event ordering.

## Architecture Context

`crate::app` is the application boundary above simulation, rendering, UI, audio, assets, and networking. `App` implements winit's `ApplicationHandler`; `AppState` owns the initialized process/runtime state consumed by the flat `app_*` modules.

The current `src/app.rs` is 6,321 lines. It contains a 213-field `AppState`, process and GPU initialization, main-menu/single-player/skirmish shell control, random-map generation, window-event routing, frame orchestration, capture/readback handling, loading-after-present, in-game modal control, and scenario teardown. It references 37 other app modules and was touched by 27 of the last 100 commits.

Existing app-layer decomposition uses stable facades with private child modules, notably `app_render`, `app_instances`, `app_skirmish_shell_render`, and `app_tactical_capture`. The new layout follows that pattern while retaining `src/app.rs` as the public facade, so consumers continue to use `crate::app::{App, AppState}`.

## Impact Analysis

Tracked implementation changes are confined to `src/app.rs` and new files below `src/app/`. `src/lib.rs` does not need a public-module change. Existing sibling modules that import `crate::app::AppState` retain that path through a facade re-export.

The principal risks are accidental changes to Rust visibility, associated-method lookup, initialization order, event consumption, render/present ordering, and tests whose module path or access to private helpers changes. There are no data-format migrations, simulation state changes, tick-order changes, RNG changes, INI changes, asset changes, or intended player-visible changes.

The flat `AppState` remains deliberately unchanged in this slice. Grouping its fields into subsystem-owned structs would touch most app modules and is a separate ownership refactor.

## Chosen Approach

Keep `src/app.rs` as a thin facade and move existing contiguous responsibilities into private direct child modules under `src/app/`. Multiple inherent `impl App` blocks remain valid Rust; cross-child helpers receive only the narrow `pub(super)` visibility needed within `crate::app`. `App`, `AppState`, and all externally used method paths remain stable.

The move is mechanical: function bodies, constants, comments, branch ordering, state reads/writes, and error propagation remain unchanged. No helper is redesigned merely to make the move easier.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING`: winit events must still reach egui before app routing, while capture, startup-splash, shell-transition, modal, paused-input, and screen-specific gates keep their current priority. A change can leak clicks/orders through overlays on every affected input event. [Rust: `src/app.rs`, `ApplicationHandler::window_event`]
- `MILESTONE-BLOCKING`: foreground loss must clear mouse capture and freeze ordinary simulation; foreground return must not silently re-anchor the fixed-step pacer. A change is visible on every Alt+Tab during a match and can change outcomes. [Rust: `src/app.rs`, `set_window_active`, `render_frame`]
- `MILESTONE-BLOCKING`: ordinary simulation advances before in-game rendering, while scenario outcome/abort gates run before and immediately after the admitted advance. Reordering can add or remove a simulation frame. [Rust: `src/app.rs`, `render_frame`]
- `MILESTONE-BLOCKING`: loading work starts only after the loading frame is submitted and presented. Moving the pump earlier makes the loading screen fail to appear or changes progress cadence on every load. [Rust: `src/app.rs`, tail of `render_frame`]
- `MILESTONE-BLOCKING`: command submission, surface presentation, transition receipt commits, capture readback, and screenshot completion retain their current sequence. Reordering can capture the wrong frame or advance shell transitions before a frame becomes visible. [Rust: `src/app.rs`, tail of `render_frame`]
- `COMPOUNDING`: `AppState` initialization order and default values remain exact. A missing or differently seeded field can affect the entire session. [Rust: `src/app.rs`, `initialize`]
- `COMPOUNDING`: shell teardown, returned RNG handoff, replay flushing, and scenario-exit cleanup stay on their current paths. A missed reset can contaminate the next match. [Rust: `src/app.rs`, shell and in-game lifecycle helpers]
- `EXACTIFICATION-RESIDUAL`: the 213-field flat `AppState` remains a broad shared owner. It is a maintenance/coupling issue, but retaining it avoids architecture and behavior risk in this structural slice.
- `EXACTIFICATION-RESIDUAL`: other oversized app files (`app_input.rs`, `app_loading.rs`, `app_sim_tick.rs`, `app_init.rs`, and `app_skirmish.rs`) remain for separately reviewable follow-up slices.
- `UNKNOWN-RISK`: none requiring gamemd.exe investigation. This change does not reinterpret native behavior; current verified/provenance-bearing Rust logic is moved intact.

## Design

### Components

- `src/app.rs`: facade, private child declarations, stable re-exports, and the small public `App` surface.
- `src/app/state.rs`: `AppState`, state-only helper types/accessors, and scenario-reset glue.
- `src/app/shell_main_menu.rs`: main-menu and single-player shell predicates, actions, modal routing, audio, and quit cascade entry.
- `src/app/shell_skirmish.rs`: skirmish shell lifecycle, validation, chooser routing, status-help routing, and launch handoff.
- `src/app/shell_random_map.rs`: random-map generation/retention, preview rasterization, saved-seed routing, and random-map setup input.
- `src/app/handler.rs`: `ApplicationHandler` implementation and window event dispatch.
- `src/app/initialize.rs`: process/window/GPU/frontend initialization and complete `AppState` construction.
- `src/app/frame.rs`: the top-level frame owner, screen dispatch, submit/present, capture/readback, and loading-after-present.
- `src/app/in_game.rs`: focus/visibility transitions, in-game menu/modal routing, scenario return, save/load/dev-overlay adapters, and UI-scale helper where appropriate.

Exact file boundaries may shift slightly during extraction when a constant or helper has only one natural owner, but responsibilities and facade contracts do not change.

### Interfaces / Contracts

- `crate::app::App` remains public with the same constructors and capture completion method.
- `crate::app::AppState` remains `pub(crate)` at the same path.
- Existing `app_*` sibling modules retain their imports and call sites.
- Child-only methods use `pub(super)`; no new crate-public API is introduced.
- No `AppState` field is renamed, reordered semantically, regrouped, or made independently owned.

### Data Flow

Winit still calls the single `ApplicationHandler` implementation. The handler initializes `AppState`, routes events through the unchanged priority ladder, and calls the unchanged frame owner. The frame owner updates wall-clock services, admits simulation when legal, acquires the surface, dispatches by `GameScreen`, submits and presents, commits transition/capture receipts, then pumps loading after presentation.

### Error Handling

Existing `anyhow::Result` propagation, logging, capture failure recording, loading fallback, and event-loop exits remain unchanged. Module extraction must not add catches, fallbacks, or suppressed errors.

### Testing Strategy

1. Check for active Cargo/rustc processes before every Cargo command.
2. After each coherent extraction, run `cargo check -p vera20k` and the narrowest applicable `cargo test -p vera20k --lib app::` filter.
3. Keep each extraction buildable and commit it incrementally.
4. Run `cargo test -p vera20k --lib` exactly once after the complete slice is stable.
5. Compare the resulting public/module call surface and inspect the final diff for non-move logic changes.

## Architectural Decisions

The design follows the existing facade/private-submodule pattern. It intentionally does not introduce a new application framework, dependency injection layer, event bus, or state-owner hierarchy.

The temporary debt is that child modules still share the flat `AppState` and some app-layer cycles remain. Those are explicit residuals rather than being hidden inside this mechanical split.

## Alternatives Considered

1. Split every oversized app file in one branch. Rejected because the diff and merge-conflict surface would be too broad, and independent subsystem boundaries deserve separate validation.
2. Decompose `AppState` into render, shell, match, input, and audio owner structs first. Rejected for this slice because it would touch most app modules and combine ownership/API changes with file movement.
3. Move code into additional flat `app_*` public modules. Rejected because it would further crowd the crate root and expose implementation organization instead of using the established private-child facade pattern.
