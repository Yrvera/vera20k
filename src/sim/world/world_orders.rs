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
    /// Returns true if any capture occurred (triggers atlas rebuild for new owner color).
    pub(crate) fn tick_capture_orders(&mut self) -> bool {
        let mut any_captured = false;
        // Snapshot engineers with active capture targets.
        let captures: Vec<(u64, u64, InternedId)> = self
            .entities
            .values()
            .filter(|e| e.capture_target.is_some() && !e.dying)
            .map(|e| (e.stable_id, e.capture_target.unwrap(), e.owner))
            .collect();

        for (engineer_id, building_id, engineer_owner) in captures {
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
    /// damage equal to the building's current HP. The pending state is NOT
    /// cleared (gamemd parity): if the damage is nullified (IronCurtain),
    /// it fires again next tick. When the building dies, the entity despawns
    /// and the pending state goes with it.
    ///
    /// Returns true if any building was destroyed this tick.
    pub(crate) fn tick_c4_plants(&mut self, rules: &RuleSet) -> bool {
        use crate::sim::components::PendingC4Detonation;
        let mut destroyed_structure = false;

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

            // Adjacent to the target (Chebyshev distance ≤ 1)?
            //
            // gamemd has the SEAL walk INTO the building's cell, but our
            // pathfinder treats building footprints as blocked, so exact-cell
            // match would never trigger. Engineer-capture has the same
            // constraint and uses Chebyshev-≤-1 (above); we do the same.
            // Documented parity drift: SEAL stands one cell next to the
            // building rather than inside it during the plant animation.
            let attacker_cell = self
                .entities
                .get(attacker_id)
                .map(|e| (e.position.rx, e.position.ry));
            let target_cell = self
                .entities
                .get(target_id)
                .map(|b| (b.position.rx, b.position.ry));
            let adjacent_to_target = match (attacker_cell, target_cell) {
                (Some((arx, ary)), Some((trx, try_))) => {
                    let dx = (arx as i32 - trx as i32).abs();
                    let dy = (ary as i32 - try_ as i32).abs();
                    dx <= 1 && dy <= 1
                }
                _ => false,
            };
            if !adjacent_to_target {
                continue; // walk-up still in progress; movement layer handles it
            }

            // Already claimed by another attacker?
            let already_claimed = self
                .entities
                .get(target_id)
                .is_some_and(|b| b.pending_c4_detonation.is_some());
            if already_claimed {
                // Second SEAL — hover, no-op. Matches gamemd's marker-set early-return.
                continue;
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
                if let Some(ref mut anim) = a.animation {
                    anim.switch_to(crate::sim::animation::SequenceKind::Attack);
                }
            }

            // SealPlaceBomb spatial sound is queued via SimSoundEvent::C4Planted
            // — variant added in Task 8a.
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
            return destroyed_structure;
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
            // Pending state NOT cleared on purpose (gamemd parity).
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

            let killed = self.apply_c4_damage_to_building(
                building_id,
                dmg as u16,
                c4_warhead_id,
                attacker_for_credit,
                rules,
            );
            if killed {
                destroyed_structure = true;
                // pending_c4_detonation goes away with the entity via despawn path.
                // Trigger scatter walk-away for any attacker on this cell with
                // c4_plant pointing at this building. Matches gamemd
                // Mission_Enter post-detonation block.
                self.queue_c4_post_detonation_scatter(building_id);
            }
        }

        destroyed_structure
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

    /// Apply one C4Warhead damage instance to a building entity. Returns true
    /// if the building died this call. Honors IronCurtain via the standard
    /// invulnerability check. Used by `tick_c4_plants` Phase 2.
    fn apply_c4_damage_to_building(
        &mut self,
        building_id: u64,
        damage: u16,
        warhead_id: crate::sim::intern::InternedId,
        attacker_id: Option<u64>,
        rules: &RuleSet,
    ) -> bool {
        // Check IC — if invulnerable, damage is nullified but pending state
        // stays, so we try again next tick.
        let invuln = self
            .entities
            .get(building_id)
            .and_then(|e| e.invulnerability.clone());
        if crate::sim::superweapon::invulnerability::is_invulnerable(
            invuln.as_ref(),
            self.tick as u32,
        ) {
            return false;
        }

        // Resolve warhead, apply Verses, subtract HP.
        let warhead_name = self.interner.resolve(warhead_id).to_string();
        let Some(warhead) = rules.warhead(&warhead_name) else {
            return false;
        };
        let armor_idx: usize = match self.entities.get(building_id) {
            Some(b) => {
                let obj_armor = rules
                    .object(self.interner.resolve(b.type_ref))
                    .map(|o| o.armor.as_str())
                    .unwrap_or("none");
                crate::sim::combat::armor_index(obj_armor)
            }
            None => return false,
        };
        let verses_pct = warhead.verses.get(armor_idx).copied().unwrap_or(100);
        let scaled = (damage as i32 * verses_pct as i32 / 100).max(0) as u16;

        let Some(b) = self.entities.get_mut(building_id) else {
            return false;
        };
        let new_hp = b.health.current.saturating_sub(scaled);
        b.health.current = new_hp;
        if new_hp == 0 {
            b.dying = true;
            if let Some(att) = attacker_id {
                b.last_attacker_id = Some(att);
            }
            true
        } else {
            false
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
    pub(crate) fn tick_attack_pursuit(
        &mut self,
        rules: &RuleSet,
        path_grid: Option<&PathGrid>,
    ) {
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
