//! Production command regressions for native CellClass membership authority.
use super::SuperWeaponInstance;
use crate::map::bridge_facts::BridgeCellFacts;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::rules::{ini_parser::IniFile, ruleset::RuleSet};
use crate::sim::{command::Command, house_state::HouseState, production, world::Simulation};
use std::collections::BTreeMap;

fn fixture() -> (Simulation, RuleSet) {
    fixture_with_extra("")
}

fn fixture_with_extra(extra: &str) -> (Simulation, RuleSet) {
    let base = "[InfantryTypes]\n0=E1\n1=BRUTE\n2=BOOM\n[VehicleTypes]\n0=MTNK\n\
         [AircraftTypes]\n[BuildingTypes]\n0=GAPILE\n1=BIG\n\
         [SuperWeaponTypes]\n0=IC\n1=GM\n\
         [IC]\nType=IronCurtain\nRechargeTime=1\n\
         [GM]\nType=GeneticConverter\nRechargeTime=1\n\
         [General]\nMutateExplosion=no\n\
         [CombatDamage]\nC4Warhead=Super\n[SpecialWeapons]\nMutateWarhead=Mutate\n\
         [Warheads]\n0=Super\n1=Mutate\n2=DeathWH\n\
         [Super]\nInfDeath=2\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [Mutate]\nInfDeath=9\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [BOOM]\nStrength=100\nSpeed=4\nExplodes=yes\nDeathWeapon=DeathBoom\n\
         [DeathBoom]\nDamage=400\nWarhead=DeathWH\n\
         [DeathWH]\nCellSpread=1\nPercentAtMax=1\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [BIG]\nStrength=1000\nFoundation=3x1\n\
         [E1]\nStrength=100\nSpeed=4\nCost=200\nTechLevel=1\nOwner=Americans\n\
         [BRUTE]\nStrength=200\nSpeed=4\n\
         [MTNK]\nStrength=300\nSpeed=6\n\
         [GAPILE]\nStrength=1000\nFoundation=1x1\nFactory=InfantryType\n";
    let mut ini = IniFile::from_str(base);
    ini.merge(&IniFile::from_str(extra));
    let rules = RuleSet::from_ini(&ini).unwrap();
    assert!(!rules.general.mutate_explosion);
    assert_eq!(rules.general.mutate_warhead, "Mutate");
    let mut sim = Simulation::with_seed(23);
    sim.intern_rule_type_ids(&rules);
    sim.resolve_type_handles(&rules);
    let owner = sim.interner.intern("Americans");
    sim.houses
        .insert(owner, HouseState::new(owner, 0, None, true, 50_000, 10));
    sim.session.house_order.push(owner);
    sim.session.game_options.super_weapons = true;
    let cells = (0..16)
        .flat_map(|y| (0..16).map(move |x| test_terrain_cell(x, y)))
        .collect();
    sim.resolved_terrain = Some(ResolvedTerrainGrid::from_cells(16, 16, cells));
    sim.playfield_bounds = Some(test_playfield_bounds());
    sim.spawn_object_at_height("GAPILE", "Americans", 10, 10, 0, 0, &rules)
        .unwrap();
    (sim, rules)
}

fn launch_command(sim: &mut Simulation, rules: &RuleSet, name: &str, rx: u16, ry: u16) {
    let owner = sim.interner.intern("Americans");
    let sw_type_id = sim.interner.intern(name);
    let mut instance = SuperWeaponInstance::new(sw_type_id, owner);
    instance.is_active = true;
    instance.is_ready = true;
    sim.super_weapons
        .entry(owner)
        .or_default()
        .insert(sw_type_id, instance);
    assert!(sim.apply_command_with_overlays(
        "Americans",
        &Command::LaunchSuperWeapon {
            sw_type_id,
            target_rx: rx,
            target_ry: ry,
        },
        Some(rules),
        None,
        &BTreeMap::new(),
        None
    ));
    assert!(
        !sim.super_weapons[&owner][&sw_type_id].is_ready,
        "actual command consumes readiness"
    );
}

fn held_infantry_survives_launch(name: &str) {
    let (mut sim, rules) = fixture();
    let owner = sim.interner.intern("Americans");
    assert!(production::enqueue_by_type(
        &mut sim,
        &rules,
        "Americans",
        "E1"
    ));
    let held = sim
        .production
        .factory_shadow
        .view(owner, production::ProductionCategory::Infantry)
        .unwrap()
        .object
        .unwrap()
        .entity_id
        .unwrap();
    let twin = sim
        .construct_object_limbo_at_height("E1", "Americans", 0, 0, 0, 0, &rules)
        .unwrap();
    let marked = sim
        .spawn_object_at_height("E1", "Americans", 1, 1, 0, 0, &rules)
        .unwrap();
    assert!(
        sim.substrate
            .entities
            .get(marked)
            .unwrap()
            .lifecycle
            .cell_marked
    );
    assert!(sim.substrate.entities.get(held).unwrap().lifecycle.in_limbo);
    assert!(!sim.substrate.occupancy.contains_entity(0, 0, held));
    assert!(!sim.substrate.occupancy.contains_entity(0, 0, twin));
    assert!(
        sim.substrate
            .occupancy
            .get(1, 1)
            .unwrap()
            .iter_layer(crate::sim::movement::locomotor::MovementLayer::Ground)
            .any(|member| member.entity_id == marked)
    );
    let before_factory = sim
        .production
        .factory_shadow
        .iter_insertion_ordered()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    launch_command(&mut sim, &rules, name, 0, 0);
    for id in [held, twin] {
        let object = sim
            .substrate
            .entities
            .get(id)
            .expect("unmarked object retained");
        assert_eq!(
            object.health.current, 100,
            "{name} must use marked cell membership: {id}"
        );
        assert!(!object.dying);
        assert!(
            object.lifecycle.in_limbo && !object.lifecycle.cell_marked && !object.in_logic_vector
        );
    }
    assert!(
        sim.substrate
            .entities
            .get(marked)
            .is_none_or(|object| object.health.current == 0),
        "marked control receives the launch"
    );
    assert_eq!(
        sim.production
            .factory_shadow
            .iter_insertion_ordered()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        before_factory
    );
    production::validate_restored_factory_state(&sim, &rules).unwrap();
}

#[test]
fn iron_curtain_command_does_not_damage_factory_held_or_unmarked_infantry() {
    held_infantry_survives_launch("IC");
}

#[test]
fn genetic_converter_command_does_not_damage_factory_held_or_unmarked_infantry() {
    held_infantry_survives_launch("GM");
}

fn marked_brutes(sim: &Simulation) -> Vec<u64> {
    sim.substrate
        .entities
        .values()
        .filter(|entity| {
            sim.interner.resolve(entity.type_ref()) == "BRUTE" && entity.lifecycle.cell_marked
        })
        .map(|entity| entity.stable_id())
        .collect()
}

#[test]
fn genetic_converter_command_preserves_stable_id_batch_replacement_order() {
    let (mut sim, rules) = fixture();
    // Native cell order is center then east; existing Rust replacement order
    // is stable-ID order. Keep that compatibility decision explicit.
    let east = sim
        .spawn_object_at_height("E1", "Soviet", 6, 5, 0, 0, &rules)
        .unwrap();
    let center = sim
        .spawn_object_at_height("E1", "Soviet", 5, 5, 0, 0, &rules)
        .unwrap();
    launch_command(&mut sim, &rules, "GM", 5, 5);
    let brutes = marked_brutes(&sim);
    assert_eq!(brutes, vec![center + 1, center + 2]);
    let owner = sim.interner.intern("Americans");
    for (brute, (rx, ry)) in brutes.into_iter().zip([(6, 5), (5, 5)]) {
        let object = sim.substrate.entities.get(brute).unwrap();
        assert_eq!(
            (object.position.rx, object.position.ry, object.position.z),
            (rx, ry, 0)
        );
        assert_eq!(object.owner(), owner);
        assert_eq!(object.health.current, 200);
    }
    for victim in [east, center] {
        let object = sim.substrate.entities.get(victim).unwrap();
        assert!(object.health.current == 0 && object.dying);
        assert!(
            object.lifecycle.cell_marked && object.in_logic_vector,
            "legacy per-cell corpse membership is retained by this admission change"
        );
    }
    assert!(sim.substrate.pending_delete.is_empty());
}

#[test]
fn genetic_converter_command_uses_selected_bridge_membership_and_original_victims_only() {
    let (mut sim, rules) = fixture();
    let ground = sim
        .spawn_object_at_height("E1", "Americans", 5, 5, 0, 0, &rules)
        .unwrap();
    {
        let cell = sim
            .resolved_terrain
            .as_mut()
            .unwrap()
            .cell_mut(5, 5)
            .unwrap();
        cell.bridge_facts.raw_flags = crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
        cell.has_bridge_deck = true;
        cell.bridge_walkable = true;
        cell.bridge_deck_level = 4;
    }
    let mut deck = Vec::new();
    for _ in 0..2 {
        let id = sim
            .construct_object_limbo_at_height("E1", "Soviet", 5, 5, 0, 4, &rules)
            .unwrap();
        sim.substrate.entities.get_mut(id).unwrap().on_bridge = true;
        assert!(matches!(
            sim.reveal(id),
            crate::sim::world::RevealOutcome::Revealed { .. }
        ));
        deck.push(id);
    }
    launch_command(&mut sim, &rules, "GM", 5, 5);
    assert_eq!(
        sim.substrate.entities.get(ground).unwrap().health.current,
        100
    );
    for &id in &deck {
        assert_eq!(sim.substrate.entities.get(id).unwrap().health.current, 0);
    }
    // Replacement retains legacy Z=0 even for a deck victim. It is not a
    // native AnimToInfantry placement claim. Both original victims get one attempt.
    let brutes: Vec<_> = sim
        .substrate
        .entities
        .values()
        .filter(|e| sim.interner.resolve(e.type_ref()) == "BRUTE")
        .collect();
    assert_eq!(brutes.len(), 2);
    for object in brutes {
        assert_eq!(object.position.z, 0);
        assert_eq!(
            object.health.current, 200,
            "new replacements never re-enter selection"
        );
    }
    assert_eq!(sim.allocate_stable_id(), deck[1] + 3);
}

#[test]
fn genetic_converter_command_missing_brute_keeps_kills_and_consumes_readiness() {
    let (mut sim, rules) =
        fixture_with_extra("[InfantryTypes]\n1=NO_BRUTE\n[NO_BRUTE]\nStrength=200\nSpeed=4\n");
    assert!(rules.object("BRUTE").is_none());
    let victim = sim
        .spawn_object_at_height("E1", "Americans", 5, 5, 0, 0, &rules)
        .unwrap();
    launch_command(&mut sim, &rules, "GM", 5, 5);
    let corpse = sim.substrate.entities.get(victim).unwrap();
    assert!(corpse.health.current == 0 && corpse.dying);
    assert!(marked_brutes(&sim).is_empty());
}

#[test]
fn genetic_converter_explosion_command_admits_replacements_after_nested_damage() {
    for (other_x, expected_replacements) in [(5, 2), (6, 1)] {
        let (mut sim, mut rules) = fixture_with_extra(
            "[Warheads]\n3=MutationAoE\n[SpecialWeapons]\nMutateExplosionWarhead=MutationAoE\n\
             [MutationAoE]\nCellSpread=1\nPercentAtMax=1\nInfDeath=1\n\
             Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        );
        rules.general.mutate_explosion = true;
        let boomer = sim
            .spawn_object_at_height("BOOM", "Soviet", 5, 5, 0, 0, &rules)
            .unwrap();
        let other = sim
            .spawn_object_at_height("E1", "Soviet", other_x, 5, 0, 0, &rules)
            .unwrap();
        let tank = sim
            .spawn_object_at_height("MTNK", "Soviet", 5, 6, 0, 0, &rules)
            .unwrap();
        if other_x == 6 {
            // The east infantry is (320,-64) leptons from the mutation's
            // cell-center impact: outside radius256. It is exactly256 from
            // BOOM's off-center DeathWeapon, so it dies only as collateral.
            for id in [boomer, other] {
                let position = &sim.substrate.entities.get(id).unwrap().position;
                assert_eq!(
                    (
                        position.sub_x.to_num::<i32>(),
                        position.sub_y.to_num::<i32>()
                    ),
                    (192, 64)
                );
            }
        }
        launch_command(&mut sim, &rules, "GM", 5, 5);
        for victim in [boomer, other, tank] {
            assert!(
                sim.substrate
                    .entities
                    .get(victim)
                    .is_some_and(|e| e.health.current == 0 && e.dying)
            );
        }
        let brutes = marked_brutes(&sim);
        assert_eq!(
            brutes.len(),
            expected_replacements,
            "only original infantry receivers get replacements; tank damage and collateral deaths still commit"
        );
        assert!(brutes.iter().all(|id| *id > tank));
        for id in brutes {
            let object = sim.substrate.entities.get(id).unwrap();
            assert_eq!(
                object.health.current, 200,
                "replacement must not be exposed to the earlier nested DeathWeapon"
            );
            assert_eq!((object.position.rx, object.position.ry), (5, 5));
        }
    }
}

// Retail rulesmd.ini:818 selects C4Warhead=Super; [Super] uses InfDeath=2.
// Native ordinary death retains membership during action 0xC (0x00518635).
#[test]
fn iron_curtain_command_forces_authored_strength_and_attributes_retained_deaths() {
    use crate::sim::animation::SequenceKind;
    use crate::sim::superweapon::invulnerability::{InvulnKind, apply_invulnerability};
    let (mut sim, rules) = fixture();
    let owner = sim.interner.intern("Americans");
    let enemy = sim.interner.intern("Soviet");
    sim.houses
        .insert(enemy, HouseState::new(enemy, 1, None, true, 10_000, 10));
    sim.session.house_order.push(enemy);
    let tank = sim
        .spawn_object_at_height("MTNK", "Americans", 5, 5, 0, 0, &rules)
        .unwrap();
    let over_strength = sim
        .spawn_object_at_height("E1", "Soviet", 5, 5, 0, 0, &rules)
        .unwrap();
    let wounded = sim
        .spawn_object_at_height("E1", "Soviet", 5, 5, 0, 0, &rules)
        .unwrap();

    let frame = sim.session.binary_frame;
    {
        let object = sim.substrate.entities.get_mut(over_strength).unwrap();
        object.health.current = 150;
        object.health.max = 150;
    }
    {
        let object = sim.substrate.entities.get_mut(wounded).unwrap();
        object.health.current = 40;
        apply_invulnerability(object, frame, 100, InvulnKind::IronCurtain);
    }
    launch_command(&mut sim, &rules, "IC", 5, 5);
    assert_eq!(
        sim.substrate
            .entities
            .get(over_strength)
            .unwrap()
            .health
            .current,
        50,
        "receiver damage is authored Strength=100, not a raw kill or health.max"
    );
    let dead = sim.substrate.entities.get(wounded).unwrap();
    assert_eq!(
        dead.health.current, 0,
        "forced C4 bypasses prior invulnerability"
    );
    assert!(dead.dying && dead.lifecycle.cell_marked && dead.in_logic_vector);
    assert_eq!(
        dead.animation.as_ref().unwrap().sequence,
        SequenceKind::Die2
    );
    assert_eq!(dead.killed_by, Some(owner));
    // The ordinary sequence still owns the object; the existing shared UnInit
    // terminal owns score publication. Verify the attributed receipt survives it.
    sim.uninit_with_rules(wounded, &rules);
    assert_eq!(sim.houses[&owner].stats.units_killed, 1);
    assert_eq!(sim.houses[&enemy].stats.units_lost, 1);
    assert!(
        sim.substrate
            .entities
            .get(tank)
            .unwrap()
            .invulnerability
            .is_some()
    );
}

#[test]
fn iron_curtain_command_selects_deck_members_and_overlapping_foundation() {
    use crate::sim::movement::locomotor::MovementLayer;
    let (mut sim, rules) = fixture();
    let ground = sim
        .spawn_object_at_height("MTNK", "Americans", 5, 5, 0, 0, &rules)
        .unwrap();
    {
        let cell = sim
            .resolved_terrain
            .as_mut()
            .unwrap()
            .cell_mut(5, 5)
            .unwrap();
        cell.bridge_facts.raw_flags = crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
        cell.has_bridge_deck = true;
        cell.bridge_walkable = true;
        cell.bridge_deck_level = 4;
    }
    let deck = sim
        .construct_object_limbo_at_height("MTNK", "Americans", 5, 5, 0, 4, &rules)
        .unwrap();
    sim.substrate.entities.get_mut(deck).unwrap().on_bridge = true;
    sim.reveal(deck);
    let big = sim
        .spawn_object_at_height("BIG", "Americans", 2, 4, 0, 0, &rules)
        .unwrap();
    let outside = sim
        .spawn_object_at_height("MTNK", "Americans", 7, 7, 0, 0, &rules)
        .unwrap();
    let twin = sim
        .construct_object_limbo_at_height("MTNK", "Americans", 4, 4, 0, 0, &rules)
        .unwrap();
    assert!(
        sim.substrate
            .occupancy
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Bridge)
            .any(|member| member.entity_id == deck)
    );
    assert!(sim.substrate.occupancy.contains_entity(4, 4, big));
    assert_eq!(sim.substrate.entities.get(big).unwrap().position.rx, 2);
    launch_command(&mut sim, &rules, "IC", 5, 5);
    for id in [deck, big] {
        assert!(
            sim.substrate
                .entities
                .get(id)
                .unwrap()
                .invulnerability
                .is_some(),
            "selected member {id}"
        );
    }
    for id in [ground, outside, twin] {
        assert!(
            sim.substrate
                .entities
                .get(id)
                .unwrap()
                .invulnerability
                .is_none(),
            "excluded member {id}"
        );
    }
}

#[test]
fn iron_curtain_command_runs_nested_death_before_the_next_native_cell() {
    // Native table visits the center before east. Swapping the two objects
    // changes whether the tank was protected before the synchronous DeathWeapon.
    for (boomer_x, tank_x, survives) in [(5, 6, false), (6, 5, true)] {
        let (mut sim, rules) = fixture();
        sim.spawn_object_at_height("BOOM", "Americans", boomer_x, 5, 0, 0, &rules)
            .unwrap();
        let tank = sim
            .spawn_object_at_height("MTNK", "Americans", tank_x, 5, 0, 0, &rules)
            .unwrap();
        launch_command(&mut sim, &rules, "IC", 5, 5);
        let object = sim.substrate.entities.get(tank).unwrap();
        assert_eq!(
            object.health.current > 0,
            survives,
            "center/east receiver ordering"
        );
        assert_eq!(object.invulnerability.is_some(), survives);
    }
}

#[test]
fn iron_curtain_command_uses_packed_aliases_and_stamps_missing_cells() {
    command_uses_packed_aliases_and_stamps_missing_cells("IC", "MTNK");
}

#[test]
fn genetic_converter_command_uses_packed_aliases_and_stamps_missing_cells() {
    command_uses_packed_aliases_and_stamps_missing_cells("GM", "E1");
}

fn command_uses_packed_aliases_and_stamps_missing_cells(name: &str, object_type: &str) {
    for (x, y) in [(512, 1), (u16::MAX, 2)] {
        let (mut sim, rules) = fixture();
        let victim = sim
            .spawn_object_at_height(object_type, "Americans", 0, 2, 0, 0, &rules)
            .unwrap();
        launch_command(&mut sim, &rules, name, x, y);
        let object = sim.substrate.entities.get(victim).unwrap();
        if name == "GM" {
            assert!(object.health.current == 0 && object.dying);
            let brutes = marked_brutes(&sim);
            assert_eq!(brutes.len(), 1);
            let replacement = sim.substrate.entities.get(brutes[0]).unwrap();
            assert_eq!(
                (replacement.position.rx, replacement.position.ry),
                (0, 2),
                "replacement uses the actual victim cell after fixed-stride alias / word wrap"
            );
        } else {
            assert!(
                object.invulnerability.is_some(),
                "fixed-stride alias / independent word wrap reaches actual cell (0,2)"
            );
        }
    }
    let (mut sim, rules) = fixture();
    let victim = sim
        .spawn_object_at_height(object_type, "Americans", 5, 5, 0, 0, &rules)
        .unwrap();
    let original_hp = sim.substrate.entities.get(victim).unwrap().health.current;
    let allocated: Vec<_> = (0..16)
        .flat_map(|y| (0..16).map(move |x| (x, y)))
        .filter(|&cell| cell != (5, 5))
        .collect();
    sim.resolved_terrain
        .as_mut()
        .unwrap()
        .test_set_native_allocated_cells(&allocated);
    for mapless in [false, true] {
        if mapless {
            sim.resolved_terrain = None;
        }
        launch_command(&mut sim, &rules, name, 5, 5);
        let object = sim.substrate.entities.get(victim).unwrap();
        assert_eq!(object.health.current, original_hp);
        assert!(!object.dying && object.invulnerability.is_none());
        assert!(marked_brutes(&sim).is_empty());
        assert_eq!(
            sim.effective_shared_cell_dummy().snapshot().coord,
            if mapless { (6, 6) } else { (5, 5) },
            "missing lookup stamps the process dummy; mapless visits through the final native cell"
        );
    }
}

#[test]
fn iron_curtain_command_observes_native_deck_order_after_nested_bridge_drop_in() {
    use crate::sim::bridge_state::{
        AnchorSpan, Axis, BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState,
        BridgeheadAnchorClass, DamageState, Direction,
    };
    use crate::sim::movement::locomotor::MovementLayer;
    let (mut sim, rules) = fixture_with_extra("[DeathWH]\nWall=yes\n[DeathBoom]\nDamage=2000\n");
    assert!(rules.warhead("DeathWH").unwrap().wall);
    // The selected damaged anchor takes one hit to collapse. Seed the existing
    // runtime map/record boundary; launch, nested death and fallout are real.
    let span = [(5, 5), (6, 5), (7, 5), (8, 5), (4, 5)];
    for &(x, y) in &span {
        let cell = sim
            .resolved_terrain
            .as_mut()
            .unwrap()
            .cell_mut(x, y)
            .unwrap();
        cell.bridge_facts.raw_flags = crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
        // Current bridge dispatcher admits impacts within one terrain level.
        // Deck Z=4 and ground Z=3 exercise its actual collapse/DropIn path.
        cell.level = 3;
        cell.has_bridge_deck = true;
        cell.bridge_walkable = true;
        cell.bridge_deck_level = 4;
    }
    sim.resolved_terrain
        .as_mut()
        .unwrap()
        .cell_mut(5, 5)
        .unwrap()
        .bridge_facts
        .raw_flags |= crate::map::bridge_facts::BRIDGE_FLAG_ANCHOR_SELF;
    let mut state =
        BridgeRuntimeState::from_resolved_terrain(sim.resolved_terrain.as_ref().unwrap(), true, 1);
    state.test_seed_cell(
        5,
        5,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Damaged,
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0,
            damaged_variant: true,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        },
    );
    state.test_seed_anchor_span(AnchorSpan {
        id: 1,
        anchor: (5, 5),
        cells: [
            Some(span[0]),
            Some(span[1]),
            Some(span[2]),
            Some(span[3]),
            Some(span[4]),
            None,
        ],
        axis: Axis::NS,
        direction: Direction::E,
        damage_state: DamageState::Damaged,
        bridge_group_id: 1,
    });
    sim.bridge_state = Some(state);
    // Older deck recipient is visited only after the newer Infantry's callback.
    let tank = sim
        .construct_object_limbo_at_height("MTNK", "Americans", 5, 5, 0, 4, &rules)
        .unwrap();
    sim.substrate.entities.get_mut(tank).unwrap().on_bridge = true;
    sim.reveal(tank);
    // Keep it alive through the nested DeathWeapon without setting IC first.
    sim.substrate.entities.get_mut(tank).unwrap().health.current = 10_000;
    sim.substrate.entities.get_mut(tank).unwrap().health.max = 10_000;
    let boomer = sim
        .construct_object_limbo_at_height("BOOM", "Americans", 5, 5, 0, 4, &rules)
        .unwrap();
    sim.substrate.entities.get_mut(boomer).unwrap().on_bridge = true;
    sim.reveal(boomer);
    assert_eq!(
        sim.substrate
            .occupancy
            .get(5, 5)
            .unwrap()
            .snapshot_layer(MovementLayer::Bridge),
        vec![boomer, tank]
    );
    let unmarked = sim
        .construct_object_limbo_at_height("MTNK", "Americans", 5, 5, 0, 4, &rules)
        .unwrap();
    sim.substrate.entities.get_mut(unmarked).unwrap().on_bridge = true;
    launch_command(&mut sim, &rules, "IC", 5, 5);
    let current = sim.substrate.entities.get(boomer).unwrap();
    assert!(
        current.dying && !current.on_bridge,
        "nested fallout relayers the retained current receiver"
    );
    let recipient = sim.substrate.entities.get(tank).unwrap();
    assert!(!recipient.on_bridge && recipient.health.current > 0);
    assert!(
        recipient.invulnerability.is_none(),
        "native deck traversal prepends tank after boomer; current boomer then has no successor"
    );
    let ground = sim
        .substrate
        .occupancy
        .get(5, 5)
        .unwrap()
        .snapshot_layer(MovementLayer::Ground);
    assert_eq!(ground, vec![tank, boomer]);
    let rebuilt = crate::sim::occupancy::OccupancyGrid::rebuild(&sim.substrate.entities);
    assert_eq!(
        rebuilt
            .get(5, 5)
            .unwrap()
            .snapshot_layer(MovementLayer::Ground),
        ground,
        "serialized re-entry order preserves the live list during restore"
    );
    let twin = sim.substrate.entities.get(unmarked).unwrap();
    assert!(twin.on_bridge && twin.lifecycle.in_limbo && !twin.lifecycle.cell_marked);
    assert_eq!(
        twin.position.z, 4,
        "unmarked deck coordinate is not a DropIn recipient"
    );
    assert_eq!(
        sim.substrate
            .occupancy
            .get(5, 5)
            .unwrap()
            .snapshot_layer(MovementLayer::Bridge),
        Vec::<u64>::new()
    );
}

// Wide native diamond used by the existing production admission fixtures;
// explicitly supplies the mandatory mode-one bounds, including edge-cell tests.
pub(super) fn test_playfield_bounds() -> crate::sim::cell_rect::PlayfieldBounds {
    crate::sim::cell_rect::PlayfieldBounds {
        base: 0,
        off_fc: -100,
        off_100: -100,
        off_104: 200,
        off_108: 200,
    }
}

pub(super) fn test_terrain_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
    ResolvedTerrainCell {
        rx,
        ry,
        source_tile_index: 0,
        source_sub_tile: 0,
        final_tile_index: 0,
        final_sub_tile: 0,
        is_wood_bridge_repair_tile: false,
        level: 0,
        filled_clear: false,
        tileset_index: Some(0),
        land_type: 0,
        yr_cell_land_type: 0,
        slope_type: 0,
        template_height: 0,
        render_offset_x: 0,
        render_offset_y: 0,
        terrain_class: TerrainClass::Clear,
        speed_costs: SpeedCostProfile {
            foot: Some(100),
            track: Some(100),
            wheel: Some(100),
            hover: Some(100),
            amphibious: Some(100),
            ..SpeedCostProfile::default()
        },
        is_water: false,
        is_cliff_like: false,
        is_rough: false,
        is_road: false,
        accepts_smudge: false,
        allows_tiberium: false,
        height_in_pixels: 0,
        variant: 0,
        has_ramp: false,
        canonical_ramp: None,
        ground_walk_blocked: false,
        terrain_object_blocks: false,
        terrain_object_occupation: None,
        overlay_blocks: false,
        overlay_zone_type: None,
        outside_playfield: false,
        zone_type: 0,
        base_ground_walk_blocked: false,
        base_build_blocked: false,
        base_land_type: 0,
        base_yr_cell_land_type: 0,
        base_terrain_class: Default::default(),
        base_speed_costs: SpeedCostProfile {
            foot: Some(100),
            track: Some(100),
            wheel: Some(100),
            hover: Some(100),
            amphibious: Some(100),
            ..SpeedCostProfile::default()
        },
        build_blocked: false,
        has_bridge_deck: false,
        bridge_walkable: false,
        bridge_transition: false,
        bridge_deck_level: 0,
        bridge_layer: None,
        bridge_facts: BridgeCellFacts::default(),
        tube_index: None,
        radar_left: [0, 0, 0],
        radar_right: [0, 0, 0],
        has_damaged_data: false,
        bridgehead_anchor_class_at_load: None,
    }
}
