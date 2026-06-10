//! Native-shaped CellRect passability and occupancy validators.
//!
//! These are read-only substrate facades over the existing Rust grids. They do
//! not collapse terrain, overlays, reservations, and object-list occupancy into
//! one store; the point is to expose the two distinct gamemd validator surfaces
//! while preserving the current Rust-native ownership split.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::passability;
use crate::sim::pathfinding::zone_map::{ZoneGrid, ZoneId};

/// Fixed cell-array stride — the engine indexes cells `y*0x200 + x` regardless of
/// the loaded map's playfield width. The valid linear range is `[0, MAX_CELL_INDEX]`.
/// This is NOT the loaded-map width index (that is `PathGrid`'s `y*width+x` cache).
pub const CELL_ROW_STRIDE: i64 = 0x200;
/// Highest valid linear cell index under the fixed 512-wide stride.
pub const MAX_CELL_INDEX: i64 = 0x3FFFF;

/// Linear cell index using the fixed 512-wide stride (NOT the loaded-map width).
///
/// Returns `None` only when the index falls outside `[0, MAX_CELL_INDEX]`; the
/// dummy fallback (`get_cellclass_fallback`) turns that `None` into a non-null
/// reference, mirroring the engine's never-null `Get_CellClass`.
pub fn cell_linear_index(x: i32, y: i32) -> Option<i64> {
    let idx = (y as i64) * CELL_ROW_STRIDE + (x as i64);
    (0..=MAX_CELL_INDEX).contains(&idx).then_some(idx)
}

/// A non-null cell reference — `Real` for an in-range, present cell, or `Dummy`
/// carrying the requested coord for an out-of-range / missing lookup.
///
/// Never the absence of a value: the engine's coord→cell lookup returns a
/// non-null dummy that stores the requested coord and lets the caller keep
/// dispatching on it. The dummy carries only the coord; its other field values
/// are an open RE item and must NOT be read until that lands.
#[derive(Debug, Clone, Copy)]
pub enum CellRef<'a> {
    Real(&'a ResolvedTerrainCell),
    Dummy { coord: (i32, i32) },
}

// `ResolvedTerrainCell` is not `PartialEq`; compare `Real` by pointer identity
// (same backing cell) and `Dummy` by the coord it carries. This is enough for the
// facade's only need: distinguishing the dummy fallback and asserting its coord.
impl PartialEq for CellRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CellRef::Real(a), CellRef::Real(b)) => std::ptr::eq(*a, *b),
            (CellRef::Dummy { coord: a }, CellRef::Dummy { coord: b }) => a == b,
            _ => false,
        }
    }
}
impl Eq for CellRef<'_> {}

/// Engine `Get_CellClass`: coord → cell via the fixed stride; an out-of-range or
/// missing cell returns `CellRef::Dummy { coord }` carrying the *requested* coord
/// (NOT `(0,0)`, NOT `None`). The width-based `PathGrid`/`ResolvedTerrainGrid`
/// index stays as the cache; this is the never-null parity lookup.
pub fn get_cellclass_fallback<'a>(
    terrain: Option<&'a ResolvedTerrainGrid>,
    x: i32,
    y: i32,
) -> CellRef<'a> {
    if cell_linear_index(x, y).is_some() {
        if let (Ok(rx), Ok(ry)) = (u16::try_from(x), u16::try_from(y)) {
            if let Some(cell) = terrain.and_then(|t| t.cell(rx, ry)) {
                return CellRef::Real(cell);
            }
        }
    }
    CellRef::Dummy { coord: (x, y) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl CellRect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn single(rx: u16, ry: u16) -> Self {
        Self::new(rx as i32, ry as i32, 1, 1)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CellReservationGrid {
    masks: BTreeMap<(u16, u16), u32>,
}

impl CellReservationGrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reserve(&mut self, rx: u16, ry: u16, reservation_arg: i32) {
        let mask = reservation_mask(reservation_arg);
        if mask != 0 {
            *self.masks.entry((rx, ry)).or_default() |= mask;
        }
    }

    pub fn clear(&mut self, rx: u16, ry: u16, reservation_arg: i32) {
        let mask = reservation_mask(reservation_arg);
        if mask == 0 {
            return;
        }
        if let Some(bits) = self.masks.get_mut(&(rx, ry)) {
            *bits &= !mask;
            if *bits == 0 {
                self.masks.remove(&(rx, ry));
            }
        }
    }

    pub fn has_reservation(&self, rx: u16, ry: u16, reservation_arg: i32) -> bool {
        let mask = reservation_mask(reservation_arg);
        mask != 0
            && self
                .masks
                .get(&(rx, ry))
                .is_some_and(|bits| bits & mask != 0)
    }
}

pub struct CellRectPassabilityContext<'a> {
    pub rect: CellRect,
    pub speed_type: SpeedType,
    pub required_zone_id: Option<ZoneId>,
    pub movement_zone: MovementZone,
    pub required_height_or_level: Option<i16>,
    pub bridge_aware_zone: bool,
    pub reject_any_overlay: bool,
    pub path_grid: Option<&'a PathGrid>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub overlay_grid: Option<&'a OverlayGrid>,
    pub occupancy: Option<&'a OccupancyGrid>,
    pub zone_grid: Option<&'a ZoneGrid>,
}

pub struct CellRectOccupancyContext<'a> {
    pub rect: CellRect,
    pub reservation_arg: i32,
    pub reservations: Option<&'a CellReservationGrid>,
    pub occupancy: Option<&'a OccupancyGrid>,
    pub entities: Option<&'a EntityStore>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub overlay_grid: Option<&'a OverlayGrid>,
    pub map_size: Option<(u16, u16)>,
    /// The map's isometric-diamond playfield bounds, when available. When present,
    /// the playfield-corner test uses the exact diamond formula
    /// (`rect_in_playfield_diamond`); when `None`, the test falls back to the
    /// `map_size` rectangle — a non-authoritative convenience for callers that have
    /// no diamond bounds yet (NOT the engine's shape).
    pub playfield_bounds: Option<PlayfieldBounds>,
}

/// The five map bound values that define the engine's isometric playfield diamond,
/// read by `cell_in_playfield_diamond`.
///
/// Field meanings verified 2026-06-04 (see
/// `docs/research/CELLCLASS_PLAYFIELD_BOUNDS_FROM_LOCALSIZE_GHIDRA_REPORT.md`): `base` is the map's
/// `[Map] Size=` width; the other four are the raw `[Map] LocalSize=` values (left, top, width, height)
/// stored verbatim — there is no transform here. The `*2` doubling and the `+2`/`+4` constants live
/// entirely in `cell_in_playfield_diamond`. All five are signed map-coord values. The `off_*` field
/// names are legacy (named after their source struct offsets) and kept to avoid a rename churn across
/// the tests and the diamond fn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayfieldBounds {
    /// `[Map] Size=` width (3rd value).
    pub base: i32,
    /// `[Map] LocalSize=` left (1st value).
    pub off_fc: i32,
    /// `[Map] LocalSize=` top (2nd value).
    pub off_100: i32,
    /// `[Map] LocalSize=` width (3rd value).
    pub off_104: i32,
    /// `[Map] LocalSize=` height (4th value).
    pub off_108: i32,
}

pub fn check_passability_rect(ctx: CellRectPassabilityContext<'_>) -> bool {
    if ctx.rect.width <= 0 || ctx.rect.height <= 0 {
        return true;
    }

    let mut x = 0;
    while x < ctx.rect.width {
        let mut y = 0;
        while y < ctx.rect.height {
            let cx = ctx.rect.x.saturating_add(x);
            let cy = ctx.rect.y.saturating_add(y);
            if !check_cell_passability(&ctx, cx, cy) {
                return false;
            }
            y += 1;
        }
        x += 1;
    }
    true
}

pub fn check_occupancy_rect(ctx: CellRectOccupancyContext<'_>) -> bool {
    let mask = reservation_mask(ctx.reservation_arg);

    if ctx.rect.width > 0 && ctx.rect.height > 0 {
        let mut x = 0;
        while x < ctx.rect.width {
            let mut y = 0;
            while y < ctx.rect.height {
                let cx = ctx.rect.x.saturating_add(x);
                let cy = ctx.rect.y.saturating_add(y);
                let Some((rx, ry)) = to_cell_coord(cx, cy) else {
                    return false;
                };

                if terrain_object_blocks(ctx.resolved_terrain, rx, ry) {
                    return false;
                }
                if mask != 0
                    && ctx.reservations.is_some_and(|reservations| {
                        reservations.has_reservation(rx, ry, ctx.reservation_arg)
                    })
                {
                    return false;
                }
                if overlay_present(ctx.overlay_grid, rx, ry) {
                    return false;
                }
                // The engine scans two separate per-cell columns in this order:
                // (d) the reduced-ZoneType column (column 0 == Ground passes), then
                // (e) the slope/special byte. They are split — not fused — so the
                // first-blocker scan order is reproduced even though both reject the
                // same way; a cell with only a slope and a cell with only a non-Ground
                // zone-type each reject independently.
                let tcell = ctx.resolved_terrain.and_then(|terrain| terrain.cell(rx, ry));
                if tcell.is_some_and(|cell| cell.zone_type != zone_class::GROUND) {
                    return false;
                }
                if tcell.is_some_and(|cell| cell.slope_type != 0) {
                    return false;
                }
                if ground_building_present(ctx.occupancy, ctx.entities, rx, ry) {
                    return false;
                }

                y += 1;
            }
            x += 1;
        }
    }

    rect_in_playfield(
        ctx.rect,
        ctx.playfield_bounds,
        ctx.resolved_terrain,
        ctx.map_size
            .or_else(|| {
                ctx.resolved_terrain
                    .map(|terrain| (terrain.width(), terrain.height()))
            })
            .or_else(|| ctx.overlay_grid.map(|grid| (grid.width(), grid.height()))),
    )
}

fn check_cell_passability(ctx: &CellRectPassabilityContext<'_>, x: i32, y: i32) -> bool {
    let Some((rx, ry)) = to_cell_coord(x, y) else {
        return false;
    };

    if ctx.reject_any_overlay && overlay_present(ctx.overlay_grid, rx, ry) {
        return false;
    }

    if ctx.speed_type == SpeedType::Winged {
        return true;
    }

    let terrain_cell = ctx
        .resolved_terrain
        .and_then(|terrain| terrain.cell(rx, ry));
    let path_cell = ctx.path_grid.and_then(|grid| grid.cell(rx, ry));
    if terrain_cell.is_none() && path_cell.is_none() {
        return false;
    }

    if let Some(required_zone) = ctx.required_zone_id {
        let Some(zone_grid) = ctx.zone_grid else {
            return false;
        };
        let Some(zone_map) = zone_grid.map_for(ctx.movement_zone) else {
            return false;
        };
        let layer = if ctx.bridge_aware_zone {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        if zone_map.zone_at(rx, ry, layer) != required_zone {
            return false;
        }
    }

    let base_level = path_cell
        .map(|cell| cell.signed_level())
        .or_else(|| terrain_cell.map(|cell| cell.level as i8 as i16))
        .unwrap_or(0);
    let structural_bridge = path_cell.is_some_and(|cell| cell.has_structural_bridge())
        || terrain_cell.is_some_and(|cell| cell.bridge_facts.has_structural_bridge());

    let selected_bridge_layer = match ctx.required_height_or_level {
        Some(required) if required == base_level => {
            if structural_bridge && !ctx.bridge_aware_zone {
                return false;
            }
            false
        }
        Some(required) => {
            if !structural_bridge || required != base_level.saturating_add(4) {
                return false;
            }
            true
        }
        None => structural_bridge,
    };

    let occupation_layer = if selected_bridge_layer {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    };
    if ctx
        .occupancy
        .is_some_and(|occupancy| occupancy.count_on_layer(rx, ry, occupation_layer) > 0)
    {
        return false;
    }

    if selected_bridge_layer {
        return true;
    }

    terrain_cell.map_or_else(
        || ctx.path_grid.map_or(true, |grid| grid.is_walkable(rx, ry)),
        |cell| speed_type_allows_cell(cell, ctx.speed_type, ctx.movement_zone),
    )
}

fn speed_type_allows_cell(
    cell: &ResolvedTerrainCell,
    speed_type: SpeedType,
    movement_zone: MovementZone,
) -> bool {
    if cell.zone_type == zone_class::WALL {
        return matches!(
            movement_zone,
            MovementZone::Destroyer
                | MovementZone::AmphibiousDestroyer
                | MovementZone::InfantryDestroyer
                | MovementZone::CrusherAll
        );
    }
    if let Some(cost) = cell.speed_costs.cost_for_speed_type(speed_type) {
        return cost > 0;
    }
    passability::is_passable_for_speed_type(cell.land_type, speed_type)
}

fn terrain_object_blocks(resolved_terrain: Option<&ResolvedTerrainGrid>, rx: u16, ry: u16) -> bool {
    resolved_terrain
        .and_then(|terrain| terrain.cell(rx, ry))
        .is_some_and(|cell| cell.terrain_object_blocks)
}

fn overlay_present(overlay_grid: Option<&OverlayGrid>, rx: u16, ry: u16) -> bool {
    overlay_grid
        .map(|grid| grid.cell(rx, ry).overlay_id.is_some())
        .unwrap_or(false)
}

fn ground_building_present(
    occupancy: Option<&OccupancyGrid>,
    entities: Option<&EntityStore>,
    rx: u16,
    ry: u16,
) -> bool {
    let (Some(occupancy), Some(entities)) = (occupancy, entities) else {
        return false;
    };
    occupancy.get(rx, ry).is_some_and(|cell| {
        cell.iter_layer(MovementLayer::Ground).any(|occupant| {
            entities
                .get(occupant.entity_id)
                .is_some_and(|entity| entity.category == EntityCategory::Structure)
        })
    })
}

/// Engine `IsRectInPlayfield`: test exactly four corners — NW `(x,y)`,
/// NE `(x+w-1, y)`, SW `(x, y+h-1)`, SE `(x+w-1, y+h-1)` — in that fixed order,
/// short-circuit AND, using INCLUSIVE `w-1`/`h-1` far edges. Each corner is judged
/// by the isometric diamond predicate (`cell_in_playfield_diamond`), NOT a
/// rectangular `0 <= x < width` index test.
///
/// A 0-size rect is NOT a no-op: with `width == 0` the NE/SE x become `x-1`, and
/// with `height == 0` the SW/SE y become `y-1`, so the corners are evaluated at
/// decremented coords and all four must still satisfy the diamond.
///
/// When `bounds` is `None` the function falls back to the `map_size` rectangle —
/// a non-authoritative convenience for callers that have no diamond bounds wired
/// yet. The diamond is the engine's shape; the rectangle is only a placeholder.
fn rect_in_playfield(
    rect: CellRect,
    bounds: Option<PlayfieldBounds>,
    terrain: Option<&ResolvedTerrainGrid>,
    map_size: Option<(u16, u16)>,
) -> bool {
    let max_x = rect.x.saturating_add(rect.width).saturating_sub(1);
    let max_y = rect.y.saturating_add(rect.height).saturating_sub(1);
    let corners = [
        (rect.x, rect.y),   // NW
        (max_x, rect.y),    // NE
        (rect.x, max_y),    // SW
        (max_x, max_y),     // SE
    ];

    if let Some(bounds) = bounds {
        return corners
            .into_iter()
            .all(|(sx, sy)| cell_in_playfield_diamond(sx, sy, &bounds, terrain));
    }

    // Fallback (no diamond bounds supplied): rectangular bounds. Not the engine's
    // shape — only used until callers thread real playfield bounds.
    let Some((width, height)) = map_size else {
        return true;
    };
    corners
        .into_iter()
        .all(|(x, y)| x >= 0 && y >= 0 && x < i32::from(width) && y < i32::from(height))
}

/// Engine `Is_Cell_In_Playfield` with `height_flag = 1` (the value the sole rect
/// caller passes): the isometric-diamond containment test for a single cell.
///
/// With `sx`, `sy` the cell's signed coords and `h` the height extension, the cell
/// passes iff its sum `sx+sy` lies in the half-open band `(base+LOW, base+HIGH]`
/// (low exclusive, high inclusive) AND both differences are strictly below their
/// bound:
/// - `(base + LOW)  <  (sx + sy)`            (strict low)
/// - `(sx + sy)     <= (base + HIGH)`        (inclusive high)
/// - `(sx - sy)     <  RIGHT`                (strict)
/// - `(sy - sx)     <  LEFT`                 (strict)
///
/// where `LOW = off_100*2 + h`, `HIGH = 2 + (off_108 + off_100)*2 + h`,
/// `RIGHT = (off_104 + off_fc)*2 - base`, `LEFT = base - off_fc*2`.
///
/// Height extension (height_flag = 1): `h = signed(cell.level)`; if the cell's slope
/// byte is nonzero AND `sx+sy < base + 4 + off_100*2 + h` then `h += 1`. An
/// out-of-grid cell contributes `h = 0` (flat).
fn cell_in_playfield_diamond(
    sx: i32,
    sy: i32,
    bounds: &PlayfieldBounds,
    terrain: Option<&ResolvedTerrainGrid>,
) -> bool {
    let base = bounds.base;

    // Height extension from the cell at (sx, sy). The cell level byte is read signed;
    // a nonzero slope byte bumps h by 1 when the cell sits below the slope threshold.
    let mut h = 0i32;
    if let (Ok(rx), Ok(ry)) = (u16::try_from(sx), u16::try_from(sy)) {
        if let Some(cell) = terrain.and_then(|t| t.cell(rx, ry)) {
            h = i32::from(cell.level as i8);
            if cell.slope_type != 0 && (sx + sy) < base + 4 + bounds.off_100 * 2 + h {
                h += 1;
            }
        }
    }

    let low = bounds.off_100 * 2 + h;
    let high = 2 + (bounds.off_108 + bounds.off_100) * 2 + h;
    let right = (bounds.off_104 + bounds.off_fc) * 2 - base;
    let left = base - bounds.off_fc * 2;

    let sum = sx + sy;
    (base + low) < sum && sum <= (base + high) && (sx - sy) < right && (sy - sx) < left
}

fn reservation_mask(reservation_arg: i32) -> u32 {
    if reservation_arg == -1 {
        0
    } else {
        1u32 << ((reservation_arg as u32) & 0x1F)
    }
}

fn to_cell_coord(x: i32, y: i32) -> Option<(u16, u16)> {
    if x < 0 || y < 0 || x > i32::from(u16::MAX) || y > i32::from(u16::MAX) {
        return None;
    }
    Some((x as u16, y as u16))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::map::bridge_facts::{BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::occupancy::CellListInsertion;
    use crate::sim::pathfinding::zone_map::ZoneGrid;

    fn terrain_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
            is_cliff_redraw: false,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            overlay_blocks: false,
            zone_type: zone_class::GROUND,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: TerrainClass::Clear,
            base_speed_costs: SpeedCostProfile::default(),
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn flat_terrain(width: u16, height: u16) -> ResolvedTerrainGrid {
        let cells = (0..height)
            .flat_map(|ry| (0..width).map(move |rx| terrain_cell(rx, ry)))
            .collect();
        ResolvedTerrainGrid::from_cells(width, height, cells)
    }

    #[test]
    fn cellrect_occupancy_minus_one_skips_reservation_but_rejects_cell_blockers() {
        let mut terrain = flat_terrain(3, 1);
        terrain.cells[1].slope_type = 2;
        let mut reservations = CellReservationGrid::new();
        reservations.reserve(0, 0, 3);

        let clear_reserved = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: -1,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
            playfield_bounds: None,
        };
        assert!(check_occupancy_rect(clear_reserved));

        let sloped = CellRectOccupancyContext {
            rect: CellRect::single(1, 0),
            reservation_arg: -1,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
            playfield_bounds: None,
        };
        assert!(!check_occupancy_rect(sloped));
    }

    #[test]
    fn cellrect_occupancy_house_reservation_blocks_same_house_only() {
        let terrain = flat_terrain(2, 1);
        let mut reservations = CellReservationGrid::new();
        reservations.reserve(0, 0, 5);

        let same_house = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: 5,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
            playfield_bounds: None,
        };
        assert!(!check_occupancy_rect(same_house));

        let other_house = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: 6,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
            playfield_bounds: None,
        };
        assert!(check_occupancy_rect(other_house));

        let skipped = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: -1,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
            playfield_bounds: None,
        };
        assert!(check_occupancy_rect(skipped));
    }

    #[test]
    fn cellrect_passability_uses_movement_zone_zone_id_and_speed_type_separately() {
        let mut terrain = flat_terrain(2, 1);
        terrain.cells[0].speed_costs = SpeedCostProfile {
            track: Some(100),
            foot: Some(0),
            ..Default::default()
        };
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let zone_grid = ZoneGrid::build_with_terrain(
            &path_grid,
            &BTreeMap::new(),
            Some(&terrain),
            &[],
            terrain.width(),
            terrain.height(),
        );
        let zone_id =
            zone_grid
                .map_for(MovementZone::Normal)
                .unwrap()
                .zone_at(0, 0, MovementLayer::Ground);

        let wrong_zone = CellRectPassabilityContext {
            rect: CellRect::single(0, 0),
            speed_type: SpeedType::Track,
            required_zone_id: Some(zone_id.saturating_add(1)),
            movement_zone: MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: false,
            reject_any_overlay: false,
            path_grid: Some(&path_grid),
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            occupancy: None,
            zone_grid: Some(&zone_grid),
        };
        assert!(!check_passability_rect(wrong_zone));

        let foot_speed_blocked = CellRectPassabilityContext {
            rect: CellRect::single(0, 0),
            speed_type: SpeedType::Foot,
            required_zone_id: Some(zone_id),
            movement_zone: MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: false,
            reject_any_overlay: false,
            path_grid: Some(&path_grid),
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            occupancy: None,
            zone_grid: Some(&zone_grid),
        };
        assert!(!check_passability_rect(foot_speed_blocked));

        let track_passes = CellRectPassabilityContext {
            rect: CellRect::single(0, 0),
            speed_type: SpeedType::Track,
            required_zone_id: Some(zone_id),
            movement_zone: MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: false,
            reject_any_overlay: false,
            path_grid: Some(&path_grid),
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            occupancy: None,
            zone_grid: Some(&zone_grid),
        };
        assert!(check_passability_rect(track_passes));
    }

    #[test]
    fn cellrect_passability_bridge_bits_are_not_occupancy_rect_blockers() {
        let mut terrain = flat_terrain(1, 1);
        terrain.cells[0].bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL;
        terrain.cells[0].has_bridge_deck = true;
        terrain.cells[0].bridge_walkable = true;
        terrain.cells[0].bridge_deck_level = 4;
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            0,
            0,
            10,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let passability = CellRectPassabilityContext {
            rect: CellRect::single(0, 0),
            speed_type: SpeedType::Track,
            required_zone_id: None,
            movement_zone: MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: true,
            reject_any_overlay: false,
            path_grid: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            occupancy: Some(&occupancy),
            zone_grid: None,
        };
        assert!(!check_passability_rect(passability));

        let occupancy_rect = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: -1,
            reservations: None,
            occupancy: Some(&occupancy),
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
            playfield_bounds: None,
        };
        assert!(check_occupancy_rect(occupancy_rect));
    }

    // --- T1: fixed-stride cell index + non-null dummy fallback ---

    #[test]
    fn cell_index_uses_512_wide_stride_not_map_width() {
        // (x=0, y=1) -> 0x200 under the fixed stride, regardless of any loaded width.
        assert_eq!(cell_linear_index(0, 1), Some(0x200));
        assert_eq!(cell_linear_index(1, 0), Some(1));
        // Out of the [0, 0x3FFFF] linear range -> None (then a dummy at the caller).
        assert_eq!(cell_linear_index(-1, 0), None);
    }

    #[test]
    fn get_cellclass_oob_returns_dummy_with_requested_coord() {
        let g = flat_terrain(2, 2);
        assert!(matches!(
            get_cellclass_fallback(Some(&g), 0, 0),
            CellRef::Real(_)
        ));
        // Out of bounds: a non-null dummy carrying the *requested* coord
        // (never None, never (0,0)).
        assert_eq!(
            get_cellclass_fallback(Some(&g), -3, 7),
            CellRef::Dummy { coord: (-3, 7) }
        );
    }

    // --- T2: passability shadow agreement + zero-size short-circuit ---

    #[test]
    fn passability_rect_shadow_agrees_with_pathgrid_on_plain_cells() {
        // On cells with no overlay/zone/height constraint, a 1x1 passability rect
        // must AGREE with PathGrid::is_walkable. Divergence is surfaced (the assert
        // names the cell), never equalized away.
        let terrain = flat_terrain(4, 4);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        for ry in 0..4u16 {
            for rx in 0..4u16 {
                let ctx = CellRectPassabilityContext {
                    rect: CellRect::single(rx, ry),
                    speed_type: SpeedType::Track,
                    required_zone_id: None,
                    movement_zone: MovementZone::Normal,
                    required_height_or_level: None,
                    bridge_aware_zone: false,
                    reject_any_overlay: false,
                    path_grid: Some(&path_grid),
                    resolved_terrain: Some(&terrain),
                    overlay_grid: None,
                    occupancy: None,
                    zone_grid: None,
                };
                assert_eq!(
                    check_passability_rect(ctx),
                    path_grid.is_walkable(rx, ry),
                    "passability/PathGrid divergence at ({rx},{ry})"
                );
            }
        }
    }

    #[test]
    fn passability_zero_size_rect_returns_true() {
        let terrain = flat_terrain(1, 1);
        let ctx = CellRectPassabilityContext {
            rect: CellRect::new(0, 0, 0, 0),
            speed_type: SpeedType::Track,
            required_zone_id: None,
            movement_zone: MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: false,
            reject_any_overlay: false,
            path_grid: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            occupancy: None,
            zone_grid: None,
        };
        // width<=0 -> true, no cell read.
        assert!(check_passability_rect(ctx));
    }

    // --- T3: occupancy blocker order + degenerate-rect corner check ---

    #[test]
    fn occupancy_blocker_order_matches_engine() {
        // The reduced-ZoneType column (d) and the slope/special byte (e) reject
        // independently: a cell with ONLY a slope rejects, and a cell with ONLY a
        // non-Ground zone-type rejects, each on its own column.
        let mut terrain = flat_terrain(3, 1);
        terrain.cells[1].slope_type = 2; // (e) only
        terrain.cells[2].zone_type = zone_class::WATER; // (d) only

        let clear = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: -1,
            reservations: None,
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
            playfield_bounds: None,
        };
        assert!(check_occupancy_rect(clear)); // clear cell passes

        let slope_only = CellRectOccupancyContext {
            rect: CellRect::single(1, 0),
            reservation_arg: -1,
            reservations: None,
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
            playfield_bounds: None,
        };
        assert!(!check_occupancy_rect(slope_only));

        let zone_only = CellRectOccupancyContext {
            rect: CellRect::single(2, 0),
            reservation_arg: -1,
            reservations: None,
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
            playfield_bounds: None,
        };
        assert!(!check_occupancy_rect(zone_only));
    }

    /// A diamond bounds fixture chosen so the playable region is a clean interior:
    /// pass iff `12 < sx+sy <= 26` AND `sx-sy < 14` AND `sy-sx < 6` (flat terrain,
    /// so the height extension `h = 0`). Derived from the resolved formula in
    /// `cell_in_playfield_diamond` with these five values:
    ///   base=10, off_fc=2, off_100=1, off_104=10, off_108=6
    ///   LOW=off_100*2 = 2; HIGH=2+(off_108+off_100)*2 = 16;
    ///   RIGHT=(off_104+off_fc)*2-base = 14; LEFT=base-off_fc*2 = 6;
    /// so base+LOW=12 (strict low), base+HIGH=26 (inclusive high).
    fn diamond_bounds() -> PlayfieldBounds {
        PlayfieldBounds {
            base: 10,
            off_fc: 2,
            off_100: 1,
            off_104: 10,
            off_108: 6,
        }
    }

    fn occupancy_with_bounds(rect: CellRect) -> CellRectOccupancyContext<'static> {
        CellRectOccupancyContext {
            rect,
            reservation_arg: -1,
            reservations: None,
            occupancy: None,
            entities: None,
            resolved_terrain: None,
            overlay_grid: None,
            map_size: None,
            playfield_bounds: Some(diamond_bounds()),
        }
    }

    #[test]
    fn rect_in_playfield_is_isometric_diamond_inclusive_four_corners() {
        // A 1x1 rect on the diamond's INCLUSIVE high edge of the sum band passes:
        // (13,13) has sum 26 == base+HIGH, and both diagonals are inside.
        assert!(check_occupancy_rect(occupancy_with_bounds(CellRect::single(13, 13))));
        // One cell past the high edge (sum 27 > 26) fails — proves the band is a
        // diamond on sx+sy, not a rectangle on raw x/y.
        assert!(!check_occupancy_rect(occupancy_with_bounds(CellRect::single(14, 13))));

        // A 2x1 rect whose NW corner (13,13) is inside but whose INCLUSIVE far
        // corner (x+w-1, y) = (14,13) leaves the diamond (sum 27) fails — proves the
        // far corner uses w-1 AND that the corner predicate is the diamond.
        assert!(!check_occupancy_rect(occupancy_with_bounds(CellRect::new(13, 13, 2, 1))));

        // A point inside both diagonals but with sum just above the strict low edge
        // (sum 13 > 12) passes; the same cell pair off the low edge (sum 12) fails.
        assert!(check_occupancy_rect(occupancy_with_bounds(CellRect::single(7, 6))));
        assert!(!check_occupancy_rect(occupancy_with_bounds(CellRect::single(6, 6))));
    }

    #[test]
    fn occupancy_zero_size_rect_still_runs_playfield_corners() {
        // A 0-size rect is NOT a no-op and NOT an auto-pass: with width=0/height=0 the
        // far corners become (x-1, y)/(x, y-1)/(x-1, y-1), so all four corners are
        // evaluated at DECREMENTED coords and each must still satisfy the diamond.
        //
        // At (13,13) the decremented corners (12,13)/(13,12)/(12,12) have sums
        // 25/25/24 — all inside (12 < sum <= 26) — so the 0-size rect PASSES.
        assert!(check_occupancy_rect(occupancy_with_bounds(CellRect::new(13, 13, 0, 0))));

        // At (7,6) the NW corner (sum 13) is inside, but the decremented NE corner
        // (6,6) has sum 12, which fails the strict low edge (12 < 12 is false). So the
        // 0-size rect FAILS even though its (undecremented) NW corner is inside —
        // exactly the engine's decremented-corner behavior. The corresponding 1x1
        // rect at (7,6) passes (its corners are all (7,6), sum 13).
        assert!(!check_occupancy_rect(occupancy_with_bounds(CellRect::new(7, 6, 0, 0))));
        assert!(check_occupancy_rect(occupancy_with_bounds(CellRect::single(7, 6))));
    }
}
