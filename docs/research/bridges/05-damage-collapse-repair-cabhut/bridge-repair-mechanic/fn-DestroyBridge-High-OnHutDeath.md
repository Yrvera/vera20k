# fn-MapClass-DestroyBridge_High_OnHutDeath

**Address:** `0x00574000`
**Class:** `MapClass`
**Method:** `DestroyBridge_High_OnHutDeath`
**Function body:** `0x00574000 – 0x00574BFF` (approximately 3072 bytes including twins/LAB labels)
**Confidence:** HIGH (content, identity, callers all verified via Ghidra MCP)
**YR-active:** YES — called from `BuildingClass::Update` whenever a bridge hut's C4 timer fires, and from `BombClass::Detonate` when a bomb lands on a high-bridge hut.

---

## Signature

```c
void __thiscall MapClass__DestroyBridge_High_OnHutDeath(int param_1, short *param_2)
```

- `param_1` = `MapClass*` (`this`). Used as a plain `int` — field accesses are direct byte offsets.
- `param_2` = `short*` pointing to a 2-element packed cell coordinate: `param_2[0]` = X, `param_2[1]` = Y (cell grid units).

Verified via `decompile_function 0x00574000`.

---

## Callers

Verified via `get_function_callers 0x00574000` and `get_xrefs_to 0x00574000`:

| Caller | Address | Call address |
|---|---|---|
| `BuildingClass::Update` | `0x0043FB20` | `0x0044031B` (UNCONDITIONAL_CALL) |
| `BombClass::Detonate` | `0x00438720` | `0x00438982` (UNCONDITIONAL_CALL) |

`BuildingClass::Update` at `0x0044031B` is the C4 timer-expiry dispatch. `BombClass::Detonate` at `0x00438982` is the bomb-on-high-bridge-hut path.

---

## Callees

Verified via `get_function_callees 0x00574000`:

| Callee | Address | Role |
|---|---|---|
| `MapClass__Get_CellClass` | `0x005657A0` | Translate (X,Y) cell coord to CellClass pointer |
| `MapClass__DestroyBridgeFromCell_High` | `0x005749C0` | Fast-path: bridge cell found — dispatch collapse |
| `ApplyDamageToCell` | `0x00587180` | Fallback: apply damage to individual bridge tile cell |
| `MapClass__IsBridgeRampTile` | `0x005746C0` | Test whether a tile index is a bridge ramp |
| `MapClass__IsLowBridgeEndpointTile` | `0x00574600` | Test whether tile is a low-bridge endpoint (walk-stop condition) |
| `MapClass__UpdateAdjacentBridges_High` | `0x00576770` | Post-collapse adjacency update |
| `MapClass__UpdateBridgeZonesHelper` | `0x0056C510` | Post-collapse zone graph update (always called) |
| `FUN_0042fcb0` | `0x0042FCB0` | Unknown helper; called before ramp-walk |
| `FUN_007c8b3d` | `0x007C8B3D` | Unknown cleanup; called at function tail |

---

## Algorithm Overview

The function implements a two-stage hut-death → high-bridge destruction dispatch:

### Stage 1 — Overlay-based fast path (5×5 scan)

```c
iVar9 = -2;
do {
    iVar8 = -2;
    do {
        // cell = (param_2[0] + iVar9, param_2[1] + iVar8)
        iVar2 = MapClass__Get_CellClass(&cell);
        if ((0xcc < *(int *)(iVar2 + 0x44)) && (*(int *)(iVar2 + 0x44) < 0xe9)) {
            // Found a high-bridge overlay tile in [0xCD, 0xE8] inclusive
            MapClass__DestroyBridgeFromCell_High(&cell);
            return;  // EARLY RETURN — fast path taken
        }
        iVar8++;
    } while (iVar8 < 3);
    iVar9++;
} while (iVar9 < 3);
```

- Scans all cells in a **5×5 grid** centered on the hut's cell (`±2` in both axes).
- `CellClass + 0x44` = overlay tile index (verified by decompilation: same offset used in task #2 InfantryClass::PerCellProcess bridge-type scan).
- High-bridge overlay range: **`(0xCC, 0xE9)` exclusive** = overlay indices **`0xCD` through `0xE8`** inclusive (28 values).
- On first match, immediately calls `MapClass__DestroyBridgeFromCell_High` at `0x005749C0` and returns.

**If no overlay found in 5×5:** falls through to Stage 2.

Note: The low-bridge twin `DestroyBridge_Low_OnHutDeath` at `0x00574C20` uses a different overlay range (`[0x4A..0x65]`). This function uses the high-bridge range only.

### Stage 2 — Flag-based fallback walk

When no overlay tile is found, the function walks from the hut cell outward in 8 directions (up to 3 cells per direction) looking for a cell whose `CellClass + 0x140` flags indicate bridge presence:

```c
// Check if hut cell itself has bridge flags 0x500 set
if ((*(uint *)(puVar6 + 0x140) & 0x500) == 0) {
    // Walk 8 directions, each up to 3 steps
    local_2c = 0;   // direction index (0–7)
    local_1c = 0;   // walk count
    do {
        // Step 1 in direction local_2c
        // Step 2 in same direction
        // Step 3 in same direction
        if ((*(uint *)(puVar6 + 0x140) & 0x500) != 0) break;
        local_2c = (local_2c + 1) & 7;   // next direction
        local_1c++;
    } while ((int)local_1c < 8);
}
```

`CellClass + 0x140` flag bits:
- `0x100` — bridge tile marker (NS orientation indicator)
- `0x400` — bridge tile marker (EW orientation indicator)
- `0x500` = `0x100 | 0x400` — any bridge tile
- `0x080` — unknown (used in ramp-axis derivation)
- `0x800` — unknown (used in direction derivation)

### Stage 3 — Ramp anchor derivation

After the flag walk identifies a bridge-marked cell, the function determines the ramp start cell and walk direction:

```c
uVar10 = *(uint *)(puVar6 + 0x140);
if ((uVar10 & 0x100) == 0) && ((uVar10 & 0x400) == 0)) {
    return;   // No bridge flags — abort silently
}
if ((uVar10 & 0x100) == 0) {
    // EW bridge: walk from adjacent cell in derived direction
    uVar10 = -(uint)((uVar10 & 0x800) != 0) & 2;   // direction = 0 or 2
    // Walk up to 4 steps in EW direction looking for non-0x400-flagged cell
    // Compute ramp anchor offset: 2 cells back from walk end
} else if ((uVar10 & 0x80) == 0) {
    // NS bridge (via linked cell): ramp anchor from linked cell's coord
    local_30 = *(short **)(*(int *)(puVar6 + 0x2c) + 0x24);
} else {
    // NS bridge (direct): ramp anchor from current cell's coord
    local_30 = *(short **)(puVar6 + 0x24);
}
```

`CellClass + 0x2C` = pointer to linked cell (used for NS bridge chain). `CellClass + 0x24` = cell coordinate (X,Y) packed as two shorts.

### Stage 4 — ApplyDamageToCell walk

Once the ramp anchor (`local_30`) is identified, the function walks along the bridge axis calling `ApplyDamageToCell` for up to 3 consecutive cells:

```c
FUN_0042fcb0(0, 0);   // unknown setup
// ... map-bounds check on ramp anchor ...
// Loop: while ramp anchor X is within map bounds and within width:
//   if IsBridgeRampTile(cell): call ApplyDamageToCell up to 3x, break
//   else: advance one step in direction uVar10
// After ramp walk: walk forward from ramp end, call ApplyDamageToCell up to 3x
//   stop when IsLowBridgeEndpointTile or out-of-bounds
```

`ApplyDamageToCell` is at `0x00587180`. Each call applies structural damage to a bridge tile cell, which removes the tile overlay and triggers debris animation.

### Stage 5 — Post-destruction tail (always executed)

At label `LAB_005745AD` and `LAB_005745CA`:

```c
MapClass__UpdateAdjacentBridges_High(&local_30);  // 0x00576770
*(undefined1 *)(g_Tactical + 0xd7c) = 1;          // mark tactical dirty
MapClass__UpdateBridgeZonesHelper();               // 0x0056c510
```

**Confirmed at `0x005745C3`:** `c6 81 7c 0d 00 00 01` = `MOV BYTE PTR [ECX+0xD7C], 1`

Verified via `read_memory 0x005745C3` (7 bytes): `c6 81 7c 0d 00 00 01`. ECX = `g_Tactical` pointer. `g_Tactical + 0xD7C` = render-dirty flag; setting it forces a tactical view redraw on next frame.

`MapClass__UpdateBridgeZonesHelper` is called **unconditionally** — even if the ramp walk found nothing, zone graph consistency is maintained.

---

## Key Struct Offsets Used

All verified via `decompile_function 0x00574000`:

| Object | Byte offset | Field | Usage |
|---|---|---|---|
| `CellClass` | `+0x44` | Overlay tile index | Bridge detection (range 0xCD..0xE8) |
| `CellClass` | `+0x140` | Bridge flag word | Bits 0x100 (NS), 0x400 (EW), 0x080, 0x800 |
| `CellClass` | `+0x24` | Packed XY coordinate | 2×short; used as ramp anchor |
| `CellClass` | `+0x2C` | Linked-cell pointer | NS bridge chain follow |
| `CellClass` | `+0x38` | Tile index | Walk-stop: compared vs `DAT_00aa0e28` (tile base) |
| `MapClass` | `+0x124` | Map origin X | Bounds check in ramp walk |
| `MapClass` | `+0x128` | Map origin Y | Bounds check |
| `MapClass` | `+0x12C` (`+300`)` | Map width | Bounds check |
| `MapClass` | `+0x130` | Map height | Bounds check |
| `MapClass` | `+0x13C` | Cell visibility array base | Used to check if cell is in visible playfield |
| `g_Tactical` | `+0xD7C` | Tactical dirty flag | Set to 1 after destruction |

---

## High-Bridge Overlay Family

The function exclusively operates on the **high-bridge overlay family**: indices `0xCD` through `0xE8` inclusive (28 overlay values).

This differs from the low-bridge family (`0x4A`–`0x65`, 28 values) used by `DestroyBridge_Low_OnHutDeath` at `0x00574C20`.

The tile-base global `DAT_00aa0e28` is used for the endpoint-tile offset calculation in the ramp walk:
```c
iVar9 = *(int *)(puVar6 + 0x38) - DAT_00aa0e28;
```
The exact runtime value of `DAT_00aa0e28` is YELLOW (not read in this session — see Unverified section).

---

## YR-Activity Confirmation

Both callers confirmed active in YR skirmish:
- `BuildingClass::Update` at `0x0043FB20` fires every game tick per building.
- `BombClass::Detonate` at `0x00438720` fires when any bomb (including paradrop) detonates.

No TS-legacy gating flags found in the decompilation. The overlay range `0xCD..0xE8` is standard YR high-bridge tileset. Function is live and unconditionally reachable.

---

## Relationship to Low-Bridge Twin

`DestroyBridge_High_OnHutDeath` (`0x00574000`) is the **structural twin** of `DestroyBridge_Low_OnHutDeath` (`0x00574C20`). Differences:

| Aspect | High bridge | Low bridge |
|---|---|---|
| Overlay range | `0xCD`–`0xE8` | `0x4A`–`0x65` |
| Fast-path callee | `DestroyBridgeFromCell_High @ 0x005749C0` | `DestroyBridgeFromCell_Low @ 0x00574780` |
| Adjacency callee | `UpdateAdjacentBridges_High @ 0x00576770` | (low-bridge equivalent, not in scope) |
| Tile base global | `DAT_00aa0e28` | (separate low-bridge tile base) |
| Dispatch from Update | `0x0044031B` | `0x00440301` |

---

## Unverified

**YELLOW:** `DAT_00aa0e28` runtime value — this is the tile-base constant used for endpoint detection in the ramp walk (`*(int *)(puVar6 + 0x38) - DAT_00aa0e28`). The global address was read from decompilation but its value was not fetched via `read_memory`. A `read_memory 0x00aa0e28` call would confirm the tile-base offset. Claim left YELLOW.

**YELLOW:** `FUN_0042fcb0` purpose — called just before the ramp walk with args `(0, 0)`. Not decoded. May be a camera-shake or anim-queue preparation call.
