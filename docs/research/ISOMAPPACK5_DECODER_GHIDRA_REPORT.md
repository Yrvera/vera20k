# IsoMapPack5 Decoder — Ghidra Research Report

**Address:** `0x0056BAC0` (`FUN_0056bac0`)
**Confidence:** HIGH — decompiled directly; all offsets verified from raw bytes or
cross-matched with already-verified CELLCLASS_STRUCT / MAPCLASS reports.
**Active in YR:** Yes — called unconditionally from `Read_Map_Section_And_IsoMapPacks`
on every map load (skirmish, campaign, multiplayer). No flag gate. Single call site.

---

## 1. Overview

`FUN_0056bac0` decodes the `[IsoMapPack5]` section of a `.map` file.
The input is a base64-decoded, LZO-chunked byte stream.
For each cell record it writes three fields to the corresponding `CellClass` instance:
`IsoTileTypeIndex` (+0x38), `sub_tile_idx` (+0x11A), and `Level` (+0x11B).
A fourth byte (+0x119) receives the 11th record byte (IceGrowth — unused in RA2/YR).

Call chain to confirm live-in-YR path (all verified):
```
ScenarioClass__Full_Init @ 0x00686B20
  → Read_Map_Section_And_IsoMapPacks @ 0x004ACE70
    → FUN_0056bac0 @ 0x0056BAC0   (single xref: UNCONDITIONAL_CALL at 0x004AD6E6)
```

---

## 2. Input Format — [IsoMapPack5] byte layout

The `.map` file section is base64-encoded. After decoding, the binary is a
sequence of LZO1X1 chunks. Each chunk header (verified from `LZOStraw__Constructor`):
```
uint16  uncompressed_size
uint16  compressed_size   (if == uncompressed_size: uncompressed literal block)
```
Chunk size constant: `0x2000` (8192 bytes).

After LZO decompression, the stream is a sequence of **11-byte cell records**
terminated by a **4-byte sentinel** (`0x00000000`, verified by `read_memory` at
`DAT_00ABD480`).

### Per-cell record layout (verified from decompile + ASSET_PARSING report §6.2)

| Bytes | Type   | Content                          | Written to CellClass field |
|-------|--------|----------------------------------|---------------------------|
| 0–1   | u16 LE | X coordinate (signed as i16)     | (used for cell lookup)     |
| 2–3   | u16 LE | Y coordinate (used as unsigned)  | (used for cell lookup)     |
| 4–7   | u32 LE | Raw tile index (pre-validation)  | `+0x38` (IsoTileTypeIndex) |
| 8     | u8     | Sub-tile index within IsoTileType| `+0x11A` (sub_tile_idx)    |
| 9     | u8     | Elevation level (0 = ground)     | `+0x11B` (Level)           |
| 10    | u8     | IceGrowth (TS Snow only; 0 in YR)| `+0x119` (Unknown_0x119)   |

Total record size: **11 bytes** (verified).

---

## 3. Core Logic — Annotated Pseudocode

```
// param_1 = MapClass* (this, passed in ECX per __thiscall; confirmed by
//            MOV EBX,ECX at 0x0056BAC2 and MOV EDX,[EBX+0x13C] later)
// param_2 = straw ptr (BufferStraw wrapping LZO-decompressed data)
FUN_0056bac0(MapClass* this, Straw* straw):

  LZOStraw__Constructor(1, 0x2000)       // init internal LZO straw; chunk size = 8192
  FUN_006c9890(straw)                     // link straw chain

  loop:
    read 4 bytes from straw → header      // reads raw X (i16) + Y (u16) word pair

    // Sentinel check — DAT_00ABD480 = 0x00000000 (verified by read_memory)
    if header.low_i16 == 0 AND header.high_u16 == 0:
      break

    // Cell index arithmetic — NOTE: X is sign-extended (i16), Y is unsigned (u16)
    cell_idx = (i32)(i16)header.low + (u32)header.high * 0x200
    //        = signed_X + Y * 512
    //        = Y * 512 + signed_X  (same formula, Y contribution dominates)

    // Bounds check: [0, 0x3FFFF] = [0, 262143]
    if cell_idx < 0 OR cell_idx > 0x3FFFF:
      // Out-of-bounds cell: stash header in DAT_00ABDC74, point to dummy cell
      DAT_00ABDC74 = header
      cell_ptr = &DAT_00ABDC50              // dummy/sentinel CellClass

    else:
      // Lookup cell pointer from MapClass cell_array
      // MapClass+0x138 = VectorClass<CellClass*>
      // MapClass+0x13C = VectorClass data pointer (cell_array[262144])
      cell_array = *(int*)(this + 0x13C)    // verified: cell_array data ptr
      cell_ptr = cell_array[cell_idx]       // cell_array[Y*512 + X]
      if cell_ptr == NULL:
        DAT_00ABDC74 = header
        cell_ptr = &DAT_00ABDC50            // redirect to dummy cell

    // Check if we're writing to the dummy cell (skip payload but CONSUME 7 bytes)
    if cell_ptr == NULL OR cell_ptr == &DAT_00ABDC50:
      skip 7 bytes from straw              // consume tile(4) + sub_tile(1) + level(1) + ice(1)
      restore header sentinel in local_34

    else:
      // Write tile index
      cell[+0x38] = 0xFFFF                 // pre-clear to "no tile" sentinel
      read 4 bytes from straw → raw_tile   // d[4..7]
      validated_tile = FUN_00544E30(raw_tile)  // tile-index fixup (see §4)
      cell[+0x38] = validated_tile          // IsoTileTypeIndex

      // Write sub-tile and level
      read 1 byte from straw → cell[+0x11A]  // d[8] = sub_tile_idx
      read 1 byte from straw → cell[+0x11B]  // d[9] = Level (elevation)

      // Write IceGrowth byte (last byte of 11-byte record)
      // IMPORTANT: puVar4 is reassigned to cell+0x119 before the read
      cell_ptr += 0x119                     // advance pointer to cell+0x119
      read 1 byte from straw → *cell_ptr    // d[10] → cell[+0x119]

    reset local_34 = DAT_00ABD480           // restore sentinel ref for next iteration

  // Cleanup
  CellClass__Constructor()                  // (appears to be destructor/cleanup)
  LZOStraw__Constructor()                   // (cleanup)
  return 1
```

---

## 4. Tile Index Validation — FUN_00544E30

**Address:** `0x00544E30`
**Confidence:** HIGH (decompiled, verified)

This function remaps raw tile indices from the map file into validated indices
by walking a sorted tileset-boundary table.

```c
int FUN_00544e30(int raw_tile_index):
  if raw_tile_index == 0xFFFF:
    return 0xFFFF     // pass-through: "no tile" sentinel unchanged

  // Walk the tileset boundary array (sorted ascending by boundary)
  iVar1 = raw_tile_index   // running accumulation of index
  piVar3 = DAT_00aa107c    // array of (boundary, count) int pairs
  for i in 0..DAT_00aa1088:
    if raw_tile_index < *piVar3:   // raw_tile_index < this boundary
      return iVar1                 // index valid within this tileset range
    iVar1 += piVar3[1]             // add offset/count
    piVar3++
  return iVar1   // fallthrough: index in last range
```

**Purpose:** RA2 map files use tile indices relative to the tileset as exported
by the editor. The engine's global tile array may have gaps (unloaded tilesets
skipped). `FUN_00544E30` converts from the flat editor index to the engine's
sparse index by adding offsets at each tileset boundary.

**Special case:** `0xFFFF` passes through unchanged. The cell is pre-cleared
to `0xFFFF` before the straw read (`*puVar1 = 0xffff`), ensuring that if
`FUN_00544E30` returns `0xFFFF`, the cell keeps the "no tile" state.

---

## 5. Key Field Semantics After Decode

| CellClass Offset | Name in Ghidra struct | Written by decoder? | Value written |
|------------------|-----------------------|---------------------|---------------|
| `+0x38`  | `IsoTileTypeIndex` | Yes | d[4..7] run through FUN_00544E30; 0xFFFF = "no tile" |
| `+0x11A` | `Height` (Ghidra name; semantic = sub_tile_idx) | Yes | d[8] = sub-tile slot within IsoTileType (0-based) |
| `+0x11B` | `Level` | Yes | d[9] = elevation level (each = 15 px world Z) |
| `+0x119` | `Unknown_0x119` | Yes | d[10] = IceGrowth byte (TS Snow only; 0 in all RA2/YR maps) |
| `+0x11C` | `SlopeIndex` | No — NOT touched by decoder | Set later by `RecalcAttributes` via TMP_ReadSlopeType |

**CRITICAL DETAIL — +0x11A naming conflict resolved:**
The Ghidra struct labels `+0x11A` as "Height" (TS-era name) but this is misleading.
Per `CELL_0x11A_POLARITY_RECONCILE_GHIDRA_REPORT.md` (2026-05-18, HIGH confidence):
`+0x11A` is the **sub-tile index** (icon index within the IsoTileType TMP).
`+0x11B` is the true **Level** (Z elevation).

---

## 6. Pre-Fill Context (What Exists Before Decoder Runs)

Before `FUN_0056bac0` is called, `Read_Map_Section_And_IsoMapPacks` fills
every cell in the playfield with a default tile (verified from decompile):

```c
// In Read_Map_Section_And_IsoMapPacks, the MapClass cell-iterator loop:
while (cell = MapClass__CellIterator_Next()) {
    tile_variant = Random__RandomRanged(0, uStack_110)   // 0 for Clear fill, 0..3 for Water fill
    cell[+0x38] = tile_variant + uStack_10c              // IsoTileTypeIndex = ClearTile or WaterTile
    cell[+0x11B] += (char)ppuStack_108                   // Level += [Map] Level= baseline offset
    cell[+0x11A] = 0                                     // sub_tile_idx = 0 (always)
}
```

IsoMapPack5 then **overwrites** these values per cell. Cells absent from
IsoMapPack5 keep the pre-fill values (rare for retail maps — editors typically
emit every cell).

**IMPORTANT:** `[Map] Level=` is added to the cell's existing Level (which is 0)
via **addition** (`+= baseline`), then IsoMapPack5 writes Level as an absolute
value (no addition — just a direct write via straw read). This means the
baseline offset does NOT apply to IsoMapPack5 cells. Only the pre-fill cells
that are NOT overwritten by IsoMapPack5 carry the baseline offset.

---

## 7. Bounds and Sentinel Details

| Item | Value | Evidence |
|------|-------|----------|
| Sentinel at `DAT_00ABD480` | `0x00000000` | `read_memory` at `0x00ABD480` → `[0,0,0,0]` |
| Sentinel check | Both halves must equal both halves of `DAT_00ABD480` (two separate short comparisons) | Decompile: `(short)local_34 == (short)DAT_00abd480 && local_34._2_2_ == DAT_00abd480._2_2_` |
| Cell index bounds | `[0, 0x3FFFF]` = `[0, 262143]` inclusive | `iVar2 < 0 || 0x3ffff < iVar2` → skip |
| Cell array capacity | 262144 (`0x40000`) slots — full 512×512 grid | `MAPCLASS_COMPLETE_DECODE.md` §3 / `MAPCLASS_GHIDRA_REPORT.md` |
| "No tile" sentinel | `0xFFFF` (written to cell+0x38 before straw read, preserved if validation returns it) | Decompile: `*puVar1 = 0xffff` then validated write |
| Dummy cell (OOB/null) | `&DAT_00ABDC50` (global static dummy) | Decompile; OOB cells redirect here, 7 bytes still consumed |

**TINY DETAIL — OOB cells still consume 7 bytes:** When a cell is out of
bounds (`cell_idx < 0 || > 0x3FFFF`) or its pointer is NULL, the decoder
does NOT skip 11 bytes — it consumed 4 bytes for the header already, then
skips 7 more (`puVar4 = local_30; uVar3 = 7; FUN_0055c7c0(puVar4, uVar3)`).
Total = 4 + 7 = 11 bytes consumed per out-of-bounds record.

**TINY DETAIL — X coordinate is sign-extended (i16), Y is unsigned (u16):**
`iVar2 = local_34._2_2_ * 0x200 + (int)(short)local_34`
= `Y_u16 * 512 + (i32)(i16)X`. Negative X would produce a negative cell_idx,
which is caught by the `< 0` bounds check. In practice, map coordinates are
always non-negative, so this edge case only fires for corrupt or editor-
generated cells with X = 0xFFFF etc.

**TINY DETAIL — sub_tile read ordering:** The stream bytes are read as:
d[8] → `cell+0x11A` first, then d[9] → `cell+0x11B`, then `puVar4 += 0x119`
and d[10] → `*puVar4` = `cell+0x119`. This is NOT three adjacent byte reads
into adjacent offsets — the `+= 0x119` pointer arithmetic for the last byte
is a Ghidra decompiler artifact; the real effect is simply: last byte → `+0x119`.

---

## 8. What the Decoder Does NOT Write

The decoder does NOT touch:

- `+0x11C` (SlopeIndex) — set by `RecalcAttributes` after map load
- `+0x11D` (HeightInPixels) — computed by `RecalcAttributes`
- `+0x11E` (OverlayData / bridge damage state) — set by `ReadMapOverlayPacks`
- `+0x44` (OverlayTypeIndex) — set by `ReadMapOverlayPacks`
- `+0x140` (Flags) — not modified by decoder
- `+0x124` (OccupationFlags) — not modified

The execution order in `ScenarioClass__Full_Init` is:
1. `Read_Map_Section_And_IsoMapPacks` (pre-fill + IsoMapPack5 decode) — this report
2. `ReadMapOverlayPacks` (overlay + bridge overlay decode)
3. `CellClass__RecalcAttributes` per cell (derives SlopeIndex, LandType, HeightInPixels)

---

## 9. IceGrowth Byte — Active in YR?

The 11th byte (`d[10]`) is written to `cell + 0x119`. In all RA2 and YR maps,
this byte is `0x00`. It was used in Tiberian Sun Snow theater maps for ice growth.

**Active in YR:** No for gameplay effect. The byte is still written (always 0),
and the field exists in the struct, but no YR code path reads `+0x119` for
any behavior that matters in a standard YR skirmish.
Evidence: CELLCLASS_STRUCT report labels `+0x119` as `Unknown_0x119` (init = 0,
no HIGH-confidence read site found). ASSET_PARSING_BRIDGES §6.2 confirms
`terrain_type` in the record comment, which maps to the same conclusion.

---

## 10. Caller Verification — Active in YR

**Single call site:** `Read_Map_Section_And_IsoMapPacks @ 0x004ACE70`
(xref confirmed: `UNCONDITIONAL_CALL` at `0x004AD6E6`).

**Caller's caller:** `ScenarioClass__Full_Init @ 0x00686B20`
(confirmed by decompilation — calls `Read_Map_Section_And_IsoMapPacks(param_1)`
unconditionally on the standard map-load path for all game modes including
multiplayer skirmish). No TS flag gates observed.

**IsoMapPack5 is tried last** (after IsoMapPack1–4 LCW variants):
All five variants are attempted sequentially; in all YR skirmish maps only
`[IsoMapPack5]` is populated (LZO-compressed). The older variants are TS/legacy.

---

## 11. Rust Implementation Status

File: `c:/Users/enok/Documents/ra2-rust-game/src/map/map_file.rs`

| Aspect | gamemd.exe behavior | Rust status |
|--------|---------------------|-------------|
| Record size | 11 bytes | Correct (`CELL_RECORD_SIZE = 11`) |
| d[0..1] X field | u16 LE, used as signed i16 for cell lookup | Rust reads as `u16`, uses `rx` directly — **does not sign-extend X** |
| d[2..3] Y field | u16 LE, used as unsigned | Correct |
| d[4..7] tile_index | i32 LE, passed to FUN_00544E30 for validation | Rust reads `i32` but does NOT call tile-index fixup (`FUN_00544E30`) |
| d[8] sub_tile | u8 → `cell+0x11A` | Correct (`pub sub_tile: u8`) |
| d[9] level (z) | u8 → `cell+0x11B` | Correct (`pub z: u8`) |
| d[10] IceGrowth | u8 → `cell+0x119`, always 0 in YR | Parsed but discarded (`// d[10] = ice_growth`) — no parity issue |
| Termination sentinel | `0x00000000` (both shorts = 0) | Rust checks `rx == 0 && ry == 0` — equivalent but slightly different: gamemd compares each half to `DAT_00ABD480` (which is 0x00000000), Rust directly checks `rx == 0 && ry == 0`. Semantically identical given sentinel = 0. |
| Tile index fixup | FUN_00544E30 remaps indices across tileset boundaries | **MISSING** — Rust uses raw tile_index without remapping |
| OOB bounds check | `[0, 0x3FFFF]` inclusive, skip cell but consume 7 bytes | Rust skips invalid rx/ry but does not enforce the 0x3FFFF bound or the "consume bytes" behavior (irrelevant at Rust layer since data is pre-decoded) |
| `[Map] Level=` interaction | IsoMapPack5 Level is an absolute write; pre-fill cells get the baseline offset added but IsoMapPack5 cells do not | Rust applies IsoMapPack5 z directly — correct for IsoMapPack5 cells; the `[Map] Level=` pre-fill baseline is **missing** (confirmed by SEA_TILES report §5) |

**Most significant gap:** Tile-index fixup via `FUN_00544E30` is missing. For
maps where the tileset composition differs from the editor's flat index (any
theater with unused tilesets), tiles would render incorrect graphics.

---

## 12. Open Questions — Final State

- `[RESOLVED] OQ-1 — What is the sentinel value?`
  → `0x00000000` (evidence: `read_memory @ DAT_00ABD480` = `00 00 00 00`)

- `[RESOLVED] OQ-2 — Is the sentinel check one or two comparisons?`
  → Two separate `short` comparisons (low word and high word separately checked
  against `DAT_00ABD480`). Functionally a single 4-byte equality check since
  the sentinel is 0x00000000. (evidence: decompile of FUN_0056bac0)

- `[RESOLVED] OQ-3 — What is param_1?`
  → MapClass* (`this` in ECX per `__thiscall`; confirmed by `MOV EBX,ECX` at
  0x0056BAC2 and `MOV EDX,[EBX+0x13C]` for cell_array access)

- `[RESOLVED] OQ-4 — What is param_1 + 0x13C?`
  → `VectorClass<CellClass*>` data pointer = `cell_array[262144]`
  (evidence: `MAPCLASS_GHIDRA_REPORT.md` §2 / MAPCLASS_COMPLETE_DECODE §3)

- `[RESOLVED] OQ-5 — What goes into cell+0x119 (d[10])?`
  → IceGrowth byte (TS Snow only, always 0 in YR). Written via pointer
  arithmetic: `puVar4 += 0x119; FUN_0055c7c0(puVar4, 1)` (evidence: decompile)

- `[RESOLVED] OQ-6 — Does FUN_00544E30 clamp the tile index?`
  → No. It remaps across tileset boundary offsets. Only `0xFFFF` is a
  special case (pass-through). No saturation/clamping behavior. (evidence: decompile)

- `[RESOLVED] OQ-7 — Are there callers other than Read_Map_Section_And_IsoMapPacks?`
  → No. Single xref: UNCONDITIONAL_CALL at 0x004AD6E6 in
  Read_Map_Section_And_IsoMapPacks. (evidence: get_xrefs_to FUN_0056bac0)

- `[RESOLVED] OQ-8 — Is the decoder conditionally skipped based on any flag?`
  → No. The `if (0 < (int)uVar8)` check in the caller only skips if
  `Pipe__Constructor` returned 0 (i.e., the `[IsoMapPack5]` section is
  absent from the map file). Standard YR maps always have this section.
  (evidence: decompile of Read_Map_Section_And_IsoMapPacks)

- `[RESOLVED] OQ-9 — Does X use signed or unsigned arithmetic?`
  → Signed (sign-extended to i32 before multiplication). `(int)(short)local_34`.
  Y is unsigned u16. (evidence: decompile, explicit cast `(int)(short)`)

- `[RESOLVED] OQ-10 — What happens to OOB records?`
  → 7 bytes are consumed (not skipped). Combined with the 4 already read for
  the header = 11 bytes total consumed. Payload goes into `local_30` scratch
  buffer. (evidence: decompile: `puVar4 = local_30; uVar3 = 7`)

- `[DEFERRED] OQ-11 — What does DAT_00ABDC50 (dummy cell) contain?`
  (category: `out-of-scope`; reason: dummy cell is used only for OOB/null
  records and is never used for any gameplay output; content irrelevant for
  parity. read_memory showed it as all-zeros at static load time.)

- `[DEFERRED] OQ-12 — What is FUN_006C9890 (straw chain link)?`
  (category: `out-of-scope`; reason: Straw/Pipe infrastructure plumbing,
  already covered at high level by ASSET_PARSING_BRIDGES §6.4; not specific
  to IsoMapPack5 record format or cell field semantics.)

---

## Sources

### Ghidra addresses decompiled (this session)

- `0x0056BAC0` — `FUN_0056bac0` (IsoMapPack5 decoder — primary function)
- `0x004ACE70` — `Read_Map_Section_And_IsoMapPacks` (caller — full decompile)
- `0x00686B20` — `ScenarioClass__Full_Init` (caller's caller — confirmed call chain)
- `0x00544E30` — `FUN_00544e30` (tile-index fixup — full decompile)
- `0x0055C7C0` — `FUN_0055c7c0` (Straw read implementation)
- `0x006C9890` — `FUN_006c9890` (straw chain linker)

### Memory reads (this session)

- `read_memory @ 0x00ABD480` (4 bytes) → `00 00 00 00` — sentinel = 0x00000000
- `read_memory @ 0x0056BAC0` (256 bytes) — assembly bytes confirming `__thiscall`
  ECX→EBX and `[EBX+0x13C]` cell_array access

### Prior research documents cross-referenced

- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` — field offsets +0x38, +0x119, +0x11A, +0x11B, +0x11C
- `CELL_0x11A_POLARITY_RECONCILE_GHIDRA_REPORT.md` — HIGH-confidence verdict on +0x11A semantics
- `MAPCLASS_GHIDRA_REPORT.md` / `MAPCLASS_COMPLETE_DECODE.md` — MapClass+0x13C = cell_array data ptr
- `ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md` §6.2, §6.4 — IsoMapPack5 overview and LZO codec
- `SEA_TILES_GHIDRA_REPORT.md` §5 — `[Map] Level=` pre-fill interaction

### INI files checked

- `ini/rulesmd.ini`, `ini/artmd.ini` — no IsoMapPack5-specific INI keys exist;
  the format is purely binary within the map file. `[Map] Level=` key is read
  by `Read_Map_Section_And_IsoMapPacks` (confirmed in decompile).
