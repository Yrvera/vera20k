// Ordinary extended Terrain DrawIt 0071C304/0071C34E. Shared palette, native
// row walker, and SHP vertex projection precede this source in the module.
@group(2) @binding(0) var old_words: texture_2d<u32>;
@group(2) @binding(1) var old_depth: texture_2d<f32>;

struct TerrainOutput {
    @location(0) color: vec4f,
    @builtin(frag_depth) depth: f32,
};

fn terrain_pixel(input: VertexOutput, shadow: bool) -> TerrainOutput {
    let texel = vec2i(clamp(input.uv * vec2f(textureDimensions(source_indices)),
        vec2f(0.0), vec2f(textureDimensions(source_indices)) - 1.0));
    let index = textureLoad(source_indices, texel, 0).r;
    // Native compressed zero runs advance both destinations without touching
    // either one. The source index owns this stencil, including black colors.
    if index == 0u { discard; }
    let height = max(i32(round(input.rect_top_height.y)), 1);
    let top = i32(round(input.rect_top_height.x));
    let row = clamp(i32(floor(input.world_pos.y)) - top, 0, height - 1);
    let candidate = native_row_z(input.z_gradient, top - i32(round(camera.camera_pos.y + camera.native_z_origin_y)),
        height, i32(round(input.z_adjust)), row);
    let p = vec2i(input.position.xy);
    let previous = decoded_native_z(textureLoad(old_depth, p, 0).r);
    // 004990E0 / 00497390 compare signed candidate against zero-extended old
    // u16 BEFORE the store truncates. A repeated negative candidate can pass.
    if candidate >= previous { discard; }
    var output: TerrainOutput;
    output.depth = stored_native_depth(candidate);
    if shadow {
        let word = (textureLoad(old_words, p, 0).r >> 1u) & 0x7befu;
        let encoded = vec3f(f32(RETAIL_FIVE[(word >> 11u) & 31u]),
            f32(RETAIL_SIX[(word >> 5u) & 63u]), f32(RETAIL_FIVE[word & 31u])) / 255.0;
        output.color = vec4f(srgb_decode(encoded), 1.0);
    } else {
        let color = textureLoad(t_sprite, texel, 0);
        output.color = vec4f(resolve_palette(color.rgb, input.tint, vec3f(1.0), input.palette_light, index), 1.0);
    }
    return output;
}
@fragment
fn fs_body(input: VertexOutput) -> TerrainOutput { return terrain_pixel(input, false); }
@fragment
fn fs_shadow(input: VertexOutput) -> TerrainOutput { return terrain_pixel(input, true); }
