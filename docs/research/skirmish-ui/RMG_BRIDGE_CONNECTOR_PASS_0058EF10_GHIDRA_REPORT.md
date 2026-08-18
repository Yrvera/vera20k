# RMG Bridge & Connector Pass — `0x0058EF10` subtree (Ghidra report)

**Program identity:** `gamemd.exe`, PE, `x86:LE:32`, image base `00400000`, 10036 functions —
verified via `get_current_program_info` (2026-07-25).

**Scope:** `RandomMapGenerator__BridgeAndConnectorPass 0x0058EF10` and its complete callee
subtree, including `RmgRegion__CarveConnectorsOrBridges 0x005905D0`,
`RandomMapGenerator__PlaceLowBridgeDeck 0x0058F2C0`, `FUN_00590970`, `FUN_00590FD0`, the seven
carve helpers, `FUN_0058F0C0`, `FUN_0058C800`, and the deck-support helpers.

**Why it matters:** this is the last unenumerated RNG consumer in the MapType 3/4 region block.
MapType 3 (Inland) and 4 (Mountainous) do not generate in the Rust port; this subtree is the
IslandPasses-half evidence blocker.

**Authority note:** label drift is rife in this neighbourhood. Every claim below is re-derived
from the function body / assembly this session; where a pre-existing Ghidra label is wrong it is
called out explicitly.

---

## 1. Live entry gate (VERIFIED)

`disassemble_bytes 0x00598D40-0x00598D90`:

```
00598d55  MOV EAX,[ESI + 0x3c]        ; RMG object +0x3C = MapType
00598d58  CMP EAX,0x4      JZ  ->
00598d5d  CMP EAX,0x3      JNZ skip
00598d62  CALL 0x0058ebc0             ; region rebuild / cull   (no RNG)
00598d67  CALL 0x0058ef10             ; <-- THIS PASS
00598d6e  CALL 0x005a19e0  (ECX=ESI)  ; cliff drops
00598d7b  CALL 0x00578e60  (ECX=g_Map=0x87F7E8, args EBX, -1)  ; cliff level/face fixup
00598d82  CALL 0x005a17f0  (ECX=ESI)  ; cliff rebuild
```

`get_function_callers 0x0058EF10` returns **two** callers, not one:

| Caller | Verdict |
|---|---|
| `RandomMapGenerator__Generate 0x00598960` @ `0x00598D67` | LIVE |
| `FUN_005A1E10` @ `0x005A1E25` | **DEAD** |

`FUN_005A1E10` (`disassemble_bytes 0x005A1E10-0x005A1E50`) is a 0x38-byte `__thiscall` that
re-runs the *identical* five-call MapType-3/4 block behind the identical `[this+0x3C]==3||==4`
gate, differing only in that it pushes literal `0` where the driver pushes `EBX` into
`0x00578E60`. `get_xrefs_to 0x005A1E10` → **"No references found"**. It is an unreferenced
duplicate — do not port it, and do not treat it as a second entry point.

Any prose asserting `0x0058EF10` has a *sole* caller is wrong; the correct statement is
"one live caller, one dead duplicate".

---

## 2. `RandomMapGenerator__BridgeAndConnectorPass 0x0058EF10` — full contract (VERIFIED)

`decompile_function 0x0058ef10`. `undefined4 __cdecl` (no `this`), returns **1
unconditionally**. Local `ok` (`local_5`) starts at 1 and only step 2 can clear it.

Order of operations:

1. **`FUN_004A8BF0(0)`** — placement-cursor footprint clear. `get_function_callees 0x004A8BF0`
   → `FUN_004A94F0`, `FUN_004A95A0`, `FUN_007C978A`. No RNG.

2. **Cell walk, `MapClass__RaisePinchedCliffCell(cell, -1)` per cell.**
   `MapClass__CellIterator_Init 0x00578350` / `MapClass__CellIterator_Next 0x00578290`.
   Loop guard is `while (cell != 0 && ok != 0)` — a `0` return stops the walk on the *next*
   iteration, i.e. the failing cell is still processed.

   `decompile_function 0x00579010`: **label drift already corrected in-project** — this is NOT
   a bridge-ramp placer. It reads only the cliff higher-neighbour ring mask
   (`MapClass__ComputeBridgeAdjacencyMask_Low 0x00579B70`) and, when the cell is "pinched"
   between higher ground, writes **`CellClass+0x11B += 4`** (one terrain quantum up), sets
   **scratch `+0x38 = zoneFilter`** via `RmgGrid__SetRegionTag 0x005A0090`, then recurses into
   all 8 neighbours. Its only failure branch (`zone > 0 && zone != filter && filter != -1`) is
   **unreachable from this pass**, because this pass always passes `-1`. Therefore `ok` stays 1
   in practice and steps 6a–6c always run.

3. **Scratch reset.** For every slot of the `W*W` scratch array at `DAT_00ABED10`
   (stride `0x50`, index `y*g_PathfinderLinearMapWidth + x`): `+0x38 = -1` **and** `+0x3C = -1`.
   This is required precisely because step 2 stomped `+0x38` on every raised cell.

4. **Delete every region object**, descending index over `DAT_00ABDF94[0 .. DAT_00ABDFA0-1]`:
   `FUN_0058C070` then `FUN_007C8B3D(ptr)`. `g_nRmgNextRegionId = 0`.
   `get_function_callees 0x0058C070` → `FUN_007C8B3D` only. No RNG.

5. **Region rebuild.** Ascending linear scan of the scratch array (stride `0x50`). A slot with
   `+0x38 == -1` whose packed coord passes `FUN_005AC370` seeds one `FUN_0058C800` flood; a
   returned region object gets `+0x1A = 0`. **This step CAN consume RNG** (see ledger row A).
   The rebuild completes entirely before step 6 begins.

6. If `ok`, **three separate complete loops**, each `0 .. DAT_00ABDFA0-1` ascending:
   - **6a** `FUN_0058F0C0(region)` — build the region's neighbour-id vector (`+0x04`) and its
     cell count (`+0x0C`). No RNG.
   - **6b** `RmgRegion__CarveConnectorsOrBridges(region)` — the RNG-heavy pass.
   - **6c** free `region+0x04` and store `0`.

**Load-bearing negative result:** 6a and 6b are **not** interleaved. Every region's neighbour
vector exists before the first connector call, so a connector reading a neighbour's
`+0x04/+0x10/+0x14` always sees final data. A port that fused the two loops would give region
`i` a different view of regions `j > i` and diverge. `DAT_00ABDFA0` is re-read each iteration,
but neither pass appends regions.

---

## 3. `FUN_0058F0C0` — neighbour vector + area (VERIFIED, no RNG)

`decompile_function 0x0058f0c0`. `__fastcall(RmgRegion* this)`.

- Allocates a 0x18-byte dynamic-vector object (`operator_new`, vtable `PTR_FUN_007E4E78`,
  growth step 10) into `this+0x04`.
- Allocates a `g_nRmgNextRegionId`-byte flag array, zeroed.
- Walks the region's border-cell list (`FUN_0058D410`) and, for each of the 8 directions using
  `g_DirectionOffsets 0x0089F688` (`0=N,1=NE,…,7=NW` under +X east / +Y south), marks
  `flags[scratch(nb)->+0x38] = 1` when the neighbour is inside the diamond
  (`DAT_00ABED04 < x+y`, `x-y < DAT_00ABED04`, `y-x < DAT_00ABED04`, `x+y <= DAT_00ABED08`) and
  its region id is `>= 0`.
- Appends every marked id except `this->+0x08` to `this+0x04`.
- Recounts `this+0x0C` = number of scratch slots with `+0x38 == this->id` **and** a non-zero
  packed coord.

`get_function_callees 0x0058D410` → `FUN_0042F860`, `FUN_0042FCB0`, cell iterator,
`MapCoord_StepByDir_GetCell`, `operator_new`. No RNG anywhere.

---

## 4. `RmgRegion__CarveConnectorsOrBridges 0x005905D0` — identity verified from the body

`__fastcall(RmgRegion* this)` — prologue `MOV ESI,ECX` at `0x005905D6` and
`MOV [ESP+0x10], ESI` at `0x005905D9` (`disassemble_bytes 0x005905D0-0x00590620`), so `this` is
cached in a stack slot and later re-loaded into **EBP** at `0x00590770`; **ESI is the
neighbour** at that point. (This matters: the `MOV ECX,EBP` at `0x005908E2` passes **this**, not
the neighbour, into `FUN_00590970`.)

The function branches on `this+0x14`:

### 4.1 What a "connector" is versus a bridge

They are **two mutually exclusive branches selected by the region's own water flag**, not two
options weighed against each other:

- **`this+0x14 != 0` (water region) → BRIDGE branch.** For every *unordered* pair `(i, j)`,
  `i` from 0, `j` from `i+1`, of the water region's neighbours, both of which are
  **land** (`+0x14 == 0`), sit at the **same level as each other and as the water region**
  (`+0x10` triple-equal), and are each "substantial"
  (`neighbourVector->+0x10 > 1 || region->+0x0C > 0x32`), call
  `RandomMapGenerator__PlaceLowBridgeDeck(A, B)`. A *bridge* therefore spans **water**, joining
  two land regions of equal level across it. This branch never reads Accessibility.

- **`this+0x14 == 0` (land region) → CONNECTOR branch.** For every neighbour with
  `this->id < neighbour->id` (the id ordering dedupes each pair to one visit) **and a differing
  level** (`+0x10`), carve ramps through the cliff line between the two plateaus. A *connector*
  is therefore a **cliff ramp**: a stair through a level change on dry land, cut by the seven
  carve helpers below. This is the **only** consumer of `g_nRmgSeedAccessibility 0x00ABE044`
  (`get_xrefs_to 0x00ABE044`: exactly one READ, at `0x005907B8`).

Note the asymmetry in the two loops' geometry: the bridge branch iterates *pairs of the water
region's neighbours* (`O(n²)` over the neighbour list), the connector branch iterates *single
neighbours of this region*.

### 4.2 Connector branch — predicates, caps, rollback

Per qualifying pair, in order (`disassemble_bytes 0x00590770-0x00590830`,
`0x00590860-0x00590920`):

1. `r1 = draw B` (see ledger) → `[0, 100]`.
2. `CMP EAX,[0x00ABE044]; JGE` at `0x005907B8`/`0x005907BE` — **signed** comparison.
   If `r1 < Accessibility`: `r2 = draw C` → `[1, 2]`. Else `r2 = 0`.
   `connections = r2 + 1` → **1 on failure, 2 or 3 on success. Never uniform over 1..3.**
   `Accessibility <= 0` yields exactly 1 connection; it does **not** skip the pass.
3. `lowerId` / `higherId` split: if `this->+0x10 < neighbour->+0x10` then
   `higherId = neighbour->id, lowerId = this->id`, else swapped. `higherId` is used only to
   locate the region index for the border list; **`lowerId` is what gets passed down** as
   `FUN_00590970`'s 2nd argument.
4. `FUN_0058D410` builds the shared border-cell list (`+0x04` = array, `+0x10` = count).
5. If `connections < 1` → clear `this+0x1B` and skip. Otherwise **attempt loop, cap 100**
   (`CMP EBX,0x64; JGE exit` at `0x00590865`): draw D picks a uniform border cell, then
   `MapClass__Is_Cell_In_Playfield(cell, 1)`, then
   `FUN_00590970(cell, lowerId, (float)attemptIndex * 0.01f)`.
   The float multiplier is `float [0x007EAAE0] = 0x3C23D70A = 0.01f` (`read_memory 0x007EAAE0`),
   applied to the **0-based attempt index** (`FILD dword [ESP+0x24]` at `0x005908D5`), so the
   leniency parameter runs `0.00, 0.01, … 0.99`.
   Loop exits when `successCount >= connections` **or** attempt index reaches 100.
6. **Rollback:** if `successCount == 0`, `this+0x1B = 0`. There is no cell-level rollback —
   partial carves are kept.

**Player-visible effect:** the number of ramps through each cliff line between adjacent
plateaus, i.e. choke-point density on Inland/Mountainous maps, readable on the minimap at a
glance. Trigger frequency: every MapType 3/4 skirmish generation, for every adjacent
different-level region pair — dozens of times per map.

---

## 5. `FUN_00590970` — role, signature, and the "hi" question (VERIFIED)

### 5.1 Real signature

Ghidra's registered signature `char FUN_00590970(short*, undefined4, float)` is **incomplete**.
`disassemble_bytes 0x00590970-0x005909D0`:

```
00590970  SUB ESP,0x10
00590973  MOV EAX,[ESP+0x1c]     ; arg3 = leniency float
0059097a  MOV ESI,[ESP+0x20]     ; arg1 = CellStruct*
0059097f  MOV EDI,[ESP+0x28]     ; arg2 = lowerRegionId
00590983  MOV EBP,ECX            ; <-- ECX IS CONSUMED
0059098d  MOV ECX,EBP            ; forwarded as `this` to FUN_00590FD0
005909ac  RET 0xc                ; exactly THREE stack args
```

True signature: **`char __thiscall FUN_00590970(RmgRegion* this /*ECX*/, CellStruct* cell,
int lowerRegionId, float leniency)`**.

**Verdict on the "3 params / a documented `hi` argument is never passed" note:** the params
count is right (3 stack args, proven by `RET 0xC` and by the caller's three pushes at
`0x0058DD`–`0x005908EE`), and the missing piece is the **ECX `this`, not a `hi`**. There is no
`lo`/`hi` range argument anywhere in this function: every one of its eight draws uses a
**compile-time-fixed span of 2** (constant `0x007ED8B0`), and it never calls the uniform-range
helper `0x00598030`. Any doc describing `FUN_00590970` as taking or forwarding a `hi` bound is
wrong at the premise.

The genuinely dead argument in this chain is one level down: `FUN_00590FD0` passes
`lowerRegionId` to `FUN_005A1E50` as its 3rd stack argument (`PUSH EAX` at `0x0059103A`), and
**`FUN_005A1E50` never reads that parameter** (`decompile_function 0x005A1E50` — `param_3`
unused; the id it matches on is `param_2` = EDX = `this->+0x08`). So the *lower* region id
reaches the window test and is discarded; the window test counts cells of the **caller's own**
region.

### 5.2 `FUN_00590FD0` — the orientation mask (VERIFIED)

`__thiscall(RmgRegion* this /*ECX*/, CellStruct* cell, char* okFlag, int lowerRegionId,
float leniency)`, `RET 0x10`.

1. **Count threshold** (`disassemble_bytes 0x00590FD0-0x00591030`):
   `threshold = ftol( (double[0x007ED6E0] - double[0x007ED6D8]) * (float[0x007E2AC8] - leniency)
   + double[0x007ED6D8] )`.
   `read_memory 0x007ED6D8` = `0x4014000000000000` = **5.0**; `0x007ED6E0` =
   `0x402E000000000000` = **15.0**; `read_memory 0x007E2AC8` = `0x3F800000` = **1.0f**.
   So `threshold = trunc(10·(1 − leniency) + 5)` → **15 at attempt 0, falling to 5 by attempt
   99**. This is the real leniency ramp: a *cell-count* requirement, not merely "extra
   orientations".
2. **Centre test:** `FUN_005A1E50(rect{cx-2, cy-2, 5, 5}, this->id, lowerRegionId, threshold)`.
   `FUN_005A1E50` counts cells in the rect whose scratch `+0x38 == this->id` and returns 1 as
   soon as the count reaches `threshold`. If the centre fails → `*okFlag = 0`, return 0, and
   `FUN_00590970` returns 0 immediately.
3. **3×3 ring:** the same 5×5 window is re-tested at the 9 offsets
   `(cx-2 + 5·(col-1), cy-2 + 5·(row-1))`, `row, col ∈ {0,1,2}`, skipping the centre
   (`CMP ESI,1; CMP EDI,ESI; JZ` at `0x0059106F`–`0x00591076`) — i.e. a 15×15 area tiled by
   nine 5×5 windows. Each passing window ORs in one bit from the 9-dword table at
   `0x0082AF18`.

   `read_memory 0x0082AF18` (36 bytes) = `40 80 01 20 00 02 10 08 04` (as dwords):

   | grid slot | table value | direction |
   |---|---|---|
   | row0 col0 (y−5, x−5) | `0x40` | NW |
   | row0 col1 (y−5, x)   | `0x80` | N  |
   | row0 col2 (y−5, x+5) | `0x01` | NE |
   | row1 col0 (y, x−5)   | `0x20` | W  |
   | row1 col1 (centre)   | `0x00` | (skipped) |
   | row1 col2 (y, x+5)   | `0x02` | E  |
   | row2 col0 (y+5, x−5) | `0x10` | SW |
   | row2 col1 (y+5, x)   | `0x08` | S  |
   | row2 col2 (y+5, x+5) | `0x04` | SE |

   This is exactly the same bit layout as the cliff ring mask used by
   `MapClass__ComputeBridgeAdjacencyMask_Low` (`0x01 NE, 0x02 E, 0x04 SE, 0x08 S, 0x10 SW,
   0x20 W, 0x40 NW, 0x80 N`) — i.e. `bit = 1 << ((dir + 7) mod 8)` against the
   `0=N … 7=NW` direction order.

4. `mask == 0xFF` (region present in all 8 surrounding windows) → `FUN_00590970` returns 0
   at `0x005909B1` — a fully interior cell needs no ramp.

### 5.3 Orientation dispatch (VERIFIED)

Eight guarded blocks, first success wins (`goto LAB_00590E8B`). "has" = bit set, "lacks" = bit
clear.

| # | Guard | Meaning | Helper | Draws |
|---|---|---|---|---|
| 1 | `(m & 0x82)==0x82 && (m & 0x38)==0` | has N+E, lacks S/SW/W → NE corner | `FUN_00593AF0` | 0 |
| 2 | `(m & 0x0A)==0x0A && (m & 0x11)==0` | has E+S, lacks NE/SW → SE corner | `FUN_00593550` | 0 |
| 3 | `(m & 0x28)==0x28 && (m & 0x83)==0` | has S+W, lacks N/NE/E → SW corner | `FUN_00593030` | 0 |
| 4 | `(m & 0xA0)==0xA0 && (m & 0x0E)==0` | has N+W, lacks E/SE/S → NW corner | `FUN_00593030` | 0 |
| 5 | `(m & 0x70)==0 && (m & 0x88)!=0` | lacks W/SW/NW, has N or S | `FUN_00592440` | 2 |
| 6 | `(m & 0x07)==0 && (m & 0x88)!=0` | lacks NE/E/SE, has N or S | `FUN_00591740` | 2 |
| 7 | `(m & 0xC1)==0 && (m & 0x22)!=0` | lacks N/NE/NW, has E or W | `FUN_00591D80` | 2 |
| 8 | `(m & 0x1C)==0 && (m & 0x22)!=0` | lacks SE/S/SW, has E or W | `FUN_005910F0` | 2 |

**`FUN_00593030` really is called twice** — blocks 3 and 4, `CALL 0x00593030` at `0x00590AE5`
and `0x00590B4C` (`disassemble_bytes 0x00590A80-0x00590B60`). This is not decompiler confusion:
there are only seven distinct helpers for eight cases, and the SW/NW pair (which mirror across
the isometric diagonal) shares one. All eight are `__thiscall` with `ECX = EBP = this region`
and three stack args (two `CellStruct*` endpoints + `lowerRegionId`).

Blocks 5–8 each perturb both endpoint coordinates by an independent `U{0,1}` (rows E1–E8 in the
ledger) before calling their helper; blocks 1–4 use deterministic geometry (`±1, ±4, ±5, ±6`
offsets selected by individual mask bits).

### 5.4 Leniency fallback (VERIFIED, no RNG)

`if (leniency > 0.5 && nothing succeeded)` — i.e. attempt index ≥ 51 — four extra attempts with
**fixed** geometry and no draws of their own, each still gated on a mask bit being clear:

| Guard | Helper | Endpoints |
|---|---|---|
| `(m & 0x08)==0` | `FUN_005910F0` | `(x−4, y+1)` … `(x+4, y+1)` |
| `(m & 0x02)==0` | `FUN_00591740` | `(x+1, y+4)` … `(x+1, y−4)` |
| `(m & 0x08)==0` | `FUN_00591D80` | `(x+4, y−1)` … `(x−4, y−1)` |
| `(m & 0x20)==0` | `FUN_00592440` | `(x−1, y−4)` … `(x−1, y+4)` |

Each invoked helper still consumes its own single `U{0,1}` (ledger row F), so the fallback can
add up to four draws per attempt. Note the first and third share the guard `(m & 0x08)==0`
(S bit) — that is what the binary does, not a transcription slip.

---

## 6. `RandomMapGenerator__PlaceLowBridgeDeck 0x0058F2C0` — the named blocker, decoded

`__thiscall(RmgRegion* waterRegion /*ECX*/, int regionA, int regionB)`. Returns 1 on placement,
0 after exhausting attempts. `decompile_function 0x0058f2c0`.

Attempt counter `local_1D4` starts at 0, incremented at the tail; `if (199 < n) return` →
**200 attempts max**. The loop also exits at the top when a deck was placed.

### 6.1 Per attempt

1. **Seed cell.** Inner rejection nest (`disassemble_bytes 0x0058F330-0x0058F380`):
   draw a uniform index over `W·W` (ledger row G), require
   `scratch[idx].+0x38 == waterRegion->id`, and require `FUN_0050E470(MapCoord_Set(0,0))` to
   return 0. Both extra predicates re-enter the **draw** loop, so this site can consume many
   values per attempt.
2. **Two corridor walks** from that cell — one along Y ("NS", `bVar1`) and one along X ("EW",
   `bVar2`) — each expanded outward in both directions while `FUN_005A7250` says "keep going",
   testing the cell **and its `+2` partner** with `MapClass__Is_Cell_In_Playfield(…, 1)` and
   `CellClass__IsSpecialTerrainTile 0x004863D0`. Each axis yields a span (`|Δ|`, computed with
   the `SAR 31 / XOR / SUB` abs idiom) and the scratch region ids at its two ends.
3. **End-id validation.** An axis survives only if its two end ids are `{A, B}` in either
   order. If both survive, the **strictly shorter wins; on a tie EW wins**
   (`if (nsLen < ewLen) drop EW; else drop NS`, with EW evaluated first).
4. **Length gate.** `len < attempt/25 + 8`, integer division, attempt 0-based → the gate
   loosens by one cell every 25 failed attempts (8, 9, 10, …, 15).
5. **Deck rect.** EW → `{x, y, w = span+1, h = 3}`; NS → `{x, y, w = 3, h = span+1}`.
   `RandomMapGenerator__ValidateLowBridgeDeckArea 0x005902C0` must accept it.

### 6.2 Overlay stamp (VERIFIED, including the INI mapping)

Per cell of the rect, `MapClass__Get_CellClass` then a write to **`CellClass+0x44`** (the
overlay-type index — corroborated by `RandomMapGenerator__PlaceBridgeRepairHut` and
`ValidateLowBridgeDeckArea`, both of which treat `+0x44 == -1` as "no overlay") and
**`CellClass+0x11E`** (the cross-deck sub-index, `0..2`):

| EW deck | value | NS deck | value |
|---|---|---|---|
| first column (`x == x0`) | `0x5E` | first row (`y == y0`) | `0x60` |
| last column (`x == x0+w-1`) | `0x5C` | last row (`y == y0+h-1`) | `0x62` |
| middle columns | `0x4A + (x mod 4)` | middle rows | `0x53 + (y mod 4)` |
| every cell | `+0x11E = (char)(y − y0)` | every cell | `+0x11E = (char)(x − x0)` |

The `mod 4` is the MSVC signed-remainder idiom (`AND 0x80000003`, then
`(v-1 | 0xFFFFFFFC)+1` when negative) — reproduce it exactly for negative coordinates.

Resolved against the in-repo INI (`ini/rulesmd.ini`, `[OverlayTypes]`, **ordinal position**, not
the key number — key `1` is ordinal `0`):

| index | overlay | role |
|---|---|---|
| `0x4A`–`0x4D` (74–77) | `LOBRDG01`–`LOBRDG04` | EW deck middle, 4-phase repeat |
| `0x53`–`0x56` (83–86) | `LOBRDG10`–`LOBRDG13` | NS deck middle, 4-phase repeat |
| `0x5C` (92) | `LOBRDG19` | EW deck east end |
| `0x5E` (94) | `LOBRDG21` | EW deck west end |
| `0x60` (96) | `LOBRDG23` | NS deck north end |
| `0x62` (98) | `LOBRDG25` | NS deck south end |

`ini/rules.ini` has the identical ordinals for this range, so the RA2 fallback needs no separate
table.

### 6.3 End pieces — tile choice, level handling, sub-tile (VERIFIED except two loads)

Per deck end: probe a rect with `RandomMapGenerator__IsUniformLevelBridgeEndArea 0x005A7440`;
if it passes, flip one `U{0,1}` coin (ledger rows H1–H4); then
`RandomMapGenerator__StampIsometricTileBlock(tileIndex /*ECX*/, origin /*EDX*/, -1, -1)`.

The coin changes **both the tile index and (for two of the four ends) the anchor**:

| Branch / end | probe rect | coin==0 / probe failed | coin==1 |
|---|---|---|---|
| EW, east end | `{x+w, y−2, 6, 6}` | tile `[0x00ABBEC4]+0`, anchor `(x+w, y)` | tile `?+0xA`, anchor `(x+w, y)` |
| EW, west end | `{x−6, y−2, 6, 6}` | tile `[0x00ABBEC4]+2`, anchor `(x−1, y)` | tile `?+0x9`, anchor `(x−4, y)` |
| NS, north end | `{x−2, y−6, 7, 6}` | tile `[0x00ABBEC4]+1`, anchor `(x, y−1)` | tile `[0x00ABBEC8]+0xD`, anchor `(x, y−4)` |
| NS, south end | `{x−2, y+h, 7, 6}` | tile `[0x00ABBEC4]+3`, anchor `(x, y+h)` | tile `[0x00ABBEC8]+0xC`, anchor `(x, y+h)` |

Citations: `disassemble_bytes 0x0058FC40-0x0058FC98` (EW east: `MOV ECX,[0x00ABBEC4]` at
`0x0058FC6C`, `ADD ECX,0xA` at `0x0058FC40`); `0x0058FD10-0x0058FD70` (EW west:
`LEA ECX,[EAX+2]` at `0x0058FD54` off `MOV EAX,[0x00ABBEC4]` at `0x0058FD33`, and
`LEA ECX,[EAX+9]` at `0x0058FD25`); `0x0058FFD0-0x00590068` (NS north:
`MOV ECX,[0x00ABBEC8]` at `0x0059004A` + `ADD ECX,0xD` at `0x0059006E`; default
`MOV ECX,[0x00ABBEC4]` at `0x00590090` + `INC ECX` at `0x005900B2`);
`0x00590140-0x005901B0` (NS south: `MOV EAX,[0x00ABBEC8]` at `0x00590141` +
`LEA ECX,[EAX+0xC]` at `0x00590159`; default `MOV ECX,[0x00ABBEC4]` at `0x00590173` +
`ADD ECX,3` at `0x0059018E`).

`get_xrefs_to 0x00ABBEC4`: four READs, all inside `PlaceLowBridgeDeck`, plus WRITEs from
`Read_Theater_TileSets_INI` at `0x00545AFB` and `0x00545EE9` — so both globals are
**theater tileset base indices** resolved at theater load, not RMG-computed. `0x00ABBEC8` is a
second, distinct tileset base (sub-indices `0x9`, `0xA`, `0xC`, `0xD`).

**Level handling.** `decompile_function 0x005A6C10`:
`StampIsometricTileBlock(tileIdx, origin, scratchId, levelBase)` writes per stamped sub-cell:
`CellClass+0x11A = (char)subIdx`, `CellClass+0x38 = type[0xA5]` (tileset base index),
`CellClass+0x11C = (u8) subRecord->+0x2A`, and `scratch->+0x38 = scratchId` — and it writes
`CellClass+0x11B` **only when `levelBase != -1`**, as
`+0x11B = (s8) subRecord->+0x28 + (char)levelBase − 4`.
The deck ends pass `(-1, -1)`, so a deck-end stamp **does not change cell level** and sets
scratch `+0x38 = -1` (the "unassigned" marker). `+0x11C` is written from the sub-tile record
byte `+0x2A` — confirming the established fact that `+0x11C` is not RMG-computed.

**Deck cells themselves are never stamped as tiles** — only overlays (`+0x44`) and `+0x11E`.
Only the four end pieces go through the tile stamper.

### 6.4 Repair huts (VERIFIED)

Per end, `RandomMapGenerator__PlaceBridgeRepairHut 0x005904B0` is called with a primary rect
and, only if that returns 0, a fallback rect:

| Branch | end | primary rect | fallback rect |
|---|---|---|---|
| EW | west | `{x, y−1, 2, 5}` | `{x−1, y−2, 3, 7}` |
| EW | east | `{x+w−2, y−1, 2, 5}` | `{x+w−2, y−2, 3, 7}` |
| NS | north | `{x−1, y, 5, 2}` | `{x−2, y−1, 7, 3}` |
| NS | south | `{x−1, y+h−2, 5, 2}` | `{x−2, y+h−2, 7, 3}` |

`decompile_function 0x005904B0`: it scans `(w+1) × (h+1)` cells **inclusive**, takes the first
cell with `+0x44 == -1`, `CellClass__IsClearTile 0x00486380`, and `+0xE4 == 0`, then constructs
one `CABHUT` (`BuildingTypeClass__FindIndexByName`, string `0x0082BA00`) owned by `Neutral`
(country lookup `FUN_005117D0`, string `0x0082BA08`), `operator_new(0x720)`,
`BuildingClass__Constructor`, and Unlimbos it at cell-centre leptons
(`x*256+128, y*256+128, 0`) with direction 0. **At most one hut per call**, so a finished deck
normally carries a Neutral CABHUT at each end.

It consumes no map-gen RNG, **but see ledger row X** — the BuildingClass construction chain does
draw once from a different RNG instance.

### 6.5 The two area validators are NOT interchangeable (VERIFIED)

- `RandomMapGenerator__ValidateLowBridgeDeckArea 0x005902C0` walks
  `y ∈ [y, y+h]`, `x ∈ [x, x+w]` **inclusive** → `(w+1) × (h+1)` cells, one row and column
  MORE than the deck. Per cell: `+0x44 == -1`, `(s8) +0x11B == lvl` (lvl read from `(x, y)`
  first), and `IsClearTile 0x00486380 || 0x004865D0`.
- `RandomMapGenerator__IsUniformLevelBridgeEndArea 0x005A7440` walks
  `y ∈ [y, y+h)`, `x ∈ [x, x+w)` → **exactly `w × h`**. Per cell: `(s8) +0x11B == lvl`, then
  `if (0x004866D0() || 0x004866F0()) verdict = acceptOverride else verdict = IsClearTile ||
  0x00486650() || 0x00486670()`.

Both do four corner diamond tests first with the literal operator set
(`g_nMapDiamondWidth < x+y && x−y < W && y−x < W && x+y <= g_nMapDiamondMaxCoordSum`).

**Label-drift warning carried forward:** the `0x004865D0` call renders as
`CellClass__HasBridgeOverlay`. Reading its body (`decompile_function 0x004865D0`) shows it is a
predicate on `CellClass+0x38` (tile index) against the shore-piece set (`[0x00ABAD28]`, 0x2A
entries), the water set (`[0x00AA0738]`, 0x0E entries), and four 4-entry RMG river-bridge
tilesets (`[0x00AA073C]`, `[0x00ABB110]`, `[0x00AA1050]`, `[0x00AA10A0]`) — i.e. "shore, water,
or river-bridge tile". The deck-area rule is therefore **"clear OR water"**, not
"clear OR bridge". Porting from the label accepts the wrong cells.

---

## 7. Complete RNG draw ledger for the whole subtree

**Instance:** every draw below loads `ECX = 0x00ABE890` immediately before
`CALL Random__Next 0x0065C780` — verified at each cited address. The one exception (row X) uses
a different instance and is flagged.

**Two uniform shapes are present in this subtree, and they are not interchangeable:**

- **Shape P ("pre-divided, single FMUL"):** `FILD qword(rnd) ; FMUL double[K] ; [optional FADD
  double[1.0]] ; Math__ftol`, where `K = span·(1+2⁻³²)·2⁻³²` is a *compile-time* constant.
  Constants used here (`read_memory 0x007ED890` +48, `read_memory 0x007E1718`):
  - `0x007ED8B0` = `0x3E00000000100000` = `2·(1+2⁻³²)·2⁻³²`
  - `0x007ED8B8` = `0x3E59400000194000` = `101·(1+2⁻³²)·2⁻³²`
  - `0x007E1718` = `0x3FF0000000000000` = `1.0`
- **Shape H ("helper shape, two FMULs"):** `FILD qword(span) ; FSTP double tmp ; FILD
  qword(rnd) ; FMUL double tmp ; FMUL double[0x007ED898] ; Math__ftol`, where
  `0x007ED898` = `0x3DF0000000100000` = `(1+2⁻³²)·2⁻³²`. This is
  `RandomMapGenerator__NextUniformRange 0x00598030` **inlined with `lo = 0`** — the helper is
  never actually *called* from this subtree.

Neither shape matches the river-bridge builder's `6/(2³²−1)` / `4/(2³²−1)` constants at
`0x007EDA40` / `0x007EDA38`. Do not share a Rust helper across the two families.

### 7.1 Rounding — and why every rejection loop here is unreachable

`disassemble_function 0x007C5F00`: `Math__ftol` reads the ambient control word, and if it
differs from `[0x00822D80]` it `FLDCW`s that value and **never restores the original**.
`read_memory 0x00822D80` = `0x0E7F` → PC = `10b` (53-bit double), **RC = `11b` = round toward
zero (chop)**.

Consequence: after the first `Math__ftol` anywhere in the process the x87 rounding mode is
chop, so every `FMUL` at every draw site above also truncates. Working the boundary case
`rnd = 0xFFFFFFFF`:

- Shape P, span 101: exact product `101 − 101·2⁻⁶⁴` → chop → just below 101 → `ftol` = 100.
  `CMP EAX,0x64; JA` does **not** retry.
- Shape P, span 2 + 1.0: exact `2 − 2⁻⁶³` → chop → `2 − 2⁻⁵²`; `+1.0` → chop → `3 − 2⁻⁵¹`;
  `ftol` = 2. `CMP EAX,2; JA` does **not** retry.
- Shape P, span 2, no FADD: `ftol` = 1. `CMP EAX,1; JA` does **not** retry.
- Shape H, span `n`: exact `n − n·2⁻⁶⁴` → chop → `ftol` = `n−1` = `hi`. `JA` does **not** retry.

So **under the installed control word no rejection loop in this subtree can fire.** Under
round-to-nearest they would all fire exactly at `rnd = 0xFFFFFFFF`. A Rust port should still
implement the loops verbatim (they cost nothing and protect against a control-word change), but
must not "simplify" the arithmetic in a way that changes which side of the boundary lands.
This directly contradicts any prior note claiming these loops "fire for `0xFFFFFFFF`" — that is
true only if RC is round-to-nearest, which it is not here.

### 7.2 The ledger, in execution order

| # | Address | Function | Shape / constant | Result domain | Frequency |
|---|---|---|---|---|---|
| **A** | `0x0058CAFE` | `FUN_0058C800` (step 5 rebuild) | H, span = border-cell count | `[0, count−1]` | per rebuild flood that (a) is region id 0, (b) is non-water, (c) covered ≤ 0x4A cells |
| **B** | `0x00590797` | `CarveConnectorsOrBridges` | P, `0x007ED8B8` (span 101) | `[0, 100]` | **per region pair** (land branch, `this.id < nb.id`, differing level) |
| **C** | `0x005907C5` | `CarveConnectorsOrBridges` | P, `0x007ED8B0` + `1.0` | `[1, 2]` | per region pair, **only if `B < Accessibility`** (signed) |
| **D** | `0x0059088E` | `CarveConnectorsOrBridges` | H, span = border-list length | `[0, len−1]` | **per attempt** (≤ 100 per pair) |
| **E1** | `0x00590BB3` | `FUN_00590970` blk5 | P, `0x007ED8B0` (no FADD) | `{0, 1}` | per attempt, only if blk5 guard matches |
| **E2** | `0x00590BE2` | `FUN_00590970` blk5 | " | `{0, 1}` | " |
| **E3** | `0x00590C80` | `FUN_00590970` blk6 | " | `{0, 1}` | per attempt, only if blk6 guard matches |
| **E4** | `0x00590CB0` | `FUN_00590970` blk6 | " | `{0, 1}` | " |
| **E5** | `0x00590D4F` | `FUN_00590970` blk7 | " | `{0, 1}` | per attempt, only if blk7 guard matches |
| **E6** | `0x00590D7E` | `FUN_00590970` blk7 | " | `{0, 1}` | " |
| **E7** | `0x00590E1C` | `FUN_00590970` blk8 | " | `{0, 1}` | per attempt, only if blk8 guard matches |
| **E8** | `0x00590E4C` | `FUN_00590970` blk8 | " | `{0, 1}` | " |
| **F1** | `0x0059283D` | `FUN_00592440` | P, `0x007ED8B0` | `{0, 1}` | once per invocation (blk5, or fallback #4) |
| **F2** | `0x00591AB7` | `FUN_00591740` | " | `{0, 1}` | once per invocation (blk6, or fallback #2) |
| **F3** | `0x0059218B` | `FUN_00591D80` | " | `{0, 1}` | once per invocation (blk7, or fallback #3) |
| **F4** | `0x005914A9` | `FUN_005910F0` | " | `{0, 1}` | once per invocation (blk8, or fallback #1) |
| **G** | `0x0058F347` | `PlaceLowBridgeDeck` | H, span = `W·W` | `[0, W·W−1]` | **per attempt** (≤ 200 per region pair), re-drawn on scratch-id / `FUN_0050E470` rejection — so ≥ 1 per attempt, often many |
| **H1** | `0x0058FBE2` | `PlaceLowBridgeDeck` EW east end | P, `0x007ED8B0` | `{0, 1}` | once, only if the end-area probe passed |
| **H2** | `0x0058FCCB` | `PlaceLowBridgeDeck` EW west end | " | `{0, 1}` | " |
| **H3** | `0x00590013` | `PlaceLowBridgeDeck` NS north end | " | `{0, 1}` | " |
| **H4** | `0x005900FC` | `PlaceLowBridgeDeck` NS south end | " | `{0, 1}` | " |
| **X** | `0x006F3254` | `TechnoClass__Constructor` (via `PlaceBridgeRepairHut` → `BuildingClass__Constructor`) | raw `Random__Next`, stored to `word [this+0x3C8]` | u16 | once per CABHUT placed (≤ 2 per successful deck). **`ECX = [0x00A8B230] + 0x218` — a DIFFERENT RNG instance**, so it does not perturb the map-gen stream, but it does perturb that one |

**Discarded / conditional draws are all accounted for:** the only discarded values are the
rejection re-draws (rows A, B, C, D, G and the `{0,1}` rows), and per §7.1 none of the pure
rejection loops can fire under the installed control word — with the important exception of
**row G**, whose rejection is *not* arithmetic (`scratch+0x38 != this->id`, `FUN_0050E470`) and
therefore does discard values in normal operation. Row G is the one place in this subtree where
a port must model rejection to stay in sync.

**Functions in the subtree proven to draw nothing** (each absent from the complete
`get_xrefs_to 0x0065C780` listing, and their own callee sets likewise absent):
`FUN_004A8BF0`, `MapClass__CellIterator_Init/Next`, `MapClass__RaisePinchedCliffCell`
(+ `0x00579B70`, `0x00481810`, `0x005A00C0`, `0x005A0090`), `FUN_0058C070`, `FUN_005AC370`,
`MapClass__Get_CellClass`, `FUN_0058F0C0`, `FUN_0058D410`, `FUN_00590FD0`, `FUN_005A1E50`,
`FUN_00593030`, `FUN_00593550`, `FUN_00593AF0`, `MapClass__Is_Cell_In_Playfield`,
`CellClass__IsSpecialTerrainTile`, `FUN_005A7250`, `FUN_0050E470`, `MapCoord_Set`,
`ValidateLowBridgeDeckArea`, `IsUniformLevelBridgeEndArea`, `StampIsometricTileBlock`,
`PlaceBridgeRepairHut` (itself).

### 7.3 True `Random__Next` call-site count, and the 15-vs-16 discrepancy

Derived from the complete `get_xrefs_to 0x0065C780` listing (single call, whole binary), then
intersected with the subtree:

| Scope | Sites |
|---|---|
| `FUN_0058C800` (step-5 rebuild flood) | 1 |
| `RmgRegion__CarveConnectorsOrBridges` | 3 |
| `FUN_00590970` | 8 |
| `FUN_005910F0` + `FUN_00591740` + `FUN_00591D80` + `FUN_00592440` | 4 |
| `RandomMapGenerator__PlaceLowBridgeDeck` | 5 |
| **Total, map-gen RNG instance `0x00ABE890`** | **21** |
| plus `TechnoClass__Constructor` (different instance) | 1 → **22** overall |

The historical numbers resolve as follows — both were *scoping* errors, not counting errors:

- **15** = `CarveConnectorsOrBridges` (3) + `FUN_00590970` (8) + the four edge helpers (4).
  This is the scope "functions unique to the land/connector carve".
- **16** = the same 15 **plus** `FUN_0058C800`'s single site, i.e. "the connector carve plus the
  region-rebuild flood".
- Both **omit `RandomMapGenerator__PlaceLowBridgeDeck` entirely** (5 sites) — which is exactly
  the water/bridge half that was never decoded. Neither figure is the subtree total.

---

## 8. Every grid / CellClass field the subtree writes

Two distinct arrays. **`DAT_00ABED10`** is the RMG's own per-cell scratch grid: stride `0x50`,
indexed `y * g_PathfinderLinearMapWidth + x`, with the packed cell coord at `+0x00`/`+0x02`
(which is why `MapClass__Get_CellClass(scratchSlotPtr)` works — `Get_CellClass` takes a
`CellStruct*`, verified `decompile_function 0x005657A0`: `idx = y*0x200 + x`, table at
`MapClass+0x13C`). **`CellClass`** is the real map cell.

| Field | Writer | Value | Later consumer |
|---|---|---|---|
| scratch `+0x38` (region id) | `MapClass__RaisePinchedCliffCell` via `RmgGrid__SetRegionTag 0x005A0090` | `-1` (this pass always passes `-1`) | immediately invalidated by step 3 |
| scratch `+0x38`, `+0x3C` | step 3 of `0x0058EF10` | `-1`, `-1` | step 5 rebuild seed scan (`+0x38 == -1`), `FUN_0058C800`'s visited marker (`+0x3C`) |
| scratch `+0x38`, `+0x3C` | `FUN_0058C800` flood | new region id | `FUN_0058F0C0` (neighbour ids, area), `FUN_005A1E50` (window counts), `PlaceLowBridgeDeck` (seed filter + corridor end ids), `CarveConnectorsOrBridges` |
| scratch `+0x38` = `-1`, scratch `+0x4B` = `0` | `FUN_0058C800` discard path (region too small, ≤ 0x4A cells) | | the same rebuild scan; `+0x4B` is a scratch tag read by the tech-building placer `0x005A95B0` |
| scratch `+0x38` | `StampIsometricTileBlock` | `scratchId` param — **`-1`** from every deck-end call | rebuild scan / region queries |
| `CellClass+0x11B` (level, quantum 4) | `MapClass__RaisePinchedCliffCell` | `+= 4` | `FUN_0058C800` flood connectivity (`+0x11B` equality), both area validators, `0x00578E60` cliff level/face fixup (next stage), `0x005A19E0` cliff drops |
| `CellClass+0x11B` | `StampIsometricTileBlock`, **only when `levelBase != -1`** | `subRec->+0x28 + levelBase − 4` | as above. Deck ends pass `-1`, so they do **not** touch level |
| `CellClass+0x38` (tile index) | `FUN_0058C800` discard path (`= 0`), `StampIsometricTileBlock` (`= tileType[0xA5]`) | | `CellClass__HasBridgeOverlay 0x004865D0` (the shore/water/river-bridge predicate), `CellClass__IsClearTile`, renderer |
| `CellClass+0x11A` (sub-tile index in block) | `FUN_0058C800` discard path (`= 0`), `StampIsometricTileBlock` (`= (char)idx`) | | renderer, `0x00578E60` |
| `CellClass+0x11C` (slope type, 0..4) | `StampIsometricTileBlock` only | `(u8) subRec->+0x2A`, copied verbatim | `CellClass__ApplyLAT_and_SlopeFixup` second half. Confirms the established fact: **`+0x11C` is not RMG-computed** |
| `CellClass+0x44` (overlay index) | `PlaceLowBridgeDeck` deck stamp | `0x4A+(x%4)` / `0x53+(y%4)` / `0x5C` / `0x5E` / `0x60` / `0x62` | `ValidateLowBridgeDeckArea` and `PlaceBridgeRepairHut` (both require `-1`), `OverlayClass__Mark`, renderer, bridge-destruction logic |
| `CellClass+0x11E` (cross-deck sub-index) | `PlaceLowBridgeDeck` deck stamp | `0..2` | overlay frame selection for the 3-wide bridge deck |
| `RmgRegion+0x1A` | step 5 of `0x0058EF10` | `0` | unread within this subtree |
| `RmgRegion+0x1B` | `CarveConnectorsOrBridges` | `0` when a pair carved nothing | unread within this subtree — inferred connectivity flag |
| `RmgRegion+0x04` | `FUN_0058F0C0` (alloc), step 6c (`= 0` after free) | | `CarveConnectorsOrBridges` neighbour walk |
| `RmgRegion+0x0C` | `FUN_0058F0C0` | region cell count | `CarveConnectorsOrBridges` bridge-branch "substantial" test (`> 0x32`) |
| `RmgRegion+0x10`, `+0x14` | `FUN_0058C800` at construction | level, water flag | both branches of `CarveConnectorsOrBridges` |
| one `BuildingClass` (`Neutral` `CABHUT`) | `PlaceBridgeRepairHut` | Unlimboed at cell-centre leptons, dir 0 | the live game (repairable bridge) |
| `g_nRmgNextRegionId 0x00ABED14` | step 4 (`= 0`), `RmgRegion__Ctor` | | `FUN_0058F0C0` flag-array size, `FUN_0058C800` |

---

## 9. Tiberian Sun / dead-branch check

| Candidate | Verdict | Evidence |
|---|---|---|
| `FUN_005A1E10` — duplicate MapType-3/4 driver block | **DEAD in the shipped binary** | `get_xrefs_to 0x005A1E10` → "No references found". Zero callers, zero data refs. Do not port |
| `MapClass__RaisePinchedCliffCell` zone-filter early-out (`zone > 0 && zone != filter && filter != -1` → return 0) | **Unreachable from this pass** | this pass passes `-1` at `0x0058EF10`'s call site; the recursion propagates the same `-1`. The branch exists for callers that pass a real zone id, of which this pass is not one |
| `if (connections < 1)` → `this+0x1B = 0` in `CarveConnectorsOrBridges` | **Unreachable** | `connections = r2 + 1` with `r2 >= 0`, so `connections >= 1` always. Dead guard; the `+0x1B = 0` write is still reachable via the `successCount == 0` path |
| Arithmetic rejection loops (rows B, C, D, and every `{0,1}` row) | **Unreachable under the installed FPU control word** | §7.1 |
| Bridge branch's `neighbourVector->+0x10 > 1 \|\| region->+0x0C > 0x32` "substantial" test | LIVE | both terms are region data written this same pass |
| `IsUniformLevelBridgeEndArea`'s `acceptOverride` path | **status unknown** | see Unverified |
| Any fog-of-war / subterranean interaction | none present | no `SpecialFlags` read, no tunnel field touched anywhere in the subtree |

Nothing in this subtree is gated behind a `SpecialFlags` bit or an INI toggle. The whole pass is
unconditionally live for MapType 3 and 4 in a stock YR skirmish; the only INI-derived input is
`g_nRmgSeedAccessibility 0x00ABE044`.

---

## 10. Unverified (YELLOW) — do not build on these without further work

1. **`RmgRegion` field semantics.** `+0x08` = id, `+0x0C` = cell count, `+0x10` = level,
   `+0x14` = water flag, `+0x1A`/`+0x1B` = flags — all inferred from control-flow role in this
   subtree plus `RmgRegion__Ctor 0x0058BF70`'s assignments as seen through `FUN_0058C800`. The
   writers outside this subtree were not traced, and no consumer of `+0x1A`/`+0x1B` was found at
   all. Whether higher `+0x10` means higher ground is **not** established.
2. **Two of the eight end-piece tile loads.** The EW-east coin path (`ADD ECX,0xA` at
   `0x0058FC40`) and the EW-west coin path (`LEA ECX,[EAX+9]` at `0x0058FD25`) take their base
   from a register loaded in a byte range I did not disassemble (`0x0058FC10-0x0058FC3F`,
   `0x0058FCE0-0x0058FD11`). By analogy with the NS ends the base is almost certainly
   `[0x00ABBEC8]`, but that is an inference, not a read.
3. **`IsUniformLevelBridgeEndArea`'s `acceptOverride` byte.** Ghidra loses the argument at both
   call sites; the value the deck placer passes is unknown, so the "override accepts" branch
   cannot be resolved to accept-or-reject.
4. **The four cell-class predicates** `0x004866D0`, `0x004866F0`, `0x00486650`, `0x00486670`
   used by `IsUniformLevelBridgeEndArea`, and **`FUN_0050E470`** / **`FUN_005A7250`** used by
   the deck placer. Their bodies were not read; `FUN_005A7250` shares its callee set with
   `IsUniformLevelBridgeEndArea` (`get_function_callees 0x005A7250`), which suggests it is a
   sibling area predicate rather than a stepper, and that would change how §6.1's corridor walk
   should be described. **Resolve this before porting the corridor walk.**
5. **`FUN_0058C800`'s exact discard/merge topology.** Row A's firing condition (region id 0,
   non-water, ≤ 0x4A cells) is read off the decompile but the interaction between the
   `local_8 != 0` guard, the `0x4A` cell cap and the two merge paths was not walked on a
   concrete fixture. The draw's *existence* is certain (`0x0058CAFE`); its *frequency* is not.
6. **The seven carve helpers' tile-level output.** `FUN_00593AF0` and siblings were not decoded
   tile-by-tile; only their draw counts (0 for the four corner helpers, 1 each for the four edge
   helpers) and the fact that they reach `StampIsometricTileBlock` are established
   (`get_function_callees 0x00593AF0`).
7. **The chop-rounding argument's temporal scope.** `Math__ftol` installs `0x0E7F` and never
   restores, so RC = chop holds from the first `_ftol` onward — but an external CW change (e.g.
   a Direct3D device create without `D3DCREATE_FPU_PRESERVE`) between two `_ftol` calls would
   let one `FMUL` round differently. Within this subtree every `FMUL` is immediately followed by
   `Math__ftol` and preceded by an earlier one, so the exposure is nil in practice; proving it
   for the whole run needs a live capture.
8. **`0x00ABBEC4` / `0x00ABBEC8` tileset identities.** Both are written by
   `Read_Theater_TileSets_INI` (`0x00545AFB`, `0x00545EE9`); the reset block at
   `0x00545AD1-0x00545AFB` was read but the actual assignment at `0x00545EE9` and the tileset
   *names* were not. Needed to know which `.tmp` family the end pieces come from.

---

## 11. Equivalence check for the implementation handoff

- **Draw ledger / stream order:** certifiable today by emulating `Random__Next 0x0065C780` plus
  each draw site's `FMUL`/`ftol` shape with `emulate_function` over an exhaustive boundary set
  (`0`, `1`, `0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFE`, `0xFFFFFFFF`) for each of the three
  constants `0x007ED898`, `0x007ED8B0`, `0x007ED8B8`, both with and without the `+1.0`. That
  input domain is finite for the constant-span shapes, so exhaustive vectors are proof;
  for shape H the span is runtime, so vectors are evidence-not-proof and must be sampled per
  span actually used (`W·W`, border-list lengths).
- **Orientation mask / dispatch table:** the eight guards over a `u8` mask are a finite domain
  (256 values) — an exhaustive Rust-vs-emulated table comparison IS proof.
- **Deck overlay stamp:** certifiable by comparing generated `.map` `[OverlayPack]` bytes
  against a gamemd-generated Inland/Mountainous map with the same seed — a
  gamemd-derived byte golden.
- **Everything stateful** (attempt counts, corridor walks, region rebuild) certifies only via a
  gamemd-derived trace or a full generated-map byte comparison. **UNVERIFIED-pending-instrument**
  until a seeded gamemd map-generation capture exists.
