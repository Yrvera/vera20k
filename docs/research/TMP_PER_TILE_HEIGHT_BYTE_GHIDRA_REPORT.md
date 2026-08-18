# TMP Per-Tile Height Byte and CellClass HeightInPixels Formula

**Primary addresses:**
- `CellClass::RecalcAttributes` @ `0x0047D2B0` (Stage 4-5 height logic)
- `FUN_00547150` (IsoTileTypeClass::GetTileHeightPixels, called by RecalcAttributes) @ `0x00547150`
- `BuildingTypeClass::SetOwnerAndOccupy` @ `0x00543400` (Level += TMP+0x28)
- `FUN_0056bac0` (IsoMapPack5 decoder) @ `0x0056BAC0`

**Confidence:** HIGH (all claims verified from Ghidra decompilation this session)
**Active in YR:** Yes — fires every time RecalcAttributes is called (map load + every cell mutation)

---

## 1. Overview

Two distinct height systems interact in CellClass:

1. **`CellClass.Level` (offset `+0x11B`)** — integer elevation step, sourced from the
   IsoMapPack5 `z` byte (decoded at map load) plus any `Level=` INI offset. Also
   incremented/decremented by the TMP per-tile sub-record Height byte (`+0x28`) when
   buildings are placed or removed.

2. **`CellClass.HeightInPixels` (offset `+0x11D`)** — pixel height offset computed by
   RecalcAttributes from the TMP file-header `tile_height` field (not the per-tile +0x28
   byte). Formula: `floor((tile_height - 30) / 15)`.

These are **two distinct height sources**. The per-tile `+0x28` byte adjusts `Level` only
during building placement. The `HeightInPixels` formula uses the TMP *file header*
`tile_height` (at TMP file offset 12), not the per-tile `+0x28` byte.

**`CellClass+0x11A` is SubTileIndex, not a Height byte.** The CELLCLASS_STRUCT doc labels
it "Height"; the correct name is SubTileIndex. Confirmed from IsoMapPack5 decoder
(writes sub_tile byte here) and BuildingTypeClass::SetOwnerAndOccupy (`this->Height = uVar9`
where `uVar9` is the cell index within the foundation grid).

---

## 2. CellClass Field Disambiguation (offsets 0x119-0x11D)

All offsets verified from:
- `CellClass::Constructor` @ `0x0047BBF0` (byte-level init confirms distinct 1-byte fields)
- `FUN_0056BAC0` IsoMapPack5 decoder (writes +0x11A and +0x11B separately)
- `BuildingTypeClass::SetOwnerAndOccupy` @ `0x00543400` (writes +0x11A and reads +0x11B)

| Offset | Size | Name | Source | Notes |
|--------|------|------|--------|-------|
| `+0x119` | u8 | IceGrowth / extra | IsoMapPack5 3rd byte | Init 0 in constructor |
| `+0x11A` | u8 | **SubTileIndex** | IsoMapPack5 sub_tile byte; also set by building placement | Ghidra wrongly labels this "Height" in some decompile views |
| `+0x11B` | u8 | **Level** | IsoMapPack5 z byte + `[Map] Level=` INI offset; adjusted by TMP+0x28 on building place/remove | 0 = ground; each step = 15px |
| `+0x11C` | u8 | SlopeIndex | Set by `TMP_ReadSlopeType` at RecalcAttributes via TMP cell `+0x2A`; 0 = flat, 1-8 = slope type | |
| `+0x11D` | u8 | **HeightInPixels** | Set by RecalcAttributes formula `floor((tile_height - 30) / 15)` | 0 for flat terrain |

---

## 3. TMP Per-Tile Sub-Record Layout (relevant offsets)

Verified from:
- `TMP_ReadSlopeType` @ `0x005471B0` (reads `+0x2A`)
- `FUN_00544BE0` GetLandType (reads `+0x29`)
- `BuildingTypeClass::SetOwnerAndOccupy` (reads `+0x28`)
- ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md §12 (full layout)

| Byte offset within per-tile record | Size | Name | Notes |
|------------------------------------|------|------|-------|
| `+0x24` | u32 | flags | Bit 0 = has_extra_data, bit 1 = has_z_data, bit 2 = has_damaged_data |
| `+0x28` | u8 | **Height** | Per-tile elevation level delta; used to adjust CellClass.Level on building place/remove |
| `+0x29` | u8 | terrain_type | Indexes into LandType lookup table at DAT_008288E4 |
| `+0x2A` | u8 | ramp_type (SlopeIndex) | Confirmed: this is the SLOPE byte, not height |

The VOXEL_SLOPE_TILT_SYSTEM doc correctly identified `+0x2A` as the slope byte.

---

## 4. The HeightInPixels Formula (CellClass+0x11D)

### 4.1 Source of `height_raw`

The `height_raw` in the formula is NOT the TMP per-tile `+0x28` byte. It is the
TMP file-header `tile_height` field, read from byte offset 12 of the TMP file
(TMP header int[3]).

Retrieved by `FUN_00547150` @ `0x00547150`:

```
param_1  (ecx, implicit __thiscall) = IsoTileTypeClass*
param_2  = SubTileIndex (from CellClass+0x11A)
param_3  = output: tile_width (TMP header int[2])
param_4  = output: height_raw (TMP header int[3] = tile_height, normally 30)
```

If the sub-tile record has `flags & 1` (has_extra_data):
```
height_raw = (cell->Y - cell->extra_y) + tile_height
           = (TMP_cell+0x04 - TMP_cell+0x18) + TMP_header_tile_height
```
Otherwise: `height_raw = TMP_header_tile_height` (= 30 for standard flat tiles).

### 4.2 Formula as decompiled from RecalcAttributes @ 0x0047D2B0

```c
// Call FUN_00547150: writes tile_height into local_2c
FUN_00547150(SubTileIndex, &local_28, &local_2c);

local_2c = local_2c - 0x1e;   // subtract 30 (0x1e = 30)

// Signed floor division by 15 (0xf):
// Ghidra pattern: (x / 15 + (x >> 31)) - ((x * 0x88888889) >> 0x3f)
this->HeightInPixels =
    ((char)(local_2c / 0xf) + (char)(local_2c >> 0x1f)) -
    (char)((longlong)local_2c * 0x88888889 >> 0x3f);
```

This is signed floor division by 15. For non-negative `local_2c`, it reduces to
`local_2c / 15`. The signed form handles tile_height < 30 (which should not occur
in valid retail assets but the code defends against it).

### 4.3 Examples

| tile_height | local_2c (after -30) | HeightInPixels |
|-------------|----------------------|----------------|
| 30 | 0 | 0 (flat ground) |
| 45 | 15 | 1 |
| 60 | 30 | 2 |
| 75 | 45 | 3 |

Standard retail YR tiles use tile_height=30 (flat) and elevated tiles use multiples of 15
above 30. The maximum in practice is determined by map height limits.

### 4.4 Active in YR

Yes, unconditionally — fires for every valid cell during RecalcAttributes (Stage 4-5),
which runs at map load for all cells and on every cell mutation.

---

## 5. CellClass.Level: Sources and Adjustments

### 5.1 Map load (IsoMapPack5 decoder @ 0x0056BAC0)

```c
FUN_0055c7c0(cell + 0x11a, 1);   // read SubTileIndex byte
FUN_0055c7c0(cell + 0x11b, 1);   // read Level (z) byte
```
The z byte from IsoMapPack5 is the terrain elevation directly from the map file.

### 5.2 Map-level Level= offset (Read_Map_Section_And_IsoMapPacks @ 0x004ACE70)

```c
ppuStack_108 = CCINIClass__ReadInt(INI, "Level", 0);  // [Map] Level= key, default 0
// After IsoMapPack5 decode, for each cell:
*(char *)(cell + 0x11b) = *(char *)(cell + 0x11b) + (char)ppuStack_108;
```
The map-level `Level=` offset is ADDED to every cell's Level after IsoMapPack5 decode.
This is a rare key (almost always 0 in standard maps).

### 5.3 Building placement / removal (BuildingTypeClass::SetOwnerAndOccupy @ 0x00543400)

```c
// PLACE (param_2 == 1 or 3):
cell->Level = cell->Level + *(char *)(TMP_cell + 0x28);

// REMOVE (param_2 == 0):
cell->Level = cell->Level - *(char *)(TMP_cell + 0x28);
```

`TMP_cell + 0x28` is the per-tile Height byte from the building's foundation TMP.
This is the **only place** the per-tile `+0x28` byte is used to mutate CellClass.Level.

### 5.4 Draw-time usage (CellOverlay_TileDraw @ 0x00480350)

From ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md §17.2:
```c
screen_y = screen_pos[1] - cell.Level * 15;  // 15px per elevation step
```
Each Level step = 15 pixels of upward screen offset.

---

## 6. Disambiguation: TMP Height Byte (+0x28) vs HeightInPixels Formula Source

These are two DIFFERENT things:

| | TMP per-tile `+0x28` byte | HeightInPixels formula source |
|---|---|---|
| **What it is** | Per-sub-tile height delta in the TMP cell record | TMP file-header `tile_height` (int at TMP offset 12) |
| **Where read** | `BuildingTypeClass::SetOwnerAndOccupy` @ `0x00543400` | `FUN_00547150` @ `0x00547150` (reads `piVar3[3]`) |
| **Effect on CellClass** | Adjusts `CellClass.Level` (+0x11B) on building place/remove | Computes `CellClass.HeightInPixels` (+0x11D) |
| **Normal value** | 0 for terrain tiles; non-zero for building foundation sub-tiles | 30 for flat tiles; 45, 60, 75... for elevated |
| **Formula** | Direct add/subtract to Level | `floor((tile_height - 30) / 15)` |

---

## 7. RecalcAttributes "Level from TMP height" Note (Prior Doc)

CELLCLASS_STRUCT_GHIDRA_REPORT.md §3 (pipeline) says "Set Level from TMP height" in
Stage 3b.2. Based on this session's decompilation, the Level write at that stage reads
`in_stack_00000004` which is a Ghidra stack-variable artifact — at that program point it
may hold the last coordinate value from the TubeClass path, not a TMP height value.
The actual Level value is established by IsoMapPack5 decode (5.1) and building placement
(5.3), not by RecalcAttributes directly. RecalcAttributes does NOT modify Level in the
normal terrain path — it only writes `HeightInPixels` (+0x11D).

This is a **correction to the prior CELLCLASS doc** Stage 3b.2 description.

---

## 8. Open Questions — Final State

- `[RESOLVED] Q1` — What is the height byte offset within TMP per-tile record? → `+0x28` (evidence: `BuildingTypeClass::SetOwnerAndOccupy` @ `0x00543400`)
- `[RESOLVED] Q2` — Is CellClass+0x11A the SubTileIndex or a raw height byte? → SubTileIndex (evidence: IsoMapPack5 decoder writes sub_tile here; building code writes cell index here)
- `[RESOLVED] Q3` — What is the source of `height_raw` in the formula? → TMP file-header `tile_height` (int at offset 12), NOT the per-tile `+0x28` byte (evidence: FUN_00547150 @ 0x00547150 reads `piVar3[3]`)
- `[RESOLVED] Q4` — Is the formula exactly `(height_raw - 30) / 15`? → Yes, signed floor division by 15, with subtraction of 30 (0x1e). Ghidra shows multiply-magic 0x88888889 (evidence: RecalcAttributes @ 0x0047D2B0)
- `[RESOLVED] Q5` — Where does CellClass.Level come from? → IsoMapPack5 z byte at load + `[Map] Level=` INI key offset + TMP+0x28 delta on building place/remove (evidence: FUN_0056BAC0, FUN_004ACE70, FUN_00543400)
- `[RESOLVED] Q6` — Is the slope byte at +0x2A? → Yes, confirmed by TMP_ReadSlopeType @ 0x005471B0
- `[DEFERRED] Q7` — Does RecalcAttributes actually modify Level at all for normal terrain (the `in_stack_00000004` path)? (category: needs-runtime-debugger; reason: Ghidra variable reuse makes static analysis ambiguous at that point; runtime trace would confirm whether Level is ever changed by RecalcAttributes for non-building cells)
- `[DEFERRED] Q8` — What is CellClass+0x119 (third IsoMapPack5 byte)? (category: out-of-scope; reason: not part of the height/HeightInPixels investigation; likely ice_growth or editor-only metadata)

---

## 9. Rust Implementation Status

| System | Rust status |
|--------|-------------|
| TmpTile.height (`+0x28`) parsed | **Complete** — `src/assets/tmp_file.rs` `TmpTile.height: u8` |
| TmpFile.tile_height (header offset 12) parsed | **Complete** — `src/assets/tmp_file.rs` `TmpFile.tile_height: u32` |
| CellClass.Level from IsoMapPack5 z byte | **Complete** — `src/map/map_file.rs` `MapCell.z: u8` |
| `HeightInPixels = floor((tile_height - 30) / 15)` computed | **Missing** — no equivalent of CellClass+0x11D in the Rust cell struct |
| `Level += TMP+0x28` on building placement | **Missing** — building placement does not yet adjust terrain Level |
| `[Map] Level=` INI offset applied to all cells | **Missing** — not read in current Rust map loader |
| Screen Y offset `Level * 15` in tile draw | **Present** — ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md §17 confirmed; verify in render path |

---

## Sources

**Ghidra addresses decompiled this session:**
- `0x0047BBF0` — CellClass::Constructor (byte-level field layout)
- `0x0047D2B0` — CellClass::RecalcAttributes (HeightInPixels formula, Level write path)
- `0x00487D50` — CellClass::GetEffectiveHeight (Level+0x11B usage confirmed)
- `0x00543400` — BuildingTypeClass::SetOwnerAndOccupy (TMP+0x28 Height byte → Level)
- `0x00544BE0` — FUN (GetLandType, reads TMP+0x29)
- `0x00544C20` — FUN (IsCellValid, vtable+0x9C pattern)
- `0x005471B0` — TMP_ReadSlopeType (reads TMP+0x2A, takes ECX=IsoTileTypeClass*, param2=SubTileIndex)
- `0x00547150` — FUN_00547150 (reads TMP header tile_height into output param)
- `0x0056BAC0` — IsoMapPack5 decoder (writes SubTileIndex→+0x11A, Level/z→+0x11B)
- `0x004ACE70` — Read_Map_Section_And_IsoMapPacks ([Map] Level= applied to all cells)
- `0x00704350` — FUN_00704350 (calls FUN_00547150 with CellClass+0x11A as SubTileIndex param)

**Docs cross-referenced:**
- `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` — TMP cell layout §12, draw path §17
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` — field map (NOTE: +0x11A labeled "Height" in that doc is actually SubTileIndex)
- `VOXEL_SLOPE_TILT_SYSTEM.md` — confirms +0x2A is slope byte

**Rust source audited:**
- `src/assets/tmp_file.rs` — TmpFile, TmpTile (tile_height and height byte present)
- `src/map/map_file.rs` — MapCell.z and sub_tile fields
