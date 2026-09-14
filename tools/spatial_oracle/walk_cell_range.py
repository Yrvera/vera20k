"""Original Coord->Cell range wrapper and Cell range geometry/query order.

The weapon slot and object fields are supplied. No range verdict, Cell getter,
map lookup, distance calculation or line-of-fire callable is substituted.
"""
from pathlib import Path
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_FPCW
from tools.native_oracle import load_image, run_checked, STACK_BASE, STACK_SIZE, SCRATCH, RET_MAGIC, finish_vectors, provenance
from tools.spatial_oracle.map_queries import dwords, packed

ACTOR, TYPE, HOUSE, VT, WEAPON, SLOT, PROJECTILE, CELLS, COORD = [SCRATCH + i * 0x2000 for i in range(9)]
MAP, TABLE, DUMMY = 0x87F7E8, 0xC00000, 0xABDC50


def query(row):
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(u)
    u.mem_map(STACK_BASE, STACK_SIZE)
    u.mem_map(SCRATCH, 0x20000)
    u.mem_map(RET_MAGIC, 0x1000)
    u.reg_write(UC_X86_REG_FPCW, 0x0E7F)
    def read32(p): return struct.unpack('<I', u.mem_read(p, 4))[0]
    def xyz(p): return list(struct.unpack('<iii', u.mem_read(p, 12)))
    def cell_coord(p): return list(struct.unpack('<hh', u.mem_read(p + 0x24, 4)))
    def ret(cleanup, result):
        sp = u.reg_read(UC_X86_REG_ESP)
        u.reg_write(UC_X86_REG_EAX, result & 0xffffffff)
        u.reg_write(UC_X86_REG_EIP, read32(sp))
        u.reg_write(UC_X86_REG_ESP, sp + cleanup + 4)
    u.mem_write(VT, bytes(u.mem_read(0x7EB058, 0x600)))
    u.mem_write(ACTOR, dwords(VT))
    u.mem_write(ACTOR + 0x6C0, dwords(TYPE))
    u.mem_write(ACTOR + 0x21C, dwords(HOUSE))
    u.mem_write(ACTOR + 0x9C, dwords(*row.get('source', [2624, 2624, 0])))
    u.mem_write(ACTOR + 0x74, bytes([int(row.get('marked', False))]))
    u.mem_write(ACTOR + 0x8C, bytes([int(row.get('on_bridge', False))]))
    u.mem_write(WEAPON + 0xA0, dwords(PROJECTILE))
    u.mem_write(WEAPON + 0xB4, dwords(row.get('range', 768)))
    u.mem_write(WEAPON + 0xB8, dwords(row.get('minimum', 0)))
    u.mem_write(WEAPON + 0x134, bytes([int(row.get('cell_rangefinding', False))]))
    u.mem_write(SLOT, dwords(WEAPON))
    u.mem_write(COORD, dwords(*row.get('target', [3036, 2780, 123])))
    u.mem_write(0x89E7C0, dwords(104))
    u.mem_write(0xAC13C8, dwords(104))
    u.mem_write(0xAC13BC, dwords(416))
    u.mem_write(0xB0EB24, dwords(416))
    u.mem_write(0xAA0738, dwords(row.get('water_base', 314)))
    table = bytearray(0x100000)
    def put_cell(p, c):
        u.mem_write(p, dwords(0x7E4EEC))
        u.mem_write(p + 0x24, packed(*c['coord']))
        u.mem_write(p + 0x38, dwords(c.get('tile', 0)))
        u.mem_write(p + 0x11B, bytes([c.get('level', 0) & 255, c.get('slope', 0)]))
        u.mem_write(p + 0x140, dwords(c.get('flags', 0)))
    cells = row.get('cells', [{'coord': [10, 10]}, {'coord': [11, 10]}])
    for i, c in enumerate(cells):
        p = CELLS + i * 0x200
        put_cell(p, c)
        x, y = c.get('slot', c['coord'])
        struct.pack_into('<I', table, (y * 512 + x) * 4, p)
    put_cell(DUMMY, {'coord': [99, 98], 'tile': 65535, **row.get('dummy', {})})
    u.mem_write(TABLE, bytes(table))
    u.mem_write(MAP + 0x13C, dwords(TABLE, 0x40000))
    events = []
    geometry = None
    def observe(_u, address, _size, _data):
        nonlocal geometry
        sp = u.reg_read(UC_X86_REG_ESP)
        if address == read32(VT + 0x3F8):
            events.append(['weapon', read32(sp + 4)])
            ret(4, SLOT)
        elif address in (0x565730, 0x578080):
            events.append([hex(address), xyz(read32(sp + 4))])
        elif address == 0x5657A0:
            events.append([hex(address), list(struct.unpack('<hh', u.mem_read(read32(sp + 4), 4)))])
        elif address in (0x486840, 0x4867E0):
            p = u.reg_read(UC_X86_REG_ECX)
            events.append([hex(address), p == DUMMY, cell_coord(p)])
        elif address == 0x47B3A0:
            p = u.reg_read(UC_X86_REG_ECX)
            events.append([hex(address), p == DUMMY, cell_coord(p)])
        elif address == 0x6F7220:
            p = read32(sp + 8)
            events.append(['range', xyz(read32(sp + 4)), p == DUMMY, cell_coord(p)])
        elif address == 0x6F7379:
            geometry = xyz(sp + 0x20)
        elif address == 0x4CC310:
            events.append(['line', xyz(u.reg_read(UC_X86_REG_ECX)), xyz(u.reg_read(UC_X86_REG_EDX))])
    u.hook_add(UC_HOOK_CODE, observe)
    sp = STACK_BASE + STACK_SIZE - 0x1000
    u.mem_write(sp, dwords(RET_MAGIC, COORD, 0))
    u.reg_write(UC_X86_REG_ESP, sp)
    u.reg_write(UC_X86_REG_ECX, ACTOR)
    run_checked(u, 0x6F7970, RET_MAGIC, count=100000, required_addresses=[0x6F7970, 0x6F77B0, 0x6F7220])
    assert u.reg_read(UC_X86_REG_ESP) == sp + 12
    return {'input': row, 'result': bool(u.reg_read(UC_X86_REG_EAX) & 255), 'target_geometry': geometry, 'events': events, 'dummy_coord': cell_coord(DUMMY)}


def generate():
    rows = [
        {},
        {'range': 326}, {'range': 325}, {'minimum': 326}, {'minimum': 327},
        {'range': -512, 'target': [3550, 2780, 0], 'cells': []},
        {'target': [3550, 2780, 0], 'range': 1024},
        {'target': [3550, 2780, 0], 'range': 1024, 'dummy': {'level': 2, 'slope': 1}},
        {'target': [3550, 2780, 0], 'range': 1024, 'dummy': {'level': -1}},
        # The first missing target and subsequent missing shooter cell share
        # one pointer. Source recentering overwrites the target's coordinates.
        {'target': [3550, 2780, 0], 'source': [5184, 2624, 0], 'cell_rangefinding': True, 'range': 1, 'dummy': {'level': 2}},
        {'target': [-257, 2780, 0], 'source': [128, 2688, 0], 'range': 1024,
         'cells': [{'coord': [0, 10], 'level': 3, 'flags': 256}], 'dummy': {'level': 1}},
        {'target': [-130816, 512, 0], 'source': [384, 384, 0], 'range': 1,
         'cells': [{'coord': [1, 1]}]},
        {'cell_rangefinding': True},
        {'source': [2624, 2624, 800], 'marked': True},
        {'source': [2624, 2624, 800], 'marked': True, 'cell_rangefinding': True},
        {'cells': [{'coord': [10, 10], 'flags': 256}, {'coord': [11, 10], 'flags': 256}]},
        {'cells': [{'coord': [10, 10], 'flags': 256}, {'coord': [11, 10], 'flags': 256}], 'cell_rangefinding': True, 'on_bridge': True},
    ]
    for tile in [313, 314, 327, 328, -1]:
        rows.append({'cells': [{'coord': [10, 10]}, {'coord': [11, 10], 'tile': tile, 'flags': 256, 'level': 1}], 'range': 500})
    for base, tile in [(-1, 0), (-1, 13), (2147483640, 2147483640)]:
        rows.append({'water_base': base, 'cells': [{'coord': [10, 10]}, {'coord': [11, 10], 'tile': tile, 'flags': 256}], 'range': 400})
    return [query(row) for row in rows]


if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'), provenance=lambda: provenance(
        scope='Original6F7970->6F77B0->6F7220 normal non-arcing Infantry Cell-target range and lookup ordering; explicit supplied object/weapon/map fields.',
        entry_points={'coordinate_cell_wrapper': 0x6F7970, 'range_source': 0x6F77B0, 'range': 0x6F7220, 'cell_coords': 0x486840, 'cell_tile_gate': 0x4867E0, 'cell_ground': 0x47B3A0, 'map_ground': 0x578080, 'map_cell': 0x565730, 'line': 0x4CC310},
        assumptions=['Supplied original Infantry table7EB058 and Cell table7E4EEC; actor non-garrison, no bunker/open-top/veteran range bonuses. Projectile has all flags false, including non-arcing and no wall/cliff collision; original line callable executes.', 'Supplied independently established104 level/208 high-flight and416 bridge constants, x87 control0E7F. WaterSet base is a supplied theater input; tile and Dummy level/slope/flags are supplied current state.', 'No constructors or complete PerCell/weapon selection/flight behavior claimed.'],
        substitutions=['GetWeapon+3F8 records requested slot and supplies one original-shaped weapon slot. No other callable substitution.']))
