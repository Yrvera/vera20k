// Exact AnimClass Shadow=yes destination edit.
//
// The target and destination snapshot are non-sRGB views of the tactical
// composition. This is deliberate: native flags 0x601 halve the stored,
// gamma-encoded destination word wherever the SHP shadow stencil is nonzero.

struct Camera {
    screen_size: vec2f,
    camera_pos: vec2f,
    zoom: f32,
    pad0: f32,
};
@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var t_stencil: texture_2d<f32>;
@group(1) @binding(1) var s_stencil: sampler;
@group(2) @binding(0) var destination_snapshot: texture_2d<f32>;

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
    let quad = array<vec2f, 6>(
        vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0),
        vec2f(0.0, 1.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0),
    );
    let local = quad[index];
    // Keep the ordinary batch shader's pixel placement exactly. The encoded
    // pass changes only the material/compositor, never AnimClass coordinates.
    let is_zoomed = abs(camera.zoom - 1.0) >= 0.001;
    let pad = select(0.0, 0.5 / camera.zoom, is_zoomed);
    let raw = (
        instance.position - vec2f(pad, pad)
        + local * (instance.size + vec2f(pad * 2.0, pad * 2.0))
        - camera.camera_pos
    ) * camera.zoom;
    let pixel = select(raw, floor(raw + vec2f(0.5, 0.5)), !is_zoomed);

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
    if (textureSample(t_stencil, s_stencil, input.uv).a < 0.5) {
        discard;
    }
    let destination = vec4u(round(
        textureLoad(destination_snapshot, vec2i(input.position.xy), 0) * 255.0
    ));

    // The native 0x601 blitter edits the packed RGB565 destination word. Its
    // `(word >> 1) & 0x7bef` operation halves each R5/G6/B5 integer with
    // truncation while preventing component bits from crossing boundaries.
    // Unpacking zero-fills the discarded low bits, matching the active retail
    // DirectDraw storage round trip rather than a floating RGBA8 multiply.
    let packed = (
        ((destination.r >> 3u) << 11u)
        | ((destination.g >> 2u) << 5u)
        | (destination.b >> 3u)
    );
    let halved = (packed >> 1u) & 0x7befu;
    let stored = vec4u(
        ((halved >> 11u) & 0x1fu) << 3u,
        ((halved >> 5u) & 0x3fu) << 2u,
        (halved & 0x1fu) << 3u,
        destination.a,
    );
    return vec4f(stored) / 255.0;
}
