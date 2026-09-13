"""Original Foot speed setter and complete Drive/Ship constructor ownership.

These are supplied CPU frames, not command/gameplay or callback admission proofs.
All executed functions use original bytes, including the common base constructor.
"""
from pathlib import Path
import struct

from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_ESP
from tools.native_oracle import finish_vectors, provenance, run_checked, RET_MAGIC, SCRATCH
from tools.spatial_oracle.locomotor_track_cursor import OriginalCursor, LOCO, FOOT, SP
from tools.spatial_oracle.map_queries import dwords


def witness(family, requested):
    n = OriginalCursor(family)
    n.seed(-1, -1, False, 0)
    u = n.uc
    # Existing owner memory is separate from the object being constructed.
    u.mem_write(FOOT, bytes([0xA5]) * 0x800)
    u.reg_write(UC_X86_REG_ECX, FOOT)
    u.mem_write(SP, dwords(RET_MAGIC) + struct.pack('<d', requested))
    run_checked(u, 0x4D3710, RET_MAGIC, count=40)
    assert u.reg_read(UC_X86_REG_ESP) == SP + 12
    applied = struct.unpack('<d', u.mem_read(FOOT + 0x578, 8))[0]
    before = bytes(u.mem_read(FOOT, 0x800))

    ctor = 0x4AF540 if family == 'drive' else 0x69EC50
    u.mem_write(LOCO, bytes([0x5A]) * 0x80)
    u.reg_write(UC_X86_REG_ECX, LOCO)
    u.reg_write(UC_X86_REG_ESP, SP)
    u.mem_write(SP, dwords(RET_MAGIC))
    run_checked(u, ctor, RET_MAGIC, count=100, required_addresses=(0x55A6C0,))
    assert u.reg_read(UC_X86_REG_EAX) == LOCO
    assert u.reg_read(UC_X86_REG_ESP) == SP + 4
    assert bytes(u.mem_read(FOOT, 0x800)) == before
    target = struct.unpack('<d', u.mem_read(LOCO + 0x50, 8))[0]
    result = dict(applied=applied, constructor_target=target,
                  constructor_preserves_owner=True)

    if family == 'drive':
        # Supply linked owner and a populated stash; END transfers it without
        # invoking the caller's release or executing a callback.
        interface, output, stash = LOCO + 0x18, SCRATCH + 0x7000, SCRATCH + 0x7100
        u.mem_write(LOCO + 0xC, dwords(FOOT))
        u.mem_write(LOCO + 0x68, dwords(stash))
        u.reg_write(UC_X86_REG_ESP, SP)
        u.mem_write(SP, dwords(RET_MAGIC, interface, output))
        run_checked(u, 0x4AF930, RET_MAGIC, count=30, required_addresses=(0x4AF94D,))
        assert u.reg_read(UC_X86_REG_ESP) == SP + 12
        assert n.ints(output) == [stash] and n.ints(LOCO + 0x68) == [0]
        assert bytes(u.mem_read(FOOT, 0x800)) == before
        result['end_preserves_owner'] = True
    return dict(input=dict(family=family, requested=requested), output=result)


def generate():
    return [witness(family, requested) for family in ('drive', 'ship')
            for requested in (-0.5, 0.0, 0.25, 0.5, 1.0, 1.25)]


def metadata():
    return provenance(
        scope='Foot applied-fraction finite clamp and preservation across complete Drive/Ship construction and successful Drive END',
        assumptions=[
            'Supplied disjoint owner/class memory, finite exactly representable fractions and startup x87 state',
            'Constructor global frame/null values use mapped image state; their runtime producers are excluded',
            'END uses supplied owner-link/stash; caller release, active-slot replacement and callback admission are excluded',
        ], substitutions=['No code patches, hooks returning call results, or omitted constructor callees'],
        entry_points={'setter': 0x4D3710, 'drive_constructor': 0x4AF540,
                      'ship_constructor': 0x69EC50, 'common_constructor': 0x55A6C0,
                      'drive_end': 0x4AF930})


if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'), provenance=metadata)
