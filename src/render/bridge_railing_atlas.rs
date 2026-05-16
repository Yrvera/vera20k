//! Bridge railing atlas — RAILBRDG.tem SHP packed into a single atlas page,
//! plus concrete + wood lookup tables that map a bridge table slot to a
//! sub-tile-gated railing frame entry.
//!
//! This renderer path still loads RAILBRDG, but its 10-entry table now uses the
//! recovered DAT_00ABC210 values from
//! `ra2-rust-game-docs/BRIDGE_THEATER_LOAD_TABLE_WRITERS_GHIDRA_REPORT.md`.
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
    pub required_sub_tile: u8,
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
    tile_bases: Option<BridgeRailingTileBases>,
    concrete_table: [Option<RailingEntry>; 10],
    wood_table: [Option<RailingEntry>; 10],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeRailingTileBases {
    pub slope_set_pieces_start: u16,
    pub slope_set_pieces2_start: u16,
}

impl BridgeRailingTileBases {
    fn table_slot_for_tile_index(self, tile_index: i32) -> Option<u8> {
        let tile_index = u16::try_from(tile_index).ok()?;
        table_slot_in_range(tile_index, self.slope_set_pieces_start)
            .or_else(|| table_slot_in_range(tile_index, self.slope_set_pieces2_start))
    }
}

impl BridgeRailingAtlas {
    pub fn entry(
        &self,
        kind: BridgeKind,
        table_slot: u8,
        caller_sub_tile: u8,
    ) -> Option<&RailingEntry> {
        let table = match kind {
            BridgeKind::Concrete => &self.concrete_table,
            BridgeKind::Wood => &self.wood_table,
        };
        entry_from_table(table, table_slot, caller_sub_tile)
    }

    pub fn entry_for_tile(
        &self,
        kind: BridgeKind,
        tile_index: i32,
        caller_sub_tile: u8,
    ) -> Option<&RailingEntry> {
        let table = match kind {
            BridgeKind::Concrete => &self.concrete_table,
            BridgeKind::Wood => &self.wood_table,
        };
        entry_for_tile_from_table(table, self.tile_bases?, tile_index, caller_sub_tile)
    }
}

fn table_slot_in_range(tile_index: u16, range_start: u16) -> Option<u8> {
    let local = u32::from(tile_index).checked_sub(u32::from(range_start))?;
    (local < 10).then_some(local as u8)
}

fn entry_from_table(
    table: &[Option<RailingEntry>; 10],
    table_slot: u8,
    caller_sub_tile: u8,
) -> Option<&RailingEntry> {
    table
        .get(table_slot as usize)
        .and_then(Option::as_ref)
        .filter(|entry| entry.required_sub_tile == caller_sub_tile)
}

fn entry_for_tile_from_table(
    table: &[Option<RailingEntry>; 10],
    tile_bases: BridgeRailingTileBases,
    tile_index: i32,
    caller_sub_tile: u8,
) -> Option<&RailingEntry> {
    let table_slot = tile_bases.table_slot_for_tile_index(tile_index)?;
    entry_from_table(table, table_slot, caller_sub_tile)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RailingTableValue {
    shp_frame_1based: u8,
    required_sub_tile: u8,
    dx: i16,
    dy: i16,
}

const SKIP_RAILING: RailingTableValue = RailingTableValue {
    shp_frame_1based: 0,
    required_sub_tile: 0,
    dx: 0,
    dy: 0,
};

// Verified DAT_00ABC210 values. FUN_00547230 uses this same 10-entry table for
// both DAT_00ABC1F8 and DAT_00AA1098 ranges.
const DAT_00ABC210_RAILING_VALUES: [RailingTableValue; 10] = [
    SKIP_RAILING,
    SKIP_RAILING,
    SKIP_RAILING,
    SKIP_RAILING,
    RailingTableValue {
        shp_frame_1based: 13,
        required_sub_tile: 6,
        dx: 48,
        dy: 12,
    },
    SKIP_RAILING,
    RailingTableValue {
        shp_frame_1based: 14,
        required_sub_tile: 1,
        dx: 48,
        dy: 12,
    },
    SKIP_RAILING,
    SKIP_RAILING,
    SKIP_RAILING,
];

const CONCRETE_RAILING_VALUES: [RailingTableValue; 10] = DAT_00ABC210_RAILING_VALUES;

const WOOD_RAILING_VALUES: [RailingTableValue; 10] = DAT_00ABC210_RAILING_VALUES;

pub fn build_bridge_railing_atlas(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    asset_manager: &AssetManager,
    theater_palette: &Palette,
    theater_ext: &str,
    tile_bases: Option<BridgeRailingTileBases>,
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
        tile_bases,
        concrete_table,
        wood_table,
    })
}

fn build_table(
    values: &[RailingTableValue; 10],
    frame_entries: &HashMap<usize, FrameEntry>,
) -> [Option<RailingEntry>; 10] {
    let mut out: [Option<RailingEntry>; 10] = [None; 10];
    for (slot, value) in values.iter().enumerate() {
        if value.shp_frame_1based == 0 {
            continue;
        }
        let frame_0based = (value.shp_frame_1based - 1) as usize;
        let Some(e) = frame_entries.get(&frame_0based) else {
            continue;
        };
        out[slot] = Some(RailingEntry {
            shp_frame: value.shp_frame_1based,
            required_sub_tile: value.required_sub_tile,
            dx: value.dx,
            dy: value.dy,
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
        .map(|s| {
            (s.width as u64 + SPRITE_PADDING as u64) * (s.height as u64 + SPRITE_PADDING as u64)
        })
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

    fn test_frame_entries() -> HashMap<usize, FrameEntry> {
        HashMap::from([
            (
                12,
                FrameEntry {
                    uv_origin: [0.0, 0.0],
                    uv_size: [1.0, 1.0],
                    pixel_size: [60.0, 30.0],
                    offset_x: -30.0,
                    offset_y: -15.0,
                },
            ),
            (
                13,
                FrameEntry {
                    uv_origin: [0.0, 0.0],
                    uv_size: [1.0, 1.0],
                    pixel_size: [60.0, 30.0],
                    offset_x: -30.0,
                    offset_y: -15.0,
                },
            ),
        ])
    }

    #[test]
    fn build_table_returns_all_none_when_shp_frame_is_zero() {
        let table = build_table(&[SKIP_RAILING; 10], &HashMap::new());
        for (slot, entry) in table.iter().enumerate() {
            assert!(
                entry.is_none(),
                "slot {slot} should be None for shp_frame=0"
            );
        }
    }

    #[test]
    fn build_table_skips_slots_with_missing_frame_entries() {
        // Slot 0 references frame 3 (1-based), but frame_entries is empty —
        // build_table must yield None rather than panicking.
        let mut values = [SKIP_RAILING; 10];
        values[0] = RailingTableValue {
            shp_frame_1based: 3,
            required_sub_tile: 0,
            dx: 0,
            dy: 0,
        };
        let table = build_table(&values, &HashMap::new());
        assert!(table[0].is_none());
    }

    #[test]
    fn dat_00abc210_slots_gate_on_required_sub_tile() {
        let frames = test_frame_entries();
        let table = build_table(&DAT_00ABC210_RAILING_VALUES, &frames);

        let slot_4 = entry_from_table(&table, 4, 6).expect("slot 4 should match sub-tile 6");
        assert_eq!(slot_4.shp_frame, 13);
        assert_eq!(slot_4.required_sub_tile, 6);
        assert_eq!((slot_4.dx, slot_4.dy), (48, 12));
        assert!(entry_from_table(&table, 4, 1).is_none());

        let slot_6 = entry_from_table(&table, 6, 1).expect("slot 6 should match sub-tile 1");
        assert_eq!(slot_6.shp_frame, 14);
        assert_eq!(slot_6.required_sub_tile, 1);
        assert_eq!((slot_6.dx, slot_6.dy), (48, 12));
        assert!(entry_from_table(&table, 6, 6).is_none());

        assert!(entry_from_table(&table, 0, 0).is_none());
        assert!(entry_from_table(&table, 5, 0).is_none());
    }

    #[test]
    fn tile_index_lookup_derives_slot_before_sub_tile_gate() {
        let frames = test_frame_entries();
        let table = build_table(&DAT_00ABC210_RAILING_VALUES, &frames);
        let tile_bases = BridgeRailingTileBases {
            slope_set_pieces_start: 100,
            slope_set_pieces2_start: 200,
        };

        let slot_4 =
            entry_for_tile_from_table(&table, tile_bases, 104, 6).expect("slot 4/sub-tile 6");
        assert_eq!(slot_4.shp_frame, 13);
        assert!(entry_for_tile_from_table(&table, tile_bases, 104, 4).is_none());

        let slot_6 =
            entry_for_tile_from_table(&table, tile_bases, 206, 1).expect("slot 6/sub-tile 1");
        assert_eq!(slot_6.shp_frame, 14);
        assert!(entry_for_tile_from_table(&table, tile_bases, 206, 6).is_none());

        assert!(entry_for_tile_from_table(&table, tile_bases, 99, 6).is_none());
        assert!(entry_for_tile_from_table(&table, tile_bases, 210, 1).is_none());
    }
}
