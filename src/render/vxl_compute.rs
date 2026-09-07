//! GPU compute VXL renderer — replaces the CPU software rasterizer.
//!
//! Uses two compute shader passes to render VXL models:
//! 1. **Splat pass**: one thread per voxel, projects to screen space via
//!    fixed-point truncation, packs (depth | vpl_page | color_index) into u32,
//!    writes via atomicMin to the atomic framebuffer.
//! 2. **Resolve pass**: one thread per pixel, unpacks, applies VPL remap +
//!    palette RGBA lookup, writes final RGBA to output buffer.
//!
//! The renderer is an offline batch tool used during atlas builds, not per-frame.
//! After rendering, the RGBA output is read back to CPU for shelf-packing into
//! the UnitAtlas texture.
//!
//! ## Dependency rules
//! - Part of render/ — depends on assets/ (VplFile, VxlFile),
//!   render/vxl_raster (LimbRenderData, SpriteBounds), render/vxl_normals.

use bytemuck::{Pod, Zeroable};

use crate::assets::vpl_file::VplFile;
use crate::render::vxl_raster::SpriteBounds;

// ---------------------------------------------------------------------------
// Input data for one limb — prepared by the caller from VxlFile + LimbRenderData.
// ---------------------------------------------------------------------------

/// Pre-packed voxel data for one limb, ready for GPU upload.
pub struct GpuLimb {
    /// Packed voxel positions: x | (y << 8) | (z << 16).
    pub positions: Vec<u32>,
    /// Packed voxel data: color_index | (normal_index << 8).
    pub data: Vec<u32>,
    /// Normal index → VPL page mapping (256 entries).
    pub vpl_pages: [u8; 256],
    /// Combined world+section transform matrix.
    pub combined: glam::Mat4,
}

// ---------------------------------------------------------------------------
// GPU-side uniform structs — must match WGSL layout exactly.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SplatParams {
    mat_col0: [f32; 4],
    mat_col1: [f32; 4],
    mat_col2: [f32; 4],
    mat_col3: [f32; 4],
    scale: f32,
    fp_scale: f32,
    fb_width: u32,
    fb_height: u32,
    buf_off_x_fp: i32,
    buf_off_y_fp: i32,
    fill_size: i32,
    half_fill: i32,
    voxel_count: u32,
    /// Smallest screen-space depth any voxel of this draw can reach.
    depth_min: f32,
    /// `65535 / (depth_max - depth_min)` for this draw.
    depth_scale: f32,
    _pad2: u32,
}

/// The screen-space depth window one draw's voxels can occupy, from the eight
/// corners of every limb's occupied voxel box pushed through that limb's
/// `combined` matrix. Every limb of a sprite shares one atomic framebuffer, so
/// the window is per draw, not per limb.
///
/// VERA-internal, gamemd equivalent: none. The original has no per-pixel depth
/// at all: `VXL_Sort_Rasterize` (0x00754510) bubble-sorts sections by centre
/// depth and each `g_VXL_RasterizerTable` variant (e.g. 0x007DF7C0) paints
/// straight into `g_VXL_VisibilityMap` in an axis order chosen by the facing
/// signs, so nearer voxels simply land last. This GPU path keeps a 16-bit
/// depth test instead, and that test used to clamp to a fixed -50..+50 model
/// units. A long hull tilted by the camera runs past that window; every
/// clamped voxel then ties on depth and `atomicMin` breaks the tie on the
/// packed page and colour bytes, so the darkest hidden voxel won and the
/// Amphibious Transport's stern and bow drew as flat dark shapes. Deriving the
/// window from the actual bounds keeps every voxel strictly ordered.
/// Intra-section painter's order vs. depth test remains a recorded DRIFT for
/// coplanar/overlapping voxels within one section.
pub(crate) fn draw_depth_window(limbs: &[GpuLimb]) -> (f32, f32) {
    let mut lo: f32 = f32::INFINITY;
    let mut hi: f32 = f32::NEG_INFINITY;
    for gl in limbs {
        if gl.positions.is_empty() {
            continue;
        }
        let (mut min, mut max) = ([u32::MAX; 3], [0u32; 3]);
        for &p in &gl.positions {
            let c = [p & 0xFF, (p >> 8) & 0xFF, (p >> 16) & 0xFF];
            for a in 0..3 {
                min[a] = min[a].min(c[a]);
                max[a] = max[a].max(c[a]);
            }
        }
        for corner in 0..8u32 {
            let pick = |axis: usize| {
                if corner & (1 << axis) == 0 {
                    min[axis]
                } else {
                    max[axis]
                }
            } as f32;
            let world = gl
                .combined
                .transform_point3(glam::Vec3::new(pick(0), pick(1), pick(2)));
            lo = lo.min(world.z);
            hi = hi.max(world.z);
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        return (-1.0, 1.0);
    }
    // One unit of slack on each side so the extreme voxels never saturate.
    (lo - 1.0, hi + 1.0)
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ResolveParams {
    fb_width: u32,
    fb_height: u32,
    pixel_count: u32,
    vpl_section_count: u32,
}

// ---------------------------------------------------------------------------
// VxlComputeRenderer — owns pipelines, bind group layouts, reusable buffers.
// ---------------------------------------------------------------------------

/// GPU compute renderer for VXL voxel models.
///
/// Created once at app init. Reused across atlas builds.
pub struct VxlComputeRenderer {
    splat_pipeline: wgpu::ComputePipeline,
    resolve_pipeline: wgpu::ComputePipeline,
    splat_bgl: wgpu::BindGroupLayout,
    resolve_bgl: wgpu::BindGroupLayout,
    // Reusable buffers (grown on demand, never shrink).
    atomic_fb: wgpu::Buffer,
    /// Output buffer: one u32 per pixel, low byte = post-VPL palette index.
    /// House remap and theater palette lookup happen at fragment-shader time.
    output_palette_indices: wgpu::Buffer,
    staging: wgpu::Buffer,
    fb_capacity: u32, // current max pixel count
    // Persistent lookup tables (uploaded once per atlas build).
    vpl_buffer: Option<wgpu::Buffer>,
    vpl_section_count: u32,
}

impl VxlComputeRenderer {
    /// Create compute pipelines and allocate initial buffers.
    pub fn new(device: &wgpu::Device) -> Self {
        let splat_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vxl_splat.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("vxl_splat.wgsl").into()),
        });
        let resolve_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vxl_resolve.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("vxl_resolve.wgsl").into()),
        });

        // Splat bind group layout.
        let splat_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vxl_splat_bgl"),
            entries: &[
                bgl_uniform(0),    // SplatParams
                bgl_storage_ro(1), // voxel_positions
                bgl_storage_ro(2), // voxel_data
                bgl_storage_ro(3), // vpl_pages
                bgl_storage_rw(4), // atomic_fb
            ],
        });

        // Resolve bind group layout.
        let resolve_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vxl_resolve_bgl"),
            entries: &[
                bgl_uniform(0),    // ResolveParams
                bgl_storage_rw(1), // atomic_fb
                bgl_storage_ro(2), // vpl_table
                bgl_storage_rw(3), // output_palette_indices
            ],
        });

        let splat_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vxl_splat_pl"),
            bind_group_layouts: &[&splat_bgl],
            push_constant_ranges: &[],
        });
        let resolve_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vxl_resolve_pl"),
            bind_group_layouts: &[&resolve_bgl],
            push_constant_ranges: &[],
        });

        let splat_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("vxl_splat_pipeline"),
            layout: Some(&splat_pl),
            module: &splat_shader,
            entry_point: Some("splat_main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let resolve_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("vxl_resolve_pipeline"),
            layout: Some(&resolve_pl),
            module: &resolve_shader,
            entry_point: Some("resolve_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Initial buffer capacity for a 128×128 sprite (16384 pixels).
        let init_cap: u32 = 128 * 128;
        let fb_byte_size = (init_cap as u64) * 4;

        let atomic_fb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vxl_atomic_fb"),
            size: fb_byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let output_palette_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vxl_output_palette_indices"),
            size: fb_byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vxl_staging"),
            size: fb_byte_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            splat_pipeline,
            resolve_pipeline,
            splat_bgl,
            resolve_bgl,
            atomic_fb,
            output_palette_indices,
            staging,
            fb_capacity: init_cap,
            vpl_buffer: None,
            vpl_section_count: 0,
        }
    }

    /// Upload VPL lookup table to GPU. Call once per atlas build.
    pub fn upload_vpl(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vpl: &VplFile) {
        // Flatten pages into a byte array, then pack 4 bytes per u32.
        let total_bytes = vpl.num_sections as usize * 256;
        let mut flat = vec![0u8; total_bytes];
        for (page_idx, page) in vpl.pages_slice().iter().enumerate() {
            let offset = page_idx * 256;
            flat[offset..offset + 256].copy_from_slice(page);
        }
        // Pad to multiple of 4.
        while flat.len() % 4 != 0 {
            flat.push(0);
        }
        let packed: Vec<u32> = flat
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let byte_data = bytemuck::cast_slice(&packed);

        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vxl_vpl_table"),
            size: byte_data.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buf, 0, byte_data);
        self.vpl_buffer = Some(buf);
        self.vpl_section_count = vpl.num_sections;
    }

    /// Render a VXL sprite via GPU compute and read back the palette-index result.
    ///
    /// `limbs` contains pre-packed voxel data and transforms for each limb.
    /// Multiple VXL models can be composited by passing all their limbs
    /// together — atomicMin handles depth compositing automatically.
    /// `bounds` provides the sprite dimensions.
    /// `scale` is the pixel scale factor from VxlRenderParams.
    ///
    /// Returns palette-index pixel data (width × height bytes). Each byte is
    /// the post-VPL, pre-house-remap palette index. Byte 0 = transparent.
    pub fn render_sprite(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        limbs: &[GpuLimb],
        bounds: &SpriteBounds,
        scale: f32,
    ) -> Vec<u8> {
        let pixel_count = bounds.width * bounds.height;
        if pixel_count == 0 {
            return vec![];
        }

        if self.vpl_buffer.is_none() {
            log::warn!("VxlComputeRenderer: no VPL uploaded, returning empty sprite");
            return vec![0u8; pixel_count as usize];
        }

        // Ensure buffers are large enough.
        self.ensure_capacity(device, pixel_count);

        // Clear atomic framebuffer to 0xFFFFFFFF (empty).
        let clear_data = vec![0xFFFFFFFFu32; pixel_count as usize];
        queue.write_buffer(&self.atomic_fb, 0, bytemuck::cast_slice(&clear_data));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vxl_compute_encoder"),
        });

        // One depth window for the whole draw: every limb shares the atomic FB.
        let (depth_min, depth_max) = draw_depth_window(limbs);
        let depth_scale: f32 = 65535.0 / (depth_max - depth_min).max(f32::EPSILON);

        // Splat pass: one dispatch per limb (all share the same atomic FB).
        for gl in limbs {
            let voxel_count = gl.positions.len() as u32;
            if voxel_count == 0 {
                continue;
            }

            // Pack VPL pages (256 bytes → 64 u32s).
            let vpl_pages_packed: Vec<u32> = gl
                .vpl_pages
                .chunks(4)
                .map(|c| {
                    u32::from_le_bytes([
                        c[0],
                        c.get(1).copied().unwrap_or(0),
                        c.get(2).copied().unwrap_or(0),
                        c.get(3).copied().unwrap_or(0),
                    ])
                })
                .collect();

            // The shader computes dot(mat_row, pos) for each output component.
            // glam stores column-major, so transposing gives rows as columns.
            let m = gl.combined.transpose();
            let params_data = SplatParams {
                mat_col0: m.col(0).into(),
                mat_col1: m.col(1).into(),
                mat_col2: m.col(2).into(),
                mat_col3: m.col(3).into(),
                scale: scale,
                fp_scale: 65536.0,
                fb_width: bounds.width,
                fb_height: bounds.height,
                buf_off_x_fp: bounds.buf_off_x_fp,
                buf_off_y_fp: bounds.buf_off_y_fp,
                fill_size: bounds.fill_size,
                half_fill: bounds.half_fill,
                voxel_count,
                depth_min,
                depth_scale,
                _pad2: 0,
            };

            // Create per-limb GPU buffers.
            let params_buf = create_buffer_init(
                device,
                "splat_params",
                bytemuck::bytes_of(&params_data),
                wgpu::BufferUsages::UNIFORM,
            );
            let pos_buf = create_buffer_init(
                device,
                "voxel_positions",
                bytemuck::cast_slice(&gl.positions),
                wgpu::BufferUsages::STORAGE,
            );
            let data_buf = create_buffer_init(
                device,
                "voxel_data",
                bytemuck::cast_slice(&gl.data),
                wgpu::BufferUsages::STORAGE,
            );
            let pages_buf = create_buffer_init(
                device,
                "vpl_pages",
                bytemuck::cast_slice(&vpl_pages_packed),
                wgpu::BufferUsages::STORAGE,
            );

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vxl_splat_bg"),
                layout: &self.splat_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: pos_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: data_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: pages_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.atomic_fb.as_entire_binding(),
                    },
                ],
            });

            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("vxl_splat_pass"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.splat_pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                cpass.dispatch_workgroups((voxel_count + 255) / 256, 1, 1);
            }
        }

        // Resolve pass.
        let resolve_params = ResolveParams {
            fb_width: bounds.width,
            fb_height: bounds.height,
            pixel_count,
            vpl_section_count: self.vpl_section_count.max(1),
        };
        let resolve_params_buf = create_buffer_init(
            device,
            "resolve_params",
            bytemuck::bytes_of(&resolve_params),
            wgpu::BufferUsages::UNIFORM,
        );

        let resolve_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vxl_resolve_bg"),
            layout: &self.resolve_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: resolve_params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.atomic_fb.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.vpl_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.output_palette_indices.as_entire_binding(),
                },
            ],
        });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vxl_resolve_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.resolve_pipeline);
            cpass.set_bind_group(0, &resolve_bg, &[]);
            cpass.dispatch_workgroups((pixel_count + 255) / 256, 1, 1);
        }

        // Copy output to staging buffer for readback.
        // Output is one u32 per pixel (low byte = palette index).
        let byte_size = (pixel_count as u64) * 4;
        encoder.copy_buffer_to_buffer(&self.output_palette_indices, 0, &self.staging, 0, byte_size);

        queue.submit(std::iter::once(encoder.finish()));

        // Map staging buffer and read back palette indices (low byte of each u32).
        let buffer_slice = self.staging.slice(..byte_size);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());

        match receiver.recv() {
            Ok(Ok(())) => {
                let mapped = buffer_slice.get_mapped_range();
                // Extract low byte of each u32 → palette-index byte per pixel.
                let result: Vec<u8> = mapped.chunks_exact(4).map(|c| c[0]).collect();
                drop(mapped);
                self.staging.unmap();
                result
            }
            _ => {
                log::error!("VxlComputeRenderer: staging buffer map failed");
                self.staging.unmap();
                vec![0u8; pixel_count as usize]
            }
        }
    }

    /// Ensure atomic_fb, output, and staging buffers can hold `pixel_count` pixels.
    fn ensure_capacity(&mut self, device: &wgpu::Device, pixel_count: u32) {
        if pixel_count <= self.fb_capacity {
            return;
        }
        // Grow to next power of two for amortization.
        let new_cap = pixel_count.next_power_of_two();
        let byte_size = (new_cap as u64) * 4;

        self.atomic_fb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vxl_atomic_fb"),
            size: byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.output_palette_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vxl_output_palette_indices"),
            size: byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        self.staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vxl_staging"),
            size: byte_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.fb_capacity = new_cap;
        log::debug!(
            "VxlComputeRenderer: grew buffers to {} pixels ({} bytes)",
            new_cap,
            byte_size,
        );
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_buffer_init(
    device: &wgpu::Device,
    label: &str,
    data: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: data,
        usage,
    })
}

fn bgl_uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_storage_ro(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_storage_rw(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

#[cfg(test)]
mod depth_window_tests {
    use super::*;
    use glam::{Mat4, Vec3};

    fn limb(extent: u32, combined: Mat4) -> GpuLimb {
        // Two voxels at the box extremes along +Y are enough to define the box.
        GpuLimb {
            positions: vec![0, extent << 8],
            data: vec![1, 1],
            vpl_pages: [0; 256],
            combined,
        }
    }

    #[test]
    fn window_covers_a_hull_longer_than_the_old_fixed_range() {
        // The old shader clamped to -50..+50. A 160-unit hull tilted 60 deg by
        // the camera reaches about +-69 in depth; the window must contain it.
        let tilt: Mat4 = Mat4::from_rotation_x(-60f32.to_radians());
        let centred: Mat4 = tilt * Mat4::from_translation(Vec3::new(0.0, -80.0, 0.0));
        let (lo, hi) = draw_depth_window(&[limb(160, centred)]);
        let expect: f32 = 80.0 * 60f32.to_radians().sin();
        assert!(
            lo <= -expect && hi >= expect,
            "window {lo}..{hi} must cover +-{expect}"
        );
        assert!(
            lo > -expect - 2.0 && hi < expect + 2.0,
            "window {lo}..{hi} is too loose"
        );
    }

    #[test]
    fn window_spans_every_limb_of_a_draw() {
        let near: Mat4 = Mat4::from_translation(Vec3::new(0.0, 0.0, 30.0));
        let far: Mat4 = Mat4::from_translation(Vec3::new(0.0, 0.0, -90.0));
        let (lo, hi) = draw_depth_window(&[limb(4, near), limb(4, far)]);
        assert!(lo <= -90.0 && hi >= 30.0, "{lo}..{hi}");
    }

    #[test]
    fn empty_draw_gets_a_finite_window() {
        let (lo, hi) = draw_depth_window(&[]);
        assert!(lo < hi && lo.is_finite() && hi.is_finite());
    }
}
