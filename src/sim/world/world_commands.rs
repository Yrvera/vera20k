//! Command dispatch for the Simulation.
//!
//! Contains `apply_command()` and its helper methods: selection snapshots,
//! ownership checks, and friendship queries. Split from world.rs for size.
//!
//! Dependency rules: same as sim/ (depends on rules/, map/; never render/ui/audio/net).

use std::collections::BTreeMap;

use super::Simulation;
use crate::map::houses::are_houses_friendly;
use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::rules::object_type::ObjectCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat;
use crate::sim::command::Command;
use crate::sim::components::OrderIntent;
use crate::sim::docking::building_dock::{self, DockPhase, DockState};
use crate::sim::mission::{DockTeardown, MissionType};
use crate::sim::movement;
use crate::sim::movement::air_movement;
use crate::sim::movement::bump_crush;
use crate::sim::movement::jumpjet_movement;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::movement::teleport_movement;
use crate::sim::movement::tunnel_movement;
use crate::sim::passenger;
use crate::sim::pathfinding::PathGrid;
use crate::sim::production;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, ra2_speed_to_leptons_per_second};

/// Read-only snapshot of entity + rules data needed for issuing movement commands.
/// Captured once to avoid repeated entity lookups and type_ref clones.
///
/// `pub(crate)` so the pursuit pre-combat stage in `world_orders.rs` can reuse
/// it — pursuit-issued movement must match Move-command-issued movement
/// exactly to keep behavior consistent.
pub(crate) struct MoveInfo {
    pub(crate) speed: SimFixed,
    pub(crate) loco_kind: Option<LocomotorKind>,
    pub(crate) loco_layer: MovementLayer,
    pub(crate) speed_type: SpeedType,
    pub(crate) hover_attack: bool,
    pub(crate) is_teleporter: bool,
    pub(crate) is_harvester: bool,
    pub(crate) is_infantry: bool,
    pub(crate) accel_factor: SimFixed,
    pub(crate) decel_factor: SimFixed,
    pub(crate) slowdown_distance: SimFixed,
    pub(crate) movement_zone: MovementZone,
    pub(crate) position: (u16, u16),
    pub(crate) regular_crusher: bool,
    pub(crate) omni_crusher: bool,
    pub(crate) drive_accelerates: bool,
    pub(crate) mover_is_crusher: bool,
}

impl MoveInfo {
    pub(crate) fn crush_capability(&self) -> bump_crush::CrushCapability {
        bump_crush::CrushCapability::new(self.regular_crusher, self.omni_crusher)
    }

    pub(crate) fn can_crush_units(&self) -> bool {
        self.crush_capability().can_crush_units()
    }
}

impl Simulation {
    /// Snapshot entity + rules data needed for movement dispatch in one lookup.
    pub(crate) fn resolve_move_info(
        &self,
        entity_id: u64,
        rules: Option<&RuleSet>,
    ) -> Option<MoveInfo> {
        let e = self.substrate.entities.get(entity_id)?;
        let loco = e.locomotor.as_ref();
        let loco_kind = loco.map(|l| l.kind);
        let loco_layer = e.movement_layer_or_ground();
        let speed_type = loco.map(|l| l.speed_type).unwrap_or(SpeedType::Track);
        let hover_attack = loco.map(|l| l.hover_attack).unwrap_or(false);
        let loco_multiplier = loco
            .map(|l| l.speed_multiplier)
            .unwrap_or(SimFixed::from_num(1));

        let obj = rules.and_then(|r| self.object_type(e.type_ref, r));
        let base_speed = obj
            .map(|o| ra2_speed_to_leptons_per_second(o.speed))
            .unwrap_or(ra2_speed_to_leptons_per_second(4));
        let speed = (base_speed * loco_multiplier).max(SimFixed::lit("25"));

        Some(MoveInfo {
            speed,
            loco_kind,
            loco_layer,
            speed_type,
            hover_attack,
            is_teleporter: obj.map_or(false, |o| o.teleporter),
            is_harvester: obj.map_or(false, |o| o.harvester),
            is_infantry: obj.map_or(false, |o| o.category == ObjectCategory::Infantry),
            accel_factor: obj.map_or(SIM_ZERO, |o| o.accel_factor),
            decel_factor: obj.map_or(SIM_ZERO, |o| o.decel_factor),
            slowdown_distance: obj.map_or(SIM_ZERO, |o| SimFixed::from_num(o.slowdown_distance)),
            movement_zone: obj.map_or(MovementZone::Normal, |o| o.movement_zone),
            position: (e.position.rx, e.position.ry),
            regular_crusher: e.regular_crusher,
            omni_crusher: e.omni_crusher,
            drive_accelerates: e.drive_accelerates,
            mover_is_crusher: e.regular_crusher || e.omni_crusher,
        })
    }

    /// Dispatch a single command, returning true if it was successfully applied.
    pub(crate) fn apply_command(
        &mut self,
        command_owner: &str,
        cmd: &Command,
        rules: Option<&RuleSet>,
        path_grid: Option<&PathGrid>,
        height_map: &BTreeMap<(u16, u16), u8>,
    ) -> bool {
        match cmd {
            Command::Select { entity_ids, .. } => {
                let mut snapshot = entity_ids.clone();
                snapshot.sort_unstable();
                snapshot.dedup();
                self.apply_selection_snapshot(&snapshot)
            }
            Command::Move {
                entity_id,
                target_rx,
                target_ry,
                queue,
                group_id,
            } => {
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                if self
                    .substrate.entities
                    .get(*entity_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
                // Drop any dock reservation (depot + aircraft + docked-idle) and
                // retask onto a fresh Move via the verb API. The legacy field
                // clears below stay authoritative in Slice 6.
                self.assign_mission_with_teardown(*entity_id, MissionType::Move, DockTeardown::All);
                // Clear attack and order intent.
                if let Some(e) = self.substrate.entities.get_mut(*entity_id) {
                    e.attack_target = None;
                    e.order_intent = None;
                    e.dock_state = None;
                    e.c4_plant = None;
                    Self::clear_aircraft_dock_phase(e);
                }
                // Snapshot speed, locomotor, and rules data in one lookup.
                let Some(info) = self.resolve_move_info(*entity_id, rules) else {
                    return false;
                };
                // Chrono Miners (Teleporter=yes + Harvester=yes) drive normally for
                // player commands — they only teleport on return-to-refinery
                // (handled by miner_system::chrono_teleport, not here).
                let use_teleport_move = !info.is_harvester
                    && (info.loco_kind == Some(LocomotorKind::Teleport) || info.is_teleporter);

                // Build entity block set for friendly-passable pathfinding.
                let (entity_blocks, entity_block_map) = bump_crush::build_entity_block_set(
                    &self.substrate.entities,
                    command_owner,
                    &self.house_alliances,
                    &self.interner,
                    rules,
                );
                let general_rules = rules.map(|r| &r.general);
                let result = if use_teleport_move {
                    // Teleport locomotor or non-harvester Teleporter=yes: instant relocation.
                    // `use_teleport_move` already excludes harvesters, so is_harvester=false.
                    let default_general = crate::rules::ruleset::GeneralRules::default();
                    teleport_movement::issue_teleport_command(
                        &mut self.substrate.entities,
                        *entity_id,
                        (*target_rx, *target_ry),
                        general_rules.unwrap_or(&default_general),
                        false,
                    )
                } else if info.loco_kind == Some(LocomotorKind::Tunnel) {
                    // Tunnel locomotor: short routes use surface, long routes burrow.
                    let Some(grid) = path_grid else { return false };
                    let tunnel_speed = rules
                        .map(|r| r.general.tunnel_speed)
                        .unwrap_or(SimFixed::from_num(6));
                    let cost_grid = self.terrain_costs.get(&info.speed_type);
                    tunnel_movement::issue_tunnel_move_command(
                        grid,
                        (*target_rx, *target_ry),
                        info.speed,
                        tunnel_speed,
                        cost_grid,
                        info.movement_zone,
                        &mut self.substrate.entities,
                        *entity_id,
                    )
                } else if info.loco_layer == MovementLayer::Air {
                    // Jumpjet infantry walk fallback: ≤3 cells + !HoverAttack → ground walk.
                    if info.loco_kind == Some(LocomotorKind::Jumpjet) && info.is_infantry {
                        let dx = (*target_rx as i32 - info.position.0 as i32).unsigned_abs();
                        let dy = (*target_ry as i32 - info.position.1 as i32).unsigned_abs();
                        let dist_cells = dx.max(dy);
                        if jumpjet_movement::should_use_walk_fallback(
                            info.hover_attack,
                            true,
                            dist_cells,
                        ) {
                            let Some(grid) = path_grid else { return false };
                            let cost_grid = self.terrain_costs.get(&info.speed_type);
                            return movement::issue_move_command_with_layered(
                                &mut self.substrate.entities,
                                grid,
                                *entity_id,
                                (*target_rx, *target_ry),
                                info.speed,
                                *queue,
                                cost_grid,
                                Some(&entity_blocks),
                                self.resolved_terrain.as_ref(),
                                self.zone_grid.as_ref(),
                                Some(&entity_block_map),
                                info.mover_is_crusher,
                            );
                        }
                    }
                    // Air units fly in straight lines — no A* pathfinding needed.
                    let ok = air_movement::issue_air_move_command(
                        &mut self.substrate.entities,
                        *entity_id,
                        (*target_rx, *target_ry),
                        info.speed,
                    );
                    // Set Move mission so the aircraft flies to destination
                    // before the Idle handler can redirect it to RTB.
                    if ok {
                        if let Some(e) = self.substrate.entities.get_mut(*entity_id) {
                            if e.aircraft_mission.is_some() {
                                e.aircraft_mission =
                                    Some(crate::sim::aircraft::AircraftMission::Move {
                                        sub_state: 0,
                                    });
                            }
                        }
                    }
                    ok
                } else {
                    let Some(grid) = path_grid else { return false };
                    let cost_grid = self.terrain_costs.get(&info.speed_type);
                    movement::issue_move_command_with_layered(
                        &mut self.substrate.entities,
                        grid,
                        *entity_id,
                        (*target_rx, *target_ry),
                        info.speed,
                        *queue,
                        cost_grid,
                        Some(&entity_blocks),
                        self.resolved_terrain.as_ref(),
                        self.zone_grid.as_ref(),
                        Some(&entity_block_map),
                        info.mover_is_crusher,
                    )
                };
                // Stamp acceleration/deceleration parameters onto the newly created
                // MovementTarget so the per-tick movement loop can ramp speed.
                if result {
                    if let Some(e) = self.substrate.entities.get_mut(*entity_id) {
                        if let Some(ref mut mt) = e.movement_target {
                            mt.accel_factor = info.accel_factor;
                            mt.decel_factor = info.decel_factor;
                            mt.slowdown_distance = info.slowdown_distance;
                            mt.group_id = *group_id;
                        }
                    }
                }
                result
            }
            Command::Stop { entity_id } => {
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                // Cancel any depot dock reservation, then retask onto Stop.
                self.assign_mission_with_teardown(*entity_id, MissionType::Stop, DockTeardown::Depot);
                if let Some(e) = self.substrate.entities.get_mut(*entity_id) {
                    movement::clear_navigation_for_entity(e);
                    e.movement_target = None;
                    e.attack_target = None;
                    e.order_intent = None;
                    e.dock_state = None;
                    e.c4_plant = None;
                }
                // Cancel any special locomotor states in progress.
                if let Some(e) = self.substrate.entities.get_mut(*entity_id) {
                    e.teleport_state = None;
                    e.tunnel_state = None;
                    e.droppod_state = None;
                    // Restore ground layer and base locomotor if overridden.
                    if let Some(ref mut loco) = e.locomotor {
                        if loco.layer == MovementLayer::Underground {
                            loco.layer = MovementLayer::Ground;
                        }
                        if loco.is_overridden() {
                            loco.end_override();
                        }
                    }
                }
                true
            }
            Command::Attack {
                attacker_id,
                target_id,
            } => {
                if !self.entity_owned_by_id(command_owner, *attacker_id) {
                    return false;
                }
                if !self.substrate.entities.contains(*target_id) {
                    return false;
                }
                if !self.can_attack_target_by_id(*attacker_id, *target_id) {
                    return false;
                }
                // Cancel aircraft RTB/wait + docked-idle (not depot), then retask
                // onto Attack keeping the interrupt stack (combat sets the target).
                self.assign_mission_keep_fields(
                    *attacker_id,
                    MissionType::Attack,
                    DockTeardown::AircraftOnly,
                );
                if let Some(e) = self.substrate.entities.get_mut(*attacker_id) {
                    e.order_intent = None;
                    Self::clear_aircraft_dock_phase(e);
                }
                combat::issue_attack_command(
                    &mut self.substrate.entities,
                    *attacker_id,
                    *target_id,
                    rules,
                    &self.interner,
                )
            }
            Command::ForceAttack {
                attacker_id,
                target_id,
            } => {
                if !self.entity_owned_by_id(command_owner, *attacker_id) {
                    return false;
                }
                if !self.substrate.entities.contains(*target_id) {
                    return false;
                }
                // Force-attack bypasses friendship check (Ctrl+click). Release a
                // docked-idle aircraft only, then retask onto Attack keeping fields.
                self.assign_mission_keep_fields(
                    *attacker_id,
                    MissionType::Attack,
                    DockTeardown::IdleOnly,
                );
                if let Some(e) = self.substrate.entities.get_mut(*attacker_id) {
                    e.order_intent = None;
                }
                combat::issue_attack_command(
                    &mut self.substrate.entities,
                    *attacker_id,
                    *target_id,
                    rules,
                    &self.interner,
                )
            }
            Command::ForceAttackCell {
                attacker_id,
                target_rx,
                target_ry,
            } => {
                if !self.entity_owned_by_id(command_owner, *attacker_id) {
                    return false;
                }
                // No target-entity existence check — cells always "exist". Release
                // a docked-idle aircraft only, then retask onto Attack keeping fields.
                self.assign_mission_keep_fields(
                    *attacker_id,
                    MissionType::Attack,
                    DockTeardown::IdleOnly,
                );
                if let Some(e) = self.substrate.entities.get_mut(*attacker_id) {
                    e.order_intent = None;
                    Self::clear_aircraft_dock_phase(e);
                }
                combat::issue_attack_cell_command(
                    &mut self.substrate.entities,
                    *attacker_id,
                    *target_rx,
                    *target_ry,
                    rules,
                    &self.interner,
                )
            }
            Command::AttackMove {
                entity_id,
                target_rx,
                target_ry,
                queue,
            } => {
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                if self
                    .substrate.entities
                    .get(*entity_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
                // Release a docked-idle aircraft only, then retask onto AttackMove
                // (the order_intent set after the move issues is the real driver).
                self.assign_mission_keep_fields(
                    *entity_id,
                    MissionType::AttackMove,
                    DockTeardown::IdleOnly,
                );
                if let Some(e) = self.substrate.entities.get_mut(*entity_id) {
                    e.attack_target = None;
                }

                // Snapshot speed, locomotor, and rules data in one lookup.
                let Some(info) = self.resolve_move_info(*entity_id, rules) else {
                    return false;
                };
                // Chrono Miners drive normally for player commands.
                let use_teleport_move = !info.is_harvester
                    && (info.loco_kind == Some(LocomotorKind::Teleport) || info.is_teleporter);

                let (entity_blocks, entity_block_map) = bump_crush::build_entity_block_set(
                    &self.substrate.entities,
                    command_owner,
                    &self.house_alliances,
                    &self.interner,
                    rules,
                );
                let default_general = crate::rules::ruleset::GeneralRules::default();
                let general_rules_ref = rules.map(|r| &r.general).unwrap_or(&default_general);
                let issued = if use_teleport_move {
                    // `use_teleport_move` excludes harvesters, so is_harvester=false.
                    teleport_movement::issue_teleport_command(
                        &mut self.substrate.entities,
                        *entity_id,
                        (*target_rx, *target_ry),
                        general_rules_ref,
                        false,
                    )
                } else if info.loco_layer == MovementLayer::Air {
                    // Air units fly in straight lines.
                    let ok = air_movement::issue_air_move_command(
                        &mut self.substrate.entities,
                        *entity_id,
                        (*target_rx, *target_ry),
                        info.speed,
                    );
                    if ok {
                        if let Some(e) = self.substrate.entities.get_mut(*entity_id) {
                            if e.aircraft_mission.is_some() {
                                e.aircraft_mission =
                                    Some(crate::sim::aircraft::AircraftMission::Move {
                                        sub_state: 0,
                                    });
                            }
                        }
                    }
                    ok
                } else {
                    let Some(grid) = path_grid else { return false };
                    let cost_grid = self.terrain_costs.get(&info.speed_type);
                    movement::issue_move_command_with_layered(
                        &mut self.substrate.entities,
                        grid,
                        *entity_id,
                        (*target_rx, *target_ry),
                        info.speed,
                        *queue,
                        cost_grid,
                        Some(&entity_blocks),
                        self.resolved_terrain.as_ref(),
                        self.zone_grid.as_ref(),
                        Some(&entity_block_map),
                        info.mover_is_crusher,
                    )
                };
                if issued {
                    if let Some(e) = self.substrate.entities.get_mut(*entity_id) {
                        e.order_intent = Some(OrderIntent::AttackMove {
                            goal_rx: *target_rx,
                            goal_ry: *target_ry,
                        });
                    }
                }
                issued
            }
            Command::Guard {
                entity_id,
                target_id,
            } => self.apply_guard_command(command_owner, *entity_id, *target_id, rules),
            Command::DeployMcv { entity_id } => {
                let Some(rules) = rules else { return false };
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                if self.substrate.entities.get(*entity_id).is_some_and(|entity| {
                    self
                        .object_type(entity.type_ref, rules)
                        .is_some_and(|obj| obj.enslaves.is_some() && obj.deploys_into.is_some())
                }) {
                    return crate::sim::slave_miner::deploy_slave_miner(self, *entity_id, rules)
                        .is_some();
                }
                self.deploy_mcv(*entity_id, rules, height_map)
            }
            Command::UndeployBuilding { entity_id } => {
                let Some(rules) = rules else { return false };
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                if self.substrate.entities.get(*entity_id).is_some_and(|entity| {
                    self
                        .object_type(entity.type_ref, rules)
                        .is_some_and(|obj| obj.enslaves.is_some() && obj.undeploys_into.is_some())
                }) {
                    return crate::sim::slave_miner::undeploy_slave_miner(self, *entity_id, rules)
                        .is_some();
                }
                self.undeploy_building(*entity_id, rules)
            }
            Command::ToggleInfantryDeploy { entity_id } => {
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                let Some(rules) = rules else { return false };
                // INI gate: only DeployFire=yes types respond.
                let type_str = match self.substrate.entities.get(*entity_id) {
                    Some(e) => self.interner.resolve(e.type_ref).to_string(),
                    None => return false,
                };
                let Some(obj) = rules.object(&type_str) else {
                    return false;
                };
                if !obj.deploy_fire {
                    return false;
                }
                let deploy_sound = obj.deploy_sound.clone();
                let undeploy_sound = obj.undeploy_sound.clone();
                // Per-type animation duration from artmd.ini sequence frame
                // counts. Fall back to DEPLOY_DEFAULT_TICKS when the art
                // section or sequence is missing.
                let art_entry = rules
                    .art_registry
                    .resolve_metadata_entry(&type_str, &obj.image);
                let deploying_ticks = crate::sim::deploy::compute_anim_ticks(
                    art_entry,
                    crate::sim::deploy::DeployPhaseKind::Deploying,
                );
                let undeploying_ticks = crate::sim::deploy::compute_anim_ticks(
                    art_entry,
                    crate::sim::deploy::DeployPhaseKind::Undeploying,
                );

                let Some(entity) = self.substrate.entities.get_mut(*entity_id) else {
                    return false;
                };
                let (rx, ry) = (entity.position.rx, entity.position.ry);
                let new_phase: Option<crate::sim::deploy::DeployPhase>;
                let mut emit_deploy_sound = false;
                let mut emit_undeploy_sound = false;
                match entity.deploy_state {
                    None => {
                        new_phase = Some(crate::sim::deploy::DeployPhase::Deploying {
                            ticks_remaining: deploying_ticks,
                        });
                        emit_deploy_sound = true;
                    }
                    Some(crate::sim::deploy::DeployPhase::Deployed) => {
                        new_phase = Some(crate::sim::deploy::DeployPhase::Undeploying {
                            ticks_remaining: undeploying_ticks,
                        });
                        emit_undeploy_sound = true;
                        // Belt-and-braces: clear any stale movement target.
                        entity.movement_target = None;
                    }
                    Some(crate::sim::deploy::DeployPhase::Deploying { .. })
                    | Some(crate::sim::deploy::DeployPhase::Undeploying { .. }) => {
                        return false;
                    }
                }
                // Sound plays BEFORE state field write — matches the original's
                // Do_Action ordering (voc cue precedes the Doing-field mutation).
                if emit_deploy_sound {
                    if let Some(sound_name) = deploy_sound {
                        let sound_id = self.interner.intern(&sound_name);
                        self.sound_events
                            .push(crate::sim::world::SimSoundEvent::EntityDeployed {
                                deploy_sound_id: sound_id,
                                rx,
                                ry,
                            });
                    }
                }
                if emit_undeploy_sound {
                    if let Some(sound_name) = undeploy_sound {
                        let sound_id = self.interner.intern(&sound_name);
                        self.sound_events.push(
                            crate::sim::world::SimSoundEvent::EntityUndeployed {
                                undeploy_sound_id: sound_id,
                                rx,
                                ry,
                            },
                        );
                    }
                }
                entity.deploy_state = new_phase;
                true
            }
            Command::SetRally {
                owner,
                rx,
                ry,
                producer_ids,
            } => {
                production::set_rally_point_for_owner(self, owner, *rx, *ry);
                self.set_rally_target_for_producers(command_owner, producer_ids, *rx, *ry, rules)
            }
            Command::QueueProduction { owner, type_id, .. } => {
                let Some(rules) = rules else { return false };
                let owner_s = self.interner.resolve(*owner).to_string();
                let type_s = self.interner.resolve(*type_id).to_string();
                production::enqueue_by_type(self, rules, &owner_s, &type_s)
            }
            Command::TogglePauseProduction { owner, category } => {
                let owner_s = self.interner.resolve(*owner).to_string();
                production::toggle_pause_for_owner_category(self, &owner_s, *category)
            }
            Command::CycleProducerFocus { owner, category } => {
                let Some(rules) = rules else { return false };
                let owner_s = self.interner.resolve(*owner).to_string();
                production::cycle_active_producer_for_owner_category(
                    self, rules, &owner_s, *category,
                )
            }
            Command::PlaceReadyBuilding {
                owner,
                type_id,
                rx,
                ry,
            } => {
                let Some(rules) = rules else { return false };
                let owner_s = self.interner.resolve(*owner).to_string();
                let type_s = self.interner.resolve(*type_id).to_string();
                production::place_ready_building(
                    self, rules, &owner_s, &type_s, *rx, *ry, path_grid, height_map,
                )
            }
            Command::CancelLastProduction { owner } => {
                let Some(rules) = rules else { return false };
                let owner_s = self.interner.resolve(*owner).to_string();
                production::cancel_last_for_owner(self, rules, &owner_s)
            }
            Command::CancelProductionByType { owner, type_id } => {
                let Some(rules) = rules else { return false };
                let owner_s = self.interner.resolve(*owner).to_string();
                let type_s = self.interner.resolve(*type_id).to_string();
                production::cancel_by_type_for_owner(self, rules, &owner_s, &type_s)
            }
            Command::SellBuilding { entity_id } => {
                let Some(rules) = rules else { return false };
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                production::sell_building(self, rules, *entity_id)
            }
            Command::ToggleRepair { entity_id } => {
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                production::toggle_repair(self, *entity_id)
            }
            Command::MinerReturn {
                entity_id,
                target_refinery_id,
            } => {
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                if self
                    .substrate.entities
                    .get(*entity_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
                let explicit_refinery = match target_refinery_id {
                    Some(refinery_id) => {
                        let Some(rules) = rules else { return false };
                        if !self.valid_explicit_miner_refinery(
                            command_owner,
                            *entity_id,
                            *refinery_id,
                            rules,
                        ) {
                            return false;
                        }
                        Some(*refinery_id)
                    }
                    None => None,
                };
                let previous_refinery = self
                    .substrate.entities
                    .get(*entity_id)
                    .and_then(|e| e.miner.as_ref())
                    .and_then(|m| m.reserved_refinery);
                let explicit_refinery_changed = explicit_refinery
                    .is_some_and(|refinery_id| previous_refinery != Some(refinery_id));
                if explicit_refinery_changed {
                    if let Some(old_refinery) = previous_refinery {
                        self.production
                            .dock_reservations
                            .cancel_miner(old_refinery, *entity_id);
                    }
                }
                // Update miner state in EntityStore.
                let Some(e) = self.substrate.entities.get_mut(*entity_id) else {
                    return false;
                };
                let Some(ref mut miner) = e.miner else {
                    return false;
                };
                if let Some(refinery_id) = explicit_refinery {
                    miner.reserved_refinery = Some(refinery_id);
                    if explicit_refinery_changed {
                        miner.dock_queued = false;
                        miner.dock_phase = crate::sim::miner::RefineryDockPhase::Approach;
                    }
                }
                miner.forced_return = true;
                miner.state = crate::sim::miner::MinerState::ForcedReturn;
                // Clear any in-progress movement — the miner system will path to refinery.
                e.movement_target = None;
                true
            }
            Command::RepairAtDepot {
                entity_id,
                depot_id,
            } => {
                let Some(rules) = rules else { return false };
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                if self
                    .substrate.entities
                    .get(*entity_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
                // Validate depot exists, is friendly, and has UnitRepair=yes.
                let depot_info = self.substrate.entities.get(*depot_id).and_then(|depot| {
                    if !command_owner.eq_ignore_ascii_case(self.interner.resolve(depot.owner)) {
                        return None;
                    }
                    let obj = self.object_type(depot.type_ref, rules)?;
                    if !obj.unit_repair {
                        return None;
                    }
                    Some((depot.position.rx, depot.position.ry, obj.foundation.clone()))
                });
                let Some((depot_rx, depot_ry, foundation)) = depot_info else {
                    return false;
                };
                // Validate entity is a unit or infantry (not structure/aircraft).
                let entity_ok = self.substrate.entities.get(*entity_id).is_some_and(|e| {
                    matches!(
                        e.category,
                        crate::map::entities::EntityCategory::Unit
                            | crate::map::entities::EntityCategory::Infantry
                    ) && e.health.current < e.health.max
                        && !e.dying
                });
                if !entity_ok {
                    return false;
                }
                // Cancel any existing depot reservation, then retask onto Enter.
                self.assign_mission_with_teardown(*entity_id, MissionType::Enter, DockTeardown::Depot);
                // Set dock state and issue move toward depot.
                let (dock_rx, dock_ry) =
                    building_dock::depot_dock_cell(depot_rx, depot_ry, &foundation);
                if let Some(e) = self.substrate.entities.get_mut(*entity_id) {
                    e.attack_target = None;
                    e.order_intent = None;
                    e.dock_state = Some(DockState {
                        dock_building_id: *depot_id,
                        phase: DockPhase::Approach,
                        service_timer: 0,
                        no_funds_ticks: 0,
                    });
                }
                // Issue movement toward dock cell.
                let info = self.resolve_move_info(*entity_id, Some(rules));
                let speed = info
                    .as_ref()
                    .map(|i| i.speed)
                    .unwrap_or(ra2_speed_to_leptons_per_second(4));
                let speed_type = info
                    .as_ref()
                    .map(|i| i.speed_type)
                    .unwrap_or(SpeedType::Track);
                let crusher = info.as_ref().map_or(false, |i| i.mover_is_crusher);
                let (entity_blocks, entity_block_map) = bump_crush::build_entity_block_set(
                    &self.substrate.entities,
                    command_owner,
                    &self.house_alliances,
                    &self.interner,
                    Some(rules),
                );
                if let Some(grid) = path_grid {
                    let cost_grid = self.terrain_costs.get(&speed_type);
                    movement::issue_move_command_with_layered(
                        &mut self.substrate.entities,
                        grid,
                        *entity_id,
                        (dock_rx, dock_ry),
                        speed,
                        false,
                        cost_grid,
                        Some(&entity_blocks),
                        self.resolved_terrain.as_ref(),
                        self.zone_grid.as_ref(),
                        Some(&entity_block_map),
                        crusher,
                    );
                }
                true
            }
            Command::EnterTransport {
                passenger_id,
                transport_id,
            } => {
                let Some(rules) = rules else { return false };
                if !self.entity_owned_by_id(command_owner, *passenger_id) {
                    return false;
                }
                if self
                    .substrate.entities
                    .get(*passenger_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
                // Validate transport exists and has cargo capacity.
                let transport_info = self.substrate.entities.get(*transport_id).and_then(|t| {
                    let obj = self.object_type(t.type_ref, rules)?;
                    let cargo = t.passenger_role.cargo()?;
                    Some((t.position.rx, t.position.ry, obj.clone(), cargo.clone()))
                });
                let Some((trx, try_, transport_obj, cargo)) = transport_info else {
                    return false;
                };
                // Validate passenger can enter.
                let pax_ok = self.substrate.entities.get(*passenger_id).and_then(|p| {
                    let pobj = self.object_type(p.type_ref, rules)?;
                    if passenger::can_enter_transport(
                        p,
                        self.substrate.entities.get(*transport_id)?,
                        pobj,
                        &transport_obj,
                        &cargo,
                        rules,
                        &self.houses,
                        &self.interner,
                        path_grid,
                    ) {
                        Some(())
                    } else {
                        None
                    }
                });
                if pax_ok.is_none() {
                    return false;
                }
                // Retask onto Enter (no dock reservation touched); the legacy
                // field clears below stay authoritative.
                self.assign_mission_with_teardown(
                    *passenger_id,
                    MissionType::Enter,
                    DockTeardown::None,
                );
                // Clear existing state on the passenger.
                if let Some(e) = self.substrate.entities.get_mut(*passenger_id) {
                    e.attack_target = None;
                    e.order_intent = None;
                    e.dock_state = None;
                    e.passenger_role = passenger::PassengerRole::Boarding {
                        target_transport_id: *transport_id,
                        phase: passenger::BoardingPhase::Approach,
                    };
                }
                // Issue movement toward transport cell.
                let info = self.resolve_move_info(*passenger_id, Some(rules));
                let speed = info
                    .as_ref()
                    .map(|i| i.speed)
                    .unwrap_or(ra2_speed_to_leptons_per_second(4));
                let speed_type = info
                    .as_ref()
                    .map(|i| i.speed_type)
                    .unwrap_or(SpeedType::Track);
                let crusher = info.as_ref().map_or(false, |i| i.mover_is_crusher);
                let (entity_blocks, entity_block_map) = bump_crush::build_entity_block_set(
                    &self.substrate.entities,
                    command_owner,
                    &self.house_alliances,
                    &self.interner,
                    Some(rules),
                );
                if let Some(grid) = path_grid {
                    let cost_grid = self.terrain_costs.get(&speed_type);
                    movement::issue_move_command_with_layered(
                        &mut self.substrate.entities,
                        grid,
                        *passenger_id,
                        (trx, try_),
                        speed,
                        false,
                        cost_grid,
                        Some(&entity_blocks),
                        self.resolved_terrain.as_ref(),
                        self.zone_grid.as_ref(),
                        Some(&entity_block_map),
                        crusher,
                    );
                }
                true
            }
            Command::UnloadPassengers { transport_id } => {
                if !self.entity_owned_by_id(command_owner, *transport_id) {
                    return false;
                }
                let has_passengers = self
                    .substrate.entities
                    .get(*transport_id)
                    .and_then(|t| t.passenger_role.cargo())
                    .is_some_and(|c| !c.is_empty());
                if !has_passengers {
                    return false;
                }
                if let Some(e) = self.substrate.entities.get_mut(*transport_id) {
                    e.order_intent = Some(OrderIntent::Unloading);
                }
                true
            }
            Command::HarvestCell {
                entity_id,
                target_rx,
                target_ry,
            } => {
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                if self
                    .substrate.entities
                    .get(*entity_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
                let Some(e) = self.substrate.entities.get_mut(*entity_id) else {
                    return false;
                };
                let Some(ref mut miner) = e.miner else {
                    return false;
                };
                miner.target_ore_cell = Some((*target_rx, *target_ry));
                miner.state = crate::sim::miner::MinerState::MoveToOre;
                // Clear in-progress movement so the miner re-paths to the new target.
                e.movement_target = None;
                true
            }
            Command::PlantC4 {
                attacker_id,
                target_building_id,
            } => {
                let Some(rules) = rules else { return false };
                if !self.entity_owned_by_id(command_owner, *attacker_id) {
                    return false;
                }
                if self
                    .substrate.entities
                    .get(*attacker_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
                // Validate attacker has C4=yes flag.
                let c4_ok = self.substrate.entities.get(*attacker_id).and_then(|e| {
                    let obj = self.object_type(e.type_ref, rules)?;
                    obj.c4.then_some(())
                });
                if c4_ok.is_none() {
                    return false;
                }
                // Validate target is a CanC4, non-invisible enemy building, not iron-curtained.
                // TODO(parity): also reject selling-in-progress buildings (Mission==0x13);
                // requires building Mission state which isn't modeled yet.
                let target_info = self.substrate.entities.get(*target_building_id).and_then(|b| {
                    if b.category != crate::map::entities::EntityCategory::Structure {
                        return None;
                    }
                    if b.dying {
                        return None;
                    }
                    let obj = self.object_type(b.type_ref, rules)?;
                    if !obj.can_c4 || obj.invisible_in_game {
                        return None;
                    }
                    if crate::sim::superweapon::invulnerability::is_invulnerable(
                        b.invulnerability.as_ref(),
                        self.session.tick as u32,
                    ) {
                        return None;
                    }
                    Some((b.position.rx, b.position.ry, b.owner))
                });
                let Some((trx, try_, target_owner)) = target_info else {
                    return false;
                };
                // Enemy-only.
                if crate::map::houses::are_houses_friendly(
                    &self.house_alliances,
                    command_owner,
                    self.interner.resolve(target_owner),
                ) {
                    return false;
                }
                // Retask onto Sabotage (no dock reservation touched); the legacy
                // field clears below stay authoritative.
                self.assign_mission_with_teardown(
                    *attacker_id,
                    MissionType::Sabotage,
                    DockTeardown::None,
                );
                // Clear conflicting state and set c4_plant.
                if let Some(e) = self.substrate.entities.get_mut(*attacker_id) {
                    e.attack_target = None;
                    e.order_intent = None;
                    e.dock_state = None;
                    e.capture_target = None;
                    e.c4_plant = Some(crate::sim::components::C4PlantState {
                        target_building_id: *target_building_id,
                    });
                }
                // Issue movement toward the building's cell.
                let info = self.resolve_move_info(*attacker_id, Some(rules));
                let speed = info
                    .as_ref()
                    .map(|i| i.speed)
                    .unwrap_or(ra2_speed_to_leptons_per_second(4));
                let speed_type = info
                    .as_ref()
                    .map(|i| i.speed_type)
                    .unwrap_or(crate::rules::locomotor_type::SpeedType::Foot);
                let crusher = info.as_ref().map_or(false, |i| i.mover_is_crusher);
                let (entity_blocks, entity_block_map) =
                    crate::sim::movement::bump_crush::build_entity_block_set(
                        &self.substrate.entities,
                        command_owner,
                        &self.house_alliances,
                        &self.interner,
                        Some(rules),
                    );
                if let Some(grid) = path_grid {
                    let cost_grid = self.terrain_costs.get(&speed_type);
                    movement::issue_move_command_with_layered(
                        &mut self.substrate.entities,
                        grid,
                        *attacker_id,
                        (trx, try_),
                        speed,
                        false,
                        cost_grid,
                        Some(&entity_blocks),
                        self.resolved_terrain.as_ref(),
                        self.zone_grid.as_ref(),
                        Some(&entity_block_map),
                        crusher,
                    );
                }
                true
            }
            Command::CaptureBuilding {
                engineer_id,
                target_building_id,
            } => {
                let Some(rules) = rules else { return false };
                if !self.entity_owned_by_id(command_owner, *engineer_id) {
                    return false;
                }
                if self
                    .substrate.entities
                    .get(*engineer_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
                // Validate engineer has Engineer=yes flag.
                let eng_ok = self.substrate.entities.get(*engineer_id).and_then(|e| {
                    let obj = self.object_type(e.type_ref, rules)?;
                    obj.engineer.then_some(())
                });
                if eng_ok.is_none() {
                    return false;
                }
                // Validate target is a capturable enemy building.
                let target_info = self.substrate.entities.get(*target_building_id).and_then(|b| {
                    if b.category != crate::map::entities::EntityCategory::Structure {
                        return None;
                    }
                    if b.dying {
                        return None;
                    }
                    let obj = self.object_type(b.type_ref, rules)?;
                    if !obj.capturable && !obj.bridge_repair_hut {
                        return None;
                    }
                    Some((b.position.rx, b.position.ry, b.owner))
                });
                let Some((trx, try_, target_owner)) = target_info else {
                    return false;
                };
                // Must be an enemy building.
                if crate::map::houses::are_houses_friendly(
                    &self.house_alliances,
                    command_owner,
                    self.interner.resolve(target_owner),
                ) {
                    return false;
                }
                // Retask onto Capture (no dock reservation touched); the legacy
                // field clears below stay authoritative.
                self.assign_mission_with_teardown(
                    *engineer_id,
                    MissionType::Capture,
                    DockTeardown::None,
                );
                // Clear conflicting state and set capture target.
                if let Some(e) = self.substrate.entities.get_mut(*engineer_id) {
                    e.attack_target = None;
                    e.order_intent = None;
                    e.dock_state = None;
                    e.capture_target = Some(*target_building_id);
                }
                // Issue movement toward the building's cell.
                let info = self.resolve_move_info(*engineer_id, Some(rules));
                let speed = info
                    .as_ref()
                    .map(|i| i.speed)
                    .unwrap_or(ra2_speed_to_leptons_per_second(4));
                let speed_type = info
                    .as_ref()
                    .map(|i| i.speed_type)
                    .unwrap_or(crate::rules::locomotor_type::SpeedType::Foot);
                let crusher = info.as_ref().map_or(false, |i| i.mover_is_crusher);
                let (entity_blocks, entity_block_map) =
                    crate::sim::movement::bump_crush::build_entity_block_set(
                        &self.substrate.entities,
                        command_owner,
                        &self.house_alliances,
                        &self.interner,
                        Some(rules),
                    );
                if let Some(grid) = path_grid {
                    let cost_grid = self.terrain_costs.get(&speed_type);
                    movement::issue_move_command_with_layered(
                        &mut self.substrate.entities,
                        grid,
                        *engineer_id,
                        (trx, try_),
                        speed,
                        false,
                        cost_grid,
                        Some(&entity_blocks),
                        self.resolved_terrain.as_ref(),
                        self.zone_grid.as_ref(),
                        Some(&entity_block_map),
                        crusher,
                    );
                }
                true
            }
            Command::LaunchSuperWeapon {
                sw_type_id,
                target_rx,
                target_ry,
            } => {
                if !self.session.game_options.super_weapons {
                    return false;
                }
                let owner_iid = self.interner.intern(command_owner);
                let sw_type_str = self.interner.resolve(*sw_type_id).to_string();

                // Look up the instance and verify it's ready.
                let is_ready = self
                    .super_weapons
                    .get(&owner_iid)
                    .and_then(|weapons| weapons.get(sw_type_id))
                    .map_or(false, |inst| inst.is_active && inst.is_ready);
                if !is_ready {
                    log::warn!(
                        "LaunchSuperWeapon '{}' by '{}' — not ready",
                        sw_type_str,
                        command_owner,
                    );
                    return false;
                }

                // Look up the type to determine dispatch kind.
                let Some(sw_type) = rules.and_then(|r| r.super_weapon(&sw_type_str)) else {
                    return false;
                };
                let kind = sw_type.kind;
                let recharge = sw_type.recharge_time_frames;

                // Dispatch based on kind.
                let success = match kind {
                    crate::rules::superweapon_type::SuperWeaponKind::LightningStorm => {
                        let rules = rules.unwrap();
                        crate::sim::superweapon::lightning_storm::start(
                            self, rules, owner_iid, *target_rx, *target_ry,
                        )
                    }
                    crate::rules::superweapon_type::SuperWeaponKind::IronCurtain => {
                        let rules = rules.unwrap();
                        crate::sim::superweapon::iron_curtain::launch(
                            self, rules, owner_iid, *target_rx, *target_ry,
                        )
                    }
                    crate::rules::superweapon_type::SuperWeaponKind::ForceShield => {
                        let rules = rules.unwrap();
                        crate::sim::superweapon::force_shield::launch(
                            self, rules, owner_iid, *target_rx, *target_ry,
                        )
                    }
                    crate::rules::superweapon_type::SuperWeaponKind::GeneticConverter => {
                        let rules = rules.unwrap();
                        crate::sim::superweapon::genetic_converter::launch(
                            self, rules, owner_iid, *target_rx, *target_ry,
                        )
                    }
                    crate::rules::superweapon_type::SuperWeaponKind::PsychicReveal => {
                        let rules = rules.unwrap();
                        crate::sim::superweapon::psychic_reveal::launch(
                            self, rules, owner_iid, *target_rx, *target_ry,
                        )
                    }
                    crate::rules::superweapon_type::SuperWeaponKind::ParaDrop => {
                        let rules = rules.unwrap();
                        crate::sim::superweapon::paradrop::launch(
                            self,
                            rules,
                            owner_iid,
                            *target_rx,
                            *target_ry,
                            crate::sim::superweapon::paradrop::ParaDropKind::Generic,
                            path_grid,
                        )
                    }
                    crate::rules::superweapon_type::SuperWeaponKind::AmerParaDrop => {
                        let rules = rules.unwrap();
                        crate::sim::superweapon::paradrop::launch(
                            self,
                            rules,
                            owner_iid,
                            *target_rx,
                            *target_ry,
                            crate::sim::superweapon::paradrop::ParaDropKind::American,
                            path_grid,
                        )
                    }
                    other => {
                        log::warn!("SuperWeapon kind {:?} not yet implemented", other);
                        false
                    }
                };

                if success {
                    // Reset the instance — restart charging.
                    if let Some(weapons) = self.super_weapons.get_mut(&owner_iid) {
                        if let Some(inst) = weapons.get_mut(sw_type_id) {
                            inst.reset_after_fire(recharge, self.session.tick);
                        }
                    }
                }
                success
            }
            Command::EnterBunker { unit_id, bunker_id } => {
                let Some(rules) = rules else { return false };
                if !self.entity_owned_by_id(command_owner, *unit_id) {
                    return false;
                }
                if self
                    .substrate
                    .entities
                    .get(*unit_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
                // Target must be an own tank bunker (seeded `bunker_runtime`).
                let is_bunker = self
                    .substrate
                    .entities
                    .get(*bunker_id)
                    .is_some_and(|b| b.bunker_runtime.is_some());
                if !is_bunker || !self.entity_owned_by_id(command_owner, *bunker_id) {
                    return false;
                }
                // Rules-gated weapon/Bunkerable check (the bus stays rules-free).
                if !crate::sim::docking::bunker_link::can_auto_deploy_here(self, *unit_id, rules) {
                    return false;
                }
                // Admission query over the bus; commit only on ROGER.
                if crate::sim::radio::transmit(
                    self,
                    *unit_id,
                    *bunker_id,
                    crate::sim::radio::RadioMessage::CanEnter,
                    crate::sim::radio::RadioPayload::default(),
                ) != crate::sim::radio::RadioResponse::Roger
                {
                    return false;
                }
                // Commit: start the install machine (ArriveWait + installing_unit).
                crate::sim::radio::transmit(
                    self,
                    *unit_id,
                    *bunker_id,
                    crate::sim::radio::RadioMessage::DockNow,
                    crate::sim::radio::RadioPayload::default(),
                );
                // Retask onto Enter (no dock reservation), mark the unit as
                // approaching THIS bunker (the install machine's keep-alive gate).
                self.assign_mission_with_teardown(*unit_id, MissionType::Enter, DockTeardown::None);
                if let Some(e) = self.substrate.entities.get_mut(*unit_id) {
                    e.attack_target = None;
                    e.order_intent = None;
                    e.dock_state = None;
                    e.c4_plant = None;
                    e.bunker_link =
                        crate::sim::game_entity::BunkerLink::Approaching(*bunker_id);
                }
                // Issue an approach move toward the bunker cell (mirror EnterTransport).
                let bunker_cell = self
                    .substrate
                    .entities
                    .get(*bunker_id)
                    .map(|b| (b.position.rx, b.position.ry));
                if let Some((brx, bry)) = bunker_cell {
                    let info = self.resolve_move_info(*unit_id, Some(rules));
                    let speed = info
                        .as_ref()
                        .map(|i| i.speed)
                        .unwrap_or(ra2_speed_to_leptons_per_second(4));
                    let speed_type = info
                        .as_ref()
                        .map(|i| i.speed_type)
                        .unwrap_or(SpeedType::Track);
                    let crusher = info.as_ref().map_or(false, |i| i.mover_is_crusher);
                    let (entity_blocks, entity_block_map) = bump_crush::build_entity_block_set(
                        &self.substrate.entities,
                        command_owner,
                        &self.house_alliances,
                        &self.interner,
                        Some(rules),
                    );
                    if let Some(grid) = path_grid {
                        let cost_grid = self.terrain_costs.get(&speed_type);
                        movement::issue_move_command_with_layered(
                            &mut self.substrate.entities,
                            grid,
                            *unit_id,
                            (brx, bry),
                            speed,
                            false,
                            cost_grid,
                            Some(&entity_blocks),
                            self.resolved_terrain.as_ref(),
                            self.zone_grid.as_ref(),
                            Some(&entity_block_map),
                            crusher,
                        );
                    }
                }
                true
            }
            Command::EjectBunker { bunker_id } => {
                let Some(rules) = rules else { return false };
                if !self.entity_owned_by_id(command_owner, *bunker_id) {
                    return false;
                }
                let has_occupant = self
                    .substrate
                    .entities
                    .get(*bunker_id)
                    .is_some_and(|b| b.bunker_occupant.is_some());
                if !has_occupant {
                    return false;
                }
                crate::sim::docking::bunker_link::release_normal(
                    self, *bunker_id, rules, path_grid,
                );
                true
            }
        }
    }

    fn set_rally_target_for_producers(
        &mut self,
        command_owner: &str,
        producer_ids: &[u64],
        rx: u16,
        ry: u16,
        rules: Option<&RuleSet>,
    ) -> bool {
        let Some(rules) = rules else {
            return true;
        };
        let mut ids = producer_ids.to_vec();
        ids.sort_unstable();
        ids.dedup();
        for stable_id in ids {
            let eligible = self.substrate.entities.get(stable_id).is_some_and(|entity| {
                entity.category == crate::map::entities::EntityCategory::Structure
                    && command_owner.eq_ignore_ascii_case(self.interner.resolve(entity.owner))
                    && self
                        .object_type(entity.type_ref, rules)
                        .is_some_and(|obj| obj.has_rally_line())
            });
            if eligible {
                if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                    entity.rally_target = Some((rx, ry));
                }
            }
        }
        true
    }

    /// Cancel depot dock reservation for an entity. Called before issuing new orders.
    pub(crate) fn cancel_depot_dock(&mut self, entity_id: u64) {
        if let Some(e) = self.substrate.entities.get(entity_id) {
            if let Some(ref ds) = e.dock_state {
                self.production
                    .depot_dock_reservations
                    .cancel(ds.dock_building_id, entity_id);
            }
        }
    }

    /// Cancel aircraft dock reservation if in ReturnToBase or WaitForDock phase.
    pub(crate) fn cancel_aircraft_dock(&mut self, entity_id: u64) {
        if let Some(e) = self.substrate.entities.get(entity_id) {
            if let Some(ref ammo) = e.aircraft_ammo {
                use crate::sim::docking::aircraft_dock::AircraftDockPhase;
                if matches!(
                    ammo.dock_phase,
                    Some(AircraftDockPhase::ReturnToBase) | Some(AircraftDockPhase::WaitForDock)
                ) {
                    self.production.airfield_docks.cancel(entity_id);
                }
            }
        }
    }

    /// Clear aircraft dock phase on an entity if interruptible (RTB/WaitForDock).
    fn clear_aircraft_dock_phase(entity: &mut crate::sim::game_entity::GameEntity) {
        if let Some(ref mut ammo) = entity.aircraft_ammo {
            use crate::sim::docking::aircraft_dock::AircraftDockPhase;
            if matches!(
                ammo.dock_phase,
                Some(AircraftDockPhase::ReturnToBase) | Some(AircraftDockPhase::WaitForDock)
            ) {
                ammo.dock_phase = None;
                ammo.target_airfield = None;
            }
        }
    }

    /// Release a DockedIdle aircraft from its helipad and trigger takeoff.
    /// Called when a docked aircraft receives a Move or Attack command.
    pub(crate) fn release_docked_idle(&mut self, entity_id: u64) {
        let Some(entity) = self.substrate.entities.get_mut(entity_id) else {
            return;
        };
        if let Some(crate::sim::aircraft::AircraftMission::DockedIdle { .. }) =
            entity.aircraft_mission
        {
            // Release dock slot.
            self.production.airfield_docks.release(entity_id);
            // Clear to Idle — the command handler will set the appropriate mission.
            entity.aircraft_mission = Some(crate::sim::aircraft::AircraftMission::Idle);
            // Trigger takeoff.
            if let Some(ref mut loco) = entity.locomotor {
                if loco.air_phase == crate::sim::movement::locomotor::AirMovePhase::Landed {
                    loco.air_phase = crate::sim::movement::locomotor::AirMovePhase::Ascending;
                }
            }
        }
    }

    /// Replace the current selection with exactly the given stable entity IDs.
    fn apply_selection_snapshot(&mut self, stable_ids: &[u64]) -> bool {
        // Deselect all via EntityStore.
        let keys: Vec<u64> = self.substrate.entities.keys_sorted();
        for &id in &keys {
            if let Some(e) = self.substrate.entities.get_mut(id) {
                e.selected = false;
            }
        }
        // Select the requested IDs.
        for &stable_id in stable_ids {
            if let Some(e) = self.substrate.entities.get_mut(stable_id) {
                e.selected = true;
            }
        }
        true
    }

    /// Check ownership using stable_id via EntityStore.
    pub(crate) fn entity_owned_by_id(&self, command_owner: &str, stable_id: u64) -> bool {
        self.substrate.entities
            .get(stable_id)
            .is_some_and(|e| command_owner.eq_ignore_ascii_case(self.interner.resolve(e.owner)))
    }

    /// Validate an explicit refinery selected by a player miner-return order.
    fn valid_explicit_miner_refinery(
        &self,
        command_owner: &str,
        miner_id: u64,
        refinery_id: u64,
        rules: &RuleSet,
    ) -> bool {
        let Some(miner) = self.substrate.entities.get(miner_id) else {
            return false;
        };
        if miner.miner.is_none() {
            return false;
        }
        let harvester_type = self.interner.resolve(miner.type_ref);
        let Some(refinery) = self.substrate.entities.get(refinery_id) else {
            return false;
        };
        if refinery.category != crate::map::entities::EntityCategory::Structure {
            return false;
        }
        if refinery.health.current == 0 || refinery.dying || refinery.building_up.is_some() {
            return false;
        }
        let refinery_owner = self.interner.resolve(refinery.owner);
        if !are_houses_friendly(&self.house_alliances, command_owner, refinery_owner) {
            return false;
        }
        let refinery_type = self.interner.resolve(refinery.type_ref);
        rules.is_refinery_type(refinery_type)
            && rules.harvester_can_dock_at(harvester_type, refinery_type)
    }

    /// Check whether the attacker can attack the target (i.e. they are not allies).
    /// Uses EntityStore for ownership lookup.
    fn can_attack_target_by_id(&self, attacker_id: u64, target_id: u64) -> bool {
        let Some(attacker) = self.substrate.entities.get(attacker_id) else {
            return false;
        };
        let Some(target) = self.substrate.entities.get(target_id) else {
            return false;
        };
        !are_houses_friendly(
            &self.house_alliances,
            self.interner.resolve(attacker.owner),
            self.interner.resolve(target.owner),
        )
    }

    /// Apply a Guard command: anchor at current position, optionally attack a target.
    fn apply_guard_command(
        &mut self,
        command_owner: &str,
        entity_id: u64,
        target_id: Option<u64>,
        rules: Option<&RuleSet>,
    ) -> bool {
        if !self.entity_owned_by_id(command_owner, entity_id) {
            return false;
        }
        let anchor = self
            .substrate.entities
            .get(entity_id)
            .map(|e| (e.position.rx, e.position.ry));
        let Some((anchor_rx, anchor_ry)) = anchor else {
            return false;
        };
        if let Some(e) = self.substrate.entities.get_mut(entity_id) {
            e.movement_target = None;
        }
        match target_id.filter(|&tid| self.substrate.entities.contains(tid)) {
            Some(tid) => {
                if !self.can_attack_target_by_id(entity_id, tid) {
                    return false;
                }
                let issued = combat::issue_attack_command(
                    &mut self.substrate.entities,
                    entity_id,
                    tid,
                    rules,
                    &self.interner,
                );
                if issued {
                    if let Some(e) = self.substrate.entities.get_mut(entity_id) {
                        e.order_intent = Some(OrderIntent::Guard {
                            anchor_rx,
                            anchor_ry,
                        });
                    }
                }
                issued
            }
            None => {
                if let Some(e) = self.substrate.entities.get_mut(entity_id) {
                    e.attack_target = None;
                    e.order_intent = Some(OrderIntent::Guard {
                        anchor_rx,
                        anchor_ry,
                    });
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::house_state::HouseState;
    use crate::sim::miner::{Miner, MinerConfig, MinerKind, MinerState, RefineryDockPhase};
    use crate::sim::movement::locomotor::LocomotorState;

    fn amcv_move_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             0=AMCV\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [AMCV]\n\
             Strength=1000\n\
             Speed=4\n\
             Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\n\
             MovementZone=Normal\n\
             Crusher=yes\n\
             DeploysInto=GACNST\n",
        );
        RuleSet::from_ini(&ini).expect("amcv rules")
    }

    fn spawn_rule_backed_unit(sim: &mut Simulation, sid: u64, type_id: &str, rules: &RuleSet) {
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern(type_id);
        let obj = rules.object(type_id).expect("object type");
        let health = obj.strength.clamp(0, u16::MAX as i32) as u16;
        let mut entity = GameEntity::new(
            sid,
            20,
            20,
            0,
            0,
            owner,
            Health {
                current: health,
                max: health,
            },
            type_ref,
            EntityCategory::Unit,
            0,
            obj.sight.max(0) as u16,
            true,
        );
        entity.locomotor = Some(LocomotorState::from_object_type(
            obj,
            rules.general.flight_level,
        ));
        entity.regular_crusher = obj.crusher;
        entity.drive_accelerates = obj.accelerates;
        entity.omni_crusher = obj.omni_crusher;
        sim.substrate.entities.insert(entity);
    }

    #[test]
    fn resolve_move_info_uses_stock_amcv_speed_without_deployable_multiplier() {
        let rules = amcv_move_rules();
        let mut sim = Simulation::new();
        spawn_rule_backed_unit(&mut sim, 1, "AMCV", &rules);

        let info = sim.resolve_move_info(1, Some(&rules)).expect("move info");

        assert_eq!(info.speed, ra2_speed_to_leptons_per_second(4));
    }

    #[test]
    fn resolve_move_info_carries_regular_crusher() {
        let rules = amcv_move_rules();
        let mut sim = Simulation::new();
        spawn_rule_backed_unit(&mut sim, 1, "AMCV", &rules);

        let info = sim.resolve_move_info(1, Some(&rules)).expect("move info");

        assert!(info.regular_crusher);
        assert!(!info.omni_crusher);
        assert_eq!(info.movement_zone, MovementZone::Normal);
        assert!(info.can_crush_units());
        assert_eq!(
            info.crush_capability(),
            bump_crush::CrushCapability::new(true, false)
        );
    }

    #[test]
    fn resolve_move_info_carries_accelerates_flag() {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             0=AMCV\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [AMCV]\n\
             Strength=1000\n\
             Speed=4\n\
             Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\n\
             MovementZone=Normal\n\
             Accelerates=false\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("amcv rules");
        let mut sim = Simulation::new();
        spawn_rule_backed_unit(&mut sim, 1, "AMCV", &rules);

        let info = sim.resolve_move_info(1, Some(&rules)).expect("move info");

        assert!(!info.drive_accelerates);
    }

    #[test]
    fn player_drive_move_command_passes_zone_grid_to_path_search() {
        let rules = amcv_move_rules();
        let mut sim = Simulation::new();
        spawn_rule_backed_unit(&mut sim, 1, "AMCV", &rules);
        let grid = crate::sim::pathfinding::PathGrid::new(64, 64);
        sim.zone_grid = Some(crate::sim::pathfinding::zone_map::ZoneGrid::build(
            &grid,
            &BTreeMap::new(),
            5,
            1,
        ));
        crate::sim::movement::reset_path_search_used_zone_grid_marker();

        let applied = sim.apply_command(
            "Americans",
            &Command::Move {
                entity_id: 1,
                target_rx: 25,
                target_ry: 20,
                queue: false,
                group_id: None,
            },
            Some(&rules),
            Some(&grid),
            &BTreeMap::new(),
        );

        assert!(applied);
        assert!(crate::sim::movement::path_search_used_zone_grid_marker());
    }

    fn miner_return_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             0=HARV\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             0=GAREFN\n\
             1=OTHERPROC\n\
             [HARV]\n\
             Name=War Miner\n\
             Harvester=yes\n\
             Dock=GAREFN\n\
             Speed=4\n\
             [GAREFN]\n\
             Name=Ore Refinery\n\
             Strength=900\n\
             Foundation=4x3\n\
             Refinery=yes\n\
             [OTHERPROC]\n\
             Name=Other Refinery\n\
             Strength=900\n\
             Foundation=4x3\n\
             Refinery=yes\n",
        );
        RuleSet::from_ini(&ini).expect("miner return rules")
    }

    fn spawn_miner(sim: &mut Simulation, sid: u64) {
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("HARV");
        let mut entity = GameEntity::new(
            sid,
            20,
            20,
            0,
            0,
            owner,
            Health {
                current: 600,
                max: 600,
            },
            type_ref,
            EntityCategory::Unit,
            0,
            5,
            true,
        );
        entity.miner = Some(Miner::new(MinerKind::War, &MinerConfig::default(), 0));
        sim.substrate.entities.insert(entity);
    }

    fn spawn_refinery(sim: &mut Simulation, sid: u64, type_id: &str, rx: u16, ry: u16) {
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern(type_id);
        let entity = GameEntity::new(
            sid,
            rx,
            ry,
            0,
            0,
            owner,
            Health {
                current: 900,
                max: 900,
            },
            type_ref,
            EntityCategory::Structure,
            0,
            5,
            false,
        );
        sim.substrate.entities.insert(entity);
    }

    fn rally_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             0=GAPILE\n\
             1=GAWEAP\n\
             2=GAPOWR\n\
             3=NAWEAP\n\
             [GAPILE]\nFactory=InfantryType\nStrength=500\n\
             [GAWEAP]\nFactory=UnitType\nStrength=1000\n\
             [GAPOWR]\nStrength=750\n\
             [NAWEAP]\nFactory=UnitType\nStrength=1000\n",
        );
        RuleSet::from_ini(&ini).expect("rally rules")
    }

    fn spawn_structure_for_owner(
        sim: &mut Simulation,
        sid: u64,
        type_id: &str,
        owner_name: &str,
        rx: u16,
        ry: u16,
    ) {
        let owner = sim.interner.intern(owner_name);
        let type_ref = sim.interner.intern(type_id);
        sim.substrate.entities.insert(GameEntity::new(
            sid,
            rx,
            ry,
            0,
            0,
            owner,
            Health {
                current: 1000,
                max: 1000,
            },
            type_ref,
            EntityCategory::Structure,
            0,
            5,
            false,
        ));
    }

    #[test]
    fn set_rally_updates_only_owned_eligible_producers() {
        let rules = rally_rules();
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let enemy = sim.interner.intern("Soviet");
        sim.houses.insert(
            owner,
            HouseState::new(owner, 0, Some(owner), true, 10_000, 10),
        );
        sim.houses.insert(
            enemy,
            HouseState::new(enemy, 1, Some(enemy), false, 10_000, 10),
        );
        spawn_structure_for_owner(&mut sim, 2, "GAPILE", "Americans", 10, 10);
        spawn_structure_for_owner(&mut sim, 3, "GAWEAP", "Americans", 12, 10);
        spawn_structure_for_owner(&mut sim, 4, "GAPOWR", "Americans", 14, 10);
        spawn_structure_for_owner(&mut sim, 5, "NAWEAP", "Soviet", 16, 10);

        let command = Command::SetRally {
            owner,
            rx: 40,
            ry: 41,
            producer_ids: vec![3, 2, 2, 4, 5],
        };

        assert!(sim.apply_command("Americans", &command, Some(&rules), None, &BTreeMap::new()));
        assert_eq!(sim.substrate.entities.get(2).unwrap().rally_target, Some((40, 41)));
        assert_eq!(sim.substrate.entities.get(3).unwrap().rally_target, Some((40, 41)));
        assert_eq!(sim.substrate.entities.get(4).unwrap().rally_target, None);
        assert_eq!(sim.substrate.entities.get(5).unwrap().rally_target, None);
        assert_eq!(sim.houses.get(&owner).unwrap().rally_point, Some((40, 41)));
    }

    #[test]
    fn miner_return_with_explicit_refinery_seeds_clicked_target() {
        let rules = miner_return_rules();
        let mut sim = Simulation::new();
        spawn_miner(&mut sim, 1);
        spawn_refinery(&mut sim, 2, "GAREFN", 10, 10);
        spawn_refinery(&mut sim, 3, "GAREFN", 30, 30);
        {
            let miner = sim.substrate.entities.get_mut(1).unwrap().miner.as_mut().unwrap();
            miner.reserved_refinery = Some(2);
            miner.dock_queued = true;
            miner.dock_phase = RefineryDockPhase::Unloading;
        }
        assert!(sim.production.dock_reservations.try_reserve(2, 1));

        let applied = sim.apply_command(
            "Americans",
            &Command::MinerReturn {
                entity_id: 1,
                target_refinery_id: Some(3),
            },
            Some(&rules),
            None,
            &BTreeMap::new(),
        );

        assert!(applied);
        let miner = sim.substrate.entities.get(1).unwrap().miner.as_ref().unwrap();
        assert_eq!(miner.reserved_refinery, Some(3));
        assert!(miner.forced_return);
        assert_eq!(miner.state, MinerState::ForcedReturn);
        assert!(!miner.dock_queued);
        assert_eq!(miner.dock_phase, RefineryDockPhase::Approach);
        assert!(!sim.production.dock_reservations.is_occupied(2));
    }

    #[test]
    fn generic_miner_return_can_reselect_later_without_rules() {
        let mut sim = Simulation::new();
        spawn_miner(&mut sim, 1);

        let applied = sim.apply_command(
            "Americans",
            &Command::MinerReturn {
                entity_id: 1,
                target_refinery_id: None,
            },
            None,
            None,
            &BTreeMap::new(),
        );

        assert!(applied);
        let miner = sim.substrate.entities.get(1).unwrap().miner.as_ref().unwrap();
        assert_eq!(miner.reserved_refinery, None);
        assert!(miner.forced_return);
        assert_eq!(miner.state, MinerState::ForcedReturn);
    }

    #[test]
    fn explicit_miner_return_rejects_incompatible_refinery() {
        let rules = miner_return_rules();
        let mut sim = Simulation::new();
        spawn_miner(&mut sim, 1);
        spawn_refinery(&mut sim, 2, "OTHERPROC", 10, 10);

        let applied = sim.apply_command(
            "Americans",
            &Command::MinerReturn {
                entity_id: 1,
                target_refinery_id: Some(2),
            },
            Some(&rules),
            None,
            &BTreeMap::new(),
        );

        assert!(!applied);
        let miner = sim.substrate.entities.get(1).unwrap().miner.as_ref().unwrap();
        assert_eq!(miner.reserved_refinery, None);
        assert!(!miner.forced_return);
        assert_eq!(miner.state, MinerState::SearchOre);
    }

    fn bunker_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[VehicleTypes]\n0=TANK\n1=NOGUN\n\n[InfantryTypes]\n\n[AircraftTypes]\n\n\
             [BuildingTypes]\n0=NATBNK\n\n\
             [TANK]\nStrength=400\nArmor=heavy\nSpeed=6\nBunkerable=yes\nPrimary=120mm\n\n\
             [NOGUN]\nStrength=400\nArmor=heavy\nSpeed=6\nBunkerable=yes\n\n\
             [NATBNK]\nStrength=1000\nArmor=heavy\nBunker=yes\n",
        ))
        .expect("bunker rules")
    }

    fn spawn_bunker_struct(sim: &mut Simulation, sid: u64, owner: &str, rx: u16, ry: u16) {
        let owner_id = sim.interner.intern(owner);
        let type_id = sim.interner.intern("NATBNK");
        let mut ge = GameEntity::new(
            sid,
            rx,
            ry,
            0,
            0,
            owner_id,
            Health {
                current: 1000,
                max: 1000,
            },
            type_id,
            EntityCategory::Structure,
            0,
            5,
            false,
        );
        ge.bunker_runtime = Some(crate::sim::docking::bunker_install::BunkerRuntime::idle());
        sim.substrate.entities.insert(ge);
    }

    fn spawn_bunkerable(sim: &mut Simulation, sid: u64, owner: &str, type_name: &str, rx: u16, ry: u16) {
        let owner_id = sim.interner.intern(owner);
        let type_id = sim.interner.intern(type_name);
        let ge = GameEntity::new(
            sid,
            rx,
            ry,
            0,
            0,
            owner_id,
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
        sim.substrate.entities.insert(ge);
    }

    #[test]
    fn enter_bunker_admits_and_starts_install_machine() {
        use crate::sim::docking::bunker_install::BunkerState;
        use crate::sim::game_entity::BunkerLink;
        let rules = bunker_rules();
        let mut sim = Simulation::new();
        spawn_bunker_struct(&mut sim, 2, "Americans", 10, 10);
        spawn_bunkerable(&mut sim, 1, "Americans", "TANK", 14, 14);

        let applied = sim.apply_command(
            "Americans",
            &Command::EnterBunker {
                unit_id: 1,
                bunker_id: 2,
            },
            Some(&rules),
            None,
            &BTreeMap::new(),
        );

        assert!(applied);
        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!(unit.bunker_link, BunkerLink::Approaching(2));
        assert_eq!(unit.mission.current, MissionType::Enter);
        let rt = sim.substrate.entities.get(2).unwrap().bunker_runtime.unwrap();
        assert_eq!(rt.state, BunkerState::ArriveWait);
        assert_eq!(rt.installing_unit, Some(1));
    }

    #[test]
    fn enter_bunker_rejects_unit_without_a_weapon() {
        use crate::sim::docking::bunker_install::BunkerState;
        use crate::sim::game_entity::BunkerLink;
        let rules = bunker_rules();
        let mut sim = Simulation::new();
        spawn_bunker_struct(&mut sim, 2, "Americans", 10, 10);
        // Bunkerable but no Primary → CanAutoDeployHere rejects it.
        spawn_bunkerable(&mut sim, 1, "Americans", "NOGUN", 14, 14);

        let applied = sim.apply_command(
            "Americans",
            &Command::EnterBunker {
                unit_id: 1,
                bunker_id: 2,
            },
            Some(&rules),
            None,
            &BTreeMap::new(),
        );

        assert!(!applied);
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().bunker_link,
            BunkerLink::None
        );
        assert_eq!(
            sim.substrate.entities.get(2).unwrap().bunker_runtime.unwrap().state,
            BunkerState::Idle,
            "rejected admission leaves the machine idle"
        );
    }

    #[test]
    fn enter_enemy_bunker_is_rejected() {
        let rules = bunker_rules();
        let mut sim = Simulation::new();
        spawn_bunker_struct(&mut sim, 2, "Soviets", 10, 10);
        spawn_bunkerable(&mut sim, 1, "Americans", "TANK", 14, 14);

        let applied = sim.apply_command(
            "Americans",
            &Command::EnterBunker {
                unit_id: 1,
                bunker_id: 2,
            },
            Some(&rules),
            None,
            &BTreeMap::new(),
        );

        assert!(!applied, "cannot bunker into an enemy building");
    }

    #[test]
    fn eject_bunker_releases_occupant() {
        use crate::sim::game_entity::BunkerLink;
        let rules = bunker_rules();
        let mut sim = Simulation::new();
        spawn_bunker_struct(&mut sim, 2, "Americans", 10, 10);
        spawn_bunkerable(&mut sim, 1, "Americans", "TANK", 14, 14);
        sim.reveal(1);
        sim.add_entity_occupancy(1);
        crate::sim::docking::bunker_link::install_bunker_link(&mut sim, 2, 1);
        assert_eq!(sim.substrate.entities.get(2).unwrap().bunker_occupant, Some(1));

        let applied = sim.apply_command(
            "Americans",
            &Command::EjectBunker { bunker_id: 2 },
            Some(&rules),
            None,
            &BTreeMap::new(),
        );

        assert!(applied);
        assert_eq!(sim.substrate.entities.get(2).unwrap().bunker_occupant, None);
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().bunker_link,
            BunkerLink::None
        );
        // Released at the anchor SW of the bunker (10,10) + (-1,+1) when no grid.
        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!((unit.position.rx, unit.position.ry), (9, 11));
        assert_eq!(unit.mission.current, MissionType::Move);
    }

    #[test]
    fn eject_empty_bunker_is_noop() {
        let rules = bunker_rules();
        let mut sim = Simulation::new();
        spawn_bunker_struct(&mut sim, 2, "Americans", 10, 10);

        let applied = sim.apply_command(
            "Americans",
            &Command::EjectBunker { bunker_id: 2 },
            Some(&rules),
            None,
            &BTreeMap::new(),
        );

        assert!(!applied, "ejecting an empty bunker does nothing");
    }

    #[test]
    fn bunker_full_lifecycle_enter_install_then_eject() {
        use crate::sim::docking::bunker_install::{tick_bunker_install, BunkerState};
        use crate::sim::game_entity::BunkerLink;
        let rules = bunker_rules();
        let mut sim = Simulation::new();
        spawn_bunker_struct(&mut sim, 2, "Americans", 10, 10);
        // Place the tank ON the bunker cell so the install needs no pathfinding
        // (the movement subsystem is not run in this harness).
        spawn_bunkerable(&mut sim, 1, "Americans", "TANK", 10, 10);
        sim.reveal(1);
        sim.add_entity_occupancy(1);

        // 1) Enter: admission + install machine starts.
        assert!(sim.apply_command(
            "Americans",
            &Command::EnterBunker {
                unit_id: 1,
                bunker_id: 2,
            },
            Some(&rules),
            None,
            &BTreeMap::new(),
        ));
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().bunker_link,
            BunkerLink::Approaching(2)
        );

        // 2) Drive the install machine to Occupied. Clear facing_target each tick
        // to simulate the body turn completing (no movement subsystem here).
        for _ in 0..6 {
            tick_bunker_install(&mut sim, &rules, None);
            if let Some(u) = sim.substrate.entities.get_mut(1) {
                u.facing_target = None;
            }
        }
        let rt = sim.substrate.entities.get(2).unwrap().bunker_runtime.unwrap();
        assert_eq!(rt.state, BunkerState::Occupied);
        assert_eq!(sim.substrate.entities.get(2).unwrap().bunker_occupant, Some(1));
        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!(unit.bunker_link, BunkerLink::Installed(2));
        assert!(!unit.in_logic_vector, "occupant hidden while installed");
        assert_eq!(
            sim.bunker_wall_events.iter().filter(|e| e.up).count(),
            1,
            "one walls-up event on install"
        );

        // 3) Eject: occupant released near the bunker, links cleared, walls-down.
        sim.bunker_wall_events.clear();
        assert!(sim.apply_command(
            "Americans",
            &Command::EjectBunker { bunker_id: 2 },
            Some(&rules),
            None,
            &BTreeMap::new(),
        ));
        assert_eq!(sim.substrate.entities.get(2).unwrap().bunker_occupant, None);
        let unit = sim.substrate.entities.get(1).unwrap();
        assert_eq!(unit.bunker_link, BunkerLink::None);
        assert!(unit.in_logic_vector, "occupant revealed on eject");
        assert_eq!(unit.mission.current, MissionType::Move);
        assert_eq!(
            sim.bunker_wall_events.iter().filter(|e| !e.up).count(),
            1,
            "one walls-down event on eject"
        );
    }
}
