# Engine Domain Boundaries and Ownership Cleanup Design

**Date:** 2026-08-15  
**Status:** APPROVED — adversarial review passed; F01-F14 frozen  
**Scope:** production client, headless scenario path, simulation, rules, map, render/UI/sidebar/audio, persistence/replay, and architecture tests

## Outcome

Establish one source-backed model of the current VERA20k engine, then close the finite ownership and dependency gaps in the frozen ledger below. The endpoint is not a new engine and not a visual/gameplay redesign. It is the current behavior expressed through explicit owners, one-way dependencies, concrete APIs, and module paths that match those owners.

The work is complete only when every frozen item is either implemented or explicitly retained as an intentional boundary, each coherent slice has focused `--lib` validation and an independent read-only review, the final full `cargo test -p vera20k --lib` passes once, and the feature branch is clean and committed.

## Evidence and constraints

### Project contracts

- `ENGINE.md` is authoritative for evidence, deterministic state, native-to-Rust translation, module boundaries, change management, and Cargo use.
- `AGENTS.md` requires one bounded slice at a time, an evidence label for sim behavior, focused `--lib` tests while working, one final full `--lib` suite, and incremental commits on a `feature/*` branch.
- This design extends rather than reopens the already merged `docs/plans/2026-08-14-app-sim-boundary-and-appstate-ownership-design.md` and `docs/plans/2026-08-15-single-app-tree-design.md`. `SimFrameOutput`, sim-owned navigation, scenario bootstrap/finalization, `PlatformState`, and the `src/app/` facade are treated as completed foundations.

### Current-source anchors

- Process entry and application construction: `src/main.rs:11-89`.
- Winit event/input dispatch: `src/app/handler.rs:47-664`.
- Frame admission, simulation, render, present, and post-present loading: `src/app/frame.rs:22-654`.
- Flat process aggregate: `src/app/state.rs:26-522`.
- Match construction: `src/app_init.rs:1126-2179`, `src/app_init_helpers.rs:548-733`.
- Authoritative frame output: `src/sim/world/mod.rs:282-316`, `4758-4811`.
- Master-frame body and late commit: `src/sim/world/mod.rs:4820-5748`.
- Snapshot validation/restoration: `src/sim/snapshot.rs:261-306`, `507-609`, `1284-1583`.
- Dynamic overlay authority: `src/sim/overlay_grid.rs:116-137`, `298-319`, `510`; app render index handoff: `src/app_sim_tick.rs:1470-1474`, `1787-1815`.
- Diagnostic replay creation/record/flush: `src/app_sim_tick.rs:35-117`, `871-882`, `1414-1421`.
- Public root app modules: `src/lib.rs:49-193`.

### Verified behavior used as a preservation oracle

- `docs/research/RENDER_LOGIC_COUPLING_MAIN_TICK_GHIDRA_REPORT.md`: in normal local skirmish, the native render call occurs after input/AI/map logic but before the later `LogicClassPerTickUpdateLiveVector`, service work, and frame-counter commit. Scenario-delay can render without logic; replay and network paths have additional conditional calls. The report's shorthand "after all logic" is not used as an ordering oracle here.
- `docs/research/ADVANCE_TICK_PHASE_PARTITION_NATIVE_SPINE_GHIDRA_REPORT.md`: native phase spine and late-frame ordering.
- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md` and `docs/research/miner/ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md`: `CreditsClass::AI` advances the displayed counter once per game frame with the existing step formula.
- `docs/research/MARKTERRAINDIRTY_FULL_CALLER_MATRIX_GHIDRA_REPORT.md` and `docs/research/RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE_GHIDRA_REPORT.md`: verified terrain/radar dirty producers and update ordering. These documents support a later parity fix, but the architecture slices must not invent unverified radar color behavior.
- `docs/research/LOADING_FIRST_RENDERER_CORRECTED_COMPOSITION_DATA_READINESS_GHIDRA_REPORT.md`: loader work continues only after the first native loading frame is presented.
- `docs/research/REGULAR_OVERLAY_WALL_AUTOFILL_COMMIT_GHIDRA_REPORT.md`: `OverlayGrid` is authoritative simulation state; presentation is a consumer.

No new gamemd behavior claim was inferred from file placement. Unless a ledger slice cites verified research, it is a behavior-preserving Rust architecture change.

## Current production architecture

### Runtime flow

```text
main
  -> app::App / winit ApplicationHandler
  -> AppState initialization

frontend choice
  -> app_loading state machine
  -> app_init rules/map/assets/scenario assembly
  -> Simulation + app presentation resources
  -> app_transitions installs the match

window input
  -> app handler priority (capture -> splash -> modal/shell -> in-game)
  -> app input/context/sidebar command producers
  -> Simulation pending command queue

frame
  -> app_sim_tick drains due commands
  -> Simulation::advance_app_frame
  -> SimFrameOutput
  -> app translates presentation/audio/lifecycle facts
  -> app_render reads the committed sim plus presentation resources
  -> GPU submit/present

save/load
  -> app filesystem/UI transaction
  -> GameSnapshot validates serialized Simulation and content identity
  -> Simulation restores skipped caches from current immutable match inputs
  -> app commits the replacement
```

The current Rust render path deliberately reads the state returned by `Simulation::advance_app_frame`; `SimFrameOutput` is an owned transient event batch, not a frozen render snapshot. That current Rust order is preserved and characterized before runtime extraction. It is **not** claimed to match the exact native split: native normal-local rendering occurs before later live-vector/service/frame-counter work, and its scenario-delay/replay/network branches differ. Changing Rust to that exact native order is a separate parity task, not an incidental result of this refactor. The cleanup introduces an immutable borrow facade, not a second copy of the world.

### Domain owners today

| Domain | Current authority | Current consumer/mirror | Decision |
|---|---|---|---|
| Platform/window/pacing | `AppState::platform` | app handler/frame | Correct owner; retain. |
| Static source map | `MapFile` and load result | app presentation/bootstrap | Immutable source; retain. |
| Rules layering | `RulesLayerStack` -> `RuleSet` | sim and app presentation | Correct concept, wrong type dependencies inside `RuleSet`; fix. |
| Art data | full `ArtRegistry` cloned into `RuleSet` and separately stored in `AppState` | sim animation timing and render art lookup read different clones | Duplicate immutable resource with divergence risk; make `RuleSet`/`SimResources` the sole match owner and let presentation borrow it. |
| Entities, scheduler, clocks, RNG, houses, fog facts | `Simulation` | render/sidebar/app | Correct authority; narrow reads/mutations. |
| Dynamic paths/zones/terrain projection | `Simulation` | app render/input | Correct authority after prior work; remove caller-selectable path ambiguity. |
| Base resolved terrain | app-retained immutable source-derived grid | cloned/reprojected into live sim, used for restore/static rendering | Intentional dual representation; rename and bind as a resource so it cannot masquerade as live authority. |
| Dynamic overlays | serialized/hashed `OverlayGrid` | app candidate list and minimap pixels | Sim is authority; make app structures explicit indexes/caches only. |
| Commands | serialized sim queue | multiple app producers | Correct owner, fragmented ingress; fix. |
| Trigger definitions/runtime | static definitions in load/app, runtime in sim | app passes definitions each frame | Runtime owner is correct; bind immutable definitions once. |
| Audio events | pure `SimSoundEvent` output | app queue/players/EVA | Correct layer direction; group fragmented app runtime. |
| Save bytes/content validation | `GameSnapshot` | app filesystem/UI | Correct split; make app commit transactional and unify repository policy. |
| Native replay/lockstep substrate | `sim::replay`, `net::lockstep` | tests/tooling, byte round-trips | Intentional substrate; not dead, and no speculative transport abstraction. |
| Diagnostic replay log | field on `Simulation` | app creates, records, flushes | Misplaced non-authoritative app diagnostics; move. |
| Frontend map catalog/routes | parallel app vectors/booleans | shell input/render | Duplicate projections/invalid states; consolidate. |
| GPU/render caches | flat `AppState` fields | renderer | App-owned and non-authoritative, but poorly grouped/named; group after boundaries close. |
| Tests | module-local, including sim tests importing render/net | all layers | Preserve behavior tests; relocate boundary-breaking integration tests and add dependency guards. |

### Proven dependency inversions

Target dependency direction is:

```text
util/assets
    -> rules and static map data
    -> simulation
    -> render/sidebar/UI/audio views
    -> app composition root
```

Render/UI may depend on immutable sim/map/rules observations. App may depend on every lower layer. Reverse imports are forbidden.

Current production exceptions are finite:

1. `util -> sim`
   - `src/util/fixed_math.rs:280,288` delegates production facing conversion to `sim::substrate::direction_tables`.
   - The direction-table module is pure, read-only, and depends only on util. Move it atomically to `util::direction_tables`; sim callers consume the lower owner.

2. `rules -> sim`
   - animation sequence data: `src/rules/infantry_sequence.rs:22`, `src/rules/shp_vehicle_sequence.rs:26`, `src/rules/ruleset.rs:2264`, `3335-3355`;
   - sim interned handles inside `RuleSet`: `src/rules/ruleset.rs:2265-2272`, `2765-2819`;
   - mission data: `src/rules/ruleset.rs:2275`, `2622`;
   - locomotor installation resolver: `src/rules/object_type.rs:1257-1264`.

3. `rules -> map`
   - `RuleSet::first_building_type_for_overlay` accepts `map::overlay_types::OverlayTypeRegistry` at `src/rules/ruleset.rs:2857-2868`; ordinary wall selling calls it at `src/sim/world/world_commands.rs:360-392`.
   - The registry is parsed from rules/art data despite its current map path. Rules-semantic overlay registry data moves under rules; render-only overlay asset helpers move under render/assets. First-match declaration order is preserved.

4. `map -> sim/render`
   - map mission selector: `src/map/entities.rs:17`, `60-65`;
   - bridge facts: `src/map/theater.rs:20`, `src/map/resolved_terrain.rs:270-284`;
   - fixed cell indexing/passability reexports: `src/map/resolved_terrain.rs:1344-1359`, `1685-1689`, `1972-2042`;
   - visible GPU instances and sim fog/bridge state: `src/map/terrain.rs:18`, `701-849`;
   - `SimRng` adapter: `src/map/rmg/randomize.rs:11-22`.

5. `render/sidebar/ui -> app`
   - combat-light draw DTO: `src/render/combat_light.rs:12`;
   - cursor ID/software cursor DTOs: `src/render/cursor_atlas.rs:3`, `361-460`;
   - sidebar `TargetingMode`: `src/sidebar/sidebar_view.rs:30`, `73`, `349-359`;
   - map menu DTO: `src/ui/main_menu.rs:5`, `src/ui/skirmish_shell/state/*.rs`, `src/skirmish_scenarios.rs:7`.

Test-only reverse edges are tracked separately and do not justify moving production types upward.

## Concrete defects and compounding risks

Severity always includes its trigger frequency.

- **High, on failed panel/dev save loads:** startup correlation/receipt state is cleared before validation while `load_save_file` silently returns on error (`src/app/in_game.rs:335-360`, `511-526`; `src/app_input.rs:1559-1648`). A corrupt or mismatched save can leave the running match partially de-certified.
- **High, on every context/minimap order:** two batch producers extend `pending_commands` directly (`src/app_context_order.rs:213-239`, `src/app_sidebar_render.rs:260-315`) while other producers use the scheduler API. Current bytes are not proven wrong, but there is no enforceable ingress boundary.
- **High, on nearly every in-game input/render pass:** `current_sidebar_view(&mut AppState)` advances displayed credits, synchronizes targeting, and clamps scroll; it is called by render, gadgets, input, and tooltips. Projection cadence therefore depends on consumer count instead of one game-frame update.
- **High, on dynamic ore/wall/bridge state and snapshot restore:** live overlay identity is sim-owned, but an app vector remains the iteration anchor and minimap pixels are another cache. The tactical renderer revalidates against `OverlayGrid`, so the architecture is mostly correct but the cache contract is implicit.
- **High architectural pressure, on every rules/content change:** lower-layer `RuleSet` embeds sim runtime types and handles, forcing mutually coupled rules/sim changes.
- **High architectural pressure, on every animated object/effect render:** the full `ArtRegistry` is cloned into `RuleSet` and separately installed in `AppState` (`src/app_init.rs:1464`, `1531`, `2208`; `src/rules/ruleset.rs:2254`; `src/app/state.rs:288`). Sim timing and presentation can observe different copies.
- **High architectural pressure, on every scenario load and headless parity run:** app and headless construction diverge because entity construction is coupled to GPU atlas creation; headless omits houses/entities and the selected mode pass (`src/headless_scenario.rs:8-13`, `47-220`).
- **Medium, on every frame:** `advance_tick`/master-frame adapters accept behavioral resources and an optional path grid even though the sim owns canonical navigation (`src/sim/world/mod.rs:4735-4755`, `4820-4829`).
- **Medium, on map chooser/random-map paths:** three map collections and four shell-route booleans can drift or represent impossible combinations (`src/app/state.rs:121-127`, `188-191`).
- **Medium, on every audible event and scenario exit:** players, registries, queues, EVA gates, and teardown sequencing are flat fields with no cohesive app owner. The sim/audio boundary itself is correct.
- **Medium maintenance risk, on nearly every app change:** about forty-five root `app_*` modules remain publicly declared in `src/lib.rs`, and `AppState` still has roughly 160 public-to-crate fields across unrelated owners.
- **Medium maintenance risk, on authoritative field changes:** world, snapshot, and manual hash logic are oversized and separated from some of their owners. Mechanical splitting would be dangerous; owner-based extraction is needed.

## Target architecture

### Concrete owners, not framework abstractions

No new trait is introduced unless a second real implementation already exists. The target types are concrete:

```text
AppState
  platform: PlatformState
  assets: ProcessAssets
  renderer: RendererState
  audio: AppAudioRuntime
  frontend: FrontendState
  persistence: PersistenceState
  match_state: Option<MatchState>
  diagnostics: DiagnosticsState

MatchState
  runtime: sim::runtime::SimRuntime
  input: MatchInputState
  presentation: MatchPresentationState
  audio: MatchAudioState
  diagnostics: MatchDiagnosticsState

SimRuntime
  simulation: Simulation
  resources: SimResources

SimResources
  rules
  immutable map/height/overlay facts
  trigger definitions
  base resolved-terrain template
```

`AppAudioRuntime` is process-wide because music, device/channel state, registries, and shell audio span match transitions. `MatchAudioState` contains only match-local queues, cooldown/edge state, and teardown state. `PersistenceState` owns the one `SaveRepository`, list cache, and last-save/load metadata. `DiagnosticsState` is process-wide developer/capture UI configuration; `MatchDiagnosticsState` owns match-lifetime parity capture and diagnostic replay state. `ProcessAssets` similarly remains process-wide. Its concrete `AssetManagerSlot::{Available, Loading}` makes the worker lease explicit and guarantees that completion, cancellation, and failure return the manager; a naked `Option<AssetManager>` is not the state machine.

`RuleSet` is the sole match owner of the complete immutable `ArtRegistry`. `SimResources` owns that `RuleSet`; renderer and construction code borrow art data through resources. Where sim needs compact resolved IDs or GPU-independent frame counts, it stores narrowed catalogs/handles, not another full mutable registry.

The exact field partition is validated per ownership cone. No `Deref`, forwarding-field compatibility layer, mirrored old/new fields, service locator, or generic event bus is allowed.

### Runtime interface

```text
frontend choices
  -> app-owned shell/session resolution
  -> sim-owned MatchLaunchDescriptor
  -> shared GPU-free construct_scenario
  -> SimRuntime
  -> app/loading builds PresentationManifest from resources + SimView

input/network substrate
  -> SimRuntime::queue_command(s) / typed ingress
  -> SimRuntime::advance_frame(lane, tick_ms)
  -> SimFrameOutput

render/sidebar/UI
  <- SimRuntime::view()          // immutable borrow facade, no world clone
  <- SimFrameOutput              // one-consumption transient facts

save restore
  -> SaveRepository validates bytes/header/content
  -> PreparedLoad builds restored sim + map-restore output using current bound facts
  -> app runs one enumerated infallible commit bundle only after all fallible work succeeds
```

`SimRuntime::advance_frame` always uses its own canonical path snapshot and bound immutable resources. `Simulation` retains internal deterministic ownership; `SimRuntime` is the construction/resource/API boundary used by app, headless, and replay execution. Raw `Simulation::advance_master_frame` remains module-private for implementation tests only; there is no production adapter that accepts independently swappable rules/map/path resources.

### Presentation projections

- Rename the immutable app/resource grid to `base_resolved_terrain` (or `terrain_template`). It is never a live runtime view.
- Replace the generic `Vec<OverlayEntry>` name with an `OverlayRenderIndex`. Its frozen contract is: source-map coordinates retain source order; an update to an existing coordinate retains its slot; the first dynamic appearance appends in `SimFrameOutput.overlay_updates` order; clearing leaves a coordinate tombstone that draws nothing because live `OverlayGrid` is empty; reoccupation with a different ID/data reuses the slot and reads the new live value; full restore appends only missing occupied coordinates in restore-output order. It stores coordinates and only proven immutable source metadata, never live overlay identity/frame/occupancy.
- Keep the existing verified terrain-dirty event channel. Wiring additional source-backed producers is a separate parity commit; no radar color formula changes belong to this architecture work.
- Sidebar credits update exactly once after a committed **ordinary** gameplay frame. No-admit redraws, pause/menu, inactive-window redraws, and `TickLane::NetworkModal` do not advance it; an exact-step capture advances it iff that step commits an ordinary frame. Targeting/gadget/scroll changes are applied by their explicit input/state transitions, not by view construction. `build_sidebar_view` and all consumers are pure reads of the same retained projection. The corrected credit rule is `SIDEBAR_SYSTEM_GHIDRA_REPORT.md:962-972`: the stored 1/3 value does not delay the current step.
- Render-owned cursor, light, terrain-instance, and software-cursor DTOs live in render. Sidebar owns its narrow armed-entry projection. Scenario catalog DTOs live with scenario catalog data, not app initialization.

### Persistence and diagnostics

- `SaveRepository` owns directory discovery, header scan, read/write/delete, and both explicitly named current policies: panel ordering by embedded snapshot time and quickload selection by filesystem modification time. Their disagreement is preserved and tested in this behavior-preserving refactor; unifying them is `VERA-internal / gamemd equivalent UNCHECKED` and remains residual.
- `PreparedLoad` performs byte/content validation, stable-identity restoration, cache rebuild, map-authority restore, type/sound resolution, and creation of the overlay restore output before commit. Failure changes no sim hash, match/startup admission state, screen, pacer, diagnostics, overlay index, lighting, or panel state. The commit bundle is enumerated and infallible: reset exit runtime, swap the restored sim, sync speed, clear transient combat lights, apply overlay-index output, rebuild best-effort presentation atlases without discarding the prior atlas on failure, recompute lighting, reset pacer, close panel, and record the path. A successful same-content in-scenario load preserves the current accepted-startup admission state, matching the existing quickload path; any future cross-session load needs its own explicit admission receipt.
- Diagnostic `ReplayLog` belongs to `MatchDiagnosticsState`, not serialized `Simulation` or process-wide `DiagnosticsState`. Failed load leaves the active segment untouched. Successful in-scenario load retry-flushes/closes the pre-load segment before commit and lazily starts a new segment at the restored tick; new-match replacement and scenario/app teardown likewise retry-flush before dropping the owner. Native replay codecs remain in `sim::replay`.
- Fog facts stay serialized/hashed in sim. The merged local-owner visibility becomes an explicitly nonserialized `FogViewCache`, discarded/dirty after load and owner change and rebuilt before the first tactical render. `FogState::generation` remains in its exact snapshot position for `SNAPSHOT_VERSION = 81` compatibility and continues as an explicitly named/updated wire shadow, but render no longer consumes it. Removing that shadow requires a future coordinated snapshot-version bump. Tests prove cache rebuild/local-owner changes cannot alter `state_hash`. Debug logging is toggled through a sim method that updates existing and future entities consistently.

## Approaches considered

### A. Mechanical module-tree completion first

Move every `app_*` file under `src/app/`, group fields, and leave runtime APIs unchanged. This gives a tidy tree but hides the wrong owners inside new folders and structs. It also creates broad call-site churn before correctness boundaries are fixed.

### B. Dependency- and authority-first incremental closure — recommended

Fix transactional defects and mutation ingress first; move misplaced lower-layer vocabulary/DTOs; then introduce the runtime/resource/view boundaries; only afterward group state and move modules. Each slice is behavior-preserving or separately evidence-backed, has focused tests, and is independently reviewable.

### C. Big-bang runtime/AppState rewrite

Create the final owner tree, private `Simulation`, and new construction path in one branch-wide rewrite. This has the shortest-looking end state but the largest tick/RNG/save/render regression surface and makes attribution or rollback impractical.

## Frozen cleanup ledger

The ledger is finite. Later discoveries are residuals unless they are required to make the current item correct or deterministic.

The dependency order is strict:

```text
F01, F02, F03 (independent correctness/API closures)
  -> F04 rules vocabulary/resource ownership
  -> F05 map lower-layer closure
  -> F06 presentation DTO closure
  -> F07 SimRuntime/SimResources/minimal SimView foundation
  -> F08 explicit terrain/overlay projections
  -> F09 shared construction plus app/headless/replay execution
  -> F10 diagnostics/fog/read API closure
  -> F11 process/frontend/audio owners
  -> F12 app state and module-tree completion
  -> F13 named sim owner decomposition
  -> F14 final guards and closed compatibility disposition
```

Within a multi-commit item, each commit has one compiling owner. A field moves from old owner to new owner in the same commit; no mirrored compatibility field or forwarding `Deref` is permitted.

### F01 — Transactional persistence and one save repository

**Why first:** high correctness risk on every failed panel/dev load; medium user-visible inconsistency on quickload selection after file copy/touch.

- Extract concrete `app::persistence::{PersistenceState, SaveRepository, PreparedLoad}` without waiting for F07; move the repository, list cache, and last-save/load metadata into that owner atomically.
- Preserve the two current named selection policies: panel order uses embedded snapshot time; quickload uses filesystem modification time. Test contradictory order explicitly and mark unification `VERA-internal / gamemd equivalent UNCHECKED`.
- `PreparedLoad` performs every fallible step before mutation: bytes/header/content validation, identity restoration, skipped-cache rebuild, map-authority restore, type/sound resolution, and restore-output creation.
- Commit only the enumerated infallible bundle in the persistence section above. Best-effort atlas rebuild retains the prior atlas on failure.
- Preserve sim hash, admission receipt/correlation, screen, pacer, diagnostics, overlay index, lighting, and panel state for bad bytes, content mismatch, missing resources, identity failure, and map/cache restore failure.
- Preserve accepted-startup admission on a successful same-content in-scenario load.

### F02 — One command-ingress API

**Why:** high determinism/API risk on every context/minimap command and prerequisite for any future live lockstep owner.

- Add concrete single/batch ingress beside the current scheduler.
- Route context, minimap, sidebar, local, replay, and test producers through it.
- Make `pending_commands` non-public outside the owning module, with read-only test helpers where needed.
- Preserve byte round-trips, issue ticks, stable order, and serialized queue layout.

### F03 — Pure sidebar projection

**Why:** high cadence risk on nearly every in-game frame.

- Separate explicit update/input transitions from pure view construction.
- Advance displayed credits once after a committed ordinary gameplay frame using the corrected verified formula; the stored 1/3 value never delays the current step.
- Freeze gates: no-admit redraw, pause/menu, inactive-window redraw, and `NetworkModal` do not advance; exact-step capture advances iff it commits an ordinary frame.
- Apply targeting/gadget/scroll mutations at their owning input/state transitions, then share one immutable view with render, input, gadgets, and tooltips.
- Prove repeated reads do not mutate state.

### F04 — Rules owns rules vocabulary and immutable match art

**Why:** high compile-time and runtime divergence pressure on every content/sim/art change.

- Move mission-control data and mission selectors to `rules::mission_data`.
- Move immutable sequence vocabulary/catalog construction to `rules::animation_sequence`; keep animation runtime in sim.
- Consolidate installed locomotor parsing in `rules::locomotor_type`.
- Move rules-semantic `OverlayTypeRegistry` data under rules and render/asset-only helpers under their consumers, eliminating `rules -> map` while preserving declaration/first-match order used by wall selling.
- Keep canonical names in `RuleSet`; build sim-owned `ResolvedRuleHandles` beside `TypeHandleTable` after interning and restore them from rules on load.
- Make `RuleSet` the sole owner of the full immutable `ArtRegistry`; remove the app clone and let presentation borrow it.
- Preserve enum discriminants, serde representations, rules hash, lookup order, effect/animation timing, state hashes, and RNG.

### F05 — Low-level util and static map data no longer depend upward

**Why:** high architectural pressure on every direction/map/terrain change and medium render pressure every frame.

- Move the pure `sim::substrate::direction_tables` tree atomically to `util::direction_tables`; update sim consumers downward and preserve every native table/exact-equality test.
- Move static bridge axis/anchor facts to `map::bridge_facts`.
- Move fixed cell indexing to a lower map coordinate module and use rules passability types directly.
- Keep map parsing/static terrain in `map`; move visible instance construction to `render::terrain_instances`. Shared tactical projection math lives in a presentation-neutral coordinate module only if both input and render have real callers; otherwise the concrete consumer owns it.
- Remove the map-owned `SimRng` implementation through the existing generic RNG input/app adapter.
- Move tests that need upper layers to integration/boundary modules rather than retaining production reverse imports.
- Preserve bridge classification, cell indices, passability tables, culling/projection results, tile UVs, and RMG cursor consumption.

### F06 — Presentation layers no longer import app

**Why:** medium architecture risk on every render/sidebar/UI change.

- Move cursor/software-cursor DTOs to render.
- Move combat-light draw records to render.
- Give sidebar a concrete `ArmedSidebarEntry` projection instead of `TargetingMode`.
- Consolidate `MapMenuEntry` with the scenario catalog DTO outside app initialization.
- Move generic credit glyph generation to `render::sidebar_text` and keep only an app/sidebar adapter.
- Remove compatibility reexports only after all in-repo callers move.

### F07 — Bind immutable resources and establish the runtime/view foundation

**Why:** medium-high authority risk on every frame and prerequisite for F08-F10.

- First add a production call-order characterization for current Rust: command drain -> `advance_app_frame` -> digest -> local fog preparation -> output translation -> render. It must also prove deferred loading begins only after loading present. This freezes current Rust behavior without claiming native-order parity.
- Introduce concrete `SimRuntime { simulation, resources }` and a minimal immutable `SimView<'_>`.
- Move the app's simulation slot to `SimRuntime` atomically; then move one complete immutable resource cone per commit: rules/art, overlay registry, height/map facts, trigger definitions, and base terrain template. Remove each old app field in the same commit.
- Always pin runtime-owned navigation; remove caller-selectable path grids and per-frame optional behavioral resources from the production app adapter.
- Keep `SimFrameOutput` shape/order unless a narrower source-backed delta replaces a coarse hint.
- Direct `Simulation::advance_master_frame` becomes implementation-private; app is the first runtime client, with headless/replay migrated in F09.

### F08 — Explicit terrain template and overlay presentation index

**Why:** high ambiguity on dynamic overlay play and every snapshot restore.

- Rename/bind the immutable base grid as the runtime's `TerrainTemplate`; live resolved terrain remains sim authority and snapshot restore rebuilds from the template.
- Replace the app overlay vector with `OverlayRenderIndex` using the exact source/update/tombstone/reoccupation/restore order contract above.
- Make tactical, lighting, and restore consumers query live identities/facts through `SimView`; template reads are limited to static geometry and restore.
- Keep minimap dirty cells as an explicit ordered channel. Only verified producer additions from the radar research may change behavior, in a separately evidence-labeled commit; radar color math is out of scope.
- Test clear/reoccupy-different-ID, dynamic ore and wall insertion, bridge changes, full snapshot restore, and minimap dirty ordering.

### F09 — One GPU-free scenario construction and one execution resource contract

**Why:** high parity risk on every headless/replay run and every future load-path change; ordinary app loads use the fuller path.

- Before extraction, add a representative app-path construction fingerprint covering processed-rules hash/layer order, map digest, Scenario/Main/MapGen cursors, stable-ID and LogicVector order, house order, state hash, animation/effect/HVA metadata, overlay/smudge authority, and post-map outputs.
- Resolve shell/random frontend choices in app, then pass a sim-owned behavioral launch descriptor containing only resolved gameplay facts.
- Extract GPU-free `construct_scenario` used by app and headless. App/loading builds `PresentationManifest` afterward from immutable resources plus `SimView`; sim never imports presentation.
- Parse HVA frame counts through a GPU-independent assets/rules catalog consumed before sim finalization and by renderer; renderer never writes sim metadata.
- Store `SimRuntime` in `HeadlessScenario` and `ReplayRunner`; remove their independently swappable rules/height/path/overlay execution inputs. Any raw test adapter is module-private and named fixture-only.
- Compare app and headless GPU-free results for the complete fingerprint above and runtime-backed replay hashes for every tick.
- Assert construction/pumping still starts only after the first loading present (`src/app/frame.rs:626-632`, `src/app_loading.rs:1418-1423`).

### F10 — Immutable sim view and diagnostics boundary

**Why:** medium mutation/API risk on every render frame and every diagnostic/load session.

- Expand the minimal `SimView<'_>` getters and migrate app/render/sidebar reads one owner cone at a time.
- Close direct app mutations with methods for debug logging, fog view preparation, replay recording, and other proven writes.
- Move diagnostic replay state/I/O to match diagnostics; retain native replay/lockstep substrate.
- Define its lifecycle explicitly: failed load retains the segment; successful load closes it before commit and starts a restored-tick segment lazily; new match, scenario teardown, and app exit use the existing retry-safe flush before drop.
- Move merged local-owner visibility to nonserialized `FogViewCache`. Discard/dirty it on load and owner change; rebuild before first tactical render.
- Keep the exact version-81 serialized `generation` field as an explicitly named/updated compatibility shadow in the same wire position; render does not consume it. Do not bump `SNAPSHOT_VERSION` in this goal.
- Prove first-render, owner-change, round-trip, and repeated-cache rebuild behavior cannot alter `state_hash`.
- Progressively reduce `Simulation` field visibility; no giant privacy flip.

### F11 — Consolidate process assets, frontend routes/catalog, and audio owners

**Why:** medium route/catalog/audio risk on common shell and match transitions.

- Introduce process-wide `ProcessAssets` with explicit `AssetManagerSlot::{Available, Loading}` lease/return transitions. Preserve raw/MIX precedence, theater activation, sticky CRC caches, and every failure/cancellation return path; clarify current-resolution versus process-sticky lookup names.
- Split the load result into concrete scenario/runtime inputs and presentation assets instead of one cross-domain bag.
- Replace parallel map collections with one `ScenarioCatalog` and explicit derived indices.
- Replace shell route booleans with one route enum carrying the Skirmish return destination; remove the proven unwritable compatibility branch.
- Extract process-wide `AppAudioRuntime` and match-local `MatchAudioState`. Preserve sound-event order, IDs/positions, channel selection, cooldowns, music/EVA transitions, and exit teardown. Do not implement currently dropped sounds without evidence.

### F12 — Finish the closed app state and module-tree inventory

**Why:** medium maintenance risk on nearly every engine change; low direct runtime risk.

- Form the exact root owners shown in Target Architecture: `PlatformState`, `ProcessAssets`, `RendererState`, `AppAudioRuntime`, `FrontendState`, `PersistenceState`, optional `MatchState`, and `DiagnosticsState`. `MatchState` contains `SimRuntime`, `MatchInputState`, `MatchPresentationState`, `MatchAudioState`, and `MatchDiagnosticsState`. No unrelated flat field remains.
- Move the current root inventory from `src/lib.rs:54-193` under one `src/app/` tree:
  - `frontend/`: launch/list-maps/skirmish/session/shell/menu/score/quit/startup/random-map modules;
  - `loading/`: init/init-helpers/loading/composition/progress/transitions and presentation-install pieces;
  - `input/`: input/hotkeys/context-order/commands/camera/cursor/entity-pick/gadgets/options/tooltips/messages;
  - `match_runtime/`: sim-tick/frame-pacer/scenario-exit and output translation;
  - `presentation/`: render/instances/sidebar/building/fire/light/chute/UI overlays/selection/target lines/spawn/capture;
  - `persistence/`: F01 repository/transactions, save-load panel, and options persistence;
  - `diagnostics/`: process debug/capture UI plus match-lifetime parity and diagnostic replay owners; the save/load panel stays under `persistence/`.
- The existing `src/app/{handler,frame,initialize,in_game,shell_*}` files move into those same owners; no parallel facade tree remains.
- Finish criterion: `src/lib.rs` declares no `pub mod app_*`; only intentional public entry types are reexported, and `AppState` contains only the eight named owners above.
- Oversized app verdicts are closed: F01/F12 split `app_input`; F07/F10/F12 split `app_sim_tick`; F09/F11/F12 split `app_init` and `app_loading`; remaining owner-local files are retained without a line-count target.

### F13 — Named simulation owner decomposition, no phase rewrite

**Why:** medium maintenance risk on authoritative field/save/hash changes; every match depends on exact order.

- Centralize object-kind lookup, membership get/set, and removal in `ObjectSubstrate`; replace the repeated registration/removal dispatch while retaining store locations and serialization order.
- Move `ScenarioSession` hash/restore helpers beside session; move `ObjectSubstrate` fold/validation helpers beside substrate. `world_hash.rs` and `snapshot.rs` remain ordered coordinators with identical call/fold order.
- `sim/world/mod.rs` retains the authoritative phase spine after F07 removes resource/API concerns; no phase reordering or wholesale file move is authorized.
- Explicit retain verdicts: `sim/combat/mod.rs`, `rules/ruleset.rs`, `map/resolved_terrain.rs`, `sim/snapshot.rs`, `sim/world/world_hash.rs`, `world_commands.rs`, and `lifecycle.rs` may remain large after the named extractions because their remaining order/data cohesion is load-bearing.
- Prove per-tick hashes, RNG cursors, LogicVector membership, lifecycle removal order, snapshot bytes/version, and restored hashes are identical.

### F14 — Final dependency guards and closed compatibility disposition

**Why:** low direct runtime risk; required to keep the repaired architecture from regressing.

- Add source-level guards for: no production `util -> sim/rules/map/render/app`; no production `rules -> sim/map`; no production `map -> sim/render`; no production `render/sidebar/ui -> app`; no sim -> presentation/audio/net; and no root `app_*` declarations.
- Relocate the known test-only sim -> render/net checks to integration/boundary test modules.
- Remove the cursor/app compatibility reexports after F06 and the unwritable shell boolean branch after F11.
- Correct the stale `app_sidebar_text`/audio layering comments; keep the live implementations.
- Retain `render::tactical_compat` because external use is unknown; retain native replay/lockstep, `pending_smudge_requests`, `resource_nodes`, and inert `Tunnel`/`DropPod` discriminants because tooling/schema reachability is known or removal evidence is absent.
- No other compatibility deletion is in scope. A later discovery is a residual unless it violates a final guard.

## Intentional boundaries and residuals

These are not silent omissions:

- **Intentional:** base resolved terrain and live sim resolved terrain remain two representations because the live grid is skipped from snapshots and rebuilt from an immutable source template. Their names/types must make the distinction explicit.
- **Intentional:** this goal preserves current Rust's render-after-`advance_app_frame` order through `SimView`; there is no cloned render world. Exact native pre-late-region render placement and scenario-delay/replay/network cadence remain a separate parity task.
- **Intentional:** snapshot version 81 keeps the serialized fog-generation wire shadow even after render uses a nonserialized cache. Removing it requires a coordinated future version bump.
- **Intentional:** native replay and `net::lockstep` remain tested substrate although no production transport/session is wired.
- **Intentional:** serialization discriminants and inert compatibility variants remain unless an explicit schema/replay rebaseline is approved.
- **Residual:** exact radar color/pixel parity beyond the verified dirty-producer pipeline.
- **Residual:** quickload mtime selection versus panel embedded-time ordering is preserved. Unifying it is `VERA-internal / gamemd equivalent UNCHECKED`.
- **Residual:** renderer-only building animation phase drift across save/load (`src/app/state.rs:470-476`).
- **Residual:** broad clippy/style backlog, active float audit, clone counts, and performance tuning not required by a frozen owner slice. Baseline clippy currently reports eight `approx_constant` errors and hundreds of warnings; architecture work must not become a mass lint rewrite.
- **Residual:** external consumers of public `render::tactical_compat` are unknown, so deletion is not authorized merely because in-repo production callers are absent.
- **Intentional:** `app_sidebar_text` and `render::tactical_draw_plan` are live; stale comments may be corrected, but the modules are not dead.
- **Intentional:** large `RuleSet`, resolved-terrain, snapshot, and hash files are split only when a real owner seam exists, not to satisfy a line-count target.

## Player-experience preservation ledger

Every slice must state which rows it can affect and how they were checked.

| Experience | Invariant |
|---|---|
| Launch/loading | First native loading frame is presented before deferred loader work; rules/mode/map layering and accepted RNG continuation are unchanged. |
| Commands | Same command bytes, issue/execute tick, house order, queue order, input delay, and replay record. |
| Simulation | Same phase order, lifecycle order, state hash, logical RNG cursors, and terminal result. |
| Terrain/overlays | Same tactical draw order, live overlay identity/frame, bridge state, passability, and restore result unless a commit cites verified parity evidence. |
| Fog/render | Same local owner and visibility facts; same current Rust render-after-advance relationship; cache rebuild cannot affect hash. No claim of exact native late-region placement is introduced. |
| Sidebar/input | Credits advance once per game frame; repeated hover/tooltip/render reads cannot advance state; hit testing and targeting remain unchanged. |
| Audio | Same output order, sound IDs, screen positions, channels, cooldowns, EVA/music gates, and teardown ordering. |
| Persistence | Failed load is side-effect free; successful load has identical validated sim hash/session and explicit diagnostic/admission handling. |
| Frontend | Same route transitions, map order/filtering, random-map retention, and return destination. |
| Headless | Same rules layers, entities/houses, launch facts, post-map pass, hash, and RNG as the GPU-free portion of app construction. |

## Validation and review protocol

### Frozen item-to-test matrix

Names beginning with `new:` are acceptance tests that must be added before or with the slice; the other names identify existing filters/fixtures to retain.

| Item | Required production-path checks |
|---|---|
| F01 | `new: failed_load_preserves_complete_running_match_transaction` covers bad bytes, content mismatch, missing template/registry/rules, restore validation, and map/cache restore failure; `new: quickload_and_panel_keep_explicit_latest_policies`; `validated_load_requires_exact_content_and_session_metadata`; snapshot round-trip/state-hash tests. |
| F02 | `gsi_16_01_local_scheduler_uses_move_bytes_and_fences_queued_move_metadata`; `recorded_scheduler_does_not_apply_offline_input_delay`; `synchronized_queue_roundtrip_preserves_bytes_order_and_dispatch_state`; `new: all_app_command_producers_use_identical_batch_ingress_order`; pending-queue snapshot round trip. |
| F03 | `new: sidebar_view_reads_are_pure`; `new: credits_advance_once_per_committed_ordinary_frame`; explicit no-admit/pause/menu/inactive/NetworkModal/exact-step gate cases; existing sidebar view/gadget/tooltip tests. |
| F04 | Rules parser/mission/sequence/locomotor tests; `new: rules_hash_and_enum_wire_values_survive_vocabulary_move`; `new: wall_overlay_first_match_order_survives_registry_move`; animation/effect timing and representative state-hash fixtures; art lookup equality from sim and render consumers. |
| F05 | Native direction-table exact-equality and fixed-math facing tests; resolved-terrain, bridge-theater, terrain, RMG randomize/build/pipeline tests; `new: util_has_no_upward_dependency`; `new: map_dependency_move_preserves_cell_index_passability_and_projection`; before/after MapGen cursor equality. |
| F06 | Cursor atlas, combat-light, sidebar view, map catalog, terrain-instance, and sidebar-text tests; compile-time proof of zero production presentation -> app imports; byte-equal `SpriteInstance` output for a representative viewport. |
| F07 | `new: current_rust_frame_call_order_is_preserved`; `new: loading_pump_starts_only_after_present`; app frame-output/state-hash fixtures; `new: runtime_always_uses_bound_navigation_and_resources`; no production caller can supply alternate path/rules/trigger inputs. |
| F08 | `gsi_04_09_render_handoff_replaces_regerminated_overlay_variant`; `new: overlay_render_index_preserves_source_dynamic_tombstone_and_restore_order`; bridge clear/repair and snapshot restore fixtures; minimap dirty-order test; terrain-template/live-state access guard. |
| F09 | `new: app_and_headless_gpu_free_construction_fingerprints_match` across rules/map/RNG/order/hash/catalog/post-map fields; `new: runtime_backed_replay_hashes_match_each_tick`; launch/bootstrap/post-map/RNG routing tests; first-loading-present ordering check. |
| F10 | Diagnostic replay JSON/flush-retry tests; `new: replay_segment_survives_failed_load_and_rotates_on_success_new_match_and_teardown`; `new: debug_toggle_updates_existing_and_future_entities`; `new: fog_view_cache_is_discarded_and_rebuilt_after_load_or_owner_change`; version-81 byte round trip; repeated cache builds leave `state_hash` unchanged. |
| F11 | `new: asset_manager_lease_returns_on_success_failure_and_cancel`; asset precedence/sticky-cache tests; `new: scenario_catalog_indices_cannot_drift`; route exclusivity/return tests; audio event-order/channel/cooldown/music-EVA/exit-teardown fixtures. |
| F12 | Owner construction/transition tests for each moved state cone; all affected app module tests; `new: app_state_contains_only_named_owners`; `new: no_root_app_modules_remain`. Mechanical moves must keep behavior-fixture outputs byte/hash equal. |
| F13 | Entity-store/LogicVector/lifecycle tests; snapshot round trip/version test; world-hash/RNG/global parity fixtures; `new: substrate_registration_and_removal_dispatch_is_order_identical`; before/after serialized bytes and per-tick hashes. |
| F14 | Source-level dependency guards and root-module guard; relocated integration tests still run; public compatibility retain/remove inventory asserted in one architecture test. |

For every coherent slice:

1. Check `Get-Process cargo,rustc` and wait if another session owns Cargo.
2. Builder implements only the current slice.
3. Run the smallest relevant `cargo test -p vera20k --lib <filter>` set; architecture-only file moves also run the affected module tests.
4. Capture literal test output and inspect `git diff --check` plus the exact diff.
5. Give a fresh read-only critic the requirement, source evidence, diff, and validation. The builder never grades itself.
6. Fix every correctness/architecture gap and repeat with a fresh critic until pass.
7. Commit the coherent, passing slice before starting the next one.

High-risk checkpoints add before/after hash and logical-RNG assertions. Snapshot/restore slices add round-trip and failure-atomicity tests. Dependency moves add source guardrails only after current violations are removed.

At ledger completion, run `cargo test -p vera20k --lib` exactly once, verify no Cargo/rustc process remains, verify the branch is clean, and produce the final owner map plus residual list.

## Approval record

- First independent architecture/parity review: **BLOCK**. It required exact native-vs-current render wording, a version-81 fog wire decision, feasible F01 sequencing/current selection policies, exact sidebar and overlay cadence/order, construction fingerprints, a per-item test matrix, the missing `rules -> map` edge, ArtRegistry ownership, runtime-backed headless/replay, and a closed final inventory.
- Second fresh adversarial review: **BLOCK** on two remaining omissions only: production `util -> sim` direction tables and ambiguous diagnostic replay ownership.
- Final correction review: **PASS**. F05 now moves/guards the pure direction tables under util; `MatchDiagnosticsState` now owns replay with explicit load/new-match/teardown lifecycle and tests.
- Autonomous coordinator approval: the recommended dependency-first approach is approved. Implementation is authorized only in frozen order, one coherent slice at a time, under the builder/focused-test/fresh-critic/commit protocol above.
