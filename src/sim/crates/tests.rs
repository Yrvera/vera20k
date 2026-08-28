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
             [WATER]\nCrate=yes\n[MODCRATE]\nCrate=yes\nCrateTrigger=yes\n",
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
        native_row_present: true,
        native_speed_bits: [crate::util::native_x87::NativeF32Bits::ONE; 8],
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

fn pickup_rules() -> crate::rules::ruleset::RuleSet {
    crate::rules::ruleset::RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n[VehicleTypes]\n0=PICKER\n1=GOODIE\n[AircraftTypes]\n[BuildingTypes]\n\
         [PICKER]\nStrength=100\nSpeed=5\nTrainable=yes\nPrimary=GUN\n\
         [GOODIE]\nStrength=100\nSpeed=5\nCrateGoodie=yes\n\
         [GUN]\nDamage=10\nROF=15\nRange=5\nProjectile=PROJ\nWarhead=WH\n\
         [PROJ]\nInviso=yes\nAG=yes\n[WH]\nVerses=100%\n\
         [CrateRules]\nCrateImg=SILVER\nWoodCrateImg=WOOD\nWaterCrateImg=WATER\nCrateRegen=3\n",
    ))
    .expect("pickup rules")
}

fn add_house_and_picker(sim: &mut Simulation, rules: &crate::rules::ruleset::RuleSet) -> u64 {
    let owner = sim.interner.intern("AMERICANS");
    sim.session.house_order.push(owner);
    sim.houses.insert(
        owner,
        crate::sim::house_state::HouseState::new(owner, 0, Some(owner), true, 1000, 10),
    );
    sim.spawn_object_at_height("PICKER", "AMERICANS", 7, 7, 0, 0, rules)
        .expect("picker spawn")
}

fn install_pickup_crate(
    sim: &mut Simulation,
    overlays: &OverlayTypeRegistry,
    overlay_name: &str,
    cell: (u16, u16),
    data: u8,
) {
    let id = overlays.id_for_name(overlay_name).expect("crate identity");
    assert!(
        sim.overlay_grid
            .as_mut()
            .unwrap()
            .place_crate_overlay_bytes(cell.0, cell.1, id)
    );
    assert!(
        sim.overlay_grid
            .as_mut()
            .unwrap()
            .write_crate_data_no_dirty(cell.0, cell.1, data)
    );
    sim.crate_authority.slots[0].cell_x = cell.0 as i16;
    sim.crate_authority.slots[0].cell_y = cell.1 as i16;
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

#[test]
fn active_retail_fixed_money_skips_selection_draw_then_removes_before_amount_draw() {
    let mut rules = pickup_rules();
    rules.crate_rules.powerups[crate::rules::crate_rules::CrateEffect::Money as usize].data =
        crate::util::native_x87::NativeF64Bits::from_bits(2000.0_f64.to_bits());
    let overlays = registry();
    let mut sim = sim(0x9137, false);
    sim.session.game_options.crates = false;
    let picker = add_house_and_picker(&mut sim, &rules);
    install_pickup_crate(&mut sim, &overlays, "WOOD", (7, 7), 0);

    let mut expected = sim.scenario_rng.clone();
    let amount = expected.next_range_u32_inclusive(2000, 2900) as i32;
    let result = sim.pickup_crate_at(
        (7, 7),
        picker,
        CratePickupInputs {
            rules: &rules,
            overlays: &overlays,
            path_grid: None,
            event_49: None,
        },
    );

    assert_eq!(result, NativePickupReturn::One);
    assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
    let owner = sim.entities().get(picker).unwrap().owner;
    assert_eq!(sim.houses[&owner].credits, 1000_i32.wrapping_add(amount));
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(7, 7).overlay_id,
        None
    );
    assert!(!sim.crate_authority.slots[0].is_occupied());
}

struct KillOnEvent49;

impl super::pickup::CrateEvent49Dispatch for KillOnEvent49 {
    fn raise(
        &mut self,
        sim: &mut Simulation,
        collector_id: u64,
        _tag_id: crate::sim::intern::InternedId,
    ) {
        sim.uninit(collector_id);
    }
}

struct MoveAndUnlimboSqdOnEvent49 {
    callback_destination: crate::sim::components::DriveCoord,
}

impl super::pickup::CrateEvent49Dispatch for MoveAndUnlimboSqdOnEvent49 {
    fn raise(
        &mut self,
        sim: &mut Simulation,
        collector_id: u64,
        _tag_id: crate::sim::intern::InternedId,
    ) {
        let collector = sim.entities_mut().get_mut(collector_id).expect("SQD tombstone");
        collector.lifecycle.in_limbo = false;
        collector.position.rx = 20;
        collector.position.ry = 21;
        collector.position.sub_x = crate::util::fixed_math::SimFixed::from_num(11);
        collector.position.sub_y = crate::util::fixed_math::SimFixed::from_num(13);
        collector.ship_locomotion.as_mut().unwrap().destination = Some(self.callback_destination);
    }
}

fn configure_sqd_attach_pair(
    sim: &mut Simulation,
    rules: &crate::rules::ruleset::RuleSet,
) -> (u64, u64) {
    let attacker = add_house_and_picker(sim, rules);
    let victim = sim
        .spawn_object_at_height("PICKER", "AMERICANS", 9, 10, 0, 0, rules)
        .expect("SQD victim");
    {
        let attacker = sim.entities_mut().get_mut(attacker).unwrap();
        attacker.lifecycle.in_limbo = true;
        attacker.parasite_manager = Some(
            crate::sim::parasite_attachment::ParasiteManagerState::default(),
        );
        attacker.ship_locomotion = Some(Default::default());
    }
    (attacker, victim)
}

#[test]
fn sqd_attach_runs_real_trigger_and_money_effect_before_reciprocal_continuation() {
    let mut rules = pickup_rules();
    rules.crate_rules.powerups[crate::rules::crate_rules::CrateEffect::Money as usize].data =
        crate::util::native_x87::NativeF64Bits::from_bits(2000.0_f64.to_bits());
    let overlays = registry();
    let mut sim = sim(0x5344, false);
    sim.session.game_options.crates = false;
    let (attacker_id, victim_id) = configure_sqd_attach_pair(&mut sim, &rules);
    let owner = sim.entities().get(attacker_id).unwrap().owner;
    let before_credits = sim.houses[&owner].credits;
    let tag = sim.interner.intern("TAG_SQD_CRATE");
    sim.entities_mut().get_mut(attacker_id).unwrap().attached_tag_id = Some(tag);
    install_pickup_crate(&mut sim, &overlays, "MODCRATE", (9, 10), 0);
    let callback_destination = crate::sim::components::DriveCoord::cell(40, 41, 7);
    let mut callback = MoveAndUnlimboSqdOnEvent49 {
        callback_destination,
    };

    let result = sim
        .pickup_crate_from_sqd_attach(
            attacker_id,
            victim_id,
            CratePickupInputs {
                rules: &rules,
                overlays: &overlays,
                path_grid: None,
                event_49: Some(&mut callback),
            },
        )
        .expect("accepted SQD Attach seam");

    assert_eq!(result, NativePickupReturn::One);
    assert!(sim.houses[&owner].credits > before_credits, "real Money effect ran");
    let attacker = sim.entities().get(attacker_id).unwrap();
    let ship = attacker.ship_locomotion.as_ref().unwrap();
    let victim_coord = crate::sim::movement::crate_callers::MovementCrateProbe::current_coord(
        sim.entities().get(victim_id).unwrap(),
    );
    assert_eq!(
        (attacker.position.rx, attacker.position.ry),
        (9, 10),
        "One/unlimbo ForceTrack raw-applies the immutable victim request"
    );
    assert_eq!(ship.destination, Some(callback_destination), "callback retarget survives");
    assert_eq!(ship.head_to, Some(victim_coord));
    assert_eq!(ship.target_speed_fraction, crate::util::native_x87::NativeF64Bits::ONE);
    assert_eq!(sim.entities().get(victim_id).unwrap().parasite_attacker_id, Some(attacker_id));
    assert_eq!(attacker.parasite_manager.as_ref().unwrap().victim_id, Some(victim_id));
}

#[test]
fn sqd_attach_event49_death_keeps_crate_and_still_installs_both_links() {
    let rules = pickup_rules();
    let overlays = registry();
    let mut sim = sim(0x5345, false);
    sim.session.game_options.crates = false;
    let (attacker_id, victim_id) = configure_sqd_attach_pair(&mut sim, &rules);
    let tag = sim.interner.intern("TAG_SQD_KILL");
    sim.entities_mut().get_mut(attacker_id).unwrap().attached_tag_id = Some(tag);
    install_pickup_crate(&mut sim, &overlays, "MODCRATE", (9, 10), 0);
    let mut kill = KillOnEvent49;

    let result = sim
        .pickup_crate_from_sqd_attach(
            attacker_id,
            victim_id,
            CratePickupInputs {
                rules: &rules,
                overlays: &overlays,
                path_grid: None,
                event_49: Some(&mut kill),
            },
        )
        .expect("accepted SQD Attach seam");

    assert_eq!(result, NativePickupReturn::Zero);
    let attacker = sim.entities().get(attacker_id).expect("deferred tombstone");
    assert!(!attacker.lifecycle.object_alive);
    assert_eq!(attacker.parasite_manager.as_ref().unwrap().victim_id, Some(victim_id));
    assert_eq!(sim.entities().get(victim_id).unwrap().parasite_attacker_id, Some(attacker_id));
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(9, 10).overlay_id,
        overlays.id_for_name("MODCRATE"),
        "Event-49 death returns before crate removal"
    );
}

#[test]
fn active_retail_event49_death_returns_zero_before_latch_rng_or_removal() {
    let rules = pickup_rules();
    let overlays = registry();
    let mut sim = sim(0x91, false);
    sim.session.game_options.crates = false;
    let picker = add_house_and_picker(&mut sim, &rules);
    let tag = sim.interner.intern("TAG_CRATE");
    sim.entities_mut().get_mut(picker).unwrap().attached_tag_id = Some(tag);
    install_pickup_crate(&mut sim, &overlays, "MODCRATE", (7, 7), 0);
    let before_rng = sim.scenario_rng.logical_state();
    let before_slot = sim.crate_authority.slots[0];
    let mut kill = KillOnEvent49;

    let result = sim.pickup_crate_at(
        (7, 7),
        picker,
        CratePickupInputs {
            rules: &rules,
            overlays: &overlays,
            path_grid: None,
            event_49: Some(&mut kill),
        },
    );

    assert_eq!(result, NativePickupReturn::Zero);
    assert!(!sim.crate_authority.pickup_any_latch);
    assert_eq!(sim.crate_authority.slots[0], before_slot);
    assert_eq!(sim.scenario_rng.logical_state(), before_rng);
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(7, 7).overlay_id,
        overlays.id_for_name("MODCRATE")
    );
}

#[test]
fn active_retail_trigger_crate_without_tag_sets_latch_and_continues_pickup() {
    let mut rules = pickup_rules();
    rules.crate_rules.powerups[crate::rules::crate_rules::CrateEffect::Money as usize].data =
        crate::util::native_x87::NativeF64Bits::from_bits(2000.0_f64.to_bits());
    let overlays = registry();
    let mut sim = sim(0x72, false);
    sim.session.game_options.crates = false;
    let picker = add_house_and_picker(&mut sim, &rules);
    install_pickup_crate(&mut sim, &overlays, "MODCRATE", (7, 7), 0);

    assert_eq!(
        sim.pickup_crate_at(
            (7, 7),
            picker,
            CratePickupInputs {
                rules: &rules,
                overlays: &overlays,
                path_grid: None,
                event_49: None,
            },
        ),
        NativePickupReturn::One
    );
    assert!(sim.crate_authority.pickup_any_latch);
    assert!(!sim.crate_authority.slots[0].is_occupied());
}

#[test]
fn active_retail_armor_effect_uses_strict_radius_and_raw_multiplier_bits_without_owner_filter() {
    let mut rules = pickup_rules();
    rules.crate_rules.radius_leptons = 768;
    rules.crate_rules.powerups[crate::rules::crate_rules::CrateEffect::Armor as usize].data =
        crate::util::native_x87::NativeF64Bits::from_bits(1.5_f64.to_bits());
    let overlays = registry();
    let mut sim = sim(0x77, false);
    sim.session.game_options.crates = false;
    let picker = add_house_and_picker(&mut sim, &rules);
    let enemy = sim.interner.intern("ENEMY");
    sim.session.house_order.push(enemy);
    sim.houses.insert(
        enemy,
        crate::sim::house_state::HouseState::new(enemy, 1, Some(enemy), false, 1000, 10),
    );
    let near = sim
        .spawn_object_at_height("PICKER", "ENEMY", 8, 7, 0, 0, &rules)
        .expect("near enemy");
    let boundary = sim
        .spawn_object_at_height("PICKER", "ENEMY", 10, 7, 0, 0, &rules)
        .expect("boundary enemy");
    for stable_id in [picker, near, boundary] {
        let entity = sim.entities_mut().get_mut(stable_id).unwrap();
        entity.position.sub_x = crate::util::fixed_math::SimFixed::from_num(
            crate::util::lepton::CELL_CENTER_LEPTON,
        );
        entity.position.sub_y = crate::util::fixed_math::SimFixed::from_num(
            crate::util::lepton::CELL_CENTER_LEPTON,
        );
    }
    install_pickup_crate(&mut sim, &overlays, "WOOD", (7, 7), 9);

    assert_eq!(
        sim.pickup_crate_at(
            (7, 7),
            picker,
            CratePickupInputs {
                rules: &rules,
                overlays: &overlays,
                path_grid: None,
                event_49: None,
            },
        ),
        NativePickupReturn::One
    );
    assert_eq!(
        sim.entities().get(picker).unwrap().armor_multiplier.bits(),
        1.5_f64.to_bits()
    );
    assert_eq!(
        sim.entities().get(near).unwrap().armor_multiplier.bits(),
        1.5_f64.to_bits(),
        "enemy inside the radius is modified"
    );
    assert_eq!(
        sim.entities().get(boundary).unwrap().armor_multiplier.bits(),
        1.0_f64.to_bits(),
        "three-cell/768-lepton boundary is strict"
    );
}
