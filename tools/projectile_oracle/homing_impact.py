"""Execute retail BulletClass AI admission/clamp/snap instruction ranges.

Only external floor and target-coordinate inputs are supplied by hooks; the
branching, ObjectClass height getter/setter, and Bullet coordinate setter run
the original executable bytes. This does not certify the upstream homing
trajectory, target-coordinate producers, or downstream warhead implementation.
"""

import hashlib
import json
import struct
from pathlib import Path

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_EBP, UC_X86_REG_EBX, UC_X86_REG_ECX,
    UC_X86_REG_EIP, UC_X86_REG_ESI, UC_X86_REG_ESP, UC_X86_REG_FPCW,
)

from tools.rmg_oracle.harness import _load_image, GAMEMD, NATIVE_FPCW

BASE = 0x20000000
SP = BASE + 0xE000
BULLET, TYPE, TARGET, TARGET_VTABLE = BASE, BASE + 0x1000, BASE + 0x2000, BASE + 0x3000
TARGET_AIM, TARGET_LOCATION = BASE + 0x4000, BASE + 0x4010
SOURCE, SOURCE_VTABLE, SOURCE_TYPE, SOURCE_GET_TYPE = BASE + 0x5000, BASE + 0x6000, BASE + 0x7000, BASE + 0x4020


def write_i32(uc, address, value):
    uc.mem_write(address, struct.pack("<I", value & 0xFFFFFFFF))


def read_i32(uc, address):
    return struct.unpack("<i", uc.mem_read(address, 4))[0]


def coord(uc, address):
    return list(struct.unpack("<iii", uc.mem_read(address, 12)))


def setup(old_height, candidate_z, airburst, inaccurate, target_present=True):
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    _load_image(uc)
    uc.mem_map(BASE, 0x10000)
    uc.reg_write(UC_X86_REG_ESP, SP)
    uc.reg_write(UC_X86_REG_EBP, BULLET)
    uc.reg_write(UC_X86_REG_EBX, BULLET + 0xE8)
    uc.reg_write(UC_X86_REG_FPCW, NATIVE_FPCW)
    write_i32(uc, BULLET, 0x007E46E4)
    write_i32(uc, BULLET + 0xAC, TYPE)
    write_i32(uc, BULLET + 0x10C, TARGET if target_present else 0)
    uc.mem_write(BULLET + 0x9C, struct.pack("<iii", 500, 128, 208 + old_height))
    uc.mem_write(BULLET + 0xE8, struct.pack("<ddd", 4.0, 0.0, 0.0))
    uc.mem_write(TYPE + 0x294, bytes([airburst]))
    uc.mem_write(TYPE + 0x2A2, bytes([inaccurate]))
    uc.mem_write(SP + 0x24, struct.pack("<iii", 504, 128, candidate_z))
    uc.mem_write(SP + 0x30, struct.pack("<iii", 640, 128, 624))
    write_i32(uc, TARGET, TARGET_VTABLE)
    write_i32(uc, TARGET_VTABLE + 0x58, TARGET_AIM)
    write_i32(uc, TARGET_VTABLE + 0x48, TARGET_LOCATION)
    floor_queries = []

    def hook(uc, address, size, user):
        sp = uc.reg_read(UC_X86_REG_ESP)
        if address == 0x00578080:
            argument = read_i32(uc, sp + 4)
            floor_queries.append(coord(uc, argument))
            uc.reg_write(UC_X86_REG_EAX, 208)
            uc.reg_write(UC_X86_REG_EIP, read_i32(uc, sp))
            uc.reg_write(UC_X86_REG_ESP, sp + 8)
        elif address in (TARGET_AIM, TARGET_LOCATION):
            argument = read_i32(uc, sp + 4)
            result = (640, 128, 624 if address == TARGET_AIM else 208)
            uc.mem_write(argument, struct.pack("<iii", *result))
            uc.reg_write(UC_X86_REG_EAX, argument)
            uc.reg_write(UC_X86_REG_EIP, read_i32(uc, sp))
            uc.reg_write(UC_X86_REG_ESP, sp + 8)
        elif address == SOURCE_GET_TYPE:
            uc.reg_write(UC_X86_REG_EAX, SOURCE_TYPE)
            uc.reg_write(UC_X86_REG_EIP, read_i32(uc, sp))
            uc.reg_write(UC_X86_REG_ESP, sp + 4)

    uc.hook_add(UC_HOOK_CODE, hook)
    return uc, floor_queries


def main():
    admissions = []
    for velocity in [(4, 0, 0), (3, 4, 0), (0, 0, 4), (3, 4, 12)]:
        for height in (-1, 0, 1):
            for distance in (1, 2, 3, 6, 7, 1000):
                for airburst in (False, True):
                    for empty_target in (False, True):
                        uc, queries = setup(height, 207, airburst, False)
                        uc.mem_write(BULLET + 0xE8, struct.pack("<ddd", *velocity))
                        write_i32(uc, SP + 0x10, distance)
                        if empty_target:
                            uc.mem_write(SP + 0x30, bytes(12))
                        uc.emu_start(0x00466DB1, 0x00466E6B, count=10000)
                        admissions.append(dict(height=height, distance=distance, velocity=velocity,
                            airburst=airburst, empty_target=empty_target,
                            impact=bool(uc.mem_read(SP + 0x18, 1)[0]),
                            candidate=coord(uc, SP + 0x24), floor_queries=queries))
    handoffs = []
    for height in (-1, 0, 1):
        for mode in (0, 1, 2):
            for airburst in (False, True):
                for inaccurate in (False, True):
                    for present in (False, True):
                        uc, queries = setup(height, 208 + height, airburst, inaccurate, present)
                        uc.mem_write(BULLET + 0x9C, struct.pack("<iii", 504, 128, 208 + height))
                        uc.emu_start(0x00467BF0, 0x00467C0C, count=10000)
                        write_i32(uc, SP + 0x60, mode)
                        uc.emu_start(0x00467CA9, 0x00467E53, count=10000)
                        handoffs.append(dict(height=height, fuse_mode=mode, airburst=airburst,
                            inaccurate=inaccurate, target_present=present,
                            impact=coord(uc, BULLET + 0x9C), floor_queries=queries))
    source_modes = []
    for present in (False, True):
        for jumpjet in (False, True):
            for mode in (0, 1, 2):
                uc, _ = setup(1, 209, False, False)
                write_i32(uc, BULLET + 0xB0, SOURCE if present else 0)
                write_i32(uc, SOURCE, SOURCE_VTABLE)
                write_i32(uc, SOURCE_VTABLE + 0x84, SOURCE_GET_TYPE)
                uc.mem_write(SOURCE_TYPE + 0xD94, bytes([jumpjet]))
                uc.reg_write(UC_X86_REG_ESI, mode)
                uc.emu_start(0x00467C3C, 0x00467C6A, count=10000)
                source_modes.append(dict(source_present=present, source_jumpjet=jumpjet,
                    detector_mode=mode, admitted_mode=uc.reg_read(UC_X86_REG_ESI)))
    output = dict(binary_sha256=hashlib.sha256(GAMEMD.read_bytes()).hexdigest(),
        coverage="admission 0x466DB1..0x466E6B; common impact clamp 0x467BF0..0x467C0C; mode1 final snap 0x467CA9..0x467E53 (near-object flag zero)",
        admissions=admissions, handoffs=handoffs, source_modes=source_modes)
    path = Path(__file__).parent / "homing_impact_vectors.json"
    path.write_text(json.dumps(output, indent=2) + "\n")
    print(f"Wrote {len(admissions)} admission and {len(handoffs)} handoff vectors: {path}")


if __name__ == "__main__":
    main()
