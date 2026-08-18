# MapClass::CollapseBridge_EW_Low — Decode Doc

Address: `0x00575220` (Ghidra-confirmed; task description listed `0x00575540` which is
`CollapseBridge_NS_Low` — address discrepancy noted; this doc uses the verified address.)  
Scope: Full function.

## Summary

`MapClass::CollapseBridge_EW_Low` is the EW-axis low-bridge collapse walker. Given the
start cell of a low (wooden) EW bridge, it walks the bridge along the X axis (east-west),
spawns 3 explosion animations per step at perpendicular cells (Y−1, Y, Y+1), and calls
`DestroyBridge_Low` up to 3 times per step to collapse individual bridge sections. It
performs a span-finding pre-pass (backward X−, forward X+) to centre the walk on the
bridge's midpoint, runs up to 4 steps, and on exit calls `UpdateBridgeZonesHelper` and
sets `g_Tactical+0xD7C = 1`. It is the exact X-axis twin of `CollapseBridge_NS_Low`
(`0x00575540`), which walks Y instead.

## Active in YR

**Yes.** Verified via `get_function_callers 0x00575220`: single caller
`MapClass::DestroyBridgeFromCell_Low @ 0x00574780` (decode task #5), which is reachable
from `DestroyBridge_Low_OnHutDeath` (task #3). Live YR path, no TS gating.

## Decompilation Excerpt

From `decompile_function 0x00575220`:

```c
void MapClass__CollapseBridge_EW_Low(int *param_1)
{
    // param_1: packed cell coord — (short)iVar4 = X, (uint)iVar4>>16 = Y
    iVar4 = *param_1;   // packed coord: X = low 16 bits, Y = high 16 bits
    iVar13 = 0;
    local_1c = (ushort)iVar4;   // X coordinate
    sStack_1a = (short)(iVar4 >> 16); // Y coordinate
    local_28 = 1;               // step direction (+1 or -1 along X)
    iVar12 = 0;

    // ---- Span-finder: walk X-- to count backward cells in overlay [0x4A..0x65] ----
    do {
        local_1c--;
        iVar13++;
        iVar3 = MapClass__Get_CellClass(&local_1c);
        if (*(int*)(iVar3+0x44) < 0x4A) break;
    } while (*(int*)(iVar3+0x44) < 0x66);

    // ---- Span-finder: walk X++ to count forward cells ----
    do {
        local_1c++;
        iVar12++;
        iVar3 = MapClass__Get_CellClass(&local_1c);
        if (*(int*)(iVar3+0x44) < 0x4A) break;
        uVar1 = local_1c;
    } while (*(int*)(iVar3+0x44) < 0x66);

    if (iVar12 < iVar13) local_28 = -1;  // more cells behind → walk backward

    // Start X = input_X - (backward_count - forward_count) / 2
    uVar11 = iVar4 - (iVar13 - iVar12) / 2;
    local_1c = (ushort)uVar11;  // start X for main loop

    // ---- Main loop: up to 4 steps along X ----
    param_1 = (int*)4;  // loop counter
    while (0 < (int)param_1) {
        iVar4 = sStack_1a * 0x200 + (int)(short)local_1c;  // cell index = Y*0x200 + X
        puVar5 = *(CellClass**)(g_CellArray_Base + iVar4*4);

        if (*(int*)(puVar5+0x44) != 100) {  // 100 = 0x64 = destroyed-anchor sentinel; skip anim
            // Spawn 3 animations at perpendicular cells (Y-1, Y, Y+1) for current X
            sStack_12 = sStack_1a - 1;
            local_2c = 3;
            do {
                // Get cell at (X=sVar10, Y=sStack_12)
                // Compute anim position from CellClass+0x24 coord (leptons), +0x80 centre offset
                local_c = (short)*(CellClass*)(puVar5+0x24) * 0x100 + 0x80;  // X leptons
                local_8 = *(short*)(puVar5+0x24+2) * 0x100 + 0x80;           // Y leptons
                local_4 = (char)puVar5[0x11B] * DAT_00abde88;                // Z = height * scale

                // 4 RNG calls in fixed order (load-bearing for lockstep):
                Random__RandomRanged(0, 0x7ffffffe);  local_c = Math__ftol();  // X jitter
                Random__RandomRanged(0, 0x7ffffffe);  local_8 = Math__ftol();  // Y jitter
                pvVar6 = operator_new(0x1c8);
                if (pvVar6 != NULL) {
                    uVar7 = Random__RandomRanged(1, 5);  // frame delay
                    iVar4 = Random__RandomRanged(0, *(int*)(g_RulesClass_Instance+0x168) - 1);
                    // anim type from RulesClass+0x15C[index]
                    AnimClass__Constructor(
                        *(int*)(*(int*)(g_RulesClass_Instance+0x15C) + iVar4*4),
                        &local_c, uVar7, 1, 0x600, 0, 0);
                }
                sStack_12++;  // next Y row
                local_2c--;
            } while (local_2c != 0);
        }

        // ---- Collapse this bridge section (retry up to 3x) ----
        iVar4 = 0;
        do {
            cVar2 = DestroyBridge_Low(&local_1c);  // @ 0x0057BAA0
            if (cVar2 != '\0') break;
            iVar4++;
        } while (iVar4 < 3);

        local_1c += local_28;  // advance X by step direction
        param_1 = (int*)((int)param_1 - 1);  // decrement loop counter

        // Check if next cell is still in overlay band [0x4A..0x65]
        iVar4 = sStack_1a * 0x200 + (int)(short)local_1c;
        puVar5 = *(CellClass**)(g_CellArray_Base + iVar4*4);
        if ((*(int*)(puVar5+0x44) < 0x4A) || (0x65 < *(int*)(puVar5+0x44))) break;
    }

    // ---- Exit bookkeeping (always) ----
    MapClass__UpdateBridgeZonesHelper();
    *(byte*)(g_Tactical + 0xd7c) = 1;  // mark tactical dirty
}
```

## Behavioral Analysis

### Address discrepancy note

The task description listed address `0x00575540` for `CollapseBridge_EW_Low`. Ghidra
confirms `0x00575540` is `CollapseBridge_NS_Low` (walks Y axis). The correct EW variant
is at `0x00575220`, confirmed via `get_function_by_address 0x00575220` returning
"MapClass__CollapseBridge_EW_Low" and via the Ghidra plate comment on `0x00575220`
identifying it as the X-axis twin. This doc uses `0x00575220`.

### Span-finder (centering)

Before the main loop, the function walks X− and X+ from the input cell, counting how many
cells have overlay `CellClass+0x44 ∈ [0x4A, 0x65)`. The backward count (`iVar13`) and
forward count (`iVar12`) determine:
- Walk direction: if more cells are behind than ahead, walk backward (direction = -1).
- Start position: `start_X = input_X - (backward - forward) / 2` (signed integer division,
  rounds toward zero — odd spans have asymmetric centering).

This centres the 4-step walk on the approximate midpoint of the bridge.

### Main loop — 4 steps

The main loop runs up to 4 iterations (hardcoded `local_2c = 4` via `param_1 = (int*)4`
decremented each iteration). Each step:

1. **Sentinel check:** If `CellClass+0x44 == 0x64` (100 decimal), skip animation spawning.
   `0x64` is the destroyed-anchor sentinel tile — it marks a cell that is already the
   collapse anchor and should not get debris animations.

2. **3 animations at perpendicular cells:** For Y−1, Y, Y+1 at the current X:
   - Compute anim world position from `CellClass+0x24` (cell coord in leptons, `*0x100 + 0x80`
     to get cell-centre leptons).
   - Z = `CellClass+0x11B` (height byte) × `DAT_00ABDE88` (height scale factor).
   - 4 RNG calls in fixed order: X-jitter, Y-jitter, frame-delay (1–5), anim-index
     (0 to `RulesClass+0x168 - 1`). Anim type = `RulesClass+0x15C[index]`.
   - **RNG order is load-bearing for multiplayer lockstep**: up to 48 RNG consumptions
     per call (4 steps × 3 cells × 4 RNG calls = 48).

3. **Collapse call:** `DestroyBridge_Low(&local_1c)` called up to 3 times; retries on
   return value 0 (failure).

4. **Advance:** `local_1c += local_28` (X ± 1). Break if next cell is outside
   `[0x4A, 0x65]` overlay band.

### Post-loop (always)

- `MapClass::UpdateBridgeZonesHelper()` — unconditional (verified via callees).
- `g_Tactical + 0xD7C = 1` — marks tactical display dirty for redraw.

Note: unlike `DestroyBridge_Low_OnHutDeath` (task #3), this function does NOT call
`UpdateAdjacentBridges_High` — that call is only in the hut-death entry point.

### Axis difference from NS twin

`CollapseBridge_NS_Low` (`0x00575540`) steps Y (outer axis: Y, perpendicular: X−1..X+1).
This function steps X (outer axis: X, perpendicular: Y−1..Y+1). Otherwise the algorithm,
sentinel value (0x64), overlay band [0x4A..0x65], loop count (4), and RNG order are
identical.

## Struct Field Accesses

`param_1` is `int*` — pointer to a packed 32-bit cell coord: low 16 = X, high 16 = Y.

| Source | Offset | Field | Role |
|---|---|---|---|
| CellClass | `+0x44` | Overlay subtype | Low-bridge band check [0x4A..0x65]; `0x64`=destroyed-anchor sentinel |
| CellClass | `+0x24` | Cell coord (leptons) | Base position for animation world coordinates |
| CellClass | `+0x11B` | Height byte | Z-coordinate input (multiplied by `DAT_00ABDE88`) |

`CellClass+0x24` is read as a packed 32-bit value: `(short)*(uint32*)(+0x24)` = X leptons,
`(short)(*(uint32*)(+0x24) >> 16)` = Y leptons. The `*0x100 + 0x80` formula converts from
cell-grid coords (256 leptons/cell) to the centre of the cell.

## Globals Referenced

| Global | Role |
|---|---|
| `g_CellArray_Base` | Cell array base (`Y*0x200 + X` indexed) |
| `DAT_00ABDC50` | Sentinel null-cell for out-of-bounds coords |
| `DAT_00ABDC74` | Scratch coord storage on out-of-bounds |
| `DAT_00ABDE88` | Height scale factor for Z coordinate |
| `g_RulesClass_Instance + 0x15C` | Pointer to debris/explosion anim type array |
| `g_RulesClass_Instance + 0x168` | Count of entries in anim type array |
| `g_Tactical + 0xD7C` | Tactical display dirty flag |

## Out-of-scope Refs

- `DestroyBridge_Low` @ `0x0057BAA0` — the per-section collapse function; distinct from
  `DestroyBridge_Low_OnHutDeath` (entry point). Decode tasks exist for the entry point (#3)
  and its variants; `0x0057BAA0` is a lower-level function.
- `MapClass::UpdateBridgeZonesHelper` @ `0x0056C510` — decode task #11
- `MapClass::DestroyBridgeFromCell_Low` @ `0x00574780` — sole caller; decode task #5
- `CollapseBridge_NS_Low` @ `0x00575540` — X/Y axis twin; decode task #7

## Unverified Claims (YELLOW)

- `CellClass+0x24` field interpretation as "cell coord in leptons" is inferred from the
  `*0x100 + 0x80` formula (cell-to-lepton conversion with cell-centre offset) and the
  address being passed to `AnimClass::Constructor` as world position. Struct layout
  confirmation expected from task #21.
- `CellClass+0x11B` as "height byte" is inferred from the multiply by `DAT_00ABDE88`
  to produce a Z value; the field is read with a `(char)` cast. Not confirmed via struct
  layout.
- `DAT_00ABDE88` is identified as a height scale factor from usage context. Static value
  unknown; runtime-populated.
- The "destroyed-anchor sentinel = 0x64 (100 decimal)" interpretation is inferred from
  the decompilation checking `*(int*)(puVar5+0x44) != 100` before spawning anims. The
  Ghidra plate comment on `0x00575540` confirms "sentinel = 0x64/0x65" for NS_Low;
  this function uses 100 = 0x64. Confirmed consistent.
