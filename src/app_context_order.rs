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

use crate::app::AppState;
use crate::app_commands::preferred_local_owner;
use crate::app_entity_pick::{
    hover_target_at_point, pick_any_target_stable_id, pick_enemy_target_stable_id,
};
use crate::app_input::{is_alt_held, is_ctrl_held, is_shift_held, selected_stable_ids_sorted};
use crate::app_types::{HoverTargetKind, OrderMode};
use crate::map::entities::EntityCategory;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::intern::InternedId;

/// The verb a Ctrl / Shift / Alt chord selects for a tactical click.
///
/// Retail contract, read from `TechnoClass::What_Action_OnCell`,
/// `TechnoClass::What_Action_OnObject` and `TechnoClass::Player_Send_Command`:
///
/// * **Ctrl alone → force fire.** The cell path takes the attack branch with no
///   enemy under the cursor; the object path drops the ally guard.
/// * **Alt alone → force move.** The object path returns the plain Move action
///   instead of whatever context action the object would resolve, and the cell
///   path returns Move without running its occupancy probe.
/// * **Ctrl+Shift → attack move.** `What_Action_OnCell` cancels Shift and Ctrl
///   against each other, so the action itself resolves as an ordinary
///   Move/Attack; `Player_Send_Command` then promotes a committed Move or Attack
///   mission to attack-move whenever the chord test passes. That test reads the
///   raw key state, so the chord still fires when Alt is also held — and because
///   the cancel has already cleared Ctrl, the Ctrl+Alt guard-area gate cannot.
/// * **Ctrl+Alt → guard area** (patrol when the cell carries a waypoint).
/// * **Shift alone → no order at all in retail.** On an object it returns the
///   add-to-selection action, which the object click handler has no case for and
///   therefore sends no mission; on a cell it returns the plain Move action, an
///   ordinary immediate move. Retail's *deferred*-order verb is Planning Mode —
///   a separate bindable command class with its own event opcodes — not a
///   modifier. VERA has no Planning Mode, so Shift keeps VERA's order-queue
///   verb: VERA-internal, gamemd equivalent (Planning Mode) UNIMPLEMENTED.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrderModifier {
    /// No modifier held — the object/cell context action stands.
    Normal,
    /// Ctrl — force fire.
    ForceFire,
    /// Alt — force move.
    ForceMove,
    /// Ctrl+Shift — attack move.
    AttackMove,
    /// Ctrl+Alt — guard area.
    GuardArea,
    /// Shift — VERA's order queue. No retail equivalent on this modifier.
    Queue,
}

/// Resolve the retail modifier verb from the three held-key states.
///
/// Ordering mirrors the binary: the Ctrl+Shift chord is tested against the raw
/// key state before anything else, then the Ctrl+Alt guard-area gate, then the
/// single-modifier branches. Shift outranks Alt because the cell path returns on
/// Shift before it ever reaches the Alt test, and the object path returns the
/// add-to-selection action before the Alt branch.
pub(crate) fn resolve_order_modifiers(ctrl: bool, shift: bool, alt: bool) -> OrderModifier {
    if ctrl && shift {
        OrderModifier::AttackMove
    } else if ctrl && alt {
        OrderModifier::GuardArea
    } else if ctrl {
        OrderModifier::ForceFire
    } else if shift {
        OrderModifier::Queue
    } else if alt {
        OrderModifier::ForceMove
    } else {
        OrderModifier::Normal
    }
}

/// INI voice key for the mission a command commits.
///
/// `TechnoClass::Player_Send_Command` dispatches the order-ack line by mission
/// number: Harvest, Attack, Move (shared with attack-move), Enter, Capture and
/// Unload each have their own slot, and every other mission falls to a default
/// branch that draws a random entry from the type's `VoiceSpecialAttack` list.
///
/// Two retail slots have no VERA counterpart yet:
/// * Capture plays `VoiceCapture` and falls back to the Enter slot when the type
///   has no `VoiceCapture=`; VERA does not parse that key, so capture uses the
///   fallback unconditionally.
/// * Deploy/unload plays `VoiceDeploy` / `VoiceUndeploy`, neither of which VERA
///   parses — those orders stay silent.
///
/// The self-click halt is silent in retail as well: it builds a detonate event
/// directly instead of sending a mission, so the voice dispatch never runs.
fn order_voice_key(command: &Command) -> Option<&'static str> {
    match command {
        Command::Move { .. } | Command::AttackMove { .. } => Some("VoiceMove"),
        Command::Attack { .. } | Command::ForceAttack { .. } | Command::ForceAttackCell { .. } => {
            Some("VoiceAttack")
        }
        Command::HarvestCell { .. } => Some("VoiceHarvest"),
        Command::MinerReturn { .. }
        | Command::EnterTransport { .. }
        | Command::RepairAtDepot { .. }
        | Command::EnterBunker { .. }
        | Command::CaptureBuilding { .. } => Some("VoiceEnter"),
        // Sabotage and area guard have no dedicated slot, so retail takes the
        // default branch and speaks a VoiceSpecialAttack line.
        Command::PlantC4 { .. } | Command::Guard { .. } => Some("VoiceSpecialAttack"),
        _ => None,
    }
}

/// The entity a command acts on, when the command targets exactly one.
///
/// Used to find which queued order belongs to the speaking object.
fn command_actor_id(command: &Command) -> Option<u64> {
    match command {
        Command::Move { entity_id, .. }
        | Command::AttackMove { entity_id, .. }
        | Command::Guard { entity_id, .. }
        | Command::HarvestCell { entity_id, .. }
        | Command::MinerReturn { entity_id, .. }
        | Command::RepairAtDepot { entity_id, .. }
        | Command::ToggleInfantryDeploy { entity_id }
        | Command::DeployMcv { entity_id }
        | Command::UndeployBuilding { entity_id }
        | Command::Stop { entity_id } => Some(*entity_id),
        Command::Attack { attacker_id, .. }
        | Command::ForceAttack { attacker_id, .. }
        | Command::ForceAttackCell { attacker_id, .. }
        | Command::PlantC4 { attacker_id, .. } => Some(*attacker_id),
        Command::EnterTransport { passenger_id, .. } => Some(*passenger_id),
        Command::EnterBunker { unit_id, .. } => Some(*unit_id),
        Command::CaptureBuilding { engineer_id, .. } => Some(*engineer_id),
        Command::UnloadPassengers { transport_id } => Some(*transport_id),
        Command::EjectBunker { bunker_id } => Some(*bunker_id),
        _ => None,
    }
}

/// Play the single order-ack line for a batch of freshly queued orders.
///
/// Retail's dispatch loop clears the voice-enable flag at the end of *every*
/// iteration and restores it once after the loop, so exactly one object speaks —
/// the first entry of the selection array — and it speaks the line for the
/// mission *it* resolved, not an order-wide line. If that first object resolved
/// no order (its action mapped to a cursor-only code) nothing is spoken.
fn emit_resolved_order_voice(state: &mut AppState, speaker_id: u64, queued: &[CommandEnvelope]) {
    let Some(voice_field) = queued
        .iter()
        .find(|env| command_actor_id(&env.payload) == Some(speaker_id))
        .and_then(|env| order_voice_key(&env.payload))
    else {
        return;
    };
    emit_entity_order_voice(state, speaker_id, voice_field);
}

/// Play one entity's voice line for the given `Voice*` INI key.
///
/// The superseded app-layer order-voice helper always spoke the lowest-`stable_id` selected
/// entity; retail speaks the object that resolved the order, so order resolution
/// needs to name the speaker explicitly.
fn emit_entity_order_voice(state: &mut AppState, speaker_id: u64, voice_field: &str) {
    let Some(sim) = &state.simulation else { return };
    let Some(rules) = &state.rules else { return };
    let Some(entity) = sim.entities().get(speaker_id) else {
        return;
    };
    let Some(obj) = rules.object(sim.interner.resolve(entity.type_ref)) else {
        return;
    };
    let voice_id: Option<&String> = match voice_field {
        "VoiceMove" => obj.voice_move.as_ref(),
        "VoiceAttack" => obj.voice_attack.as_ref(),
        "VoiceHarvest" => obj.voice_harvest.as_ref(),
        "VoiceEnter" => obj.voice_enter.as_ref(),
        "VoiceSpecialAttack" => obj.voice_special_attack.as_ref(),
        _ => None,
    };
    let Some(id) = voice_id else { return };
    let event = if voice_field == "VoiceAttack" {
        crate::audio::events::GameSoundEvent::UnitAttackOrder {
            sound_id: id.clone(),
        }
    } else {
        crate::audio::events::GameSoundEvent::UnitMoveOrder {
            sound_id: id.clone(),
        }
    };
    state.sound_events.push(event);
}

/// Commit a resolved order batch: one voice line, the action lines, the queue.
///
/// Every exit from order resolution goes through here so the single-speaker rule
/// holds for the capability branches (garrison, C4, capture, depot, bunker,
/// deploy) as well as for the move/attack tail.
fn finish_order(
    state: &mut AppState,
    queued: Vec<CommandEnvelope>,
    speaker_id: Option<u64>,
) -> bool {
    if queued.is_empty() {
        return false;
    }
    if let Some(speaker_id) = speaker_id {
        emit_resolved_order_voice(state, speaker_id, &queued);
    }
    let current_tick = state.simulation.as_ref().map_or(0, |s| s.session.tick);
    crate::app_target_lines::record_command_lines(&mut state.target_lines, &queued, current_tick);
    if let Some(sim) = &mut state.simulation {
        sim.pending_commands.extend(queued);
    }
    true
}

/// Every selected mobile can accept an attack-move order.
///
/// The chord test walks the whole selection and fails the chord if *any*
/// selected object answers the per-type "can attack move" predicate false.
/// VERA's stand-in is the same armed-and-not-a-harvester test the force-fire
/// terrain path uses. The predicate's result for selected *structures* is
/// gamemd-UNCHECKED, so only mobiles are tested here.
fn selection_can_attack_move(
    sim: &crate::sim::world::Simulation,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    selected_units: &[u64],
) -> bool {
    if selected_units.is_empty() {
        return false;
    }
    selected_units.iter().all(|&sid| {
        sim.entities().get(sid).is_some_and(|e| {
            if e.miner.is_some() {
                return false;
            }
            rules
                .and_then(|r| r.object(sim.interner.resolve(e.type_ref)))
                .is_some_and(|obj| obj.primary.is_some() || obj.secondary.is_some())
        })
    })
}

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
    // Retail modifier map: Ctrl = force fire, Alt = force move,
    // Ctrl+Shift = attack move, Ctrl+Alt = guard area. Shift alone has no retail
    // order semantics at all and carries VERA's order queue instead — see
    // `OrderModifier` for the full derivation.
    let mut modifier = resolve_order_modifiers(
        is_ctrl_held(state),
        is_shift_held(state),
        is_alt_held(state),
    );
    let order_mode = state.queued_order_mode;
    let owner: String = preferred_local_owner(state).unwrap_or_else(|| "Americans".to_string());
    let owner_id: InternedId = state
        .simulation
        .as_ref()
        .and_then(|s| s.interner.get(&owner))
        .unwrap_or_default();

    let mut queued: Vec<CommandEnvelope> = Vec::new();
    let mut consumed_order_mode = false;
    // The one object that speaks the order-ack line. Retail lets only the first
    // entry of the selection array speak; VERA's selection order is stable-id
    // ascending, so this is its first selected entity.
    let mut speaker_id: Option<u64> = None;

    if let Some(sim) = &mut state.simulation {
        let execute_tick = sim.session.tick;
        let selected_ids: Vec<u64> = selected_stable_ids_sorted(sim.entities());
        if selected_ids.is_empty() {
            return false;
        }
        speaker_id = selected_ids.first().copied();

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

        // The chord test fails — and the order resolves normally — unless every
        // selected object can accept an attack-move order.
        if modifier == OrderModifier::AttackMove
            && !selection_can_attack_move(sim, state.rules.as_ref(), &selected_units)
        {
            modifier = OrderModifier::Normal;
        }
        let queue_mode: bool = modifier == OrderModifier::Queue;
        let force_fire: bool = modifier == OrderModifier::ForceFire;
        let force_move: bool = modifier == OrderModifier::ForceMove;
        // Force fire, force move and guard area each replace the object's own
        // context action outright, so the capability branches below (miner
        // return, garrison, C4, engineer capture, depot, bunker, self-click
        // deploy) are skipped for them. The attack-move chord does not: it
        // cancels both its modifiers before the action resolves and only
        // promotes the *committed* mission afterwards, so a chorded click on a
        // capturable building still captures.
        let context_actions_enabled: bool = matches!(
            modifier,
            OrderModifier::Normal | OrderModifier::Queue | OrderModifier::AttackMove
        );
        // The ore/gem harvest action hangs off the *cell* action, which stays
        // Move under Alt and under the cancelled chord but is replaced by force
        // fire and guard area.
        let cell_context_enabled: bool = !force_fire && modifier != OrderModifier::GuardArea;
        // A held chord overrides the sticky sidebar order mode for this click.
        let order_mode = match modifier {
            OrderModifier::AttackMove => OrderMode::AttackMove,
            OrderModifier::GuardArea => OrderMode::Guard,
            _ => order_mode,
        };

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
        let clicked_friendly_refinery_id = context_actions_enabled
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
        let clicked_ore = cell_context_enabled
            && match (
                sim.overlay_grid.as_ref(),
                state.overlay_registry.as_ref(),
                state.rules.as_ref(),
            ) {
                (Some(grid), Some(registry), Some(rules)) if !rules.tiberium_types.is_empty() => {
                    crate::sim::tiberium::tiberium_cell_view(
                        grid,
                        registry,
                        &rules.tiberium_types,
                        (target_rx, target_ry),
                    )
                    .is_some()
                }
                _ => sim
                    .production
                    .resource_nodes
                    .get(&(target_rx, target_ry))
                    .is_some_and(|node| node.remaining > 0),
            };

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
            // The manual return order commits mission Enter, so its ack is the
            // VoiceEnter slot (e.g. CMIN ChronoMinerReturn) — resolved from the
            // command itself by `order_voice_key`.
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
            // The harvest order commits mission Harvest, so its ack is the
            // VoiceHarvest slot (e.g. CMIN ChronoMinerHarvest); a non-miner in
            // the same selection commits Move and would speak VoiceMove.
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
            if context_actions_enabled && clicked_friendly {
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
                                } else if entity.bunker_occupant.is_some() {
                                    // Own occupied tank bunker → eject the installed unit.
                                    Some(Command::EjectBunker {
                                        bunker_id: target.stable_id,
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
                                    return finish_order(state, queued, speaker_id);
                                }
                            }
                        }
                    }
                }
            }
            if select_friendly_clicks && clicked_friendly && context_actions_enabled {
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
            if context_actions_enabled {
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
                        return finish_order(state, queued, speaker_id);
                    }
                }
            }

            // C4 plant: SEAL / Tanya / Psi-Corp Trooper clicking a CanC4 enemy
            // structure. Ordered before the engineer-capture branch so C4 takes
            // priority for any unit with both flags.
            if context_actions_enabled {
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
                        // Sabotage has no dedicated voice slot in retail, so
                        // `order_voice_key` routes it to VoiceSpecialAttack.
                        return finish_order(state, queued, speaker_id);
                    }
                }
            }

            // Engineer capture: engineer clicking a capturable enemy building.
            if context_actions_enabled {
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
                        return finish_order(state, queued, speaker_id);
                    }
                }
            }

            // Service depot: damaged own vehicles clicking an own UnitRepair
            // building drive to the depot and auto-repair. Ordered before the
            // friendly-fallthrough so the click isn't consumed as re-selection.
            if context_actions_enabled {
                let depot_target = hover.as_ref().and_then(|target| {
                    if !matches!(target.kind, HoverTargetKind::FriendlyStructure) {
                        return None;
                    }
                    let rules = state.rules.as_ref()?;
                    let building = sim.entities().get(target.stable_id)?;
                    let obj = rules.object(sim.interner.resolve(building.type_ref))?;
                    obj.unit_repair.then_some(target.stable_id)
                });
                if let Some(depot_id) = depot_target {
                    let repair_ids: Vec<u64> = selected_units
                        .iter()
                        .copied()
                        .filter(|&sid| {
                            sim.entities().get(sid).is_some_and(|e| {
                                e.category == EntityCategory::Unit
                                    && e.health.current < e.health.max
                                    && !e.is_deployed()
                            })
                        })
                        .collect();
                    if !repair_ids.is_empty() {
                        for unit_id in repair_ids {
                            queued.push(CommandEnvelope::new(
                                owner_id,
                                execute_tick,
                                Command::RepairAtDepot {
                                    entity_id: unit_id,
                                    depot_id,
                                },
                            ));
                        }
                        return finish_order(state, queued, speaker_id);
                    }
                }
            }

            // Tank bunker: an own bunkerable vehicle clicking an own EMPTY tank
            // bunker installs into it. The bunker holds one unit, so only the
            // first eligible vehicle is sent. Occupied bunkers are ejected via
            // the self-click path below.
            if context_actions_enabled {
                let bunker_target = hover.as_ref().and_then(|target| {
                    if !matches!(target.kind, HoverTargetKind::FriendlyStructure) {
                        return None;
                    }
                    let building = sim.entities().get(target.stable_id)?;
                    (building.bunker_runtime.is_some() && building.bunker_occupant.is_none())
                        .then_some(target.stable_id)
                });
                if let Some(bunker_id) = bunker_target {
                    let unit_id = selected_units.iter().copied().find(|&sid| {
                        sim.entities().get(sid).is_some_and(|e| !e.is_deployed())
                            && state.rules.as_ref().is_some_and(|rules| {
                                crate::sim::docking::bunker_link::can_auto_deploy_here(
                                    sim, sid, rules,
                                )
                            })
                    });
                    if let Some(unit_id) = unit_id {
                        queued.push(CommandEnvelope::new(
                            owner_id,
                            execute_tick,
                            Command::EnterBunker { unit_id, bunker_id },
                        ));
                        return finish_order(state, queued, speaker_id);
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
            if clicked_friendly && context_actions_enabled {
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
                                return finish_order(state, queued, speaker_id);
                            }
                        }
                    }
                }
            }
            if select_friendly_clicks && clicked_friendly && context_actions_enabled {
                return false;
            }

            // Alt force-move: the object path returns the plain Move action, so
            // nothing under the cursor is treated as a target and the
            // destination becomes the clicked object's own cell (retail resolves
            // that cell, then falls back to a nearby passable one — the Move
            // payload below already routes through the same fallback).
            let (target_rx, target_ry) = if force_move {
                hover
                    .as_ref()
                    .and_then(|t| sim.entities().get(t.stable_id))
                    .map_or((target_rx, target_ry), |e| (e.position.rx, e.position.ry))
            } else {
                (target_rx, target_ry)
            };

            let attack_target: Option<u64> = if force_move {
                None
            } else if force_fire {
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
            // Assign a shared group_id when multiple units move together.
            // The movement system uses this to sync speed to the slowest member.
            let move_group_id: Option<u32> = if selected_units.len() > 1 && attack_target.is_none()
            {
                Some(execute_tick as u32)
            } else {
                None
            };

            // VERA-internal: force-fire on a shrouded cell is rejected here.
            // gamemd equivalent CONTRADICTS this — the FootClass shroud wrapper
            // around What_Action_OnCell explicitly preserves the force-fire
            // action code through the shroud, and the function this gate's old
            // comment cited as a "shroud check" is in fact the waypoint lookup.
            // Left in place because removing it changes click routing; recorded
            // as a DRIFT for its own slice. Computed once outside the loop.
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
                    // to plain Move.
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
    finish_order(state, queued, speaker_id)
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
        sim.entities_mut()
            .insert(GameEntity::new_at_frame_zero_for_test(
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
        sim.entities_mut()
            .insert(GameEntity::new_at_frame_zero_for_test(
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

    /// The retail modifier map, one row per chord. Ctrl force-fires, Alt forces
    /// a move, Ctrl+Shift is attack move and Ctrl+Alt is guard area.
    #[test]
    fn modifier_map_matches_retail_chords() {
        // (ctrl, shift, alt) -> verb
        assert_eq!(
            resolve_order_modifiers(false, false, false),
            OrderModifier::Normal
        );
        assert_eq!(
            resolve_order_modifiers(true, false, false),
            OrderModifier::ForceFire
        );
        assert_eq!(
            resolve_order_modifiers(false, false, true),
            OrderModifier::ForceMove
        );
        assert_eq!(
            resolve_order_modifiers(true, true, false),
            OrderModifier::AttackMove
        );
        assert_eq!(
            resolve_order_modifiers(true, false, true),
            OrderModifier::GuardArea
        );
        assert_eq!(
            resolve_order_modifiers(false, true, false),
            OrderModifier::Queue
        );
    }

    /// The chord test reads the raw key state, so Alt does not defeat it — and
    /// because Shift and Ctrl have already cancelled each other by the time the
    /// guard-area gate is evaluated, Ctrl+Shift+Alt is attack move, not guard.
    #[test]
    fn attack_move_chord_survives_a_held_alt() {
        assert_eq!(
            resolve_order_modifiers(true, true, true),
            OrderModifier::AttackMove
        );
    }

    /// Shift outranks Alt: the cell path returns on Shift before it reaches the
    /// Alt test, and the object path returns the add-to-selection action first.
    #[test]
    fn shift_outranks_alt_without_ctrl() {
        assert_eq!(
            resolve_order_modifiers(false, true, true),
            OrderModifier::Queue
        );
    }

    /// Each order speaks the voice slot of the mission it commits, and missions
    /// with no dedicated slot fall to the VoiceSpecialAttack default branch.
    #[test]
    fn order_voice_key_follows_the_committed_mission() {
        assert_eq!(
            order_voice_key(&Command::Move {
                entity_id: 1,
                target_rx: 0,
                target_ry: 0,
                queue: false,
                group_id: None,
            }),
            Some("VoiceMove")
        );
        // Attack-move commits through the Move voice slot, not the Attack one.
        assert_eq!(
            order_voice_key(&Command::AttackMove {
                entity_id: 1,
                target_rx: 0,
                target_ry: 0,
                queue: false,
            }),
            Some("VoiceMove")
        );
        assert_eq!(
            order_voice_key(&Command::Attack {
                attacker_id: 1,
                target_id: 2,
            }),
            Some("VoiceAttack")
        );
        assert_eq!(
            order_voice_key(&Command::HarvestCell {
                entity_id: 1,
                target_rx: 0,
                target_ry: 0,
            }),
            Some("VoiceHarvest")
        );
        assert_eq!(
            order_voice_key(&Command::EnterTransport {
                passenger_id: 1,
                transport_id: 2,
            }),
            Some("VoiceEnter")
        );
        // Area guard and sabotage have no slot of their own.
        assert_eq!(
            order_voice_key(&Command::Guard {
                entity_id: 1,
                target_id: None,
            }),
            Some("VoiceSpecialAttack")
        );
        assert_eq!(
            order_voice_key(&Command::PlantC4 {
                attacker_id: 1,
                target_building_id: 2,
            }),
            Some("VoiceSpecialAttack")
        );
    }

    fn chord_rules() -> crate::rules::ruleset::RuleSet {
        let ini = crate::rules::ini_parser::IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             0=MTNK\n\
             1=HARV\n\
             2=SREF\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [MTNK]\n\
             Strength=300\n\
             Primary=105mm\n\
             [HARV]\n\
             Strength=1000\n\
             Harvester=yes\n\
             Primary=105mm\n\
             [SREF]\n\
             Strength=200\n\
             Secondary=105mm\n\
             [WeaponTypes]\n\
             0=105mm\n\
             [105mm]\n\
             Damage=60\n\
             Range=5\n",
        );
        crate::rules::ruleset::RuleSet::from_ini(&ini).expect("chord rules")
    }

    /// The chord requires *every* selected object to accept attack-move; one
    /// harvester in the selection makes it inert and the order resolves plainly.
    #[test]
    fn attack_move_chord_requires_every_selected_mobile() {
        let mut rules = chord_rules();
        let mut sim = Simulation::new();
        rules.resolve_bridge_warheads(&mut sim.interner);
        let height_map: std::collections::BTreeMap<(u16, u16), u8> =
            std::collections::BTreeMap::new();

        let tank = sim
            .spawn_object("MTNK", "Americans", 5, 5, 0, &rules, &height_map)
            .expect("tank");
        let miner = sim
            .spawn_object("HARV", "Americans", 6, 5, 0, &rules, &height_map)
            .expect("miner");
        // A secondary-only unit still counts as armed.
        let arty = sim
            .spawn_object("SREF", "Americans", 7, 5, 0, &rules, &height_map)
            .expect("secondary-only unit");

        assert!(selection_can_attack_move(&sim, Some(&rules), &[tank, arty]));
        assert!(!selection_can_attack_move(
            &sim,
            Some(&rules),
            &[tank, miner]
        ));
        // An empty selection cannot attack-move either.
        assert!(!selection_can_attack_move(&sim, Some(&rules), &[]));
    }
}
