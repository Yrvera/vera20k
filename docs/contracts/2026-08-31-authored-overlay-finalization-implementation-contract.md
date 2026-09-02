# Authored Overlay Finalization Implementation Contract

Date: 2026-08-31
Scope: Bridge transaction 3: fresh-load OverlayPack/OverlayData finalization, fixed-map low Mark, both load-time Recalc boundaries, generated phase preservation, and the native-identity/lifecycle prerequisites they share
Status: READY_FOR_IMPLEMENTATION — amended 2026-09-01 for verified authored-wall ScenarioInit success and retained blocker-neighbor counts
Landing status (2026-09-01): PR #207 delivered the authored arms of G1-G9, G12, and G13 (critic 3 PASS, no blocking findings). Still open for the transaction-3 continuation: G10, G11, the G6 value-only `MapClass+0x134` aggregate, the ancillary `InitCellAttributes` slot seam, the G7 `None`-plane rejection gate, the CellAnim tiberium remap/ZAdjust child fields, and OQ-37 (post-`Full_Init` OreTwinkle Scenario draws) — see the design ledger's "Transaction 3 residual ledger after PR #207".

## Gap Being Closed

VERA20k must replace its split raw-pack projections and flattened generated replay with one typed,
consumed-once load transaction that preserves active-retail source admission, Scenario and native-ID
continuation, inline Overlay object/Mark effects, finalized live cell state, animation and ore-queue
boundaries, and the preview/gameplay lifetime split.

## Scope

Included:

- every gameplay-equivalent fresh Full_Init family that can reach the authored reader: campaign,
  stock offline noncampaign, LAN/IPX, WOL state 2, and replay relaunch;
- physical source provenance independent of signed `NewINIFormat`, with an explicit generated
  no-authored-Mark arm and a separate stream-restore negative boundary that never creates a fresh
  descriptor;
- the fresh-load native numeric-ID prefix from Clear_Scene's `1,000,000`, through the applicable
  constructor stream, Fill/Resize, the map-read set-from-snapshot `+10,000` transform, and raw
  `[Tubes]` constructor attempts;
- one y-outer/x-inner authored OverlayPack transaction, native admission, ephemeral Overlay lifecycle,
  ordinary/high/low Mark, successful authored-wall stamp/connectivity/count effects, synchronous child
  animations, OverlayData, and the unconditional reader drain;
- the persistent shared-dummy identity/state seam, the first pre-Terrain anti-diagonal Recalc, and a
  non-Clone consumed-once finalized identity/state/authored-blocker-count payload;
- Terrain construction, growth-then-spread queue initialization before object sections, and the
  post-object argument-0 InitCellAttributes animation/value/Recalc/wall-owner boundary;
- generated `.SED` direct materialization without authored Mark, actual synthetic-Full_Init state,
  phase-aware generator Building/Anim/Recalc/queue/final-argument-1 lifecycle, and preservation of
  PR #170's preconsumed Techno words;
- active RMG preview native-object/native-ID/animation/sound/ore-queue lifetime across same-key and
  changed-key Generate, Cancel/re-entry, acceptance, and later gameplay launch;
- occupied-overlay presentation, atlas/name dependency, minimap/radar, and bridge presentation
  authority from final live simulation state;
- an exact ancillary ordering seam for tag-line clear/restamp, opaque-slot zeroing, light-cache
  invalidation/routing, and post-Recalc wall-owner reuse, including negative ownership assertions.

Excluded:

- positive Tube topology, hierarchy, direction 8, traversal, save/rebuild, or automatic-shell
  behavior; transaction 5 owns it, and receives only consumed-once successful `TubeNativeInit`
  bindings from this transaction;
- later high-bridge topology, records, zones, hierarchy, and edge restamp; transaction 4 owns those
  semantics after this transaction supplies their exact shared load contribution;
- runtime low-bridge mutation, repair, or collapse; this transaction closes only load-time low Mark
  and preserves the Road/Tube split;
- generic AttachedTag event-`0x19`/`0x1A` storage and FootClass consumer semantics, semantic rendered
  LightConvert/ZAdjust output, and the persisted/swizzled `Cell+0x30` pointer slot; their owners are
  respectively the generic trigger subsystem, transaction 20, and transaction 21/OQ-19;
- Cache-B byte `+9`, dummy save/restore, and native save/checksum decisions not proved by this load
  corridor;
- editor-only, TS-only, OpenTS rail, or dormant algorithms, and any inference that low Road Mark
  creates TubeClass;
- a literal C++ inheritance/vtable/global-vector port where a narrow Rust owner preserves the proved
  order, identity, failure, and result semantics.

This contract distills the approved architecture design through Revision 19 and the transaction-3 disparity
scan. Those documents are synthesis/navigation only; the native reports, retail data, and current
Rust reads below decide each row.

## Activation And Native Transaction Baseline

The three activation axes are independent:

| Axis | Active-retail rule | Contract consequence |
|---|---|---|
| Physical source | A successful fresh non-`.SED` load may come from Loose or MIX storage; a generated `.SED` is direct materialized; `LegacyFallback` is not a gameplay-equivalent parity source | Preserve `LoadedMapSource` provenance. Never infer generated authority from trace presence or a filename alone, and reject `LegacyFallback` before any native-ID, Scenario, Fill, or Mark effect in every gameplay-equivalent builder. |
| Fresh-load family | Campaign, stock offline, LAN/IPX, WOL state 2, and replay relaunch have distinct pre-Fill prefixes; stream restore does not enter Full_Init | Every gameplay-equivalent fresh path carries a typed family and consumed receipt; Generic/untyped paths reject before any ID or RNG effect. |
| Map format | `Read_INI_Basic` defaults missing `NewINIFormat` to signed zero; only signed value `>1` enables the two encoded pack bodies | Format `<=1` skips OverlayPack/OverlayData only. The native-ID prefix, raw Tubes boundary, shared drain, and ungated Full_Init sweeps still occur. |
| Persistence restore | A stream restore is a separate seed-zero Scenario restore transaction and never enters Full_Init, Fill, the fresh prefix, map reader, or authored pack routine | Keep `ScenarioRestoreContext` in persistence ownership. It is not a fresh-descriptor variant and no conversion between the two contexts is permitted. |
| Startup provenance | `LoadingStartup::Accepted` can normalize validated stock offline directly. `LoadingStartup::UnverifiedLegacy` is conditionally admissible only after `MatchLaunchDescriptor::from_resolved` and the existing prefix-plan validation both succeed. | Do not confuse a startup compatibility variant with physical `LoadedMapSource::LegacyFallback`. Resolved+validated legacy startup becomes the same typed stock-offline descriptor; unresolved/manual/failed validation rejects before effects and never panics or guesses. |

Every positive authored format-active descriptor owns the authoritative 32-bit fresh Scenario
seed/state, exact `LoadedMapSource`, ordered House/type inputs, active starts/waypoints, and exclusive
non-Clone `ScenarioBootstrapRng`. It must execute one of these exact pre-Fill algorithms:

| Fresh family | Exact normalized pre-Fill Scenario algorithm | Mandatory rejection |
|---|---|---|
| Stock offline noncampaign | Preserve merged P0-R1 exactly: House pass 1 -> selected common `+0x80` Gather -> selected Battle/Cooperative `+0x84` second Gather/chooser -> zero-Scenario-draw House/type reset -> identical House pass 2 -> Fill | Missing validated `MatchLaunchDescriptor`, active starts, stock callback family, or P0-R1 consumed receipt |
| Campaign | Construct the campaign House set once, in explicit `[Houses]` source-row order; if the section is empty, use verified registered-HouseType fallback order. Every successfully constructed House performs its rejection-capable `RandomRanged(450,1800)`. Run no multiplayer Gather/chooser, reset, disposable pass, or second House pass; then Fill. | Missing campaign tag, unresolved row/fallback construction order, or an attempt to reuse stock-offline roster/count |
| LAN/IPX Battle | Initialize from the network-authoritative seed -> disposable House pass -> common `+0x80` Gather -> Battle `+0x84` independent second Gather/chooser -> zero-draw reset -> identical final House pass -> Fill | Missing LAN provenance, normalized human/AI/Special slots, starts, ordered House inputs, or Battle callback family |
| LAN/IPX Cooperative | Initialize from the network-authoritative seed -> disposable House pass -> common `+0x80` Gather -> Cooperative `+0x84` independent second Gather/chooser with normalized human-prefix/AI-suffix partition -> zero-draw reset -> identical final House pass -> Fill | Missing LAN provenance, Coop partition/starts/House order, or Cooperative callback family |
| WOL state `2` | Initialize from the network-authoritative seed -> disposable House pass -> common `+0x80` Gather -> common `AssignStartingPoints`, which performs its second Gather plus only the zero-occupied player and exactly-two-occupied AI chooser draws -> zero-draw reset -> identical final House pass -> Fill | Selector not proved exactly `2`, unresolved player-versus-AI classification/starts/House order, or substitution of LAN selected `+0x84` |
| Replay fresh relaunch | Initialize fresh Scenario/Main state from the recorded seed/session -> add zero replay-specific Scenario calls -> dispatch the recorded inherited campaign, stock-offline, LAN Battle/Coop, or WOL-state-2 algorithm above -> Fill | Missing recorded provenance/seed, unknown inherited family, or any default-to-stock-offline fallback |

Fill and authored Mark borrow that same logical cursor. Acceptance must compare full logical cursor
checkpoints after every House pass, Gather/chooser, reset, Fill, each low-Mark row, and the first later
constructor; callback labels without state equality are insufficient. Current Rust can positively
construct only the validated stock-offline descriptor. Campaign/LAN/WOL/replay entry points remain
explicitly unsupported until an upstream owner supplies all normalized inputs above; they must never
borrow stock-offline state merely because a seed or authored filename exists.

Generated admission is narrower than physical `.SED` recognition. The only current positive gameplay
receipt is consumed-once accepted shell-RMG staging for stock-offline Battle id `1` or FFA id `2`,
bound to that selected record and its phase transport. Arbitrary/external/headless `.SED`, a filename
suffix, caller seed, trace-like artifact, unsupported mode, or absent/cancelled/replaced staging must
reject before receipt/native-ID/Scenario/Fill effects. Once the valid generated receipt is selected,
the path is generated-materialized and executes zero authored Mark regardless of serialized format or
construction-journal presence; a missing required phase journal then fails explicitly at its own
validation boundary and can never fall back to authored processing.

For authored fresh loads, the required live order is:

1. initialize the exact family-specific Scenario state and fresh native-ID cursor;
2. create the one staged `Simulation` load owner, execute the typed pre-Fill prefix, and run Fill on
   its sole Scenario stream;
3. discover required scheduler/assets roots without live effects, then retain failure after the
   already-spent prefix IDs and Fill RNG but before the first OverlayPack/Recalc Anim effect;
4. apply the map-read native-ID transform from saved cursor, consume raw `[Tubes]` source rows once,
   and retain successful parsed fact/native-ID bindings;
5. execute one inline OverlayPack row transaction, including authored-wall stamp/cleanup/connectivity/
   real-cell blocker-count effects and its common queued-object tail, then independent OverlayData,
   then the unconditional shared dead-object drain;
6. execute the first real-cell anti-diagonal Recalc, move the finalized identity/state/authored-
   blocker-count payload into the live OverlayGrid and global count owner, construct Terrain,
   initialize growth then spread queues, and construct later object sections;
7. immediately scalar-delete terrain-marked Anims, cross the ancillary slots, perform the authored
   value-only operation and final Recalc for each cell, then reconstruct current-wall ownership;
8. publish terrain/overlay/presentation authority from the final live simulation state.

For generated `.SED`, the encoded bodies remain inert and authored Mark never replays. The same staged
owner instead consumes actual synthetic-Full_Init state and the generator's native phase journal at
the proved Recalc, constructor, queue, and final argument-1 boundaries.

The independent wrapping signed-32-bit native-ID cursor obeys these proved formulas:

```text
R(W,H) = H * (2W - 1) + 1
HB(H,S) = H * (1 + S)

C_saved(campaign) = 1_000_000 + |E_campaign| + |P| + HB(Hc,S1) + R2
C_saved(noncampaign/.SED) =
    1_000_000 + |E_multi| + HB(H1,S0) + R1 + |P| + HB(H2,S1) + R2

map_read_cursor = wrap32(C_saved + 0x2710)
first_allocated_overlay = wrap32(C_saved + 10_000 + T + 1)
```

Here `E_*` and `P` are actual successful ordered constructor events, `T` is the count of successful
Tube allocations before the first Overlay, and every synchronous CellAnim or terrain Anim advances
the same cursor before the next Overlay. The map-read transform sets from `C_saved`; it does not add
to the then-current cursor. Collision-free Rust runtime handles remain a separate namespace and may
coexist with duplicate native numeric IDs.

Preview Generate uses a different formula and never executes the gameplay map-read `+0x2710`
reservation. Every Generate first frees spread then growth queues and resets the wrapping native-ID
cursor to `1,000,000`, without rewinding Scenario RNG, before choosing its setup branch:

```text
matching normalized (width,height,theater,player-count) key:
    setup constructor cost = 0
    first newly constructed generator object = 1_000_001

missing or changed normalized key:
    C_preview = wrap32(
        1_000_000
        + R(W,H)
        + |P_preview|
        + HB(H_preview,S_preview)
        + K_preview
    )
    first newly constructed generator object = wrap32(C_preview + 1)
```

Active retail data proves `K_preview=0`; the custom-theater constructor arm remains supported. The
numeric control `R=10`, `|P_preview|=5`, `HB=3`, `K_preview=0` yields
`C_preview=1,000,018` and first object `1,000,019`. Matching-key setup may therefore assign
`1,000,001` to a new object while retained cross-class objects already hold that numeric ID; the
stable runtime handles still must differ.

Acceptance does not clean the live preview queues. An accepted `.SED` gameplay launch has this exact
queue trace, including the apparently redundant second cleanup pair:

```text
launch generator entry:  FreeSpread -> FreeGrowth
Full_Init/Clear_Scene:    FreeSpread -> FreeGrowth
Full_Init rebuild:        BuildGrowth -> BuildSpread
```

The two free pairs must not be collapsed, deduplicated, or inverted, and there is exactly one rebuild
pair after them. Cancel and no-Generate re-entry perform none of these events; their retained queues
survive until the next Generate or accepted launch reaches its proved owner.

The fixed map lookup is also exact-width behavior, not shorthand array indexing. Each coordinate is
first narrowed/wrapped to signed `i16` at the native lookup call boundary. Native then computes

```text
linear = sign_extend_i16(y) * 512 + sign_extend_i16(x)
```

in signed 32-bit arithmetic and only afterward applies linear-range and null-slot admission. The
product/sum is never truncated to `i16`. Thus a negative coordinate can legally alias a real slot;
the decisive control `x=-510,y=2` yields linear index `514`. Only a failed range/null-slot admission
returns and stamps the one persistent shared dummy.

The authored identity body has this exact ordered admission/construction sequence:

1. signed `NewINIFormat > 1`, then an independently signed-positive decoded `OverlayPack` length;
2. y-outer/x-inner byte read; failed read or decoded `0xFF` leaves the `-1` sentinel and skips;
3. direct unsigned `0..254` OverlayType registry access—native has no count/index/null guard, so Rust
   may hard-error malformed state for memory safety but must not call it a native rejection filter;
4. admit when the type's virtual image lookup is non-null **or** `CellAnim` is non-null;
5. admit a crate only when `g_GameMode == 0`; non-crates remain admitted in every mode;
6. for `W=Map+0xF4`, `H=Map+0xF8`, require all four radar inequalities:
   `W < x+y`, `x-y < W`, `y-x < W`, and `x+y <= W+2H`;
7. attempt the `0xB0` Overlay allocation. Null performs no constructor, stable handle, native ID,
   registry join, Mark/dirty, child, or queue effect; the later four-high anchor restore check remains
   a harmless no-op;
8. after allocation/base construction, attempt the Object registry, pointer-expiration listener,
   all-Abstract listener, and Tag-removal listener joins in that order; then assign the preincremented
   native numeric ID; then attempt the Overlay registry join. Native joins are best-effort, while Rust
   deliberately hard-errors any injected growth failure rather than accepting a partial object;
9. direct-call base `ObjectClass::Unlimbo`, whose virtual Mark first performs base tactical dirty and
   then derived dispatch. Derived Mark rejects slope `>4` for every type except `0xB2`, after base
   dirty but before any high/low/ordinary cell write, Recalc, or common-tail UnInit.

All ordinary pre-allocation reader rejections have zero allocation/ID/registry/Mark/dirty/queue
effects. OverlayData has its own independent signed-positive length and uses only the same four radar
inequalities before writing a real cell; it does not inherit identity, type, art, crate, allocation,
or slope admission.

## Evidence Baseline

| Source | Role | Use |
|---|---|---|
| `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_MARK_LOAD_CONTEXT_SOURCE_PROVENANCE_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY | Physical source, fresh-family, replay/generated admission, separate restore exclusion, and Scenario ownership |
| `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_OVERLAYPACK_INLINE_TRANSACTION_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY | Reader gate, y/x order, filters, ordinary/high/low dispatch, tactical dirty, OverlayData, and both Full_Init sweeps |
| `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_OVERLAY_EPHEMERAL_OBJECT_FINALIZATION_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY | Overlay allocation/registry/ID/Mark/UnInit order, wall and slope paths, shared drain, survivor lifetime |
| `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_OVERLAY_WALL_SCENARIOINIT_ACCEPTANCE_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY | ScenarioInit wall-predicate bypass, stamp/cleanup/connectivity/common-tail order, compact active retail IDs/census, signed aliases, and retained wrapping count plane |
| `docs/research/CELL_0X122_DYNAMIC_BLOCKER_LIFECYCLE_RUST_MAPPING_GHIDRA_REPORT.md` | PRIMARY | Global count reader/writer shapes and authored-baseline/runtime-lifecycle composition rule |
| `docs/research/bridges/01-assets-map-load-overlay/LOW_OVERLAY_MARK_FIXED_MAP_STAMP_RNG_TRANSACTION_GHIDRA_REPORT.md` | PRIMARY | Eight low trigger ids, fixed/search/body tables, exact raw `3*L` Scenario transaction |
| `docs/research/bridges/01-assets-map-load-overlay/LOW_OVERLAY_MARK_SCENARIO_LOAD_ACTIVATION_BOUNDARY_GHIDRA_REPORT.md` | PRIMARY | Fixed-map low activation and zero-draw non-body arms |
| `docs/research/bridges/01-assets-map-load-overlay/LOW_OVERLAY_MARK_ALL_LOAD_CONTEXT_SCENARIO_RNG_LIFECYCLE_GHIDRA_REPORT.md` | PRIMARY | Low Mark cursor continuation across active fresh-load families |
| `docs/research/bridges/01-assets-map-load-overlay/OVERLAYPACK_SHARED_DUMMY_FINAL_RECALC_FIELDS_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY | Signed fixed-stride lookup, persistent dummy, dummy no-op Recalc, and minimum final payload |
| `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_TIBERIUM_GERMINATE_SIDE_EFFECT_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY | Exact Land-5 receiver-only density transaction and authored/generated ore-queue boundaries |
| `docs/research/bridges/01-assets-map-load-overlay/TERRAIN_ATTACHED_ANIM_LOAD_LIFECYCLE_SIDE_EFFECTS_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY | First-generation Recalc Anim ordering, native ID/registry/RNG/Middle/sound effects, scalar deletion and recreation |
| `docs/research/bridges/00-system-models/RMG_PREVIEW_ANIM_BUILDING_IDENTITY_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY | Preview setup branches, reset/cleanup, active Building/Anim/sound lifetime, duplicate native IDs, Cancel/re-entry |
| `docs/research/bridges/01-assets-map-load-overlay/FULL_INIT_AND_PREVIEW_NATIVE_ID_PREFIX_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY | Exact preview/fresh-load ID prefixes, wrapping map-read transform, raw Tube fault matrix, first-Overlay oracle, queue lifetime |
| `docs/research/bridges/01-assets-map-load-overlay/INITCELLATTRIBUTES_TAG_LINE_LIGHTING_TAIL_REINVESTIGATION_GHIDRA_REPORT.md` | PRIMARY | Final ancillary slot order, generic tag-line exclusion, light routing, opaque slot, and current-wall owner timing |
| `gamemd.exe` `0x00598E48`, `0x00598FE7`, `0x00599153`, `0x005A4259`, `0x0059937D`, `0x0059944C` | PRIMARY | Generated Recalc/constructor/resource/final-Init phase order |
| active retail `rulesmd.ini`, `artmd.ini`, `all01umd.map`, and 385-payload map census | PRIMARY | Four high ids, active TileAnim/RandomRate/sound variants, event-`0x1A` reachability, format-4 and zero shipped low-trigger facts |
| Current Rust owner set at `origin/main@50e4b7ba4732fd3fb48e5b819e1abc55327ec557` | PRIMARY | Directly read implementation baseline; PR #197 is test-only and does not touch this scope |
| `docs/gap-scans/2026-08-31-disparity-scan-authored-overlay-finalization.md` | DERIVATIVE | Bounded 25-mechanism inventory and G1-G13 reconciliation |
| `docs/plans/2026-08-28-active-retail-bridge-parity-design.md` Revision 19 | SYNTHESIS | Approved transaction ownership, exclusions, ordering, and critic protocol |
| Research-index brief/handoff results | NAVIGATION_ONLY | Index health and older bridge-document discovery; no exact new worktree report was indexed |
| `C:\Users\enok\Documents\OpenTS` | NAVIGATION_ONLY | Inherited naming and function-location leads only; no parity conclusion or algorithm is taken from it |

## Parity Delta Table

| Evidence class | Delivery class | Mechanism/result | gamemd.exe behavior | Current Rust behavior | Required Rust delta | Evidence | Acceptance test |
|---|---|---|---|---|---|---|---|
| REQUIRED_FIX | MILESTONE-BLOCKING | G1: typed source/family/format/receipt and exact pre-Fill cursor boundary | Physical source, startup provenance, signed format, and fresh family are independent; each positive campaign/stock/LAN-Battle/LAN-Coop/WOL2/replay descriptor runs the exact normalized algorithm above on one Scenario stream before Fill/Mark; Accepted and resolved+validated UnverifiedLegacy can supply stock offline; only accepted stock Battle/FFA RMG staging supplies generated; physical LegacyFallback, unresolved/manual legacy startup, external `.SED`, unsupported generated combinations, and Generic reject before effects; restore remains separate | Source and format storage exist, but generated is inferred from trace presence; `LoadingStartup` cannot represent non-stock families; UnverifiedLegacy currently traverses a compatibility resolution path with an `expect`; headless hardcodes authored; physical LegacyFallback rejection is narrow | Add the typed descriptor and non-Clone receipt; implement exact family algorithms/cursor checkpoints; normalize UnverifiedLegacy only after `MatchLaunchDescriptor::from_resolved` plus prefix-plan validation, returning an explicit error on unresolved/manual/failure rather than panic or variant-wide rejection; derive generated only from consumed accepted id-1/id-2 staging; universally reject physical LegacyFallback/Generic before effects; keep restore separate; return one post-Fill cursor | Context report §§6.2-6.4 and handoff; P0-R1 report/tests; `src/app/frontend/list_maps.rs:31-51`; `src/map/basic.rs:18-35,68-82`; `src/match_bootstrap.rs:80-128`; `src/app/shell_skirmish.rs:256-264`; `src/app/loading/pump.rs:217-269,359-439`; `src/app/loading/init.rs:1867-1884`; `src/headless_scenario.rs:75-116` | Full cursor tests cover all families; ingress tests distinguish Accepted, resolved+validated UnverifiedLegacy positive, unresolved/manual/failed-plan legacy negative with no panic/effects, physical LegacyFallback universal negative, generated staging matrix, Generic, and separate restore |
| REQUIRED_FIX | MILESTONE-BLOCKING | G2: one inline authored row transaction | Signed format and positive body gate a `y=0..511`, `x=0..511` traversal; each row applies failed-read/`0xFF`, unguarded type access, image-or-CellAnim, nonzero-mode crate reject, four exact radar inequalities, and allocation gates in order; allocation then performs four base joins, native ID, Overlay join, base dirty, slope gate, derived Mark, child effects, and tail synchronously before the next row | Parsing collapses rows, resolved terrain preprojects them, a late high-only loop runs separately, and OverlayGrid decodes/filters them again | Add one map-owned routine over ordered bytes and one live cell surface, with the complete nine-step sequence above, a narrow raw-Scenario and load-effect sink, deliberate hard load errors for malformed type and registry growth, and no second decoder/filter/Mark authority | Inline report §§3-7; ephemeral report §§1-3; `src/map/overlay.rs:129-176`; `src/map/resolved_terrain.rs:2051-2082,2438-2481`; `src/sim/overlay_grid.rs:198-258` | An interleaved fixture pins every gate and side-effect boundary, radar equalities, null allocation, four joins/ID/Overlay join, slope `>4`/`0xB2`, row/Recalc/dirty/child/state order, and data's radar-only independence |
| REQUIRED_FIX | COMPOUNDING | G3: exact fixed-map low procedural Mark | IDs `0x7A..0x7D` and `0xE9..0xEC` run settled fixed/search/body tables inline; successful length `L` consumes exactly `3L` raw `Next() & 3`; all other arms consume zero; writes use the signed-i32 linearized real-or-dummy seam and never create Tubes | No Rust owner implements low tables or accepts the post-Fill Scenario cursor; late bridge projection only handles high stamps | Implement the settled algorithm inside the row transaction using a raw-only Scenario adapter; narrow/wrap x/y to `i16` before lookup, sign-extend each operand and compute `y*512+x` in signed i32 before admission; use no ranged helper, clone, reseed, component pass, Tube synthesis, or post-hoc reordering | Three low reports; Inline report §§5-6; dummy report lookup proof; `src/sim/scenario_bootstrap.rs:2064-2180`; `src/map/resolved_terrain.rs:2438-2470` | Table/geometry fixtures cover all success/no-op/failure/search arms, exact `3L` logical cursor continuation, occupied writes, edge dummy misses, `(-510,2)->514` real-slot alias, overwrites, and first later constructor state |
| REQUIRED_FIX | MILESTONE-BLOCKING | G4: ordinary/high writes, germination, and tactical dirty | Every allocated object dirties tactical once before derived dispatch; ordinary writes identity/state 0, Land-5 state 1 then exact N..NW same-class density `[0,1,3,4,6,7,8,10,11]`, crates write `0xFF` last; high owners preserve only anchor state around temporary `0/9` and Recalc | Current projections approximate ordinary state, separate high stamping, omit exact germination and object-level dirty timing | Put all writes and one dirty intent in G2's transaction; preserve high anchor restore, receiver-only zero-argument germination, class/range fallback, persistent-dummy aliases, and zero RNG/queue/Recalc from germination itself | Inline report §§6-9; germination report §§4-8; `src/map/resolved_terrain.rs:2438-2481`; `src/sim/overlay_grid.rs:219-258` | Four-high fixtures plus Land-5 early-zero/class/range/dummy/density/no-data/data-override/crate/slope cases pin exact bytes and negative effects |
| REQUIRED_FIX | MILESTONE-BLOCKING | G5: OverlayData then first live Recalc | Positive OverlayData independently overwrites admitted real-cell state after all identity rows; the ungated first sweep then visits `H*(2W-1)` real cells in anti-diagonal order before Terrain, validates identity and refreshes LAT/slope/Land/zone/cache/Anim without reading state | Two nominal sweeps run before overlay admission in linear/precomputed terrain construction; data is copied later into separate projections | Split Fill from finalization, apply data once, drain, then run one exact live anti-diagonal sweep and capture its validated identity/state | Inline report §§8-11; dummy report §§4-7; `src/map/resolved_terrain.rs:2033-2039`; `src/map/lat.rs:331-375`; `src/sim/overlay_grid.rs:246-257` | Format-inactive, absent, empty, identity-empty, rejected-identity/data-only, exact coordinate/count, live-neighbor, identity-clear, and data-not-read-by-Recalc fixtures |
| REQUIRED_FIX | MILESTONE-BLOCKING | G6: authored Terrain/queue/post-object Init boundary | First-generation terrain Anims arise at their first per-Mark or first-sweep Recalc; after Terrain, native builds all growth then all spread queues before later objects; after objects/Smudge it scalar-deletes marked Anims, clears/routes ancillary slots, value-accounts, unlatches, Recalcs/recreates, and only then reconstructs current-wall owner without rebuilding queues | Terrain/objects and both sweeps are folded into a monolithic resolved grid; descriptors are precomputed/sorted and spawned last; post-map rebuilds queues after all objects and interleaves growth/spread; `finalize_constructed_scenario` currently invokes the reusable wall-owner helper before the absent post-object/final-Recalc owner | Stage the real Simulation before Fill; use pure root discovery then synchronous Anim sink; clear Terrain-source ore during live Terrain construction; seed whole growth then whole spread immediately after Terrain; add immediate marked-Anim scalar deletion and exact final live cell pass; retain queue snapshot and authored local total/state until teardown reset; relocate the existing wall-owner helper invocation after every final-current Recalc without changing its algorithm | Anim report §§3-10; germination report §§5-9; ancillary report §§3-8; `src/sim/runtime.rs:505-663,732-825`; `src/sim/scenario_post_map.rs:41-86`; `src/sim/ore_growth.rs:896-960`; `src/sim/terrain_spawn.rs:731-776` | First-generation order/latch, custom RandomRate, stock-zero Main-RNG negative, sound/Middle, post-Terrain queue snapshot against later occupier, immediate delete/no StopSound/Expire/pending, value formula/wrap, final recreation, no queue rebuild, and production trace proving wall ownership reads only post-final-Recalc current identities |
| REQUIRED_FIX | MILESTONE-BLOCKING | G7: consumed-once finalized overlay payload | Later owners receive validated real identity, final state byte, and the authored real-cell blocker-neighbor plane; derived terrain fields remain in MapClass and no pack is decoded or authored walls reconstructed again | The transaction branch now implements the non-Clone identity/state/count payload, shape-checked OverlayGrid install, persistence/hash, and `Some`-plane pathfinding baseline; production/headless builders still do not consume it and therefore remain legacy `None` | Replace production/headless raw constructors with the one finalized-payload move; expose no raw-pack/rules/RNG/filter/Recalc/final-wall-scan interface; reject malformed retained storage and, after migration, current-version `None` on save/restore. If any v114 `None` snapshot escapes before closure, bump again instead of reconstructing from final identities | Dummy report §§6-8; authored-wall report §§3-8; `src/map/authored_overlay.rs`; `src/sim/overlay_grid.rs`; `src/sim/movement/bump_crush.rs` | Compile/API tests make duplicate consumption and second decode impossible; cell-for-cell identity/state/count fixtures match live state, later mutation, deterministic digest, authority-mode hashing, malformed retained-storage rejection, and the final no-`None` persistence gate |
| REQUIRED_FIX | COMPOUNDING | G8: final-state presentation authority | Procedural identities and surviving post-Recalc state are render/minimap/bridge authority; rejected or cleared identities are absent | Atlas/name and render rows originate from raw `map_data.overlays`, then merely filter against a later grid | Build occupied render/name/minimap/bridge entries from the final sim-owned OverlayGrid/terrain; registry-wide low-variant preload may remain broad but raw pack membership is never occupancy | Inline report final consumers; `src/app/frontend/skirmish.rs:2638-2659,2684-2688,2713-2726,2800-2860`; `src/app/loading/init.rs:2335-2382` | A trigger-created body absent from raw rows renders and appears in atlas/minimap/bridge inputs; rejected/Recalc-cleared rows appear nowhere |
| REQUIRED_FIX | MILESTONE-BLOCKING | G9: production/headless/auxiliary boundary equivalence | Every gameplay-equivalent builder observes the same typed family, source, format, prefix, IDs, RNG, effects, and failure point | Headless always chooses authored and uses weaker `from_overlay_entries`; auxiliary builders can silently take untyped pure-map behavior | Thread the same descriptor/receipt and staged orchestrator through production, headless, and parity builders; retain only a named zero-live-effect diagnostic that cannot certify gameplay parity | Context report; `src/headless_scenario.rs:75-116,202-241`; `src/map/resolved_terrain.rs:1818-1925`; `src/app/loading/init.rs:805-813` | Typed production/headless parity fixtures for authored format 4, authored absent/1, and generated; untyped Generic rejects before native ID or Scenario draw |
| REQUIRED_FIX | MILESTONE-BLOCKING | G10: generated synthetic/generator phase lifecycle | Format-zero synthetic Full_Init still runs ungated boundaries on actual staged state; generator order is CABHUT, Recalc `0x00598E48`, Neutral-Tech constructors, later Recalcs `0x00598FE7/0x00599153/0x005A4259/0x0059937D`, queue init, then `InitCellAttributes(1)` germination/value/Recalc with no rebuild | `RmgConstructionTrace` is flat Building-only data applied before one final Anim descriptor set; final emitted cells cannot encode intermediate Anim IDs/RNG/sound/queue state | Replace/widen it with a consumed-once phase journal emitted inside the generator pipeline and applied by the staged Simulation; carry Building word then native ID then outcome, preserve discarded/failed distinctions and PR #170 word, capture actual synthetic state, interleave live Anim effects, seed queues pre-final-Init, and run exact arg-1 helper/value/Recalc without a persistent total | Preview report §§5-9; anim report generated sections; germination report §§4-9; `src/map/construction_trace.rs:6-36`; `src/map/rmg/pipeline.rs`; `src/sim/scenario_bootstrap.rs:2141-2180`; `src/sim/runtime.rs:620-663` | A poisoned staged synthetic control and phase-interleaving fixture prove each boundary, failed CABHUT zero effects, discarded Neutral-Tech consume/no-bind, emitted binding reuse without second draw/ID, custom RandomRate, pre-final queues, arg-1 density/value/wrap, no persistent total, and missing-asset hard failure |
| REQUIRED_FIX | MILESTONE-BLOCKING | G11: preview lifetime plus independent fresh-load native IDs and raw Tubes | Every preview Generate frees spread then growth and resets ID to `1,000,000`; matching key spends zero setup IDs and gives the first new object `1,000,001`; missing/changed key advances through `R+|P_preview|+HB+K_preview` and gives the first object `C_preview+1`, with retail `K_preview=0`; Cancel/re-entry retains live queues; accepted `.SED` runs two distinct spread-then-growth free pairs followed by exactly one growth-then-spread rebuild; preview permits duplicate numeric IDs and never takes Full_Init/map-read `+0x2710`; gameplay fresh load separately uses exact `C_saved`, set-from-snapshot `+10,000`, then spend-before-parse Tube construction | Offline runtime has no preview-native owner/cursor; shell retains UI payload only; Anim RandomRate precedes stable-ID allocation and aliases stable ID as native ID; filtered `explicit_tubes` drops malformed rows | Add `PreviewNativeLifecycle` to the process shell, independent wrapping native-ID cursors and bindings, distinct stable handles, branch/token/latch/live-order retention, exact cleanup/queue order, and accepted-launch separation; preserve both accepted-launch cleanup pairs without deduplication and the single rebuild; encode explicit preview formulas with no map-read reservation; gameplay consumes raw ordered Tube values once, assigning before parse, hard-errors at proved allocation/parse points, and emits `TubeFact + TubeNativeInit` without topology | Preview report §§2-10; prefix report §§4-12; `src/app/frontend/skirmish_session.rs:86-92`; `src/app/shell_random_map.rs:157-217,377-398`; `src/sim/anim_class.rs:526-595`; `src/map/map_file.rs:242-245`; `src/map/tubes.rs:17-85`; `src/rules/ini_parser.rs:19-25,156-162` | Same/changed/missing key, terminal churn, Cancel/no-Generate/re-entry, accept-versus-launch; matching `1,000,000 -> 1,000,001`; rebuild `R=10,P=5,HB=3,K=0` gives `1,000,018 -> 1,000,019`; no preview `+0x2710`; accepted launch exact `FreeSpread,FreeGrowth,FreeSpread,FreeGrowth,BuildGrowth,BuildSpread`; legal duplicate numeric IDs; gameplay `1,000,018 -> 1,010,018`, `1,000,037,T=0 -> O1=1,010,038`, `0xFFFFFFF0 -> 0x00002700`, `T=0/2`; Tube fault/binding tests |
| REQUIRED_FIX | MILESTONE-BLOCKING | G12: real load-object registries, drain, and slope survivor | Each allocated Overlay joins base registries, receives native ID, joins Overlay registry, direct-base Unlimbos/Marks, then common success (including every slope-admitted authored wall) or steep-slope survivor lifecycle; dead objects remain registered through data and drain once outside the format gate; slope survivors remain registered until scene teardown but never render/save/checksum. Generic counter-zero wall rejection exists outside authored Full_Init. | The transaction branch has a dedicated load-object lifecycle, but its generic `finish_wall_reject` must not be selected by authored finalization | Retain the dedicated lifecycle and exact registry/order/state/broadcast/queue events; route slope-admitted authored walls through common two-broadcast finalization after G13 effects; preserve generic counter-zero rejection as a separate non-authored method; drain in reader epilogue and retain slope survivors through scene teardown | Ephemeral report §§4-12 plus 2026-09-01 correction; authored-wall report §§2-4; `src/sim/world/load_object_lifecycle.rs` | Common/authored-wall two-broadcast, separate generic counter-zero three-broadcast/full-Limbo, slope state, data-before-drain visibility, mixed duplicate queue, body-absent/format-1/generated seeded drain, growth failures, exact destructor removal order, no ID refund, and slope no-grid/entity/render/save/hash plus teardown release |
| REQUIRED_FIX | MILESTONE-BLOCKING | G13: authored wall success and retained blocker-neighbor plane | Nonzero ScenarioInit forces the post-slope wall predicate true; Mark stamps identity/state, performs N/E/S/W/self cleanup and same-ID connectivity, keeps owner `-1`, increments eight signed-fixed-map neighbor counters with wrapping `u8`, then uses the common Recalc/UnInit tail. OverlayData and later low identity overwrites do not rebuild or reverse those counts. Runtime direct removal Recalcs before cleanup and decrements after the full fan-out; every cleanup wall Recalcs in visit order and auto-removal decrements only after a changed-zone Recalc; wall sale leaves the sold anchor stale. Terminal cleanup receivers are N/W/S/E while chain recursion is N/E/S/W. Runtime true-dummy identity/state and packed tactical/radar output remain live, but authored/runtime dummy counts never enter the real plane. Only GASAND/GAWALL/NAWALL hardcoded cleanup rows are active retail; the isolated-damaged cleanup input is code/data-conditional with no established shipped-map or ordinary placement witness. | Commit `95f77159` implemented and focused-validated the wall helper, retained plane, persistence/pathfinding consumption, real/dummy lookup, and Recalc/count/pointer/navigation core, but fresh critic 1 returned `NEEDS_FIX`: production deferred terminal radar, omitted damage tactical dirty, and reordered placement/sale effects. The synchronous host correction across damage, sale, and placement passed its correction-focused matrix; a new fresh critic remains mandatory. The helper/payload install also lacks a production authored-reader callsite, and native non-entity pointer listeners remain unrepresented | Obtain a new fresh critic verdict that rechecks every critic-1 finding. Then execute the wall helper inside the ordered row transaction, deliver cleanup/count/common effects synchronously to the real load owners, and consume the finalized payload in production/headless construction without scanning final walls. Keep the unrepresented listener roster open unless a later verified owner closes it | Authored-wall report; Cell+0x122 lifecycle report; wall destruction report; `src/map/authored_overlay.rs`; `src/sim/overlay_grid.rs`; `src/sim/movement/bump_crush.rs`; `src/sim/pathfinding/core.rs`; `src/sim/superweapon/lightning_storm.rs` | ScenarioInit bypass, cleanup/connectivity/owner, overlap wrapping, low overwrite retention, authored and runtime fixed-stride aliases, authored dummy-count negative plus runtime dummy tactical/radar positive, direct/cleanup/sale count-tail and pointer-expiry order, active-retail Lightning navigation/tactical/radar, runtime placement owner/count/common-tail order, OverlayData no-count-write, common lifecycle, active-only thresholds, and consumed-once no-final-scan fixtures |
| TEST_ONLY | MILESTONE-BLOCKING | P1: merged Scenario prefix and PR #170 generated constructor binding | One post-Fill Scenario cursor continues into Mark/generator events; emitted/discarded constructors preserve the already-consumed word contract | P0/P0-R1 and PR #170 already implement the cursor/bound-word portions | Reuse their owner and fixtures; do not reopen formulas or draw again when widening the phase transport | `src/sim/scenario_bootstrap.rs`; merged PR #170 and PR #196 tests | Existing prefix and construction-trace families remain green beside new end-to-end cursor fixtures |
| TEST_ONLY | MILESTONE-BLOCKING | P2: stable handles and one shared dummy identity | Native numeric IDs may duplicate; failed map-coordinate admission shares one persistent dummy whose Recalc is a total no-op | Rust already has a collision-free stable-ID allocator and one `Arc` shared dummy identity | Preserve both existing identity shapes while G3/G4's required fixes add only proved signed-dword overlay identity and byte state to the dummy; keep real-cell animation latch and foreign ancillary storage out of this preservation row | Dummy report; Anim report dummy early return; `src/sim/world/substrate.rs:148-168`; `src/sim/world/mod.rs:3968-3977`; `src/map/resolved_terrain.rs:774-886` | Duplicate-native-ID objects keep distinct handles; repeated true misses observe the last dummy overlay identity/state, dummy Recalc changes nothing, and no dummy latch/tag/opaque field is added |
| TEST_ONLY | COMPOUNDING | P3: current-wall owner reconstruction algorithm only | Given the post-final-Recalc current wall identity, native selects the nearest eligible Building with the already-verified GSI-04.07 semantics | The existing helper's selection algorithm is semantically reusable; its production call-order mismatch is classified and fixed by G6, not by this preservation row | Preserve the helper algorithm and its focused unit tests unchanged while G6 relocates the sole production invocation; do not introduce a second owner or algorithm | Ancillary report wall tail; `src/sim/runtime.rs:732-825`; `src/sim/overlay_grid.rs:368-419` | Existing focused helper fixtures remain green; G6's separate production trace proves the relocated post-final-current call order |

All thirteen required gaps are current transaction work. `COMPOUNDING` describes trigger frequency,
not permission to defer: shipped-map low-trigger frequency is zero in the bounded census, but the
active custom/editor-compatible mechanism shares the deterministic cursor and must close before this
transaction can pass.

## Required Rust Changes

### G13 authored-wall effect ledger

For every reader-admitted, allocated `Wall=yes` row that passes the universal slope gate, preserve this
order exactly:

1. use successful Full_Init's nonzero ScenarioInit context to guarantee the wall predicate result;
   do not execute or approximate the counter-zero predicate body;
2. stamp anchor state `0` and the compact overlay identity;
3. call the cleanup cross in `N,E,S,W,self` order. At each visited wall, probe `N,E,S,W` and connect
   only the same compact identity, preserve the upper state/damage nibble while replacing the lower
   connectivity nibble, run its `RecalcAttributes(-1)` and zone-change work, and issue that visit's
   tactical/radar dirty effects;
4. admit no Building/gate/tower connection before Buildings exist and no damaged-wall auto-destruction
   during this authored identity pass;
5. let ScenarioInit skip only Mark's later explicit `MergeAdjacentCellZone`/incremental-zone pair;
   it does not suppress the cleanup Recalc/zone work;
6. leave wall owner `-1` because the authored constructor's pending owner is `-1`;
7. increment `N,NE,E,SE,S,SW,W,NW` neighbor counters through the signed fixed-map seam with raw
   wrapping `u8`, retaining real and aliased-real output but exporting no true-dummy count;
8. run the common second anchor Recalc, then common OnMap/Limbo/UnInit/death/queue finalization;
9. let later OverlayData replace only the final state byte. It neither reruns cleanup nor changes the
   count plane; if OverlayData is absent, Mark-derived connectivity remains.

### Typed launch and staged owner

- `src/match_bootstrap.rs`, `src/app/loading/pump.rs`, and `src/app/loading/init.rs`
  - add one immutable typed fresh-load descriptor containing physical source, fresh family, signed
    format, generated arm, and a structurally consumed-once prefix receipt;
  - implement the tabled campaign, LAN Battle, LAN Cooperative, WOL-state-2, and inherited replay
    algorithms on the one Scenario owner, while preserving merged stock-offline P0-R1; reject any
    family whose upstream normalized inputs are unavailable rather than synthesizing them;
  - admit generated materialization only from consumed accepted shell-RMG staging bound to stock
    Battle id `1` or FFA id `2`; reject arbitrary/external/headless `.SED`, cancelled/replaced/absent
    staging, unsupported modes, and source/context disagreement before effects;
  - resolve `LoadingStartup::UnverifiedLegacy` through `MatchLaunchDescriptor::from_resolved` and the
    existing prefix-plan validator before constructing typed stock offline; resolved+validated is a
    positive compatibility path, while unresolved/manual/failed validation returns an explicit
    pre-effect error instead of `expect`, fallback, or variant-wide rejection;
  - reject `LegacyFallback` and `Generic` before any receipt, native-ID, Scenario, Fill, or Mark
    effect for every gameplay-equivalent production/headless/auxiliary builder; keep a separately
    named pure-map diagnostic incapable of parity certification;
  - keep persistence-owned `ScenarioRestoreContext` out of `LoadingRequest`, `MapLoadInitial`, the
    fresh descriptor, Fill, and the pack routine; provide no conversion between restore and fresh
    contexts;
  - move, rather than clone/install through `&self`, the prefix receipt and one
    `ScenarioBootstrapRng` into a staged Simulation before Fill;
  - preserve exact state on asset-root failure: family prefix native IDs and Fill RNG are already
    spent, but no OverlayPack/Recalc Anim construction has happened.
- `src/sim/scenario_bootstrap.rs` and `src/sim/runtime.rs`
  - expose narrow Fill/Main wrappers plus a raw Scenario callback; never expose a second RNG owner;
  - own the independent wrapping native numeric-ID cursor beside, not inside, the stable-handle
    allocator;
  - retain one staged Simulation identity through final gameplay construction—no late Simulation,
    shadow registry, or ownership transfer.

### Map-owned authored finalization

- `src/map/resolved_terrain.rs`, `src/map/overlay.rs`, `src/map/lat.rs`, and a narrow map-side effect
  interface
  - split Fill materialization from load finalization and make the production staged API return
    `Result` rather than silently constructing a partial `Self`;
  - discover scheduler/Anim roots purely after Fill, then execute the sole OverlayPack transaction,
    OverlayData, shared drain, and exact first anti-diagonal Recalc;
  - extend `SharedCellDummy` in place with only the proved signed-dword overlay identity and byte
    state; narrow/wrap each coordinate to signed `i16`, then compute
    `sign_extend(y)*512+sign_extend(x)` in signed i32 before range/null-slot admission; never truncate
    the linear result to `i16`; preserve dummy no-op Recalc and add no animation-latch, tag-bit,
    opaque-slot, or light field to the dummy;
  - dispatch each slope-admitted authored `Wall=yes` row through ScenarioInit-forced success: stamp,
    N/E/S/W/self cleanup, same-ID cardinal connectivity, owner `-1`, eight signed-fixed-map wrapping
    count writes, anchor Recalc, and the common load-object tail; do not call the generic wall-reject
    path from this transaction;
  - return derived terrain plus a separate non-Clone `FinalizedOverlayPayload` containing final real
    identity/state and the authored real-cell blocker-neighbor plane; do not store that payload in a
    Clone terrain grid, reconstruct it from final wall identities, or export dummy count state.
- `src/map/map_file.rs`, `src/map/tubes.rs`, and `src/map/tube_facts.rs`
  - enumerate raw `[Tubes]` values in source order exactly once, not `explicit_tubes`;
  - allocate/assign the numeric ID before token parsing, hard-error malformed allocated rows and
    allocation-null rows at their distinct points, and emit a source-record-keyed successful
    `TubeNativeInit` binding for transaction 5 with no topology or second ID allocation.

### Simulation lifecycle and live cell authority

- `src/sim/overlay_grid.rs`
  - replace the production raw-pack constructor with `from_finalized_map_payload` and no rules,
    source filter, Mark, RNG, or Recalc capability;
  - keep subsequent mutations synchronized with resolved terrain through the current live authority.
- `src/sim/movement/bump_crush.rs` and `src/sim/pathfinding/core.rs`
  - consume the authored count plane as the baseline of the sole global `BlockerNeighborCounts`
    authority, then compose terrain-object, building, foot, and later runtime-wall lifecycle deltas;
  - remove the final-OverlayGrid `Wall=yes` scan as authored authority. A final identity may have been
    overwritten after its wall increment, and rectangular clipping loses signed 512-stride aliases.
- `src/sim/world/`, with a dedicated load-lifecycle module
  - add `LoadObjectLifecycle` on Simulation for ephemeral Overlay registry memberships, broadcasts,
    queue, destruction ordering, and slope survivors;
  - reuse only the proven duplicate-aware selection/traversal shape from ordinary pending deletion;
    never drain unrelated runtime objects at the reader boundary;
  - route authored walls through ordinary common completion after their wall effects; retain the
    generic counter-zero three-broadcast wall-reject method only for non-authored callers;
  - keep the owner load/transient (`serde(skip, default)` and outside current-object hash/save) and
    release surviving slope records at scene teardown without projecting them into gameplay owners.
- `src/sim/anim_class.rs`, `src/sim/terrain_object.rs`, and `src/sim/terrain_spawn.rs`
  - allocate/register native ID before optional RandomRate, then perform Reveal/Unlimbo,
    Logic/live registration, delay-zero Middle/StartSound, and producer marker/Z/latch writes;
  - preserve collision-free handles separately; never use numeric native ID as a collection key;
  - add an immediate scalar-delete path for terrain-marked Anims that compacts live order and releases
    current sounds without StopSound, ExpireAnim, pending delete, occupancy, or owner mutation;
  - integrate Terrain-source tiberium clear and occupation before ore-queue initialization.
- `src/sim/ore_growth.rs` and `src/sim/scenario_post_map.rs`
  - separate reset/configuration from seeding; scan the whole live real-cell surface for growth, then
    the whole surface for spread, immediately after Terrain;
  - retain those queues through later object occupation and final InitCellAttributes; remove the
    current late rebuild;
  - keep authored argument-0 signed wrapping aggregate in its proved load-state analogue with
    teardown reset, and keep generated argument-1 aggregate local-only with no invented consumer.

### Generated and preview phase ownership

- `src/map/construction_trace.rs`, `src/map/rmg/build.rs`, `src/map/rmg/pipeline.rs`, and
  `src/map/rmg/mod.rs`
  - widen the worker output into a consumed-once phase journal emitted at actual native boundaries,
    rather than deriving history from final cells;
  - preserve CABHUT constructed/failed-search distinction, all Neutral-Tech constructed outcomes,
    each Recalc, queue initialization, and terminal InitCellAttributes event;
  - retain PR #170's preconsumed Techno word and attach the separately consumed Building native ID to
    emitted bindings so final projection consumes neither twice.
- `src/app/frontend/skirmish_session.rs`, `src/app/shell_random_map.rs`, and
  `src/app/frontend/skirmish.rs`
  - add `PreviewNativeLifecycle` to the process-session owner, not the UI candidate;
  - retain active preview Buildings/Anims/latches/sounds/queues/native cursor and stable handles across
    Cancel/re-entry; implement reset-before-full/selective cleanup, token validation, and acceptance
    versus later launch separation;
  - on accepted `.SED` launch emit both ordered `FreeSpread -> FreeGrowth` pairs (generator entry,
    then Clear_Scene) and exactly one later `BuildGrowth -> BuildSpread` pair; never coalesce them;
  - ensure a clean worker receives the retained latch/live-Anim-order prestate needed to emit valid
    phase events, rather than reconstructing suppressed generations afterward.

### Ancillary routing and presentation

- expose one ordered finalization trace/seam for: real-cell raw tag-bit clear route; opaque-slot zero route;
  exactly one light-cache invalidation/recompute-routing event; per-cell unlatch; tag-line restamp
  route; value/germination; Recalc; and post-current wall-owner reconstruction;
- transaction 3 must not create generic trigger bits/consumers, semantic LightConvert cells, an opaque
  `+0x30` field, any ancillary dummy storage, or new BridgeFacts/zone/topology inputs from those slots;
- derive final occupied overlay render rows, names, atlas dependencies, minimap/radar, and bridge
  presentation from live OverlayGrid/terrain after the second boundary. Registry-wide asset preload
  can remain a superset.

### Determinism and persistence constraints

- Scenario, native-ID, runtime-handle, registry, Anim, sound, and queue event order must be deterministic
  and single-owned. No payload, cursor, journal, or Tube binding is clonable production authority.
- Preserve current stable-handle allocation and current snapshot/hash treatment of already-modeled
  persistent objects. Do not add native numeric IDs, the authored local total, raw ancillary slots,
  shared-dummy scratch, preview-native state, or load-only Overlay survivors to native-equivalence
  save/current-object checksum surfaces without separate proof; transaction 21 owns native
  persistence decisions.
- Preview state is process-shell state, never gameplay bridge/entity authority. Accepted launch starts
  a fresh Full_Init cursor and performs the proved cleanup/queue sequence; it does not transfer preview
  objects or native IDs into gameplay.
- Missing AnimType and injected allocation/registration/queue growth failures hard-error rather than
  silently omit an event or preserve native partial degradation.

## Acceptance Tests

1. `load_descriptor_source_family_format_matrix` (G1, G9)
   - setup: Loose and MIX authored maps; accepted generated Battle id 1/FFA id 2; unsupported mode,
     external/headless `.SED`, cancelled/replaced/absent staging, generated-without-journal,
     physical `LegacyFallback`; Accepted startup; resolved+validated, unresolved/manual, and failed-
     plan `UnverifiedLegacy`; every fresh family with complete and missing normalized inputs;
     missing/1/2/4 format; Generic/untyped; plus a separate persistence `ScenarioRestoreContext`;
   - action: enter production/headless fresh builders, then exercise restore through only its
     persistence owner;
   - expected: accepted ids 1/2 select generated-materialized once and never authored-Mark even when
     journal validation later fails; every other generated case rejects before effects; authored
     positive descriptors select only their proved family; missing inputs never borrow stock offline;
     only pack bodies obey signed `>1`; Accepted and resolved+validated UnverifiedLegacy produce the
     same typed stock-offline receipt/cursor, while unresolved/manual/failed validation returns an
     error with no panic or effects; physical LegacyFallback and Generic/untyped reject identically
     across builders before receipt/native ID/Scenario/Full_Init/Fill/prefix/map-reader/Mark; restore
     retains seed-zero state with the same zero fresh-path call set.
2. `fresh_family_prefixes_deliver_exact_mark_cursor` (G1, G3, P1)
   - setup: campaign explicit rows and empty-section registered fallback; merged stock offline; LAN
     Battle and Cooperative normalized slots; WOL selector 2 with zero-occupied-player and exactly-two-
     occupied-AI chooser gates; replay inheriting each family; seeds forcing ranged rejection;
   - action: execute each normalized prefix, Fill, one successful/no-op low row, and the first later
     Techno/Anim constructor;
   - expected: full logical Scenario states match after every House pass, common Gather, selected or
     common assignment callback, zero-draw reset, Fill, Mark row, and later constructor; campaign has
     one House pass/no multiplayer callbacks, LAN uses selected `+0x84`, WOL never does, replay adds
     zero calls, and no family can default to stock offline.
3. `fresh_load_prefix_tubes_and_first_overlay_absolute_ids` (G1, G11, G12)
   - setup: campaign/noncampaign/accepted-`.SED` constructor streams, custom theater Assign window,
     raw Tube rows at `T=0/2`, and a wrap boundary;
   - action: run through first allocated Overlay and one synchronous child Anim;
   - expected: formulas and ordered events match, including `1,000,018 -> 1,010,018`,
     `1,000,037,T=0 -> O1=1,010,038`, and `0xFFFFFFF0 -> 0x00002700`; stable handles remain distinct.
4. `raw_tube_constructor_fault_matrix_is_spend_before_parse` (G11)
   - setup: successful, allocated-malformed, allocation-null, and later valid raw source rows;
   - action: consume the section once;
   - expected: success binds fact/native ID, allocated-malformed spends one then hard-errors,
     allocation-null spends zero then hard-errors, nothing continues past either error, and a
     transaction-5 consumer allocates zero additional IDs.
5. `authored_overlay_rows_are_one_synchronous_yx_transaction` (G2-G4, G12, G13)
   - setup: format/body length signed gates; byte-read failure and `0xFF`; malformed registry ID;
     image-null with/without CellAnim; crate under zero/nonzero game mode; all four radar boundaries
     including rejected `x+y==W` and admitted `x+y==W+2H` when both diagonals are strict; allocation
     null; injected failure at each registry join; slope `>4` for ordinary/high/low and exempt `0xB2`;
     interleaved ordinary/high/low/wall rows, later overwrites, and CellAnim/terrain-Anim children;
   - action: execute the reader;
   - expected: exact y/x and nine-step gate order; ordinary rejections and allocation-null have zero
     constructor/handle/ID/registry/dirty/queue effect; malformed type and Rust registry growth hard-
     error at their documented points; successful construction orders Object -> pointer-expiration ->
     all-Abstract -> Tag joins -> native ID -> Overlay join -> base dirty -> derived slope/Mark;
      allocation-null's high restore is a no-op; slope survivors dirty once before zero cell/Recalc/
      queue effects; a slope-admitted authored wall bypasses the build predicate under ScenarioInit,
      performs wall effects, and takes common completion; every admitted coordinate completes before
      the next; no generated-cell dirty;
     and positive OverlayData independently applies only its radar gate.
6. `low_mark_tables_consume_exact_raw_scenario_words` (G3)
   - setup: every trigger/table arm including fixed, exact-opposite search, successful bodies of
     several `L`, no-op/failure, occupied targets, edge misses, and signed coordinates
     `x=-510,y=2` resolving to real linear slot `514`;
   - action: continue the merged post-Fill cursor through later rows and first object constructor;
   - expected: only successful bodies consume raw `3L`; x/y narrow to `i16`, the signed-i32 linear
     expression is not truncated, the negative-axis alias hits real slot 514, true misses share the
     dummy, all coordinate/state writes match, and no Tube, ranged call, clone/reseed, or component
     reordering occurs.
7. `ordinary_germination_and_high_anchor_bytes_are_exact` (G4)
   - setup: all four high ids, Land-5 tiberium/non-tiberium/range-miss cases, mixed IDs of one class,
     mixed states, all N..NW counts, repeated dummy misses, crate and later OverlayData;
   - action: Mark in source order;
   - expected: exact temporary/restored high bytes, density table, receiver-only writes, crate-last,
     persistent dummy aliases, zero germination RNG/Recalc/dirty/queue effects, and data wins later.
8. `overlay_data_drain_and_first_recalc_boundary` (G5, G7, G12)
   - setup: absent/empty identity, rejected identity with data, format 1 and 4, dead duplicate queue and
     live slope survivor;
   - action: finish reader and first sweep;
   - expected: data writes admitted real cells, drain runs once outside gate while keeping live entries,
     sweep visits exactly `H*(2W-1)` anti-diagonal cells, identity validation may clear identity/state,
     state otherwise survives, and Recalc never reads state.
9. `finalized_overlay_payload_is_linear_and_consumed_once` (G7, G9, G13)
   - setup: procedural, rejected, data-only, identity-cleared, ordinary, and authored-wall cells with
     a later low-body identity overwrite;
   - action: move payload into OverlayGrid/global count state in production and headless builders;
   - expected: one-for-one identity/state/authored-count equality with live state and later mutation;
     the overwritten wall's count survives without a final-wall scan; API rejects clone, duplicate
     consume, raw pack, rule, RNG, filter, and second-Recalc authority.
10. `authored_first_generation_anim_order_and_failures` (G6, G12)
   - setup: source-order per-Mark candidate, sweep-only candidate, already latched peer, custom
     RandomRate, stock zero-RandomRate, WA01X/non-01 waterfall, missing AnimType, and injected allocation/
     registration failure;
   - action: run Mark and first sweep through the staged Simulation sink;
   - expected: native ID/registration precedes optional Scenario draw, Main RNG is unchanged,
     Reveal/Logic/Middle/sound precede producer writes, latch suppresses duplicates, and invalid assets
     hard-fail at the exact boundary.
11. `asset_root_discovery_is_pure_and_precedes_first_anim_effect` (G6, G9)
    - setup: production and headless maps with a missing required root after valid prefix/Fill;
    - action: discover roots;
    - expected: zero handles/RNG/registrations/sounds/latches/overlay writes from discovery; failure is
      before OverlayPack/Recalc Anim effects while the trace retains already-spent prefix IDs and Fill
      RNG state.
12. `terrain_then_growth_then_spread_precedes_object_occupancy` (G6)
    - setup: Terrain clears one resource source and a later ground object occupies another
      spread-eligible resource cell;
    - action: construct Terrain, seed queues, then all object sections/Smudge;
    - expected: source clear is observed, full growth scan precedes full spread scan, later occupancy
      does not change seeded queues, and no final/post-map rebuild occurs.
13. `authored_final_init_scalar_deletes_then_recreates_live_set` (G6, P3)
    - setup: mixed Anim registry with unrelated survivors, marked live sounds, configured StopSound and
      ExpireAnim, one object-mutated former candidate, one unchanged peer, resource/non-resource cells,
      and a current wall;
    - action: execute `InitCellAttributes(0)`;
    - expected: immediate live-order compaction/current-sound detach, no StopSound/Expire/pending/owner/
      occupancy effect, exact ancillary slot trace, value-only signed formula and wrapping total,
      per-cell unlatch/Recalc, selective recreation, unchanged queues, and existing wall helper only
      after final current identity.
14. `ancillary_slots_route_without_becoming_bridge_state` (G6)
    - setup: injected route descriptors for event `0x19`, event `0x1A`, both, ordinary/sentinel light
      cases, and poison BridgeFacts/zone/dummy-field assertions; no generic trigger-bit storage is
      instantiated by transaction 3;
    - action: record the finalization seam;
    - expected: raw-clear route then opaque route then exactly one light invalidation route then
      real-cell unlatch then `0x19`-precedence restamp route then value/Recalc/wall order;
      transaction 3 stores none of the generic bits, semantic light values, opaque pointer, or
      ancillary dummy fields and none enter bridge topology.
15. `generated_phase_journal_preserves_native_interleaving` (G10, P1)
    - setup: actual staged synthetic state, failed CABHUT search, constructed CABHUT, emitted and
      discarded Neutral-Tech, early/later animated tiles, custom RandomRate, and PR #170 words;
    - action: generate and consume the journal in the staged Simulation;
    - expected: exact Recalc address-order equivalents, Building word -> native ID -> outcome,
      discard consumes/no-binds, failed search consumes neither, emitted projection consumes no second
      word/ID, Anim IDs/RNG/sounds interleave, and poisoning final cells cannot reproduce history.
16. `generated_final_init_germinates_after_queue_snapshot` (G10)
    - setup: recognized/unrecognized resources, same-class/dummy neighbors, signed overflow values, and
      a pre-final seeded queue snapshot;
    - action: terminal `InitCellAttributes(1)`;
    - expected: immediate delete, per-cell unlatch, exact density/value return, wrapping local total,
      then Recalc/recreate; queue state is unchanged and no persistent aggregate is created.
17. `preview_native_lifecycle_matches_same_changed_cancel_accept_branches` (G11)
    - setup: missing, changed, and matching normalized storage keys; repeated Generate; active Building/
      Anim/sound/queue state; Cancel/no-Generate re-entry; accepted launch;
    - action: exercise shell transitions and next gameplay load;
    - expected: reset before cleanup without RNG rewind; matching key spends zero setup IDs and gives
      the first new object `1,000,001`; changed/missing key applies
      `wrap32(1_000_000+R+|P_preview|+HB+K_preview)` with retail `K_preview=0`, so
      `R=10,P=5,HB=3,K=0` yields `1,000,018 -> 1,000,019`; no preview branch applies `+0x2710`;
      legal duplicate numeric IDs retain unique handles; old Anim/latch/sound behavior, terminal
      churn, Cancel persistence, and first-later-Generate cleanup match; accepted launch emits exactly
      `FreeSpread, FreeGrowth, FreeSpread, FreeGrowth, BuildGrowth, BuildSpread` with no collapse,
      inversion, intermediate rebuild, or extra final rebuild, then uses the fresh unrelated gameplay
      native-ID cursor.
18. `overlay_load_lifecycle_drain_and_scene_teardown` (G12, G13)
    - setup: ordinary and authored-wall common successes, slope survivor, a separate generic counter-
      zero wall-reject method control, mixed queue
      `[alive A, dead B, B, alive C, dead D]`, format/body-absent/generated seeded drain, and capacity
      fault injection;
    - action: finish reader then teardown scene;
    - expected: ordinary and authored-wall common two-broadcast completion; no authored wall-reject
      selection; separate generic control retains three-broadcast/full-Limbo; stable duplicate erase/
      recheck, exact Overlay/Limbo/type/queue/Object/listener/free order, no ID refund, hard failure on
      growth, and slope remains registered but absent from grid/entity/render/save/hash until release.
19. `authored_wall_finalization_retains_native_count_plane` (G13)
    - setup: reachable state-zero same/different-ID authored cardinal walls, overlap at `u8` wrap, a
      low body overwriting an earlier wall, `(-510,2)`-style signed fixed-stride real alias, a true
      dummy miss, absent then present OverlayData, and poison Building/gate candidates;
    - action: run the authored row transaction through payload consumption and build global counts;
    - expected: exact `N,E,S,W,self` visit and per-visit tactical/radar dirty order; each visit probes
      `N,E,S,W`, connects only the same compact ID, retains the reachable zero upper nibble, runs
      Recalc/zone work, and performs no Building/gate connection; the authored chronology never feeds
      a damaged state into cleanup, so no runtime damaged-wall branch is invented. ScenarioInit
      skips only Mark's later explicit zone pair; owner remains `-1`; eight count writes precede common
      finalization and wrap; overwritten-wall and real-alias contributions survive with no dummy output;
      absent data retains connectivity, present data replaces state only; no final-wall reconstruction.
20. `final_live_overlay_drives_all_presentation_surfaces` (G8)
    - setup: low-created identity absent from raw pack, data override, rejected raw row, and Recalc-cleared
      row;
    - action: build atlas/name/render/minimap/radar/bridge inputs after final Init;
    - expected: the procedural survivor appears with final state and dependencies; rejected/cleared rows
      do not; asset registry preload may remain a harmless superset.
21. Existing PR #170, PR #196, raw bridge-fact/TIBTRE, high-anchor, wall-owner, generated no-Mark,
    snapshot/hash, and bridge parity-harness families remain green. The full suite is run once only at
    the final PR gate as `cargo test -p vera20k --lib`; focused work also uses `--lib` exclusively.

## Continuation slice B: post-`Full_Init` setup tail (2026-09-01)

Scope: the `FUN_00684C30` tail opened as OQ-37 by PR #207's critic and the G6 value-only aggregate.
Native evidence (all decompiled live): `FUN_00684C30 @ 0x00684C30` (order at
`0x00684FAB..0x006850F3`), `Clear_Scene @ 0x006851F0` (`DAT_00A8ED78` nulled at `0x0068562E`),
`ParticleSystemClass @ 0x0062DC50`, `ParticleSystemTypeClass::Find_Or_Allocate @ 0x00644630`
("GasCloudSys" @ `0x0083DA90`, present in retail `[ParticleSystems]`), `CellClass::Get_Tiberium_Value
@ 0x00485020`, `Random::RandomRanged @ 0x0065C7E0`, `AnimClass @ 0x00421EA0` (ID and registry
before `RandomRate`, no DetailLevel gate, `Middle` at delay 0), `CellClass` vtable `+0x48 @
0x00486840`, `MapClass::CellIterator_Next @ 0x00578290`, `RulesClass::ReadGeneral @ 0x0066D661..
0x0066D699` (`[General] OreTwinkle`), `RulesClass::ReadAudioVisual @ 0x0066B7F8..0x0066B812`
(`[AudioVisual] OreTwinkleChance`, default 0x32), `MapClass::InitCellAttributes @ 0x00568BB0` and
the `Full_Init` store at `0x0087F91C`.

Required Rust (delivered on `feature/bridge-post-load-tail`):
- `GeneralRules::ore_twinkle: Option<String>` and `ore_twinkle_chance: i32` with the verified
  reader sections and defaults;
- `Simulation::run_post_load_ore_twinkle_pass` at the end of `finalize_scenario_post_map`: the
  GasCloudSys `ParticleSystemClass` native ID when not yet constructed since `Clear_Scene`
  (authored loads spend it here; a generated launch spends it earlier through
  `construct_post_load_particle_system_id` in the generated arm before the construction-trace
  replay, mirroring the synthetic `Full_Init` setup at `0x00599A5B`), then per real cell in
  `CellIterator` order one signed `RandomRanged(0, chance-1)` Scenario draw when
  `Get_Tiberium_Value` is nonzero, and on a zero roll one `AnimClass` with a fresh native ID at the
  cell centre and ground height, `(delay 0, loop 1, flags 0x600, ZAdjust 0)`, registered before any
  `RandomRate` draw;
- `AnimClass::AI @ 0x00423AC0` `HideIfNoOre` consumer in `Simulation::visit_anim`: before the
  MakeInfantry `vtable+0xF0` call, the bounce-landing block, and the trailer block,
  `AnimClass+0x19D` (`AnimDrawRuntime::hidden`) is rewritten every tick from the cell's
  `Get_Tiberium_Value` (dummy overlay pair for a coordinate outside the grid rectangle);
- `Simulation::authored_tiberium_value_total` written by the authored final sweep from the exact
  `Get_Tiberium_Value` model with wrapping signed-32 accumulation.

Acceptance: `rules::ruleset::tests::ore_twinkle_keys_follow_the_native_reader_sections`,
`sim::rng::tests::signed_random_ranged_matches_unsigned_form_and_swaps_negative_bounds`,
`sim::scenario_post_map::tests::generated_launch_particle_id_precedes_constructors_and_post_map_skips_it`,
`sim::scenario_post_map::tests::ore_twinkle_hides_while_its_cell_has_no_ore_and_reappears_with_it`,
`sim::ore_twinkle::tests::get_tiberium_value_is_zero_for_non_resources_and_wraps_signed_products`,
`sim::scenario_post_map::tests::post_load_ore_twinkle_pass_rolls_each_resource_cell_in_native_order`
(native order, Scenario cursor equality against an independent replay, unchanged Main RNG, native-ID
order `particle -> twinkle...`, draw flags 0x600, cells outside the diamond never rolled),
`..._is_inert_without_the_rules_anim`, and the ignored retail Dustbowl headless load asserting the
stored aggregate.

Recorded residuals from this corridor: the ParticleSystem object itself, the `[Basic] FillSilos`
credits-to-tiberium loop, the per-Building vtable `+0x4E0` call, the TagType attach pass, the
campaign-only view setup, `FUN_004F42F0`/sidebar presentation, `MapClass::ParanoidUnrevealAll`
(shroud owner), the constructor `ZAdjust`/`AnimType+0x348` substitution on the load-anim path,
map-INI empty-value `OreTwinkle=` shadowing, and `FUN_00586BF0` bridge-record gap restamping (routed
to transaction 4/13).

## Continuation slice C: generated final `InitCellAttributes(1)` germination and queue order (2026-09-02)

Scope: the generator-tail part of G10 that changes retail gameplay on every random map — the final
`InitCellAttributes(1)` ore-density rewrite, its value-only caller-local sum, and the growth-then-spread
queue initialization that precedes it. The phase journal (anim chronology across the generator Recalc
boundaries, synthetic `Full_Init` state) stays open as the rest of G10.

Native evidence (decompiled live 2026-09-01):
- `RandomMapGenerator::Generate @ 0x00598960` tail `0x00599370..0x0059945B`: final whole-map
  `RecalcAttributes(-1)` loop (`0x0059937D`) -> optional callback -> `TiberiumClass::InitGrowthQueues_All
  @ 0x00722D00` -> `TiberiumClass::InitSpreadQueues_All @ 0x00722240` -> scratch free ->
  `MapClass::InitCellAttributes(1)` (`push 1` at `0x0059943F`, call at `0x0059944C`); the return is
  caller-local.
- `MapClass::InitCellAttributes @ 0x00568BB0 (arg)`: scalar-delete terrain-attached Anims; clear
  `Flags & 0xFFCFFFFF`; per real cell in `CellIterator` order: `+0x30 = 0`, `FUN_00483E30` light
  routing, clear `0x20000`, AttachedTag `0x19`/`0x1A` restamp, `arg == 0 ? Get_Tiberium_Value() :
  SpreadCellGerminate(0)`, `local += return`, `RecalcAttributes(-1)`, wall-owner reconstruction for a
  current `Wall=yes` overlay; returns the sum.
- `CellClass::SpreadCellGerminate @ 0x004818E0 (randomize = 0)`: requires `Cell+0x44 != -1` and
  `OverlayToTiberiumIndex @ 0x005FDD20 != -1`; captures `TiberiumClass+0xB8 (Value)`; resolves the
  eight `g_DirectionOffsets @ 0x0089F688` neighbours (`EDI & 7` at `0x00481968`; runtime-initialized
  table, order N, NE, E, SE, S, SW, W, NW) through the stamping `MapClass::Get_CellClass @ 0x005657A0`
  (`0x004819A6`; a miss stamps the shared dummy and the read continues on it); counts those whose
  `OverlayToTiberiumIndex` equals the receiver's; `OverlayData = g_OreDensityByNeighborCount @
  0x0081CD28[count % TiberiumClass+0xE4]` (`IDIV` at `0x004819CA`; table `[0,1,3,4,6,7,8,10,11,7,0,1]`);
  returns `(OverlayData + 1) * Value`. No RNG for argument 0.
- `CellClass::RecalcAttributes @ 0x0047D2B0` reads `OverlayTypeIndex` and writes `bOverlayData = 0`
  on its steep-slope/NoUseTileLandType branches but never reads `+0x11E`; after the generator's
  whole-map Recalc the identities are final, so the per-cell Recalc after germination changes no
  attribute.
- `TiberiumClass::InitGrowthQueues_All @ 0x00722D00` / `InitSpreadQueues_All @ 0x00722240`: free and
  rebuild every TiberiumClass's queue storage from the current cell state, growth for all types first,
  then spread for all types.
- Third `InitCellAttributes` caller: `MapClass::Resize @ 0x00567092/0x005670E2` -> `MapClass::InitZoneMap
  @ 0x00567110` -> `InitCellAttributes(0)` on the freshly constructed (pre-Fill) cells: no Anim, ID,
  RNG, wall, or stored-total effect.

Player effect and frequency: the generator paints densities with `density_draw` and increments; native
then rewrites every ore cell's density from its same-class neighbour count, so field interiors become 11
and edges thin. Before this slice Rust kept the painted densities -> different ore value and harvester
income on every random map.

Landed Rust:
- `src/sim/tiberium_germinate.rs`: shared `spread_cell_germinate_without_randomization` (caller-owned
  neighbour lookup and receiver write; returns the new density and native value) and
  `run_generated_final_cell_attributes` (real cells in `CellIterator` order through
  `native_fixed_cell_index`; misses stamp the terrain's shared dummy and read its overlay fields;
  direct density write, no runtime dirtiness; caller-local wrapping total in the receipt).
- `src/sim/runtime.rs::initialize_native_tiberium_queues` (generalized from the authored-only
  initializer; takes the grid as a read-only view and the map `basic`/`special_flags` sections).
- `src/app/loading/init.rs` generated arm: queue initialization then the germination pass after
  `populate_staged_app_scenario`; `finalize_constructed_scenario` receives
  `tiberium_queues_preinitialized = true` for both arms; `refresh_generated_tiberium_presentation_frames`
  rewrites only resource-identity presentation frames from the germinated grid.
- `src/sim/crates.rs`: the crate Mark seam routes through the shared helper; its neighbour visit order
  moves from NW-first to the native N-first order (affects only the dummy's retained coordinate).

Acceptance tests: `sim::tiberium_germinate::tests::{generated_pass_rewrites_every_resource_density_from_same_class_neighbours,
generated_pass_counts_the_shared_dummy_for_missing_neighbours_in_native_order,
generated_queues_are_seeded_before_germination_and_left_alone}`,
`app::loading::init::startup_crate_presentation_tests::generated_presentation_frames_follow_germinated_ore_densities_only`,
and the retained `sim::crates::tests::road_tiberium_crate_mark_germinates_from_same_type_neighbors`.

Residuals: the per-cell Recalc / terrain-Anim scalar-delete-recreate chronology and native-ID order of
the generated pass (G10 phase journal); the ancillary slots (`+0x30`, light routing, tag restamp) shared
with the ancillary seam; OQ-38, native queue rebuild parity (`TiberiumClass::RebuildGrowthQueue @
0x007233A0`, `RebuildSpreadQueue @ 0x007228B0`, `CellClass::CanGrowTiberium @ 0x00483620`,
`CellClass::CanSpreadTiberium @ 0x00483690`, decompiled live 2026-09-02): the Rust admission predicates
(`OverlayData > TiberiumClass index / 2` for spread — native, not a mis-port — flat slope, growth
`OverlayData < MaxDensity - 1`) are verified; DRIFT recorded for the next slice: native inserts in
`CellIterator` order into a binary heap and pops its root, Rust inserts row-major and pops a stably sorted
front (equal-priority order differs on every map); native requires the percentage doubles `>= 1e-05`
where Rust accepts `ppm >= 0`; native spread requires `CellClass+0xE4 FirstObject == 0` where Rust
excludes only terrain-object cells; the Scenario flag gates are read from the map/rules booleans.

## Known Non-Requirements

- Do not construct Tube topology or use `explicit_tubes` as native-ID accounting input.
- Do not let low Road overlays construct TubeClass, import OpenTS tables, or add TS rail behavior.
- Do not batch high rows before low rows, replay low bodies after decode, use ranged RNG, or replace
  the persistent shared dummy with per-miss scratch values.
- Do not derive generated provenance from a construction-trace `Option`, `.SED` suffix alone, or the
  accepted preview payload.
- Do not admit an external/headless/arbitrary `.SED` or unsupported game mode as generated gameplay;
  only consumed accepted stock-offline Battle id `1`/FFA id `2` staging provides current positive
  provenance, and a missing phase journal never falls back to authored Mark.
- Do not default campaign, LAN, WOL, or replay to stock offline. Missing normalized family inputs are
  an explicit unsupported-load error, not permission to reconstruct a cursor from a seed.
- Do not infer that format `<=1` skips the shared drain, first Recalc, final Init, native-ID prefix, or
  Tube constructor boundary.
- Do not add stream restore to the fresh descriptor or loading pipeline, and do not convert
  `ScenarioRestoreContext` into any fresh context. Restore remains persistence-owned and seed-zero.
- Do not reinterpret `LoadedMapSource::LegacyFallback` as authored or generated parity input; it is
  a universal pre-effect rejection for gameplay-equivalent builders.
- Do not reject `LoadingStartup::UnverifiedLegacy` merely by variant name or confuse it with physical
  LegacyFallback. Only resolved+validated compatibility startup becomes typed stock offline;
  unresolved/manual/failed validation errors before effects and never uses `expect`.
- Do not precompute final Anim descriptors as execution authority, delay child IDs until after the
  row, or reconstruct phase history from final cells.
- Do not use `native_unique_id` as a stable/runtime collection key or forbid native duplicate IDs.
- Do not apply the gameplay map-read `+0x2710` reservation or fresh Full_Init formula to preview;
  preview uses the explicit matching-zero or changed/missing `R+P_preview+HB+K_preview` setup branch.
- Do not send terrain-marked Anims through ordinary Destroy/UnInit/pending deletion, play StopSound,
  spawn ExpireAnim, or invent cell/entity occupation.
- Do not rebuild ore queues after object sections or after final Init; do not retain a second grid in
  the queue owner or persist the generated local aggregate.
- Do not promote steep-slope Overlay survivors into OverlayGrid, `GameEntity`, occupancy, Logic,
  Display, presentation, native save, or current-object checksum.
- Do not treat generic counter-zero wall rejection as authored Full_Init behavior. Preserve the method
  separately, but ScenarioInit makes it unreachable for a slope-admitted authored wall.
- Do not reconstruct authored blocker-neighbor counts from final `Wall=yes` identities or store a
  shared-dummy counter in fresh output. Ordered wall increments can survive later identity overwrite;
  only real fixed-map slots, including signed aliases, cross the finalized payload.
- Do not implement generic trigger-line semantics, semantic light output, or `Cell+0x30` storage in
  this transaction. Their ordering routes and negative no-bridge ownership are required.
- Do not replace the existing wall-owner reconstruction algorithm or run it before final current
  Recalc identity is known.
- Do not close GSI-04.12/BR-M04, GSI-04.15/BR-M11, or positive Tube ownership from this transaction
  alone; their later contributors and fresh full-row critics remain mandatory.

## Blockers And Follow-Ups

- No `BLOCKED` or `UNKNOWN` row remains inside the selected transaction. All material active-retail
  behavior needed to implement G1-G13 is proven.
- Synthetic-Full_Init tile eligibility is content-dependent, not an evidence blocker: implementation
  must transport the actual staged state and acceptance must not assume an empty generation.
- After a fresh read-only contract critic passes, the authorized next action is implementation on the
  current `feature/bridge-authored-overlay-finalization` branch with mechanism-coherent commits and
  focused `--lib` tests. A task-by-task `/write-plan` is optional and was not separately requested.
- Any new contradiction reopens the living inventory and the affected row. Use `/re-investigate` for
  one narrow native question; do not approximate through it.
- After transaction 3 merges, transaction 4 receives this shared high-load evidence/diff/output;
  transaction 5 receives successful `TubeNativeInit` bindings; transactions 20/21 retain the routed
  light and persistence questions. GSI-04.12, GSI-04.13, and GSI-04.15 remain open until all owners,
  critics, merges, and the bridge-wide reverse audit pass.

## Source Ledger

- `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_MARK_LOAD_CONTEXT_SOURCE_PROVENANCE_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_OVERLAYPACK_INLINE_TRANSACTION_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_OVERLAY_EPHEMERAL_OBJECT_FINALIZATION_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_OVERLAY_WALL_SCENARIOINIT_ACCEPTANCE_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/CELL_0X122_DYNAMIC_BLOCKER_LIFECYCLE_RUST_MAPPING_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_TIBERIUM_GERMINATE_SIDE_EFFECT_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/OVERLAYPACK_SHARED_DUMMY_FINAL_RECALC_FIELDS_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/TERRAIN_ATTACHED_ANIM_LOAD_LIFECYCLE_SIDE_EFFECTS_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/FULL_INIT_AND_PREVIEW_NATIVE_ID_PREFIX_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/INITCELLATTRIBUTES_TAG_LINE_LIGHTING_TAIL_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/00-system-models/RMG_PREVIEW_ANIM_BUILDING_IDENTITY_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/LOW_OVERLAY_MARK_FIXED_MAP_STAMP_RNG_TRANSACTION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/LOW_OVERLAY_MARK_SCENARIO_LOAD_ACTIVATION_BOUNDARY_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/LOW_OVERLAY_MARK_ALL_LOAD_CONTEXT_SCENARIO_RNG_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_STRUCT_GHIDRA_REPORT.md`
- `docs/research/miner/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`
- `docs/research/LOADING_FULL_INIT_PROGRESS_SEQUENCE_AFTER_00552D60_GHIDRA_REPORT.md`
- `docs/research/MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md`
- `docs/research/skirmish-ui/RMG_TERRAIN_SHAPING_CORE_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_RANDOM_MAP_GENERATOR_00598960_GHIDRA_REPORT.md`
- `docs/gap-scans/2026-08-31-disparity-scan-authored-overlay-finalization.md`
- `docs/plans/2026-08-28-active-retail-bridge-parity-design.md` Revision 19, approved
- current Rust baseline `origin/main@50e4b7ba4732fd3fb48e5b819e1abc55327ec557`

## Ghidra Annotation Candidates

- None for this contract pass. The focused reports retain their candidate ledgers; no synchronization
  was requested, and the contract introduces no new live-binary conclusion.
