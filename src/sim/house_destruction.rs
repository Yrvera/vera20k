//! Shared active-YR House-wide destructive Techno sweep.
//!
//! `HouseClass` pending-result expiry, multiplayer defeat, and Trigger Action
//! 119 all enter the same native operation at `gamemd 0x004FC6D0`.  This is
//! not mission Scatter: it walks the live Techno registry and synchronously
//! calls each admitted concrete `ReceiveDamage` receiver.

use crate::rules::ruleset::RuleSet;
use crate::sim::combat::{
    EntityDamageEvent, RAD_NO_ATTACKER, ReceiverCallFlags,
};
use crate::sim::intern::InternedId;
use crate::sim::world::Simulation;

/// Effective House identity read by `FUN_0070F820`.
///
/// The temporary marker and source are independent pointers.  A present marker
/// with a null source therefore resolves null; it must not fall back to current
/// ownership.
fn effective_owner(sim: &Simulation, stable_id: u64) -> Option<InternedId> {
    let entity = sim.substrate.entities.get(stable_id)?;
    if let Some(controller_id) = entity.mind_control_controller_id
        && let Some(original_owner) = sim
            .substrate
            .entities
            .get(controller_id)
            .and_then(|controller| controller.capture_manager.as_ref())
            .and_then(|manager| {
                manager
                    .controlled_nodes
                    .iter()
                    .find(|node| node.victim_id == stable_id)
            })
            .map(|node| node.original_owner)
    {
        return Some(original_owner);
    }
    if entity.temporary_owner_transfer_marker.is_some() {
        return entity.temporary_owner_transfer_source;
    }
    Some(entity.owner)
}

/// Resolve the first registered literal Civilian HouseType owner.
fn first_civilian_house(sim: &Simulation, rules: &RuleSet) -> Option<InternedId> {
    let civilian_type = rules.trigger_house_type_index("Civilian")?;
    sim.session.house_order.iter().copied().find(|house_id| {
        sim.houses
            .get(house_id)
            .and_then(|house| house.country)
            .and_then(|country| rules.country_index(sim.interner.resolve(country)))
            == Some(civilian_type)
    })
}

/// `CaptureManagerClass::SetOriginalOwner @ 0x00472330`.
///
/// Resolving Civilian is the success condition.  Native returns true even if
/// its reverse node scan finds no matching victim, so that exact asymmetry is
/// retained here.
fn rewrite_reversible_original_owner_to_civilian(
    sim: &mut Simulation,
    rules: &RuleSet,
    controller_id: u64,
    victim_id: u64,
) -> bool {
    let Some(civilian) = first_civilian_house(sim, rules) else {
        return false;
    };
    if let Some(manager) = sim
        .substrate
        .entities
        .get_mut(controller_id)
        .and_then(|controller| controller.capture_manager.as_mut())
    {
        for node in manager.controlled_nodes.iter_mut().rev() {
            if node.victim_id == victim_id {
                node.original_owner = civilian;
            }
        }
    }
    true
}

/// Result-sweep variant of the incoming Temporal detach helper at
/// `FUN_0071AD40`.
///
/// The victim backlink/warped byte and every owner-side manager link are
/// cleared before the C4 receiver is entered.  The chain is future-validated
/// on snapshot admission, but the visited set also keeps this synchronous
/// native boundary total under a corrupted in-memory graph.
fn clear_incoming_temporal_chain(sim: &mut Simulation, victim_id: u64) {
    let mut cursor = sim
        .substrate
        .entities
        .get_mut(victim_id)
        .and_then(|victim| {
            victim.being_temporally_warped_out = false;
            victim.temporal_targeting_me_id.take()
        });
    let mut visited = std::collections::BTreeSet::new();
    while let Some(owner_id) = cursor {
        if !visited.insert(owner_id) {
            break;
        }
        let next = sim
            .substrate
            .entities
            .get(owner_id)
            .and_then(|owner| owner.temporal_manager)
            .and_then(|manager| manager.next_owner_id);
        if let Some(manager) = sim
            .substrate
            .entities
            .get_mut(owner_id)
            .and_then(|owner| owner.temporal_manager.as_mut())
        {
            manager.target_id = None;
            manager.previous_owner_id = None;
            manager.next_owner_id = None;
            manager.warp_points = 0;
        }
        cursor = next;
    }
}

fn destruction_receiver_event(
    stable_id: u64,
    current_health: i32,
    c4: InternedId,
) -> EntityDamageEvent {
    EntityDamageEvent::direct_receiver(
        stable_id,
        current_health,
        0,
        RAD_NO_ATTACKER,
        None,
        c4,
        ReceiverCallFlags {
            ignore_defenses: true,
            arg6: true,
        },
    )
}

/// Run the shared House destruction operation.
///
/// A represented House with no matching Technos is still a successful empty
/// sweep for Trigger Action 119. Active init/load already pre-resolves the C4
/// handle; the idempotent install below also keeps direct test/tool callers on
/// that same configured Rules authority instead of inventing a hardcoded C4.
pub(crate) fn sweep_house_technos(
    sim: &mut Simulation,
    rules: &RuleSet,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    swept_house: InternedId,
) -> bool {
    if sim.rule_handles.is_none() {
        sim.resolve_type_handles(rules);
    }
    let c4 = sim.rule_handles().c4;

    // Native reloads both the live registry length and the entry at `index`
    // after every synchronous receiver.  A fatal receiver compacts its
    // successor into the same slot.  A surviving receiver remains there and
    // is advanced exactly once by the previous-pointer guard.
    let mut index = 0usize;
    let mut previous_receiver = None;
    while index < sim.tactical_registration_order().len() {
        let stable_id = sim.tactical_registration_order()[index];
        let Some((current_owner, controller_id, current_health)) = sim
            .substrate
            .entities
            .get(stable_id)
            .map(|entity| {
                (
                    entity.owner,
                    entity.mind_control_controller_id,
                    i32::from(entity.health.current),
                )
            })
        else {
            index += 1;
            continue;
        };

        if effective_owner(sim, stable_id) != Some(swept_house) {
            index += 1;
            continue;
        }

        if current_owner != swept_house
            && let Some(controller_id) = controller_id
            && rewrite_reversible_original_owner_to_civilian(
                sim,
                rules,
                controller_id,
                stable_id,
            )
        {
            index += 1;
            continue;
        }

        if previous_receiver == Some(stable_id) {
            index += 1;
            continue;
        }

        // Native copied signed health before detaching Temporal state.
        clear_incoming_temporal_chain(sim, stable_id);
        let event = destruction_receiver_event(stable_id, current_health, c4);
        sim.commit_noncombat_aoe_hits(rules, overlay_registry, &[event]);
        previous_receiver = Some(stable_id);
        // Deliberately no increment: removal exposes the next live Techno at
        // this slot; survival is handled by the guard above.
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::rules::ini_parser::IniFile;
    use crate::sim::capture_manager::{
        CaptureManagerState, CaptureNodeState,
    };
    use crate::sim::game_entity::TemporalManagerState;
    use crate::sim::house_state::HouseState;

    fn rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[Countries]\n0=Americans\n1=Russians\n2=Neutral\n3=Alliance\n4=British\n5=French\n6=Arabs\n7=Africans\n8=Koreans\n9=YuriCountry\n\
             [Americans]\nName=Americans\n\
             [Russians]\nName=Russians\n\
             [Neutral]\nName=Civilian\nMultiplayPassive=yes\n\
             [Alliance]\nName=Alliance\n\
             [British]\nName=British\n\
             [French]\nName=French\n\
             [Arabs]\nName=Arabs\n\
             [Africans]\nName=Africans\n\
             [Koreans]\nName=Koreans\n\
             [YuriCountry]\nName=YuriCountry\n\
             [InfantryTypes]\n0=INF\n\
             [VehicleTypes]\n0=VEH\n\
             [AircraftTypes]\n0=AIR\n\
             [BuildingTypes]\n0=BLD\n\
             [INF]\nStrength=100\nArmor=none\n\
             [VEH]\nStrength=100\nArmor=none\nCrewed=yes\n\
             [AIR]\nStrength=100\nArmor=none\n\
             [BLD]\nStrength=100\nArmor=none\nFoundation=1x1\nCrewed=yes\n\
             [Warheads]\n0=SWEEPC4\n\
             [SWEEPC4]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
             [CombatDamage]\nC4Warhead=SWEEPC4\n",
        ))
        .expect("House destruction rules")
    }

    fn add_house(sim: &mut Simulation, name: &str, country: &str) -> InternedId {
        let owner = sim.interner.intern(name);
        let country = sim.interner.intern(country);
        sim.houses.insert(
            owner,
            HouseState::new(owner, 0, Some(country), false, 0, 10),
        );
        sim.session.house_order.push(owner);
        owner
    }

    fn fixture() -> (Simulation, RuleSet, InternedId, InternedId, InternedId) {
        let rules = rules();
        let mut sim = Simulation::new();
        let target = add_house(&mut sim, "Target", "Americans");
        let other = add_house(&mut sim, "Other", "Russians");
        let civilian = add_house(&mut sim, "NeutralHouse", "Neutral");
        sim.resolve_type_handles(&rules);
        (sim, rules, target, other, civilian)
    }

    fn spawn(
        sim: &mut Simulation,
        rules: &RuleSet,
        type_id: &str,
        owner: InternedId,
        rx: u16,
    ) -> u64 {
        let owner = sim.interner.resolve(owner).to_owned();
        sim.spawn_object(
            type_id,
            &owner,
            rx,
            4,
            0,
            rules,
            &BTreeMap::new(),
        )
        .expect("spawn sweep fixture Techno")
    }

    #[test]
    fn receiver_packet_uses_current_health_configured_c4_and_null_source() {
        let c4 = InternedId::from_index(17);
        let event = destruction_receiver_event(41, -123, c4);
        assert_eq!(event.target_id, 41);
        assert_eq!(event.damage, -123);
        assert_eq!(event.distance_leptons, Some(0));
        assert_eq!(event.attacker_id, RAD_NO_ATTACKER);
        assert_eq!(event.source_house, None);
        assert_eq!(event.warhead_ref, c4);
        assert_eq!(
            event.receiver_flags,
            Some(ReceiverCallFlags {
                ignore_defenses: true,
                arg6: true,
            }),
        );
    }

    #[test]
    fn live_cursor_revisits_compacted_slot_and_advances_one_surviving_receiver() {
        let (mut sim, rules, target, other, _) = fixture();
        let survivor = spawn(&mut sim, &rules, "VEH", target, 2);
        let skipped = spawn(&mut sim, &rules, "VEH", other, 3);
        let first_fatal = spawn(&mut sim, &rules, "VEH", target, 4);
        let second_fatal = spawn(&mut sim, &rules, "VEH", target, 5);
        sim.substrate
            .entities
            .get_mut(survivor)
            .unwrap()
            .health
            .current = 0;

        assert!(sweep_house_technos(&mut sim, &rules, None, target));

        assert!(sim.substrate.entities.get(survivor).is_some());
        assert!(sim.substrate.entities.get(skipped).is_some());
        assert!(sim
            .substrate
            .entities
            .get(first_fatal)
            .is_some_and(|entity| entity.dying));
        assert!(sim
            .substrate
            .entities
            .get(second_fatal)
            .is_some_and(|entity| entity.dying));
        assert_eq!(
            sim.tactical_registration_order(),
            &[survivor, skipped],
            "both fatal removals are consumed from the same compacting cursor",
        );
    }

    #[test]
    fn all_four_techno_categories_include_limbo_and_unrevealed_objects() {
        let (mut sim, rules, target, _, _) = fixture();
        let infantry = spawn(&mut sim, &rules, "INF", target, 2);
        let vehicle = spawn(&mut sim, &rules, "VEH", target, 3);
        let aircraft = spawn(&mut sim, &rules, "AIR", target, 4);
        let building = spawn(&mut sim, &rules, "BLD", target, 5);
        sim.substrate
            .entities
            .get_mut(infantry)
            .unwrap()
            .lifecycle
            .in_limbo = true;
        sim.substrate
            .entities
            .get_mut(aircraft)
            .unwrap()
            .lifecycle
            .cell_marked = false;
        // The Rust Logic list also carries non-Techno registries. A missing
        // EntityStore identity models such an entry and must be skipped by the
        // typed Techno projection without affecting the live cursor.
        sim.set_logic_order_for_test(vec![999_999, infantry, vehicle, aircraft, building]);

        assert!(sweep_house_technos(&mut sim, &rules, None, target));

        for id in [infantry, vehicle, aircraft, building] {
            assert!(
                sim.substrate.entities.get(id).is_some_and(|entity| entity.dying),
                "Techno {id} was admitted regardless of limbo/reveal state"
            );
        }
        let spawned_ids = sim
            .substrate
            .entities
            .values()
            .filter(|entity| entity.stable_id > building)
            .map(|entity| entity.stable_id)
            .collect::<Vec<_>>();
        assert_eq!(
            spawned_ids.len(),
            1,
            "the arg6=true sweep packet produces no Crewed Unit survivor; Building ignores arg6 and spawns one"
        );
        let spawned_building_survivor = sim
            .substrate
            .entities
            .get(spawned_ids[0])
            .expect("Crewed Building receiver synchronously spawned its survivor");
        assert_eq!(
            spawned_building_survivor.category,
            crate::map::entities::EntityCategory::Infantry
        );
        assert!(
            spawned_building_survivor.dying,
            "the live cursor visits and destroys the same-House survivor appended by the Building receiver"
        );
        assert_eq!(
            sim.tactical_registration_order(),
            &[999_999, infantry, spawned_building_survivor.stable_id],
            "both SHP death animations persist and each is advanced once by the guard"
        );
    }

    #[test]
    fn temporary_marker_uses_optional_source_and_never_falls_back_to_current_owner() {
        let (mut sim, rules, source, destination, _) = fixture();
        let transferred = spawn(&mut sim, &rules, "VEH", destination, 2);
        let null_source = spawn(&mut sim, &rules, "VEH", destination, 3);
        {
            let entity = sim.substrate.entities.get_mut(transferred).unwrap();
            entity.temporary_owner_transfer_marker = Some(destination);
            entity.temporary_owner_transfer_source = Some(source);
        }
        {
            let entity = sim.substrate.entities.get_mut(null_source).unwrap();
            entity.temporary_owner_transfer_marker = Some(destination);
            entity.temporary_owner_transfer_source = None;
        }

        assert!(sweep_house_technos(&mut sim, &rules, None, source));
        assert!(sim
            .substrate
            .entities
            .get(transferred)
            .is_some_and(|entity| entity.dying));
        assert!(sim
            .substrate
            .entities
            .get(null_source)
            .is_some_and(|entity| !entity.dying && entity.health.current == 100));

        assert!(sweep_house_technos(&mut sim, &rules, None, destination));
        assert!(
            sim.substrate.entities.get(null_source).is_some(),
            "marker+null source remains effectively ownerless"
        );
    }

    fn controlled_fixture(
        include_civilian_house: bool,
    ) -> (Simulation, RuleSet, InternedId, InternedId, u64, u64) {
        let (mut sim, rules, original, controller_owner, civilian) = fixture();
        if !include_civilian_house {
            sim.houses.remove(&civilian);
            sim.session.house_order.retain(|owner| *owner != civilian);
        }
        let controller_id = spawn(&mut sim, &rules, "VEH", controller_owner, 2);
        let victim_id = spawn(&mut sim, &rules, "VEH", controller_owner, 3);
        sim.substrate.entities.get_mut(controller_id).unwrap().capture_manager = Some(
            CaptureManagerState {
                max_control: 1,
                infinite_mind_control: false,
                controlled_nodes: vec![CaptureNodeState {
                    victim_id,
                    original_owner: original,
                    capture_frame: 1,
                    link_visible_frames: 15,
                }],
            },
        );
        {
            let victim = sim.substrate.entities.get_mut(victim_id).unwrap();
            victim.mind_control_controller_id = Some(controller_id);
            // Reversible control has precedence over the temporary-transfer
            // pair when both raw states coexist after a load.
            victim.temporary_owner_transfer_marker = Some(controller_owner);
            victim.temporary_owner_transfer_source = Some(controller_owner);
        }
        (
            sim,
            rules,
            original,
            controller_owner,
            controller_id,
            victim_id,
        )
    }

    #[test]
    fn reversible_victim_rewrites_to_first_civilian_and_is_spared() {
        let (mut sim, rules, original, _, controller_id, victim_id) =
            controlled_fixture(true);
        let civilian = sim
            .session
            .house_order
            .iter()
            .copied()
            .find(|owner| sim.interner.resolve(*owner) == "NeutralHouse")
            .unwrap();

        assert!(sweep_house_technos(&mut sim, &rules, None, original));

        let victim = sim.substrate.entities.get(victim_id).unwrap();
        assert!(!victim.dying);
        assert_eq!(victim.mind_control_controller_id, Some(controller_id));
        assert_eq!(
            sim.substrate
                .entities
                .get(controller_id)
                .unwrap()
                .capture_manager
                .as_ref()
                .unwrap()
                .controlled_nodes[0]
                .original_owner,
            civilian,
        );
    }

    #[test]
    fn reversible_victim_falls_through_to_damage_when_civilian_is_unresolved() {
        let (mut sim, rules, original, _, _, victim_id) = controlled_fixture(false);
        assert!(sweep_house_technos(&mut sim, &rules, None, original));
        assert!(sim
            .substrate
            .entities
            .get(victim_id)
            .is_some_and(|victim| victim.dying));
    }

    #[test]
    fn controller_death_releases_victim_before_the_live_cursor_rechecks_it() {
        let (mut sim, rules, original, controller_owner, controller_id, victim_id) =
            controlled_fixture(true);

        assert!(sweep_house_technos(
            &mut sim,
            &rules,
            None,
            controller_owner,
        ));

        assert!(sim
            .substrate
            .entities
            .get(controller_id)
            .is_some_and(|controller| controller.dying));
        let victim = sim.substrate.entities.get(victim_id).unwrap();
        assert!(!victim.dying);
        assert_eq!(victim.owner, original);
        assert_eq!(victim.mind_control_controller_id, None);
    }

    #[test]
    fn full_incoming_temporal_chain_clears_before_a_surviving_receiver() {
        let (mut sim, rules, target, other, _) = fixture();
        let victim_id = spawn(&mut sim, &rules, "VEH", target, 2);
        let head_id = spawn(&mut sim, &rules, "VEH", other, 3);
        let middle_id = spawn(&mut sim, &rules, "VEH", other, 4);
        let tail_id = spawn(&mut sim, &rules, "VEH", other, 5);
        {
            let victim = sim.substrate.entities.get_mut(victim_id).unwrap();
            victim.temporal_targeting_me_id = Some(head_id);
            victim.being_temporally_warped_out = true;
        }
        for (id, previous, next, points) in [
            (head_id, None, Some(middle_id), 11),
            (middle_id, Some(head_id), Some(tail_id), 13),
            (tail_id, Some(middle_id), None, 17),
        ] {
            sim.substrate.entities.get_mut(id).unwrap().temporal_manager = Some(
                TemporalManagerState {
                    target_id: Some(victim_id),
                    previous_owner_id: previous,
                    next_owner_id: next,
                    warp_points: points,
                },
            );
        }
        // A zero-health Techno is still admitted. Its zero-damage receiver
        // persists at the same live-array slot, proving both preclear ordering
        // and the previous-pointer guard without invoking an unrelated
        // scenario damage-suppression gate.
        sim.substrate
            .entities
            .get_mut(victim_id)
            .unwrap()
            .health
            .current = 0;

        assert!(sweep_house_technos(&mut sim, &rules, None, target));

        let victim = sim.substrate.entities.get(victim_id).unwrap();
        assert!(!victim.dying);
        assert!(!victim.being_temporally_warped_out);
        assert_eq!(victim.temporal_targeting_me_id, None);
        for id in [head_id, middle_id, tail_id] {
            assert_eq!(
                sim.substrate.entities.get(id).unwrap().temporal_manager,
                Some(TemporalManagerState {
                    target_id: None,
                    previous_owner_id: None,
                    next_owner_id: None,
                    warp_points: 0,
                }),
            );
        }
    }
}
