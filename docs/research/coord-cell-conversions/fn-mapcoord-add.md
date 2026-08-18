# MapCoord_Add — decode doc

**Address:** `0x0042d510`
**Kind:** function-decode-v1
**Source:** decompile_function @ 0x0042d510

---

## Summary

`MapCoord_Add` (0x0042d510) adds two packed cell coordinates (CellStruct — two adjacent
`short` fields, X then Y) component-wise and writes the result into a caller-provided
`undefined4*` output buffer using `CONCAT22(a.Y + b.Y, a.X + b.X)`. It is the canonical
cell-neighbor-step primitive: 52 callers across building placement, overlay destruction,
bridge logic, ramp updates, radar, AI perimeter scanning, path smoothing, and slave
management all use it to offset a current cell index by a precomputed delta (a foundation
outline offset, a cardinal direction step, or a bridge tile direction vector).

No callees — the entire body is a short-arithmetic expression and one store.

---

## Active in YR

**YES — actively called in normal YR skirmish play.**

52 callers confirmed via `get_function_callers 0x0042d510` (re-verified, limit=100). Named callers include:

- `BuildingClass__Unlimbo` @ `0x00440580` — iterates foundation cells when placing a building
- `CellClass__ApplyLAT_and_SlopeFixup` @ `0x0047ca80` — LAT auto-transition neighbor lookup
- `CellClass__DestroyOverlay` @ `0x00480cb0` — wall chain-reaction and destruction neighbor walk
- `CellClass__GetRadarColor` @ `0x00587410` — radar color computation (neighbor check)
- `CellClass__PostDestructionWallCleanup` @ `0x00480630` — wall connectivity frame update
- `HouseClass__AI_ScanBasePerimeter` @ `0x005082c0` — AI perimeter scan stepping along edges
- `MapClass__AddBridgeZoneEdges` @ `0x005851b0` — bridge zone edge insertion
- `MapClass__ApplyBridgeDestruction_NS_High` @ `0x0057e7a0` — bridge span destruction (NS, high)
- `MapClass__ApplyBridgeDestruction_NS_Low` @ `0x0057dd50` — bridge span destruction (NS, low)
- `MapClass__BridgePavementSpanWalker` @ `0x00569760` — bridge pavement walker
- `MapClass__ComputeBridgeAdjacencyMask_Low` @ `0x00579b70` — bridge adjacency mask (low)
- `MapClass__RemoveBridgeZoneEdges` @ `0x00584e50` — bridge zone edge removal
- `MapClass__RepairBridgeWalker_EW_High` @ `0x00580600` — bridge repair walker (EW, high)
- `MapClass__RepairBridgeWalker_EW_Low` @ `0x0057fbc0` — bridge repair walker (EW, low)
- `MapClass__RepairBridgeWalker_NS_High` @ `0x005800d0` — bridge repair walker (NS, high)
- `MapClass__RepairBridgeWalker_NS_Low` @ `0x0057f6a0` — bridge repair walker (NS, low)
- `MapClass__Resize` @ `0x00565c10` — map resize operation
- `MapClass__UpdateRamp_EW_CollapseA_High` @ `0x00572da0` — ramp collapse update (EW, A, high)
- `MapClass__UpdateRamp_EW_CollapseA_Low` @ `0x0056f8b0` — ramp collapse update (EW, A, low)
- `MapClass__UpdateRamp_EW_CollapseB_High` @ `0x00573170` — ramp collapse update (EW, B, high)
- `MapClass__UpdateRamp_EW_CollapseB_Low` @ `0x0056fc80` — ramp collapse update (EW, B, low)
- `MapClass__UpdateRamp_NS_CollapseA_High` @ `0x00572440` — ramp collapse update (NS, A, high)
- `MapClass__UpdateRamp_NS_CollapseA_Low` @ `0x0056ef50` — ramp collapse update (NS, A, low)
- `MapClass__UpdateRamp_NS_CollapseB_High` @ `0x005727e0` — ramp collapse update (NS, B, high)
- `MapClass__UpdateRamp_NS_CollapseB_Low` @ `0x0056f2f0` — ramp collapse update (NS, B, low)
- `Path_smooth_single_segment` @ `0x0042b420` — drive locomotor path smoothing
- `ProcessBridgeDamageStateMachine_High` @ `0x00576ba0` — bridge damage state machine (high)
- `ProcessBridgeDamageStateMachine_Low` @ `0x00571490` — bridge damage state machine (low)
- `ProcessBridgeDestruction_High` @ `0x00573540` — bridge destruction (high)
- `ProcessBridgeDestruction_Low` @ `0x00570050` — bridge destruction (low)
- `RepairBridgeSegment` @ `0x00575ee0` — bridge segment repair
- `SlaveManagerClass__AI_Update` @ `0x006af6c0` — Yuri slave AI update
- `SlaveManagerClass__IsSlaveAtCell` @ `0x006b0880` — slave-at-cell check
- `BuildingPlacement_OverlayRenderer` @ `0x006d5030` — building placement overlay render
- + 18 FUN_* callers (addresses: 0x00484ae0, 0x00484d60, 0x005060b0, 0x0050c340, 0x00568e40,
  0x0056a080, 0x00582d70, 0x00583c60, 0x00585930, 0x00586bf0, 0x0058c800, 0x0058d620,
  0x005a08d0, 0x005a1fb0, 0x005a4b60, 0x005a5360, 0x006b2a70, 0x00738d30)

None gated behind a TS-only flag. Reachable during normal YR skirmish.
(verified via `get_function_callers 0x0042d510`, limit=100, 52 total)

---

## Signature

```c
// verified via decompile_function 0x0042d510
void __thiscall MapCoord_Add(short *param_1, undefined4 *param_2, short *param_3)
```

**Calling convention:** `__thiscall` — `param_1` (`this`) arrives in ECX.

| Param | Type | Semantics |
|---|---|---|
| `param_1` | `short *` | Source A (current cell coord): `param_1[0]` = X cell, `param_1[1]` = Y cell |
| `param_2` | `undefined4 *` | Output: receives `CONCAT22(Y_result, X_result)` — packed short pair |
| `param_3` | `short *` | Source B (delta / offset): `param_3[0]` = dX, `param_3[1]` = dY |

**Return value:** void. The result is in `*param_2`. Callers typically dereference the
returned `short*` (Ghidra reuses `param_3` as the expression result — the actual store
goes to `*param_2`).

---

## Control Flow

```c
// verified via decompile_function 0x0042d510
void __thiscall MapCoord_Add(short *param_1, undefined4 *param_2, short *param_3) {
    param_3 = (short *)CONCAT22(param_3[1] + param_1[1], *param_3 + *param_1);
    *param_2 = param_3;
    return;
}
```

No branches, no guards. Unconditional component-wise addition:
- `X_out = param_3[0] + param_1[0]`  (low short)
- `Y_out = param_3[1] + param_1[1]`  (high short)
- `*param_2 = CONCAT22(Y_out, X_out)` — packed into one `undefined4`

`CONCAT22(hi, lo)` is a Ghidra macro: `(hi << 16) | (lo & 0xFFFF)`.

---

## CellStruct Layout

| Byte offset | Size | Field | Units |
|---|---|---|---|
| `+0x00` | `short` (2 bytes) | X cell index | cell units, +X = east |
| `+0x02` | `short` (2 bytes) | Y cell index | cell units, +Y = south |

Total: **4 bytes**. The packed form stores X in the low 16 bits and Y in the high 16 bits.
This matches the layout at `CellClass+0x24` described in `fn-get-center-coords.md`.

`param_1` is `short*` — so `param_1[0]` = byte offset 0 = X, `param_1[1]` = byte
offset 2 = Y. NOT the int* × 4 rule from CLAUDE.md — this is short* × 2.

---

## Globals

None. The function body contains no global reads or writes.
(verified via `decompile_function 0x0042d510`)

---

## INI Keys

None. This is a pure arithmetic utility.

---

## Enum Values

None. The X and Y components are cell-unit short values whose semantics depend on
caller context.

---

## Observable vs Internal

**Observable:** Any caller that passes the result to `MapClass__Get_CellClass` and then
reads or modifies that cell produces player-visible state (wall chain reactions, overlay
removal, building placement, bridge destruction). A wrong output directly moves which
cell is targeted by one or more cells.

**Internal:** The intermediate packed `undefined4` result — players see only the downstream
effect (which cell gets targeted), not the raw packed short arithmetic.

---

## Caller Pattern Analysis

Five distinct patterns found across sampled callers:

### Pattern A — Foundation outline iteration (`BuildingClass__Unlimbo`)

```c
// verified via decompile_function 0x00440580
// GetFoundation() returns array of short[2] cell deltas terminated by (0x7FFF, 0x7FFF)
piVar = GetFoundation(BuildingTypeClass*);
while (piVar[0] != 0x7FFF) {
    puVar = (undefined4 *)MapCoord_Add(&baseCell, &outBuf, piVar);
    cellCoord = *puVar;
    CellClass* pCell = MapClass__Get_CellClass(&cellCoord);
    // ... operate on pCell (place building tile, set occupancy, etc.)
    piVar += 2;  // advance to next foundation cell delta
}
```

`param_1` = `&baseCell` (NW-corner cell, cell units — Get_Cell_Packed frame).
`param_3` = current `short[2]` element from `GetFoundation()` array (cell deltas relative
to NW corner — **Foundation outline frame**, CLAUDE.md frame #4).
Output: absolute cell index for each foundation cell tile.

### Pattern B — Cardinal direction neighbor stepping (`CellClass__ApplyLAT_and_SlopeFixup`, `CellClass__DestroyOverlay`)

```c
// verified via decompile_function 0x0047ca80 and decompile_function 0x00480cb0
// g_DirectionOffsets is an array of short[2] per-direction steps {dX, dY}
puVar = (undefined4 *)MapCoord_Add(&localCoord, &outBuf,
                                   &g_DirectionOffsets + (dirIndex & 7));
cellCoord = *puVar;
CellClass* pNeighbor = MapClass__Get_CellClass(&cellCoord);
```

`param_1` = current cell coord (short pair at a local variable).
`param_3` = `g_DirectionOffsets + (dir & 7)` — element of the global 8-entry direction
step table, one `short[2]` per cardinal/diagonal direction.
This is the fundamental "step N/E/S/W from current cell" pattern; the loop increments
the direction index (`uVar14 += 2` in `DestroyOverlay`; `uVar14 += 2` also in `ApplyLAT`)
to iterate all four cardinal neighbors.

### Pattern C — Bridge zone edge stepping (`MapClass__AddBridgeZoneEdges`)

```c
// verified via decompile_function 0x005851b0
// Adds a direction step derived from bridge tile orientation bitmask
puVar = (undefined4 *)MapCoord_Add(&bridgeCell, &outBuf, &directionOffsets + stepDir);
cellCoord = *puVar;
// Insert zone edge at resulting cell for bridge zone marking
```

`param_1` = current bridge cell coord.
`param_3` = direction-step entry selected by bridge tile orientation bitmask.
Pattern is structurally identical to Pattern B but the direction is determined by bridge
geometry rather than a loop over all directions.

### Pattern D — AI perimeter scan step-and-sample (`HouseClass__AI_ScanBasePerimeter`)

```c
// verified via decompile_function 0x005082c0
// Walks 4 sides of a bounding rectangle, stepping by g_DirectionOffsets each tick
puVar18 = (undefined4 *)MapCoord_Add(local_50, puVar18); // step + store
// Also called with local_44, local_4c, local_48 for secondary candidate coords
```

`param_1` = current walk position (one of several local short-pair buffers).
`param_3` = direction step for the current rectangle side.
The outer loop iterates over 4 sides; each side walks `local_9c` steps from a corner
to the next corner. Uses MapCoord_Add to advance position cell-by-cell.

### Pattern E — Inline form (callers that bypass MapCoord_Add)

Several callers use the equivalent inline expression directly rather than calling
`MapCoord_Add`:

```c
// from CellClass__DestroyOverlay (inline, not calling MapCoord_Add):
param_2 = CONCAT22(*(short *)((int)&g_DirectionOffsets + (uVar14 & 7) * 4 + 2) + uVar4,
                   *(short *)(&g_DirectionOffsets + (uVar14 & 7)) + uVar2);
```

This confirms `MapCoord_Add` is not always called — the compiler inlines the expression in
hot loops. Semantics are identical; `MapCoord_Add` is the canonical form.

---

## Callees

None. (verified via `get_function_callees 0x0042d510`)

---

## Overflow / Edge Behavior

`short + short` wraps at `±32767`. Whether this is a problem depends on the operand range:

- **Foundation outline deltas** (`BuildingClass__Unlimbo`, Pattern A): typically `-2..+4` cells.
  Safe for all legal map positions.
- **Direction steps** (`g_DirectionOffsets`, Patterns B–D): `{-1,0,+1}` values per axis.
  Safe unless the current cell is at map edge (index 0 or map_W/map_H - 1). At the map
  boundary, `0 + (-1) = -1` wraps to `0x7FFF` on a `signed short` if sign-extended to int,
  or stays `-1` as a raw short — callers must range-check before using the result.
  `MapClass__Is_Cell_In_Playfield` is called in some callers (e.g., `HouseClass__AI_ScanBasePerimeter`)
  before dereferencing the stepped-to cell, suggesting edge overflow is a known hazard.
- **Bridge offset steps** (Pattern C): similar small deltas; same map-edge caveat applies.

The function itself performs no bounds check. Callers are responsible for staying in-range.

---

## Reference Frames at Callsites

| Pattern | `param_1` frame | `param_3` frame | Output frame |
|---|---|---|---|
| A (foundation) | Get_Cell_Packed (NW cell, cells) | Foundation outline (cell deltas relative to NW) | Get_Cell_Packed (absolute cell coord, cells) |
| B (cardinal step) | Get_Cell_Packed (current cell, cells) | Direction step from `g_DirectionOffsets` (cell delta ±1) | Get_Cell_Packed (neighbor cell, cells) |
| C (bridge step) | Get_Cell_Packed (bridge cell, cells) | Direction step (cell delta) | Get_Cell_Packed (adjacent cell, cells) |
| D (AI perimeter) | Get_Cell_Packed (walk position, cells) | Direction step per side | Get_Cell_Packed (next position, cells) |

All inputs and outputs are in **cell units (short pair, NW cell / Get_Cell_Packed frame)**.

---

## Struct Field Accesses

No `this`-object fields accessed directly by this function. The body touches only its
parameters. Struct layout is imposed by calling convention and caller construction.

---

## Rust Equivalent

```rust
// CellStruct: packed short pair — X (low 16 bits), Y (high 16 bits). Cell units.
// from Get_Cell_Packed (NW cell, cell units): short X = low half, short Y = high half.
#[repr(C)]
struct CellStruct {
    x: i16,  // cell index, +X = east
    y: i16,  // cell index, +Y = south
}

fn mapcoord_add(a: CellStruct, b: CellStruct) -> CellStruct {
    CellStruct {
        x: a.x.wrapping_add(b.x),
        y: a.y.wrapping_add(b.y),
    }
}
```

Use `wrapping_add` to match the binary's short overflow behavior. Callers should
bounds-check the result before indexing into the cell array.

---

## Out-of-scope refs

| Symbol | Address | Reason deferred |
|---|---|---|
| `g_DirectionOffsets` | global array | 8-entry short[2] table of cardinal+diagonal cell steps. Layout and values need separate decode. |
| `GetFoundation` | BuildingTypeClass vtable+0x90 | Returns short[2] delta array. Verified in fn-coordstruct-set.md as a known pattern. |
| `MapClass__Get_CellClass` | — | Converts packed cell coord to CellClass*. Active-in-YR, not this decode's scope. |
| `MapClass__Is_Cell_In_Playfield` | — | Range-check guard used by callers at map edges. |
| `CONCAT22` | Ghidra macro | `(hi << 16) | (lo & 0xFFFF)` — compiler emits no subroutine call. |

---

## Unverified

All claims verified from live Ghidra decompilation. Caller count corrected from 30 → 52 after two proofer audits (original session used a
stale/incomplete caller list; re-ran `get_function_callers 0x0042d510` with limit=100
in this session to produce the accurate count of 52).
- `decompile_function 0x0042d510` — main function body
- `get_function_callers 0x0042d510` — 52 callers (limit=100, re-verified)
- `get_function_callees 0x0042d510` — no callees
- `decompile_function 0x00440580` — `BuildingClass__Unlimbo` (Pattern A)
- `decompile_function 0x0047ca80` — `CellClass__ApplyLAT_and_SlopeFixup` (Pattern B)
- `decompile_function 0x005851b0` — `MapClass__AddBridgeZoneEdges` (Pattern C)
- `decompile_function 0x005082c0` — `HouseClass__AI_ScanBasePerimeter` (Pattern D)
- `decompile_function 0x00480cb0` — `CellClass__DestroyOverlay` (Pattern B + inline form)
