// Voxel sprite fragment shader.
//
// Atlas tiles store post-VPL, pre-house-remap palette indices (R8Uint).
// At fragment time:
//   byte = textureLoad(atlas, uv);
//   if (byte == 0) discard;
//   if (16 <= byte < 32) → rgb = house_ramp[house_idx][byte - 16]
//   else                 → rgb = palette[byte]
//   color = apply_fx(color, fx_flags, fx_params, ic_tint);
//   return rgb * tint, alpha;
//
// Bind groups:
//   group 0: camera uniform
//   group 1: atlas (R8Uint)
//   group 2: palette (Rgba8Unorm) + house_ramp (Rgba8Unorm) + sampler

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
    @location(7) house_color_idx: u32,
    @location(8) fx_flags: u32,
    @location(9) fx_params: vec4f,
    @location(10) ic_tint: vec4f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) atlas_uv: vec2f,
    @location(1) tint: vec3f,
    @location(2) alpha: f32,
    @location(3) @interpolate(flat) house_color_idx: u32,
    @location(4) @interpolate(flat) fx_flags: u32,
    @location(5) fx_params: vec4f,
    @location(6) ic_tint: vec4f,
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
    out.house_color_idx = instance.house_color_idx;
    out.fx_flags = instance.fx_flags;
    out.fx_params = instance.fx_params;
    out.ic_tint = instance.ic_tint;
    return out;
}

fn apply_fx(color: vec4f, flags: u32, params: vec4f, ic: vec4f) -> vec4f {
    // Phase 1 stub: future phases (cloak/EMP/IC/warp) wire branches here.
    var c = color;
    if ((flags & 1u) != 0u) { c.a = c.a * params.x; }                // cloak
    if ((flags & 2u) != 0u) {                                        // EMP
        let luma = dot(c.rgb, vec3f(0.299, 0.587, 0.114));
        c = vec4f(mix(c.rgb, vec3f(luma), params.y), c.a);
    }
    if ((flags & 4u) != 0u) {                                        // iron curtain
        c = vec4f(mix(c.rgb, ic.rgb, ic.a), c.a);
    }
    if ((flags & 8u) != 0u) { c.a = c.a * params.w; }                // warp
    return c;
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
        let ramp_coord: vec2i = vec2i(i32(byte - 16u), i32(in.house_color_idx));
        rgb = textureLoad(house_ramp, ramp_coord, 0).rgb;
    } else {
        let palette_coord: vec2i = vec2i(i32(byte), 0);
        rgb = textureLoad(palette, palette_coord, 0).rgb;
    }

    var color: vec4f = vec4f(rgb * in.tint, in.alpha);
    color = apply_fx(color, in.fx_flags, in.fx_params, in.ic_tint);
    return color;
}
