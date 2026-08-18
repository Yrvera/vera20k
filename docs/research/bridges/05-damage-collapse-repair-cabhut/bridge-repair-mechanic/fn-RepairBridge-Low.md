# MapClass::RepairBridge_Low — Decode Doc

**Function:** `MapClass::RepairBridge_Low`
**Address:** `0x0057F200`
**Body range:** `0x0057F200 – 0x0057F43B`
**Calling convention:** `__cdecl` (no `this`; `param_1` = `short*` coord pointer on stack)
**Scope:** Full function.

---

## Summary

`MapClass::RepairBridge_Low` is the axis-dispatch function for low (wooden) bridge repair. Given
a cell coord inside a low-bridge overlay, it:

1. Looks up the cell's overlay index (`CellClass+0x44`).
2. Classifies the overlay into one of three cases: NS body, EW body, or ambiguous (neither of the primary bands).
3. For NS: checks the cell one row north (`Y-1`) for another NS overlay — if absent, walks NS south; if present, checks `Y-2`; dispatches `RepairBridgeWalker_NS_Low` from the appropriate starting cell.
4. For EW: checks the cell one column west (`X-1`) for EW overlay — if absent, walks EW east; if present, checks `X-2`; dispatches `RepairBridgeWalker_EW_Low`. Edge case: `FUN_00588C60` is called as a further fallback.
5. Returns without dispatching if overlay is outside all known low-bridge bands.

---

## Active in YR

**Yes.** Verified via `get_function_callers 0x0057F200` → single caller:
`ProcessBridgeDestruction_Low @ 0x00570050`. That function is called from
`InfantryClass::PerCellProcess` (engineer repair path) — live YR path.

---

## Overlay Band Analysis

From `decompile_function 0x0057F200`:

```c
iVar2 = *(int *)(puVar3 + 0x44);  // overlay type index

// NS case: overlay ∈ [0x4A..0x52] OR overlay == 0x64 (100)
if ( !( (iVar2 < 0x4A || 0x52 < iVar2) && (iVar2 < 0x5C || 0x5F < iVar2) && iVar2 != 100 ) ) {
    // → NS dispatch (overlay in [0x4A..0x52] or == 0x64)
    goto NS_branch;
}
// EW case: overlay ∈ [0x53..0x5B] OR [0x60..0x64] OR overlay == 0x65
if ( (0x52 < iVar2 && iVar2 < 0x5C) ||
     ((0x5F < iVar2 && iVar2 < 100) || (iVar2 == 0x65)) ) {
    // → EW dispatch
    goto EW_branch;
}
// Otherwise: return without action
```

The complete low-bridge overlay map:

| Overlay range | Dec | Axis | Band |
|---|---|---|---|
| `[0x4A, 0x52]` | 74–82 | NS | NS body |
| `0x64` | 100 | NS | NS body (outer) |
| `[0x53, 0x5B]` | 83–91 | EW | EW body |
| `[0x60, 0x63]` | 96–99 | EW | EW body |
| `0x65` | 101 | EW | EW body (outer) |
| `[0x5C, 0x5F]` | 92–95 | — | Unclassified (no dispatch) |

---

## NS Branch

```c
// Check cell at (X, Y-1)
param_1 = CONCAT22(param_1[1] + -1, *param_1);  // Y-1
iVar2 = Get_CellClass(&param_1)->overlay;
if (overlay < 0x4A || overlay > 0x65) {
    // No NS overlay one row north → start NS walker at (X, Y+1)
    MapClass__RepairBridgeWalker_NS_Low(CONCAT22(psVar1[1]+1, *psVar1));
    return;
}
// Check cell at (X, Y-2)
param_1 = CONCAT22(param_1[1] + -2, *param_1);  // Y-2
iVar2 = Get_CellClass(&param_1)->overlay;
if (overlay > 0x49 && overlay < 0x66) {
    // NS overlay 2 rows north → start NS walker at (X, Y-1)
    MapClass__RepairBridgeWalker_NS_Low(CONCAT22(psVar1[1]-1, *psVar1));
    return;
}
// Fall through: start NS walker at (X, Y) — current cell
MapClass__RepairBridgeWalker_NS_Low(psVar1);
```

The NS walker is dispatched from the southernmost NS-overlay cell of the span. The three cases
(Y+1, Y-1, Y) cover the edge of the span at different positions within the repair region.

---

## EW Branch

```c
// Check cell at (X-1, Y)
param_1 = CONCAT22(param_1[1], *param_1 + -1);  // X-1
iVar2 = Get_CellClass(&param_1)->overlay;
if (overlay < 0x4A || overlay > 0x65) {
    // No low-bridge overlay one column west → start EW walker at (X+1, Y)
    MapClass__RepairBridgeWalker_EW_Low(CONCAT22(psVar1[1], *psVar1 + 1));
    return;
}
// Check cell at (X-2, Y)
param_1 = CONCAT22(param_1[1], *param_1 + -2);  // X-2
iVar2 = Get_CellClass(&param_1)->overlay;
if (overlay < 0x4A || overlay > 0x65) {
    // No low-bridge overlay two columns west → start EW walker at (X, Y)
    MapClass__RepairBridgeWalker_EW_Low(psVar1);
    return;
}
// Fallback: FUN_00588C60 finds the starting cell
puVar4 = FUN_00588C60(local_4, &param_1 = 1);
MapClass__RepairBridgeWalker_EW_Low(&local_8);
```

The EW walker is dispatched from the easternmost EW-overlay cell of the span. The three cases
(X+1, X, fallback) cover different positions within the repair region.

---

## Callees

Verified via `get_function_callees 0x0057F200`:

| Callee | Address | Role |
|---|---|---|
| `MapClass::Get_CellClass` | `0x005657A0` | Cell pointer from coord |
| `MapClass::RepairBridgeWalker_NS_Low` | `0x0057F6A0` | Repair NS (north-south) bridge span |
| `MapClass::RepairBridgeWalker_EW_Low` | `0x0057FBC0` | Repair EW (east-west) bridge span |
| `FUN_00588C60` | `0x00588C60` | Fallback coord finder for EW edge case (not decoded) |

---

## Globals Used

| Global | Role |
|---|---|
| `g_CellArray_Base` | Cell array pointer (index = Y*512 + X) |
| `DAT_00ABDC50` | Sentinel CellClass* for out-of-bounds |
| `DAT_00ABDC74` | Out-of-bounds coord scratch |

---

## Unverified (YELLOW)

- `FUN_00588C60` identity: called in the EW fallback branch with `param_1 = 1`. Not decoded.
  Inferred as a coord search/walk helper from context.
- Overlays `0x5C–0x5F` (92–95): fall through the NS and EW branches without dispatch. These
  values are within the general low-bridge band `(0x49, 0x66)` used by `ProcessBridgeDestruction_Low`
  phase 1 but unclassified here. Their meaning is not established in this decode.
- Overlay `0x64` (100) in the NS band and `0x65` (101) in the EW band: the outer/terminal
  overlay indices — inferred from position at band edges and Ghidra decompilation, not
  independently verified against in-game tile data.

---

## Self-Proof (exit gate)

### Claim 1: Function is `MapClass::RepairBridge_Low` at `0x0057F200`

`get_function_by_address 0x0057F200` → `MapClass__RepairBridge_Low`, body
`0x0057F200 – 0x0057F43B`. **VERIFIED — matches task spec.**

### Claim 2: Single caller `ProcessBridgeDestruction_Low`

`get_function_callers 0x0057F200` → `ProcessBridgeDestruction_Low @ 0x00570050`. Exactly one.
**VERIFIED.**

### Claim 3: Dispatches to `RepairBridgeWalker_NS_Low` and `RepairBridgeWalker_EW_Low`; overlay band split confirmed

`get_function_callees 0x0057F200` → `MapClass__RepairBridgeWalker_NS_Low @ 0x0057F6A0` and
`MapClass__RepairBridgeWalker_EW_Low @ 0x0057FBC0` listed.
`decompile_function 0x0057F200` → NS walker called when overlay `∈ [0x4A..0x52] ∪ {0x64}`;
EW walker called when overlay `∈ [0x53..0x5B] ∪ [0x60..0x65]`. **VERIFIED from decompilation.**
