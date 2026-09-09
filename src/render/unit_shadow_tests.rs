use super::super::{BatchRenderer, CachedUnitSprite, UnitAtlas, VxlLayer};
use super::*;

#[test]
#[ignore = "requires extracted original parts in VERA20K_SHADOW_PROBE_DIR"]
fn stock_shadow_mask_uses_production_part_pixels_and_offsets() {
    use crate::assets::{hva_file::HvaFile, vpl_file::VplFile, vxl_file::VxlFile};
    use crate::render::vxl_raster::{self, VxlRenderParams};
    let root = std::path::PathBuf::from(
        std::env::var_os("VERA20K_SHADOW_PROBE_DIR").expect("set rendering-parity root"),
    );
    let vpl = VplFile::from_bytes(
        &std::fs::read(root.join("grizzly-raster-proof/extract/voxels.vpl")).unwrap(),
    )
    .unwrap();
    for (model, folder) in [("GTNK", "aligned-native"), ("HTNK", "rhino-neutral-native")] {
        for step in (0..32u8).step_by(4) {
            let mut body = vec![0; 65536];
            for suffix in ["", "TUR", "BARL"] {
                let base = if model == "GTNK" && suffix.is_empty() {
                    root.join("grizzly-raster-proof/extract/gtnk")
                } else {
                    root.join("grizzly-part-composition/extract")
                        .join(format!("{model}{suffix}"))
                };
                let vxl = VxlFile::from_bytes(&std::fs::read(base.with_extension("VXL")).unwrap())
                    .unwrap();
                let hva = HvaFile::from_bytes(&std::fs::read(base.with_extension("HVA")).unwrap())
                    .unwrap();
                let sprite = vxl_raster::render_vxl(
                    &vxl,
                    Some(&hva),
                    &VxlRenderParams {
                        facing: step * 8,
                        ..Default::default()
                    },
                    Some(&vpl),
                );
                // Retail GTNK/HTNK TurretOffset is zero in these static scenes.
                // Each production part's own native/padded offset is its anchor.
                composite_mask_part(
                    &mut body,
                    &sprite.palette_indices,
                    sprite.width,
                    sprite.height,
                    [128 + sprite.offset_x as i32, 128 + sprite.offset_y as i32],
                );
            }
            let native = std::fs::read(
                root.join("grizzly-part-composition")
                    .join(folder)
                    .join(format!("{step:02}.bin")),
            )
            .unwrap();
            assert_eq!(
                body.iter()
                    .zip(native)
                    .filter(|(a, b)| (**a != 0) != (*b != 0))
                    .count(),
                0,
                "{model} {step} all body mask pixels"
            );
        }
    }
    eprintln!(
        "16 original Unit composed masks match current production part offsets/nonzero pixels"
    );
}

fn sample_indices(
    gpu: &crate::render::terrain_draw_gpu_tests::Gpu,
    atlas: &UnitAtlas,
    key: &UnitSpriteKey,
) -> Vec<u8> {
    let entry = atlas.get(key).unwrap();
    let texture = &atlas.pages[entry.page].texture;
    let width = texture.width;
    let height = texture.height;
    let shader=gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("atlas index readback"),source:wgpu::ShaderSource::Wgsl("@group(0) @binding(0) var t:texture_2d<u32>; @group(0) @binding(1) var<storage,read_write> output:array<u32>; @compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) p:vec3<u32>) { let d=textureDimensions(t); if p.x<d.x && p.y<d.y { output[p.y*d.x+p.x]=textureLoad(t,vec2<i32>(p.xy),0).r; } }".into())});
    let pipeline = gpu
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let out = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(width * height * 4),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: out.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: out.as_entire_binding(),
            },
        ],
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
    }
    encoder.copy_buffer_to_buffer(&out, 0, &read, 0, out.size());
    gpu.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    rx.recv().unwrap().unwrap();
    let data = read.slice(..).get_mapped_range();
    let mut result = Vec::new();
    let x = (entry.uv_origin[0] * width as f32).round() as usize;
    let y = (entry.uv_origin[1] * height as f32).round() as usize;
    for dy in 0..entry.pixel_size[1] as usize {
        for dx in 0..entry.pixel_size[0] as usize {
            let p = ((y + dy) * width as usize + x + dx) * 4;
            result.push(u32::from_le_bytes(data[p..p + 4].try_into().unwrap()) as u8);
        }
    }
    result
}

#[test]
#[ignore = "requires GPU; actual atlas packing/upload, mask first fill/hit and repack"]
fn shadow_first_fill_hits_and_atlas_growth_keep_payloads() {
    let gpu = crate::render::terrain_draw_gpu_tests::Gpu::new();
    let batch = BatchRenderer::new_with_device(
        &gpu.device,
        &gpu.queue,
        wgpu::TextureFormat::Bgra8UnormSrgb,
    );
    let make = |layer, pixels: Vec<u8>, width, height, offset: [f32; 2]| CachedUnitSprite {
        key: UnitSpriteKey {
            type_id: "generated".into(),
            facing: 0,
            layer,
            frame: 0,
            slope_type: 0,
        },
        pixels,
        width,
        height,
        offset_x: offset[0],
        offset_y: offset[1],
        native_draw_bounds: Some([
            offset[0] as i32,
            offset[1] as i32,
            width as i32,
            height as i32,
        ]),
    };
    let cache = vec![
        make(
            VxlLayer::Body,
            vec![33, 0, 33, 0, 0, 33, 0, 33],
            4,
            2,
            [-2., -1.],
        ),
        make(VxlLayer::Shadow, vec![1; 24], 6, 4, [-3., -2.]),
    ];
    let shadow_key = cache[1].key.clone();
    let body_key = cache[0].key.clone();
    let mut atlas = super::super::pack_sprites_on_device(
        &gpu.device,
        &gpu.queue,
        &batch,
        &cache,
        Default::default(),
    )
    .unwrap();
    atlas.rendered_cache = cache;
    let body = *atlas.get(&body_key).unwrap();
    assert!(atlas.prepare_native_shadow(&gpu.queue, &shadow_key, [(body, [0., 0.])]));
    let expected = vec![
        1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 1, 1, 1, 1,
    ];
    assert_eq!(sample_indices(&gpu, &atlas, &shadow_key), expected);
    let bomb = std::iter::from_fn(|| -> Option<(UnitSpriteEntry, [f32; 2])> {
        panic!("cache hit read new body")
    });
    assert!(atlas.prepare_native_shadow(&gpu.queue, &shadow_key, bomb));
    // Transfer to a genuinely different shelf layout. The old page remains
    // live and must retain the payload that an already queued draw references.
    let retained = atlas.shadow_masks.borrow().clone();
    let mut cache = std::mem::take(&mut atlas.rendered_cache);
    cache.push(make(VxlLayer::Turret, vec![57; 320], 40, 8, [-20., -4.]));
    let mut grown = super::super::pack_sprites_on_device(
        &gpu.device,
        &gpu.queue,
        &batch,
        &cache,
        Default::default(),
    )
    .unwrap();
    grown.rendered_cache = cache;
    assert_ne!(
        atlas.get(&shadow_key).unwrap().uv_origin,
        grown.get(&shadow_key).unwrap().uv_origin
    );
    grown.restore_shadow_masks(&gpu.queue, retained);
    assert!(grown.prepare_native_shadow(&gpu.queue, &shadow_key, []));
    assert_eq!(sample_indices(&gpu, &grown, &shadow_key), expected);
    assert_eq!(sample_indices(&gpu, &atlas, &shadow_key), expected);
    assert_eq!(
        sample_indices(&gpu, &grown, &body_key),
        vec![33, 0, 33, 0, 0, 33, 0, 33]
    );
    let start = std::time::Instant::now();
    for _ in 0..20_000 {
        assert!(grown.prepare_native_shadow(&gpu.queue, &shadow_key, []));
    }
    eprintln!(
        "20k stable shadow cache-hit CPU lookup: {:?}; no mask reads/uploads",
        start.elapsed()
    );
}
