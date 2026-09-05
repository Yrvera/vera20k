//! World-side gates for the combat-produced EVA inputs:
//! `TechnoClass::Death_Announcement @ 0x004D98C0` (owner + radar type 7) and
//! `HouseClass::NotifyUnderAttack @ 0x004F93E0` (own line, ally line).

use super::{SimSoundEvent, Simulation};
use crate::sim::combat::{UnderAttackEvent, UnitLostEvent};
use crate::sim::house_state::HouseState;
use crate::sim::intern::InternedId;

fn house(sim: &mut Simulation, name: &str, human: bool) -> InternedId {
    let id = sim.interner.intern(name);
    sim.houses
        .insert(id, HouseState::new(id, 0, Some(id), human, 5_000, 10));
    sim.session.house_order.push(id);
    id
}

fn ally(sim: &mut Simulation, asker: &str, other: &str) {
    sim.house_alliances
        .entry(asker.to_ascii_uppercase())
        .or_default()
        .insert(other.to_ascii_uppercase());
}

fn unit_lost_owners(sim: &mut Simulation) -> Vec<InternedId> {
    sim.sound_events
        .drain(..)
        .filter_map(|event| match event {
            SimSoundEvent::UnitLost { owner } => Some(owner),
            _ => None,
        })
        .collect()
}

fn ally_line_owners(sim: &mut Simulation) -> Vec<InternedId> {
    sim.sound_events
        .drain(..)
        .filter_map(|event| match event {
            SimSoundEvent::AllyUnderAttack { owner } => Some(owner),
            _ => None,
        })
        .collect()
}

/// `0x004D98CA` owner gate, then `CreateRadarEvent(7)` (`0x004D98FE`): type 7
/// dedupes within 8 cells while the previous diamond is alive (200 frames).
#[test]
fn unit_lost_announces_for_human_owners_deduped_by_the_radar_window() {
    let mut sim = Simulation::new();
    let human = house(&mut sim, "Americans", true);
    let ai = house(&mut sim, "Russians", false);

    sim.dispatch_unit_lost_events(&[UnitLostEvent {
        rx: 20,
        ry: 20,
        owner: ai,
    }]);
    assert!(
        unit_lost_owners(&mut sim).is_empty(),
        "AI deaths are silent"
    );

    sim.dispatch_unit_lost_events(&[UnitLostEvent {
        rx: 20,
        ry: 20,
        owner: human,
    }]);
    assert_eq!(unit_lost_owners(&mut sim), vec![human]);

    // A second death 5 cells away inside the window: suppressed.
    sim.dispatch_unit_lost_events(&[UnitLostEvent {
        rx: 25,
        ry: 20,
        owner: human,
    }]);
    assert!(unit_lost_owners(&mut sim).is_empty());

    // 8 cells away: a new diamond, a new line.
    sim.dispatch_unit_lost_events(&[UnitLostEvent {
        rx: 28,
        ry: 20,
        owner: human,
    }]);
    assert_eq!(unit_lost_owners(&mut sim), vec![human]);

    // After the 200-frame blink the first diamond expires and the cell
    // announces again.
    for _ in 0..200 {
        sim.radar_events.tick();
    }
    sim.dispatch_unit_lost_events(&[UnitLostEvent {
        rx: 20,
        ry: 20,
        owner: human,
    }]);
    assert_eq!(unit_lost_owners(&mut sim), vec![human]);
}

/// The own line carries the radar accept as `eva_allowed` and nothing else
/// limits it: two bases 8+ cells apart in the same tick both announce.
#[test]
fn under_attack_own_line_has_no_cooldown_beyond_the_radar_dedupe() {
    let mut sim = Simulation::new();
    let human = house(&mut sim, "Americans", true);
    let ping = |rx: u16| UnderAttackEvent {
        rx,
        ry: 10,
        owner: human,
        miner: false,
        structure: true,
    };
    sim.dispatch_under_attack_events(&[ping(10), ping(12), ping(30)]);
    let allowed: Vec<bool> = sim
        .sound_events
        .drain(..)
        .filter_map(|event| match event {
            SimSoundEvent::UnderAttack { eva_allowed, .. } => Some(eva_allowed),
            _ => None,
        })
        .collect();
    assert_eq!(allowed, vec![true, false, true]);
}

/// `0x004F955D..0x004F95B3`: a building of a house that lists a human house
/// as its ally (one-way, the victim's own bitfield) gives that human the ally
/// line behind `CreateRadarEvent(0x10)`. Harvester pings (`UnitClass` path)
/// and `MultiplayPassive=` victims in a non-campaign mode do not.
#[test]
fn ally_under_attack_reaches_human_allies_of_the_victim_house() {
    let mut sim = Simulation::new();
    sim.session.game_mode_nonzero = true;
    let human = house(&mut sim, "Americans", true);
    let friend = house(&mut sim, "Russians", false);
    let stranger = house(&mut sim, "Germans", false);
    let other_human = house(&mut sim, "British", true);
    ally(&mut sim, "Russians", "Americans");
    ally(&mut sim, "Americans", "Germans");

    let ping = |owner: InternedId, rx: u16, structure: bool| UnderAttackEvent {
        rx,
        ry: 10,
        owner,
        miner: !structure,
        structure,
    };

    // The friend's building: the human hears the ally line; British does not
    // (Russians never listed them).
    sim.dispatch_under_attack_events(&[ping(friend, 10, true)]);
    assert_eq!(ally_line_owners(&mut sim), vec![human]);
    let _ = other_human;

    // A one-way alliance from the human to the stranger is not enough: the
    // victim's own bitfield decides.
    sim.dispatch_under_attack_events(&[ping(stranger, 40, true)]);
    assert!(ally_line_owners(&mut sim).is_empty());

    // The friend's harvester: `UnitClass::ReceiveDamage` has no ally branch.
    sim.dispatch_under_attack_events(&[ping(friend, 60, false)]);
    assert!(ally_line_owners(&mut sim).is_empty());

    // Same cell again within the window: the type-0x10 diamond dedupes.
    sim.dispatch_under_attack_events(&[ping(friend, 10, true)]);
    assert!(ally_line_owners(&mut sim).is_empty());

    // A passive victim house (`HouseType+0x1A6`) is skipped in MP modes.
    sim.houses.get_mut(&friend).unwrap().multiplay_passive = true;
    sim.dispatch_under_attack_events(&[ping(friend, 90, true)]);
    assert!(ally_line_owners(&mut sim).is_empty());
    sim.session.game_mode_nonzero = false;
    sim.dispatch_under_attack_events(&[ping(friend, 120, true)]);
    assert_eq!(ally_line_owners(&mut sim), vec![human]);
}
