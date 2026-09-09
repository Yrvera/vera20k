// One VXL's ordered native writes and independent 256x256 center/crop.
struct Params {
    count: u32, crop_x: u32, crop_y: u32, width: u32,
    height: u32, pad0: u32, pad1: u32, pad2: u32,
}
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> commands: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> winners: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> output_indices: array<u32>;

@compute @workgroup_size(64)
fn paint_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.count { return; }
    let command = commands[id.x];
    // Host checks 24-bit ordinal capacity. Palette never breaks a tie;
    // even zero-color stores have a nonzero winning ordinal.
    atomicMax(&winners[command.x], ((id.x + 1u) << 8u) | command.y);
}

@compute @workgroup_size(64)
fn resolve_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.width * params.height { return; }
    let x = params.crop_x + id.x % params.width;
    let y = params.crop_y + id.x / params.width;
    output_indices[id.x] = atomicLoad(&winners[y * 256u + x]) & 255u;
}
