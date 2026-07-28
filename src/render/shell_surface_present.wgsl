// Encoded-byte RGB565 presentation for the stock main-menu shell.

@group(0) @binding(0) var source: texture_2d<f32>;

@group(0) @binding(1)
var<storage, read> codebook: array<u32>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> @builtin(position) vec4f {
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    return vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

@fragment
fn fs_main(
    @builtin(position) position: vec4f,
) -> @location(0) vec4f {
    let encoded = textureLoad(source, vec2i(position.xy), 0);
    let encoded_bytes = vec4u(round(encoded * 255.0));
    let red = codebook[encoded_bytes.r >> 3u];
    let green = codebook[32u + (encoded_bytes.g >> 2u)];
    let blue = codebook[encoded_bytes.b >> 3u];
    let presented = vec4u(red, green, blue, encoded_bytes.a);
    return vec4f(presented) / 255.0;
}
