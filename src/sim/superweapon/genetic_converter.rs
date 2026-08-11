//! GeneticConverter superweapon launch handler.
//!
//! Mutates infantry in target area into Brutes. Two code paths depending on
//! Rules->MutateExplosion: either AoE via MutateExplosionWarhead, or per-cell
//! MutateWarhead applied to infantry in a 3×3 grid. On infantry death by
//! either warhead, spawns a Brute at the death position.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, sim/combat/combat_aoe, sim/superweapon/cell_grid,
//!   sim/game_entity, sim/components, sim/world.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::map::entities::EntityCategory;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::combat_aoe::{
    AoELayerContext, AreaDamageReceiver, TerrainCollectionView,
    apply_aoe_damage_with_terrain_and_scenario, bridge_adjusted_impact_z,
};
use crate::sim::components::WorldEffect;
use crate::sim::intern::InternedId;
use crate::sim::superweapon::cell_grid::iter_cells_3x3;
use crate::sim::world::{SimSoundEvent, Simulation};

/// Brute type_ref for Tier 1. Generalize to rules.general.animation_to_infantry[0]
/// when the full AnimClass death-to-infantry pipeline is implemented.
const BRUTE_TYPE_REF: &str = "BRUTE";

/// Exact signed damage loaded by SuperClass::Launch case 9 immediately before
/// its direct Apply_area_damage call (`MOV EDX, 0x2710`).
const MUTATE_AOE_DAMAGE: i32 = 10_000;

/// Launch GeneticConverter at (target_rx, target_ry). Mutates infantry in area.
pub fn launch(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    target_rx: u16,
    target_ry: u16,
    overlay_registry: Option<&OverlayTypeRegistry>,
) -> bool {
    // 1. Spawn invoke anim (IonBlast equivalent).
    spawn_invoke_anim(sim, "IONBLAST", target_rx, target_ry);

    // 2. Collect infantry IDs + their positions BEFORE damage (for Brute spawn).
    let (killed_infantry_cells, kill_count) = if rules.general.mutate_explosion {
        apply_mutate_explosion(sim, rules, target_rx, target_ry, owner, overlay_registry)
    } else {
        apply_mutate_per_cell(sim, rules, target_rx, target_ry)
    };

    // 3. Spawn a Brute at each killed-infantry cell.
    let owner_name = sim.interner.resolve(owner).to_string();
    for (rx, ry) in killed_infantry_cells {
        spawn_brute(sim, rules, &owner_name, rx, ry);
    }

    // 4. Sound event.
    sim.sound_events.push(SimSoundEvent::SuperWeaponLaunched {
        owner,
        rx: target_rx,
        ry: target_ry,
    });

    log::info!(
        "GeneticConverter launched at ({}, {}) by '{}', {} infantry mutated",
        target_rx,
        target_ry,
        sim.interner.resolve(owner),
        kill_count
    );

    true
}

/// MutateExplosion path: AoE damage via MutateExplosionWarhead.
/// Returns list of (rx, ry) cell positions of infantry killed.
fn apply_mutate_explosion(
    sim: &mut Simulation,
    rules: &RuleSet,
    target_rx: u16,
    target_ry: u16,
    owner: InternedId,
    overlay_registry: Option<&OverlayTypeRegistry>,
) -> (Vec<(u16, u16)>, usize) {
    let warhead_id = rules.general.mutate_explosion_warhead.clone();
    let Some(warhead) = rules.warhead(&warhead_id) else {
        log::warn!("MutateExplosionWarhead '{}' not found in rules", warhead_id);
        return (Vec::new(), 0);
    };
    let warhead_ref = sim.interner.intern(&warhead_id);
    let base_damage: i32 = MUTATE_AOE_DAMAGE;
    let impact_z = bridge_adjusted_impact_z(sim.resolved_terrain.as_ref(), target_rx, target_ry);
    let air_impact = crate::sim::combat::combat_aoe::air_impact_from_layer_z(
        sim.resolved_terrain.as_ref(),
        target_rx,
        target_ry,
        crate::util::lepton::CELL_CENTER_LEPTON,
        crate::util::lepton::CELL_CENTER_LEPTON,
        impact_z,
    );
    // Native SuperClass::Launch already constructed the launch-level
    // MutateExplosion animation before this direct Apply_area_damage call and
    // passes affect_resource=false. There is no Warhead AnimList producer here.
    let scenario_no_damage = sim.session.no_damage;
    let terrain_objects = TerrainCollectionView {
        objects: &sim.production.terrain_objects,
        cells: &sim.production.terrain_object_cells,
    };
    let aoe = apply_aoe_damage_with_terrain_and_scenario(
        &mut sim.substrate.entities,
        target_rx,
        target_ry,
        base_damage,
        warhead,
        rules,
        &sim.interner,
        (
            crate::sim::combat::RAD_NO_ATTACKER,
            Some(owner),
            warhead_ref,
        ),
        AoELayerContext {
            occupancy: Some(&sim.substrate.occupancy),
            terrain: sim.resolved_terrain.as_mut(),
            overlay_grid: sim.overlay_grid.as_mut(),
            overlay_registry,
            scenario_rng: Some(&mut sim.scenario_rng),
            air_impact,
            impact_z,
        },
        Some(terrain_objects),
        scenario_no_damage,
        None,
    );
    let receivers = aoe.receivers;

    // Mutation is infantry-only, but its damage still enters the ordinary
    // ReceiveDamage -> death helper transaction. Snapshot the transformation
    // cells first, preserve the AoE's object-list order, then create Brutes only
    // after every nested death detonation has returned.
    let candidates: Vec<(u64, u16, u16)> = receivers
        .iter()
        .filter_map(|receiver| {
            let AreaDamageReceiver::Entity(event) = receiver else {
                return None;
            };
            sim.substrate
                .entities
                .get(event.target_id)
                .and_then(|entity| {
                    (entity.category == EntityCategory::Infantry)
                        .then(|| (event.target_id, entity.position.rx, entity.position.ry))
                })
        })
        .collect();
    sim.commit_noncombat_aoe_receivers(rules, overlay_registry, &receivers);

    let killed: Vec<(u16, u16)> = candidates
        .into_iter()
        .filter_map(|(id, rx, ry)| {
            sim.substrate
                .entities
                .get(id)
                .is_some_and(|entity| entity.health.current == 0 && entity.dying)
                .then_some((rx, ry))
        })
        .collect();
    let count = killed.len();
    (killed, count)
}

/// Per-cell path: apply MutateWarhead to infantry in 3×3 grid.
fn apply_mutate_per_cell(
    sim: &mut Simulation,
    _rules: &RuleSet,
    target_rx: u16,
    target_ry: u16,
) -> (Vec<(u16, u16)>, usize) {
    let cells: Vec<(u16, u16)> = iter_cells_3x3(target_rx, target_ry).collect();

    // Collect infantry IDs + cell positions first (avoid borrow conflict).
    let victims: Vec<(u64, u16, u16)> = sim
        .substrate
        .entities
        .values()
        .filter(|e| e.category == EntityCategory::Infantry)
        .filter(|e| e.health.current > 0 && !e.dying)
        .filter(|e| {
            cells
                .iter()
                .any(|(rx, ry)| e.position.rx == *rx && e.position.ry == *ry)
        })
        .map(|e| (e.stable_id, e.position.rx, e.position.ry))
        .collect();

    let mut killed: Vec<(u16, u16)> = Vec::new();
    for (id, rx, ry) in &victims {
        if let Some(e) = sim.substrate.entities.get_mut(*id) {
            e.health.current = 0;
            e.dying = true;
            killed.push((*rx, *ry));
        }
    }
    let count = killed.len();
    (killed, count)
}

/// Spawn a Brute infantry at the given cell, owned by the launching player.
fn spawn_brute(sim: &mut Simulation, rules: &RuleSet, owner_name: &str, rx: u16, ry: u16) {
    let spawned = sim.spawn_object_at_height(
        BRUTE_TYPE_REF,
        owner_name,
        rx,
        ry,
        /* facing */ 0,
        /* z */ 0,
        rules,
    );
    if spawned.is_none() {
        log::warn!(
            "GeneticConverter: failed to spawn Brute for '{}' at ({},{})",
            owner_name,
            rx,
            ry
        );
    }
}

fn spawn_invoke_anim(sim: &mut Simulation, anim_name: &str, rx: u16, ry: u16) {
    let iid = sim.interner.intern(anim_name);
    let frames = sim.effect_frame_counts.get(&iid).copied().unwrap_or(20);
    sim.world_effects.push(WorldEffect {
        anim_spawn: None,
        shp_name: iid,
        rx,
        ry,
        sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
        sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
        z: 5,
        frame: 0,
        total_frames: frames,
        frame_delay: 1,
        elapsed_frames: 0,
        translucent: false,
        delay_frames: 0,
        start_sound_id: None,
        start_sound_emitted: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::bridge_facts::{BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts};
    use crate::map::overlay_types::OverlayTypeRegistry;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::locomotor::MovementLayer;
    use crate::sim::occupancy::CellListInsertion;
    use crate::sim::overlay_grid::OverlayGrid;
    use crate::sim::rng::SimRng;

    #[test]
    fn mutate_explosion_bridge_target_mutates_only_bridge_layer() {
        let rules = genetic_test_rules();
        let mut sim = Simulation::new();
        add_same_cell_bridge_infantry(&mut sim);
        let owner = sim.interner.intern("Americans");

        let (killed, count) = apply_mutate_explosion(&mut sim, &rules, 5, 5, owner, None);

        assert_eq!(count, 1);
        assert_eq!(killed, vec![(5, 5)]);
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().health.current,
            100,
            "ground infantry under the bridge must not be mutated by a deck impact"
        );
        assert_eq!(
            sim.substrate.entities.get(2).unwrap().health.current,
            0,
            "bridge-deck infantry must be mutated by a bridge-targeted impact"
        );
        assert!(!sim.substrate.entities.get(1).unwrap().dying);
        assert!(sim.substrate.entities.get(2).unwrap().dying);
    }

    #[test]
    fn gsi_04_07_damage_gsi_04_11_mutate_explosion_exact_boundary_and_death_transaction() {
        fn run(victim_hp: u16) -> (Simulation, Vec<(u16, u16)>, usize, u64) {
            let ini = IniFile::from_str(
                "[InfantryTypes]\n0=BOOMER\n1=BRUTE\n\
                 [VehicleTypes]\n0=TANK\n\
                 [AircraftTypes]\n\
                 [BuildingTypes]\n\
                 [Warheads]\n0=MutateExplosion\n1=WallWH\n\
                 [OverlayTypes]\n0=TESTWALL\n\
                 [General]\nMutateExplosion=yes\n\
                 [CombatDamage]\nMaxDamage=10000\nMutateExplosionWarhead=MutateExplosion\n\
                 [BOOMER]\nStrength=10000\nArmor=none\nSpeed=4\nExplodes=yes\nDeathWeapon=DeathBoom\n\
                 [TANK]\nStrength=10000\nArmor=heavy\nSpeed=4\nExplodes=yes\nDeathWeapon=DeathBoom\n\
                 [BRUTE]\nStrength=200\nArmor=none\nSpeed=4\n\
                 [DeathBoom]\nDamage=400\nWarhead=WallWH\n\
                 [MutateExplosion]\nCellSpread=1\nPercentAtMax=1\n\
                 Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
                 [WallWH]\nCellSpread=0\nWall=yes\n\
                 Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
                 [TESTWALL]\nWall=yes\nArmor=concrete\nStrength=400\n",
            );
            let art = IniFile::from_str("[TESTWALL]\nDamageLevels=2\n");
            let rules = RuleSet::from_ini(&ini).expect("mutation death transaction rules");
            let registry = OverlayTypeRegistry::from_ini(&ini, Some(&art));
            assert!(rules.warhead("MutateExplosion").is_some());
            assert!(rules.warhead("WallWH").is_some());
            for object_id in ["BOOMER", "TANK"] {
                assert_eq!(
                    rules.object(object_id).unwrap().death_weapon.as_deref(),
                    Some("DeathBoom")
                );
            }

            let mut sim = Simulation::with_seed(1);
            let owner = sim.interner.intern("Americans");
            let soviet = sim.interner.intern("Soviet");
            let mut infantry = GameEntity::test_default(10, "BOOMER", "Soviet", 5, 5);
            infantry.owner = soviet;
            infantry.type_ref = sim.interner.intern("BOOMER");
            infantry.category = EntityCategory::Infantry;
            infantry.is_voxel = false;
            infantry.health = Health {
                current: victim_hp,
                max: victim_hp,
            };
            sim.substrate.entities.insert(infantry);
            let _ = sim.reveal(10);

            let mut unit = GameEntity::test_default(20, "TANK", "Soviet", 6, 5);
            unit.owner = soviet;
            unit.type_ref = sim.interner.intern("TANK");
            unit.health = Health {
                current: victim_hp,
                max: victim_hp,
            };
            sim.substrate.entities.insert(unit);
            let _ = sim.reveal(20);

            let mut overlays = OverlayGrid::new(12, 12);
            overlays.place_overlay(5, 5, 0, 0);
            overlays.place_overlay(6, 5, 0, 0);
            sim.overlay_grid = Some(overlays);

            let (killed, count) =
                apply_mutate_explosion(&mut sim, &rules, 5, 5, owner, Some(&registry));
            let rng_state = sim.scenario_rng.state();
            (sim, killed, count, rng_state)
        }

        let (fatal, killed, count, fatal_rng) = run(MUTATE_AOE_DAMAGE as u16);
        for id in [10, 20] {
            assert!(fatal.substrate.entities.get(id).is_some_and(|entity| {
                entity.health.current == 0 && entity.dying && !entity.in_logic_vector
            }));
            assert!(!fatal.live_object_order_snapshot().contains(&id));
        }
        assert_eq!((killed, count), (vec![(5, 5)], 1));
        for cell in [(5, 5), (6, 5)] {
            assert_eq!(
                fatal
                    .overlay_grid
                    .as_ref()
                    .unwrap()
                    .cell(cell.0, cell.1)
                    .overlay_id,
                None
            );
        }
        assert_eq!(
            fatal.substrate.pending_delete,
            vec![20, 10],
            "concrete Unit UnInit is inline; the synthetic non-animated Infantry fixture drains later"
        );
        assert_eq!(fatal_rng, SimRng::new(1).state());

        let (boundary, killed, count, boundary_rng) = run(MUTATE_AOE_DAMAGE as u16 + 1);
        assert_eq!((killed, count), (Vec::new(), 0));
        for (id, cell) in [(10, (5, 5)), (20, (6, 5))] {
            assert_eq!(
                boundary
                    .overlay_grid
                    .as_ref()
                    .unwrap()
                    .cell(cell.0, cell.1)
                    .overlay_id,
                Some(0)
            );
            assert!(boundary.live_object_order_snapshot().contains(&id));
            assert_eq!(
                boundary.substrate.entities.get(id).unwrap().health.current,
                1
            );
        }
        assert!(boundary.substrate.pending_delete.is_empty());
        assert_eq!(boundary_rng, SimRng::new(1).state());
    }

    fn genetic_test_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[InfantryTypes]\n0=E1\n1=BRUTE\n\n\
             [VehicleTypes]\n\n\
             [AircraftTypes]\n\n\
             [BuildingTypes]\n\n\
             [General]\nMutateExplosion=yes\n\n\
             [CombatDamage]\nMutateExplosionWarhead=MutateExplosion\n\n\
             [E1]\nStrength=100\nArmor=none\nSpeed=4\nPrimary=DUMMYW\n\n\
             [BRUTE]\nStrength=200\nArmor=none\nSpeed=4\n\n\
             [DUMMYW]\nDamage=1\nROF=1\nRange=1\nWarhead=MutateExplosion\n\n\
             [MutateExplosion]\nCellSpread=1\nPercentAtMax=1\n\
             Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        ))
        .expect("genetic test rules should parse")
    }

    fn add_same_cell_bridge_infantry(sim: &mut Simulation) {
        let owner = sim.interner.intern("Soviet");
        let type_ref = sim.interner.intern("E1");

        let mut ground = GameEntity::test_default(1, "E1", "Soviet", 5, 5);
        ground.owner = owner;
        ground.type_ref = type_ref;
        ground.category = EntityCategory::Infantry;
        ground.is_voxel = false;
        ground.health = Health {
            current: 100,
            max: 100,
        };

        let mut bridge = GameEntity::test_default(2, "E1", "Soviet", 5, 5);
        bridge.owner = owner;
        bridge.type_ref = type_ref;
        bridge.category = EntityCategory::Infantry;
        bridge.is_voxel = false;
        bridge.health = Health {
            current: 100,
            max: 100,
        };
        bridge.on_bridge = true;
        bridge.position.z = 4;

        sim.substrate.entities.insert(ground);
        sim.substrate.entities.insert(bridge);
        sim.substrate.occupancy.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        sim.substrate.occupancy.add(
            5,
            5,
            2,
            MovementLayer::Bridge,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        sim.resolved_terrain = Some(bridge_terrain());
    }

    fn bridge_terrain() -> ResolvedTerrainGrid {
        let mut cells = Vec::new();
        for ry in 0..10 {
            for rx in 0..10 {
                cells.push(test_terrain_cell(rx, ry));
            }
        }
        let idx = 5 * 10 + 5;
        cells[idx].bridge_facts = BridgeCellFacts {
            raw_flags: BRIDGE_FLAG_STRUCTURAL,
            ..BridgeCellFacts::default()
        };
        cells[idx].has_bridge_deck = true;
        cells[idx].bridge_walkable = true;
        cells[idx].bridge_deck_level = 4;
        ResolvedTerrainGrid::from_cells(10, 10, cells)
    }

    fn test_terrain_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
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
            speed_costs: SpeedCostProfile::default(),
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
            base_speed_costs: Default::default(),
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
}
