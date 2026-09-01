# Phase 14 crate-authority foundation design

**Status:** approved for implementation after design-review  
**Phase hypothesis:** GSI-14 row 303, “Crates / powerups,” contains several independently deliverable mechanisms. This design closes only the authoritative creation and delivery of scenario-start crates. Pickup, regeneration, effects, and specific-cell producers remain separate Phase 14 mechanisms.  
**Caller/order evidence:** `docs/research/SCENARIO_START_CRATE_POST_MAP_CALLER_GATE_GHIDRA_REPORT.md`  
**Placement/runtime evidence:** `docs/research/PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md`  
**Supersedes as design authority:** `docs/plans/2026-07-23-crate-authority-design.md` and `docs/plans/2026-07-23-crate-evidence-foundation-plan.md`

## Outcome

For an admitted stock offline Skirmish session, when the finalized global
`Crates` option is enabled, VERA20k will create scenario-start crates through
one persistent simulation authority that matches active retail in rule inputs,
signed attempt count, RNG consumption, slot state, placement acceptance,
visible-versus-ghost results, overlay mutation, snapshot/hash ownership, and
first-frame presentation.

Fixed-map campaign mode 0 will not run this scenario-start scatter. That
negative rule is owned by the active caller gate in
`ScenarioClass__Full_Init @ 0x00686B20`, not by an invented helper-local
comparison. Successful playable random-map generation reaches the same
post-map mechanism through its separate direct caller; preview generation
does not.

This replaces the current approximation, which uses a temporary local
`[bool; 256]`, drops native timer state, draws from the wrong rectangle,
bypasses native Mark predicates, hides newly placed crates from the initial
presentation list, and performs AI opening credits before crates.

## Player-experience ledger

| Situation | Required player-visible or deterministic result |
|---|---|
| Stock offline Skirmish, Crates on | The human-seat-derived attempt count is scattered before AI opening credits and alliances; visible crates render on the first gameplay frame. |
| Fixed-map campaign mode 0 | No scenario-start random-crate scatter is created by this helper, regardless of the stock minimum. Authored/specific-cell crate producers are unaffected and remain later mechanisms. |
| Successful playable random-map generation | The direct post-map caller runs the same startup authority once generation succeeds. |
| Random-map preview | No gameplay crate slots, overlays, or RNG side effects are created. |
| Crates off | No slots, overlays, timer draws, or coordinate draws are created. |
| Water/common identity | Destination land type selects the configured water or wood/common overlay identity; Mark then selects Float only when the selected identity equals the current water identity, otherwise Track. |
| Aliased configured identities | Water identity comparison has priority. If water and wood/common resolve to the same identity, the selected identity uses Float facts. |
| Configured `none` image | Null identity survives the hard prechecks, then becomes an accepted timed ghost; it does not retry or fail closed. |
| Terrain object, non-exempt steep slope, occupation, speed-zero, bridge-plane, allocation, Unlimbo, or Mark failure after hard prechecks | The call is accepted as a ghost: the slot and timer persist, RNG is consumed, and no visible overlay is stamped. Raw overlay ID `0xB2` is the exact retail exception that remains Mark-eligible above slope four. |
| Occupied overlay or snapped cell outside the playfield | The attempt is rejected before construction and retries, preserving the exact draw sequence. |
| All 256 slots occupied | The random call returns without consuming RNG. |
| Save/restore or lockstep hash | Every raw slot word round-trips and affects VERA's versioned state hash; retail's narrow multiplayer checksum remains unchanged. |
| First presentation build | Visible startup crate cells enter the existing `OverlayRenderIndex`; ghosts do not, and frame zero is used for `Crate=yes`. |

## Scope boundary

### In this mechanism

- The layered `[CrateRules]` startup-crate subset required by this path:
  `CrateMinimum`, `CrateMaximum`, `CrateRegen`, `WoodCrateImg`,
  `CrateImg`, and `WaterCrateImg`, including constructor state, section/key
  retention, signed integers, exact binary64 regen bits, and overlay identity
  allocation.
- One persistent 256-entry crate slot table owned by `Simulation`.
- Exact scenario-start signed count and random-placement transaction.
- Native accepted-ghost behavior for every failure after the outside-playfield
  and occupied-overlay hard prechecks that is representable in Rust.
- Exact timer draw/interpolation, pre-increment frame ownership, and `aux` word.
- Crate-specific overlay writes that preserve unrelated cell fields.
- Snapshot version 114 and a version-gated world-hash fold for the raw slot
  table. No change to the retail quick checksum.
- Production post-map routing for fixed-map mode 5, the successful playable
  random-map caller, and the negative campaign/preview gates.
- Correct post-map order: crates, then AI opening credits, then alliances.
- Initial render-index synchronization and runtime overlay-name
  preregistration.
- Active executable, installed INI, and installed CRATE/WCRATE theater-asset
  validation.

### Explicitly not in this mechanism

- Unrelated `[CrateRules]`/crate-adjacent authority:
  `FreeMCV`, `SoloCrateMoney`, `CrateRadius`, `UnitCrateType`,
  `CrateGoodie`, fixed Silver/Wood/Water goodie mappings, heal sound, and
  sound-registry lookup. None is consumed by scenario-start scatter.
- `[Powerups]` parsing, weighted selection, anti-stack remaps, pickup return
  barriers, event 49/50 emission, the eight positive-weight stock effects, and
  their sound/animation consumers.
- Runtime regeneration scans and crate removal/clear behavior. `CrateRegen`
  is parsed now only because every accepted startup slot immediately needs the
  native duration/aux state.
- Specific-cell placement used by trigger action 108 and `CrateBeneath`.
- `CarriesCrate`, `CrateBeneath`, `IsMoney`, `CrateTrigger`,
  `TruckCrate`, and `TrainCrate` producers/consumers.
- Raw modes 3/4 networking. The native fixed-map comparison admits nonzero
  modes, but current ordinary production does not admit a network simulation
  session.
- The failed native Overlay allocation's orphan object-array identity and
  UniqueID. Gameplay-observable slot/timer/RNG and cell results are retained.
- Invalid-domain OOM, corrupt pointer graphs, and malformed nonfinite rule
  inputs.

The exclusions remain open Phase 14 work. This PR may establish shared types
that later mechanisms reuse, but it must not claim those later consumers as
closed.

## Evidence contract

| Native behavior | Decisive evidence |
|---|---|
| Fixed-map caller and mode/control-byte gates | `ScenarioClass__Full_Init @ 0x00686B20`, assembly `0x00687BC8..0x00687BEC`; caller/order correction report |
| Ordinary fresh-loader control byte | `ScenarioClass__Read_Scenario_INI @ 0x00686730`, `XOR DL,DL` at `0x0068683A`, call at `0x00686845` |
| Generated-map direct caller | `ScenarioClass__Read_Scenario @ 0x00684620`, `0x0068498E..0x00684990` |
| Rules finalization and option/count/order | `Full_Init @ 0x00686B20`; `Post_Map_Init @ 0x00686890`, especially `0x0068695C..0x00686AF2` |
| Rules constructor and layered reader for the six startup fields | `RulesClass__Constructor @ 0x00665650`; `RulesClass__ReadCrateRules @ 0x0066B900` |
| Random placement, slots, and Mark facts | `MapClass__PlaceRandomCrate`, `MapClass__PlaceCrate`, and `MapClass__CrateSlot` bodies/callers in the placement/runtime report |
| Timer formula | accepted placement body: `regen*450`, `regen*1800`, `RandomRanged(0,0x7ffffffe)`, forward interpolation, x87 truncate |
| Slot lifetime shape | 256 entries × 16 bytes: signed start, aux word, signed duration, packed signed 16-bit x/y; `(0,0)` is the sole empty test |
| Retail installed values | active `rulesmd.ini`: min 1, max 255, regen 3, wood/common CRATE, water WCRATE; `[MultiplayerDialogSettings] Crates=yes` |
| Retail art | installed `ra2.mix -> temperat.mix`: `CRATE.TEM` and `WCRATE.TEM`, SHP(TS), 60×60, two frames |

All simulation-behavior comments and tests introduced by implementation must
cite the relevant body or one of the two primary reports. The caller/order
correction report overrides the older statement that the active startup path
has no game-mode gate. The placement/runtime report remains authoritative for
the placement transaction where it is not contradicted.

## Existing behavior to preserve

- The scenario RNG is already the correct stream, and
  `SimRng::next_range_u32_inclusive` models the active mask-and-reject
  `RandomRanged` shape, including no draw for equal bounds.
- `find_nearby_passable_cell` already owns the shared nearby-cell search and
  must remain the sole FNPC implementation. Crate code supplies native query
  facts; it does not fork the search algorithm.
- `RawCellOccupationGrid` already exposes exact ground/deck bytes.
- `BridgeCellFacts` already preserves native raw bridge flags, including the
  structural `0x100` deck selector.
- `OverlayGrid` is the simulation authority for live overlay identity/data and
  publishes dirty cells to the existing frame-update path.
- `OverlayRenderIndex` owns source-order install, coordinate upsert,
  tombstones, and restore ordering.
- Rendering forces frame zero for `Crate=yes`, and the atlas can load crate
  body art.
- The retail multiplayer quick-checksum deliberately excludes native crate
  slots and remains unchanged.
- `ScenarioPostMapOutput.crates: Option<_>` correctly carries the active
  campaign-versus-Skirmish distinction and remains optional.

## Design

### 1. Layered startup-crate rules authority

Move the startup subset of `CrateRules` from the monolithic `ruleset.rs`
parser into focused `rules/crate_rules.rs` state:

- `wood_crate_img`, `crate_img`, and `water_crate_img`: optional allocated
  overlay identities;
- `minimum` and `maximum`: signed `i32`;
- `regen: NativeF64Bits`.

Constructor values are literal native values: three null image pointers,
minimum 1, maximum 255, and regen bits for 10.0.

`RulesPassProcessor` gains a narrow accumulator. Each pass first performs the
existing explicit registry allocations and type readers. After
`allocate_late_global_references`, the accumulator applies
`ReadCrateRules`-equivalent updates while the Overlay registry contains the
identities the native reader would see. Section absence returns without writes;
each missing key retains the current accumulated value. A valid `none` image
stores a null identity rather than being treated as a lookup error.

The accumulator is returned as semantic state in `ProcessedRulesLayers`, next
to the compatibility projection and content hash. `RuleSet::from_processed_rules`
builds ordinary projected fields through a private parser and installs the
exact accumulated startup-crate state. `RuleSet::from_ini` wraps its one INI in
a `RulesLayerStack`, so direct tests and production share one path.

`CrateImg` is retained even though destination land type chooses the
water-versus-wood/common configured identity. Native Mark classification tests
the selected identity against the current configured image pointers, and
configured aliasing/null values are therefore observable.

No sound, unit, radius, money, FreeMCV, fixed-goodie, or unrelated crate field
is parsed by this mechanism.

### 2. Persistent crate slots

Add focused `sim/crates/state.rs` authority:

```text
CrateSlot {
    start_frame: i32,
    aux: u32,
    duration: i32,
    cell_x: i16,
    cell_y: i16,
}

CrateAuthority {
    slots: [CrateSlot; 256],
}
```

Fresh/reset slots are exactly `{-1, 0, 0, 0, 0}`. A slot is empty only when
both coordinates are zero; no derived lookup from `OverlayGrid` replaces this
test. The first empty slot is always found by ascending index.
`CrateAuthority` is a direct `Simulation` field adjacent to overlay/map
authorities, not a presentation cache or temporary local.

This slice writes slot state during startup placement but does not advance or
clear timers. The raw representation is nevertheless complete now so later
regeneration and pickup do not need a migration or second authority.

### 3. Count and random-cell frame

`scenario_start_crate_count` accepts signed `i32` values and performs:

```text
min(maximum, max(minimum, human_node_count))
```

No clamp to zero or 256 is added. A negative requested result executes zero
placement-loop iterations. Values over 256 continue invoking the random
placement entry, whose full-slot early return consumes no RNG. The outer
post-map loop consumes each requested call regardless of visible, ghost, or
failure result; it does not top up successful overlays.

The human count remains human, non-passive houses only. The active random
rectangle has left/top 1 and width/height `SizeW + SizeH - 1`, derived with
native signed/wrapping arithmetic from `Simulation.playfield_bounds.base` and
`playfield_size_height`. Each coordinate is exactly:

```text
left + RandomRanged(0, width - 1)
top  + RandomRanged(0, height - 1)
```

It never substitutes the canonical iso-array `session.map_width`.

Each random-placement call consumes X then Y from `scenario_rng`, up to 1000
search attempts. The drawn cell chooses Float versus Track for the existing
FNPC query. The snapped destination selects the configured
`WaterCrateImg` versus `WoodCrateImg` identity from land type.

### 4. Placement transaction and ghost semantics

The transaction order is explicit:

1. Find the first empty slot. If none exists, return without RNG.
2. Draw X then Y and run FNPC.
3. Reject and retry if the snapped cell is outside the playfield or already has
   an overlay.
4. Select the water or wood/common configured identity.
5. Classify Mark movement by identity, not destination surface:
   compare the selected identity with current `WaterCrateImg` first and use
   Float on equality; otherwise, matching `CrateImg` or `WoodCrateImg` uses
   Track. Water-first priority preserves configured pointer aliases.
6. Run constructor/Unlimbo/Mark facts. A null/unknown selected identity,
   terrain object, slope above four when the selected raw overlay ID is not
   exact `0xB2`, nonzero selected occupation byte,
   non-bridge selected-speed zero, bridge-plane failure, allocation failure,
   Unlimbo failure, or Mark failure is an accepted ghost.
7. Structural bridge `raw_flags & 0x100` selects deck occupation and bypasses
   the non-bridge terrain-speed-zero rejection.
8. Visible success stamps overlay identity and data `0xFF`; a ghost leaves the
   cell unchanged. Both stop retries.
9. Record the slot coordinate, draw/install the timer, and report the outcome.

Only the outside-playfield and occupied-overlay checks in step 3 are retryable
hard rejections after a snapped candidate exists. Every later failure consumes
the call as an accepted timed ghost. This includes valid configured `none`.

Rust has no fallible native Overlay allocator or separate Unlimbo object graph.
Production registry/grid invariants make allocation succeed; deterministic test
injection may exercise the ghost result, but must not turn it into retry or
fail-closed behavior.

`CratePlacement` becomes signed/count-aware:

```text
requested: i32
accepted: u32
visible: u32
```

`accepted` includes ghosts.

### 5. Exact timer math

Timer calculation uses `NativeF64Bits` and `X87Chop53`, not host floating
point in simulation:

```text
lower = regen * 450.0
upper = regen * 1800.0
draw = RandomRanged(0, 0x7ffffffe)
value = lower + draw / 2147483646.0 * (upper - lower)
duration = x87 truncate-toward-zero(value)
start = current pre-increment binary frame
aux = high dword of stored upper double
```

Retail `regen=3` must produce 1350 at draw zero, 5400 at the inclusive maximum,
and aux `0x40B51800`. A narrow pure helper exposes golden vectors independent
of placement search.

### 6. Crate-specific overlay writes

Do not route crate placement through generic
`place_overlay_native_runtime`, whose wall/protection gates differ. Add the
smallest crate-specific `OverlayGrid` primitive:

- install the chosen identity and data `0xFF`;
- preserve wall-owner and unrelated cell authority;
- run existing synchronous passability invalidation/publication;
- enqueue the existing dirty-cell fact once for a visible result.

Ghost placement leaves the cell untouched. Native zero-rectangle dirty/redraw
attempts map to a Rust-native no-pixel no-op; no persistent render queue is
invented merely to record an empty rectangle. Radar invalidation is not emitted.

Removal and raw low-byte postwrite primitives remain deferred with their
consumers.

### 7. Post-map production delivery and order

`Simulation::finalize_scenario_post_map` preserves the earlier verified
tiberium and navigation seams, then handles session modes as follows.

For `skirmish_session: Some`:

1. evaluate `session.game_options.crates`;
2. count current human non-passive houses and make the exact startup attempts;
3. apply Skirmish AI opening credits;
4. apply launch alliances;
5. return `ScenarioPostMapOutput.crates = Some(receipt)`.

For fixed-map campaign `skirmish_session: None`, do not call startup crate
placement; apply campaign-authored alliances and return `crates = None`.

The existing accepted-random-map launch attaches generated map data to the
ordinary Skirmish loading request through `with_accepted_random_map`; it must
therefore reach this same `skirmish_session: Some` post-map seam after successful
acceptance. Preview generation stops before that request and must not create
simulation state. This mechanism adds regression coverage to that production
connection rather than a second simulation API. No raw network mode is
constructed here.

The option-off Skirmish receipt remains `Some` with requested/accepted/visible
zero so mode ownership stays distinct from option state.

### 8. First-frame presentation delivery

The app currently builds `overlays_connected` before post-map finalization and
does not react to startup crates. After finalization it will query only visible
accepted crate coordinates from simulation authority, materialize occupied
`OverlayEntry` values from the live grid, and upsert them through
`OverlayRenderIndex` before `AppState` construction. Ghosts have no overlay
identity and do not appear.

`preregister_runtime_overlay_names` will include every registry entry with
`Crate=yes`, alongside existing wall and low-bridge registration. Existing
atlas resolution and `CRATE_BODY_FRAME = 0` remain unchanged.

```text
active rules layers
  -> startup CrateRules subset
  -> gated and ordered post-map production command
  -> Simulation crate slot + OverlayGrid
  -> initial OverlayRenderIndex
  -> existing crate atlas/frame-zero draw path
```

### 9. Snapshot and hash ownership

Bump `SNAPSHOT_VERSION` from 113 to 114. `CrateAuthority` serializes its raw
slot array rather than reconstructing from overlays, preserving ghosts, signed
durations, and aux words.

Add `include_crate_authority_v114` to the versioned world-hash schema. Current
hashes fold all five slot fields in ascending slot order. Historical probe
helpers disable the new field, and a
`state_hash_without_crate_authority_v114` probe proves the version boundary.
The pre-v114 session identity fold stays byte-for-byte stable.

`compute_retail_multiplayer_checksum` is unchanged because retail's quick
checksum excludes this array; VERA's broader state hash includes it for
deterministic snapshot/replay protection.

## Architecture fit

- `rules/` owns layered INI semantics and overlay identities; it does not
  depend on simulation, rendering, or audio.
- `sim/` owns crate slots, RNG, placement, timers, overlay state, and
  deterministic persistence.
- `app/loading` supplies map/session inputs and consumes post-map receipts; it
  does not decide crate behavior.
- `render/` consumes overlay identities/data and registry flags.
- No simulation module depends on app, render, UI, sidebar, audio, or network.
- Persistent native floating rule state is stored as bits and evaluated through
  the existing native x87 helper.

## Expected file-level change set

- Add `src/rules/crate_rules.rs` and export it from `src/rules/mod.rs`.
- Update `src/rules/ini_parser.rs`, `src/rules/ruleset.rs`, and focused rule
  tests for only the six startup fields.
- Split `src/sim/crates.rs` into a narrow state/placement module layout, or use
  an equivalently small layout if live-tree review shows a less disruptive fit.
- Update the simulation owner, `src/sim/overlay_grid.rs`,
  `src/sim/scenario_post_map.rs`, snapshot code, and world-hash schema/tests.
- Update `src/app/loading/init.rs`, runtime overlay-name preregistration, and
  `OverlayRenderIndex` tests.
- Add ignored active-retail validation tests only where filesystem installation
  evidence is required; ordinary behavior tests remain focused `--lib` tests.

The write-plan gate will pin exact paths and task order against the live tree
before implementation.

## Validation design

### Focused Rust `--lib` tests

- Constructor versus installed values for minimum, maximum, regen, and three
  image names.
- Multi-pass section absence and missing-key retention for only those fields.
- Signed/no-clamp min/max and inverted count order.
- Image `none`, unknown allocation, aliasing, and water-first Mark
  classification.
- Fresh 256-slot bytes, `(0,0)` sole emptiness, first-free ordering, and
  full-table zero-draw behavior.
- Signed request counts, negative/inverted values, and requests above 256.
- Exact `left + RandomRanged(0,width-1)` / Y equivalent, X/Y/timer draw
  counts, and retry ownership.
- Destination water/common identity selection, identity-based Float/Track
  selection, terrain-object ghost, non-exempt slope ghost, raw-ID-`0xB2`
  steep-slope eligibility, ground/deck raw-byte equality,
  structural bridge bypass, non-bridge speed-zero ghost, null-image ghost,
  visible stamp, and ghost cell preservation.
- Timer interpolation endpoints, an interior golden vector, start frame, aux,
  and expression-direction proof.
- Visible-only initial overlay-index sync, name preregistration, and frame-zero
  renderer/atlas preservation.
- Fixed-map campaign negative gate, fixed-map Skirmish positive gate,
  Crates-off zero-draw behavior, generated playable positive gate, and preview
  negative gate.
- Observable post-map ordering: startup crates before AI credits before
  alliances.
- Snapshot v114 round-trip of visible and ghost slots; state-hash sensitivity;
  historical hash stability; retail quick-checksum invariance.

### Literal external evidence

- Re-hash active `gamemd.exe` and record exact SHA-256.
- Re-read decisive caller/order and placement bodies from the active Ghidra
  session when an implementation detail is ambiguous.
- Read installed `rulesmd.ini` and compare exactly the six startup fields plus
  `[MultiplayerDialogSettings] Crates` to the production `RuleSet`/session
  result.
- Use supported release `asset.exe` to prove `CRATE.TEM` and `WCRATE.TEM`
  resolve from installed theater MIX and have expected SHP(TS) metadata.
- Run a production-loading test proving allocated identities reach crate
  placement and the initial presentation index, not only a unit helper.

### Cargo discipline

Before every Cargo command, check `Get-Process cargo,rustc`. While
implementing, run only focused `cargo test -p vera20k --lib <filter>`
commands. Run `cargo test -p vera20k --lib` exactly once after a fresh critic
passes and the PR is otherwise ready.

## Alternatives considered

### A. Chosen: narrow layered rules subset plus persistent crate authority

This follows native rules-process timing, creates one simulation owner, and
connects it through gated production loading and presentation. Later pickup and
regeneration mechanisms can consume the same slots without migration.

### B. Parse the full crate-rule cluster now

Rejected. Sound, radius, unit, money, fixed-goodie, and FreeMCV authority have
no startup consumer. Pulling them into this PR would create disconnected,
insufficiently evidenced state and expand review into later Phase 14
mechanisms.

### C. Flatten INI layers and patch missing values after merge

Rejected. A flattened projection cannot reproduce section/key retention or
resolve overlay identities at native Process-pass timing.

### D. Keep local bool slots and add a separate timer map

Rejected. It creates duplicate slot authorities and cannot represent ghosts or
raw save/load state.

### E. Implement all crate pickup/effects in the same PR

Rejected. Pickup return barriers, effect consumers, triggers, audio, and
animation are independently player-visible mechanisms.

## Adversarial review record

The first draft incorrectly inferred from the helper's missing internal
game-mode comparison that campaign must receive startup crates. Active caller
analysis disproved that hypothesis: `Full_Init` skips `Post_Map_Init` for
raw mode 0, and generated maps have a separate direct parent. This revision
therefore preserves `ScenarioPostMapOutput.crates: Option<_>` and the existing
campaign exclusion.

The first draft also over-bundled unrelated crate rules, put AI credits before
crates, selected Mark movement from destination surface, treated valid
`none` as fail-closed, and left the random coordinate upper bound open. This
revision:

- narrows rules authority to the six fields consumed at startup;
- orders crates before AI credits and alliances;
- makes Mark movement identity-based with water-first alias priority;
- makes every post-precheck failure, including null identity, an accepted timed
  ghost;
- preserves raw overlay ID `0xB2` as the exact steep-slope Mark exception;
- pins the inclusive coordinate as `RandomRanged(0,width-1)`.

Residual risks for design-review/review-plan:

1. Confirm exact slot coordinate versus timer-field write order where it affects
   a testable returned trace or failure state.
2. Confirm the accumulator can resolve per-pass overlay identities without
   exposing private registry internals or re-running flattened parsing.
3. Prove the existing
   `start_skirmish_session -> LoadingRequest::with_accepted_random_map` path
   retains `skirmish_session: Some` through post-map finalization, while dialog
   preview completion alone never constructs simulation state.
4. Keep first-presentation synchronization a consumer of simulation state,
   never a second mutation of `OverlayGrid`.
5. Prove all post-precheck failure cases stop retrying and still install the
   slot timer.

The fresh design-review approved this revision. The design still does not
claim the mechanism implemented.
