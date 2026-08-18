# App–Sim Authority Boundary and AppState Ownership Design

## Goal

Make simulation the sole owner of match outcomes, RNG, lifecycle timing, overlay/navigation authority, and authoritative entity mutations; make the app issue commands and consume immutable views plus typed outputs; then decompose `AppState` incrementally by ownership without a whole-struct rewrite.

## Architecture Context

`AppState` is the process-wide application root in `src/app/state.rs`. It currently holds roughly 213 flat fields spanning window/platform state, GPU and audio resources, frontend shells, loaded-map data, input state, presentation caches, and `Simulation` itself. Recent commits already extracted the major orchestration owners (`app/event_handler.rs`, `app/frame_pipeline.rs`, `app/frontend.rs`, and `app/in_game.rs`), but those owners still reach through one flat state object.

The intended project layering is explicit in `ENGINE.md:114-123`: `sim/` owns deterministic state and gameplay; render, UI, sidebar, audio, and networking sit above it; app orchestrates without absorbing gameplay logic. The current frame path only partially follows that contract:

1. `app_sim_tick::advance_one_simulation_frame` removes due commands and calls `Simulation::advance_master_frame` with app-owned rules, map heights, `PathGrid`, overlay registry, animation sequences, and trigger definitions (`src/app_sim_tick.rs:1042-1109`).
2. `Simulation::advance_master_frame` owns the deterministic phase spine and returns a coarse `TickResult` (`src/sim/world/mod.rs:253-274`, `4480-4491`, `5333-5378`).
3. After the state hash has been calculated, app code mutates simulation entities through living animation, voxel, and harvest-overlay ticks; drains public event vectors; mutates building animation overlays; and spawns particle systems (`src/app_sim_tick.rs:1110-1156`; `src/app_building_anim.rs:284-432`).
4. App code also drains overlay dirty cells, mutates `Simulation::resolved_terrain`, decides whether navigation changed, rebuilds the app-owned dynamic `PathGrid`, and then asks sim to rebuild zones (`src/app_sim_tick.rs:1682-1748`, `1900-1957`).
5. The renderer independently predicts wall autofill from a presentation overlay list, while sim placement commits only the clicked wall cell (`src/app_render/build_instances.rs:650-779`; `src/sim/production/production_placement.rs:219-275`).
6. Skirmish startup algorithms live in `app_skirmish.rs` and directly consume `Simulation::scenario_rng`; the score-screen builder also consumes that stream after match termination (`src/app_skirmish.rs:258-445`, `1208-1410`; `src/app_sim_tick.rs:410-550`).
7. App options directly write `sim.session.game_options.game_speed` (`src/app_options_persist.rs:68-74`).

Several correct patterns already exist and should be extended rather than replaced:

- Commands are deterministic envelopes and `PlaceReadyBuilding` already carries explicit owner, type, and cell.
- Sim owns `OverlayGrid`, wall ownership, entity lifecycle, the three RNG streams, state hashing, serialization, and the authoritative phase spine.
- Sim produces presentation facts such as `SimSoundEvent`, `SimFireEvent`, `LifecycleOutput`, and trigger effects, but app currently drains their backing vectors separately.
- Pending deletion is already sim-owned. The single ordinary drain runs after the late frame commit and is skipped on terminal calls (`src/sim/world/mod.rs:4411-4447`). This matches the verified native placement of `FUN_00725C70` in the `Main_Tick` tail and its terminal skip gates (`docs/research/PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md`; `docs/research/MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md`).

The exact ordinary wall rule is now closed for stock YR and the local-human path in `docs/research/REGULAR_OVERLAY_WALL_AUTOFILL_COMMIT_GHIDRA_REPORT.md`. One clicked command recomputes N/E/S/W gaps, uses `GuardRange >> 8`, requires same linked overlay and owner, validates every intermediate cell, creates fillers nearest-to-click, and consumes one ready product. Preview duplicates the read-only scan; commit owns the result. The native nonlocal pending-owner global remains a named residual, so Rust should retain its stronger explicit command-owner contract.

## Impact Analysis

### App–sim authority migration

- `src/sim/world/mod.rs`: frame entry/output boundary, post-spine authoritative mutation, event packaging, navigation ownership, privacy.
- `src/sim/production/production_placement.rs`: exact regular-wall preview/commit query and placement result production.
- `src/sim/overlay_grid.rs`: separate authoritative navigation mutation from presentation deltas; stop exposing an app-drained authority queue.
- `src/sim/animation.rs`, particle/miner modules: move authoritative entity/particle writes before the returned state hash.
- New focused sim modules for frame I/O, runtime resources/navigation, scenario bootstrap, and match finalization where extraction keeps files near the project’s ~600-line convention.
- `src/app_sim_tick.rs`: become a command submitter plus `SimFrameOutput` consumer; remove post-hash sim writes and inference from `spawned_entities` plus due commands.
- `src/app_skirmish.rs`: retain shell/session descriptor construction, but move start assignment, fallback placement, starting-force selection, and RNG cursor ownership into sim.
- `src/app_render/build_instances.rs`: render a sim-produced wall preview; remove the app algorithm.
- `src/app_options_persist.rs`: persist local preferences and enqueue/apply an explicit sim setting transition instead of writing session state.
- Snapshot/load and headless call sites: bind immutable runtime resources and deterministically rebuild navigation caches after state restoration.

### AppState decomposition follow-on

- `src/app/state.rs` remains the root aggregate.
- Focused state types live under `src/app/state/` and are introduced one at a time.
- Initial ownership candidates are:
  - `PlatformState`: window, active/hidden lifecycle, and local frame-admission clocks.
  - `FrontendState`: shell selection/loading/RMG process state and frontend RNG.
  - `MatchState`: the sim runtime handle plus app-local match identity, elapsed wall clock, and final score presentation input.
  - `InputState`: cursor/hotkeys, selection order, command mode, control groups, and targeting state.
  - `PresentationState`: render/audio/HUD caches, likely split further rather than becoming another giant struct.
- The grouping order is consumer-driven. A group moves only after its owner modules and complete read/write cone are known. Flat residual fields remain until their owner is equally clear.

### Main risks

- Tick-order drift when moving work across the hash boundary.
- Scenario RNG draw-order drift during bootstrap or finalization.
- Duplicate or dropped transient events during output consolidation.
- Stale navigation after command-tail placement/removal, snapshot load, or bridge/terrain changes.
- Borrow pressure if immutable resources and mutable sim state are not separated cleanly.
- Mechanical call-site churn during `AppState` grouping masking a behavior change.

No persistent snapshot schema needs to change merely to package transient outputs or move derived navigation caches. Any later serialized field change remains separately gated by the repository snapshot-version rules.

## Chosen Approach

Use incremental authority closure, then incremental state grouping.

The app–sim portion lands as independently testable slices. The final boundary is a sim-owned runtime that accepts commands/settings and returns a `SimFrameOutput`; presentation reads a `SimView`. Immutable rules/map/animation inputs are bound once as runtime resources rather than supplied as app decisions every frame. Derived navigation lives with the sim runtime and is rebuilt deterministically after load.

Only after those owners are visible does `AppState` gain nested groups. Each group migration is atomic for that group: define one owner type, move a bounded field set, update every reader/writer, validate, and commit. There will be no broad field sorting, generated forwarding accessors, `Deref` compatibility layer, or duplicate old/new field storage.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — Every mutation that can affect targeting, entity state, particles, RNG cadence, navigation, save state, or hash must complete inside the sim transaction before the returned hash. Current post-hash entity and particle writes violate this (`src/app_sim_tick.rs:1110-1156`; `src/app_building_anim.rs:380-432`).
- `MILESTONE-BLOCKING` — Scenario setup and score-finalization RNG draws stay on the scenario stream in their existing deterministic order. Moving code between modules must not redraw, precompute on a presentation path, or consume on only one peer (`src/app_skirmish.rs:258-445`, `927-951`, `1208-1410`; `src/app_sim_tick.rs:497-548`).
- `MILESTONE-BLOCKING` — Regular wall fill uses one clicked command, `GuardRange >> 8`, N/E/S/W order, same type and owner endpoints, endpoint-before-legality checks, no partial fill through a blocker, nearest-first commit, and one production consumption. Stock `GuardRange=5` permits endpoints at distances 1-5 and at most four fillers (`doc: REGULAR_OVERLAY_WALL_AUTOFILL_COMMIT_GHIDRA_REPORT.md §§4, 7, 13`).
- `MILESTONE-BLOCKING` — A command recomputes wall fill against execution-time state. Preview cells never enter the command (`doc: REGULAR_OVERLAY_WALL_AUTOFILL_COMMIT_GHIDRA_REPORT.md §§2, 6, 13`).
- `MILESTONE-BLOCKING` — Overlay identity, owner, connectivity, passability, and navigation consequences become visible before the frame hash and before the next sim reader. App may cache an `OverlayDelta`, but it cannot finish authority (`doc: REGULAR_OVERLAY_WALL_AUTOFILL_COMMIT_GHIDRA_REPORT.md §§5, 8, 13`).
- `MILESTONE-BLOCKING` — Pending deletion remains one drain after frame commit. Victory, defeat, quit, and disconnect terminal calls skip both commit and drain; ordinary pause does not invent another drain (`GHIDRA: Main_Tick 0x0055DE00, drain call 0x0055DE9F; docs: PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md and MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md`).
- `MILESTONE-BLOCKING` — Each transient sim event is delivered exactly once and in producer order. Packaging vectors into `SimFrameOutput` must not reorder sound, fire, lifecycle, trigger, placement, or overlay deltas (`src/sim/world/mod.rs:321-524`, `604-677`).
- `COMPOUNDING` — App currently owns the canonical `PathGrid` while sim owns terrain, overlays, costs, and zones. Leaving this split keeps every new terrain/structure mechanic dependent on an app repair pass (`src/app_sim_tick.rs:1682-1748`, `1900-1957`; `src/sim/world/mod.rs:686-702`, `3281-3384`).
- `COMPOUNDING` — Public mutable sim fields and drainable vectors permit future app-side authoritative writes. Privacy should tighten as each consumer migrates, not in one unreviewable visibility sweep.
- `COMPOUNDING` — `AppState` grouping before authority closure would merely hide the wrong owner inside `MatchState`. Match grouping therefore follows, rather than precedes, the sim runtime migration.
- `COMPOUNDING` — AppState migration must preserve one source of truth per field. Compatibility aliases or mirrored fields would create silent divergent writes.
- `EXACTIFICATION-RESIDUAL` — The native nonlocal client path that supplies `DAT_00880994` before overlay marking is unresolved. Rust uses the explicit `PlaceReadyBuilding.owner` on every peer, avoiding the legacy global without changing the verified endpoint rule (`doc: REGULAR_OVERLAY_WALL_AUTOFILL_COMMIT_GHIDRA_REPORT.md Q20`).
- `EXACTIFICATION-RESIDUAL` — Laser Fence Post, Firestorm wall, and AI perimeter planning are separate native branches and remain outside the regular stock-player wall slice. Their trigger and non-scope are recorded in the wall report.
- `UNKNOWN-RISK` — The native pseudo-building filler construction’s entire transitive RNG call graph was not exhausted. The verified fill helper makes no direct RNG decision. Rust’s direct overlay representation should not fabricate pseudo-building RNG draws; any future constructor-parity work must investigate before changing the shared stream (`doc: REGULAR_OVERLAY_WALL_AUTOFILL_COMMIT_GHIDRA_REPORT.md §4.1/Q13`).
- `EXACTIFICATION-RESIDUAL` — App-local per-building animation phase bases do not round-trip save/load and can re-phase presentation after loading. They remain presentation state unless a separate AnimClass persistence slice proves an authoritative dependency (`src/app/state.rs:483-499`).

## Design

### Components

#### `SimRuntime`

A sim-layer owner around authoritative `Simulation`, immutable `SimResources`, and derived `NavigationState`. It is the production entry point used by the app and headless runtime. Snapshots serialize authoritative `Simulation`; load rebinds resources and rebuilds derived navigation before advancing.

This is a Rust-native owner, not a port of native globals or vtables.

#### `SimResources`

Immutable match inputs resolved during loading: rules, height/map facts, overlay registry, animation sequence data needed by authoritative transitions, and trigger definitions. Presentation may share immutable source data where needed, but app cannot replace it mid-frame or pass per-frame behavioral choices.

#### `NavigationState`

The canonical dynamic `PathGrid` plus derived terrain-cost and zone caches. Mutation APIs cover structure add/remove, overlay change, bridge change, and terrain-object change. Overlay mutation updates passability and navigation inside the sim transaction; presentation receives a delta afterward.

#### `SimFrameOutput`

An owned, one-consumption batch:

- the existing tick summary and state hash;
- ordered placement results, including primary and autofill cells;
- sound, fire, lifecycle, trigger, combat-light, overlay, and other presentation events;
- coarse refresh hints only where a presentation cache cannot consume a narrower delta.

The output is derived from the committed frame and is not authoritative save/hash state.

#### `SimView`

An immutable facade for renderer/HUD queries. It exposes entity, house, fog, overlay, session, and navigation observations without allowing app mutation. It may initially delegate to existing immutable methods while direct public fields are closed progressively.

#### Scenario bootstrap and finalization

`sim::scenario_bootstrap` owns start gathering/assignment, fallback placement, MCV and starting-force selection, veterancy, and the exact scenario-RNG cursor. App constructs a `MatchLaunchDescriptor` from shell choices and receives a bootstrap receipt plus loading/presentation facts.

Sim finalization produces raw authoritative score rows and consumes the victory-bonus RNG once. App localizes names, chooses colors/assets, and renders the score screen without touching RNG or houses.

#### App ownership groups

`AppState` remains a small root over focused owners. The target shape is illustrative, not a mandate to move every residual field:

```text
AppState
  platform: PlatformState
  frontend: FrontendState
  match_state: MatchState
  input: InputState
  presentation: PresentationState
  remaining flat fields until their ownership cone is proven
```

The first follow-on migration should be the smallest complete owner with low behavioral risk, likely `PlatformState` (window lifecycle and local frame admission). `MatchState` waits until `SimRuntime` replaces the current split `simulation`/`path_grid` authority. `PresentationState` should split again if render, audio, and HUD fields would otherwise form another oversized grab bag.

### Interfaces / Contracts

The target production flow is:

```text
Frontend choices
  -> MatchLaunchDescriptor
  -> sim::scenario_bootstrap::build_runtime(...)
  -> SimRuntime

Input
  -> Vec<CommandEnvelope>
  -> SimRuntime::advance_frame(commands, lane, tick_ms)
  -> SimFrameOutput

Rendering/UI/audio
  <- SimRuntime::view()
  <- SimFrameOutput
```

Wall placement is:

```text
cursor/type/owner
  -> SimRuntime::wall_placement_preview(...)       // read-only
  -> app renders returned cells

PlaceReadyBuilding { owner, type_id, rx, ry }
  -> sim revalidates and recomputes at execution
  -> primary + ordered autofill commit
  -> overlay/navigation authority updated
  -> PlacementCommitted event in SimFrameOutput
```

Game speed at match creation comes from the launch descriptor. An in-match deterministic speed change enters through an explicit command or sim transition API at a defined frame boundary. Purely presentational options remain app-owned.

Transient event vectors become private once their final output consumer migrates. App never calls `drain(..)` on `Simulation` directly.

### Data Flow

1. Loading resolves immutable sim resources and app presentation resources.
2. Sim bootstrap consumes all setup RNG and returns a ready runtime plus immutable receipt.
3. App translates input to commands without applying outcomes.
4. Sim advances the native-shaped phase spine.
5. Existing post-spine authoritative animation/entity/particle work runs inside sim in the same relative slot it occupies today, before hashing. It is not moved earlier through gameplay phases without separate native evidence.
6. Command-tail overlay/structure changes finalize navigation before hashing.
7. Pending delete retains its current late-commit ordering.
8. Sim hashes authoritative state, packages transient outputs by moving them out in producer order, and returns.
9. App consumes events once and renders through `SimView`; presentation caches never feed state back into sim.

### Error Handling

- `SimRuntime` construction fails explicitly if required resources or map identity are missing; production advance never silently falls back to `None` resources.
- Snapshot restore validates the resource/map identity and rebuilds navigation before exposing a runnable runtime.
- Preview returns an explicit invalid/unavailable result rather than treating missing authority as an empty legal band.
- Output consumption is move-based. A batch cannot be drained twice.
- AppState group migrations are behavior-preserving and introduce no fallback alias to the old flat field.

### Testing Strategy

Each coherent slice uses the repository’s focused `--lib` tier after checking that no other Cargo process is active. Each passing slice is committed before the next begins. The full `cargo test -p vera20k --lib` suite runs exactly once at the end.

Required focused coverage:

- dual-runtime determinism: same launch descriptor and commands produce identical setup, output order, RNG states, and hashes;
- the returned hash covers living animation completion-facing, voxel/harvest state, bale overlay state, and particle spawns;
- terminal and ordinary frame fixtures preserve frame-commit/pending-delete ordering;
- every output category is delivered once and empty on a second frame without new producers;
- regular wall endpoints at distances 1, 5, and 6; wrong owner/type; damaged endpoint; blocker with no partial fill; multi-direction N/E/S/W ordering; one product consumption;
- preview equals execution result when state is unchanged and safely diverges when execution-time state changes;
- wall/structure/bridge/terrain navigation is current before hash and after snapshot restore;
- bootstrap RNG receipt and score-finalization RNG order remain bit-identical to pre-migration fixtures;
- app boundary tests prove no app module mutates sim overlay, entities, RNG, session options, or navigation after `advance_frame`;
- each AppState ownership migration compiles all consumers and has focused owner tests where logic exists; mechanical grouping commits contain no behavioral edits.

## Architectural Decisions

- Follow the existing command/event pattern rather than introduce an ECS, callback bus, or app-owned transaction layer.
- Add a sim runtime owner because immutable resources and derived authoritative caches need a home without serializing GPU/UI state or recreating native globals.
- Keep transient outputs un-hashed but derive them only from committed ordered producers.
- Preserve current post-spine relative timing during the first authority move. Exact native rescheduling is separate evidence-backed work.
- Keep `AppState` as the application composition root. Nested owners improve borrowing and visibility without forcing every app function into a new object-oriented service.
- Migrate one ownership cone at a time. This deliberately leaves temporary flat residual fields, which is preferable to a 213-field mechanical rewrite that obscures authority errors.
- Do not use `Deref`, forwarding accessors for every field, or duplicate old/new fields. Those patterns hide coupling instead of decomposing it.

## Alternatives Considered

### Patch only the known leaks

Move the currently identified RNG and post-hash calls but retain public `Simulation`, app-owned `PathGrid`, separate drains, and renderer-owned wall logic. This is smaller initially but leaves the same architectural trap for every future mechanic and does not satisfy the command/view/event boundary.

### Big-bang private Simulation and AppState rewrite

Create the final runtime API and move all AppState fields in one atomic change. This produces a superficially clean endpoint but has an excessive blast radius, makes tick-order regressions hard to isolate, and directly violates the requested incremental AppState constraint.

### Chosen incremental authority closure

Close one authoritative producer/consumer slice at a time, then group one complete AppState owner at a time. It reaches the same clean direction while keeping behavioral commits evidence-backed, tests focused, and mechanical state moves reviewable.
