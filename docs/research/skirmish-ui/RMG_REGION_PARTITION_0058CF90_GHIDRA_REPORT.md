# RMG Region Partition: `FUN_0058cf90`, `FUN_0058e740`, `FUN_0058e9b0`, `FUN_0058d010`

**Status:** COMPLETE  
**Active in YR:** Yes — unconditional code path in `FUN_00598960`, every random-map generation  
**Session date:** 2026-06-01  

---

## Investigation Scope

**Target questions:**
1. What does `FUN_0058cf90` initialize?
2. What is the region struct layout and field semantics?
3. What is the exact decoded argument to `FUN_0058e740` and what does the function do?
4. What do `FUN_0058e9b0` and `FUN_0058d010` do per region?
5. What is the region-count source and region-array layout?
6. Which `Random__Next` draws are consumed in this phase?

**Non-goals:** Water seeding, start placement, tiberium placement, RNG primitive internals, `FUN_0058ebc0` / `FUN_0058ef10` (mode34 making-regions), pathfinding deeper than one callee.

**Evidence to mark COMPLETE:** Verified field offsets from assembly, argument formula decoded from assembly, role of each function established from decompile.

**Stop conditions:** All four target functions decompiled; argument formula assembly traced; region struct fields confirmed by `get_assembly_context`.

---

## Verified Globals

| Global | Address | Role | Evidence |
|--------|---------|------|----------|
| `DAT_00abdfa0` | `0x00abdfa0` | Region count (number of live region objects) | verified via `decompile_function 0x00598960`; `iVar10 = DAT_00abdfa0` used as loop bound |
| `DAT_00abdf94` | `0x00abdf94` | Region pointer array (`RegionClass*[n]`) | verified via `decompile_function 0x00598960`; indexed `DAT_00abdf94 + iVar10 * 4` |
| `DAT_00abed10` | `0x00abed10` | Cell-region data block base (0x50 stride, `mapW * mapW` entries) | verified via `decompile_function 0x0058cf90` and `get_assembly_context 0x0058cfc6` |
| `DAT_0089c2dc` | `0x0089c2dc` | Map width (`mapW`); total cells = `mapW * mapW` | verified via `decompile_function 0x0058cf90`; `iVar2 = DAT_0089c2dc * DAT_0089c2dc` |
| `DAT_00abed04` | `0x00abed04` | Map half-diagonal bound (isometric valid-cell gate) | verified via `decompile_function 0x0058e740` and `FUN_0058e9b0` |
| `DAT_00abed08` | `0x00abed08` | Map opposite-diagonal bound | verified via `decompile_function 0x0058e740` |
| `DAT_00abed14` | `0x00abed14` | Region ID counter (monotone; assigned to new regions) | verified via `decompile_function 0x0058bf70` (`DAT_00abed14 + 1`); confirmed zeroed before `FUN_0058cf90` at `0x00598c89` via `get_assembly_context 0x00598c73` |

---

## Cell-Region Entry Layout (0x50 stride from `DAT_00abed10`)

Each entry is exactly `0x50` bytes. The stride `ADD ESI, 0x50` is verified at `0x0058cffe` via `get_assembly_context`.

| Offset | Width | Semantics | Evidence |
|--------|-------|-----------|----------|
| `+0x00` | `i16` | Cell X coordinate (column) | verified via `get_assembly_context 0x0058cfc6`; `CMP word ptr [ESI + EAX*0x1]` after loading base = cell coord pair |
| `+0x02` | `i16` | Cell Y coordinate (row) | verified via `get_assembly_context 0x0058cfcd`; `CMP word ptr [ESI + EAX*0x1 + 0x2]` |
| `+0x38` | `i32` | Region ID assigned to this cell (`-1` = unassigned) | verified via assembly at `0x0058cfc6`: `CMP dword ptr [ESI + EAX*0x1 + 0x38],-0x1`; also `0x0058d02c`: `CMP dword ptr [EAX + 0x38],-0x1` |
| `+0x3c` | `i32` | BFS visit stamp / scratch (set to current region ID during flood-fill, prevents re-enqueue) | verified via `decompile_function 0x0058c800`: `*(int*)(iVar7 + 0x3c) != local_3c` guard before enqueue |
| `+0x4b` | `u8` | Water/bridge cell flag — set during water seeding (other slot), consumed by `FUN_0058e9b0` | verified via `get_assembly_context 0x0058eb1b`: `MOV CL,byte ptr [EAX + 0x4b]` gating region assignment |

---

## Region Object Layout (allocated by `FUN_0058bf70`)

`FUN_0058bf70` is the region constructor. Verified via `decompile_function 0x0058bf70`.

| Field | Offset in `RegionClass` | Semantics | Evidence |
|-------|------------------------|-----------|----------|
| `id` | `param_1[2]` = `+0x08` | Assigned from `DAT_00abed14` at construction | verified via `decompile_function 0x0058bf70`: `param_1[2] = DAT_00abed14` |
| `terrain_type` | `param_1[4]` = `+0x10` | CellClass terrain type byte at seed cell (`+0x11b` of CellClass) | verified via `decompile_function 0x0058bf70` |
| `is_bridge_or_passable` | `param_1[5]` = `+0x14` (`char`) | Non-zero if the region is "active" (enabled for flood-fill loop) | verified via `get_assembly_context 0x00598cad`: `CMP byte ptr [ESI + 0x14],BL` gating per-region loop |
| `cell_count` | `+0x0c` in region | Number of cells belonging to this region | verified via `decompile_function 0x00598960`: `(8000 < *(int *)(iVar11 + 0xc))` in argument formula; also `decompile_function 0x0058c800` increments `*(int*)(iVar7 + 0xc)` for each cell matching region ID |
| `seed_cell` | `+0x16` (`u32` coord pair) | Cell that seeded the region (stored as packed `(y<<16|x)`) | verified via `decompile_function 0x0058bf70`: `*(undefined4 *)((int)param_1 + 0x16) = param_2` |

---

## `FUN_0058cf90` — Bridge/Impassable Cell Scanner

**Active in YR: Yes** (unconditional call in generator entry, `0x00598c8f`)

**Role:** Iterates all `mapW*mapW` cell entries (0x50 stride). For each cell where `+0x38 == -1` (unassigned) and coordinates are non-zero (not the null cell), checks if the cell has a bridge overlay (`CellClass__HasBridgeOverlay`) or is impassable (`FUN_004867b0`). If so, calls `FUN_0058c800` to create a region for it and flood-fill it.

**Verified via `decompile_function 0x0058cf90`:**
- Loop bound: `DAT_0089c2dc * DAT_0089c2dc` (verified `get_assembly_context 0x0058cf91`)
- Condition: `*(int*)(iVar3 + 0x38 + DAT_00abed10) == -1` plus non-zero coords (verified `get_assembly_context 0x0058cfc6`)
- Calls `FUN_0058c800` (the region flood-fill constructor) on each bridge/impassable seed cell
- Stride: `iVar3 += 0x50` (verified `get_assembly_context 0x0058cffe`)

**Effect:** After `FUN_0058cf90`, all bridge-overlay and impassable cells have been seeded as regions. The `DAT_00abed14` counter has been incremented for each created region.

---

## `FUN_0058c800` — Region Flood-Fill Constructor

**Active in YR: Yes** (called by `FUN_0058cf90` and the per-region loop)

This is a large function implementing BFS flood-fill. It:
1. Constructs a new `RegionClass` object at a `0x50`-byte heap block via `FUN_0058bf70`
2. Sets `+0x38` and `+0x3c` on each cell to the region ID as it propagates
3. Writes the terrain type byte (`CellClass+0x11b`) to `CellClass+0x11b` of each neighbor
4. Counts cells in `region+0x0c`
5. Returns the new region pointer

**Key field writes confirmed via `decompile_function 0x0058c800`:**
- `*(int*)(iVar7 + 0x3c) = local_3c` — marks cell as enqueued (BFS visit stamp)
- `*(int*)(iVar7 + 0x38) = local_3c` — assigns region ID
- `*(int*)(iVar7 + 0xc) += 1` — cell count

`Random__Next @ 0x0065c780` **is called** once within `FUN_0058c800` in the branch where `local_3c == 0` and no neighbors share terrain type — this is the seed cell random selection for the first region. Confirmed via `decompile_function 0x0058c800`: `uVar10 = Random__Next()` inside the `local_3c == 0` branch. This is the **only** `Random__Next` draw in the region partition phase.

---

## `FUN_0058e740` Argument Formula — Decoded from Assembly

**Active in YR: Yes**

Per-region loop at `0x00598ca4`–`0x00598cf0` (verified via `get_assembly_context 0x00598ca4`):

```
// ESI = current region pointer (from DAT_00abdf94[i])
CMP byte ptr [ESI + 0x14], BL    ; skip if region.is_active == 0
MOV EDX, [MapSeed + 0x3c]        ; theater/mode
CMP EDX, 3                       ; mode34 = (theater == 3 || theater == 4)
JZ  set_mode34_true
CMP EDX, 4
JZ  set_mode34_true
XOR EAX, EAX                     ; !mode34 = 1 → added as 1 when mode34=false
JMP next
set_mode34_true: MOV EAX, 1      ; mode34=true → XOR with !mode34 gives 0
next:
MOV EDX, [ESI + 0xc]             ; region.cell_count
XOR ECX, ECX
CMP EDX, 0x1f40                  ; 0x1f40 = 8000
SETG CL                          ; CL = (cell_count > 8000) ? 1 : 0
XOR EDX, EDX
CMP AL, BL                       ; AL = mode34 flag (1 if mode34)
SETZ DL                          ; DL = !mode34
LEA EAX, [ECX + EDX*1 + 4]      ; arg = (cell_count>8000) + !mode34 + 4
MOV ECX, ESI                     ; ECX = region ptr (fastcall this)
PUSH EAX
CALL FUN_0058e740
```

**Decoded formula:** `arg = 4 + (region.cell_count > 8000 ? 1 : 0) + (mode34 ? 0 : 1)`

- mode34 false (theaters 0/1/2), small region (≤8000 cells): `4 + 0 + 1 = 5`
- mode34 false, large region (>8000 cells): `4 + 1 + 1 = 6`
- mode34 true (theater 3 or 4), small region: `4 + 0 + 0 = 4`
- mode34 true, large region: `4 + 1 + 0 = 5`

**Verified via `get_assembly_context 0x00598ca4`:** `CMP EDX,EBP` where `EBP = 0x1f40` (set at `0x00598c9f`), `SETG CL`, `SETZ DL`, `LEA EAX,[ECX + EDX*0x1 + 0x4]`.

---

## `FUN_0058e740` — BFS Multi-Pass Region Expander

**Active in YR: Yes** (called per region)

**Role:** Takes `param_1` (the decoded `arg` 4–6) as an iteration count. Runs `param_1` rounds of BFS expansion on a region's frontier:

- Each round: allocates a new BFS queue (`0x18` byte heap block), seeds it from the previous round's output
- For each frontier cell: checks all 8 neighbors using `g_DirectionOffsets` (global table of 8 `(dx,dy)` pairs)
- Validates neighbor is within isometric bounds (`DAT_00abed04`/`DAT_00abed08`)
- Checks neighbor terrain type (`CellClass+0x11b`) matches current region's type (`local_10 + 0x10`)
- If neighbor `+0x38 == -1` (unassigned) and is clear or bridge: enqueues and assigns `*(region+0x38) = *(region+0x8)` (region ID)
- If neighbor is already assigned to a different region: returns `0` (conflict — stops expansion)
- Returns `1` on success

The iteration count `arg` (4–6) controls how many BFS expansion passes are run per region. More passes = wider flood-fill from the seed frontier. mode34 and large-region conditions get fewer passes (terrain 3/4 may be geographically different).

**Verified via `decompile_function 0x0058e740`.**

---

## `FUN_0058e9b0` — Water-Cell Region Propagation

**Active in YR: Yes** (called after `FUN_0058e740` for each active region)

**Role:** A secondary BFS pass that propagates region assignment specifically into water/passable-marked cells (`+0x4b` flag). 

- Calls `FUN_0058d410` to get frontier cells of the region
- Filters cells from frontier into a work queue
- BFS-expands into neighbors where `*(cell + 0x4b) != 0` AND `*(cell + 0x38) == -1`
- Assigns `*(neighbor + 0x38) = *(region + 0x8)` (same region ID)

The `+0x4b` gate means this pass only assigns region IDs into cells already marked as water/passable-accessible by the earlier water seeding phase. This prevents dry land being claimed via water bridges.

**Verified via `get_assembly_context 0x0058eb1b`:** `MOV CL,byte ptr [EAX + 0x4b]` directly gates the assignment at `0x0058eb1e: TEST CL,CL / JZ 0x0058eb74`.

---

## `FUN_0058d010` — Fallback Unassigned Cell Seeder

**Active in YR: Yes** (called once after the per-region loop, `0x00598cfc`)

**Role:** Iterates all `mapW*mapW` cells. For each cell where `+0x38 == -1` (still unassigned after the per-region loop) and coordinates are non-zero, calls `FUN_0058c800` to create a new region seeded from that cell. Takes a `param_1` byte written to `region + 0x1a` of any created region.

This function catches terrain cells that were not seeded in `FUN_0058cf90` (not bridge/impassable) and not reached by BFS expansion. Each such isolated cell gets its own region.

**Verified via `decompile_function 0x0058d010`:**
- Loop: `DAT_0089c2dc * DAT_0089c2dc` iterations, `+= 0x50` stride
- Guard: `*(int*)(psVar2 + 0x1c) == -1` — NOTE: this is offset `+0x38` relative to `psVar2` which starts at the entry base, but the decompiler offset appears as `+0x1c` because `psVar2` is `(short*)` typed (sizeof short = 2, so `0x1c * 2 = 0x38`). **Assembly confirms** `CMP dword ptr [EAX + 0x38],-0x1` at `0x0058d02c` (verified via `get_assembly_context 0x0058d02c`).
- Calls `FUN_0058c800` (verified `get_assembly_context 0x0058cff4`)
- Sets `*(region + 0x1a) = param_1` from result (verified via `decompile_function 0x0058d010`: `*(undefined1 *)(iVar3 + 0x1a) = param_1`)
- Caller passes `XOR CL,CL` (zero) for `param_1` — `0x00598cfa: XOR CL,CL / CALL 0x0058d010`

---

## `FUN_0058d410` — Region Frontier Builder

**Active in YR: Yes** (called by `FUN_0058e740` and `FUN_0058e9b0`)

Iterates all cells via `MapClass__CellIterator`. For cells belonging to the current region (matching `region + 0x8` ID), checks all 8 neighbors. If any neighbor belongs to a different region, the cell is a border cell. Adds border cells to an output list (dynamic array `0x18` bytes).

This is the "frontier extraction" subroutine — not a BFS itself, just a border-cell collector.

**Verified via `decompile_function 0x0058d410`.**

---

## `FUN_005ac290` — Region Deregistration

Called in the init cleanup loop (before `FUN_0058cf90`) to remove any previously registered regions from `DAT_00abdf94`. Decrements `DAT_00abdfa0`. Frees the region's internal buffer and the region object itself.

**Verified via `decompile_function 0x005ac290`.**

---

## Phase Execution Order (verified from `FUN_00598960`)

1. Zero `DAT_00abed14` (`0x00598c89`)
2. Reset all cells: `+0x38 = -1`, `+0x3c = -1` for all `mapW*mapW` entries (`0x00598c52/0x0058c5c`)
3. Deregister all previous regions via `FUN_005ac290` (`0x00598c81`)
4. `FUN_0058cf90()` — seed regions from bridge/impassable cells
5. For each region `i` in `[0, DAT_00abdfa0)` where `region->+0x14 != 0`:
   a. `FUN_0058e740(arg)` where `arg = 4 + (region.cell_count > 8000) + !mode34`
   b. `FUN_0058e9b0()` — water-cell propagation
6. `FUN_0058d010(0)` — fallback: seed any unassigned cells as new regions

All verified via `decompile_function 0x00598960`.

---

## RNG Draw Analysis

- `Random__Next @ 0x0065c780` is called **once** inside `FUN_0058c800` in the `local_3c == 0` branch (first-region seed selection). This is confirmed by `decompile_function 0x0058c800`.
- The `get_function_callers 0x0065c780` confirms `FUN_0058c800 @ 0058c800` is a caller.
- `FUN_0058e740`, `FUN_0058e9b0`, and `FUN_0058d010` do NOT directly call `Random__Next` — they use deterministic BFS. `FUN_0058c800` is also called by `FUN_0058d010`, which may trigger additional RNG draws if `local_3c == 0` applies.
- The RNG instance used is `g_MapGenRng`, initialized in `FUN_00598960` before this phase.

---

## Implementation Handoff

### Handoff 1 — Cell-Region Array

**Verified behavior:** All `mapW*mapW` cells use a flat array `DAT_00abed10` with stride `0x50`. Each entry holds at `+0x38` the region ID (i32, -1 = unassigned) and at `+0x3c` a BFS visit stamp. Cell coordinates are at `+0x00` (i16 X) and `+0x02` (i16 Y).

**Rust delta:** Add `cell_region_data: Vec<CellRegionEntry>` with layout:
```rust
struct CellRegionEntry {
    x: i16, y: i16,
    // ... 0x34 bytes of other cell data (owned by other subsystems) ...
    region_id: i32,       // offset +0x38
    bfs_visit: i32,       // offset +0x3c
    // ... 0x0c bytes padding to 0x50 total ...
    water_flag: u8,       // offset +0x4b
    // pad to 0x50
}
```

**Affected surface:** `src/sim/` random map generation, region phase.

**Acceptance scenario:** After phase 5, every non-null cell has `region_id >= 0`; no cell has `region_id == -1` unless it is outside the isometric valid range.

**Proposed Rust test name:** `test_rmg_region_partition_all_cells_assigned`

**Risk:** The `+0x4b` water flag is written by the water seeding phase (another slot) before this phase runs. If the water phase is not completed first, `FUN_0058e9b0` will see no water cells and produce different output.

---

### Handoff 2 — `FUN_0058e740` Argument Formula

**Verified behavior:** Per assembly `0x00598ca4`–`0x00598ce3`: `arg = 4 + (region.cell_count > 8000 ? 1 : 0) + (mode34 ? 0 : 1)`. Range is 4–6.

**Rust delta:**
```rust
let large = region.cell_count > 8000;
let mode34 = matches!(map_seed.theater, 3 | 4);
let bfs_passes = 4 + large as u32 + (!mode34) as u32;
expand_region_bfs(region, bfs_passes);
```

**Affected surface:** `src/sim/rmg/region.rs` (to be created), called from generator entry.

**Acceptance scenario:** For a theater-0 map with a large region (>8000 cells), `bfs_passes == 6`; for theater-3, `bfs_passes == 5`; for theater-3 small region, `bfs_passes == 4`. Unit test can verify without running full generation.

**Proposed Rust test name:** `test_rmg_e740_argument_formula`

**Risk:** None — formula is exact assembly-level arithmetic.

---

### Handoff 3 — `FUN_0058d010` Fallback Pass

**Verified behavior:** After all region BFS passes, a full cell scan creates new regions for any cell still at `region_id == -1`. `param_1 = 0` (caller passes `XOR CL,CL`), written to `region + 0x1a`.

**Rust delta:** Rust must implement a post-pass after the per-region BFS loop that sweeps `cell_region_data` and calls the region constructor for any remaining unassigned in-bounds cells.

**Affected surface:** Generator entry sequencing.

**Acceptance scenario:** After phase 6, no in-bounds non-null cell has `region_id == -1`.

**Proposed Rust test name:** `test_rmg_d010_fallback_no_unassigned_cells`

**Risk:** If `FUN_0058c800` is not yet implemented (flood-fill constructor), this fallback is a no-op. Must be implemented together.

---

## Negative Facts / Do Not Do

1. **Do NOT implement `FUN_0058e740` as a simple count loop.** It allocates a BFS queue object each pass and frees the previous one — the queue is a dynamic heap object, not a stack array. Verified via `operator_new(0x18)` inside the loop in `decompile_function 0x0058e740`.

2. **Do NOT apply `FUN_0058e9b0` to all cells.** It only processes cells with `+0x4b != 0` (water flag). Cells with `+0x4b == 0` are skipped at `0x0058eb1e: TEST CL,CL / JZ 0x0058eb74`. Verified via `get_assembly_context 0x0058eb1b`.

3. **Do NOT use the decompiler's `+0x1c` in `FUN_0058d010`.** The decompiler typed `psVar2` as `short*`, making all offsets appear halved. The actual cell-entry offset is `+0x38` (confirmed by assembly `CMP dword ptr [EAX + 0x38],-0x1` at `0x0058d02c`).

4. **Do NOT confuse `DAT_00abed14` with a region index.** It is the ID counter: initialized to 0 before `FUN_0058cf90`, incremented by each `FUN_0058bf70` call. The region array index in `DAT_00abdf94` is separate. Verified via `decompile_function 0x0058bf70`.

5. **Do NOT run the "Making regions" phase (`FUN_0058ebc0` etc.) for non-mode34 maps.** The `if (theater == 3 || theater == 4)` gate is confirmed in assembly at `0x00598cb9–0x00598cc1` and in `decompile_function 0x00598960`.

---

## Remaining Uncertainty

- **`g_DirectionOffsets` exact values:** The table at `&g_DirectionOffsets` is referenced in `FUN_0058e740` and `FUN_0058e9b0` as `8 * 4`-byte entries (each `(dx: i16, dy: i16, pad)`). Values are unverified in this session; the BFS iterates all 8 directions.
- **`FUN_0042fcb0` role:** Called at the start of `FUN_0058e740`, `FUN_0058e9b0`, and `FUN_0058c800`. Likely a heap-pool or allocator init. Not decoded in this session.
- **`FUN_0058bf70` object size:** `operator_new(0x50)` called within `FUN_0058c800` for the region object. Exact full layout beyond the verified fields is not traced here.
- **RNG draw count:** `FUN_0058c800` calls `Random__Next` once per invocation in the `local_3c == 0` first-region branch. Total draw count depends on how many regions trigger this branch; not counted analytically in this session.
- **`PTR_FUN_007e3890` / `PTR_FUN_007e3898`:** These are vtable pointers used for the dynamic BFS queue's grow/check methods. Exact semantics unverified; they appear to be a resizable array class.

---

## Inline Citation Index

| Claim | Citation |
|-------|----------|
| Phase execution order | `verified via decompile_function 0x00598960` |
| `DAT_00abdfa0` = region count | `verified via decompile_function 0x00598960` |
| `DAT_00abed10` stride 0x50 | `verified via get_assembly_context 0x0058cffe` |
| Cell `+0x38` = region ID | `verified via get_assembly_context 0x0058cfc6, 0x0058d02c` |
| Cell `+0x3c` = BFS visit stamp | `verified via decompile_function 0x0058c800` |
| Cell `+0x4b` = water flag | `verified via get_assembly_context 0x0058eb1b` |
| `mapW * mapW` loop bound | `verified via decompile_function 0x0058cf90` |
| Region `+0x08` = ID | `verified via decompile_function 0x0058bf70` |
| Region `+0x0c` = cell count | `verified via decompile_function 0x0058c800, 0x00598960` |
| Region `+0x14` = active flag | `verified via get_assembly_context 0x00598cad` |
| Argument formula (`SETG/SETZ/LEA`) | `verified via get_assembly_context 0x00598ca4, 0x00598cbc` |
| `0x1f40` = 8000 threshold | `verified via get_assembly_context 0x00598c9f` |
| mode34 = theater `{3,4}` | `verified via get_assembly_context 0x00598cb9` |
| `FUN_0058d010` `param_1 = 0` | `verified via get_assembly_context 0x00598cfa` |
| `FUN_0058d010` creates fallback regions | `verified via decompile_function 0x0058d010` |
| `FUN_0058d410` = frontier builder | `verified via decompile_function 0x0058d410` |
| `DAT_00abed14` zeroed before `FUN_0058cf90` | `verified via get_assembly_context 0x00598c89` |
| `Random__Next` in `FUN_0058c800` | `verified via decompile_function 0x0058c800, get_function_callers 0x0065c780` |

---

## Queue Discipline & Helper Identities (decoded 2026-07-20)

### 1. Class-test identity in `0x0058C800` / `0x0058CF90` — BOTH prior readings were partly wrong

Resolved from raw CALL targets, not labels (verified via search_instructions
CALL-scan of FUN_0058c800 + disassemble_function 0x0058CF90, 2026-07-20):

- `0x0058C800` head pair: `CALL 0x004865D0` @0x0058c877, then `CALL 0x004867B0` @0x0058c882.
- `0x0058C800` neighbor-loop pair: `CALL 0x004865D0` @0x0058c9f9, then `CALL 0x004867B0` @0x0058ca13.
- `0x0058C800` blob-dissolve neighbor scan: `CALL 0x004865D0` only @0x0058cbbf (no green test).
- `0x0058CF90` seed gate: `CALL 0x004865D0` @0x0058cfde, then `CALL 0x004867B0` @0x0058cfe9.

**What the two functions actually test** (both read the cell's iso tile index at
CellClass+0x38 and compare against theater tileset bases):

- **`0x004865D0`** (Ghidra label `CellClass__HasBridgeOverlay` — LABEL DRIFT, it
  tests no bridge set): returns 1 iff tile index in `[ShorePieces, +0x2A)` or
  `[WaterSet, +0x0E)` or `[WaterfallEast, +4)` or `[WaterfallWest, +4)` or
  `[WaterfallNorth, +4)` or `[WaterfallSouth, +4)`. I.e. a **shore/water/waterfall
  membership test** (verified via decompile_function 0x004865D0, 2026-07-20). The
  four `DAT_` globals (0x00aa073c/0x00abb110/0x00aa10a0/0x00aa1050) were identified
  as the four Waterfall* tileset bases from the key-to-global assignment loop in
  `Read_Theater_TileSets_INI` 0x00545150 (verified via decompile_function
  0x00545150 + get_assembly_context 0x00545e8f/e9e/ead/ebc, 2026-07-20).
- **`0x004867B0`**: returns 1 iff tile index == `g_GreenTile` base or in
  `[ClearToGreenLat, +0x10)` — the green-LAT membership test (verified via
  decompile_function 0x004867B0, 2026-07-20).
- **`0x00486380`** (`CellClass__IsClearTile`): tile index == 0xFFFF or == 0
  (verified via decompile_function 0x00486380, 2026-07-20). **NOT called** by
  0x0058C800/0x0058CF90 — the 2026-07-19 slot-6 audit claim "IsClearTile OR
  green-LAT" is WRONG for these two functions. It IS the pair used by the
  multi-pass expander 0x0058E740 (see item 2).

**Verdict:** region class bit = water-ish (shore/water/waterfall) OR green
(GreenTile/ClearToGreenLat). Neither "bridge overlay" (label) nor "clear tile"
(audit) is correct for the flood-fill/seed sites.

### 2. `0x0058E740` — multi-pass region expander

`__thiscall`, ECX = region object (saved at [ESP+0x18]; region+0x8 = id,
region+0x10 = level), stack arg = pass count (verified via disassemble_function
0x0058E740, 2026-07-20).

- **Seeding:** calls `FUN_0058D410(region)` ONCE before the pass loop.
  0x0058D410 walks ALL map cells via `MapClass__CellIterator` and collects every
  cell whose scratch region == this region's id that has >=1 in-diamond neighbor
  with a different region id — i.e. the full current region boundary (verified
  via decompile_function 0x0058D410, 2026-07-20). **Pass 1 seeds from the full
  region frontier; each later pass seeds only from the previous pass's
  newly-claimed cells** (after each pass: old vector destroyed via vtable
  destructor(1), then the new vector becomes the frontier).
- **Queue discipline:** NOT a stack. Per-pass append-only arrays scanned by
  ascending index (ring-by-ring BFS). Within a cell, neighbors probed in
  `g_DirectionOffsets` (0x0089F688) index order 0 through 7 (verified via
  disassemble_function 0x0058E740: LEA EDX,[ECX*4+0x89f688], INC EBX, CMP EBX,8).
- **Enqueue conditions** per neighbor: (a) in-diamond (`x+y > DAT_00abed04`,
  `|x-y| < DAT_00abed04`, `x+y <= DAT_00abed08`); (b) real cell level (+0x11B) ==
  region level; (c) scratch region (+0x38) == -1; (d) class test = `CALL
  0x00486380` (IsClearTile) OR `CALL 0x004867B0` (green) — @0x0058e88f/0x0058e8a9.
  On success: append coord to the new vector (capacity-gated; growth may fail),
  then **unconditionally** claim: scratch+0x38 = region id, real cell +0x11B =
  region level. The claim happens even if the vector append was refused.
- **Abort semantics (return 0):** hitting an in-diamond, level-matching neighbor
  whose scratch region is a DIFFERENT region id aborts the whole call — and so
  does an UNCLAIMED (-1) level-matching neighbor that fails the clear/green class
  test (the class-fail path falls into the same `iVar4 != region_id` check with
  iVar4 = -1; verified via disassemble_function 0x0058E740: JZ 0x0058e916 ->
  CMP EDI,[EDX+0x8] -> JNZ 0x0058e986 -> destructors -> AL=0). Returns 1 only if
  all passes complete.
- **The 0x18-byte queue object** (also used by 0x0058C800, 0x0058D410,
  0x005A0700): vtable `PTR_FUN_007e3890`; +0x4 element array, +0x8 capacity,
  +0xC/+0xD allocation flags, +0x10 active count, +0x14 growth step; ctor
  `FUN_0042fcb0(0,0)` then growth=10 default. This is the Westwood
  `DynamicVectorClass<CellStruct>` layout (4-byte coord elements; vtable slot +8
  = grow/ensure-capacity, called before append). In 0x0058E740 the growth step
  is overridden to `3 x previous-frontier count` (LEA EAX,[EAX+EAX*2]
  @0x0058e7a0); in 0x0058C800 growth = 50000; in 0x005A0700 growth =
  `this+0x180 * this+0x184` (all verified via the respective
  decompiles/disassembly, 2026-07-20).

### 3. `FUN_005AC370` — the "validity gate" is just CellStruct inequality

`FUN_005AC370(this = short* a, arg = short* b)` returns 1 iff `a[0] != b[0] ||
a[1] != b[1]` — a two-short coordinate `operator!=` (verified via
decompile_function 0x005AC370, 2026-07-20). At the region-cell-count site in
0x0058C800's tail (@0x0058ce61) ECX = scratch-record pointer (record+0 holds the
cell's MapCoord shorts) and the stack arg is a local (0,0) pair (verified via
disassemble_function 0x0058C800 @0x0058ce52-0x0058ce61, 2026-07-20). So region
+0xC counts only scratch records whose region id matches AND whose stored coord
is not (0,0) — i.e. records actually populated with a real cell. Same usage
from FUN_0058EF10 @0x0058effd and FUN_007610F0 (verified via get_xrefs_to
0x005AC370, 2026-07-20).

### 4. `FUN_005A0700(0)` — border-cell collector of region id N, and the draw site

`__thiscall`; at the 0x0058C800 callsite ECX = the global object at
**0x00ABDFD8** (the RMG generator singleton; its +0x180/+0x184 fields form the
vector growth step) and the pushed arg = 0 = region id (verified via
disassemble_function 0x0058C800 @0x0058cad1-0x0058cad7, 2026-07-20). The
function scans the whole 0x50-stride scratch grid and collects, into a
`DynamicVectorClass<CellStruct>`, the stored coord of every record with coord !=
(0,0) and scratch region == arg that has >=1 in-diamond neighbor whose region !=
arg — i.e. **the border cells of region id 0**, not a region list (verified via
decompile_function 0x005A0700, 2026-07-20). Returned object: +0x4 = coord
array, +0x10 = count (the decompiler's dword[1]/dword[4]).

**Draw-site disasm** (0x0058cae6-0x0058cb20, verified via disassemble_function
0x0058C800, 2026-07-20): ESI = count-1; count stored as u64 ->
`FILD qword [ESP+0x34]` -> `FSTP double [ESP+0x2c]` (count as double). Loop:
`MOV ECX,0xabe890` (g_MapGenRng instance) -> `CALL 0x0065C780` (Random__Next raw
u32) -> zero-extended to u64 -> `FILD qword [ESP+0x34]` -> `FMUL double [ESP+0x2c]`
(x count) -> `FMUL double [0x007ED898]` (= 0x3DF0000000100000 =
2^-32 * (1+2^-32), bytes re-read via read_memory 0x007ED898, 2026-07-20) ->
`CALL 0x007C5F00` (Math__ftol) -> `CMP EAX,ESI; JA` redraw while index >
count-1 (unsigned rejection). The accepted index picks
`coord = array[idx]` — **a uniformly drawn border cell of region 0**, which the
first-region branch then uses as the anchor for the dissolve walk.

### Unverified (this section)

- The semantic identity of the object at 0x00ABDFD8 beyond "RMG generator
  singleton whose +0x180/+0x184 are map-dimension-like fields" (name inferred
  from prior RMG docs; not re-derived from the binary this session).
- `FUN_0042fcb0` internals (vector ctor by shape only; body not decoded).
- Whether `0x004865D0`'s label (`CellClass__HasBridgeOverlay`, PROOFED in
  chronominer-locomotion docs 2026-05-24) is wrong in that doc's context too —
  label drift recorded here; the chronominer doc should be re-checked separately.
