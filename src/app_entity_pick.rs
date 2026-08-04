//! Entity picking, hover-target resolution, and selection snapshot computation.
//!
//! Handles click and box selection logic, enemy target picking for attack
//! commands, and hover-target classification (friendly/enemy/structure/unit).
//!
//! Extracted from app_render.rs to keep files under 400 lines.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app_types::HoverTargetKind;
use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::InternedId;
use crate::sim::vision::FogState;

/// Elliptical distance threshold for picking objects at a screen point.
/// `dx*dx + 0.5*dy*dy < 200` with 0.5 Y-weight compensating for isometric projection.
/// Max horizontal reach: sqrt(200) ≈ 14px, max vertical reach: sqrt(400) = 20px.
const PICK_DISTANCE_THRESHOLD: f32 = 200.0;
type TacticalBridgeInverseMap =
    std::collections::BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell>;
/// The simulation's house table, as the picker needs it: owner → player state.
type HouseStates = std::collections::BTreeMap<InternedId, crate::sim::house_state::HouseState>;

/// Y-axis weight in the elliptical pick distance formula.
/// Makes the hit zone taller than wide, matching the isometric projection where
/// units appear elongated vertically.
const PICK_Y_WEIGHT: f32 = 0.5;

/// Compute the elliptical pick distance for object selection.
/// Returns `dx² + 0.5 * dy²` — an ellipse wider vertically than horizontally.
fn pick_distance_sq(dx: f32, dy: f32) -> f32 {
    dx * dx + PICK_Y_WEIGHT * dy * dy
}

pub(crate) fn pick_enemy_target_stable_id(
    sim: &crate::sim::world::Simulation,
    world_x: f32,
    world_y: f32,
    friendly_owner: &str,
    ignore_visibility: bool,
    rules: Option<&RuleSet>,
    height_map: &std::collections::BTreeMap<(u16, u16), u8>,
    bridge_height_map: Option<&TacticalBridgeInverseMap>,
) -> Option<u64> {
    hover_target_at_point(
        sim,
        world_x,
        world_y,
        friendly_owner,
        ignore_visibility,
        rules,
        height_map,
        bridge_height_map,
    )
    .and_then(|hover| match hover.kind {
        HoverTargetKind::EnemyUnit | HoverTargetKind::EnemyStructure => Some(hover.stable_id),
        _ => None,
    })
}

/// Force-fire: pick any entity under cursor (including friendlies).
pub(crate) fn pick_any_target_stable_id(
    sim: &crate::sim::world::Simulation,
    world_x: f32,
    world_y: f32,
    ignore_visibility: bool,
    rules: Option<&RuleSet>,
    height_map: &std::collections::BTreeMap<(u16, u16), u8>,
    bridge_height_map: Option<&TacticalBridgeInverseMap>,
) -> Option<u64> {
    // Use empty owner so everything is considered "enemy" in hover logic.
    hover_target_at_point(
        sim,
        world_x,
        world_y,
        "",
        ignore_visibility,
        rules,
        height_map,
        bridge_height_map,
    )
    .filter(|hover| hover.kind != HoverTargetKind::HiddenEnemy)
    .map(|hover| hover.stable_id)
}

pub(crate) fn hover_target_at_point(
    sim: &crate::sim::world::Simulation,
    world_x: f32,
    world_y: f32,
    local_owner: &str,
    ignore_visibility: bool,
    rules: Option<&RuleSet>,
    height_map: &std::collections::BTreeMap<(u16, u16), u8>,
    bridge_height_map: Option<&TacticalBridgeInverseMap>,
) -> Option<HoverTargetKindWithId> {
    let local_owner_id: InternedId = sim.interner.get(local_owner).unwrap_or_default();
    let mut best: Option<(u64, f32)> = None;
    for entity in sim.entities().values() {
        let is_structure = entity.category == EntityCategory::Structure;
        let type_str = sim.interner.resolve(entity.type_ref);
        let owner_str = sim.interner.resolve(entity.owner);
        let (sx, sy) = if is_structure {
            crate::render::locomotor_visual::screen_position(entity)
        } else {
            crate::app_instances::interpolated_screen_position_entity(entity)
        };
        // Hit test: structures use foundation cells, mobile units use elliptical distance.
        if is_structure {
            let foundation = rules
                .and_then(|r| r.object(type_str))
                .map(|o| o.foundation.as_str())
                .unwrap_or("1x1");
            if !click_hits_foundation(
                world_x,
                world_y,
                entity.position.rx,
                entity.position.ry,
                foundation,
                height_map,
                bridge_height_map,
            ) {
                continue;
            }
        } else {
            let dx = sx - world_x;
            let dy = sy - world_y;
            if pick_distance_sq(dx, dy) >= PICK_DISTANCE_THRESHOLD {
                continue;
            }
        }
        // Distance for tie-breaking (prefer closest to anchor point).
        let dx = sx - world_x;
        let dy = sy - world_y;
        let dist_sq = dx * dx + dy * dy;
        let is_friendly = sim.fog.is_friendly(local_owner, owner_str);
        let is_visible = ignore_visibility
            || (sim
                .fog
                .is_cell_revealed(local_owner_id, entity.position.rx, entity.position.ry)
                && !sim.fog.is_cell_gap_covered(
                    local_owner_id,
                    entity.position.rx,
                    entity.position.ry,
                ));
        let kind = if is_friendly {
            if is_structure {
                HoverTargetKind::FriendlyStructure
            } else {
                HoverTargetKind::FriendlyUnit
            }
        } else if !is_visible {
            HoverTargetKind::HiddenEnemy
        } else if is_structure {
            HoverTargetKind::EnemyStructure
        } else {
            HoverTargetKind::EnemyUnit
        };
        match best {
            Some((_, best_dist_sq)) if dist_sq >= best_dist_sq => {}
            _ => best = Some((encode_hover_kind_with_id(kind, entity.stable_id), dist_sq)),
        }
    }
    best.map(|(encoded, _)| decode_hover_kind_with_id(encoded))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HoverTargetKindWithId {
    pub(crate) kind: HoverTargetKind,
    pub(crate) stable_id: u64,
}

fn encode_hover_kind_with_id(kind: HoverTargetKind, stable_id: u64) -> u64 {
    let kind_bits = match kind {
        HoverTargetKind::FriendlyUnit => 1u64,
        HoverTargetKind::FriendlyStructure => 2,
        HoverTargetKind::EnemyUnit => 3,
        HoverTargetKind::EnemyStructure => 4,
        HoverTargetKind::HiddenEnemy => 5,
    };
    (stable_id << 3) | kind_bits
}

fn decode_hover_kind_with_id(encoded: u64) -> HoverTargetKindWithId {
    let kind = match encoded & 0b111 {
        1 => HoverTargetKind::FriendlyUnit,
        2 => HoverTargetKind::FriendlyStructure,
        3 => HoverTargetKind::EnemyUnit,
        4 => HoverTargetKind::EnemyStructure,
        _ => HoverTargetKind::HiddenEnemy,
    };
    HoverTargetKindWithId {
        kind,
        stable_id: encoded >> 3,
    }
}

pub(crate) fn compute_click_selection_snapshot(
    entities: &EntityStore,
    fog: Option<&FogState>,
    local_owner: Option<&str>,
    world_x: f32,
    world_y: f32,
    click_radius: f32,
    additive: bool,
    rules: Option<&RuleSet>,
    houses: Option<&HouseStates>,
    height_map: &std::collections::BTreeMap<(u16, u16), u8>,
    bridge_height_map: Option<&TacticalBridgeInverseMap>,
    interner: Option<&crate::sim::intern::StringInterner>,
) -> Option<Vec<u64>> {
    let current: Vec<u64> = selected_stable_ids_sorted_from_store(entities);
    let Some(picked_sid) = pick_entity_at_point(
        entities,
        fog,
        local_owner,
        world_x,
        world_y,
        click_radius,
        rules,
        houses,
        height_map,
        bridge_height_map,
        interner,
    ) else {
        // Non-shift click clears selection; shift-click with no hit keeps it.
        return if additive {
            None
        } else if current.is_empty() {
            None
        } else {
            Some(Vec::new())
        };
    };
    let mut out = if additive {
        current.clone()
    } else {
        Vec::new()
    };
    if additive {
        if let Some(idx) = out.iter().position(|v| *v == picked_sid) {
            out.remove(idx);
        } else {
            out.push(picked_sid);
        }
    } else {
        out.push(picked_sid);
    }
    out.sort_unstable();
    out.dedup();
    Some(out)
}

/// Did the band box catch anything at all?
///
/// gamemd asks this before it clears the selection, and the question is far
/// looser than "which objects join the selection": it walks the drawn-object
/// list and accepts any live object inside the rectangle — no owner filter, no
/// `Selectable=`, no limbo or docked test. A rectangle holding nothing but enemy
/// units, or a single building, still counts as caught and still replaces the
/// selection. A rectangle holding nothing at all leaves the selection untouched
/// and the release falls through to the ordinary click/action path.
///
/// Limbo is the one exclusion, and it is a registration property rather than a
/// filter: an object without map presence is never in the drawn-object list, so
/// a passenger inside a transport cannot make a rectangle "non-empty".
///
/// The list is **partly** visibility-filtered, and the asymmetry is native:
/// mobile objects append themselves from inside their own draw routine, so an
/// object the shroud hides is never registered, while buildings are appended in
/// bulk from the whole building array with no visibility test at all. A building
/// under unexplored shroud therefore does make a rectangle non-empty, and an
/// enemy unit under it does not.
pub(crate) fn band_rect_contains_drawn_object(
    entities: &EntityStore,
    fog: Option<&FogState>,
    local_owner: Option<&str>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    interner: Option<&crate::sim::intern::StringInterner>,
) -> bool {
    let local_owner_id = local_owner.and_then(|o| interner.and_then(|i| i.get(o)));
    entities.values().any(|entity| {
        if !entity.lifecycle.object_alive || entity.lifecycle.in_limbo {
            return false;
        }
        let (sx, sy) = crate::app_instances::interpolated_screen_position_entity(entity);
        if !(sx >= min_x && sx <= max_x && sy >= min_y && sy <= max_y) {
            return false;
        }
        if entity.category == EntityCategory::Structure {
            // Bulk-registered, shroud or no shroud.
            return true;
        }
        let owner_str = interner.map_or("", |i| i.resolve(entity.owner));
        is_drawn_for_local_owner(fog, local_owner, owner_str, entity, local_owner_id)
    })
}

/// Would the local player see this object drawn right now?
///
/// Deliberately distinct from the eligibility gate's visibility tail: with no
/// fog state at all (the sandbox full-visibility switch) everything is drawn,
/// where the eligibility gate falls back to an owner test.
fn is_drawn_for_local_owner(
    fog: Option<&FogState>,
    local_owner: Option<&str>,
    entity_owner: &str,
    entity: &crate::sim::game_entity::GameEntity,
    local_owner_id: Option<InternedId>,
) -> bool {
    let Some(local_owner) = local_owner else {
        return true;
    };
    let Some(fog) = fog else {
        // Sandbox full visibility: everything is drawn.
        return true;
    };
    if fog.is_friendly(local_owner, entity_owner) {
        return true;
    }
    let owner_id = local_owner_id.unwrap_or_default();
    let pos = &entity.position;
    fog.is_cell_revealed(owner_id, pos.rx, pos.ry)
        && !fog.is_cell_gap_covered(owner_id, pos.rx, pos.ry)
}

/// Resolve a band-box release into the selection snapshot to commit.
///
/// Three outcomes, matching the native release:
/// * `None` — the rectangle caught no drawn object (or a shift-drag added
///   nothing). The selection is left exactly as it was.
/// * `Some(list)` — the rectangle caught something. A plain drag replaces the
///   selection with the eligible members; a shift drag **adds** them.
///
/// A rectangle that caught something but nothing eligible — a box over an enemy
/// patrol, say — returns an empty list, because the native clear runs before the
/// per-object filter does.
pub(crate) fn compute_box_selection_snapshot(
    entities: &EntityStore,
    fog: Option<&FogState>,
    local_owner: Option<&str>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    additive: bool,
    rules: Option<&RuleSet>,
    houses: Option<&HouseStates>,
    interner: Option<&crate::sim::intern::StringInterner>,
) -> Option<Vec<u64>> {
    if !band_rect_contains_drawn_object(
        entities,
        fog,
        local_owner,
        min_x,
        min_y,
        max_x,
        max_y,
        interner,
    ) {
        return None;
    }
    let current: Vec<u64> = selected_stable_ids_sorted_from_store(entities);
    let candidates = entities_in_rect(
        entities,
        fog,
        local_owner,
        min_x,
        min_y,
        max_x,
        max_y,
        rules,
        houses,
        interner,
    );
    if additive {
        // The native band callback only ever calls Select — there is no Deselect
        // anywhere on this path, and the shift branch merely skips the clear. So
        // a shift drag across units already in the group keeps them.
        let mut out = current;
        let mut added = false;
        for sid in candidates {
            if !out.contains(&sid) {
                out.push(sid);
                added = true;
            }
        }
        if !added {
            return None;
        }
        out.sort_unstable();
        out.dedup();
        return Some(out);
    }
    let mut out = candidates;
    out.sort_unstable();
    out.dedup();
    Some(out)
}

/// Get sorted stable IDs of selected entities from EntityStore.
fn selected_stable_ids_sorted_from_store(entities: &EntityStore) -> Vec<u64> {
    let mut ids: Vec<u64> = entities
        .values()
        .filter(|e| e.selected)
        .map(|e| e.stable_id)
        .collect();
    ids.sort_unstable();
    ids
}

fn entities_in_rect(
    entities: &EntityStore,
    fog: Option<&FogState>,
    local_owner: Option<&str>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    rules: Option<&RuleSet>,
    houses: Option<&HouseStates>,
    interner: Option<&crate::sim::intern::StringInterner>,
) -> Vec<u64> {
    let local_owner_id = local_owner.and_then(|o| interner.and_then(|i| i.get(o)));
    entities
        .values()
        .filter_map(|entity| {
            // Rectangle first, the way the native walk does it: the per-object
            // filter only runs for objects the box actually covers.
            let (sx, sy) = crate::app_instances::interpolated_screen_position_entity(entity);
            if !(sx >= min_x && sx <= max_x && sy >= min_y && sy <= max_y) {
                return None;
            }
            let owner_str = interner.map_or("", |i| i.resolve(entity.owner));
            let type_str = interner.map_or("", |i| i.resolve(entity.type_ref));
            if !is_selectable_entity(
                fog,
                local_owner,
                owner_str,
                entity,
                type_str,
                rules,
                houses,
                local_owner_id,
            ) {
                return None;
            }
            // Structures excluded from band-box selection (RA2 convention).
            if entity.category == EntityCategory::Structure {
                return None;
            }
            if !can_be_selected_now(entity, entities, rules, interner) {
                return None;
            }
            Some(entity.stable_id)
        })
        .collect()
}

/// The band-box-only half of the eligibility chain.
///
/// A single click reaches an object through a shorter chain than a band box
/// does: only the band path asks `CanBeSelectedNow`, which refuses an object
/// that is enslaved, mid-way through a deploy transition, linked into a bunker,
/// or radio-docked on a building. The docked clause is the one players feel — it
/// keeps the miner unloading on the refinery pad out of a box dragged across the
/// base, while a direct click on that same miner still selects it.
///
/// The radio-dock clause is a **pair** of tests, and both halves matter: the
/// dock flag alone is not enough, the object also has to be standing on a
/// building. That second half is what keeps the rule narrow — the same flag is
/// raised on a vehicle leaving a war factory, and such a vehicle stops being
/// refused the moment it drives clear of the factory footprint, exactly as the
/// native cell lookup behaves.
///
/// Two native clauses are approximated rather than reproduced, because VERA has
/// no matching state:
/// * The native enslaved test reads a slave-owner pointer that every slave
///   carries. VERA's nearest state is the slave-harvester component, which only
///   the Slave Miner's slaves own; a mind-controlled unit is a different field
///   in the original and is deliberately not tested here.
/// * The native bunker test reads one link field. VERA splits the link into an
///   approach marker and an installed link, and only the installed link is
///   treated as linked.
fn can_be_selected_now(
    entity: &crate::sim::game_entity::GameEntity,
    entities: &EntityStore,
    rules: Option<&RuleSet>,
    interner: Option<&crate::sim::intern::StringInterner>,
) -> bool {
    use crate::sim::deploy::DeployPhase;
    if entity.slave_harvester.is_some() {
        return false;
    }
    if matches!(
        entity.deploy_state,
        Some(DeployPhase::Deploying { .. } | DeployPhase::Undeploying { .. })
    ) {
        return false;
    }
    if entity.bunker_link.installed_in().is_some() {
        return false;
    }
    if entity.dock_entered_with.is_some()
        && building_covers_cell(
            entities,
            rules,
            interner,
            entity.position.rx,
            entity.position.ry,
        )
    {
        return false;
    }
    true
}

/// Does a building's footprint cover this cell? The native lookup asks the cell
/// itself; VERA has no cell→occupier index on this path, so the building list is
/// walked instead. It only runs for an object that already carries the dock
/// flag, which is a handful of objects in a normal match.
fn building_covers_cell(
    entities: &EntityStore,
    rules: Option<&RuleSet>,
    interner: Option<&crate::sim::intern::StringInterner>,
    rx: u16,
    ry: u16,
) -> bool {
    entities.values().any(|candidate| {
        if candidate.category != EntityCategory::Structure || candidate.lifecycle.in_limbo {
            return false;
        }
        let type_str = interner.map_or("", |i| i.resolve(candidate.type_ref));
        let foundation = rules
            .and_then(|r| r.object(type_str))
            .map(|o| o.foundation.as_str())
            .unwrap_or("1x1");
        let (fw, fh) = crate::rules::foundation::foundation_dimensions(foundation);
        let brx = candidate.position.rx;
        let bry = candidate.position.ry;
        rx >= brx && rx < brx.saturating_add(fw) && ry >= bry && ry < bry.saturating_add(fh)
    })
}

/// Check if a world-space click point falls on a building's foundation cells.
/// The building occupies cells `(rx..rx+fw, ry..ry+fh)`. We convert the click
/// to cell coords and check containment. This matches the original engine which
/// uses the foundation footprint, not the visual sprite bounds.
fn click_hits_foundation(
    world_x: f32,
    world_y: f32,
    entity_rx: u16,
    entity_ry: u16,
    foundation: &str,
    height_map: &std::collections::BTreeMap<(u16, u16), u8>,
    bridge_height_map: Option<&TacticalBridgeInverseMap>,
) -> bool {
    let (fw, fh) = crate::rules::foundation::foundation_dimensions(foundation);
    let (click_rx, click_ry) =
        crate::app_sim_tick::world_point_to_cell(world_x, world_y, height_map, bridge_height_map);
    let crx = click_rx as i32;
    let cry = click_ry as i32;
    let brx = entity_rx as i32;
    let bry = entity_ry as i32;
    crx >= brx && crx < brx + fw as i32 && cry >= bry && cry < bry + fh as i32
}

fn pick_entity_at_point(
    entities: &EntityStore,
    fog: Option<&FogState>,
    local_owner: Option<&str>,
    world_x: f32,
    world_y: f32,
    click_radius: f32,
    rules: Option<&RuleSet>,
    houses: Option<&HouseStates>,
    height_map: &std::collections::BTreeMap<(u16, u16), u8>,
    bridge_height_map: Option<&TacticalBridgeInverseMap>,
    interner: Option<&crate::sim::intern::StringInterner>,
) -> Option<u64> {
    // Elliptical pick distance: dx² + 0.5*dy² < 200, with the 0.5 Y-weight
    // compensating for isometric projection.
    // The click_radius parameter is kept for API compatibility but not used.
    let _ = click_radius;
    let local_owner_id = local_owner.and_then(|o| interner.and_then(|i| i.get(o)));
    let mut best_mobile: Option<(u64, f32)> = None;
    let mut best_structure: Option<(u64, f32)> = None;

    for entity in entities.values() {
        let owner_str = interner.map_or("", |i| i.resolve(entity.owner));
        let type_str = interner.map_or("", |i| i.resolve(entity.type_ref));
        if !is_selectable_entity(
            fog,
            local_owner,
            owner_str,
            entity,
            type_str,
            rules,
            houses,
            local_owner_id,
        ) {
            continue;
        }
        let is_structure = entity.category == EntityCategory::Structure;
        if is_structure {
            // Foundation-based hit test: click must land on one of the building's
            // foundation cells.
            let foundation = rules
                .and_then(|r| r.object(type_str))
                .map(|o| o.foundation.as_str())
                .unwrap_or("1x1");
            if !click_hits_foundation(
                world_x,
                world_y,
                entity.position.rx,
                entity.position.ry,
                foundation,
                height_map,
                bridge_height_map,
            ) {
                continue;
            }
            let (sx, sy) = crate::render::locomotor_visual::screen_position(entity);
            let dx = sx - world_x;
            let dy = sy - world_y;
            let dist_sq = pick_distance_sq(dx, dy);
            match best_structure {
                Some((_, best_dist)) if dist_sq >= best_dist => {}
                _ => best_structure = Some((entity.stable_id, dist_sq)),
            }
        } else {
            let (sx, sy) = crate::app_instances::interpolated_screen_position_entity(entity);
            let dx = sx - world_x;
            let dy = sy - world_y;
            let dist_sq = pick_distance_sq(dx, dy);
            if dist_sq < PICK_DISTANCE_THRESHOLD {
                match best_mobile {
                    Some((_, best_dist)) if dist_sq >= best_dist => {}
                    _ => best_mobile = Some((entity.stable_id, dist_sq)),
                }
            }
        }
    }

    best_mobile.or(best_structure).map(|(sid, _)| sid)
}

/// Selection eligibility for picking — the `TechnoClass::Select` and
/// `ObjectClass::Select` rejections that depend on the object itself, followed
/// by the visibility check.
///
/// gamemd refuses to select an object owned by a house that is not a human
/// player (so an enemy AI tank never joins a band-box), one that is in limbo
/// (loaded into a transport or garrison, so it has no map presence), or one
/// whose type says `Selectable=no` (the scripted paradrop and spy planes, walls,
/// civilian props). The already-selected rejection is not mirrored here: it
/// exists to keep duplicates out of the native selection array, and a picked
/// snapshot is deduplicated and re-committed wholesale.
///
/// A house the caller does not know about is treated as human, matching the sim
/// commit; the commit is authority either way.
fn is_selectable_entity(
    fog: Option<&FogState>,
    local_owner: Option<&str>,
    entity_owner: &str,
    entity: &crate::sim::game_entity::GameEntity,
    entity_type: &str,
    rules: Option<&RuleSet>,
    houses: Option<&HouseStates>,
    local_owner_id: Option<InternedId>,
) -> bool {
    if houses.is_some_and(|h| h.get(&entity.owner).is_some_and(|house| !house.is_human)) {
        return false;
    }
    if entity.lifecycle.in_limbo {
        return false;
    }
    let type_selectable =
        rules.is_none_or(|r| r.object(entity_type).is_none_or(|obj| obj.selectable));
    if !type_selectable {
        return false;
    }
    let pos = &entity.position;
    let Some(local_owner) = local_owner else {
        return true;
    };
    let Some(fog) = fog else {
        return entity_owner.eq_ignore_ascii_case(local_owner);
    };
    if fog.is_friendly(local_owner, entity_owner) {
        return true;
    }
    let owner_id = local_owner_id.unwrap_or_default();
    fog.is_cell_revealed(owner_id, pos.rx, pos.ry)
        && !fog.is_cell_gap_covered(owner_id, pos.rx, pos.ry)
}
