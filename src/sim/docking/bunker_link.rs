//! Tank-bunker reciprocal link helpers (install / break / release trio).
//!
//! Owns the writes to both sides of the bunker link (`GameEntity.bunker_link` on
//! the unit, `GameEntity.bunker_occupant` on the building) plus the three distinct
//! teardown helpers and the admission predicate.
//!
//! sim/ only — never render/ui/sidebar/audio/net.
use crate::rules::ruleset::RuleSet;
use crate::sim::docking::bunker_install::{BunkerRuntime, BunkerState};
use crate::sim::game_entity::BunkerLink;
use crate::sim::mission::{verb, MissionType};
use crate::sim::pathfinding::PathGrid;
use crate::sim::radio::{transmit, RadioMessage, RadioPayload};
use crate::sim::world::{SimSoundEvent, Simulation};

/// Exit-search ring limit for the normal release (mirrors the refinery exit).
const BUNKER_EXIT_SEARCH_MAX_RADIUS: i32 = 16;

/// The per-unit half of the bunker admission gate: a Bunkerable vehicle that has
/// a primary weapon. (Movement-zone / busy-guard sub-checks are not reproduced —
/// they exclude no stock bunkerable vehicle.) Resolved against `rules`, so this is
/// called from the command dispatch (which has `rules`), never from the radio bus.
pub fn can_auto_deploy_here(sim: &Simulation, unit_id: u64, rules: &RuleSet) -> bool {
    let Some(unit) = sim.substrate.entities.get(unit_id) else {
        return false;
    };
    let Some(obj) = sim.object_type(unit.type_ref, rules) else {
        return false;
    };
    obj.bunkerable && obj.primary.is_some()
}

/// Install: write both reciprocal links, clear the unit's pending navigation,
/// hide it (full conceal — combat/render is deferred), set Guard mission, and
/// emit the wall-up sound. Entry anims are emitted by the install machine just
/// before this call.
pub fn install_bunker_link(sim: &mut Simulation, building_id: u64, unit_id: u64) {
    // Building side first: occupant pointer + state → Occupied.
    if let Some(b) = sim.substrate.entities.get_mut(building_id) {
        b.bunker_occupant = Some(unit_id);
        if let Some(rt) = b.bunker_runtime.as_mut() {
            rt.state = BunkerState::Occupied;
            rt.installing_unit = None;
        }
    }
    let now = sim.binary_frame;
    if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
        u.bunker_link = BunkerLink::Installed(building_id);
        // Clear any pending navigation/turn so the hidden unit holds no stale
        // destination.
        u.movement_target = None;
        u.facing_target = None;
        u.forced_drive_track = None;
        verb::assign_mission(&mut u.mission, MissionType::Guard, now);
    }
    // Hide: drop cell occupancy + leave the active set.
    sim.remove_entity_occupancy(unit_id);
    sim.conceal(unit_id);
    emit_bunker_wall_sound(sim, building_id, true);
}

/// Clear BOTH sides of the link and send the radio BREAK. Returns the unit id
/// that was installed (for callers that re-place it). Does NOT reveal/place/anim.
pub fn break_bunker_link(sim: &mut Simulation, building_id: u64) -> Option<u64> {
    let unit_id = sim.substrate.entities.get(building_id)?.bunker_occupant?;
    // BREAK over the bus (clears any bus-level radio contact both ways).
    transmit(
        sim,
        building_id,
        unit_id,
        RadioMessage::Break,
        RadioPayload::default(),
    );
    if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
        u.bunker_link = BunkerLink::None;
    }
    if let Some(b) = sim.substrate.entities.get_mut(building_id) {
        b.bunker_occupant = None;
    }
    Some(unit_id)
}

/// Emit the positional wall sound event (`up` = walls rising on install,
/// `false` = walls falling on a teardown). The app layer resolves the actual
/// sound id and skips it when the rules key is empty.
pub(crate) fn emit_bunker_wall_sound(sim: &mut Simulation, building_id: u64, up: bool) {
    let Some(b) = sim.substrate.entities.get(building_id) else {
        return;
    };
    let (rx, ry) = (b.position.rx, b.position.ry);
    sim.sound_events.push(if up {
        SimSoundEvent::BunkerWallsUp { rx, ry }
    } else {
        SimSoundEvent::BunkerWallsDown { rx, ry }
    });
}

/// Normal eject (player EjectBunker): walls-down sound + exit anim, clear both
/// links, reveal the unit at a passable cell near the bunker's SW corner, and
/// order it to Move. Resets the bunker to Idle.
pub fn release_normal(
    sim: &mut Simulation,
    building_id: u64,
    rules: &RuleSet,
    grid: Option<&PathGrid>,
) {
    let _ = rules; // the health-gated exit anim wires this in a later slice.
    emit_bunker_wall_anim(sim, building_id, false);
    emit_bunker_wall_sound(sim, building_id, false);
    let cell = bunker_exit_cell(sim, building_id, grid);
    let Some(unit_id) = break_bunker_link(sim, building_id) else {
        reset_bunker_idle(sim, building_id);
        return;
    };
    let now = sim.binary_frame;
    if let Some((rx, ry)) = cell {
        if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
            u.position.rx = rx;
            u.position.ry = ry;
        }
        sim.reveal(unit_id);
        sim.add_entity_occupancy(unit_id);
    }
    if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
        verb::assign_mission(&mut u.mission, MissionType::Move, now);
    }
    reset_bunker_idle(sim, building_id);
}

/// Sell/destroy teardown (UndockUnit): NO sound/anim; clear both links; reveal
/// the unit at the building cell, idle (no Move, no nearby-passable search). The
/// full-conceal hide model requires the reveal+place here to reproduce gamemd's
/// "unit at the building cell" visible result (gamemd's hide is light).
pub fn release_sell_destroy(sim: &mut Simulation, building_id: u64) {
    let Some((brx, bry)) = sim
        .substrate
        .entities
        .get(building_id)
        .map(|b| (b.position.rx, b.position.ry))
    else {
        return;
    };
    let Some(unit_id) = break_bunker_link(sim, building_id) else {
        return;
    };
    if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
        u.position.rx = brx;
        u.position.ry = bry;
        // South per facing convention; gamemd UndockUnit head, no orderly move.
        u.facing = 0x80;
    }
    sim.reveal(unit_id);
    sim.add_entity_occupancy(unit_id);
    // No Move mission, no sound, no anims (matches UndockUnit).
}

/// Clear-only teardown (super / temporal-non-building / unit death). Clears both
/// links + plays the down sound/anim when occupied, but does NOT reposition the
/// unit. Implemented for contract completeness; no live trigger exists in this
/// slice (prerequisite systems absent + the concealed unit is not damageable
/// until the combat slice).
pub fn release_clear(sim: &mut Simulation, building_id: u64) {
    if sim
        .substrate
        .entities
        .get(building_id)
        .and_then(|b| b.bunker_occupant)
        .is_some()
    {
        emit_bunker_wall_anim(sim, building_id, false);
        emit_bunker_wall_sound(sim, building_id, false);
        break_bunker_link(sim, building_id);
    }
    reset_bunker_idle(sim, building_id);
}

/// Despawn safety net: if `id` is a bunker with an occupant, or a unit installed
/// in a bunker, clear the reciprocal side. No anims/sound/placement.
pub fn break_links_on_despawn(sim: &mut Simulation, id: u64) {
    let Some(e) = sim.substrate.entities.get(id) else {
        return;
    };
    let occupant = e.bunker_occupant;
    let host = e.bunker_link.installed_in();
    if let Some(unit_id) = occupant {
        if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
            u.bunker_link = BunkerLink::None;
        }
    }
    if let Some(building_id) = host {
        if let Some(b) = sim.substrate.entities.get_mut(building_id) {
            b.bunker_occupant = None;
            if let Some(rt) = b.bunker_runtime.as_mut() {
                *rt = BunkerRuntime::idle();
            }
        }
    }
}

/// Reset the bunker runtime to empty/idle. The building's `mission` is never
/// used to track install in this model (`bunker_runtime.state` is the machine),
/// so only the runtime needs resetting on release.
fn reset_bunker_idle(sim: &mut Simulation, building_id: u64) {
    if let Some(b) = sim.substrate.entities.get_mut(building_id) {
        if let Some(rt) = b.bunker_runtime.as_mut() {
            *rt = BunkerRuntime::idle();
        }
    }
}

/// gamemd exit anchor for the normal release: building NW corner + (-1 west,
/// +1 south), then the nearest passable cell (modulo-spread pick). Falls back to
/// the anchor cell when no path grid is available (headless tests).
fn bunker_exit_cell(
    sim: &Simulation,
    building_id: u64,
    grid: Option<&PathGrid>,
) -> Option<(u16, u16)> {
    let b = sim.substrate.entities.get(building_id)?;
    let ax = b.position.rx as i32 - 1;
    let ay = b.position.ry as i32 + 1;
    match grid {
        Some(g) => crate::sim::miner::find_nearby_passable_cell_with_index(
            ax,
            ay,
            g,
            Some(&sim.substrate.occupancy),
            BUNKER_EXIT_SEARCH_MAX_RADIUS,
            sim.binary_frame as u64,
        ),
        None if ax >= 0 && ay >= 0 => Some((ax as u16, ay as u16)),
        None => Some((b.position.rx, b.position.ry)),
    }
}

/// Wall anim event emitter. No-op shim until the wall-anim event slice wires the
/// sim→app event vec + overlay consumer.
fn emit_bunker_wall_anim(sim: &mut Simulation, building_id: u64, up: bool) {
    let _ = (sim, building_id, up);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::components::Health;
    use crate::sim::docking::bunker_install::BunkerRuntime;
    use crate::sim::game_entity::{GameEntity, Presence};

    fn rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[VehicleTypes]\n0=TANK\n1=NOGUN\n\n[InfantryTypes]\n\n[AircraftTypes]\n\n\
             [BuildingTypes]\n0=NATBNK\n\n\
             [TANK]\nStrength=400\nArmor=heavy\nSpeed=6\nBunkerable=yes\nPrimary=120mm\n\n\
             [NOGUN]\nStrength=400\nArmor=heavy\nSpeed=6\nBunkerable=yes\n\n\
             [NATBNK]\nStrength=1000\nArmor=heavy\nBunker=yes\n",
        ))
        .expect("rules parse")
    }

    fn spawn_bunker(sim: &mut Simulation, sid: u64, owner: &str) {
        let owner_id = sim.interner.intern(owner);
        let type_id = sim.interner.intern("NATBNK");
        let mut ge = GameEntity::new(
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

    fn spawn_tank(sim: &mut Simulation, sid: u64, owner: &str, type_name: &str) {
        let owner_id = sim.interner.intern(owner);
        let type_id = sim.interner.intern(type_name);
        let ge = GameEntity::new(
            sid,
            12,
            12,
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

    #[test]
    fn can_auto_deploy_requires_bunkerable_and_a_weapon() {
        let mut sim = Simulation::new();
        let rules = rules();
        spawn_tank(&mut sim, 1, "Americans", "TANK"); // bunkerable + primary
        spawn_tank(&mut sim, 2, "Americans", "NOGUN"); // bunkerable, no primary
        assert!(can_auto_deploy_here(&sim, 1, &rules));
        assert!(!can_auto_deploy_here(&sim, 2, &rules));
    }

    #[test]
    fn install_writes_both_sides_hides_unit_and_emits_up_sound() {
        let mut sim = Simulation::new();
        spawn_bunker(&mut sim, 2, "Americans");
        spawn_tank(&mut sim, 1, "Americans", "TANK");
        // The unit must be a live (InCell) member before it can be concealed.
        sim.reveal(1);
        sim.add_entity_occupancy(1);

        install_bunker_link(&mut sim, 2, 1);

        let bunker = sim.substrate.entities.get(2).unwrap();
        assert_eq!(bunker.bunker_occupant, Some(1));
        assert_eq!(bunker.bunker_runtime.unwrap().state, BunkerState::Occupied);

        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!(unit.bunker_link, BunkerLink::Installed(2));
        assert!(!unit.in_logic_vector, "installed unit left the active set");
        assert_eq!(unit.presence, Presence::Limbo);
        assert_eq!(unit.mission.current, MissionType::Guard);

        let up = sim
            .sound_events
            .iter()
            .filter(|e| matches!(e, SimSoundEvent::BunkerWallsUp { .. }))
            .count();
        assert_eq!(up, 1, "exactly one walls-up sound on install");
    }

    #[test]
    fn break_clears_both_sides_and_returns_unit() {
        let mut sim = Simulation::new();
        spawn_bunker(&mut sim, 2, "Americans");
        spawn_tank(&mut sim, 1, "Americans", "TANK");
        sim.reveal(1);
        sim.add_entity_occupancy(1);
        install_bunker_link(&mut sim, 2, 1);

        let released = break_bunker_link(&mut sim, 2);
        assert_eq!(released, Some(1));
        assert_eq!(sim.substrate.entities.get(2).unwrap().bunker_occupant, None);
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().bunker_link,
            BunkerLink::None
        );
    }

    fn installed_sim() -> Simulation {
        let mut sim = Simulation::new();
        spawn_bunker(&mut sim, 2, "Americans");
        spawn_tank(&mut sim, 1, "Americans", "TANK");
        sim.reveal(1);
        sim.add_entity_occupancy(1);
        install_bunker_link(&mut sim, 2, 1);
        sim.sound_events.clear(); // drop the install up-sound; assert on down events
        sim
    }

    fn down_sounds(sim: &Simulation) -> usize {
        sim.sound_events
            .iter()
            .filter(|e| matches!(e, SimSoundEvent::BunkerWallsDown { .. }))
            .count()
    }

    #[test]
    fn release_normal_reveals_near_bunker_moves_and_plays_down_sound() {
        let mut sim = installed_sim();
        release_normal(&mut sim, 2, &rules(), None);
        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!(unit.bunker_link, BunkerLink::None);
        assert!(unit.in_logic_vector, "unit revealed");
        // anchor = building NW (10,10) + (-1,+1) = (9,11)
        assert_eq!((unit.position.rx, unit.position.ry), (9, 11));
        assert_eq!(unit.mission.current, MissionType::Move);
        assert_eq!(sim.substrate.entities.get(2).unwrap().bunker_occupant, None);
        assert_eq!(down_sounds(&sim), 1, "one walls-down on normal eject");
    }

    #[test]
    fn release_sell_destroy_places_at_building_cell_no_move_no_sound() {
        let mut sim = installed_sim();
        release_sell_destroy(&mut sim, 2);
        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!(unit.bunker_link, BunkerLink::None);
        assert!(unit.in_logic_vector, "unit revealed at building cell");
        assert_eq!((unit.position.rx, unit.position.ry), (10, 10));
        assert_eq!(unit.facing, 0x80);
        assert_eq!(unit.mission.current, MissionType::Guard, "no Move order");
        assert_eq!(down_sounds(&sim), 0, "sell teardown is silent");
    }

    #[test]
    fn release_clear_plays_down_sound_but_does_not_reposition() {
        let mut sim = installed_sim();
        release_clear(&mut sim, 2);
        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!(unit.bunker_link, BunkerLink::None);
        assert!(!unit.in_logic_vector, "clear does not reveal");
        assert_eq!(
            (unit.position.rx, unit.position.ry),
            (12, 12),
            "not repositioned"
        );
        assert_eq!(down_sounds(&sim), 1, "one walls-down on clear");
        assert_eq!(sim.substrate.entities.get(2).unwrap().bunker_occupant, None);
    }

    #[test]
    fn despawn_safety_net_clears_surviving_side() {
        // Building despawns → the occupant unit's link is cleared.
        let mut sim = installed_sim();
        break_links_on_despawn(&mut sim, 2);
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().bunker_link,
            BunkerLink::None
        );

        // Unit despawns → the surviving building's back-pointer is cleared.
        let mut sim = installed_sim();
        break_links_on_despawn(&mut sim, 1);
        assert_eq!(sim.substrate.entities.get(2).unwrap().bunker_occupant, None);
    }
}
