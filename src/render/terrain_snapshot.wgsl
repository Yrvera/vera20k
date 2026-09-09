// Read the current attachments while neither is attached for writing.
// The color binding is explicitly non-sRGB: these are encoded display bytes.
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var live_depth: texture_depth_2d;

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32) -> @builtin(position) vec4f {
    let p = array<vec2f, 3>(vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0));
    return vec4f(p[vertex], 0.0, 1.0);
}

struct Snapshot {
    @location(0) word: u32,
    @location(1) depth: f32,
};
@fragment
fn fs_main(@builtin(position) position: vec4f) -> Snapshot {
    let p = vec2i(position.xy);
    let rgb = vec3u(round(textureLoad(scene, p, 0).rgb * 255.0));
    var output: Snapshot;
    output.word = ((rgb.r >> 3u) << 11u) | ((rgb.g >> 2u) << 5u) | (rgb.b >> 3u);
    output.depth = textureLoad(live_depth, p, 0);
    return output;
}
