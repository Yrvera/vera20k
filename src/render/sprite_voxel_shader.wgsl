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
    // Depth axis: depth = 1 - (row - world_origin_y) / world_height.
    world_origin_y: f32,
    world_height: f32,
    pad1: f32,
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
    @location(11) z_adjust: f32,
    @location(12) z_gradient: u32,
    // (top, height) of the composite blit rect this layer belongs to; zero
    // height means the layer's own quad. A turreted unit's hull, turret and
    // barrel are one native cache blit (`0x0073B140`), so they share one seed.
    @location(13) z_rect: vec2f,
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
    // World-pixel position of this fragment (unpadded quad).
    @location(7) world_pos: vec2f,
    // Blit rect top row and height in world pixels.
    @location(8) @interpolate(flat) rect_top_height: vec2f,
    @location(9) @interpolate(flat) z_adjust: f32,
    @location(10) @interpolate(flat) z_gradient: u32,
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
    // frag_depth overrides this.
    out.clip_position = vec4f(clip_x, clip_y, 0.5, 1.0);
    out.atlas_uv = instance.uv_origin + quad_uv[idx] * instance.uv_size;
    out.tint = instance.tint;
    out.alpha = instance.alpha;
    out.remap_row = instance.remap_row;
    out.fx_flags = instance.fx_flags;
    out.fx_params = instance.fx_params;
    out.effect_tint = instance.effect_tint;
    out.world_pos = instance.position + local * instance.size;
    out.rect_top_height = select(
        vec2f(instance.position.y, instance.size.y),
        instance.z_rect,
        instance.z_rect.y > 0.0,
    );
    out.z_adjust = instance.z_adjust;
    out.z_gradient = instance.z_gradient;
    return out;
}

// Native Z of row `row` (0 = top) of a blit; mirrors `native_z::sprite_row_z`
// and the copy in zsprite_shader.wgsl. The VXL cache blit walks the same
// gradient table (`VXL_CacheBlit @ 0x00707480` -> extended blitter).
fn native_row_z(entry: u32, screen_top: i32, height: i32, z_adjust: i32, row: i32) -> i32 {
    let default_z: i32 = 32768;
    var seed: i32;
    var accum: i32 = 0;
    var increment: i32;
    var threshold: i32;
    var step_dir: i32;
    if (entry == 2u) {
        increment = 1;
        threshold = 3;
        step_dir = 1;
        let raw: i32 = ((default_z - height - screen_top + 1) & 0xFFFF) + z_adjust;
        seed = (raw / 3) * 3 - height / 3;
        accum = 3 - (height % 3);
        if (accum == 3) {
            accum = 0;
            seed = seed + 1;
        }
    } else if (entry == 1u) {
        increment = 2;
        threshold = 3;
        step_dir = -1;
        let raw: i32 = ((default_z - screen_top) & 0xFFFF) + z_adjust;
        seed = (raw / 3) * 3;
    } else {
        increment = 1;
        threshold = 1;
        step_dir = -1;
        seed = ((default_z - screen_top) & 0xFFFF) + z_adjust;
    }
    let steps: i32 = (accum + max(row, 0) * increment) / threshold;
    return seed + step_dir * steps;
}

fn native_depth(in: VertexOutput) -> f32 {
    let camera_row: i32 = i32(round(camera.camera_pos.y));
    var rect_top: f32 = in.rect_top_height.x;
    var height: i32 = max(i32(round(in.rect_top_height.y)), 1);
    var gradient: u32 = in.z_gradient & 0xFFu;
    var z_adjust: i32 = i32(round(in.z_adjust));
    // Unit final composite 0x73B140: full-width upper h-16 then bottom 16,
    // seeded independently by Standard_SHP_blitter. A shared rectangle across
    // hull/turret/barrel makes their depth decision identical at every pixel.
    // Retain full quads/UVs so the split introduces no extra zoom-padding edge.
    if ((in.z_gradient & 0x400u) != 0u) {
        rect_top = round(rect_top);
        if (height > 16) {
            let boundary: f32 = rect_top + f32(height - 16);
            if (in.world_pos.y < boundary) {
                height = height - 16;
                gradient = 0u;
                z_adjust = z_adjust - 5;
            } else {
                rect_top = boundary;
                height = 16;
                gradient = 2u;
            }
        }
        // fx_params.w is this voxel body's tactical clip height in world
        // pixels, supplied from the actual scissor. Zero retains the whole
        // viewport for offscreen callers. Do not bake the sidebar/footer size
        // into this shader or infer it from atlas storage padding.
        let clip_height: f32 = select(
            camera.screen_size.y / camera.zoom,
            in.fx_params.w,
            in.fx_params.w > 0.0,
        );
        let bottom: f32 = min(rect_top + f32(height), f32(camera_row) + round(clip_height));
        rect_top = max(rect_top, f32(camera_row));
        height = i32(bottom - rect_top);
        if (height <= 0 || in.world_pos.y < rect_top || in.world_pos.y >= bottom) {
            discard;
        }
    }
    let screen_top: i32 = i32(round(rect_top)) - camera_row;
    let row: i32 = clamp(i32(floor(in.world_pos.y - rect_top)), 0, height - 1);
    let z: i32 = native_row_z(gradient, screen_top, height, z_adjust, row);
    let ground_row: f32 = f32(32768 - z + camera_row);
    let world_height: f32 = max(camera.world_height, 1.0);
    return clamp(1.0 - (ground_row - camera.world_origin_y) / world_height, 0.001, 0.999);
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


// RA2_DEBUG_DEPTH_VIEW (camera.pad1 > 0.5): depth as grey, wrapping every
// 128 world rows, so depth ordering can be read off a screenshot.
fn debug_depth_color(depth: f32) -> vec4f {
    let rows: f32 = (1.0 - depth) * max(camera.world_height, 1.0);
    let g: f32 = fract(rows / 128.0);
    return vec4f(g, g, g, 1.0);
}

struct FragOutput {
    @location(0) color: vec4f,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fs_main(in: VertexOutput) -> FragOutput {
    let atlas_size: vec2f = vec2f(textureDimensions(atlas));
    let atlas_coord: vec2i = vec2i(in.atlas_uv * atlas_size);
    let byte: u32 = textureLoad(atlas, atlas_coord, 0).r;

    // Color 0 = transparent (matches gamemd visibility-map invariant).
    if (byte == 0u) {
        discard;
    }

    var out: FragOutput;
    out.depth = native_depth(in);

    // Ground shadow stencil (FX_SHADOW = 1 << 6): every non-zero atlas byte
    // darkens the destination. The native darken blitter halves the encoded
    // 16-bit word; this pass alpha-blends black in linear space against an
    // sRGB target, so the alpha that halves an encoded value is
    // 1 - 0.5^2.2 = 0.782 rather than 0.5 (the bridge shadow's 128/255 is a
    // recorded lighter drift; this path takes the closer value).
    if ((in.fx_flags & 64u) != 0u) {
        out.color = vec4f(0.0, 0.0, 0.0, 0.782 * in.alpha);
        return out;
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
    out.color = color;
    if (camera.pad1 > 0.5) {
        out.color = debug_depth_color(out.depth);
    }
    return out;
}
