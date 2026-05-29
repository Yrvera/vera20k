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
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::combat_aoe::{apply_aoe_damage, bridge_adjusted_impact_z, AoELayerContext};
use crate::sim::components::WorldEffect;
use crate::sim::intern::InternedId;
use crate::sim::superweapon::cell_grid::iter_cells_3x3;
use crate::sim::world::{SimSoundEvent, Simulation};

/// Brute type_ref for Tier 1. Generalize to rules.general.animation_to_infantry[0]
/// when the full AnimClass death-to-infantry pipeline is implemented.
const BRUTE_TYPE_REF: &str = "BRUTE";

/// Mutate damage constant — large enough to kill any infantry in one hit.
/// Matches the design intent of the original engine's case-9 AoE path (exact
/// binary constant not yet extracted).
const MUTATE_AOE_DAMAGE: i32 = 9999;

/// Launch GeneticConverter at (target_rx, target_ry). Mutates infantry in area.
pub fn launch(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    target_rx: u16,
    target_ry: u16,
) -> bool {
    // 1. Spawn invoke anim (IonBlast equivalent).
    spawn_invoke_anim(sim, "IONBLAST", target_rx, target_ry);

    // 2. Collect infantry IDs + their positions BEFORE damage (for Brute spawn).
    let (killed_infantry_cells, kill_count) = if rules.general.mutate_explosion {
        apply_mutate_explosion(sim, rules, target_rx, target_ry, owner)
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
) -> (Vec<(u16, u16)>, usize) {
    let warhead_id = rules.general.mutate_explosion_warhead.clone();
    let Some(warhead) = rules.warhead(&warhead_id) else {
        log::warn!("MutateExplosionWarhead '{}' not found in rules", warhead_id);
        return (Vec::new(), 0);
    };
    let owner_str = sim.interner.resolve(owner).to_string();
    let base_damage: i32 = MUTATE_AOE_DAMAGE;
    let impact_z = bridge_adjusted_impact_z(sim.resolved_terrain.as_ref(), target_rx, target_ry);
    let hits = apply_aoe_damage(
        &sim.entities,
        target_rx,
        target_ry,
        base_damage,
        warhead,
        rules,
        &sim.interner,
        &owner_str,
        AoELayerContext {
            occupancy: Some(&sim.occupancy),
            terrain: sim.resolved_terrain.as_ref(),
            impact_z,
        },
    );

    // Emit warhead AnimList anim + smudge for the Mutate detonation,
    // kill-independent. Runs even if no infantry is in range.
    let mut explosions: Vec<crate::sim::combat::ExplosionEffect> = Vec::new();
    crate::sim::combat::emit_warhead_detonation_effects(
        warhead,
        base_damage,
        target_rx,
        target_ry,
        crate::util::lepton::CELL_CENTER_LEPTON,
        crate::util::lepton::CELL_CENTER_LEPTON,
        0,
        &mut sim.interner,
        &mut explosions,
        &mut sim.pending_smudge_requests,
    );
    for fx in &explosions {
        let frames = sim
            .effect_frame_counts
            .get(&fx.shp_name)
            .copied()
            .unwrap_or(20);
        sim.world_effects.push(WorldEffect {
            anim_spawn: None,
            shp_name: fx.shp_name,
            rx: fx.rx,
            ry: fx.ry,
            sub_x: fx.sub_x,
            sub_y: fx.sub_y,
            z: fx.z,
            frame: 0,
            total_frames: frames,
            rate_ms: 67,
            elapsed_ms: 0,
            translucent: true,
            delay_ms: 0,
            start_sound_id: None,
            start_sound_emitted: false,
        });
    }

    let mut killed: Vec<(u16, u16)> = Vec::new();
    for (id, dmg) in &hits {
        // Pre-snapshot category + position + HP BEFORE mutating.
        let snapshot = sim
            .entities
            .get(*id)
            .map(|e| (e.category, e.position.rx, e.position.ry, e.health.current));
        let Some((cat, rx, ry, hp)) = snapshot else {
            continue;
        };
        if cat != EntityCategory::Infantry {
            continue;
        }
        if let Some(e) = sim.entities.get_mut(*id) {
            let new_hp = hp.saturating_sub(*dmg);
            e.health.current = new_hp;
            if new_hp == 0 && !e.dying {
                e.dying = true;
                killed.push((rx, ry));
            }
        }
    }
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
        if let Some(e) = sim.entities.get_mut(*id) {
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
        rate_ms: 67,
        elapsed_ms: 0,
        translucent: false,
        delay_ms: 0,
        start_sound_id: None,
        start_sound_emitted: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::bridge_facts::{BridgeCellFacts, BRIDGE_FLAG_STRUCTURAL};
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::locomotor::MovementLayer;
    use crate::sim::occupancy::CellListInsertion;

    #[test]
    fn mutate_explosion_bridge_target_mutates_only_bridge_layer() {
        let rules = genetic_test_rules();
        let mut sim = Simulation::new();
        add_same_cell_bridge_infantry(&mut sim);
        let owner = sim.interner.intern("Americans");

        let (killed, count) = apply_mutate_explosion(&mut sim, &rules, 5, 5, owner);

        assert_eq!(count, 1);
        assert_eq!(killed, vec![(5, 5)]);
        assert_eq!(
            sim.entities.get(1).unwrap().health.current,
            100,
            "ground infantry under the bridge must not be mutated by a deck impact"
        );
        assert_eq!(
            sim.entities.get(2).unwrap().health.current,
            0,
            "bridge-deck infantry must be mutated by a bridge-targeted impact"
        );
        assert!(!sim.entities.get(1).unwrap().dying);
        assert!(sim.entities.get(2).unwrap().dying);
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

        sim.entities.insert(ground);
        sim.entities.insert(bridge);
        sim.occupancy.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        sim.occupancy.add(
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
            is_cliff_redraw: false,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            overlay_blocks: false,
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
