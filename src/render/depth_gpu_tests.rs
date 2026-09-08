//! Offscreen regression coverage for the actual tactical depth shaders.
//!
//! These tests upload production `SpriteInstance`/`CameraUniform` data and
//! execute the checked-in WGSL. They do not replace shader math with a CPU
//! implementation. Synthetic texels isolate depth admission, transparent holes,
//! BUILDNGZ seed handling, and tint preservation; they are not retail-scene or
//! gamemd pixel-parity goldens. Pipeline setup is a minimal offscreen harness,
//! so production draw-list routing still needs its own tests/comparison.
//!
//! Run explicitly on a machine with a wgpu adapter:
//! `cargo test -p vera20k --lib render::depth_gpu_tests:: -- --ignored`

use std::mem::{offset_of, size_of};
use std::time::Duration;

use wgpu::util::DeviceExt;

use super::batch::{CameraUniform, SpriteInstance};
use super::draw_state::DrawState;
use super::frame_readback::PendingBgra8Readback;
use super::native_z::{Z_GRADIENT_ZSHAPE_CLIP_FLAG, Z_GRADIENT_ZSHAPE_FLAG, ZGradient};

const SIDE: u32 = 8;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

#[derive(Clone, Copy)]
enum Shader {
    Batch,
    Terrain,
    SpriteRead,
    SpriteWrite,
}

impl Shader {
    fn source(self) -> &'static str {
        match self {
            Self::Batch => include_str!("batch_shader.wgsl"),
            Self::Terrain => include_str!("zdepth_shader.wgsl"),
            Self::SpriteRead | Self::SpriteWrite => include_str!("zsprite_shader.wgsl"),
        }
    }

    fn writes_depth(self) -> bool {
        matches!(self, Self::Terrain | Self::SpriteWrite)
    }
}

struct Layer {
    shader: Shader,
    instance: SpriteInstance,
    rgba: Vec<u8>,
    z_bytes: Vec<u8>,
}

impl Layer {
    fn solid(shader: Shader, rgba: [u8; 4], z_adjust: f32) -> Self {
        Self {
            shader,
            instance: SpriteInstance {
                position: [0.0, 0.0],
                size: [SIDE as f32; 2],
                uv_size: [1.0; 2],
                tint: [1.0; 3],
                alpha: 1.0,
                depth: 0.5,
                z_adjust,
                z_gradient: ZGradient::Flat as u32,
                ..Default::default()
            },
            rgba: rgba.repeat((SIDE * SIDE) as usize),
            z_bytes: vec![0; (SIDE * SIDE) as usize],
        }
    }

    fn hole(&mut self, x: u32, y: u32) {
        self.rgba[((y * SIDE + x) * 4 + 3) as usize] = 0;
    }
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Gpu {
    fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: None,
            ..Default::default()
        }))
        .expect("explicit GPU regression test requires a wgpu adapter");
        eprintln!("Depth shader test adapter: {:?}", adapter.get_info());
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Depth shader regression device"),
            ..Default::default()
        }))
        .expect("request offscreen test device");
        Self { device, queue }
    }

    fn texture(&self, format: wgpu::TextureFormat, bytes: &[u8]) -> wgpu::TextureView {
        self.device
            .create_texture_with_data(
                &self.queue,
                &wgpu::TextureDescriptor {
                    label: Some("Depth test source"),
                    size: extent(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                bytes,
            )
            .create_view(&Default::default())
    }

    fn render(&self, layers: &[Layer]) -> Vec<[u8; 4]> {
        let camera = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Depth test camera"),
                contents: bytemuck::bytes_of(&CameraUniform {
                    screen_size: [SIDE as f32; 2],
                    camera_pos: [0.0; 2],
                    zoom: 1.0,
                    world_origin_y: -100.0,
                    world_height: 256.0,
                    _pad: 0.0,
                }),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let sampler = self
            .device
            .create_sampler(&wgpu::SamplerDescriptor::default());
        let color = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth test output"),
            size: extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth test shared Z"),
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            ..color_descriptor()
        });
        let color_view = color.create_view(&Default::default());
        let depth_view = depth.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());

        for (index, layer) in layers.iter().enumerate() {
            let module = self
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Production tactical shader under test"),
                    source: wgpu::ShaderSource::Wgsl(layer.shader.source().into()),
                });
            let pipeline = self
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Offscreen tactical shader harness"),
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs_main"),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: size_of::<SpriteInstance>() as u64,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &INSTANCE_ATTRIBUTES,
                        }],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: FORMAT,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: Default::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: layer.shader.writes_depth(),
                        // TMP leaf 0x547CF0 admits equality. Ordinary SHP leaves
                        // 0x494B60/0x497FD0 and building leaves 0x4958D0/0x4990E0
                        // use strict less-than. This exercises those contracts,
                        // not production BatchRenderer's pipeline selection.
                        depth_compare: if matches!(layer.shader, Shader::Terrain) {
                            wgpu::CompareFunction::LessEqual
                        } else {
                            wgpu::CompareFunction::Less
                        },
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    multiview: None,
                    cache: None,
                });
            let camera_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Depth test camera group"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                }],
            });
            let color_source = self.texture(wgpu::TextureFormat::Rgba8UnormSrgb, &layer.rgba);
            let z_source = self.texture(wgpu::TextureFormat::R8Unorm, &layer.z_bytes);
            let mut texture_entries = vec![
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ];
            if matches!(layer.shader, Shader::Terrain) {
                texture_entries.push(wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&z_source),
                });
            }
            let texture_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Depth test source group"),
                layout: &pipeline.get_bind_group_layout(1),
                entries: &texture_entries,
            });
            let zshape_group = matches!(layer.shader, Shader::SpriteRead | Shader::SpriteWrite)
                .then(|| {
                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Depth test zshape group"),
                        layout: &pipeline.get_bind_group_layout(2),
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&z_source),
                        }],
                    })
                });
            let instances = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Production SpriteInstance bytes"),
                    contents: bytemuck::bytes_of(&layer.instance),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Depth shader regression draw"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: if index == 0 {
                            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: if index == 0 {
                            wgpu::LoadOp::Clear(1.0)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &camera_group, &[]);
            pass.set_bind_group(1, &texture_group, &[]);
            if let Some(group) = &zshape_group {
                pass.set_bind_group(2, group, &[]);
            }
            pass.set_vertex_buffer(0, instances.slice(..));
            pass.draw(0..6, 0..1);
        }

        let readback =
            PendingBgra8Readback::encode(&self.device, &mut encoder, &color, FORMAT, SIDE, SIDE)
                .expect("encode offscreen readback");
        let submission = self.queue.submit([encoder.finish()]);
        readback
            .finish(&self.device, submission, Duration::from_secs(10))
            .expect("read actual GPU pixels")
            .chunks_exact(4)
            .map(|bgra| [bgra[2], bgra[1], bgra[0], bgra[3]])
            .collect()
    }
}

fn extent() -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: SIDE,
        height: SIDE,
        depth_or_array_layers: 1,
    }
}

fn color_descriptor() -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: None,
        size: extent(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    }
}

// Derive offsets from the actual repr(C) structs, including nested DrawState.
// Locations/formats are the existing production vertex ABI in batch.rs.
macro_rules! attribute {
    ($location:expr, $format:ident, $($field:tt)+) => {
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::$format,
            offset: offset_of!(SpriteInstance, $($field)+) as u64,
            shader_location: $location,
        }
    };
}

const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 14] = [
    attribute!(0, Float32x2, position),
    attribute!(1, Float32x2, size),
    attribute!(2, Float32x2, uv_origin),
    attribute!(3, Float32x2, uv_size),
    attribute!(4, Float32, depth),
    attribute!(5, Float32x3, tint),
    attribute!(6, Float32, alpha),
    attribute!(7, Uint32, draw_state.remap_row),
    attribute!(8, Uint32, draw_state.fx_flags),
    attribute!(9, Float32x4, draw_state.fx_params),
    attribute!(10, Float32x4, draw_state.effect_tint),
    attribute!(11, Float32, z_adjust),
    attribute!(12, Uint32, z_gradient),
    attribute!(13, Float32x2, zshape_origin),
];

#[test]
#[ignore = "requires a wgpu adapter; run explicitly"]
fn terrain_and_building_depth_hide_only_behind_pixels_and_preserve_holes() {
    let gpu = Gpu::new();
    // The terrain depth here represents native ground row 8; flat SHP with
    // z_adjust=0 is behind it. This fixture is deliberately below the depth
    // axis clamp and samples interior pixels (no raster-edge ambiguity).
    let mut terrain = Layer::solid(Shader::Terrain, RED, -16.0);
    terrain.z_bytes.fill(8);
    // The far lane represents row 0, allowing the sprite through. This makes
    // admission depend on the sampled TMP Z byte, not just instance depth.
    for y in 0..SIDE {
        terrain.z_bytes[(y * SIDE + 5) as usize] = 16;
    }
    terrain.hole(3, 3);
    let behind = Layer::solid(Shader::SpriteRead, BLUE, 0.0);
    let pixels = gpu.render(&[terrain, behind]);
    assert_eq!(
        pixels[(3 * SIDE + 3) as usize],
        BLUE,
        "terrain alpha hole wrote depth"
    );
    assert_eq!(
        pixels[(3 * SIDE + 4) as usize],
        RED,
        "behind sprite passed terrain Z"
    );
    assert_eq!(
        pixels[(3 * SIDE + 5) as usize],
        BLUE,
        "TMP per-pixel depth byte was ignored"
    );

    // The high-bridge atlas stores row bytes with the opposite sign from TMP.
    let mut bridge = Layer::solid(Shader::Terrain, RED, 0.0);
    bridge.z_bytes.fill(8);
    bridge.instance.draw_state.fx_params[3] = -1.0;
    let pixels = gpu.render(&[bridge, Layer::solid(Shader::SpriteRead, BLUE, 0.0)]);
    assert!(
        pixels.iter().all(|pixel| *pixel == RED),
        "bridge row atlas sign failed to occlude the behind sprite"
    );

    // A nearer read-only sprite may change color, but must not prevent a
    // later, intermediate-depth sprite from drawing (native unit blitters
    // 0x494B60/0x497FD0 test existing Z without writing it).
    let pixels = gpu.render(&[
        Layer::solid(Shader::Terrain, RED, -8.0),
        Layer::solid(Shader::SpriteRead, GREEN, -24.0),
        Layer::solid(Shader::SpriteRead, BLUE, -12.0),
    ]);
    assert!(
        pixels.iter().all(|pixel| *pixel == BLUE),
        "read-only sprite wrote Z or front sprite was rejected"
    );

    let mut building = Layer::solid(Shader::SpriteWrite, GREEN, -24.0);
    building.hole(3, 3);
    let pixels = gpu.render(&[
        Layer::solid(Shader::Terrain, RED, -8.0),
        building,
        Layer::solid(Shader::SpriteRead, BLUE, -12.0),
    ]);
    assert_eq!(
        pixels[(3 * SIDE + 3) as usize],
        BLUE,
        "building alpha hole wrote depth"
    );
    assert_eq!(
        pixels[(3 * SIDE + 4) as usize],
        GREEN,
        "building did not write Z"
    );

    // Equality differs between TMP and SHP: terrain can replace equal Z,
    // while a sprite at the same native row must fail the strict test.
    let pixels = gpu.render(&[
        Layer::solid(Shader::Terrain, RED, -8.0),
        Layer::solid(Shader::Terrain, GREEN, -8.0),
        Layer::solid(Shader::SpriteRead, BLUE, -5.0),
    ]);
    assert_eq!(
        pixels[(3 * SIDE + 3) as usize],
        GREEN,
        "equal sprite depth must fail after equal terrain depth passes"
    );
}

#[test]
#[ignore = "requires a wgpu adapter; run explicitly"]
fn building_zshape_uses_constant_raw_seed_and_signed_shape_bytes() {
    let gpu = Gpu::new();
    // CC_Draw_Shape 0x437C39..0x437C72 / 0x437E67..0x437EA7:
    // BUILDNGZ takes the raw seed DEFAULT_Z-height-screen_top+1+z_adjust,
    // then subtracts the signed shape byte. It does not walk the gradient.
    // Eight rows at top 0/height 8/z_adjust 0 have a constant ground row 7.
    let mut building = Layer::solid(Shader::SpriteWrite, BLUE, 0.0);
    building.instance.z_gradient = ZGradient::Vertical as u32 | Z_GRADIENT_ZSHAPE_FLAG;
    building.z_bytes.fill(128);
    for y in 0..SIDE {
        building.z_bytes[(y * SIDE + 1) as usize] = 129;
        building.z_bytes[(y * SIDE + 6) as usize] = 127;
    }
    let pixels = gpu.render(&[Layer::solid(Shader::Terrain, RED, -7.0), building]);
    for y in 0..SIDE {
        assert_eq!(
            pixels[(y * SIDE + 1) as usize],
            BLUE,
            "+1 zshape lane must be nearer at every row {y}"
        );
        assert_eq!(
            pixels[(y * SIDE + 3) as usize],
            RED,
            "neutral zshape lane must fail equality at every row {y}"
        );
        assert_eq!(
            pixels[(y * SIDE + 6) as usize],
            RED,
            "-1 zshape lane must be farther at every row {y}"
        );
    }
}

#[test]
#[ignore = "requires a wgpu adapter; run explicitly"]
fn second_shape_clips_color_and_depth_and_seeds_from_intersected_bottom() {
    let gpu = Gpu::new();
    // A shape at (2,-2) intersects the 8x8 stored body at x=2..7, y=0..5.
    // These are synthetic regression fixtures for the binary-derived shape
    // intersection contract, not native-executed parity goldens.
    let clipped = |extended: bool| {
        let mut body = Layer::solid(Shader::SpriteWrite, BLUE, 0.0);
        body.instance.zshape_origin = [2.0, -2.0];
        body.instance.z_gradient = ZGradient::Vertical as u32
            | Z_GRADIENT_ZSHAPE_CLIP_FLAG
            | if extended { Z_GRADIENT_ZSHAPE_FLAG } else { 0 };
        body.z_bytes.fill(128);
        body
    };

    // The intersected bottom is row 6, giving extended raw-seed ground row 5.
    // At equality only the signed +1 lane (shape x=1 -> world x=3) can pass.
    let mut body = clipped(true);
    for y in 0..SIDE {
        body.z_bytes[(y * SIDE + 1) as usize] = 129;
    }
    let pixels = gpu.render(&[Layer::solid(Shader::Terrain, RED, -5.0), body]);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let expected = if x == 3 && y < 6 { BLUE } else { RED };
            assert_eq!(
                pixels[(y * SIDE + x) as usize],
                expected,
                "extended shape intersection/seed at ({x},{y})"
            );
        }
    }

    // Raw SHP uses the ordinary vertical gradient after intersection. For
    // the six-row rect, the first three rows have ground row 6, the next
    // three row 5. Shape bytes must not be subtracted on this dispatch.
    let mut raw = clipped(false);
    raw.z_bytes.fill(255);
    let pixels = gpu.render(&[Layer::solid(Shader::Terrain, RED, -5.0), raw]);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let expected = if x >= 2 && y < 3 { BLUE } else { RED };
            assert_eq!(
                pixels[(y * SIDE + x) as usize],
                expected,
                "raw frame must clip, keep ordinary gradient, and ignore shape byte at ({x},{y})"
            );
        }
    }

    // Probe depth separately: an intermediate-depth terrain draw replaces
    // the background outside the shape, but cannot replace its opaque body.
    // A discarded color fragment must not leave an invisible depth blocker.
    let pixels = gpu.render(&[
        Layer::solid(Shader::Terrain, RED, 0.0),
        clipped(true),
        Layer::solid(Shader::Terrain, GREEN, -4.0),
    ]);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let expected = if x >= 2 && y < 6 { BLUE } else { GREEN };
            assert_eq!(
                pixels[(y * SIDE + x) as usize],
                expected,
                "shape-outside fragment wrote depth at ({x},{y})"
            );
        }
    }
}

#[test]
#[ignore = "requires a wgpu adapter; run explicitly"]
fn depth_sprite_preserves_batch_tint_and_opacity() {
    let gpu = Gpu::new();
    let mut ordinary = Layer::solid(Shader::Batch, [120, 160, 200, 255], -8.0);
    ordinary.instance.tint = [0.5, 1.3, 0.75];
    ordinary.instance.alpha = 0.75;
    ordinary.instance.draw_state = DrawState {
        effect_tint: [1.4, 0.65, 1.1, 1.0],
        fx_params: [0.6, 0.0, 1.0, 0.0],
        ..Default::default()
    };
    let baseline = gpu.render(std::slice::from_ref(&ordinary));
    ordinary.shader = Shader::SpriteRead;
    let depth_read = gpu.render(std::slice::from_ref(&ordinary));
    ordinary.shader = Shader::SpriteWrite;
    let depth_write = gpu.render(std::slice::from_ref(&ordinary));
    for (index, expected) in baseline.iter().enumerate() {
        for actual in [depth_read[index], depth_write[index]] {
            for channel in 0..4 {
                assert!(
                    actual[channel].abs_diff(expected[channel]) <= 1,
                    "depth pipeline changed batch color/opacity at pixel {index}: {actual:?} != {expected:?}"
                );
            }
        }
    }
    // Ensure the test isn't vacuously comparing empty images.
    assert!(
        baseline
            .iter()
            .all(|pixel| pixel[0] > 10 && pixel[1] > 10 && pixel[2] > 10)
    );
}
