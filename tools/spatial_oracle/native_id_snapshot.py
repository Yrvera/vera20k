"""Original Scenario save/load followed by the native numeric-ID increment.

Only IStream transport, diagnostic logging and clock are supplied callbacks.
The raw save/read and Scenario post-read initialization execute original code.
"""
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_EAX, UC_X86_REG_EIP
from tools.native_oracle import load_image, run_checked, finish_vectors, provenance
from tools.spatial_oracle.map_queries import dwords

SCENARIO, STREAM, VTABLE = 0x20000000, 0x20008000, 0x20009000
RETURN, READ, WRITE = 0x30000000, 0x30000100, 0x30000200


def native_case(value):
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(u)
    u.mem_map(0x10000000, 0x100000)
    u.mem_map(SCENARIO, 0x10000)
    u.mem_map(RETURN, 0x1000)
    u.mem_write(STREAM, dwords(VTABLE))
    u.mem_write(VTABLE + 12, dwords(READ, WRITE))
    u.mem_write(SCENARIO + 0x214, dwords(value))
    u.mem_write(SCENARIO + 0x614, dwords(-1))
    data = bytearray()
    position = 0

    def ret(cleanup=0):
        sp = u.reg_read(UC_X86_REG_ESP)
        address = struct.unpack('<I', u.mem_read(sp, 4))[0]
        u.reg_write(UC_X86_REG_EAX, 0)
        u.reg_write(UC_X86_REG_ESP, sp + 4 + cleanup)
        u.reg_write(UC_X86_REG_EIP, address)

    def hook(_u, address, _size, _data):
        nonlocal position
        if address in (0x4068E0, 0x6C8C40):
            ret()
            return
        if address not in (READ, WRITE):
            return
        sp = u.reg_read(UC_X86_REG_ESP)
        owner, pointer, length, out = struct.unpack('<4I', u.mem_read(sp + 4, 16))
        assert owner == STREAM
        if address == WRITE:
            data.extend(u.mem_read(pointer, length))
        else:
            assert position + length <= len(data)
            u.mem_write(pointer, bytes(data[position:position + length]))
            position += length
        if out:
            u.mem_write(out, dwords(length))
        ret(16)

    u.hook_add(UC_HOOK_CODE, hook)

    def call(address, args=(), required=()):
        sp = 0x100FF000
        u.mem_write(sp, dwords(RETURN, *args))
        u.reg_write(UC_X86_REG_ESP, sp)
        u.reg_write(UC_X86_REG_ECX, SCENARIO)
        run_checked(u, address, RETURN, count=1000000, required_addresses=required)

    call(0x689310, (STREAM,))
    saved = struct.unpack_from('<I', data, 0x214)[0]
    u.mem_write(SCENARIO + 0x214, dwords(0xABADCAFE))
    call(0x689470, (STREAM,), (0x683560,))
    loaded = struct.unpack('<I', u.mem_read(SCENARIO + 0x214, 4))[0]
    call(0x68BCB0)
    return dict(input=value, saved=saved, loaded=loaded, next=u.reg_read(UC_X86_REG_EAX),
                stream_bytes=len(data), read_bytes=position)


def cases():
    return {'cases': [native_case(value) for value in
                     (0, 1, 1010038, 0x7FFFFFFE, 0x7FFFFFFF, 0xFFFFFFFE, 0xFFFFFFFF)]}


if __name__ == '__main__':
    finish_vectors(cases, Path(__file__).with_suffix('.json'), provenance=lambda: provenance(
        scope='Scenario+214 ID cursor through original Scenario save/load and subsequent increment',
        assumptions=['Valid stopped elapsed timer, three empty dynamic arrays, false11F8 string flag',
                     'Supplied in-memory IStream; no native file chooser, disk transport or complete game save claim'],
        substitutions=['IStream Read/Write byte transport', '4068E0 diagnostic logging returns',
                       '6C8C40 clock returns0'],
        entry_points={'save': 0x689310, 'load': 0x689470, 'post_read': 0x683560, 'next_id': 0x68BCB0}))
