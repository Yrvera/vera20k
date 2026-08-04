//! Tank-bunker install state machine (building side).
//!
//! Models the `Bunker=yes` mission helper: a facing-driven 6-state machine that,
//! once a candidate unit is on the footprint, shoves blockers, turns the unit to
//! face the building, force-tracks it onto the building cell, turns it South,
//! plays entry anims, then installs (hide + reciprocal link + up sound). The
//! inter-state waits are turn/track completions — NOT frame-count timers.
//!
//! sim/ only — never render/ui/sidebar/audio/net.
use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::game_entity::BunkerLink;
use crate::sim::movement;
use crate::sim::movement::bump_crush::scatter_blocker;
use crate::sim::movement::drive_track::begin_forced_turn_track;
use crate::sim::movement::facing_from_delta;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::entity_occupancy_cells;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;
use serde::{Deserialize, Serialize};

/// Desired body facing for an installed unit (South).
const SOUTH_FACING: u8 = 0x80;

/// Install progress for a tank bunker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BunkerState {
    /// Empty / not installing.
    #[default]
    Idle,
    /// Candidate admitted; waiting for it to arrive on the footprint + stop, then shove blockers.
    ArriveWait,
    /// Waiting for the footprint to clear of other objects, then face the building.
    ClearWait,
    /// Turning the unit to face the building.
    TurnToBuilding,
    /// Force-track sub-cell step onto the building cell in progress.
    TrackStep,
    /// Turning the unit to South (desired body facing 0x80).
    TurnSouth,
    /// Installed.
    Occupied,
}

/// Building-side bunker runtime. `Some(..)` on `Bunker=yes` buildings from spawn;
/// its presence is what marks an entity as a tank bunker (the radio bus routes on
/// `bunker_runtime.is_some()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct BunkerRuntime {
    pub state: BunkerState,
    /// Candidate unit during ArriveWait..TurnSouth; `None` when Idle/Occupied
    /// (Occupied tracks the occupant via `GameEntity.bunker_occupant`).
    pub installing_unit: Option<u64>,
}

impl BunkerRuntime {
    /// An empty, idle bunker (seeded at spawn for `Bunker=yes` buildings).
    pub fn idle() -> Self {
        Self {
            state: BunkerState::Idle,
            installing_unit: None,
        }
    }
}

/// Advance every actively-installing tank bunker by one tick. The waits between
/// states are facing-turn / force-track completions, NOT frame-count timers.
/// `ClearWait` actively scatters other units off the footprint (gamemd Scatter())
/// so the installing unit can take the install cell.
pub fn tick_bunker_install(sim: &mut Simulation, rules: &RuleSet, path_grid: Option<&PathGrid>) {
    for building_id in sim.substrate.entities.keys_sorted() {
        let active = sim.substrate.entities.get(building_id).is_some_and(|b| {
            matches!(
                b.bunker_runtime.map(|rt| rt.state),
                Some(BunkerState::ArriveWait)
                    | Some(BunkerState::ClearWait)
                    | Some(BunkerState::TurnToBuilding)
                    | Some(BunkerState::TrackStep)
                    | Some(BunkerState::TurnSouth)
            )
        });
        if active {
            step_install(sim, rules, path_grid, building_id);
        }
    }
}

fn step_install(
    sim: &mut Simulation,
    rules: &RuleSet,
    path_grid: Option<&PathGrid>,
    building_id: u64,
) {
    let Some((state, candidate)) = sim
        .substrate
        .entities
        .get(building_id)
        .and_then(|b| b.bunker_runtime.map(|rt| (rt.state, rt.installing_unit)))
    else {
        return;
    };
    let Some(unit_id) = candidate else {
        set_state(sim, building_id, BunkerState::Idle, None);
        return;
    };
    // Abort if the candidate vanished or stopped approaching THIS bunker
    // (any retask clears the unit-side marker).
    let approaching = sim
        .substrate
        .entities
        .get(unit_id)
        .is_some_and(|u| u.bunker_link == BunkerLink::Approaching(building_id));
    if !approaching {
        set_state(sim, building_id, BunkerState::Idle, None);
        return;
    }

    match state {
        BunkerState::ArriveWait => {
            if on_footprint_and_stopped(sim, building_id, unit_id) {
                set_state(sim, building_id, BunkerState::ClearWait, Some(unit_id));
            }
        }
        BunkerState::ClearWait => {
            if footprint_clear_of_others(sim, building_id, unit_id) {
                if let Some(f) = facing_to_anchor(sim, building_id, unit_id) {
                    if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
                        u.facing_target = Some(f);
                    }
                }
                set_state(sim, building_id, BunkerState::TurnToBuilding, Some(unit_id));
            } else {
                // Shove the blockers off the footprint (gamemd Scatter()); wait.
                shove_footprint_blockers(sim, path_grid, building_id, unit_id);
            }
        }
        BunkerState::TurnToBuilding => {
            if !is_turning(sim, unit_id) {
                if start_install_force_track(sim, rules, building_id, unit_id) {
                    set_state(sim, building_id, BunkerState::TrackStep, Some(unit_id));
                } else {
                    // Already on the install cell: skip the slide, turn South.
                    if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
                        u.facing_target = Some(SOUTH_FACING);
                    }
                    set_state(sim, building_id, BunkerState::TurnSouth, Some(unit_id));
                }
            }
        }
        BunkerState::TrackStep => {
            if !is_moving(sim, unit_id) {
                if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
                    u.facing_target = Some(SOUTH_FACING);
                }
                set_state(sim, building_id, BunkerState::TurnSouth, Some(unit_id));
            }
        }
        BunkerState::TurnSouth => {
            if !is_turning(sim, unit_id) {
                // Walls rise just before the unit hides (health-gated variant).
                crate::sim::docking::bunker_link::emit_bunker_wall_anim(
                    sim,
                    building_id,
                    true,
                    rules,
                );
                crate::sim::docking::bunker_link::install_bunker_link(sim, building_id, unit_id);
            }
        }
        BunkerState::Idle | BunkerState::Occupied => {}
    }
}

fn set_state(sim: &mut Simulation, building_id: u64, state: BunkerState, unit: Option<u64>) {
    if let Some(b) = sim.substrate.entities.get_mut(building_id) {
        if let Some(rt) = b.bunker_runtime.as_mut() {
            rt.state = state;
            rt.installing_unit = unit;
        }
    }
}

fn is_turning(sim: &Simulation, unit_id: u64) -> bool {
    sim.substrate
        .entities
        .get(unit_id)
        .is_some_and(|u| u.facing_target.is_some())
}

fn is_moving(sim: &Simulation, unit_id: u64) -> bool {
    sim.substrate
        .entities
        .get(unit_id)
        .is_some_and(|u| u.forced_drive_track.is_some() || u.movement_target.is_some())
}

/// The candidate is on one of the bunker's footprint cells and not moving.
fn on_footprint_and_stopped(sim: &Simulation, building_id: u64, unit_id: u64) -> bool {
    let Some(building) = sim.substrate.entities.get(building_id) else {
        return false;
    };
    let footprint = entity_occupancy_cells(building);
    let Some(unit) = sim.substrate.entities.get(unit_id) else {
        return false;
    };
    let on = footprint
        .iter()
        .any(|&(cx, cy)| cx == unit.position.rx && cy == unit.position.ry);
    on && unit.movement_target.is_none() && unit.forced_drive_track.is_none()
}

/// No live vehicle/infantry other than the candidate stands on the footprint.
fn footprint_clear_of_others(sim: &Simulation, building_id: u64, unit_id: u64) -> bool {
    let Some(building) = sim.substrate.entities.get(building_id) else {
        return true;
    };
    let footprint = entity_occupancy_cells(building);
    !sim.substrate.entities.iter_sorted().any(|(id, e)| {
        id != unit_id
            && id != building_id
            && e.in_logic_vector
            && matches!(e.category, EntityCategory::Unit | EntityCategory::Infantry)
            && footprint
                .iter()
                .any(|&(cx, cy)| cx == e.position.rx && cy == e.position.ry)
    })
}

/// Issue a Scatter move to every live vehicle/infantry (other than the installer)
/// standing on the bunker footprint, so the install cell clears. Uses the
/// scenario RNG stream (the documented forced-scatter routing). Each blocker
/// walks one cell via normal locomotion; the machine waits in `ClearWait` until
/// the footprint is physically clear.
fn shove_footprint_blockers(
    sim: &mut Simulation,
    path_grid: Option<&PathGrid>,
    building_id: u64,
    unit_id: u64,
) {
    let Some(building) = sim.substrate.entities.get(building_id) else {
        return;
    };
    let footprint = entity_occupancy_cells(building);
    let blockers: Vec<u64> = sim
        .substrate
        .entities
        .iter_sorted()
        .filter(|(id, e)| {
            *id != unit_id
                && *id != building_id
                && e.in_logic_vector
                && matches!(e.category, EntityCategory::Unit | EntityCategory::Infantry)
                && footprint
                    .iter()
                    .any(|&(cx, cy)| cx == e.position.rx && cy == e.position.ry)
        })
        .map(|(id, _)| id)
        .collect();
    for blocker_id in blockers {
        scatter_blocker(
            &mut sim.substrate.entities,
            blocker_id,
            path_grid,
            &sim.substrate.occupancy,
            MovementLayer::Ground,
            &mut sim.scenario_rng,
            // No rules handle on this path; an absent table resolves to the
            // constructed `Scatter=yes` default, matching an unread slot.
            None,
        );
    }
}

/// Body facing from the candidate toward the building anchor; `None` when the
/// candidate already sits on the anchor cell (no turn needed).
fn facing_to_anchor(sim: &Simulation, building_id: u64, unit_id: u64) -> Option<u8> {
    let (bx, by) = sim
        .substrate
        .entities
        .get(building_id)
        .map(|b| (b.position.rx, b.position.ry))?;
    let (ux, uy) = sim
        .substrate
        .entities
        .get(unit_id)
        .map(|u| (u.position.rx, u.position.ry))?;
    let dx = bx as i32 - ux as i32;
    let dy = by as i32 - uy as i32;
    if dx == 0 && dy == 0 {
        None
    } else {
        Some(facing_from_delta(dx, dy))
    }
}

/// Map an 8-bit body facing (toward the building) to one of the 4 diagonal
/// install approach curves (0x43 NE / 0x44 SE / 0x45 SW / 0x46 NW). Exact octant
/// boundaries are an in-game-verify item; state progression is unit-tested.
fn octant_install_track(facing: u8) -> u8 {
    match facing {
        0x00..=0x3F => 0x43,
        0x40..=0x7F => 0x44,
        0x80..=0xBF => 0x45,
        _ => 0x46,
    }
}

/// Begin the sub-cell force-track that slides the candidate onto the install
/// cell (the building anchor). Head offset = cell delta in leptons (no half-cell
/// offset, unlike the refinery exit). Returns `false` when the candidate is
/// already on the anchor cell (no slide needed).
fn start_install_force_track(
    sim: &mut Simulation,
    rules: &RuleSet,
    building_id: u64,
    unit_id: u64,
) -> bool {
    let Some((bx, by, building_sub_x, building_sub_y)) =
        sim.substrate.entities.get(building_id).map(|b| {
            (
                b.position.rx,
                b.position.ry,
                b.position.sub_x.to_num::<i32>(),
                b.position.sub_y.to_num::<i32>(),
            )
        })
    else {
        return false;
    };
    let Some((ux, uy, tref)) = sim
        .substrate
        .entities
        .get(unit_id)
        .map(|u| (u.position.rx, u.position.ry, u.type_ref))
    else {
        return false;
    };
    let dcx = bx as i32 - ux as i32;
    let dcy = by as i32 - uy as i32;
    if dcx == 0 && dcy == 0 {
        return false;
    }
    let facing = facing_from_delta(dcx, dcy);
    let track = octant_install_track(facing);
    let speed_raw = sim.object_type(tref, rules).map(|o| o.speed).unwrap_or(4);
    let speed = crate::util::fixed_math::ra2_speed_to_leptons_per_second(speed_raw.max(1));
    // Native passes the building's exact GetCoords result, not merely its cell
    // origin. Express that absolute head relative to the unit's current origin.
    let Some(forced) = begin_forced_turn_track(
        track,
        dcx * 256 + building_sub_x,
        dcy * 256 + building_sub_y,
        speed,
        false,
    ) else {
        return false;
    };
    let (entities, cell_occupation) = (
        &mut sim.substrate.entities,
        &mut sim.substrate.cell_occupation,
    );
    let Some(unit) = entities.get_mut(unit_id) else {
        return false;
    };
    movement::install_forced_drive_track(unit, cell_occupation, forced)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::world::RevealOutcome;

    fn rules() -> RuleSet {
        RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
            "[VehicleTypes]\n0=TANK\n\n[InfantryTypes]\n\n[AircraftTypes]\n\n\
             [BuildingTypes]\n0=NATBNK\n\n\
             [TANK]\nStrength=400\nArmor=heavy\nSpeed=6\nBunkerable=yes\nPrimary=120mm\n\n\
             [NATBNK]\nStrength=1000\nArmor=heavy\nBunker=yes\n",
        ))
        .expect("rules parse")
    }

    fn spawn_bunker(sim: &mut Simulation, sid: u64) {
        let owner_id = sim.interner.intern("Americans");
        let type_id = sim.interner.intern("NATBNK");
        let mut ge = GameEntity::new_at_frame_zero_for_test(
            sid,
            10,
            10,
            0,
            0,
            owner_id,
            Health {
                current: 1000,
                max: 1000,
            },
            type_id,
            EntityCategory::Structure,
            0,
            5,
            false,
        );
        ge.bunker_runtime = Some(BunkerRuntime::idle());
        sim.substrate.entities.insert(ge);
    }

    fn spawn_tank_on(sim: &mut Simulation, sid: u64, rx: u16, ry: u16) {
        let owner_id = sim.interner.intern("Americans");
        let type_id = sim.interner.intern("TANK");
        let ge = GameEntity::new_at_frame_zero_for_test(
            sid,
            rx,
            ry,
            0,
            0,
            owner_id,
            Health {
                current: 400,
                max: 400,
            },
            type_id,
            EntityCategory::Unit,
            0,
            5,
            true,
        );
        sim.substrate.entities.insert(ge);
    }

    fn rt(sim: &Simulation, id: u64) -> BunkerRuntime {
        sim.substrate
            .entities
            .get(id)
            .unwrap()
            .bunker_runtime
            .unwrap()
    }

    #[test]
    fn octant_track_maps_quadrants() {
        assert_eq!(octant_install_track(0x00), 0x43);
        assert_eq!(octant_install_track(0x40), 0x44);
        assert_eq!(octant_install_track(0x80), 0x45);
        assert_eq!(octant_install_track(0xC0), 0x46);
    }

    #[test]
    fn gsi_04_05_bunker_force_track_stores_exact_building_head_and_premark() {
        let mut sim = Simulation::new();
        let rules = rules();
        spawn_bunker(&mut sim, 2);
        spawn_tank_on(&mut sim, 1, 9, 10);
        {
            let building = sim.substrate.entities.get_mut(2).unwrap();
            building.position.sub_x = crate::util::fixed_math::SimFixed::from_num(96);
            building.position.sub_y = crate::util::fixed_math::SimFixed::from_num(160);
        }
        assert!(matches!(sim.reveal(1), RevealOutcome::Revealed { .. }));

        assert!(start_install_force_track(&mut sim, &rules, 2, 1));
        let unit = sim.substrate.entities.get(1).unwrap();
        let forced = unit
            .forced_drive_track
            .as_ref()
            .expect("bunker Force_Track installed");
        assert_eq!(forced.track.head_offset_x, 256 + 96);
        assert_eq!(forced.track.head_offset_y, 160);
        let exact_head = crate::sim::components::DriveCoord {
            x: 10 * 256 + 96,
            y: 10 * 256 + 160,
            z: 0,
        };
        let drive = unit.drive_locomotion.as_ref().expect("Drive runtime");
        assert_eq!(drive.destination, Some(exact_head));
        assert_eq!(drive.head_to, Some(exact_head));
        assert_eq!(
            drive.occupation_head_to,
            Some(crate::sim::components::DriveOccupationFootprint {
                rx: 10,
                ry: 10,
                layer: MovementLayer::Ground,
            })
        );
        assert!(sim.substrate.occupancy.contains_entity(9, 10, 1));
        assert!(!sim.substrate.occupancy.contains_entity(10, 10, 1));
        assert_eq!(
            sim.substrate
                .cell_occupation
                .vehicle_bits(9, 10, MovementLayer::Ground),
            crate::sim::occupancy::VEHICLE_OCCUPATION_BIT
        );
        assert_eq!(
            sim.substrate
                .cell_occupation
                .vehicle_bits(10, 10, MovementLayer::Ground),
            crate::sim::occupancy::VEHICLE_OCCUPATION_BIT,
            "assignment premarks the exact building-head cell without moving the object list"
        );
    }

    #[test]
    fn install_machine_walks_to_occupied() {
        let mut sim = Simulation::new();
        let rules = rules();
        spawn_bunker(&mut sim, 2);
        spawn_tank_on(&mut sim, 1, 10, 10); // already on the anchor cell
        assert!(matches!(sim.reveal(1), RevealOutcome::Revealed { .. }));
        sim.substrate.entities.get_mut(1).unwrap().bunker_link = BunkerLink::Approaching(2);
        set_state(&mut sim, 2, BunkerState::ArriveWait, Some(1));

        // ArriveWait -> ClearWait (on footprint, stopped).
        tick_bunker_install(&mut sim, &rules, None);
        assert_eq!(rt(&sim, 2).state, BunkerState::ClearWait);

        // ClearWait -> TurnToBuilding (footprint clear; delta 0 => no facing turn).
        tick_bunker_install(&mut sim, &rules, None);
        assert_eq!(rt(&sim, 2).state, BunkerState::TurnToBuilding);

        // TurnToBuilding -> TurnSouth (delta 0 => force-track skipped; faces South).
        tick_bunker_install(&mut sim, &rules, None);
        assert_eq!(rt(&sim, 2).state, BunkerState::TurnSouth);
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().facing_target,
            Some(SOUTH_FACING)
        );

        // Simulate the South turn completing (movement clears facing_target).
        sim.substrate.entities.get_mut(1).unwrap().facing_target = None;

        // TurnSouth -> install.
        tick_bunker_install(&mut sim, &rules, None);
        assert_eq!(rt(&sim, 2).state, BunkerState::Occupied);
        assert_eq!(
            sim.substrate.entities.get(2).unwrap().bunker_occupant,
            Some(1)
        );
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().bunker_link,
            BunkerLink::Installed(2)
        );
        assert!(
            !sim.substrate.entities.get(1).unwrap().in_logic_vector,
            "installed unit is hidden"
        );
        let installed = sim.substrate.entities.get(1).unwrap();
        assert!(installed.lifecycle.object_alive);
        assert!(installed.lifecycle.in_limbo);
        assert!(!installed.lifecycle.cell_marked);
        assert_eq!(
            sim.bunker_wall_events.iter().filter(|e| e.up).count(),
            1,
            "exactly one walls-up anim event on install"
        );
    }

    #[test]
    fn install_aborts_when_candidate_not_approaching() {
        let mut sim = Simulation::new();
        let rules = rules();
        spawn_bunker(&mut sim, 2);
        spawn_tank_on(&mut sim, 1, 10, 10);
        assert!(matches!(sim.reveal(1), RevealOutcome::Revealed { .. }));
        // No Approaching marker (a retask cleared it) → machine resets.
        set_state(&mut sim, 2, BunkerState::ArriveWait, Some(1));
        tick_bunker_install(&mut sim, &rules, None);
        assert_eq!(rt(&sim, 2).state, BunkerState::Idle);
        assert_eq!(rt(&sim, 2).installing_unit, None);
    }

    #[test]
    fn install_shoves_a_footprint_blocker() {
        let mut sim = Simulation::new();
        let rules = rules();
        spawn_bunker(&mut sim, 2);
        spawn_tank_on(&mut sim, 1, 10, 10); // installer on the anchor
        spawn_tank_on(&mut sim, 3, 10, 10); // blocker on the same footprint cell
        assert!(matches!(sim.reveal(1), RevealOutcome::Revealed { .. }));
        assert!(matches!(sim.reveal(3), RevealOutcome::Revealed { .. }));
        sim.substrate.entities.get_mut(1).unwrap().bunker_link = BunkerLink::Approaching(2);
        set_state(&mut sim, 2, BunkerState::ArriveWait, Some(1));

        // ArriveWait -> ClearWait.
        tick_bunker_install(&mut sim, &rules, None);
        assert_eq!(rt(&sim, 2).state, BunkerState::ClearWait);

        // Blocker present → stay in ClearWait and shove it (issue a move).
        tick_bunker_install(&mut sim, &rules, None);
        assert_eq!(
            rt(&sim, 2).state,
            BunkerState::ClearWait,
            "waits while a blocker occupies the footprint"
        );
        assert!(
            sim.substrate
                .entities
                .get(3)
                .unwrap()
                .movement_target
                .is_some(),
            "the footprint blocker was scattered"
        );

        // Simulate the blocker physically leaving the footprint cell.
        {
            let b = sim.substrate.entities.get_mut(3).unwrap();
            b.position.rx = 20;
            b.movement_target = None;
        }

        // Footprint clear now → advance.
        tick_bunker_install(&mut sim, &rules, None);
        assert_eq!(rt(&sim, 2).state, BunkerState::TurnToBuilding);
    }
}
