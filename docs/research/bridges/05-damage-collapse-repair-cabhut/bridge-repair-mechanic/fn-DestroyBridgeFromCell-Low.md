# MapClass::DestroyBridgeFromCell_Low — Decode

**Function:** `MapClass::DestroyBridgeFromCell_Low`
**Address:** `0x00574780`
**Body range:** `0x00574780 – 0x005749BB`
**Input:** `short *param_1` — a cell coordinate pair `{X: short, Y: short}` for any cell in the
  target low bridge (the anchor from which the walk begins).
**Output doc:** `ra2-rust-game-docs/bridge-repair-mechanic/fn-DestroyBridgeFromCell-Low.md`

Verified via:
- `decompile_function 0x00574780` — full body returned, includes an inline Ghidra comment
- `get_function_by_address 0x00574780` — name and body range confirmed
- `get_function_by_address 0x00575220` — confirms `MapClass__CollapseBridge_EW_Low` (NOT NS)
- `get_function_by_address 0x00575540` — confirms `MapClass__CollapseBridge_NS_Low` (NOT EW)
- `read_memory 0x00574928`, `0x00574970`, `0x005749A8` — NS_Low CALL targets verified = `0x00575540`
- `read_memory 0x00574830` — EW_Low CALL target at `0x574832` = `E8 E9 09 00 00` → `0x00575220`

**Active in YR:** YES — called from `InfantryClass::PerCellProcess` mission==0x11 bridge-hut path
(low-bridge sub-case) and from `DestroyBridge_Low_OnHutDeath`. Fires every time a C4 infantry
(SEAL/Tanya/PsiTrooper) destroys a low bridge repair hut, or a low bridge takes lethal damage.

---

## CRITICAL: Task-description address swap corrected

The task description states:
> "Dispatches to CollapseBridge_NS_Low (0x00575220) or CollapseBridge_EW_Low (0x00575540)"

**This is WRONG.** Direct Ghidra verification shows the addresses are swapped:

| Function | Actual address | Evidence |
|----------|---------------|----------|
| `MapClass__CollapseBridge_EW_Low` | **`0x00575220`** | `get_function_by_address 0x00575220` |
| `MapClass__CollapseBridge_NS_Low` | **`0x00575540`** | `get_function_by_address 0x00575540` |

The CALL targets in the function body also confirm this:
- At `0x574832`: `E8 E9 09 00 00` → `0x574837 + 0x0009E9 = 0x575220` = `CollapseBridge_EW_Low`
  (called for NS-axis bridges — see §3 for the axis-naming convention)
- At `0x57492D/0x574976/0x5749AF`: all call `0x575540` = `CollapseBridge_NS_Low`
  (called for EW-axis bridges)

The naming convention is explained in §3.

---

## 1. Purpose

`DestroyBridgeFromCell_Low` is the **canonical-start selection and walker dispatcher** for
low-bridge destruction. It answers the question: "Given an arbitrary low-bridge cell, which
axis is this bridge on, and which end of the bridge should be the canonical start for the
collapse walker?"

It is the destruction-side twin of `RepairBridge_Low` at `0x57F200` — same structure, same
axis detection, but calls collapse walkers instead of repair walkers.

---

## 2. Input: cell coordinate encoding

`param_1` is `short *` pointing to a 2-short struct: `{X: param_1[0], Y: param_1[1]}`.

The cell index is computed as:
```c
iVar2 = param_1[1] * 0x200 + (int)*param_1;
```
This is the standard gamemd flat-array cell index: `Y * 512 + X`. Valid range: `[0, 0x3FFFF]` (512×512 map).

If the computed index is out of range or the `g_CellArray_Base[index]` pointer is null, the
function falls back to a static scratch struct `DAT_00abdc50` with coordinates saved to
`DAT_00abdc74`. This is a safety guard for invalid cells — does not affect normal operation.

Verified from decompile: `iVar2 = param_1[1] * 0x200 + (int)*param_1` and
`puVar3 = *(undefined **)(g_CellArray_Base + iVar2 * 4)`.

---

## 3. Overlay range classification and the axis-naming convention

The overlay type index is read from `CellClass + 0x44`:
```c
iVar2 = *(int *)(puVar3 + 0x44);
```

Low-bridge overlays occupy indices `0x4A–0x65` (decimal 74–101). They split into two groups
that correspond to bridge **axis** — but the function's naming convention is inverted relative
to what a reader might expect:

| Overlay range | Ghidra name | What axis the bridge actually runs on |
|---------------|-------------|---------------------------------------|
| `[0x4A..0x52]` ∪ `[0x5C..0x5F]` ∪ `{0x64}` | "NS range" in code logic | **EW-running bridge** (bridge body runs east-west; NS-named function walks N–S perpendicular to it) |
| `[0x53..0x5B]` ∪ `[0x60..0x63]` ∪ `{0x65}` | "EW range" in code logic | **NS-running bridge** (bridge body runs north-south; EW-named function walks E–W perpendicular) |

The called walkers (`CollapseBridge_EW_Low` / `CollapseBridge_NS_Low`) are named for the
**direction they walk** across the bridge, not the direction the bridge runs. An EW-running
bridge has its walker walk NS (perpendicular) — hence the seemingly inverted naming.

The precise overlay sub-ranges from the decompile:

**NS-overlay group** (EW-axis bridge body):
```
if ((iVar2 < 0x4a) || (0x52 < iVar2)) — i.e., NOT in [0x4A..0x52]
AND if ((iVar2 < 0x5c) || (0x5f < iVar2)) — i.e., NOT in [0x5C..0x5F]
AND iVar2 != 100 (0x64)
```
Falls through to EW range check. Inverted: the NS group is `[0x4A..0x52] | [0x5C..0x5F] | {0x64}`.

**EW-overlay group** (NS-axis bridge body):
```
if (((0x52 < iVar2) && (iVar2 < 0x5c)) || (((0x5f < iVar2 && (iVar2 < 100)) || (iVar2 == 0x65))))
```
= `[0x53..0x5B] | [0x60..0x63] | {0x65}`.

If overlay is outside both groups (< 0x4A or > 0x65), the function returns immediately — not a
low-bridge cell.

Verified from decompile output: `if (((iVar2 < 0x4a) || (0x52 < iVar2)) && ((iVar2 < 0x5c || (0x5f < iVar2)))) && (iVar2 != 100))`.

---

## 4. NS-overlay path — canonical start selection (Y-axis walk)

For NS-overlay cells (bridge body runs EW), the function walks **north** (Y−1, Y−2) to find
the canonical start cell for `CollapseBridge_EW_Low`:

```c
// Step 1: try Y-1
param_1 = (short *)CONCAT22(param_1[1] + -1, *param_1);   // Y-1, same X
iVar2 = MapClass__Get_CellClass(&param_1);
if (overlay_at(iVar2) outside [0x4A..0x65]) {
    // Y-1 is not a bridge cell → original cell is already the south edge
    // canonical start = Y+1 (north edge)
    param_1 = (short *)CONCAT22(psVar1[1] + 1, *psVar1);
    MapClass__CollapseBridge_EW_Low(&param_1);
    return;
}

// Step 2: try Y-2
param_1 = (short *)CONCAT22(psVar1[1] + -2, *psVar1);     // Y-2, same X
iVar2 = MapClass__Get_CellClass(&param_1);
if (overlay_at(iVar2) outside [0x4A..0x65]) {
    // Y-2 is not a bridge cell → original cell is the middle → Y-1 is north edge
    MapClass__CollapseBridge_EW_Low(psVar1);               // psVar1 = original coord
    return;
}

// Fallback: neither Y-1 nor Y-2 are non-bridge → use FUN_00588c60 to compute
param_1 = (short *)0x1;
puVar4 = (undefined4 *)FUN_00588c60(local_4, &param_1);
local_8 = *puVar4;
MapClass__CollapseBridge_EW_Low(&local_8);
```

The check `(*(int *)(iVar2 + 0x44) < 0x4a) || (0x65 < *(int *)(iVar2 + 0x44))` = "overlay index
outside the full low-bridge range [0x4A..0x65]."

Key insight: the bridge is 3 cells wide in Y. The canonical start passed to
`CollapseBridge_EW_Low` is always the **north edge** (highest Y value = southernmost row in
screen coordinates). The walk steps backward (south) through the 3 rows.

---

## 5. EW-overlay path — canonical start selection (X-axis walk)

For EW-overlay cells (bridge body runs NS), the function walks **west** (X−1, X−2) to find
the canonical start for `CollapseBridge_NS_Low`:

```c
// Step 1: try X-1
param_1 = (short *)CONCAT22(param_1[1], *param_1 + -1);   // same Y, X-1
iVar2 = MapClass__Get_CellClass(&param_1);
if (overlay_at(iVar2) outside [0x4A..0x65]) {
    // X-1 is not a bridge cell → pass X+1 (east edge)
    param_1 = (short *)CONCAT22(psVar1[1], *psVar1 + 1);
    MapClass__CollapseBridge_NS_Low(&param_1);
    return;
}

// Step 2: try X-2
param_1 = (short *)CONCAT22(psVar1[1], *psVar1 + -2);     // same Y, X-2
iVar2 = MapClass__Get_CellClass(&param_1);
if (overlay_at(iVar2) outside [0x4A..0x65]) {
    // X-2 is not a bridge cell → pass original coord
    MapClass__CollapseBridge_NS_Low(psVar1);
    return;
}

// Fallback: FUN_00588c60
param_1 = (short *)0x1;
puVar4 = (undefined4 *)FUN_00588c60(local_4, &param_1);
local_8 = *puVar4;
MapClass__CollapseBridge_NS_Low(&local_8);
```

Same logic as NS-path but walking X instead of Y, and dispatching to `CollapseBridge_NS_Low`.

---

## 6. FUN_00588c60 — fallback coord helper

`FUN_00588c60 @ 0x00588c60` (body `0x00588c60 – 0x00588c88`). Called in both paths as a last-resort
fallback when neither of the two back-step cells are outside the bridge range. Purpose: unknown
without decompiling — likely computes the canonical start via a different strategy (possibly
just returns the original coord). Identity: MEDIUM. Flagged as out-of-scope-ref.

Verified by `get_function_by_address 0x00588c60` — unnamed (`FUN_00588c60`).

---

## 7. CollapseWalker dispatch summary

| Overlay group | Bridge axis | Walker called | Walker address |
|---------------|-------------|---------------|----------------|
| NS group: `[0x4A..0x52] ∪ [0x5C..0x5F] ∪ {0x64}` | EW-running | `CollapseBridge_EW_Low` | `0x00575220` |
| EW group: `[0x53..0x5B] ∪ [0x60..0x63] ∪ {0x65}` | NS-running | `CollapseBridge_NS_Low` | `0x00575540` |

Walker addresses verified:
- `get_function_by_address 0x00575220` = `MapClass__CollapseBridge_EW_Low` ✓
- `get_function_by_address 0x00575540` = `MapClass__CollapseBridge_NS_Low` ✓
- `read_memory 0x00574832` (CALL at `0x574832`): `E8 E9 09 00 00` → target `0x575220` (EW_Low) ✓
- `read_memory 0x00574928` (CALL near `0x57492D`): `E8 0E 0C 00 00` → target `0x575540` (NS_Low) ✓
- `read_memory 0x00574970` (CALL at `0x574976`): `E8 C5 0B 00 00` → target `0x575540` (NS_Low) ✓
- `read_memory 0x005749A8` (CALL at `0x5749AF`): `E8 8C 0B 00 00` → target `0x575540` (NS_Low) ✓

---

## 8. Out-of-scope refs

| Symbol | Address | Role |
|--------|---------|------|
| `MapClass__CollapseBridge_EW_Low` | `0x00575220` | NS-overlay walker — separate decode task #8 |
| `MapClass__CollapseBridge_NS_Low` | `0x00575540` | EW-overlay walker — separate decode task #7 |
| `FUN_00588c60` | `0x00588c60` | Fallback coord helper — unknown identity |
| `MapClass__Get_CellClass` | (named) | Cell accessor — used throughout |
| `g_CellArray_Base` | DAT (global) | Cell array base pointer |
| `DAT_00abdc50`, `DAT_00abdc74` | globals | Out-of-range cell fallback storage |

---

## 9. Self-proof (exit gate step 4)

### Claim 1: Function address and name
`get_function_by_address 0x00574780` → `MapClass__DestroyBridgeFromCell_Low`, body `0x00574780 – 0x005749BB`. **MATCHES task spec address.**

### Claim 2: CollapseBridge_NS_Low is at `0x00575540`, NOT `0x00575220`
`get_function_by_address 0x00575540` → `MapClass__CollapseBridge_NS_Low`. **MATCHES Ghidra label.**
`get_function_by_address 0x00575220` → `MapClass__CollapseBridge_EW_Low`. **Task description had these swapped — corrected in §0.**

### Claim 3: CALL at `0x574928` area targets `0x575540` (NS_Low)
`read_memory 0x00574928` (16 bytes) = `50 89 54 24 18 E8 0E 0C 00 00 5F 5E 83 C4 08 C2`.
`E8` at offset 5 = address `0x57492D`. Target = `0x57492D + 5 + 0x00000C0E = 0x575540`. **MATCHES.**

---

## 10. Active-in-YR classification

| Finding | Active in YR? |
|---------|---------------|
| Full function reachable via C4-plant on bridge hut | **YES** |
| Full function reachable via direct bridge damage | **YES** |
| NS-overlay path dispatching to CollapseBridge_EW_Low | **YES** |
| EW-overlay path dispatching to CollapseBridge_NS_Low | **YES** |
| FUN_00588c60 fallback | **YES** (reachable but unusual — only fires when 3-cell walk fails to find an edge) |
| Out-of-range cell safety guard | **YES** (defensive; fires on bad coords) |

---

## Sources

**Ghidra MCP calls:**
- `decompile_function 0x00574780`
- `get_function_by_address 0x00574780`
- `get_function_by_address 0x00575220` → `CollapseBridge_EW_Low`
- `get_function_by_address 0x00575540` → `CollapseBridge_NS_Low`
- `get_function_by_address 0x00588c60` → `FUN_00588c60`
- `read_memory 0x00574928` (16 bytes) — NS_Low CALL at `0x57492D`
- `read_memory 0x00574970` (16 bytes) — NS_Low CALL at `0x574976`
- `read_memory 0x005749A8` (16 bytes) — NS_Low CALL at `0x5749AF`
- `read_memory 0x00574830` (80 bytes) — EW_Low CALL at `0x574832`
- `read_memory 0x00574808` (16 bytes) — overlay range guard bytes verified

**Prior docs cross-referenced:**
- `BRIDGEHEAD_DIRECT_DAMAGE_SLOT3_COLLAPSE_GHIDRA_REPORT.md`
- `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`
- `fn-InfantryClass-PerCellProcess-C4Plant.md` (caller context)
