//! Carrier-aircraft paradrop mission handlers — stock SW Open + Rescue equivalents.
//!
//! The enum variants keep their older Rust names for save compatibility, but
//! standard superweapon PDPLANE behavior is the gamemd Mission_Open (0x1A) →
//! Mission_Rescue (0x1B) chain, not binary Mission_ParaDropApproach/Overfly.
//!
//! Open-equivalent: flies in toward target. When distance ≤ ParadropRadius,
//! queues Rescue-equivalent after the verified 3-game-frame return delay.
//!
//! Rescue-equivalent: calls Drop_Payload once per execution and reschedules at
//! the 5-game-frame Mission_Rescue cadence. When cargo empty, redirects to the
//! opposite-edge exit cell and silently despawns at the boundary.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, sim/aircraft, sim/intern, sim/world.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::ruleset::RuleSet;
use crate::sim::aircraft::AircraftMission;
use crate::sim::intern::InternedId;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;
use crate::sim::world::edge_cell::{Edge, find_passable_at_edge};

/// Mission_Open returns 3 game frames before Mission_Rescue executes.
/// Current sim convention is 3 sim ticks per gamemd frame.
pub const PARADROP_OPEN_TO_RESCUE_DELAY_TICKS: u16 = 9;

/// Per-tick outcome for the Open-equivalent state. Caller (the aircraft tick)
/// applies these mutations in the apply phase.
pub struct ApproachOutcome {
    pub new_mission: AircraftMission,
    pub fire_fog_reveal: bool,
    pub play_chute_sound: bool,
    pub move_to: Option<(u16, u16)>,
}

pub fn tick_approach(
    sim: &Simulation,
    rules: &RuleSet,
    aircraft_id: u64,
    target_rx: u16,
    target_ry: u16,
    has_revealed_fog: bool,
    path_grid: Option<&PathGrid>,
) -> ApproachOutcome {
    let aircraft = match sim.entities.get(aircraft_id) {
        Some(e) => e,
        None => {
            return ApproachOutcome {
                new_mission: AircraftMission::Idle,
                fire_fog_reveal: false,
                play_chute_sound: false,
                move_to: None,
            };
        }
    };

    let cargo_count = aircraft.passenger_role.cargo().map_or(0, |c| c.count());

    // Cargo empty mid-approach → abort.
    if cargo_count == 0 {
        return ApproachOutcome {
            new_mission: AircraftMission::Idle,
            fire_fog_reveal: false,
            play_chute_sound: false,
            move_to: None,
        };
    }

    // Chebyshev distance × 256 leptons/cell as the approximation for gamemd's
    // building-padded 3D distance. Acceptable starting point; flag if parity
    // drift becomes visible during /fidelity-check.
    let dx = (aircraft.position.rx as i32 - target_rx as i32).abs();
    let dy = (aircraft.position.ry as i32 - target_ry as i32).abs();
    let dist_leptons = dx.max(dy) * 256;

    let radius = rules.general.paradrop_radius;

    // Mission_Open only queues Mission_Rescue here; Drop_Payload owns the
    // successful-drop sound/reveal side effects.
    if dist_leptons <= radius {
        let exit = compute_exit_cell(sim, aircraft.owner, target_rx, target_ry, path_grid);
        return ApproachOutcome {
            new_mission: AircraftMission::ParaDropOverfly {
                exit_rx: exit.0,
                exit_ry: exit.1,
                drop_cooldown: PARADROP_OPEN_TO_RESCUE_DELAY_TICKS,
                landing_state: 0,
                payload_count: cargo_count as u8,
            },
            fire_fog_reveal: false,
            play_chute_sound: false,
            move_to: Some(exit),
        };
    }

    // Still approaching — keep flying toward target.
    ApproachOutcome {
        new_mission: AircraftMission::ParaDropApproach {
            target_rx,
            target_ry,
            has_revealed_fog,
        },
        fire_fog_reveal: false,
        play_chute_sound: false,
        move_to: if aircraft.movement_target.is_none() {
            Some((target_rx, target_ry))
        } else {
            None
        },
    }
}

/// Per-tick outcome for the Rescue-equivalent state.
pub struct OverflyOutcome {
    pub new_mission: AircraftMission,
    pub move_to: Option<(u16, u16)>,
    /// True when caller should invoke drop_payload::try_drop in the apply phase.
    pub try_drop: bool,
    /// Payload count BEFORE decrement (try_drop consumes this).
    pub payload_count_pre_dec: u8,
    /// True when aircraft has reached an exit/boundary and should be despawned silently.
    pub silent_despawn: bool,
}

pub fn tick_overfly(
    sim: &Simulation,
    aircraft_id: u64,
    exit_rx: u16,
    exit_ry: u16,
    drop_cooldown: u16,
    landing_state: u8,
    payload_count: u8,
) -> OverflyOutcome {
    let aircraft = match sim.entities.get(aircraft_id) {
        Some(e) => e,
        None => {
            return OverflyOutcome {
                new_mission: AircraftMission::Idle,
                move_to: None,
                try_drop: false,
                payload_count_pre_dec: 0,
                silent_despawn: false,
            };
        }
    };

    let cargo_count = aircraft.passenger_role.cargo().map_or(0, |c| c.count());
    let cargo_empty = cargo_count == 0;

    let new_cooldown = drop_cooldown.saturating_sub(1);
    let new_landing = landing_state.saturating_sub(1);

    // P19: cargo empty → fly to exit; despawn at boundary.
    if cargo_empty {
        let map_w = sim.fog.width;
        let map_h = sim.fog.height;
        let at_exit = aircraft.position.rx == exit_rx && aircraft.position.ry == exit_ry;
        let despawn = at_exit
            || aircraft.position.rx == 0
            || aircraft.position.ry == 0
            || aircraft.position.rx + 1 >= map_w
            || aircraft.position.ry + 1 >= map_h;
        return OverflyOutcome {
            new_mission: AircraftMission::ParaDropOverfly {
                exit_rx,
                exit_ry,
                drop_cooldown: new_cooldown,
                landing_state: new_landing,
                payload_count,
            },
            move_to: if !despawn && aircraft.movement_target.is_none() {
                Some((exit_rx, exit_ry))
            } else {
                None
            },
            try_drop: false,
            payload_count_pre_dec: payload_count,
            silent_despawn: despawn,
        };
    }

    // Mission_Rescue drops once per execution and returns 5 game frames. It
    // does not check LandingState on the in-range drop branch, so the mirrored
    // landing byte must not add another throttle on top of drop_cooldown.
    let can_drop = new_cooldown == 0;

    OverflyOutcome {
        new_mission: AircraftMission::ParaDropOverfly {
            exit_rx,
            exit_ry,
            drop_cooldown: new_cooldown,
            landing_state: new_landing,
            payload_count,
        },
        move_to: None,
        try_drop: can_drop,
        payload_count_pre_dec: payload_count,
        silent_despawn: false,
    }
}

/// Resolve the opposite-edge exit cell for the carrier aircraft.
/// Encoding: waypoint_edge → opposite via +2 mod 4 (P12).
///
/// Fallback chain when no passable opposite-edge cell exists:
///   1. Try the opposite edge.
///   2. Try the South edge as a generic fallback.
///   3. Fall back to a playfield corner — forces deterministic despawn
///      via the boundary check rather than looping at the target.
pub fn compute_exit_cell(
    sim: &Simulation,
    owner: InternedId,
    target_rx: u16,
    target_ry: u16,
    path_grid: Option<&PathGrid>,
) -> (u16, u16) {
    let waypoint_edge = sim.houses.get(&owner).map_or(0, |h| h.waypoint_edge);
    let opposite_idx = (waypoint_edge + 2) % 4;
    let exit_edge = Edge::from_index(opposite_idx).unwrap_or(Edge::South);

    let map_w = sim.fog.width;
    let map_h = sim.fog.height;
    let target = (target_rx, target_ry);

    if let Some(grid) = path_grid {
        if let Some(cell) = find_passable_at_edge(grid, map_w, map_h, exit_edge, target) {
            return cell;
        }
        if exit_edge != Edge::South {
            if let Some(cell) = find_passable_at_edge(grid, map_w, map_h, Edge::South, target) {
                return cell;
            }
        }
    }
    // Final fallback: playfield corner forces silent_despawn boundary check.
    (map_w.saturating_sub(1), map_h.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::passenger::{PassengerCargo, PassengerRole};

    fn paradrop_rules(radius: i32) -> RuleSet {
        let ini = IniFile::from_str(&format!(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             0=PDPLANE\n\
             [BuildingTypes]\n\
             [General]\n\
             ParadropRadius={radius}\n\
             [PDPLANE]\n\
             Name=Paradrop Plane\n\
             Strength=400\n"
        ));
        RuleSet::from_ini(&ini).expect("paradrop mission test rules should parse")
    }

    fn sim_with_loaded_pdplane(rx: u16, ry: u16, cargo_count: u32) -> (Simulation, u64) {
        let mut sim = Simulation::new();
        sim.fog.width = 100;
        sim.fog.height = 100;
        let owner = sim.interner.intern("Americans");
        let mut aircraft = GameEntity::test_default(1, "PDPLANE", "Americans", rx, ry);
        aircraft.owner = owner;
        aircraft.type_ref = sim.interner.intern("PDPLANE");
        let mut cargo = PassengerCargo::new(cargo_count, 0);
        for id in 2..(2 + u64::from(cargo_count)) {
            cargo.passengers.push(id);
        }
        cargo.total_size = cargo_count;
        aircraft.passenger_role = PassengerRole::Transport { cargo };
        sim.entities.insert(aircraft);
        (sim, 1)
    }

    #[test]
    fn test_chebyshev_threshold_arithmetic() {
        // ParadropRadius=1024 → exactly 4 cells triggers transition.
        let radius = 1024i32;
        assert!(4i32 * 256 <= radius);
        assert!(5i32 * 256 > radius);
    }

    #[test]
    fn test_opposite_edge_indices() {
        // 0=N → 2=S, 1=E → 3=W, 2=S → 0=N, 3=W → 1=E.
        assert_eq!((0u8 + 2) % 4, 2);
        assert_eq!((1u8 + 2) % 4, 3);
        assert_eq!((2u8 + 2) % 4, 0);
        assert_eq!((3u8 + 2) % 4, 1);
    }

    #[test]
    fn open_equivalent_enters_rescue_after_verified_open_delay() {
        let rules = paradrop_rules(1024);
        let (sim, aircraft_id) = sim_with_loaded_pdplane(10, 10, 4);

        let outcome = tick_approach(&sim, &rules, aircraft_id, 14, 10, false, None);

        match outcome.new_mission {
            AircraftMission::ParaDropOverfly {
                drop_cooldown,
                landing_state,
                payload_count,
                ..
            } => {
                assert_eq!(drop_cooldown, PARADROP_OPEN_TO_RESCUE_DELAY_TICKS);
                assert_eq!(landing_state, 0);
                assert_eq!(payload_count, 4);
            }
            other => panic!("expected Rescue-equivalent state, got {:?}", other),
        }
        assert!(!outcome.fire_fog_reveal);
        assert!(!outcome.play_chute_sound);
        assert_eq!(outcome.move_to, Some((99, 99)));
    }

    #[test]
    fn rescue_equivalent_does_not_use_landing_state_as_extra_drop_throttle() {
        let (sim, aircraft_id) = sim_with_loaded_pdplane(10, 10, 1);

        let outcome = tick_overfly(&sim, aircraft_id, 99, 99, 0, 5, 1);

        assert!(
            outcome.try_drop,
            "LandingState should not delay an in-range Rescue-equivalent drop"
        );
        match outcome.new_mission {
            AircraftMission::ParaDropOverfly {
                drop_cooldown,
                landing_state,
                ..
            } => {
                assert_eq!(drop_cooldown, 0);
                assert_eq!(landing_state, 4);
            }
            other => panic!("expected Rescue-equivalent state, got {:?}", other),
        }
    }

    #[test]
    fn rescue_equivalent_respects_mission_cadence_cooldown() {
        let (sim, aircraft_id) = sim_with_loaded_pdplane(10, 10, 1);

        let outcome = tick_overfly(&sim, aircraft_id, 99, 99, 2, 0, 1);

        assert!(!outcome.try_drop);
        match outcome.new_mission {
            AircraftMission::ParaDropOverfly { drop_cooldown, .. } => {
                assert_eq!(drop_cooldown, 1);
            }
            other => panic!("expected Rescue-equivalent state, got {:?}", other),
        }
    }
}
