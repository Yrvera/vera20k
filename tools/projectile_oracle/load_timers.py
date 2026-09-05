"""Execute retail Bullet timer production, save/load, and proximity checks.

Run: python -m tools.projectile_oracle.load_timers (configured RA2_DIR).
Fire's upstream coordinates/type/target are supplied. The actual late Fire
body, concrete WhatAmI receivers, detector, Save/Load bodies, size receiver,
global frame reader, and math execute. Hooks implement only IStream bytes and
pointer registration/fixup; there are no substituted timer/math results.
"""
import hashlib
import json
import struct
from pathlib import Path

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_EBP, UC_X86_REG_ESI, UC_X86_REG_ECX, UC_X86_REG_EIP,
    UC_X86_REG_ESP, UC_X86_REG_FPCW,
)
from tools.rmg_oracle.harness import GAMEMD, _load_image, NATIVE_FPCW

assert hashlib.sha256(GAMEMD.read_bytes()).hexdigest() == (
    '1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c'
)
BASE = 0x20000000
BULLET, PTYPE, TARGET, STREAM, VTABLE, COORD, HOOK = [BASE+i*0x2000 for i in range(7)]
SP, STOP = BASE+0x40000, BASE+0x48000


def w32(u, a, n):
    u.mem_write(a, struct.pack('<I', n & 0xffffffff))


def r32(u, a):
    return struct.unpack('<I', u.mem_read(a, 4))[0]


def signed(n):
    return struct.unpack('<i', struct.pack('<I', n & 0xffffffff))[0]


def run(arm, launch_frame, elapsed, target_kind, distance, failed_load=False,
        origin=(3200,3200,500), reference=(3700,3200,500), candidate=None):
    candidate = candidate or (3700-distance,3200,500)
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    _load_image(u)
    u.mem_map(BASE, 0x50000)
    u.reg_write(UC_X86_REG_FPCW, NATIVE_FPCW)
    output, input_bytes, cursor = bytearray(), b'', 0

    def hook(u, address, size, _):
        nonlocal cursor
        if address in (0x467CA9, 0x467FBA):
            u.emu_stop()
            return
        pop = None
        sp = u.reg_read(UC_X86_REG_ESP)
        if address in (HOOK, HOOK+16):
            dest, length = r32(u, sp+8), r32(u, sp+12)
            if address == HOOK:
                output.extend(u.mem_read(dest, length))
            elif failed_load and input_bytes == bytes(output):
                u.reg_write(UC_X86_REG_EAX, 0x80004005)
                pop = 20
            else:
                assert cursor+length <= len(input_bytes)
                u.mem_write(dest, input_bytes[cursor:cursor+length])
                cursor += length
            if pop is None:
                u.reg_write(UC_X86_REG_EAX, 0)
                pop = 20
        elif address in (0x6CF240, 0x6CF2C0):
            # Supplied pointer registry accepts this isolated object's refs.
            u.reg_write(UC_X86_REG_EAX, 0)
            pop = 12 if address == 0x6CF240 else 16
        if pop is not None:
            u.reg_write(UC_X86_REG_EIP, r32(u, sp))
            u.reg_write(UC_X86_REG_ESP, sp+pop)

    u.hook_add(UC_HOOK_CODE, hook)

    def invoke(address, args=(), this=0, stop=STOP):
        u.reg_write(UC_X86_REG_ESP, SP)
        u.reg_write(UC_X86_REG_ECX, this)
        for i, word in enumerate((STOP, *args)):
            w32(u, SP+4*i, word)
        u.emu_start(address, stop, count=100000)
        assert u.reg_read(UC_X86_REG_EIP) == stop
        return signed(u.reg_read(UC_X86_REG_EAX))

    for address, word in ((BULLET, 0x7E46E4), (BULLET+0xAC, PTYPE),
                          (BULLET+0x10C, TARGET if target_kind != 'none' else 0),
                          (TARGET, 0x7E22A4 if target_kind == 'aircraft' else 0x7F5C70),
                          (PTYPE+0x2F0, arm), (STREAM, VTABLE),
                          (VTABLE+0x10, HOOK), (VTABLE+0xC, HOOK+16),
                          (0xA8ED84, launch_frame)):
        w32(u, address, word)
    u.mem_write(BULLET+0x9C, struct.pack('<iii', *origin))
    u.mem_write(COORD, struct.pack('<iii', *candidate))
    invoke(0x4E1100, this=BULLET+0xB8)
    u.reg_write(UC_X86_REG_EBX, BULLET)
    u.reg_write(UC_X86_REG_ESP, SP)
    u.mem_write(SP+0x44, struct.pack('<iii', *reference))
    u.emu_start(0x468A3F, 0x468A98, count=10000)
    assert u.reg_read(UC_X86_REG_EIP) == 0x468A98
    produced = list(struct.unpack('<10i', u.mem_read(BULLET+0xB8, 40)))
    saved_frame = (launch_frame+elapsed) & 0xffffffff
    w32(u, 0xA8ED84, saved_frame)
    before_mode = invoke(0x4E11F0, (COORD,), BULLET+0xB8)
    before = list(struct.unpack('<10i', u.mem_read(BULLET+0xB8, 40)))
    assert invoke(0x46AFB0, (BULLET, STREAM, 0)) == 0
    assert len(output) == 4+0x160
    # Start in a different live frame. Execute the original global stream
    # reader through its third Read, which restores the saved frame before
    # the separately dispatched Bullet Load (outer ordering: 67E8B5 < 67F138).
    w32(u, 0xA8ED84, saved_frame+12345)
    input_bytes, cursor = struct.pack('<III', 0, 0, saved_frame), 0
    invoke(0x67F9C0, this=STREAM, stop=0x67FA22)
    assert r32(u, 0xA8ED84) == saved_frame
    input_bytes, cursor = bytes(output), 0
    load_result = invoke(0x46AE70, (BULLET, STREAM))
    loaded = list(struct.unpack('<10i', u.mem_read(BULLET+0xB8, 40)))
    after_mode = invoke(0x4E11F0, (COORD,), BULLET+0xB8)
    after = list(struct.unpack('<10i', u.mem_read(BULLET+0xB8, 40)))
    admission = []
    for dropping, impact, rot, ranged in [(False,False,0,True), (True,False,0,True),
                                        (True,True,0,True), (True,False,8,False),
                                        (True,False,0,False)]:
        u.mem_write(BULLET+0xB8, struct.pack('<10i', *loaded))
        u.mem_write(PTYPE+0x29C, bytes([dropping]))
        u.mem_write(PTYPE+0x2A0, bytes([ranged]))
        w32(u, PTYPE+0x2DC, rot)
        u.reg_write(UC_X86_REG_ESP, SP)
        u.reg_write(UC_X86_REG_EBP, BULLET)
        u.mem_write(SP+0x24, bytes(u.mem_read(COORD, 12)))
        u.mem_write(SP+0x18, bytes([impact]))
        u.emu_start(0x467C0C, STOP, count=10000)
        at = u.reg_read(UC_X86_REG_EIP)
        assert at in (0x467CA9, 0x467FBA), hex(at)
        admission.append(dict(dropping=dropping, impact=impact, rot=rot, ranged=ranged,
                              mode=signed(u.reg_read(UC_X86_REG_ESI)),
                              watermark=signed(r32(u, BULLET+0xDC)),
                              detonate=at == 0x467CA9))
    # Timer padding (+4/+10) is stack residue and not semantic input/output.
    def fields(words):
        return dict(first=[words[0], words[2]], arm=[words[3], words[5]],
                    reference=words[6:9], watermark=words[9])
    return dict(arm=arm, launch_frame=launch_frame, elapsed=elapsed,
                origin=list(origin), reference=list(reference), candidate=list(candidate),
                target_kind=target_kind, distance=distance, failed_load=failed_load,
                produced=fields(produced), before=fields(before), before_mode=before_mode,
                saved_frame=saved_frame, load_result=load_result, loaded=fields(loaded),
                after=fields(after), after_mode=after_mode, admission=admission)


rows = [run(arm, frame, elapsed, target, distance)
        for arm in [0, 1, 2, 10, 9999999, -1, 2147483647, -2147483648]
        for frame in [100, 0xfffffffe, 0x7ffffffe]
        for elapsed in [0, 1, 10]
        for target in ['unit', 'aircraft', 'none']
        for distance in [20, 80, 94, 600]]
rows += [run(arm, frame, elapsed, 'unit', 20)
         for arm in [-1, 2]
         for frame, elapsed in [(0xffffffff, 10), (100, 0x80000000)]]
rows += [run(10, 100, 1, 'unit', 0, origin=point, reference=(0,0,0), candidate=point)
         for point in [(63,1,1), (64,-1,2), (120,140,200), (65535,4096,-20000),
                       (2147483647,2147483647,2147483647), (-2147483648,1,-2147483648),
                       (1000000000,32767,1000000001), (123456789,-1987654321,987654321)]]
rows += [run(10, 100, 1, 'unit', 20, failed_load=True)]
Path(__file__).with_suffix('.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
print('original Bullet Fire/Save/global frame read/Load/Check cases:', len(rows))
print('positive Arm gates opened by load:', sum(r['before_mode'] == 0 and r['after_mode'] != 0 for r in rows))
