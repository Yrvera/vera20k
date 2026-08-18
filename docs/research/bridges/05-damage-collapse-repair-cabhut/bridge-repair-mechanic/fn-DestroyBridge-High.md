# MapClass::DestroyBridge_High — Decode Doc

**Function:** `DestroyBridge_High`
**Address:** `0x0057CCF0`
**Body range:** `0x0057CCF0 – 0x0057CF59`
**Calling convention:** `__cdecl` (`param_1` = `short*` coord pointer on stack)
**Scope:** Full function.

---

## Summary

`DestroyBridge_High` is the axis-dispatch function for per-cell high (concrete) bridge
destruction. Given a cell coord containing a high-bridge overlay, it:

1. Reads `CellClass+0x44` (overlay type index) of the input cell.
2. Classifies the overlay into NS or EW axis based on its value within the high-bridge range.
3. Dispatches to `DestroyBridgeWalker_NS_High` or `DestroyBridgeWalker_EW_High` from the
   appropriate starting cell (using neighbor checks to find the span start).
4. Returns 0 silently if the overlay does not fall in any recognized high-bridge band.

This is a compiled twin of `DestroyBridge_Low @ 0x0057BAA0`, with high-bridge overlay
bands `[0xCD..0xE8]` instead of low-bridge `[0x4A..0x65]`.

---

## Active in YR

**Yes.** Verified via `get_function_callers 0x0057CCF0`:
- `ApplyDamageToCell @ 0x00587180`
- `Apply_area_damage @ 0x00489280`
- `MapClass::CollapseBridge_EW_High @ 0x00575870`
- `MapClass::CollapseBridge_NS_High @ 0x00575BA0`

All four are live YR paths. Fires on every high-bridge tile destruction event.

---

## Overlay Band Classification

From `decompile_function 0x0057CCF0`:

| Overlay range | Dec | Axis | Band |
|---|---|---|---|
| `[0xCD, 0xD5]` | 205–213 | NS | NS body |
| `[0xDF, 0xE2]` | 223–226 | NS | NS body (secondary) |
| `0xE7` | 231 | NS | NS terminal |
| `[0xD6, 0xDE]` | 214–222 | EW | EW body |
| `[0xE3, 0xE6]` | 227–230 | EW | EW body (secondary) |
| `0xE8` | 232 | EW | EW terminal |
| All others | — | — | No dispatch (return 0) |

The general high-bridge band `(0xCC, 0xE9)` is used for bridge detection; the NS/EW sub-bands
here further classify the axis.

---

## NS Branch

```c
// Classified as NS: overlay ∈ [0xCD..0xD5] ∪ [0xDF..0xE2] ∪ {0xE7}
// Check (X, Y-1):
if (cell_at(X, Y-1)->overlay < 0xCD || > 0xE8) {
    // No high-bridge overlay one row north → start NS walker at (X, Y+1)
    DestroyBridgeWalker_NS_High(CONCAT22(Y+1, X));
    return;
}
// Check (X, Y-2):
if (cell_at(X, Y-2)->overlay > 0xCC && < 0xE9) {
    // Two rows of NS overlay above → start walker at (X, Y-1)
    DestroyBridgeWalker_NS_High(CONCAT22(Y-1, X));
    return;
}
// Fall through: start at (X, Y)
DestroyBridgeWalker_NS_High(psVar1);
```

---

## EW Branch

```c
// Classified as EW: overlay ∈ [0xD6..0xDE] ∪ [0xE3..0xE6] ∪ {0xE8}
// Check (X-1, Y):
if (cell_at(X-1, Y)->overlay < 0xCD || > 0xE8) {
    // No high-bridge overlay one column west → start EW walker at (X+1, Y)
    DestroyBridgeWalker_EW_High(CONCAT22(Y, X+1));
    return;
}
// Check (X-2, Y):
if (cell_at(X-2, Y)->overlay > 0xCC && < 0xE9) {
    // Fallback: FUN_00588C60 finds the start coord
    puVar5 = FUN_00588C60(local_4, &param_1=1);
    DestroyBridgeWalker_EW_High(puVar5);
    return;
}
// Fall through: start at (X, Y)
DestroyBridgeWalker_EW_High(psVar1);
```

---

## Callees

Verified via `get_function_callees 0x0057CCF0`:

| Callee | Address | Role |
|---|---|---|
| `MapClass::Get_CellClass` | `0x005657A0` | Cell pointer from coord |
| `MapClass::DestroyBridgeWalker_NS_High` | `0x0057CF60` | Walk NS high-bridge span and destroy tiles |
| `MapClass::DestroyBridgeWalker_EW_High` | `0x0057D530` | Walk EW high-bridge span and destroy tiles |
| `FUN_00588C60` | `0x00588C60` | Fallback coord finder for EW edge case (not decoded) |

---

## Globals Used

| Global | Role |
|---|---|
| `g_CellArray_Base` | Cell array pointer (index = Y*512 + X) |
| `DAT_00ABDC50` | Sentinel CellClass* for out-of-bounds |
| `DAT_00ABDC74` | Out-of-bounds coord scratch |

---

## Relationship to DestroyBridge_Low

`DestroyBridge_High` (`0x0057CCF0`) and `DestroyBridge_Low` (`0x0057BAA0`) are structurally
identical — same axis-detection logic, same neighbor checks, same walker dispatch pattern.
The only differences:
- Low uses bands `[0x4A..0x65]`; high uses `[0xCD..0xE8]`.
- Low dispatches to `Walker_NS_Low @ 0x0057BB00` / `Walker_EW_Low @ 0x0057C350`; high dispatches to `Walker_NS_High @ 0x0057CF60` / `Walker_EW_High @ 0x0057D530`.

---

## Unverified (YELLOW)

- `FUN_00588C60` identity — same function used in `DestroyBridge_Low` EW fallback. Not decoded.
- The precise `DestroyBridgeWalker_NS_High` and `DestroyBridgeWalker_EW_High` behavior
  (which overlays they write, how many cells they walk) is not traced here — those are
  separate decode tasks.

---

## Self-Proof (exit gate)

### Claim 1: Function is `DestroyBridge_High` at `0x0057CCF0`

`get_function_by_address 0x0057CCF0` → `DestroyBridge_High`, body `0x0057CCF0 – 0x0057CF59`.
**VERIFIED — matches task spec.**

### Claim 2: Four callers — `ApplyDamageToCell`, `Apply_area_damage`, `CollapseBridge_EW_High`, `CollapseBridge_NS_High`

`get_function_callers 0x0057CCF0` → exactly four callers as listed above.
**VERIFIED — all live YR paths.**

### Claim 3: Dispatches to `DestroyBridgeWalker_NS_High @ 0x0057CF60` and `DestroyBridgeWalker_EW_High @ 0x0057D530`

`get_function_callees 0x0057CCF0` → both walkers listed.
`decompile_function 0x0057CCF0` → NS walker called for overlay `∈ [0xCD..0xD5] ∪ [0xDF..0xE2] ∪ {0xE7}`;
EW walker called for overlay `∈ [0xD6..0xDE] ∪ [0xE3..0xE6] ∪ {0xE8}`. **VERIFIED.**
