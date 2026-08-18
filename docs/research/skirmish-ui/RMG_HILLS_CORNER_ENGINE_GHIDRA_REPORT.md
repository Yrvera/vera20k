# RMG "Hills" Corner-Morph Engine — Ghidra Ground-Truth Report

**Binary:** gamemd.exe (project `testProsjekt`, program `/gamemd.exe`)
**Scope:** the corner-height grid engine that the RMG hills pipeline `FUN_005a35f0`
builds, morphs, and finalizes. Read-only Ghidra decode; every claim cites the MCP call
used to verify it this session.
**Status of source labels:** treated as navigation hints only; all behavior verified from
function bodies + disassembly. `param_1`/arg models from the decompiler were cross-checked
against `disassemble_bytes` wherever the calling convention mattered (it mattered — see §6).

---

## 0. Orientation — where the engine sits

`FUN_005a35f0` (the hills driver, verified `decompile_function 0x005a35f0`) runs, in order:

1. `FUN_005a33f0()` — pre-pass (RNG here, not our scope).
2. `FUN_005a2f50()` — **the walk**; owns ALL hill RNG (context, confirmed by absence below).
3. `FUN_006b2a70()` — **corner-grid build** (§1).
4. per-scratch-cell **morph loop** (§0.1) calling `FUN_006b4100` (§3), `FUN_006b4240`
   (§4), `FUN_006b3e60` (§5) → `FUN_006b3a80` (§6).
5. `FUN_006b3850()` — **finalize** (§7).

Steps 3–5 consume **no RNG** (see §9). The scratch heights consumed in step 4 were
produced by the walk in step 2.

### 0.1 Driver morph loop — direction sign & step count (load-bearing for the port)

Evidence: `disassemble_bytes 0x005a3640 len 220`.

Per scratch cell `ESI` (stride `0x50`, base `DAT_00abed10`, count `g_PathfinderLinearMapWidth²`)
with a non-zero coord:

```
target(double) = (double)cell.level(+0x11b)  FADD  scratch.height(double @ scratch+0x08)
current(int)   = FUN_006b4100(cell)                       ; §3
delta(int)     = ftol_0x007c5f00( target - current )      ; truncate toward zero
count          = abs(delta)
direction      = (delta >= 0) ? +1 : -1                   ; SETGE/DEC/AND 0xFFFFFFFE/INC idiom
repeat count times:
    mask = FUN_006b4240(cell, direction)                  ; §4  (EDX = direction)
    FUN_006b3e60(direction, force=0, scratch.coord, mask) ; §5  (ECX=dir, DL=0, push mask, push coord)
```

So **direction = +1 raise / −1 lower**, and the driver **always passes force = 0** (rollback
active). `scratch.coord` = the packed `(x,y)` at `scratch+0x00`. `scratch+0x08` is the
`double` fractional height the walk deposited.

---

## 1. `FUN_006B2A70` — corner-grid build

Evidence: `decompile_function 0x006b2a70`; `read_memory 0x0083ff18`, `0x0083fdd8`,
`0x00b0b6dc`; helpers §2.

### Allocation
- Frees the previous grid (`FUN_007c8b3d(DAT_00b0b6ec)` = free/operator delete) if non-null.
- `count = (DAT_0087f918 + 1) * (DAT_0087f914 + 1)` entries; `operator_new(count * 8)`.
  - `DAT_0087f914` = interior map **width** W (cells); `DAT_0087f918` = interior **height** H.
- Zero-inits every entry: `[+0]=0 (int height)`, `[+4]=0`, `[+5]=0`.
- Publishes geometry into working globals:
  - `DAT_008759a8 = DAT_0087f90c` (origin **x**), `DAT_008759ac = DAT_0087f910` (origin **y**)
  - `DAT_008759a0 = H+1` (grid rows), `DAT_008759a4 = W+1` (grid **stride** = corners per row)
- `DAT_00b0b6ec` = base pointer of the corner grid.

### Corner grid layout (the data structure)
`(W+1) × (H+1)` grid, **row-major**, entry stride **8 bytes**:

| off | type | meaning |
|-----|------|---------|
| +0  | i32  | corner **height** (always a multiple of 15; clamped [0,180]) |
| +4  | u8   | **LOCKED** flag (set here at build; immovable) |
| +5  | u8   | **VISITED/MODIFIED** flag (set by the morph, read by finalize) |
| +6  | u8   | unused in RMG (see §6; only a dormant force-override reads it) |
| +7  | u8   | pad |

Index of corner `(cx, cy)` (grid-local) = `cy*(W+1) + cx`. Map coord → grid-local:
`cx = map_x − origin_x`, `cy = map_y − origin_y`.

**Cell → corner mapping** (confirmed identically in §3/§4/§7): cell at grid-local `(cx,cy)`
owns the 4 corners
- **NW** = corner(cx, cy)        → index `i`            (`i = cy*(W+1)+cx`)
- **NE** = corner(cx+1, cy)      → index `i+1`   (byte `+8`)
- **SW** = corner(cx, cy+1)      → index `i+(W+1)`
- **SE** = corner(cx+1, cy+1)    → index `i+(W+1)+1`

### Per-corner height formula
For each grid point `(cx,cy)` with map coord `(mx,my) = (cx+origin_x, cy+origin_y)`, the
build resolves an in-diamond **owner cell**, then:

```
height = ramp_delta[ owner.slope ][ 0 /* NW */ ] + owner.level(+0x11b) * 15
```

`ramp_delta[slope]` is `PTR_DAT_0083ff18[slope]` (a pointer; see §8). **Index [0] (NW) is
used unconditionally**, even when the owner cell was reached by the NW/N/W fallback — a
deliberate boundary approximation (those corners are locked anyway).

Owner-cell resolution (the `local_c` walk): starting from `(mx,my)`, try in order
`(mx,my)`, `(mx,my−1)`, `(mx−1,my−1)`, `(mx−1,my)` and take the **first in-diamond** one;
if none is in-diamond, the corner is locked (height stays 0). "In-diamond" = the 4-inequality
test in §2 (`FUN_005ac230`, inlined here).

### Corner-locking predicate (the exact conditions)
The corner is **LOCKED** (`entry+4 = 1`) if *any* of these adjacent tests demands it. Each
test looks at a neighbor cell of the corner:

1. **North cell** `(mx, my−1)` — if in-diamond, LOCK **unless**:
   `overlay(+0x44)==−1 && occupier(+0xe4)==0 && scratch(+0x45)==0 &&
    (tile(+0x38) ≥ IsoTileTypeCount || tile==0xFFFF || IsoTileType[tile]+0x2E0 != 0)`.
2. **NW cell** `(mx−1, my−1)` — same predicate.
3. **West cell** `(mx−1, my)` — if in-diamond: get cell, LOCK if `FUN_006b2520()==0` (§2).
4. **This corner's own cell** `(mx, my)` — if in-diamond (`FUN_005ac230`): LOCK if
   `FUN_006b2520()==0`.
5. Owner cell resolved **out-of-diamond** → LOCK.

> **Correction to the task context.** The gloss "`IsoTileType+0x2E0 != 0` ⇒ *not*
> morphable" is **inverted**. In every use (build blocks 1–2, `FUN_006b2520`, `FUN_006b4100`,
> finalize) the clause `(tile≥count || tile==0xFFFF || IsoTileType[tile]+0x2E0 != 0)` is the
> **"OK to morph / don't lock"** branch. Therefore **`IsoTileType[tile]+0x2E0 != 0` means the
> tile PERMITS morphing**; `== 0` (a real fixed tile without the flag) is what forces the
> lock. Verified by purpose (leaving a corner unlocked = "surrounding terrain is editable")
> and by `FUN_006b2520`'s return wiring. The address `+0x2E0` and the `!= 0` test are correct;
> only the English is backwards. Exact INI name of the flag not resolved this session.

Note: `local_2c` (the sampled height) is written to `entry+0` **for every corner regardless
of lock state**; locking only sets the `+4` byte.

---

## 2. `FUN_006B2520` — cell water/morphability test  &  two inlined helpers

Evidence: `decompile_function 0x006b2520`, `0x005ac230`, `0x0058c2a0`.

### `FUN_0058c2a0(coord*)` — scratch-cell address (pure arithmetic, no RNG)
```
return (coord.y * g_PathfinderLinearMapWidth + coord.x) * 0x50 + DAT_00abed10;
```
Stride `0x50`, base `DAT_00abed10`. This is how every "scratch +0x45 water flag" read is
located.

### `FUN_005ac230(coord*)` — in-diamond predicate (no RNG)
Returns 1 iff, with `x=coord[0], y=coord[1]`:
```
(x+y > DAT_00abed04) && (x−y < DAT_00abed04) && (y−x < DAT_00abed04) && (x+y <= DAT_00abed08)
```
`DAT_00abed04` = diamond **min** bound, `DAT_00abed08` = diamond **max** bound. This exact
4-inequality block is inlined throughout the engine.

### `FUN_006B2520(CellClass* cell)` — returns the "OK-to-morph" verdict for a cell
`__fastcall` (ECX = cell). Returns:
- **0** if: coord `(cell+0x24)` out-of-diamond, **or** `overlay(+0x44) != −1`, **or**
  `occupier(+0xe4) != 0`, **or** `scratch(+0x45) != 0` (water/protected).
- else, with `tile = cell+0x38`:
  - if `tile < IsoTileTypeCount && tile != 0xFFFF`: return `IsoTileType[tile] + 0x2E0` byte
    (the morph-permit flag — nonzero = morphable).
  - else (no real tile): return **1**.

So `FUN_006b2520()==0` ⇒ "this cell blocks morphing ⇒ lock the adjacent corner". Consistent
with §1's inverted-gloss note.

---

## 3. `FUN_006B4100` — current level of a cell

Evidence: `decompile_function 0x006b4100`.

`int __fastcall FUN_006b4100(CellClass* cell)`:

1. Load the cell's 4 corner heights from the grid (order `[NW, NE, SE, SW]` — same mapping
   as §1): `iVar4 = cy*(W+1)+cx` (NW), `+8`=NE, `iVar1 = (cy+1)*(W+1)+cx` (SW), `iVar1+8`=SE.
2. `m = min(NW, NE, SE, SW)` (seed 1000).
3. **Eligibility** (identical to §2's `FUN_006b2520` core): in-diamond AND `overlay==−1` AND
   `occupier==0` AND `scratch+0x45==0` AND `(tile≥count || tile==0xFFFF || IsoTileType[tile]+0x2E0 != 0)`.
   - **eligible →** `return m / 15` (integer; `m ≥ 0`, so plain floor division, range 0..12).
   - **ineligible →** `return (signed char)cell.level(+0x11b)` (the cell's own stored level).

"Current level" = the floor of the minimum corner height ÷ 15 for editable cells, else the
persisted level byte. No RNG.

---

## 4. `FUN_006B4240` — corner-mask picker

Evidence: `decompile_function 0x006b4240` (offsets cross-checked against the §1 layout).

`int __fastcall FUN_006b4240(CellClass* cell, int direction)`.

Loads the cell's 4 corners into `local_30[0..3] = [NW, NE, SE, SW]` **and** each corner's
byte at **`entry+4` = the LOCKED flag** (not +5). Computes `mn=min`, `mx=max`. Returns a
4-bit mask; **bit i corresponds to corner i in [NW=bit0, NE=bit1, SE=bit2, SW=bit3]**:

| case | condition to set corner i's bit |
|------|--------------------------------|
| `mx == mn` (all equal) | `LOCKED_i == 0` — every unlocked corner |
| `direction > 0` (raise) | `height_i < mx && LOCKED_i == 0` — unlocked corners **below the max** |
| `direction <= 0` (lower) | `height_i > mn && LOCKED_i == 0` — unlocked corners **above the min** |

> **Correction to the task context.** The filter is the **LOCKED** flag (`entry+4`), **not**
> a "visited" flag. `entry+4` is set only at build time (§1); the morph never rewrites it.
> `entry+5` (visited) is written by the adjust/propagation (§5/§6) and is read only by
> **finalize** (§7) — the picker never consults it. The height comparisons (`< max` / `> min`
> / all-equal) in the context are correct; the flag source is not "visited".

Bit order NW=0,NE=1,SE=2,SW=3 verified by the `1 << i` loop over `local_30 = [NW,NE,SE,SW]`.
(§5 re-maps this order for its own row-major iteration — see §5.) No RNG.

---

## 5. `FUN_006B3E60` — corner adjust + undo (2×2 block apply)

Evidence: `decompile_function 0x006b3e60`; driver call `disassemble_bytes 0x005a3640`
(args); undo-stack shape cross-checked with §6 disassembly.

`char __fastcall FUN_006b3e60(int direction /*ECX*/, char force /*DL*/, packed_coord coord,
uint mask)` — returns **1 = success**, **0 = failed & rolled back**. Driver passes
`direction = ±1`, `force = 0`.

### Undo stack (globals)
| global | role |
|--------|------|
| `DAT_00b0b654` | undo buffer base pointer |
| `DAT_00b0b660` | undo write index / count (reset to 0 at entry) |
| `DAT_00b0b658` | current capacity (entries) |
| `DAT_00b0b664` | grow increment |
| `DAT_00b0b65d` | "buffer allocated" byte |
| `DAT_00b0b650` | DynamicVectorClass-style object; `(*(vtbl+8))(newCount,0)` grows it |

Undo entry = **12 bytes** `{ i32 corner_x, i32 corner_y, i32 old_height }`. Ensure-capacity:
if `count < capacity` push directly, else (if growable) call the vtable grow at `+8`. For the
port this is just a `Vec<UndoEntry>`.

### The apply
Iterate the **2×2 corner block** whose NW is the cell's NW corner: grid rows
`cy..cy+2`, cols `cx..cx+2`. A bit `local_14` walks `1,2,4,8` in **row-major** order
`NW(bit0), NE(bit1), SW(bit2), SE(bit3)`.

The incoming `mask` (picker order NW,NE,**SE**,**SW**) is remapped to this iteration order by
`m' = (mask>>1 & 4) | ((mask & 4)<<1) | (mask & 3)` — i.e. **swap bits 2↔3** (SE/SW). This
remap is verified in both `FUN_006b3e60` and its sibling `0x006b3cd0`
(`disassemble_bytes … 0x006b3d3a-0x006b3d4d`).

For each block corner selected by `m'`:
```
if (corner.LOCKED(+4) == 0) {
    new = corner.height + direction*15;           // ±15
    corner.height = new;
    if (new < 0 || new > 0xB4 /*180*/) {          // clamp violation
        corner.height = new - direction*15;       // restore; NO undo entry recorded
    } else {
        push_undo{ x, y, old = new - direction*15 };
    }
    corner.VISITED(+5) = 1;                        // set unconditionally in this branch
    ok = FUN_006b3a80(direction, force, x, y);     // slope propagation, §6
    if (!ok && force == 0) { rollback(); return 0; }
} else if (force == 0) {                           // corner is LOCKED and not forced
    rollback(); return 0;
}
```
- **Sign source:** `direction*15` (`+15` raise, `−15` lower). Height clamp is **[0, 0xB4]**
  (0..180 = 12 levels × 15).
- **rollback()** replays undo entries **LIFO**, restoring each `corner.height = old`, then
  frees the buffer. It restores **heights only** — VISITED bytes set during a failed op stay
  set (harmless: finalize re-derives the same unchanged slope, §7).
- Success (all masked corners processed, no fatal propagation failure) → loop exhausts →
  **return 1**.

---

## 6. `FUN_006B3A80` — slope propagation (recursive)

Evidence: `decompile_function 0x006b3a80` **and** full `disassemble_bytes 0x006b3a80 len 720`
(the decompiler mis-modeled the recursion — assembly is authoritative here).

`char __fastcall FUN_006b3a80(int direction /*ECX*/, char force /*DL*/, int corner_x /*stack*/,
int corner_y /*stack*/)`, `RET 0x8`. Returns **1 = success**, **0 = failure**.

- `center = grid[corner_y*(W+1)+corner_x].height` (saved at `[ESP+0x14]`).
- Iterate the **8 neighbors** `(dx,dy) ∈ {−1,0,1}²  \ (0,0)` (inner `dx = EDI`, outer
  `dy = EBP`, both `−1..1`):
  - **Out of grid bounds** (`nx<0 || ny<0 || nx>=W+1 || ny>=H+1`, using `DAT_008759a4=W+1`,
    `DAT_008759a0=H+1`): if `force==0` **return 0 (fail)**; else skip.
  - In-bounds neighbor `n`:
    - If `n.LOCKED(+4) != 0` **and not** (`force && n[+6] != 0`): compute
      `d = |n.height − center|`; **if `d > 15` return 0 (fail)**; else continue (no change).
      *(A locked corner that would need to move fails the whole op.)*
    - Else (movable): if `|n.height − center| <= 15` → continue (no change). If `> 15`:
      - **raise** (`direction == 1`): if `n.height < center`, push_undo, set
        `n.height = center − 15`, `n.VISITED = 1`.
      - **lower** (`direction != 1`): if `n.height > center`, push_undo, set
        `n.height = center + 15`, `n.VISITED = 1`.
      - Then, if `n.LOCKED == 0`, **recurse** `FUN_006b3a80(direction, force, nx, ny)`
        (verified at `0x006b3c68-0x006b3c7c`: ECX=direction and DL=force are carried through
        unchanged; only the two stack args become `nx,ny`). If the recursion returns 0 and
        `force==0` → **return 0 (fail)**; if forced, ignore and continue.
- Exhaust all 8 neighbors → **return 1**.

**The ">15" test** is the abs-diff `CMP EAX,0xf; JG/JLE` idiom (`0x006b3b52`, `0x006b3b6b`):
neighbors differing from the just-moved corner by **more than one level** are pulled along by
exactly one level, recursively. **Any LOCKED corner within the >15 reach fails the whole
operation** (returns 0), which unwinds to `FUN_006b3e60` and replays the undo stack.

> The `n[+6]` force-override path (`0x006b3b42`) is **dead in standard RMG**: build never
> writes `+6`, and the driver always calls with `force==0`, so this branch is unreachable in a
> normal skirmish. Documented for completeness; do not port as active behavior.

> Sibling `0x006b3cd0` (seen in the same disassembly window) is a **force-apply** variant of
> §5 (`force=1`, resets state via `FUN_006b2700`, same 2↔3 bit remap). Not on the required
> list and not invoked by the main morph loop; noted only so it isn't mistaken for §5.

---

## 7. `FUN_006B3850` — finalize (tile + slope + level write-back)

Evidence: `decompile_function 0x006b3850`; ramp table `read_memory 0x0083ff18`/`0x0083fdd8`.

Iterates every map cell (`MapClass__CellIterator_Init/Next`). For each cell, loads its 4
corners `local_10 = [NW, NE, SE, SW]` (mapping per §1: `iVar5=cy*(W+1)+cx` NW row,
`iVar4=(cy+1)*(W+1)+cx` SW row).

1. **Modified gate:** process only if **any** of the 4 corners has `VISITED(+5) != 0`
   (checks `NW+5`, `NE+0xd`, `SE+0xd`, `SW+5`). Untouched cells are skipped.
2. **Spread gate:** `max − min < 0x10` (i.e. `<= 15`). Since all corner heights are multiples
   of 15, this admits only spreads of `{0, 15}` (a single-level ramp).
3. **Eligibility gate:** identical to §3 — in-diamond, `overlay==−1`, `occupier==0`,
   `scratch+0x45==0`, `(tile≥count || tile==0xFFFF || IsoTileType[tile]+0x2E0 != 0)`.
4. If all gates pass:
   - `cell.level(+0x11b) = min / 15` (the decompiler's
     `((char)(m/15)+(char)(m>>31)) − (char)((longlong)m*0x88888889>>0x3f)` is signed-div-by-15;
     `m ≥ 0` so it reduces to `m/15`, range 0..12).
   - Subtract `min` from each of the 4 corner values → a **[NW,NE,SE,SW]** delta vector in
     `{0,15}`.
   - **Pattern match:** walk all **19** entries of `PTR_DAT_0083ff18` (`ppuVar6` from
     `0x83ff18` while `< 0x83ff64`); each entry is a pointer to an `int[4]`. Compare **in
     order `[NW,NE,SE,SW]`** (`p[0]==Δ0 && p[1]==Δ1 && p[2]==Δ2 && p[3]==Δ3`). On the first
     match at index `s`:
     - `cell.slope(+0x11c) = s`
     - `cell.tile(+0x38) = (s == 0) ? g_ClearTile : g_RampBase + s − 1`
     - break.
   - **No match:** slope/tile are left unchanged (only `level` was written). With spread ≤ 15
     and 15-multiple corners, the delta vector is always one of the 16 `{0,15}⁴` combos, all
     of which appear in the table, so no-match does not occur in practice — but the port must
     replicate "leave slope/tile alone" if it ever does.
5. After the whole pass, free the grid (`FUN_007c8b3d(DAT_00b0b6ec); DAT_00b0b6ec = 0`).

Finalize sets neither `sub_tile(+0x11a)` nor consumes RNG.

---

## 8. Ramp corner table `0x0083FF18` — actual bytes & corner order

Evidence: `read_memory 0x0083ff18 len 304`, `read_memory 0x0083fdd8 len 288`,
`read_memory 0x00b0b6dc len 16`.

**`0x0083FF18` is a POINTER TABLE, not the deltas.** It holds 19 little-endian pointers:

```
[0]  0x00b0b6dc   (BSS, zero-filled → {0,0,0,0})
[1]  0x0083fdd8   [2] 0x0083fde8 … [18] 0x0083fef8   (each +0x10; 4×i32 payload)
```
`(int*)PTR_DAT_0083ff18[slope]` → the `int[4]` delta array; `[0]` is what the build reads
(§1). The finalize dereferences the same pointers (§7).

Actual payloads (little-endian i32, decoded), **order [NW, NE, SE, SW]** (confirmed: build
adds `[0]` to a corner that *is* a cell's NW corner; finalize matches against `[NW,NE,SE,SW]`):

| slope | NW | NE | SE | SW |    | slope | NW | NE | SE | SW |
|------:|---:|---:|---:|---:|----|------:|---:|---:|---:|---:|
| 0  | 0  | 0  | 0  | 0  |    | 10 | 15 | 0  | 15 | 15 |
| 1  | 0  | 15 | 15 | 0  |    | 11 | 15 | 15 | 0  | 15 |
| 2  | 0  | 0  | 15 | 15 |    | 12 | 15 | 15 | 15 | 0  |
| 3  | 15 | 0  | 0  | 15 |    | 13 | 0  | 15 | 30 | 15 |
| 4  | 15 | 15 | 0  | 0  |    | 14 | 15 | 0  | 15 | 30 |
| 5  | 0  | 0  | 15 | 0  |    | 15 | 30 | 15 | 0  | 15 |
| 6  | 0  | 0  | 0  | 15 |    | 16 | 15 | 30 | 15 | 0  |
| 7  | 15 | 0  | 0  | 0  |    | 17 | 0  | 15 | 0  | 15 |
| 8  | 0  | 15 | 0  | 0  |    | 18 | 15 | 0  | 15 | 0  |
| 9  | 0  | 15 | 15 | 15 |    |    |    |    |    |    |

**All 19 entries match the expected values exactly** (entry 0 from the zeroed BSS at
`0x00b0b6dc`, entries 1–18 from `0x0083fdd8`). Units: **15 = one full level** (a corner's rise
per level equals `level*15` in §1; 30 = two levels for the double-ramp slopes 13–16). Corner
ORDER is **[NW, NE, SE, SW]** — *not* a clockwise or NESW variant.

---

## 9. Negative facts / NO-RNG confirmation

- **The corner engine consumes ZERO RNG.** None of `FUN_006b2a70`, `FUN_006b2520`,
  `FUN_005ac230`, `FUN_0058c2a0`, `FUN_006b4100`, `FUN_006b4240`, `FUN_006b3e60`,
  `FUN_006b3a80`, `FUN_006b3850` calls any RNG routine (verified across all decompiles and the
  `0x006b3a80` full disassembly). The only indirect call anywhere in the engine is the undo
  vector's grow method `(*(DAT_00b0b650_vtbl + 8))(...)` — a memory op, not RNG. **All hill RNG
  is upstream in `FUN_005a2f50` (the walk)** and `FUN_005a33f0`, both called *before*
  `FUN_006b2a70`. The engine is a pure deterministic function of the scratch heights + map.
- **No Tiberian-Sun-only behavior is active** in the engine paths, **except** the dormant
  `+6` force-override in §6 (unreachable because the driver never forces and `+6` is never
  written). It is not tunnel/subterranean/fog related.
- The driver's step-count math (§0.1) uses FP (`FILD/FADD/FSUBR/ftol`) but is deterministic;
  it lives in the driver, not the corner engine.
- `IsoTileType+0x2E0` semantics were **corrected** vs the task context (nonzero = *morphable*;
  see §1/§2). The address and `!=0` test are as given; only the English gloss was inverted.

---

## 10. Key offsets & globals

| symbol / offset | value/role | evidence |
|---|---|---|
| `DAT_00b0b6ec` | corner-grid base ptr | 0x006b2a70 |
| grid entry stride | 8 bytes: i32 height +0, u8 locked +4, u8 visited +5, u8 +6 (unused) | 0x006b2a70 / 0x006b3a80 asm |
| `DAT_0087f914` / `DAT_0087f918` | interior W / H (cells) | 0x006b2a70 |
| `DAT_0087f90c` / `DAT_0087f910` | origin x / y | 0x006b2a70 |
| `DAT_008759a4` | grid stride = W+1 | 0x006b2a70 (+ used everywhere) |
| `DAT_008759a0` | grid rows = H+1 | 0x006b2a70 / 0x006b3a80 |
| `DAT_008759a8` / `DAT_008759ac` | origin x / y (working copy) | 0x006b2a70 |
| `DAT_00abed04` / `DAT_00abed08` | diamond min / max bound | 0x005ac230 |
| `DAT_00abed10` | scratch-cell array base (stride 0x50) | 0x0058c2a0 |
| `g_PathfinderLinearMapWidth` (0x0089c2dc) | scratch linear stride | 0x0058c2a0 |
| scratch `+0x00` / `+0x08` / `+0x45` | packed coord / f64 height / water flag | 0x005a35f0 / 0x006b2520 |
| CellClass `+0x24` | packed coord (x,y i16) | all |
| CellClass `+0x38` | tile index (0xFFFF = none) | 0x006b2520 / 0x006b3850 |
| CellClass `+0x44` | overlay (−1 = none) | 0x006b2520 |
| CellClass `+0xe4` | occupier (0 = none) | 0x006b2520 |
| CellClass `+0x11a` / `+0x11b` / `+0x11c` | sub_tile / level / slope | 0x006b3850 |
| `PTR_DAT_0083ff18` | 19 ptrs → int[4] ramp deltas; data @ 0x0083fdd8 (+ entry0 @ 0x00b0b6dc) | §8 |
| `IsoTileType[tile] + 0x2E0` | tile morph-permit byte (≠0 = morphable) | 0x006b2520 |
| `g_ClearTile` / `g_RampBase` | tile idx for slope 0 / base for slopes ≥1 | 0x006b3850 |
| undo: `DAT_00b0b654/658/660/664/65d/650` | base/cap/count/grow/allocated/vector-obj | 0x006b3e60 / 0x006b3a80 asm |
| `FUN_007c8b3d` / `FUN_007c5f00` | free(operator delete) / ftol | 0x006b2a70 / 0x005a3640 |

---

## 11. Implementation handoff (Rust port)

**Data structure.** `struct Corner { height: i32, locked: bool, visited: bool }` in a
row-major `Vec<Corner>` of `(W+1)*(H+1)`, stride `W+1`. Heights are always multiples of 15,
clamped `[0, 180]`. Map coord → grid-local: `(mx-origin_x, my-origin_y)`. Cell `(cx,cy)`
corners: NW=`i`, NE=`i+1`, SW=`i+(W+1)`, SE=`i+(W+1)+1` where `i=cy*(W+1)+cx`.

**Geometry — RESOLVED 2026-07-20** (`decompile_function 0x00565C10` `MapClass__Resize`,
the sole writer of `+0x124..0x130`): `origin = (1, 1)` (`+0x124 = 1`, `+0x128 = 1` written
literally); `W = H = Size.w + Size.h − 1` (`+0x12C = +0x130 = param_2[3]-1+param_2[2]`).
For a generated map `Size = (0, 0, map_w, map_h)`, so **`W = H = map_w + map_h − 1`** (=
`stride − 2`, since `g_PathfinderLinearMapWidth = Size.w + Size.h + 1`). The corner grid is
`(map_w+map_h)²`; a cell at map coord `(mx,my)` maps to grid-local `(mx−1, my−1)`, and every
diamond-band cell (coords in `[1, W]`) has all four corners in range. The diamond bounds
`DAT_00ABED04/08` are also set here: `min = Size.w`, `max = Size.w + 2·Size.h` — matching the
generator's `MapGeometry` (`diamond_min = map_w`, `diamond_max = map_w + 2·map_h`).

**Morphable flag — RESOLVED:** `IsoTileType+0x2E0` is the `[TileSetNNNN] Morphable=` flag
(cross-ref `ALLOWTIBERIUM_THEATER_READER_AND_RUST_SURFACE_GHIDRA_REPORT.md`), already parsed
in the port as `TilesetLookup::is_morphable(tile_id)`. Eligibility clause `(tile≥count ||
tile==0xFFFF || IsoTileType[tile]+0x2E0 != 0)` ⇒ Rust `tile == 0xFFFF || is_morphable(tile)`.

**Implemented** in `src/map/rmg/phases/hills_corners.rs` (Task 12, 2026-07-20; 7 tests) +
`hills.rs::run` orchestration.

**Ramp table.** Bake the 19×4 table from §8 verbatim, order `[NW,NE,SE,SW]`. Corner height
seed = `ramp[slope][NW] + level*15`; finalize matches `(corner − min)` against the table.

**Build (§1).** For each grid point resolve the owner cell via the 4-step
`(0,0),(0,−1),(−1,−1),(−1,0)` in-diamond fallback; seed height from the owner's **NW** delta
+ `level*15`. Lock the corner per the §1 predicate (remember `+0x2E0 != 0` = morphable).

**Morph contract (§0.1 + §5 + §6).** For each scratch cell with a non-zero coord:
`steps = trunc((cell.level + scratch.height_f64) − current_level(cell))`,
`dir = steps>=0 ? +1 : −1`, repeat `abs(steps)` times:
`mask = pick(cell, dir)`; `apply(dir, force=false, coord, mask)`.

- `pick` (§4): `[NW,NE,SE,SW]` bits; all-equal→all unlocked; raise→unlocked `< max`;
  lower→unlocked `> min`. **Filter on `locked`, never `visited`.**
- `apply` (§5): iterate the 2×2 block in order `NW,NE,SW,SE`; remap the picker mask by
  swapping bits 2↔3. Per selected **unlocked** corner: `h += dir*15`; if out of `[0,180]`
  restore (no undo push); else push `{x,y,old}`. Set `visited=true`; run `propagate`. If
  `propagate` fails (and not forced) → LIFO-replay the undo stack (heights only) and return
  failure. A locked selected corner (not forced) also triggers rollback+fail.
- `propagate` (§6): recursive DFS over the 8 neighbors; carry `dir`+`force` unchanged. A
  neighbor movable and `|Δ| > 15` is pulled to `center ∓ 15` (raise pulls-up lower neighbors,
  lower pulls-down higher ones), undo-pushed, visited, and recursed. A **locked** neighbor
  with `|Δ| > 15`, or an **out-of-grid** neighbor, returns failure (when not forced). This is
  the rollback trigger.

**Rollback contract.** One shared undo stack per `apply` call (`Vec<{x,y,old_height}>`),
reset at entry, grown on demand, replayed LIFO on failure restoring `height` only. Because
the driver never forces, every `apply` is all-or-nothing on heights; `visited` marks are
sticky (finalize tolerates this).

**Finalize (§7).** For each cell with any visited corner, spread `≤ 15`, and eligible:
`level = min/15`; match `(corners − min)` `[NW,NE,SE,SW]` against the 19 patterns → `slope`,
`tile = slope==0 ? ClearTile : RampBase + slope − 1`. Leave slope/tile untouched on the
(practically impossible) no-match.

**Determinism.** The whole engine is a pure function of the pre-morph map + scratch heights;
it draws no RNG. Keep the driver's `ftol` step-count truncation exact, but note it is outside
the corner engine proper.
