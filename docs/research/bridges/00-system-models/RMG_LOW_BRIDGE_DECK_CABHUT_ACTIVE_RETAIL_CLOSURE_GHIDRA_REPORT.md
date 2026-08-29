# Active-Retail RMG Low Bridge Deck, Ends, and CABHUT — Ghidra Re-investigation

**Address(es):** `0x0058EF10`, `0x0058F2C0`, `0x005902C0`, `0x005904B0`,
`0x005905D0`, `0x004865D0`, `0x005A6C10`, `0x005A7250`, `0x005A7440`,
`0x005A95B0`, `0x00595400`, `0x0043B740`, `0x00440580`, `0x00449440`,
`0x00464AC0`, `0x00716150`, `0x0047C620`, `0x0059E740`, `0x005A5020`, `0x005A6510`,
`0x005A82E0`, `0x005A91E0`, `0x005A1E10`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** The active YR random-map bridge-and-connector pass for map types 3 and 4:
water-region eligibility, low-deck candidate search, validation, overlay/data stamping, paved end
selection and tile stamping, bridge-repair hut search/construction, the ordered CABHUT/neutral-tech
constructor trace needed by launch parity, and the narrow negative distinction from waterfall
terrain shaping and unreachable RMG-shaped helpers.
**Non-Scope:** Land-region ramp carving formulas, the broader river/water generator, fixed-map
bridge discovery and runtime destruction/repair, high bridges, authored map object parsing, shell
pixel composition, and general neutral-tech footprint parity beyond the construction-event seam.
Those mechanisms keep their existing owners; facts are repeated here only when they fix ordering
or rule out a false bridge owner.
**Confidence:** High. All material control-flow, constants, rectangles, tile families, call-site
arguments, RNG sites, construction order, and exclusions were verified against the active retail
`gamemd.exe`. Installed YR retail INIs/TMP metadata establish theater identities and CABHUT facts.
OpenTS was used only to navigate inherited function correspondences and was never accepted as
parity authority.
**Active in YR:** Conditional active-retail behavior. `RandomMapGenerator__Generate` reaches the
pass for map types 3 and 4, including stock Skirmish Create Random Map and launch of an accepted
`.SED`. It is not an ordinary fixed-map load phase.

## 0. Working Notes Gate

- **Target question:** What complete low-bridge mechanism is active in retail YR RMG, which
  constructor events must advance the launch Scenario cursor, and which bridge-named or
  RMG-shaped routines must be excluded?
- **Prior state:** The lifecycle report proved launch regeneration and the dual-RNG contract, but
  intentionally left the low-deck placer's full geometry, end identities, CABHUT semantics, and
  waterfall-topology boundary open. Current Rust implements only a seed picker and two partial
  validators and skips the water-region branch.
- **Evidence needed to close:** active call path; all candidate and region gates; exact attempt and
  RNG rules; deck/end sweep semantics; direct overlay/data writes; exact theater tile identities;
  CABHUT rectangles and construction behavior; neutral-tech failed-attempt ordering; current Rust
  owners; and code/data-xref evidence for every scoped exclusion.
- **Stop condition:** every scoped branch is VERIFIED or excluded by active-retail reachability,
  with no OPEN, approximate, or implementation-blocking item.

## 1. Verdict

The active YR type-3/type-4 connector pass has two distinct branches. A land-class region connects
unequal-height neighboring regions with ramps. A flood-class region instead considers unordered
pairs of neighboring land regions and, when both are substantial and all three region levels are
equal, calls `RandomMapGenerator__PlaceLowBridgeDeck @ 0x0058F2C0`. Current Rust returns before
this water branch, so it removes an active player-visible bridge mechanism and also skips all of
its MapGen and Scenario RNG effects.

The low-deck placer makes at most 200 attempts. Each attempt rejection-draws a seed cell from the
entire square scratch array until it finds a real cell owned by the water region. It grows two
orthogonal three-cell-wide candidate corridors, validates the approach terrain and region pair,
prefers the shorter candidate with an east-west tie break, applies the attempt-dependent strict
length bound `length < attempt / 25 + 8`, and validates an inclusive one-cell margin around the
deck. A success directly writes the complete low-bridge overlay rectangle and cross-section data;
it does not call `OverlayClass::Mark`.

The end pieces are ordinary theater tile blocks. Defaults use `PavedRoadEnds`; alternate approach
pieces use `PavedRoads`. Alternate selection consumes a MapGen `U{0,1}` draw only when that end's
area validator succeeds. Tile-block stamping writes tile, sub-tile, slope, and scratch identity,
but the `-1` level argument preserves the existing level. The placer then independently searches
one primary and one fallback rectangle at each end for a CABHUT. A qualifying cell causes a
Neutral CABHUT constructor and therefore one raw Scenario word before `Unlimbo`; success of the
deck is not conditional on either hut.

The confusing `BuildRiverBridge @ 0x0059E740` is not a runtime low-bridge owner. It shapes river
terrain with waterfall tile sets, water fills, and level mutations and never writes low-deck
overlay or cross-section fields. The tile predicate at `0x004865D0` includes waterfall tile
families because a candidate deck validator may absorb that terrain, not because the waterfall
routine emits bridge topology. Three other RMG-shaped constructor helpers are unreachable from
the active generator and must not be added to the construction trace.

## 2. Evidence Hierarchy and Correspondence Routing

| Source | Permitted role in this report | Result |
|---|---|---|
| Active retail `gamemd.exe` | control flow, constants, arguments, field writes, RNG, call/xref reachability | parity authority |
| Installed YR retail theater/rules/art data | key identities, tileset spans, CABHUT type/foundation/owner semantics | data authority |
| Current Rust source | implementation ownership and delta only | implementation evidence |
| `C:\Users\enok\Documents\OpenTS\code\mapgen.cpp` | locate inherited `Connect_Regions_With_Bridge`, `Is_Bridge_Allowed`, `Place_Bridge_Hut`, and region-driver shapes | navigation lead only |
| `OpenTS\code\isotype.cpp`, `scenario.cpp` | locate PavedRoad keys and `.SED` entry correspondence | navigation lead only |

OpenTS correspondences were rechecked at every material point. In particular, the YR binary—not
the readable reference—establishes the active caller set, both-land endpoint condition, exact
runtime globals, waterfall-family predicate, YR overlay numbers, CABHUT construction chain, and
dormant-helper exclusions. `TrainBridgeSet` is inherited TS data surface with no active YR bridge
role and is not a port target.

## 3. Active Call Graph and Phase Order

```text
RandomMapGenerator__Generate @ 0x00598960
  map type 3/4 only
  -> island/flood terrain rebuild
  -> BridgeAndConnectorPass @ 0x0058EF10
       pass 1: build adjacency for every region
       pass 2: process connections for every region
         -> RmgRegion__CarveConnectorsOrBridges @ 0x005905D0
              flood-class region
              -> PlaceLowBridgeDeck @ 0x0058F2C0
                   -> rectangle road gate @ 0x005A7250
                   -> ValidateLowBridgeDeckArea @ 0x005902C0
                        -> tile-family predicate @ 0x004865D0
                   -> IsUniformLevelBridgeEndArea @ 0x005A7440
                   -> StampIsometricTileBlock @ 0x005A6C10
                   -> PlaceBridgeRepairHut @ 0x005904B0
                        -> BuildingClass__Constructor @ 0x0043B740
                             -> TechnoClass constructor raw Scenario draw
       pass 3: release native adjacency vectors
  -> starts
  -> neutral tech placement @ 0x005A95B0 or 0x00595400
       -> BuildingClass__Constructor -> same raw Scenario draw
```

Live disassembly of `0x0058EF10` shows the adjacency loop completes before the connection loop
begins; the native vectors are released only after every region has been processed. Rust may use
value-owned vectors, but it must preserve the complete-prepass view and region iteration order.

The region class flag read by `0x005905D0` is the flood-build classification produced from the
tile-family predicate or green terrain. It is not a generic runtime `Water` land-type flag. For a
flood-class region, the active bridge candidate gates are:

1. choose each unordered pair from that region's ordered neighbor list;
2. both neighboring regions are land-class;
3. each neighbor is substantial: `neighbor_count > 1 || cell_count > 50`;
4. neighbor A level equals neighbor B level and equals the flood region level;
5. call `0x0058F2C0(flood, neighborA, neighborB)` once for the pair.

The low-deck return value does not feed a retry at the region-driver level. Each qualifying pair
gets its one 200-attempt placer invocation.

## 4. Low-Deck Candidate Search

### 4.1 Attempt and seed-cell rules

`PlaceLowBridgeDeck` uses a zero-based attempt counter and stops after attempts `0..199` or the
first successful deck. At the start of every attempt, it samples one index uniformly from the
entire `scratch_width²` array. Each sampled index consumes a MapGen draw. It rejects and redraws
when either:

- the scratch record's region id is not the current flood-region id; or
- its stored coordinate is `(0,0)`, the sentinel for an unstamped/out-of-diamond record.

There is no native retry bound on this inner rejection loop. A valid active flood region owns
cells, so ordinary generation terminates. This seed picker is the only genuinely map-dependent
rejection-draw loop in the scoped bridge/connector subtree; arithmetic range guards elsewhere do
not fire for the generator's truncation and constants.

### 4.2 Two candidate axes

From the accepted seed `(x,y)`, native constructs both candidates before choosing one:

| Candidate | Initial paired walks | Final deck shape | Native selection name in this report |
|---|---|---|---|
| north/south walk | two `3 x 1` rectangles beginning at `(x-1,y)` and moving in opposite Y directions | `3 x span` | NS |
| west/east walk | two `1 x 3` rectangles beginning at `(x,y-1)` and moving in opposite X directions | `span x 3` | EW |

For both directions, every `0x005A7250` call passes the paved-road and paved-road-end override
arguments as zero. The walk continues while the current strip cannot accept clear/misc-pave/pave
terrain. After each step it probes the two outer cells of the three-wide strip; either leaving the
playfield or meeting `IsSpecialTerrainTile` kills that axis. A surviving first side also requires
the three-by-three approach block immediately beyond it to pass `0x005A7250`; the opposite side is
walked and must have its own beyond-end three-by-three block pass.

The twelve live call sites to `0x005A7250` all supply the same zero overrides. Consequently
`PavedRoads` and `PavedRoadEnds` refuse in candidate approaches; all other accepted tiles are
exactly Clear, MiscPave, or Pave under the generic rectangle predicate. There is no hidden
road-accepting active variant for low decks.

### 4.3 Region identity, length choice, and tie behavior

For a surviving NS candidate, native reads the scratch region id at the north and south endpoint
anchors. For a surviving EW candidate it reads west and east. The unordered pair must be exactly
the two land-region ids passed by `0x005905D0`; a flood-region endpoint or any third region refuses.

Invalid lengths are represented by a large sentinel. If both axes remain valid, strictly shorter
NS wins; a tie disables NS and therefore selects EW. Native then computes:

```text
max_exclusive_length = attempt / 25 + 8
```

The candidate length must be strictly less than this value. The thresholds therefore relax in
eight 25-attempt bands, from `< 8` at attempts `0..24` through `< 15` at attempts `175..199`.
The deck rectangle includes both endpoints: EW width is `eastX - westX + 1`, height `3`; NS width
is `3`, height `southY - northY + 1`.

No candidate-walk or length-choice step consumes RNG. Besides seed-cell rejection, the only
MapGen draws after a candidate succeeds are the conditional paved-end coins described below.

## 5. Deck Area Validator

`ValidateLowBridgeDeckArea @ 0x005902C0` reads the origin cell's signed level before checking the
four diamond corners. If any corner lies outside the map diamond it refuses. It then sweeps one row
and one column beyond the deck: `(width+1) * (height+1)` cells, inclusive on both axes. Every cell
must satisfy all of:

- `Cell+0x44 == -1` (no overlay);
- signed `Cell+0x11B` equals the origin level;
- tile is exactly Clear/unassigned or `0x004865D0(tile)` is true.

The helper at `0x004865D0` is not an overlay query despite stale naming. It is a leaf tile-family
predicate and reads no overlay field. Its exact accepted families are:

| Family | Range |
|---|---|
| ShorePieces | `base .. base + 0x2A` |
| WaterSet | `base .. base + 0x0E` |
| Waterfall east | `base .. base + 4` |
| Waterfall west | `base .. base + 4` |
| Waterfall south | `base .. base + 4` |
| Waterfall north | `base .. base + 4` |

These range ends are exclusive. The predicate takes no sub-tile argument and therefore accepts all
four tiles of each waterfall set. It does not accept `BridgeSet`, `WoodBridgeSet`, or existing
low-bridge overlays. The deck validator consumes no RNG.

Current Rust's `TileIds::is_bridge_absorbable` accepts only six WaterSet variants plus shore and
explicitly defers waterfall sets. Both choices are wrong for this call site: the live helper uses
fourteen water tiles and all four four-tile waterfall bands. A dedicated exact predicate or an
exactly widened owner is required; it must not silently reuse `is_special_terrain`, whose sub-tile
exceptions and extra cliff families implement a different native function.

## 6. Direct Deck Stamp

After validation, native writes every cell in the exact deck rectangle directly:

| Axis / cell role | `Cell+0x44` overlay id | `Cell+0x11E` cross-section data |
|---|---:|---:|
| EW west column | `0x5E` | `y - origin_y`, values `0..2` |
| EW east column | `0x5C` | `y - origin_y`, values `0..2` |
| EW interior | `0x4A + signed_mod(x,4)` | `y - origin_y`, values `0..2` |
| NS north row | `0x60` | `x - origin_x`, values `0..2` |
| NS south row | `0x62` | `x - origin_x`, values `0..2` |
| NS interior | `0x53 + signed_mod(y,4)` | `x - origin_x`, values `0..2` |

Coordinates on active generated maps are positive, but the implementation must preserve the
native signed-remainder rule rather than substitute an unsigned hash. The direct stamp writes no
isometric tile for body cells and consumes no RNG. It does not construct `OverlayClass`, call
`OverlayClass::Mark`, or run a fixed-map endpoint expansion.

The successful `.SED` reader branch is exclusive with the ordinary scenario overlay-pack reader;
there is no later native replay that expands these generated endpoints. Rust projection must carry
the already-materialized overlay/data rectangle once. Re-running fixed-map Mark would change
variants, topology flags, and Scenario consumption.

## 7. Paved End Validation, Coins, and Tile Identities

### 7.1 End-area predicate

`IsUniformLevelBridgeEndArea @ 0x005A7440` first checks the four corners, then reads the origin
level. Unlike the deck validator, it sweeps exactly `width * height` cells with exclusive upper
bounds. Every cell must have the same signed level. It ignores overlays. With the active override
argument zero at all four calls, PavedRoads and PavedRoadEnds refuse; otherwise the tile must be
Clear, MiscPave, or Pave. It consumes no RNG.

The exact end-validation rectangles are:

| End | Rectangle |
|---|---|
| EW east | `{deck.x + deck.w, deck.y - 2, 6, 6}` |
| EW west | `{deck.x - 6, deck.y - 2, 6, 6}` |
| NS north | `{deck.x - 2, deck.y - 6, 7, 6}` |
| NS south | `{deck.x - 2, deck.y + deck.h, 7, 6}` |

The north/south width is seven, not six. Older prose that generalized all four areas to `6x6` is
stale.

### 7.2 Conditional coins and exact tiles

Native draws `MapGen U{0,1}` only after the corresponding end-area validator returns true. A true
coin chooses an alternate PavedRoads block; validator false or coin false chooses the default
PavedRoadEnds block. The four ends execute in the order shown by their axis branch:

| Axis/end | Alternate when area true and coin true | Alternate anchor | Default | Default anchor |
|---|---|---|---|---|
| EW east | `PavedRoads + 10` | `(x+w, y)` | `PavedRoadEnds + 0` | `(x+w, y)` |
| EW west | `PavedRoads + 9` | `(x-4, y)` | `PavedRoadEnds + 2` | `(x-1, y)` |
| NS north | `PavedRoads + 13` | `(x, y-4)` | `PavedRoadEnds + 1` | `(x, y-1)` |
| NS south | `PavedRoads + 12` | `(x, y+h)` | `PavedRoadEnds + 3` | `(x, y+h)` |

All eight tile-block calls pass scratch id `-1` and level base `-1` to
`StampIsometricTileBlock @ 0x005A6C10`. For each non-null TMP subcell it writes tile id, sub-tile,
slope byte, and scratch tag `-1`. It does not change the cell level because level base is `-1`.
It consumes no RNG.

### 7.3 Retail theater data

`Read_Theater_TileSets_INI @ 0x00545150` resolves `[General] PavedRoads` to global
`0x00ABBEC8` and `PavedRoadEnds` to `0x00ABBEC4`. The installed active YR theater data is:

| Theater INI | PavedRoads key / count / file | PavedRoadEnds key / count / file |
|---|---|---|
| `temperatmd.ini` | `20 / 21 / Proad` | `36 / 4 / p_end` |
| `urbanmd.ini` | `20 / 21 / Proad` | `36 / 4 / p_end` |
| `urbannmd.ini` | `20 / 21 / Proad` | `36 / 4 / p_end` |
| `desertmd.ini` | `20 / 21 / Proad` | `36 / 4 / p_end` |
| `snowmd.ini` | `20 / 21 / Proad` | `38 / 4 / p_end` |
| `lunarmd.ini` | `20 / 0` | `36 / 0` |

The keys resolve to cumulative flat bases even for zero-length Lunar sets, per the verified theater
loader contract. Missing TMP payload prevents a useful lunar end block; it does not change the
native arithmetic identities above. No active YR call reads `TrainBridgeSet` for this mechanism.

## 8. Bridge Repair Huts

### 8.1 Search rectangles and fallback order

After deck and end tiles, each axis performs two independent hut searches. A false primary result
immediately tries its fallback; a true primary suppresses only that end's fallback. The deck still
returns success if all hut searches fail.

| Axis/end | Primary rectangle | Fallback rectangle |
|---|---|---|
| EW west | `{x, y-1, 2, 5}` | `{x-1, y-2, 3, 7}` |
| EW east | `{x+w-2, y-1, 2, 5}` | `{x+w-2, y-2, 3, 7}` |
| NS north | `{x-1, y, 5, 2}` | `{x-2, y-1, 7, 3}` |
| NS south | `{x-1, y+h-2, 5, 2}` | `{x-2, y+h-2, 7, 3}` |

`PlaceBridgeRepairHut @ 0x005904B0` scans each supplied rectangle inclusive on both axes—again
`(w+1)*(h+1)` cells—in Y-major, then X-major order. The first cell satisfying all three predicates
wins:

- `Cell+0x44 == -1`;
- tile passes exact Clear/unassigned test;
- `Cell+0xE4 == 0` (no occupier).

There is no MapGen draw in the helper and no random selection among qualifying cells.

### 8.2 Construction and active outcome

Only after a cell qualifies, native resolves house `Neutral` and building type `CABHUT`, allocates
a `0x720`-byte `BuildingClass`, calls its constructor, and invokes `Unlimbo` at cell center
`(x*256+128, y*256+128, 0)` with direction zero. The helper ignores the `Unlimbo` return and
returns true immediately after the construction attempt. Allocation occurs after the candidate
search, so an all-failing search consumes no Scenario word.

The installed retail data gives CABHUT `Foundation=1x1`, `BridgeRepairHut=yes`, and Neutral
construction ownership. Live `BuildingClass::Unlimbo @ 0x00440580` first calls the Building
placement virtual at vtable `+0x1AC`; the CABHUT target is `0x00449440`, which delegates the
ordinary path through `BuildingTypeClass__CanPlaceAt @ 0x00464AC0` and
`TechnoTypeClass__CanPlaceAt @ 0x00716150`. The latter walks the foundation and validates its
target cell through `Cell_passability_building_placement @ 0x0047C620`. That leaf requires the
cell to be in the playfield, free of disqualifying overlay/occupier/bridge bits and slope, and
buildable for the type's land/speed row. For stock 1x1 CABHUT in this active generator phase,
the endpoint approach is already playfield-gated, the helper supplies the sole clear/unoccupied
foundation cell, clear terrain has the buildable/flat row, and no prior topology writer has set
the excluded flags. `TechnoClass__Unlimbo` therefore emits the building. Thus the ordinary active
retail event is `Emitted { CABHUT, cell }`; the helper's ignored return does not introduce a
second placement attempt or make deck success conditional on the hut.

System allocation failure is outside deterministic active-retail parity: no gameplay rule or RNG
selects it, and native itself cannot produce a usable object in that state. It is an explicit OOM
exclusion, not an unresolved approximation.

### 8.3 Scenario constructor word

`BuildingClass__Constructor @ 0x0043B740` reaches the base Techno constructor. Its unconditional
tail loads the process Scenario RNG, performs one raw `Random__Next`, and stores the low 16 bits at
`Techno+0x3C8`. This happens before `Unlimbo`. Therefore:

- each qualifying CABHUT construction contributes exactly one ordered Scenario event;
- a failed hut search contributes none;
- the word is shared Techno state, not a bridge-private random choice;
- MapGen decides geometry/end variants, while Scenario supplies constructor words;
- projection must install the already-consumed word and must not draw again.

At most two CABHUT constructors occur per successful deck, one per end. Their event order is the
axis branch's first-end primary/fallback search followed by second-end primary/fallback search.

## 9. Neutral-Tech Construction Events in the Same Trace

The active construction trace cannot stop at CABHUT. After starts, active type-3/type-4 generation
calls the whole-map neutral-tech owner `0x005A95B0`:

1. draw intended building count `U[0,4]` from MapGen;
2. for each intended building, draw a type from retail `NeutralTechBuildings`;
3. allocate and construct the Building **before** the placement-attempt loop;
4. run up to 100 outer placement attempts, with the whole-map anchor rejection behavior;
5. on success retain/emits the building; after 100 failures destroy it.

Every constructed object consumes one Scenario word at step 3. A later discarded object retains
the cursor effect and has no emitted entity binding. Map type 2 reaches the region-scoped owner
`0x00595400` after a `U[0,2]` pass count and has the same construct-before-attempt property. It is
not a bridge placement phase, but it must share the generic construction trace vocabulary so the
type-3/type-4 launch cursor remains exact after CABHUTs.

Stock `NeutralTechBuildings` is `CAAIRP,CATHOSP,CAOILD,CAOUTP,CAMACH,CAPOWR`. Current Rust draws
the type before attempts but returns only successful `TechPlacement` rows, so it has exactly the
decision point needed to append a construction event but currently loses discarded events and all
constructor words.

The required ordered event shape is:

```text
RmgConstructionEvent {
  ordinal,
  phase: BridgeRepairHut | NeutralTech,
  techno_type,
  outcome: Discarded | Emitted { entity_index, cell }
}
```

Launch replay consumes one raw Scenario word per event. `Discarded` drops the low word. `Emitted`
adds a validated `GeneratedTechnoInit` binding for the final generated `MapEntity`. The existing
`GeneratedTechnoInitTable` already rejects duplicate/missing indices and type/cell mismatches and
installs a preconsumed word without a second draw; Unit 2 must carry the trace to that owner.

## 10. Waterfall Terrain Is Not Bridge Topology

`RandomMapGenerator__BuildRiverBridge @ 0x0059E740` is reached only from the active river carver.
Despite its inherited name, it:

- fills water rectangles and mutates river-region ids;
- grows/dilates the river arm and changes terrain levels;
- stamps the four theater waterfall tile families through isometric tile-block helpers;
- writes tile/sub-tile/slope/level/scratch facts;
- does not directly write `Cell+0x44` overlay or `Cell+0x11E` cross-section data;
- has no callee that materializes a low bridge, bridge flags, Tube records, or runtime bridge
  topology.

A full instruction search over the 1,626-instruction function found no access to `+0x44` or
`+0x11E`. The tile-stamp callees likewise write isometric terrain facts only. This mechanism stays
owned by river/waterfall terrain. Its only scoped bridge relationship is that
`ValidateLowBridgeDeckArea -> 0x004865D0` accepts those waterfall tile families during a later
candidate check.

Required negative characterization: after a deterministic `BuildRiverBridge` success, waterfall
tiles/levels may change, but overlay id, overlay data/cross index, raw bridge flags, explicit Tube
records, and generated CABHUT/tech trace remain untouched by that function.

## 11. Dormant and TS-Only Exclusions

Fresh live xref/caller and pointer-byte census proves:

| Function | Reachability result | Scope verdict |
|---|---|---|
| `0x005A6510` | sole caller `0x005A5020` | exclude |
| `0x005A82E0` | sole caller `0x005A5020` | exclude |
| `0x005A5020` | no code/data refs; little-endian entry pointer absent from image | exclude root and descendants |
| `0x005A91E0` | no code/data refs; entry pointer absent from image | exclude |
| `0x005A1E10` duplicate-shaped helper | no code/data refs; entry pointer absent | exclude |

None is in the transitive active caller closure of `RandomMapGenerator__Generate`. They must not be
used to add guessed constructor events, bridge phases, or MapGen draws. `TrainBridgeSet` appears in
the inherited OpenTS theater reader but has no active YR consumer in this mechanism; it is a
TS-only/dormant data lead and is excluded.

## 12. Current Rust Delta

| Rust owner | Current state | Verified mismatch / required ownership |
|---|---|---|
| `src/map/rmg/phases/carve_driver.rs` | returns immediately for `waterish`; claims branch costs no draws | active flood branch calls low-deck placer and consumes seed rejections, conditional end coins, and Scenario constructor events |
| `src/map/rmg/pipeline.rs` | rebuilds regions and iterates a simplified `ConnectorRegion`; adjacency prepass is only implicit | must provide ordered neighbor/cell-count/flood-class facts and run both land ramp and water deck branches in native pass order |
| `src/map/rmg/phases/bridge_deck.rs` | seed picker plus two validators only; header calls RMG dormant | mechanism is active for type 3/4; implement full search, stamp, ends, huts, and exact predicate |
| `TileIds::is_bridge_absorbable` | six WaterSet variants + shore; waterfall deferred | exact helper is WaterSet span 14 + shore 42 + four waterfall spans 4 |
| `src/map/rmg/phases/tech_buildings.rs` | returns successful placements only | emit ordered construction event before attempts, including discarded attempts |
| `PipelineOutput` / `GeneratedMap` | structures and MapGen continuation only | carry immutable ordered construction trace separately from surviving entities |
| `src/map/rmg/emit.rs` | emits neutral tech structures in vector order | include CABHUTs and preserve stable generated entity indices used by trace bindings |
| `RandomMapGenerationRetention` | accepted preview `GeneratedMap` becomes loading authority | preview remains UI-only; `.SED` launch regenerates and creates a new trace |
| `retained_random_map_initial` | explicitly skips launch regeneration | contradicted by active `.SED` reader path; remove/bypass as gameplay authority |
| `ScenarioBootstrapRng` | created after map generation and installs only MapGen continuation | one match-seeded owner must span preload, Fill, trace replay, projection, post-map, and sim |
| `construct_scenario` | always calls fresh fixed-map projection | generated maps must validate/install the trace-derived init table and consume no second constructor draw |
| `GeneratedTechnoInitTable` | already validates index/type/cell and preconsumed word | suitable receiving owner; no design replacement needed |
| `src/map/rmg/phases/bridge.rs` | implements waterfall terrain but calls it river bridge; file claims entire RMG dormant | retain terrain behavior; correct scope comments and add no-topology characterization |

The smallest architecture-correct repair is one launch-generated map plus one ordered construction
trace, replayed on the single gameplay `ScenarioBootstrapRng`. It is not a second Scenario
continuation stored beside MapGen, and it is not a CABHUT-only counter.

## 13. Coverage Ledger

| Mechanism / branch | Status | Evidence | Remaining research |
|---|---|---|---|
| type-3/type-4 active entry | VERIFIED | `0x00598960` map-type branch to `0x0058EF10` | none |
| three-pass adjacency/connection/release order | VERIFIED | live `0x0058EF10` disassembly | none |
| flood-class bridge branch | VERIFIED | `0x005905D0` | none |
| both-land/substantial/equal-level gates | VERIFIED | `0x005905D0` | none |
| 200 attempts and whole-scratch seed rejection | VERIFIED | `0x0058F2C0` | none |
| NS/EW corridor walks and approach gates | VERIFIED | `0x0058F2C0`, all `0x005A7250` calls | none |
| endpoint region pair | VERIFIED | `0x0058F2C0` | none |
| shorter-wins, EW tie, strict length bands | VERIFIED | `0x0058F2C0` | none |
| inclusive deck area and level/overlay gates | VERIFIED | `0x005902C0` | none |
| exact water/shore/waterfall family helper | VERIFIED | `0x004865D0` | none |
| direct EW/NS overlay and data stamp | VERIFIED | `0x0058F2C0` | none |
| exact end rectangles and override zero | VERIFIED | `0x0058F2C0`, `0x005A7440` | none |
| conditional end coins and execution order | VERIFIED | `0x0058F2C0` | none |
| PavedRoads/PavedRoadEnds identities | VERIFIED binary + retail data | `0x00545150`, theater INIs/TMP counts | none |
| tile-block stamp semantics | VERIFIED | `0x005A6C10` | none |
| hut primary/fallback rectangles | VERIFIED | `0x0058F2C0` | none |
| inclusive hut scan and candidate gates | VERIFIED | `0x005904B0` | none |
| CABHUT type/foundation/owner | VERIFIED retail data | rules/art INIs | none |
| CABHUT construction and Scenario draw | VERIFIED | `0x005904B0 -> 0x0043B740 -> Techno ctor` | none |
| ordinary CABHUT emitted outcome | VERIFIED binary + retail data | `0x00440580`, `0x00449440`, `0x00464AC0`, `0x00716150`, `0x0047C620` | none |
| neutral-tech construct-before-attempt | VERIFIED | `0x005A95B0`, `0x00595400` | none |
| waterfall terrain no-topology boundary | VERIFIED | `0x0059E740` and callee/write census | none |
| dormant helper exclusion | VERIFIED | xref/caller/pointer-byte census | none |
| current Rust mismatch | VERIFIED | direct source scan in Section 12 | implementation required |

## 14. Open Questions — Final State

- `[RESOLVED] OQ-01 — Is low-deck placement active in retail YR? -> Yes, conditionally for RMG map types 3/4.`
- `[RESOLVED] OQ-02 — Does the connector pass build adjacency before any connection? -> Yes, all-region prepass, then all-region connection pass.`
- `[RESOLVED] OQ-03 — What does the region water flag mean here? -> Flood-build class from the active terrain classifier, not generic runtime Water land type.`
- `[RESOLVED] OQ-04 — Can a water/flood neighbor be a deck endpoint? -> No; both endpoint regions must be land-class.`
- `[RESOLVED] OQ-05 — How often is one neighbor pair attempted? -> Once, through one 200-attempt placer call.`
- `[RESOLVED] OQ-06 — Which rejection loops really spend MapGen draws? -> Seed-cell region/(0,0) rejections; candidate walks and validators spend none.`
- `[RESOLVED] OQ-07 — Is the attempt limit 199 or 200? -> 200 attempts, zero-based indices 0..199.`
- `[RESOLVED] OQ-08 — What happens on equal NS/EW lengths? -> EW wins.`
- `[RESOLVED] OQ-09 — Is the length threshold inclusive? -> No; candidate length must be strictly less than attempt/25+8.`
- `[RESOLVED] OQ-10 — Does deck validation sweep only the deck? -> No; it sweeps (w+1)*(h+1).`
- `[RESOLVED] OQ-11 — Does the deck helper accept six or fourteen water tiles? -> Fourteen, plus shore 42 and four waterfall bands of four.`
- `[RESOLVED] OQ-12 — Are waterfall sub-tile exceptions relevant? -> No; 0x004865D0 takes only tile id and accepts each whole four-tile band.`
- `[RESOLVED] OQ-13 — Are existing low bridges accepted by the deck tile helper? -> No; overlay refuses first and bridge tilesets are not in 0x004865D0.`
- `[RESOLVED] OQ-14 — Does native construct OverlayClass/Mark for generated decks? -> No; it writes the full overlay/data rectangle directly.`
- `[RESOLVED] OQ-15 — Which globals own alternate/default end identities? -> PavedRoads at 0x00ABBEC8; PavedRoadEnds at 0x00ABBEC4.`
- `[RESOLVED] OQ-16 — Are all end validator rectangles 6x6? -> No; NS ends are 7x6.`
- `[RESOLVED] OQ-17 — Is an end coin drawn when the area fails? -> No; failure selects default without a draw.`
- `[RESOLVED] OQ-18 — Does end tile stamping alter level? -> No; level-base -1 preserves it.`
- `[RESOLVED] OQ-19 — Is a hut chosen randomly within its rectangle? -> No; first Y-major/X-major qualifying cell wins.`
- `[RESOLVED] OQ-20 — Does a failed hut search consume Scenario? -> No; allocation/constructor occurs only after a candidate.`
- `[RESOLVED] OQ-21 — Does a qualifying CABHUT consume Scenario before placement? -> Yes, one raw word in the base Techno constructor before Unlimbo.`
- `[RESOLVED] OQ-22 — Can ordinary stock CABHUT fail its 1x1 foundation after the helper gate? -> No on the active generated-map fixture; the sole clear/unoccupied cell is the inherited placement walk's footprint.`
- `[RESOLVED] OQ-23 — Does missing either hut fail the deck? -> No; deck success is already committed.`
- `[RESOLVED] OQ-24 — Are final neutral-tech rows enough to reconstruct Scenario draws? -> No; construction precedes the 100 placement attempts and discarded events remain spent.`
- `[RESOLVED] OQ-25 — Is BuildRiverBridge a low-bridge topology owner? -> No; it is waterfall/river terrain shaping only.`
- `[RESOLVED] OQ-26 — Why are waterfall tiles in a bridge validator? -> They are absorbable terrain for a later deck candidate, not topology emitted by BuildRiverBridge.`
- `[RESOLVED] OQ-27 — Are 0x005A5020 descendants or 0x005A91E0 active trace owners? -> No; no active callers or stored entry pointers.`
- `[RESOLVED] OQ-28 — Is TrainBridgeSet active YR evidence? -> No; inherited TS-only/dormant surface.`
- `[RESOLVED] OQ-29 — Can preview output be launch authority? -> No; verified `.SED` launch regenerates and produces a fresh construction trace.`
- `[RESOLVED] OQ-30 — Does any scoped material claim rely on OpenTS? -> No; every one was independently verified in gamemd/retail data.`

## 15. Adversarial Review

1. **Could the zero-draw claim for the water branch be saved by saying deck placement is dormant?**
   No. The active type-3/type-4 Generate path reaches `0x0058EF10`, `0x005905D0` is its live
   region connection virtual, and the only low-deck caller is live from its flood branch.
2. **Could `0x004865D0` really be an overlay/bridge test whose decompile lost the cell read?**
   No. Fresh disassembly/callee review shows a leaf range predicate over tile id globals and no
   `Cell+0x44` access. Overlay refusal is an explicit independent check in `0x005902C0`.
3. **Could default end pieces come from PavedRoads too, making the two globals interchangeable?**
   No. Live loads at all four default paths reference `0x00ABBEC4`; all alternates reference
   `0x00ABBEC8` with offsets 9/10/12/13. Retail INI parsing resolves those separate keys.
4. **Could CABHUT words be replayed by counting final structures?**
   No. CABHUT construction order is interleaved with deck/end work, and neutral-tech owners
   construct before attempts that can discard the object. Only an ordered event trace preserves
   both words and final bindings.
5. **Could waterfall terrain be promoted to bridge topology because its native function is named
   BuildRiverBridge?** No. The complete function/callee write census shows tile/level/scratch
   mutation and no overlay/data/flag/Tube topology. Name inheritance is not behavior evidence.
6. **Could the OpenTS loop be ported verbatim?** No. It is useful readable correspondence, but YR
   active reachability, both-land eligibility, exact globals, overlay constants, and helper family
   ranges were re-established from `gamemd.exe` and retail data.

## 16. Exhaustive-Slice Closure Checks

### 16.1 Zero-add pass

After forming the mechanism model, a second pass started at the active Generate map-type branch
and walked the full scoped call/callee closure again. It added no new active bridge mechanism:

- `0x0058EF10` has exactly the adjacency, connection, and release phases already listed;
- `0x005905D0` has the land-ramp and flood-deck branches only;
- `0x0058F2C0` calls only the two area predicates, direct cell writes, end tile stamper, and hut
  helper relevant here;
- `0x005904B0` has one Building constructor path after candidate qualification;
- active generated Building constructor callers reduce to CABHUT plus the two neutral-tech owners;
- `0x0059E740` and its callees add terrain facts only;
- xref/pointer census adds none of the dormant helpers to the active root.

The pass changed no conclusion and produced no deferred material question.

### 16.2 Cold spot-checks

- **Cold spot-check A:** re-read the `0x0058F2C0` end-selection disassembly without the OpenTS
  source open. The independent result again resolved alternates to PavedRoads `+10,+9,+13,+12`,
  defaults to PavedRoadEnds `+0,+2,+1,+3`, the exact anchors above, and coin-after-validator order.
- **Cold spot-check B:** re-read `0x005A7440` and `0x005904B0` from entry. The independent result
  again found exclusive `w*h` end sweeps versus inclusive `(w+1)*(h+1)` hut sweeps, overlay-ignore
  versus overlay-required-clear, and allocation only after the first qualifying hut cell.

## 17. Implementation Handoff

| Verified requirement | Current Rust delta | Required effect | Acceptance gate | Forbidden shortcut |
|---|---|---|---|---|
| Process every active flood-region neighbor pair in native pass order. | waterish region returns false | retain ordered neighbor facts/counts and call low-deck placer after full adjacency prepass | fixed synthetic region graph visits exact qualifying pairs once | do not infer water solely from runtime land type |
| Preserve exact MapGen draw order. | no deck calls/draws | rejection-draw seed, then conditional end coins only | fixture asserts accepted cell, rejection count, coin count, and post-phase cursor | do not draw coins on failed end areas |
| Reproduce NS/EW search and choice. | absent | exact strip/approach gates, region pair, EW tie, strict length bands | fixtures cover one-axis, two-axis shorter, tie, and each threshold boundary | do not collapse into nearest-end search |
| Validate exact absorbable families. | six water + shore only | water span 14, shore 42, four waterfall spans 4; no extra special families | boundary test at every base-1/base/base+last/base+span | do not reuse sub-tile-sensitive special-terrain predicate |
| Direct-stamp complete deck. | absent | write exact overlay/data rectangle once | EW and NS golden rectangles include endpoints/interior/cross index | do not call fixed-map Overlay::Mark |
| Stamp correct ends. | absent | exact four validators, conditional coins, bases/offsets/anchors, preserve level | all default/alternate paths and NS 7x6 asymmetry tested | do not treat PavedRoads and PavedRoadEnds as one family |
| Construct up to two ordered CABHUTs. | absent | inclusive primary/fallback scan; first candidate emits neutral CABHUT and trace event | primary, fallback, no-cell, two-end, and deck-success-without-hut fixtures | do not make hut success a deck gate |
| Preserve all generated constructor events. | successful tech rows only | append CABHUT/neutral-tech events at constructor point, including discarded tech | emitted/discarded interleave yields exact trace and post-replay cursor | do not count final entities to infer draws |
| Bind emitted constructor words. | table exists but is not fed | replay trace on single launch Scenario owner; validate entity index/type/cell; projection draws zero | identity mismatch fails before mutation; correct table leaves cursor unchanged during spawn | do not create a second Scenario cursor |
| Regenerate accepted `.SED` at launch. | preview retained as gameplay map | preview remains UI-only; launch builds fresh map/trace after match reseed and Full-Init prefix | preview construction count cannot affect gameplay map/words; same `.SED` fresh-process launch agrees | do not hand preview `GeneratedMap` into play |
| Keep waterfall terrain separate. | misleading dormant/bridge comments | retain terrain outputs; characterize no topology/trace writes | deterministic river-bridge fixture changes terrain only | do not create overlays/flags/CABHUTs from `0x0059E740` |

Suggested focused tests:

- `rmg_flood_region_visits_each_eligible_land_pair_once`
- `rmg_low_deck_seed_rejections_and_end_coins_match_native_cursor`
- `rmg_low_deck_east_west_wins_equal_length_tie`
- `rmg_low_deck_length_gate_relaxes_every_twenty_five_attempts`
- `rmg_deck_validator_accepts_exact_water_shore_and_four_waterfall_bands`
- `rmg_low_deck_direct_stamp_matches_ew_and_ns_overlay_data_rectangles`
- `rmg_low_deck_end_areas_and_tiles_match_all_four_native_cases`
- `rmg_bridge_hut_primary_then_fallback_scan_is_inclusive_and_ordered`
- `rmg_bridge_success_does_not_require_a_repair_hut`
- `rmg_constructor_trace_interleaves_cabhut_emitted_and_neutral_tech_discarded`
- `rmg_trace_replay_binds_generated_words_without_projection_redraw`
- `rmg_accepted_preview_is_not_launch_map_authority`
- `rmg_build_river_bridge_changes_no_overlay_cross_flag_tube_or_constructor_trace`
- `rmg_dormant_constructor_helpers_never_enter_active_trace`

## 18. Negative Facts / Evidence-Backed Exclusions

- Do not call the active type-3/type-4 RMG bridge system dormant.
- Do not skip the water-region branch on the premise that it consumes no RNG.
- Do not classify the region flag as a generic water land type without the flood-build predicate.
- Do not permit flood/water regions as deck endpoints; both endpoints are land regions.
- Do not accept only six water variants in deck validation; native uses fourteen.
- Do not apply waterfall sub-tile exceptions or unrelated cliff families to `0x004865D0`.
- Do not shrink the deck validator to `w*h`; its margin is inclusive on both axes.
- Do not expand the end validator to `(w+1)*(h+1)`; it is exactly `w*h`.
- Do not let overlays refuse an end area; that helper ignores them.
- Do not draw a paved-end coin when its area predicate is false.
- Do not alter level while stamping paved end blocks; the level argument is `-1`.
- Do not rerun `OverlayClass::Mark` over generated low decks.
- Do not choose a random qualifying CABHUT cell; native takes the first scan-order cell.
- Do not fail or retry a committed deck because CABHUT placement failed.
- Do not infer constructor events from surviving structures.
- Do not use MapGen for Techno constructor words or Scenario for deck geometry.
- Do not treat `BuildRiverBridge` waterfall terrain as runtime bridge topology.
- Do not add Tube records, bridge raw flags, overlays, or CABHUTs to that waterfall routine.
- Do not activate `0x005A5020`, `0x005A6510`, `0x005A82E0`, `0x005A91E0`, or `0x005A1E10`.
- Do not port `TrainBridgeSet` from OpenTS into active YR behavior.
- Do not retain the preview-generated map or preview Scenario effects into gameplay.
- Do not claim OOM behavior as a deterministic parity mechanism; native has no gameplay recovery
  contract for allocation failure.

## 19. Ghidra Annotation Candidates

No metadata was changed during this read-only investigation. The following are certainty-gated
candidates for a later explicitly authorized sync:

- rename stale `0x004865D0` to an exact water/shore/waterfall tile-family predicate name;
- document `0x0058F2C0` with active type-3/type-4 reachability, EW tie, length bands, and direct
  overlay/data stamp;
- document `0x005A7440` as the exact end-area predicate with all low-deck call-site overrides zero;
- document `0x005904B0` with inclusive scan, constructor-before-Unlimbo, ignored return, and
  Scenario-word side effect;
- mark `0x0059E740` as waterfall/river terrain shaping, not runtime low-bridge topology;
- tag `0x005A5020`, `0x005A6510`, `0x005A82E0`, `0x005A91E0`, and `0x005A1E10` as
  unreachable/dormant only if the project's annotation policy accepts reachability tags.

## 20. Sources

- Fresh read-only Ghidra decompile/disassembly/call-site review:
  `0x0058EF10`, `0x0058F2C0`, `0x005902C0`, `0x005904B0`, `0x005905D0`,
  `0x004865D0`, `0x005A6C10`, `0x005A7250`, `0x005A7440`, `0x005A95B0`,
  `0x00595400`, `0x00440580`, `0x00449440`, `0x00464AC0`, `0x00716150`, `0x0047C620`, and
  `0x0059E740`.
- Fresh read-only caller/xref and little-endian entry-pointer census:
  `0x005A5020`, `0x005A6510`, `0x005A82E0`, `0x005A91E0`, `0x005A1E10`.
- Installed retail data: `temperatmd.ini`, `snowmd.ini`, `urbanmd.ini`, `urbannmd.ini`,
  `desertmd.ini`, `lunarmd.ini`, `rulesmd.ini`/fallback `rules.ini`, and
  `artmd.ini`/fallback `art.ini`; corresponding TMP corpus metadata.
- Reconciled primary reports:
  `RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`,
  `MAPGEN_SAME_PROCESS_LIFECYCLE_BRIDGE_CALLER_RECONCILIATION_GHIDRA_REPORT.md`,
  `RMG_BRIDGE_CONNECTOR_PASS_0058EF10_GHIDRA_REPORT.md`, and
  `RMG_MODE34_WATER_BRIDGES_TECH_GHIDRA_REPORT.md`.
- Current Rust source:
  `src/map/rmg/phases/bridge_deck.rs`, `bridge.rs`, `carve_driver.rs`,
  `tech_buildings.rs`, `src/map/rmg/tiles.rs`, `pipeline.rs`, `build.rs`, `emit.rs`,
  `src/app/shell_random_map.rs`, `src/app/loading/init.rs`,
  `src/sim/scenario_bootstrap.rs`, `src/sim/runtime.rs`, and
  `src/sim/world/world_spawn.rs`.
- OpenTS correspondence leads only:
  `C:\Users\enok\Documents\OpenTS\code\mapgen.cpp`, `isotype.cpp`, and
  `scenario.cpp`. No material conclusion in this report relies on OpenTS alone.
