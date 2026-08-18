# RMG "meander arm" and border-band retag — `0x005A08D0` / `0x005A0410`

**Target:** gamemd.exe, image base `00400000` (verified via `get_current_program_info`:
`name=gamemd.exe`, `image_base=00400000`, `function_count=10035`).
**Date:** 2026-07-25. **Status:** VERIFIED-from-binary except where the **Unverified** section says otherwise.
**Scope:** the two functions the 2026-07-25 mode-3/4 audit flagged as never actually decoded.

Both were unnamed (`get_plate_comment 0x005A08D0` / `0x005A0410` returned no comment).
Renamed and plated this session:

| Address | New name | Old |
|---|---|---|
| `0x005A08D0` | `RandomMapGenerator__GrowMeanderArm` | `FUN_005a08d0` |
| `0x005A0410` | `RandomMapGenerator__RetagBorderBand` | `FUN_005a0410` |
| `0x005A0700` | `RandomMapGenerator__CollectRegionBorderCells` | `FUN_005a0700` |

> **Headline correction to the audit.** Only **one** of the two is the meander arm.
> `0x005A08D0` **is** the meander arm and it **does** drive the canyon and the bridge
> plateau. `0x005A0410` is **not** a meander arm and **consumes zero RNG draws** — it is a
> ring-dilation retag/flatten helper used by `GrowLake`'s auto-seed path only.

---

## 0. Shared vocabulary

**RMG grid.** Base pointer `[0x00ABED10]`, record stride `0x50`, index
`(y * g_PathfinderLinearMapWidth + x) * 0x50` where `g_PathfinderLinearMapWidth` is
`[0x0089C2DC]` (verified via `list_globals name_substring=PathfinderLinear`, 182 xrefs).
The full grid is `g_PathfinderLinearMapWidth²` records.

Fields used by these functions (all verified from the disassembly cited below):

| Offset | Meaning | Evidence |
|---|---|---|
| `+0x00` | the record's own `CellStruct {short x, short y}` | `decompile_function 0x005A0700` reads it as the emitted coordinate |
| `+0x38` | region tag (`int`) | written = `tag` at `0x005A0D6E`, compared at `0x005A1020` |
| `+0x3c` | BFS / enqueue stamp (`int`) | zeroed map-wide at `0x005A0A43`, set at `0x005A0F6A` |
| `+0x44` | "lake allowed" byte | `decompile_function 0x0059C920` (GrowLake) — not touched by either target |
| `+0x4b` | "claimed by directional growth" byte | sole writer of `1` is `0x005A0D6A` (see §4) |

**CellClass** fields (separate object, from `MapClass__Get_CellClass` `0x005657A0`):

| Offset | Meaning | Evidence |
|---|---|---|
| `+0x38` | isometric tile index | `CellClass__IsClearTile` (`decompile_function 0x00486380`) returns true iff `+0x38 == 0 \|\| +0x38 == 0xFFFF`; `RandomMapGenerator__CarveRiver` writes `g_WaterSet_TileSetBase` here |
| `+0x11a` | sub-tile index within the tileset entry | written alongside `+0x38` with a `U{0..3}` draw in `0x0059E740` |
| `+0x11b` | cell height / level byte | set from the generator base level `this->+0x30c`, `+= 4` / `-= 4` in the canyon and bridge passes, and passed as the height argument to `MapClass__ApplyBridgeTile` |
| `+0x11c` | **not touched** by either target function | negative result, checked across both full disassemblies |

**Isometric bounds test** (used by both). With `D04 = [0x00ABED04]`, `D08 = [0x00ABED08]`,
a cell `(x,y)` is in bounds iff
`x + y > D04 && x - y < D04 && y - x < D04 && x + y <= D08`
(verified `0x005A0DC0`–`0x005A0DE6` and `0x005A0590`–`0x005A05B9`). Both globals are
currently unnamed in the Ghidra project.

**`g_DirectionOffsets` @ `0x0089F688`** — 8 entries of `{short dx, short dy}`, indexed by
`(dir & 7)`, `+X = east`, `+Y = south`:

```
[0] ( 0,-1) N   [1] ( 1,-1) NE  [2] ( 1, 0) E   [3] ( 1, 1) SE
[4] ( 0, 1) S   [5] (-1, 1) SW  [6] (-1, 0) W   [7] (-1,-1) NW
```

The table is **all zero in the file image** (`read_memory 0x0089F660 len 80` → all zeros);
it is filled at runtime by the initializer at `0x0049F2F0`. Proof of the values:
`disassemble_bytes 0x0049F2B0-0x0049F305` shows `0x0049F2F1 XOR EDX,EDX` (⇒ `DX = 0`) and
`0x0049F2F3 OR ECX,0xFFFFFFFF` (⇒ `CX = -1`), and `disassemble_function 0x0049F300` shows
the eight stores in order `(0,CX) (1,CX) (1,DX) (1,1) (DX,1) (CX,1) (CX,DX) (CX,CX)` to
`0x0089F688..0x0089F6A4`. `get_xrefs_to 0x0089F68C` confirms a **single WRITE** in the whole
binary, at `0x0049F322`.

> **Label drift recorded:** the function at `0x0049F2F0` is currently named
> `Foundation_direction_table_init`. It has nothing to do with building foundations — it is
> the map adjacent-cell table initializer. Left renamed-as-is; drift noted in its plate comment.

**RNG.** Every draw in this subsystem is `Random__Next` (`0x0065C780`) on the instance
`g_MapGenRng @ 0x00ABE890` (`MOV ECX,0xabe890` at `0x005A0B53`, `0x005A0C99`, `0x005A0F0F`).
`RandomMapGenerator__NextGaussian` (`0x005980C0`, `MOV ECX,0xabdfb8` at `0x005A1047`) draws
from the *same* stream through its function-pointer uniform source — this is already
established in that function's plate comment and is unchanged by this session.

**Uniform scale constant.** `[0x007ED898]` = `read_memory` → `0x3DF0000000100000` =
`1.0 / 4294967295.0` (= `2.3283064370807974e-10`), **not** `2^-32`. The u32 is zero-extended
into a `qword` and loaded with `FILD qword` (unsigned-correct).

---

## 1. `RandomMapGenerator__GrowMeanderArm` — `0x005A08D0`

### 1.1 Signature and identity

```
char __thiscall RandomMapGenerator__GrowMeanderArm(
        MapSeedClass* this,     // ECX
        int    tag,             // [EBP+0x08]
        float  stepDensity,     // [EBP+0x0C]   <-- decompiler DROPS this; it IS used
        int    rect[4],         // [EBP+0x10]   {x0, y0, w, h}
        short  refCell[2],      // [EBP+0x14]   {x, y}
        char   bClaimFrontier)  // [EBP+0x18]
```

`RET 0x14` (`0x005A1183`) = 20 bytes = 5 stack args, confirming the `__thiscall` + 5 layout.
`this` is the RMG: it is dereferenced at `+0x180` / `+0x184` (map width / height) at
`0x005A08DE`/`0x005A08E8`.

**`stepDensity` is live.** Ghidra's decompiler shows `param_3` unused; the assembly at
`0x005A0C65` is `FMUL float ptr [EBP + 0xc]`. Any port written from the decompiler output
alone will get the step count wrong.

### 1.2 Callers and their arguments (`get_xrefs_to 0x005A08D0`)

Two call sites, both read from the push order via `get_assembly_context`.

**(a) `RandomMapGenerator__CarveRiver` @ `0x0059E3C2` — the CANYON.**

```
0059e384 MOV EDX,[EBX+0x308]      ; tag  = this->+0x308 (current region id)
0059e38a LEA EAX,[ESP+0x74]       ; refCell = the river's original start cell (local_c4)
0059e38e PUSH 0x1                 ; bClaimFrontier = 1
0059e390 LEA ECX,[ESP+0xd4]       ; rect
0059e397 PUSH EAX
0059e398 PUSH ECX
0059e39e PUSH 0x3c23d70a          ; stepDensity = 0.01f
0059e3a3 PUSH EDX
0059e3a4 MOV ECX,EBX              ; this
0059e3a6..0059e3bb  store {EDI, EDI, ESI, ESI} = {0, 0, 0x200, 0x200} into the rect
0059e3c2 CALL 0x005a08d0
```

rect = `{0, 0, 512, 512}` (whole map) ⇒ initial heading `0.0` (east, see §1.4).

Gate on the caller side (`decompile_function 0x0059D510`), all required:
`local_ec == 0` (no bridge placed on this river) **and** success-so-far **and**
`this->+0x30c == 4` **and** not a branch (`local_111 == 0`) **and** a 70 % roll
(`Random__Next()/2^32 < 0.7`, one extra draw at `0x0059E33x` that belongs to CarveRiver's
ledger, not this function's).

`this->+0x30c` is the generator base ground level. `search_instructions operand "0x30c], 0x4"`
finds exactly three sites: `MapSeedClass__InitDefaults 0x005958BE` sets it to `4`,
`CarveRiver 0x0059E335` compares it to `4`, `CarveRiver 0x0059E512` adds `4`. So the canyon
is **live in a stock YR skirmish for the first successful river only** — after one canyon the
level is 8 and the `== 4` gate can never pass again in that generation.

On success the caller runs `FUN_005A0160(tag, 6, 0,0,0x200,0x200, 0,0)` (6-ring dilation),
then sweeps every cell whose grid tag **differs** from the region and does
`CellClass+0x11b += 4`. Player-visible result: the river and the grown arm stay at level 4
while the whole rest of the map rises to level 8 — **the river runs through a 4-level-deep
canyon**, and the arm decides how wide and where that canyon floor wanders.

**(b) `RandomMapGenerator__PlaceBridge` (`FUN_0059E740`) @ `0x0059EC65` — the BRIDGE PLATEAU.**

```
0059ec49 LEA ECX,[ESP+0x3c]
0059ec4d PUSH 0x0                 ; bClaimFrontier = 0
0059ec4f PUSH ECX                 ; refCell = local_ec
0059ec50 MOV ECX,[ESP+0x48]       ; this = the RMG
0059ec54 LEA EDX,[ESP+0x78]       ; rect  = &local_c8[4]
0059ec58 MOV EAX,[ECX+0x308]
0059ec5e PUSH EDX
0059ec5f PUSH 0x3b449ba6          ; stepDensity = 0.003f
0059ec64 PUSH EAX                 ; tag
0059ec65 CALL 0x005a08d0
```

The rect and refCell come from PlaceBridge's `switch(dir)` (`decompile_function 0x0059E740`).
`P3 = param_3` and `P4 = param_4` are the two span endpoints:

| dir | rect `{x0,y0,w,h}` | refCell | resulting heading |
|---|---|---|---|
| 0 (river heading N) | `{0, P3.y-4, 512, 512-(P3.y-4)}` | `(P3.x, P3.y-4)` | `3π/2` → south |
| 2 (river heading E) | `{0, 0, P3.x+4, 512}` | `(P3.x+5, P3.y)` | `π` → west |
| 4 (river heading S) | `{0, 0, 512, P4.y+4}` | `(P4.x, P4.y+1)` | `π/2` → north |
| 6 (river heading W) | `{P4.x-4, 0, 512-(P4.x-4), 512}` | `(P4.x-4, P4.y)` | `0` → east |

i.e. the half-plane **behind** the bridge, growing away from it. If the growth returns 0 the
whole bridge attempt is abandoned (`0059ec6a TEST CL,CL; JZ 0x0059ffd8`). On success,
PlaceBridge dilates 2 rings (`FUN_005A0160(tag, 2, rect…)`), raises every cell **in** the
region by `CellClass+0x11b += 4`, lays the ramp/deck via `MapClass__ApplyBridgeTile` at that
raised height, and finally adds a ±12-cell jump to the river walker
(`local_c8[4..7] = {0, 0xC, 0, -0xC}`, `local_c8[0..3] = {-0xC, 0, 0xC, 0}`;
`*param_7 += local_c8[dir/2 + 4]`, `*param_8 += local_c8[dir/2]`) — **the "bridge far side"**:
the river resumes 12 cells past the bridge, at the unraised level.

### 1.3 Data structures it allocates

```
n        = max(100, this->+0x180 * this->+0x184 * 2)
nodePool = operator_new(n * 8)      // n nodes of { u32 CellStruct coord; f32 cost; }
heap     = operator_new(0x14)       // { int count; int capacity=n; int* slots; void* hi; void* lo; }
heap.slots = operator_new(n*4 + 4)  // 1-BASED binary MIN-heap of node pointers, zeroed
```

Push is a standard sift-up guarded by `if (count + 1 < capacity)` (`0x005A0BB3`); on overflow
the node is written to the pool but **not** enqueued — unreachable in practice because the
pool holds `2·W·H` entries and each cell can be enqueued at most once.
Pop is `slots[1] = slots[count]; slots[count] = 0; count--;` then `FUN_005AD870` (sift-down).

### 1.4 Initial heading (`0x005A0989`–`0x005A0A03`)

```
heading = 0.0
if (rect.x != 0)                 heading = 0.0                    // east
else if (rect.w != 0x200)        heading = π       (0x400921FB54442D18)   // west
if (rect.y != 0)                 heading = 3π/2    (0x4012D97C7F3321D2)   // south
else if (rect.h != 0x200)        heading = π/2     (0x3FF921FB54442D18)   // north
```

The `y` test runs second and **unconditionally overwrites** whatever the `x` test produced
when `rect.y != 0`. Angle convention: an angle `a` points at `(dx, dy) = (cos a, −sin a)`
because the measured angle is `atan(−dy/dx)`.

### 1.5 Cost function (both the seed phase and the expansion phase)

```
dx = c.x - refCell.x ;  dy = c.y - refCell.y
a  = (dx == 0) ? π/2 : atan(-( (double)dy / (double)dx ))      // atan = 0x004CADE0
if (dx < 0) a += π                                             // [0x007E44D0] = π
d  = |a - heading|
while (d >= 2π) d -= 2π                                        // [0x007E3CC0] = 2π
if (d > π) d = 2π - d
u  = (double)(u64)Random__Next() * (1.0/4294967295.0)          // [0x007ED898]
cost = u * 2.0  +  d * 1.5                                     // [0x007ED790]=2.0, [0x007ED798]=1.5
```

Stored as `float` in `node.cost`. Verified at `0x005A0AEA`–`0x005A0B9F` (seed phase) and
`0x005A0EA6`–`0x005A0F4F` (expansion phase) — the two blocks are byte-for-byte the same
sequence of x87 ops. `dx` and `dy` are `MOVSX`-extended 16-bit values; the divide is
`FILD dy; FIDIV dx; FCHS` (integer divide operand, not a double divide).

### 1.6 Step count (`0x005A0C40`–`0x005A0CC8`) — exact x87 order

```
FLDLN2 ; FILD dword[borderCount] ; FYL2X        ->  L = ln(borderCount)     (natural log)
FLD [0x007E1718](=1.0d) ; FCOMP                 ->  V = (1.0 > L) ? 1.0 : L
FDIVR [0x007E1718]                              ->  t = 1.0d / V
FMUL  float[EBP+0x0C]                           ->  t *= (double)stepDensity
FDIVR float[0x007E2AC8](=1.0f)                  ->  t = 1.0f / t
FMUL  float[0x007E5168](=0.5f)                  ->  t *= 0.5f
CALL  Math__ftol (0x007C5F00)                   ->  nBase = trunc(t)
half = nBase / 2                                (CDQ/SUB/SAR = C truncating divide)
do {  r = Random__Next()
      v = ftol( (double)(u64)r * (double)(u64)(half+1) * [0x007ED898] )   // multiply order matters
   } while ((unsigned)v > (unsigned)half)
nSteps = nBase + v
```

Constant values, all from `read_memory`: `0x007E1718` = `1.0` (f64),
`0x007E2AC8` = `1.0f`, `0x007E5168` = `0.5f`.

Algebraically `nBase = trunc(0.5 · max(1, ln B) / stepDensity)`, i.e.
`50 · max(1, ln B)` for the canyon call and `166.66… · max(1, ln B)` for the bridge call —
but the port must reproduce the *op sequence*, because the two reciprocals and the
float/double mixing are not associative.

The rejection loop is **mathematically dead**: `v = trunc(u · (half+1))` with
`u = r/(2³²−1) ∈ [0,1]` yields `v ≤ half` for every `r`. It is therefore **exactly one draw**,
always. (Keep the draw; drop the loop.)

If `borderCount == 0`, `FYL2X` gives `−inf`, `V` collapses to `1.0`, and the step count draw
still happens even though the main loop breaks on the first iteration (empty heap).

### 1.7 Control flow

```
1. Zero grid[c].+0x3c for every cell reachable by MapClass__CellIterator (0x005A0A43).
2. borderList = CollectRegionBorderCells(tag)   [0x005A0A58]
   B = borderList.count
   SEED PHASE, in list order:
     for each border cell c:
        if !(rect.x0 <= c.x < rect.x0+rect.w  &&  rect.y0 <= c.y < rect.y0+rect.h)  -> skip, NO draw
        node = pool[next]; node.coord = c; node.cost = cost(c)     // 1 uniform draw
        heap.push(node)
   delete borderList
3. nSteps = <§1.6>                                                  // exactly 1 uniform draw
4. cur = heap.pop()   (NULL if empty)
5. for (step = 0; step < nSteps; step++)          // head re-tests ok && cur != NULL
     a. if grid[cur].+0x38 == 0 && CellClass__IsClearTile(cur):
             grid[cur].+0x4b = 1
             grid[cur].+0x38 = tag
     b. for d in {0, 2, 4, 6}:                     // N, E, S, W  -- 4-connected
             n = cur + g_DirectionOffsets[d]        (MapCoord_Add 0x0042D510)
             if !inDiamondBounds(n): continue                       // NO draw, NO abort
             if grid[n].+0x38 == 0 && grid[n].+0x3c != tag
                && inRect(n) && CellClass__IsClearTile(n):
                    node = pool[next]; node.coord = n
                    node.cost = cost(n)                             // 1 uniform draw
                    grid[n].+0x3c = tag
                    heap.push(node); continue
             t = grid[n].+0x38
             if t == 0: continue                                    // unclaimed -> fine
             if t != tag: ok = false                                // FOREIGN REGION -> abort
     c. heading += NextGaussian() * π/4                             // [0x007E3D88], ALWAYS
     d. cur = heap.pop()
6. if bClaimFrontier != 0:
     while ((cur = heap.pop()) != NULL && ok):
        t = grid[cur].+0x38
        if t == 0: grid[cur].+0x38 = tag      // NOTE: no IsClearTile test, no +0x4b write
        else if t != tag: ok = false
7. free pool, free heap.  return ok
```

Two ordering facts a port will get wrong if it reads the decompiler casually:

* Step 5c fires **on every executed iteration**, including the iteration that sets `ok = false`
  and the iteration that empties the heap — both conditions are only re-tested at the loop
  head (`0x005A0D18`/`0x005A0D24`). So a failing arm still burns its Gaussian.
* The four neighbour directions are scanned in the fixed order **N, E, S, W** and a direction
  that enqueues does `goto` past the collision test, so a cell can enqueue *or* abort, never both.

### 1.8 Complete RNG draw ledger

All draws are `g_MapGenRng @ 0x00ABE890` via `Random__Next 0x0065C780`.

| # | When | Site | Kind | Count |
|---|---|---|---|---|
| 1 | seed phase, per border cell **inside** `rect`, in `CollectRegionBorderCells` emission order | `0x005A0B5A` | raw uniform | one each; cells outside `rect` consume **zero** |
| 2 | step-count, once, unconditionally | `0x005A0C9E` | raw uniform | exactly 1 (rejection loop is unreachable) |
| 3 | per step, per **accepted** neighbour, scanned N→E→S→W | `0x005A0F16` | raw uniform | 0–4 per step |
| 4 | per step, after (3), unconditional | `0x005A104C` | **Gaussian** | `2·N` raw draws, `N ≥ 1` geometric(`p = π/4`), **or 0** when the polar cache is primed |
| 5 | frontier-claim tail loop | — | — | **none** |

Total = `1 + (#border cells inside rect) + (#accepted neighbours) + Σ(Gaussian raw draws)`.

Gaussian caveat (already documented on `0x005980C0`, restated because it dominates this
function's stream): the generator is Marsaglia polar with an unbounded rejection loop, two
uniforms per attempt, acceptance `π/4`, and it caches its second value. So consecutive steps
alternate roughly `0, 2·N, 0, 2·N, …` and ≈21.5 % of refills reject at least once. A port that
models "2 draws per Gaussian" desyncs.

### 1.9 State written, and who consumes it

| Write | Site | Consumers |
|---|---|---|
| `grid[c].+0x3c = 0` (whole map) | `0x005A0A43` | internal reset only |
| `grid[c].+0x3c = tag` | `0x005A0F6A` | internal enqueue stamp only — every other RMG pass zeroes it first |
| `grid[c].+0x38 = tag` (main loop, clear-tile only) | `0x005A0D6E` | `CollectRegionBorderCells`, `FUN_005A0160`, the canyon `+0x11b += 4` sweep at `0x0059E42x`, PlaceBridge's `+0x11b += 4` sweep, `MapClass__MarkBridgesForRepair_High`, `GrowLake`'s `+0x44` mask, and the CarveRiver/GrowLake rollback sweeps |
| `grid[c].+0x38 = tag` (tail loop, no clear-tile test) | `0x005A111D` | same |
| `grid[c].+0x4b = 1` | `0x005A0D6A` | **sole writer of 1 in the whole binary.** Only reader: `0x0058EB1B` in `FUN_0058E9B0` (a later region flood that requires `grid+0x38 == -1 && +0x4b != 0`). Cleared to 0 at `0x0059D404` (GrowLake rollback), `0x0059E584` / `0x0059E5FF` (CarveRiver rollback), `0x0058CC3C` / `0x0058CDC2` (`FUN_0058C800`), `0x0059C4E2` (`FUN_0059BBC0`). Enumerated with `search_instructions operand_pattern="0x4b]"` (38 program-wide matches). |
| **no CellClass write at all** | — | this function never paints a tile, sub-tile or height |

### 1.10 Tiberian Sun / dead-branch check

Nothing here is TS legacy; both call sites are reachable in a stock YR skirmish
(`RandomMapGenerator__Generate 0x00598960` → water branch for map types 3/4). Branches that
never execute are *defensive*, not dormant features:

* `[0x00ABED10] == 0` guards (`0x005A0DF4`, `0x005A0FF8`, `0x005A10F9`) — the grid is always
  allocated once `Generate` is running. If ever taken they force `t = -1 ≠ tag` ⇒ abort.
* heap-capacity-full guard (`0x005A0BB5`, `0x005A0F97`) — capacity `2·W·H` exceeds the number
  of distinct enqueueable cells.
* the step-count rejection retry (§1.6) — mathematically unreachable.
* `bClaimFrontier` tail loop — live for the canyon call (`=1`), dead for the bridge call (`=0`).

---

## 2. `RandomMapGenerator__RetagBorderBand` — `0x005A0410`

### 2.1 Signature, caller, arguments

```
undefined4 __thiscall RandomMapGenerator__RetagBorderBand(
        MapSeedClass* this,  // ECX
        int srcTag,          // [ESP+0x34 at entry]
        int nRings,          // [ESP+0x38]
        int newTag)          // [ESP+0x3C]
```

`RET 0xC` (`0x005A06EE`) = 3 stack args. Always returns `1` (`0x005A06E8 MOV AL,0x1`).

**Single caller** (`get_xrefs_to 0x005A0410`): `RandomMapGenerator__GrowLake` @ `0x0059CA9A`.
Push order from `get_assembly_context`:

```
0059ca92 PUSH -0x2      ; newTag  = -2
0059ca94 PUSH 0x2       ; nRings  = 2
0059ca96 PUSH 0x0       ; srcTag  = 0
0059ca98 MOV ECX,EBX    ; this
0059ca9a CALL 0x005a0410
```

It is reached only on `GrowLake`'s **auto-seed** path (seed cell `== {0,0}`), which is how
`RandomMapGenerator__SeedWaterInlandMountain 0x0059C580` always calls `GrowLake`. The
surrounding sequence (`decompile_function 0x0059C920`):

```
for all cells: grid[c].+0x44 = 0          // clear the "lake allowed" mask
RetagBorderBand(srcTag=0, nRings=2, newTag=-2)
for all cells:
    t = grid[c].+0x38
    if (t == 0 || t == this->+0x308) grid[c].+0x44 = 1     // allowed
    else if (t == -2)                grid[c].+0x38 = 0     // undo the temp tag
```

**Net player-visible effect:** a new auto-seeded lake can never start, or grow into, any land
cell within 2 cells of an already-tagged (water / previously-claimed) cell — a 2-cell
separation margin between lakes and between a lake and the river.

### 2.2 Algorithm

```
ring = CollectRegionBorderCells(srcTag)                     // 0x005A0422
if (nRings > 1) zero grid[c].+0x3c over every map cell      // 0x005A0478
for (i = 0; i < nRings; i++) {
    for (k = ring.count-1; k >= 0; k--) {                   // reverse index order
        c = ring[k]
        grid[c].+0x38       = newTag                        // 0x005A04CA
        CellClass(c).+0x38  = 0                             // 0x005A04DD
        CellClass(c).+0x11a = 0                             // 0x005A04E0
        CellClass(c).+0x11b = (u8)this->+0x30c              // 0x005A04EE
    }
    if (i < nRings-1) {
        next = new DynamicVector (growth-step = ring.count)
        for (k = ring.count-1; k >= 0; k--)
            for (d = 0; d < 8; d++) {                       // ALL 8 directions
                n = ring[k] + g_DirectionOffsets[d]
                if (!inDiamondBounds(n)) continue
                if (grid[n].+0x38 != srcTag) continue
                if (grid[n].+0x3c == i+1)   continue
                next.push(n)
                grid[n].+0x3c = i+1
            }
        delete ring; ring = next
    }
}
delete ring
if (nRings > 1) zero grid[c].+0x3c over every map cell       // 0x005A06D8
return 1
```

Note the terrain write: as well as retagging, it **flattens** each band cell to tile 0,
sub-tile 0, and the generator's base level `this->+0x30c` (`= 4` initially, from
`MapSeedClass__InitDefaults 0x005958BE`). In the `GrowLake` usage the cells are then
un-retagged, but the CellClass flatten is **not** undone.

### 2.3 RNG draw ledger

**ZERO draws.** `disassemble_function 0x005A0410` contains no `CALL 0x0065C780`
(`Random__Next`) and no `CALL 0x005980C0` (`NextGaussian`). Its callees are
`RandomMapGenerator__CollectRegionBorderCells` (also draw-free, §3),
`MapClass__CellIterator_Init/Next` (`0x00578350`/`0x00578290`),
`MapClass__Get_CellClass` (`0x005657A0`), `operator_new` (`0x007C8E17`), the
`DynamicVectorClass` constructor `0x0042FCB0`, and the vector's virtual `Resize`/`dtor`.

### 2.4 Tiberian Sun / dead-branch check

Live in stock YR: every `GrowLake` attempt launched by `SeedWaterInlandMountain` (up to 10 per
generation) passes through it. The `nRings > 1` guards around the `+0x3c` clears are live
(`nRings == 2`). The `piVar3 == NULL` allocation-failure branch is defensive. No TS-only path.

---

## 3. `RandomMapGenerator__CollectRegionBorderCells` — `0x005A0700`

Support function for both targets (and for `FUN_005A0160`); documented here because the
meander arm's seed-phase draw order is exactly this function's emission order.

```
DynamicVectorClass<CellStruct>* __thiscall CollectRegionBorderCells(MapSeedClass* this, int tag)
```

Growth step = `this->+0x184 * this->+0x180` (map W·H). It sweeps the grid by **raw byte
offset** (`g_PathfinderLinearMapWidth²` records × `0x50`), not by the map cell iterator, and
emits a record when:

1. the record's own `CellStruct` at `+0x00` is not `(0,0)` (cell `(0,0)` is skipped), **and**
2. `grid[c].+0x38 == tag`, **and**
3. at least one **in-bounds** 8-neighbour has `grid[n].+0x38 != tag`.

Out-of-bounds neighbours are skipped and do **not** qualify a cell as a border cell, so rim
cells only count if they actually touch a differently-tagged cell. The loop `break`s on the
first differing neighbour, so each cell is emitted at most once. **Zero RNG draws.**

---

## 4. Port-facing summary (what the mode-3/4 water phase needs)

1. Implement `GrowMeanderArm` as: best-first min-heap growth, cost `2·U + 1.5·angleDiff`,
   heading seeded from the rect and random-walked by `Gaussian · π/4` per step, over the
   4-connected N/E/S/W neighbourhood, aborting on contact with a foreign region tag.
2. Draw stream per §1.8. The step count must use the exact x87 op sequence in §1.6, including
   the double→float→double round trips.
3. It writes only grid state (`+0x38`, `+0x3c`, `+0x4b`) — never tiles or heights. The visible
   canyon/plateau comes from the *callers'* `+0x11b ± 4` sweeps after `FUN_005A0160` dilation.
4. `RetagBorderBand` is a plain 2-ring dilation that retags to `-2` and flattens tile/sub-tile/
   height to the base level; it consumes no randomness, so it can be implemented in any
   internal order without touching the stream.

---

## 5. Unverified (YELLOW)

Everything below is **not** proven from the binary this session and must not be relied on.

* **`FUN_005A0160`** (`0x005A0160`) — read only far enough to see it is an *n*-ring dilation
  that retags to `param_1`, aborts on foreign tags, and optionally writes `CellClass+0x11b`.
  Its `param_7`/`param_8` semantics and its interaction with `CellClass__HasBridgeOverlay`
  were **not** decoded. UNCHECKED.
* **`FUN_0058E9B0`** — identified as the only reader of `grid+0x4b`; the pass it belongs to,
  and what `grid+0x38 == -1` means there, were not investigated. UNCHECKED.
* **`FUN_0059E740` (`PlaceBridge`) internals** — the `switch(dir)` rect/refCell table in §1.2
  is transcribed from `decompile_function 0x0059E740` and was **not** re-verified against the
  assembly. Which of `P3`/`P4` is the near vs far span endpoint is UNCHECKED, so the
  "half-plane behind the bridge" reading of the rect is INFERRED, not proven.
* **`[0x00ABED04]` / `[0x00ABED08]`** — used as the isometric diamond bounds; their exact
  definition and writers were not traced. The *test* is verified; the *names* are not.
* **`CellClass+0x11b` = "height/level"** — supported by three independent uses (set from the
  generator base level; `± 4` in the canyon/bridge passes; passed as the height argument to
  `MapClass__ApplyBridgeTile`) but not confirmed against the map-file `Level` field.
  Treat the *offset* as verified and the *unit* as INFERRED.
* **The 70 % canyon roll and the `+0x30c == 4` gate** are read from
  `decompile_function 0x0059D510` plus `search_instructions "0x30c], 0x4"`. The comparison
  constant `0.7` was **not** re-read from memory. UNCHECKED.
* **No equivalence check exists yet.** Nothing in this report is parity-certified: there is no
  named repo test, emulation vector, or gamemd-derived trace for either function.
  Status: **UNVERIFIED-pending-instrument**. The natural instrument is `emulate_function` over
  `0x005A08D0` with a seeded `g_MapGenRng` and a synthetic grid, compared against a Rust port —
  that spike has not been run.
