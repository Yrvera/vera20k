"""Vectors for the Gaussian helper 0x005980C0 (Box-Muller) and its RNG callback.

Layout of the helper's control block (the global at 0x00ABDFB8):
    +0x00  u8      cached-value flag
    +0x08  f64     cached second variate
    +0x10  ptr     callback returning a uniform [0,1) double on ST0

The callback the generator installs is 0x00598000:
    Random__Next(g_MapGenRng) -> FILD -> FMUL [0x007ED898]
i.e. exactly `next_u32() as f64 * K`.

The helper returns its variate on the FPU stack, so the harness captures ST0
via an injected store stub rather than a lossy register read.

CAVEAT (recorded in the vector file): the `ln` is computed with x87 FYL2X,
which unicorn implements in softfloat. If that differs from real x87 hardware,
these vectors describe the emulator rather than the retail CPU. Treat a Rust
match here as necessary-but-not-sufficient until a hardware capture confirms.
"""

import struct

from harness import call, write_vectors

GAUSS_FN = 0x005980C0
CALLBACK = 0x00598000  # unit-draw callback the generator installs
CTRL_BLOCK = 0x00ABDFB8  # helper's cache + callback pointer
RNG_GLOBAL = 0x00ABE890  # g_MapGenRng
RNG_STRUCT_LEN = 0xC + 250 * 4

SEEDS = (1234, 0xFFFF)
CALLS_PER_SEED = 8


def control_block(cached_flag: int = 0, cached_value: float = 0.0) -> bytes:
    """Build the helper's control block: flag, cached double, callback ptr."""
    blob = bytearray(0x18)
    blob[0] = cached_flag
    struct.pack_into("<d", blob, 0x08, cached_value)
    struct.pack_into("<I", blob, 0x10, CALLBACK)
    return bytes(blob)


def seeded_rng(seed: int) -> bytes:
    from gen_rng_vectors import seeded_struct

    return seeded_struct(seed)


if __name__ == "__main__":
    vectors = {
        "source": "unicorn/gamemd.exe",
        "fn": hex(GAUSS_FN),
        "callback": hex(CALLBACK),
        "ctrl_block": hex(CTRL_BLOCK),
        "caveat": (
            "ln computed via x87 FYL2X under unicorn softfloat; may differ "
            "from real x87 hardware. Necessary-but-not-sufficient evidence."
        ),
        "cases": [],
    }

    for seed in SEEDS:
        rng_state = seeded_rng(seed)
        ctrl = control_block()
        for index in range(CALLS_PER_SEED):
            result = call(
                GAUSS_FN,
                ecx=CTRL_BLOCK,
                writes={RNG_GLOBAL: rng_state, CTRL_BLOCK: ctrl},
                dumps={
                    "rng": (RNG_GLOBAL, RNG_STRUCT_LEN),
                    "ctrl": (CTRL_BLOCK, 0x18),
                },
                capture_st0=True,
            )
            vectors["cases"].append(
                {
                    "seed": seed,
                    "call": index,
                    "value_bits": f"{result['st0_bits']:016x}",
                    "value": result["st0"],
                    "cached_flag": bytes.fromhex(result["dumps"]["ctrl"])[0],
                }
            )
            print(
                f"seed {seed:>6} call {index}: {result['st0']:+.17e} "
                f"bits={result['st0_bits']:016X} "
                f"cached={bytes.fromhex(result['dumps']['ctrl'])[0]}"
            )
            # Carry both the RNG state and the cache forward, exactly as the
            # generator does across consecutive draws.
            rng_state = bytes.fromhex(result["dumps"]["rng"])
            ctrl = bytes.fromhex(result["dumps"]["ctrl"])

    write_vectors("x87.json", vectors)
