"""Execute retail ordinary Bullet AI predicates with controlled world receivers.

The original ordinary AI tail, map-size predicate and math execute. Hooks provide
height, cell identity, first building, nearest selected object and alliance results;
those receivers are not covered by the admission vectors. Reflection vectors additionally
execute the original matrix helpers with supplied matrix bytes. This oracle does
not certify the upstream launch/trajectory or downstream damage mechanisms.
"""

import hashlib
import itertools
import json
import struct
from pathlib import Path

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_EBP, UC_X86_REG_ECX, UC_X86_REG_EDX,
    UC_X86_REG_EIP, UC_X86_REG_ESP, UC_X86_REG_FPCW,
)
from tools.rmg_oracle.harness import _load_image, GAMEMD, NATIVE_FPCW

BASE, SP = 0x20000000, 0x2000E000
BULLET, TYPE, SOURCE, OBJECT = BASE, BASE + 0x1000, BASE + 0x2000, BASE + 0x3000
CELL, TARGET_CELL, BUILDING = BASE + 0x4000, BASE + 0x5000, BASE + 0x6000
VTABLE, RAW_COORD = BASE + 0x7000, BASE + 0x7800


def i32(uc, addr, value):
    uc.mem_write(addr, struct.pack('<I', value & 0xFFFFFFFF))


def read_i32(uc, addr):
    return struct.unpack('<i', uc.mem_read(addr, 4))[0]


def coords(uc, addr):
    return list(struct.unpack('<iii', uc.mem_read(addr, 12)))


def return_value(uc, value, argument_bytes):
    sp = uc.reg_read(UC_X86_REG_ESP)
    uc.reg_write(UC_X86_REG_EAX, value)
    uc.reg_write(UC_X86_REG_EIP, read_i32(uc, sp))
    uc.reg_write(UC_X86_REG_ESP, sp + 4 + argument_bytes)


def setup(case):
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    _load_image(uc)
    uc.mem_map(BASE, 0x10000)
    uc.reg_write(UC_X86_REG_ESP, SP)
    uc.reg_write(UC_X86_REG_EBP, BULLET)
    uc.reg_write(UC_X86_REG_FPCW, NATIVE_FPCW)
    i32(uc, BULLET, 0x007E46E4)
    i32(uc, BULLET + 0xAC, TYPE)
    i32(uc, BULLET + 0xB0, SOURCE if case.get('source', False) else 0)
    uc.mem_write(BULLET + 0x9C, struct.pack('<iii', *case.get('old', (128, 128, case.get('old_height', 500)))))
    uc.mem_write(BULLET + 0x140, struct.pack('<iii', *case.get('target', (640, 128, 0))))
    velocity = case.get('velocity', (20, 0, -6))
    uc.mem_write(BULLET + 0xE8, struct.pack('<ddd', *velocity))
    uc.mem_write(SP + 0x90, struct.pack('<ddd', *velocity))
    uc.mem_write(TYPE + 0x2C0, bytes([case.get('vertical', False)]))
    uc.mem_write(TYPE + 0x2A2, bytes([case.get('inaccurate', False)]))
    uc.mem_write(SP + 0x24, struct.pack('<iii', *case.get('candidate', (384, 128, 0))))
    uc.mem_write(OBJECT + 0x9C, struct.pack('<iii', *case.get('object_coord', (384, 128, 0))))
    i32(uc, 0x0089DE70, 104)
    i32(uc, 0x0089DE64, 416)
    i32(uc, 0x0087F7E8 + 0xF4, case.get('map_width', 2))
    i32(uc, 0x0087F7E8 + 0xF8, case.get('map_height', 3))
    calls = []

    def hook(uc, address, size, user):
        sp = uc.reg_read(UC_X86_REG_ESP)
        if address == 0x005F5F40:
            calls.append('height')
            return_value(uc, case.get('old_height', 500), 0)
        elif address in (0x005657A0, 0x00565730):
            arg = read_i32(uc, sp + 4)
            if address == 0x005657A0:
                cell = list(struct.unpack('<hh', uc.mem_read(arg, 4)))
            else:
                cell = [int(value / 256) for value in coords(uc, arg)[:2]]
            target_cell = [int(value / 256) for value in case.get('target', (640, 128, 0))[:2]]
            calls.append(['cell', *cell])
            return_value(uc, TARGET_CELL if cell == target_cell else CELL, 4)
        elif address == 0x0047C520:
            calls.append('building')
            return_value(uc, BUILDING if case.get('same_building', False) else 0, 0)
        elif address == 0x0047C3D0:
            calls.append('nearest')
            selected = case.get('selected', 'none')
            return_value(uc, SOURCE if selected == 'source' else OBJECT if selected == 'object' else 0, 12)
        elif address == 0x004F9A90:
            calls.append('alliance')
            return_value(uc, int(case.get('allied', False)), 4)

    uc.hook_add(UC_HOOK_CODE, hook)
    return uc, calls


def admission(case):
    uc, calls = setup(case)
    uc.emu_start(0x004677D3, 0x00467B7A, count=100000)
    return dict(**case, impact=bool(uc.mem_read(SP + 0x18, 1)[0]),
        reason=read_i32(uc, SP + 0x20), near_target=bool(uc.mem_read(SP + 0x1F, 1)[0]),
        result=coords(uc, SP + 0x24), calls=calls)


def slope_matrices():
    # Execute the complete original startup initializer chronology with no
    # hooks. In particular 755852..755875 writes identity into rows 17..20.
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    _load_image(uc)
    uc.mem_map(BASE, 0x10000)
    uc.mem_map(0x30000000, 0x1000)
    uc.reg_write(UC_X86_REG_FPCW, 0x037F)

    def run(address, arguments=()):
        uc.mem_write(SP, struct.pack('<' + 'I' * (len(arguments) + 1), 0x30000000, *arguments))
        uc.reg_write(UC_X86_REG_ESP, SP)
        uc.emu_start(address, 0x30000000, count=10000000)

    run(0x007CEAAF)
    run(0x007CBF49, (0x300, 0x300))
    run(0x007C5EE4)
    for address in (0x754910, 0x7549A0, 0x7549C0, 0x7549E0, 0x754A20, 0x754A50, 0x754CB0):
        run(address)
    return [list(struct.unpack('<12I', uc.mem_read(0xB45188 + 48 * slope, 48))) for slope in range(21)]


def reflection(matrix, slope, velocity, elasticity):
    uc, _ = setup(dict(velocity=velocity))
    uc.mem_write(0x00B45188 + slope * 48, struct.pack('<12I', *matrix))
    uc.mem_write(CELL + 0x11C, bytes([slope]))
    uc.mem_write(SP + 0x44, struct.pack('<iii', 384, 128, -1))
    uc.mem_write(SP + 0x50, struct.pack('<d', elasticity))
    uc.emu_start(0x00467666, 0x00467778, count=100000)
    result = list(struct.unpack('<ddd', uc.mem_read(SP + 0x90, 24)))
    return dict(slope=slope, velocity=velocity, elasticity=elasticity,
        result_f32_bits=[struct.unpack('<I', struct.pack('<f', value))[0] for value in result],
        quantized_result=[int(value) for value in result])


def nearest(objects):
    # Original E4 walk, eligibility, virtual +48 (including Building center),
    # low-byte distance and strict tie handling. No hooks.
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    _load_image(uc)
    uc.mem_map(BASE, 0x10000)
    uc.mem_map(0x30000000, 0x1000)
    uc.reg_write(UC_X86_REG_FPCW, NATIVE_FPCW)
    uc.mem_write(0x822D80, struct.pack('<H', NATIVE_FPCW))
    addresses = [BASE + 0x1000 + i * 0x2000 for i in range(len(objects))]
    i32(uc, CELL + 0xE4, addresses[0] if addresses else 0)
    for index, (address, obj) in enumerate(zip(addresses, objects)):
        building = obj.get('building', False)
        i32(uc, address, 0x7E3EBC if building else 0x7F522C if obj.get('terrain', False) else 0x7F5C70)
        # Abstract410170 clears bits0..2; Object5F3900 adds2; only
        # Techno6F2B40 adds1. Terrain71BB90 never adds Techno identity.
        flags = 2 if obj.get('terrain', False) else 3
        if not obj.get('eligible', True):
            flags &= ~1
        uc.mem_write(address + 0x14, bytes([flags]))
        i32(uc, address + 0x30, addresses[index + 1] if index + 1 < len(addresses) else 0)
        uc.mem_write(address + 0x9C, struct.pack('<iii', *obj['coord']))
        if building:
            kind = address + 0x1000
            i32(uc, address + 0x520, kind)
            i32(uc, kind + 0xEF0, obj.get('foundation', 0))
            foundation = obj.get('foundation', 0)
            obj['dimensions'] = [read_i32(uc, 0x8192B8 + foundation * 4), read_i32(uc, 0x819310 + foundation * 4)]
    uc.mem_write(SP, struct.pack('<IIII', 0x30000000, RAW_COORD, 0, 0))
    uc.reg_write(UC_X86_REG_ESP, SP)
    uc.reg_write(UC_X86_REG_ECX, CELL)
    uc.emu_start(0x47C3D0, 0x30000000, count=100000)
    result = uc.reg_read(UC_X86_REG_EAX)
    return dict(objects=objects, selected=addresses.index(result) if result else None)


def final_handoff(case):
    from . import homing_impact as h
    uc, _ = h.setup(1, case['candidate'][2], case['airburst'], case['inaccurate'], case['target_present'])
    uc.mem_write(h.SP + 0x24, struct.pack('<iii', *case['candidate']))
    uc.mem_write(h.BULLET + 0xE8, struct.pack('<ddd', *case['velocity']))
    uc.mem_write(h.SP + 0x1F, bytes([case['near_target']]))
    i32(uc, h.SP + 0x60, case['mode'])
    uc.emu_start(0x467CA9, 0x467E53, count=100000)
    return dict(**case, result=coords(uc, h.BULLET + 0x9C), target_aim=[640,128,624], target_location=[640,128,208])


def geometry(case, matrices):
    from .shared_collision import execute, MEM, TARGET_TYPE, WALL_HOUSE
    uc, queries = execute(dict(case,candidate=[int(v) for v in case['candidate']]), prepare_only=True)
    uc.reg_write(UC_X86_REG_ESP, SP)
    uc.reg_write(UC_X86_REG_EBP, BULLET)
    old = case.get('old', [640,640,500])
    candidate = case['candidate']
    uc.mem_write(BULLET+0x9C, struct.pack('<iii',*old))
    uc.mem_write(SP+0xA8, struct.pack('<iii',*old))
    uc.mem_write(SP+0x44, struct.pack('<iii',*[int(v) for v in candidate]))
    uc.mem_write(SP+0x68, struct.pack('<ddd',*candidate))
    uc.mem_write(SP+0x90, struct.pack('<ddd',*case.get('velocity',[20,3,-6])))
    uc.mem_write(SP+0x50, struct.pack('<d',case.get('elasticity',0.75)))
    source=SOURCE if case.get('source',True) else 0
    building=case.get('building')
    dimensions=None
    if building:
        address=read_i32(uc,MEM+(2*512+2)*4)
        i32(uc,address+0xE4,OBJECT)
        uc.mem_write(0xA8E9A0,b'\x01')
        i32(uc,OBJECT,0x7E3EBC)
        uc.mem_write(OBJECT+0x14,b'\x03')
        i32(uc,OBJECT+0x520,TARGET_TYPE)
        i32(uc,OBJECT+0x21C,WALL_HOUSE)
        index=building.get('foundation',0)
        i32(uc,TARGET_TYPE+0xEF0,index)
        i32(uc,TARGET_TYPE+0x408,TARGET_TYPE+0x2000 if building.get('undeploy',False) else 0)
        dimensions=[read_i32(uc,0x8192B8+index*4),read_i32(uc,0x819310+index*4)]
        if building.get('source_identity',False):
            source=OBJECT
    i32(uc,BULLET+0xB0,source)
    i32(uc,SP+0x64,source)
    for slope,matrix in enumerate(matrices):
        uc.mem_write(0xB45188+slope*48,struct.pack('<12I',*matrix))
    uc.emu_start(0x467494,0x4677D3,count=100000)
    return dict(**case,impact=bool(uc.mem_read(SP+0x18,1)[0]),result=coords(uc,SP+0x24),
        result_candidate_bits=list(struct.unpack('<3Q',uc.mem_read(SP+0x68,24))),
        result_velocity_bits=list(struct.unpack('<3Q',uc.mem_read(SP+0x90,24))),foundation_dimensions=dimensions,queries=queries)


def main():
    rows = []
    for same_cell, same_building, height, vertical in itertools.product(
            (False, True), (False, True), (207, 208, 209), (False, True)):
        rows.append(admission(dict(candidate=(640 if same_cell else 384, 128, 0),
            same_building=same_building, old_height=height, vertical=vertical)))
    for selected, source, allied, inaccurate, distance in itertools.product(
            ('none', 'source', 'object'), (False, True), (False, True), (False, True), (127, 128, 129)):
        rows.append(admission(dict(selected=selected, source=source, allied=allied,
            inaccurate=inaccurate, object_coord=(384 + distance, 128, 0))))
    for velocity, height in itertools.product(((0, 0, 9), (6, 8, 0), (10, 0, 0), (3, 4, 0)), (9, 10)):
        rows.append(admission(dict(velocity=velocity, old_height=height)))
    for x, y in itertools.product((-257, -256, -255, -1, 0, 255, 256, 512, 768, 1024), (128, 512, 1024)):
        rows.append(admission(dict(candidate=(x, y, 500))))
    matrices = slope_matrices()
    reflections = [reflection(matrix, slope, velocity, elasticity)
        for slope, matrix in enumerate(matrices)
        for velocity in [(20, 3, -6), (-100, 71, -41), (0, 0, -6)]
        for elasticity in (0.0, 0.75, 1.0)]
    output = dict(sha256=hashlib.sha256(GAMEMD.read_bytes()).hexdigest(),
        fpcw=NATIVE_FPCW, instruction_range=['004677D3', '00467B7A'], admissions=rows,
        slope_matrices=matrices, reflections=reflections,
        geometry=[geometry(case,matrices) for case in [
            *[dict(candidate=[640,640,z],source=source,cells={'2,2':dict(overlay=overlay)})
                for z,source,overlay in itertools.product((-100.5,-100,-99.5,-0.5,0,149.5,150,150.5),(False,True),(-1,2,26,243,0))],
            *[dict(candidate=[640,640,z],old=[640,640,old_z],source=source,cells={'2,2':dict(flags=256)})
                for z,old_z,source in itertools.product((415,416,417),(415,416,417),(False,True))],
            *[dict(candidate=[640,640,149.5],source=source,source_allied=allied,building=dict(undeploy=undeploy,foundation=foundation))
                for source,allied,undeploy,foundation in itertools.product((False,True),(False,True),(False,True),(0,1))],
            dict(candidate=[640,640,149.5],source=True,building=dict(source_identity=True)),
        ]],
        final_handoffs=[final_handoff(dict(candidate=[640+distance,128,z], velocity=velocity,
            mode=mode, near_target=near, airburst=airburst, inaccurate=inaccurate, target_present=present))
            for distance,z,velocity in itertools.product((383,384,385,1200,1203), (623,624,625), ((4,0,0),(200,0,0)))
            for mode,near,airburst,inaccurate,present in [(0,True,False,False,True), (0,False,False,False,True),
                (1,False,False,False,True),(0,True,True,False,True),(0,True,False,True,True),(0,True,False,False,False)]],
        nearest=[nearest(objects) for objects in [
            [], [dict(coord=[384,128,900]), dict(coord=[640,128,0])],
            [dict(coord=[384,128,0], terrain=True), dict(coord=[640,128,900])],
            [dict(coord=[256,0,0], eligible=False), dict(coord=[511,255,0])],
            [dict(coord=[511,255,0]), dict(coord=[384,128,0], terrain=True)],
            [dict(coord=[384,128,0]), dict(coord=[384,128,0], building=True, foundation=1)],
            [dict(coord=[384,128,0]), dict(coord=[384,128,0], building=True, foundation=4)],
        ]])
    path = Path(__file__).with_name('ordinary_collision_vectors.json')
    path.write_text(json.dumps(output, indent=2) + '\n')
    print(f'{len(rows)} native ordinary admission vectors -> {path}')


if __name__ == '__main__':
    main()
