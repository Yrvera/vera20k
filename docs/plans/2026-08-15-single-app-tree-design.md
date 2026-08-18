# Single App Tree Design

## Goal

Consolidate every app-owned Rust module under one literal `src/app/` directory, with a shallow ownership-oriented hierarchy and no root-level `app_*.rs` files or sibling `src/app_*` directories.

## Architecture Context

The app layer currently has three simultaneous physical layouts:

- `src/app.rs` is the public `app` module root.
- `src/app/` contains the focused facade modules extracted from the former orchestrator.
- Roughly fifty `src/app_*.rs` files and four sibling directories (`src/app_render/`, `src/app_instances/`, `src/app_skirmish_shell_render/`, and `src/app_tactical_capture/`) remain declared from `src/lib.rs` as crate-root modules.

This makes one logical layer appear as several unrelated roots. It also leaves `src/lib.rs` responsible for app-internal module wiring and preserves historical `app_` prefixes that are redundant once code is nested below `app`.

The existing `src/app/` extraction establishes the pattern to continue: the `App` facade coordinates focused modules, `AppState` remains the composition root, and deterministic simulation remains below the app boundary. A current-code scan finds no `crate::app` dependency under `src/sim/`; that invariant must remain true throughout the relocation.

Not every skirmish-named root module is app-owned. `src/sim/scenario_bootstrap.rs` and `src/sim/scenario_post_map.rs` import shared `skirmish_launch` contracts. Those neutral contracts stay outside `app`; moving them would introduce a forbidden sim-to-app dependency.

This is a structural refactor only. It does not introduce new gamemd.exe behavior, INI interpretation, timing, RNG, state, rendering, serialization, or simulation rules. Current Rust module bodies and tests are the behavioral source of truth for the move.

## Impact Analysis

### End-state module surface

The app layer converges on:

```text
src/
  app/
    mod.rs
    frame.rs
    handler.rs
    initialize.rs
    types.rs
    state/
    platform/
    frontend/
    match_runtime/
    input/
    presentation/
```

There will be no `src/app.rs`, root-level `src/app_*.rs`, or sibling `src/app_*` directories at completion. Broad domains may contain submodules, but microfolders for one trivial file are avoided.

### Public paths

`src/lib.rs` retains `pub mod app;` and eventually stops declaring app-internal crate-root modules. Existing item visibility is preserved at new nested paths during each move. The old crate-root path is removed atomically; compatibility aliases are not retained. Public binary consumers such as `src/main.rs` are updated in the same slice that moves their modules.

### Tests and companion files

Embedded `#[cfg(test)]` modules move with their owners. Special companions move with the owning domain and their relative declarations are updated in the same commit:

- `src/app_init_helpers_retail_placement_oracle_tests.rs`, currently attached with `#[path]`;
- `src/app_render_tests.rs`, currently attached with `include!`;
- child directories belonging to file modules such as `src/app_skirmish_shell_render/`.

### Risk areas

- A missed `crate::app_*` or `vera20k::app_*` path can break library or binary compilation.
- Declaring old and new module roots together could compile duplicate types or tests.
- Moving `src/app.rs` to `src/app/mod.rs` changes the base directory of its current `#[path = "app_startup_splash.rs"]` attribute; the splash moves to its final frontend location in the same foundation slice.
- `app_render`, `app_instances`, and lower `render` helpers currently have cross-module type coupling. Relocation updates paths but does not redesign those interfaces.
- Bulk formatting can create unrelated churn. In particular, project policy forbids running rustfmt on `mod.rs`; module-root formatting is maintained manually.

No task in this design touches `src/sim/`, simulation ordering, deterministic hashes, snapshots, replay schemas, or INI/assets.

## Chosen Approach

Use one complete subtree per short-lived branch and PR. Establish `src/app/mod.rs` first, then migrate leaf domains before high-fanout runtime modules. Each relocation is atomic for its ownership cone: move files and child modules, update declarations/imports/tests/docs, remove old paths, validate, and commit before starting another domain.

The first branch is `feature/app-root-layout`. It performs only the foundation:

1. Move `src/app.rs` to `src/app/mod.rs` without changing the public `app` path.
2. Create the broad `frontend` domain.
3. Move the private process-start splash from `src/app_startup_splash.rs` to `src/app/frontend/startup_splash.rs` so the module-root move does not require a temporary `../` path.
4. Update the splash imports in the existing app facade modules.

Later branches migrate domains in dependency-aware order:

1. frontend shell renderers and capture subtrees;
2. small options, debug, and presentation services;
3. rendering, instances, sidebar, and overlays;
4. input, cursor, picking, context orders, camera, and commands;
5. loading, initialization, match runtime, transitions, and high-fanout shared app types;
6. final crate-root declaration/path scan.

Each later branch rechecks its live dependency cone before moving files. The sequence is directional, not permission to relocate all remaining files in one PR.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — No function body, call order, event order, RNG draw, timer calculation, render composition, command construction, or persistence behavior changes during a relocation. The ordinary stock-skirmish experience must be byte-for-byte driven by the same code after path resolution. `[source: current Rust module bodies and embedded tests]`
- `MILESTONE-BLOCKING` — `src/sim/` remains independent of `app`, `render`, `ui`, `sidebar`, `audio`, and `net`. Shared skirmish launch contracts therefore remain outside `app`. `[source: ENGINE.md layering contract; current Rust import scan]`
- `COMPOUNDING` — Old and new module declarations cannot coexist after a commit. Duplicate roots would permit type identity splits, duplicate tests, and hidden stale consumers. `[source: src/lib.rs and Rust module ownership]`
- `COMPOUNDING` — Existing item visibility is preserved during path moves. Privacy tightening or facade re-export redesign would make failures harder to attribute and is deferred to a separate API review. `[source: src/lib.rs public module declarations]`
- `COMPOUNDING` — Embedded tests, `#[path]` companions, `include!` companions, and child module directories move with their owning module. Leaving tests behind would falsely certify a different path than production uses. `[source: src/app_init_helpers.rs; src/app_render/mod.rs; src/app_skirmish_shell_render.rs]`
- `COMPOUNDING` — Module-root files are formatted manually; no recursive rustfmt is run on `mod.rs`. This keeps relocation diffs reviewable and avoids unrelated formatting churn. `[source: AGENTS.md/ENGINE.md project workflow]`
- `EXACTIFICATION-RESIDUAL` — Existing lower-layer imports of app-owned presentation types are not repaired by this relocation. Trigger: later interface cleanup around cursor/render DTOs. Player effect: none today. Frequency: compile-time architecture only. Downstream risk: continued coupling, recorded for a separate ownership slice.
- `EXACTIFICATION-RESIDUAL` — `app_frame_pacer.rs` currently combines platform frame admission with match elapsed-time accounting. It is not moved wholesale into a misleading domain; its eventual split receives a dedicated design. Trigger: file-layout cleanup only. Player effect: none while deferred. Downstream risk: low because ownership is already documented and state remains separate.

## Design

### Components

#### `src/app/mod.rs`

The sole app module root and public facade. It owns `App`, top-level orchestration imports, broad domain declarations, and the small number of facade re-exports needed by sibling app modules.

#### Root orchestrator modules

`frame.rs`, `handler.rs`, and `initialize.rs` remain directly below `app` because they coordinate multiple domains. They are not forced into an artificial owner.

#### Broad domains

- `platform/`: process/window clocks, startup options, and process-local settings.
- `frontend/`: shell screens, loading presentation, random-map shell flow, score presentation, and capture tooling.
- `match_runtime/`: running-match app orchestration, transitions, scenario exit, and frame handoff.
- `input/`: raw input, hotkeys, cursor, picking, context orders, gadgets, camera control, and command submission.
- `presentation/`: render planning/submission, instances, sidebar, overlays, effects, messages, tooltips, and debug presentation.
- `state/`: focused app-owned state groups; it remains the composition root rather than duplicating state within domain modules.

The exact owner of an ambiguous file is resolved from its callers and state writes before that file moves. A domain name is not sufficient evidence by itself.

### Interfaces / Contracts

- `vera20k::app` remains the public top-level module.
- Each moved public module keeps equivalent visibility under `vera20k::app::<domain>::...` until a separate API review.
- No forwarding modules remain at old `vera20k::app_*` paths.
- Shared non-app contracts stay at their current neutral paths unless separately designed.

### Data Flow

Relocation does not alter data flow. Winit events still enter the app handler, frontend or running-match orchestration still updates `AppState`, input still issues commands, simulation remains authoritative, and presentation still consumes sim/app views. Only compile-time module resolution changes.

### Error Handling

There is no runtime fallback or dual-path migration. A slice either compiles entirely at its new path or is not committed. Missing path updates are compile/test failures. Filesystem and asset error behavior remains in the unchanged module bodies.

### Testing Strategy

For every slice:

1. check that no Cargo/rustc process is owned by another task;
2. run `git diff --check`;
3. scan for the removed old module and filesystem paths;
4. run focused `cargo test -p vera20k --lib <new-module-filter>` tests;
5. compile all library modules through the focused `--lib` invocation;
6. run the full `cargo test -p vera20k --lib` suite exactly once before that branch's PR is declared ready.

Tests are not rewritten merely to keep old paths alive. Expected test module names change to their new hierarchy; assertions and fixtures remain unchanged.

## Architectural Decisions

- Use `src/app/mod.rs` rather than the idiomatic-but-physically-split `src/app.rs` plus `src/app/`, because the explicit goal is one literal app directory.
- Use a shallow set of broad ownership domains. A completely flat directory would recreate the original problem, while a directory per tiny concept would create a new form of fragmentation.
- Preserve behavior and visibility during movement. API tightening, shared-type extraction, AppState regrouping, and logic changes are separate changes.
- Use one subtree per branch/PR. Incremental commits inside one giant relocation branch would still impose a giant review and merge-conflict surface.
- Keep shared skirmish contracts outside app because simulation consumes them.

## Alternatives Considered

### Keep `src/app.rs` beside `src/app/`

This is conventional Rust module layout and requires fewer moves, but it does not meet the requested literal one-directory outcome.

### Flatten every app file directly into `src/app/`

This removes sibling roots but produces a directory with roughly fifty files and preserves redundant names. Navigation and ownership remain unclear.

### Create a deep directory for every small subsystem

This makes local ownership explicit but replaces top-level clutter with excessive nesting and long paths. The chosen design limits folders to broad domains.

### Relocate all app modules in one branch

Even with incremental commits, the PR would touch most of the app layer and make path omissions, merge conflicts, and behavioral review unnecessarily difficult.
