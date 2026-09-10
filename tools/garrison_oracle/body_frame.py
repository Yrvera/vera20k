"""Execute stock YR GetCurrentFrame for completed garrison bodies.

Native virtual table and helper instructions run without replacement. The
fixture supplies an initialized idle building, type, occupant count and rules;
it does not execute map loading, boarding, animation scheduling or GPU drawing.
"""
from pathlib import Path
import struct
from tools.native_oracle import SCRATCH, call, finish_vectors, provenance

ENTRY = 0x0043EF90
BUILDING, TYPE, RULES = SCRATCH, SCRATCH + 0x1000, SCRATCH + 0x3000

def native_frame(health, occupants, tech_level):
    b = bytearray(0x720)
    t = bytearray(0x1800)
    r = bytearray(0x1800)
    for offset, value in [(0, 0x007E3EBC), (0x6c, health), (0x520, TYPE),
                          (0x534, 1), (0x694, occupants)]:
        struct.pack_into('<I', b, offset, value)
    struct.pack_into('<I', t, 0xa0, 2000)
    struct.pack_into('<i', t, 0x634, tech_level)
    t[0x157b] = 1
    struct.pack_into('<d', r, 0x1700, 0.5)
    struct.pack_into('<d', r, 0x1708, 0.25)
    return call(ENTRY, ecx=BUILDING, writes={BUILDING: bytes(b), TYPE: bytes(t),
        RULES: bytes(r), 0x008871E0: struct.pack('<I', RULES)},
        required_addresses=[0x0043F02C, 0x004581F0, 0x005F5C60],
        timeout_instr=1000)['eax']

def generate():
    return {'source': 'unicorn/gamemd.exe', 'function': hex(ENTRY), 'cases': [
        dict(health=h, occupants=o, tech_level=t, frame=native_frame(h,o,t))
        for t in [-1, 0, 5] for o in [0, 8]
        for h in [1, 499, 500, 501, 999, 1000, 1001, 1999, 2000]]}

if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'),
        provenance=lambda: provenance(
            scope='54 completed CanBeOccupied body-frame cases, strength 2000; not all inputs or GPU pixels',
            assumptions=['Building state 1 from native Guard/GrandOpening 0x44995D/0x447780',
                         'Original BuildingClass vtable and type/health/occupant accessors',
                         'Supplied object/type/rules data; stock ConditionYellow=.5 and ConditionRed=.25',
                         'No laser fence, firestorm or gate flags; fresh emulator per case'],
            substitutions=[], entry_points={'GetCurrentFrame': ENTRY}))
