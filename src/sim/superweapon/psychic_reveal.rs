//! PsychicReveal superweapon launch handler.
//!
//! Reveals shroud in a radius around the target cell for the owning house.
//! Matches binary's double-call to MapClass::RevealAroundCell (verified).
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, sim/vision, sim/world.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::ruleset::RuleSet;
use crate::sim::intern::InternedId;
use crate::sim::vision;
use crate::sim::world::{SimSoundEvent, Simulation};

/// Launch PsychicReveal at (target_rx, target_ry). Reveals shroud in radius.
pub fn launch(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    target_rx: u16,
    target_ry: u16,
    sw_type: InternedId,
) -> bool {
    let radius = rules.general.psychic_reveal_radius as u16;

    // Double call matches binary (verified). Both calls pass identical args.
    vision::reveal_radius(&mut sim.fog, owner, target_rx, target_ry, radius);
    vision::reveal_radius(&mut sim.fog, owner, target_rx, target_ry, radius);

    sim.sound_events.push(SimSoundEvent::SuperWeaponLaunched {
        owner,
        sw_type,
        rx: target_rx,
        ry: target_ry,
    });

    log::info!(
        "PsychicReveal launched at ({}, {}) by '{}', radius={}",
        target_rx,
        target_ry,
        sim.interner.resolve(owner),
        radius
    );

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    fn minimal_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
             [General]\nPsychicRevealRadius=5\n",
        );
        RuleSet::from_ini(&ini).expect("test rules")
    }

    #[test]
    fn pr_reveals_cells_in_radius() {
        let rules = minimal_rules();
        let mut sim = Simulation::new();
        sim.fog.width = 30;
        sim.fog.height = 30;
        let owner = sim.interner.intern("Americans");
        let sw_test = sim.interner.intern("SWTEST");
        assert!(launch(&mut sim, &rules, owner, 10, 10, sw_test));
        let vis = sim.fog.by_owner.get(&owner).expect("owner fog exists");
        assert!(vis.is_visible(10, 10));
    }

    #[test]
    fn pr_does_not_reveal_beyond_radius() {
        let rules = minimal_rules();
        let mut sim = Simulation::new();
        sim.fog.width = 30;
        sim.fog.height = 30;
        let owner = sim.interner.intern("Americans");
        let sw_test = sim.interner.intern("SWTEST");
        launch(&mut sim, &rules, owner, 10, 10, sw_test);
        let vis = sim.fog.by_owner.get(&owner).expect("owner fog exists");
        // Radius=5, so (25, 25) is well outside.
        assert!(!vis.is_visible(25, 25));
    }

    /// The launch event carries the launched `[SuperWeaponTypes]` section, the
    /// same object `SuperClass::Launch @ 0x006CC390` reads its `Type=` case
    /// and (for `ForceShield`) its `StartSound=` from. Without it the app
    /// layer cannot tell one superweapon's cue from another's.
    #[test]
    fn the_launch_event_names_the_superweapon_type_that_fired() {
        let rules = minimal_rules();
        let mut sim = Simulation::new();
        sim.fog.width = 30;
        sim.fog.height = 30;
        let owner = sim.interner.intern("Americans");
        let sw_test = sim.interner.intern("PsychicRevealSpecial");
        assert!(launch(&mut sim, &rules, owner, 10, 11, sw_test));
        let launched = sim
            .sound_events
            .iter()
            .find_map(|event| match event {
                SimSoundEvent::SuperWeaponLaunched {
                    owner,
                    sw_type,
                    rx,
                    ry,
                } => Some((*owner, *sw_type, *rx, *ry)),
                _ => None,
            })
            .expect("a launch emits exactly one SuperWeaponLaunched event");
        assert_eq!(launched, (owner, sw_test, 10, 11));
        assert_eq!(sim.interner.resolve(launched.1), "PsychicRevealSpecial");
    }
}
