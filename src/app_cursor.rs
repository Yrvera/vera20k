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
    if state.middle_mouse_panning {
        return Some(CursorFeedbackKind::Pan);
    }
    if state.minimap_dragging || is_cursor_over_minimap(state) {
        // Show the minimap-specific Move cursor when hovering over the minimap
        // (reference §7.4 — MiniFrame/MiniCount for the Move cursor = frames 42–51).
        return Some(CursorFeedbackKind::MinimapMove);
    }
    // Edge-scroll arrows override everything else (except minimap above).
    if let Some(dir) = edge_scroll_direction(state) {
        return Some(CursorFeedbackKind::Scroll(dir));
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
    let Some(sim) = &state.simulation else {
        return None;
    };
    let selected = crate::app_input::selected_stable_ids_sorted(&sim.entities);
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
    // Force-fire override: when Ctrl is held (and Alt isn't), show the attack
    // cursor over allies, own units, and empty cells. Only fires if the
    // selection has at least one armed unit (gamemd
    // SelectBestObjectForAction priority: armed mobile = 5 wins the cursor
    // source). Placed AFTER the shroud check above so over-shroud Ctrl-hold
    // falls through to the queued_order_mode cursor already chosen at lines
    // 79-84 — that branch already returned by here.
    if crate::app_input::is_ctrl_held(state) && !crate::app_input::is_alt_held(state) {
        let selection_has_armed_unit = sim.entities.values().filter(|e| e.selected).any(|e| {
            let type_str = sim.interner.resolve(e.type_ref);
            state
                .rules
                .as_ref()
                .and_then(|r| r.object(type_str))
                .is_some_and(|obj| obj.primary.is_some() || obj.secondary.is_some())
        });
        if selection_has_armed_unit {
            // EnemyUnit is the standard attack-reticle cursor; reuse it for
            // force-fire over allies/own/empty. (Exact mouse SHP frame for
            // gamemd's distinct action 0x33 is unverified — cosmetic-only
            // follow-up; tracked in the design doc.)
            return Some(CursorFeedbackKind::EnemyUnit);
        }
    }
    if let Some(hover) = crate::app_entity_pick::hover_target_at_point(
        sim,
        world_x,
        world_y,
        &owner,
        state.sandbox_full_visibility,
        state.rules.as_ref(),
        &state.height_map,
        Some(&state.tactical_bridge_inverse_map),
    ) {
        let kind = capability_cursor_for_hover(sim, &selected, &hover, state.rules.as_ref());
        return Some(kind);
    }
    // Check for ore/gem under cursor — show attack cursor when miners are selected.
    let has_ore = sim
        .production
        .resource_nodes
        .get(&(hover_rx, hover_ry))
        .is_some_and(|n| n.remaining > 0);
    if has_ore {
        let any_miner = selected
            .iter()
            .any(|&sid| sim.entities.get(sid).is_some_and(|e| e.miner.is_some()));
        if any_miner {
            return Some(CursorFeedbackKind::AttackMove);
        }
    }
    Some(match state.queued_order_mode {
        crate::app_render::OrderMode::Move => CursorFeedbackKind::Move,
        crate::app_render::OrderMode::AttackMove => CursorFeedbackKind::AttackMove,
        crate::app_render::OrderMode::Guard => CursorFeedbackKind::Guard,
    })
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
    hover: &crate::app_entity_pick::HoverTargetKindWithId,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> CursorFeedbackKind {
    use crate::map::entities::EntityCategory;

    let hovered_entity = sim.entities.get(hover.stable_id);
    let hovered_obj =
        rules.and_then(|r| hovered_entity.and_then(|e| r.object(sim.interner.resolve(e.type_ref))));

    // 1. Deployer self-hover — the cursor is over the selected unit itself.
    //    Show the deploy cursor for units with Deployer=yes (e.g. GGI, Guardian GI)
    //    OR units with DeploysInto= set (e.g. MCV → ConYard).  In the original game
    //    both kinds show the deploy cursor when hovering over themselves.
    if selected.len() == 1 && selected[0] == hover.stable_id {
        let entity = sim.entities.get(selected[0]);
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
            }
        }
    }

    // Pick the "best" selected unit for capability cursor checks.
    // Matches the original engine's SelectBestObjectForAction priority system.
    let hover_pos = hovered_entity.map(|e| (e.position.rx, e.position.ry));
    let best_id = select_best_for_action(sim, selected, hover_pos, rules);

    if let Some(best_id) = best_id {
        // DIAG c4-cursor: outer probe — fires for every hover, shows whether
        // the sel_obj lookup succeeds. Remove once C4 bug is diagnosed.
        log::info!(
            "c4-cursor: best_id={} sel_type={:?} obj_lookup={}",
            best_id,
            sim.entities
                .get(best_id)
                .map(|e| sim.interner.resolve(e.type_ref)),
            sim.entities
                .get(best_id)
                .and_then(|e| rules.and_then(|r| r.object(sim.interner.resolve(e.type_ref))))
                .is_some(),
        );
        if let (Some(sel_entity), Some(sel_obj)) = (
            sim.entities.get(best_id),
            sim.entities
                .get(best_id)
                .and_then(|e| rules.and_then(|r| r.object(sim.interner.resolve(e.type_ref)))),
        ) {
            // 2. C4 plant: SEAL / Tanya / Psi-Corp Trooper hovering an enemy
            //    structure with CanC4=yes, not InvisibleInGame, not iron-curtained.
            //    SabotageCursor flag remains in the data model (parsed in
            //    object_type.rs) for modder weapon-overlay use, but cursor
            //    logic is now driven by C4=yes — matches gamemd action 0x10.
            // DIAG c4-cursor: inner probe — shows each field of the C4 gate
            // so we can see which condition rejects the Demolish cursor.
            // Remove once C4 bug is diagnosed.
            log::info!(
                "c4-cursor: sel.c4={} hover.kind={:?} hover_type={:?} can_c4={:?} invis={:?} invuln={}",
                sel_obj.c4,
                hover.kind,
                hovered_entity.map(|e| sim.interner.resolve(e.type_ref)),
                hovered_obj.map(|o| o.can_c4),
                hovered_obj.map(|o| o.invisible_in_game),
                hovered_entity.is_some_and(|e| {
                    crate::sim::superweapon::invulnerability::is_invulnerable(
                        e.invulnerability.as_ref(),
                        sim.tick as u32,
                    )
                }),
            );
            if sel_obj.c4
                && matches!(hover.kind, HoverTargetKind::EnemyStructure)
                && hovered_obj.map_or(false, |o| o.can_c4 && !o.invisible_in_game)
                && !hovered_entity.is_some_and(|e| {
                    crate::sim::superweapon::invulnerability::is_invulnerable(
                        e.invulnerability.as_ref(),
                        sim.tick as u32,
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

            // 6. Infantry garrisoning a CanBeOccupied building (friendly or neutral/civilian).
            //    Original engine checks Occupier=yes via BuildingClass::CanDock.
            //    Neutral/civilian buildings are classified as EnemyStructure but still
            //    garrisonable — only show Enter for those, not actual enemy-player buildings.
            if is_infantry && sel_obj.occupier && hovered_obj.map_or(false, |o| o.can_be_occupied) {
                let is_garrisonable_target = match hover.kind {
                    HoverTargetKind::FriendlyStructure => true,
                    HoverTargetKind::EnemyStructure => {
                        // Only neutral/civilian buildings — not real enemy player buildings.
                        hovered_entity.map_or(false, |e| {
                            let ow = sim.interner.resolve(e.owner);
                            ow.eq_ignore_ascii_case("neutral") || ow.eq_ignore_ascii_case("special")
                        })
                    }
                    _ => false,
                };
                if is_garrisonable_target {
                    return CursorFeedbackKind::Enter;
                }
            }

            // 7. AttackCursorOnFriendlies — treat friendly targets as attack targets.
            if sel_obj.attack_cursor_on_friendlies {
                if matches!(
                    hover.kind,
                    HoverTargetKind::FriendlyUnit | HoverTargetKind::FriendlyStructure
                ) {
                    let in_range = any_selected_unit_in_range(
                        sim,
                        selected,
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
        }
    }

    // 9. Generic fallback.
    match hover.kind {
        HoverTargetKind::FriendlyUnit => CursorFeedbackKind::FriendlyUnit,
        HoverTargetKind::FriendlyStructure => CursorFeedbackKind::FriendlyStructure,
        HoverTargetKind::EnemyUnit | HoverTargetKind::EnemyStructure => {
            let in_range = any_selected_unit_in_range(
                sim,
                selected,
                hover.stable_id,
                rules,
                sim.resolved_terrain.as_ref(),
            );
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

/// Check if any selected unit has a weapon that can reach the target entity.
fn any_selected_unit_in_range(
    sim: &crate::sim::world::Simulation,
    selected_ids: &[u64],
    target_id: u64,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
) -> bool {
    let rules = match rules {
        Some(r) => r,
        None => return true,
    };
    let target_pos = match sim.entities.get(target_id) {
        Some(t) => (
            t.position.rx,
            t.position.ry,
            t.position.sub_x,
            t.position.sub_y,
        ),
        None => return false,
    };
    for &sid in selected_ids {
        let Some(entity) = sim.entities.get(sid) else {
            continue;
        };
        let Some(obj) = rules.object(sim.interner.resolve(entity.type_ref)) else {
            continue;
        };
        let weapon = match obj.primary.as_ref().and_then(|w| rules.weapon(w)) {
            Some(w) => w,
            None => continue,
        };
        if weapon.range <= crate::util::fixed_math::SIM_ZERO {
            continue;
        }
        let in_range = if let Some(t) = terrain {
            let src = (
                entity.position.rx as i64 * 256 + entity.position.sub_x.to_num::<i64>(),
                entity.position.ry as i64 * 256 + entity.position.sub_y.to_num::<i64>(),
                combat::in_range::effective_z_leptons(entity),
            );
            combat::in_range::compute_in_range(
                entity,
                src,
                &combat::TargetKind::Entity(target_id),
                weapon,
                rules,
                &sim.interner,
                &sim.entities,
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

/// Pick the single "best" selected object for determining the action cursor.
///
/// Matches the original engine's `SelectBestObjectForAction` (0x005353d0):
///   Priority 5 — mobile, not building, has weapons (WeaponRange > 0)
///   Priority 4 — mobile, not building
///   Priority 3 — can move (any mobile entity)
///   Priority 2 — exists on map
///   Priority 1 — deploying
///   Priority 0 — warping/teleporting
/// Ties within the same priority broken by closest distance to the hover target.
fn select_best_for_action(
    sim: &crate::sim::world::Simulation,
    selected: &[u64],
    hover_pos: Option<(u16, u16)>,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> Option<u64> {
    use crate::map::entities::EntityCategory;

    let mut best_id: Option<u64> = None;
    let mut best_priority: i32 = -1;
    let mut best_dist: u32 = u32::MAX;

    for &sid in selected {
        let Some(entity) = sim.entities.get(sid) else {
            continue;
        };
        // Compute priority tier.
        let priority = if entity.category == EntityCategory::Structure {
            // Buildings: can't move, priority 2 (exists on map).
            2
        } else {
            // Mobile unit: at least priority 3.
            let obj = rules.and_then(|r| r.object(sim.interner.resolve(entity.type_ref)));
            let has_weapon = obj
                .and_then(|o| o.primary.as_ref())
                .and_then(|w| rules.and_then(|r| r.weapon(w)))
                .is_some_and(|w| w.range > crate::util::fixed_math::SIM_ZERO);
            if has_weapon { 5 } else { 4 }
        };

        // Distance to hover target (squared, in cells).
        let dist = hover_pos.map_or(0u32, |(hx, hy)| {
            let dx = (entity.position.rx as i32 - hx as i32).unsigned_abs();
            let dy = (entity.position.ry as i32 - hy as i32).unsigned_abs();
            dx * dx + dy * dy
        });

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
/// Alias mappings live here: Pan→Move, Guard→Select, FriendlyUnit→Select, etc.
pub(crate) fn cursor_id_for_feedback(kind: CursorFeedbackKind) -> Option<CursorId> {
    match kind {
        CursorFeedbackKind::FriendlyUnit
        | CursorFeedbackKind::FriendlyStructure
        | CursorFeedbackKind::Guard => Some(CursorId::Select),
        CursorFeedbackKind::Move => Some(CursorId::Move),
        CursorFeedbackKind::Pan => Some(CursorId::Pan),
        CursorFeedbackKind::AttackMove => Some(CursorId::AttackMove),
        CursorFeedbackKind::EnemyUnit | CursorFeedbackKind::EnemyStructure => {
            Some(CursorId::Attack)
        }
        CursorFeedbackKind::EnemyOutOfRange => Some(CursorId::AttackOutOfRange),
        CursorFeedbackKind::Invalid => Some(CursorId::NoMove),
        CursorFeedbackKind::PlaceValid | CursorFeedbackKind::PlaceInvalid => None,
        CursorFeedbackKind::Scroll(dir) => Some(scroll_dir_to_cursor_id(dir)),
        CursorFeedbackKind::MinimapMove => Some(CursorId::MinimapMove),
        CursorFeedbackKind::Enter => Some(CursorId::Enter),
        CursorFeedbackKind::EngineerRepair => Some(CursorId::EngineerRepair),
        CursorFeedbackKind::Demolish => Some(CursorId::Demolish),
        CursorFeedbackKind::Deploy => Some(CursorId::Deploy),
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

pub(crate) fn current_software_cursor_frame(
    sequence: &SoftwareCursorSequence,
) -> Option<&SoftwareCursorFrame> {
    if sequence.frames.is_empty() {
        return None;
    }
    if sequence.frames.len() == 1 || sequence.interval_ms == 0 {
        return sequence.frames.first();
    }
    let elapsed_ms: u64 = cursor_animation_start()
        .elapsed()
        .as_millis()
        .try_into()
        .ok()?;
    let frame_idx = ((elapsed_ms / sequence.interval_ms) % sequence.frames.len() as u64) as usize;
    sequence.frames.get(frame_idx)
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

/// Screen margin (pixels from window edge) that triggers edge-scroll cursors.
/// Must match EDGE_SCROLL_MARGIN in app_sim_tick.rs.
const EDGE_SCROLL_MARGIN: f32 = 10.0;

/// Return the edge-scroll direction (if any) based on cursor proximity to window edges.
/// Diagonal corners are detected by combining horizontal and vertical proximity.
fn edge_scroll_direction(state: &AppState) -> Option<ScrollDir> {
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    let sidebar_x = sw - state.sidebar_layout_spec.sidebar_width;
    let x = state.cursor_x;
    let y = state.cursor_y;
    let near_left = x < EDGE_SCROLL_MARGIN;
    let near_right = x < sidebar_x && x > sidebar_x - EDGE_SCROLL_MARGIN;
    let near_top = y < EDGE_SCROLL_MARGIN;
    let near_bottom = y > sh - EDGE_SCROLL_MARGIN;
    match (near_left, near_right, near_top, near_bottom) {
        (true, _, true, _) => Some(ScrollDir::NW),
        (_, true, true, _) => Some(ScrollDir::NE),
        (true, _, _, true) => Some(ScrollDir::SW),
        (_, true, _, true) => Some(ScrollDir::SE),
        (_, _, true, _) => Some(ScrollDir::N),
        (_, _, _, true) => Some(ScrollDir::S),
        (true, _, _, _) => Some(ScrollDir::W),
        (_, true, _, _) => Some(ScrollDir::E),
        _ => None,
    }
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
    use super::{capability_cursor_for_hover, super_weapon_cursor_id};
    use crate::app_entity_pick::HoverTargetKindWithId;
    use crate::app_types::{CursorFeedbackKind, CursorId, HoverTargetKind};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::sim::world::Simulation;
    use std::collections::BTreeMap;

    /// Repro for "SEAL right-click on enemy buildings does nothing".
    /// Loads the actual retail INIs the runtime uses, spawns a SEAL via
    /// `spawn_object` (the same code path the barracks uses on production
    /// completion), then calls `capability_cursor_for_hover` and asserts
    /// the returned cursor is `Demolish`. If it isn't, the body of the
    /// function prints which gate condition rejected it.
    #[test]
    fn seal_hovering_enemy_building_shows_demolish() {
        // 1. Load and merge retail INIs (same as app_init_helpers::load_rules_ini).
        let base = std::fs::read_to_string("ini/rules.ini").expect("ini/rules.ini");
        let patch = std::fs::read_to_string("ini/rulesmd.ini").expect("ini/rulesmd.ini");
        let mut ini = IniFile::from_str(&base);
        let patch_ini = IniFile::from_str(&patch);
        ini.merge(&patch_ini);
        let mut rules = RuleSet::from_ini(&ini).expect("parse merged rules");

        // 2. Build a Simulation. resolve_bridge_warheads is required by the
        //    c4 tick path even though we don't tick here — keeps the sim in a
        //    consistent state with what the runtime would see.
        let mut sim = Simulation::new();
        rules.resolve_bridge_warheads(&mut sim.interner);
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
        if let Some(e) = sim.entities.get_mut(seal_id) {
            e.selected = true;
        }

        // 5. Construct the hover descriptor the runtime would build when
        //    the cursor is over an enemy building.
        let hover = HoverTargetKindWithId {
            kind: HoverTargetKind::EnemyStructure,
            stable_id: bld_id,
        };

        // 6. Call the same function the live cursor pipeline calls.
        let result = capability_cursor_for_hover(&sim, &[seal_id], &hover, Some(&rules));

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
        let base = std::fs::read_to_string("ini/rules.ini").expect("ini/rules.ini");
        let patch = std::fs::read_to_string("ini/rulesmd.ini").expect("ini/rulesmd.ini");
        let mut ini = IniFile::from_str(&base);
        let patch_ini = IniFile::from_str(&patch);
        ini.merge(&patch_ini);
        let mut rules = RuleSet::from_ini(&ini).expect("parse merged rules");

        let mut sim = Simulation::new();
        rules.resolve_bridge_warheads(&mut sim.interner);
        let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();

        let miner_id = sim
            .spawn_object("CMIN", "Americans", 5, 5, 0, &rules, &height_map)
            .expect("Chrono Miner spawned");
        let refinery_id = sim
            .spawn_object("GAREFN", "Americans", 10, 10, 0, &rules, &height_map)
            .expect("Refinery spawned");

        if let Some(e) = sim.entities.get_mut(miner_id) {
            e.selected = true;
        }

        let hover = HoverTargetKindWithId {
            kind: HoverTargetKind::FriendlyStructure,
            stable_id: refinery_id,
        };

        let result = capability_cursor_for_hover(&sim, &[miner_id], &hover, Some(&rules));
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
}
