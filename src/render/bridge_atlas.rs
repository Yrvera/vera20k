//! Dedicated bridge-body atlas for the zdepth bridge pass.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::assets::shp_file::ShpFile;
use crate::map::overlay::OverlayEntry;
use crate::map::overlay_types::{OverlayTypeFlags, OverlayTypeRegistry};
use crate::render::batch::{BatchRenderer, BatchTexture};
use crate::render::gpu::GpuContext;
use crate::render::overlay_atlas::OverlaySpriteEntry;
use crate::rules::art_data::{self, ArtRegistry};
use crate::rules::ini_parser::IniFile;
use wgpu::util::DeviceExt;

const SPRITE_PADDING: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BridgeFrameKind {
    Body,
    Shadow,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BridgeAtlasKey {
    pub name: String,
    pub frame: u8,
    pub kind: BridgeFrameKind,
}

pub struct BridgeAtlas {
    pub texture: BatchTexture,
    pub depth_texture_view: wgpu::TextureView,
    pub zdepth_bind_group: wgpu::BindGroup,
    entries: HashMap<BridgeAtlasKey, OverlaySpriteEntry>,
}

impl BridgeAtlas {
    pub fn body_entry(&self, name: &str, frame: u8) -> Option<&OverlaySpriteEntry> {
        self.entries.get(&BridgeAtlasKey {
            name: name.to_string(),
            frame,
            kind: BridgeFrameKind::Body,
        })
    }

    pub fn shadow_entry(&self, name: &str, frame: u8) -> Option<&OverlaySpriteEntry> {
        self.entries.get(&BridgeAtlasKey {
            name: name.to_string(),
            frame,
            kind: BridgeFrameKind::Shadow,
        })
    }
}

/// Atlas lookup interface — abstracts the GPU-backed `BridgeAtlas` so that
/// instance builders can be exercised in unit tests with a pure-data mock.
pub trait BridgeAtlasLookup {
    fn body_entry(&self, name: &str, frame: u8) -> Option<&OverlaySpriteEntry>;
}

impl BridgeAtlasLookup for BridgeAtlas {
    fn body_entry(&self, name: &str, frame: u8) -> Option<&OverlaySpriteEntry> {
        BridgeAtlas::body_entry(self, name, frame)
    }
}

struct RenderedBridge {
    key: BridgeAtlasKey,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    offset_x: f32,
    offset_y: f32,
}

pub fn is_high_bridge_body_name(name: &str) -> bool {
    matches!(
        name.to_ascii_uppercase().as_str(),
        "BRIDGE1" | "BRIDGEB1" | "BRIDGE2" | "BRIDGEB2"
    )
}

pub fn build_bridge_atlas(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    overlays: &[OverlayEntry],
    overlay_names: &BTreeMap<u8, String>,
    asset_manager: &AssetManager,
    theater_palette: &Palette,
    unit_palette: &Palette,
    theater_ext: &str,
    theater_name: &str,
    overlay_registry: &OverlayTypeRegistry,
    rules_ini: &IniFile,
    art_registry: &ArtRegistry,
) -> Option<BridgeAtlas> {
    let mut needed: HashSet<BridgeAtlasKey> = HashSet::new();
    for entry in overlays {
        let Some(name) = overlay_names.get(&entry.overlay_id) else {
            continue;
        };
        if !is_high_bridge_body_name(name) {
            continue;
        }
        // Pack body half (frames 0..18) AND shadow half (frames 18..36) — RE doc §3.3.2.
        for frame in 0u8..18u8 {
            needed.insert(BridgeAtlasKey {
                name: name.clone(),
                frame,
                kind: BridgeFrameKind::Body,
            });
            needed.insert(BridgeAtlasKey {
                name: name.clone(),
                frame,
                kind: BridgeFrameKind::Shadow,
            });
        }
    }
    if needed.is_empty() {
        return None;
    }

    let mut rendered: Vec<RenderedBridge> = Vec::with_capacity(needed.len());
    for key in &needed {
        let flags: OverlayTypeFlags = overlay_registry
            .flags_by_name(&key.name)
            .cloned()
            .unwrap_or_default();
        let palette: &Palette = if flags.wall {
            unit_palette
        } else {
            theater_palette
        };
        if let Some(sprite) = render_bridge_sprite(
            asset_manager,
            palette,
            key,
            theater_ext,
            theater_name,
            rules_ini,
            art_registry,
            &flags,
        ) {
            rendered.push(sprite);
        }
    }
    if rendered.is_empty() {
        return None;
    }

    Some(pack_bridge_sprites(gpu, batch, &rendered))
}

/// Alpha approximating the native SHP shadow blitter's destination halve.
///
/// The blitter tests each source byte against zero and, where non-zero, halves
/// the destination pixel in place, so a shadow darkens whatever is already in
/// the framebuffer and overlapping shadows darken twice. Black texels at this
/// alpha under source-alpha blending give the same *shape* and the same
/// compositing behaviour.
///
/// VERA-internal; gamemd equivalence UNCHECKED and known to diverge. The
/// blitter halves the **stored, gamma-encoded** word, but VERA composites into
/// an sRGB target, so wgpu blends in **linear** space and the realized result is
/// markedly lighter than an encoded-space halve — see
/// `shadow_darken_is_lighter_than_the_native_encoded_halve`, which pins the
/// residual. No single alpha can close this: an encoded-space halve is not a
/// linear operation. Closing it needs this pass composited against a non-sRGB
/// target, which is a render-target change well outside the atlas.
const SHADOW_DARKEN_ALPHA: u8 = 128;

/// Convert an SHP shadow frame's 1-bit stencil into black RGBA with the darken
/// alpha baked in, so the instance builder needs no per-sprite alpha of its own.
///
/// Index 0 is the stencil's "no shadow here" value and stays fully transparent;
/// every other index is a shadow pixel. The blitter never looks at the actual
/// index value, so neither does this.
pub(crate) fn shadow_stencil_to_rgba(stencil: &[u8]) -> Vec<u8> {
    let mut rgba: Vec<u8> = Vec::with_capacity(stencil.len() * 4);
    for &index in stencil {
        let alpha: u8 = if index == 0 { 0 } else { SHADOW_DARKEN_ALPHA };
        rgba.extend_from_slice(&[0, 0, 0, alpha]);
    }
    rgba
}

fn render_bridge_sprite(
    asset_manager: &AssetManager,
    palette: &Palette,
    key: &BridgeAtlasKey,
    theater_ext: &str,
    theater_name: &str,
    rules_ini: &IniFile,
    art_registry: &ArtRegistry,
    flags: &OverlayTypeFlags,
) -> Option<RenderedBridge> {
    let image_id: String = art_registry.resolve_overlay_image_id(&key.name, rules_ini);
    let mut candidates: Vec<String> = art_data::overlay_shp_candidates(
        Some(art_registry),
        &key.name,
        &image_id,
        theater_ext,
        theater_name,
    );
    if let Some(alias) = decrement_numeric_suffix(&key.name) {
        candidates.push(format!("{}.{}", alias, theater_ext));
        candidates.push(format!("{}.shp", alias));
        candidates.push(format!("{}.{}", alias.to_ascii_lowercase(), theater_ext));
        candidates.push(format!("{}.shp", alias.to_ascii_lowercase()));
    }

    let shp: ShpFile = candidates.iter().find_map(|name| {
        let data = asset_manager.get_ref(name)?;
        let shp = ShpFile::from_bytes(data).ok()?;
        let has_drawable = shp
            .frames
            .iter()
            .any(|fr| fr.frame_width > 0 && fr.frame_height > 0);
        has_drawable.then_some(shp)
    })?;

    // Bridge SHPs split frames into a body half (front) and shadow half (back).
    // RE doc §3.3.2: shadow_frame_idx = (shp.frames.len() / 2) + state_byte.
    let half: usize = if flags.bridge_deck {
        shp.frames.len() / 2
    } else {
        shp.frames.len()
    };
    let (window_start, window_len): (usize, usize) = match key.kind {
        BridgeFrameKind::Body => (0, half),
        BridgeFrameKind::Shadow => (half, shp.frames.len().saturating_sub(half)),
    };
    let requested_idx: usize =
        window_start + (key.frame as usize).min(window_len.saturating_sub(1));
    let mut frame_idx = requested_idx;
    if !shp
        .frames
        .get(frame_idx)
        .is_some_and(|fr| fr.frame_width > 0 && fr.frame_height > 0)
    {
        frame_idx = shp
            .frames
            .iter()
            .skip(window_start)
            .take(window_len)
            .enumerate()
            .find(|(_, fr)| fr.frame_width > 0 && fr.frame_height > 0)
            .map(|(idx, _)| window_start + idx)?;
    }

    let frame = &shp.frames[frame_idx];
    // Shadow frames are a 1-bit stencil, never colour data — sending them
    // through a theater palette is what made the earlier bridge-shadow attempt
    // paint solid cyan and forced the draw call to be disabled.
    let frame_rgba: Vec<u8> = match key.kind {
        BridgeFrameKind::Body => shp.frame_to_rgba(frame_idx, palette).ok()?,
        BridgeFrameKind::Shadow => shadow_stencil_to_rgba(&frame.pixels),
    };
    let full_w: u32 = shp.width as u32;
    let full_h: u32 = shp.height as u32;
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

    Some(RenderedBridge {
        key: key.clone(),
        rgba: full_rgba,
        width: full_w,
        height: full_h,
        offset_x: -(full_w as f32) / 2.0,
        offset_y: -(full_h as f32) / 2.0 + flags.y_draw_offset(),
    })
}

fn pack_bridge_sprites(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    sprites: &[RenderedBridge],
) -> BridgeAtlas {
    let mut indices: Vec<usize> = (0..sprites.len()).collect();
    indices.sort_by(|&a, &b| sprites[b].height.cmp(&sprites[a].height));

    let total_area: u64 = sprites
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
        let mut trial: Vec<(usize, u32, u32)> = Vec::with_capacity(sprites.len());
        let mut cx: u32 = 0;
        let mut cy: u32 = 0;
        let mut shelf_h: u32 = 0;
        for &idx in &indices {
            let w: u32 = sprites[idx].width;
            let h: u32 = sprites[idx].height;
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
            .map(|&(idx, _, py)| py + sprites[idx].height)
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
        atlas_width = (atlas_width.saturating_mul(2)).min(max_texture_dim);
    }

    let mut rgba: Vec<u8> = vec![0u8; (atlas_width * atlas_height * 4) as usize];
    let mut depth: Vec<u8> = vec![BRIDGE_DEPTH_NEUTRAL; (atlas_width * atlas_height) as usize];
    let mut entries: HashMap<BridgeAtlasKey, OverlaySpriteEntry> =
        HashMap::with_capacity(placements.len());
    let aw: f32 = atlas_width as f32;
    let ah: f32 = atlas_height as f32;

    for &(idx, px, py) in &placements {
        let spr: &RenderedBridge = &sprites[idx];
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
        write_bridge_depth_rows(&mut depth, atlas_width, atlas_height, spr, px, py);
        entries.insert(
            spr.key.clone(),
            OverlaySpriteEntry {
                uv_origin: [px as f32 / aw, py as f32 / ah],
                uv_size: [w as f32 / aw, h as f32 / ah],
                pixel_size: [w as f32, h as f32],
                offset_x: spr.offset_x,
                offset_y: spr.offset_y,
            },
        );
    }

    let texture: BatchTexture = batch.create_texture(gpu, &rgba, atlas_width, atlas_height);
    let depth_texture_view = create_r8_texture(gpu, &depth, atlas_width, atlas_height);
    let zdepth_bind_group = batch.create_zdepth_bind_group(gpu, &texture.view, &depth_texture_view);

    BridgeAtlas {
        texture,
        depth_texture_view,
        zdepth_bind_group,
        entries,
    }
}

/// R8 texel meaning that this atlas pixel contributes no bridge-body row offset.
///
/// The zdepth fragment shader computes `base_depth - z_sample * scale`, so a
/// zero texel leaves the instance's own sort depth untouched.
const BRIDGE_DEPTH_NEUTRAL: u8 = 0;

/// Copy one bridge entry's full-canvas native row gradient into the shared atlas.
///
/// `CellClass::DrawOverlay_Body @ 0x0047F6A0` selects the active extended SHP
/// blitter for high-bridge format-3 bodies. Its `0x004990E0` leaf decrements the
/// candidate by one native Z unit per full-canvas source/destination scanline.
/// Body row `y` therefore stores `y`; shadow entries and atlas padding retain
/// neutral zero. Retail BRIDGE/BRIDGB canvases top out at 242 rows and fit R8.
fn write_bridge_depth_rows(
    depth: &mut [u8],
    atlas_width: u32,
    atlas_height: u32,
    sprite: &RenderedBridge,
    px: u32,
    py: u32,
) {
    if sprite.key.kind != BridgeFrameKind::Body {
        return;
    }

    let copy_width = sprite.width.min(atlas_width.saturating_sub(px));
    let copy_height = sprite.height.min(atlas_height.saturating_sub(py));
    for y in 0..copy_height {
        let row_depth = y.min(u32::from(u8::MAX)) as u8;
        let dst_start = ((py + y) * atlas_width + px) as usize;
        let dst_end = dst_start + copy_width as usize;
        if let Some(row) = depth.get_mut(dst_start..dst_end) {
            row.fill(row_depth);
        }
    }
}

fn create_r8_texture(gpu: &GpuContext, data: &[u8], width: u32, height: u32) -> wgpu::TextureView {
    let texture = gpu.device.create_texture_with_data(
        &gpu.queue,
        &wgpu::TextureDescriptor {
            label: Some("Bridge Depth Atlas R8"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        data,
    );
    texture.create_view(&Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_and_shadow_keys_are_distinct() {
        let body = BridgeAtlasKey {
            name: "BRIDGE1".into(),
            frame: 5,
            kind: BridgeFrameKind::Body,
        };
        let shadow = BridgeAtlasKey {
            name: "BRIDGE1".into(),
            frame: 5,
            kind: BridgeFrameKind::Shadow,
        };
        assert_ne!(body, shadow);
    }

    #[test]
    fn gsi_13_09_body_depth_plane_encodes_local_rows_and_keeps_shadow_padding_zero() {
        fn rendered(kind: BridgeFrameKind, width: u32, height: u32) -> RenderedBridge {
            RenderedBridge {
                key: BridgeAtlasKey {
                    name: "BRIDGE1".into(),
                    frame: 0,
                    kind,
                },
                // An all-transparent canvas still receives the full row plane;
                // the shader's color discard decides which pixels participate.
                rgba: vec![0; (width * height * 4) as usize],
                width,
                height,
                offset_x: 0.0,
                offset_y: 0.0,
            }
        }

        let atlas_width = 9;
        let atlas_height = 5;
        let mut plane = vec![BRIDGE_DEPTH_NEUTRAL; (atlas_width * atlas_height) as usize];
        let body = rendered(BridgeFrameKind::Body, 3, 3);
        let shadow = rendered(BridgeFrameKind::Shadow, 2, 3);

        write_bridge_depth_rows(&mut plane, atlas_width, atlas_height, &body, 1, 1);
        write_bridge_depth_rows(&mut plane, atlas_width, atlas_height, &shadow, 6, 1);

        for (atlas_y, expected) in [(1u32, 0u8), (2, 1), (3, 2)] {
            let start = (atlas_y * atlas_width + 1) as usize;
            assert_eq!(&plane[start..start + 3], &[expected; 3]);
        }
        for atlas_y in 1u32..=3 {
            let shadow_start = (atlas_y * atlas_width + 6) as usize;
            assert_eq!(&plane[shadow_start..shadow_start + 2], &[0; 2]);
            assert_eq!(plane[(atlas_y * atlas_width) as usize], 0);
            assert_eq!(plane[(atlas_y * atlas_width + 4) as usize], 0);
        }
    }

    #[test]
    fn shadow_frames_become_a_black_stencil_not_palette_colour() {
        // A shadow frame carries only 0 (nothing) and non-zero (shadow). The
        // native blitter TESTs the byte and halves the destination; it never
        // reads a colour, so any palette lookup here is wrong by construction.
        let rgba = shadow_stencil_to_rgba(&[0, 1, 0, 4, 255]);
        assert_eq!(rgba.len(), 5 * 4);
        // Index 0 → fully transparent, destination untouched.
        assert_eq!(&rgba[0..4], &[0, 0, 0, 0]);
        assert_eq!(&rgba[8..12], &[0, 0, 0, 0]);
        // Every non-zero index → the same black darken texel, whatever its value.
        for offset in [4usize, 12, 16] {
            assert_eq!(
                &rgba[offset..offset + 4],
                &[0, 0, 0, SHADOW_DARKEN_ALPHA],
                "non-zero stencil byte at {offset} must darken, not tint"
            );
        }
    }

    #[test]
    fn shadow_darken_is_lighter_than_the_native_encoded_halve() {
        // Recorded DRIFT, not a parity assertion. The blitter halves the
        // stored gamma-encoded word: an encoded 0.5 destination becomes an
        // encoded 0.25. VERA composites into an sRGB target, so the blend runs
        // in linear space and lands much lighter. This test pins the size of
        // that gap so it stays visible instead of decaying into folklore.
        fn srgb_to_linear(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        fn linear_to_srgb(c: f32) -> f32 {
            if c <= 0.0031308 {
                12.92 * c
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        }

        let dst_encoded: f32 = 0.5;
        let src_alpha: f32 = f32::from(SHADOW_DARKEN_ALPHA) / 255.0;
        // out = a*src + (1-a)*dst, in linear space, with src black.
        let realized: f32 = linear_to_srgb((1.0 - src_alpha) * srgb_to_linear(dst_encoded));
        let native: f32 = dst_encoded / 2.0;

        assert!(
            realized > native,
            "the linear-space blend must come out lighter than the encoded halve \
             (realized {realized}, native {native})"
        );
        assert!(
            (realized - 0.360).abs() < 0.01,
            "realized darken drifted from the recorded value; \
             expected ~0.360 encoded, got {realized}"
        );
        // Roughly 44% too light. Recorded so a future render-target change can
        // be measured against it rather than eyeballed.
        assert!(
            ((realized - native) / native - 0.44).abs() < 0.05,
            "recorded shadow lightness drift is ~44% of the native value, got {}",
            (realized - native) / native
        );
    }
}

fn decrement_numeric_suffix(name: &str) -> Option<String> {
    let split: usize = name.rfind(|c: char| !c.is_ascii_digit())?;
    if split + 1 >= name.len() {
        return None;
    }
    let (prefix, digits) = name.split_at(split + 1);
    let width: usize = digits.len();
    let n: u32 = digits.parse().ok()?;
    if n == 0 {
        return None;
    }
    Some(format!("{}{:0width$}", prefix, n - 1, width = width))
}
