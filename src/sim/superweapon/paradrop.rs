//! ParaDrop / AmerParaDrop superweapon launch handler.
//!
//! Mirrors gamemd.exe SuperClass::Launch cases 5 (ParaDrop, side-branched on
//! HouseClass.Side) and 6 (AmerParaDrop, always-American config).
//!
//! Per-side branch picks an infantry list from rules; for each (inf_type, num)
//! entry, spawns one PDPLANE at the house's waypoint edge with `num` limbo
//! infantry loaded as cargo. The carrier's initial Rust mission is
//! ParaDropApproach, which models the stock Mission_Open superweapon path.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, sim/aircraft, sim/movement,
//!   sim/passenger, sim/pathfinding, sim/world.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::ruleset::RuleSet;
use crate::sim::aircraft::AircraftMission;
use crate::sim::intern::InternedId;
use crate::sim::movement::air_movement;
use crate::sim::movement::locomotor::AirMovePhase;
use crate::sim::passenger::PassengerRole;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::edge_cell::{Edge, find_paradrop_carrier_edge_cell};
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::fixed_math::{SimFixed, ra2_speed_to_leptons_per_second};

#[derive(Debug, Clone, Copy)]
pub enum ParaDropKind {
    /// Type=ParaDrop — side-branched on HouseClass.side_index.
    Generic,
    /// Type=AmerParaDrop — always uses the AmerParaDropList.
    American,
}

/// Launch entry point. Returns true if at least one carrier was spawned.
pub fn launch(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    target_rx: u16,
    target_ry: u16,
    kind: ParaDropKind,
    _path_grid: Option<&PathGrid>,
) -> bool {
    // Bridge rejection deferred — map system does not yet expose is_bridge_cell.
    let (target_rx, target_ry) = (target_rx, target_ry);

    // Pick the per-side infantry list.
    let list: Vec<(String, u32)> = match kind {
        ParaDropKind::American => rules.general.amer_paradrop_list.clone(),
        ParaDropKind::Generic => {
            let side = sim.houses.get(&owner).map_or(0, |h| h.side_index);
            match side {
                0 => rules.general.ally_paradrop_list.clone(),
                2 => rules.general.yuri_paradrop_list.clone(),
                _ => rules.general.sov_paradrop_list.clone(), // Soviet fallback
            }
        }
    };

    if list.is_empty() {
        log::warn!(
            "Paradrop launch by '{}': per-side list is empty; aborting",
            sim.interner.resolve(owner),
        );
        return false;
    }

    // Resolve the carrier's spawn edge cell.
    let waypoint_edge_idx = sim.houses.get(&owner).map_or(0, |h| h.waypoint_edge);
    let edge = match Edge::from_index(waypoint_edge_idx) {
        Some(e) => e,
        None => {
            log::warn!(
                "Paradrop launch: invalid waypoint_edge {}; falling back to north edge",
                waypoint_edge_idx
            );
            Edge::North
        }
    };
    let edge_cell = match find_paradrop_carrier_edge_cell(
        sim.fog.width,
        sim.fog.height,
        edge,
        (target_rx, target_ry),
    ) {
        Some(c) => c,
        None => {
            log::warn!(
                "Paradrop launch: no carrier edge cell on edge {:?} for target ({},{})",
                edge,
                target_rx,
                target_ry,
            );
            return false;
        }
    };

    // EVA "superweapon launched" voice — same convention as IronCurtain etc.
    sim.sound_events.push(SimSoundEvent::SuperWeaponLaunched {
        owner,
        rx: target_rx,
        ry: target_ry,
    });

    // Spawn one PDPLANE per (inf_type, num) entry.
    let mut spawned_any = false;
    for (inf_type_name, num) in list {
        if spawn_pdplane(
            sim,
            rules,
            owner,
            edge_cell,
            target_rx,
            target_ry,
            &inf_type_name,
            num,
        ) {
            spawned_any = true;
        }
    }
    spawned_any
}

fn spawn_pdplane(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    edge_cell: (u16, u16),
    target_rx: u16,
    target_ry: u16,
    inf_type: &str,
    num: u32,
) -> bool {
    let owner_str = sim.interner.resolve(owner).to_string();
    let pdplane_type = rules.general.paradrop_aircraft_type.clone();

    // Spawn the carrier at ground z; ascent ramp is skipped (parity drift S8).
    let pdplane_id = match sim.spawn_object_at_height(
        &pdplane_type,
        &owner_str,
        edge_cell.0,
        edge_cell.1,
        /*facing*/ 0,
        /*z*/ 0,
        rules,
    ) {
        Some(id) => id,
        None => {
            log::warn!(
                "Paradrop spawn: failed to create carrier '{}' at edge ({},{})",
                pdplane_type,
                edge_cell.0,
                edge_cell.1,
            );
            return false;
        }
    };

    // Jump straight to cruise altitude AND install a cargo hold sized for the
    // paradrop payload. Vanilla PDPLANE has no Passengers= key, so spawn_object
    // doesn't initialize a Transport cargo — gamemd's spawner builds the
    // CargoClass linked list directly. Mirror that here.
    let flight_level = SimFixed::from_num(rules.general.flight_level);
    if let Some(entity) = sim.entities.get_mut(pdplane_id) {
        if let Some(loco) = entity.locomotor.as_mut() {
            loco.altitude = flight_level;
            loco.target_altitude = flight_level;
            loco.air_phase = AirMovePhase::Cruising;
        }
        if !entity.passenger_role.is_transport() {
            entity.passenger_role = PassengerRole::Transport {
                cargo: crate::sim::passenger::PassengerCargo::new(num, /*size_limit*/ 0),
            };
        }
    }

    // Load N limbo-created infantry into cargo as Inside passengers.
    let inf_size = rules.object(inf_type).map(|o| o.size).unwrap_or(1);
    let mut loaded = 0u32;
    for _ in 0..num {
        let pax_id = match sim.spawn_object_limbo_at_height(
            inf_type,
            &owner_str,
            edge_cell.0,
            edge_cell.1,
            /*facing*/ 0,
            /*z*/ 0,
            rules,
        ) {
            Some(id) => id,
            None => break,
        };
        if let Some(pax) = sim.entities.get_mut(pax_id) {
            pax.passenger_role = PassengerRole::Inside {
                transport_id: pdplane_id,
            };
        }
        let boarded = sim
            .entities
            .get_mut(pdplane_id)
            .and_then(|a| a.passenger_role.cargo_mut())
            .map(|c| {
                c.board_forced(pax_id, inf_size);
                true
            })
            .unwrap_or(false);
        if !boarded {
            // No hold - give up; the partial cargo flies.
            break;
        }
        loaded += 1;
    }

    if loaded == 0 {
        // No passengers loaded — kill the empty carrier rather than fly empty.
        if let Some(entity) = sim.entities.get_mut(pdplane_id) {
            entity.health.current = 0;
            entity.dying = true;
        }
        return false;
    }

    // Set initial mission to the Open-equivalent paradrop state and issue
    // movement to target.
    if let Some(entity) = sim.entities.get_mut(pdplane_id) {
        entity.aircraft_mission = Some(AircraftMission::ParaDropApproach {
            target_rx,
            target_ry,
            has_revealed_fog: false,
        });
    }
    let speed = rules
        .object(&pdplane_type)
        .map(|o| ra2_speed_to_leptons_per_second(o.speed.max(1)))
        .unwrap_or(SimFixed::from_num(8));
    air_movement::issue_air_move_command(
        &mut sim.entities,
        pdplane_id,
        (target_rx, target_ry),
        speed,
    );

    log::info!(
        "Paradrop: spawned '{}' for '{}' carrying {} '{}' at edge ({},{}) → target ({},{})",
        pdplane_type,
        owner_str,
        loaded,
        inf_type,
        edge_cell.0,
        edge_cell.1,
        target_rx,
        target_ry,
    );
    true
}
