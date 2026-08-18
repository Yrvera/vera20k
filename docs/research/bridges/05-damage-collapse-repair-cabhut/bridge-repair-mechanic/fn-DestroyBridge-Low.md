# fn-DestroyBridge-Low

**Runbook:** function-decode-v1
**Target:** `DestroyBridge_Low @ 0x0057BAA0`
**Confidence:** HIGH — function identity confirmed via `get_function_by_address 0x0057BAA0`; decompilation via `decompile_function 0x0057BAA0`; callers confirmed via `get_function_callers 0x0057BAA0`; callees via `get_function_callees 0x0057BAA0`.
**YR-active:** YES — called from `CollapseBridge_NS_Low`, `CollapseBridge_EW_Low`, `ApplyDamageToCell`, and `Apply_area_damage` in standard YR play.

---

## Function Identity

Verified via `get_function_by_address 0x0057BAA0`:
```
Function: DestroyBridge_Low at 0057baa0
Signature: undefined DestroyBridge_Low(void)
Entry: 0057baa0
Body: 0057baa0 - 0057bce5
```
Body size: `0x0057BCE5 - 0x0057BAA0 = 0x245` bytes (581 bytes). Medium-sized function.

Ghidra plate comment (pre-existing annotation, summarized): "MapClass::DestroyBridge_Low — per-cell destroyer (low variant). NOT the same as DestroyBridge_Low_MapInit @ 0x574C20."

The decompiler signature: `uint DestroyBridge_Low(short *param_1)` where `param_1` is a packed cell coordinate (two shorts: X = `*param_1`, Y = `param_1[1]`).

---

## Callers

Verified via `get_function_callers 0x0057BAA0`:
```
ApplyDamageToCell @ 00587180
Apply_area_damage @ 00489280
MapClass__CollapseBridge_EW_Low @ 00575220
MapClass__CollapseBridge_NS_Low @ 00575540
```

Four callers:
- `CollapseBridge_NS_Low` and `CollapseBridge_EW_Low` — the span collapse walkers (decoded in tasks #7, #8). These call `DestroyBridge_Low` per cell as they walk the span.
- `ApplyDamageToCell @ 0x00587180` — called when a cell takes combat damage; triggers bridge destruction if the cell contains a bridge tile.
- `Apply_area_damage @ 0x00489280` — area-effect damage path; same trigger.

---

## Callees

Verified via `get_function_callees 0x0057BAA0`:

| Callee | Address | Role |
|---|---|---|
| `MapClass__Get_CellClass` | `0x005657A0` | Cell lookup by packed coord |
| `MapClass__DestroyBridgeWalker_NS_Low` | `0x0057BCF0` | NS-axis walk: destroys contiguous low-bridge tiles northward/southward |
| `MapClass__DestroyBridgeWalker_EW_Low` | `0x0057C2B0` | EW-axis walk: destroys contiguous low-bridge tiles eastward/westward |
| `FUN_00588c60` | `0x00588C60` | Coord adjustment helper (one call site, EW path) |

---

## Algorithm

The function is a **per-cell dispatch** that identifies the bridge axis from the hit cell's overlay index, finds the correct walker anchor, and calls the appropriate walker. It does **not** walk the span itself — that is delegated entirely to the two walker functions.

### Step 1: Cell lookup

```c
iVar2 = param_1[1] * 0x200 + (int)*param_1;  // cell index = Y * 512 + X
puVar3 = *(undefined **)(g_CellArray_Base + iVar2 * 4);
uVar4 = *(uint *)(puVar3 + 0x44);             // read overlay tile index
```

`CellClass + 0x44` = overlay tile index field (confirmed by prior decode sessions).

### Step 2: Overlay classification

Three overlay bands are checked:

| Band | Hex range | Decimal | Classification |
|---|---|---|---|
| NS primary | `0x4A..0x52` | 74–82 | Low bridge NS-axis tiles |
| NS secondary | `0x5C..0x5F` | 92–95 | Low bridge NS-axis extra tiles |
| NS singleton | `0x64` | 100 | Low bridge NS endpoint tile |
| EW primary | `0x53..0x5B` | 83–91 | Low bridge EW-axis tiles |
| EW secondary | `0x60..0x63` | 96–99 | Low bridge EW-axis extra tiles |
| EW singleton | `0x65` | 101 | Low bridge EW endpoint tile |

The full low-bridge overlay range is `[0x4A..0x65]` (74–101), split into NS and EW halves.

If `overlay ∉ [0x4A..0x65]`, the function returns `uVar4 & 0xFFFFFF00` (= 0, low byte zeroed) — the "not a bridge cell" signal to the caller.

### Step 3: NS path — anchor detection

For NS-axis cells:

```c
// Check cell at Y-1 (one north)
neighbor_Y_minus_1 = Get_CellClass({X, Y-1});
if (neighbor_Y_minus_1.overlay < 0x4A || neighbor_Y_minus_1.overlay > 0x65) {
    // Not a bridge cell north — start walker at Y+1 (one south)
    DestroyBridgeWalker_NS_Low({X, Y+1});
} else {
    // Check cell at Y-2 (two north)
    neighbor_Y_minus_2 = Get_CellClass({X, Y-2});
    if (neighbor_Y_minus_2.overlay in [0x4A..0x65]) {
        // Bridge extends at Y-2 too — start walker at Y-1
        DestroyBridgeWalker_NS_Low({X, Y-1});
    } else {
        // Bridge starts at Y — start walker at Y itself
        DestroyBridgeWalker_NS_Low({X, Y});
    }
}
```

The anchor logic finds the **northernmost** bridge cell in the span and starts the walker there, so it walks the full span from north to south regardless of which cell was hit.

### Step 4: EW path — anchor detection

Mirror of NS path, operating on X axis:

```c
// Check cell at X-1 (one west)
neighbor_X_minus_1 = Get_CellClass({X-1, Y});
if (neighbor_X_minus_1.overlay < 0x4A || neighbor_X_minus_1.overlay > 0x65) {
    // Not a bridge cell west — start walker at X+1 (one east)
    DestroyBridgeWalker_EW_Low({X+1, Y});
} else {
    // Check cell at X-2 (two west)
    neighbor_X_minus_2 = Get_CellClass({X-2, Y});
    if (neighbor_X_minus_2.overlay in [0x4A..0x65]) {
        // Bridge extends at X-2 — start walker at X-1 (via FUN_00588c60 coord helper)
        DestroyBridgeWalker_EW_Low(adjusted_coord);
    } else {
        // Bridge starts at X — start walker at X itself
        DestroyBridgeWalker_EW_Low({X, Y});
    }
}
```

One EW site calls `FUN_00588C60` to produce the adjusted coordinate before passing to `DestroyBridgeWalker_EW_Low`. This helper takes `param_1 = (short*)0x1` and `local_4` (local coord buffer) — it likely extracts the adjusted X−1 coord from the local buffer.

### Step 5: Return value

Returns a `uint`. The return value from the walker calls is passed through. The "not a bridge cell" case returns `uVar4 & 0xFFFFFF00` = 0 (AL = 0). Callers use this to decide retry behavior (per the Ghidra plate comment: "returns 0 if not in bridge band, signals CollapseBridge to retry up to 3 times").

---

## Overlay Range Summary

| Range | Band | Axis | Notes |
|---|---|---|---|
| `[0x4A..0x52]` | NS primary | NS | 9 tiles |
| `[0x5C..0x5F]` | NS secondary | NS | 4 tiles |
| `0x64` | NS singleton | NS | 1 tile (endpoint) |
| `[0x53..0x5B]` | EW primary | EW | 9 tiles |
| `[0x60..0x63]` | EW secondary | EW | 4 tiles |
| `0x65` | EW singleton | EW | 1 tile (endpoint) |
| **Total** | | | `[0x4A..0x65]` = 28 tiles |

These are the **low bridge** overlay indices. The high-bridge equivalent (`DestroyBridge_High`) uses `[0xCD..0xE8]`.

---

## Relationship to DestroyBridge_High

`DestroyBridge_High @ 0x0057CCF0` is the structural twin for high bridges. The algorithm is identical — same anchor detection logic, same walker dispatch pattern, but:
- Overlay range: `[0xCD..0xE8]` instead of `[0x4A..0x65]`
- Walkers: `DestroyBridgeWalker_NS_High` and `DestroyBridgeWalker_EW_High`

---

## Key Constants

| Constant | Value | Meaning |
|---|---|---|
| Low bridge overlay base | `0x4A` (74) | First low bridge tile index |
| Low bridge overlay top | `0x65` (101) | Last low bridge tile index |
| NS primary top | `0x52` (82) | Last NS-primary tile |
| EW primary base | `0x53` (83) | First EW-primary tile |
| EW primary top | `0x5B` (91) | Last EW-primary tile |
| NS secondary base | `0x5C` (92) | First NS-secondary tile |
| NS secondary top | `0x5F` (95) | Last NS-secondary tile |
| EW secondary base | `0x60` (96) | First EW-secondary tile |
| EW secondary top | `0x63` (99) | Last EW-secondary tile |
| NS endpoint | `0x64` (100) | NS endpoint singleton |
| EW endpoint | `0x65` (101) | EW endpoint singleton |

---

## Out-of-Scope References

- `MapClass__DestroyBridgeWalker_NS_Low @ 0x0057BCF0` — the NS span walker; not decoded in this session.
- `MapClass__DestroyBridgeWalker_EW_Low @ 0x0057C2B0` — the EW span walker; not decoded in this session.
- `FUN_00588C60 @ 0x00588C60` — coord helper at the EW X−1 anchor site; purpose narrowly inferred, not decoded.
- `ApplyDamageToCell @ 0x00587180` and `Apply_area_damage @ 0x00489280` — callers; not decoded here.
- `CellClass + 0x44` overlay field — confirmed in multiple prior decode sessions (tasks #7–#10, #21).

---

## Summary

| Field | Value |
|---|---|
| Address | `0x0057BAA0` |
| Body | `0x0057BAA0 – 0x0057BCE5` |
| Callers | `CollapseBridge_NS_Low`, `CollapseBridge_EW_Low`, `ApplyDamageToCell`, `Apply_area_damage` |
| Purpose | Classify hit cell as NS or EW low bridge, find span anchor, dispatch correct walker |
| NS overlay range | `[0x4A..0x52] ∪ [0x5C..0x5F] ∪ {0x64}` |
| EW overlay range | `[0x53..0x5B] ∪ [0x60..0x63] ∪ {0x65}` |
| Not-bridge signal | Returns 0 if `overlay ∉ [0x4A..0x65]` |

---

## Self-Proof (exit gate)

### Claim 1: Function at `0x0057BAA0` is `DestroyBridge_Low`, body ends at `0x0057BCE5`

`get_function_by_address 0x0057BAA0` → `Function: DestroyBridge_Low at 0057baa0`, body `0057baa0 - 0057bce5`. **VERIFIED.**

### Claim 2: Callers include both collapse walkers and both damage paths

`get_function_callers 0x0057BAA0` → `ApplyDamageToCell @ 00587180`, `Apply_area_damage @ 00489280`, `MapClass__CollapseBridge_EW_Low @ 00575220`, `MapClass__CollapseBridge_NS_Low @ 00575540`. **VERIFIED.**

### Claim 3: Callees include `MapClass__DestroyBridgeWalker_NS_Low @ 0x0057BCF0` and `MapClass__DestroyBridgeWalker_EW_Low @ 0x0057C2B0`

`get_function_callees 0x0057BAA0` → `MapClass__DestroyBridgeWalker_NS_Low @ 0057bcf0` and `MapClass__DestroyBridgeWalker_EW_Low @ 0057c2b0`. **VERIFIED.**

---

## Unverified (YELLOW)

- Exact interpretation of `FUN_00588C60` — inferred as coord adjustment helper from call context (one site in EW path with `param_1 = (short*)0x1`). Not decompiled in this session.
- The return value semantics of the two walker functions are not decoded here — only that `DestroyBridge_Low` passes through whatever the walker returns (or 0 for non-bridge cells).
- `Apply_area_damage @ 0x00489280` caller context not checked — bridge destruction from area damage is confirmed reachable in YR from this caller list.
