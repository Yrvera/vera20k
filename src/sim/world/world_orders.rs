//! Order-intent tick systems for the Simulation.
//!
//! Handles automatic target acquisition for attack-move and guard orders
//! (pre-combat), and resuming movement after combat ends (post-combat).
//!
//! Dependency rules: same as sim/ (depends on rules/, map/; never render/ui/audio/net).

use std::collections::BTreeSet;

use super::Simulation;
use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat;
use crate::sim::components::OrderIntent;
use crate::sim::intern::InternedId;
use crate::sim::mission::MissionType;
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
    ///
    /// The `order_intent.is_some()` selector is retired in spirit (the busy role
    /// moves to the `mission` substrate) but kept unchanged in code: `OrderIntent`
    /// carries the AttackMove/Guard *coords* that `MissionType` cannot encode.
    /// Full retirement (a goal field on the mission/nav substrate) is a later slice.
    pub(crate) fn tick_order_intents_pre_combat(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        turn_suppressed: &BTreeSet<u64>,
    ) {
        // Collect attacker candidates from EntityStore.
        let keys: Vec<u64> = self.substrate.entities.keys_sorted();
        let mut attacker_ids: Vec<u64> = Vec::new();
        for &id in &keys {
            if turn_suppressed.contains(&id) {
                continue;
            }
            if let Some(entity) = self.substrate.entities.get(id) {
                if entity.order_intent.is_some() && entity.attack_target.is_none() {
                    attacker_ids.push(id);
                }
            }
        }

        for attacker_id in attacker_ids {
            let Some(scan_mask) = self
                .substrate
                .entities
                .get(attacker_id)
                .map(combat::scan_mission_for)
            else {
                continue;
            };
            let Some(target_sid) = combat::acquire_best_target_for_entity(
                &self.substrate.entities,
                rules,
                &self.interner,
                attacker_id,
                Some(&self.fog),
                self.resolved_terrain.as_ref(),
                self.playfield_bounds.is_some(),
                // VERA-internal entry with no single native counterpart, so it
                // keeps the passive block's mask — `1`, or `2` for a player
                // "guard this spot" order. gamemd equivalent UNCHECKED.
                scan_mask,
                self.zone_grid.as_ref(),
                combat::line_of_fire::LineOfFireInputs {
                    overlay_grid: self.overlay_grid.as_ref(),
                    overlay_registry,
                    alliances: Some(&self.fog.alliances),
                },
            ) else {
                continue;
            };
            let _ = combat::issue_attack_command(
                &mut self.substrate.entities,
                attacker_id,
                target_sid,
                Some(rules),
                &self.interner,
            );
        }
    }

    /// Post-combat: entities with an OrderIntent but no active attack or movement
    /// resume their patrol/guard movement toward the original goal. The resume
    /// coords stay on `OrderIntent` — the `mission` substrate has no goal field
    /// yet (Slice-8 follow-up); only the busy-signalling role moved off it.
    #[cfg(test)]
    pub(crate) fn tick_order_intents_post_combat(
        &mut self,
        path_grid: Option<&PathGrid>,
        rules: Option<&RuleSet>,
    ) {
        self.tick_order_intents_post_combat_with_overlay_registry(
            path_grid,
            rules,
            None,
            &BTreeSet::new(),
        );
    }

    pub(crate) fn tick_order_intents_post_combat_with_overlay_registry(
        &mut self,
        path_grid: Option<&PathGrid>,
        rules: Option<&RuleSet>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        turn_suppressed: &BTreeSet<u64>,
    ) {
        let Some(grid) = path_grid else { return };
        // Collect (stable_id, goal) for entities that need to resume movement.
        let keys: Vec<u64> = self.substrate.entities.keys_sorted();
        let mut resumes: Vec<(u64, u16, u16)> = Vec::new();
        for &id in &keys {
            if turn_suppressed.contains(&id) {
                continue;
            }
            if let Some(entity) = self.substrate.entities.get(id) {
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
                .substrate
                .entities
                .get(stable_id)
                .map(|e| {
                    // `FootClass::GetCurrentSpeed @ 0x004DB1A0`: a resumed order
                    // re-queries the getter like any other, so the FASTER stage
                    // runs here too.
                    let obj = rules.and_then(|r| self.object_type(e.type_ref, r));
                    let bs: SimFixed =
                        crate::sim::combat::veterancy::entity_mover_speed_leptons_per_second(
                            e,
                            obj,
                            obj.map_or(4, |o| o.speed),
                            rules.map_or(1.0, |r| r.general.veteran_speed),
                        );
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
                    &mut self.substrate.entities,
                    stable_id,
                    (goal_rx, goal_ry),
                    speed,
                );
            } else {
                let blocker_neighbor_counts =
                    bump_crush::build_blocker_neighbor_counts_with_overlays(
                        &self.substrate.entities,
                        grid.width(),
                        grid.height(),
                        self.resolved_terrain.as_ref(),
                        self.overlay_grid.as_ref(),
                        overlay_registry,
                        &self.interner,
                        rules,
                    );
                let _ = movement::issue_move_command_with_layered(
                    &mut self.substrate.entities,
                    grid,
                    stable_id,
                    (goal_rx, goal_ry),
                    speed,
                    false,
                    None,
                    None,
                    self.resolved_terrain.as_ref(),
                    self.zone_grid.as_ref(),
                    None,
                    false, // mover_is_crusher
                    Some(&blocker_neighbor_counts),
                    self.playfield_bounds,
                    Some(&mut self.substrate.cell_occupation),
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
    pub(crate) fn tick_capture_orders(
        &mut self,
        rules: &RuleSet,
        turn_suppressed: &BTreeSet<u64>,
    ) -> bool {
        let mut any_captured = false;
        // Snapshot engineers with active capture targets.
        let captures: Vec<(u64, u64, InternedId)> = self
            .substrate
            .entities
            .values()
            .filter(|e| {
                e.capture_target.is_some() && !e.dying && !turn_suppressed.contains(&e.stable_id)
            })
            .map(|e| (e.stable_id, e.capture_target.unwrap(), e.owner))
            .collect();

        for (engineer_id, building_id, engineer_owner) in captures {
            // Skip BridgeRepairHut targets — repair tick handles them.
            let target_bridge_hut = self
                .substrate
                .entities
                .get(building_id)
                .and_then(|b| {
                    self.object_type(b.type_ref, rules)
                        .map(|t| t.bridge_repair_hut)
                })
                .unwrap_or(false);
            if target_bridge_hut {
                continue;
            }

            // Check building still exists and is capturable.
            let building_ok = self
                .substrate
                .entities
                .get(building_id)
                .is_some_and(|b| b.category == EntityCategory::Structure && !b.dying);
            if !building_ok {
                // Target lost — clear capture order.
                if let Some(e) = self.substrate.entities.get_mut(engineer_id) {
                    e.capture_target = None;
                }
                continue;
            }

            // Distance check: adjacent = Chebyshev distance <= 1 cell.
            let (eng_rx, eng_ry) = self
                .substrate
                .entities
                .get(engineer_id)
                .map(|e| (e.position.rx, e.position.ry))
                .unwrap_or((0, 0));
            let (bld_rx, bld_ry) = self
                .substrate
                .entities
                .get(building_id)
                .map(|e| (e.position.rx, e.position.ry))
                .unwrap_or((0, 0));
            let dx = (eng_rx as i32 - bld_rx as i32).abs();
            let dy = (eng_ry as i32 - bld_ry as i32).abs();

            if dx <= 1 && dy <= 1 {
                // CAPTURE: the ownership chokepoint moves HouseState counts,
                // the by-owner index, and the entity owner exactly once.
                self.change_owner_with_rules(building_id, engineer_owner, rules);
                // Destroy engineer (consumed on capture).
                self.uninit_with_rules(engineer_id, rules);
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
    pub(crate) fn tick_bridge_repair_orders_with_overlay_registry(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        turn_suppressed: &BTreeSet<u64>,
    ) -> bool {
        use crate::sim::bridge_state::cells_in_5x5_scan;

        let mut any_repair = false;
        let keys = self.substrate.entities.keys_sorted();
        let mut key_idx = 0;

        while key_idx < keys.len() {
            let engineer_id = keys[key_idx];
            if turn_suppressed.contains(&engineer_id) {
                key_idx += 1;
                continue;
            }
            let Some((building_id, engineer_owner)) =
                self.substrate.entities.get(engineer_id).and_then(|e| {
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
                .substrate
                .entities
                .get(building_id)
                .and_then(|b| {
                    self.object_type(b.type_ref, rules)
                        .map(|t| t.bridge_repair_hut)
                })
                .unwrap_or(false);
            if !target_bridge_hut {
                key_idx += 1;
                continue;
            }

            // Target alive + still a Structure.
            let target_alive = self
                .substrate
                .entities
                .get(building_id)
                .is_some_and(|b| b.category == EntityCategory::Structure && !b.dying);
            if !target_alive {
                if let Some(e) = self.substrate.entities.get_mut(engineer_id) {
                    e.capture_target = None;
                }
                key_idx += 1;
                continue;
            }

            // Adjacency only issues the scripted enter move; repair itself
            // waits until the engineer has arrived inside the building cell.
            let Some((erx, ery)) = self
                .substrate
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
                .substrate
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
            let eva_allowed =
                self.radar_events
                    .push(crate::sim::radar::RadarEventType::BridgeRepaired, brx, bry);

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
                // bridge repair walker-variant pick — gamemd draws g_MapGenRng, not the
                // scenario stream. Direct field (NOT bridge_rng(); `bs`/`terrain` hold live
                // disjoint borrows). VERA fixed-map construction currently keeps
                // Seed(0); native fresh-process state is verified, while cross-match
                // retention is UNCHECKED. Accepted generated maps continue their
                // post-RMG cursor. The scenario/main cursors are left untouched.
                bs.repair_bridge_from_engineer_scan(&scan, &mut self.mapgen_rng, terrain)
            } else {
                crate::sim::bridge_state::RepairOutcome::default()
            };

            if outcome.zones_dirty || outcome.repaired_cells > 0 {
                any_repair = true;
            }

            crate::sim::world::bridge_orchestrator::project_pending_low_bridge_overlay_writes(
                self,
                overlay_registry,
            );

            // Step B2: zone-graph refresh. The repair restores cells
            // (Destroyed -> Healthy) but the endpoint records are
            // deactivate-only at construction; without this the bidirectional
            // `refresh_endpoint_active_flags` never runs on the repair path,
            // so the long-range A* zone edge (gated on `record.active`) stays
            // missing even though per-cell walkability is restored. Mirrors
            // the collapse cascade's `refresh_bridge_zones_if_dirty` call.
            crate::sim::world::bridge_orchestrator::refresh_bridge_zones_if_dirty(
                self,
                outcome.zones_dirty,
            );

            // Step C: terrain/radar dirty propagation. The walker emits each
            // `ToggleBridgePavement @ 0x0056E990` damage-selector clear in
            // native traversal order, followed by destroyed-anchor restores.
            self.mark_radar_terrain_dirty_cells(outcome.radar_cells.iter().copied());

            // Step D: engineer consumed.
            self.uninit_with_rules(engineer_id, rules);
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
    /// if the wrapping native-frame elapsed count reaches
    /// `rules.c4_delay_ticks`, apply C4Warhead
    /// damage equal to the building's current HP. For normal buildings the
    /// pending state is not cleared if damage is nullified by IronCurtain, so
    /// it fires again next frame. When the building dies, the entity despawns
    /// and the pending state goes with it. BridgeRepairHut dispatch clears
    /// the pending marker after the bridge path runs because the hut survives.
    ///
    /// Returns the per-tick C4 outcome: `destroyed_structure` is true if any
    /// building died, and `bridge_state_changed` is true if any C4 detonation
    /// on a `BridgeRepairHut` collapsed a bridge.
    pub(crate) fn tick_c4_plants_with_overlay_registry(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        turn_suppressed: &BTreeSet<u64>,
    ) -> C4TickOutcome {
        use crate::sim::components::PendingC4Detonation;
        let mut destroyed_structure = false;
        let mut bridge_state_changed = false;

        // ---- Phase 1: walk-up + plant claim ----
        // Snapshot attackers with c4_plant. Deterministic sorted order via
        // keys_sorted then look up c4_plant.
        let mut walkup: Vec<(u64, u64)> = Vec::new();
        for sid in self.substrate.entities.keys_sorted() {
            if turn_suppressed.contains(&sid) {
                continue;
            }
            if let Some(e) = self.substrate.entities.get(sid) {
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
                .substrate
                .entities
                .get(target_id)
                .is_some_and(|b| b.category == EntityCategory::Structure && !b.dying);
            if !target_alive {
                if let Some(e) = self.substrate.entities.get_mut(attacker_id) {
                    e.c4_plant = None;
                }
                continue;
            }

            // gamemd claims only when the infantry's current cell resolves
            // to the target building. Normal pathing stops at the blocked
            // footprint boundary, then we issue the one-cell enter move below.
            let attacker_cell = self
                .substrate
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
                .substrate
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
            if let Some(b) = self.substrate.entities.get_mut(target_id) {
                b.pending_c4_detonation = Some(PendingC4Detonation {
                    start_frame: self.session.binary_frame as i32,
                    duration_frames: rules.c4_delay_ticks as i32,
                    source_entity_id: Some(attacker_id),
                });
            }

            // Drive the plant animation (FireUp = Attack sequence).
            if let Some(a) = self.substrate.entities.get_mut(attacker_id) {
                a.movement_target = None;
                if let Some(ref mut anim) = a.animation {
                    anim.switch_to(crate::sim::animation::SequenceKind::Attack);
                }
            }

            // SealPlaceBomb spatial sound. App-side dispatcher resolves to
            // `[SealPlaceBomb]` from soundmd.ini.
            if let Some(a) = self.substrate.entities.get(attacker_id) {
                self.sound_events
                    .push(crate::sim::world::SimSoundEvent::C4Planted {
                        rx: a.position.rx,
                        ry: a.position.ry,
                    });
            }
        }

        // ---- Phase 2: detonation ----
        let mut det_keys: Vec<u64> = Vec::new();
        for sid in self.substrate.entities.keys_sorted() {
            if let Some(e) = self.substrate.entities.get(sid) {
                let bridge_hut = rules
                    .object(self.interner.resolve(e.type_ref))
                    .is_some_and(|object| object.bridge_repair_hut);
                if e.pending_c4_detonation.is_some() && !e.dying && bridge_hut {
                    det_keys.push(sid);
                }
            }
        }
        // Early-out returns before rule-handle resolution so pre-feature
        // fixtures never intern warhead names they do not exercise. They do not
        // call it; guarding here keeps them passing.
        if det_keys.is_empty() {
            return C4TickOutcome {
                destroyed_structure,
                bridge_state_changed,
            };
        }

        let c4_warhead_id = self.rule_handles().c4;
        for building_id in det_keys {
            let pending = self
                .substrate
                .entities
                .get(building_id)
                .and_then(|e| e.pending_c4_detonation);
            let Some(pending) = pending else { continue };

            if !pending.is_expired_at(self.session.binary_frame as i32) {
                continue;
            }

            // Timer elapsed — apply C4Warhead damage. Damage value = current_hp
            // for guaranteed one-shot kill (matches gamemd's
            // `&iStack_28 = this->Health` argument to TakeDamage).
            // Normal-building pending state is only cleared by despawn;
            // BridgeRepairHut returns consumed_pending_marker below.
            let dmg: i32 = self
                .substrate
                .entities
                .get(building_id)
                .map(|b| b.health.current as i32)
                .unwrap_or(0);
            if dmg <= 0 {
                continue;
            }

            // Resolve kill-credit. Attacker may have despawned — fall back to None.
            let attacker_for_credit = pending
                .source_entity_id
                .filter(|&source_id| self.substrate.entities.get(source_id).is_some());

            let outcome = self.apply_c4_damage_to_building(
                building_id,
                dmg,
                c4_warhead_id,
                attacker_for_credit,
                rules,
                overlay_registry,
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
                if let Some(building) = self.substrate.entities.get_mut(building_id) {
                    building.pending_c4_detonation = None;
                }
                if let Some(attacker_id) = pending.source_entity_id
                    && let Some(attacker) = self.substrate.entities.get_mut(attacker_id)
                {
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

    /// BuildingClass::Update's shared C4/PostMortem expiry tail. Called from
    /// the current Structure LogicVector visit, so the forced receiver and any
    /// nested DeathWeapon complete before the next live object is visited.
    pub(crate) fn tick_pending_building_detonation(
        &mut self,
        building_id: u64,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) {
        let Some((pending, health, bridge_hut)) = self
            .substrate
            .entities
            .get(building_id)
            .and_then(|building| {
                let pending = building.pending_c4_detonation?;
                let object = rules.object(self.interner.resolve(building.type_ref))?;
                Some((
                    pending,
                    i32::from(building.health.current),
                    object.bridge_repair_hut,
                ))
            })
        else {
            return;
        };
        if !pending.is_expired_at(self.session.binary_frame as i32) || health <= 0 {
            return;
        }

        if bridge_hut {
            // Preserve the existing bridge-specific Phase-5 consumer outside
            // this damage slice. It owns collapse, attacker cleanup, and the
            // result flag that invalidates bridge navigation for the caller.
            return;
        }

        let event = crate::sim::combat::EntityDamageEvent::direct_receiver(
            building_id,
            health,
            0,
            pending
                .source_entity_id
                .unwrap_or(crate::sim::combat::RAD_NO_ATTACKER),
            None,
            self.rule_handles().c4,
            crate::sim::combat::ReceiverCallFlags {
                ignore_defenses: true,
                arg6: false,
            },
        );
        self.commit_noncombat_aoe_hits(rules, overlay_registry, &[event]);
    }

    fn building_entry_target_footprint(
        &self,
        target_id: u64,
        rules: &RuleSet,
    ) -> Option<Vec<(u16, u16)>> {
        let target = self.substrate.entities.get(target_id)?;
        let obj = self.object_type(target.type_ref, rules)?;
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
        self.substrate
            .entities
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
        if movement::issue_direct_move(&mut self.substrate.entities, attacker_id, entry_cell, speed)
        {
            if let Some(target) = self
                .substrate
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
        // Mirror the native-frame bit-twiddle: `(frame >> 12 + 1) >> 1 & 7`.
        // C operator precedence: `>>` is left-to-right at same level, so
        // this evaluates as `(((frame >> 12) + 1) >> 1) & 7`.
        let dir: usize = ((((self.session.binary_frame >> 12) + 1) >> 1) & 7) as usize;
        let (dx, dy) = DIR_DELTAS[dir];

        let bld_cell = self
            .substrate
            .entities
            .get(dead_building_id)
            .map(|b| (b.position.rx, b.position.ry));
        let Some((brx, bry)) = bld_cell else { return };

        // Collect attackers on this cell with c4_plant on this building.
        let mut scatterers: Vec<u64> = Vec::new();
        for sid in self.substrate.entities.keys_sorted() {
            if let Some(e) = self.substrate.entities.get(sid) {
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
            if let Some(e) = self.substrate.entities.get_mut(sid) {
                e.c4_plant = None;
            }
            // Queue a Move command for the next tick. Simpler than
            // reimplementing the pathfind call; 1-tick delay is below the
            // human-observable threshold.
            if let Some(owner) = self.substrate.entities.get(sid).map(|e| e.owner) {
                self.queue_command(crate::sim::command::CommandEnvelope::new(
                    owner,
                    self.session.tick + 1,
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
        damage: i32,
        warhead_id: crate::sim::intern::InternedId,
        attacker_id: Option<u64>,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) -> C4DamageOutcome {
        // BridgeRepairHut target: reroute the explosion into the bridge
        // collapse cascade and leave the hut at full HP. The hut never
        // takes C4 / demo-truck damage — destruction is the linked bridge
        // segment's, not the hut's. Also the right entry point for a
        // future demo-truck damage path.
        let target_bridge_hut = self
            .substrate
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
                .substrate
                .entities
                .get(building_id)
                .map(|b| (b.position.rx, b.position.ry));
            let bridge_state_changed = match bld_center {
                Some(center) => {
                    crate::sim::world::bridge_orchestrator::dispatch_bridge_collapse_from_hut_with_overlay_registry(
                        self,
                        rules,
                        center,
                        overlay_registry,
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

        let event = crate::sim::combat::EntityDamageEvent::direct_receiver(
            building_id,
            damage,
            0,
            attacker_id.unwrap_or(crate::sim::combat::RAD_NO_ATTACKER),
            None,
            warhead_id,
            crate::sim::combat::ReceiverCallFlags {
                ignore_defenses: true,
                arg6: false,
            },
        );
        self.commit_noncombat_aoe_hits(rules, overlay_registry, &[event]);
        if self
            .substrate
            .entities
            .get(building_id)
            .is_none_or(|b| b.dying)
        {
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
    /// - Objects holding a target their own scanner picked up (see below)
    /// - Objects on the **Sticky** mission, which drop the target instead
    ///
    /// **A passively-acquired target is never pursued.** The original's passive
    /// commit writes the target pointer and nothing else — no mission assign and
    /// no destination assign — and a Guard-mission unit derives any destination
    /// it does have from its OWN position, never from the target's. So an idle
    /// unit that notices an enemy fires from where it stands and stays put.
    /// Without this skip an idle base-defence force would walk off across the
    /// map, unleashed, the first time an enemy scouted past: nothing carries
    /// these units home because they have no `OrderIntent` to resume.
    #[cfg(test)]
    pub(crate) fn tick_attack_pursuit(&mut self, rules: &RuleSet, path_grid: Option<&PathGrid>) {
        self.tick_attack_pursuit_with_overlay_registry(rules, path_grid, None, &BTreeSet::new());
    }

    pub(crate) fn tick_attack_pursuit_with_overlay_registry(
        &mut self,
        rules: &RuleSet,
        path_grid: Option<&PathGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        turn_suppressed: &BTreeSet<u64>,
    ) {
        let Some(grid) = path_grid else {
            return;
        };

        // Phase 1: collect pursuit decisions (read-only on entities).
        // Two action kinds: issue a new path, or clear an existing one.
        enum PursuitAction {
            IssueMove {
                entity_id: u64,
                goal: (u16, u16),
            },
            ClearMovement {
                entity_id: u64,
            },
            /// A Sticky object that cannot already shoot its target: drop the
            /// target AND the destination, and produce no pursuit cell.
            DropTargetAndMovement {
                entity_id: u64,
            },
        }

        let keys: Vec<u64> = self.substrate.entities.keys_sorted();
        let mut actions: Vec<PursuitAction> = Vec::new();

        for &id in &keys {
            if turn_suppressed.contains(&id) {
                continue;
            }
            let Some(entity) = self.substrate.entities.get(id) else {
                continue;
            };
            let Some(attack) = entity.attack_target.as_ref() else {
                continue;
            };

            // Skip filters — see "Skips" doc above.
            if entity.dying {
                continue;
            }
            if entity.passively_acquired_target {
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
                &self.substrate.entities,
                Some(rules),
                &self.interner,
            );
            let Some((trx, try_, _tsx, _tsy)) = target_pos else {
                continue;
            };

            // Resolve the weapon using the shared helper. None means no weapon
            // can engage; combat tick will drop on its own weapon-select fail.
            let Some(weapon) = combat::pursuit_selected_weapon(
                entity,
                &attack.target,
                &self.substrate.entities,
                rules,
                &self.interner,
                self.resolved_terrain.as_ref(),
                Some(&self.house_alliances),
            ) else {
                continue;
            };

            // Range check — the SAME predicate the combat tick's fire gate
            // uses, line-of-fire walk included. The approach search
            // (`FootClass::Greatest_Threat_Scan @ 0x004D5690`, vt+0x53C,
            // reached from `Mission_Attack` 0x004D4DC0 at 0x004D4E6A) decides
            // with `TechnoClass::InRange` 0x006F7220 at 0x004D622C /
            // 0x004D6550, and `InRange` ends in the wall/cliff walk at
            // 0x006F7642. If this stage used the plain radius while the fire
            // gate ran the walk, a unit ordered to shoot across a wall would
            // halt here and then be refused the shot, standing still under a
            // live order.
            //
            // The one refusal that must NOT produce a pursuit cell is
            // `MinimumRange`: native's approach search picks a candidate
            // FARTHER out, and the target's own cell — VERA's only candidate —
            // is the worst one in that set. `HoldInsideMinimumRange` therefore
            // takes the halt arm, which is exactly what this stage did for that
            // case before the walk landed. See `PursuitRangeVerdict`.
            let verdict = combat::pursuit_in_range(
                entity,
                &attack.target,
                weapon,
                &self.substrate.entities,
                rules,
                &self.interner,
                self.resolved_terrain.as_ref(),
                // Same alliance view the fire gate hands the walk
                // (`resolve_attacker_fire` passes `fog.alliances`) and the same
                // one the sibling pre-combat scan uses, so the two stages
                // cannot disagree on the `AlliedWallTransparency` arm.
                &combat::line_of_fire::LineOfFireInputs {
                    overlay_grid: self.overlay_grid.as_ref(),
                    overlay_registry,
                    alliances: Some(&self.fog.alliances),
                },
            );

            if verdict == combat::PursuitRangeVerdict::CloseIn {
                // **Sticky never chases.** The one place the engine tells
                // Sticky apart from Guard at all — they share a mission handler
                // — is the pursuit-cell producer: when the can-fire-at query
                // fails and the object's committed mission is Sticky, it drops
                // both its target and its destination and returns "nowhere to
                // go", *before* the fallthrough that lets a Guard-family object
                // pursue. That is the whole of `[Sticky]`'s "just like guard
                // mode, but cannot move".
                //
                // Read off the RAW committed selector, as the original does —
                // not the derived reading.
                if entity.mission.current().known() == Some(MissionType::Sticky) {
                    actions.push(PursuitAction::DropTargetAndMovement { entity_id: id });
                } else if entity.movement_target.is_none() {
                    // Out of range, no current pursuit — issue a path.
                    actions.push(PursuitAction::IssueMove {
                        entity_id: id,
                        goal: (trx, try_),
                    });
                }
                // else: existing pursuit movement is still running; let it continue.
            } else if entity.movement_target.is_some() {
                // `CanFire` — halt for firing. `HoldInsideMinimumRange` halts
                // here too: closing further cannot help, and this is the arm it
                // took before the walk landed.
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
                        .substrate
                        .entities
                        .get(entity_id)
                        .map(|e| self.interner.resolve(e.owner).to_string())
                        .unwrap_or_default();
                    let (entity_blocks, entity_block_map) = bump_crush::build_entity_block_set(
                        &self.substrate.entities,
                        &owner_str,
                        &self.house_alliances,
                        &self.interner,
                        Some(rules),
                    );
                    let cost_grid = self.terrain_costs.get(&info.speed_type);
                    let blocker_neighbor_counts =
                        bump_crush::build_blocker_neighbor_counts_with_overlays(
                            &self.substrate.entities,
                            grid.width(),
                            grid.height(),
                            self.resolved_terrain.as_ref(),
                            self.overlay_grid.as_ref(),
                            overlay_registry,
                            &self.interner,
                            Some(rules),
                        );
                    let _issued = movement::issue_move_command_with_layered(
                        &mut self.substrate.entities,
                        grid,
                        entity_id,
                        goal,
                        info.speed,
                        false, // queue
                        cost_grid,
                        Some(&entity_blocks),
                        self.resolved_terrain.as_ref(),
                        self.zone_grid.as_ref(),
                        Some(&entity_block_map),
                        info.mover_is_crusher,
                        Some(&blocker_neighbor_counts),
                        self.playfield_bounds,
                        Some(&mut self.substrate.cell_occupation),
                    );
                    // No-op if A* fails — pursuit retries next tick.
                }
                PursuitAction::ClearMovement { entity_id } => {
                    if let Some(e) = self.substrate.entities.get_mut(entity_id) {
                        e.movement_target = None;
                    }
                }
                PursuitAction::DropTargetAndMovement { entity_id } => {
                    if let Some(e) = self.substrate.entities.get_mut(entity_id) {
                        e.attack_target = None;
                        e.passively_acquired_target = false;
                        e.movement_target = None;
                        e.navigation.nav_com = None;
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
