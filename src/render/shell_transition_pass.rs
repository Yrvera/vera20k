//! Offscreen shell transition bridge compositor.
//!
//! Owns source/destination shell render targets and composites them into the
//! swapchain. This is an app/render bridge for the temporary main-menu ->
//! Skirmish shortcut, not a verified native shell transition.

use super::gpu::GpuContext;

const SHADER_SRC: &str = include_str!("shell_transition.wgsl");

pub(crate) struct ShellRenderTarget<'a> {
    pub(crate) color: &'a wgpu::TextureView,
    pub(crate) depth: &'a wgpu::TextureView,
}

#[allow(dead_code)] // Textures are retained to keep their views alive.
pub(crate) struct ShellTransitionPass {
    source_color: wgpu::Texture,
    destination_color: wgpu::Texture,
    source_depth: wgpu::Texture,
    destination_depth: wgpu::Texture,
    source_color_view: wgpu::TextureView,
    destination_color_view: wgpu::TextureView,
    source_depth_view: wgpu::TextureView,
    destination_depth_view: wgpu::TextureView,
    params_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
}

impl ShellTransitionPass {
    pub(crate) fn new(gpu: &GpuContext, width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let source_color = create_color_texture(gpu, width, height, "Shell Transition Source RT");
        let destination_color =
            create_color_texture(gpu, width, height, "Shell Transition Destination RT");
        let source_depth =
            create_depth_texture(gpu, width, height, "Shell Transition Source Depth");
        let destination_depth =
            create_depth_texture(gpu, width, height, "Shell Transition Destination Depth");
        let source_color_view = source_color.create_view(&Default::default());
        let destination_color_view = destination_color.create_view(&Default::default());
        let source_depth_view = source_depth.create_view(&Default::default());
        let destination_depth_view = destination_depth.create_view(&Default::default());
        let params_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shell Transition Params"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shell Transition Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let bind_group_layout = create_bind_group_layout(gpu);
        let bind_group = create_bind_group(
            gpu,
            &bind_group_layout,
            &source_color_view,
            &destination_color_view,
            &sampler,
            &params_buffer,
        );
        let pipeline = create_pipeline(gpu, &bind_group_layout);

        Self {
            source_color,
            destination_color,
            source_depth,
            destination_depth,
            source_color_view,
            destination_color_view,
            source_depth_view,
            destination_depth_view,
            params_buffer,
            bind_group,
            bind_group_layout,
            pipeline,
            sampler,
            width,
            height,
        }
    }

    pub(crate) fn size_matches(&self, width: u32, height: u32) -> bool {
        self.width == width.max(1) && self.height == height.max(1)
    }

    pub(crate) fn source_render_target(&self) -> ShellRenderTarget<'_> {
        ShellRenderTarget {
            color: &self.source_color_view,
            depth: &self.source_depth_view,
        }
    }

    pub(crate) fn destination_render_target(&self) -> ShellRenderTarget<'_> {
        ShellRenderTarget {
            color: &self.destination_color_view,
            depth: &self.destination_depth_view,
        }
    }

    pub(crate) fn draw(
        &self,
        gpu: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        progress: f32,
    ) {
        let params = [
            progress.clamp(0.0, 1.0),
            self.width as f32,
            self.height as f32,
            0.0,
        ];
        gpu.queue
            .write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&params));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Shell Bridge Transition Composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(crate::app_types::CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn create_color_texture(
    gpu: &GpuContext,
    width: u32,
    height: u32,
    label: &'static str,
) -> wgpu::Texture {
    gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: gpu.surface_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn create_depth_texture(
    gpu: &GpuContext,
    width: u32,
    height: u32,
    label: &'static str,
) -> wgpu::Texture {
    gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

fn create_bind_group_layout(gpu: &GpuContext) -> wgpu::BindGroupLayout {
    gpu.device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shell Transition BGL"),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn create_bind_group(
    gpu: &GpuContext,
    layout: &wgpu::BindGroupLayout,
    source: &wgpu::TextureView,
    destination: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    params_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Shell Transition BG"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(destination),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    })
}

fn create_pipeline(gpu: &GpuContext, bgl: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline {
    let shader = gpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shell Transition Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });
    let layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shell Transition Pipeline Layout"),
            bind_group_layouts: &[bgl],
            push_constant_ranges: &[],
        });

    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shell Transition Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
}
