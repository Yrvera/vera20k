//! Simulation-owned offline-skirmish world bootstrap.
//!
//! Active Battle startup resolves participant starts before terrain loading when
//! every authored start exists, then carries the same Scenario RNG cursor into
//! terrain Fill and the live world. House construction, runtime/deficient-map
//! start assignment, opening forces, shroud, AI credits, and alliances remain
//! behind the same simulation authority boundary.

use std::collections::{BTreeMap, HashMap};

use crate::map::entities::EntityCategory;
use crate::map::houses::HouseRoster;
use crate::map::map_file::MapFile;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::waypoints::Waypoint;
use crate::rng_continuation::MapGenRngContinuation;
use crate::rules::ruleset::RuleSet;
use crate::sim::ai::AiPlayerState;
use crate::sim::cell_rect::{PlayfieldBounds, cell_is_in_playfield};
use crate::sim::house_state::{HouseDifficulty, HouseState, determine_waypoint_edge};
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::rng::{SimRng, SimRngLogicalState};
use crate::sim::scenario_session::ScenarioDescriptor;
use crate::sim::world::Simulation;
use crate::skirmish_launch::{
    LaunchCountry, LaunchStartPosition, LaunchTeam, SkirmishLaunchSession,
};
use crate::util::native_x87::{X87Chop53, sqrt_approx_f32};

#[derive(Debug, Clone, Copy)]
pub(crate) struct NativeStartBounds {
    pub(crate) min_rx: u16,
    pub(crate) min_ry: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl NativeStartBounds {
    /// gamemd-derived: active YR start placement is bounded by the MapClass
    /// CELL-ARRAY rect, never by `LocalSize=`. `MapClass::Resize @ 0x00565C10` writes
    /// `MapClass+0x124..0x130` as `(1, 1, SizeW+SizeH-1, SizeW+SizeH-1)`, and
    /// both `ScenarioClass__Gather_Start_Positions @ 0x00688380` (deficient
    /// seed block `0x00688528..0x0068857C`) and
    /// `Try_Unlimbo_Object_At_Or_Near_Cell @ 0x00688ED0` (fallback probe clamp)
    /// read exactly those four fields. Playable-area
    /// acceptance is the separate isometric-diamond predicate
    /// (`cell_rect::cell_is_in_playfield`); `LocalSize=` feeds that test's
    /// band constants only, never an axis-aligned bound. Treating LocalSize
    /// as a cell rectangle here previously rejected most authored starts and
    /// clamped every displaced MCV onto the false edge.
    pub(crate) fn from_session(sim: &Simulation, terrain: &ResolvedTerrainGrid) -> Self {
        // `session.map_width/height` hold the canonical cell-array extent
        // (~SizeW+SizeH), so the native rect side is that extent minus one.
        // Sessions without header data (headless fixtures) fall back to the
        // terrain grid extent — VERA-internal, same shape.
        let (extent_x, extent_y) = if sim.session.map_width != 0 && sim.session.map_height != 0 {
            (sim.session.map_width, sim.session.map_height)
        } else {
            (terrain.width(), terrain.height())
        };
        Self {
            min_rx: 1,
            min_ry: 1,
            width: extent_x.saturating_sub(1),
            height: extent_y.saturating_sub(1),
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
    playfield_bounds: Option<PlayfieldBounds>,
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
        // gamemd-derived: active YR `ScenarioClass__Gather_Start_Positions
        // @ 0x00688380`, seed block `0x00688528..0x0068857C`: the first draw
        // `RandomRanged(0, rect.h - 10)` lands on the Y axis
        // (`+ 10 + rect.y`), the second `RandomRanged(10, rect.w - 10)` on the
        // X axis (`+ rect.x`). The rect is the MapClass cell-array rect (see
        // `NativeStartBounds::from_session`), which is square, so a transposed
        // mapping is RNG-neutral — but the seeded POINT mirrors across the
        // diagonal, so the axes are kept exactly as native writes them.
        let y_span = u32::from(bounds.height.saturating_sub(10));
        let x_high = u32::from(bounds.width.saturating_sub(10));
        let seed_ry = rng
            .next_range_u32_inclusive(0, y_span)
            .wrapping_add(10)
            .wrapping_add(u32::from(bounds.min_ry)) as u16;
        let seed_rx = rng
            .next_range_u32_inclusive(10, x_high)
            .wrapping_add(u32::from(bounds.min_rx)) as u16;

        let Some((rx, ry)) = find_nearby_start_rect(
            terrain,
            occupancy,
            bounds,
            playfield_bounds,
            seed_rx,
            seed_ry,
        ) else {
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
pub(crate) fn find_nearby_start_rect(
    terrain: &ResolvedTerrainGrid,
    occupancy: &crate::sim::occupancy::OccupancyGrid,
    bounds: NativeStartBounds,
    playfield_bounds: Option<PlayfieldBounds>,
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
            // gamemd-derived: active YR `FootClass__Find_Nearby_Passable_Cell
            // @ 0x0056DC20` applies `MapClass__Is_Cell_In_Playfield_CellClass
            // @ 0x00578540` to the candidate anchor before
            // `CellRect__CheckPassability @ 0x0056E7C0` scans its 8x8 rect.
            // Only the anchor is diamond-gated; requiring all 64 cells to lie in
            // the diamond would be stricter than retail.
            if !cell_is_in_playfield(
                (rx, ry),
                playfield_bounds,
                Some(terrain),
                Some((terrain.width(), terrain.height())),
            ) {
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
    descriptor: &MatchLaunchDescriptor,
    map_data: &MapFile,
    launch_seed: u32,
) -> Option<PreloadedBattleStartPlan> {
    let session = descriptor.session();
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
    let squared_distance = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
    let root = sqrt_approx_f32(X87Chop53::load_i32(squared_distance))
        .expect("signed squared start distance stays in the verified finite x87 domain");
    f32::from_bits(root.bits()) as i32
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkirmishLaunchApplyResult {
    pub(crate) local_owner: Option<String>,
    pub(crate) spawned_mcvs: u32,
    pub(crate) active_slots: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedSkirmishSlot {
    pub(crate) owner_name: String,
    pub(crate) country: LaunchCountry,
    pub(crate) color_index: u8,
    pub(crate) start_position: LaunchStartPosition,
    pub(crate) team: LaunchTeam,
    pub(crate) is_human: bool,
    pub(crate) difficulty: HouseDifficulty,
}

/// Sim-owned behavioral launch descriptor (F09).
///
/// Wraps a `SkirmishLaunchSession` whose shell-random choices — country and
/// color, for the local slot and every AI slot — are proven resolved. The
/// validating constructor is the only way in, so sim entry points cannot
/// receive an unresolved frontend session and silently launch with the
/// placeholder country/color a random slot still carries. Start positions
/// remain potentially random by design: gamemd assigns them at scenario load
/// with the gameplay Scenario RNG (see `assign_native_battle_starts`), not in
/// the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchLaunchDescriptor {
    session: SkirmishLaunchSession,
}

/// A launch slot still carried an unresolved random shell choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedShellChoice {
    /// `None` = the local player slot; `Some(i)` = AI opponent index `i`.
    pub ai_slot: Option<usize>,
    /// Which choice was left random: `"country"` or `"color"`.
    pub choice: &'static str,
}

impl std::fmt::Display for UnresolvedShellChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ai_slot {
            None => write!(f, "local slot still has a random {}", self.choice),
            Some(index) => write!(f, "AI slot {index} still has a random {}", self.choice),
        }
    }
}

impl MatchLaunchDescriptor {
    /// Validate that the app's shell close transaction resolved every
    /// random choice before the session crosses into sim.
    pub fn from_resolved(session: SkirmishLaunchSession) -> Result<Self, UnresolvedShellChoice> {
        if session.local.country_random {
            return Err(UnresolvedShellChoice {
                ai_slot: None,
                choice: "country",
            });
        }
        if session.local.color_random {
            return Err(UnresolvedShellChoice {
                ai_slot: None,
                choice: "color",
            });
        }
        for (index, opponent) in session.opponents.iter().enumerate() {
            if opponent.country_random {
                return Err(UnresolvedShellChoice {
                    ai_slot: Some(index),
                    choice: "country",
                });
            }
            if opponent.color_random {
                return Err(UnresolvedShellChoice {
                    ai_slot: Some(index),
                    choice: "color",
                });
            }
        }
        Ok(Self { session })
    }

    /// The resolved session's gameplay facts.
    pub(crate) fn session(&self) -> &SkirmishLaunchSession {
        &self.session
    }
}

/// Construct the active offline-skirmish House array before any map object.
///
/// Active YR `ScenarioClass__Full_Init @ 0x00686B20` calls
/// `ScenarioClass__Create_Houses @ 0x00687F10` before terrain and Techno map
/// sections. The standard offline order is the human participant, AI slots,
/// then Neutral and Special. Object Reveal can therefore commit owned counts
/// and house-indexed base reservations directly, without a repair pass.
pub(crate) fn initialize_skirmish_launch_houses(
    sim: &mut Simulation,
    house_roster: &HouseRoster,
    rules: &RuleSet,
    descriptor: &MatchLaunchDescriptor,
) {
    let session = descriptor.session();
    assert!(
        sim.houses.is_empty()
            && sim.session.house_order.is_empty()
            && sim.ai_players.is_empty()
            && sim.entities().is_empty()
            && sim.production.terrain_objects.is_empty(),
        "skirmish houses must be initialized before map objects"
    );
    let slots = normalized_launch_slots(session);
    sim.session.game_options = session
        .options
        .to_game_options(session.opponents.len() as i32);
    populate_launch_houses(sim, &slots, rules);
    populate_special_houses(sim, house_roster, rules);
}

/// Commit the explicit launch-session diplomacy graph at the post-crate boundary.
pub(crate) fn apply_skirmish_launch_alliances(
    sim: &mut Simulation,
    house_roster: &HouseRoster,
    session: &SkirmishLaunchSession,
) {
    let slots = normalized_launch_slots(session);
    sim.house_alliances = launch_alliance_map(house_roster, &slots, &session.mode);
}

/// Apply an already validated explicit session without placeholder RNG draws.
pub(crate) fn apply_explicit_skirmish_launch_session(
    sim: &mut Simulation,
    map_data: &MapFile,
    house_roster: &HouseRoster,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
    descriptor: &MatchLaunchDescriptor,
) -> SkirmishLaunchApplyResult {
    apply_resolved_skirmish_launch_session(
        sim,
        map_data,
        house_roster,
        rules,
        height_map,
        resolved_terrain,
        descriptor,
        None,
    )
}

/// Apply the exact Battle assignment prepared before the first loading frame.
pub(crate) fn apply_preloaded_battle_launch_session(
    sim: &mut Simulation,
    map_data: &MapFile,
    house_roster: &HouseRoster,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
    descriptor: &MatchLaunchDescriptor,
    plan: &PreloadedBattleStartPlan,
) -> SkirmishLaunchApplyResult {
    apply_resolved_skirmish_launch_session(
        sim,
        map_data,
        house_roster,
        rules,
        height_map,
        resolved_terrain,
        descriptor,
        Some(plan),
    )
}

fn apply_resolved_skirmish_launch_session(
    sim: &mut Simulation,
    map_data: &MapFile,
    house_roster: &HouseRoster,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
    descriptor: &MatchLaunchDescriptor,
    preloaded_battle_plan: Option<&PreloadedBattleStartPlan>,
) -> SkirmishLaunchApplyResult {
    let session = descriptor.session();
    let slots = normalized_launch_slots(session);
    if sim.houses.is_empty() {
        // Direct unit-level callers may enter before the shared app load funnel.
        // The initializer rejects any already-constructed map object, so this
        // fallback cannot recreate the former object-before-house production path.
        initialize_skirmish_launch_houses(sim, house_roster, rules, descriptor);
    }
    assert_eq!(
        sim.session.house_order.len(),
        slots.len() + 2,
        "launch House array must be complete before start assignment"
    );
    for (registered, slot) in sim.session.house_order.iter().zip(&slots) {
        assert_eq!(
            sim.interner.resolve(*registered),
            slot.owner_name,
            "participant House order changed after map construction"
        );
    }
    for (registered, expected) in sim.session.house_order[slots.len()..]
        .iter()
        .zip(["Neutral", "Special"])
    {
        assert!(
            sim.interner
                .resolve(*registered)
                .eq_ignore_ascii_case(expected),
            "special House order changed after map construction"
        );
    }

    let bounds = NativeStartBounds::from_session(sim, resolved_terrain);
    let cooperative = session
        .mode
        .override_file
        .eq_ignore_ascii_case("MPCoopMD.ini");
    let (_starts, start_assignment) = if let Some(plan) = preloaded_battle_plan {
        debug_assert!(!cooperative, "Cooperative never owns a Battle preload plan");
        // The same immutable table already drove the first loading markers.
        // Its constructor/assignment RNG prefix was installed before terrain
        // Fill, so consuming it here must not draw or replace the live cursor.
        (plan.gathered_starts().to_vec(), plan.assignment().clone())
    } else {
        let preassignment_starts = sim.gather_native_start_positions(
            &map_data.waypoints,
            slots.len(),
            resolved_terrain,
            bounds,
        );
        // Standard Battle +0x80 gathers once before explicit preassignment,
        // then +0x84 gathers again before final assignment. The first vector's
        // cells are only provisional; deficient-map draws remain runtime-owned.
        let starts = if cooperative {
            preassignment_starts
        } else {
            sim.gather_native_start_positions(
                &map_data.waypoints,
                slots.len(),
                resolved_terrain,
                bounds,
            )
        };
        let assignment = if cooperative {
            let human_start_spots = map_data
                .ini
                .section("Header")
                .and_then(|header| header.get_i32("NumCoopHumanStartSpots"))
                .unwrap_or(0)
                .max(0) as usize;
            sim.assign_native_cooperative_starts(session, &starts, human_start_spots)
        } else {
            sim.assign_native_battle_starts(session, &starts)
        };
        (starts, assignment)
    };
    // Session start-slot -> house table: filled after the random-assignment
    // draws, before tick 0 — lockstep state (hashed + serialized).
    sim.session.start_slot_houses.clear();
    for (start_idx, slot_idx) in start_assignment.start_table.iter().enumerate() {
        let Some(slot_idx) = *slot_idx else {
            continue;
        };
        let Some(slot) = slots.get(slot_idx) else {
            continue;
        };
        let owner = sim.interner.intern(&slot.owner_name);
        sim.session
            .start_slot_houses
            .insert(start_idx as u32, owner);
    }
    let assignments = &start_assignment.placements;
    let mut spawned_mcvs = 0;
    let mut local_owner = slots.first().map(|slot| slot.owner_name.clone());

    assign_launch_base_centers(sim, &slots, assignments);

    if session.options.bases {
        for (slot_idx, waypoint) in assignments {
            let Some(slot) = slots.get(*slot_idx) else {
                continue;
            };
            let mcv_type = launch_mcv_type_for_country(slot.country, rules);
            if place_starting_mcv(
                sim,
                mcv_type,
                &slot.owner_name,
                waypoint.rx,
                waypoint.ry,
                bounds,
                rules,
                height_map,
                resolved_terrain,
            )
            .is_some()
            {
                spawned_mcvs += 1;
            } else {
                log::warn!(
                    "Failed to seed session MCV '{}' for {} at waypoint {} ({},{})",
                    mcv_type,
                    slot.owner_name,
                    waypoint.index,
                    waypoint.rx,
                    waypoint.ry
                );
                if slot.is_human {
                    local_owner = None;
                }
            }
        }

        if spawned_mcvs > 0 {
            log::info!(
                "Seeded {} session skirmish MCV(s) for {} active slot(s)",
                spawned_mcvs,
                slots.len()
            );
        }
    }

    seed_starting_extra_units(
        sim,
        &slots,
        rules,
        height_map,
        resolved_terrain,
        bounds,
        session.options.unit_count,
        map_data
            .special_flags
            .initial_veteran
            .unwrap_or(rules.initial_veteran),
    );

    // The selected-mode starting-force orchestrator advances Scenario RNG once
    // after the complete house pass, including zero-budget runs.
    let _ = sim.scenario_rng.next_range_u32_inclusive(0, 0xffff);

    let local_human_owner = slots
        .iter()
        .find(|slot| slot.is_human)
        .map(|slot| slot.owner_name.as_str());
    apply_launch_shroud_option(sim, local_human_owner);

    SkirmishLaunchApplyResult {
        local_owner,
        spawned_mcvs,
        active_slots: slots.len(),
    }
}

/// Apply the lobby's one-shot unexplored-shroud choice to the local human
/// viewer plane. AI houses keep their own unexplored knowledge and the reveal
/// never grants VERA's current-sight flag.
pub(crate) fn apply_launch_shroud_option(sim: &mut Simulation, local_owner: Option<&str>) {
    if sim.session.game_options.shroud {
        return;
    }
    let Some(owner) = local_owner.and_then(|owner| sim.interner.get(owner)) else {
        return;
    };
    if !sim.houses.get(&owner).is_some_and(|house| house.is_human) {
        return;
    }
    if let Some(terrain) = sim.resolved_terrain.as_ref() {
        sim.fog
            .reveal_cells_for_owner(owner, terrain.iter().map(|cell| (cell.rx, cell.ry)));
    } else {
        sim.fog.reveal_all_for_owner(owner);
    }
}

fn assign_launch_base_centers(
    sim: &mut Simulation,
    slots: &[NormalizedSkirmishSlot],
    assignments: &[(usize, Waypoint)],
) {
    for (slot_idx, waypoint) in assignments {
        let Some(slot) = slots.get(*slot_idx) else {
            continue;
        };
        let waypoint_edge = sim
            .playfield_bounds
            .map(|bounds| determine_waypoint_edge((waypoint.rx, waypoint.ry), bounds));
        if let Some(house) = crate::sim::house_state::house_state_for_owner_mut(
            &mut sim.houses,
            &slot.owner_name,
            &sim.interner,
        ) {
            house.base_center = Some((waypoint.rx, waypoint.ry));
            if let Some(waypoint_edge) = waypoint_edge {
                house.waypoint_edge = waypoint_edge;
            }
        }
    }
}

pub(crate) fn normalized_launch_slots(
    session: &SkirmishLaunchSession,
) -> Vec<NormalizedSkirmishSlot> {
    let mut slots = Vec::with_capacity(1 + session.opponents.len());
    slots.push(NormalizedSkirmishSlot {
        owner_name: session.player_name.clone(),
        country: session.local.country,
        color_index: session.local.color_index,
        start_position: session.local.start_position,
        team: session.local.team,
        is_human: true,
        difficulty: HouseDifficulty::Normal,
    });
    for (idx, opponent) in session.opponents.iter().enumerate() {
        slots.push(NormalizedSkirmishSlot {
            owner_name: format!("Computer{}", idx + 1),
            country: opponent.country,
            color_index: opponent.color_index,
            start_position: opponent.start_position,
            team: opponent.team,
            is_human: false,
            difficulty: HouseDifficulty::from_native(opponent.difficulty.as_i32())
                .expect("AiDifficulty uses native HouseClass discriminants"),
        });
    }
    slots
}

pub(crate) fn populate_special_houses(
    sim: &mut Simulation,
    house_roster: &HouseRoster,
    rules: &RuleSet,
) {
    for special_name in ["Neutral", "Special"] {
        let definition = house_roster
            .houses
            .iter()
            .find(|house| house.name.eq_ignore_ascii_case(special_name));
        let name = definition.map_or(special_name, |house| house.name.as_str());
        let name_id = sim.interner.intern(name);
        let country_name = definition
            .and_then(|house| house.country.as_deref())
            .unwrap_or(special_name);
        let country_id = Some(sim.interner.intern(country_name));
        let declared_side = definition.and_then(|house| house.side.as_deref());
        let side_idx = crate::sim::house_state::resolve_house_side_index(
            rules,
            Some(country_name),
            declared_side,
            crate::sim::house_state::side_index_from_name(declared_side),
        );
        let mut house = HouseState::new(
            name_id,
            side_idx,
            country_id,
            false,
            sim.session.game_options.starting_credits,
            sim.session.game_options.tech_level,
        );
        // Stock Neutral/Special are MultiplayPassive, which keeps them out of
        // defeat evaluation and out of the game-over alive scan. `country_name`
        // is always concrete here — the roster's `Country=` when the map named
        // one, otherwise the special house's own `[Countries]` entry, which is
        // what this house is being built as.
        house.multiplay_passive =
            crate::sim::house_state::resolve_multiplay_passive(Some(rules), Some(country_name));
        sim.houses.insert(name_id, house);
        sim.session.house_order.push(name_id);
    }
}

pub(crate) fn populate_launch_houses(
    sim: &mut Simulation,
    slots: &[NormalizedSkirmishSlot],
    rules: &RuleSet,
) {
    for slot in slots {
        let name_id = sim.interner.intern(&slot.owner_name);
        let country_name = slot.country.country_name();
        let country_id = sim.interner.intern(country_name);
        let side_index = crate::sim::house_state::resolve_house_side_index(
            rules,
            Some(country_name),
            None,
            slot.country.side_index(),
        );
        let mut house = HouseState::new(
            name_id,
            side_index,
            Some(country_id),
            slot.is_human,
            sim.session.game_options.starting_credits,
            sim.session.game_options.tech_level,
        );
        house.difficulty = slot.difficulty;
        // ScenarioClass::Create_Houses leaves generated human CurrentIQ at
        // the constructor value zero and stamps generated computer slots with
        // Rules.MaxIQLevels in non-campaign sessions.
        if !slot.is_human {
            house.current_iq = rules.general.max_iq_levels;
        }
        house.multiplay_passive =
            crate::sim::house_state::resolve_multiplay_passive(Some(rules), Some(country_name));
        sim.houses.insert(name_id, house);
        sim.session.house_order.push(name_id);
        if !slot.is_human {
            sim.ai_players.push(AiPlayerState::new(name_id));
            log::info!("AI player registered: {}", slot.owner_name);
        }
    }
}

/// Apply the generated skirmish AI opening-credit grant at the Post_Map_Init handoff.
///
/// Active YR `ScenarioClass__Post_Map_Init @ 0x00686890` visits each non-human,
/// non-`MultiplayPassive` house, calls the secondary vslot body at `0x004F6990`,
/// then passes its result to `HouseClass__Add_Credits @ 0x004F9950`. Generated
/// stock Battle houses have no stored resources at this point, so the returned
/// amount is their current balance and each participating AI finishes with twice
/// the lobby-selected credits. The AI-owner list keeps this launch-only helper
/// away from special and nonparticipant houses without recreating native pointers.
pub(crate) fn apply_skirmish_ai_opening_credits(sim: &mut Simulation) {
    for ai in &sim.ai_players {
        let Some(house) = sim.houses.get_mut(&ai.owner) else {
            continue;
        };
        if house.is_human || house.multiplay_passive {
            continue;
        }
        let opening_grant = house.credits;
        house.credits += opening_grant;
    }
}

fn normalize_house_key(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

pub(crate) fn launch_alliance_map(
    house_roster: &HouseRoster,
    slots: &[NormalizedSkirmishSlot],
    mode: &crate::skirmish_launch::SkirmishLaunchMode,
) -> crate::map::houses::HouseAllianceMap {
    let mut alliances = house_roster.alliance_map();
    for slot in slots {
        alliances
            .entry(normalize_house_key(&slot.owner_name))
            .or_default();
    }
    for left in slots {
        let LaunchTeam::Team(team) = left.team else {
            continue;
        };
        for right in slots {
            if left.owner_name == right.owner_name || right.team != LaunchTeam::Team(team) {
                continue;
            }
            let left_key = normalize_house_key(&left.owner_name);
            let right_key = normalize_house_key(&right.owner_name);
            alliances
                .entry(left_key.clone())
                .or_default()
                .insert(right_key.clone());
            alliances.entry(right_key).or_default().insert(left_key);
        }
    }
    if mode.override_file.eq_ignore_ascii_case("MPCoopMD.ini") {
        for left in slots {
            for right in slots {
                if left.owner_name == right.owner_name || left.is_human != right.is_human {
                    continue;
                }
                let left_key = normalize_house_key(&left.owner_name);
                let right_key = normalize_house_key(&right.owner_name);
                alliances
                    .entry(left_key.clone())
                    .or_default()
                    .insert(right_key.clone());
                alliances.entry(right_key).or_default().insert(left_key);
            }
        }
    }
    alliances
}

pub(crate) const STARTING_MCV_FACING: u8 = 64;
pub(crate) const STARTING_MCV_FALLBACK_MAX_RADIUS: i32 = 31;
pub(crate) const STARTING_EXTRA_UNIT_FALLBACK_START_RADIUS: i32 = 4;
pub(crate) const STARTING_MCV_FALLBACK_DIRECTIONS: &[(i32, i32)] = &[
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

fn place_starting_mcv(
    sim: &mut Simulation,
    mcv_type: &str,
    owner: &str,
    base_rx: u16,
    base_ry: u16,
    bounds: NativeStartBounds,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
) -> Option<u64> {
    place_starting_object_near_base(
        sim,
        mcv_type,
        owner,
        base_rx,
        base_ry,
        STARTING_MCV_FACING,
        1,
        bounds,
        rules,
        height_map,
        resolved_terrain,
    )
}

fn place_starting_object_near_base(
    sim: &mut Simulation,
    type_id: &str,
    owner: &str,
    base_rx: u16,
    base_ry: u16,
    facing: u8,
    start_radius: i32,
    bounds: NativeStartBounds,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
) -> Option<u64> {
    let category = rules.object(type_id)?.category;
    if starting_object_cell_placeable(sim, resolved_terrain, base_rx, base_ry, category) {
        return sim.spawn_object(type_id, owner, base_rx, base_ry, facing, rules, height_map);
    }

    for radius in start_radius..=STARTING_MCV_FALLBACK_MAX_RADIUS {
        let start_direction = sim.scenario_rng.next_range_u32_inclusive(0, 7) as usize;
        for jitter_pass in 0..2 {
            for offset in 0..8 {
                let direction = (start_direction + offset) & 7;
                let (dx, dy) = STARTING_MCV_FALLBACK_DIRECTIONS[direction];
                let (mut rx, mut ry) = bounds.clamp(
                    i32::from(base_rx) + dx * radius,
                    i32::from(base_ry) + dy * radius,
                );

                if jitter_pass != 0 {
                    let x_amount = sim.scenario_rng.next_range_u32_inclusive(0, 1) as i32;
                    let x_sign = sim.scenario_rng.next_range_u32_inclusive(0, 99);
                    let y_amount = sim.scenario_rng.next_range_u32_inclusive(0, 1) as i32;
                    let y_sign = sim.scenario_rng.next_range_u32_inclusive(0, 99);
                    let jittered = bounds.clamp(
                        i32::from(rx) + if x_sign < 50 { x_amount } else { -x_amount },
                        i32::from(ry) + if y_sign < 50 { y_amount } else { -y_amount },
                    );
                    rx = jittered.0;
                    ry = jittered.1;
                }

                if (rx, ry) == (base_rx, base_ry)
                    || !starting_object_cell_placeable(sim, resolved_terrain, rx, ry, category)
                {
                    continue;
                }
                if let Some(id) =
                    sim.spawn_object(type_id, owner, rx, ry, facing, rules, height_map)
                {
                    return Some(id);
                }
            }
        }
    }

    None
}

#[derive(Debug, Clone)]
pub(crate) struct StartingUnitCandidate {
    pub(crate) type_id: String,
    pub(crate) cost: i32,
}

pub(crate) const STARTING_EXTRA_UNIT_MAX_PLACEMENT_FAILURES: u8 = 20;

pub(crate) fn starting_unit_prefers_vehicle(spent: i32, budget: i32) -> bool {
    // gamemd 0x005D7337..0x005D7349 compares remaining budget against
    // trunc(initial_budget / 3). Expressed through `spent`, the strict
    // boundary is initial_budget - trunc(initial_budget / 3).
    spent < budget.wrapping_sub(budget / 3)
}

pub(crate) fn seed_starting_extra_units(
    sim: &mut Simulation,
    slots: &[NormalizedSkirmishSlot],
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    resolved_terrain: &ResolvedTerrainGrid,
    bounds: NativeStartBounds,
    unit_count: i32,
    initial_veteran: bool,
) -> u32 {
    if unit_count <= 0 {
        return 0;
    }
    let budget = starting_unit_budget(
        rules,
        slots,
        sim.session.game_options.tech_level,
        unit_count,
    );
    if budget <= 0 {
        return 0;
    }
    let mut spawned = 0;

    for slot in slots {
        let Some(base_center) = crate::sim::house_state::house_state_for_owner(
            &sim.houses,
            &slot.owner_name,
            &sim.interner,
        )
        .and_then(|house| house.base_center) else {
            continue;
        };
        let candidates = starting_unit_candidates_for_country(
            rules,
            slot.country,
            sim.session.game_options.tech_level,
        );
        if candidates.vehicles.is_empty() && candidates.infantry.is_empty() {
            continue;
        }
        let mut spent = 0i32;
        let mut placement_failures_left = STARTING_EXTRA_UNIT_MAX_PLACEMENT_FAILURES;
        while spent < budget && placement_failures_left != 0 {
            let prefer_vehicle = starting_unit_prefers_vehicle(spent, budget);
            let preferred = if prefer_vehicle {
                &candidates.vehicles
            } else {
                &candidates.infantry
            };
            let pool = if preferred.is_empty() {
                if prefer_vehicle {
                    &candidates.infantry
                } else {
                    break;
                }
            } else {
                preferred
            };
            let candidate_index =
                sim.scenario_rng
                    .next_range_u32_inclusive(0, pool.len() as u32 - 1) as usize;
            let candidate = &pool[candidate_index];
            let Some(stable_id) = place_starting_object_near_base(
                sim,
                &candidate.type_id,
                &slot.owner_name,
                base_center.0,
                base_center.1,
                STARTING_MCV_FACING,
                STARTING_EXTRA_UNIT_FALLBACK_START_RADIUS,
                bounds,
                rules,
                height_map,
                resolved_terrain,
            ) else {
                placement_failures_left -= 1;
                continue;
            };

            if initial_veteran {
                if let Some(entity) = sim.entities_mut().get_mut(stable_id) {
                    entity.veterancy = 200;
                }
            }
            let mission = if slot.is_human {
                MissionType::Guard
            } else {
                MissionType::AreaGuard
            };
            let _ = sim.mission_assign_exact(
                stable_id,
                MissionId::from_known(mission),
                sim.session.binary_frame,
            );
            spawned += 1;
            spent = spent.wrapping_add(candidate.cost);
        }
    }

    spawned
}

pub(crate) fn starting_unit_budget(
    rules: &RuleSet,
    slots: &[NormalizedSkirmishSlot],
    tech_level: i32,
    unit_count: i32,
) -> i32 {
    if unit_count <= 0 {
        return 0;
    }
    let mut total_cost: i32 = 0;
    let mut eligible_count: i32 = 0;
    for id in &rules.vehicle_ids {
        let Some(object) = rules.object(id) else {
            continue;
        };
        if !object.allowed_to_start_in_multiplayer
            || object.tech_level > tech_level
            || !slots
                .iter()
                .any(|slot| launch_country_can_own_object(slot.country, object))
            || rules
                .general
                .base_unit_types
                .iter()
                .any(|base_unit| base_unit.eq_ignore_ascii_case(&object.id))
        {
            continue;
        }
        eligible_count += 1i32;
        total_cost = total_cost.wrapping_add(object.cost);
    }
    for id in &rules.infantry_ids {
        let Some(object) = rules.object(id) else {
            continue;
        };
        if !object.allowed_to_start_in_multiplayer
            || object.tech_level > tech_level
            || !slots
                .iter()
                .any(|slot| launch_country_can_own_object(slot.country, object))
        {
            continue;
        }
        eligible_count += 1i32;
        total_cost = total_cost.wrapping_add(object.cost);
    }
    eligible_count = eligible_count.max(1);
    (eligible_count / 2)
        .wrapping_add(total_cost)
        .wrapping_div(eligible_count)
        .wrapping_mul(unit_count)
}

#[derive(Debug, Default)]
pub(crate) struct StartingUnitCandidates {
    pub(crate) vehicles: Vec<StartingUnitCandidate>,
    pub(crate) infantry: Vec<StartingUnitCandidate>,
}

pub(crate) fn starting_unit_candidates_for_country(
    rules: &RuleSet,
    country: LaunchCountry,
    tech_level: i32,
) -> StartingUnitCandidates {
    let vehicles = rules
        .vehicle_ids
        .iter()
        .filter_map(|id| {
            let object = rules.object(id)?;
            if !starting_unit_vehicle_allowed(rules, object, tech_level)
                || !launch_country_can_own_object(country, object)
            {
                return None;
            }
            Some(StartingUnitCandidate {
                type_id: id.clone(),
                cost: object.cost,
            })
        })
        .collect();
    let infantry = rules
        .infantry_ids
        .iter()
        .filter_map(|id| {
            let object = rules.object(id)?;
            if !starting_unit_infantry_allowed(object, tech_level)
                || !launch_country_can_own_object(country, object)
            {
                return None;
            }
            Some(StartingUnitCandidate {
                type_id: id.clone(),
                cost: object.cost,
            })
        })
        .collect();
    StartingUnitCandidates { vehicles, infantry }
}

fn starting_unit_vehicle_allowed(
    rules: &RuleSet,
    object: &crate::rules::object_type::ObjectType,
    tech_level: i32,
) -> bool {
    object.allowed_to_start_in_multiplayer
        && object.tech_level <= tech_level
        && !rules
            .general
            .base_unit_types
            .iter()
            .any(|base_unit| base_unit.eq_ignore_ascii_case(&object.id))
}

fn starting_unit_infantry_allowed(
    object: &crate::rules::object_type::ObjectType,
    tech_level: i32,
) -> bool {
    object.allowed_to_start_in_multiplayer && object.tech_level <= tech_level
}

fn starting_object_cell_placeable(
    sim: &Simulation,
    resolved_terrain: &ResolvedTerrainGrid,
    rx: u16,
    ry: u16,
    category: crate::rules::object_type::ObjectCategory,
) -> bool {
    // Acceptance is the retail isometric-diamond predicate: the mode's
    // starting-unit creator (vtable `+0xC8` @ `0x005D7030`) unlimbos at the
    // start coord, and its scan-place fallback @ `0x00688ED0` gates the seed
    // cell and every ring candidate with `MapClass::Is_Cell_In_Playfield(cell,
    // 1)`. The cell-array rect only clamps probe coordinates — it is never an
    // acceptance test.
    if !crate::sim::cell_rect::cell_is_in_playfield(
        (i32::from(rx), i32::from(ry)),
        sim.playfield_bounds,
        Some(resolved_terrain),
        Some((resolved_terrain.width(), resolved_terrain.height())),
    ) {
        return false;
    }
    if let Some(occupancy) = sim.occupancy().get(rx, ry) {
        let compatible_infantry = category == crate::rules::object_type::ObjectCategory::Infantry
            && occupancy
                .occupants
                .first()
                .and_then(|occupant| sim.entities().get(occupant.entity_id))
                .is_some_and(|entity| entity.category == EntityCategory::Infantry)
            && occupancy
                .occupants
                .iter()
                .filter(|occupant| occupant.sub_cell.is_some())
                .count()
                < 5;
        if !compatible_infantry {
            return false;
        }
    }
    let Some(cell) = resolved_terrain.cell(rx, ry) else {
        return false;
    };
    if cell.overlay_blocks || cell.terrain_object_blocks {
        return false;
    }
    if cell.has_bridge_deck && cell.bridge_walkable {
        return true;
    }
    cell.zone_type == crate::map::resolved_terrain::zone_class::GROUND
}

pub(crate) fn launch_mcv_type_for_country<'a>(
    country: LaunchCountry,
    rules: &'a RuleSet,
) -> &'a str {
    rules
        .general
        .base_unit_types
        .iter()
        .find_map(|id| {
            let object = rules.object(id)?;
            launch_country_can_own_object(country, object).then_some(id.as_str())
        })
        .or_else(|| {
            rules
                .general
                .base_unit_types
                .iter()
                .find(|id| rules.object(id).is_some())
                .map(String::as_str)
        })
        .unwrap_or("AMCV")
}

fn launch_country_can_own_object(
    country: LaunchCountry,
    object: &crate::rules::object_type::ObjectType,
) -> bool {
    let country_name = country.country_name();
    if !object.owner.iter().any(|owner| owner == country_name) {
        return false;
    }
    if !object.required_houses.is_empty()
        && !object
            .required_houses
            .iter()
            .any(|required| required == country_name)
    {
        return false;
    }
    !object
        .forbidden_houses
        .iter()
        .any(|forbidden| forbidden == country_name)
}

/// Opaque owner for the RNG cursors used while a map becomes a world.
///
/// The app may drive content-loading algorithms through the two narrow draw
/// wrappers and may transport an accepted generated-map continuation, but it
/// cannot replace, extract, or draw from those cursors. Consuming this owner is
/// the only production handoff into a freshly constructed Simulation.
pub(crate) struct ScenarioBootstrapRng {
    scenario: SimRng,
    main: SimRng,
    mapgen: Option<SimRng>,
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
            mapgen: None,
        }
    }

    /// Install the accepted random map's process-global cursor exactly once.
    pub(crate) fn install_generated_mapgen_continuation(
        &mut self,
        continuation: MapGenRngContinuation,
    ) {
        assert!(
            self.mapgen.is_none(),
            "generated MapGen continuation may only be installed once"
        );
        self.mapgen = Some(SimRng::from_mapgen_continuation(continuation));
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

    /// Finish the app-to-sim construction handoff with every bound cursor.
    pub(crate) fn into_simulation(self, descriptor: &ScenarioDescriptor) -> Simulation {
        let mut sim = Simulation::from_descriptor(descriptor);
        sim.install_terrain_load_advanced_scenario_rng(self.scenario);
        sim.install_variant_advanced_main_rng(self.main);
        if let Some(mapgen) = self.mapgen {
            sim.install_generated_mapgen_rng(mapgen);
        }
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
            self.playfield_bounds,
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

impl Simulation {
    /// Register an AI player for every playable roster house except the local
    /// owner (F10 boundary method: the app names the local owner, sim owns
    /// the writes). Neutral/civilian/special houses never receive AI.
    pub(crate) fn register_ai_players_from_roster(
        &mut self,
        house_roster: &HouseRoster,
        local_owner: &str,
    ) {
        use crate::sim::ai::AiPlayerState;

        for house in &house_roster.houses {
            let up = house.name.to_ascii_uppercase();
            if matches!(
                up.as_str(),
                "NEUTRAL" | "SPECIAL" | "CIVILIAN" | "GOODGUY" | "BADGUY" | "JP"
            ) {
                continue;
            }
            if house.name.eq_ignore_ascii_case(local_owner) {
                continue;
            }
            self.ai_players
                .push(AiPlayerState::new(self.interner.intern(&house.name)));
            log::info!("AI player registered: {}", house.name);
        }
    }

    /// Mark the named house human-controlled (F10 boundary method). Returns
    /// false when the owner is not interned or owns no house.
    pub(crate) fn mark_house_human(&mut self, owner: &str) -> bool {
        let Some(owner_id) = self.interner.get(owner) else {
            return false;
        };
        let Some(house) = self.houses.get_mut(&owner_id) else {
            return false;
        };
        house.is_human = true;
        true
    }
}

/// Map-roster house construction shared by app and headless (F09):
/// native order requires houses before every object section.
pub(crate) fn initialize_map_roster_houses(
    sim: &mut Simulation,
    house_roster: &HouseRoster,
    rules: Option<&RuleSet>,
) {
    assert!(
        sim.houses.is_empty()
            && sim.session.house_order.is_empty()
            && sim.entities().is_empty()
            && sim.production.terrain_objects.is_empty(),
        "scenario houses must be initialized before map objects"
    );
    for house in &house_roster.houses {
        let fallback_side = crate::sim::house_state::side_index_from_name(house.side.as_deref());
        let side_idx = rules.map_or(fallback_side, |rules| {
            crate::sim::house_state::resolve_house_side_index(
                rules,
                house.country.as_deref(),
                house.side.as_deref(),
                fallback_side,
            )
        });
        let player_control = house.player_control == Some(true);
        let name_id = sim.interner.intern(&house.name);
        let country_id = house.country.as_deref().map(|c| sim.interner.intern(c));
        let mut house_state = crate::sim::house_state::HouseState::new(
            name_id,
            side_idx,
            country_id,
            false,
            sim.session.game_options.starting_credits,
            sim.session.game_options.tech_level,
        );
        house_state.player_control = player_control;
        // HouseClass::Read_Scenario_INI reads `IQ=` from this exact named
        // house section, defaults it to zero, and changes a value above
        // MaxIQLevels to literal one before storing CurrentIQ (+0x24C).
        house_state.current_iq = rules.map_or_else(
            || house.iq.unwrap_or(0),
            |rules| house.scenario_current_iq(rules.general.max_iq_levels),
        );
        // MultiplayPassive lives on the country/house type. A roster section
        // with no `Country=` resolves through `[Countries]` entry zero.
        house_state.multiplay_passive =
            crate::sim::house_state::resolve_multiplay_passive(rules, house.country.as_deref());
        sim.houses.insert(name_id, house_state);
        sim.session.house_order.push(name_id);
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
    fn nearoref_native_start_bounds_use_inclusive_full_cell_array_endpoints() {
        let mut sim = Simulation::new();
        sim.session.map_width = 138;
        sim.session.map_height = 138;
        sim.session.local_left = 2;
        sim.session.local_top = 4;
        sim.session.local_width = 76;
        sim.session.local_height = 48;
        let terrain = ResolvedTerrainGrid::from_cells(138, 138, Vec::new());

        let bounds = NativeStartBounds::from_session(&sim, &terrain);

        assert_eq!(
            (
                bounds.min_rx,
                bounds.min_ry,
                bounds.width,
                bounds.height,
                bounds.max_rx(),
                bounds.max_ry(),
            ),
            (1, 1, 137, 137, 137, 137)
        );
        assert_eq!(bounds.clamp(0, 0), (1, 1));
        assert_eq!(bounds.clamp(1, 1), (1, 1));
        assert_eq!(bounds.clamp(100, 75), (100, 75));
        assert_eq!(bounds.clamp(137, 137), (137, 137));
        assert_eq!(bounds.clamp(138, 138), (137, 137));
        assert_eq!(bounds.clamp(i32::MIN, i32::MAX), (1, 137));
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
    fn generated_mapgen_continuation_replaces_only_mapgen_cursor() {
        let match_seed = 0x51C0_1005;
        let map_seed: u16 = 0xBEEF;
        let mut expected_mapgen = SimRng::new(u64::from(map_seed));
        for _ in 0..353 {
            let _ = expected_mapgen.next_u32();
        }
        let generated = expected_mapgen.logical_state();

        let mut owner = ScenarioBootstrapRng::new(match_seed);
        owner.install_generated_mapgen_continuation(MapGenRngContinuation::from_native_parts(
            generated.words,
            usize::try_from(generated.index_a).expect("test MapGen cursor A is non-negative"),
            usize::try_from(generated.index_b).expect("test MapGen cursor B is non-negative"),
        ));
        let actual = owner.into_simulation(&descriptor(match_seed)).rng_state();

        assert_eq!(
            actual.scenario,
            SimRng::new(u64::from(match_seed)).logical_state()
        );
        assert_eq!(
            actual.main,
            SimRng::new(u64::from(match_seed)).logical_state()
        );
        assert_eq!(actual.mapgen, expected_mapgen.logical_state());
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
