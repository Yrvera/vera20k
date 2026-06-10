//! Cell occupancy, infantry sub-cell, crush, and scatter logic for ground movement.
//!
//! Extracted from movement.rs to keep that file under 600 lines. Contains:
//! - `CellOccupancy` — tracks what entities occupy each cell (vehicles vs infantry sub-cells)
//! - `OccupancyGrid` — persistent per-cell occupancy (see sim/occupancy.rs)
//! - Sub-cell allocation for infantry (spots 2, 3, 4 — max 3 per cell)
//! - Crush checks: Crusher/CrusherAll movement zones vs crushable/omni_crush_resistant
//! - Scatter: issue movement commands to displace friendly blockers (replaces old teleport "bump")
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/entity_store, sim/game_entity, sim/locomotor,
//!   sim/pathfinding, sim/rng, rules/locomotor_type.

use std::collections::BTreeSet;

use crate::sim::pathfinding::{BlockerNeighborCounts, EntityBlockEntry, LayeredEntityBlockMap};

use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::{CellOccupancy, OccupancyGrid};
use crate::sim::pathfinding::PathGrid;
use crate::sim::rng::SimRng;
use crate::util::fixed_math::{SimFixed, fixed_distance};

/// Functional infantry sub-cell positions. The original engine uses sub-cells
/// 2 (NE), 3 (SW), 4 (SE) — three corners of the isometric diamond. Sub-cells
/// 0 (center) and 1 (NW) are never assigned to infantry by the placement function
/// (FUN_00481180 explicitly skips them: `if (uVar11 != 0 && uVar11 != 1)`).
pub const FUNCTIONAL_SUB_CELLS: [u8; 3] = [2, 3, 4];

/// Maximum infantry that can share one cell (one per functional sub-cell spot).
pub const MAX_INFANTRY_PER_CELL: usize = 3;

/// Preference order tables for infantry sub-cell placement.
/// Indexed by quadrant result (0-4). Each entry lists 4 sub-cell indices to try.
/// The placement loop skips indices 0 and 1, so effective choices are from {2, 3, 4}.
const SUBCELL_PREFERENCE: [[u8; 4]; 5] = [
    [1, 2, 3, 4], // quadrant 0 (center/NW) — not used directly, random table instead
    [0, 2, 3, 4], // quadrant 1 (dead — GetSubCell never returns 1)
    [0, 1, 4, 3], // quadrant 2 (NE) — effective: 4, then 3
    [0, 1, 4, 2], // quadrant 3 (SW) — effective: 4, then 2
    [0, 2, 3, 1], // quadrant 4 (SE) — effective: 2, then 3
];

/// Random rotation tables for sub-cell placement.
/// When quadrant is 0 (center/NW), one of these 4 rotations is picked randomly.
const SUBCELL_RANDOM_ROTATIONS: [[u8; 4]; 4] =
    [[1, 2, 3, 4], [2, 3, 4, 1], [3, 4, 1, 2], [4, 1, 2, 3]];

/// Determine which sub-cell quadrant a lepton position falls in.
///
/// Returns: 0 (center/NW), 2 (NE), 3 (SW), 4 (SE). Never returns 1.
fn get_subcell_quadrant(sub_x: SimFixed, sub_y: SimFixed) -> u8 {
    let center: SimFixed = SimFixed::from_num(128);
    let cx: SimFixed = sub_x - center;
    let cy: SimFixed = sub_y - center;
    let dist: SimFixed = fixed_distance(cx, cy);
    if dist < SimFixed::from_num(60) {
        return 0;
    }
    let mut bits: u8 = if sub_x > center { 1 } else { 0 };
    if sub_y > center {
        bits |= 2;
    }
    if bits == 0 {
        return 0; // NW quadrant → merged with center
    }
    bits + 1
}

/// The 8 directional offsets in isometric cell coordinates (dx, dy).
const NEIGHBOR_OFFSETS: [(i32, i32); 8] = [
    (0, -1),  // N
    (1, -1),  // NE
    (1, 0),   // E
    (1, 1),   // SE
    (0, 1),   // S
    (-1, 1),  // SW
    (-1, 0),  // W
    (-1, -1), // NW
];

/// Build the set of cells blocked by entities for pathfinding purposes.
///
/// RA2 key optimization: **moving friendly units are treated as passable terrain**
/// during path calculation. Only stationary units/buildings and enemy units block.
/// This prevents convoy deadlocks and constant repath thrashing in group movement.
///
/// `mover_owner` is the owner of the unit requesting the path.
/// `alliances` is the house alliance graph for friendship checks.
/// Build layer-separated sets of cells blocked by entities for pathfinding.
///
/// Returns `(ground_blocks, bridge_blocks)`. Units on the bridge layer only
/// block bridge pathfinding, and ground units only block ground pathfinding.
/// This enables units to coexist above and below a bridge simultaneously,
/// matching the original engine's `FirstObject`/`AltObject` dual-layer system.
///
/// RA2 cooperative pathfinding: friendly-moving units are recorded in an
/// `entity_block_map` keyed by selected object-list layer and the blocker's
/// current cell, with value equal to the blocker's next cell
/// (movement_target.path[next_index]). The A* cost function walks this map to
/// compute the code-2 dynamic cost per gamemd.exe AStar_compute_edge_cost
/// (0x00429830). Stationary units/buildings and enemies hard-block via the
/// BTreeSet outputs.
///
/// When `rules` is provided, structure footprints are expanded across all
/// occupied cells (foundation + AddOccupy − RemoveOccupy). Without `rules`
/// only the anchor cell is marked, which can let A* route through buildings.
///
/// Returns `(ground_blocks, bridge_blocks, entity_block_map)`.
pub fn build_entity_block_sets(
    entities: &EntityStore,
    mover_owner: &str,
    alliances: &crate::map::houses::HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> (
    BTreeSet<(u16, u16)>,
    BTreeSet<(u16, u16)>,
    LayeredEntityBlockMap,
) {
    let mut ground_blocked: BTreeSet<(u16, u16)> = BTreeSet::new();
    let bridge_blocked: BTreeSet<(u16, u16)> = BTreeSet::new();
    let mut entity_block_map = LayeredEntityBlockMap::new();
    for entity in entities.values() {
        // A Dying corpse is off the occupancy grid (uninit unmarked it); exclude
        // it here too so movers don't path around a building that no longer
        // exists.
        if entity.dying {
            continue;
        }
        // Entities inside transports don't occupy cells.
        if entity.passenger_role.is_inside_transport() {
            continue;
        }
        let Some(layer) = entity.occupancy_list_layer() else {
            continue;
        };
        let pos = (entity.position.rx, entity.position.ry);
        // Buildings always block (they never move). Always ground layer.
        // With rules, expand to the full foundation so A* sees every occupied
        // cell — without it, only the anchor blocks (legacy behavior).
        if entity.category == EntityCategory::Structure {
            if let Some(obj) = rules.and_then(|r| r.object(interner.resolve(entity.type_ref))) {
                let foundation_cells = crate::sim::production::building_base_foundation_cells(
                    pos.0,
                    pos.1,
                    &obj.foundation,
                );
                let is_bunker_occupied = obj.bunker
                    && (entity.bunker_occupant.is_some()
                        || entity
                            .passenger_role
                            .cargo()
                            .is_some_and(|cargo| cargo.count() > 0));
                let cells = crate::sim::production::building_movement_blocking_cells_for_state(
                    &foundation_cells,
                    pos.0,
                    obj.bib,
                    obj.number_impassable_rows,
                    obj.bunker,
                    is_bunker_occupied,
                    false,
                );
                for cell in cells {
                    ground_blocked.insert(cell);
                }
            } else {
                ground_blocked.insert(pos);
            }
            continue;
        }
        // Enemy units: soft-block with code 5 (cost 20x).
        let entity_owner_str = interner.resolve(entity.owner);
        let is_friendly =
            crate::map::houses::are_houses_friendly(alliances, mover_owner, entity_owner_str);
        if !is_friendly {
            entity_block_map.insert(
                layer,
                pos,
                EntityBlockEntry {
                    next_cell: None,
                    cost_code: 5,
                },
            );
            continue;
        }
        // Friendly moving units: code-2 chain walk entry.
        if let Some(ref mt) = entity.movement_target {
            if let Some(&next_cell) = mt.path.get(mt.next_index) {
                if next_cell != pos {
                    entity_block_map.insert(
                        layer,
                        pos,
                        EntityBlockEntry {
                            next_cell: Some(next_cell),
                            cost_code: 2,
                        },
                    );
                    continue;
                }
            }
        }
        // Stationary friendly: soft-block with code 6 (cost 8x).
        entity_block_map.insert(
            layer,
            pos,
            EntityBlockEntry {
                next_cell: None,
                cost_code: 6,
            },
        );
    }
    (ground_blocked, bridge_blocked, entity_block_map)
}

/// Build a combined block set (both layers merged) for the flat A* pathfinder
/// which doesn't distinguish layers. Returns `(blocks, entity_block_map)`.
pub fn build_entity_block_set(
    entities: &EntityStore,
    mover_owner: &str,
    alliances: &crate::map::houses::HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> (BTreeSet<(u16, u16)>, LayeredEntityBlockMap) {
    let (ground, bridge, entity_block_map) =
        build_entity_block_sets(entities, mover_owner, alliances, interner, rules);
    (ground.union(&bridge).copied().collect(), entity_block_map)
}

pub(crate) fn build_blocker_neighbor_counts(
    entities: &EntityStore,
    width: u16,
    height: u16,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    interner: &crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> BlockerNeighborCounts {
    let mut counts = BlockerNeighborCounts::new(width, height);

    if let Some(terrain) = resolved_terrain {
        for y in 0..height {
            for x in 0..width {
                let Some(cell) = terrain.cell(x, y) else {
                    continue;
                };
                if cell.overlay_blocks {
                    counts.add_single_cell_neighbor_source(x, y);
                }
                if cell.terrain_object_blocks {
                    counts.add_single_cell_neighbor_source(x, y);
                }
            }
        }
    }

    for entity in entities.values() {
        // Dying corpses are off the occupancy grid — don't let them inflate the
        // A* dynamic-blocker neighbor costs.
        if entity.dying {
            continue;
        }
        if entity.passenger_role.is_inside_transport() || entity.occupancy_list_layer().is_none() {
            continue;
        }
        let pos = (entity.position.rx, entity.position.ry);
        if entity.category == EntityCategory::Structure {
            let (width, height) = rules
                .and_then(|r| r.object(interner.resolve(entity.type_ref)))
                .map(|obj| crate::sim::production::foundation_dimensions(&obj.foundation))
                .unwrap_or((1, 1));
            counts.add_building_expanded_foundation(pos.0, pos.1, width, height);
        } else {
            counts.add_single_cell_neighbor_source(pos.0, pos.1);
        }
    }

    counts
}

// ---------------------------------------------------------------------------
// Sub-cell allocation
// ---------------------------------------------------------------------------

/// Find the first available sub-cell in a cell. Returns `None` if the cell is
/// full (3 infantry) or contains a vehicle/structure.
pub fn allocate_sub_cell(occ: Option<&CellOccupancy>, layer: MovementLayer) -> Option<u8> {
    let Some(o) = occ else {
        // Empty cell — first infantry gets sub-cell 2 (NE corner).
        return Some(FUNCTIONAL_SUB_CELLS[0]);
    };
    // Vehicle/structure in cell blocks all sub-cells.
    if o.has_blockers_on(layer) {
        return None;
    }
    let infantry: Vec<(u64, u8)> = o.infantry(layer).collect();
    if infantry.len() >= MAX_INFANTRY_PER_CELL {
        return None;
    }
    // Find first sub-cell not already occupied.
    FUNCTIONAL_SUB_CELLS
        .iter()
        .copied()
        .find(|&spot| !infantry.iter().any(|&(_, s)| s == spot))
}

/// Can infantry enter this cell? True if there's an available sub-cell and no
/// vehicles/structures blocking.
pub fn cell_passable_for_infantry(occ: Option<&CellOccupancy>, layer: MovementLayer) -> bool {
    allocate_sub_cell(occ, layer).is_some()
}

/// Find the first available sub-cell, accounting for both the (stale) occupancy
/// map and sub-cells reserved by earlier movers this tick.
///
/// This prevents duplicate sub-cell assignment when multiple infantry enter
/// the same cell within one simulation tick. Without this, the stale occupancy
/// map shows the cell as empty for all movers, causing overlapping sub-cells
/// and subsequent blocking/repath oscillation.
pub fn allocate_sub_cell_with_reserved(
    occ: Option<&CellOccupancy>,
    layer: MovementLayer,
    reserved: Option<&[u8]>,
) -> Option<u8> {
    // Vehicle/structure in cell blocks all sub-cells.
    if let Some(o) = occ {
        if o.has_blockers_on(layer) {
            return None;
        }
    }
    let infantry: Vec<(u64, u8)> = occ.map_or_else(Vec::new, |o| o.infantry(layer).collect());
    let stale_count: usize = infantry.len();
    let reserved_count: usize = reserved.map_or(0, |v| v.len());
    if stale_count + reserved_count >= MAX_INFANTRY_PER_CELL {
        return None;
    }
    FUNCTIONAL_SUB_CELLS.iter().copied().find(|&spot| {
        let in_stale: bool = infantry.iter().any(|&(_, s)| s == spot);
        let in_reserved: bool = reserved.is_some_and(|v| v.contains(&spot));
        !in_stale && !in_reserved
    })
}

/// Allocate sub-cell using quadrant-based directional preference tables.
///
/// Infantry approaching from a specific direction prefers the sub-cell on that
/// side of the diamond. If occupied, a directional preference table biases the
/// fallback. For center/NW entries, a random rotation picks which sub-cell to
/// try first.
///
/// Use this when the infantry's lepton position (approach direction) and RNG
/// are available. Falls back to `allocate_sub_cell_with_reserved` semantics
/// at call sites without position data (spawning, terrain checks).
pub fn allocate_sub_cell_with_preference(
    occ: Option<&CellOccupancy>,
    layer: MovementLayer,
    reserved: Option<&[u8]>,
    sub_x: SimFixed,
    sub_y: SimFixed,
    rng: &mut SimRng,
) -> Option<u8> {
    // Vehicle/structure blocks all infantry.
    if let Some(o) = occ {
        if o.has_blockers_on(layer) {
            return None;
        }
    }
    let infantry: Vec<(u64, u8)> = occ.map_or_else(Vec::new, |o| o.infantry(layer).collect());
    let stale_count: usize = infantry.len();
    let reserved_count: usize = reserved.map_or(0, |v| v.len());
    if stale_count + reserved_count >= MAX_INFANTRY_PER_CELL {
        return None;
    }

    let is_occupied = |spot: u8| -> bool {
        let in_stale: bool = infantry.iter().any(|&(_, s)| s == spot);
        let in_reserved: bool = reserved.is_some_and(|v| v.contains(&spot));
        in_stale || in_reserved
    };

    let quadrant: u8 = get_subcell_quadrant(sub_x, sub_y);

    // Fast-path: if the quadrant maps directly to a functional sub-cell and it's free,
    // use it without consulting the preference table.
    if quadrant >= 2 && !is_occupied(quadrant) {
        return Some(quadrant);
    }

    // Select preference list: random rotation for center/NW, fixed table otherwise.
    let pref: &[u8; 4] = if quadrant == 0 {
        let rotation: usize = rng.next_range_u32(4) as usize;
        &SUBCELL_RANDOM_ROTATIONS[rotation]
    } else {
        &SUBCELL_PREFERENCE[quadrant as usize]
    };

    // Search preference list, skipping indices 0 and 1 (matching original engine).
    for &spot in pref {
        if spot >= 2 && !is_occupied(spot) {
            return Some(spot);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Crush logic
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrushCapability {
    pub regular_crusher: bool,
    pub omni_crusher: bool,
}

impl CrushCapability {
    pub const fn new(regular_crusher: bool, omni_crusher: bool) -> Self {
        Self {
            regular_crusher,
            omni_crusher,
        }
    }

    pub const fn can_crush_units(self) -> bool {
        self.regular_crusher || self.omni_crusher
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriveCrushPhase {
    EnteringCell,
    FullyInCell,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DriveCrushOutcome {
    None,
    Scatter { blockers: Vec<u64> },
    Kill { victims: Vec<u64> },
}

pub const CRUSH_DISTANCE_SQ_LIMIT: i64 = 0x3fff;

pub fn within_crush_distance_sq(crusher: (i32, i32), victim: (i32, i32)) -> bool {
    let dx = i64::from(victim.0 - crusher.0);
    let dy = i64::from(victim.1 - crusher.1);
    dx * dx + dy * dy <= CRUSH_DISTANCE_SQ_LIMIT
}

fn entity_crush_coord(entity: &GameEntity) -> (i32, i32) {
    (
        i32::from(entity.position.rx) * 256 + entity.position.sub_x.to_num::<i32>(),
        i32::from(entity.position.ry) * 256 + entity.position.sub_y.to_num::<i32>(),
    )
}

/// Whether `mover_zone` can crush a target with the given properties.
///
/// Crush hierarchy:
///
/// 1. OmniCrushResistant blocks ALL crush (MCVs, Battle Fortress, Slave Miner, T-Rex)
/// 2. OmniCrusher (per-unit flag, only Battle Fortress) crushes anything not
///    OmniCrushResistant, regardless of Crushable flag
/// 3. CrusherAll (MovementZone) crushes walls — for unit crushing it works
///    like OmniCrusher since only BFRT has it and also has OmniCrusher=yes
/// 4. Standard Crusher zones crush only infantry with Crushable=yes
/// 5. Structures and aircraft are NEVER crushable
pub fn can_crush(
    capability: CrushCapability,
    target_category: EntityCategory,
    target_crushable: bool,
    target_low_silhouette: bool,
    target_omni_crush_resistant: bool,
) -> bool {
    // Structures and aircraft are never crushed.
    if matches!(
        target_category,
        EntityCategory::Structure | EntityCategory::Aircraft
    ) {
        return false;
    }
    // OmniCrushResistant blocks everything.
    if target_omni_crush_resistant {
        return false;
    }
    // OmniCrusher (Battle Fortress) crushes any non-resistant mobile entity.
    if capability.omni_crusher {
        return true;
    }

    capability.regular_crusher
        && target_category == EntityCategory::Infantry
        && target_crushable
        && !target_low_silhouette
}

fn is_low_silhouette_for_crush(entity: &GameEntity) -> bool {
    if entity.category != EntityCategory::Infantry {
        return false;
    }
    if entity.infantry.is_some_and(|infantry| infantry.is_prone) {
        return true;
    }
    matches!(
        entity.deploy_state,
        Some(crate::sim::deploy::DeployPhase::Deployed)
            | Some(crate::sim::deploy::DeployPhase::Undeploying { .. })
    ) && !entity.deployed_crushable
}

/// Collect entity IDs in a cell that the mover would crush on entry.
///
/// Returns an empty vec if the mover can't crush anything there.
pub fn collect_crush_victims(
    cell: (u16, u16),
    occupancy: &OccupancyGrid,
    layer: MovementLayer,
    crush_capability: CrushCapability,
    entities: &EntityStore,
) -> Vec<u64> {
    let Some(occ) = occupancy.get(cell.0, cell.1) else {
        return Vec::new();
    };
    let mut victims: Vec<u64> = Vec::new();

    for occupant in occ.iter_layer(layer) {
        if let Some(e) = entities.get(occupant.entity_id) {
            if can_crush(
                crush_capability,
                e.category,
                e.crushable,
                is_low_silhouette_for_crush(e),
                e.omni_crush_resistant,
            ) {
                victims.push(occupant.entity_id);
            }
        }
    }

    victims
}

/// Emit `EntityCrushed` (CrushSound) and `EntityDied` (DieSound) sound
/// events for a single crush victim. Each event is skipped if the
/// corresponding `ObjectType` field is `None`. Caller must invoke BEFORE
/// removing the victim from the EntityStore so victim.position and
/// victim.type_ref are still valid.
pub fn emit_crush_kill_sounds(
    victim: &crate::sim::game_entity::GameEntity,
    rules: &crate::rules::ruleset::RuleSet,
    interner: &mut crate::sim::intern::StringInterner,
    sound_events: &mut Vec<crate::sim::world::SimSoundEvent>,
) {
    emit_crush_kill_sounds_at(
        victim,
        (i32::from(victim.position.rx), i32::from(victim.position.ry)),
        rules,
        interner,
        sound_events,
    );
}

pub fn emit_crush_kill_sounds_at(
    victim: &crate::sim::game_entity::GameEntity,
    crush_coord: (i32, i32),
    rules: &crate::rules::ruleset::RuleSet,
    interner: &mut crate::sim::intern::StringInterner,
    sound_events: &mut Vec<crate::sim::world::SimSoundEvent>,
) {
    let rx = crush_coord.0.clamp(0, i32::from(u16::MAX)) as u16;
    let ry = crush_coord.1.clamp(0, i32::from(u16::MAX)) as u16;
    let type_str = interner.resolve(victim.type_ref).to_string();
    let Some(obj) = rules.object(&type_str) else {
        return;
    };
    if let Some(ref crush_sound) = obj.crush_sound {
        let id = interner.intern(crush_sound);
        sound_events.push(crate::sim::world::SimSoundEvent::EntityCrushed {
            crush_sound_id: id,
            rx,
            ry,
        });
    }
    if let Some(ref die_sound) = obj.die_sound {
        let id = interner.intern(die_sound);
        sound_events.push(crate::sim::world::SimSoundEvent::EntityDied {
            die_sound_id: id,
            rx,
            ry,
        });
    }
}

/// Check whether a mover can enter a cell after crushing all occupants.
///
/// Returns `true` if the mover can crush everything in the cell (i.e. the cell
/// would become empty after crush kills are applied).
pub fn cell_passable_after_crush(
    cell: (u16, u16),
    occupancy: &OccupancyGrid,
    layer: MovementLayer,
    crush_capability: CrushCapability,
    entities: &EntityStore,
) -> bool {
    let Some(occ) = occupancy.get(cell.0, cell.1) else {
        return true; // empty cell
    };
    // Boolean crush passability is category-specific; it does not choose a
    // first occupant from CellClass list order.
    // All blockers must be crushable.
    for eid in occ.blockers(layer) {
        if let Some(e) = entities.get(eid) {
            if !can_crush(
                crush_capability,
                e.category,
                e.crushable,
                is_low_silhouette_for_crush(e),
                e.omni_crush_resistant,
            ) {
                return false;
            }
        }
    }
    // All infantry must be crushable.
    for (eid, _) in occ.infantry(layer) {
        if let Some(e) = entities.get(eid) {
            if !can_crush(
                crush_capability,
                e.category,
                e.crushable,
                is_low_silhouette_for_crush(e),
                e.omni_crush_resistant,
            ) {
                return false;
            }
        }
    }
    true
}

pub fn classify_drive_crush_phase(
    phase: DriveCrushPhase,
    occ: &[u64],
    entities: &EntityStore,
    crusher_id: u64,
    alliances: &crate::map::houses::HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
    crusher_coord: (i32, i32),
    capability: CrushCapability,
) -> DriveCrushOutcome {
    if !capability.can_crush_units() {
        return DriveCrushOutcome::None;
    }
    let Some(crusher) = entities.get(crusher_id) else {
        return DriveCrushOutcome::None;
    };
    let crusher_owner = interner.resolve(crusher.owner);
    let mut selected = Vec::new();
    for &id in occ {
        if id == crusher_id {
            continue;
        }
        let Some(victim) = entities.get(id) else {
            continue;
        };
        match phase {
            DriveCrushPhase::EnteringCell => selected.push(id),
            DriveCrushPhase::FullyInCell => {
                let victim_owner = interner.resolve(victim.owner);
                if crate::map::houses::are_houses_friendly(alliances, crusher_owner, victim_owner) {
                    continue;
                }
                if !within_crush_distance_sq(crusher_coord, entity_crush_coord(victim)) {
                    continue;
                }
                if can_crush(
                    capability,
                    victim.category,
                    victim.crushable,
                    is_low_silhouette_for_crush(victim),
                    victim.omni_crush_resistant,
                ) {
                    selected.push(id);
                }
            }
        }
    }
    selected.sort_unstable();
    selected.dedup();
    match (phase, selected.is_empty()) {
        (_, true) => DriveCrushOutcome::None,
        (DriveCrushPhase::EnteringCell, false) => DriveCrushOutcome::Scatter { blockers: selected },
        (DriveCrushPhase::FullyInCell, false) => DriveCrushOutcome::Kill { victims: selected },
    }
}

// ---------------------------------------------------------------------------
// Scatter displacement (replaces old "bump" teleport)
// ---------------------------------------------------------------------------
//
// The original engine uses CellClass::Scatter_Objects to tell occupants to
// move out of the way. All 6 locomotor call sites pass force=1 with a
// NullCoord, which triggers UnitClass::Scatter Branch A: random direction,
// Set_Destination only (no mission change). The blocker walks away via its
// normal locomotor — it is never teleported.
//
// Our implementation: find a walkable, unoccupied adjacent cell and issue
// the blocker a 1-cell movement command via `issue_direct_move`.

/// Try to scatter a blocker to an adjacent cell by issuing a movement command.
///
/// Matches the original engine's movement scatter (Branch A — NullCoord):
/// search 8 neighbors starting from a random direction, pick the first
/// walkable + unoccupied cell, issue the blocker a movement order to walk
/// there.
///
/// Returns `true` if the blocker was given a scatter movement command.
pub fn scatter_blocker(
    entities: &mut EntityStore,
    blocker_id: u64,
    path_grid: Option<&PathGrid>,
    occupancy: &OccupancyGrid,
    layer: MovementLayer,
    rng: &mut SimRng,
) -> bool {
    // Read blocker properties (immutable borrow).
    let Some(blocker) = entities.get(blocker_id) else {
        return false;
    };
    // Buildings are immutable obstacles — never scatter targets. Bail before
    // the RNG read so determinism is preserved for all legitimate cases.
    if blocker.category == EntityCategory::Structure {
        return false;
    }
    // Don't scatter a blocker that's already moving.
    if blocker.movement_target.is_some() {
        return false;
    }
    let bpos = (blocker.position.rx, blocker.position.ry);
    let speed = blocker
        .locomotor
        .as_ref()
        .map(|l| l.speed_multiplier * crate::util::fixed_math::SimFixed::from_num(1024))
        .unwrap_or(crate::util::fixed_math::SimFixed::from_num(1024));

    // Find a valid adjacent cell. Random start direction matches Branch A.
    let start_dir = rng.next_range_u32(8) as usize;
    let mut target: Option<(u16, u16)> = None;

    for i in 0..8 {
        let dir = (start_dir + i) % 8;
        let (dx, dy) = NEIGHBOR_OFFSETS[dir];
        let nx = bpos.0 as i32 + dx;
        let ny = bpos.1 as i32 + dy;
        if nx < 0 || ny < 0 {
            continue;
        }
        let (nx, ny) = (nx as u16, ny as u16);

        // Must be walkable terrain.
        if let Some(grid) = path_grid {
            if !grid.is_walkable(nx, ny) {
                continue;
            }
        }
        // Must not be occupied by vehicles/structures. Infantry sub-cells OK.
        if let Some(occ) = occupancy.get(nx, ny) {
            if occ.has_blockers_on(layer) {
                continue;
            }
        }
        target = Some((nx, ny));
        break;
    }

    let Some(dest) = target else {
        return false;
    };

    // Issue a 1-cell movement command. The blocker walks there via normal
    // locomotor processing — no teleport.
    crate::sim::movement::movement_commands::issue_direct_move(entities, blocker_id, dest, speed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::game_entity::{GameEntity, InfantryRuntime};
    use crate::sim::occupancy::CellListInsertion;

    fn infantry(id: u64, rx: u16, ry: u16, sub: u8) -> GameEntity {
        let mut e = GameEntity::test_default(id, "E1", "Allies", rx, ry);
        e.category = EntityCategory::Infantry;
        e.sub_cell = Some(sub);
        e.crushable = true;
        e
    }

    fn vehicle(id: u64, rx: u16, ry: u16) -> GameEntity {
        let mut e = GameEntity::test_default(id, "MTNK", "Allies", rx, ry);
        e.category = EntityCategory::Unit;
        e.crushable = false;
        e
    }

    fn structure(id: u64, rx: u16, ry: u16) -> GameEntity {
        let mut e = GameEntity::test_default(id, "GAREFN", "Allies", rx, ry);
        e.category = EntityCategory::Structure;
        e.crushable = false;
        e
    }

    #[test]
    fn blocker_neighbor_counts_include_bridge_layer_occupants_globally() {
        let mut entities = EntityStore::new();
        let mut blocker = vehicle(1, 2, 2);
        blocker.on_bridge = true;
        entities.insert(blocker);
        let interner = crate::sim::intern::StringInterner::new();

        let counts = build_blocker_neighbor_counts(&entities, 5, 5, None, &interner, None);

        assert_eq!(counts.count_at(1, 2), 1);
        assert_eq!(counts.count_at(3, 3), 1);
        assert_eq!(counts.count_at(2, 2), 0);
    }

    #[test]
    fn blocker_neighbor_counts_building_uses_expanded_foundation_rectangle_once() {
        let mut entities = EntityStore::new();
        entities.insert(structure(1, 2, 2));
        let interner = crate::sim::intern::StringInterner::new();

        let counts = build_blocker_neighbor_counts(&entities, 5, 5, None, &interner, None);

        for y in 1..=3 {
            for x in 1..=3 {
                assert_eq!(
                    counts.count_at(x, y),
                    1,
                    "1x1 fallback structure should count expanded rectangle cell ({x},{y})"
                );
            }
        }
        assert_eq!(counts.count_at(0, 2), 0);
        assert_eq!(counts.count_at(2, 0), 0);
    }

    /// Helper: build an OccupancyGrid from a set of entity descriptions.
    fn make_occ(entries: &[(u16, u16, u64, MovementLayer, Option<u8>)]) -> OccupancyGrid {
        let mut grid = OccupancyGrid::new();
        for &(rx, ry, eid, layer, sub) in entries {
            grid.add(
                rx,
                ry,
                eid,
                layer,
                sub,
                CellListInsertion::PrependNonBuilding,
            );
        }
        grid
    }

    // -- can_crush tests --

    #[test]
    fn test_crusher_crushes_crushable_infantry() {
        assert!(can_crush(
            CrushCapability::new(true, false),
            EntityCategory::Infantry,
            true,
            false,
            false,
        ));
    }

    #[test]
    fn test_crusher_cannot_crush_non_crushable_infantry() {
        assert!(!can_crush(
            CrushCapability::new(true, false),
            EntityCategory::Infantry,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn test_regular_crusher_cannot_crush_low_silhouette_infantry() {
        assert!(!can_crush(
            CrushCapability::new(true, false),
            EntityCategory::Infantry,
            true,
            true, // low_silhouette
            false,
        ));
    }

    #[test]
    fn test_omni_crusher_crushes_non_crushable_infantry() {
        assert!(can_crush(
            CrushCapability::new(false, true),
            EntityCategory::Infantry,
            false,
            true, // low_silhouette does not gate CrusherAll/Omni-style crush
            false,
        ));
    }

    #[test]
    fn test_omni_crusher_crushes_vehicles() {
        assert!(can_crush(
            CrushCapability::new(false, true),
            EntityCategory::Unit,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn test_omni_crush_resistant_blocks_all() {
        assert!(!can_crush(
            CrushCapability::new(false, true),
            EntityCategory::Infantry,
            true,
            true,
            true, // omni_crush_resistant
        ));
    }

    #[test]
    fn test_structures_never_crushable() {
        assert!(!can_crush(
            CrushCapability::new(false, true),
            EntityCategory::Structure,
            true,
            false,
            false,
        ));
    }

    #[test]
    fn test_crusher_cannot_crush_vehicles() {
        assert!(!can_crush(
            CrushCapability::new(true, false),
            EntityCategory::Unit,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn test_normal_zone_cannot_crush() {
        assert!(!can_crush(
            CrushCapability::new(false, false),
            EntityCategory::Infantry,
            true,
            false,
            false,
        ));
    }

    #[test]
    fn normal_zone_regular_crusher_crushes_crushable_infantry() {
        assert!(can_crush(
            CrushCapability::new(true, false),
            EntityCategory::Infantry,
            true,
            false,
            false,
        ));
    }

    #[test]
    fn normal_zone_non_crusher_still_cannot_crush() {
        assert!(!can_crush(
            CrushCapability::new(false, false),
            EntityCategory::Infantry,
            true,
            false,
            false,
        ));
    }

    #[test]
    fn missing_crusher_flag_does_not_crush_infantry() {
        assert!(!can_crush(
            CrushCapability::new(false, false),
            EntityCategory::Infantry,
            true,
            false,
            false,
        ));
    }

    #[test]
    fn crush_distance_gate_includes_0x3fff() {
        assert!(within_crush_distance_sq((0, 0), (127, 14)));
    }

    #[test]
    fn crush_distance_gate_excludes_0x4000() {
        assert!(!within_crush_distance_sq((0, 0), (128, 0)));
    }

    #[test]
    fn classify_drive_crush_phase_entering_scatters_without_kill() {
        let mut entities = EntityStore::new();
        let mut crusher = vehicle(1, 5, 5);
        crusher.regular_crusher = true;
        entities.insert(crusher);
        let mut victim = GameEntity::test_default(2, "E1", "Soviet", 5, 5);
        victim.category = EntityCategory::Infantry;
        victim.crushable = true;
        entities.insert(victim);
        let interner = crate::sim::intern::test_interner();

        let outcome = classify_drive_crush_phase(
            DriveCrushPhase::EnteringCell,
            &[2],
            &entities,
            1,
            &crate::map::houses::HouseAllianceMap::new(),
            &interner,
            (5 * 256 + 128, 5 * 256 + 128),
            CrushCapability::new(true, false),
        );

        assert_eq!(outcome, DriveCrushOutcome::Scatter { blockers: vec![2] });
    }

    #[test]
    fn classify_drive_crush_phase_full_cell_kills_centered_enemy() {
        let mut entities = EntityStore::new();
        let mut crusher = vehicle(1, 5, 5);
        crusher.regular_crusher = true;
        entities.insert(crusher);
        let mut victim = GameEntity::test_default(2, "E1", "Soviet", 5, 5);
        victim.category = EntityCategory::Infantry;
        victim.crushable = true;
        entities.insert(victim);
        let interner = crate::sim::intern::test_interner();

        let outcome = classify_drive_crush_phase(
            DriveCrushPhase::FullyInCell,
            &[2],
            &entities,
            1,
            &crate::map::houses::HouseAllianceMap::new(),
            &interner,
            (5 * 256 + 128, 5 * 256 + 128),
            CrushCapability::new(true, false),
        );

        assert_eq!(outcome, DriveCrushOutcome::Kill { victims: vec![2] });
    }

    #[test]
    fn classify_drive_crush_phase_full_cell_skips_allied_victim() {
        let mut entities = EntityStore::new();
        let mut crusher = vehicle(1, 5, 5);
        crusher.regular_crusher = true;
        entities.insert(crusher);
        let mut victim = infantry(2, 5, 5, 2);
        victim.crushable = true;
        entities.insert(victim);
        let interner = crate::sim::intern::test_interner();

        let outcome = classify_drive_crush_phase(
            DriveCrushPhase::FullyInCell,
            &[2],
            &entities,
            1,
            &crate::map::houses::HouseAllianceMap::new(),
            &interner,
            (5 * 256 + 128, 5 * 256 + 128),
            CrushCapability::new(true, false),
        );

        assert_eq!(outcome, DriveCrushOutcome::None);
    }

    // -- sub-cell allocation tests --

    #[test]
    fn test_allocate_sub_cell_empty_cell() {
        // No occupancy entry → first spot (2 = NE corner).
        assert_eq!(allocate_sub_cell(None, MovementLayer::Ground), Some(2));
    }

    #[test]
    fn test_allocate_sub_cell_one_infantry() {
        let grid = make_occ(&[(5, 5, 1, MovementLayer::Ground, Some(2))]);
        let occ = grid.get(5, 5).unwrap();
        assert_eq!(allocate_sub_cell(Some(occ), MovementLayer::Ground), Some(3));
    }

    #[test]
    fn test_allocate_sub_cell_two_infantry() {
        let grid = make_occ(&[
            (5, 5, 1, MovementLayer::Ground, Some(2)),
            (5, 5, 2, MovementLayer::Ground, Some(3)),
        ]);
        let occ = grid.get(5, 5).unwrap();
        assert_eq!(allocate_sub_cell(Some(occ), MovementLayer::Ground), Some(4));
    }

    #[test]
    fn test_allocate_sub_cell_full() {
        let grid = make_occ(&[
            (5, 5, 1, MovementLayer::Ground, Some(2)),
            (5, 5, 2, MovementLayer::Ground, Some(3)),
            (5, 5, 3, MovementLayer::Ground, Some(4)),
        ]);
        let occ = grid.get(5, 5).unwrap();
        assert_eq!(allocate_sub_cell(Some(occ), MovementLayer::Ground), None);
    }

    #[test]
    fn test_vehicle_blocks_all_sub_cells() {
        let grid = make_occ(&[(5, 5, 99, MovementLayer::Ground, None)]);
        let occ = grid.get(5, 5).unwrap();
        assert_eq!(allocate_sub_cell(Some(occ), MovementLayer::Ground), None);
    }

    #[test]
    fn test_cell_passable_for_infantry_empty() {
        assert!(cell_passable_for_infantry(None, MovementLayer::Ground));
    }

    #[test]
    fn test_cell_passable_for_infantry_with_vehicle() {
        let grid = make_occ(&[(5, 5, 1, MovementLayer::Ground, None)]);
        let occ = grid.get(5, 5).unwrap();
        assert!(!cell_passable_for_infantry(
            Some(occ),
            MovementLayer::Ground
        ));
    }

    // -- collect_crush_victims tests --

    #[test]
    fn test_collect_crush_victims_infantry() {
        let mut store = EntityStore::new();
        let inf = infantry(1, 5, 5, 2);
        store.insert(inf);

        let grid = make_occ(&[(5, 5, 1, MovementLayer::Ground, Some(2))]);

        let victims = collect_crush_victims(
            (5, 5),
            &grid,
            MovementLayer::Ground,
            CrushCapability::new(true, false),
            &store,
        );
        assert_eq!(victims, vec![1]);
    }

    #[test]
    fn test_collect_crush_victims_non_crushable() {
        let mut store = EntityStore::new();
        let mut inf = infantry(1, 5, 5, 2);
        inf.crushable = false;
        store.insert(inf);

        let grid = make_occ(&[(5, 5, 1, MovementLayer::Ground, Some(2))]);

        let victims = collect_crush_victims(
            (5, 5),
            &grid,
            MovementLayer::Ground,
            CrushCapability::new(true, false),
            &store,
        );
        assert!(victims.is_empty());
    }

    #[test]
    fn test_collect_crush_victims_skips_deployed_uncrushable_infantry() {
        let mut store = EntityStore::new();
        let mut inf = infantry(1, 5, 5, 2);
        inf.deploy_state = Some(crate::sim::deploy::DeployPhase::Deployed);
        inf.deployed_crushable = false;
        store.insert(inf);

        let grid = make_occ(&[(5, 5, 1, MovementLayer::Ground, Some(2))]);

        let victims = collect_crush_victims(
            (5, 5),
            &grid,
            MovementLayer::Ground,
            CrushCapability::new(true, false),
            &store,
        );
        assert!(victims.is_empty());
    }

    #[test]
    fn test_collect_crush_victims_keeps_deployed_crushable_infantry_crushable() {
        let mut store = EntityStore::new();
        let mut inf = infantry(1, 5, 5, 2);
        inf.deploy_state = Some(crate::sim::deploy::DeployPhase::Deployed);
        inf.deployed_crushable = true;
        store.insert(inf);

        let grid = make_occ(&[(5, 5, 1, MovementLayer::Ground, Some(2))]);

        let victims = collect_crush_victims(
            (5, 5),
            &grid,
            MovementLayer::Ground,
            CrushCapability::new(true, false),
            &store,
        );
        assert_eq!(victims, vec![1]);
    }

    #[test]
    fn test_collect_crush_victims_skips_prone_infantry_for_regular_crusher() {
        let mut store = EntityStore::new();
        let mut inf = infantry(1, 5, 5, 2);
        inf.infantry = Some(InfantryRuntime {
            fear_level: 50,
            is_prone: true,
        });
        store.insert(inf);

        let grid = make_occ(&[(5, 5, 1, MovementLayer::Ground, Some(2))]);

        let victims = collect_crush_victims(
            (5, 5),
            &grid,
            MovementLayer::Ground,
            CrushCapability::new(true, false),
            &store,
        );
        assert!(victims.is_empty());
    }

    // -- scatter_blocker tests --

    #[test]
    fn test_scatter_blocker_issues_movement() {
        let grid = PathGrid::new(10, 10);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(42);

        let mut store = EntityStore::new();
        let v = vehicle(1, 5, 5);
        store.insert(v);

        let result = scatter_blocker(
            &mut store,
            1,
            Some(&grid),
            &occupancy,
            MovementLayer::Ground,
            &mut rng,
        );
        assert!(result, "scatter_blocker should succeed with open cells");

        // Blocker should now have a movement_target (walking, not teleported).
        let e = store.get(1).unwrap();
        assert!(
            e.movement_target.is_some(),
            "Blocker should have a movement command"
        );
        // Position should NOT have changed yet — blocker walks on next tick.
        assert_eq!(e.position.rx, 5);
        assert_eq!(e.position.ry, 5);
    }

    #[test]
    fn test_scatter_blocker_all_blocked() {
        let grid = PathGrid::new(3, 3);
        let mut occupancy = OccupancyGrid::new();
        for &(dx, dy) in &NEIGHBOR_OFFSETS {
            let nx = (1 + dx) as u16;
            let ny = (1 + dy) as u16;
            occupancy.add(
                nx,
                ny,
                100,
                MovementLayer::Ground,
                None,
                CellListInsertion::PrependNonBuilding,
            );
        }
        let mut rng = SimRng::new(42);

        let mut store = EntityStore::new();
        let v = vehicle(1, 1, 1);
        store.insert(v);

        let result = scatter_blocker(
            &mut store,
            1,
            Some(&grid),
            &occupancy,
            MovementLayer::Ground,
            &mut rng,
        );
        assert!(!result, "scatter_blocker should fail when all blocked");
        assert!(store.get(1).unwrap().movement_target.is_none());
    }

    #[test]
    fn test_scatter_blocker_skips_already_moving() {
        let grid = PathGrid::new(10, 10);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(42);

        let mut store = EntityStore::new();
        let mut v = vehicle(1, 5, 5);
        v.movement_target = Some(crate::sim::components::MovementTarget {
            path: vec![(5, 5), (6, 5)],
            path_layers: vec![MovementLayer::Ground; 2],
            next_index: 1,
            speed: crate::util::fixed_math::SimFixed::from_num(1024),
            ..Default::default()
        });
        store.insert(v);

        let result = scatter_blocker(
            &mut store,
            1,
            Some(&grid),
            &occupancy,
            MovementLayer::Ground,
            &mut rng,
        );
        assert!(
            !result,
            "scatter_blocker should not scatter already-moving unit"
        );
    }

    #[test]
    fn test_scatter_blocker_skips_structure() {
        let grid = PathGrid::new(10, 10);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(42);

        let mut store = EntityStore::new();
        store.insert(structure(100, 5, 5));

        let result = scatter_blocker(
            &mut store,
            100,
            Some(&grid),
            &occupancy,
            MovementLayer::Ground,
            &mut rng,
        );

        assert!(
            !result,
            "scatter_blocker must refuse Structure blockers — buildings are \
             never scatter targets in the original engine"
        );

        // Structure must not have been issued any movement.
        let e = store.get(100).expect("structure still alive");
        assert!(
            e.movement_target.is_none(),
            "Structure must not receive a movement_target from scatter"
        );

        // RNG must NOT have been consumed (determinism: a fresh rng with the
        // same seed gives the same first value as one that hasn't been touched).
        let mut control_rng = SimRng::new(42);
        assert_eq!(
            rng.next_range_u32(8),
            control_rng.next_range_u32(8),
            "scatter_blocker must not consume RNG when bailing on a Structure blocker"
        );
    }

    #[test]
    fn test_scatter_deterministic() {
        let grid = PathGrid::new(10, 10);
        let occupancy = OccupancyGrid::new();

        let mut store1 = EntityStore::new();
        store1.insert(vehicle(1, 5, 5));
        let mut rng1 = SimRng::new(42);
        scatter_blocker(
            &mut store1,
            1,
            Some(&grid),
            &occupancy,
            MovementLayer::Ground,
            &mut rng1,
        );

        let mut store2 = EntityStore::new();
        store2.insert(vehicle(1, 5, 5));
        let mut rng2 = SimRng::new(42);
        scatter_blocker(
            &mut store2,
            1,
            Some(&grid),
            &occupancy,
            MovementLayer::Ground,
            &mut rng2,
        );

        let t1 = store1.get(1).unwrap().movement_target.as_ref().unwrap();
        let t2 = store2.get(1).unwrap().movement_target.as_ref().unwrap();
        assert_eq!(t1.path, t2.path, "Scatter must be deterministic");
    }

    // -- allocate_sub_cell_with_reserved tests --

    #[test]
    fn test_allocate_with_reserved_empty_cell_no_reservations() {
        assert_eq!(
            allocate_sub_cell_with_reserved(None, MovementLayer::Ground, None),
            Some(2)
        );
    }

    #[test]
    fn test_allocate_with_reserved_skips_reserved_spot() {
        let reserved: Vec<u8> = vec![2];
        assert_eq!(
            allocate_sub_cell_with_reserved(None, MovementLayer::Ground, Some(&reserved)),
            Some(3)
        );
    }

    #[test]
    fn test_allocate_with_reserved_full_from_reservations() {
        let reserved: Vec<u8> = vec![2, 3, 4];
        assert_eq!(
            allocate_sub_cell_with_reserved(None, MovementLayer::Ground, Some(&reserved)),
            None
        );
    }

    #[test]
    fn test_allocate_with_reserved_full_mixed() {
        let grid = make_occ(&[
            (5, 5, 1, MovementLayer::Ground, Some(2)),
            (5, 5, 2, MovementLayer::Ground, Some(3)),
        ]);
        let occ = grid.get(5, 5).unwrap();
        let reserved: Vec<u8> = vec![4];
        assert_eq!(
            allocate_sub_cell_with_reserved(Some(occ), MovementLayer::Ground, Some(&reserved)),
            None
        );
    }

    #[test]
    fn test_allocate_with_reserved_vehicle_blocks() {
        let grid = make_occ(&[(5, 5, 99, MovementLayer::Ground, None)]);
        let occ = grid.get(5, 5).unwrap();
        assert_eq!(
            allocate_sub_cell_with_reserved(Some(occ), MovementLayer::Ground, None),
            None
        );
    }

    // -- quadrant detection tests --

    #[test]
    fn test_quadrant_center() {
        // Distance from (128,128) is 0 — well within 60-lepton threshold.
        assert_eq!(
            get_subcell_quadrant(SimFixed::from_num(128), SimFixed::from_num(128)),
            0
        );
    }

    #[test]
    fn test_quadrant_near_center() {
        // (150, 140): distance = sqrt(22^2 + 12^2) ≈ 25 — within 60-lepton threshold.
        assert_eq!(
            get_subcell_quadrant(SimFixed::from_num(150), SimFixed::from_num(140)),
            0
        );
    }

    #[test]
    fn test_quadrant_nw_returns_zero() {
        // (40, 40): X<=128, Y<=128 → NW quadrant → returns 0 (merged with center).
        assert_eq!(
            get_subcell_quadrant(SimFixed::from_num(40), SimFixed::from_num(40)),
            0
        );
    }

    #[test]
    fn test_quadrant_ne() {
        // (200, 40): X>128, Y<=128 → bits=1 → returns 2 (NE).
        assert_eq!(
            get_subcell_quadrant(SimFixed::from_num(200), SimFixed::from_num(40)),
            2
        );
    }

    #[test]
    fn test_quadrant_sw() {
        // (40, 200): X<=128, Y>128 → bits=2 → returns 3 (SW).
        assert_eq!(
            get_subcell_quadrant(SimFixed::from_num(40), SimFixed::from_num(200)),
            3
        );
    }

    #[test]
    fn test_quadrant_se() {
        // (200, 200): X>128, Y>128 → bits=3 → returns 4 (SE).
        assert_eq!(
            get_subcell_quadrant(SimFixed::from_num(200), SimFixed::from_num(200)),
            4
        );
    }

    // -- preference-aware allocation tests --

    #[test]
    fn test_preference_ne_entry_fast_path() {
        let mut rng = SimRng::new(42);
        let result = allocate_sub_cell_with_preference(
            None,
            MovementLayer::Ground,
            None,
            SimFixed::from_num(200),
            SimFixed::from_num(40),
            &mut rng,
        );
        assert_eq!(result, Some(2));
    }

    #[test]
    fn test_preference_ne_entry_occupied_fallback() {
        let grid = make_occ(&[(5, 5, 1, MovementLayer::Ground, Some(2))]);
        let occ = grid.get(5, 5).unwrap();
        let mut rng = SimRng::new(42);
        let result = allocate_sub_cell_with_preference(
            Some(occ),
            MovementLayer::Ground,
            None,
            SimFixed::from_num(200),
            SimFixed::from_num(40),
            &mut rng,
        );
        assert_eq!(result, Some(4));
    }

    #[test]
    fn test_preference_sw_entry() {
        let mut rng = SimRng::new(42);
        let result = allocate_sub_cell_with_preference(
            None,
            MovementLayer::Ground,
            None,
            SimFixed::from_num(40),
            SimFixed::from_num(200),
            &mut rng,
        );
        assert_eq!(result, Some(3));
    }

    #[test]
    fn test_preference_sw_entry_occupied_fallback() {
        let grid = make_occ(&[(5, 5, 1, MovementLayer::Ground, Some(3))]);
        let occ = grid.get(5, 5).unwrap();
        let mut rng = SimRng::new(42);
        let result = allocate_sub_cell_with_preference(
            Some(occ),
            MovementLayer::Ground,
            None,
            SimFixed::from_num(40),
            SimFixed::from_num(200),
            &mut rng,
        );
        assert_eq!(result, Some(4));
    }

    #[test]
    fn test_preference_se_entry() {
        let mut rng = SimRng::new(42);
        let result = allocate_sub_cell_with_preference(
            None,
            MovementLayer::Ground,
            None,
            SimFixed::from_num(200),
            SimFixed::from_num(200),
            &mut rng,
        );
        assert_eq!(result, Some(4));
    }

    #[test]
    fn test_preference_se_entry_occupied_fallback() {
        let grid = make_occ(&[(5, 5, 1, MovementLayer::Ground, Some(4))]);
        let occ = grid.get(5, 5).unwrap();
        let mut rng = SimRng::new(42);
        let result = allocate_sub_cell_with_preference(
            Some(occ),
            MovementLayer::Ground,
            None,
            SimFixed::from_num(200),
            SimFixed::from_num(200),
            &mut rng,
        );
        assert_eq!(result, Some(2));
    }

    #[test]
    fn test_preference_center_entry_randomizes() {
        let mut seen: BTreeSet<u8> = BTreeSet::new();
        for seed in 0..20u64 {
            let mut rng = SimRng::new(seed);
            let result = allocate_sub_cell_with_preference(
                None,
                MovementLayer::Ground,
                None,
                SimFixed::from_num(128),
                SimFixed::from_num(128),
                &mut rng,
            );
            assert!(result.is_some());
            seen.insert(result.unwrap());
        }
        assert!(seen.contains(&2), "expected sub-cell 2 from randomization");
        assert!(seen.contains(&3), "expected sub-cell 3 from randomization");
        assert!(seen.contains(&4), "expected sub-cell 4 from randomization");
    }

    #[test]
    fn test_preference_all_occupied() {
        let grid = make_occ(&[
            (5, 5, 1, MovementLayer::Ground, Some(2)),
            (5, 5, 2, MovementLayer::Ground, Some(3)),
            (5, 5, 3, MovementLayer::Ground, Some(4)),
        ]);
        let occ = grid.get(5, 5).unwrap();
        let mut rng = SimRng::new(42);
        let result = allocate_sub_cell_with_preference(
            Some(occ),
            MovementLayer::Ground,
            None,
            SimFixed::from_num(200),
            SimFixed::from_num(40),
            &mut rng,
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_preference_respects_reserved() {
        let reserved: Vec<u8> = vec![2];
        let mut rng = SimRng::new(42);
        let result = allocate_sub_cell_with_preference(
            None,
            MovementLayer::Ground,
            Some(&reserved),
            SimFixed::from_num(200),
            SimFixed::from_num(40),
            &mut rng,
        );
        assert_eq!(result, Some(4));
    }

    #[test]
    fn test_preference_vehicle_blocks() {
        let grid = make_occ(&[(5, 5, 99, MovementLayer::Ground, None)]);
        let occ = grid.get(5, 5).unwrap();
        let mut rng = SimRng::new(42);
        let result = allocate_sub_cell_with_preference(
            Some(occ),
            MovementLayer::Ground,
            None,
            SimFixed::from_num(200),
            SimFixed::from_num(40),
            &mut rng,
        );
        assert_eq!(result, None);
    }

    // -- emit_crush_kill_sounds tests --

    fn build_test_rules(
        crush_sound: Option<&str>,
        die_sound: Option<&str>,
    ) -> crate::rules::ruleset::RuleSet {
        let mut e1 = String::from("Strength=125\nArmor=none\nSpeed=4\n");
        if let Some(s) = crush_sound {
            e1.push_str(&format!("CrushSound={}\n", s));
        }
        if let Some(s) = die_sound {
            e1.push_str(&format!("DieSound={}\n", s));
        }
        let ini_text = format!(
            "[InfantryTypes]\n0=E1\n\n[VehicleTypes]\n\n[AircraftTypes]\n\n[BuildingTypes]\n\n[E1]\n{}\n",
            e1
        );
        let ini = crate::rules::ini_parser::IniFile::from_str(&ini_text);
        crate::rules::ruleset::RuleSet::from_ini(&ini).expect("test rules build")
    }

    fn build_victim(
        interner: &mut crate::sim::intern::StringInterner,
        rx: u16,
        ry: u16,
    ) -> GameEntity {
        let mut victim = infantry(1, rx, ry, 2);
        victim.type_ref = interner.intern("E1");
        victim
    }

    #[test]
    fn emit_crush_kill_sounds_emits_both_when_both_keys_set() {
        let rules = build_test_rules(Some("InfantrySquish"), Some("GIDie"));
        let mut interner = crate::sim::intern::StringInterner::new();
        let victim = build_victim(&mut interner, 5, 5);
        let mut events = Vec::new();

        emit_crush_kill_sounds(&victim, &rules, &mut interner, &mut events);

        assert_eq!(events.len(), 2, "expected 2 events, got {:?}", events);
        let crushed = events.iter().find_map(|e| match e {
            crate::sim::world::SimSoundEvent::EntityCrushed {
                crush_sound_id,
                rx,
                ry,
            } => Some((*crush_sound_id, *rx, *ry)),
            _ => None,
        });
        let (cid, crx, cry) = crushed.expect("missing EntityCrushed");
        assert_eq!(interner.resolve(cid), "InfantrySquish");
        assert_eq!((crx, cry), (5, 5));

        let died = events.iter().find_map(|e| match e {
            crate::sim::world::SimSoundEvent::EntityDied {
                die_sound_id,
                rx,
                ry,
            } => Some((*die_sound_id, *rx, *ry)),
            _ => None,
        });
        let (did, drx, dry) = died.expect("missing EntityDied");
        assert_eq!(interner.resolve(did), "GIDie");
        assert_eq!((drx, dry), (5, 5));
    }

    #[test]
    fn emit_crush_kill_sounds_skips_crush_when_field_is_none() {
        let rules = build_test_rules(None, Some("GIDie"));
        let mut interner = crate::sim::intern::StringInterner::new();
        let victim = build_victim(&mut interner, 7, 9);
        let mut events = Vec::new();

        emit_crush_kill_sounds(&victim, &rules, &mut interner, &mut events);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            crate::sim::world::SimSoundEvent::EntityDied { .. }
        ));
    }

    #[test]
    fn emit_crush_kill_sounds_skips_die_when_field_is_none() {
        let rules = build_test_rules(Some("InfantrySquish"), None);
        let mut interner = crate::sim::intern::StringInterner::new();
        let victim = build_victim(&mut interner, 3, 4);
        let mut events = Vec::new();

        emit_crush_kill_sounds(&victim, &rules, &mut interner, &mut events);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            crate::sim::world::SimSoundEvent::EntityCrushed { .. }
        ));
    }

    #[test]
    fn emit_crush_kill_sounds_no_events_when_both_none() {
        let rules = build_test_rules(None, None);
        let mut interner = crate::sim::intern::StringInterner::new();
        let victim = build_victim(&mut interner, 1, 1);
        let mut events = Vec::new();

        emit_crush_kill_sounds(&victim, &rules, &mut interner, &mut events);

        assert!(events.is_empty(), "expected no events, got {:?}", events);
    }
}
