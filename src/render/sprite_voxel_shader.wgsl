// Voxel sprite fragment shader.
//
// Atlas tiles store post-VPL, pre-house-remap palette indices (R8Uint).
// At fragment time:
//   byte = textureLoad(atlas, uv);
//   if (byte == 0) discard;
//   if (16 <= byte < 32) → rgb = house_ramp[house_idx][byte - 16]
//   else                 → rgb = palette[byte]
//   color = apply_fx(color, fx_flags, fx_params, effect_tint);
//   return palette_light(rgb, tint), alpha;  (tint applied in sRGB byte space)
//
// Bind groups:
//   group 0: camera uniform
//   group 1: atlas (R8Uint)
//   group 2: palette (Rgba8UnormSrgb) + house_ramp (Rgba8UnormSrgb) + sampler
//   (sRGB format → sampler returns linear RGB; the tint is applied after
//   re-encoding to sRGB, the space gamemd's LightConvert scales in)

struct Camera {
    screen_size: vec2f,
    camera_pos: vec2f,
    zoom: f32,
    pad0: f32,
};

@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var atlas: texture_2d<u32>;

@group(2) @binding(0) var palette: texture_2d<f32>;
@group(2) @binding(1) var house_ramp: texture_2d<f32>;
@group(2) @binding(2) var palette_sampler: sampler;

struct Instance {
    @location(0) position: vec2f,
    @location(1) size: vec2f,
    @location(2) uv_origin: vec2f,
    @location(3) uv_size: vec2f,
    @location(4) depth: f32,
    @location(5) tint: vec3f,
    @location(6) alpha: f32,
    @location(7) remap_row: u32,
    @location(8) fx_flags: u32,
    @location(9) fx_params: vec4f,
    @location(10) effect_tint: vec4f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) atlas_uv: vec2f,
    @location(1) tint: vec3f,
    @location(2) alpha: f32,
    @location(3) @interpolate(flat) remap_row: u32,
    @location(4) @interpolate(flat) fx_flags: u32,
    @location(5) fx_params: vec4f,
    @location(6) effect_tint: vec4f,
};

@vertex
fn vs_main(
    @builtin(vertex_index) idx: u32,
    instance: Instance,
) -> VertexOutput {
    // Quad vertices: 6 vertices forming 2 triangles (matches batch_shader.wgsl).
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
    let raw_pos: vec2f = (instance.position - vec2f(pad, pad)
        + local * (instance.size + vec2f(pad * 2.0, pad * 2.0))
        - camera.camera_pos) * camera.zoom;
    let pixel_pos: vec2f = select(raw_pos, floor(raw_pos + vec2f(0.5, 0.5)), !is_zoomed);

    // Convert pixel to clip space (matches batch_shader convention).
    let clip_x: f32 = (pixel_pos.x / camera.screen_size.x) * 2.0 - 1.0;
    let clip_y: f32 = -((pixel_pos.y / camera.screen_size.y) * 2.0 - 1.0);

    var out: VertexOutput;
    out.clip_position = vec4f(clip_x, clip_y, instance.depth, 1.0);
    out.atlas_uv = instance.uv_origin + quad_uv[idx] * instance.uv_size;
    out.tint = instance.tint;
    out.alpha = instance.alpha;
    out.remap_row = instance.remap_row;
    out.fx_flags = instance.fx_flags;
    out.fx_params = instance.fx_params;
    out.effect_tint = instance.effect_tint;
    return out;
}

fn apply_fx(color: vec4f, _flags: u32, params: vec4f, effect_tint: vec4f) -> vec4f {
    // Original location: `RA2-GAME.EXE-IDB` canon,
    // `rendering.drawStateEffects.ra2yr.json`; the same branch exists in the
    // SHP shader so representation does not affect active visual state.
    // Match the SHP path exactly: selector opacity plus independent YR
    // invulnerability brightness, with no inferred EMP or mirror styling.
    // Invulnerability/temporal brightness is folded into the gamma-space
    // light multiply by the caller (natively 0x0070E380 scales the brightness
    // argument itself, which selects the same LightConvert row); only opacity
    // is applied here.
    return vec4f(color.rgb, color.a * params.x);
}


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
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let atlas_size: vec2f = vec2f(textureDimensions(atlas));
    let atlas_coord: vec2i = vec2i(in.atlas_uv * atlas_size);
    let byte: u32 = textureLoad(atlas, atlas_coord, 0).r;

    // Color 0 = transparent (matches gamemd visibility-map invariant).
    if (byte == 0u) {
        discard;
    }

    // RGB substitution: bytes in [16, 32) sample the per-house ramp; all
    // others sample the theater palette directly.
    var rgb: vec3f;
    if (byte >= 16u && byte < 32u) {
        let ramp_coord: vec2i = vec2i(i32(byte - 16u), i32(in.remap_row));
        rgb = textureLoad(house_ramp, ramp_coord, 0).rgb;
    } else {
        let palette_coord: vec2i = vec2i(i32(byte), 0);
        rgb = textureLoad(palette, palette_coord, 0).rgb;
    }

    var color: vec4f = vec4f(palette_light(rgb, in.tint * in.effect_tint.rgb), in.alpha);
    color = apply_fx(color, in.fx_flags, in.fx_params, in.effect_tint);
    return color;
}
