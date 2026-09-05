from tools.rmg_oracle.harness import GAMEMD
import hashlib
assert hashlib.sha256(GAMEMD.read_bytes()).hexdigest() == '1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c'
"""Original live-gravity and ordinary candidate producer; collision supplied."""
import json, struct
from pathlib import Path
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_FPCW
from tools.rmg_oracle.harness import _load_image, NATIVE_FPCW

BASE = 0x20000000
BULLET, PTYPE, RULES, SP = BASE, BASE+0x2000, BASE+0x4000, BASE+0xA000

def run(velocity, gravity_sequence, floater, origin):
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    _load_image(u)
    u.mem_map(BASE, 0x10000)
    for address, value in ((BULLET+0xAC, PTYPE), (0x8871E0, RULES)):
        u.mem_write(address, struct.pack('<i', value))
    u.mem_write(PTYPE+0x295, bytes([floater]))
    u.mem_write(BULLET+0xE8, struct.pack('<ddd', *velocity))
    u.mem_write(SP+0x24, struct.pack('<iii', *origin))
    frames = []
    for gravity in gravity_sequence:
        u.mem_write(RULES+0x16B8, struct.pack('<i', gravity))
        u.mem_write(SP+0x90, bytes(u.mem_read(BULLET+0xE8, 24)))
        for reg, value in ((UC_X86_REG_ESP, SP), (UC_X86_REG_EBP, BULLET), (UC_X86_REG_FPCW, NATIVE_FPCW)):
            u.reg_write(reg, value)
        u.emu_start(0x46718F, 0x467494, count=30000)
        raw = bytes(u.mem_read(SP+0x90, 24))
        candidate = bytes(u.mem_read(SP+0x44, 12))
        frames.append(dict(bits=[f'{b:016x}' for b in struct.unpack('<QQQ', raw)], candidate=list(struct.unpack('<iii', candidate)), candidate_bits=[f'{b:016x}' for b in struct.unpack('<QQQ', u.mem_read(SP+0x68, 24))]))
        # Supply only the admitted fallthrough commit between visits. Native
        # collision/early-tail/reflection ownership has a separate oracle.
        u.mem_write(BULLET+0xE8, raw)
        u.mem_write(SP+0x24, candidate)
    return dict(velocity=velocity, input_bits=[f'{b:016x}' for b in struct.unpack('<QQQ', struct.pack('<ddd', *velocity))], gravity_sequence=gravity_sequence, floater=floater, origin=origin, frames=frames)

rows = [run(v, g, floater, origin)
        for v in ((0.0, -0.0, 0.0), (98.82575273513794, -0.0, 15.279717743396759), (0.25, -0.75, -1.5), (3.25, -4.5, 6.75))
        for g in ((6,)*8, (5,)*8, (0,)*8, (-1,)*8, (6, 3, 1, 0, -1, 2, 5, 6))
        for floater in (False, True)
        for origin in ((1280, -1280, 1000), (2147483646, -2147483647, -10), (3200, 3200, 100))]
Path(__file__).with_suffix('.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
print('Original ordinary gravity and candidate:', len(rows), 'cases,', sum(len(r['frames']) for r in rows), 'successive producer results')
