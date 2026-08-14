//! SHP entity instance builders — per-frame SpriteInstance generation for buildings and infantry.
//!
//! Handles building animation overlays (Active/Idle/Special), bibs, build-up
//! animations, and infantry sprite frame resolution.
//! Extracted from app_instances.rs to keep files under the 600-line limit.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use super::helpers::{
    ANIM_DRAW_DEPTH_BIAS_PX, EntityDrawBand, apply_bridge_depth_bias, apply_shape_z_adjust,
    compute_sprite_depth, effective_anim_z_adjust, entity_draw_band, ground_sort_row, in_view,
    is_under_bridge_render_state, tactical_entity_render_admission,
};
use crate::app::AppState;
use crate::app_render::draw_plan_lowering::{
    GroundPieceInstance, GroundTexture, NativeGroundOrder, PlannedBuildingPieceInstance,
    PlannedGroundObjectInstance,
};
use crate::map::entities::EntityCategory;
use crate::map::lighting;
use crate::render::batch::SpriteInstance;
use crate::render::draw_state::{DrawState, ObserverDrawContext};
use crate::render::sprite_atlas::ShpSpriteKey;
use crate::render::tactical_draw_plan::{
    BlitPolicy, BuildingPieceKind, SpriteEncoding, TacticalCoord,
};
use crate::render::unit_atlas::{UnitSpriteKey, VxlLayer, canonical_turret_facing};
use crate::rules::house_colors::HouseColorIndex;
use crate::sim::animation;
use crate::sim::components::BuildingUp;

/// Sort keys of the bodies currently hanging under a parachute, by entity id.
///
/// A parachute canopy is not an object in gamemd — `AnimClass::GetLayer` forces
/// an owner-attached anim into the owner's own layer, and the canopy is
/// composed into the descending body's draw. So the canopy has to sort at
/// exactly the body's key, and the only way to guarantee that is for the body
/// to hand its key over rather than for the canopy builder to re-derive one
/// from the same inputs and drift when either side changes.
///
/// Only ever read by key, never iterated, so the hash order is not observable.
pub(crate) type ParachuteBodyDepths = std::collections::HashMap<u64, f32>;

/// Iterate visible SHP sprite entities from EntityStore and build SpriteInstances.
///
/// Build SpriteInstances for all SHP entities (buildings, infantry).
/// Ground bodies, building bibs/anims, and building turret VXLs are emitted as
/// one parent-owned group so the native global order cannot split their display
/// call at an atlas boundary.
/// `top_instances` and aligned `top_pages` receive SHP bodies whose locomotor
/// puts them above the Ground
/// band — in stock YR that is the Rocketeer at hover height, the one infantry
/// type on a Jumpjet locomotor.
/// `parachute_body_depths` collects the sort key of every body currently under
/// a parachute, keyed by entity — see [`ParachuteBodyDepths`].
/// `selected_building_depth_paged` receives a second copy of every selected
/// building's body, for the depth-only stamp that lets the art clip its own
/// selection brackets. It is taken here rather than rebuilt later because this
/// is where the resolved atlas entry, buildup frame and sort depth already
/// exist together; re-deriving them elsewhere would be a second source of truth
/// that drifts the moment either side changes.
pub(crate) fn build_shp_instances(
    state: &AppState,
    paged: &mut [Vec<SpriteInstance>],
    bridge_paged: &mut [Vec<SpriteInstance>],
    top_instances: &mut Vec<SpriteInstance>,
    top_pages: &mut Vec<usize>,
    top_ids: &mut Vec<u64>,
    parachute_body_depths: &mut ParachuteBodyDepths,
    selected_building_depth_paged: &mut [Vec<SpriteInstance>],
    ground_objects: &mut Vec<PlannedGroundObjectInstance>,
    ground_order: &NativeGroundOrder,
) {
    let (sim, atlas) = match (&state.simulation, &state.sprite_atlas) {
        (Some(s), Some(a)) => (s, a),
        _ => return,
    };
    let z = state.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.camera_x,
        state.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );
    let local_owner = crate::app_commands::preferred_local_owner_name(state);
    let local_owner_id = local_owner.as_deref().and_then(|o| sim.interner.get(o));
    let ignore_visibility = state.sandbox_full_visibility;
    let art_reg: Option<&crate::rules::art_data::ArtRegistry> = state.art_registry.as_ref();

    let encounter_order =
        super::helpers::tactical_entity_encounter_order(sim, state.rules.as_ref());
    for stable_id in encounter_order {
        let Some(entity) = sim.entities().get(stable_id) else {
            continue;
        };
        if entity.is_voxel {
            continue;
        }
        // Common visibility, passenger, limbo, and DrawState admission is shared below.
        let owner_str = sim.interner.resolve(entity.owner);
        let active_disguise = entity.disguise.as_ref().filter(|state| state.disguised);
        let type_str = active_disguise
            .and_then(|state| state.disguise_type)
            .map(|id| sim.interner.resolve(id))
            .unwrap_or_else(|| sim.interner.resolve(entity.type_ref));
        let remap_owner = active_disguise
            .and_then(|state| state.disguised_as_house)
            .map(|id| sim.interner.resolve(id))
            .unwrap_or(owner_str);
        // Wall buildings render as overlays (auto-tiled connectivity frames).
        // Their Y-sorted rendering in the object pass is handled by including
        // wall overlay instances in the unified merge (draw_merged_object_pass),
        // not here. Skip them to avoid drawing frame 0 (isolated pillar).
        if entity.category == EntityCategory::Structure {
            let is_wall = state
                .rules
                .as_ref()
                .and_then(|r| r.object(type_str))
                .map(|o| o.wall)
                .unwrap_or(false);
            if is_wall {
                continue;
            }
        }
        let pos = &entity.position;
        let hc: HouseColorIndex = state
            .house_color_map
            .get(remap_owner)
            .copied()
            .unwrap_or(crate::rules::house_colors::NO_REMAP);
        let Some(draw_decision) = tactical_entity_render_admission(
            entity,
            owner_str,
            local_owner.as_deref(),
            local_owner_id,
            &sim.fog,
            ignore_visibility,
            sim.session.binary_frame,
            super::units::house_color_to_remap_row(hc),
            ObserverDrawContext {
                owner_is_allied: local_owner.as_deref().is_some_and(|observer| {
                    crate::map::houses::is_allied_with(&sim.house_alliances, observer, owner_str)
                }),
                detects_cloak: local_owner_id
                    .is_some_and(|observer| sim.fog.has_sensor_for_house(observer, pos.rx, pos.ry)),
            },
        ) else {
            continue;
        };
        // Buildings are the one class gamemd draws off its own render-coordinate
        // virtual rather than the plain object coordinate; everything below this
        // point — body, bib, anims, turret, and the row they sort on — wants that
        // lifted anchor. See `locomotor_visual::BUILDING_ART_LIFT_PX`.
        let (sx, sy) = {
            let anchor = crate::render::locomotor_visual::screen_position(entity);
            if entity.category == EntityCategory::Structure {
                crate::render::locomotor_visual::building_art_anchor(anchor.0, anchor.1)
            } else {
                anchor
            }
        };
        let interp_z = pos.z;
        if !in_view(sx, sy, 200.0, 200.0, cam_x, cam_y, sw, sh, 200.0) {
            continue;
        }
        let draw_state = draw_decision.state;
        // Determine if this building is in its make/build-up or build-down animation.
        let is_building_up: bool =
            entity.category == EntityCategory::Structure && entity.building_up.is_some();
        let is_building_down: bool =
            entity.category == EntityCategory::Structure && entity.building_down.is_some();
        let (shp_frame, make_type_id): (u16, Option<String>) = if is_building_up {
            let bu: &BuildingUp = entity.building_up.as_ref().expect("checked above");
            let make_key: String = format!("{}_MAKE", type_str);
            let total_make_frames: u16 =
                atlas.make_frame_counts.get(&make_key).copied().unwrap_or(0);
            if total_make_frames > 0 {
                // Map elapsed ticks to make frame index (forward: 0 → last).
                let progress: f32 = bu.elapsed_ticks as f32 / bu.total_ticks.max(1) as f32;
                let frame: u16 =
                    ((progress * total_make_frames as f32) as u16).min(total_make_frames - 1);
                (frame, Some(make_key))
            } else {
                (0, None)
            }
        } else if is_building_down {
            let bd = entity.building_down.as_ref().expect("checked above");
            let make_key: String = format!("{}_MAKE", type_str);
            let total_make_frames: u16 =
                atlas.make_frame_counts.get(&make_key).copied().unwrap_or(0);
            if total_make_frames > 0 {
                // Map elapsed ticks to make frame index in reverse (last → 0).
                let progress: f32 = bd.elapsed_ticks as f32 / bd.total_ticks.max(1) as f32;
                let reverse_frame: u16 = total_make_frames.saturating_sub(1).saturating_sub(
                    ((progress * total_make_frames as f32) as u16).min(total_make_frames - 1),
                );
                (reverse_frame, Some(make_key))
            } else {
                (0, None)
            }
        } else {
            match entity.category {
                EntityCategory::Structure => {
                    let obj = state.rules.as_ref().and_then(|r| r.object(type_str));
                    let frame = if obj.map(|o| o.can_be_occupied).unwrap_or(false) {
                        let occupant_count = entity
                            .passenger_role
                            .cargo()
                            .map(|c| c.count())
                            .unwrap_or(0);
                        let tech_level = obj.map(|o| o.tech_level).unwrap_or(-1);
                        let (cy, cr) = state
                            .rules
                            .as_ref()
                            .map(|r| (r.general.condition_yellow, r.general.condition_red))
                            .unwrap_or((0.5, 0.25));
                        rendered_garrison_body_frame_index(
                            0,
                            entity.building_damage_state_active,
                            occupant_count,
                            entity.health.current,
                            entity.health.max,
                            tech_level,
                            cy,
                            cr,
                        )
                    } else {
                        0
                    };
                    (frame, None)
                }
                _ => (
                    resolve_infantry_shp_frame(
                        state,
                        type_str,
                        entity.facing,
                        entity.animation.as_ref(),
                    ),
                    None,
                ),
            }
        };
        let key: ShpSpriteKey = ShpSpriteKey {
            type_id: make_type_id.as_deref().unwrap_or(type_str).to_string(),
            facing: 0,
            frame: shp_frame,
            house_color: hc,
        };
        // Fallback: a CanBeOccupied building requesting frame 1/2/3 may miss the
        // atlas if its SHP has fewer than 4 frames. Retry with frame 0 so the
        // building still draws (Approach A in the design doc). Non-garrisonable
        // misses keep their existing skip path.
        let entry = match atlas.get(&key) {
            Some(e) => e,
            None if shp_frame != 0
                && entity.category == EntityCategory::Structure
                && state
                    .rules
                    .as_ref()
                    .and_then(|r| r.object(type_str))
                    .map(|o| o.can_be_occupied)
                    .unwrap_or(false) =>
            {
                let fallback_key = ShpSpriteKey {
                    type_id: make_type_id.as_deref().unwrap_or(type_str).to_string(),
                    facing: 0,
                    frame: 0,
                    house_color: hc,
                };
                match atlas.get(&fallback_key) {
                    Some(e) => e,
                    None => continue,
                }
            }
            None => continue,
        };

        let final_x: f32 = sx + entry.offset_x;
        let final_y: f32 = sy + entry.offset_y;
        let band = entity_draw_band(entity);
        let base_depth: f32 = match entity.category {
            EntityCategory::Structure => {
                // `sy` already carries the render-coordinate lift, so it *is* the
                // NW footprint cell's tile row — the row gamemd's YSort (X + Y
                // off the render coords) reduces to. A building therefore sorts
                // on its own cell rather than one iso row north of it.
                compute_sprite_depth(state, sy, interp_z)
            }
            _ => {
                // The drawn row carries this body's height lift; the sort key
                // must not. A hovering Rocketeer or a descending paradrop key
                // off the cell it is over, exactly like a GI standing there.
                let depth_y: f32 = sy + entry.offset_y + entry.pixel_size[1];
                compute_sprite_depth(state, ground_sort_row(entity, depth_y), interp_z)
            }
        };
        let depth: f32 = match band {
            EntityDrawBand::Top => base_depth,
            EntityDrawBand::Ground => apply_bridge_depth_bias(state, entity, base_depth),
        };
        if entity.parachute_state.is_some() {
            parachute_body_depths.insert(entity.stable_id, depth);
        }
        let mut tint: [f32; 3] = match entity.category {
            EntityCategory::Infantry => state.lighting_grid.infantry_tint_at((pos.rx, pos.ry)),
            EntityCategory::Structure => {
                state.lighting_grid.building_body_tint_at((pos.rx, pos.ry))
            }
            _ => state.lighting_grid.techno_tint_at((pos.rx, pos.ry)),
        };
        // Entity ambient glow so infantry are visible on dark maps.
        // Buildings do NOT get entity glow; only non-building technos use the extra-light rules.
        if entity.category == EntityCategory::Infantry {
            if let Some(rules) = &state.rules {
                let glow = rules.general.extra_infantry_light;
                if glow > 0.0 {
                    tint[0] = (tint[0] + glow).min(lighting::TOTAL_AMBIENT_CAP);
                    tint[1] = (tint[1] + glow).min(lighting::TOTAL_AMBIENT_CAP);
                    tint[2] = (tint[2] + glow).min(lighting::TOTAL_AMBIENT_CAP);
                }
            }
        }
        let under_bridge = is_under_bridge_render_state(state, entity)
            && entity.category != EntityCategory::Structure;
        let collect_ground = band == EntityDrawBand::Ground && !under_bridge;
        let target_pages = match band {
            // Top stays flat; page identity is carried beside each instance.
            EntityDrawBand::Top => None,
            EntityDrawBand::Ground if under_bridge => Some(&mut *bridge_paged),
            EntityDrawBand::Ground => Some(&mut *paged),
        };
        let body = SpriteInstance {
            position: [final_x, final_y],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth,
            tint,
            alpha: 1.0,
            draw_state,
            ..Default::default()
        };

        let mut building_pieces = Vec::new();
        if entity.category == EntityCategory::Structure {
            // Only the selected building's own art participates in clipping its
            // brackets, so the stamp bucket stays empty in ordinary play and
            // costs one extra quad per selected structure otherwise.
            if entity.selected {
                if let Some(bucket) = selected_building_depth_paged.get_mut(entry.page as usize) {
                    // Deliberately NOT the body's sort depth. gamemd anchors a
                    // shape's Z on the bottom edge of its blit rect, and the
                    // per-pixel ramp it lays over the sprite cancels the
                    // walker's own per-row step, so every pixel of a building
                    // ends up carrying that one bottom-row value. The sort key
                    // is a different quantity — the north-west footprint cell's
                    // tile row — and using it here would put the stamp north of
                    // every bracket corner, so nothing would ever clip.
                    bucket.push(SpriteInstance {
                        depth: compute_sprite_depth(state, final_y + entry.pixel_size[1], interp_z),
                        ..body
                    });
                }
            }
            building_pieces.push(PlannedBuildingPieceInstance {
                kind: if is_building_up || is_building_down {
                    BuildingPieceKind::BuildupOrSpecial
                } else {
                    BuildingPieceKind::Body
                },
                z_bias: 0,
                policy: BlitPolicy::opaque(SpriteEncoding::Plain),
                target: GroundTexture::ShpPage(entry.page as usize),
                instance: body,
            });
        } else if collect_ground {
            let coord = TacticalCoord {
                x: i32::from(pos.rx) * 256 + crate::util::fixed_math::sim_to_i32(pos.sub_x),
                y: i32::from(pos.ry) * 256 + crate::util::fixed_math::sim_to_i32(pos.sub_y),
                z: i32::from(pos.z),
            };
            if let Some(parent) =
                ground_order.object_draw(entity.stable_id, coord, SpriteEncoding::Plain)
            {
                ground_objects.push(PlannedGroundObjectInstance::object(
                    parent,
                    vec![GroundPieceInstance {
                        target: GroundTexture::ShpPage(entry.page as usize),
                        instance: body,
                    }],
                ));
            }
        } else if band == EntityDrawBand::Top {
            top_instances.push(body);
            top_pages.push(entry.page as usize);
            top_ids.push(entity.stable_id);
        } else {
            target_pages.expect("Ground SHP target was selected")[entry.page as usize].push(body);
        }

        // Emit building animation overlays and bib — but NOT during build-up/down.
        // Bibs and anims use the raw cell position (sy) — their own SHP offsets
        // (baked into the canvas) handle correct placement relative to the cell.
        if entity.category == EntityCategory::Structure && !is_building_up && !is_building_down {
            if let Some(art) = art_reg {
                // Bib is drawn INSIDE BuildingClass_DrawBody in the original engine,
                // right after the main body sprite, as part of the same object pass.
                // It overwrites the body's terrain-colored pixels at the ramp area.
                emit_building_bib(
                    &mut building_pieces,
                    atlas,
                    art,
                    state.rules.as_ref(),
                    type_str,
                    hc,
                    sx,
                    sy,
                    interp_z,
                    depth,
                    tint,
                    draw_state,
                );
                // Building anims render in the same pass as building bodies so they
                // can sort together via depth. Anims use the building's entity depth
                // so they render at the same depth as the body — visible where the
                // body has transparent pixels, covered where it's opaque.
                let is_garrisoned = entity.passenger_role.cargo().is_some_and(|c| !c.is_empty());
                let is_player_owned = !crate::rules::house_colors::is_non_player_house(owner_str);
                let world_height: f32 = state
                    .terrain_grid
                    .as_ref()
                    .map(|g| g.world_height)
                    .unwrap_or(1.0);
                emit_building_anims(
                    &mut building_pieces,
                    atlas,
                    art,
                    state.rules.as_ref(),
                    type_str,
                    hc,
                    sx,
                    sy,
                    depth,
                    tint,
                    entity.building_anim_overlays.as_ref(),
                    crate::app_building_anim::building_anim_elapsed_logic_frames(
                        state,
                        entity.stable_id,
                    ),
                    Some(&sim.session.game_options),
                    Some(&sim.interner),
                    is_garrisoned,
                    is_player_owned,
                    entity.building_damage_state_active,
                    world_height,
                    draw_state,
                );
            }
            // Emit VXL turret on top of building (e.g., SAM site, Prism Tower).
            if let Some(rules_obj) = state.rules.as_ref().and_then(|r| r.object(type_str)) {
                if rules_obj.turret_anim_is_voxel {
                    if let Some(turret_id) = &rules_obj.turret_anim {
                        if let Some((page, instance)) = emit_building_turret_vxl(
                            state,
                            turret_id,
                            entity
                                .barrel_facing
                                .as_ref()
                                .map(|f| f.current(sim.session.binary_frame))
                                .unwrap_or(0u16),
                            hc,
                            sx,
                            sy,
                            interp_z,
                            depth,
                            tint,
                            draw_state,
                            rules_obj.turret_anim_x,
                            rules_obj.turret_anim_y,
                        ) {
                            building_pieces.push(PlannedBuildingPieceInstance {
                                kind: BuildingPieceKind::PoweredOrActiveOverlay,
                                z_bias: 0,
                                policy: BlitPolicy::opaque(SpriteEncoding::Voxel),
                                target: GroundTexture::UnitAtlasPage(page),
                                instance,
                            });
                        }
                    }
                }
            }
        }

        if entity.category == EntityCategory::Structure {
            let location = TacticalCoord {
                x: i32::from(pos.rx) * 256 + crate::util::fixed_math::sim_to_i32(pos.sub_x),
                y: i32::from(pos.ry) * 256 + crate::util::fixed_math::sim_to_i32(pos.sub_y),
                z: i32::from(pos.z),
            };
            let actual_type = state
                .rules
                .as_ref()
                .and_then(|rules| rules.object(sim.interner.resolve(entity.type_ref)));
            if let Some(parent) = actual_type.and_then(|object_type| {
                ground_order.building_object_draw(
                    entity.stable_id,
                    location,
                    object_type,
                    SpriteEncoding::Plain,
                )
            }) {
                ground_objects.push(PlannedGroundObjectInstance::building(
                    parent,
                    building_pieces,
                ));
            }
        }
    }
}

/// Emit a VXL turret sprite on top of a building (e.g., SAM site turret, Prism Tower).
///
/// Looks up the pre-rendered turret VXL from the UnitAtlas at the current turret facing,
/// positioned at the building's screen origin + pixel offset from TurretAnimX/Y.
///
/// The turret carries the building's own sort depth verbatim. `TurretAnimZAdjust=`
/// is deliberately not folded in: gamemd's ground-layer sort key for a building
/// reads only `TurretAnimIsVoxel` (+32 leptons) and `Gate` (−16 leptons), while
/// `TurretAnimZAdjust` is read by a different virtual that composes the
/// building's *draw* Z. Using it as a sort bias here would pull the turret
/// toward the camera and put it over units standing in front of the building —
/// by 1.3 to 4 iso rows on the defences a player actually fights around
/// (SAM Site and Sentry Gun −20, Flak and Gattling Cannon −40, Slave Miner −50,
/// Grand Cannon −60), more on a couple of civilian map props. The set is small
/// and does not include Prism Tower, whose turret is not a voxel.
fn emit_building_turret_vxl(
    state: &AppState,
    turret_id: &str,
    turret_facing: u16,
    _hc: HouseColorIndex,
    building_sx: f32,
    building_sy: f32,
    _z: u8,
    building_depth: f32,
    tint: [f32; 3],
    draw_state: DrawState,
    anim_x: i32,
    anim_y: i32,
) -> Option<(usize, SpriteInstance)> {
    let unit_atlas = match &state.unit_atlas {
        Some(a) => a,
        None => return None,
    };
    let key = UnitSpriteKey {
        type_id: turret_id.to_string(),
        facing: canonical_turret_facing(turret_facing),
        layer: VxlLayer::Composite,
        frame: 0,
        slope_type: 0, // building turrets don't tilt on slopes
    };
    let entry = unit_atlas.get(&key)?;
    // Position turret at building cell origin + pixel offset from INI.
    // TurretAnimX/Y are screen pixel offsets added to the building's own draw
    // point, which is exactly what the native turret draw does with them.
    let center_x: f32 = building_sx;
    let tx: f32 = center_x + anim_x as f32 + entry.offset_x;
    let ty: f32 = building_sy + anim_y as f32 + entry.offset_y + 3.0;
    Some((
        entry.page,
        SpriteInstance {
            position: [tx, ty],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth: building_depth,
            tint,
            alpha: 1.0,
            draw_state,
            ..Default::default()
        },
    ))
}

/// Emit the BibShape SpriteInstance for a building's ground-level pad.
///
/// BibShape is a separate SHP (e.g., GAREFNBB for the Allied Refinery dock) drawn
/// behind the building at the same cell position. It provides the flat ground
/// surface where harvesters dock or other ground-level detail.
fn emit_building_bib(
    pieces: &mut Vec<PlannedBuildingPieceInstance>,
    atlas: &crate::render::sprite_atlas::SpriteAtlas,
    art_reg: &crate::rules::art_data::ArtRegistry,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    building_type: &str,
    house_color: HouseColorIndex,
    screen_x: f32,
    screen_y: f32,
    _z: u8,
    building_depth: f32,
    tint: [f32; 3],
    draw_state: DrawState,
) {
    let rules_image: String = rules
        .and_then(|r| r.object(building_type))
        .map(|o| o.image.clone())
        .unwrap_or_else(|| building_type.to_string());
    let art_entry = match art_reg.resolve_metadata_entry(building_type, &rules_image) {
        Some(e) => e,
        None => return,
    };
    let bib_name: &str = match art_entry.bib_shape.as_deref() {
        Some(name) => name,
        None => return,
    };
    let bib_key: ShpSpriteKey = ShpSpriteKey {
        type_id: bib_name.to_uppercase(),
        facing: 0,
        frame: 0,
        house_color,
    };
    let Some(bib_entry) = atlas.get(&bib_key) else {
        return;
    };
    let bx: f32 = screen_x + bib_entry.offset_x;
    let by: f32 = screen_y + bib_entry.offset_y;
    // The bib is drawn inside the building's draw body pass — it doesn't sort
    // independently. The entire building (body + bib) sorts as one unit at the
    // building's YSort position. Use the building's depth so bib and body stay
    // together in the Y-sorted merge, preventing bibs from incorrectly
    // overlapping walls at closer iso rows.
    pieces.push(PlannedBuildingPieceInstance {
        kind: BuildingPieceKind::Bib,
        z_bias: 0,
        policy: BlitPolicy::opaque(SpriteEncoding::Plain),
        target: GroundTexture::ShpPage(bib_entry.page as usize),
        instance: SpriteInstance {
            position: [bx, by],
            size: bib_entry.pixel_size,
            uv_origin: bib_entry.uv_origin,
            uv_size: bib_entry.uv_size,
            depth: building_depth,
            tint,
            alpha: 1.0,
            draw_state,
            ..Default::default()
        },
    });
}

/// Frame of a looping building animation, `elapsed_logic_frames` after the
/// animation object was created.
///
/// gamemd advances the animation's own frame counter by one every `rate` logic
/// frames and, on reaching `LoopEnd`, resets it to `LoopStart`; the counter and
/// its timer both start at construction. The phase is therefore a pure function
/// of how long this animation has existed — which is why identical buildings
/// raised at different times do not animate in lockstep.
///
/// DRIFT — the first sweep is missing when `Start=` differs from `LoopStart=`.
/// The native counter is relative to `Start` and begins at zero, so the drawn
/// frame is `Start + counter`: the animation plays `Start..LoopEnd` once when it
/// is created and only then settles into `LoopStart..LoopEnd-1`. This goes
/// straight into the loop. Trigger: creation of the slot animation — building
/// placement for `[CAWA19_A]`, `[GACTWR_A]`, `[NATBNK_A]`, `[NATBNK_B]` and
/// `[YAROCK_A]`, and crossing `ConditionYellow` for the 21 `…_AD` damaged
/// replacements that also qualify. Effect: a one-off sweep through the other
/// half of the SHP is skipped. Frequency: once per animation creation, never
/// repeating, so it costs a brief transient and nothing steady-state.
fn looping_frame_values(
    loop_start: u16,
    loop_end: u16,
    start_frame: u16,
    rate_logic_frames: u16,
    ping_pong: bool,
    elapsed_logic_frames: u32,
) -> u16 {
    // LoopEnd is EXCLUSIVE in RA2 art.ini — e.g. GAPOWR_A has LoopStart=0,
    // LoopEnd=8 meaning frames 0..8 (0-7), while GAPOWR_AD starts at frame 8.
    // The ranges are contiguous: normal=[0..8), damaged=[8..16).
    let range: u16 = loop_end.saturating_sub(loop_start).max(1);
    let rate: u32 = u32::from(rate_logic_frames).max(1);
    let tick: u32 = elapsed_logic_frames / rate;

    if ping_pong {
        return ping_pong_frame_value(loop_end, start_frame, tick);
    }
    loop_start + (tick % range as u32) as u16
}

/// Frame of a `PingPong=yes` building animation `tick` frame-advances after
/// construction.
///
/// The native bounce is **not** symmetric about the loop range, and it does not
/// read `LoopStart` at all. The frame counter is relative to `Start=`, and the
/// direction flips when that counter reaches `LoopEnd - Start` or equals
/// `Start`. The flip returns immediately without touching the counter, so each
/// endpoint frame is displayed for one full frame delay rather than being
/// stepped over — `GARADR_A` (`Start=0`, `LoopEnd=14`) is a 28-step bounce
/// across frames 0..=14, not a 26-step one across 0..=13.
fn ping_pong_frame_value(loop_end: u16, start_frame: u16, tick: u32) -> u16 {
    let high: u32 = u32::from(loop_end.saturating_sub(start_frame));
    let low: u32 = u32::from(start_frame);
    if high == 0 {
        return start_frame;
    }
    // The counter climbs from zero on construction and turns at `high`.
    if tick <= high {
        return start_frame + tick as u16;
    }
    if low >= high {
        // Both turning points land on the same counter value (or invert), so
        // gamemd's own behaviour here is degenerate: it flips once at the top,
        // then the descending counter can never satisfy the `== Start` test
        // again and runs away downwards for the rest of the animation's life,
        // walking off the start of the SHP. `[GAPLUG_BD]` (Start=10,
        // LoopStart=10, LoopEnd=20) is the only stock section that hits it.
        //
        // VERA-INTERNAL, and a DELIBERATE DIVERGENCE from gamemd rather than an
        // approximation of it: hold the last frame gamemd draws before the
        // runaway. Reproducing the runaway faithfully would mean drawing
        // negative frame indices, i.e. garbage or nothing.
        return start_frame + high as u16;
    }
    let span: u32 = high - low;
    let phase: u32 = (tick - high) % (2 * span);
    let counter: u32 = if phase <= span {
        high - phase
    } else {
        low + (phase - span)
    };
    start_frame + counter as u16
}

/// Whether an `InfantryAbsorb` building's ActiveAnim slot is the one gamemd
/// clears for the current occupancy.
///
/// The native branch only ever touches the first two ActiveAnim slots: with no
/// occupants it clears the second and creates the first, and with one or more it
/// clears the first and creates the second. Any further ActiveAnim slot is
/// outside the branch and keeps rendering.
fn infantry_absorb_slot_is_hidden(active_slot_ordinal: usize, is_garrisoned: bool) -> bool {
    match active_slot_ordinal {
        0 => is_garrisoned,
        1 => !is_garrisoned,
        _ => false,
    }
}

struct BuildingAnimFrameView<'a> {
    anim_type: &'a str,
    loop_start: u16,
    loop_end: u16,
    loop_count: i32,
    start_frame: u16,
    ping_pong: bool,
}

fn selected_building_anim_view<'a>(
    anim: &'a crate::rules::art_data::BuildingAnimConfig,
    building_damage_state_active: bool,
    is_garrisoned: bool,
) -> BuildingAnimFrameView<'a> {
    let variant = if building_damage_state_active {
        anim.damaged_variant.as_ref()
    } else if is_garrisoned {
        anim.garrisoned_variant.as_ref()
    } else {
        None
    };
    match variant {
        Some(v) => BuildingAnimFrameView {
            anim_type: &v.anim_type,
            loop_start: v.loop_start,
            loop_end: v.loop_end,
            loop_count: v.loop_count,
            start_frame: v.start_frame,
            ping_pong: v.ping_pong,
        },
        None => BuildingAnimFrameView {
            anim_type: &anim.anim_type,
            loop_start: anim.loop_start,
            loop_end: anim.loop_end,
            loop_count: anim.loop_count,
            start_frame: anim.start_frame,
            ping_pong: anim.ping_pong,
        },
    }
}

/// Emit SpriteInstances for a building's animation overlays.
///
/// Each anim overlay (e.g., CAOILD_A for Oil Derrick's tower) is looked up
/// in the sprite atlas and positioned at the building's cell center + the
/// animation's (X, Y) pixel offset from art.ini.
fn emit_building_anims(
    pieces: &mut Vec<PlannedBuildingPieceInstance>,
    atlas: &crate::render::sprite_atlas::SpriteAtlas,
    art_reg: &crate::rules::art_data::ArtRegistry,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    building_type: &str,
    house_color: HouseColorIndex,
    screen_x: f32,
    screen_y: f32,
    building_depth: f32,
    tint: [f32; 3],
    overlays: Option<&crate::sim::components::BuildingAnimOverlays>,
    anim_elapsed_logic_frames: u32,
    game_options: Option<&crate::sim::game_options::GameOptions>,
    interner: Option<&crate::sim::intern::StringInterner>,
    is_garrisoned: bool,
    is_player_owned: bool,
    building_damage_state_active: bool,
    world_height: f32,
    draw_state: DrawState,
) {
    let rules_image: String = rules
        .and_then(|r| r.object(building_type))
        .map(|o| o.image.clone())
        .unwrap_or_else(|| building_type.to_string());
    let art_entry = match art_reg.resolve_metadata_entry(building_type, &rules_image) {
        Some(e) => e,
        None => return,
    };
    // Ordinal of this entry within the `ActiveAnim` family, i.e. its offset from
    // the first of gamemd's four contiguous ActiveAnim slots. Parse order is key
    // order (`ActiveAnim`, `…Two`, `…Three`, `…Four`), so counting Active-kind
    // entries reproduces the slot index the native branches switch on.
    let mut active_slot_ordinal: usize = 0;
    for anim in &art_entry.building_anims {
        let this_active_ordinal: usize = active_slot_ordinal;
        if matches!(anim.kind, crate::rules::art_data::BuildingAnimKind::Active) {
            active_slot_ordinal += 1;
        }
        // Determine current frame based on animation type and art.ini properties.
        //
        // One-shot anims (Active/Production with LoopCount>0): driven by ECS overlays.
        // Infinite-loop anims (LoopCount=-1 or IdleAnim): per-building loop phase.
        // Special/Super: event-triggered one-shot — skip entirely if not in overlays.
        let selected =
            selected_building_anim_view(anim, building_damage_state_active, is_garrisoned);
        let anim_upper: String = anim.anim_type.to_uppercase();
        let anim_upper_id: Option<crate::sim::intern::InternedId> =
            interner.and_then(|i| i.get(&anim_upper));
        let frame: u16 = if matches!(
            anim.kind,
            crate::rules::art_data::BuildingAnimKind::Active
                | crate::rules::art_data::BuildingAnimKind::Production
        ) {
            if selected.loop_count < 0 {
                // Refinery ore-pile tier display: ActiveAnim/Two/Three/Four map
                // to slots 3..6 in gamemd, and exactly ONE renders at a time —
                // picked by `floor(stored * 4 / Storage)` (tier 0..3+). The
                // Allied/Soviet dump path bypasses the refinery's StorageClass
                // entirely (credits go straight to the owner), so the building's
                // own stored amount stays 0 and tier is always 0 → only the
                // primary slot (ActiveAnim = GAREFNL1) renders. The non-primary
                // slots (Two/Three/Four) must be suppressed; otherwise all four
                // ore-pile sprites stack on top of each other every frame.
                let obj = rules.and_then(|r| r.object(building_type));
                if obj.map(|o| o.refinery).unwrap_or(false) && !anim.is_primary {
                    continue;
                }
                // Infantry-absorb power plant (Yuri's Bio Reactor): gamemd shows
                // exactly ONE of the first two ActiveAnim slots and swaps them on
                // occupant count — empty picks `ActiveAnim`, one or more occupants
                // picks `ActiveAnimTwo`. Whichever is not selected is cleared, so
                // the two layers are never on screen together.
                if matches!(anim.kind, crate::rules::art_data::BuildingAnimKind::Active)
                    && obj.is_some_and(|o| o.infantry_absorb && o.extra_power > 0)
                    && infantry_absorb_slot_is_hidden(this_active_ordinal, is_garrisoned)
                {
                    continue;
                }
                // Infinite loop ActiveAnim on a capturable tech building
                // (Oil Derrick, Airport, etc.): the primary slot (ActiveAnim)
                // only plays after capture. Decorative civilian buildings
                // (country flags, etc.) always animate.
                let is_capturable: bool = obj.map(|o| o.capturable).unwrap_or(false);
                if anim.is_primary && is_capturable && !is_player_owned {
                    selected.start_frame
                } else {
                    looping_frame_values(
                        selected.loop_start,
                        selected.loop_end,
                        selected.start_frame,
                        crate::app_building_anim::building_anim_rate_logic_frames(
                            art_reg,
                            selected.anim_type,
                            game_options,
                        ),
                        selected.ping_pong,
                        anim_elapsed_logic_frames,
                    )
                }
            } else {
                // One-shot: look up current frame from ECS BuildingAnimOverlays component.
                overlays
                    .and_then(|o| o.anims.iter().find(|a| anim_upper_id == Some(a.anim_type)))
                    .map(|a| a.frame)
                    .unwrap_or_else(|| {
                        resting_building_anim_frame_values(
                            selected.loop_start,
                            selected.loop_end,
                            selected.start_frame,
                        )
                    })
            }
        } else if matches!(anim.kind, crate::rules::art_data::BuildingAnimKind::Idle) {
            looping_frame_values(
                selected.loop_start,
                selected.loop_end,
                selected.start_frame,
                crate::app_building_anim::building_anim_rate_logic_frames(
                    art_reg,
                    selected.anim_type,
                    game_options,
                ),
                selected.ping_pong,
                anim_elapsed_logic_frames,
            )
        } else {
            // Special/Super are one-shot event-triggered animations (e.g., GAREFNOR ore
            // conveyor). Only render if actively playing in the BuildingAnimOverlays state.
            // When not triggered, skip this anim entirely — don't show frame 0.
            match overlays.and_then(|o| o.anims.iter().find(|a| anim_upper_id == Some(a.anim_type)))
            {
                Some(s) if !s.finished => s.frame,
                _ => continue,
            }
        };
        // If the computed frame isn't in the atlas, fall back to the last
        // available frame rather than skipping the overlay entirely.
        // This prevents a visual glitch where the anim disappears for one
        // tick when the atlas has fewer frames than the art.ini loop range.
        let mut anim_key: ShpSpriteKey = ShpSpriteKey {
            type_id: selected.anim_type.to_string(),
            facing: 0,
            frame,
            house_color,
        };
        let mut anim_entry_opt = atlas.get(&anim_key);
        if anim_entry_opt.is_none() && frame > 0 {
            // Try the previous frame as fallback.
            anim_key.frame = frame - 1;
            anim_entry_opt = atlas.get(&anim_key);
        }
        let Some(anim_entry) = anim_entry_opt else {
            continue;
        };

        // Position: cell center + anim X/Y offset from art.ini.
        // Building anims use building positioning (building convention).
        // The anim's own draw offset (XDrawOffset/YDrawOffset) is already baked
        // into anim_entry.offset_x/y by the sprite atlas builder.
        let ax: f32 = screen_x + anim.x as f32 + anim_entry.offset_x;
        let ay: f32 = screen_y + anim.y as f32 + anim_entry.offset_y;

        // Building anims start from the building body's depth (emitted in the
        // same pass; on-top-of-own-body comes from instance order), then apply
        // the native ZAdjust sort bias: the per-slot override from the
        // building's art section (e.g. ActiveAnimZAdjust=) wins when nonzero,
        // else the anim type's own ZAdjust= applies; anim SHP draws also carry
        // a constant -2px bias. Negative = toward camera. This orders anims
        // correctly against OTHER nearby objects.
        let type_z_adjust: i32 = art_reg
            .anim_runtime_config(&selected.anim_type)
            .map(|c| c.z_adjust)
            .unwrap_or(0);
        let z_adjust_px: i32 =
            effective_anim_z_adjust(anim.z_adjust, type_z_adjust) + ANIM_DRAW_DEPTH_BIAS_PX;
        let anim_depth: f32 = apply_shape_z_adjust(building_depth, z_adjust_px, world_height);

        pieces.push(PlannedBuildingPieceInstance {
            kind: BuildingPieceKind::PoweredOrActiveOverlay,
            z_bias: z_adjust_px,
            policy: BlitPolicy::opaque(SpriteEncoding::Plain),
            target: GroundTexture::ShpPage(anim_entry.page as usize),
            instance: SpriteInstance {
                position: [ax, ay],
                size: anim_entry.pixel_size,
                uv_origin: anim_entry.uv_origin,
                uv_size: anim_entry.uv_size,
                depth: anim_depth,
                tint,
                alpha: 1.0,
                draw_state,
                ..Default::default()
            },
        });
    }
}

fn resting_building_anim_frame(anim: &crate::rules::art_data::BuildingAnimConfig) -> u16 {
    resting_building_anim_frame_values(anim.loop_start, anim.loop_end, anim.start_frame)
}

fn resting_building_anim_frame_values(loop_start: u16, loop_end: u16, start_frame: u16) -> u16 {
    if loop_end > loop_start {
        // LoopEnd is exclusive — last valid frame is loop_end - 1.
        loop_end - 1
    } else {
        start_frame
    }
}

fn resolve_infantry_shp_frame(
    state: &AppState,
    type_id: &str,
    facing: u8,
    anim: Option<&animation::Animation>,
) -> u16 {
    // Pass raw facing (not canonical) to resolve_shp_frame so the
    // facing-to-index division works correctly for any facing count
    // (6, 8, 10, etc.). The absolute frame index encodes the direction.
    let sequence_set = state.animation_sequences.get(type_id);
    if let (Some(anim_state), Some(set)) = (anim, sequence_set) {
        if let Some(def) = set.get(&anim_state.sequence) {
            return animation::resolve_shp_frame(def, facing, anim_state.frame_index);
        }
    }
    // Fallback when no sequence data was built for this type: the standing
    // block is frames 0..7, so the facing slot is the frame index. Uses the
    // same native facing table as the real path so the two cannot disagree.
    animation::infantry_facing_slot(facing)
}

/// Rendered body SHP frame index for a `CanBeOccupied=yes` building.
///
/// Native `GetCurrentFrame` only enters the garrison body-frame formula when
/// the building damage/BState field is nonzero. Healthy idle garrisons keep
/// the raw body frame, which is frame 0 in the current Rust model.
fn rendered_garrison_body_frame_index(
    raw_body_frame: u16,
    building_damage_state_active: bool,
    occupant_count: u32,
    health_current: u16,
    health_max: u16,
    tech_level: i32,
    condition_yellow: f32,
    condition_red: f32,
) -> u16 {
    if !building_damage_state_active {
        return raw_body_frame;
    }
    building_frame_index(
        occupant_count,
        health_current,
        health_max,
        tech_level,
        condition_yellow,
        condition_red,
    )
}

/// BState-gated body SHP formula for a `CanBeOccupied=yes` building.
///
/// For civilian buildings (`tech_level == -1`) the yellow-tier damage step is
/// skipped, and the (occupied, red-HP) collapse maps frame 3 → frame 1 so
/// 3-frame civilian SHPs render correctly.
fn building_frame_index(
    occupant_count: u32,
    health_current: u16,
    health_max: u16,
    tech_level: i32,
    condition_yellow: f32,
    condition_red: f32,
) -> u16 {
    let mut base: u16 = 0;
    if occupant_count > 0 {
        base = 2;
    }
    let ratio = if health_max == 0 {
        1.0
    } else {
        health_current as f32 / health_max as f32
    };
    let red_tier = ratio <= condition_red;
    let yellow_tier = tech_level > 0 && ratio <= condition_yellow;
    if red_tier || yellow_tier {
        base += 1;
    }
    if tech_level == -1 && base == 3 {
        return 1;
    }
    base
}

#[cfg(test)]
mod tests {
    use super::building_frame_index;
    use super::infantry_absorb_slot_is_hidden;
    use super::looping_frame_values;
    use super::rendered_garrison_body_frame_index;
    use super::resting_building_anim_frame;
    use super::selected_building_anim_view;
    use crate::app_building_anim::building_anim_rate_logic_frames;
    use crate::rules::art_data::{
        ArtRegistry, BuildingAnimConfig, BuildingAnimKind, BuildingAnimVariantConfig,
    };
    use crate::rules::ini_parser::IniFile;
    use crate::sim::game_options::GameOptions;

    /// Stock `[GAPOWR_A]`, the Allied power plant's looping smokestack.
    const GAPOWR_A_ART: &str = "[GAPOWR_A]\nNormalized=yes\nStart=0\nLoopStart=0\nLoopEnd=8\n\
                                LoopCount=-1\nRate=220\n";

    fn stock_game_options() -> GameOptions {
        GameOptions::default()
    }

    #[test]
    fn looping_building_anim_rate_applies_normalized_game_speed_scaling() {
        // Rate=220 is a native frame delay of 900/220 = 4 logic frames, and
        // Normalized=yes rescales that through the match game speed on
        // construction. At the stock GameSpeed=1 the delay becomes 6.
        let art = ArtRegistry::from_ini(&IniFile::from_str(GAPOWR_A_ART));
        let options = stock_game_options();
        assert_eq!(options.game_speed, 1);

        assert_eq!(
            building_anim_rate_logic_frames(&art, "GAPOWR_A", Some(&options)),
            6
        );
        // Without the Normalized= step the raw 900/Rate delay stands.
        assert_eq!(building_anim_rate_logic_frames(&art, "GAPOWR_A", None), 4);
    }

    #[test]
    fn looping_building_anim_rate_of_unnormalized_section_is_not_rescaled() {
        // [NATSLA_B] is explicitly Normalized=no so its hard frame delay is kept.
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[NATSLA_B]\nNormalized=no\nStart=0\nEnd=9\nRate=300\n",
        ));
        assert_eq!(
            building_anim_rate_logic_frames(&art, "NATSLA_B", Some(&stock_game_options())),
            3
        );
    }

    #[test]
    fn looping_building_anim_advances_one_frame_per_rate_logic_frames() {
        // 8 frames at 6 logic frames each: frame 0 holds for logic frames 0..5,
        // frame 1 begins on logic frame 6, and the cycle wraps after 48.
        assert_eq!(looping_frame_values(0, 8, 0, 6, false, 0), 0);
        assert_eq!(looping_frame_values(0, 8, 0, 6, false, 5), 0);
        assert_eq!(looping_frame_values(0, 8, 0, 6, false, 6), 1);
        assert_eq!(looping_frame_values(0, 8, 0, 6, false, 47), 7);
        assert_eq!(looping_frame_values(0, 8, 0, 6, false, 48), 0);
    }

    #[test]
    fn looping_building_anim_phase_follows_each_buildings_own_creation_frame() {
        // Two identical power plants raised 15 logic frames apart. gamemd bases
        // each slot animation's frame timer on its own construction frame, so at
        // any later moment the two are on different frames of the same loop.
        // This is the whole point of the per-building phase: a base full of
        // power plants must not pulse in unison.
        let rate: u16 = 6;
        let older_elapsed: u32 = 40;
        let newer_elapsed: u32 = 40 - 15;

        let older = looping_frame_values(0, 8, 0, rate, false, older_elapsed);
        let newer = looping_frame_values(0, 8, 0, rate, false, newer_elapsed);

        assert_eq!(older, 6);
        assert_eq!(newer, 4);
        assert_ne!(older, newer);
    }

    #[test]
    fn looping_building_anim_damaged_variant_uses_its_own_section_rate() {
        // Stock `[GARADR]`: the damaged dish replacement carries Rate=180 where
        // the healthy one carries Rate=220, so the delay has to be resolved from
        // whichever variant was selected, not from the base slot.
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[GARADR_A]\nNormalized=yes\nLoopStart=0\nLoopEnd=14\nLoopCount=-1\nRate=220\n\
             PingPong=yes\n\
             [GARADR_AD]\nImage=GARADR_A\nNormalized=yes\nLoopStart=15\nLoopEnd=29\n\
             LoopCount=-1\nRate=180\nPingPong=yes\n",
        ));
        let options = stock_game_options();

        // 900/220 = 4 → normalized 6; 900/180 = 5 → (5*8)/(1+1) = 20.
        assert_eq!(
            building_anim_rate_logic_frames(&art, "GARADR_A", Some(&options)),
            6
        );
        assert_eq!(
            building_anim_rate_logic_frames(&art, "GARADR_AD", Some(&options)),
            20
        );
    }

    #[test]
    fn ping_pong_building_anim_dwells_on_both_turning_frames() {
        // Stock `[GARADR_A]` — the Allied radar dish — is Start=0, LoopEnd=14,
        // PingPong=yes. gamemd flips direction only after the counter reaches
        // LoopEnd-Start and returns without stepping back, so frame 14 is drawn
        // for a full delay and the bounce is 28 steps over frames 0..=14.
        let frames: Vec<u16> = (0..30)
            .map(|t| looping_frame_values(0, 14, 0, 1, true, t))
            .collect();

        assert_eq!(
            frames,
            vec![
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4,
                3, 2, 1, 0, 1,
            ]
        );
        // The cycle is 2 × (LoopEnd - Start), not 2 × (range - 1).
        assert_eq!(frames[0], frames[28]);
    }

    #[test]
    fn ping_pong_building_anim_ignores_loop_start_and_keys_off_start_frame() {
        // The native flip tests read `Start=` and `LoopEnd=` only. `[GAPLUG_BD]`
        // is Start=10, LoopStart=10, LoopEnd=20, so both turning points land on
        // the same counter value and gamemd walks off the end; VERA holds the
        // last frame it draws instead.
        assert_eq!(looping_frame_values(10, 20, 10, 1, true, 10), 20);
        assert_eq!(looping_frame_values(10, 20, 10, 1, true, 40), 20);
    }

    #[test]
    fn infantry_absorb_building_shows_exactly_one_active_slot() {
        // Yuri's Bio Reactor: ActiveAnim=YAPOWR_A while empty, ActiveAnimTwo=
        // YAPOWR_B once anything is inside — never both, and never neither.
        assert!(!infantry_absorb_slot_is_hidden(0, false));
        assert!(infantry_absorb_slot_is_hidden(1, false));

        assert!(infantry_absorb_slot_is_hidden(0, true));
        assert!(!infantry_absorb_slot_is_hidden(1, true));
    }

    #[test]
    fn infantry_absorb_swap_leaves_later_active_slots_alone() {
        // The native branch only reaches the first two ActiveAnim slots.
        assert!(!infantry_absorb_slot_is_hidden(2, false));
        assert!(!infantry_absorb_slot_is_hidden(3, true));
    }

    #[test]
    fn looping_building_anim_rate_falls_back_to_native_default_without_a_section() {
        let art = ArtRegistry::empty();
        assert_eq!(
            building_anim_rate_logic_frames(&art, "NAOBEL_A", Some(&stock_game_options())),
            crate::rules::art_data::DEFAULT_ART_RATE_LOGIC_FRAMES
        );
    }

    #[test]
    fn one_shot_building_anim_rests_on_last_loop_frame() {
        // LoopEnd is exclusive in RA2 art.ini: LoopEnd=8 means frames 0..8 (8 frames),
        // so the resting frame is 7 (the last valid frame before LoopEnd).
        let anim = BuildingAnimConfig {
            anim_type: "GAAIRC_A".to_string(),
            damaged_variant: None,
            garrisoned_variant: None,
            kind: BuildingAnimKind::Active,
            x: 0,
            y: 0,
            y_sort: 0,
            z_adjust: 0,
            loop_start: 0,
            loop_end: 8,
            loop_count: 1,
            rate: 100,
            start_frame: 0,
            ping_pong: false,
            is_primary: false,
        };

        assert_eq!(resting_building_anim_frame(&anim), 7);
    }

    #[test]
    fn one_shot_building_anim_without_loop_range_uses_start_frame() {
        let anim = BuildingAnimConfig {
            anim_type: "TEST".to_string(),
            damaged_variant: None,
            garrisoned_variant: None,
            kind: BuildingAnimKind::Active,
            x: 0,
            y: 0,
            y_sort: 0,
            z_adjust: 0,
            loop_start: 0,
            loop_end: 0,
            loop_count: 1,
            rate: 100,
            start_frame: 3,
            ping_pong: false,
            is_primary: false,
        };

        assert_eq!(resting_building_anim_frame(&anim), 3);
    }

    #[test]
    fn damaged_active_anim_view_uses_damaged_variant_frame_range() {
        let anim = BuildingAnimConfig {
            anim_type: "CASEAT02_A".to_string(),
            damaged_variant: Some(BuildingAnimVariantConfig {
                anim_type: "CASEAT02_AD".to_string(),
                loop_start: 21,
                loop_end: 39,
                loop_count: -1,
                rate: 150,
                start_frame: 21,
                ping_pong: false,
            }),
            garrisoned_variant: None,
            kind: BuildingAnimKind::Active,
            x: 0,
            y: 0,
            y_sort: 0,
            z_adjust: 0,
            loop_start: 0,
            loop_end: 20,
            loop_count: -1,
            rate: 150,
            start_frame: 0,
            ping_pong: false,
            is_primary: true,
        };

        let selected = selected_building_anim_view(&anim, true, false);

        assert_eq!(selected.anim_type, "CASEAT02_AD");
        assert_eq!(selected.start_frame, 21);
        assert_eq!(selected.loop_start, 21);
        assert_eq!(selected.loop_end, 39);
        assert_eq!(
            looping_frame_values(
                selected.loop_start,
                selected.loop_end,
                selected.start_frame,
                4,
                selected.ping_pong,
                0,
            ),
            21
        );
    }

    #[test]
    fn damaged_active_anim_variant_follows_stored_gate_not_health() {
        let anim = BuildingAnimConfig {
            anim_type: "CASEAT02_A".to_string(),
            damaged_variant: Some(BuildingAnimVariantConfig {
                anim_type: "CASEAT02_AD".to_string(),
                loop_start: 21,
                loop_end: 39,
                loop_count: -1,
                rate: 150,
                start_frame: 21,
                ping_pong: false,
            }),
            garrisoned_variant: None,
            kind: BuildingAnimKind::Active,
            x: 0,
            y: 0,
            y_sort: 0,
            z_adjust: 0,
            loop_start: 0,
            loop_end: 20,
            loop_count: -1,
            rate: 150,
            start_frame: 0,
            ping_pong: false,
            is_primary: true,
        };

        assert_eq!(
            selected_building_anim_view(&anim, false, false).anim_type,
            "CASEAT02_A"
        );
        assert_eq!(
            selected_building_anim_view(&anim, true, false).anim_type,
            "CASEAT02_AD"
        );
    }

    #[test]
    fn garrisoned_active_anim_variant_follows_stored_gate_not_health() {
        let anim = BuildingAnimConfig {
            anim_type: "CAWASH19_A".to_string(),
            damaged_variant: Some(BuildingAnimVariantConfig {
                anim_type: "CAWASH19_AD".to_string(),
                loop_start: 12,
                loop_end: 24,
                loop_count: -1,
                rate: 120,
                start_frame: 12,
                ping_pong: false,
            }),
            garrisoned_variant: Some(BuildingAnimVariantConfig {
                anim_type: "CAWASH19_AG".to_string(),
                loop_start: 24,
                loop_end: 36,
                loop_count: -1,
                rate: 120,
                start_frame: 24,
                ping_pong: false,
            }),
            kind: BuildingAnimKind::Active,
            x: 0,
            y: 0,
            y_sort: 0,
            z_adjust: 0,
            loop_start: 0,
            loop_end: 12,
            loop_count: -1,
            rate: 120,
            start_frame: 0,
            ping_pong: false,
            is_primary: true,
        };

        assert_eq!(
            selected_building_anim_view(&anim, false, true).anim_type,
            "CAWASH19_AG"
        );
        assert_eq!(
            selected_building_anim_view(&anim, true, true).anim_type,
            "CAWASH19_AD"
        );
    }

    // Civilian (TechLevel == -1) — matches CABHUT, CALA01, CAGAS01, CABUNK01, etc.
    // Yellow-tier damage step is gated on TechLevel > 0, so it never fires here.
    // Frame 3 collapses to 1 (occupied + red).

    #[test]
    fn civilian_empty_healthy_returns_0() {
        assert_eq!(building_frame_index(0, 100, 100, -1, 0.5, 0.25), 0);
    }

    #[test]
    fn civilian_empty_yellow_tier_returns_0() {
        // ratio = 0.4: below ConditionYellow but above ConditionRed.
        // Yellow gate is `tech_level > 0` — fails for civilian, so no +1.
        assert_eq!(building_frame_index(0, 40, 100, -1, 0.5, 0.25), 0);
    }

    #[test]
    fn civilian_empty_red_tier_returns_1() {
        assert_eq!(building_frame_index(0, 20, 100, -1, 0.5, 0.25), 1);
    }

    #[test]
    fn civilian_occupied_healthy_bstate_formula_returns_2() {
        assert_eq!(building_frame_index(1, 100, 100, -1, 0.5, 0.25), 2);
    }

    #[test]
    fn civilian_occupied_yellow_tier_returns_2() {
        // Same yellow-gate behavior as empty case.
        assert_eq!(building_frame_index(1, 40, 100, -1, 0.5, 0.25), 2);
    }

    #[test]
    fn civilian_occupied_red_tier_collapses_to_1() {
        // base=2 (occupied) + 1 (red) = 3 → collapse rule → 1.
        assert_eq!(building_frame_index(1, 20, 100, -1, 0.5, 0.25), 1);
    }

    // Buildable (TechLevel >= 1) — TS-era "buildable garrisonable" structures
    // (none in standard YR but the formula path is real). Yellow tier fires.

    #[test]
    fn buildable_empty_healthy_returns_0() {
        assert_eq!(building_frame_index(0, 100, 100, 5, 0.5, 0.25), 0);
    }

    #[test]
    fn buildable_empty_yellow_tier_returns_1() {
        assert_eq!(building_frame_index(0, 40, 100, 5, 0.5, 0.25), 1);
    }

    #[test]
    fn buildable_occupied_healthy_returns_2() {
        assert_eq!(building_frame_index(1, 100, 100, 5, 0.5, 0.25), 2);
    }

    #[test]
    fn buildable_occupied_red_tier_returns_3() {
        // No civilian collapse (tech_level != -1).
        assert_eq!(building_frame_index(1, 20, 100, 5, 0.5, 0.25), 3);
    }

    // Edge cases.

    #[test]
    fn zero_max_hp_treats_as_healthy() {
        // Avoids division-by-zero; entity not yet fully initialized.
        assert_eq!(building_frame_index(0, 0, 0, -1, 0.5, 0.25), 0);
    }

    #[test]
    fn boundary_at_condition_red_inclusive() {
        // ratio == ConditionRed exactly → red_tier fires (<=).
        assert_eq!(building_frame_index(0, 25, 100, -1, 0.5, 0.25), 1);
    }

    #[test]
    fn occupied_cagas01_healthy_bstate_zero_renders_frame_zero() {
        assert_eq!(
            rendered_garrison_body_frame_index(0, false, 1, 100, 100, -1, 0.5, 0.25),
            0
        );
    }

    #[test]
    fn occupied_cagas01_yellow_bstate_false_stays_raw_frame() {
        assert_eq!(
            rendered_garrison_body_frame_index(7, false, 1, 40, 100, -1, 0.5, 0.25),
            7
        );
    }

    #[test]
    fn occupied_cagas01_yellow_bstate_true_uses_frame_two() {
        assert_eq!(
            rendered_garrison_body_frame_index(0, true, 1, 40, 100, -1, 0.5, 0.25),
            2
        );
    }

    #[test]
    fn occupied_cagas01_red_bstate_true_collapses_to_frame_one() {
        assert_eq!(
            rendered_garrison_body_frame_index(0, true, 1, 20, 100, -1, 0.5, 0.25),
            1
        );
    }

    #[test]
    fn zero_max_hp_render_frame_treats_as_healthy() {
        assert_eq!(
            rendered_garrison_body_frame_index(4, false, 1, 0, 0, -1, 0.5, 0.25),
            4
        );
    }
}
