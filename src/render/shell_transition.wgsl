struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct Params {
    progress: f32,
    width: f32,
    height: f32,
    _pad0: f32,
};

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var destination_tex: texture_2d<f32>;
@group(0) @binding(2) var transition_sampler: sampler;
@group(0) @binding(3) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0),
    );
    var out: VertexOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = vec2<f32>(
        0.5 * (out.position.x + 1.0),
        0.5 * (1.0 - out.position.y),
    );
    return out;
}

fn sample_shifted(tex: texture_2d<f32>, uv: vec2<f32>, offset_px: f32) -> vec4<f32> {
    let shifted = uv - vec2<f32>(offset_px / max(params.width, 1.0), 0.0);
    if (shifted.x < 0.0 || shifted.x > 1.0 || shifted.y < 0.0 || shifted.y > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    return textureSample(tex, transition_sampler, shifted);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let progress = clamp(params.progress, 0.0, 1.0);
    let source_offset_px = -round(progress * 32.0);
    let destination_offset_px = round((1.0 - progress) * 64.0);
    let source_alpha = 1.0 - 0.30 * progress;
    let destination_alpha = progress;

    let source = sample_shifted(source_tex, in.uv, source_offset_px);
    let destination = sample_shifted(destination_tex, in.uv, destination_offset_px);
    let faded_source = source.rgb * source_alpha * source.a;
    let rgb = faded_source * (1.0 - destination_alpha) + destination.rgb * destination_alpha * destination.a;
    return vec4<f32>(rgb, 1.0);
}
