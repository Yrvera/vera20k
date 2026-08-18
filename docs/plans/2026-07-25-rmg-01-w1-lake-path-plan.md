# RMG-01 W1' — the lake path: implementation plan

Item W1' of
`docs/contracts/2026-07-25-rmg-01-water-subslice-implementation-contract.md`.

**Revision 2** (2026-07-25). Revision 1 went through adversarial review, verdict
**FIX FIRST**: five confirmed issues, all on the Rust-side reuse axis. Every
formula revision 1 transcribed from the binary was re-derived from
`0x0059C920` / `0x0059C580` and found correct — the errors were all in what the
plan claimed *already existed in the port*. Revision 1's missing grounding
structure is the reason: it was written directly instead of through
`/write-plan`, so nothing forced a pass over the reuse claims.

Base: `dev = b656a5ab`.

**Read the cited report sections directly — do not work from this plan's
paraphrase where a formula is involved.**

## Grounding Summary

**What this slice does.** Map types 3 (inland) and 4 (mountainous) carve water
*into* land instead of shaping it out of a full sea. Their seeder
(`SeedWaterInlandMountain 0x0059C580`) makes at most one river system and at
most one standalone lake. W1' implements the lake half.

**Where it sits.** `src/map/rmg/phases/water.rs`, inside the early-return arm
added by the W0 follow-up (`d071aa12`). Today that arm returns immediately after
the clear-tile fill, so types 3/4 produce a land-only map. W1' replaces the
return with the lake driver. The five sea-shaping tail passes stay out of this
path — that is settled and must not be reintroduced.

**Why lakes alone are a coherent slice.** The driver gates the river on
`WaterAmount > 0x14` but never gates the lake. For `0 < WaterAmount <= 20` the
original carves no river at all, so lakes-only is the *complete* output for that
band, not a partial one.

**What the slice does not do.** River, meander, bridges, canyon (W2'–W4'). The
pre-folded single-`FMUL` uniform shape belongs to the bridge builder in W3',
not here.

**State of the evidence.** The water sub-slice's decode is closed end to end.
The last symbolic term, the cell-iterator extent, resolved to `M × (2N − 1)`
(`N = 0x0087F8DC`, `M = 0x0087F8E0`).

**What certifies this.** Nothing yet. Every RMG test in the repo is
Rust-versus-prior-Rust. RMG-11 (the generator oracle) is the only instrument
that can certify the goal and is `BLOCKED_EXTERNAL`. Nothing in this slice may
be described as parity-VERIFIED.

## Key Technical Decisions

| # | Decision | Confidence | Source |
|---|---|---|---|
| 1 | `shore::run(ctx, id, keep)` is the port of `0x0057A0C0`; GrowLake's success post-pass calls it `(id, false)` | **high** | Ghidra `decompile_function 0x0057A0C0` + `get_function_callers 0x0057A0C0` (five callers, all RMG water). Label was drifted (`MarkBridgesForRepair_High`), corrected this session |
| 2 | `RmgRng::uniform(min, max)` is the native inclusive uniform, with the correct rejection loop and operand order — do not write another | **high** | `read_memory 0x007ED898` = `0x3DF0000000100000`, identical to `RANGE_K_BITS`; native shape inlined at `0x0059CB13`–`0x0059CB48`, standalone `0x00598030` |
| 3 | `x87::approx_sqrt` is `Sqrt_Approx 0x004CAC40`; the distance term needs it, never `f64::sqrt` | **high** | Reproduced line for line at `x87.rs:256-275`. **Caveat:** only 11 retail table points are pinned by a repo check, not all 16384 |
| 4 | `x87::ftol` truncates toward zero, matching `Math__ftol 0x007C5F00` | **high** | Saturate-vs-wrap divergence needs \|value\| ≥ 2³¹; every GrowLake ftol input is bounded far below |
| 5 | `Gaussian` is Marsaglia polar with the spare cached, and the cache spans the whole generation | **high** | `0x005980C0`; `Generate` initialises `g_RmgGaussianState` once per run via the `REP MOVSD` at `0x005989A6`. Port threads one `Gaussian` from `build.rs:185`. **Its doc comment misnames it "Box-Muller"** — fix in passing |
| 6 | **The min-heap already exists — reuse `blob.rs:63-129`, do not build one** | **high** | Review finding 3. Already `pub(crate)`, already used by `lat_patches`, `starts`, `tiberium`, `trees`. Its silent-drop-when-full, strict-`>` sift-up and left-first strict-`<` sift-down match `0x0059CDF2`/`0x0059D084` and `FloatMinHeap__SiftDown 0x005AD870` |
| 7 | `ScratchCell` needs a **new** `+0x44` field; the driver path and the river-end path set it differently | **high** | Review finding 4. `scratch.rs:15-45` has +0x38/+0x3C/+0x40/+0x45/+0x47/+0x4A/+0x4B and nothing at +0x44 |
| 8 | The three MapSeed accumulators need a named owner in the port | **high** | Review finding 5. No MapSeed analogue exists; `GridCell::default().level` hardcodes 4 |
| 9 | Lakes-only is complete-correct for `0 < WaterAmount <= 20` | **medium** | Driver structure verified. **But** `Generate` does not call the WaterAmount randomiser, so a loaded `.SED` or user-set option can hold any value — the ~20% figure describes freshly randomised seeds only |

## Why this slice is worth landing on its own

The driver is exact (`RMG_MODE34_WATER_BRIDGES_TECH…` §3, re-verified in review):

```
+0x308 += 1                                  // region id 1 = first water region
if (map_type == 3 || 4) && WaterAmount > 0x14:
    for attempt in 0..9:                     // stop on first success
        if CarveRiver(&{0,0}, 0.0, 0) != 0 { +0x308 += 1; break }
for attempt in 0..9:                         // always; stop on first success
    if GrowLake(&{0,0}) != 0 { +0x308 += 1; break }
```

`CMP dword ptr [ESI+0x4C],0x14; JLE` is **signed**, strictly greater than 20.
Both loops are `CMP EDI,0xA; JGE` with stop-on-first-success.

Note the outer gate in `Generate` itself: `SeedWaterInlandMountain` is called
only when `WaterAmount != 0`. A zero-water map of type 3/4 enters no seeder at
all — which is what the port does today.

## What already exists and must be reused

- `shore::run` — decision 1.
- `RmgRng::uniform` — decision 2.
- `Gaussian` — decision 5.
- `x87::approx_sqrt` / `approx_sqrt_f32` — decision 3. The GrowLake call site is
  `FILD int / FSTP double / CALL / FSTP float`, so `approx_sqrt_f32(i32) -> f32`
  is the right shape.
- `x87::ftol` — decision 4.
- **`blob.rs`'s min-heap — decision 6.** Only `cap = max(2·remaining + 2, 100)`
  is new. The silent drop when `count + 1 >= cap` is behaviour, not a safety
  net: dropping it changes lake shapes.

## What must be built new

1. **`ScratchCell` field at `+0x44`** (decision 7). Do **not** reuse
   `water_lock` (+0x45) or `shore_enable` (+0x4A) — other phases read them.
   The driver path sets it only for `id == 0 || id == +0x308`; the river-end
   path sets it for *all* cells. Both callers must be modelled.
2. **The three accumulators** (decision 8): placed-water total (`+0x304`),
   region id (`+0x308`), base level (`+0x30C`, default 4). `+0x304` is what
   makes the WaterAmount quota a **cap** rather than a target — a river that
   consumed the quota makes all ten lake attempts return 0 without drawing.
   W2' depends on that coupling, so give it a real owner now.
3. **Ring dilator** (`FUN_005A0160`, 8 params, ring count is param 2, no RNG).
   Predicate in `…MODE34…` §4.3. *Body not independently re-verified — the
   report re-verified it 2026-07-25 with an inline citation.*
4. **Border-cell collector** (`FUN_005A0700`): scratch scan, coord ≠ (0,0),
   `+0x38 == id`, at least one in-diamond 8-neighbour with a different id.
   Predicate confirmed in review, including the break-on-first-differing-neighbour.
5. **Border peeler** (`FUN_005A0410`, no RNG, always returns 1) — `rings` times,
   border cells get scratch `+0x38 = newId`, real tile `+0x38 = 0`,
   `+0x11A = 0`, `+0x11B = (u8)+0x30C`.

## GrowLake, step by step

Source: `…MODE34…` §4.2, steps 1–10. Every formula below was re-derived from
`0x0059C920` during review and found accurate. Take them from the report.

- **Step 1 quota** — `FILD [EBX+0x184]; FIMUL [EBX+0x180]; FIMUL WaterAmount;
  FMUL 0.008; FADD 100.0` then `ftol`. `remaining = target − (+0x304)`;
  `SUB [EBX+0x304]; CMP 0x4B; JLE` (signed) → `remaining <= 75` returns 0, and
  that is the **water-phase termination condition**, not an error.
- **Step 4 allow-mask** — peel is `RetagBorderBand(0, 2, −2)` (argument values
  byte-verified at `0x0059CA92`). Then `id == 0 → mask 1`, `id == +0x308 →
  mask 1`, and **`scratch tag −2 → 0`** — the *scratch region tag* is reset, not
  the tile. (`0x0059CAAE`–`0x0059CAC3`: `CMP EAX,-0x2` … `MOV dword ptr [ECX],0x0`
  with `ECX = DAT_00abed10 + off + 0x38`.) The peeler already flattened those
  cells' tiles, so writing the tile again is a no-op *and skips the reset that
  matters*: a surviving −2 residue changes the region-0 set that
  `CollectRegionBorderCells(srcTag=0)` sees on lake attempts 2–10, hence a
  different mask, seed and lake. Net effect of the mask: lakes grow on empty
  cells ≥3 cells (8-dir) from foreign water, or on own-region cells.
- **Step 5 seed pick** — `rx = U[0,W−1]`, `ry = U[0,H−1]`; seed is
  `(rx+ry+1, ry+W−rx)`. The cap is `INC EDX; CMP EDX,0xC8; JGE bail` at
  `0x0059CBE3`, sitting **after both draws and before the accept predicate** —
  so 200 draw-pairs but only **199 validated candidates**. `for attempt in
  0..200 { draw; test }` would grow a lake where the original returns 0.
- **Step 6 size** — `mean = remaining/3`, `σ = remaining/6` as **integer** magic
  divides (`0x55555556`, `0x2AAAAAAB`) — round toward zero. Recentre arm is
  `(mean−σ > upper) || (mean+σ < 75.0)`, which with `upper = remaining` reduces
  to `remaining <= 149`. Bounded Gaussian into `[75.0, upper]`.
- **Step 7 growth** — neighbours in dirs `{0,2,4,6}` = N,E,S,W, proven from the
  runtime table initializer at `0x0049F2F0`. (`read_memory 0x0089F688` alone
  returns zeros — the table is not in the static image.) Priority is
  `f32(dist·0.5 + 10.0·rand01 − 0.02·placed)`, constants at
  `0x007ED764/68/6C`, computed as `10.0 × (draw × K)`. **`dist` is the distance
  from the *seed* cell** — the easiest thing here to get wrong. One draw per
  accepted neighbour. A neighbour with a foreign nonzero id sets `alive = 0`
  **softly** — the loop continues.
- **Step 8 drain** — after the size cutoff **every** remaining queue entry is
  popped and must still satisfy (scratch 0, clear, mask) or `alive = 0`.
  `placed` keeps counting, so the final lake is `size + frontier length`.
- **Step 9 gate** — `alive && placed > 75 && placed > size/4` (`CDQ; AND 3;
  SAR 2` — toward zero). Then `shore::run(id, false)` → dilator
  `(id, 1, rect{0,0,0x200,0x200}, 0, 0)` → green pass. The green pass is report
  §4.2 step 9: tile 0/0xFFFF **inside the region** → `[0x00AA0E18]`. It paints
  cells within the region, *not* shore-adjacent cells outside it — it is not the
  sea-shaping seeder's shore→green tail pass, which does not run for these types.
- **Step 10** — success adds `placed` to `+0x304`. Failure rolls back to
  scratch `+0x38 = 0`, `+0x4B = 0`, tile `+0x38 = 0`, `+0x11A = 0`,
  **`+0x11B = (u8)+0x30C`, which defaults to 4 — not 0.**

## Draw accounting

`shore::run` costs `2 × M × (2N − 1)` bounded uniforms — confirmed in review and
**not** a divergence: `select` takes `uniform(0,5)` unconditionally per iterated
cell in both passes, matching `0x0057ACF0` (`MOV EDX,5; XOR ECX,ECX; CALL
0x00598030` at entry, before the mask and every early-out), and the port's
diamond extent matches (`RmgGrid::is_valid`, `mod.rs:130-131`). It is the
dominant consumer in the whole water stage, far exceeding lake growth itself.

Revision 1 flagged a possible pre-existing divergence here. There was one, but
it was a different defect: the port ran the sea-shaping tail — including
`shore::run` — for map types 3/4, which the original never does. **Fixed and
merged separately** (`d071aa12`), with a draw-count regression test. Do not
reintroduce it.

## Tests

1. `0 < WaterAmount <= 20` on map types 3 and 4 produces water, and the water is
   a single connected lake region — the complete-correct band.
2. `WaterAmount == 0` still yields an all-clear map, and still consumes **zero**
   draws (W0's two tests must stay green — the draw-count one is the load-bearing
   one, and it must be updated, not deleted, once the seeder starts drawing for
   nonzero amounts).
3. Failure path: a fixture where the seed pick exhausts its attempts leaves the
   map byte-identical to its pre-call state — proves rollback, including
   `+0x11B` restored to 4 rather than 0, **and** the `−2 → 0` scratch reset, by
   asserting attempt 2 sees the same region-0 set as attempt 1.
4. Determinism across repeat runs, plus an exact draw-cursor assertion for one
   fixed seed, measured against the post-lake cursor.
5. The retail resolvability pass must stay green with types 3/4 now emitting
   water and shore tiles. This is the check that caught W0's bug — a unit test
   with a permissive predicate did not.

Each of tests 1–4 must be falsified against the pre-change code before being
believed, as in W0.

## Residuals this slice does not close

- `x87.rs:293` calls the Gaussian "Box-Muller"; it is Marsaglia polar.
- The sqrt table is pinned at 11 points while `x87.rs` prose claims all 16384
  are reproduced. The exhaustive check is cheap and is the only genuine
  `VERIFIED` reachable in this goal today.
- `shore.rs:79`'s spike-mask list `{0xC7,0x7C,0xF1,0x1F,0xC6,0x6C,0xB1,0x1B}`
  has never been re-derived, and it sits on the lake's success path.
- Whether `+0x30C` is ever anything but 4 — no writer found; the claim rests on
  a plate comment citing `MapSeedClass__InitDefaults`.
- `FUN_005A0160`'s body (call-site arguments confirmed; body not independently
  re-verified this session).
