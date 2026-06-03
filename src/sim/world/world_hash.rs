//! Deterministic state hashing for the Simulation.
//!
//! Produces a reproducible u64 hash over the entire simulation state:
//! tick counter, RNG state, production queues, fog-of-war, entity components.
//! Used for replay verification and desync detection in multiplayer.
//!
//! Dependency rules: same as sim/ (depends on rules/, map/; never render/ui/audio/net).

use std::hash::{Hash, Hasher};

use super::Simulation;

fn hash_drive_track_state(
    state: &crate::sim::movement::drive_track::DriveTrackState,
    hasher: &mut impl Hasher,
) {
    state.raw_track_index.hash(hasher);
    state.point_index.hash(hasher);
    state.residual.hash(hasher);
    state.transform_flags.hash(hasher);
    state.head_offset_x.hash(hasher);
    state.head_offset_y.hash(hasher);
    state.cell_offset_x.hash(hasher);
    state.cell_offset_y.hash(hasher);
    state.target_facing.hash(hasher);
}

/// Fold the `MissionCom` mission component into the state hash.
///
/// Explicit field fold (MissionCom intentionally does NOT derive `Hash`): enum
/// discriminants cast to `u16` (matching the `category as u8` idiom in
/// `hash_entities`), `Option`s as a `0u8`/`1u8` presence tag plus value. As of
/// Slice 8 `mission` is canonical hashed lockstep state, no longer an unhashed
/// shadow; `refresh_mission_shadow` keeps `current`/`substate` a deterministic
/// projection of the authoritative machines, so this fold cannot desync.
fn hash_mission_com(mission: &crate::sim::mission::MissionCom, hasher: &mut impl Hasher) {
    (mission.current as u16).hash(hasher);
    match mission.queued {
        Some(m) => {
            1u8.hash(hasher);
            (m as u16).hash(hasher);
        }
        None => 0u8.hash(hasher),
    }
    match mission.suspended {
        Some(m) => {
            1u8.hash(hasher);
            (m as u16).hash(hasher);
        }
        None => 0u8.hash(hasher),
    }
    mission.substate.hash(hasher);
    mission.timer.start_frame.hash(hasher);
    mission.timer.duration.hash(hasher);
    mission.tick_counter.hash(hasher);
}

impl Simulation {
    /// Deterministic state hash over canonicalized simulation state.
    ///
    /// Hashes tick, both RNG streams, production, fog, alliances, and all entity
    /// components in stable-entity-ID order (EntityStore keys_sorted) for determinism.
    pub fn state_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        self.tick.hash(&mut hasher);
        self.total_sim_ms.hash(&mut hasher);
        self.binary_frame.hash(&mut hasher);
        // Hash ALL THREE RNG streams in a fixed order. Order is part of the hash
        // contract and must never change. Hashing only some streams would let a
        // divergence in another produce identical hashes on two desynced clients
        // (desync detector goes blind exactly where the RNG-stream split matters).
        self.scenario_rng.hash_state(&mut hasher);
        self.main_rng.hash_state(&mut hasher);
        // mapgen_rng (gamemd g_MapGenRng): appended AFTER the two gameplay streams.
        // This order is part of the hash contract and must never change.
        self.mapgen_rng.hash_state(&mut hasher);
        self.substrate.next_stable_entity_id.hash(&mut hasher);
        self.substrate.next_occupancy_enter_order.hash(&mut hasher);

        // LogicClass active-object order — authoritative (drives reconciliation order).
        let order = self.substrate.logic.as_slice();
        order.len().hash(&mut hasher);
        for id in order {
            id.hash(&mut hasher);
        }

        self.hash_game_options(&mut hasher);
        self.hash_houses(&mut hasher);
        self.hash_production(&mut hasher);
        self.hash_power_states(&mut hasher);
        self.hash_fog_and_alliances(&mut hasher);
        self.hash_bridge_state(&mut hasher);
        self.hash_overlay_grid(&mut hasher);
        self.hash_smudge_grid(&mut hasher);
        self.hash_super_weapons(&mut hasher);
        self.hash_entities(&mut hasher);
        self.hash_particle_systems(&mut hasher);

        hasher.finish()
    }

    /// Hash all particle systems in stable-id order (BTreeMap iteration).
    /// Each system contributes its type, position, lifetime, and ordered particle list.
    fn hash_particle_systems(&self, hasher: &mut impl Hasher) {
        self.particle_systems.len().hash(hasher);
        for (id, sys) in self.particle_systems.iter() {
            id.hash(hasher);
            sys.type_id.0.hash(hasher);
            sys.coords.x.hash(hasher);
            sys.coords.y.hash(hasher);
            sys.coords.z.hash(hasher);
            sys.lifetime.hash(hasher);
            sys.facing.hash(hasher);
            sys.marked_for_deletion.hash(hasher);
            sys.done_spawning.hash(hasher);
            sys.particles.len().hash(hasher);
            for p in &sys.particles {
                p.type_id.0.hash(hasher);
                p.coords.x.hash(hasher);
                p.coords.y.hash(hasher);
                p.coords.z.hash(hasher);
                p.lifetime_remaining.hash(hasher);
                p.animation_state.hash(hasher);
                p.translucency.hash(hasher);
                p.state_advance_counter.hash(hasher);
                p.marked_for_deletion.hash(hasher);
            }
        }
    }

    /// Hash per-match game options for lockstep verification.
    fn hash_game_options(&self, hasher: &mut impl Hasher) {
        let opts = &self.game_options;
        opts.short_game.hash(hasher);
        opts.bases.hash(hasher);
        opts.bridges_destroyable.hash(hasher);
        opts.super_weapons.hash(hasher);
        opts.build_off_ally.hash(hasher);
        opts.crates.hash(hasher);
        opts.mcv_redeploy.hash(hasher);
        opts.fog_of_war.hash(hasher);
        opts.shroud.hash(hasher);
        opts.tiberium_grows.hash(hasher);
        opts.multi_engineer.hash(hasher);
        opts.harvester_truce.hash(hasher);
        opts.ally_change_allowed.hash(hasher);
        opts.starting_credits.hash(hasher);
        opts.unit_count.hash(hasher);
        opts.tech_level.hash(hasher);
        opts.game_speed.hash(hasher);
        opts.ai_difficulty.hash(hasher);
        opts.ai_players.hash(hasher);
    }

    /// Hash per-player house state (BTreeMap = deterministic order).
    fn hash_houses(&self, hasher: &mut impl Hasher) {
        for (owner, house) in &self.houses {
            owner.hash(hasher);
            house.credits.hash(hasher);
            house.side_index.hash(hasher);
            house.is_human.hash(hasher);
            house.is_defeated.hash(hasher);
            house.has_won.hash(hasher);
            house.has_lost.hash(hasher);
            house.owned_building_count.hash(hasher);
            house.owned_unit_count.hash(hasher);
            house.tech_level.hash(hasher);
            if let Some((rx, ry)) = house.rally_point {
                1u8.hash(hasher);
                rx.hash(hasher);
                ry.hash(hasher);
            } else {
                0u8.hash(hasher);
            }
            if let Some((rx, ry)) = house.base_center {
                1u8.hash(hasher);
                rx.hash(hasher);
                ry.hash(hasher);
            } else {
                0u8.hash(hasher);
            }
        }
    }

    /// Hash all production-related state: queues, ready items, resources.
    fn hash_production(&self, hasher: &mut impl Hasher) {
        for (owner, queues) in &self.production.queues_by_owner {
            owner.hash(hasher);
            for (category, queue) in queues {
                category.hash(hasher);
                for item in queue {
                    item.owner.hash(hasher);
                    item.type_id.hash(hasher);
                    item.queue_category.hash(hasher);
                    item.state.hash(hasher);
                    item.total_base_frames.hash(hasher);
                    item.remaining_base_frames.hash(hasher);
                    item.progress_carry.hash(hasher);
                    item.enqueue_order.hash(hasher);
                }
            }
        }
        for (owner, ready) in &self.production.ready_by_owner {
            owner.hash(hasher);
            for type_id in ready {
                type_id.hash(hasher);
            }
        }
        for (owner, categories) in &self.production.active_producer_by_owner {
            owner.hash(hasher);
            for (category, sid) in categories {
                category.hash(hasher);
                sid.hash(hasher);
            }
        }
        self.production.next_enqueue_order.hash(hasher);

        for (&(rx, ry), node) in &self.production.resource_nodes {
            rx.hash(hasher);
            ry.hash(hasher);
            (node.resource_type as u8).hash(hasher);
            node.remaining.hash(hasher);
        }
        self.production.ore_growth_state.hash_state(hasher);
        // Hash terrain spawners (TIBTRE-style ore generators).
        for (&(rx, ry), spawner) in &self.production.terrain_spawners {
            rx.hash(hasher);
            ry.hash(hasher);
            spawner.hash(hasher);
        }
        for (&stable_id, terrain) in &self.production.terrain_objects {
            stable_id.hash(hasher);
            terrain.hash(hasher);
        }
        for (&(rx, ry), &stable_id) in &self.production.terrain_object_cells {
            rx.hash(hasher);
            ry.hash(hasher);
            stable_id.hash(hasher);
        }
        self.production.next_terrain_object_id.hash(hasher);
        for (&(rx, ry), &bits) in &self.production.terrain_occupation_bits {
            rx.hash(hasher);
            ry.hash(hasher);
            bits.hash(hasher);
        }
        for &(rx, ry) in &self.production.tiberium_spawning_terrain_cells {
            rx.hash(hasher);
            ry.hash(hasher);
        }
        self.production.default_ore_overlay_id.hash(hasher);
        // Hash refinery radio/contact state.
        for (&ref_sid, contacts) in &self.production.dock_reservations.contacts {
            ref_sid.hash(hasher);
            for &miner_sid in contacts {
                miner_sid.hash(hasher);
            }
        }
        // `waiting_retry_queue` removed in Slice 4 (V3-proven FIFO DRIFT — gamemd
        // stores no wait-queue; rejected dockers re-probe on demand). The
        // remaining `contacts`/`contact_entered`/`on_pad` folds are the
        // transitional registry mirror, retired in a later slice.
        for (&ref_sid, &miner_sid) in &self.production.dock_reservations.contact_entered {
            ref_sid.hash(hasher);
            miner_sid.hash(hasher);
        }
        for (&ref_sid, &miner_sid) in &self.production.dock_reservations.on_pad {
            ref_sid.hash(hasher);
            miner_sid.hash(hasher);
        }
    }

    /// Hash per-player power states for deterministic replay.
    fn hash_power_states(&self, hasher: &mut impl Hasher) {
        // BTreeMap<InternedId, _> iterates in deterministic sorted order.
        for (owner_id, state) in &self.power_states {
            owner_id.hash(hasher);
            state.total_output.hash(hasher);
            state.total_drain.hash(hasher);
            state.power_blackout_remaining.hash(hasher);
        }
    }

    /// Hash fog-of-war visibility and house alliance data.
    fn hash_fog_and_alliances(&self, hasher: &mut impl Hasher) {
        self.fog.width.hash(hasher);
        self.fog.height.hash(hasher);
        for (owner, fog) in &self.fog.by_owner {
            owner.hash(hasher);
            fog.cells_raw().hash(hasher);
        }
        for (owner, allies) in &self.house_alliances {
            owner.hash(hasher);
            for ally in allies {
                ally.hash(hasher);
            }
        }
    }

    fn hash_bridge_state(&self, hasher: &mut impl Hasher) {
        let Some(bridge_state) = &self.bridge_state else {
            0u8.hash(hasher);
            return;
        };
        1u8.hash(hasher);
        let mut entries: Vec<_> = bridge_state.iter_cells().collect();
        entries.sort_by_key(|((rx, ry), _)| (*rx, *ry));
        for ((rx, ry), cell) in entries {
            rx.hash(hasher);
            ry.hash(hasher);
            cell.deck_present.hash(hasher);
            cell.damage_state.hash(hasher);
            cell.destroyable.hash(hasher);
            cell.deck_level.hash(hasher);
            cell.bridge_group_id.hash(hasher);
            cell.axis.hash(hasher);
            cell.role.hash(hasher);
            cell.anchor_span_id.hash(hasher);
            cell.overlay_byte.hash(hasher);
            cell.damaged_variant.hash(hasher);
            cell.bridgehead_anchor_class.hash(hasher);
        }
        // Hash AnchorSpan registry (Task 7 added this field). BTreeMap iterates
        // in sorted-key order, so iteration is deterministic.
        for (id, span) in bridge_state.anchor_spans() {
            id.hash(hasher);
            span.hash(hasher);
        }
        bridge_state.endpoint_records().len().hash(hasher);
        for record in bridge_state.endpoint_records() {
            record.endpoint_a.hash(hasher);
            record.endpoint_b.hash(hasher);
            record.group_id.hash(hasher);
            record.active.hash(hasher);
            record.bridge_kind.hash(hasher);
        }
    }

    fn hash_overlay_grid(&self, hasher: &mut impl Hasher) {
        let Some(overlay_grid) = &self.overlay_grid else {
            0u8.hash(hasher);
            return;
        };
        1u8.hash(hasher);
        for (rx, ry, cell) in overlay_grid.iter_occupied() {
            rx.hash(hasher);
            ry.hash(hasher);
            cell.overlay_id.hash(hasher);
            cell.overlay_data.hash(hasher);
        }
    }

    /// Hash all occupied smudge cells in stable cell-coord order.
    /// Must be deterministic across replays — visual divergence between clients
    /// is jarring even though smudges are cosmetic.
    fn hash_smudge_grid(&self, hasher: &mut impl Hasher) {
        let Some(grid) = &self.smudge_grid else {
            0u8.hash(hasher);
            return;
        };
        1u8.hash(hasher);
        let mut entries: Vec<(u16, u16, Option<u16>, Option<(u16, u16)>, u8)> = grid
            .iter_occupied()
            .map(|(rx, ry, c)| (rx, ry, c.type_id, c.footprint_origin, c.frame_offset))
            .collect();
        entries.sort();
        entries.len().hash(hasher);
        for e in &entries {
            e.hash(hasher);
        }
    }

    /// Hash per-house superweapon state and active lightning storm.
    fn hash_super_weapons(&self, hasher: &mut impl Hasher) {
        for (owner, weapons) in &self.super_weapons {
            owner.hash(hasher);
            for (type_id, inst) in weapons {
                type_id.hash(hasher);
                inst.is_active.hash(hasher);
                inst.is_ready.hash(hasher);
                inst.is_suspended.hash(hasher);
                inst.charge_start_tick.hash(hasher);
                inst.charge_duration.hash(hasher);
                inst.charge_drain_state.hash(hasher);
                inst.ready_tick.hash(hasher);
            }
        }
        // Hash lightning storm global state.
        self.lightning_storm.is_some().hash(hasher);
        if let Some(ref ls) = self.lightning_storm {
            ls.owner.hash(hasher);
            ls.target_rx.hash(hasher);
            ls.target_ry.hash(hasher);
            ls.deferment_remaining.hash(hasher);
            ls.duration_remaining.hash(hasher);
            ls.center_bolt_timer.hash(hasher);
            ls.scatter_bolt_timer.hash(hasher);
            ls.last_bolt_rx.hash(hasher);
            ls.last_bolt_ry.hash(hasher);
        }
        // Hash queued lightning storm.
        self.queued_lightning_storm.is_some().hash(hasher);
        if let Some(ref qs) = self.queued_lightning_storm {
            qs.owner.hash(hasher);
            qs.target_rx.hash(hasher);
            qs.target_ry.hash(hasher);
        }
    }

    /// Hash all entity components in stable-entity-ID order.
    /// BTreeMap iterates in key order (= stable_id), so no manual sort needed.
    fn hash_entities(&self, hasher: &mut impl Hasher) {
        for entity in self.substrate.entities.values() {
            entity.stable_id.hash(hasher);
            entity.occupancy_enter_order.hash(hasher);
            entity.position.rx.hash(hasher);
            entity.position.ry.hash(hasher);
            entity.position.z.hash(hasher);
            entity.position.sub_x.hash(hasher);
            entity.position.sub_y.hash(hasher);
            entity.facing.hash(hasher);
            entity.facing_target.hash(hasher);
            entity.owner.hash(hasher);
            entity.health.current.hash(hasher);
            entity.health.max.hash(hasher);
            entity.type_ref.hash(hasher);
            (entity.category as u8).hash(hasher);
            entity.foundation.hash(hasher);
            entity.regular_crusher.hash(hasher);
            entity.drive_accelerates.hash(hasher);
            entity.building_damage_state_active.hash(hasher);
            entity.vision_range.hash(hasher);

            if let Some(ref movement) = entity.movement_target {
                1u8.hash(hasher);
                movement.next_index.hash(hasher);
                movement.speed.hash(hasher);
                movement.movement_delay.hash(hasher);
                movement.blocked_delay.hash(hasher);
                movement.path_blocked.hash(hasher);
                movement.path_stuck_counter.hash(hasher);
                movement.path.hash(hasher);
                movement.path_layers.hash(hasher);
            } else {
                0u8.hash(hasher);
            }

            entity.navigation.hash(hasher);

            if let Some(ref drive_track) = entity.drive_track {
                1u8.hash(hasher);
                hash_drive_track_state(drive_track, hasher);
            } else {
                0u8.hash(hasher);
            }

            entity.drive_locomotion.hash(hasher);

            if let Some(ref forced) = entity.forced_drive_track {
                1u8.hash(hasher);
                forced.turn_track_index.hash(hasher);
                forced.speed.hash(hasher);
                hash_drive_track_state(&forced.track, hasher);
            } else {
                0u8.hash(hasher);
            }

            if let Some(ref loco) = entity.locomotor {
                1u8.hash(hasher);
                (loco.kind as u8).hash(hasher);
                (loco.layer as u8).hash(hasher);
                (loco.phase as u8).hash(hasher);
            } else {
                0u8.hash(hasher);
            }

            if let Some(ref bridge) = entity.bridge_occupancy {
                1u8.hash(hasher);
                bridge.deck_level.hash(hasher);
            } else {
                0u8.hash(hasher);
            }
            entity.on_bridge.hash(hasher);
            entity.low_bridge_tube_state.hash(hasher);

            if let Some(ref inv) = entity.invulnerability {
                1u8.hash(hasher);
                inv.start_frame.hash(hasher);
                inv.duration_frames.hash(hasher);
                let kind_byte: u8 = match inv.kind {
                    crate::sim::superweapon::invulnerability::InvulnKind::IronCurtain => 0,
                    crate::sim::superweapon::invulnerability::InvulnKind::ForceShield => 1,
                };
                kind_byte.hash(hasher);
            } else {
                0u8.hash(hasher);
            }

            if let Some(ref attack) = entity.attack_target {
                1u8.hash(hasher);
                attack.cooldown_ticks.hash(hasher);
                attack.target.hash(hasher);
                attack.burst_remaining.hash(hasher);
                attack.burst_delay_ticks.hash(hasher);
                attack.pending_infantry_fire.hash(hasher);
            } else {
                0u8.hash(hasher);
            }

            // Slot-indexed fold: capacity + each slot's Option (null holes and
            // pad positions are hash-relevant). Replaces the old len + ordered-id
            // fold — an intended one-time re-baseline at this behavior boundary.
            entity.radio_contacts.hash_fold(hasher);
            // Dock-entered flag (+0x418 analogue). Intended one-time re-baseline
            // at this behavior boundary alongside the slot-folded contacts.
            match entity.dock_entered_with {
                Some(sid) => {
                    1u8.hash(hasher);
                    sid.hash(hasher);
                }
                None => 0u8.hash(hasher),
            }
            entity.rally_target.hash(hasher);
            entity.capture_target.hash(hasher);
            entity.c4_plant.hash(hasher);
            entity.pending_c4_detonation.hash(hasher);
            entity.bunker_occupant.hash(hasher);
            // Reciprocal link + install machine are authoritative lifecycle state.
            entity.bunker_link.hash(hasher);
            entity.bunker_runtime.hash(hasher);
            if let Some(gate) = entity.building_gate {
                1u8.hash(hasher);
                gate.mission_18_active.hash(hasher);
                (gate.phase as u8).hash(hasher);
                (gate.mission_state as u8).hash(hasher);
                // Same u32 values, same order as the old (last_frame, ticks_remaining)
                // pairs — the MissionTimer regrouping leaves the hash pre-image identical.
                gate.transition_timer.duration.hash(hasher);
                gate.transition_total_ticks.hash(hasher);
                gate.transition_timer.start_frame.hash(hasher);
                gate.hold_timer.duration.hash(hasher);
                gate.hold_timer.start_frame.hash(hasher);
            } else {
                0u8.hash(hasher);
            }

            match entity.deploy_state {
                None => 0u8.hash(hasher),
                Some(crate::sim::deploy::DeployPhase::Deploying { ticks_remaining }) => {
                    1u8.hash(hasher);
                    ticks_remaining.hash(hasher);
                }
                Some(crate::sim::deploy::DeployPhase::Deployed) => {
                    2u8.hash(hasher);
                }
                Some(crate::sim::deploy::DeployPhase::Undeploying { ticks_remaining }) => {
                    3u8.hash(hasher);
                    ticks_remaining.hash(hasher);
                }
            }

            if let Some(infantry) = entity.infantry {
                1u8.hash(hasher);
                infantry.fear_level.hash(hasher);
                infantry.is_prone.hash(hasher);
            } else {
                0u8.hash(hasher);
            }

            if let Some(ref miner) = entity.miner {
                1u8.hash(hasher);
                (miner.state as u8).hash(hasher);
                (miner.kind as u8).hash(hasher);
                (miner.cargo.len() as u16).hash(hasher);
                for bale in &miner.cargo {
                    (bale.resource_type as u8).hash(hasher);
                    bale.value.hash(hasher);
                }
                miner.home_refinery.hash(hasher);
                miner.reserved_refinery.hash(hasher);
                miner.target_ore_cell.hash(hasher);
                // harvest_timer is now a MissionTimer (start_frame + duration)
                // — intended one-time re-baseline. unload_timer was deleted.
                miner.harvest_timer.hash(hasher);
                miner.forced_return.hash(hasher);
                miner.dock_queued.hash(hasher);
                miner.dock_phase.hash(hasher);
                miner.dock_pivot_facing.hash(hasher);
            } else {
                0u8.hash(hasher);
            }

            // Passenger/transport state.
            match &entity.passenger_role {
                crate::sim::passenger::PassengerRole::None => {
                    0u8.hash(hasher);
                }
                crate::sim::passenger::PassengerRole::Transport { cargo } => {
                    1u8.hash(hasher);
                    cargo.capacity.hash(hasher);
                    (cargo.passengers.len() as u32).hash(hasher);
                    for &pid in &cargo.passengers {
                        pid.hash(hasher);
                    }
                    cargo.total_size.hash(hasher);
                }
                crate::sim::passenger::PassengerRole::Boarding {
                    target_transport_id,
                    phase,
                } => {
                    2u8.hash(hasher);
                    target_transport_id.hash(hasher);
                    (*phase as u8).hash(hasher);
                }
                crate::sim::passenger::PassengerRole::Inside { transport_id } => {
                    3u8.hash(hasher);
                    transport_id.hash(hasher);
                }
            }
            entity.weapon_override.hash(hasher);
            // Homing missile flight state. `HomingState` has a manual `Hash`
            // impl that excludes the render-only `pitch: f32` field — see
            // sim::movement::homing_movement.
            if let Some(ref h) = entity.homing_state {
                1u8.hash(hasher);
                h.hash(hasher);
            } else {
                0u8.hash(hasher);
            }
            // Barrel facing — Hash-derived, all primitive fields contribute.
            if let Some(ref barrel) = entity.barrel_facing {
                1u8.hash(hasher);
                barrel.hash(hasher);
            } else {
                0u8.hash(hasher);
            }

            // Body rocking + slope-transition state. I16F16 doesn't implement
            // Hash directly; .to_bits() gives the underlying i32.
            if let Some(ref r) = entity.rocking {
                1u8.hash(hasher);
                r.angle_sideways.to_bits().hash(hasher);
                r.angle_forwards.to_bits().hash(hasher);
                r.vel_sideways.to_bits().hash(hasher);
                r.vel_forwards.to_bits().hash(hasher);
                r.is_ship_rocking.hash(hasher);
                r.prev_slope.hash(hasher);
                r.curr_slope.hash(hasher);
                r.transition_ticks_remaining.hash(hasher);
            } else {
                0u8.hash(hasher);
            }

            // Mission substrate — folded as of Slice 8 (MissionCom is now
            // canonical hashed state, not an unhashed shadow).
            hash_mission_com(&entity.mission, hasher);
        }
    }
}

#[cfg(test)]
mod rally_hash_tests {
    use super::Simulation;
    use crate::sim::components::{DriveCoord, DriveLocomotionRuntime};
    use crate::sim::game_entity::GameEntity;

    #[test]
    fn entity_rally_target_changes_state_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        sim_a
            .substrate.entities
            .insert(GameEntity::test_default(1, "GAWEAP", "Americans", 10, 10));
        sim_b
            .substrate.entities
            .insert(GameEntity::test_default(1, "GAWEAP", "Americans", 10, 10));

        sim_b.substrate.entities.get_mut(1).unwrap().rally_target = Some((30, 31));

        assert_ne!(sim_a.state_hash(), sim_b.state_hash());
    }

    #[test]
    fn building_damage_state_changes_state_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let mut entity_a = GameEntity::test_default(1, "GAPOWR", "Americans", 10, 10);
        let mut entity_b = entity_a.clone();
        entity_a.category = crate::map::entities::EntityCategory::Structure;
        entity_b.category = crate::map::entities::EntityCategory::Structure;
        entity_b.building_damage_state_active = true;
        sim_a.substrate.entities.insert(entity_a);
        sim_b.substrate.entities.insert(entity_b);

        assert_ne!(sim_a.state_hash(), sim_b.state_hash());
    }

    #[test]
    fn drive_locomotion_state_changes_state_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let entity_a = GameEntity::test_default(1, "AMCV", "Americans", 10, 10);
        let mut entity_b = entity_a.clone();
        let mut drive = DriveLocomotionRuntime::default();
        drive.destination = Some(DriveCoord::cell(45, 40, 0));
        drive.path.directions = vec![2, 2, 2, 2, 2];
        drive.residual_budget = 3;
        entity_b.drive_locomotion = Some(drive);
        sim_a.substrate.entities.insert(entity_a);
        sim_b.substrate.entities.insert(entity_b);

        assert_ne!(sim_a.state_hash(), sim_b.state_hash());
    }

    #[test]
    fn drive_accelerates_changes_state_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let entity_a = GameEntity::test_default(1, "GTNK", "Americans", 10, 10);
        let mut entity_b = entity_a.clone();
        entity_b.drive_accelerates = false;
        sim_a.substrate.entities.insert(entity_a);
        sim_b.substrate.entities.insert(entity_b);

        assert_ne!(sim_a.state_hash(), sim_b.state_hash());
    }
}

#[cfg(test)]
mod particle_hash_tests {
    use super::Simulation;
    use crate::rules::particle_system_type::ParticleSystemTypeId;
    use crate::sim::particles::ParticleSystem;
    use crate::util::fixed_math::SimFixed;
    use glam::IVec3;

    fn fake_system(coords: IVec3) -> ParticleSystem {
        ParticleSystem {
            stable_id: 0,
            type_id: ParticleSystemTypeId(0),
            coords,
            offset: IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(0),
            lifetime: -1,
            spark_spawn_frames: 0,
            facing: 0x1D,
            marked_for_deletion: false,
            directionless: false,
            attached_entity: None,
            owner_entity: None,
            target_coords: IVec3::ZERO,
            owner_house: None,
            done_spawning: false,
        }
    }

    #[test]
    fn empty_particle_store_hashes_consistently() {
        let a = Simulation::new();
        let b = Simulation::new();
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn particle_state_changes_hash() {
        let mut sim = Simulation::new();
        let h1 = sim.state_hash();
        sim.particle_systems
            .insert(fake_system(IVec3::new(100, 0, 0)));
        let h2 = sim.state_hash();
        assert_ne!(h1, h2);
    }

    #[test]
    fn state_advance_counter_changes_hash() {
        use crate::rules::particle_type::ParticleTypeId;
        use crate::sim::particles::Particle;

        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let mut sys_a = fake_system(IVec3::ZERO);
        let mut sys_b = fake_system(IVec3::ZERO);
        let make_p = |counter: u8| Particle {
            type_id: ParticleTypeId(0),
            coords: IVec3::ZERO,
            previous_coords: IVec3::ZERO,
            origin: IVec3::ZERO,
            direction: [SimFixed::from_num(0); 3],
            velocity: SimFixed::from_num(0),
            lifetime_remaining: 100,
            damage_counter: 0,
            state_ai_advance: 4,
            animation_state: 0,
            translucency: 0,
            hit_ground: false,
            marked_for_deletion: false,
            drift_x: 0,
            drift_y: 0,
            drift_z: 0,
            current_color: [0; 3],
            color_index: 0,
            color_accumulator: SimFixed::from_num(0),
            prev_delta: [SimFixed::from_num(0); 3],
            state_advance_counter: counter,
        };
        sys_a.particles.push(make_p(0));
        sys_b.particles.push(make_p(3));
        sim_a.particle_systems.insert(sys_a);
        sim_b.particle_systems.insert(sys_b);
        assert_ne!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "state_advance_counter must affect state hash"
        );
    }

    #[test]
    fn terrain_spawners_included_in_state_hash() {
        use crate::sim::terrain_spawn::TerrainSpawnerState;

        let mut sim_a = Simulation::new();
        let sim_b = Simulation::new();
        let type_ref = sim_a.interner.intern("TIBTRE01");
        sim_a
            .production
            .terrain_spawners
            .insert((10, 10), TerrainSpawnerState::new(type_ref, 3000, 3, 22));

        assert_ne!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "terrain_spawners must affect state hash",
        );
    }

    #[test]
    fn terrain_spawner_active_fields_change_state_hash() {
        use crate::sim::terrain_spawn::{TerrainSpawnerPhase, TerrainSpawnerState};

        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let type_ref = sim_a.interner.intern("TIBTRE01");
        let state = TerrainSpawnerState::new(type_ref, 3000, 3, 22);
        sim_a
            .production
            .terrain_spawners
            .insert((10, 10), state.clone());
        sim_b.production.terrain_spawners.insert((10, 10), state);
        assert_eq!(sim_a.state_hash(), sim_b.state_hash());

        let spawner_b = sim_b
            .production
            .terrain_spawners
            .get_mut(&(10, 10))
            .unwrap();
        spawner_b.phase = TerrainSpawnerPhase::Active {
            current_frame: 1,
            ticks_until_next_frame: 2,
        };
        assert_ne!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "all terrain spawner state fields must affect state hash",
        );
    }
}

#[cfg(test)]
mod tube_movement_hash_tests {
    use super::Simulation;
    use crate::map::tube_facts::TubeId;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::tube_movement::{LowBridgeTubeMovementState, LowBridgeTubePhase};

    #[test]
    fn active_low_bridge_tube_state_changes_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let owner = sim_a.interner.intern("Allies");
        let type_ref = sim_a.interner.intern("MTNK");
        let mut entity_a = GameEntity::new(
            1,
            0,
            0,
            0,
            0,
            owner,
            Health {
                current: 100,
                max: 100,
            },
            type_ref,
            crate::map::entities::EntityCategory::Unit,
            0,
            5,
            true,
        );
        let entity_b = entity_a.clone();
        entity_a.low_bridge_tube_state = Some(LowBridgeTubeMovementState {
            tube_id: TubeId(3),
            cursor: 1,
            entry: (0, 0),
            exit: (4, 0),
            phase: LowBridgeTubePhase::Traversing,
        });
        sim_a.substrate.entities.insert(entity_a);
        sim_b.substrate.entities.insert(entity_b);

        assert_ne!(sim_a.state_hash(), sim_b.state_hash());
    }
}

#[cfg(test)]
mod radio_contact_hash_tests {
    use super::Simulation;
    use crate::map::entities::EntityCategory;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;

    fn vehicle_entity(sim: &mut Simulation, id: u64) -> GameEntity {
        GameEntity::new(
            id,
            10,
            10,
            0,
            0,
            sim.interner.intern("Americans"),
            Health {
                current: 100,
                max: 100,
            },
            sim.interner.intern("MTNK"),
            EntityCategory::Unit,
            0,
            5,
            true,
        )
    }

    #[test]
    fn live_radio_contacts_change_state_hash_per_mover() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let mut contacted = vehicle_entity(&mut sim_a, 1);
        let unrelated = vehicle_entity(&mut sim_a, 2);
        let contacted_b = vehicle_entity(&mut sim_b, 1);
        let unrelated_b = vehicle_entity(&mut sim_b, 2);

        contacted.mark_live_contact_with(100);
        sim_a.substrate.entities.insert(contacted);
        sim_a.substrate.entities.insert(unrelated);
        sim_b.substrate.entities.insert(contacted_b);
        sim_b.substrate.entities.insert(unrelated_b);

        assert_ne!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "per-mover live contacts must affect deterministic state hash",
        );
        assert!(!sim_a.substrate.entities.get(2).unwrap().has_live_contact_with(100));
    }

    #[test]
    fn despawn_contact_cleanup_hash_matches_never_contacted_state() {
        let mut with_stale_contact = Simulation::new();
        let mut never_contacted = Simulation::new();

        let mut removed = vehicle_entity(&mut with_stale_contact, 1);
        let mut survivor = vehicle_entity(&mut with_stale_contact, 2);
        removed.mark_live_contact_with(2);
        survivor.mark_live_contact_with(1);
        with_stale_contact.substrate.entities.insert(removed);
        with_stale_contact.substrate.entities.insert(survivor);

        let removed_b = vehicle_entity(&mut never_contacted, 1);
        let survivor_b = vehicle_entity(&mut never_contacted, 2);
        never_contacted.substrate.entities.insert(removed_b);
        never_contacted.substrate.entities.insert(survivor_b);

        with_stale_contact.despawn_entity(1);
        never_contacted.despawn_entity(1);

        assert_eq!(
            with_stale_contact.state_hash(),
            never_contacted.state_hash(),
            "cleanup should leave the same hash as a sim that never carried the stale contact",
        );
    }
}

#[cfg(test)]
mod infantry_hash_tests {
    use super::Simulation;
    use crate::map::entities::EntityCategory;
    use crate::sim::animation::SequenceKind;
    use crate::sim::combat::{AttackTarget, PendingInfantryFire};
    use crate::sim::components::Health;
    use crate::sim::game_entity::{GameEntity, InfantryRuntime};

    fn infantry_entity(sim: &mut Simulation) -> GameEntity {
        GameEntity::new(
            1,
            0,
            0,
            0,
            0,
            sim.interner.intern("Allies"),
            Health {
                current: 100,
                max: 100,
            },
            sim.interner.intern("E1"),
            EntityCategory::Infantry,
            0,
            5,
            false,
        )
    }

    #[test]
    fn infantry_fear_and_prone_change_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let mut a = infantry_entity(&mut sim_a);
        let b = infantry_entity(&mut sim_b);
        a.infantry = Some(InfantryRuntime {
            fear_level: 10,
            is_prone: false,
        });
        sim_a.substrate.entities.insert(a);
        sim_b.substrate.entities.insert(b);
        assert_ne!(sim_a.state_hash(), sim_b.state_hash());

        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let mut a = infantry_entity(&mut sim_a);
        let b = infantry_entity(&mut sim_b);
        a.infantry = Some(InfantryRuntime {
            fear_level: 0,
            is_prone: true,
        });
        sim_a.substrate.entities.insert(a);
        sim_b.substrate.entities.insert(b);
        assert_ne!(sim_a.state_hash(), sim_b.state_hash());
    }

    #[test]
    fn pending_infantry_fire_changes_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let mut a = infantry_entity(&mut sim_a);
        let mut b = infantry_entity(&mut sim_b);
        a.attack_target = Some(AttackTarget::new(99));
        b.attack_target = Some(AttackTarget::new(99));
        sim_a.substrate.entities.insert(a);
        sim_b.substrate.entities.insert(b);
        assert_eq!(sim_a.state_hash(), sim_b.state_hash());

        sim_a
            .substrate.entities
            .get_mut(1)
            .unwrap()
            .attack_target
            .as_mut()
            .unwrap()
            .pending_infantry_fire = Some(PendingInfantryFire {
            sequence: SequenceKind::Attack,
            fire_frame: 2,
        });
        assert_ne!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "pending infantry fire state must affect state hash"
        );
    }
}

#[cfg(test)]
mod smudge_hash_tests {
    use super::*;
    use crate::sim::smudge_grid::{SmudgeCell, SmudgeGrid};

    #[test]
    fn hash_changes_when_smudge_placed() {
        let mut sim = Simulation::new();
        sim.smudge_grid = Some(SmudgeGrid::new(8, 8));
        let h0 = sim.state_hash();
        if let Some(grid) = sim.smudge_grid.as_mut() {
            grid.test_force_set(
                2,
                3,
                SmudgeCell {
                    type_id: Some(0),
                    footprint_origin: Some((2, 3)),
                    frame_offset: 0,
                },
            );
        }
        let h1 = sim.state_hash();
        assert_ne!(h0, h1);
    }
}

#[cfg(test)]
mod bridge_overlay_hash_tests {
    use super::Simulation;
    use crate::sim::bridge_state::{
        Axis, BridgeCellRole, BridgeEndpointRecord, BridgeRecordKind, BridgeRuntimeCell,
        BridgeRuntimeState, DamageState,
    };

    fn make_bridge_state_with_overlay(byte: u8) -> BridgeRuntimeState {
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(
            2,
            2,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Anchor,
                anchor_span_id: None,
                overlay_byte: byte,
                damaged_variant: false,
                bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
            },
        );
        state
    }

    #[test]
    fn overlay_byte_difference_changes_state_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        sim_a.bridge_state = Some(make_bridge_state_with_overlay(0x18));
        sim_b.bridge_state = Some(make_bridge_state_with_overlay(0xD2));
        assert_ne!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "overlay_byte must contribute to state hash",
        );
    }

    #[test]
    fn identical_overlay_bytes_hash_equal() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        sim_a.bridge_state = Some(make_bridge_state_with_overlay(0x18));
        sim_b.bridge_state = Some(make_bridge_state_with_overlay(0x18));
        assert_eq!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "identical bridge states must hash equal",
        );
    }

    #[test]
    fn bridgehead_anchor_class_difference_changes_state_hash() {
        use crate::sim::bridge_state::BridgeheadAnchorClass;
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();

        let mut state_a = make_bridge_state_with_overlay(0x18);
        let state_b = make_bridge_state_with_overlay(0x18);
        if let Some(cell) = state_a.cell_mut(2, 2) {
            cell.bridgehead_anchor_class = BridgeheadAnchorClass::Damaged;
        }
        sim_a.bridge_state = Some(state_a);
        sim_b.bridge_state = Some(state_b);

        assert_ne!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "bridgehead_anchor_class must contribute to state hash",
        );
    }

    #[test]
    fn bridge_endpoint_record_kind_difference_changes_state_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();

        let mut state_a = make_bridge_state_with_overlay(0x18);
        let mut state_b = make_bridge_state_with_overlay(0x18);
        let mut record = BridgeEndpointRecord {
            endpoint_a: (1, 1),
            endpoint_b: (4, 1),
            group_id: 1,
            active: true,
            bridge_kind: BridgeRecordKind::High,
        };
        state_a.test_set_endpoint_records(vec![record]);
        record.bridge_kind = BridgeRecordKind::Low;
        state_b.test_set_endpoint_records(vec![record]);

        sim_a.bridge_state = Some(state_a);
        sim_b.bridge_state = Some(state_b);

        assert_ne!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "bridge endpoint record kind must contribute to state hash",
        );
    }
}

#[cfg(test)]
mod binary_frame_tests {
    use super::Simulation;
    use std::collections::BTreeMap;

    #[test]
    fn binary_frame_drift_free_at_22ms_ticks() {
        let mut sim = Simulation::new();
        let height_map = BTreeMap::new();
        // 45 ticks at 22ms = 990ms ≈ 14.85 binary frames; floor = 14.
        for _ in 0..45 {
            sim.advance_tick(&[], None, &height_map, None, None, 22);
        }
        assert_eq!(sim.total_sim_ms, 990);
        assert_eq!(sim.binary_frame, 14);
    }

    #[test]
    fn binary_frame_advances_each_66ms_block() {
        let mut sim = Simulation::new();
        let height_map = BTreeMap::new();
        // Three 67ms ticks should each advance binary_frame by 1
        // (67ms * 15 / 1000 = 1.005, floor = 1 per tick).
        sim.advance_tick(&[], None, &height_map, None, None, 67);
        assert_eq!(sim.binary_frame, 1);
        sim.advance_tick(&[], None, &height_map, None, None, 67);
        assert_eq!(sim.binary_frame, 2);
        sim.advance_tick(&[], None, &height_map, None, None, 67);
        assert_eq!(sim.binary_frame, 3);
    }

    #[test]
    fn binary_frame_changes_state_hash() {
        let mut sim_a = Simulation::new();
        let sim_b = Simulation::new();
        let height_map = BTreeMap::new();
        sim_a.advance_tick(&[], None, &height_map, None, None, 100);
        // sim_b stays at frame 0; sim_a is at (100*15/1000)=1.
        assert_ne!(sim_a.state_hash(), sim_b.state_hash());
    }
}

#[cfg(test)]
mod rocking_hash_tests {
    use super::Simulation;
    use crate::map::entities::EntityCategory;
    use crate::sim::components::{Health, RockingState};
    use crate::sim::game_entity::GameEntity;
    use crate::util::fixed_math::SimFixed;

    fn make_sim_with_one_vehicle() -> Simulation {
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let type_id = sim.interner.intern("HTNK");
        let id = sim.substrate.next_stable_entity_id;
        sim.substrate.next_stable_entity_id += 1;
        let e = GameEntity::new(
            id,
            10,
            10,
            0,
            0,
            owner,
            Health {
                current: 400,
                max: 400,
            },
            type_id,
            EntityCategory::Unit,
            0,
            5,
            true,
        );
        sim.substrate.entities.insert(e);
        sim
    }

    #[test]
    fn rocking_state_contributes_to_hash() {
        let a = make_sim_with_one_vehicle();
        let b = make_sim_with_one_vehicle();
        assert_eq!(a.state_hash(), b.state_hash());

        // Mutate only the rocking state of one — hashes must diverge.
        let mut a = a;
        let id = a.substrate.entities.values().next().unwrap().stable_id;
        a.substrate.entities.get_mut(id).unwrap().rocking = Some(RockingState {
            angle_sideways: SimFixed::lit("0.1"),
            ..Default::default()
        });
        assert_ne!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn rocking_velocity_contributes_to_hash() {
        let mut a = make_sim_with_one_vehicle();
        let mut b = make_sim_with_one_vehicle();
        let id_a = a.substrate.entities.values().next().unwrap().stable_id;
        let id_b = b.substrate.entities.values().next().unwrap().stable_id;
        a.substrate.entities.get_mut(id_a).unwrap().rocking = Some(RockingState {
            vel_sideways: SimFixed::lit("0.01"),
            ..Default::default()
        });
        b.substrate.entities.get_mut(id_b).unwrap().rocking = Some(RockingState {
            vel_sideways: SimFixed::lit("0.02"),
            ..Default::default()
        });
        assert_ne!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn rocking_none_vs_default_contributes_to_hash() {
        let mut a = make_sim_with_one_vehicle();
        let b = make_sim_with_one_vehicle();
        let id = a.substrate.entities.values().next().unwrap().stable_id;
        a.substrate.entities.get_mut(id).unwrap().rocking = Some(RockingState::default());
        // a has Some(default), b has None — hashes must diverge.
        assert_ne!(a.state_hash(), b.state_hash());
    }
}

#[cfg(test)]
mod c4_hash_tests {
    use super::Simulation;
    use crate::map::entities::EntityCategory;
    use crate::sim::components::{C4PlantState, Health, PendingC4Detonation};
    use crate::sim::game_entity::GameEntity;

    #[test]
    fn c4_state_changes_hash() {
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let type_id = sim.interner.intern("GHOST");
        let id = sim.substrate.next_stable_entity_id;
        sim.substrate.next_stable_entity_id += 1;
        let e = GameEntity::new(
            id,
            10,
            10,
            0,
            0,
            owner,
            Health {
                current: 125,
                max: 125,
            },
            type_id,
            EntityCategory::Infantry,
            0,
            5,
            false,
        );
        sim.substrate.entities.insert(e);
        let h_initial = sim.state_hash();

        // Mutate c4_plant — hash must change.
        sim.substrate.entities.get_mut(id).unwrap().c4_plant = Some(C4PlantState {
            target_building_id: 99,
        });
        let h_with_plant = sim.state_hash();
        assert_ne!(h_initial, h_with_plant, "c4_plant must affect state hash");

        // Mutate pending_c4_detonation — hash must change again.
        sim.substrate.entities.get_mut(id).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
            plant_start_tick: 100,
            attacker_id: 7,
        });
        let h_with_pending = sim.state_hash();
        assert_ne!(
            h_with_plant, h_with_pending,
            "pending_c4_detonation must affect state hash"
        );
    }
}

#[cfg(test)]
mod homing_state_hash_tests {
    use super::Simulation;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::homing_movement::{HomingPhase, HomingState};
    use crate::util::fixed_math::{SIM_ONE, SIM_ZERO, SimFixed};

    fn make_homing(yaw_bam: u16) -> HomingState {
        HomingState {
            phase: HomingPhase::Cruise,
            target_id: Some(42),
            last_known_rx: 25,
            last_known_ry: 5,
            yaw_bam,
            pitch_bam: 0x4000,
            speed: SimFixed::from_num(30),
            pos_x_cells: SimFixed::from_num(5),
            pos_y_cells: SimFixed::from_num(5),
            altitude: SimFixed::from_num(320),
            vz: SIM_ZERO,
            rot_ini: 60,
            missile_rot_var: SIM_ONE,
            floater: false,
            very_high: false,
            arm_ticks_remaining: 0,
            frame_counter: 0,
            stall_counter: 0,
            stall_ema: SIM_ZERO,
            last_distance_to_target: SIM_ZERO,
            pitch: 0.0,
        }
    }

    #[test]
    fn homing_state_presence_changes_hash() {
        let mut a = Simulation::new();
        let mut b = Simulation::new();
        let a_id = a
            .substrate.entities
            .insert(GameEntity::test_default(1, "AAHeatSeeker2", "Allied", 5, 5));
        b.substrate.entities
            .insert(GameEntity::test_default(1, "AAHeatSeeker2", "Allied", 5, 5));

        // Hashes match while both bullets lack homing_state.
        assert_eq!(a.state_hash(), b.state_hash());

        // Attaching homing_state to `a` only — hashes must diverge.
        a.substrate.entities.get_mut(a_id).unwrap().homing_state = Some(make_homing(0));
        assert_ne!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn homing_state_yaw_changes_hash() {
        let mut a = Simulation::new();
        let mut b = Simulation::new();
        let a_id = a
            .substrate.entities
            .insert(GameEntity::test_default(1, "AAHeatSeeker2", "Allied", 5, 5));
        let b_id = b
            .substrate.entities
            .insert(GameEntity::test_default(1, "AAHeatSeeker2", "Allied", 5, 5));
        a.substrate.entities.get_mut(a_id).unwrap().homing_state = Some(make_homing(0));
        b.substrate.entities.get_mut(b_id).unwrap().homing_state = Some(make_homing(0x4000));
        assert_ne!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn homing_state_pitch_excluded_from_hash() {
        // The manual Hash impl on HomingState skips the render-only `pitch`
        // field; mutating it must not change the state hash.
        let mut a = Simulation::new();
        let mut b = Simulation::new();
        let a_id = a
            .substrate.entities
            .insert(GameEntity::test_default(1, "AAHeatSeeker2", "Allied", 5, 5));
        let b_id = b
            .substrate.entities
            .insert(GameEntity::test_default(1, "AAHeatSeeker2", "Allied", 5, 5));
        a.substrate.entities.get_mut(a_id).unwrap().homing_state = Some(make_homing(0));
        let mut h = make_homing(0);
        h.pitch = 1.234;
        b.substrate.entities.get_mut(b_id).unwrap().homing_state = Some(h);
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "render-only pitch must not affect state hash"
        );
    }
}
