---
title: Disparity Scan - Authored Overlay Finalization and Fixed-Map Low Bridge Mark (Transaction 3)
date: 2026-08-31
scope: Active-retail OverlayPack/OverlayData loading, fixed-map low procedural Mark, and both load-time Recalc boundaries
methodology: bounded research inventory, direct Rust verification, active-YR reports with parent cold checks, retail-data census
---

# Disparity Scan - Authored Overlay Finalization and Fixed-Map Low Bridge Mark (Transaction 3)

## Scope and evidence basis

This scan is deliberately bounded to bridge transaction 3. It covers the physical-source,
fresh-context, and signed `NewINIFormat > 1` activation boundary; the single synchronous
`OverlayPack` y-outer/x-inner transaction; ordinary, high, and low Mark dispatch; the persistent
fallback cell; `OverlayDataPack`; the pre-Terrain and post-object whole-map Recalc boundaries; the
minimum finalized identity/state/count payload; generated `.SED` no-authored-Mark arm plus its staged
generator Recalc/animation lifecycle; and the runtime/presentation handoff. It is the primary
implementation scan for GSI-04.13 / BR-M05, a shared high-load
contribution to GSI-04.12 / BR-M04, and the initial load-time Road contribution to GSI-04.15 /
BR-M11. It does not close later high-bridge topology, runtime low mutation, or positive Tube/Road
ownership.

The decisive active-binary sources are:

- `AUTHORED_OVERLAYPACK_INLINE_TRANSACTION_REINVESTIGATION_GHIDRA_REPORT.md`, which proves the
  exact reader, filters, inline Mark order, high/ordinary writes, tactical dirty, OverlayData,
  both real-cell sweeps, tile-animation lifecycle, and active retail reachability;
- `AUTHORED_OVERLAY_EPHEMERAL_OBJECT_FINALIZATION_REINVESTIGATION_GHIDRA_REPORT.md`, which proves
  real Overlay allocation/base-registry/ID/Overlay-registry order, direct base Unlimbo, child-Anim
  interleaving, common-success versus wall/slope lifetimes, the unconditional shared live drain,
  non-presentation survivors, and prefix-relative Tube/Overlay identity;
- `AUTHORED_OVERLAY_WALL_SCENARIOINIT_ACCEPTANCE_REINVESTIGATION_GHIDRA_REPORT.md`, which proves
  authored ScenarioInit reachability, successful wall stamp/cleanup/connectivity/common-tail order,
  compact active-retail wall IDs and census, and the retained real-cell wrapping blocker-neighbor
  count plane that final identities cannot reconstruct;
- `AUTHORED_MARK_LOAD_CONTEXT_SOURCE_PROVENANCE_REINVESTIGATION_GHIDRA_REPORT.md`, which proves
  physical source, fresh-load family, Scenario-cursor, generated, replay, and restore boundaries;
- `OVERLAYPACK_SHARED_DUMMY_FINAL_RECALC_FIELDS_REINVESTIGATION_GHIDRA_REPORT.md`, which proves the
  signed fixed-stride lookup, one persistent dummy, dummy field surface, Recalc early return, and
  minimum finalized payload;
- `AUTHORED_TIBERIUM_GERMINATE_SIDE_EFFECT_REINVESTIGATION_GHIDRA_REPORT.md`, which proves the
  exact ordinary Land-code-5 density transaction, fixed real/dummy neighbor reads, and its
  no-RNG/no-queue boundary;
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md`, whose active `CellClass::Get_Tiberium_Value @ 0x00485020`
  proof fixes authored argument-0 value-only accounting as `(OverlayData + 1) * Value`;
- `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`, whose 2026-07-24 retail live recheck pins the same
  function's non-resource zero return and exact recognized-resource multiplication;
- `LOADING_FULL_INIT_PROGRESS_SEQUENCE_AFTER_00552D60_GHIDRA_REPORT.md` and
  `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md`, which prove the authored Full_Init return store at
  `MapClass+0x134` / `0x0087F91C` and the cell-array teardown reset;
- `TERRAIN_ATTACHED_ANIM_LOAD_LIFECYCLE_SIDE_EFFECTS_REINVESTIGATION_GHIDRA_REPORT.md`, which proves
  per-Mark versus whole-sweep creation, ID/registry/RNG/Middle ordering, immediate scalar deletion,
  sound/owner/occupancy exclusions, recreation, and the generated multi-phase Recalc interleaving;
- `RMG_PREVIEW_ANIM_BUILDING_IDENTITY_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`, which proves the
  active preview Set_Defaults/manual branch, per-Generate native-ID reset, exact replacement cleanup,
  Cancel/re-entry persistence, separate collision-free handles, and fresh-Full_Init `+10,000`
  negative/positive boundary;
- `FULL_INIT_AND_PREVIEW_NATIVE_ID_PREFIX_REINVESTIGATION_GHIDRA_REPORT.md`, which proves every
  branch-specific preview and fresh-Full_Init constructor stream through `C_saved`, the set-from-
  snapshot wrapping map-read transform, custom-theater window, Tube failure semantics, first-Overlay
  formula, and growth/spread queue lifetime across preview, Cancel, re-entry, and accepted `.SED`;
- `INITCELLATTRIBUTES_TAG_LINE_LIGHTING_TAIL_REINVESTIGATION_GHIDRA_REPORT.md`, which proves the
  two-pass line-mask producer, full map-bounds/shared-dummy behavior, active Foot consumer and scratch
  quirk, official retail reachability, ordinary-versus-sentinel light recomputation, opaque pointer-
  slot classification, and post-Recalc wall-owner ordering/ownership split;
- `RMG_TERRAIN_SHAPING_CORE_GHIDRA_REPORT.md` and
  `SKIRMISH_RANDOM_MAP_GENERATOR_00598960_GHIDRA_REPORT.md`, whose active generator tail establishes
  growth/spread queue initialization before terminal `InitCellAttributes(1)` and radar work.

The parent independently cold-checked every load-bearing call site and instruction family named by
the initial reports, including the format default/gate, both Full_Init sweep boundaries, iterator
order, dummy guard, four low write families, high/ordinary writes, and Recalc's
LAT/zone/cache/animation work. The focused germination and terrain-animation reports directly close
the newly exposed subordinate side effects. The reports also retain the settled three low-Mark
investigations from 2026-08-30. Retail
data confirms the four active high ids (`0x18`, `0x19`, `0xED`, `0xEE`), format-4 authored pack
reachability, and zero low triggers in a bounded 385-payload shipped-map census. The zero-trigger
result is a content fact, not evidence that the active custom/editor-compatible mechanism may be
omitted.

The current Rust comparison is direct against worktree `HEAD` / `origin/main`
`50e4b7ba4732fd3fb48e5b819e1abc55327ec557`, preserving PR #170 and merged PR #196.
Intervening PR #197 added only Team-AI INI tests and did not touch a transaction-3 owner.

## Living candidate inventory

| Candidate mechanism | Active-YR evidence status | Current transaction disposition |
|---|---|---|
| Physical source, fresh family, and format activation | Verified | Implement; keep the three axes independent |
| One 512x512 y/x decoded-row transaction and native filters | Verified | Implement as one map-owned transaction |
| Ordinary/high identity-state writes and high anchor restore | Verified | Replace split late high-only projection |
| Base-object tactical dirty before derived dispatch | Verified | Implement one intent per accepted object; no generated-body repeats |
| Authored OverlayClass allocation/registries/Mark/UnInit/shared-drain and slope-survivor lifecycle | Verified, including direct base Unlimbo, child Anim interleaving, authored-wall common-success reachability, generic counter-zero rejection exclusion, unconditional generated/body-absent drain, non-presentation slope survival, and absolute prefix-relative IDs | Implement one lightweight load-object registry/queue owner from the closed OQ-33/OQ-34 evidence; authored walls use the common queued tail |
| Authored `Wall=yes` ScenarioInit success, cardinal connectivity, owner `-1`, and blocker-neighbor count plane | Verified, including native order, signed fixed-map real aliases, wrapping `u8`, later low-body overwrite retention, active retail census, runtime removal conditions, shared-dummy traversal, pointer-expiry ordering, and sale exclusion | Plane/persistence/pathfinding and runtime count/dummy/navigation/radar owners are implemented and focused-validated on the transaction branch; fresh criticism remains open, as do production authored-row dispatch/consumed-once installation and the unrepresented native non-entity pointer-listener roster |
| Low fixed/search/body algorithm and exact `3*L` raw RNG | Verified by settled reports and cold checks | Implement without reordering or ranged substitution |
| Persistent dummy, signed `i16 y*512+x` lookup, overlay/state fields | Verified | Extend existing shared identity; do not add derived fields |
| Independent OverlayData traversal | Verified | Execute after the one Mark transaction; real cells only |
| First anti-diagonal Recalc sweep | Verified | Implement after data and before Terrain/Technos |
| Finalized real identity/state/authored-blocker-count handoff | Verified | Add one consumed-once payload; keep derived terrain fields in terrain and never reconstruct authored walls from final identities |
| Authored per-Mark/first-sweep animations, post-Terrain growth/spread queue boundary, and post-object second sweep | Verified | Initialize queues from post-Terrain/pre-object live state, retain them through later occupancy, then implement exact scalar-delete, value-only accumulation, unlatch, and recreation side effects without a rebuild |
| Generated `.SED` direct materialization and no authored Mark | Verified | Preserve direct identity/state output; do not replay authored pack Mark |
| Generated synthetic-Full_Init and generator-native staged Recalc/animation/resource lifecycle | Verified, with staged synthetic eligible set content-dependent | Preserve/capture actual phase history, pre-final queue init, and final arg-1 germination/local-value pass without rebuilding queues; final cells and a flat constructor trace are insufficient |
| RMG preview common generator Recalc/Anim/Building phases and shell identity lifecycle | Verified: argument-1 Set_Defaults/manual setup, active registries/sound, reset-before-cleanup, exact same/changed/missing-key lifetime, Cancel/re-entry persistence, accepted-launch cleanup | Implement a process-shell preview owner and independent native-ID/runtime-handle identities; OQ-32 is resolved |
| Native-ID prefix before the first preview generator or authored Overlay object | Verified: exact campaign/noncampaign constructor-stream formulas, preview matching/changed/missing branches, set-from-snapshot wrapping `+0x2710`, custom-theater window, Tube allocation/fault semantics, and empty shared-queue reader prestate | Implement the consumed-once shared cursor/Tube-constructor prefix and preview lifetime; do not promote Tube topology |
| Post-final-Recalc current-wall owner reconstruction | Verified and already represented by the GSI-04.07 wall-owner helper | Preserve/reuse it after all final-current Recalcs; an output-equivalent global pass is valid and no second owner is added |
| Runtime OverlayGrid and presentation atlas visibility | Verified Rust dependency | Consume finalized identities/data rather than raw pack rows |
| Low Mark synthesizing Tube topology | Active-YR negative | Exclude; `[Tubes]` is a separate earlier reader |
| OpenTS rail/TS-only overlay semantics | Active-YR negative | Exclude; OpenTS remains navigation only |
| AttachedTag event `0x19`/`0x1A` row/column accelerator bits (`0x100000`/`0x200000`) | Verified generic trigger-line consumer, not physical bridge topology or zones | Exclude from bridge facts/topology; transaction 3 exposes the ordered clear/restamp seam, while the generic trigger owner retains actual bits/consumers |
| Save/restore dummy serialization | Unchecked outside this corridor | Defer to transaction 21; do not infer |
| Cache-B byte `+9` semantics | Unknown, nonblocking | Keep open outside payload/transaction decision |
| Per-cell LightConvert/ZAdjust recomputation in `InitCellAttributes` | Verified active presentation side effect; only sentinel ids keep the neutral defaults | Route semantic output to transaction 20; transaction 3 executes one invalidation/routing slot and must not leak stale preview state |
| `Cell+0x30 = 0` in `InitCellAttributes` | Persisted/swizzled pointer-shaped lifecycle slot verified; live meaning/consumer unresolved | Route with OQ-19 to transaction 21; do not invent a bridge field or numeric scratch |

Dependencies are the merged P0 Techno-constructor RNG owner and P0-R1 stock-offline Scenario
prefix; transaction 3 borrows their one post-Fill Scenario cursor but does not reopen their
mechanisms. GSI-02.09 and GSI-17.01/04/07 provide routing/lifecycle context, not substitute bridge
owners. BR-M04 remains open for transaction 4, BR-M11 for its later Road mutation work, and the
positive GSI-04.15 Tube mechanism for transaction 5.

## Summary

- 25 candidate mechanisms inventoried: 18 positive transaction-3 candidates, 3 verified
  exclusions, and 4 later-owner/unknown facts
- all 18 positive active-YR mechanisms are materially verified for this transaction
- 13 verified Rust gaps
- zero transaction-3 candidates await native verification; OQ-34's complete preview/fresh-load
  native-ID prefix is closed
- 10 verified matches or reusable partial matches, including the existing wall-owner reconstruction
- synthetic-Full_Init eligibility remains implementation-acceptance gating unless captured from
  actual staged state; dummy persistence, Cache-B, presentation LightConvert ownership, and
  `Cell+0x30` meaning remain later/nonblocking rather than transaction-3 evidence blockers

The gaps are one dependency-coherent transaction because the same live cell state and Scenario
cursor cross all of them. This is a disparity snapshot, not a row-completion certificate: GSI-04.12,
GSI-04.13, and GSI-04.15 remain open until implementation, independent criticism, merge, and the
later owning transactions/reverse audit are complete.

## Verified gaps

### CRITICAL priority

**G1. Rust has no exact three-axis source/context/format admission boundary**

- **Active-YR evidence:** successful non-`.SED` fresh loads reach Full_Init regardless of Loose vs
  MIX physical storage. `Read_INI_Basic` writes default `0`, and only signed
  `NewINIFormat > 1` enables the two pack bodies. Campaign, LAN, WOL-state-2, and replay relaunch
  have distinct prefixes but can all Mark; stream restore never enters Full_Init; generated `.SED`
  calls synthetic Full_Init with format `0`, then directly materializes deck cells and completes its
  independently staged generator finalization. Format `<=1` suppresses only pack bodies: every fresh
  Full_Init still owns its family-dependent pre-map native-ID prefix, map-read transform, successful
  Tube IDs, ungated Recalc Anim IDs, and later objects.
- **Rust state:** `LoadedMapSource` already records `Loose`, `Mix`, `Generated`, and
  `LegacyFallback` (`src/app/frontend/list_maps.rs:34-50`), and `BasicSection` retains
  `new_ini_format` (`src/map/basic.rs:32,80`). Nevertheless production selects
  `OverlayLoadSource::GeneratedMaterialized` from
  `generated_construction_trace.is_some()` (`src/app/loading/init.rs:1879-1883`), while the
  headless path always selects `Authored` (`src/headless_scenario.rs:102-116`). The two-state map
  enum (`src/map/resolved_terrain.rs:55-58`) carries neither a typed fresh family nor the format
  gate or native-ID prefix receipt, and the loaders distinguish only the Mark-active untyped case.
- **Required delta:** derive physical overlay source from `LoadedMapSource` exactly once; carry an
  orthogonal typed fresh-load family plus the closed OQ-34 consumed-once native-ID prefix receipt; read the
  signed format value at the map transaction. Every gameplay-equivalent fresh Full_Init requires
  both even when format-inactive; only the pack bodies and Mark draws skip. Restore and generated
  provenance remain explicit no-Mark arms. Missing generated phase transport must fail after
  selecting no-Mark rather than reconstruct history or fall back to authored processing. A named
  untyped pure-map/no-live-effects diagnostic is explicitly non-parity.
- **Acceptance:** source/format/family matrix fixtures cover Loose, Mix, Generated with missing
  trace (no authored Mark followed by explicit phase-transport error), campaign/network/replay,
  stream restore, typed format absent/1/2/4, untyped headless/generic rejection before any ID/draw at
  both absent/1 and 4, and a separately labeled non-parity pure-map diagnostic.
- **Verdict:** MISSING / DRIFT.
- **Priority rationale:** every authored load crosses this boundary, and most shipped multiplayer
  maps use format 4. A wrong arm either deletes a real Mark transaction or replays one over generated
  material, shifting all later Scenario consumers whenever a low trigger is present.

**G2. The one native inline OverlayPack transaction is split across incompatible Rust owners**

- **Active-YR evidence:** after decode, native visits y `0..511`, x `0..511`; filters and constructs
  each accepted ephemeral object, runs base Mark/tactical dirty, dispatches ordinary/high/low work,
  and completes any common Recalc before the next row. High, low, and ordinary rows are interleaved.
- **Rust state:** parsing collapses the pack to non-`0xFF` `OverlayEntry` rows
  (`src/map/overlay.rs:129-176`). `ResolvedTerrainGrid::build_inner` pre-indexes those rows
  (`src/map/resolved_terrain.rs:2076-2082`), derives overlay effects during row-major cell
  construction, and later runs a separate raw-entry bridge loop (`:2442-2470`). Production then
  decodes/filter-projects them again through `OverlayGrid::from_native_overlay_packs`
  (`src/sim/overlay_grid.rs:206-258`; app call sites `src/app/loading/init.rs:1117,1987`). No owner
  represents the native single-row transaction.
- **Required delta:** one map-owned authored routine consumes the ordered decoded bytes/entries,
  native registry/art/game-mode/radar admission, and one live cell surface. Every accepted row
  completes before the next. Remove all second-decoder/filter authority from the runtime handoff.
- **Acceptance:** an adversarial interleaved fixture puts low, high, ordinary, rejected, and later
  overwriting rows in native pack order and records exact row/Recalc/dirty events.
- **Verdict:** ARCHITECTURE DRIFT.
- **Priority rationale:** the mismatch affects every format-active authored pack; common maps can
  misproject high/ordinary attributes, while custom low-trigger maps additionally change topology
  and RNG order.

**G3. Fixed-map low procedural Mark and its exact Scenario transaction are absent**

- **Active-YR evidence:** trigger ids `0x7A..0x7D` and `0xE9..0xEC` execute the settled fixed,
  search, and body tables inline. Successful length `L` bodies consume exactly `3L` raw
  `Scenario::Next() & 3` words; fixed/search/no-op/failure arms consume zero. Every fixed/body write
  stores signed-dword identity and byte state then calls Recalc; missing coordinates share one
  persistent dummy. Low Mark creates no Tube.
- **Rust state:** the late resolved-terrain loop dispatches authored high stamps only
  (`src/map/resolved_terrain.rs:2442-2470`), and `OverlayGrid::from_native_overlay_packs` performs
  only generic identity admission/Recalc (`src/sim/overlay_grid.rs:206-258`). Neither accepts the
  post-Fill Scenario cursor or implements low tables/search/body writes.
- **Required delta:** borrow the existing one Scenario owner through a raw-only callback after Fill
  and before authored Technos; implement the settled low algorithm in the inline map transaction.
  `map` must not depend on `sim`, clone/reseed the cursor, or use a ranged helper.
- **Acceptance:** full logical cursor states bracket prefix, Fill, Mark, later pack rows, and first
  Unit/Aircraft/Infantry/Structure construction. Geometry fixtures cover every success/no-op/failure,
  occupied write, edge miss, and exact-opposite search arm.
- **Verdict:** MISSING.
- **Priority rationale:** shipped payloads currently contain no trigger, so ordinary retail-map
  frequency is zero in the bounded census; the first active custom/editor trigger deterministically
  changes an entire crossing and every later Scenario result, making the mechanism release-blocking
  for compatibility rather than frequently visible stock content.

**G4. Rust does not model the exact ordinary/high writes or transaction-level tactical dirty**

- **Active-YR evidence:** ordinary Mark writes identity and state `0`. For Land code 5 it writes
  state `1`, derives the receiver TiberiumClass, performs exactly eight real-or-persistent-dummy
  neighbor lookups in `N,NE,E,SE,S,SW,W,NW` order, counts the same derived class without reading
  neighbor state, and writes receiver density from `[0,1,3,4,6,7,8,10,11]`; argument `0` draws no
  RNG. Land-5/non-Tiberium returns before those lookups and retains state `1`; a flagged image-range
  miss maps to class `0`. It then writes `0xFF` for crates. High ids
  `0x18/0x19/0xED/0xEE` alone save anchor state, run their
  setters with temporary anchor state `0/9` through common Recalc, and restore only the anchor byte;
  structural/neighbor effects remain. Every accepted ephemeral object dirties tactical state once
  in base `ObjectClass::Mark`, even when derived Mark later slope-rejects; generated body cells do
  not repeat it.
- **Rust state:** the late bridge loop writes a bridge-facts identity, conditionally stamps high
  structure, and later copies data (`src/map/resolved_terrain.rs:2442-2481`). The sim pack loader
  approximates ordinary state and Recalc (`src/sim/overlay_grid.rs:219-258`) but has no exact high
  save/temporary/Recalc/restore window or object-level load dirty event.
- **Required delta:** make these writes, the complete zero-argument germination algorithm, and one
  dirty intent explicit in the same inline map transaction, preserving successful-construction and
  slope-rejection timing. Germination rewrites only receiver state, has no Recalc/dirty/direct
  queue/bitmap/heap effect, and ignores its credit return. A later OverlayData body wins; without
  data, the computed density must survive into the later tiberium-queue rebuild.
- **Acceptance:** exact pre/post bytes and neighbor facts for all four high ids; Land-5
  exact N..NW order, same-TiberiumClass density table, mixed-id/state neighbors, Land-5/non-Tiberium
  early return, range-miss class-0 fallback, source-order overwrites, repeated dummy misses/aliases,
  no-data `2x2` y/x result `[0,1;3,4]`, later-data override, zero RNG/Recalc/dirty/queue; dummy final
  coordinate equal to the last true N..NW miss with later real hits preserving the stamp and no
  helper dummy-identity/state write; queue predicates consuming rather than recomputing density (all
  four growth-eligible, state `0` spread-density-ineligible and `1/3/4` eligible with other gates
  held constant); authored `InitCellAttributes(0)` making zero second germination calls; crate-last
  ordinary rows; art/crate/radar rejects; slope reject after one dirty; helper argument `0` with no
  optional bridge-counter increment; and zero per-generated-cell dirty.
- **Verdict:** DRIFT / MISSING.
- **Priority rationale:** ordinary/high rows occur on many authored maps, so identity/terrain drift
  is common; the tactical-dirty count is load-only but is deterministic and cannot be assigned to
  a later batch without changing native side-effect order.

**G5. OverlayData and the first global Recalc execute against the wrong state and at the wrong time**

- **Active-YR evidence:** within the format gate, positive OverlayData independently overwrites the
  state byte of every admitted real radar-diamond cell, including identity-empty/rejected rows. The
  first whole-map Recalc is outside the gate, follows the reader, runs before Terrain/Technos, visits
  exactly `H*(2W-1)` cells in playable-width anti-diagonals, and performs live identity validation,
  LAT/slope, CliffBack Land, zone/cache, and conditional tile-animation work. Recalc never reads the
  state byte.
- **Rust state:** `lat::apply_load_recalc_sweeps` runs both nominal sweeps over the raw/final cell
  vector before overlay admission (`src/map/resolved_terrain.rs:2033`; `src/map/lat.rs:333-375`) and
  iterates linear vector order, not the native iterator. Overlay data is copied much later into only
  bridge facts (`src/map/resolved_terrain.rs:2472-2481`) and separately into OverlayGrid
  (`src/sim/overlay_grid.rs:246-257`). The full live Recalc projection therefore never sees the
  finalized inline identities.
- **Required delta:** separate Fill materialization from post-pack Recalc. Apply data once, then
  run one exact anti-diagonal real-cell sweep over live overlay/terrain state before Terrain.
- **Acceptance:** format-inactive/absent-pack/data-only cases, exact coordinate trace/count,
  interleaved identity/data, live LAT neighbor ordering, CliffBack Land, zone/cache, identity clear,
  and no false assertion that data drives Recalc.
- **Verdict:** ORDERING / STATE DRIFT.
- **Priority rationale:** every fresh map runs the sweep and most authored multiplayer maps run the
  pack bodies. Wrong LAT, slope, land, zone, cache, or animation state can affect movement and
  rendering on ordinary stock maps even without a low trigger.

**G6. The authored post-Terrain queue boundary and post-object InitCellAttributes/animation lifecycle are absent**

- **Active-YR evidence:** after `[Terrain]`, native initializes growth queues and then spread queues
  from the then-current live real-cell resource state, before Units, Aircraft, Infantry, Structures,
  or Smudge can add occupancy. It retains that queue snapshot without a later rebuild. After those
  object sections, `InitCellAttributes(0)` scans current live Anim order and immediately scalar-deletes every
  terrain-marked `Anim+0x197`, then clears each cell latch `0x20000` immediately before its second
  equal-count/equal-order Recalc, recreating the surviving animations and refreshing object-derived
  attributes. Before that boundary, an eligible animation can be constructed during per-Mark Recalc
  in decoded source order, with the first whole-map sweep constructing remaining cells in
  anti-diagonal order. Base Object registration, fresh native numeric ID on an independent runtime
  handle, sound-handle initialization, and
  Anim-registry insertion precede optional RandomRate Scenario consumption. Reveal/Unlimbo and
  Logic/live registration add no entity/cell occupation; delay-zero Middle and optional StartSound
  precede producer marker/ZAdjust/latch writes, while `Start` is conditional on raw SHP
  frame-count/2 `+0x298 == 0`. Main RNG is absent and all 20 active stock TileAnim rows have zero
  RandomRate. Direct deletion compacts registries and releases/detaches current sound handles without
  configured StopSound, ExpireAnim, or pending deletion. The conditional destructor owner branch is
  a no-op because these producer objects are owner-null. Its first real-cell iterator clears raw
  `0x100000|0x200000` on every real cell but not the shared dummy. The second writes
  `Cell+0x30=0`, crosses the ordinary-cell LightConvert/ZAdjust recomputation slot, clears the
  `0x20000` animation latch, then restamps `0x100000` for an AttachedTag event `0x19` or otherwise
  `0x200000` for event `0x1A`, with `0x19` precedence. Stamps traverse the complete rectangular map
  bounds through shared-dummy lookup, so sparse misses accumulate bits on the uncleared dummy. The
  active consumer is `FootClass::PerCellProcess @ 0x004D85D0`: marked-cell entry scans every matching
  tag. After a horizontal scan, its vertical gate tests the final row lookup/dummy rather than the
  mover cell, while the vertical scan retains the mover's original X. Official `all01umd.map` proves
  event-`0x1A` reachability. These are generic trigger accelerators, not bridge-zone or topology bits.
  The lighting literals are defaults only: ordinary cells recompute current Scenario/light-source/
  height/RGB-key/brightness outputs; only `(0,0)` and `(-1,-1)` remain neutral. It then calls value-only
  `Get_Tiberium_Value`, contributing signed zero for a non-resource cell or signed 32-bit
  `(existing_state + 1) * TiberiumClass.Value` for a recognized resource to a wrapping signed 32-bit
  return total before Recalc, Recalcs the current cell, and reconstructs owner state if that post-
  Recalc cell is a wall. It never calls germination and does not rebuild the queues. Full_Init
  persists that return at `MapClass+0x134` / `0x0087F91C`, and the cell-array teardown resets the field
  to zero. The active direct-xref set proves the write but no gameplay consumer, so exact state must be
  retained without inventing save/hash/presentation semantics. Generated argument 1 does not write it.
- **Rust state:** terrain objects and raw overlay occupation are folded into initial resolved-cell
  construction (`src/map/resolved_terrain.rs:2057-2082`). Tile animations are created once during
  that construction and merely sorted by an anti-diagonal key (`:2237,:2421`). Runtime constructs
  terrain then entities and finally spawns that precomputed vector (`src/sim/runtime.rs:626-660`),
  with no post-object Recalc/unlatch/recreation boundary. Production builds resolved terrain and the
  duplicate OverlayGrid before calling `construct_app_scenario` (`src/app/loading/init.rs:1857-1994,
  2127`; `src/app/loading/init_helpers.rs:518-551`), so no Simulation/native-ID/live-registry sink
  exists for synchronous per-Mark/first-sweep Anim construction. Production and headless scheduler-
  root binding derives from already-precomputed `tile_animations()` descriptors
  (`src/app/loading/init.rs:1885-1905`; `src/headless_scenario.rs:202-223`), which cannot remain the
  prerequisite for the live constructors that must produce those effects.
- **Required delta:** construct each authored first-generation animation at its first native Recalc;
  preserve per-Mark versus remaining first-sweep order and the exact ID/registry/RNG/Middle/post-write
  sequence. Route each effect synchronously through one map-defined sink implemented by the sole sim
  load orchestrator and the shared native-ID cursor plus collision-free Anim-handle owners; a
  buffered/final descriptor vector is not an execution authority. Consume `ScenarioBootstrapRng`
  before Fill into one staged real Simulation load runtime and retain the same Scenario/native-ID/
  handle/registry/queue identities through final payload installation; do not construct a late sim or
  transfer a shadow registry. Split a pure map-owned scheduler-root discovery step after Fill and
  before OverlayPack/first Recalc so app/headless can bind required assets with zero live effects;
  actual construction remains sink-only. After Terrain, let the sole sim ore-queue owner initialize growth then spread
  from a temporary read-only view of the live map, retain that state through later object occupancy,
  and forbid a post-object rebuild. Construct object sections in native order while the first Anim
  generation remains live. Store the authored argument-0 return in one map-load-state field with the
  proved teardown reset, while keeping generated argument 1 local-only. Add a narrow
  immediate scalar-delete primitive rather than calling generic Destroy/UnInit; release active sound
  with no StopSound, preserve unrelated registry survivor order, leave owner/occupancy untouched,
  expose the exact common raw-clear/opaque-zero/light/tag-restamp integration slots around the owned
  unlatch/value-or-germinate/Recalc/wall-owner order, then recreate the final set. Transaction 3 owns
  one cell-light cache invalidation/recompute-routing event at that slot and reuses the existing wall-
  owner helper after all final-current Recalcs; it does not materialize generic tag bits/consumers,
  semantic LightConvert values, or a new
  `+0x30` field. Route those semantics respectively to the generic trigger owner, transaction 20, and
  transaction 21/OQ-19. Do not model two
  sweeps as a precomputed `sweep_count=2` loop or delay all construction until survivors are known.
- **Acceptance:** source-order versus sweep-order first creation; no latched duplicate; one pre-Fill
  staged Simulation owner surviving unchanged through gameplay; no callback to an absent sim and no
  end-of-load live-owner transfer; production/headless pure root discovery covering required roots
  with zero IDs/handles/RNG/registrations/sounds/latches/overlay writes and a missing-asset failure
  before the first OverlayPack/Recalc Anim-construction effect while preserving already-spent prefix
  native IDs and Fill RNG state;
  ID-and-Anim-registry-before-RandomRate; custom Scenario cursor plus stock-zero and unchanged-Main
  controls; Reveal/Logic with no occupancy; Middle-before-producer writes and conditional Start;
  WA01X first StartSound/current-handle release/final restart plus non-`01` waterfall no-StartSound;
  configured-StopSound, ExpireAnim, pending-delete, and owner-mutation negatives; mixed-registry
  survivor order; immediate deletion; per-cell unlatch; one object/terrain-mutated former candidate
  deleted without recreation while an unchanged peer recreates from live state; object-derived
  changes appearing only at the second boundary; exact value-only early-zero/formula/wrapping local
  accumulation; exact authored `MapClass+0x134` persistence/reset plus generated no-write; no invented
  field consumer; no germination; an exact slot/event trace for first-pass raw clear and second-pass
  opaque zero -> cell-light cache invalidation/recompute routing -> unlatch -> `0x19`-before-`0x1A` tag-line restamp ->
  tiberium result -> Recalc -> current-wall owner sequence; negative assertions that transaction 3
  materializes none of the generic tag bits/consumers, semantic LightConvert state, or `+0x30`, and
  that none enter BridgeFacts/topology/zones; existing wall-owner-helper reuse; explicit generic-
  trigger/transaction-20/transaction-21 routing; growth-before-spread immediately after Terrain;
  and an adversarial
  spread-eligible resource cell whose later ground occupier does not retroactively alter the seeded
  queue state, so initialization delayed until after Units/Structures or a final rebuild cannot pass;
  and explicit hard-load failure rather than silent omission for a missing referenced AnimType or
  forced Anim allocation/registration failure.
- **Verdict:** MISSING / ARCHITECTURE DRIFT.
- **Priority rationale:** every fresh load crosses this boundary; visible impact occurs on maps with
  animated tiles or object-sensitive passability, and silent zone/cache drift can affect pathing
  whenever terrain objects occupy relevant cells.

### HIGH priority

**G7. Runtime overlay state has no consumed-once finalized payload**

- **Active-YR evidence:** after OverlayData and the first sweep, the minimum real payload is the
  validated overlay identity, final state byte, and the authored real-cell blocker-neighbor plane
  produced by ordered wall Mark writes. Land/zone/LAT/cache remain live derived cell state. Later
  consumers do not decode the packs or reconstruct authored wall counts again.
- **Rust state:** the transaction branch now carries identity/state plus the real-cell wrapping count
  plane through a non-Clone `FinalizedOverlayPayload`, installs `Some(plane)` through the narrow
  OverlayGrid constructor, persists/hashes/shape-validates it, and seeds the global count builder
  without scanning final walls. Production and headless still use legacy raw constructors, so live
  maps and their current v114 saves remain `None` and deliberately retain the temporary final-wall
  fallback until the one production payload boundary lands.
- **Required delta:** consume identity/state/count exactly once in production and headless through the
  narrow OverlayGrid/global-count installation with no raw pack, rules, RNG, filter, Recalc, final-wall
  scan, or dummy export. Once every gameplay builder produces `Some(plane)`, reject `None` at the
  current-version save/restore boundary; if any v114 `None` save escapes before that gate lands, bump
  the snapshot version again rather than silently upgrading it through final identities.
- **Acceptance:** procedural/rejected/identity-empty/data-only/wall-overwritten cells match one-for-one
  in terrain, runtime grid, global counts, hashes, and later mutation; real fixed-stride aliases survive,
  true dummy misses add no output, and duplicate consumption/second decode/final-wall rebuild are impossible.
- **Verdict:** PARTIAL / PRODUCTION DISCONNECTED; G7 REMAINS OPEN.
- **Priority rationale:** every production load currently uses the duplicate constructor; it is
  harmless only when both projections happen to agree and becomes immediately player-visible on
  the first procedural or Recalc-cleared identity.

**G8. Presentation asset discovery is still keyed to raw map entries**

- **Active-YR evidence:** procedural low bodies become live overlay identities before gameplay and
  must be renderable; their final data comes from the finalized cell state.
- **Rust state:** atlas/name discovery clones and iterates `map_data.overlays`
  (`src/app/frontend/skirmish.rs:2686,2717`) before filtering render entries against OverlayGrid
  (`src/app/loading/init.rs:2336-2369`). Procedurally created identities absent from the raw pack
  cannot enter that render index, even though low variants are broadly preregistered by name.
- **Required delta:** derive occupied render entries from the finalized/live overlay payload while
  retaining registry-level preloading for runtime variants; do not make raw pack membership the
  final occupancy authority.
- **Acceptance:** a trigger-generated body cell is present in OverlayGrid, overlay render index,
  atlas dependency closure, minimap/radar, and bridge presentation with its post-data state.
- **Verdict:** PARTIAL / DRIFT.
- **Priority rationale:** the issue triggers only on procedural custom/editor content in the current
  retail census, but every such produced deck cell would otherwise simulate without a matching
  rendered body.

**G9. Headless and auxiliary constructors cannot express the exact load boundary**

- **Active-YR evidence:** Mark-active authored loads require a proven fresh family/Scenario cursor and
  format `>1`; format-inactive authored loads need no Mark draw cursor at the pack body but still
  execute their family-dependent native-ID prefix and ungated sweep identities. Generated/direct
  materialized and restore are distinct.
- **Rust state:** `build_headless_terrain_bootstrap` hardcodes `Authored` and has no typed context or
  format-sensitive error (`src/headless_scenario.rs:75-116`). Selector-free/focused constructors
  similarly default toward authored semantics inside `ResolvedTerrainGrid`, so tests and parity
  digests can silently certify a path production cannot validly enter.
- **Required delta:** expose the same explicit source/family/format descriptor and consumed-once
  native-ID prefix receipt to headless and auxiliary builders. Require it for every fresh gameplay-
  equivalent path regardless of format; an untyped no-live-effects map diagnostic is non-parity.
- **Acceptance:** production/headless/auxiliary outputs, Scenario cursor, native-ID cursor/event
  trace, and failure point agree for typed authored format 4, typed authored absent/1, and generated
  materialized; untyped Generic rejects before any ID/draw at both absent/1 and 4.
- **Verdict:** MISSING.
- **Priority rationale:** headless is not the ordinary graphical path, but it backs determinism and
  parity tests; a permissive false model would repeatedly hide production drift during every future
  bridge transaction.

**G10. Generated construction replay collapses native Recalc/animation/resource phases into final state**

- **Active-YR evidence:** generated `.SED` first reaches synthetic Full_Init. Format defaults to
  zero and suppresses encoded pack bodies, but Full_Init Recalc/InitCellAttributes boundaries are
  ungated and may construct animations according to the actual staged premap. After direct
  materialization, generator work is sequenced as bridge/CABHUT attempts; first whole-map Recalc at
  `0x00598E48`; start-point/AddTechBuildings/Neutral-Tech constructors; Tiberium and Recalcs at
  `0x00598FE7` and `0x00599153`; hills/LAT/trees/rocks plus direct helper Recalc at `0x005A4259`;
  final Recalc at `0x0059937D`; tiberium growth/spread queue initialization; scratch cleanup; and
  final `InitCellAttributes(1)`. That final call scalar-deletes the
  marked Anim generation and, in its real-cell pass, clears each cell latch, calls
  `SpreadCellGerminate(0)`, and then Recalcs that cell. An absent/unrecognized resource contributes
  signed zero; a recognized resource rewrites density and returns signed 32-bit
  `(new_state + 1) * TiberiumClass.Value`, which is added to a wrapping signed 32-bit local total
  before Recalc. No persistent owner/consumer for the total is proved, and the already initialized
  queues are not rebuilt afterward. Authored Full_Init passes zero and performs value-only
  `Get_Tiberium_Value` accounting from existing state with the analogous signed formula/local total,
  but no all-cell germination.
  Therefore animation IDs, optional RandomRate draws, registry/sound effects, and even transient
  generations can interleave after CABHUT and before Neutral-Tech constructor attempts. Final cells
  cannot reconstruct that history.
- **Rust state:** `RmgConstructionTrace` is a flat Building-attempt vector and runtime replays the
  complete trace before spawning one final descriptor set. `ResolvedTerrainGrid` retains only final
  cells/descriptors. RMG first tiberium placement leaves density zero, repeat hits increment it, and
  emit copies that phase-local byte directly into both OverlayEntry and OverlayDataPack; the pipeline
  ends after final LAT/emit with no `InitCellAttributes(1)` or local total. Production then re-decodes
  those packs into OverlayGrid and initializes ore queues from that one unfinalized byte set. Neither
  representation can insert all first/later generator Recalc effects at their native positions,
  preserve a synthetic-Full_Init generation, or hold native pre-final queue state beside post-final
  live overlay state.
- **Required delta:** extend or replace the trace with a consumed-once phase-aware transport that
  retains each actual CABHUT construction, every emitted/discarded Neutral-Tech constructor, and each
  generator Recalc/queue/Init boundary. Replay every actual Building construction as Techno Scenario word,
  Building native unique ID, then placement outcome while preserving PR-#170's bound word. A discarded
  Neutral-Tech consumes both but binds no entity; a failed CABHUT site search occurs before
  construction and consumes neither, while a constructed stock CABHUT consumes both and emits.
  Consume that transport inside the sole simulation load orchestrator through the existing
  Building/Anim constructors, live registries/sound owner, independent native-ID counter, and
  collision-free runtime-handle allocator; do not
  precompute a parallel animation history. An emitted binding carries both its preconsumed Techno
  word and Building native ID into final projection, which performs neither a second draw nor native-ID
  allocation and may assign/use its runtime handle independently. Interleave native animation IDs,
  Scenario draws, registry/sound effects; capture the actual
  synthetic Full_Init boundary rather than assuming zero; initialize the sole sim ore queues at the
  native pre-final-Init point from a temporary read-only live-map view with no second decode or
  retained grid; and execute each final per-cell latch clear, exact helper early-zero/density rewrite/
  signed `(new_state + 1) * Value` return and wrapping signed-32-bit local aggregation,
  then Recalc in that order without a post-pass queue rebuild or invented persistent total. Preserve
  direct generated identity/state through that final mutation and continue to skip authored pack
  Mark.
- **Acceptance:** an actual staged synthetic control plus a generated interleaving fixture asserts
  constructed CABHUT effects -> first-Recalc animations -> all Neutral-Tech constructors ->
  later-paint animations -> queue initialization -> final immediate delete -> anti-diagonal per-cell
  unlatch -> germination/local-value -> Recalc/recreate. It pins failed-CABHUT-search zero effects, discarded Neutral-Tech
  consume/no-bind behavior, native-ID/runtime-handle order, PR-#170 stored RNG words, custom
  RandomRate Scenario continuation, emitted binding reuse with zero projection draw/native-ID
  allocation, unchanged Main RNG,
  live Anim survivor/sound order, and a stock-waterfall
  zero-RandomRate control. A separate resource fixture pins helper early-zero, same-class/dummy
  density, exact per-cell signed return, wrapping signed-32-bit local total,
  and final payload state while proving queues retain their earlier initialization state, no
  post-pass rebuild occurs, and no persistent aggregate is invented. Poisoning final cells or flattening the trace must not be able to
  reproduce the accepted event history; a missing referenced AnimType or forced Anim allocation/
  registration failure must abort explicitly rather than silently dropping a generated generation.
- **Verdict:** MISSING / ORDERING ARCHITECTURE DRIFT.
- **Priority rationale:** the missing final resource pass triggers on every generated map containing
  tiberium. Animation history additionally triggers where generated phases create eligible animated
  tiles; waterfalls make that repeatable in ordinary active RMG content, while custom RandomRate
  shifts later Scenario consumers. Even stock zero-RandomRate rows still shift object IDs and visible
  loop-sound lifecycle.

**G11. Rust has no active preview-native object/ID lifetime and conflates native IDs with stable handles**

- **Active-YR evidence:** preview Generate takes argument `1`, reaches Set_Defaults/manual setup, and
  never takes Full_Init, Clear_Scene, or the map-read `+10,000` reservation. `g_GameActive=1`, so its
  Buildings and tile Anims take ordinary active registration, Unlimbo/Logic, latch, and admitted-
  sound paths. Every Generate resets `Scenario+0x214` to `1,000,000` before cleanup but does not
  rewind Scenario RNG. Let `R(W,H)=H*(2W-1)+1` for row-major real Size-diamond Cells plus the dummy,
  and `HB(H,S)=H*(1+S)` for House plus optional Super blocks. Missing/changed normalized width-height-
  theater-player-count storage identity full-cleans old objects/sounds, then consumes
  `C_pre_gen = 1_000_000 + R + |P_preview| + HB + K_preview` with wrapping signed-32-bit addition;
  retail has `K_preview=0`. A matching key skips every setup constructor, so the first new object can
  be `1,000,001` while retained Type/House/Super/real-or-dummy Cell/Anim objects already own the same
  numeric ID. It selectively deletes Unit/Infantry/Building/Terrain but retains old final Anims,
  latches, and sounds through intermediate Recalcs until terminal live-order deletion and recreation.
  Native AssignUniqueID performs no collision check, so these cross-class duplicates are legal.
  Every Generate frees spread then growth and later rebuilds growth then spread. Cancel/common teardown
  destroys only UI/snapshot owners; live preview objects, sounds, counter, queues, and Scenario
  advancement survive Cancel and no-Generate re-entry. The first later Generate resets then full-
  cleans because the snapshot is absent. Acceptance itself does not clean; accepted `.SED` launch
  frees spread then growth at generator entry, repeats that free in Full_Init/Clear_Scene, then Full_Init
  rebuilds growth then spread. Every fresh Full_Init starts its own cursor at `1,000,000`. With actual
  ordered successful constructor-event streams `E_campaign`, `E_multi`, `P`, Resize operands
  `(Hc,S1)`, `(H1,S0)`, `(H2,S1)`, and cell totals `R1/R2`, campaign derives
  `C_saved = 1_000_000 + |E_campaign| + |P| + HB(Hc,S1) + R2`; noncampaign and accepted `.SED`
  derive `C_saved = 1_000_000 + |E_multi| + HB(H1,S0) + R1 + |P| + HB(H2,S1) + R2`, all with
  wrapping signed-32-bit addition. Map read sets the cursor from the snapshot to
  `wrap32(C_saved + 10_000)` rather than adding to its then-current value, while retaining the custom-
  theater shadowed-Assign window. Every successfully allocated Tube source row spends one ID before
  token parsing; malformed allocated and allocation-null rows then hard-error, with only the former
  spending. There is no reject-and-continue arm. A fresh reader's shared deferred queue prestate is
  exactly empty. For `T` successfully constructed Tube rows, the first admitted/allocated Overlay ID
  is `O1 = wrap32(C_saved + 10_000 + T + 1)`, and synchronous child Anims advance the same cursor
  before the next Overlay.
- **Rust state:** `OfflineSkirmishRuntime` has Scenario RNG and options but no Scenario native-ID
  counter or process-shell preview object owner (`src/app/frontend/skirmish_session.rs:86-92`). Shell
  retention owns presentation/storage only (`src/app/shell_random_map.rs:178-217`), clears the
  candidate at generation entry without the native reset/cleanup branches (`:398`), and applies a
  Building-only trace after complete generation (`:377-385`; session `:515`). The trace has no Anim,
  destructor, ID, or sound events (`src/map/construction_trace.rs:6-35`). `AnimClass` chooses
  RandomRate before allocating its stable ID, then aliases that collision-free key into
  `native_unique_id` (`src/sim/anim_class.rs:543,550,554-558`), making the legal duplicate-native-ID
  window impossible and omitting the fresh-load prefix. `parse_tubes_section` silently continues past
  malformed or missing-sentinel source rows (`src/map/tubes.rs:24-35,38-85`), and the convenience
  `explicit_tubes` projection retains only validated `Vec<TubeFact>`. The raw source is not lost:
  `MapFile` also retains its full `IniFile` (`src/map/map_file.rs:242-245`), and `IniSection::get_values`
  exposes exact first-insertion value order (`src/rules/ini_parser.rs:19-25,156-162`). Current load
  authority nevertheless consumes only the filtered facts and has no constructor-allocation outcome,
  spend-before-parse, hard-error point, or native-ID binding.
- **Required delta:** add a process-shell preview-native lifecycle owner independent of the UI
  candidate/snapshot. It retains live registered preview Buildings/Anims, latches, sounds, native
  counter, and distinct collision-free handles through Cancel/re-entry. Select and apply reset plus
  exact full/selective cleanup before generation, then pass a consumed-once retained-latch/live-Anim-
  order prestate and generation token into the Recalc producer; validate that token when applying its
  phase-aware journal. A clean worker cannot reconstruct latch-suppressed events afterward. Apply the journal
  at native boundaries: Building
  word -> native ID -> outcome; Anim native ID/register -> optional RandomRate -> active placement/
  sound; terminal registry-order destruction and final recreation. Keep `native_unique_id` as a
  reproduced numeric field, never the collection key. The gameplay load orchestrator separately
  owns one common fresh-Full_Init native-ID cursor from Clear_Scene through all actual pre-map
  constructors, the set-from-snapshot wrapping `+0x2710` map-read transform, successful Tube constructors, and later
  Overlay/Anim/Building effects, and cleans accepted preview state before launch. Promote only the
  exact pre-map/Tubes constructor-ID prefix required for this transaction. Consume the already-retained
  raw `[Tubes]` values exactly once, in source order, through allocation/Assign-before-parse so malformed
  allocated and allocation-null rows fail at the proved point; do not use filtered `explicit_tubes` as
  accounting input. Every successful Tube ID
  is stored in a consumed-once binding keyed to its parsed source record so transaction 5 installs it
  without a second native-ID allocation; malformed/allocation-null rows hard-error and have no
  binding. Tube topology/
  traversal remains transaction 5. Preview objects never become gameplay bridge/entity authority.
- **Acceptance:** all thirteen report fixtures: preview-branch exclusion; constructed-discarded
  Building cost; failed CABHUT pre-search zero cost; stock TileAnim ID/zero-RandomRate/four `*01X`
  sound controls; custom RandomRate order; terminal churn; same-key retained-old-Anim and legal
  duplicate-native-ID window; changed/missing reset-before-full-cleanup then exact real-Cell/dummy ID
  prefix versus matching-key zero Cell IDs; Cancel/no-Generate re-entry
  retention; first later Generate reset/full-clean/new order; and acceptance-versus-later-launch
  separation. Matching storage asserts zero setup constructors and legal retained-object numeric-ID
  duplicates; changed/missing storage asserts exact `R+P_preview+HB+K_preview` order and retail
  `K_preview=0`. Every Generate and Cancel/re-entry pins the exact queue lifetime. Accepted `.SED`
  asserts two consecutive free-spread-then-growth pairs (generator entry and Full_Init/Clear_Scene),
  then one Full_Init growth-then-spread rebuild.
  Fresh campaign, noncampaign, and `.SED` controls derive `C_saved` from their exact ordered formulas,
  retain a custom shadowed theater Assign, assert the set-from-snapshot wrapping `+0x2710` transform,
  then cover `O1 = wrap32(C_saved + 10_000 + T + 1)` at `T=0`, `T=2`, and a wrap boundary. Allocated
  malformed and allocation-null Tube rows hard-error before Overlay with spend-one versus spend-zero.
  Concrete numeric oracles pin `1,000,018 -> 1,010,018` at map read,
  `C_saved=1,000,037,T=0 -> O1=1,010,038`, preview `1,000,018 -> first object 1,000,019`, and
  map-read cursor bit pattern `0xFFFFFFF0 -> 0x00002700`.
  A transaction-5 handoff control consumes the
  successful source-record bindings with zero second native-ID allocation; preview proves that whole reservation
  prefix absent. A poison prestate fixture proves an old latch suppresses one intermediate Anim while
  terminal unlatch recreates it, and rejects a stale generation token. Every record asserts distinct
  stable runtime handle and native numeric ID.
- **Verdict:** MISSING / IDENTITY ARCHITECTURE DRIFT.
- **Priority rationale:** the lifecycle triggers on every RMG preview and replacement; native IDs
  advance for every constructed Building/Anim. Player-audible impact occurs whenever an admitted
  waterfall loop is retained or destroyed at the wrong boundary, while identity/order drift can
  compound into all later constructors even when stock RandomRate is zero.

**G12. Authored Overlay rows have no real load-object registry/deferred-finalization lifecycle**

- **Active-YR evidence:** every reader-admitted, successfully allocated row constructs a real
  OverlayClass in exact order: Abstract/Object bases; best-effort Object, pointer-expiration, all-
  Abstract, and Tag-listener appends; preincremented native ID; Overlay registry append; then a direct
  base ObjectClass::Unlimbo whose virtual Mark reaches Overlay Mark. Common success may construct an
  ordinary CellAnim and a first-eligible terrain Anim between that Overlay ID and the next row. It
  then sets OnMap false/Limbo true; UnInit broadcasts pointer expiration #1 while memberships remain,
  virtual Limbo no-ops, clears alive, and appends to the duplicate-permitting shared queue. The dead
  object remains in every joined registry through all later identity rows and OverlayData. During a
  successful authored Full_Init, `ScenarioInit` is nonzero and short-circuits the wall build predicate
  to true, so a slope-admitted wall completes its wall effects and then follows the same common queued
  tail. The separate counter-zero generic wall-rejection path still orders UnInit broadcast #1 -> full
  Limbo/Destroy/Mark-remove broadcast #2 -> death/queue, but it is unreachable from authored Full_Init
  and is not an authored acceptance branch. Steep slope `>4` except
  `0xB2` returns after base Mark/dirty but before cell/Recalc/UnInit, leaving an alive, InLimbo/on-map/
  redraw, registered, ID-bearing, unqueued survivor until scene teardown. It never joins cell,
  Display, Logic, current-object checksum, native save, or render surfaces. After temp pixel cleanup,
  the reader invokes the shared live drain exactly once outside format/body gates, including generated
  format 0. The drain preserves alive entries, removes all duplicates of each selected dead pointer,
  processes shifted and live-appended successors, invokes Release, then scalar-finalizes once. The
  scalar destructor broadcasts while memberships still exist (#2 common/authored wall; #3 only for
  the generic counter-zero wall-rejection path), removes Overlay
  registry, game-active Limbos (no-op), clears type, then base destruction removes queue, Object,
  pointer-expiration, all-Abstract, and Tag memberships in that order before free; IDs are not refunded.
- **Rust state:** resolved terrain applies raw/high projections before Simulation exists and has no
  load Object owner (`src/map/resolved_terrain.rs`). OverlayGrid performs only a final-cell filter/
  stamp (`src/sim/overlay_grid.rs`) and cannot express queued dead or slope-survivor state. The sole
  pending-delete drain runs at ordinary frame tail, not the reader boundary, though its live-order/
  duplicate-selection algorithm is reusable (`src/sim/world/lifecycle.rs`). App loading constructs
  Simulation only after map finalization (`src/app/loading/init.rs`), so there is no current owner for
  shared native ID, registries, queue, or synchronous child Anims. A stack-local finalizer would also
  be wrong because native steep-slope survivors outlive load completion.
- **Required delta:** create the sole staged Simulation load runtime before Fill and attach a lightweight
  `LoadObjectLifecycle` keyed by collision-free runtime handles with five ordered registry memberships,
  alive/limbo/on-map/redraw state, and the shared queue. Drive every allocated authored row through a
  synchronous map-defined sink, allocate native IDs from the closed OQ-34 cursor contract, construct ordinary
  CellAnim and terrain Anim effects before the next row, preserve dead registry visibility through
  data, represent the exact common/authored-wall broadcasts and destructor order while retaining the
  separate generic counter-zero rejection method, then run the shared drain in
  the unconditional reader epilogue. Keep the same lifecycle registry attached to the process-
  scenario/Simulation owner after load and retain slope survivors there until scene teardown; do not
  transfer/reconstruct it from final cells or promote survivors to GameEntity/OverlayGrid/
  presentation/save/checksum. Match native allocation-null as no construction; hard-error injected
  base/Overlay-registry or queue growth failure rather than silently preserving partial degradation.
- **Acceptance:** explicit-`C_saved`/two-Tube constructor order; success+CellAnim+terrain-Anim/slope/
  next-row ID chain; data-before-drain visibility; common and authored-wall success's exact two-
  broadcast event order; authored wall success versus slope rejection while the generic counter-zero
  three-broadcast/full-Limbo method remains separately tested; slope state;
  mixed `[alive A, dead B, B, alive C, dead D]` shared queue; format-1 and absent/empty-body
  unconditional drain; reader-reject/allocation-null zero effects; registry/queue-growth hard errors;
  fresh reader queue `[]` becoming exact drain input `[overlay0, overlay1]` for two common successes,
  with no House/Super/Type/Cell/Tube prefix handle in the queue and no Overlay-lifecycle runtime
  handle allocated by the consumed prefix receipt;
  generated format-0 zero Overlay constructions plus seeded shared drain; slope presentation/save/
  current-checksum negative; slope survivor remains registered after load and releases only at scene
  teardown with no final-cell reconstruction; exact Overlay/Limbo/type/queue/Object/three-listener/free removal order
  and no ID refund; and the exact OQ-34-derived absolute prefix.
- **Verdict:** MISSING / LIFECYCLE ARCHITECTURE DRIFT.
- **Priority rationale:** constructor/ID/deferred-drain order triggers for every admitted row on
  format-active authored maps, common in shipped content; optional child Anims are visibly active when
  configured and all IDs affect later object identity. Slope survival is content/terrain-conditional,
  but the shared reader drain runs on every fresh reader call even when the pack body is absent.

**G13. Authored walls lack their ScenarioInit success transaction and retained blocker-neighbor count plane**

- **Active-YR evidence:** `ScenarioClass::Full_Init` increments `ScenarioInit @ 0x00A8E7AC` before the
  authored Overlay reader and keeps it nonzero while `OverlayClass::Mark @ 0x005FC570` runs. After the
  universal slope gate (`slope > 4`, except `0xB2`), the wall build predicate is therefore forced true.
  The wall path stamps identity/data, calls `PostDestructionWallCleanup(cell,1)` in fixed
  N/E/S/W/self order, recomputes same-compact-ID cardinal connectivity with no Building owner yet,
  leaves owner `-1`, increments each of the eight N/NE/E/SE/S/SW/W/NW neighbor `CellClass+0x122`
  bytes with wrapping `u8`, and then reaches the common anchor Recalc/UnInit/death/queue tail.
  OverlayData later overwrites only state. A later low procedural body can overwrite the wall identity
  without decrementing the earlier count writes, so final identities are not a reconstruction source.
  Signed fixed-map neighbor arithmetic may alias another real 512-stride slot; only a true miss reaches
  the shared dummy, whose count write has no fresh-game output. Active retail contains 13,064 such wall
  cells across 71 winning MIX entries / 187 logical maps, so the success path is routine rather than
  dormant.
- **Runtime lifecycle refinement:** direct `DestroyOverlay` decrements unconditionally after its full
  cardinal cleanup. Cleanup auto-removal decrements only after Recalc changes zone type. House wall
  sale has no sold-anchor `+0x122` access and deliberately leaves that contribution stale. The binary
  also contains CYCL/BARB/FENC threshold rows, but retail never sets those types `Wall=yes`; only
  GASAND/GAWALL/NAWALL are active and the TS/mod-only rows remain excluded. Runtime cardinal and
  cleanup probes use the same signed fixed-grid lookup as their count tails: a real alias is visited,
  connected, dirtied, Recalced, and conditionally removed; it is not rectangle-clipped. Direct
  terminal cleanup receivers run `N,W,S,E`, while the penultimate chain remains `N,E,S,W`; each
  cleanup receiver completes its `N,E,S,W,self` walk. A true miss returns the persistent dummy,
  whose overlay identity/state can affect later visits and whose packed coordinate is a real radar
  dirty output even though no dummy count enters the exported real plane. Direct cell-pointer expiry
  follows the complete cleanup fan-out and precedes the direct count decrement; cleanup-removal
  expiry precedes its Recalc. Active retail `[IonWH]` has `Wall=yes`, so Lightning Storm is a live
  caller; `[MutateExplosion]` has no wall flags and is a dormant Genetic Mutator exclusion.
- **Rust state:** the transaction branch now carries a wrapping real-cell plane through
  `LiveOverlayCells -> FinalizedOverlayPayload -> OverlayGrid`, serializes/hashes/shape-validates it,
  seeds `BlockerNeighborCounts` from `Some(plane)` without a final-wall scan, preserves legacy `None`
  only for old constructors, and applies verified runtime placement/direct/conditional-cleanup/sale
  deltas. Direct removal now executes its represented Recalc before cleanup; every visited live wall
  Recalcs in cleanup order; cleanup count reversal follows its changed-zone Recalc; and the direct
  decrement remains after the full fan-out. All eight count probes now use the stamping lookup;
  chain/cleanup/sale preserve shared-dummy state and exact packed radar coordinates. Standard combat,
  ambient Wave, crush, world-event, sale, and Lightning paths have synchronous navigation/radar
  ownership, and represented Techno Cell targets use the native forward clear-first expiry order.
  Focused validation now passes the 106-test `wall` filter, 58 overlay-grid tests, 11 Lightning
  tests, the pointer-expiry order test, the shared-dummy hash test, and three distinct live-object
  detach-sweep controls; fresh criticism remains pending. The broader native non-entity
  expiry-listener roster is not represented and remains open. The authored wall helper itself
  remains test-only: no
  production authored-row reader calls it or consumes the finalized payload, so live maps still enter
  legacy mode. Generic
  `finish_wall_reject` remains valid only for counter-zero non-authored construction.
- **Required delta:** wire the proved wall helper into the one synchronous authored row transaction,
  apply its cleanup/count/common effects through the real load-effect and common lifecycle owners, and
  consume the finalized payload in production/headless construction. Preserve real fixed-stride aliases,
  suppress only dummy count-plane output while retaining runtime dummy state/radar output, and keep
  OverlayData/later low writes count-neutral. Complete fresh criticism of the runtime slice, then
  remove production's legacy
  constructor path so authored authority can never fall back to final-wall reconstruction, then reject
  current-version `None` saves/restores (or bump beyond v114 if such a save has escaped).
- **Acceptance:** fixtures pin ScenarioInit predicate bypass on a slope-accepted wall; exact
  N/E/S/W/self cleanup and N/E/S/W connectivity; owner `-1`; eight-neighbor wrapping increments;
  overlap wrapping; later low-body identity overwrite with retained counts; fixed-stride real alias
  update; runtime fixed-alias connectivity/chain/cleanup/sale; authored true-dummy count-output
  absence plus runtime dummy state/radar presence; exact direct/cleanup pointer-expiry order; active
  Lightning navigation/radar publication; OverlayData
  state-only behavior; common two-broadcast authored-
  wall finalization; separate generic counter-zero rejection; and a finalized-payload consumer that
  receives the count plane without scanning final walls.
- **Verdict:** PARTIAL / PRODUCTION DISCONNECTED; G13 REMAINS OPEN.
- **Priority rationale:** the active-winner census contains 13,064 encoded authored-wall occurrences
  across 71 retail entries, and at least one shipped flat-wall witness proves the successful path is
  reachable. Every reader-admitted, allocated, slope-accepted authored wall executes it. The ordinary
  visual identity often masks the mismatch, but hierarchy path expansion diverges whenever overwritten
  or aliased contributions differ, and final-snapshot reconstruction cannot repair that history.

## Doc-derived candidates needing verification

OQ-32, OQ-33, and OQ-34 are fully resolved by their focused reports and are now verified gaps G11
and G12, not remaining evidence candidates. OQ-34 closes the exact branch-specific preview setup,
complete campaign/noncampaign fresh-Full_Init constructor formulas, custom-theater window, set-from-
snapshot map-read transform, Tube allocation/fault semantics, empty reader-queue prestate, and first-
Overlay formula. No material transaction-3 native behavior remains in this section awaiting focused
verification.

Every other material source/context, row, write, RNG, dummy, data, Recalc, payload/count plane,
animation-lifecycle, and generated launch phase-order rule has active-YR evidence. The exact set of
eligible animations in the synthetic pre-materialization launch Full_Init remains content/state
dependent, not an inferred rule: implementation acceptance must capture the actual staged boundary
or prove it empty for the exact staged state and may not derive zero from `NewINIFormat=0`.

## Deferred / blocked by later owners

- Synthetic-Full_Init animation eligibility is not deferred to a later transaction. Its per-seed
  inventory is bounded-open, so G10 remains open until transaction 3 transports/captures the actual
  staged lifecycle or proves zero for every staged state it constructs.
- Transaction 21 must independently verify whether save/restore serializes or reconstructs dummy
  overlay/state. Fresh-load constructor behavior is not authority for persistence.
- Cache-B byte `+9` remains unidentified but has no observed writer/reader that changes the dummy,
  finalized payload, low control flow, or current Recalc implementation decision.
- `InitCellAttributes`' ordinary-cell LightConvert/ZAdjust recomputation is active but remains
  presentation-owned transaction-20 work; transaction 3 executes one invalidation/routing slot and
  avoids stale preview caches. Only sentinel ids retain the neutral defaults.
- `InitCellAttributes`' `Cell+0x30 = 0` persisted/swizzled pointer-slot writer is proved, but its live
  meaning/consumer remains transaction-21/OQ-19 work; it must not become an invented bridge fact or
  numeric scratch.
- The `0x100000`/`0x200000` AttachedTag bits are generic event-`0x19` row and event-`0x1A` column scan
  accelerators. They are a verified bridge-topology exclusion; transaction 3 exposes their exact
  ordered slots, while the generic trigger owner remains responsible for real/dummy bits and offers.
- BR-M04 remains open after this shared high-load contribution; transaction 4 owns the wider high
  topology/structural reconciliation.
- BR-M11 remains open after the load-time Road contribution; later mutation/repair behavior belongs
  to its owning transaction.
- Positive Tube topology/traversal remains transaction 5. Transaction 3 promotes only each successful
  parsed Tube's native-ID constructor/binding prefix so transaction 5 cannot allocate it twice; low
  Overlay Mark remains independent and has the negative no-synthesis obligation.
- Malformed native type/art crash fidelity may be represented as a safe typed load error rather than
  reproducing undefined process failure, provided valid-row admission and cursor effects remain exact.

## Doc errors discovered

The nine original 2026-08-31 reports plus the 2026-09-01 authored-wall report are mutually consistent
after the explicit scope corrections.
The preview-native report corrects the terrain-animation report's unqualified synthetic-Full_Init
wording: setup preview takes Set_Defaults/manual setup and no Full_Init, while fresh authored or
accepted `.SED` gameplay launch takes Full_Init. The Overlay-object report corrects the preview
report's provisional universal `first Building = 1,000,001`: matching storage skips every setup
constructor, while changed/missing storage runs its exact Cell/Type/House/Super prefix after reset.
The full-prefix report also replaces any provisional rejected-Tube continuation with the proved
spend-before-parse/hard-error matrix. Older documents remain useful for
their narrow evidence but have stale wording that must not drive implementation:

- older low reports say the reader “returns” for format `<=1`; only the two pack bodies skip, while
  the common drain and Full_Init sweeps remain;
- the older load timeline omits post-object `InitCellAttributes` and its second sweep;
- one fixed-map report says global Recalc observes the final state byte; Recalc never reads
  `Cell+0x11E`;
- one context report couples generated no-Mark to format `>1`; generated materialized is no-Mark
  regardless of its serialized output;
- `ASSET_PARSING_BRIDGES...` misclassifies `0xED/0xEE` as low; both are high NWSE anchors;
- `CELLCLASS_RECALCZONETYPE_00483C80...` places Land at `+0x48`; active Land is dword `+0xEC`;
- older scratch-cell wording implies low writes vanish; the one dummy retains explicit identity/state
  until another writer or Resize;
- `FUN_00483E30_BRIDGE_Z_AT_MAP_LOAD...` treats the six call-site literals as ordinary-cell outputs;
  native overwrites them through `FUN_00484180`, leaving only `(0,0)`/`(-1,-1)` neutral;
- any simple “both mover-cell bits mean row then column” description is wrong after a horizontal
  scan: the vertical gate reads the final row lookup/shared dummy and then scans mover-original X.
- any authored-wall predicate-failure fixture is wrong: successful Full_Init keeps ScenarioInit
  nonzero, so a slope-admitted authored wall takes wall effects plus the common queued tail. The
  three-broadcast wall rejection remains a separate generic counter-zero path.
- any reconstruction of authored blocker-neighbor counts from final `Wall=yes` identities is wrong:
  later low-body overwrites do not reverse earlier wall increments, and signed 512-stride lookups can
  alias another real cell.

## Verified matches and reusable partial matches

| Native requirement | Current Rust asset to preserve |
|---|---|
| Exact physical map provenance | `LoadedMapSource::{Loose,Mix,Generated,LegacyFallback}` already exists. |
| Signed format value | `BasicSection::new_ini_format: Option<i32>` preserves the key/default distinction needed to compute native default 0. |
| One post-Fill Scenario owner | Merged P0/P0-R1 `ScenarioBootstrapRng` owns the full logical cursor and can provide a narrow raw borrow. |
| Persistent fallback identity and Resize reconstruction | `SharedCellDummy` is one shared `Arc` identity; existing coordinate/level/slope/bridge facts and in-place reset tests must be extended, not replaced. |
| Useful Recalc submechanisms | Existing LAT, zone, CliffBack, overlay passability, and tile-animation helpers are reusable if orchestrated over live state in native order. |
| Generated direct overlay/data materialization | RMG emits its complete deck cells and current `GeneratedMaterialized` intent already avoids the late high-only authored stamp. Preserve this input, then apply generator-native Recalc/final-germination mutation; source authority must move from trace presence. |
| Runtime overlay mutation owner | `OverlayGrid` is already the live identity/state owner after load; only its construction authority must narrow to the finalized payload. |
| Collision-free runtime object handles | `src/sim/world/mod.rs::allocate_stable_id` already supplies one shared collision-free handle namespace for modeled Object analogues. Preserve it independently of the wrapping native numeric-ID cursor. |
| Shared live duplicate-aware finalization scan | `src/sim/world/substrate.rs::pending_delete` and `src/sim/world/lifecycle.rs::process_pending_delete` already preserve alive entries, collapse all selected duplicates, finalize once, and process the shifted successor. Reuse that algorithm at the reader boundary with the exact Overlay/base destructor ordering. |
| Current-wall owner reconstruction | The existing GSI-04.07 wall-owner helper is reusable; run it only after every final-current Recalc. A global post-Recalc pass is output-equivalent and avoids a second bridge owner. |

## Ghidra annotation candidates

The certainty-gated candidates in the initial re-swarm reports were synchronized only after all
workers stopped. Parent dry-run, apply, save, and readback passed for the selected call sites and
guards. No extra candidate was discovered by this Rust-only scan.

## Recommendation

Implement G1-G13 as one transaction with internally coherent commits, because splitting at the
current `ResolvedTerrainGrid`/`OverlayGrid` duplication would leave two authorities and an
unreviewable intermediate state. The smallest exact architecture is:

1. normalize physical source plus mandatory typed fresh family/native-ID receipt for every gameplay-
   equivalent Full_Init, then enforce the signed format gate only on the two pack bodies;
2. materialize Fill, apply the closed OQ-34 native-ID prefix through bound Tube constructors, then run one
   map-owned inline OverlayPack transaction with the sole borrowed raw Scenario adapter, extended
   shared dummy, load-object lifecycle sink, and authored-wall success/count-plane effects;
3. apply OverlayData while successful dead Overlay objects remain registered, then execute the
   unconditional shared live drain while preserving slope survivors;
4. execute the exact first live anti-diagonal Recalc, then emit and consume one identity/state/authored-
   blocker-count payload into runtime OverlayGrid, pathfinding state, and presentation without a final-
   wall reconstruction pass;
5. construct Terrain, initialize growth then spread queues from that live pre-object state, construct
   Technos/Smudge in native order while retaining the queue snapshot, then execute the second exact
   delete/unlatch/value-only-aggregate/Recalc animation boundary without rebuilding queues, storing
   the authored return at the map-load-state `+0x134` analogue until teardown reset;
6. retain generated direct materialization/no-authored-Mark, common seeded reader drain, while replacing the flat RMG replay
   with consumed-once synthetic/generator phase boundaries interleaving actual CABHUT constructions,
   every Neutral-Tech constructor, each native animation Recalc/lifecycle effect, pre-final ore-queue
   initialization, and final all-cell germination/local-value aggregation without queue rebuild;
7. add the separate process-shell preview lifecycle plus the common fresh-load native-ID cursor,
   keeping collision-free handles independent and applying exact replacement/Cancel/launch cleanup;
8. keep restore, positive Tube topology/traversal, later high topology, and later Road mutation on
   their independently proved arms.

Focused validation must use module-scoped `cargo test -p vera20k --lib ...` commands only. A fresh
critic must receive this requirement, the native reports, implementation contract, diff, and literal
test output; the full `cargo test -p vera20k --lib` runs exactly once at the final PR gate.
