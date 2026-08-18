# CellClass +0x11A Semantic Reconciliation — Ghidra Research Report

**Topic:** Resolve the three-way conflict between `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md`,
`BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`, and `BRIDGE_SYSTEM.md` over what the byte
at `CellClass + 0x11A` actually is.

**Date:** 2026-05-18
**Confidence:** HIGH on the semantics of `+0x11A`, `+0x11B`, `+0x11C` (all three offsets
verified directly by reading raw bytes, decompiling readers, decompiling writers, and
cross-checking against Ghidra's `CellClass` struct layout).
**Active in YR:** Yes — every read and write site cited is on the live retail-YR draw +
map-load path. No TS-only gating.

---

## 1. The conflict

Three docs name the byte at `cell + 0x11A` three different ways:

| Doc | §  | Claim for `+0x11A` |
|-----|----|--------------------|
| `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md` | Phase 1C, §6 table line 510 | `sub_tile (icon idx within IsoTileType)` — consumed by `TMP_TileBlitter` |
| `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md` | Phase 2B+2C, §10.3 open question | "damage_state_1" — read by `UpdateAdjacentBridges_High` |
| `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` | §2 byte-offsets table line 64 | `Height` — "tile sub-type byte. Read by `TMP_ReadSlopeType` for slope lookup. Written ONLY to 0 in RecalcAttributes' cliff-fallback at 0x47D5E9 — otherwise preserved." |
| `BRIDGE_SYSTEM.md` | line 18, 541 | `bridge_sub_type` — "Bridge body orientation / sub-type", "bit 0 = orientation" |

---

## 2. Verdict (one line)

**`cell + 0x11A` is the per-cell sub-tile (icon) index byte of the cell's `IsoTileType`.**
It is the same field Ghidra's struct names `Height` (a poor name — it is not a Z
coordinate; `+0x11B` is). For bridges, it is the per-cell *sub-tile slot* within the
bridge's `IsoTileType` (which encodes orientation/segment among 16+ sub-tiles).

- Phase 1C is **CORRECT**.
- BRIDGE_DEFERRED_MECHANICS is **CORRECT** about offsets and the cliff-fallback write,
  but the field name "Height" is misleading (it is the iso-tile sub-tile index, not a
  Z field).
- BRIDGE_SYSTEM is **PARTIALLY CORRECT** — the byte is a sub-tile/sub-type index, but
  it is **not bridge-specific** (every cell has one — sand, grass, slopes, water, all
  use the same byte to pick their sub-tile within their `IsoTileType`).
- Phase 2B+2C of BRIDGE_DISPLAY_TABLE is **WRONG**: `UpdateAdjacentBridges_High` reads
  `+0x11A` as a *sub-tile index* (compared to literal sub-tile values 5/7/8/12), not
  as a damage state. The damage state lives at `+0x11E`.

---

## 3. Evidence

### 3.1 Authoritative struct layout (Ghidra `CellClass`)

```
offset 282 (0x11A) | byte | Height
offset 283 (0x11B) | byte | Level
offset 284 (0x11C) | byte | SlopeIndex
```

(Ghidra struct size 328 bytes, queried 2026-05-18.) The label `Height` at `+0x11A` is
a Ghidra inheritance from RA1/TS struct nomenclature — the actual semantics, verified
below, is "iso-tile sub-tile index".

### 3.2 Read sites of `cell + 0x11A` (all live in YR)

**`CellOverlay_TileDraw @ 0x00480350`** — per-frame ISO terrain blit:
```
uVar1 = *(undefined1 *)(param_1 + 0x11a);              // read
if (piVar5[0xbc] < 2) goto LAB_00480403;
cVar2 = FUN_005471f0(uVar1);                           // sub-tile validity check
...
TMP_TileBlitter(*(...+0x34), uVar1, g_PrimarySurface, ...);
```
The byte is passed as the sub-tile/icon-index argument into the tile blitter.

**`TMP_ReadSlopeType @ 0x005471B0`** — slope lookup via the cell's IsoTileType:
```
int __thiscall TMP_ReadSlopeType(int *iso_tile_type, int sub_idx) {
  piVar1 = (int *)(**(code **)(*iso_tile_type + 0x9c))();   // vtable: get TMP image
  if (piVar1 != 0) {
    if (piVar1[sub_idx % (piVar1[1] * *piVar1) + 4] != 0)
      return (int)*(char *)(piVar1[...] + 0x2a);
  }
  return 0;
}
```
Called from `RecalcAttributes` as `TMP_ReadSlopeType(this->Height)` where `Height` is
the `+0x11A` field — the byte indexes into the iso-tile's sub-tile array to read the
sub-tile's slope code byte (offset 0x2A within the sub-tile entry). This is the
"sub_tile_idx → slope" pipeline, not a separate "Height" concept.

**`UpdateAdjacentBridges_High @ 0x00576770`** — bridge-rim refresh:
```
iVar7 = (*(int *)(puVar6 + 0x38) - DAT_00aa0e28) + 1;
if (((iVar7 == DAT_00abc2b4) || (iVar7 == DAT_00aa1130)) && (puVar6[0x11a] == '\b')) {  // 8
  uVar10 = 2;
} else if ((... range checks on iVar7 ...) && (puVar6[0x11a] == '\x05')) {  // 5
  uVar10 = 2;
} else if ((... ) && (puVar6[0x11a] == '\f')) {                              // 12
  uVar10 = 4;
} else if ((... ) && (puVar6[0x11a] == '\a')) {                              // 7
  ...
}
```
The constants 5, 7, 8, 12 are **sub-tile slot numbers** within a bridge IsoTileType
that identify "rim/end" sub-tiles of various bridge orientations. They are NOT damage
states — damage states live at `+0x11E` and span 0..0x11 (0..17). This decisively
refutes Phase 2B+2C's "damage_state_1" label.

**`UpdateBridgeEdgeTiles_High @ 0x00576200`** — same pattern: compares
`puVar15[0x11a]` to literal sub-tile values `'\x04'` and `'\x02'`, never writes
`+0x11A`. Writes `+0x11E = 0` (bridge_state) and `+0x44 = -1` (overlay) instead.

### 3.3 Write sites of `cell + 0x11A`

**`Read_Map_Section_And_IsoMapPacks @ 0x004ACE70`** — map-clear loop (per cell):
```
*(int *)(iVar4 + 0x38) = iVar5 + uStack_10c;             // IsoTileTypeIndex = ClearTile
*(char *)(iVar4 + 0x11b) = *(char *)(iVar4 + 0x11b) + (char)ppuStack_108;  // Level += base
*(undefined1 *)(iVar4 + 0x11a) = 0;                       // sub-tile index = 0
```
After clear, IsoMapPack5 (`FUN_0056bac0`) overwrites both `+0x38` (tile type) and
`+0x11A` (sub-tile) from packed map data.

**`CellClass__RecalcAttributes @ 0x0047D2B0`** — cliff-fallback path:
- At `0x47D5E9`: `88 86 1A 01 00 00` = `MOV [ESI+0x11A], AL` with `AL=0`. Resets
  sub-tile index to 0 when the iso tile is rejected.
- At `0x47D5F9`: `88 86 1C 01 00 00` = `MOV [ESI+0x11C], AL` with `AL=0`. Resets
  SlopeIndex to 0 in the same path. BRIDGE_DEFERRED_MECHANICS' citation of these
  two writes is verified byte-for-byte.

(Confirmed via raw memory read 2026-05-18.)

**`CellClass__RecalcAttributes` Level write at `0x47D94E`** — `88 86 1B 01 00 00` =
`MOV [ESI+0x11B], AL`. This is the `Level` byte (signed height level), NOT the
sub-tile byte. BRIDGE_DEFERRED_MECHANICS' offset citation is correct.

### 3.4 The neighbor bytes (no conflict)

| Offset | Byte name | Role | Doc agreement |
|--------|-----------|------|---------------|
| `+0x11A` | `Height` (Ghidra) / sub-tile index | Iso-tile sub-tile slot 0..N-1. Drives `TMP_TileBlitter`, `TMP_ReadSlopeType`, `FUN_005471F0` pavement check, and bridge-rim sub-tile matching in `UpdateAdjacentBridges_High` / `UpdateBridgeEdgeTiles_High`. | All three docs reference this byte by different names; BRIDGE_DEFERRED_MECHANICS offset is right, label "Height" is misleading. |
| `+0x11B` | `Level` (signed i8) | Z-level (each level = 15 px world Z). `MOVSX` reads throughout movement / pathfinding. Bridge deck = `Level + 4`. | All three docs agree. |
| `+0x11C` | `SlopeIndex` | 0..20 terrain slope. Written by `RecalcAttributes` from `TMP_ReadSlopeType(this->Height)` (i.e. derived FROM `+0x11A`). | All three docs agree. |
| `+0x11D` | `HeightInPixels` (derived) | `(height_raw - 30) / 15`. | BRIDGE_DEFERRED_MECHANICS only. |
| `+0x11E` | `bridge_damage_state` | 0..17 damage-state byte. Primary frame-index driver for bridge overlay SHP. | All three docs agree. THIS is the byte Phase 2B+2C confused with `+0x11A`. |

---

## 4. Corrections needed (per doc)

### 4.1 `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md`

- §10.3 "Open Question (Conflicts)": **resolved**. Phase 1C's "sub_tile (icon idx)"
  label is correct; Phase 2B+2C's "damage_state_1" interpretation of
  `UpdateAdjacentBridges_High`'s read is wrong. The literals 5/7/8/12 in
  `UpdateAdjacentBridges_High` (and 2/4 in `UpdateBridgeEdgeTiles_High`) are sub-tile
  slot indices within the bridge IsoTileType, NOT damage states.
- §6 table line 510 (`+0x11A` row): keep as-is.
- Any references inside Phase 2B+2C that label `+0x11A` as "damage_state_1" or
  "damage_state" should be re-tagged as "sub_tile_idx".

### 4.2 `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`

- §2 line 64 (`+0x11A` row): offsets and write-site address (0x47D5E9) are correct.
  Label "Height" is misleading — the byte is the iso-tile sub-tile index, not a Z
  field. Suggest renaming to "SubTileIndex" or "IsoSubTile" with a note that
  Ghidra's auto-label "Height" is RA1/TS-era nomenclature. The "Tile sub-type byte"
  description is accurate; the "Read by TMP_ReadSlopeType for slope lookup" detail
  is verified.
- §2 line 65 (`+0x11B` Level row): correct.
- §2 line 66 (`+0x11C` SlopeIndex row): correct.

### 4.3 `BRIDGE_SYSTEM.md`

- Line 18 / 541: "bridge_sub_type" with note "(bit 0 = orientation)" is too
  bridge-specific. The byte is the per-cell sub-tile index for *any* terrain
  (sand, grass, slope, water — and bridges). For bridges specifically it encodes
  which segment/orientation sub-tile of the bridge IsoTileType is shown, but the
  byte itself is not bridge-specific. Suggested name: `sub_tile_idx` with a note
  on bridge interpretation. "Bit 0 = orientation" is an over-narrow claim — see
  the 5/7/8/12 and 2/4 sub-tile slot enumeration in `UpdateAdjacentBridges_High`
  for evidence that the byte is a small enum of sub-tile slot numbers, not a
  bit-packed value.

---

## 5. One-line YR-vs-TS check

All read sites (`CellOverlay_TileDraw`, `TMP_ReadSlopeType`, `UpdateAdjacentBridges_High`,
`UpdateBridgeEdgeTiles_High`) and all write sites (map load, `RecalcAttributes`,
IsoMapPack5 decode) are on the unconditional retail-YR map/draw path. No
`SpecialFlags & 0x1000` (FogOfWar), no scenario-flag gating. **Active in YR: Yes.**

---

## 6. Sources

- Ghidra MCP read-only session, 2026-05-18.
- `gamemd.exe` decompilations:
  - `CellClass__RecalcAttributes @ 0x0047D2B0`
  - `CellOverlay_TileDraw @ 0x00480350`
  - `TMP_ReadSlopeType @ 0x005471B0`
  - `FUN_005471F0 @ 0x005471F0` (pavement bit pre-check)
  - `MapClass__UpdateAdjacentBridges_High @ 0x00576770`
  - `MapClass__UpdateBridgeEdgeTiles_High @ 0x00576200`
  - `Read_Map_Section_And_IsoMapPacks @ 0x004ACE70`
- Raw byte reads at `0x47D5E6`, `0x47D5F4`, `0x47D94E` confirming `MOV [ESI+0x11A]`,
  `MOV [ESI+0x11C]`, `MOV [ESI+0x11B]` opcodes.
- Ghidra `CellClass` struct layout (size 328): offset 282=`Height`, 283=`Level`,
  284=`SlopeIndex`.
