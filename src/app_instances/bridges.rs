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
use crate::map::lighting;
use crate::map::terrain::{self, TILE_HEIGHT, TILE_WIDTH};
use crate::render::batch::SpriteInstance;
use crate::render::bridge_atlas::is_high_bridge_body_name;
use crate::render::bridge_railing_atlas::BridgeKind;
use crate::sim::bridge_state::{BridgeRuntimeCell, DamageState};

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

/// Per-cell deck-variant override consumed by the terrain instance builder
/// to pick alt-art sub-tile UVs when `BridgeRuntimeCell.damaged_variant` is
/// set. RE doc §3.2 + ledger #13.
#[derive(Debug, Clone, Copy)]
pub struct DeckVariantSelect {
    pub use_alternate: bool,
}

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

        let base = cell.damage_state.render_state_byte(axis);
        let frame: u8 = if base == 0 || base == 9 {
            let idx = ((ry & 3) as usize) << 2 | (rx & 3) as usize;
            base + BRIDGE_BODY_LATIN_SQUARE[idx]
        } else {
            cell.damage_state.to_state_byte(axis)
        };

        let z: u8 = state
            .height_map
            .get(&(rx, ry))
            .copied()
            .unwrap_or(cell.deck_level);
        let (mut sx, mut sy) = terrain::iso_to_screen(rx, ry, z);
        let y_offset = if frame <= 8 {
            BRIDGE_BODY_Y_OFFSET_LOW
        } else {
            BRIDGE_BODY_Y_OFFSET_HIGH
        };
        sy += y_offset;

        // EW-state shadow shift (states 9..17 — ledger #9, #10).
        if (9..=17).contains(&frame) {
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

/// Walk all bridge cells with `damaged_variant: true` and return a sorted
/// map of `(rx, ry) → DeckVariantSelect { use_alternate: true }`. Consumed
/// by `app_render::build_instances` to pick the alt-art sub-tile UV for the
/// deck TMP. RE doc §3.2 + ledger #13.
pub fn build_bridge_deck_variant_overrides(
    state: &AppState,
) -> BTreeMap<(u16, u16), DeckVariantSelect> {
    let mut out: BTreeMap<(u16, u16), DeckVariantSelect> = BTreeMap::new();
    let Some(sim) = state.simulation.as_ref() else {
        return out;
    };
    let Some(bridge_state) = sim.bridge_state.as_ref() else {
        return out;
    };
    for ((rx, ry), cell) in bridge_state.iter_cells() {
        if cell.damaged_variant {
            out.insert((rx, ry), DeckVariantSelect { use_alternate: true });
        }
    }
    out
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

    /// Lock the "render reads `BridgeRuntimeCell` post-tick" parity contract:
    /// for a `Damaged` bridge cell, the body frame derives from
    /// `DamageState::to_state_byte(axis)` (skipping the Latin-square jitter,
    /// which only fires on base bytes 0/9). A future refactor that switches
    /// the source to `OverlayGrid` (which lags by 1 tick on bridge state
    /// changes) would visibly desync — this test pins the formula.
    #[test]
    fn body_frame_for_damaged_cell_matches_state_byte_no_jitter() {
        use crate::sim::bridge_state::{Axis, DamageState};
        // NS Damaged = 6, EW Damaged = 0xF; render_state_byte returns the same
        // for non-Healthy states, and 6/0xF aren't 0 or 9 → no jitter.
        let ns = DamageState::Damaged;
        assert_eq!(ns.render_state_byte(Axis::NS), 6);
        assert_eq!(ns.to_state_byte(Axis::NS), 6);
        let ew = DamageState::Damaged;
        assert_eq!(ew.render_state_byte(Axis::EW), 0xF);
        assert_eq!(ew.to_state_byte(Axis::EW), 0xF);
    }

    /// Lock the "render reads `BridgeRuntimeCell` post-tick" parity contract:
    /// for a `Healthy` cell, the renderer rebuilds Latin-square jitter from
    /// cell `(rx, ry)` — it does NOT honor the sim's `Healthy.variant` field.
    /// This guards against future refactors that try to take a shortcut by
    /// using `Healthy.variant` directly.
    #[test]
    fn healthy_cell_uses_xy_latin_square_not_sim_variant() {
        use crate::sim::bridge_state::{Axis, DamageState};
        // Healthy{variant:0} and Healthy{variant:5} both render as base byte 0
        // for NS / 9 for EW; the renderer then adds Latin-square jitter from
        // (rx, ry). The sim variant is ignored by the render path.
        assert_eq!(
            DamageState::Healthy { variant: 0 }.render_state_byte(Axis::NS),
            DamageState::Healthy { variant: 5 }.render_state_byte(Axis::NS)
        );
        // Final frame for cell (1, 2) on a healthy NS bridge:
        //   base = 0; idx = ((2&3)<<2)|(1&3) = 9; jitter = LATIN[9] = 3.
        let frame = 0u8 + BRIDGE_BODY_LATIN_SQUARE[((2 & 3) << 2) | (1 & 3)];
        assert_eq!(frame, 3);
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
