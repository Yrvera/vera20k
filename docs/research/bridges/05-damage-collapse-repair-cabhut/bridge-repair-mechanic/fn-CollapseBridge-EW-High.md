# fn-CollapseBridge_EW_High (and NS_High twin)

**LABEL NOTE:** The task list has NS/EW inverted for all four collapse walkers. The address assigned to task #10 (`0x00575BA0`) is `MapClass__CollapseBridge_NS_High` in Ghidra. The actual EW_High is at `0x00575870` (which is task #9's assigned address). Both are structurally identical with a single axis swap. This doc covers `0x00575BA0` = `CollapseBridge_NS_High` (the function at the assigned address) and references `0x00575870` = `CollapseBridge_EW_High`.

Confirmed via `get_function_by_address 0x00575BA0` and `get_function_by_address 0x00575870`.

---

## Primary Target: MapClass__CollapseBridge_NS_High

**Address:** `0x00575BA0`
**Function body:** `0x00575BA0 – 0x00575ED0`
**Confidence:** HIGH (content, identity, callers verified via Ghidra MCP)
**YR-active:** YES — reachable from `DestroyBridgeFromCell_High @ 0x005749C0`.

### Signature

```c
void MapClass__CollapseBridge_NS_High(uint *param_1)
```

- `param_1` = packed cell coordinate (2 shorts): `*(short *)param_1` = X, `*((short *)param_1 + 1)` = Y.
- No `this` pointer — operates on global map.

Verified via `decompile_function 0x00575BA0`.

### Caller

Verified via `get_function_callers 0x00575BA0`:

| Caller | Address |
|---|---|
| `MapClass__DestroyBridgeFromCell_High` | `0x005749C0` |

### Callees

Verified via `get_function_callees 0x00575BA0`:

| Callee | Address | Role |
|---|---|---|
| `MapClass__Get_CellClass` | `0x005657A0` | Convert cell coord to CellClass pointer |
| `DestroyBridge_High` | `0x0057CCF0` | Destroy one high-bridge tile cell |
| `AnimClass__Constructor` | `0x00421EA0` | Spawn debris anim |
| `Random__RandomRanged` | `0x0065C7E0` | RNG (4× per debris anim) |
| `Math__ftol` | `0x007C5F00` | Float-to-int for position jitter |
| `operator_new` | `0x007C8E17` | Alloc AnimClass |
| `MapClass__UpdateBridgeZonesHelper` | `0x0056C510` | Post-collapse zone graph update |

---

## Algorithm — NS_High (0x00575BA0)

This function is a **high-bridge variant** of `CollapseBridge_NS_Low @ 0x00575540`, with the overlay band and sentinel substituted:

| Aspect | NS_Low (0x00575540) | NS_High (0x00575BA0) |
|---|---|---|
| Overlay band | `[0x4A, 0x65]` | `[0xCD, 0xE8]` |
| Span-finder test | `0x4a <= ovl < 0x66` | `0xcd <= ovl < 0xe9` |
| Main loop test | `< 0x4a or > 0x65` = break | `< 0xcd or > 0xe8` = break |
| Terminal sentinel | `0x65` (NS low anchor) | `0xE8` (high anchor) |
| Bridge-tile callee | `DestroyBridge_Low @ 0x0057BAA0` | `DestroyBridge_High @ 0x0057CCF0` |

### Phase 1: Span finder (axis = Y)

```c
uVar9 = *param_1;          // packed coord
local_1c = (short)uVar9;   // X (fixed for NS walker)
local_14 = 1;              // step direction (+1 or -1)
iVar11 = 0;  // backward Y-- count
iVar10 = 0;  // forward Y++ count

// Walk Y-- while overlay in [0xCD..0xE8]
do {
    param_1 = CONCAT22(param_1.Y - 1, X);
    iVar11++;
    iVar4 = MapClass__Get_CellClass(&param_1);
    if (*(int *)(iVar4 + 0x44) < 0xcd) break;
} while (*(int *)(iVar4 + 0x44) < 0xe9);

// Walk Y++ while overlay in [0xCD..0xE8]
do {
    param_1 = CONCAT22(param_1.Y + 1, X);
    iVar10++;
    iVar4 = MapClass__Get_CellClass(&param_1);
    if (*(int *)(iVar4 + 0x44) < 0xcd) break;
} while (*(int *)(iVar4 + 0x44) < 0xe9);

if (iVar10 < iVar11) local_14 = -1;   // step toward shorter side

// Start Y: center of span
uVar9 = (uVar9 >> 16) - (iVar11 - iVar10) / 2;  // SIGNED division
```

`CellClass + 0x44` = overlay index (verified across multiple prior tasks in this session).

### Phase 2: Collapse loop (4 iterations)

```c
local_2c = 4;
param_1 = CONCAT22((short)uVar9, local_1c);   // (X, start_Y)
while (0 < local_2c) {
    // Bounds check + cell lookup
    if (*(int *)(puVar5 + 0x44) != 0xe8) {   // 0xE8 = NS/EW high anchor sentinel
        // Spawn 3 debris anims at (X-1, current_Y), (X, current_Y), (X+1, current_Y)
        // 4 RNG calls per anim:
        //   1. Random__RandomRanged(0, 0x7FFFFFFE) → X jitter (ftol)
        //   2. Random__RandomRanged(0, 0x7FFFFFFE) → Y jitter (ftol)
        //   3. Random__RandomRanged(1, 5) → frame delay
        //   4. Random__RandomRanged(0, RulesClass+0x168 - 1) → anim index
        // anim type = RulesClass+0x15C[index]
        // position = cell.+0x24 * 0x100 + 0x80
        // Z = cell.+0x11B * DAT_00abde88
        // Flags = 0x600
        AnimClass__Constructor(...);   // × 3
    }
    // Destroy bridge tile, up to 3 retries
    iVar10 = 0;
    do {
        cVar3 = DestroyBridge_High(&param_1);
        if (cVar3 != '\0') break;
        iVar10++;
    } while (iVar10 < 3);

    // Advance Y by local_14 (+/-1)
    uVar8 = param_1.Y + local_14;
    local_2c--;
    param_1 = CONCAT22(uVar8, X);

    // Break if new cell overlay outside [0xCD..0xE8]
    if ((*(int *)(puVar5 + 0x44) < 0xcd) || (0xe8 < *(int *)(puVar5 + 0x44))) break;
}
```

**Terminal sentinel `0xE8`:** Overlay index 232 = high-bridge destroyed anchor. When a cell carries this overlay, the debris-spawn phase is skipped for that step (cell is already "rubbled").

**RNG lockstep:** Up to `4 × 3 × 4 = 48` `Random__RandomRanged` calls per invocation. Same count and order as NS/EW_Low walkers. Must be reproduced exactly in the Rust port.

### Phase 3: Post-collapse tail (unconditional)

```c
MapClass__UpdateBridgeZonesHelper();       // 0x0056C510
*(undefined1 *)(g_Tactical + 0xd7c) = 1;  // tactical dirty
return;
```

Same tail as all other collapse walkers in this system. `UpdateBridgeZonesHelper` is unconditional.

---

## Structural Twin: MapClass__CollapseBridge_EW_High

**Address:** `0x00575870`
**Body:** `0x00575870 – 0x00575B9E`
**Caller:** `MapClass__DestroyBridgeFromCell_High @ 0x005749C0` (same caller)

Confirmed via `get_function_by_address 0x00575870`.

The EW_High variant has the same structure with axis swap:
- Where NS_High steps **Y** and spawns anims at **X-1, X, X+1**, EW_High steps **X** and spawns at **Y-1, Y, Y+1**.
- Same overlay band `[0xCD..0xE8]`.
- EW_High terminal sentinel: `0xE7` (EW high destroyed anchor = 231).

The plate comment at `0x00575BA0` itself says: "Compiled twin of 0x575540 with high overlay band [0xCD..0xE8]; destroyed-anchor sentinel = 0xE7" — note this comment is on the NS_High function but describes the EW_High sentinel as `0xE7`. Looking at the decompilation of `0x00575BA0`, the sentinel checked is `0xE8` (`if (*(int *)(puVar5 + 0x44) != 0xe8)`). So:
- NS_High sentinel: `0xE8` (verified from decompilation of `0x00575BA0`)
- EW_High sentinel: `0xE7` (from plate comment — **YELLOW**, not directly decompiled here)

---

## Overlay Family Summary — High Bridge

| Range | Type |
|---|---|
| `[0xCD, 0xE8]` | Live high-bridge overlay (both NS and EW) |
| `0xE7` | EW destroyed anchor (skip debris spawn) — YELLOW |
| `0xE8` | NS/general destroyed anchor (skip debris spawn) — verified |
| `< 0xCD` or `> 0xE8` | Exit span-finder / main loop |

---

## NS/EW Label Inversion in Task List

All four low and high collapse walkers have their NS/EW labels swapped in the task manifest:

| Task | Manifest label | Assigned address | Ghidra actual name |
|---|---|---|---|
| #7 | CollapseBridge_NS_Low | `0x00575220` | `MapClass__CollapseBridge_EW_Low` |
| #8 | CollapseBridge_EW_Low | `0x00575540` | `MapClass__CollapseBridge_NS_Low` |
| #9 | CollapseBridge_NS_High | `0x00575870` | `MapClass__CollapseBridge_EW_High` |
| #10 | CollapseBridge_EW_High | `0x00575BA0` | `MapClass__CollapseBridge_NS_High` |

The decode addresses are correct. The NS/EW labels in the manifest are swapped. This should be corrected in the synthesis doc.

---

## Unverified

**YELLOW:** EW_High terminal sentinel `0xE7` — stated in the plate comment at `0x00575BA0` but not verified by directly decompiling `0x00575870` in this session.

**YELLOW:** `DAT_00abde88` runtime value — cell height scaler for anim Z position. Not fetched.
