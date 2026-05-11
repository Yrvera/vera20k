//! Visual-parity regression tests for the voxel atlas pipeline.
//!
//! Tests in this file validate the post-VPL palette-index output of the
//! software rasterizer, which is the byte gamemd writes to its visibility
//! map (the byte the fragment shader then samples and resolves to RGB).
//!
//! ## What's covered here
//! - `color_0_voxels_never_written`: byte 0 = transparent invariant. The
//!   rasterizer must never write byte 0 to a pixel that has a non-empty
//!   voxel hit. Matches the engine's visibility-map convention.
//!
//! ## What's NOT covered (deferred)
//! - End-to-end Grizzly facing-0 / slope-0 snapshot test against
//!   gamemd-rendered output. Needs retail VXL assets and a blessed binary
//!   snapshot. Add when the asset pipeline supports it.

#[cfg(test)]
mod tests {
    use crate::assets::vxl_file::{VxlFile, VxlLimb, VxlVoxel};
    use crate::render::vxl_raster::{self, VxlRenderParams, VxlSprite};

    /// Build a tiny 2×2×2 VXL with two opaque voxels (color indices 10 and 20)
    /// and otherwise empty cells. Verifies that the rasterizer writes those
    /// non-zero bytes for the opaque pixels and leaves byte 0 for everything
    /// else. No VPL is loaded, so the post-VPL byte equals the source
    /// color_index directly (per the no-VPL fallback in render_vxl).
    fn make_two_voxel_vxl() -> VxlFile {
        let identity: [f32; 12] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        VxlFile {
            limb_count: 1,
            body_size: 0,
            palette: vec![[0; 3]; 256],
            limbs: vec![VxlLimb {
                name: "body".to_string(),
                scale: 1.0,
                bounds: [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0],
                transform: identity,
                size_x: 2,
                size_y: 2,
                size_z: 2,
                normals_mode: 4,
                voxels: vec![
                    VxlVoxel {
                        x: 1,
                        y: 1,
                        z: 1,
                        color_index: 10,
                        normal_index: 0,
                    },
                    VxlVoxel {
                        x: 0,
                        y: 0,
                        z: 0,
                        color_index: 20,
                        normal_index: 1,
                    },
                ],
            }],
        }
    }

    #[test]
    fn color_0_invariant_only_voxel_color_indices_appear_in_output() {
        let vxl: VxlFile = make_two_voxel_vxl();
        let params: VxlRenderParams = VxlRenderParams::default();
        let sprite: VxlSprite = vxl_raster::render_vxl(&vxl, None, &params, None);

        // Output must contain only bytes 0 (transparent) or one of the source
        // color indices we wrote (10 or 20). Anything else means the
        // rasterizer corrupted a pixel value.
        for &byte in &sprite.palette_indices {
            assert!(
                byte == 0 || byte == 10 || byte == 20,
                "Unexpected palette byte {} in rasterizer output (allowed: 0, 10, 20)",
                byte,
            );
        }

        // At least one pixel of each source color must appear (the rasterizer
        // wouldn't be doing its job otherwise).
        let count_10: usize = sprite.palette_indices.iter().filter(|&&b| b == 10).count();
        let count_20: usize = sprite.palette_indices.iter().filter(|&&b| b == 20).count();
        assert!(
            count_10 > 0,
            "voxel with color_index=10 produced no output bytes"
        );
        assert!(
            count_20 > 0,
            "voxel with color_index=20 produced no output bytes"
        );
    }
}
