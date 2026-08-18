# fn-CollapseBridge_NS_Low (and EW_Low twin)

**LABEL NOTE:** The task list assigned "CollapseBridge_NS_Low" to address `0x00575220` and "CollapseBridge_EW_Low" to `0x00575540`. Ghidra's actual labels are **inverted**: `0x00575220` = `MapClass__CollapseBridge_EW_Low`, `0x00575540` = `MapClass__CollapseBridge_NS_Low`. Both are decoded here since this task covers the assigned address `0x00575220`, and the NS variant at `0x00575540` is the structural twin covered by task #8. Confirmed via `get_function_by_address 0x00575220` and `get_function_by_address 0x00575540`.

---

## Primary Target: MapClass__CollapseBridge_EW_Low

**Address:** `0x00575220`
**Function body:** `0x00575220 – 0x0057553F`
**Confidence:** HIGH (content, identity, callers verified via Ghidra MCP)
**YR-active:** YES — reachable from `DestroyBridgeFromCell_Low @ 0x00574780`, which is called by the bridge-collapse chain.

### Signature

```c
void MapClass__CollapseBridge_EW_Low(int *param_1)
```

- `param_1` = packed cell coordinate (2 shorts): `*(short *)param_1` = X, `*((short *)param_1 + 1)` = Y.
- No `this` pointer — function operates on the global map via `g_CellArray_Base` and `g_RulesClass_Instance`.

Verified via `decompile_function 0x00575220`.

### Caller

Verified via `get_function_callers 0x00575220`:

| Caller | Address |
|---|---|
| `MapClass__DestroyBridgeFromCell_Low` | `0x00574780` |

Single caller. `DestroyBridgeFromCell_Low` selects the NS or EW collapse walker based on the bridge orientation determined by its own overlay-range analysis.

### Callees

Verified via `get_function_callees 0x00575220`:

| Callee | Address | Role |
|---|---|---|
| `MapClass__Get_CellClass` | `0x005657A0` | Convert cell coord to CellClass pointer |
| `DestroyBridge_Low` | `0x0057BAA0` | Destroy one low-bridge tile cell |
| `AnimClass__Constructor` | `0x00421EA0` | Spawn debris anim at cell |
| `Random__RandomRanged` | `0x0065C7E0` | RNG (called 4× per debris anim) |
| `Math__ftol` | `0x007C5F00` | Float-to-int for jitter offset |
| `operator_new` | `0x007C8E17` | Alloc AnimClass instance |
| `MapClass__UpdateBridgeZonesHelper` | `0x0056C510` | Post-collapse zone graph update |

---

### Algorithm — EW_Low (0x00575220)

**Phase 1: Span finder (axis = X)**

```c
iVar4 = *param_1;        // packed coord: low16=X, high16=Y
iVar13 = 0;
local_1c = (ushort)iVar4;   // start X
sVar8 = (short)((uint)iVar4 >> 16);  // Y (fixed)
local_28 = 1;
iVar12 = 0;

// Walk backward (X--) while overlay in [0x4A..0x65]
do {
    local_1c--;
    iVar13++;
    iVar3 = MapClass__Get_CellClass(&local_1c);
    if (*(int *)(iVar3 + 0x44) < 0x4a) break;
} while (*(int *)(iVar3 + 0x44) < 0x66);

// Walk forward (X++) while overlay in [0x4A..0x65]
do {
    local_1c = uVar1 + 1;
    iVar12++;
    iVar3 = MapClass__Get_CellClass(&local_1c);
    if (*(int *)(iVar3 + 0x44) < 0x4a) break;
    uVar1 = local_1c;
} while (*(int *)(iVar3 + 0x44) < 0x66);

// Choose direction: step toward shorter span side
if (iVar12 < iVar13) local_28 = -1;   // -1 = step X-- (west), +1 = step X++ (east)

// Start cell: center of span, biased by signed (back-fwd)/2
uVar11 = iVar4 - (iVar13 - iVar12) / 2;  // SIGNED integer division
```

`CellClass + 0x44` = overlay tile index (same offset verified in prior tasks).

**Phase 2: Collapse loop (4 iterations)**

```c
param_1 = (int *)0x4;   // iteration counter (decrements to 0)
while (0 < (int)param_1) {
    sVar8 = (short)uVar9;   // current X
    iVar4 = sStack_1a * 0x200 + (int)sVar8;   // cell index

    // Terminal check: overlay != 0x64 (EW destroyed-anchor sentinel)
    if (*(int *)(puVar5 + 0x44) != 100) {   // 100 = 0x64
        // Spawn 3 debris anims at cells (Y-1, Y, Y+1) from current X
        // (perpendicular = Y axis for EW bridge)
        local_34 = CONCAT22(sStack_1a + -1, sVar8);   // (X, Y-1)
        local_2c = 3;
        do {
            // Each anim: 4 RNG calls in this order:
            // 1. Random__RandomRanged(0, 0x7FFFFFFE) → X jitter (ftol)
            // 2. Random__RandomRanged(0, 0x7FFFFFFE) → Y jitter (ftol)
            // 3. Random__RandomRanged(1, 5) → frame delay
            // 4. Random__RandomRanged(0, RulesClass+0x168 - 1) → anim index
            // anim type = *(RulesClass+0x15C)[index]
            // position = cell.+0x24 (packed XY in leptons, * 0x100 + 0x80 = cell center)
            // Z = cell.+0x11B * DAT_00abde88
            // Flags = 0x600
            AnimClass__Constructor(...);
            sStack_12++;   // Y+1 for next iteration
        } while (local_2c-- != 0);
    }

    // Destroy the bridge tile (up to 3 retries)
    iVar4 = 0;
    do {
        cVar2 = DestroyBridge_Low(&local_1c);
        if (cVar2 != '\0') break;
        iVar4++;
    } while (iVar4 < 3);

    // Advance X in chosen direction; break if out of overlay band
    local_1c += local_28;
    if ((*(int *)(puVar5 + 0x44) < 0x4a) || (0x65 < *(int *)(puVar5 + 0x44))) break;
    param_1--;
}
```

Key constants:
- `0x64` = 100 decimal = EW low-bridge **destroyed-anchor sentinel** overlay index. When a cell carries this, the debris-spawn phase is skipped (tile is already a "rubble" state).
- The loop counter starts at 4 (`param_1 = 4`) but breaks early if the overlay check fails — so effective span is 1–4 cells.
- Each cell spawns 3 anims (one at Y-1, Y, Y+1 from current X).
- Total RNG consumption: up to `4 iterations × 3 anims × 4 RNG calls = 48` calls per invocation. **Load-bearing for multiplayer lockstep** — the RNG order must be preserved exactly.

**Phase 3: Post-collapse tail (unconditional)**

```c
MapClass__UpdateBridgeZonesHelper();        // zone graph sync
*(undefined1 *)(g_Tactical + 0xd7c) = 1;   // tactical dirty flag
return;
```

`UpdateBridgeZonesHelper` at `0x0056C510` and `g_Tactical+0xD7C=1` are confirmed identical to the pattern in `DestroyBridge_High_OnHutDeath`. Verified via `decompile_function 0x00575220`.

---

## Structural Twin: MapClass__CollapseBridge_NS_Low

**Address:** `0x00575540`
**Caller:** `MapClass__DestroyBridgeFromCell_Low @ 0x00574780` (same caller as EW)

The NS variant has an **identical structure** with one axis swap: where EW steps X (and spawns anims at Y-1, Y, Y+1), NS steps Y (and spawns anims at X-1, X, X+1).

Key differences:

| Aspect | EW_Low (0x00575220) | NS_Low (0x00575540) |
|---|---|---|
| Walk axis (span finder) | X-- / X++ | Y-- / Y++ |
| Step axis (main loop) | X ± local_28 | Y ± local_14 |
| Perp anim cells | Y-1, Y, Y+1 | X-1, X, X+1 |
| Terminal sentinel | `0x64` (EW anchor) | `0x65` (NS anchor) |
| Initial `param_1` fetch | `iVar4 = *param_1` (int) | `uVar9 = *param_1` (uint) |

The NS plate comment at `0x00575540` confirms (verified via `decompile_function 0x00575540`):
> "sentinel = 0x64 for EW twin, 0x65 for NS"

**NS-Low terminal sentinel:** overlay index `0x65` (101 decimal) = NS low-bridge destroyed-anchor.

---

## Debris Anim Parameters

Both walkers spawn debris using the same anim pool from `RulesClass`:

| Field | Offset | Content |
|---|---|---|
| `RulesClass + 0x15C` | `0x15C` | Pointer to explosion anim array |
| `RulesClass + 0x168` | `0x168` | Count of explosion anims |
| `CellClass + 0x24` | `0x24` | Packed XY coord (lepton units) used as anim position base |
| `CellClass + 0x11B` | `0x11B` | Cell height byte; multiplied by `DAT_00abde88` for Z |

Anim position: `cell.+0x24` (X,Y) unpacked to leptons, then `* 0x100 + 0x80` = cell-center in lepton space (same formula as standard coord centering: `(coord << 8) | 0x80`).

`DAT_00abde88` = cell height scaler (global constant). Its runtime value is YELLOW (not read in this session).

---

## Overlay Band and Sentinels (Low Bridge)

| Range | Meaning |
|---|---|
| `[0x4A, 0x65]` | Live low-bridge overlay (both NS and EW) |
| `0x64` | EW destroyed-anchor (skip debris spawn) |
| `0x65` | NS destroyed-anchor (skip debris spawn) |
| `< 0x4A` or `> 0x65` | Exit span-finder / main loop (out of bridge band) |

---

## RNG Lockstep Note

Each invocation of either walker consumes up to 48 `Random__RandomRanged` calls in a deterministic order. The Rust port must reproduce this exact order to maintain multiplayer lockstep. The 4-call per-anim sequence is:

1. X-jitter: `Random__RandomRanged(0, 0x7FFFFFFE)` → `Math::ftol`
2. Y-jitter: `Random__RandomRanged(0, 0x7FFFFFFE)` → `Math::ftol`
3. Frame delay: `Random__RandomRanged(1, 5)`
4. Anim index: `Random__RandomRanged(0, RulesClass+0x168 - 1)`

---

## Unverified

**YELLOW:** `DAT_00abde88` runtime value — cell height scaler for anim Z position. Address observed in decompilation but not fetched via `read_memory`.

**YELLOW:** Whether `MapClass__UpdateAdjacentBridges_High` (or a Low equivalent) is called by either walker — the decompilation of both shows only `UpdateBridgeZonesHelper` and the Tactical flag, no `UpdateAdjacentBridges_Low`. The High variant calls `UpdateAdjacentBridges_High` before the zone helper; the Low walker omits this call entirely. This may be intentional or a latent gap — flagged for synthesis.
