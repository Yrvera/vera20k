# RMG Region Split `RmgRegion::Split` @ `0x0058D620` — Ghidra Research Report

**Date:** 2026-07-25
**Program:** `gamemd.exe`, image base `00400000` (verified via `get_current_program_info`)
**Status:** VERIFIED from binary this session unless a claim is explicitly marked YELLOW in §11.
**Scope:** the region SPLIT that runs only for MapType 3 (Inland) / 4 (Mountainous); its complete
RNG draw ledger; the `RegionSize` → split-threshold expression; the contract of
`RandomMapGenerator::SplitOversizedRegions` @ `0x0058EBC0` which drives it; the proven
`RmgRegion` struct layout.

**Why this matters:** the Rust port does not generate map types 3 and 4. This function plus
`0x0058EBC0` is one of the two undecoded blockers for that block, and it is the sole consumer of
the `RegionSize` dialog option (which the port ignores entirely).

---

## 1. Identity and call context

| Address | Role | Convention | Evidence |
|---|---|---|---|
| `0x0058D620` | `RmgRegion::Split` — splits one region into 1..N new regions | `__thiscall` (ECX = region), returns `bool` in AL | `disassemble_function 0x0058D620` (`MOV EBX,ECX` @ `0x0058D62D`; `XOR AL,AL` @ `0x0058D63F`; `MOV AL,0x1` @ `0x0058E5C3`) |
| `0x0058EBC0` | `RandomMapGenerator::SplitOversizedRegions` — sole caller | `__cdecl`, no args | `get_function_callers 0x0058D620` → exactly one caller, `0x0058EBC0` |
| `0x0058C6F0` | frontier cost function; sole caller is `0x0058D620` | `__stdcall` (`RET 0x10`), 3 args on stack; ECX is loaded by the caller but **unused** | `disassemble_function 0x0058C6F0`; `get_function_callers 0x0058C6F0` → one caller |
| `0x0058E5D0` | `RmgRegion::CollectBorderCells` | `__thiscall` | `decompile_function 0x0058E5D0` |
| `0x0058BF70` | `RmgRegion` constructor | `__thiscall(this, CellStruct seed)` | `decompile_function 0x0058BF70` |
| `0x0058D0A0` | cell coord → region id (reads linear node `+0x38`) | `__fastcall` | `decompile_function 0x0058D0A0` |
| `0x005AD870` | generic float-keyed min-heap sift-down | `__thiscall(heap, index)` | `decompile_function 0x005AD870`; `get_function_callers 0x005AD870` → 13 callers incl. `TiberiumClass__GrowthProcessor`, so it is NOT RMG-specific |

**Entry predicate (only one):**

```
if (region->Finalised (+0x1A) != 0) return false;    // 0x0058D631 .. 0x0058D648
```

`0x0058EBC0` only calls it when `+0x1A == 0`, so **in practice the function always returns `true`**
and the caller always destroys the region. Verified: `disassemble_function 0x0058EBC0`
(`MOV AL,byte ptr [ESI + 0x1a]` @ `0x0058EDCD`, `JNZ` past the call).

There is **no recursion**. Re-entry is driven entirely by the caller: on a `true` return the caller
frees the split region and **restarts its scan from index 0** (`JMP 0x0058EDB5` @ `0x0058EE8B`).
Newly created sub-regions were appended to the global region list by the constructor
(`0x0058BF70`), so they are themselves re-examined and possibly split again.

**Attempt caps:** none in the classic sense. The bounds are:
* growth loop ≤ `maxSteps` iterations (RNG draw #1, §4), and stops early when the open heap empties;
* frontier node arena is `2*CellCount + 10` entries — a heap push whose `count+1 >= capacity`
  is **silently dropped**, not grown (`CMP ECX,EAX / JNC 0x0058D891` @ `0x0058DA78`).

---

## 2. Split geometry — the actual algorithm

The split is **not** an axis cut. It is a randomized best-first ("cheapest frontier") growth from one
random seed cell, biased along a heading that performs a Gaussian random walk; the grown set and its
complement are then each decomposed into connected components, and every component becomes a new
region.

### 2.1 Setup (`0x0058D649` – `0x0058D739`)

1. `nodes = operator_new((CellCount*2 + 10) * 8)` — arena of `{CellStruct coord; float cost;}`.
2. `heap = operator_new(0x14)` — 1-based binary **min-heap** of `node*`, ordered by `node->cost`:
   `{+0x00 count, +0x04 capacity, +0x08 array, +0x0C max-ptr-seen, +0x10 min-ptr-seen}`.
   Sift-up is inlined at each push; sift-down is `0x005AD870`, called with index 1 after each pop.
   Both comparisons are **strict `<`** on the `float` key, so ties keep the incumbent.
3. Every linear map node's scratch field `+0x3C` is set to `-1` over the whole `W×W` grid
   (`0x0058D6DF`; `W = g_PathfinderLinearMapWidth @ 0x0089C2DC`, stride `0x50`).
4. Every cell in this region's cell list is stamped `linearNode+0x38 = -2` (`0x0058D6FD`), i.e.
   "candidate, belongs to the region being split".

### 2.2 Seed and heading

* `maxSteps` = RNG draw #1 (§4), uniform in `[CellCount/8, CellCount/3]`.
* `seedIdx` = RNG draw #2, uniform in `[0, CellCount-1]`; `seed = region->Cells[seedIdx]`.
* `nodes[0] = {seed, 0.0f}`, pushed; `linearNode(seed)+0x3C = -3` (enqueued marker).
* `theta` = RNG draw #3 = `u32 * (2π / (2^32-1))` — the initial heading, uniform on `[0, 2π]`.

### 2.3 Growth loop (`0x0058D8E9` – `0x0058DB35`)

```
while (current != NULL && steps < maxSteps) {
    linearNode(current)+0x38 = -3;                 // claim this cell for the grown set
    for k in 0..3 {                                 // FOUR neighbours only
        dir = 2*k;                                  // g_DirectionOffsets[0,2,4,6] = N, E, S, W
        nb  = current + g_DirectionOffsets[dir];
        if (!inPlayfieldDiamond(nb)) continue;
        if (linearNode(nb)+0x38 != -2) continue;    // not a candidate of this region
        if (linearNode(nb)+0x3C == -3) continue;    // already enqueued
        if (!CellClass::IsClearTile(nb)) continue;  // 0x00486380
        cost = SplitFrontierCost(seed, nb, theta);  // 0x0058C6F0 — consumes RNG draw #5
        nodes[used] = { nb, (float)cost };
        linearNode(nb)+0x3C = -3;
        heap.push(&nodes[used]); used++;
    }
    steps++;
    theta += NextGaussian() * (pi/8);               // RNG draw #4
    if (heap.empty) break;
    current = heap.pop_min();
}
```

Verified: the neighbour loop runs exactly 4 times — the direction counter at `[ESP+0x10]` advances
by 2 (`ADD EDI,0x2` @ `0x0058DACC`) and the parallel linear-delta cursor walks
`0x0089A304 → 0x0089A31C` in steps of 8 (`CMP EAX,0x89A324 / JL` @ `0x0058DACF`).
The cardinal identity of indices 0/2/4/6 is proven from the table initializer
`Foundation_direction_table_init @ 0x0049F2F0` (`decompile_function 0x0049F2F0`):

| idx | dword @ `0x0089F688 + 4*idx` | (dx, dy) | compass |
|---|---|---|---|
| 0 | `0xFFFF0000` | (0, −1) | **N** |
| 1 | `0xFFFF0001` | (+1, −1) | NE |
| 2 | `0x00000001` | (+1, 0) | **E** |
| 3 | `0x00010001` | (+1, +1) | SE |
| 4 | `0x00010000` | (0, +1) | **S** |
| 5 | `0x0001FFFF` | (−1, +1) | SW |
| 6 | `0x0000FFFF` | (−1, 0) | **W** |
| 7 | `0xFFFFFFFF` | (−1, −1) | NW |

(The table is zero in the static image — `read_memory 0x0089F688` returns zeros; it is written at
startup by `0x0049F2F0`. The parallel linear-index delta table at `0x0089A304` is written by
`PathfinderClass__ResizeMapArrays` — `get_xrefs_to 0x0089A304` → `WRITE from 0x0042ACA1`.)

**Playfield diamond test** (identical in all four loops of this function):
```
x + y >  DAT_00ABED04
x - y <  DAT_00ABED04
y - x <  DAT_00ABED04
x + y <= DAT_00ABED08
```

### 2.4 Frontier cost `0x0058C6F0` — exact expression

```
dx = cell.X - seed.X ;  dy = cell.Y - seed.Y
dist = Sqrt_Approx( (double)(dx*dx + dy*dy) )            // 0x004CAC40, see §2.5
ang  = (dx == 0) ? pi/2                                   // [0x007E2820] = pi/2
                 : atan( -((double)dy / dx) ) + (dx < 0 ? pi : 0)   // atan @ 0x004CADE0, pi @ [0x007E44D0]
d = |ang - theta|
while (d >= 2pi) d -= 2pi                                 // [0x007E3CC0] = 2pi
if (d > pi) d = 2pi - d
U = (double)Random_Next(g_RmgRandom) * (1/(2^32-1))       // [0x007ED898]
cost = ((2.5*U) + (1.5*d)) + (0.15*dist)                  // 2.5 @ [0x007ED6A8], 1.5 @ [0x007ED6B8], 0.15 @ [0x007ED6B0]
```

The x87 association above is the literal instruction order (`FMUL k; FMUL 2.5; FLD 1.5; FMUL d;
FADDP; FLD 0.15; FMUL dist; FADDP` @ `0x0058C7C6`–`0x0058C7EC`). The 80-bit result is stored by the
caller as **f32** (`FSTP float ptr [EBP + 0x4]` @ `0x0058DA3E`) — the heap key is single precision.

Constants read directly: `read_memory 0x007ED6A8` → `0x4004000000000000` = 2.5;
`0x007ED6B0` → `0x3FC3333333333333` = 0.15; `0x007ED6B8` → `0x3FF8000000000000` = 1.5;
`0x007E2820` → `0x3FF921FB54442D18` = π/2; `0x007E44D0` → π; `0x007E3CC0` → 2π;
`0x007ED898` → `0x3DF0000000100000` = **1/(2³²−1)** = 2.3283064370807974e-10 (not 2⁻³²);
`0x007ED8A0` → `0x3FD921FB54442D18` = π/8; `0x007ED8A8` → `0x3E1921FB545D4F13` = 2π/(2³²−1).

### 2.5 `Sqrt_Approx` @ `0x004CAC40` is NOT a real square root

`disassemble_function 0x004CAC40`: it rounds the double argument to f32, then does an
exponent-halve plus a **16384-entry lookup table at `0x008650BC`** indexed by the top 14 bits of the
mantissa (`AND ECX,0x7FFFFF; SHR ECX,0xA; MOV ECX,[ECX*4 + 0x8650BC]`), and returns an f32.
Special cases: `x == 0.0f` returns `0.0f` (`[0x007E1748]` = 0.0f); `x < 0` is first multiplied by
`[0x007E4900]` = −1.0.
The table **is** present in the retail image (`read_memory 0x008650BC` → `00000000 FF010000
FF030000 FF050000 …`), so a port can extract it byte-for-byte. The same helper is used by the
Gaussian (§4.4), so it is on the critical path twice.

### 2.6 Drain, relabel, component split (`0x0058DB3B` – `0x0058DF7F`)

1. All linear nodes' `+0x3C` reset to `-1` (`0x0058DB3B`).
2. **Every cell still sitting in the open heap is also stamped `+0x38 = -3`** (`0x0058DBC8` –
   `0x0058DC1C`). The grown set therefore includes the unexpanded frontier, not just the expanded
   cells.
3. Arena and heap are freed.
4. Walk the original region's cell list **descending** (`i = ActiveCount-1 … 0`, `0x0058DC6F`).
   For each cell whose `linearNode+0x38 < -1` (i.e. still `-2` or `-3`):
   * `new RmgRegion(cell)` (`operator_new(0x50)` + `0x0058BF70`) — this assigns the next region id
     and appends it to the global region list;
   * push the cell, `new->CellCount++`, `linearNode(cell)+0x38 = new->Id`;
   * append `new` to a local "new regions" vector;
   * **flood fill, 8-connected** (`INC EAX; CMP EAX,0x8; JL` @ `0x0058DF3B`), over neighbours whose
     `+0x38` equals *the same marker value the seed cell had* (`-2` or `-3`), claiming them into
     `new`. The worklist is a LIFO stack (pop-last), so cell insertion order is **DFS order**.

So the split yields **one new region per connected component of each of the two marker classes** —
two regions in the common case, more when a class is disconnected. The original region object is
never reused; the caller frees it.

### 2.7 Per-new-region level assignment (`0x0058DF85` – `0x0058E50F`)

For each new region `cur`, iterated **descending**, skipping any with `cur->Merged (+0x24) != 0`:

1. `border = cur->CollectBorderCells()` (`0x0058E5D0`).
2. `adj = operator_new(g_nRmgNextRegionId)` zeroed; for every border cell, for all **8** directions,
   if the neighbour's `+0x38` is `>= 0` and `!= cur->Id`, set `adj[thatId] = 1`.
3. Scan `adj`; for each flagged id, linear-search the global region list for the object with that id
   (`0x0058E0EA` – `0x0058E13E`), and when that region has `+0x24 == 0`:
   * track `best` = the neighbour with the **largest CellCount** whose `Water (+0x14) == 0`
     (strict `>`, first-wins on ties, scanned in ascending region-id order), remembering its Level;
   * track `min` / `max` of neighbour Levels (over **all** live neighbours, water included).
4. Build the candidate-level list from `span = max - min`:

   | `span` | candidates pushed, in this order |
   |---|---|
   | `0`  | `min-4` if `min >= 4`; then `max+4` if `max <= 7` |
   | `4`  | `max`, then `min` |
   | `8`  | `trunc((max+min)/2)` |
   | else | *(empty)* |

   With no live neighbours at all, `min = max = -1`, so only `max+4 = 3` is pushed.
5. **MapType filter** (`CMP EAX,0x3 / CMP EAX,0x4` on `[0x00ABE014]` @ `0x0058E2F9`): for MapType 3
   or 4, every candidate equal to `0` is removed (descending scan with a shift-down compaction).
   Since this function only runs for types 3/4, the filter is effectively unconditional.
6. Then:
   * **`cur->CellCount > 100`** (`CMP dword ptr [EAX + 0xC],0x64 / JLE` @ `0x0058E354`):
     RNG draw #6 picks a candidate index uniformly in `[0, candCount-1]`; `cur->Level = cand[idx]`;
     then iterate **all map cells** (`MapClass::CellIterator_Init/Next` @ `0x00578350` / `0x00578290`)
     and for every cell whose region id equals `cur->Id`, do
     `cell->Level(+0x11B) += (int8)(newLevel - oldLevel)`.
   * **`cur->CellCount <= 100`**: no RNG. If `best != NULL`, `cur->Level = best->Level`, apply the
     same `+0x11B` delta pass, then **move every one of `cur`'s cells into `best`** (push to
     `best`'s vector, `best->CellCount++`, `linearNode+0x38 = best->Id`) and set `cur->Merged = 1`.
     If `best == NULL`, nothing happens at all.

### 2.8 Epilogue (`0x0058E513` – `0x0058E5CB`)

Walk the global region list descending; every region that is not `this` and has `+0x24 != 0`
(merged) is destructed (`0x0058C070`) and freed. Then `this->+0x1C += 1` (a dead write — the caller
frees `this` immediately after). Return `true`.

---

## 3. `RegionSize` → threshold (lives in `0x0058EBC0`, not in `0x0058D620`)

The task brief located this contract in `FUN_0058D620`. **That is wrong** — `0x0058D620` contains no
reference to `0x00ABE048` and no threshold at all. The threshold is computed once in the caller,
at `0x0058ED89`, verbatim (`disassemble_function 0x0058EBC0`):

```
FILD  dword ptr [0x00ABE048]     ; RegionSize option (int32)
FMUL  double ptr [0x007E44E8]    ; * 0.005      (read_memory -> 0x3F747AE147AE147B)
FADD  double ptr [0x007E8AE8]    ; + 0.05       (read_memory -> 0x3FA999999999999A)
FIMUL dword ptr [0x00ABE15C]     ; * mapDimA (int32)
FIMUL dword ptr [0x00ABE158]     ; * mapDimB (int32)
FADD  ST0,ST0                    ; * 2
CALL  0x007C5F00                 ; Math__ftol
```

```
threshold = trunc( 2 * ((((0.005 * RegionSize) + 0.05) * dimA) * dimB) )
```

* **Multiply/add order is as written** — the `*2` is applied last, by `FADD ST0,ST0`, i.e. after both
  dimension multiplies. Re-associating changes the last ulp.
* **Truncation mode: chop toward zero — VERIFIED.** `Math__ftol @ 0x007C5F00` loads the control word
  from `[0x00822D80]` before `FISTP` (`disassemble_function 0x007C5F00`). `read_memory 0x00822D80`
  → `0x0E7F`; RC bits 11:10 = `11` = round-toward-zero. (Ghidra renders `ftol` as *round*; that is
  wrong.) PC bits 9:8 = `10` = 53-bit double precision.
* **Comparison direction: STRICT.** `CMP dword ptr [ESI + 0xC],EBX / JLE 0x0058EDED` @ `0x0058EDDB`
  — signed. A region splits **iff `CellCount > threshold`**; on `<=` it is marked
  `Finalised (+0x1A) = 1` and never revisited.
* `0x00ABE048` has **exactly one** cross-reference in the whole binary — this read
  (`get_xrefs_to 0x00ABE048`). No writer references it directly, so it is written through a struct
  pointer. Its identity as the `RegionSize` dialog option is inherited from a parallel session, not
  proven here — see §11.
* `0x00ABE158` / `0x00ABE15C` are read only here and in `RandomMapGenerator__CreateStartingPoints`
  @ `0x00594B53` / `0x00594B64`, where they are multiplied together the same way
  (`FILD [0x00ABE15C]; FIMUL [0x00ABE158]`) to form an area. That corroborates "generated map
  dimensions"; which one is width vs height is **not** proven (and does not affect the threshold,
  since only their product is used).

**No-op case confirmed by arithmetic:** the scalar ranges from `2*0.05 = 0.1` (RegionSize 0) to
`2*(0.005*100+0.05) = 1.1` (RegionSize 100) times `dimA*dimB`. Regions only exist inside the
playfield diamond, which is roughly half the bounding rectangle, so a high `RegionSize` puts the
threshold above the largest achievable region and the entire split pass degenerates to "mark every
region final" — no splits, no RNG consumed, no terrain change.

---

## 4. Complete RNG draw ledger

**All** draws in this subsystem come from **one** generator instance, `g_RmgRandom @ 0x00ABE890`.
`Random__Next @ 0x0065C780` (`decompile_function`) is a 250-lag Fibonacci XOR generator:
`state[i] ^= state[j]; result = state[i]; i++; j++; wrap both at > 249`, with
`+0x00 disabled-flag (returns 0 if set), +0x04 i, +0x08 j, +0x0C uint32 state[250]`.
The instance is re-initialised at the top of `RandomMapGenerator__Generate` by `REP MOVSD` from
`[0x0082AF14]` (`disassemble_bytes 0x00598990-0x005989E0`).

The uniform scale used everywhere is **`1/(2³²−1)`**, not `2⁻³²`.

| # | Callsite | Kind | Frequency | Expression |
|---|---|---|---|---|
| 1 | `0x0058D787` (loop from `0x0058D782`) | uniform + rejection | once per Split call | step budget |
| 2 | `0x0058D7D2` (loop from `0x0058D7CD`) | uniform + rejection | once per Split call | seed cell index |
| 3 | `0x0058D8C6` | uniform, no rejection | once per Split call | initial heading |
| 4 | `0x0058DAF0` → `NextGaussian @ 0x005980C0` | Gaussian (Marsaglia polar, **with spare cache**) | once per growth-loop iteration | heading walk |
| 5 | `0x0058C7B5` (inside `SplitFrontierCost`) | uniform, no rejection | **once per accepted frontier cell** | cost jitter |
| 6 | `0x0058E382` (loop from `0x0058E37D`) | uniform + rejection | once per new region with `CellCount > 100` | level choice |

Site #5 is by far the highest-volume consumer and is easy to miss: it lives in the callee, not in
`0x0058D620`.

### 4.1 Draw #1 — step budget

```
lo    = CellCount / 8          ; SAR-based signed trunc division  (0x0058D752)
hi    = CellCount / 3          ; 0x55555556 magic-multiply signed trunc division (0x0058D73D)
range = hi - lo + 1
repeat {
    r = Random_Next(g_RmgRandom)
    v = ftol( (((double)r * (double)range) * (1/(2^32-1))) + (double)lo )
} while ((uint32)v > (uint32)hi)      ; JA @ 0x0058D7B4
maxSteps = v
```
FP order verified at `0x0058D794`–`0x0058D7A6`: `FILD r; FMUL range; FMUL k; FADD lo`.
The rejection is a rounding guard only — the exact-arithmetic maximum is `hi`.

### 4.2 Draw #2 — seed cell index

```
n = region->Cells.ActiveCount (+0x38)
repeat {
    r = Random_Next(g_RmgRandom)
    v = ftol( ((double)r * (double)n) * (1/(2^32-1)) )
} while ((uint32)v > (uint32)(n-1))   ; JA @ 0x0058D7F4
seed = region->Cells[v]
```
No additive base here (`0x0058D7DF`–`0x0058D7ED`).
**Parity hazard:** this indexes the region's cell list, whose *order* is set by
`0x0058EBC0`'s rebuild (descending linear index — §5) and, for split-produced regions, by the DFS
component fill of §2.6. Cell-list order is therefore load-bearing, not incidental.

### 4.3 Draw #3 — initial heading
```
theta = (double)Random_Next(g_RmgRandom) * (2*pi/(2^32-1))     ; [0x007ED8A8] @ 0x0058D8D9
```

### 4.4 Draw #4 — the Gaussian, and its **spare cache**

`NextGaussian @ 0x005980C0` operates on the state object `g_RmgGaussianState @ 0x00ABDFB8`:
`{+0x00 bool hasSpare, +0x08 double spare, +0x10 pfn uniform}`.
`disassemble_function 0x005980C0`:

```
if (hasSpare) { hasSpare = false; return spare; }     // ZERO uniform draws
do {
    u1 = 2*U() - 1                                     // [0x007E1718] = 1.0
    u2 = 2*U() - 1
    s  = u1*u1 + u2*u2
} while (!(s < 1.0) || s == 0.0);                      // FCOM 1.0 / FCOM 0.0 @ 0x0059810A, 0x00598117
m  = Sqrt_Approx(-2*ln(s)/s);                          // FLDLN2/FYL2X then 0x004CAC40
spare = u1*m; hasSpare = true;
return u2*m;
```

`U()` is the indirect call at `+0x10`, initialised to `0x00598000`
(`disassemble_bytes 0x00598990-0x005989E0`: `MOV dword ptr [ESP+0x2C],0x598000` then `REP MOVSD`
into `0x00ABDFB8`). `disassemble_bytes 0x00598000` shows it is
`Random_Next(g_RmgRandom) * (1/(2^32-1))` — the **same** stream, scaled to `[0, 1]`.

**Consequences for a bit-exact port:**
* one `NextGaussian` costs `2*N` uniform draws where `N` is geometric with `p = π/4` — **but only on
  calls that miss the cache**; every other call costs **zero** draws.
* `hasSpare` and `spare` are zeroed once per `RandomMapGenerator::Generate` (the same `REP MOVSD`
  above), *not* per split, so the cache parity carries across every region split in a generation and
  across any other RMG consumer of the Gaussian.
* The rejection test is `s < 1.0` **and** `s != 0.0`; both `u1` and `u2` are drawn before the test,
  so a rejected pair costs 2 draws.
* `Sqrt_Approx` (§2.5) is used here too, so the Gaussian's magnitude is the LUT approximation, not
  an IEEE sqrt.

### 4.5 Draw #6 — level choice
```
repeat {
    r = Random_Next(g_RmgRandom)
    v = ftol( ((double)r * (double)candCount) * (1/(2^32-1)) )
} while ((uint32)v > (uint32)(candCount-1))    ; JA @ 0x0058E3A4
level = cand[v]
```
If `candCount == 0` the loop exits immediately with `v = 0` and the code reads `cand[0]` out of an
empty (possibly NULL-backed) vector. Reachable only if `span ∉ {0,4,8}` while `CellCount > 100`; not
observed to be reachable with the level quantisation this code produces, but a port should not
silently "fix" it without checking (see §11).

---

## 5. `RandomMapGenerator::SplitOversizedRegions` @ `0x0058EBC0`

Full contract, verified from `disassemble_function 0x0058EBC0`:

**Phase 1 — rebuild (`0x0058EBC0`–`0x0058ED83`).**
For every region in the global list: call vtable slot `+0xC` on the embedded cell vector at `+0x28`
(clear), set `CellCount (+0xC) = 0`, and reset the bbox to `+0x40 = 9999, +0x44 = 9999,
+0x48 = 0, +0x4C = 0` (`MOV EDI,0x270F` @ `0x0058EBD2`). Note it clears the **cell list** as well as
the count — the list is fully rebuilt, not merely re-counted.

Then walk the linear cell grid **descending** (`SUB EDX,0x50` @ `0x0058ED77`, starting at
`(W*W-1)*0x50`). For each node passing the playfield-diamond test, read `rid = node+0x38`; if
`0 <= rid < regionCount`, then: `region[rid].CellCount++`, **push the coord onto
`region[rid]`'s cell vector**, and extend `region[rid]`'s bbox at `+0x40..+0x4C`
(origin X, origin Y, width, height — grown by the `if (x < origin) {origin = x; w += origin-x;}`
idiom at `0x0058ED21`–`0x0058ED6D`).

The descending walk is why every rebuilt cell list is in **descending linear-index order**; RNG draw
#2 indexes into exactly this list.

**Phase 2 — threshold** (§3), computed once, outside every loop.

**Phase 3 — split loop (`0x0058EDB5`–`0x0058EE8B`).**
```
i = 0
while (i < regionCount) {
    r = regions[i]
    if (r->Finalised(+0x1A) == 0 && r->Water(+0x14) == 0) {
        if (r->CellCount > threshold) {
            if (RmgRegion::Split(r)) {              // always true in practice
                destroy r, remove from list, restart at i = 0
            }
        } else r->Finalised = 1
    }
    i++
}
```
Removal: virtual `Remove` at `[DAT_00ABDF90 + 0x10]` finds the index, then the tail is shifted down
and `DAT_00ABDFA0` (count) decremented (`0x0058EE15`–`0x0058EE5C`); the region's own cell-vector
buffer is freed and then the object.

**Culling predicate:** the only thing culled here is a *successfully split* region — there is no
size-based or quality-based cull in `0x0058EBC0`. The size-based **merge** (regions of ≤ 100 cells
absorbed into their largest non-water neighbour) happens inside `0x0058D620` §2.7, and those merged
regions are freed in `0x0058D620`'s epilogue §2.8.

**RNG: `0x0058EBC0` consumes none, directly.** Verified by enumerating every `CALL` in its
disassembly: vtable `+0xC` (clear) @ `0x0058EBE9`, vtable `+0x8` (grow) @ `0x0058ECE1`,
`Math__ftol` @ `0x0058EDA9`, `RmgRegion::Split` @ `0x0058EDE2`, a virtual destructor @ `0x0058EE11`,
vtable `+0x10` (remove) @ `0x0058EE29`, and `operator delete` @ `0x0058EE70` / `0x0058EE83`. None of
these is an RNG entry point. (Per the standing tooling warning, absence was established by reading
the full instruction listing and by `get_xrefs_to`, **not** by `search_instructions` — that tool
cannot match absolute memory operands.) All RNG in the pass therefore comes from `0x0058D620` and
its callees.

---

## 6. `RmgRegion` struct layout (size `0x50`)

Proven writer-by-writer. The constructor `0x0058BF70` (`decompile_function 0x0058BF70`) establishes
most of it; `operator_new(0x50)` @ `0x0058DCB3` fixes the size.

| Offset | Type | Meaning | Proving writer |
|---|---|---|---|
| `+0x00` | int | 0 at construction; a pointer-like slot the caller destroys via `(**(void**)*r)(1)` before freeing | ctor `*param_1 = 0`; `0x0058EE05`–`0x0058EE13` |
| `+0x04` | int | 0 at construction; role unknown | ctor `param_1[1] = 0` |
| `+0x08` | int | **Region Id** — `g_nRmgNextRegionId` at construction, then the counter increments | ctor `param_1[2] = DAT_00ABED14; … DAT_00ABED14++` |
| `+0x0C` | int | **CellCount** (the "area" the threshold compares against) | ctor `= 0`; `0x0058EBF4` reset; `0x0058ECAF` increment; `0x0058DD15`/`0x0058DEED`/`0x0058E4C3` increments |
| `+0x10` | int | **Level** (terrain elevation step) — seeded from the seed cell's `CellClass::Level` | ctor `param_1[4] = (char)*(cell+0x11B)`; rewritten at `0x0058E3B1` and `0x0058E422` |
| `+0x14` | byte | **Water/shore flag** — `CellClass__HasBridgeOverlay(seedCell)`; see the label-drift note below | ctor `*(byte*)(param_1+5) = uVar1` |
| `+0x16` | CellStruct | seed cell coord (unaligned dword) | ctor `*(u32*)((int)param_1 + 0x16) = param_2` |
| `+0x1A` | byte | **Finalised** — 0 at construction; set to 1 when `CellCount <= threshold`; gates `Split` | ctor `= 0`; `0x0058EDED` `MOV byte ptr [ESI + 0x1A],1`; read @ `0x0058D631` |
| `+0x1B` | byte | 1 at construction; no writer found in this subsystem | ctor `= 1` |
| `+0x1C` | int | **split counter** — incremented at the end of `Split`; the object is freed right after, so it is a dead write on the live path | ctor `= 0`; `0x0058E554`–`0x0058E561` |
| `+0x20` | int | 0 at construction; role unknown | ctor `param_1[8] = 0` |
| `+0x24` | byte | **Merged/dead flag** — set to 1 when the region is absorbed into `best`; such regions are destructed in `Split`'s epilogue | ctor `= 0`; `0x0058E4EB` `MOV byte ptr [EBP + 0x24],1`; read @ `0x0058DFAF`, `0x0058E101`, `0x0058E52E` |
| `+0x28` | ptr | cell vector: vtable (`PTR_FUN_007E3890`) | ctor `param_1[10] = &PTR_FUN_007E3890` |
| `+0x2C` | ptr | cell vector: array of `CellStruct` | ctor `= 0` |
| `+0x30` | int | cell vector: capacity | ctor `= 0` |
| `+0x34` | byte | cell vector: valid flag (1) | ctor `*(byte*)(param_1+0xD) = 1` |
| `+0x35` | byte | cell vector: is-allocated | ctor `= 0`; freed under this flag @ `0x0058EE68` |
| `+0x38` | int | cell vector: **ActiveCount** | ctor `= 0`; incremented at every push site |
| `+0x3C` | int | cell vector: growth step = **100000** | ctor `param_1[0xF] = 10` then `= 100000` |
| `+0x40` | int | bbox origin X | `0x0058ED16`; reset to 9999 @ `0x0058EC0C` |
| `+0x44` | int | bbox origin Y | `0x0058ED18`; reset to 9999 @ `0x0058EC0E` |
| `+0x48` | int | bbox width | `0x0058ED1B` / `0x0058ED33`; reset to 0 @ `0x0058EC11` |
| `+0x4C` | int | bbox height | `0x0058ED1E` / `0x0058ED59`; reset to 0 @ `0x0058EC14` |

**Correction to the inherited inference set:** `+0x1B` was previously guessed as "connectivity". The
constructor writes `1` there and nothing in this subsystem reads or rewrites it; its meaning stays
unknown (§11). `+0x10 level`, `+0x14 water`, `+0x1A finalised`, `+0x0C area` are all now proven.

**Label drift note (`+0x14`):** the constructor calls `CellClass__HasBridgeOverlay @ 0x004865D0`,
but `decompile_function 0x004865D0` shows the body tests the cell's **iso-tile index (`+0x38`)**
against six tile-set ranges — `g_ShorePieces .. +0x2A`, `g_WaterSet_TileSetBase .. +0xE`, and four
4-tile sets at `DAT_00AA073C`, `DAT_00ABB110`, `DAT_00AA1050`, `DAT_00AA10A0`. It touches no bridge
overlay field. Substantively it is an "is this a water/shore tile" test, which is exactly how
`0x0058EBC0` uses `+0x14` (skip water regions) and how `Split` uses it (a water neighbour can never
become the merge target). The existing Ghidra name is misleading; treat `+0x14` as **water**.

### 6.1 Related globals proven this session

| Global | Meaning | Evidence |
|---|---|---|
| `0x00ABDF90` | region list: DynamicVector header (vtable) | `0x0058EE20` `MOV ECX,0xABDF90`; ctor grow path |
| `0x00ABDF94` | region list: array of `RmgRegion*` | ctor append; `0x0058EBDB` |
| `0x00ABDF98` / `0x00ABDF9D` / `0x00ABDFA4` | region list capacity / is-allocated / growth | ctor grow path |
| `0x00ABDFA0` | region list count | ctor `DAT_00ABDFA0++`; `0x0058EE3E` decrement |
| `0x00ABDFB8` | **`g_RmgGaussianState`** `{bool hasSpare; double spare; pfn uniform}` | `0x005989A6`–`0x005989C9` init; `0x005980C0` consumer |
| `0x00ABE890` | **`g_RmgRandom`** — the 250-lag Fibonacci RNG used by *every* RMG draw | `0x00598996` init; ECX at `0x0058D787`, `0x0058D7D2`, `0x0058D8C6`, `0x0058C7B5`, `0x0058E382`, `0x00598003` |
| `0x00ABED14` | **`g_nRmgNextRegionId`** — next region id; also the size of the adjacency byte array | ctor read+increment; `0x0058DFC1` alloc size; `0x0058E0AE` loop bound |
| `0x00ABED10` | base of the linear map node array (stride `0x50`) | every `[… * 0x50 + DAT_00ABED10]` access |
| `0x00ABED04` / `0x00ABED08` | playfield diamond bounds | the four-way bounds test, all loops |
| `0x0089C2DC` | `g_PathfinderLinearMapWidth` — linear grid is `W × W` | `IMUL EAX,EAX` @ `0x0058D6DF`, `0x0058EC25` |
| `0x00ABE014` | MapType (3 = Inland, 4 = Mountainous) | read @ `0x0058E2F9`; `get_xrefs_to 0x00ABE014` → **WRITE from `RandomMapSetupDialog__Proc @ 0x005967B7`** |

**Linear map node fields used here** (stride `0x50`):
`+0x00` = the cell's `CellStruct` coord (read at `0x0058EC49`);
`+0x38` = region id, or one of the sentinels `-1` (none), `-2` (candidate of the region being
split), `-3` (claimed by the grown set);
`+0x3C` = split-local scratch: `-1` cleared, `-3` "already enqueued".

---

## 7. Tiberian Sun / dead-code check

* `RmgRegion::Split` itself has **no** flag-gated branch. Every branch in it is reachable in a stock
  YR skirmish once MapType is 3 or 4.
* The only conditional feature gate is the MapType 3/4 candidate-zero filter at `0x0058E2F9`. Since
  the entire pass only runs for MapType 3/4 (`0x0058EBC0`'s sole live caller is inside the driver's
  type-3/4 block), the `else` arm of that test is **unreachable in stock YR** — but it is dead by
  reachability, not by a TS legacy flag.
* `0x0058EBC0`'s second Ghidra-visible caller, `FUN_005A1E10` @ `0x005A1E20`, is itself unreferenced
  (noted by the parallel session; `get_function_callers 0x0058D620` confirms `Split` has only the
  one caller, so this does not affect `0x0058D620`).
* Nothing here touches fog of war, subterranean movement, or any other known TS-only subsystem.
* The `+0x1C` split counter increment (§2.8) is effectively dead on the live path — the object is
  freed immediately after. It is not TS legacy, just a vestigial write.

---

## 8. What a Rust port needs (implementation handoff)

Ordered by risk, all of it bit-exactness-critical because every one of these feeds the shared RNG
stream and therefore shifts *everything* generated afterwards:

1. **One RNG stream.** A single 250-lag Fibonacci XOR generator instance for the whole RMG, seeded
   as `RandomMapGenerator::Generate` seeds `0x00ABE890`. Draw order must match §4 exactly, including
   draw #5 (one draw per *accepted* frontier cell, evaluated in N/E/S/W order) and the rejection
   loops (a rejected value still consumes a draw).
2. **Gaussian spare cache.** Persist `hasSpare`/`spare` across the whole generation, reset once per
   `Generate`. Getting this wrong desynchronises the stream on every other growth step.
3. **`Sqrt_Approx` LUT.** Extract the 16384-entry table at `0x008650BC` (64 KB) from the retail exe
   and reimplement the exponent-halve + lookup, returning `f32`. Used by both the frontier cost and
   the Gaussian.
4. **x87 evaluation order.** `((2.5*U) + (1.5*d)) + (0.15*dist)` at 80-bit intermediate precision,
   then rounded to `f32` for the heap key; the threshold's `*2`-last association; `ftol` = truncate
   toward zero with a 53-bit precision control word. Cross-reference
   `docs/research/skirmish-ui/RMG_X87_FP_CONTRACT_GHIDRA_REPORT.md`.
5. **Cell-list ordering.** Descending linear-index order from the `0x0058EBC0` rebuild; DFS order
   from the component fill. RNG draw #2 indexes it directly, so any reordering changes the map.
6. **Heap tie-breaking.** 1-based binary min-heap, strict `<` on the `f32` key, sift-up inlined on
   push, `0x005AD870` sift-down from index 1 after each pop, and the **silent drop** when
   `count+1 >= 2*CellCount+10`.
7. **Region ids are global and monotonic** — the adjacency array in §2.7 is indexed by raw region
   id and sized by the running id counter, so ids must never be recycled.

**Equivalence check for this handoff:** the finite parts (threshold expression, `ftol` mode,
direction table, struct offsets) are proven by exhaustive reading of the instruction listing and can
be certified by unit vectors. The stateful parts (draw ledger, growth order, component order)
**cannot** be certified by Rust-vs-Rust goldens; they need a gamemd-derived trace or a generated-map
byte comparison. Until such an instrument exists, any port of this pass is
**UNVERIFIED-pending-instrument**.

---

## 9. Answers to the specific questions asked

1. **Entry predicate:** only `+0x1A == 0`. **Geometry:** randomized best-first growth from one random
   seed cell along a Gaussian-random-walking heading (§2), then 8-connected component labelling of
   the grown set and its complement (§2.6) — not an axis cut. **Re-entry:** no recursion; the caller
   restarts its scan at index 0 after each split. **Termination:** each new region is either marked
   final (`CellCount <= threshold`) or split again; regions ≤ 100 cells are merged away instead.
   **Caps:** `maxSteps ∈ [area/8, area/3]`, arena `2*area+10` with silent overflow drop.
2. **Draw ledger:** six callsites, §4 — three uniform-with-rejection, two plain uniform, one Gaussian.
   The Gaussian is Marsaglia polar with a rejection loop **and a spare cache** (every other call
   costs zero draws — the brief's "2×N draws" is only true on cache misses).
3. **Threshold:** `trunc(2 * (((0.005*RegionSize + 0.05) * dimA) * dimB))`, chop toward zero, split
   iff `CellCount > threshold` (strict). It lives in `0x0058EBC0`, **not** in `0x0058D620` (§3).
4. **`0x0058EBC0`:** rebuilds cell counts, cell lists and bboxes for every region from the linear
   grid (descending order), computes the threshold once, then splits-or-finalises; culls only the
   regions that were successfully split; consumes **no** RNG directly (§5).
5. **Struct layout:** §6, with writers cited; `+0x04`, `+0x1B`, `+0x20` remain unknown.
6. **TS check:** nothing TS-gated; one branch dead by reachability only (§7).

---

## 10. Discrepancy against the task brief (recorded, not silently corrected)

* The brief located the `RegionSize` threshold "in `FUN_0058D620`". It is in `0x0058EBC0`
  (`0x0058ED89`). `0x0058D620` has no reference to `0x00ABE048`.
* The brief said the split has "4 RNG sites". It has **six** — the two that are easy to miss are the
  per-frontier-cell draw inside `0x0058C6F0` and the per-new-region level draw at `0x0058E382`.
* The brief described the Gaussian as "2×N draws with N geometric (p=π/4)". True only for
  cache-missing calls; the spare cache makes alternating calls free.
* The inferred field `+0x1B = connectivity` is not supported by any writer found here.

---

## 11. Unverified — YELLOW

* **`0x00ABE048` = the `RegionSize` dialog option.** Only one xref exists (the read at `0x0058ED89`);
  no writer references the address directly, so the "MapSeed `+0x70`" identification is inherited
  from a parallel session and was **not** re-proven here. What *is* proven: it is an `int32` whose
  only use in the entire binary is scaling this split threshold.
* **`0x00ABE158` vs `0x00ABE15C` = height vs width.** Only their product is used; the individual
  assignment is unproven.
* **`RmgRegion +0x04`, `+0x1B`, `+0x20`.** Written by the constructor (`0`, `1`, `0`), no reader or
  second writer found in this subsystem.
* **`RmgRegion +0x00`.** Zeroed at construction; `0x0058EE05` treats it as a pointer to an object
  with a virtual destructor. What it points to, and who sets it, was not traced.
* **Empty candidate list with `CellCount > 100`** (§4.5) would read `cand[0]` out of bounds. Whether
  `span ∉ {0,4,8}` is actually reachable depends on the level quantisation produced upstream, which
  was not analysed.
* **Level quantisation.** The `{0, 4, 8}` span arithmetic strongly implies region Levels are
  multiples of 4, but the upstream producer of those Levels was not decoded here.
* **`g_ShorePieces`, `g_WaterSet_TileSetBase`, `DAT_00AA073C`, `DAT_00ABB110`, `DAT_00AA1050`,
  `DAT_00AA10A0`** (the six tile-set ranges behind `+0x14`) are Ghidra labels; the ranges' identity
  as water/shore tile sets was inferred from the label text plus usage, not from the tile-set loader.
* **`0x004CADE0` = `atan`** is taken from its use (`-(dy/dx)` in, angle out, plus the `+π` quadrant
  fixup). Its body was not decompiled, so whether it is an exact CRT `atan` or another approximation
  is **unknown** — and if it is approximate, a port must reproduce it too.
* **`0x00486380` = `CellClass::IsClearTile`** — Ghidra label, body not read this session.
* **The driver's MapType gate.** The parallel session placed it at `0x00598D55`; `0x00ABE014`'s xref
  set does not include that address, so either the gate reads a different copy of MapType or the
  cited address needs re-checking. Not resolved here.

---

## 12. Evidence index (every MCP call used for a load-bearing claim)

```
get_current_program_info
decompile_function   0x0058D620, 0x0058EBC0, 0x0058C6F0, 0x0058BF70, 0x0058D0A0,
                     0x0058E5D0, 0x005AD870, 0x0065C780, 0x0049F2F0, 0x004865D0, 0x0042D510
disassemble_function 0x0058D620, 0x0058EBC0, 0x0058C6F0, 0x005980C0, 0x007C5F00, 0x004CAC40
disassemble_bytes    0x00598000-0x00598040, 0x00598990-0x005989E0, 0x00594B40-0x00594B90
get_function_callers 0x0058D620, 0x0058C6F0, 0x005AD870
get_function_callees 0x0058BF70
get_xrefs_to         0x00ABE048, 0x00ABE158, 0x00ABE15C, 0x00ABE014, 0x0089A304,
                     0x0089F688, 0x00ABDFB8
read_memory          0x007ED890(+8/+16/+24), 0x007ED6A8..0x007ED6B8, 0x007E44E8, 0x007E8AE8,
                     0x007E2820, 0x007E44D0, 0x007E3CC0, 0x007E1748, 0x007E4900,
                     0x00822D80, 0x008650BC, 0x0089F688, 0x0089A300
list_segments
list_globals         name_substring=DirectionOffsets
```

Direction-table values corroborated by the existing repo docs
`docs/research/bridges/01-assets-map-load-overlay/GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`
and re-proven here from `0x0049F2F0`. `CellClass +0x11B = Level` corroborated by
`docs/research/TMP_PER_TILE_HEIGHT_BYTE_GHIDRA_REPORT.md` and by the `RmgRegion` constructor's read.
