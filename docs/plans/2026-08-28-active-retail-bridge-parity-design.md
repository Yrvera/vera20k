# Active-Retail Bridge Parity Architecture Design

## Goal

Parity-close VERA20k's complete active-retail Yuri's Revenge bridge system, including GSI-04.12 high bridges, GSI-04.13 low/water bridges, GSI-04.14 destruction/repair/CABHUT behavior, GSI-04.15 TubeClass, and every active cross-system bridge consumer established by the frozen coverage map.

The result must preserve behavior that already matches, replace behavior that conflicts with active `gamemd.exe` or retail data, implement missing behavior, and leave no approximate, unchecked, missing, or residual bridge mechanism behind.

## Status and Scope Decision

**Revision 6 after the latest REVISE design-review verdict; pending a new fresh re-review. No Rust implementation is authorized by this document yet.**

Discovery is frozen by `docs/research/bridges/00-system-models/ACTIVE_RETAIL_BRIDGE_COVERAGE_REINVESTIGATION_GHIDRA_REPORT.md` at commit `50d0ef8a`. Ten successive read-only omission audits expanded the boundary to 27 mechanism rows and 31 explicit open questions; the tenth pass added nothing.

The fourth review's random-map blocker and the Revision-4/Revision-5 critics' launch, projection,
trace-schema and authored-constructor blockers are closed by
`docs/research/bridges/00-system-models/RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`
at commits `7fee6929`, `4a63fa15`, and `f1e6054b`. It proves that preview and gameplay are separate generation runs, that every
generator entry reconstructs MapGen from the stored map seed, and that RMG Techno construction
advances the caller's Scenario stream even for later-failed placement attempts. Preview uses the
process shell Scenario cursor; successful Start replaces Scenario/Main from the match seed before
the `.SED` reader invokes the generator again. The launch generator nests `Full_Init` before its
CABHUT/neutral-Techno attempts, so one match-seeded Scenario owner must span preload, Fill, the
ordered attempt trace, projection and Simulation. Generated low decks are already fully stamped
and never pass through fixed-map `OverlayClass::Mark`. It also proves the dormant RMG helper
exclusions and the fixed authored-Techno constructor order needed to preserve the shared cursor
before later bridge RNG consumers. No open or deferred lifecycle term remains.

The design treats those rows as a coverage map, not as implementation modules. It uses 21 narrower closure units so each builder owns one coherent transaction and every critic receives a bounded requirement, native evidence, diff, and literal validation output.

In scope:

- active stock, mode-active, and content-conditional retail YR bridge mechanisms;
- valid custom-map `[Tubes]` content accepted by the active executable;
- active retail random-map low bridge and CABHUT production;
- cross-system consumers in movement, combat, targeting, terrain/resource placement, triggers, presentation, and persistence;
- evidence-backed negative boundaries needed to keep TS-only, dormant, editor-only, or name-only mechanisms out.

Out of scope:

- OpenTS behavior without independent active-retail proof;
- TS `TrainBridgeSet`, Mech, DropPod, subterranean and inactive LaserFence/meteor branches;
- editor-only tube writing and map-editing UI;
- malformed sentinel-less tube crash fidelity;
- RMG waterfall shaping as bridge topology;
- WaterBridge TMP names, Golden Gate scenery names, refinery radio “bridge” terminology, and generic tag bookkeeping as physical bridge mechanisms.

An evidence-backed exclusion is part of closure. It prevents a later implementation from importing a plausible but non-retail OpenTS or asset-name behavior.

## Architecture Context

The repository already has the correct broad dependency direction, but bridge authority is distributed and several consumers reconstruct the wrong fact locally.

### Immutable and load-time inputs

- `src/map/theater.rs` owns theater bridge-piece inputs.
- `src/map/bridge_facts.rs` owns raw structural cell facts and anchor/ramp classifications.
- `src/map/resolved_terrain.rs` projects TMP, overlay, level and bridge facts into resolved cells.
- `src/map/tube_facts.rs` and `src/map/tubes.rs` own explicit tube data.
- `src/map/rmg/` owns separately seeded MapGen decisions and emits an ordered
  `RmgConstructionTrace` containing every CABHUT/neutral-Techno attempt, including discarded ones.
  The shell-preview or gameplay-load caller applies that trace to its own Scenario RNG.

These modules may feed simulation, but they must not depend on it. Low Road overlays, structural high-bridge flags, and TubeClass records remain separate inputs.

### Deterministic simulation owners

- `src/sim/bridge_state/` owns mutable bridge state, endpoint records and damage/repair projection.
- `src/sim/map/bridge_topology.rs` exposes simulation-facing cell bridge views.
- `src/sim/occupancy.rs` and entity `BridgeOccupancy` state own ground/deck lists and persistent `OnBridge` identity.
- `src/sim/pathfinding/` owns entry, A*, zones, hierarchy and post-A* smoothing.
- `src/sim/movement/` owns locomotor transitions, path markers, tubes, scatter and blocked-unit behavior.
- `src/sim/combat/` owns target acquisition, fire gates, AoE admission, selected-plane damage, Rocker admission and bridge-object tolerance.
- `src/sim/world/mod.rs::projectile_collides_at`, `src/sim/projectile.rs::projectile_bridge_crossing`, and the caller in `src/sim/world/techno_ai.rs` own the already-matching projectile bridge-plane collision path that unit 18 must preserve.
- `src/sim/rocking/` owns persistent rocking impulse/timer state; combat must supply its exact native enumeration and dispatch inputs rather than bypassing it with a presentation effect.
- `src/sim/world/bridge_orchestrator.rs` coordinates collapse, repair, CABHUT outputs and deterministic side effects.
- spawn, production, landing, teleport, trigger and terrain-resource owners remain in their existing domains and consume the same bridge facts.

No one Boolean is authority for all of these. Native behavior intentionally distinguishes raw structural `0x100`, transition `0x200`, destroyed/inactive `0x400`, orientation `0x800`, sparkle suppression `0x1000`, mutable state, effective cell height, walkability, persistent object `OnBridge`, ground/deck occupant lists, and TubeClass direction 8.

### Presentation and persistence owners

- `src/render/` and `src/app/presentation/` derive terrain, object split, shadow/railing, PixelFX, action-line, radar and audio commands from immutable simulation snapshots and map assets.
- snapshot/hash owners serialize authoritative deterministic state and rebuild derived topology after load.

Presentation must never repair missing simulation authority by inventing a second bridge model. Conversely, render-only rules such as `TooBigToFitUnderBridge` must not leak into movement legality.

### Current System Map fit

The System Map's `bridge-helpers` service already describes bridge topology/deck predicates feeding traversal, occupancy, area damage, visibility and drawing. This work extends verified connections across existing owners; it does not justify a new cross-layer service or a dependency reversal. System Map entries should change only when an implemented, verified connection changes the mapped graph.

## Design Forces

1. **Exact facts are plural.** Raw bits, mutable bridge state, effective height, object plane and occupant list cannot be collapsed safely.
2. **Native transactions cross modules.** Load stamps, movement relayers, collapse, repair and post-load rebuild must preserve their native order even though Rust owners are separated.
3. **RNG instances are authority.** Scenario, main and MapGen streams cannot be substituted for one another, and fixed draws such as DBRIS `RandomRanged(1,1)` still advance state.
4. **Content frequency does not erase active behavior.** Explicit tubes and campaign trigger actions remain required when the active loader/runtime accepts them.
5. **Presentation has exact raw-fact consumers.** PixelFX and action-line height use raw flags in states where generalized walkability/deck predicates may disagree.
6. **Existing correct behavior is an asset.** Bullet bridge-plane crossing, tactical inverse search, TIBTRE `0x500` refusal and other matches need preservation tests, not gratuitous rewrites.
7. **The closure process is part of correctness.** A builder's own tests cannot close a mechanism without a fresh read-only critic pass.

## Approaches Considered

### A. Distributed exact-delta closure in current owners — chosen

Keep raw inputs, mutable topology, occupancy, pathing, movement, combat, presentation and persistence in their present architectural domains. For each closure unit, establish an implementation contract, correct the smallest owner set needed for the native transaction, add exact positive and negative fixtures, and run the builder/critic loop before proceeding.

Cross-owner consistency comes from explicit facts and transaction inputs, not from moving all bridge code into one module. Shared queries expose raw structural state, mutable state, effective level and object plane separately. Coordinators may sequence a native transaction, but they do not become universal policy owners.

This approach minimizes architecture drift and makes preservation of already-correct paths explicit.

### B. Central `BridgeService` monolith

Move bridge loading, topology, movement, combat, rendering and persistence decisions behind one new service.

This looks attractive because it centralizes terminology, but it would reverse current dependency boundaries, mix immutable map data with mutable simulation and presentation, and encourage a single generalized “has bridge” answer where native needs multiple independent facts. It would also make unrelated movement/combat/render code depend on a subsystem-shaped facade rather than their established domain owners. Rejected.

### C. Literal OpenTS/native-class-shaped port

Recreate OpenTS/Westwood `MapClass`, `CellClass`, `FootClass`, `TubeClass` and bridge walkers as a parallel class hierarchy, then route Rust through it.

Readable inherited code would speed copying, but OpenTS mixes TS behavior, reconstructed choices and names that do not match active YR. A parallel hierarchy would duplicate current Rust state, create synchronization hazards, and import excluded `TrainBridgeSet`/legacy behavior. Rejected as both parity and architecture risk.

## Chosen Architecture

### 1. Preserve separate authoritative facts

The following concepts remain separately queryable and testable:

- raw cell flag word and named masks;
- tile index, subtile, level, overlay ID/data and anchor relation;
- mutable bridge state and endpoint/connection records;
- resolved low Road land/passability;
- explicit TubeClass identity, endpoints, direction path and length;
- persistent entity `OnBridge` state;
- ground/deck object lists and occupation bytes;
- derived zone/hierarchy/path-marker state;
- render-only bridge/depth decisions.

No closure unit may replace one with another unless active native evidence proves equivalence at that exact consumer.

### 2. Use existing domain owners with narrow shared views

`map` continues to own decoded/load-time facts. `sim::bridge_state` owns mutable bridge topology. Occupancy and movement own object-plane transitions. Pathfinding owns reachability and smoothing. Combat owns target/damage plane decisions. Presentation owns draw construction from immutable snapshots.

Where a consumer currently receives an impoverished Boolean, extend that consumer's input with the smallest explicit view it needs. Examples include effective level plus raw structural state for smoothing, selected occupant plane for Rocker, and raw `0x1000` rather than walkability for PixelFX. Do not add a generic omniscient bridge context passed everywhere.

### 3. Model native transaction boundaries explicitly

#### RMG generation transaction

1. On first setup entry only, if `MapSeed+0x74 == -1`, draw the seed from the process shell Scenario
   RNG. Keep Randomize and derived-option draws on Main RNG. Neither path uses MapGen.
2. Each Generate call begins with a fresh MapGen object seeded from `MapSeed+0x74`; it never starts
   from the prior preview continuation. The pure geometry job returns its map payload, MapGen
   continuation and an `RmgConstructionTrace` whose stable ordinals record every native constructor
   attempt in generator order. Each event contains phase (`BridgeRepairHut` or `NeutralTech`), type
   identity, and `Discarded` or `Emitted { entity_index, cell }` outcome. A discarded event carries
   no invented cell: active neutral-Technos are constructed before their placement loops and can
   fail without a native final cell. Successful output rows alone are not a sufficient reconstruction.
3. Preview replays the trace against `OfflineSkirmishRuntime`'s existing shell Scenario cursor.
   Every event consumes one raw word. Discarded events drop the low word; emitted events create a
   display-only binding for the stable entity index. The returned cursor replaces the shell cursor
   even if the candidate is later cancelled.
4. A second Generate replaces the candidate and resets MapGen from the current map seed while
   continuing the already-advanced shell Scenario cursor. Use Map with a valid preview performs no
   third generation. Common teardown writes `RandMap.img`; Cancel preserves the returned shell RNG
   and commits neither `.SED` nor sentinel, while accepted result `1` writes `RandMap.Sed`, rebuilds
   the chooser preview, and commits the ordinary sentinel selection.
5. The accepted preview map and its MapGen continuation are UI/file artifacts, never gameplay map
   authority. Successful Start creates exactly one gameplay `ScenarioBootstrapRng` in
   `LoadingRequest` from the match seed. `prepare_battle_start_plan` consumes that owner directly and
   retains resolved plan outcomes, not a separately seeded cursor. `MapLoadInitial` then carries the
   same owner into `load_map_from_initial`.
6. `.SED` reader success runs a second complete generator call from the stored map seed. Native
   launch enters `Full_Init` before the generator's bridge/CABHUT/neutral-Techno phases. Rust matches
   this nesting by advancing the one `ScenarioBootstrapRng` through house/start-plan work and terrain
   Fill first, then replaying the launch `RmgConstructionTrace`. `into_simulation` transfers that
   same cursor so Post-Map and gameplay consumers continue it; no match-seeded parallel cursor or
   cursor replacement is permitted.
7. Launch replay builds a `GeneratedTechnoInitTable` keyed by stable generated entity index. A
   successful event stores the consumed low word as `techno_ctor_random_word`; a discarded event has
   no binding. `MapLoadInitial` carries the completed table to projection.
   `spawn_from_map_with_resolved` validates entity index, type and cell identity, installs that word
   on `GameEntity`, and performs no second draw. Fixed authored-map Technos use prerequisite P0's
   direct constructor-draw path in native section/key/upgrade order. The field participates in
   deterministic snapshots and hashes.
8. Shell preview generation never creates runtime bridge records, zones, occupancy, or live
   simulation effects. Launch generation feeds the ordinary load transaction only after the native
   generation-time constructor sequence has been accounted for exactly.
9. Preserve `BuildRiverBridge @ 0x0059E740` as waterfall terrain shaping and prove by a negative
   characterization that it writes no runtime bridge overlay/flag topology.

The unit-2 implementation contract must preserve these interface roles (names may change only if
the critic can map an equally explicit owner one-for-one):

```rust
struct RmgConstructionTrace {
    events: Vec<RmgConstructionEvent>,
}

struct RmgConstructionEvent {
    ordinal: u32,
    phase: RmgConstructionPhase, // BridgeRepairHut | NeutralTech
    techno_type: TechnoTypeId,
    outcome: RmgConstructionOutcome, // Discarded | Emitted { entity_index, cell }
}

struct GeneratedTechnoInit {
    entity_index: usize,
    techno_type: TechnoTypeId,
    cell: CellCoord,
    techno_ctor_random_word: u16,
}
```

`LoadingRequest` owns `scenario_bootstrap_rng: ScenarioBootstrapRng`. `MapLoadInitial` carries the
generated-source identity, `RmgConstructionTrace`, and then the validated generated-init table.
`GameEntity` owns the installed constructor word. The stable entity index plus type/cell tuple makes
a stale or misordered binding a load error rather than silently spending another draw.

The two event phases are exhaustive for the active generator. `0x005A6510` and `0x005A82E0` are
reachable only from no-xref `0x005A5020`; `0x005A91E0` also has no caller, and none of the three
entry addresses occurs as an image function pointer. Unit 2 must preserve that evidence-backed
exclusion rather than widening the trace for dormant RMG-shaped code.

#### Scenario-load transaction

1. After a fixed map is selected, parse it through the normal scenario loader. For `.SED`, first read
   seed/options and run the launch generator inside the load path after the successful-Start RNG reset;
   do not substitute the accepted preview payload.
2. Apply high structural stamps and later overlay-data replacement in their verified section order.
3. On fixed authored maps only, run low endpoint/body `OverlayClass::Mark` expansion and its
   scenario-load RNG draws. On generated maps, preserve the generator's complete direct three-wide
   overlay/data rectangle and skip Mark entirely; the generated source flag is explicit rather than
   inferred from overlay ids. This branch follows `InitMapFromSyntheticINI @ 0x00599650` and direct
   deck writer `0x0058F2C0`: the successful `.SED` arm in `Read_Scenario @ 0x00684620` is exclusive
   with the ordinary INI/overlay-pack path, so it has no later Mark replay.
4. Parse explicit `[Tubes]` independently; classify automatic same-cell shells from final theater land data without synthesizing traversal.
5. Recalculate terrain, records, zones and hierarchy only at the verified load boundary established by units 1, 3, 4 and 5.

#### Path-planning transaction

1. Query entry using current/destination effective layers and structural facts.
2. Run A* with dual-layer visits, hierarchy, peer markers and direction-8 tube exits.
3. After success, run both native smoothing/straight-segment passes while carrying effective height and raw structural state.
4. Produce the final layered path before any entity relayer mutation.

#### Movement relayer transaction

1. Remove occupation/list membership using the old persistent plane.
2. Update location, height and `OnBridge` in the owning locomotor path.
3. Add occupation/list membership using the new plane.
4. Preserve explicit tube direction-8 progress/finalization separately.

Selected-layer scatter/crusher dispatch and stopped-blocked safety are distinct occupant-reaction transactions triggered from their verified callers; they are not appended to every successful relayer step.

#### Damage and collapse transaction

1. Preserve the four verified dispatcher paths, original impact-cell admission, strict Z window, native high/low family selection and RNG ownership.
2. Preserve deck/ground occupant-plane selection as an explicit input to ordinary splash, CellSpread tolerance and Rocker.
3. Let the unit-11 and unit-12 implementation contracts establish the still-open internal ordering among tolerance, ordinary damage, Rocker admission/dispatch and bridge state-machine admission; this design does not guess that order.
4. Once a bridge destroy primitive is entered, execute its BlowUpBridge fallout synchronously before the next destroyed cell.
5. Apply exact edge re-stamp, zones, events, radar/audio intents and DBRIS Anim/Bounce work in the contract-proved order.

#### Repair and CABHUT transaction

1. Preserve the outer Y-major discriminator and inner X-major first-hit scan distinction.
2. Choose overlay, bridge-record or no-overlay fallback geometry exactly.
3. Apply repair mutations, pavement/flood-fill/level restoration only in their verified branches.
4. Rebuild zones/topology and emit observers/tags, sound/EVA and radar effects in native order.
5. Keep C4 timer, attached-bomb and hut-death entries distinct even when they share collapse machinery.

#### Restore transaction

1. Deserialize authoritative cell/object/tube/hut/effect/RNG state.
2. Reconstruct raw/mutable bridge projections without retaining stale derived pointers.
3. Rebuild records, zones, hierarchy, radar dirties and render snapshots deterministically.
4. Validate that save/load does not change object plane, tube progress, pending debris or RNG continuation.

### 4. Keep content-conditional mechanisms installed but dormant without data

Explicit `[Tubes]`, bridge trigger actions/events and Psychic Sensor enemy action lines must be implemented in their normal owners. They consume no state and produce no behavior when the corresponding valid map/type content is absent. They must not be compiled out, replaced with stock-map prevalence assumptions, or merged into low Road behavior. Automatic same-cell TubeClass shells are a separate load-time mechanism: units 1 and 5 must classify their final-theater activation and preserve their zero-length, non-traversable boundary until native evidence proves an active consumer.

### 5. Carry evidence and closure state alongside work

Each closure unit progresses through:

```text
FROZEN-COVERAGE -> CONTRACTED -> BUILT+FOCUSED-VALIDATION
                -> CRITIC-N FINDING/FIX -> CRITIC-N+1 PASS -> CLOSED
```

Any new active caller reopens coverage. Any unresolved native term keeps the contract blocked. Any critic finding keeps the unit open. Passing a subset of tests never changes the owning GSI row to closed while another required mechanism is open.

### P0 — shared authored-Techno constructor-RNG prerequisite

Before RMG unit 2 or fixed-low-load unit 3 begins, one builder closes the smallest shared load
prerequisite needed by both. This is not a new bridge mechanism or a general Techno rewrite. It
models the one unconditional raw Scenario draw at `TechnoClass__Constructor @ 0x006F3254`, stores
the low word as `GameEntity::techno_ctor_random_word`, and preserves the active authored load order:

1. `[Units]`, then `[Aircraft]`, then `[Infantry]`, then `[Structures]`;
2. ascending INI key index within each section;
3. one word after valid house/type/allocation reaches the constructor, even when Unlimbo later
   fails; zero for malformed/unknown/pre-construction-rejected rows;
4. after a base structure successfully Unlimbos, one word for each selected non-`-1` authored
   upgrade constructor in the native declared-count/slot order.

Current `parse_map_entities` already preserves the four base section groups, but it must retain the
minimum structure-upgrade payload needed to reproduce constructor events. The projection funnel
receives an explicit source mode: `Authored` consumes/stores the next Scenario word at each native
constructor event; `Generated(GeneratedTechnoInitTable)` validates and installs the preconsumed
word for each emitted entity and consumes zero. Later placement failure never rolls the cursor back.

P0 gets its own implementation contract, focused `--lib` validation, commit, and fresh-critic loop.
Its production fixture interleaves one valid object in every section, a constructed-then-failed
Unlimbo, a rejected-before-construction row, and a structure upgrade, and asserts exact word binding
plus the final cursor. A paired generated fixture asserts zero projection draws. P0 remains open if
any active constructor event, section/key order, upgrade event, or cursor transfer is approximate.

## Closure Units and Dependency Order

P0 plus the 21 bridge units below are implementation boundaries, not new architecture layers.

| Order | Closure unit | Primary coverage | Dependency / ordinary-play oracle |
|---|---|---|---|
| 1 | Theater/rules/assets, raw flags, automatic-shell theater classification, TIBTRE mask preservation | BR-M01, BR-M06, BR-M24 | exact ten piece keys; raw-mask fixtures; automatic-shell corpus verdict; TIBTRE rejects `0x500`; retain raw SpecialFlags/session inputs for unit 10 |
| 2 | Active RMG preview/accept/`.SED` launch lifecycle, low deck/end/CABHUT production, and waterfall-topology exclusion | BR-M02, BR-M03 | P0 and unit 1; `7fee6929`, `4a63fa15`, `f1e6054b`; fresh MapGen per run; first-entry/re-entry and no-preview gates; location-free discarded trace events; one launch `ScenarioBootstrapRng`; validated `GeneratedTechnoInitTable`; active-phase-only trace; complete stamped output; no generated Mark replay; `BuildRiverBridge` negative characterization |
| 3 | Fixed-map low overlay procedural load and Road mutation | BR-M05, BR-M11 | P0 and unit 1; Lost Lake/Killer plus destroyed low fixture; generated-source bypass preserving its full direct deck payload and zero Mark draws |
| 4 | High topology, records, zones, hierarchy and edge restamp | BR-M04, BR-M10, BR-M17 | unit 1; Bay of Pigs/Hills and Deadman's Ridge |
| 5 | Explicit TubeClass load/hierarchy/direction-8/persistence and automatic-shell non-traversal | BR-M12, part of BR-M22 | units 1/4; sealed valid custom tube fixture; zero-length shell negative case |
| 6 | Dual occupancy, entry, A*, peer markers and locomotor transitions | BR-M07, BR-M09, BR-M13 | units 4/5; deck, under-span, ramp, gap |
| 7 | Two native post-A* smoothing passes | BR-M25 | unit 6; no wrong-plane shortcut |
| 8 | Selected-layer scatter/crusher and stuck safety | BR-M23 | unit 6; ten-object order and bridge exemption |
| 9 | Spawn, Unlimbo, landing, paradrop, teleport and relayers | BR-M08, part of BR-M13/BR-M21 | unit 6; correct list/height after placement |
| 10 | Impact admission, destruction-authority matrix, Z/family gates and four-path RNG | BR-M16 | units 1/3/4; campaign/editor SpecialFlags vs session option authority, CombatDamage non-owner, CABHUT bypass, strict impact boundary and negative family |
| 11 | CellSpread bridge-object tolerance | BR-M27 | unit 10; V3WH/V3EWH/DMISLWH plane fixtures |
| 12 | AoE Rocker admission, enumeration, selected-plane dispatch and exact impulse/timer state | BR-M26 | units 10/11; native numeric attenuation/timer/order oracle plus Rocker-negative case |
| 13 | High/low ladders, setters and direct walkers | BR-M17 | units 3/4/10; exact mutation/state trace |
| 14 | Synchronous fallout and DBRIS Anim/Bounce | BR-M15, BR-M18 | units 12/13; object order, RNG, bounce damage |
| 15 | Engineer repair and no-overlay restoration | BR-M19 | units 3/4/13; scan order and terrain/height tail |
| 16 | CABHUT cursor, timer, attached bomb and fallback collapse | BR-M19 | units 13/15; each entry and negative gate |
| 17 | Repair observers/tags and multi-engineer order | BR-M18, BR-M19 | units 15/16; event/output ordering |
| 18 | Projectile, fire, superweapon and remaining effect-plane consumers | BR-M14, parts of BR-M15/BR-M16/BR-M21 | units 6/10; preserve Bullet crossing and close gates |
| 19 | Tactical inverse, acquisition, Mirage, placement, orders and triggers | BR-M21 | units 4/6/18; deck/under-span interaction fixtures |
| 20 | Render, split, shadow/railing, PixelFX, action lines, radar/audio | BR-M20 | all sim facts stable; frame/order and raw-bit fixtures |
| 21 | Save/load/checksum/rebuild and deterministic projection | BR-M22 | all authoritative state stable; continuation oracle |

### Open-question routing

Every frozen question has a pre-implementation owner. A unit cannot become `CONTRACTED` until all questions routed to it are resolved or proved inapplicable by active-retail evidence.

| Open questions | Owning closure unit(s) |
|---|---|
| OQ-01 complete consumer census | every unit as it closes; final bridge-wide reverse audit is the zero-add owner |
| OQ-02 runtime constants | 6, 9, 14, 20 |
| OQ-03 automatic tubes; OQ-21 activity label | 1 for theater-corpus classification; 5 for shell identity/non-traversal |
| OQ-04 RMG low placer; OQ-22 waterfall ownership | 2 |
| OQ-05 low overlay Mark RNG | 3 |
| OQ-06 traversal call placement; OQ-07 A* ties/hierarchy; OQ-09 peer markers | 6 |
| OQ-08 tube hierarchy pairs | 5 |
| OQ-10 `ShouldBeOnBridge` reachability/event consumers | 6 and 19 |
| OQ-11 locomotor reachability | 6 and 9 |
| OQ-12 teleport/landing state | 9 |
| OQ-13 projectile/effect families | 14 and 18 |
| OQ-14 edge restamp | 4 and 13 |
| OQ-15 collapse debris | 14 |
| OQ-16 repair selection/restoration | 15, 16 and 17 |
| OQ-17 rendering assets | 1 for asset identity; 20 for presentation binding |
| OQ-18 cross-system consumers | 9, 18 and 19 |
| OQ-19 persistence | 5 and 21 |
| OQ-20 retail fixtures | each owning unit; final reverse audit owns the complete matrix |
| OQ-23 low repair tail | 15 |
| OQ-24 low bridge/TubeClass split | 3 and 5 |
| OQ-25 scatter/crusher/stuck dispatch | 8 |
| OQ-26 target acquisition layer gate | 19 |
| OQ-27 raw-mask terrain/resource consumers | 1 and final reverse audit |
| OQ-28 action-line endpoint height | 20 |
| OQ-29 post-A* smoothing | 7 |
| OQ-30 Rocker secondary effect | 12 |
| OQ-31 CellSpread tolerance | 11 |

The fourth-review prerequisite concerning preview/cancel/accept/`.SED` dual-RNG ownership and the
Revision-4 critic findings concerning one launch cursor, generated-object bindings and generated
overlay replay, plus Revision-5 findings concerning discarded-event shape, dormant helper exclusion
and authored constructor ownership, are not left as new open questions. They are resolved before
implementation by the exhaustive lifecycle report at `7fee6929`, `4a63fa15`, and `f1e6054b`.
They are mandatory P0/unit-2/unit-3 contract inputs.

### Evidence-backed exclusion routing

Negative facts are closure requirements, not prose-only cautions. Each routed unit must include the applicable do-not-do test or source assertion in its implementation contract, and its critic must check that the excluded behavior was not introduced.

| Frozen negative fact(s) | Owning closure unit(s) |
|---|---|
| TS Mech, DropPod and subterranean bindings; inactive meteor and LaserFence branches | 5, 6, 9, 14 and 18 as applicable to the inherited lead |
| `TooBigToFitUnderBridge` is render-only, never movement/passability | 6 and 20 |
| automatic same-cell tubes are never joined or given direction-8 movement; low Road never receives high/tube state | 3, 5 and 6 |
| `BuildRiverBridge` waterfall shaping and unreferenced type-3/4 block are not runtime bridge topology | 2 |
| `TrainBridgeSet` is TS-only; `RAILBRDG` remains render data rather than train topology | 1, 4 and 20 |
| shore/water/RMG helper labels, WaterBridge TMP names, refinery radio `0x16`, Golden Gate scenery names and generic TagType registry names are not physical bridge mechanisms | 1, 2, 3, 17 and 19 according to the real owner |
| active YR has no proactive bridge-specific AI attack/repair/destroy opcode, CABHUT priority or special route policy; ordinary targeting/orders/pathing remain active | 6, 16 and 19 |
| no continuous per-cell bridge HP; Scenario/session SpecialFlags authority is not `[CombatDamage]`; weapon-damage authority does not gate CABHUT/attached-bomb collapse | 1, 10, 13 and 16 |
| CABHUT immunity, ownership/capture and `MultiEngineer` do not suppress verified collapse/repair transactions; hut placement does not repair automatically | 15, 16 and 17 |
| collapse force-damages ground occupants and DropIns deck occupants; it does not force-kill deck occupants; `BridgeVoxelMax` does not gate metallic debris | 14 |
| stale `UnregisterBridgeRepairHut` and `RecalcBridgeShroudFlags` names do not create hut-registry or per-layer-shroud mechanisms | 17, 19 and 20 |
| no dedicated bridge network packet; deterministic simulation/RNG carries results | 21 |
| map-editor fallout suppression is not an ordinary gameplay branch | 14 |
| malformed sentinel-less tube crashes may be rejected safely; editor-only tube/placement writing UI remains excluded while runtime placement stays active | 5 and 19 |
| no per-layer shroud model; cold BSS zero never proves a runtime constant dormant | every affected contract, with units 6, 14, 19 and 20 as the known consumers |
| accepted RMG preview data/continuation is never gameplay authority; preview Scenario state is never carried through Start; successful output objects cannot stand in for failed constructor events; generated low-deck cells never receive a fixed-map Mark replay | 2, 3 and 21 |
| no-xref RMG-shaped `0x005A6510`/`0x005A82E0`/`0x005A91E0` are not active generator phases; discarded neutral-Techno events do not invent a final cell | 2 |
| fixed authored Techno construction is not RNG-free; generated projection is not allowed to draw the already-consumed word again | P0 and 2 |

## Builder and Critic Protocol

For every closure unit:

1. Resolve its open questions against live `gamemd.exe` and retail data into a sourced implementation contract. OpenTS may locate functions but supplies no required behavior.
2. Assign one builder. The builder may preserve correct code, replace wrong code, and implement missing code only within that unit and its smallest verified prerequisite.
3. Check `cargo`/`rustc` ownership before validation. Run focused `cargo test -p vera20k --lib <filter>` commands only; never a bare Cargo test.
4. Commit the coherent evidence-backed slice after focused validation.
5. Give a fresh read-only critic who did not build the unit its requirement, native/retail evidence, exact diff, and literal validation output.
6. If it fails, fix the largest finding, commit the correction, and give the full updated bundle to a new critic. The new critic must recheck prior findings as well as the new diff.
7. Repeat until a fresh critic passes with no material finding. Approximate or unverified behavior cannot be relabeled residual; the unit and owning rows stay open.

Critics do not edit. Builders do not self-approve. A critic pass proves only the bounded unit, not the bridge system.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — ordinary units must select and remain on the correct deck/ground plane through entry, A*, smoothing, locomotion and occupancy. Trigger: every high-bridge crossing. Player effect: refused routes, wrong-layer shortcuts, overlap or units falling between layers. Frequency: common on high-bridge maps. [BR-M07, M09, M13, M23, M25]
- `MILESTONE-BLOCKING` — low bridges must remain flat Road overlays and mutate exactly through intact/damaged/destroyed/repair states. Trigger: every low crossing and bridge damage. Player effect: wrong movement class, impassable water or invented tunnel behavior. Frequency: common on stock low-bridge maps. [BR-M05, M11, M17, M19]
- `MILESTONE-BLOCKING` — collapse and repair must preserve native per-cell transaction/RNG order. Trigger: bridge weapon damage, CABHUT C4, attached bombs or engineer repair. Player effect: different survivors, debris damage, bridge shape, zones and events. Frequency: common whenever bridges are contested. [BR-M16..M19, M26, M27]
- `MILESTONE-BLOCKING` — bridge destruction authority must follow the active mode/source matrix: scenario `[SpecialFlags]` where authoritative, skirmish/multiplayer session `BridgeDestruction` where authoritative, never `[CombatDamage] DestroyableBridges`; CABHUT C4/attached bombs bypass the weapon gate. Trigger: every attempted weapon or hut-driven collapse when sources disagree. Player effect: bridges become wrongly indestructible/destructible or hut sabotage stops working. Frequency: every configured disagreement and every CABHUT collapse. [BR-M01, BR-M16, BR-M19]
- `MILESTONE-BLOCKING` — authored Techno construction must advance the one Scenario cursor before later bridge load/gameplay consumers. Trigger: each valid authored Unit/Aircraft/Infantry/Structure or structure-upgrade constructor, including a later failed Unlimbo. Player effect: low-Mark variants and all later bridge damage/debris/repair randomness diverge. Frequency: essentially every stock fixed map with authored Technos. [P0 prerequisite]
- `MILESTONE-BLOCKING` — active RMG must emit traversable low decks and CABHUTs while preserving the two-run lifecycle and all three RNG owners. Trigger: every preview and every launch of a retail random map; deck production itself is active on types 3/4. Player effect: generated water regions lack intended connections, fixed-map Mark corrupts already-stamped generated decks, accepted maps differ from fresh `.SED` launches, or split/double constructor draws shift all later Scenario randomness. Frequency: every random-map session, with bridge placement on every qualifying generated map. [BR-M02]
- `COMPOUNDING` — exact topology/zone/hierarchy rebuild feeds all later path decisions. Trigger: load, collapse, repair and restore. Player effect: long-lived or post-load path divergence. Frequency: every topology mutation. [BR-M04, M10, M22]
- `MILESTONE-BLOCKING` — spawn, Unlimbo, factory exit, paradrop, landing and teleport must initialize the correct object plane, list, occupation bytes and height. Trigger: every bridge-adjacent creation or relayer. Player effect: a unit appears under the intended deck, occupies both planes, blocks the wrong route or renders at the wrong Z. Frequency: intermittent on bridge maps and common for factories/air insertion placed nearby. [BR-M08]
- `MILESTONE-BLOCKING` — direct target acquisition and weapon-specific gates must distinguish deck from under-span objects. Trigger: ordinary scans and attacks near high bridges. Player effect: units acquire or fire at invalid cross-plane targets. Frequency: common in combat near high bridges. [BR-M14, M16, M21]
- `COMPOUNDING` — scatter/crusher and stuck logic must use the selected plane. Trigger: moving blockers, crushing and stopped invalid occupancy. Player effect: wrong units scatter, teleport one cell, or self-damage. Frequency: intermittent but repeatable in traffic. [BR-M23]
- `COMPOUNDING` — projectile/Bounce collision and DBRIS fallout must use the bridge plane. Trigger: shots and collapse debris. Player effect: projectiles pass through decks or debris damages the wrong layer. Frequency: frequent during bridge combat/collapse. [BR-M14, M15, M18]
- `COMPOUNDING` — render consumers must use their exact raw facts and ordering. Trigger: every frame near bridges, ore/water sparkles, selected paths or Psychic Sensor lines. Player effect: occlusion seams, wrong rails/shadows, sparkling under decks or floating action lines. Frequency: continuously visible where applicable. [BR-M20]
- `PRESERVATION-BLOCKING` — TIBTRE-driven ore placement must continue rejecting raw structural/destroyed mask `0x500` rather than generalized walkability. Trigger: each `TIBTRE01..03` spread attempt near a bridge. Player effect: ore appears on structural or destroyed bridge cells and changes economy/pathing. Frequency: periodic on maps containing the stock spawning terrain. [BR-M24]
- `CONTENT-CONDITIONAL-BLOCKING` — valid explicit tubes must load, connect hierarchy, move and persist exactly. Trigger: a valid custom map with `[Tubes]`. Player effect: unusable or nondeterministic tunnel routes. Frequency: zero in scanned shipped maps, guaranteed when authored. [BR-M12, M22]
- `CONTENT-CONDITIONAL-BLOCKING` — tagged bridge collapse events/actions must reach only the verified footprint. Trigger: authored campaign/custom trigger content. Player effect: scripts fail or unrelated tags fire. Frequency: map-dependent. [BR-M18, M21]
- `COMPOUNDING` — snapshot/restore must retain authoritative state and rebuild derived state. Trigger: every save/load. Player effect: plane, path, debris or RNG divergence immediately or later. Frequency: every restored bridge-bearing game. [BR-M22]
- `RESOLVED-EXCLUSION` — TS-only, dormant, editor-only and name-only mechanisms remain excluded. Trigger: future code searches or OpenTS comparison. Player effect if violated: invented behavior and architecture drift. Frequency: development-time risk rather than retail runtime. [negative-fact ledger]

## Determinism and Persistence

- Scenario, Main and MapGen RNG streams remain distinct. Each closure contract names the stream and exact draw order.
- Every RMG call reconstructs MapGen from the stored map seed. Preview consumes and returns the
  process shell Scenario cursor; successful Start discards that cursor by constructing the gameplay
  Scenario/Main streams from the match seed before `.SED` regeneration.
- `LoadingRequest` owns one match-seeded `ScenarioBootstrapRng`. Battle-start preloading consumes it
  in place; `MapLoadInitial` moves it through terrain Fill and ordered RMG construction replay;
  `into_simulation` transfers it without reseeding, replacement, or a parallel `SimRng`.
- Generation-time constructor events are authoritative even when the attempted object is later
  deleted. `RmgConstructionTrace` records all attempts. `GeneratedTechnoInitTable` binds the one
  consumed low word for each emitted entity, and validated projection installs
  `GameEntity::techno_ctor_random_word` without spending the same Scenario draw twice.
- P0 gives fixed authored-map constructors the same field from a direct draw in native
  unit/aircraft/infantry/structure/upgrade order. It consumes before Unlimbo and therefore retains
  the cursor advance when placement later fails.
- Generated low-deck overlay/data rectangles are final load payloads. Only fixed authored low
  endpoints run `OverlayClass::Mark` and its Scenario draws.
- Fixed-range draws still advance the verified RNG stream.
- Linked-list/vector traversal order is retained where it controls outcomes: collapse fallout, scatter snapshots, repair selection and observer delivery.
- Presentation consumes no gameplay RNG and cannot mutate bridge state.
- New deterministic runtime state must be included in snapshots/hash only when native-active behavior persists across ticks. Derived topology is rebuilt at the verified restore boundary.
- Snapshot schema changes are isolated to the closure unit that introduces authoritative persistent state; old-version rejection and deterministic round trips are tested there.

## Validation Strategy

During closure units, use only focused `--lib` tests after confirming no other session owns Cargo. Favor native-trace tables and small retail fixtures over broad certification matrices.

Required fixture families:

- P0 authored-constructor fixture with one valid Unit, Aircraft, Infantry, Structure, a
  constructed-then-failed placement, a pre-construction rejection, and a structure upgrade;
  assert exact per-object words and final Scenario cursor in native section/key/upgrade order, then
  assert a generated projection with validated bindings spends zero additional draws;
- Lost Lake and Killer: intact low crossings;
- Bay of Pigs and Hills: high deck, under-span, dual-plane and AttackMove;
- Deadman's Ridge: high collapse gap;
- Shrapnel Mountain: destroyed low bridge;
- deterministic RMG type 3/4 preview/cancel/accept/launch sequence asserting fresh MapGen state on
  each run; first setup entry with seed `-1` taking one shell seed draw and re-entry taking none;
  continuing shell Scenario cursor across repeated previews and Cancel; no third Generate on Use Map
  with a preview and exactly one generation on Use Map without one; `.img` versus `.SED` commit gates;
  successful-Start Scenario/Main reseed; unconditional `.SED` regeneration; one launch cursor through
  plan preload, Fill, ordered construction replay, projection, Post-Map and Simulation; complete
  constructor-event order including failures; failed-event consume/no-binding and emitted-event
  consume-once/bind/no-double-draw rules; stored `Techno+0x3C8` value per CABHUT; final
  MapGen/gameplay-Scenario continuations; and generated low-deck projection preserving every direct
  overlay/data cell with no fixed-map Mark call or Scenario draw;
- `BuildRiverBridge` negative fixture proving waterfall shaping emits no structural/low-overlay bridge topology;
- sealed valid custom `[Tubes]` map;
- automatic same-cell TubeClass shell classification and zero-length non-traversal case;
- spawn/Unlimbo/factory/landing/teleport plane/list/height cases, including a refused-placement negative path;
- CABHUT collapse/repair and tagged event/action cases;
- destruction-authority matrix covering campaign/editor map `[SpecialFlags]`, skirmish/multiplayer session `BridgeDestruction`, conflicting `[CombatDamage] DestroyableBridges`, weapon admission, and CABHUT/attached-bomb bypass;
- DBRIS bounce/landing damage;
- V3WH/V3EWH/DMISLWH AoE tolerance cases plus native-derived Rocker admission, 7x7 enumeration, numeric attenuation, timer jitter, selected-plane and dispatch-order oracles;
- TIBTRE ore placement and raw-mask negative cases;
- selected and Psychic Sensor action lines;
- snapshot continuation across active movement, collapse debris and repair.

The full suite `cargo test -p vera20k --lib` runs exactly once after P0 and all 21 bridge units and
their critic cycles pass, immediately before the bridge-wide reverse audit is declared ready. It is
not rerun per unit.

## Bridge-Wide Reverse Audit

After P0 and all bridge-unit passes:

1. Start from each active native writer/consumer and prove a Rust owner, exact test or evidence-backed exclusion.
2. Start from every Rust bridge field, helper, ignored test, approximation marker and branch and prove current active-retail authority or remove/correct it.
3. Re-run the OpenTS correspondence ledger as leads and confirm no active YR mechanism disappeared between unit boundaries.
4. Recheck all 27 mechanism rows, all 31 open questions, and every entry in the complete frozen negative-fact ledger; every open item must be resolved and every exclusion preserved, not deferred.
5. Re-run named retail fixture traces for load, move, target, damage, collapse, repair, render and restore.
6. Run the full `--lib` suite once, record literal output, update only verified System Map connections, and produce the final handoff.

Any omission or regression reopens its owning unit and requires the builder/fresh-critic loop again.

## Adversarial Design Questions

### Why not close the four GSI rows independently?

Because active consumers cross their nominal ownership. High topology feeds combat, presentation and save/load; low collapse feeds ordinary Road pathing; TubeClass feeds hierarchy and persistence; CABHUT transactions feed events, audio and repair. Independent row closure would recreate the omissions found by the ten-pass coverage audit.

### What could make ordinary skirmish still feel wrong after many green unit tests?

Wrong-plane movement smoothing, target acquisition, synchronous collapse fallout, topology rebuild and render ordering. These are common, cross-owner paths whose effects compound. The design gives each a named closure unit and then requires a final end-to-end reverse audit.

### What decision would cause the most expensive rework later?

Collapsing raw structural state, mutable walkability and object `OnBridge` into one generalized predicate. It would infect pathing, damage, PixelFX, action lines and persistence with subtly different semantics. The chosen design keeps them explicit at the start.

### Could the critic process itself become ceremonial?

Yes, if critics receive only a summary or builder-selected tests. The required bundle includes the complete requirement, native evidence, exact diff and literal output; critics are read-only, did not build the unit, and must recheck prior fixes. A pass cannot waive an open evidence term.

### Could dependency sequencing hide a prerequisite expansion?

Yes. A builder may promote only the smallest missing prerequisite necessary for the current unit, must record it in that unit's contract, and cannot start the prerequisite's wider backlog. Cross-unit discoveries reopen the coverage map instead of being absorbed silently.

## Approval Gate

The preferred approach is distributed exact-delta closure in current owners. It is approved only after a separate design-review pass verifies:

- every behavior claim is cited to the frozen native/retail coverage report, the closed dual-RNG
  lifecycle/follow-up report at `7fee6929`, `4a63fa15`, and `f1e6054b`, or explicitly open;
- all 27 mechanisms map to a closure unit;
- all 31 open questions route to a pre-implementation owner and final audit;
- no chosen interface collapses distinct native facts;
- the player-experience ledger covers ordinary high/low traversal, combat, collapse/repair, RMG, presentation, content-conditional tubes/triggers and restore;
- the critic protocol cannot close a unit with unresolved evidence.
- P0 has a bounded authored-constructor requirement and its own builder/fresh-critic cycle before
  unit 2 or unit 3 can rely on the shared Scenario cursor.

Until that review passes, implementation remains blocked by design rather than by user authority.

## Sources

- `docs/research/bridges/00-system-models/ACTIVE_RETAIL_BRIDGE_COVERAGE_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/00-system-models/RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`
- active `gamemd.exe` addresses and retail inputs enumerated by that report
- current Rust owners at `origin/main` snapshot `0a6e6742`
- `docs/system-map/topology.v2.json` `bridge-helpers` service boundary
- `C:\Users\enok\Documents\OpenTS` correspondence ledger, as navigation leads only
