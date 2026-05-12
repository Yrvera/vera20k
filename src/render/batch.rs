//! Sprite batch renderer — draws many textured quads per frame using GPU instancing.
//!
//! One draw call renders hundreds of sprites, each with its own screen position,
//! size, and UV coordinates. Essential for terrain tiles (hundreds per viewport).
//! Bind group 0 = camera uniform (screen size + scroll offset), bind group 1 = texture.
//! Instance buffer provides per-sprite data as vertex attributes (step_mode = Instance).
//!
//! ## Dependency rules
//! - Part of render/ — depends on render/gpu for GpuContext.

use std::collections::HashMap;

use wgpu::util::DeviceExt;

use crate::render::gpu::GpuContext;

/// WGSL shader for instanced sprite rendering (loaded from batch_shader.wgsl).
///
/// Vertex shader: generates quad from vertex_index, applies per-instance position/size,
/// and transforms screen-space pixel coordinates to clip space using the camera uniform.
/// Fragment shader: samples the sprite texture with per-instance UV coordinates.
const BATCH_SHADER: &str = include_str!("batch_shader.wgsl");

/// WGSL shader with per-pixel Z-depth output via @builtin(frag_depth).
/// Samples a parallel R8 depth atlas to compute per-pixel depth for terrain
/// tiles (cliff occlusion) and overlays.
const ZDEPTH_SHADER: &str = include_str!("zdepth_shader.wgsl");

/// WGSL voxel-sprite shader: byte → (palette | house_ramp) → fx pipeline.
/// Atlas is R8Uint (palette indices); palette + per-house RGB ramp are
/// sampled via PaletteSet (bind group 2).
const VOXEL_SPRITE_SHADER: &str = include_str!("sprite_voxel_shader.wgsl");

/// Per-sprite instance data uploaded to the GPU each frame.
///
/// Each instance defines one textured quad: position on screen, pixel size,
/// UV rectangle within the texture, and depth for the depth buffer.
/// The vertex shader uses these along with the camera uniform to produce
/// clip-space positions with correct depth ordering.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    /// Top-left corner position in world/screen pixels.
    pub position: [f32; 2],
    /// Width and height of the sprite in pixels.
    pub size: [f32; 2],
    /// Top-left UV coordinate in the texture (0.0 to 1.0).
    pub uv_origin: [f32; 2],
    /// UV width and height (0.0 to 1.0).
    pub uv_size: [f32; 2],
    /// Depth value for the depth buffer (0.0 = near/front, 1.0 = far/back).
    /// Lower screen_y objects get larger depth (drawn behind).
    pub depth: f32,
    /// RGB color tint from map lighting. [1.0, 1.0, 1.0] = no tint (full brightness).
    /// Values < 1.0 darken, > 1.0 brighten (up to 2.0 cap from the lighting formula).
    pub tint: [f32; 3],
    /// Alpha multiplier for translucency. 1.0 = fully opaque, 0.5 = 50% translucent.
    /// Used for chrono warp "being warped" visual (50% during chrono delay).
    pub alpha: f32,
    /// Per-house ramp row index. 0 = no remap (SHP / non-voxel paths;
    /// row 0 of the PaletteSet house_ramp_tex mirrors the theater palette's
    /// [16, 32) range, so byte ranges stay correct). 1..=MAX_HOUSES-1 =
    /// per-house ramp row. Read by the voxel sprite fragment shader; other
    /// pipelines ignore it.
    pub house_color_idx: u32,
    /// Bitfield of active visual FX. Bit 0 = cloak, bit 1 = EMP, bit 2 =
    /// iron curtain, bit 3 = warp, bit 4 = mirror. Phase 1 stubs this as 0;
    /// phases 2-5 populate.
    pub fx_flags: u32,
    /// FX scalar parameters: [cloak_alpha, emp_dim, ic_phase, warp_phase].
    /// Stub in Phase 1.
    pub fx_params: [f32; 4],
    /// Iron-curtain tint: [r, g, b, intensity]. Stub in Phase 1.
    pub ic_tint: [f32; 4],
}

/// Number of vertex attributes in SpriteInstance: 7 base + 4 voxel-shader fields.
const INSTANCE_ATTRIBUTE_COUNT: usize = 11;

/// Size of one SpriteInstance in bytes (4 × vec2f = 32 bytes).
const INSTANCE_STRIDE: u64 = std::mem::size_of::<SpriteInstance>() as u64;

/// Camera uniform data sent to the GPU vertex shader.
///
/// Allows the shader to convert screen-space pixel coordinates into
/// normalized clip space, and to apply camera scrolling.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// Viewport width and height in pixels.
    pub screen_size: [f32; 2],
    /// Camera scroll position in world pixels (top-left corner of viewport).
    pub camera_pos: [f32; 2],
    /// Zoom level: 1.0 = native scale, >1.0 = zoomed in, <1.0 = zoomed out.
    pub zoom: f32,
    /// Padding for 16-byte alignment.
    pub _pad: f32,
}

/// A GPU texture prepared for batch rendering.
///
/// Created via `BatchRenderer::create_texture()`.
pub struct BatchTexture {
    /// Bind group containing texture view + sampler.
    pub bind_group: wgpu::BindGroup,
    /// Raw texture view — exposed for use by the Z-depth pipeline bind group.
    pub view: wgpu::TextureView,
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
}

/// A reusable GPU instance buffer entry. Tracks the wgpu buffer and its current
/// capacity (in number of SpriteInstance elements). Grows on demand, never shrinks.
struct PooledBuffer {
    /// The GPU buffer. Has VERTEX | COPY_DST usage so we can write_buffer() into it.
    buffer: wgpu::Buffer,
    /// Maximum number of SpriteInstance elements the buffer can hold.
    capacity: usize,
}

/// Pool of named GPU instance buffers that persist across frames.
///
/// Instead of creating and destroying GPU buffers every frame (expensive driver
/// round-trips), this pool keeps buffers alive and overwrites their contents
/// with `queue.write_buffer()`. Buffers grow automatically when needed (2x strategy)
/// but never shrink, avoiding repeated reallocations.
///
/// Usage pattern:
/// 1. Call `upload()` for each named buffer (mutably borrows pool).
/// 2. Call `get()` during the render pass to retrieve buffer refs (immutably borrows pool).
pub struct InstanceBufferPool {
    /// Named buffers keyed by a static string (e.g., "terrain", "units").
    buffers: HashMap<&'static str, PooledBuffer>,
    /// Instance counts for each buffer written this frame.
    /// Stored separately so `get()` can return count without needing the data.
    counts: HashMap<&'static str, u32>,
}

/// Minimum buffer capacity in elements. Avoids tiny buffers that immediately
/// need reallocation. 64 instances × 48 bytes = 3 KB — negligible VRAM.
const MIN_POOL_CAPACITY: usize = 64;

impl InstanceBufferPool {
    /// Create an empty pool. Buffers are allocated lazily on first use.
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            counts: HashMap::new(),
        }
    }

    /// Upload instance data into a named buffer, reusing/growing as needed.
    ///
    /// On first call for a given key, allocates a new GPU buffer. On subsequent
    /// frames, reuses the existing buffer if it fits, or replaces it with a larger
    /// one (2x growth). Data is uploaded via `queue.write_buffer()` — a simple
    /// memcpy, much cheaper than `create_buffer_init()` which allocates new VRAM.
    ///
    /// If `instances` is empty, the count is set to 0 and no GPU upload occurs.
    pub fn upload(&mut self, gpu: &GpuContext, key: &'static str, instances: &[SpriteInstance]) {
        let needed: usize = instances.len();
        if needed == 0 {
            self.counts.insert(key, 0);
            return;
        }

        let entry: &mut PooledBuffer = self.buffers.entry(key).or_insert_with(|| {
            let cap: usize = needed.max(MIN_POOL_CAPACITY);
            PooledBuffer {
                buffer: Self::alloc_buffer(gpu, key, cap),
                capacity: cap,
            }
        });

        // Grow if the current buffer is too small.
        if needed > entry.capacity {
            let new_cap: usize = (entry.capacity * 2).max(needed);
            entry.buffer = Self::alloc_buffer(gpu, key, new_cap);
            entry.capacity = new_cap;
        }

        let byte_data: &[u8] = bytemuck::cast_slice(instances);
        gpu.queue.write_buffer(&entry.buffer, 0, byte_data);
        self.counts.insert(key, needed as u32);
    }

    /// Get a previously uploaded buffer and its instance count.
    ///
    /// Returns None if the key was never uploaded or had 0 instances.
    /// Safe to call from the render pass — only borrows &self.
    pub fn get(&self, key: &'static str) -> Option<(&wgpu::Buffer, u32)> {
        let count: u32 = *self.counts.get(key)?;
        if count == 0 {
            return None;
        }
        let entry: &PooledBuffer = self.buffers.get(key)?;
        Some((&entry.buffer, count))
    }

    /// Allocate a GPU buffer with VERTEX + COPY_DST usage for the given capacity.
    fn alloc_buffer(gpu: &GpuContext, label: &str, capacity: usize) -> wgpu::Buffer {
        let byte_size: u64 = (capacity as u64) * (std::mem::size_of::<SpriteInstance>() as u64);
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: byte_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}

/// Instanced sprite batch renderer.
///
/// Draws many textured quads in a single draw call using GPU instancing.
/// Call `update_camera()` and `prepare_instances()` each frame before drawing.
///
/// Pipelines:
/// - `pipeline` / `zdepth_pipeline` (terrain): depth write ON — terrain writes Z-data.
/// - `overlay_pipeline` (cliff redraw, UI): depth write ON, LessEqual — for passes
///   that must write depth (cliff face redraw after entities).
pub struct BatchRenderer {
    /// Render pipeline for terrain (depth write + Less compare).
    pipeline: wgpu::RenderPipeline,
    /// Render pipeline with depth write ON, LessEqual compare.
    /// Used for cliff redraw (must write depth) and UI passes.
    overlay_pipeline: wgpu::RenderPipeline,
    /// Render pipeline with per-pixel Z-depth (frag_depth output, Less compare).
    /// Used for terrain tiles with TMP Z-data.
    zdepth_pipeline: wgpu::RenderPipeline,
    /// Render pipeline for non-wall overlays (ore, trees): depth compare Always,
    /// depth write OFF. Overlays draw unconditionally over terrain because
    /// tiles without Z-data skip Z-testing entirely.
    overlay_passthrough_pipeline: wgpu::RenderPipeline,
    /// Layout for texture bind groups (group 1).
    texture_bind_group_layout: wgpu::BindGroupLayout,
    /// Layout for zdepth texture bind groups (group 1): color + sampler + R8 depth.
    zdepth_texture_bind_group_layout: wgpu::BindGroupLayout,
    /// Layout for the unit-atlas R8Uint texture (binding 0). Used by the voxel
    /// sprite pipeline; tiles store palette indices, not RGB.
    pub unit_atlas_bind_group_layout: wgpu::BindGroupLayout,
    /// Layout for the PaletteSet bind group (palette + house_ramp + sampler).
    /// Stored here so the voxel sprite pipeline can be created at BatchRenderer
    /// init time, before the actual PaletteSet exists. PaletteSet creates a
    /// structurally-identical layout for its own bind group at theater load.
    pub voxel_palette_bind_group_layout: wgpu::BindGroupLayout,
    /// Voxel sprite pipeline: byte → (palette | house_ramp) → FX. Reads from
    /// `unit_atlas_bind_group_layout` (group 1) + `voxel_palette_bind_group_layout`
    /// (group 2). Same vertex layout as the other batch pipelines.
    pub voxel_sprite_pipeline: wgpu::RenderPipeline,
    /// Layout for camera uniform bind group (group 0).
    /// Stored so other pipelines (e.g., fog shader) can reuse the same layout.
    camera_bind_group_layout: wgpu::BindGroupLayout,
    /// Camera uniform buffer (group 0) — world camera with zoom.
    camera_buffer: wgpu::Buffer,
    /// Camera bind group — world camera with zoom.
    camera_bind_group: wgpu::BindGroup,
    /// UI camera uniform buffer — always zoom=1.0 for screen-fixed elements.
    ui_camera_buffer: wgpu::Buffer,
    /// UI camera bind group — always zoom=1.0.
    ui_camera_bind_group: wgpu::BindGroup,
    /// Per-frame instance buffer. Recreated each frame in prepare_instances().
    instance_buffer: Option<wgpu::Buffer>,
    /// Number of instances in the current buffer.
    instance_count: u32,
}

impl BatchRenderer {
    /// Create a new BatchRenderer. Compiles shader, creates pipeline and camera uniform.
    pub fn new(gpu: &GpuContext) -> Self {
        let shader: wgpu::ShaderModule =
            gpu.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Batch Shader"),
                    source: wgpu::ShaderSource::Wgsl(BATCH_SHADER.into()),
                });

        // Bind group 0: Camera uniform.
        let camera_bind_group_layout: wgpu::BindGroupLayout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Batch Camera BGL"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        // Bind group 1: Texture + sampler.
        let texture_bind_group_layout: wgpu::BindGroupLayout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Batch Texture BGL"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        // Camera uniform buffer (initialized with default values).
        let camera_uniform: CameraUniform = CameraUniform {
            screen_size: [1024.0, 768.0],
            camera_pos: [0.0, 0.0],
            zoom: 1.0,
            _pad: 0.0,
        };
        let camera_buffer: wgpu::Buffer =
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Camera Uniform"),
                    contents: bytemuck::cast_slice(&[camera_uniform]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let camera_bind_group: wgpu::BindGroup =
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Camera Bind Group"),
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
            });

        // UI camera — identical layout but always zoom=1.0 for screen-fixed elements.
        let ui_camera_buffer: wgpu::Buffer =
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("UI Camera Uniform"),
                    contents: bytemuck::cast_slice(&[camera_uniform]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let ui_camera_bind_group: wgpu::BindGroup =
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("UI Camera Bind Group"),
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ui_camera_buffer.as_entire_binding(),
                }],
            });

        // Instance buffer vertex layout (matches SpriteInstance memory layout):
        //   position(8) + size(8) + uv_origin(8) + uv_size(8) = 32 → loc 0-3
        //   depth(4) + tint(12) + alpha(4) = 20 → loc 4-6 (offsets 32, 36, 48)
        //   house_color_idx(4) at offset 52 → loc 7 (Uint32)
        //   fx_flags(4) at offset 56 → loc 8 (Uint32)
        //   fx_params(16) at offset 60 → loc 9 (Float32x4)
        //   ic_tint(16) at offset 76 → loc 10 (Float32x4)
        // Total stride: 92 bytes.
        let instance_attrs: [wgpu::VertexAttribute; INSTANCE_ATTRIBUTE_COUNT] = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 8,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 16,
                shader_location: 2,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 24,
                shader_location: 3,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32,
                offset: 32,
                shader_location: 4,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 36,
                shader_location: 5,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32,
                offset: 48,
                shader_location: 6,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: 52,
                shader_location: 7,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: 56,
                shader_location: 8,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 60,
                shader_location: 9,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 76,
                shader_location: 10,
            },
        ];

        let pipeline_layout: wgpu::PipelineLayout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Batch Pipeline Layout"),
                    bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
                    push_constant_ranges: &[],
                });

        // Terrain pipeline: depth buffer enabled (write + Less compare).
        // Terrain tiles sort correctly against each other via the depth buffer.
        let pipeline: wgpu::RenderPipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Batch Pipeline (Terrain)"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: INSTANCE_STRIDE,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &instance_attrs,
                        }],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: gpu.surface_format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        // Overlay pipeline: depth write ON, LessEqual compare.
        // Used for cliff redraw (must write depth to occlude sprites behind cliffs)
        // and UI passes that don't interact with game depth.
        let overlay_pipeline: wgpu::RenderPipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Batch Pipeline (Overlay Write)"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: INSTANCE_STRIDE,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &instance_attrs,
                        }],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: gpu.surface_format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        // Passthrough pipeline for non-wall overlays: depth compare Always, no write.
        // Tiles without embedded Z-data (flag 0x02 at cell header byte 36) skip
        // Z-testing entirely. Ore, gems, and terrain objects have no Z-data, so
        // they paint unconditionally over terrain.
        let overlay_passthrough_pipeline: wgpu::RenderPipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Batch Pipeline (Overlay Passthrough)"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: INSTANCE_STRIDE,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &instance_attrs,
                        }],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: gpu.surface_format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: false,
                        depth_compare: wgpu::CompareFunction::Always,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        // Z-depth bind group layout: color texture + sampler + R8 depth texture.
        let zdepth_texture_bind_group_layout: wgpu::BindGroupLayout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ZDepth Texture BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        // Z-depth pipeline: per-pixel depth via frag_depth, Less compare.
        let zdepth_shader: wgpu::ShaderModule =
            gpu.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("ZDepth Shader"),
                    source: wgpu::ShaderSource::Wgsl(ZDEPTH_SHADER.into()),
                });
        let zdepth_pipeline_layout: wgpu::PipelineLayout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ZDepth Pipeline Layout"),
                    bind_group_layouts: &[
                        &camera_bind_group_layout,
                        &zdepth_texture_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });
        let zdepth_pipeline: wgpu::RenderPipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("ZDepth Pipeline (Terrain)"),
                    layout: Some(&zdepth_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &zdepth_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: INSTANCE_STRIDE,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &instance_attrs,
                        }],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &zdepth_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: gpu.surface_format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        // Bind group layout for the unit-atlas R8Uint texture (voxel sprite path).
        // Single texture entry, no sampler — sampled via textureLoad with integer coords.
        let unit_atlas_bind_group_layout: wgpu::BindGroupLayout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("unit_atlas_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });

        // Bind group layout for the PaletteSet (palette texture + house_ramp
        // texture + non-filtering sampler). Must structurally match
        // PaletteSet::new()'s layout (wgpu compares layouts structurally, not
        // by reference, so two distinct BindGroupLayouts with identical
        // entries are interchangeable).
        let voxel_palette_bind_group_layout: wgpu::BindGroupLayout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("voxel_palette_bgl_for_pipeline"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        // Voxel sprite pipeline: 3 bind groups (camera, atlas R8Uint, PaletteSet).
        // Same vertex layout as the existing batch pipelines (uses all 11 attributes).
        // Depth: write OFF, compare LessEqual — units sort against the depth
        // buffer that terrain wrote, but don't write depth themselves (one
        // unit's pixels shouldn't occlude another unit's translucent pixels).
        let voxel_shader: wgpu::ShaderModule =
            gpu.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Voxel Sprite Shader"),
                    source: wgpu::ShaderSource::Wgsl(VOXEL_SPRITE_SHADER.into()),
                });
        let voxel_sprite_pipeline_layout: wgpu::PipelineLayout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Voxel Sprite Pipeline Layout"),
                    bind_group_layouts: &[
                        &camera_bind_group_layout,
                        &unit_atlas_bind_group_layout,
                        &voxel_palette_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });
        let voxel_sprite_pipeline: wgpu::RenderPipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Voxel Sprite Pipeline"),
                    layout: Some(&voxel_sprite_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &voxel_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: INSTANCE_STRIDE,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &instance_attrs,
                        }],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &voxel_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: gpu.surface_format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: false,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        Self {
            pipeline,
            overlay_pipeline,
            zdepth_pipeline,
            overlay_passthrough_pipeline,
            texture_bind_group_layout,
            zdepth_texture_bind_group_layout,
            unit_atlas_bind_group_layout,
            voxel_palette_bind_group_layout,
            voxel_sprite_pipeline,
            camera_bind_group_layout,
            camera_buffer,
            camera_bind_group,
            ui_camera_buffer,
            ui_camera_bind_group,
            instance_buffer: None,
            instance_count: 0,
        }
    }

    /// Create a single-channel R8Uint atlas texture from byte data.
    ///
    /// Used for voxel sprite atlases where each byte is a palette index
    /// (post-VPL, pre-house-remap). Sampled in shader via `textureLoad` (no
    /// filtering, integer coords).
    pub fn create_unit_atlas_texture(
        &self,
        gpu: &GpuContext,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> BatchTexture {
        debug_assert_eq!(
            pixels.len(),
            (width * height) as usize,
            "pixel buffer size must equal width * height"
        );

        let texture: wgpu::Texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("unit_atlas_r8uint"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
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

        let view: wgpu::TextureView = texture.create_view(&Default::default());
        let bind_group: wgpu::BindGroup =
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("unit_atlas_bg"),
                layout: &self.unit_atlas_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                }],
            });

        BatchTexture {
            bind_group,
            view,
            width,
            height,
        }
    }

    /// Upload RGBA pixel data to the GPU as a batch-renderable texture.
    ///
    /// Uses nearest-neighbor sampling (pixel art). The returned BatchTexture
    /// can be shared across multiple draw_batch() calls.
    pub fn create_texture(
        &self,
        gpu: &GpuContext,
        rgba_data: &[u8],
        width: u32,
        height: u32,
    ) -> BatchTexture {
        let texture: wgpu::Texture = gpu.device.create_texture_with_data(
            &gpu.queue,
            &wgpu::TextureDescriptor {
                label: Some("Batch Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            rgba_data,
        );

        let view: wgpu::TextureView = texture.create_view(&Default::default());
        let sampler: wgpu::Sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Batch Sampler (Nearest)"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group: wgpu::BindGroup =
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Batch Texture BG"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

        BatchTexture {
            bind_group,
            view,
            width,
            height,
        }
    }

    /// Update the camera uniform with current viewport size and scroll position.
    ///
    /// Call once per frame before draw_batch(). screen_width/height are in pixels.
    /// camera_x/y define the top-left corner of the visible area in world coordinates.
    pub fn update_camera(
        &self,
        gpu: &GpuContext,
        screen_width: f32,
        screen_height: f32,
        camera_x: f32,
        camera_y: f32,
        zoom: f32,
    ) {
        // Round camera to integer pixels — sub-pixel camera offsets cause
        // visible seams between adjacent terrain tiles.
        let cam = [camera_x.round(), camera_y.round()];
        let uniform: CameraUniform = CameraUniform {
            screen_size: [screen_width, screen_height],
            camera_pos: cam,
            zoom,
            _pad: 0.0,
        };
        gpu.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
        // UI camera — same position but always zoom=1.0 so screen-fixed elements
        // (sidebar, minimap, cursor) don't scale with the game world zoom.
        let ui_uniform: CameraUniform = CameraUniform {
            screen_size: [screen_width, screen_height],
            camera_pos: cam,
            zoom: 1.0,
            _pad: 0.0,
        };
        gpu.queue.write_buffer(
            &self.ui_camera_buffer,
            0,
            bytemuck::cast_slice(&[ui_uniform]),
        );
    }

    /// Upload instance data for this frame.
    ///
    /// Creates a new GPU buffer from the provided instances. Must be called
    /// before draw_batch() each frame. The buffer is stored in the renderer
    /// so it remains valid during the render pass.
    pub fn prepare_instances(&mut self, gpu: &GpuContext, instances: &[SpriteInstance]) {
        if instances.is_empty() {
            self.instance_buffer = None;
            self.instance_count = 0;
            return;
        }
        self.instance_buffer = Some(gpu.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Batch Instances"),
                contents: bytemuck::cast_slice(instances),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));
        self.instance_count = instances.len() as u32;
    }

    /// Draw all prepared instances with the given texture.
    ///
    /// Issues a single instanced draw call: 6 vertices (one quad) × N instances.
    /// Call prepare_instances() first to upload this frame's instance data.
    pub fn draw_batch<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        texture: &'a BatchTexture,
    ) {
        let Some(instance_buffer) = &self.instance_buffer else {
            return;
        };
        if self.instance_count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &texture.bind_group, &[]);
        render_pass.set_vertex_buffer(0, instance_buffer.slice(..));
        render_pass.draw(0..6, 0..self.instance_count);
    }

    /// Create a standalone instance buffer (not stored in the renderer).
    ///
    /// Use this when drawing multiple batches per render pass — each batch
    /// gets its own buffer that stays alive until the render pass ends.
    /// Returns None if instances is empty.
    pub fn create_instance_buffer(
        &self,
        gpu: &GpuContext,
        instances: &[SpriteInstance],
    ) -> Option<(wgpu::Buffer, u32)> {
        if instances.is_empty() {
            return None;
        }
        let buffer: wgpu::Buffer =
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("External Batch Instances"),
                    contents: bytemuck::cast_slice(instances),
                    usage: wgpu::BufferUsages::VERTEX,
                });
        Some((buffer, instances.len() as u32))
    }

    /// Draw instances from an external buffer with the given texture.
    ///
    /// Unlike draw_batch(), this doesn't use the internally stored instance buffer.
    /// Use with create_instance_buffer() when drawing multiple texture groups
    /// in a single render pass (e.g., terrain tiles + unit sprites).
    pub fn draw_with_buffer<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        texture: &'a BatchTexture,
        buffer: &'a wgpu::Buffer,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &texture.bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..6, 0..count);
    }

    /// Draw voxel sprite instances using the voxel sprite pipeline.
    ///
    /// Bind groups: 0 = camera, 1 = unit atlas (R8Uint), 2 = PaletteSet (palette
    /// + house_ramp + sampler). The fragment shader does the byte → (palette
    /// or house_ramp) → fx pipeline per pixel.
    pub fn draw_voxel_sprites<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        atlas: &'a BatchTexture,
        palette_bind_group: &'a wgpu::BindGroup,
        buffer: &'a wgpu::Buffer,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.voxel_sprite_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &atlas.bind_group, &[]);
        render_pass.set_bind_group(2, palette_bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..6, 0..count);
    }

    /// Draw a sub-range of voxel sprite instances using the voxel sprite pipeline.
    /// Used by the multi-way Y-sort merge passes that interleave voxel and SHP
    /// draw calls. Same semantics as `draw_voxel_sprites` but with a start index.
    pub fn draw_voxel_sprites_range<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        atlas: &'a BatchTexture,
        palette_bind_group: &'a wgpu::BindGroup,
        buffer: &'a wgpu::Buffer,
        start: u32,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.voxel_sprite_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &atlas.bind_group, &[]);
        render_pass.set_bind_group(2, palette_bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..6, start..start + count);
    }

    /// Access the camera bind group for use by external pipelines (e.g., fog shader).
    pub fn camera_bind_group(&self) -> &wgpu::BindGroup {
        &self.camera_bind_group
    }

    /// Access the camera bind group layout so external pipelines can share it.
    pub fn camera_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bind_group_layout
    }

    /// Access the UI camera bind group (zoom=1.0) for screen-fixed elements.
    pub fn ui_camera_bind_group(&self) -> &wgpu::BindGroup {
        &self.ui_camera_bind_group
    }

    /// Access the overlay (no-depth) pipeline for manual draw calls.
    pub fn overlay_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.overlay_pipeline
    }

    /// Create a reusable texture that supports `queue.write_texture()` updates.
    ///
    /// Returns both the raw `wgpu::Texture` (needed for write_texture) and the
    /// `BatchTexture` (needed for draw calls). The texture is created with
    /// `TEXTURE_BINDING | COPY_DST` usage so it can be updated each frame
    /// without recreating the bind group.
    pub fn create_updatable_texture(
        &self,
        gpu: &GpuContext,
        rgba_data: &[u8],
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, BatchTexture) {
        let texture: wgpu::Texture = gpu.device.create_texture_with_data(
            &gpu.queue,
            &wgpu::TextureDescriptor {
                label: Some("Updatable Batch Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            rgba_data,
        );

        let view: wgpu::TextureView = texture.create_view(&Default::default());
        let sampler: wgpu::Sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Updatable Batch Sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group: wgpu::BindGroup =
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Updatable Batch Texture BG"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

        let batch_tex = BatchTexture {
            bind_group,
            view,
            width,
            height,
        };
        (texture, batch_tex)
    }

    /// Upload RGBA pixel data as a bilinear-filtered texture (smooth interpolation).
    ///
    /// Unlike `create_texture()` which uses nearest-neighbor (pixel art), this uses
    /// linear filtering for smooth gradients. Used by the fog mask renderer where
    /// per-cell values need to blend smoothly across tile boundaries.
    /// Uses Rgba8Unorm (not sRGB) so interpolation is linear in value space.
    pub fn create_texture_bilinear(
        &self,
        gpu: &GpuContext,
        rgba_data: &[u8],
        width: u32,
        height: u32,
    ) -> BatchTexture {
        let texture: wgpu::Texture = gpu.device.create_texture_with_data(
            &gpu.queue,
            &wgpu::TextureDescriptor {
                label: Some("Batch Texture (Bilinear)"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            rgba_data,
        );

        let view: wgpu::TextureView = texture.create_view(&Default::default());
        let sampler: wgpu::Sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Batch Sampler (Linear)"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let bind_group: wgpu::BindGroup =
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Batch Texture BG (Bilinear)"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

        BatchTexture {
            bind_group,
            view,
            width,
            height,
        }
    }

    /// Draw instances using the overlay pipeline (LessEqual, depth write ON).
    ///
    /// Used for cliff redraw (must write depth) and UI passes.
    pub fn draw_with_buffer_no_depth<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        texture: &'a BatchTexture,
        buffer: &'a wgpu::Buffer,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.overlay_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &texture.bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..6, 0..count);
    }

    /// Draw a sub-range of sprites with LessEqual depth test and depth write ON.
    pub fn draw_depth_range<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        texture: &'a BatchTexture,
        buffer: &'a wgpu::Buffer,
        start: u32,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.overlay_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &texture.bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..6, start..start + count);
    }

    /// Create a bind group for the Z-depth pipeline (color + sampler + R8 depth).
    ///
    /// The color texture view and depth texture view must have identical UV layout
    /// (same atlas dimensions and tile placements) so the shader can sample both
    /// at the same UV coordinates.
    pub fn create_zdepth_bind_group(
        &self,
        gpu: &GpuContext,
        color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        let sampler: wgpu::Sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ZDepth Sampler (Nearest)"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ZDepth Bind Group"),
            layout: &self.zdepth_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
            ],
        })
    }

    /// Draw instances with the Z-depth pipeline (per-pixel frag_depth, Less compare).
    ///
    /// Used for terrain tiles with TMP Z-data. The bind_group must be created via
    /// `create_zdepth_bind_group()` with matching color + depth atlas textures.
    pub fn draw_with_buffer_zdepth<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
        buffer: &'a wgpu::Buffer,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.zdepth_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..6, 0..count);
    }

    /// Draw sprites/overlays with depth test bypassed (Always compare).
    /// Sprites never interact with the Z-buffer — painted over terrain unconditionally.
    pub fn draw_with_buffer_passthrough<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        texture: &'a BatchTexture,
        buffer: &'a wgpu::Buffer,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.overlay_passthrough_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &texture.bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..6, 0..count);
    }

    /// Draw a sub-range of sprites with depth test bypassed (Always compare).
    /// Used for the multi-way merge of Y-sorted VXL + SHP draw groups.
    pub fn draw_passthrough_range<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        texture: &'a BatchTexture,
        buffer: &'a wgpu::Buffer,
        start: u32,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.overlay_passthrough_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &texture.bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..6, start..start + count);
    }
}
