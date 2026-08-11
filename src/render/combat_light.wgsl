// Exact encoded-byte RGB565 edit used by the native persistent combat-light
// vector. The scene snapshot and target are compatible non-sRGB views of the
// same sRGB-format tactical composition resources.

struct Camera {
    screen_size: vec2f,
    camera_pos: vec2f,
    zoom: f32,
    pad: f32,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var scene: texture_2d<f32>;
@group(2) @binding(0) var native_masks: texture_2d_array<u32>;

struct Instance {
    @location(0) center: vec2i,
    @location(1) surface_index: u32,
    @location(2) flags: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) local_pixel: vec2f,
    @location(1) @interpolate(flat) surface_index: u32,
    @location(2) @interpolate(flat) flags: u32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, instance: Instance) -> VertexOutput {
    let quad = array<vec2f, 6>(
        vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0),
        vec2f(0.0, 1.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0),
    );
    let local = quad[vertex];
    // CoordsToClient2 has already committed its signed integer projection;
    // conversion to float happens only for camera/zoom presentation.
    let screen_center = (vec2f(instance.center) - camera.camera_pos) * camera.zoom;
    // The BSurface footprint is screen-fixed. Only its projected centre follows
    // the tactical camera; zoom must not scale the 256x128 byte surface itself.
    let screen = screen_center - vec2f(128.0, 64.0) + local * vec2f(256.0, 128.0);

    var output: VertexOutput;
    output.position = vec4f(
        screen.x / camera.screen_size.x * 2.0 - 1.0,
        1.0 - screen.y / camera.screen_size.y * 2.0,
        0.0,
        1.0,
    );
    output.local_pixel = local * vec2f(256.0, 128.0);
    output.surface_index = instance.surface_index;
    output.flags = instance.flags;
    return output;
}

fn quantize_565(encoded: vec4u) -> vec4u {
    return vec4u(encoded.r & 0xf8u, encoded.g & 0xfcu, encoded.b & 0xf8u, encoded.a);
}

fn transform_channel(value: u32, mask: u32, flags: u32, disable_bit: u32) -> u32 {
    if ((flags & 1u) != 0u) {
        return (value * (256u - mask)) >> 8u;
    }
    if ((flags & disable_bit) != 0u) {
        return value;
    }
    return min((value * (256u + mask)) >> 8u, 255u);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let local = vec2i(floor(input.local_pixel));
    if (local.x < 0 || local.y < 0 || local.x >= 256 || local.y >= 128) {
        discard;
    }
    let mask = textureLoad(native_masks, local, i32(input.surface_index), 0).r;
    if (mask == 0u) {
        discard;
    }

    let destination = quantize_565(vec4u(round(textureLoad(scene, vec2i(input.position.xy), 0) * 255.0)));
    var transformed = vec4u(
        transform_channel(destination.r, mask, input.flags, 2u),
        transform_channel(destination.g, mask, input.flags, 4u),
        transform_channel(destination.b, mask, input.flags, 8u),
        destination.a,
    );
    transformed = quantize_565(transformed);
    return vec4f(transformed) / 255.0;
}
