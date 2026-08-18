# FUN_00569760 (pavement walker) + FUN_00586990 (cell-list dispatch) — Ghidra body decode

**Author / date:** /re-swarm slot-3 batch, 2026-05-18
**Scope:** Full body decode of exactly two functions:

- `FUN_00569760` @ `0x00569760` — low-bridge **pavement walker** (per
  `BRIDGE_REPAIR_AND_HUT_DEATH §17`, `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT`).
- `FUN_00586990` @ `0x00586990` — **cell-list zone-marker pass** (per
  `BRIDGE_REPAIR_AND_HUT_DEATH §17`); called as the post-walk dispatch.

This report is intentionally narrow. It does **not** re-decode the bridge
dispatchers (`DestroyBridge_*_MapInit`, `RepairBridge_*`, the four
`UpdateRamp_*` walkers) — those are owned by sibling re-swarm slots and by
the existing `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` Phase-2/3 work.
It extends the 1-line summaries given in §17 of that doc with full body decode.

**Confidence per finding:**
- Body decompile: HIGH (Ghidra MCP `decompile_function`, both functions ≤200 lines).
- Caller graph: HIGH for callers Ghidra knows (`get_xrefs_to`); MEDIUM for one
  unmapped-function region (see §1.5 caveat).
- Argument types: HIGH (cross-checked via disassembly prologue and call sites).
- "Active in YR" verdict: HIGH (callers are all live YR bridge code paths).

---

## 0. TL;DR

- `FUN_00569760(CellClass* start_cell, uint dir_code, int* out_screen_rect)` walks
  **up to 30 cells** stepping along `g_DirectionOffsets[dir_code]`, looking for
  ramp-transition tile-IDs from the theater INI. When it finds one of the recognised
  ramp endpoint or 5-variant ramp tiles, it (a) toggles bridge pavement off via
  `MapClass::ToggleBridgePavement`, **and/or** (b) flood-fills the tile index to the
  "healthy" ramp variant via `MapClass::SetOverlayAndPropagate`, **and/or** (c)
  bumps `cell+0x11B += 4` on the 3-cell perpendicular ramp body. After the
  ramp-transition step it continues walking and, for every cell on the walk that is
  NOT the "healthy" ramp variant, spawns a single new `OverlayClass` from
  `g_OverlayTypeClass_Array[0xED]` (dir 2) or `[0xEE]` (dir 4) — these are the
  bridge **destroyed-overlay markers**. It also computes a screen-rect bounding box
  in `out_screen_rect` so the caller can do `TacticalClass::DirtyScreenRect`.
- `FUN_00586990(MapClass* this, DynVec<CellCoord>* list)` is a two-pass walk over
  the coord list accumulated during the walker. **Pass 1** zeroes the level-0 zone
  attribute slot in `MapClass.zone_speed_cache[+0x70]` for each in-bounds coord, and
  invokes `CellClass::RecalcAttributes` on the cell to re-derive LAT / drawability
  bits. **Pass 2** re-scans the same list and, for any coord whose zone slot is
  still zero (i.e. wasn't refilled), calls `FUN_00584550(coord)` — the
  **incremental hierarchical zone-graph rebuild** around one cell. Net effect: the
  walker accumulates touched coords, then this dispatch performs the per-cell
  attribute refresh + zone-graph patch in deferred batch form.
- **Both functions only run on the destruction-side path** (`ProcessBridgeDestruction_Low/High`).
  `ToggleBridgePavement` does NOT call FUN_00569760 (the doc-side hint that it
  might was inverted — see §1.5). The repair walkers (`RepairBridgeWalker_*_*`) do
  not call FUN_00569760 either; they touch overlay/state bytes inline and finish
  with `FUN_005868a0` (the rectangle iterator that itself calls FUN_00586990).
- **Active in YR: Yes** for both. Every caller is a live YR bridge code path
  (engineer/IFV repair → `RepairBridge_*` → `ProcessBridgeDestruction_*` walk; hut
  C4 → `DestroyBridge_*_MapInit` → … → eventually one of the dispatchers).

---

## 1. FUN_00569760 — body decode

### 1.1 Signature & param types

Ghidra-inferred prototype (cross-checked at three call sites in
`ProcessBridgeDestruction_Low` @ `0x00570771`, `0x005707f8`, `0x00570a4e`):

```c
void __cdecl FUN_00569760(
    CellClass* param_1,    // seed cell (pointer; reads +0x24 coord, +0x38 tile_id,
                           //   +0x11A overlay-state byte, +0x11B height-adjust byte)
    uint       param_2,    // direction code from {2, 4}; 2 = N-S ramp axis,
                           //   4 = E-W ramp axis; & 7 indexes g_DirectionOffsets[8]
    int*       param_3     // out: 4-int screen rect [left, top, width, height];
                           //   NULL allowed (skips screen-rect math)
);
```

**param_1 is a pointer (not an int-cast).** Verified by the byte-offset reads in
the decompile: `*(int *)(param_1 + 0x24)`, `*(undefined4 *)(iVar9 + 0x24)`, and
`*(char *)(iVar9 + 0x11b)`. All are direct byte offsets, never `param_1[N]`
indexing, so the CLAUDE.md `int * → ×4` trap does not apply.

**param_2 only branches on values 2 and 4** in the body. Other values fall through
the search loop without ever calling `ToggleBridgePavement` / `SetOverlayAndPropagate`,
then proceed to the post-search overlay-spawn phase. All known call sites pass
2 or 4 — see §1.5.

### 1.2 Walk shape

Two distinct phases, sequential, both walking along a fixed direction:

**Phase A — "Find a ramp transition" loop (max 30 steps):**

```text
i = 1
while i < 30:                             # local_5c < 0x1e
    cell = step(cell, g_DirectionOffsets[param_2 & 7])      # +0x24 coord += offset
    tile_id_rel = cell[+0x38] - DAT_00abad1c + 1            # low-bridge band index
    if param_2 == 2 and tile_id_rel in known_NS_endpoints:
        handle_NS_branch(cell)                              # see §1.3
        break
    if param_2 == 4 and tile_id_rel in known_EW_endpoints:
        handle_EW_branch(cell)                              # see §1.3
        break
    i += 1
if i == 30:                                                 # max-iter exit
    DynamicVectorClass::Clear/Free(&local_18)               # free, no Phase B
    return
```

Each step is a single direction-offset add — **linear span**, not flood-fill, not
radial. The 30-cell cap is hardcoded as `0x1e`. The step direction is constant per
call.

**Phase B — "Spawn destroyed-overlay markers along the walked span":**

After the find loop, the function walks the same span again from the seed cell
(loop counter `local_5c` set in Phase A, then decremented in Phase B), and for
each cell that is NOT (a) a healthy bridge cell with `+0x140 & 1 == 0` matching
`DAT_00abad30`, AND not (b) a damaged cell with `(byte)+0x11A > 4` matching
`DAT_00aa1028`, the function does:

```c
if (param_2 == 2):
    new = operator_new(0xb0);
    if (new): OverlayClass::Constructor(g_OverlayTypeClass_Array[+0x3B4], &cell_coord, -1);
else:        // param_2 == 4 (also catches other dir_codes that fell through)
    new = operator_new(0xb0);
    if (new): OverlayClass::Constructor(g_OverlayTypeClass_Array[+0x3B8], &cell_coord, -1);
```

`+0x3B4` and `+0x3B8` are slot indices into the **global OverlayTypeClass
registry table at `g_OverlayTypeClass_Array`** (the "bridge destroyed marker"
overlays — the visible debris-graphics that appear after a bridge collapse).
These are theater-set at INI load.

Phase B always also recomputes a screen-rect bounding box for the entire span
into `param_3` (left,top,width,height) using `TacticalClass::CoordsToClient2`,
**only if param_3 != NULL**.

### 1.3 Per-cell write map

When the find-loop hits a recognised ramp endpoint/variant cell, the body
mutates state. The set of writes depends on which DAT theater-INI bucket the
tile_id falls in, and on `cell[+0x11A]` (the overlay-state byte):

| Branch (param_2, condition)                                              | Writes performed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
|--------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `param_2 == 2` AND `tile_rel == DAT_00abc1e8` AND `+0x11A == 4`          | `ToggleBridgePavement(cell, 0, 0)` (single-cell pavement-off); then `screen_rect = client(cell + DAT_0089f6a0)`; `ValidateBridgeZones(cell)` → OR into local_61 (zones-dirty accumulator).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `param_2 == 2` AND `tile_rel == DAT_00aa0e38` AND `+0x11A == 4`          | Same as above (alternative N-S endpoint tile).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `param_2 == 2` AND `tile_rel ∈ {DAT_00abad30 .. DAT_00abad30+4}` AND `+0x11A == 4` | `SetOverlayAndPropagate(cell, DAT_00abad30-1 + DAT_00abad1c, -1, -1, 0)` — flood-fills tile to "healthy" variant; `ValidateBridgeZones(cell)` → OR into local_61. **Additionally**, if `tile_rel == DAT_00abad30 + 4` (the 5th variant), bump `cell[+0x11B] += 4` on three sibling cells along the ramp axis: (current), (current + g_DirectionOffsets[0]), (current + DAT_0089f698). Then accumulate a 2×5 cell-rect (offsets x=0..1, y=-2..2) into `local_18` (the DynamicVector — see §1.4 for the dispatch). |
| `param_2 == 4` AND `tile_rel == DAT_00abc1d0` AND `+0x11A == 2`          | `ToggleBridgePavement(cell, 0, 0)`; screen_rect = client(cell + g_DirectionOffsets[0]); ValidateBridgeZones.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `param_2 == 4` AND `tile_rel == DAT_00aa1540` AND `+0x11A == 2`          | Same as above (alternative E-W endpoint tile).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `param_2 == 4` AND `tile_rel ∈ {DAT_00aa1028 .. DAT_00aa1028+4}` AND `+0x11A == 2` | `SetOverlayAndPropagate(cell, DAT_00abad1c-1 + DAT_00aa1028, -1, -1, 0)`; ValidateBridgeZones; if variant +4: bump three perpendicular-axis cells' `+0x11B += 4`, accumulate same 2×5 cell-rect.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |

DAT bucket identities (all theater-INI tile-set bases per
`Read_Theater_TileSets_INI` @ `0x00545B88` xrefs):

- `DAT_00abc1e8` — N-S single-cell ramp endpoint (low bridge)
- `DAT_00aa0e38` — N-S single-cell ramp endpoint (alternative)
- `DAT_00abad30` — N-S 5-variant ramp band base
- `DAT_00abc1d0` — E-W single-cell ramp endpoint (low bridge)
- `DAT_00aa1540` — E-W single-cell ramp endpoint (alternative)
- `DAT_00aa1028` — E-W 5-variant ramp band base
- `DAT_00abad1c` — global low-bridge tile-band base used in `tile_rel = tile_id - DAT_00abad1c + 1`

In addition to the per-branch writes above, **Phase B** writes happen on every
non-recognised cell along the span (see §1.2): `operator_new(0xB0)` →
`OverlayClass::Constructor(g_OverlayTypeClass_Array[+0x3B4 or +0x3B8], coord, -1)`.
This spawns a single new OverlayClass per cell, of type "bridge destroyed marker".

### 1.4 Side effects (other than the per-cell writes)

1. **Stack DynamicVectorClass.** Allocates a `DynamicVectorClass<int>` on the
   stack (`local_18 ..`), initialised via `FUN_0042fcb0(0, 0)` (capacity-0, owned).
   Accumulates packed-coord `int`s during the find-loop **only for the
   5-variant-variant-4 branch** (the `+4` variant on either axis spawns a 2×5
   cell-rect of accumulated coords).
2. **Dispatch tail.** After the walks, if `local_8 != 0` (i.e. cells were
   accumulated), the function calls `FUN_00586990(&local_18)` → see §2 for what
   that does to those cells.
3. **DynamicVector free.** Always runs the destructor sequence at function exit
   (`PTR_FUN_007e38d0`); if the vector grew its own heap buffer (`local_b != 0`)
   it calls `FUN_007c8b3d` (the `operator_delete` trampoline) on `local_14`.
4. **Zones rebuild gate.** `local_61` accumulates the bool returned by
   `MapClass::ValidateBridgeZones` from each branch that mutated tiles. If any
   branch flipped it to non-zero, `MapClass::UpdateBridgeZonesHelper()` is invoked
   at function tail. This is the **full pathfinding zone rebuild** (13 passability
   classes × BFS coloring; `BRIDGE_REPAIR_AND_HUT_DEATH §11`).
5. **Screen rect.** If `param_3 != NULL`, computes a tactical-screen bounding box
   for the walked span (using `TacticalClass::CoordsToClient2` on both endpoints
   and `cell[+0x11B] * DAT_00abde88` for height offset). Returns it as
   `[left, top, width, height]` with each side padded by `-0x40 / +0x80`. Caller
   uses this directly to call `TacticalClass::DirtyScreenRect`.
6. **Off-map fallback.** The classic `(iVar < 0 || iVar > 0x3FFFF) ||
   g_CellArray_Base[iVar] == NULL` pattern points to `DAT_00abdc50` (the off-map
   sentinel cell) and writes the requested coord into `DAT_00abdc74` — same
   fallback pattern as elsewhere in the codebase, unchanged here.

### 1.5 Caller graph

`get_xrefs_to(0x00569760)`:

```text
0x00570771   ProcessBridgeDestruction_Low — call at LAB_005707f5 with dir=2
                (NS-axis ramp branch, after ToggleBridgePavement on single-cell endpoint)
0x005707f8   ProcessBridgeDestruction_Low — call after recursive ProcessBridgeDestruction_Low(coord-2y),
                dir=2 (NS-axis 5-variant ramp branch)
0x00570a4e   ProcessBridgeDestruction_Low — call after recursive ProcessBridgeDestruction_Low(coord-2x),
                dir=4 (EW-axis 5-variant ramp branch)
0x0056ab66   <unmapped function, see caveat>  — same shape, almost certainly the
                ProcessBridgeDestruction_High twin's analog of the dir=2 case
0x0056acaf   <unmapped function, see caveat>  — dir=2 5-variant branch
0x0056acd5   <unmapped function, see caveat>  — pair of the previous
0x0056ae22   <unmapped function, see caveat>  — dir=4 5-variant branch
```

**Caveat — four xrefs land in an unmapped-function region.** Addresses
`0x0056ab66`, `0x0056acaf`, `0x0056acd5`, `0x0056ae22` are inside the body of
what Ghidra has NOT registered as a function (`get_function_by_address` returns
"No function found"). The byte at `0x0056a080` is `83 ec 34 56 8b f1` — a
canonical `__thiscall` prologue (`sub esp, 0x34; push esi; mov esi, ecx`),
indicating an unregistered function entry that extends past `0x0056ae33`. Given:
the four call sites mirror the four `ProcessBridgeDestruction_Low` call sites
1:1, the function lives in the same `.text` neighborhood as the other bridge
destruction code, and is the only candidate caller for the missing High twin,
this is **almost certainly the `ProcessBridgeDestruction_High` analog or a very
close cousin**. The hard constraint forbids `create_function` so this cannot be
pinned with read-only tools. **MEDIUM confidence on identity, HIGH on call
shape.** Suggested follow-up for any session with write access: `create_function`
at `0x0056a080` and re-run `get_function_callers`.

**Negative finding — `MapClass::ToggleBridgePavement` does NOT call
FUN_00569760.** Decompiled `ToggleBridgePavement` @ `0x0056E990` in full; its
only calls are to itself (8-neighbor recurse), `RadarClass::MarkTerrainDirty`,
`TacticalClass::CoordsToClient2`, `TacticalClass::DirtyScreenRect`, and
`FUN_005471f0` (overlay-byte filter). The arrow points the **other way**:
FUN_00569760 calls `ToggleBridgePavement` on the recognised endpoint cells (see
§1.3). The brief's wording — "ToggleBridgePavement callers" — is a misreading of
the LAT_RETRIGGER doc that pre-dates this audit. Likewise neither
`RepairBridge_Low` (`0x57F200`) nor `RepairBridge_High` (`0x57F440`) calls
FUN_00569760 — they call their own `RepairBridgeWalker_*` family.

---

## 2. FUN_00586990 — body decode

### 2.1 Signature & param types

Verified by disassembly (`__thiscall` prologue: `mov ESI, ECX` immediately, EBX
loaded from `[esp+8]`):

```c
void __thiscall FUN_00586990(
    MapClass*           this,    // implicit ECX; reads +0x6C zone_cell_count,
                                 //   +0x70 zone_speed_cache, +0xF4 map_width, +0xF8 map_height
    DynVec<CellCoord>*  list     // explicit stack arg; reads [+0x4]=buffer ptr, [+0x10]=count
                                 //   buffer is an array of packed-i16[2] CellCoords (4 bytes each)
);
```

The decompile signature `(int param_1, int param_2)` is Ghidra naming convention
only — both refer to memory holding pointers. `this[+0xF4]` / `this[+0xF8]` etc.
match the `MapClass` layout in `MAPCLASS_COMPLETE_DECODE.md` (`+0x6C` =
`zone_cell_count`, `+0x70` = `zone_speed_cache` ptr to 10-byte/cell array, `+0xF4` =
`map_size_width`, `+0xF8` = `map_size_height`).

### 2.2 Walk shape

Two sequential passes over the same coord list. Each pass iterates
**backwards** (`i = list.count - 1; i >= 0; --i`):

```text
Pass 1 — clear zone slot + recalc cell attributes
for i in range(list.count-1, -1, -1):
    coord = list.buffer[i]                             # i16[2]
    if MapClass::Is_Cell_In_Playfield(&coord, 1):
        linear = (this.map_width + this.map_height + 1) * coord.y + coord.x
        linear = clamp(linear, 0, this.zone_cell_count - 1)
        this.zone_speed_cache[linear].slot0 = 0        # *(ushort*)(... + linear * 10) = 0
        cell = MapClass::Get_CellClass(coord)          # off-map fallback to DAT_00abdc50
        CellClass::RecalcAttributes(cell)              # re-derive LAT / overlay bits

Pass 2 — incremental zone-graph rebuild for cells still un-coloured
for i in range(list.count-1, -1, -1):
    coord = list.buffer[i]
    if MapClass::Is_Cell_In_Playfield(&coord, 1):
        linear = clamp((this.map_width + this.map_height + 1) * coord.y + coord.x,
                       0, this.zone_cell_count - 1)
        if this.zone_speed_cache[linear].slot0 == 0:   # nobody refilled it
            FUN_00584550(&coord)                        # zone-graph patch around one cell
```

Termination: both loops are bounded by `list.count`. No flood-fill, no neighbor
walk inside this function — neighborhood expansion happens **inside**
`CellClass::RecalcAttributes` (LAT propagation) and `FUN_00584550` (3-level zone
block rebuild).

### 2.3 Per-cell write map

Per coord visited:

- **Pass 1 write:** `(ushort*)&this.zone_speed_cache[linear * 10] = 0` (clears
  the level-0 zone slot). Pass 1 does not write the cell directly; it delegates
  cell-flag refresh to `CellClass::RecalcAttributes` (which is a much larger LAT
  + overlay + draw-bit recomputation — outside this scope, but it touches the
  `cell+0x140` flag word among many other things).
- **Pass 2 write:** none directly. `FUN_00584550(&coord)` performs an incremental
  rebuild of the hierarchical zone graph for levels 2, 1, 0 around the coord — it
  rewrites bytes at `MapClass +0x70`, `+0x80..+0x88` (zone_graph[0..2]),
  `+0x8C..+0xD3` (zone_conn_vecs), `+0x74..+0x7F`, and emits final bidirectional
  edges. (Documented at the plate comment on FUN_00584550 itself; not re-decoded
  here because it's outside the brief and already plate-commented in Ghidra.)

### 2.4 Side effects

- Triggers `CellClass::RecalcAttributes` per coord — the LAT-retrigger source
  identified in `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`.
- Triggers `FUN_00584550` (zone-graph patch) per coord that survived Pass 1
  without being refilled — i.e. cells whose `RecalcAttributes` didn't restore
  their zone slot (broken bridge tiles, water cells, etc.).
- Does **not** mutate any CellClass `+0x140` flag bits directly. Does not call
  `ValidateBridgeZones` or `UpdateBridgeZonesHelper` (those are the **caller's**
  responsibility).
- Does **not** invoke `RadarClass::MarkTerrainDirty` or `TacticalClass::DirtyScreenRect`.

### 2.5 Caller graph

`get_xrefs_to(0x00586990)`:

```text
0x00586961   FUN_005868a0                              — rectangle iterator wrapper;
                                                         passes its locally-built coord list
0x00569722   FUN_00568e40 (high pavement walker)       — tail call when accumulated cells > 0
0x0056a048   FUN_00569760 (this function's pair)       — tail call when local_8 > 0
0x00570aa4   ProcessBridgeDestruction_Low              — tail call when local_8 > 0
0x00573fc7   ProcessBridgeDestruction_High             — tail call (mirror of above)
0x00571a40   ProcessBridgeDamageStateMachine_Low       — emits coord list during damage step
0x00571f59   ProcessBridgeDamageStateMachine_Low       — second emit site
0x00577135   ProcessBridgeDamageStateMachine_High      — twin of above
0x00577645   ProcessBridgeDamageStateMachine_High      — twin
```

`FUN_005868a0` decompiles to a **rectangle-region driver**: walks `[x, x+w) ×
[y, y+h)` from a 4-int rect arg, appends each in-bounds cell coord to a stack
DynamicVector, then calls `FUN_00586990(&local_18)`. Used by per-cell damage
visualisation paths to redraw a rectangular footprint.

---

## 3. Interaction with bridge cell flags at `cell + 0x140`

`cell + 0x140` is the 32-bit flag word documented in
`BRIDGE_REPAIR_AND_HUT_DEATH §13` and `BRIDGE_RUNTIME_DEEP_DIVE`. The two
functions interact with it as follows:

- **FUN_00569760** does NOT read or write `cell + 0x140` itself. It reads
  `cell + 0x11A` (overlay-state byte: values 0x02, 0x04, 0x05, 0x07, 0x08,
  0x0C identify ramp variants) and `cell + 0x11B` (height-adjust byte: bumps
  `+= 4` on three perpendicular-axis cells when handling a 5-variant ramp
  variant-4). The `+0x140 & 0x2000` "pavement bit" is **read and modified
  indirectly** via `ToggleBridgePavement` (which the walker calls; see
  `PAVEMENT_AND_TILE_PROPAGATION_GHIDRA_REPORT §2.x`). The `+0x140 & 0x80`
  (overlay bit) and `& 0x100` (structural bit) are NOT read here — they are
  read by the caller (`ProcessBridgeDestruction_Low`) to decide *whether* to
  invoke the walker in the first place.
- **FUN_00586990** does NOT read or write `cell + 0x140` directly. It triggers
  `CellClass::RecalcAttributes`, which **does** rewrite `+0x140` extensively
  (LAT bits, drawability bits, etc.) — but those rewrites are not in this
  function's scope.

---

## 4. Active in YR — verdict

| Code path                                                                                       | Active in YR? | Evidence                                                                                                                                                                          |
|-------------------------------------------------------------------------------------------------|---------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `ProcessBridgeDestruction_Low` → FUN_00569760 (dir=2 single-cell endpoint)                      | **Yes**       | Reached on engineer/IFV C4-on-CABHUT and on any low-bridge cell damage that triggers the ramp branch. Verified caller chain in `BRIDGE_REPAIR_AND_HUT_DEATH §3.6`.                |
| `ProcessBridgeDestruction_Low` → FUN_00569760 (dir=4 single-cell endpoint)                      | **Yes**       | Same path, E-W axis branch.                                                                                                                                                       |
| `ProcessBridgeDestruction_Low` → FUN_00569760 (dir=2 / dir=4 5-variant `+4` ramp)               | **Yes**       | Only reached on the specific 5th-variant tile-id match, but that tile is in the standard YR ramp set populated by `Read_Theater_TileSets_INI` (verified xref from `0x00545B88`).   |
| FUN_00569760 dir-codes other than 2 or 4                                                        | **No**        | All call sites pass only 2 or 4. The body has no other typed branch; other values silently fall through the find loop and hit only the overlay-spawn phase. No active caller.     |
| FUN_00586990 from `FUN_005868a0` (rectangle driver)                                             | **Yes**       | Live YR damage visualisation path.                                                                                                                                                |
| FUN_00586990 from any walker tail (`FUN_00569760`, `FUN_00568e40`, ProcessBridgeDestruction_*)  | **Yes**       | All these are live YR destruction paths; the tail call only fires when accumulated cells > 0 (i.e. when the 5-variant +4 branch was hit).                                         |
| FUN_00586990 from `ProcessBridgeDamageStateMachine_*` (4 sites)                                 | **Yes**       | Live YR per-cell damage-state ticks. (Not re-decoded here — owned by other docs.)                                                                                                 |
| TS-only legacy code                                                                             | **None found**| All callers are bridge code paths reachable in standard YR skirmishes. No `SpecialFlags & 0x1000` (fog-of-war) gates, no `Tunnel/Subterranean` references, no `MultiplayerDialogSettings` opt-ins. |

---

## 5. Open Questions

1. **`g_OverlayTypeClass_Array[+0x3B4]` and `[+0x3B8]` identities.** Confirmed
   as global registry slot offsets used to construct destroyed-overlay marker
   `OverlayClass`es. Mapping to the specific INI overlay names (e.g.
   `BRIDGE1`/`BRIDGE2` style markers) was not pursued — out of scope, but a
   straightforward follow-up via xrefs to `g_OverlayTypeClass_Array`.

2. **The unmapped function at `0x0056A080`.** Four xrefs to FUN_00569760 land in
   this region. Mirror-shape against `ProcessBridgeDestruction_Low` strongly
   suggests it is the corresponding High twin (or a near-cousin), but read-only
   constraint prevents `create_function`. A sibling session with write access
   should register and rename it. If it is indeed `ProcessBridgeDestruction_High`,
   then `0x573540` (the currently labelled `ProcessBridgeDestruction_High`) is
   actually a *different* code path — worth re-verifying labels.

3. **Why FUN_00569760 spawns OverlayClass markers when called from the
   destruction path.** Phase B always spawns markers on the walked span (except
   on healthy cells). Caller `ProcessBridgeDestruction_Low` then calls
   `TacticalClass::DirtyScreenRect` using the screen-rect output. The combined
   effect is: collapse-side spawn of debris overlays AND screen-rect redraw.
   This appears to be the destroyed-bridge-overlay placement, but I did NOT
   trace the visual output through the renderer to confirm; that's a render-side
   verification follow-up.

4. **Behaviour when `param_2 ∉ {2, 4}`.** No live caller does this, but a
   theoretical caller would skip both ramp-handling branches and fall through to
   Phase B. The Phase B `else` branch uses `+0x3B8`, so the spawn type would be
   the dir=4 overlay regardless. Not a parity concern (no caller exists) but
   worth noting if anyone considers calling this function from new code.

---

## 6. Sources

- Ghidra MCP `decompile_function(0x00569760)` — full body, 187 lines decompile.
- Ghidra MCP `decompile_function(0x00586990)` — full body, 49 lines decompile.
- Ghidra MCP `disassemble_function(0x00586990)` — used to verify `__thiscall`
  binding and parameter passing.
- Ghidra MCP `decompile_function(0x0056E990)` (`MapClass::ToggleBridgePavement`)
  — used to disprove the "calls FUN_00569760" hypothesis.
- Ghidra MCP `decompile_function(0x00570050)` (`ProcessBridgeDestruction_Low`)
  — used to verify the dir=2 / dir=4 call shapes and the cell-list accumulation
  in the 5-variant +4 branch.
- Ghidra MCP `decompile_function(0x005868A0)` — used to confirm the
  rectangle-region wrapper around FUN_00586990.
- Ghidra MCP `get_xrefs_to(0x00569760, 0x00586990, 0x00abc1e8, 0x00aa0e38)` —
  used for the caller graphs and DAT bucket identities.
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §11, §13, §17, §18 — the
  Phase-2 line-summary entries that this report expands.
- `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md` — original mention
  of "pavement walker" terminology.
- `BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md` §6 — `DAT_0087F8C0` dead-list
  context (not invoked here; FUN_00586990 uses MapClass live structures, not
  the dead-list).
- `MAPCLASS_COMPLETE_DECODE.md` §"Layout table" rows for `+0x6C / +0x70 / +0xF4 /
  +0xF8` — used to identify MapClass offsets referenced by FUN_00586990.
- `PAVEMENT_AND_TILE_PROPAGATION_GHIDRA_REPORT.md` §2 — cross-reference for
  `SetOverlayAndPropagate` (`0x0056EB80`) semantics invoked by FUN_00569760.
