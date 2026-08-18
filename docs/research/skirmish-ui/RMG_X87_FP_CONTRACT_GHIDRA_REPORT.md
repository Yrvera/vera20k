# RMG Floating-Point Contract (FPU mode, sqrt, Box-Muller) — Ghidra Research Report

**Address(es):** `0x005980C0` (Gaussian/Box-Muller helper), `0x00598000` (unit-draw
callback), `0x004CAC40` (approximate sqrt), `0x008650BC` (sqrt lookup table),
`0x007C5F00` (`_ftol2`), `0x00822D80` (ambient FPU control word), constants
`0x007E1718` (1.0), `0x007E2800` (0.0), `0x007ED898` (range constant)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** The floating-point execution contract the random-map generator
runs under: FPU control word, the approximate square root, and the exact
Box-Muller sequence including its cache. Everything needed to reproduce the
generator's FP results bit-for-bit.
**Non-Scope:** the terrain phases that consume these values; `FYL2X` internal
accuracy on real silicon (see §7 Open).
**Confidence:** High for the instruction sequences, the control word, and the
sqrt mechanism (all read from disassembly + confirmed by emulation). Medium for
the `ln` low bits (see §7).
**Active in YR:** Conditional — reached only on the random-map generation path,
but the control word (§2) is process-wide.

## 1. Why this matters

Three findings here each independently break naive reproduction. Any Rust port
that uses ordinary `f64` arithmetic and `f64::sqrt` will diverge from the
original in the low bits, and those bits reach gameplay through `ftol`
truncations that decide cell counts, patch sizes, and hill heights.

## 2. The ambient FPU control word is 0x0E7F (53-bit, truncate)

`_ftol2 @ 0x007C5F00` is the CRT's float-to-int helper. It reads the current
control word, compares it against `[0x00822D80]`, and — when they differ —
loads that value and **never restores the previous one**:

```
FNSTCW [ESP]
CMP    EDX, [0x00822D80]
JNZ    slow
FISTP  qword [EAX]          ; fast path: CW already correct
RET
slow:
MOV    EDX, [0x00822D80]
FLDCW  [ESP]                ; load and LEAVE it set
FISTP  qword [EAX]
RET
```

`[0x00822D80] = 0x0E7F` (verified via `read_memory 0x00822D80` → `7F 0E 00 00`):

| Field | Bits | Value | Meaning |
|---|---|---|---|
| Exception masks | 0–5 | `111111` | all masked |
| Precision control | 8–9 | `10` | **53-bit (double)** |
| Rounding control | 10–11 | `11` | **round toward zero (truncate)** |

**Consequence:** after the first float-to-int conversion anywhere in the
process, every subsequent x87 add/sub/mul/div/sqrt rounds *toward zero* at
53-bit precision. Rust's `f64` operators always round to nearest-even.
Reproduction therefore needs truncating arithmetic, not plain `f64` math.
Implemented as `TruncF64` in `src/map/rmg/x87.rs`; `util::native_x87::X87Chop53`
covers similar ground but could not be reused (private fields, no division, and
it is still uncommitted).

Measured impact: modelling the Box-Muller chain with round-to-nearest `f64`
matches only **2 of 16** emulated results (max error 24 ULP); the same model
scored 11/16 against vectors captured at the *wrong* control word — i.e. the
rounding mode alone is worth several ULP per value.

## 3. `sqrt` is a table-driven single-precision approximation — NOT `FSQRT`

`0x004CAC40` never issues `FSQRT`. It is a fast approximation:

```
FLD   double [ESP+4]
FCOM  float [0x007E1748]      ; == 0.0 ? return it
...
FSTP  float [ESP+4]           ; <-- input NARROWED TO SINGLE PRECISION
MOV   ECX,[ESP+4]
MOV   EAX,ECX
AND   ECX,0x7FFFFF            ; mantissa
SHR   EAX,0x17                ; biased exponent
SUB   EAX,0x7F                ; unbias
TEST  AL,0x1
JZ    +                       ; odd exponent -> set implicit bit
OR    ECX,0x800000
SAR   AX,0x1                  ; halve exponent (16-bit arithmetic shift)
MOVSX EAX,AX
SHR   ECX,0xA                 ; 14-bit table index
ADD   EAX,0x7F                ; rebias
MOV   ECX,[ECX*0x4 + 0x8650BC]   ; table lookup -> result mantissa
SHL   EAX,0x17
ADD   ECX,EAX                 ; recombine exponent + mantissa
MOV   [ESP+4],ECX
FLD   float [ESP+4]           ; <-- RESULT IS SINGLE PRECISION
RET
```

Key points:
- The input is **stored as a `float` first**, discarding everything below ~2⁻²⁴.
- The result is **also single precision**, then widened back to double.
- The table at `0x008650BC` has 16384 entries × 4 bytes and stores **result
  mantissa bits only**; the exponent is computed by halving and rebiasing.

**Consequence:** the returned "square root" is far coarser than 53 bits — see
the accuracy note below, which measures it at ~2^-14 rather than the ~24 bits a
first reading of the single-precision store suggests. This — not `FYL2X` — is
the dominant source of divergence from an exact `sqrt`. Modelling it correctly
moved a test model from 0/16 exact to 11/16 exact against same-configuration
vectors.

**Table provenance — RESOLVED 2026-07-20.** The table is data inside the retail
binary and must NOT be copied into this (public) repository, but it is pure
arithmetic and is therefore *derived* instead:

```
index < 8192 :  m = 1 + index/8192              (even exponent, m in [1,2))
index >= 8192:  m = 2 * (1 + (index-8192)/8192) (odd exponent,  m in [2,4))
table[index]  = trunc((sqrt(m) - 1) * 2^23)
```

Verified to reproduce **all 16384 entries exactly**. Implemented in
`src/map/rmg/x87.rs::sqrt_table_entry`, computed on demand — no table ships.

**Accuracy:** the index keeps only the top 14 bits of the significand, so the
result carries roughly **2^-14** relative accuracy — substantially coarser than
even single precision. `sqrt(1e-6)` returns `0.0009999456` against a true
`0.001` (5.4e-5 relative).

## 4. Box-Muller helper `0x005980C0` — exact sequence

Control block (the global at `0x00ABDFB8`, installed by the generator entry):

| Offset | Type | Purpose |
|---|---|---|
| +0x00 | u8 | cached-value flag |
| +0x08 | f64 | cached second variate |
| +0x10 | ptr | callback returning a uniform `[0,1)` on ST0 |

The callback is `0x00598000`, which is exactly
`(double)Random__Next(g_MapGenRng) * [0x007ED898]` — i.e. the project's
`next_unit()`.

Sequence:

```
if cached_flag != 0:
    cached_flag = 0
    return cached_value              ; no RNG consumed
loop:
    x  = 2*callback() - 1.0
    y  = 2*callback() - 1.0
    r2 = y*y + x*x                   ; y*y computed first, then + x*x
    if !(r2 < 1.0):   goto loop      ; FCOM vs 1.0, C0 clear -> retry
    if r2 == 0.0:     goto loop      ; FCOM vs 0.0, C3 set  -> retry
ln_r2 = FYL2X(ln2, r2)               ; = ln(2) * log2(r2)
t     = (-ln_r2 - ln_r2) / r2        ; note: negate-and-subtract, not *2
scale = approx_sqrt(t)               ; the §3 table routine
cached_value = scale * y
cached_flag  = 1
return scale * x
```

Tiny details that matter:
- **Each rejection consumes two draws** and the retry re-enters *after* a
  `FSTP ST0` that pops the failed `r2`.
- `-2·ln(r2)` is computed as `(-t) - t`, which is exact for doubles and
  therefore equals `-2.0 * t`; no divergence here.
- The cache means **alternate calls consume zero RNG draws**. Confirmed
  empirically: the cached flag alternates 1,0,1,0 across successive calls.
- `r2 >= 1.0` is expressed as "not less than", so a `r2` of exactly 1.0 retries.

## 5. Verification performed

- Emulated `0x005980C0` under unicorn with the control block wired to the real
  callback and `g_MapGenRng` seeded from the verified RNG, capturing ST0 through
  an injected `FSTP qword` stub (unicorn's ST-register read is lossy).
  Vectors: `tools/rmg_oracle/vectors/x87.json` (2 seeds × 8 calls).
- Control-word sensitivity confirmed by regenerating the same vectors at
  `0x037F` vs `0x0E7F` — results differ in the low 1–2 hex digits.
- Model convergence: exact-`sqrt` model 0/16 → table-`sqrt` model 11/16 exact
  (remainder ≤3 ULP, attributable to the rounding mode not yet modelled).

## 6. Two rounding details found during implementation

Both were caught by bisecting a 2-ULP mismatch with an exact-arithmetic
reference, and both change generated output:

1. **The unit-draw callback's multiply truncates.** `0x00598000` is
   `FILD` (draw as integer) then `FMUL` by the range constant — under
   round-toward-zero. Computing `draw as f64 * K` with ordinary Rust
   arithmetic rounds to nearest and drifts by an ulp on some draws, which then
   propagates through the whole Box-Muller chain.
2. **The uniform helper scales before normalising.** `0x00598030` computes
   `draw * span * K + min`, i.e. it multiplies by the span *first* and only
   then applies the range constant. The natural Rust phrasing
   `next_unit() * span + min` groups the operations differently and rounds
   differently.

## 7. Open

- **`FYL2X` fidelity.** unicorn inherits QEMU's x87 transcendental helpers,
  which approximate rather than reproduce hardware. Because §3 narrows the sqrt
  input to single precision, `ln` errors below ~2⁻²⁴ relative are mostly
  discarded — but not always (values landing near a float rounding boundary).
  A real-hardware capture is needed to certify. 32-bit MSVC with inline `__asm`
  is available on this machine (`.../Hostx64/x86/cl.exe`), which is the
  cheapest route: set CW to 0x0E7F, run the §4 sequence, dump the bits.
- ~~Sqrt table index encoding~~ — RESOLVED, see §3.

  Note that the `ln` question is now the *only* thing separating this module
  from full certification: `src/map/rmg/x87.rs` reproduces all 16 emulated
  Box-Muller vectors bit-exactly, so if unicorn's `FYL2X` matches hardware,
  the Rust matches hardware too.

## 7. Implementation Handoff

| Verified behavior | Evidence | Rust status (2026-07-20) | Affected surface | Required effect | Acceptance | Risk / do-not-do |
|---|---|---|---|---|---|---|
| All generator FP runs at 53-bit precision, truncate toward zero | `0x007C5F00`, `[0x00822D80]`=0x0E7F | **done** (`TruncF64`) | `src/map/rmg/x87.rs` | route every add/sub/mul/div through the truncating emulation in `util::native_x87` | Box-Muller vectors match bit-exactly | Do NOT use plain `f64` operators; they round to nearest |
| `sqrt` is a table approximation (~2^-14) | `0x004CAC40`, table `0x008650BC` | **done** (derived, not shipped) | `src/map/rmg/x87.rs` | port the narrow-to-float, index, table-lookup, recombine sequence | matches emulated values | Do NOT call `f64::sqrt`; it is ~29 bits too accurate. Do NOT commit the extracted table — public repo |
| Box-Muller caches the second variate; alternate calls consume no draws | `0x005980C0` +0x00/+0x08 | **done** (`Gaussian`) | `src/map/rmg/x87.rs` | model the cache; draw accounting depends on it | cached flag alternates 1,0,1,0 | Do NOT regenerate both variates per call — it desyncs the draw stream |
| Rejection consumes two draws per failed attempt | `0x005980C0` retry edge | **done** | same | re-draw both, do not reuse | stream stays aligned across rejections | — |

## Sources

- Ghidra read-only: `disassemble_function 0x005980C0`, `0x004CAC40`,
  `0x007C5F00`, `0x0065C780`; `read_memory 0x00598000`, `0x00822D80`,
  `0x007E1718`, `0x007E2800`, `0x008650BC`.
- Emulation: `tools/rmg_oracle/harness.py` (unicorn 2.1.4),
  `gen_x87_vectors.py`, vectors in `tools/rmg_oracle/vectors/x87.json`.
- Related: `RMG_RNG_SEED_MAPGENRNG_GHIDRA_REPORT.md` (the draw source),
  `RMG_TERRAIN_SHAPING_CORE_GHIDRA_REPORT.md` (the consumers).
