# fn-MapClass-UpdateAdjacentBridges_High

**Address:** `0x00576770`
**Class:** `MapClass`
**Method:** `UpdateAdjacentBridges_High`
**Confidence:** HIGH (content, identity, callers verified via Ghidra MCP)
**YR-active:** YES — called from `DestroyBridge_High_OnHutDeath` (C4/bomb collapse), `DestroyBridge_Low_OnHutDeath` (also calls the high variant after low-bridge collapse), and `ProcessBridgeDamageStateMachine_High`.

---

## Signature

```c
void __thiscall MapClass__UpdateAdjacentBridges_High(int param_1, short *param_2)
```

- `param_1` = `MapClass*` (int, direct byte offsets)
- `param_2` = starting cell coordinate (2 shorts: X, Y) — typically the ramp-anchor cell passed from the collapse function

Verified via `decompile_function 0x00576770`.

---

## Callers

Verified via `get_function_callers 0x00576770`:

| Caller | Address | Context |
|---|---|---|
| `MapClass__DestroyBridge_High_OnHutDeath` | `0x00574000` | High-bridge C4/bomb collapse |
| `MapClass__DestroyBridge_Low_OnHutDeath` | `0x00574C20` | Low-bridge collapse (also calls high adjacency update) |
| `ProcessBridgeDamageStateMachine_High` | `0x00576BA0` | Bridge damage state machine (not C4 path) |

Note: `DestroyBridge_Low_OnHutDeath` at `0x00574C20` calls this **high** variant despite being a low-bridge collapse. This means the high-bridge edge tile update runs for both low and high bridge destructions.

---

## Callees

Verified via `get_function_callees 0x00576770`:

| Callee | Address | Role |
|---|---|---|
| `MapClass__UpdateBridgeEdgeTiles_High` | `0x00576200` | Update the visual edge tiles at bridge endpoint |
| `TacticalClass__DirtyScreenRect` | `0x006D2790` | Mark screen rect dirty for redraw |

---

## Algorithm

### Phase 1 — Adjacent bridge-flagged cell search (8-direction walk)

```c
uVar8 = 0;   // direction index (0–7)
iVar7 = 0;   // counter
do {
    // Step one cell in direction uVar8 from param_2
    sVar5 = *psVar1 + g_DirectionOffsets[uVar8].x;
    sVar3 = psVar1[1] + g_DirectionOffsets[uVar8].y;
    iVar4 = sVar3 * 0x200 + sVar5;  // cell index
    // Bounds check + cell lookup
    if ((*(uint *)(puVar6 + 0x140) & 0x500) != 0) break;  // found bridge-flagged cell
    uVar8 = (uVar8 + 1) & 7;
    iVar7++;
} while (iVar7 < 8);
```

Walks all 8 adjacent cells (NESW + diagonals) looking for one with `CellClass + 0x140` flags `0x500` (`0x100 | 0x400` = any bridge-flagged cell). Breaks on first match.

If no bridge-flagged neighbor found after 8 directions: falls through with `puVar6` pointing to wherever the walk ended.

### Phase 2 — Bridge orientation and axis determination

```c
uVar8 = *(uint *)(puVar6 + 0x140);
if (((uVar8 & 0x100) == 0) && ((uVar8 & 0x400) == 0)) return;  // no bridge
```

Three branches based on flag bits:

**EW bridge (flag 0x400 set, not 0x100):**
```c
// Walk EW direction to find non-0x400-flagged cell (up to 4 steps, then bail)
// Derive ramp anchor as 2 steps back from walk endpoint
```

**NS bridge via linked cell (flag 0x100 set, 0x80 not set):**
```c
param_2 = *(short **)(*(int *)(puVar6 + 0x2c) + 0x24);  // follow CellClass+0x2C link
```

**NS bridge direct (flag 0x100 and 0x80 set):**
```c
param_2 = *(short **)(puVar6 + 0x24);  // use this cell's own coord
```

`CellClass + 0x2C` = linked-cell pointer (NS bridge chain). `CellClass + 0x24` = packed XY coord (same offsets used in prior decode tasks).
`CellClass + 0x80` = sub-flag within `+0x140` word. `CellClass + 0x800` = direction sub-flag.

### Phase 3 — Diagonal bounds check

```c
while (true) {
    iVar7 = (int)param_2._2_2_;   // Y
    iVar4 = (int)(short)param_2;  // X
    if (iVar7 + iVar4 <= DAT_0087f8dc) return;      // diagonal bound NW
    if (DAT_0087f8dc <= iVar4 - iVar7) return;       // diagonal bound NE
    if (DAT_0087f8dc <= iVar7 - iVar4) return;       // diagonal bound SW
    if (DAT_0087f8dc + DAT_0087f8e0 * 2 < iVar7 + iVar4) return; // diagonal bound SE
    // Check cell visibility array
    if (*(int *)(param_1 + 0x13C)[iVar4] != 0) break;
    // Advance one step in bridge direction (uVar8)
    param_2 = advance_one_step(param_2, uVar8);
}
```

`DAT_0087f8dc` and `DAT_0087f8e0` are diamond-map boundary constants. The condition `iVar7 + iVar4 <= DAT_0087f8dc` rejects cells that are off the diamond map in the NW direction. These 4 checks form the standard RA2 isometric diamond bounds test.

`param_1 + 0x13C` = `MapClass` cell-visibility array base (confirmed from prior decode of `DestroyBridge_High_OnHutDeath`).

### Phase 4 — Tile classification and edge tile update

Once a valid in-bounds cell is found, the function computes a tile-relative index and classifies the bridge endpoint type:

```c
iVar7 = (*(int *)(puVar6 + 0x38) - DAT_00aa0e28) + 1;  // tile index relative to tile-base
```

`CellClass + 0x38` = tile index. `DAT_00aa0e28` = high-bridge tile base global (same as used in `DestroyBridge_High_OnHutDeath`).

The classification compares `iVar7` against several global tile-constant pairs:

| Condition | Height byte | UpdateBridgeEdgeTiles arg |
|---|---|---|
| `iVar7 == DAT_00abc2b4` or `== DAT_00aa1130` AND `cell.+0x11A == 0x08` | `\b` (8) | 2 |
| `iVar7 in [DAT_00abad30, DAT_00abad30+3]` AND `cell.+0x11A == 0x05` | `\x05` (5) | 2 |
| `iVar7 == DAT_00aa1548` or `== DAT_00aa0740` AND `cell.+0x11A == 0x0C` | `\f` (12) | 4 |
| `iVar7 in [DAT_00aa1028, DAT_00aa1028+3]` AND `cell.+0x11A == 0x07` | `\a` (7) | 4 |

`CellClass + 0x11A` = cell height/slope byte (one byte before the `+0x11B` Z-scaler used in collapse walkers).

If none match → loop back (`goto LAB_00576a74`) to advance another step.

If matched → call `MapClass__UpdateBridgeEdgeTiles_High(&coord, arg, &rect_out)` at `0x00576200`:

```c
cVar2 = MapClass__UpdateBridgeEdgeTiles_High(&param_2, uVar10, &local_10);
if (cVar2 == '\0') return;
if (rect unchanged from pre-call) return;  // no visual change
TacticalClass__DirtyScreenRect(local_10, local_c, local_8, local_4, 0);
```

- If `UpdateBridgeEdgeTiles_High` returns 0 (failure), early return — no dirty rect.
- If it returns non-zero but the output rect is unchanged from the pre-call snapshot (`DAT_00abd470..47C`), early return — no dirty rect.
- Otherwise: call `TacticalClass__DirtyScreenRect` at `0x006D2790` to mark the changed screen region dirty.

The rect output (`local_10..local_4`) is a 4-int screen rect (left, top, right, bottom or similar).

---

## Key Struct Offsets

All verified via `decompile_function 0x00576770`:

| Object | Byte offset | Field |
|---|---|---|
| `CellClass` | `+0x140` | Bridge flag word (0x100=NS, 0x400=EW, 0x080, 0x800) |
| `CellClass` | `+0x24` | Packed XY coord |
| `CellClass` | `+0x2C` | Linked-cell pointer |
| `CellClass` | `+0x38` | Tile index |
| `CellClass` | `+0x11A` | Cell height/slope byte |
| `MapClass` | `+0x13C` | Cell visibility array base |
| `g_DirectionOffsets` | global | 8-entry table of (dx, dy) short pairs |

---

## Observable Effect

`UpdateAdjacentBridges_High` is the function responsible for **visually updating bridge endpoint tiles** after a collapse or damage event. Without it, the bridge tiles at the edges of a destroyed span would display incorrect graphics (intact ramp tiles next to a gap). It is called unconditionally as part of every high-bridge collapse tail (and also from `DestroyBridge_Low_OnHutDeath`).

The call to `TacticalClass__DirtyScreenRect` ensures the player sees the updated tiles in the same frame — the redraw is not deferred.

---

## Unverified

**YELLOW:** `DAT_0087f8dc` and `DAT_0087f8e0` — diamond map boundary constants. Addresses visible in decompilation but values not fetched via `read_memory`.

**YELLOW:** `DAT_00abc2b4`, `DAT_00aa1130`, `DAT_00abad30`, `DAT_00aa1548`, `DAT_00aa0740`, `DAT_00aa1028` — tile classification constants. These are addresses of globals holding tile index values. Not fetched.

**YELLOW:** `DAT_00abd470..47C` — 4-int pre-call snapshot of the dirty-rect state. These are globals that `UpdateBridgeEdgeTiles_High` writes to. Not fetched.
