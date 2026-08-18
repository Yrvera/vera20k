# Smudge System Design

**Date:** 2026-05-06
**Status:** Approved (brainstorm complete)
**Research basis:**
- `ra2-rust-game-docs/SMUDGE_CLASS_GHIDRA_REPORT.md` (audited YELLOW 2026-05-06; corrections applied below)
- `ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md` (HIGH confidence, 2026-05-06)
- `ra2-rust-game-docs/ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` (Morphable flag)

## Goal

Spawn craters and scorch marks at runtime when explosions and infantry deaths occur, indistinguishable from gamemd.exe in placement, timing, RNG advancement, and ore-destruction side effects. Also load pre-placed `[Smudge]` map entries.

## Architecture Context

### What exists today

- **Per-cell mutable sim state pattern: `OverlayGrid`.** [src/sim/overlay_grid.rs](src/sim/overlay_grid.rs) — small dedicated grid in sim, seeded from map, mutated during gameplay, hashed for determinism. This is the template for `SmudgeGrid`.
- **Warhead detonation pipeline.** [src/sim/combat/mod.rs:535](src/sim/combat/mod.rs#L535) emits `ExplosionEffect { shp_name, rx, ry, z }` events for the killing-blow warhead's `AnimList`. These events are render-only today; no sim-side anim entity.
- **Theater INI parser.** [src/map/theater.rs](src/map/theater.rs) parses `[TileSetNNNN]` sections — currently reads `FileName`, `SetName`, `TilesInSet` only. Missing 13 of 15 documented per-TileSet keys (per `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` §10).
- **AnimType registry.** [src/rules/anim_type.rs](src/rules/anim_type.rs) parses `Damage`, `InfDeath`, `AnimList`-referenced fields, etc. Does NOT parse `Scorch=`, `Crater=`, `ForceBigCraters=`.
- **`MapFile` parser.** [src/map/map_file.rs](src/map/map_file.rs) parses `[Map]`, `[OverlayPack]`, `[Units]`, `[Triggers]`, etc. Does NOT parse `[Smudge]` section.
- **Sim RNG.** Deterministic, used for combat scatter, AI, etc. Available where smudge dispatch needs it.

### What does not exist

- AnimClass-equivalent sim entity (decision: don't add one — Approach A in brainstorm).
- Per-cell smudge state.
- Render layer for ground decals between terrain and entities.
- The hardcoded `Reduce_Tiberium(6)` side-effect on crater spawn.

## Impact Analysis

### Files touched

| File | Change kind |
|---|---|
| `src/rules/smudge_type.rs` | NEW — `SmudgeTypeRegistry` |
| `src/rules/anim_type.rs` | EXTEND — three new bool fields, two new u16 frame-dim fields |
| `src/rules/ruleset.rs` | EXTEND — wire SmudgeTypeRegistry into RuleSet |
| `src/map/theater.rs` | EXTEND — parse `Morphable=` per TileSet |
| `src/map/map_file.rs` | EXTEND — parse `[Smudge]` section |
| `src/map/resolved_terrain.rs` | EXTEND — propagate `morphable` onto `ResolvedTerrainCell` as `accepts_smudge: bool` |
| `src/sim/smudge_grid.rs` | NEW — `SmudgeGrid` |
| `src/sim/combat/smudge_dispatch.rs` | NEW — three dispatcher entry points + 256-entry unit-vec table |
| `src/sim/combat/mod.rs` | EXTEND — wire dispatcher into existing dead-entities loop and explosion emission |
| `src/sim/world/mod.rs` | EXTEND — own `SmudgeGrid`; seed at sim init |
| `src/sim/world/world_hash.rs` | EXTEND — hash `SmudgeGrid` |
| `src/render/smudge.rs` | NEW — decal layer |
| `src/render/mod.rs` | EXTEND — call new layer between terrain and entities |

### Risk areas

1. **Determinism.** Three new RNG draws per smudge spawn (filter pick + the two discarded `RandomRanged` calls in DestructionEffects + the unit-vec byte in SpawnSurvivors). All must use sim RNG. State hash must include `SmudgeGrid`.
2. **OverlayGrid double-write site.** Crater spawn calls `OverlayGrid::reduce_tiberium(rx, ry, 6)` BEFORE attempting placement. Adds a new write site to `OverlayGrid` from the smudge dispatcher. Composes additively with existing warhead-Tiberium ore-destroy logic — matches gamemd.
3. **Render-layer ordering.** Smudges draw between terrain and entities. Existing render pipeline in [src/render/](src/render/) draws terrain → entities → cliff-redraw → UI. Must insert smudges in the right slot (after terrain, before entities) without breaking cliff-redraw which depends on the depth buffer state from the entity pass.
4. **Snapshot serialization.** `SmudgeGrid` joins the snapshotted set (`MEMORY.md` references the snapshot serialization plan). Adds one more grid to the save/load API.

### Migration / backwards compatibility

None — net-new feature. No existing save files reference smudges.

## Chosen Approach

**Approach A from brainstorm:** spawn dispatcher inline at ExplosionEffect emission and at building-destruction handling. No AnimClass sim entity. Per-cell state in a dedicated `SmudgeGrid` mirroring `OverlayGrid`'s shape. Render reads SmudgeGrid each frame.

**Why A over B (sim entity for AnimClass) or C (warhead-driven):**

- A reuses existing patterns (OverlayGrid, dead-entities loop, ExplosionEffect emission point) without introducing a new sim entity category.
- The "first frame of anim" timing is implicit because we evaluate AnimType flags at the same sim tick where the warhead detonates and emits the anim — this matches gamemd's `AnimClass::Start` behavior.
- B (sim entity) would create one tracked entity per explosion, which scales poorly toward our 20k-unit / 30-player target and is justified only if we later need persistent sim-side anim state for something else.
- C (warhead-driven) cannot honor that the spawn flag lives on AnimType, not Warhead — distinguishable parity drift.

## Tiny-Detail Ledger

Implementation must preserve every item below. Each cites its source. Items 1-28 from the parent research; items 29-30 from the trade-off RE pass.

| # | Detail | Source |
|---|---|---|
| 1 | Trigger flags are on **AnimType**, not Warhead: `Scorch=` (+0x36B), `Crater=` (+0x36D), `ForceBigCraters=` (+0x36E) | SMUDGE_SPAWN_TRIGGERS §1 |
| 2 | Trigger fires on the FIRST frame of the anim, not on detonation, not on anim end | GHIDRA 0x00424F00 |
| 3 | Altitude gate: `(coord.z - ground_z) < 30` strictly less-than | GHIDRA 0x42505A |
| 4 | Both Scorch+Crater set → 50/50 random, exactly: `RandomRanged(0, 0x7FFFFFFE) * 2^-31 < 0.5` (constants 0x007E3570 / 0x007E1738) | GHIDRA 0x42507A-AB |
| 5 | Crater path ALWAYS calls `Reduce_Tiberium(6)` BEFORE attempting placement — ore destroyed even if `CanPlaceHere` later rejects | GHIDRA 0x004250E1-E7 |
| 6 | `Reduce_Tiberium(6)` uses immediate hardcoded 6, no INI key | GHIDRA 0x004250E5 |
| 7 | ForceBigCraters path passes hardcoded `(dmg=300, dmg2=300, forceBig=1)` regardless of anim sprite size | GHIDRA 0x00425104 |
| 8 | Non-ForceBig anim path passes `(dmg=AnimType+0x29C, dmg2=AnimType+0x2A0, forceBig=0)` — these are SHP frame width/height in pixels | GHIDRA 0x424F57-FE4 |
| 9 | Default frame dims when not yet cached: width AND height = 30 (0x1E). Eliminated by eager init at AnimType registry load. | GHIDRA 0x424F57; design decision below |
| 10 | One smudge per anim spawn at most — each branch returns after spawning | GHIDRA 0x004250CA, 0x0042511B, 0x00425141 |
| 11 | Crater spawn filters all SmudgeTypes by per-type `Crater=yes` (live filter, NOT a pre-built rules list) | GHIDRA 0x6B5C90 |
| 12 | Scorch spawn filters all SmudgeTypes by per-type `Burn=yes` (live filter) | GHIDRA 0x6B59A0 |
| 13 | Big/small threshold: `0x3C < dmg AND 0x32 < dmg2` (strictly less-than, dmg=60 fails, dmg=61 passes) | SMUDGE_CLASS §4.1 |
| 14 | `forceBig != 0` is a truthy-test, NOT `== 1` — gamemd's ForceBig path passes 300 and it works | GHIDRA 0x4250FE |
| 15 | If filtered list is empty after size filter, fall back to unfiltered Crater/Burn pool | SMUDGE_CLASS §4.1 |
| 16 | DestructionEffects fires only when foundation ≥ 2×2 | GHIDRA |
| 17 | DestructionEffects fires THREE RandomRanged calls before the smudge: `(0, W-2)` discarded, `(0, H-2)` discarded, `(0, 99)` is the actual roll. Discards are RNG-state advances — must replicate. | GHIDRA |
| 18 | DestructionEffects spawns 1 forceBig smudge at building center `(cell_X*256+128, cell_Y*256+128, building.Z)` with `(dmg=100, dmg2=100, forceBig=1)` | GHIDRA |
| 19 | SpawnSurvivors per-cell uses `RandomRanged(0, 99) < 50` and a per-cell random offset of magnitude `0x80` leptons via the unit-vec helper | GHIDRA 0x4432F2, 0x443387 |
| 20 | SpawnSurvivors call passes `(100, 100, 0)` — forceBig=0 but threshold met (100>60, 100>50) so big smudges ARE selectable | GHIDRA 0x44330A-0x443358 |
| 21 | `[Smudge]` map entry format `Key=TYPENAME,X,Y,IsBaked`. **IsBaked != 0 → entry SKIPPED** (not loaded) | audit W1, GHIDRA 0x6B4C80 |
| 22 | Map-load coord: `(X*256+128, Y*256+128, 0)` cell center, ground level | SMUDGE_CLASS §4.4 |
| 23 | Per-cell gates: in-bounds, no existing smudge, no overlay, no building, slope==0 (flat ground only), tileset accepts smudge | audit M1+M2, SMUDGE_CLASS §5 |
| 24 | All cells in the W×H footprint must pass — partial fits fail | SMUDGE_CLASS §5 |
| 25 | The DAT_00B0B788/8A "dedup globals" are zero-init only and never written. Repeat-hit prevention is via the `Cell+0x48 != -1` check inside CanPlaceHere. **Do NOT implement a separate global dedup.** | audit W2 |
| 26 | Render is one composite SHP frame per SmudgeType. `SmudgeTypeClass::Draw_It` calls `CC_Draw_Shape` with frame index 0 always; the per-cell screen offsets `((y-x)*30, (y+x)*-15)` cancel back to footprint origin so all cells of a multi-cell footprint draw at the same screen pixels. Our render emits one SpriteInstance per footprint origin cell (visually identical pixels, fewer draw calls). No animation, no facing, no remap. **Corrects earlier ledger entry that claimed `frame index = cell offset within W×H grid` — that's the screen-shift index, not an SHP frame index.** | SMUDGE_CLASS §7 + Ghidra `SmudgeTypeClass::Draw_It @ 0x006B55F0` (verified 2026-05-07) |
| 27 | SmudgeType `Width=` / `Height=` are CELL counts, not pixel counts. Don't conflate with anim sprite width/height (which ARE pixels). | SMUDGE_CLASS §1, ledger #9 |
| 28 | SmudgeGrid included in `world_hash` — visual divergence between replays is jarring | SMUDGE_CLASS §9 |
| 29 | **Smudge placement gate keys on `IsoTileTypeClass.Morphable=yes`.** Per-TileSet bool, default `false`, parsed from `[TileSetNNNN] Morphable=` in theater INI (temperatmd.ini etc.). Stored at +0x2E0. | ISOMETRIC_TILE_TYPE_CLASS §3.3, SMUDGE_SPAWN_TRIGGERS §11.1 |
| 30 | **SpawnSurvivors per-cell offset is uniform-random angle, fixed magnitude.** 1 RNG byte → angle table → unit vector × 128 leptons. Caller cell-snaps via `>> 8` then `*256+128`. Net effect: random pick of foundation cell or 1-cell-neighbor (smudges scatter beyond foundation). | SMUDGE_SPAWN_TRIGGERS §11.2 |

## Design

### Components

#### 1. `src/rules/smudge_type.rs` — SmudgeTypeRegistry

```rust
pub struct SmudgeTypeDef {
    pub name: String,
    pub crater: bool,
    pub burn: bool,
    pub width: u8,
    pub height: u8,
    pub image_name: Option<String>,
    pub is_theater: bool,
}

pub struct SmudgeTypeRegistry {
    types: Vec<SmudgeTypeDef>,
    by_name: HashMap<String, u16>,
}

impl SmudgeTypeRegistry {
    pub fn from_rules_ini(ini: &IniFile) -> Self;
    pub fn get(&self, id: u16) -> Option<&SmudgeTypeDef>;
    pub fn find_by_name(&self, name: &str) -> Option<u16>;
    pub fn iter_with_id(&self) -> impl Iterator<Item = (u16, &SmudgeTypeDef)>;
}
```

`u16` ID is the registry index (= gamemd's `Cell+0x48` value). Indices are stable within a session.

Wired into `RuleSet` as `pub smudge_types: SmudgeTypeRegistry`.

#### 2. `src/rules/anim_type.rs` extensions

Add three bool fields and two u16 frame-dim fields:

```rust
pub struct AnimType {
    // ...existing fields...
    pub scorch: bool,
    pub crater: bool,
    pub force_big_craters: bool,
    pub frame_width: u16,
    pub frame_height: u16,
}
```

Defaults: all bools `false`, both u16 = 0 (replaced at registry-load with eager SHP frame-rect lookup).

**Eager frame-dim init.** During `AnimType` registry construction, after the SHP file is loaded for an anim, read frame 0's bounding rect and store the width/height. This eliminates ledger item #9's "default 30 fallback" — the gamemd lazy-cache pattern is replaced with eager init because we already have the SHP data at that point. Open Q1 from research is disposed.

If the SHP cannot be loaded (rare), fall back to `(30, 30)` to match gamemd's default.

#### 3. `src/map/theater.rs` extensions

Parse `Morphable=` per TileSet:

```rust
pub struct Tileset {
    // ...existing fields...
    pub morphable: bool,
}
```

Default `false`. Read with `ini.get_bool(section, "Morphable").unwrap_or(false)`.

#### 4. `src/map/resolved_terrain.rs` extensions

Add `accepts_smudge: bool` to `ResolvedTerrainCell`. Computed at resolve time from the cell's tileset's `morphable` field.

#### 5. `src/map/map_file.rs` extensions

Parse `[Smudge]` section. Format: `Key=TYPENAME,X,Y,IsBaked`.

```rust
pub struct MapSmudgeEntry {
    pub type_name: String,
    pub rx: u16,
    pub ry: u16,
}

pub struct MapFile {
    // ...existing fields...
    pub smudges: Vec<MapSmudgeEntry>,
}
```

Skip entries where `IsBaked != 0` (ledger #21). If TYPENAME doesn't resolve to a registered SmudgeType at sim-init time, log a warning and skip.

#### 6. `src/sim/smudge_grid.rs` — SmudgeGrid

Mirrors `OverlayGrid`'s shape (Vec-backed flat grid).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default,
         serde::Serialize, serde::Deserialize)]
pub struct SmudgeCell {
    pub type_id: Option<u16>,
    pub footprint_origin: Option<(u16, u16)>,
    pub frame_offset: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmudgeGrid {
    width: u16,
    height: u16,
    cells: Vec<SmudgeCell>,
    #[serde(skip, default)]
    dirty_cells: Vec<(u16, u16)>,
}
```

`type_id` = registry index. `footprint_origin` = top-left cell of the W×H footprint that owns this cell (so each cell of a multi-cell smudge knows which "parent" footprint it belongs to). `frame_offset` = `(rx - origin.rx) + (ry - origin.ry) * footprint_width` — the SHP frame index for this cell within the footprint.

##### API

```rust
impl SmudgeGrid {
    pub fn new(width: u16, height: u16) -> Self;

    pub fn from_map_entries(
        entries: &[MapSmudgeEntry],
        registry: &SmudgeTypeRegistry,
        terrain: &ResolvedTerrainGrid,
        overlay: &OverlayGrid,
        width: u16, height: u16,
    ) -> Self;

    pub fn cell(&self, rx: u16, ry: u16) -> &SmudgeCell;

    pub fn try_place(
        &mut self,
        kind: SmudgeKind,
        coord: SimCoord,
        dmg: i32, dmg2: i32, force_big: bool,
        registry: &SmudgeTypeRegistry,
        terrain: &ResolvedTerrainGrid,
        overlay: &OverlayGrid,
        occupancy: &OccupancyGrid,
        rng: &mut SimRng,
    ) -> bool;

    pub fn iter_occupied(&self) -> impl Iterator<Item = (u16, u16, &SmudgeCell)>;
    pub fn drain_dirty(&mut self) -> Vec<(u16, u16)>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmudgeKind { Burn, Crater }
```

`try_place` internally:
1. Compute `(rx, ry)` from `coord.x >> 8`, `coord.y >> 8`.
2. Build candidate list: iterate `registry.iter_with_id()`, keep types matching `kind` flag (`crater` for `Kind::Crater`, `burn` for `Kind::Burn`).
3. Apply size filter:
   - `force_big == true`: keep only `width >= 2 && height >= 2`.
   - `force_big == false`: keep `width == 1 && height == 1` OR `(0x3C < dmg AND 0x32 < dmg2)`.
4. If filtered list is empty: fall back to step 2's unfiltered list (ledger #15).
5. Random pick: `let idx = rng.gen_range(0..filtered.len()); let chosen = filtered[idx];`.
6. Run `can_place_here(rx, ry, chosen.width, chosen.height, terrain, overlay, occupancy)`:
   - All cells in `[rx..rx+W] × [ry..ry+H]` in-bounds
   - For every cell: `self.cells[idx].type_id.is_none()`, `overlay.cell(rx', ry').overlay_id.is_none()`, no building occupant in `occupancy`, `terrain.cell(rx', ry').slope_type == 0`, `terrain.cell(rx', ry').accepts_smudge`
7. If passes: write footprint cells with `type_id = Some(chosen_id)`, `footprint_origin = Some((rx, ry))`, `frame_offset = computed`, push to `dirty_cells`. Return `true`.
8. Else: return `false`.

#### 7. `src/sim/combat/smudge_dispatch.rs` — Dispatcher + unit-vec table

```rust
pub static UNIT_VEC_TABLE: OnceLock<[(i32, i32); 256]> = OnceLock::new();

fn unit_vec_for_byte(b: u8) -> (i32, i32) {
    UNIT_VEC_TABLE.get_or_init(build_unit_vec_table)[b as usize]
}

fn build_unit_vec_table() -> [(i32, i32); 256] {
    let mut t = [(0i32, 0i32); 256];
    for b in 0u32..256 {
        let raw = ((b << 8) as i16) as i32 - 0x3FFF;
        let angle = raw as f64 * (-std::f64::consts::PI / 32768.0);
        let sin = angle.sin();
        let cos = angle.cos();
        t[b as usize] = (
            (sin * 65536.0).round() as i32,
            (-cos * 65536.0).round() as i32,
        );
    }
    t
}

fn random_offset_at_radius(rng: &mut SimRng, magnitude_leptons: i32) -> (i32, i32) {
    let b = rng.next_byte();
    let (sin_q16, neg_cos_q16) = unit_vec_for_byte(b);
    let dx = (sin_q16 as i64 * magnitude_leptons as i64) >> 16;
    let dy = (neg_cos_q16 as i64 * magnitude_leptons as i64) >> 16;
    (dx as i32, dy as i32)
}
```

The table is built once at engine init using f64 — runtime is pure integer Q16.16 multiply. Deterministic across machines because the table is computed eagerly and stored as i32, never recomputed.

##### Three dispatcher entry points

```rust
pub fn try_dispatch_anim_smudge(
    rules: &RuleSet,
    anim_type_id: u16,
    coord: SimCoord,
    ground_z: i32,
    smudge_grid: &mut SmudgeGrid,
    overlay_grid: &mut OverlayGrid,
    occupancy: &OccupancyGrid,
    terrain: &ResolvedTerrainGrid,
    rng: &mut SimRng,
) {
    let Some(anim) = rules.anim_types.get(anim_type_id) else { return; };

    if (coord.z - ground_z) >= 30 { return; }

    let dmg_w = anim.frame_width as i32;
    let dmg_h = anim.frame_height as i32;

    if anim.scorch {
        if !anim.crater {
            smudge_grid.try_place(
                SmudgeKind::Burn, coord, dmg_w, dmg_h, false,
                &rules.smudge_types, terrain, overlay_grid, occupancy, rng,
            );
            return;
        }
        if rng.gen_below_half_normalized() {
            smudge_grid.try_place(
                SmudgeKind::Burn, coord, dmg_w, dmg_h, false,
                &rules.smudge_types, terrain, overlay_grid, occupancy, rng,
            );
            return;
        }
    }
    if anim.crater {
        let (rx, ry) = (coord.x >> 8, coord.y >> 8);
        overlay_grid.reduce_tiberium(rx as u16, ry as u16, 6);
        if anim.force_big_craters {
            smudge_grid.try_place(
                SmudgeKind::Crater, coord, 300, 300, true,
                &rules.smudge_types, terrain, overlay_grid, occupancy, rng,
            );
        } else {
            smudge_grid.try_place(
                SmudgeKind::Crater, coord, dmg_w, dmg_h, false,
                &rules.smudge_types, terrain, overlay_grid, occupancy, rng,
            );
        }
    }
}

pub fn try_dispatch_building_destruction_smudges(
    rx: u16, ry: u16, building_z: i32,
    foundation_w: u8, foundation_h: u8,
    rules: &RuleSet,
    smudge_grid: &mut SmudgeGrid,
    overlay_grid: &mut OverlayGrid,
    occupancy: &OccupancyGrid,
    terrain: &ResolvedTerrainGrid,
    rng: &mut SimRng,
) {
    if foundation_w < 2 || foundation_h < 2 { return; }

    let _discard1 = rng.gen_range(0..=foundation_w as i32 - 2);
    let _discard2 = rng.gen_range(0..=foundation_h as i32 - 2);
    let roll = rng.gen_range(0..100);
    let center = SimCoord {
        x: (rx as i32) * 256 + 128,
        y: (ry as i32) * 256 + 128,
        z: building_z,
    };

    let kind = if roll < 50 { SmudgeKind::Burn } else { SmudgeKind::Crater };
    if matches!(kind, SmudgeKind::Crater) {
        overlay_grid.reduce_tiberium(rx, ry, 6);
    }
    smudge_grid.try_place(
        kind, center, 100, 100, true,
        &rules.smudge_types, terrain, overlay_grid, occupancy, rng,
    );
}

pub fn try_dispatch_building_survivor_smudges(
    foundation_cells: &[(u16, u16)],
    rules: &RuleSet,
    smudge_grid: &mut SmudgeGrid,
    overlay_grid: &mut OverlayGrid,
    occupancy: &OccupancyGrid,
    terrain: &ResolvedTerrainGrid,
    path_grid: &PathGrid,
    rng: &mut SimRng,
) {
    for &(cell_rx, cell_ry) in foundation_cells {
        if !path_grid.is_cell_passable(cell_rx, cell_ry) { continue; }

        let roll = rng.gen_range(0..100);
        let (dx, dy) = random_offset_at_radius(rng, 0x80);
        let base_x = (cell_rx as i32) * 256 + 128;
        let base_y = (cell_ry as i32) * 256 + 128;
        let off_x = base_x + dx;
        let off_y = base_y + dy;
        let snap_rx = (off_x >> 8).clamp(0, terrain.width() as i32 - 1) as u16;
        let snap_ry = (off_y >> 8).clamp(0, terrain.height() as i32 - 1) as u16;
        let coord = SimCoord {
            x: (snap_rx as i32) * 256 + 128,
            y: (snap_ry as i32) * 256 + 128,
            z: 0,
        };

        let kind = if roll < 50 { SmudgeKind::Burn } else { SmudgeKind::Crater };
        if matches!(kind, SmudgeKind::Crater) {
            overlay_grid.reduce_tiberium(snap_rx, snap_ry, 6);
        }
        smudge_grid.try_place(
            kind, coord, 100, 100, false,
            &rules.smudge_types, terrain, overlay_grid, occupancy, rng,
        );
    }
}
```

#### 8. `src/sim/combat/mod.rs` integration

Two integration points:

**A. ExplosionEffect emission** ([src/sim/combat/mod.rs:535](src/sim/combat/mod.rs#L535)). After pushing the `ExplosionEffect`, call `try_dispatch_anim_smudge` with the same anim type and coord. The anim type already comes from `wh.anim_list[idx]` which we resolve via `rules.anim_type(name)`.

**B. Dead-entities loop** ([src/sim/combat/mod.rs:420](src/sim/combat/mod.rs#L420)). For Structure entities, after the existing destruction effects (ejection, AOE):
1. Look up foundation `(W, H)` from the building type.
2. Call `try_dispatch_building_destruction_smudges` with center cell + dimensions.
3. Compute foundation cells, call `try_dispatch_building_survivor_smudges`.

#### 9. `src/sim/world/mod.rs` integration

```rust
pub struct World {
    // ...existing fields...
    pub smudge_grid: SmudgeGrid,
}
```

Init: `SmudgeGrid::from_map_entries(map.smudges, &rules.smudge_types, &resolved_terrain, &overlay_grid, w, h)`.

#### 10. `src/sim/world/world_hash.rs` integration

```rust
fn hash_smudge_grid(&self, hasher: &mut impl Hasher) {
    let mut entries: Vec<_> = self.smudge_grid.iter_occupied()
        .map(|(rx, ry, c)| (rx, ry, c.type_id, c.frame_offset))
        .collect();
    entries.sort();
    entries.hash(hasher);
}
```

#### 11. `src/render/smudge.rs` — render layer

Reads `&SmudgeGrid`, `&SmudgeTypeRegistry`, theater for SHP filenames. Generates `SmudgeInstance` per visible cell. Drawn between terrain pass and entity pass; no depth write (passthrough), follows existing entity-sprite shader pattern.

```rust
pub struct SmudgeInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub depth: f32,
}

pub fn build_visible_instances(
    grid: &SmudgeGrid,
    registry: &SmudgeTypeRegistry,
    atlas_lookup: &SmudgeAtlasLookup,
    camera_x: f32, camera_y: f32,
    screen_w: f32, screen_h: f32,
) -> Vec<SmudgeInstance>;
```

SHP loaded once per registered SmudgeType at registry construction. Theater-aware filename per the `is_theater` flag.

### Interfaces / Contracts

- **SmudgeGrid is owned by `World`.** Sim mutates via dispatcher; render reads via `&World`. Same boundary as OverlayGrid.
- **`AnimType.scorch / crater / force_big_craters` are read at dispatch time** via `&RuleSet`. AnimType registry is immutable post-load.
- **Tileset.morphable propagates one-way** to `ResolvedTerrainCell.accepts_smudge`. Set at terrain-resolve time, never mutated.
- **SimCoord** is the same lepton-space coord type used elsewhere (x, y, z in leptons; cell coord = `x >> 8`).

### Data Flow

```
LOAD-TIME:
  rulesmd.ini  → RuleSet { smudge_types, anim_types (with scorch/crater/force_big_craters,
                          frame_width/frame_height) }
  theater INI  → Tileset { morphable }  → ResolvedTerrainCell { accepts_smudge }
  map file     → MapFile { smudges }    → SmudgeGrid (seeded; IsBaked != 0 entries skipped)

RUNTIME (sim tick):
  WarheadType::Detonate path
    └─→ ExplosionEffect emit    + try_dispatch_anim_smudge(...)
                                       ├─ height gate
                                       ├─ scorch/crater branch (50/50 if both set)
                                       ├─ Reduce_Tiberium(6) [crater path]
                                       └─ SmudgeGrid::try_place → CanPlaceHere → write footprint cells

  Dead-entities loop (Structure)
    └─→ try_dispatch_building_destruction_smudges  (foundation ≥ 2×2)
    └─→ try_dispatch_building_survivor_smudges     (per cell)

RENDER (per frame):
  SmudgeGrid + SmudgeTypeRegistry + atlas → SmudgeInstance buffer → draw between terrain and entities
```

### Error Handling

- Unknown SmudgeType in `[Smudge]` map entry: warn-log + skip. Never panic.
- SHP file missing for SmudgeType: warn-log; smudge type stays in registry but `try_place` candidates are filtered out at render-pick time. (Determinism not affected because filter happens at render, not at sim's `try_place`.)
- Out-of-bounds coord: handled inside `try_place` via clamp/bounds check; dispatcher does not panic.
- Empty foundation (no passable cells): `try_dispatch_building_survivor_smudges` is a no-op for that building.

### Testing Strategy

#### Unit tests

1. `SmudgeTypeRegistry::from_rules_ini` — loads `[SmudgeTypes]` numeric list and per-name sections; defaults applied; bool flags parsed.
2. `SmudgeGrid::from_map_entries` — builds from MapSmudgeEntry list; IsBaked entries filtered (covered upstream in `MapFile` parse, but verify SmudgeGrid still works on a list that includes them).
3. `SmudgeGrid::try_place` — happy path (1×1 smudge on empty cell), W×H footprint, footprint partially blocked by overlay, slope != 0 rejection, Morphable=false rejection, force_big size filter.
4. `random_offset_at_radius` — table values: at byte=0, byte=64, byte=128, byte=192 verify the produced offsets match hand-computed `sin*magnitude` / `-cos*magnitude` to ±1 lepton.
5. Dispatcher: scorch-only AnimType emits Burn, crater-only emits Crater + Reduce_Tiberium, both-flags emits 50/50 (mock RNG to verify branch).
6. Altitude gate: `(coord.z - ground_z) == 30` returns no smudge; `== 29` does.

#### Integration tests

7. End-to-end: spawn an explosion via warhead detonation with `AnimList` containing a `Crater=yes` AnimType; verify SmudgeGrid gains a crater on the impact cell and OverlayGrid loses 6 ore on the same cell.
8. Building destruction: 4×4 building destroyed produces 1 forceBig smudge at center + up to 16 random-cell-pick smudges (per surviving cell). Verify the two `RandomRanged` discard calls happened (RNG counter advanced by exactly 3 per spawn).
9. Map-load: a map with `[Smudge]` entries (mix of IsBaked=0 and IsBaked=1) — only the unbaked ones appear in SmudgeGrid.

#### Determinism tests

10. Same seed + same combat sequence → identical SmudgeGrid hash across two runs.
11. Snapshot round-trip: serialize → deserialize → hash → match.

### Determinism Considerations

- All RNG draws use `world.rng` (the existing sim RNG).
- Three new draw points: filter random pick (1 draw per smudge), the two discarded RandomRanged calls in DestructionEffects (2 draws per ≥2×2 building), the unit-vec byte in SpawnSurvivors (1 draw per foundation cell).
- Total RNG advances per typical 4×4 building destruction: 1 (DestructionEffects roll) + 2 (discards) + 1 (its filter pick) + 16 × (1 roll + 1 unit-vec byte + ~1 filter pick) ≈ 50 draws. All deterministic given seed.
- SmudgeGrid hashed as part of `world_hash`. Replay desync surfaces immediately.
- Unit-vec table built once at engine init using f64 cos/sin then frozen as i32. Identical across machines because the table is computed and stored — no per-call f64.

## Architectural Decisions

### A1. Approach A (inline dispatcher), not B (sim entity)

Recorded above. Justification: existing patterns (OverlayGrid, ExplosionEffect emission, dead-entities loop) cover all the hooks we need. Adding a sim entity for animations is a separate, larger refactor that smudges alone don't justify.

### A2. Eager AnimType frame-dim init, not lazy

Gamemd lazily caches SHP frame width/height into `AnimType+0x29C/+0x2A0` on first `AnimClass::Start` call (default 30 if uncached). We compute eagerly at registry load because we already have the SHP. This eliminates the "default 30 fallback" path and Open Question #1 from the research. Visually identical for any anim with a loaded SHP (the common case); strictly better for any anim where gamemd would have hit the lazy-cache window.

### A3. SmudgeGrid as flat `Vec<SmudgeCell>`, not BTreeMap

Mirrors `OverlayGrid`. Per-cell access is O(1) and the grid is dense enough that cache locality matters more than memory savings. Hash iteration uses `iter_occupied` filter.

### A4. Unit-vector table is global `OnceLock`, not per-World

Pure deterministic function of fixed inputs. No per-game state. Single shared table is correct.

### A5. Crater path's `Reduce_Tiberium(6)` runs unconditionally before placement

Matches gamemd ledger #5: the ore reduction happens even if `CanPlaceHere` later rejects the smudge. This means a crater-flagged anim destroys ore on overlay cells (where the smudge can't visually appear) too. Real gameplay effect, not a bug.

### Tech debt

None introduced. The two RE deferrals (vtable+0x6C resolver, and the discarded RandomRanged interpretation) are accepted as resolved by user direction.

## Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **B. Sim entity for AnimClass** with first-frame hook | Larger refactor than smudges justify; entity count grows per explosion; no other near-term feature needs it |
| **C. Smudges keyed off WarheadType flags** instead of AnimType | Cannot honor that gamemd actually keys on AnimType (a warhead's AnimList can mix scorch-flagged and non-scorch-flagged anims); produces visible parity drift |
| **LandType heuristic** for accepts-smudge instead of `Morphable` flag (brainstorm Trade-off #1 option a) | Resolved by RE: the actual flag is `Morphable=` per TileSet, exact INI mapping verified |
| **Uniform-square random offset** for SpawnSurvivors instead of unit-vector × magnitude (brainstorm Trade-off #2 option a) | Resolved by RE: actual semantic is uniform random angle × fixed magnitude, with cell-snap downstream — produces a different cell-distribution than uniform-square |
| **Pre-built `Scorches/Scorches1..4` lists from `[CombatDamage]`** | Verified TS-legacy dead code in YR (zero xrefs to RulesClass+0x7D4..+0x84C). Don't implement. |
| **Global `(last_X, last_Y)` dedup** per smudge doc §6 | Verified dead code (audit W2). Real dedup is `Cell.smudge_id.is_none()` inside `can_place_here`. Don't implement. |

## Out of scope (deferred follow-ups)

- **Render path for the smudge SHP atlas** — needs to share or extend the existing sprite-atlas infrastructure. May need a new atlas page if the existing one is full (per `feedback_multi_atlas`).
- **Theater-variant SHP filenames** — handled by the same theater-aware filename helper that overlays use; verify it covers smudge types correctly.
- **Tile-anim spawning from `Tile%dAnim` keys** — flagged in the IsoTileTypeClass doc but unrelated to smudges; separate brainstorm.
