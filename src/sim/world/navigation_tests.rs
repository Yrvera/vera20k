//! Production navigation reconstruction must respect Mark-owned cell presence.

use super::{empty_heights, gsi_04_10_clear_terrain, make_test_entity};
use crate::map::entities::EntityCategory;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::production::{ProductionCategory, enqueue_by_type};
use crate::sim::snapshot::GameSnapshot;
use crate::sim::world::{Simulation, TickLane};
use std::collections::BTreeMap;

fn rules_and_overlays() -> (RuleSet, OverlayTypeRegistry) {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n\
         [BuildingTypes]\n0=YARD\n1=HELD\n2=UPGRADE\n\
         [YARD]\nStrength=500\nFoundation=2x2\nBib=yes\nFactory=BuildingType\n\
         [HELD]\nStrength=300\nCost=100\nTechLevel=1\nOwner=Americans\nFoundation=3x3\n\
         [UPGRADE]\nStrength=100\nFoundation=4x4\n\
         [OverlayTypes]\n0=ROAD\n[ROAD]\nLand=Road\n\
         [Road]\nFoot=37%\nTrack=100%\n",
    );
    (
        RuleSet::from_ini(&ini).unwrap(),
        OverlayTypeRegistry::from_ini(&ini, None),
    )
}

fn assert_only_marked_foundation(sim: &Simulation) {
    let path = sim.path_grid_snapshot().unwrap();
    for ry in 0..3 {
        for rx in 0..3 {
            assert!(
                path.is_walkable(rx, ry),
                "held factory coordinate has no footprint"
            );
        }
    }
    assert!(!path.is_walkable(6, 6), "marked parent remains a blocker");
    assert!(!path.is_walkable(6, 7));
    assert!(
        path.is_walkable(7, 6),
        "the parent's HasBib relaxation is preserved"
    );
    assert!(path.is_walkable(7, 7));
    for (rx, ry) in [(8, 6), (9, 6), (6, 8), (6, 9), (9, 9)] {
        assert!(
            path.is_walkable(rx, ry),
            "unmarked upgrade adds no foundation at {rx},{ry}"
        );
    }
}

fn assert_retained_roles(sim: &Simulation, parent_id: u64, upgrade_id: u64, held_id: u64) {
    assert!(
        sim.substrate
            .entities
            .get(parent_id)
            .unwrap()
            .lifecycle
            .cell_marked
    );
    let upgrade = sim.substrate.entities.get(upgrade_id).unwrap();
    assert!(!upgrade.lifecycle.in_limbo && !upgrade.lifecycle.cell_marked);
    assert!(
        upgrade
            .structure_upgrade_link
            .is_some_and(|link| link.parent_stable_id == parent_id)
    );
    let held = sim.substrate.entities.get(held_id).unwrap();
    assert!(held.lifecycle.in_limbo && !held.lifecycle.cell_marked);
    assert_eq!(
        sim.production
            .factory_shadow
            .view(held.owner(), ProductionCategory::Building)
            .unwrap()
            .object
            .as_ref()
            .unwrap()
            .entity_id,
        Some(held_id)
    );
}

#[test]
fn held_factory_and_attached_upgrade_stay_off_navigation_through_frame_and_restore() {
    let (rules, overlays) = rules_and_overlays();
    let mut sim = Simulation::with_seed(0x4D41_524B);
    sim.session.map_width = 16;
    sim.session.map_height = 16;
    sim.session.game_options.crates = false;
    sim.production.ore_growth_config.grows = false;
    sim.production.ore_growth_config.spreads = false;
    let terrain = gsi_04_10_clear_terrain(16, 16);
    sim.install_resolved_terrain_for_new_map(terrain.clone());
    let mut overlay_grid = OverlayGrid::from_overlay_entries(&[], 16, 16);
    overlay_grid.retain_zero_wall_plane_for_tests();
    sim.overlay_grid = Some(overlay_grid);
    sim.intern_rule_type_ids(&rules);
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(
        owner,
        crate::sim::house_state::HouseState::new(owner, 0, None, true, 50_000, 10),
    );
    let mut yard = make_test_entity("YARD", EntityCategory::Structure);
    yard.cell_x = 6;
    yard.cell_y = 6;
    yard.structure_upgrades = [Some("UPGRADE".to_owned()), None, None];
    assert_eq!(
        sim.spawn_from_map(&[yard], Some(&rules), &empty_heights()),
        2
    );
    sim.resolve_type_handles(&rules);
    let parent_id = sim
        .substrate
        .entities
        .values()
        .find(|entity| entity.structure_upgrade_link.is_none())
        .unwrap()
        .stable_id();
    let upgrade = sim
        .substrate
        .entities
        .values()
        .find(|entity| entity.structure_upgrade_link.is_some())
        .unwrap();
    assert!(!upgrade.lifecycle.in_limbo && !upgrade.lifecycle.cell_marked);
    let upgrade_id = upgrade.stable_id();
    assert!(enqueue_by_type(&mut sim, &rules, "Americans", "HELD"));
    let held_id = sim
        .production
        .factory_shadow
        .view(owner, ProductionCategory::Building)
        .unwrap()
        .object
        .as_ref()
        .unwrap()
        .entity_id
        .unwrap();
    let held = sim.substrate.entities.get(held_id).unwrap();
    assert!(held.lifecycle.in_limbo && !held.lifecycle.cell_marked);
    assert_eq!((held.position.rx, held.position.ry), (0, 0));

    assert!(sim.rebuild_dynamic_navigation(&rules));
    assert_only_marked_foundation(&sim);
    assert_retained_roles(&sim, parent_id, upgrade_id, held_id);
    let before = sim.path_grid_snapshot().unwrap();
    // A real dirty-overlay frame triggers the same canonical rebuild used by
    // ordinary terrain mutation, independent of the held building's location.
    sim.overlay_grid
        .as_mut()
        .unwrap()
        .place_overlay(12, 12, 0, 0);
    let output = sim.advance_app_frame(
        &[],
        Some(&rules),
        &BTreeMap::new(),
        Some(&overlays),
        67,
        TickLane::Ordinary,
        None,
    );
    assert_eq!(output.overlay_updates.len(), 1);
    assert_eq!(
        (output.overlay_updates[0].rx, output.overlay_updates[0].ry),
        (12, 12)
    );
    assert!(!std::sync::Arc::ptr_eq(
        &before,
        &sim.path_grid_snapshot().unwrap()
    ));
    assert_only_marked_foundation(&sim);
    assert_retained_roles(&sim, parent_id, upgrade_id, held_id);

    let bytes = GameSnapshot::save(&sim, 0, 0, "marked-navigation.map", 0);
    let mut restored = GameSnapshot::load(&bytes).unwrap().sim;
    restored.restore_after_snapshot_load().unwrap();
    restored.rebuild_caches_after_load(
        terrain,
        Default::default(),
        Vec::new(),
        Vec::new(),
        BTreeMap::new(),
    );
    restored
        .restore_map_authority_after_snapshot_load(&rules, &overlays)
        .unwrap();
    assert!(
        restored
            .substrate
            .entities
            .get(held_id)
            .unwrap()
            .lifecycle
            .in_limbo
    );
    assert_only_marked_foundation(&restored);
    assert_retained_roles(&restored, parent_id, upgrade_id, held_id);

    // Death is distinct from cell unlinking. Static navigation keeps a marked
    // footprint until the real lifecycle owner removes it.
    restored
        .substrate
        .entities
        .get_mut(parent_id)
        .unwrap()
        .dying = true;
    assert!(restored.rebuild_dynamic_navigation(&rules));
    assert_only_marked_foundation(&restored);
    restored.uninit_with_rules(parent_id, &rules);
    assert!(
        !restored
            .substrate
            .entities
            .get(parent_id)
            .unwrap()
            .lifecycle
            .cell_marked
    );
    assert!(restored.rebuild_dynamic_navigation(&rules));
    assert!(restored.path_grid_snapshot().unwrap().is_walkable(6, 6));
}
