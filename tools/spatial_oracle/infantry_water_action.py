"""Original zone-3 movement action remaps and pre-admission water-state writes.

Supplied live map/type/sequence state; original callables execute unchanged.
Audio is explicitly disabled at the original global gate; requests are observed.
"""
from pathlib import Path
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_ESP, UC_X86_REG_FPCW
from tools.native_oracle import load_image, run_checked, STACK_BASE, STACK_SIZE, SCRATCH, RET_MAGIC, finish_vectors, provenance

ACTOR, TYPE, SEQUENCES, LOCO, CELL = [SCRATCH + n * 0x4000 for n in range(5)]
TABLE, MAP = 0xC00000, 0x87F7E8


def query(row):
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(uc)
    uc.mem_map(STACK_BASE, STACK_SIZE)
    uc.mem_map(SCRATCH, 0x20000)
    uc.mem_map(RET_MAGIC, 0x1000)
    uc.reg_write(UC_X86_REG_FPCW, 0x0E7F)

    def write32(address, *values):
        uc.mem_write(address, struct.pack('<' + 'I' * len(values), *(v & 0xffffffff for v in values)))

    def read32(address):
        return struct.unpack('<i', uc.mem_read(address, 4))[0]

    def call(address, owner, args):
        sp = STACK_BASE + STACK_SIZE - 0x1000
        write32(sp, RET_MAGIC, *args)
        uc.reg_write(UC_X86_REG_ESP, sp)
        uc.reg_write(UC_X86_REG_ECX, owner)
        run_checked(uc, address, RET_MAGIC, count=10000, required_addresses=[address])
        assert uc.reg_read(UC_X86_REG_ESP) == sp + 4 * (len(args) + 1)

    write32(ACTOR, 0x7EB058)
    write32(ACTOR + 0x6C0, TYPE)
    write32(TYPE + 0xE3C, SEQUENCES)
    for action in range(42):
        write32(SEQUENCES + action * 36 + 4, 6)
    if row.get('absent') is not None:
        write32(SEQUENCES + row['absent'] * 36 + 4, 0)
    write32(TYPE + 0x5B4, row.get('zone', 3))
    write32(TYPE + 0xEA4, 101)
    write32(TYPE + 0xEA8, 102)
    write32(ACTOR + 0x6C4, row.get('current', -1))
    write32(ACTOR + 0x6C, 100)
    write32(ACTOR + 0x9C, 2688, 2688, 0)
    uc.mem_write(ACTOR + 0x90, b'\x01')
    uc.mem_write(ACTOR + 0x8C, bytes([row.get('bridge', False)]))
    write32(ACTOR + 0x6E8, row.get('old_water_state', 1))
    write32(ACTOR + 0x6D4, row.get('fear', 0))
    write32(ACTOR + 0xAC, 5)
    write32(ACTOR + 0xB4, -1)
    write32(ACTOR + 0x100, 17, 0, 91, 92)
    write32(ACTOR + 0xF8, 7)
    write32(0xA8ED84, 100)
    uc.mem_write(0x8464AC, b'\x00')
    table = bytearray(0x100000)
    struct.pack_into('<I', table, (10 * 512 + 10) * 4, CELL)
    uc.mem_write(TABLE, bytes(table))
    write32(MAP + 0x13C, TABLE, 0x40000)
    write32(0x87F924, TABLE)
    write32(CELL, 0x7E4EEC)
    uc.mem_write(CELL + 0x24, struct.pack('<hh', 10, 10))
    write32(CELL + 0xEC, row.get('land', 2))
    call(0x75AA90, LOCO, [])
    write32(LOCO + 0xC, ACTOR)
    write32(ACTOR + 0x674, LOCO + 4)
    events = []
    sounds = []

    def observe(_uc, address, _size, _data):
        if address in (0x51D6F0, 0x41BEA0, 0x5657A0, 0x51D8B8, 0x51D90B, 0x51D925, 0x51D9D2, 0x51DA34, 0x7509E0):
            events.append(hex(address))
        if address == 0x7509E0:
            sounds.append({'index': uc.reg_read(UC_X86_REG_ECX),
                           'xyz': list(struct.unpack('<iii', uc.mem_read(uc.reg_read(UC_X86_REG_EDX), 12))),
                           'context': read32(uc.reg_read(UC_X86_REG_ESP) + 4),
                           'water_state_at_call': read32(ACTOR + 0x6E8)})

    uc.hook_add(UC_HOOK_CODE, observe)
    call(0x51D6F0, ACTOR, [row.get('request', 3), int(row.get('force', False)), 0])
    return {'input': row, 'accepted': uc.reg_read(UC_X86_REG_EAX) & 0xff,
            'doing': read32(ACTOR + 0x6C4), 'water_state': read32(ACTOR + 0x6E8),
            'frame': read32(ACTOR + 0xF8), 'timer_start': read32(ACTOR + 0x100),
            'timer_duration': read32(ACTOR + 0x108), 'timer_repeat': read32(ACTOR + 0x10C),
            'events': events, 'sounds': sounds}


def generate():
    rows = [{'request': request, 'land': land, 'bridge': bridge, 'old_water_state': old, 'current': current}
            for request in (0, 2, 3, 6) for land in (0, 2, 6) for bridge in (False, True)
            for old in (0, 1) for current in (-1, 17, 31)]
    rows += [{'request': 3, 'absent': 3}, {'request': 3, 'absent': 17},
             {'request': 3, 'zone': 0}, {'request': 3, 'fear': 200},
             {'request': 3, 'current': 31, 'force': True},
             {'request': 3, 'old_water_state': -1}, {'request': 3, 'old_water_state': 2}]
    return [query(row) for row in rows]


if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'), provenance=provenance(
        scope='Original zone3 Infantry movement-action remap, pre-admission water-state and sound-request ordering; no sound playback, retail load, full Doing lifecycle or Rust parity.',
        assumptions=[
            'Supplied original Infantry and Cell vtables, currentXYZ2688,2688,0, real Cell10,10, flat level/slope0. Original coordinate and Map5657A0 calls execute; map construction/terrain producers are not executed.',
            'Own type MovementZone+5B4=3 except zone0 contrast; actor+2DC=0,+74=false, alive+90=true,+6C=100; no carried/highflight/falling/zero-health callback branches. All42 sequence counts supplied6 except declared absent slot.',
            'Direct requests0/2/3/6 cross LandType0/2/6, OnBridge false/true, old+6E8=0/1 and currentDoing-1/17/31. Additional contrasts test absent requested/mapped sequence, fear200, force and old+6E8=-1/2.',
            'Frame100, old logical timer17/91/92 and image frame7; ignored middle timer word+104 is excluded. Random-start false. No later timer/sequence advancement.',
            'Type+EA4/+EA8 sound indices supplied101/102. Original global8464AC is explicitly false, so original7509E0 returns at its disable gate. Observed entry arguments prove caller request ordering only, not audible output or stock sound index identity.',
            'Native zone identity: ReadINI71605E..716081 writes type+5B4 from474E40; its names table81BA88 has index3 pointer81BB38=AmphibiousDestroyer. Stock reachability lead: active rulesmd.ini SHA3d341ef8a13a4b5ab24af2eef48ac94931ac2bb87d950fe3330a07e2d25672ef GHOST(SEAL)/TANY use Walk and AmphibiousDestroyer; fixture does not execute INI load or prove every mode/map override.'
        ], substitutions=[], entry_points={'do_action':0x51D6F0,'packed_cell_query':0x5657A0,'sound_request':0x7509E0,'walk_constructor':0x75AA90}))
