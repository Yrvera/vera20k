//! Nearby-passable-cell search (engine `Find_Nearby_Passable_Cell`).
//!
//! Square (Chebyshev) ring expansion around a seed cell — concentric perimeters,
//! NOT a Manhattan diamond (see `ring_cells`); per-candidate passability (plus an
//! optional occupancy check that always SKIPS reservations); frame-counter
//! selection when no target cell is given, nearest-distance selection when a
//! target is given. This is a read-only projection over the cell grids — it
//! mutates nothing and consumes no RNG stream, so threading it changes no hashed
//! state by itself. Depends only on `sim/` grids + `rules/` data; never on
//! render/ui/sidebar/audio/net.
//!
//! Determinism contract:
//! - The square-ring candidate ORDER is fully deterministic; it feeds both the
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

/// Hard radius cap for the ring search.
///
/// The engine derives its own cap from the two map-rectangle scalars —
/// `min(mapRectWidth + mapRectHeight, 32)` — and that sum exceeds 32 on any
/// playable map, so the effective cap IS this constant there. It is NOT
/// `Speed + Sight`: that reading is refuted, and a caller that derives its radius
/// from a unit's speed and sight abandons the search around 21 rings early.
pub const RADIUS_HARD_CAP: u16 = 32;
/// Candidate-pool early-terminate count — the search stops collecting once it has
/// this many surviving candidates.
pub const MAX_CANDIDATES: usize = 24;

/// Terrain-level delta the height gate admits, exclusive: a candidate passes when
/// `abs(seedLevel - bridgeRise - candidateLevel) < 2`. For an ordinary candidate
/// (`bridgeRise == 0`) that reads "at most one level from the seed"; for a bridge
/// candidate it does not — see [`candidate_height_ok`].
const MAX_SEED_LEVEL_DELTA_EXCLUSIVE: i16 = 2;
/// Levels a bridge deck sits above the ground cell that carries it. When the
/// candidate carries a bridge the height gate subtracts this from the SEED level —
/// not from the candidate's — so it does NOT normalize a deck to ground; the
/// arithmetic and what it actually admits are spelled out in [`candidate_height_ok`].
const BRIDGE_LEVEL_RISE: i16 = 4;

/// Diagonal steps south-east of a candidate that the direct/indirect occlusion test
/// probes. The engine's projection ray starts a fixed `0x600` leptons — six cells at
/// 256 leptons per cell — south-east of the candidate's own centre on BOTH axes, then
/// walks back up-screen to it, so a cell seven or more diagonal steps away can never
/// draw over the candidate. See [`is_direct_candidate`].
const OCCLUSION_PROBE_CELLS: i32 = 6;
/// Terrain levels of rise per diagonal step in the occlusion threshold `2q - 1`. One
/// terrain level lifts a cell's rendered diamond up-screen by half a cell diagonal, so
/// covering one more diagonal step of separation costs two levels of height.
const OCCLUSION_RISE_PER_STEP: i16 = 2;
/// Constant term of the occlusion threshold `2q - 1`: the immediate south-east
/// neighbour (`q == 1`) needs only a single level of rise to occlude.
const OCCLUSION_RISE_BIAS: i16 = -1;

/// The subset of the passability config the search always supplies the same way for
/// each 1x1 candidate: `required_height_or_level = -1` (None) is fixed by the search.
/// Overlay rejection is NOT fixed here — it is a per-call caller argument and lives
/// in [`NearbySearchOptions`].
#[derive(Debug, Clone, Copy)]
pub struct PassabilityArgs {
    pub speed_type: SpeedType,
    pub required_zone_id: Option<ZoneId>,
    pub movement_zone: MovementZone,
    pub bridge_aware_zone: bool,
}

/// Per-call arguments the CALLER varies between otherwise identical searches.
///
/// The engine's free-unit placement is the worked example: it runs the same search
/// twice, and the only argument that differs between the two calls is overlay
/// rejection — the first pass refuses any cell carrying an overlay (so a fresh unit
/// is not dropped onto the ore field), the second pass accepts one. Defaulting to
/// "accept" keeps every caller that does not name the option on today's behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NearbySearchOptions {
    /// Reject any candidate cell that carries an overlay (ore, gems, walls).
    pub reject_any_overlay: bool,
}

/// FNPC query — mirrors the engine `Find_Nearby_Passable_Cell` caller args.
pub struct NearbyQuery<'a> {
    /// Per-candidate passability config (built into a 1x1 passability rect).
    pub passability: PassabilityArgs,
    /// Bridge filter applied AFTER passability (an FNPC filter, not a passability arg).
    pub allow_bridge_cells: bool,
    /// Caller's terrain-level gate. When set, a candidate is admitted only while it
    /// stays within one level of the seed cell, with a four-level correction for a
    /// candidate that carries a bridge. Both free-unit placement attempts set it.
    pub check_height: bool,
    /// When set, each candidate also runs `check_occupancy_rect(.., reservation_arg = -1)`.
    pub check_occupancy: bool,
    /// Radius cap the caller supplies. The engine's own expression is
    /// `min(map-rect width + map-rect height, RADIUS_HARD_CAP)`, which is the cap
    /// itself on any playable map; this value is clamped to the cap internally.
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
    /// The map's isometric playfield diamond ([Map] Size width + LocalSize),
    /// threaded into the occupancy check's final playfield-corner test. When
    /// `None` the test falls back to the `map_size`/terrain rectangle.
    pub playfield_bounds: Option<crate::sim::cell_rect::PlayfieldBounds>,
}

/// A surviving FNPC candidate, classified `direct` vs indirect by the engine's
/// screen-occlusion test on the candidate's own south-east diagonal (see
/// [`is_direct_candidate`]) — NOT a cardinal-axis test, and NOT an absolute-height
/// test. Direct candidates are preferred at selection time.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    cell: (i32, i32),
    direct: bool,
}

/// Engine `Find_Nearby_Passable_Cell`.
///
/// `frame_counter` MUST be the sim per-tick counter (`Simulation.session.binary_frame`),
/// read as the current frame — never an RNG draw. Returns `None` for the
/// no-candidate case (engine null-cell `{0,0}`), which the caller interprets as
/// "no cell": clear the destination, retry next tick.
pub fn find_nearby_passable_cell(
    seed: (i32, i32),
    q: &NearbyQuery<'_>,
    frame_counter: u32,
) -> Option<(u16, u16)> {
    find_nearby_passable_cell_with_options(seed, q, NearbySearchOptions::default(), frame_counter)
}

/// Engine `Find_Nearby_Passable_Cell` with the caller's per-call options spelled
/// out. [`find_nearby_passable_cell`] is this with the default options.
pub fn find_nearby_passable_cell_with_options(
    seed: (i32, i32),
    q: &NearbyQuery<'_>,
    options: NearbySearchOptions,
    frame_counter: u32,
) -> Option<(u16, u16)> {
    let candidates = collect_candidates(seed, q, options);
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
/// The outer loop runs `r = 0 .. cap` where `cap = min(caller radius_cap,
/// [`RADIUS_HARD_CAP`])`; the largest ring actually scanned is `cap - 1`. The engine
/// derives its own cap from a map-rectangle sum capped at the same constant, and that
/// sum exceeds it on any playable map, so the cap IS [`RADIUS_HARD_CAP`] there. It is
/// NOT `Speed + Sight` — that reading is refuted. Ring shape and order match the
/// engine exactly (see `ring_cells`).
///
/// Per-ring early-out: once ANY direct candidate has been accepted, the search
/// finishes the *current* ring and then STOPS scanning further rings — biasing the
/// result toward the nearest ring that yields a direct hit. The 24-candidate cap is
/// also honored mid-ring (the engine compares the running count to the same 24 after
/// every accept and jumps straight to selection on equality).
///
/// Arming the early-out is ASYMMETRIC with the pool split: in the bridge-aware zone the
/// engine skips the occlusion projection entirely, so *any* accepted candidate arms the
/// early-out there, while selection still re-runs the projection on every stored
/// candidate to split the pools. That is why the flag and the stored classification are
/// computed separately below.
fn collect_candidates(
    seed: (i32, i32),
    q: &NearbyQuery<'_>,
    options: NearbySearchOptions,
) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let cap = q.radius_cap.min(RADIUS_HARD_CAP) as i32;
    let mut direct_found = false;
    // The height gate's reference level is read once, from the seed cell.
    let seed_level = q.check_height.then(|| cell_level(q, seed.0, seed.1));

    let mut r = 0;
    while r < cap {
        for (cx, cy) in ring_cells(seed, r) {
            if !candidate_passes(q, options, seed_level, cx, cy) {
                continue;
            }
            let direct = is_direct_candidate(q, cx, cy);
            // Bridge-aware searches skip the projection, so any accept arms the
            // early-out; `direct` still carries the projection for the pool split.
            direct_found |= q.passability.bridge_aware_zone || direct;
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

/// Engine "direct" classification — a screen-occlusion test on the candidate's own
/// south-east diagonal, measured RELATIVE to the candidate's own terrain level.
///
/// The engine projects the candidate's lepton centre down the isometric height ray: it
/// starts [`OCCLUSION_PROBE_CELLS`] cells south-east of that centre and walks back
/// up-screen toward it in small steps, asking at each cell whether that cell's
/// *rendered* diamond — its own diamond lifted up-screen by half a cell diagonal per
/// terrain level — already covers the candidate's centre. The candidate is DIRECT when
/// the walk resolves back to the candidate's own cell, i.e. when nothing south-east of
/// it draws over it.
///
/// Why the south-east diagonal specifically: cell `+X` renders down-right and `+Y`
/// down-left, so `+X +Y` is straight DOWN the screen. A cell south-east of the
/// candidate is drawn in FRONT of it, and its height lifts it up-screen across the
/// candidate. A cliff due east or due south is a different screen direction and does
/// not occlude, so only the exact diagonal is probed.
///
/// The two axes carry identical offsets, so the ray arithmetic collapses to one scalar
/// and yields a fixed threshold table — the minimum rise at diagonal step `q` that
/// makes the candidate INDIRECT:
///
/// | step `q`      | 1  | 2  | 3  | 4  | 5  | 6   | 7+          |
/// |---------------|----|----|----|----|----|-----|-------------|
/// | rise to occlude | +1 | +3 | +5 | +7 | +9 | +11 | never probed |
///
/// which is `OCCLUSION_RISE_PER_STEP * q + OCCLUSION_RISE_BIAS`.
///
/// Being relative rather than absolute is the whole point: on a uniformly raised
/// plateau every difference is 0, so every cell is direct exactly as at level 0, and a
/// candidate at level 7 under a level-7 neighbour is direct while the same candidate
/// under a level-8 neighbour is not. Off-grid probes read level 0, matching the engine's
/// out-of-range cell lookup, which returns a shared zeroed cell rather than failing.
///
/// The candidate's own bridge bit deliberately plays NO part here — it belongs to the
/// caller's height gate ([`candidate_height_ok`]) and to the bridge filter, and folding
/// it in was what made this port call every bridge cell indirect.
///
/// VERA-internal, gamemd equivalent UNCHECKED: the engine adds four levels to a *probe*
/// cell that carries a bridge, but only when the candidate cell carries a second,
/// unidentified cell flag. That flag's writer was not found, so the correction is
/// omitted rather than invented. Direction of the residual: gamemd classifies slightly
/// MORE cells indirect than we do near a bridge deck. Trigger: a candidate within six
/// diagonal steps north-west of a bridge deck that also carries the unidentified flag;
/// free-unit placement rejects bridge cells outright, so it is unreachable there.
fn is_direct_candidate(q: &NearbyQuery<'_>, cx: i32, cy: i32) -> bool {
    let base_level = cell_level(q, cx, cy);
    for step in 1..=OCCLUSION_PROBE_CELLS {
        let rise = cell_level(q, cx + step, cy + step).saturating_sub(base_level);
        let rise_that_occludes = OCCLUSION_RISE_PER_STEP * step as i16 + OCCLUSION_RISE_BIAS;
        if rise >= rise_that_occludes {
            return false;
        }
    }
    true
}

/// Run the per-candidate predicates in engine order: passability first (1x1 rect,
/// `required_height_or_level = -1`, caller-supplied overlay rejection), then the
/// optional occupancy check with reservations SKIPPED (`-1`), then the caller's
/// height gate, then the bridge filter last (a candidate that is a structural-bridge
/// cell is dropped unless bridges are allowed).
fn candidate_passes(
    q: &NearbyQuery<'_>,
    options: NearbySearchOptions,
    seed_level: Option<i16>,
    cx: i32,
    cy: i32,
) -> bool {
    let rect = CellRect::new(cx, cy, 1, 1);

    let passable = check_passability_rect(CellRectPassabilityContext {
        rect,
        speed_type: q.passability.speed_type,
        required_zone_id: q.passability.required_zone_id,
        movement_zone: q.passability.movement_zone,
        required_height_or_level: None, // the search always passes -1 (L21)
        bridge_aware_zone: q.passability.bridge_aware_zone,
        // Caller argument, forwarded verbatim: the engine's two free-unit attempts
        // differ in this one value and nothing else.
        reject_any_overlay: options.reject_any_overlay,
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
            playfield_bounds: q.playfield_bounds,
        })
    {
        return false;
    }

    if let Some(seed_level) = seed_level
        && !candidate_height_ok(q, seed_level, cx, cy)
    {
        return false;
    }

    // Bridge filter applied AFTER passability/occupancy/height.
    if !q.allow_bridge_cells && candidate_is_bridge_cell(q, cx, cy) {
        return false;
    }

    true
}

/// The caller's height gate: `abs(seedLevel - bridgeRise - candidateLevel) < 2`,
/// where `bridgeRise` is [`BRIDGE_LEVEL_RISE`] when the candidate carries a bridge
/// and 0 otherwise.
///
/// For an ordinary candidate that is "stay within one level of the seed". For a
/// bridge candidate the rise comes off the SEED side of the subtraction, so a deck
/// does NOT compare as ground: a bridge candidate sitting at the seed's own level
/// reads as four levels away and is REJECTED, and the only bridge candidates the
/// gate admits are those three to five levels BELOW the seed. Direction and signum
/// arithmetic is a recurring bug class here — this describes the subtraction as
/// written, which is the audited engine expression; do not "fix" it toward the
/// intent the name suggests.
///
/// At both free-unit placement callsites the bridge term never decides anything:
/// those callsites also set `allow_bridge_cells = false`, so a bridge candidate is
/// dropped by the bridge filter whatever this gate says.
///
/// RESIDUAL, recorded not fixed (out of scope for the direct/indirect work): the engine
/// also ADDS [`BRIDGE_LEVEL_RISE`] to the SEED level when the bridge-aware zone flag is
/// set and the seed cell itself carries a bridge; this port reads the seed level raw.
/// *Trigger:* a search whose seed is a bridge cell, run with `bridge_aware_zone` — today
/// only the movement reroute helper sets that flag, and only for a goal on a bridge
/// deck, so it fires when a unit is ordered onto an unreachable bridge cell. *Effect:*
/// a four-level shift in which candidates the gate admits around such a seed. Both
/// free-unit placement callsites clear the flag, so it is inert there.
fn candidate_height_ok(q: &NearbyQuery<'_>, seed_level: i16, cx: i32, cy: i32) -> bool {
    let bridge_rise = if candidate_is_bridge_cell(q, cx, cy) {
        BRIDGE_LEVEL_RISE
    } else {
        0
    };
    let delta = seed_level
        .saturating_sub(bridge_rise)
        .saturating_sub(cell_level(q, cx, cy));
    delta.abs() < MAX_SEED_LEVEL_DELTA_EXCLUSIVE
}

/// A cell's terrain level, read from the path grid first and the resolved terrain
/// second — the same order and the same "missing reads as 0" default the passability
/// facade uses for its own level term, so the two never disagree about a cell.
fn cell_level(q: &NearbyQuery<'_>, cx: i32, cy: i32) -> i16 {
    let (Ok(rx), Ok(ry)) = (u16::try_from(cx), u16::try_from(cy)) else {
        return 0;
    };
    q.path_grid
        .and_then(|grid| grid.cell(rx, ry))
        .map(|cell| cell.signed_level())
        .or_else(|| {
            q.resolved_terrain
                .and_then(|terrain| terrain.cell(rx, ry))
                .map(|cell| cell.level as i8 as i16)
        })
        .unwrap_or(0)
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

    /// A grid whose every cell sits at the same raised terrain level — a plateau. The
    /// occlusion test is relative, so this must behave exactly like `flat_terrain`.
    fn plateau_terrain(width: u16, height: u16, level: u8) -> ResolvedTerrainGrid {
        let mut terrain = flat_terrain(width, height);
        for cell in terrain.cells.iter_mut() {
            cell.level = level;
        }
        terrain
    }

    /// A grid that climbs one terrain level per south-east diagonal step, so EVERY cell
    /// is occluded by its immediate SE neighbour and no candidate is ever direct.
    fn diagonal_ramp_terrain(width: u16, height: u16) -> ResolvedTerrainGrid {
        let mut terrain = flat_terrain(width, height);
        for cell in terrain.cells.iter_mut() {
            cell.level = ((cell.rx as u32 + cell.ry as u32) / 2) as u8;
        }
        terrain
    }

    /// Classify one cell against a terrain grid, with a path grid built from it.
    fn direct_at(terrain: &ResolvedTerrainGrid, cell: (i32, i32)) -> bool {
        let path_grid = PathGrid::from_resolved_terrain(terrain);
        let q = base_query(terrain, &path_grid);
        is_direct_candidate(&q, cell.0, cell.1)
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
            playfield_bounds: None,
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
        // On flat terrain every accepted cell is DIRECT (nothing south-east of it rises
        // above it), so ring 0 alone yields a direct hit: the early-out finishes ring 0
        // (which emits the seed twice) and STOPS — it never walks out to fill 24.
        let terrain = flat_terrain(40, 40);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let mut q = base_query(&terrain, &path_grid);
        q.radius_cap = 100; // requests beyond the hard cap; clamps to 32
        let candidates = collect_candidates((20, 20), &q, NearbySearchOptions::default());
        // Ring 0 = seed twice, both direct -> early-out after ring 0.
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().all(|c| c.cell == (20, 20) && c.direct));
    }

    #[test]
    fn find_nearby_early_out_fires_on_a_raised_plateau_exactly_as_at_level_zero() {
        // THE BUG THIS CLOSES. The occlusion test is relative: on a uniformly raised
        // plateau every south-east difference is 0, so every accepted cell is DIRECT and
        // the per-ring early-out fires on ring 0 — byte-for-byte the level-0 result. The
        // absolute `level == 0` placeholder this replaced marked every cell above level 0
        // indirect, so the early-out never armed, collection ran on to MAX_CANDIDATES
        // across rings 1-3, and a refinery's free miner could land three cells out.
        for level in [0u8, 2, 7] {
            let terrain = plateau_terrain(40, 40, level);
            let path_grid = PathGrid::from_resolved_terrain(&terrain);
            let mut q = base_query(&terrain, &path_grid);
            q.radius_cap = 100; // requests beyond the hard cap; clamps to 32
            let candidates = collect_candidates((20, 20), &q, NearbySearchOptions::default());
            assert_eq!(
                candidates.len(),
                2,
                "level {level}: the early-out must stop after ring 0"
            );
            assert!(candidates.iter().all(|c| c.cell == (20, 20) && c.direct));
        }
    }

    #[test]
    fn find_nearby_direct_threshold_is_two_levels_per_diagonal_step_minus_one() {
        // A cell `q` diagonal steps SOUTH-EAST of the candidate occludes it — making the
        // candidate INDIRECT — once it rises `2q - 1` levels above it. One level below
        // that threshold leaves the candidate direct. Walked as a fixture per step
        // rather than asserted from the same formula the code uses.
        const GRID: usize = 20;
        for (step, threshold) in [(1i32, 1i16), (2, 3), (3, 5), (4, 7), (5, 9), (6, 11)] {
            for (rise, expect_direct) in [(threshold - 1, true), (threshold, false)] {
                let mut terrain = flat_terrain(GRID as u16, GRID as u16);
                let probe = 5 + step as usize;
                terrain.cells[probe * GRID + probe].level = rise as u8;
                assert_eq!(
                    direct_at(&terrain, (5, 5)),
                    expect_direct,
                    "step {step}: a rise of {rise} against a threshold of {threshold}"
                );
            }
        }
    }

    #[test]
    fn find_nearby_direct_probes_only_six_cells_down_the_south_east_diagonal() {
        // The ray starts a fixed six cells south-east and walks back, so step 7 is never
        // probed; and only the exact diagonal is probed, because a cliff due east or due
        // south is a different screen direction and cannot draw over the candidate.
        const GRID: usize = 20;
        let mut terrain = flat_terrain(GRID as u16, GRID as u16);
        terrain.cells[12 * GRID + 12].level = 13; // (12,12): seven diagonal steps out
        terrain.cells[5 * GRID + 6].level = 10; // (6,5): due EAST of the candidate
        terrain.cells[6 * GRID + 5].level = 10; // (5,6): due SOUTH of the candidate
        assert!(direct_at(&terrain, (5, 5)));
    }

    #[test]
    fn find_nearby_direct_reads_the_relative_rise_not_the_absolute_level() {
        const GRID: usize = 20;
        // Absolute height is irrelevant; only the south-east difference decides.
        let high_plateau = plateau_terrain(GRID as u16, GRID as u16, 7);
        assert!(
            direct_at(&high_plateau, (5, 5)),
            "a level-7 cell among level-7 cells is direct"
        );
        let mut stepped_up = plateau_terrain(GRID as u16, GRID as u16, 7);
        stepped_up.cells[6 * GRID + 6].level = 8;
        assert!(
            !direct_at(&stepped_up, (5, 5)),
            "the same cell under a single level of rise at step 1 is indirect"
        );

        // Ground falling away to the south-east can never occlude.
        let mut downhill = plateau_terrain(GRID as u16, GRID as u16, 5);
        for step in 1..=6usize {
            downhill.cells[(5 + step) * GRID + (5 + step)].level = 5u8.saturating_sub(step as u8);
        }
        assert!(direct_at(&downhill, (5, 5)));

        // At the map's south-east corner every probe leaves the grid and reads level 0,
        // matching the engine's out-of-range cell lookup (a shared zeroed cell, never
        // null), so a raised corner cell is still direct.
        let mut corner = flat_terrain(GRID as u16, GRID as u16);
        corner.cells[(GRID - 1) * GRID + (GRID - 1)].level = 3;
        assert!(direct_at(&corner, (GRID as i32 - 1, GRID as i32 - 1)));
    }

    #[test]
    fn find_nearby_direct_ignores_the_candidates_own_bridge_bit() {
        // The projection reads terrain LEVELS only. A candidate's own bridge bit is read
        // by the height gate and the bridge filter, never by this test — folding it in
        // is what made the placeholder call every bridge cell on flat ground indirect.
        let mut terrain = flat_terrain(20, 20);
        terrain.cells[5 * 20 + 5].bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL;
        assert!(direct_at(&terrain, (5, 5)));
    }

    #[test]
    fn find_nearby_slope_keeps_the_cell_under_the_rise_indirect() {
        // A genuine slope still splits the pools. Seed (5,5) is walled off so the search
        // reaches ring 1, and is itself raised two levels — which is exactly one diagonal
        // step south-east of ring-1's (4,4). So (4,4) is occluded and lands in the
        // indirect pool while every other ring-1 cell stays on flat ground and is direct,
        // and selection — which consults the indirect pool only when there are no directs
        // — can never return (4,4).
        const GRID: usize = 20;
        let mut terrain = flat_terrain(GRID as u16, GRID as u16);
        terrain.cells[5 * GRID + 5].zone_type = zone_class::WALL;
        terrain.cells[5 * GRID + 5].level = 2;
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let q = base_query(&terrain, &path_grid);

        let candidates = collect_candidates((5, 5), &q, NearbySearchOptions::default());
        let occluded = candidates
            .iter()
            .find(|c| c.cell == (4, 4))
            .expect("(4,4) is collected on ring 1");
        assert!(
            !occluded.direct,
            "(4,4) sits under a +2 rise one diagonal step south-east"
        );
        assert!(
            candidates
                .iter()
                .filter(|c| c.cell != (4, 4))
                .all(|c| c.direct),
            "every other ring-1 cell is on flat ground and stays direct"
        );
        for frame in 0..12u32 {
            assert_ne!(
                find_nearby_passable_cell((5, 5), &q, frame),
                Some((4, 4)),
                "frame {frame}: the direct pool is non-empty, so the occluded cell is never picked"
            );
        }
    }

    #[test]
    fn find_nearby_bridge_aware_zone_arms_the_early_out_without_the_projection() {
        // The engine skips the projection entirely in the bridge-aware zone, so ANY
        // accepted candidate arms the per-ring early-out there — while selection still
        // re-runs the projection to split the pools. On terrain climbing one level per
        // south-east step nothing is ever direct, so without the asymmetry the search
        // runs on to the 24-candidate cap.
        let terrain = diagonal_ramp_terrain(20, 20);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let mut q = base_query(&terrain, &path_grid);

        let unarmed = collect_candidates((8, 8), &q, NearbySearchOptions::default());
        assert_eq!(unarmed.len(), MAX_CANDIDATES);
        assert!(unarmed.iter().all(|c| !c.direct));

        q.passability.bridge_aware_zone = true;
        let armed = collect_candidates((8, 8), &q, NearbySearchOptions::default());
        assert_eq!(armed.len(), 2, "ring 0 alone arms the early-out");
        assert!(
            armed.iter().all(|c| !c.direct),
            "the stored classification still comes from the projection"
        );
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
    fn find_nearby_overlay_rejection_is_a_caller_argument() {
        // The engine's two free-unit attempts differ in exactly one argument: the
        // first rejects a cell carrying an overlay, the second accepts one. So the
        // same query over the same grid must return the ore seed with the default
        // options and refuse it with `reject_any_overlay`.
        const ORE_OVERLAY_ID: u8 = 102;
        let terrain = flat_terrain(5, 5);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let mut overlay = OverlayGrid::new(5, 5);
        overlay.place_overlay(2, 2, ORE_OVERLAY_ID, 0);
        let mut q = base_query(&terrain, &path_grid);
        q.overlay_grid = Some(&overlay);

        assert_eq!(
            find_nearby_passable_cell((2, 2), &q, 0),
            Some((2, 2)),
            "the overlay-allowed attempt keeps the ore cell"
        );
        let rejecting = NearbySearchOptions {
            reject_any_overlay: true,
        };
        let found = find_nearby_passable_cell_with_options((2, 2), &q, rejecting, 0)
            .expect("clear ground exists on the next ring");
        assert_ne!(
            found,
            (2, 2),
            "the overlay-rejecting attempt must walk off the ore cell"
        );
    }

    #[test]
    fn find_nearby_height_gate_drops_candidates_more_than_one_level_from_seed() {
        // `check_height` admits a candidate only while it stays within one level of
        // the seed. Seed (2,2) is walled off so the search reaches ring 1, where
        // (2,1) sits three levels up: it survives with the gate off and is dropped
        // with the gate on, while the level-1 cell at (1,2) survives either way.
        let mut terrain = flat_terrain(5, 5);
        terrain.cells[2 * 5 + 2].zone_type = zone_class::WALL; // seed rejected
        terrain.cells[1 * 5 + 2].level = 3; // (2,1): three levels above the seed
        terrain.cells[2 * 5 + 1].level = 1; // (1,2): one level above the seed
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let mut q = base_query(&terrain, &path_grid);

        let without_gate: Vec<(i32, i32)> =
            collect_candidates((2, 2), &q, NearbySearchOptions::default())
                .into_iter()
                .map(|c| c.cell)
                .collect();
        assert!(without_gate.contains(&(2, 1)));
        assert!(without_gate.contains(&(1, 2)));

        q.check_height = true;
        let with_gate: Vec<(i32, i32)> =
            collect_candidates((2, 2), &q, NearbySearchOptions::default())
                .into_iter()
                .map(|c| c.cell)
                .collect();
        assert!(
            !with_gate.contains(&(2, 1)),
            "a candidate three levels from the seed must be rejected"
        );
        assert!(
            with_gate.contains(&(1, 2)),
            "a candidate one level from the seed must survive"
        );
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

        let directs: Vec<_> = collect_candidates((3, 3), &q, NearbySearchOptions::default())
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
        assert_eq!(
            pick0, pick9,
            "target selection must ignore the frame counter"
        );
        // On flat terrain the per-ring early-out stops at ring 0 (seed is direct), so
        // the nearest-distance pool is the seed itself; the chosen cell is at/east of
        // the seed and never west of it.
        let pick = pick0.expect("a candidate exists");
        assert!(
            pick.0 >= 4,
            "nearest-to-target should not lean away from the target"
        );
    }

    /// Slice 3b wiring: with the map's playfield diamond threaded into the query,
    /// the occupancy check rejects rectangle-inside but diamond-outside cells, so
    /// FNPC can never pick an off-diamond border-filler cell. Without the bounds,
    /// the rectangle fallback accepts the same seed — the parity gap this closes.
    #[test]
    fn find_nearby_occupancy_rejects_off_diamond_cells_when_bounds_threaded() {
        use crate::sim::cell_rect::PlayfieldBounds;
        // Diamond (same fixture as the cell_rect acceptance tests): pass iff
        // 12 < x+y <= 26, x−y < 14, y−x < 6.
        let bounds = PlayfieldBounds {
            base: 10,
            off_fc: 2,
            off_100: 1,
            off_104: 10,
            off_108: 6,
        };
        let terrain = flat_terrain(30, 30);
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let mut q = base_query(&terrain, &path_grid);
        q.check_occupancy = true;
        q.map_size = None;

        // Without the diamond: the off-diamond seed (14,13) (sum 27) passes the
        // rectangle fallback and is returned directly.
        assert_eq!(
            find_nearby_passable_cell((14, 13), &q, 0),
            Some((14, 13)),
            "rectangle fallback accepts the off-diamond seed (the old behavior)"
        );

        // With the diamond: the seed and every other sum>26 ring cell are rejected;
        // the pool is exactly the three in-diamond ring-1 survivors in ring order
        // ((13,12), (14,12), (13,13)) and the frame counter walks that pool.
        q.playfield_bounds = Some(bounds);
        let in_diamond = |c: (u16, u16)| {
            let (x, y) = (c.0 as i32, c.1 as i32);
            12 < x + y && x + y <= 26 && x - y < 14 && y - x < 6
        };
        for frame in 0..6u32 {
            let pick = find_nearby_passable_cell((14, 13), &q, frame)
                .expect("in-diamond candidates exist on ring 1");
            assert!(
                in_diamond(pick),
                "FNPC picked off-diamond cell {pick:?} with bounds threaded"
            );
        }
        assert_eq!(
            find_nearby_passable_cell((14, 13), &q, 0),
            Some((13, 12)),
            "frame 0 picks the first in-diamond survivor in engine ring order"
        );
    }
}
