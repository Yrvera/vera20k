// Camera uniform: viewport size, scroll position, and depth parameters.
struct Camera {
    screen_size: vec2f,
    camera_pos: vec2f,
    // Zoom level: 1.0 = native, >1.0 = zoomed in, <1.0 = zoomed out.
    zoom: f32,
    pad0: f32,
};
@group(0) @binding(0) var<uniform> camera: Camera;

// Texture and sampler (nearest-neighbor for pixel art).
@group(1) @binding(0) var t_sprite: texture_2d<f32>;
@group(1) @binding(1) var s_sprite: sampler;

// Per-instance data from the instance buffer.
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
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
    @location(1) tint: vec3f,
    @location(2) alpha: f32,
    @location(3) @interpolate(flat) fx_flags: u32,
    @location(4) fx_params: vec4f,
    @location(5) effect_tint: vec4f,
};

@vertex
fn vs_main(
    @builtin(vertex_index) idx: u32,
    instance: Instance,
) -> VertexOutput {
    // Quad vertex positions: (0,0) top-left to (1,1) bottom-right.
    var quad_pos = array<vec2f, 6>(
        vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0),
        vec2f(0.0, 1.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0),
    );
    var quad_uv = array<vec2f, 6>(
        vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0),
        vec2f(0.0, 1.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0),
    );

    let local: vec2f = quad_pos[idx];
    // Screen-space pixel position of this vertex, scaled by zoom.
    // At native zoom (1.0) we pixel-snap to prevent sub-pixel seams between
    // adjacent tiles. At other zoom levels we skip the snap — floor() on
    // fractional screen positions causes 1px gaps between tiles because
    // adjacent world-space boundaries can round to different integers.
    // We also expand each quad by 0.5 screen pixels per side to eliminate
    // rasterization gaps at diamond edge midpoints (top-left fill rule).
    let is_zoomed: bool = abs(camera.zoom - 1.0) >= 0.001;
    let pad: f32 = select(0.0, 0.5 / camera.zoom, is_zoomed);
    let raw_pos: vec2f = (instance.position - vec2f(pad, pad) + local * (instance.size + vec2f(pad * 2.0, pad * 2.0)) - camera.camera_pos) * camera.zoom;
    let pixel_pos: vec2f = select(raw_pos, floor(raw_pos + vec2f(0.5, 0.5)), !is_zoomed);

    // Convert pixel coordinates to clip space.
    // Screen: (0,0) = top-left, (screen_size) = bottom-right.
    // Clip: (-1,-1) = bottom-left, (1,1) = top-right.
    let clip_x: f32 = (pixel_pos.x / camera.screen_size.x) * 2.0 - 1.0;
    let clip_y: f32 = 1.0 - (pixel_pos.y / camera.screen_size.y) * 2.0;

    var output: VertexOutput;
    output.position = vec4f(clip_x, clip_y, instance.depth, 1.0);
    output.uv = instance.uv_origin + quad_uv[idx] * instance.uv_size;
    output.tint = instance.tint;
    output.alpha = instance.alpha;
    output.fx_flags = instance.fx_flags;
    output.fx_params = instance.fx_params;
    output.effect_tint = instance.effect_tint;
    return output;
}

fn apply_fx(color: vec4f, _flags: u32, params: vec4f, effect_tint: vec4f) -> vec4f {
    // Original location: `RA2-GAME.EXE-IDB` canon,
    // `rendering.drawStateEffects.ra2yr.json`; this representation-neutral
    // branch mirrors the voxel shader so SHP and VXL share one DrawState ABI.
    // YR resolves selector opacity and invulnerability brightness before either
    // SHP or VXL submission. EMP and mirror deliberately remain no-op residuals.
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
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let color: vec4f = textureSample(t_sprite, s_sprite, input.uv);
    // Discard fully transparent pixels so they don't write to the depth buffer.
    // Without this, transparent regions of sprite quads would occlude objects behind them.
    if (color.a < 0.01) {
        discard;
    }
    // Map lighting happens before the shared DrawState effect branch, matching
    // the voxel fragment path. Alpha 1.0 = opaque; no draw state changes order.
    return apply_fx(
        vec4f(palette_light(color.rgb, input.tint * input.effect_tint.rgb), color.a * input.alpha),
        input.fx_flags,
        input.fx_params,
        input.effect_tint,
    );
}
