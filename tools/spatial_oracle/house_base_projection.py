"""Original4FD150 center/radius projection, stopping before sector work."""
from pathlib import Path
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_FPCW
from tools.native_oracle import finish_vectors, load_image, run_checked, provenance, STACK_BASE, STACK_SIZE, SCRATCH, RET_MAGIC
from tools.spatial_oracle.map_queries import dwords, packed
HOUSE = SCRATCH
COUNTRY = SCRATCH + 24576
OBJECTS = SCRATCH + 28672
TYPES = SCRATCH + 65536
RULES = SCRATCH + 139264
PAD = SCRATCH + 151552
FREE = SCRATCH + 159744
CELLS = SCRATCH + 196608
VECTOR = SCRATCH + 163840
BASE = SCRATCH + 393216
RAW = SCRATCH + 401408
MAP = 8910824
TABLE = 12582912
DUMMY = 11263056

def query(row):
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(u)
    u.mem_map(STACK_BASE, STACK_SIZE)
    u.mem_map(SCRATCH, 458752)
    u.mem_map(RET_MAGIC, 4096)
    u.reg_write(UC_X86_REG_FPCW, 3711)
    u.mem_write(HOUSE + 52, dwords(COUNTRY))
    u.mem_write(COUNTRY + 276, struct.pack('<5f', 1, 1, 1, 1, 1))
    u.mem_write(HOUSE + 21392, struct.pack('<5f', 1, row.get('unit_bonus', 1), 1, 1, 1))
    u.mem_write(8942048, dwords(RULES))
    u.mem_write(RULES + 2908, dwords(PAD))
    u.mem_write(RULES + 6120, b'\x01')
    u.mem_write(PAD, dwords(PAD + 256, PAD + 256))
    u.mem_write(PAD + 256 + 1004, dwords(PAD + 2048))
    u.mem_write(FREE, dwords(8348184))
    u.mem_write(FREE + 1552, dwords(1400))
    u.mem_write(HOUSE + 752, dwords(row['tracked_count']))
    u.mem_write(HOUSE + 21648, packed(14, 14))
    u.mem_write(HOUSE + 21652, packed(*row.get('alternate', [0, 0])))
    u.mem_write(HOUSE + 21656, dwords(9999))
    u.mem_write(HOUSE + 108, dwords(VECTOR))
    u.mem_write(HOUSE + 120, dwords(len(row['buildings'])))
    for i, b in enumerate(row['buildings']):
        actor = OBJECTS + i * 4096
        typ = TYPES + i * 8192
        u.mem_write(VECTOR + i * 4, dwords(actor))
        u.mem_write(actor, dwords(8273596))
        u.mem_write(actor + 1312, dwords(typ))
        u.mem_write(actor + 108, dwords(b.get('health', 100)))
        u.mem_write(actor + 129, bytes([b.get('limbo', False)]))
        u.mem_write(actor + 156, dwords(*b['xyz']))
        u.mem_write(typ, dwords(8275312))
        u.mem_write(typ + 1552, dwords(b['cost']))
        u.mem_write(typ + 3744, dwords(FREE if b.get('free', False) else 0))
        u.mem_write(typ + 3824, dwords(0))
    u.mem_write(MAP + 244, dwords(8, 8, 0, 0, 8, 8))
    u.mem_write(MAP + 316, dwords(TABLE, 262144))
    u.mem_write(TABLE, bytes(1048576))
    u.mem_write(MAP + 104, dwords(BASE, 289))
    u.mem_write(BASE, bytes(289 * 4))
    u.mem_write(MAP + 24, dwords(RAW))
    u.mem_write(RAW, packed(1, 2))
    u.mem_write(9038400, struct.pack('<90f', *[1.0] * 90))
    u.mem_write(11070852, dwords(100))
    u.mem_write(DUMMY, bytes(512))
    u.mem_write(DUMMY, dwords(8277740))
    u.mem_write(DUMMY + 68, dwords(-1))
    for y in range(17):
        for x in range(17):
            c = CELLS + (y * 17 + x) * 512
            u.mem_write(c, dwords(8277740))
            u.mem_write(c + 36, packed(x, y))
            u.mem_write(c + 68, dwords(-1))
            u.mem_write(TABLE + (y * 512 + x) * 4, dwords(c))
    for p in (9037760, 11277256, 11263624):
        u.mem_write(p, dwords(104))
    events = []

    def observe(_u, a, _s, _d):
        if a in (4582864, 4487872, 5692448, 5230987, 5231313, 5231657, 5231854):
            events.append(hex(a))
    u.hook_add(UC_HOOK_CODE, observe)
    sp = STACK_BASE + STACK_SIZE - 4096
    u.mem_write(sp, dwords(RET_MAGIC))
    u.reg_write(UC_X86_REG_ESP, sp)
    u.reg_write(UC_X86_REG_ECX, HOUSE)
    run_checked(u, 5230928, (5231663, 5231864), count=200000, required_addresses=[5230928, 5230987])
    return dict(input=row, primary=list(struct.unpack('<hh', u.mem_read(HOUSE + 21648, 4))), radius=struct.unpack('<i', u.mem_read(HOUSE + 21656, 4))[0], endpoint=hex(u.reg_read(UC_X86_REG_EIP)), events=events)

def generate():
    first = dict(cost=2000, free=True, xyz=[5 * 256 + 128, 5 * 256 + 128, 0])
    second = dict(cost=1000, xyz=[13 * 256 + 128, 9 * 256 + 128, 0])
    inputs = [dict(tracked_count=0, buildings=[first, second]), dict(tracked_count=1, buildings=[]), dict(tracked_count=2, buildings=[first, second]), dict(tracked_count=2, buildings=[first, second], unit_bonus=0.75), dict(tracked_count=2, buildings=[dict(first, limbo=True), second]), dict(tracked_count=2, buildings=[dict(first, health=0), second]), dict(tracked_count=2, buildings=[first, second], alternate=[1, 1])]
    return [query(r) for r in inputs]
if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'), provenance=lambda: provenance(scope='original House4FD150 entry through center/radius publication, stopping before4FD42F sector aggregation or at4FD4F8 epilogue. Actual BuildingType+84 cost, Building+48 center and FNPC execute. Supplied tracking count, ordered Building records and cost multiplier premise; not count lifecycle or whole House AI/Rust parity.', assumptions=['House primary14,14/radius9999 to expose reset, optional alternate1,1. Declared object health/limbo, original Building/BuildingType/UnitType vtables, foundationindex0, suppliedCost/FreeUnit1400. House and country factors1 except explicitUnit.75. SeparateAircrafttrue; valid nonmatchingpad/dock pointer.', 'Map Size8,8 LocalSize0,0,8,8 with allocated17x17flatcells/nooverlay/rawoccupation0; uniformNormalrawgroup1, all90landspeeds1; frame100/FPCW0E7F and104heightconstants. No map loader, actual Building placement or House field producer claim.'], substitutions=[], entry_points={'projection': 5230928, 'radius_published_stop': 5231663, 'epilogue_stop': 5231864, 'cost': 4582864, 'building_center': 4487872, 'nearby': 5692448}))
