# Automatic Tube Shell Recalc Load Order — Ghidra Research Report

**Address(es):** `CellClass::RecalcAttributes @ 0x0047D2B0`, `TubeClass::Constructor @ 0x00727FD0`, `Read_Theater_TileSets_INI @ 0x00545150`, `MapClass::ReadTubesINI @ 0x007283C0`, `ScenarioClass::Full_Init @ 0x00686B20`, `MapClass::InitCellAttributes @ 0x00568BB0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** automatic same-cell `TubeClass` shell construction inside `CellClass::RecalcAttributes`; its exact predicate, direction, constructor effects, fresh-authored-load timing, explicit-`[Tubes]` interaction, native-ID/registry order, active YR theater bindings, retail TMP eligibility, and the corresponding current Rust ownership gap
**Non-Scope:** direction-8 locomotion internals, full explicit-tube traversal semantics, complete Tube save/load/destruction behavior, bridge damage/repair, full RMG generation semantics, and every runtime cell-mutation caller outside the load boundaries needed here
**Confidence:** High for executable behavior, constructor/order effects, theater globals, retail bands, and retail TMP eligibility; Medium for stock-map frequency because a final-state placement census of all shipped map payloads remains deferred
**Active in YR:** Conditional — the executable path and four active YR theater keys are live, and 36 exact stock URBAN/NEWURBAN TMP/subtile identities satisfy its tile/land predicate; whether any winning shipped map payload places one of those identities in a qualifying final cell remains uncensused

## 1. Overview

`CellClass::RecalcAttributes` can synchronously allocate a zero-step `TubeClass` shell for a normal-path cell whose final land is native `Tunnel` (`10`), whose signed tube index is invalid, and whose tile ID falls in the first four cumulative tile IDs of one of four theater families. The shell is a registered abstract object: it consumes the shared scenario unique-ID cursor, appends to the global Tube vector, and normally publishes its array index to the cell before Recalc continues into terrain-attached Anim construction.

This mechanism is separate from stock low bridge overlays. Retail `LOBRDG*`/`LOBRDB*` cells resolve through the Road/`NoUseTileLandType` early return and cannot reach the automatic Tube branch. Retail theater assets do nevertheless activate the automatic predicate independently: 36 URBAN/NEWURBAN tunnel-floor TMP subtiles map to native land `10` within the four fixed bands.

The current Rust shape and tile predicate are mostly correct, but ownership is not. `ResolvedTerrainGrid::build_inner` eagerly batches shells during Fill in rectangular row-major order, before production constructs raw explicit Tubes and without assigning shell native IDs. The live authored Recalc transaction does not create Tube effects, so it cannot reproduce inline Overlay-Mark timing, anti-diagonal sweep timing, constructor identity order, or the final-sweep no-duplicate rule.

## 2. Class Layout / Key Offsets

| Owner | Offset / global | Type | Verified purpose |
|---|---:|---|---|
| `CellClass` | `+0x24/+0x26` | `i16, i16` | map coordinate passed to the automatic constructor |
| `CellClass` | `+0x38` | `i32` | final isometric tile ID tested against the four fixed bands |
| `CellClass` | `+0x44` | `i32` | raw overlay type index; overlay branches can return before automatic construction |
| `CellClass` | `+0xEC` | `i32` | native final land type; automatic construction requires exact value `10` |
| `CellClass` | `+0x116` | `i16` | signed Tube-array index; negative or `>= g_TubeCount` admits construction |
| `CellClass` | `+0x11A` | `u8` | TMP subtile index used by land lookup |
| `CellClass` | `+0x11B` | `u8` | level; explicit Recalc override is applied after automatic Tube construction |
| `CellClass` | `+0x11C` | `u8` | TMP slope, read before LAT and land resolution |
| `TubeClass` | `+0x10` | `u32` | `AbstractClass` scenario unique ID assigned by the constructor |
| `TubeClass` | `+0x24/+0x26` | `i16, i16` | entry cell coordinate |
| `TubeClass` | `+0x28/+0x2A` | `i16, i16` | exit coordinate; automatic shell initially copies entry |
| `TubeClass` | `+0x2C` | `i32` | direction from `[2, 4, 6, 0]` |
| `TubeClass` | `+0x30..+0x1BC` | `[i32; 100]` | path array, initialized entirely to `-1` |
| `TubeClass` | `+0x1C0` | `i32` | path length, initialized to `0` for automatic shells |
| global | `0x008B413C` | `TubeClass **` | Tube vector data |
| global | `0x008B4148` | `i32` | Tube vector count |
| global | `0x0081CC20` | `[i32; 4]` | automatic direction table: `2, 4, 6, 0` |
| global | `0x00AA1054` | `i32` | cumulative base for `[General] Tunnels` |
| global | `0x00ABB108` | `i32` | cumulative base for `[General] TrackTunnels` |
| global | `0x00AA10B4` | `i32` | cumulative base for `[General] DirtTunnels` |
| global | `0x00ABAD2C` | `i32` | cumulative base for `[General] DirtTrackTunnels` |
| global | `0x008288E4` | `[i32; 16]` | TMP-terrain-byte to native-LandType table; entry `5` is `10` |

`TubeClass::Constructor` installs four Tube vptrs at `0x007F59B0`, `0x007F5994`, `0x007F598C`, and `0x007F5984`. It first calls `AbstractClass__Constructor_Full @ 0x00410170`, then `AbstractClass__AssignUniqueID @ 0x00410230` after those vptr stores and before filling the path array or appending to the Tube vector.

## 3. Core Logic

### 3.1 Recalc predicate and branch order

The verified normal-path branch at `0x0047D8B0..0x0047D945` is:

```text
if final_land == 10
and (signed_i16(cell.tube_index) < 0
     or signed_i16(cell.tube_index) >= g_TubeCount)
then
    test tile_id against the following inclusive ranges, in this order:
        Tunnels_base          .. Tunnels_base + 3
        TrackTunnels_base     .. TrackTunnels_base + 3
        DirtTunnels_base      .. DirtTunnels_base + 3
        DirtTrackTunnels_base .. DirtTrackTunnels_base + 3
    first matching range owns base
    ordinal = tile_id - base
    if ordinal != -1 and operator_new(0x1C4) succeeds:
        TubeClass::Constructor(cell.coord, [2,4,6,0][ordinal])
```

The comparisons are signed x86 comparisons. A nonnegative cell index strictly below the current Tube count suppresses construction. A negative index or an out-of-range nonnegative index admits it. Range overlap is resolved by the first family listed above.

The `ordinal != -1` check is redundant for ordinary matched `base..=base+3` inputs. It does not gate absent keys. `Read_Theater_TileSets_INI` initializes each missing family base to signed `-1`; therefore an absent family can make valid tile IDs `0..=2` match its signed `-1..=2` band, yielding ordinals `1..=3`. Current Rust represents absence as `None` and skips it, which is a conditional-content mismatch.

Zero-count TileSets are not absent. Their cumulative base is still published and the Recalc branch never checks `TilesInSet`. This is active in retail: `TrackTunnels` aliases a later cumulative base in five theaters, both dirt families alias in five theaters, and all four LUNAR families alias tile base `166`.

### 3.2 Branches that cannot construct a shell

The following paths return before `0x0047D8B0`:

- the shared dummy cell;
- invalid/sparse tile or subtile paths;
- the overlay early path selected by the recovered overlay Land/`NoUseTileLandType` rules after its LAT/zone work.

Retail low bridge overlays take that early path: their final land is Road rather than Tunnel. Consequently an authored low bridge overlay Mark cannot create one of these shells and automatic shells must not be synthesized into low-bridge spans.

An admitted ordinary overlay with `NoUseTileLandType=false` can fall through to tile-derived land. `OverlayClass::Mark @ 0x005FC570` calls Recalc synchronously at its common successful tail (`0x005FD200`) and at its procedural stamp callsites. If the resulting cell remains tile-land `10`, the shell can therefore be constructed inline during that Mark, before later authored overlay rows.

### 3.3 Constructor effects and failure boundaries

For a successful `operator_new(0x1C4)`:

1. Initialize the `AbstractClass` base.
2. Copy entry coordinate to both entry and exit.
3. Store direction and zero path length.
4. Install Tube vptrs.
5. Call `AbstractClass__AssignUniqueID`; if the Scenario singleton exists, this advances the shared Scenario cursor exactly once.
6. Fill all 100 path dwords with `-1`.
7. Grow/append to `g_TubeArray`; on success increment `g_TubeCount` and store the pointer.
8. If the entry coordinate is not `(0,0)`, resolve the cell, scan the Tube vector for this pointer, and write the found index as `i16`; if append failed, write `-1`.

The `(0,0)` guard is material. A successfully appended automatic shell at raw coordinate `(0,0)` does not publish an index to that cell. A later qualifying Recalc can allocate another shell and spend another unique ID. Current Rust always publishes an index at `(0,0)`.

Allocation failure at the outer Recalc `operator_new` spends no ID and performs no constructor effect. Dynamic-vector growth failure happens after unique-ID assignment; the Tube is not appended, and a non-origin cell receives `-1`, allowing a later Recalc retry. This resource-failure order is deterministic even though ordinary production is not expected to exhaust the vector.

No RNG, sound, tactical dirtying, radar update, Logic-vector insertion, or Object-list insertion occurs in this constructor slice. The deterministic effects are the scenario ID cursor, Tube object fields, Tube vector, and normally the cell's signed Tube index.

### 3.4 Fresh authored load and final sweep order

Active `ScenarioClass::Full_Init` orders the relevant work as follows:

```text
Read_Map_Section_And_IsoMapPacks
MapClass::ReadTubesINI                         call at 0x00687A0B
ReadMapOverlayPacks                           call at 0x00687A34
  each admitted Overlay Mark may Recalc inline
OverlayDataPack completion and shared drain
first anti-diagonal CellIterator Recalc sweep call at 0x00687A5A
TerrainClass map section                      call at 0x00687A74
Unit / Aircraft / Infantry / Building / Smudge load work
MapClass::InitCellAttributes(0)               call at 0x00687B92
  delete terrain-attached Anims
  clear the terrain-Anim latch
  final anti-diagonal Recalc sweep
  reconstruct wall owner after each final Recalc
```

`MapClass::ReadTubesINI` constructs every source row in linked-list source order. Each allocated row consumes a native ID before its text is parsed, and the reader finally overwrites the entry cell's `+0x116` with the source row index. A valid explicit Tube index therefore suppresses an automatic shell on that cell.

An automatic shell is constructed at the first qualifying Recalc. With no qualifying inline Mark, this is the first post-data anti-diagonal sweep. Once the constructor publishes a valid index, the final post-object sweep does not duplicate it. Within one Recalc, the Tube constructor call at `0x0047D940` precedes level-override application, subtile dimension work, and terrain-attached Anim allocation beginning later in the function; its native ID therefore precedes a same-Recalc terrain Anim ID.

The origin exception and append-failure case are the two reasons a later sweep can retry despite an earlier successful constructor call.

### 3.5 Active retail bands and TMP eligibility

`Read_Theater_TileSets_INI` reads the four `[General]` values as TileSet ordinals, resets all four bases to `-1`, then walks `TileSet####` sections in ordinal order while maintaining the cumulative tile base. Equality with a configured ordinal publishes the current cumulative base before that section's tile count is consumed.

| Theater | `Tunnels` band | `TrackTunnels` band | `DirtTunnels` band | `DirtTrackTunnels` band |
|---|---:|---:|---:|---:|
| TEMPERATE | `566..569` | `572..575` | `783..786` | `783..786` |
| SNOW | `487..490` | `493..496` | `730..733` | `730..733` |
| URBAN | `566..569` | `572..575` | `775..778` | `775..778` |
| NEWURBAN | `566..569` | `572..575` | `775..778` | `775..778` |
| DESERT | `522..525` | `528..531` | `687..690` | `687..690` |
| LUNAR | `166..169` | `166..169` | `166..169` | `166..169` |

The production retail MIX loader census covered 64 unique candidate base IDs, 66 loaded TMP assets including variants, and 731 present subcells. Thirty-six subcells carry TMP terrain byte `5`; `IsometricTileTypeClass__GetSubtileLandType @ 0x00544BE0` indexes `0x008288E4`, whose entry `5` is native land `10`.

Exact positives:

- URBAN `tunnel01.urb:[3,6,9]`, `tunnel02.urb:[1,2,3]`, `tunnel03.urb:[3,6,9]`, `tunnel04.urb:[1,2,3]`;
- URBAN `dtunn01.urb:[3,6,9]`, `dtunn02.urb:[1,2,3]`, `dtunn03.urb:[3,6,9]`, `dtunn04.urb:[1,2,3]`;
- NEWURBAN `tunnel01.ubn:[3,6,9]`, `tunnel02.ubn:[1,2,3]`, `tunnel03.ubn:[3,6,9]`, `tunnel04.ubn:[1,2,3]`.

TEMPERATE, SNOW, DESERT, LUNAR, zero-count aliases, and NEWURBAN's remaining families contribute no positive TMP subcells. This is an asset-level eligibility census, not a count of constructed shells in one map.

## 4. INI Keys

| File / section key | Native read type | Default | Effect |
|---|---|---:|---|
| theater `*MD.INI [General] Tunnels` | signed integer TileSet ordinal | `-1` | publishes `0x00AA1054`, first band checked |
| theater `*MD.INI [General] TrackTunnels` | signed integer TileSet ordinal | `-1` | publishes `0x00ABB108`, second band checked |
| theater `*MD.INI [General] DirtTunnels` | signed integer TileSet ordinal | `-1` | publishes `0x00AA10B4`, third band checked |
| theater `*MD.INI [General] DirtTrackTunnels` | signed integer TileSet ordinal | `-1` | publishes `0x00ABAD2C`, fourth band checked |
| `TileSet#### TilesInSet` | signed integer | `-1` terminates theater load | advances the cumulative base; not consulted by Recalc |
| map `[Tubes]` rows | source-ordered strings | no rows | create explicit Tubes before authored overlays and suppress automatic construction where the entry index is valid |

All six active YR theater INIs declare all four family keys. `TrackTunnels` and one dirt family commonly use `TilesInSet=0`; LUNAR uses zero for all four. Those zero counts do not disable the fixed Recalc bands.

Retail `rulesmd.ini` declares `TunnelSpeed=1` and a `[Tunnel]` speed table, but those locomotion consumers are outside this constructor slice. Retail low bridge overlay definitions use `Land=Road`; they do not bind this automatic Tunnel-land mechanism.

## 5. Integration Points

| Producer / consumer | Address | Integration fact |
|---|---:|---|
| `Read_Theater_TileSets_INI` | `0x00545150` | sole writer family for all four runtime band bases after reset to `-1` |
| `IsometricTileTypeClass__GetSubtileLandType` | `0x00544BE0` | maps TMP terrain byte to final native land; byte `5 -> 10` |
| `CellClass::RecalcAttributes` | `0x0047D2B0` | sole automatic constructor call at `0x0047D940` |
| `TubeClass::Constructor` | `0x00727FD0` | common constructor used by automatic and explicit Tubes |
| `MapClass::ReadTubesINI` | `0x007283C0` | sole active authored-map explicit Tube reader; runs before overlays |
| `OverlayClass::Mark` | `0x005FC570` | successful authored overlay admission can Recalc synchronously before the next row |
| `ScenarioClass::Full_Init` | `0x00686B20` | explicit Tubes, authored overlays, first Recalc sweep, objects, final InitCellAttributes ordering owner |
| `MapClass::InitCellAttributes` | `0x00568BB0` | destroys transient terrain Anims, clears latches, performs final Recalc, then wall-owner repair |
| `RandomMapGenerator::Generate` | call at `0x0059944C` | also reaches `InitCellAttributes`; complete RMG transaction order is outside this slice |
| `MapClass::InitZoneMap` | call at `0x005671E4` | can run a Recalc sweep; an already valid index suppresses duplicates |

`TubeClass::Constructor` has one additional COM/class-factory-style allocation call at `0x006C0156` using `(0,0), direction 0`; it is not the active authored map reader and is excluded from load-order authority.

## 6. Current Rust Implementation Status

### Matching pieces to preserve

- `src/map/tube_facts.rs:39..87` represents automatic shells as `entry == exit`, empty path, and `TubeSource::AutoLowBridge`.
- `src/map/resolved_terrain.rs:3882..3975` uses final native land `10`, no existing index, first-four-tile bands, family order, and direction table `[2,4,6,0]`.
- Zero-count family aliases are intentionally preserved by cumulative `bounds.start`; focused tests cover that native quirk.
- The low-Road negative test at `src/map/resolved_terrain.rs:7555..7565` correctly excludes retail low bridge overlays.
- A*, movement admission, and zone consumers filter automatic zero-step shells out of explicit nonzero Tube traversal. That separation must remain.

### Mismatches and missing ownership

- `ResolvedTerrainGrid::build_inner` calls `seed_explicit_map_tubes` and `build_auto_low_bridge_tubes` once at `src/map/resolved_terrain.rs:3489..3491`, after eager terrain projection. The traversal is rectangular row-major and is not a live Recalc transaction.
- `ResolvedTerrainGrid::recalc_authored_load_cell` at `src/map/resolved_terrain.rs:1393..1638` has no Tube effect. `AuthoredOverlayFinalizer` therefore cannot construct a shell inline during Mark or in the first anti-diagonal sweep.
- Production builds the resolved grid at `src/app/loading/init.rs:2025..2038`, already containing convenience explicit facts and automatic shells, then constructs raw native `[Tubes]` receipts only at `src/app/loading/init.rs:2079..2088`. This reverses the native ownership boundary.
- Raw explicit Tube construction correctly reserves/assigns native IDs before parsing at `src/map/tubes.rs:49..133` and `src/sim/native_identity.rs:89..126`, but `Simulation.native_map_tubes` is disconnected from `ResolvedTerrainGrid::tube_facts`; no non-test consumer joins receipt identity to topology.
- Automatic shells have no `TubeNativeInit`, consume no `NativeUniqueIdCursor` value, and cannot interleave before terrain Anim construction.
- Current Rust skips an absent family (`None`) instead of preserving the native signed `-1` base behavior for tile IDs `0..=2`.
- Current Rust publishes a tube index at coordinate `(0,0)` instead of preserving the constructor's origin guard.
- The headless authored bootstrap and RMG preview also use eager convenience facts without a shared Simulation/native-ID owner. Preview reachability must be decided at its verified lifecycle boundary, not silently inherited from map Fill.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Recalc dummy/invalid/early-return paths | verified | `0x0047D2B0`, cold decompile and assembly | none for this slice |
| Recalc final-land and signed index predicate | verified | `0x0047D8B0..0x0047D8D4` | none |
| four fixed bands, order, and ordinal | verified | `0x0047D8D4..0x0047D924` | none |
| allocation and automatic constructor call | verified | `0x0047D924..0x0047D945` | none |
| direction table | verified | raw memory `0x0081CC20` | none |
| TMP byte-to-land table | verified | `0x00544BE0`, raw memory `0x008288E4` | none |
| Tube base/ID/path/vector/cell-index constructor effects | verified | `0x00727FD0`, `0x00410170`, `0x00410230` | none |
| `(0,0)` no-publication exception | verified | `0x0072807E..0x007280BE` | none |
| vector-grow failure after ID | verified | `0x00728029..0x0072807E` | none |
| explicit Tube read/parse/index order | verified | `0x007283C0`, sole Full_Init xref `0x00687A0B` | none |
| authored Overlay Mark inline Recalc | verified | `0x005FC570`, common tail and procedural callsites | none |
| first/final authored sweeps and relative object/Anim order | verified | `0x00686B20`, `0x00568BB0`, Tube call before later Recalc Anim region | none |
| four theater key writers/defaults | verified | `0x00545150`, writer xrefs for four globals | none |
| six retail cumulative bands | verified | retail `*MD.INI` cumulative parse; production TheaterData loader | none |
| retail TMP eligibility | verified | retail MIX production loader; ignored oracle passed `1/1`; exact positive identity set | none |
| stock winning-map final-cell prevalence | deferred | asset eligibility does not decode winning map placement/final overlays | run a 385-payload final-state tile/subtile census |
| complete runtime mutation caller census | deferred | Recalc xrefs prove the intrinsic branch but runtime mutation ownership is outside fresh-load scope | trace only if runtime Tube creation becomes an owning-row requirement |
| Tube save/load/destruction/COM lifecycle | deferred | constructor and CRC were bounded; persistence is not needed to place the load constructor | dedicated GSI-04.15 persistence slice |
| current Rust producer/identity/consumer ownership | verified | exhaustive source-reference scan listed in Section 6 | implementation required |
| complete RMG/preview identity prefix | touched-not-exhausted | `InitCellAttributes` RMG xref and current Rust callers | close in the generated/preview lifecycle transaction, not by authored-load guesswork |

The deferred pile is three bounded contexts out of twenty ledger areas (15%). None changes the automatic constructor semantics or fresh-authored-load handoff proved here.

### Exhaustion gate record

The zero-add pass re-decompiled the primary function and relevant top-level callees (`0x0047D2B0`, `0x00727FD0`, `0x00544BE0`, `0x00410170`, `0x00410230`, `0x007283C0`, `0x00568BB0`, `0x00686B20`, `0x005FC570`, `0x00545150`). The first pass promoted the already-seeded absent-base and origin-coordinate questions; both were resolved. A second cold pass over Recalc, the Tube constructor, and theater loading added zero questions.

Adversarial questions answered from evidence:

1. **What if two family bands overlap?** The first match in Tunnels/Track/Dirt/DirtTrack order owns the base; exact aliases produce the same ordinal and direction.
2. **What if the family key is absent?** Its signed `-1` base remains live and can match valid tile IDs `0..=2`; native does not use an Option-style absence gate.
3. **What if a valid explicit Tube already owns the cell?** Any nonnegative index below `g_TubeCount` suppresses automatic construction; a stale/out-of-range index does not.
4. **What if the cell is `(0,0)`?** The Tube appends and consumes an ID, but the constructor skips cell-index publication, so later Recalc can duplicate it.
5. **What if Tube-vector growth fails after allocation?** The constructor has already consumed an ID; it writes `-1` to a non-origin cell and a later Recalc may retry.
6. **What if first and final authored sweeps both qualify?** Normal non-origin append success publishes a valid index during the first qualifying visit, so the final sweep does not duplicate it.

Cold spot checks separately re-read the signed predicate/range assembly at `0x0047D8B0..0x0047D945` and the constructor's ID/vector/origin assembly at `0x00728017..0x007280BE`.

## 8. Open Questions — Final State of the Investigation Log

- `[RESOLVED] AT-01 — Which Recalc branches reach or bypass automatic construction? → Only the normal valid-tile path reaches it; dummy, sparse/invalid, and qualifying overlay early paths return first.` (evidence: `0x0047D2B0`, automatic block `0x0047D8B0..0x0047D945`)
- `[RESOLVED] AT-02 — What are the exact ranges, order, overlap rule, and invalid-base behavior? → Four signed inclusive base..base+3 ranges are checked Tunnels, TrackTunnels, DirtTunnels, DirtTrackTunnels; first match wins; signed -1 remains a real base and can match tile 0..2.` (evidence: `0x0047D8D4..0x0047D924`, `0x00545150`)
- `[RESOLVED] AT-03 — How are LandType and existing Tube index bounded? → Exact land 10; signed i16 index negative or sign-extended index >= i32 Tube count admits construction.` (evidence: `0x0047D8B0..0x0047D8D4`)
- `[RESOLVED] AT-04 — What are allocation, dynamic-vector, count, and i16 edge semantics? → Outer allocation failure performs nothing; vector growth happens after ID assignment; success increments i32 count and final cell index is the found array ordinal truncated to i16.` (evidence: `0x0047D924..0x0047D945`, `0x00728029..0x007280BE`)
- `[RESOLVED] AT-05 — What base constructor, registries, and native-ID effects occur? → Abstract base initialization, Tube vptrs, one Scenario ID, path initialization, Tube-vector append, then cell-index publication.` (evidence: `0x00727FD0`, `0x00410170`, `0x00410230`)
- `[RESOLVED] AT-06 — Does construction consume RNG/audio/dirty/Logic/Object effects? → No; the bounded constructor effects are scenario ID, Tube fields/vector, and cell index.` (evidence: complete `0x00727FD0` body and callees)
- `[RESOLVED] AT-07 — What is the `+0x116` write order and `(0,0)` behavior? → Publication follows append and pointer scan; raw origin skips publication entirely.` (evidence: `0x0072807E..0x007280BE`)
- `[RESOLVED] AT-08 — Which fresh-load calls can construct? → Successful admitted Overlay Mark Recalc, the first post-data anti-diagonal sweep, and the final InitCellAttributes sweep; first valid publication suppresses later visits.` (evidence: `0x005FC570`, `0x00686B20`, `0x00568BB0`)
- `[RESOLVED] AT-09 — How do explicit valid or out-of-range Tubes interact? → Explicit rows construct before overlays and publish their source row index; valid index suppresses auto, negative/stale/out-of-range admits it.` (evidence: `0x007283C0`, `0x00687A0B`, `0x0047D8BD..0x0047D8D4`)
- `[RESOLVED] AT-10 — Which runtime globals and family names bind each theater? → The four `[General]` keys map one-to-one to globals `0x00AA1054`, `0x00ABB108`, `0x00AA10B4`, `0x00ABAD2C`; all six active MD theater INIs declare them.` (evidence: `0x00545150`, retail theater INIs)
- `[RESOLVED] AT-11 — Do retail TMPs activate final land 10 in the fixed bands? → Yes: exactly 36 URBAN/NEWURBAN TMP/subtile identities; TMP byte 5 maps to land 10.` (evidence: `0x00544BE0`, `0x008288E4`, passing retail corpus oracle)
- `[DEFERRED] AT-12 — Which winning shipped maps place a qualifying final cell?` (category: `requires-different-system-context`; reason: the verified asset census does not decode all winning map IsoMapPack/LAT/overlay final states; next-step-if-pursued: add/run a read-only 385-payload final-state placement census keyed by theater, final tile, subtile, overlay branch, and explicit Tube index)
- `[RESOLVED] AT-13 — What immediate downstream order is load-bearing? → Tube construction and ID assignment precede same-Recalc terrain Anim construction; automatic shells remain zero-step facts and are not explicit Tube traversal routes.` (evidence: `0x0047D940` before later Anim region; constructor/producer reports)
- `[RESOLVED] AT-14 — Which inherited/TS-era claims are stale? → Automatic shells are not stock low-overlay topology; the correct callsite is 0x0047D940, and the mechanism is not a one-shot late map-init batch.` (evidence: Recalc/Mark/retail rules and stale-doc list below)
- `[RESOLVED] AT-15 — Does Rust's eager helper match native ownership? → No: it batches row-major during Fill, has no Recalc or native-ID integration, and would suppress a later correct constructor if left in authored Fill.` (evidence: `src/map/resolved_terrain.rs:3489..3491`, `:3882..3975`)
- `[RESOLVED] AT-16 — What is explicit Tube seeding and identity order? → Source-order constructor IDs precede parse and all authored overlay/automatic/Anim constructors; current Rust topology and native receipt are disconnected.` (evidence: `0x007283C0`, `src/map/tubes.rs:49..133`, `src/app/loading/init.rs:2025..2088`)
- `[RESOLVED] AT-17 — Which edge cases affect an exact implementation? → zero-count aliases, absent signed -1 bases, overlapping first-match bands, origin no-publication, stale indexes, outer allocation failure, and vector-growth failure after ID.` (evidence: primary/constructor/theater bodies)
- `[DEFERRED] AT-18 — What complete teardown/save/restore lifecycle owns automatic shells?` (category: `out-of-scope`; reason: constructor placement and fresh-load order do not require the full persistence graph; next-step-if-pursued: trace Tube destructor, save/load factory, swizzle, CRC, and Simulation snapshot ownership as the GSI-04.15 persistence transaction)
- `[RESOLVED] AT-19 — What acceptance vectors are required? → inline qualifying Mark, no-overlay anti-diagonal creation, explicit suppression, final no-duplicate, same-Recalc Tube-before-Anim ID, origin duplicate, absent/zero-count/overlap families, and the exact 36-retail-identity corpus.` (evidence: Sections 3 and 10)
- `[DEFERRED] AT-20 — What exact generated-map and preview prefix owns automatic shells?` (category: `requires-different-system-context`; reason: RMG and preview have distinct lifecycle/cursor owners beyond the authored load transaction; next-step-if-pursued: close them in the generated lifecycle and preview identity transactions using the verified intrinsic Recalc constructor rule)

## 9. Visual/UI Composition Ledger

Omitted: this slice has no independent visual/UI composition surface. Any visible tunnel-floor rendering is owned by the ordinary TMP renderer, not by the zero-step Tube shell constructor.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Automatic construction occurs synchronously at the first qualifying Recalc | `0x0047D2B0`, Mark and Full_Init call order | mismatch | `ResolvedTerrainGrid::recalc_authored_load_cell`, `AuthoredOverlayFinalizer`, production load owner | surface a Tube-construction effect from live Recalc and execute it inline through the Simulation-owned host | a qualifying no-overlay cell constructs in anti-diagonal order; a qualifying ordinary Mark constructs before the next overlay row | do not retain authored Fill's eager batch, or it will prefill the index and suppress the native transaction |
| Explicit Tube identities precede all automatic shells and share one native cursor | `0x007283C0`, `0x00687A0B`, `0x00727FD0` | missing join | `src/map/tubes.rs`, `Simulation.native_map_tubes`, `ResolvedTerrainGrid::tube_facts`, `src/app/loading/init.rs` | bind the explicit native receipt to the gameplay Tube registry before authored overlays, then append automatic shells into the same ordered registry/cursor | two explicit source rows followed by one auto shell and one terrain Anim yield strictly ordered IDs in that sequence | do not use convenience-parsed/filtered Tube facts as native identity authority |
| Tube constructor ID/append/cell-index effects precede terrain Anim creation | `0x00728017..0x007280BE`, `0x0047D940` | missing | authored production host and Recalc effect/resume boundary | construct/register Tube, publish cell index when allowed, then resume later Recalc effects including Anim | one qualifying cell with a terrain Anim root spends Tube ID first and Anim ID second | do not batch effects after the sweep or sort them by type |
| Valid index suppresses later sweeps; origin does not publish | `0x0047D8BD..0x0047D8D4`, `0x0072807E..0x007280BE` | mismatch at origin | cell Tube index and Tube registry mutation | preserve signed validity test and raw `(0,0)` no-publication exception | ordinary cell constructs once across first/final sweep; origin constructs once per qualifying sweep | do not normalize origin into the ordinary publication path |
| Missing family base is signed `-1`; zero-count family retains its cumulative base | `0x00545150`, signed range assembly | partial mismatch | TheaterData representation and automatic direction lookup | represent the native runtime base value, not only a semantic Option, for the Recalc predicate | absent first family admits land-10 tile IDs 0..2 with directions 4/6/0; zero-count base still covers four IDs | do not gate on `TilesInSet > 0` |
| Retail eligibility is exactly 36 identities | retail MIX oracle and `0x008288E4` | oracle exists; producer lifecycle wrong | `src/map/theater_tests.rs`, authored Recalc integration tests | retain the exact corpus oracle and add construction-order coverage using those identities or exact synthetic equivalents | ignored retail oracle remains `1 passed` and exact positive set unchanged | do not relabel these as low bridge overlay cells |
| Resource failure has two different ID boundaries | `0x0047D924..0x0047D945`, `0x00728029..0x007280BE` | unchecked/missing | production host failure contract | outer allocation rejection spends nothing; post-ID registry rejection leaves index invalid for retry | injectable host fixtures distinguish pre-constructor failure from post-ID append failure | do not collapse all failure into one pre-ID `Result` |
| Generated/RMG Recalc follows the same intrinsic predicate but has another lifecycle owner | `0x0059944C`, Recalc body | eager approximation | RMG generated lifecycle and preview | reuse the verified intrinsic rule only after its phase/cursor owner is closed | generated qualifying tile constructs at its real Recalc phase without duplicating authored-prefix behavior | do not solve preview by borrowing the live match Simulation cursor |

### Stale Docs / Follow-up Docs

- `docs/research/bridges/00-system-models/BRIDGE_REMAINING_GAPS_FOLLOWUP_GHIDRA_REPORT.md`: replace “stock low bridge behavior relies on auto shell/predicate data” with “stock low Road overlays do not reach automatic Tube construction; automatic shells are a separate Tunnel-land tile-family mechanism.”
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`: retain constructor/layout facts, but replace ownership wording that equates automatic shells with retail low bridge overlays.
- `docs/research/bridges/01-assets-map-load-overlay/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`: replace “Rail/subway bridgeheads” and one-shot-map-init wording with the four exact Tunnel family keys and first-qualifying-Recalc timing.
- `docs/research/pathfinding/FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md`: its exhaustive claim is incomplete for native-ID parity until it includes the automatic Tube block at `0x0047D8B0..0x0047D945`.
- `docs/research/bridges/05-damage-collapse-repair-cabhut/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`: replace the stale Tube callsite `0x0047DA35` with `0x0047D940`.
- `docs/research/bridges/00-system-models/ACTIVE_RETAIL_BRIDGE_COVERAGE_REINVESTIGATION_GHIDRA_REPORT.md`: OQ-03/OQ-21 asset-classification portion is now resolved by the exact 36-identity census; winning shipped-map placement frequency remains open.

## 11. Ghidra Annotation Candidates

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|---|---|---|---|---|---|
| `0x0047D2B0` | plate comment omits automatic Tube construction | add exact four-band predicate and Tube-before-Anim constructor order | comment | body and call at `0x0047D940` | deferred; no sync requested |
| `0x00AA1054` | `DAT_00AA1054` | `g_Tunnels_TileSetBase` | rename/comment | `[General] Tunnels` read plus sole writer/read xrefs | deferred; no sync requested |
| `0x00ABB108` | `DAT_00ABB108` | `g_TrackTunnels_TileSetBase` | rename/comment | `[General] TrackTunnels` read plus sole writer/read xrefs | deferred; no sync requested |
| `0x00AA10B4` | `DAT_00AA10B4` | `g_DirtTunnels_TileSetBase` | rename/comment | `[General] DirtTunnels` read plus sole writer/read xrefs | deferred; no sync requested |
| `0x00ABAD2C` | `DAT_00ABAD2C` | `g_DirtTrackTunnels_TileSetBase` | rename/comment | `[General] DirtTrackTunnels` read plus sole writer/read xrefs | deferred; no sync requested |
| `0x0081CC20` | unlabeled direction data | `g_AutoTubeDirectionByOrdinal` as four `i32` values | label/comment | indexed only by the verified ordinal at `0x0047D935`; bytes `2,4,6,0` | deferred; no sync requested |

## Sources

- Ghidra decompilation/disassembly: `0x0047D2B0`, `0x0047D8B0..0x0047D945`, `0x00727FD0`, `0x00410170`, `0x00410230`, `0x007283C0`, `0x00544BE0`, `0x00545150`, `0x005FC570`, `0x00686B20`, `0x00568BB0`, `0x00684C30`.
- Ghidra raw data/xrefs: `0x0081CC20`, `0x008288E4`, `0x00AA1054`, `0x00ABB108`, `0x00AA10B4`, `0x00ABAD2C`, constructor and load call xrefs.
- Retail INIs: `ini/temperatmd.ini`, `ini/snowmd.ini`, `ini/urbanmd.ini`, `ini/urbannmd.ini`, `ini/desertmd.ini`, `ini/lunarmd.ini`, `ini/rulesmd.ini`, `ini/artmd.ini`.
- Retail assets: production `AssetManager` mount rooted at `C:\Users\enok\Documents\Command and Conquer Red Alert II`; 64 unique candidate IDs, 66 loaded TMPs, 731 present subcells, 36 positives.
- Literal retail oracle already completed without Cargo: `vera20k-5dbe854afac4ab10.exe active_retail_automatic_shell_corpus_is_exact --ignored --nocapture --test-threads=1` → `1 passed; 0 failed; 7931 filtered out`.
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`.
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_DOC_VERIFICATION.md`.
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`.
- `docs/research/bridges/00-system-models/ACTIVE_RETAIL_BRIDGE_COVERAGE_REINVESTIGATION_GHIDRA_REPORT.md`.
- `docs/research/bridges/00-system-models/BRIDGE_REMAINING_GAPS_FOLLOWUP_GHIDRA_REPORT.md`.
- `docs/research/bridges/01-assets-map-load-overlay/FULL_INIT_AND_PREVIEW_NATIVE_ID_PREFIX_REINVESTIGATION_GHIDRA_REPORT.md`.
- `docs/research/bridges/01-assets-map-load-overlay/AUTHORED_OVERLAYPACK_INLINE_TRANSACTION_REINVESTIGATION_GHIDRA_REPORT.md`.
- `docs/research/bridges/01-assets-map-load-overlay/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`.
- `docs/research/pathfinding/FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md`.
- Rust read-only owner scan: `src/map/resolved_terrain.rs`, `src/map/tube_facts.rs`, `src/map/tubes.rs`, `src/map/authored_overlay.rs`, `src/map/theater.rs`, `src/map/theater_tests.rs`, `src/sim/native_identity.rs`, `src/sim/world/mod.rs`, `src/app/loading/init.rs`, `src/headless_scenario.rs`, `src/app/shell_random_map.rs`.
