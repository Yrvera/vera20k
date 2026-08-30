# Active-Retail Bridge Parity Architecture Design

## Goal

Parity-close VERA20k's complete active-retail Yuri's Revenge bridge system, including GSI-04.12 high bridges, GSI-04.13 low/water bridges, GSI-04.14 destruction/repair/CABHUT behavior, GSI-04.15 TubeClass, and every active cross-system bridge consumer established by the frozen coverage map.

The result must preserve behavior that already matches, replace behavior that conflicts with active `gamemd.exe` or retail data, implement missing behavior, and leave no approximate, unchecked, missing, or residual bridge mechanism behind.

## Status and Scope Decision

**Revision 13 after the fourth 2026-08-30 REVISE design-review verdict; pending a new fresh re-review. No further Rust implementation is authorized by this document until that review passes.**

Discovery is frozen by `docs/research/bridges/00-system-models/ACTIVE_RETAIL_BRIDGE_COVERAGE_REINVESTIGATION_GHIDRA_REPORT.md` at commit `50d0ef8a`. Ten successive read-only omission audits expanded the boundary to 27 mechanism rows and 31 explicit open questions; the tenth pass added nothing.

The fourth review's random-map blocker and the Revision-4/Revision-5 critics' launch, projection,
trace-schema and authored-constructor blockers are closed by
`docs/research/bridges/00-system-models/RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`
at commits `7fee6929`, `4a63fa15`, `f1e6054b`, and `a776f270`. It proves that preview and gameplay
are separate generation runs, that every generator entry reconstructs MapGen from the stored map
seed, and that RMG Techno construction advances the caller's Scenario stream even for later-failed
placement attempts. Preview uses the process shell Scenario cursor; successful Start replaces
Scenario/Main from the match seed before the `.SED` reader invokes the generator again. The launch
generator nests `Full_Init` before its CABHUT/neutral-Techno attempts, so one match-seeded Scenario
owner must span preload, Fill, the ordered attempt trace, projection and Simulation. Generated low
decks are already fully stamped and never pass through fixed-map `OverlayClass::Mark`. It also
proves the dormant RMG helper exclusions, the fixed authored-Techno constructor order, the active
Post-Map starting-force paths, and the class-level constructor invariant that covers every runtime
Techno creation ingress. The stored constructor word is later read for report-sound selection, so
it needs a persistent owner. Those generator-phase findings remain closed; the separately discovered
pre-Fill House/Gather prefix and non-offline Mark-entry contexts are routed below.

The design treats those rows as a coverage map, not as implementation modules. It uses 21 narrower
implementation transactions to preserve dependency order, but the 27 `BR-M` rows are the closure,
builder, critic, and publication gates required by this project. A transaction may supply part of
more than one mechanism, and a mechanism may span more than one transaction; neither fact permits a
mechanism to inherit another row's pass.

Current implementation baseline is `origin/main` commit `a3e4ce9a` (2026-08-30), not the pre-implementation
snapshot used by Revision 7. PR #170 has merged P0, transaction 1 (load inputs/raw facts), and
transaction 2 (RMG low-bridge launch construction), including their focused validation and critic
corrections. Later merged work also changed ground-height ownership, Drive/Ship slope handling,
radar-overlay projection, FNPC bridge projection, and waypoint handling. Those changes are evidence
to preserve, not proof that any remaining `BR-M` row is closed. Before each remaining mechanism is
contracted, its owner must run a fresh direct disparity scan against then-current `main`; stale Rust
gap descriptions in the frozen coverage report are historical hypotheses only.

The completed P0-R1 reinvestigation disproves Revision 9's eligible/ineligible split. Current `main`
does not reproduce the active stock-offline prefix even when a `PreloadedBattleStartPlan` exists:
native runs a first disposable House construction pass, both active mode Gather callbacks, a zero-draw
House/type reset, and a second final House construction pass before terrain Fill. P0-R1 must replace
the optional Battle-only plan with the universal stock-offline transaction below and pass an
independent builder/critic cycle before transaction 3. The follow-up all-context audit now proves
the remaining boundary: campaign uses a single `[Houses]`-driven pass; LAN and WOL retain both House
passes plus common `+0x80`, with LAN using selected `+0x84` and WOL state `2` using common
`AssignStartingPoints` as the second Gather/chooser; replay inherits its recorded campaign/
noncampaign family; stream restore and generated `.SED` do not run Mark; and shipped
`gamemd.exe` has no persistent editor load mode. Transaction 3 must use this typed matrix rather
than allowing any non-offline context to inherit the stock-offline plan by analogy.

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
   authority. Successful Start has exactly one *logical* gameplay Scenario stream beginning at the
   match seed. Before terrain Fill, every active stock offline noncampaign mode performs the complete
   two-House-pass/two-Gather transaction defined below. Rust may partially evaluate that transaction
   into one immutable `PreFillScenarioPrefixPlan`, including default-cell deficient-start retries,
   only under the full-state equivalence proof below. `LoadingRequest` retains the consumed-once plan
   and `MapLoadInitial` constructs the sole downstream `ScenarioBootstrapRng` from the same match
   seed before adopting its validated continuation.
6. `.SED` reader success runs a second complete generator call from the stored map seed. Native
   launch enters `Full_Init` before the generator's bridge/CABHUT/neutral-Techno phases. Rust matches
   this nesting by continuing the one logical Scenario stream through the complete pre-Fill prefix, terrain
   Fill, and then the launch `RmgConstructionTrace`. `into_simulation` transfers that cursor so
   Post-Map and gameplay consumers continue it. No independently seeded parallel authority,
   unchecked cursor substitution, or second downstream continuation is permitted.
7. Launch replay builds a `GeneratedTechnoInitTable` keyed by stable generated entity index. A
   successful event stores the consumed low word as `techno_ctor_random_word`; a discarded event has
   no binding. `MapLoadInitial` carries the completed table to projection.
   `spawn_from_map_with_resolved` validates entity index, type and cell identity, installs that word
   on `GameEntity`, and performs no second draw. Fixed authored-map, Post-Map, and runtime Technos
   use prerequisite P0's fresh constructor-draw path; snapshot restore reinstalls the serialized
   word without drawing. The field participates in deterministic snapshots and hashes.
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

`LoadingRequest` owns exactly one consumed-once `PreFillScenarioPrefixPlan` for every valid active
stock offline noncampaign launch. There is no valid no-plan fallthrough and no Battle/FFA
terrain-independence eligibility gate. The plan is a partial evaluation of this exact native
transaction on the one match-seeded Scenario stream:

```text
S0
  -> House pass 1: H * RandomRanged(450, 1800)
  -> selected-mode vtable +0x80 Gather/preassignment
  -> selected-mode vtable +0x84 Gather/assignment/chooser
  -> rules/type reset and deletion of every pass-1 House (zero draws)
  -> House pass 2: H * RandomRanged(450, 1800)
S1
  -> Fill_In_Data
```

`H` is the same ordered set in both passes: every human node including observers, every valid AI
slot, then Neutral and Special. Stock mode IDs `1, 2, 4, 5, 6, 7, 8, 9` use the Battle family and
ID `3` uses Cooperative; both families execute the `+0x80` and `+0x84` Gather calls. Each deficient
Gather retry consumes exactly two ranged Scenario draws, Y then X, with no retry cap. It evaluates
the already-resized but otherwise default `CellClass` state: clear cells, overlay `-1`, level `0`,
and no occupier, before Fill, Iso, overlays, Terrain, or Technos. A narrow pre-Fill view may therefore
derive bounds from the verified Size/LocalSize inputs and answer only those default-cell predicates;
it must not consult later resolved terrain, overlays, occupancy, or bridge state. Sparse preassigned
start entries below the target count remain in order and the plan retains both Gather vectors and the
final assignment/chooser result rather than collapsing them into one completed vector.

The accepted generated-map path is not a filename/content inference. A valid preview writes start
staging in `Scenario + 0x11C0`; successful Start copies that staging before `.SED` regeneration.
The plan records explicit `AcceptedRmgStartStaging` provenance (or the separately proved authored
source) so a fresh external `.SED`, a cancelled preview, and an authored map cannot borrow those
entries. This provenance is independent of the later generated-vs-authored overlay load-source
discriminator.

The plan contains the complete logical pre-state/fingerprint `S0`, pass-1 House outcomes, both
Gather outcomes, final assignments, pass-2 House outcomes, and complete post-state/cursor `S1`.
`MapLoadInitial` creates one `ScenarioBootstrapRng` from the same match seed, requires exact full-state
equality with `S0`, installs `S1` once, and rejects a second installation because the pre-state no
longer matches. The plan exposes no RNG interface and is consumed from `LoadingRequest`; later House
and assignment installation is draw-free. After installation the bootstrap owner alone supplies
Fill, fixed-map low Mark or generated-construction replay as applicable, Post-Map, and simulation
draws. This is the same transition as direct execution, not a second gameplay RNG.

Focused tests must compare an independent reference stream across both H-sized House passes, both
mode-family callbacks, zero/one/many deficient retries, the zero-draw reset, Fill, RMG emitted and
discarded constructors, Post-Map, and simulation. They must include observers, AI/Neutral/Special,
Cooperative, sparse input, duplicate-install rejection, tampered pre-state, and accepted generated
staging provenance. The fresh design critic must recheck this universal transaction,
single-installation, and downstream-owner model against current `main`; any failure keeps P0-R1 and
transaction 3 blocked.

`MapLoadInitial` also carries the generated-source identity, `RmgConstructionTrace`, and then the validated generated-init table.
`GameEntity` owns the installed constructor word. Authored structure upgrades are distinct
`GameEntity` owners linked by stable parent ID and upgrade slot. The stable generated entity index
plus type/cell tuple makes a stale or misordered RMG binding a load error rather than silently
spending another draw.

The two event phases are exhaustive for the active generator. `0x005A6510` and `0x005A82E0` are
reachable only from no-xref `0x005A5020`; `0x005A91E0` also has no caller, and none of the three
entry addresses occurs as an image function pointer. Unit 2 must preserve that evidence-backed
exclusion rather than widening the trace for dormant RMG-shaped code.

#### Scenario-load transaction

1. After a fixed map is selected, parse it through the normal scenario loader. For `.SED`, first read
   seed/options and run the launch generator inside the load path after the successful-Start RNG reset;
   do not substitute the accepted preview payload.
2. For a fresh authored load, initialize the complete Scenario state from the authoritative launch
   seed before `Start_Scenario`, then select exactly one typed pre-Fill context:
   - stock offline noncampaign: P0-R1's two House passes, selected `+0x80`, selected `+0x84`, and
     zero-draw reset;
   - campaign: one constructor invocation per `[Houses]` row (or every registered HouseType when the
     section is empty), no multiplayer Gather/chooser and no second pass;
   - LAN/IPX: network seed -> House pass 1 -> common `+0x80` Gather -> selected Battle or
     Cooperative `+0x84` second Gather/chooser -> zero-draw reset -> identical House pass 2;
   - WOL state `2`: network seed -> House pass 1 -> common `+0x80` Gather -> common
     `AssignStartingPoints` (second Gather plus only the zero-occupied player and exactly-two-occupied
     AI chooser draws) -> zero-draw reset -> identical House pass 2;
   - replay: recorded seed/session and the corresponding campaign/noncampaign family, with no
     replay-only draw.
   Fill consumes from and returns the same cursor. A stream restore is a separate no-Full_Init/no-Mark
   transaction and retains native seed-zero Scenario restore behavior; it never enters this list.
3. On the authored Full_Init arm, parse explicit `[Tubes]` after Fill and before overlays. Keep them
   independent of low Road behavior; classify automatic same-cell shells from final theater land data
   without synthesizing traversal.
4. When native `NewINIFormat > 1` permits the pack reader, execute one decoded OverlayPack traversal,
   `y=0..511` outer then `x=0..511` inner. Each coordinate completes synchronously before the next:
   ordinary overlays run ordinary Mark, the four high anchors run their high structural stamp, and
   the eight low procedural triggers run the exact low Mark transaction on the same Scenario cursor
   continuation after prefix and Fill. Later packed coordinates can observe or overwrite earlier
   procedural writes; there is no component post-pass and no high-before-low phase split.
5. Only after the entire OverlayPack traversal, execute the complete OverlayDataPack traversal and
   overwrite each allocated/in-bounds cell's state byte. Then run the whole-map `RecalcAttributes`
   pass before Terrain and authored Unit/Aircraft/Infantry/Structure construction. Every procedural
   write still recalculates immediately in step 4; the final Recalc projects the later data overwrite.
6. On generated `.SED`, the early synthetic Full_Init defaults `NewINIFormat` to `0`, so steps 4/5
   are inert. Later preserve the generator's complete direct three-wide overlay/data rectangle and
   skip authored high/low Mark entirely. The explicit successful `.SED`/generated provenance, not
   overlay ids or construction-trace presence, selects this arm.
7. Rebuild records, zones and hierarchy only at the verified load boundary established by units 1,
   3, 4 and 5.

The app layer owns one explicit `ScenarioLoadContextDescriptor`, orthogonal to physical map source.
`LoadingRequest` creates it from proven startup/session/replay/stream provenance and
`MapLoadInitial` carries it through the load. Its normalized prefix kind is one of stock offline,
campaign, LAN, WOL-state-2, replay-with-recorded-family, or stream restore; it contains only the
roster/mode/House/start inputs the simulation prefix needs. Network transport, replay I/O and UI
session types remain in `app`; `sim::scenario_bootstrap` receives the normalized kind and inputs,
not app or networking dependencies. A seedless/generic current Rust entry may not guess stock
offline: a surface without proved context provenance returns an explicit unsupported-load-context
result and cannot enter authored Mark. Tests and headless callers must supply a typed context.

The actual app source enum is `LoadedMapSource::{Loose, Mix, Generated, LegacyFallback}`. App loading
derives the map-layer `OverlayLoadSource` exactly once and carries it explicitly:

```text
Loose | Mix    -> OverlayLoadSource::Authored
Generated      -> OverlayLoadSource::GeneratedMaterialized
LegacyFallback -> unsupported for exact OverlayPack Mark until explicit provenance is supplied
```

This mapping is independent of `generated_construction_trace`: a `Generated` load with a missing or
empty trace still skips authored Mark, while Loose/Mix remain authored. Load context is a second,
orthogonal gate. A stock-offline `Generated` path is accepted only for the proved chooser boundary,
Battle id `1` or FFA id `2`, with explicit accepted-preview start staging; arbitrary generated/mode
combinations and fresh external `.SED` injection are rejected. An authored campaign/LAN/WOL/replay
source can run Mark when `NewINIFormat > 1`; stream restore never runs Full_Init or Mark.

`ScenarioBootstrapRng` remains the sole cursor owner. After Fill returns, app orchestration retains a
non-clonable borrowed raw-only adapter from `sim::scenario_bootstrap`, but **no sim type crosses into
`map`**. The app invokes the map-owned inline OverlayPack routine with a map-native
`&mut dyn FnMut() -> u32`/equivalent raw-call interface backed by that borrow. `map` cannot range-wrap,
clone, reseed or import `sim`; it can only request the next raw word at the exact low-body write.
Inline processing applies `raw & 3` exactly `3*L` times on successful procedural body writes and
zero on every fixed/search/no-op/failure arm, then app releases the same borrow before authored
Techno construction. `src/map/overlay.rs` or the narrow low-Mark owner owns the recovered tables and
loop; `src/map/resolved_terrain.rs` owns overlay/state mutation and Recalc projection, not Scenario.

Every invalid coordinate returned by the native lookup aliases one persistent shared dummy across
the whole pass. Transaction 3 therefore extends the existing `SharedCellDummy` owner with the
overlay ID/state, Land/zone and cache fields actually read or written by low Mark/Recalc; it must not
allocate a fresh default per lookup. Occupied-body writes, missing lookups and edge rows still consume
their exact raw words and mutate that dummy in longitudinal/j order, while dummy Recalc remains a
no-op. Full-state tests bracket prefix, Fill, Mark and the first authored constructor so a hidden
second cursor, ranged helper, reordered batch or lost dummy alias cannot pass.

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

1. Deserialize authoritative cell/object/tube/hut/effect state and the raw serialized RNG fields as
   separate inputs; do not assume every serialized RNG byte remains the post-load authority.
2. Apply the proved per-stream restore rule. Scenario's serialized `+0x218` bytes are read and then
   overwritten by native `Random::Seed(0)`, so the required poststate is the complete seed-zero
   Scenario state and the restore path runs no Mark. Main and MapGen restore behavior remains open
   under OQ-19 and transaction 21 until separately proved; neither may inherit Scenario's rule or an
   unchanged-continuation assumption.
3. Reconstruct raw/mutable bridge projections without retaining stale derived pointers.
4. Rebuild records, zones, hierarchy, radar dirties and render snapshots deterministically.
5. Validate unchanged object plane, tube progress and pending debris; validate Scenario's seed-zero
   poststate exactly, and validate Main/MapGen only against the later OQ-19 contract.

### 4. Keep content-conditional mechanisms installed but dormant without data

Explicit `[Tubes]`, bridge trigger actions/events and Psychic Sensor enemy action lines must be implemented in their normal owners. They consume no state and produce no behavior when the corresponding valid map/type content is absent. They must not be compiled out, replaced with stock-map prevalence assumptions, or merged into low Road behavior. Automatic same-cell TubeClass shells are a separate load-time mechanism: units 1 and 5 must classify their final-theater activation and preserve their zero-length, non-traversable boundary until native evidence proves an active consumer.

### 5. Carry evidence and closure state alongside work

Each mechanism progresses through:

```text
FROZEN-COVERAGE -> CONTRACTED -> BUILT+FOCUSED-VALIDATION
                -> CRITIC-N FINDING/FIX -> CRITIC-N+1 PASS -> CLOSED
```

Any new active caller reopens coverage. Any unresolved native term keeps the contract blocked. Any critic finding keeps the mechanism open. Passing a subset of tests never changes the owning GSI row to closed while another required mechanism is open.

### P0 — shared Techno-constructor RNG prerequisite and P0-R1 prefix correction

Before fixed-low-load transaction 3 begins, one builder closes the smallest shared Scenario-prefix
correction needed to keep its first Mark draw exact. The original constructor P0 and transaction 2
are merged on current `main`, but P0-R1 is reopened because the merged optional plan omits the first
House pass, the second Gather in Cooperative, and every valid no-plan stock-offline path. P0-R1 must
generalize that substrate to the universal transaction above and pass a fresh critic. The completed
all-context audit supplies transaction 3's campaign/LAN/WOL/replay/save/generated/editor matrix; its
typed preservation fixtures remain mandatory and cannot be waived by a correct offline fixture. The
original constructor prerequisite is not a new bridge mechanism or a general Techno rewrite. It
models the one unconditional raw Scenario draw at
`TechnoClass__Constructor @ 0x006F3254`, stores the low word as
`GameEntity::techno_ctor_random_word`, and makes one internal construction funnel assign that field
exactly once.

The funnel receives an explicit initialization mode:

```rust
enum TechnoConstructorInit {
    FreshScenario,
    PreconsumedGenerated(GeneratedTechnoInit),
    Restored(u16),
}

struct StructureUpgradeLink {
    parent_stable_id: u64,
    slot: u8,
}
```

`FreshScenario` consumes and stores one raw word after valid type/house resolution and allocation
reach construction but before Unlimbo or any placement decision. It covers every active fresh
Techno ingress:

1. fixed authored `[Units]`, `[Aircraft]`, `[Infantry]`, and `[Structures]` in native section order,
   with increasing entry index within each section;
2. after a base structure successfully Unlimbos, each selected non-`-1` authored upgrade in native
   declared-count/slot order;
3. Post-Map starting MCV and extra-unit construction;
4. every ordinary runtime path that reaches `spawn_object_at_height` or
   `spawn_object_limbo_at_height`, including production, free units, sell survivors, slave-miner,
   spawn-manager, paradrop and other superweapon creation.

A later Unlimbo/placement failure never rolls the cursor back. Malformed rows, unknown house/type,
allocation failure, or another rejection before the native constructor consume zero.
`PreconsumedGenerated` validates the stable-index/type/cell binding from
`GeneratedTechnoInitTable`, installs its word and consumes zero. `Restored` reinstates the serialized
constructor word and consumes zero; this object-field preservation is independent of the separately
proved seed-zero Scenario RNG poststate.

Current `parse_map_entities` already preserves the four base section groups, but it must retain the
structure upgrade count and three type slots. After a base structure successfully Unlimbos, each
selected upgrade is constructed as a distinct `GameEntity`, owns its constructor word, and carries
`StructureUpgradeLink { parent_stable_id, slot }`. The link and word participate in snapshots and
hashes. This is the minimum faithful native constructor owner; it does not claim closure of the
broader structure-upgrade gameplay backlog.

Outside tests and deserialization, direct production use of `GameEntity::new_at_frame` must be
impossible or fail a source-boundary assertion. The common internal funnel owns the three current
construction sites in `world_spawn.rs`: map projection, `spawn_object_at_height`, and
`spawn_object_limbo_at_height`. `spawn_object` remains a delegating wrapper.

P0 gets its own implementation contract, focused `--lib` validation, commit, and fresh-critic loop.
Its required fixtures assert:

- exact fixed-map word binding and final cursor for one valid object in every section, a
  constructed-then-failed Unlimbo, a pre-construction rejection, and an upgrade entity with stable
  parent/slot identity;
- exact Post-Map starting-MCV and extra-unit word/cursor order, including a later placement failure;
- one ordinary placed runtime spawn and one limbo runtime spawn, each consuming and retaining its
  own word;
- a generated projection with validated bindings consuming zero additional draws;
- a snapshot round trip retaining every word and upgrade link without a constructor draw.

P0 remains open if any active fresh-construction ingress, section/entry order, upgrade event,
failure boundary, persistent owner, source mode, or cursor transfer is approximate.

## Implementation Transactions and Dependency Order

P0 plus the 21 bridge transactions below are dependency and native-transaction boundaries, not new
architecture layers and not mechanism pass gates.

| Order | Closure unit | Primary coverage | Dependency / ordinary-play oracle |
|---|---|---|---|
| 1 | Theater/rules/assets, raw flags, automatic-shell theater classification, TIBTRE mask preservation | BR-M01, BR-M06, BR-M24 | exact ten piece keys; raw-mask fixtures; automatic-shell corpus verdict; TIBTRE rejects `0x500`; retain raw SpecialFlags/session inputs for unit 10 |
| 2 | Active RMG preview/accept/`.SED` launch lifecycle, low deck/end/CABHUT production, and waterfall-topology exclusion | BR-M02, BR-M03 | P0 and unit 1; `7fee6929`, `4a63fa15`, `f1e6054b`, `a776f270`; fresh MapGen per run; first-entry/re-entry and no-preview gates; location-free discarded trace events; one launch `ScenarioBootstrapRng`; validated `GeneratedTechnoInitTable`; active-phase-only trace; complete stamped output; no generated Mark replay; `BuildRiverBridge` negative characterization |
| 3 | Fixed-map low overlay procedural load and Road mutation | BR-M05, BR-M11 | P0/P0-R1 and unit 1; complete active load-context cursor audit; Lost Lake/Killer plus destroyed low fixture; exact `NewINIFormat` activation; generated-source bypass preserving its full direct deck payload and zero Mark draws |
| 4 | High topology, records, zones, hierarchy and edge restamp | BR-M04, BR-M10, BR-M17 | unit 1; Bay of Pigs/Hills and Deadman's Ridge |
| 5 | Explicit TubeClass load/hierarchy/direction-8/persistence and automatic-shell non-traversal | BR-M12, part of BR-M22 | units 1/4; sealed valid custom tube fixture; zero-length shell negative case |
| 6 | Dual occupancy, entry, A*, peer markers and locomotor transitions | BR-M07, BR-M09, BR-M13 | units 4/5; deck, under-span, ramp, gap |
| 7 | Two native post-A* smoothing passes | BR-M25 | unit 6; no wrong-plane shortcut |
| 8 | Selected-layer scatter/crusher and stuck safety | BR-M23 | unit 6; ten-object order and bridge exemption |
| 9 | Spawn, Unlimbo, landing, paradrop, teleport and relayers | BR-M08, part of BR-M13/BR-M21 | unit 6; correct list/height after placement |
| 10 | Impact admission, destruction-authority matrix, Z/family gates and four-path RNG | BR-M01, BR-M16 | units 1/3/4; campaign/editor SpecialFlags, complete skirmish and true-network session `BridgeDestruction` handoff, CombatDamage non-owner, CABHUT bypass, strict impact boundary and negative family |
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
| 21 | Save/load/checksum/rebuild and deterministic projection | BR-M22 | authoritative state/derived rebuild stable; per-stream restore oracle with proved Scenario seed-zero and OQ-19-gated Main/MapGen |

### Mechanism closure gates

Every `BR-M` row has one persistent builder identity, one complete mechanism contract, and its own
fresh-critic chain. The builder may work through several dependency transactions, but another
builder's transaction or critic result cannot close the row. If a builder must be replaced, the
handoff records the reason, complete evidence/diff/output bundle, and new owner; the row is then
recriticized from its full requirement rather than only the replacement's last diff.

| Mechanism gate | Contributing transaction(s) |
|---|---|
| BR-M01 | 1 and 10 |
| BR-M02 | 2 |
| BR-M03 | 2 (negative characterization) |
| BR-M04 | 4 |
| BR-M05 | 3 |
| BR-M06 | 1 |
| BR-M07 | 6 |
| BR-M08 | 9 |
| BR-M09 | 6 |
| BR-M10 | 4 |
| BR-M11 | 3 and later mutation-preservation checks in 13/15 |
| BR-M12 | 5 |
| BR-M13 | 6 and 9 |
| BR-M14 | 18 |
| BR-M15 | 14 and 18 |
| BR-M16 | 10 and 18 |
| BR-M17 | 4 and 13 |
| BR-M18 | 14 and 17 |
| BR-M19 | 15, 16 and 17 |
| BR-M20 | 20 |
| BR-M21 | 9, 18 and 19 |
| BR-M22 | 5 and 21 |
| BR-M23 | 8 |
| BR-M24 | 1 |
| BR-M25 | 7 |
| BR-M26 | 12 |
| BR-M27 | 11 |

A row passes only after all of its contributing transactions, preservation tests, routed open
questions, and negative facts are exact and a critic who did not build it returns no material
finding. Split rows therefore remain open after an early transaction. Bundled transactions must be
decomposed into reviewable mechanism-scoped deltas or a separately named prerequisite commit; a
shared commit never grants a shared pass.

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

The destruction-authority matrix has an additional mandatory deferred evidence term already present
in the frozen source set: the complete UI/session writer chain into `DAT_00A8B260` and the true
network-multiplayer handoff are not yet proved. Transaction 10 and the BR-M01/BR-M16 mechanism
contracts must resolve `0x006B8AE0`, `0x006B8CA0`, `0x00671EA0`, and `Full_Init` branches
`0x0068794D..0x00687966`/`0x00687C16..0x00687C29` against active retail modes. Until that proof or an
evidence-backed exclusion exists, neither BR-M01 nor BR-M16 may pass. This corrects the design's
earlier claim of a fully settled matrix; it does not silently add or waive a frozen open question.

The fourth-review prerequisite concerning preview/cancel/accept/`.SED` dual-RNG ownership and the
Revision-4 critic findings concerning one launch cursor, generated-object bindings and generated
overlay replay, plus Revision-5 findings concerning discarded-event shape, dormant helper exclusion
and authored constructor ownership, and Revision-6 findings concerning Post-Map/runtime construction,
constructor-word persistence and upgrade identity are not left as new open questions. They are
resolved before implementation by the exhaustive lifecycle report at `7fee6929`, `4a63fa15`,
`f1e6054b`, and `a776f270`. They are mandatory P0/unit-2/unit-3 contract inputs.

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
| fixed authored, Post-Map and runtime Techno construction are not RNG-free; generated projection and snapshot restore are not allowed to draw a constructor word again | P0 and 2 |

## Builder, Critic, and Publication Protocol

For every `BR-M` mechanism:

1. Resolve every contributing transaction and open question against live `gamemd.exe` and retail data into one sourced mechanism contract. OpenTS may locate functions but supplies no required behavior.
2. Assign one persistent builder for the mechanism. The builder may preserve correct code, replace wrong code, and implement missing code only within that mechanism and its smallest verified prerequisite.
3. Check `cargo`/`rustc` ownership before validation. Run focused `cargo test -p vera20k --lib <filter>` commands only; never a bare Cargo test.
4. Commit the coherent evidence-backed slice after focused validation.
5. Give a fresh read-only critic who did not build the mechanism its complete requirement, native/retail evidence, exact diff, and literal validation output. For a split mechanism, the bundle includes every earlier contributing transaction and preservation test.
6. If it fails, fix the largest finding, commit the correction, and give the full updated bundle to a new critic. The new critic must recheck prior findings as well as the new diff.
7. Repeat until a fresh critic passes with no material finding. Approximate or unverified behavior cannot be relabeled residual; the mechanism and every owning row stay open.
8. After the mechanism pass, push its dedicated `feature/<mechanism>` branch and open a draft PR targeting `main`; publication is preauthorized. Record the contract, exact commits, critic pass, and focused literal output in the PR.
9. A mechanism PR remains draft because `ENGINE.md` reserves the one full `cargo test -p vera20k --lib` certification for the bridge-wide completion boundary. Do not declare an intermediate PR ready by substituting focused tests for that suite. Opening is authorized here; readiness and merging are not. After opening the first such PR, this autonomous run must stop, report the authority blocker, and request the user/reviewer decision needed to certify and merge it. It may resume the next mechanism only after then-current `main` contains the preceding mechanism; stacking or rewriting around the boundary is forbidden. The design does not claim an internally automatic draft-to-merge transition.

Critics do not edit. Builders do not self-approve. A critic pass proves only the bounded mechanism,
not the bridge system. The final bridge-wide audit runs the full `--lib` suite exactly once and is the
only point at which the aggregate completion PR may be declared ready for `main`.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — ordinary units must select and remain on the correct deck/ground plane through entry, A*, smoothing, locomotion and occupancy. Trigger: every high-bridge crossing. Player effect: refused routes, wrong-layer shortcuts, overlap or units falling between layers. Frequency: common on high-bridge maps. [BR-M07, M09, M13, M23, M25]
- `MILESTONE-BLOCKING` — low bridges must remain flat Road overlays and mutate exactly through intact/damaged/destroyed/repair states. Trigger: every low crossing and bridge damage. Player effect: wrong movement class, impassable water or invented tunnel behavior. Frequency: common on stock low-bridge maps. [BR-M05, M11, M17, M19]
- `MILESTONE-BLOCKING` — collapse and repair must preserve native per-cell transaction/RNG order. Trigger: bridge weapon damage, CABHUT C4, attached bombs or engineer repair. Player effect: different survivors, debris damage, bridge shape, zones and events. Frequency: common whenever bridges are contested. [BR-M16..M19, M26, M27]
- `MILESTONE-BLOCKING` — bridge destruction authority must follow the active mode/source matrix: scenario `[SpecialFlags]` where authoritative, skirmish/multiplayer session `BridgeDestruction` where authoritative, never `[CombatDamage] DestroyableBridges`; CABHUT C4/attached bombs bypass the weapon gate. Trigger: every attempted weapon or hut-driven collapse when sources disagree. Player effect: bridges become wrongly indestructible/destructible or hut sabotage stops working. Frequency: every configured disagreement and every CABHUT collapse. [BR-M01, BR-M16, BR-M19]
- `MILESTONE-BLOCKING` — every fresh Techno construction must consume and retain its one Scenario word, while generated projection and restore must not double-draw it. Trigger: valid authored base/upgrade objects, Post-Map starting forces, and ordinary runtime production/spawn, including a later failed Unlimbo. Player effect: later bridge damage/debris/repair randomness and constructor-word-driven report choices diverge; fixed-map low-Mark variants do not, because native Mark runs before authored Technos. Frequency: essentially every match, with authored-map impact on most stock maps and runtime impact whenever units are created. [P0 prerequisite]
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
- `COMPOUNDING` — snapshot/restore must apply each native persistence transformation and rebuild derived state; Scenario specifically becomes seed-zero rather than retaining its serialized cursor. Trigger: every save/load. Player effect: plane, path, debris or RNG divergence immediately or later. Frequency: every restored bridge-bearing game. [BR-M22]
- `RESOLVED-EXCLUSION` — TS-only, dormant, editor-only and name-only mechanisms remain excluded. Trigger: future code searches or OpenTS comparison. Player effect if violated: invented behavior and architecture drift. Frequency: development-time risk rather than retail runtime. [negative-fact ledger]

## Determinism and Persistence

- Scenario, Main and MapGen RNG streams remain distinct. Each closure contract names the stream and exact draw order.
- Every RMG call reconstructs MapGen from the stored map seed. Preview consumes and returns the
  process shell Scenario cursor; successful Start discards that cursor by constructing the gameplay
  Scenario/Main streams from the match seed before `.SED` regeneration.
- Every valid active stock offline noncampaign launch owns one immutable
  `PreFillScenarioPrefixPlan`. It evaluates both H-sized House passes, both mode-family Gather calls,
  default-cell deficient retries and the zero-draw reset from the match-seeded `S0`; load validates
  the complete pre-state, installs the exact `S1` once, and then owns every downstream draw. A
  no-plan stock-offline fallthrough, one-House-pass plan, Battle-only plan, or Cooperative one-Gather
  path is explicitly nonconforming. Non-offline contexts use the separately proved typed matrix:
  campaign single House pass; LAN House1/`+0x80`/selected-`+0x84`/reset/House2; WOL
  House1/`+0x80`/common-assignment/reset/House2; replay inheritance; and save/generated no-Mark
  boundaries. None may substitute the offline prefix.
- Generation-time constructor events are authoritative even when the attempted object is later
  deleted. `RmgConstructionTrace` records all attempts. `GeneratedTechnoInitTable` binds the one
  consumed low word for each emitted entity, and validated projection installs
  `GameEntity::techno_ctor_random_word` without spending the same Scenario draw twice.
- P0 gives fixed authored-map, Post-Map and runtime constructors the same field from a direct draw
  in native order. It consumes before Unlimbo and therefore retains the cursor advance when
  placement later fails. Authored upgrades are distinct stable entities linked to their base and
  slot, so their constructor words persist for later Techno consumers, snapshots and hashes.
- Generated projection and snapshot restore install an existing constructor word and consume zero.
- Generated low-deck overlay/data rectangles are final load payloads. Only fixed authored low
  endpoints run `OverlayClass::Mark` and its Scenario draws.
- Fixed-range draws still advance the verified RNG stream.
- Linked-list/vector traversal order is retained where it controls outcomes: collapse fallout, scatter snapshots, repair selection and observer delivery.
- Presentation consumes no gameplay RNG and cannot mutate bridge state.
- New deterministic runtime state must be included in snapshots/hash only when native-active behavior persists across ticks. Derived topology is rebuilt at the verified restore boundary.
- Snapshot schema changes are isolated to the closure unit that introduces authoritative persistent state; old-version rejection and deterministic round trips are tested there.

## Validation Strategy

During mechanism work, use only focused `--lib` tests after confirming no other session owns Cargo. Favor native-trace tables and small retail fixtures over broad certification matrices.

Required fixture families:

- P0 constructor fixture family: fixed-map Unit/Aircraft/Infantry/Structure order, a
  constructed-then-failed placement, a pre-construction rejection, and a distinct linked structure
  upgrade; Post-Map starting MCV/extra-unit order including failure; ordinary placed and limbo
  runtime spawns; generated projection spending zero additional draws; and snapshot round trip
  retaining words/upgrade links without a constructor draw;
- P0-R1 stock-offline prefix fixtures for all ids `1..9`, two identical native-roster House passes
  including observer and invalid-AI-slot cases, two independent Battle/Cooperative Gathers, sparse and
  deficient default-cell inputs, zero-draw reset, accepted-RMG staging provenance permitted only for
  Battle id `1`/FFA id `2`, rejection for other generated/mode combinations, exact full-state single
  installation, duplicate/tampered rejection, and draw-free later assignment projection;
- typed non-offline load-context fixtures: campaign single House pass and no Gather; LAN full
  House1/`+0x80`/selected-`+0x84`/reset/House2 sequence; WOL full House1/`+0x80`/common-assignment/
  reset/House2 sequence with both gated chooser arms; replay adds zero before its recorded family;
  stream restore performs no Mark and ends Scenario at seed zero; a generic unsupported context
  cannot guess stock offline; and generated `.SED` retains the stock-offline prefix while its source
  provenance suppresses Mark;
- authored low-Mark raw-seam fixtures bracketing full cursor states after prefix, after Fill, after
  exact `3*L` `raw & 3` writes, and before the first authored Techno; fixed/search/no-op/failure arms
  draw zero, and no ranged helper or cloned cursor is accepted;
- source mapping fixtures: Loose and Mix map to authored Mark; Generated maps to materialized/no-Mark
  even with missing or empty construction trace; LegacyFallback and untyped Generic reject rather
  than guessing; accepted generated start staging is distinct from both map source and trace;
- one interleaved authored OverlayPack fixture where an earlier low trigger writes cells and a later
  packed coordinate observes/overwrites them, including a high anchor in the same y/x traversal;
  then a conflicting OverlayData byte must win before the final global Recalc produces the exact Road,
  LAT/CliffBack, zone and compact-cache result with no per-cell radar dirty or Tube creation;
- low-Mark adversarial geometry fixtures for adjacent endpoints, first of two exact opposites, wrong
  ID/state pass-through, occupied fixed-row successful no-op, missing opposite partial fixed end,
  occupied/missing body overwrite, and edge lookups aliasing one persistent extended dummy in exact
  row/j order while preserving draw count and return/tail behavior;
- Lost Lake and Killer: intact low crossings;
- Bay of Pigs and Hills: high deck, under-span, dual-plane and AttackMove;
- Deadman's Ridge: high collapse gap;
- Shrapnel Mountain: destroyed low bridge;
- deterministic RMG type 3/4 preview/cancel/accept/launch sequence asserting fresh MapGen state on
  each run; first setup entry with seed `-1` taking one shell seed draw and re-entry taking none;
  continuing shell Scenario cursor across repeated previews and Cancel; no third Generate on Use Map
  with a preview and exactly one generation on Use Map without one; `.img` versus `.SED` commit gates;
  successful-Start Scenario/Main reseed; unconditional `.SED` regeneration; one launch cursor through
  the complete stock-offline pre-Fill prefix, Fill, ordered construction replay, projection, Post-Map
  and Simulation; complete
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
- snapshot restore across active movement, collapse debris and repair, asserting the complete
  seed-zero Scenario poststate and separately contracted Main/MapGen behavior rather than unchanged
  generic RNG continuation.

The full suite `cargo test -p vera20k --lib` runs exactly once after P0 and all 27 bridge mechanisms
and their critic cycles pass, immediately before the bridge-wide reverse audit is declared ready. It
is not rerun per transaction, mechanism iteration, or intermediate draft PR.

## Bridge-Wide Reverse Audit

After P0 and all bridge-mechanism passes:

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
  lifecycle/follow-up report at `7fee6929`, `4a63fa15`, `f1e6054b`, and `a776f270`, the P0-R1 and
  three low-Mark follow-up reports in Sources, or explicitly open;
- all 27 mechanisms map to explicit mechanism gates and contributing implementation transactions;
- all 31 open questions route to a pre-implementation owner and final audit;
- no chosen interface collapses distinct native facts;
- the player-experience ledger covers ordinary high/low traversal, combat, collapse/repair, RMG, presentation, content-conditional tubes/triggers and restore;
- the critic protocol cannot close a mechanism with unresolved evidence;
- the universal stock-offline pre-Fill substitution includes both House passes, both mode-family
  Gather calls, exact default-cell deficient retries, zero-draw reset and explicit generated-staging
  provenance limited to accepted Battle/FFA on one native Scenario stream, and cannot install stale
  state, install twice, or leave a parallel downstream owner;
- the app-owned load-context descriptor is orthogonal to physical `LoadedMapSource`; Loose/Mix map
  to authored, Generated maps to materialized even without a trace, LegacyFallback/Generic reject,
  and normalized prefix inputs enter `sim` without app/network dependencies;
- one y/x OverlayPack traversal interleaves high/low Mark, followed by the complete OverlayData pass
  and global Recalc before Terrain/Technos;
- app retains the one Scenario borrow after Fill and backs a map-native raw-call interface, so `map`
  imports no `sim` type; the same cursor returns before authored Technos with no ranged substitute,
  clone or reseed, and edge writes alias one extended persistent shared dummy;
- restore asserts Scenario's verified seed-zero poststate while leaving Main/MapGen explicitly
  OQ-19-gated rather than claiming generic unchanged continuation;
- intermediate mechanism publication remains draft and merge-gated, while the full `--lib` suite
  runs exactly once at bridge-wide completion;
- P0 has a bounded all-ingress constructor requirement and its merged builder/fresh-critic evidence;
  P0-R1's universal stock-offline prefix correction and fresh critic pass before transaction 3 uses
  that shared Scenario cursor; and transaction 3 preserves the completed campaign/LAN/WOL/replay/
  save/generated/editor context matrix without an untyped offline fallback.

Until that review passes, implementation remains blocked by design rather than by user authority.

## Sources

- `docs/research/bridges/00-system-models/ACTIVE_RETAIL_BRIDGE_COVERAGE_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/00-system-models/RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/00-system-models/SCENARIO_PREFIX_PLAN_INELIGIBLE_FALLBACK_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/LOW_OVERLAY_MARK_FIXED_MAP_STAMP_RNG_TRANSACTION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/LOW_OVERLAY_MARK_SCENARIO_LOAD_ACTIVATION_BOUNDARY_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/LOW_OVERLAY_MARK_ALL_LOAD_CONTEXT_SCENARIO_RNG_LIFECYCLE_GHIDRA_REPORT.md`
- active `gamemd.exe` addresses and retail inputs enumerated by that report
- current Rust owners at `origin/main` snapshot `a3e4ce9a`
- `docs/system-map/topology.v2.json` `bridge-helpers` service boundary
- `C:\Users\enok\Documents\OpenTS` correspondence ledger, as navigation leads only
