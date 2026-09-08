// Per-pixel Z-tested SHP sprite shader (gamemd `0x2E00` / `0x6E00` draws).
//
// Same vertex mapping as batch_shader.wgsl. The fragment stage recomputes the
// native 16-bit Z of its row from the blit rect's screen top, the gradient
// entry and the class Z term (`native_z.rs` is the Rust twin of this math),
// subtracts the BUILDNGZ z-shape byte when the instance carries one, and maps
// the integer Z onto the normalised depth axis: one native Z unit is one world
// pixel row. Two pipelines share it: depth compare Less without write (every
// object except buildings: leaves 0x00494b60 / 0x00497fd0) and Less with
// write (building bodies: leaves 0x004958d0 / 0x004990e0).

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

@group(1) @binding(0) var t_sprite: texture_2d<f32>;
@group(1) @binding(1) var s_sprite: sampler;

// BUILDNGZ.SHA frame 0, remapped by -0x41 and stored biased by +128 (R8Unorm).
@group(2) @binding(0) var t_zshape: texture_2d<f32>;

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
    @location(13) zshape_origin: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
    @location(1) tint: vec3f,
    @location(2) alpha: f32,
    @location(3) @interpolate(flat) fx_flags: u32,
    @location(4) fx_params: vec4f,
    @location(5) effect_tint: vec4f,
    // World-pixel position of this fragment (unpadded quad).
    @location(6) world_pos: vec2f,
    // Blit rect top row and height in world pixels.
    @location(7) @interpolate(flat) rect_top_height: vec2f,
    @location(8) @interpolate(flat) z_adjust: f32,
    @location(9) @interpolate(flat) z_gradient: u32,
    @location(10) @interpolate(flat) zshape_origin: vec2f,
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
    // frag_depth overrides this.
    output.position = vec4f(clip_x, clip_y, 0.5, 1.0);
    output.uv = instance.uv_origin + quad_uv[idx] * instance.uv_size;
    output.tint = instance.tint;
    output.alpha = instance.alpha;
    output.fx_flags = instance.fx_flags;
    output.fx_params = instance.fx_params;
    output.effect_tint = instance.effect_tint;
    output.world_pos = instance.position + local * instance.size;
    output.rect_top_height = vec2f(instance.position.y, instance.size.y);
    output.z_adjust = instance.z_adjust;
    output.z_gradient = instance.z_gradient;
    output.zshape_origin = instance.zshape_origin;
    return output;
}

fn apply_fx(color: vec4f, _flags: u32, params: vec4f, effect_tint: vec4f) -> vec4f {
    return vec4f(color.rgb * effect_tint.rgb, color.a * params.x);
}

// Native Z of the row `row` (0 = top) of a blit whose top is at screen row
// `screen_top` (Z-buffer coordinates, YOrigin = 0), walked with gradient
// `entry` (0, 1, 2) and Z term `z_adjust`. Mirrors `native_z::sprite_row_z`.
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
fn fs_main(input: VertexOutput) -> FragOutput {
    let color: vec4f = textureSample(t_sprite, s_sprite, input.uv);
    if (color.a < 0.01) {
        discard;
    }

    let camera_row: i32 = i32(round(camera.camera_pos.y));
    let rect_top: f32 = input.rect_top_height.x;
    let height: i32 = max(i32(round(input.rect_top_height.y)), 1);
    let screen_top: i32 = i32(round(rect_top)) - camera_row;
    let row: i32 = clamp(i32(floor(input.world_pos.y - rect_top)), 0, height - 1);
    let entry: u32 = input.z_gradient & 0xFFu;
    var z: i32 = native_row_z(entry, screen_top, height, i32(round(input.z_adjust)), row);

    if ((input.z_gradient & 0x100u) != 0u) {
        let canvas: vec2i = vec2i(floor(input.world_pos)) - vec2i(input.zshape_origin);
        let dims: vec2i = vec2i(textureDimensions(t_zshape));
        if (canvas.x >= 0 && canvas.y >= 0 && canvas.x < dims.x && canvas.y < dims.y) {
            let texel: f32 = textureLoad(t_zshape, canvas, 0).r;
            z = z - (i32(texel * 255.0 + 0.5) - 128);
        }
    }

    // row = DefaultZ - Z + camera_y; depth = 1 - (row - origin) / world_height.
    let ground_row: f32 = f32(32768 - z + camera_row);
    let world_height: f32 = max(camera.world_height, 1.0);
    let frag_depth: f32 = clamp(1.0 - (ground_row - camera.world_origin_y) / world_height, 0.001, 0.999);

    var output: FragOutput;
    output.color = apply_fx(
        vec4f(color.rgb * input.tint, color.a * input.alpha),
        input.fx_flags,
        input.fx_params,
        input.effect_tint,
    );
    output.depth = frag_depth;
    if (camera.pad1 > 0.5) {
        output.color = debug_depth_color(frag_depth);
    }
    return output;
}
