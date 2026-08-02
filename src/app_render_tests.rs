use super::HoverTargetKind;
use crate::app_entity_pick::{
    compute_box_selection_snapshot, compute_click_selection_snapshot, hover_target_at_point,
    pick_enemy_target_stable_id,
};
use crate::app_input::CLICK_SELECT_RADIUS;
use crate::app_sidebar_render::sync_targeting_mode;
use crate::map::entities::EntityCategory;
use crate::map::houses::HouseAllianceMap;
use crate::sim::components::Health;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::test_intern;
use crate::sim::production::ReadyBuildingView;
use crate::sim::vision::FogState;
use crate::sim::world::Simulation;
use std::collections::{BTreeMap, BTreeSet};

/// Spawn a mobile unit **at a cell**. Where it lands on screen is derived, so
/// tests take their click and box coordinates from [`screen_of`] rather than
/// inventing pixel values that no longer have anywhere to be stored.
fn spawn_mobile(store: &mut EntityStore, sid: u64, rx: u16, ry: u16, owner: &str, selected: bool) {
    let mut entity = GameEntity::new_at_frame_zero_for_test(
        sid,
        rx,
        ry,
        0,
        0,
        test_intern(owner),
        Health {
            current: 100,
            max: 100,
        },
        test_intern("E1"),
        EntityCategory::Unit,
        0,
        5,
        false,
    );
    entity.selected = selected;
    store.insert(entity);
}

/// Where an entity in `store` is drawn — the same answer the picking code gets.
fn screen_of(store: &EntityStore, sid: u64) -> (f32, f32) {
    crate::render::locomotor_visual::screen_position(store.get(sid).expect("entity"))
}

fn allied_fog_with_visible_cells(
    local_owner: &str,
    allied_owner: &str,
    visible_cells: &[(u16, u16)],
) -> FogState {
    let mut alliances = HouseAllianceMap::default();
    let allied_names = BTreeSet::from([
        local_owner.to_ascii_uppercase(),
        allied_owner.to_ascii_uppercase(),
    ]);
    alliances.insert(local_owner.to_ascii_uppercase(), allied_names.clone());
    alliances.insert(allied_owner.to_ascii_uppercase(), allied_names);

    let mut by_owner = BTreeMap::new();
    let mut visibility = crate::sim::vision::OwnerVisibility::new(64, 64);
    for &(rx, ry) in visible_cells {
        visibility.mark_visible(rx, ry);
    }
    by_owner.insert(crate::sim::intern::test_intern(allied_owner), visibility);

    FogState {
        width: 64,
        height: 64,
        by_owner,
        alliances,
        ..Default::default()
    }
}

#[test]
fn test_click_replace_selects_only_target() {
    let mut store = EntityStore::new();
    // Two cells apart is 67px, comfortably outside CLICK_SELECT_RADIUS, so a
    // click on one cannot also catch the other.
    spawn_mobile(&mut store, 1, 10, 10, "Americans", true);
    spawn_mobile(&mut store, 2, 12, 10, "Americans", false);
    let (cx, cy) = screen_of(&store, 2);

    let empty_heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let snapshot = compute_click_selection_snapshot(
        &store,
        None,
        None,
        cx,
        cy,
        CLICK_SELECT_RADIUS,
        false,
        None,
        &empty_heights,
        None,
        None,
    )
    .expect("snapshot");
    assert_eq!(snapshot, vec![2]);
}

#[test]
fn test_click_additive_toggles_membership() {
    let mut store = EntityStore::new();
    spawn_mobile(&mut store, 1, 10, 10, "Americans", true);
    spawn_mobile(&mut store, 2, 12, 10, "Americans", false);
    let (first_x, first_y) = screen_of(&store, 1);
    let (second_x, second_y) = screen_of(&store, 2);

    let empty_heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let added = compute_click_selection_snapshot(
        &store,
        None,
        None,
        second_x,
        second_y,
        CLICK_SELECT_RADIUS,
        true,
        None,
        &empty_heights,
        None,
        None,
    )
    .expect("snapshot");
    assert_eq!(added, vec![1, 2]);

    let removed = compute_click_selection_snapshot(
        &store,
        None,
        None,
        first_x,
        first_y,
        CLICK_SELECT_RADIUS,
        true,
        None,
        &empty_heights,
        None,
        None,
    )
    .expect("snapshot");
    assert_eq!(removed, Vec::<u64>::new());
}

#[test]
fn test_box_additive_toggles_and_excludes_structures() {
    let mut store = EntityStore::new();
    spawn_mobile(&mut store, 1, 10, 10, "Americans", true);
    spawn_mobile(&mut store, 2, 12, 10, "Americans", true);
    spawn_mobile(&mut store, 3, 14, 10, "Americans", false);
    let building = GameEntity::new_at_frame_zero_for_test(
        4,
        11,
        11,
        0,
        0,
        test_intern("Americans"),
        Health {
            current: 100,
            max: 100,
        },
        test_intern("GAPOWR"),
        EntityCategory::Structure,
        0,
        5,
        false,
    );
    store.insert(building);

    // The box covers all four: the units span (0,315)..(120,375) and the
    // building sits at (0,345).
    let snapshot =
        compute_box_selection_snapshot(&store, None, None, -40.0, 300.0, 160.0, 400.0, true, None)
            .expect("snapshot");
    assert_eq!(snapshot, vec![3]);
}

#[test]
fn test_box_replace_can_clear_selection_when_empty() {
    let mut store = EntityStore::new();
    spawn_mobile(&mut store, 1, 10, 10, "Americans", true);

    let snapshot =
        compute_box_selection_snapshot(&store, None, None, 300.0, 300.0, 340.0, 340.0, false, None)
            .expect("snapshot");
    assert!(snapshot.is_empty());
}

#[test]
fn test_click_selection_allows_visible_allied_units_for_local_owner() {
    let mut store = EntityStore::new();
    let entity = GameEntity::new_at_frame_zero_for_test(
        7,
        11,
        10,
        0,
        0,
        test_intern("British"),
        Health {
            current: 100,
            max: 100,
        },
        test_intern("E1"),
        EntityCategory::Unit,
        0,
        5,
        false,
    );
    store.insert(entity);
    let (cx, cy) = screen_of(&store, 7);

    let fog = allied_fog_with_visible_cells("Americans", "British", &[(11, 10)]);

    let empty_heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let snapshot = compute_click_selection_snapshot(
        &store,
        Some(&fog),
        Some("Americans"),
        cx,
        cy,
        CLICK_SELECT_RADIUS,
        false,
        None,
        &empty_heights,
        None,
        None,
    )
    .expect("snapshot");

    assert_eq!(snapshot, vec![7]);
}

#[test]
fn test_pick_enemy_target_ignores_hidden_entities() {
    let mut sim = Simulation::new();
    let soviet_id = sim.interner.intern("Soviet");
    let e1_id = sim.interner.intern("E1");

    let hidden = GameEntity::new_at_frame_zero_for_test(
        2,
        10,
        10,
        0,
        0,
        soviet_id,
        Health {
            current: 100,
            max: 100,
        },
        e1_id,
        EntityCategory::Unit,
        0,
        5,
        false,
    );
    sim.entities_mut().insert(hidden);
    let (hx, hy) =
        crate::render::locomotor_visual::screen_position(sim.entities().get(2).expect("hidden"));

    let empty_heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let picked_hidden =
        pick_enemy_target_stable_id(&sim, hx, hy, "Americans", false, None, &empty_heights, None);
    assert!(
        picked_hidden.is_none(),
        "Hidden enemy must not be targetable"
    );

    let visible = GameEntity::new_at_frame_zero_for_test(
        3,
        11,
        10,
        0,
        0,
        soviet_id,
        Health {
            current: 100,
            max: 100,
        },
        e1_id,
        EntityCategory::Unit,
        0,
        5,
        false,
    );
    sim.entities_mut().insert(visible);
    let (vx, vy) =
        crate::render::locomotor_visual::screen_position(sim.entities().get(3).expect("visible"));
    sim.fog
        .mark_visible_for_owner(crate::sim::intern::test_intern("Americans"), 11, 10);

    let picked_visible =
        pick_enemy_target_stable_id(&sim, vx, vy, "Americans", false, None, &empty_heights, None);
    assert_eq!(picked_visible, Some(3));

    let still_hidden =
        pick_enemy_target_stable_id(&sim, hx, hy, "Americans", false, None, &empty_heights, None);
    assert_ne!(still_hidden, Some(2));
}

#[test]
fn test_hover_target_distinguishes_friendly_and_enemy_categories() {
    let mut sim = Simulation::new();
    let americans_id = sim.interner.intern("Americans");
    let soviet_id = sim.interner.intern("Soviet");
    let gapowr_id = sim.interner.intern("GAPOWR");
    let e1_id = sim.interner.intern("E1");

    // Hover coordinates come from the drawn position, which is the cell
    // centre — the point `screen_to_iso` resolves back to the right cell.
    let friendly = GameEntity::new_at_frame_zero_for_test(
        10,
        5,
        5,
        0,
        0,
        americans_id,
        Health {
            current: 100,
            max: 100,
        },
        gapowr_id,
        EntityCategory::Structure,
        0,
        5,
        false,
    );
    sim.entities_mut().insert(friendly);
    let (friendly_sx, friendly_sy) =
        crate::render::locomotor_visual::screen_position(sim.entities().get(10).expect("friendly"));

    let enemy = GameEntity::new_at_frame_zero_for_test(
        11,
        20,
        5,
        0,
        0,
        soviet_id,
        Health {
            current: 100,
            max: 100,
        },
        e1_id,
        EntityCategory::Unit,
        0,
        5,
        false,
    );
    sim.entities_mut().insert(enemy);
    let (enemy_sx, enemy_sy) =
        crate::render::locomotor_visual::screen_position(sim.entities().get(11).expect("enemy"));
    sim.fog
        .mark_visible_for_owner(crate::sim::intern::test_intern("Americans"), 20, 5);

    // Provide empty height maps — structure picking now uses foundation-based hit testing.
    let empty_heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let friendly_hover = hover_target_at_point(
        &sim,
        friendly_sx,
        friendly_sy,
        "Americans",
        false,
        None,
        &empty_heights,
        None,
    )
    .expect("friendly hover");
    assert_eq!(friendly_hover.kind, HoverTargetKind::FriendlyStructure);
    assert_eq!(friendly_hover.stable_id, 10);

    let enemy_hover = hover_target_at_point(
        &sim,
        enemy_sx,
        enemy_sy,
        "Americans",
        false,
        None,
        &empty_heights,
        None,
    )
    .expect("enemy hover");
    assert_eq!(enemy_hover.kind, HoverTargetKind::EnemyUnit);
    assert_eq!(enemy_hover.stable_id, 11);
}

#[test]
fn test_ready_buildings_do_not_auto_arm_placement() {
    let mut armed: Option<crate::app_types::TargetingMode> = None;
    let mut preview = None;
    let ready = vec![ReadyBuildingView {
        type_id: crate::sim::intern::test_intern("GAPOWR"),
        display_name: "Power Plant".to_string(),
        queue_category: crate::sim::production::ProductionCategory::Building,
    }];

    sync_targeting_mode(&mut armed, &mut preview, &ready, &[], None);

    assert!(
        armed.is_none(),
        "ready building should not auto-arm placement"
    );
    assert!(preview.is_none());
}

#[test]
fn test_invalid_armed_building_clears_when_not_ready() {
    let mut armed = Some(crate::app_types::TargetingMode::BuildingPlacement(
        "GAPOWR".to_string(),
    ));
    let mut preview = Some(crate::sim::production::BuildingPlacementPreview {
        type_id: crate::sim::intern::test_intern("GAPOWR"),
        rx: 5,
        ry: 5,
        width: 2,
        height: 2,
        valid: false,
        reason: None,
        cell_valid: vec![false; 4],
    });

    sync_targeting_mode(&mut armed, &mut preview, &[], &[], None);

    assert!(armed.is_none());
    assert!(preview.is_none());
}

#[test]
fn test_sw_armed_preserved_when_ready() {
    use crate::sim::superweapon::SuperWeaponView;
    let mut armed = Some(crate::app_types::TargetingMode::SuperWeapon(
        "LightningStormSpecial".to_string(),
    ));
    let mut preview = None;
    let sw = SuperWeaponView {
        type_id: crate::sim::intern::test_intern("LightningStormSpecial"),
        display_name: "LightningStormSpecial".to_string(),
        progress: 1.0,
        is_ready: true,
        is_online: true,
        sidebar_image: Some("INTICON".to_string()),
        kind: crate::rules::superweapon_type::SuperWeaponKind::LightningStorm,
    };

    sync_targeting_mode(&mut armed, &mut preview, &[], &[sw], None);

    assert!(armed.is_some(), "armed SW should be preserved while ready");
}

#[test]
fn test_sw_armed_cleared_when_not_ready() {
    use crate::sim::superweapon::SuperWeaponView;
    let mut armed = Some(crate::app_types::TargetingMode::SuperWeapon(
        "LightningStormSpecial".to_string(),
    ));
    let mut preview = None;
    let sw = SuperWeaponView {
        type_id: crate::sim::intern::test_intern("LightningStormSpecial"),
        display_name: "LightningStormSpecial".to_string(),
        progress: 0.5,
        is_ready: false, // Charging, not yet ready.
        is_online: true,
        sidebar_image: Some("INTICON".to_string()),
        kind: crate::rules::superweapon_type::SuperWeaponKind::LightningStorm,
    };

    sync_targeting_mode(&mut armed, &mut preview, &[], &[sw], None);

    assert!(armed.is_none(), "armed SW should clear when not ready");
}

#[test]
fn test_sw_armed_cleared_when_view_gone() {
    let mut armed = Some(crate::app_types::TargetingMode::SuperWeapon(
        "LightningStormSpecial".to_string(),
    ));
    let mut preview = None;

    // No SW views — granting building destroyed.
    sync_targeting_mode(&mut armed, &mut preview, &[], &[], None);

    assert!(
        armed.is_none(),
        "armed SW should clear when view disappears"
    );
}
