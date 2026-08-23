//! Native-shaped CellRect passability and occupancy validators.
//!
//! These are read-only substrate facades over the existing Rust grids. They do
//! not collapse terrain, overlays, reservations, and object-list occupancy into
//! one store; the point is to expose the two distinct gamemd validator surfaces
//! while preserving the current Rust-native ownership split.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::map::playfield::{lepton_to_packed_cell_component, rect_playfield_corners};
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::{OccupancyGrid, RawCellOccupationGrid};
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::zone_map::{ZoneGrid, ZoneId};

// Fixed cell indexing is map-owned (map::cell_index, F05); sim re-exports
// so runtime consumers keep their paths.
pub use crate::map::cell_index::{CELL_ROW_STRIDE, MAX_CELL_INDEX, cell_linear_index};
pub(crate) use crate::map::cell_index::{canonical_cell_coord, packed_cell_coord};
pub use crate::map::playfield::PlayfieldBounds;

/// A non-null cell reference — `Real` for an in-range, present cell, or `Dummy`
/// carrying the requested coord and shared fallback height bytes for an
/// out-of-range / missing lookup.
///
/// Never the absence of a value: the engine's coord→cell lookup returns a
/// non-null dummy that stores the requested coord and lets the caller keep
/// dispatching on it. Coordinate writes do not reconstruct or clear its
/// independently persistent level/slope state.
#[derive(Debug, Clone, Copy)]
pub enum CellRef<'a> {
    Real(&'a ResolvedTerrainCell),
    Dummy {
        coord: (i32, i32),
        level: i8,
        slope_type: u8,
    },
}

// `ResolvedTerrainCell` is not `PartialEq`; compare `Real` by pointer identity
// (same backing cell) and `Dummy` by the value snapshot returned by the lookup.
impl PartialEq for CellRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CellRef::Real(a), CellRef::Real(b)) => std::ptr::eq(*a, *b),
            (
                CellRef::Dummy {
                    coord: a,
                    level: al,
                    slope_type: aslope,
                },
                CellRef::Dummy {
                    coord: b,
                    level: bl,
                    slope_type: bslope,
                },
            ) => a == b && al == bl && aslope == bslope,
            _ => false,
        }
    }
}
impl Eq for CellRef<'_> {}

/// Engine `Get_CellClass`: coord → cell via the fixed stride; an out-of-range or
/// missing cell returns `CellRef::Dummy` carrying the packed requested coord and
/// preserved shared fallback bytes (NOT `(0,0)`, NOT `None`). The width-based
/// `PathGrid`/`ResolvedTerrainGrid` index stays as the cache; this is the
/// never-null parity lookup. Components are not checked separately: any valid
/// linear index aliases its canonical 512-wide slot.
pub fn get_cellclass_fallback<'a>(
    terrain: Option<&'a ResolvedTerrainGrid>,
    x: i32,
    y: i32,
) -> CellRef<'a> {
    let (x, y) = packed_cell_coord(x, y);
    if let Some(index) = cell_linear_index(x, y) {
        let rx = (index % CELL_ROW_STRIDE) as u16;
        let ry = (index / CELL_ROW_STRIDE) as u16;
        if let Some(cell) = terrain.and_then(|t| t.cell(rx, ry)) {
            return CellRef::Real(cell);
        }
    }
    if let Some(terrain) = terrain {
        terrain.stamp_dummy_cell_requested_coord(x, y);
    }
    let (level, slope_type) = terrain
        .map(ResolvedTerrainGrid::dummy_cell_level_slope)
        .unwrap_or((0, 0));
    CellRef::Dummy {
        coord: (x, y),
        level,
        slope_type,
    }
}

/// Engine world/lepton coordinate lookup, preserving full signed-i32 `/256`
/// quotients and wrapping fixed-stride index arithmetic. Coordinate words are
/// narrowed only after a miss, when native stamps the shared dummy.
///
/// Verified against `MapClass::Get_CellClass @ 0x00565730`: a real slot leaves
/// the shared dummy untouched, while either an invalid slot or a null entry
/// stamps the converted packed coordinate before returning the dummy.
pub fn get_cellclass_fallback_leptons<'a>(
    terrain: Option<&'a ResolvedTerrainGrid>,
    x_leptons: i32,
    y_leptons: i32,
) -> CellRef<'a> {
    let x = x_leptons / 256;
    let y = y_leptons / 256;
    let index = y
        .wrapping_mul(CELL_ROW_STRIDE as i32)
        .wrapping_add(x);
    if (0..=MAX_CELL_INDEX as i32).contains(&index) {
        let rx = (index % CELL_ROW_STRIDE as i32) as u16;
        let ry = (index / CELL_ROW_STRIDE as i32) as u16;
        if let Some(cell) = terrain.and_then(|terrain| terrain.cell(rx, ry)) {
            return CellRef::Real(cell);
        }
    }

    let (x, y) = packed_cell_coord(x, y);
    if let Some(terrain) = terrain {
        terrain.stamp_dummy_cell_requested_coord(x, y);
    }
    let (level, slope_type) = terrain
        .map(ResolvedTerrainGrid::dummy_cell_level_slope)
        .unwrap_or((0, 0));
    CellRef::Dummy {
        coord: (x, y),
        level,
        slope_type,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Raw, single-cell inputs to the shared YR movement-clearance leaf.
///
/// Callers retain responsibility for object-specific admission (Foot's cost
/// classifier, aircraft owner/shroud rules, and DropPod's Unlimbo path). This
/// type deliberately represents only `CellClass::IsClearToMove`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IsClearToMoveRequest {
    pub speed_type: SpeedType,
    pub movement_zone: MovementZone,
    /// `None` corresponds to native zone `-1` (no zone comparison).
    pub requested_zone: Option<i16>,
    pub actual_zone: i16,
    /// Native signed `CellClass+0x11B` base level.
    pub base_level: i16,
    /// Native `CellClass::Flags & 0x100` bridge gate.
    pub has_bridge: bool,
    /// `None` corresponds to native level `-1` (select bridge occupation on a bridge cell).
    pub requested_level: Option<i16>,
    pub is_bridge: bool,
    /// Native normal `OccupationFlags+0x124` low byte.
    pub ground_occupation_bits: u8,
    /// Native alternate `AltOccupationFlags+0x128` low byte.
    pub deck_occupation_bits: u8,
    pub ignore_infantry: bool,
    pub ignore_vehicles: bool,
    /// Whether the selected land-by-SpeedType row permits this cell.
    pub land_passable: bool,
    pub is_wall_overlay: bool,
    /// OverlayTypeClass's extra Crusher/AmphibiousCrusher admission byte.
    pub wall_allows_crusher: bool,
}

/// Deterministic result of the `CellClass::IsClearToMove` leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IsClearToMoveResult {
    /// Winged returns before CellClass selects a terrain or occupation plane.
    ClearWinged,
    Clear {
        selected_layer: MovementLayer,
    },
    ZoneMismatch,
    LevelMismatch,
    Occupied {
        remaining_bits: u8,
    },
    WallBlocked,
    LandBlocked,
}

/// Live-map adapter for callers that already own the class-specific admission
/// wrapper around `CellClass::IsClearToMove`.
///
/// `land_passable` remains caller-owned because Foot +0x1AC, placement, and
/// landing wrappers have distinct structural gates. Raw Cell occupation and
/// bridge-plane selection are centralized here instead of being reconstructed
/// independently by each caller.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LiveCellPassabilityQuery<'a> {
    pub target: (u16, u16),
    pub speed_type: SpeedType,
    pub movement_zone: MovementZone,
    pub requested_zone: Option<i16>,
    pub actual_zone: i16,
    pub requested_layer: Option<MovementLayer>,
    pub ignore_infantry: bool,
    pub ignore_vehicles: bool,
    pub land_passable: bool,
    pub path_grid: Option<&'a PathGrid>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub raw_occupation: Option<&'a RawCellOccupationGrid>,
}

/// Evaluate one live Cell through the shared native leaf.
// Native: `CellClass::CheckCellPassability` @ `0x004834A0`. Callers must still
// retain their own +0x1AC, CanThisExistHere, aircraft, or virtual-Unlimbo
// decisions.
//
// The earlier citation here — "CellClass::IsClearToMove @ 0x0047C650" — was
// wrong twice over: `0x0047C650` is not a function entry, and the function
// containing it is `Cell_passability_building_placement` @ `0x0047C620`, a
// building-placement test whose tail returns the buildable flag.
pub(crate) fn evaluate_live_cell_passability(
    query: LiveCellPassabilityQuery<'_>,
) -> IsClearToMoveResult {
    // Winged is the first native branch and therefore does not require a real
    // map cell. Preserve that ordering before resolving terrain metadata.
    if query.speed_type == SpeedType::Winged {
        return evaluate_is_clear_to_move(IsClearToMoveRequest {
            speed_type: query.speed_type,
            movement_zone: query.movement_zone,
            requested_zone: query.requested_zone,
            actual_zone: query.actual_zone,
            base_level: 0,
            has_bridge: false,
            requested_level: None,
            is_bridge: false,
            ground_occupation_bits: 0,
            deck_occupation_bits: 0,
            ignore_infantry: query.ignore_infantry,
            ignore_vehicles: query.ignore_vehicles,
            land_passable: query.land_passable,
            is_wall_overlay: false,
            wall_allows_crusher: false,
        });
    }

    let terrain_cell = query
        .resolved_terrain
        .and_then(|terrain| terrain.cell(query.target.0, query.target.1));
    let path_cell = query
        .path_grid
        .and_then(|grid| grid.cell(query.target.0, query.target.1));
    if terrain_cell.is_none() && path_cell.is_none() {
        return IsClearToMoveResult::LandBlocked;
    }

    let base_level = path_cell
        .map(|cell| cell.signed_level())
        .or_else(|| terrain_cell.map(|cell| i16::from(cell.level as i8)))
        .unwrap_or(0);
    let has_bridge = path_cell.is_some_and(|cell| cell.has_structural_bridge())
        || terrain_cell.is_some_and(|cell| cell.bridge_facts.has_structural_bridge());
    let requested_level = query.requested_layer.map(|layer| match layer {
        MovementLayer::Bridge => base_level.saturating_add(4),
        _ => base_level,
    });
    let ground_occupation_bits = query
        .raw_occupation
        .map_or(0, |grid| grid.ground_bits(query.target.0, query.target.1));
    let deck_occupation_bits = query
        .raw_occupation
        .map_or(0, |grid| grid.deck_bits(query.target.0, query.target.1));

    evaluate_is_clear_to_move(IsClearToMoveRequest {
        speed_type: query.speed_type,
        movement_zone: query.movement_zone,
        requested_zone: query.requested_zone,
        actual_zone: query.actual_zone,
        base_level,
        has_bridge,
        requested_level,
        is_bridge: query.requested_layer == Some(MovementLayer::Bridge),
        ground_occupation_bits,
        deck_occupation_bits,
        ignore_infantry: query.ignore_infantry,
        ignore_vehicles: query.ignore_vehicles,
        land_passable: query.land_passable,
        is_wall_overlay: terrain_cell.is_some_and(|cell| cell.zone_type == zone_class::WALL),
        // The parsed overlay model does not expose OverlayTypeClass+0x22D.
        wall_allows_crusher: false,
    })
}

/// Evaluate the native shared movement-clearance leaf without collapsing its callers.
// Native: `CellClass::CheckCellPassability` @ `0x004834A0` keeps Winged, bridge,
// raw-occupation and wall gates distinct; the FootClass +0x1AC predicate and
// object Unlimbo remain outside this seam. Its Destroyer-class wall escape set
// matches native's `{2, 3, 8, 0xC}`, tested at `0x004835A2`, `0x004835A7`,
// `0x004835AC` and `0x004835C5`.
pub(crate) fn evaluate_is_clear_to_move(input: IsClearToMoveRequest) -> IsClearToMoveResult {
    if input.speed_type == SpeedType::Winged {
        return IsClearToMoveResult::ClearWinged;
    }

    if input
        .requested_zone
        .is_some_and(|requested| requested != input.actual_zone)
    {
        return IsClearToMoveResult::ZoneMismatch;
    }

    let selected_layer = match input.requested_level {
        Some(level) if level == input.base_level => {
            if input.has_bridge && !input.is_bridge {
                return IsClearToMoveResult::LevelMismatch;
            }
            MovementLayer::Ground
        }
        Some(level) if input.has_bridge && level == input.base_level.saturating_add(4) => {
            MovementLayer::Bridge
        }
        Some(_) => return IsClearToMoveResult::LevelMismatch,
        None if input.has_bridge => MovementLayer::Bridge,
        None => MovementLayer::Ground,
    };

    let mut remaining_bits = match selected_layer {
        MovementLayer::Bridge => input.deck_occupation_bits,
        _ => input.ground_occupation_bits,
    };
    if input.ignore_infantry {
        remaining_bits &= 0xE0;
    }
    if input.ignore_vehicles {
        remaining_bits &= 0x5F;
    }
    if remaining_bits != 0 {
        return IsClearToMoveResult::Occupied { remaining_bits };
    }

    let mut wall_cleared = false;
    if input.is_wall_overlay {
        let clears_wall = matches!(
            input.movement_zone,
            MovementZone::Destroyer
                | MovementZone::AmphibiousDestroyer
                | MovementZone::InfantryDestroyer
                | MovementZone::CrusherAll
        ) || (input.wall_allows_crusher
            && matches!(
                input.movement_zone,
                MovementZone::Crusher | MovementZone::AmphibiousCrusher
            ));
        if !clears_wall {
            return IsClearToMoveResult::WallBlocked;
        }
        wall_cleared = true;
    }

    if selected_layer == MovementLayer::Bridge || wall_cleared || input.land_passable {
        IsClearToMoveResult::Clear { selected_layer }
    } else {
        IsClearToMoveResult::LandBlocked
    }
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

/// Native CellRect scan: x outer, y inner, wrapping endpoints and signed
/// comparisons. Returning false from `visit` stops at the first failed cell.
pub(crate) fn scan_cell_rect(rect: CellRect, mut visit: impl FnMut(i32, i32) -> bool) -> bool {
    let end_x = rect.x.wrapping_add(rect.width);
    let end_y = rect.y.wrapping_add(rect.height);
    let mut x = rect.x;
    while x < end_x {
        let mut y = rect.y;
        while y < end_y {
            if !visit(x, y) {
                return false;
            }
            y = y.wrapping_add(1);
        }
        x = x.wrapping_add(1);
    }
    true
}

/// Sparse authority for CellClass `+0xDC`. A fixed-stride coordinate selects a
/// real entry only when the active terrain has an allocated CellClass pointer;
/// every invalid or valid-but-unallocated lookup shares `dummy_mask`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CellReservationGrid {
    masks: BTreeMap<(u16, u16), u32>,
    dummy_mask: u32,
}

impl CellReservationGrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reserve(
        &mut self,
        terrain: Option<&ResolvedTerrainGrid>,
        x: i32,
        y: i32,
        reservation_arg: i32,
    ) {
        let mask = reservation_mask(reservation_arg);
        if mask == 0 {
            return;
        }
        if let Some(cell) = reservation_cell_coord(terrain, x, y) {
            *self.masks.entry(cell).or_default() |= mask;
        } else {
            self.dummy_mask |= mask;
        }
    }

    pub fn clear(
        &mut self,
        terrain: Option<&ResolvedTerrainGrid>,
        x: i32,
        y: i32,
        reservation_arg: i32,
    ) {
        let mask = reservation_mask(reservation_arg);
        if mask == 0 {
            return;
        }
        let Some(cell) = reservation_cell_coord(terrain, x, y) else {
            self.dummy_mask &= !mask;
            return;
        };
        if let Some(bits) = self.masks.get_mut(&cell) {
            *bits &= !mask;
            if *bits == 0 {
                self.masks.remove(&cell);
            }
        }
    }

    pub fn raw_mask(&self, terrain: Option<&ResolvedTerrainGrid>, x: i32, y: i32) -> u32 {
        reservation_cell_coord(terrain, x, y).map_or(self.dummy_mask, |cell| {
            self.masks.get(&cell).copied().unwrap_or(0)
        })
    }

    pub fn has_reservation(
        &self,
        terrain: Option<&ResolvedTerrainGrid>,
        x: i32,
        y: i32,
        reservation_arg: i32,
    ) -> bool {
        let mask = reservation_mask(reservation_arg);
        mask != 0 && self.raw_mask(terrain, x, y) & mask != 0
    }

    pub(crate) fn reserve_rect(
        &mut self,
        terrain: Option<&ResolvedTerrainGrid>,
        rect: CellRect,
        reservation_arg: i32,
    ) {
        scan_cell_rect(rect, |x, y| {
            self.reserve(terrain, x, y, reservation_arg);
            true
        });
    }

    pub(crate) fn clear_rect(
        &mut self,
        terrain: Option<&ResolvedTerrainGrid>,
        rect: CellRect,
        reservation_arg: i32,
    ) {
        scan_cell_rect(rect, |x, y| {
            self.clear(terrain, x, y, reservation_arg);
            true
        });
    }

    pub(crate) fn has_reservation_inclusive(
        &self,
        terrain: Option<&ResolvedTerrainGrid>,
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
        reservation_arg: i32,
    ) -> bool {
        if min_x > max_x || min_y > max_y || reservation_mask(reservation_arg) == 0 {
            return false;
        }
        let mut x = min_x;
        loop {
            let mut y = min_y;
            loop {
                if self.has_reservation(terrain, x, y, reservation_arg) {
                    return true;
                }
                if y == max_y {
                    break;
                }
                y = y.wrapping_add(1);
            }
            if x == max_x {
                break;
            }
            x = x.wrapping_add(1);
        }
        false
    }

    /// Same-house reservation connectivity around a center cell. Bits are
    /// N, NE, E, SE, S, SW, W, NW. A center without the requested house bit
    /// returns the native `-1` sentinel as `u32::MAX`.
    pub fn house_reservation_neighbor_mask(
        &self,
        terrain: Option<&ResolvedTerrainGrid>,
        x: i32,
        y: i32,
        reservation_arg: i32,
    ) -> u32 {
        if !self.has_reservation(terrain, x, y, reservation_arg) {
            return u32::MAX;
        }
        const NEIGHBORS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        let mut result = 0u32;
        for (index, (dx, dy)) in NEIGHBORS.into_iter().enumerate() {
            if self.has_reservation(
                terrain,
                x.wrapping_add(dx),
                y.wrapping_add(dy),
                reservation_arg,
            ) {
                result |= 1u32 << index;
            }
        }
        result
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (u16, u16, u32)> + '_ {
        self.masks.iter().map(|(&(rx, ry), &mask)| (rx, ry, mask))
    }

    pub(crate) fn dummy_mask(&self) -> u32 {
        self.dummy_mask
    }
}

fn reservation_cell_coord(
    terrain: Option<&ResolvedTerrainGrid>,
    x: i32,
    y: i32,
) -> Option<(u16, u16)> {
    canonical_cell_coord(x, y)
        .filter(|&(rx, ry)| terrain.is_none_or(|terrain| terrain.cell(rx, ry).is_some()))
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
    /// Derived index of live TerrainClass objects in the active ground list.
    pub terrain_object_cells: Option<&'a BTreeMap<(u16, u16), u64>>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub overlay_grid: Option<&'a OverlayGrid>,
    /// The configured map's final normalized isometric-diamond fields. Absence
    /// rejects the query: active MapClass has no rectangular/unbounded substitute
    /// for `IsRectInPlayfield @ 0x00578390`.
    pub playfield_bounds: Option<PlayfieldBounds>,
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

    if !scan_cell_rect(ctx.rect, |x, y| {
        occupancy_blocker_at(&ctx, x, y, mask).is_none()
    }) {
        return false;
    }

    rect_is_in_playfield_height_aware(ctx.rect, ctx.playfield_bounds, ctx.resolved_terrain)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OccupancyBlocker {
    TerrainObject,
    Reservation,
    Overlay,
    ZoneType,
    Slope,
    Building,
}

fn occupancy_blocker_at(
    ctx: &CellRectOccupancyContext<'_>,
    x: i32,
    y: i32,
    reservation_mask: u32,
) -> Option<OccupancyBlocker> {
    let canonical = canonical_cell_coord(x, y);

    if canonical.is_some_and(|cell| {
        ctx.terrain_object_cells
            .is_some_and(|terrain| terrain.contains_key(&cell))
    }) {
        return Some(OccupancyBlocker::TerrainObject);
    }
    if reservation_mask != 0
        && ctx.reservations.is_some_and(|reservations| {
            reservations.raw_mask(ctx.resolved_terrain, x, y) & reservation_mask != 0
        })
    {
        return Some(OccupancyBlocker::Reservation);
    }
    if canonical.is_some_and(|(rx, ry)| overlay_present(ctx.overlay_grid, rx, ry)) {
        return Some(OccupancyBlocker::Overlay);
    }

    let cell = get_cellclass_fallback(ctx.resolved_terrain, x, y);
    if matches!(cell, CellRef::Real(cell) if cell.zone_type != zone_class::GROUND) {
        return Some(OccupancyBlocker::ZoneType);
    }
    if match cell {
        CellRef::Real(cell) => cell.slope_type != 0,
        CellRef::Dummy { slope_type, .. } => slope_type != 0,
    } {
        return Some(OccupancyBlocker::Slope);
    }
    if canonical
        .is_some_and(|(rx, ry)| ground_building_present(ctx.occupancy, ctx.entities, rx, ry))
    {
        return Some(OccupancyBlocker::Building);
    }
    None
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

    // Native location: `CellClass::IsClearToMove` (YR 1.001). CellRect callers
    // do not yet carry the raw occupation grid, so their existing object-list
    // blocker projection is kept explicit here. World/movement callers with raw
    // bytes must construct `IsClearToMoveRequest` directly rather than infer bits.
    let projected_ground_bits = u8::from(
        ctx.occupancy
            .is_some_and(|grid| grid.count_on_layer(rx, ry, MovementLayer::Ground) > 0),
    ) * 0x40;
    let projected_deck_bits = u8::from(
        ctx.occupancy
            .is_some_and(|grid| grid.count_on_layer(rx, ry, MovementLayer::Bridge) > 0),
    ) * 0x40;
    let is_wall_overlay = terrain_cell.is_some_and(|cell| cell.zone_type == zone_class::WALL);
    let land_passable = terrain_cell.map_or_else(
        || ctx.path_grid.map_or(true, |grid| grid.is_walkable(rx, ry)),
        |cell| speed_type_allows_cell(cell, ctx.speed_type, ctx.movement_zone),
    );
    matches!(
        evaluate_is_clear_to_move(IsClearToMoveRequest {
            speed_type: ctx.speed_type,
            movement_zone: ctx.movement_zone,
            requested_zone: None,
            actual_zone: 0,
            base_level,
            has_bridge: structural_bridge,
            requested_level: ctx.required_height_or_level,
            is_bridge: ctx.bridge_aware_zone,
            ground_occupation_bits: projected_ground_bits,
            deck_occupation_bits: projected_deck_bits,
            ignore_infantry: false,
            ignore_vehicles: false,
            land_passable,
            is_wall_overlay,
            // The parsed overlay model has no authority for native +0x22D.
            wall_allows_crusher: false,
        }),
        IsClearToMoveResult::Clear { .. } | IsClearToMoveResult::ClearWinged
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
    cell.speed_costs
        .cost_for_speed_type(speed_type)
        .is_none_or(|cost| cost > 0)
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

/// Height-aware `MapClass::IsRectInPlayfield @ 0x00578390`: test NW, NE,
/// SW, then SE with native wrapping/truncation and short-circuit order.
/// Missing configured bounds reject instead of substituting a rectangular or
/// unbounded approximation that active MapClass does not have.
fn rect_is_in_playfield_height_aware(
    rect: CellRect,
    bounds: Option<PlayfieldBounds>,
    terrain: Option<&ResolvedTerrainGrid>,
) -> bool {
    let Some(bounds) = bounds else {
        return false;
    };
    rect_playfield_corners(rect.x, rect.y, rect.width, rect.height)
        .into_iter()
        .all(|cell| cell_is_in_playfield_height_aware(cell, Some(bounds), terrain))
}

/// Explicit mode-zero `MapClass::IsCellInPlayfield @ 0x00578460` seam.
/// No CellClass lookup or dummy state is touched.
pub fn cell_is_in_playfield_geometry_only(
    cell: (i32, i32),
    bounds: PlayfieldBounds,
) -> bool {
    bounds.contains_geometry_packed(cell.0, cell.1)
}

/// Explicit mode-one `MapClass::IsCellInPlayfield @ 0x00578460` seam.
/// Missing configured bounds reject; active native callers do not replace the
/// playfield diamond with a terrain rectangle or unconditional success.
pub(crate) fn cell_is_in_playfield_height_aware(
    cell: (i32, i32),
    bounds: Option<PlayfieldBounds>,
    terrain: Option<&ResolvedTerrainGrid>,
) -> bool {
    let Some(bounds) = bounds else {
        return false;
    };
    let (x, y) = packed_cell_coord(cell.0, cell.1);
    let (level, slope) = match get_cellclass_fallback(terrain, x, y) {
        CellRef::Real(cell) => (cell.level as i8, cell.slope_type),
        CellRef::Dummy {
            level, slope_type, ..
        } => (level, slope_type),
    };
    bounds.contains_height_aware_packed(x, y, level, slope)
}

/// Forced-mode-one signed-lepton wrapper from
/// `MapClass::IsCoordInPlayfield @ 0x005785F0`. Division by 256 truncates
/// toward zero, the quotients truncate to signed i16, and z is ignored.
pub fn cell_is_in_playfield_leptons(
    coord: (i32, i32, i32),
    bounds: Option<PlayfieldBounds>,
    terrain: Option<&ResolvedTerrainGrid>,
) -> bool {
    let cell = (
        lepton_to_packed_cell_component(coord.0),
        lepton_to_packed_cell_component(coord.1),
    );
    cell_is_in_playfield_height_aware(cell, bounds, terrain)
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
    use crate::map::map_file::MapHeader;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::occupancy::CellListInsertion;
    use crate::sim::pathfinding::zone_map::ZoneGrid;

    fn map_header_with_rects(size: (i32, i32), local: [i32; 4]) -> MapHeader {
        MapHeader {
            theater: "TEMPERATE".to_string(),
            fill: "Clear".to_string(),
            level: 0,
            width: size.0 as u32,
            height: size.1 as u32,
            local_left: local[0] as u32,
            local_top: local[1] as u32,
            local_width: local[2] as u32,
            local_height: local[3] as u32,
        }
    }

    #[test]
    fn playfield_bounds_from_map_header_keeps_nearoref_native_values() {
        let bounds =
            PlayfieldBounds::from_map_header(&map_header_with_rects((80, 58), [2, 4, 76, 48]));

        assert_eq!(
            bounds,
            PlayfieldBounds {
                base: 80,
                off_fc: 2,
                off_100: 4,
                off_104: 76,
                off_108: 48,
            }
        );
    }

    #[test]
    fn playfield_bounds_from_map_header_clips_signed_local_before_margins() {
        let bounds =
            PlayfieldBounds::from_map_header(&map_header_with_rects((80, 80), [-5, -6, 100, 100]));

        assert_eq!(
            bounds,
            PlayfieldBounds {
                base: 80,
                off_fc: 2,
                off_100: 2,
                off_104: 76,
                off_108: 72,
            }
        );
    }

    #[test]
    fn playfield_bounds_from_map_header_preserves_native_empty_and_small_results() {
        assert_eq!(
            PlayfieldBounds::from_map_header(&map_header_with_rects((40, 50), [0; 4])),
            PlayfieldBounds {
                base: 40,
                off_fc: 2,
                off_100: 2,
                off_104: 0,
                off_108: 0,
            }
        );
        assert_eq!(
            PlayfieldBounds::from_map_header(&map_header_with_rects((0, 0), [0; 4])),
            PlayfieldBounds {
                base: 0,
                off_fc: 2,
                off_100: 2,
                off_104: -4,
                off_108: -8,
            }
        );
    }

    #[test]
    fn playfield_bounds_from_map_header_caps_present_rect_at_native_margins() {
        let bounds =
            PlayfieldBounds::from_map_header(&map_header_with_rects((40, 50), [5, 6, 40, 50]));

        assert_eq!(
            bounds,
            PlayfieldBounds {
                base: 40,
                off_fc: 5,
                off_100: 6,
                off_104: 33,
                off_108: 38,
            }
        );
    }

    fn clear_to_move_request() -> IsClearToMoveRequest {
        IsClearToMoveRequest {
            speed_type: SpeedType::Track,
            movement_zone: MovementZone::Normal,
            requested_zone: None,
            actual_zone: 7,
            base_level: 2,
            has_bridge: false,
            requested_level: Some(2),
            is_bridge: false,
            ground_occupation_bits: 0,
            deck_occupation_bits: 0,
            ignore_infantry: false,
            ignore_vehicles: false,
            land_passable: true,
            is_wall_overlay: false,
            wall_allows_crusher: false,
        }
    }

    #[test]
    fn is_clear_to_move_keeps_winged_and_raw_ignore_masks_separate() {
        let winged = IsClearToMoveRequest {
            speed_type: SpeedType::Winged,
            requested_zone: Some(99),
            ground_occupation_bits: 0xFF,
            deck_occupation_bits: 0xFF,
            land_passable: false,
            is_wall_overlay: true,
            ..clear_to_move_request()
        };
        assert_eq!(
            evaluate_is_clear_to_move(winged),
            IsClearToMoveResult::ClearWinged
        );

        let infantry_ignored = IsClearToMoveRequest {
            ground_occupation_bits: 0x01,
            ignore_infantry: true,
            ..clear_to_move_request()
        };
        assert!(matches!(
            evaluate_is_clear_to_move(infantry_ignored),
            IsClearToMoveResult::Clear { .. }
        ));

        let vehicle_ignored = IsClearToMoveRequest {
            ground_occupation_bits: 0x20,
            ignore_vehicles: true,
            ..clear_to_move_request()
        };
        assert!(matches!(
            evaluate_is_clear_to_move(vehicle_ignored),
            IsClearToMoveResult::Clear { .. }
        ));

        let generic_remains = IsClearToMoveRequest {
            ground_occupation_bits: 0x61,
            ignore_infantry: true,
            ignore_vehicles: true,
            ..clear_to_move_request()
        };
        assert_eq!(
            evaluate_is_clear_to_move(generic_remains),
            IsClearToMoveResult::Occupied {
                remaining_bits: 0x40
            }
        );
    }

    #[test]
    fn is_clear_to_move_preserves_bridge_and_wall_admission_order() {
        let bridge_unspecified = IsClearToMoveRequest {
            has_bridge: true,
            requested_level: None,
            deck_occupation_bits: 0,
            land_passable: false,
            ..clear_to_move_request()
        };
        assert_eq!(
            evaluate_is_clear_to_move(bridge_unspecified),
            IsClearToMoveResult::Clear {
                selected_layer: MovementLayer::Bridge
            }
        );

        let bridge_uses_deck_occupation = IsClearToMoveRequest {
            has_bridge: true,
            requested_level: None,
            ground_occupation_bits: 0x40,
            deck_occupation_bits: 0x20,
            ..clear_to_move_request()
        };
        assert_eq!(
            evaluate_is_clear_to_move(bridge_uses_deck_occupation),
            IsClearToMoveResult::Occupied {
                remaining_bits: 0x20
            }
        );

        let base_without_bridge_flag = IsClearToMoveRequest {
            has_bridge: true,
            requested_level: Some(2),
            is_bridge: false,
            ..clear_to_move_request()
        };
        assert_eq!(
            evaluate_is_clear_to_move(base_without_bridge_flag),
            IsClearToMoveResult::LevelMismatch
        );

        let crusher_wall = IsClearToMoveRequest {
            movement_zone: MovementZone::Crusher,
            is_wall_overlay: true,
            wall_allows_crusher: true,
            land_passable: false,
            ..clear_to_move_request()
        };
        assert!(matches!(
            evaluate_is_clear_to_move(crusher_wall),
            IsClearToMoveResult::Clear {
                selected_layer: MovementLayer::Ground
            }
        ));

        let normal_wall = IsClearToMoveRequest {
            is_wall_overlay: true,
            land_passable: false,
            ..clear_to_move_request()
        };
        assert_eq!(
            evaluate_is_clear_to_move(normal_wall),
            IsClearToMoveResult::WallBlocked
        );
    }

    #[test]
    fn live_cell_passability_reads_the_selected_raw_plane() {
        let path_grid = PathGrid::test_all_passable(2, 2);
        let mut raw = RawCellOccupationGrid::new();
        raw.mark_ground(1, 1, 0x20);
        let query = LiveCellPassabilityQuery {
            target: (1, 1),
            speed_type: SpeedType::Track,
            movement_zone: MovementZone::Normal,
            requested_zone: None,
            actual_zone: 0,
            requested_layer: Some(MovementLayer::Ground),
            ignore_infantry: false,
            ignore_vehicles: false,
            land_passable: true,
            path_grid: Some(&path_grid),
            resolved_terrain: None,
            raw_occupation: Some(&raw),
        };
        let occupied = evaluate_live_cell_passability(query);
        assert_eq!(
            occupied,
            IsClearToMoveResult::Occupied {
                remaining_bits: 0x20
            }
        );

        let vehicle_ignored = evaluate_live_cell_passability(LiveCellPassabilityQuery {
            ignore_vehicles: true,
            ..query
        });
        assert!(matches!(vehicle_ignored, IsClearToMoveResult::Clear { .. }));
    }

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
            height_in_pixels: 0,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            terrain_object_occupation: None,
            overlay_blocks: false,
            overlay_zone_type: None,
            outside_playfield: false,
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

    fn wide_test_playfield() -> PlayfieldBounds {
        PlayfieldBounds {
            base: 0,
            off_fc: -1_000,
            off_100: -1_000,
            off_104: 2_000,
            off_108: 2_000,
        }
    }

    #[test]
    fn cellrect_occupancy_minus_one_skips_reservation_but_rejects_cell_blockers() {
        let mut terrain = flat_terrain(3, 1);
        terrain.cells[1].slope_type = 2;
        let mut reservations = CellReservationGrid::new();
        reservations.reserve(Some(&terrain), 0, 0, 3);

        let clear_reserved = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: -1,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            terrain_object_cells: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            playfield_bounds: Some(wide_test_playfield()),
        };
        assert!(check_occupancy_rect(clear_reserved));

        let sloped = CellRectOccupancyContext {
            rect: CellRect::single(1, 0),
            reservation_arg: -1,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            terrain_object_cells: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            playfield_bounds: Some(wide_test_playfield()),
        };
        assert!(!check_occupancy_rect(sloped));
    }

    #[test]
    fn cellrect_occupancy_house_reservation_blocks_same_house_only() {
        let terrain = flat_terrain(2, 1);
        let mut reservations = CellReservationGrid::new();
        reservations.reserve(Some(&terrain), 0, 0, 5);

        let same_house = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: 5,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            terrain_object_cells: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            playfield_bounds: Some(wide_test_playfield()),
        };
        assert!(!check_occupancy_rect(same_house));

        let other_house = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: 6,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            terrain_object_cells: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            playfield_bounds: Some(wide_test_playfield()),
        };
        assert!(check_occupancy_rect(other_house));

        let skipped = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: -1,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            terrain_object_cells: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            playfield_bounds: Some(wide_test_playfield()),
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
            terrain_object_cells: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            playfield_bounds: Some(wide_test_playfield()),
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
            CellRef::Dummy {
                coord: (-3, 7),
                level: 0,
                slope_type: 0,
            }
        );
    }

    #[test]
    fn gsi_04_01_get_cellclass_fixed_stride_aliases_canonical_slot() {
        let terrain = flat_terrain(512, 2);

        assert_eq!(
            get_cellclass_fallback(Some(&terrain), -1, 1),
            CellRef::Real(terrain.cell(511, 0).expect("canonical index 511"))
        );
        assert_eq!(
            get_cellclass_fallback(Some(&terrain), 512, 0),
            CellRef::Real(terrain.cell(0, 1).expect("canonical index 512"))
        );

        assert_eq!(
            get_cellclass_fallback(Some(&terrain), -1, 0),
            CellRef::Dummy {
                coord: (-1, 0),
                level: 0,
                slope_type: 0,
            }
        );
        let missing_canonical_cell = flat_terrain(2, 1);
        assert_eq!(
            get_cellclass_fallback(Some(&missing_canonical_cell), 512, 0),
            CellRef::Dummy {
                coord: (512, 0),
                level: 0,
                slope_type: 0,
            }
        );
    }

    #[test]
    fn gsi_04_01_lookup_misses_stamp_only_packed_dummy_coord() {
        let mut terrain = flat_terrain(512, 2);
        terrain.test_set_native_allocated_cells(&[(0, 0), (511, 0)]);
        terrain.test_set_dummy_cell_level_slope(-5, 7);
        assert_eq!(terrain.dummy_cell_requested_coord(), (0, 0));

        assert_eq!(
            get_cellclass_fallback(Some(&terrain), -1, 1),
            CellRef::Real(terrain.cell(511, 0).expect("fixed-stride alias slot"))
        );
        assert_eq!(terrain.dummy_cell_requested_coord(), (0, 0));

        assert_eq!(
            get_cellclass_fallback(Some(&terrain), -1, 0),
            CellRef::Dummy {
                coord: (-1, 0),
                level: -5,
                slope_type: 7,
            }
        );
        assert_eq!(terrain.dummy_cell_requested_coord(), (-1, 0));

        // Packed (512,0) has a valid fixed-array index, but its canonical
        // (0,1) slot is null in this allocation mask. Native still stamps the
        // requested words; high Rust-only bits do not survive the seam.
        assert_eq!(
            get_cellclass_fallback(Some(&terrain), 0x1_0200, 0x1_0000),
            CellRef::Dummy {
                coord: (512, 0),
                level: -5,
                slope_type: 7,
            }
        );
        assert_eq!(terrain.dummy_cell_requested_coord(), (512, 0));
        assert_eq!(terrain.dummy_cell_level_slope(), (-5, 7));
    }

    #[test]
    fn gsi_04_01_lookup_world_leptons_truncate_before_fallback() {
        let terrain = flat_terrain(1, 1);
        assert_eq!(
            get_cellclass_fallback(Some(&terrain), -2, 0),
            CellRef::Dummy {
                coord: (-2, 0),
                level: 0,
                slope_type: 0,
            }
        );

        assert_eq!(
            get_cellclass_fallback_leptons(Some(&terrain), -1, -255),
            CellRef::Real(terrain.cell(0, 0).expect("negative fractions truncate to zero"))
        );
        assert_eq!(
            get_cellclass_fallback_leptons(Some(&terrain), -255, -1),
            CellRef::Real(terrain.cell(0, 0).expect("negative fractions truncate to zero"))
        );
        assert_eq!(terrain.dummy_cell_requested_coord(), (-2, 0));

        // Full quotients (32768,-64) cancel to fixed index zero before either
        // component is narrowed to its dummy-cell word.
        assert_eq!(
            get_cellclass_fallback_leptons(Some(&terrain), 8_388_608, -16_384),
            CellRef::Real(terrain.cell(0, 0).expect("full-i32 quotient index cancellation"))
        );
        assert_eq!(terrain.dummy_cell_requested_coord(), (-2, 0));

        assert_eq!(
            get_cellclass_fallback_leptons(Some(&terrain), -256, 0),
            CellRef::Dummy {
                coord: (-1, 0),
                level: 0,
                slope_type: 0,
            }
        );
        assert_eq!(terrain.dummy_cell_requested_coord(), (-1, 0));

        assert_eq!(
            get_cellclass_fallback_leptons(Some(&terrain), 256, 0),
            CellRef::Dummy {
                coord: (1, 0),
                level: 0,
                slope_type: 0,
            }
        );
        assert_eq!(terrain.dummy_cell_requested_coord(), (1, 0));
    }

    #[test]
    fn gsi_04_01_lookup_allocation_probe_has_no_dummy_side_effect() {
        let mut terrain = flat_terrain(512, 2);
        terrain.test_set_native_allocated_cells(&[(0, 0), (511, 0)]);
        terrain.test_set_dummy_cell_level_slope(-4, 9);
        let _ = get_cellclass_fallback(Some(&terrain), -3, 0);
        assert_eq!(terrain.dummy_cell_requested_coord(), (-3, 0));

        assert!(terrain.cellclass_allocation_probe(0, 0));
        assert!(terrain.cellclass_allocation_probe(-1, 1));
        assert!(!terrain.cellclass_allocation_probe(1, 0));
        assert!(!terrain.cellclass_allocation_probe(512, 0));
        assert!(!terrain.cellclass_allocation_probe(-1, 0));
        assert_eq!(terrain.dummy_cell_requested_coord(), (-3, 0));
        assert_eq!(terrain.dummy_cell_level_slope(), (-4, 9));
    }

    #[test]
    fn gsi_04_01_lookup_clone_copies_then_owns_dummy_state() {
        let mut terrain = flat_terrain(1, 1);
        terrain.test_set_dummy_cell_level_slope(-6, 11);
        let _ = get_cellclass_fallback(Some(&terrain), -1, 0);

        let cloned = terrain.clone();
        assert_eq!(cloned.dummy_cell_requested_coord(), (-1, 0));
        assert_eq!(cloned.dummy_cell_level_slope(), (-6, 11));
        let _ = get_cellclass_fallback(Some(&cloned), -2, 0);
        assert_eq!(cloned.dummy_cell_requested_coord(), (-2, 0));
        assert_eq!(terrain.dummy_cell_requested_coord(), (-1, 0));
        assert_eq!(cloned.dummy_cell_level_slope(), (-6, 11));
        assert_eq!(terrain.dummy_cell_level_slope(), (-6, 11));

        let reconstructed = flat_terrain(1, 1);
        assert_eq!(reconstructed.dummy_cell_requested_coord(), (0, 0));
        assert_eq!(reconstructed.dummy_cell_level_slope(), (0, 0));
    }

    #[test]
    fn playfield_rect_wrapper_matches_native_corner_contract() {
        let terrain = flat_terrain(512, 2);

        // Only the low word of each requested component reaches the native
        // lookup: 0xFFFF is -1, so (-1,1) aliases canonical (511,0).
        assert_eq!(cell_linear_index(0xFFFF, 1), Some(511));
        assert_eq!(cell_linear_index(0x1_0000, 0), Some(0));
        assert_eq!(
            get_cellclass_fallback(Some(&terrain), 0xFFFF, 1),
            CellRef::Real(terrain.cell(511, 0).expect("canonical index 511"))
        );
        assert_eq!(
            get_cellclass_fallback(Some(&flat_terrain(2, 1)), 0xFFFF, 0),
            CellRef::Dummy {
                coord: (-1, 0),
                level: 0,
                slope_type: 0,
            }
        );

        // Far corners use x+width-1/y+height-1, then truncate each component
        // to its stored word, without saturating at the i32 or i16 boundary.
        assert_eq!(
            rect_playfield_corners(i32::from(i16::MAX), i32::from(i16::MIN), 2, 0),
            [
                (i32::from(i16::MAX), i32::from(i16::MIN)),
                (i32::from(i16::MIN), i32::from(i16::MIN)),
                (i32::from(i16::MAX), i32::from(i16::MAX)),
                (i32::from(i16::MIN), i32::from(i16::MAX)),
            ]
        );
        assert!(rect_is_in_playfield_height_aware(
            CellRect::new(7, 6, 0x1_0001, 1),
            Some(diamond_bounds()),
            None,
        ));
        assert_eq!(
            rect_playfield_corners(7, 6, -1, -2),
            [(7, 6), (5, 6), (7, 3), (5, 3)]
        );
        assert!(!rect_is_in_playfield_height_aware(
            CellRect::new(7, 6, -1, -2),
            Some(diamond_bounds()),
            None,
        ));
    }

    #[test]
    fn playfield_modes_skip_or_apply_cell_height_explicitly() {
        let bounds = PlayfieldBounds {
            base: 10,
            off_fc: 2,
            off_100: 1,
            off_104: 10,
            off_108: 6,
        };
        let mut terrain = flat_terrain(16, 16);
        terrain.cells[6 * 16 + 7].slope_type = 1;

        // (7,6), sum=13, is just inside the geometry-only strict low edge 12.
        // Mode one looks up the sloped cell, bumps h to one, and moves that edge
        // to 13, so the same cell is excluded. Mode zero cannot touch the cell.
        assert!(cell_is_in_playfield_geometry_only((7, 6), bounds));
        assert!(!cell_is_in_playfield_height_aware(
            (7, 6),
            Some(bounds),
            Some(&terrain),
        ));
    }

    #[test]
    fn playfield_leptons_truncate_toward_zero() {
        assert_eq!(lepton_to_packed_cell_component(-1), 0);
        assert_eq!(lepton_to_packed_cell_component(-255), 0);
        assert_eq!(lepton_to_packed_cell_component(-256), -1);
        assert_eq!(lepton_to_packed_cell_component(-257), -1);

        let bounds = diamond_bounds();
        assert_eq!(
            cell_is_in_playfield_leptons(
                (7 * 256 + 255, 6 * 256 + 1, i32::MAX),
                Some(bounds),
                None
            ),
            cell_is_in_playfield_height_aware((7, 6), Some(bounds), None),
        );
    }

    #[test]
    fn playfield_absence_does_not_use_rectangular_or_unbounded_fallback() {
        assert!(!cell_is_in_playfield_height_aware((1, 1), None, None));
        assert!(!rect_is_in_playfield_height_aware(
            CellRect::single(1, 1),
            None,
            None,
        ));
    }

    #[test]
    fn gsi_04_01_playfield_height_reads_fixed_stride_alias() {
        let mut terrain = flat_terrain(512, 1);
        terrain.cells[511].level = u8::MAX; // signed level -1
        let bounds = PlayfieldBounds {
            base: 0,
            off_fc: -5,
            off_100: 0,
            off_104: 10,
            off_108: 0,
        };

        // Requested (-1,1) aliases canonical (511,0). Its signed level -1
        // shifts the strict low sum boundary from 0 to -1, making sum=0 pass.
        // The zero-field dummy leaves the boundary at 0 and therefore fails.
        assert!(!cell_is_in_playfield_height_aware(
            (-1, 1),
            Some(bounds),
            None,
        ));
        assert!(cell_is_in_playfield_height_aware(
            (-1, 1),
            Some(bounds),
            Some(&terrain),
        ));
    }

    #[test]
    fn gsi_04_01_dummy_state_persists_across_fallback_lookups() {
        let mut terrain = flat_terrain(1, 1);
        assert_eq!(terrain.dummy_cell_level_slope(), (0, 0));
        terrain.test_set_dummy_cell_level_slope(-4, 7);
        terrain.set_dummy_cell_level(-5);
        assert_eq!(terrain.clone().dummy_cell_level_slope(), (-5, 7));

        assert_eq!(
            get_cellclass_fallback(Some(&terrain), 0xFFFF, 0),
            CellRef::Dummy {
                coord: (-1, 0),
                level: -5,
                slope_type: 7,
            }
        );
        assert_eq!(
            get_cellclass_fallback(Some(&terrain), 0xFFFE, 0),
            CellRef::Dummy {
                coord: (-2, 0),
                level: -5,
                slope_type: 7,
            }
        );

        let bounds = PlayfieldBounds {
            base: -1,
            off_fc: -5,
            off_100: 0,
            off_104: 10,
            off_108: 3,
        };
        let zero_dummy = flat_terrain(1, 1);
        assert!(!cell_is_in_playfield_height_aware(
            (-1, 0),
            Some(bounds),
            Some(&zero_dummy),
        ));
        assert!(cell_is_in_playfield_height_aware(
            (-1, 0),
            Some(bounds),
            Some(&terrain),
        ));
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
            terrain_object_cells: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            playfield_bounds: Some(wide_test_playfield()),
        };
        assert!(check_occupancy_rect(clear)); // clear cell passes

        let slope_only = CellRectOccupancyContext {
            rect: CellRect::single(1, 0),
            reservation_arg: -1,
            reservations: None,
            occupancy: None,
            entities: None,
            terrain_object_cells: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            playfield_bounds: Some(wide_test_playfield()),
        };
        assert!(!check_occupancy_rect(slope_only));

        let zone_only = CellRectOccupancyContext {
            rect: CellRect::single(2, 0),
            reservation_arg: -1,
            reservations: None,
            occupancy: None,
            entities: None,
            terrain_object_cells: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            playfield_bounds: Some(wide_test_playfield()),
        };
        assert!(!check_occupancy_rect(zone_only));
    }

    /// A diamond bounds fixture chosen so the playable region is a clean interior:
    /// pass iff `12 < sx+sy <= 26` AND `sx-sy < 14` AND `sy-sx < 6` (flat terrain,
    /// so the height extension `h = 0`). Derived from the resolved formula in
    /// the canonical MapClass playfield predicate with these five values:
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
            terrain_object_cells: None,
            resolved_terrain: None,
            overlay_grid: None,
            playfield_bounds: Some(diamond_bounds()),
        }
    }

    #[test]
    fn rect_in_playfield_is_isometric_diamond_inclusive_four_corners() {
        // A 1x1 rect on the diamond's INCLUSIVE high edge of the sum band passes:
        // (13,13) has sum 26 == base+HIGH, and both diagonals are inside.
        assert!(check_occupancy_rect(occupancy_with_bounds(
            CellRect::single(13, 13)
        )));
        // One cell past the high edge (sum 27 > 26) fails — proves the band is a
        // diamond on sx+sy, not a rectangle on raw x/y.
        assert!(!check_occupancy_rect(occupancy_with_bounds(
            CellRect::single(14, 13)
        )));

        // A 2x1 rect whose NW corner (13,13) is inside but whose INCLUSIVE far
        // corner (x+w-1, y) = (14,13) leaves the diamond (sum 27) fails — proves the
        // far corner uses w-1 AND that the corner predicate is the diamond.
        assert!(!check_occupancy_rect(occupancy_with_bounds(CellRect::new(
            13, 13, 2, 1
        ))));

        // A point inside both diagonals but with sum just above the strict low edge
        // (sum 13 > 12) passes; the same cell pair off the low edge (sum 12) fails.
        assert!(check_occupancy_rect(occupancy_with_bounds(
            CellRect::single(7, 6)
        )));
        assert!(!check_occupancy_rect(occupancy_with_bounds(
            CellRect::single(6, 6)
        )));
    }

    #[test]
    fn occupancy_zero_size_rect_still_runs_playfield_corners() {
        // A 0-size rect is NOT a no-op and NOT an auto-pass: with width=0/height=0 the
        // far corners become (x-1, y)/(x, y-1)/(x-1, y-1), so all four corners are
        // evaluated at DECREMENTED coords and each must still satisfy the diamond.
        //
        // At (13,13) the decremented corners (12,13)/(13,12)/(12,12) have sums
        // 25/25/24 — all inside (12 < sum <= 26) — so the 0-size rect PASSES.
        assert!(check_occupancy_rect(occupancy_with_bounds(CellRect::new(
            13, 13, 0, 0
        ))));

        // At (7,6) the NW corner (sum 13) is inside, but the decremented NE corner
        // (6,6) has sum 12, which fails the strict low edge (12 < 12 is false). So the
        // 0-size rect FAILS even though its (undecremented) NW corner is inside —
        // exactly the engine's decremented-corner behavior. The corresponding 1x1
        // rect at (7,6) passes (its corners are all (7,6), sum 13).
        assert!(!check_occupancy_rect(occupancy_with_bounds(CellRect::new(
            7, 6, 0, 0
        ))));
        assert!(check_occupancy_rect(occupancy_with_bounds(
            CellRect::single(7, 6)
        )));
    }

    #[test]
    fn gsi_04_05_reservation_masks_aliases_dummy_and_neighbors_are_native_shaped() {
        assert_eq!(reservation_mask(-1), 0);
        assert_eq!(reservation_mask(-2), 1 << 30);
        assert_eq!(reservation_mask(32), 1);
        assert_eq!(reservation_mask(63), 1 << 31);

        let mut grid = CellReservationGrid::new();
        grid.reserve(None, -1, 1, 32);
        assert_eq!(grid.raw_mask(None, 511, 0), 1);
        grid.reserve(None, 512, 0, 1);
        assert_eq!(grid.raw_mask(None, 0, 1), 2);

        grid.reserve(None, -1, 0, 3);
        assert_eq!(grid.raw_mask(None, -512, 0), 1 << 3);
        assert_eq!(grid.dummy_mask(), 1 << 3);
        grid.clear(None, -513, 0, 3);
        assert_eq!(grid.dummy_mask(), 0, "every invalid lookup shares +0xDC");

        let mut neighbors = CellReservationGrid::new();
        neighbors.reserve(None, 10, 10, 4);
        assert_eq!(
            neighbors.house_reservation_neighbor_mask(None, 9, 9, 4),
            u32::MAX
        );
        assert_eq!(
            neighbors.house_reservation_neighbor_mask(None, 10, 10, 4),
            0
        );
        for (dx, dy) in [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ] {
            neighbors.reserve(None, 10 + dx, 10 + dy, 4);
        }
        assert_eq!(
            neighbors.house_reservation_neighbor_mask(None, 10, 10, 4),
            0xff
        );
    }

    #[test]
    fn gsi_04_05_reservation_valid_unallocated_slots_share_dummy_but_allocated_alias_is_real() {
        let mut terrain = flat_terrain(3, 1);
        terrain.test_set_native_allocated_cells(&[(0, 0)]);
        assert!(terrain.cell(0, 0).is_some());
        assert!(terrain.cell(1, 0).is_none());
        assert!(terrain.cell(2, 0).is_none());

        let mut grid = CellReservationGrid::new();
        grid.reserve(Some(&terrain), 1, 0, 6);
        assert_eq!(grid.raw_mask(Some(&terrain), 2, 0), 1 << 6);
        assert!(grid.has_reservation_inclusive(Some(&terrain), 2, 0, 2, 0, 6));
        assert_eq!(
            grid.house_reservation_neighbor_mask(Some(&terrain), 2, 0, 6),
            0xff,
            "the center and all null neighbors dereference the shared dummy"
        );
        assert!(!check_occupancy_rect(CellRectOccupancyContext {
            rect: CellRect::single(2, 0),
            reservation_arg: 6,
            reservations: Some(&grid),
            occupancy: None,
            entities: None,
            terrain_object_cells: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            playfield_bounds: None,
        }));

        grid.reserve(Some(&terrain), 0, 0, 7);
        assert_eq!(grid.raw_mask(Some(&terrain), 0, 0), 1 << 7);
        assert_eq!(grid.raw_mask(Some(&terrain), 1, 0), 1 << 6);
        grid.clear(Some(&terrain), 2, 0, 6);
        assert_eq!(grid.raw_mask(Some(&terrain), 1, 0), 0);
        assert_eq!(grid.raw_mask(Some(&terrain), 0, 0), 1 << 7);

        let alias_terrain = flat_terrain(512, 1);
        let mut aliases = CellReservationGrid::new();
        aliases.reserve(Some(&alias_terrain), -1, 1, 0);
        assert_eq!(aliases.raw_mask(Some(&alias_terrain), 511, 0), 1);
        assert_eq!(aliases.dummy_mask(), 0);
    }

    #[test]
    fn gsi_04_05_reservation_rect_scan_wraps_and_empty_rects_still_test_corners() {
        let mut visited = Vec::new();
        assert!(scan_cell_rect(CellRect::new(i32::MAX, 0, 2, 1), |x, y| {
            visited.push((x, y));
            true
        }));
        assert!(
            visited.is_empty(),
            "wrapped endpoint is below the signed start, so native scan skips"
        );
        assert!(check_occupancy_rect(occupancy_with_bounds(CellRect::new(
            13, 13, 0, 0
        ))));
        assert!(!check_occupancy_rect(occupancy_with_bounds(CellRect::new(
            7, 6, 0, 0
        ))));
        assert!(!check_occupancy_rect(occupancy_with_bounds(CellRect::new(
            7, 6, -1, -1
        ))));
    }

    #[test]
    fn gsi_04_05_reservation_checkoccupancy_first_blocker_order_is_exact() {
        use crate::sim::components::Health;
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;
        use crate::sim::intern::InternedId;

        let mut terrain = flat_terrain(1, 1);
        terrain.cells[0].zone_type = zone_class::WATER;
        terrain.cells[0].slope_type = 2;
        let mut reservations = CellReservationGrid::new();
        reservations.reserve(Some(&terrain), 0, 0, 0);
        let mut overlays = OverlayGrid::new(1, 1);
        overlays.place_overlay(0, 0, 7, 0);
        let mut terrain_objects = BTreeMap::new();
        terrain_objects.insert((0, 0), 88);
        let mut entities = EntityStore::new();
        entities.insert(GameEntity::new_at_frame_zero_for_test(
            1,
            0,
            0,
            0,
            0,
            InternedId::from_index(0),
            Health {
                current: 10,
                max: 10,
            },
            InternedId::from_index(1),
            EntityCategory::Structure,
            0,
            0,
            false,
        ));
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            0,
            0,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );

        macro_rules! blocker {
            () => {
                occupancy_blocker_at(
                    &CellRectOccupancyContext {
                        rect: CellRect::single(0, 0),
                        reservation_arg: 0,
                        reservations: Some(&reservations),
                        occupancy: Some(&occupancy),
                        entities: Some(&entities),
                        terrain_object_cells: Some(&terrain_objects),
                        resolved_terrain: Some(&terrain),
                        overlay_grid: Some(&overlays),
                        playfield_bounds: None,
                    },
                    0,
                    0,
                    reservation_mask(0),
                )
            };
        }

        assert_eq!(blocker!(), Some(OccupancyBlocker::TerrainObject));
        terrain_objects.clear();
        assert_eq!(blocker!(), Some(OccupancyBlocker::Reservation));
        reservations.clear(Some(&terrain), 0, 0, 0);
        assert_eq!(blocker!(), Some(OccupancyBlocker::Overlay));
        overlays.clear_overlay(0, 0);
        assert_eq!(blocker!(), Some(OccupancyBlocker::ZoneType));
        terrain.cells[0].zone_type = zone_class::GROUND;
        assert_eq!(blocker!(), Some(OccupancyBlocker::Slope));
        terrain.cells[0].slope_type = 0;
        assert_eq!(blocker!(), Some(OccupancyBlocker::Building));
    }

    #[test]
    fn gsi_04_05_reservation_checkoccupancy_ignores_deck_buildings_and_ground_units() {
        use crate::sim::components::Health;
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;
        use crate::sim::intern::InternedId;

        let terrain = flat_terrain(2, 1);
        let mut entities = EntityStore::new();
        for (stable_id, category, rx) in [
            (1, EntityCategory::Structure, 0),
            (2, EntityCategory::Unit, 1),
        ] {
            entities.insert(GameEntity::new_at_frame_zero_for_test(
                stable_id,
                rx,
                0,
                0,
                0,
                InternedId::from_index(0),
                Health {
                    current: 10,
                    max: 10,
                },
                InternedId::from_index(stable_id as u32),
                category,
                0,
                0,
                false,
            ));
        }
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            0,
            0,
            1,
            MovementLayer::Bridge,
            None,
            CellListInsertion::AppendBuilding,
        );
        occupancy.add(
            1,
            0,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        for x in 0..=1 {
            assert_eq!(
                occupancy_blocker_at(
                    &CellRectOccupancyContext {
                        rect: CellRect::single(x, 0),
                        reservation_arg: -1,
                        reservations: None,
                        occupancy: Some(&occupancy),
                        entities: Some(&entities),
                        terrain_object_cells: None,
                        resolved_terrain: Some(&terrain),
                        overlay_grid: None,
                        playfield_bounds: None,
                    },
                    i32::from(x),
                    0,
                    0,
                ),
                None
            );
        }
    }
}
