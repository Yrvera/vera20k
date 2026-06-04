//! Nearby-passable-cell search (engine `Find_Nearby_Passable_Cell`).
//!
//! Diamond-ring expansion around a seed cell; per-candidate passability (plus an
//! optional occupancy check that always SKIPS reservations); frame-counter
//! selection when no target cell is given, nearest-distance selection when a
//! target is given. This is a read-only projection over the cell grids — it
//! mutates nothing and consumes no RNG stream, so threading it changes no hashed
//! state by itself. Depends only on `sim/` grids + `rules/` data; never on
//! render/ui/sidebar/audio/net.
//!
//! Determinism contract:
//! - The diamond-ring candidate ORDER is fully deterministic; it feeds both the
//!   frame-counter modulo index and the nearest-distance tie-break.
//! - Nearest-distance uses integer squared Euclidean distance (`dx*dx + dy*dy`)
//!   so the comparison stays fixed-point per the sim layering invariant.
//! - `frame_counter % pool.len()` reproduces the engine's same-tick aliasing by
//!   construction: two no-target calls on the same frame with the same candidate
//!   count return the same index. Do NOT add any per-call perturbation.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::sim::cell_rect::{
    CellRect, CellRectOccupancyContext, CellRectPassabilityContext, check_occupancy_rect,
    check_passability_rect,
};
use crate::sim::entity_store::EntityStore;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::zone_map::{ZoneGrid, ZoneId};

/// Hard radius cap for the diamond-ring search — `min(Speed + Sight, RADIUS_HARD_CAP)`.
pub const RADIUS_HARD_CAP: u16 = 32;
/// Candidate-pool early-terminate count — the search stops collecting once it has
/// this many surviving candidates.
pub const MAX_CANDIDATES: usize = 24;

/// The subset of the passability config FNPC always supplies the same way for each
/// 1x1 candidate. FNPC always passes `required_height_or_level = -1` (None) and
/// `reject_any_overlay = false`; those are fixed by the search, not the caller.
#[derive(Debug, Clone, Copy)]
pub struct PassabilityArgs {
    pub speed_type: SpeedType,
    pub required_zone_id: Option<ZoneId>,
    pub movement_zone: MovementZone,
    pub bridge_aware_zone: bool,
}

/// FNPC query — mirrors the engine `Find_Nearby_Passable_Cell` caller args.
pub struct NearbyQuery<'a> {
    /// Per-candidate passability config (built into a 1x1 passability rect).
    pub passability: PassabilityArgs,
    /// Bridge filter applied AFTER passability (an FNPC filter, not a passability arg).
    pub allow_bridge_cells: bool,
    /// FNPC's internal `±2` height gate — DEFERRED until its comparison semantics
    /// are pinned; the spawn path passes required-height `-1`, so it no-ops there.
    pub check_height: bool,
    /// When set, each candidate also runs `check_occupancy_rect(.., reservation_arg = -1)`.
    pub check_occupancy: bool,
    /// Radius cap; the caller computes `min(Speed + Sight, RADIUS_HARD_CAP)`.
    pub radius_cap: u16,
    /// `None` => frame-counter selection; `Some` => nearest-distance to target.
    pub target_cell: Option<(i32, i32)>,
    // Borrowed grids the per-candidate predicates read:
    pub path_grid: Option<&'a PathGrid>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub overlay_grid: Option<&'a OverlayGrid>,
    pub occupancy: Option<&'a OccupancyGrid>,
    pub entities: Option<&'a EntityStore>,
    pub zone_grid: Option<&'a ZoneGrid>,
    pub map_size: Option<(u16, u16)>,
}

/// A surviving FNPC candidate, classified `direct` (on a cardinal axis from the
/// seed) vs indirect. Direct candidates are preferred at selection time.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    cell: (i32, i32),
    direct: bool,
}

/// Engine `Find_Nearby_Passable_Cell`.
///
/// `frame_counter` MUST be the sim per-tick counter (`Simulation::binary_frame`),
/// read as the current frame — never an RNG draw. Returns `None` for the
/// no-candidate case (engine null-cell `{0,0}`), which the caller interprets as
/// "no cell": clear the destination, retry next tick.
pub fn find_nearby_passable_cell(
    seed: (i32, i32),
    q: &NearbyQuery<'_>,
    frame_counter: u32,
) -> Option<(u16, u16)> {
    let candidates = collect_candidates(seed, q);
    if candidates.is_empty() {
        return None;
    }

    // Direct candidates are preferred; only fall back to indirects when there are
    // no directs at all.
    let has_direct = candidates.iter().any(|c| c.direct);
    let pool: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| c.direct == has_direct)
        .collect();
    if pool.is_empty() {
        return None;
    }

    let chosen = match q.target_cell {
        // No target: deterministic frame-counter modulo over the preferred pool.
        None => pool[(frame_counter as usize) % pool.len()],
        // Target given: nearest by integer squared Euclidean distance; ties resolve
        // to the earlier ring-order candidate (stable, no frame/RNG input).
        Some((tx, ty)) => pool
            .iter()
            .copied()
            .min_by_key(|c| {
                let dx = (c.cell.0 - tx) as i64;
                let dy = (c.cell.1 - ty) as i64;
                dx * dx + dy * dy
            })
            .expect("pool is non-empty"),
    };

    cell_to_u16(chosen.cell)
}

/// Walk concentric diamond rings outward from the seed, collecting surviving
/// candidates in deterministic order, early-terminating at `MAX_CANDIDATES`.
///
/// Ring `r` is the set of cells with `|dx| + |dy| == r`. Within a ring the visit
/// order is the top edge, then the bottom edge, then the left and right columns —
/// a fixed traversal so the candidate index is reproducible. Ring 0 (the seed
/// itself) is included first.
fn collect_candidates(seed: (i32, i32), q: &NearbyQuery<'_>) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let cap = q.radius_cap.min(RADIUS_HARD_CAP) as i32;

    for r in 0..=cap {
        for (cx, cy) in diamond_ring(seed, r) {
            if !candidate_passes(q, cx, cy) {
                continue;
            }
            // Direct == on a cardinal axis from the seed (a straight line out).
            let direct = cx == seed.0 || cy == seed.1;
            out.push(Candidate {
                cell: (cx, cy),
                direct,
            });
            if out.len() >= MAX_CANDIDATES {
                return out;
            }
        }
    }
    out
}

/// Cells of the diamond ring `|dx| + |dy| == r` around `seed`, in a fixed order:
/// the top edge (dy from -r upward to 0) and the bottom edge first as full rows,
/// then the left and right columns for the interior rows. Ring 0 is just the seed.
fn diamond_ring(seed: (i32, i32), r: i32) -> Vec<(i32, i32)> {
    if r == 0 {
        return vec![seed];
    }
    let mut cells: Vec<(i32, i32)> = Vec::with_capacity((4 * r) as usize);
    // Top apex row down to the bottom apex, visiting each ring row's two cells in
    // left-then-right order; the apex rows have a single cell.
    for dy in -r..=r {
        let span = r - dy.abs(); // |dx| at this row
        if span == 0 {
            cells.push((seed.0, seed.1 + dy));
        } else {
            cells.push((seed.0 - span, seed.1 + dy));
            cells.push((seed.0 + span, seed.1 + dy));
        }
    }
    cells
}

/// Run the per-candidate predicates in FNPC order: passability first (1x1 rect,
/// `required_height_or_level = -1`, `reject_any_overlay = false`), then the optional
/// occupancy check with reservations SKIPPED (`-1`), then the bridge filter AFTER
/// both (a candidate that is a structural-bridge cell is dropped unless bridges are
/// allowed).
fn candidate_passes(q: &NearbyQuery<'_>, cx: i32, cy: i32) -> bool {
    let rect = CellRect::new(cx, cy, 1, 1);

    let passable = check_passability_rect(CellRectPassabilityContext {
        rect,
        speed_type: q.passability.speed_type,
        required_zone_id: q.passability.required_zone_id,
        movement_zone: q.passability.movement_zone,
        required_height_or_level: None, // FNPC always passes -1 (L21)
        bridge_aware_zone: q.passability.bridge_aware_zone,
        reject_any_overlay: false, // FNPC passes 0 (overlays not rejected here)
        path_grid: q.path_grid,
        resolved_terrain: q.resolved_terrain,
        overlay_grid: q.overlay_grid,
        occupancy: q.occupancy,
        zone_grid: q.zone_grid,
    });
    if !passable {
        return false;
    }

    if q.check_occupancy
        && !check_occupancy_rect(CellRectOccupancyContext {
            rect,
            reservation_arg: -1, // FNPC always SKIPS reservation (never a house index)
            reservations: None,
            occupancy: q.occupancy,
            entities: q.entities,
            resolved_terrain: q.resolved_terrain,
            overlay_grid: q.overlay_grid,
            map_size: q.map_size,
        })
    {
        return false;
    }

    // Bridge filter applied AFTER passability/occupancy.
    if !q.allow_bridge_cells && candidate_is_bridge_cell(q, cx, cy) {
        return false;
    }

    true
}

/// Whether a candidate cell is a structural-bridge cell (filtered out when bridges
/// are disallowed). Reads both the terrain bridge facts and the path-grid bridge bit.
fn candidate_is_bridge_cell(q: &NearbyQuery<'_>, cx: i32, cy: i32) -> bool {
    let (Ok(rx), Ok(ry)) = (u16::try_from(cx), u16::try_from(cy)) else {
        return false;
    };
    let terrain_bridge = q
        .resolved_terrain
        .and_then(|t| t.cell(rx, ry))
        .is_some_and(|c| c.bridge_facts.has_structural_bridge());
    let path_bridge = q
        .path_grid
        .and_then(|g| g.cell(rx, ry))
        .is_some_and(|c| c.has_structural_bridge());
    terrain_bridge || path_bridge
}

fn cell_to_u16(cell: (i32, i32)) -> Option<(u16, u16)> {
    match (u16::try_from(cell.0), u16::try_from(cell.1)) {
        (Ok(x), Ok(y)) => Some((x, y)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::bridge_facts::{BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts};
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};

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

    fn track_args() -> PassabilityArgs {
        PassabilityArgs {
            speed_type: SpeedType::Track,
            required_zone_id: None,
            movement_zone: MovementZone::Normal,
            bridge_aware_zone: false,
        }
    }

    fn base_query<'a>(
        terrain: &'a ResolvedTerrainGrid,
        path_grid: &'a PathGrid,
    ) -> NearbyQuery<'a> {
        NearbyQuery {
            passability: track_args(),
            allow_bridge_cells: true,
            check_height: false,
            check_occupancy: false,
            radius_cap: RADIUS_HARD_CAP,
            target_cell: None,
            path_grid: Some(path_grid),
            resolved_terrain: Some(terrain),
            overlay_grid: None,
            occupancy: None,
            entities: None,
            zone_grid: None,
            map_size: Some((terrain.width(), terrain.height())),
        }
    }

    #[test]
    fn find_nearby_diamond_ring_visit_order() {
        // Ring 0 is the seed; ring 1 visits the four orthogonal neighbours in the
        // fixed top->bottom, left->right traversal. The radius cap and the 24-cap
        // are honored.
        let ring0 = diamond_ring((5, 5), 0);
        assert_eq!(ring0, vec![(5, 5)]);
        let ring1 = diamond_ring((5, 5), 1);
        assert_eq!(ring1, vec![(5, 4), (4, 5), (6, 5), (5, 6)]);
        // Ring 2 has 8 cells (|dx|+|dy| == 2).
        assert_eq!(diamond_ring((5, 5), 2).len(), 8);

        // Radius cap clamps to RADIUS_HARD_CAP and the pool early-terminates at 24.
        let terrain = flat_terrain(40, 40);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let mut q = base_query(&terrain, &path_grid);
        q.radius_cap = 100; // requests beyond the hard cap
        let candidates = collect_candidates((20, 20), &q);
        assert_eq!(candidates.len(), MAX_CANDIDATES);
    }

    #[test]
    fn find_nearby_calls_occupancy_with_skip_reservation() {
        // FNPC's per-candidate occupancy check always uses reservation_arg = -1
        // (SkipReservation) and never a house index. With an occupant in the seed cell,
        // the seed is dropped (its Ground layer is non-empty) and a free neighbour is
        // returned instead — exercising the occupancy gate on the SkipReservation path.
        use crate::sim::movement::locomotor::MovementLayer;
        use crate::sim::occupancy::{CellListInsertion, OccupancyGrid};
        let terrain = flat_terrain(5, 5);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            2,
            2,
            7,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let entities = EntityStore::new();
        let mut q = base_query(&terrain, &path_grid);
        q.check_occupancy = true;
        q.occupancy = Some(&occupancy);
        q.entities = Some(&entities);

        let found = find_nearby_passable_cell((2, 2), &q, 0);
        // The seed (2,2) is occupied; FNPC must pick a different, free cell.
        assert!(found.is_some());
        assert_ne!(found, Some((2, 2)));
    }

    #[test]
    fn find_nearby_allow_bridge_filters_after_passability() {
        // A passable bridge cell at the seed is dropped when bridges are disallowed.
        let mut terrain = flat_terrain(3, 3);
        terrain.cells[1 * 3 + 1].bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL;
        let path_grid = PathGrid::from_resolved_terrain(&terrain);

        let mut q = base_query(&terrain, &path_grid);
        q.allow_bridge_cells = false;
        let found = find_nearby_passable_cell((1, 1), &q, 0);
        // Bridge seed filtered out; a non-bridge neighbour is chosen instead.
        assert!(found.is_some());
        assert_ne!(found, Some((1, 1)));

        // With bridges allowed, the seed is eligible again.
        q.allow_bridge_cells = true;
        assert!(find_nearby_passable_cell((1, 1), &q, 0).is_some());
    }

    #[test]
    fn find_nearby_no_candidate_returns_none() {
        // Every cell rejected -> no candidate -> None. WALL zone_type with a Normal
        // movement-zone rejects in speed_type_allows_cell (only the destroyer family
        // crosses walls), so every candidate fails passability.
        let mut terrain = flat_terrain(3, 3);
        for c in terrain.cells.iter_mut() {
            c.zone_type = zone_class::WALL;
        }
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let mut q = base_query(&terrain, &path_grid);
        q.radius_cap = 2;
        assert_eq!(find_nearby_passable_cell((1, 1), &q, 0), None);
    }

    #[test]
    fn find_nearby_passes_required_height_minus_one() {
        // FNPC always supplies required_height_or_level = -1 (None) regardless of any
        // caller height: a flat seed cell is found with no height gating applied.
        let terrain = flat_terrain(3, 3);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let q = base_query(&terrain, &path_grid);
        let found = find_nearby_passable_cell((1, 1), &q, 0);
        assert_eq!(found, Some((1, 1)));
    }

    #[test]
    fn find_nearby_selection_uses_frame_counter_modulo() {
        // No target: the chosen index is frame_counter % pool.len(), direct-preferred.
        // Walking the frame counter cycles deterministically through the direct pool.
        let terrain = flat_terrain(7, 7);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let q = base_query(&terrain, &path_grid);

        let directs: Vec<_> = collect_candidates((3, 3), &q)
            .into_iter()
            .filter(|c| c.direct)
            .collect();
        assert!(!directs.is_empty());

        for frame in 0..directs.len() as u32 * 2 {
            let expected = directs[(frame as usize) % directs.len()].cell;
            assert_eq!(
                find_nearby_passable_cell((3, 3), &q, frame),
                cell_to_u16(expected),
                "frame {frame} selection mismatch"
            );
        }
    }

    #[test]
    fn find_nearby_same_tick_aliasing() {
        // Two no-target calls on the same frame with the same candidate set return the
        // SAME cell (reproduce gamemd aliasing; do not spread).
        let terrain = flat_terrain(7, 7);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let q = base_query(&terrain, &path_grid);
        let a = find_nearby_passable_cell((3, 3), &q, 42);
        let b = find_nearby_passable_cell((3, 3), &q, 42);
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    #[test]
    fn find_nearby_selection_is_bit_identical_across_runs() {
        // Determinism guard (the hash-neutral, pre-authority-flip portion of the
        // parity harness): replaying the same FNPC query over the same grid yields a
        // bit-identical sequence of chosen cells across a frame sweep. When T7 flips
        // FNPC authority, this is the determinism property the replay/baseline harness
        // builds on; landing it now keeps the search itself replay-safe.
        let terrain = flat_terrain(11, 11);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let q = base_query(&terrain, &path_grid);
        let run = |seed: (i32, i32)| -> Vec<Option<(u16, u16)>> {
            (0..40u32)
                .map(|f| find_nearby_passable_cell(seed, &q, f))
                .collect()
        };
        assert_eq!(run((5, 5)), run((5, 5)));
    }

    #[test]
    fn find_nearby_target_selection_uses_nearest_distance() {
        // With a target, selection is nearest-Euclidean over the preferred pool, with
        // no frame-counter influence: the same target gives the same cell for any frame.
        let terrain = flat_terrain(9, 9);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let mut q = base_query(&terrain, &path_grid);
        q.target_cell = Some((7, 4)); // east of the seed (4,4)
        let pick0 = find_nearby_passable_cell((4, 4), &q, 0);
        let pick9 = find_nearby_passable_cell((4, 4), &q, 9);
        assert_eq!(pick0, pick9, "target selection must ignore the frame counter");
        // Nearest direct candidate toward (7,4) is on the +x axis from the seed.
        let pick = pick0.expect("a candidate exists");
        assert!(pick.0 >= 4, "nearest-to-target should lean toward the target");
    }
}
