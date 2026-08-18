# CellClass Bridge-Relevant Fields — Struct Decode Doc

Source struct: `CellClass`
Scope: Bridge-mechanic fields only — `+0x24`, `+0x2C`, `+0x38`, `+0x44`, `+0x11A`, `+0x140`.
Confidence: HIGH — all offsets verified via `decompile_function` on two primary callers.

---

## Method

All offsets verified by:
1. `decompile_function 0x00574C20` (`MapClass::DestroyBridge_Low_OnHutDeath`) — confirmed `+0x24`, `+0x2C`, `+0x38`, `+0x44`, `+0x140`.
2. `decompile_function 0x00574000` (`MapClass::DestroyBridge_High_OnHutDeath`) — confirmed same fields, different overlay band and tile-base global.
3. `decompile_function 0x005746C0` (`MapClass::IsBridgeRampTile`) — confirmed `+0x11A` sub-tile field.

`CellClass` is accessed as `undefined *` (raw byte pointer) in the decompilation — all offsets are
direct byte offsets.

---

## Field Definitions

### `CellClass + 0x24` — Map cell coordinate (packed short×2)

**Size:** 4 bytes (two packed `short` values)
**Purpose:** The cell's own grid coordinate. Packed as `(x, y)` where `x` = low 16 bits, `y` = high 16 bits (Ghidra reads it as `(short)coord` = X and `coord._2_2_` = Y).

**Verified in `DestroyBridge_Low_OnHutDeath @ 0x00574C20`:**
```c
// Pure-bridgehead branch (flags & 0x100 == 0):
local_1c = *(short **)(puVar6 + 0x24);  // load cell coord as starting point
// ... walk perpendicular direction, incrementing local_1c with direction offsets

// bridge-cell-with-0x80 branch:
local_30 = *(short **)(puVar6 + 0x24);  // cell's own coord as anchor

// bridge-cell-without-0x80 branch (uses +0x2C):
local_30 = *(short **)(*(int *)(puVar6 + 0x2c) + 0x24);  // neighbor cell's coord
```

Same pattern confirmed in `DestroyBridge_High_OnHutDeath @ 0x00574000` — identical `+0x24` accesses.

The packed coord is decoded in the cell-array index formula:
```c
iVar9 = iVar2 * 0x200 + (int)iVar8;  // iVar2 = Y, iVar8 = X; index = Y*512 + X
```

---

### `CellClass + 0x2C` — Neighbor cell pointer (CellClass*)

**Size:** 4 bytes (pointer)
**Purpose:** Pointer to a neighbor or associated `CellClass` instance. Used in the bridge-body
walk when a cell has the 0x100 flag set but NOT the 0x80 flag — the anchor is taken from
the neighbor cell's `+0x24` coord rather than the current cell's.

**Verified in `DestroyBridge_Low_OnHutDeath @ 0x00574C20`:**
```c
else if ((uVar10 & 0x80) == 0) {
    local_30 = *(short **)(*(int *)(puVar6 + 0x2c) + 0x24);
    //                     ^^^^^^^^^^^^^^^^^
    //           read CellClass* at +0x2C, then read its +0x24 coord
}
```

Same pattern confirmed in `DestroyBridge_High_OnHutDeath @ 0x00574000`.

**Interpretation:** For bridge body cells (flag 0x100, no 0x80), `+0x2C` points to the
"bridgehead" cell (flag 0x400) that anchors this span. The walk starts from the bridgehead
coord, not the current cell coord.

---

### `CellClass + 0x38` — Tile type index (int)

**Size:** 4 bytes (`int`)
**Purpose:** Theater tile-type index. Compared against theater tile globals to identify ramp/endpoint tile types. Used in the bridge-body walk to compute a relative offset from the tile-base global.

**Verified in `DestroyBridge_Low_OnHutDeath @ 0x00574C20`:**
```c
iVar9 = *(int *)(puVar6 + 0x38) - DAT_00abad1c;
cVar1 = MapClass__IsLowBridgeEndpointTile(puVar6);
```
The subtraction `+0x38 - DAT_00abad1c` computes the tile's offset from the NS-low tile-set
base. This value is checked against `-2` after `IsLowBridgeEndpointTile` returns — a tile with
`+0x38 - DAT_00abad1c == -2` is the NS inner ramp tile (2 before the endpoint).

**Verified in `DestroyBridge_High_OnHutDeath @ 0x00574000`:**
```c
iVar9 = *(int *)(puVar6 + 0x38) - DAT_00aa0e28;
cVar1 = MapClass__IsLowBridgeEndpointTile(puVar6);
```
High-bridge path uses `DAT_00aa0e28` as the tile-set base instead.

**Also consumed by:** `MapClass::IsBridgeRampTile @ 0x005746C0` and `MapClass::IsLowBridgeEndpointTile @ 0x00574600` — both take `param_1 = tile type index` from this field (passed as the first argument when the cell is fetched).

---

### `CellClass + 0x44` — Overlay type index (int)

**Size:** 4 bytes (`int`)
**Purpose:** The overlay currently placed on the cell. Two distinct bands identify bridge types:

| Band | Condition | Bridge type |
|---|---|---|
| Low (wooden) | `0x49 < +0x44 && +0x44 < 0x66` | Low bridge inner body cell |
| High (concrete) | `0xCC < +0x44 && +0x44 < 0xE9` | High bridge inner body cell |

**Verified in `DestroyBridge_Low_OnHutDeath @ 0x00574C20`:**
```c
if ((0x49 < *(int *)(iVar2 + 0x44)) && (*(int *)(iVar2 + 0x44) < 0x66)) {
    MapClass__DestroyBridgeFromCell_Low(&param_2);
    return;
}
```

**Verified in `DestroyBridge_High_OnHutDeath @ 0x00574000`:**
```c
if ((0xcc < *(int *)(iVar2 + 0x44)) && (*(int *)(iVar2 + 0x44) < 0xe9)) {
    MapClass__DestroyBridgeFromCell_High(&param_2);
    return;
}
```

The 5×5 inner scan uses these bands to find the first bridge-body cell to start destruction from.
A cell outside both bands is not considered a bridge cell in the overlay-based pass.

---

### `CellClass + 0x11A` — Sub-tile index (signed byte)

**Size:** 1 byte (`signed char`)
**Purpose:** Which frame within the tile type's multi-frame sheet the cell uses. For bridge ramp
cells, the sub-tile value encodes the ramp face/orientation.

**Verified in `MapClass::IsBridgeRampTile @ 0x005746C0`:**
```c
if ((param_1 == DAT_00aa1548) && (*(char *)(param_2 + 0x11a) == '\f')) return 1;  // 0x0C
if ((param_1 == DAT_00aa0740) && (*(char *)(param_2 + 0x11a) == '\f')) return 1;  // 0x0C
if ((...DAT_00abad30...) && (*(char *)(param_2 + 0x11a) == '\x04')) return 1;
if ((param_1 == DAT_00abc2b4) && (*(char *)(param_2 + 0x11a) == '\b')) return 1;  // 0x08
if ((param_1 == DAT_00aa1130) && (*(char *)(param_2 + 0x11a) == '\b')) return 1;  // 0x08
if ((...DAT_00aa1028...) && (*(char *)(param_2 + 0x11a) == '\x02')) return 1;
```

| Sub-tile value | Meaning |
|---|---|
| `0x0C` (12) | Theater-A ramp |
| `0x04` (4) | Theater-B ramp (4-tile set, all 4 faces) |
| `0x08` (8) | Theater-C ramp |
| `0x02` (2) | Theater-D ramp (4-tile set, all 4 faces) |

Also used in `MapClass::IsLowBridgeEndpointTile @ 0x00574600` with expected sub-tile `0x04`
(NS endpoint) or `0x02` (EW endpoint) — see `fn-IsLowBridgeEndpointTile.md`.

---

### `CellClass + 0x140` — Bridge flags (uint32)

**Size:** 4 bytes (`uint32`)
**Purpose:** Bitmask of bridge state flags for this cell. The flags direct the bridge-walk
algorithm's branching and anchor resolution.

**Verified in `DestroyBridge_Low_OnHutDeath @ 0x00574C20` and `DestroyBridge_High_OnHutDeath @ 0x00574000`:**

```c
uVar10 = *(uint *)(puVar6 + 0x140);

if ((uVar10 & 0x500) == 0) {   // neither bridge cell nor bridgehead → walk outward
    ...
}
if (((uVar10 & 0x100) == 0) && ((uVar10 & 0x400) == 0)) {
    return;  // no useful flag → abort
}
if ((uVar10 & 0x100) == 0) {   // bridgehead-only (0x400 set): walk perpendicular
    ...
}
else if ((uVar10 & 0x80) == 0) {  // bridge body, no anchor flag: use neighbor +0x2C
    local_30 = *(short **)(*(int *)(puVar6 + 0x2c) + 0x24);
}
else {                            // bridge body with anchor flag: use self +0x24
    local_30 = *(short **)(puVar6 + 0x24);
}
uVar10 = -(uint)((*(uint *)(puVar6 + 0x140) & 0x800) != 0) & 6;  // direction: 0x800 → offset 6, else 0
```

| Flag bit | Mask | Meaning |
|---|---|---|
| `0x80` | `0x080` | Anchor flag — cell has its own coord as the walk anchor (vs. using `+0x2C` neighbor) |
| `0x100` | `0x100` | Bridge body cell |
| `0x400` | `0x400` | Bridge bridgehead / endpoint cell |
| `0x500` | `0x500` | Bridge-present composite (`0x100 \| 0x400`) — any bridge-participating cell |
| `0x800` | `0x800` | Direction discriminator — selects the walk direction offset (0 or 6 in `g_DirectionOffsets`) |

**Flag semantics:**
- `0x500 == 0`: cell is not bridge-related at all → fallback 8-direction search triggers
- `0x100` only: bridge body cell (span interior)
- `0x400` only: bridgehead cell (anchor/ramp terminal)
- `0x100 | 0x400` both: unclear from this context; the code branches on `0x100` first
- `0x80`: set on bridge body cells that self-anchor (NS vs EW orientation distinction)
- `0x800`: selects walk direction; `0x800=1` → `uVar10=6`, `0x800=0` → `uVar10=0`

---

## Summary Table

| Offset | Size | Type | Field name | Purpose |
|---|---|---|---|---|
| `+0x24` | 4 | packed short×2 | `coord` | Cell map coordinate (X low short, Y high short) |
| `+0x2C` | 4 | ptr (CellClass*) | `neighbor_cell` | Associated/neighbor cell — used as anchor source for body cells without 0x80 flag |
| `+0x38` | 4 | int | `tile_type_index` | Theater tile type index; compared against runtime tile globals for ramp/endpoint detection |
| `+0x44` | 4 | int | `overlay_type_index` | Overlay type index; bands `(0x49,0x66)` = low bridge, `(0xCC,0xE9)` = high bridge |
| `+0x11A` | 1 | signed byte | `sub_tile_index` | Tile frame index within tile type; encodes ramp orientation for ramp tile detection |
| `+0x140` | 4 | uint32 | `bridge_flags` | Bridge state bitmask: 0x80=self-anchor, 0x100=body, 0x400=bridgehead, 0x800=direction |

---

## Adjacent Fields (out of scope)

Assembly context shows `+0x2C` is accessed as a raw pointer — the field at that offset in the
bridge mechanic is the neighbor cell pointer. The field immediately before `+0x2C` at `+0x28`
is not accessed in the bridge path. The field at `+0x30` is also not used in the bridge path.

---

## Unverified (YELLOW)

- **`+0x2C` as "neighbor cell" identity:** Inferred from usage — `*(int *)(puVar6 + 0x2c)` is
  dereferenced to get a second cell pointer, then `+0x24` is read from it. The exact invariant
  (which neighbor, set during map load vs. runtime) is not traced here. Consistent with a
  "bridgehead pointer" for body cells.
- **`0x100 | 0x400` both set:** No test case in the decompiled path for this combination.
  The code branches on `0x100` first then tests `0x400` only in the `0x100 == 0` branch —
  whether `0x100 | 0x400` is a valid state is not established here.
- **`+0x140` write sites:** Flags are read here but where they are written (map load, repair
  functions) is not traced in this narrow decode. Write site analysis is out of scope.

---

## Self-Proof (exit gate)

### Claim 1: `+0x44` overlay bands `(0x49, 0x66)` for low and `(0xCC, 0xE9)` for high

`decompile_function 0x00574C20` → `if ((0x49 < *(int *)(iVar2 + 0x44)) && (*(int *)(iVar2 + 0x44) < 0x66))`.
`decompile_function 0x00574000` → `if ((0xcc < *(int *)(iVar2 + 0x44)) && (*(int *)(iVar2 + 0x44) < 0xe9))`.
Both verify `+0x44` as int-sized overlay index. **VERIFIED at both low and high bridge paths.**

### Claim 2: `+0x140` bridge-flags uint32; masks 0x80, 0x100, 0x400, 0x500, 0x800 all present

`decompile_function 0x00574C20` → `*(uint *)(puVar6 + 0x140)` read and tested with all five
mask values. Same confirmed in `decompile_function 0x00574000`. **VERIFIED from both callers.**

### Claim 3: `+0x11A` sub-tile index (signed byte) used by `IsBridgeRampTile`

`decompile_function 0x005746C0` → six branch checks each read `*(char *)(param_2 + 0x11a)` and
compare against `0x0C`, `0x04`, `0x08`, `0x02`. param_2 = CellClass*. **VERIFIED.**
