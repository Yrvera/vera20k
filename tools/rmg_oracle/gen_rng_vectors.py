"""Machine-derived RNG vectors: seeded state + first N draws, for several seeds.

Runs the real gamemd.exe routines under unicorn (see harness.py):
  Random__Seed 0x0065C6D0  __thiscall(this=ECX, seed=stack) -> fills this+0xC..
  Random__Next 0x0065C780  __thiscall(this=ECX) -> EAX, mutates state in place

Every draw is a separate emulated call with the struct carried forward, so the
vectors capture the real state evolution rather than a reimplementation of it.
"""

import struct

from harness import SCRATCH, call, write_vectors

SEED_FN = 0x0065C6D0
NEXT_FN = 0x0065C780
STRUCT = SCRATCH
STRUCT_LEN = 0xC + 250 * 4  # locked/idx_a/idx_b + 250 state dwords = 0x3F4

SEEDS = (0, 1, 1234, 0x7FFF, 0xFFFF)
DRAWS_PER_SEED = 16


def seeded_struct(seed: int) -> bytes:
    """Run Random__Seed and return the full 0x3F4-byte generator struct."""
    result = call(
        SEED_FN,
        ecx=STRUCT,
        stack_args=[seed],
        dumps={"s": (STRUCT, STRUCT_LEN)},
    )
    return bytes.fromhex(result["dumps"]["s"])


def draws(state: bytes, count: int) -> tuple[list[int], bytes]:
    """Chain `count` calls to Random__Next, carrying the struct forward."""
    current, values = state, []
    for _ in range(count):
        result = call(
            NEXT_FN,
            ecx=STRUCT,
            writes={STRUCT: current},
            dumps={"s": (STRUCT, STRUCT_LEN)},
        )
        values.append(result["eax"])
        current = bytes.fromhex(result["dumps"]["s"])
    return values, current


if __name__ == "__main__":
    vectors = {
        "source": "unicorn/gamemd.exe",
        "seed_fn": hex(SEED_FN),
        "next_fn": hex(NEXT_FN),
        "struct_len": STRUCT_LEN,
        "cases": [],
    }
    for seed in SEEDS:
        blob = seeded_struct(seed)
        locked = blob[0]
        idx_a, idx_b = struct.unpack_from("<II", blob, 4)
        values, _ = draws(blob, DRAWS_PER_SEED)
        vectors["cases"].append(
            {
                "seed": seed,
                "locked": locked,
                "idx_a": idx_a,
                "idx_b": idx_b,
                "state_hex": blob[0xC:].hex(),
                "draws": [f"{v:08x}" for v in values],
            }
        )
        print(
            f"seed {seed:>6}: locked={locked} idx={idx_a}/{idx_b} "
            f"state[0]={blob[0xC:0x10][::-1].hex().upper()} "
            f"draw[0]={values[0]:08X}"
        )

    write_vectors("rng.json", vectors)
