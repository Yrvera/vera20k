# Native executable oracle. Run with python -m tools.projectile_oracle.vertical_motion
# Controlled structs supply upstream world/target/type state. FireAt probes execute
# original scalar bodies and BulletFire; their hooks supply listed virtual/world
# leaves, not native numeric results. Vertical and arc-domain math have no hooks.
from tools.rmg_oracle.harness import GAMEMD
import hashlib
assert hashlib.sha256(GAMEMD.read_bytes()).hexdigest() == '1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c'
"""Original Vertical ramp and integer candidate stores, no emulated leaves."""
import json, struct
from pathlib import Path
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_EBX, UC_X86_REG_FPCW
from tools.rmg_oracle.harness import _load_image, NATIVE_FPCW

BASE = 0x20000000
BULLET, PTYPE, SP = BASE, BASE + 0x2000, BASE + 0x8000

def run(velocity, acceleration, maximum, origin):
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    _load_image(u)
    u.mem_map(BASE, 0x10000)
    for address, value in ((BULLET+0xAC, PTYPE), (BULLET+0x110, maximum), (PTYPE+0x2D0, acceleration)):
        u.mem_write(address, struct.pack('<i', value))
    u.mem_write(BULLET+0xE8, struct.pack('<ddd', *velocity))
    u.mem_write(SP+0x24, struct.pack('<iii', *origin))
    frames = []
    for frame in range(8):
        for reg, value in ((UC_X86_REG_ESP, SP), (UC_X86_REG_EBP, BULLET), (UC_X86_REG_EBX, BULLET+0xE8), (UC_X86_REG_FPCW, NATIVE_FPCW)):
            u.reg_write(reg, value)
        u.emu_start(0x4671E0, 0x467334, count=30000)
        frames.append(dict(bits=[f'{b:016x}' for b in struct.unpack('<QQQ', u.mem_read(BULLET+0xE8, 24))], candidate=list(struct.unpack('<iii', u.mem_read(SP+0x24, 12)))))
    return dict(velocity=velocity, input_bits=[f'{b:016x}' for b in struct.unpack('<QQQ', struct.pack('<ddd', *velocity))], acceleration=acceleration, maximum=maximum, origin=origin, frames=frames)

rows = [run(v, a, m, origin)
        for v in ((0.0, -0.0, 0.0), (1.0, 0.0, 0.0), (-2.449137817620151e-16, -0.0, -0.9999389052391052), (0.25, -0.75, -1.5), (3.25, -4.5, 6.75), (99.75, 0.125, -0.25))
        for a in (0, 1, 3)
        for m in (1, 10, 100)
        for origin in ((1280, -1280, 1000), (2147483646, -2147483647, -10))]
rows.append(run((1.0, 0.0, 0.0), 10, 100, (640, 640, 5)))
rows.append(run((0.0, 0.0, 1.0), 1, 50, (0, 0, 0)))
Path(__file__).with_suffix('.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
print('Original Vertical ramp and candidate:', len(rows), 'cases,', sum(len(r['frames']) for r in rows), 'successive frame results')
