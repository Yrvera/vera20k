# RMG Low-Bridge Launch Construction Implementation Contract

**Date:** 2026-08-29
**Status:** READY_FOR_PLAN
**Owning slice:** Active-retail bridge parity, closure unit 2
**Coverage:** BR-M02, BR-M03 and the launch-construction prerequisite shared with generated
neutral Technos
**Implementation authority:** active-retail `gamemd.exe` and Yuri's Revenge retail data only

## Gap Being Closed

For active random-map types 3 and 4, Rust currently omits the water-region low-bridge production
branch, records only surviving neutral-tech placements, advances no shell/gameplay Scenario cursor
for generated Building constructors, and treats an accepted preview `GeneratedMap` as gameplay
authority instead of regenerating the accepted `.SED` after the match reseed. This removes deck/end
terrain, CABHUTs, ordered MapGen and Scenario draws, and the exact constructor words installed on
generated entities.

## Scope

Included:

- the persistent RMG setup seed/options state relevant to first entry and re-entry;
- preview Generate, Cancel, Use Map, and accepted `.SED` ownership as they affect RMG/Scenario
  state;
- launch-time `.SED` regeneration from the accepted options;
- the one authoritative match-seeded `ScenarioBootstrapRng` order through the existing Battle
  preload transfer, terrain Fill, generated-constructor trace, entity projection, post-map, and
  Simulation handoff;
- type-3/type-4 flood-region low-deck eligibility and iteration order;
- exact low-deck seed search, axis walks, validators, length policy, direct overlay/data stamp,
  end validators/coins/tile blocks, and CABHUT placement;
- ordered `BridgeRepairHut` and `NeutralTech` construction events, including discarded neutral
  buildings;
- stable generated entity binding through the existing `GeneratedTechnoInitTable`;
- a negative characterization proving `BuildRiverBridge @ 0x0059E740` remains waterfall terrain
  shaping and produces no bridge topology or construction event;
- correction of stale Rust comments that call active RMG behavior dormant.

Excluded:

- land-region ramp formulas and the unresolved ramp-end tileset key;
- the rest of river/water terrain generation except the scoped no-topology boundary;
- fixed-map bridge discovery/Mark, runtime movement, destruction, collapse, repair, zones, Tube
  traversal, rendering, audio, and UI pixel parity;
- general neutral-tech placement parity except constructor-event timing/outcome transport;
- shell chrome, text, progress-bar pixels, and preview raster color parity;
- allocation failure/OOM behavior;
- TS-only `TrainBridgeSet` and unreachable RMG-shaped helpers.

The scope is based on the completed exhaustive re-investigation and lifecycle reports, not on an
unverified design inference. The approved bridge design identifies this as closure unit 2; this
contract replaces ownership hypotheses with exact deltas.

## Activation and Native Order

The target is active when the stock offline Skirmish Create Random Map workflow or another valid
`.SED` selects RMG map type 3 or 4. `RandomMapGenerator__Generate @ 0x00598960` reaches
`BridgeAndConnectorPass @ 0x0058EF10` only for those two types. The pass builds adjacency for all
regions, processes all region connections, then releases adjacency vectors. Flood-class regions
consider unordered neighbor pairs and call `PlaceLowBridgeDeck @ 0x0058F2C0` when both endpoints
are land-class, both satisfy `neighbor_count > 1 || cell_count > 50`, and both endpoint levels
equal the flood region's level.

The launch lifecycle is:

```text
shell process Scenario cursor
  first setup entry with Seed=-1: one U[0,0xFFFF] seed draw
  preview Generate: fresh seed-built MapGen + ordered generated Building constructor events
  Cancel: preview Scenario effects persist and completed preview output remains available;
          no .SED/selection commit
  Use Map with preview: no third generation; persist RandMap.Sed/options

successful Start
  reseed Scenario and Main from match seed
  read accepted .SED
  fresh seed-built MapGen launch generation
    Full-Init prefix (houses/start assignment and Fill on one Scenario owner)
    CABHUT construction events during type-3/type-4 connector work
    neutral-tech construction events after starts
  generated entity projection installs preconsumed constructor words
  Post-Map and gameplay continue the same Scenario owner
```

Rust may precompute MapGen-only geometry before the Scenario prefix. That is an acceptable
Rust-native translation because generated constructor words do not choose geometry or placement
success. It must retain the exact ordered construction trace and replay it after the authoritative
Battle/Fill prefix and before generated entity projection, on the one owner ultimately transferred
to Simulation.

## Evidence Baseline

| Source | Role | Use |
|---|---|---|
| `docs/research/bridges/00-system-models/RMG_LOW_BRIDGE_DECK_CABHUT_ACTIVE_RETAIL_CLOSURE_GHIDRA_REPORT.md` | PRIMARY native/retail | Complete active call graph, low-deck/end/hut mechanism, exact constants/rectangles, retail PavedRoad/CABHUT identities, constructor trace, waterfall and dormant exclusions |
| `docs/research/bridges/00-system-models/RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY native/runtime | Setup seed stream, per-call MapGen reset, preview/Cancel/accept state, Start reseed, `.SED` launch regeneration, one Scenario owner, trace/binding contract, generated direct-overlay rule |
| `docs/research/MAPGEN_SAME_PROCESS_LIFECYCLE_BRIDGE_CALLER_RECONCILIATION_GHIDRA_REPORT.md` | PRIMARY native reconciliation | Active MapGen/Scenario owner distinction and caller census |
| `docs/research/skirmish-ui/RMG_BRIDGE_CONNECTOR_PASS_0058EF10_GHIDRA_REPORT.md` and `RMG_MODE34_WATER_BRIDGES_TECH_GHIDRA_REPORT.md` | PRIMARY native corroboration | Connector pass, low-deck, CABHUT, and neutral-tech callsites |
| Active retail theater INIs/TMPs plus `rulesmd.ini`/`artmd.ini` fallback chain | PRIMARY retail data | `PavedRoads`, `PavedRoadEnds`, CABHUT `Foundation=1x1`, Neutral/repair-hut semantics, neutral-tech roster |
| `src/map/rmg/phases/bridge_deck.rs`, `carve_driver.rs`, `bridge.rs`, `tech_buildings.rs`, `src/map/rmg/tiles.rs`, `pipeline.rs`, `build.rs`, `emit.rs`, and `mod.rs` at `babb252f` | PRIMARY Rust | Current generation algorithms, omissions, outputs, and stale comments |
| `src/app/shell_random_map.rs`, `src/app/shell_skirmish.rs`, `src/ui/skirmish_shell/state/random_map_setup.rs`, `src/app/frontend/skirmish_session.rs`, `src/app/loading/pump.rs`, and `src/app/loading/init.rs` at `babb252f` | PRIMARY Rust | Current seed stream, setup persistence, retained-preview shortcut, `.SED` generation path, Battle plan and bootstrap creation |
| `src/sim/scenario_bootstrap.rs`, `src/sim/runtime.rs`, `src/sim/world/world_spawn.rs`, and `src/sim/game_entity.rs` at `babb252f` | PRIMARY Rust | Existing authoritative cursor transfer and generated-Techno binding seam |
| `docs/plans/2026-08-28-active-retail-bridge-parity-design.md` | DERIVATIVE approved design | Unit boundary and architecture constraints only; not native evidence |
| `C:\Users\enok\Documents\OpenTS\code\mapgen.cpp`, `isotype.cpp`, and `scenario.cpp` | NAVIGATION ONLY | Located inherited correspondences; no parity row relies on it |

The research-index preflight was anchored at `0x0058F2C0`, `0x005904B0`, and `0x005A7440`.
It found the two older verified RMG bridge reports and the lifecycle reconciliation. The newly
committed exhaustive report was not yet indexed, so the contract cites it directly. Index output
is navigation, not evidence authority.

## Visual Asset Role Table

This contract changes preview/gameplay ownership but does not claim pixel changes. The UI-facing
roles must remain separated:

| Asset/state | Exact frame | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|
| setup-generated preview surface / Rust `PreviewImage` | single generated bitmap | after progress/final projection | inside random-map setup preview rect | yes while setup open | content preview | no | no | no | no | lifecycle report; current `rasterise_generated_map` |
| `RandMap.img` | single PCX-like preview bitmap | chooser/loading preview holder | behind the loading text layers | yes after accepted generation; also written by common teardown after a generated preview | content preview | no | no | yes | no | lifecycle report §9 and current decoder |
| `RandMap.Sed` | not visual | accepted options/seed reader | never painted | no | no | no | no | launch input | no | `.SED` reader path |
| accepted preview `GeneratedMap` | no native visual frame; Rust in-memory fallback | current Rust only | current loading fallback may color its MapFile | may be used as fallback preview today | preview-only after this slice | no | no | yes | gameplay-inactive | lifecycle report and `LoadingRequest::retained_random_map` |
| loading-screen text/chrome | existing asset-specific frames outside scope | existing path | after preview-holder branch | yes | no | yes | text overlays | yes | no | lifecycle report §9; current composition owner |

Required ownership correction: the accepted preview may remain available only to preview/loading
composition if needed, never as `MapLoadInitial.map_data`, MapGen continuation, construction trace,
or generated-Techno binding authority. No row authorizes changing preview frame selection, chrome,
text order, or raster colors.

## Exact Low-Deck Baseline

### Region and attempt gates

- Process connector regions in creation order after a complete adjacency prepass.
- Flood-class region pairs are unordered and visited once in ordered-neighbor nested-loop order.
- Both endpoints are land-class, substantial, and at the same level as the flood region.
- One qualifying pair invokes a 200-attempt placer, attempts `0..199`.
- Each attempt draws uniformly over the whole `scratch_width²` array until it finds a real
  non-`(0,0)` record owned by the flood region. Every rejection consumes MapGen.

### Candidate search and choice

- NS starts from paired `3x1` strips and walks Y; EW starts from paired `1x3` strips and walks X.
- All twelve rectangle-road callsites pass paved-road and paved-road-end overrides zero.
- Every stepped strip checks its two outer cells for playfield membership and special terrain.
- Each surviving side requires the beyond-end `3x3` approach block.
- Endpoint scratch region ids must equal the requested unordered land-region pair.
- Strictly shorter candidate wins; EW wins an equal-length tie.
- A candidate requires `length < attempt / 25 + 8`.

### Deck validator and direct stamp

- Read origin level before corner checks.
- Validate four diamond corners.
- Sweep `(w+1)*(h+1)` cells inclusive; require no overlay, identical signed level, and exact Clear
  or native absorbable tile family.
- Absorbable means WaterSet `base..base+14`, ShorePieces `base..base+42`, or any of four waterfall
  bands `base..base+4`, with exclusive range ends. It is tile-only, not sub-tile-sensitive.
- EW directly writes west `0x5E`, east `0x5C`, interior `0x4A + signed_mod(x,4)`, and density
  `y-origin_y`.
- NS directly writes north `0x60`, south `0x62`, interior `0x53 + signed_mod(y,4)`, and density
  `x-origin_x`.
- The full rectangle is already materialized; generated projection never invokes fixed-map Mark.

### End pieces

- End-area helper checks corners before origin, sweeps exact exclusive `w*h`, ignores overlays,
  requires one signed level, rejects PavedRoads/PavedRoadEnds, and otherwise accepts exact
  Clear/MiscPave/Pave.
- Rectangles: EW east/west `6x6`; NS north/south `7x6`, with exact origins in the research report.
- Coin is drawn only if its end area passes; false area or false coin uses default.
- Alternates are PavedRoads `+10,+9,+13,+12`; defaults are PavedRoadEnds `+0,+2,+1,+3` with the
  exact axis/end anchors in the report.
- The tile-block stamper uses scratch id `-1`, level base `-1`; it changes tile/sub-tile/slope and
  scratch region id at record `+0x38`, preserves cell level, and leaves the independent scratch
  stamp at record `+0x3C` unchanged.

### CABHUT

- Per end, scan the exact primary rectangle then its fallback only after primary failure.
- Each supplied rectangle is scanned inclusive `(w+1)*(h+1)`, Y-major then X-major.
- First cell with no overlay, exact Clear/unassigned tile, and no occupier wins.
- The stock 1x1 Neutral CABHUT constructs at that cell, consuming one raw Scenario word before
  Unlimbo, then emits on the active generated fixture.
- At most two huts construct per successful deck. Missing huts do not fail the deck.

## Parity Delta Table

| ID | Evidence class | Delivery class | Mechanism/result | `gamemd.exe` behavior | Current Rust behavior | Required Rust delta | Evidence | Acceptance test |
|---|---|---|---|---|---|---|---|---|
| U2-01 | REQUIRED_FIX | MILESTONE-BLOCKING | Persistent setup seed/options and seed stream | Global MapSeed options survive setup close. Only first entry with `Seed==-1` draws Scenario `U[0,0xFFFF]`; re-entry with a valid seed draws none. | `shell_skirmish.rs:627-633` opens from `RmgOptions::default()` every time; `random_map_setup.rs:194-202` draws the unset seed from caller-supplied `frontend_main_rng`. | Give offline-skirmish process state persistent RMG options. Draw only an unset seed from `OfflineSkirmishRuntime`'s Scenario owner; reuse valid options/seed on re-entry. Keep option Randomize/derived rerolls on Main. | lifecycle report §§3.1, 8; current lines above | Open/cancel/reopen fixture proves one Scenario seed draw total, zero Main seed draw, option persistence, and no second seed draw. Separate Randomize fixture proves only Main advances. |
| U2-02 | TEST_ONLY | COMPOUNDING | Generate/OK generation count | Every Generate starts fresh MapGen from current seed. Use Map with a valid preview runs no third generation; Use Map without one runs exactly one preview generation. | Worker generation creates a fresh `RmgRng`; `accept_setup` takes the candidate; OK-without-preview schedules one generation. | Preserve behavior while changing retained ownership and adding traces. | lifecycle report §§3.2, 3.5; current modal/poll paths | Counters prove repeated Generate restarts MapGen, OK with preview adds zero runs, and OK without preview adds exactly one. |
| U2-03 | REQUIRED_FIX | MILESTONE-BLOCKING | Preview Scenario constructor effects and Cancel | Every preview construction event advances the process shell Scenario cursor. Cancel preserves those effects, writes/keeps the preview product when one exists, but does not commit `.SED`/sentinel/selection. | Preview worker is MapGen-only; `poll_random_map_generation` retains geometry but does not touch `OfflineSkirmishRuntime.scenario_rng`. Cancel discards candidate/accepted map state. | Return an ordered construction trace from preview generation; on main-thread completion replay one raw shell Scenario word per event and retain that cursor regardless of later Cancel. Keep `.SED`/selection commit gated on acceptance. | lifecycle report §§3.3-3.4; current `shell_random_map.rs:359-390,432-470` | Generate→Cancel advances shell Scenario by exact trace length; `.img` reflects completed preview; `.SED`, sentinel, and selection stay unchanged; reopen continues the advanced cursor. |
| U2-04 | REQUIRED_FIX | MILESTONE-BLOCKING | Accepted preview versus launch authority | Accepted setup persists options/seed and preview bitmap, but successful Start reseeds Scenario/Main and `.SED` reader runs a new generation. Preview MapFile/MapGen/constructor words are never gameplay authority. | `RandomMapGenerationRetention` stores accepted `(String, GeneratedMap)` at `shell_random_map.rs:125-190`; `LoadingRequest` carries it at `pump.rs:221-314`; `retained_random_map_initial` installs it directly at `init.rs:833-844`. | Retain only UI preview state if needed. Route every accepted `.SED` launch through the normal `.SED` reader/generator path. Never transport preview MapGen continuation or preview constructor bindings to gameplay. | lifecycle report §§3.6-3.8; current Rust lines | Preview and launch generation counters show two same-seed runs; launch begins MapGen at fresh `0/103`; mutating/discarding preview-only storage cannot change launched map, trace, words, or final cursors. |
| U2-05 | TEST_ONLY | COMPOUNDING | Ordinary `.SED` generation entry | On successful `.SED` parse, launch generates from normalized options and returns launch MapGen continuation. | `load_map_initial_with_assets` already detects `.SED`, loads options/theater/TMP/tech inputs, calls `generate_map`, and returns its continuation at `init.rs:1041-1177`; the accepted-preview shortcut bypasses it. | Preserve this path and make it the single accepted-launch entry. Extend its `MapLoadInitial` payload with construction trace/source identity. | lifecycle report §3.7; current Rust | Direct `.SED` and accepted UI `.SED` take the same generator entry and produce identical map/trace/MapGen continuation. |
| U2-06 | REQUIRED_FIX | MILESTONE-BLOCKING | Complete flood-region branch | After full adjacency prepass, every active flood-class region evaluates exact unordered land-neighbor pairs and calls one low-deck placer per qualifying pair. | `pipeline.rs:224-260` builds only `{id,level,waterish}` and loops driver; `carve_driver.rs:119-129` returns immediately for `waterish`. Neighbor count/cell count are unavailable to the branch. | Extend Rust-native connector facts with ordered neighbor ids, cell count, flood class, and level or supply equivalent read-only access. Run exact flood-pair gates while preserving existing land-ramp order and the complete-prepass view. | closure report §§3, 4; current Rust | Synthetic ordered region graph covers land/flood classes, count threshold on each side, triple-level gate, pair order, one visit per pair, and no MapGen draws for ineligible pairs. |
| U2-07 | REQUIRED_FIX | COMPOUNDING | Seed-cell rejection picker | Per attempt, whole-square uniform draws reject wrong region and `(0,0)`, with each rejection spent and no counter or escape. | `bridge_deck.rs` routes the production placer through the exact unbounded draw/reject loop after an RNG-free impossibility pre-scan. | Preserve the RNG-free no-cell guard, unbounded eligible-region loop, and actual placer ownership. | closure report §4.1; `PlaceLowBridgeDeck @ 0x0058F2C0`, picker loop `0x0058F337..0x0058F38F`; current Rust | Rejection/cursor tests remain; a controlled draw source reaches an eligible record after the former 10,000,000-draw boundary without returning `None`; integrated placer fixture proves the accepted seed and post-rejection cursor. |
| U2-08 | REQUIRED_FIX | MILESTONE-BLOCKING | NS/EW corridor construction and selection | Build both axes with exact strip/approach gates and zero road overrides; endpoint pair exact; shorter wins, EW tie; strict `length < attempt/25+8`; max 200 attempts. | No full placer exists in `bridge_deck.rs`; `carve_driver` never calls one. | Implement the exact candidate walks and selection using existing `CarveCtx` playfield, scratch, tile blocks, and MapGen owner. Do not share the deck/end sweep or collapse axes into a nearest-point algorithm. | closure report §§4.2-4.3 | Fixtures cover NS-only, EW-only, both shorter directions, equal tie, invalid third-region endpoint, special/playfield stop, and every 25-attempt threshold boundary. |
| U2-09 | REQUIRED_FIX | MILESTONE-BLOCKING | Exact deck absorbable predicate | Inclusive deck margin accepts Clear or exact WaterSet 14, ShorePieces 42, and four waterfall bands 4; no sub-tile rule or other special family. | `bridge_deck.rs:134-174` has correct sweep/overlay/level shape but calls `TileIds::is_bridge_absorbable`; `tiles.rs:22,193-201` accepts only WaterSet 6 + shore and explicitly defers waterfalls. | Add/use an exact tile-only predicate for `0x004865D0`. Preserve the current sub-tile-sensitive `is_special_terrain` for its different owner. | closure report §5; current Rust | Boundary matrix tests base-1/base/base+last/base+span for all six families, WaterSet tiles 6..13, waterfall sub-tiles ignored, and unrelated cliff/bridge families rejected. |
| U2-10 | REQUIRED_FIX | MILESTONE-BLOCKING | Direct low-deck overlay/data stamp | Successful EW/NS deck directly writes exact endpoint/interior overlay ids and cross-section byte over full 3-wide rectangle; no body tile or Scenario draw; no later Mark. | No stamp exists. Emitter projects current `GridCell.overlay/density`, which is a suitable direct carrier. | Write exact overlay/density values in native coordinate order. Mark the generated source through loading/projection so fixed-map `OverlayClass::Mark` expansion is never replayed over it. | closure report §6; lifecycle report §12.3 | EW and NS golden rectangles prove ids/data and untouched tiles. Generated projection preserves rectangle and Scenario cursor; fixed authored endpoint fixture still uses ordinary Mark behavior. |
| U2-11 | TEST_ONLY | COMPOUNDING | End-area predicate | Corners first; exact exclusive `w*h`; equal signed level; overlays ignored; zero override rejects paved roads/ends and accepts Clear/MiscPave/Pave. | `bridge_deck.rs:181-229` appears exact and has focused fixtures. | Preserve helper and call it with the four exact rectangles. | closure report §7.1; current Rust | Existing tests remain plus integrated NS `7x6` and EW `6x6` rectangle-call fixtures. |
| U2-12 | REQUIRED_FIX | MILESTONE-BLOCKING | Conditional paved-end coins and block stamps | Per end, draw `U{0,1}` only after validator success; alternates PavedRoads `+10,+9,+13,+12`, defaults PavedRoadEnds `+0,+2,+1,+3`; exact anchors; scratch/level args `-1`; level preserved. | End selection and stamping use the shared native-equivalent block stamper; the remaining acceptance matrix is incomplete. | Preserve exact per-axis order, conditional draws, identities, anchors, and block stamping that writes tile/sub-tile/slope plus `scratch.region=-1`, leaves `scratch.stamp` unchanged, and preserves level. | closure report §7; `StampIsometricTileBlock @ 0x005A6C10`; retail theater data | All four ends cover validator false, coin false, coin true; assert draw counts, exact anchors/tiles/subcells/slopes, scratch region `-1`, unchanged scratch stamp, and level preservation. |
| U2-13 | REQUIRED_FIX | MILESTONE-BLOCKING | CABHUT search, occupancy, emission | Per end scan primary then fallback; each inclusive Y-major/X-major; first overlay-free Clear/unoccupied cell constructs/emits stock Neutral 1x1 CABHUT and spends one Scenario word; hut failure never rolls deck back. | CABHUT phase and emitted structure are absent. `GridCell.occupied` exists and neutral-tech placement already respects it. | Add exact hut search. On qualification mark the cell occupied for later hut/tech gates, append a CABHUT emitted placement in final entity order, and append a `BridgeRepairHut` construction event at that point. Return deck success independently of hut result. | closure report §8; retail CABHUT data | Primary, fallback, no-cell, two-end, overlap-prevention, and deck-success-without-hut fixtures prove scan order, occupancy, placement order, and trace ordinals. |
| U2-14 | REQUIRED_FIX | MILESTONE-BLOCKING | Neutral-tech discarded constructor events | Each intended neutral building selects type and constructs before up to 100 attempts; one Scenario word is spent even when all attempts fail and object is destroyed. | `tech_buildings.rs:67-197` draws type before attempts but only returns successful `TechPlacement` rows. | Append a `NeutralTech` construction event immediately after native constructor qualification/type resolution and before placement attempts. On success bind outcome to emitted entity; on exhaustion retain `Discarded` with no invented cell. | closure report §9; lifecycle report §3.3 | Fully blocked map produces discarded events equal to intended constructed count and advances replay cursor; mixed fixture proves emitted/discarded order and no fabricated binding for discarded events. |
| U2-15 | REQUIRED_FIX | COMPOUNDING | Stable trace and entity identity | Trace is ordered across CABHUT then neutral-tech phases. Emitted outcome names stable final `MapEntity` index/type/cell; discarded has no binding. | `PipelineOutput` has only successful structure tuples (`pipeline.rs:115-123`); `GeneratedMap` has map/MapGen/waypoints/stages (`mod.rs:195-207`); emitter assigns entities only from final structure vector. | Introduce immutable `RmgConstructionTrace`/event/outcome types. Carry it through `PipelineOutput`, `GeneratedMap`, and `MapLoadInitial`. Ensure final generated structures use one stable order so each emitted event is assigned/validated against its exact `MapFile.entities` index. | closure report §9; lifecycle report §12.2 | Interleaved fixture asserts trace ordinals, final entity order, emitted indices/type/cell, discarded no-cell, and deterministic repeatability. |
| U2-16 | REQUIRED_FIX | MILESTONE-BLOCKING | Single launch Scenario replay and binding table | After match reseed and Full-Init/Fill prefix, consume one raw Scenario word per trace event. Emitted words enter binding table; discarded words drop. Generated projection consumes none. | `MapLoadInitial` carries only optional MapGen continuation (`init.rs:806-844`); bootstrap is created at `init.rs:1410`; no trace replay occurs; `construct_scenario` always uses fresh-map projection at `runtime.rs:349-490`. | Add a narrow `ScenarioBootstrapRng` replay method after terrain Fill borrow ends. It returns `GeneratedTechnoInitTable` from emitted events and advances the same owner later moved into Simulation. Thread optional generated bindings/source into `construct_app_scenario`/`construct_scenario`; call `spawn_generated_from_map_with_resolved` for generated maps. | lifecycle report §§12.1-12.2; current Rust | Exact trace + seed fixture independently computes words/poststate; projection leaves Scenario unchanged. Duplicate/missing/index/type/cell errors fail before first entity mutation. Post-map first draw matches independent continuation. |
| U2-17 | TEST_ONLY | COMPOUNDING | Existing generated init validation | Preconsumed generated words must validate entity index/type/cell and never draw again. | `GeneratedTechnoInitTable` and `spawn_generated_from_map_with_resolved` already implement duplicate/missing/unexpected/identity validation and preconsumed install at `world_spawn.rs:36-145,292-321`. | Preserve this owner and connect it; do not replace it with positional blind zipping or a second constructor mode. Extend table visibility only as narrowly as app/sim boundary requires. | P0 contract/implementation; current Rust | Existing validation tests remain; integrated generated-map test proves table is actually selected. |
| U2-18 | TEST_ONLY | COMPOUNDING | Preloaded Battle cursor transfer | Complete Battle/FFA start plans consume the house/start prefix from the launch seed and transfer the validated post-prefix cursor before terrain; final owner is not double-advanced. | `PreloadedBattleStartPlan` stores before/after logical state and cursor and `install_before_terrain` validates/transfers it (`scenario_bootstrap.rs:236-283,296-351,1760-1771`). | Preserve this transfer. Trace replay must occur after it and after terrain Fill; do not reconstruct/replace the owner. Stale comments that call preview map “retained gameplay map” must be corrected. | lifecycle report §12.1; current Rust | Combined preload+Fill+trace test matches one independently advanced Scenario stream and rejects a mismatched prestate. |
| U2-19 | TEST_ONLY | COMPOUNDING | Launch MapGen continuation | Simulation receives the logical MapGen state after launch generation, not preview generation. | Generated map carries `MapGenRngContinuation`; bootstrap installs whichever initial map path supplied (`build.rs:283-289`, `init.rs:1410-1413`). | Preserve transport, but only from launch `.SED` generation. | lifecycle report §3.7; current Rust | Distinct preview/launch test poisons preview continuation and proves Simulation receives launch continuation. |
| U2-20 | TEST_ONLY | COMPOUNDING | Waterfall terrain no-topology boundary | `BuildRiverBridge @ 0x0059E740` changes waterfall/water/level/scratch terrain only; no low overlay/data/raw flags/Tube/CABHUT/trace. | `bridge.rs:182-475` stamps terrain fields and has no overlay/density/trace write, but tests do not pin the full negative boundary and comments call RMG dormant. | Preserve code; add focused negative characterization. | closure report §10; current Rust | Deterministic success snapshot asserts allowed terrain changes and unchanged overlay, density, raw bridge flags, explicit tubes, structures, and trace. |
| U2-21 | DOC_ONLY | COMPOUNDING | Active/dormant naming boundary | RMG is conditionally active through stock Create Random Map/`.SED`; `BuildRiverBridge` is waterfall terrain despite its name; dormant helpers/TrainBridgeSet remain excluded. | `bridge_deck.rs:1-6` and `bridge.rs:1-10` call the whole RMG dormant; `bridge.rs` presents waterfall terrain as a crossing/bridge. | Correct module/API comments and test names to state conditional active RMG, low deck versus waterfall terrain roles, and exclusions. Do not rename verified behavior based only on stale Ghidra labels. | closure report §§1,10-11 | Diff/source review contains no dormant claim for active RMG, no TrainBridgeSet field, and no activation of excluded helpers. |

There are no `BLOCKED` or `UNKNOWN` rows. No active exact difference in this slice is deferred as a
residual; every `REQUIRED_FIX`, `TEST_ONLY`, and `DOC_ONLY` row must close before unit 2 can pass.

## Required Rust Changes

### 1. Setup and preview lifecycle

- `OfflineSkirmishRuntime` owns the process-continuing shell Scenario cursor and must also own or
  mediate persistent RMG setup options.
- `RandomMapSetupModalState` remains pure UI state. Its unset seed must be supplied from Scenario,
  not Main; option Randomize stays on `frontend_main_rng`.
- The generation worker remains MapGen-only and returns a `GeneratedMap` containing geometry plus
  ordered construction trace. When the main thread collects it, it replays the trace against the
  shell Scenario cursor exactly once. Do not move app state or Scenario RNG into the worker.
- Candidate/accepted preview state may retain a `MapFile`/preview only for setup/chooser/loading
  composition. It cannot satisfy `MapLoadInitial` or supply gameplay RNG state.
- Cancel must not roll back the shell Scenario cursor. Acceptance persists `.SED` options and the
  ordinary sentinel/preview state, then Start later regenerates.

### 2. RMG region and deck production

- Extend `ConnectorRegion` or add a narrow immutable region view containing id, level, flood
  classification, cell count, and ordered neighbor ids. Rust vectors may replace native heap
  vectors, but the full adjacency prepass and iteration order are semantic.
- `bridge_deck` becomes the exact low-deck owner. It may reuse `CarveCtx` or a narrower context,
  but it needs `grid`, `scratch`, `ids`, `blocks`, `rng`, playfield, output structures, and trace.
- Use the existing `TileBlocks` retail TMP provider for end stamps. Do not hard-code block
  dimensions/subcells from stock files.
- Use `GridCell.overlay`/`density` as the direct generated overlay/data payload and
  `GridCell.occupied` for CABHUT/neutral-tech admission order.
- Keep `end_area_is_placeable` and `pick_seed_cell` as preservation anchors. Correct the deck
  absorbable predicate rather than broadening `is_special_terrain`.

### 3. Construction trace and launch bootstrap

The required public-to-crate shape is semantic, not literal naming, but it must preserve these
fields:

```rust
pub(crate) struct RmgConstructionTrace {
    events: Vec<RmgConstructionEvent>,
}

pub(crate) struct RmgConstructionEvent {
    ordinal: usize,
    phase: RmgConstructionPhase,
    techno_type: String,
    outcome: RmgConstructionOutcome,
}

pub(crate) enum RmgConstructionPhase {
    BridgeRepairHut,
    NeutralTech,
}

pub(crate) enum RmgConstructionOutcome {
    Discarded,
    Emitted {
        entity_index: usize,
        cell: (u16, u16),
    },
}
```

- Ordinals are contiguous production order and checked in debug/tests or by constructor.
- Discarded events have no cell/index. Do not invent a final coordinate for a deleted neutral
  building.
- Emitted entity indices are finalized against `MapFile.entities` after all generation phases.
  A Rust-native intermediate placement id may be used during pipeline execution if emit assigns
  the final stable index deterministically.
- `MapLoadInitial` distinguishes fixed versus generated source and carries launch trace only for
  generated maps.
- After `bootstrap_rng.install_preloaded_battle_plan` and `terrain_draws`, drop both draw wrappers,
  replay the trace on bootstrap Scenario, and obtain the `GeneratedTechnoInitTable`.
- `construct_scenario` receives optional generated bindings. Generated projection takes the
  validated preconsumed path; fixed maps retain ordinary fresh construction.
- `ScenarioBootstrapRng::into_simulation` still transfers the same Scenario/Main/MapGen owners.
  No second launch-seeded Scenario cursor or preview cursor may replace them.

### 4. Determinism and architecture constraints

- The trace affects deterministic constructor-word assignment and post-load Scenario cursor. Its
  ordering and emitted identity mapping must be included in deterministic tests; the persistent
  installed word is already snapshot/hash state from P0.
- `map::rmg` remains pre-play map construction and must not depend on `sim`. Define trace vocabulary
  in `map::rmg` or a dependency-neutral map type; app/sim translates events to
  `GeneratedTechnoInit` during replay.
- `sim` remains the only live gameplay RNG owner. App may drive the narrow bootstrap replay API but
  may not extract or mutate raw cursors.
- Do not reproduce native globals, vtables, object allocation, or raw Cell pointers. Exact order,
  results, draws, and bytes are the contract.
- Generated direct overlays must be tagged by source/entry path so ordinary fixed-map overlay
  reconstruction stays intact.

## Acceptance Tests

Every scenario below is automated and maps to table rows.

1. **Setup seed/persistence matrix (U2-01):** initialize OfflineSkirmishRuntime and both RNG
   references; open with unset options, cancel, reopen, edit/reopen. Assert first open takes one
   Scenario ranged draw, Main none, later opens no seed draw, and all persisted options survive.
   Randomize advances Main only.
2. **Preview generation-count matrix (U2-02):** instrument generator entry. Repeated Generate
   starts from identical seed cursor, Use Map with preview adds zero runs, Use Map without preview
   adds one.
3. **Preview trace/Cancel (U2-03):** use a deterministic trace with emitted CABHUT and discarded
   tech event. Poll completion, then Cancel. Assert shell Scenario advances exactly two raw words,
   remains advanced on reopen, preview file/product exists, and `.SED`/selection remain unchanged.
4. **Preview-versus-launch poison test (U2-04,U2-19):** accept a preview, replace its in-memory map,
   trace, and MapGen continuation with poison values without changing `.SED`, then launch. Assert
   regenerated map/trace/constructor words/MapGen continuation equal a fresh `.SED` launch and
   contain none of the poison.
5. **Direct `.SED`/UI convergence (U2-05):** same options and match seed through direct `.SED` and
   accepted UI launch produce identical MapFile, trace, MapGen logical state, generated words, and
   post-load Scenario state.
6. **Flood-region graph matrix (U2-06):** ordered graph covers non-flood, one water endpoint,
   neighbor count `1` with cell count `50`, count `2`, cell count `51`, mismatched levels, and one
   valid three-neighbor case. Assert exact pair list/order and no duplicate calls.
7. **Integrated seed picker (U2-07):** independent manual draw walk predicts wrong-region and
   unstamped rejections, accepted seed, and next MapGen word; actual placer matches.
8. **Axis/length matrix (U2-08):** synthetic strips cover NS-only, EW-only, NS shorter, EW shorter,
   equal tie, wrong endpoint region, special/playfield stop, and attempts 24/25/49/50/174/175/199
   around strict threshold bands.
9. **Absorbable family boundaries (U2-09):** exact base boundaries for WaterSet 14, ShorePieces 42,
   four waterfall spans 4; all waterfall subtiles give same verdict; BridgeSet, WoodBridgeSet,
   cliffs, base-1, and exclusive end refuse.
10. **Direct deck golden rectangles (U2-10):** one EW and one NS success assert every overlay id,
    density byte, untouched tile/level, no Scenario effect, and no secondary Mark expansion during
    generated projection.
11. **End-area preservation/integration (U2-11):** retain existing pure tests and assert actual
    placer passes EW `6x6` and NS `7x6` rectangles with overlays ignored.
12. **Four-end coin/stamp matrix (U2-12):** for each end cover area-false, coin-false, coin-true.
    Assert conditional draw count, tile base/offset, anchor, TMP subcells/slopes, scratch `-1`, and
    unchanged level.
13. **CABHUT matrix (U2-13):** primary first cell, primary later cell, primary fail/fallback first,
    both fail, two ends, and overlap with prior hut. Assert inclusive Y/X order, `occupied`, Neutral
    CABHUT entity order, one event per qualifying cell, and deck success with zero huts.
14. **Neutral-tech discarded/mixed trace (U2-14,U2-15):** all-blocked map and mixed-success map
    assert constructor event exists before attempts, discarded has no binding, emitted final index
    maps to exact type/cell, ordinals interleave after any CABHUTs, and repeated run is identical.
15. **Bootstrap replay oracle (U2-16,U2-18):** independently advance one `SimRng` through Battle
    house/start prefix, terrain Fill fixture, and an emitted/discarded trace. Rust bootstrap final
    logical state and emitted low words match; wrong prestate refuses before replay.
16. **Generated projection validation (U2-16,U2-17):** correct table installs exact words and draws
    zero; duplicate, missing, unexpected, type mismatch, and cell mismatch fail before first entity
    or occupancy mutation. Fixed-map projection still spends fresh words.
17. **Post-map continuation (U2-16):** after generated projection, the first known Post-Map consumer
    reads the exact next independent Scenario value, proving no hidden reseed/double draw.
18. **Waterfall negative characterization (U2-20):** deterministic successful terrain crossing may
    change tile/subtile/slope/level/scratch, but overlay, density, raw bridge flags, explicit tubes,
    structure output, and construction trace remain byte-for-byte unchanged.
19. **Exclusion/source boundary (U2-21):** source scan/test API exposes no `TrainBridgeSet`, no calls
    to excluded helpers, and no comments labeling stock Create Random Map/`.SED` RMG dormant.

Focused validation commands must all include `--lib`. Suitable filters include the narrow owners
for `rmg::phases::bridge_deck`, `carve_driver`, `tech_buildings`, `rmg::build`, random-map shell,
loading bootstrap, and generated spawn. Check `Get-Process cargo,rustc` before each command. The
full `cargo test -p vera20k --lib` is reserved for the one final bridge-wide certification after
all closure units pass.

## Known Non-Requirements

- Do not add `TrainBridgeSet`, TS rail-bridge generation, or OpenTS-only behavior.
- Do not activate `0x005A5020`, `0x005A6510`, `0x005A82E0`, `0x005A91E0`, or
  `0x005A1E10`.
- Do not make `BuildRiverBridge @ 0x0059E740` create low overlays, bridge flags, Tube records,
  CABHUTs, or trace events.
- Do not use six water variants for the low-deck validator; that narrower generator-write span is
  a different predicate.
- Do not reuse `TileIds::is_special_terrain` for `0x004865D0`; its sub-tile exceptions and extra
  families are semantically different.
- Do not share one sweep between deck and end validators.
- Do not draw an end coin on area failure or randomize CABHUT cell choice.
- Do not make CABHUT success a deck commit gate.
- Do not reconstruct trace events from final structure count.
- Do not give discarded events an invented cell/entity index.
- Do not use MapGen for Techno constructor words or Scenario for deck geometry.
- Do not send the shell Scenario owner to the worker thread; replay the pure returned trace.
- Do not carry preview Scenario, MapGen continuation, map, or generated init bindings into play.
- Do not construct a second gameplay Scenario cursor after preload/Fill or replace the validated
  `PreloadedBattleStartPlan` transfer.
- Do not replay fixed-map `OverlayClass::Mark` over a generated deck.
- Do not change loading chrome/text/frame order or preview pixel composition in this slice.
- Do not refactor land ramps, shore/water generation, or runtime bridge systems as collateral work.
- Do not guess allocation/OOM recovery behavior; exclude it as non-deterministic system failure.

## Blockers and Follow-Ups

**Blockers:** none.

All material active behavior, data identities, current Rust owners, and acceptance outputs are
proven. No `/re-investigate`, `/re-swarm`, `/disparity-scan`, or architecture brainstorm is needed
before implementation. The approved bridge design plus this contract is sufficient to implement
the bounded unit directly. `/write-plan` is optional only if the goal owner later explicitly asks
for a task-by-task plan; it is not a parity blocker.

After implementation and focused validation, give a fresh read-only critic this contract, both
primary reports, retail-data evidence, exact diff, and literal test output. A material finding
reopens the unit. Fix its largest finding and submit the entire updated bundle to a new fresh
critic, including recheck of prior fixes, until one passes without unresolved or approximate
behavior.

## Source Ledger

- `docs/research/bridges/00-system-models/RMG_LOW_BRIDGE_DECK_CABHUT_ACTIVE_RETAIL_CLOSURE_GHIDRA_REPORT.md`
- `docs/research/bridges/00-system-models/RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/MAPGEN_SAME_PROCESS_LIFECYCLE_BRIDGE_CALLER_RECONCILIATION_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/RMG_BRIDGE_CONNECTOR_PASS_0058EF10_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/RMG_MODE34_WATER_BRIDGES_TECH_GHIDRA_REPORT.md`
- active retail YR theater/rules/art/TMP data under the configured retail install
- `src/map/rmg/phases/bridge_deck.rs`
- `src/map/rmg/phases/carve_driver.rs`
- `src/map/rmg/phases/bridge.rs`
- `src/map/rmg/phases/tech_buildings.rs`
- `src/map/rmg/tiles.rs`
- `src/map/rmg/pipeline.rs`
- `src/map/rmg/build.rs`
- `src/map/rmg/emit.rs`
- `src/map/rmg/mod.rs`
- `src/app/shell_random_map.rs`
- `src/app/shell_skirmish.rs`
- `src/ui/skirmish_shell/state/random_map_setup.rs`
- `src/app/frontend/skirmish_session.rs`
- `src/app/loading/pump.rs`
- `src/app/loading/init.rs`
- `src/sim/scenario_bootstrap.rs`
- `src/sim/runtime.rs`
- `src/sim/world/world_spawn.rs`
- `src/sim/game_entity.rs`
- OpenTS files listed above, navigation only

## Ghidra Annotation Candidates

No annotation was applied. Deferred candidates from the exhaustive report are:

| Address | Current metadata issue | Proposed annotation | Exact proof | Status |
|---|---|---|---|---|
| `0x004865D0` | stale bridge/overlay-style role name | exact water/shore/waterfall tile-family predicate | function body is a leaf tile-id range test; no overlay read | DEFERRED, no sync authority |
| `0x0058F2C0` | incomplete role/order documentation | active type-3/type-4 low-deck placer, EW tie, strict length bands, direct stamp | active caller plus full body/callsite review | DEFERRED, no sync authority |
| `0x005A7440` | easy to conflate with deck validator | exact end-area predicate; all low-deck overrides zero | four live callsites and body | DEFERRED, no sync authority |
| `0x005904B0` | constructor/scan side effects underdocumented | inclusive CABHUT scan, constructor-before-Unlimbo, Scenario word | body plus Building/Techno constructor chain | DEFERRED, no sync authority |
| `0x0059E740` | inherited bridge name misleading | waterfall/river terrain shaping, no runtime topology | complete field-write/callee census | DEFERRED, no sync authority |

## Handoff

Implement closure unit 2 on `feature/bridge-movement-parity`. Preserve the already-correct seed
picker, end-area predicate, Battle cursor-transfer contract, MapGen continuation transport, and
`GeneratedTechnoInitTable`. Close every table row with focused `--lib` validation and one or more
coherent commits. Do not run the full `cargo test -p vera20k --lib` yet. Then begin the required
fresh-critic loop; unit 3 may not begin until a new critic passes unit 2 with no residual,
unverified, approximate, or missing behavior.
