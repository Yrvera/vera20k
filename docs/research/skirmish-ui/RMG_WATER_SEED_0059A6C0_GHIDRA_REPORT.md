# RMG Water / Base-Terrain Seeding Phase — Ghidra Research Report

**Addresses:** `FUN_0059a6c0 @ 0x0059A6C0`, `FUN_0059c630 @ 0x0059C630`  
**Entry point:** `FUN_00598960 @ 0x00598960` ("RMG: Seeding water" progress string)  
**Investigation mode:** Algorithm decode — how the initial land/water partition is seeded  
**Active in YR:** Yes — `FUN_0059a6c0` is the standard path taken for theater/mode values 0/1/2 (all non-water theaters in normal YR skirmish). `FUN_0059c630` runs unconditionally after either seeding branch.

---

## Investigation Scaffolding

**Target question:** How is the initial land/water (and base ground) partition of the RMG map seeded? Which sub-algorithms and RNG draws are consumed? What cell fields are written?

**Non-goals:** Do not decode `FUN_0059c580` (theater-3/4 alternate) beyond contrast notes. Do not decode `FUN_0059bbc0` flood-fill internals more than one level. Do not decode region partition, start placement, tiberium, hills, or the RNG primitive.

**Evidence needed to mark COMPLETE:**
1. Theater gating condition confirmed from `FUN_00598960` assembly.
2. Water-shape dispatch (`param_1[0xf]` = MapSeed+0x3C values 0/1/2) confirmed.
3. Cell write fields (`+0x38`, `+0x11a`) confirmed from assembly.
4. RNG instance routing confirmed per sub-function.
5. `FUN_0059c630` algorithm (water-tile variant selection) confirmed.

**Stop conditions:** Stop when all five evidence items above are satisfied.

---

## 1. Theater Gating — Which Path Is Taken

`FUN_00598960` (verified via `decompile_function 0x00598960`) contains the following gating logic (post-`Register_heap_pool("RMG: Seeding water")`):

```
if ((*(param_1 + 0x3c) == 3) || (*(param_1 + 0x3c) == 4)) {
    if (*(param_1 + 0x4c) != 0) {
        FUN_0059c580();   // theater-3/4 alternate (water-heavy / naval theaters)
    }
} else {
    FUN_0059a6c0();       // standard land seeding (theaters 0/1/2)
}
FUN_0059c630();           // unconditional finalizer
```

- `MapSeed+0x3C` is the theater/mode selector (int32). Values 3 and 4 are water-heavy/naval theaters; 0/1/2 are standard land theaters.
- In all normal YR skirmish games (Temperate, Snow, Urban), `+0x3C` is 0, 1, or 2 → **`FUN_0059a6c0` is the active path**.
- `FUN_0059c580` requires BOTH `+0x3C == 3 or 4` AND `+0x4C != 0`; if `+0x4C == 0` for a water theater, neither sub-function runs (all cells remain water-set), and only the finalizer runs.
- `FUN_0059c630` **always runs** in both branches.

`verified via decompile_function 0x00598960`

---

## 2. `FUN_0059a6c0` — Standard Land/Water Seeding Algorithm

**Active in YR: Yes** (theaters 0/1/2)

### 2.1 Phase 1 — Flood map with water base value

```
MapClass__CellIterator_Init();
while (cell = MapClass__CellIterator_Next()) {
    *(cell + 0x38) = g_WaterSet_TileSetBase;
}
```

Every cell on the map is initialized to `g_WaterSet_TileSetBase` (the water tile-set's first tile ID). After this pass all cells are water.

`verified via decompile_function 0x0059a6c0` — cell offset `+0x38` is the IsometricTileTypeClass index (confirmed by `FUN_005a6c10` decompile below which writes `piVar1[0xa5]` into `iVar6 + 0x38`).

### 2.2 Phase 2 — Water-shape dispatch

`param_1[0xf]` = `*(MapSeed + 0x3C)` = the theater/mode value, which for the standard path is 0, 1, or 2. Dispatch:

| `MapSeed+0x3C` | Sub-function | Description |
|---|---|---|
| 0 | `FUN_0059ad10 @ 0x0059AD10` | Archipelago-style (multiple discrete islands) |
| 1 | `FUN_0059afa0 @ 0x0059AFA0` | Continental (single large landmass) |
| 2 | `FUN_0059b200 @ 0x0059B200` | Islands-in-sea (two landmasses from opposing sides) |

`verified via decompile_function 0x0059a6c0`

Note: value 3 and 4 are handled by `FUN_0059c580` (theater-3/4 alternate, see §6), so inside `FUN_0059a6c0` only 0/1/2 are possible.

### 2.3 Phase 3 — Remove isolated water cells (4-neighbor clear check)

After shape placement, any cell that is still `g_WaterSet_TileSetBase` AND has all 4 cardinal neighbors passing `CellClass__IsClearTile` is reset to `0` (no water):

```
while (cell = MapClass__CellIterator_Next()) {
    if (*(cell + 0x38) == g_WaterSet_TileSetBase) {
        bool all_clear = true;
        for (dir in [0,2,4,6]) {
            neighbor = Pathfinding_update_continued(dir);
            if (!CellClass__IsClearTile(neighbor)) all_clear = false;
        }
        if (all_clear) *(cell + 0x38) = 0;
    }
}
```

`verified via decompile_function 0x0059a6c0`

### 2.4 Phase 4 — Region data reset

Calls `MapClass__MarkBridgesForRepair_High(0, 0)` then resets all `DAT_00abed10` region cells (`+0x38 = 0xffffffff`, `+0x3c = 0xffffffff`) in a `DAT_0089c2dc * DAT_0089c2dc` loop. Region pointer array `DAT_00abdf94` entries are cleaned up via vtable calls and freed.

`verified via decompile_function 0x0059a6c0`

### 2.5 Phase 5 — Shore-to-green pass

For any cell that is a shore piece (`CellClass__IsShorePieceTile`), its clear cardinal neighbors are upgraded from water to `g_GreenTile` (grass/ground base tile):

```
while (cell = MapClass__CellIterator_Next()) {
    if (CellClass__IsShorePieceTile(cell)) {
        for (dir in [0,2,4,6]) {
            neighbor = Pathfinding_update_continued(dir);
            if (CellClass__IsClearTile(neighbor)) {
                *(neighbor + 0x38) = g_GreenTile;
            }
        }
    }
}
```

`verified via decompile_function 0x0059a6c0`

---

## 3. Sub-function Detail: Water-Shape Algorithms

### 3.1 `FUN_0059ad10` — Archipelago (mode=0)

- Reads `MapSeed+0x50` (water level / island count parameter).
- Sets `MapSeed+0x308 = 1` (progress counter).
- Calls `FUN_005ada40(0, 0)` (island shape initializer).
- Computes half-width `uVar5 = max(2, MapSeed[0x50] / 2)`.
- Makes one or more **`Random__Next` draws from `g_MapGenRng` (0x00ABE890)** — `MOV ECX,0xABE890` at `0x0059AD6E` immediately precedes `CALL 0x0065C780` (Random__Next), inside a rejection-sample loop. (CORRECTED — previously claimed to use a separate `0x87f7e8` RNG instance; that was a misread, see §4.) (CORRECTED 2026-07-20 — the draw is NOT an "island center offset in `[0, uVar5]`"; it is the **extra-island count** in `[1, uVar5]`: the scale chain ends with `FADD [0x007E1718]` (+1.0) at `0x0059AD8E`, and the accepted result is added to `MapSeed+0x50` (`ADD EDX,EAX` at `0x0059ADCC`) to form the island count passed to `FUN_0059a8f0`. verified via disassemble_function 0x0059AD10, 2026-07-20. Full decode in §12.5.)
- Resets all region `+0x3c` fields to 0 via a `MapClass` cell-iterator loop whose receiver is `0x87f7e8` (`MapClass__CellIterator_Init 0x00578350` / `_Next 0x00578290` — **not an RNG**).
- Calls `FUN_0059a8f0` (partition grid builder) which uses `g_MapGenRng @ 0x00ABE890` for its draws.
- Iterates grid entries up to 10 times calling `FUN_0059bbc0` (flood-fill blob placement, also uses `g_MapGenRng`).

`verified via disassemble_function 0x0059ad10, 2026-06-01; address re-verified 2026-07-20` — `0x0059AD6E MOV ECX,0xABE890` → `CALL 0x0065C780` (g_MapGenRng draw); `0x0059ADDC MOV ECX,0x87f7e8` → `CALL 0x00578350`/`CALL 0x00578290` (MapClass cell iterator, not RNG). (The previously cited `0x0059ADCC` is `ADD EDX,EAX`; the `MOV ECX,0x87f7e8` is at `0x0059ADDC`. verified via disassemble_function 0x0059AD10, 2026-07-20)

### 3.2 `FUN_0059afa0` — Continental (mode=1)

- Computes map area via `FUN_0042b1f0` = `(DAT_0087f8e0 + 4) * DAT_0087f8dc * 2` (map bounding dimensions).
- Computes target water fraction from `MapSeed+0x4c` using the formula: `local_28 = (const_max - const_min) * (1.0 - MapSeed[0x4c] * 0.01) + const_min`.
- Does NOT call `Random__Next` directly; no direct RNG draws confirmed at top level.
- Calls `FUN_0059bbc0` (flood-fill, uses `g_MapGenRng`) in a loop of up to **100** calls (counter 0..99 inclusive; `CMP dword ptr [ESP+0x18],0x64 / JGE exit` at `0x0059B0D6`, verified via disassemble_function 0x0059AFA0, 2026-07-20) until the water fraction target is met.
- Uses Manhattan-distance scan to find the nearest unvisited cell to the **fixed map center** — NOT "to the last placement" as previously stated: the distance reference registers `EBX`/`EBP` hold the constant center (`DAT_0087F8E0/2 + DAT_0087F8DC/2` and that value +1, computed once at `0x0059B01A..0x0059B03F`) and are never updated inside the scan loop (`SUB EAX,EBX` / `SUB EAX,EBP` at `0x0059B19D`/`0x0059B1A8`). (CORRECTED, verified via disassemble_function 0x0059AFA0, 2026-07-20. Full decode in §12.6.)

`verified via decompile_function 0x0059afa0`

**Note:** `FUN_0059afa0`'s callees list does NOT include `Random__Next` — all RNG is inside `FUN_0059bbc0`. `verified via get_function_callees 0x0059afa0`

### 3.3 `FUN_0059b200` — Islands-in-Sea (mode=2)

- Makes **one `Random__Next` draw from `g_MapGenRng` (0x00ABE890)** at `0x0059B2C4` (`MOV ECX,0xABE890` → `CALL 0x0065C780`) to decide whether the split axis is horizontal or vertical. (CORRECTED — the `0x87f7e8` load at `0x0059B270` feeds the `MapClass` cell iterator for the region-reset loop, not an RNG.)
- Runs two landmass placements (loop count=2 via `local_c0 = 2`): one for each side.
- Calls `FUN_0059bbc0` (flood-fill, uses `g_MapGenRng`) in a loop of up to **100 calls TOTAL across BOTH landmasses** — not per landmass: the call counter `[ESP+0x24]` is zeroed once at `0x0059B22E` before the two-rect loop and is never reset when advancing to the second rect (the per-rect reset at `0x0059B39F`/`0x0059B3A5` touches only the placed-count `[ESP+0x20]` and fraction `[ESP+0x70]`). `CMP dword ptr [ESP+0x24],0x64 / JGE exit` at `0x0059B4BF`. (CORRECTED, verified via disassemble_function 0x0059B200, 2026-07-20. Full decode in §12.7.)
- Checks `MapSeed+0x4c` (water level) against a double threshold for shape control.

`verified via disassemble_function 0x0059b200, 2026-06-01` — `0x0059B2C4 MOV ECX,0xABE890` → `CALL 0x0065C780` (g_MapGenRng); `0x0059B270 MOV ECX,0x87f7e8` → `CALL 0x00578350`/`0x00578290` (cell iterator).

---

## 4. RNG Instance Routing — Single Stream (CORRECTED 2026-06-01)

The water seeding phase draws from a **single** RNG instance: `g_MapGenRng @ 0x00ABE890`. An earlier version of this report claimed a second "`0x87f7e8`" RNG instance drove the outer shape selection; that was a **misread**. `0x87f7e8` is the global `MapClass` cell-iterator object (the `this` receiver for `MapClass__CellIterator_Init 0x00578350` / `MapClass__CellIterator_Next 0x00578290`), **not** a random-number generator.

| Function | Draw site | RNG Instance | Evidence |
|---|---|---|---|
| `FUN_0059ad10` island-center offset | `0x0059AD6E` → `CALL 0x0065C780` | `g_MapGenRng 0x00ABE890` | `MOV ECX,0xABE890` (disassemble_function 0x0059ad10, 2026-06-01) |
| `FUN_0059b200` split-axis decision | `0x0059B2C4` → `CALL 0x0065C780` | `g_MapGenRng 0x00ABE890` | `MOV ECX,0xABE890` (disassemble_function 0x0059b200, 2026-06-01) |
| `FUN_0059a8f0` (partition grid) | — | `g_MapGenRng 0x00ABE890` | slot-4 decompile_function 0x0059a8f0 (not re-disassembled this audit pass) |
| `FUN_0059bbc0` (flood-fill) | — | `g_MapGenRng 0x00ABE890` | slot-4 decompile_function 0x0059bbc0 (not re-disassembled this audit pass) |
| `FUN_0059c630` (finalizer) | — | `g_MapGenRng 0x00ABE890` | slot-4 decompile_function 0x0059c630 (not re-disassembled this audit pass) |

**`0x87f7e8` identity (CORRECTED):** It is the global `MapClass` cell-iteration object. At both sites previously cited as "RNG draws" (`0x0059ADDC`, `0x0059B270`; the earlier `0x0059ADCC` citation was off by 0x10 — that address holds `ADD EDX,EAX`, verified via disassemble_function 0x0059AD10 2026-07-20), `MOV ECX,0x87f7e8` is immediately followed by `CALL 0x00578350` (`MapClass__CellIterator_Init`) and `CALL 0x00578290` (`MapClass__CellIterator_Next`) — the region-reset cell loop — **not** `Random__Next (0x0065C780)`. `get_xrefs_to 0x0087f7e8` shows it referenced pervasively across pathfinding (`AStar_*`, `Zone_precheck`, `PathfinderClass__*`) and `AircraftClass`/`AnimClass` map queries, consistent with a map/cell object rather than an RNG. (verified via disassemble_function 0x0059ad10 + 0x0059b200; get_xrefs_to 0x0087f7e8, 2026-06-01)

**Implication for Rust:** All water-seed draws — shape selection, flood-fill, and finalizer — pull from the single `g_MapGenRng` stream seeded from `MapSeed+0x74`. Water-shape selection is therefore **fully deterministic** from the map seed; route every draw through the one map-gen RNG. There is no second / non-deterministic stream.

---

## 5. `FUN_0059c630` — Unconditional Water Finalizer

**Active in YR: Yes** — called after both `FUN_0059a6c0` and `FUN_0059c580`.

Uses `g_MapGenRng @ 0x00ABE890` exclusively for all draws. `verified via read_memory 0x0059c6a8`

### Algorithm

For every cell that is `g_WaterSet_TileSetBase` AND has `CellClass+0x11a == 0` (no sub-tile placed):

**Draw mechanism note (corrected 2026-07-20):** all three finalizer draws are
scaled-FP chains — `Math__ftol(draw × K)` with per-draw double constants —
NOT integer modulo reductions. The earlier "reduce mod N" wording was loose
shorthand and produces different values from the same raw draw if implemented
as `%`. Exact chains (verified via disassemble_function 0x0059C630 +
read_memory 0x007ED9D8/0x007ED9E0/0x007ED9E8, 2026-07-20):
- selector: `ftol(draw × [0x007ED9E8] + 1.0)`, `[0x007ED9E8]` =
  `0x3E24000000140000` (≈10·2⁻³²), FADD `1.0 @ 0x007E1718`, rejection while
  `> 10` (`CMP EAX,0xA / JA` at `0x0059C701`) → range **{1..10}**; the
  single-cell path is taken when the result **== 1**, i.e. p = **1/10**
  (not 1/11).
- 2×2 variant: `ftol(draw × [0x007ED9E0])`, `[0x007ED9E0]` =
  `0x3E6E4000001E4000` (≈242·2⁻³²), rejection while `> 0xF1` → {0..241}.
- single-cell band: `ftol(draw × [0x007ED9D8])`, `[0x007ED9D8]` =
  `0x3E69200000192000` (≈201·2⁻³²), rejection while `> 0xC8` → {0..200}.
All three constants carry the same perturbed-mantissa pattern as the range
constant `0x007ED898` — use the literal bit patterns, never `N/2^32`.

1. Check **East** (`dir=2`), **South** (`dir=4`), and **Southeast** (`dir=3`) neighbors — the current cell plus E+S+SE form a **2x2 block with the current cell as NW anchor**. (Dir args pushed at `0x0059C674`/`0x0059C67D`/`0x0059C688` into step helper `0x00481810`, which adds `g_DirectionOffsets[dir]` to the packed coord at `CellClass+0x24`; the runtime table at `0x0089F688` is filled by the initializer at `0x0049F2F0..0x0049F394` with the clockwise-from-North set 0=N(0,-1), 1=NE(1,-1), 2=E(1,0), 3=SE(1,1), 4=S(0,1), 5=SW(-1,1), 6=W(-1,0), 7=NW(-1,-1). verified via disassemble_function 0x0059C630 + decompile_function 0x00481810 + disassemble_function 0x0049F300 with preamble at 0x0049F2F0 via get_assembly_context, 2026-07-20.) If all three are also water-base with no sub-tile:
   - Selector draw (see mechanism note above): `ftol(draw × ≈10·2⁻³² + 1.0)`, rejection >10 → {1..10}. If result ≠ 1 (p = 9/10):
     - Draw from `g_MapGenRng`, rejection-sampled to 0..241 (242 values; `CMP ECX,0xF1 / JA` redraw at `0x0059C732`).
     - Compute `variant_offset`: if draw < 240 (`CMP ECX,0xF0 / JL` at `0x0059C73A`) → `variant_offset = draw / 10` (0..23, signed magic-number division at `0x0059C74B`); **edge case**: draws 240–241 take `variant_offset = 0xF7 - draw` (`MOV EDI,0xF7 / SUB EDI,ECX` at `0x0059C742`), i.e. 240→7, 241→6.
     - Lookup region value from `DAT_00abed10` for this cell's position using `CellClass+0x24` (X short) and `CellClass+0x26` (Y short): `region_value = *((cell_y * g_PathfinderLinearMapWidth + cell_x) * 0x50 + 0x38 + DAT_00abed10)`, or `-1` if `DAT_00abed10 == 0`.
     - Call `FUN_005a6c10` with **four** args: `param_1 (ECX) = DAT_00AA0738 (g_WaterSet_TileSetBase) + variant_offset`, `param_2 (EDX) = &CellClass+0x24` (packed cell position), `param_3 (stack) = region_value`, `param_4 (stack) = -1` — places an isometric water tile into the cell. (Call at `0x0059C795`; the earlier "`FUN_005a6c10(region_id, 0xffffffff)`" two-arg reading was a decompiler artifact hiding the ECX/EDX register args. verified via disassemble_function 0x0059C630, 2026-07-20)
     - Skip normal variant assignment.

2. If single-cell (no matching 3-neighbor cluster) or if `result == 1`:
   - Band draw (see mechanism note above): `ftol(draw × ≈201·2⁻³²)` (range 0..200; `CMP ECX,0xC8 / JA` redraw at `0x0059C7BF`).
   - Write `CellClass+0x38 = (value / 40) + 8 + g_WaterSet_TileSetBase` (unsigned /40 via magic multiply at `0x0059C7C7..0x0059C7D6`).
   - This selects one of **6** water tile variants: bands 0–4 span 40 values each (0–39, 40–79, 80–119, 120–159, 160–199); band 5 is hit only by value 200, p = 1/201. (verified via disassemble_function 0x0059C630, 2026-07-20)

`verified via decompile_function 0x0059c630`

### Cell Fields Written by `FUN_005a6c10`

`FUN_005a6c10` (tile placer, `decompile_function 0x005a6c10`) writes:
- `CellClass+0x38` = `IsometricTileTypeClass[tile_id].field[0xa5]` (tile set ID)
- `CellClass+0x11a` = sub-tile index within tile set (byte, checked by `0x11a == 0` guard in FUN_0059c630)
- `CellClass+0x11b` = sub-tile info byte from tile data `+0x28` (adjusted by player_id offset when param_4 != -1)
- `CellClass+0x11c` = tile flag byte from tile data `+0x2a`

`verified via decompile_function 0x005a6c10`

---

## 6. `FUN_0059c580` — Theater-3/4 Alternate (Contrast Note)

**Active in YR: Conditional** — only when `MapSeed+0x3C == 3 or 4` AND `MapSeed+0x4C != 0`.

Short function. Gated on `MapSeed+0x4C > 20` for the extra `FUN_0059d510` call. Calls `FUN_0059c920` (water-shape placer, different from the 0/1/2 set) up to 10 times. Does not perform the full land/water initialization that `FUN_0059a6c0` does. `FUN_0059c630` still runs after.

`verified via decompile_function 0x0059c580`

---

## 7. CellClass Field Reference

| Offset | Size | Meaning | Source |
|---|---|---|---|
| `+0x24` | 2 bytes (short) | Cell X coordinate | `FUN_0059c630` decompile: `*(short *)(iVar1 + 0x24)` for cell index |
| `+0x26` | 2 bytes (short) | Cell Y coordinate | `FUN_0059c630` decompile: `*(short *)(iVar1 + 0x26) * DAT_0089c2dc` |
| `+0x38` | 4 bytes (int) | Tile type ID (IsometricTileTypeClass index) | Written in all phases; `= g_WaterSet_TileSetBase` for water, `= 0` for cleared, `= g_GreenTile` for shore-adjacent grass, variant in finalizer |
| `+0x11a` | 1 byte | Sub-tile index (0 = unplaced, non-zero = tile already placed) | Read as guard in `FUN_0059c630`; written by `FUN_005a6c10` |
| `+0x11b` | 1 byte | Sub-tile detail | Written by `FUN_005a6c10` |
| `+0x11c` | 1 byte | Tile flag | Written by `FUN_005a6c10` |

---

## 8. Globals Referenced

| Symbol | Address | Value / Description |
|---|---|---|
| `g_MapGenRng` | `0x00ABE890` | Map generation RNG; seeded from `MapSeed+0x74` in entry; struct = LFG-XOR R(250,103) 1012-byte struct |
| `0x87f7e8` | `0x0087F7E8` | `MapClass` cell-iterator / map global object (receiver for `MapClass__CellIterator_Init 0x00578350` / `_Next 0x00578290`); **NOT an RNG** (corrected 2026-06-01). Pervasive across map/pathfinding code. (get_xrefs_to 0x0087f7e8, 2026-06-01) |
| `g_WaterSet_TileSetBase` | unknown (loaded from data seg) | First tile ID of the water tile set; all cells initialized to this |
| `g_GreenTile` | unknown (loaded from data seg) | Grass/ground base tile ID for shore-adjacent cells |
| `g_PathfinderLinearMapWidth` (was `DAT_0089c2dc`) | `0x0089C2DC` | Linear map width (stride for cell-region array indexing); now labeled `g_PathfinderLinearMapWidth` in the Ghidra project, 181 xrefs (verified via list_globals, 2026-07-20) |
| `DAT_00abed10` | `0x00ABED10` | Region cell array base pointer |
| `DAT_00abed14` | `0x00ABED14` | Region counter; zeroed by `FUN_0059a6c0` |
| `DAT_00abdfa0` | `0x00ABDFA0` | Region count |
| `DAT_00abdf94` | `0x00ABDF94` | Region pointer array |
| `DAT_0087f8e4/e8/ec/f0` | `0x0087F8E4/E8/EC/F0` | Map bounding box: left/right/top/bottom (used in landmass placement bounds) |
| `DAT_0087f8dc/e0` | `0x0087F8DC/E0` | Map extent parameters (used in area calc and center computation) |

---

## 9. Implementation Handoff

### 9.1 Two-phase cell initialization

**Verified behavior:** Every cell is first written to `g_WaterSet_TileSetBase` (all-water baseline), then land placement carves out non-water cells.  
**Rust delta:** No RMG terrain seeding exists; the Rust codebase has only a `RandMap.Sed` sentinel. A Rust implementation must:  
1. Fill all cells with the water tile type.  
2. Dispatch to a shape sub-function by `MapSeed.theater` (0/1/2).  
3. Apply the isolated-water-cell removal pass.  
4. Apply the shore-to-green pass.  
**Affected surface:** `src/map/` (cell data), `src/assets/` (tile ID resolution for `g_WaterSet_TileSetBase`/`g_GreenTile`).  
**Acceptance scenario:** Given any seed with theater=0/1/2, after seeding all cells start as water; after shape placement some cells are non-water; after the finalizer, water cells have variant tile IDs in the correct range.  
**Proposed Rust test name:** `test_water_seed_all_cells_start_as_water_then_shape_applied`  
**Risk:** `g_WaterSet_TileSetBase` and `g_GreenTile` values are runtime globals loaded from tile-set data, not compile-time constants — the Rust implementation must resolve these at asset load time.

### 9.2 Single RNG stream — all water-seed draws use g_MapGenRng

**Verified behavior:** Every water-seed draw (island center offset in mode=0, split-axis in mode=2, flood-fill placement, finalizer variant) consumes from `g_MapGenRng (0x00ABE890)` — the one stream seeded from `MapSeed+0x74`. (CORRECTED 2026-06-01 — there is no second RNG; `0x87f7e8` is the `MapClass` cell iterator.)  
**Rust delta:** Route all water-seed draws through the single map-gen RNG; do NOT introduce a second stream.  
**Affected surface:** `src/map/rmg.rs` (future), `src/sim/rng.rs`.  
**Acceptance scenario:** Seeding with a known seed reproduces gamemd's island center / split-axis selection bit-for-bit.  
**Proposed Rust test name:** `test_water_seed_single_rng_stream_deterministic`  
**Risk:** LOW — single deterministic stream; the only seeding subtlety is the seed-fill transform documented in the RNG-seed report. (verified via disassemble_function 0x0059ad10, 0x0059b200, 2026-06-01)

### 9.3 Finalizer water-tile variant selection

**Verified behavior:** For isolated water cells (no 3-neighbor cluster), `CellClass+0x38` is set to `g_WaterSet_TileSetBase + 8 + (ftol(draw × ≈201·2⁻³²) / 40)` (scaled-FP chain, NOT `draw % 201` — see §5 mechanism note) — **6** distinct variants: bands 0–4 span 40 values each, band 5 is reached only by draw value 200 (p = 1/201). (verified via disassemble_function 0x0059C630, 2026-07-20)  
**Rust delta:** No finalizer implemented; all water cells would have the same tile ID.  
**Affected surface:** `src/map/rmg.rs`, `src/render/` (tile selection from tile-set).  
**Acceptance scenario:** Random water cells display 6 different visual water variants (band 5 rare, p = 1/201 per eligible cell), not a uniform tile.  
**Proposed Rust test name:** `test_water_finalizer_variant_distribution_6_bands`  
**Risk:** `CellClass+0x11a` guard must be checked correctly; any cell that had `FUN_005a6c10` place a multi-cell tile must NOT have its `+0x38` overwritten by the finalizer.

---

## 10. Negative Facts / Do Not Do

1. **DO route all water-seed shape draws through `g_MapGenRng (0x00ABE890)`.** (CORRECTED 2026-06-01 — an earlier version said to use a separate `0x87f7e8` RNG; that was wrong.) The `0x87f7e8` that appears in `FUN_0059ad10`/`FUN_0059b200` is the `MapClass` cell-iterator receiver (feeds `MapClass__CellIterator_Init/_Next`), **not** an RNG — do not model it as a second stream. `verified via disassemble_function 0x0059ad10, 0x0059b200, 2026-06-01`

2. **Do NOT call `FUN_0059a6c0` for theater values 3 or 4.** The gating in `FUN_00598960` explicitly bypasses it for those values. The standard land-seeding algorithm is only ever reached for `MapSeed+0x3C` ∈ {0, 1, 2}. `verified via decompile_function 0x00598960`

3. **Do NOT skip the shore-to-green pass.** Phase 5 of `FUN_0059a6c0` upgrades water cells adjacent to shore-piece tiles to `g_GreenTile`. Omitting this leaves walkable shore cells as water (wrong passability and visual).  `verified via decompile_function 0x0059a6c0`

4. **Do NOT skip the `CellClass+0x11a == 0` guard in the finalizer.** `FUN_0059c630` only assigns water tile variants to cells that have NOT already had a multi-cell tile placed (sub-tile index = 0). Overwriting placed tiles breaks the isometric tile layout. `verified via decompile_function 0x0059c630`

5. **Do NOT implement `FUN_0059c580` (theater-3/4 path) as the standard case.** In normal YR skirmish maps, theater is 0/1/2. The naval/water-theater path is a separate, minor-population code path gated on `+0x3C == 3 or 4`. `verified via decompile_function 0x00598960, decompile_function 0x0059c580`

---

## 11. Unverified / Remaining Uncertainty

**YELLOW — Unverified:**

- **`0x87f7e8` identity — RESOLVED (2026-06-01):** Not an RNG. It is the `MapClass` cell-iterator object (receiver for `MapClass__CellIterator_Init 0x00578350` / `_Next 0x00578290`). The earlier "fourth RNG instance / possibly time-seeded / non-deterministic" concern is **withdrawn** — water-shape selection is fully deterministic from `MapSeed+0x74`. (verified via disassemble_function 0x0059ad10, 0x0059b200; get_xrefs_to 0x0087f7e8)

- **Exact values of `g_WaterSet_TileSetBase` and `g_GreenTile`.** Both are loaded from data-segment globals; their values depend on the tile set loaded for the active theater. Not confirmed as compile-time constants. Must be resolved from the tile-set data at runtime. (`runtime globals; values UNVERIFIED`)

- **`FUN_0059bbc0` flood-fill algorithm detail — RESOLVED (2026-07-20):** fully decoded in §12 (seed selection, min-heap discipline, every draw site, FP constants, cell-field writes). The per-call draw count is data-dependent but exactly enumerated in §12.2.

- **`FUN_0059c580` / `FUN_0059c920` RNG routing.** Expected to be `g_MapGenRng` (the only generator RNG; see §4), but the theater-3/4 alternate was not traced this pass.

- **`MapSeed+0x50` semantic label.** Used as the island-count/density parameter in `FUN_0059ad10` (half-width for center offset range). The INI key name mapping to `+0x50` is unconfirmed in this session.

---

## 12. Water-Shape Internals (decoded 2026-07-20)

Complete formula/draw-count decode of the previously-deferred shape stage:
`FUN_005ADA40`, `FUN_0059A8F0`, `FUN_0059BBC0`, `FUN_0059AD10`, `FUN_0059AFA0`,
`FUN_0059B200` plus their leaf helpers. All FP operand orders below are taken from
disassembly (not decompile); every double constant was read from memory with its 8-byte
bit pattern (§12.8). The process FPU control word is 0x0E7F (53-bit precision,
round-toward-zero) per the RMG x87-environment report, so `Math__ftol (0x007C5F00)` is
plain truncation.

**Polarity note (load-bearing):** the map enters this stage ALL WATER (§2.1). The
flood-fill blobs are **LAND** carved out of water: on commit `FUN_0059BBC0` writes
`CellClass+0x38 = 0` (clear/land) for every blob cell, and blob growth is gated on the
cell still holding a water/shore-family tile. The "water fraction" targets in modes 1/2
are therefore **land-fraction** targets. (verified via decompile_function 0x0059bbc0 +
disassemble_function 0x0059bbc0 `MOV dword ptr [EAX+0x38],0x0` at 0x0059C02B, 2026-07-20)

### 12.1 Shared infrastructure

- **Scratch array** `DAT_00ABED10`, stride 0x50, `g_PathfinderLinearMapWidth
  (0x0089C2DC)`-squared entries, indexed `(y*width + x)*0x50`. Entry fields used here:
  `+0x00` int16 x, `+0x02` int16 y (self coords, read by the mode-2 linear scan),
  `+0x38` int32 = committed-blob id (0 = free), `+0x3C` int32 = candidate-tested marker
  (blob id), `+0x4B` byte cleared on rollback. (verified via decompile_function
  0x0059bbc0 / 0x0059b200, 2026-07-20)
- **Blob id** = `MapSeed+0x308`, incremented after every *successful* blob (and after
  every failed attempt in mode 0). `MapSeed+0x30C` supplies the byte written to
  `CellClass+0x11B` on rollback re-watering. (verified via decompile_function
  0x0059bbc0, 2026-07-20)
- **Diamond bounds**: candidate `(x,y)` accepted iff `x+y > DAT_00ABED04`,
  `x-y < DAT_00ABED04`, `y-x < DAT_00ABED04`, `x+y <= DAT_00ABED08` (verified via
  disassemble_function 0x0059bbc0, 0x0059C083..0x0059C0A9, 2026-07-20).
- **Zero-rect sentinel** `0x00ABE2F0..FC`: `FUN_0059BBC0`, `FUN_0059BAB0`,
  `FUN_0059B940` compare their region-rect argument against this global 4-int rect; on
  equality the region/ellipse constraint is disabled. Its only writer zeroes all four
  dwords (`XOR EAX,EAX` at 0x0058B720 then `MOV [0x00ABE2F0..FC],EAX`, an init stub) —
  so the sentinel is the **{0,0,0,0} rect**, and since every shape caller passes a
  non-zero rect, **the ellipse path is always active** in modes 0/1/2. (verified via
  get_xrefs_to 0x00abe2f0 + get_assembly_context 0x0058b722, 2026-07-20)
- **Logical-to-diamond transform** used by all three shapes for a logical rect
  `(x0,y0,w,h)` with `W = DAT_0087F8DC`:
  `center_diamond_x = w/2 + h/2 + x0 + 1 + y0`,
  `center_diamond_y = h/2 - w/2 - x0 + W + y0` (all C-truncating int divides).
  (verified via disassemble_function 0x0059ad10 0x0059AE9E..0x0059AECB and
  0x0059b200 0x0059B3C9..0x0059B3F9, 2026-07-20)
- **Water-family tile gate** `0x004865D0` (project label `CellClass__HasBridgeOverlay`
  — the label is misleading in this context; the body reads `CellClass+0x38` and
  returns 1 iff the tile index is in `[g_ShorePieces, g_ShorePieces+0x2A)` (42 shore
  pieces, `g_ShorePieces @ 0x00ABAD28`) OR `[g_WaterSet_TileSetBase, +0x0E)`
  (`0x00AA0738`) OR one of four 4-tile runtime sets based at `DAT_00AA073C`,
  `DAT_00ABB110`, `DAT_00AA1050`, `DAT_00AA10A0`). Flood-fill growth and random seed
  selection require this to return 1 (cell still water/shore family). (verified via
  decompile_function 0x004865d0, 2026-07-20)
- **Shoreline tiler** `0x0057A0C0` (label `MapClass__MarkBridgesForRepair_High` — in
  this context it is the shore/water tiling+validation pass): four full-map
  cell-iterator passes calling per-cell workers (`UpdateBridgeTile_Low`,
  `ClearBridgeCell_Low`, `SelectBridgeTileVariant_Low` pass-1 and pass-2) with the blob
  id; allocates/frees the scratch array if absent; returns a success char. Worker
  internals NOT decoded this session (see §12.10). (verified via decompile_function
  0x0057a0c0, 2026-07-20)
- **`MapClass__Get_CellClass @ 0x005657A0`**: packed-short cell coord to
  `CellClass*` via `cells[(y*0x200)+x]` table at `this+0x13C` (receiver `0x0087F7E8`),
  with an out-of-range fallback dummy cell `&DAT_00ABDC50`. (verified via
  decompile_function 0x005657a0, 2026-07-20)
- **Gaussian source** `FUN_005980C0`, receiver `0x00ABDFB8` (`MOV ECX,0xABDFB8`
  before both calls in `FUN_0059BBC0` at 0x0059C2BC/0x0059C2C9): standard polar
  Box-Muller with cached second deviate — state byte at +0 (have-cached flag), cached
  double at +8, uniform-draw function pointer at +0x10. Consumes uniform pairs
  `u=2r-1` in a rejection loop (`s=u1^2+u2^2` in (0,1)), returns
  `sqrt(-2*ln(s)/s)*u1` (via `log2` and the `0.6931471805599453` fold), caches the
  `*u2` twin. The cache **persists across calls and across blobs** — draw parity
  matters for bit-exactness. (verified via decompile_function 0x005980c0 +
  disassemble_function 0x0059bbc0, 2026-07-20)

### 12.2 `FUN_0059BBC0` — land-blob flood fill (core routine)

Signature (thiscall, `ECX = MapSeed`, `RET 0x20`):
`(int max_cells, int* rect4, int* seed_xy, int ellipse_mode, i16pair* target,
double drift_scale, char directed)` — callers pass `ellipse_mode=1` always;
`drift_scale` = 0.25 (mode 0) or 0.75 (modes 1/2); `directed` = 0 (mode 0) or 1
(modes 1/2). Returns the number of popped cells, or 0 on failure/rollback.
(verified via decompile_function + disassemble_function 0x0059bbc0, 2026-07-20)

**Setup.** `max_cells = max(max_cells, 400)` (`CMP EAX,0x190` at 0x0059BBD1);
node capacity `cap = max(max_cells*8+2, 100)`; allocates a node pool of `cap` 8-byte
nodes `{packed i16 coord dword, float32 key}` (bump-allocated, never freed
mid-run) and a **1-indexed binary min-heap** object `{count, cap, ptr-array[cap+1],
maxptr, minptr}` keyed on the float32 at node+4. Pop = take `heap[1]`, move
`heap[count]` to slot 1, `count--`, sift-down via `FUN_005AD870`. Sift-down prefers a
child only on **strict** key `<` (ties keep parent, left child checked first);
sift-up (inlined at 0x0059BF26/0x0059C25C) moves the parent down only while
`key[parent] > new` (FCOMP `TEST AH,0x41` — stops on less-or-equal). So the queue
discipline is a **binary min-heap priority queue**, NOT a stack or FIFO. (verified via
decompile_function 0x005ad870 + disassemble_function 0x0059bbc0, 2026-07-20)

**Seed selection** (only when `seed_xy == (0,0)`; modes 1/2 pass a real seed for the
first call, mode 0 always passes a real center): rejection loop of at most **200**
attempts (`CMP EDX,0xC8` at 0x0059BD7E; on the 200th failure return 0):
1. draw `d1` from `g_MapGenRng` (`MOV ECX,0xABE890` at 0x0059BCAC), chain
   `FILD u64(d1); FMUL double(W); FMUL [0x007ED898](2^-32); ftol` with
   `W=DAT_0087F8DC`, redraw while result > W-1 (defensive; cannot fire);
2. same for `d2` scaled by `H=DAT_0087F8E0` (site 0x0059BD01);
3. seed diamond coords: `x = d1 + d2 + 1`, `y = (W - d1) + d2` (16-bit adds at
   0x0059BD31..0x0059BD6C);
4. accept iff scratch exists, `scratch[seed]+0x38 == 0`, and the seed cell passes the
   water-family gate `0x004865D0`; else count the attempt and redraw.
(verified via disassemble_function 0x0059bbc0 0x0059BC89..0x0059BDD1, 2026-07-20)

**Aspect weights** (computed once; only when `rect4 != {0,0,0,0}`): with `w=rect[2]`,
`h=rect[3]`: if `h < w` then `wx=1.0f`, `wy=float((w FIDIV h)*1.2)`; else `wy=1.0f`,
`wx=float((h FIDIV w)*1.2)` — `FILD larger; FIDIV smaller; FMUL [0x007E5190](1.2);
FSTP float`. Ellipse coefficients (always): `A = 1.0/((w*0.5)*(w*0.5))`,
`B = 1.0/((h*0.5)*(h*0.5))` via `FILD; FMUL [0x007E1738](0.5); FLD ST0; FMUL ST1;
FDIVR [0x007E1718](1.0)` (0x0059BFA2..0x0059BFD6). (verified via disassemble_function
0x0059bbc0, 2026-07-20)

**Initialization:** drifting center doubles `cx,cy` = seed coords;
`scratch[seed]+0x3C = blob_id`; seed node pushed with key 0.0f and popped immediately.
(verified via decompile_function 0x0059bbc0, 2026-07-20)

**Main loop** — runs while `pops < max_cells` and queue non-empty; per popped node P:
1. Commit P: `scratch[P]+0x38 = blob_id`; `CellClass(P)+0x38 = 0` (land).
2. Neighbor scan over `g_DirectionOffsets (0x0089F688)` indices **0,2,4,6 only**
   (N, E, S, W; `dir += 2` loop 0x0059C2A8..0x0059C2B6). For each neighbor C:
   accept iff diamond-bounds pass AND `scratch[C]+0x38 == 0` AND
   `scratch[C]+0x3C != blob_id` AND water-family gate passes AND node pool not
   exhausted AND ellipse test `FUN_0059BAB0(C, rect4, 1, A, B)` passes. Then:
   - rounded center `rx = ftol(cx + 0.5)`, `ry = ftol(cy + 0.5)` (`FADD
     [0x007E1738](0.5)` at 0x0059C156/0x0059C167 — recomputed per accepted neighbor);
   - key: if `directed == 0`:
     `key = float( sqrt((Cx-rx)^2 + (Cy-ry)^2) + draw*5.0*2^-32 )` — one
     `g_MapGenRng` draw per accepted neighbor (site 0x0059C1EE, `MOV ECX,0xABE890`),
     chain `FILD u64(draw); FMUL [0x007ED808](5.0); FMUL [0x007ED898](2^-32);
     FADD sqrt` (`Sqrt_Approx @ 0x004CAC40` on the integer sum-of-squares);
     if `directed == 1`: `key = float(FUN_0059B940(&(rx,ry), &C, rect4, target, wx,
     wy))` — **no RNG draw** (see §12.9.2);
   - `scratch[C]+0x3C = blob_id`; node appended `{C, key}` and heap-inserted (the
     insert is skipped silently if `count+1 >= cap` — the cell then stays marked and
     is never revisited).
3. Center drift (once per pop, after the 4-neighbor scan): **two** Gaussian calls
   `cx += gauss()*drift_scale; cy += gauss()*drift_scale` (`FMUL double [EBP+0x1C]` =
   the caller's drift_scale; sites 0x0059C2BC..0x0059C2E3). Because Box-Muller caches,
   this consumes 2 uniform draws (plus rejection redraws) per uncached call and 0 per
   cached call.
4. Pop next.
(verified via disassemble_function 0x0059bbc0 0x0059BFE2..0x0059C32C, 2026-07-20)

**Drain phase** (queue non-empty when budget hit): pop every remaining node; for each,
if `scratch+0x38 == 0` and the water-family gate still passes, then
`CellClass+0x38 = 0` and `scratch+0x38 = blob_id`; the pop counter increments for
every drained node regardless. No draws in this phase. (verified via
disassemble_function 0x0059bbc0 0x0059C332..0x0059C3F6, 2026-07-20)

**Epilogue:** free pool+heap; call shoreline tiler `0x0057A0C0(blob_id, 1)`; then a
full-map pass resets every cell whose tile is in `[g_ShorePieces, +0x2A)` to tile 0
with `+0x11A = 0` (shore pieces are transient at this stage). If the tiler returned
success: `MapSeed+0x308++`, return pop count. If it failed: **rollback** — every cell
whose `scratch+0x38 == blob_id` gets `scratch+0x38 = 0`, `scratch+0x4B = 0`,
`CellClass+0x38 = g_WaterSet_TileSetBase (0x00AA0738)`, `+0x11A = 0`,
`+0x11B = byte(MapSeed+0x30C)`; return 0. (verified via decompile_function +
disassemble_function 0x0059bbc0 0x0059C3FC..0x0059C52C, 2026-07-20)

### 12.3 `FUN_0059A8F0` — island partition grid builder (mode 0 only)

Args: `(vector16* islands_out, int n, int* rect4)` where rect4 = the playable rect
`{DAT_0087F8E4, E8, EC, F0}` and `n` = island count. (verified via decompile_function
0x0059a8f0 + disassemble_function 0x0059ad10, 2026-07-20)

1. `s = ftol(Sqrt_Approx((double)n))`; `C = (s*s == n) ? s : s+1` (slots per strip);
   `R = (n <= C*s) ? s : C` (strip count); `leftover = R*C - n`.
2. Initial cell size: `cw = rect.w / C`, `ch = rect.h / C` (truncating idiv).
3. **Orientation draw** (1x `g_MapGenRng`, site 0x0059A97C): `FILD u64(draw); FMUL
   [0x007ED898](2^-32); FCOMP [0x007E1738](0.5)`. If `draw*2^-32 < 0.5` then
   **column strips**: `cw = rect.w / R`, within-strip step `(0, ch)`, strip advance
   `(cw, 0)`; else **row strips**: `ch = rect.h / R`, within-strip step `(cw, 0)`,
   strip advance `(0, ch)`.
4. Two int vectors (ctor `FUN_00477BE0(0,0)`, vtable `0x007E4E78` whose slot +8 =
   grow fn `0x00477E10`, growth 10; verified via read_memory 0x007e4e78): list A =
   per-strip slot counts (R entries, each = C), list B = strip indices `0..R-1`.
5. **Leftover removal** — `leftover` iterations; each: rejection draw
   `idx = ftol(draw * len(B) * 2^-32)` (chain `FILD u64(draw); FMUL double(lenB);
   FMUL 2^-32`, redraw while `idx > lenB-1`, site 0x0059AB02), set `A[B[idx]] = s`
   (that strip loses `C-s` slots — one slot when `C = s+1`), then remove `B[idx]`
   by shift-left. Each strip can be shortened at most once.
6. **Emission** — for strip i in `0..R-1` (in order): `cnt = A[i]`; if `cnt < C`, the
   strip start is offset by half a cell along the strip axis (`x += cw/2` for rows,
   `y += ch/2` for columns); then `cnt` entries are appended to `islands_out`:
   `{x+2, y+2, cw-4, ch-4}` (16-byte logical rects), stepping by the within-strip
   step; after the strip, advance by the strip step and reset the along-axis
   coordinate to the rect origin. Emission order is strictly strip-major,
   slot-in-strip minor.
(all verified via disassemble_function 0x0059a8f0, 2026-07-20)

Draw count: exactly `1 + leftover` draws (plus cannot-fire rejection redraws).

### 12.4 `FUN_005ADA40` — 16-byte-entry dynamic vector ctor (NOT a shape initializer)

`FUN_005ADA40(cap, buf)` on a 6-dword object: `{vtable=0x007ED970-table, data, cap,
flag byte, owns byte (+0xD), count (+0x10), growth (+0x14)}`; allocates `cap<<4` bytes
when `cap>0 && buf==0`. `FUN_0059AD10` calls it with `(0,0)` (empty vector, growth
later set to 10, vtable switched to `0x007ED99C`) to hold the island rects appended by
`FUN_0059A8F0` via vtable slot +8. The "island shape initializer" description in
earlier notes is wrong — it is a generic DynamicVector ctor for 16-byte elements.
(verified via decompile_function 0x005ada40 + disassemble_function 0x0059ad10,
2026-07-20)

### 12.5 `FUN_0059AD10` — archipelago (MapType 0), complete

1. `MapSeed+0x308 = 1`; empty island vector constructed (§12.4).
2. `m = max(2, MapSeed[+0x50]/2)` (`+0x50` = NumPlayers). **Extra-island draw**
   (1x `g_MapGenRng`, site 0x0059AD6E): rejection chain `extra = ftol(draw*m*2^-32 +
   1.0)` (`FMUL double(m); FMUL [0x007ED898]; FADD [0x007E1718](1.0)`), redraw while
   `extra > m` (cannot fire: floor <= m-1, +1 <= m), so `extra` is in `[1, m]`.
3. `FUN_0059A8F0(&vec, NumPlayers + extra, playable_rect)` builds the island rect
   list.
4. Reset pass: every cell's `scratch+0x3C = 0` (cell-iterator loop, receiver
   0x0087F7E8).
5. While the vector is non-empty, take **entry[0]** (pop-front; after processing, all
   remaining entries shift down by 0x10 — order is exactly the §12.3 emission order):
   - **Blob-size draw** (1x `g_MapGenRng` per entry, site 0x0059AE5B):
     `size = ftol((draw*2^-32*0.05 + 0.45) * (2*w*h))` — chain `FILD u64(draw);
     FMUL [0x007ED898](2^-32); FMUL [0x007E8AE8](0.05); FADD [0x007ED990](0.45);
     FILD int(w*h*2); FMULP` — uniform in `[0.9, 1.0) * (w*h)`.
   - Compute the entry's diamond center (§12.1 transform).
   - Up to **10 attempts** (`CMP ESI,0xA`): `FUN_0059BBC0(size, &entry_rect,
     &center_xy, 1, &packed_center, 0.25, 0)` — undirected mode, drift 0.25
     (`PUSH 0x0 / PUSH 0x3FD00000` = double 0.25), seed always the entry center (no
     rejection sampling), size drawn ONCE per entry (not per attempt). Break on
     nonzero return; on zero return `MapSeed+0x308++` and retry.
6. There is no island-center rejection-sample loop in this function — the only
   rejection loops are the two defensive draw chains above; island positions come
   deterministically from the §12.3 grid.
(all verified via disassemble_function 0x0059ad10, 2026-07-20)

### 12.6 `FUN_0059AFA0` — continental (MapType 1), complete

1. `MapSeed+0x308 = 1`; `area = FUN_0042B1F0() = (DAT_0087F8E0 + 4) * DAT_0087F8DC
   * 2` (verified via decompile_function 0x0042b1f0).
2. **Land-fraction target** (exact FP order, 0x0059AFC2..0x0059AFFB):
   `t = 1.0 - FILD(MapSeed[+0x4C]) * [0x007E3808](0.01)` (FSUBR), then
   `target = (FLD [0x007ED728](0.5) FSUB [0x007ED720](0.45)) * t + 0.45` — i.e.
   `target = (0.5-0.45)*(1 - Water*0.01) + 0.45`, the max-min difference computed at
   runtime by FSUB (about 0.04999999999999999). Water=0 gives 0.5; Water=100 gives
   0.45.
3. Per-call growth cap: `cap = ftol(area * [0x007ED730](0.03) * target)`
   (`FILD area; FST double; FMUL 0.03; FMUL target`).
4. Center: `c = DAT_0087F8E0/2 + DAT_0087F8DC/2`; first seed and permanent target =
   `(c+1, c)` (diamond). Reset pass: all `scratch+0x3C = 0`.
5. Loop while `placed_fraction < target` (FCOMP; also gated `0.0 < target` via
   `[0x007E2800](0.0)`) and `< 100` calls (`CMP [ESP+0x18],0x64` at 0x0059B0D6):
   - `n = min(ftol((target - fraction) * area), cap)`;
   - `FUN_0059BBC0(n, &rect{E4+1, E8+1, EC-2, F0-2}, &seed_xy, 1,
     &packed(c+1,c), 0.75, 1)` — directed mode, drift 0.75 (`PUSH 0x3FE80000`),
     target always the fixed map center;
   - `placed += return`; `fraction = FILD(placed) FDIV area`;
   - **nearest-cell scan** for the next seed: cell-iterator order (receiver
     0x0087F7E8, anti-diagonal scan), skip cells with `scratch+0x3C != 0`, metric
     `|cell.y - c| + |cell.x - (c+1)|` (Manhattan, fixed center), strict `<` against
     best (init 50000) — first-in-iterator-order wins ties. Result (packed, or (0,0)
     if none) becomes the next call's seed; (0,0) triggers BBC0's random-seed path.
6. No direct RNG draws at this level (§3.2 note stands).
(all verified via disassemble_function 0x0059afa0, 2026-07-20)

### 12.7 `FUN_0059B200` — islands-in-sea (MapType 2), complete

1. `MapSeed+0x308 = 1`; `area` as §12.6.
2. **Per-landmass land-fraction target** (0x0059B225..0x0059B252, same FP shape as
   §12.6): `target = ([0x007ED748](0.2) - [0x007ED740](0.15)) * (1 - Water*0.01)
   + 0.15` (runtime FSUB, about 0.05000000000000002). Per-call cap
   `= ftol(area * [0x007ED750](0.06) * target)`.
3. Reset pass: all `scratch+0x3C = 0`.
4. **Split-axis draw** (1x `g_MapGenRng`, site 0x0059B2C4): `FILD u64(draw);
   FMUL [0x007ED898](2^-32); FCOMP [0x007E1738](0.5)`. With playable rect
   `(X,Y,Wp,Hp) = DAT_0087F8E4/E8/EC/F0`:
   - `draw*2^-32 < 0.5` — **left/right** halves: rect1 = `{X, Y, Wp/2-1, Hp}`,
     rect2 = `{X + Wp/2 + 1, Y, Wp/2-1, Hp}`;
   - `>= 0.5` — **top/bottom** halves: rect1 = `{X, Y, Wp, Hp/2-1}`,
     rect2 = `{X, Y + Hp/2 + 1, Wp, Hp/2-1}`.
5. For each rect (rect1 then rect2; per-rect placed-count and fraction reset, call
   counter SHARED, see §3.3 correction): compute the diamond center (§12.1);
   aspect weights: `w < h` gives `wLx = 1.0`, `wLy = (FILD h; FDIV w)*1.2`; else
   `wLy = 1.0`, `wLx = (w FDIV h)*1.2` (doubles here, unlike BBC0's floats);
   ellipse coefficients `A = 1/((w*0.5)^2)`, `B = 1/((h*0.5)^2)` (same FDIVR-1.0
   chain).
6. Inner loop while `fraction < target` and shared counter `< 100`:
   - `n = min(ftol((target - fraction)*area), cap)`;
   - `FUN_0059BBC0(n, &rect_i, &seed_xy, 1, &packed_center, 0.75, 1)` — directed,
     drift 0.75; first seed = rect center;
   - **next-seed scan**: LINEAR walk of the scratch array (index 0..width^2-1, i.e.
     y-major/x-minor by scratch layout — NOT the cell-iterator order used in mode 1),
     reading each entry's own `+0x00/+0x02` coords; candidate must satisfy the
     diamond window `x+y` in `[W+2*ry+1, W+2*ry+2h+3]`, `x-y` in
     `[2*rx-W+1, 2*rx-W+2w+3]` (rect_i in logical coords, `W=DAT_0087F8DC`),
     `scratch+0x3C == 0`, AND the ellipse test `FUN_0059BAB0(entry, &center, 1, A,
     B)`; metric `ftol(|y-cy|*wLy + |x-cx|*wLx)` (FILD |dy|; FMUL wLy; FILD |dx|;
     FMUL wLx; FADDP; ftol), strict `<` vs best (init 50000), first-wins ties in
     linear order. If none found the seed becomes (0,0) — random-seed path.
(all verified via disassemble_function 0x0059b200, 2026-07-20)

### 12.8 FP constants (read_memory, 2026-07-20)

| Address | u64 bit pattern | Value | Used by |
|---|---|---|---|
| `0x007ED898` | `0x3DF0000000100000` | ~2^-32 (perturbed mantissa!) | all draw scalings |
| `0x007ED808` | `0x4014000000000000` | 5.0 | BBC0 undirected jitter |
| `0x007E5190` | `0x3FF3333333333333` | 1.2 | aspect weights (BBC0, B200) |
| `0x007E1738` | `0x3FE0000000000000` | 0.5 | half-cell, round-nearest, 0.5-threshold |
| `0x007E1718` | `0x3FF0000000000000` | 1.0 | FDIVR, +1.0, ellipse compare |
| `0x007E3808` | `0x3F847AE147AE147B` | 0.01 | WaterAmount % scale |
| `0x007E2800` | `0x0000000000000000` | 0.0 | target>0 gates |
| `0x007ED720` | `0x3FDCCCCCCCCCCCCD` | 0.45 | mode-1 min land fraction |
| `0x007ED728` | `0x3FE0000000000000` | 0.5 | mode-1 max land fraction |
| `0x007ED730` | `0x3F9EB851EB851EB8` | 0.03 | mode-1 per-call cap factor |
| `0x007ED740` | `0x3FC3333333333333` | 0.15 | mode-2 min land fraction |
| `0x007ED748` | `0x3FC999999999999A` | 0.2 | mode-2 max land fraction |
| `0x007ED750` | `0x3FAEB851EB851EB8` | 0.06 | mode-2 per-call cap factor |
| `0x007E8AE8` | `0x3FA999999999999A` | 0.05 | mode-0 blob-size span |
| `0x007ED990` | `0x3FDCCCCCCCCCCCCD` | 0.45 | mode-0 blob-size base |
| `0x007E1748` | `0x00000000` (float32) | 0.0f | B940 zero-distance return |
| immediate | `0x3FD0000000000000` | 0.25 | mode-0 drift scale (pushed) |
| immediate | `0x3FE8000000000000` | 0.75 | mode-1/2 drift scale (pushed) |

`0x007ED898` carries the same perturbed-mantissa pattern documented in §5 — use the
literal bit pattern, never a clean `2^-32`.

### 12.9 Leaf helpers

**12.9.1 `FUN_0059BAB0` — region membership test.** `(i16pair* cell, int* rect4,
char ellipse_mode, double A, double B)`. Sentinel rect returns 1. Ellipse mode (the
only mode reached; `ellipse_mode=1` from all callers): with `W = DAT_0087F8DC`,
`t1 = cell.x - 2*rect.x - cell.y + W - 1`, `t2 = cell.x - 2*rect.y - W + cell.y - 1`
(integer), `dx = t1*0.5 - rect.w*0.5`, `dy = t2*0.5 - rect.h*0.5` (each FILD;
FMUL [0x007E1738]; FSUBP), pass iff `dy^2*B + dx^2*A < 1.0` (FLD/FMUL ST; FMUL B;
FLD/FMUL ST; FMUL A; FADDP; FCOMP [0x007E1718]). The rect-membership branch
(`ellipse_mode==0`) is dead in the water-shape paths. (verified via
disassemble_function 0x0059bab0, 2026-07-20)

**12.9.2 `FUN_0059B940` — directed-growth key.** `(i16pair* center, i16pair* cand,
int* rect4, i16pair* target, float wx, float wy)`. Sentinel rect gives plain
`Sqrt_Approx(euclid^2(center, cand))`. Otherwise: logical deltas
`dLx = |((cand.x-cand.y)>>1) - ((tgt.x-tgt.y)>>1)|`,
`dLy = |((cand.x+cand.y)>>1) - ((tgt.x+tgt.y)>>1)|` (SAR = floor-shift); if both 0,
returns 0.0f (`FLD float [0x007E1748]`). Else
`norm = Sqrt_Approx((double)(dLy^2 + dLx^2))`;
`cheb = max(|cand.x-center.x|, |cand.y-center.y|)` (diamond frame);
`key = cheb * ( (dLy/norm)*wy + wx*(dLx/norm) )` — exact x87 order:
`FILD dLx; FDIV norm; FSTP tmp; FIDIVR dLy (dLy/norm); FMUL float wy; FLD float wx;
FMUL double tmp; FADDP; FILD cheb; FMULP`. No RNG. (verified via
disassemble_function 0x0059b940, 2026-07-20)

**12.9.3 `FUN_005AD870` — heap sift-down.** 1-indexed; a child is chosen only on
strict key `<` (left checked before right); swap loop repeats until no child is
smaller. (verified via decompile_function 0x005ad870, 2026-07-20)

### 12.10 Unverified (YELLOW)

- ~~**Shoreline-tiler worker internals**~~ — RESOLVED 2026-07-20: fully decoded in
  §13 (verdict logic, tile writes, tables, draw counts).
- **The four 4-tile runtime sets** in the water-family gate (`DAT_00AA073C`,
  `DAT_00ABB110`, `DAT_00AA1050`, `DAT_00AA10A0`): which tilesets these hold at .SED
  generation time (and whether tile 0 falls inside one, letting the gate accept
  clear/land cells) is runtime data — unverified statically.
- **Box-Muller uniform source binding**: the draw goes through the function pointer at
  `0x00ABDFB8+0x10` (runtime-installed); its binding to `g_MapGenRng` is per the
  existing Box-Muller/RMG x87 report and repo port, not re-verified this session.
- **`Sqrt_Approx (0x004CAC40)` rounding behavior** under CW 0x0E7F is taken from the
  prior RMG x87 report (table-sqrt port already landed), not re-derived here.
- **`MapSeed+0x30C` semantics** (byte written to `CellClass+0x11B` on rollback):
  field meaning not traced this session.

---

## 13. Shore Tiler 0x0057A0C0 (decoded 2026-07-20)

Resolves the first YELLOW item of §12.10: full decode of the shore tiler and every
worker that contributes to its commit/rollback verdict. All claims verified live this
session; label names in the Ghidra project for this whole family
(`MapClass__MarkBridgesForRepair_High`, `*BridgeTile*`, `IsBridgeDeckTile`) are
**drift** — the actual subject is the theater ShorePieces tileset, not bridges.

### 13.1 Signature, callers, pass structure

`0x0057A0C0` is `__thiscall (this = MapClass 0x0087F7E8, int region_id, char
keep_flag)`, `RET 0x8`, returns a success char in AL (verified via
disassemble_function 0x0057a0c0, 2026-07-20; caller `FUN_0059BBC0` passes
`(blob_id, 1)` per §12.2). Xrefs: called ONLY from the five RMG shape/carver
routines `FUN_0059A6C0` (0x0059A788), `FUN_0059BBC0` (0x0059C433), `FUN_0059C920`
(0x0059D282), `FUN_0059D510` (0x0059E260), `FUN_0059E740` (0x0059EC86) — **RMG-only;
not shared with normal map loading or WAE map load** (verified via get_xrefs_to
0x0057a0c0, 2026-07-20). The MODE34 report documents the two argument conventions:
drivers call `(id, 0)` (region-checked final tiling), water-seed/flood-fill call
`(id, 1)` (region checks bypassed).

Body (verified via decompile_function + disassemble_function 0x0057a0c0,
2026-07-20):

1. `FUN_004A8BF0(ECX=0x0087F7E8, 0)` — clears the placement-footprint state
   (`this+0x117C = 0`, `+0x1178 = DAT_008A03F8` invalid marker). This forces the
   later anchor-setter `FUN_004A91B0` onto its fast path (see §13.5, step "apply").
   (verified via decompile_function 0x004a8bf0 / 0x004a91b0, 2026-07-20)
2. Scratch array `DAT_00ABED10` (stride 0x50): if null, allocate `W*W*0x50` bytes
   (`W = [0x0089C2DC]`, the linear map width global) and default-construct every
   entry via `FUN_0058BDC0` (all zero except `+0x40 = 0xFFFFFFFF`, `+0x4A = 1`);
   remember a local "allocated here" flag and free (`FUN_007C8B3D`) + null the array
   in the epilogue if set. In the RMG pipeline the array already exists, so this
   branch is normally dead. (verified via decompile_function 0x0057a0c0 +
   0x0058bdc0, 2026-07-20)
3. Reset pass: for every linear index `i < W*W`, `scratch[i]+0x40 = -1` (cached
   shore mask invalidated; accessor `FUN_0058C2C0(ECX=i) = i*0x50 + [0x00ABED10]`).
   (verified via disassemble_function 0x0057a0c0 0x0057A12B..0x0057A14C +
   0x0058c2c0, 2026-07-20)
4. Four full-map passes. Before each, the MapClass row iterator is reset identically:
   `[0x0087F8F4]=1, [0x0087F8F8]=W_map, [0x0087F8FC]=W_map-1,
   [0x0087F900]=W_map*0x800 + [0x0087F924] + 4` with `W_map = [0x0087F8DC]`, then
   cells come from `MapClass__CellIterator_Next 0x00578290` (ECX=0x0087F7E8) until
   it returns 0:
   - **Pass A** `MapClass__UpdateBridgeTile_Low 0x0057A430(cell, id, flag)` — result
     char kept in BL; the loop aborts as soon as a call returns 0 (remaining cells
     unvisited).
   - **Pass B** `MapClass__ClearBridgeCell_Low 0x0057A320(cell, id, flag)` — void;
     only runs if BL is still 1; does not change BL.
   - **Pass C** `MapClass__SelectBridgeTileVariant_Low 0x0057ACF0(cell, 1, id,
     flag)` — BL = result, abort on 0.
   - **Pass D** same with variant `2`.
5. Epilogue: `[0x00880990] = 0` (the "current tile type being stamped" temp — the
   project label `g_UIModeLock` on this global is drift); if `[0x0088098C]` holds an
   object, call its vtable+0x20 with arg 1 and null it (not populated on the RMG
   path — see §13.11); free scratch if locally allocated; return BL.
   (verified via disassemble_function 0x0057a0c0 0x0057A2C4..0x0057A310, 2026-07-20)

So the tiler's verdict = AND of every Pass A call and every Pass C/D call, in
iterator order, with early abort.

### 13.2 Neighbor-water mask — `MapClass__ComputeBridgeSurfaceMask 0x0057B210`

`__thiscall (this=0x0087F7E8, CellClass* cell, int mode)` (verified via
disassemble_function 0x0057b210, 2026-07-20).

- Water predicate used throughout: `0x00485060` returns 1 iff `cell+0x38` is in
  `[g_WaterSet_TileSetBase, +0x0E)` with `g_WaterSet_TileSetBase = [0x00AA0738]`
  (verified via decompile_function 0x00485060, 2026-07-20). This is 14 water tiles —
  narrower than the §12.1 water-family gate `0x004865D0`.
- Clear predicate: `0x00486380` returns 1 iff `cell+0x38 == 0 || == 0xFFFF`
  (verified via decompile_function 0x00486380, 2026-07-20).

Flow:
1. `self_water = IsWater(cell)` (computed first, used for missing neighbors).
2. Diamond bounds vs the RMG bounds globals: require `x+y > [0x00ABED04]` AND
   `x-y < [0x00ABED04]` AND `y-x < [0x00ABED04]` AND `x+y <= [0x00ABED08]`
   (x,y = signed 16-bit halves of `cell+0x24`), else return 0.
3. Mode gates: `mode==0` → require `IsClear(cell)` else return 0. `mode==1` →
   return 0 if `IsWater(cell)`. `mode==2` → no gate.
4. `scratch = FUN_0058C2A0(ECX=&coord)` = `(y*[0x0089C2DC] + x)*0x50 +
   [0x00ABED10]`; require `scratch+0x4A != 0` else return 0.
5. **Cache: if `scratch+0x40 >= 0`, return it unchanged** — the cached value is
   returned for ANY mode once the mode gates pass. The mode only changes the
   prechecks; the mask payload is shared. This matters: Pass A stores the mode-2
   mask into `+0x40` before asking for the mode-0 mask of the same cell — the
   second call returns the cached mode-2 value (gated by IsClear).
6. Otherwise compute (NOT stored back here; only Pass A stores): for each of the 8
   neighbors, from the cell-pointer array `[this+0x13C] = [0x0087F924]` indexed
   `y*0x200+x`:
   `bit = (neighbor_ptr != 0 && IsWater(neighbor)) || (neighbor_ptr == 0 &&
   self_water)`.
   Bit layout (+X east, +Y south): **N(x,y-1)=0x80, NE(x+1,y-1)=0x01, E(x+1,y)=0x02,
   SE(x+1,y+1)=0x04, S(x,y+1)=0x08, SW(x-1,y+1)=0x10, W(x-1,y)=0x20,
   NW(x-1,y-1)=0x40** (offsets -0x804, -0x800, -0x7FC, -4, +4, +0x7FC, +0x800,
   +0x804 bytes from the cell's slot; verified via disassemble_function 0x0057b210
   0x0057B2EA..0x0057B429, 2026-07-20).

### 13.3 Pass A — land erosion, `0x0057A430(cell, id, flag)`

(verified via decompile_function 0x0057a430, 2026-07-20)

1. `scratch(cell)+0x40 = ComputeMask(cell, 2)` — caches the unconditional mask.
2. `m = ComputeMask(cell, 0)` — via the cache this equals the mode-2 mask if the
   cell is clear-tile, else 0. If `m <= 0`: return 1 (nothing to do).
3. Detect "problem" shapes (`bad = true` if any):
   - `m == 0x0B` (NE|E|S) or `m == 0x1A` (E|S|SW);
   - `(m&0xA0)==0xA0` (N and W) and `(m&0x11)==0` (no NE, no SW);
   - `(m&0x82)==0x82` (N and E) and `(m&0x44)==0` (no NW, no SE);
   - `(m&0x0A)==0x0A` (E and S) and `(m&0x11)==0`;
   - `(m&0x28)==0x28` (S and W) and `(m&0x44)==0`;
   - W set: if mask(mode 0) of `(x+1,y)` or `(x+2,y)` has E-bit → bad (1-cell-thin
     vertical land strip); E set: `(x-1,y)`/`(x-2,y)` W-bit; S set:
     `(x,y-1)`/`(x,y-2)` N-bit; N set: `(x,y+1)`/`(x,y+2)` S-bit. Probes use the
     `[0x0087F924]` array with the dummy-cell fallback `&DAT_00ABDC50`
     (`[0x00ABDC74]=coord`).
4. Flood trigger: `(m&0x88)==0x88` (water both N and S) OR `(m&0x22)==0x22` (both
   E and W) OR `bad`:
   - `owner = FUN_005A00C0(&coord)` = `scratch(coord)+0x38` (region id;
     verified via decompile_function 0x005a00c0, 2026-07-20).
   - **FAILURE: if `owner > 0 && owner != id && flag == 0` → return 0.** This is
     Pass A's only failure: eroding this land cell would eat a cell owned by a
     different region. With `flag != 0` (water-seed/flood-fill calls) Pass A can
     NEVER fail.
   - Otherwise convert the cell to water: `cell+0x11A = 0`, `cell+0x38 =
     g_WaterSet_TileSetBase`, and `FUN_005A0090(&coord, flag ? 0 : id)` (region-id
     write, verified via decompile_function 0x005a0090, 2026-07-20).
   - Then for all 8 directions (`MapCoord_StepByDir_GetCell 0x00481810`, which adds
     `g_DirectionOffsets[dir]` from 0x0089F688 in N,NE,E,SE,S,SW,W,NW order and
     resolves via `MapClass__Get_CellClass`): if the neighbor passes the
     `[0x00ABED04]/[0x00ABED08]` diamond test, invalidate its cached mask
     (`scratch+0x40 = -1`) and **recurse** `UpdateBridgeTile_Low(neighbor, id,
     flag)`. **The recursion's return value is discarded** — a foreign-region
     failure inside the recursion does not fail the pass; it can only fail when the
     top-level iterator reaches such a cell itself (order-dependent). (verified via
     decompile_function 0x0057a430 — call result unused, 2026-07-20)
5. Return 1.

### 13.4 Pass B — thin-water cleanup, `0x0057A320(cell, id, flag)`

(verified via decompile_function 0x0057a320, 2026-07-20)

Only acts if `cell+0x38` is in `[g_WaterSet_TileSetBase, +0x0C)` (note **0x0C**, not
the 0x0E of the water predicate — the last two water tiles are exempt) AND
`ComputeMask(cell, 2)` is one of exactly
`{0xC7, 0x7C, 0xF1, 0x1F, 0xC6, 0x6C, 0xB1, 0x1B}` (water-spike patterns). Then:
- `cell+0x38 = 0xFFFF`, `cell+0x11A = 0` (revert to empty/clear);
- if `flag != 0`: `scratch(cell)+0x38 = id` (the un-watered cell keeps region
  ownership — only in keep-flag mode);
- for all 8 neighbors inside the diamond: recompute and store
  `scratch+0x40 = ComputeMask(neighbor, 2)` (cache refresh; `+0x40` is first set to
  -1 so the call recomputes).
Never fails.

### 13.5 Passes C/D — piece selection, `0x0057ACF0(cell, variant, id, flag)`

(verified via decompile_function + disassemble_function 0x0057acf0, 2026-07-20)

**RNG first:** the function's first action — before any mask work — is
`r = FUN_00598030(ECX=0, EDX=5)` (`MOV EDX,0x5; XOR ECX,ECX` at 0x0057ACF9). So
**every cell the iterator yields in Pass C and again in Pass D consumes one bounded
draw**, even cells that immediately return 1 with mask 0. `FUN_00598030(lo,hi)` is
the RMG bounded-uniform helper: loop `{ raw = Random__Next(ECX=0x00ABE890);
val = ftol( (double)raw * (double)(hi-lo+1) * [0x007ED898] + (double)lo ); } while
(val > hi)` — x87 order FILD u64(raw); FMUL range; FMUL K; FADD lo; CALL _ftol
(0x007C5F00, truncating), unsigned compare `JA` for the rejection (verified via
disassemble_function 0x00598030, 2026-07-20). `[0x007ED898]` is the perturbed
2^-32 of §12.8 (bytes `00 00 10 00 00 00 F0 3D` re-verified via read_memory
0x007ED898, 2026-07-20). With lo=0, hi=5 the rejection can only fire on
`raw == 0xFFFFFFFF`. `Random__Next 0x0065C780` body re-verified as the 250-entry
lagged XOR generator (state: flag byte +0, idx1 +4, idx2 +8, table +0xC; verified
via decompile_function 0x0065c780, 2026-07-20).

Then `m = ComputeMask(cell, variant==2 ? 1 : 0)`; if 0 → return 1.

**Pass D (variant 2) — inner corners** (mode-1 mask: skips water cells):
| condition | piece (1-based) |
|---|---|
| `(m&0xA0)==0xA0`, `(m&0x11)==0x11` | `(r&1)+0x17` (23/24) |
| `(m&0xA0)==0xA0`, else | NE set → 0x15, else 0x1E |
| `(m&0x82)==0x82`, `(m&0x44)==0x44` | `(r&1)+0x0F` (15/16) |
| `(m&0x82)==0x82`, else | SE set → 0x0E, else 0x16 |
| `(m&0x0A)==0x0A`, `(m&0x11)==0x11` | `(r&1)+7` (7/8) |
| `(m&0x0A)==0x0A`, else | NE set → 0x0D, else 6 |
| `(m&0x28)==0x28`, `(m&0x44)==0x44` | `(r&1)+0x1F` (31/32) |
| `(m&0x28)==0x28`, else | SE set → 5, else 0x1D |
| none | return 1 |
(tests in that order: N+W, N+E, S+E, S+W.)

**Pass C (variant 1) — straight shores and outer corners** (mode-0 mask: only
clear-tile cells get a nonzero mask). For the first matching cardinal bit, walk the
shoreline to measure run parity, `len` starting at 1:
- E (`m&0x02`): `a=Step(cell,S); b=Step(a,E)`; while `!IsWater(a) && IsWater(b)`:
  `len++; a=Step(a,S); b=Step(b,S)`. Piece: `(len odd && !(m&0x80))` → 0x0C;
  `!(m&0x04)` → 0x0C; `(m&0x18)` → 0x0C; else `r%3 + 9` (9/10/11).
- W (`m&0x20`): `a=Step(cell,S); b=Step(a,W)`; advance S. `(len odd && !(m&0x80))`
  → 0x1C; `!(m&0x10)` → 0x1C; `(m&0x0C)` → 0x1C; else `r%3 + 0x19` (25/26/27).
- S (`m&0x08`): `a=Step(cell,E); b=Step(a,S)`; advance E. `(len odd)` → 4;
  `!(m&0x04)` → 4; `(m&0x03)` → 4; else `r%3 + 1` (1/2/3).
- N (`m&0x80`): `a=Step(cell,E); b=Step(a,N)`; advance E. `(len odd)` → 0x14;
  `!(m&0x01)` → 0x14; `(m&0x06)` → 0x14; else `r%3 + 0x11` (17/18/19).
- If no cardinal bit: outer corners — NE(0x01) → `(r&1)+0x23` (35/36); SE(0x04) →
  `(r&1)+0x21` (33/34); SW(0x10) → `(r&1)+0x27` (39/40); NW(0x40) → `(r&1)+0x25`
  (37/38); none → return 1.
(the E-branch's `r%3` goes through a CDQ/XOR/SUB abs that is a no-op for r>=0.)

**Apply** (shared tail, 0x0057B12D):
- `[0x00880990] = IsoTileTypeArray[g_ShorePieces + piece - 1]` — array base
  `[0x00A8ED2C]`, `g_ShorePieces = [0x00ABAD28]` (theater INI [General]
  ShorePieces); piece indices are 1-based.
- anchor = cell coord + `(dx,dy)` from the offset table
  `[0xABDB64 + piece*4]` (two int16s; §13.7.3).
- `FUN_004A91B0(ECX=0x0087F7E8, &out_old, &anchor)`: with `this+0x117C == 0`
  (guaranteed by the prologue's `FUN_004A8BF0(0)`), this just writes the anchor
  into `this+0x1174` = `[0x0088095C]/[0x0088095E]` and returns (verified via
  decompile_function 0x004a91b0 + address arithmetic 0x0087F7E8+0x1174, 2026-07-20).
- `ok = 1;` call `MapClass__ApplyBridgeTile 0x0057B440(lo, hi, cell+0x11B, id,
  &ok, flag)` with `(lo,hi) = (0,0)` for variant 1 and `(g_ShorePieces,
  g_ShorePieces+0x29)` for variant 2 (verified via disassemble_function 0x0057acf0
  0x0057B18E..0x0057B1E3, 2026-07-20).
- return `ok` (0 = FAILURE propagated to the tiler).

### 13.6 Tile stamping and the verdict — `MapClass__ApplyBridgeTile 0x0057B440`

`(int lo, int hi, int level, int region, char* ok, char flag2)`, `RET 0x18`; reads
the tile type from `[0x00880990]` and the anchor from `[0x0088095C]/[0x0088095E]`
(verified via disassemble_function 0x0057b440, 2026-07-20). `*ok` is written ONLY
on the hard-fail paths; the function's own AL return (0 = stopped early, 1 = all
subcells done) is ignored by the caller.

1. `tileType->vtbl+0x2C()` must equal 0x12 else return (no fail — see §13.11);
   `data = tileType->vtbl+0x9C()` must be non-null. Subtile grid: width
   `[tileType+0x2E4]`, height `[tileType+0x2E8]`, entries `[data+0x10 + (j*width+i)
   *4]` (null entry = hole, skip). New tileset index: `[tileType+0x294]`.
2. If `flag2 != 0`: `region := 0`.
3. Per subcell (row-major j outer, i inner): target `(x,y) = anchor + (i,j)`; skip
   if outside the map diamond (`W=[0x0087F8DC]`, `x+y > W && x-y < W && y-x < W &&
   x+y <= W + 2*[0x0087F8E0]`); resolve cell from `[0x0087F924]` (dummy 0xABDC50
   fallback); `owner = FUN_005A00C0(&(x,y))`.
4. Ownership gate (flag2==0; under flag2!=0 every subcell goes straight to step 5
   with region forced 0):
   - `owner == region` → orientation check (step 5).
   - `owner <= 0`, `owner != region`: if `IsClear(cell)` → write region id :=
     `region`, go to step 6; else if `region == -1` → skip subcell; else →
     **`*ok=0`, stop (HARD FAIL)**.
   - `owner > 0`, `owner != region`, `region == -1`: same as previous bullet
     (clear → adopt -1, else skip subcell).
   - `owner > 0`, `owner != region`, `region != -1`: if `IsClear(cell)` → region id
     := `region`, step 6. Else compute `old = cell+0x38 - g_ShorePieces`, `new =
     [tileType+0x294] - g_ShorePieces`; if both in [0,0x29] AND
     `class[old] == class[new]` (table 0x0082A7F4, §13.7.1) → **return 1
     immediately** (whole call succeeds, remaining subcells skipped — an equivalent
     shore piece from another region is already there); else → **`*ok=0`, stop
     (HARD FAIL)**.
5. Orientation check (reached when owner==region, or always under flag2):
   `old/new` as above; if both in [0,0x29] and `3 <= |orient[old] - orient[new]|
   <= 5` (table 0x0082A89C, §13.7.2) → **`*ok=0`, stop (HARD FAIL)** — an
   opposing-facing shore piece already occupies the cell. Else step 6.
6. Write gate: if `IsClear(cell)` OR `cell+0x38` in `[lo,hi]` → **write**:
   `cell+0x38 = [tileType+0x294]`, `cell+0x11A = (byte)(j*width+i)`,
   `cell+0x11B = [subtileEntry+0x28] + level` (subtile height byte + the source
   cell's level); continue with next subcell.
   Else (occupied by something outside [lo,hi]):
   - if `IsShorePieceTile(cell)` (`0x004865B0`: tile in [g_ShorePieces, +0x2A),
     verified via decompile_function 0x004865b0, 2026-07-20) AND
     `IsOnBridgeRamp(ECX=[tileType+0x294], EDX=j*width+i)` (`0x00578D80`: the NEW
     piece's tileset/subtile is in the slope/ramp families — DAT_00AA1020+0x28,
     four 4-tile sets with subtile exceptions, DAT_00ABBEBC+0x14; verified via
     decompile_function 0x00578d80, 2026-07-20) → **`*ok=0`, stop (HARD FAIL)**.
   - else if `[tileType+0x294]` in `[g_ShorePieces, +0x2A)` AND `FUN_004863D0(cell)`
     (cell's existing tile is in the ramp/bridge families: DAT_00AA1020+0x28, the
     four 4-tile sets, DAT_00ABBEBC+0x14, DAT_00ABAD24+4,
     g_BridgeSet_TileSetBase+0x10, g_WoodBridgeSet_TileSetBase+0x10,
     DAT_00ABC2C8+2, DAT_00AA101C+0x1C; verified via decompile_function 0x004863d0,
     2026-07-20) → **`*ok=0`, stop (HARD FAIL)**.
   - else → stop stamping this tile, `*ok` untouched (**soft stop** — Select
     still returns success).

Region-id writes in step 4 go through `FUN_005A0090` with ECX=0x00ABDFD8 (receiver
ignored by the helper — it uses `[0x00ABED10]` directly).

### 13.7 Data tables

**13.7.1 Piece-class table `0x0082A7F4`** (42 ints, indexed by 0-based piece
`tile - g_ShorePieces`; equal class = interchangeable for the cross-region
acceptance in §13.6 step 4): `[0,0,0,1,2,3,4,4,5,5,5,6,7,8,9,9,10,10,10,11,12,13,
14,14,15,15,15,16,17,18,19,19,20,20,21,21,22,22,23,23,24,25]` (verified via
read_memory 0x0082A7F4 len 168, 2026-07-20).

**13.7.2 Orientation table `0x0082A89C`** (42 ints, octant-like 0..7; |diff| in
[3,5] = conflicting facings, §13.6 step 5): `[4,4,4,4,4,4,3,3,2,2,2,2,2,2,1,1,0,0,
0,0,0,0,7,7,6,6,6,6,6,6,5,5,3,3,1,1,7,7,5,5,4,4]` (verified via read_memory
0x0082A89C len 168, 2026-07-20).

**13.7.3 Anchor-offset table `0xABDB64` (entries at `+piece*4`, piece 1-based;
two int16 `(dx,dy)`)** — zero in the static image; filled once at startup by the
static initializer at `0x0057A9C0` (registered in the CRT init pointer table at
0x00813AB4; the code region is not a defined Ghidra function). Decoded from its
disassembly (verified via disassemble_bytes 0x0057A9C0..0x0057ACE8 + get_xrefs_to
0x0057a9c0 + search_byte_patterns "68 db ab 00", 2026-07-20):

| piece | (dx,dy) | piece | (dx,dy) | piece | (dx,dy) |
|---|---|---|---|---|---|
| 1 | (0,-1) | 15 | (-1,0) | 29 | (0,-1) |
| 2 | (0,-1) | 16 | (-1,0) | 30 | (0,0) |
| 3 | (0,-1) | 17 | (0,0) | 31 | (0,-1) |
| 4 | (0,-1) | 18 | (0,0) | 32 | (0,-1) |
| 5 | (0,-2) | 19 | (0,0) | 33 | (-1,-1) |
| 6 | (-1,-2) | 20 | (0,0) | 34 | (-1,-1) |
| 7 | (-1,-1) | 21 | (0,0) | 35 | (-1,0) |
| 8 | (-1,-1) | 22 | (-1,0) | 36 | (-1,0) |
| 9 | (-1,0) | 23 | (0,0) | 37 | (0,0) |
| 10 | (-1,0) | 24 | (0,0) | 38 | (0,0) |
| 11 | (-1,0) | 25 | (0,0) | 39 | (0,-1) |
| 12 | (-1,0) | 26 | (0,0) | 40 | (0,-1) |
| 13 | (-2,-1) | 27 | (0,0) | 41 | (0,0) |
| 14 | (-2,0) | 28 | (0,0) | 42 | (0,0) |

(entries 41/42 exist but no Select path produces pieces > 0x28.)

### 13.8 RNG accounting

- Pass A and Pass B: **zero draws**.
- Pass C and Pass D: **exactly one `FUN_00598030(0,5)` bounded draw per iterated
  cell**, drawn before any other work; each bounded draw = 1 raw
  `Random__Next(0x00ABE890)` draw, +1 per rejection (only on raw 0xFFFFFFFF).
  Cells after an abort are not iterated (no draws). ECX=0x00ABE890 confirmed at the
  single Random__Next site 0x0059805E..0x00598063 inside the helper; the helper is
  the only RNG consumer in the whole tiler family. (verified via
  disassemble_function 0x00598030 / 0x0057acf0 + decompile_function 0x0057a430 /
  0x0057a320 / 0x0057b440 — no other Random callsites, 2026-07-20)
- Draw usage: parity `(r&1)` for paired pieces, `r%3` for 3-variant straight runs;
  a draw is consumed even when the chosen branch ignores `r`.

### 13.9 Failure modes (what rolls a blob back)

With `flag=1` (water-seed `0x0059A6C0` and flood-fill `0x0059BBC0` calls):
- Pass A cannot fail. Failures come only from Passes C/D via `*ok=0`:
  1. an opposing-orientation shore piece (|orient diff| 3..5) already stamped where
     a new piece's subcell lands (§13.6 step 5) — i.e. two shorelines too close,
     facing each other;
  2. a shore-piece cell where the new piece's subcell is a ramp-family piece, or a
     ramp/bridge-family tile underneath a new shore piece (§13.6 step 6).

With `flag=0` (final driver calls):
- Pass A fails when eroding a thin/notched land cell would flood a cell whose
  region id is set and differs from `id`;
- Passes C/D additionally fail when a subcell lands on a foreign-region non-clear
  cell whose existing piece class differs (§13.6 step 4).

In practice on an all-water map with one fresh blob (flag=1), rollback happens when
the blob's shoreline geometry forces overlapping/contradictory shore pieces —
typically blobs that are too thin or self-adjacent across a 1-2 cell water gap.

### 13.10 Label drift recorded

`MapClass__MarkBridgesForRepair_High (0x0057A0C0)`, `MapClass__UpdateBridgeTile_Low
(0x0057A430)`, `MapClass__ClearBridgeCell_Low (0x0057A320)`,
`MapClass__SelectBridgeTileVariant_Low (0x0057ACF0)`, `MapClass__ApplyBridgeTile
(0x0057B440)`, `MapClass__ComputeBridgeSurfaceMask (0x0057B210)`,
`CellClass__IsBridgeDeckTile (0x00485060)`, `IsOnBridgeRamp (0x00578D80)`,
`g_UIModeLock (0x00880990)` — all are shore/ShorePieces machinery, not bridge or UI
logic. Bodies verified as documented above.

### 13.11 Unverified (YELLOW)

- **`vtbl+0x2C == 0x12`** (§13.6 step 1): presumed the abstract-type/WhatAmI
  virtual returning the isometric-tile-type id; the vtable slot was not resolved to
  a function body this session. If some ShorePieces tile type returned a different
  value the tile would silently no-op (treated as success).
- **`[0x0088098C]` object** (tiler epilogue vtable+0x20(1) call): identity and
  whether it is ever non-null on the RMG path not traced; assumed editor-only
  state.
- **Identities of the ramp-family tileset globals** in `0x004863D0`/`0x00578D80`
  (DAT_00AA1020 +0x28, DAT_00AA073C/DAT_00ABB110/DAT_00AA1050/DAT_00AA10A0 +4 each,
  DAT_00ABBEBC +0x14, DAT_00ABAD24 +4, DAT_00ABC2C8 +2, DAT_00AA101C +0x1C): which
  theater INI [General] keys fill each was not traced this session (the two bridge
  sets carry existing labels `g_BridgeSet_TileSetBase`/`g_WoodBridgeSet_TileSetBase`
  — labels, not re-verified). On a fresh RMG water map most are absent from stamped
  cells, so these gates rarely fire there.
- **`[0x00ABED04]/[0x00ABED08]` writers**: treated as the RMG diamond bounds set by
  map-prep (per the map-prep report §10); not re-traced from this function family.
- **Scratch `+0x4A` clearers**: default 1 from the constructor; whether any RMG
  stage clears it (making ComputeMask return 0 for those cells) not traced.
