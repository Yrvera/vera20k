"""Machine-derived RNG vectors: seeded state + first N draws, for several seeds.

Run with ``python -m tools.rmg_oracle.gen_rng_vectors`` (read-only check by default).
Runs the real gamemd.exe routines under Unicorn (see tools.native_oracle):
  Random__Seed 0x0065C6D0  __thiscall(this=ECX, seed=stack) -> fills this+0xC..
  Random__Next 0x0065C780  __thiscall(this=ECX) -> EAX, mutates state in place

Every draw is a separate emulated call with the struct carried forward, so the
vectors capture the real state evolution rather than a reimplementation of it.
"""

import struct
from pathlib import Path

from tools.native_oracle import SCRATCH, call, finish_vectors, provenance

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


def generate() -> dict:
    """Execute original seed/draw bodies; return the existing vector schema."""
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
    return vectors


def main() -> None:
    finish_vectors(
        generate,
        Path(__file__).parent / "vectors" / "rng.json",
        provenance=lambda: provenance(
            scope="Random seed and first 16 draws for five selected seeds; not full generator parity.",
            assumptions=[
                "A zero-initialized 0x3f4-byte generator is supplied to Random__Seed.",
                "Each call uses a fresh emulator; only generator bytes are carried between draws.",
            ],
            substitutions=[],
            entry_points={"Random__Seed": SEED_FN, "Random__Next": NEXT_FN},
        ),
    )


if __name__ == "__main__":
    main()
