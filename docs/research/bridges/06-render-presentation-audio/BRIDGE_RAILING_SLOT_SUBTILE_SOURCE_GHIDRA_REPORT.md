# Bridge Railing Slot/Sub-Tile Source - Ghidra Report

**Date:** 2026-05-16
**Scope:** Targeted follow-up to verify how `FUN_00547230 @ 0x00547230` receives its table slot source and required-sub-tile input, and whether the current Rust implementation can safely treat them as the same value.
**Target binary:** `gamemd.exe` / Yuri's Revenge.
**Rust code changes:** None.
**Active in YR:** Yes. The path is called from the live tactical overlay layer: `FUN_006D7C00 @ 0x006D7C00` -> `FUN_004802A0 @ 0x004802A0` -> `FUN_00547230 @ 0x00547230`.
**Overall confidence:** HIGH for the binary source mapping; MEDIUM for the Rust-field mapping because the Rust renderer's current bridge state may not yet carry every binary runtime tile-class mutation.

## Summary

The table slot and caller sub-tile are different binary inputs.

- Caller sub-tile comes from `CellClass+0x11A`.
- The table range/slot comes from `IsoTileTypeClass+0x294`, after first selecting the `IsoTileTypeClass*` via `CellClass+0x38`.
- `CellClass+0x38` is the cell's current global `IsoTileTypeIndex`, used as an index into `g_IsoTileTypeArray` (`DAT_00A8ED2C`).
- `IsoTileTypeClass+0x294` is initialized by `IsometricTileTypeClass__Constructor @ 0x005447C0` from the constructor's tile-index argument. For ordinary entries this is the global tile index allocated during the theater `[TileSet####]` load loop.
- Therefore, using the same Rust value for `(table_slot, caller_sub_tile)` is not faithful. It can accidentally suppress the only two nonzero `DAT_00ABC210` entries, because slot 4 requires sub-tile 6 and slot 6 requires sub-tile 1.

## Call Chain

### `FUN_006D7C00 @ 0x006D7C00`

This is the tactical overlay sweep that calls the railing trampoline.

Relevant behavior:

1. It computes the visible cell sweep from the viewport.
2. For each in-bounds cell, it calls `MapClass__Get_CellClass(&cell_coord)`.
3. It sets `ECX` to the returned `CellClass*`.
4. It calls `FUN_004802A0`.

Evidence:

```text
006D7ED6 CALL 0x005657A0
006D7EDB MOV ECX,EAX
006D7EDD CALL 0x004802A0
```

Decompiler summary:

```text
MapClass__Get_CellClass(&param_2);
FUN_004802a0(piVar11, uVar12);
```

The decompiler hides the `ECX` receiver, but the assembly shows that the `CellClass*` return in `EAX` becomes `ECX` for `FUN_004802A0`.

Confidence: HIGH.

## Trampoline Input Mapping

### `FUN_004802A0 @ 0x004802A0`

Decompiled signature:

```text
void __thiscall FUN_004802a0(int cell, undefined4 *screen_xy, undefined4 *clip_rect)
```

Relevant decompile:

```text
if (*(int *)(cell + 0x38) == 0xffff) {
    sub_tile = 0;
    iso = *(int *)(DAT_00a8ed2c + g_ClearTile * 4);
} else {
    sub_tile = *(undefined1 *)(cell + 0x11a);
    iso = *(int *)(DAT_00a8ed2c + *(int *)(cell + 0x38) * 4);
    if (1 < *(int *)(iso + 0x2f0)) {
        FUN_005471f0(sub_tile);
    }
}
if (*(char *)(iso + 0x2e1) != '\0') {
    FUN_00547230(
        iso,
        sub_tile,
        g_PrimarySurface,
        screen_x,
        screen_y + *(char *)(cell + 0x11b) * -0xf,
        clip_x,
        clip_y,
        clip_w,
        clip_h,
        *(char *)(cell + 0x11b) * -0xf + 0x3a
    );
}
```

Assembly around the call confirms:

```text
004802A6 MOV EAX,[EBP + 0x38]
004802B8 MOV BL,byte ptr [EBP + 0x11A]
004802BE MOV ESI,[ECX + EAX*4]       ; ECX = DAT_00A8ED2C
...
00480336 PUSH EBX                    ; stack arg: sub_tile
00480337 MOV ECX,ESI                 ; this/ECX: IsoTileType*
00480339 CALL 0x00547230
```

Findings:

| Binary source | Meaning | Used as |
|---|---|---|
| `CellClass+0x38` | current global `IsoTileTypeIndex`; `0xFFFF` means clear fallback | index into `DAT_00A8ED2C` |
| `DAT_00A8ED2C[cell+0x38]` | `IsoTileTypeClass*` | receiver for `FUN_00547230` |
| `CellClass+0x11A` | current sub-tile index within the TMP/IsoTileType | `FUN_00547230` `param_2` / required-sub-tile comparator |
| `CellClass+0x11B` | signed height level | vertical screen offset and zheight-like argument |
| `IsoTileType+0x2E1` | shadow-caster/railing-emitter flag | outer gate before calling `FUN_00547230` |

Tiny detail: `FUN_004802A0` calls `FUN_005471F0(sub_tile)` when `IsoTileType+0x2F0 > 1`, but discards the return value. This matches the earlier display-table report's "dead pre-check/prefetch-like" observation. The call is not the source of the table slot.

Confidence: HIGH.

## Consumer Slot Mapping

### `FUN_00547230 @ 0x00547230`

Relevant decompile:

```text
iVar4 = *(int *)(iso + 0x294);
if (iVar4 in [DAT_00ABC1F8, DAT_00ABC1F8 + 10)) {
    base = DAT_00ABC1F8;
    entry = DAT_00ABC210[(iVar4 - base)];
} else if (iVar4 in [DAT_00AA1098, DAT_00AA1098 + 10)) {
    base = DAT_00AA1098;
    entry = DAT_00ABC210[(iVar4 - base)];
} else {
    fallback to DAT_00ABC2D0 shadow-caster ranges
}

if (param_2 != entry.required_sub_tile) return;
if (entry.frame == 0) return;
draw frame entry.frame - 1;
```

Findings:

- The slot is not `CellClass+0x11A`.
- The slot is `IsoTileType+0x294 - selected_range_base`.
- The same `DAT_00ABC210` table is selected for both `[DAT_00ABC1F8,+10)` and `[DAT_00AA1098,+10)`.
- The required-sub-tile comparison uses `CellClass+0x11A` as forwarded by `FUN_004802A0`.

Confidence: HIGH.

## Loader Mapping for `IsoTileType+0x294`

### `IsometricTileTypeClass__Constructor @ 0x005447C0`

Relevant decompile:

```text
ObjectTypeClass__Constructor(name);
this[0xA5] = param_2;        // byte offset 0x294
...
if (editor_clone_flag == 0) {
    DAT_00A8ED2C[DAT_00A8ED38++] = this;
}
```

The `param_2` passed by the theater loader is the current global tile index for the tile being constructed.

### `Read_Theater_TileSets_INI @ 0x00545150`

Relevant decompile around the `[TileSet####]` loop:

```text
iStack_960 = 0;       // TileSet section number
iStack_9EC = 0;       // cumulative/global tile index

if (iVar11 == iStack_8C0) DAT_00ABC1F8 = iVar16;  // SlopeSetPieces
if (iVar11 == iStack_908) DAT_00AA1098 = iVar16;  // SlopeSetPieces2

...
// For each tile in the current TileSet:
iVar13 = iStack_9EC;
iStack_9EC = iStack_9EC + 1;
IsometricTileTypeClass__Constructor(this, iVar13, ...);
```

Findings:

- `[General] SlopeSetPieces` and `SlopeSetPieces2` are theater TileSet section numbers, not direct table-slot values.
- During theater load, the binary stores the cumulative first tile index of those TileSet sections into `DAT_00ABC1F8` and `DAT_00AA1098`.
- `IsoTileType+0x294` stores the per-tile cumulative/global index assigned by the same loader.
- For the `DAT_00ABC210` path, the local table slot is the current tile's global index minus the stored cumulative base for the matching SlopeSetPieces section.

Confidence: HIGH.

## INI Data Check

Retail theater INI values in this repo:

| Theater file | `SlopeSetPieces` | `SlopeSetPieces2` |
|---|---:|---:|
| `ini/temperat.ini` / `ini/temperatmd.ini` | 25 | 26 |
| `ini/snow.ini` / `ini/snowmd.ini` | 25 | 26 |
| `ini/urban.ini` / `ini/urbanmd.ini` | 25 | 26 |
| `ini/urbannmd.ini` | 115 | 116 |
| `ini/desertmd.ini` | 25 | 26 |
| `ini/lunarmd.ini` | 25 | 26 |

These are TileSet section numbers. They must be translated through the theater lookup's cumulative bounds/start table to match `DAT_00ABC1F8` and `DAT_00AA1098`.

Confidence: HIGH.

## Rust Implementation Status

Current Rust fields and behavior relevant to this path:

- `src/map/resolved_terrain.rs` exposes `ResolvedTerrainCell.final_tile_index` and `final_sub_tile`.
- `src/map/theater.rs` currently parses `BridgeSet`, `WoodBridgeSet`, and bridge middle/top keys, but a source scan found no parsed `SlopeSetPieces` or `SlopeSetPieces2` fields.
- `src/render/bridge_railing_atlas.rs` now stores `required_sub_tile` and uses recovered `DAT_00ABC210` values.
- `src/app_instances/bridges.rs` currently documents and implements the table slot source as `ResolvedTerrainCell.final_sub_tile`, then passes that same value as the caller sub-tile.

Binary comparison:

| Binary concept | Current likely Rust analogue | Confidence / caveat |
|---|---|---|
| `CellClass+0x11A` sub-tile | `ResolvedTerrainCell.final_sub_tile` | HIGH for static resolved terrain; if runtime bridge mutations alter sub-tile, use runtime state instead |
| `CellClass+0x38` current global IsoTileType index | `ResolvedTerrainCell.final_tile_index` | MEDIUM-HIGH for static terrain; runtime bridge destruction/repair may need a runtime tile-index mirror |
| `DAT_00ABC1F8` / `DAT_00AA1098` cumulative base | `TheaterData.lookup.bounds()[SlopeSetPieces].start` and `[SlopeSetPieces2].start` | HIGH if Rust parses those `[General]` keys as TileSet section numbers |
| `IsoTileType+0x294` | usually the same global tile index used to select the `IsoTileType` | HIGH for ordinary constructed entries, based on constructor + loader |

The important correction: `final_sub_tile` should not be used as the `DAT_00ABC210` table slot. It is only the required-sub-tile comparator input.

## Implementation Implications

These are not Rust code changes, only verified constraints for a future patch:

1. The low-level `DAT_00ABC210` table values and required-sub-tile gate are valid.
2. A faithful emitter needs two independent inputs:
   - table slot = current global tile index minus the cumulative base for `SlopeSetPieces` or `SlopeSetPieces2`
   - caller sub-tile = current cell sub-tile (`CellClass+0x11A`)
3. For the current Rust architecture, the least speculative static mapping is:
   - parse `[General] SlopeSetPieces` and `SlopeSetPieces2`
   - translate section numbers to cumulative starts via `TilesetLookup::bounds()`
   - compare the cell's current tile index against `[start, start + 10)`
   - pass `final_sub_tile` only as the required-sub-tile comparator
4. If the bridge runtime can mutate the cell tile index after `ResolvedTerrainGrid` is built, the renderer must use that runtime tile index instead of static `ResolvedTerrainCell.final_tile_index`.
5. No evidence supports using overlay names to select the `DAT_00ABC210` entry. Overlay names may still be useful for Rust's existing RAILBRDG-specific path, but they are not the binary `FUN_00547230` selector.

## Open Questions

1. Does current Rust maintain a runtime mirror of `CellClass+0x38` for high bridge damage/destruction states, or is `ResolvedTerrainCell.final_tile_index` static after map load? This is a Rust architecture question, not a binary question.
2. The exact visual relationship between this verified `C_SHADOW.SHP` emitter and any separate `RAILBRDG` overlay visuals remains open from the parent report.
3. If future work ports the `DAT_00ABC2D0` fallback, the same slot/source pattern applies, but the relevant range bases are the five `ShadowCaster=yes` starts, not `SlopeSetPieces`.

## Sources

- Ghidra decompile: `FUN_006D7C00 @ 0x006D7C00`
- Ghidra assembly context: call at `0x006D7EDD`
- Ghidra decompile: `FUN_004802A0 @ 0x004802A0`
- Ghidra assembly context: call at `0x00480339`
- Ghidra decompile: `FUN_00547230 @ 0x00547230`
- Ghidra decompile: `FUN_005471F0 @ 0x005471F0`
- Ghidra decompile: `IsometricTileTypeClass__Constructor @ 0x005447C0`
- Ghidra decompile: `Read_Theater_TileSets_INI @ 0x00545150`
- Existing report: `BRIDGE_THEATER_LOAD_TABLE_WRITERS_GHIDRA_REPORT.md`
- Existing report: `ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md`
- Existing report: `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md`
- INI files checked: `ini/temperat*.ini`, `ini/snow*.ini`, `ini/urban*.ini`, `ini/urbannmd.ini`, `ini/desertmd.ini`, `ini/lunarmd.ini`
