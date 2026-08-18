# CellClass__HasBridgeOverlay — 0x004865d0

**Proposed Ghidra label:** `CellClass__HasBridgeOverlay` (existing name authoritative — labeler skip rename, add plate comment only)

## Summary

Returns 1 (true) if the cell's overlay tile index (`CellClass+0x38`) falls within any of the known bridge tile-set ranges loaded from theater INI, or 0 (false) otherwise. Called by `TeleportLocomotionClass__PostWarpValidation` to determine whether the warp destination has a bridge overlay — which affects Z-height handling for the arriving unit.

The function checks five tile-set ranges, all stored in globals written by `Read_Theater_TileSets_INI` at startup. All five globals read as `0x00000000` at runtime in a static Ghidra snapshot (no theater loaded), confirming the ranges are populated only after theater INI is parsed.

## Active in YR

**Yes.** Called by `TeleportLocomotionClass__PostWarpValidation` at `0x00718aa9`, and by 11 other sites across the bridge/map system (confirmed via `get_xrefs_to 0x004865d0`). `PostWarpValidation` runs every tick when a unit is armed for warp and checks the destination cell before committing.

## Decompilation

Source: `decompile_function 0x004865d0`

```c
int __fastcall CellClass__HasBridgeOverlay(int param_1)
{
  int iVar1;
  uint3 uVar2;

  iVar1 = *(int *)(param_1 + 0x38);   // CellClass+0x38 = overlay tile index
  uVar2 = (uint3)((uint)iVar1 >> 8);  // high 3 bytes (carries iVar1 upper bits)

  // Shore-pieces range: [g_ShorePieces, g_ShorePieces + 0x2a)
  if ((g_ShorePieces <= iVar1) && (iVar1 < g_ShorePieces + 0x2a)) {
    return CONCAT31(uVar2, 1);          // low byte = 1 → true
  }

  // Four additional bridge tile-set ranges (loaded from theater INI):
  if (((iVar1 < DAT_00aa0738) || (DAT_00aa0738 + 0xe <= iVar1)) &&
     ((((iVar1 < DAT_00aa073c) || (DAT_00aa073c + 4 <= iVar1)) &&
       ((iVar1 < DAT_00abb110) || (DAT_00abb110 + 4 <= iVar1))) &&
      (((iVar1 < DAT_00aa1050) || (DAT_00aa1050 + 4 <= iVar1)) &&
       ((iVar1 < DAT_00aa10a0) || (DAT_00aa10a0 + 4 <= iVar1)))))) {
    return (uint)uVar2 << 8;            // low byte = 0 → false
  }
  return CONCAT31(uVar2, 1);            // low byte = 1 → true
}
```

**Return value**: the low byte is the boolean: `1` = cell has a bridge overlay, `0` = no bridge overlay. Upper bytes carry the high portion of the tile index but callers test only the low byte.

## Behavioral Analysis

The function encodes "is this cell's overlay tile a bridge tile?" by range-checking the overlay tile index against:

1. **Shore-pieces range**: `[g_ShorePieces, g_ShorePieces + 0x2a)` — 42 tiles. Shore pieces include the bridge surface tiles in RA2/YR.
2. **DAT_00aa0738 range**: 14 tiles (`+0xe`). Bridge ramp tiles for one theater.
3. **DAT_00aa073c range**: 4 tiles. Bridge ramp tiles variant.
4. **DAT_00abb110 range**: 4 tiles.
5. **DAT_00aa1050 range**: 4 tiles.
6. **DAT_00aa10a0 range**: 4 tiles.

All range bases are written by `Read_Theater_TileSets_INI` (confirmed via `get_xrefs_to 0x00aa0738` → WRITE from `Read_Theater_TileSets_INI`). This means the bridge detection is theater-dependent and initialized at map load, not at startup.

### How PostWarpValidation uses the result

`TeleportLocomotionClass__PostWarpValidation` calls `HasBridgeOverlay` on the destination cell. If the result is true (bridge), the warp validation path adjusts for bridge Z-height, consistent with the `g_BridgeZOffset_Teleport` path in `Update_Position`.

## Struct Field Accesses

### CellClass fields (via `param_1`)

| Byte Offset | Access | Purpose |
|---|---|---|
| +0x38 | `*(int *)(param_1 + 0x38)` | Overlay tile index (tile set ID) |

## Globals Referenced

| Symbol | Address | Value at runtime | Written by | Purpose |
|---|---|---|---|---|
| `g_ShorePieces` | (not resolved to address in this session — YELLOW) | 0 (static snapshot) | `Read_Theater_TileSets_INI` | Shore tile range base |
| `DAT_00aa0738` | `0x00aa0738` | 0x00000000 | `Read_Theater_TileSets_INI` (3 write sites) | Bridge tile range 1 base (14 tiles) |
| `DAT_00aa073c` | `0x00aa073c` | 0x00000000 | `Read_Theater_TileSets_INI` | Bridge tile range 2 base (4 tiles) |
| `DAT_00abb110` | `0x00abb110` | 0x00000000 | `Read_Theater_TileSets_INI` (2 write sites) | Bridge tile range 3 base (4 tiles) |
| `DAT_00aa1050` | `0x00aa1050` | 0x00000000 | `Read_Theater_TileSets_INI` (2 write sites) | Bridge tile range 4 base (4 tiles) |
| `DAT_00aa10a0` | `0x00aa10a0` | (not read in this session) | `Read_Theater_TileSets_INI` | Bridge tile range 5 base (4 tiles) |

All runtime values are 0 in the static Ghidra snapshot (no theater loaded). Actual values set at map load.

## Callers

| Address | Function | Role |
|---|---|---|
| `0x00718aa9` | `TeleportLocomotionClass__PostWarpValidation` | Warp destination bridge check |
| 11 other sites | Various map/bridge functions | General bridge-cell queries |

## Out-of-Scope Refs

| Symbol | Address | Reason |
|---|---|---|
| `Read_Theater_TileSets_INI` | `0x00545a98` (approximate) | Theater INI parser; not teleport-specific |
| `CellClass__IsOnBridgeSurface` | `0x00485063` | Reads same globals; separate function, not teleport-specific |
| `IsOnBridgeRamp` | `0x00578dc9` | Reads same globals; separate function, not teleport-specific |

## Unverified (YELLOW)

- **`g_ShorePieces` address**: the symbol `g_ShorePieces` appears in the decompile but its address was not resolved via `get_xrefs_to` in this session. The name suggests it is labeled in Ghidra. Its role as the shore-pieces tile range base is inferred from the function name and the 42-tile range width (`+0x2a`).

- **`DAT_00aa10a0`**: address confirmed in decompile; `read_memory` not called for this address in this session. Assumed to follow the same `Read_Theater_TileSets_INI` write pattern as the other four range globals, based on consistent xref pattern.

- **Return value upper bytes**: `CONCAT31(uVar2, 1)` packs the high 3 bytes of the tile index with low byte = 1. Callers that test only the low byte (boolean) see a clean `1`/`0`. Whether any caller uses the upper bytes was not verified.
