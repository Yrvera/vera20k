"""Golden-vector oracle for RMG parity: runs real gamemd.exe code under unicorn.

Maps the PE's sections, gives the emulated CPU a real stack, calls a target
function with a chosen calling convention, and dumps chosen memory ranges.
Produces machine-derived goldens (never hand-computed) per CLAUDE.md.

Requires unicorn >= 2.1.1 (2.0.x imports `distutils`, gone in Python 3.12+).
"""

import json
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_MODE_32, Uc
from unicorn.x86_const import (
    UC_X86_REG_EAX,
    UC_X86_REG_ECX,
    UC_X86_REG_EDX,
    UC_X86_REG_ESP,
)

GAMEMD = Path(
    r"C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe"
)
IMAGE_BASE = 0x00400000
IMAGE_SIZE = 0x00A00000  # covers .text/.rdata/.data of gamemd
STACK_BASE = 0x10000000
STACK_SIZE = 0x00100000
SCRATCH = 0x20000000  # writable scratch for struct inputs/outputs
SCRATCH_SIZE = 0x00010000
RET_MAGIC = 0x30000000  # sentinel return address; emulation stops here

_IMAGE_CACHE: bytes | None = None


def _image_bytes() -> bytes:
    global _IMAGE_CACHE
    if _IMAGE_CACHE is None:
        _IMAGE_CACHE = GAMEMD.read_bytes()
    return _IMAGE_CACHE


def _load_image(uc: Uc) -> None:
    """Map the PE by section headers so RVA-addressed globals resolve."""
    data = _image_bytes()
    pe_off = struct.unpack_from("<I", data, 0x3C)[0]
    n_sections = struct.unpack_from("<H", data, pe_off + 6)[0]
    opt_size = struct.unpack_from("<H", data, pe_off + 20)[0]
    sec_off = pe_off + 24 + opt_size
    uc.mem_map(IMAGE_BASE, IMAGE_SIZE)
    uc.mem_write(IMAGE_BASE, data[:0x1000])  # headers
    for i in range(n_sections):
        off = sec_off + i * 40
        vaddr = struct.unpack_from("<I", data, off + 12)[0]
        rawsz = struct.unpack_from("<I", data, off + 16)[0]
        rawptr = struct.unpack_from("<I", data, off + 20)[0]
        if rawsz:
            uc.mem_write(IMAGE_BASE + vaddr, data[rawptr : rawptr + rawsz])


def call(
    func: int,
    *,
    ecx: int | None = None,
    edx: int | None = None,
    stack_args: list[int] | None = None,
    writes: dict[int, bytes] | None = None,
    dumps: dict[str, tuple[int, int]] | None = None,
    timeout_instr: int = 5_000_000,
) -> dict:
    """Call `func`; return {'eax': int, 'dumps': {name: hexstr}}.

    ecx/edx    -> __thiscall / __fastcall registers
    stack_args -> pushed right-to-left above the sentinel return address
    writes     -> {addr: bytes} preloaded into scratch/struct memory
    dumps      -> {name: (addr, length)} read back after the call
    """
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    _load_image(uc)
    uc.mem_map(STACK_BASE, STACK_SIZE)
    uc.mem_map(SCRATCH, SCRATCH_SIZE)
    uc.mem_map(RET_MAGIC & 0xFFFFF000, 0x1000)
    for addr, blob in (writes or {}).items():
        uc.mem_write(addr, blob)

    sp = STACK_BASE + STACK_SIZE - 0x1000
    for value in reversed(stack_args or []):
        sp -= 4
        uc.mem_write(sp, struct.pack("<I", value))
    sp -= 4
    uc.mem_write(sp, struct.pack("<I", RET_MAGIC))
    uc.reg_write(UC_X86_REG_ESP, sp)
    if ecx is not None:
        uc.reg_write(UC_X86_REG_ECX, ecx)
    if edx is not None:
        uc.reg_write(UC_X86_REG_EDX, edx)

    uc.emu_start(func, RET_MAGIC, count=timeout_instr)

    out = {"eax": uc.reg_read(UC_X86_REG_EAX) & 0xFFFFFFFF, "dumps": {}}
    for name, (addr, length) in (dumps or {}).items():
        out["dumps"][name] = bytes(uc.mem_read(addr, length)).hex()
    return out


def write_vectors(path: str, obj: dict) -> None:
    p = Path(__file__).parent / "vectors" / path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(obj, indent=2))
    print(f"wrote {p}")


if __name__ == "__main__":
    # Smoke test: Random__Seed 0x0065C6D0 is __thiscall(this=ECX, seed=stack)
    # and fills 250 state dwords starting at this+0xC.
    result = call(
        0x0065C6D0,
        ecx=SCRATCH,
        stack_args=[1234],
        dumps={"struct": (SCRATCH, 0x3F4)},
    )
    blob = bytes.fromhex(result["dumps"]["struct"])
    locked = blob[0]
    idx_a, idx_b = struct.unpack_from("<II", blob, 4)
    state = struct.unpack_from("<4I", blob, 0xC)
    print(f"eax        = 0x{result['eax']:08x} (expect 0x{SCRATCH:08x})")
    print(f"locked     = {locked} (expect 0)")
    print(f"idx_a/b    = {idx_a}/{idx_b} (expect 0/103)")
    print("state[0..3] = " + " ".join(f"{v:08X}" for v in state))
