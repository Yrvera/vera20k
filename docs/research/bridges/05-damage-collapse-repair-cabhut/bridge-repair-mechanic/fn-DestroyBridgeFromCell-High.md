# MapClass::DestroyBridgeFromCell_High — Decode Doc

Address: `0x005749C0`  
Scope: Full function.

## Summary

`MapClass::DestroyBridgeFromCell_High` determines the axis (NS or EW) of a high bridge given
a single cell coordinate, then dispatches to `CollapseBridge_EW_High` or
`CollapseBridge_NS_High` with the bridge's anchor cell. It is the high-bridge twin of
`DestroyBridgeFromCell_Low` (low-bridge, `0x00574780`). The function reads `CellClass+0x44`
(overlay subtype) of the input cell to classify it as NS or EW, then checks ±1 and ±2
cells along the appropriate axis to identify the anchor position (start of the bridge), and
calls the corresponding collapse function. Called exclusively from
`MapClass::DestroyBridge_High_OnHutDeath`.

## Active in YR

**Yes.** Verified via `get_function_callers 0x005749C0`: single caller
`MapClass::DestroyBridge_High_OnHutDeath @ 0x00574000`, which is itself called from
`BombClass::Detonate` and `BuildingClass::Update` (live YR paths, verified in task #1 and
task #4 decode docs). No TS-only gating flags.

## Decompilation Excerpt

From `decompile_function 0x005749C0`:

```c
void MapClass__DestroyBridgeFromCell_High(short *param_1)
{
    psVar1 = param_1;  // save original coord
    iVar2 = param_1[1] * 0x200 + (int)*param_1;  // cell index = Y*0x200 + X
    if ((iVar2 < 0) || (0x3ffff < iVar2) ||
        (puVar3 = *(CellClass**)(g_CellArray_Base + iVar2*4), puVar3 == NULL)) {
        DAT_00abdc74 = *(uint32*)param_1;
        puVar3 = &DAT_00abdc50;  // out-of-bounds sentinel
    }
    iVar2 = *(int*)(puVar3 + 0x44);  // CellClass+0x44: overlay subtype

    // ---- NS path: subranges [0xCD..0xD5], [0xDF..0xE2], or == 0xE7 ----
    if (iVar2 in [0xCD..0xD5] || iVar2 in [0xDF..0xE2] || iVar2 == 0xE7) {
        // Check cell at (X, Y-1)
        param_1 = CONCAT22(param_1[1]-1, *param_1);
        iVar2 = MapClass__Get_CellClass(&param_1);
        if (*(int*)(iVar2+0x44) < 0xCD || 0xE8 < *(int*)(iVar2+0x44)) {
            // Y-1 is not a high-bridge tile → original cell is the start; go +1 south
            param_1 = CONCAT22(psVar1[1]+1, *psVar1);
            MapClass__CollapseBridge_EW_High(&param_1);   // 0x00574A80 → 0x00575870
            return;
        }
        // Check cell at (X, Y-2)
        param_1 = CONCAT22(psVar1[1]-2, *psVar1);
        iVar2 = MapClass__Get_CellClass(&param_1);
        if (*(int*)(iVar2+0x44) >= 0xCD && *(int*)(iVar2+0x44) <= 0xE8) {
            // Y-2 also bridge → 3rd entry in sequence; use FUN_00588c60 to find anchor
            param_1 = (short*)0x1;
            puVar4 = FUN_00588c60(local_4, &param_1);
            local_8 = *puVar4;
            MapClass__CollapseBridge_EW_High(&local_8);  // 0x00574AFC → 0x00575870
        } else {
            // Y-2 not bridge → original cell is 2nd; start is psVar1
            MapClass__CollapseBridge_EW_High(psVar1);    // 0x00574ACD → 0x00575870
        }
        return;
    }

    // ---- EW path: subranges [0xD6..0xDE], [0xE3..0xE6], or == 0xE8 ----
    if (iVar2 in [0xD6..0xDE] || iVar2 in [0xE3..0xE6] || iVar2 == 0xE8) {
        // Check cell at (X-1, Y)
        param_1 = CONCAT22(param_1[1], *param_1-1);
        iVar2 = MapClass__Get_CellClass(&param_1);
        if (*(int*)(iVar2+0x44) < 0xCD || 0xE8 < *(int*)(iVar2+0x44)) {
            // X-1 not bridge → original is start; go +1 east
            param_1 = CONCAT22(psVar1[1], *psVar1+1);
            MapClass__CollapseBridge_NS_High(&param_1);  // 0x00574B8D → 0x00575BA0
            return;
        }
        // Check cell at (X-2, Y)
        param_1 = CONCAT22(psVar1[1], *psVar1-2);
        iVar2 = MapClass__Get_CellClass(&param_1);
        if (*(int*)(iVar2+0x44) >= 0xCD && *(int*)(iVar2+0x44) <= 0xE8) {
            // X-2 also bridge → 3rd cell; anchor = psVar1 at X-1
            param_1 = CONCAT22(psVar1[1], *psVar1-1);
            MapClass__CollapseBridge_NS_High(&param_1);  // 0x00574BDA → 0x00575BA0
        } else {
            // X-2 not bridge → start is psVar1 (original)
            MapClass__CollapseBridge_NS_High(psVar1);    // 0x00574C13 → 0x00575BA0
        }
        return;
    }
    // iVar2 not in any recognized subrange → return silently (no-op)
}
```

## Behavioral Analysis

### Overlay subtype classification

The function uses `CellClass+0x44` to classify the input cell's high-bridge tile type.
The complete range of valid high-bridge overlay subtypes spans `[0xCD, 0xE8]` inclusive.
Within this range, tiles are split into two axis families:

| Axis | Subranges | Meaning |
|---|---|---|
| NS (North-South) | `[0xCD..0xD5]`, `[0xDF..0xE2]`, `0xE7` | Bridge running N-S; dispatch to `CollapseBridge_EW_High` |
| EW (East-West) | `[0xD6..0xDE]`, `[0xE3..0xE6]`, `0xE8` | Bridge running E-W; dispatch to `CollapseBridge_NS_High` |

Note the naming inversion: NS-axis tiles dispatch to `CollapseBridge_EW_High`, and EW-axis
tiles dispatch to `CollapseBridge_NS_High`. This is consistent with the decompilation —
the collapse function name refers to the collapse sweep direction, which is perpendicular
to the bridge's own axis.

### Anchor selection (3-position probe)

For each axis, the function determines which of 3 possible positions within the bridge
the input cell occupies (start, middle, or end), then provides the collapse function with
the bridge's start cell:

**NS path (checks Y−1 and Y−2):**
1. If Y−1 is NOT a high-bridge tile → input cell is position 0 (start); pass Y+1 to collapse.
2. If Y−1 IS bridge AND Y−2 is NOT → input cell is position 1 (middle); pass original cell.
3. If both Y−1 and Y−2 are bridge → input cell is position 2 (end); use `FUN_00588C60` helper.

**EW path (checks X−1 and X−2):**
1. If X−1 is NOT a high-bridge tile → input cell is position 0; pass X+1 to collapse.
2. If X−1 IS bridge AND X−2 is NOT → input cell is position 1; pass original cell.
3. If both X−1 and X−2 are bridge → input cell is position 2; pass X−1 to collapse.

For EW case 3, unlike NS case 3 which calls `FUN_00588C60`, the anchor is simply
`(psVar1[1], *psVar1 - 1)` (X−1). `FUN_00588C60` is only called for NS position-2.

### Call site verification

All three `CollapseBridge_NS_High` call sites verified via `get_xrefs_from`:
- `0x00574B8D` → `CollapseBridge_NS_High @ 0x00575BA0` (EW position 0)
- `0x00574BDA` → `CollapseBridge_NS_High @ 0x00575BA0` (EW position 1)
- `0x00574C13` → `CollapseBridge_NS_High @ 0x00575BA0` (EW position 2)

`CollapseBridge_EW_High @ 0x00575870` call sites verified via `get_xrefs_to 0x00575870`:
- `0x00574A80` (NS position 0), `0x00574ACD` (NS position 1), `0x00574AFC` (NS position 2)
— all from `MapClass__DestroyBridgeFromCell_High`.

## Struct Field Accesses

`param_1` is `short*` — cell coordinate pair (X=`*param_1`, Y=`param_1[1]`), cell units.

| Source | Offset | Field | Role |
|---|---|---|---|
| CellClass (from Get_CellClass) | `+0x44` | Overlay subtype | High-bridge tile family; range `[0xCD..0xE8]` |

All other state is local: the function does not read any BuildingClass or MapClass
instance fields beyond `g_CellArray_Base` for cell lookup.

## Globals Referenced

| Global | Role |
|---|---|
| `g_CellArray_Base` | Cell array base for `Y*0x200 + X` indexed lookup |
| `DAT_00abdc50` | Sentinel null-cell for out-of-bounds coords |
| `DAT_00abdc74` | Scratch coord storage on out-of-bounds (written by Get_CellClass) |

## Out-of-scope Refs

- `MapClass::CollapseBridge_NS_High` @ `0x00575BA0` — decode task #9
- `MapClass::CollapseBridge_EW_High` @ `0x00575870` — decode task #10
- `FUN_00588C60` — called for NS position-2 anchor lookup; purpose partially unclear
- `MapClass::DestroyBridge_High_OnHutDeath` @ `0x00574000` — sole caller; decode task #4

## Unverified Claims (YELLOW)

- The "naming inversion" observation (NS tiles → EW collapse, EW tiles → NS collapse) is
  directly read from decompilation: NS subranges call `CollapseBridge_EW_High`, EW
  subranges call `CollapseBridge_NS_High`. This is counter-intuitive and may reflect that
  collapse function names describe the sweep direction, not the bridge axis. The exact
  naming convention should be confirmed when decoding tasks #9 and #10.
- `FUN_00588C60` for NS position-2 is an unknown helper. The EW position-2 case does NOT
  call it — it directly computes X−1. The asymmetry is present in the decompilation but
  the reason is not decoded here.
- The `CellClass+0x44` field is confirmed as "overlay subtype" in context of high-bridge
  range `[0xCD..0xE8]`. Verified here matches the range cited in task description. Full
  field semantics to be confirmed by `decode-struct-CellClass_BridgeFields` (task #21).
