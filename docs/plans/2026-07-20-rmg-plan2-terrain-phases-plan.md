# RMG Plan 2 — Terrain Generation Phases Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained but
> cites the research doc that owns its formulas — read the cited section before
> implementing. Plan 1 (2026-07-20-random-map-generator-plan.md) is COMPLETE and
> this plan builds directly on its modules.

**Goal:** Implement the base generation pipeline (map types 0–2, all theaters):
water → finalize → regions → green spread → starts → tech buildings → tiberium →
hills → LAT patches/trees/rocks → emit, producing a playable `GeneratedMap` whose
draw stream and formulas match gamemd bit-for-bit where research is exact.

**Architecture:** All new code lands in `src/map/rmg/` (map layer, pre-sim; the
`sim/`-independence test in `emit.rs` already guards this). Phases are plain
functions over `RmgGrid` + `RmgScratch` + `RmgRng`, committed in `STAGE_ORDER`.
`generate()` grows a deps struct (theater + rules); the `MapLoadInitial` seam is
already wired from Plan 1 Task 13.

**Design Doc:** docs/plans/2026-07-19-random-map-generator-design.md (ledger items 15–53c, 65–66)

---

## Grounding Summary

- **Formula-exact docs exist for:** green spread, hills (full corner engine +
  19-entry ramp table), LAT patches/trees/rocks (all constants asm-verified),
  starts (gather/selector/6×6 gate/flood-fill), tiberium (recheck doc GREEN,
  every FP constant read from memory), water finalizer `0x0059C630`, region
  partition (role-level + audit fixes), LAT fixup (`0x0047CA80`, HIGH).
- **NOT decoded (blocks bit-exact water):** `0x005ADA40` (island shape init),
  `0x0059A8F0` (partition grid builder), `0x0059BBC0` (flood-fill blob placer),
  continental water-fraction constants in `0x0059AFA0`. Also open: `0x00599650`
  genW/genH player-count term (plan-1 residual), MapClass cell-iterator
  traversal order, region BFS queue discipline, `0x005A45E0` TREE00-miss
  branch, land-type-3 identity, `[Lighting]` scenario-field mapping,
  OrePatchLamps consumer. → **Task 1 closes all of these before dependent tasks.**
- **Repo pattern:** Plan-1 modules (`rng.rs` draw-exact rejection loops,
  `x87.rs` TruncF64 chains, `scratch.rs` diamond test, `settings.rs`
  carry-defaults). Theater `[General]` key capture follows the existing
  `TheaterCliffRanges` pattern in `src/map/theater.rs:162`.
- **INI:** theater INIs `[General]` ClearTile/RampBase/RoughTile/SandTile/
  GreenTile/ClearToSandLat/ClearToGreenLat/ClearToRoughLat/ClearToPaveLat/
  WaterSet/ShorePieces (temperatmd.ini:46–63); rulesmd.ini:3082
  `NeutralTechBuildings`; `[OverlayTypes]` 0-based positions (ore 102, gem 27,
  SROCK 168–172, TROCK 173–177); `[TerrainTypes]` TREE01–TREE25;
  `ini/rmgmd.ini` (extracted 2026-07-20) lamp + ambient vectors.
- **Key reconciliation (supersedes RMG_START_GENERATION doc):** `DAT_00ABE028`
  IS `MapSeed+0x50` (0xABDFD8+0x50) — the start quota *is* clamped NumPlayers,
  not a separate constant-4 global (RMG_TIBERIUM_RECHECK §6 address arithmetic +
  xref scan). The "default 4" writes are dialog defaults, not the launch value.

## Key Technical Decisions

- **One RE task up front (Task 1), then pure implementation.** The water shape
  internals are the only undecoded stage in scope; everything else has
  formula-level docs. — **Confidence:** high. **Source:** RMG_WATER_SEED §11,
  RMG_TERRAIN_SHAPING_CORE §7/§8.
- **`RmgGrid` models the CellClass fields the phases touch** (tile, sub_tile,
  level, overlay, density, occupied, start-marker) — not the full CellClass.
  Attributes (land type, clear/shore/water predicates) are derived on demand
  from current tiles, which matches post-RecalcAttributes state at every
  consumer because native recalcs immediately before each consumer stage. —
  **Confidence:** medium (equivalence argument, not bit-proof; flagged for
  review). **Source:** stage list RMG_TERRAIN_SHAPING_CORE §5.
- **Theater tile identities live on `TheaterData`** as a new parsed struct,
  following `TheaterCliffRanges`. Resolved to flat tile indices via
  `TilesetLookup::bounds()`. — **Confidence:** high. **Source:**
  src/map/theater.rs:162, RMG_TERRAIN_SHAPING_CORE §2.4.
- **Tech buildings include the maptype-2 per-region path** (`0x00595400`,
  decoded in RMG_MODE34 doc). Deviation from the design's P3 phasing —
  justified: map type 2 is in Plan-2 scope and its tech path is already
  decoded; leaving it out ships wrong type-2 maps. — **Confidence:** high.
  **Source:** design ledger item 50; RMG_MODE34_WATER_BRIDGES_TECH doc.
- **Region objects are a `Vec<RmgRegion>` with explicit id counter**, not
  pointer soup; iteration in creation order matches the native array walk. —
  **Confidence:** high. **Source:** RMG_REGION_PARTITION layout tables.
- **All phase FP goes through `TruncF64`/`approx_sqrt`/`Gaussian`/`ftol`**
  from plan-1 `x87.rs`; constants by bit pattern (design items 65–66). —
  **Confidence:** high (instrument-verified in Plan 1).
- **Full-map parity status:** phases are certified only at formula/unit level
  plus determinism; full-map byte-golden vs gamemd stays
  **UNVERIFIED-pending-instrument** (live gamemd cell-grid capture — not in
  this plan). — **Confidence:** high that this is the honest label.

## Open Questions

### Resolved During Planning
- `DAT_00ABE028` identity → MapSeed+0x50 (NumPlayers). See Grounding.
- `g_DirectionOffsets` values → N(0,−1) NE(1,−1) E(1,0) SE(1,1) S(0,1)
  SW(−1,1) W(−1,0) NW(−1,−1) (verified `0x0049F2F0` decompile, 3 docs agree).
- LAT fixup algorithm → fully decoded in
  docs/research/LAT_GROUPS_AND_SLOPE_FIXUP_GHIDRA_REPORT.md (HIGH).
- RMGMD.INI retail values → extracted to `ini/rmgmd.ini` 2026-07-20.

### Deferred to Task 1 (Ghidra)
- Water shape internals (`0x005ADA40`, `0x0059A8F0`, `0x0059BBC0`, constants
  in `0x0059AFA0`, full draw sequences of `0x0059AD10`/`0x0059B200`).
- `0x00599650`: genW/genH formula (ftol inputs), diamond-bound + scratch-origin
  values, `[Lighting]`/scenario-lighting field mapping from RMGMD vectors.
- MapClass cell-iterator traversal order (draw-order-critical everywhere).
- Region BFS queue discipline + neighbor order in `0x0058C800`/`0x0058E740`;
  scratch `+0x45` vs `+0x4B` water-flag reconciliation (ledger 23).
- `0x005A45E0` TREE00 lookup-miss branch (does a failed find consume the size?).
- Cell land-type (+0xEC) derivation and the identity of excluded value 3.
- OrePatchLamps vector consumer (which stage, if any in scope).

### Deferred to Implementation
- Exact FP op order inside the hills walk — terrain doc says take it from asm
  when implementing (`0x005A2F50`); the plan task requires reading the disasm.
- 6×6-gate tile-range globals for non-TEMPERATE theaters (road/pave sets exist
  per theater; resolve from theater INI at load).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/map/rmg/tiles.rs` | theater tile identities + tile predicates (clear/water-ish/shore/LAT membership) |
| Create | `src/map/rmg/grid.rs` | `RmgGrid` cell model (tile/sub_tile/level/overlay/density/occupied/flags) + iterator + land-type |
| Create | `src/map/rmg/region.rs` | `RmgRegion` objects, id counter, flood-fill constructor, BFS expander, fallback seeder |
| Create | `src/map/rmg/phases/mod.rs` | phase module root + shared helpers (uniform helpers over grid) |
| Create | `src/map/rmg/phases/water.rs` | all-water fill, shape dispatch 0/1/2, isolated-water removal, shore-to-green |
| Create | `src/map/rmg/phases/water_finalize.rs` | `0x0059C630` variant bands + 2×2 anchor placement |
| Create | `src/map/rmg/phases/green_spread.rs` | `0x0059B740` |
| Create | `src/map/rmg/phases/starts.rs` | quota buckets, gather, farthest-point selector (+ TiberiumLayout slots), 6×6 gate, per-start flood-fill |
| Create | `src/map/rmg/phases/tech_buildings.rs` | `0x005A95B0` + maptype-2 `0x00595400` |
| Create | `src/map/rmg/phases/tiberium.rs` | `0x005A23A0` driver + `0x005A28C0` placer + gem pass |
| Create | `src/map/rmg/phases/hills.rs` | `0x005A35F0` pipeline: seed, walk, corner engine, finalize, quad cleanup |
| Create | `src/map/rmg/phases/lat_patches.rs` | `0x005A38C0`/`0x005A3AE0`/`0x005A4280` + patch placer `0x005A4B60` |
| Create | `src/map/rmg/phases/trees.rs` | tree count + scatterer `0x005A45E0` |
| Create | `src/map/rmg/phases/rocks.rs` | TEMPERATE rock overlays |
| Create | `src/map/rmg/lat_fixup.rs` | `ApplyLAT_and_SlopeFixup` port (from LAT_GROUPS doc) |
| Modify | `src/map/rmg/mod.rs` | deps struct, phase wiring in `generate()`, real dimensions |
| Modify | `src/map/rmg/emit.rs` | populate cells/overlays/terrain objects/waypoints/[Lighting]/[Basic] |
| Modify | `src/map/rmg/settings.rs` | lamp lists + ambient/light vectors from RMGMD.INI |
| Modify | `src/map/theater.rs` | parse + store the RMG tile-identity `[General]` keys |

## Interface Changes

- `generate(options, settings, deps)` — deps grows from `Option<&TheaterData>`
  to a `RmgDeps<'_> { theater: &TheaterData, theater_general: &RmgTileKeys,
  rules: &Ruleset }` struct. Sole caller: the `.SED` branch in
  `src/app_init.rs` (Plan-1 Task 13) — update it in the same task.
- `GeneratedMap` unchanged (map_file, start_waypoints, stages_run).
- `TheaterData` gains one field (additive; no existing consumer breaks).

## Risk Areas

- **Draw-stream drift** is the whole risk profile: any wrong rejection loop,
  swapped probability order, or missed defensive draw desynchronizes every
  later phase. Mitigation: per-phase draw-count assertions in tests, exact
  op-order from disasm for FP chains.
- **Iterator order**: phases draw per-cell in iterator order; a wrong traversal
  order silently reorders the entire stream (Task 1 resolves; every phase task
  depends on it).
- **`app_init.rs` seam**: modified by Plan 1; parallel sessions also touch
  app files — re-read before editing (Task 14).
- The two pre-existing failing tests (`slice6_retask_tests`,
  `global_parity_harness_tests`) belong to a parallel session — do not touch.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1→4 | genW/genH dimension formula | wrong size changes every map | disasm `0x00599650` ftol inputs |
| 1→all | cell-iterator traversal order | reorders entire draw stream | disasm `0x00578350/290` |
| 5 | water shape draw sequences | first consumer of the stream; everything downstream shifts | Task-1 doc + draw-count tests |
| 6 | finalizer band math (mod 11 / 242 edge 240→7, 241→6 / mod 201 6-band) | visible water texture every map | unit tests vs doc constants |
| 7 | region BFS pass count `4+(cnt>8000)+!mode34`, enqueue gate clear-OR-greenLAT | region ids feed starts+tiberium | unit test + audit facts |
| 8 | green spread `min(len/3,1000)`, shift-down removal | visible shoreline vegetation | unit test |
| 9 | selector +20 cross-region bonus; slot formula `(TibLayout%×12/NumPlayers+2)×quota`; 6×6 gate; 2-draw candidate loop | start positions + tiberium slot layout | unit tests; formulas from disasm-cited doc |
| 10 | foundation gate (level-equal, land≠3, water flag) | tech buildings visible every non-type-0 map | unit test on crafted grid |
| 11 | two-stage truncation `trunc(trunc(lerp)×mult)`; gate chain order; TIBTRE01–03 only; gem pass size `trunc(Δ×15)+500` | ore layout every map | vector test over tib∈[0,100] |
| 12 | Ruggedness<10 skip; ±0xF corner steps; undo-on-locked; 19-pattern table; quad cleanup level+1 | hill shapes every rugged map | table-equality + fixture tests |
| 13 | rough→sand→green fresh-draw order; patch means [20,40]; jitter [0,5); TEMPERATE-only rocks/sand/green | terrain texture every map | unit tests + draw-count |
| 14 | LAT mask bit order N/E/S/W = bits 0–3 | tile transitions everywhere | LAT doc table tests |
| 15 | `[Lighting]` values from Time + RMGMD vectors | global map tint | Task-1 mapping + unit test |

---

## Tasks

### Task 1: RE follow-up — close every open bit-exactness item (Ghidra, BLOCKING) — DONE 2026-07-20

Executed as three parallel agents + parent decodes + a shore-tiler follow-up
agent. All items closed; findings in RMG_WATER_SEED §12/§13,
SKIRMISH_RANDOM_MAP_GENERATOR §10, RMG_REGION_PARTITION appendix,
RMG_TERRAIN_SHAPING_CORE appendix; AUDIT_LOG lines per doc. Headlines beyond
the plan's expectations: cell iteration is a diamond anti-diagonal scan;
`Get_CellClass` has a shared border-cell fallback (aliasing is observable);
water blobs are LAND carved from an all-water map; the shore tiler draws one
uniform(0,5) per iterated cell in its stamp passes; the finalizer/region
class tests were mislabeled in older docs (corrected); TREE00 unreachable;
land 3 = Rock; OrePatchLamps dead. Tasks 2/3/4/8 implemented against these
findings (commits 0dbdbbe5, c5f63d77, 75b4d55c, 4d8ab6a0).

**Why:** The only stage without formula-level research is the water shape set;
several cross-cutting orderings (iterator, BFS queue) silently shape the whole
draw stream. No implementation task that consumes these may start first.

**This is a research task — no Rust code.** Use `/re-investigate`-style
verification discipline; extend existing docs rather than fork new ones.

**Step 1: Water shape internals** (extend RMG_WATER_SEED doc):
- `0x005ADA40` (called by `0x0059AD10` archipelago init) — full decode.
- `0x0059A8F0` partition grid builder — full decode incl. every draw.
- `0x0059BBC0` flood-fill blob placer — full decode: seed pick, growth rule,
  per-cell/per-neighbor draw counts, termination.
- `0x0059AD10` / `0x0059AFA0` / `0x0059B200` — complete the remaining
  formulas: archipelago center-offset loop; continental water-fraction
  constants (`const_min`/`const_max` in `(max-min)*(1-Water*0.01)+min`) and the
  Manhattan nearest-unvisited scan; islands-in-sea split geometry and the
  double threshold on `+0x4C`.

**Step 2: `0x00599650` map-prep internals** (extend the 00598960 or terrain doc):
- genW/genH formula — read the disasm around `0x00599665..0x005996D7` for the
  ftol inputs (player-count term × dimension scale).
- The values written to the diamond bounds `DAT_00ABED04/08`, scratch origin
  `DAT_0087F90C/910`, corner-grid dims `DAT_0087F914/918`, and view-region
  globals `DAT_0087F8E4..F0` — as formulas of genW/genH.
- Tail lighting: which `[Lighting]`-equivalent scenario fields
  (`ScenarioClass+0x3528/3534/3538/353C/3544`) get which
  `RMGLevelLightSettings`/Ambient vector values, with scaling.
- OrePatchLamps consumer: xref `MapSeed+0x2C4`/`+0x2E0` readers; record which
  stage consumes them (or that none in Plan-2 scope does).

**Step 3: Orderings**:
- `MapClass__CellIterator_Init 0x00578350` / `_Next 0x00578290` — exact
  traversal order (linear array order? which stride? skips?).
- `0x0058C800` flood-fill: queue discipline (FIFO/LIFO/heap), neighbor
  iteration order, exact enqueue conditions, and the rejection-sampled seed
  pick (audit slot 6). `0x0058E740`: same for its per-pass queue.
- Scratch water flag: reconcile `+0x45` (terrain/tiberium docs) vs `+0x4B`
  (region doc) — decompile the writers/readers and record which byte each
  phase touches (they may be two distinct flags; ledger 23).
- `0x005A45E0`: the branch when `TerrainTypeClass__Find_By_Name_Index` misses
  (TREE00) — is the tree counted, skipped, or does placement abort?
- Cell land type: where `+0xEC` is written (RecalcAttributes path), the
  tile→land mapping, and which land value `3` is.

**Step 4:** Update the docs with inline Ghidra citations; add an AUDIT_LOG line;
list every resolved item in the doc's Open Questions final state.

**Verify:** every item above has a doc section with a cited Ghidra call; no
UNVERIFIED item remains that a Plan-2 task consumes (TREE00 may stay YELLOW if
the binary genuinely defers to runtime — then implement the miss as "no tree
placed, counted per whatever the branch shows" and mark it).

---

### Task 2: Theater tile identities (`tiles.rs` + `theater.rs` capture)

**Why:** Every phase gates on tile predicates; this is the substrate.

**Files:** Create `src/map/rmg/tiles.rs`; Modify `src/map/theater.rs` (new
parsed struct + field on `TheaterData`), `src/map/rmg/mod.rs` (re-export).

**Pattern:** `TheaterCliffRanges` (src/map/theater.rs:162) — numeric
`[General]` keys parsed at theater load.

**Step 1:** In `theater.rs`, add (near `TheaterCliffRanges`):

```rust
/// Theater `[General]` tileset numbers the random-map generator resolves to
/// flat tile indices. Missing keys default to 0 like the original's globals.
#[derive(Debug, Clone, Copy, Default)]
pub struct RmgTileKeys {
    // All Option<u16> resolved flat starts — a missing key is None (the
    // original's globals default to -1, per the LAT doc's 0x005455B5 table;
    // the plan's earlier "default 0" claim was wrong).
    pub clear_tile: Option<u16>,
    pub ramp_base: Option<u16>,
    pub rough_tile: Option<u16>,
    pub sand_tile: Option<u16>,
    pub green_tile: Option<u16>,
    pub clear_to_rough_lat: Option<u16>,
    pub clear_to_sand_lat: Option<u16>,
    pub clear_to_green_lat: Option<u16>,
    pub clear_to_pave_lat: Option<u16>,
    pub pave_tile: Option<u16>,
    pub water_set: Option<u16>,
    pub shore_pieces: Option<u16>,
    pub misc_pave_tile: Option<u16>,
    pub paved_roads: Option<u16>,
    pub medians: Option<u16>,
}
```

Parse in the same place `TheaterCliffRanges` is built (key names per
temperatmd.ini:46–63 and the LAT doc §2.3: `ClearTile`, `RampBase`,
`RoughTile`, `SandTile`, `GreenTile`, `ClearToRoughLat`, `ClearToSandLat`,
`ClearToGreenLat`, `ClearToPaveLat`, `PaveTile`, `WaterSet`, `ShorePieces`,
`MiscPaveTile`, `PavedRoads`, `Medians`). Store as `pub rmg_tiles: RmgTileKeys`
on `TheaterData`.

**Step 2:** `src/map/rmg/tiles.rs` — resolve tileset numbers to flat tile
indices and expose the predicates:

```rust
//! Flat tile indices and tile predicates for the generator.
//!
//! The original resolves theater `[General]` tileset numbers to the first
//! tile of each set at load; phases compare `CellClass` tile indices against
//! those bases. `TileIds::resolve` reproduces that with `TilesetLookup`.

pub struct TileIds {
    pub clear: i32,          // ClearTile set start (flat index; 0 in practice)
    pub ramp_base: i32,      // RampBase set start
    pub rough: i32,
    pub sand: i32,
    pub green: i32,
    pub rough_lat: i32,      // ClearToRoughLat set start (span 0x10)
    pub sand_lat: i32,
    pub green_lat: i32,
    pub pave_lat: i32,
    pub pave: i32,
    pub water_base: i32,     // WaterSet start (span 0x28)
    pub shore_base: i32,     // ShorePieces start + set length
    pub shore_len: i32,
    pub misc_pave: i32,      // span 14 (0x486650)
    pub paved_roads: i32,    // span 15 (0x4866D0)
    // ... plus the water-ish auxiliary set bases from 0x004863D0 that exist
    // in the theater (bridge sets via TheaterData bridge tables).
}
```

`resolve(theater: &TheaterData) -> TileIds` maps each tileset number through
`TilesetLookup::bounds()`. Predicates (free functions over `(ids, tile)`):

- `is_clear(tile)` — `tile == 0 || tile == 0xFFFF` (verified `0x00486380`;
  NOT membership in the clear set).
- `is_green_lat(tile)` — `tile == green || (green_lat..green_lat+0x10).contains(tile)` (`0x004867B0`).
- `is_sand_lat(tile)` — same shape with sand (`0x00486790`).
- ~~`is_water_ish`~~ — DROPPED (corrected during execution, 2026-07-20):
  `0x004863D0` is the cliff/impassable predicate over CliffSet@`0x00AA1020`,
  already modeled by `TheaterCliffRanges::is_cliff_or_impassable_tile`
  (verified via decompile_function 0x004863D0 + the 0x005455B5 key table in
  the LAT doc). Phases that "check water" per the terrain doc actually check
  cliff/impassable — reuse `theater.cliff_ranges`, do not duplicate.
- `is_shore_piece(tile)` — ShorePieces set membership.
- `is_road_blocking(tile)` / `is_pave_blocking(tile)` — the four 6×6-gate
  ranges (spans 15/4/14/16 per RMG_START_POINT_SCORING §2.3).

**Step 3:** Tests — resolve against the real `ini/temperatmd.ini` (include_str
like plan-1 settings tests): assert clear=0-set, green resolves to set 41's
start, ramp to set 9, spans as documented; predicate truth tables on
hand-picked indices.

**Step 4:** `cargo test -p vera20k --lib rmg::tiles` → PASS. Commit.

---

### Task 3: Generator grid (`grid.rs`)

**Why:** Phases mutate a CellClass-like grid; `MapCell` emission is a
projection of it at the end.

**Files:** Create `src/map/rmg/grid.rs`; Modify `mod.rs` (re-export).

**Step 1:** Types:

```rust
/// One generated cell — the CellClass fields the generator reads or writes.
#[derive(Debug, Clone, Copy)]
pub struct GridCell {
    /// Flat tile index; 0 = clear ground, -1 (0xFFFF) = unassigned-clear.
    pub tile: i32,
    /// Sub-tile index within a multi-cell tile block (0 = anchor/unset).
    pub sub_tile: u8,
    /// Ground level (z).
    pub level: u8,
    /// Slope index (0 flat; 1..18 ramps) — mirrors the +0x11C byte.
    pub slope: u8,
    /// Overlay type index, -1 = none.
    pub overlay: i32,
    /// Overlay density/frame byte.
    pub density: u8,
    /// An object occupies this cell (tree, tech building).
    pub occupied: bool,
    /// Start-cell marker (the +0x140 bit-4 equivalent).
    pub start_marker: bool,
}

pub struct RmgGrid {
    width: usize,          // linear stride (g_PathfinderLinearMapWidth analog)
    cells: Vec<GridCell>,
}
```

Initial cell state comes from Task 1's `0x00599650` findings (levels init to 4
per the plan-1 header-geometry verification; tile init per doc).

**Step 2:** API: `get/get_mut(x, y)`, `iter_native()` — an iterator yielding
cells in the EXACT traversal order Task 1 verified for
`MapClass__CellIterator`, plus `step(coord, dir)` using the verified
`DIRECTION_OFFSETS` table:

```rust
/// Clockwise-from-north neighbor offsets, index = direction code 0..7.
pub const DIRECTION_OFFSETS: [(i16, i16); 8] = [
    (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1),
];
```

**Step 3:** `land_type(&self, ids: &TileIds, x, y) -> u8` per Task 1's +0xEC
mapping; `is_clear_at`, `has_overlay`, etc. thin wrappers over `tiles.rs`
predicates.

**Step 4:** Tests: direction table matches the doc values; iterator order
matches Task 1's finding on a small grid (spell the expected sequence out);
default cell state.

**Step 5:** `cargo test -p vera20k --lib rmg::grid` → PASS. Commit.

---

### Task 4: Real dimensions + diamond bounds (`mod.rs` / `emit.rs`)

**Why:** Kills `PLACEHOLDER_INTERIOR`; every phase needs true genW/genH and
diamond bounds.

**Files:** Modify `src/map/rmg/mod.rs`, `src/map/rmg/emit.rs`.

**Step 1:** Implement `fn generated_dimensions(options) -> (u32, u32)` from
Task 1's `0x00599650` formula (player-count term × `dimension_scale`, exact
ftol truncation via `x87::ftol`). Replace `PLACEHOLDER_INTERIOR`.

**Step 2:** Compute diamond min/max + scratch/corner origins per Task 1's
formulas; construct `RmgScratch`/`RmgGrid` from them in `generate()`.

**Step 3:** Tests: dimensions for the 4×4 option grid × player counts 2..8
against hand-walked formula values (formula from doc, not from the code —
independent recomputation in the test); header geometry stays consistent with
plan-1's verified Size/LocalSize padding.

**Step 4:** `cargo test -p vera20k --lib rmg` → PASS. Commit.

---

### Task 5: Water phase, map types 0–2 (`phases/water.rs`)

**Why:** First stream consumer; everything downstream shifts if this drifts.
**Depends on:** Tasks 1–4.

**Doc:** RMG_WATER_SEED §2–§4 + Task-1 extensions.

**Step 1:** `pub fn run(grid, scratch, rng, ids, options)`:
1. Fill every cell `tile = ids.water_base`.
2. Dispatch on `options.map_type`: 0 archipelago, 1 continental, 2
   islands-in-sea — each implemented exactly per the Task-1-extended doc
   (island center rejection draws, `0x0059A8F0` grid, `0x0059BBC0` blobs with
   their verified draw counts; continental ≤100 blob calls w/ water-fraction
   target `(max−min)·(1−Water·0.01)+min` and Manhattan nearest-unvisited scan;
   islands: 1 split-axis draw, 2 landmasses ≤100 calls each).
   Archipelago half-width: `max(2, num_players / 2)`.
3. Isolated-water removal: any still-water cell whose 4 cardinal neighbors are
   all clear → `tile = 0`.
4. Region-data reset (`scratch.reset_region_ids()` + free region objects).
5. Shore-to-green: for each shore-piece cell, clear cardinal neighbors get
   `tile = ids.green`.

All iteration in native iterator order; all draws via `rng` with the exact
rejection shapes from the doc.

**Step 2:** Tests: all-water precondition post-fill; post-shape some non-water;
isolated-removal on a crafted 3×3; shore-to-green writes green only on clear
cardinals; draw-count regression per shape function on fixed seeds (assert the
number of draws consumed matches a hand-walked count from the doc for a tiny
map — this is the stream-shape ratchet).

**Step 3:** `cargo test -p vera20k --lib rmg::phases::water` → PASS. Commit.

---

### Task 6: Water finalizer (`phases/water_finalize.rs`)

**Why:** Assigns visible water variants; formula-complete today.

**Doc:** RMG_WATER_SEED §5 (all constants disasm-verified 2026-07-20).

**All three draws are scaled-FP chains, NOT integer mods** (corrected during
/review-plan via disassemble_function 0x0059C630 + read_memory 0x007ED9D8..E8,
2026-07-20 — the water doc's "reduce mod N" wording was stale). Implement each
as a TruncF64 chain with the literal bit constants; an integer `%` maps the
same raw draw to a different value.

```rust
/// ≈10·2⁻³² — finalizer 2×2-vs-single selector scale (0x007ED9E8).
const FINALIZE_K10_BITS: u64 = 0x3E24_0000_0014_0000;
/// ≈242·2⁻³² — 2×2 variant draw scale (0x007ED9E0).
const FINALIZE_K242_BITS: u64 = 0x3E6E_4000_001E_4000;
/// ≈201·2⁻³² — single-cell band draw scale (0x007ED9D8).
const FINALIZE_K201_BITS: u64 = 0x3E69_2000_0019_2000;
```

**Step 1:** For every cell in native order with `tile == water_base &&
sub_tile == 0`:
- If E (dir 2), S (dir 4), SE (dir 3) neighbors are also water-base with
  sub_tile 0:
  - d1 = `ftol(draw × K10 + 1.0)` (FILD → FMUL K10 → FADD 1.0 → ftol),
    rejection-resampled while > 10 → d1 ∈ {1..10}; if `d1 != 1`
    (2×2 path taken with p = 9/10, NOT 10/11):
    - d2 = `ftol(draw × K242)`, rejection while > 241 → {0..241};
    - `variant = if d2 < 240 { d2 / 10 } else { 0xF7 - d2 }` (240→7, 241→6;
      signed magic-number division, plain `/ 10` on non-negative i32 is exact);
    - place multi-cell tile `water_base + variant` anchored at this cell
      (block placement per `0x005A6C10`: writes tile, sub_tile per block cell,
      level/slope bytes from tile data — model with `TilesetLookup`
      block dims; region arg −1 ignored for grid state);
    - continue (skip single-variant path).
- Single-cell path (or `d1 == 1`): d3 = `ftol(draw × K201)`, rejection while
  > 200 → {0..200}; `tile = water_base + 8 + d3 / 40` (unsigned div; 6 bands,
  band 5 only at 200).

**Step 2:** Tests: band split (d3 values 0,39,40,199,200 → offsets 8..13);
edge-case d2 draws 240/241 → variants 7/6; the FP chain vs `%` divergence
(raw draw 0x80000000 → d3 = 100, whereas `% 201` would give 3 — assert the
chain result); the `sub_tile == 0` guard skips placed blocks; 2×2 anchor
requires all three neighbors; single-path probability gate `d1 == 1`.

**Step 3:** `cargo test -p vera20k --lib rmg::phases::water_finalize` → PASS. Commit.

---

### Task 7: Region partition (`region.rs` + `phases/mod.rs` wiring)

**Why:** Region ids feed starts, tech, and tiberium.
**Depends on:** Task 1 (queue discipline, water-flag byte, seed-pick draws).

**Doc:** RMG_REGION_PARTITION + audit slot-6 fixes (design ledger 23–29).

**Step 1:** `region.rs` types:

```rust
pub struct RmgRegion {
    pub id: i32,
    pub cell_count: i32,
    pub terrain_type: u8,      // seed cell's level/terrain byte (+0x10)
    pub active: bool,          // +0x14
    pub seed_cell: (i16, i16), // +0x16 packed
    pub done: bool,            // +0x1A
    pub start_quota: i32,      // +0x20 (filled by starts phase)
    pub cells: Vec<(i16, i16)>,          // +0x2C array, +0x38 count
    pub field_slots: Vec<(i16, i16)>,    // region_sub: +0x4 array, +0x10 count
}
```

**Step 2:** Phase order (verified `0x00598960`):
1. zero id counter; reset all scratch region/stamp to −1; drop old regions.
2. Seed scan (`0x0058CF90`): for each unassigned, non-null cell that has a
   bridge overlay OR passes green-LAT membership → flood-fill constructor.
3. Per active region: BFS expander with pass count
   `4 + (cell_count > 8000) as i32 + (!mode34) as i32` (mode34 = map_type 3|4),
   then the water-flag propagation pass (Task-1-reconciled flag byte).
4. Fallback seeder: every remaining unassigned cell → new region (param 0 →
   `done` byte).

Flood-fill constructor per Task 1: queue discipline, neighbor order, the
enqueue gate `IsClearTile() || green_lat_membership()` (audit fix — NOT a
bridge check), terrain-type match, and the rejection-sampled seed-cell pick
(may consume >1 draw).

**Step 3:** Tests: pass-count formula table (4 combos); all-cells-assigned
post-fallback on a crafted grid; enqueue gate truth table; draw-count on a
seeded fixture.

**Step 4:** `cargo test -p vera20k --lib rmg::region` → PASS. Commit.

---

### Task 8: Green spread (`phases/green_spread.rs`)

**Doc:** RMG_TERRAIN_SHAPING_CORE §3.1.

**Step 1:** Collect vector: for every green-LAT cell (native order), each of
the 4 even directions' clear neighbors appended (duplicates allowed as native).
`count = min(len / 3, 1000)`. Repeat count times: index draw = the inline
chain `FILD draw → FMUL (f64)len → FMUL K(0x007ED898) → ftol`, unsigned
rejection while `> len − 1`, where **len is the LIVE list length re-read at
each conversion** (verified via disassemble_function 0x0059B740, 2026-07-20:
`0x0059B828..0x0059B869`). This is arithmetically identical to
`RmgRng::uniform(0, len - 1)` — same span-before-K multiply order, `+0.0`
floor is exact, rejection equivalent for non-negative results — so use
`uniform(0, len - 1)`. Then remove the entry by shift-down, set that cell's
tile to green, append its clear even-direction neighbors.

**Step 2:** Tests: cap at 1000; `len/3` truncation; shift-down removal order
preserved (fixture comparing picked sequence for a fixed seed); newly
converted cells re-feed the list.

**Step 3:** `cargo test -p vera20k --lib rmg::phases::green_spread` → PASS. Commit.

---

### Task 9: Starts (`phases/starts.rs`) — DONE 2026-07-20

> **Completion summary.** Implemented as `phases/starts.rs` + `phases/zones.rs` +
> `sqrt_approx.rs`. Differences from the steps below, all binary-verified this
> session (see the START_GENERATION doc §11 addendum + the new
> RMG_ZONE_SUBSYSTEM doc):
> - Step 2's "quota = num_players" was imprecise: the quota is the separate
>   global `DAT_00ABE028` (standard setup 4); `.SED NumPlayers` only bounds the
>   clearing floods. `StartsArgs` carries both.
> - The rebuild's cell filter is class-byte == 0 (`CellClass+0x4C` is the
>   passability class, not an occupier) AND derived-zone == reference — this
>   required porting the zone subsystem (scanline fill, adjacency edges,
>   per-kind derived tables, largest-component reference) into `zones.rs`.
> - The 6×6 gate's real polarity: clear/misc-pave/pave PASS, roads +
>   PavedRoadEnds reject, everything else rejects (the scoring doc's original
>   reading was wrong; corrected there).
> - All distances use the retail 16 KiB table sqrt (`0x004CAC40`), generated at
>   runtime and golden-tested byte-exact against `ini/sqrt_table.bin`.
> - Deletion threshold `max(genW·genH·0.03, 400)`; sort key multiplier is the
>   PRE-deletion region count; quota split is cumulative-fraction rounding.
> - Deferred to Task 10: `0x00595400` body (the coin flip IS consumed here, so
>   the stream is exact up to the first winning region). Deferred to Task 15:
>   the `0x0058B820` preview-metadata blob; wiring real TMP terrain bytes +
>   `[Land]` Wheel flags into `ZoneParams` (test stubs cover them today).

**Why:** Waypoints + field slots (tiberium input) + per-start flood-fill.

**Docs:** RMG_START_GENERATION, RMG_START_POINT_SCORING (formulas + asm),
RMG_TIBERIUM_RECHECK §6 (slot formula), design ledger 43–49.

**Step 1:** Outer contract: `loop { b = gen_starts(); if !b continue;
if flood(num_players) { break } }` — both native callees return 1
unconditionally, so implement as straight calls but keep the retry shape as a
comment-documented no-op (do NOT invent failure semantics).

**Step 2:** `gen_starts` (`0x00594B50`): quota = num_players (the
`DAT_00ABE028` = MapSeed+0x50 identity). Distribute proportionally across
selected regions (rounded per-bucket, last bucket = remainder) into
`region.start_quota`; call the per-region gatherer with the running start
index offset.

**Step 3:** Per-region gatherer (`0x00594870`), exact draw shapes:
- lane draw: `Random__Next`, `FILD`, `FMUL [0x7ED8C0]` (≈3·2⁻³²·(1+2⁻²⁴) —
  use the bit pattern `0x3E08_0000_0018_0000`... **read the literal qword from
  the doc's constants table: `0x007ED8C0` bytes `000018000000083e`**), ftol,
  reject >2. TruncF64 chain in that exact order.
- iteration limit `lane + quota*15`, absolute cap 300.
- per iteration: candidate index draw = `rand × count × K` (FMUL count then
  FMUL `0x007ED898` K — note op order differs from `uniform`: no +min, span
  = count, reject > count−1); 6×6 gate at (x−3, y−3, 6, 6) — all 36 cells
  must pass road/water/misc-pave/pave range checks and IsClearTile; view-region
  margin check (globals per Task 1's formulas, +4 inset); admit to buffer.
- selector (`0x00594F40`): slot target
  `trunc((TibLayout×0.01×12.0/NumPlayers + 2.0) × region.start_quota)` with the
  exact special cases (quota==0 && cand>0 → target=cand; target>cand||target==0
  → target=cand and if quota==0 return None); pair-scan seed (max distance via
  `approx_sqrt`, +20.0 cross-region when scratch region ids differ), then
  greedy max-min; NO RNG. Output = `region.field_slots`.
- waypoint writes: for i in 0..quota: waypoint slot (offset+i) = selected
  cell i; set grid `start_marker`; mirror metadata (our model: the
  `start_waypoints` vec on `GeneratedMap`).

**Step 4:** Flood pass (`0x005A1FB0`): for i in 0..num_players: read waypoint
i; flood ≤400 pops from it in the Task-1-verified queue order, marking scratch
stamp = i+1 (+ the +0x45-equivalent byte per Task 1) gated on diamond bounds,
unclaimed stamp, IsClearTile.

**Step 5:** Tests: slot-target formula vectors; +20 bonus gate; 6×6 rejection
on crafted road/water cells; farthest-first on a line of 4 candidates; 2-draw
per-attempt stream shape; proportional bucket remainder.

**Step 6:** `cargo test -p vera20k --lib rmg::phases::starts` → PASS. Commit.

---

### Task 10: Tech buildings (`phases/tech_buildings.rs`) — DONE 2026-07-20

> **Completion summary.** Implemented as `phases/tech_buildings.rs`.
> `0x005A95B0` + `0x00595400` decoded live this session. Both driver paths
> ported exactly: map type 2 iterates regions in creation order (uniform(0,2)
> passes, one building per `start_quota > 0` region, anchor from
> `region.cells`); map types 1/3/4 place uniform(0,4) buildings at
> `uniform(0, stride²−1)` random scratch cells with the two-level
> empty-slot/non-clear rejection (confirmed the `g_PathfinderLinearMapWidth²`
> bound at `0x005A971E`). Shared foundation gate (`0x00578540` frame test =
> `diamond_frame_contains`, occupancy, clear, level match, scratch `+0x45`
> protected-clearing flag). The land-type-3 (Rock) check is a proven no-op
> after the clear-tile test (clear ⇒ land 0), documented and omitted. Phase
> takes resolved `TechType`s (name + rectangle footprint); ruleset resolution
> of `NeutralTechBuildings` + emission as neutral `[Structures]` is Task 15.

**Docs:** RMG_TERRAIN_SHAPING_CORE §3.8; RMG_MODE34 doc (maptype-2 path
`0x00595400`); design ledger 50.

**Step 1:** Skip entirely when `map_type == 0`.
- **maptype ≠ 2:** n = `uniform(0, 4)`; per building: uniform pick from
  `rules` `NeutralTechBuildings` list (`[General]`, 6 entries — parse via
  existing Ruleset general-section access; if not yet parsed, add the key to
  the rules general struct in this task); ≤100 attempts: uniform random
  scratch slot with the 200-try inner empty-slot rejection (keeps last pick);
  foundation cells (building's foundation from `ObjectType.foundation` +
  occupy lists) must each be: in-diamond/on-map, unoccupied, clear,
  `level == anchor level`, `land_type != 3`, `!water_flag`; on success mark
  cells occupied and record the placement (name + anchor cell) on a new
  `GeneratedMap.tech_buildings: Vec<(String, u16, u16)>` (emitted as
  `[Structures]` neutral entries in Task 15); failure after 100 → drop.
- **maptype == 2:** passes = `uniform(0, 2)`; per pass, for each region with
  `start_quota > 0`: one uniform type pick, ≤100 anchors drawn from the
  region's cell list, same foundation gate.

**Step 2:** Tests: type-0 no-op; foundation gate rejects level mismatch /
water flag / occupied on a crafted grid; draw shape (n draw always consumed).

**Step 3:** `cargo test -p vera20k --lib rmg::phases::tech_buildings` → PASS. Commit.

---

### Task 11: Tiberium (`phases/tiberium.rs`) — DONE 2026-07-20

> **Completion summary.** Implemented as `phases/tiberium.rs`. Driver
> `0x005A23A0` + placer `0x005A28C0` re-decoded live to confirm the RECHECK
> doc; `bVar12` read exactly from the driver's switch (`0→Res==3`,
> `1|3|4→Res!=3`, `2→true`). Both passes + the gem-anchor block (average
> waypoint / random-cell reference, truncated running-min argmin) ported with
> the two-stage-truncated field-count formula and the Gaussian ±100 size
> jitter. Placer: `blob::MinHeap`, 10-seed cap with map-wide `stamp` wipe per
> seed, anchor rebind on first written cell (clears the frontier), TIBTRE draw
> once per seed generation, ore(102+d)/gem(27+d) overlay writes, revisit
> density increment, and the scratch `+0x45` blocked gate. `GetTiberiumType`
> (`0x00485010→0x005FDD20`) confirmed as an `IsTiberium`-flag check → modeled
> as the ore/gem overlay ranges. `region.field_slots` is now `Option` so the
> null-vs-empty selector-list distinction (which changes the gem-anchor draw
> stream) is preserved. **Cross-phase invariant made explicit:** the driver
> iterates regions in native creation order with `global_start_base` advancing
> by native-order quota, reading start waypoints written by the starts phase
> in sorted order — the port reproduces gamemd's exact (quirky) association
> because `Regions.list` stays in creation order and starts writes at sorted
> offsets.

**Doc:** RMG_TIBERIUM_FIELD_COUNT_AND_GATES_RECHECK (GREEN — every constant
read from memory; embed them by bit pattern).

**Step 1:** Driver per region (regions in native array order,
`global_start_base` running across regions):
- gem-anchor block when `bVar12` (= `Resources == 3 && map_type ∉ {1,3,4}`…
  **derive bVar12's exact predicate from the doc §5: types 1/3/4 make
  `bVar12 = Resources != 3` — re-read the doc block when implementing and
  copy its decision table into a unit test**): reference point = average of
  region's start waypoints (or rejection-sampled random region cell when no
  starts); `nearest_slot = argmin approx_sqrt(d²)` with ftol'd running min
  init 500000.
- field count: `lerp = trunc(tib% × 0.01 × (max−min) + min)` (FPU op order:
  FILD tib → FMUL 0.01 → FILD span → FMULP → FIADD min → ftol);
  `region_total = trunc(lerp_as_f64 × max(start_quota_as_f64, 0.5))`;
  skip region if `field_slots.len() == 0 || region_total == 0`;
  `per_field = region_total / slots` (signed truncating div).
- per slot i: Gaussian×50.0 rejection-resampled into [−100,100];
  `size = trunc(per_field + j)`; if ≥0 → placer(slot_coord, size,
  base+i+1, i == nearest_slot).
- gem second pass (UNCONDITIONAL): per-start scores = mean approx_sqrt
  distance to all slots (sum first, one divide); `size = trunc((score −
  min_score) × 15.0) + 500`; origin = start's waypoint cell;
  `gem2 = Resources == 3 && map_type ∈ {1,3,4}`; placer per start with id
  base+s+1; then `global_start_base += start_quota`.

**Step 2:** Placer (`0x005A28C0`): 10-seed cap incl. first; per seed: map-wide
stamp wipe (native iterator), reseed from the SAME origin; min-heap keyed f32
priority, capacity `size×10` silent drop; pop path: blocked-flag gate; first
written cell → anchor rebind + queue clear + (if !gem) TIBTRE draw
`trunc(rand × K₃ + 1.0)` reject >3 → `TIBTRE01..03` terrain object placed;
overlay write `overlay = d + base` (base 102 ore / 27 gem) with density draw
`trunc(rand × K₁₂)` reject >11; else density<11 → +=1; else no placed++.
Neighbor admission chain in the doc's exact short-circuit order (diamond →
IsClearTile → fresh-empty admit → revisit admit); priority
`approx_sqrt(d²) + uniform_unit×5.0` (one draw per admitted neighbor);
claim stamp at PUSH time.

**Step 3:** Tests: two-stage truncation vectors over tib ∈ {0,1,37,50,99,100} ×
min/max {(900,1050),(2500,5500)} × quota 0..8; gate-chain truth table on a
crafted 3×3 (all four reject paths + both admits); TIBTRE names ∈ 01..03;
gem2 decision table; per-seed stamp wipe observable.

**Step 4:** `cargo test -p vera20k --lib rmg::phases::tiberium` → PASS. Commit.

---

### Task 12: Hills (`phases/hills.rs`) — DONE 2026-07-20

> **Corner engine complete** (`phases/hills_corners.rs`, 7 tests; `hills::run`
> orchestrates seed→walk→build→morph→finalize→quad-cleanup). Both blockers
> resolved: the corner-grid geometry from `MapClass__Resize` (origin (1,1),
> W=H=map_w+map_h−1, grid (map_w+map_h)²) and the morphable flag
> (`IsoTileType+0x2E0` = `[TileSet] Morphable=` = `TilesetLookup::is_morphable`).
> Build lock predicate, mask picker (LOCKED-filtered), recursive
> adjust+propagate with LIFO height-only rollback, finalize pattern-match
> against the 19×4 `[NW,NE,SE,SW]` ramp table, and the 2×2 quad cleanup all
> ported. Engine consumes no RNG. Full geometry write-up appended to
> `RMG_HILLS_CORNER_ENGINE_GHIDRA_REPORT.md`.

Historical note (superseded by the DONE summary above):

> **Status.** Steps 1–2 (water-adjacency seed `0x005A33F0` + height random walk
> `0x005A2F50`) are implemented and committed (`phases/hills.rs`, 5 tests). Both
> functions were re-decoded live: FP constants + operand order verified from the
> `0x005A2F50` disasm, `0x004863D0` confirmed as the cliff/obstacle predicate (NOT
> "water-ish" — reuses `TheaterData::is_cliff_or_impassable_tile`), the full
> pipeline order + the 2×2 quad-cleanup tail read from `0x005A35F0`. The verified
> walk FP contract is in `RMG_TERRAIN_SHAPING_CORE_GHIDRA_REPORT.md` §3.2-impl.
> **Remaining (steps 3–6):** the corner-morph engine — corner build `0x006B2A70`,
> current-level `0x006B4100`, mask pick `0x006B4240`, corner adjust `0x006B3E60`,
> recursive slope propagation `0x006B3A80`, finalize `0x006B3850`, and the quad
> cleanup — being decoded into `RMG_HILLS_CORNER_ENGINE_GHIDRA_REPORT.md`. The
> corner engine consumes no RNG, so the draw stream is already exact through the
> walk; the morph is a deterministic post-process.
>
> **Corner-engine research: DONE** (`RMG_HILLS_CORNER_ENGINE_GHIDRA_REPORT.md`,
> 2026-07-20). Full contract captured: corner grid `(W+1)×(H+1)` `{i32 height,
> locked, visited}`, cell→corner mapping (NW=i, NE=i+1, SW=i+W+1, SE=i+W+2),
> the build lock predicate (with the corrected `IsoTileType+0x2E0 != 0 =
> MORPHABLE` polarity), mask pick (bits NW/NE/SE/SW, filter on LOCKED not
> visited), the recursive adjust+propagate with LIFO height-only rollback, the
> finalize pattern-match, and the verified 19×4 ramp table `[NW,NE,SE,SW]`.
> **One open item before implementation:** the corner-grid geometry —
> `DAT_0087F90C/910/914/918` = `MapClass+0x124..0x130` (origin x/y, interior
> W/H) — and which coordinate frame it uses for a **generated** map. The
> xrefs are all reads; `[Map] Size` (e.g. Dustbowl `0,0,70,76`) vs `LocalSize`
> (`2,8,65,62`) must be pinned, and reconciled with the scratch diamond-band
> coords the morph loop iterates, before the build's `(mx-origin, my-origin)`
> indexing is safe. Do NOT implement the corner engine until this frame is
> verified — a wrong frame silently breaks every corner index.

**Doc:** RMG_TERRAIN_SHAPING_CORE §3.2 (a)–(f) + ramp table; design ledger
31–36. **Read the `0x005A2F50` disasm for the exact FP op order** (doc's own
instruction — decompile op order is not sufficient for bit-exactness).

**Step 1:** (a) water-adjacency seed: shore-piece cell → first in-diamond
clear neighbor over 8 dirs gets `height = 0.5` + water flag; water-ish cell →
flag only; first match breaks.

**Step 2:** (b) random walk (skip whole pass when `R×0.0025 < 0.025`):
per cell row-major: smooth from W and N (`h += h[nbr]; v += v[nbr]; ×0.5`
each), NW tilt term clamped `±(R×0.0001+0.1)`, out-of-diamond neighbor adds
`v += R×0.0025`; flagged cells clamp `h ≥ 0`, `v := 0.0025`
(bits `0x3F64_7AE1_47AE_147B`); draw 1 Gaussian rejection into
`[−v, R×0.005−v]` (window recentered when out of ±0.025); draw 2 Gaussian
rejection keeping `h ∈ [−2,2]` biased by tilt; final pass ftol-truncates all
heights. All arithmetic TruncF64 in disasm order.

**Step 3:** (c) corner grid `(W+1)×(H+1)`: corner height =
`slope_corner_table[slope][corner] + level×0xF`; locking per the four
adjacency checks (overlay, occupier, water flag, non-morphable/non-clear/non-
0xFFFF tile, out-of-diamond). Embed the 19-entry table verbatim (ledger 35):

```rust
/// Corner deltas per slope type, order NW,NE,SE,SW, units 1/15 level.
pub const RAMP_CORNERS: [[i32; 4]; 19] = [
    [0,0,0,0], [0,15,15,0], [0,0,15,15], [15,0,0,15], [15,15,0,0],
    [0,0,15,0], [0,0,0,15], [15,0,0,0], [0,15,0,0],
    [0,15,15,15], [15,0,15,15], [15,15,0,15], [15,15,15,0],
    [0,15,30,15], [15,0,15,30], [30,15,0,15], [15,30,15,0],
    [0,15,0,15], [15,0,15,0],
];
```

**Step 4:** (d) per-cell push: `n = |ftol(level + h − current)|` where current
= min(4 corners)/0xF or own level when ineligible; n × {corner-mask pick
(all-equal → all unvisited; raising → unvisited below max; lowering →
unvisited above min), ±0xF adjust clamped [0,0xB4], undo record, visited mark,
recursive |Δ|≤0xF propagation; locked hit → rollback whole op}.

**Step 5:** (e) finalize: modified-corner cells with spread <0x10, in-diamond,
no overlay/occupier, !water-flag, morphable/clear/0xFFFF tile:
`level = min/15`; slope from pattern match against `RAMP_CORNERS`;
`tile = clear` (slope 0) else `ramp_base + slope − 1`.

**Step 6:** (f) quad cleanup: quads (0,0),(1,0),(1,1),(0,1) of slopes
{5,6,7,8} → flatten (tile 0xFFFF, sub_tile 0, slope 0); {11,12,9,10} →
flatten + level += 1.

**Step 7:** Tests: table equality vs doc; R<10 skip; clamp/lock behavior on a
crafted 4×4 (locked neighbor rolls back); quad-cleanup level bump; walk
draw-count fixture on a fixed seed.

**Step 8:** `cargo test -p vera20k --lib rmg::phases::hills` → PASS. Commit.

---

### Task 13: LAT patches, trees, rocks (`phases/lat_patches.rs`, `trees.rs`, `rocks.rs`) — DONE 2026-07-20

> **Status.** All three phases committed. `phases/lat_patches.rs` (7 tests): prob
> setup (temperate shore-bias + base mix, non-temperate sand/green), both painters
> with the sequential fresh-draw rough→sand→green test and the Gaussian size
> draws, and the shared min-heap patch placer. Mean draw constant `0x007EDAD0`
> (`0x3E35_0000_0015_0000`). `phases/trees.rs` (9 tests): count formula, whole-map
> anchor pick (`uniform(0, S²−1)` + `(0,0)`-reject, 200 tries), density/size
> Gaussian clamps, and the `0x005A45E0` region-walk placer. The tree-index draw
> was PINNED to `ftol(rand·0x3E39000000190000 + 1.0)` reject >25 → **`v ∈ [1,25]`**
> (the `+1.0` offset means `TREE00` is unreachable; supersedes the old
> `uniform(0,25)` working model — see `RMG_TERRAIN_SHAPING_CORE` §3.5-impl).
> `phases/rocks.rs` (6 tests): temperate-only SROCK/TROCK pass with the verified
> sand=LAT-range-only vs green=base+range predicate asymmetry. The LAT fixup
> between patches and trees is Task 14 (RNG-free); the port already has
> `map::lat::apply_lat` to build on.

**Doc:** RMG_TERRAIN_SHAPING_CORE §3.3–§3.5; design ledger 37–42.

**Step 1:** `lat_patches.rs` —
- TEMPERATE (theater option 0): probability setup (`V = Veg×0.01`; non-shore
  cells with sand-prob 0: sand 0.005, rough `V×0.02`, green 0.005; shore-piece
  cells with any in-diamond 5×5 neighbor: sand 0.005, green 0, rough
  `V×0.02×10`); painter: clear stamps; three mean draws
  `ftol(rand×21/2³² + 20.0)` in order rough, sand, green (constants
  `0x007EDAD0`, `0x007E44C8`); per clear cell (slope 0, no overlay, no
  occupier, !water-flag) sequential fresh-draw tests r1<rough → rough else
  r2<sand → sand else r3<green → green; hit → size = Gaussian×20+mean
  rejection-clamped [4,80] → placer(cell, tile, ftol(size), y×0x200+x, 0);
  then full-map LAT fixup pass (Task 14's function).
- non-TEMPERATE: fill sand-prob 0.005 / green-prob 0.001
  (bits `0x3F50_624D_D2F1_A9FC`, written-but-unread); per clear cell (slope 0,
  !water-flag, NO overlay/occupier check): one draw < 0.005 → rough patch
  Gaussian×15+20 clamp [4,60]; LAT fixup; NO sand/green/rocks.
- Patch placer (`0x005A4B60`): min-heap from origin; pop → set tile; admit
  8-neighbors in-diamond, clear, stamp ≠ patch_id, slope 0, no overlay, no
  occupier; priority `approx_sqrt(d²) + rand×5/2³²` (one draw per admitted
  neighbor); place exactly `size` cells or frontier empty.

**Step 2:** `trees.rs` — count `ftol((Width_opt×0.1 + 0.7) × Veg×0.01 ×
max_trees)` (constants `0x007E3860`, `0x007EDAC0`, `0x007E3808`); while
count>0 && iterations<100: uniform slot (200-try empty/non-clear reject keeps
last), density Gaussian×0.1+0.2 clamp [0.05,0.4], size Gaussian×10+25 clamp
[10,35]; `count -= scatter(cell, ftol(size), density)`. Scatterer: heap region
≤ size×25 cells (visited flag, water flag blocks); per popped clear,
unoccupied, overlay-free, land≠3 cell: draw < density → uniform 0..25 →
`TREE{v/10}{v%10}` → place terrain object (TREE00 miss per Task 1's finding);
stop after `size` trees; return placed.

**Step 3:** `rocks.rs` (TEMPERATE only) — quota = uniform
`[0, (H+4)×W×2/200]` inclusive (one draw); attempts = quota×5; per attempt:
uniform slot (reject empty); `overlay == -1` required; sand-LAT → overlay
`uniform(0,4) + 168` (SROCK01–05); clear or green-LAT → `+173` (TROCK01–05);
density := 0.

**Step 4:** Tests: probability order + fresh draws (stream fixture); mean
range [20,40]; clamp windows; non-TEMPERATE emits no sand/green/rocks; rock
indices only 168–177; tree-count formula vectors; land≠3 exclusion.

**Step 5:** `cargo test -p vera20k --lib rmg::phases` → PASS. Commit.

---

### Task 14: LAT fixup (`lat_fixup.rs`) — PARTIAL 2026-07-20 (LAT groups done; slope fixup deferred)

> **Status.** `phases/lat_fixup.rs` committed (12 tests). Re-verified
> `CellClass::ApplyLAT_and_SlopeFixup` live (`decompile_function 0x0047CA80`) —
> the Apr-24 doc matches the binary exactly. Ported the LAT half: Rough→Sand→
> Green→Pave, cardinal mask (bits N/E/S/W from `DIRECTION_OFFSETS[0,2,4,6]`),
> direct `lat_base+mask`, hardcoded exemptions (Green shore+0x29 & waterbridge+1;
> Pave miscpave/medians+0xD & pavedroads+0x14; Rough/Sand none), tile-0 map-edge
> sentinel, Rough unguarded / others `!= -1` guarded. Added `WaterBridge` to
> `RmgTileKeys`/`TileIds`. **Deferred:** the slope-fixup half — it dispatches on
> the `+0x11C` slope-type byte (0..4), which is NOT the port's `slope:0..18`
> field; the slope-type source needs its own RE pass. It is RNG-free, so
> deferral does not desync the draw stream; impact is cliff-ramp tile visuals.

**Doc:** docs/research/LAT_GROUPS_AND_SLOPE_FIXUP_GHIDRA_REPORT.md (HIGH —
read §2.3's group parameter table + exemption ranges before writing).

**Step 1:** Port the four-group pass (Rough → Sand → Green → Pave) over
`RmgGrid`: per group, cells matching base-or-LAT-range get a 4-bit cardinal
mask (bit0 N, bit1 E, bit2 S, bit3 W; neighbor sets bit when NOT in group
range and NOT in any exemption range); mask 0 → base tile; else
`lat_base + mask`. Then the slope-fixup half per the doc. Map-edge sentinel
behavior per doc.

**Step 2:** Call sites: patch painter (Task 13) and any recalc point Task 1
identifies as LAT-applying.

**Step 3:** Tests: mask bit assignment table from the doc; isolated cell →
base; exemption ranges honored (crafted neighbors); idempotence on already-
fixed terrain.

**Step 4:** `cargo test -p vera20k --lib rmg::lat_fixup` → PASS. Commit.

---

### Task 15: Emit + `generate()` wiring (`emit.rs`, `mod.rs`, `settings.rs`) — IN PROGRESS 2026-07-20 (emit projection done; phase wiring remains)

> **Status.** `emit::populate` committed (`17389d40`, 5 new tests). Resolved the
> grid→MapFile coordinate transform: the RMG grid `(x,y)` frame IS the engine
> cell frame the IsoMapPack loader reads into `MapCell{rx,ry}` (verified via
> map-prep `0x00599650` §10.2 — same diamond predicate), so the projection is an
> **identity coordinate mapping**. Cells (0xFFFF→loader −1, tile 0 kept, level→z),
> overlays (+`OverlayDataPack::from_cells` density grid), trees/TIBTRE→terrain,
> neutral tech→`[Structures]` (Neutral house, health 256), starts→`[Waypoints]`.
> **Remaining:** wire the phases into `generate()` and swap `empty_map_file` for
> `empty_map_file`+phases+`populate`.
>
> **STAGE_ORDER reconciled** (`a97ce884`): inserted RecalcAfterPatches/Trees/Rocks
> to match the native driver tail; Rocks gated to temperate.
>
> **Pipeline wired** (`84000c6f`): `pipeline::run_pipeline` runs every phase in
> STAGE_ORDER over shared grid/scratch/rng/gauss, threading regions→starts→
> tech/tiberium, the zone field into starts, waypoints into tiberium, and
> collecting trees/TIBTRE/tech for emit. Inputs are pre-resolved (`PipelineInputs`)
> so it's unit-testable with synthetic `TileBlocks`; 4 tests incl. determinism.
>
> **theater→`TileBlocks` adapter done** (`571eeb64`): `theater_blocks::
> TheaterTileBlocks::build` resolves each flat tile index → TMP filename →
> `TmpFile` and projects each cell's `(height, terrain_type)` into a
> `TileBlock`/`SubTile`; the app supplies the byte loader. 3 tests.
>
> **build layer done** (`f6213c7f`): `build::generate_map` produces a **populated
> `MapFile` end-to-end** from a `ResolvedTheaterInputs` snapshot (`TileIds::
> resolve` + cliff ranges + per-tile `Morphable` table + `wheel_impassable_from_
> rules`) — `run_pipeline` then `emit::populate`. `wheel_impassable_from_rules`
> maps each RA2 `LandType` to its rules section (`Wheel<=1%`; stock → only Rock).
> Snapshotting decouples the run from `TheaterData`, so it's unit-testable with a
> hand-built resolution + synthetic `TileBlocks`. 5 tests incl. determinism.
>
> **app_init `.sed` hook done** (`2faf3601`): the random-map load branch now
> drives `build::generate_map` with a real `TheaterTileBlocks` (from
> `theater.lookup` + the asset manager) and `ResolvedTheaterInputs::from_theater`
> (wheel table from `rulesmd.ini`, graceful fallback). A `.sed` selection now
> produces a **populated, deterministic `MapFile`** through the normal load path.
>
> **The generator pipeline is complete and VERIFIED IN-GAME** (`cacc073f`).
> `RA2_QUICKPLAY=RandMap.Sed` generated a temperate 74x82 2-player map on real
> theater data: **no panic**, all 319 referenced tiles resolved (`0 out-of-range,
> 0 parse errors`), the tile atlas built (361 tiles), and the sim ran to tick
> 1440 with 573 ore cells + 7 TIBTRE trees driving the economy — i.e. a populated,
> renderable, playable generated map through the normal load path. The identity
> coordinate transform + tile-sentinel mapping are confirmed correct (the renderer
> resolved every emitted tile index). **Remaining minor/flagged items** (none block a map appearing): (a)
> tech_types (`NeutralTechBuildings` footprints) → no tech buildings; (b)
> map-type-3/4 island passes (no phase module); (c) map-type-0 tech path; (d)
> `Width`+0x64 binary check; (e) hills slope-fixup half (`+0x11C` RE); (f) the
> starts phase `debug_assert` can panic in debug builds on start-starved maps.
> Step 1 settings (ore-patch lamps unused per §10.4; ambient lighting a tail
> detail) remains optional.

**Why:** Turns phase state into the playable `MapFile`; completes the pipeline.

**Step 1:** `settings.rs`: parse the remaining RMGMD keys —
`TemperateOrePatchLamps`/`SnowOrePatchLamps` (name lists),
`TemperateAmbientLight`/`SnowAmbientLight` (4-int by Time),
ambient R/G/B vectors (4-int each). Tests against `ini/rmgmd.ini` values.

**Step 2:** `emit.rs` population from `RmgGrid`:
- cells: interior grid → `MapCell { rx, ry, tile_index, sub_tile, z: level }`
  with the grid→map coordinate offset from Task 1's `0x00599650` findings
  (LocalSize inset 2,5). **Tile-sentinel mapping is explicit:** grid uses
  native semantics (0 = clear set, 0xFFFF = unassigned-clear) while
  `MapCell.tile_index` uses −1 = clear ground (map_file.rs:145) — emit
  0xFFFF as −1, and emit 0 as the resolved clear-set flat index (verify
  against how the loader treats each before locking the mapping in a test).
- overlays: cells with `overlay != -1` → `OverlayEntry { rx, ry, overlay_id,
  frame: density }` + `OverlayDataPack::from_decoded` grid.
- terrain objects: trees + TIBTRE placements → `TerrainObject { rx, ry, name }`.
- tech buildings → `[Structures]` neutral-house entries in the `ini` document
  (health 256, facing 0, matching the loader's expected shape — read
  `src/map/map_file.rs` entity parsing before writing this).
- waypoints: start slot i → `Waypoint { index: i, rx, ry }` (0..NumPlayers−1).
- `[Lighting]` (+`[Basic]` name from options Description format): values per
  Task 1's scenario-lighting mapping from Time + theater ambient vectors.
- `RandMap.img`-equivalent preview is Plan 3 — do NOT emit here.

**Step 3:** `mod.rs`: replace the stage-recording skeleton with real phase
dispatch in `STAGE_ORDER` (keeping `stages_run` recording); `RmgDeps` struct;
update the `src/app_init.rs` caller (re-read that file first — parallel
sessions). Region reset between tiberium and hills per stage list (scratch
clear + region free); `Emit` stage last.

**Step 4:** Tests: emitted MapCell/tile round-trip through
`TilesetLookup::bounds`; overlay pack byte at rock/ore cells; waypoint count ==
num_players; `[Lighting]` values for Time 0..3; the existing
`sim_does_not_reference_the_generator` guard still passes.

**Step 5:** `cargo test -p vera20k --lib rmg && cargo check -p vera20k` → PASS.
Commit.

---

### Task 16: Determinism, end-to-end smoke, verification pass

**Step 1:** Determinism: for seeds {0, 1234, 0xFFFF} × map types {0,1,2} ×
theaters {0,1}: `generate` twice → assert byte-equal cells, overlays,
terrain objects, waypoints (extend plan-1's reproducibility test to full
content hashing).

**Step 2:** E2E: generate a 2-player temperate map, feed through the
`.SED` launch branch (existing plan-1 route) in a headless test as far as
`MapFile` consumption allows (`load_map_from_initial` unit-level); assert the
loader accepts the generated map (no panics, waypoint capacity ≥ NumPlayers).

**Step 3:** Draw-stream ledger: one test per phase asserting total draws
consumed on a fixed tiny fixture (golden counts hand-walked from the docs at
implementation time, recorded in the test as the ratchet).

**Step 4:** Verification record: AUDIT_LOG line
`- **YYYY-MM-DD** — RMG plan-2 verification — <per-phase status>`; every
phase labeled either "formula-verified vs doc <name>" or
UNVERIFIED-pending-instrument for full-map parity. Update
`docs/plans/2026-07-19-random-map-generator-design.md` P1 status.

**Step 5:** Full `cargo test -p vera20k` — expect only the two pre-existing
parallel-session failures. Commit.

---

## Sim Checklist

- Generator is map-layer; no `sim/` files change. The `emit.rs` guard test
  enforces no `sim/` → `map::rmg` dependency.
- No f32/f64 enters `sim/`; all generator FP is TruncF64/x87 module.
- No tick ordering or state-hash impact (pre-play construction).

## Sources & References

- **Design doc:** docs/plans/2026-07-19-random-map-generator-design.md
- **Plan 1 (complete):** docs/plans/2026-07-20-random-map-generator-plan.md
- **Ghidra reports:** skirmish-ui/RMG_TERRAIN_SHAPING_CORE,
  RMG_WATER_SEED_0059A6C0, RMG_REGION_PARTITION_0058CF90,
  RMG_START_GENERATION_00594B50_005A1FB0, RMG_START_POINT_SCORING_00594870,
  RMG_TIBERIUM_FIELD_COUNT_AND_GATES_RECHECK, RMG_TIBERIUM_CREATION_005A23A0,
  RMG_MODE34_WATER_BRIDGES_TECH, SKIRMISH_RANDOM_MAP_GENERATOR_00598960,
  RMG_X87_FP_CONTRACT; ../LAT_GROUPS_AND_SLOPE_FIXUP_GHIDRA_REPORT.md
- **Key addresses** (kept here, never in Rust comments): pipeline `0x00598960`;
  water `0x0059A6C0/0x0059AD10/0x0059AFA0/0x0059B200/0x0059A8F0/0x0059BBC0/
  0x005ADA40`; finalizer `0x0059C630`; regions `0x0058CF90/0x0058C800/
  0x0058E740/0x0058E9B0/0x0058D010`; green `0x0059B740`; starts
  `0x00594B50/0x00594870/0x00594F40/0x005A7250/0x005A1FB0`; tech
  `0x005A95B0/0x00595400`; tiberium `0x005A23A0/0x005A28C0`; hills
  `0x005A35F0/0x005A2F50/0x005A33F0/0x006B2A70/0x006B3E60/0x006B3A80/
  0x006B3850`; LAT stage `0x005A38C0/0x005A3AE0/0x005A4280/0x005A4B60/
  0x005A45E0`; LAT fixup `0x0047CA80`; map prep `0x00599650`.
- **INI keys:** theater `[General]` tile identities (temperatmd.ini:46–63);
  rulesmd.ini `NeutralTechBuildings`; `[OverlayTypes]` positions 27/102/
  168–177; `[TerrainTypes]` TREE01–25; `ini/rmgmd.ini` all keys.
- **Related code:** src/map/rmg/* (plan-1), src/map/theater.rs,
  src/map/overlay.rs, src/map/waypoints.rs, src/map/map_file.rs,
  src/rules/object_type.rs (foundations).
