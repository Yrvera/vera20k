//! Lightning Storm state machine — bolt generation and area damage.
//!
//! Only one storm can be active globally at a time. The storm has a deferment
//! countdown before bolts begin, then generates center + scatter bolts each
//! tick for the configured duration.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, sim/components, sim/combat.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::ruleset::RuleSet;
use crate::sim::combat::combat_aoe::{AoELayerContext, apply_aoe_damage, bridge_adjusted_impact_z};
use crate::sim::components::WorldEffect;
use crate::sim::intern::InternedId;
use crate::sim::world::{SimSoundEvent, Simulation};

/// Lightning storm bolt animation names (WeatherConBolts from art.ini).
const BOLT_ANIMS: &[&str] = &["WCLBOLT1", "WCLBOLT2", "WCLBOLT3"];

/// Maximum retry attempts for scatter bolt placement (avoid infinite loop).
const MAX_SCATTER_RETRIES: u32 = 10;

/// Queued lightning storm request — activated when the current storm ends.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueuedLightningStorm {
    pub owner: InternedId,
    pub target_rx: u16,
    pub target_ry: u16,
}

/// Active lightning storm state.
///
/// Global — only one storm at a time (per original engine).
/// Stored as `Simulation.lightning_storm: Option<LightningStormState>`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LightningStormState {
    /// House that launched the storm.
    pub owner: InternedId,
    /// Storm center cell X.
    pub target_rx: u16,
    /// Storm center cell Y.
    pub target_ry: u16,
    /// Ticks remaining before bolts begin (deferment countdown).
    pub deferment_remaining: i32,
    /// Ticks remaining for active bolt generation.
    pub duration_remaining: i32,
    /// Ticks until next center bolt.
    pub center_bolt_timer: i32,
    /// Ticks until next scatter bolt.
    pub scatter_bolt_timer: i32,
    /// Last bolt cell X (for separation enforcement).
    pub last_bolt_rx: u16,
    /// Last bolt cell Y (for separation enforcement).
    pub last_bolt_ry: u16,
}

/// Start a new lightning storm. If one is already active, queues the request
/// so it activates when the current storm ends.
pub fn start(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    target_rx: u16,
    target_ry: u16,
) -> bool {
    if sim.lightning_storm.is_some() {
        log::info!("Lightning Storm queued — one already active, will start when current ends");
        sim.queued_lightning_storm = Some(QueuedLightningStorm {
            owner,
            target_rx,
            target_ry,
        });
        return true;
    }

    let state = LightningStormState {
        owner,
        target_rx,
        target_ry,
        deferment_remaining: rules.general.lightning_deferment,
        duration_remaining: rules.general.lightning_storm_duration,
        center_bolt_timer: rules.general.lightning_hit_delay,
        scatter_bolt_timer: rules.general.lightning_scatter_delay,
        last_bolt_rx: target_rx,
        last_bolt_ry: target_ry,
    };

    sim.lightning_storm = Some(state);

    // Sound event for EVA warning.
    sim.sound_events.push(SimSoundEvent::SuperWeaponLaunched {
        owner,
        rx: target_rx,
        ry: target_ry,
    });

    log::info!(
        "Lightning Storm started at ({}, {}) by '{}', deferment={} duration={}",
        target_rx,
        target_ry,
        sim.interner.resolve(owner),
        rules.general.lightning_deferment,
        rules.general.lightning_storm_duration,
    );

    true
}

/// Process the active lightning storm for one tick.
/// Called from `tick_superweapons()` each tick.
pub fn process(sim: &mut Simulation, rules: &RuleSet) {
    let Some(ref mut storm) = sim.lightning_storm else {
        return;
    };

    // Phase 1: deferment countdown.
    if storm.deferment_remaining > 0 {
        storm.deferment_remaining -= 1;
        return;
    }

    // Phase 2: active storm — decrement duration.
    storm.duration_remaining -= 1;
    if storm.duration_remaining <= 0 {
        log::info!("Lightning Storm ended");
        sim.lightning_storm = None;
        // Activate queued storm if one is waiting.
        if let Some(queued) = sim.queued_lightning_storm.take() {
            log::info!("Activating queued Lightning Storm");
            start(sim, rules, queued.owner, queued.target_rx, queued.target_ry);
        }
        return;
    }

    // Extract storm fields for bolt generation (avoid borrow conflict).
    let target_rx = storm.target_rx;
    let target_ry = storm.target_ry;
    let last_rx = storm.last_bolt_rx;
    let last_ry = storm.last_bolt_ry;
    let owner = storm.owner;

    // Center bolt
    storm.center_bolt_timer -= 1;
    let spawn_center = storm.center_bolt_timer <= 0;
    if spawn_center {
        storm.center_bolt_timer = rules.general.lightning_hit_delay;
    }

    // Scatter bolt
    storm.scatter_bolt_timer -= 1;
    let spawn_scatter = storm.scatter_bolt_timer <= 0;
    if spawn_scatter {
        storm.scatter_bolt_timer = rules.general.lightning_scatter_delay;
    }

    let spread = rules.general.lightning_cell_spread;
    let separation = rules.general.lightning_separation;

    if spawn_center {
        spawn_bolt(sim, rules, target_rx, target_ry, owner);
    }

    if spawn_scatter {
        let (rx, ry) = pick_scatter_cell(
            sim, target_rx, target_ry, last_rx, last_ry, spread, separation,
        );
        spawn_bolt(sim, rules, rx, ry, owner);
        // Update last bolt position on the storm state.
        if let Some(ref mut storm) = sim.lightning_storm {
            storm.last_bolt_rx = rx;
            storm.last_bolt_ry = ry;
        }
    }
}

/// Pick a random cell within `spread` of the storm center, enforcing
/// `separation` manhattan distance from the last bolt.
fn pick_scatter_cell(
    sim: &mut Simulation,
    center_rx: u16,
    center_ry: u16,
    last_rx: u16,
    last_ry: u16,
    spread: i32,
    separation: i32,
) -> (u16, u16) {
    let diameter = (spread * 2 + 1) as u32;
    for _ in 0..MAX_SCATTER_RETRIES {
        // Random offset within [-spread, +spread] for both axes.
        let dx = sim.superweapon_rng().next_range_u32(diameter) as i32 - spread;
        let dy = sim.superweapon_rng().next_range_u32(diameter) as i32 - spread;
        let rx = (center_rx as i32 + dx).max(0) as u16;
        let ry = (center_ry as i32 + dy).max(0) as u16;

        // Check manhattan distance from last bolt.
        let manhattan = (rx as i32 - last_rx as i32).abs() + (ry as i32 - last_ry as i32).abs();
        if manhattan >= separation {
            return (rx, ry);
        }
    }
    // Fallback: use the last attempted position (avoids infinite loop).
    let dx = sim.superweapon_rng().next_range_u32(diameter) as i32 - spread;
    let dy = sim.superweapon_rng().next_range_u32(diameter) as i32 - spread;
    (
        (center_rx as i32 + dx).max(0) as u16,
        (center_ry as i32 + dy).max(0) as u16,
    )
}

/// Spawn a single lightning bolt at the given cell: visual effect + area damage.
fn spawn_bolt(sim: &mut Simulation, rules: &RuleSet, rx: u16, ry: u16, owner: InternedId) {
    // 1. Pick a random bolt animation.
    let anim_idx = sim
        .superweapon_rng()
        .next_range_u32(BOLT_ANIMS.len() as u32) as usize;
    let anim_name = BOLT_ANIMS[anim_idx];
    let anim_iid = sim.interner.intern(anim_name);
    let frames = sim
        .effect_frame_counts
        .get(&anim_iid)
        .copied()
        .unwrap_or(20);

    sim.world_effects.push(WorldEffect {
        anim_spawn: None,
        shp_name: anim_iid,
        rx,
        ry,
        sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
        sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
        z: 0,
        frame: 0,
        total_frames: frames,
        rate_ms: 67, // ~15 fps
        elapsed_ms: 0,
        translucent: true,
        delay_ms: 0,
        start_sound_id: None,
        start_sound_emitted: false,
    });

    // 2. Apply area damage via lightning warhead.
    let warhead_id = &rules.general.lightning_warhead;
    if let Some(warhead) = rules.warhead(warhead_id) {
        let owner_str = sim.interner.resolve(owner).to_string();
        let impact_z = bridge_adjusted_impact_z(sim.resolved_terrain.as_ref(), rx, ry);
        let hits = apply_aoe_damage(
            &sim.entities,
            rx,
            ry,
            rules.general.lightning_damage,
            warhead,
            rules,
            &sim.interner,
            &owner_str,
            AoELayerContext {
                occupancy: Some(&sim.substrate.occupancy),
                terrain: sim.resolved_terrain.as_ref(),
                impact_z,
            },
        );

        // Apply damage to entities.
        for (stable_id, damage) in hits {
            if let Some(entity) = sim.entities.get_mut(stable_id) {
                entity.health.current = entity.health.current.saturating_sub(damage);
                entity.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
            }
        }

        // Emit warhead AnimList anim + smudge for this bolt detonation,
        // kill-independent. The bolt visual above is the strike sprite;
        // this is the warhead's AnimList anim (e.g. EXPLOSION).
        let mut explosions: Vec<crate::sim::combat::ExplosionEffect> = Vec::new();
        crate::sim::combat::emit_warhead_detonation_effects(
            warhead,
            rules.general.lightning_damage,
            rx,
            ry,
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
    } else {
        log::warn!("Lightning warhead '{}' not found in rules", warhead_id);
    }

    // 3. Sound event for the bolt strike.
    sim.sound_events
        .push(SimSoundEvent::SuperWeaponStrike { rx, ry });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::bridge_facts::{BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts};
    use crate::map::entities::EntityCategory;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::locomotor::MovementLayer;
    use crate::sim::occupancy::CellListInsertion;
    use crate::sim::world::Simulation;

    fn lightning_test_setup() -> (Simulation, RuleSet) {
        // RuleSet only registers warheads referenced by a weapon, so a dummy
        // unit + weapon are needed to anchor LWH in the warhead table.
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[InfantryTypes]\n0=DUMMY\n\n\
             [VehicleTypes]\n\n\
             [AircraftTypes]\n\n\
             [BuildingTypes]\n0=GAPOWR\n\n\
             [DUMMY]\nStrength=100\nArmor=none\nSpeed=4\nPrimary=DUMMYW\n\n\
             [GAPOWR]\nStrength=200\nArmor=wood\n\n\
             [DUMMYW]\nDamage=1\nROF=1\nRange=1\nWarhead=LWH\n\n\
             [General]\nLightningDamage=100\nLightningWarhead=LWH\n\n\
             [LWH]\nCellSpread=1\nPercentAtMax=1\nAnimList=EXPLOSION\n\
             Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        ))
        .expect("lightning test rules should parse");
        let sim = Simulation::with_seed(1);
        (sim, rules)
    }

    #[test]
    fn lightning_strike_emits_anim_smudge_into_pending_requests() {
        let (mut sim, rules) = lightning_test_setup();
        let owner = sim.interner.intern("Americans");

        spawn_bolt(&mut sim, &rules, 5, 5, owner);

        let anim_count = sim
            .pending_smudge_requests
            .iter()
            .filter(|r| matches!(r, crate::sim::combat::SmudgeSpawnRequest::Anim { .. }))
            .count();
        assert_eq!(
            anim_count, 1,
            "one bolt → one Anim smudge request in pending_smudge_requests"
        );

        let explosion_iid = sim.interner.intern("EXPLOSION");
        assert!(
            sim.world_effects
                .iter()
                .any(|fx| fx.shp_name == explosion_iid && fx.rx == 5 && fx.ry == 5),
            "lightning warhead AnimList anim must be pushed to world_effects"
        );
    }

    #[test]
    fn lightning_bridge_strike_damages_only_bridge_layer() {
        let (mut sim, rules) = lightning_test_setup();
        add_same_cell_bridge_targets(&mut sim, "DUMMY");
        let owner = sim.interner.intern("Americans");

        spawn_bolt(&mut sim, &rules, 5, 5, owner);

        assert_eq!(
            sim.entities.get(1).unwrap().health.current,
            100,
            "ground occupant under the bridge must not be hit by a deck strike"
        );
        assert_eq!(
            sim.entities.get(2).unwrap().health.current,
            0,
            "bridge-deck occupant must be hit by a bridge-targeted Lightning strike"
        );
    }

    #[test]
    fn lightning_storm_crossing_condition_yellow_sets_building_damage_state() {
        let (mut sim, rules) = lightning_test_setup();
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("GAPOWR");
        let mut building = GameEntity::test_default(10, "GAPOWR", "Soviet", 5, 5);
        building.category = EntityCategory::Structure;
        building.owner = sim.interner.intern("Soviet");
        building.type_ref = type_ref;
        building.health = Health {
            current: 150,
            max: 200,
        };
        sim.entities.insert(building);

        spawn_bolt(&mut sim, &rules, 5, 5, owner);

        let building = sim.entities.get(10).expect("building remains in sim");
        assert_eq!(building.health.current, 50);
        assert!(building.building_damage_state_active);
    }

    fn add_same_cell_bridge_targets(sim: &mut Simulation, type_name: &str) {
        let owner = sim.interner.intern("Soviet");
        let type_ref = sim.interner.intern(type_name);

        let mut ground = GameEntity::test_default(1, type_name, "Soviet", 5, 5);
        ground.owner = owner;
        ground.type_ref = type_ref;
        ground.health = Health {
            current: 100,
            max: 100,
        };

        let mut bridge = GameEntity::test_default(2, type_name, "Soviet", 5, 5);
        bridge.owner = owner;
        bridge.type_ref = type_ref;
        bridge.health = Health {
            current: 100,
            max: 100,
        };
        bridge.on_bridge = true;
        bridge.position.z = 4;

        sim.entities.insert(ground);
        sim.entities.insert(bridge);
        sim.substrate.occupancy.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        sim.substrate.occupancy.add(
            5,
            5,
            2,
            MovementLayer::Bridge,
            None,
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
