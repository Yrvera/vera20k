use super::*;

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
use crate::rules::ini_parser::IniFile;
use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};
use crate::sim::cell_rect::PlayfieldBounds;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

const MAP: u16 = 24;

fn registry() -> OverlayTypeRegistry {
    OverlayTypeRegistry::from_ini(
        &IniFile::from_str(
            "[OverlayTypes]\n0=OTHER\n1=SILVER\n2=WOOD\n3=WATER\n4=MODCRATE\n\
             [OTHER]\nWall=yes\n[SILVER]\nCrate=yes\n[WOOD]\nCrate=yes\n\
             [WATER]\nCrate=yes\n[MODCRATE]\nCrate=yes\n",
        ),
        None,
    )
}

fn rules(extra: &str) -> crate::rules::ruleset::RuleSet {
    crate::rules::ruleset::RuleSet::from_ini(&IniFile::from_str(&format!(
        "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
         [CrateRules]\nCrateImg=SILVER\nWoodCrateImg=WOOD\nWaterCrateImg=WATER\n\
         CrateRegen=3\n{extra}",
    )))
    .expect("crate rules")
}

fn speed_costs() -> SpeedCostProfile {
    SpeedCostProfile {
        foot: Some(100),
        track: Some(100),
        wheel: Some(100),
        float: Some(100),
        amphibious: Some(100),
        float_beach: Some(100),
        hover: Some(100),
    }
}

fn terrain(water: bool) -> ResolvedTerrainGrid {
    let land_type = if water {
        LandType::Water.as_index()
    } else {
        LandType::Clear.as_index()
    };
    let template = ResolvedTerrainCell {
        rx: 0,
        ry: 0,
        source_tile_index: 0,
        source_sub_tile: 0,
        final_tile_index: 0,
        final_sub_tile: 0,
        is_wood_bridge_repair_tile: false,
        level: 0,
        filled_clear: false,
        tileset_index: Some(0),
        land_type,
        yr_cell_land_type: land_type,
        slope_type: 0,
        template_height: 0,
        height_in_pixels: 0,
        render_offset_x: 0,
        render_offset_y: 0,
        terrain_class: TerrainClass::Clear,
        speed_costs: speed_costs(),
        is_water: water,
        is_cliff_like: false,
        is_rough: false,
        is_road: false,
        accepts_smudge: false,
        allows_tiberium: true,
        variant: 0,
        has_ramp: false,
        canonical_ramp: None,
        ground_walk_blocked: false,
        terrain_object_blocks: false,
        terrain_object_occupation: None,
        overlay_blocks: false,
        overlay_zone_type: None,
        outside_playfield: false,
        zone_type: zone_class::GROUND,
        base_ground_walk_blocked: false,
        base_build_blocked: false,
        base_land_type: land_type,
        base_yr_cell_land_type: land_type,
        base_terrain_class: TerrainClass::Clear,
        base_speed_costs: speed_costs(),
        build_blocked: false,
        has_bridge_deck: false,
        bridge_walkable: false,
        bridge_transition: false,
        bridge_deck_level: 0,
        bridge_layer: None,
        bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
        tube_index: None,
        radar_left: [0; 3],
        radar_right: [0; 3],
        has_damaged_data: false,
        bridgehead_anchor_class_at_load: None,
    };
    let cells = (0..MAP)
        .flat_map(|ry| {
            let value = template.clone();
            (0..MAP).map(move |rx| ResolvedTerrainCell {
                rx,
                ry,
                ..value.clone()
            })
        })
        .collect();
    ResolvedTerrainGrid::from_cells(MAP, MAP, cells)
}

fn sim(seed: u64, water: bool) -> Simulation {
    let mut sim = Simulation::with_seed(seed);
    sim.session.map_width = MAP;
    sim.session.map_height = MAP;
    sim.session.game_options.crates = true;
    sim.session.game_mode_nonzero = true;
    sim.playfield_bounds = Some(PlayfieldBounds {
        base: 0,
        off_fc: -128,
        off_100: -128,
        off_104: 256,
        off_108: 256,
    });
    sim.overlay_grid = Some(OverlayGrid::new(MAP, MAP));
    sim.resolved_terrain = Some(terrain(water));
    sim
}

#[test]
fn active_retail_crate_slots_start_with_exact_native_words() {
    let sim = Simulation::new();
    assert!(!sim.crate_authority.pickup_any_latch);
    assert_eq!(sim.crate_authority.slots.len(), CRATE_SLOT_CAPACITY);
    assert!(sim.crate_authority.slots.iter().all(|slot| {
        *slot
            == CrateSlot {
                start_frame: -1,
                timer_aux: 0,
                duration_frames: 0,
                cell_x: 0,
                cell_y: 0,
            }
    }));
}

#[test]
fn active_retail_random_placement_uses_first_slot_and_x_y_timer_rng_order() {
    let rules = rules("");
    let registry = registry();
    let path = PathGrid::test_all_passable(MAP, MAP);
    let mut sim = sim(0xABCD, false);
    let mut expected_rng = sim.scenario_rng.clone();
    let expected_cell = (
        expected_rng.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
        expected_rng.next_range_u32_inclusive(1, u32::from(MAP - 1)) as u16,
    );
    let expected_timer = placement::build_crate_timer(&mut expected_rng, rules.crate_rules.regen);

    assert!(sim.place_random_crate(&rules, &registry, Some(&path)));

    let slot = sim.crate_authority.slots[0];
    assert_eq!(
        slot.cell(),
        Some((expected_cell.0 as i16, expected_cell.1 as i16))
    );
    assert_eq!(slot.start_frame, 0);
    assert_eq!(slot.duration_frames, expected_timer.duration);
    assert_eq!(slot.timer_aux, expected_timer.aux);
    assert_eq!(expected_timer.aux, 0x40B5_1800);
    assert_eq!(
        sim.scenario_rng.logical_state(),
        expected_rng.logical_state()
    );
    assert_eq!(
        sim.crate_presentation,
        vec![
            CratePresentationEvent::DirtyScreenRect {
                cell: Some(expected_cell),
                force: false,
            },
            CratePresentationEvent::CellRedraw {
                cell: expected_cell,
                frame: 0,
            },
        ]
    );
}

#[test]
fn active_retail_timer_interpolation_has_verified_endpoint_words() {
    use crate::util::native_x87::NativeF64Bits;

    fn timer_at(draw: u32) -> (i32, u32) {
        let regen = NativeF64Bits::from_bits(3.0_f64.to_bits());
        let regen = crate::util::native_x87::X87Chop53::load_f64(regen).unwrap();
        let lower = crate::util::native_x87::X87Chop53::mul(
            regen,
            crate::util::native_x87::X87Chop53::load_i32(450),
        );
        let upper = crate::util::native_x87::X87Chop53::mul(
            regen,
            crate::util::native_x87::X87Chop53::load_i32(1800),
        );
        let ratio = crate::util::native_x87::X87Chop53::div(
            crate::util::native_x87::X87Chop53::load_i32(draw as i32),
            crate::util::native_x87::X87Chop53::load_i32(0x7fff_fffe),
        )
        .unwrap();
        let value = crate::util::native_x87::X87Chop53::add(
            lower,
            crate::util::native_x87::X87Chop53::mul(
                ratio,
                crate::util::native_x87::X87Chop53::sub(upper, lower),
            ),
        );
        (
            crate::util::native_x87::X87Chop53::ftol_i64(value).unwrap() as i32,
            (crate::util::native_x87::X87Chop53::store_f64(upper)
                .unwrap()
                .bits()
                >> 32) as u32,
        )
    }

    assert_eq!(timer_at(0), (1350, 0x40B5_1800));
    assert_eq!(timer_at(0x7fff_fffe), (5400, 0x40B5_1800));
}

#[test]
fn active_retail_post_precheck_failures_are_timed_ghosts_and_write_specific_low_byte_late() {
    let rules = rules("");
    let registry = registry();
    let path = PathGrid::test_all_passable(MAP, MAP);
    for faults in [
        CratePlacementFaults {
            allocation: true,
            ..Default::default()
        },
        CratePlacementFaults {
            construction: true,
            ..Default::default()
        },
        CratePlacementFaults {
            unlimbo: true,
            ..Default::default()
        },
        CratePlacementFaults {
            mark: true,
            ..Default::default()
        },
    ] {
        let mut sim = sim(7, false);
        sim.session.binary_frame = 23;
        let before_rng = sim.scenario_rng.logical_state();
        assert!(sim.place_specific_crate_with_faults(
            &rules,
            &registry,
            Some(&path),
            (8, 9),
            0x114,
            faults,
        ));
        let slot = sim.crate_authority.slots[0];
        assert_eq!(slot.cell(), Some((8, 9)));
        assert_eq!(slot.start_frame, 23);
        assert!(slot.duration_frames >= 1350 && slot.duration_frames <= 5400);
        let cell = sim.overlay_grid.as_ref().unwrap().cell(8, 9);
        assert_eq!(cell.overlay_id, None);
        assert_eq!(cell.overlay_data, 0x14);
        assert_ne!(sim.scenario_rng.logical_state(), before_rng);
        assert_eq!(
            sim.crate_presentation[..2],
            [
                CratePresentationEvent::DirtyScreenRect {
                    cell: None,
                    force: false,
                },
                CratePresentationEvent::CellRedraw {
                    cell: (8, 9),
                    frame: 23,
                },
            ]
        );
    }
}

#[test]
fn active_retail_specific_full_dword_sentinel_is_not_a_low_byte_test() {
    let rules = rules("");
    let registry = registry();
    let path = PathGrid::test_all_passable(MAP, MAP);

    let mut exact = sim(11, false);
    assert!(exact.place_specific_crate(&rules, &registry, Some(&path), (5, 6), 0x14));
    assert_eq!(
        exact.overlay_grid.as_ref().unwrap().cell(5, 6).overlay_data,
        u8::MAX
    );

    let mut wider = sim(11, false);
    assert!(wider.place_specific_crate(&rules, &registry, Some(&path), (5, 6), 0x114));
    assert_eq!(
        wider.overlay_grid.as_ref().unwrap().cell(5, 6).overlay_data,
        0x14
    );
}

#[test]
fn active_retail_water_image_uses_float_while_land_uses_track() {
    let rules = rules("");
    let registry = registry();
    let path = PathGrid::test_all_passable(MAP, MAP);
    let water_id = registry.id_for_name("WATER").unwrap();
    let wood_id = registry.id_for_name("WOOD").unwrap();

    let mut water = sim(1, true);
    water
        .resolved_terrain
        .as_mut()
        .unwrap()
        .cell_mut(4, 4)
        .unwrap()
        .speed_costs
        .track = Some(0);
    assert!(water.place_specific_crate(&rules, &registry, Some(&path), (4, 4), 0x14));
    assert_eq!(
        water.overlay_grid.as_ref().unwrap().cell(4, 4).overlay_id,
        Some(water_id)
    );

    let mut land = sim(1, false);
    land.resolved_terrain
        .as_mut()
        .unwrap()
        .cell_mut(4, 4)
        .unwrap()
        .speed_costs
        .float = Some(0);
    assert!(land.place_specific_crate(&rules, &registry, Some(&path), (4, 4), 0x14));
    assert_eq!(
        land.overlay_grid.as_ref().unwrap().cell(4, 4).overlay_id,
        Some(wood_id)
    );
}

#[test]
fn active_retail_slot_clear_preserves_timer_and_unrelated_cell_state() {
    let rules = rules("");
    let registry = registry();
    let path = PathGrid::test_all_passable(MAP, MAP);
    let mut sim = sim(1, false);
    sim.session.binary_frame = 10;
    assert!(sim.place_specific_crate(&rules, &registry, Some(&path), (7, 8), 0x14));
    sim.crate_presentation.clear();
    let owner = sim.interner.intern("OWNER");
    sim.overlay_grid.as_mut().unwrap().cell_mut(7, 8).wall_owner = Some(owner);
    sim.crate_authority.slots[0].duration_frames = 100;
    sim.session.binary_frame = 35;

    assert!(sim.remove_crate_at_cell(&rules, &registry, (7, 8)));

    let slot = sim.crate_authority.slots[0];
    assert!(!slot.is_occupied());
    assert_eq!(slot.start_frame, -1);
    assert_eq!(slot.duration_frames, 75);
    let cell = sim.overlay_grid.as_ref().unwrap().cell(7, 8);
    assert_eq!(
        (cell.overlay_id, cell.overlay_data, cell.wall_owner),
        (None, 0, Some(owner))
    );
    assert_eq!(
        sim.crate_presentation,
        vec![CratePresentationEvent::DirtyScreenRect {
            cell: Some((7, 8)),
            force: false,
        }]
    );
    assert!(sim.radar_terrain_dirty_cells.is_empty());
}

#[test]
fn active_retail_mode_zero_removes_any_live_crate_type_without_a_slot() {
    let rules = rules("");
    let registry = registry();
    let mod_id = registry.id_for_name("MODCRATE").unwrap();
    let mut sim = sim(1, false);
    sim.session.game_mode_nonzero = false;
    sim.overlay_grid
        .as_mut()
        .unwrap()
        .place_overlay(3, 4, mod_id, 77);
    sim.crate_authority = CrateAuthority::default();

    assert!(sim.remove_crate_at_cell(&rules, &registry, (3, 4)));
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(3, 4).overlay_id,
        None
    );
    assert!(
        sim.crate_authority
            .slots
            .iter()
            .all(|slot| !slot.is_occupied())
    );
}

#[test]
fn active_retail_full_slot_table_spends_no_rng_and_duplicate_cells_are_allowed() {
    let rules = rules("");
    let registry = registry();
    let path = PathGrid::test_all_passable(MAP, MAP);
    let mut sim = sim(17, false);
    for slot in &mut sim.crate_authority.slots {
        slot.cell_x = 6;
        slot.cell_y = 6;
    }
    let before = sim.scenario_rng.logical_state();
    assert!(!sim.place_random_crate(&rules, &registry, Some(&path)));
    assert_eq!(sim.scenario_rng.logical_state(), before);
    assert_eq!(
        sim.crate_authority.slots[0].cell(),
        sim.crate_authority.slots[255].cell()
    );
}

#[test]
fn active_retail_regeneration_gates_and_replaces_expired_slot_immediately() {
    let mut rules = rules("");
    rules.crate_rules.regen = crate::util::native_x87::NativeF64Bits::POSITIVE_ZERO;
    let registry = registry();
    let path = PathGrid::test_all_passable(MAP, MAP);
    let mut sim = sim(99, false);
    assert!(sim.place_specific_crate(&rules, &registry, Some(&path), (8, 8), 0x14));
    assert_eq!(sim.crate_authority.slots[0].duration_frames, 0);

    sim.session.game_mode_nonzero = false;
    let paused_rng = sim.scenario_rng.logical_state();
    sim.update_crate_regeneration(&rules, &registry, Some(&path));
    assert_eq!(sim.scenario_rng.logical_state(), paused_rng);
    assert_eq!(sim.crate_authority.slots[0].cell(), Some((8, 8)));

    sim.session.game_mode_nonzero = true;
    sim.update_crate_regeneration(&rules, &registry, Some(&path));
    assert_ne!(sim.scenario_rng.logical_state(), paused_rng);
    assert!(sim.crate_authority.slots[0].is_occupied());
    assert_ne!(sim.crate_authority.slots[0].cell(), Some((8, 8)));
}

#[test]
fn active_retail_crate_slots_and_pickup_latch_change_future_state_hash() {
    let baseline = Simulation::new();
    let mut slot = Simulation::new();
    slot.crate_authority.slots[17].cell_x = 9;
    slot.crate_authority.slots[17].duration_frames = -7;
    let mut latch = Simulation::new();
    latch.crate_authority.pickup_any_latch = true;
    assert_ne!(baseline.state_hash(), slot.state_hash());
    assert_ne!(baseline.state_hash(), latch.state_hash());
}

#[test]
fn active_retail_crate_authority_round_trips_ghosts_paused_timers_and_latch() {
    let mut source = Simulation::new();
    source.crate_authority.pickup_any_latch = true;
    source.crate_authority.slots[0] = CrateSlot {
        start_frame: -1,
        timer_aux: 0xDEAD_BEEF,
        duration_frames: -17,
        cell_x: 12,
        cell_y: 13,
    };
    source.crate_authority.slots[255] = CrateSlot {
        start_frame: i32::MIN,
        timer_aux: u32::MAX,
        duration_frames: i32::MAX,
        cell_x: -1,
        cell_y: -2,
    };
    source
        .crate_presentation
        .push(CratePresentationEvent::CellRedraw {
            cell: (1, 2),
            frame: 3,
        });

    let bytes = bincode::serialize(&source).expect("serialize crate authority");
    let restored: Simulation = bincode::deserialize(&bytes).expect("restore crate authority");

    assert_eq!(restored.crate_authority, source.crate_authority);
    assert!(restored.crate_presentation.is_empty());
}

#[test]
fn active_retail_scheduler_runs_regeneration_before_house_tail_on_preincrement_frame() {
    use std::collections::BTreeMap;

    let mut rules = rules("");
    rules.crate_rules.regen = crate::util::native_x87::NativeF64Bits::POSITIVE_ZERO;
    let registry = registry();
    let path = PathGrid::test_all_passable(MAP, MAP);
    let mut sim = sim(991, false);
    sim.session.binary_frame = 44;
    assert!(sim.place_specific_crate(&rules, &registry, Some(&path), (9, 9), 0x14));
    let original = sim.crate_authority.slots[0].cell();

    let tick = sim.advance_tick(
        &[],
        Some(&rules),
        &BTreeMap::new(),
        Some(&path),
        Some(&registry),
        67,
    );

    assert!(tick.frame_committed);
    assert_eq!(sim.session.binary_frame, 45);
    assert_ne!(sim.crate_authority.slots[0].cell(), original);
    assert_eq!(sim.crate_authority.slots[0].start_frame, 44);
    let trace = sim.take_master_frame_test_trace();
    let regen = trace
        .iter()
        .position(|rung| *rung == crate::sim::world::MasterFrameTestRung::CrateRegeneration)
        .expect("regen rung");
    let houses = trace
        .iter()
        .position(|rung| *rung == crate::sim::world::MasterFrameTestRung::Houses)
        .expect("house rung");
    assert!(regen < houses);
}
