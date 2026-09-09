"""Execute original YR startup/camera/facing and Blinn-Phong instructions.

python -m tools.voxel_oracle.lighting --check (or explicit --write)

Active ordinary unit chain: 73B70E -> 754BE0 (camera), 73B71C -> 5AF980
(camera * locomotor), vtable 7F6180 -> 4DAF10 -> 706640 -> 706ED0 ->
753D00 -> 7586F0. The geometry's HVA matrix is applied only afterward.
All addresses were checked against the original executable on 2026-09-09.
This fixture executes the lighting initializer, including both inverse matrices,
the VXL normals-mode lookup, Sqrt_Approx, and the complete native normal loop.
It does not emulate object dispatch, rasterization, palette conversion or the GPU.
"""

from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_FPCW

from tools.native_oracle import (
    SCRATCH, STACK_BASE, STACK_SIZE, NATIVE_FPCW, call, load_image,
    run_checked, finish_vectors, provenance,
)

IDENTITY = struct.pack("<12f", 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0)


def dump_call(address, destination, size, **kwargs):
    result = call(address, dumps={"result": (destination, size)}, **kwargs)
    return bytes.fromhex(result["dumps"]["result"])


def native_camera():
    # Run the actual initializer setters, preserving their produced globals.
    pitch = dump_call(0x7549A0, 0xB43F00, 8)
    yaw = dump_call(0x754980, 0xB44498, 8)
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(uc)
    uc.mem_map(STACK_BASE, STACK_SIZE)
    uc.reg_write(UC_X86_REG_ESP, STACK_BASE + STACK_SIZE - 0x1000)
    uc.reg_write(UC_X86_REG_FPCW, NATIVE_FPCW)
    uc.mem_write(0xB43F00, pitch)
    uc.mem_write(0xB44498, yaw)
    run_checked(uc, 0x7558CE, 0x755904,
                required_addresses=[0x5AE860, 0x5AEF60, 0x5AF1A0])
    return bytes(uc.mem_read(0xB44318, 48))


def native_facing(step):
    # The preceding 55A755..55A75D reduces Current facing to these 32 values.
    # Execute its following subtraction, double multiplication, float store,
    # and native RotateZ. The enclosing stack matrix starts at identity.
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(uc)
    uc.mem_map(STACK_BASE, STACK_SIZE)
    sp = STACK_BASE + STACK_SIZE - 0x1000
    uc.reg_write(UC_X86_REG_ESP, sp)
    uc.reg_write(UC_X86_REG_FPCW, NATIVE_FPCW)
    uc.reg_write(UC_X86_REG_ECX, step)
    uc.mem_write(sp + 0xC, IDENTITY)
    run_checked(uc, 0x55A760, 0x55A77E, required_addresses=[0x5AF1A0])
    return bytes(uc.mem_read(sp + 0xC, 48))


def native_multiply(left, right):
    return dump_call(0x5AF980, SCRATCH, 48, ecx=SCRATCH, edx=SCRATCH + 0x100,
                     stack_args=[SCRATCH + 0x200],
                     writes={SCRATCH + 0x100: left, SCRATCH + 0x200: right})


def native_pages(draw_matrix, light, mode):
    # VXL_Init_BlinnPhong accesses the selected VXL tailer's normals mode.
    # Minimal non-owning VXL state: one header with index zero, one tailer.
    vxl = bytearray(24)
    struct.pack_into("<II", vxl, 16, SCRATCH + 0x100, SCRATCH + 0x200)
    tailer = bytearray(0xA4)
    tailer[0xA3] = mode
    return dump_call(0x753D00, 0xB45990, 256, ecx=SCRATCH, edx=0,
                     stack_args=[0, SCRATCH + 0x400, 0x887430, 0x887470, 0x40400000],
                     writes={SCRATCH: bytes(vxl), SCRATCH + 0x100: bytes(12),
                             SCRATCH + 0x200: bytes(tailer),
                             SCRATCH + 0x400: draw_matrix,
                             0x887430: IDENTITY, 0x887470: light},
                     required_addresses=[0x5AFC20, 0x5AF4D0, 0x7564B0,
                                         0x7586F0, 0x4CAC40, 0x758850])


def generate():
    # Init_Game 52BDEE reads this exact float from 7E1E68, then calls 754C00.
    light = dump_call(0x754C00, 0x887470, 12, stack_args=[0x3F490E56])
    camera = native_camera()
    cases = []
    for step in range(32):
        matrix = native_multiply(camera, native_facing(step))
        for mode in (2, 4):
            cases.append({"step": step, "mode": mode,
                          "draw_matrix_bits": list(struct.unpack("<12I", matrix)),
                          "pages": native_pages(matrix, light, mode).hex()})
    return {"source": "unicorn/gamemd.exe", "light_bits": list(struct.unpack("<3I", light)),
            "camera_bits": list(struct.unpack("<12I", camera)), "cases": cases}


if __name__ == "__main__":
    finish_vectors(generate, Path(__file__).with_suffix(".json"),
                   provenance=lambda: provenance(
                       scope="All 32 flat ordinary-unit facing steps, normals modes 2 and 4; entire 256-byte LUT",
                       assumptions=[
                           "x87 0x0E7F: WinMain 6BBFB7..6BBFC9 selects chop and startup precision; original instructions and tables",
                           "startup setters and camera/facing native blocks execute without substitutions",
                           "minimal VXL header/tailer selects one valid normals mode; zero initial stale LUT slots",
                           "Init_Game 52BDDD..52BDE9 initializes viewer matrix 887430 to identity; 52BDF5 initializes light with retail angle",
                           "ordinary stationary facing input; slope, rocking, dispatch, rasterization, VPL and GPU excluded",
                       ], substitutions=[], entry_points={
                           "light_setup": 0x754C00, "camera_block": 0x7558CE,
                           "facing_block": 0x55A760, "matrix_product": 0x5AF980,
                           "lighting_initializer": 0x753D00, "lighting_loop": 0x7586F0}))
