"""Original House50BF60 ordered f32 cost-factor fold, including underflow."""
from pathlib import Path
import struct
from tools.native_oracle import call, finish_vectors, provenance, SCRATCH
from tools.spatial_oracle.map_queries import dwords

HOUSE, VECTOR, BUILDING, TYPE = [SCRATCH + n for n in (0, 0x6000, 0x8000, 0xA000)]


def query(count):
    # Same immutable type, as for an arbitrary number of stock NAINDP plants.
    writes = {
        HOUSE + 0x144: dwords(VECTOR), HOUSE + 0x150: dwords(count),
        HOUSE + 0x5390: struct.pack('<5f', 7, 8, 9, 10, 11),
        VECTOR: dwords(*([BUILDING] * max(count, 1))),
        BUILDING + 0x520: dwords(TYPE),
        TYPE + 0x16D0: struct.pack('<5f', 1, 0.75, 1, 1, 1),
    }
    result = call(0x50BF60, ecx=HOUSE, writes=writes,
                  dumps={'factors': (HOUSE + 0x5390, 20)},
                  required_addresses=[0x50BF60, 0x50C04A], timeout_instr=100000)
    return dict(count=count, factor_bits=list(struct.unpack(
        '<5I', bytes.fromhex(result['dumps']['factors']))))


def generate():
    return [query(n) for n in (0, 1, 2, 4, 32, 300, 304, 350, 360, 512)]


if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'), provenance=lambda: provenance(
        scope='Original House50BF60 reset and ordered single-precision cost-factor fold. '
              'No FactoryPlant membership producer, cost consumer or full House AI claim.',
        assumptions=['Supplied vector counts0..512 with repeated pointers to one immutable '
                     'Building/Type record. This aliases equivalent stock NAINDP type inputs; '
                     'it does not represent distinct constructed objects. Factors are '
                     'Infantry1, Units0.75, Aircraft1, Buildings1, Defenses1. '
                     'Incoming House factors7..11 expose reset; default native FPCW0E7F.'],
        substitutions=[], entry_points={'fold': 0x50BF60, 'return': 0x50C04A}))
