//! Cursor feedback analysis and software cursor frame selection.
//!
//! Determines what cursor state to show based on hover target, selection,
//! and game mode. Extracted from app_ui_overlays.rs for file-size limits.

use std::time::Instant;

use crate::app::AppState;
use crate::app_commands::preferred_local_owner_name;
use crate::app_instances::CellVisibilityState;
use crate::app_types::{
    CursorFeedbackKind, CursorId, HoverTargetKind, ScrollDir, SoftwareCursorFrame,
    SoftwareCursorSequence,
};
use crate::sim::combat;

pub(crate) fn current_cursor_feedback_kind(state: &AppState) -> Option<CursorFeedbackKind> {
    // The active band is the outermost pixel of the whole window, including
    // the sidebar. It wins even when that pixel overlaps a sidebar/minimap hit.
    if let Some((dir, blocked)) = crate::app_camera::edge_scroll_cursor_state(state) {
        return Some(if blocked {
            CursorFeedbackKind::ScrollBlocked(dir)
        } else {
            CursorFeedbackKind::Scroll(dir)
        });
    }
    if state.minimap_dragging || is_cursor_over_minimap(state) {
        // Show the minimap-specific Move cursor when hovering over the minimap
        // (reference §7.4 — MiniFrame/MiniCount for the Move cursor = frames 42–51).
        return Some(CursorFeedbackKind::MinimapMove);
    }
    if current_sidebar_view_hit(state) {
        return None;
    }
    // Superweapon targeting cursor takes precedence over building placement.
    // Sidebar/minimap hits are already short-circuited above, so the SW reticle
    // only renders on the tactical map.
    if let Some(section) = state.armed_super_weapon_type() {
        let cursor_id = state
            .rules
            .as_ref()
            .and_then(|r| r.super_weapon(section))
            .and_then(|sw| sw.action.as_deref())
            .and_then(super_weapon_cursor_id)
            .unwrap_or(CursorId::Default);
        return Some(CursorFeedbackKind::SuperWeaponTarget(cursor_id));
    }
    if let Some(preview) = state.building_placement_preview.as_ref() {
        return Some(if preview.valid {
            CursorFeedbackKind::PlaceValid
        } else {
            CursorFeedbackKind::PlaceInvalid
        });
    }
    if state.armed_building_type().is_some() {
        return Some(CursorFeedbackKind::Invalid);
    }
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
        return None;
    };
    // Repair / Sell cursor modes take over the tactical map regardless of
    // selection — the wrench/dollar shows over own buildings, no-repair/no-sell
    // elsewhere. Placed before the empty-selection early return below because
    // gamemd shows these cursors even with nothing selected.
    let (repair_mode, sell_mode) = crate::app_sidebar_render::current_sidebar_view(state)
        .map(|view| (view.repair_button.active, view.sell_button.active))
        .unwrap_or_default();
    if repair_mode || sell_mode {
        let repair = repair_mode;
        let (wx, wy) =
            crate::app_sim_tick::screen_point_to_world(state, state.cursor_x, state.cursor_y);
        let valid = crate::app_commands::own_building_under_point(state, wx, wy).is_some()
            || (!repair && crate::app_commands::sell_wall_under_cursor_is_eligible(state));
        return Some(if repair {
            CursorFeedbackKind::RepairMode(valid)
        } else {
            CursorFeedbackKind::SellMode(valid)
        });
    }
    let selected = crate::app_input::selected_stable_ids_in_order(state);
    if selected.is_empty() {
        return None;
    }
    let owner = preferred_local_owner_name(state).unwrap_or_else(|| "Americans".to_string());
    let (world_x, world_y) =
        crate::app_sim_tick::screen_point_to_world(state, state.cursor_x, state.cursor_y);
    let (hover_rx, hover_ry) =
        crate::app_sim_tick::screen_point_to_world_cell(state, state.cursor_x, state.cursor_y);
    let owner_id = sim.interner.get(&owner);
    if crate::app_instances::cell_visibility_for_local_owner(
        owner_id,
        Some(&sim.fog),
        hover_rx,
        hover_ry,
        state.sandbox_full_visibility,
    ) != CellVisibilityState::Visible
    {
        // Over shrouded/fogged cells the player can still issue move orders,
        // so show the queued-order-mode cursor (Move / AttackMove / Guard)
        // instead of reverting to the default arrow.
        return Some(match state.queued_order_mode {
            crate::app_render::OrderMode::Move => CursorFeedbackKind::Move,
            crate::app_render::OrderMode::AttackMove => CursorFeedbackKind::AttackMove,
            crate::app_render::OrderMode::Guard => CursorFeedbackKind::Guard,
        });
    }
    let modifier = crate::app_context_order::resolve_order_modifiers(
        crate::app_input::is_ctrl_held(state),
        crate::app_input::is_shift_held(state),
        crate::app_input::is_alt_held(state),
    );
    let hover = crate::app_entity_pick::hover_target_at_point(
        sim,
        world_x,
        world_y,
        &owner,
        state.sandbox_full_visibility,
        state.rules.as_ref(),
        &state.height_map(),
        Some(&state.tactical_bridge_inverse_map),
    );
    // gamemd's DetermineAction resolves ONE object for the whole selection and
    // shows that object's action, for the cell branch as well as the object
    // branch. Resolve it before the split so every branch below reads the same
    // object instead of taking `.any()` over the selection.
    let action_target = hover
        .as_ref()
        .and_then(|h| sim.entities().get(h.stable_id))
        .map_or(ActionDistanceTarget::CellCentre(hover_rx, hover_ry), |e| {
            ActionDistanceTarget::Object(e.stable_id)
        });
    let best_id = select_best_for_action(sim, &selected, action_target, state.rules.as_ref());

    // Force-fire override: with Ctrl held the cell path takes the attack branch
    // over allies, own units and empty ground. Ctrl+Shift is attack-move and
    // Ctrl+Alt is guard area — neither force-fires — so this reads the resolved
    // modifier verb rather than raw Ctrl. The armed test runs on the one
    // resolved object, matching gamemd's single-object dispatch.
    if modifier == crate::app_context_order::OrderModifier::ForceFire {
        let best_is_armed = best_id.is_some_and(|id| {
            sim.entities().get(id).is_some_and(|e| {
                let type_str = sim.interner.resolve(e.type_ref);
                state
                    .rules
                    .as_ref()
                    .and_then(|r| r.object(type_str))
                    .is_some_and(|obj| obj.primary.is_some() || obj.secondary.is_some())
            })
        });
        if best_is_armed {
            // EnemyUnit is the standard attack-reticle cursor; reuse it for
            // force-fire over allies/own/empty. (Exact mouse SHP frame for
            // gamemd's distinct force-fire action is unverified — cosmetic-only
            // follow-up; tracked in the design doc.)
            return Some(CursorFeedbackKind::EnemyUnit);
        }
    }
    if let Some(hover) = hover.as_ref() {
        let kind = capability_cursor_for_hover(
            sim,
            &selected,
            best_id,
            hover,
            state.rules.as_ref(),
            sim.path_grid(),
        );
        return Some(kind);
    }

    // No object under the cursor. gamemd runs the full What_Action_OnCell ladder
    // for the resolved object and answers Move or No-Move, so an unreachable or
    // blocked destination shows the barred cursor instead of the move cursor.
    let cell_action = what_action_on_cell(
        sim,
        best_id,
        (hover_rx, hover_ry),
        sim.path_grid(),
        modifier,
    );
    if cell_action == CellAction::NoMove {
        return Some(CursorFeedbackKind::Invalid);
    }

    // Ore/gem harvest hangs off the cell action: UnitClass only substitutes the
    // harvest action when the base action came back Move, and it tests the one
    // resolved object's harvester flags, not the whole selection.
    let has_ore = match (
        sim.overlay_grid.as_ref(),
        state.overlay_registry.as_ref(),
        state.rules.as_ref(),
    ) {
        (Some(grid), Some(registry), Some(rules)) if !rules.tiberium_types.is_empty() => {
            crate::sim::tiberium::tiberium_cell_view(
                grid,
                registry,
                &rules.tiberium_types,
                (hover_rx, hover_ry),
            )
            .is_some()
        }
        _ => false,
    };
    if has_ore
        && best_id.is_some_and(|id| sim.entities().get(id).is_some_and(|e| e.miner.is_some()))
    {
        return Some(CursorFeedbackKind::Harvest);
    }
    Some(match state.queued_order_mode {
        crate::app_render::OrderMode::Move => CursorFeedbackKind::Move,
        crate::app_render::OrderMode::AttackMove => CursorFeedbackKind::AttackMove,
        crate::app_render::OrderMode::Guard => CursorFeedbackKind::Guard,
    })
}

/// gamemd's `What_Action_OnCell` outcome for an empty cell: action 1 (Move) or
/// action 2 (No-Move).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellAction {
    Move,
    NoMove,
}

/// Resolve the empty-cell action for the object that owns the cursor.
///
/// gamemd's ladder, in order: outside the playfield answers No-Move; Shift
/// answers Move and returns *before* the occupancy probe; Alt with the
/// force-move capability answers Move; a unit that cannot accept move orders
/// answers No-Move; otherwise the cell occupancy probe decides Move vs No-Move.
///
/// VERA implements the playfield test, both modifier short-circuits, and a
/// terrain-passability stand-in for the occupancy probe (`Can_Enter_Cell` has no
/// VERA equivalent yet, so the same layered-walkability test the click path uses
/// stands in for it — that residual is terrain-only, with no per-cell occupancy).
/// Aircraft and subterranean movers answer Move for any in-playfield cell.
///
/// With no path grid or no resolved object the cursor keeps its previous
/// behaviour and reports Move: VERA-internal, gamemd equivalent UNCHECKED (a
/// live gamemd session always has a map and a selection here).
fn what_action_on_cell(
    sim: &crate::sim::world::Simulation,
    best_id: Option<u64>,
    cell: (u16, u16),
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    modifier: crate::app_context_order::OrderModifier,
) -> CellAction {
    use crate::sim::movement::locomotor::MovementLayer;

    let Some(grid) = path_grid else {
        return CellAction::Move;
    };
    if cell.0 >= grid.width() || cell.1 >= grid.height() {
        return CellAction::NoMove;
    }
    let Some(entity) = best_id.and_then(|id| sim.entities().get(id)) else {
        return CellAction::Move;
    };
    // Only mobile objects run this ladder; a selected structure takes a
    // different override in gamemd, and VERA answers a rally-point click there.
    if entity.category == crate::map::entities::EntityCategory::Structure {
        return CellAction::Move;
    }
    if matches!(
        modifier,
        crate::app_context_order::OrderModifier::Queue
            | crate::app_context_order::OrderModifier::ForceMove
    ) {
        return CellAction::Move;
    }
    match entity.movement_layer_or_ground() {
        MovementLayer::Air | MovementLayer::Underground => CellAction::Move,
        MovementLayer::Ground | MovementLayer::Bridge => {
            if grid.is_any_layer_walkable(cell.0, cell.1) {
                CellAction::Move
            } else {
                CellAction::NoMove
            }
        }
    }
}

/// Determine the cursor feedback kind for a hover target, checking ObjectType
/// capability flags from rules.ini before falling back to the generic attack/select logic.
///
/// The original engine picks a single "best" selected unit via
/// `SelectBestObjectForAction` (priority: armed mobile > unarmed mobile >
/// immobile; ties broken by distance to target) and uses that unit's
/// `What_Action_OnObject` to determine the cursor for the entire group.
///
/// Priority (highest first):
/// 1. Deployer self-hover: selected unit IS the hovered entity and has Deployer=yes.
/// 2. SabotageCursor: selected unit has SabotageCursor=yes hovering an enemy structure.
/// 3. Engineer capturing: selected Engineer hovering capturable enemy building.
/// 4. Engineer repairing: selected Engineer hovering damaged friendly building.
/// 5. Infantry boarding: selected infantry hovering friendly transport (Passengers>0).
/// 6. Infantry garrisoning: selected Occupier infantry hovering friendly CanBeOccupied building.
/// 7. AttackCursorOnFriendlies: selected unit attacks friendlies, treat as attack target.
/// 8. Harvester docking: selected miner hovering friendly refinery (gamemd action 0x1A).
/// 9. Generic friendly/enemy/in-range/out-of-range fallback.
fn capability_cursor_for_hover(
    sim: &crate::sim::world::Simulation,
    selected: &[u64],
    best_id: Option<u64>,
    hover: &crate::app_entity_pick::HoverTargetKindWithId,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
) -> CursorFeedbackKind {
    use crate::map::entities::EntityCategory;

    let hovered_entity = sim.entities().get(hover.stable_id);
    let hovered_obj =
        rules.and_then(|r| hovered_entity.and_then(|e| r.object(sim.interner.resolve(e.type_ref))));

    // 1. Deployer self-hover — the cursor is over the selected unit itself.
    //    Show the deploy cursor for units with Deployer=yes (e.g. GGI, Guardian GI)
    //    OR units with DeploysInto= set (e.g. MCV → ConYard).  In the original game
    //    both kinds show the deploy cursor when hovering over themselves.
    //    gamemd gates its self-click actions on a selection of exactly one.
    if selected.len() == 1 && selected[0] == hover.stable_id {
        let entity = sim.entities().get(selected[0]);
        let obj =
            entity.and_then(|e| rules.and_then(|r| r.object(sim.interner.resolve(e.type_ref))));
        if let Some(obj) = obj {
            if obj.deployer || obj.deploys_into.is_some() {
                return CursorFeedbackKind::Deploy;
            }
        }
        // 1b. Garrisoned building self-hover — show deploy cursor to unload occupants.
        if let Some(entity) = entity {
            if entity.category == EntityCategory::Structure {
                if let Some(obj) = obj {
                    if obj.can_be_occupied {
                        let has_occupants =
                            entity.passenger_role.cargo().is_some_and(|c| !c.is_empty());
                        if has_occupants {
                            return CursorFeedbackKind::Deploy;
                        }
                    }
                }
                // Own occupied tank bunker self-hover → eject (deploy cursor).
                if entity.bunker_occupant.is_some() {
                    return CursorFeedbackKind::Deploy;
                }
                if rules.is_some_and(|rules| {
                    sim.should_show_undeploy_building_command(hover.stable_id, rules)
                }) {
                    return CursorFeedbackKind::Deploy;
                }
            }
        }
    }

    // The caller already resolved the single object whose action drives the
    // cursor for the whole selection, matching gamemd's DetermineAction.
    if let Some(best_id) = best_id {
        if let (Some(sel_entity), Some(sel_obj)) = (
            sim.entities().get(best_id),
            sim.entities()
                .get(best_id)
                .and_then(|e| rules.and_then(|r| r.object(sim.interner.resolve(e.type_ref)))),
        ) {
            // 2. C4 plant: SEAL / Tanya / Psi-Corp Trooper hovering an enemy
            //    structure with CanC4=yes, not InvisibleInGame, not iron-curtained.
            //    SabotageCursor flag remains in the data model (parsed in
            //    object_type.rs) for modder weapon-overlay use, but cursor
            //    logic is now driven by C4=yes — matches gamemd action 0x10.
            if sel_obj.c4
                && matches!(hover.kind, HoverTargetKind::EnemyStructure)
                && hovered_obj.map_or(false, |o| o.can_c4 && !o.invisible_in_game)
                && !hovered_entity.is_some_and(|e| {
                    crate::sim::superweapon::invulnerability::is_invulnerable(
                        e.invulnerability.as_ref(),
                        sim.session.tick as u32,
                    )
                })
            {
                return CursorFeedbackKind::Demolish;
            }

            let is_infantry = sel_entity.category == EntityCategory::Infantry;

            if sel_obj.engineer {
                // 3. Engineer on bridge repair hut → repair (Enter cursor).
                if matches!(hover.kind, HoverTargetKind::EnemyStructure) {
                    if hovered_obj.map_or(false, |o| o.bridge_repair_hut) {
                        return CursorFeedbackKind::Enter;
                    }
                }
                // 4. Engineer on capturable enemy building → capture (Enter cursor).
                if matches!(hover.kind, HoverTargetKind::EnemyStructure) {
                    if hovered_obj.map_or(false, |o| o.capturable) {
                        return CursorFeedbackKind::Enter;
                    }
                }
                // 5. Engineer on damaged friendly building → repair.
                if matches!(hover.kind, HoverTargetKind::FriendlyStructure) {
                    if let Some(he) = hovered_entity {
                        if he.health.current < he.health.max {
                            return CursorFeedbackKind::EngineerRepair;
                        }
                    }
                }
            }

            // 5. Infantry boarding a friendly transport (Passengers > 0).
            if is_infantry && matches!(hover.kind, HoverTargetKind::FriendlyUnit) {
                if hovered_obj.map_or(false, |o| o.passengers > 0) {
                    return CursorFeedbackKind::Enter;
                }
            }

            // 6. Infantry garrisoning uses the shared CanDock-equivalent predicate.
            //    garrisonable — only show Enter for those, not actual enemy-player buildings.
            if is_infantry {
                if let Some(rules) = rules {
                    if crate::sim::passenger::can_entity_enter_garrison(
                        sim,
                        rules,
                        best_id,
                        hover.stable_id,
                        path_grid,
                    ) {
                        return CursorFeedbackKind::Enter;
                    }
                }
            }

            // 7. AttackCursorOnFriendlies — treat friendly targets as attack targets.
            if sel_obj.attack_cursor_on_friendlies {
                if matches!(
                    hover.kind,
                    HoverTargetKind::FriendlyUnit | HoverTargetKind::FriendlyStructure
                ) {
                    let in_range = resolved_unit_in_range(
                        sim,
                        best_id,
                        hover.stable_id,
                        rules,
                        sim.resolved_terrain.as_ref(),
                    );
                    return if in_range {
                        if hover.kind == HoverTargetKind::FriendlyUnit {
                            CursorFeedbackKind::EnemyUnit
                        } else {
                            CursorFeedbackKind::EnemyStructure
                        }
                    } else {
                        CursorFeedbackKind::EnemyOutOfRange
                    };
                }
            }

            // 8. Harvester docking — selected miner hovering own/ally refinery.
            //    Matches gamemd action 0x1A (TechnoClass dock branch). Alliance
            //    gate comes from HoverTargetKind::FriendlyStructure; refinery
            //    detection from RuleSet::is_refinery_type (same key used by
            //    the click pipeline in app_context_order.rs).
            if sel_entity.miner.is_some()
                && matches!(hover.kind, HoverTargetKind::FriendlyStructure)
                && hovered_entity.is_some_and(|e| {
                    rules.is_some_and(|r| r.is_refinery_type(sim.interner.resolve(e.type_ref)))
                })
            {
                return CursorFeedbackKind::Enter;
            }

            // Service depot — a damaged own vehicle over an own UnitRepair
            // building shows the enter/dock cursor (the click issues
            // RepairAtDepot; see app_context_order.rs).
            if sel_entity.category == EntityCategory::Unit
                && sel_entity.health.current < sel_entity.health.max
                && matches!(hover.kind, HoverTargetKind::FriendlyStructure)
                && hovered_obj.map_or(false, |o| o.unit_repair)
            {
                return CursorFeedbackKind::Enter;
            }

            // Tank bunker — an own bunkerable vehicle over an own EMPTY tank
            // bunker shows the enter cursor (the click issues EnterBunker).
            if matches!(hover.kind, HoverTargetKind::FriendlyStructure)
                && hovered_entity
                    .is_some_and(|he| he.bunker_runtime.is_some() && he.bunker_occupant.is_none())
                && rules.is_some_and(|r| {
                    crate::sim::docking::bunker_link::can_auto_deploy_here(sim, best_id, r)
                })
            {
                return CursorFeedbackKind::Enter;
            }
        }
    }

    // 9. Generic fallback.
    match hover.kind {
        HoverTargetKind::FriendlyUnit => CursorFeedbackKind::FriendlyUnit,
        HoverTargetKind::FriendlyStructure => CursorFeedbackKind::FriendlyStructure,
        HoverTargetKind::EnemyUnit | HoverTargetKind::EnemyStructure => {
            let in_range = best_id.is_some_and(|id| {
                resolved_unit_in_range(
                    sim,
                    id,
                    hover.stable_id,
                    rules,
                    sim.resolved_terrain.as_ref(),
                )
            });
            if in_range {
                if hover.kind == HoverTargetKind::EnemyUnit {
                    CursorFeedbackKind::EnemyUnit
                } else {
                    CursorFeedbackKind::EnemyStructure
                }
            } else {
                CursorFeedbackKind::EnemyOutOfRange
            }
        }
        HoverTargetKind::HiddenEnemy => CursorFeedbackKind::Invalid,
    }
}

/// Does the object that owns the cursor have a weapon that reaches the target?
///
/// gamemd shows the action of ONE resolved object, so the in-range split is a
/// property of that object alone, not of any unit in the selection. Both weapon
/// slots count, matching the all-slots weapon-range query the object resolver
/// itself uses.
fn resolved_unit_in_range(
    sim: &crate::sim::world::Simulation,
    actor_id: u64,
    target_id: u64,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
) -> bool {
    let rules = match rules {
        Some(r) => r,
        None => return true,
    };
    let target_pos = match sim.entities().get(target_id) {
        Some(t) => (
            t.position.rx,
            t.position.ry,
            t.position.sub_x,
            t.position.sub_y,
        ),
        None => return false,
    };
    let Some(entity) = sim.entities().get(actor_id) else {
        return false;
    };
    let Some(obj) = rules.object(sim.interner.resolve(entity.type_ref)) else {
        return false;
    };
    for slot in [obj.primary.as_ref(), obj.secondary.as_ref()] {
        let weapon = match slot.and_then(|w| rules.weapon(w)) {
            Some(w) => w,
            None => continue,
        };
        if weapon.range <= crate::util::fixed_math::SIM_ZERO {
            continue;
        }
        let in_range = if let Some(t) = terrain {
            let Some(source_z) = combat::in_range::effective_z_leptons(entity, t) else {
                continue;
            };
            let src = (
                entity.position.rx as i64 * 256 + entity.position.sub_x.to_num::<i64>(),
                entity.position.ry as i64 * 256 + entity.position.sub_y.to_num::<i64>(),
                source_z,
            );
            combat::in_range::compute_in_range(
                entity,
                src,
                &combat::TargetKind::Entity(target_id),
                weapon,
                rules,
                &sim.interner,
                sim.entities(),
                t,
            )
        } else {
            let dist_sq = combat::lepton_distance_sq_raw(
                entity.position.rx,
                entity.position.ry,
                entity.position.sub_x,
                entity.position.sub_y,
                target_pos.0,
                target_pos.1,
                target_pos.2,
                target_pos.3,
            );
            combat::is_within_range_leptons(dist_sq, weapon.range)
        };
        if in_range {
            return true;
        }
    }
    false
}

/// What the object resolver measures its candidates' distance to.
///
/// gamemd's resolver takes a cell *or* an object: with a cell it measures to the
/// cell centre at ground level, with an object to that object's own world point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionDistanceTarget {
    /// Cell centre, in cell coordinates.
    CellCentre(u16, u16),
    /// A specific object's world point.
    Object(u64),
}

/// Leptons per cell — a cell centre sits half a cell in on both axes.
const LEPTONS_PER_CELL: i64 = 256;

/// World point in leptons for one entity, including altitude when terrain is
/// resolved. gamemd's resolver reads the object's full 3-D world coordinate.
fn entity_world_leptons(
    entity: &crate::sim::game_entity::GameEntity,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
) -> (i64, i64, i64) {
    let z = terrain
        .and_then(|t| combat::in_range::effective_z_leptons(entity, t))
        .unwrap_or(0);
    (
        entity.position.rx as i64 * LEPTONS_PER_CELL + entity.position.sub_x.to_num::<i64>(),
        entity.position.ry as i64 * LEPTONS_PER_CELL + entity.position.sub_y.to_num::<i64>(),
        z,
    )
}

/// Integer square root — the resolver compares whole-lepton distances, not
/// squares, so ties that round to the same distance must resolve the same way.
fn isqrt_u64(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut guess = 1u64 << ((64 - value.leading_zeros()).div_ceil(2));
    loop {
        let next = (guess + value / guess) / 2;
        if next >= guess {
            return guess;
        }
        guess = next;
    }
}

/// Pick the single object whose action drives the cursor for the whole selection.
///
/// Matches the original engine's `SelectBestObjectForAction` score ladder:
///   5 — actionable, not a building, and at least one weapon slot has range
///   4 — actionable, not a building
///   3 — actionable
///   2 — on the map but not actionable
/// (the engine's tiers 1 and 0 cover deploy-in-progress and warp states that
/// VERA does not model here yet — recorded, not invented.)
///
/// Ties are *evaluated*, not skipped: a strictly higher score always replaces
/// the incumbent and overwrites the stored distance even when it is farther
/// away, while an equal score replaces only on a strictly smaller distance.
/// Distance is 3-D Euclidean in leptons — to the cell centre when a cell was
/// supplied, otherwise to the target object's own world point.
fn select_best_for_action(
    sim: &crate::sim::world::Simulation,
    selected: &[u64],
    target: ActionDistanceTarget,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> Option<u64> {
    use crate::map::entities::EntityCategory;

    let terrain = sim.resolved_terrain.as_ref();
    let target_point: (i64, i64, i64) = match target {
        ActionDistanceTarget::CellCentre(cx, cy) => (
            cx as i64 * LEPTONS_PER_CELL + LEPTONS_PER_CELL / 2,
            cy as i64 * LEPTONS_PER_CELL + LEPTONS_PER_CELL / 2,
            0,
        ),
        ActionDistanceTarget::Object(id) => match sim.entities().get(id) {
            Some(e) => entity_world_leptons(e, terrain),
            None => return None,
        },
    };

    let mut best_id: Option<u64> = None;
    let mut best_priority: i32 = -1;
    let mut best_dist: u64 = u64::MAX;

    for &sid in selected {
        let Some(entity) = sim.entities().get(sid) else {
            continue;
        };
        let priority = if entity.category == EntityCategory::Structure {
            // A building is actionable but is a building, so it stops at 3.
            3
        } else {
            // The engine's weapon test queries *all* weapon slots, so a
            // secondary-only unit scores 5 just like a primary-armed one.
            let obj = rules.and_then(|r| r.object(sim.interner.resolve(entity.type_ref)));
            let has_weapon = obj.is_some_and(|o| {
                [o.primary.as_ref(), o.secondary.as_ref()]
                    .into_iter()
                    .flatten()
                    .filter_map(|w| rules.and_then(|r| r.weapon(w)))
                    .any(|w| w.range > crate::util::fixed_math::SIM_ZERO)
            });
            if has_weapon { 5 } else { 4 }
        };

        let (sx, sy, sz) = entity_world_leptons(entity, terrain);
        let (dx, dy, dz) = (
            sx - target_point.0,
            sy - target_point.1,
            sz - target_point.2,
        );
        let dist = isqrt_u64((dx * dx + dy * dy + dz * dz) as u64);

        if priority > best_priority || (priority == best_priority && dist < best_dist) {
            best_priority = priority;
            best_dist = dist;
            best_id = Some(sid);
        }
    }
    best_id
}

/// Map a game-state cursor intent to the visual CursorId to display.
/// Returns None for feedback kinds that use procedural visuals instead of a software cursor
/// (e.g. building placement preview).
/// Alias mappings live here: FriendlyUnit→Select, etc.
pub(crate) fn cursor_id_for_feedback(kind: CursorFeedbackKind) -> Option<CursorId> {
    match kind {
        CursorFeedbackKind::FriendlyUnit | CursorFeedbackKind::FriendlyStructure => {
            Some(CursorId::Select)
        }
        // Guard-area has its own reticle (cursor row 22); it is not the select
        // cursor, which is what VERA used to show while guard mode was armed.
        CursorFeedbackKind::Guard => Some(CursorId::GuardArea),
        CursorFeedbackKind::Move => Some(CursorId::Move),
        CursorFeedbackKind::AttackMove => Some(CursorId::AttackMove),
        CursorFeedbackKind::EnemyUnit | CursorFeedbackKind::EnemyStructure => {
            Some(CursorId::Attack)
        }
        // Harvest and out-of-range attack share cursor row 21 — the action
        // switch falls through from the attack case straight into the harvest
        // case, so both land on the same row.
        CursorFeedbackKind::EnemyOutOfRange | CursorFeedbackKind::Harvest => {
            Some(CursorId::AttackOutOfRange)
        }
        CursorFeedbackKind::Invalid => Some(CursorId::NoMove),
        CursorFeedbackKind::PlaceValid | CursorFeedbackKind::PlaceInvalid => None,
        CursorFeedbackKind::Scroll(dir) => Some(scroll_dir_to_cursor_id(dir)),
        CursorFeedbackKind::ScrollBlocked(dir) => Some(blocked_scroll_dir_to_cursor_id(dir)),
        CursorFeedbackKind::MinimapMove => Some(CursorId::MinimapMove),
        CursorFeedbackKind::Enter => Some(CursorId::Enter),
        CursorFeedbackKind::EngineerRepair => Some(CursorId::EngineerRepair),
        CursorFeedbackKind::Demolish => Some(CursorId::Demolish),
        CursorFeedbackKind::Deploy => Some(CursorId::Deploy),
        CursorFeedbackKind::RepairMode(valid) => Some(if valid {
            CursorId::Repair
        } else {
            CursorId::NoRepair
        }),
        CursorFeedbackKind::SellMode(valid) => Some(if valid {
            CursorId::Sell
        } else {
            CursorId::NoSell
        }),
        CursorFeedbackKind::SuperWeaponTarget(id) => Some(id),
    }
}

fn scroll_dir_to_cursor_id(dir: ScrollDir) -> CursorId {
    match dir {
        ScrollDir::N => CursorId::ScrollN,
        ScrollDir::NE => CursorId::ScrollNE,
        ScrollDir::E => CursorId::ScrollE,
        ScrollDir::SE => CursorId::ScrollSE,
        ScrollDir::S => CursorId::ScrollS,
        ScrollDir::SW => CursorId::ScrollSW,
        ScrollDir::W => CursorId::ScrollW,
        ScrollDir::NW => CursorId::ScrollNW,
    }
}

fn blocked_scroll_dir_to_cursor_id(dir: ScrollDir) -> CursorId {
    match dir {
        ScrollDir::N => CursorId::NoMoveN,
        ScrollDir::NE => CursorId::NoMoveNE,
        ScrollDir::E => CursorId::NoMoveE,
        ScrollDir::SE => CursorId::NoMoveSE,
        ScrollDir::S => CursorId::NoMoveS,
        ScrollDir::SW => CursorId::NoMoveSW,
        ScrollDir::W => CursorId::NoMoveW,
        ScrollDir::NW => CursorId::NoMoveNW,
    }
}

/// Phase of the animated software cursor.
///
/// The original keeps this in engine globals and the contract has two halves.
/// Setting a new mouse shape zeroes the current frame and re-anchors the timer,
/// so every cursor change restarts its sequence at frame 0 rather than dropping
/// into the middle of a shared free-running phase. The per-frame update then
/// advances by exactly **one** frame once the interval has elapsed and
/// re-anchors again — never several — so a frame-rate stall makes the animation
/// lag instead of skipping frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CursorAnimation {
    shape: Option<CursorId>,
    frame: usize,
    anchor_ms: u64,
}

impl CursorAnimation {
    pub(crate) const fn new() -> Self {
        Self {
            shape: None,
            frame: 0,
            anchor_ms: 0,
        }
    }

    /// Resolve the frame index to draw for `id` at `now_ms`, updating the phase.
    pub(crate) fn advance(
        &mut self,
        id: CursorId,
        frame_count: usize,
        interval_ms: u64,
        now_ms: u64,
    ) -> usize {
        if self.shape != Some(id) {
            self.shape = Some(id);
            self.frame = 0;
            self.anchor_ms = now_ms;
            return 0;
        }
        if frame_count <= 1 || interval_ms == 0 {
            return 0;
        }
        if now_ms.saturating_sub(self.anchor_ms) >= interval_ms {
            self.frame = (self.frame + 1) % frame_count;
            self.anchor_ms = now_ms;
        }
        self.frame
    }
}

thread_local! {
    static CURSOR_ANIMATION: std::cell::Cell<CursorAnimation> =
        const { std::cell::Cell::new(CursorAnimation::new()) };
}

/// The frame of `sequence` to draw for cursor `id` right now.
///
/// The id is part of the query because the phase is keyed on it: asking for a
/// different cursor than last frame restarts that cursor's animation.
pub(crate) fn software_cursor_frame_for(
    id: CursorId,
    sequence: &SoftwareCursorSequence,
) -> Option<&SoftwareCursorFrame> {
    if sequence.frames.is_empty() {
        return None;
    }
    let now_ms: u64 = cursor_animation_start()
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let frame_idx = CURSOR_ANIMATION.with(|cell| {
        let mut animation = cell.get();
        let idx = animation.advance(id, sequence.frames.len(), sequence.interval_ms, now_ms);
        cell.set(animation);
        idx
    });
    sequence.frames.get(frame_idx)
}

/// Shell screens (menu, skirmish setup, score) only ever show the default
/// arrow, which is a single static frame.
pub(crate) fn current_software_cursor_frame(
    sequence: &SoftwareCursorSequence,
) -> Option<&SoftwareCursorFrame> {
    software_cursor_frame_for(CursorId::Default, sequence)
}

fn cursor_animation_start() -> &'static Instant {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now)
}

fn is_cursor_over_minimap(state: &AppState) -> bool {
    // Minimap interaction disabled when radar is not online.
    let minimap_visible: bool = state
        .radar_anim
        .as_ref()
        .map_or(true, |ra| ra.is_minimap_visible());
    if !minimap_visible {
        return false;
    }
    let Some(_minimap) = &state.minimap else {
        return false;
    };
    let rect = crate::app_sidebar_render::active_minimap_screen_rect(state);
    state
        .minimap
        .as_ref()
        .unwrap()
        .contains_screen_point_in_rect(
            state.cursor_x,
            state.cursor_y,
            rect.x,
            rect.y,
            rect.w,
            rect.h,
        )
}

pub(crate) fn current_sidebar_view_hit(state: &AppState) -> bool {
    let sw = state.sidebar_layout_spec.sidebar_width;
    let panel_rect = crate::sidebar::Rect {
        x: state.render_width() as f32 - sw - 10.0,
        y: 10.0,
        w: sw,
        h: state.render_height() as f32 - 20.0,
    };
    panel_rect.contains(state.cursor_x, state.cursor_y)
}

/// Map a SuperWeaponType `Action=` INI string to its targeting cursor.
///
/// Action strings come from `[SWType] Action=` in rulesmd.ini. Cursor
/// frame ranges are pre-loaded in `render/cursor_atlas.rs`.
///
/// Returns `None` for `IonCannon` (TS-legacy, no YR SW uses it) and any
/// unrecognized string. Caller should fall back to `CursorId::Default`.
pub(crate) fn super_weapon_cursor_id(action: &str) -> Option<CursorId> {
    match action {
        "Nuke" => Some(CursorId::Nuke),
        "ChronoSphere" => Some(CursorId::Chronosphere),
        "ChronoWarp" => Some(CursorId::Chronosphere),
        "IronCurtain" => Some(CursorId::IronCurtain),
        "LightningStorm" => Some(CursorId::LightningStorm),
        "ParaDrop" => Some(CursorId::Paradrop),
        "AmerParaDrop" => Some(CursorId::Paradrop),
        "PsychicDominator" => Some(CursorId::PsychicDominator),
        "SpyPlane" => Some(CursorId::SpyPlane),
        "GeneticConverter" => Some(CursorId::GeneticMutator),
        "ForceShield" => Some(CursorId::ForceShield),
        "PsychicReveal" => Some(CursorId::PsychicReveal),
        // IonCannon is TS-legacy — no YR superweapon uses this Action.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActionDistanceTarget, CellAction, capability_cursor_for_hover, select_best_for_action,
        super_weapon_cursor_id, what_action_on_cell,
    };
    use crate::app_entity_pick::HoverTargetKindWithId;
    use crate::app_types::{CursorFeedbackKind, CursorId, HoverTargetKind};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::sim::world::Simulation;
    use std::collections::BTreeMap;

    fn cursor_contract_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             0=GHOST\n\
             [VehicleTypes]\n\
             0=CMIN\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             0=NAPOWR\n\
             1=GAREFN\n\
             [GHOST]\n\
             Strength=100\n\
             C4=yes\n\
             [CMIN]\n\
             Strength=1000\n\
             Harvester=yes\n\
             Dock=GAREFN\n\
             [NAPOWR]\n\
             Strength=750\n\
             CanC4=yes\n\
             [GAREFN]\n\
             Strength=1000\n\
             Refinery=yes\n",
        );
        RuleSet::from_ini(&ini).expect("cursor contract rules")
    }

    /// Repro for "SEAL right-click on enemy buildings does nothing".
    /// Loads a narrow stock-shaped rules contract, spawns a SEAL via
    /// `spawn_object` (the same code path the barracks uses on production
    /// completion), then calls `capability_cursor_for_hover` and asserts
    /// the returned cursor is `Demolish`. If it isn't, the body of the
    /// function prints which gate condition rejected it.
    #[test]
    fn seal_hovering_enemy_building_shows_demolish() {
        // 1. Load the narrow stock-shaped contract consumed by this cursor path.
        let mut rules = cursor_contract_rules();

        // 2. Build a Simulation. resolve_type_handles is required by the
        //    c4 tick path even though we don't tick here — keeps the sim in a
        //    consistent state with what the runtime would see.
        let mut sim = Simulation::new();
        sim.resolve_type_handles(&rules);
        let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();

        // 3. Spawn a SEAL and an enemy Power Plant via the same path the
        //    barracks uses on production completion.
        let seal_id = sim
            .spawn_object("GHOST", "Americans", 5, 5, 0, &rules, &height_map)
            .expect("SEAL spawned");
        let bld_id = sim
            .spawn_object("NAPOWR", "Soviets", 10, 10, 0, &rules, &height_map)
            .expect("Power Plant spawned");

        // 4. Mark the SEAL as selected (mirrors clicking it in-game).
        if let Some(e) = sim.entities_mut().get_mut(seal_id) {
            e.selected = true;
        }

        // 5. Construct the hover descriptor the runtime would build when
        //    the cursor is over an enemy building.
        let hover = HoverTargetKindWithId {
            kind: HoverTargetKind::EnemyStructure,
            stable_id: bld_id,
        };

        // 6. Call the same function the live cursor pipeline calls.
        let result = capability_cursor_for_hover(
            &sim,
            &[seal_id],
            Some(seal_id),
            &hover,
            Some(&rules),
            None,
        );

        // 7. Dump the gate inputs so we can see which condition fails
        //    if the assertion below trips.
        let seal_obj = rules.object("GHOST");
        let bld_obj = rules.object("NAPOWR");
        eprintln!(
            "DIAG: seal.c4={:?} bld.can_c4={:?} bld.invis={:?} cursor={:?}",
            seal_obj.map(|o| o.c4),
            bld_obj.map(|o| o.can_c4),
            bld_obj.map(|o| o.invisible_in_game),
            result,
        );

        assert_eq!(
            result,
            CursorFeedbackKind::Demolish,
            "SEAL hovering an enemy Power Plant should show Demolish cursor",
        );
    }

    /// Chrono Miner hovering its own Allied Refinery should show the dock
    /// (Enter) cursor. gamemd action 0x1A — the TechnoClass dock branch fires
    /// for any harvester targeting a same-owner refinery.
    #[test]
    fn chrono_miner_hovering_own_refinery_shows_enter() {
        let mut rules = cursor_contract_rules();

        let mut sim = Simulation::new();
        sim.resolve_type_handles(&rules);
        let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();

        let miner_id = sim
            .spawn_object("CMIN", "Americans", 5, 5, 0, &rules, &height_map)
            .expect("Chrono Miner spawned");
        let refinery_id = sim
            .spawn_object("GAREFN", "Americans", 10, 10, 0, &rules, &height_map)
            .expect("Refinery spawned");

        if let Some(e) = sim.entities_mut().get_mut(miner_id) {
            e.selected = true;
        }

        let hover = HoverTargetKindWithId {
            kind: HoverTargetKind::FriendlyStructure,
            stable_id: refinery_id,
        };

        let result = capability_cursor_for_hover(
            &sim,
            &[miner_id],
            Some(miner_id),
            &hover,
            Some(&rules),
            None,
        );
        assert_eq!(
            result,
            CursorFeedbackKind::Enter,
            "Chrono Miner hovering its own Refinery should show the Enter (dock) cursor",
        );
    }

    #[test]
    fn maps_every_yr_active_action() {
        assert_eq!(super_weapon_cursor_id("Nuke"), Some(CursorId::Nuke));
        assert_eq!(
            super_weapon_cursor_id("ChronoSphere"),
            Some(CursorId::Chronosphere)
        );
        assert_eq!(
            super_weapon_cursor_id("ChronoWarp"),
            Some(CursorId::Chronosphere)
        );
        assert_eq!(
            super_weapon_cursor_id("IronCurtain"),
            Some(CursorId::IronCurtain)
        );
        assert_eq!(
            super_weapon_cursor_id("LightningStorm"),
            Some(CursorId::LightningStorm)
        );
        assert_eq!(super_weapon_cursor_id("ParaDrop"), Some(CursorId::Paradrop));
        assert_eq!(
            super_weapon_cursor_id("AmerParaDrop"),
            Some(CursorId::Paradrop)
        );
        assert_eq!(
            super_weapon_cursor_id("PsychicDominator"),
            Some(CursorId::PsychicDominator)
        );
        assert_eq!(super_weapon_cursor_id("SpyPlane"), Some(CursorId::SpyPlane));
        assert_eq!(
            super_weapon_cursor_id("GeneticConverter"),
            Some(CursorId::GeneticMutator)
        );
        assert_eq!(
            super_weapon_cursor_id("ForceShield"),
            Some(CursorId::ForceShield)
        );
        assert_eq!(
            super_weapon_cursor_id("PsychicReveal"),
            Some(CursorId::PsychicReveal)
        );
    }

    #[test]
    fn returns_none_for_ts_legacy_and_unknown() {
        assert_eq!(super_weapon_cursor_id("IonCannon"), None);
        assert_eq!(super_weapon_cursor_id(""), None);
        assert_eq!(super_weapon_cursor_id("BogusAction"), None);
    }

    /// Rules for the cell-action and best-object contracts: one plain armed
    /// tank, one unarmed truck, one unit armed only through `Secondary=`.
    fn cell_action_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             0=MTNK\n\
             1=TRUCKA\n\
             2=SREF\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [MTNK]\n\
             Strength=300\n\
             Primary=105mm\n\
             [TRUCKA]\n\
             Strength=150\n\
             [SREF]\n\
             Strength=200\n\
             Secondary=105mm\n\
             [WeaponTypes]\n\
             0=105mm\n\
             [105mm]\n\
             Damage=60\n\
             Range=5\n",
        );
        RuleSet::from_ini(&ini).expect("cell action rules")
    }

    fn sim_with_tank() -> (Simulation, RuleSet, u64) {
        let mut rules = cell_action_rules();
        let mut sim = Simulation::new();
        sim.resolve_type_handles(&rules);
        let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
        let tank = sim
            .spawn_object("MTNK", "Americans", 2, 2, 0, &rules, &height_map)
            .expect("tank spawned");
        (sim, rules, tank)
    }

    /// gamemd answers action 2 (no-move) for a cell its occupancy probe rejects,
    /// so the barred cursor appears before the click — the move cursor is not a
    /// constant over empty ground.
    #[test]
    fn blocked_cell_resolves_to_no_move() {
        use crate::app_context_order::OrderModifier;
        use crate::sim::pathfinding::PathGrid;

        let (sim, _rules, tank) = sim_with_tank();
        let mut grid = PathGrid::new(8, 8);
        grid.set_blocked(6, 6, true);

        assert_eq!(
            what_action_on_cell(&sim, Some(tank), (4, 4), Some(&grid), OrderModifier::Normal),
            CellAction::Move,
            "an open cell inside the playfield answers move",
        );
        assert_eq!(
            what_action_on_cell(&sim, Some(tank), (6, 6), Some(&grid), OrderModifier::Normal),
            CellAction::NoMove,
            "a blocked cell answers no-move",
        );
    }

    /// Outside the playfield the ladder answers no-move before it looks at the
    /// cell at all.
    #[test]
    fn cell_outside_the_playfield_resolves_to_no_move() {
        use crate::app_context_order::OrderModifier;
        use crate::sim::pathfinding::PathGrid;

        let (sim, _rules, tank) = sim_with_tank();
        let grid = PathGrid::new(8, 8);

        assert_eq!(
            what_action_on_cell(&sim, Some(tank), (8, 3), Some(&grid), OrderModifier::Normal),
            CellAction::NoMove,
        );
    }

    /// gamemd returns action 1 on Shift and on Alt *before* running the
    /// occupancy probe, so both show the move cursor even over a blocked cell.
    #[test]
    fn shift_and_alt_skip_the_occupancy_probe() {
        use crate::app_context_order::OrderModifier;
        use crate::sim::pathfinding::PathGrid;

        let (sim, _rules, tank) = sim_with_tank();
        let mut grid = PathGrid::new(8, 8);
        grid.set_blocked(6, 6, true);

        assert_eq!(
            what_action_on_cell(&sim, Some(tank), (6, 6), Some(&grid), OrderModifier::Queue),
            CellAction::Move,
        );
        assert_eq!(
            what_action_on_cell(
                &sim,
                Some(tank),
                (6, 6),
                Some(&grid),
                OrderModifier::ForceMove
            ),
            CellAction::Move,
        );
    }

    /// The engine's weapon test queries every weapon slot, so a `Secondary=`-only
    /// unit reaches the armed tier and outranks an unarmed one regardless of
    /// which is closer.
    #[test]
    fn secondary_only_unit_outranks_a_closer_unarmed_unit() {
        let mut rules = cell_action_rules();
        let mut sim = Simulation::new();
        sim.resolve_type_handles(&rules);
        let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();

        let truck = sim
            .spawn_object("TRUCKA", "Americans", 9, 10, 0, &rules, &height_map)
            .expect("unarmed truck");
        let arty = sim
            .spawn_object("SREF", "Americans", 2, 2, 0, &rules, &height_map)
            .expect("secondary-only unit");

        let best = select_best_for_action(
            &sim,
            &[truck, arty],
            ActionDistanceTarget::CellCentre(10, 10),
            Some(&rules),
        );
        assert_eq!(
            best,
            Some(arty),
            "the armed tier wins even from much farther away",
        );
    }

    /// Ties inside one tier break on 3-D lepton distance to the *cell centre*.
    /// Both candidates sit in cells the same number of cell indices away, so a
    /// cell-index tie-break cannot separate them; the sub-cell offset can.
    #[test]
    fn tie_break_uses_lepton_distance_to_the_cell_centre() {
        let mut rules = cell_action_rules();
        let mut sim = Simulation::new();
        sim.resolve_type_handles(&rules);
        let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();

        let near = sim
            .spawn_object("MTNK", "Americans", 8, 10, 0, &rules, &height_map)
            .expect("near tank");
        let far = sim
            .spawn_object("MTNK", "Americans", 12, 10, 0, &rules, &height_map)
            .expect("far tank");
        // Nudge the far tank's sub-cell offset toward the target so that both
        // sit two cell indices away but the far one is closer in leptons.
        if let Some(e) = sim.entities_mut().get_mut(far) {
            e.position.sub_x = crate::util::fixed_math::SimFixed::from_num(-120);
        }

        let best = select_best_for_action(
            &sim,
            &[near, far],
            ActionDistanceTarget::CellCentre(10, 10),
            Some(&rules),
        );
        assert_eq!(
            best,
            Some(far),
            "sub-cell position decides the tie, not the cell index",
        );
    }
}

#[cfg(test)]
mod cursor_animation_tests {
    use super::CursorAnimation;
    use crate::app_types::{CursorFeedbackKind, CursorId, ScrollDir};

    /// Retail interval for every animated cursor row: rate 4 x 16 ms.
    const INTERVAL_MS: u64 = 64;

    /// The first look at a cursor starts it at frame 0 and anchors the timer
    /// there, instead of sampling a process-wide clock.
    #[test]
    fn a_new_cursor_shape_starts_at_frame_zero() {
        let mut anim = CursorAnimation::new();
        assert_eq!(anim.advance(CursorId::Move, 10, INTERVAL_MS, 5_000), 0);
    }

    /// One frame per elapsed interval, and exactly one — a long stall does not
    /// skip ahead.
    #[test]
    fn animation_advances_one_frame_per_interval_and_never_skips() {
        let mut anim = CursorAnimation::new();
        anim.advance(CursorId::Move, 10, INTERVAL_MS, 0);
        assert_eq!(anim.advance(CursorId::Move, 10, INTERVAL_MS, 63), 0);
        assert_eq!(anim.advance(CursorId::Move, 10, INTERVAL_MS, 64), 1);
        // A 1-second stall still yields a single step.
        assert_eq!(anim.advance(CursorId::Move, 10, INTERVAL_MS, 1_064), 2);
    }

    #[test]
    fn animation_wraps_at_the_end_of_the_sequence() {
        let mut anim = CursorAnimation::new();
        anim.advance(CursorId::Attack, 5, INTERVAL_MS, 0);
        for step in 1..=4 {
            assert_eq!(
                anim.advance(CursorId::Attack, 5, INTERVAL_MS, step * INTERVAL_MS),
                step as usize
            );
        }
        assert_eq!(
            anim.advance(CursorId::Attack, 5, INTERVAL_MS, 5 * INTERVAL_MS),
            0
        );
    }

    /// The clause a process-global phase cannot express: switching cursor
    /// restarts the new sequence at frame 0 rather than dropping into whatever
    /// phase the shared clock happened to be in.
    #[test]
    fn changing_cursor_restarts_the_sequence() {
        let mut anim = CursorAnimation::new();
        anim.advance(CursorId::Move, 10, INTERVAL_MS, 0);
        assert_eq!(
            anim.advance(CursorId::Move, 10, INTERVAL_MS, 3 * INTERVAL_MS),
            1
        );
        assert_eq!(
            anim.advance(CursorId::Attack, 5, INTERVAL_MS, 3 * INTERVAL_MS),
            0
        );
        // And going back restarts again rather than resuming.
        assert_eq!(
            anim.advance(CursorId::Move, 10, INTERVAL_MS, 3 * INTERVAL_MS),
            0
        );
    }

    /// A rate-0 row is static however much time passes.
    #[test]
    fn static_rows_never_advance() {
        let mut anim = CursorAnimation::new();
        anim.advance(CursorId::IronCurtain, 5, 0, 0);
        assert_eq!(anim.advance(CursorId::IronCurtain, 5, 0, 10_000), 0);
    }

    /// Guard mode shows the guard-area reticle, not the select cursor.
    #[test]
    fn guard_feedback_maps_to_the_guard_area_reticle() {
        assert_eq!(
            super::cursor_id_for_feedback(CursorFeedbackKind::Guard),
            Some(CursorId::GuardArea)
        );
    }

    /// Harvest shares cursor row 21 with an out-of-range attack, and is not the
    /// attack-move reticle VERA used to show.
    #[test]
    fn harvest_feedback_maps_to_cursor_row_twenty_one() {
        assert_eq!(
            super::cursor_id_for_feedback(CursorFeedbackKind::Harvest),
            Some(CursorId::AttackOutOfRange)
        );
        assert_ne!(
            super::cursor_id_for_feedback(CursorFeedbackKind::Harvest),
            Some(CursorId::AttackMove)
        );
    }

    #[test]
    fn item82_allowed_and_blocked_scroll_feedback_use_directional_rows() {
        assert_eq!(
            super::cursor_id_for_feedback(CursorFeedbackKind::Scroll(ScrollDir::NE)),
            Some(CursorId::ScrollNE),
        );
        assert_eq!(
            super::cursor_id_for_feedback(CursorFeedbackKind::ScrollBlocked(ScrollDir::NE)),
            Some(CursorId::NoMoveNE),
        );
        assert_eq!(
            super::cursor_id_for_feedback(CursorFeedbackKind::ScrollBlocked(ScrollDir::S)),
            Some(CursorId::NoMoveS),
        );
    }
}
