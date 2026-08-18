# fn MapClass::RepairBridge_High — Decode Doc

**Function:** `MapClass::RepairBridge_High`
**Address:** `0x0057F440`
**Body range:** `0x0057F440 – 0x0057F69F`
**Calling convention:** `__cdecl` — `param_1` = anchor cell coord pointer (short*)

Scope: Full body — high-bridge direction-detect + walker dispatcher. Twin of `RepairBridge_Low`
for high (concrete) bridges.

---

## Summary

`RepairBridge_High` is called from `ProcessBridgeDestruction_High @ 0x00573540` when an
engineer is repairing a high bridge. It takes an anchor cell coord (within the high bridge
overlay band `[0xCD..0xE8]`) and dispatches to the correct directional walker
(`RepairBridgeWalker_NS_High` or `RepairBridgeWalker_EW_High`) based on the overlay value and
the cells in the NS/EW direction from the anchor.

The dispatch logic is structurally identical to `MapClass::DestroyBridgeFromCell_High` (the
destruction-side twin), but calls repair walkers instead of collapse walkers.

---

## Active in YR

**YES.** Called from `ProcessBridgeDestruction_High @ 0x00573540` (confirmed via
`get_function_callers 0x0057F440`), which is in the engineer repair path for high bridges.
Fires once per bridge repair event on the first tick that the engineer is at the repair site.

---

## Callers

From `get_function_callers 0x0057F440`:

| Caller | Address | Notes |
|---|---|---|
| `ProcessBridgeDestruction_High` | `0x00573540` | High-bridge repair orchestrator |

---

## Logic — Direction Detection and Dispatch

Input: anchor cell coord, expected to contain a cell with overlay in high-bridge band `[0xCD..0xE8]`.

```c
iVar2 = *(int *)(cell + 0x44);   // read overlay

// Determine NS vs EW sub-range
// NS range: [0xCD..0xD5] | [0xDF..0xE2] | == 0xE7
// EW range: [0xD6..0xDE] | [0xE3..0xE6] | == 0xE8

if (NOT in NS range) {
    if (in EW range) {
        // Walk back along X axis (west-1) to find canonical west-edge anchor
        param_1 = (x-1, y);
        if (cell_at(param_1).overlay NOT in [0xCD..0xE8]) {
            // anchor is already at west edge → call NS-axis walker from (x+1, y)
            RepairBridgeWalker_EW_High((x+1, y));  // NOTE: EW walker walks NS axis
            return;
        }
        param_1 = (x-2, y);
        if (cell_at(param_1).overlay NOT in [0xCD..0xE8]) {
            RepairBridgeWalker_EW_High(psVar1);     // walker from original coord
            return;
        }
        // Both x-1 and x-2 are in band → use FUN_00588c60 fallback
        RepairBridgeWalker_EW_High(&local_8);
    }
    return;
}
// NS range: Walk back along Y axis (north-1) to find canonical north-edge anchor
param_1 = (x, y-1);
if (cell_at(param_1).overlay NOT in [0xCD..0xE8]) {
    // anchor is already at north edge → walker from (x, y+1)
    RepairBridgeWalker_NS_High((x, y+1));   // NOTE: NS walker walks EW axis
    return;
}
param_1 = (x, y-2);
if (cell_at(param_1).overlay IN [0xCD..0xE8]) {
    // both y-1 and y-2 are in band → anchor is y-1
    RepairBridgeWalker_NS_High((x, y-1));
    return;
}
// y-2 not in band → original coord is the north-edge anchor
RepairBridgeWalker_NS_High(psVar1);
```

---

## Overlay Sub-Range to Direction Mapping

From `decompile_function 0x0057F440`:

| Overlay range | Walk axis | Walker called |
|---|---|---|
| NS: `[0xCD..0xD5]` | Y axis | `MapClass::RepairBridgeWalker_NS_High` |
| NS: `[0xDF..0xE2]` | Y axis | `MapClass::RepairBridgeWalker_NS_High` |
| NS: `== 0xE7` | Y axis | `MapClass::RepairBridgeWalker_NS_High` |
| EW: `[0xD6..0xDE]` | X axis | `MapClass::RepairBridgeWalker_EW_High` |
| EW: `[0xE3..0xE6]` | X axis | `MapClass::RepairBridgeWalker_EW_High` |
| EW: `== 0xE8` | X axis | `MapClass::RepairBridgeWalker_EW_High` |

**Note on naming convention:** `RepairBridgeWalker_NS_High` is named for the bridge axis (NS =
North-South bridge), but it walks along the perpendicular (EW) direction — one cell triplet per
step across the bridge width. `RepairBridgeWalker_EW_High` similarly walks NS. This is the same
perpendicular-triplet convention as the low-bridge collapse walkers.

---

## High Bridge Overlay Band

Full band: `[0xCD..0xE8]` (decimal 205–232). This is the high bridge overlay range verified
across the entire bridge-mechanic codebase:
- `DestroyBridge_High_OnHutDeath @ 0x00574000` — `CMP EAX, 0xCD` / `CMP EAX, 0xE8` (from prior
  assembly context verification at `0x00574049`)
- `ProcessBridgeDestruction_High` (the caller) — same band check
- This function — `CMP iVar2 < 0xCD || 0xE8 < iVar2` for full-band test

---

## CellClass Fields Accessed

| Offset | Type | Use |
|---|---|---|
| `+0x44` | int overlay | Compared to high-bridge band `[0xCD..0xE8]` for direction detect |

No other CellClass fields accessed directly — the walker callees handle tile and flag fields.

---

## Globals Referenced

| Global | Role |
|---|---|
| `g_CellArray_Base @ 0x0087F924` | Flat cell pointer array base for coord-to-cell lookup |
| `DAT_00ABDC50` | Fallback/sentinel cell for out-of-bounds coords |
| `DAT_00ABDC74` | Fallback coord store for OOB cells |

---

## Out-of-scope Refs

- `MapClass::RepairBridgeWalker_NS_High` — callee, not yet decoded
- `MapClass::RepairBridgeWalker_EW_High` — callee, not yet decoded
- `ProcessBridgeDestruction_High @ 0x00573540` — caller, decode task #81
- `FUN_00588C60` — fallback anchor finder (parallel to the EW-2 case), not yet decoded

---

## Self-Proof

### Claim 1: Function at `0x0057F440` is `MapClass::RepairBridge_High`, body `0x0057F440–0x0057F69F`

`get_function_by_address 0x0057F440` →
```
Function: MapClass__RepairBridge_High at 0057f440
Body: 0057f440 - 0057f69f
```
**VERIFIED.**

### Claim 2: Single caller is `ProcessBridgeDestruction_High @ 0x00573540`

`get_function_callers 0x0057F440` →
```
ProcessBridgeDestruction_High @ 00573540
```
Exactly one caller. **VERIFIED.**

### Claim 3: High-bridge overlay band `[0xCD..0xE8]`; NS sub-range includes `[0xCD..0xD5]`, EW includes `[0xD6..0xDE]`

`decompile_function 0x0057F440` → outer condition:
```c
if ((((iVar2 < 0xcd) || (0xd5 < iVar2)) && ((iVar2 < 0xdf || (0xe2 < iVar2)))) && (iVar2 != 0xe7))
```
Negation of this is the NS range: `[0xCD..0xD5]` | `[0xDF..0xE2]` | `0xE7`.
Inner EW check:
```c
if (((0xd5 < iVar2) && (iVar2 < 0xdf)) || (((0xe2 < iVar2 && (iVar2 < 0xe7)) || (iVar2 == 0xe8))))
```
EW range: `[0xD6..0xDE]` | `[0xE3..0xE6]` | `0xE8`. Together cover `[0xCD..0xE8]`. **VERIFIED.**
