"""Execute 468BB0 and all reached receivers without behavioral hooks.

Synthetic object/cell/house state is the input. Original 565730/5657A0/578080,
47B3A0, 4CC360, 486840/4867E0, 5F6B90/41B920/661F90, 410540/447AC0,
5F6360 and 4F9A50 execute against the retail image. The only code hook records
addresses and cell-query inputs; it never changes registers, memory or control.
This is a post-commit probe oracle, not an upstream trajectory oracle.
"""
import hashlib
import itertools
import json
import struct
from pathlib import Path
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_FPCW
from tools.rmg_oracle.harness import _load_image, GAMEMD, NATIVE_FPCW
from .ordinary_collision import BASE, SP, BULLET, TYPE, SOURCE, OBJECT, RAW_COORD, i32, read_i32, coords

MEM = 0x21000000
RULES, HOUSE, WALL_HOUSE, TARGET_TYPE, LOCO = MEM + 0x20000, MEM + 0x30000, MEM + 0x38000, MEM + 0x50000, MEM + 0x52000
DUMMY, STOP = 0xABDC50, 0x30000000


def execute(case, prepare_only=False):
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    _load_image(uc)
    uc.mem_map(BASE, 0x10000)
    uc.mem_map(MEM, 0x100000)
    uc.mem_map(STOP, 0x1000)
    uc.reg_write(UC_X86_REG_FPCW, NATIVE_FPCW)
    uc.mem_write(0x822D80, struct.pack('<H', NATIVE_FPCW))
    for address in (0x89DE70, 0x89E7C0, 0xAC13C8):
        i32(uc, address, 104)
    i32(uc, 0xAC13BC, 416)
    i32(uc, 0x89DE64, 416)
    i32(uc, 0x87F924, MEM)
    i32(uc, 0x87F928, 512 * 8)

    def cell(address, x, y, row):
        i32(uc, address, 0x7E4EEC)
        uc.mem_write(address + 0x24, struct.pack('<hh', x, y))
        i32(uc, address + 0x38, row.get('tile', 0xFFFF))
        i32(uc, address + 0x44, row.get('overlay', 0 if row.get('wall', False) else -1))
        i32(uc, address + 0x50, 0)
        uc.mem_write(address + 0x11B, bytes([row.get('level', 0) & 255, row.get('slope', 0)]))
        i32(uc, address + 0x140, row.get('flags', 0))

    cell(DUMMY, 0, 0, case.get('dummy', {}))
    for y in range(8):
        for x in range(8):
            if [x, y] in case.get('missing', []):
                continue
            address = MEM + 0x10000 + (y * 8 + x) * 0x148
            i32(uc, MEM + (y * 512 + x) * 4, address)
            cell(address, x, y, case.get('cells', {}).get(f'{x},{y}', {}))
    i32(uc, 0x8871E0, RULES)
    uc.mem_write(RULES + 0x1850, bytes([case.get('transparency', False)]))
    i32(uc, 0xA83D84, MEM + 0x58000)
    i32(uc, MEM + 0x58000, MEM + 0x59000)
    uc.mem_write(MEM + 0x592A8, b'\x01')
    i32(uc, 0xA8022C, MEM + 0x5A000)
    i32(uc, MEM + 0x5A000, WALL_HOUSE)
    i32(uc, HOUSE + 0x30, 1)
    i32(uc, WALL_HOUSE + 0x30, 2)
    i32(uc, HOUSE + 0x5788, 4 if case.get('source_allied', False) else 0)
    i32(uc, WALL_HOUSE + 0x5788, 2 if case.get('wall_allied', False) else 0)
    i32(uc, SOURCE + 0x21C, HOUSE)
    i32(uc, BULLET, 0x7E46E4)
    i32(uc, BULLET + 0xAC, TYPE)
    i32(uc, BULLET + 0xB0, SOURCE if case.get('source', True) else 0)
    candidate = case.get('candidate', [640, 640, 500])
    uc.mem_write(BULLET + 0x9C, struct.pack('<iii', *candidate))
    uc.mem_write(BULLET + 0x134, struct.pack('<iii', *case.get('origin', [128, 128, 0])))
    uc.mem_write(BULLET + 0x140, struct.pack('<iii', *case.get('launch_target', [1408, 640, 0])))
    uc.mem_write(BULLET + 0x14C, struct.pack('<hh', *case.get('previous', [1, 2])))
    for key, offset in [('cliffs', 0x296), ('walls', 0x298), ('level', 0x29D), ('flak', 0x2A3), ('aa', 0x2A4)]:
        uc.mem_write(TYPE + offset, bytes([case.get(key, False)]))
    i32(uc, 0xAA0738, case.get('water_base', 100))
    target = case.get('target')
    foundation = None
    if target:
        category = target.get('category', 'unit')
        i32(uc, BULLET + 0x10C, OBJECT)
        i32(uc, OBJECT, {'unit': 0x7F5C70, 'building': 0x7E3EBC, 'aircraft': 0x7E22A4}[category])
        uc.mem_write(OBJECT + 0x9C, struct.pack('<iii', *target.get('coord', [640, 640, 500])))
        uc.mem_write(OBJECT + 0x74, bytes([target.get('marked', True)]))
        uc.mem_write(OBJECT + 0x8C, bytes([target.get('on_bridge', False)]))
        i32(uc, OBJECT + 0x520, TARGET_TYPE)
        i32(uc, OBJECT + 0x6C4, TARGET_TYPE)
        i32(uc, TARGET_TYPE + 0xEF0, target.get('foundation', 0))
        if category == 'building':
            index = target.get('foundation', 0)
            foundation = [read_i32(uc, 0x8192B8 + index * 4), read_i32(uc, 0x819310 + index * 4)]
        if target.get('rocket', False):
            i32(uc, RULES + (0x514 if target.get('dmisl', False) else 0x4E0), TARGET_TYPE)
            i32(uc, OBJECT + 0x674, LOCO)
            i32(uc, LOCO, 0x7F0B1C)
            i32(uc, LOCO + 0x3C, target.get('phase', 0))
    trace = []
    queries = []
    receivers = {0x4CC360, 0x486840, 0x4867E0, 0x5F6B90, 0x41B920, 0x661F90, 0x410540, 0x447AC0, 0x5F6360, 0x4F9A50}

    def observe(uc, address, size, user):
        if address in receivers:
            trace.append(f'{address:08X}')
        if address in (0x565730, 0x5657A0, 0x578080):
            arg = read_i32(uc, uc.reg_read(UC_X86_REG_ESP) + 4)
            xy = list(struct.unpack('<hh' if address == 0x5657A0 else '<ii', uc.mem_read(arg, 4 if address == 0x5657A0 else 8)))
            queries.append([f'{address:08X}', *xy])

    uc.hook_add(UC_HOOK_CODE, observe)
    if prepare_only:
        return uc, queries
    uc.mem_write(SP, struct.pack('<II', STOP, RAW_COORD))
    uc.reg_write(UC_X86_REG_ESP, SP)
    uc.reg_write(UC_X86_REG_ECX, BULLET)
    uc.emu_start(0x468BB0, STOP, count=100000)
    return dict(**case, admitted=bool(uc.reg_read(UC_X86_REG_EAX) & 255), result=coords(uc, RAW_COORD),
        dummy_coord=list(struct.unpack('<hh', uc.mem_read(DUMMY + 0x24, 4))),
        foundation_dimensions=foundation, queries=queries, receivers=trace)


def main():
    cases = []
    for z in (-417, -416, -415, -1, 0, 1):
        cases.append(dict(candidate=[640, 640, z]))
        for target_z in (-1, 0, 1):
            cases.append(dict(candidate=[640, 640, z], flak=True, target=dict(coord=[640, 640, target_z])))
    for tile in (99, 100, 113, 114, 65535):
        cases.append(dict(level=True, cells={'2,2': dict(tile=tile)}))
    for marked, height, distance in itertools.product((False, True), (207, 208, 209), (127, 128)):
        cases.append(dict(aa=True, candidate=[640 + distance, 640, height], target=dict(coord=[640, 640, height], marked=marked)))
    for phase in range(7):
        for dmisl in (False, True):
            cases.append(dict(aa=True, candidate=[640, 640, 0], target=dict(category='aircraft', rocket=True, dmisl=dmisl, phase=phase, marked=False, coord=[640, 640, 0])))
    for foundation, distance in itertools.product((0, 1, 4), (127, 255, 383)):
        cases.append(dict(aa=True, candidate=[640 + distance, 640, 300], target=dict(category='building', foundation=foundation, coord=[640, 640, 300])))
    for source_level, previous_level, candidate_level, flags in itertools.product((0, 4), (0, 1, 4), (3, 4, 5), (0, 128)):
        cases.append(dict(cliffs=True, cells={'0,0':dict(level=source_level), '1,2':dict(level=previous_level), '2,2':dict(level=candidate_level, flags=flags)}))
    for offset, source_allied, wall_allied, target_same in itertools.product(([0,0], [100,0], [0,100]), (False, True), (False, True), (False, True)):
        cases.append(dict(walls=True, transparency=True, source_allied=source_allied, wall_allied=wall_allied,
            candidate=[640+offset[0],640+offset[1],0], launch_target=[640 if target_same else 1408,640,0], cells={'2,2':dict(wall=True)}))
    for missing in ([[0,0]], [[5,2]], [[1,2]], [[2,2]], [[0,0],[5,2]], [[0,0],[5,2],[1,2],[2,2]]):
        cases.append(dict(walls=True, cliffs=True, missing=missing, candidate=[640,740,500], cells={'2,2':dict(wall=True,level=4)}))
    rows = [execute(case) for case in cases]
    output = dict(sha256=hashlib.sha256(GAMEMD.read_bytes()).hexdigest(), fpcw=NATIVE_FPCW, hooks='observation only', cases=rows)
    path = Path(__file__).with_name('shared_collision_vectors.json')
    path.write_text(json.dumps(output, indent=2)+'\n')
    print(f'{len(rows)} original shared-probe vectors -> {path}')


if __name__ == '__main__':
    main()
