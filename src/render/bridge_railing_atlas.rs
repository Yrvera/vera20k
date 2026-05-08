//! Bridge railing atlas — RAILBRDG.tem SHP packed into a single atlas page,
//! plus concrete + wood lookup tables that map a deck sub-tile index to a
//! railing frame entry.
//!
//! Mirrors the binary's `g_BridgeRailingSHP` (theater-loaded) and the two
//! parallel 10-entry railing tables (concrete + wood). See
//! `ra2-rust-game-docs/BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md` §3.4.1 for the
//! entry format. Final values live in
//! `ra2-rust-game-docs/BRIDGE_RAILING_TABLE_VALUES.md` once a live-debugger
//! capture is taken (Phase D Task 3).
//!
//! ## Dependency rules
//! - Part of render/ — depends on assets/ + render/batch.

use std::collections::HashMap;

use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::assets::shp_file::ShpFile;
use crate::render::batch::{BatchRenderer, BatchTexture};
use crate::render::gpu::GpuContext;

const SPRITE_PADDING: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeKind {
    Concrete,
    Wood,
}

/// One entry of the 10-element bridge-railing lookup table.
/// `shp_frame == 0` means "no railing for this sub-tile" (skip emit).
#[derive(Clone, Copy, Debug)]
pub struct RailingEntry {
    pub shp_frame: u8,
    pub dx: i16,
    pub dy: i16,
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub pixel_size: [f32; 2],
    pub offset_x: f32,
    pub offset_y: f32,
}

pub struct BridgeRailingAtlas {
    pub texture: BatchTexture,
    concrete_table: [Option<RailingEntry>; 10],
    wood_table: [Option<RailingEntry>; 10],
}

impl BridgeRailingAtlas {
    pub fn entry(&self, kind: BridgeKind, sub_idx: u8) -> Option<&RailingEntry> {
        let table = match kind {
            BridgeKind::Concrete => &self.concrete_table,
            BridgeKind::Wood => &self.wood_table,
        };
        table.get(sub_idx as usize).and_then(Option::as_ref)
    }
}

/// Concrete-bridge railing values — placeholder all-zero entries until a
/// live-debugger capture replaces them. Each tuple is
/// `(shp_frame_1based, dx, dy)`; `shp_frame == 0` ⇒ no railing for the slot.
const CONCRETE_RAILING_VALUES: [(u8, i16, i16); 10] = [
    (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0),
    (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0),
];

/// Wood-bridge railing values — placeholder all-zero entries until a
/// live-debugger capture replaces them.
const WOOD_RAILING_VALUES: [(u8, i16, i16); 10] = [
    (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0),
    (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0),
];

pub fn build_bridge_railing_atlas(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    asset_manager: &AssetManager,
    theater_palette: &Palette,
    theater_ext: &str,
) -> Option<BridgeRailingAtlas> {
    let candidates = [
        format!("railbrdg.{}", theater_ext),
        "railbrdg.shp".to_string(),
    ];
    let shp: ShpFile = candidates.iter().find_map(|name| {
        let data = asset_manager.get_ref(name)?;
        let shp = ShpFile::from_bytes(data).ok()?;
        let has_drawable = shp
            .frames
            .iter()
            .any(|fr| fr.frame_width > 0 && fr.frame_height > 0);
        has_drawable.then_some(shp)
    })?;

    let (texture, frame_entries) = pack_single_shp(gpu, batch, &shp, theater_palette)?;

    let concrete_table = build_table(&CONCRETE_RAILING_VALUES, &frame_entries);
    let wood_table = build_table(&WOOD_RAILING_VALUES, &frame_entries);

    Some(BridgeRailingAtlas {
        texture,
        concrete_table,
        wood_table,
    })
}

fn build_table(
    values: &[(u8, i16, i16); 10],
    frame_entries: &HashMap<usize, FrameEntry>,
) -> [Option<RailingEntry>; 10] {
    let mut out: [Option<RailingEntry>; 10] = [None; 10];
    for (slot, &(shp_frame_1based, dx, dy)) in values.iter().enumerate() {
        if shp_frame_1based == 0 {
            continue;
        }
        let frame_0based = (shp_frame_1based - 1) as usize;
        let Some(e) = frame_entries.get(&frame_0based) else {
            continue;
        };
        out[slot] = Some(RailingEntry {
            shp_frame: shp_frame_1based,
            dx,
            dy,
            uv_origin: e.uv_origin,
            uv_size: e.uv_size,
            pixel_size: e.pixel_size,
            offset_x: e.offset_x,
            offset_y: e.offset_y,
        });
    }
    out
}

#[derive(Clone, Copy, Debug)]
struct FrameEntry {
    uv_origin: [f32; 2],
    uv_size: [f32; 2],
    pixel_size: [f32; 2],
    offset_x: f32,
    offset_y: f32,
}

struct RenderedFrame {
    frame_idx: usize,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    offset_x: f32,
    offset_y: f32,
}

fn pack_single_shp(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    shp: &ShpFile,
    palette: &Palette,
) -> Option<(BatchTexture, HashMap<usize, FrameEntry>)> {
    let full_w: u32 = shp.width as u32;
    let full_h: u32 = shp.height as u32;
    let mut rendered: Vec<RenderedFrame> = Vec::with_capacity(shp.frames.len());
    for (frame_idx, frame) in shp.frames.iter().enumerate() {
        if frame.frame_width == 0 || frame.frame_height == 0 {
            continue;
        }
        let frame_rgba: Vec<u8> = match shp.frame_to_rgba(frame_idx, palette) {
            Ok(rgba) => rgba,
            Err(_) => continue,
        };
        let mut full_rgba: Vec<u8> = vec![0u8; (full_w * full_h * 4) as usize];
        let fw: u32 = frame.frame_width as u32;
        let fh: u32 = frame.frame_height as u32;
        let fx: u32 = frame.frame_x as u32;
        let fy: u32 = frame.frame_y as u32;
        for y in 0..fh {
            let dst_y: u32 = fy + y;
            if dst_y >= full_h {
                break;
            }
            let src_off: usize = (y * fw * 4) as usize;
            let dst_off: usize = ((dst_y * full_w + fx) * 4) as usize;
            let copy_w: u32 = fw.min(full_w.saturating_sub(fx));
            let bytes: usize = (copy_w * 4) as usize;
            if src_off + bytes <= frame_rgba.len() && dst_off + bytes <= full_rgba.len() {
                full_rgba[dst_off..dst_off + bytes]
                    .copy_from_slice(&frame_rgba[src_off..src_off + bytes]);
            }
        }
        rendered.push(RenderedFrame {
            frame_idx,
            rgba: full_rgba,
            width: full_w,
            height: full_h,
            offset_x: -(full_w as f32) / 2.0,
            offset_y: -(full_h as f32) / 2.0,
        });
    }
    if rendered.is_empty() {
        return None;
    }

    // Shelf-pack: sort tallest first, walk shelves left-to-right, double width
    // until everything fits below max_texture_dim.
    let mut indices: Vec<usize> = (0..rendered.len()).collect();
    indices.sort_by(|&a, &b| rendered[b].height.cmp(&rendered[a].height));

    let total_area: u64 = rendered
        .iter()
        .map(|s| (s.width as u64 + SPRITE_PADDING as u64) * (s.height as u64 + SPRITE_PADDING as u64))
        .sum();
    let estimated_side: u32 = (total_area as f64).sqrt().ceil() as u32;
    let max_texture_dim: u32 = gpu.device.limits().max_texture_dimension_2d;
    let mut atlas_width: u32 = estimated_side.clamp(64, max_texture_dim);

    let placements: Vec<(usize, u32, u32)>;
    let atlas_height: u32;
    loop {
        let mut trial: Vec<(usize, u32, u32)> = Vec::with_capacity(rendered.len());
        let mut cx: u32 = 0;
        let mut cy: u32 = 0;
        let mut shelf_h: u32 = 0;
        for &idx in &indices {
            let w: u32 = rendered[idx].width;
            let h: u32 = rendered[idx].height;
            if cx + w > atlas_width {
                cy += shelf_h + SPRITE_PADDING;
                cx = 0;
                shelf_h = 0;
            }
            trial.push((idx, cx, cy));
            cx += w + SPRITE_PADDING;
            shelf_h = shelf_h.max(h);
        }
        let trial_height: u32 = trial
            .iter()
            .map(|&(idx, _, py)| py + rendered[idx].height)
            .max()
            .unwrap_or(1);
        if trial_height <= max_texture_dim {
            placements = trial;
            atlas_height = trial_height;
            break;
        }
        if atlas_width >= max_texture_dim {
            placements = trial;
            atlas_height = trial_height.min(max_texture_dim);
            break;
        }
        atlas_width = atlas_width.saturating_mul(2).min(max_texture_dim);
    }

    let mut rgba: Vec<u8> = vec![0u8; (atlas_width * atlas_height * 4) as usize];
    let mut entries: HashMap<usize, FrameEntry> = HashMap::with_capacity(placements.len());
    let aw: f32 = atlas_width as f32;
    let ah: f32 = atlas_height as f32;
    for &(idx, px, py) in &placements {
        let spr: &RenderedFrame = &rendered[idx];
        let w: u32 = spr.width;
        let h: u32 = spr.height;
        for y in 0..h {
            let src_start: usize = (y * w * 4) as usize;
            let src_end: usize = src_start + (w * 4) as usize;
            let dst_start: usize = (((py + y) * atlas_width + px) * 4) as usize;
            let dst_end: usize = dst_start + (w * 4) as usize;
            if src_end <= spr.rgba.len() && dst_end <= rgba.len() {
                rgba[dst_start..dst_end].copy_from_slice(&spr.rgba[src_start..src_end]);
            }
        }
        entries.insert(
            spr.frame_idx,
            FrameEntry {
                uv_origin: [px as f32 / aw, py as f32 / ah],
                uv_size: [w as f32 / aw, h as f32 / ah],
                pixel_size: [w as f32, h as f32],
                offset_x: spr.offset_x,
                offset_y: spr.offset_y,
            },
        );
    }

    let texture: BatchTexture = batch.create_texture(gpu, &rgba, atlas_width, atlas_height);
    Some((texture, entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_table_returns_all_none_when_shp_frame_is_zero() {
        let table = build_table(&[(0, 0, 0); 10], &HashMap::new());
        for (slot, entry) in table.iter().enumerate() {
            assert!(entry.is_none(), "slot {slot} should be None for shp_frame=0");
        }
    }

    #[test]
    fn build_table_skips_slots_with_missing_frame_entries() {
        // Slot 0 references frame 3 (1-based), but frame_entries is empty —
        // build_table must yield None rather than panicking.
        let table = build_table(&[(3, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0),
                                  (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0)],
                                 &HashMap::new());
        assert!(table[0].is_none());
    }
}
