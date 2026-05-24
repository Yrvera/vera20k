//! Order-intent tick systems for the Simulation.
//!
//! Handles automatic target acquisition for attack-move and guard orders
//! (pre-combat), and resuming movement after combat ends (post-combat).
//!
//! Dependency rules: same as sim/ (depends on rules/, map/; never render/ui/audio/net).

use super::Simulation;
use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat;
use crate::sim::components::OrderIntent;
use crate::sim::intern::InternedId;
use crate::sim::movement;
use crate::sim::movement::air_movement;
use crate::sim::movement::bump_crush;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::pathfinding::PathGrid;
use crate::util::fixed_math::SimFixed;
use crate::util::fixed_math::ra2_speed_to_leptons_per_second;

/// Result of one `apply_c4_damage_to_building` call.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct C4DamageOutcome {
    /// HP reached 0; building marked dying this tick.
    pub killed_building: bool,
    /// The C4 hit a BridgeRepairHut, the hut survived, and the connected
    /// bridge collapsed. The app needs to rebuild PathGrid.
    pub bridge_state_changed: bool,
    /// The target's pending C4 marker should be cleared even though the
    /// building entity survived. Used by BridgeRepairHut dispatch.
    pub consumed_pending_marker: bool,
}

/// Result of `tick_c4_plants` across all per-tick plants + detonations.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct C4TickOutcome {
    pub destroyed_structure: bool,
    pub bridge_state_changed: bool,
}

impl Simulation {
    /// Pre-combat: entities with an OrderIntent but no current AttackTarget
    /// try to acquire a nearby enemy to engage.
    pub(crate) fn tick_order_intents_pre_combat(&mut self, rules: &RuleSet) {
        // Collect attacker candidates from EntityStore.
        let keys: Vec<u64> = self.entities.keys_sorted();
        let mut attacker_ids: Vec<u64> = Vec::new();
        for &id in &keys {
            if let Some(entity) = self.entities.get(id) {
                if entity.order_intent.is_some() && entity.attack_target.is_none() {
                    attacker_ids.push(id);
                }
            }
        }

        for attacker_id in attacker_ids {
            let Some(target_sid) = combat::acquire_best_target_for_entity(
                &self.entities,
                rules,
                &self.interner,
                attacker_id,
                Some(&self.fog),
                self.resolved_terrain.as_ref(),
            ) else {
                continue;
            };
            let _ = combat::issue_attack_command(
                &mut self.entities,
                attacker_id,
                target_sid,
                Some(rules),
                &self.interner,
            );
        }
    }

    /// Post-combat: entities with an OrderIntent but no active attack or movement
    /// resume their patrol/guard movement toward the original goal.
    pub(crate) fn tick_order_intents_post_combat(
        &mut self,
        path_grid: Option<&PathGrid>,
        rules: Option<&RuleSet>,
    ) {
        let Some(grid) = path_grid else { return };
        // Collect (stable_id, goal) for entities that need to resume movement.
        let keys: Vec<u64> = self.entities.keys_sorted();
        let mut resumes: Vec<(u64, u16, u16)> = Vec::new();
        for &id in &keys {
            if let Some(entity) = self.entities.get(id) {
                let intent = match entity.order_intent {
                    Some(ref i) => *i,
                    None => continue,
                };
                if entity.attack_target.is_some() || entity.movement_target.is_some() {
                    continue;
                }
                match intent {
                    OrderIntent::AttackMove { goal_rx, goal_ry }
                        if (entity.position.rx, entity.position.ry) != (goal_rx, goal_ry) =>
                    {
                        resumes.push((id, goal_rx, goal_ry));
                    }
                    OrderIntent::Guard {
                        anchor_rx,
                        anchor_ry,
                    } if (entity.position.rx, entity.position.ry) != (anchor_rx, anchor_ry) => {
                        resumes.push((id, anchor_rx, anchor_ry));
                    }
                    _ => {}
                }
            }
        }

        for (stable_id, goal_rx, goal_ry) in resumes {
            let (base_speed, loco_multiplier, is_air) = self
                .entities
                .get(stable_id)
                .map(|e| {
                    let bs: SimFixed = rules
                        .and_then(|r| r.object(self.interner.resolve(e.type_ref)))
                        .map(|obj| ra2_speed_to_leptons_per_second(obj.speed))
                        .unwrap_or(ra2_speed_to_leptons_per_second(4));
                    let lm: SimFixed = e
                        .locomotor
                        .as_ref()
                        .map(|l| l.speed_multiplier)
                        .unwrap_or(SimFixed::from_num(1));
                    let air: bool = e
                        .locomotor
                        .as_ref()
                        .is_some_and(|l| l.layer == MovementLayer::Air);
                    (bs, lm, air)
                })
                .unwrap_or((
                    ra2_speed_to_leptons_per_second(4),
                    SimFixed::from_num(1),
                    false,
                ));
            let speed: SimFixed = (base_speed * loco_multiplier).max(SimFixed::lit("25"));

            if is_air {
                let _ = air_movement::issue_air_move_command(
                    &mut self.entities,
                    stable_id,
                    (goal_rx, goal_ry),
                    speed,
                );
            } else {
                let _ = movement::issue_move_command_with_layered(
                    &mut self.entities,
                    grid,
                    stable_id,
                    (goal_rx, goal_ry),
                    speed,
                    false,
                    None,
                    None,
                    self.resolved_terrain.as_ref(),
                    None,
                    false, // mover_is_crusher
                );
            }
        }
    }

    /// Tick engineer capture orders: check if any engineer with a capture_target
    /// has arrived adjacent to its target building. If so, transfer ownership and
    /// consume the engineer.
    ///
    /// Engineers targeting `BridgeRepairHut=yes` buildings are skipped here —
    /// they are consumed earlier in the tick by `tick_bridge_repair_orders`.
    /// This skip is defense in depth in case ordering ever changes; the
    /// original game never captures CABHUTs.
    /// Returns true if any capture occurred (triggers atlas rebuild for new owner color).
    pub(crate) fn tick_capture_orders(&mut self, rules: &RuleSet) -> bool {
        let mut any_captured = false;
        // Snapshot engineers with active capture targets.
        let captures: Vec<(u64, u64, InternedId)> = self
            .entities
            .values()
            .filter(|e| e.capture_target.is_some() && !e.dying)
            .map(|e| (e.stable_id, e.capture_target.unwrap(), e.owner))
            .collect();

        for (engineer_id, building_id, engineer_owner) in captures {
            // Skip BridgeRepairHut targets — repair tick handles them.
            let target_bridge_hut = self
                .entities
                .get(building_id)
                .and_then(|b| {
                    rules
                        .object(self.interner.resolve(b.type_ref))
                        .map(|t| t.bridge_repair_hut)
                })
                .unwrap_or(false);
            if target_bridge_hut {
                continue;
            }

            // Check building still exists and is capturable.
            let building_ok = self
                .entities
                .get(building_id)
                .is_some_and(|b| b.category == EntityCategory::Structure && !b.dying);
            if !building_ok {
                // Target lost — clear capture order.
                if let Some(e) = self.entities.get_mut(engineer_id) {
                    e.capture_target = None;
                }
                continue;
            }

            // Distance check: adjacent = Chebyshev distance <= 1 cell.
            let (eng_rx, eng_ry) = self
                .entities
                .get(engineer_id)
                .map(|e| (e.position.rx, e.position.ry))
                .unwrap_or((0, 0));
            let (bld_rx, bld_ry) = self
                .entities
                .get(building_id)
                .map(|e| (e.position.rx, e.position.ry))
                .unwrap_or((0, 0));
            let dx = (eng_rx as i32 - bld_rx as i32).abs();
            let dy = (eng_ry as i32 - bld_ry as i32).abs();

            if dx <= 1 && dy <= 1 {
                // CAPTURE: transfer building ownership.
                let old_owner = self.entities.get(building_id).map(|b| b.owner);
                if let Some(b) = self.entities.get_mut(building_id) {
                    b.owner = engineer_owner;
                }
                // Update house owned counts for both old and new owner.
                // Resolve interned IDs to strings before &mut self calls.
                let engineer_owner_str = self.interner.resolve(engineer_owner).to_string();
                if let Some(old_owner_id) = old_owner {
                    let old_owner_str = self.interner.resolve(old_owner_id).to_string();
                    self.decrement_owned_count(&old_owner_str, EntityCategory::Structure);
                }
                self.increment_owned_count(&engineer_owner_str, EntityCategory::Structure);
                // Destroy engineer (consumed on capture).
                self.despawn_entity(engineer_id);
                any_captured = true;
            }
        }
        any_captured
    }

    /// Tick bridge-repair orders: any engineer with `capture_target` pointing
    /// at a `BridgeRepairHut=yes` building first enters the building footprint.
    /// Once the engineer's current cell resolves to that building, the
    /// PerCellProcess-style arrival branch triggers bridge repair on the cells
    /// in a 5x5 scan around the engineer's arrival cell.
    ///
    /// Flow:
    ///   1. Create a non-drawing `BridgeRepaired` radar event at the hut.
    ///   2. Emit `SimSoundEvent::BridgeRepaired` at the building's cell.
    ///   3. Run overlay-family bridge repair over the 5x5 scan around the arrival cell.
    ///   4. Despawn the engineer (consumed by repair).
    ///
    /// Returns `true` if any repair mutated bridge state (caller ORs into
    /// `TickResult.bridge_state_changed` so the app rebuilds PathGrid).
    pub(crate) fn tick_bridge_repair_orders(&mut self, rules: &RuleSet) -> bool {
        use crate::sim::bridge_state::cells_in_5x5_scan;

        let mut any_repair = false;
        let keys = self.entities.keys_sorted();
        let mut key_idx = 0;

        while key_idx < keys.len() {
            let engineer_id = keys[key_idx];
            let Some((building_id, engineer_owner)) =
                self.entities.get(engineer_id).and_then(|e| {
                    if e.dying {
                        return None;
                    }
                    Some((e.capture_target?, e.owner))
                })
            else {
                key_idx += 1;
                continue;
            };

            // Resolve target type; only proceed for BridgeRepairHut=yes.
            let target_bridge_hut = self
                .entities
                .get(building_id)
                .and_then(|b| {
                    rules
                        .object(self.interner.resolve(b.type_ref))
                        .map(|t| t.bridge_repair_hut)
                })
                .unwrap_or(false);
            if !target_bridge_hut {
                key_idx += 1;
                continue;
            }

            // Target alive + still a Structure.
            let target_alive = self
                .entities
                .get(building_id)
                .is_some_and(|b| b.category == EntityCategory::Structure && !b.dying);
            if !target_alive {
                if let Some(e) = self.entities.get_mut(engineer_id) {
                    e.capture_target = None;
                }
                key_idx += 1;
                continue;
            }

            // Adjacency only issues the scripted enter move; repair itself
            // waits until the engineer has arrived inside the building cell.
            let Some((erx, ery)) = self
                .entities
                .get(engineer_id)
                .map(|e| (e.position.rx, e.position.ry))
            else {
                key_idx += 1;
                continue;
            };
            let engineer_cell = (erx, ery);
            let Some(target_footprint) = self.building_entry_target_footprint(building_id, rules)
            else {
                key_idx += 1;
                continue;
            };
            if !target_footprint.contains(&engineer_cell) {
                if self.adjacent_to_target_footprint(engineer_cell, &target_footprint)
                    && !self.infantry_has_active_movement(engineer_id)
                {
                    self.issue_building_enter_target_cell(
                        engineer_id,
                        engineer_cell,
                        &target_footprint,
                        rules,
                    );
                }
                key_idx += 1;
                continue;
            }

            let Some((brx, bry)) = self
                .entities
                .get(building_id)
                .map(|b| (b.position.rx, b.position.ry))
            else {
                key_idx += 1;
                continue;
            };

            // ---- Trigger fires this tick ----

            // Step A0: create the non-drawing BridgeRepaired radar event before
            // bridge mutation. Its dedup result gates EVA in the app layer.
            let eva_allowed = self.radar_events.push(
                crate::sim::radar::RadarEventType::BridgeRepaired,
                brx,
                bry,
                rules.radar_event_config.event_duration_ms,
            );

            // Step A: emit BridgeRepaired sound event at the BUILDING's cell.
            self.sound_events
                .push(crate::sim::world::SimSoundEvent::BridgeRepaired {
                    rx: brx,
                    ry: bry,
                    owner: engineer_owner,
                    eva_allowed,
                });

            // Step B: 5x5 scan from the engineer's arrival cell + repair dispatch.
            let scan: Vec<(u16, u16)> = cells_in_5x5_scan(engineer_cell).collect();
            let outcome = if let (Some(bs), Some(terrain)) =
                (self.bridge_state.as_mut(), self.resolved_terrain.as_ref())
            {
                bs.repair_bridge_from_engineer_scan(&scan, &mut self.rng, terrain)
            } else {
                crate::sim::bridge_state::RepairOutcome::default()
            };

            if outcome.zones_dirty || outcome.repaired_cells > 0 {
                any_repair = true;
            }

            // Step C: terrain/radar dirty propagation. The walker only emits
            // cells for destroyed-anchor restoration, matching gamemd's
            // `MarkTerrainDirty` gate.
            self.mark_radar_terrain_dirty_cells(outcome.radar_cells.iter().copied());

            // Step D: engineer consumed.
            self.despawn_entity(engineer_id);
            // gamemd iterates a live object vector. Removing the current
            // engineer compacts the next object into this slot; the scheduler
            // then advances, so that immediate successor waits until later.
            key_idx += 2;
        }

        any_repair
    }

    /// Tick C4 plant orders.
    ///
    /// Phase 1 (walk-up): for each entity with `c4_plant`, check if it's
    /// Chebyshev-≤-1 adjacent to the target building's anchor cell; if so
    /// and the building doesn't already have a `pending_c4_detonation`
    /// claimed by another attacker, claim it. Second attackers on an
    /// already-claimed target hover (no-op) — matches gamemd's `+0x6df`
    /// marker check.
    ///
    /// Phase 2 (detonation): for each building with `pending_c4_detonation`,
    /// if the elapsed tick count >= `rules.c4_delay_ticks`, apply C4Warhead
    /// damage equal to the building's current HP. For normal buildings the
    /// pending state is not cleared if damage is nullified by IronCurtain, so
    /// it fires again next tick. When the building dies, the entity despawns
    /// and the pending state goes with it. BridgeRepairHut dispatch clears
    /// the pending marker after the bridge path runs because the hut survives.
    ///
    /// Returns the per-tick C4 outcome: `destroyed_structure` is true if any
    /// building died, and `bridge_state_changed` is true if any C4 detonation
    /// on a `BridgeRepairHut` collapsed a bridge.
    pub(crate) fn tick_c4_plants(&mut self, rules: &RuleSet) -> C4TickOutcome {
        use crate::sim::components::PendingC4Detonation;
        let mut destroyed_structure = false;
        let mut bridge_state_changed = false;

        // ---- Phase 1: walk-up + plant claim ----
        // Snapshot attackers with c4_plant. Deterministic sorted order via
        // keys_sorted then look up c4_plant.
        let mut walkup: Vec<(u64, u64)> = Vec::new();
        for sid in self.entities.keys_sorted() {
            if let Some(e) = self.entities.get(sid) {
                if let Some(plant) = e.c4_plant {
                    if !e.dying {
                        walkup.push((sid, plant.target_building_id));
                    }
                }
            }
        }

        for (attacker_id, target_id) in walkup {
            // Target gone or dying? Clear c4_plant.
            let target_alive = self
                .entities
                .get(target_id)
                .is_some_and(|b| b.category == EntityCategory::Structure && !b.dying);
            if !target_alive {
                if let Some(e) = self.entities.get_mut(attacker_id) {
                    e.c4_plant = None;
                }
                continue;
            }

            // gamemd claims only when the infantry's current cell resolves
            // to the target building. Normal pathing stops at the blocked
            // footprint boundary, then we issue the one-cell enter move below.
            let attacker_cell = self
                .entities
                .get(attacker_id)
                .map(|e| (e.position.rx, e.position.ry));
            let target_footprint = self.building_entry_target_footprint(target_id, rules);
            let (Some(attacker_cell), Some(target_footprint)) = (attacker_cell, target_footprint)
            else {
                continue;
            };

            // Already claimed by another attacker?
            let already_claimed = self
                .entities
                .get(target_id)
                .is_some_and(|b| b.pending_c4_detonation.is_some());
            if already_claimed {
                // Second SEAL — hover, no-op. Matches gamemd's marker-set early-return.
                continue;
            }

            if !target_footprint.contains(&attacker_cell) {
                if self.adjacent_to_target_footprint(attacker_cell, &target_footprint)
                    && !self.infantry_has_active_movement(attacker_id)
                {
                    self.issue_building_enter_target_cell(
                        attacker_id,
                        attacker_cell,
                        &target_footprint,
                        rules,
                    );
                }
                continue; // walk-up or enter-cell movement still in progress
            }

            // Claim the plant.
            if let Some(b) = self.entities.get_mut(target_id) {
                b.pending_c4_detonation = Some(PendingC4Detonation {
                    plant_start_tick: self.tick,
                    attacker_id,
                });
            }

            // Drive the plant animation (FireUp = Attack sequence).
            if let Some(a) = self.entities.get_mut(attacker_id) {
                a.movement_target = None;
                if let Some(ref mut anim) = a.animation {
                    anim.switch_to(crate::sim::animation::SequenceKind::Attack);
                }
            }

            // SealPlaceBomb spatial sound. App-side dispatcher resolves to
            // `[SealPlaceBomb]` from soundmd.ini.
            if let Some(a) = self.entities.get(attacker_id) {
                self.sound_events
                    .push(crate::sim::world::SimSoundEvent::C4Planted {
                        rx: a.position.rx,
                        ry: a.position.ry,
                    });
            }
        }

        // ---- Phase 2: detonation ----
        let mut det_keys: Vec<u64> = Vec::new();
        for sid in self.entities.keys_sorted() {
            if let Some(e) = self.entities.get(sid) {
                if e.pending_c4_detonation.is_some() && !e.dying {
                    det_keys.push(sid);
                }
            }
        }
        // Early-out skips the rules.c4_warhead_id() lookup, which panics if
        // resolve_bridge_warheads hasn't been called. Pre-feature tests don't
        // call it; guarding here keeps them passing.
        if det_keys.is_empty() {
            return C4TickOutcome {
                destroyed_structure,
                bridge_state_changed,
            };
        }

        let c4_warhead_id = rules.c4_warhead_id();
        let delay = rules.c4_delay_ticks as u64;

        for building_id in det_keys {
            let pending = self
                .entities
                .get(building_id)
                .and_then(|e| e.pending_c4_detonation);
            let Some(pending) = pending else { continue };

            if self.tick.saturating_sub(pending.plant_start_tick) < delay {
                continue;
            }

            // Timer elapsed — apply C4Warhead damage. Damage value = current_hp
            // for guaranteed one-shot kill (matches gamemd's
            // `&iStack_28 = this->Health` argument to TakeDamage).
            // Normal-building pending state is only cleared by despawn;
            // BridgeRepairHut returns consumed_pending_marker below.
            let dmg: i32 = self
                .entities
                .get(building_id)
                .map(|b| b.health.current as i32)
                .unwrap_or(0);
            if dmg <= 0 {
                continue;
            }

            // Resolve kill-credit. Attacker may have despawned — fall back to None.
            let attacker_for_credit: Option<u64> = self
                .entities
                .get(pending.attacker_id)
                .map(|_| pending.attacker_id);

            let outcome = self.apply_c4_damage_to_building(
                building_id,
                dmg as u16,
                c4_warhead_id,
                attacker_for_credit,
                rules,
            );
            bridge_state_changed |= outcome.bridge_state_changed;
            if outcome.killed_building {
                destroyed_structure = true;
                // pending_c4_detonation goes away with the entity via despawn path.
                // Trigger scatter walk-away for any attacker on this cell with
                // c4_plant pointing at this building. Matches gamemd
                // Mission_Enter post-detonation block.
                self.queue_c4_post_detonation_scatter(building_id);
            } else if outcome.consumed_pending_marker {
                if let Some(building) = self.entities.get_mut(building_id) {
                    building.pending_c4_detonation = None;
                }
                if let Some(attacker) = self.entities.get_mut(pending.attacker_id) {
                    if attacker
                        .c4_plant
                        .is_some_and(|plant| plant.target_building_id == building_id)
                    {
                        attacker.c4_plant = None;
                    }
                }
            }
        }

        C4TickOutcome {
            destroyed_structure,
            bridge_state_changed,
        }
    }

    fn building_entry_target_footprint(
        &self,
        target_id: u64,
        rules: &RuleSet,
    ) -> Option<Vec<(u16, u16)>> {
        let target = self.entities.get(target_id)?;
        let obj = rules.object(self.interner.resolve(target.type_ref))?;
        // Infantry building-entry resolves through normal building cell lookup.
        // AddOccupy/RemoveOccupy only affect hidden occupancy counters.
        Some(c4_base_foundation_cells(
            target.position.rx,
            target.position.ry,
            obj.foundation.as_str(),
        ))
    }

    fn adjacent_to_target_footprint(
        &self,
        attacker_cell: (u16, u16),
        target_footprint: &[(u16, u16)],
    ) -> bool {
        target_footprint.iter().any(|&(trx, try_)| {
            let dx = (attacker_cell.0 as i32 - trx as i32).abs();
            let dy = (attacker_cell.1 as i32 - try_ as i32).abs();
            dx <= 1 && dy <= 1
        })
    }

    fn infantry_has_active_movement(&self, attacker_id: u64) -> bool {
        self.entities
            .get(attacker_id)
            .is_some_and(|attacker| attacker.movement_target.is_some())
    }

    fn issue_building_enter_target_cell(
        &mut self,
        attacker_id: u64,
        attacker_cell: (u16, u16),
        target_footprint: &[(u16, u16)],
        rules: &RuleSet,
    ) {
        let Some(entry_cell) = target_footprint.iter().copied().min_by_key(|&(rx, ry)| {
            let dx = (attacker_cell.0 as i32 - rx as i32).abs();
            let dy = (attacker_cell.1 as i32 - ry as i32).abs();
            (dx.max(dy), dx + dy, rx, ry)
        }) else {
            return;
        };

        let speed = self
            .resolve_move_info(attacker_id, Some(rules))
            .as_ref()
            .map(|info| info.speed)
            .unwrap_or(ra2_speed_to_leptons_per_second(4));
        if movement::issue_direct_move(&mut self.entities, attacker_id, entry_cell, speed) {
            if let Some(target) = self
                .entities
                .get_mut(attacker_id)
                .and_then(|attacker| attacker.movement_target.as_mut())
            {
                target.bypass_grid = true;
            }
        }
    }

    /// Post-detonation: any attacker that was on the destroyed building's
    /// cell with `c4_plant` targeting this building scatters one cell in a
    /// deterministic direction derived from the current tick. Matches gamemd
    /// `Mission_Enter` post-detonation block:
    /// `uVar13 = (tick >> 12 + 1) >> 1 & 7` → 1 of 8 directions via
    /// the direction-delta tables.
    ///
    /// Also clears each attacker's `c4_plant`.
    fn queue_c4_post_detonation_scatter(&mut self, dead_building_id: u64) {
        // 8 cardinal+ordinal directions in standard RA2 order:
        // N, NE, E, SE, S, SW, W, NW.
        const DIR_DELTAS: [(i16, i16); 8] = [
            (0, -1),  // N
            (1, -1),  // NE
            (1, 0),   // E
            (1, 1),   // SE
            (0, 1),   // S
            (-1, 1),  // SW
            (-1, 0),  // W
            (-1, -1), // NW
        ];
        // Mirror gamemd's bit-twiddle: `(tick >> 12 + 1) >> 1 & 7`.
        // C operator precedence: `>>` is left-to-right at same level, so
        // this evaluates as `(((tick >> 12) + 1) >> 1) & 7`.
        let dir: usize = ((((self.tick >> 12) + 1) >> 1) & 7) as usize;
        let (dx, dy) = DIR_DELTAS[dir];

        let bld_cell = self
            .entities
            .get(dead_building_id)
            .map(|b| (b.position.rx, b.position.ry));
        let Some((brx, bry)) = bld_cell else { return };

        // Collect attackers on this cell with c4_plant on this building.
        let mut scatterers: Vec<u64> = Vec::new();
        for sid in self.entities.keys_sorted() {
            if let Some(e) = self.entities.get(sid) {
                if !e.dying
                    && e.position.rx == brx
                    && e.position.ry == bry
                    && e.c4_plant
                        .map_or(false, |p| p.target_building_id == dead_building_id)
                {
                    scatterers.push(sid);
                }
            }
        }

        for sid in scatterers {
            let target_rx = (brx as i16 + dx).max(0) as u16;
            let target_ry = (bry as i16 + dy).max(0) as u16;
            if let Some(e) = self.entities.get_mut(sid) {
                e.c4_plant = None;
            }
            // Queue a Move command for the next tick. Simpler than
            // reimplementing the pathfind call; 1-tick delay is below the
            // human-observable threshold.
            if let Some(owner) = self.entities.get(sid).map(|e| e.owner) {
                self.pending_commands
                    .push(crate::sim::command::CommandEnvelope::new(
                        owner,
                        self.tick + 1,
                        crate::sim::command::Command::Move {
                            entity_id: sid,
                            target_rx,
                            target_ry,
                            queue: false,
                            group_id: None,
                        },
                    ));
            }
        }
    }

    /// Apply one C4Warhead damage instance to a building entity. Returns
    /// `C4DamageOutcome` reporting whether the building died and whether a
    /// connected bridge collapsed (for `BridgeRepairHut` targets). Non-hut
    /// targets honor IronCurtain via the standard invulnerability check.
    /// Used by `tick_c4_plants` Phase 2.
    fn apply_c4_damage_to_building(
        &mut self,
        building_id: u64,
        damage: u16,
        warhead_id: crate::sim::intern::InternedId,
        attacker_id: Option<u64>,
        rules: &RuleSet,
    ) -> C4DamageOutcome {
        // BridgeRepairHut target: reroute the explosion into the bridge
        // collapse cascade and leave the hut at full HP. The hut never
        // takes C4 / demo-truck damage — destruction is the linked bridge
        // segment's, not the hut's. Also the right entry point for a
        // future demo-truck damage path.
        let target_bridge_hut = self
            .entities
            .get(building_id)
            .and_then(|b| {
                rules
                    .object(self.interner.resolve(b.type_ref))
                    .map(|t| t.bridge_repair_hut)
            })
            .unwrap_or(false);
        if target_bridge_hut {
            let bld_center = self
                .entities
                .get(building_id)
                .map(|b| (b.position.rx, b.position.ry));
            let bridge_state_changed = match bld_center {
                Some(center) => {
                    crate::sim::world::bridge_orchestrator::dispatch_bridge_collapse_from_hut(
                        self, rules, center,
                    )
                }
                None => false,
            };
            let _ = attacker_id; // hut survives — no last_attacker_id update
            return C4DamageOutcome {
                killed_building: false,
                bridge_state_changed,
                consumed_pending_marker: true,
            };
        }

        // Check IC for normal C4 targets. If invulnerable, damage is
        // nullified but pending state stays, so we try again next tick.
        let invuln = self
            .entities
            .get(building_id)
            .and_then(|e| e.invulnerability.clone());
        if crate::sim::superweapon::invulnerability::is_invulnerable(
            invuln.as_ref(),
            self.tick as u32,
        ) {
            return C4DamageOutcome::default();
        }

        // Resolve warhead, apply Verses, subtract HP.
        let warhead_name = self.interner.resolve(warhead_id).to_string();
        let Some(warhead) = rules.warhead(&warhead_name) else {
            return C4DamageOutcome::default();
        };
        let armor_idx: usize = match self.entities.get(building_id) {
            Some(b) => {
                let obj_armor = rules
                    .object(self.interner.resolve(b.type_ref))
                    .map(|o| o.armor.as_str())
                    .unwrap_or("none");
                crate::sim::combat::armor_index(obj_armor)
            }
            None => return C4DamageOutcome::default(),
        };
        let verses_pct = warhead.verses.get(armor_idx).copied().unwrap_or(100);
        let scaled = (damage as i32 * verses_pct as i32 / 100).max(0) as u16;

        let Some(b) = self.entities.get_mut(building_id) else {
            return C4DamageOutcome::default();
        };
        let new_hp = b.health.current.saturating_sub(scaled);
        b.health.current = new_hp;
        if new_hp == 0 {
            b.dying = true;
            if let Some(att) = attacker_id {
                b.last_attacker_id = Some(att);
            }
            C4DamageOutcome {
                killed_building: true,
                bridge_state_changed: false,
                consumed_pending_marker: false,
            }
        } else {
            C4DamageOutcome::default()
        }
    }

    /// Pre-combat: entities with an `attack_target` that's out of weapon
    /// range walk toward the target. Entities that just entered range halt
    /// their movement so the combat tick can fire from a stationary
    /// position.
    ///
    /// Range failure preserves the target; pursuit closes the gap.
    ///
    /// Skips entities that can't or shouldn't pursue:
    /// - Structures (can't move)
    /// - Aircraft (own state machine in `attack_mission.rs`)
    /// - Deployed-fire infantry (locked while deployed)
    /// - Entities inside transports
    /// - Dying entities
    pub(crate) fn tick_attack_pursuit(&mut self, rules: &RuleSet, path_grid: Option<&PathGrid>) {
        let Some(grid) = path_grid else {
            return;
        };

        // Phase 1: collect pursuit decisions (read-only on entities).
        // Two action kinds: issue a new path, or clear an existing one.
        enum PursuitAction {
            IssueMove { entity_id: u64, goal: (u16, u16) },
            ClearMovement { entity_id: u64 },
        }

        let keys: Vec<u64> = self.entities.keys_sorted();
        let mut actions: Vec<PursuitAction> = Vec::new();

        for &id in &keys {
            let Some(entity) = self.entities.get(id) else {
                continue;
            };
            let Some(attack) = entity.attack_target.as_ref() else {
                continue;
            };

            // Skip filters — see "Skips" doc above.
            if entity.dying {
                continue;
            }
            if entity.category == EntityCategory::Structure {
                continue;
            }
            if entity.aircraft_mission.is_some() {
                continue;
            }
            if entity.is_deployed() {
                continue;
            }
            if entity.passenger_role.is_inside_transport() {
                continue;
            }

            // Resolve target coords using the same helper combat tick uses.
            // None means entity-target despawned; combat tick's target-dead
            // branch handles cleanup.
            let target_pos = combat::resolve_target_coords(
                &attack.target,
                &self.entities,
                Some(rules),
                &self.interner,
            );
            let Some((trx, try_, tsx, tsy)) = target_pos else {
                continue;
            };

            // Resolve weapon range using shared helper. None means no weapon
            // can engage; combat tick will drop on its own weapon-select fail.
            let Some(weapon_range) = combat::pursuit_weapon_range(
                entity,
                &attack.target,
                &self.entities,
                rules,
                &self.interner,
            ) else {
                continue;
            };

            // Range check — same math as combat tick.
            let dist_sq = combat::lepton_distance_sq_raw(
                entity.position.rx,
                entity.position.ry,
                entity.position.sub_x,
                entity.position.sub_y,
                trx,
                try_,
                tsx,
                tsy,
            );
            let in_range = combat::is_within_range_leptons(dist_sq, weapon_range);

            if !in_range {
                if entity.movement_target.is_none() {
                    // Out of range, no current pursuit — issue a path.
                    actions.push(PursuitAction::IssueMove {
                        entity_id: id,
                        goal: (trx, try_),
                    });
                }
                // else: existing pursuit movement is still running; let it continue.
            } else if entity.movement_target.is_some() {
                // In range — halt for firing.
                actions.push(PursuitAction::ClearMovement { entity_id: id });
            }
        }

        // Phase 2: apply mutations.
        for action in actions {
            match action {
                PursuitAction::IssueMove { entity_id, goal } => {
                    let Some(info) = self.resolve_move_info(entity_id, Some(rules)) else {
                        continue;
                    };
                    let owner_str = self
                        .entities
                        .get(entity_id)
                        .map(|e| self.interner.resolve(e.owner).to_string())
                        .unwrap_or_default();
                    let (entity_blocks, entity_block_map) = bump_crush::build_entity_block_set(
                        &self.entities,
                        &owner_str,
                        &self.house_alliances,
                        &self.interner,
                        Some(rules),
                    );
                    let cost_grid = self.terrain_costs.get(&info.speed_type);
                    let _issued = movement::issue_move_command_with_layered(
                        &mut self.entities,
                        grid,
                        entity_id,
                        goal,
                        info.speed,
                        false, // queue
                        cost_grid,
                        Some(&entity_blocks),
                        self.resolved_terrain.as_ref(),
                        Some(&entity_block_map),
                        info.mover_is_crusher,
                    );
                    // No-op if A* fails — pursuit retries next tick.
                }
                PursuitAction::ClearMovement { entity_id } => {
                    if let Some(e) = self.entities.get_mut(entity_id) {
                        e.movement_target = None;
                    }
                }
            }
        }
    }
}

fn c4_base_foundation_cells(origin_rx: u16, origin_ry: u16, foundation: &str) -> Vec<(u16, u16)> {
    let (w, h) = crate::rules::foundation::foundation_dimensions(foundation);
    let mut cells = Vec::with_capacity(w as usize * h as usize);

    for dx in 0..w {
        for dy in 0..h {
            let rx = origin_rx as i32 + dx as i32;
            let ry = origin_ry as i32 + dy as i32;
            if rx >= 0 && rx <= u16::MAX as i32 && ry >= 0 && ry <= u16::MAX as i32 {
                cells.push((rx as u16, ry as u16));
            }
        }
    }

    cells
}
