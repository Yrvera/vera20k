//! Persistent per-cell occupancy grid — tracks which entities occupy each map cell.
//!
//! Replaces the ephemeral `build_occupancy_maps()` approach with an incrementally
//! maintained grid. Entities are added on spawn/move-in, removed on death/move-out.
//! Structures occupy all their foundation cells.
//!
//! Unified single grid with layer-tagged occupants (no separate ground/bridge maps).
//! Equivalent to the original engine's CellClass::FirstObject/AltObject linked lists.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/movement/locomotor (MovementLayer).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::sim::components::{DriveLocomotionRuntime, DriveOccupationFootprint};
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::locomotor::MovementLayer;
use crate::util::fixed_math::{SIM_ZERO, SimFixed};
use crate::util::native_x87::{X87Chop53, sqrt_approx_f32};

/// UnitClass vehicle-occupation bit in both CellClass occupation planes.
pub(crate) const VEHICLE_OCCUPATION_BIT: u8 = 0x20;
/// Generic ObjectClass occupation bit used by landed AircraftClass objects.
pub(crate) const OBJECT_OCCUPATION_BIT: u8 = 0x40;
pub(crate) const BUILDING_OCCUPATION_BIT: u8 = 0x80;

/// Object-list layer after the native display-layer eligibility gate.
/// Aircraft use Fly height rather than their Air path layer; every other
/// category retains the existing locomotor-layer gate and OnBridge selector.
pub(crate) fn cell_list_layer_for_entity(entity: &GameEntity) -> Option<MovementLayer> {
    if entity.category != EntityCategory::Aircraft {
        return entity.occupancy_list_layer();
    }
    let locomotor = entity.locomotor.as_ref()?;
    if locomotor.kind != crate::rules::locomotor_type::LocomotorKind::Fly
        || locomotor.altitude > SIM_ZERO
    {
        return None;
    }
    Some(if entity.on_bridge {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    })
}

/// Convert a coordinate's exact intra-cell position into the raw Infantry
/// occupation mask. The native selector has no sub-cell-1 result: center and
/// the northwest quadrant both select bit 0, while the other quadrants select
/// bits 2, 3, and 4.
pub(crate) fn infantry_raw_occupation_mask(sub_x: SimFixed, sub_y: SimFixed) -> u8 {
    const CELL_LEPTON_MASK: i32 = 0xff;
    const CELL_CENTER: i32 = 128;
    const CENTER_RADIUS: i64 = 60;

    let dx = (sub_x.to_num::<i32>() & CELL_LEPTON_MASK) - CELL_CENTER;
    let dy = (sub_y.to_num::<i32>() & CELL_LEPTON_MASK) - CELL_CENTER;
    let squared = X87Chop53::load_i32(dx * dx + dy * dy);
    let root_bits = sqrt_approx_f32(squared)
        .expect("intra-cell squared distance is finite and representable as f32");
    let root = X87Chop53::load_f32(root_bits)
        .expect("intra-cell approximate distance is finite and normal or zero");
    let radius =
        X87Chop53::ftol_i64(root).expect("intra-cell approximate distance fits a signed integer");
    let sub_cell = if radius < CENTER_RADIUS {
        0
    } else {
        let quadrant = u8::from(dx > 0) | (u8::from(dy > 0) << 1);
        if quadrant == 0 { 0 } else { quadrant + 1 }
    };
    1 << sub_cell
}

/// One cell's two independent raw occupation bytes.
///
/// These bytes are authoritative state. They deliberately carry no owner or
/// reference-count information: producers OR their mask when marking and clear
/// that mask destructively when unmarking.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct RawCellOccupation {
    ground: u8,
    deck: u8,
}

/// Sparse canonical storage for the raw ground/deck occupation bytes.
///
/// An absent entry is exactly `(ground = 0, deck = 0)`. Zero entries are
/// removed eagerly, so serialization and deterministic hashing have one
/// representation for an unoccupied cell. This state is independent of both
/// the CellClass-style object lists and the owner-aware Drive compatibility
/// cache below.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct RawCellOccupationGrid {
    cells: BTreeMap<(u16, u16), RawCellOccupation>,
}

impl RawCellOccupationGrid {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn mark_ground(&mut self, rx: u16, ry: u16, mask: u8) {
        if mask != 0 {
            self.cells.entry((rx, ry)).or_default().ground |= mask;
        }
    }

    pub(crate) fn clear_ground(&mut self, rx: u16, ry: u16, mask: u8) {
        self.update_and_prune(rx, ry, |cell| cell.ground &= !mask);
    }

    pub(crate) fn ground_bits(&self, rx: u16, ry: u16) -> u8 {
        self.cells.get(&(rx, ry)).map_or(0, |cell| cell.ground)
    }

    /// The active bridge-avoidance consumer treats every nonzero ground byte as
    /// occupied, including the consumer-visible bit without an active producer.
    pub(crate) fn ground_is_occupied(&self, rx: u16, ry: u16) -> bool {
        self.ground_bits(rx, ry) != 0
    }

    pub(crate) fn mark_deck(&mut self, rx: u16, ry: u16, mask: u8) {
        if mask != 0 {
            self.cells.entry((rx, ry)).or_default().deck |= mask;
        }
    }

    pub(crate) fn clear_deck(&mut self, rx: u16, ry: u16, mask: u8) {
        self.update_and_prune(rx, ry, |cell| cell.deck &= !mask);
    }

    pub(crate) fn deck_bits(&self, rx: u16, ry: u16) -> u8 {
        self.cells.get(&(rx, ry)).map_or(0, |cell| cell.deck)
    }

    fn update_and_prune(&mut self, rx: u16, ry: u16, update: impl FnOnce(&mut RawCellOccupation)) {
        let key = (rx, ry);
        let should_remove = self.cells.get_mut(&key).is_some_and(|cell| {
            update(cell);
            cell.ground == 0 && cell.deck == 0
        });
        if should_remove {
            self.cells.remove(&key);
        }
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.cells.len()
    }

    /// Canonical coordinate-key iteration used by the deterministic hash.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (u16, u16, u8, u8)> + '_ {
        self.cells
            .iter()
            .map(|(&(rx, ry), cell)| (rx, ry, cell.ground, cell.deck))
    }
}

/// Entity-aware ownership behind one native occupation plane.
///
/// The public observation remains the native ORed bit. Keeping the contributing
/// entity IDs lets a Unit ignore its own head-to mark without manufacturing a
/// phantom `CellOccupant` in the destination cell.
#[derive(Debug, Clone, Default)]
struct VehicleOccupationPlane {
    owners: BTreeMap<u64, u8>,
}

impl VehicleOccupationPlane {
    fn mark(&mut self, entity_id: u64) {
        self.owners.insert(entity_id, VEHICLE_OCCUPATION_BIT);
    }

    fn clear(&mut self, entity_id: u64) {
        self.owners.remove(&entity_id);
    }

    fn bits(&self) -> u8 {
        self.owners
            .values()
            .copied()
            .fold(0, |bits, mark| bits | mark)
    }

    fn bits_ignoring(&self, entity_id: u64) -> u8 {
        self.owners
            .iter()
            .filter(|(owner, _)| **owner != entity_id)
            .map(|(_, mark)| *mark)
            .fold(0, |bits, mark| bits | mark)
    }

    fn is_empty(&self) -> bool {
        self.owners.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
struct CellVehicleOccupation {
    ground: VehicleOccupationPlane,
    deck: VehicleOccupationPlane,
}

impl CellVehicleOccupation {
    fn plane(&self, layer: MovementLayer) -> Option<&VehicleOccupationPlane> {
        match layer {
            MovementLayer::Ground => Some(&self.ground),
            MovementLayer::Bridge => Some(&self.deck),
            MovementLayer::Air | MovementLayer::Underground => None,
        }
    }

    fn plane_mut(&mut self, layer: MovementLayer) -> Option<&mut VehicleOccupationPlane> {
        match layer {
            MovementLayer::Ground => Some(&mut self.ground),
            MovementLayer::Bridge => Some(&mut self.deck),
            MovementLayer::Air | MovementLayer::Underground => None,
        }
    }

    fn is_empty(&self) -> bool {
        self.ground.is_empty() && self.deck.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CellOccupationFootprint {
    rx: u16,
    ry: u16,
    layer: MovementLayer,
}

/// Reverse lookup for one Unit's at-most-current/head occupation marks.
///
/// Native storage is one ORed bit per owner and plane, not a reference count:
/// coincident current/head roles therefore occupy one slot here as well.
#[derive(Debug, Clone, Copy, Default)]
struct OwnerOccupationFootprints {
    first: Option<CellOccupationFootprint>,
    second: Option<CellOccupationFootprint>,
}

impl OwnerOccupationFootprints {
    fn insert(&mut self, footprint: CellOccupationFootprint) {
        if self.first == Some(footprint) || self.second == Some(footprint) {
            return;
        }
        if self.first.is_none() {
            self.first = Some(footprint);
            return;
        }
        if self.second.is_none() {
            self.second = Some(footprint);
            return;
        }
        panic!("CellOccupationGrid owner exceeded current/head footprint bound");
    }

    fn remove(&mut self, footprint: CellOccupationFootprint) {
        if self.first == Some(footprint) {
            self.first = self.second.take();
        } else if self.second == Some(footprint) {
            self.second = None;
        }
    }

    fn iter(self) -> impl Iterator<Item = CellOccupationFootprint> {
        [self.first, self.second].into_iter().flatten()
    }

    fn is_empty(&self) -> bool {
        self.first.is_none() && self.second.is_none()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        usize::from(self.first.is_some()) + usize::from(self.second.is_some())
    }
}

/// Independent CellClass-style vehicle-occupation bit planes.
///
/// This is deliberately separate from [`OccupancyGrid`]: object-list identity
/// and order come only from `CellOccupant`, while an accepted Drive track can
/// mark its head-to cell before the unit is linked there. The index is transient
/// and rebuilt from serialized entity/Drive state.
#[derive(Debug, Clone, Default)]
pub struct CellOccupationGrid {
    cells: BTreeMap<(u16, u16), CellVehicleOccupation>,
    footprints_by_owner: BTreeMap<u64, OwnerOccupationFootprints>,
}

impl CellOccupationGrid {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn rebuild(entities: &crate::sim::entity_store::EntityStore) -> Self {
        let mut grid = Self::new();
        for entity in entities.values() {
            if entity.category != EntityCategory::Unit
                || !entity.lifecycle.cell_marked
                || entity.passenger_role.is_inside_transport()
            {
                continue;
            }
            let current_cleared = entity
                .drive_locomotion
                .as_ref()
                .is_some_and(|drive| drive.current_occupation_cleared);
            if !current_cleared && let Some(layer) = entity.occupancy_list_layer() {
                grid.mark_vehicle_on_layer(
                    entity.position.rx,
                    entity.position.ry,
                    entity.stable_id,
                    layer,
                );
            }
            if let Some(mark) = entity
                .drive_locomotion
                .as_ref()
                .and_then(|drive| drive.occupation_head_to)
            {
                grid.mark_vehicle_on_layer(mark.rx, mark.ry, entity.stable_id, mark.layer);
            }
        }
        grid
    }

    /// Reconcile one entity from its serialized current/head-to footprint.
    ///
    /// Most world command paths update this transient index directly. This
    /// narrow reconciliation also covers internal direct/scatter orders whose
    /// command surface owns only `EntityStore`.
    pub(crate) fn reconcile_entity(&mut self, entity: &GameEntity) {
        let old_footprints = self
            .footprints_by_owner
            .get_mut(&entity.stable_id)
            .map(|footprints| {
                let old = *footprints;
                *footprints = OwnerOccupationFootprints::default();
                old
            })
            .unwrap_or_default();
        for footprint in old_footprints.iter() {
            self.clear_vehicle_plane(footprint, entity.stable_id);
        }

        if entity.category != EntityCategory::Unit
            || !entity.lifecycle.cell_marked
            || entity.passenger_role.is_inside_transport()
        {
            self.footprints_by_owner.remove(&entity.stable_id);
            return;
        }
        let current_cleared = entity
            .drive_locomotion
            .as_ref()
            .is_some_and(|drive| drive.current_occupation_cleared);
        if !current_cleared && let Some(layer) = entity.occupancy_list_layer() {
            self.mark_vehicle_on_layer(
                entity.position.rx,
                entity.position.ry,
                entity.stable_id,
                layer,
            );
        }
        if let Some(head) = entity
            .drive_locomotion
            .as_ref()
            .and_then(|drive| drive.occupation_head_to)
        {
            self.mark_vehicle_on_layer(head.rx, head.ry, entity.stable_id, head.layer);
        }
        if self
            .footprints_by_owner
            .get(&entity.stable_id)
            .is_some_and(OwnerOccupationFootprints::is_empty)
        {
            self.footprints_by_owner.remove(&entity.stable_id);
        }
    }

    /// Exact explicit-plane mark. Ground/deck storage is independent.
    pub(crate) fn mark_vehicle_on_layer(
        &mut self,
        rx: u16,
        ry: u16,
        entity_id: u64,
        layer: MovementLayer,
    ) {
        if !matches!(layer, MovementLayer::Ground | MovementLayer::Bridge) {
            return;
        }
        let cell = self.cells.entry((rx, ry)).or_default();
        cell.plane_mut(layer)
            .expect("ground/deck layer has an occupation plane")
            .mark(entity_id);
        self.footprints_by_owner
            .entry(entity_id)
            .or_default()
            .insert(CellOccupationFootprint { rx, ry, layer });
    }

    /// Exact explicit-plane clear. It intentionally accepts the selected layer
    /// directly so an elevated clear never depends on a cell's current bridge
    /// structural flag.
    pub(crate) fn clear_vehicle_on_layer(
        &mut self,
        rx: u16,
        ry: u16,
        entity_id: u64,
        layer: MovementLayer,
    ) {
        let footprint = CellOccupationFootprint { rx, ry, layer };
        self.clear_vehicle_plane(footprint, entity_id);
        let remove_owner = self
            .footprints_by_owner
            .get_mut(&entity_id)
            .is_some_and(|footprints| {
                footprints.remove(footprint);
                footprints.is_empty()
            });
        if remove_owner {
            self.footprints_by_owner.remove(&entity_id);
        }
    }

    fn clear_vehicle_plane(&mut self, footprint: CellOccupationFootprint, entity_id: u64) {
        let remove_cell = self
            .cells
            .get_mut(&(footprint.rx, footprint.ry))
            .is_some_and(|cell| {
                if let Some(plane) = cell.plane_mut(footprint.layer) {
                    plane.clear(entity_id);
                }
                cell.is_empty()
            });
        if remove_cell {
            self.cells.remove(&(footprint.rx, footprint.ry));
        }
    }

    /// Mark-layer selection: elevated objects use the deck only when the cell
    /// still carries the structural bridge fact.
    pub(crate) fn mark_vehicle_by_height(
        &mut self,
        rx: u16,
        ry: u16,
        entity_id: u64,
        at_or_above_bridge_height: bool,
        has_structural_bridge: bool,
    ) -> MovementLayer {
        let layer = if at_or_above_bridge_height && has_structural_bridge {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        self.mark_vehicle_on_layer(rx, ry, entity_id, layer);
        layer
    }

    /// Clear-layer selection: the height result alone selects the deck. This
    /// preserves cleanup after the structural bridge flag has disappeared.
    pub(crate) fn clear_vehicle_by_height(
        &mut self,
        rx: u16,
        ry: u16,
        entity_id: u64,
        at_or_above_bridge_height: bool,
    ) -> MovementLayer {
        let layer = if at_or_above_bridge_height {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        self.clear_vehicle_on_layer(rx, ry, entity_id, layer);
        layer
    }

    pub(crate) fn vehicle_bits(&self, rx: u16, ry: u16, layer: MovementLayer) -> u8 {
        self.cells
            .get(&(rx, ry))
            .and_then(|cell| cell.plane(layer))
            .map_or(0, VehicleOccupationPlane::bits)
    }

    pub(crate) fn vehicle_bits_ignoring(
        &self,
        rx: u16,
        ry: u16,
        layer: MovementLayer,
        entity_id: u64,
    ) -> u8 {
        self.cells
            .get(&(rx, ry))
            .and_then(|cell| cell.plane(layer))
            .map_or(0, |plane| plane.bits_ignoring(entity_id))
    }

    pub(crate) fn occupied_by_other(
        &self,
        rx: u16,
        ry: u16,
        layer: MovementLayer,
        entity_id: u64,
    ) -> bool {
        self.vehicle_bits_ignoring(rx, ry, layer, entity_id) & VEHICLE_OCCUPATION_BIT != 0
    }

    pub(crate) fn occupied_cells_ignoring(
        &self,
        layer: MovementLayer,
        entity_id: u64,
    ) -> impl Iterator<Item = (u16, u16)> + '_ {
        self.cells.iter().filter_map(move |(&(rx, ry), cell)| {
            let occupied = cell
                .plane(layer)
                .is_some_and(|plane| plane.bits_ignoring(entity_id) & VEHICLE_OCCUPATION_BIT != 0);
            occupied.then_some((rx, ry))
        })
    }
}

/// Replace the Drive head-to mark without disturbing a still-valid current-cell
/// mark. The old auxiliary cell is cleared before the new one is installed.
pub(crate) fn replace_drive_head_to_occupation(
    drive: &mut DriveLocomotionRuntime,
    occupation: &mut CellOccupationGrid,
    entity_id: u64,
    current_cell: (u16, u16),
    current_layer: MovementLayer,
    next: DriveOccupationFootprint,
) {
    if let Some(old) = drive.occupation_head_to.take() {
        let aliases_marked_current = (old.rx, old.ry, old.layer)
            == (current_cell.0, current_cell.1, current_layer)
            && !drive.current_occupation_cleared;
        if !aliases_marked_current {
            occupation.clear_vehicle_on_layer(old.rx, old.ry, entity_id, old.layer);
        }
    }
    occupation.mark_vehicle_on_layer(next.rx, next.ry, entity_id, next.layer);
    drive.occupation_head_to = Some(next);
}

/// Clear an obsolete head-to mark during accepted track replacement while
/// preserving a coincident committed current-cell mark.
pub(crate) fn clear_drive_head_to_occupation_for_replacement(
    drive: &mut DriveLocomotionRuntime,
    occupation: &mut CellOccupationGrid,
    entity_id: u64,
    current_cell: (u16, u16),
    current_layer: MovementLayer,
) {
    let Some(old) = drive.occupation_head_to.take() else {
        return;
    };
    let aliases_marked_current = (old.rx, old.ry, old.layer)
        == (current_cell.0, current_cell.1, current_layer)
        && !drive.current_occupation_cleared;
    if !aliases_marked_current {
        occupation.clear_vehicle_on_layer(old.rx, old.ry, entity_id, old.layer);
    }
}

/// A paid Drive point clears the owner's current-coordinate occupation before
/// the coordinate commit. Object-list membership is intentionally untouched.
pub(crate) fn clear_current_drive_occupation_for_paid_point(
    drive: &mut DriveLocomotionRuntime,
    occupation: &mut CellOccupationGrid,
    entity_id: u64,
    current_cell: (u16, u16),
    current_layer: MovementLayer,
) {
    occupation.clear_vehicle_on_layer(current_cell.0, current_cell.1, entity_id, current_layer);
    drive.current_occupation_cleared = true;
}

/// AddContent after an actual cell crossing re-marks the committed cell.
pub(crate) fn mark_current_drive_occupation_after_crossing(
    drive: &mut DriveLocomotionRuntime,
    occupation: &mut CellOccupationGrid,
    entity_id: u64,
    current_cell: (u16, u16),
    current_layer: MovementLayer,
) {
    occupation.mark_vehicle_on_layer(current_cell.0, current_cell.1, entity_id, current_layer);
    drive.current_occupation_cleared = false;
}

/// Normal track completion promotes an aliased head-to mark into the current
/// mark. There is no endpoint clear when the unit has actually relinked there.
pub(crate) fn finish_drive_head_to_occupation(
    drive: &mut DriveLocomotionRuntime,
    occupation: &mut CellOccupationGrid,
    entity_id: u64,
    current_cell: (u16, u16),
    current_layer: MovementLayer,
) {
    let Some(head) = drive.occupation_head_to.take() else {
        return;
    };
    if (head.rx, head.ry, head.layer) == (current_cell.0, current_cell.1, current_layer) {
        occupation.mark_vehicle_on_layer(current_cell.0, current_cell.1, entity_id, current_layer);
        drive.current_occupation_cleared = false;
    } else {
        occupation.clear_vehicle_on_layer(head.rx, head.ry, entity_id, head.layer);
    }
}

/// Hard limbo/world removal clears the pending head-to cell before the ordinary
/// current-cell RemoveContent clear.
pub(crate) fn clear_drive_head_to_occupation_for_remove(
    drive: &mut DriveLocomotionRuntime,
    occupation: &mut CellOccupationGrid,
    entity_id: u64,
) {
    if let Some(head) = drive.occupation_head_to.take() {
        occupation.clear_vehicle_on_layer(head.rx, head.ry, entity_id, head.layer);
    }
}

/// Single occupant entry in a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellOccupant {
    pub entity_id: u64,
    pub layer: MovementLayer,
    /// Infantry sub-cell (2, 3, or 4). None for vehicles/structures.
    pub sub_cell: Option<u8>,
    /// Whether this occupant is a structure, carried over from the insertion
    /// category. gamemd's per-cell building lookup walks the object list and
    /// returns only BuildingClass objects, so gates that ask "is there a
    /// building in this cell" must not be satisfied by a tank parked on it.
    /// Insertion order alone cannot answer that — a lone occupant carries no
    /// ordering information — so the category is recorded per occupant.
    pub is_building: bool,
}

/// Requested insertion order for a cell's selected gamemd object list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellListInsertion {
    PrependNonBuilding,
    AppendBuilding,
}

impl CellListInsertion {
    pub fn from_category(category: EntityCategory) -> Self {
        if category == EntityCategory::Structure {
            Self::AppendBuilding
        } else {
            Self::PrependNonBuilding
        }
    }
}

/// All occupants of a single cell.
#[derive(Debug, Clone, Default)]
pub struct CellOccupancy {
    /// Occupant list. Common case is 0-3 infantry or 1 vehicle per cell.
    pub occupants: Vec<CellOccupant>,
}

impl CellOccupancy {
    /// All occupants on a selected movement layer in gamemd list order.
    pub fn iter_layer(&self, layer: MovementLayer) -> impl Iterator<Item = &CellOccupant> + '_ {
        self.occupants.iter().filter(move |o| o.layer == layer)
    }

    /// Non-infantry occupants on a given layer, preserving layer-list order.
    pub fn blockers(&self, layer: MovementLayer) -> impl Iterator<Item = u64> + '_ {
        self.occupants
            .iter()
            .filter(move |o| o.layer == layer && o.sub_cell.is_none())
            .map(|o| o.entity_id)
    }

    /// Infantry occupants on a given layer, preserving layer-list order.
    pub fn infantry(&self, layer: MovementLayer) -> impl Iterator<Item = (u64, u8)> + '_ {
        self.occupants
            .iter()
            .filter(move |o| o.layer == layer && o.sub_cell.is_some())
            .map(|o| (o.entity_id, o.sub_cell.unwrap()))
    }

    /// Whether this cell has any occupants on the given layer.
    pub fn is_empty_on(&self, layer: MovementLayer) -> bool {
        !self.occupants.iter().any(|o| o.layer == layer)
    }

    /// Whether this cell has any non-infantry occupants on the given layer.
    pub fn has_blockers_on(&self, layer: MovementLayer) -> bool {
        self.occupants
            .iter()
            .any(|o| o.layer == layer && o.sub_cell.is_none())
    }

    /// Whether this cell holds a structure on the given layer.
    ///
    /// Strictly narrower than `has_blockers_on`, which also reports vehicles.
    /// This is the predicate for gamemd's per-cell building lookup, which
    /// returns only BuildingClass objects.
    pub fn has_building_on(&self, layer: MovementLayer) -> bool {
        self.occupants
            .iter()
            .any(|o| o.layer == layer && o.is_building)
    }

    /// Count occupants on a given layer.
    pub fn count_on(&self, layer: MovementLayer) -> usize {
        self.occupants.iter().filter(|o| o.layer == layer).count()
    }
}

/// Persistent per-cell occupancy index, owned by `ObjectSubstrate`.
///
/// Mirrors entity positions: every entity that occupies a map cell has an entry.
/// Structures occupy all their foundation cells. Maintained incrementally — add
/// on spawn/move-in, remove on death/move-out.
#[derive(Debug, Clone)]
pub struct OccupancyGrid {
    cells: BTreeMap<(u16, u16), CellOccupancy>,
    /// Monotonic counter bumped on every cell-membership mutation. Lets the
    /// movement tick detect when a mover's pathfinding entity-block snapshot is
    /// stale and must be rebuilt before a same-tick repath. Transient scheduling
    /// state only: never serialized, never part of the state hash — it gates
    /// *when* a deterministic rebuild happens, never *what* it produces.
    generation: u64,
}

impl Default for OccupancyGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl OccupancyGrid {
    /// Rebuild occupancy from scratch by scanning all entities.
    /// Used at map load (deserialization) and for debug validation.
    pub fn rebuild(entities: &crate::sim::entity_store::EntityStore) -> Self {
        let mut grid = Self::new();
        let mut ordered: Vec<&GameEntity> = entities.values().collect();
        ordered.sort_by_key(|entity| (entity.occupancy_enter_order, entity.stable_id));
        for entity in ordered {
            // Global storage, native-alive, limbo, and cell-list membership are
            // independent facts. Only an object whose Mark transaction succeeded
            // participates in this rebuilt cache.
            if !entity.lifecycle.cell_marked {
                continue;
            }
            // Entities inside transports don't occupy cells.
            if entity.passenger_role.is_inside_transport() {
                continue;
            }
            let Some(layer) = cell_list_layer_for_entity(entity) else {
                continue;
            };
            let sid = entity.stable_id;
            let sub = if entity.category == EntityCategory::Infantry {
                entity.sub_cell
            } else {
                None
            };
            let insertion = CellListInsertion::from_category(entity.category);
            for (rx, ry) in entity_occupancy_cells(entity) {
                grid.add(rx, ry, sid, layer, sub, insertion);
            }
        }
        grid
    }
}

pub(crate) fn entity_occupancy_cells(entity: &GameEntity) -> Vec<(u16, u16)> {
    if entity.category != EntityCategory::Structure {
        return vec![(entity.position.rx, entity.position.ry)];
    }

    let (w, h) = crate::rules::foundation::foundation_dimensions(&entity.foundation);
    let mut cells = Vec::with_capacity(w as usize * h as usize);
    for dx in 0..w {
        for dy in 0..h {
            let Some(rx) = entity.position.rx.checked_add(dx) else {
                continue;
            };
            let Some(ry) = entity.position.ry.checked_add(dy) else {
                continue;
            };
            cells.push((rx, ry));
        }
    }
    cells
}

impl OccupancyGrid {
    /// Create an empty occupancy grid.
    pub fn new() -> Self {
        Self {
            cells: BTreeMap::new(),
            generation: 0,
        }
    }

    /// Current mutation generation. Bumped on every `add`/`remove`/`update_sub_cell`
    /// (and thus `move_entity`). Compare across two points to detect whether cell
    /// membership changed in between. Transient — not hashed, resets to 0 on rebuild.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Add an entity to a cell. For structures, caller must invoke once per
    /// foundation cell.
    pub fn add(
        &mut self,
        rx: u16,
        ry: u16,
        entity_id: u64,
        layer: MovementLayer,
        sub_cell: Option<u8>,
        insertion: CellListInsertion,
    ) {
        self.generation = self.generation.wrapping_add(1);
        let new_occupant = CellOccupant {
            entity_id,
            layer,
            sub_cell,
            is_building: insertion == CellListInsertion::AppendBuilding,
        };
        let occ = self.cells.entry((rx, ry)).or_default();
        match insertion {
            CellListInsertion::PrependNonBuilding => {
                let index = occ
                    .occupants
                    .iter()
                    .position(|o| o.layer == layer)
                    .unwrap_or(0);
                occ.occupants.insert(index, new_occupant);
            }
            CellListInsertion::AppendBuilding => {
                let index = occ
                    .occupants
                    .iter()
                    .rposition(|o| o.layer == layer)
                    .map_or(occ.occupants.len(), |i| i + 1);
                occ.occupants.insert(index, new_occupant);
            }
        }
    }

    /// Remove an entity from a cell, walking ALL layers. No-op if entity not found.
    /// For structures, caller must invoke once per foundation cell.
    pub fn remove(&mut self, rx: u16, ry: u16, entity_id: u64) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(occ) = self.cells.get_mut(&(rx, ry)) {
            occ.occupants.retain(|o| o.entity_id != entity_id);
            if occ.occupants.is_empty() {
                self.cells.remove(&(rx, ry));
            }
        }
    }

    /// Remove an entity from ONLY the given layer's object list in a cell.
    ///
    /// This is the gamemd-native `RemoveContent` behavior: it walks only the
    /// selected per-cell list (ground vs bridge/deck) chosen by the occupant's
    /// `OnBridge` byte at the call site, never the other layer. On a bridge cell
    /// crossing the removal must observe the OLD (pre-transition) layer — see
    /// `move_entity_layered`. No-op if no matching-layer entry exists.
    ///
    /// For the single-entry-per-cell invariant this grid maintains (each entity is
    /// `add`ed exactly once per cell), the per-layer result equals the layer-agnostic
    /// `remove`; the per-layer form is the authoritative two-layer selector and keeps
    /// the remove/add halves on the verified independent layers.
    pub fn remove_on_layer(&mut self, rx: u16, ry: u16, entity_id: u64, layer: MovementLayer) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(occ) = self.cells.get_mut(&(rx, ry)) {
            occ.occupants
                .retain(|o| !(o.entity_id == entity_id && o.layer == layer));
            if occ.occupants.is_empty() {
                self.cells.remove(&(rx, ry));
            }
        }
    }

    /// Move an entity from one cell to another (layer-agnostic remove + add).
    ///
    /// Convenience for callers that do NOT change the occupant's object-list layer
    /// across the move (teleport, same-layer steps). For a bridge cell crossing that
    /// may flip `on_bridge`, use `move_entity_layered` so the old-cell removal
    /// observes the OLD layer and the new-cell insertion the NEW layer.
    pub fn move_entity(
        &mut self,
        old_rx: u16,
        old_ry: u16,
        new_rx: u16,
        new_ry: u16,
        entity_id: u64,
        layer: MovementLayer,
        sub_cell: Option<u8>,
        insertion: CellListInsertion,
    ) {
        self.remove(old_rx, old_ry, entity_id);
        self.add(new_rx, new_ry, entity_id, layer, sub_cell, insertion);
    }

    /// Authoritative two-layer cell crossing — the verified gamemd order:
    /// 1. remove from the OLD cell on the **OLD** object-list layer (the
    ///    pre-transition `on_bridge` layer; `RemoveContent` walks only that list),
    /// 2. add to the NEW cell on the **NEW** object-list layer (the post-transition
    ///    `on_bridge` layer; `AddContent` selects the list by the new byte).
    ///
    /// Old-cell removal observes the pre-transition layer; new-cell insertion the
    /// post-transition layer. The two halves may target different layers when the
    /// occupant stepped on/off the deck during the crossing — this asymmetry is the
    /// load-bearing part of the contract (the list layer is selected by the
    /// occupant's `OnBridge` byte sampled at each call site, not a single layer
    /// reused for both halves).
    #[allow(clippy::too_many_arguments)]
    pub fn move_entity_layered(
        &mut self,
        old_rx: u16,
        old_ry: u16,
        new_rx: u16,
        new_ry: u16,
        entity_id: u64,
        old_layer: MovementLayer,
        new_layer: MovementLayer,
        sub_cell: Option<u8>,
        insertion: CellListInsertion,
    ) {
        self.remove_on_layer(old_rx, old_ry, entity_id, old_layer);
        self.add(new_rx, new_ry, entity_id, new_layer, sub_cell, insertion);
    }

    /// Update an entity's sub-cell within the same cell.
    pub fn update_sub_cell(&mut self, rx: u16, ry: u16, entity_id: u64, new_sub_cell: Option<u8>) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(occ) = self.cells.get_mut(&(rx, ry)) {
            if let Some(o) = occ.occupants.iter_mut().find(|o| o.entity_id == entity_id) {
                o.sub_cell = new_sub_cell;
            }
        }
    }

    /// Get occupancy for a cell (all layers).
    pub fn get(&self, rx: u16, ry: u16) -> Option<&CellOccupancy> {
        self.cells.get(&(rx, ry))
    }

    /// Check if a cell has no occupants on a given layer.
    pub fn is_empty_on_layer(&self, rx: u16, ry: u16, layer: MovementLayer) -> bool {
        self.cells
            .get(&(rx, ry))
            .map_or(true, |occ| occ.is_empty_on(layer))
    }

    /// Count total occupants on a layer in a cell.
    pub fn count_on_layer(&self, rx: u16, ry: u16, layer: MovementLayer) -> usize {
        self.cells
            .get(&(rx, ry))
            .map_or(0, |occ| occ.count_on(layer))
    }

    /// Check if a specific entity is in a specific cell.
    pub fn contains_entity(&self, rx: u16, ry: u16, entity_id: u64) -> bool {
        self.cells
            .get(&(rx, ry))
            .is_some_and(|occ| occ.occupants.iter().any(|o| o.entity_id == entity_id))
    }

    /// Total number of occupied cells (for diagnostics).
    pub fn occupied_cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Assert that this grid matches an expected grid. Panics with a diff on mismatch.
    /// Only compiled in debug builds — used as a safety net after each tick.
    #[cfg(debug_assertions)]
    pub fn debug_assert_matches(&self, expected: &OccupancyGrid) {
        let self_cells: std::collections::BTreeSet<(u16, u16)> =
            self.cells.keys().copied().collect();
        let expected_cells: std::collections::BTreeSet<(u16, u16)> =
            expected.cells.keys().copied().collect();
        let missing: Vec<_> = expected_cells.difference(&self_cells).collect();
        let extra: Vec<_> = self_cells.difference(&expected_cells).collect();
        if !missing.is_empty() || !extra.is_empty() {
            panic!(
                "OccupancyGrid mismatch: {} missing cells, {} extra cells.\n\
                 Missing (expected but not in grid): {:?}\n\
                 Extra (in grid but not expected): {:?}",
                missing.len(),
                extra.len(),
                &missing[..missing.len().min(10)],
                &extra[..extra.len().min(10)],
            );
        }
        for (&cell, expected_occ) in &expected.cells {
            let actual_occ = self.cells.get(&cell).unwrap();
            if actual_occ.occupants != expected_occ.occupants {
                panic!(
                    "OccupancyGrid mismatch at cell ({},{}): expected {:?}, got {:?}",
                    cell.0, cell.1, expected_occ.occupants, actual_occ.occupants,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gsi_04_12_raw_occupation_preserves_every_raw_bit() {
        let mut grid = RawCellOccupationGrid::new();
        for mask in [
            0x01,
            0x02,
            0x04,
            0x08,
            0x10,
            VEHICLE_OCCUPATION_BIT,
            0x40,
            0x80,
        ] {
            grid.mark_ground(7, 9, mask);
        }

        assert_eq!(grid.ground_bits(7, 9), u8::MAX);
        assert!(grid.ground_is_occupied(7, 9));
        assert_eq!(grid.ground_bits(9, 7), 0);
        assert!(!grid.ground_is_occupied(9, 7));
    }

    #[test]
    fn gsi_04_12_raw_occupation_clear_is_destructive_not_reference_counted() {
        let mut grid = RawCellOccupationGrid::new();
        grid.mark_ground(3, 4, VEHICLE_OCCUPATION_BIT);
        grid.mark_ground(3, 4, VEHICLE_OCCUPATION_BIT);
        assert_eq!(grid.ground_bits(3, 4), VEHICLE_OCCUPATION_BIT);

        grid.clear_ground(3, 4, VEHICLE_OCCUPATION_BIT);

        assert_eq!(grid.ground_bits(3, 4), 0);
        assert_eq!(grid.entry_count(), 0);
    }

    #[test]
    fn gsi_04_12_raw_occupation_ground_and_deck_planes_are_independent() {
        let mut grid = RawCellOccupationGrid::new();
        grid.mark_ground(11, 12, 0x21);
        grid.mark_deck(11, 12, 0xC0);
        assert_eq!(grid.ground_bits(11, 12), 0x21);
        assert_eq!(grid.deck_bits(11, 12), 0xC0);

        grid.clear_ground(11, 12, VEHICLE_OCCUPATION_BIT);
        assert_eq!(grid.ground_bits(11, 12), 0x01);
        assert_eq!(grid.deck_bits(11, 12), 0xC0);

        grid.clear_deck(11, 12, 0x40);
        assert_eq!(grid.ground_bits(11, 12), 0x01);
        assert_eq!(grid.deck_bits(11, 12), 0x80);
    }

    #[test]
    fn generation_bumps_on_every_mutation() {
        let mut grid = OccupancyGrid::new();
        let g0 = grid.generation();
        grid.add(
            1,
            1,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let g1 = grid.generation();
        assert!(g1 > g0, "add must bump generation");
        grid.move_entity(
            1,
            1,
            2,
            2,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let g2 = grid.generation();
        assert!(g2 > g1, "move_entity (remove+add) must bump generation");
        grid.update_sub_cell(2, 2, 10, Some(3));
        let g3 = grid.generation();
        assert!(g3 > g2, "update_sub_cell must bump generation");
        grid.remove(2, 2, 10);
        assert!(grid.generation() > g3, "remove must bump generation");
    }

    #[test]
    fn add_and_get() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let occ = grid.get(5, 5).unwrap();
        assert_eq!(occ.occupants.len(), 1);
        assert_eq!(occ.occupants[0].entity_id, 1);
        assert_eq!(occ.occupants[0].layer, MovementLayer::Ground);
        assert!(occ.occupants[0].sub_cell.is_none());
    }

    #[test]
    fn remove_cleans_up_empty_cell() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.remove(5, 5, 1);
        assert!(grid.get(5, 5).is_none());
        assert_eq!(grid.occupied_cell_count(), 0);
    }

    #[test]
    fn remove_nonexistent_is_noop() {
        let mut grid = OccupancyGrid::new();
        grid.remove(5, 5, 99);
        assert!(grid.get(5, 5).is_none());
    }

    #[test]
    fn move_entity_transfers_between_cells() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.move_entity(
            5,
            5,
            6,
            6,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        assert!(grid.get(5, 5).is_none());
        let occ = grid.get(6, 6).unwrap();
        assert_eq!(occ.occupants.len(), 1);
        assert_eq!(occ.occupants[0].entity_id, 1);
    }

    #[test]
    fn layer_filtering() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            3,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );

        let occ = grid.get(5, 5).unwrap();
        let ground_ids: Vec<u64> = occ
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ground_ids, vec![3, 1]);
        let ground_blockers: Vec<u64> = occ.blockers(MovementLayer::Ground).collect();
        assert_eq!(ground_blockers, vec![1]);
        let bridge_blockers: Vec<u64> = occ.blockers(MovementLayer::Bridge).collect();
        assert_eq!(bridge_blockers, vec![2]);
        let ground_inf: Vec<(u64, u8)> = occ.infantry(MovementLayer::Ground).collect();
        assert_eq!(ground_inf, vec![(3, 2)]);
        assert_eq!(occ.infantry(MovementLayer::Bridge).count(), 0);
    }

    #[test]
    fn is_empty_on_layer() {
        let mut grid = OccupancyGrid::new();
        assert!(grid.is_empty_on_layer(5, 5, MovementLayer::Ground));
        grid.add(
            5,
            5,
            1,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        assert!(grid.is_empty_on_layer(5, 5, MovementLayer::Ground));
        assert!(!grid.is_empty_on_layer(5, 5, MovementLayer::Bridge));
    }

    #[test]
    fn infantry_subcells() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            11,
            MovementLayer::Ground,
            Some(3),
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            12,
            MovementLayer::Ground,
            Some(4),
            CellListInsertion::PrependNonBuilding,
        );

        let occ = grid.get(5, 5).unwrap();
        let inf: Vec<(u64, u8)> = occ.infantry(MovementLayer::Ground).collect();
        assert_eq!(inf.len(), 3);
        assert!(!occ.has_blockers_on(MovementLayer::Ground));
    }

    #[test]
    fn multi_cell_building() {
        let mut grid = OccupancyGrid::new();
        for dy in 0..2u16 {
            for dx in 0..2u16 {
                grid.add(
                    10 + dx,
                    10 + dy,
                    100,
                    MovementLayer::Ground,
                    None,
                    CellListInsertion::AppendBuilding,
                );
            }
        }
        assert!(grid.contains_entity(10, 10, 100));
        assert!(grid.contains_entity(11, 10, 100));
        assert!(grid.contains_entity(10, 11, 100));
        assert!(grid.contains_entity(11, 11, 100));
        assert!(!grid.contains_entity(12, 12, 100));

        for dy in 0..2u16 {
            for dx in 0..2u16 {
                grid.remove(10 + dx, 10 + dy, 100);
            }
        }
        assert_eq!(grid.occupied_cell_count(), 0);
    }

    #[test]
    fn update_sub_cell() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        grid.update_sub_cell(5, 5, 1, Some(4));
        let occ = grid.get(5, 5).unwrap();
        let inf: Vec<(u64, u8)> = occ.infantry(MovementLayer::Ground).collect();
        assert_eq!(inf, vec![(1, 4)]);
    }

    #[test]
    fn count_on_layer() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            3,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        assert_eq!(grid.count_on_layer(5, 5, MovementLayer::Ground), 2);
        assert_eq!(grid.count_on_layer(5, 5, MovementLayer::Bridge), 1);
        assert_eq!(grid.count_on_layer(5, 5, MovementLayer::Air), 0);
    }

    #[test]
    fn non_buildings_prepend_on_same_layer() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let ids: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![2, 1]);
    }

    #[test]
    fn buildings_append_on_same_layer() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            100,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let ids: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![2, 1, 100]);
    }

    #[test]
    fn layers_have_independent_order() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            10,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            20,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let ground: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        let bridge: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Bridge)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ground, vec![2, 1]);
        assert_eq!(bridge, vec![20, 10]);
    }

    #[test]
    fn remove_preserves_remaining_order() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            3,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.remove(5, 5, 2);
        let ids: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![3, 1]);
    }

    #[test]
    fn transition_removes_old_layer_inserts_new_layer() {
        // GATE A2 / P5: the authoritative two-layer crossing removes from the OLD
        // cell on the OLD object-list layer and inserts into the NEW cell on the
        // NEW layer. Step-onto-deck: occupant starts on Ground at the old cell and
        // lands on Bridge at the new cell.
        let mut grid = OccupancyGrid::new();
        grid.add(
            1,
            1,
            9,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.move_entity_layered(
            1,
            1,
            2,
            2,
            9,
            MovementLayer::Ground, // OLD layer
            MovementLayer::Bridge, // NEW layer
            None,
            CellListInsertion::PrependNonBuilding,
        );
        // Old cell emptied (the ground entry was removed); new cell holds the
        // occupant on the BRIDGE layer, not Ground.
        assert!(grid.get(1, 1).is_none());
        assert_eq!(grid.count_on_layer(2, 2, MovementLayer::Ground), 0);
        assert_eq!(grid.count_on_layer(2, 2, MovementLayer::Bridge), 1);
        assert!(grid.contains_entity(2, 2, 9));
    }

    #[test]
    fn remove_on_layer_walks_only_the_selected_layer() {
        // GATE A2: RemoveContent walks ONLY the selected per-cell list. With a
        // single occupant tagged Ground, removing on the WRONG (Bridge) layer is a
        // no-op; removing on the right (Ground) layer removes it.
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            7,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.remove_on_layer(5, 5, 7, MovementLayer::Bridge);
        assert!(
            grid.contains_entity(5, 5, 7),
            "wrong-layer remove must miss"
        );
        grid.remove_on_layer(5, 5, 7, MovementLayer::Ground);
        assert!(grid.get(5, 5).is_none(), "right-layer remove must hit");
    }

    #[test]
    fn move_entity_reinserts_with_requested_order() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            1,
            1,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            2,
            2,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.move_entity(
            1,
            1,
            2,
            2,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let ids: Vec<u64> = grid
            .get(2, 2)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn rebuild_uses_category_insertion() {
        let mut entities = crate::sim::entity_store::EntityStore::new();
        let mut first = crate::sim::game_entity::GameEntity::test_default(1, "E1", "Allies", 5, 5);
        first.category = EntityCategory::Infantry;
        first.sub_cell = Some(2);
        first.lifecycle.cell_marked = true;
        let mut second =
            crate::sim::game_entity::GameEntity::test_default(2, "HTNK", "Allies", 5, 5);
        second.category = EntityCategory::Unit;
        second.lifecycle.cell_marked = true;
        let mut structure =
            crate::sim::game_entity::GameEntity::test_default(100, "GAPOWR", "Allies", 5, 5);
        structure.category = EntityCategory::Structure;
        structure.lifecycle.cell_marked = true;
        entities.insert(first);
        entities.insert(second);
        entities.insert(structure);
        let grid = OccupancyGrid::rebuild(&entities);
        let ids: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![2, 1, 100]);
    }

    #[test]
    fn rebuild_uses_cell_entry_order_not_stable_id_order() {
        let mut entities = crate::sim::entity_store::EntityStore::new();
        let mut structure =
            crate::sim::game_entity::GameEntity::test_default(100, "GAPOWR", "Allies", 5, 5);
        structure.category = EntityCategory::Structure;
        structure.occupancy_enter_order = 1;
        structure.lifecycle.cell_marked = true;
        let mut older_mobile =
            crate::sim::game_entity::GameEntity::test_default(50, "MTNK", "Allies", 5, 5);
        older_mobile.category = EntityCategory::Unit;
        older_mobile.occupancy_enter_order = 2;
        older_mobile.lifecycle.cell_marked = true;
        let mut newer_mobile =
            crate::sim::game_entity::GameEntity::test_default(10, "HTNK", "Allies", 5, 5);
        newer_mobile.category = EntityCategory::Unit;
        newer_mobile.occupancy_enter_order = 3;
        newer_mobile.lifecycle.cell_marked = true;

        entities.insert(newer_mobile);
        entities.insert(older_mobile);
        entities.insert(structure);

        let grid = OccupancyGrid::rebuild(&entities);
        let ids: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![10, 50, 100]);
    }

    #[test]
    fn rebuild_expands_structure_foundation_cells() {
        let mut entities = crate::sim::entity_store::EntityStore::new();
        let mut structure =
            crate::sim::game_entity::GameEntity::test_default(100, "GAPOWR", "Allies", 5, 5);
        structure.category = EntityCategory::Structure;
        structure.foundation = "2x2".to_string();
        structure.lifecycle.cell_marked = true;
        entities.insert(structure);

        let grid = OccupancyGrid::rebuild(&entities);

        for cell in [(5, 5), (5, 6), (6, 5), (6, 6)] {
            assert!(
                grid.contains_entity(cell.0, cell.1, 100),
                "structure should occupy foundation cell {cell:?}"
            );
        }
        assert_eq!(grid.occupied_cell_count(), 4);
    }

    #[test]
    fn lifecycle_authority_alive_limbo_does_not_rebuild_occupancy() {
        let mut entities = crate::sim::entity_store::EntityStore::new();
        let limbo = crate::sim::game_entity::GameEntity::test_default(1, "MTNK", "Allies", 5, 5);

        assert!(limbo.lifecycle.object_alive);
        assert!(limbo.lifecycle.in_limbo);
        assert!(!limbo.lifecycle.cell_marked);
        entities.insert(limbo);

        let grid = OccupancyGrid::rebuild(&entities);
        assert!(!grid.contains_entity(5, 5, 1));
        assert_eq!(grid.occupied_cell_count(), 0);
    }

    fn gsi_04_05_unit(stable_id: u64, rx: u16, ry: u16) -> crate::sim::game_entity::GameEntity {
        let mut entity =
            crate::sim::game_entity::GameEntity::test_default(stable_id, "MTNK", "Allies", rx, ry);
        entity.category = EntityCategory::Unit;
        entity.lifecycle.cell_marked = true;
        entity.locomotor = Some(
            crate::sim::movement::locomotor::LocomotorState::for_test_kind(
                crate::rules::locomotor_type::LocomotorKind::Drive,
            ),
        );
        entity.drive_locomotion = Some(DriveLocomotionRuntime::default());
        entity
    }

    #[test]
    fn gsi_04_05_head_to_premark_is_separate_from_object_list() {
        let mut objects = OccupancyGrid::new();
        objects.add(
            2,
            2,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let mut bits = CellOccupationGrid::new();
        bits.mark_vehicle_on_layer(2, 2, 1, MovementLayer::Ground);
        let mut drive = DriveLocomotionRuntime::default();
        replace_drive_head_to_occupation(
            &mut drive,
            &mut bits,
            1,
            (2, 2),
            MovementLayer::Ground,
            DriveOccupationFootprint {
                rx: 3,
                ry: 2,
                layer: MovementLayer::Ground,
            },
        );

        assert!(objects.contains_entity(2, 2, 1));
        assert!(!objects.contains_entity(3, 2, 1));
        assert_eq!(bits.vehicle_bits(2, 2, MovementLayer::Ground), 0x20);
        assert_eq!(bits.vehicle_bits(3, 2, MovementLayer::Ground), 0x20);
    }

    #[test]
    fn gsi_04_05_paid_same_cell_point_clears_current_not_head_or_list() {
        let mut objects = OccupancyGrid::new();
        objects.add(
            2,
            2,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let mut bits = CellOccupationGrid::new();
        bits.mark_vehicle_on_layer(2, 2, 1, MovementLayer::Ground);
        bits.mark_vehicle_on_layer(3, 2, 1, MovementLayer::Ground);
        let mut drive = DriveLocomotionRuntime {
            occupation_head_to: Some(DriveOccupationFootprint {
                rx: 3,
                ry: 2,
                layer: MovementLayer::Ground,
            }),
            ..Default::default()
        };

        clear_current_drive_occupation_for_paid_point(
            &mut drive,
            &mut bits,
            1,
            (2, 2),
            MovementLayer::Ground,
        );

        assert!(objects.contains_entity(2, 2, 1));
        assert_eq!(bits.vehicle_bits(2, 2, MovementLayer::Ground), 0);
        assert_eq!(bits.vehicle_bits(3, 2, MovementLayer::Ground), 0x20);
        assert!(drive.current_occupation_cleared);
    }

    #[test]
    fn gsi_04_05_actual_crossing_relinks_and_remarks_committed_cell() {
        let mut objects = OccupancyGrid::new();
        objects.add(
            2,
            2,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let mut bits = CellOccupationGrid::new();
        let mut drive = DriveLocomotionRuntime {
            occupation_head_to: Some(DriveOccupationFootprint {
                rx: 3,
                ry: 2,
                layer: MovementLayer::Ground,
            }),
            current_occupation_cleared: true,
            ..Default::default()
        };
        bits.mark_vehicle_on_layer(3, 2, 1, MovementLayer::Ground);

        objects.move_entity(
            2,
            2,
            3,
            2,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        mark_current_drive_occupation_after_crossing(
            &mut drive,
            &mut bits,
            1,
            (3, 2),
            MovementLayer::Ground,
        );

        assert!(!objects.contains_entity(2, 2, 1));
        assert!(objects.contains_entity(3, 2, 1));
        assert_eq!(bits.vehicle_bits(3, 2, MovementLayer::Ground), 0x20);
        assert!(!drive.current_occupation_cleared);
    }

    #[test]
    fn gsi_04_05_soft_stop_preserves_committed_head_to_reservation() {
        let mut entity = gsi_04_05_unit(1, 2, 2);
        let head = DriveOccupationFootprint {
            rx: 3,
            ry: 2,
            layer: MovementLayer::Ground,
        };
        entity.drive_locomotion.as_mut().unwrap().occupation_head_to = Some(head);
        let mut bits = CellOccupationGrid::rebuild(&{
            let mut entities = crate::sim::entity_store::EntityStore::new();
            entities.insert(entity.clone());
            entities
        });

        crate::sim::movement::clear_navigation_for_entity(&mut entity);

        assert_eq!(
            entity.drive_locomotion.as_ref().unwrap().occupation_head_to,
            Some(head)
        );
        assert_eq!(bits.vehicle_bits(3, 2, MovementLayer::Ground), 0x20);
        bits.reconcile_entity(&entity);
        assert_eq!(bits.vehicle_bits(3, 2, MovementLayer::Ground), 0x20);
    }

    #[test]
    fn gsi_04_05_reconcile_one_owner_leaves_large_unrelated_set_untouched() {
        const UNRELATED_OWNERS: u16 = 2_048;

        let mut entity = gsi_04_05_unit(1, 1, 1);
        entity.drive_locomotion.as_mut().unwrap().occupation_head_to =
            Some(DriveOccupationFootprint {
                rx: 2,
                ry: 1,
                layer: MovementLayer::Ground,
            });
        let mut entities = crate::sim::entity_store::EntityStore::new();
        entities.insert(entity.clone());
        let mut bits = CellOccupationGrid::rebuild(&entities);

        for offset in 0..UNRELATED_OWNERS {
            let entity_id = u64::from(offset) + 2;
            let rx = offset + 100;
            let layer = if offset & 1 == 0 {
                MovementLayer::Ground
            } else {
                MovementLayer::Bridge
            };
            bits.mark_vehicle_on_layer(rx, 10, entity_id, layer);
        }

        entity.position.rx = 3;
        entity.drive_locomotion.as_mut().unwrap().occupation_head_to =
            Some(DriveOccupationFootprint {
                rx: 3,
                ry: 1,
                layer: MovementLayer::Ground,
            });
        bits.reconcile_entity(&entity);

        assert_eq!(bits.vehicle_bits(1, 1, MovementLayer::Ground), 0);
        assert_eq!(bits.vehicle_bits(2, 1, MovementLayer::Ground), 0);
        assert_eq!(bits.vehicle_bits(3, 1, MovementLayer::Ground), 0x20);
        assert_eq!(
            bits.footprints_by_owner
                .get(&1)
                .expect("reconciled owner footprint")
                .len(),
            1,
            "coincident current/head roles are one native OR-bit footprint"
        );
        assert_eq!(
            bits.footprints_by_owner.len(),
            usize::from(UNRELATED_OWNERS) + 1
        );
        for offset in 0..UNRELATED_OWNERS {
            let rx = offset + 100;
            let layer = if offset & 1 == 0 {
                MovementLayer::Ground
            } else {
                MovementLayer::Bridge
            };
            assert_eq!(bits.vehicle_bits(rx, 10, layer), 0x20);
        }
    }

    #[test]
    fn gsi_04_05_hard_remove_clears_head_then_current_footprint() {
        let mut bits = CellOccupationGrid::new();
        bits.mark_vehicle_on_layer(2, 2, 1, MovementLayer::Ground);
        bits.mark_vehicle_on_layer(3, 2, 1, MovementLayer::Ground);
        let mut drive = DriveLocomotionRuntime {
            occupation_head_to: Some(DriveOccupationFootprint {
                rx: 3,
                ry: 2,
                layer: MovementLayer::Ground,
            }),
            ..Default::default()
        };

        clear_drive_head_to_occupation_for_remove(&mut drive, &mut bits, 1);
        bits.clear_vehicle_on_layer(2, 2, 1, MovementLayer::Ground);

        assert_eq!(drive.occupation_head_to, None);
        assert_eq!(bits.vehicle_bits(2, 2, MovementLayer::Ground), 0);
        assert_eq!(bits.vehicle_bits(3, 2, MovementLayer::Ground), 0);
    }

    #[test]
    fn gsi_04_05_owner_ignores_own_reservation_other_mover_does_not() {
        let mut bits = CellOccupationGrid::new();
        bits.mark_vehicle_on_layer(3, 2, 1, MovementLayer::Ground);

        assert!(!bits.occupied_by_other(3, 2, MovementLayer::Ground, 1));
        assert!(bits.occupied_by_other(3, 2, MovementLayer::Ground, 2));
    }

    #[test]
    fn gsi_04_05_ground_deck_independent_and_elevated_clear_needs_no_bridge_flag() {
        let mut bits = CellOccupationGrid::new();
        bits.mark_vehicle_by_height(4, 4, 1, false, true);
        bits.mark_vehicle_by_height(4, 4, 2, true, true);
        assert_eq!(bits.vehicle_bits(4, 4, MovementLayer::Ground), 0x20);
        assert_eq!(bits.vehicle_bits(4, 4, MovementLayer::Bridge), 0x20);

        let cleared = bits.clear_vehicle_by_height(4, 4, 2, true);
        assert_eq!(cleared, MovementLayer::Bridge);
        assert_eq!(bits.vehicle_bits(4, 4, MovementLayer::Bridge), 0);
        assert_eq!(bits.vehicle_bits(4, 4, MovementLayer::Ground), 0x20);
    }

    #[test]
    fn gsi_04_05_normal_finish_promotes_endpoint_and_clears_runtime_head() {
        let mut bits = CellOccupationGrid::new();
        bits.mark_vehicle_on_layer(3, 2, 1, MovementLayer::Ground);
        let mut drive = DriveLocomotionRuntime {
            occupation_head_to: Some(DriveOccupationFootprint {
                rx: 3,
                ry: 2,
                layer: MovementLayer::Ground,
            }),
            current_occupation_cleared: true,
            ..Default::default()
        };

        finish_drive_head_to_occupation(&mut drive, &mut bits, 1, (3, 2), MovementLayer::Ground);

        assert_eq!(drive.occupation_head_to, None);
        assert!(!drive.current_occupation_cleared);
        assert_eq!(bits.vehicle_bits(3, 2, MovementLayer::Ground), 0x20);
    }

    #[test]
    fn gsi_04_05_rebuild_restores_active_current_and_head_footprint() {
        let mut entities = crate::sim::entity_store::EntityStore::new();
        let mut unit = gsi_04_05_unit(1, 2, 2);
        unit.drive_locomotion.as_mut().unwrap().occupation_head_to =
            Some(DriveOccupationFootprint {
                rx: 3,
                ry: 2,
                layer: MovementLayer::Ground,
            });
        entities.insert(unit);

        let bits = CellOccupationGrid::rebuild(&entities);

        assert_eq!(bits.vehicle_bits(2, 2, MovementLayer::Ground), 0x20);
        assert_eq!(bits.vehicle_bits(3, 2, MovementLayer::Ground), 0x20);
    }
}
