//! Native transient tactical combat-light presentation.
//!
//! gamemd does not create an animation, particle, or map light for an
//! IronCurtain/ForceShield impact. It appends a small record to a dedicated
//! vector and later edits the already-composed 16-bit tactical surface. This
//! renderer keeps that presentation state outside the simulation, builds the
//! native 64 byte-mask surfaces once, and applies every active record in the
//! native reverse insertion order in encoded RGB565 space.

use wgpu::util::DeviceExt;

/// One combat light the tactical pass draws this frame. Render-owned draw DTO
/// (F06); the app runtime that ages and drains lights produces these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CombatLightDrawRecord {
    pub coord: crate::sim::projectile::ProjectileCoord,
    pub surface_index: u8,
    pub flags: u32,
}
use crate::render::batch::CameraUniform;
use crate::render::gpu::GpuContext;

const SHADER: &str = include_str!("combat_light.wgsl");
const ANIM_SHADOW_SHADER: &str = include_str!("anim_shadow.wgsl");
const MASK_WIDTH: u32 = 256;
const MASK_HEIGHT: u32 = 128;
const MASK_LAYER_COUNT: u32 = 64;
const INITIAL_CAPACITY: usize = 16;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuLightInstance {
    center: [i32; 2],
    surface_index: u32,
    flags: u32,
}

struct CompositionTargets {
    texture: wgpu::Texture,
    render_view: wgpu::TextureView,
    encoded_view: wgpu::TextureView,
    scratch_texture: wgpu::Texture,
    // Retained with the bind group to make GPU resource ownership explicit.
    _scratch_encoded_view: wgpu::TextureView,
    scene_bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

/// App-owned renderer for the native persistent combat-light vector.
pub(crate) struct CombatLightRenderer {
    pipeline: wgpu::RenderPipeline,
    /// Exact encoded-destination compositor for ordinary AnimClass
    /// `Shadow=yes` draws (native flags 0x601).
    anim_shadow_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    mask_bind_group: wgpu::BindGroup,
    scene_bind_group_layout: wgpu::BindGroupLayout,
    instance_buffer: wgpu::Buffer,
    capacity: usize,
    record_count: usize,
    surface_format: wgpu::TextureFormat,
    encoded_format: wgpu::TextureFormat,
    targets: CompositionTargets,
}

impl CombatLightRenderer {
    pub(crate) fn new(
        gpu: &GpuContext,
        batch: &crate::render::batch::BatchRenderer,
    ) -> Self {
        let encoded_format = gpu.surface_format.remove_srgb_suffix();
        let camera_layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Combat Light Camera Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Combat Light Camera"),
                contents: bytemuck::bytes_of(&CameraUniform {
                    screen_size: [1.0, 1.0],
                    camera_pos: [0.0, 0.0],
                    zoom: 1.0,
                    _pad: 0.0,
                }),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let camera_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Combat Light Camera Bind Group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let scene_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Combat Light Scene Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    }],
                });
        let mask_layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Combat Light Native Mask Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                }],
            });
        let mask_texture = create_native_mask_texture(gpu);
        let mask_view = mask_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Combat Light Native Mask Array View"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let mask_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Combat Light Native Mask Bind Group"),
            layout: &mask_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&mask_view),
            }],
        });

        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Combat Light Exact RGB565 Shader"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Combat Light Exact RGB565 Pipeline Layout"),
                bind_group_layouts: &[&camera_layout, &scene_bind_group_layout, &mask_layout],
                push_constant_ranges: &[],
            });
        let attributes = wgpu::vertex_attr_array![0 => Sint32x2, 1 => Uint32, 2 => Uint32];
        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Combat Light Exact RGB565 Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GpuLightInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &attributes,
                    }],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: encoded_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            });

        let anim_shadow_shader =
            gpu.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("AnimClass Exact Encoded Shadow Shader"),
                    source: wgpu::ShaderSource::Wgsl(ANIM_SHADOW_SHADER.into()),
                });
        let anim_shadow_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("AnimClass Exact Encoded Shadow Pipeline Layout"),
                    bind_group_layouts: &[
                        batch.camera_bind_group_layout(),
                        batch.texture_bind_group_layout(),
                        &scene_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });
        let anim_shadow_attributes = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
            2 => Float32x2,
            3 => Float32x2,
            4 => Float32
        ];
        let anim_shadow_pipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("AnimClass Exact Encoded Shadow Pipeline"),
                    layout: Some(&anim_shadow_layout),
                    vertex: wgpu::VertexState {
                        module: &anim_shadow_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<
                                crate::render::batch::SpriteInstance,
                            >() as u64,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &anim_shadow_attributes,
                        }],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &anim_shadow_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            // No sRGB conversion is allowed here: 0x601 edits
                            // the stored encoded destination, not linear light.
                            format: encoded_format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview: None,
                    cache: None,
                });

        let instance_buffer = allocate_instance_buffer(gpu, INITIAL_CAPACITY);
        let targets = create_composition_targets(
            gpu,
            &scene_bind_group_layout,
            gpu.surface_format,
            encoded_format,
            1,
            1,
        );
        Self {
            pipeline,
            anim_shadow_pipeline,
            camera_buffer,
            camera_bind_group,
            mask_bind_group,
            scene_bind_group_layout,
            instance_buffer,
            capacity: INITIAL_CAPACITY,
            record_count: 0,
            surface_format: gpu.surface_format,
            encoded_format,
            targets,
        }
    }

    /// Prepare one tactical frame. The composition surface is recreated only
    /// when the render resolution changes.
    pub(crate) fn prepare(
        &mut self,
        gpu: &GpuContext,
        records: &[CombatLightDrawRecord],
        screen_size: [f32; 2],
        camera_pos: [f32; 2],
        zoom: f32,
    ) {
        let width = screen_size[0].max(1.0) as u32;
        let height = screen_size[1].max(1.0) as u32;
        if self.targets.width != width || self.targets.height != height {
            self.targets = create_composition_targets(
                gpu,
                &self.scene_bind_group_layout,
                self.surface_format,
                self.encoded_format,
                width,
                height,
            );
        }
        gpu.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&CameraUniform {
                screen_size,
                camera_pos,
                zoom,
                _pad: 0.0,
            }),
        );
        self.record_count = records.len();
        if records.is_empty() {
            return;
        }
        if records.len() > self.capacity {
            self.capacity = records.len().next_power_of_two();
            self.instance_buffer = allocate_instance_buffer(gpu, self.capacity);
        }
        let instances: Vec<GpuLightInstance> = records
            .iter()
            .map(|record| GpuLightInstance {
                center: project_combat_light_anchor(record.coord),
                surface_index: u32::from(record.surface_index),
                flags: record.flags,
            })
            .collect();
        gpu.queue
            .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
    }

    pub(crate) fn composition_view(&self) -> wgpu::TextureView {
        self.targets.render_view.clone()
    }

    /// Texture behind [`Self::composition_view`]. Screenshot retention copies
    /// it at the UI/cursor boundary before the cursor pass modifies it.
    pub(crate) fn composition_texture(&self) -> &wgpu::Texture {
        &self.targets.texture
    }

    /// Apply a contiguous native-ordered run of AnimClass shadow stencils.
    ///
    /// A fresh destination snapshot precedes *every* instance. That is
    /// load-bearing for overlapping DestroyAnim shadows: two nonzero stencils
    /// halve the already-halved encoded destination rather than both reading
    /// the same pre-run colour.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_anim_shadow_run(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        batch: &crate::render::batch::BatchRenderer,
        stencil: &crate::render::batch::BatchTexture,
        buffer: &wgpu::Buffer,
        start: u32,
        count: u32,
        tactical: [u32; 4],
    ) {
        let extent = wgpu::Extent3d {
            width: self.targets.width,
            height: self.targets.height,
            depth_or_array_layers: 1,
        };
        for index in anim_shadow_instance_indices(start, count) {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.targets.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.targets.scratch_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                extent,
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("AnimClass Native 0x601 Encoded Shadow Edit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.encoded_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.anim_shadow_pipeline);
            pass.set_bind_group(0, batch.camera_bind_group(), &[]);
            pass.set_bind_group(1, &stencil.bind_group, &[]);
            pass.set_bind_group(2, &self.targets.scene_bind_group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.set_scissor_rect(tactical[0], tactical[1], tactical[2], tactical[3]);
            pass.draw(0..6, index..index + 1);
        }
    }

    /// Apply the vector after tactical objects and before later tactical
    /// families. A scene copy per record preserves destination-dependent
    /// overlap while the records are consumed in their already-reversed order.
    pub(crate) fn draw(&self, encoder: &mut wgpu::CommandEncoder, tactical: [u32; 4]) {
        if self.record_count == 0 {
            return;
        }
        let extent = wgpu::Extent3d {
            width: self.targets.width,
            height: self.targets.height,
            depth_or_array_layers: 1,
        };
        for index in 0..self.record_count {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.targets.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.targets.scratch_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                extent,
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Combat Light Native Surface Edit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.encoded_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_bind_group(1, &self.targets.scene_bind_group, &[]);
            pass.set_bind_group(2, &self.mask_bind_group, &[]);
            pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
            pass.set_scissor_rect(tactical[0], tactical[1], tactical[2], tactical[3]);
            pass.draw(0..6, index as u32..index as u32 + 1);
        }
    }

    /// Copy the completed tactical frame into the swapchain or upscale input.
    pub(crate) fn copy_to(&self, encoder: &mut wgpu::CommandEncoder, destination: &wgpu::Texture) {
        debug_assert_eq!(destination.width(), self.targets.width);
        debug_assert_eq!(destination.height(), self.targets.height);
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.targets.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: destination,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.targets.width,
                height: self.targets.height,
                depth_or_array_layers: 1,
            },
        );
    }
}

fn anim_shadow_instance_indices(start: u32, count: u32) -> std::ops::Range<u32> {
    start..start.saturating_add(count)
}

/// Native `CoordsToClient2` projects the planar terms with signed integer
/// division, so both axes truncate toward zero before camera and presentation
/// scaling. VERA's common world-row bias remains after that native projection
/// so this direct-surface primitive aligns with the rest of the tactical scene.
fn project_combat_light_anchor(coord: crate::sim::projectile::ProjectileCoord) -> [i32; 2] {
    let (planar_x, planar_y) =
        crate::render::tactical_compat::project_native_planar(coord.x, coord.y);
    [
        planar_x,
        planar_y
            .wrapping_sub(crate::util::flh_transform::adjust_for_z_leptons(coord.z))
            .wrapping_add(15),
    ]
}

fn allocate_instance_buffer(gpu: &GpuContext, capacity: usize) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Combat Light Instances"),
        size: (capacity * std::mem::size_of::<GpuLightInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_composition_targets(
    gpu: &GpuContext,
    scene_layout: &wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
    encoded_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> CompositionTargets {
    let extent = wgpu::Extent3d {
        width: width.max(1),
        height: height.max(1),
        depth_or_array_layers: 1,
    };
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Combat Light Tactical Composition Surface"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: surface_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[encoded_format],
    });
    let render_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Combat Light Tactical sRGB Render View"),
        format: Some(surface_format),
        usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
        ..Default::default()
    });
    let encoded_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Combat Light Tactical Encoded View"),
        format: Some(encoded_format),
        usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
        ..Default::default()
    });
    let scratch_texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Combat Light Destination Snapshot"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: surface_format,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[encoded_format],
    });
    let scratch_encoded_view = scratch_texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Combat Light Destination Encoded Sampling View"),
        format: Some(encoded_format),
        usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
        ..Default::default()
    });
    let scene_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Combat Light Destination Snapshot Bind Group"),
        layout: scene_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&scratch_encoded_view),
        }],
    });
    CompositionTargets {
        texture,
        render_view,
        encoded_view,
        scratch_texture,
        _scratch_encoded_view: scratch_encoded_view,
        scene_bind_group,
        width: extent.width,
        height: extent.height,
    }
}

fn create_native_mask_texture(gpu: &GpuContext) -> wgpu::Texture {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Combat Light Native 64-Surface Mask Array"),
        size: wgpu::Extent3d {
            width: MASK_WIDTH,
            height: MASK_HEIGHT,
            depth_or_array_layers: MASK_LAYER_COUNT,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let masks = build_native_mask_atlas();
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &masks,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(MASK_WIDTH),
            rows_per_image: Some(MASK_HEIGHT),
        },
        wgpu::Extent3d {
            width: MASK_WIDTH,
            height: MASK_HEIGHT,
            depth_or_array_layers: MASK_LAYER_COUNT,
        },
    );
    texture
}

/// Build BSurface indices 0..63: draw odd-radius concentric filled circles on
/// a 256x256 byte surface, then retain every other source scanline.
fn build_native_mask_atlas() -> Vec<u8> {
    const SOURCE_HEIGHT: usize = 256;
    let mut atlas = vec![0; MASK_WIDTH as usize * MASK_HEIGHT as usize * MASK_LAYER_COUNT as usize];
    let mut source = vec![0; MASK_WIDTH as usize * SOURCE_HEIGHT];
    let layer_stride = MASK_WIDTH as usize * MASK_HEIGHT as usize;
    for layer in 0..MASK_LAYER_COUNT as usize {
        source.fill(0);
        let mut radius = (layer as i32) * 2 + 1;
        let mut intensity: u8 = 12;
        while radius > 0 {
            fill_midpoint_circle(&mut source, radius, intensity);
            radius -= 2;
            intensity = intensity.saturating_add(4);
        }
        let layer_start = layer * layer_stride;
        for destination_y in 0..MASK_HEIGHT as usize {
            let source_start = destination_y * 2 * MASK_WIDTH as usize;
            let destination_start = layer_start + destination_y * MASK_WIDTH as usize;
            atlas[destination_start..destination_start + MASK_WIDTH as usize]
                .copy_from_slice(&source[source_start..source_start + MASK_WIDTH as usize]);
        }
    }
    atlas
}

fn fill_midpoint_circle(surface: &mut [u8], radius: i32, intensity: u8) {
    let mut x = 0;
    let mut y = radius;
    let mut decision = 3 - radius * 2;
    while y >= x {
        draw_span(surface, 128 + x, 128 - y, 128 + y, intensity);
        draw_span(surface, 128 + y, 128 - x, 128 + x, intensity);
        draw_span(surface, 128 - x, 128 - y, 128 + y, intensity);
        draw_span(surface, 128 - y, 128 - x, 128 + x, intensity);
        if decision < 0 {
            decision += 4 * x + 6;
        } else {
            decision += 4 * (x - y) + 10;
            y -= 1;
        }
        x += 1;
    }
}

fn draw_span(surface: &mut [u8], y: i32, x0: i32, x1: i32, intensity: u8) {
    debug_assert!((0..256).contains(&y));
    debug_assert!((0..256).contains(&x0));
    debug_assert!((0..256).contains(&x1));
    let start = y as usize * MASK_WIDTH as usize + x0 as usize;
    let end = y as usize * MASK_WIDTH as usize + x1 as usize;
    surface[start..=end].fill(intensity);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native_channel(destination: u8, mask: u8, flags: u32, disable_bit: u32) -> u8 {
        let quantized = if disable_bit == 4 {
            destination & 0xfc
        } else {
            destination & 0xf8
        };
        let transformed = if flags & 1 != 0 {
            (u16::from(quantized) * (256 - u16::from(mask)) >> 8) as u8
        } else if flags & disable_bit != 0 {
            quantized
        } else {
            (u16::from(quantized) * (256 + u16::from(mask)) >> 8).min(255) as u8
        };
        if disable_bit == 4 {
            transformed & 0xfc
        } else {
            transformed & 0xf8
        }
    }

    fn cpu_draw_native_light(
        coord: crate::sim::projectile::ProjectileCoord,
        surface_index: usize,
        flags: u32,
        camera: [i32; 2],
        zoom: i32,
        size: [usize; 2],
        background: [u8; 3],
    ) -> ([i32; 2], Option<[i32; 4]>, Vec<[u8; 3]>) {
        let anchor = project_combat_light_anchor(coord);
        let top_left = [
            (anchor[0] - camera[0]) * zoom - MASK_WIDTH as i32 / 2,
            (anchor[1] - camera[1]) * zoom - MASK_HEIGHT as i32 / 2,
        ];
        let masks = build_native_mask_atlas();
        let layer_stride = MASK_WIDTH as usize * MASK_HEIGHT as usize;
        let layer = &masks[surface_index * layer_stride..(surface_index + 1) * layer_stride];
        let mut pixels = vec![background; size[0] * size[1]];
        let mut bounds: Option<[i32; 4]> = None;
        for local_y in 0..MASK_HEIGHT as usize {
            for local_x in 0..MASK_WIDTH as usize {
                let mask = layer[local_y * MASK_WIDTH as usize + local_x];
                if mask == 0 {
                    continue;
                }
                let x = top_left[0] + local_x as i32;
                let y = top_left[1] + local_y as i32;
                if x < 0 || y < 0 || x >= size[0] as i32 || y >= size[1] as i32 {
                    continue;
                }
                pixels[y as usize * size[0] + x as usize] = [
                    native_channel(background[0], mask, flags, 2),
                    native_channel(background[1], mask, flags, 4),
                    native_channel(background[2], mask, flags, 8),
                ];
                bounds = Some(match bounds {
                    None => [x, y, x, y],
                    Some([left, top, right, bottom]) => {
                        [left.min(x), top.min(y), right.max(x), bottom.max(y)]
                    }
                });
            }
        }
        (anchor, bounds, pixels)
    }

    #[test]
    fn gsi_04_07_invulnerability_light_projection_truncates_before_full_rectangle_draw() {
        let width = 320;
        let (anchor, bounds, pixels) = cpu_draw_native_light(
            crate::sim::projectile::ProjectileCoord::new(255, 0, 0),
            63,
            1,
            [0, 0],
            1,
            [width, 200],
            [160, 160, 160],
        );

        assert_eq!(anchor, [29, 29]);
        assert_eq!(bounds, Some([0, 0, 156, 92]));
        assert_eq!(pixels[29 * width + 29], [0, 0, 0]);
        assert_eq!(pixels[29 * width + 156], [152, 152, 152]);
        assert_eq!(pixels[29 * width + 157], [160, 160, 160]);
    }

    #[test]
    fn gsi_04_07_invulnerability_light_masks_use_native_midpoint_surfaces() {
        let atlas = build_native_mask_atlas();
        let stride = MASK_WIDTH as usize * MASK_HEIGHT as usize;
        assert_eq!(atlas.len(), stride * 64);

        let layer0 = &atlas[..stride];
        assert_eq!(layer0[64 * 256 + 128], 12);
        assert_eq!(layer0[64 * 256 + 127], 12);
        assert_eq!(layer0[63 * 256 + 128], 0);

        let layer63 = &atlas[63 * stride..64 * stride];
        assert_eq!(layer63[64 * 256 + 128], 255);
        assert_eq!(layer63[64 * 256 + 1], 12);
        assert_eq!(layer63[64 * 256], 0);
        assert_eq!(layer63[0], 0);
    }

    #[test]
    fn gsi_04_07_invulnerability_light_ic_darkens_and_force_shield_only_brightens_blue() {
        assert_eq!(native_channel(160, 64, 1, 2), 120);
        assert_eq!(native_channel(160, 64, 1, 4), 120);
        assert_eq!(native_channel(160, 64, 1, 8), 120);
        assert_eq!(native_channel(160, 64, 6, 2), 160);
        assert_eq!(native_channel(160, 64, 6, 4), 160);
        assert_eq!(native_channel(160, 64, 6, 8), 200);
    }

    #[test]
    fn phase3_anim_shadow_pipeline_halves_encoded_destination_per_nonzero_stencil() {
        wgpu::naga::front::wgsl::parse_str(ANIM_SHADOW_SHADER)
            .expect("the production encoded-shadow WGSL must validate");

        fn encoded_edit(destination: f32, stencil: u8) -> f32 {
            if stencil == 0 {
                destination
            } else {
                destination * 0.5
            }
        }
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

        let destination = 0.5;
        let native = encoded_edit(destination, 1);
        let approximate = linear_to_srgb(0.5 * srgb_to_linear(destination));
        assert_eq!(native, 0.25);
        assert!((approximate - 0.36078).abs() < 0.001);
        assert!(
            (native - approximate).abs() > 0.1,
            "the exact encoded edit must remain distinguishable from source-alpha sRGB blending",
        );
        assert_eq!(encoded_edit(destination, 0), destination);

        let repeated = encoded_edit(encoded_edit(destination, 9), 4);
        assert_eq!(repeated, 0.125, "overlap must halve the already-halved destination");

        assert!(ANIM_SHADOW_SHADER.contains("textureLoad(destination_snapshot"));
        assert!(ANIM_SHADOW_SHADER.contains("destination.rgb * 0.5"));
        assert!(ANIM_SHADOW_SHADER.contains("discard"));
    }

    #[test]
    fn phase3_anim_shadow_compositor_snapshots_before_every_overlapping_instance() {
        assert_eq!(
            anim_shadow_instance_indices(7, 3).collect::<Vec<_>>(),
            vec![7, 8, 9],
            "each instance gets its own copy-then-edit iteration",
        );
        assert!(anim_shadow_instance_indices(4, 0).is_empty());
    }
}
