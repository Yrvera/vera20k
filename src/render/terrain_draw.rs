//! Destination-dependent ordinary terrain SHP drawing.
//!
//! Terrain DrawIt 0071C304/0071C34E reaches 004990E0/00497390: signed Z
//! comparison precedes the u16 store, and shadow pixels halve the packed
//! destination. See TERRAIN_STATIC_BODY_SHADOW_NATIVE_2026_09_09.md.
//! Live tactical Depth32Float remains the only Z authority. wgpu 27 requires
//! full-size depth copies, so a scissored MRT pass snapshots the piece's rect
//! into integer color and float depth scratch while live Z is not attached.

use super::batch::{
    BatchRenderer, BatchTexture, CameraUniform, SPRITE_INSTANCE_ATTRIBUTES, SpriteInstance,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerrainPiece {
    Body,
    Shadow,
}

struct Targets {
    source_color: wgpu::Texture,
    source_depth: wgpu::TextureView,
    _encoded_source: wgpu::TextureView,
    words: wgpu::TextureView,
    depth: wgpu::TextureView,
    source: wgpu::BindGroup,
    snapshot: wgpu::BindGroup,
}

pub(crate) struct TerrainDrawRenderer {
    snapshot_pipeline: wgpu::RenderPipeline,
    body_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    source_layout: wgpu::BindGroupLayout,
    snapshot_layout: wgpu::BindGroupLayout,
    targets: Option<Targets>,
    camera: CameraUniform,
}

impl TerrainDrawRenderer {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        batch: &BatchRenderer,
    ) -> Self {
        assert!(
            matches!(
                format,
                wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Rgba8UnormSrgb
            ),
            "native terrain output requires the declared sRGB tactical attachment"
        );
        let entry = |binding, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let source_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Terrain live attachments"),
            entries: &[
                entry(0, wgpu::TextureSampleType::Float { filterable: false }),
                entry(1, wgpu::TextureSampleType::Depth),
            ],
        });
        let snapshot_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Terrain immutable piece snapshot"),
            entries: &[
                entry(0, wgpu::TextureSampleType::Uint),
                entry(1, wgpu::TextureSampleType::Float { filterable: false }),
            ],
        });
        let snapshot_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terrain encoded RGB565 and native Z snapshot"),
            source: wgpu::ShaderSource::Wgsl(include_str!("terrain_snapshot.wgsl").into()),
        });
        let snapshot_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Terrain snapshot layout"),
                bind_group_layouts: &[&source_layout],
                push_constant_ranges: &[],
            });
        let target = |format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        };
        let snapshot_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Terrain piece snapshot"),
            layout: Some(&snapshot_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &snapshot_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &snapshot_shader,
                entry_point: Some("fs_main"),
                targets: &[
                    target(wgpu::TextureFormat::R32Uint),
                    target(wgpu::TextureFormat::R32Float),
                ],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        // Reuse the production SHP projection/ABI verbatim, without its shape
        // binding or fragment policy. Both consumers share native_row_z.
        let shp = include_str!("zsprite_shader.wgsl");
        let vertex = shp[..shp.find("fn apply_fx(").expect("SHP projection boundary")]
            .replace("@group(2) @binding(0) var t_zshape: texture_2d<f32>;", "");
        let source = super::tactical_shader::source(&format!(
            "{vertex}\n{}",
            include_str!("terrain_edit.wgsl")
        ));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terrain signed Z and packed destination edit"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Terrain edit layout"),
            bind_group_layouts: &[
                batch.camera_bind_group_layout(),
                batch.texture_bind_group_layout(),
                &snapshot_layout,
            ],
            push_constant_ranges: &[],
        });
        let pipeline = |fragment| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fragment),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<SpriteInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &SPRITE_INSTANCE_ATTRIBUTES,
                    }],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    targets: &[target(format)],
                    compilation_options: Default::default(),
                }),
                primitive: Default::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    // Manual comparison must precede wrapped storage. Hardware Less
                    // on low16(candidate) is not equivalent for signed candidates.
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        Self {
            snapshot_pipeline,
            body_pipeline: pipeline("fs_body"),
            shadow_pipeline: pipeline("fs_shadow"),
            source_layout,
            snapshot_layout,
            targets: None,
            camera: CameraUniform {
                screen_size: [1.0, 1.0],
                camera_pos: [0.0, 0.0],
                zoom: 1.0,
                world_origin_y: 0.0,
                world_height: 1.0,
                _pad: 0.0,

                native_z_origin_y: 0.0,
                _native_z_pad: 0.0,
            },
        }
    }

    /// Called after composition resize and after the optional upscale depth
    /// swap. Bindings track both concrete source handles, not only dimensions.
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        color: &wgpu::Texture,
        depth: &wgpu::TextureView,
        camera: CameraUniform,
    ) {
        self.camera = camera;
        if self
            .targets
            .as_ref()
            .is_some_and(|t| t.source_color == *color && t.source_depth == *depth)
        {
            return;
        }
        assert!(color.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING));
        assert!(
            depth
                .texture()
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
        );
        assert_eq!(color.size(), depth.texture().size());
        let encoded_source = color.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Terrain encoded source bytes"),
            format: Some(color.format().remove_srgb_suffix()),
            usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
            ..Default::default()
        });
        let scratch = |format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Terrain piece scratch"),
                    size: color.size(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let words = scratch(wgpu::TextureFormat::R32Uint);
        let scratch_depth = scratch(wgpu::TextureFormat::R32Float);
        let binding = |binding, view| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        };
        let source = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain live snapshot sources"),
            layout: &self.source_layout,
            entries: &[binding(0, &encoded_source), binding(1, depth)],
        });
        let snapshot = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain piece snapshot results"),
            layout: &self.snapshot_layout,
            entries: &[binding(0, &words), binding(1, &scratch_depth)],
        });
        self.targets = Some(Targets {
            source_color: color.clone(),
            source_depth: depth.clone(),
            _encoded_source: encoded_source,
            words,
            depth: scratch_depth,
            source,
            snapshot,
        });
    }

    /// Replay exactly one piece against the immediately preceding destination.
    /// Scratch uses absolute framebuffer coordinates; scissor does not reset
    /// SHP row/gradient origins when the top or left of the sprite is clipped.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_piece(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        batch: &BatchRenderer,
        atlas: &BatchTexture,
        buffer: &wgpu::Buffer,
        index: u32,
        instance: &SpriteInstance,
        piece: TerrainPiece,
        tactical: [u32; 4],
    ) {
        let Some(rect) = piece_scissor(instance, self.camera, tactical) else {
            return;
        };
        let targets = self
            .targets
            .as_ref()
            .expect("terrain prepare precedes drawing");
        let attachment = |view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Terrain snapshot one piece"),
                color_attachments: &[attachment(&targets.words), attachment(&targets.depth)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_scissor_rect(rect[0], rect[1], rect[2], rect[3]);
            pass.set_pipeline(&self.snapshot_pipeline);
            pass.set_bind_group(0, &targets.source, &[]);
            pass.draw(0..3, 0..1);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Terrain native piece edit"),
            color_attachments: &[attachment(color)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_scissor_rect(rect[0], rect[1], rect[2], rect[3]);
        pass.set_pipeline(match piece {
            TerrainPiece::Body => &self.body_pipeline,
            TerrainPiece::Shadow => &self.shadow_pipeline,
        });
        pass.set_bind_group(0, batch.camera_bind_group(), &[]);
        pass.set_bind_group(1, &atlas.bind_group, &[]);
        pass.set_bind_group(2, &targets.snapshot, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..6, index..index + 1);
    }
}

fn piece_scissor(
    instance: &SpriteInstance,
    camera: CameraUniform,
    tactical: [u32; 4],
) -> Option<[u32; 4]> {
    // Include the same half-screen-pixel zoom padding as the SHP vertex path.
    let pad = if (camera.zoom - 1.0).abs() >= 0.001 {
        0.5
    } else {
        0.0
    };
    let left = ((instance.position[0] - camera.camera_pos[0]) * camera.zoom - pad).floor() as i32;
    let top = ((instance.position[1] - camera.camera_pos[1]) * camera.zoom - pad).floor() as i32;
    let right = ((instance.position[0] + instance.size[0] - camera.camera_pos[0]) * camera.zoom
        + pad)
        .ceil() as i32;
    let bottom = ((instance.position[1] + instance.size[1] - camera.camera_pos[1]) * camera.zoom
        + pad)
        .ceil() as i32;
    let x = left.max(tactical[0] as i32).max(0);
    let y = top.max(tactical[1] as i32).max(0);
    let r = right
        .min((tactical[0] + tactical[2]) as i32)
        .min(camera.screen_size[0] as i32);
    let b = bottom
        .min((tactical[1] + tactical[3]) as i32)
        .min(camera.screen_size[1] as i32);
    (r > x && b > y).then_some([x as u32, y as u32, (r - x) as u32, (b - y) as u32])
}
