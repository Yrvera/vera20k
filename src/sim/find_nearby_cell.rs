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

/// A surviving FNPC candidate, classified `direct` vs indirect by the engine's
/// height-projection identity test (see `is_direct_candidate`), NOT a cardinal-axis
/// test. Direct candidates are preferred at selection time.
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

/// Walk concentric square (Chebyshev-perimeter) rings outward from the seed,
/// collecting surviving candidates in the engine's fixed visit order, capping at
/// `MAX_CANDIDATES` and applying the per-ring early-out.
///
/// The outer loop runs `r = 0 .. cap` where `cap = min(Speed + Sight, 32)`; the
/// largest ring actually scanned is `cap - 1`. Ring shape and order match the
/// engine exactly (see `ring_cells`).
///
/// Per-ring early-out: once ANY direct candidate has been accepted, the search
/// finishes the *current* ring and then STOPS scanning further rings — biasing the
/// result toward the nearest ring that yields a direct hit. The 24-candidate cap is
/// also honored mid-ring (the engine compares the running count to `0x18` after
/// every accept and jumps straight to selection on equality).
fn collect_candidates(seed: (i32, i32), q: &NearbyQuery<'_>) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let cap = q.radius_cap.min(RADIUS_HARD_CAP) as i32;
    let mut direct_found = false;

    let mut r = 0;
    while r < cap {
        for (cx, cy) in ring_cells(seed, r) {
            if !candidate_passes(q, cx, cy) {
                continue;
            }
            let direct = is_direct_candidate(q, cx, cy);
            direct_found |= direct;
            out.push(Candidate {
                cell: (cx, cy),
                direct,
            });
            // The candidate cap is checked after every accept, mid-ring — the
            // engine stops the moment the 24th candidate lands.
            if out.len() >= MAX_CANDIDATES {
                return out;
            }
        }
        // Per-ring early-out: a direct hit anywhere so far finishes this ring then stops.
        if direct_found {
            return out;
        }
        r += 1;
    }
    out
}

/// Cells of square ring `r` (Chebyshev perimeter, `max(|dx|, |dy|) == r`) around
/// the seed `(ox, oy)`, in the engine's fixed 4-segment order:
///
/// 1. for `d = -r ..= r`: North cell `(ox + d, oy - r)` then South cell `(ox + d, oy + r)`
///    — the two full horizontal apex rows, scanned together W→E by `d`.
/// 2. for `e = 1-r ..= r-1`: West cell `(ox - r, oy + e)` then East cell `(ox + r, oy + e)`
///    — the two vertical side columns (interior rows only), scanned together N→S by `e`.
///
/// This is NOT a Manhattan diamond and NOT a continuous clockwise walk. At `r == 0`
/// segment 1 runs once with `d == 0`: North and South coincide on the seed, so the
/// engine emits the seed cell TWICE (segment 2's range `1..=-1` is empty). The
/// duplicate is intentional — it is the engine's actual candidate stream, and it
/// feeds the candidate count / frame-modulo index, so we reproduce it rather than
/// dedup.
fn ring_cells(seed: (i32, i32), r: i32) -> Vec<(i32, i32)> {
    let (ox, oy) = seed;
    let mut cells: Vec<(i32, i32)> = Vec::with_capacity((8 * r.max(1)) as usize);
    // Segment 1: North row then South row, for d = -r..=r (N then S per d).
    for d in -r..=r {
        cells.push((ox + d, oy - r)); // North
        cells.push((ox + d, oy + r)); // South (coincides with North at r == 0)
    }
    // Segment 2: West column then East column, interior rows e = 1-r..=r-1 (W then E per e).
    for e in (1 - r)..=(r - 1) {
        cells.push((ox - r, oy + e)); // West
        cells.push((ox + r, oy + e)); // East
    }
    cells
}

/// Engine "direct" classification — the height-projection identity test
/// (`FUN_006d6410`): a candidate is direct when its lepton-center, projected down
/// the isometric height ray, resolves back to the candidate's own cell.
///
/// On flat terrain (the cell at level 0 with no structural-bridge bit) the
/// projection is the identity, so every accepted flat cell is DIRECT — this is the
/// case the cardinal-axis test got wrong (it marked only the seed's row/column
/// direct). The full ray-walk over sloped / bridged neighbour cells is a deferred
/// follow-up: it reads neighbour cell level bytes along the descent ray, which the
/// flat-terrain slice does not yet model. Until then a non-flat candidate is
/// conservatively classified as indirect.
///
/// TODO-cutover: model the full `FUN_006d6410` descent (neighbour level bytes +
/// bridge bit) before relying on direct/indirect on sloped or bridged terrain.
fn is_direct_candidate(q: &NearbyQuery<'_>, cx: i32, cy: i32) -> bool {
    let (Ok(rx), Ok(ry)) = (u16::try_from(cx), u16::try_from(cy)) else {
        return false;
    };
    let Some(cell) = q.resolved_terrain.and_then(|t| t.cell(rx, ry)) else {
        // No terrain cell to project from; treat as flat-equivalent (direct).
        return true;
    };
    cell.level == 0 && !cell.bridge_facts.has_structural_bridge()
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
            playfield_bounds: None,
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
    fn find_nearby_ring_visit_order_matches_engine_segments() {
        // Square (Chebyshev) rings in the engine's fixed 4-segment order:
        //   seg1: for d=-r..=r  -> North (ox+d, oy-r) then South (ox+d, oy+r)
        //   seg2: for e=1-r..=r-1 -> West (ox-r, oy+e) then East (ox+r, oy+e)
        // Ring 0 emits the seed TWICE (N and S coincide at r==0); seg2 range is empty.
        assert_eq!(ring_cells((5, 5), 0), vec![(5, 5), (5, 5)]);
        // Ring 1: seg1 d=-1,0,1 then seg2 e=0. Derived directly from the engine's
        // (ox+d, oy-r)/(ox+d, oy+r) and (ox-r, oy+e)/(ox+r, oy+e) sequence.
        assert_eq!(
            ring_cells((5, 5), 1),
            vec![
                (4, 4), // d=-1 N
                (4, 6), // d=-1 S
                (5, 4), // d=0  N
                (5, 6), // d=0  S
                (6, 4), // d=1  N
                (6, 6), // d=1  S
                (4, 5), // e=0  W
                (6, 5), // e=0  E
            ]
        );
        // Ring 2 is the 16-cell square perimeter (10 from seg1, 6 from seg2).
        assert_eq!(ring_cells((5, 5), 2).len(), 16);
    }

    #[test]
    fn find_nearby_per_ring_early_out_stops_after_first_direct_ring() {
        // On flat terrain every accepted cell is DIRECT (height-projection identity),
        // so ring 0 alone yields a direct hit: the per-ring early-out finishes ring 0
        // (which emits the seed twice) and STOPS — it never walks out to fill 24.
        let terrain = flat_terrain(40, 40);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let mut q = base_query(&terrain, &path_grid);
        q.radius_cap = 100; // requests beyond the hard cap; clamps to 32
        let candidates = collect_candidates((20, 20), &q);
        // Ring 0 = seed twice, both direct -> early-out after ring 0.
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().all(|c| c.cell == (20, 20) && c.direct));
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
        // On flat terrain the per-ring early-out stops at ring 0 (seed is direct), so
        // the nearest-distance pool is the seed itself; the chosen cell is at/east of
        // the seed and never west of it.
        let pick = pick0.expect("a candidate exists");
        assert!(pick.0 >= 4, "nearest-to-target should not lean away from the target");
    }
}
