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

use crate::app::AppState;
use crate::map::lighting;
use crate::map::terrain::{self, TILE_HEIGHT, TILE_WIDTH};
use crate::render::batch::SpriteInstance;
use crate::render::bridge_atlas::is_high_bridge_body_name;
use crate::sim::bridge_state::DamageState;

use super::helpers::{compute_sprite_depth_params, in_view};

/// Latin-square jitter for healthy bridge body frames at base state byte 0
/// (NS) or 9 (EW). Verified raw memory read at gamemd's `g_LatinSquare`
/// (RE doc §5; ledger #1).
const BRIDGE_BODY_LATIN_SQUARE: [u8; 16] =
    [0, 1, 2, 3, 3, 2, 1, 0, 2, 3, 0, 1, 1, 0, 3, 2];

/// Body Y offset for state bytes 0..8 (HasBridge cells). RE doc §3.3.3, ledger #5.
const BRIDGE_BODY_Y_OFFSET_LOW: f32 = -16.0;
/// Body Y offset for state bytes 9..17. RE doc §3.3.3, ledger #5.
const BRIDGE_BODY_Y_OFFSET_HIGH: f32 = -31.0;

/// Bonus added to `cell.deck_level` before the depth calc for HasBridge cells.
/// RE doc §3.3.1, ledger #6.
const BRIDGE_HEIGHT_BONUS: u8 = 4;

/// Shadow X displacement on EW states 9..17. RE doc §10 open Q2 — value
/// unresolved between -15 and -45. Defaults to -15. Single change point.
pub const BRIDGE_SHADOW_EW_DX: i32 = -15;
/// Shadow Y displacement on EW states 9..17. Verified -0x2D = +7
/// (RE doc §3.3.2, ledger #10).
pub const BRIDGE_SHADOW_EW_DY: i32 = 7;

/// Build sprite instances for the bridge body pass (RE doc §3.3, Step 5
/// pass 1). Reads `BridgeRuntimeCell.overlay_byte` post-tick; uses
/// Latin-square jitter on base state bytes 0 and 9 only.
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

        let base = cell.damage_state.render_state_byte(axis);
        let frame: u8 = if base == 0 || base == 9 {
            let idx = ((ry & 3) as usize) << 2 | (rx & 3) as usize;
            base + BRIDGE_BODY_LATIN_SQUARE[idx]
        } else {
            cell.damage_state.to_state_byte(axis)
        };
        let y_offset = if frame <= 8 {
            BRIDGE_BODY_Y_OFFSET_LOW
        } else {
            BRIDGE_BODY_Y_OFFSET_HIGH
        };

        let z: u8 = state
            .height_map
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
        let tint: [f32; 3] = state
            .lighting_grid
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
        });
    }
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

    #[test]
    fn latin_square_table_is_canonical_4x4() {
        // RE doc §5: verified raw memory read at g_LatinSquare.
        assert_eq!(
            BRIDGE_BODY_LATIN_SQUARE,
            [0, 1, 2, 3, 3, 2, 1, 0, 2, 3, 0, 1, 1, 0, 3, 2]
        );
    }
}
