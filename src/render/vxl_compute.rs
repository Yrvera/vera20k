//! GPU replay of the shared native VXL visibility writes.
//!
//! CPU preparation owns x87 geometry, encoded spans and VPL results. A paint
//! ordinal selects the last native store independently of invocation order,
//! palette value or floating depth. Each draw has one VXL's native center.

use crate::render::vxl_raster::{PreparedDraw, VxlSprite};

const SURFACE_BYTES: u64 = 256 * 256 * 4;
const MAX_PAINT_WRITES: usize = 0x00ff_ffff;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ReplayParams {
    count: u32,
    crop_x: u32,
    crop_y: u32,
    width: u32,
    height: u32,
    _padding: [u32; 3],
}

/// Atlas-build replay buffers. Per-frame rendering samples the completed
/// atlas and does not construct or replay these command streams.
pub struct VxlComputeRenderer {
    paint: wgpu::ComputePipeline,
    resolve: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    atomic_fb: wgpu::Buffer,
    output: wgpu::Buffer,
    staging: wgpu::Buffer,
}

impl VxlComputeRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vxl_native.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("vxl_native.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Native VXL replay layout"),
            entries: &[
                buffer_binding(0, wgpu::BufferBindingType::Uniform),
                buffer_binding(1, wgpu::BufferBindingType::Storage { read_only: true }),
                buffer_binding(2, wgpu::BufferBindingType::Storage { read_only: false }),
                buffer_binding(3, wgpu::BufferBindingType::Storage { read_only: false }),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Native VXL replay pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let buffer = |label, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: SURFACE_BYTES,
                usage,
                mapped_at_creation: false,
            })
        };
        Self {
            paint: pipeline("paint_main"),
            resolve: pipeline("resolve_main"),
            layout,
            atomic_fb: buffer(
                "Native VXL winners",
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            ),
            output: buffer(
                "Native VXL cropped indices",
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            ),
            staging: buffer(
                "Native VXL readback",
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            ),
        }
    }

    /// A checked 24-bit ordinal plus 8-bit palette byte selects the last store.
    /// Untouched addresses are zero; a zero-color store has a nonzero ordinal
    /// and erases earlier stores. Unsupported device/packing limits return
    /// None so the caller can replay the identical draw on CPU.
    pub(crate) fn render_native(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draw: &PreparedDraw,
    ) -> Option<VxlSprite> {
        let [offset_x, offset_y, crop_x, crop_y, width, height] = draw.rect;
        if crop_x < 0
            || crop_y < 0
            || width <= 0
            || height <= 0
            || crop_x.checked_add(width)? > 256
            || crop_y.checked_add(height)? > 256
            || draw.writes.len() > MAX_PAINT_WRITES
        {
            return None;
        }
        let count = draw.writes.len() as u32;
        let groups = count.div_ceil(64);
        let limits = device.limits();
        if groups > limits.max_compute_workgroups_per_dimension
            || (draw.writes.len().max(1) * 8) as u64
                > u64::from(limits.max_storage_buffer_binding_size)
        {
            return None;
        }
        let mut commands: Vec<[u32; 2]> = draw
            .writes
            .iter()
            .map(|write| [u32::from(write.address), u32::from(write.color)])
            .collect();
        if commands.is_empty() {
            commands.push([0; 2]);
        }
        use wgpu::util::DeviceExt;
        let command_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Native VXL ordered writes"),
            contents: bytemuck::cast_slice(&commands),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let params = ReplayParams {
            count,
            crop_x: crop_x as u32,
            crop_y: crop_y as u32,
            width: width as u32,
            height: height as u32,
            _padding: [0; 3],
        };
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Native VXL replay parameters"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Native VXL replay"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: command_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.atomic_fb.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.output.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Native VXL replay encoder"),
        });
        encoder.clear_buffer(&self.atomic_fb, 0, None);
        if groups > 0 {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Native VXL ordered winner pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.paint);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(groups, 1, 1);
        }
        // Separate passes order completed atomics before crop extraction.
        // https://www.w3.org/TR/WGSL/#atomic-builtin-functions
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Native VXL crop pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.resolve);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((width as u32 * height as u32).div_ceil(64), 1, 1);
        }
        let byte_size = (width * height) as u64 * 4;
        encoder.copy_buffer_to_buffer(&self.output, 0, &self.staging, 0, byte_size);
        queue.submit(std::iter::once(encoder.finish()));
        let slice = self.staging.slice(..byte_size);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        if !matches!(receiver.recv(), Ok(Ok(()))) {
            self.staging.unmap();
            return None;
        }
        let mapped = slice.get_mapped_range();
        let palette_indices = mapped.chunks_exact(4).map(|pixel| pixel[0]).collect();
        drop(mapped);
        self.staging.unmap();
        Some(VxlSprite {
            palette_indices,
            depth: vec![],
            width: width as u32,
            height: height as u32,
            offset_x: offset_x as f32,
            offset_y: offset_y as f32,
        })
    }
}

fn buffer_binding(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a real GPU; native synthetic visibility and metadata readback"]
    fn native_gpu_matches_executable_fixtures() {
        use crate::assets::{hva_file::HvaFile, vpl_file::VplFile, vxl_file::VxlFile};
        use crate::render::vxl_raster::{VxlRenderParams, prepare_native_draw};
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("explicit native raster GPU test requires an adapter");
        eprintln!("Native VXL GPU adapter: {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let mut renderer = VxlComputeRenderer::new(&device);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tools/voxel_oracle/raster_fixtures.json"
        ))
        .unwrap();
        let bytes = |hex: &str| {
            hex.as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>()
        };
        let mut cases = 0;
        for model in fixture["models"].as_array().unwrap() {
            let vxl = VxlFile::from_bytes(&bytes(model["vxl"].as_str().unwrap())).unwrap();
            let hva = model["hva"]
                .as_str()
                .map(|hex| HvaFile::from_bytes(&bytes(hex)).unwrap());
            let vpl = VplFile::from_bytes(&bytes(model["vpl"].as_str().unwrap())).unwrap();
            for case in model["cases"].as_array().unwrap() {
                let params = VxlRenderParams {
                    facing: case["step"].as_u64().unwrap() as u8 * 8,
                    ..Default::default()
                };
                let draw = prepare_native_draw(&vxl, hva.as_ref(), &params, Some(&vpl)).unwrap();
                let mut expected = vec![0u8; 65536];
                for pixel in case["pixels"].as_array().unwrap() {
                    expected[pixel[0].as_u64().unwrap() as usize] =
                        pixel[1].as_u64().unwrap() as u8;
                }
                let actual = renderer.render_native(&device, &queue, &draw).unwrap();
                assert_eq!(
                    actual.palette_indices,
                    draw.crop_indices(&expected).unwrap(),
                    "{} facing {}",
                    model["name"],
                    params.facing
                );
                assert_eq!(
                    actual.palette_indices,
                    draw.render_cpu().unwrap().palette_indices
                );
                let rect = case["rect"].as_array().unwrap();
                assert_eq!(
                    [
                        actual.offset_x as i32,
                        actual.offset_y as i32,
                        actual.width as i32,
                        actual.height as i32
                    ],
                    [
                        rect[0].as_i64().unwrap() as i32,
                        rect[1].as_i64().unwrap() as i32,
                        rect[4].as_i64().unwrap() as i32,
                        rect[5].as_i64().unwrap() as i32
                    ]
                );
                cases += 1;
            }
        }
        assert_eq!(cases, 32);
        eprintln!(
            "Native GPU readback: {cases} executable fixtures match every cropped palette byte"
        );
        if let Some(root) = std::env::var_os("VERA20K_VOXEL_PROBE_DIR") {
            let root = std::path::PathBuf::from(root);
            let vxl = VxlFile::from_bytes(&std::fs::read(root.join("extract/gtnk.vxl")).unwrap())
                .unwrap();
            let hva = HvaFile::from_bytes(&std::fs::read(root.join("extract/gtnk.hva")).unwrap())
                .unwrap();
            let vpl = VplFile::from_bytes(&std::fs::read(root.join("extract/voxels.vpl")).unwrap())
                .unwrap();
            for step in 0..32u8 {
                let params = VxlRenderParams {
                    facing: step * 8,
                    ..Default::default()
                };
                let draw = prepare_native_draw(&vxl, Some(&hva), &params, Some(&vpl))
                    .expect("stock hull must use encoded native preparation");
                let native_dir = root.join("all-facings").join(format!("{step:02}"));
                let expected = std::fs::read(native_dir.join("gtnk-facing0-indexed.bin")).unwrap();
                let metadata: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(native_dir.join("probe-result.json")).unwrap(),
                )
                .unwrap();
                let raw = bytes(metadata["rect_raw"].as_str().unwrap());
                let rect: [i32; 6] = std::array::from_fn(|i| {
                    i32::from_le_bytes(raw[i * 4..i * 4 + 4].try_into().unwrap())
                });
                assert_eq!(draw.rect, rect);
                let actual = renderer.render_native(&device, &queue, &draw).unwrap();
                assert_eq!(
                    actual.palette_indices,
                    draw.crop_indices(&expected).unwrap(),
                    "stock GTNK GPU step {step}"
                );
                assert_eq!(
                    [
                        actual.offset_x as i32,
                        actual.offset_y as i32,
                        actual.width as i32,
                        actual.height as i32
                    ],
                    [rect[0], rect[1], rect[4], rect[5]]
                );
            }
            eprintln!("Stock GTNK GPU readback: 32 native hull buffers and sprite metadata match");
            for name in ["LCRF", "ZEP", "ORCA"] {
                let assets = root.join("stock-performance/extract");
                let native = root.join("stock-performance/native").join(name);
                if !native.exists() {
                    continue;
                }
                let vxl = VxlFile::from_bytes(
                    &std::fs::read(assets.join(format!("{name}.VXL"))).unwrap(),
                )
                .unwrap();
                let hva = HvaFile::from_bytes(
                    &std::fs::read(assets.join(format!("{name}.HVA"))).unwrap(),
                )
                .unwrap();
                for step in [0, 8, 12, 24] {
                    let params = VxlRenderParams {
                        facing: step * 8,
                        ..Default::default()
                    };
                    let draw = prepare_native_draw(&vxl, Some(&hva), &params, Some(&vpl))
                        .expect("stock core must use native spans");
                    let case: serde_json::Value = serde_json::from_slice(
                        &std::fs::read(native.join(format!("{step:02}.json"))).unwrap(),
                    )
                    .unwrap();
                    let rect: [i32; 6] =
                        std::array::from_fn(|i| case["rect"][i].as_i64().unwrap() as i32);
                    assert_eq!(draw.rect, rect);
                    let mut expected = vec![0u8; 65536];
                    for pixel in case["pixels"].as_array().unwrap() {
                        expected[pixel[0].as_u64().unwrap() as usize] =
                            pixel[1].as_u64().unwrap() as u8;
                    }
                    let actual = renderer.render_native(&device, &queue, &draw).unwrap();
                    assert_eq!(
                        actual.palette_indices,
                        draw.crop_indices(&expected).unwrap(),
                        "stock {name} GPU step {step}"
                    );
                    assert_eq!(
                        actual.palette_indices,
                        draw.render_cpu().unwrap().palette_indices
                    );
                    assert_eq!(
                        [
                            actual.offset_x as i32,
                            actual.offset_y as i32,
                            actual.width as i32,
                            actual.height as i32
                        ],
                        [rect[0], rect[1], rect[4], rect[5]]
                    );
                }
                eprintln!(
                    "Stock {name} GPU readback: 4 native core buffers and sprite metadata match"
                );
            }
        }
    }
}
