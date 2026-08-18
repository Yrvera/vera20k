# RMG map-type 3/4 — `0x00578E60` cliff-level + cliff-face fixup (Ghidra report)

**Date:** 2026-07-25
**Binary:** `gamemd.exe`, image base `00400000` (verified: `get_current_program_info`)
**Scope:** close the last RNG consumers in the map-type-3/4 region/bridge block so the
generator stream is reproducible.

**Headline:** the block's remaining gap is *not* bridge code. `0x00578E60` and both of its
inner passes are **cliff** code. The whole function draws **exactly one** RNG value per
qualifying cell in its second pass, and **zero** in its first.

---

## 0. Label drift resolved (renamed + plated this session)

| Address | Old name (drifted) | Proven role → new name | Proving call |
|---|---|---|---|
| `0x00578E60` | `MapClass__MarkBridgesForRepair_Low` | `MapClass__RmgFixupCliffLevelsAndFaces` | `disassemble_function 0x00578E60` |
| `0x00579010` | `MapClass__PlaceBridgeRamp_Low` | `MapClass__RaisePinchedCliffCell` | `disassemble_function 0x00579010` |
| `0x00579620` | `MapClass__SelectDestroyedBridgeTile_Low` | `MapClass__SelectAndStampCliffFaceTile` | `disassemble_function 0x00579620` |
| `0x0057B440` | `MapClass__ApplyBridgeTile` | `MapClass__StampPendingIsoTileBlock` | `disassemble_function 0x0057B440` |
| `0x00598030` | `RandomMapGenerator__NextUniformRange` | `RandomMapGenerator__RandomRangeInclusive` | `disassemble_function 0x00598030` |
| `0x00880990` | `g_UIModeLock` | `g_pMapPendingIsoTileType` (= `MapClass+0x11A8`) | `decompile_function 0x004A91B0` |
| `0x0089F6A0` | `_g_refinery_unload_adjacent_lookup_dx` | `g_dwDirectionOffset6W` (element 6 of `g_DirectionOffsets`) | `decompile_function 0x0049F2F0` |
| `0x00ABDDA4` | (unnamed) | `g_dwCliffPieceAnchorOffsets[41]` | `get_xrefs_to 0x00ABDDA8` |
| `0x0082A7F4` | (unnamed) | `g_nShorePieceGroupTable[42]` | `read_memory 0x0082A7F4` |
| `0x0082A89C` | (unnamed) | `g_nShorePieceOrientTable[42]` | `read_memory 0x0082A89C` |
| `0x00579320` | (unrecognised code) | label `CliffPieceAnchorOffsets_StaticInit` | `disassemble_bytes 0x00579311-0x00579620` |

`0x00579AC5` is **not a function** — it is the shared stamp tail *inside* `0x00579620`
(the target of every `JMP 0x00579AC5` in the selector). `0x00579B70` is the cliff
higher-neighbour ring mask, already plated by an earlier pass this session; its bit layout
is re-verified independently below.

**Evidence that this is cliff, not bridge, code:** the tile the selector resolves comes
from `g_nCliffSet_TileSetBase` (`0x00AA1020`, read at `0x00579ACA`). The other readers of
that global are `CellClass__IsSpecialTerrainTile`, `IsOnBridgeRamp`,
`RandomMapGenerator__RerollAdjacentDuplicateCliffTiles` and
`RandomMapGenerator__PickAlternateCliffVariant` — all cliff code (`get_xrefs_to
0x00AA1020`). The real bridge RNG consumers are separate functions:
`MapClass__SelectBridgeTileVariant_Low` `0x0057ACF0` and the four
`MapClass__RepairBridgeWalker_*` at `0x0057F8CF / 0x0057FDEA / 0x00580306 / 0x00580831`
(`get_xrefs_to 0x00598030`).

---

## 1. `0x00578E60` — full contract

`__thiscall(this = g_Map 0x0087F7E8)(int unused, int zoneFilter)`, `RET 8`
(`disassemble_function 0x00578E60`).

### 1.1 Call site

`disassemble_bytes 0x00598D50-0x00598D8F` gives the type-3/4 block verbatim:

```
00598D55  MOV EAX,[ESI+0x3C]        ; map type
00598D58  CMP EAX,4 ; JZ  0x00598D62
00598D5D  CMP EAX,3 ; JNZ 0x00598D87 ; <- types 3 and 4 only
00598D62  CALL 0x0058EBC0            ; RandomMapGenerator__SplitOversizedRegions
00598D67  CALL 0x0058EF10            ; RandomMapGenerator__BridgeAndConnectorPass
00598D6E  CALL 0x005A19E0            ; RandomMapGenerator__JitterCliffEdges
00598D73  PUSH -1 ; PUSH EBX ; MOV ECX,0x87F7E8
00598D7B  CALL 0x00578E60            ; <- THIS
00598D82  CALL 0x005A17F0            ; RandomMapGenerator__RerollAdjacentDuplicateCliffTiles
```

Note `0x0058EF10` sits between `0x0058EBC0` and `0x005A19E0` — the handoff brief listed
four calls, there are five.

`FUN_005A1E10` is a byte-for-byte out-of-line twin of the same block
(`decompile_function 0x005A1E10`) with **zero xrefs** (`get_xrefs_to 0x005A1E10` →
"No references found"), i.e. the copy the compiler inlined into
`RandomMapGenerator__Generate`. It is dead but corroborates the ordering.

### 1.2 Arguments

- **`param_1` (stack `[ESP+0x1C]`) is never read.** The complete disassembly touches only
  `[ESP+0x13]`, `[ESP+0x14]`, `[ESP+0x20]`. Dead argument.
- **`param_2` (`[ESP+0x20]`, loaded to `EDI` at `0x00578F2F`) = `0xFFFFFFFF`** — the RMG
  scratch-grid *zone filter*. `-1` disables every zone gate downstream (see §3.2, §5.4).

### 1.3 Body, in order

1. **Scratch-grid ensure.** If `DAT_00ABED10 == 0`, `operator_new(W*W*0x50)` and run the
   record ctor `FUN_0058BDC0` on each (`W = g_PathfinderLinearMapWidth` `0x0089C2DC`);
   remember that we allocated so we free at the end. In the normal type-3/4 path the grid
   already exists (the region pass allocated it), so **no alloc and no free happens**.
   Record stride `0x50`; `+0x38` = int zone id (ctor writes 0), `+0x4A` = byte gate
   (ctor writes 1) (`decompile_function 0x0058BDC0`; note the ctor's `undefined2*`
   arithmetic — `param_1 + 0x1C` = byte `0x38`, `param_1 + 0x25` = byte `0x4A`).
2. **Gate reset.** `for i in 0..W*W: FUN_0058C2C0(i)->+0x4A = 1`. `FUN_0058C2C0(i)` =
   `DAT_00ABED10 + i*0x50` (`decompile_function 0x0058C2C0`). `+0x4A` is the "this cell
   participates in the cliff mask" flag read by `0x00579B70`.
3. **`FUN_004A8BF0(this, 0)`** — clears the placement-cursor shape (`Map+0x117C = 0`)
   (`decompile_function 0x004A8BF0`). This is load-bearing: it makes the later
   `Set_Cursor_Position` call degenerate to a plain field write (§5.3).
4. **PASS 1** — reset the diamond iterator, sweep, call `MapClass__RaisePinchedCliffCell`
   on **every** cell.
5. **PASS 2** — reset the iterator again, sweep, call
   `MapClass__SelectAndStampCliffFaceTile` on cells with
   `cell->+0x11C == 0 && CellClass__IsClearTile(cell)`.
6. **Teardown** — `Map+0x11A8 = 0`; if `Map+0x11A4` (`0x0088098C`) `!= 0`, call its
   `vtable+0x20` with `1` and null it; free the scratch grid iff step 1 allocated it.

### 1.4 Iterator

Both passes reset `MapClass` fields and drive `MapClass__CellIterator_Next` `0x00578290`:

```
Map+0x110 = DAT_0087F8DC              ; y
Map+0x10C = 1                         ; x
Map+0x114 = DAT_0087F8DC - 1          ; run length
Map+0x118 = g_CellArray_Base + (DAT_0087F8DC*512 + 1)*4
```

(`0x0087F8F8 = Map+0x110`, `0x0087F8F4 = Map+0x10C`, `0x0087F8FC = Map+0x114`,
`0x0087F900 = Map+0x118`; `0x0087F7E8 + 0xF4 = 0x0087F8DC`, so `Map+0xF4` *is*
`DAT_0087F8DC`). Each step advances `-0x1FF` dwords (`x+1, y-1`) along a NE diagonal and
starts a new diagonal on run exhaustion — the standard playable-diamond walk
(`decompile_function 0x00578290`). The cell array is a **pointer table** of `0x40000`
entries at `[0x0087F924]`, indexed `y*0x200 + x`.

### 1.5 Loop-abort semantics

```
BL = 1
cell = Next()
while (cell != 0) {
    if (BL == 0) break;              ; abort-on-first-failure
    ... BL = InnerPass(cell, zoneFilter) ...
    cell = Next();
}
```
Both passes share this shape (`0x00578F37` and `0x00578F91`). PASS 1 can never set `BL=0`
with `zoneFilter = -1` (§3.2). PASS 2 can (§5.4).

---

## 2. Complete RNG draw ledger

### 2.1 Proof of the ledger's completeness

`get_xrefs_to 0x0065C780` (`Random__Next`, full 400-row listing) contains **no** entry for
any function in this subtree: `0x00578E60`, `0x00579010`, `0x00579B70`, `0x0057B440`,
`0x00481810`, `0x005A00C0`, `0x005A0090`, `0x0058BDC0`, `0x0058C2A0`, `0x0058C2C0`,
`0x00578290`, `0x004A8BF0`, `0x004A91B0`, `0x00486380`, `0x004863D0`, `0x004865B0`,
`0x00578D80`, `0x0042D470`. The single hit inside the subtree is
`0x00598063 in RandomMapGenerator__RandomRangeInclusive`.

The two other RNG wrappers are not reached either: `get_xrefs_to 0x00598000`
(`RandomMapGenerator__NextUniform01`) returns only two DATA references; `get_xrefs_to
0x005980C0` (the Gaussian) lists 22 callers, none in this subtree.

`get_xrefs_to 0x00598030` lists 11 callers; exactly one — `0x00579630` — is in this
subtree.

### 2.2 The ledger

| Stage | Draws | Depends on |
|---|---|---|
| Scratch-grid alloc + ctor + `+0x4A` reset | **0** | — |
| `FUN_004A8BF0(0)` cursor-shape clear | **0** | — |
| **PASS 1** — `MapClass__RaisePinchedCliffCell` over the whole diamond, incl. its unbounded 8-way recursion | **0** | — |
| **PASS 2** — `MapClass__SelectAndStampCliffFaceTile` | **1 × `RandomRangeInclusive(0,5)` per invocation** | number of cells the iterator reaches with `+0x11C == 0` **and** `+0x38 ∈ {0, 0xFFFF}`, in iterator order, up to and including the first invocation that returns 0 |
| Teardown | **0** | — |

The PASS 2 draw is taken at the **first instruction** of `0x00579620`
(`0x00579629 MOV EDX,5 / XOR ECX,ECX / CALL 0x00598030`), **unconditionally and before any
predicate** — including before the mask is computed. So a cell that ends up selecting no
piece at all still consumes one draw, and the aborting invocation consumes one too.

Draws are **per cell**, never per ramp or per cliff face: the selector computes at most
four extra neighbour masks but never redraws.

### 2.3 `RandomRangeInclusive` exact contract (`0x00598030`)

`__fastcall(ECX = lo, EDX = hi) -> EAX`, no stack args, plain `RET`.

```
span = hi - lo + 1
do {
    raw = Random__Next(0x00ABE890)                       ; u32, zero-extended before FILD
    r   = ftol( (double)raw * (double)span * K + (double)lo )
} while ((unsigned)r > (unsigned)hi)                     ; JA — unsigned
return r
```

- `K` = the double at `0x007ED898` = `00 00 10 00 00 00 F0 3D` = `0x3DF0000000100000`
  = `(1 + 2⁻³²)·2⁻³²`, the nearest double to `1/(2³²−1)` (`read_memory 0x007ED898`).
- `Math__ftol` `0x007C5F00` **truncates toward zero**.
- **Rejection sampling, inclusive of both ends** — not modulo, not exclusive.
- For the only call in this subtree, `lo = 0, hi = 5`: the sole rejected `raw` is
  `0xFFFFFFFF`. Product `= 6·(2⁶⁴−1)/2⁶⁴ = 6 − 6·2⁻⁶⁴`, which rounds to exactly `6.0` in
  double (ulp near 6 is `2⁻⁵⁰`), so `ftol → 6 > 5 → redraw`. `raw = 0xFFFFFFFE` gives
  `6 − 6·2⁻³²`, ~1.4e-9 below 6 — far more than one ulp — so it truncates to 5 and is
  accepted. **Probability of a second draw = 2⁻³².**

`Random__Next` `0x0065C780` is the 250-lag Fibonacci XOR on `0x00ABE890`
(`state[i] ^= state[j]`, both indices reset to 0 past `0xF9`), with an early
`if (*this != 0) return 0` disable flag at offset 0 (`decompile_function 0x0065C780`).

### 2.4 How the single roll is spent

`r ∈ [0,5]` is consumed as `r % 3` (via `CDQ/IDIV 3`, remainder in `EDX`; `r ≥ 0` so
remainder `∈ {0,1,2}`) and as `r & 1`. Over `r = 0..5` the pair `(r%3, r&1)` hits each of
the six combinations exactly once — one `[0,5]` draw is a joint uniform over a 3-way and a
2-way variant choice. A Rust port must draw **one** value and derive both, not two.

---

## 3. `0x00579010` — `MapClass__RaisePinchedCliffCell`

`__thiscall(this = g_Map)(CellClass *cell, int zoneFilter)`, `RET 8`. Returns 1 always
except the zone early-out.

### 3.1 What it decides

`m = Mask(cell)` from `0x00579B70` (§4). Set the raise flag if any of:

| Condition | Meaning |
|---|---|
| `(m & 0xA0) == 0xA0 && (m & 0x11) == 0x11` | N+W and NE+SW higher |
| `(m & 0x20) && (Mask(cell E) & 0x02)` | pinched along X (`0x0057904A`) |
| `(m & 0x02) && (Mask(cell W) & 0x20)` | pinched along X (`0x005790A8`) |
| `(m & 0x08) && (Mask(cell N) & 0x80)` | pinched along Y (`0x00579106`) |
| `(m & 0x80) && (Mask(cell S) & 0x08)` | pinched along Y (`0x00579163`) |
| `(m & 0x11) == 0x11 && ((m & 0x0A) != 0x0A \|\| (m & 0xE0)) && ((m & 0xA0) != 0xA0 \|\| (m & 0x0E))` | NE/SW diagonal pinch |
| `(m & 0x44) == 0x44 && ((m & 0x28) != 0x28 \|\| (m & 0x83)) && ((m & 0x82) != 0x82 \|\| (m & 0x38))` | SE/NW diagonal pinch |
| `(m & 0x2C)==0x24 \|\| (m & 0xA1)==0x21 \|\| (m & 0x1A)==0x12 \|\| (m & 0xC2)==0x42 \|\| (m & 0x0B)==0x09 \|\| (m & 0x68)==0x48 \|\| (m & 0x86)==0x84 \|\| (m & 0xB0)==0x90` | eight corner-with-missing-link patterns |

plus the two unconditional triggers `(m & 0x88) == 0x88` (N+S higher) and
`(m & 0x22) == 0x22` (E+W higher).

The four neighbour probes are "am I in a one-cell-wide low channel": if this cell has
higher ground to the W, look at the cell to the E and ask whether *it* has higher ground
to *its* E.

### 3.2 What it does when it fires

```
zone = FUN_005A00C0(coord)                        ; scratch record +0x38
if (zone > 0 && zone != zoneFilter && zoneFilter != -1) return 0;   ; DEAD (zoneFilter is always -1)
cell->+0x11B += 4                                 ; ONE terrain step up — the only field written
FUN_005A0090(coord, zoneFilter)                   ; scratch record +0x38 := -1
for dir in 0..8: recurse(MapCoord_StepByDir_GetCell(cell, dir), zoneFilter)
```

The `return 0` at `0x005792BA` requires `zoneFilter != -1`; the RMG driver always passes
`-1`, so **PASS 1 never aborts the sweep and returns 1 for every cell**.

Termination: raising the cell breaks the exact-`+4` relation that set the bits, so the
flood is finite.

**Naming note.** There is no ramp here — no sub-tile is chosen, no tile is placed, no
`+0x38`/`+0x11A`/`+0x11C` is written. "Ramp" in the old label was wrong. **PASS 1 places
nothing and draws nothing**; it is purely a level-repair flood, and its only observable
output is the `+0x11B` byte.

**Codegen trap for readers:** at `0x005792EE` the compiler pushes the *recursive* call's
`zoneFilter` argument **before** the `0x00481810` call, so the two pushes preceding
`CALL 0x00481810` are not both its arguments (`0x00481810` is `RET 4`).

---

## 4. `0x00579B70` — the higher-neighbour ring mask

`__stdcall(CellClass *cell)`, `RET 4`. Returns 0 when the cell is outside the playable
diamond or its scratch record's `+0x4A` is 0. Otherwise bit *i* is set iff the neighbour
satisfies **both**

- `neighbour->+0x11B == cell->+0x11B + 4` (exactly one terrain step higher), and
- `CellClass__IsSpecialTerrainTile(neighbour) == 0`.

Bit layout re-derived from the explicit index arithmetic in `decompile_function
0x00579B70` (each local paired with the bit that reads it):

| Bit | Offset | Dir |
|---|---|---|
| `0x01` | `(+1,−1)` | NE |
| `0x02` | `(+1, 0)` | E |
| `0x04` | `(+1,+1)` | SE |
| `0x08` | `( 0,+1)` | S |
| `0x10` | `(−1,+1)` | SW |
| `0x20` | `(−1, 0)` | W |
| `0x40` | `(−1,−1)` | NW |
| `0x80` | `( 0,−1)` | N |

i.e. mask bit *i* = direction `(i+1) mod 8` in the standard `0=N` clockwise order — a
rotate-by-one of `g_DirectionOffsets` (§7). Compass names follow the project convention
`+X = east, +Y = south`.

`CellClass__IsSpecialTerrainTile` `0x004863D0` tests `cell->+0x38` against a list of
theater tile-set ranges (cliff set + `0x28`, four 4-tile ramp sets with sub-tile
exceptions, bridge sets, etc.); each base is `−1` when the theater omits that set, which
disables its test.

---

## 5. `0x00579620` — `MapClass__SelectAndStampCliffFaceTile`

`__thiscall(this = g_Map)(CellClass *cell, int zoneFilter)`, `RET 8`.

### 5.1 Piece selection (piece index 1..0x28)

`r = RandomRangeInclusive(0,5)` first, then `m = Mask(cell)`:

```
(m & 0xA0)==0xA0 : a = Mask(cell S), b = Mask(cell E)
                   (a & 0x20) || (b & 0x80)  ->  0x22    else  (r%3)+9
(m & 0x82)==0x82 : 0x27
(m & 0x0A)==0x0A : 0x21
(m & 0x28)==0x28 : 0x28
else if (m & 0x02): c = Mask(cell S), d = Mask(cell SE)
    (m&4) && !(m&0x18) && (c&0x0A)!=0x0A       -> (r%3)+0x23
    else !(m&0x0C) && !(d&2) && (r&1)          -> 0x01
    else                                        -> 0x26
else if (m & 0x20): e = Mask(cell S)
    (m&0x10) && !(m&0x0C) && (e&0x28)!=0x28    -> (r%3)+0x0F
    else                                        -> 0x12
else if (m & 0x08): f = Mask(cell E), g = Mask(cell SE)
    (m&4) && !(m&3) && (f&0x0A)!=0x0A          -> (r%3)+0x17
    else !(m&6) && !(g&8) && (r&1)             -> 0x15
    else                                        -> 0x1A
else if (m & 0x80): h = Mask(cell E)
    (m&1) && !(m&6) && (h&0x82)!=0x82          -> (r%3)+0x05
    else                                        -> 0x08
else if (m & 0x01): 0x02
else if (m & 0x04): (r&1)+0x1D
else if (m & 0x10): 0x16
else if (m & 0x40): (r%3)+0x0C
else              : no stamp, return 1
finally           : if (piece <= 0) return 1            ; 0x00579ABD
```

Producible pieces: `1, 2, 5..0x12, 0x15..0x1A, 0x1D, 0x1E, 0x21..0x28`.
Never produced: `3, 4, 0x13, 0x14, 0x1B, 0x1C, 0x1F, 0x20`.

The two `if (piece == -1)` tests at `0x00579719` and `0x005798E8` are **unreachable** —
their operands are `≥ 1` by construction. Compiler sentinel, not behaviour.

### 5.2 Draw count is one — even for four-mask cells

The branches that compute up to two extra neighbour masks (`a`/`b`, `c`/`d`, `f`/`g`) use
the same `r`. There is no second `CALL 0x00598030` anywhere in the function
(`disassemble_function 0x00579620`).

### 5.3 Stamp tail `0x00579AC5`

```
tileType = (*(void***)0x00A8ED2C)[ g_nCliffSet_TileSetBase + piece - 1 ]
Map+0x11A8 (0x00880990) = tileType
anchor   = cell->coord + (short)[0x00ABDDA4 + piece*4], (short)[0x00ABDDA6 + piece*4]
FUN_004A91B0(g_Map, &out, &anchor)                     ; Set_Cursor_Position
flag = 1
ok = MapClass__StampPendingIsoTileBlock(0, 0, cell->+0x11B, zoneFilter, &flag, 0)
return (ok || flag) ? 1 : 0
```

- `0x00A8ED2C` holds a **pointer to** the `IsometricTileTypeClass*` array, indexed
  `base + piece - 1`.
- **The anchor offset is always `(0,0)` for every reachable piece.**
  `g_dwCliffPieceAnchorOffsets` is zero in the file image (`read_memory 0x00ABDDA4 len
  164`) and its only writer is the static initializer at `0x00579320`
  (`get_xrefs_to 0x00ABDDA8` → `WRITE from 0057934E`), which builds every entry from
  `MapCoord_Set(0,0)` except call #24, which pushes `(x=1, y=0)` and stores to
  `0x00ABDE24` = index `0x20` — and `0x20` is not producible. So the tile is stamped at the
  cell's own coordinate. (`MapCoord_Set` `0x0042D470` writes `x` at `+0`, `y` at `+2`.)
- `FUN_004A91B0` is `DisplayClass::Set_Cursor_Position`. Because step 3 of `0x00578E60`
  cleared `Map+0x117C`, it takes the early branch: `prev = Map+0x1174; Map+0x1174 = anchor`.
  That field (`= 0x0088095C`) is exactly what the stamper reads as the block origin —
  the coordinate is passed through a `MapClass` field, not an argument.
- `0x00579AC5` sets a **1-byte, `1`-initialised in/out flag** in the incoming `param_1`
  stack slot (`MOV byte ptr [ESP+0x40],1` at `0x00579B3A` targets `orig+0x04`, the same
  byte read back at `0x00579B48`). The anchor coordinate itself is copied to a separate
  local before that overwrite.

### 5.4 When PASS 2 aborts

`MapClass__StampPendingIsoTileBlock` returns 0 on many routes, but only **two** of them
clear `*flag`, and only those abort the sweep:

- `0x0057B724` — the target cell's scratch zone id is already `−1` (i.e. this pass stamped
  it), **and** both the existing and incoming tile indices fall in
  `[g_ShorePieces, +0x2A)`, **and** `3 ≤ |g_nShorePieceOrientTable[a] −
  g_nShorePieceOrientTable[b]| ≤ 5`.
- `0x0057B77D` — the target cell is not clear and either
  (`IsShorePieceTile(cell) && IsOnBridgeRamp(newTileIdx, subIdx)`) or
  (`newTileIdx ∈ [g_ShorePieces, +0x2A)` and `IsSpecialTerrainTile(cell)`).

Every other failure (`vtable+0x2C != 0x12`, null image block, refused sub-cell) returns 0
with `*flag` still 1, so the selector returns 1 and **the sweep continues**.

---

## 6. Grid / `CellClass` fields written, and who consumes them

| Field | Written by | Value | Consumed by |
|---|---|---|---|
| `+0x11B` (byte, cell level) | `0x005792D3` (PASS 1) | `+= 4` (one terrain quantum) | `0x00579B70`'s `== level+4` test on the very next mask evaluation; `RandomMapGenerator__JitterCliffEdges` `0x005A19E0` writes it in ±4 units; passed as `levelBase` into the stamper by `0x00579B20` |
| `+0x11B` (byte) | `0x0057B66C` (stamper) | `subTile->+0x28 + levelBase` | rendering; the mask on later cells |
| `+0x38` (int, iso tile index) | `0x0057B65E` (stamper) | `tileType->+0x294` | `CellClass__IsClearTile` (the PASS 2 gate), `CellClass__IsShorePieceTile`, `CellClass__IsSpecialTerrainTile`, `RandomMapGenerator__RerollAdjacentDuplicateCliffTiles` `0x005A17F0` |
| `+0x11A` (byte, sub-tile index) | `0x0057B661` (stamper) | `Width*j + i` | `CellClass__IsSpecialTerrainTile`'s ramp sub-tile exceptions; `0x005A17F0` |
| `+0x11C` | **not written anywhere in this subtree** | — | **read** as the PASS 2 gate (`0x00578F95`); written elsewhere (the brief's `+0x2A`-of-subtile copy is in `0x005A17F0` / `0x005A1350`, not here) |
| scratch `+0x38` (zone id) | `FUN_005A0090` from PASS 1 (`0x005792E7`) and the stamper (`0x0057B623`, `0x0057B6CC`) | `zoneFilter` = `−1` | `FUN_005A00C0` in PASS 1's early-out (dead) and in the stamper's "already stamped this pass" test |
| scratch `+0x4A` (byte gate) | `0x00578EE2` | `1` for every record | `0x00579B70` — returns mask 0 when clear |
| `Map+0x1174` | `FUN_004A91B0` | the stamp anchor | the stamper's origin (`0x0088095C`) |
| `Map+0x11A8` | `0x00579AD6`, cleared at `0x00578FCD` | the chosen tile type | the stamper's entry read |

---

## 7. Bonus — `g_DirectionOffsets` `0x0089F688` mapping is now **PROVEN**, not inferred

`0x0049F2F0` **is** the initializer for `0x0089F688`. `read_memory 0x0089F688 len 32`
returns all zeros (file image), and `decompile_function 0x0049F2F0` stores exactly eight
dwords into `0x0089F688..0x0089F6A4`. Decoding each as `(short dx @+0, short dy @+2)`:

| Index | Stored constant | `(dx, dy)` | Direction |
|---|---|---|---|
| 0 | `0xFFFF0000` | `( 0, −1)` | N |
| 1 | `0xFFFF0001` | `( 1, −1)` | NE |
| 2 | `0x00000001` | `( 1,  0)` | E |
| 3 | `0x00010001` | `( 1,  1)` | SE |
| 4 | `0x00010000` | `( 0,  1)` | S |
| 5 | `0x0001FFFF` | `(−1,  1)` | SW |
| 6 | `0x0000FFFF` | `(−1,  0)` | W |
| 7 | `0xFFFFFFFF` | `(−1, −1)` | NW |

So `0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW` under `+X = east, +Y = south`. This
matches the previously inferred mapping; it is now backed by the stored constants rather
than by consumer inference. `MapCoord_StepByDir_GetCell` `0x00481810` indexes it with
`dir & 7` and returns without stepping for `dir ≥ 8`.

Element 6 (`0x0089F6A0`) carried the unrelated stale name
`_g_refinery_unload_adjacent_lookup_dx`; renamed to `g_dwDirectionOffset6W` this session.

---

## 8. Tiberian Sun / dead-code check

| Item | Status | Gating evidence |
|---|---|---|
| `param_1` of `0x00578E60` | **Dead argument** — never read | full `disassemble_function 0x00578E60`: no `[ESP+0x1C]` access |
| PASS 1's zone early-out (`0x005792BA`) | **Dead on the RMG path** | requires `zoneFilter != -1`; both call sites pass `-1` (`0x00598D73 PUSH -1`) |
| The stamper's whole `zoneFilter != -1` branch, incl. `g_nShorePieceGroupTable` `0x0082A7F4` | **Dead on the RMG path**, live for `MapClass__SelectBridgeTileVariant_Low` / `FUN_0059E740` | `0x0057B5EA CMP EDI,-1 / JZ` |
| `if (piece == -1)` at `0x00579719`, `0x005798E8` | **Unreachable** | operands are `(r%3)+9` and `(r%3)+0x0F/0x17/5` ⇒ always `≥ 1` |
| `g_dwCliffPieceAnchorOffsets[0x20] = (1,0)` | **Unreachable** | `0x20` not in the selector's producible set |
| `FUN_005A1E10` | **Dead function** (out-of-line twin of the inlined block) | `get_xrefs_to 0x005A1E10` → no references |
| Scratch-grid alloc/free in `0x00578E60` | **Normally skipped** — the region pass already allocated `DAT_00ABED10` | `RmgRegion__Split`, `Rmg__GetCellRegionId` etc. read `0x0089C2DC` / the grid earlier in `Generate` |

No branch here is gated on `SpecialFlags` or on a TS-only feature toggle. The dead paths
above are argument-specialisation and compiler artefacts, not TS legacy. Fog-of-war,
tunnels and subterranean logic do not appear anywhere in this subtree.

---

## 9. Is the type-3/4 region/bridge block now fully decoded?

**The RNG stream is closed for `0x00578E60`.** With `0x0058EBC0`, `0x005A19E0`,
`0x005A17F0` / `0x005A1350` already decoded and `0x00578E60` decoded here, the only
undecoded member of the five-call block is **`0x0058EF10`
(`RandomMapGenerator__BridgeAndConnectorPass`)**, which the handoff brief listed as
already-covered but which is a distinct function from `0x0058EBC0`. It is a heavy RNG
consumer in its own right — `get_xrefs_to 0x0065C780` shows draws inside
`RmgRegion__CarveConnectorsOrBridges` (3), `RandomMapGenerator__PlaceLowBridgeDeck` (5),
`FUN_00590970` (8), and `0x0058EF10` itself reads the scratch grid at `0x0058EF6F` /
`0x0058EFCB`. Until that subtree is enumerated the block's stream is not reproducible
end-to-end.

---

## Unverified (YELLOW)

Everything below is *not* backed by a Ghidra call this session. Do not build on it.

1. **The INI key behind `g_nCliffSet_TileSetBase` `0x00AA1020`.** Writers are
   `Read_Theater_TileSets_INI` at `0x00545A8C` (a bulk reset — `disassemble_bytes
   0x00545A60-0x00545A95` shows nine adjacent globals taking the same `ESI`),
   `0x00545DF9` and `0x00546CA1`. `0x00545DF9` is inside a
   `CMP EDI,[ESP+0xF8] / JNZ / MOV [0x00AA1020],EBX` name-match chain; the string it
   matches was not read. Same for `g_ShorePieces` `0x00ABAD28` (`0x00545E08`,
   `0x00546C95`).
2. **Whether the cliff tile-index window `[cliffBase, +0x28)` overlaps the shore window
   `[g_ShorePieces, +0x2A)`.** They are set from *different* name matches
   (different `[ESP+…]` slots), so they are different tile sets, but their numeric
   adjacency in a stock theater was not read. This decides whether PASS 2's abort at
   `0x0057B724` / `0x0057B77D` is reachable at all in stock play: if the windows are
   disjoint, **PASS 2 never aborts** and the draw count is simply "one per gated cell over
   the full diamond". If they overlap, the count is data-dependent. **This is the single
   remaining unknown in the draw ledger.**
3. **The semantic meaning of `g_nShorePieceOrientTable` values `0..7`.** The values and
   the `3 ≤ |Δ| ≤ 5` predicate are verified; reading them as an 8-way orientation class is
   inference.
4. **What `DAT_0087F8DC` / `DAT_0087F8E0` are named** (map dimension vs. local-size
   half-extent). Their *use* — the playable-diamond test
   `(N < x+y) && (x−y < N) && (y−x < N) && (x+y ≤ N + 2M)` and the iterator seed — is
   verified; the naming is not.
5. **Writer of `g_PathfinderLinearMapWidth` `0x0089C2DC`.** `get_xrefs_to` (limit 40) came
   back all-READ; the listing was capped, so the writer was not located.
6. **`IsOnBridgeRamp` `0x00578D80`** has the same tile-set-window shape as
   `CellClass__IsSpecialTerrainTile` but takes `(tileIndex, subTileIndex)` instead of a
   cell. Its name is probably drifted too. Not renamed — outside this session's scope.

---

## Ghidra changes applied this session (NOT saved — coordinator owns `save_program`)

Renames: `0x00578E60`, `0x00579010`, `0x00579620`, `0x0057B440`, `0x00598030`,
`0x0089F6A0`, `0x00880990`, `0x00ABDDA4`, `0x0082A7F4`, `0x0082A89C`.
New label: `CliffPieceAnchorOffsets_StaticInit` at `0x00579320`.
Plate comments on all of the above.
