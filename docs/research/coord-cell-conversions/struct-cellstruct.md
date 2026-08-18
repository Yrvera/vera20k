# CellStruct — Decode Doc

## Summary

`CellStruct` is a 4-byte packed cell coordinate used throughout gamemd.exe to
identify a single cell on the map. It is NOT a named Ghidra struct — Ghidra has
no `CellStruct` type definition. In the binary it is passed as a raw 32-bit value
(often in a register or via `undefined4*`) or as two consecutive `short` fields
accessed via `short*`. Layout:

```
Offset 0 (low  16 bits): cell_X — signed short, column index (+X = east)
Offset 2 (high 16 bits): cell_Y — signed short, row index    (+Y = south)
```

Packed form: `CONCAT22(cell_Y, cell_X)` in Ghidra notation = `(cell_Y << 16) | (cell_X & 0xFFFF)`.

The 4-byte packing allows the entire coordinate to be passed in a single register
and compared/stored as a 32-bit integer. This is the **Frame #2** coordinate type
in the CLAUDE.md reference-frame table: "Get_Cell_Packed (NW cell)".

(verified from six function decompilations across decoded tasks #1, #4, #10, and #15,
and `decompile_function 0x005657a0`)

## Active in YR

**Yes.** Used pervasively across all game systems: pathfinding, zone lookup,
cell-class access, object placement, and coordinate conversion. Every function
that converts a lepton-space position to a cell uses this packed format.

## Field layout

| Offset (bytes) | Size | Type | Name | Semantics |
|---|---|---|---|---|
| 0 | 2 | signed short | cell_X | Column index; +X = east; range 0–511 in normal maps |
| 2 | 2 | signed short | cell_Y | Row index; +Y = south; range 0–511 in normal maps |

**Total size: 4 bytes.**

No Ghidra struct definition exists — layout is inferred entirely from decompilation
evidence cited below.
(verified via `get_struct_layout("CellStruct")` → "Structure not found")

### Access patterns

Ghidra consistently represents the packed CellStruct in two forms:

**Form A — undefined4 (register/pointer):**
The full 4-byte value is passed as `undefined4` or `undefined4*`. Reading back:
```rust
let x = (packed & 0x0000_FFFF) as i16;
let y = (packed >> 16) as i16;
```

**Form B — short* (two-field array):**
The same 4 bytes are accessed via `short*` as `param[0]` (cell_X) and `param[1]` (cell_Y).

Both forms are used interchangeably across different callers of the same functions.

## Behavioral analysis

### Signed vs unsigned

`cell_X` and `cell_Y` are **signed shorts** (`i16`). Evidence:
- `ObjectClass__Get_Cell_Packed` uses the sign-correct arithmetic shift
  `(v + (v >> 31 & 0xFF)) >> 8` to produce the cell index — this produces signed
  values for negative lepton inputs. The result is cast to `(short)` in the decompilation.
  (verified via `decompile_function 0x0041bea0`)
- `MapCoord_Step_By_Direction` adds signed `short` deltas to `short` cell fields.
  (verified via `decompile_function 0x0042d490`)
- `MapClass__Get_CellClass` treats cell_X and cell_Y as signed (`short*` accesses)
  and checks `iVar1 < 0` as the out-of-map sentinel.
  (verified via `decompile_function 0x005657a0`)

Value -1 (= 0xFFFF as unsigned short) appears as a sentinel for "no tube" in
`CellClass+0x116` (task #10). In packed CellStruct context, negative cell_X or
cell_Y values indicate off-map or invalid cells.

### Map cell address space

`MapClass__Get_CellClass` (0x005657a0) computes the CellClass array index as:
```c
iVar1 = param_2[1] * 0x200 + (int)*param_2;   // cell_Y * 512 + cell_X
```
Maximum valid index: 0x3ffff = 262143 = 512 × 511. This establishes:
- Map cell array has **512 columns** (cell_X range 0–511)
- Maximum 511 rows before the 0x3ffff bound (cell_Y range 0–511)
- The CellClass lookup array at `MapClass+0x13c` is a 512×512 = 262,144-entry pointer array

The ZoneMap uses a different formula (`f8+1+f4` stride, task #4) — that is a
smaller sub-array for zone pathfinding data, not the main cell-class table.
(verified via `decompile_function 0x005657a0`)

### Coordinate reference frame

- **Frame #2** (CLAUDE.md): "Get_Cell_Packed (NW cell)". For buildings, the
  cell index points to the NW corner of the foundation. For mobile objects,
  it is the cell containing the object's current lepton position.
- **Conversion from leptons (Frame #1)**: `cell = (lepton + (lepton >> 31 & 0xFF)) >> 8`
  (sign-correct arithmetic shift). See `ObjectClass__Get_Cell_Packed` doc.
- **Conversion to leptons**: `lepton = cell * 256` (for NW cell corner);
  `lepton = cell * 256 + 128` for cell center.
- **Rust canonical form**: `(u16, u16)` for valid map cells (0–511 range),
  or `(i16, i16)` where negative values are off-map sentinels.

### CONCAT22 — Ghidra notation

`CONCAT22(cell_Y, cell_X)` in Ghidra pseudocode produces a 32-bit value:
- High 16 bits = first argument (cell_Y)
- Low 16 bits = second argument (cell_X)

```rust
fn pack_cell(x: i16, y: i16) -> u32 {
    ((y as u32) << 16) | (x as u16 as u32)
}

fn unpack_cell(packed: u32) -> (i16, i16) {
    let x = packed as i16;
    let y = (packed >> 16) as i16;
    (x, y)
}
```

This packing is used in:
- `ObjectClass__Get_Cell_Packed` (0x0041bea0): produces CONCAT22(cell_Y, cell_X)
- `MapCoord_Step_By_Direction` (0x0042d490): reads and writes the packed form
- `MapClass__CellCoordToLinearIndex` (0x0056d430): receives the packed form as `short*`
(all verified in prior decode tasks)

### Function-level evidence (cross-decoded)

| Source function | Address | Evidence | Task |
|---|---|---|---|
| `ObjectClass__Get_Cell_Packed` | 0x0041bea0 | Produces CONCAT22(cell_Y, cell_X); (short) cast of each component | #1 |
| `MapClass__CellCoordToLinearIndex` | 0x0056d430 | Receives as `short*`; `*param_2`=cell_X, `param_2[1]`=cell_Y | #4 |
| `MapCoord_Step_By_Direction` | 0x0042d490 | Receives as `short*`; adds delta shorts; outputs CONCAT22 | #10 |
| `BuildingClass__GetDockCoord` | 0x00447b20 | Reads packed result of vtable+0x1B8 as `short*[0]`=X, `[1]`=Y | #17 |
| `MapClass__Get_CellClass` | 0x005657a0 | Receives as `short*`; `*param_2`=cell_X, `param_2[1]`=cell_Y | this task |

(verified via `decompile_function 0x0041bea0`, `decompile_function 0x0056d430`,
`decompile_function 0x0042d490`, `decompile_function 0x00447b20`,
`decompile_function 0x005657a0`)

### CellClass own-coordinates vs CellStruct

The `CellClass` struct (Ghidra: 328 bytes) has its own `MapCoord_X` at offset 0x24
and `MapCoord_Y` at offset 0x26. These are the cell's self-referential coordinates
within the CellClass instance — NOT a CellStruct field. CellStruct is the caller-
side encoding; `CellClass.MapCoord_X/Y` is how the CellClass identifies its own
position.
(verified via `get_struct_layout("CellClass")`)

### Rust implementation guidance

```rust
/// Packed cell coordinate: low 16 = cell_X (east), high 16 = cell_Y (south).
/// Negative i16 values are off-map sentinels.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CellCoord {
    pub x: i16,
    pub y: i16,
}

impl CellCoord {
    pub fn from_packed(v: u32) -> Self {
        Self { x: v as i16, y: (v >> 16) as i16 }
    }
    pub fn to_packed(self) -> u32 {
        ((self.y as u32) << 16) | (self.x as u16 as u32)
    }
    pub fn is_valid(self) -> bool {
        self.x >= 0 && self.y >= 0 && self.x < 512 && self.y < 512
    }
}
```

The 512-wide array bounds come from `MapClass__Get_CellClass`'s index formula.

## Usage patterns

| Pattern | Description |
|---|---|
| `CONCAT22(cell_Y, cell_X)` | Construction via two shorts |
| `(short*)[0]` = cell_X, `[1]` = cell_Y | Access via short* array |
| `undefined4*` = packed 32-bit value | Storage/pass-by-pointer as a dword |
| `cell_Y * 0x200 + cell_X` | Linear CellClass array index (stride 512) |
| `(f8+1+f4) * cell_Y + cell_X` | ZoneMap linear index (smaller stride) |

## Out-of-scope refs

- `CellClass` struct fields beyond `MapCoord_X/Y` — full CellClass decode is out of scope
- `MapClass+0x13c` — the CellClass pointer array; its initialization is out of scope
- Zone map stride formula — documented in `fn-cell-coord-to-linear.md`

## Unverified claims (YELLOW)

**UNVERIFIED**: Whether cell_X or cell_Y can validly be negative in normal
gameplay (beyond the -1 tube sentinel). The sign-correct arithmetic shift in
`ObjectClass__Get_Cell_Packed` handles negative leptons, but standard YR maps
place the playfield at positive cell coordinates. Negative cell values would
require an object at negative lepton coordinates — presumably off-map. The exact
range of valid cell coordinates in a standard YR map was not confirmed from the
map-load code in this session.

**UNVERIFIED**: The 512 column count applies to the CellClass pointer array;
the actual playable map may be smaller (configurable per scenario). The 0x3ffff
bound in `MapClass__Get_CellClass` is the hard array limit, not the playfield size.
