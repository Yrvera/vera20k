# Bridgehead Anchor Renderer — Design (G3 renderer follow-up)

## Goal

When `BridgeRuntimeCell.bridgehead_anchor_class != Variant0`, render the cell using the matching BridgeSet variant tile_id — closing the visual loop on sim G3's bridgehead direct-damage path so direct ramp fire produces the same anchor-tile transition gamemd's `SetOverlayAndPropagate` writes to `IsoTileTypeIndex (+0x38)`.

Scope: HIGH bridges (concrete `BridgeSet`) only. LOW bridges (`WoodBridgeSet`) deferred until the LOW state machine writes the field.

## Architecture Context

**Where anchor cells render today.** An anchor is a **terrain tile**, not a bridge-body SHP sprite. It flows through:

- `MapFile.cells.tile_index` → LAT-adjusted in `ResolvedTerrainGrid::build` → `ResolvedTerrainCell.final_tile_index`.
- `build_terrain_grid_from_resolved` ([src/map/terrain.rs:424](../../src/map/terrain.rs#L424)) copies into `TerrainCell.tile_id` (precomputed, immutable).
- Per-frame `build_visible_instances` ([src/map/terrain.rs:524](../../src/map/terrain.rs#L524)) reads `TerrainCell.tile_id`, `sub_tile`, `variant`, calls a UV lookup `f(tile_id, sub_tile, variant)`.

**Existing precedent.** `build_visible_instances` already overrides the `variant` argument per-frame based on sim state ([terrain.rs:579-586](../../src/map/terrain.rs#L579-L586)): when `cell.has_damaged_data`, the `variant` slot is sourced from `bridge_state.cell(rx, ry).damaged_variant`. This is the FA2 `{base}a/b/c/d` sibling-TMP swap. It overrides one argument level — `variant` — at render time without rebuilding `TerrainGrid`.

**The anchor case differs**: gamemd's `+0x38` write changes the **`tile_id` itself** (a different tile within `BridgeSet`), not a variant slot. So this design adds a parallel override **one level up** from the existing one.

**Theater plumbing.** `TheaterData` ([src/map/theater.rs:362](../../src/map/theater.rs#L362)) already parses `[General] BridgeSet=` and `WoodBridgeSet=` (tileset indices). It does NOT yet parse `BridgeMiddle1=` / `BridgeMiddle2=`, which are the two scalars the state machine needs.

**Atlas pre-load.** `theater::collect_used_tiles` ([src/map/theater.rs:540](../../src/map/theater.rs#L540)) scans actual map cells for `(tile_id, sub_tile)` pairs. Tiles never present at map load are not in the atlas → runtime atlas miss. The 4 NS + 4 EW variant tile_ids must be pre-injected.

**Sim state.** `BridgeRuntimeCell.bridgehead_anchor_class` ([src/sim/bridge_state/mod.rs:455](../../src/sim/bridge_state/mod.rs#L455)) is currently initialized unconditionally to `Variant0` in `from_resolved_terrain` (line 549). gamemd derives it from the cell's loaded `+0x38` (map authors can place pre-damaged anchors); G3 sim ignores that.

## Impact Analysis

**Touched files:**

- [src/map/theater.rs](../../src/map/theater.rs) — parse two more `[General]` keys; add 2 fields to `TheaterData`; add `BridgeAnchorVariantTable` helper.
- [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs) — new field `ResolvedTerrainCell.bridgehead_anchor_class_at_load: Option<BridgeheadAnchorClass>`; new pre-classification pass near the existing bridgehead-detection loop (~line 550).
- [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) — `from_resolved_terrain` reads the pre-classified value instead of `Variant0` default.
- [src/app_init_helpers.rs](../../src/app_init_helpers.rs) — `inject_bridge_anchor_variant_tiles` helper injects the 8 variant tile_ids (×all sub_tiles) into the atlas pre-load set.
- [src/map/terrain.rs](../../src/map/terrain.rs) — `TerrainGrid` gains `anchor_variant_table: Option<BridgeAnchorVariantTable>`; `build_terrain_grid_from_resolved` takes one more arg; `build_visible_instances` adds the tile_id override branch.
- [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) — no change (hash already includes `bridgehead_anchor_class`); but tests must update for maps with pre-damaged anchors.

**Files NOT touched** (active parallel session — leave alone):

- src/sim/miner.rs, src/sim/movement/, src/sim/pathfinding/ — parallel-session work.
- [src/app_instances/bridges.rs](../../src/app_instances/bridges.rs) — bridge-body SHP renderer, unrelated to anchor terrain tiles.

**Risk areas:**

- **Pre-damaged anchor initialization** changes the initial state-hash for maps that author pre-damaged anchors. One-time deterministic shift; world-hash tests on such maps need updating.
- **Atlas pre-load surface**: must enumerate all sub_tiles of each of 8 variant tile_ids. Missing any one is a per-cell atlas miss on the corresponding cell.
- **TerrainGrid constructor signature**: gains one parameter. Callers update once.
- **Theater INI mods**: keys can be absent → variant table is `None` → override disabled → cell renders native tile_id. Parity drift, but only on mods without those keys; acceptable defensive fallback.
- **Determinism**: pre-classification is pure read from `final_tile_index` + immutable theater data. No RNG, no mutation. State-hash deterministic.

## Chosen Approach

**Approach A** from the brainstorm: theater-derived variant table at theater load, resolved-terrain pre-classification, per-frame tile_id override in the existing visible-instance pipeline. Sim has no theater dependency; renderer reads the table; classification flows through the resolved-terrain layer as additional per-cell metadata.

Rejected approaches are listed under "Alternatives Considered."

## Tiny-Detail Ledger

All items verified inline against gamemd.exe during the brainstorm (Ghidra MCP session 2026-05-13). Citations: `[GHIDRA <addr>]` = live decompilation; `[INI: <file>:<line>]` = retail theater INI value.

**R1. `BridgeMiddle1` parsing.** `[General] BridgeMiddle1=N` read from theater INI. Default `0xFFFFFFFF` (treated as "missing"). `[GHIDRA 0x00545c1e]` `[INI: temperat.ini:97 = 7]`. → `TheaterData::bridge_middle_1: Option<u8>`.

**R2. `BridgeMiddle2` parsing.** `[General] BridgeMiddle2=N` read from theater INI. `[GHIDRA 0x00545c3a]` `[INI: temperat.ini:98 = 12]`. → `TheaterData::bridge_middle_2: Option<u8>`.

**R3. NS variant tile_ids (4 entries).** `{BS + M1 - 1, BS + M1, BS + M1 + 1, BS + M1 + 2}` where `BS = BridgeSet's first tile_id`, `M1 = BridgeMiddle1`. Enum order: `Variant0 → AboutToFall`. For temperate (`BS = lookup of BridgeSet tileset start`, `M1 = 7`): {BS+6, BS+7, BS+8, BS+9}. `[GHIDRA 0x00576BD2 entry gate]`. → `BridgeAnchorVariantTable::ns`.

**R4. EW variant tile_ids (4 entries).** `{BS + M2 - 1, BS + M2, BS + M2 + 1, BS + M2 + 2}` where `M2 = BridgeMiddle2`. For temperate (M2 = 12): {BS+11, BS+12, BS+13, BS+14}. `[GHIDRA 0x00576C91 entry gate]`. → `BridgeAnchorVariantTable::ew`.

**R5. First-hit anchor write target.**
- NS: `anchor.tile_id ← M1 + 2 + BS` = NS variant tile_ids[3] (= `AboutToFall`). `[GHIDRA 0x00577701]`
- EW: `anchor.tile_id ← M2 + 2 + BS` = EW variant tile_ids[3] (= `AboutToFall`). `[GHIDRA 0x0057769b]`

**Implication:** despite the enum names `Variant0/Variant1/Damaged/AboutToFall`, sustained direct ramp fire jumps the anchor straight from `Variant0` to `AboutToFall`. The `Damaged` enum value (iVar2 = M*+2, tile_id index 2 in the variant list) is reached **only** via neighbor `UpdateRamp_*_DamageB` progression (sim G3 ledger #13: `Variant1 → Damaged`), never on the anchor cell from a first-hit. /write-plan must add a clarifying doc-comment on the `BridgeheadAnchorClass` enum.

**R6. Sub_tile preservation.** `SetOverlayAndPropagate` writes `+0x38` (`IsoTileTypeIndex`) only; the cell's sub_tile field is unchanged. Renderer override must preserve `cell.sub_tile` when swapping `tile_id`. `[GHIDRA SetOverlayAndPropagate @ 0x0056EB80, reads +0x38 only]`.

**R7. Per-frame override gate.** Override fires iff `bridge_state.cell(rx, ry).bridgehead_anchor_class != Variant0` AND the variant table is `Some`. `Variant0` → use cell's native `tile_id` (idempotent for both Anchor and Bridgehead role cells — see R10). `[Logical consequence of R5 + R6]`.

**R8. Atlas pre-load.** All 8 variant tile_ids × all sub_tiles in each tile_id's TMP template must be loaded at theater init, NOT lazily at first damage. Without this, atlas miss = blank cell at the moment damage is applied — instant visual artifact. → `inject_bridge_anchor_variant_tiles` extends the needed-set before `load_tile_images` runs.

**R9. Map-load pre-classification.** Cells where `tileset_index == bridge_set` AND `final_tile_index` matches one of the 8 variant tile_ids must have `bridgehead_anchor_class_at_load = Some(matched variant)`. Cells not matching default to `None` (sim init then defaults to `Variant0`). Closes the parity hole for author-damaged anchors. `[Logical consequence of gamemd reading +0x38 at map load — the runtime state is derived from the loaded cell, not zero'd]`.

**R10. Bridgehead-role cells share the same override path.** Sim G3 writes `bridgehead_anchor_class` on both Anchor- and Bridgehead-role cells (DamageB neighbor propagation hits Bridgeheads — sim G3 ledger #13). Bridgehead cells' native `tile_id` IS already in the variant range; the override behaves correctly for them:
- When sim-class == Variant0, override is bypassed and native = Variant0's tile_id (idempotent).
- When sim writes Variant1 (DamageB progression), override swaps to Variant1's tile_id.

The override gate (R7) does NOT branch on role — same code path serves both.

**R11. Missing theater INI keys.** When `bridge_middle_1` or `bridge_middle_2` is `None`, `BridgeAnchorVariantTable::from_theater` returns `None`. `TerrainGrid.anchor_variant_table` is `None`. Override always falls through to native `tile_id`. Parity drift = no visible damage on bridges for that mod. Acceptable defensive fallback.

**R12. Logging.** Log once at theater load if `bridge_set.is_some()` but either `BridgeMiddle1` or `BridgeMiddle2` is `None` ("BridgeSet present without BridgeMiddle1/2; anchor damage visuals disabled"). No per-frame logging on the renderer hot path. → wired into `theater::load_theater`.

**R13. Sentinel tile_id handling.** Pre-classification skips cells where `final_tile_index < 0` and treats `0xFFFF` as tile_id 0 per the existing `normalize_tile_id` convention. Reverse-match never matches the sentinel value.

**R14. Out-of-scope: collapse 5th tile.** Sim G3 doesn't write the collapse-branch tile_id (`BS + M* + 3`, e.g., BS+10 NS / BS+15 EW). Renderer ignores it. When future work adds the collapse path, the enum gains a 5th variant; the table extends by 1 entry per axis; atlas pre-load extends by 2 tile_ids. Renderer changes will be additive.

**R15. Cell layer mismatch (atlas).** Each variant tile_id's TMP template may have multiple sub_tiles (typical 1×1 templates have 1; some bridges may be larger). Atlas pre-load enumerates `0..=N-1` for each template via the TMP file's own `template_width × template_height`, matching how `collect_used_tiles` currently feeds the loader.

## Design

### Components

**1. `theater.rs` — Variant table + INI parsing**

Add two `[General]` keys to `TheaterData`:

```
pub struct TheaterData {
    // ... existing ...
    pub bridge_set: Option<u16>,
    pub wood_bridge_set: Option<u16>,
    /// `[General] BridgeMiddle1=` — BridgeSet-relative tile_id offset for NS
    /// bridgehead variant block. Used to compute the 4 NS variant tile_ids.
    pub bridge_middle_1: Option<u8>,
    /// `[General] BridgeMiddle2=` — same for EW.
    pub bridge_middle_2: Option<u8>,
}
```

Extend `parse_general_int` calls in `load_theater` (~line 451) to fill these. Log once at `INFO` level if `bridge_set` present but `bridge_middle_*` absent (R12).

New small struct in `theater.rs`:

```
/// Theater-derived 4-NS + 4-EW tile_id table for HIGH bridge anchor variants.
/// Built once at theater load from BridgeSet + BridgeMiddle1/2.
#[derive(Debug, Clone, Copy)]
pub struct BridgeAnchorVariantTable {
    /// Variant0..AboutToFall (4 entries, enum order).
    pub ns: [u16; 4],
    pub ew: [u16; 4],
}

impl BridgeAnchorVariantTable {
    /// Returns None if BridgeSet, BridgeMiddle1, or BridgeMiddle2 is missing,
    /// or if BridgeMiddle1 < 1 (would underflow the Variant0 = BS+M-1 offset).
    pub fn from_theater(td: &TheaterData) -> Option<Self> { ... }

    /// Lookup the tile_id for a (axis, class) pair. Returns None when class
    /// is Variant0 (caller uses native cell.tile_id in that case).
    pub fn tile_id_for(&self, axis: Axis, class: BridgeheadAnchorClass) -> Option<u16> { ... }

    /// Reverse-match a tile_id to (axis, class). Used at map load to
    /// pre-classify author-damaged anchors. None if tile_id is not a variant.
    pub fn match_tile_id(&self, tile_id: u16) -> Option<(Axis, BridgeheadAnchorClass)> { ... }
}
```

Note: `BridgeAnchorVariantTable::from_theater` requires the `BridgeSet` tile_id start, not just the tileset index. It computes start by querying `TilesetLookup::bounds()[bridge_set_idx as usize].start`. So `from_theater` takes `(td: &TheaterData)` and reads `td.lookup` internally.

**2. `resolved_terrain.rs` — Pre-classification**

`ResolvedTerrainCell` gains one field:

```
/// Author-damaged anchor pre-classification: Some(variant) if this cell's
/// final_tile_index matches one of the 8 bridgehead variant tile_ids.
/// None when not a variant tile (the common case; sim defaults to Variant0).
/// Used by BridgeRuntimeState::from_resolved_terrain.
pub bridgehead_anchor_class_at_load: Option<BridgeheadAnchorClass>,
```

(Axis is derivable from the matched table entry but already available via `BridgeLayer.direction` and the bridgehead-detection pass — no extra storage needed.)

Pre-classification logic in `ResolvedTerrain::build` (placed after the existing bridgehead-detection loop ~line 605, before the gap-fill pass ~line 611):

```
// 1. Resolve the variant table from theater data.
let variant_table: Option<BridgeAnchorVariantTable> =
    theater_data.and_then(BridgeAnchorVariantTable::from_theater);

// 2. For each cell whose tileset_index matches bridge_set, reverse-match
//    final_tile_index against the variant table.
if let Some(table) = variant_table {
    for cell in cells.iter_mut() {
        let Some(ts_idx) = cell.tileset_index else { continue };
        let Some(bs) = theater_data.and_then(|td| td.bridge_set) else { continue };
        if ts_idx != bs { continue; }
        if cell.final_tile_index < 0 { continue; }
        let tid = if cell.final_tile_index == 0xFFFF { 0 } else { cell.final_tile_index as u16 };
        if let Some((_axis, class)) = table.match_tile_id(tid) {
            cell.bridgehead_anchor_class_at_load = Some(class);
        }
    }
}
```

**3. `bridge_state/mod.rs` — Sim init reads pre-classification**

In `BridgeRuntimeState::from_resolved_terrain` (~line 549), replace:

```
bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
```

with:

```
bridgehead_anchor_class: resolved
    .bridgehead_anchor_class_at_load
    .unwrap_or(BridgeheadAnchorClass::Variant0),
```

Sim still has no theater dependency — it reads only `ResolvedTerrainCell` data.

Add a doc-comment to the `BridgeheadAnchorClass` enum (~line 165) clarifying R5: the four variants are the four `IsoTileTypeIndex` slots gamemd uses. First-hit anchor writes jump straight to `AboutToFall`; `Damaged` is only reached on neighbor cells via DamageB progression. (Per CLAUDE.md, no gamemd addresses in code comments — just behavioral description.)

**4. `app_init_helpers.rs` — Atlas pre-load extension**

After `collect_used_tiles` (~line 194) and before `load_tile_images`:

```
if let Some(table) = theater_data.as_ref()
    .and_then(BridgeAnchorVariantTable::from_theater)
{
    inject_bridge_anchor_variant_tiles(
        &mut needed,
        &table,
        &theater_data.unwrap().lookup,
        &asset_manager,
    );
}
```

Helper (lives in `theater.rs` next to `collect_used_tiles`):

```
/// Inject TileKey entries for the 8 bridge-anchor variant tile_ids × all
/// sub_tiles in each tile_id's TMP template. Required so the atlas has the
/// variant tiles loaded before any bridge damage happens at runtime — gamemd
/// pre-loads BridgeSet's full tileset; this is the equivalent pre-load for
/// our atlas pipeline.
pub fn inject_bridge_anchor_variant_tiles(
    needed: &mut HashSet<TileKey>,
    table: &BridgeAnchorVariantTable,
    lookup: &TilesetLookup,
    asset_manager: &AssetManager,
) { ... }
```

Enumerates sub_tiles by peeking each variant tile_id's TMP `template_width × template_height` via the existing TMP parser. (R15.)

**5. `terrain.rs` — TerrainGrid carries the table; per-frame override**

`TerrainGrid` gains:

```
/// Theater-derived bridge-anchor variant table, threaded from TheaterData.
/// None when theater lacks BridgeMiddle1/2 keys.
pub anchor_variant_table: Option<BridgeAnchorVariantTable>,
```

`build_terrain_grid_from_resolved` ([terrain.rs:424](../../src/map/terrain.rs#L424)) gains a parameter:

```
pub fn build_terrain_grid_from_resolved(
    resolved: &ResolvedTerrainGrid,
    local_bounds: Option<LocalBounds>,
    anchor_variant_table: Option<BridgeAnchorVariantTable>,  // NEW
) -> TerrainGrid { ... }
```

Per-frame override in `build_visible_instances` ([terrain.rs:575-595](../../src/map/terrain.rs#L575-L595)):

```
// Existing variant override (damaged_variant — unchanged).
let effective_variant: u8 = if cell.has_damaged_data {
    bridge_state.and_then(|bs| bs.cell(cell.rx, cell.ry))
        .map(|bc| bc.damaged_variant as u8)
        .unwrap_or(0)
} else { cell.variant };

// NEW: tile_id override for bridgehead anchor variants.
let effective_tile_id: u16 = grid
    .anchor_variant_table
    .and_then(|table| {
        let bc = bridge_state?.cell(cell.rx, cell.ry)?;
        if bc.bridgehead_anchor_class == BridgeheadAnchorClass::Variant0 {
            return None;
        }
        let axis = bc.axis?;
        table.tile_id_for(axis, bc.bridgehead_anchor_class)
    })
    .unwrap_or(cell.tile_id);

let placement = match &uv_fn {
    Some(f) => f(effective_tile_id, cell.sub_tile, effective_variant),
    ...
};
```

Per R6, `cell.sub_tile` is passed unchanged.

### Interfaces / Contracts

**New public surface:**

- `theater::BridgeAnchorVariantTable` struct + three methods (`from_theater`, `tile_id_for`, `match_tile_id`).
- `TheaterData::bridge_middle_1`, `TheaterData::bridge_middle_2` (additive).
- `ResolvedTerrainCell::bridgehead_anchor_class_at_load` (additive).
- `TerrainGrid::anchor_variant_table` (additive).
- `theater::inject_bridge_anchor_variant_tiles` (helper, called once from app init).

**Modified signatures:**

- `build_terrain_grid_from_resolved` gains `anchor_variant_table: Option<BridgeAnchorVariantTable>` arg.

**Internal contracts:**

- The override gate is the **single source of truth** for "should I render the variant tile?" It does not branch on cell role (anchor vs bridgehead) — uniform `class != Variant0` test.
- `BridgeAnchorVariantTable::match_tile_id` returns `None` for non-variant tile_ids; callers must default to `Variant0` (sim's existing semantic).
- `tile_id_for` returns `None` when class is `Variant0` (matches override gate's expectation that Variant0 falls through to native).

### Data Flow

```
Theater load
  ├─ load_theater(): parses BridgeMiddle1/2 → TheaterData
  └─ BridgeAnchorVariantTable::from_theater(td) → Option<[u16;4] NS + [u16;4] EW]

Map load
  ├─ ResolvedTerrain::build()
  │   └─ pre-classification pass: each BridgeSet-tileset cell's
  │       final_tile_index reverse-matched against variant table
  │       → ResolvedTerrainCell.bridgehead_anchor_class_at_load
  ├─ BridgeRuntimeState::from_resolved_terrain()
  │   └─ copies bridgehead_anchor_class_at_load into BridgeRuntimeCell
  │       (defaults Variant0 if None)
  └─ inject_bridge_anchor_variant_tiles() — atlas pre-loads 8 tile_ids
      × all sub_tiles before load_tile_images runs

Sim tick (no changes from G3)
  └─ bridgehead_advance_state writes anchor.bridgehead_anchor_class

Render per frame
  └─ build_visible_instances:
      for each TerrainCell:
        effective_tile_id = override or native (per R7)
        atlas lookup(effective_tile_id, cell.sub_tile, effective_variant)
```

### Error Handling

- **Missing INI keys** (R11/R12): `bridge_middle_*` defaults to `None`. `BridgeAnchorVariantTable::from_theater` returns `None`. Renderer override is bypassed. One `log::info!` at theater load.
- **Underflow guard**: `from_theater` returns `None` if `BridgeMiddle1 < 1` (Variant0 = BS + M - 1 would underflow). Same for `BridgeMiddle2`.
- **Out-of-bounds tile_id**: `tile_id_for` arithmetic produces `BS + offset`; if any of the 8 falls outside `TilesetLookup::len()`, `from_theater` returns `None` (with a `log::warn!` once). Defensive against malformed theater INIs.
- **Atlas miss at render time**: existing renderer logs per-cell warnings via `log::warn!` in the body-instance path. The terrain pass has no equivalent because the atlas is presumed complete after pre-load; if pre-load fails (a variant TMP is missing from MIX), the cell falls through to whatever the UV function returns for an unknown key. Add a `log::warn!` at the new override path if the override produces a tile_id not in `atlas`, but only once per (rx,ry) to avoid hot-path spam.
- **No runtime panics** anywhere on this path. All failures degrade gracefully to "render native tile_id".

### Testing Strategy

**Unit tests:**

1. **`BridgeAnchorVariantTable::from_theater` happy path** — temperate (M1=7, M2=12, BS=lookup): `ns = [BS+6, BS+7, BS+8, BS+9]`, `ew = [BS+11, BS+12, BS+13, BS+14]`.
2. **`from_theater` returns None on missing M1**.
3. **`from_theater` returns None on missing M2**.
4. **`from_theater` returns None on missing BridgeSet**.
5. **`from_theater` returns None on M1=0 (underflow)**.
6. **`from_theater` returns None on out-of-bounds variant tile_id**.
7. **`match_tile_id` round-trip** — for each (axis, class), `match_tile_id(tile_id_for(axis, class)) == Some((axis, class))`.
8. **`match_tile_id` rejects non-variant tile_id** — pre-Variant0 (BS+5), post-AboutToFall (BS+10), between blocks (BS+10), outside BridgeSet (BS-1).
9. **`tile_id_for(Variant0) == None`** — Variant0 falls through to native.

**ResolvedTerrain integration:**

10. **Pre-classification: pristine anchor** — cell's `final_tile_index = BS+6` (NS Variant0). After build, `bridgehead_anchor_class_at_load == Some(Variant0)`. Same test passes if it's `None` (Variant0 default — both encode the same sim state).
11. **Pre-classification: author-damaged anchor** — cell's `final_tile_index = BS+9` (NS AboutToFall). `bridgehead_anchor_class_at_load == Some(AboutToFall)`.
12. **Pre-classification: non-variant BridgeSet cell** — cell's `final_tile_index = BS+0` (a non-bridgehead BridgeSet tile, e.g., the anchor-base tile). `bridgehead_anchor_class_at_load == None`.
13. **Pre-classification: missing theater table** — `theater_data = None` or keys missing. All cells get `None`. No panics.

**Sim init:**

14. **`BridgeRuntimeState::from_resolved_terrain` copies pre-classification** — resolved cell with `Some(AboutToFall)` → bridge cell `bridgehead_anchor_class == AboutToFall`. Resolved cell with `None` → bridge cell `Variant0`.

**Renderer (build_visible_instances):**

15. **Override fires** — bridge_state cell with `class = Damaged`, table set; rendered tile_id == `table.ns[2]` (NS Damaged). Sub_tile passed unchanged.
16. **Override bypassed for Variant0** — class = Variant0 → effective_tile_id == cell.tile_id (native).
17. **Override bypassed when table is None** — `anchor_variant_table = None` → native tile_id always used.
18. **Override bypassed when axis is None** — bridge cell with `axis = None` (shouldn't happen on real anchor/bridgehead cells, but guard against orphan body cells) → native tile_id.
19. **EW axis routes correctly** — cell with axis=EW, class=Damaged → tile_id == `table.ew[2]`.

**Atlas pre-load:**

20. **`inject_bridge_anchor_variant_tiles` adds 8 tile_ids × N sub_tiles** — given a table and a TilesetLookup with bridgehead templates of `template_width × template_height`, the `needed` set grows by `8 × (W × H)` entries (minus duplicates if any sub_tiles were already needed from the map cells).
21. **Pre-load helper handles missing variant TMPs** — if a variant TMP file isn't in the MIX archive, the helper logs and skips without panic.

**Integration test (extends existing G3 work):**

22. **End-to-end: shoot ramp, anchor tile changes** — synthetic 5x5 map with a HIGH NS bridge. Pre-conditions: pristine anchor at `BS+6` analog, atlas pre-loaded. Fire IonCannonWH at a bridgehead cell. Assert: (a) `bridge_state.cell(anchor).bridgehead_anchor_class == AboutToFall`, (b) `build_visible_instances` for the anchor cell emits a SpriteInstance whose `uv_origin` matches the variant tile_id's atlas entry. Pixel-equivalent check via atlas-key extraction.

23. **Map-load init: pre-damaged anchor renders damaged from frame 1** — load a map with `final_tile_index = BS+9` at an anchor cell. Before any damage, `build_visible_instances` emits the AboutToFall tile_id sprite. No first-tick blank-frame regression.

**Existing tests to update:**

- World-hash tests on maps with pre-damaged anchors will see a new initial hash (was Variant0, now matches map). Update fixture expectations.

### Determinism

- Pre-classification is pure read (no RNG, no shared state).
- `BridgeAnchorVariantTable::from_theater` is pure compute over theater data, called once at theater load.
- Per-frame override is read-only against immutable theater + per-frame sim state.
- World-hash already includes `bridgehead_anchor_class`; no new hash field needed. Hash divergence with previous build is one-time and deterministic going forward.

## Architectural Decisions

**Patterns followed:**

- **Mirror the existing per-frame override pattern.** The new tile_id override sits next to the existing `damaged_variant` variant override in `build_visible_instances`. Same shape, same data source (`bridge_state.cell(rx,ry)`), same Option-chain idiom.
- **Theater-derived metadata baked into ResolvedTerrain.** Sim keeps reading only `ResolvedTerrainCell`; theater data flows through the existing resolution layer rather than being threaded directly into sim.
- **`Option<u8>` for INI keys.** Matches the existing `bridge_set: Option<u16>` / `wood_bridge_set: Option<u16>` convention on `TheaterData`.
- **Atlas pre-load extension via `inject_*` helper.** Reuses the existing `collect_used_tiles` + `load_tile_images` pipeline — additive call, no atlas-architecture change.

**Patterns deviated from:**

- **`build_terrain_grid_from_resolved` gains a parameter.** Today it takes only `(resolved, local_bounds)`. Adding `anchor_variant_table` is a minor signature change; alternative is to copy the table into `ResolvedTerrainGrid` itself, which spreads theater data wider for less gain. Keep the explicit parameter.

**Tech debt introduced / deferred:**

- **WoodBridgeSet renderer support.** Out of scope per user's brainstorm choice; deferred until LOW state machine writes `bridgehead_anchor_class` for wood bridges. Atlas pre-load could optionally pre-load wood variants too (forward-compatible) — kept out of scope to keep the change focused.
- **Collapse 5th tile (`BS + M* + 3`).** Sim doesn't write this from any current path; renderer ignores. Future enum extension is additive (R14).
- **Per-cell role gating could be tighter.** The override gate doesn't check `role == Anchor || role == Bridgehead`. It just checks `class != Variant0`. Since non-Anchor/Bridgehead cells always have `class == Variant0` (sim never writes them), the looser gate is equivalent in behavior. If a future change to sim ever writes the class on Body or Tail cells, the renderer would render the variant tile, which would be wrong — but the sim contract today rules this out. Acceptable risk, documented in module comment.

**Determinism:**

- Lockstep-safe: all new logic is theater + map-data dependent only, no runtime entropy.
- State-hash compat: maps with pre-damaged anchors will produce different initial hashes than the previous build. One-time break, deterministic afterward.

## Alternatives Considered

**Approach B — Sim caches resolved tile_id directly.** `BridgeRuntimeCell` would carry `bridgehead_anchor_tile_id: Option<u16>` alongside the enum. Sim writes both. Renderer reads tile_id directly without enum-to-tile_id table lookup. **Rejected**: forces sim to depend on theater data (variant table threaded into every bridge-tick write), doubles per-cell sim state, leaks render-layer concerns (specific tile_ids) into sim. Strictly worse architecturally.

**Approach C — Event-driven `TerrainGrid` mutation.** Sim emits `SimEvent::AnchorTileClassChanged`, app layer mutates `TerrainGrid.cell.tile_id` in place. Renderer reads tile_id directly with no per-frame override. **Rejected**: requires `TerrainGrid` to become mutable post-build (currently treated as immutable, sort-stable), adds event-plumbing surface that doesn't exist for the rest of the bridge system (bridge body uses per-frame `BridgeRuntimeState` reads, not events), and creates sim/render drift risk if events are missed (save/load, replay branching). Not worth the architectural change.

**Mutate `ResolvedTerrainCell.final_tile_index` directly.** Most literal mirror of gamemd's `+0x38` write. **Rejected**: `ResolvedTerrainGrid` is currently treated as immutable input to sim and to multiple caches (PathGrid, zone grid, lighting, render). Mutating it would require coordinated invalidation across all consumers — much larger refactor than warranted, and the per-frame override pattern accomplishes the same observable result without disturbing the immutability assumption.

## Open Items for /write-plan

1. **`BridgeheadAnchorClass` enum doc clarification.** Add one-paragraph doc-comment explaining R5 (first-hit jumps Variant0 → AboutToFall; `Damaged` is neighbor-only). No address citations per CLAUDE.md comment policy.
2. **Atlas miss diagnostics.** Decide whether to add a one-shot per-cell `log::warn!` in the override path when the resolved tile_id isn't in the atlas, or rely on the existing atlas-side warnings.
3. **TerrainGrid signature change.** `build_terrain_grid_from_resolved` gains one arg. Confirm all call sites at /write-plan time (likely two — production and a synthetic-map test helper).
4. **World-hash test fixtures.** Audit existing world-hash tests for maps with pre-damaged anchor tile_ids; update expected hashes for those (none, if all current test maps use pristine anchors — verify).
5. **`inject_bridge_anchor_variant_tiles` enumeration of sub_tiles** — confirm the existing TMP-parsing helper exposes `template_width × template_height` reachable from theater code without circular deps. (Likely fine — `tmp_file.rs` is in `assets/`, already a dep.)
