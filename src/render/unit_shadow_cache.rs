//! Retained shadow stencil payloads within the existing unit atlas pages.
//!
//! Native 706BD0/707280 retains the first composed-body mask for a cache key.
//! This bounded owner keeps that history across frames and atlas growth. It
//! does not emulate the original shared 4 MB arena's global purge policy.

use super::{UnitAtlas, UnitSpriteEntry, UnitSpriteKey};
use std::collections::HashMap;

impl UnitAtlas {
    /// Fill only on the first supported request. Parts are the exact atlas
    /// entries and anchor offsets used by this Unit's visible body draw.
    /// Hits perform no body composition, pixel filtering, or texture upload.
    pub(crate) fn prepare_native_shadow(
        &self,
        queue: &wgpu::Queue,
        key: &UnitSpriteKey,
        parts: impl IntoIterator<Item = (UnitSpriteEntry, [f32; 2])>,
    ) -> bool {
        if self.shadow_masks.borrow().contains_key(key) {
            return true;
        }
        let Some(entry) = self
            .entries
            .get(key)
            .filter(|e| e.native_draw_bounds.is_some())
        else {
            return false;
        };
        let Some(template) = self.rendered_cache.iter().find(|s| &s.key == key) else {
            return false;
        };
        let mut parts = parts.into_iter().peekable();
        if parts.peek().is_none() {
            return false;
        }
        let mut body = vec![0u8; 65536];
        for (part, offset) in parts {
            let Some(source) = self.rendered_cache.iter().find(|s| {
                self.entries.get(&s.key).is_some_and(|e| {
                    e.page == part.page
                        && e.uv_origin == part.uv_origin
                        && e.uv_size == part.uv_size
                })
            }) else {
                return false;
            };
            let x = 128 + source.offset_x as i32 + offset[0] as i32;
            let y = 128 + source.offset_y as i32 + offset[1] as i32;
            composite_mask_part(
                &mut body,
                &source.pixels,
                source.width,
                source.height,
                [x, y],
            );
        }
        let mut pixels = template.pixels.clone();
        if super::vxl_raster::shadow::mask_body(
            &mut pixels,
            template.width,
            [template.offset_x as i32, template.offset_y as i32],
            &body,
        )
        .is_none()
        {
            return false;
        }
        self.upload_shadow_pixels(queue, entry, &pixels);
        self.shadow_masks.borrow_mut().insert(key.clone(), pixels);
        true
    }

    fn upload_shadow_pixels(&self, queue: &wgpu::Queue, entry: &UnitSpriteEntry, pixels: &[u8]) {
        let page = &self.pages[entry.page].texture;
        let width = entry.pixel_size[0] as u32;
        let height = entry.pixel_size[1] as u32;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: page.view.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: (entry.uv_origin[0] * page.width as f32).round() as u32,
                    y: (entry.uv_origin[1] * page.height as f32).round() as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub(super) fn restore_shadow_masks(
        &self,
        queue: &wgpu::Queue,
        masks: HashMap<UnitSpriteKey, Vec<u8>>,
    ) {
        let mut retained = self.shadow_masks.borrow_mut();
        for (key, pixels) in masks {
            let Some(entry) = self.entries.get(&key) else {
                continue;
            };
            if pixels.len() != (entry.pixel_size[0] * entry.pixel_size[1]) as usize {
                continue;
            }
            self.upload_shadow_pixels(queue, entry, &pixels);
            retained.insert(key, pixels);
        }
    }
}

/// 4914C0 inner indexed copy skips zero source; later parts overwrite only
/// nonzero pixels. For the shadow query the composed nonzero domain suffices.
fn composite_mask_part(body: &mut [u8], source: &[u8], width: u32, height: u32, point: [i32; 2]) {
    for y in 0..height {
        let dy = point[1] + y as i32;
        if !(0..256).contains(&dy) {
            continue;
        }
        for x in 0..width {
            let dx = point[0] + x as i32;
            if !(0..256).contains(&dx) {
                continue;
            }
            if source[(y * width + x) as usize] != 0 {
                body[(dy * 256 + dx) as usize] = 1;
            }
        }
    }
}

#[cfg(test)]
#[path = "unit_shadow_tests.rs"]
mod tests;
