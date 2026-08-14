//! Simulation-owned skirmish start gathering and assignment.
//!
//! Active Battle startup resolves participant starts before terrain loading when
//! every authored start exists, then carries the same Scenario RNG cursor into
//! terrain Fill and the live world. Runtime/deficient-map resolution uses the
//! same algorithms against resolved terrain and live occupation.

use crate::map::map_file::MapFile;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::waypoints::Waypoint;
use crate::sim::rng::{SimRng, SimRngLogicalState};
use crate::sim::scenario_session::ScenarioDescriptor;
use crate::sim::world::Simulation;
use crate::skirmish_launch::{LaunchStartPosition, SkirmishLaunchSession};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub(crate) struct NativeStartBounds {
    pub(crate) min_rx: u16,
    pub(crate) min_ry: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl NativeStartBounds {
    pub(crate) fn from_session(sim: &Simulation, terrain: &ResolvedTerrainGrid) -> Self {
        if sim.session.local_width != 0 && sim.session.local_height != 0 {
            Self {
                min_rx: sim.session.local_left,
                min_ry: sim.session.local_top,
                width: sim.session.local_width,
                height: sim.session.local_height,
            }
        } else {
            Self {
                min_rx: 0,
                min_ry: 0,
                width: terrain.width(),
                height: terrain.height(),
            }
        }
    }

    fn max_rx(self) -> u16 {
        self.min_rx.saturating_add(self.width.saturating_sub(1))
    }

    fn max_ry(self) -> u16 {
        self.min_ry.saturating_add(self.height.saturating_sub(1))
    }

    pub(crate) fn clamp(self, rx: i32, ry: i32) -> (u16, u16) {
        (
            rx.clamp(i32::from(self.min_rx), i32::from(self.max_rx())) as u16,
            ry.clamp(i32::from(self.min_ry), i32::from(self.max_ry())) as u16,
        )
    }

    pub(crate) fn contains(self, rx: u16, ry: u16) -> bool {
        rx >= self.min_rx && rx <= self.max_rx() && ry >= self.min_ry && ry <= self.max_ry()
    }
}

/// Build the native multiplayer-start vector before assigning houses.
///
/// Only waypoint indices below the computed target count are examined. When
/// authored starts are deficient, each retry consumes the two asymmetric
/// ranged draws and runs the 8x8 nearby-passable search; invalid searches do
/// not advance the vector and have no artificial retry cap.
pub(crate) fn native_gather_start_positions(
    waypoints: &HashMap<u32, Waypoint>,
    participant_count: usize,
    terrain: &ResolvedTerrainGrid,
    occupancy: &crate::sim::occupancy::OccupancyGrid,
    bounds: NativeStartBounds,
    rng: &mut SimRng,
) -> Vec<Waypoint> {
    let authored_prefix = (0..8u32)
        .take_while(|index| waypoints.contains_key(index))
        .count();
    let target_count = authored_prefix.max(participant_count);
    let mut starts = Vec::with_capacity(target_count);

    for index in 0..target_count as u32 {
        if let Some(waypoint) = waypoints.get(&index) {
            starts.push(*waypoint);
        }
    }

    while starts.len() < target_count {
        let x_span = u32::from(bounds.width.saturating_sub(10));
        let y_high = u32::from(bounds.height.saturating_sub(10));
        let seed_rx = rng
            .next_range_u32_inclusive(0, x_span)
            .wrapping_add(u32::from(bounds.min_rx))
            .wrapping_add(10) as u16;
        let seed_ry = rng
            .next_range_u32_inclusive(10, y_high)
            .wrapping_add(u32::from(bounds.min_ry)) as u16;

        let Some((rx, ry)) = find_nearby_start_rect(terrain, occupancy, bounds, seed_rx, seed_ry)
        else {
            continue;
        };
        starts.push(Waypoint {
            index: starts.len() as u32,
            rx,
            ry,
        });
    }

    starts
}

/// Reduced active call-shape of FootClass::Find_Nearby_Passable_Cell used by
/// start gathering: scan square rings from the seed, finish the first ring
/// containing an 8x8 passable candidate, and select its first candidate at
/// frame zero.
fn find_nearby_start_rect(
    terrain: &ResolvedTerrainGrid,
    occupancy: &crate::sim::occupancy::OccupancyGrid,
    bounds: NativeStartBounds,
    seed_rx: u16,
    seed_ry: u16,
) -> Option<(u16, u16)> {
    for radius in 0..32i32 {
        let mut ring = Vec::new();
        let r = radius;
        for dx in -r..=r {
            ring.push((i32::from(seed_rx) + dx, i32::from(seed_ry) - r));
            if r != 0 {
                ring.push((i32::from(seed_rx) + dx, i32::from(seed_ry) + r));
            }
        }
        for dy in (1 - r)..r {
            ring.push((i32::from(seed_rx) - r, i32::from(seed_ry) + dy));
            if r != 0 {
                ring.push((i32::from(seed_rx) + r, i32::from(seed_ry) + dy));
            }
        }

        let mut accepted = Vec::new();
        for (rx, ry) in ring {
            if rx < i32::from(bounds.min_rx)
                || ry < i32::from(bounds.min_ry)
                || rx + i32::from(DEFICIENT_START_RECT_W) - 1 > i32::from(bounds.max_rx())
                || ry + i32::from(DEFICIENT_START_RECT_H) - 1 > i32::from(bounds.max_ry())
            {
                continue;
            }
            let (rx, ry) = (rx as u16, ry as u16);
            if deficient_start_rect_track_passable(terrain, occupancy, rx, ry) {
                accepted.push((rx, ry));
                if accepted.len() == 24 {
                    break;
                }
            }
        }
        if let Some(first) = accepted.first().copied() {
            return Some(first);
        }
    }
    None
}

/// Assign the gathered vector through standard Battle mode's `+0x84` callback.
/// Explicit starts populate a last-writer-wins table before the HouseClass
/// pass; every non-special House then honors its table entry or uses the
/// Battle selector's first-random/then-farthest rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeStartAssignment {
    pub(crate) placements: Vec<(usize, Waypoint)>,
    pub(crate) start_table: Vec<Option<usize>>,
}

const NATIVE_MULTIPLAYER_START_LIMIT: usize = 8;
pub(crate) const HOUSE_CONSTRUCTOR_TIMER_MIN: u32 = 450;
pub(crate) const HOUSE_CONSTRUCTOR_TIMER_MAX: u32 = 1800;

/// Immutable standard-Battle state prepared before the first loading frame.
///
/// Complete authored start vectors need no terrain-dependent fallback, so the
/// frontend can reproduce the native pre-render constructor and assignment RNG
/// prefix once. The resulting table is then shared by loading composition and
/// gameplay initialization.
#[derive(Debug, Clone)]
pub(crate) struct PreloadedBattleStartPlan {
    gathered_starts: Vec<Waypoint>,
    assignment: NativeStartAssignment,
    scenario_rng_before: SimRngLogicalState,
    scenario_rng_before_fingerprint: u64,
    scenario_rng_after: SimRngLogicalState,
    scenario_rng_after_cursor: SimRng,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PreloadedBattleStartPlanError {
    #[error(
        "preloaded Battle plan expected Scenario RNG fingerprint {expected:#018x}, got {actual:#018x}"
    )]
    ScenarioRngPrestateMismatch { expected: u64, actual: u64 },
}

impl PreloadedBattleStartPlan {
    pub(crate) fn gathered_starts(&self) -> &[Waypoint] {
        &self.gathered_starts
    }

    pub(crate) fn start_table(&self) -> &[Option<usize>] {
        &self.assignment.start_table
    }

    pub(crate) fn assignment(&self) -> &NativeStartAssignment {
        &self.assignment
    }

    /// Validate and transfer the one pre-loading RNG prefix to the stream that
    /// later terrain Fill and Simulation construction will continue.
    fn install_before_terrain(
        &self,
        scenario_rng: &mut SimRng,
    ) -> Result<(), PreloadedBattleStartPlanError> {
        if scenario_rng.logical_state() != self.scenario_rng_before {
            return Err(PreloadedBattleStartPlanError::ScenarioRngPrestateMismatch {
                expected: self.scenario_rng_before_fingerprint,
                actual: scenario_rng.state(),
            });
        }
        *scenario_rng = self.scenario_rng_after_cursor.clone();
        debug_assert_eq!(scenario_rng.logical_state(), self.scenario_rng_after);
        Ok(())
    }
}

/// Prepare the terrain-independent stock Battle/FFA prefix exactly once.
///
/// Sparse or deficient authored starts deliberately return `None`: their two
/// Gather passes depend on resolved terrain and must stay runtime-owned.
/// Random maps are eligible once their retained GeneratedMap exists: gamemd's
/// Full_Init 0x00686B20 assigns the retained RmgRegion start waypoints before
/// DrawLoadingScreen 0x00552D60 consumes that same table.
/// FFA provenance: constructor 0x005C5CE0 installs vtable 0x007EE424, whose
/// +0x80 (0x005D6BE0), +0x84 (0x005D6C70), and +0xC4 (0x005D6890) callbacks
/// are byte-identical to Battle's active start-assignment callbacks. Full_Init
/// 0x00686B20 calls +0x80 and ordinarily +0x84 for offline g_GameMode 5 before
/// DrawLoadingScreen.
pub(crate) fn preload_standard_battle_start_plan(
    session: &SkirmishLaunchSession,
    map_data: &MapFile,
    launch_seed: u32,
) -> Option<PreloadedBattleStartPlan> {
    if !has_verified_preload_start_callbacks(session) {
        return None;
    }
    let selected_map = session.selected_map_file.as_deref()?;
    if selected_map.trim() != selected_map
        || selected_map.is_empty()
        || selected_map.eq_ignore_ascii_case("auto")
    {
        return None;
    }

    let participant_count = 1usize.checked_add(session.opponents.len())?;
    let authored_prefix = (0..NATIVE_MULTIPLAYER_START_LIMIT)
        .take_while(|index| map_data.waypoints.contains_key(&(*index as u32)))
        .count();
    let authored_count = (0..NATIVE_MULTIPLAYER_START_LIMIT)
        .filter(|index| map_data.waypoints.contains_key(&(*index as u32)))
        .count();
    if authored_prefix != authored_count || authored_prefix < participant_count {
        return None;
    }

    let gathered_starts: Vec<Waypoint> = (0..authored_prefix)
        .filter_map(|index| map_data.waypoints.get(&(index as u32)).copied())
        .collect();
    let mut scenario_rng = SimRng::new(u64::from(launch_seed));
    let scenario_rng_before = scenario_rng.logical_state();
    let scenario_rng_before_fingerprint = scenario_rng.state();

    // HouseClass construction precedes both Battle/FFA callbacks. Every generated
    // participant plus Neutral and Special consumes one rejection-capable
    // RandomRanged(450,1800), in HouseClass order.
    for _ in 0..participant_count + 2 {
        let _ = scenario_rng
            .next_range_u32_inclusive(HOUSE_CONSTRUCTOR_TIMER_MIN, HOUSE_CONSTRUCTOR_TIMER_MAX);
    }

    // Battle and FFA +0x80/+0x84 each Gather. With a complete contiguous
    // authored vector both calls return this same data and consume no RNG.
    let assignment = native_assign_launch_starts(session, &gathered_starts, &mut scenario_rng);
    let scenario_rng_after = scenario_rng.logical_state();

    Some(PreloadedBattleStartPlan {
        gathered_starts,
        assignment,
        scenario_rng_before,
        scenario_rng_before_fingerprint,
        scenario_rng_after,
        scenario_rng_after_cursor: scenario_rng,
    })
}

fn has_verified_preload_start_callbacks(session: &SkirmishLaunchSession) -> bool {
    let mode = &session.mode;
    if !mode.map_filter.eq_ignore_ascii_case("standard")
        || !mode.random_maps_allowed
        || mode.must_ally
    {
        return false;
    }
    match mode.id {
        1 => {
            mode.ui_name_key.eq_ignore_ascii_case("GUI:Battle")
                && mode.tooltip_key.eq_ignore_ascii_case("STT:ModeBattle")
                && mode.override_file.eq_ignore_ascii_case("MPBattleMD.ini")
                && mode.allies_allowed
        }
        2 => {
            mode.ui_name_key.eq_ignore_ascii_case("GUI:FreeForAll")
                && mode.tooltip_key.eq_ignore_ascii_case("STT:ModeFreeForAll")
                && mode
                    .override_file
                    .eq_ignore_ascii_case("MPFreeForAllMD.ini")
                && !mode.allies_allowed
        }
        _ => false,
    }
}

pub(crate) fn native_assign_launch_starts(
    session: &SkirmishLaunchSession,
    starts: &[Waypoint],
    rng: &mut SimRng,
) -> NativeStartAssignment {
    if starts.is_empty() {
        return NativeStartAssignment {
            placements: Vec::new(),
            start_table: Vec::new(),
        };
    }

    let requested: Vec<LaunchStartPosition> = std::iter::once(session.local.start_position)
        .chain(
            session
                .opponents
                .iter()
                .map(|opponent| opponent.start_position),
        )
        .collect();
    let mut explicit_owner = vec![None; starts.len()];
    for (slot, request) in requested.iter().enumerate() {
        let LaunchStartPosition::Position(index) = request else {
            continue;
        };
        if let Some(owner) = explicit_owner.get_mut(usize::from(*index)) {
            *owner = Some(slot);
        }
    }

    // Battle +0x84 builds its occupied-byte array from the complete
    // Scenario+0x1180 table before its HouseClass pass begins. The selected
    // mode's +0x80 callback populated that table in HouseClass order, so
    // duplicate explicit starts have already resolved last-writer-wins.
    let mut occupied: Vec<bool> = explicit_owner.iter().map(Option::is_some).collect();
    let mut assigned = vec![None; requested.len()];

    // Unlike generic AssignStartingPoints, standard Battle walks every
    // non-special House once and honors table ownership for AI houses too.
    for slot in 0..requested.len() {
        let explicit = explicit_owner
            .iter()
            .rposition(|owner| *owner == Some(slot));
        let start_index =
            explicit.unwrap_or_else(|| choose_battle_automatic_start(starts, &occupied, rng));
        occupied[start_index] = true;
        explicit_owner[start_index] = Some(slot);
        assigned[slot] = Some(starts[start_index]);
    }

    NativeStartAssignment {
        placements: assigned
            .into_iter()
            .enumerate()
            .filter_map(|(slot, start)| start.map(|start| (slot, start)))
            .collect(),
        start_table: explicit_owner,
    }
}

/// Cooperative's custom start callback partitions authored positions into a
/// human prefix and an AI suffix. The explicit Scenario start table reserves
/// all of its entries before HouseClass iteration; automatic human placement
/// draws within the prefix and probes forward, while the remaining houses
/// take the first free suffix entry without a draw.
pub(crate) fn native_assign_cooperative_launch_starts(
    session: &SkirmishLaunchSession,
    starts: &[Waypoint],
    human_start_spots: usize,
    rng: &mut SimRng,
) -> NativeStartAssignment {
    if starts.is_empty() {
        return NativeStartAssignment {
            placements: Vec::new(),
            start_table: Vec::new(),
        };
    }

    let requested: Vec<LaunchStartPosition> = std::iter::once(session.local.start_position)
        .chain(
            session
                .opponents
                .iter()
                .map(|opponent| opponent.start_position),
        )
        .collect();
    let mut explicit_owner = vec![None; starts.len()];
    for (slot, request) in requested.iter().enumerate() {
        let LaunchStartPosition::Position(index) = request else {
            continue;
        };
        if let Some(owner) = explicit_owner.get_mut(usize::from(*index)) {
            *owner = Some(slot);
        }
    }

    let mut occupied: Vec<bool> = explicit_owner.iter().map(Option::is_some).collect();
    let human_house_count = 1usize;
    let human_start_spots = human_start_spots.min(starts.len());
    let mut assigned = vec![None; requested.len()];

    for slot in 0..requested.len() {
        let explicit = explicit_owner.iter().position(|owner| *owner == Some(slot));
        let start_index = if let Some(explicit) = explicit {
            explicit
        } else {
            let occupied_count = occupied.iter().filter(|used| **used).count();
            if occupied_count < human_house_count && human_start_spots != 0 {
                let mut candidate =
                    rng.next_range_u32_inclusive(0, human_start_spots as u32 - 1) as usize;
                while occupied[candidate] {
                    candidate = (candidate + 1) % human_start_spots;
                }
                candidate
            } else {
                (human_start_spots..starts.len())
                    .find(|candidate| !occupied[*candidate])
                    .or_else(|| occupied.iter().position(|used| !*used))
                    .expect("participant count cannot exceed gathered starts")
            }
        };
        occupied[start_index] = true;
        explicit_owner[start_index] = Some(slot);
        assigned[slot] = Some(starts[start_index]);
    }

    NativeStartAssignment {
        placements: assigned
            .into_iter()
            .enumerate()
            .filter_map(|(slot, start)| start.map(|start| (slot, start)))
            .collect(),
        start_table: explicit_owner,
    }
}

pub(crate) fn choose_battle_automatic_start(
    starts: &[Waypoint],
    occupied: &[bool],
    rng: &mut SimRng,
) -> usize {
    let used_count = occupied.iter().filter(|used| **used).count();
    if used_count == 0 {
        return rng.next_range_u32_inclusive(0, starts.len() as u32 - 1) as usize;
    }

    let free: Vec<usize> = occupied
        .iter()
        .enumerate()
        .filter_map(|(index, used)| (!*used).then_some(index))
        .collect();
    let distance_sum = |candidate: usize| -> i32 {
        occupied
            .iter()
            .enumerate()
            .filter(|(_, used)| **used)
            .map(|(other, _)| native_start_distance(starts[candidate], starts[other]))
            .sum()
    };
    let mut iter = free.into_iter();
    let mut selected = iter
        .next()
        .expect("participant count cannot exceed gathered starts");
    let mut selected_sum = distance_sum(selected);
    for candidate in iter {
        let sum = distance_sum(candidate);
        if sum > selected_sum {
            selected = candidate;
            selected_sum = sum;
        }
    }
    selected
}

pub(crate) fn native_start_distance(left: Waypoint, right: Waypoint) -> i32 {
    let dx = i32::from((left.rx as i16).wrapping_sub(right.rx as i16));
    let dy = i32::from((left.ry as i16).wrapping_sub(right.ry as i16));
    crate::map::rmg::x87::approx_sqrt_f32(dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy)))
        as i32
}

const DEFICIENT_START_RECT_W: u16 = 8;
const DEFICIENT_START_RECT_H: u16 = 8;

pub(crate) fn deficient_start_rect_track_passable(
    terrain: &ResolvedTerrainGrid,
    occupancy: &crate::sim::occupancy::OccupancyGrid,
    rx: u16,
    ry: u16,
) -> bool {
    crate::sim::cell_rect::check_passability_rect(
        crate::sim::cell_rect::CellRectPassabilityContext {
            rect: crate::sim::cell_rect::CellRect::new(
                i32::from(rx),
                i32::from(ry),
                i32::from(DEFICIENT_START_RECT_W),
                i32::from(DEFICIENT_START_RECT_H),
            ),
            speed_type: crate::rules::locomotor_type::SpeedType::Track,
            required_zone_id: None,
            movement_zone: crate::rules::locomotor_type::MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: false,
            reject_any_overlay: false,
            path_grid: None,
            resolved_terrain: Some(terrain),
            overlay_grid: None,
            occupancy: Some(occupancy),
            zone_grid: None,
        },
    )
}

/// Opaque owner for the two gameplay cursors used while a map becomes a world.
///
/// The app may drive content-loading algorithms through the two narrow draw
/// wrappers, but it cannot replace or extract either cursor. Consuming this
/// owner is the only production handoff into a freshly constructed Simulation.
pub(crate) struct ScenarioBootstrapRng {
    scenario: SimRng,
    main: SimRng,
}

/// Scenario-stream access granted only to the terrain Fill callback.
pub(crate) struct ScenarioFillRng<'a> {
    rng: &'a mut SimRng,
}

impl ScenarioFillRng<'_> {
    pub(crate) fn next_range_u32_inclusive(&mut self, low: u32, high: u32) -> u32 {
        self.rng.next_range_u32_inclusive(low, high)
    }
}

/// Main-stream access granted only to one-time TMP variant-table generation.
pub(crate) struct VariantMainRng<'a> {
    rng: &'a mut SimRng,
}

impl VariantMainRng<'_> {
    pub(crate) fn next_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }
}

impl ScenarioBootstrapRng {
    pub(crate) fn new(seed: u32) -> Self {
        let seed = u64::from(seed);
        Self {
            scenario: SimRng::new(seed),
            main: SimRng::new(seed),
        }
    }

    /// Install the already-resolved pre-render Battle prefix exactly once.
    pub(crate) fn install_preloaded_battle_plan(
        &mut self,
        plan: &PreloadedBattleStartPlan,
    ) -> Result<(), PreloadedBattleStartPlanError> {
        plan.install_before_terrain(&mut self.scenario)
    }

    /// Borrow the two independent load-time consumers without exposing either
    /// raw cursor or allowing one callback to draw from the other stream.
    pub(crate) fn terrain_draws(&mut self) -> (ScenarioFillRng<'_>, VariantMainRng<'_>) {
        (
            ScenarioFillRng {
                rng: &mut self.scenario,
            },
            VariantMainRng {
                rng: &mut self.main,
            },
        )
    }

    /// Finish the app-to-sim construction handoff with both exact cursors.
    pub(crate) fn into_simulation(self, descriptor: &ScenarioDescriptor) -> Simulation {
        let mut sim = Simulation::from_descriptor(descriptor);
        sim.install_terrain_load_advanced_scenario_rng(self.scenario);
        sim.install_variant_advanced_main_rng(self.main);
        sim
    }
}

impl Simulation {
    pub(crate) fn gather_native_start_positions(
        &mut self,
        waypoints: &HashMap<u32, Waypoint>,
        participant_count: usize,
        terrain: &ResolvedTerrainGrid,
        bounds: NativeStartBounds,
    ) -> Vec<Waypoint> {
        native_gather_start_positions(
            waypoints,
            participant_count,
            terrain,
            &self.substrate.occupancy,
            bounds,
            &mut self.scenario_rng,
        )
    }

    pub(crate) fn assign_native_battle_starts(
        &mut self,
        session: &SkirmishLaunchSession,
        starts: &[Waypoint],
    ) -> NativeStartAssignment {
        native_assign_launch_starts(session, starts, &mut self.scenario_rng)
    }

    pub(crate) fn assign_native_cooperative_starts(
        &mut self,
        session: &SkirmishLaunchSession,
        starts: &[Waypoint],
        human_start_spots: usize,
    ) -> NativeStartAssignment {
        native_assign_cooperative_launch_starts(
            session,
            starts,
            human_start_spots,
            &mut self.scenario_rng,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(seed: u32) -> ScenarioDescriptor {
        ScenarioDescriptor {
            seed,
            ..ScenarioDescriptor::default()
        }
    }

    #[test]
    fn untouched_bootstrap_cursors_match_fresh_descriptor_construction() {
        let seed = 0x51C0_1001;
        let expected = Simulation::from_descriptor(&descriptor(seed)).rng_state();
        let actual = ScenarioBootstrapRng::new(seed)
            .into_simulation(&descriptor(seed))
            .rng_state();

        assert_eq!(actual, expected);
    }

    #[test]
    fn main_variant_draws_do_not_advance_scenario_cursor() {
        let seed = 0x51C0_1002;
        let mut owner = ScenarioBootstrapRng::new(seed);
        {
            let (scenario, mut main) = owner.terrain_draws();
            let _ = main.next_u32();
            drop(scenario);
        }
        let actual = owner.into_simulation(&descriptor(seed)).rng_state();
        let mut expected_main = SimRng::new(u64::from(seed));
        let _ = expected_main.next_u32();

        assert_eq!(
            actual.scenario,
            SimRng::new(u64::from(seed)).logical_state()
        );
        assert_eq!(actual.main, expected_main.logical_state());
    }

    #[test]
    fn terrain_scenario_draws_transfer_without_advancing_main() {
        let seed = 0x51C0_1003;
        let mut owner = ScenarioBootstrapRng::new(seed);
        {
            let (mut scenario, main) = owner.terrain_draws();
            let _ = scenario.next_range_u32_inclusive(5, 17);
            drop(main);
        }
        let actual = owner.into_simulation(&descriptor(seed)).rng_state();
        let mut expected_scenario = SimRng::new(u64::from(seed));
        let _ = expected_scenario.next_range_u32_inclusive(5, 17);

        assert_eq!(actual.scenario, expected_scenario.logical_state());
        assert_eq!(actual.main, SimRng::new(u64::from(seed)).logical_state());
    }

    #[test]
    fn preloaded_battle_prefix_transfers_once_before_terrain() {
        let seed = 0x51C0_1004;
        let before = SimRng::new(u64::from(seed));
        let mut after = before.clone();
        let _ = after
            .next_range_u32_inclusive(HOUSE_CONSTRUCTOR_TIMER_MIN, HOUSE_CONSTRUCTOR_TIMER_MAX);
        let plan = PreloadedBattleStartPlan {
            gathered_starts: Vec::new(),
            assignment: NativeStartAssignment {
                placements: Vec::new(),
                start_table: Vec::new(),
            },
            scenario_rng_before: before.logical_state(),
            scenario_rng_before_fingerprint: before.state(),
            scenario_rng_after: after.logical_state(),
            scenario_rng_after_cursor: after.clone(),
        };
        let mut owner = ScenarioBootstrapRng::new(seed);

        owner
            .install_preloaded_battle_plan(&plan)
            .expect("fresh bootstrap owner accepts the plan prefix");
        assert!(
            owner.install_preloaded_battle_plan(&plan).is_err(),
            "the same constructor/assignment prefix cannot be installed twice"
        );
        let actual = owner.into_simulation(&descriptor(seed)).rng_state();

        assert_eq!(actual.scenario, after.logical_state());
        assert_eq!(actual.main, before.logical_state());
    }
}
