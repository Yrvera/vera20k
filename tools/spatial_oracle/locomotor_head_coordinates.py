"""Original Drive/Ship head-coordinate expressions with native direction startup.

Fresh selection adds a direction to the current Foot XYZ; the second-node and
chain expressions add a direction to their supplied previous coordinate. These
are interior coordinate-producing blocks, not complete movement admission or
final retained-field publication. No expression is reimplemented in Python.
"""
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_ECX, UC_X86_REG_EDX,
    UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP, UC_X86_REG_ESP,
)
from tools.native_oracle import (
    load_image, run_checked, STACK_BASE, STACK_SIZE, SCRATCH, SCRATCH_SIZE,
    RET_MAGIC, finish_vectors, provenance,
)
from tools.spatial_oracle.map_queries import dwords


DIRECTION_INIT, DIRECTION_TABLE = 0x49F3A0, 0x89F6D8
FOOT, LOCO, PRIOR = SCRATCH + 0x1000, SCRATCH + 0x2000, SCRATCH + 0x3000
BLOCKS = {
    ('drive', 'fresh'): (0x4B32AF, 0x4B32F0, 0x38),
    ('ship', 'fresh'): (0x6A28FF, 0x6A293F, 0x38),
    ('drive', 'second_node'): (0x4B40B0, 0x4B40D9, 0x40),
    ('ship', 'second_node'): (0x6A36E0, 0x6A3705, 0x40),
    ('drive', 'chain'): (0x4B1BC4, 0x4B1BF4, 0x20),
    ('ship', 'chain'): (0x6A120A, 0x6A123A, 0x20),
}


class OriginalCoordinates:
    def __init__(self):
        self.uc = u = Uc(UC_ARCH_X86, UC_MODE_32)
        load_image(u)
        u.mem_map(STACK_BASE, STACK_SIZE)
        u.mem_map(SCRATCH, SCRATCH_SIZE)
        u.mem_map(RET_MAGIC, 0x1000)
        self.sp = STACK_BASE + STACK_SIZE - 0x1000
        u.mem_write(self.sp, dwords(RET_MAGIC))
        u.reg_write(UC_X86_REG_ESP, self.sp)
        run_checked(u, DIRECTION_INIT, RET_MAGIC, count=100,
                    required_addresses=[DIRECTION_INIT, 0x49F413])
        assert u.reg_read(UC_X86_REG_ESP) == self.sp + 4

    def coord(self, address):
        return list(struct.unpack('<iii', self.uc.mem_read(address, 12)))

    def directions(self):
        return [list(struct.unpack('<ii', self.uc.mem_read(DIRECTION_TABLE + 8*i, 8)))
                for i in range(8)]

    def evaluate(self, case):
        u, sp = self.uc, self.sp
        family, operation = case['family'], case['operation']
        base, current, direction = case['base'], case['current'], case['direction']
        start, end, output = BLOCKS[family, operation]
        for register in (UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_ECX,
                         UC_X86_REG_EDX, UC_X86_REG_ESI, UC_X86_REG_EDI,
                         UC_X86_REG_EBP):
            u.reg_write(register, 0)
        u.reg_write(UC_X86_REG_ESP, sp)
        u.reg_write(UC_X86_REG_EBP, LOCO)
        u.mem_write(sp, bytes(0x100))
        u.mem_write(LOCO + 0xC, dwords(FOOT))
        u.mem_write(FOOT + 0x9C, dwords(*current))
        u.mem_write(PRIOR, dwords(*base))
        required = [start]
        if operation == 'fresh':
            assert current == base
            u.reg_write(UC_X86_REG_EDX, FOOT + 0x9C)
            u.reg_write(UC_X86_REG_ESI, direction)
            required.append(0x41C230)  # Original XYZ copy constructor executes.
        elif operation == 'second_node':
            u.reg_write(UC_X86_REG_EBX, base[0] & 0xFFFFFFFF)
            u.mem_write(sp + 0x44, dwords(base[1], base[2]))
            if family == 'drive':
                u.reg_write(UC_X86_REG_EDX, direction)
            else:
                u.reg_write(UC_X86_REG_EAX, direction)
                u.reg_write(UC_X86_REG_EDX, base[1] & 0xFFFFFFFF)
        else:
            assert operation == 'chain'
            u.reg_write(UC_X86_REG_ESI, PRIOR)
            u.reg_write(UC_X86_REG_EDX, direction)
        run_checked(u, start, end, count=100, required_addresses=required)
        assert u.reg_read(UC_X86_REG_ESP) == sp
        assert self.coord(PRIOR) == base
        assert self.coord(FOOT + 0x9C) == current
        return self.coord(sp + output)


def inputs():
    poses = {
        'centered': [2176, 2176, 416],
        'noncentered_retained_z': [2133, 2201, 731],
        'cell_edges_negative_z': [2048, 2303, -347],
        'signed_xy': [-1, -257, -104],
        'signed_wrap': [2147483600, -2147483600, -2147483648],
        'null_source': [0, 0, 0],
    }
    for family, operation in BLOCKS:
        for name, base in poses.items():
            for direction in range(8):
                # The chain source is the retained head, even when the current
                # Foot is elsewhere/at another Z. This separate Foot memory is
                # supplied context, not a simulation of intervening movement.
                current = base if operation == 'fresh' else [2209, 2184, 104]
                yield dict(family=family, operation=operation, name=name,
                           direction=direction, base=base, current=current)


def generate():
    directions = OriginalCoordinates().directions()
    rows = []
    for case in inputs():
        original = OriginalCoordinates()
        rows.append(dict(input=case, output=original.evaluate(case)))
        assert original.directions() == directions
    return dict(directions=directions, cases=rows)


if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'), provenance=lambda: provenance(
        scope='Original Drive/Ship fresh, second-node and chain coordinate-producing expressions',
        assumptions=[
            'All six interior blocks use supplied live register/stack frames; movement admission and final head/selector stores are excluded',
            'Original 49F3A0 initializer writes the direction table before each case; no direction deltas are supplied',
            'Original first-node expressions call XYZ copy constructor41C230; no code or calls are substituted',
            'Second-node source is supplied at the post-PUSH1/PUSH0 frame offsets; no intervening first-node admission is executed',
            'Chain source is the prior retained head, with distinct supplied current Foot XYZ; callbacks and complete chaining are excluded',
            'Signed/overflow/NullCoord inputs bound scalar behavior, not active stock-map reachability',
            '288 cases cover two active-retail families, three expressions, six supplied poses and all eight initialized directions',
        ], substitutions=[], entry_points={
            'direction_initializer': DIRECTION_INIT,
            'coordinate_copy_constructor': 0x41C230,
            **{f'{family}_{operation}': row[0] for (family, operation), row in BLOCKS.items()},
        }))
