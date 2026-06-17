//! Teleport (chrono) locomotor — instant relocation with chrono delay.
//!
//! Implements the Teleport state machine for chrono-style movement:
//! Relocate (instant, one tick) → ChronoDelay (being_warped countdown) → Idle.
//!
//! Self-teleport relocates the unit in a single tick (Phase 0), then the unit
//! sits at the destination 50% translucent for `chrono_delay` ticks until fully
//! materialized.
//!
//! Units with `Locomotor=Teleport` always use this. Units with `Teleporter=yes`
//! but a different base locomotor (e.g., Chrono Miner with Drive) get a temporary
//! override via the piggyback mechanism, restoring their base locomotor after arrival.
//!
//! No pathfinding — the unit is relocated instantly. Occupancy is cleared at the
//! old position and marked at the new position during the Relocate phase.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/game_entity, sim/entity_store, sim/locomotor.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use crate::rules::locomotor_type::LocomotorKind;
use crate::rules::ruleset::GeneralRules;
use crate::sim::components::{AnimClassSpawnDescriptor, WorldEffect};
use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::InternedId;
use crate::sim::movement::locomotor::OverrideKind;
use crate::sim::occupancy::{CellListInsertion, OccupancyGrid};
use crate::util::fixed_math::isqrt_i64;
use crate::util::lepton::CELL_CENTER_LEPTON;

const TELEPORT_WARP_DRAW_FLAGS: u32 = 0x600;
const TELEPORT_WARP_DELAY: u16 = 0;
const TELEPORT_WARP_LOOP_COUNT: u8 = 1;
const TELEPORT_WARP_Z_ADJUST: i32 = 0;
const TELEPORT_WARP_REVERSE: bool = false;
const FALLBACK_WARP_FRAME_COUNT: u16 = 20;

/// World-effect bridge for verified teleport `AnimClass` constructor rows.
pub struct TeleportVisuals<'a> {
    pub world_effects: &'a mut Vec<WorldEffect>,
    pub effect_frame_counts: &'a BTreeMap<InternedId, u16>,
    pub warp_out_type: InternedId,
    pub warp_out_rate_ms: u32,
}

impl TeleportVisuals<'_> {
    fn spawn_warp_out(&mut self, rx: u16, ry: u16, z: u8) {
        let total_frames = self
            .effect_frame_counts
            .get(&self.warp_out_type)
            .copied()
            .unwrap_or(FALLBACK_WARP_FRAME_COUNT);
        let mut anim_spawn = AnimClassSpawnDescriptor::new(
            self.warp_out_type,
            rx,
            ry,
            CELL_CENTER_LEPTON,
            CELL_CENTER_LEPTON,
            z,
        );
        anim_spawn.delay = TELEPORT_WARP_DELAY;
        anim_spawn.loop_count = TELEPORT_WARP_LOOP_COUNT;
        anim_spawn.draw_flags = TELEPORT_WARP_DRAW_FLAGS;
        anim_spawn.z_adjust = TELEPORT_WARP_Z_ADJUST;
        anim_spawn.reverse = TELEPORT_WARP_REVERSE;

        self.world_effects.push(WorldEffect::from_anim_spawn(
            anim_spawn,
            total_frames,
            self.warp_out_rate_ms,
            true,
            None,
        ));
    }
}

/// Phase within the teleport state machine.
///
/// Phase 0 relocates instantly in one tick, then the chrono delay timer
/// counts down while the unit is semi-transparent at the destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TeleportPhase {
    /// Instant relocation: position updated, occupancy swapped. Executes in
    /// one tick, then transitions to ChronoDelay.
    Relocate,
    /// Post-warp chrono delay: unit sits at destination 50% translucent,
    /// `being_warped_ticks` counts down each tick. When it reaches 0 the
    /// teleport is complete and the base locomotor is restored.
    ChronoDelay,
}

/// State for an in-progress teleport.
///
/// Set by `issue_teleport_command()` and cleared when the chrono delay
/// expires. The render system reads `being_warped_ticks` to apply 50%
/// translucency while the unit materializes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TeleportState {
    /// Current phase in the teleport sequence.
    pub phase: TeleportPhase,
    /// Destination cell coordinates.
    pub target_rx: u16,
    pub target_ry: u16,
    /// Chrono delay countdown in game ticks. While > 0 the unit is "being warped"
    /// and the renderer draws it at 50% alpha. Set from the distance-based formula
    /// in the original engine: `delay = distance_leptons / ChronoDistanceFactor`,
    /// clamped to `ChronoMinimumDelay`.
    pub being_warped_ticks: u32,
}

/// Compute the chrono warp delay in game ticks from distance.
///
/// When `ChronoTrigger=yes`, delay scales linearly with distance in leptons,
/// divided by `ChronoDistanceFactor` (default 48), clamped to at least
/// `ChronoMinimumDelay` (default 16). Short distances below `ChronoRangeMinimum`
/// are forced to the minimum.
pub fn compute_chrono_delay(rules: &GeneralRules, distance_leptons: i32) -> u32 {
    if !rules.chrono_trigger {
        return rules.chrono_minimum_delay.max(0) as u32;
    }
    let mut delay = if rules.chrono_distance_factor > 0 {
        distance_leptons / rules.chrono_distance_factor
    } else {
        0
    };
    if delay < rules.chrono_minimum_delay {
        delay = rules.chrono_minimum_delay;
    }
    if distance_leptons < rules.chrono_range_minimum {
        delay = rules.chrono_minimum_delay;
    }
    delay.max(0) as u32
}

/// Issue a teleport move command to an entity.
///
/// If the entity's base locomotor is not Teleport but it has `Teleporter=yes`,
/// a temporary override is applied for legacy callers.
///
/// The chrono delay is computed from the Euclidean distance in leptons
/// (see `compute_chrono_delay`). One cell = 256 leptons.
///
/// `is_harvester` skips the chrono lock entirely for harvester units (e.g.,
/// the Chrono Miner): `being_warped_ticks` is forced to 0 and the Relocate
/// phase finishes the teleport in a single tick. Non-harvester teleporters
/// (Chrono Legionnaire and friends) run the full distance-based delay.
///
/// Returns `true` if the teleport was initiated, `false` if the entity
/// is missing required fields.
pub fn issue_teleport_command(
    entities: &mut EntityStore,
    entity_id: u64,
    target: (u16, u16),
    rules: &GeneralRules,
    is_harvester: bool,
) -> bool {
    {
        let Some(entity) = entities.get_mut(entity_id) else {
            log::warn!("issue_teleport_command: entity {} not found", entity_id);
            return false;
        };

        // Legacy helper path: non-migrated callers may still put Teleport over a
        // non-Teleport base locomotor as a temporary override. CMIN far return uses
        // `issue_active_teleport_head_to_coord` instead, because Teleport is its
        // primary active locomotor in gamemd.
        if let Some(ref mut loco) = entity.locomotor {
            if loco.kind != LocomotorKind::Teleport {
                loco.begin_override(OverrideKind::Teleport);
            }
        }
    }

    start_teleport_state(entities, entity_id, target, rules, is_harvester)
}

/// Start a teleport because the active Teleport locomotor received Head_To_Coord.
///
/// This is the gamemd-shaped entry point for CMIN far return after the
/// Set_Destination bridge decides not to activate Drive piggyback.
pub fn issue_active_teleport_head_to_coord(
    entities: &mut EntityStore,
    entity_id: u64,
    target: (u16, u16),
    rules: &GeneralRules,
    is_harvester: bool,
) -> bool {
    {
        let Some(entity) = entities.get(entity_id) else {
            log::warn!(
                "issue_active_teleport_head_to_coord: entity {} not found",
                entity_id
            );
            return false;
        };
        if !entity
            .locomotor
            .as_ref()
            .is_some_and(|loco| loco.active_kind() == LocomotorKind::Teleport)
        {
            return false;
        }
    }
    start_teleport_state(entities, entity_id, target, rules, is_harvester)
}

fn start_teleport_state(
    entities: &mut EntityStore,
    entity_id: u64,
    target: (u16, u16),
    rules: &GeneralRules,
    is_harvester: bool,
) -> bool {
    let Some(entity) = entities.get_mut(entity_id) else {
        log::warn!("start_teleport_state: entity {} not found", entity_id);
        return false;
    };

    // Compute distance in leptons (1 cell = 256 leptons) for chrono delay.
    let dx = (entity.position.rx as i32 - target.0 as i32) * 256;
    let dy = (entity.position.ry as i32 - target.1 as i32) * 256;
    let dist_sq = (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64);
    let distance_leptons = isqrt_i64(dist_sq) as i32;
    let chrono_ticks = if is_harvester {
        0
    } else {
        compute_chrono_delay(rules, distance_leptons)
    };

    // Remove any existing ground movement.
    entity.movement_target = None;

    // Attach the teleport state machine — starts in Relocate (instant).
    entity.teleport_state = Some(TeleportState {
        phase: TeleportPhase::Relocate,
        target_rx: target.0,
        target_ry: target.1,
        being_warped_ticks: chrono_ticks,
    });
    entity.push_debug_event(
        0,
        DebugEventKind::SpecialMovementStart {
            kind: "Teleport".into(),
        },
    );

    true
}

/// Advance all in-progress teleport state machines.
///
/// Called once per simulation tick from `advance_tick()`.
/// Relocate executes instantly (one tick), then ChronoDelay counts down
/// `being_warped_ticks` each subsequent tick until the teleport completes.
pub fn tick_teleport_movement(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    live_order: &[u64],
    tick_ms: u32,
    sim_tick: u64,
    mut visuals: Option<&mut TeleportVisuals<'_>>,
) {
    if tick_ms == 0 {
        return;
    }

    // Collect entity IDs that need cleanup after ticking.
    let mut finished: Vec<u64> = Vec::new();

    let sorted_keys;
    let ordered_ids = if live_order.is_empty() {
        sorted_keys = entities.keys_sorted();
        sorted_keys.as_slice()
    } else {
        live_order
    };

    for &id in ordered_ids {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        let Some(ref mut teleport) = entity.teleport_state else {
            continue;
        };

        // Track phase before processing to detect transitions.
        let phase_before = teleport.phase;

        match teleport.phase {
            TeleportPhase::Relocate => {
                // Instant relocation in one tick — matches original Phase 0.
                let old_rx = entity.position.rx;
                let old_ry = entity.position.ry;
                let old_z = entity.position.z;
                if let Some(visuals) = visuals.as_deref_mut() {
                    visuals.spawn_warp_out(old_rx, old_ry, old_z);
                }
                entity.position.rx = teleport.target_rx;
                entity.position.ry = teleport.target_ry;
                entity.position.sub_x = CELL_CENTER_LEPTON;
                entity.position.sub_y = CELL_CENTER_LEPTON;
                entity.position.refresh_screen_coords();
                if let Some(visuals) = visuals.as_deref_mut() {
                    visuals.spawn_warp_out(
                        entity.position.rx,
                        entity.position.ry,
                        entity.position.z,
                    );
                }
                let layer = entity.locomotor.as_ref().map_or(
                    crate::sim::movement::locomotor::MovementLayer::Ground,
                    |l| l.layer,
                );
                occupancy.move_entity(
                    old_rx,
                    old_ry,
                    teleport.target_rx,
                    teleport.target_ry,
                    id,
                    layer,
                    entity.sub_cell,
                    CellListInsertion::from_category(entity.category),
                );
                // Harvester instant-warp: when chrono delay is 0, finish in one
                // tick (cleanup runs at end of this tick) — no post-warp lock.
                if teleport.being_warped_ticks == 0 {
                    finished.push(id);
                } else {
                    teleport.phase = TeleportPhase::ChronoDelay;
                }
            }
            TeleportPhase::ChronoDelay => {
                // Count down chrono delay ticks. Unit remains 50% translucent until 0.
                if teleport.being_warped_ticks > 0 {
                    teleport.being_warped_ticks -= 1;
                }
                if teleport.being_warped_ticks == 0 {
                    finished.push(id);
                }
            }
        }

        // Log phase transition if it changed.
        let phase_after = teleport.phase;
        if phase_after != phase_before {
            let phase_name = format!("{:?}", phase_after);
            // Drop the borrow on teleport before pushing debug event.
            let _ = teleport;
            entity.push_debug_event(
                sim_tick as u32,
                DebugEventKind::SpecialMovementPhase { phase: phase_name },
            );
        }
    }

    // Clean up finished teleports: remove TeleportState and restore base locomotor.
    for id in finished {
        if let Some(entity) = entities.get_mut(id) {
            entity.teleport_state = None;
            if let Some(ref mut loco) = entity.locomotor {
                if loco.is_overridden() {
                    loco.end_override();
                }
            }
            entity.push_debug_event(sim_tick as u32, DebugEventKind::SpecialMovementEnd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::rules::object_type::{ObjectCategory, ObjectType, PipScale};
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
    use crate::sim::pathfinding::PathGrid;
    use crate::util::fixed_math::SimFixed;

    fn make_drive_obj() -> ObjectType {
        ObjectType {
            id: "CMIN".to_string(),
            category: ObjectCategory::Vehicle,
            name: None,
            ui_name: None,
            cost: 0,
            strength: 100,
            armor: "none".to_string(),
            speed: 6,
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
            image: "CMIN".to_string(),
            power: 0,
            extra_power: 0,
            foundation: "1x1".to_string(),
            pixel_selection_bracket_delta: 0,
            build_cat: None,
            adjacent: 6,
            base_normal: true,
            eligibile_for_ally_building: false,
            crewed: false,
            voice_select: None,
            voice_move: None,
            voice_attack: None,
            die_sound: None,
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
            explodes: false,
            death_weapon: None,
            super_weapon: None,
            super_weapon2: None,
            spy_sat: false,
            gap_generator: false,
            psychic_detection_radius: 0,
            sensor_array: false,
            sensors: false,
            sensors_sight: 0,
            cloak_generator: false,
            radar: false,
            radar_invisible: false,
            radar_visible: false,
            harvester: false,
            refinery: false,
            bib: false,
            gate: false,
            deploy_time_ticks: 0,
            gate_close_delay_ticks: 0,
            storage: 0,
            free_unit: None,
            dock: vec![],
            queueing_cell: None,
            pads: Vec::new(),
            add_occupy: Vec::new(),
            remove_occupy: Vec::new(),
            unloading_class: None,
            ammo: -1,
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
            locomotor: LocomotorKind::Drive,
            speed_type: SpeedType::Track,
            movement_zone: MovementZone::Normal,
            considered_aircraft: false,
            zfudge_bridge: 7,
            too_big_to_fit_under_bridge: false,
            crashable: false,
            teleporter: true,
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
            cloning: false,
            exit_coord: None,
            crushable: false,
            deployed_crushable: true,
            crusher: false,
            no_force_shield: false,
            omni_crusher: false,
            omni_crush_resistant: false,
            immune_to_radiation: false,
            engineer: false,
            deployer: false,
            capturable: false,
            repairable: false,
            can_be_occupied: false,
            can_occupy_fire: false,
            show_occupant_pips: false,
            bridge_repair_hut: false,
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
            bunkerable: true,
            weapon_list: vec![],
            attack_cursor_on_friendlies: false,
            sabotage_cursor: false,
            c4: false,
            can_c4: false,
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
            wall: false,
            light_visibility: 0,
            light_intensity: 0.0,
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

    fn make_teleport_harvester_obj() -> ObjectType {
        let mut obj = make_drive_obj();
        obj.locomotor = LocomotorKind::Teleport;
        obj.harvester = true;
        obj.teleporter = true;
        obj.turret_rot = 5;
        obj
    }

    fn default_rules() -> GeneralRules {
        GeneralRules::default()
    }

    #[test]
    fn test_teleport_issues_and_completes() {
        let mut entities = EntityStore::new();
        let mut e = GameEntity::test_default(1, "CLEG", "Americans", 5, 5);
        e.position.z = 0;
        entities.insert(e);
        let rules = default_rules();

        assert!(issue_teleport_command(
            &mut entities,
            1,
            (20, 20),
            &rules,
            false
        ));
        let entity = entities.get(1).expect("should exist");
        let ts = entity
            .teleport_state
            .as_ref()
            .expect("should have TeleportState");
        assert_eq!(ts.phase, TeleportPhase::Relocate);
        assert!(
            ts.being_warped_ticks >= 16,
            "should have at least minimum delay"
        );

        // One tick relocates instantly (matches original Phase 0).
        tick_teleport_movement(&mut entities, &mut OccupancyGrid::new(), &[], 33, 0, None);

        let entity = entities.get(1).expect("should exist");
        assert_eq!(entity.position.rx, 20, "Should have relocated to target");
        assert_eq!(entity.position.ry, 20);
        let ts = entity.teleport_state.as_ref().expect("still warping");
        assert_eq!(
            ts.phase,
            TeleportPhase::ChronoDelay,
            "should be in chrono delay"
        );

        // Tick through ChronoDelay (being_warped_ticks countdown).
        let delay = ts.being_warped_ticks;
        for _ in 0..delay + 5 {
            tick_teleport_movement(&mut entities, &mut OccupancyGrid::new(), &[], 33, 0, None);
        }

        // TeleportState should be removed after completion.
        let entity = entities.get(1).expect("should exist");
        assert!(
            entity.teleport_state.is_none(),
            "TeleportState should be removed after completion"
        );
    }

    #[test]
    fn relocate_spawns_departure_and_arrival_warpout_rows() {
        let mut entities = EntityStore::new();
        let mut e = GameEntity::test_default(1, "CLEG", "Americans", 5, 5);
        e.position.z = 2;
        entities.insert(e);
        let rules = default_rules();

        assert!(issue_teleport_command(
            &mut entities,
            1,
            (8, 9),
            &rules,
            true
        ));

        let warp_out_type = crate::sim::intern::test_intern("WARPOUT");
        let mut effect_frame_counts = BTreeMap::new();
        effect_frame_counts.insert(warp_out_type, 13);
        let mut world_effects = Vec::new();
        {
            let mut visuals = TeleportVisuals {
                world_effects: &mut world_effects,
                effect_frame_counts: &effect_frame_counts,
                warp_out_type,
                warp_out_rate_ms: 42,
            };
            tick_teleport_movement(
                &mut entities,
                &mut OccupancyGrid::new(),
                &[],
                33,
                0,
                Some(&mut visuals),
            );
        }

        assert_eq!(world_effects.len(), 2);
        for (effect, (rx, ry)) in world_effects.iter().zip([(5, 5), (8, 9)]) {
            assert_eq!(effect.shp_name, warp_out_type);
            assert_eq!((effect.rx, effect.ry, effect.z), (rx, ry, 2));
            assert_eq!(effect.total_frames, 13);
            assert_eq!(effect.rate_ms, 42);
            let row = effect.anim_spawn.as_ref().expect("AnimClass row");
            assert_eq!(row.type_name, warp_out_type);
            assert_eq!((row.rx, row.ry, row.z), (rx, ry, 2));
            assert_eq!(row.delay, TELEPORT_WARP_DELAY);
            assert_eq!(row.loop_count, TELEPORT_WARP_LOOP_COUNT);
            assert_eq!(row.draw_flags, TELEPORT_WARP_DRAW_FLAGS);
            assert_eq!(row.z_adjust, TELEPORT_WARP_Z_ADJUST);
            assert_eq!(row.reverse, TELEPORT_WARP_REVERSE);
        }
    }

    #[test]
    fn chrono_delay_tick_does_not_spawn_extra_warpout_rows() {
        let mut entities = EntityStore::new();
        let e = GameEntity::test_default(1, "CLEG", "Americans", 5, 5);
        entities.insert(e);
        let rules = default_rules();

        assert!(issue_teleport_command(
            &mut entities,
            1,
            (20, 20),
            &rules,
            false
        ));

        let warp_out_type = crate::sim::intern::test_intern("WARPOUT");
        let effect_frame_counts = BTreeMap::new();
        let mut world_effects = Vec::new();
        {
            let mut visuals = TeleportVisuals {
                world_effects: &mut world_effects,
                effect_frame_counts: &effect_frame_counts,
                warp_out_type,
                warp_out_rate_ms: 120,
            };
            tick_teleport_movement(
                &mut entities,
                &mut OccupancyGrid::new(),
                &[],
                33,
                0,
                Some(&mut visuals),
            );
            tick_teleport_movement(
                &mut entities,
                &mut OccupancyGrid::new(),
                &[],
                33,
                1,
                Some(&mut visuals),
            );
        }

        assert_eq!(
            world_effects.len(),
            2,
            "only Relocate emits the verified departure and arrival rows"
        );
    }

    #[test]
    fn teleport_movement_uses_live_object_order_not_stable_id_scan() {
        fn teleporter(id: u64, rx: u16, ry: u16, target_rx: u16) -> GameEntity {
            let mut entity = GameEntity::test_default(id, "CLEG", "Americans", rx, ry);
            entity.teleport_state = Some(TeleportState {
                phase: TeleportPhase::Relocate,
                target_rx,
                target_ry: 20,
                being_warped_ticks: 0,
            });
            entity
        }

        let mut live_entities = EntityStore::new();
        live_entities.insert(teleporter(1, 5, 5, 21));
        live_entities.insert(teleporter(2, 6, 5, 22));

        tick_teleport_movement(
            &mut live_entities,
            &mut OccupancyGrid::new(),
            &[2],
            33,
            0,
            None,
        );

        let first = live_entities.get(1).expect("id 1");
        assert_eq!(
            (first.position.rx, first.position.ry),
            (5, 5),
            "non-live-order IDs are not swept by stable-id fallback"
        );
        assert!(first.teleport_state.is_some());
        let second = live_entities.get(2).expect("id 2");
        assert_eq!((second.position.rx, second.position.ry), (22, 20));
        assert!(second.teleport_state.is_none());

        let mut fallback_entities = EntityStore::new();
        fallback_entities.insert(teleporter(1, 5, 5, 21));
        fallback_entities.insert(teleporter(2, 6, 5, 22));

        tick_teleport_movement(
            &mut fallback_entities,
            &mut OccupancyGrid::new(),
            &[],
            33,
            0,
            None,
        );

        assert_eq!(
            fallback_entities.get(1).map(|entity| (
                entity.position.rx,
                entity.position.ry,
                entity.teleport_state.is_none()
            )),
            Some((21, 20, true))
        );
        assert_eq!(
            fallback_entities.get(2).map(|entity| (
                entity.position.rx,
                entity.position.ry,
                entity.teleport_state.is_none()
            )),
            Some((22, 20, true))
        );
    }

    #[test]
    fn test_teleport_with_piggyback_restores_drive() {
        let mut entities = EntityStore::new();
        let obj = make_drive_obj();
        let loco = LocomotorState::from_object_type(&obj, 1500);
        let mut e = GameEntity::test_default(1, "CMIN", "Americans", 5, 5);
        e.locomotor = Some(loco);
        entities.insert(e);
        let rules = default_rules();

        // Pass is_harvester=false so the test still exercises the full chrono-delay path.
        // (CMIN type fixture used here has harvester=false; the harvester instant-warp
        // path is covered by the dedicated tests below.)
        assert!(issue_teleport_command(
            &mut entities,
            1,
            (20, 20),
            &rules,
            false
        ));
        // Should have overridden to Teleport.
        let entity = entities.get(1).expect("should exist");
        let loco = entity.locomotor.as_ref().expect("has loco");
        assert_eq!(loco.kind, LocomotorKind::Teleport);
        assert!(loco.is_overridden());

        // Complete the whole sequence: 1 tick for Relocate + chrono delay ticks.
        for _ in 0..200 {
            tick_teleport_movement(&mut entities, &mut OccupancyGrid::new(), &[], 33, 0, None);
        }

        // Should have restored to Drive.
        let entity = entities.get(1).expect("should exist");
        let loco = entity.locomotor.as_ref().expect("has loco");
        assert_eq!(loco.kind, LocomotorKind::Drive);
        assert!(!loco.is_overridden());
        assert_eq!(loco.layer, MovementLayer::Ground);
    }

    #[test]
    fn teleporter_empty_destination_starts_teleport_without_drive_override() {
        let mut entities = EntityStore::new();
        let obj = make_teleport_harvester_obj();
        let loco = LocomotorState::from_object_type(&obj, 1500);
        let mut e = GameEntity::test_default(1, "CMIN", "Americans", 5, 5);
        e.locomotor = Some(loco);
        entities.insert(e);
        let rules = default_rules();

        assert!(crate::sim::movement::set_destination_for_teleporter_entity(
            &mut entities,
            None,
            1,
            (20, 20),
            SimFixed::from_num(6),
            false,
            None,
            None,
            None,
            None,
            None,
            false,
            &rules,
            true,
            true,
            false,
        ));

        let entity = entities.get(1).expect("entity");
        assert!(entity.teleport_state.is_some());
        let loco = entity.locomotor.as_ref().expect("loco");
        assert_eq!(loco.active_kind(), LocomotorKind::Teleport);
        assert_eq!(loco.primary_kind(), LocomotorKind::Teleport);
        assert!(loco.piggyback.is_none());
        assert!(!loco.is_overridden());
    }

    #[test]
    fn teleporter_building_destination_activates_drive_piggyback() {
        let mut entities = EntityStore::new();
        let obj = make_teleport_harvester_obj();
        let loco = LocomotorState::from_object_type(&obj, 1500);
        let mut e = GameEntity::test_default(1, "CMIN", "Americans", 5, 5);
        e.locomotor = Some(loco);
        entities.insert(e);
        let rules = default_rules();
        let grid = PathGrid::test_all_passable(32, 32);

        assert!(crate::sim::movement::set_destination_for_teleporter_entity(
            &mut entities,
            Some(&grid),
            1,
            (10, 10),
            SimFixed::from_num(6),
            false,
            None,
            None,
            None,
            None,
            None,
            false,
            &rules,
            true,
            true,
            true,
        ));

        let entity = entities.get(1).expect("entity");
        assert!(entity.teleport_state.is_none());
        assert!(entity.movement_target.is_some());
        let loco = entity.locomotor.as_ref().expect("loco");
        assert_eq!(loco.active_kind(), LocomotorKind::Drive);
        assert_eq!(loco.primary_kind(), LocomotorKind::Teleport);
        assert!(loco.piggyback.is_some());

        entities.get_mut(1).expect("entity").movement_target = None;
        assert_eq!(
            crate::sim::movement::tick_locomotor_piggyback_restore(&mut entities),
            1
        );
        let loco = entities
            .get(1)
            .and_then(|entity| entity.locomotor.as_ref())
            .expect("loco");
        assert_eq!(loco.active_kind(), LocomotorKind::Teleport);
        assert!(loco.is_primary_active());
    }

    #[test]
    fn test_chrono_delay_formula() {
        let mut rules = default_rules();
        // Default: factor=48, minimum=16, trigger=true, range_minimum=0

        // Short distance: 256 leptons (1 cell) → 256/48 = 5, clamped to 16
        assert_eq!(compute_chrono_delay(&rules, 256), 16);

        // Medium distance: 5120 leptons (20 cells) → 5120/48 = 106
        assert_eq!(compute_chrono_delay(&rules, 5120), 106);

        // Very short distance below range minimum
        rules.chrono_range_minimum = 512;
        assert_eq!(compute_chrono_delay(&rules, 200), 16); // forced to minimum

        // ChronoTrigger=false → always minimum
        rules.chrono_trigger = false;
        assert_eq!(compute_chrono_delay(&rules, 5120), 16);
    }

    /// Harvester units skip the chrono lock entirely — when is_harvester=true
    /// the lock duration is 0 regardless of distance.
    #[test]
    fn test_harvester_skips_chrono_delay() {
        let mut entities = EntityStore::new();
        let e = GameEntity::test_default(1, "CMIN", "Americans", 5, 5);
        entities.insert(e);
        let rules = default_rules();

        // Long distance (~80 cells diagonal) — non-harvester would compute ~604 ticks delay.
        assert!(issue_teleport_command(
            &mut entities,
            1,
            (90, 90),
            &rules,
            true
        ));
        let ts = entities
            .get(1)
            .and_then(|e| e.teleport_state.as_ref())
            .expect("should have TeleportState");
        assert_eq!(
            ts.being_warped_ticks, 0,
            "harvester instant-warp must zero the chrono lock"
        );
    }

    /// With is_harvester=true, the Relocate phase finishes the teleport in a single
    /// tick (skipping ChronoDelay).
    #[test]
    fn test_harvester_relocate_cleans_up_in_one_tick() {
        let mut entities = EntityStore::new();
        let obj = make_drive_obj();
        let loco = LocomotorState::from_object_type(&obj, 1500);
        let mut e = GameEntity::test_default(1, "CMIN", "Americans", 5, 5);
        e.locomotor = Some(loco);
        entities.insert(e);
        let rules = default_rules();

        assert!(issue_teleport_command(
            &mut entities,
            1,
            (20, 20),
            &rules,
            true
        ));
        // Override applied at issue time.
        let entity = entities.get(1).expect("should exist");
        assert!(entity.locomotor.as_ref().expect("loco").is_overridden());

        // Single tick: position snaps, then cleanup runs because being_warped_ticks==0.
        tick_teleport_movement(&mut entities, &mut OccupancyGrid::new(), &[], 33, 0, None);

        let entity = entities.get(1).expect("should exist");
        assert_eq!(entity.position.rx, 20);
        assert_eq!(entity.position.ry, 20);
        assert!(
            entity.teleport_state.is_none(),
            "harvester teleport should clean up in one tick"
        );
        let loco = entity.locomotor.as_ref().expect("has loco");
        assert_eq!(loco.kind, LocomotorKind::Drive, "base locomotor restored");
        assert!(!loco.is_overridden(), "override ended");
    }

    /// Regression: non-harvester (Chrono Legionnaire path) still goes through the
    /// full Relocate → ChronoDelay countdown.
    #[test]
    fn test_non_harvester_uses_full_chrono_delay() {
        let mut entities = EntityStore::new();
        let e = GameEntity::test_default(1, "CLEG", "Americans", 5, 5);
        entities.insert(e);
        let rules = default_rules();

        assert!(issue_teleport_command(
            &mut entities,
            1,
            (20, 20),
            &rules,
            false
        ));
        let initial_ticks = entities
            .get(1)
            .and_then(|e| e.teleport_state.as_ref())
            .map(|t| t.being_warped_ticks)
            .expect("teleport_state");
        assert!(
            initial_ticks > 0,
            "non-harvester must keep the distance-based chrono lock"
        );

        // Tick 1: Relocate snaps position and transitions to ChronoDelay (NOT cleanup).
        tick_teleport_movement(&mut entities, &mut OccupancyGrid::new(), &[], 33, 0, None);
        let ts = entities
            .get(1)
            .and_then(|e| e.teleport_state.as_ref())
            .expect("still warping after Relocate");
        assert_eq!(ts.phase, TeleportPhase::ChronoDelay);
        assert_eq!(ts.being_warped_ticks, initial_ticks);
    }
}
