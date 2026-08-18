# fn-ProcessBridgeDestruction-High

**Runbook:** function-decode-v1
**Target:** `ProcessBridgeDestruction_High @ 0x00573540`
**Confidence:** HIGH — function identity confirmed via `get_function_by_address 0x00573540`; decompilation via `decompile_function 0x00573540`; callers confirmed via `get_function_callers 0x00573540`; callees via `get_function_callees 0x00573540`.
**YR-active:** YES — called from `InfantryClass__PerCellProcess` during engineer bridge repair in a standard YR skirmish.

---

## Function Identity

Verified via `get_function_by_address 0x00573540`:
```
Function: ProcessBridgeDestruction_High at 00573540
Signature: undefined ProcessBridgeDestruction_High(void)
Entry: 00573540
Body: 00573540 - 00573ff6
```
Body size: `0x00573FF6 - 0x00573540 = 0xAB6` bytes (~2742 bytes). This is a large function.

The `__thiscall` decompiler signature assigns `param_1` (ECX = MapClass* this) and `param_2` (stack = cell coord pointer).

---

## Callers

Verified via `get_function_callers 0x00573540`:
```
InfantryClass__PerCellProcess @ 00519630
ProcessBridgeDestruction_High @ 00573540   ← recursive self-call
```

Two callers:
1. `InfantryClass__PerCellProcess` — the engineer repair action entry point.
2. `ProcessBridgeDestruction_High` itself — recursive calls for adjacent span cells (two sites in the body; see §Recursion below).

---

## Callees

Verified via `get_function_callees 0x00573540`:

| Callee | Address | Role |
|---|---|---|
| `MapClass__Get_CellClass` | `0x005657A0` | Cell lookup by packed coord |
| `MapClass__RepairBridge_High` | `0x0057F440` | Repair a single high-bridge tile |
| `MapClass__SetOverlayAndPropagate` | `0x0056EB80` | Set overlay tile + propagate to neighbors |
| `MapClass__ToggleBridgePavement` | `0x0056E990` | Toggle pavement overlay on a cell |
| `MapClass__ValidateBridgeZones` | `0x0056DB70` | Validate bridge zone connectivity |
| `MapClass__UpdateBridgeZonesHelper` | `0x0056C510` | Rebuild bridge zone graph |
| `MapClass__RecalcCellsAndRebuildZones` | `0x00586990` | Full zone recalc after repair |
| `MapCoord_Add` | `0x0042D510` | Add direction offset to cell coord |
| `ProcessBridgeDestruction_High` | `0x00573540` | Recursive self-call |
| `TacticalClass__DirtyScreenRect` | `0x006D2790` | Mark screen region dirty for redraw |
| `FUN_00568E40` | `0x00568E40` | (compute dirty rect for cell) |
| `FUN_0042FCB0` | `0x0042FCB0` | (coord/zone init helper) |
| `FUN_0042F860` | `0x0042F860` | (MapCoord helper) |
| `FUN_007C8B3D` | `0x007C8B3D` | (zone list dealloc) |

---

## High-Level Structure

The function implements the **high-bridge repair orchestration** pass. It:

1. **Scans a 5×5 region** centered on the input cell for any high-bridge tile (`overlay ∈ [0xCD..0xE8]`). If found, delegates immediately to `MapClass__RepairBridge_High` and returns — the cell itself is already repairable.
2. **Finds the nearest bridge-connected cell** by walking direction offsets from the input cell through up to 3 cells in each of 8 directions, checking `CellClass + 0x140` flags for bits `0x100` (NS-bridge segment) and `0x400` (EW-bridge segment).
3. **Dispatches on cell type** (identified by comparing the current-frame tile index against known bridge tile indices stored in `DAT_00abad30`, `DAT_00aa1028`, etc.) and the repair state byte at `CellClass + 0x11A`:
   - Pavement end-cap tiles (`repair state 0x08` or `0x0C`): calls `MapClass__ToggleBridgePavement` + dirties screen.
   - Mid-span tiles with `repair state 0x05` or `0x07`: calls `MapClass__SetOverlayAndPropagate` to restore the mid-span overlay, updates 3 adjacent cell walk-state bytes (`CellClass + 0x11B += 4`), calls `MapClass__ValidateBridgeZones`, queues affected cells for zone recalc, then **recursively calls `ProcessBridgeDestruction_High`** on an adjacent cell offset by `(-2, 0)` or `(0, -2)` cells to continue repairing along the span.
4. **Zone rebuild**: if `MapClass__ValidateBridgeZones` returned non-zero (zones changed), calls `MapClass__UpdateBridgeZonesHelper`.
5. **Zone recalc**: if any cells were queued (local_8 > 0), calls `MapClass__RecalcCellsAndRebuildZones`.

---

## Entry Scan: High-Bridge Overlay Check

Decompilation (entry loop, `iVar10`/`iVar9` from −2 to +2 inclusive):

```c
iVar10 = -2;
do {
    iVar9 = -2;
    do {
        param_2 = CONCAT22((short)iVar9 + coord.Y, (short)iVar10 + coord.X);
        iVar5 = MapClass__Get_CellClass(&param_2);
        if ((0xcc < *(int *)(iVar5 + 0x44)) && (*(int *)(iVar5 + 0x44) < 0xe9)) {
            MapClass__RepairBridge_High(&param_2);
            return;
        }
        iVar9++;
    } while (iVar9 < 3);
    iVar10++;
} while (iVar10 < 3);
```

**`CellClass + 0x44`** = overlay tile index field.
**High bridge overlay range**: `0xCC < index < 0xE9` → tiles `[0xCD..0xE8]` inclusive.
This is the same range used by `CollapseBridge_NS_High` and `CollapseBridge_EW_High` (confirmed in prior decode sessions).

If any cell in the 5×5 box contains a high-bridge overlay, `MapClass__RepairBridge_High` is called for that cell and the function returns immediately.

---

## Bridge-Connected Cell Search

After the scan fails (no existing high-bridge tile found — the bridge is fully collapsed), the function searches for a connected cell to use as the repair anchor:

```c
// CellClass+0x140 flag bits:
//   0x100 = NS bridge segment
//   0x400 = EW bridge segment
if ((*(uint *)(puVar7 + 0x140) & 0x500) == 0) {
    // walk up to 8 directions, 3 cells each, looking for flag 0x100 or 0x400
}
```

The `0x500` mask checks both `0x100` and `0x400` simultaneously (either NS or EW segment). If the origin cell already has one of these flags, the search is skipped.

---

## Repair State Dispatch

Once the target cell is found, the function reads `CellClass + 0x140` flags again and dispatches:

| Flag bits | Path |
|---|---|
| `0x100` set | NS bridge segment path |
| `0x400` set | EW bridge segment path |
| Neither | Falls through to zone recalc only |

Within each path, further dispatch is based on the tile index (`iVar10`) vs. known constants (`DAT_00abad30`, `DAT_00aa1028`, etc.) and the repair state byte at `CellClass + 0x11A`:

| repair state | action |
|---|---|
| `'\b'` (0x08) | `MapClass__ToggleBridgePavement` + dirty screen (end-cap) |
| `'\x05'` (0x05) | `MapClass__SetOverlayAndPropagate` on end tile; walk-state `+= 4` on 3 adjacent cells; validate zones; recursive call with `(-2, 0)` offset; dirty screen |
| `'\a'` (0x07) | same as 0x05 but for the other orientation; recursive call with `(0, -2)` offset |
| `'\f'` (0x0C) | `MapClass__ToggleBridgePavement` + dirty screen (variant end-cap) |

**Recursion sites:**
```c
// NS span continuation (state 0x05):
local_38 = CONCAT22(sStack_2e, (short)uVar11 + -2);  // coord.X -= 2
ProcessBridgeDestruction_High(&local_38);

// EW span continuation (state 0x07):
local_38 = CONCAT22(sStack_2e + -2, (short)uVar11);  // coord.Y -= 2
ProcessBridgeDestruction_High(&local_38);
```

Each recursive call advances 2 cells along the span axis, repairing the next segment.

---

## Walk-State Byte Update

When a mid-span cell is repaired, three adjacent cells have their walk-state byte incremented:

```c
puVar7[0x11b] = puVar7[0x11b] + '\x04';   // += 4
```

**`CellClass + 0x11B`** = walk-state / passability byte. Incrementing by 4 likely re-opens the cell for unit movement over the repaired bridge section. Three cells are updated per repair event (the end cell plus two adjacent span cells, identified via direction offsets `g_DirectionOffsets` and `DAT_0089F698` / `DAT_0089F690`).

---

## Zone Rebuild Sequence

After the main repair actions:

```c
uVar3 = MapClass__ValidateBridgeZones(&local_40);  // returns non-zero if zones changed
// ...
if ((char)param_2 != '\0') {
    MapClass__UpdateBridgeZonesHelper();
}
// ...
if (0 < local_8) {
    MapClass__RecalcCellsAndRebuildZones(&local_18);
}
```

`local_8` is a count of cells added to the zone-recalc queue during the dispatch. A vector (`local_18`) accumulates cell coords as the span is repaired, then `MapClass__RecalcCellsAndRebuildZones` processes the whole batch at the end.

---

## Relationship to ProcessBridgeDestruction_Low

`ProcessBridgeDestruction_Low @ 0x00570050` is the structural twin for low bridges. Both share the same algorithmic shape:
- 5×5 scan → immediate RepairBridge_X → return
- Direction-walk to find connected cell
- Dispatch on tile index + repair state byte
- Recursive self-call on adjacent span cell
- Zone rebuild/recalc batch at end

The high-bridge variant checks `overlay ∈ [0xCD..0xE8]` vs. the low-bridge range `[0x4A..0x65]`. Both are called from `InfantryClass__PerCellProcess` and recurse into themselves only.

---

## Key Constants / Globals

| Symbol | Address | Value/Role |
|---|---|---|
| `DAT_00abad30` | `0x00ABAD30` | Base overlay index for high bridge NS span tiles |
| `DAT_00aa1028` | `0x00AA1028` | Base overlay index for high bridge EW span tiles |
| `DAT_00aa1548` | `0x00AA1548` | Overlay index for high bridge end-cap variant |
| `DAT_00aa0740` | `0x00AA0740` | Overlay index for high bridge end-cap variant 2 |
| `DAT_00aa0e28` | `0x00AA0E28` | Frame counter / tick base for repair timing |
| `DAT_00abc2b4` | `0x00ABC2B4` | Frame stamp comparison A (repair phase gate) |
| `DAT_00aa1130` | `0x00AA1130` | Frame stamp comparison B (repair phase gate) |
| `g_CellArray_Base` | runtime | Base of cell object pointer array |
| `g_DirectionOffsets` | `0x0089F680` (approx) | Array of 8 direction (dx,dy) offsets |
| `DAT_0089F698` | `0x0089F698` | Direction offset entry (second neighbor) |
| `DAT_0089F690` | `0x0089F690` | Direction offset entry (third neighbor) |
| `DAT_00abdc50` | `0x00ABDC50` | Sentinel/dummy CellClass for out-of-bounds coords |
| `DAT_00abdc74` | `0x00ABDC74` | Out-of-bounds coord storage |

---

## Out-of-Scope References

- `MapClass__RepairBridge_High @ 0x0057F440` — individual tile repair; not decoded here.
- `MapClass__SetOverlayAndPropagate @ 0x0056EB80` — overlay write + propagation; not decoded here.
- `MapClass__ValidateBridgeZones @ 0x0056DB70` — zone validation; not decoded here.
- `MapClass__RecalcCellsAndRebuildZones @ 0x00586990` — full zone recalc; not decoded here.
- `MapClass__ToggleBridgePavement @ 0x0056E990` — pavement toggle; not decoded here.
- `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` — decoded in task #11.
- `FUN_00568E40`, `FUN_0042FCB0`, `FUN_0042F860`, `FUN_007C8B3D` — helpers; not decoded here.
- `CellClass + 0x140` flag semantics (full bitfield) — not fully decoded in this session.
- `CellClass + 0x11A` (repair state byte) and `CellClass + 0x11B` (walk-state byte) — usage confirmed here; full struct context in task #21 doc.

---

## Summary

| Field | Value |
|---|---|
| Address | `0x00573540` |
| Body | `0x00573540 – 0x00573FF6` |
| Callers | `InfantryClass__PerCellProcess`, self (recursive) |
| Purpose | High-bridge repair orchestration: scan → identify → dispatch → recurse → zone rebuild |
| Entry scan range | `CellClass+0x44 ∈ [0xCD..0xE8]` → immediate `RepairBridge_High` + return |
| Recursion | Self-calls on `(X−2, Y)` or `(X, Y−2)` to continue along the span |
| Zone rebuild | `ValidateBridgeZones` → `UpdateBridgeZonesHelper` → `RecalcCellsAndRebuildZones` |

---

## Self-Proof (exit gate)

### Claim 1: Function at `0x00573540` is `ProcessBridgeDestruction_High`, body `0x00573540 – 0x00573FF6`

`get_function_by_address 0x00573540` → `Function: ProcessBridgeDestruction_High at 00573540`, body `00573540 - 00573ff6`. **VERIFIED.**

### Claim 2: Callers are `InfantryClass__PerCellProcess` and `ProcessBridgeDestruction_High` (self)

`get_function_callers 0x00573540` → `InfantryClass__PerCellProcess @ 00519630` and `ProcessBridgeDestruction_High @ 00573540`. **VERIFIED.**

### Claim 3: `MapClass__RepairBridge_High` is in the callee list at `0x0057F440`

`get_function_callees 0x00573540` → includes `MapClass__RepairBridge_High @ 0057f440`. **VERIFIED.**

---

## Unverified (YELLOW)

- The exact addresses of the two recursive call sites within the body (`ProcessBridgeDestruction_High(&local_38)`) are visible in the decompilation but their raw addresses were not extracted via `get_assembly_context`. The recursive call pattern is confirmed structurally from the decompilation.
- `DAT_00abad30`, `DAT_00aa1028`, `DAT_00aa1548`, `DAT_00aa0740` exact values not read via `inspect_memory_content` in this session — the symbol names are from the decompilation and may reflect Ghidra labels from prior sessions.
- `CellClass + 0x140` full bitfield definition — only bits `0x100` (NS segment) and `0x400` (EW segment) confirmed by decompilation context; other bits not decoded here.
- Walk-state byte `CellClass + 0x11B` += 4 semantics: interpreted as re-opening cell for movement, but the exact passability encoding is not decoded in this session.
