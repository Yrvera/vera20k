//! Cell entry classification — unified Can_Enter_Cell result codes.
//!
//! The original RA2 engine returns 8 distinct codes when a unit
//! tries to enter a cell. Each code triggers a different movement response.
//! This module centralizes the classification logic that was previously
//! scattered as inline boolean checks in movement.rs.
//!
//! Two-phase design for borrow checker compatibility:
//! - Phase 1 (`check_terrain`): terrain + occupancy presence, no EntityStore needed
//! - Phase 2 (`classify_occupied_cell`): blocker friendship/crush, needs &EntityStore
//!
//! Bridge legality is now driven by A*'s `path_layers` (set per-step by `astar_search`
//! with the Ground→Bridge gates verified against the reference predicate), which
//! approximates the post-switch output of the original two-pass `Can_Enter_Cell`. See
//! docs/plans/2026-05-11-bridge-locomotor-layer-correctness-design.md §"Known Parity Boundary".
//!
//! TODO(RE): Cost-class refinements (search-time entity-block costs vs runtime bump) and
//! some terrain edge cases still pending. Tracked separately from G2/G6.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/bump_crush, sim/entity_store, sim/locomotor,
//!   sim/pathfinding, map/entities, map/houses, rules/locomotor_type.

use std::collections::BTreeSet;

use super::PathGrid;
use super::passability;
use super::terrain_cost::TerrainCostGrid;
use crate::map::entities::EntityCategory;
use crate::map::houses::{self, HouseAllianceMap};
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::bump_crush;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::OccupancyGrid;

// ---------------------------------------------------------------------------
// Result enums
// ---------------------------------------------------------------------------

/// Result of checking whether a unit can enter a target cell.
///
/// Maps to the original engine's Can_Enter_Cell return codes (0–7). Each variant
/// carries enough context for the movement tick to dispatch the correct
/// response without re-querying the EntityStore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellEntryResult {
    /// Code 0: Cell is passable. Enter freely.
    Clear,
    /// Code 1: Cell contains crushable occupants. Crush and enter.
    Crushable { victims: Vec<u64> },
    /// Code 2: Blocked by a moving friendly unit. Wait, then repath.
    TemporaryBlock { blocker_id: u64 },
    /// Code 3: Allied building/scatter-required soft block.
    ScatterRequired { blocker_id: Option<u64> },
    /// Code 4: Friendly wall/overlay soft block.
    FriendlyWall,
    /// Code 5: Enemy unit occupying. Attack blocker while waiting.
    OccupiedEnemy { blocker_id: u64 },
    /// Code 6: Friendly stationary non-building occupant.
    FriendlyStationary { blocker_id: u64 },
    /// Code 7: Terrain impassable (water, building footprint, etc.). Abort.
    Impassable,
}

impl CellEntryResult {
    pub fn yr_code(&self) -> u8 {
        match self {
            Self::Clear => 0,
            Self::Crushable { .. } => 1,
            Self::TemporaryBlock { .. } => 2,
            Self::ScatterRequired { .. } => 3,
            Self::FriendlyWall => 4,
            Self::OccupiedEnemy { .. } => 5,
            Self::FriendlyStationary { .. } => 6,
            Self::Impassable => 7,
        }
    }
}

/// Phase 1 result — terrain and basic occupancy check (no EntityStore needed).
///
/// Computed inside the mutable entity borrow where we cannot also access
/// EntityStore for blocker lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainCheckResult {
    /// Cell is passable (terrain OK, occupancy clear or sub-cell available).
    Clear,
    /// Terrain impassable for this unit type.
    Impassable,
    /// Cell has occupants — needs Phase 2 EntityStore lookup to classify.
    NeedsBlockerCheck,
}

/// Terrain-only result for native-shaped cell-entry checks above `PathGrid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainEntryResult {
    Clear,
    HardBlocked,
}

impl TerrainEntryResult {
    pub fn is_clear(self) -> bool {
        matches!(self, Self::Clear)
    }
}

/// Caller flavor for the terrain-entry slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainEntryMode {
    AStarNeighbor,
    RuntimeTransition,
    Smoothing,
    Scatter,
    SpawnLike,
}

/// Native-shaped terrain context for the water/pier-critical entry slice.
///
/// This is deliberately terrain/layer-only. Reduced-zone reachability and FNPC
/// candidate selection stay outside this context because gamemd uses different
/// callers and state for those questions.
#[derive(Debug, Clone, Copy)]
pub struct CellEntryTerrainContext<'a> {
    pub target: (u16, u16),
    pub movement_zone: Option<MovementZone>,
    pub speed_type: Option<SpeedType>,
    pub path_grid: Option<&'a PathGrid>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub terrain_costs: Option<&'a TerrainCostGrid>,
    pub bypass_grid: bool,
    pub mode: TerrainEntryMode,
}

/// Evaluate the terrain slice of cell entry for ground-layer movement.
///
/// `PathGrid` is a coarse structural filter. Final terrain legality must also
/// consult the mover's SpeedType against the resolved target LandType/speed row
/// so a PathGrid-walkable water cell is still illegal for ordinary ground movers.
pub fn evaluate_cell_entry_terrain(ctx: CellEntryTerrainContext<'_>) -> TerrainEntryResult {
    let (x, y) = ctx.target;

    let grid_ok = ctx
        .path_grid
        .map_or(true, |grid| ctx.bypass_grid || grid.is_walkable(x, y));
    if !grid_ok {
        return TerrainEntryResult::HardBlocked;
    }

    if let Some(speed_type) = ctx.speed_type.or_else(|| {
        if ctx.terrain_costs.is_some() {
            None
        } else {
            ctx.movement_zone.map(|zone| zone.speed_type())
        }
    }) {
        if let Some(terrain) = ctx.resolved_terrain {
            let Some(cell) = terrain.cell(x, y) else {
                return TerrainEntryResult::HardBlocked;
            };
            if !speed_type_allows_cell(cell, speed_type) {
                return TerrainEntryResult::HardBlocked;
            }
        }
    }

    terrain_cost_result(ctx.terrain_costs, x, y)
}

fn terrain_cost_result(
    terrain_costs: Option<&TerrainCostGrid>,
    x: u16,
    y: u16,
) -> TerrainEntryResult {
    if terrain_costs.is_some_and(|costs| costs.cost_at(x, y) == 0) {
        TerrainEntryResult::HardBlocked
    } else {
        TerrainEntryResult::Clear
    }
}

fn speed_type_allows_cell(cell: &ResolvedTerrainCell, speed_type: SpeedType) -> bool {
    if let Some(cost) = cell.speed_costs.cost_for_speed_type(speed_type) {
        return cost > 0;
    }
    passability::is_passable_for_speed_type(cell.land_type, speed_type)
}

/// Layer selections used by Can_Enter_Cell-style checks.
///
/// The common case uses one layer for all phases. Bridge traversal may select
/// the bridge object list while the post-traversal occupancy bits remain ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanEnterLayerContext {
    pub terrain_layer: MovementLayer,
    pub object_list_layer: MovementLayer,
    pub occupancy_bits_layer: MovementLayer,
}

impl CanEnterLayerContext {
    pub fn single(layer: MovementLayer) -> Self {
        Self {
            terrain_layer: layer,
            object_list_layer: layer,
            occupancy_bits_layer: layer,
        }
    }
}

/// Read-only cell-entry oracle row preserving gamemd's split layer decisions.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct CellEntryOracleRow {
    pub target: (u16, u16),
    pub terrain_layer: MovementLayer,
    pub object_list_layer: MovementLayer,
    pub occupancy_bits_layer: MovementLayer,
    pub terrain_result: String,
    pub yr_code: Option<u8>,
    pub occupancy_ground_present: bool,
    pub occupancy_bridge_present: bool,
}

impl CellEntryOracleRow {
    pub fn from_terrain_result(
        target: (u16, u16),
        layers: CanEnterLayerContext,
        result: TerrainCheckResult,
        occupancy: &OccupancyGrid,
    ) -> Self {
        let occ = occupancy.get(target.0, target.1);
        Self {
            target,
            terrain_layer: layers.terrain_layer,
            object_list_layer: layers.object_list_layer,
            occupancy_bits_layer: layers.occupancy_bits_layer,
            terrain_result: format!("{:?}", result),
            yr_code: match result {
                TerrainCheckResult::Clear => Some(CellEntryResult::Clear.yr_code()),
                TerrainCheckResult::Impassable => Some(CellEntryResult::Impassable.yr_code()),
                TerrainCheckResult::NeedsBlockerCheck => None,
            },
            occupancy_ground_present: occ.is_some_and(|o| !o.is_empty_on(MovementLayer::Ground)),
            occupancy_bridge_present: occ.is_some_and(|o| !o.is_empty_on(MovementLayer::Bridge)),
        }
    }
}

/// Opt-in diagnostic wrapper for Phase-1 cell entry checks.
pub fn check_terrain_with_layers_oracle(
    target: (u16, u16),
    layers: CanEnterLayerContext,
    mover_category: EntityCategory,
    path_grid: Option<&PathGrid>,
    cost_grid: Option<&TerrainCostGrid>,
    occupancy: &OccupancyGrid,
) -> (TerrainCheckResult, CellEntryOracleRow) {
    let result = check_terrain_with_layers(
        target,
        layers,
        mover_category,
        path_grid,
        cost_grid,
        occupancy,
    );
    let row = CellEntryOracleRow::from_terrain_result(target, layers, result, occupancy);
    (result, row)
}

/// Vehicle-only building entry branch that may reach the live row helper.
///
/// InfantryClass::Can_Enter_Cell does not use the radio/contact or
/// UnitRepair/Bunker NumberImpassableRows branches, so callers must not use this
/// as a shared infantry rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleBuildingEntryBranch {
    /// Contact-vector branch. The caller must supply whether this mover has
    /// RadioClass contact with the checked building.
    RadioContact { mover_has_contact: bool },
    /// UnitRepair/Bunker branch. This branch is gated by the checked building's
    /// type flags, not by RadioClass contact.
    UnitRepairOrBunker,
}

/// Decision for a checked building occupant in UnitClass-style cell entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingOccupantEntryDecision {
    /// Keep the checked building in the ordinary blocker classification path.
    KeepBlocker,
    /// Skip this building occupant and continue scanning later occupants in the
    /// cell's object list.
    SkipBlocker,
}

/// Explicit live facts needed by the UnitClass building row-helper decision.
///
/// Caller responsibilities:
/// - `candidate_building_id` must be the result of a live
///   Look_up_building_in_cell-style lookup for the candidate cell.
/// - `checked_building_id` and type/runtime flags must describe the building
///   occupant currently being inspected.
/// - `mover_category` must be the mover's semantic category; only UnitClass-style
///   vehicle movers use these exceptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveVehicleBuildingEntry {
    pub mover_category: EntityCategory,
    pub branch: VehicleBuildingEntryBranch,
    pub checked_building_id: u64,
    pub candidate_building_id: Option<u64>,
    pub candidate_x: u16,
    pub building_origin_x: u16,
    pub number_impassable_rows: i32,
    pub is_unit_repair: bool,
    pub is_bunker: bool,
    pub bunker_occupied: bool,
}

/// Decide whether UnitClass-style movement should skip a building occupant.
///
/// This models `FUN_00458A00` at its two UnitClass::Can_Enter_Cell callsites:
/// radio/contact and UnitRepair/Bunker. A `KeepBlocker` result means the caller
/// should continue with the existing Can_Enter_Cell return-code classification;
/// `SkipBlocker` means only this building occupant is ignored.
pub fn decide_live_vehicle_building_entry(
    input: LiveVehicleBuildingEntry,
) -> BuildingOccupantEntryDecision {
    if input.mover_category != EntityCategory::Unit {
        return BuildingOccupantEntryDecision::KeepBlocker;
    }

    let branch_active = match input.branch {
        VehicleBuildingEntryBranch::RadioContact { mover_has_contact } => mover_has_contact,
        VehicleBuildingEntryBranch::UnitRepairOrBunker => input.is_unit_repair || input.is_bunker,
    };
    if !branch_active {
        return BuildingOccupantEntryDecision::KeepBlocker;
    }

    if input.candidate_building_id != Some(input.checked_building_id) {
        return BuildingOccupantEntryDecision::KeepBlocker;
    }
    if input.number_impassable_rows == -1 {
        return BuildingOccupantEntryDecision::KeepBlocker;
    }
    if input.is_bunker && input.bunker_occupied {
        return BuildingOccupantEntryDecision::KeepBlocker;
    }

    let first_clear_x = i32::from(input.building_origin_x) + input.number_impassable_rows;
    if i32::from(input.candidate_x) >= first_clear_x {
        BuildingOccupantEntryDecision::SkipBlocker
    } else {
        BuildingOccupantEntryDecision::KeepBlocker
    }
}

// ---------------------------------------------------------------------------
// Phase 1: terrain + occupancy presence
// ---------------------------------------------------------------------------

/// Check terrain walkability and basic occupancy for a target cell.
///
/// This is Phase 1 of the two-phase cell entry check. It does NOT access
/// EntityStore, so it can run inside a mutable entity borrow.
///
/// For infantry movers, also checks sub-cell availability.
pub fn check_terrain(
    target: (u16, u16),
    target_layer: MovementLayer,
    mover_category: EntityCategory,
    path_grid: Option<&PathGrid>,
    cost_grid: Option<&TerrainCostGrid>,
    occupancy: &OccupancyGrid,
) -> TerrainCheckResult {
    check_terrain_with_layers(
        target,
        CanEnterLayerContext::single(target_layer),
        mover_category,
        path_grid,
        cost_grid,
        occupancy,
    )
}

/// Check terrain and occupancy using explicit CanEnter layer selections.
pub fn check_terrain_with_layers(
    target: (u16, u16),
    layers: CanEnterLayerContext,
    mover_category: EntityCategory,
    path_grid: Option<&PathGrid>,
    cost_grid: Option<&TerrainCostGrid>,
    occupancy: &OccupancyGrid,
) -> TerrainCheckResult {
    let (nx, ny) = target;

    // --- Terrain walkability ---
    let terrain_walkable = match layers.terrain_layer {
        MovementLayer::Ground => {
            let grid_ok = path_grid.map_or(true, |g| g.is_walkable(nx, ny));
            let cost_ok = cost_grid.map_or(true, |cg| cg.cost_at(nx, ny) > 0);
            grid_ok && cost_ok
        }
        MovementLayer::Bridge => {
            path_grid.is_some_and(|grid| grid.is_walkable_on_layer(nx, ny, MovementLayer::Bridge))
        }
        MovementLayer::Air | MovementLayer::Underground => true,
    };
    if !terrain_walkable {
        return TerrainCheckResult::Impassable;
    }

    // --- Occupancy ---
    let occ = occupancy.get(nx, ny);

    if mover_category == EntityCategory::Infantry {
        let selected_list_blocked =
            occ.is_some_and(|o| o.has_blockers_on(layers.object_list_layer));
        let sub =
            bump_crush::allocate_sub_cell_with_reserved(occ, layers.occupancy_bits_layer, None);
        if sub.is_some() && !selected_list_blocked {
            return TerrainCheckResult::Clear;
        }
        // No sub-cell available — needs blocker classification.
        return TerrainCheckResult::NeedsBlockerCheck;
    }

    // Vehicle/aircraft/structure: cell must be unoccupied on this layer.
    match occ {
        None => TerrainCheckResult::Clear,
        Some(o)
            if o.is_empty_on(layers.object_list_layer)
                && o.is_empty_on(layers.occupancy_bits_layer) =>
        {
            TerrainCheckResult::Clear
        }
        Some(_) => TerrainCheckResult::NeedsBlockerCheck,
    }
}

// ---------------------------------------------------------------------------
// Phase 2: blocker classification (needs EntityStore)
// ---------------------------------------------------------------------------

/// Classify an occupied cell's blockers to determine the Can_Enter_Cell code.
///
/// This is Phase 2 — runs outside the mutable entity borrow so it can read
/// blocker properties from EntityStore.
///
/// Check order (current approximation of original engine priority):
/// 1. Crush: if all occupants are crushable → Crushable
/// 2. Blocker friendship: enemy → OccupiedEnemy, friendly → moving/stationary
/// 3. JumpJet override: codes < 7 treated as Clear
///
/// TODO(RE): The recovered candidate predicate also folds in bridge legality and
/// additional terrain/object state before these occupancy outcomes are chosen.
pub fn classify_occupied_cell(
    target: (u16, u16),
    target_layer: MovementLayer,
    mover_id: u64,
    crush_capability: bump_crush::CrushCapability,
    mover_owner: &str,
    mover_locomotor: LocomotorKind,
    mover_bypass_grid: bool,
    occupancy: &OccupancyGrid,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    classify_occupied_cell_with_layers(
        target,
        CanEnterLayerContext::single(target_layer),
        mover_id,
        crush_capability,
        mover_owner,
        mover_locomotor,
        mover_bypass_grid,
        occupancy,
        entities,
        alliances,
        interner,
    )
}

/// Classify an occupied cell using explicit CanEnter layer selections.
#[allow(clippy::too_many_arguments)]
pub fn classify_occupied_cell_with_layers(
    target: (u16, u16),
    layers: CanEnterLayerContext,
    mover_id: u64,
    crush_capability: bump_crush::CrushCapability,
    mover_owner: &str,
    mover_locomotor: LocomotorKind,
    mover_bypass_grid: bool,
    occupancy: &OccupancyGrid,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    classify_occupied_cell_with_layers_and_ignored(
        target,
        layers,
        mover_id,
        crush_capability,
        mover_owner,
        mover_locomotor,
        mover_bypass_grid,
        None,
        occupancy,
        entities,
        alliances,
        interner,
    )
}

/// Classify an occupied cell while ignoring a caller-supplied subset of live
/// object-list occupants. This is the runtime UnitClass path used by refinery
/// pads and repair/bunker rows where gamemd skips only the checked building
/// occupant, then continues scanning the same cell list.
#[allow(clippy::too_many_arguments)]
pub fn classify_occupied_cell_with_layers_and_ignored(
    target: (u16, u16),
    layers: CanEnterLayerContext,
    mover_id: u64,
    crush_capability: bump_crush::CrushCapability,
    mover_owner: &str,
    mover_locomotor: LocomotorKind,
    mover_bypass_grid: bool,
    ignored_blockers: Option<&BTreeSet<u64>>,
    occupancy: &OccupancyGrid,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    // --- Crush check ---
    let victims = bump_crush::collect_crush_victims(
        target,
        occupancy,
        layers.object_list_layer,
        crush_capability,
        entities,
    );
    if !victims.is_empty()
        && bump_crush::cell_passable_after_crush(
            target,
            occupancy,
            layers.occupancy_bits_layer,
            crush_capability,
            entities,
        )
    {
        return apply_overrides(CellEntryResult::Crushable { victims }, mover_locomotor);
    }

    // --- Find primary blocker ---
    let blocker_id = find_primary_blocker(
        target,
        layers.object_list_layer,
        mover_id,
        mover_bypass_grid,
        ignored_blockers,
        occupancy,
        entities,
    );
    let Some(bid) = blocker_id else {
        // No identifiable blocker. With bypass_grid, this means the cell
        // contained only structures that we're permitted to drive through —
        // treat as Clear. Without bypass_grid, this is unexpected (Phase 1
        // would have returned Clear if the cell were truly empty).
        if ignored_blockers.is_some() {
            return apply_overrides(CellEntryResult::Clear, mover_locomotor);
        }
        return apply_overrides(CellEntryResult::Impassable, mover_locomotor);
    };

    // --- Classify blocker ---
    let result = classify_blocker(bid, mover_owner, entities, alliances, interner);
    apply_overrides(result, mover_locomotor)
}

/// Find the primary blocker entity in a cell using the current local
/// approximation's first-match rule over the selected occupancy layer.
///
/// Live building exceptions are supplied through `ignored_blockers`; bypassing
/// the static path grid does not suppress structure occupants by itself.
fn find_primary_blocker(
    target: (u16, u16),
    layer: MovementLayer,
    mover_id: u64,
    _mover_bypass_grid: bool,
    ignored_blockers: Option<&BTreeSet<u64>>,
    occupancy: &OccupancyGrid,
    _entities: &EntityStore,
) -> Option<u64> {
    let occ = occupancy.get(target.0, target.1)?;
    for occupant in occ.iter_layer(layer) {
        if occupant.entity_id == mover_id {
            continue;
        }
        if ignored_blockers.is_some_and(|ids| ids.contains(&occupant.entity_id)) {
            continue;
        }
        return Some(occupant.entity_id);
    }
    None
}

/// Classify a single blocker as enemy, friendly-moving, or friendly-stationary.
fn classify_blocker(
    blocker_id: u64,
    mover_owner: &str,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    let Some(blocker) = entities.get(blocker_id) else {
        return CellEntryResult::Impassable;
    };
    let is_friendly =
        houses::are_houses_friendly(alliances, mover_owner, interner.resolve(blocker.owner));
    if !is_friendly {
        return CellEntryResult::OccupiedEnemy { blocker_id };
    }
    if blocker
        .building_gate
        .is_some_and(|gate| !gate.can_garrison_passable())
    {
        return CellEntryResult::ScatterRequired {
            blocker_id: Some(blocker_id),
        };
    }
    // Friendly: moving -> temporary block, stationary -> code 6.
    if blocker.movement_target.is_some() {
        CellEntryResult::TemporaryBlock { blocker_id }
    } else {
        CellEntryResult::FriendlyStationary { blocker_id }
    }
}

/// Apply locomotor-specific overrides to a cell entry result.
///
/// JumpJet: all codes except Impassable treated as Clear (deep_113 line 861).
fn apply_overrides(result: CellEntryResult, locomotor: LocomotorKind) -> CellEntryResult {
    if locomotor == LocomotorKind::Jumpjet && !matches!(result, CellEntryResult::Impassable) {
        return CellEntryResult::Clear;
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::occupancy::CellListInsertion;

    fn empty_occ() -> OccupancyGrid {
        OccupancyGrid::new()
    }

    #[test]
    fn test_clear_empty_cell() {
        let result = check_terrain(
            (5, 5),
            MovementLayer::Ground,
            EntityCategory::Unit,
            None,
            None,
            &empty_occ(),
        );
        assert_eq!(result, TerrainCheckResult::Clear);
    }

    #[test]
    fn test_impassable_blocked_grid() {
        use crate::sim::pathfinding::PathGrid;
        let mut grid = PathGrid::new(10, 10);
        grid.set_blocked(5, 5, true);
        let result = check_terrain(
            (5, 5),
            MovementLayer::Ground,
            EntityCategory::Unit,
            Some(&grid),
            None,
            &empty_occ(),
        );
        assert_eq!(result, TerrainCheckResult::Impassable);
    }

    #[test]
    fn test_vehicle_occupied_needs_check() {
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            42,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let result = check_terrain(
            (5, 5),
            MovementLayer::Ground,
            EntityCategory::Unit,
            None,
            None,
            &occ,
        );
        assert_eq!(result, TerrainCheckResult::NeedsBlockerCheck);
    }

    #[test]
    fn test_infantry_subcell_available() {
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        let result = check_terrain(
            (5, 5),
            MovementLayer::Ground,
            EntityCategory::Infantry,
            None,
            None,
            &occ,
        );
        assert_eq!(result, TerrainCheckResult::Clear);
    }

    #[test]
    fn test_infantry_cell_full() {
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        occ.add(
            5,
            5,
            11,
            MovementLayer::Ground,
            Some(3),
            CellListInsertion::PrependNonBuilding,
        );
        occ.add(
            5,
            5,
            12,
            MovementLayer::Ground,
            Some(4),
            CellListInsertion::PrependNonBuilding,
        );
        let result = check_terrain(
            (5, 5),
            MovementLayer::Ground,
            EntityCategory::Infantry,
            None,
            None,
            &occ,
        );
        assert_eq!(result, TerrainCheckResult::NeedsBlockerCheck);
    }

    #[test]
    fn test_jumpjet_override_clears_non_impassable() {
        let result = apply_overrides(
            CellEntryResult::OccupiedEnemy { blocker_id: 1 },
            LocomotorKind::Jumpjet,
        );
        assert_eq!(result, CellEntryResult::Clear);
    }

    #[test]
    fn test_jumpjet_keeps_impassable() {
        let result = apply_overrides(CellEntryResult::Impassable, LocomotorKind::Jumpjet);
        assert_eq!(result, CellEntryResult::Impassable);
    }

    #[test]
    fn test_non_jumpjet_no_override() {
        let result = apply_overrides(
            CellEntryResult::OccupiedEnemy { blocker_id: 1 },
            LocomotorKind::Drive,
        );
        assert_eq!(result, CellEntryResult::OccupiedEnemy { blocker_id: 1 });
    }

    #[test]
    fn cell_entry_result_yr_codes_match_verified_table() {
        assert_eq!(CellEntryResult::Clear.yr_code(), 0);
        assert_eq!(CellEntryResult::Crushable { victims: vec![1] }.yr_code(), 1);
        assert_eq!(
            CellEntryResult::TemporaryBlock { blocker_id: 1 }.yr_code(),
            2
        );
        assert_eq!(
            CellEntryResult::ScatterRequired {
                blocker_id: Some(1),
            }
            .yr_code(),
            3
        );
        assert_eq!(CellEntryResult::FriendlyWall.yr_code(), 4);
        assert_eq!(
            CellEntryResult::OccupiedEnemy { blocker_id: 1 }.yr_code(),
            5
        );
        assert_eq!(
            CellEntryResult::FriendlyStationary { blocker_id: 1 }.yr_code(),
            6
        );
        assert_eq!(CellEntryResult::Impassable.yr_code(), 7);
    }

    #[test]
    fn friendly_closed_or_opening_gate_returns_code_3_not_code_6() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::{BuildingGatePhase, BuildingGateRuntime, GameEntity};

        let mut entities = EntityStore::new();
        let mut gate = GameEntity::test_default(100, "GAGATE_A", "Americans", 5, 5);
        gate.category = EntityCategory::Structure;
        gate.building_gate = Some(BuildingGateRuntime::default());
        entities.insert(gate);
        let alliances = HouseAllianceMap::new();
        let interner = crate::sim::intern::test_interner();

        let result = classify_blocker(100, "Americans", &entities, &alliances, &interner);
        assert_eq!(
            result,
            CellEntryResult::ScatterRequired {
                blocker_id: Some(100)
            }
        );
        assert_eq!(result.yr_code(), 3);

        entities.get_mut(100).unwrap().building_gate = Some(BuildingGateRuntime {
            mission_18_active: true,
            phase: BuildingGatePhase::Opening,
            ..Default::default()
        });
        let result = classify_blocker(100, "Americans", &entities, &alliances, &interner);
        assert_eq!(
            result,
            CellEntryResult::ScatterRequired {
                blocker_id: Some(100)
            }
        );
        assert_eq!(result.yr_code(), 3);
    }

    #[test]
    fn enemy_closed_gate_keeps_enemy_result_code() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::{BuildingGateRuntime, GameEntity};

        let mut entities = EntityStore::new();
        let mut gate = GameEntity::test_default(100, "GAGATE_A", "Soviets", 5, 5);
        gate.category = EntityCategory::Structure;
        gate.building_gate = Some(BuildingGateRuntime::default());
        entities.insert(gate);
        let alliances = HouseAllianceMap::new();
        let interner = crate::sim::intern::test_interner();

        let result = classify_blocker(100, "Americans", &entities, &alliances, &interner);
        assert_eq!(result, CellEntryResult::OccupiedEnemy { blocker_id: 100 });
        assert_eq!(result.yr_code(), 5);
    }

    fn row_entry_input(
        mover_category: EntityCategory,
        branch: VehicleBuildingEntryBranch,
        candidate_x: u16,
    ) -> LiveVehicleBuildingEntry {
        LiveVehicleBuildingEntry {
            mover_category,
            branch,
            checked_building_id: 100,
            candidate_building_id: Some(100),
            candidate_x,
            building_origin_x: 10,
            number_impassable_rows: 1,
            is_unit_repair: false,
            is_bunker: false,
            bunker_occupied: false,
        }
    }

    #[test]
    fn infantry_does_not_use_vehicle_row_contact_skip() {
        let input = row_entry_input(
            EntityCategory::Infantry,
            VehicleBuildingEntryBranch::RadioContact {
                mover_has_contact: true,
            },
            11,
        );

        assert_eq!(
            decide_live_vehicle_building_entry(input),
            BuildingOccupantEntryDecision::KeepBlocker
        );
    }

    #[test]
    fn contacted_vehicle_row_skip_opens_east_columns_but_keeps_west() {
        let contacted = VehicleBuildingEntryBranch::RadioContact {
            mover_has_contact: true,
        };
        assert_eq!(
            decide_live_vehicle_building_entry(row_entry_input(
                EntityCategory::Unit,
                contacted,
                10,
            )),
            BuildingOccupantEntryDecision::KeepBlocker
        );
        assert_eq!(
            decide_live_vehicle_building_entry(row_entry_input(
                EntityCategory::Unit,
                contacted,
                11,
            )),
            BuildingOccupantEntryDecision::SkipBlocker
        );
        assert_eq!(
            decide_live_vehicle_building_entry(row_entry_input(
                EntityCategory::Unit,
                VehicleBuildingEntryBranch::RadioContact {
                    mover_has_contact: false,
                },
                11,
            )),
            BuildingOccupantEntryDecision::KeepBlocker
        );
    }

    #[test]
    fn empty_vs_occupied_bunker_uses_explicit_runtime_occupant_arg() {
        let mut empty = row_entry_input(
            EntityCategory::Unit,
            VehicleBuildingEntryBranch::UnitRepairOrBunker,
            10,
        );
        empty.number_impassable_rows = 0;
        empty.is_bunker = true;

        assert_eq!(
            decide_live_vehicle_building_entry(empty),
            BuildingOccupantEntryDecision::SkipBlocker
        );

        let occupied = LiveVehicleBuildingEntry {
            bunker_occupied: true,
            ..empty
        };
        assert_eq!(
            decide_live_vehicle_building_entry(occupied),
            BuildingOccupantEntryDecision::KeepBlocker
        );
    }

    #[test]
    fn row_helper_requires_same_candidate_building_and_rows_value() {
        let mut other_building = row_entry_input(
            EntityCategory::Unit,
            VehicleBuildingEntryBranch::UnitRepairOrBunker,
            11,
        );
        other_building.is_unit_repair = true;
        other_building.candidate_building_id = Some(200);
        assert_eq!(
            decide_live_vehicle_building_entry(other_building),
            BuildingOccupantEntryDecision::KeepBlocker
        );

        let no_rows = LiveVehicleBuildingEntry {
            candidate_building_id: Some(100),
            number_impassable_rows: -1,
            ..other_building
        };
        assert_eq!(
            decide_live_vehicle_building_entry(no_rows),
            BuildingOccupantEntryDecision::KeepBlocker
        );
    }

    #[test]
    fn find_primary_blocker_does_not_use_bypass_grid_as_structure_skip() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        // Cell occupancy: a Structure (refinery) at (5, 5).
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            100,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );

        // EntityStore with the structure entity.
        let mut entities = EntityStore::new();
        let mut refinery = GameEntity::test_default(100, "GAREFN", "Allies", 5, 5);
        refinery.category = EntityCategory::Structure;
        entities.insert(refinery);

        // With bypass_grid=true: structure is filtered, no other occupants → None.
        let result = find_primary_blocker(
            (5, 5),
            MovementLayer::Ground,
            42,   // mover_id
            true, // mover_bypass_grid
            None,
            &occ,
            &entities,
        );
        assert_eq!(
            result,
            Some(100),
            "bypass_grid must not erase live structure blockers"
        );

        // With bypass_grid=false: structure is the primary blocker → Some(100).
        let result = find_primary_blocker(
            (5, 5),
            MovementLayer::Ground,
            42,
            false, // mover_bypass_grid
            None,
            &occ,
            &entities,
        );
        assert_eq!(
            result,
            Some(100),
            "with bypass_grid=false, Structure must still be picked as blocker (regression)"
        );
    }

    #[test]
    fn find_primary_blocker_follows_layer_order() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        occ.add(
            5,
            5,
            20,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );

        let mut entities = EntityStore::new();
        let mut blocker = GameEntity::test_default(10, "HTNK", "Allies", 5, 5);
        blocker.category = EntityCategory::Unit;
        entities.insert(blocker);
        let mut infantry = GameEntity::test_default(20, "E1", "Allies", 5, 5);
        infantry.category = EntityCategory::Infantry;
        entities.insert(infantry);

        let result = find_primary_blocker(
            (5, 5),
            MovementLayer::Ground,
            42,
            false,
            None,
            &occ,
            &entities,
        );
        assert_eq!(result, Some(20));
    }

    #[test]
    fn find_primary_blocker_skips_caller_ignored_ids() {
        use crate::sim::entity_store::EntityStore;

        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
        occ.add(
            5,
            5,
            20,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let ignored = std::collections::BTreeSet::from([10]);
        let entities = EntityStore::new();
        let result = find_primary_blocker(
            (5, 5),
            MovementLayer::Ground,
            42,
            false,
            Some(&ignored),
            &occ,
            &entities,
        );

        assert_eq!(result, Some(20));
    }

    #[test]
    fn split_context_uses_occupancy_bits_layer_for_presence() {
        use crate::sim::pathfinding::PathGrid;

        let mut grid = PathGrid::new(10, 10);
        grid.set_cell_for_test(5, 5, 0, true, true);
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let result = check_terrain_with_layers(
            (5, 5),
            CanEnterLayerContext {
                terrain_layer: MovementLayer::Bridge,
                object_list_layer: MovementLayer::Bridge,
                occupancy_bits_layer: MovementLayer::Ground,
            },
            EntityCategory::Unit,
            Some(&grid),
            None,
            &occ,
        );

        assert_eq!(result, TerrainCheckResult::NeedsBlockerCheck);
    }

    #[test]
    fn oracle_wrapper_preserves_split_layers_and_yr_code() {
        use crate::sim::pathfinding::PathGrid;

        let mut grid = PathGrid::new(10, 10);
        grid.set_cell_for_test(5, 5, 0, true, true);
        let layers = CanEnterLayerContext {
            terrain_layer: MovementLayer::Bridge,
            object_list_layer: MovementLayer::Bridge,
            occupancy_bits_layer: MovementLayer::Ground,
        };
        let (result, row) = check_terrain_with_layers_oracle(
            (5, 5),
            layers,
            EntityCategory::Unit,
            Some(&grid),
            None,
            &empty_occ(),
        );

        assert_eq!(result, TerrainCheckResult::Clear);
        assert_eq!(row.terrain_layer, MovementLayer::Bridge);
        assert_eq!(row.object_list_layer, MovementLayer::Bridge);
        assert_eq!(row.occupancy_bits_layer, MovementLayer::Ground);
        assert_eq!(row.yr_code, Some(0));
    }

    #[test]
    fn split_context_uses_object_list_layer_for_selected_blockers() {
        use crate::sim::pathfinding::PathGrid;

        let mut grid = PathGrid::new(10, 10);
        grid.set_cell_for_test(5, 5, 0, true, true);
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let result = check_terrain_with_layers(
            (5, 5),
            CanEnterLayerContext {
                terrain_layer: MovementLayer::Bridge,
                object_list_layer: MovementLayer::Bridge,
                occupancy_bits_layer: MovementLayer::Ground,
            },
            EntityCategory::Unit,
            Some(&grid),
            None,
            &occ,
        );

        assert_eq!(result, TerrainCheckResult::NeedsBlockerCheck);
    }

    #[test]
    fn split_context_scans_object_list_layer_for_primary_blocker() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        occ.add(
            5,
            5,
            20,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let mut entities = EntityStore::new();
        let mut ground = GameEntity::test_default(10, "HTNK", "Allies", 5, 5);
        ground.category = EntityCategory::Unit;
        entities.insert(ground);
        let mut bridge = GameEntity::test_default(20, "HTNK", "Soviets", 5, 5);
        bridge.category = EntityCategory::Unit;
        entities.insert(bridge);

        let alliances = HouseAllianceMap::new();
        let interner = crate::sim::intern::test_interner();
        let result = classify_occupied_cell_with_layers(
            (5, 5),
            CanEnterLayerContext {
                terrain_layer: MovementLayer::Bridge,
                object_list_layer: MovementLayer::Bridge,
                occupancy_bits_layer: MovementLayer::Ground,
            },
            42,
            bump_crush::CrushCapability::new(false, false),
            "Allies",
            LocomotorKind::Drive,
            false,
            &occ,
            &entities,
            &alliances,
            &interner,
        );

        assert_eq!(result, CellEntryResult::OccupiedEnemy { blocker_id: 20 });
    }
}
