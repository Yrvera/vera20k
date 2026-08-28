//! Native crate placement, timer, clear, and regeneration lifecycle.

use crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::rules::ruleset::{CrateRules, RuleSet};
use crate::rules::terrain_rules::LandType;
use crate::sim::cell_rect::cell_is_in_playfield_height_aware;
use crate::sim::find_nearby_cell::{
    NearbyAnchorGate, NearbyFootprint, NearbyQuery, PassabilityArgs, find_nearby_passable_cell,
};
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;
use crate::util::native_x87::{NativeF64Bits, X87Chop53};

use super::{CrateAuthority, CratePresentationEvent, CrateSlot};

const MAX_PLACEMENT_ATTEMPTS: usize = 1000;
const CRATE_SNAP_RADIUS_CAP: u16 = 32;
const MARK_STEEP_SLOPE_EXCEPTION_ID: u8 = 0xB2;
const SPECIFIC_DATA_SENTINEL: i32 = 0x14;

/// Result of the two-stage native validator. Every failure after the two hard
/// prechecks is accepted state and therefore owns a slot/timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CratePlacementOutcome {
    HardRejected,
    AcceptedVisible,
    AcceptedGhost,
}

impl CratePlacementOutcome {
    const fn accepted(self) -> bool {
        !matches!(self, Self::HardRejected)
    }
}

/// Deterministic fault seam for the native allocation/constructor/Unlimbo/Mark
/// stages. Production always supplies `default()`; focused tests inject each
/// post-precheck failure and prove it remains an accepted ghost.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CratePlacementFaults {
    pub(crate) allocation: bool,
    pub(crate) construction: bool,
    pub(crate) unlimbo: bool,
    pub(crate) mark: bool,
}

/// Outcome of scenario bootstrap's signed requested loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CratePlacement {
    pub requested: u32,
    pub placed: u32,
}

/// `min(CrateMaximum, max(CrateMinimum, human-node-count))`, with the signed
/// loop predicate preserved. A negative result performs no iterations.
///
/// gamemd-derived: `ScenarioClass__Post_Map_Init @ 0x00686890`.
pub fn scenario_start_crate_count(rules: &CrateRules, player_count: u32) -> u32 {
    rules
        .maximum
        .min(rules.minimum.max(player_count as i32))
        .max(0) as u32
}

/// Pregame human nodes only; AI/passive Houses are separate native arrays.
pub fn human_player_count(sim: &Simulation) -> u32 {
    sim.houses
        .values()
        .filter(|house| house.is_human && !house.multiplay_passive)
        .count() as u32
}

/// Bootstrap adapter. The lobby Crates option is the sole gate and a failed
/// random call is never topped up.
pub(crate) fn place_scenario_start_crates(
    sim: &mut Simulation,
    rules: &RuleSet,
    overlay_registry: &OverlayTypeRegistry,
    path_grid: Option<&PathGrid>,
    player_count: u32,
) -> CratePlacement {
    if !sim.session.game_options.crates {
        return CratePlacement {
            requested: 0,
            placed: 0,
        };
    }
    let requested = scenario_start_crate_count(&rules.crate_rules, player_count);
    let mut placed = 0;
    for _ in 0..requested {
        placed += u32::from(sim.place_random_crate(rules, overlay_registry, path_grid));
    }
    CratePlacement { requested, placed }
}

impl CrateAuthority {
    /// `MapClass__PlaceCrateAtRandomCell @ 0x0056BD40`.
    pub(crate) fn place_random(
        &mut self,
        sim: &mut Simulation,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        path_grid: Option<&PathGrid>,
    ) -> bool {
        let Some(slot_index) = self.first_empty_slot() else {
            // Native scans slots before touching Scenario RNG.
            return false;
        };
        let Some((left, top, width, height)) = active_map_rectangle(sim) else {
            return false;
        };

        for _ in 0..MAX_PLACEMENT_ATTEMPTS {
            // Load-bearing order: X, then Y.
            let x = left.wrapping_add(
                sim.scenario_rng
                    .next_range_u32_inclusive(0, u32::from(width - 1)) as u16,
            );
            let y = top.wrapping_add(
                sim.scenario_rng
                    .next_range_u32_inclusive(0, u32::from(height - 1)) as u16,
            );
            let surface = surface_at(sim, (x, y));
            let Some(snapped) = snap_to_passable(sim, path_grid, (x, y), surface) else {
                continue;
            };
            let outcome = self.place_at_snapped(
                sim,
                rules,
                overlay_registry,
                slot_index,
                snapped,
                CratePlacementFaults::default(),
            );
            if outcome.accepted() {
                return true;
            }
        }
        false
    }

    /// `MapClass__PlaceCrateAtCell @ 0x0056BEC0`: snap once, then scan slots.
    pub(crate) fn place_specific(
        &mut self,
        sim: &mut Simulation,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        path_grid: Option<&PathGrid>,
        origin: (u16, u16),
        full_data_dword: i32,
    ) -> bool {
        self.place_specific_with_faults(
            sim,
            rules,
            overlay_registry,
            path_grid,
            origin,
            full_data_dword,
            CratePlacementFaults::default(),
        )
    }

    pub(crate) fn place_specific_with_faults(
        &mut self,
        sim: &mut Simulation,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        path_grid: Option<&PathGrid>,
        origin: (u16, u16),
        full_data_dword: i32,
        faults: CratePlacementFaults,
    ) -> bool {
        let surface = surface_at(sim, origin);
        let Some(snapped) = snap_to_passable(sim, path_grid, origin, surface) else {
            return false;
        };
        let Some(slot_index) = self.first_empty_slot() else {
            return false;
        };
        let outcome =
            self.place_at_snapped(sim, rules, overlay_registry, slot_index, snapped, faults);
        if !outcome.accepted() {
            return false;
        }
        // Native compares the complete dword, but stores only its low byte.
        if full_data_dword != SPECIFIC_DATA_SENTINEL
            && let Some(grid) = sim.overlay_grid.as_mut()
        {
            let _ = grid.write_crate_data_no_dirty(snapped.0, snapped.1, full_data_dword as u8);
        }
        true
    }

    /// Clear the first nonzero-mode slot at the packed coordinate, or the
    /// direct crate overlay in mode zero.
    pub(crate) fn remove_at_cell(
        &mut self,
        sim: &mut Simulation,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        cell: (u16, u16),
    ) -> bool {
        if !sim.session.game_mode_nonzero {
            return remove_mode_zero_overlay(sim, overlay_registry, cell);
        }
        let Some(slot_index) = self.first_slot_at(cell) else {
            return false;
        };
        self.clear_slot(sim, rules, overlay_registry, slot_index)
    }

    /// `CrateSlot__ClearAndPreserveTimer @ 0x004A1750`.
    pub(crate) fn clear_slot(
        &mut self,
        sim: &mut Simulation,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        slot_index: usize,
    ) -> bool {
        let Some(slot) = self.slots.get(slot_index).copied() else {
            return false;
        };
        let Some((cell_x, cell_y)) = slot.cell() else {
            return false;
        };
        let cell = (cell_x as u16, cell_y as u16);
        let _ = remove_slot_overlay(sim, rules, overlay_registry, cell);
        self.slots[slot_index].clear_coordinate_and_preserve_timer(sim.session.binary_frame as i32);
        true
    }

    /// `MapClass__UpdateCrateRegenTimers @ 0x0056BBE0`.
    pub(crate) fn update_regeneration(
        &mut self,
        sim: &mut Simulation,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        path_grid: Option<&PathGrid>,
    ) {
        if !sim.session.game_mode_nonzero || !sim.session.game_options.crates {
            return;
        }
        let frame = sim.session.binary_frame as i32;
        for slot_index in 0..self.slots.len() {
            // No snapshot: an insertion above the cursor is observed later in
            // this same pass, including zero/negative-duration cascades.
            if self.slots[slot_index].is_occupied() && self.slots[slot_index].expired(frame) {
                let _ = self.clear_slot(sim, rules, overlay_registry, slot_index);
                let _ = self.place_random(sim, rules, overlay_registry, path_grid);
            }
        }
    }

    fn place_at_snapped(
        &mut self,
        sim: &mut Simulation,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        slot_index: usize,
        cell: (u16, u16),
        faults: CratePlacementFaults,
    ) -> CratePlacementOutcome {
        // The only hard rejections in `CrateSlot__ValidateCellAndCreateOverlay
        // @ 0x004A18F0` are mode-one playfield failure and prior identity.
        if !cell_is_in_playfield_height_aware(
            (i32::from(cell.0), i32::from(cell.1)),
            sim.playfield_bounds,
            sim.resolved_terrain.as_ref(),
        ) || sim
            .overlay_grid
            .as_ref()
            .is_none_or(|grid| grid.cell(cell.0, cell.1).overlay_id.is_some())
        {
            return CratePlacementOutcome::HardRejected;
        }

        let selected_name = if surface_at(sim, cell) == CrateSurface::Water {
            rules.crate_rules.water_crate_img.as_deref()
        } else {
            rules.crate_rules.wood_crate_img.as_deref()
        };
        let selected_id = selected_name.and_then(|name| overlay_registry.id_for_name(name));
        let visible = selected_id.is_some_and(|overlay_id| {
            crate_mark_succeeds(sim, rules, overlay_registry, cell, overlay_id, faults)
        });
        if visible {
            let overlay_id = selected_id.expect("visible crate resolved an overlay identity");
            let placed = sim
                .overlay_grid
                .as_mut()
                .is_some_and(|grid| grid.place_crate_overlay_bytes(cell.0, cell.1, overlay_id));
            debug_assert!(
                placed,
                "hard precheck established an allocated overlay cell"
            );
        }

        let outcome = if visible {
            CratePlacementOutcome::AcceptedVisible
        } else {
            CratePlacementOutcome::AcceptedGhost
        };
        // `0x004A1994..0x004A1A78`: DirtyScreenRect precedes the outer
        // CellRedraw call even for the accepted ghost's zero rectangle.
        sim.crate_presentation
            .push(CratePresentationEvent::DirtyScreenRect {
                cell: visible.then_some(cell),
                force: false,
            });
        sim.crate_presentation
            .push(CratePresentationEvent::CellRedraw {
                cell,
                frame: sim.session.binary_frame as i32,
            });

        let timer = build_crate_timer(&mut sim.scenario_rng, rules.crate_rules.regen);
        self.slots[slot_index] = CrateSlot {
            // The complete placement tail observes the pre-increment frame.
            start_frame: sim.session.binary_frame as i32,
            timer_aux: timer.aux,
            duration_frames: timer.duration,
            cell_x: cell.0 as i16,
            cell_y: cell.1 as i16,
        };
        outcome
    }
}

impl Simulation {
    pub(crate) fn place_random_crate(
        &mut self,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        path_grid: Option<&PathGrid>,
    ) -> bool {
        let mut authority = std::mem::take(&mut self.crate_authority);
        let placed = authority.place_random(self, rules, overlay_registry, path_grid);
        self.crate_authority = authority;
        placed
    }

    pub(crate) fn place_specific_crate(
        &mut self,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        path_grid: Option<&PathGrid>,
        origin: (u16, u16),
        full_data_dword: i32,
    ) -> bool {
        let mut authority = std::mem::take(&mut self.crate_authority);
        let placed = authority.place_specific(
            self,
            rules,
            overlay_registry,
            path_grid,
            origin,
            full_data_dword,
        );
        self.crate_authority = authority;
        placed
    }

    pub(crate) fn remove_crate_at_cell(
        &mut self,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        cell: (u16, u16),
    ) -> bool {
        let mut authority = std::mem::take(&mut self.crate_authority);
        let removed = authority.remove_at_cell(self, rules, overlay_registry, cell);
        self.crate_authority = authority;
        removed
    }

    pub(crate) fn update_crate_regeneration(
        &mut self,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        path_grid: Option<&PathGrid>,
    ) {
        let mut authority = std::mem::take(&mut self.crate_authority);
        authority.update_regeneration(self, rules, overlay_registry, path_grid);
        self.crate_authority = authority;
    }

    #[cfg(test)]
    pub(crate) fn place_specific_crate_with_faults(
        &mut self,
        rules: &RuleSet,
        overlay_registry: &OverlayTypeRegistry,
        path_grid: Option<&PathGrid>,
        origin: (u16, u16),
        full_data_dword: i32,
        faults: CratePlacementFaults,
    ) -> bool {
        let mut authority = std::mem::take(&mut self.crate_authority);
        let placed = authority.place_specific_with_faults(
            self,
            rules,
            overlay_registry,
            path_grid,
            origin,
            full_data_dword,
            faults,
        );
        self.crate_authority = authority;
        placed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrateSurface {
    Land,
    Water,
}

fn surface_at(sim: &Simulation, cell: (u16, u16)) -> CrateSurface {
    if sim
        .resolved_terrain
        .as_ref()
        .and_then(|terrain| terrain.cell(cell.0, cell.1))
        .is_some_and(|cell| cell.yr_cell_land_type == LandType::Water.as_index())
    {
        CrateSurface::Water
    } else {
        CrateSurface::Land
    }
}

fn active_map_rectangle(sim: &Simulation) -> Option<(u16, u16, u16, u16)> {
    // The installed MapClass rectangle spans canonical cells 1..SizeW+SizeH-1
    // on each available axis. Keeping independent width/height avoids the stale
    // square-grid assumption while retaining the native left+range formula.
    let width = sim.session.map_width.checked_sub(1)?;
    let height = sim.session.map_height.checked_sub(1)?;
    (width != 0 && height != 0).then_some((1, 1, width, height))
}

fn snap_to_passable(
    sim: &Simulation,
    path_grid: Option<&PathGrid>,
    origin: (u16, u16),
    surface: CrateSurface,
) -> Option<(u16, u16)> {
    let query = NearbyQuery {
        passability: PassabilityArgs {
            speed_type: match surface {
                CrateSurface::Water => SpeedType::Float,
                CrateSurface::Land => SpeedType::Track,
            },
            required_zone_id: None,
            movement_zone: MovementZone::Normal,
            bridge_aware_zone: false,
        },
        footprint: NearbyFootprint::SINGLE,
        anchor_gate: NearbyAnchorGate::NativeHeightAware,
        allow_bridge_cells: true,
        check_height: false,
        check_occupancy: false,
        radius_cap: sim.session.map_width.min(CRATE_SNAP_RADIUS_CAP),
        target_cell: None,
        path_grid,
        resolved_terrain: sim.resolved_terrain.as_ref(),
        overlay_grid: sim.overlay_grid.as_ref(),
        occupancy: Some(&sim.substrate.occupancy),
        entities: Some(&sim.substrate.entities),
        zone_grid: sim.zone_grid.as_ref(),
        playfield_bounds: sim.playfield_bounds,
    };
    find_nearby_passable_cell(
        (i32::from(origin.0), i32::from(origin.1)),
        &query,
        sim.session.binary_frame,
    )
}

fn crate_mark_succeeds(
    sim: &Simulation,
    rules: &RuleSet,
    overlay_registry: &OverlayTypeRegistry,
    cell: (u16, u16),
    selected_id: u8,
    faults: CratePlacementFaults,
) -> bool {
    if faults.allocation || faults.construction {
        return false;
    }
    // Constructor ground-list scan; ordinary Foot/Building occupation is not
    // this gate.
    if sim.production.terrain_object_cells.contains_key(&cell) || faults.unlimbo {
        return false;
    }
    let Some(terrain_cell) = sim
        .resolved_terrain
        .as_ref()
        .and_then(|terrain| terrain.cell(cell.0, cell.1))
    else {
        return false;
    };
    if (terrain_cell.slope_type > 4 && selected_id != MARK_STEEP_SLOPE_EXCEPTION_ID)
        || faults.mark
        || overlay_registry.flags(selected_id).is_none()
    {
        return false;
    }

    let water_id = rules
        .crate_rules
        .water_crate_img
        .as_deref()
        .and_then(|name| overlay_registry.id_for_name(name));
    let crate_id = rules
        .crate_rules
        .crate_img
        .as_deref()
        .and_then(|name| overlay_registry.id_for_name(name));
    let wood_id = rules
        .crate_rules
        .wood_crate_img
        .as_deref()
        .and_then(|name| overlay_registry.id_for_name(name));
    let speed = if water_id == Some(selected_id) {
        SpeedType::Float
    } else if crate_id == Some(selected_id) || wood_id == Some(selected_id) {
        SpeedType::Track
    } else {
        return false;
    };
    let bridge_selected = terrain_cell.bridge_facts.has_flag(BRIDGE_FLAG_STRUCTURAL);
    let occupation = if bridge_selected {
        sim.substrate.raw_cell_occupation.deck_bits(cell.0, cell.1)
    } else {
        sim.substrate
            .raw_cell_occupation
            .ground_bits(cell.0, cell.1)
    };
    if occupation != 0 {
        return false;
    }
    bridge_selected
        || match speed {
            SpeedType::Float => terrain_cell
                .speed_costs
                .float
                .is_some_and(|value| value != 0),
            SpeedType::Track => terrain_cell
                .speed_costs
                .track
                .is_some_and(|value| value != 0),
            _ => unreachable!("crate Mark selects only Float or Track"),
        }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CrateTimer {
    pub(super) duration: i32,
    pub(super) aux: u32,
}

pub(super) fn build_crate_timer(
    rng: &mut crate::sim::rng::SimRng,
    regen: NativeF64Bits,
) -> CrateTimer {
    // gamemd-derived: `CrateSlot__PlaceOverlayAndInitTimer @ 0x004A17C0`.
    let regen =
        X87Chop53::load_f64(regen).expect("ReadDouble crate regen is a finite binary64 value");
    let lower = X87Chop53::mul(regen, X87Chop53::load_i32(450));
    let upper = X87Chop53::mul(regen, X87Chop53::load_i32(1800));
    let upper_bits = X87Chop53::store_f64(upper)
        .expect("finite f32-widened crate regen times 1800 fits binary64");
    let draw = rng.next_range_u32_inclusive(0, 0x7fff_fffe);
    let ratio = X87Chop53::div(
        X87Chop53::load_i32(draw as i32),
        X87Chop53::load_i32(0x7fff_fffe),
    )
    .expect("crate timer denominator is nonzero");
    let interpolated = X87Chop53::add(lower, X87Chop53::mul(ratio, X87Chop53::sub(upper, lower)));
    CrateTimer {
        // x87 invalid conversion stores the integer-indefinite low dword.
        duration: X87Chop53::ftol_i64(interpolated).unwrap_or(i64::MIN) as i32,
        aux: (upper_bits.bits() >> 32) as u32,
    }
}

fn remove_slot_overlay(
    sim: &mut Simulation,
    rules: &RuleSet,
    overlay_registry: &OverlayTypeRegistry,
    cell: (u16, u16),
) -> bool {
    let Some(grid) = sim.overlay_grid.as_ref() else {
        return false;
    };
    if cell.0 >= grid.width() || cell.1 >= grid.height() {
        return false;
    }
    let current = grid.cell(cell.0, cell.1).overlay_id;
    let accepted = [
        rules.crate_rules.crate_img.as_deref(),
        rules.crate_rules.wood_crate_img.as_deref(),
        rules.crate_rules.water_crate_img.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter_map(|name| overlay_registry.id_for_name(name))
    .any(|identity| Some(identity) == current);
    if !accepted {
        return false;
    }
    sim.crate_presentation
        .push(CratePresentationEvent::DirtyScreenRect {
            cell: Some(cell),
            force: false,
        });
    sim.overlay_grid
        .as_mut()
        .is_some_and(|grid| grid.clear_crate_overlay_bytes(cell.0, cell.1))
}

fn remove_mode_zero_overlay(
    sim: &mut Simulation,
    overlay_registry: &OverlayTypeRegistry,
    cell: (u16, u16),
) -> bool {
    let Some(grid) = sim.overlay_grid.as_ref() else {
        return false;
    };
    if cell.0 >= grid.width() || cell.1 >= grid.height() {
        return false;
    }
    let accepted = grid
        .cell(cell.0, cell.1)
        .overlay_id
        .and_then(|id| overlay_registry.flags(id))
        .is_some_and(|flags| flags.crate_type);
    if !accepted {
        return false;
    }
    sim.crate_presentation
        .push(CratePresentationEvent::DirtyScreenRect {
            cell: Some(cell),
            force: false,
        });
    sim.overlay_grid
        .as_mut()
        .is_some_and(|grid| grid.clear_crate_overlay_bytes(cell.0, cell.1))
}
