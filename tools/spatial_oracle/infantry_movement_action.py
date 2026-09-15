"""Bounded original Infantry movement/idle action acceptance and timer writes.

Executable caller comparison: supplied valid Infantry/type/sequence state, ordinary
ground gates, no transport/remap-to-water, no full action sequencer delivery.
No original callable is replaced. All 42 requested sequence records have a
supplied count of 6 unless a row explicitly tests an absent requested sequence.
"""
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_FPCW
from tools.native_oracle import load_image, run_checked, STACK_BASE, STACK_SIZE, SCRATCH, RET_MAGIC, finish_vectors, provenance

ACTOR, TYPE, SEQUENCES, LOCO = [SCRATCH + n * 0x4000 for n in range(4)]


def query(row):
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(uc)
    uc.mem_map(STACK_BASE, STACK_SIZE)
    uc.mem_map(SCRATCH, 0x10000)
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

    # Original vtable and function slots; supplied object/type state.
    write32(ACTOR, 0x7EB058)
    write32(ACTOR + 0x6C0, TYPE)
    write32(TYPE + 0xE3C, SEQUENCES)
    for action in range(42):
        write32(SEQUENCES + action * 36 + 4, 6)
    requested = row.get('request', 3)
    if row.get('absent') and requested >= 0:
        write32(SEQUENCES + requested * 36 + 4, 0)
    write32(ACTOR + 0x6C4, row.get('current', -1))
    write32(ACTOR + 0x6C, 100)  # nonzero gate bypasses conditional +500 callback
    uc.mem_write(ACTOR + 0x90, b'\x01')
    uc.mem_write(ACTOR + 0x8D, bytes([int(row.get('object_is_falling_down', False))]))
    uc.mem_write(ACTOR + 0x6DB, bytes([int(row.get('prone', False))]))
    write32(ACTOR + 0x6D4, row.get('fear', 0))
    write32(ACTOR + 0xAC, 5)
    write32(ACTOR + 0xB4, -1)
    write32(ACTOR + 0x100, 17, 0, 91, 92)
    write32(ACTOR + 0xF8, 7)
    write32(0xA8ED84, 100)
    # Type+5B4=0; actor+2DC=0 and +74=0. Original +54 executes and returns false.
    call(0x75AA90, LOCO, [])
    write32(LOCO + 0xC, ACTOR)
    write32(ACTOR + 0x674, LOCO + 4)
    if row.get('motion'):
        write32(LOCO + 0x28, 2880, 2624, 0)
        uc.reg_write(UC_X86_REG_ECX, LOCO)
        run_checked(uc, 0x75AEC0, 0x75BD29, count=100, required_addresses=[0x75AEC0, 0x75BD25])
    events = []
    observed = {0x521161, 0x75CB20, 0x51D6F0, 0x51D91D, 0x51D934, 0x51D9D2, 0x51DA34, 0x51DA8C, 0x4DE620, 0x5F6B90}
    uc.hook_add(UC_HOOK_CODE, lambda _u, address, _size, _data: events.append(hex(address)) if address in observed else None)
    if row.get('consumer'):
        # Whole 520F40 with Guard/NavCom=NULL; supplied gates skip its earlier
        # destination-recovery work. Real Walk +A8 and Infantry +558 execute.
        call(0x520F40, ACTOR, [])
        accepted = None  # caller has no promised return-value contract
    else:
        call(0x51D6F0, ACTOR, [requested, int(row.get('force', False)), 0])
        accepted = uc.reg_read(UC_X86_REG_EAX) & 0xff
    return {'input': row, 'accepted': accepted, 'events': events,
            'doing': read32(ACTOR + 0x6C4), 'frame': read32(ACTOR + 0xF8),
            'timer_start': read32(ACTOR + 0x100), 'timer_duration': read32(ACTOR + 0x108),
            'timer_repeat': read32(ACTOR + 0x10C), 'prone': uc.mem_read(ACTOR + 0x6DB, 1)[0],
            'motion': uc.mem_read(LOCO + 0x36, 1)[0]}


def generate():
    rows = [{'request': request, 'current': current} for request in (0, 2, 3, 6) for current in range(-1, 42)]
    rows += [{'request': 3, 'current': 31, 'force': True},
             {'request': -1}, {'request': 3, 'absent': True},
             {'request': 3, 'current': 33, 'object_is_falling_down': True},
             {'request': 3, 'fear': 199}, {'request': 3, 'fear': 200}]
    rows += [{'consumer': True, 'motion': motion, 'prone': prone, 'current': current}
             for motion in (False, True) for prone in (False, True) for current in (-1, 3, 6, 17, 31)]

    return [query(row) for row in rows]


if __name__ == "__main__":
    finish_vectors(generate, Path(__file__).with_suffix(".json"), provenance=provenance(
        scope="Original Infantry movement/idle DoAction acceptance, timer/frame writes and ordinary Walk movement-flag consumer; no full action sequencer or Rust parity claim.",
        assumptions=[
            "Supplied valid Infantry object, original vtable7EB058, own Type+E3C sequence records; all42 frame counts supplied6 unless an absent requested sequence is explicit. Infantry/type constructors and retail sequence loading are not executed.",
            "Actor+6C is supplied100, alive+90=true, actor+2DC=0, actor+74=false, type MovementZone+5B4=0 and type+D94=false. Transport/carry, water remap, highflight, zero-health+500 and specialized Jumpjet branches are outside these rows.",
            "Frame100; old timer start17,duration91,repeat92 and old image frame7. Timer+104 is the native ignored stack-derived middle slot and is not assigned a logical meaning or compared.",
            "172 direct requests0/2/3/6 cross currentDoing-1..41. Six contrasts cover forced interruption, request-1, absent requested sequence, falling Paradrop refusal, and fear199/200 Panic remap.",
            "20 whole520F40 consumers use Guard mission5 and NavComNULL, skipping earlier destination recovery. Motion=true executes original Walk75AEC0 entry/head gate through75BD25; constructor supplies initial false. Numeric movement and head retirement are not executed.",
            "Force and randomStartFrame are false except the explicit forced row. No sequence advancement, randomized start, normalized-action delay, swimming sound, or complete Doing lifecycle claim.",
            "The observed events identify original call sites. The whole520F40 return value has no claimed contract and is excluded from acceptance output."
        ],
        substitutions=[],
        entry_points={"do_action":0x51D6F0,"movement_action_consumer":0x520F40,"walk_constructor":0x75AA90,"walk_motion_producer":0x75AEC0}
    ))
