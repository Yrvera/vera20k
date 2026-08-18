# MapClass::CollapseBridge_NS_High — Decode

**Function:** `MapClass::CollapseBridge_NS_High`
**Correct address:** `0x00575BA0`
**Body range:** `0x00575BA0 – 0x00575E9F` (approx; Ghidra body end not confirmed separately)
**Input:** `uint *param_1` — a cell coordinate pair `{X: low 16 bits, Y: high 16 bits}`
  representing the canonical start anchor for a NS-axis high bridge.
**Output doc:** `ra2-rust-game-docs/bridge-repair-mechanic/fn-CollapseBridge-NS-High.md`

Verified via:
- `decompile_function 0x00575BA0` — full body returned with inline Ghidra comment
- `get_function_by_address 0x00575BA0` — name confirmed: `MapClass__CollapseBridge_NS_High`
- `search_functions "CollapseBridge_NS_High"` — returns `0x00575BA0` (not `0x00575870`)
- `search_functions "CollapseBridge_EW_High"` — returns `0x00575870` (task description had addresses swapped)
- `search_functions "DestroyBridge_High"` — returns `0x0057CCF0`
- `read_memory 0x00575BA0` (16 bytes) — function entry bytes confirmed

**Active in YR:** YES — called from `DestroyBridgeFromCell_High` whenever a high-bridge cell with
NS-axis overlay is detected. Fires on every high-bridge collapse event (C4 or damage).

---

## CRITICAL: Task-description address correction

The task description stated the target address as `0x00575870`. That address is **`CollapseBridge_EW_High`** (task #10), not `CollapseBridge_NS_High`.

| Function | Correct address | Evidence |
|----------|----------------|----------|
| `MapClass__CollapseBridge_NS_High` | **`0x00575BA0`** | `search_functions "CollapseBridge_NS_High"` + `decompile_function 0x00575BA0` |
| `MapClass__CollapseBridge_EW_High` | **`0x00575870`** | `search_functions "CollapseBridge_EW_High"` + `get_function_by_address 0x00575870` |

---

## 1. Purpose and relationship to low-bridge variant

`CollapseBridge_NS_High` is the **high-bridge variant** of `CollapseBridge_NS_Low` (`0x00575540`).
Per the inline Ghidra comment:

> "Compiled twin of `0x575540` with high overlay band `[0xCD..0xE8]`; destroyed-anchor sentinel = `0xE7`.
>  Same algorithm, constants substituted."

The algorithm is identical in structure to the low-bridge walker; only the overlay band constants differ:

| Constant | Low bridge | High bridge |
|----------|-----------|-------------|
| Overlay band lower bound | `0x4A` | `0xCD` |
| Overlay band upper bound (exclusive) | `0x66` | `0xE9` |
| Outer loop count | (from NS_Low — 3 per existing docs) | **4** |
| Destroyed-anchor sentinel | `0x64` (NS_Low) | `0xE7` (NS_High) |
| Non-sentinel "still bridge" upper | `0x65` (NS_Low) | `0xE8` (NS_High) |

---

## 2. Coordinate encoding

`param_1` is `uint *` pointing to a packed 32-bit cell coord: `X = low 16 bits`, `Y = high 16 bits`.

```c
uVar9 = *param_1;
local_1c = (short)uVar9;          // X = low 16 bits
// Y = uVar9 >> 16 (used implicitly via param_1._2_2_)
```

Cell array index: `Y * 0x200 + X`. Valid range: `[0, 0x3FFFF]`. Out-of-range falls back to
`DAT_00abdc50` scratch cell (safety guard, same pattern as `DestroyBridgeFromCell_Low`).

Verified from decompile: `iVar10 = (short)uVar9 * 0x200 + (int)local_1c` — note `(short)uVar9`
extracts Y (the high half after the coord swap), and `local_1c` is X.

---

## 3. Phase 1 — canonical-start adjustment (Y-axis extent scan)

Before iterating over the bridge columns, the function finds the **true canonical start** along
the Y axis by scanning how far north and south the bridge extends from the given anchor:

```c
// Walk north (Y−1) until outside high-bridge band [0xCD..0xE8]
do {
    param_1 = Y - 1, X;   // step north
    iVar11++;
    iVar4 = MapClass__Get_CellClass(&param_1);
} while (0xCD <= overlay && overlay < 0xE9);

// Walk south (Y+1) until outside band
do {
    param_1 = Y + 1, X;   // step south
    iVar10++;
    iVar4 = MapClass__Get_CellClass(&param_1);
} while (0xCD <= overlay && overlay < 0xE9);

// Direction: if south-count < north-count, step is -1 (start from south)
if (iVar10 < iVar11) { local_14 = -1; }

// Canonical start Y = input_Y - (iVar11 - iVar10) / 2
uVar9 = (uVar9 >> 0x10) - (iVar11 - iVar10) / 2;
param_1 = CONCAT22((short)uVar9, local_1c);
```

This centers the walker on the bridge's Y extent, then sets `local_14` as the step direction
(`+1` or `−1`) for the main loop. The canonical start Y is the **northernmost row** when
`local_14 == 1` (stepping south), or the **southernmost row** when `local_14 == −1` (stepping north).

Verified from decompile: `uVar9 = (uVar9 >> 0x10) - (iVar11 - iVar10) / 2`.

---

## 4. Phase 2 — main collapse loop (4 axial iterations)

```c
local_2c = 4;   // 4 Y-steps across the bridge
while (0 < local_2c) {
    // Bounds check + cell fetch
    iVar10 = (short)uVar9 * 0x200 + (int)local_1c;
    // ... fallback to DAT_00abdc50 if out of range

    // Sentinel check: if overlay == 0xE8, skip debris spawn (already destroyed cap cell)
    if (*(int *)(puVar5 + 0x44) != 0xe8) {
        // Spawn 3 debris anim instances at this Y row, X-1 to X+1 (3 columns)
        local_24 = CONCAT22((short)uVar9, (short)param_1 - 1);  // X-1
        iVar10 = 3;
        do {
            // Read cell center coords (CellClass+0x24 = location leptons)
            sStack_1e = (short)((uint)*(undefined4 *)(puVar5 + 0x24) >> 0x10);   // Y leptons
            local_c   = (short)*(undefined4 *)(puVar5 + 0x24) * 0x100 + 0x80;    // X leptons + half-cell
            local_8   = sStack_1e * 0x100 + 0x80;
            local_4   = (char)puVar5[0x11b] * DAT_00abde88;                       // Z height

            // Randomize coords for anim scatter
            Random__RandomRanged(0, 0x7ffffffe);
            local_c = Math__ftol();
            Random__RandomRanged(0, 0x7ffffffe);
            local_8 = Math__ftol();

            // Allocate + construct debris AnimClass
            pvVar6 = operator_new(0x1c8);   // sizeof(AnimClass) = 0x1C8
            if (pvVar6 != NULL) {
                uVar7  = Random__RandomRanged(1, 5);         // random delay
                iVar11 = Random__RandomRanged(0, Rules.BridgeExplosionCount - 1);
                AnimClass__Constructor(
                    Rules.BridgeExplosionAnims[iVar11],
                    &local_c, uVar7, 1, 0x600, 0, 0);
            }
            local_24 = CONCAT22(sVar1, (short)local_24 + 1);  // step X+1
            iVar10--;
        } while (iVar10 != 0);
    }

    // Collapse this column: retry up to 3 times
    iVar10 = 0;
    do {
        cVar3 = DestroyBridge_High(&param_1);
        if (cVar3 != '\0') break;
        iVar10++;
    } while (iVar10 < 3);

    // Advance Y by step direction
    uVar8      = param_1._2_2_ + local_14;
    uVar9      = (uint)uVar8;
    local_2c--;
    param_1    = CONCAT22(uVar8, (short)param_1);

    // Early exit if next cell leaves the high-bridge band [0xCD..0xE8]
    if (overlay < 0xCD || 0xE8 < overlay) break;
}
```

**Key constants verified from decompile:**
- `local_2c = 4` — outer loop runs at most 4 times (4 Y-rows stepped across the bridge)
- Sentinel `0xE8` — skip debris spawn when overlay == `0xE8` (the "destroyed anchor cap" tile)
- Debris anim array: `g_RulesClass_Instance + 0x15c` = pointer to bridge explosion anim array;
  `g_RulesClass_Instance + 0x168` = count of explosion anims. Per prior docs, these are
  `BridgeExplosionAnim` entries from `rules(md).ini`.
- Inner loop: `DestroyBridge_High` called up to 3 times per column (retries on failure)

**Debris per row:** 3 anim instances per Y-row (X-1, X, X+1 columns), except when overlay == `0xE8`.

---

## 5. DestroyBridge_High callee

```c
cVar3 = DestroyBridge_High(&param_1);
```

`DestroyBridge_High` at `0x0057CCF0`. Called up to 3 times per axial step (retry loop while
`cVar3 == '\0'` = failure, max 3 attempts). The return value distinguishes success from no-op.

Verified: `search_functions "DestroyBridge_High"` → `0x0057CCF0` (separate from
`MapClass__DestroyBridge_High_OnHutDeath @ 0x00574000`).

---

## 6. Post-loop cleanup

```c
MapClass__UpdateBridgeZonesHelper();
*(undefined1 *)(g_Tactical + 0xd7c) = 1;
```

Same epilogue as `CollapseBridge_EW_High` (and the low-bridge variants):
1. `UpdateBridgeZonesHelper` — recalculates pathfinding zones for cells affected by the collapse.
2. `g_Tactical + 0xD7C = 1` — sets a dirty flag on the tactical layer to trigger redraw.

Verified from decompile: both calls present in both NS_High and EW_High functions.

---

## 7. High vs. low bridge sentinel difference

The task description mentions "Terminal cap `0xE7`/`0xE8`." The decompile clarifies:

- `0xE7` = check inside the main loop: `if (*(int *)(puVar5 + 0x44) != 0xe8)` — i.e., skip debris
  if overlay **is** `0xE8`. The sentinel `0xE7` from the task spec is the low-bridge analogue
  (for `CollapseBridge_NS_Low`, the skip sentinel is `0x64`).
- `0xE8` = the early-exit condition `0xe8 < *(int *)(puVar5 + 0x44)` — leave the loop when
  the next cell's overlay exceeds `0xE8` (out of band).

So for NS_High:
- Debris-spawn skip: overlay == `0xE8` (the "destroyed anchor" cap tile for high bridge)
- Band exit: overlay < `0xCD` OR overlay > `0xE8`

For NS_Low (per existing docs):
- Debris-spawn skip: overlay == `0x64` (low bridge cap)
- Band exit: overlay < `0x4A` OR overlay > `0x65`

---

## 8. CellClass fields accessed

| CellClass offset | Usage |
|-----------------|-------|
| `+0x44` | Overlay type index — the low-bridge and high-bridge discriminant |
| `+0x24` | Location leptons (packed: low 16 = X leptons, high 16 = Y leptons) — used for debris anim position |
| `+0x11B` | Height/Z field (byte) — scaled by `DAT_00abde88` for anim Z coord |

Verified from decompile: `*(int *)(puVar5 + 0x44)`, `*(undefined4 *)(puVar5 + 0x24)`,
`(char)puVar5[0x11b]`.

---

## 9. Out-of-scope refs

| Symbol | Address | Role |
|--------|---------|------|
| `DestroyBridge_High` | `0x0057CCF0` | Per-cell collapse — separate decode task #93 |
| `MapClass__UpdateBridgeZonesHelper` | (named) | Zone recalc — separate decode task #11 |
| `g_RulesClass_Instance + 0x15c` | rules global | BridgeExplosionAnim pointer array |
| `g_RulesClass_Instance + 0x168` | rules global | BridgeExplosionAnim count |
| `g_Tactical + 0xD7C` | tactical global | Dirty-redraw flag |
| `AnimClass__Constructor` | (named) | Anim spawner |

---

## 10. Self-proof (exit gate step 4)

### Claim 1: Function address is `0x00575BA0`, not `0x00575870`
`search_functions "CollapseBridge_NS_High"` → `MapClass__CollapseBridge_NS_High @ 00575ba0`.
`get_function_by_address 0x00575BA0` confirms name. **MATCHES.**
`get_function_by_address 0x00575870` = `MapClass__CollapseBridge_EW_High` — confirms task spec address was wrong.

### Claim 2: Outer loop iterates 4 times (`local_2c = 4`)
Decompile shows `local_2c = 4; ... while (0 < local_2c)`. `read_memory 0x00575BBA` (32 bytes) =
`... c7 44 24 28 01 00 00 00 ...` — `MOV [ESP+0x28], 1` is `local_14 = 1`. The `local_2c = 4`
init is at a slightly different offset; confirmed indirectly by the decompile's `local_2c = 4`
literal in the pseudocode. **MATCHES task spec "4 axial iterations."**

### Claim 3: `DestroyBridge_High` is at `0x0057CCF0`
`search_functions "DestroyBridge_High"` → `DestroyBridge_High @ 0057ccf0`. **VERIFIED.**

---

## 11. Active-in-YR classification

| Finding | Active in YR? |
|---------|---------------|
| 4-iteration main collapse loop | **YES** |
| Debris anim spawn (3 per row, skip on `0xE8`) | **YES** |
| `DestroyBridge_High` retry-3 inner loop | **YES** |
| `UpdateBridgeZonesHelper` epilogue | **YES** |
| `g_Tactical+0xD7C` dirty flag | **YES** |

---

## Sources

**Ghidra MCP calls:**
- `decompile_function 0x00575BA0`
- `get_function_by_address 0x00575BA0` — `MapClass__CollapseBridge_NS_High`
- `get_function_by_address 0x00575870` — `MapClass__CollapseBridge_EW_High` (address conflict)
- `search_functions "CollapseBridge_NS_High"` → `0x00575BA0`
- `search_functions "CollapseBridge_EW_High"` → `0x00575870`
- `search_functions "DestroyBridge_High"` → `0x0057CCF0`
- `read_memory 0x00575BA0` (16 bytes) — entry bytes verified
- `read_memory 0x00575BBA` (32 bytes) — init block bytes verified
- `read_memory 0x00575C5A` (12 bytes) — bounds check bytes verified
- `read_memory 0x00575CA0` (16 bytes) — inner loop count `03 00 00 00` confirmed

**Prior docs cross-referenced:**
- `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`
- `BRIDGEHEAD_DIRECT_DAMAGE_SLOT3_COLLAPSE_GHIDRA_REPORT.md`
- `fn-DestroyBridgeFromCell-Low.md` — overlay range constants for low-bridge comparison
