# RMG map-type 3/4 tail: `0x005A19E0`, `0x005A17F0`, `0x005A1350`

**Date:** 2026-07-25
**Binary:** `gamemd.exe`, image base `00400000`, 10035 functions
(verified via `get_current_program_info`).
**Scope:** the last two calls of the map-type 3 (Inland) / 4 (Mountainous)
"Making regions" block, plus the helper `0x005A1350`.
**Status:** VERIFIED from binary except where the **Unverified** section says otherwise.

---

## 0. Headline correction — this is the CLIFF system, not water

The task framing called `0x005A17F0` / `0x005A1350` the *water re-anchor*. That is
**wrong**, and it matters: these two functions never touch the water tile set.

- `0x005A17F0` and `0x005A1350` gate on the tile-set base stored at **`0x00AA1020`**.
- `g_WaterSet_TileSetBase` is a **different global at `0x00AA0738`**
  (verified via `list_globals name_substring=TileSetBase` → three hits:
  `g_WaterSet_TileSetBase @ 00aa0738`, `g_BridgeSet_TileSetBase @ 00aa0e28`,
  `g_WoodBridgeSet_TileSetBase @ 00abad1c`). The genuine RMG water finalizer
  `0x0059C630` uses `0x00AA0738` throughout and never `0x00AA1020`
  (verified via `decompile_function 0x0059c630`).

`0x00AA1020` is the **theater CLIFF tile-set base**, 40 consecutive tile-type
indices `[base, base+0x28)`. Proof (conclusive, not inferred):

The cliff-face piece selector at `0x00579AC5` (currently mis-labelled
`MapClass__SelectDestroyedBridgeTile_Low`) picks a **1-based** piece number
`p ∈ 1..0x28` and resolves it as
`g_IsometricTileTypeClass_Array[(DAT_00AA1020 + p) - 1]`
(verified via `decompile_function 0x00579ac5`). The variant families it rolls with
`rand % 3` / `rand & 1` are pieces
`{5,6,7} {9,10,11} {12,13,14} {15,16,17} {23,24,25} {35,36,37}` and the pair `{29,30}`.
Converted to 0-based offsets `r = p - 1` that is
`{4,5,6} {8,9,10} {11,12,13} {14,15,16} {22,23,24} {34,35,36}` and `{28,29}` —
**exactly and only** the seven families `0x005A1350` dispatches on
(verified via `read_memory 0x005a17cc` case-index table + `read_memory 0x005a1778`
jump table + `disassemble_function 0x005a1350`).

Applied in Ghidra this session (`set_global 0x00aa1020`):
`g_nCliffSet_TileSetBase`, type `int`, with the proof in its plate comment.

Corrected roles:

| Address | Old label | Verified role | Applied name |
|---|---|---|---|
| `0x005A19E0` | `FUN_005a19e0` | cliff-edge jitter ("cliff drops") | `RandomMapGenerator__JitterCliffEdges` |
| `0x005A17F0` | `FUN_005a17f0` | re-roll adjacent duplicate cliff tiles | `RandomMapGenerator__RerollAdjacentDuplicateCliffTiles` |
| `0x005A1350` | `FUN_005a1350` | pick a *different* piece in the same cliff variant family | `RandomMapGenerator__PickAlternateCliffVariant` |
| `0x004863D0` | `FUN_004863d0` | "is this cell a special terrain tile" | `CellClass__IsSpecialTerrainTile` |
| `0x00AA1020` | `DAT_00aa1020` | cliff tile-set base index (40 entries) | `g_nCliffSet_TileSetBase` |

`save_program` was **not** called — the coordinator owns saving.

### Label drift recorded (not renamed, to avoid colliding with parallel agents)

`0x00579B70` `MapClass__ComputeBridgeAdjacencyMask_Low` is **not bridge code**. It
compares cell LEVEL bytes only and returns a cliff-edge higher-neighbour ring mask
(verified via `disassemble_function 0x00579b70`). A `LABEL DRIFT` plate comment was
added at `0x00579B70`. The same drift extends to `0x00579AC5`
(`MapClass__SelectDestroyedBridgeTile_Low` = the cliff-face piece selector) and to
`MapClass__ApplyBridgeTile` / `MapClass__PlaceBridgeRamp_Low` reached from
`0x00578E60`. Left unrenamed on purpose.

---

## 1. Identity, callers, argument flow

### 1.1 `RandomMapGenerator__JitterCliffEdges` @ `0x005A19E0`

- `void __cdecl f(void)`, no parameters, no return
  (verified via `disassemble_function 0x005a19e0`: `SUB ESP,0x18` … `RET`, no `RET n`).
- Callers (verified via `get_xrefs_to 0x005a19e0`): exactly two —
  `0x00598D6E` in `RandomMapGenerator__Generate`, and `0x005A1E2C` in `FUN_005A1E10`.
- `FUN_005A1E10` (verified via `decompile_function 0x005a1e10`) is the whole map-type
  3/4 tail as a helper:

```c
void __fastcall FUN_005a1e10(int rmgState) {
  if (rmgState[0x3c] == 4 || rmgState[0x3c] == 3) {
    RandomMapGenerator__SplitOversizedRegions();      // 0x0058EBC0
    RandomMapGenerator__BridgeAndConnectorPass();
    RandomMapGenerator__JitterCliffEdges();           // 0x005A19E0
    MapClass__MarkBridgesForRepair_Low(0, 0xffffffff);// 0x00578E60
    RandomMapGenerator__RerollAdjacentDuplicateCliffTiles(); // 0x005A17F0
  }
}
```

This confirms the coordinator's established ordering **and** the `RmgState+0x3C ∈ {3,4}`
gate, from the binary.

### 1.2 `RandomMapGenerator__RerollAdjacentDuplicateCliffTiles` @ `0x005A17F0`

- `void __cdecl f(void)`, no parameters.
- **It really does delegate to `0x005A1350`, from two call sites**, and the delegation is
  a plain `__stdcall` with one stack argument, the cell's current tile-type index:
  - `0x005A1872`: `MOV ECX,[ESP+0x14]` (dead — clobbered by the callee's prologue),
    `PUSH EAX` where `EAX = C->+0x38`, `CALL 0x005A1350`.
  - `0x005A192D`: `PUSH EAX` where `EAX = C->+0x38`, `CALL 0x005A1350`.
  - `0x005A1350` ends `RET 0x4` and reads its argument from `[ESP+0x4]`
    (verified via `disassemble_function 0x005a1350` first instruction
    `MOV EAX,dword ptr [ESP + 0x4]`). So the calling convention is `__stdcall(int tile)`,
    return in `EAX`.

### 1.3 `RandomMapGenerator__PickAlternateCliffVariant` @ `0x005A1350`

- `int __stdcall f(int tileTypeIndex)`.
- Callers: only the two sites above (verified via `decompile_function 0x005a17f0` and the
  disassembly; no other xrefs appear in `get_xrefs_to 0x00aa1020`'s caller set for this
  helper).

---

## 2. Control flow

### 2.1 The neighbour mask primitive `0x00579B70`

Both `0x005A19E0`'s predicates rest on it, so its contract is part of this report.

`uint __stdcall Mask(CellClass *cell)` — `RET 4`
(verified via `disassemble_function 0x00579b70`).

Early return 0 when either:
- the cell is outside the playable diamond
  (`DAT_0087F8DC < Y+X`, `X-Y < DAT_0087F8DC`, `Y-X < DAT_0087F8DC`,
  `Y+X <= DAT_0087F8DC + DAT_0087F8E0*2`), or
- `FUN_0058C2A0(cell.coords)->+0x4A == 0`.
  `FUN_0058C2A0` is the RMG per-cell scratch accessor:
  `DAT_00ABED10 + (Y*g_PathfinderLinearMapWidth + X) * 0x50`
  (verified via `decompile_function 0x0058c2a0`). `+0x4A` is initialised to `1` by the
  record constructor `FUN_0058BDC0` (`*(u8*)(rec+0x4A) = 1`, verified via
  `decompile_function 0x0058bdc0`) and re-set to `1` for every record at the top of
  `0x00578E60`, so in practice it does not veto.

Bit `i` is set when **all three** hold for that neighbour:
1. `neighbour->+0x11B == cell->+0x11B + 4` (signed `char` compare), and
2. `CellClass__IsSpecialTerrainTile(neighbour) == 0`, and
3. the neighbour passes a bounds check
   (`0x00568300` for seven bits; the NW bit uses the inline diamond test instead).

Bit layout — read directly off the explicit `(X±1, Y±1)` index arithmetic
(`idx = Y*0x200 + X`, `g_CellArray_Base + idx*4`), verified via
`disassemble_function 0x00579b70` at `0x00579BFA`…`0x0057A0A0`:

| bit | offset (dx,dy) | compass (+X east, +Y south) | set at |
|---|---|---|---|
| `0x01` | (+1,−1) | NE | `0x00579EFF` |
| `0x02` | (+1, 0) | E  | `0x00579FA8` |
| `0x04` | (+1,+1) | SE | `0x0057A0A0` |
| `0x08` | ( 0,+1) | S  | `0x0057A04D` |
| `0x10` | (−1,+1) | SW | `0x00579FFA` |
| `0x20` | (−1, 0) | W  | `0x00579F55` |
| `0x40` | (−1,−1) | NW | `0x00579E46` |
| `0x80` | ( 0,−1) | N  | `0x00579EA3` |

**The RMG terrain quantum is 4 level units per cliff step.** Three independent
confirmations: the mask compares `+4`; `0x005A19E0` writes `±4`; and
`RandomMapGenerator__StampIsometricTileBlock` computes
`cell->+0x11B = subtile->+0x28 + levelBase - 4` (verified via
`decompile_function 0x005a6c10`).

### 2.2 `0x005A19E0` — cliff-edge jitter

```
MapClass__CellIterator_Init(this = 0x87F7E8)          ; 0x005A19E9 -> 0x00578350
cur = MapClass__CellIterator_Next()                   ; 0x00578290
while (cur) {
    nSW = cur.step(5);  nNE = cur.step(1);  nSE = cur.step(3)
    nE  = cur.step(2);  nW  = cur.step(6)              ; all via 0x00481810
    m = Mask(cur)

    if (m == 0x83) {                                   ; N | NE | E higher
        if (Mask(nSE) == 0x83 && CellClass__IsClearTile(nE)) {
            coin = RNG_COIN()                          ; draw site A, 0x005A1A7B
            if (coin == 0) {
                nE->+0x11B -= 4                        ; 0x005A1AAD
                if (DAT_00ABED10) scratch[nE].+0x38 = scratch[cur].+0x38
            }
        }
    }
    else if (m == 0x38) {                              ; S | SW | W higher
        if (Mask(nSE) == 0x38 && CellClass__IsClearTile(nW)) {
            coin = RNG_COIN()                          ; draw site B, 0x005A1B2C
            if (coin == 0) {
                nW->+0x11B -= 4                        ; 0x005A1B5E
                if (DAT_00ABED10) scratch[nW].+0x38 = scratch[cur].+0x38
            }
        }
    }
    else if (m == 0xE0) {                              ; N | NW | W higher
        if (Mask(nSW) == 0xE0 && Mask(nNE) == 0xE0 && CellClass__IsClearTile(cur)) {
            cur->+0x11B += 4                           ; 0x005A1BF3   NO RNG
            if (DAT_00ABED10) scratch[cur].+0x38 = scratch[nW].+0x38
        }
    }
    cur = MapClass__CellIterator_Next()
}
```

Facts to carry into the port:

- The mask comparisons are **exact equality**, not bit tests
  (`CMP EAX,0x83 / JNZ`, `CMP EAX,0x38 / JNZ`, `CMP EAX,0xE0 / JNZ`). Exactly three
  higher neighbours and no others.
- The fourth quadrant trio `0x0E` (E | SE | S) is **not handled**. The asymmetry is real
  and must be reproduced.
- The `+0x11B` write is **not** gated on the scratch array existing. Once the coin
  (or, for `0xE0`, `IsClearTile`) passes, the level write happens unconditionally; only
  the scratch-id copy is inside `if (DAT_00ABED10 != 0)`
  (verified via `disassemble_function 0x005a19e0` at `0x005A1AAD`/`0x005A1ABC`,
  `0x005A1B5E`/`0x005A1B6D`, `0x005A1BF3`/`0x005A1C02`).
- The cell **modified** is always one of the *higher* neighbours (E is in `0x83`,
  W is in `0x38`), or, in the `0xE0` case, the cell itself raised to plateau level with
  the region id inherited from the W neighbour (also a higher cell). Geometrically
  coherent in all three branches.
- Mutations are visible to later iterations of the same walk — the pass is
  order-dependent.
- No rollback, no rejection loop other than the RNG's own, no early return.

Direction indices: `MapCoord_StepByDir_GetCell(dir)` (`0x00481810`) reads
`g_DirectionOffsets[dir & 7]` at `0x0089F688` as `{dx:i16, dy:i16}` and calls
`MapClass__Get_CellClass`. `dir >= 8` returns without stepping. The mapping used here is
`1=NE(+1,−1)`, `2=E(+1,0)`, `3=SE(+1,+1)`, `5=SW(−1,+1)`, `6=W(−1,0)` — see
§7 for the evidence and its exact confidence level.

### 2.3 `0x005A17F0` — adjacent duplicate cliff-tile re-roll

```
MapClass__CellIterator_Init()
C = Next()
while (C) {
  if (g_nCliffSet_TileSetBase <= C->+0x38 < g_nCliffSet_TileSetBase + 0x28) {   ; signed
     S = C.step(4)                 ; (0,+1)
     E = C.step(2)                 ; (+1,0)

     if (S->+0x38 == C->+0x38 && S->+0x11A < C->+0x11A) {          ; unsigned byte cmp
        nt = PickAlternateCliffVariant(C->+0x38)
        if (nt != C->+0x38) {
            w      = g_IsometricTileTypeClass_Array[C->+0x38]->+0x2E4   ; block width
            origin = ( S.X - S->+0x11A % w , S.Y - S->+0x11A / w )
            id     = DAT_00ABED10 ? scratch[origin].+0x38 : -1
            StampIsometricTileBlock(nt, &origin, id, -1)
        }
     }
     ; C->+0x38 is RE-READ here (0x005A1906) before the second test
     if (E->+0x38 == C->+0x38 && E->+0x11A < C->+0x11A) {   ... same, using E ... }
  }
  C = Next()
}
```

Why the predicate means "duplicate adjacent placement": inside one stamped block the
sub-tile index is `row*w + col` (verified via `decompile_function 0x005a6c10`), so the
S neighbour's sub-index is always `+w` and the E neighbour's `+1` — both strictly
**greater**. `NB->+0x11A < C->+0x11A` is therefore impossible within a single block and
can only be satisfied when `NB` belongs to a **second placement of the same tile type**
butted against `C`'s block. The re-stamp targets `NB`'s block origin, i.e. the
southern/eastern of the two duplicates.

Corollary, useful for the port: `C` can never be inside the block being re-stamped (if it
were, `C` and `NB` would share a block and the sub-index test would have failed). The
S branch **can** however alter the E neighbour before the E branch reads it, and the
tile-index re-read at `0x005A1906` makes that observable. Order S-then-E is load-bearing.

The `nt != C->+0x38` guard only ever rejects the *unhandled* tile offsets — see §3.2.

Argument order for the stamper, read from the stack (verified via
`disassemble_function 0x005a17f0` at `0x005A18F6`–`0x005A1901` and
`0x005A19A9`–`0x005A19B2`): `ECX = nt`, `EDX = &origin`,
`[ESP] = scratchId`, `[ESP+4] = -1`. Matching the stamper's
`__fastcall(int tileTypeIndex, short *origin, int scratchId, int levelBase)`, so
**`levelBase = -1`**.

### 2.4 `0x005A1350` — variant picker

```
r = tile - g_nCliffSet_TileSetBase
if ((unsigned)(r - 4) > 0x20) return tile;                  ; 0x005A1363
idx = byte[0x005A17CC + (r-4)]                              ; 0..20
jmp  dword[0x005A1778 + idx*4]
```

Table contents (verified via `read_memory 0x005a17cc len=33` and
`read_memory 0x005a1778 len=84`):

| `r` | handler | result set | draws |
|---|---|---|---|
| 4 | `0x005A137B` | `base+4+n`, n∈{1,2} → {5,6} | 1 |
| 5 | `0x005A13BB` | `base+4+2n`, n∈{0,1} → {4,6} | 1 |
| 6 | `0x005A13F6` | `base+4+n`, n∈{0,1} → {4,5} | 1 |
| 8 | `0x005A1430` | {9,10} | 1 |
| 9 | `0x005A1470` | {8,10} | 1 |
| 10 | `0x005A14AB` | {8,9} | 1 |
| 11 | `0x005A14E5` | {12,13} | 1 |
| 12 | `0x005A1525` | {11,13} | 1 |
| 13 | `0x005A1560` | {11,12} | 1 |
| 14 | `0x005A159A` | {15,16} | 1 |
| 15 | `0x005A15DA` | {14,16} | 1 |
| 16 | `0x005A1615` | {14,15} | 1 |
| 22 | `0x005A164F` | {23,24} | 1 |
| 23 | `0x005A168F` | {22,24} | 1 |
| 24 | `0x005A16CA` | {22,23} | 1 |
| 28 | `0x005A1704` | `base+29` (deterministic) | **0** |
| 29 | `0x005A1718` | `base+28` (deterministic) | **0** |
| 34 | `0x005A172C` | `FUN_00598030(1,2)` → {35,36} | 1 |
| 35 | `0x005A1738` | `FUN_00598030(0,1)` → `base+34+2n` → {34,36} | 1 |
| 36 | `0x005A1757` | `FUN_00598030(0,1)` → `base+34+n` → {34,35} | 1 |
| 7, 17–21, 25–27, 30–33, and any `r<4` or `r>36` | `0x005A1766` | **input unchanged** | **0** |

(All result sets above are written as 0-based offsets from `g_nCliffSet_TileSetBase`.)

**Every handled case excludes the input piece.** The three-member families are built so
that the "first" member draws `n∈{1,2}`, the "second" draws `2n` and the "third" draws
`n∈{0,1}` — each skipping itself. So a handled call *always* returns a different tile.
This is why `0x005A17F0`'s `nt != C->+0x38` guard is exactly equivalent to "`r` was an
unhandled offset".

---

## 3. Complete RNG draw ledgers

All draws in all three functions use the **map-generator RNG instance at `0x00ABE890`**
(`g_MapGenRng` in the existing corpus; written only by `RandomMapGenerator__Generate` at
`0x0059899B`, every other xref is an RMG function — verified via
`get_xrefs_to 0x00abe890`). The generator itself is a 250-word lagged XOR array with a
"disabled" flag at `+0`, two indices at `+4`/`+8` and state at `+0xC`, returning a full
32-bit word (verified via `decompile_function 0x0065c780`).

### 3.1 The two primitive shapes

Both shapes are `uniform`, never Gaussian. Neither goes near
`RandomMapGenerator__NextGaussian`.

**COIN(0..1)** — inline, e.g. `0x005A1A7B`:
```
retry: r = Random__Next(0x00ABE890)
       v = ftol( (double)r * K2 )        ; K2 = 2.0/(2^32-1)
       if (v > 1) goto retry
```
`K2` read as `0x3E00000000100000` = `(1+2^-32)·2^-31` = `2.0/(2^32-1)`
(verified via `read_memory 0x007ed8b0`). Result is exactly `r >> 31`. Retry fires only
for `r == 0xFFFFFFFF` (the product rounds to exactly `2.0`), i.e. p = 2^-32.

**COIN1(1..2)** — same plus `FADD [0x007E1718]` (`= 1.0`, verified via
`read_memory 0x007e1718`), reject `> 2`. Result is `1 + (r >> 31)`; same 2^-32 retry.

**`FUN_00598030(min,max)`** — the RMG generic uniform (verified via
`disassemble_function 0x00598030`):
```
range = max - min + 1
retry: r = Random__Next(0x00ABE890)
       v = ftol( (double)r * (double)range * K1 + (double)min )   ; K1 = 1.0/(2^32-1)
       if ((unsigned)v > max) goto retry
```
`K1` read as `0x3DF0000000100000` = `(1+2^-32)·2^-32` (verified via
`read_memory 0x007ed898`). For `range = 2` this is **bit-identical** to the two inline
shapes: `(r·2)·K1 ≡ r·K2` exactly (the `r·2` intermediate is exact, and `2·K1 == K2` as
doubles), so `(0,1)` reproduces COIN and `(1,2)` reproduces COIN1. One draw per attempt,
same 2^-32 retry.

### 3.2 `RandomMapGenerator__JitterCliffEdges` @ `0x005A19E0` — **2 draw sites**

| # | site | shape | condition to reach it | consumer |
|---|---|---|---|---|
| A | `0x005A1A7B` | COIN(0..1) | `Mask(cur)==0x83` ∧ `Mask(SE)==0x83` ∧ `IsClearTile(E)` | `if (v==0)` → `E->+0x11B -= 4` + scratch copy |
| B | `0x005A1B2C` | COIN(0..1) | `Mask(cur)==0x38` ∧ `Mask(SE)==0x38` ∧ `IsClearTile(W)` | `if (v==0)` → `W->+0x11B -= 4` + scratch copy |

The `mask == 0xE0` branch consumes **zero** RNG — it is unconditional once its three
mask tests and `IsClearTile(cur)` pass. This matches the coordinator's audit figure of
**2** draw sites, and pins that both are conditional coins, not per-cell draws.

Per full pass the draw count is
`(#cells matching the 0x83 triple) + (#cells matching the 0x38 triple)`, plus 2^-32-rate
retries. Discarded draws: only the rejected `r == 0xFFFFFFFF` attempts.

### 3.3 `RandomMapGenerator__PickAlternateCliffVariant` @ `0x005A1350` — the "N" quantified

**N = exactly 1 draw per call for the 18 RNG-carrying offsets
`{4,5,6, 8,9,10, 11,12,13, 14,15,16, 22,23,24, 34,35,36}`, and exactly 0 draws for
`r = 28`, `r = 29`, and every unhandled `r`** (2^-32-rate retries on top).

It is **per (cell, direction)**, not per region and not per tile family: the function is
called once for the S match and once for the E match, so a single cell contributes 0, 1
or 2 draws. The 18 draw sites are
`0x005A137D, 0x005A13BD, 0x005A13F8, 0x005A1432, 0x005A1472, 0x005A14AD, 0x005A14E7,
0x005A1527, 0x005A1562, 0x005A159C, 0x005A15DC, 0x005A1617, 0x005A1651, 0x005A1691,
0x005A16CC` (inline shapes) plus the three that route through `FUN_00598030` at
`0x005A173F` / `0x005A175E` (reached from `0x005A172C`, `0x005A1738`, `0x005A1757`).

Whole-pass count for `0x005A17F0`:

```
draws = Σ over cells C with cliff tile, over dir ∈ {S, E}:
            1 if (NB.tile == C.tile ∧ NB.sub < C.sub ∧ (C.tile - base) ∈ RNG_SET)
            0 otherwise
```
where `RNG_SET` is the 18-offset set above. Because a re-stamp changes tiles that later
iterations read, the count is not computable up front — it must be simulated in
iteration order.

### 3.4 `RandomMapGenerator__RerollAdjacentDuplicateCliffTiles` @ `0x005A17F0` — 0 own sites

The function contains **no `Random__Next` call of its own**. All of its RNG cost is
`0x005A1350`'s. `RandomMapGenerator__StampIsometricTileBlock` also consumes none
(per its existing plate at `0x005A6C10`, and confirmed by
`decompile_function 0x005a6c10` — no RNG call in the body).

---

## 4. Grid / cell state written, and who consumes it

Cell field meanings used here (all verified via `decompile_function 0x005a6c10`,
`0x005a17f0`, `0x00579b70`):

| offset | type | meaning |
|---|---|---|
| `+0x24` | `i16` | cell X |
| `+0x26` | `i16` | cell Y |
| `+0x38` | `i32` | isometric tile-type index |
| `+0x11A` | `u8` | sub-tile index within the stamped block (`row*w + col`) |
| `+0x11B` | `i8` | cell LEVEL (terrain height); RMG steps it by ±4 |
| `+0x11C` | `u8` | slope / ramp byte |

RMG scratch record: `DAT_00ABED10 + (Y*g_PathfinderLinearMapWidth + X)*0x50`, field
`+0x38` = region / owner id (`int`), field `+0x4A` = per-cell enable flag.
The array is sized `g_PathfinderLinearMapWidth²` records of `0x50` bytes and is
allocated-if-null / freed-if-it-allocated by `0x00578E60`
(verified via `decompile_function 0x00578e60`). Constructor `FUN_0058BDC0` zeroes the
record, sets `+0x40 = -1` and `+0x4A = 1`; `+0x38` starts at `0`.

### 4.1 `0x005A19E0` writes

| what | where | value |
|---|---|---|
| **`+0x11B` (LEVEL)** | E neighbour | `-= 4` |
| **`+0x11B` (LEVEL)** | W neighbour | `-= 4` |
| **`+0x11B` (LEVEL)** | the cell itself | `+= 4` |
| scratch `+0x38` | E / W neighbour, or the cell | copied from the cell / from the W neighbour |

It writes **no** tile index, **no** sub-tile index, **no** overlay, and **does not touch
`+0x11C`**.

**Consumer:** the very next call in the block,
`MapClass__MarkBridgesForRepair_Low(0, -1)` @ `0x00578E60`, which re-derives the terrain
from the level field in two walks (verified via `decompile_function 0x00578e60`):
1. `MapClass__PlaceBridgeRamp_Low(cell, -1)` per cell until it returns 0;
2. for cells with `+0x11C == 0` **and** `CellClass__IsClearTile(cell)`,
   `MapClass__SelectDestroyedBridgeTile_Low(cell, -1)` — the cliff-face piece selector,
   which calls the `0x00579B70` mask (level-difference driven) and finally
   `MapClass__ApplyBridgeTile(0, 0, cell->+0x11B, …)`, passing the cell's LEVEL as the
   `levelBase` that the stamper folds into `+0x11B = subtile->+0x28 + levelBase - 4`.

So the `±4` level edits are consumed one pass later as *which cliff piece gets stamped
where* — that is the whole point of the pass.

### 4.2 `0x005A17F0` writes

Every write goes through `RandomMapGenerator__StampIsometricTileBlock` with
`levelBase = -1`:

| what | value |
|---|---|
| `+0x11A` | `row*w + col` of the new block, per cell |
| `+0x38` | the new tile-type's index (`type[0xA5]`) |
| **`+0x11B`** | **NOT written** — `levelBase == -1` skips the level assignment entirely |
| **`+0x11C`** | `= subtile->+0x2A` of the newly stamped tile, per cell |
| scratch `+0x38` | `= scratchId`, i.e. the origin cell's *pre-existing* region id (preserved) |

**Consumers:** nothing later inside the type-3/4 block re-reads them —
`0x005A17F0` is the last call. Downstream consumers are the general engine:
`+0x38`/`+0x11A` by rendering and by `CellClass__ApplyLAT_and_SlopeFixup`; `+0x11C` by
the slope/ramp half of the same function and by movement; the scratch `+0x38` region id
by whatever RMG bookkeeping runs after `FUN_005A1E10` returns.

### 4.3 On the port's `+0x11C` question

`+0x11C` is **not** written by the cliff-drop pass. It **is** written by
`0x005A17F0`, indirectly, for every cell of each re-stamped block, and the value is
copied **verbatim** from byte `+0x2A` of the per-sub-cell record inside the tile type's
image block (`decompile_function 0x005a6c10`:
`*(u8*)(cell+0x11c) = *(u8*)(sub+0x2a)`). It is a **property of the stamped sub-tile,
read out of the tile asset**, not a value the RMG computes and not a ramp index the RMG
chooses. The only in-RMG semantic constraint proven this session is that
`0x00578E60` treats `+0x11C == 0` as "flat / eligible for a cliff face"
(`decompile_function 0x00578e60`).

This is consistent with the port's own 0..18 ramp-variant index being the *same*
quantity — both are the tile asset's per-sub-cell ramp byte — but see the
**Unverified** section: the concrete numeric range was not verified here, and the
"values 0..4" claim in the pre-existing plate at `0x005A6C10` was **not** independently
confirmed by me.

---

## 5. Player-visible result

**Cliff drop (`0x005A19E0`).** Generated plateaus come out of the region/bridge passes
with long, perfectly straight cliff edges. This pass walks those edges and, on straight
NE-facing and SW-facing runs, **bites a single cell out of the top of the cliff about
half the time** — the plateau edge steps in by one cell and the cliff face re-routes
around it on the next pass. On straight NW-facing runs of three it does the opposite and
**always pushes one cell of plateau out**. Net effect for the player: cliff lines on
Inland and Mountainous maps look hand-drawn and ragged rather than ruler-straight, with
one-cell nibbles and tabs along them. It also moves the affected cell into the
neighbouring region's ownership, so pathing/region bookkeeping follows the new edge.

**Cliff-tile re-roll (`0x005A17F0`).** After the cliff faces are placed, any two
identical cliff tiles that ended up directly adjacent (south or east) are broken up: the
southern/eastern one is swapped for a different piece of the same family. The player sees
a cliff wall whose rock texture does not visibly tile — no repeated identical rock
segment stacked next to itself. Geometry, height and region ownership are unchanged; only
the artwork varies.

---

## 6. Tiberian Sun / dead-branch check

- **No `SpecialFlags` gate, no `FogOfWar` gate, no tunnel/subterranean path** anywhere in
  the three functions (verified via full `disassemble_function` of `0x005A19E0` and
  `0x005A1350`, and `decompile_function 0x005A17F0`).
- The only gate is `RmgState+0x3C ∈ {3,4}` in `FUN_005A1E10`, i.e. map type Inland /
  Mountainous — which is precisely the block the port is missing.
- Inside `0x005A1350`, the "unhandled `r`" arms are **not dead** — they are the live
  no-op path that makes the caller's `nt != tile` guard meaningful.
- The missing `0x0E` (E|SE|S) quadrant in `0x005A19E0` is **not** a disabled branch:
  there is no code for it at all. It is a genuine asymmetry in the algorithm, not TS
  residue, and the port must reproduce it.
- `MapClass__MarkBridgesForRepair_Low`'s tail touches `g_UIModeLock` and
  `DAT_0088098C` (a progress/callback object) — presentation plumbing, outside these
  three functions.

---

## 7. Unverified (YELLOW)

1. **`g_DirectionOffsets` @ `0x0089F688` contents.** The table is runtime-initialised:
   `read_memory 0x0089f688 len=32` returns all zeros, and **`get_xrefs_to 0x0089f688` /
   `0x0089f68c` return 140+ references with ZERO `WRITE` entries** — the initialiser was
   not located this session. (Negative result worth keeping:
   `search_byte_patterns c70588f68900` and a search for the literal
   `{(0,-1),(1,-1),(1,0),…}` table both returned no matches.) The mapping used in this
   report — `0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW` with the obvious unit offsets —
   rests on three converging arguments, none of which is a direct read of the table:
   - `BuildingClass__ConnectWalls` (`decompile_function 0x00452ad4`) walks
     `dir = 0,2,4,6` in lockstep with a 4-entry table at `0x00818CA0` whose contents are
     `{1,2,4,8}` (`read_memory 0x00818ca0`), matching the RA2 wall-frame N/E/S/W bit
     order.
   - the `0x00579B70` bit ring, derived from explicit arithmetic, is exactly
     `bit i ↔ direction (i+1) & 7` under that mapping.
   - the three `0x005A19E0` predicates are only geometrically coherent under it (the
     partner cell always lies *along* the cliff edge implied by the mask, and the
     modified cell is always one of the mask's higher neighbours).
   Confidence: HIGH-inferred, not VERIFIED-read. **Re-verify before the port hard-codes
   offsets.**
2. **Which theater INI key binds `g_nCliffSet_TileSetBase`.** Writers are only
   `Read_Theater_TileSets_INI` (`get_xrefs_to 0x00aa1020`): bulk reset to `-1` at
   `0x00545A8C` and `0x00546CA1`, per-key assignment at `0x00545DF9` guarded by
   `CMP EDI,[ESP+0xF8]`. The stack slot's originating key string was not traced. The
   *role* (40-entry cliff piece set) is proven independently — see §0.
3. **`+0x11C` numeric range and per-value meaning.** Proven: it is copied verbatim from
   `subtile->+0x2A`, and `0` means "flat / cliff-face eligible" to `0x00578E60`. The
   "values 0..4" statement in the pre-existing plate at `0x005A6C10` was **not**
   confirmed by me; the tile-asset parser that produces `+0x2A` was not opened.
4. **`0x005A17F0` has no `-1` guard on `g_nCliffSet_TileSetBase`.** If a theater defines
   no cliff set the base is `-1` and the range test becomes `-1 ≤ tile < 0x27`, which
   would admit ordinary low tile indices. Whether any stock YR theater leaves the cliff
   set undefined was not checked. `CellClass__IsSpecialTerrainTile` *does* guard `!= -1`,
   which is what makes the omission conspicuous.
5. **`RandomMapGenerator__Generate`'s own call at `0x00598D6E`.** Taken as established by
   the coordinator to sit inside the type-3/4 "Making regions" block; I verified the
   equivalent gate only in `FUN_005A1E10`, not at `0x00598D6E` itself.
6. **`FUN_00598030`'s argument range in `0x00579AC5`.** Not read; that function belongs to
   another agent's scope. It does not affect any ledger in this report.

---

## 8. Ghidra annotations applied this session

Renames (`rename_function_by_address`): `0x005A19E0`, `0x005A17F0`, `0x005A1350`,
`0x004863D0`. Global (`set_global`): `0x00AA1020` → `g_nCliffSet_TileSetBase : int`.
Plate comments (`set_plate_comment`): `0x005A19E0`, `0x005A17F0`, `0x005A1350`,
`0x004863D0`, and a `LABEL DRIFT` note on `0x00579B70`. All five plate targets were
confirmed empty first via `get_plate_comment`. **`save_program` deliberately not called.**

## 9. Equivalence check for the implementation handoff

Both draw ledgers are finite and fully enumerated, so the port can be certified by
**exhaustive vector comparison on `0x005A1350`**: its input domain is the 41 values
`r ∈ [-1, 0x27]` crossed with the two RNG outcomes, which is small enough that exhaustive
vectors constitute proof, not sampling. `emulate_function 0x005A1350` over that domain is
the named check to build.

`0x005A19E0` and `0x005A17F0` are stateful whole-map walks; they certify only against a
gamemd-derived trace — a per-call log of `(cell, mask, draw index, value, field written)`
diffed against the Rust pass. Until that instrument exists their status is
**UNVERIFIED-pending-instrument**, not "matches".
