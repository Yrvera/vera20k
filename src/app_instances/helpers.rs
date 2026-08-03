//! Shared helpers for instance builders — depth sorting, interpolation, visibility.
//!
//! These utilities are used by the unit, SHP, and overlay instance builders.
//! Extracted from app_instances.rs to keep files under the 600-line limit.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::map::terrain;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::components::Position;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::InternedId;
use crate::sim::vision::FogState;
use crate::util::fixed_math::SIM_ZERO;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellVisibilityState {
    Visible,
    Shrouded,
}

/// Which display band an entity's body is drawn in.
///
/// gamemd keeps five display layers and asks each object which one it belongs
/// to. `UnitClass`, `InfantryClass` and `AircraftClass` all forward that
/// question straight to the attached locomotor, so the answer is a property of
/// the locomotor rather than of the unit category:
///
/// * Drive, Walk, Hover, Ship, Mech and Teleport are Ground (layer 2)
///   unconditionally — each of those locomotor slots is a two-instruction
///   `return 2`.
/// * Fly is Top (layer 4) the moment its object's height is above zero and
///   Ground otherwise. It never consults the in-air flag, so a landed aircraft
///   is an ordinary Ground object that sorts with the tanks around it.
/// * Jumpjet is Ground while grounded, then Air (layer 3) climbing and Top once
///   it reaches its own hover height.
/// * Rocket is Air. So is DropPod, which is unreachable in stock YR.
/// * A parachuting infantryman keeps its Walk locomotor, so it stays Ground
///   despite hanging in the air — its altitude changes only where it is drawn.
///
/// Only layer 2 is kept sorted; the rest append and render in submission order,
/// and every layer above 2 is drawn after all of layer 2. Air and Top are
/// therefore indistinguishable as far as ground objects are concerned, and we
/// have no separate Air object band, so both collapse into [`EntityDrawBand::Top`]
/// here. The only ordering that collapse can disturb is Air-vs-Top against the
/// layer-3 particle stream, which in stock YR is a takeoff's worth of frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntityDrawBand {
    /// gamemd layer 2 — the single Y-sorted band that holds buildings,
    /// vehicles, infantry on foot and landed aircraft.
    Ground,
    /// gamemd layers 3 and 4 — drawn after every Ground object.
    Top,
}

/// Which band this entity's body belongs to, per gamemd's per-locomotor answer.
///
/// The state read here mirrors what `render::locomotor_visual` reads to decide
/// what is holding the entity up, because that is the same question: an entity
/// is above the Ground band exactly when something has lifted it off the floor.
pub(crate) fn entity_draw_band(entity: &GameEntity) -> EntityDrawBand {
    // Object-level falling. The locomotor underneath is still Walk or Drive,
    // both of which answer Ground, so a paradrop never leaves layer 2.
    if entity.parachute_state.is_some() {
        return EntityDrawBand::Ground;
    }
    // Scripted missiles fly on the Rocket locomotor, which answers Air.
    if entity.rocket_state.is_some() {
        return EntityDrawBand::Top;
    }
    let Some(loco) = entity.locomotor.as_ref() else {
        return EntityDrawBand::Ground;
    };
    match loco.kind {
        // The Rocket slot answers Air unconditionally — it has no height test
        // of its own. Reachable only if a missile is ever built locomotor-first,
        // since `rocket_state` answers above.
        LocomotorKind::Rocket => EntityDrawBand::Top,
        // The two flying locomotors gate on height, not on the locomotor's
        // nominal layer: `MovementLayer::Air` is fixed at construction for these
        // kinds, so a Harrier parked on its pad still carries it.
        LocomotorKind::Fly | LocomotorKind::Jumpjet => {
            if loco.altitude > SIM_ZERO {
                EntityDrawBand::Top
            } else {
                EntityDrawBand::Ground
            }
        }
        _ => EntityDrawBand::Ground,
    }
}

/// Convert the screen row an entity is *drawn* at into the row it *sorts* at.
///
/// gamemd's sort key is `renderCoords.X + renderCoords.Y` — the object's world
/// position, with the Z component structurally absent from the sum, for every
/// class that does not override the key. The row an entity is drawn at is not
/// that row: `render::locomotor_visual` lifts the projection by the entity's
/// height first, 216 px for an aircraft at stock `FlightLevel=1500`. Feeding
/// the drawn row into the depth key would sort that aircraft ~14 iso rows
/// behind its own cell, where later ground objects cover it and the terrain
/// depth buffer clips it. Adding the lift back recovers the ground row, which
/// is the screen-space image of X+Y.
///
/// This also takes the infantry walking bob back out of the key. The bob is a
/// VERA-internal ±1 px flourish; gamemd's key has no term for it.
pub(crate) fn ground_sort_row(entity: &GameEntity, drawn_row_y: f32) -> f32 {
    drawn_row_y + crate::render::locomotor_visual::height_lift_px(entity)
}

/// Compute depth for a sprite from screen position.
///
/// The depth value serves two roles: it is the painter's sort key for
/// sprite-vs-sprite ordering (merge pass sorts instances by depth
/// descending — largest = furthest back = drawn first), and it feeds the
/// terrain-occlusion (cliff) depth test. Sprites do not write the depth
/// buffer themselves.
///
/// Lower screen_y → larger depth (further from camera).
/// Higher elevation (z) → slightly smaller depth (closer to camera).
pub(crate) fn compute_sprite_depth(state: &AppState, screen_y: f32, z: u8) -> f32 {
    let (origin_y, world_height) = state
        .terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));
    compute_sprite_depth_params(origin_y, world_height, screen_y, z)
}

/// Compute sprite depth from explicit parameters.
/// Same formula as `compute_sprite_depth` but for callers that already have
/// origin_y and world_height (avoids re-extracting from AppState).
pub(crate) fn compute_sprite_depth_params(
    origin_y: f32,
    world_height: f32,
    screen_y: f32,
    z: u8,
) -> f32 {
    let iso_row: f32 = screen_y + z as f32 * terrain::HEIGHT_STEP;
    let normalized: f32 = ((iso_row - origin_y) / world_height).clamp(0.0, 1.0);
    let z_bias: f32 = z as f32 * 0.0001;
    (1.0 - normalized - z_bias).clamp(0.001, 0.999)
}

/// Extra depth bias carried by every anim SHP draw in the original engine's
/// standard shape-depth expression, on top of the anim's own `ZAdjust=`.
pub(crate) const ANIM_DRAW_DEPTH_BIAS_PX: i32 = -2;

/// Apply a native `ZAdjust=` depth-sort bias to a computed sprite depth.
///
/// The original engine composes a draw's sort value as a cell/row base plus a
/// signed pixel bias, with the height correction subtracted — smaller value =
/// closer to the camera. Our normalized depth axis points the same way (lower
/// = closer) and the base depth already encodes the row term, so a ZAdjust of
/// N pixels maps to a depth delta of `N / world_height`. Negative ZAdjust
/// pulls the sprite toward the camera (damage fires, muzzle flashes, arrows
/// and parachutes all use negative values to draw in front).
///
/// Note: 1000 is NOT a neutral value here — that convention belongs to the
/// per-cell terrain z path, which is a separate mechanism. Neutral is 0.
pub(crate) fn apply_shape_z_adjust(depth: f32, z_adjust_px: i32, world_height: f32) -> f32 {
    (depth + z_adjust_px as f32 / world_height.max(1.0)).clamp(0.001, 0.999)
}

/// Effective anim `ZAdjust`: a nonzero per-slot override (e.g. a building's
/// `ActiveAnimZAdjust=`) wins; zero falls back to the anim type's own
/// `ZAdjust=` from its art section.
pub(crate) fn effective_anim_z_adjust(slot_z_adjust: i32, type_z_adjust: i32) -> i32 {
    if slot_z_adjust != 0 {
        slot_z_adjust
    } else {
        type_z_adjust
    }
}

pub(crate) fn is_near_bridge_cell(state: &AppState, rx: u16, ry: u16) -> bool {
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let nx = rx as i32 + dx;
            let ny = ry as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            if state
                .bridge_height_map
                .contains_key(&(nx as u16, ny as u16))
            {
                return true;
            }
        }
    }
    false
}

pub(crate) fn is_under_bridge_render_state(state: &AppState, entity: &GameEntity) -> bool {
    entity.bridge_occupancy.is_none()
        && is_near_bridge_cell(state, entity.position.rx, entity.position.ry)
}

pub(crate) fn apply_bridge_depth_bias(state: &AppState, entity: &GameEntity, depth: f32) -> f32 {
    if !is_under_bridge_render_state(state, entity) {
        return depth;
    }
    (depth + entity.zfudge_bridge.max(0) as f32 * 0.00002).clamp(0.001, 0.999)
}

/// Where this entity is drawn, sub-cell offsets and height lift included.
///
/// Thin wrapper over [`crate::render::locomotor_visual::screen_position`],
/// which is the single owner of the answer. Callers must NOT apply a height
/// offset of their own on top — doing that in two places is what used to lift
/// aircraft twice.
pub(crate) fn interpolated_screen_position_entity(
    entity: &crate::sim::game_entity::GameEntity,
) -> (f32, f32) {
    crate::render::locomotor_visual::screen_position(entity)
}

/// Check whether an entity is visible to the local player based on shroud.
///
/// In standard YR (FogOfWar=false), once a cell is explored it stays fully
/// visible forever. Friendly entities are always visible. Enemy entities are
/// visible if the cell they occupy has been explored (revealed).
pub(crate) fn is_entity_visible_for_local_owner(
    local_owner: Option<&str>,
    fog: &FogState,
    pos: &Position,
    owner: &str,
    ignore_visibility: bool,
    local_owner_id: Option<InternedId>,
) -> bool {
    if ignore_visibility {
        return true;
    }
    let Some(local_owner) = local_owner else {
        return true;
    };
    if fog.is_friendly(local_owner, owner) {
        return true;
    }
    let owner_id = local_owner_id.unwrap_or_default();
    fog.is_cell_revealed(owner_id, pos.rx, pos.ry)
        && !fog.is_cell_gap_covered(owner_id, pos.rx, pos.ry)
}

pub(crate) fn cell_visibility_for_local_owner(
    local_owner_id: Option<InternedId>,
    fog: Option<&FogState>,
    rx: u16,
    ry: u16,
    ignore_visibility: bool,
) -> CellVisibilityState {
    if ignore_visibility {
        return CellVisibilityState::Visible;
    }
    let Some(local_owner_id) = local_owner_id else {
        return CellVisibilityState::Visible;
    };
    let Some(fog) = fog else {
        return CellVisibilityState::Visible;
    };
    // Standard YR (FogOfWar=false): explored = fully visible, no intermediate state.
    if fog.is_cell_revealed(local_owner_id, rx, ry) {
        CellVisibilityState::Visible
    } else {
        CellVisibilityState::Shrouded
    }
}

/// Viewport frustum cull check: is the entity's bounding box visible on screen?
pub(crate) fn in_view(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    cam_x: f32,
    cam_y: f32,
    sw: f32,
    sh: f32,
    m: f32,
) -> bool {
    x + w >= cam_x - m && x <= cam_x + sw + m && y + h >= cam_y - m && y <= cam_y + sh + m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::locomotor_visual::{ground_screen_position, screen_position};
    use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
    use crate::util::fixed_math::SimFixed;

    /// Stock YR sets one global `FlightLevel=1500` for every aircraft.
    const STOCK_FLIGHT_LEVEL_LEPTONS: i32 = 1500;

    fn entity_with_locomotor(
        type_ref: &str,
        kind: LocomotorKind,
        altitude_leptons: i32,
        rx: u16,
        ry: u16,
    ) -> GameEntity {
        let mut entity = GameEntity::test_default(1, type_ref, "Americans", rx, ry);
        let mut loco = LocomotorState::for_test_kind(kind);
        loco.altitude = SimFixed::from_num(altitude_leptons);
        entity.locomotor = Some(loco);
        entity
    }

    #[test]
    fn a_cruising_aircraft_leaves_the_ground_band() {
        // Black Eagle at the stock flight level.
        let beag = entity_with_locomotor(
            "BEAG",
            LocomotorKind::Fly,
            STOCK_FLIGHT_LEVEL_LEPTONS,
            40,
            40,
        );
        assert_eq!(entity_draw_band(&beag), EntityDrawBand::Top);
    }

    #[test]
    fn an_aircraft_on_its_pad_is_an_ordinary_ground_object() {
        // Fly answers the layer question from height alone and never looks at
        // the in-air flag, and `MovementLayer::Air` is fixed at construction
        // for the kind — so height is the only thing that may decide this.
        let parked = entity_with_locomotor("BEAG", LocomotorKind::Fly, 0, 40, 40);
        assert_eq!(
            parked.locomotor.as_ref().expect("locomotor").layer,
            MovementLayer::Air,
            "the nominal layer stays Air even parked; it must not be the predicate"
        );
        assert_eq!(entity_draw_band(&parked), EntityDrawBand::Ground);
    }

    #[test]
    fn a_hovering_jumpjet_leaves_the_ground_band_and_a_landed_one_does_not() {
        // ROCK, the Rocketeer — the one stock infantry type on a Jumpjet.
        let hovering = entity_with_locomotor("ROCK", LocomotorKind::Jumpjet, 500, 12, 12);
        let landed = entity_with_locomotor("ROCK", LocomotorKind::Jumpjet, 0, 12, 12);
        assert_eq!(entity_draw_band(&hovering), EntityDrawBand::Top);
        assert_eq!(entity_draw_band(&landed), EntityDrawBand::Ground);
    }

    #[test]
    fn ground_locomotors_never_leave_the_ground_band() {
        for kind in [
            LocomotorKind::Drive,
            LocomotorKind::Walk,
            LocomotorKind::Hover,
            LocomotorKind::Ship,
            LocomotorKind::Mech,
            LocomotorKind::Teleport,
        ] {
            let entity = entity_with_locomotor("MTNK", kind, 0, 5, 5);
            assert_eq!(
                entity_draw_band(&entity),
                EntityDrawBand::Ground,
                "{kind:?} answers Ground unconditionally in gamemd"
            );
        }
    }

    #[test]
    fn a_paradropping_infantryman_stays_in_the_ground_band() {
        // The locomotor underneath a parachute is still Walk, and Walk answers
        // Ground unconditionally — the altitude only moves where it is drawn.
        use crate::sim::entity_store::EntityStore;
        use crate::sim::movement::parachute_descent::begin_parachute_descent;

        let mut entities = EntityStore::default();
        let mut gi = GameEntity::test_default(1, "E1", "Americans", 20, 20);
        gi.category = crate::map::entities::EntityCategory::Infantry;
        gi.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Walk));
        entities.insert(gi);
        assert!(begin_parachute_descent(
            &mut entities,
            1,
            SimFixed::from_num(400)
        ));
        let gi = entities.get(1).expect("entity");

        assert_eq!(entity_draw_band(gi), EntityDrawBand::Ground);
        // ...but its key still has to come off the ground row, not the row it
        // is drawn at 57 px above.
        let (_, ground_y) = ground_screen_position(&gi.position);
        let (_, drawn_y) = screen_position(gi);
        assert_eq!(ground_y - drawn_y, 57.0);
        assert_eq!(ground_sort_row(gi, drawn_y), ground_y);
    }

    /// The Rocket slot has no height test — it is a bare `return 3`. Today the
    /// object-level `rocket_state` check answers first, so this arm is only
    /// reachable if a missile is ever built locomotor-first; gating it on
    /// altitude would then silently drop a launching missile into the ground
    /// band.
    #[test]
    fn the_rocket_locomotor_is_above_the_ground_band_at_any_height() {
        let launching = entity_with_locomotor("V3ROCKET", LocomotorKind::Rocket, 0, 8, 8);
        assert!(
            launching.rocket_state.is_none(),
            "this must exercise the locomotor arm, not the rocket_state shortcut"
        );
        assert_eq!(entity_draw_band(&launching), EntityDrawBand::Top);
    }

    /// A parachute canopy takes its body's key rather than deriving one, and
    /// this is the size of the mistake if that ever regresses: a paradrop is
    /// released at the aircraft's own flight level, so a canopy keyed off the
    /// row it is *drawn* at starts a full 216 px — about 14 iso rows — away
    /// from the man hanging on it, and closes to zero only at touchdown.
    #[test]
    fn a_canopy_keyed_off_the_drawn_row_would_start_a_whole_drop_height_from_its_body() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::movement::parachute_descent::begin_parachute_descent;

        let mut entities = EntityStore::default();
        let mut gi = GameEntity::test_default(1, "E1", "Americans", 20, 20);
        gi.category = crate::map::entities::EntityCategory::Infantry;
        gi.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Walk));
        entities.insert(gi);
        assert!(begin_parachute_descent(
            &mut entities,
            1,
            SimFixed::from_num(STOCK_FLIGHT_LEVEL_LEPTONS)
        ));
        let gi = entities.get(1).expect("entity");

        let (_, drawn_y) = screen_position(gi);
        assert_eq!(ground_sort_row(gi, drawn_y) - drawn_y, 216.0);
    }

    #[test]
    fn a_cruising_aircraft_sorts_on_its_own_cell_not_the_row_it_is_drawn_at() {
        let beag = entity_with_locomotor(
            "BEAG",
            LocomotorKind::Fly,
            STOCK_FLIGHT_LEVEL_LEPTONS,
            40,
            40,
        );
        let (_, ground_y) = ground_screen_position(&beag.position);
        let (_, drawn_y) = screen_position(&beag);

        // Cell (40, 40) at ground level projects to row 15*(40+40)+15.
        assert_eq!(ground_y, 1215.0);
        // The stock flight level lifts the drawing 216 px — ~14 iso rows.
        assert_eq!(ground_y - drawn_y, 216.0);
        assert_eq!(ground_sort_row(&beag, drawn_y), ground_y);
    }

    /// What the drawn row does to a key, measured on a concrete case.
    ///
    /// A Black Eagle cruising over cell (40, 40) and a war factory five cells
    /// north-west of it, on a 5000 px-tall map. Keyed off the row it is drawn
    /// at, the aircraft's key lands *behind* the building's; keyed off its own
    /// cell it lands in front, which is what `GetYSort` — X + Y, no Z term —
    /// gives you.
    ///
    /// A cruising aircraft no longer sorts against buildings at all, because
    /// [`EntityDrawBand::Top`] draws after the whole ground pass. The case is
    /// pinned here because the same key correction carries every body that
    /// stays in the ground band while off the floor — a paradrop above all —
    /// and there the ordering against buildings is live.
    #[test]
    fn the_drawn_row_pushes_a_lifted_bodys_key_behind_a_building_it_passes() {
        const MAP_ORIGIN_Y: f32 = 0.0;
        const MAP_WORLD_HEIGHT: f32 = 5000.0;

        let beag = entity_with_locomotor(
            "BEAG",
            LocomotorKind::Fly,
            STOCK_FLIGHT_LEVEL_LEPTONS,
            40,
            40,
        );
        let (_, drawn_y) = screen_position(&beag);

        // A building's key row is its NW cell origin: 15*(35+35)+15 - 30/2.
        let building_row: f32 = 1050.0;
        let building_depth =
            compute_sprite_depth_params(MAP_ORIGIN_Y, MAP_WORLD_HEIGHT, building_row, 0);

        let old_depth = compute_sprite_depth_params(MAP_ORIGIN_Y, MAP_WORLD_HEIGHT, drawn_y, 0);
        let new_depth = compute_sprite_depth_params(
            MAP_ORIGIN_Y,
            MAP_WORLD_HEIGHT,
            ground_sort_row(&beag, drawn_y),
            0,
        );

        // Larger depth = further back = drawn first.
        assert!(
            old_depth > building_depth,
            "the old key drew the aircraft first, so the building covered it"
        );
        assert!(
            new_depth < building_depth,
            "the ground row draws the building first and the aircraft over it"
        );
    }
}
