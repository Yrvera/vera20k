struct Camera {
    screen_size: vec2f,
    camera_pos: vec2f,
    zoom: f32,
    pad0: f32,
};
@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var t_mask: texture_2d<f32>;
@group(1) @binding(1) var s_mask: sampler;

struct Instance {
    @location(0) position: vec2f,
    @location(1) size: vec2f,
    @location(2) uv_origin: vec2f,
    @location(3) uv_size: vec2f,
    @location(4) depth: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32, instance: Instance) -> VertexOutput {
    let positions = array<vec2f, 6>(
        vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0),
        vec2f(0.0, 1.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0),
    );
    let local = positions[index];
    let screen = (instance.position + local * instance.size - camera.camera_pos) * camera.zoom;
    let pixel = floor(screen + vec2f(0.5, 0.5));

    var output: VertexOutput;
    output.position = vec4f(
        pixel.x / camera.screen_size.x * 2.0 - 1.0,
        1.0 - pixel.y / camera.screen_size.y * 2.0,
        instance.depth,
        1.0,
    );
    output.uv = instance.uv_origin + local * instance.uv_size;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let mask_byte = textureSample(t_mask, s_mask, input.uv).r * (255.0 / 256.0);
    // The pipeline's source factor is Dst and destination factor is One:
    // output = Dst * mask_byte + Dst, the native zero-blend equation.
    return vec4f(vec3f(mask_byte), 0.0);
}
