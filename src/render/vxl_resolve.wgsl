// VXL compute resolve shader — one thread per pixel.
//
// Reads the packed u32 from the atomic framebuffer, unpacks color_index and
// VPL page, performs the VPL remap (post-VPL palette index lookup), writes
// the resulting palette index byte to the output buffer (low byte of u32),
// and clears the atomic FB entry to 0xFFFFFFFF for the next sprite.
//
// Output is the post-VPL, pre-house-remap palette index. House remap and
// theater palette lookup happen at fragment-shader time, not here.

struct ResolveParams {
    fb_width: u32,
    fb_height: u32,
    pixel_count: u32,
    vpl_section_count: u32,
};

@group(0) @binding(0) var<uniform> params: ResolveParams;
// Atomic framebuffer: packed (depth_u16 << 16) | (page << 8) | color_index.
@group(0) @binding(1) var<storage, read_write> atomic_fb: array<atomic<u32>>;
// VPL remap table: pages packed 4 entries per u32.
// Layout: vpl_table[(page * 256 + color_index) / 4], byte (page * 256 + color_index) % 4.
@group(0) @binding(2) var<storage, read> vpl_table: array<u32>;
// Output palette-index buffer: one u32 per pixel, low byte = post-VPL palette
// index (high 24 bits zero). Byte 0 = transparent.
@group(0) @binding(3) var<storage, read_write> output_palette_indices: array<u32>;

@compute @workgroup_size(256)
fn resolve_main(@builtin(global_invocation_id) gid: vec3u) {
    let idx = gid.x;
    if (idx >= params.pixel_count) {
        return;
    }

    // Read packed value and clear for next sprite.
    let packed = atomicLoad(&atomic_fb[idx]);
    atomicStore(&atomic_fb[idx], 0xFFFFFFFFu);

    if (packed == 0xFFFFFFFFu) {
        // No voxel hit — transparent pixel (byte 0).
        output_palette_indices[idx] = 0u;
        return;
    }

    let color_index = packed & 0xFFu;
    let page = (packed >> 8u) & 0xFFu;

    // VPL remap: shaded_index = vpl_table[page * 256 + color_index].
    // Clamp page to valid range.
    let clamped_page = min(page, params.vpl_section_count - 1u);
    let vpl_offset = clamped_page * 256u + color_index;
    let vpl_word_idx = vpl_offset / 4u;
    let vpl_byte_idx = vpl_offset % 4u;
    let vpl_word = vpl_table[vpl_word_idx];
    let shaded_index = (vpl_word >> (vpl_byte_idx * 8u)) & 0xFFu;

    // Write the post-VPL palette index. Byte 0 = transparent (gamemd
    // visibility-map invariant); the fragment shader discards byte-0 pixels.
    output_palette_indices[idx] = shaded_index;
}
