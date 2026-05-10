//! Bridge instance emission — body, shadow, railings, deck-variant overrides.
//!
//! Reads `BridgeRuntimeCell` post-tick (NOT `OverlayGrid`) and emits sprite
//! instances for the bridge body, body shadow, and railing passes per the
//! per-frame draw chain in `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md` §3.3, §3.4.
//!
//! Three open-RE values ship as named constants here so each is a single
//! change-point if visual diff resolves them differently.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.
//! - Read-only access to sim state via `AppState`.

use std::collections::BTreeMap;

use crate::app::AppState;
use crate::map::lighting::{self, LightingGrid};
use crate::map::terrain::{self, TILE_HEIGHT, TILE_WIDTH};
use crate::render::batch::SpriteInstance;
use crate::render::bridge_atlas::{BridgeAtlasLookup, is_high_bridge_body_name};
use crate::render::bridge_railing_atlas::BridgeKind;
use crate::sim::bridge_state::{Axis, BridgeRuntimeCell, BridgeRuntimeState, DamageState};

use super::helpers::{compute_sprite_depth_params, in_view};

/// Latin-square jitter for healthy bridge body frames at base state byte 0
/// (NS) or 9 (EW). Verified raw memory read at gamemd's `g_LatinSquare`
/// (RE doc §5; ledger #1).
const BRIDGE_BODY_LATIN_SQUARE: [u8; 16] =
    [0, 1, 2, 3, 3, 2, 1, 0, 2, 3, 0, 1, 1, 0, 3, 2];

/// Body Y offset for state bytes 0..8 (NS axis — BRIDGE2 / BRIDGEB2).
/// `-(CellHeight * 2 + 1) = -31px`. Matches the legacy
/// `bridge_y_offset_for_name` mapping that this module replaced.
const BRIDGE_BODY_Y_OFFSET_NS: f32 = -31.0;
/// Body Y offset for state bytes 9..17 (EW axis — BRIDGE1 / BRIDGEB1).
/// `-(CellHeight + 1) = -16px`.
const BRIDGE_BODY_Y_OFFSET_EW: f32 = -16.0;

/// Bonus added to `cell.deck_level` before the depth calc for HasBridge cells.
/// RE doc §3.3.1, ledger #6.
const BRIDGE_HEIGHT_BONUS: u8 = 4;

/// Shadow X displacement on EW states 9..17. RE doc §10 open Q2 — value
/// unresolved between -15 and -45. Defaults to -15. Single change point.
pub const BRIDGE_SHADOW_EW_DX: i32 = -15;
/// Shadow Y displacement on EW states 9..17. Verified -0x2D = +7
/// (RE doc §3.3.2, ledger #10).
pub const BRIDGE_SHADOW_EW_DY: i32 = 7;

/// Translate a cell's `(damage_state, axis)` into the SHP frame index for
/// `bridge.tem` / `bridgb.tem`. Per `BRIDGE_RENDERING_GHIDRA_REPORT.md` §12,
/// the SHP body half is laid out:
///   - frames `0..8`: EW body (healthy 0..5, damaged 6, partial 7-8)
///   - frames `9..17`: NS body
///
/// This is the *opposite* of the sim's state-byte encoding (`Axis::NS = 0..8`,
/// `Axis::EW = 9..17`), so the renderer flips the axis-to-frame-range mapping.
///
/// Latin-square jitter applies ONLY to the boundary state (`variant: 0`),
/// matching binary `DrawOverlay_Body @ 0x47F6A0`: `if (state == 0 || state ==
/// 9) state += g_LatinSquare[...]`. Healthy variants 1..=5 are written by
/// `apply_ramp_transition` perpendicular damage (e.g., `(NS, DamageA, 0..=3)
/// → 4`); the binary draws those at `frame = state` directly with no jitter
/// (BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md §3.3.1).
fn compute_bridge_body_shp_frame(state: DamageState, axis: Axis, rx: u16, ry: u16) -> u8 {
    let axis_base: u8 = match axis {
        Axis::EW => 0,
        Axis::NS => 9,
    };
    let local: u8 = match state {
        DamageState::Healthy { variant: 0 } => {
            let idx = ((ry & 3) as usize) << 2 | (rx & 3) as usize;
            BRIDGE_BODY_LATIN_SQUARE[idx]
        }
        DamageState::Healthy { variant } => variant.min(5),
        DamageState::Damaged => 6,
        // Per RE doc §3.1: NS PartialA=7/PartialB=8, EW PartialA=8/PartialB=7
        // (the within-axis ordering is reversed for EW). The state-byte encoding
        // bakes this in, so we read the relevant bits via to_state_byte.
        other => {
            let sb = other.to_state_byte(axis);
            sb.saturating_sub(axis_base)
        }
    };
    axis_base + local
}

/// Build sprite instances for the bridge body pass (RE doc §3.3, Step 5
/// pass 1). Reads `BridgeRuntimeCell.damage_state` post-tick.
///
/// Takes only the fields the body builder actually needs from `AppState`
/// so the function is exercisable in unit tests with a pure-data mock atlas
/// (`BridgeAtlasLookup` trait). Shadow + railing builders below still take
/// `&AppState` directly — same minimal-context refactor pending.
#[allow(clippy::too_many_arguments)]
pub fn build_bridge_body_instances_inner(
    bridge_state: &BridgeRuntimeState,
    atlas: &dyn BridgeAtlasLookup,
    overlay_names: &BTreeMap<u8, String>,
    height_map: &BTreeMap<(u16, u16), u8>,
    lighting_grid: &LightingGrid,
    origin_y: f32,
    world_height: f32,
    cam_x: f32,
    cam_y: f32,
    sw: f32,
    sh: f32,
    out: &mut Vec<SpriteInstance>,
) {
    for ((rx, ry), cell) in bridge_state.iter_cells() {
        if !cell.deck_present || matches!(cell.damage_state, DamageState::Destroyed) {
            continue;
        }
        let Some(axis) = cell.axis else { continue };
        let Some(name) = overlay_names.get(&cell.overlay_byte) else {
            continue;
        };
        if !is_high_bridge_body_name(name) {
            continue;
        }

        let frame = compute_bridge_body_shp_frame(cell.damage_state, axis, rx, ry);
        let y_offset = match axis {
            Axis::NS => BRIDGE_BODY_Y_OFFSET_NS,
            Axis::EW => BRIDGE_BODY_Y_OFFSET_EW,
        };

        let z: u8 = height_map
            .get(&(rx, ry))
            .copied()
            .unwrap_or(cell.deck_level);
        let (sx, sy) = terrain::iso_to_screen(rx, ry, z);
        let sy = sy + y_offset;
        if !in_view(sx, sy, 120.0, 120.0, cam_x, cam_y, sw, sh, 120.0) {
            continue;
        }

        let Some(spr) = atlas.body_entry(name, frame) else {
            log::warn!(
                "bridge body atlas miss: name={name} frame={frame} cell=({rx},{ry})"
            );
            continue;
        };

        let depth_z = z.saturating_add(BRIDGE_HEIGHT_BONUS);
        let depth = compute_sprite_depth_params(origin_y, world_height, sy, depth_z);
        let tint: [f32; 3] = lighting_grid
            .get(&(rx, ry))
            .copied()
            .unwrap_or(lighting::DEFAULT_TINT);
        out.push(SpriteInstance {
            position: [
                sx + TILE_WIDTH / 2.0 + spr.offset_x,
                sy + TILE_HEIGHT / 2.0 + spr.offset_y,
            ],
            size: spr.pixel_size,
            uv_origin: spr.uv_origin,
            uv_size: spr.uv_size,
            depth,
            tint,
            alpha: 1.0,
            ..Default::default()
        });
    }
}

/// Thin `AppState` wrapper around `build_bridge_body_instances_inner`. Pulls
/// the seven fields the inner function needs out of `state` and forwards.
pub fn build_bridge_body_instances(
    state: &AppState,
    sw: f32,
    sh: f32,
    out: &mut Vec<SpriteInstance>,
) {
    let Some(sim) = state.simulation.as_ref() else {
        return;
    };
    let Some(bridge_state) = sim.bridge_state.as_ref() else {
        return;
    };
    let Some(atlas) = state.bridge_atlas.as_ref() else {
        return;
    };
    let (origin_y, world_height) = state
        .terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));
    build_bridge_body_instances_inner(
        bridge_state,
        atlas,
        &state.overlay_names,
        &state.height_map,
        &state.lighting_grid,
        origin_y,
        world_height,
        state.camera_x,
        state.camera_y,
        sw,
        sh,
        out,
    );
}

/// Build sprite instances for the bridge body shadow pass (RE doc §3.3.2,
/// Step 5 pass 2). Shadow frame = `(frame_count / 2) + state`. EW states
/// 9..17 get a `(BRIDGE_SHADOW_EW_DX, +BRIDGE_SHADOW_EW_DY)` shift per
/// ledger #9–10. Drawn passthrough (Z-test ON, Z-write OFF, neutral tint).
pub fn build_bridge_shadow_instances(
    state: &AppState,
    sw: f32,
    sh: f32,
    out: &mut Vec<SpriteInstance>,
) {
    let Some(sim) = state.simulation.as_ref() else {
        return;
    };
    let Some(bridge_state) = sim.bridge_state.as_ref() else {
        return;
    };
    let Some(atlas) = state.bridge_atlas.as_ref() else {
        return;
    };
    let (origin_y, world_height) = state
        .terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));
    let (cam_x, cam_y) = (state.camera_x, state.camera_y);

    for ((rx, ry), cell) in bridge_state.iter_cells() {
        if !cell.deck_present || matches!(cell.damage_state, DamageState::Destroyed) {
            continue;
        }
        let Some(axis) = cell.axis else { continue };
        let Some(name) = state.overlay_names.get(&cell.overlay_byte) else {
            continue;
        };
        if !is_high_bridge_body_name(name) {
            continue;
        }

        let frame = compute_bridge_body_shp_frame(cell.damage_state, axis, rx, ry);
        let y_offset = match axis {
            Axis::NS => BRIDGE_BODY_Y_OFFSET_NS,
            Axis::EW => BRIDGE_BODY_Y_OFFSET_EW,
        };

        let z: u8 = state
            .height_map
            .get(&(rx, ry))
            .copied()
            .unwrap_or(cell.deck_level);
        let (mut sx, mut sy) = terrain::iso_to_screen(rx, ry, z);
        sy += y_offset;

        // EW-axis shadow shift (RE doc §3.3.2, ledger #9-10).
        if axis == Axis::EW {
            sx += BRIDGE_SHADOW_EW_DX as f32;
            sy += BRIDGE_SHADOW_EW_DY as f32;
        }

        if !in_view(sx, sy, 120.0, 120.0, cam_x, cam_y, sw, sh, 120.0) {
            continue;
        }

        let Some(spr) = atlas.shadow_entry(name, frame) else {
            log::warn!(
                "bridge shadow atlas miss: name={name} frame={frame} cell=({rx},{ry})"
            );
            continue;
        };

        let depth_z = z.saturating_add(BRIDGE_HEIGHT_BONUS);
        let depth = compute_sprite_depth_params(origin_y, world_height, sy, depth_z);
        // Shadow uses neutral tint, no per-cell lighting (ledger #12).
        let tint: [f32; 3] = lighting::DEFAULT_TINT;
        out.push(SpriteInstance {
            position: [
                sx + TILE_WIDTH / 2.0 + spr.offset_x,
                sy + TILE_HEIGHT / 2.0 + spr.offset_y,
            ],
            size: spr.pixel_size,
            uv_origin: spr.uv_origin,
            uv_size: spr.uv_size,
            depth,
            tint,
            alpha: 1.0,
            ..Default::default()
        });
    }
}

/// Build sprite instances for the bridge railing pass (RE doc §3.4.1, Step 7).
/// Drawn AFTER unit/ground merge AND AFTER cliff redraw, BEFORE debug — see
/// `draw_passes.rs` ordering. Skips cells where the railing-table entry is
/// `None` (`shp_frame_1based == 0` ⇒ no railing for this sub-tile).
pub fn build_bridge_railing_instances(
    state: &AppState,
    sw: f32,
    sh: f32,
    out: &mut Vec<SpriteInstance>,
) {
    let Some(sim) = state.simulation.as_ref() else {
        return;
    };
    let Some(bridge_state) = sim.bridge_state.as_ref() else {
        return;
    };
    let Some(atlas) = state.bridge_railing_atlas.as_ref() else {
        return;
    };
    let (origin_y, world_height) = state
        .terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));
    let (cam_x, cam_y) = (state.camera_x, state.camera_y);

    for ((rx, ry), cell) in bridge_state.iter_cells() {
        if !cell.deck_present || matches!(cell.damage_state, DamageState::Destroyed) {
            continue;
        }
        let Some((kind, sub_idx)) = resolve_bridge_kind_and_sub_idx(state, rx, ry, cell) else {
            continue;
        };
        let Some(entry) = atlas.entry(kind, sub_idx) else {
            continue;
        };

        let z: u8 = state
            .height_map
            .get(&(rx, ry))
            .copied()
            .unwrap_or(cell.deck_level);
        let (sx, sy) = terrain::iso_to_screen(rx, ry, z);
        let final_x = sx + entry.dx as f32 + TILE_WIDTH / 2.0;
        let final_y = sy + entry.dy as f32 + TILE_HEIGHT / 2.0;
        if !in_view(final_x, final_y, 60.0, 60.0, cam_x, cam_y, sw, sh, 60.0) {
            continue;
        }

        let depth_z = z.saturating_add(BRIDGE_HEIGHT_BONUS);
        let depth = compute_sprite_depth_params(origin_y, world_height, final_y, depth_z);
        // Railings use neutral tint per ledger #20.
        let tint: [f32; 3] = lighting::DEFAULT_TINT;
        out.push(SpriteInstance {
            position: [final_x + entry.offset_x, final_y + entry.offset_y],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth,
            tint,
            alpha: 1.0,
            ..Default::default()
        });
    }
}

/// Resolve `(BridgeKind, sub_idx)` for a bridge cell.
///
/// Mapping per RE doc §3.4.1 + verified against the codebase
/// (src/map/resolved_terrain.rs:48-55, src/map/overlay_types.rs:25-28):
/// - `BRIDGE1`, `BRIDGEB1`, `BRIDGE2`, `BRIDGEB2` → Concrete (HIGH bridges).
///   The `1` vs `2` suffix is axis (EW vs NS), not material — all four are
///   concrete.
/// - `LOBRDG*` → Wood (LOW bridges).
///
/// `sub_idx` comes from `ResolvedTerrainCell.final_sub_tile` and is used
/// directly as the railing-table slot index.
fn resolve_bridge_kind_and_sub_idx(
    state: &AppState,
    rx: u16,
    ry: u16,
    cell: &BridgeRuntimeCell,
) -> Option<(BridgeKind, u8)> {
    let name = state
        .overlay_names
        .get(&cell.overlay_byte)?
        .to_ascii_uppercase();
    let kind = if matches!(
        name.as_str(),
        "BRIDGE1" | "BRIDGEB1" | "BRIDGE2" | "BRIDGEB2"
    ) {
        BridgeKind::Concrete
    } else if name.starts_with("LOBRDG") {
        BridgeKind::Wood
    } else {
        return None;
    };
    let sub_idx: u8 = state.resolved_terrain.as_ref()?.cell(rx, ry)?.final_sub_tile;
    Some((kind, sub_idx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin_square_value_at_xy() {
        // (cell.x, cell.y) = (1, 2) → idx = ((2&3)<<2) | (1&3) = 8|1 = 9.
        // BRIDGE_BODY_LATIN_SQUARE[9] = 3.
        assert_eq!(BRIDGE_BODY_LATIN_SQUARE[((2 & 3) << 2) | (1 & 3)], 3);
    }

    /// SHP frame selection routes EW cells to body frames 0..8 and NS cells
    /// to 9..17 — the inverse of the sim's state-byte encoding. Per
    /// `BRIDGE_RENDERING_GHIDRA_REPORT.md` §12, bridge.tem packs EW body
    /// frames at 0-8 and NS body frames at 9-17.
    #[test]
    fn damaged_cell_routes_to_axis_correct_shp_frame() {
        // NS Damaged → SHP frame 9 + 6 = 15.
        assert_eq!(
            compute_bridge_body_shp_frame(DamageState::Damaged, Axis::NS, 0, 0),
            15
        );
        // EW Damaged → SHP frame 0 + 6 = 6.
        assert_eq!(
            compute_bridge_body_shp_frame(DamageState::Damaged, Axis::EW, 0, 0),
            6
        );
    }

    /// Latin-square jitter applies only to `Healthy { variant: 0 }` — the
    /// boundary state byte that the binary `DrawOverlay_Body` checks for
    /// `state == 0 || state == 9`. Variants 1..=5 (written by
    /// `apply_ramp_transition` perpendicular damage) draw at `axis_base +
    /// variant` directly, no jitter. Per BRIDGE_DISPLAY_TABLE §3.3.1.
    #[test]
    fn healthy_variant_zero_uses_latin_square_jitter() {
        // (1, 2) → LATIN[9] = 3 → EW body SHP frame = 0 + 3 = 3.
        assert_eq!(
            compute_bridge_body_shp_frame(DamageState::Healthy { variant: 0 }, Axis::EW, 1, 2),
            3
        );
        // Same xy on NS axis → SHP frame = 9 + 3 = 12.
        assert_eq!(
            compute_bridge_body_shp_frame(DamageState::Healthy { variant: 0 }, Axis::NS, 1, 2),
            12
        );
    }

    #[test]
    fn healthy_variants_one_to_five_skip_latin_and_use_variant_directly() {
        // Per binary: state bytes 1..=5 (NS) and 10..=14 (EW) draw frame =
        // state directly (no jitter). Variant 4 written by NS_DamageA on
        // 0..=3 healthy targets per HIGH §11.1.
        assert_eq!(
            compute_bridge_body_shp_frame(DamageState::Healthy { variant: 4 }, Axis::EW, 1, 2),
            4
        );
        assert_eq!(
            compute_bridge_body_shp_frame(DamageState::Healthy { variant: 5 }, Axis::EW, 1, 2),
            5
        );
        // Same variants on NS axis: frame = 9 + variant.
        assert_eq!(
            compute_bridge_body_shp_frame(DamageState::Healthy { variant: 4 }, Axis::NS, 1, 2),
            13
        );
        assert_eq!(
            compute_bridge_body_shp_frame(DamageState::Healthy { variant: 5 }, Axis::NS, 1, 2),
            14
        );
        // Different (rx, ry) does NOT change the result for non-zero variants
        // (Latin-square is bypassed) — guards against a future refactor
        // re-introducing jitter on damage-progression frames.
        assert_eq!(
            compute_bridge_body_shp_frame(DamageState::Healthy { variant: 4 }, Axis::EW, 0, 0),
            compute_bridge_body_shp_frame(DamageState::Healthy { variant: 4 }, Axis::EW, 1, 2)
        );
    }

    #[test]
    fn shadow_ew_shift_constants_present() {
        // RE doc §10 open Q2: BRIDGE_SHADOW_EW_DX may be -15 or -45.
        // BRIDGE_SHADOW_EW_DY verified at +7. Either way, the shift must be
        // non-zero — visual diff (Task 17) resolves the X value.
        assert!(BRIDGE_SHADOW_EW_DX != 0 || BRIDGE_SHADOW_EW_DY != 0);
    }

    #[test]
    fn latin_square_table_is_canonical_4x4() {
        // RE doc §5: verified raw memory read at g_LatinSquare.
        assert_eq!(
            BRIDGE_BODY_LATIN_SQUARE,
            [0, 1, 2, 3, 3, 2, 1, 0, 2, 3, 0, 1, 1, 0, 3, 2]
        );
    }
}
