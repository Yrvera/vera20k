# Random Map Generator — Implementation Plan (Phase 1: Instruments + Foundation)

> **For Claude:** Execute task-by-task. Each task is self-contained. Do not skip
> the instrument tasks (1–5) — every later phase depends on them being green.

**Goal:** Stand up the RMG instruments (gamemd-derived golden-vector harness,
x87 op set, exact RNG) and the generator foundation (options/.SED/settings/
scratch/emit/launch seam), so terrain phases can then be implemented bit-exactly.

**Architecture:** New `src/map/rmg/` module owns the generator. It emits an
in-memory `MapFile` and plugs into the existing `MapLoadInitial` seam in
`src/app_init.rs`; everything downstream (ResolvedTerrainGrid → atlas → spawn)
is untouched. Nothing in `sim/` changes.

**Design Doc:** [2026-07-19-random-map-generator-design.md](2026-07-19-random-map-generator-design.md)

---

## Scope of THIS plan

Covers design phases **P0 (instruments)** and **P1-foundation**. It ends with:
a validated RNG + x87 op set, a complete options/.SED/settings model, the
scratch/emit skeleton, the launch branch, and a determinism harness — i.e. the
generator can be driven end-to-end and produce a (still terrain-less) `MapFile`.

**Deliberately sequenced into follow-up plans (NOT cut):**
- **Plan 2 — terrain phases** (water, regions, green spread, hills, LAT/trees/
  rocks, starts, tech buildings, tiberium). *Reason: hard dependency.* Every one
  of these consumes the Gaussian helper and f64 chains; they cannot be written
  bit-exactly until Task 4 proves the x87 `ln`/`sqrt`/`div` path against the
  binary. Writing them first would bake in unverified FP.
- **Plan 3 — dialog + preview + launch UX** (design P2).
- **Plan 4 — map types 3/4** (design P3; research complete, see
  `RMG_MODE34_WATER_BRIDGES_TECH_GHIDRA_REPORT.md`).

No parity item from the design ledger is dropped; each is assigned to a plan.

---

## Grounding Summary

- **docs/research/ (all RMG docs re-audited or written 2026-07-19/20):**
  `RMG_TERRAIN_SHAPING_CORE` (hills/LAT/trees/rocks/green-spread formulas),
  `RMG_TIBERIUM_FIELD_COUNT_AND_GATES_RECHECK` (RED→GREEN re-derivation),
  `RMG_MODE34_WATER_BRIDGES_TECH` (river/lake/bridge/tech), `RMG_RNG_SEED_MAPGENRNG`
  (PATCHED-to-GREEN), `RMG_WATER_SEED_0059A6C0`, `RMG_REGION_PARTITION_0058CF90`,
  `RMG_START_GENERATION_*`, `RMG_START_POINT_SCORING_*`,
  `SKIRMISH_RANDMAP_SED_WRITER_FULL_LAYOUT` (GREEN),
  `SKIRMISH_RANDOM_MAP_SETUP_DIALOG_CONTROLS_OPTIONS` (GREEN).
- **Ghidra verification:** performed live during the 2026-07-19/20 session
  (8-doc audit swarm + 3 re-investigations + parent spot-checks). **Ghidra MCP is
  DOWN as of this plan's writing** — every binary claim below carries the citation
  recorded in the patched docs; `/review-plan` should re-verify the ones tagged
  *re-verify* once the bridge is back (`/ghidra-up`).
- **Repo pattern mirrored:** `src/map/map_file.rs` (map data model, all fields
  `pub`), `src/map/theater.rs::TilesetLookup` (flat tile-index authoring),
  `src/util/ini_writer.rs` (INI mutation), `src/sim/rng.rs` (RNG module shape —
  structure copied, algorithm NOT reused).
- **INI keys:** none in `rules(md).ini` (verified by full-corpus grep). Generator
  config lives in **`RMGMD.INI`** inside `ra2md.mix` (extracted 2026-07-20, values
  in Task 8). Theater tile identities come from `temperat(md).ini [General]`
  (`ClearTile`, `RampBase`, `RoughTile`, `SandTile`, `GreenTile`,
  `ClearToSandLat`, `ClearToGreenLat`, `WaterSet`).
- **Still unknown after grounding:** live `g_DirectionOffsets` values (runtime-
  initialized, zero on disk); bridge-deck helper internals; TREE00 lookup-miss
  behavior. All are Plan-2/Plan-4 concerns and listed under Deferred.

## Key Technical Decisions

- **RMG gets its own RNG (`RmgRng`), not `sim::rng::SimRng`.** The native map-gen
  generator is a distinct machine (hash-seeded 250-dword lag-103 XOR with a
  non-exact 2⁻³² reduction constant). Sharing would drift both.
  **Confidence:** high — **Source:** `RMG_RNG_SEED_MAPGENRNG` (PATCHED-to-GREEN),
  disassembly of `0x0065C6D0`/`0x0065C780` this session.
- **RMG owns its x87 ops in `src/map/rmg/x87.rs`; `src/util/native_x87.rs` is
  consumed read-only, never modified.** That file is currently **untracked**
  (parallel session in progress) and lacks `div`/`sqrt`/`ln`.
  **Confidence:** high — **Source:** `git status --porcelain` = `??`; API grep
  shows `add/sub/mul/neg/compare/store_f32/store_f64/ftol_i64` only.
- **Generator returns a `MapFile`; integration is a branch inside
  `load_map_initial_with_assets`.** `MapLoadInitial` fields are private, so a
  `pub(crate)` constructor is added in `app_init.rs`.
  **Confidence:** high — **Source:** `src/app_init.rs:143`, `:322`, `:383` read
  this session.
- **Golden vectors come from a unicorn-engine harness, not `emulate_function`.**
  The 2026-07-20 spike proved `emulate_function` returns registers only and
  faults on `Random__Next`'s `PUSH`. **Confidence:** high — **Source:** spike
  recorded in design doc Testing Strategy.
- **Map dimensions come from a scale formula, not fixed buckets.**
  `scale = WidthOption * 0.33333334`, capped at 1.2 unless map type is 3/4;
  `Size = 0,0,genW+4,genH+12`; `LocalSize = 2,5,genW,genH`. Every cell is also
  initialised to **level 4**, not 0. **Confidence:** high — **Source:**
  `decompile_function 0x00599650` 2026-07-20 (this replaced an *inferred*
  64/96/128/160 bucket table that `/review-plan` flagged and the binary refuted).
- **The Python reference impl is a starting point, not a golden.**
  `docs/research/skirmish-ui/rmg_rng_reference_impl.py` self-checks but is
  **UNVERIFIED vs binary**. **Confidence:** medium (transcription risk) —
  flagged for `/review-plan`; Task 3 is what makes it authoritative.

## Open Questions

### Resolved during planning
- *Where does the generator plug in?* → `load_map_initial_with_assets`
  (`src/app_init.rs:322`), returning `MapLoadInitial` (`:143`).
- *Is `MapFile` constructible in memory?* → Yes, all fields `pub`
  (`src/map/map_file.rs:156-178`).
- *Does RMGMD.INI exist?* → Yes, in `ra2md.mix` and `ra2.mix`; values in Task 8.
- *Defaults when RMGMD.INI is absent?* → outer ctor `0x00595740`: min/max
  tiberium 2500/5500, MaxTrees 500, `+0x30C`=4, `+0x310`=0. (The earlier
  "zero trees" claim was refuted.)

### Deferred to implementation
- **`g_DirectionOffsets` numeric values** — runtime-initialized; needs one live
  debugger read. Blocks any neighbor iteration in Plan 2, not this plan.
- **x87 `ln` (FYL2X) exact semantics** — Task 4 determines whether a table-driven
  reproduction is needed or whether `f64::ln` + chop-53 suffices. Cannot be known
  until vectors exist.
- **`.SED` write byte-formatting** (spacing/order tolerance) — native reads
  key-by-key with carry defaults, so exact byte layout may not matter; Task 7
  asserts round-trip equality instead.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `tools/rmg_oracle/harness.py` | unicorn-based gamemd golden-vector harness |
| Create | `tools/rmg_oracle/vectors/*.json` | generated golden vectors (RNG, x87) |
| Create | `src/map/rmg/mod.rs` | module root, `generate()` order owner |
| Create | `src/map/rmg/options.rs` | `RmgOptions`, clamps, `.SED` read/write |
| Create | `src/map/rmg/settings.rs` | `RmgSettings` from RMGMD.INI |
| Create | `src/map/rmg/rng.rs` | `RmgRng` (seed hash + lag-103 draw + helpers) |
| Create | `src/map/rmg/x87.rs` | div/sqrt/ln + Box-Muller over `native_x87` |
| Create | `src/map/rmg/scratch.rs` | `RmgScratch` cell array + diamond bounds |
| Create | `src/map/rmg/emit.rs` | scratch → `MapFile` |
| Create | `src/map/rmg/tests/vectors.rs` | golden-vector regression tests |
| Modify | `src/map/mod.rs` | `pub mod rmg;` |
| Modify | `src/app_init.rs` | `.SED` branch + `MapLoadInitial::from_parts` |
| Modify | `src/skirmish_scenarios.rs:14-17` | sentinel capacity 2..8 |
| Read-only | `src/util/native_x87.rs` | consumed; **never edited** |

## Interface Changes

- **New public (crate) API:** `map::rmg::generate(&RmgOptions, &RmgSettings,
  &TheaterData, &AssetManager) -> Result<GeneratedMap>`. No existing consumers.
- **`MapLoadInitial::from_parts(AssetManager, MapFile) -> Self`** — new
  `pub(crate)` constructor in `app_init.rs`. Only the new branch calls it.
- **`SkirmishScenarioRecord::random_map_sentinel`** — capacity constants change
  from max 4 to max 8. Consumers: `src/ui/skirmish_shell/state/choose_map.rs`,
  `src/app_skirmish.rs` start assignment. Both read the value; neither hardcodes 4.

## Sim Checklist

**Not applicable — no task in this plan touches `sim/`.** The generator lives in
`map/` (pre-play map construction). Confirmed against the design's architecture
rule: generation runs during scenario load, before `Post_Map_Init`, and its
output is an ordinary `MapFile`. No tick-order, state-hash, or EntityStore impact.
The f64/x87 usage here is explicitly outside sim logic and is therefore permitted;
Task 12 adds a guard test asserting `map::rmg` is not referenced from `sim/`.

## Risk Areas

| Risk | Mitigation |
|---|---|
| `native_x87.rs` is untracked; a parallel session may change/remove it | RMG's `x87.rs` wraps it behind a thin local trait; if it vanishes, only that shim changes. Task 4 pins the exact API used. |
| Python reference impl may be mis-transcribed | Task 3 locks it against harness output before any consumer exists. |
| Unicorn harness may not load the PE cleanly | Task 1 has an explicit fallback (live-gamemd debugger capture) and a stop-condition. |
| `.SED` carry-default semantics misapplied | Task 6 tests default-carry explicitly with a partial `.SED`. |
| Sentinel capacity change breaks start assignment | Task 9 runs the existing skirmish launch tests as regression. |

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| 3 | RNG seed-hash + draw stream | Every terrain decision derives from it; one wrong dword ⇒ a completely different map | Harness vectors vs Rust, bit-exact |
| 4 | x87 `ln`/`sqrt`/`div` + Box-Muller | Drives hills, patch sizes, tree density, ore sizes; 1-ULP drift changes cell outcomes | Harness vectors on `0x005980C0`, bit-exact |
| 4 | `ftol` truncate-toward-zero (CW 0x0E7F) | Every formula ends in a truncation; round-vs-truncate flips cells at boundaries | Vector test incl. negatives |
| 3 | Range-reduction constant `0x3DF0000000100000` (NOT 1/2³²) | A `1.0/4294967296.0` literal silently diverges | Bit-pattern assertion test |
| 3 | Rejection-loop draw counts | Extra/missing draws desync the whole downstream stream | Vector test counting draws per helper call |
| 6 | Normalizer clamps incl. **Theater unclamped** | Wrong clamp changes generated map identity | Unit test per field bound |
| 8 | RMGMD.INI-absent defaults (2500/5500/500) | Wrong default ⇒ wrong ore totals and zero-vs-600 trees | Unit test with settings absent |
| 9 | Sentinel capacity 2..8 | Native clamps NumPlayers 2..8; max 4 blocks 5–8 player random maps | Unit test + launch regression |
| 12 | `Size`/`LocalSize` padding + `local_top = 5` | Wrong header geometry shifts every cell and the playable window | `0x00599650` write sites |
| 12 | Cells initialise to level 4 (not 0) | Hills/terracing deltas are relative to it; starting at 0 shifts the whole heightfield | `0x00599650` cell-init loop |
| 11 | Scratch stride 0x50 + field offsets | Every phase reads/writes these; a wrong offset corrupts all downstream phases | Layout unit test |
| 13 | `generate()` stage order incl. green-spread placement | Order determines what each phase sees; native order is the spec | Order assertion test (recorded phase log) |

---

## Tasks

### Task 1: Build the gamemd golden-vector harness

**Why:** Everything parity-critical in this plan is verified against gamemd-derived
vectors. The 2026-07-20 spike proved Ghidra's `emulate_function` cannot produce
them (register-only return; faults on `PUSH`). This is the blocking instrument.

**Files:** Create `tools/rmg_oracle/harness.py`

**Pattern:** New tooling (no existing equivalent). Local-only, not committed to
the public repo path rules — `tools/rmg_oracle/` is fine to commit (it contains
no retail data, only addresses).

**Step 1: Install and verify unicorn**
```bash
python -m pip install "unicorn>=2.1.1"   # 2.0.x imports distutils (gone in py3.12+)
python -c "import unicorn; print(unicorn.__version__)"
```
Expected: prints a version (verified 2026-07-20 with unicorn 2.1.4 on Python
3.13). If install fails, STOP and report — fallback is a live-gamemd debugger
capture (see Step 5 fallback note).

**Step 2: Write the harness**
```python
# tools/rmg_oracle/harness.py
"""Golden-vector oracle for RMG parity: runs real gamemd.exe code under unicorn.

Maps the PE's sections, gives the emulated CPU a real stack, calls a target
function with a chosen calling convention, and dumps chosen memory ranges.
Produces machine-derived goldens (never hand-computed) per CLAUDE.md.
"""
import json
import struct
from pathlib import Path

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import (
    UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_EAX, UC_X86_REG_EIP,
)

GAMEMD = Path(r"<ra2-install>/gamemd.exe")
IMAGE_BASE = 0x00400000
IMAGE_SIZE = 0x00A00000          # covers .text/.rdata/.data of gamemd
STACK_BASE = 0x10000000
STACK_SIZE = 0x00100000
SCRATCH = 0x20000000             # writable scratch for struct outputs
SCRATCH_SIZE = 0x00010000
RET_MAGIC = 0x30000000           # sentinel return address; stops emulation


def _load_image(uc: Uc) -> None:
    """Map the PE by section headers so RVA-addressed globals resolve."""
    data = GAMEMD.read_bytes()
    pe_off = struct.unpack_from("<I", data, 0x3C)[0]
    n_sections = struct.unpack_from("<H", data, pe_off + 6)[0]
    opt_size = struct.unpack_from("<H", data, pe_off + 20)[0]
    sec_off = pe_off + 24 + opt_size
    uc.mem_map(IMAGE_BASE, IMAGE_SIZE)
    # headers themselves
    uc.mem_write(IMAGE_BASE, data[:0x1000])
    for i in range(n_sections):
        off = sec_off + i * 40
        vaddr = struct.unpack_from("<I", data, off + 12)[0]
        rawsz = struct.unpack_from("<I", data, off + 16)[0]
        rawptr = struct.unpack_from("<I", data, off + 20)[0]
        if rawsz:
            uc.mem_write(IMAGE_BASE + vaddr, data[rawptr:rawptr + rawsz])


def call(func: int, *, ecx: int | None = None, stack_args: list[int] | None = None,
         writes: dict[int, bytes] | None = None,
         dumps: dict[str, tuple[int, int]] | None = None,
         timeout_instr: int = 2_000_000) -> dict:
    """Call `func`; return {'eax': int, 'dumps': {name: hex}}.

    ecx        -> __thiscall receiver
    stack_args -> pushed right-to-left above the sentinel return address
    writes     -> {addr: bytes} preloaded into scratch/struct memory
    dumps      -> {name: (addr, length)} read back after the call
    """
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    _load_image(uc)
    uc.mem_map(STACK_BASE, STACK_SIZE)
    uc.mem_map(SCRATCH, SCRATCH_SIZE)
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

    uc.emu_start(func, RET_MAGIC, count=timeout_instr)
    out = {"eax": uc.reg_read(UC_X86_REG_EAX) & 0xFFFFFFFF, "dumps": {}}
    for name, (addr, length) in (dumps or {}).items():
        out["dumps"][name] = uc.mem_read(addr, length).hex()
    return out


def write_vectors(path: str, obj: dict) -> None:
    p = Path(__file__).parent / "vectors" / path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(obj, indent=2))
    print(f"wrote {p}")
```

**Step 3: Smoke-test the loader**
```bash
cd tools/rmg_oracle && python -c "
from harness import call
# Random__Seed 0x0065C6D0: __thiscall(this=ECX, seed=stack), fills this+0xC..
r = call(0x0065C6D0, ecx=0x20000000, stack_args=[1234],
         dumps={'struct': (0x20000000, 0x3F4)})
print('eax', hex(r['eax']))
print('state0..3', r['dumps']['struct'][24:56])
"
```
Expected: prints `eax 0x20000000` and 32 hex chars of state. If unicorn faults,
STOP — do not proceed to Task 2; report the fault.

**Fallback (only if Step 1 or 3 is unrecoverable):** capture the same vectors from
a live gamemd under a debugger by breakpointing `0x0065C6D0` return and dumping
`[ECX]`. Record which path was used in the vectors JSON `"source"` field.

**Step 4: Commit**
```bash
git add tools/rmg_oracle/harness.py && git commit -m "tools: unicorn-based RMG golden-vector oracle"
```

---

### Task 2: Generate and store RNG golden vectors

**Why:** Turns the harness into the authoritative reference for `RmgRng`.

**Files:** Create `tools/rmg_oracle/gen_rng_vectors.py`, `tools/rmg_oracle/vectors/rng.json`

**Step 1: Write the generator**
```python
# tools/rmg_oracle/gen_rng_vectors.py
"""Machine-derived RNG vectors: seeded state + first N draws, for several seeds."""
import struct
from harness import call, write_vectors

SEED_FN = 0x0065C6D0     # Random__Seed  __thiscall(this, seed)
NEXT_FN = 0x0065C780     # Random__Next  __thiscall(this) -> EAX
STRUCT = 0x20000000
STRUCT_LEN = 0xC + 250 * 4

def seeded_struct(seed: int) -> bytes:
    r = call(SEED_FN, ecx=STRUCT, stack_args=[seed],
             dumps={"s": (STRUCT, STRUCT_LEN)})
    return bytes.fromhex(r["dumps"]["s"])

def draws(state: bytes, n: int) -> tuple[list[int], bytes]:
    """Chain n calls to Random__Next, carrying the struct forward."""
    cur, out = state, []
    for _ in range(n):
        r = call(NEXT_FN, ecx=STRUCT, writes={STRUCT: cur},
                 dumps={"s": (STRUCT, STRUCT_LEN)})
        out.append(r["eax"])
        cur = bytes.fromhex(r["dumps"]["s"])
    return out, cur

if __name__ == "__main__":
    vectors = {"source": "unicorn/gamemd.exe", "seed_fn": hex(SEED_FN),
               "next_fn": hex(NEXT_FN), "cases": []}
    for seed in (0, 1, 1234, 0x7FFF, 0xFFFF):
        st = seeded_struct(seed)
        idx_a, idx_b = struct.unpack_from("<II", st, 4)
        d, _ = draws(st, 16)
        vectors["cases"].append({
            "seed": seed,
            "locked": st[0],
            "idx_a": idx_a, "idx_b": idx_b,
            "state_hex": st[0xC:].hex(),
            "draws": [f"{x:08x}" for x in d],
        })
    write_vectors("rng.json", vectors)
```

**Step 2: Run it**
```bash
cd tools/rmg_oracle && python gen_rng_vectors.py
```
Expected: `wrote .../vectors/rng.json`. Sanity-check that every case has
`idx_a == 0`, `idx_b == 103`, `locked == 0`, 250 dwords of state, 16 draws.

**Step 3: Cross-check the Python reference impl**
```bash
cd <repo> && python - <<'EOF'
import json, sys
sys.path.insert(0, "docs/research/skirmish-ui")
from rmg_rng_reference_impl import seed_fill, rng_next
v = json.load(open("tools/rmg_oracle/vectors/rng.json"))
bad = 0
for c in v["cases"]:
    st = seed_fill(c["seed"])
    want = [int(c["state_hex"][i*8:(i+1)*8], 16) for i in range(250)]
    # vectors are little-endian bytes; convert
    raw = bytes.fromhex(c["state_hex"])
    want = [int.from_bytes(raw[i*4:i*4+4], "little") for i in range(250)]
    if st != want:
        print("SEED MISMATCH", c["seed"], "first diff at",
              next(i for i,(a,b) in enumerate(zip(st,want)) if a!=b)); bad += 1
        continue
    ia, ib, got = 0, 0x67, []
    for _ in range(16):
        val, ia, ib = rng_next(st, ia, ib); got.append(f"{val:08x}")
    if got != c["draws"]:
        print("DRAW MISMATCH", c["seed"], got[:4], c["draws"][:4]); bad += 1
print("MISMATCHES:", bad)
EOF
```
Expected: `MISMATCHES: 0`.
**If nonzero:** the Python transcription is wrong — the harness is ground truth.
Fix `rmg_rng_reference_impl.py` to match, re-run, and note the correction in
`docs/research/AUDIT_LOG.md`. Do NOT proceed with a mismatch.

**Step 4: Commit**
```bash
git add tools/rmg_oracle/gen_rng_vectors.py tools/rmg_oracle/vectors/rng.json
git commit -m "tools: machine-derived RMG RNG golden vectors"
```

---

### Task 3: Implement `RmgRng` against the vectors

**Why:** The determinism spine. Defined before any consumer exists (interfaces first).

**Files:** Create `src/map/rmg/rng.rs`; Modify `src/map/mod.rs`

**Pattern:** Module shape mirrors `src/sim/rng.rs` (struct + `new` + `next_u32`),
algorithm deliberately different (see Key Technical Decisions).

**Step 1: Add the module declaration**
```rust
// src/map/mod.rs — add alongside the existing `pub mod` lines
pub mod rmg;
```
```rust
// src/map/rmg/mod.rs
//! Random Map Generator: reproduces gamemd's `.SED`-driven map generation.
//! Depends on `map::theater` for tile identities and `util::native_x87` for
//! deterministic float math. Never depended on by `sim/`.

pub mod rng;
```

**Step 2: Write `RmgRng`**
```rust
// src/map/rmg/rng.rs
//! Exact reproduction of gamemd's map-generation RNG: a hash-seeded,
//! 250-dword lag-103 XOR generator with caller-side range reduction.
//! Distinct from `sim::rng::SimRng` — see the RMG design doc.

/// Number of state words. Native buffer is 250 dwords.
const STATE_LEN: usize = 250;
/// Second index starts here; the two cursors stay 103 apart.
const LAG: usize = 0x67;
/// Seed-hash table 1, consumed at indices 0..3.
const TABLE1: [u32; 4] = [0xBAA9_6887, 0x1E17_D32C, 0x03BC_DC3C, 0x0F33_D1B2];
/// Seed-hash table 2. Native pre-increments its index, so only [1..=4] are
/// consumed; index 0 is present for provenance and never read.
const TABLE2: [u32; 5] = [0x48AA_D7E4, 0x4B0F_3B58, 0xE874_F0C3, 0x6955_C5A6,
                          0x55A7_CA46];
/// Range-reduction multiplier. This is NOT bit-exact 2^-32: the native constant
/// carries an extra mantissa bit. A `1.0 / 4294967296.0` literal diverges.
pub const RANGE_K_BITS: u64 = 0x3DF0_0000_0010_0000;

#[derive(Debug, Clone)]
pub struct RmgRng {
    state: [u32; STATE_LEN],
    idx_a: usize,
    idx_b: usize,
}

impl RmgRng {
    /// Seed exactly as the native seeder does: 250 output words, each produced
    /// by four hash rounds that carry the previous round's pre-mangle value.
    pub fn new(seed: u16) -> Self {
        let seed = u32::from(seed);
        let mut state = [0u32; STATE_LEN];
        let mut counter: u32 = 0;
        for slot in state.iter_mut() {
            let mut val = counter;
            counter = counter.wrapping_add(1);
            let mut carry = seed;
            for round in 0..4 {
                let mangled = TABLE1[round] ^ val;
                let pre = val;
                let hi = (mangled as i32 >> 16) as i32;
                let lo = (mangled & 0xFFFF) as i32;
                let hi_hi = !(hi.wrapping_mul(hi)) as u32;
                let hi_lo = hi.wrapping_mul(lo) as u32;
                let lo_lo = lo.wrapping_mul(lo) as u32;
                let sum = hi_hi.wrapping_add(lo_lo);
                let swapped = ((sum as i32 >> 16) as u32) | (sum << 16);
                let mixed = (swapped ^ TABLE2[round + 1])
                    .wrapping_add(hi_lo) ^ carry;
                val = mixed;
                carry = pre;
            }
            *slot = val;
        }
        Self { state, idx_a: 0, idx_b: LAG }
    }

    /// One draw: XOR the lagged pair into the primary slot and return it.
    pub fn next_u32(&mut self) -> u32 {
        let value = self.state[self.idx_a] ^ self.state[self.idx_b];
        self.state[self.idx_a] = value;
        self.idx_a += 1;
        self.idx_b += 1;
        if self.idx_a >= STATE_LEN {
            self.idx_a = 0;
        }
        if self.idx_b >= STATE_LEN {
            self.idx_b = 0;
        }
        value
    }

    /// Native `[0,1)` conversion. Uses the exact binary constant, not 2^-32.
    pub fn next_unit(&mut self) -> f64 {
        f64::from_bits(RANGE_K_BITS) * f64::from(self.next_u32())
    }

    /// Native inclusive uniform helper: scale, truncate, reject above `max`.
    /// The rejection loop is part of the draw contract — it consumes draws.
    pub fn uniform(&mut self, min: i32, max: i32) -> i32 {
        let span = f64::from(max - min + 1);
        loop {
            let raw = self.next_unit() * span + f64::from(min);
            let value = raw as i32; // truncate toward zero, as native ftol
            if value <= max {
                return value;
            }
        }
    }
}
```

**Step 3: Add the vector-locked tests**
```rust
// src/map/rmg/rng.rs — append
#[cfg(test)]
mod tests {
    use super::*;

    /// Loaded from tools/rmg_oracle/vectors/rng.json (machine-derived).
    const VECTORS: &str = include_str!("../../../tools/rmg_oracle/vectors/rng.json");

    #[test]
    fn range_constant_is_not_two_pow_minus_32() {
        assert_ne!(RANGE_K_BITS, (1.0f64 / 4294967296.0).to_bits());
        assert_eq!(RANGE_K_BITS, 0x3DF0_0000_0010_0000);
    }

    #[test]
    fn matches_gamemd_golden_vectors() {
        let doc: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
        for case in doc["cases"].as_array().unwrap() {
            let seed = case["seed"].as_u64().unwrap() as u16;
            let mut rng = RmgRng::new(seed);
            assert_eq!(rng.idx_a, 0, "seed {seed}: idx_a");
            assert_eq!(rng.idx_b, 0x67, "seed {seed}: idx_b");

            let raw = hex_bytes(case["state_hex"].as_str().unwrap());
            for (i, chunk) in raw.chunks_exact(4).enumerate() {
                let want = u32::from_le_bytes(chunk.try_into().unwrap());
                assert_eq!(rng.state[i], want, "seed {seed}: state[{i}]");
            }
            for (i, want) in case["draws"].as_array().unwrap().iter().enumerate() {
                let want = u32::from_str_radix(want.as_str().unwrap(), 16).unwrap();
                assert_eq!(rng.next_u32(), want, "seed {seed}: draw[{i}]");
            }
        }
    }

    fn hex_bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
}
```

**Step 4: Verify**
```bash
cargo test -p vera20k rmg::rng -- --nocapture
```
Expected: `test result: ok.` with both tests passing. Read the literal
`test result:` line — a wrong `-p` exits 101 without running anything.

**Step 5: Commit**
```bash
git add src/map/rmg src/map/mod.rs && git commit -m "map/rmg: exact map-generation RNG locked to gamemd vectors"
```

---

### Task 4: x87 op set + Box-Muller, locked to vectors

> **SPEC CORRECTED 2026-07-20 during execution.** The original task assumed
> plain `f64` arithmetic plus `f64::sqrt`. Investigation (see
> `docs/research/skirmish-ui/RMG_X87_FP_CONTRACT_GHIDRA_REPORT.md`) proved all
> three assumptions wrong. Do not implement from the original text.

**Why:** Highest-risk instrument. Every terrain phase consumes the Gaussian, and
Plan 2 cannot start until this matches bit-exactly.

**Files:** Create `src/map/rmg/x87.rs`. **Do NOT modify `src/util/native_x87.rs`**
(untracked, owned by other work) and **do NOT commit any table extracted from
gamemd.exe** — this repository is public.

**Corrected contract (all verified, evidence in the report above):**

1. **Ambient FPU mode is 53-bit precision, round toward zero** (control word
   `0x0E7F`, read from `[0x00822D80]`; `_ftol2 @ 0x007C5F00` loads it and never
   restores). Every add/sub/mul/div must go through the truncating emulation in
   `util::native_x87::X87Chop53` — plain `f64` operators round to nearest and
   score only 2/16 against the vectors.
   `X87Chop53` provides add/sub/mul/neg/compare/store/ftol but **no div**;
   add a truncating `div` in `rmg/x87.rs`.
2. **`sqrt` is a table-driven single-precision approximation** (`0x004CAC40`),
   NOT `FSQRT`: narrow the input to `f32`, split exponent/mantissa, set the
   implicit bit on odd exponents, halve the exponent with a 16-bit arithmetic
   shift, index a 16384-entry table by `mantissa >> 10`, recombine, return as
   `f32` widened to `f64`. Modelling this moved a test model from 0/16 to 11/16
   exact. The table must be **derived arithmetically**, not shipped.
3. **Box-Muller caches the second variate** — alternate calls consume zero RNG
   draws — and each rejected attempt consumes two draws. Sequence, comparison
   senses, and the `(-t) - t` form are in the report's §4.

**Step 1: Vectors — DONE.** `tools/rmg_oracle/gen_x87_vectors.py` →
`vectors/x87.json` (2 seeds × 8 calls), captured at the correct control word
with ST0 taken through an injected `FSTP qword` stub.

**Step 2: Pin the sqrt table's index encoding.** The even-exponent half matches
a truncated-mantissa sqrt model; the odd half (`i >= 8192`) does not yet. Derive
it, then assert the derived table reproduces the retail table for all 16384
entries (compare against a locally extracted copy kept OUTSIDE the repo).

**Step 3: Resolve `FYL2X` fidelity before trusting the vectors' low bits.**
unicorn inherits QEMU's approximate x87 transcendentals. Because the sqrt
narrows to single precision, most `ln` error is discarded — but not near float
rounding boundaries. Capture from real hardware with the 32-bit MSVC available
at `.../Hostx64/x86/cl.exe`: set CW to `0x0E7F`, run the §4 sequence with
`__asm`, dump the bits, and compare against `vectors/x87.json`. If they differ,
real hardware wins and the vectors must be regenerated from it.

**Step 4: Implement `rmg/x87.rs`** — truncating `div`, the table `sqrt`, `ln`,
and a `Gaussian` with the cache — then lock it to the (hardware-confirmed)
vectors bit-exactly.

**Step 5: Verify**
```bash
cargo test -p vera20k rmg::x87 -- --nocapture
```
Expected: `test result: ok.` Do NOT relax the assertion to an epsilon compare;
bit-exact is the bar. If it fails, the failing input identifies which of the
three mechanisms above is still wrong.

**Step 6: Commit.**

### Task 5: `ftol` truncation helper + tests

**Why:** Every native formula ends in a truncation. Round-vs-truncate flips cells
at boundaries, and negative inputs are where it bites.

**Files:** Modify `src/map/rmg/x87.rs`

**Step 1: Add the helper**
```rust
// src/map/rmg/x87.rs — append to the top-level (not the tests module)

/// Native `ftol`: truncate toward zero under control word 0x0E7F.
/// Rust's `as i32` already truncates toward zero; this names the operation so
/// call sites read like the original and stay auditable.
pub fn ftol(value: f64) -> i32 {
    value as i32
}
```

**Step 2: Add tests**
```rust
// src/map/rmg/x87.rs — inside `mod tests`
    #[test]
    fn ftol_truncates_toward_zero() {
        assert_eq!(ftol(3.9), 3);
        assert_eq!(ftol(-3.9), -3);   // toward zero, NOT floor(-4)
        assert_eq!(ftol(0.9999), 0);
        assert_eq!(ftol(-0.9999), 0);
        assert_eq!(ftol(100.0), 100);
    }
```

**Step 3: Verify**
```bash
cargo test -p vera20k rmg::x87::tests::ftol -- --nocapture
```
Expected: `test result: ok.`

**Step 4: Commit**
```bash
git commit -am "map/rmg: native ftol truncation helper"
```

---

### Task 6: `RmgOptions` + normalizer clamps

**Why:** The options model is the contract every later phase reads. Clamps are
parity-critical and must land before `.SED` parsing consumes them.

**Files:** Create `src/map/rmg/options.rs`; Modify `src/map/rmg/mod.rs`

**Step 1: Declare the module**
```rust
// src/map/rmg/mod.rs — add
pub mod options;
pub mod x87;
```

**Step 2: Write the type and clamps**
```rust
// src/map/rmg/options.rs
//! `[RandomMap]` seed/options model and the native normalizer.

/// One random-map configuration. Field order mirrors the native record so the
/// `.SED` round-trip and the clamp table stay auditable side by side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RmgOptions {
    pub theater: i32,
    pub map_type: i32,
    pub resources: i32,
    pub ruggedness: i32,
    pub time: i32,
    pub water_amount: i32,
    pub num_players: i32,
    pub tiberium: i32,
    pub tiberium_layout: i32,
    pub vegetation: i32,
    pub urban_presence: i32,
    pub width: i32,
    pub height: i32,
    pub accessibility: i32,
    pub region_size: i32,
    pub seed: i32,
}

impl Default for RmgOptions {
    /// Native constructor defaults.
    fn default() -> Self {
        Self {
            theater: 0,
            map_type: 1,
            resources: 1,
            ruggedness: 0,
            time: 1,
            water_amount: 0,
            num_players: 2,
            tiberium: 0,
            tiberium_layout: 0,
            vegetation: 0,
            urban_presence: 0,
            width: 0,
            height: 0,
            accessibility: 0,
            region_size: 0,
            seed: -1,
        }
    }
}

impl RmgOptions {
    /// Native normalizer. Note there is deliberately NO theater clamp: the
    /// native routine never touches that field.
    pub fn normalize(&mut self) {
        self.resources = self.resources.clamp(0, 3);
        self.map_type = self.map_type.clamp(0, 4);
        self.time = self.time.clamp(0, 3);
        self.ruggedness = self.ruggedness.clamp(0, 100);
        self.water_amount = self.water_amount.clamp(0, 100);
        self.num_players = self.num_players.clamp(2, 8);
        self.tiberium = self.tiberium.clamp(1, 100);
        self.tiberium_layout = self.tiberium_layout.clamp(0, 100);
        self.vegetation = self.vegetation.clamp(0, 100);
        self.urban_presence = self.urban_presence.clamp(0, 100);
        self.width = self.width.clamp(0, 3);
        self.height = self.height.clamp(0, 3);
        self.accessibility = self.accessibility.clamp(0, 100);
        self.region_size = self.region_size.clamp(0, 100);
        self.seed = self.seed.clamp(0, 0xFFFF);
    }

    /// Seed as the RNG consumes it (post-normalize it always fits u16).
    pub fn seed_u16(&self) -> u16 {
        self.seed as u16
    }
}
```

**Step 3: Add tests**
```rust
// src/map/rmg/options.rs — append
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theater_is_never_clamped() {
        let mut o = RmgOptions { theater: 99, ..Default::default() };
        o.normalize();
        assert_eq!(o.theater, 99, "native normalizer does not clamp theater");
        let mut o = RmgOptions { theater: -5, ..Default::default() };
        o.normalize();
        assert_eq!(o.theater, -5);
    }

    #[test]
    fn clamp_bounds_match_native() {
        let mut o = RmgOptions {
            resources: 9, map_type: 9, time: 9, ruggedness: 500,
            water_amount: -1, num_players: 1, tiberium: 0,
            tiberium_layout: 500, vegetation: -3, urban_presence: 900,
            width: 7, height: 7, accessibility: 101, region_size: -1,
            seed: 0x1_0000, theater: 0,
        };
        o.normalize();
        assert_eq!((o.resources, o.map_type, o.time), (3, 4, 3));
        assert_eq!((o.ruggedness, o.water_amount), (100, 0));
        assert_eq!(o.num_players, 2, "players clamp low bound is 2");
        assert_eq!(o.tiberium, 1, "tiberium clamp low bound is 1, not 0");
        assert_eq!((o.tiberium_layout, o.vegetation, o.urban_presence), (100, 0, 100));
        assert_eq!((o.width, o.height), (3, 3));
        assert_eq!((o.accessibility, o.region_size), (100, 0));
        assert_eq!(o.seed, 0xFFFF);
    }

    #[test]
    fn defaults_match_native_constructor() {
        let o = RmgOptions::default();
        assert_eq!(o.map_type, 1);
        assert_eq!(o.resources, 1);
        assert_eq!(o.time, 1);
        assert_eq!(o.num_players, 2);
        assert_eq!(o.seed, -1);
        assert_eq!(o.theater, 0);
    }
}
```

**Step 4: Verify**
```bash
cargo test -p vera20k rmg::options -- --nocapture
```
Expected: `test result: ok.` (3 passed)

**Step 5: Commit**
```bash
git add src/map/rmg/options.rs && git commit -m "map/rmg: RandomMap options model + native normalizer clamps"
```

---

### Task 7: `.SED` read/write with carry-default semantics

**Why:** The `.SED` is the feature's persistent contract; carry-defaults are a
native behaviour a naive parser gets wrong (missing key ⇒ keep current value,
not zero).

**Files:** Modify `src/map/rmg/options.rs`

**Pattern:** Reading mirrors `src/map/waypoints.rs:41` style INI section reads;
writing uses `util::ini_writer::set_ini_values`.

**Step 1: Add read/write**
```rust
// src/map/rmg/options.rs — append to the top-level

use crate::rules::ini_parser::IniFile;
use crate::util::ini_writer::set_ini_values;

/// Section every random-map seed file stores its options under.
/// NOTE: `IniFile` lookups are section-scoped — `ini.section(name)?.get_i32(key)`.
/// There is no `IniFile::get(section, key)`, and no `IniFile::parse`; construct
/// with `IniFile::from_bytes(&[u8]) -> Result<_, RulesError>` or `from_str(&str)`.
const SECTION: &str = "RandomMap";

impl RmgOptions {
    /// Apply a `.SED`'s `[RandomMap]` keys over `self`. A missing or malformed
    /// key leaves the existing field untouched — native reads each integer with
    /// the current value as its default. Does not normalize; call `normalize`.
    pub fn apply_sed(&mut self, ini: &IniFile) {
        let Some(section) = ini.section(SECTION) else {
            return;
        };
        let mut read = |key: &str, field: &mut i32| {
            if let Some(value) = section.get_i32(key) {
                *field = value;
            }
        };
        read("Width", &mut self.width);
        read("Height", &mut self.height);
        read("NumPlayers", &mut self.num_players);
        read("Seed", &mut self.seed);
        read("MapType", &mut self.map_type);
        read("Theater", &mut self.theater);
        read("Time", &mut self.time);
        read("RegionSize", &mut self.region_size);
        read("Ruggedness", &mut self.ruggedness);
        read("Accessibility", &mut self.accessibility);
        read("WaterAmount", &mut self.water_amount);
        read("Tiberium", &mut self.tiberium);
        read("TiberiumLayout", &mut self.tiberium_layout);
        read("Vegetation", &mut self.vegetation);
        read("UrbanPresence", &mut self.urban_presence);
        read("Resources", &mut self.resources);
    }

    /// Serialize to `.SED` bytes, writing every key in native order.
    pub fn to_sed_bytes(&self) -> Vec<u8> {
        let values: Vec<(String, String)> = vec![
            ("Width", self.width), ("Height", self.height),
            ("NumPlayers", self.num_players), ("Seed", self.seed),
            ("MapType", self.map_type), ("Theater", self.theater),
            ("Time", self.time), ("RegionSize", self.region_size),
            ("Ruggedness", self.ruggedness),
            ("Accessibility", self.accessibility),
            ("WaterAmount", self.water_amount), ("Tiberium", self.tiberium),
            ("TiberiumLayout", self.tiberium_layout),
            ("Vegetation", self.vegetation),
            ("UrbanPresence", self.urban_presence),
            ("Resources", self.resources),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let pairs: Vec<(&str, &str)> = values
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        set_ini_values(b"", SECTION, &pairs)
    }
}
```

**Step 2: Add tests**
```rust
// src/map/rmg/options.rs — inside `mod tests`
    #[test]
    fn missing_keys_carry_existing_values() {
        let mut o = RmgOptions { ruggedness: 42, num_players: 6, ..Default::default() };
        let ini = IniFile::from_str("[RandomMap]\nSeed=7\n");
        o.apply_sed(&ini);
        assert_eq!(o.seed, 7, "present key applies");
        assert_eq!(o.ruggedness, 42, "absent key must carry, not zero");
        assert_eq!(o.num_players, 6, "absent key must carry, not zero");
    }

    #[test]
    fn malformed_value_leaves_field_untouched() {
        let mut o = RmgOptions { tiberium: 55, ..Default::default() };
        let ini = IniFile::from_str("[RandomMap]\nTiberium=abc\n");
        o.apply_sed(&ini);
        assert_eq!(o.tiberium, 55);
    }

    #[test]
    fn sed_round_trips() {
        let mut original = RmgOptions {
            theater: 2, map_type: 3, resources: 2, ruggedness: 40, time: 1,
            water_amount: 55, num_players: 6, tiberium: 30,
            tiberium_layout: 20, vegetation: 70, urban_presence: 10,
            width: 2, height: 3, accessibility: 60, region_size: 45,
            seed: 4321,
        };
        original.normalize();
        let bytes = original.to_sed_bytes();
        let mut parsed = RmgOptions::default();
        parsed.apply_sed(&IniFile::from_bytes(&bytes).unwrap());
        parsed.normalize();
        assert_eq!(parsed, original);
    }
}
```

**Step 3: Verify**
```bash
cargo test -p vera20k rmg::options -- --nocapture
```
Expected: `test result: ok.` (6 passed)
**API confirmed 2026-07-20:** `IniFile::from_bytes` (:198) / `from_str` (:207),
`IniFile::section` (:292), `IniSection::get` (:61) / `get_i32` (:70). Do not
change `ini_parser.rs`.

**Step 4: Commit**
```bash
git commit -am "map/rmg: .SED read with carry defaults + writer round-trip"
```

---

### Task 8: `RmgSettings` from RMGMD.INI, with native fallbacks

**Why:** Supplies ore bounds, tree budget, and lighting. The absent-file defaults
are parity-critical and were mis-documented until 2026-07-20.

**Files:** Create `src/map/rmg/settings.rs`; Modify `src/map/rmg/mod.rs`

**Step 1: Declare the module**
```rust
// src/map/rmg/mod.rs — add
pub mod settings;
```

**Step 2: Write it**
```rust
// src/map/rmg/settings.rs
//! `RMGMD.INI [General]` settings. The file ships inside `ra2md.mix`; when it is
//! missing the native object keeps its constructor defaults, which are NOT zero.

use crate::assets::asset_manager::AssetManager;
use crate::rules::ini_parser::IniFile;

/// Native outer-constructor defaults, used when RMGMD.INI is absent.
const DEFAULT_MIN_TIBERIUM: i32 = 2500;
const DEFAULT_MAX_TIBERIUM: i32 = 5500;
const DEFAULT_MAX_TREES: i32 = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RmgSettings {
    pub min_tiberium: i32,
    pub max_tiberium: i32,
    pub max_trees: i32,
    /// Per time-of-day (morning, day, dusk, night).
    pub level_light: [i32; 4],
    /// Per map type (archipelago, continent, team continent, inland, mountainous).
    pub vegetation_min: [i32; 5],
    pub vegetation_max: [i32; 5],
}

impl Default for RmgSettings {
    fn default() -> Self {
        Self {
            min_tiberium: DEFAULT_MIN_TIBERIUM,
            max_tiberium: DEFAULT_MAX_TIBERIUM,
            max_trees: DEFAULT_MAX_TREES,
            level_light: [0; 4],
            vegetation_min: [0; 5],
            vegetation_max: [0; 5],
        }
    }
}

impl RmgSettings {
    /// Load from the asset manager, falling back to native defaults per key.
    pub fn load(assets: &AssetManager) -> Self {
        let mut settings = Self::default();
        let Some(bytes) = assets
            .get_ref("rmgmd.ini")
            .or_else(|| assets.get_ref("rmg.ini"))
        else {
            return settings;
        };
        let Ok(ini) = IniFile::from_bytes(bytes) else {
            return settings;
        };
        settings.apply(&ini);
        settings
    }

    fn apply(&mut self, ini: &IniFile) {
        if let Some(v) = int_key(ini, "RMGMinimumTiberium") {
            self.min_tiberium = v;
        }
        if let Some(v) = int_key(ini, "RMGMaximumTiberium") {
            self.max_tiberium = v;
        }
        if let Some(v) = int_key(ini, "MaxTrees") {
            self.max_trees = v;
        }
        if let Some(v) = int_list::<4>(ini, "RMGLevelLightSettings") {
            self.level_light = v;
        }
        if let Some(v) = int_list::<5>(ini, "RMGVegetationMinimums") {
            self.vegetation_min = v;
        }
        if let Some(v) = int_list::<5>(ini, "RMGVegetationMaximums") {
            self.vegetation_max = v;
        }
    }
}

fn int_key(ini: &IniFile, key: &str) -> Option<i32> {
    ini.section("General")?.get_i32(key)
}

fn int_list<const N: usize>(ini: &IniFile, key: &str) -> Option<[i32; N]> {
    let raw = ini.section("General")?.get(key)?;
    let parsed: Vec<i32> = raw
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    (parsed.len() >= N).then(|| std::array::from_fn(|i| parsed[i]))
}
```

**Step 3: Add tests**
```rust
// src/map/rmg/settings.rs — append
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_file_uses_native_constructor_defaults() {
        let s = RmgSettings::default();
        assert_eq!(s.min_tiberium, 2500);
        assert_eq!(s.max_tiberium, 5500);
        assert_eq!(s.max_trees, 500, "absent RMGMD.INI must NOT mean zero trees");
    }

    #[test]
    fn parses_retail_values() {
        // Verbatim from RMGMD.INI extracted from ra2md.mix (2026-07-20).
        let ini = IniFile::from_str(
            "[General]\n\
              RMGMinimumTiberium=900\n\
              RMGMaximumTiberium=1050\n\
              RMGLevelLightSettings=3,3,3,3\n\
              RMGVegetationMinimums=60,60,60,60,60\n\
              RMGVegetationMaximums=100,100,100,100,100\n\
              MaxTrees=600\n",
        );
        let mut s = RmgSettings::default();
        s.apply(&ini);
        assert_eq!((s.min_tiberium, s.max_tiberium), (900, 1050));
        assert_eq!(s.max_trees, 600);
        assert_eq!(s.level_light, [3, 3, 3, 3]);
        assert_eq!(s.vegetation_min, [60; 5]);
        assert_eq!(s.vegetation_max, [100; 5]);
    }

    #[test]
    fn partial_file_keeps_defaults_for_missing_keys() {
        let ini = IniFile::from_str("[General]\nMaxTrees=42\n");
        let mut s = RmgSettings::default();
        s.apply(&ini);
        assert_eq!(s.max_trees, 42);
        assert_eq!(s.min_tiberium, 2500, "unlisted key keeps native default");
    }
}
```

**Step 4: Verify**
```bash
cargo test -p vera20k rmg::settings -- --nocapture
```
Expected: `test result: ok.` (3 passed)
**If `AssetManager::get_ref` differs,** check `grep -n "pub fn get_ref" src/assets/asset_manager.rs`
and adapt; do not change the asset manager.

**Step 5: Commit**
```bash
git add src/map/rmg/settings.rs && git commit -m "map/rmg: RMGMD.INI settings with native absent-file defaults"
```

---

### Task 9: Fix random-map sentinel player capacity (2..8)

**Why:** Confirmed drift — native clamps `NumPlayers` to 2..8, Rust's sentinel
caps at 4, which would block 5–8 player random maps. Small, isolated, and
independently testable, so it lands before the generator consumes it.

**Files:** Modify `src/skirmish_scenarios.rs:15-17`

**Blast radius (verified 2026-07-20):** neither constant has any consumer outside
`src/skirmish_scenarios.rs` — a grep across `src/` returns nothing. Only the
record's `max_players` field flows outward, so this change is narrower than the
first draft of this plan claimed.

**Step 1: Read the current constants**
```bash
sed -n '10,30p' src/skirmish_scenarios.rs
```

**Step 2: Update the capacity constants**
```rust
// src/skirmish_scenarios.rs — replace the existing constants
/// Native clamps `[RandomMap] NumPlayers` to this inclusive range.
/// Keep these `u8`: `SkirmishScenarioRecord::{min,max}_players` are `Option<u8>`.
pub const RANDOM_MAP_MIN_PLAYERS: u8 = 2;
pub const RANDOM_MAP_MAX_PLAYERS: u8 = 8;
/// Default generated-start quota before a `.SED` supplies `NumPlayers`.
/// `RANDOM_MAP_MAX_PLAYERS` previously aliased this constant, which is why the
/// sentinel capped at 4; the alias is deliberately broken here.
pub const RANDOM_MAP_GENERATED_START_QUOTA: u8 = 4;
```
Leave `RANDMAP_SED` and every other item untouched.

**Step 3: Add a test**
```rust
// src/skirmish_scenarios.rs — inside the existing `mod tests`
    #[test]
    fn random_map_sentinel_allows_native_player_range() {
        let record = SkirmishScenarioRecord::random_map_sentinel(0, "Random Map");
        assert_eq!(record.min_players, Some(RANDOM_MAP_MIN_PLAYERS));
        assert_eq!(record.max_players, Some(RANDOM_MAP_MAX_PLAYERS),
            "native clamps NumPlayers to 2..8; capping at 4 blocks 5-8p maps");
    }
```

**Step 4: Verify (including regression on consumers)**
```bash
cargo test -p vera20k skirmish_scenarios -- --nocapture
cargo test -p vera20k skirmish -- --nocapture
```
Expected: both `test result: ok.` If a consumer asserted max 4, update that
assertion only after confirming it was mirroring the old cap, not a native fact.

**Step 5: Commit**
```bash
git commit -am "skirmish: random-map sentinel honours native 2..8 player clamp"
```

---

### Task 10: `RmgScratch` cell array + diamond bounds

**Why:** The shared state every terrain phase reads and writes. Layout is
parity-critical (native stride 0x50 with fixed field offsets).

**Files:** Create `src/map/rmg/scratch.rs`; Modify `src/map/rmg/mod.rs`

**Step 1: Declare the module**
```rust
// src/map/rmg/mod.rs — add
pub mod scratch;
```

**Step 2: Write it**
```rust
// src/map/rmg/scratch.rs
//! Per-cell working state for generation. Native keeps one 0x50-byte record per
//! cell in a linear width×width array; this mirrors the fields the phases use,
//! as a Rust struct rather than raw offsets.

/// One scratch cell. Field names map to the native record's documented offsets:
/// coord +0x00, height +0x08, velocity +0x10, p_rough +0x18, p_green +0x20,
/// p_sand +0x28, region +0x38, stamp +0x3C, water_lock +0x45, visited +0x47.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScratchCell {
    pub x: i16,
    pub y: i16,
    pub height: f64,
    pub velocity: f64,
    pub p_rough: f64,
    pub p_green: f64,
    pub p_sand: f64,
    pub region: i32,
    pub stamp: i32,
    pub water_lock: bool,
    pub visited: bool,
}

impl Default for ScratchCell {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            height: 0.0,
            velocity: 0.0,
            p_rough: 0.0,
            p_green: 0.0,
            p_sand: 0.0,
            region: -1,
            stamp: -1,
            water_lock: false,
            visited: false,
        }
    }
}

/// The generator's working grid plus the isometric diamond that bounds it.
#[derive(Debug, Clone)]
pub struct RmgScratch {
    width: usize,
    cells: Vec<ScratchCell>,
    diamond_min: i32,
    diamond_max: i32,
}

impl RmgScratch {
    pub fn new(width: usize, diamond_min: i32, diamond_max: i32) -> Self {
        Self {
            width,
            cells: vec![ScratchCell::default(); width * width],
            diamond_min,
            diamond_max,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    /// Linear index used by every native phase: `y * width + x`.
    pub fn index(&self, x: i32, y: i32) -> usize {
        y as usize * self.width + x as usize
    }

    pub fn get(&self, x: i32, y: i32) -> &ScratchCell {
        &self.cells[self.index(x, y)]
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> &mut ScratchCell {
        let i = self.index(x, y);
        &mut self.cells[i]
    }

    pub fn cells(&self) -> &[ScratchCell] {
        &self.cells
    }

    pub fn cells_mut(&mut self) -> &mut [ScratchCell] {
        &mut self.cells
    }

    /// Native in-playfield test: the four-way isometric diamond comparison.
    /// Note the asymmetry — three strict `<` and one inclusive `<=`.
    pub fn in_diamond(&self, x: i32, y: i32) -> bool {
        self.diamond_min < x + y
            && x - y < self.diamond_min
            && y - x < self.diamond_min
            && x + y <= self.diamond_max
    }

    /// Clear region/stamp for a fresh pass, as the native phases do between
    /// region partition and the later placement stages.
    pub fn reset_region_ids(&mut self) {
        for cell in &mut self.cells {
            cell.region = -1;
            cell.stamp = -1;
        }
    }
}
```

**Step 3: Add tests**
```rust
// src/map/rmg/scratch.rs — append
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_native_initial_state() {
        let c = ScratchCell::default();
        assert_eq!(c.region, -1, "region id starts unassigned");
        assert_eq!(c.stamp, -1);
        assert!(!c.water_lock);
        assert_eq!(c.height, 0.0);
    }

    #[test]
    fn diamond_bounds_are_asymmetric() {
        let s = RmgScratch::new(8, 4, 12);
        // x+y must EXCEED min, but may EQUAL max.
        assert!(!s.in_diamond(2, 2), "x+y == min is outside");
        assert!(s.in_diamond(3, 2), "x+y > min is inside");
        assert!(s.in_diamond(6, 6), "x+y == max is inside (inclusive)");
        assert!(!s.in_diamond(7, 6), "x+y > max is outside");
    }

    #[test]
    fn reset_clears_region_and_stamp_only() {
        let mut s = RmgScratch::new(4, 0, 100);
        s.get_mut(1, 1).region = 5;
        s.get_mut(1, 1).stamp = 9;
        s.get_mut(1, 1).water_lock = true;
        s.reset_region_ids();
        assert_eq!(s.get(1, 1).region, -1);
        assert_eq!(s.get(1, 1).stamp, -1);
        assert!(s.get(1, 1).water_lock, "reset must not clear the water lock");
    }
}
```

**Step 4: Verify**
```bash
cargo test -p vera20k rmg::scratch -- --nocapture
```
Expected: `test result: ok.` (3 passed)

**Step 5: Commit**
```bash
git add src/map/rmg/scratch.rs && git commit -m "map/rmg: scratch cell grid + native diamond bounds"
```

---

### Task 11: `generate()` skeleton with the native stage order

**Why:** Locks the phase order — the spec every later task slots into — before
any phase exists. Order determines what each phase sees.

**Files:** Modify `src/map/rmg/mod.rs`

**Step 1: Write the skeleton**
```rust
// src/map/rmg/mod.rs — append below the `pub mod` lines
use anyhow::Result;

use crate::map::map_file::MapFile;
use crate::map::theater::TheaterData;

pub use options::RmgOptions;
pub use rng::RmgRng;
pub use settings::RmgSettings;
pub use scratch::RmgScratch;

/// Ordered list of generation stages. This is the native pipeline order and is
/// the contract later phases attach to; changing it changes generated output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Water,
    WaterFinalize,
    Regions,
    Mode34Passes,
    GreenSpread,
    RecalcA,
    Starts,
    TechBuildings,
    Tiberium,
    RegionReset,
    RecalcB,
    Hills,
    LatPatches,
    RecalcC,
    Emit,
}

/// Native stage order, including the green-spread stage that earlier research
/// omitted and the three interleaved cell-attribute recalculations.
pub const STAGE_ORDER: &[Stage] = &[
    Stage::Water,
    Stage::WaterFinalize,
    Stage::Regions,
    Stage::Mode34Passes,
    Stage::GreenSpread,
    Stage::RecalcA,
    Stage::Starts,
    Stage::TechBuildings,
    Stage::Tiberium,
    Stage::RegionReset,
    Stage::RecalcB,
    Stage::Hills,
    Stage::LatPatches,
    Stage::RecalcC,
    Stage::Emit,
];

/// A generated map plus the start slots the launch path needs.
#[derive(Debug, Clone)]
pub struct GeneratedMap {
    pub map_file: MapFile,
    pub start_waypoints: Vec<(u8, u16, u16)>,
    /// Stages actually executed, in order — asserted by tests.
    pub stages_run: Vec<Stage>,
}

/// Run the generator. Phase bodies land in Plan 2; this walks the native order
/// and records it so the ordering contract is testable today.
pub fn generate(
    options: &RmgOptions,
    settings: &RmgSettings,
    // Optional at this stage: no phase consumes theater data until Plan 2.
    // `TheaterData` has no `Default`, so tests pass `None`.
    _theater: Option<&TheaterData>,
) -> Result<GeneratedMap> {
    let mut normalized = options.clone();
    normalized.normalize();
    let mut rng = RmgRng::new(normalized.seed_u16());
    let _ = (&settings, &mut rng);

    let mut stages_run = Vec::with_capacity(STAGE_ORDER.len());
    for stage in STAGE_ORDER {
        // Mode-3/4 passes only run for island/lake map types.
        if *stage == Stage::Mode34Passes
            && !matches!(normalized.map_type, 3 | 4)
        {
            continue;
        }
        stages_run.push(*stage);
    }

    Ok(GeneratedMap {
        // Interior dimensions are computed by the map-prep stage (Plan 2);
        // 60x60 stands in until that lands so the seam is exercised today.
        map_file: emit::empty_map_file(&normalized, 60, 60),
        start_waypoints: Vec::new(),
        stages_run,
    })
}
```

**Step 2: Add ordering tests**
```rust
// src/map/rmg/mod.rs — append
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_spread_runs_after_regions_and_before_first_recalc() {
        let pos = |s: Stage| STAGE_ORDER.iter().position(|x| *x == s).unwrap();
        assert!(pos(Stage::Regions) < pos(Stage::GreenSpread));
        assert!(pos(Stage::GreenSpread) < pos(Stage::RecalcA));
    }

    #[test]
    fn hills_run_after_tiberium_and_before_lat_patches() {
        let pos = |s: Stage| STAGE_ORDER.iter().position(|x| *x == s).unwrap();
        assert!(pos(Stage::Tiberium) < pos(Stage::Hills));
        assert!(pos(Stage::Hills) < pos(Stage::LatPatches));
    }

    #[test]
    fn mode34_passes_skipped_for_normal_map_types() {
        let settings = RmgSettings::default();
        for map_type in [0, 1, 2] {
            let opts = RmgOptions { map_type, ..Default::default() };
            let out = generate(&opts, &settings, None).unwrap();
            assert!(!out.stages_run.contains(&Stage::Mode34Passes),
                "map type {map_type} must not run island passes");
        }
        for map_type in [3, 4] {
            let opts = RmgOptions { map_type, ..Default::default() };
            let out = generate(&opts, &settings, None).unwrap();
            assert!(out.stages_run.contains(&Stage::Mode34Passes),
                "map type {map_type} must run island passes");
        }
    }
}
```

**Step 3: Verify**
```bash
cargo test -p vera20k rmg::tests -- --nocapture
```
Expected: `test result: ok.` (3 passed)
**`TheaterData` has NO `Default`** (verified 2026-07-20). The tests above must
not call `TheaterData::default()`. Because `generate()` ignores the theater at
this stage, change its parameter to `Option<&TheaterData>` and pass `None` in
tests; Plan 2 tightens it to a required reference once phases consume it. Do not
add a `Default` impl to `theater.rs`.

**Step 4: Commit**
```bash
git commit -am "map/rmg: generator skeleton locking the native stage order"
```

---

### Task 12: `emit.rs` — empty `MapFile` construction + sim-boundary guard

**Why:** Gives `generate()` a real return value and proves the emit seam before
terrain exists. The guard test protects the architecture invariant.

**Files:** Create `src/map/rmg/emit.rs`; Modify `src/map/rmg/mod.rs`

**Step 1: Declare the module**
```rust
// src/map/rmg/mod.rs — add to the module list
pub mod emit;
```

**Step 2: Write it**
```rust
// src/map/rmg/emit.rs
//! Converts generator state into the engine's in-memory `MapFile`.
//! Terrain/overlay/waypoint population lands with the phases (Plan 2); this
//! establishes the header/section shape the rest of the loader expects.

use std::collections::HashMap;

use crate::map::map_file::{MapFile, MapHeader};

use super::options::RmgOptions;

/// Native width/height scale: the `.SED` Width/Height option times 1/3, capped
/// at 1.2 for every map type except the island types 3 and 4.
const DIMENSION_SCALE: f32 = 0.333_333_34;
const DIMENSION_SCALE_CAP: f32 = 1.2;
/// Native writes `Size=0,0,genW+SIZE_PAD_X,genH+SIZE_PAD_Y` and
/// `LocalSize=LOCAL_LEFT,LOCAL_TOP,genW,genH`.
const SIZE_PAD_X: u32 = 4;
const SIZE_PAD_Y: u32 = 12;
const LOCAL_LEFT: u32 = 2;
const LOCAL_TOP: u32 = 5;

/// Theater index (`.SED` Theater) to the engine's theater name. Native indexes a
/// 0x70-stride string table starting at "TEMPERATE".
pub fn theater_name(theater: i32) -> &'static str {
    match theater {
        1 => "SNOW",
        2 => "URBAN",
        3 => "DESERT",
        4 => "NEWURBAN",
        _ => "TEMPERATE",
    }
}

/// Scale factor the native map-prep applies to the width/height options.
pub fn dimension_scale(option: i32, map_type: i32) -> f32 {
    let scale = option as f32 * DIMENSION_SCALE;
    if !matches!(map_type, 3 | 4) && scale >= DIMENSION_SCALE_CAP {
        DIMENSION_SCALE_CAP
    } else {
        scale
    }
}

/// A `MapFile` with header and empty sections, ready for phases to fill.
/// `gen_w`/`gen_h` are the generated interior dimensions the pipeline computed.
pub fn empty_map_file(options: &RmgOptions, gen_w: u32, gen_h: u32) -> MapFile {
    let header = MapHeader {
        theater: theater_name(options.theater).to_string(),
        width: gen_w + SIZE_PAD_X,
        height: gen_h + SIZE_PAD_Y,
        local_left: LOCAL_LEFT,
        local_top: LOCAL_TOP,
        local_width: gen_w,
        local_height: gen_h,
    };
    // `MapFile` has no `Default`; every field is listed explicitly.
    MapFile {
        header,
        basic: Default::default(),
        briefing: Default::default(),
        preview: Default::default(),
        cells: Vec::new(),
        entities: Vec::new(),
        overlays: Vec::new(),
        overlay_data: Default::default(),
        smudges: Vec::new(),
        terrain_objects: Vec::new(),
        waypoints: HashMap::new(),
        ..todo_remaining_fields()
    }
}
```
> **Executor note:** `MapFile` has no `Default` impl. Run
> `grep -n "pub struct MapFile" -A 40 src/map/map_file.rs` and replace the
> `..todo_remaining_fields()` line with the remaining fields written out
> (`cell_tags`, `tags`, `triggers`, `events`, `actions`, `local_variables`,
> `trigger_graph`, `special_flags`, `explicit_tubes`, `ini`, plus any added
> since). Do not add a `Default` impl to `map_file.rs` — other code depends on
> its constructors.
```rust
```

**Step 3: Add tests including the boundary guard**
```rust
// src/map/rmg/emit.rs — append
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theater_indices_map_to_engine_names() {
        assert_eq!(theater_name(0), "TEMPERATE");
        assert_eq!(theater_name(1), "SNOW");
        assert_eq!(theater_name(2), "URBAN");
        assert_eq!(theater_name(3), "DESERT");
        assert_eq!(theater_name(4), "NEWURBAN");
        assert_eq!(theater_name(99), "TEMPERATE", "unclamped theater falls back");
    }

    #[test]
    fn header_pads_size_and_offsets_local_area() {
        let opts = RmgOptions::default();
        let map = empty_map_file(&opts, 60, 60);
        // Native: Size=0,0,genW+4,genH+12 and LocalSize=2,5,genW,genH.
        assert_eq!(map.header.width, 64);
        assert_eq!(map.header.height, 72);
        assert_eq!(map.header.local_left, 2);
        assert_eq!(map.header.local_top, 5, "native local_top is 5, not 4");
        assert_eq!((map.header.local_width, map.header.local_height), (60, 60));
    }

    #[test]
    fn dimension_scale_caps_only_for_non_island_map_types() {
        // option 3 * 1/3 = 1.0 -> under the cap either way
        assert!((dimension_scale(3, 1) - 1.0).abs() < 1e-6);
        // island types 3/4 are exempt from the 1.2 cap
        assert!(dimension_scale(9, 3) > 1.2, "map type 3 is uncapped");
        assert!(dimension_scale(9, 4) > 1.2, "map type 4 is uncapped");
        assert!((dimension_scale(9, 1) - 1.2).abs() < 1e-6, "other types cap at 1.2");
    }

    /// Architecture invariant: sim/ must never depend on the generator.
    #[test]
    fn sim_does_not_reference_the_generator() {
        let mut offenders = Vec::new();
        for entry in walkdir(std::path::Path::new("src/sim")) {
            let text = std::fs::read_to_string(&entry).unwrap_or_default();
            if text.contains("map::rmg") || text.contains("crate::map::rmg") {
                offenders.push(entry.display().to_string());
            }
        }
        assert!(offenders.is_empty(), "sim/ must not depend on rmg: {offenders:?}");
    }

    fn walkdir(root: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(root) else { return out };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walkdir(&path));
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
        out
    }
}
```

**Step 4: Verify**
```bash
cargo test -p vera20k rmg::emit -- --nocapture
```
Expected: `test result: ok.` (3 passed)
**Confirmed 2026-07-20:** `MapFile` has no `Default` (`#[derive(Debug)]` only,
`map_file.rs:155`) — see the executor note above. `MapHeader.width`/`height` are
`u32` (`:109-111`), which is why the dimension helpers use `u32`. The header
geometry (`Size` padding, `LocalSize` origin 2,5) is read from
`decompile_function 0x00599650`, not inferred.

**Step 5: Commit**
```bash
git add src/map/rmg/emit.rs && git commit -m "map/rmg: MapFile emit seam + sim-boundary guard test"
```

---

### Task 13: Launch branch — route `.SED` to the generator

**Why:** Wires the generator into the real load path. Last foundation task
because it depends on every prior piece existing.

**Files:** Modify `src/app_init.rs` (`MapLoadInitial` at :143,
`load_map_initial_with_assets` at :322)

**Step 1: Add the constructor**
```rust
// src/app_init.rs — directly below the MapLoadInitial struct at :143
impl MapLoadInitial {
    /// Build from parts. Used by the random-map path, which produces its
    /// `MapFile` in memory instead of parsing one from disk.
    pub(crate) fn from_parts(asset_manager: AssetManager, map_data: MapFile) -> Self {
        Self { asset_manager, map_data }
    }
}
```

**Step 2: Add the branch**
```rust
// src/app_init.rs — inside load_map_initial_with_assets, before the normal
// load_map_by_name_or_path_with_assets call
// `asset_manager` is owned here. Every borrow — including the mutable one the
// theater loader needs — must END before it is moved into MapLoadInitial.
if let Some(name) = requested_map {
    if name.to_ascii_lowercase().ends_with(".sed") {
        let mut asset_manager = asset_manager;
        let sed_path = ra2_dir.join(name);
        let mut options = crate::map::rmg::RmgOptions::default();
        if let Ok(bytes) = std::fs::read(&sed_path) {
            if let Ok(ini) = crate::rules::ini_parser::IniFile::from_bytes(&bytes) {
                options.apply_sed(&ini);
            }
        }
        options.normalize();

        let settings = crate::map::rmg::RmgSettings::load(&asset_manager);
        let theater_name = crate::map::rmg::emit::theater_name(options.theater);
        // load_theater takes `&mut AssetManager` and returns `Option`, not `Result`.
        let theater = crate::map::theater::load_theater(&mut asset_manager, theater_name)
            .with_context(|| format!("random map: theater {theater_name} unavailable"))?;
        let generated = crate::map::rmg::generate(&options, &settings, Some(&theater))?;
        return Ok(MapLoadInitial::from_parts(asset_manager, generated.map_file));
    }
}
```

**Step 3: Add a test**
```rust
// src/app_init.rs — inside the existing `mod tests` (or add one)
    #[test]
    fn sed_filenames_are_detected_case_insensitively() {
        for name in ["RandMap.Sed", "randmap.sed", "RANDMAP.SED"] {
            assert!(name.to_ascii_lowercase().ends_with(".sed"), "{name}");
        }
        assert!(!"bigmap.map".to_ascii_lowercase().ends_with(".sed"));
    }
```

**Step 4: Verify**
```bash
cargo check -p vera20k
cargo test -p vera20k app_init -- --nocapture
```
Expected: `cargo check` clean; `test result: ok.`
**Parallel-session note:** if `cargo check` reports errors in files you did not
touch, that is another session's in-progress work — do not fix or revert it.

**Step 5: Commit**
```bash
git commit -am "app_init: route .SED map selections to the random map generator"
```

---

### Task 14: Determinism + end-to-end smoke harness

**Why:** Proves the foundation holds the property everything else rests on:
identical options ⇒ identical output, and no cross-run state leakage.

**Files:** Create `src/map/rmg/tests/vectors.rs`; Modify `src/map/rmg/mod.rs`

**Step 1: Declare the test module**
```rust
// src/map/rmg/mod.rs — append
#[cfg(test)]
mod vectors_tests;
```
(Name the file `src/map/rmg/vectors_tests.rs` to match Rust module resolution.)

**Step 2: Write the tests**
```rust
// src/map/rmg/vectors_tests.rs
//! End-to-end foundation checks: determinism and RNG stream independence.

use super::*;

#[test]
fn same_options_generate_identical_output() {
    let opts = RmgOptions { seed: 4321, num_players: 6, ..Default::default() };
    let settings = RmgSettings::default();
    let a = generate(&opts, &settings, None).unwrap();
    let b = generate(&opts, &settings, None).unwrap();
    assert_eq!(a.map_file.header.width, b.map_file.header.width);
    assert_eq!(a.map_file.header.theater, b.map_file.header.theater);
    assert_eq!(a.stages_run, b.stages_run);
    assert_eq!(a.start_waypoints, b.start_waypoints);
}

#[test]
fn different_seeds_diverge_in_the_rng_stream() {
    let mut a = RmgRng::new(1234);
    let mut b = RmgRng::new(1235);
    let stream_a: Vec<u32> = (0..8).map(|_| a.next_u32()).collect();
    let stream_b: Vec<u32> = (0..8).map(|_| b.next_u32()).collect();
    assert_ne!(stream_a, stream_b);
}

#[test]
fn rng_is_independent_of_the_match_seed() {
    // The generator must never consult sim::rng; two generators built from the
    // same .SED seed agree regardless of any other RNG activity.
    let mut first = RmgRng::new(77);
    let mut sim_noise = crate::sim::rng::SimRng::new(999);
    for _ in 0..10 {
        let _ = sim_noise.next_u32();
    }
    let mut second = RmgRng::new(77);
    assert_eq!(first.next_u32(), second.next_u32());
}
```

**Step 3: Verify**
```bash
cargo test -p vera20k rmg -- --nocapture
```
Expected: all rmg tests `ok.` — this is the full module suite (rng, x87, options,
settings, scratch, emit, ordering, determinism).
**If `SimRng::new`/`next_u32` differ,** check `grep -n "pub fn" src/sim/rng.rs`
and adapt the test only.

**Step 4: Run the wider regression**
```bash
cargo test -p vera20k 2>&1 | tail -20
```
Expected: no new failures versus the pre-task baseline. Record the literal
`test result:` line.

**Step 5: Commit**
```bash
git add src/map/rmg/vectors_tests.rs && git commit -m "map/rmg: determinism + RNG-independence harness"
```

---

### Task 15: Verification pass against gamemd — DONE 2026-07-20

All five items pass; results recorded in `docs/research/AUDIT_LOG.md`
(2026-07-20 "RMG plan-1 verification" line). Clamps re-verified via
`decompile_function 0x005975E0`; RMGMD.INI re-extracted from `ra2md.mix`
(`extract-ini` now includes `rmg.ini`/`rmgmd.ini`). Residual: FYL2X
hardware capture (Task 4 step 3) stays open — vectors are emulator-derived.
Plan-2 note: RMGMD.INI also carries OrePatchLamps/Ambient* keys that
`RmgSettings` doesn't parse yet (tiberium-stage lamps + emitted `[Lighting]`).

**Why:** Confirms the foundation matches the original before Plan 2 builds
terrain on top of it.

**Verify:**
1. **RNG stream** — `cargo test -p vera20k rmg::rng` passes against
   `tools/rmg_oracle/vectors/rng.json`. Golden source must read
   `"source": "unicorn/gamemd.exe"`, never a hand-computed table.
2. **Gaussian** — `cargo test -p vera20k rmg::x87` passes against `x87.json`.
   If `ln` needed a faithful FYL2X reproduction, note that in the plan-2 handoff.
3. **Clamps** — re-check `0x005975E0` once Ghidra is back (`/ghidra-up`):
   confirm Resources 0..3, MapType 0..4, Time 0..3, NumPlayers 2..8,
   Tiberium 1..100, Width/Height 0..3, Seed 0..0xFFFF, percents 0..100, and that
   **no instruction writes `+0x38`** (theater unclamped).
4. **RMGMD.INI** — confirm the extracted values still match the file inside
   `ra2md.mix` (`MaxTrees=600`, tiberium 900/1050).
5. **Header geometry — DONE 2026-07-20** (`decompile_function 0x00599650`):
   `Size=0,0,genW+4,genH+12`, `LocalSize=2,5,genW,genH`, scale =
   `WidthOption*0.33333334` capped at 1.2 for map types other than 3/4, and
   cells initialise to level 4. The previously-inferred bucket table was wrong
   and has been removed. Remaining for Plan 2: the player-count term that
   produces `genW`/`genH` from that scale (the `ftol` inputs at `0x00599650`
   are not visible in the decompile — read them from the disassembly).

**Record results** in `docs/research/AUDIT_LOG.md` as a single line:
`- **YYYY-MM-DD** — RMG plan-1 verification — <pass/fail per item>`.

---

## Sources & References

- **Design doc:** [2026-07-19-random-map-generator-design.md](2026-07-19-random-map-generator-design.md)
- **Ghidra reports:** `docs/research/skirmish-ui/RMG_TERRAIN_SHAPING_CORE_GHIDRA_REPORT.md`,
  `RMG_RNG_SEED_MAPGENRNG_GHIDRA_REPORT.md` (PATCHED-to-GREEN 2026-07-20),
  `RMG_TIBERIUM_FIELD_COUNT_AND_GATES_RECHECK_GHIDRA_REPORT.md`,
  `RMG_MODE34_WATER_BRIDGES_TECH_GHIDRA_REPORT.md`,
  `RMG_WATER_SEED_0059A6C0_GHIDRA_REPORT.md`, `RMG_REGION_PARTITION_0058CF90_GHIDRA_REPORT.md`,
  `RMG_START_GENERATION_00594B50_005A1FB0_GHIDRA_REPORT.md`,
  `RMG_START_POINT_SCORING_00594870_GHIDRA_REPORT.md`,
  `SKIRMISH_RANDMAP_SED_WRITER_FULL_LAYOUT_GHIDRA_REPORT.md` (GREEN),
  `SKIRMISH_RANDOM_MAP_SETUP_DIALOG_CONTROLS_OPTIONS_GHIDRA_REPORT.md` (GREEN)
- **gamemd.exe addresses:** `Random__Seed 0x0065C6D0`, `Random__Next 0x0065C780`,
  seed tables `0x00839644` / `0x00839690` (effective fetches `0x839694..0x8396A0`),
  range constant `0x007ED898` (`0x3DF0000000100000`), Gaussian `0x005980C0`,
  normalizer `0x005975E0`, options ctor `0x00595680`, outer ctor `0x00595740`,
  settings loader `0x005981F0`, generator entry `0x00598960`,
  map prep `0x00599650`
- **INI keys:** `RMGMD.INI [General]` `RMGMinimumTiberium`, `RMGMaximumTiberium`,
  `MaxTrees`, `RMGLevelLightSettings`, `RMGVegetationMinimums/Maximums`,
  `TemperateOrePatchLamps`, `SnowOrePatchLamps`, ambient vectors;
  theater `[General]` `ClearTile`/`RampBase`/`RoughTile`/`SandTile`/`GreenTile`/
  `ClearToSandLat`/`ClearToGreenLat`/`WaterSet`; `rulesmd.ini [AI] NeutralTechBuildings`
- **Reference impl:** `docs/research/skirmish-ui/rmg_rng_reference_impl.py`
  (UNVERIFIED until Task 2 Step 3 passes)
- **Related code:** `src/app_init.rs:143,322,383`, `src/map/map_file.rs:140,156`,
  `src/map/theater.rs:234,273`, `src/util/native_x87.rs` (untracked, read-only),
  `src/util/ini_writer.rs:116`, `src/sim/rng.rs`, `src/skirmish_scenarios.rs:14`
