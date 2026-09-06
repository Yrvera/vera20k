//! One live object turn: AI, locomotor tails and synchronous cell/lifecycle effects.
//!
//! The master frame owns phase order. This owner completes one object's effects
//! before the live Logic cursor advances; no lifecycle work is deferred to a
//! batch tail. Native order and coordinate evidence remain beside each seam.

use std::collections::BTreeSet;

use super::{Simulation, techno_ai};
use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::lifecycle_request::LifecycleRequest;
use crate::sim::movement::{
    self, homing_movement, parachute_descent, rocket_movement, teleport_movement,
};
use crate::sim::pathfinding::PathGrid;

/// Whether this Unit visit reaches FootClass's SHP body-counter cadence.
///
/// An entry-active TubeMovement owns the UnitClass AI call and returns before
/// FootClass AI. Tube state armed later during an ordinary Foot visit does not
/// retroactively suppress work already reached by that visit, so only the
/// entry snapshot belongs in this admission predicate.
pub(super) fn shp_vehicle_counter_admitted(tube_active_at_entry: bool) -> bool {
    !tube_active_at_entry
}

#[derive(Default)]
pub(super) struct LiveObjectPassOutcome {
    pub movement: movement::MovementTickStats,
    pub destroyed_structure: bool,
    pub tube_turn_owned_ids: BTreeSet<u64>,
}

#[derive(Default)]
struct ObjectTurnOutcome {
    movement: movement::MovementTickStats,
    destroyed_structure: bool,
    tube_owned: bool,
}

impl Simulation {
    pub(super) fn advance_live_object_pass(
        &mut self,
        rules: Option<&RuleSet>,
        path_grid: Option<&PathGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) -> LiveObjectPassOutcome {
        let miner_config = rules.map(crate::sim::miner::MinerConfig::from_rules);
        let terrain_spawner_cells = self
            .production
            .terrain_spawners
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        let object_ctx = techno_ai::ObjectAiCtx {
            path_grid,
            overlay_registry,
            terrain_spawner_cells: Some(&terrain_spawner_cells),
            miner_config: miner_config.as_ref(),
        };

        let mut outcome = LiveObjectPassOutcome::default();
        self.for_each_live_object(|sim, stable_id| {
            let turn = sim.advance_live_object_turn(stable_id, rules, object_ctx);
            outcome.movement.merge(turn.movement);
            outcome.destroyed_structure |= turn.destroyed_structure;
            if turn.tube_owned {
                outcome.tube_turn_owned_ids.insert(stable_id);
            }
        });
        outcome
    }

    fn advance_live_object_turn(
        &mut self,
        stable_id: u64,
        rules: Option<&RuleSet>,
        object_ctx: techno_ai::ObjectAiCtx<'_>,
    ) -> ObjectTurnOutcome {
        let sim = self;
        let path_grid = object_ctx.path_grid;
        let overlay_registry = object_ctx.overlay_registry;
        let mut outcome = ObjectTurnOutcome::default();
        // UnitClass::AI / InfantryClass::AI give an active TubeMovement
        // object the whole live-object turn.  Capture before the leaf:
        // successful finalization clears the payload but must still skip
        // every ordinary locomotor tail and the second mission checkpoint.
        let tube_active_at_entry = sim.substrate.entities.get(stable_id).is_some_and(|entity| {
            !entity.dying
                && matches!(
                    entity.category,
                    EntityCategory::Unit | EntityCategory::Infantry
                )
                && entity.low_bridge_tube_state.is_some()
        });
        let was_structure = sim
            .substrate
            .entities
            .get(stable_id)
            .is_some_and(|entity| entity.category == EntityCategory::Structure);
        sim.object_ai_visit_one(stable_id, rules, object_ctx);
        if was_structure
            && sim
                .substrate
                .entities
                .get(stable_id)
                .is_none_or(|entity| entity.dying)
        {
            outcome.destroyed_structure = true;
        }
        if sim
            .substrate
            .entities
            .get(stable_id)
            .is_none_or(|entity| entity.dying)
        {
            return outcome;
        }

        let before_movement = sim.movement_sound_probe(stable_id);
        let cell_before_movement = sim
            .substrate
            .entities
            .get(stable_id)
            .map(|entity| (entity.position.rx, entity.position.ry));
        let one = [stable_id];
        outcome
            .movement
            .merge(movement::tick_movement_object_with_grids(
                &mut sim.substrate.entities,
                stable_id,
                path_grid,
                &sim.terrain_costs,
                &sim.house_alliances,
                &mut sim.substrate.occupancy,
                &mut sim.substrate.cell_occupation,
                &mut sim.substrate.raw_cell_occupation,
                &mut sim.substrate.next_occupancy_enter_order,
                &mut sim.scenario_rng,
                sim.session.tick,
                sim.session.binary_frame,
                sim.zone_grid.as_ref(),
                sim.resolved_terrain.as_ref(),
                sim.overlay_grid.as_ref(),
                overlay_registry,
                sim.playfield_bounds,
                &sim.terrain_speed_config,
                sim.close_enough,
                sim.path_delay_ticks,
                sim.blockage_path_delay_ticks,
                &mut sim.interner,
                rules,
                &mut sim.sound_events,
                &mut sim.pending_lifecycle_requests,
            ));

        // FootClass advances the SHP Unit body counter immediately after
        // this object's locomotor Process, against the still-current
        // absolute binary frame. The global frame commits only after the
        // complete live-object pass.
        if shp_vehicle_counter_admitted(tube_active_at_entry) {
            let shp_vehicle_cadence = sim.substrate.entities.get(stable_id).and_then(|entity| {
                if entity.category != EntityCategory::Unit || entity.is_voxel {
                    return None;
                }
                let object = rules?.object(sim.interner.resolve(entity.type_ref))?;
                Some(crate::sim::animation::ShpVehicleCadence {
                    walk_rate: object.walk_rate,
                    idle_rate: object.idle_rate,
                })
            });
            if let (Some(cadence), Some(entity)) = (
                shp_vehicle_cadence,
                sim.substrate.entities.get_mut(stable_id),
            ) {
                crate::sim::animation::tick_shp_vehicle_body_frame_counter(
                    entity,
                    cadence,
                    sim.session.binary_frame,
                );
            }
        }

        // A direction-8 producer also ends this object's ordinary turn as
        // soon as it arms TubeMovement.  The leaf itself starts on the
        // object's next visit; an entry-active leaf may have cleared the
        // payload above, hence the captured half of this predicate.
        let tube_owns_whole_turn = tube_active_at_entry
            || sim.substrate.entities.get(stable_id).is_some_and(|entity| {
                matches!(
                    entity.category,
                    EntityCategory::Unit | EntityCategory::Infantry
                ) && entity.low_bridge_tube_state.is_some()
            });
        if tube_owns_whole_turn {
            outcome.tube_owned = true;
            return outcome;
        }

        sim.tick_air_movement_with_cell_lists_one(stable_id);
        let teleport_relocating = sim
            .substrate
            .entities
            .get(stable_id)
            .and_then(|entity| entity.teleport_state.as_ref())
            .is_some_and(|state| {
                state.phase == crate::sim::movement::teleport_movement::TeleportPhase::Relocate
            });
        if let Some(rules) = rules {
            let warp_out_type = sim.interner.intern(&rules.general.warp_out.name);
            let warp_out_total_frames = rules
                .effect_frame_count(&rules.general.warp_out.name)
                .unwrap_or(teleport_movement::FALLBACK_WARP_FRAME_COUNT);
            let mut teleport_visuals = teleport_movement::TeleportVisuals {
                world_effects: &mut sim.world_effects,
                warp_out_type,
                warp_out_total_frames,
                warp_out_frame_delay: rules.general.warp_out.frame_delay,
            };
            teleport_movement::tick_teleport_movement(
                &mut sim.substrate.entities,
                &mut sim.substrate.occupancy,
                &one,
                sim.session.tick,
                Some(&mut teleport_visuals),
            );
        } else {
            teleport_movement::tick_teleport_movement(
                &mut sim.substrate.entities,
                &mut sim.substrate.occupancy,
                &one,
                sim.session.tick,
                None,
            );
        }
        sim.pending_rocket_detonations
            .extend(rocket_movement::tick_rocket_movement(
                &mut sim.substrate.entities,
                &one,
                sim.session.tick,
            ));
        sim.tick_tunnel_locomotor_one(stable_id, path_grid);
        sim.tick_drop_pod_locomotor_one(stable_id, path_grid);
        let _ = homing_movement::tick_homing_movement(
            &mut sim.substrate.entities,
            &one,
            sim.session.tick,
        );
        if let Some(rules) = rules {
            parachute_descent::tick_parachute_descent_in_order(
                &mut sim.substrate.entities,
                &one,
                rules.general.parachute_max_fall_rate,
                sim.session.tick,
            );
        }
        movement::tick_locomotor_piggyback_restore_one(&mut sim.substrate.entities, stable_id);

        let cell_after_movement = sim
            .substrate
            .entities
            .get(stable_id)
            .map(|entity| (entity.position.rx, entity.position.ry));
        if let Some(rules) = rules {
            sim.move_unit_sensor_after_cell_change(
                stable_id,
                cell_before_movement,
                cell_after_movement,
                rules,
            );
        }
        if teleport_relocating {
            // `TeleportLocomotionClass` arrival owns the exceptional exact
            // outside clear at 0x00719A99; it must not flow through the
            // ordinary promote-only per-cell writer.
            sim.clear_entity_playfield_membership_after_teleport(stable_id);
        } else if cell_before_movement != cell_after_movement {
            // `FootClass::PerCellProcess @ 0x004D85D0` runs the `Sensors=`
            // neighbour scan on its cell-enter arm, after the sensor
            // deposit has moved (`0x004D8611`/`0x004D8621`, issued just
            // above) and before its `FUN_006F5090` playfield-membership
            // tail — which is the promote below.
            if let Some(rules) = rules {
                crate::sim::world::techno_ai_cloak::uncloak_on_sensor_neighbour_after_cell_entry(
                    sim, stable_id, rules,
                );
            }
            sim.promote_entity_playfield_membership_after_move(stable_id);
        }

        let mut lifecycle_requests = std::mem::take(&mut sim.pending_lifecycle_requests);
        for request in lifecycle_requests.drain(..) {
            let LifecycleRequest::Uninit { stable_id, .. } = request;
            sim.release_move_sound(stable_id);
            if let Some(rules) = rules {
                sim.apply_lifecycle_request_with_rules(request, rules);
            } else {
                sim.apply_lifecycle_request(request);
            }
        }
        debug_assert!(lifecycle_requests.is_empty());
        sim.pending_lifecycle_requests = lifecycle_requests;

        sim.tick_move_sound_after_process(stable_id, before_movement, rules);
        sim.object_ai_post_movement_promote_one(stable_id, rules);
        outcome
    }
}
