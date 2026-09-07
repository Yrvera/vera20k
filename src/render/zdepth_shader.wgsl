// Per-pixel Z-depth shader for TMP terrain tiles and high-bridge bodies.
//
// Same vertex shader as batch_shader.wgsl. The fragment shader samples an R8
// depth atlas (binding 2) at the colour UV and derives the native Z of the
// pixel from the instance's canvas top row:
//
//   tiles   (`TMP_TileBlitter @ 0x00547CF0`): Z = base + zdata,
//           base = DefaultZ + YOrigin - diamond_top - tileH - tileH*level/2,
//           carried here as z_adjust = draw_offset.y - tileH - 15*level and
//           z_sign = +1 (zdata added: higher Z-data is farther);
//   bridges (`CellClass::DrawOverlay_Body @ 0x0047F6A0` through the extended
//           blitter with gradient entry 0): Z = seed - row, carried as
//           z_adjust = -15*(level+4) - 2 and z_sign = -1 with the atlas
//           storing the row index.
//
// depth = 1 - (row - world_origin_y) / world_height, where row is the ground
// row the Z stands for: row = canvas_top - (z_adjust + z_sign * byte). One
// native Z unit is exactly one world pixel row, shared with the sprite paths.

struct Camera {
    screen_size: vec2f,
    camera_pos: vec2f,
    zoom: f32,
    world_origin_y: f32,
    world_height: f32,
    pad1: f32,
};
@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var t_sprite: texture_2d<f32>;
@group(1) @binding(1) var s_sprite: sampler;
@group(1) @binding(2) var t_zdepth: texture_2d<f32>;

struct Instance {
    @location(0) position: vec2f,
    @location(1) size: vec2f,
    @location(2) uv_origin: vec2f,
    @location(3) uv_size: vec2f,
    @location(4) depth: f32,
    @location(5) tint: vec3f,
    @location(6) alpha: f32,
    @location(9) fx_params: vec4f,
    @location(11) z_adjust: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
    @location(1) tint: vec3f,
    @location(2) @interpolate(flat) canvas_top: f32,
    @location(3) @interpolate(flat) z_adjust: f32,
    @location(4) @interpolate(flat) z_sign: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) idx: u32,
    instance: Instance,
) -> VertexOutput {
    var quad_pos = array<vec2f, 6>(
        vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0),
        vec2f(0.0, 1.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0),
    );
    var quad_uv = array<vec2f, 6>(
        vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0),
        vec2f(0.0, 1.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0),
    );

    let local: vec2f = quad_pos[idx];
    let is_zoomed: bool = abs(camera.zoom - 1.0) >= 0.001;
    let pad: f32 = select(0.0, 0.5 / camera.zoom, is_zoomed);
    let raw_pos: vec2f = (instance.position - vec2f(pad, pad) + local * (instance.size + vec2f(pad * 2.0, pad * 2.0)) - camera.camera_pos) * camera.zoom;
    let pixel_pos: vec2f = select(raw_pos, floor(raw_pos + vec2f(0.5, 0.5)), !is_zoomed);

    let clip_x: f32 = (pixel_pos.x / camera.screen_size.x) * 2.0 - 1.0;
    let clip_y: f32 = 1.0 - (pixel_pos.y / camera.screen_size.y) * 2.0;

    var output: VertexOutput;
    // Set position.z to 0.5 (midpoint) — frag_depth overrides the actual depth.
    output.position = vec4f(clip_x, clip_y, 0.5, 1.0);
    output.uv = instance.uv_origin + quad_uv[idx] * instance.uv_size;
    output.tint = instance.tint;
    output.canvas_top = instance.position.y;
    output.z_adjust = instance.z_adjust;
    // fx_params.w carries the Z-data sign (+1 tiles, -1 bridges); zero keeps +1.
    output.z_sign = select(1.0, -1.0, instance.fx_params.w < 0.0);
    return output;
}

struct FragOutput {
    @location(0) color: vec4f,
    @builtin(frag_depth) depth: f32,
};


// --- Map-light tint in the original's colour space -------------------------
// gamemd lights a palette entry by scaling its 8-bit RGB bytes: LightConvert's
// palette pass (FUN_00556090 -> FUN_007DE200 for RGB565) computes
// (byte * scale16) >> 16 with scale16 = light_milli * 65536 / 1000 (three LEA x5
// and a SHL 3 then the double 0.065536 at 0x007ED0B0) and clamps the product to
// 255 before packing. That multiply happens on the palette bytes, i.e. in
// sRGB-encoded space. These textures are sRGB-typed, so the sampled value is
// already linear; multiplying it by the tint here would apply the light in
// linear space, which reads as tint^(1/2.2) on screen (a 1.2 unit light became
// ~1.09). Re-encode, scale, clamp, decode. The RGB565 quantisation that
// follows natively is not modelled (DRIFT, sub-pixel colour).
fn srgb_encode(c: vec3f) -> vec3f {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3f(0.0)), vec3f(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3f(0.0031308));
}
fn srgb_decode(c: vec3f) -> vec3f {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3f(2.4));
    return select(hi, lo, c <= vec3f(0.04045));
}
fn palette_light(rgb_linear: vec3f, tint: vec3f) -> vec3f {
    return srgb_decode(clamp(srgb_encode(rgb_linear) * tint, vec3f(0.0), vec3f(1.0)));
}

@fragment
fn fs_main(input: VertexOutput) -> FragOutput {
    let color: vec4f = textureSample(t_sprite, s_sprite, input.uv);
    if (color.a < 0.01) {
        discard;
    }

    // R8 depth atlas: byte 0..255 stored as 0..1.
    let z_byte: f32 = round(textureSample(t_zdepth, s_sprite, input.uv).r * 255.0);
    let ground_row: f32 = input.canvas_top - (input.z_adjust + input.z_sign * z_byte);
    let world_height: f32 = max(camera.world_height, 1.0);
    let frag_depth: f32 = clamp(1.0 - (ground_row - camera.world_origin_y) / world_height, 0.001, 0.999);

    var output: FragOutput;
    output.color = vec4f(palette_light(color.rgb, input.tint), color.a);
    output.depth = frag_depth;
    return output;
}
