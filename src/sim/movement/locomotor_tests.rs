//! Locomotor unit tests — verifies locomotor state initialization, speed type mapping,
//! and ObjectType-to-LocomotorState conversion for various unit categories.

use super::*;
use crate::rules::jumpjet_params::JumpjetParams;
use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::rules::object_type::{ObjectCategory, ObjectType, PipScale};
use crate::util::fixed_math::{SIM_ONE, SimFixed, sim_from_f32};

/// Helper to create a minimal ObjectType with the given locomotor.
fn make_obj(locomotor: LocomotorKind, category: ObjectCategory) -> ObjectType {
    ObjectType {
        id: "TEST".to_string(),
        category,
        name: None,
        ui_name: None,
        cost: 0,
        trainable: true,
        explosion_anims: Vec::new(),
        destroy_anims: Vec::new(),
        strength: 100,
        dont_score: false,
        special_threat_value: 0.0,
        armor: "none".to_string(),
        speed: 6,
        walk_rate: 1,
        idle_rate: 0,
        weight: SimFixed::lit("2.0"),
        accel_factor: SimFixed::lit("0.03"),
        decel_factor: SimFixed::lit("0.02"),
        accelerates: true,
        slowdown_distance: 512,
        sight: 5,
        tech_level: -1,
        build_time_multiplier: 1.0,
        build_time_multiplier_x1000: 1000,
        owner: vec![],
        required_houses: vec![],
        forbidden_houses: vec![],
        allowed_to_start_in_multiplayer: true,
        prerequisite: vec![],
        prerequisite_override: vec![],
        build_limit: 0,
        requires_stolen_allied_tech: false,
        requires_stolen_soviet_tech: false,
        requires_stolen_third_tech: false,
        primary: None,
        secondary: None,
        elite_primary: None,
        elite_secondary: None,
        fire_up_frame: 0,
        fire_prone_frame: 0,
        secondary_fire_frame: 0,
        secondary_prone_frame: 0,
        image: "TEST".to_string(),
        power: 0,
        extra_power: 0,
        foundation: "1x1".to_string(),
        pixel_selection_bracket_delta: 0,
        build_cat: None,
        adjacent: 6,
        protect_with_wall: false,
        wants_extra_space: false,
        base_normal: true,
        eligibile_for_ally_building: false,
        crewed: false,
        voice_select: None,
        voice_move: None,
        voice_attack: None,
        voice_harvest: None,
        voice_enter: None,
        voice_capture: None,
        prevent_attack_move: false,
        voice_die: Vec::new(),
        die_sounds: Vec::new(),
        move_sound: None,
        voice_feedback: None,
        voice_special_attack: None,
        crush_sound: None,
        deploy_sound: None,
        undeploy_sound: None,
        chrono_in_sound: None,
        chrono_out_sound: None,
        has_turret: false,
        turret_rot: 0,
        turret_anim: None,
        turret_anim_is_voxel: false,
        turret_anim_x: 0,
        turret_anim_y: 0,
        turret_anim_z_adjust: 0,
        guard_range: None,
        air_range_bonus: None,
        opportunity_fire: false,
        can_retaliate: true,
        can_passive_acquire: true,
        distributed_fire: false,
        explodes: false,
        veteran_explodes: false,
        elite_explodes: false,
        veteran_stronger: false,
        elite_stronger: false,
        veteran_scatter: false,
        elite_scatter: false,
        veteran_cloak: false,
        elite_cloak: false,
        veteran_crusher: false,
        elite_crusher: false,
        death_weapon: None,
        death_weapon_damage_modifier: 1.0,
        super_weapon: None,
        super_weapon2: None,
        spy_sat: false,
        gap_generator: false,
        psychic_detection_radius: 0,
        sensor_array: false,
        sensors: false,
        sensors_sight: 0,
        cloakable: false,
        cloaking_speed: 1,
        cloak_stop: false,
        cloak_radius_in_cells: 20,
        cloak_generator: false,
        radar: false,
        radar_invisible: false,
        veteran_radar_invisible: false,
        elite_radar_invisible: false,
        radar_visible: false,
        insignificant: false,
        harvester: false,
        refinery: false,
        weeder: false,
        bib: false,
        gate: false,
        deploy_time_ticks: 0,
        gate_close_delay_ticks: 0,
        storage: 0,
        free_unit: None,
        dock: vec![],
        queueing_cell: None,
        pads: Vec::new(),
        hidden_occupancy: crate::rules::object_type::BuildingHiddenOccupancyProfile::default(),
        base_reservation_spacing: None,
        unloading_class: None,
        ammo: -1,
        spawns: None,
        spawns_number: 0,
        spawn_regen_rate: 0,
        spawn_reload_rate: 0,
        missile_spawn: false,
        no_spawn_alt: false,
        enslaves: None,
        slaves_number: 0,
        slave_regen_rate: 0,
        slave_reload_rate: 0,
        slaved: false,
        fearless: false,
        fraidycat: false,
        crawls: false,
        veteran_fearless: false,
        elite_fearless: false,
        harvest_rate: 0,
        resource_gatherer: false,
        resource_destination: false,
        ore_purifier: false,
        locomotor,
        speed_type: SpeedType::Track,
        movement_zone: MovementZone::Normal,
        movement_restricted_to: None,
        considered_aircraft: false,
        zfudge_bridge: 7,
        too_big_to_fit_under_bridge: false,
        crashable: false,
        teleporter: false,
        hover_attack: false,
        balloon_hover: false,
        airport_bound: false,
        fighter: false,
        fly_by: false,
        fly_back: false,
        landable: false,
        jumpjet: false,
        jumpjet_params: None,
        deploys_into: None,
        undeploys_into: None,
        deploy_facing: 0x80,
        construction_yard: false,
        factory: None,
        weapons_factory: false,
        cloning: false,
        exit_coord: None,
        crushable: false,
        deployed_crushable: true,
        crusher: false,
        no_force_shield: false,
        omni_crusher: false,
        omni_crush_resistant: false,
        immune_to_radiation: false,
        damage_self: false,
        immune: false,
        type_immune: false,
        immune_to_psionics: false,
        immune_to_psionic_weapons: false,
        immune_to_poison: false,
        engineer: false,
        deployer: false,
        capturable: false,
        repairable: true,
        can_be_occupied: false,
        can_occupy_fire: false,
        show_occupant_pips: false,
        bridge_repair_hut: false,
        laser_fence: false,
        passengers: 0,
        size_limit: 0,
        size: 3,
        open_topped: false,
        gunner: false,
        ifv_mode: 0,
        open_transport_weapon: -1,
        deploy_fire: false,
        deploy_fire_weapon: None,
        max_number_occupants: 0,
        occupier: false,
        assaulter: false,
        occupy_weapon: None,
        elite_occupy_weapon: None,
        occupy_pip: 7,
        pip_scale: PipScale::None,
        infantry_absorb: false,
        unit_absorb: false,
        bunkerable: category == ObjectCategory::Vehicle,
        weapon_list: vec![],
        attack_cursor_on_friendlies: false,
        sabotage_cursor: false,
        c4: false,
        can_c4: false,
        eligible_for_delay_kill: false,
        invisible: false,
        invisible_in_game: false,
        unit_repair: false,
        bunker: false,
        unit_reload: false,
        helipad: false,
        number_of_docks: 1,
        toggle_power: false,
        powered: false,
        can_disguise: false,
        disguise_when_still: false,
        wall: false,
        to_overlay: None,
        unsellable: false,
        click_repairable: true,
        selectable: true,
        light_visibility: 0,
        light_intensity: 0.0,
        has_spotlight: false,
        light_red_tint: 1.0,
        light_green_tint: 1.0,
        light_blue_tint: 1.0,
        water_bound: false,
        naval: false,
        number_impassable_rows: -1,
        natural_particle_system: None,
        natural_particle_location: glam::IVec3::ZERO,
        refinery_smoke_particle_system: None,
        damage_particle_systems: Vec::new(),
        max_debris: 0,
        min_debris: 0,
        debris_types: Vec::new(),
        debris_maximums: Vec::new(),
        debris_anims: Vec::new(),
        close_range: false,
        cyborg: false,
        destroy_particle_systems: Vec::new(),
        damage_smoke_offset: glam::IVec3::ZERO,
        dam_smk_off_scrn_rel: false,
        destroy_smoke_offset: glam::IVec3::ZERO,
        refinery_smoke_offsets: [glam::IVec3::ZERO; 4],
        refinery_smoke_frames: 0,
        gap_radius_in_cells: 0,
        super_gap_radius_in_cells: 0,
    }
}

#[test]
fn test_drive_locomotor() {
    let obj = make_obj(LocomotorKind::Drive, ObjectCategory::Vehicle);
    let state = LocomotorState::from_object_type(&obj, 1500);
    assert_eq!(state.kind, LocomotorKind::Drive);
    assert_eq!(state.layer, MovementLayer::Ground);
    assert_eq!(state.phase, GroundMovePhase::Idle);
    assert_eq!(state.air_phase, AirMovePhase::Landed);
    assert_eq!(state.speed_multiplier, SIM_ONE);
    assert!(state.is_ground_mover());
    assert!(!state.is_air_mover());
}

#[test]
fn test_hover_cruises_at_full_base_speed() {
    // Hover now cruises at its full base Speed (throttle 1.0), not the old
    // made-up 0.65x. The accel/brake throttle ramp lives in sim/movement/hover.rs.
    let obj = make_obj(LocomotorKind::Hover, ObjectCategory::Vehicle);
    let state = LocomotorState::from_object_type(&obj, 1500);
    assert_eq!(state.kind, LocomotorKind::Hover);
    assert_eq!(state.speed_multiplier, SIM_ONE);
    assert!(state.is_ground_mover());
}

#[test]
fn test_walk_locomotor() {
    let obj = make_obj(LocomotorKind::Walk, ObjectCategory::Infantry);
    let state = LocomotorState::from_object_type(&obj, 1500);
    assert_eq!(state.kind, LocomotorKind::Walk);
    assert_eq!(state.layer, MovementLayer::Ground);
    assert!(state.is_ground_mover());
}

#[test]
fn test_fly_locomotor_air_layer() {
    let obj = make_obj(LocomotorKind::Fly, ObjectCategory::Aircraft);
    let state = LocomotorState::from_object_type(&obj, 1500);
    assert_eq!(state.kind, LocomotorKind::Fly);
    assert_eq!(state.layer, MovementLayer::Air);
    assert_eq!(state.air_phase, AirMovePhase::Landed);
    assert!(!state.is_ground_mover());
    assert!(state.is_air_mover());
    assert_eq!(state.target_altitude, SimFixed::from_num(1500));
    assert_eq!(state.climb_rate, FLY_CLIMB_RATE);
}

#[test]
fn test_jumpjet_air_layer() {
    let obj = make_obj(LocomotorKind::Jumpjet, ObjectCategory::Infantry);
    let state = LocomotorState::from_object_type(&obj, 1500);
    assert_eq!(state.kind, LocomotorKind::Jumpjet);
    assert_eq!(state.layer, MovementLayer::Air);
    assert!(!state.is_ground_mover());
    assert!(state.is_air_mover());
    assert_eq!(state.target_altitude, SimFixed::from_num(500));
}

#[test]
fn test_jumpjet_with_custom_params() {
    let mut obj = make_obj(LocomotorKind::Jumpjet, ObjectCategory::Infantry);
    obj.jumpjet = true;
    obj.jumpjet_params = Some(JumpjetParams {
        turn_rate: 4,
        speed: sim_from_f32(20.0),
        climb: sim_from_f32(8.0),
        crash: sim_from_f32(5.0),
        height: 750,
        accel: sim_from_f32(2.0),
        wobbles: 0.2,
        deviation: 40,
        no_wobbles: false,
    });
    let state = LocomotorState::from_object_type(&obj, 1500);
    assert_eq!(state.target_altitude, SimFixed::from_num(750));
    assert_eq!(state.jumpjet_speed, sim_from_f32(20.0));
    assert_eq!(state.climb_rate, sim_from_f32(8.0) * SimFixed::from_num(15));
}

#[test]
fn test_ship_is_ground_mover() {
    let obj = make_obj(LocomotorKind::Ship, ObjectCategory::Vehicle);
    let state = LocomotorState::from_object_type(&obj, 1500);
    assert_eq!(state.kind, LocomotorKind::Ship);
    assert!(state.is_ground_mover());
    assert!(!state.is_air_mover());
}

#[test]
fn cmin_locomotor_initializes_primary_and_active_teleport() {
    let mut obj = make_obj(LocomotorKind::Teleport, ObjectCategory::Vehicle);
    obj.harvester = true;
    obj.teleporter = true;
    obj.turret_rot = 5;

    let state = LocomotorState::from_object_type(&obj, 1500);

    assert_eq!(state.active_kind(), LocomotorKind::Teleport);
    assert_eq!(state.effective_kind(), LocomotorKind::Teleport);
    assert!(state.is_primary_active());
    assert_eq!(state.rot, 5);
}

#[test]
fn test_is_airborne() {
    let obj = make_obj(LocomotorKind::Fly, ObjectCategory::Aircraft);
    let mut state = LocomotorState::from_object_type(&obj, 1500);
    assert!(!state.is_airborne());
    state.altitude = SimFixed::from_num(100);
    assert!(state.is_airborne());
}

// --- Override/Piggyback mechanism tests ---

#[test]
fn test_override_teleport_round_trip() {
    let obj = make_obj(LocomotorKind::Drive, ObjectCategory::Vehicle);
    let mut state = LocomotorState::from_object_type(&obj, 1500);
    assert!(!state.is_overridden());
    assert_eq!(state.kind, LocomotorKind::Drive);
    assert_eq!(state.layer, MovementLayer::Ground);

    // Begin teleport override.
    state.begin_piggyback(LocomotorKind::Teleport, MovementLayer::Ground);
    assert!(state.is_overridden());
    assert_eq!(state.kind, LocomotorKind::Teleport);
    assert_eq!(state.layer, MovementLayer::Ground);

    // End override — should restore Drive.
    assert!(state.end_piggyback());
    assert!(!state.is_overridden());
    assert_eq!(state.kind, LocomotorKind::Drive);
    assert_eq!(state.layer, MovementLayer::Ground);
    assert_eq!(state.speed_multiplier, SIM_ONE);
}

#[test]
fn end_piggyback_without_a_stash_reports_nothing_to_pop() {
    let obj = make_obj(LocomotorKind::Drive, ObjectCategory::Vehicle);
    let mut state = LocomotorState::from_object_type(&obj, 1500);
    let result = state.end_piggyback();
    assert!(
        !result,
        "ending with nothing stashed reports nothing to pop"
    );
    assert_eq!(state.kind, LocomotorKind::Drive);
}

#[test]
fn test_override_preserves_speed_type() {
    let mut obj = make_obj(LocomotorKind::Drive, ObjectCategory::Vehicle);
    obj.speed_type = SpeedType::Wheel;
    let mut state = LocomotorState::from_object_type(&obj, 1500);
    assert_eq!(state.speed_type, SpeedType::Wheel);

    state.begin_piggyback(LocomotorKind::Teleport, MovementLayer::Ground);
    // SpeedType should still reflect the original during override.
    state.end_piggyback();
    assert_eq!(state.speed_type, SpeedType::Wheel);
}

#[test]
fn drive_piggyback_restores_primary_teleport_only_after_not_moving() {
    let obj = make_obj(LocomotorKind::Teleport, ObjectCategory::Vehicle);
    let mut state = LocomotorState::from_object_type(&obj, 1500);

    assert!(state.begin_drive_piggyback_for_teleporter());
    assert_eq!(state.active_kind(), LocomotorKind::Drive);
    assert_eq!(state.effective_kind(), LocomotorKind::Teleport);
    assert!(!state.can_restore_primary_from_piggyback(true, false, false));
    assert!(!state.can_restore_primary_from_piggyback(false, true, false));
    assert!(!state.can_restore_primary_from_piggyback(false, false, true));
    assert!(state.can_restore_primary_from_piggyback(false, false, false));

    assert!(state.restore_primary_from_piggyback());
    assert_eq!(state.active_kind(), LocomotorKind::Teleport);
    assert_eq!(state.effective_kind(), LocomotorKind::Teleport);
    assert!(state.is_primary_active());
}

#[test]
fn drive_piggyback_refuses_an_unstashed_active_drive() {
    let obj = make_obj(LocomotorKind::Teleport, ObjectCategory::Vehicle);
    let mut state = LocomotorState::from_object_type(&obj, 1500);
    state.kind = LocomotorKind::Drive;

    assert!(!state.begin_drive_piggyback_for_teleporter());
    assert_eq!(state.kind, LocomotorKind::Drive);
    assert!(state.piggyback.is_none());
}
