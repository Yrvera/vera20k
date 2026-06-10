//! Context-sensitive order resolution — translates a screen click into game commands.
//!
//! Given a click position and the current selection, determines what command to issue:
//! move, attack, garrison, deploy, harvest, rally point, etc. This is the decision tree
//! that maps player intent to `Command` envelopes.
//!
//! Extracted from app_input.rs to separate order resolution from raw input handling.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use std::collections::HashMap;

use crate::app::AppState;
use crate::app_commands::preferred_local_owner;
use crate::app_entity_pick::{
    hover_target_at_point, pick_any_target_stable_id, pick_enemy_target_stable_id,
};
use crate::app_input::{
    emit_order_voice, is_alt_held, is_ctrl_held, is_shift_held, selected_stable_ids_sorted,
};
use crate::app_types::{HoverTargetKind, OrderMode};
use crate::map::entities::EntityCategory;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::intern::InternedId;
use crate::sim::movement::group_destination;

/// Attempt to issue a context-sensitive order at the given screen point.
///
/// Returns `true` if a command was queued (consuming the click), `false` if the
/// click should fall through to selection handling.
///
/// When `select_friendly_clicks` is true, clicks on friendly units/structures
/// return `false` so the caller can treat them as selection clicks instead.
pub(crate) fn try_queue_context_order_at_screen_point(
    state: &mut AppState,
    screen_x: f32,
    screen_y: f32,
    select_friendly_clicks: bool,
) -> bool {
    let (world_x, world_y) = crate::app_sim_tick::screen_point_to_world(state, screen_x, screen_y);
    let (target_rx, target_ry) =
        crate::app_sim_tick::screen_point_to_world_cell(state, screen_x, screen_y);
    let queue_mode: bool = is_shift_held(state);
    // Force-fire is Ctrl-only — Alt+Ctrl is attack-move, not force-fire
    // (gamemd What_Action_OnCell at 0x700706: Alt clears Ctrl flag).
    let force_fire: bool = is_ctrl_held(state) && !is_alt_held(state);
    let order_mode = state.queued_order_mode;
    let owner: String = preferred_local_owner(state).unwrap_or_else(|| "Americans".to_string());
    let owner_id: InternedId = state
        .simulation
        .as_ref()
        .and_then(|s| s.interner.get(&owner))
        .unwrap_or_default();

    let mut queued: Vec<CommandEnvelope> = Vec::new();
    let mut attack_voice = false;
    let mut consumed_order_mode = false;

    if let Some(sim) = &mut state.simulation {
        let execute_tick = sim.session.tick.saturating_add(sim.input_delay_ticks);
        let selected_ids: Vec<u64> = selected_stable_ids_sorted(sim.entities());
        if selected_ids.is_empty() {
            return false;
        }

        let mut selected_units: Vec<u64> = Vec::new();
        let mut selected_miner_ids: Vec<u64> = Vec::new();
        let mut structure_owner: Option<String> = None;
        let mut mobile_count: usize = 0;
        let mut _structure_count: usize = 0;

        for &sid in &selected_ids {
            let Some(entity) = sim.entities().get(sid) else {
                continue;
            };
            if entity.category == EntityCategory::Structure {
                _structure_count += 1;
                if structure_owner.is_none() {
                    structure_owner = Some(sim.interner.resolve(entity.owner).to_string());
                }
            } else {
                mobile_count += 1;
                selected_units.push(sid);
                if entity.miner.is_some() {
                    selected_miner_ids.push(sid);
                }
            }
        }
        selected_units.sort_unstable();
        let hover = hover_target_at_point(
            sim,
            world_x,
            world_y,
            &owner,
            state.sandbox_full_visibility,
            state.rules.as_ref(),
            &state.height_map,
            Some(&state.tactical_bridge_inverse_map),
        );

        let only_miners_selected = mobile_count > 0 && selected_miner_ids.len() == mobile_count;
        let clicked_friendly_refinery_id = (!force_fire)
            .then(|| {
                hover.as_ref().and_then(|target| {
                    if target.kind != HoverTargetKind::FriendlyStructure {
                        return None;
                    }
                    let rules = state.rules.as_ref()?;
                    sim.entities().get(target.stable_id).and_then(|e| {
                        rules
                            .is_refinery_type(sim.interner.resolve(e.type_ref))
                            .then_some(target.stable_id)
                    })
                })
            })
            .flatten();
        let clicked_friendly_refinery = clicked_friendly_refinery_id.is_some();

        // Check if the clicked cell has a resource node (ore/gems).
        let clicked_ore = !force_fire
            && sim
                .production
                .resource_nodes
                .get(&(target_rx, target_ry))
                .is_some_and(|n| n.remaining > 0);

        if clicked_friendly_refinery && only_miners_selected {
            for stable_id in selected_miner_ids {
                queued.push(CommandEnvelope::new(
                    owner_id,
                    execute_tick,
                    Command::MinerReturn {
                        entity_id: stable_id,
                        target_refinery_id: clicked_friendly_refinery_id,
                    },
                ));
            }
        } else if clicked_ore && !selected_miner_ids.is_empty() {
            // Direct miners to harvest the clicked ore cell.
            for &stable_id in &selected_miner_ids {
                queued.push(CommandEnvelope::new(
                    owner_id,
                    execute_tick,
                    Command::HarvestCell {
                        entity_id: stable_id,
                        target_rx,
                        target_ry,
                    },
                ));
            }
            // Non-miner units in selection just move to that cell.
            for &stable_id in &selected_units {
                if !selected_miner_ids.contains(&stable_id) {
                    queued.push(CommandEnvelope::new(
                        owner_id,
                        execute_tick,
                        Command::Move {
                            entity_id: stable_id,
                            target_rx,
                            target_ry,
                            queue: queue_mode,
                            group_id: None,
                        },
                    ));
                }
            }
        } else if let Some(struct_own) = structure_owner {
            let clicked_friendly = hover.as_ref().is_some_and(|target| {
                matches!(
                    target.kind,
                    HoverTargetKind::FriendlyUnit | HoverTargetKind::FriendlyStructure
                )
            });
            // Self-click on a deployable structure (garrisoned building → unload,
            // ConYard → undeploy). Must run before the friendly-click fallthrough
            // below — otherwise the click is treated as plain re-selection and the
            // deploy cursor's action is lost.
            if !force_fire && clicked_friendly {
                if let Some(target) = hover.as_ref() {
                    if selected_ids.contains(&target.stable_id) {
                        if let Some(entity) = sim.entities().get(target.stable_id) {
                            if entity.category == EntityCategory::Structure {
                                let obj = state
                                    .rules
                                    .as_ref()
                                    .and_then(|r| r.object(sim.interner.resolve(entity.type_ref)));
                                let cmd = if obj.map_or(false, |o| o.can_be_occupied)
                                    && entity.passenger_role.cargo().is_some_and(|c| !c.is_empty())
                                {
                                    Some(Command::UnloadPassengers {
                                        transport_id: target.stable_id,
                                    })
                                } else if state.rules.as_ref().is_some_and(|rules| {
                                    sim.should_show_undeploy_building_command(
                                        target.stable_id,
                                        rules,
                                    )
                                }) {
                                    Some(Command::UndeployBuilding {
                                        entity_id: target.stable_id,
                                    })
                                } else {
                                    None
                                };
                                if let Some(cmd) = cmd {
                                    queued.push(CommandEnvelope::new(owner_id, execute_tick, cmd));
                                    for cmd in queued {
                                        sim.pending_commands.push(cmd);
                                    }
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            if select_friendly_clicks && clicked_friendly {
                return false;
            }
            {
                // Set rally point for the structures.
                {
                    let struct_owner_id = sim.interner.get(&struct_own).unwrap_or(owner_id);
                    let producer_ids =
                        selected_rally_producer_ids(sim, &selected_ids, struct_owner_id);
                    queued.push(CommandEnvelope::new(
                        struct_owner_id,
                        execute_tick,
                        Command::SetRally {
                            owner: struct_owner_id,
                            rx: target_rx,
                            ry: target_ry,
                            producer_ids,
                        },
                    ));
                }
                // Also issue Move commands for any mobile units in the
                // selection — RA2 moves units AND sets rally when both
                // are selected.
                if mobile_count > 0 {
                    for &stable_id in &selected_units {
                        queued.push(CommandEnvelope::new(
                            owner_id,
                            execute_tick,
                            Command::Move {
                                entity_id: stable_id,
                                target_rx,
                                target_ry,
                                queue: queue_mode,
                                group_id: None,
                            },
                        ));
                    }
                }
            }
        } else {
            // Garrison entry uses the shared CanDock-equivalent predicate before
            // issuing EnterTransport commands.
            // are classified as EnemyStructure but are still garrisonable —
            if !force_fire {
                let garrison_target = hover.as_ref().map(|target| target.stable_id);
                if let Some(transport_id) = garrison_target {
                    let infantry_ids: Vec<u64> = selected_units
                        .iter()
                        .copied()
                        .filter(|&sid| {
                            state.rules.as_ref().is_some_and(|rules| {
                                crate::sim::passenger::can_entity_enter_garrison(
                                    sim,
                                    rules,
                                    sid,
                                    transport_id,
                                    state.path_grid.as_ref(),
                                )
                            })
                        })
                        .collect();
                    if !infantry_ids.is_empty() {
                        for pax_id in infantry_ids {
                            queued.push(CommandEnvelope::new(
                                owner_id,
                                execute_tick,
                                Command::EnterTransport {
                                    passenger_id: pax_id,
                                    transport_id,
                                },
                            ));
                        }
                        for cmd in queued {
                            sim.pending_commands.push(cmd);
                        }
                        emit_order_voice(state, "VoiceMove");
                        return true;
                    }
                }
            }

            // C4 plant: SEAL / Tanya / Psi-Corp Trooper clicking a CanC4 enemy
            // structure. Ordered before the engineer-capture branch so C4 takes
            // priority for any unit with both flags.
            if !force_fire {
                let c4_target = hover.as_ref().and_then(|target| {
                    if !matches!(target.kind, HoverTargetKind::EnemyStructure) {
                        return None;
                    }
                    let rules = state.rules.as_ref()?;
                    let building = sim.entities().get(target.stable_id)?;
                    let obj = rules.object(sim.interner.resolve(building.type_ref))?;
                    if !obj.can_c4 || obj.invisible_in_game {
                        return None;
                    }
                    // Reject IC'd target at issue time (matches gamemd's
                    // What_Action_OnObject vtable[+0x80] check).
                    if crate::sim::superweapon::invulnerability::is_invulnerable(
                        building.invulnerability.as_ref(),
                        sim.session.tick as u32,
                    ) {
                        return None;
                    }
                    Some(target.stable_id)
                });
                if let Some(building_id) = c4_target {
                    let c4_attackers: Vec<u64> = selected_units
                        .iter()
                        .copied()
                        .filter(|&sid| {
                            sim.entities().get(sid).is_some_and(|e| {
                                e.category == EntityCategory::Infantry
                                    && state
                                        .rules
                                        .as_ref()
                                        .and_then(|r| r.object(sim.interner.resolve(e.type_ref)))
                                        .map_or(false, |o| o.c4)
                            })
                        })
                        .collect();
                    if !c4_attackers.is_empty() {
                        for attacker_id in c4_attackers {
                            queued.push(CommandEnvelope::new(
                                owner_id,
                                execute_tick,
                                Command::PlantC4 {
                                    attacker_id,
                                    target_building_id: building_id,
                                },
                            ));
                        }
                        for cmd in queued {
                            sim.pending_commands.push(cmd);
                        }
                        // EVA voice for the plant order. Matches gamemd's
                        // VoiceSpecialAttack=SealSpecialAttack on [GHOST].
                        emit_order_voice(state, "VoiceSpecialAttack");
                        return true;
                    }
                }
            }

            // Engineer capture: engineer clicking a capturable enemy building.
            if !force_fire {
                let capture_target = hover.as_ref().and_then(|target| {
                    if !matches!(target.kind, HoverTargetKind::EnemyStructure) {
                        return None;
                    }
                    let rules = state.rules.as_ref()?;
                    let building = sim.entities().get(target.stable_id)?;
                    let btype_str = sim.interner.resolve(building.type_ref);
                    let bowner_str = sim.interner.resolve(building.owner);
                    let obj = rules.object(btype_str)?;
                    if !obj.capturable && !obj.bridge_repair_hut {
                        return None;
                    }
                    // Don't capture neutral garrisonable buildings — those use garrison entry.
                    if obj.can_be_occupied
                        && (bowner_str.eq_ignore_ascii_case("neutral")
                            || bowner_str.eq_ignore_ascii_case("special"))
                    {
                        return None;
                    }
                    Some(target.stable_id)
                });
                if let Some(building_id) = capture_target {
                    let engineer_ids: Vec<u64> = selected_units
                        .iter()
                        .copied()
                        .filter(|&sid| {
                            sim.entities().get(sid).is_some_and(|e| {
                                e.category == EntityCategory::Infantry
                                    && state
                                        .rules
                                        .as_ref()
                                        .and_then(|r| r.object(sim.interner.resolve(e.type_ref)))
                                        .map_or(false, |o| o.engineer)
                            })
                        })
                        .collect();
                    if !engineer_ids.is_empty() {
                        for eng_id in engineer_ids {
                            queued.push(CommandEnvelope::new(
                                owner_id,
                                execute_tick,
                                Command::CaptureBuilding {
                                    engineer_id: eng_id,
                                    target_building_id: building_id,
                                },
                            ));
                        }
                        for cmd in queued {
                            sim.pending_commands.push(cmd);
                        }
                        emit_order_voice(state, "VoiceMove");
                        return true;
                    }
                }
            }

            let clicked_friendly = hover.as_ref().is_some_and(|target| {
                matches!(
                    target.kind,
                    HoverTargetKind::FriendlyUnit | HoverTargetKind::FriendlyStructure
                )
            });
            // Deploy-on-self-click: clicking a selected deployable entity deploys/undeploys it.
            if clicked_friendly && !force_fire {
                if let Some(target) = hover.as_ref() {
                    if selected_ids.contains(&target.stable_id) {
                        if let Some(entity) = sim.entities().get(target.stable_id) {
                            let obj = state
                                .rules
                                .as_ref()
                                .and_then(|r| r.object(sim.interner.resolve(entity.type_ref)));
                            let cmd = if entity.category == EntityCategory::Structure {
                                // Garrisoned building → unload occupants.
                                if obj.map_or(false, |o| o.can_be_occupied)
                                    && entity.passenger_role.cargo().is_some_and(|c| !c.is_empty())
                                {
                                    Some(Command::UnloadPassengers {
                                        transport_id: target.stable_id,
                                    })
                                // ConYard → MCV
                                } else if state.rules.as_ref().is_some_and(|rules| {
                                    sim.should_show_undeploy_building_command(
                                        target.stable_id,
                                        rules,
                                    )
                                }) {
                                    Some(Command::UndeployBuilding {
                                        entity_id: target.stable_id,
                                    })
                                } else {
                                    None
                                }
                            } else if entity.category == EntityCategory::Infantry
                                && obj.map_or(false, |o| o.deploy_fire)
                            {
                                // Deploy-fire infantry (GI, GGI, etc.) → toggle deploy.
                                Some(Command::ToggleInfantryDeploy {
                                    entity_id: target.stable_id,
                                })
                            } else {
                                // MCV → ConYard
                                if obj.map_or(false, |o| o.deploys_into.is_some() || o.deployer) {
                                    Some(Command::DeployMcv {
                                        entity_id: target.stable_id,
                                    })
                                } else {
                                    None
                                }
                            };
                            if let Some(cmd) = cmd {
                                queued.push(CommandEnvelope::new(owner_id, execute_tick, cmd));
                                for cmd in queued {
                                    sim.pending_commands.push(cmd);
                                }
                                return true;
                            }
                        }
                    }
                }
            }
            if select_friendly_clicks && clicked_friendly && !force_fire {
                return false;
            }

            let attack_target: Option<u64> = if force_fire {
                pick_any_target_stable_id(
                    sim,
                    world_x,
                    world_y,
                    state.sandbox_full_visibility,
                    state.rules.as_ref(),
                    &state.height_map,
                    Some(&state.tactical_bridge_inverse_map),
                )
            } else {
                pick_enemy_target_stable_id(
                    sim,
                    world_x,
                    world_y,
                    &owner,
                    state.sandbox_full_visibility,
                    state.rules.as_ref(),
                    &state.height_map,
                    Some(&state.tactical_bridge_inverse_map),
                )
            };
            // --- Group destination distribution ---
            // When multiple units are selected for a Move/AttackMove, distribute
            // unique destination cells via radial spread (RA2 behavior) instead of
            // sending all units to the same cell.
            let group_destinations: HashMap<u64, (u16, u16)> = if selected_units.len() > 1
                && attack_target.is_none()
            {
                if let Some(grid) = state.path_grid.as_ref() {
                    let mut vehicle_ids: Vec<u64> = Vec::new();
                    let mut infantry_ids: Vec<u64> = Vec::new();
                    for &sid in &selected_units {
                        if let Some(entity) = sim.entities().get(sid) {
                            if entity.category == EntityCategory::Infantry {
                                infantry_ids.push(sid);
                            } else {
                                vehicle_ids.push(sid);
                            }
                        }
                    }
                    let center: (u16, u16) = crate::app_sim_tick::nearest_walkable_cell_layered(
                        grid,
                        (target_rx, target_ry),
                        12,
                    )
                    .unwrap_or((target_rx, target_ry));
                    let assignments = group_destination::distribute_group_destinations(
                        grid,
                        center,
                        &vehicle_ids,
                        &infantry_ids,
                    );
                    assignments
                        .into_iter()
                        .map(|(id, rx, ry)| (id, (rx, ry)))
                        .collect()
                } else {
                    HashMap::new()
                }
            } else {
                HashMap::new()
            };

            // Assign a shared group_id when multiple units move together.
            // The movement system uses this to sync speed to the slowest member.
            let move_group_id: Option<u32> = if selected_units.len() > 1 && attack_target.is_none()
            {
                Some(execute_tick as u32)
            } else {
                None
            };

            // Force-fire on a shrouded cell is rejected — gamemd can't target
            // what it can't see (FUN_005023b0 shroud check at 0x00700600).
            // Computed once outside the per-unit loop.
            let cell_is_shrouded: bool = if force_fire && !state.sandbox_full_visibility {
                let owner_id_for_fog = sim.interner.get(&owner).unwrap_or_default();
                !sim.fog
                    .is_cell_revealed(owner_id_for_fog, target_rx, target_ry)
                    || sim
                        .fog
                        .is_cell_gap_covered(owner_id_for_fog, target_rx, target_ry)
            } else {
                false
            };

            for stable_id in selected_units {
                let payload = if let Some(target_id) = attack_target {
                    if force_fire {
                        Command::ForceAttack {
                            attacker_id: stable_id,
                            target_id,
                        }
                    } else if order_mode != OrderMode::Guard {
                        Command::Attack {
                            attacker_id: stable_id,
                            target_id,
                        }
                    } else {
                        Command::Guard {
                            entity_id: stable_id,
                            target_id: Some(target_id),
                        }
                    }
                } else if force_fire && !cell_is_shrouded {
                    // Force-fire on empty terrain: per-unit dispatch matching
                    // gamemd What_Action_OnCell — armed mobile units fire at
                    // the cell, unarmed (Engineer/Harvester/MCV) fall through
                    // to plain Move. Skips group_destinations spread because
                    // gamemd's DispatchMultiUnitOrder uses identical cell
                    // coords for every selected unit.
                    let unit_armed = sim
                        .entities()
                        .get(stable_id)
                        .and_then(|e| {
                            let type_str = sim.interner.resolve(e.type_ref);
                            state
                                .rules
                                .as_ref()
                                .and_then(|r| r.object(type_str))
                                .map(|obj| obj.primary.is_some() || obj.secondary.is_some())
                        })
                        .unwrap_or(false);
                    let is_harvester = sim
                        .entities()
                        .get(stable_id)
                        .is_some_and(|e| e.miner.is_some());

                    if unit_armed && !is_harvester {
                        Command::ForceAttackCell {
                            attacker_id: stable_id,
                            target_rx,
                            target_ry,
                        }
                    } else {
                        // Unarmed fall-through to plain Move. Reuse the same
                        // walkability fallback the regular Move path uses
                        // (lines below) — if the cell is unwalkable, route to
                        // nearest walkable cell so an Engineer ctrl-clicking
                        // water doesn't silently stall.
                        let goal: (u16, u16) = {
                            let mut g = (target_rx, target_ry);
                            if let Some(grid) = state.path_grid.as_ref() {
                                if !crate::app_sim_tick::is_any_layer_walkable(grid, g.0, g.1) {
                                    if let Some(nearest) =
                                        crate::app_sim_tick::nearest_walkable_cell_layered(
                                            grid, g, 12,
                                        )
                                    {
                                        g = nearest;
                                    }
                                }
                            }
                            g
                        };
                        Command::Move {
                            entity_id: stable_id,
                            target_rx: goal.0,
                            target_ry: goal.1,
                            queue: queue_mode,
                            group_id: None,
                        }
                    }
                } else {
                    match order_mode {
                        OrderMode::Move | OrderMode::AttackMove => {
                            let goal: (u16, u16) = group_destinations
                                .get(&stable_id)
                                .copied()
                                .unwrap_or_else(|| {
                                    let mut g = (target_rx, target_ry);
                                    if let Some(grid) = state.path_grid.as_ref() {
                                        if !crate::app_sim_tick::is_any_layer_walkable(
                                            grid, g.0, g.1,
                                        ) {
                                            if let Some(nearest) =
                                                crate::app_sim_tick::nearest_walkable_cell_layered(
                                                    grid, g, 12,
                                                )
                                            {
                                                g = nearest;
                                            }
                                        }
                                    }
                                    g
                                });
                            if order_mode == OrderMode::AttackMove {
                                Command::AttackMove {
                                    entity_id: stable_id,
                                    target_rx: goal.0,
                                    target_ry: goal.1,
                                    queue: queue_mode,
                                }
                            } else {
                                Command::Move {
                                    entity_id: stable_id,
                                    target_rx: goal.0,
                                    target_ry: goal.1,
                                    queue: queue_mode,
                                    group_id: move_group_id,
                                }
                            }
                        }
                        OrderMode::Guard => Command::Guard {
                            entity_id: stable_id,
                            target_id: None,
                        },
                    }
                };
                queued.push(CommandEnvelope::new(owner_id, execute_tick, payload));
            }
            if !queued.is_empty() {
                // Treat force-fire-cell as an attack-voice trigger too — the
                // player gave an attack order even though no entity was hit.
                attack_voice = attack_target.is_some() || (force_fire && !cell_is_shrouded);
                consumed_order_mode = true;
            }
        }
    }

    if queued.is_empty() {
        return false;
    }
    if consumed_order_mode && state.queued_order_mode != OrderMode::Move {
        state.queued_order_mode = OrderMode::Move;
    }
    if attack_voice {
        emit_order_voice(state, "VoiceAttack");
    } else {
        emit_order_voice(state, "VoiceMove");
    }
    // Record target lines for visual feedback before pushing to sim queue.
    let current_tick = state.simulation.as_ref().map_or(0, |s| s.session.tick);
    crate::app_target_lines::record_command_lines(&mut state.target_lines, &queued, current_tick);

    if let Some(sim) = &mut state.simulation {
        sim.pending_commands.extend(queued);
    }
    true
}

fn selected_rally_producer_ids(
    sim: &crate::sim::world::Simulation,
    selected_ids: &[u64],
    owner: InternedId,
) -> Vec<u64> {
    let mut producer_ids: Vec<u64> = selected_ids
        .iter()
        .copied()
        .filter(|stable_id| {
            sim.entities().get(*stable_id).is_some_and(|entity| {
                entity.category == EntityCategory::Structure && entity.owner == owner
            })
        })
        .collect();
    producer_ids.sort_unstable();
    producer_ids.dedup();
    producer_ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::world::Simulation;

    #[test]
    fn right_click_structure_selection_sends_rally_producer_ids() {
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let factory_type = sim.interner.intern("GAWEAP");
        let tank_type = sim.interner.intern("MTNK");
        sim.entities_mut().insert(GameEntity::new(
            1,
            10,
            10,
            0,
            0,
            owner,
            Health {
                current: 1000,
                max: 1000,
            },
            factory_type,
            EntityCategory::Structure,
            0,
            5,
            false,
        ));
        sim.entities_mut().insert(GameEntity::new(
            2,
            11,
            10,
            0,
            0,
            owner,
            Health {
                current: 300,
                max: 300,
            },
            tank_type,
            EntityCategory::Unit,
            0,
            5,
            true,
        ));

        let producer_ids = selected_rally_producer_ids(&sim, &[2, 1], owner);

        assert_eq!(producer_ids, vec![1]);
    }
}
