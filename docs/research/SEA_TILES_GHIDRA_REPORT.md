# Sea / Water Tiles — Ghidra Research Report

**Date:** 2026-05-17
**Primary addresses:**
- `CellClass::RecalcAttributes` @ `0x0047D2B0`
- `CellClass::RecalcZoneType` @ `0x00483C80`
- `CellClass::IsOnBridgeSurface` @ `0x00485060`
- `Read_Map_Section_And_IsoMapPacks` @ `0x004ACE70` (the `[Map] Fill=` reader)
- `FUN_00544BE0` — TMP terrain_type byte → LandType lookup
- `ShipLocomotionClass::Process` @ `0x0069FC10` (wake spawn)
- `FUN_0059A6C0` — Random Map Generator water-fill driver
- `Theater_ReadINI_LoadTileSets` @ `0x00545150` (reads `[General] WaterSet=`)

**Confidence:** HIGH overall — all primary functions decompiled and verified from the binary; memory tables read directly.
**Active in YR:** Yes — every coastal/water map exercises every code path here. Lunar theater is the one conditional exception (no water).

---

## 1. Overview — what "sea tile" actually means in the binary

A "sea tile" is a `CellClass` whose `IsoTileTypeIndex` (at `+0x38`) falls in the `WaterSet` range and whose `LandType` (at `+0xEC`) is set to `2` (Water).

Two stable invariants define it:

1. **Tile graphics:** The cell's `IsoTileTypeIndex` is in `[g_WaterSet, g_WaterSet + 14)`. `g_WaterSet` (at `DAT_00AA0738`) is the first tile_id of TileSet 21 ("Water", `FileName=Water`, `TilesInSet=14`) in every theater that has water (Temperate, Snow, Urban, Desert).
2. **Behavior:** The cell's `LandType` is `2`. This value is derived from the TMP file's per-cell `terrain_type` byte (`+0x29`) via a fixed 16-entry lookup table at `DAT_008288E4`. The water tiles' TMP cells store `terrain_type = 9`, and `DAT_008288E4[9] = 2 = Water`.

`RecalcZoneType` then translates `LandType==2` to `ZoneType=4` (Water), which is the value the passability matrix indexes for naval/amphibious gating.

There is **no separate "water overlay"** in stock YR — water is purely tile-based. The "OverlayType IsWater flag" claim found in earlier docs was a misreading; the flag at `OverlayTypeClass+0x2A8` is `IsWall` (Ghidra-confirmed via `RecalcZoneType` setting `ZoneType=2 = Wall` when it's set).

There is also **no per-frame water animation** in the engine. Open-water tiles use a deterministic per-cell variant pick (the same `GetTileVariantIndex` PRNG used for all terrain tiles) at load time, and that variant is rendered statically for the rest of the match. The "motion" players see on water is exclusively from:
- Wake animations spawned every 8 ticks by moving naval units (see §11).
- Static waterfall animations attached to a small set of tiles via `Tile%dAnim=` keys (§9.4).
- Ambient water sound (`_Amb_WavesLake`) on Naval Yards.

The open ocean itself does not animate.

---

## 2. The LandType enum — Water = 2 (binary), settled

Earlier docs had a long-running conflict: some said `LandType==2` for Water, others said `LandType==4`. **The binary answer is `2`.** Verified two ways:

### 2.1 Direct memory read of the terrain section name pointer table

`DAT_00839D68` holds 12 `char*` pointers, one per `LandType`. Each pointer is the `[Section]` name the engine looks up in `rules.ini` to read terrain speed values. Pointer table → string table read directly from memory:

| LandType | String pointer | String at that addr |
|---|---|---|
| 0 | `0x0081DC1C` | `Clear` |
| 1 | `0x0081DC14` | `Road` |
| **2** | `0x0081BAE8` | **`Water`** |
| 3 | `0x0081DC0C` | `Rock` |
| 4 | `0x0081AC58` | `Wall` |
| 5 | `0x00817278` | `Tiberium` |
| 6 | `0x0081DC04` | `Beach` |
| 7 | `0x0081DBFC` | `Rough` |
| 8 | `0x0081DBF8` | `Ice` |
| 9 | `0x0081DBEC` | `Railroad` |
| 10 | `0x0081DBE4` | `Tunnel` |
| 11 | `0x0081DBDC` | `Weeds` |

`[Water]` is at index 2. The speed table at `DAT_0089EA40` is `float[12][9]` and is indexed as `speed_table[LandType * 9 + SpeedType]`. The `[Water]` section in `rules.ini` fills row 2.

**Side-effect:** the *order* of the strings is the LandType enum. `Wall=4`, `Tiberium=5`, `Beach=6`, `Rough=7`, `Ice=8`, `Railroad=9`, `Tunnel=10`, `Weeds=11`. This makes sense of every branch in `RecalcAttributes` and `RecalcZoneType` (see §6, §7).

### 2.2 Live consumer code — `ShipLocomotionClass::Process` wake check

```c
// Address 0x0069FC10, near LAB_0069FE39 (the wake spawn block):
((iVar5 = (**(code **)(*(int *)piVar3[2] + 0x1bc))(),  // vtable+0x1BC → cell ptr
  *(int *)(iVar5 + 0xec) == 2 &&                       // cell.LandType == 2 (Water)
  (*(int *)(g_RulesClass_Instance + 0x94) != 0)))      // Rules.Wake != null
```

Direct comparison `cell.LandType == 2` to gate wake spawning is the canonical "is the ship on water" test in the binary.

**Important side note for the Rust port:** `passability.rs:39-48` defines `LandType::Water = 4`. This is a *deliberate remap* into an 8-column passability matrix the port uses for runtime checks; it is NOT a port of the binary's 12-row layout. Any time RE docs cite `LandType==2`, the Rust mapping is `LandType::Water` (raw u8 = 4 in the Rust enum). See `passability.rs:79-92` for `tmp_terrain_to_land_type(9) → Water`.

---

## 3. The TMP terrain table — how a cell *gets* LandType=2

Every TMP cell has a `terrain_type` byte at intra-cell offset `+0x29`. The engine maps it to a final LandType through a 16-entry u32 table at `DAT_008288E4`. Read directly from memory (`get_xrefs_to` confirms it's used only by `FUN_00544BE0`):

| `terrain_type` (TMP byte) | `DAT_008288E4[i]` | LandType |
|---|---|---|
| 0 | `0` | Clear |
| 1 | `8` | Ice |
| 2 | `8` | Ice |
| 3 | `8` | Ice |
| 4 | `8` | Ice |
| 5 | `10` | Tunnel |
| 6 | `9` | Railroad |
| 7 | `3` | Rock |
| 8 | `3` | Rock |
| **9** | **`2`** | **Water** |
| 10 | `6` | Beach |
| 11 | `1` | Road |
| 12 | `1` | Road |
| 13 | `0` | Clear |
| 14 | `7` | Rough |
| 15 | `3` | Rock |

**Sea tiles store `terrain_type = 9` in their TMP cell data.** This is the only path that produces `LandType=2` in stock YR.

**Tiny detail — multiple `terrain_type` bytes collapse to one LandType:**
- Four codes (1, 2, 3, 4) all map to Ice (8) — the four "ice variant" terrain bytes used by the snow theater shore tiles.
- Three codes (7, 8, 15) all map to Rock (3) — rocks/cliffs use multiple TMP codes (probably to drive different graphics or animation cells while sharing one LandType).
- Two codes (11, 12) map to Road (1).
- Codes 0 and 13 both map to Clear (0).

So the `terrain_type` byte carries *more information* than the LandType — it's a 16-value opcode that the engine has compressed into 12 LandTypes. Values 4-15 cover the TS-era richer terrain set; YR retained the same table layout.

**Out-of-range:** the lookup is `*(char *)(piVar1[param_2 % (...)] + 0x29) * 4`. Byte values 0-255 all index a valid offset within the 64-byte table only because TMP terrain_type is guaranteed ≤ 15 by the asset authoring tools. Values ≥16 would read garbage; no TMP in stock YR has a `terrain_type` byte above 15.

**Function:** `FUN_00544BE0(IsoTileTypeClass* tile_type, int cell_num)`:

```c
piVar1 = (int *)(**(code **)(*param_1 + 0x9c))();   // tile_type.get_tmp_data()
if (piVar1 != NULL) {
    int idx = param_2 % (piVar1[1] * *piVar1);       // cell_num % (width * height)
    if (piVar1[idx + 4] != 0) {                       // cell offset != 0 (cell is populated)
        return DAT_008288E4[*(char*)(piVar1[idx + 4] + 0x29) * 4];
    }
}
return 0;   // null tile → LandType 0 (Clear)
```

**Subtle off-by-one:** `param_2 % (width * height)` — the modulo means an out-of-bounds cell index *wraps* around inside the multi-cell template. This is benign because callers always pass valid indices, but worth noting if porting.

---

## 4. WaterSet — the tile range that draws water

### 4.1 The global and how it's populated

`g_WaterSet` = `DAT_00AA0738` (u32). Loaded by `Theater_ReadINI_LoadTileSets @ 0x00545150` from the per-theater INI's `[General] WaterSet=` integer.

**Two-phase population:**
1. During TileSet loop, the global stores the *tileset number* (e.g., `21`).
2. At terminator (`TilesInSet=-1`), the fixup pass rewrites every `[General]` global from a tileset number to the absolute first tile_id of that set: `g_WaterSet = g_TileSet_StartTileIds[21]`.

After load, `g_WaterSet` holds the absolute tile_id of the first water tile. All 14 water tiles occupy `[g_WaterSet, g_WaterSet + 14)`.

**Theater-by-theater values for `WaterSet`** (all theaters have `WaterSet=21`):

| Theater | INI line | WaterSet # | Tiles | FileName |
|---|---|---|---|---|
| Temperate | `temperatmd.ini` line 63 | 21 | 14 | `Water` |
| Snow | `snowmd.ini` line 58 | 21 | 14 | `Water` |
| Urban | `urbanmd.ini` line 63 | 21 | 14 | `Water` |
| Desert | `desertmd.ini` line 63 | 21 | 14 | `Water` |
| Lunar | `lunarmd.ini` line 63 | 21 | 14 | `Water` |

So the *tileset number* is identical across theaters (21 everywhere), but the actual TMP files loaded (`Water01.tem`, `Water01.sno`, `Water01.urb`, etc.) differ per theater. After theater load, `g_WaterSet` holds different absolute tile_ids per theater because each theater loads a different number of preceding tilesets — but the tile range is always 14 tiles wide.

### 4.2 Per-set INI metadata (TileSet 21, "Water")

Same across theaters (only `MarbleMadness` index differs):

| Key | Temperate | Snow | Urban | Desert |
|---|---|---|---|---|
| `SetName` | `Water` | `Water` | `Water` | `Water` |
| `FileName` | `Water` | `Water` | `Water` | `Water` |
| `TilesInSet` | 14 | 14 | 14 | 14 |
| `MarbleMadness` | 69 | 60 | 69 | 69 |
| `AllowBurrowing` | `false` | `false` | `false` | `false` |
| `RequiredForRMG` | `true` | `true` | `true` | `true` |
| `LowRadarColor` | 10,10,30 | 10,10,80 | 10,10,30 | 10,10,30 |
| `HighRadarColor` | 10,10,50 | 15,15,110 | 10,10,50 | 10,10,50 |

**Tiny detail — `AllowTiberium` is not set on the Water tileset**, which means it defaults to `false` (from `IsometricTileTypeClass` constructor). Ore therefore cannot grow on water cells — but this is the tile-side check; the cell-side check via `LandType==2 → speed_table[2*9 + 0 Foot] = 0%` also blocks unit-walked ore propagation.

**Tiny detail — `RequiredForRMG=true`** means the random map generator (FUN_0059AD10 / FUN_0059AFA0 / FUN_0059B200) is allowed to use these tiles, and the load-time TMP-cull pass (FUN_00546DA0) will keep them loaded even if no cell on the map references them.

### 4.3 Lunar theater special — water nulled

Per `IsometricTileTypeClass::Theater_ReadINI_LoadTileSets` (the interior/lunar branch inside the `TilesInSet=-1` terminator block), when `local_95c == 5` (theater code for Lunar/Interior):

```
zero g_ShorePieces
zero DAT_00AA0738 (g_WaterSet)
zero DAT_00AA1020 (CliffSet)
zero DAT_00AA101C (WaterCliffs)
zero g_WaterBridge
zero DAT_00AA0E28 (BridgeSet)
zero DAT_00ABAD1C (WoodBridgeSet)
```

After this, any code that reads `g_WaterSet` on Lunar will see 0 (or whatever sentinel). `IsOnBridgeSurface` returns false for everything because the check `tile_id < g_WaterSet + 0xe` collapses to `tile_id < 14` which most map cells won't satisfy. **Lunar maps have no water and no bridges.**

Even though `lunarmd.ini` declares `WaterSet=21` in `[General]`, the Lunar terminator branch zeros the resolved global anyway. This is a hard internal kill, not an INI override.

### 4.4 Why 14 tiles?

The 14 tiles are *variants of open-water graphics* — different ripples, different shading, and the LAT base. The first 4 tiles (`g_WaterSet + 0..3`) are the variants used by the `[Map] Fill=Water` random-fill pass (§5). The remaining 10 tiles cover more specific cell positions (sub-tile templates, edges, RMG glue). The full set is used by:
- `Read_Map_Section_And_IsoMapPacks` to randomly pick the initial fill tile (`Random_RandomRanged(0, 3)`).
- `IsoMapPack5` to draw any specific water tile placed by the editor.
- `CellClass::IsOnBridgeSurface` to classify any tile in the range as "water surface" for naval pathing.
- The bridge low-water-bridge detection (`IsOnLowWaterBridge`) only inspects the first 14 of `g_WaterSet` (i.e. all of it, since the set has 14 tiles total).

### 4.5 Sibling tilesets in `[General]` related to water

Per the per-theater INI cross-reference:

| Key | Tileset # | Tiles | Purpose |
|---|---|---|---|
| `WaterSet` | 21 | 14 | Open water |
| `ShorePieces` | 12 | 42 | Land-water edge transitions (Beach) |
| `WaterCliffs` | 15 | 28 | Cliff-faces dropping into water |
| `WaterCaves` | 57 | 4 | Small water caves (decorative) |
| `WaterBridge` | 76 | 2 | Special low-bridge spanning water |
| `WaterfallEast` | 49 | 4 | East-flowing waterfall |
| `WaterfallWest` | 51 | 4 | West-flowing waterfall |
| `WaterfallNorth` | 50 | 4 | North-flowing waterfall |
| `WaterfallSouth` | 30 | 4 | South-flowing waterfall |

(Temperate values shown. Snow shifts the waterfall tileset numbers to 35/36/37/30 but the count is the same.)

Each of these is a separate tileset in the theater INI with its own `FileName=` and `TilesInSet=` value. The values stored in the global like `g_ShorePieces` are absolute tile_ids after the load fixup, just like `g_WaterSet`.

**Tiny detail — the four waterfall directions are organized A/B/C/D**, where each is a *separate tileset* with 4 tiles (4 animation frames):
- Temperate `WaterfallEast` = tileset 49 = "Waterfalls-B" with `FileName=W-b-`.
- The "-a-/-b-/-c-/-d-" suffix on `FileName=` distinguishes the four directional sets; the `Tile%dAnim=WA01X` keys (in the master `[Waterfalls]` block, lines 1245-1260 of `temperatmd.ini`) attach the `WA01X`-`WA04X` animations to specific cells of the East tileset and similarly for B, C, D.

### 4.6 The 14-tile range invariant — `CellClass::IsOnBridgeSurface`

```c
undefined4 CellClass__IsOnBridgeSurface(int param_1)   // 0x00485060
{
    if ((DAT_00AA0738 <= *(int *)(param_1 + 0x38)) &&
        (*(int *)(param_1 + 0x38) < DAT_00AA0738 + 0xE))
    {
        return 1;
    }
    return 0;
}
```

A cell is "on bridge surface" iff its `IsoTileTypeIndex` is in `[g_WaterSet, g_WaterSet + 14)`. This is the canonical "is this a water cell" tile-range predicate. **Two tiny details:**

1. The bound is `+ 0xE` (= 14, decimal), matching `TilesInSet=14`. This is hardcoded — not derived from the tileset's loaded size. If a mod set `TilesInSet=20` for the Water set, the engine would still only treat the first 14 as bridge surface.
2. The function name is `IsOnBridgeSurface`, but every caller treats it as "is this a water cell". The name is a TS legacy artifact — in TS, ships moved on water tiles as if they were on a bridge, hence the name persists.

**Callers** (`get_xrefs_to 0x00485060`): only one direct caller from the asset cross-ref — `Cell_passability_building_placement @ 0x0047CA12`. The function is also referenced through the IsOnBridgeSurface call from other places, but as far as the binary direct xrefs show, naval-placement-validation is the primary consumer. Naval Yards (`WaterBound=yes`) call this to verify the cell is water before allowing placement.

---

## 5. How a map *starts* with water — `[Map] Fill=` and IsoMapPack5

### 5.1 The initial-fill pass — `Read_Map_Section_And_IsoMapPacks @ 0x004ACE70`

When a scenario loads, before the IsoMapPack5 cells are unpacked, the entire playfield is pre-filled with one of two tiles based on the `[Map]` section's `Fill=` key:

```c
// Read [Map] Fill key from the .map file (default = "Clear")
CCINIClass__ReadString(&PTR_LAB_00706148_5_0081fff0,    // [Map] section
                       &DAT_00820170,                    // key name "Fill"
                       s_Clear_0081dc1c,                 // default = "Clear"
                       &local_b0, 0x20);

iVar4 = FUN_007c8d20(&local_b0, s_Water_0081bae8);       // strcmp(value, "Water")
if (iVar4 == 0) {
    uStack_110 = 3;                  // upper bound for Random_RandomRanged
    uStack_10c = DAT_00AA0738;       // base = g_WaterSet
}
// else uStack_110 = 0, uStack_10c = g_ClearTile (the defaults set earlier)

MapClass__CellIterator_Init();
iVar4 = MapClass__CellIterator_Next();
while (iVar4 != 0) {
    iVar5 = Random__RandomRanged(0, uStack_110);
    *(int *)(iVar4 + 0x38) = iVar5 + uStack_10c;     // cell.IsoTileTypeIndex = base + random
    *(char *)(iVar4 + 0x11b) += (char)ppuStack_108;  // cell.Level += [Map] Level
    *(undefined1 *)(iVar4 + 0x11a) = 0;              // cell.SubTileIndex = 0
    iVar4 = MapClass__CellIterator_Next();
}
```

**Tiny details that matter:**

1. **Default `Fill` value is `Clear`** — the string at `0x0081DC1C`. If the `.map` file's `[Map]` section omits `Fill=`, the engine treats every untouched cell as `g_ClearTile`. **There is no implicit "ocean default"** — maps must explicitly opt into water-fill with `Fill=Water`.
2. **`Fill=Water` paints the entire map with random water tiles in range `[g_WaterSet, g_WaterSet + 4)`.** Note `Random_RandomRanged(0, 3)` is inclusive on both ends: 4 possible values (0, 1, 2, 3). Only the first **4** of the 14 water tiles are used for fill — the remaining 10 are only ever drawn via explicit IsoMapPack5 entries.
3. **`SubTileIndex` is zeroed** on every cell during fill. This matters because water tiles in WAE/FA2 are 1×1 templates (no multi-cell water tiles in stock content), so sub_tile is always 0 anyway.
4. **`Level` is *added*, not set.** `cell.Level += [Map] Level`. The `[Map] Level=` INI key (read at `ppuStack_108`) is the *baseline cliff height* offset for the whole map, applied to every fill cell. IsoMapPack5 entries later set their cells' Level explicitly, overwriting this.
5. **Only "Clear" and "Water" are recognized as fill values.** The strcmp only checks against `"Water"`. Anything else (or absent) falls through to the default of `g_ClearTile`. There is no `Fill=Snow`, `Fill=Sand`, etc.

### 5.2 IsoMapPack5 overrides

After the initial fill, `Read_Map_Section_And_IsoMapPacks` decodes `[IsoMapPack5]` from the `.map` file via `FUN_0056BAC0` (the LCW-compressed IsoMapPack5 unpacker). Each unpacked cell entry rewrites:
- `cell.IsoTileTypeIndex` (`+0x38`)
- `cell.SubTileIndex` (`+0x11A`)
- `cell.Level` (`+0x11B`)
- `cell.field_0x4C` indirectly via the recalc pass

This means **the only cells that remain at the initial-fill state are the cells the editor did NOT touch**. In a typical retail map, the editor exports every cell into IsoMapPack5, so the fill pass is essentially overwritten cell-by-cell. The fill matters as a *fallback* for any cell missing from IsoMapPack5 (rare).

### 5.3 The random-map generator path — `FUN_0059A6C0` (not skirmish)

`FUN_0059A6C0` is the RMG terrain shaper, called only from `FUN_00598960` (the random-map-build top-level). It does the inverse of `Fill=Water`:

1. Set every cell's `IsoTileTypeIndex` to `g_WaterSet` (single tile, no randomization).
2. Call one of three terrain generators based on map level: `FUN_0059AD10` (plain), `FUN_0059AFA0` (hills), `FUN_0059B200` (mountains).
3. Walk every cell again. For each cell still equal to `g_WaterSet`, check if **all four cardinal neighbors are Clear tiles**. If yes, convert this isolated water cell to `IsoTileTypeIndex = 0` (g_ClearTile slot 0).
4. Mark all bridges for repair.
5. Cleanup pass on dynamic vectors.
6. For each `ShorePieceTile` (LandType=Beach range), walk its 4 cardinal neighbors; any that are still Clear tiles get rewritten to `g_GreenTile` (so the LAT later draws grass-to-shore transitions cleanly).

**Tiny detail:** the cardinal-neighbor loop uses `iVar6 += 2; while (iVar6 < 8)`, so steps are `0, 2, 4, 6` indexing `g_DirectionOffsets` — N/E/S/W. The same N/E/S/W bit assignment used by LAT. Diagonals are not checked when "demoting" isolated water back to clear or "promoting" shore neighbors to green.

**RMG is not run in normal skirmish/multiplayer.** It's only invoked from the World Map / mission-pick UI. Skirmish maps come from `.map` files via the `Read_Map_Section_And_IsoMapPacks` path above.

---

## 6. RecalcAttributes — how a cell *gets* `LandType=2`

`CellClass::RecalcAttributes @ 0x0047D2B0` is called from `MapClass::InitCellAttributes` (load-time, all cells) and 30+ runtime mutation paths (overlay place/remove, bridge damage, terraform, building sell, etc.). It writes `cell.LandType` (and triggers LAT + RecalcZoneType).

### 6.1 The water path

The function's main path for a tile-only cell (no relevant overlay, slope 0):

```c
// Path: OverlayTypeIndex == -1, or overlay is non-special
iVar9 = FUN_00544BE0(this->Height);       // TMP terrain_type → LandType lookup
this->LandType = iVar9;
```

Water cells fall through this branch: `FUN_00544BE0` reads the TMP cell's `terrain_type=9` byte → returns `LandType=2`. Done. `cell.LandType = 2`.

### 6.2 Overlay-special branches (NOT water)

The early-return-after-overlay path at the top of the function:

```c
iVar13 = *(int *)(iVar9 + 0x298);              // overlay.LandType
this->LandType = iVar13;
if (((iVar13 == 4) || (iVar13 == 9)) ||         // Wall (4) or Railroad (9) overlay
    (*(char *)(iVar9 + 0x2ac) != '\0')) {       // or +0x2AC flag set
    // ... ApplyLAT, RecalcZoneType, return
}
```

**Important interpretation correction:** the comparison `== 4 || == 9` here is **Wall || Railroad**, NOT Water || Railroad. Earlier doc notes that read this as "Water (4)" were anchored to the wrong enum. The CellClass + 0xEC LandType=2 for water is the *only* value water carries; no overlay produces a LandType=4 water effect in stock content.

The flag at `OverlayTypeClass + 0x2AC` is some "flat overlay" bool (probably "IsSmudge" or "IsLayerOnly") — when set, the overlay's LandType applies but the cell still does LAT/zone recalc. Not water-related.

### 6.3 Overlay → Tiberium → LandType=5

In the no-special-overlay branch but where the overlay is in the Tiberium overlay range:

```c
iVar9 = CellClass__OverlayToTiberiumIndex();
if (iVar9 != -1 && this->SlopeIndex < 5) {     // slope 1..4 (flat ramp)
    if (*(int *)(overlay + 0x298) == 0) {       // overlay declares Clear
        this->LandType = 5;                      // override to Tiberium
    }
}
else if (iVar9 != -1) {                          // tiberium overlay on cliff (slope 5..8)
    this->LandType = FUN_00544BE0(...);           // restore TMP LandType
    this->OverlayTypeIndex = -1;                 // REMOVE the ore — can't sit on cliff
    this->field_0x11e = 0;
}
```

Tiberium overlays placed on water tiles would technically pass through, but `IsCellSuitableForBuilding`-style placement validators reject ore placement on water cells before this code runs. Tiberium can never settle on water in normal play.

### 6.4 Tube/Tunnel detection (LandType=10)

```c
if ((this->LandType == 10) &&
    (IsoTileTypeIndex in [Tunnels..+3] or [TrackTunnels..+3] or
     [DirtTunnels..+3] or [DirtTrackTunnels..+3])) {
    pvVar10 = operator_new(0x1C4);
    TubeClass__Constructor(...);
}
```

`LandType=10` is **Tunnel** (not Wall). Water tiles never have LandType=10. This branch is only hit when a tile in one of the four tunnel tilesets is loaded.

### 6.5 Weather mode interaction (rules + 0x664)

The end-of-function weather conversion:

```c
if (*(char *)(g_RulesClass_Instance + 0x664) == '\x02') {
    if ((LandType == 0) || (LandType == 2) || (LandType == 6) || (LandType == 8)) {
        this->LandType = 3;     // Rock
    }
}
```

In **weather mode 2** (some kind of "ice/frost" map mode, gated by `[Rules] WeatherCondition=` or similar — `RulesClass + 0x664`), cells with LandType Clear (0), **Water (2)**, Beach (6), or Ice (8) get rewritten to Rock (3). **This converts open water to rock during the most extreme weather mode.** Probably TS legacy and not used in any stock YR scenario — but a parity-careful port should preserve it.

Weather mode 0 (default, normal) and mode 1 leave water alone.

---

## 7. RecalcZoneType — LandType=2 → ZoneType=4

`CellClass::RecalcZoneType @ 0x00483C80` writes `ZoneType` to `cell + 0x4C` (u32). Ghidra has annotated the function with the full enum:

| ZoneType | Meaning | Trigger |
|---|---|---|
| 0 | Ground | Default — passable, no special handling |
| 1 | Road | Overlay has IsCrate flag (`+0x22D`) |
| 2 | Wall | Overlay has IsWall flag (`+0x2A8`) |
| 3 | Beach | `LandType == 6` |
| **4** | **Water** | **`LandType == 2`** |
| 5 | Building | BuildingClass occupant present (with conditions) |
| 6 | Impassable | `speed_table[LandType*9 + Wheel] <= 0.01` |
| 7 | OOB | `Is_Cell_In_Playfield(coord, 1)` returned false |

The water and beach paths are direct:

```c
iVar3 = this->LandType;
if (iVar3 == 2) {
    *(undefined4 *)&this->field_0x4c = 4;    // ZoneType = Water
    return;
}
if (iVar3 == 6) {
    *(undefined4 *)&this->field_0x4c = 3;    // ZoneType = Beach
    return;
}
```

**Tiny detail — `+0x2A8` is `IsWall`, NOT `IsWater`.** Some earlier docs flagged this as a water overlay flag. The Ghidra comment in the function header and the code itself prove it sets `ZoneType=2 (Wall)`. There is no `IsWater` overlay flag in stock YR; water comes only from the tile (LandType=2 path).

**Tiny detail — impassable check uses Wheel column.** The expression `(&DAT_0089EA48)[LandType * 9]` is offset 8 bytes (2 × float) into the speed table row, which corresponds to column 2 = Wheel SpeedType. So "impassable" means "Wheel can't traverse this terrain at >0.01 speed". This is the generic fallback for Rock/Wall LandTypes when no overlay/building is set.

**Tiny detail — water bypasses the impassable check.** The `LandType == 2 → return 4` branch runs *before* the impassable speed check. So even though Wheel can't drive on water (`speed_table[2*9 + 2 Wheel] = 0%` per `[Water] Wheel=0%`), water cells get ZoneType=Water, not ZoneType=Impassable. This is critical: it's what distinguishes water from rock in passability — only `MovementZone::Water` and amphibious zones are allowed on `ZoneType=4`.

### 7.1 The passability matrix view

From the 13×8 matrix at `DAT_0082A594` (column 4 = Water ZoneType):

| MovementZone | Col 4 (Water) | Can enter water? |
|---|---|---|
| 0 Normal | 2 (blocked) | No |
| 1 Crusher | 2 (blocked) | No |
| 2 Destroyer | 2 (blocked) | No |
| 3 AmphibiousDestroyer | 1 (pass) | Yes (SEAL, Tanya, Yuri Prime) |
| 4 AmphibiousCrusher | 1 (pass) | Yes |
| 5 Amphibious | 1 (pass) | Yes (SAPC, Robot Tank) |
| 6 Subterranean | 2 (blocked) | No |
| 7 Infantry | 2 (blocked) | No |
| 8 InfantryDestroyer | 2 (blocked) | No |
| 9 Fly | 1 (pass) | Yes (aircraft) |
| **10 Water** | **1 (pass)** | **Yes (all ships)** |
| 11 WaterBeach | 1 (pass) | Yes (mod-only) |
| 12 CrusherAll | 2 (blocked) | No |

7 of 13 zones pass water cells. The shore-crossing problem (you can't go water → ground in one step) is solved by the Beach ZoneType=3 column, not the Water column — see `WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md` for that detail.

---

## 8. `[Water]` terrain section — the speed values

From `ini/rulesmd.ini` lines 30233-30242:

```ini
[Water]
Foot=0%
Track=0%
Wheel=0%
Hover=100%
Float=100%
FloatBeach=100%
Amphibious=100%
Buildable=no
```

When loaded into `DAT_0089EA40 + row(2) * 36`, row 2 contains:

| Column | SpeedType | rules.ini key | Stored value |
|---|---|---|---|
| 0 | Foot | `Foot=0%` | `0.0f` |
| 1 | Track | `Track=0%` | `0.0f` |
| 2 | Wheel | `Wheel=0%` | `0.0f` |
| 3 | Hover | `Hover=100%` | `1.0f` |
| 4 | (slot, hardcoded) | — | `1.0f` |
| 5 | Float | `Float=100%` | `1.0f` |
| 6 | Amphibious | `Amphibious=100%` | `1.0f` |
| 7 | FloatBeach | `FloatBeach=100%` | `1.0f` |
| 8 | Buildable | `Buildable=no` | `0` (byte) |

**Tiny details:**

1. **`Hover=100%` is in the `[Water]` section.** Hover units do not slow down on water. The hover speed multiplier table treats water as fully traversable, identical to clear ground.
2. **`Buildable=no`** is interpreted by `IsCellSuitableForBuilding` (`FUN_0047C620`). Normal buildings (SpeedType=-1) check the Buildable column → `speed_table[LandType*9 + 8] = 0 → cell rejected`. WaterBound buildings (SpeedType=5=Float) bypass this and check the Float column instead → `speed_table[LandType*9 + 5] = 1.0 → cell accepted`. The clever reuse means there's no separate water-placement code path; it's all the same speed table.
3. **`FloatBeach=100%`** in `[Water]` — `FloatBeach` units (a TS-era SpeedType, no stock unit uses it) move full-speed on open water. This is consistent with `[Beach] FloatBeach=` which is also 100%, making FloatBeach the symmetric "water+beach hover" of Hover.
4. **No `Tiberium=` or `Weeds=` line** in `[Water]` — those columns get 0 (default), meaning ore can't spread *through* water cells via the propagation algorithm. (Already enforced by `AllowTiberium=false` on the Water tileset itself, this is a second line of defense.)

---

## 9. Water rendering — variant selection, LAT, animations

### 9.1 No per-frame animation on open water

The `[Water]` tileset has NO `Tile%dAnim=` keys in any theater INI (verified via INI scan). This means the TMP files for `Water01.tem` through `Water14.tem` are the entire visual surface area — no attached `AnimType` cycles, no runtime frame rotation.

The 14 water tiles cover slight visual variations (slightly different ripple patterns, deep/shallow shading), and the per-cell variant picker selects exactly one per cell at load time.

**Tiny detail — water variants are deterministic per cell.** `CellClass::GetTileVariantIndex @ 0x004814F0` is a pure function of (cell_X, cell_Y, sub_tile, tile_width, tile_height, variant_count). The seed comes from `Random__Next()` only once at first call (lazy-init of a permutation table), so all clients in a lockstep multiplayer game compute identical variants. **The same cell shows the same water ripple shape for the entire match on every machine.**

### 9.2 LAT exemptions involving water

`CellClass::ApplyLAT_and_SlopeFixup` (per the IsometricTileTypeClass report §4.3) has hardcoded exemption ranges. For the **Green** group (the LAT that handles grass-to-anything-else transitions), the exemptions are:

- `[g_ShorePieces, g_ShorePieces + 0x29]` — 42 shore tiles
- `[g_WaterBridge, g_WaterBridge + 1]` — 2 water-bridge tiles

**There is NO Green exemption for `g_WaterSet`.** That means when a grass cell neighbors a water cell, the Green LAT *will* detect the neighbor as "not green" and switch the grass cell to a LAT variant. The visual result is a hard grass→water edge unless an explicit ShorePieces tile is placed between them.

This is why every retail map uses an explicit ring of shore tiles around water — without them, you'd see harsh grass-meets-water seams. The shore tiles, being in `[ShorePieces, ShorePieces+42]`, satisfy the Green LAT exemption: grass cells next to shore are treated as if the neighbor were green, suppressing the transition.

**Tiny detail — the WaterBridge exemption is exactly 2 tiles (`+ 1` is inclusive)**, not the full WaterBridge tileset. Per the INI scan, the WaterBridge tileset has `TilesInSet=2`, so the exemption covers the entire set.

### 9.3 Variant range used for `Fill=Water` is only 4 of 14

The fill code calls `Random_RandomRanged(0, 3)` (inclusive bounds → 4 results). So `Fill=Water` only ever picks tiles in `[g_WaterSet, g_WaterSet + 4)`. The remaining 10 tiles in the set are reserved for explicit IsoMapPack5 placement (animator/editor-chosen).

**Why 4?** Most likely because the first 4 water tiles in the TMP set are the "uniform open water" variants — fillable everywhere without context. Tiles 4-13 likely contain edge-of-water graphics or special multi-cell variants that wouldn't tile correctly via random placement. (Verifying that would require reading the actual `Water01.tem`-`Water14.tem` files; the binary doesn't encode this rationale.)

### 9.4 Waterfall animations (NOT open water)

The waterfall tilesets (`Waterfalls`, `Waterfalls-B`, `Waterfalls-C`, `Waterfalls-D` in Temperate; tileset indices 30, 49, 50, 51) DO have `Tile%dAnim=` keys. From `temperatmd.ini` lines 1245-1260:

```ini
[Waterfalls]
Tile01Anim=WA01X
Tile01XOffset=-30
Tile01YOffset=59
Tile01AttachesTo=0
Tile01ZAdjust=0
Tile02Anim=WA02X
...
```

And from `artmd.ini` (lines 18670+), the `[WA01X]`, `[WB01X]`, etc. anim definitions:

```ini
[WA01X]
Flat=yes
LoopStart=0
LoopEnd=8
LoopCount=-1            ; infinite loop
Rate=320                ; 320 ms per frame ≈ 3.125 fps
ShouldUseCellDrawer=true
Theater=yes
Normalized=yes
StartSound=WaterfallLoop
```

These anims attach to specific subtiles of waterfall tilesets, looping forever at 3 fps. They are spawned by `CellClass::RecalcAttributes` when `tile_type + 0x2C8` names an AnimType and the cell's **IsoSubTileIndex** equals `tile_type + 0x2D4` (`Tile%dAttachesTo`). This field is not compared with cell height.

**Tiny detail — Tile1 of every directional set starts `WaterfallLoop`.** `WA01X`, `WB01X`, `WC01X`, and `WD01X` each declare the StartSound; `*02X` through `*04X` do not. The sound is anchored to the first animated tile of each direction.

### 9.5 Wake animation — not part of the water tile, but rendered on water

Wake animations (`[WAKE1]`, `[WAKE2]` in `artmd.ini` lines 14888-14908) are NOT attached to water tiles. They are spawned dynamically by `ShipLocomotionClass::Process` (§11) at the unit's location.

```ini
[WAKE1]
Flat=yes
Layer=ground
Translucent=yes
Rate=120
YSortAdjust=-288
DemandLoad=true
DetailLevel=2
```

**Tiny detail — `[WAKE2]` has `YSortAdjust=-64` vs `[WAKE1]`'s `-288`**. Y-sort difference of 224 pixels = a wake that appears further behind the unit. Stock rules.ini uses `Wake=WAKE1`; `;Wake=WAKE2` is commented out as an alternative.

---

## 10. Cell layout — water-relevant fields

Verified offsets on `CellClass`:

| Offset | Size | Field | Notes |
|---|---|---|---|
| `+0x24` | `i16×2` | MapCoord (X, Y) | |
| `+0x38` | `u32` | `IsoTileTypeIndex` | The tile id; water cells have value in `[g_WaterSet, g_WaterSet+14)` |
| `+0x4C` | `u32` | `ZoneType` | Written by `RecalcZoneType`. Water cells = 4. |
| `+0xEC` | `u32` | **`LandType`** | Written by `RecalcAttributes`. Water cells = 2. |
| `+0x11A` | `u8` | `SubTileIndex` | Sub-cell index for multi-cell templates; water tiles are 1×1 so always 0 |
| `+0x11B` | `u8` | `Level` | Z-elevation step (15 px per step). Water is at `Level=0` in stock maps. |

**Tiny detail — LandType and ZoneType are both u32.** Despite the name "byte enum", both fields are read with `*(int *)(cell + 0xEC) == 2` and `*(int *)(cell + 0x4C) = 4` in the live binary. So the actual storage is 32-bit, not the 8-bit one might infer from the enum range. This matters for Rust ports: the struct layout must use `u32`/`i32`-aligned fields here.

**Tiny detail — `IsoTileTypeIndex` is `u32` too**. Even though it never exceeds 65535 in practice (`u16` is enough), the field is loaded with `*(int *)(cell + 0x38)`. The early-out comparison `cell.IsoTileTypeIndex == 0xFFFF` (the "empty" sentinel) is a *u32 == 0xFFFF*, which works correctly because 0xFFFF is small.

---

## 11. Wake animation — the only runtime "motion" on water

`ShipLocomotionClass::Process @ 0x0069FC10` is the only function in the binary that spawns wake animations. The full wake conditional, lifted verbatim from the decompilation:

```c
cVar4 = (**(code **)(*piVar3 + 0x80))(piVar3);    // Is_Moving? (vtable+0x80)
if (cVar4 != '\0') {
    uVar6 = g_CurrentFrameCounter & 0x80000007;
    bVar11 = uVar6 == 0;
    if ((int)uVar6 < 0) {
        bVar11 = (uVar6 - 1 | 0xfffffff8) == 0xffffffff;
    }
    if (bVar11                                                              // (A) frame % 8 == 0
        && (iVar5 = vtable_call(techno, 0x84),                              // get TechnoTypeClass
            *(char *)(iVar5 + 0xd69) == '\0')                               // (B) type.field_0xd69 == 0
        && ((char)((int *)piVar3[2])[0x23] == '\0')                         // (C) techno.field_0x8C == 0
        && (iVar5 = vtable_call(techno, 0x1bc),                             // get current cell
            *(int *)(iVar5 + 0xec) == 2                                      // (D) cell.LandType == 2
            && (*(int *)(g_RulesClass_Instance + 0x94) != 0))                // (E) Rules.Wake != null
        && (pvVar8 = operator_new(0x1c8), pvVar8 != NULL))                   // alloc Anim
    {
        iStack_10 = *(int *)(piVar3[2] + 0xa0);                              // techno world X
        iStack_c = *(int *)(piVar3[2] + 0xa4);                               // techno world Y
        // iStack_8 left from earlier (Z)
        AnimClass__Constructor(
            *(undefined4 *)(g_RulesClass_Instance + 0x94),    // type = Rules.Wake
            &stack0xffffffec,                                  // location = (X, Y, Z)
            0, 1, 0x600, 0, 0
        );
    }
}
```

### 11.1 Five required conditions for a wake to spawn

1. **(A) Frame % 8 == 0** — the bit-AND-with-`0x80000007` is MSVC's signed-modulo-by-8 idiom. Wakes appear once every 8 game ticks (~0.5 seconds at 15 fps). Different from Drive (which uses `IDIV 10` — wakes every 10 frames for ground vehicles that drive on water via mod content). Ship uses the bitwise optimization because `8 = 2^3`.
2. **(B) `TechnoTypeClass + 0xD69 == 0`** — a per-type byte flag. Per the naval doc, when non-zero this is a "deployed" or "submerged" mark. SUB and BSUB likely have this set when submerged (so submarines don't leave a wake while diving). Surface ships (DEST, AEGIS, CARRIER, DRED, HYD) have it zero.
3. **(C) `techno + 0x8C == 0`** — the OnBridge byte. Ships transitioning onto/off a bridge tile have this non-zero for a few frames, suppressing wake during the transition. Otherwise this stays 0 for normal water travel.
4. **(D) `cell.LandType == 2`** — the cell the ship is currently in must be water. This is the canonical "ship is on water" test.
5. **(E) `Rules.Wake != null`** — the `[General] Wake=WAKE1` parsed AnimType pointer at `RulesClass+0x94`. If `Wake=` is missing or `Wake=none`, no wakes are ever spawned game-wide. Default = `WAKE1`.

### 11.2 Spawn position and arguments

- Position is `(techno.X, techno.Y, ...)` from `techno + 0xA0` and `+0xA4`. **Z is whatever was left on the stack from earlier**, which in practice is the previous cell's Z. So wakes spawn at the *unit's exact sub-cell world coords*, not at the cell center. This means wakes appear staggered along the unit's wake trail because each spawn is at a slightly different sub-cell position.
- AnimClass args: `(wake_type, &coords, 0, 1, 0x600, 0, 0)`.
  - The `0x600` (= 1536) is likely an `OwnerHouse` flag / palette mask — probably "neutral" or "fixed palette". Not the brightness (brightness in YR animations is typically in different units).
  - The `1` is `looped=true` per the AnimClass signature.
  - Final `0, 0` are extras (light source overrides) not relevant to wakes.

### 11.3 Tiny details

- **The wake spawn happens AFTER pathfinding/movement is complete**, inside the post-movement branch (LAB_0069FE39). So the wake is at the unit's POSITION AFTER moving, not before.
- **The condition order in the decompile is short-circuit AND**: tests run left to right. The cheap tests (frame check, deployed flag, on-bridge flag) come before the expensive `vtable+0x1BC` cell lookup. Smart short-circuiting.
- **`g_CurrentFrameCounter` is the game tick** — same value used for synchronization across lockstep clients. Wake spawning is therefore deterministic.
- **DriveLocomotionClass::Process** has a parallel block that spawns wakes every 10 frames (via IDIV-by-10, not the bitwise `& 7` trick). The condition `cell.LandType==2` is the same. So if you mod a tank to enter water, it would leave wakes too — at 10-frame cadence instead of 8.
- **No wake on hover units.** HoverLocomotionClass doesn't have a wake spawn block. Hovercraft (SAPC) crossing water leave no wake — they have their own "skirt" animation handled by the visual layer.

---

## 12. Building placement on water — `IsCellSuitableForBuilding`

`FUN_0047C620` at `0x0047C620` is called per foundation cell during placement. The water-specific check (lifted from the naval doc and verified via the speed table column logic):

```c
// param_2 = BuildingTypeClass.SpeedType field (+0x67C)
//   -1 = normal building (no WaterBound)
//    5 = WaterBound=yes (SpeedType=Float)

if (param_2 == -1) {
    // Normal building
    if (no_bridge_flag && no_ramp_flag && no_subcell_flag) {
        if (BuildingType.IsNaval == false) {   // +0xCCE
            // Check Buildable column: speed_table[LandType*9 + 8]
            return speed_table[cell->LandType * 0x24 + 0x20];  // DAT_0089EA60 (col 8)
        }
        // Naval=yes but no WaterBound: check shore tile range (14 tiles)
        if (cell->TileSet >= shore_start && cell->TileSet < shore_start + 14) {
            return 1;
        }
        return 0;
    }
} else {
    // WaterBound (SpeedType=5)
    // Check Float column: speed_table[LandType*9 + 5]
    if (speed_table[param_2 + cell->LandType * 9] != 0.0f) {
        return 1;
    }
}
return 0;
```

**For a water cell (LandType=2):**

- **Normal building** (Buildable column, row 2): `speed_table[2*9 + 8] = 0` (since `[Water] Buildable=no`) → cell rejected.
- **WaterBound building** (Float column, row 2): `speed_table[2*9 + 5] = 1.0` (since `[Water] Float=100%`) → cell accepted.

The same speed table that gates unit movement gates building placement. **No separate water-placement code path.**

**Tiny detail — the 14-tile range check for Naval=yes (non-WaterBound) buildings.** This is the `cell->TileSet >= shore_start && cell->TileSet < shore_start + 14` branch — it uses the same 14-tile range as `IsOnBridgeSurface`. The implication: a `Naval=yes` building without `WaterBound=yes` (an odd combination — none in stock content) would place only on the first 14 water tiles, not on the full water set. This is presumably TS legacy and irrelevant for stock YR (which always pairs Naval+WaterBound on naval yards).

---

## 13. Sinking — death on water

When a unit dies (mission state machine transition to case 4), and the unit's death cell has `LandType==2` (water), the engine triggers a sinking sequence. Detailed in `SUBMARINE_AND_SINKING_GHIDRA_REPORT.md`:

- `techno + 0x3CD = IsSinking` byte set to 1.
- `techno + 0x3CD` becomes the "death sinks" master gate.
- Sinking visual: per-frame `AngleRotatedForwards += 0.01 rad` until max `PI/4` (~45°) tilt.
- `WaterlineY` (techno+0x3CA) acts as a screen-space Y clip mask, hiding the lower portion of the ship sprite as it tilts.
- `H2O_EXP1`, `H2O_EXP2`, `H2O_EXP3` splash anims spawn randomly during the sink.
- Sound: `SinkingSound=GenLargeWaterDie` (rules `[General]` line 656). 9 naval unit entries also explicitly set `SinkingSound=GenLargeWaterDie`.
- After tilt completes, the techno is removed from the world (no wreck/debris on water).
- `Weight >= ShipSinkingWeight` (default 3.0 from `[General]`) is the threshold; lighter units (infantry, dolphin) don't sink with the heavy tilt — they just disappear.

**Tiny detail — tilt direction is based on the unit's facing octant at death.** Per the sinking doc:
- Facing octants {0, 6, 7} → negative tilt (left/forward)
- Facing octants {1, 2, 3, 4, 5} → positive tilt (right/backward)

This makes the ship list to the appropriate side relative to its travel direction.

---

## 14. Theater-by-theater summary

| Theater | WaterSet | ShorePieces | WaterCliffs | WaterCaves | WaterBridge | Waterfalls (E/W/N/S) | Notes |
|---|---|---|---|---|---|---|---|
| Temperate | 21 | 12 | 15 | 57 | 76 | 49/51/50/30 | Full water content |
| Snow | 21 | 12 | 15 | – | – | 35/37/36/30 | No WaterCaves or WaterBridge defined |
| Urban | 21 | 12 | 15 | 57 | – | 49/51/50/30 | No WaterBridge defined (but tileset 76 is loaded) |
| Desert | 21 | 12 | 15 | 57 | 76 | 49/51/50/30 | Full water content |
| Lunar | 21 | 12 | 15 | 57 | 76 | 49/51/50/30 | **All nulled at runtime** — Lunar branch in theater loader zeros these globals |

**Tiny detail — Lunar declares WaterSet=21 in INI but the engine zeros the resolved global anyway** in the theater terminator's interior-branch. The INI line is a no-op for Lunar.

**Tiny detail — Snow uses different waterfall tileset numbers** (35-37 vs 49-51) because Snow has fewer preceding terrain sets, shifting the index. The TMP files referenced (`W-b-`, `W-c-`, `W-d-`) are identically named per direction — only the tileset number used to reference them in `[General]` differs.

---

## 15. Active in YR — Yes / No / Conditional

| Subsystem | Active in YR? | Trigger frequency |
|---|---|---|
| `WaterSet` tile range | Yes — except Lunar | Every coastal map, every cell-resolution call |
| `LandType==2` water classification | Yes — except Lunar | Every cell load, every overlay change |
| `ZoneType==4` water zone | Yes — except Lunar | Every recalc; per-cell at map load |
| `IsOnBridgeSurface` (14-tile range) | Yes | Naval yard placement validation |
| `Read_Map_Section_And_IsoMapPacks` Fill=Water path | Conditional | Only on maps with `[Map] Fill=Water` (a minority — most maps fill Clear and add water explicitly) |
| Wake animation in Ship Process | Yes | Every 8 ticks per moving ship on water |
| `RecalcAttributes` weather conversion to Rock | Conditional | Only if `RulesClass+0x664 == 2` (extreme weather mode — no stock scenario uses this) |
| LAT Green exemption for ShorePieces / WaterBridge | Yes | Every LAT pass on Green-group cells next to water |
| RMG `FUN_0059A6C0` water-fill | No | RMG is not called during skirmish/multiplayer — only world-map missions |
| Lunar zero-out of WaterSet | Conditional (Lunar only) | Once at theater load on Lunar maps |
| Sinking on water-death | Yes | Every naval unit death on water (case 4 of death state machine) |
| Waterfall animations (`WA01X`-`WD04X`) | Yes | Spawned by `RecalcAttributes` when a waterfall tileset cell is loaded |
| OverlayType `IsWater` flag check | **No (does not exist)** | The `+0x2A8` flag is `IsWall`, not `IsWater` — water is purely tile-based |

---

## 16. Current Rust Implementation Status

Mapped against `src/`:

| System | File:line | Status |
|---|---|---|
| LandType enum (binary value 2 → Rust value 4) | `src/sim/pathfinding/passability.rs:39-48` | **Remapped** — intentional architectural divergence. Rust `LandType::Water = 4` in an 8-column matrix. Documented in NAVAL_SYSTEM_RESEARCH §8. Functional outputs match. |
| TMP terrain_type byte → LandType | `src/sim/pathfinding/passability.rs:79-92` | **Present.** TMP byte 9 → `LandType::Water` per `tmp_terrain_to_land_type`. |
| `WaterSet` global / tile range | `src/map/theater.rs:180-196` `is_water()` | **Partial.** Detects water tiles by SetName string-match ("water" substring), not by direct WaterSet tile_id range. Functionally equivalent for stock content where TileSet 21's SetName is literally "Water". A mod that renames the tileset would silently break this. |
| `ShorePieces` / 42-tile shore range | `src/map/lat.rs:160-186` | **Present** — used as a string-match LAT exemption (Green vs "ShorePieces" pair). |
| `WaterBridge` / 2-tile range | `src/map/lat.rs:160-186` | **Present** — same string-match approach. |
| `[Water]` terrain rules | `src/rules/terrain_rules.rs:211-221` `built_in_semantics` | **Present.** Sets `buildable=false, ground_blocked=true, water=true`. |
| `RecalcAttributes` → set LandType | `src/map/resolved_terrain.rs:1099-1167` | **Partial.** Land type derived once at map load; no runtime recompute on overlay/tile change. The Rust port treats LandType as static-per-cell. |
| `RecalcZoneType` (LandType=2 → ZoneType=4) | (implicit in passability matrix) | **Not separately modeled.** Rust uses the LandType column directly without a ZoneType layer. Functionally equivalent because the matrix lookup `passable[movement_zone][land_type]` produces the same answer. |
| `[Map] Fill=Water` initial-fill | `src/map/map_file.rs` IsoMapPack5 parser | **Partial.** Rust reads IsoMapPack5 cells but does NOT honor the `[Map] Fill=` key for cells missing from IsoMapPack5. In practice retail maps populate every cell, so this is rarely a visible gap. |
| `IsOnBridgeSurface` 14-tile range | `src/sim/pathfinding/core.rs:961-1000` `is_water_surface_cell_passable` | **Partial.** Rust uses the `is_water` SetName-match flag, not the 14-tile range. Different mechanism, same outcome for stock content. |
| Wake animation every 8 ticks | `src/sim/world/mod.rs:1136-1182` | **Present.** `if self.tick & 7 == 0 { ... }` matches the binary's `frame & 0x80000007 == 0` modulo. Filters on `is_water_mover()` and `movement_target.is_some()`. **Missing checks**: TechnoType +0xD69 (deployed/submerged), techno +0x8C (OnBridge). These are deferred but should be added for parity. |
| Wake AnimType from rules | `src/sim/world/mod.rs:1136-1182` | **Present.** Reads `rules.general.wake.name` (parsed as `WAKE1` default). |
| Wake position from unit world coords | `src/sim/world/mod.rs:1136-1182` | **Different.** Rust uses `(CELL_CENTER_LEPTON, CELL_CENTER_LEPTON)` sub_x/sub_y — cell-center, not unit's exact sub-cell position. **Parity gap:** binary spawns wake at `techno + 0xA0` (sub-cell precise), Rust spawns at cell center. The wake trail will look slightly different (stepped instead of smooth). |
| Sinking on water | (deferred — phase 5 of naval impl plan) | **Missing.** No `IsSinking` flag, no tilt animation, no water clipping mask. |
| Waterfall `Tile%dAnim=` | `src/map/theater.rs`, `src/map/resolved_terrain.rs`, `src/app_init_helpers.rs`, `src/sim/anim_class.rs`, `src/render/sprite_atlas.rs` | **Present.** Rust parses/resolves the SetName blocks, binds theater-extension assets with the ISO/cell palette, spawns the final post-object AnimClass set with signed infinite-loop semantics, preserves Ground/Top ordering and TileAnimZAdjust, and drains `WaterfallLoop` once after load. |
| Lunar theater null-out of water globals | (implicit — no water tilesets to detect) | **Equivalent.** Lunar's INI declares WaterSet=21 but the SetName-match for "water" never produces water cells because Lunar's TMPs don't have terrain_type=9 anywhere. The Rust port reaches the same end-state via different mechanism. |
| `Fill=Water` default 4-of-14 random | (none) | **Missing.** Per §5.3, Rust does not honor the `[Map] Fill=` key. |
| `[Map] Level=` baseline offset | (none) | **Likely missing.** Rust applies the IsoMapPack5 Z directly; no `Fill`-time baseline offset is added. |
| Building placement on water (`WaterBound`) | `src/sim/production_placement.rs` | **Partial.** Rust has the structural plumbing but uses `is_water` flag rather than the binary's Float-column speed-table lookup. Equivalent for stock buildings. |

---

## 17. Open Questions

1. **The 4-of-14 fill range** — why are only the first 4 water tiles used for random fill? Probably because tiles 4-13 are non-uniform variants that wouldn't tile randomly. Verifying would require inspecting `Water01.tem` through `Water14.tem` TMP graphics. Low priority for parity (the result is "fill picks one of 4 ripple variants").

2. **`TechnoType + 0xD69`** — the wake-suppression flag. Confirmed to suppress wake when non-zero. Suspected to be the "Deployed" / "Submerged" flag for submarines while diving. Not directly extracted in this pass; needed for full wake parity. Cross-reference with submarine cloak state.

3. **AnimClass constructor arg `0x600`** in the wake spawn — likely a flags/palette/owner mask. Not critical for visual parity since the AnimType (`WAKE1`) carries all the rendering data; the wake will look right regardless.

4. **`RulesClass + 0x664`** weather mode — what triggers value 2? No stock YR scenario sets this. Possibly TS legacy. Worth confirming via INI grep that no current scenario uses it.

5. **`Random_RandomRanged` PRNG state during `Fill=Water`** — the random fill draws from the same PRNG used everywhere else. Lockstep guarantees alignment, but if Rust uses a different RNG order for `Fill=`, water tile selection across the map could differ. Only matters if Rust ever implements `Fill=Water`.

6. **Per-direction water tile graphics** — are any of the 14 water tiles direction-specific (e.g., "northward-rippling water" vs "southward-rippling water")? The TMP file metadata would tell us. Not investigated this pass.

7. **`Tile%dAttachesTo`** semantics for waterfalls — VERIFIED 2026-08-14. The field at `IsometricTileTypeClass + 0x2D4` is compared unsigned to `CellClass::bIsoSubTileIndex`; it selects the specific subtile within a multi-cell template. It is not a height comparison.

8. **Animated water on the GAYARD/NAYARD/YAYARD foundation** — naval yards have `AmbientSound=_Amb_WavesLake`. Is there an associated visual ambient (water ripples?), or is it purely audio? Probably audio-only based on the INI keys.

---

## Sources

**Ghidra functions decompiled this pass:**
- `0x0047D2B0` — `CellClass::RecalcAttributes` (full body)
- `0x00483C80` — `CellClass::RecalcZoneType` (full body)
- `0x00485060` — `CellClass::IsOnBridgeSurface` (full body, 4 lines)
- `0x00544BE0` — TMP terrain_type → LandType lookup (full body)
- `0x00544C20` — IsCellValid helper (full body)
- `0x004ACE70` — `Read_Map_Section_And_IsoMapPacks` (relevant top half — `[Map] Fill=` reader + IsoMapPack5 dispatch)
- `0x0059A6C0` — RMG water-fill driver (full body)
- `0x0059AD10` — RMG level-0 generator (partial; structural only)
- `0x0069FC10` — `ShipLocomotionClass::Process` (full body — wake check is the critical block at LAB_0069FE39)

**Memory tables read directly:**
- `0x00839D68..0x00839D98` — 12 terrain section name pointers
- `0x0081BAE8` — "Water" string (confirmed, ascii)
- `0x0081DC1C, 0x0081DC14, 0x0081DC0C, 0x0081AC58, 0x00817278, 0x0081DC04, 0x0081DBFC, 0x0081DBF8, 0x0081DBEC, 0x0081DBE4, 0x0081DBDC` — Clear, Road, Rock, Wall, Tiberium, Beach, Rough, Ice, Railroad, Tunnel, Weeds (in that order)
- `0x008288E4..0x00828924` — 16-entry terrain_type → LandType u32 table
- `0x00820170` — `"Fill"` key name string

**XRef analyses:**
- xrefs to `DAT_00AA0738` (g_WaterSet): 23 callers across `IsCellSuitableForBuilding`, RMG generators, theater load (writes), bridge mask/variant selection, Read_Map_Section_And_IsoMapPacks
- xrefs to `0x00485060` (`IsOnBridgeSurface`): primary caller is `Cell_passability_building_placement` at `0x0047CA12`
- xrefs to `0x0081BAE8` ("Water" string): 4 data refs including `Read_Map_Section_And_IsoMapPacks`, terrain section name pointer table, terrain block at `0x0081BAB0`

**INI files cross-referenced** (in-repo):
- `ini/rulesmd.ini` — `[Water]` terrain section line 30233, Wake at rules.ini line 519
- `ini/temperatmd.ini`, `ini/snowmd.ini`, `ini/urbanmd.ini`, `ini/desertmd.ini`, `ini/lunarmd.ini` — `[General] WaterSet=`, `ShorePieces=`, `WaterCliffs=`, `WaterCaves=`, `WaterBridge=`, `WaterfallEast/West/North/South=`; TileSet0021 (Water) per theater; TileSet0012 (Shore Pieces); waterfall tilesets; `[Waterfalls]` master block with `Tile%dAnim=` keys at lines 1245-1260
- `ini/artmd.ini` — `[WAKE1]` and `[WAKE2]` (lines 14888-14908), `[WA01X]`-`[WD04X]` waterfall anims (lines 18670-18881)

**Companion docs cross-referenced:**
- `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` — TileSet load, LAT, MarbleMadness, variant chain, TMP layout
- `WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md` — Beach LandType=6, ShorePieces 42-tile range, AmphibiousDestroyer/Amphibious zones
- `NAVAL_SYSTEM_RESEARCH.md` — ShipLocomotionClass, naval units, building placement
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` — Cell field offsets
- `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` — RecalcZoneType decomp
- `SUBMARINE_AND_SINKING_GHIDRA_REPORT.md` — sinking state machine
- `ZONE_PASSABILITY_VERIFIED.md` — 13×8 passability matrix
- `MOVEMENT_CLASSIFIERS_REFERENCE.md` — MovementZone enum, ZoneType enum

**Conflicts resolved this pass:**
- **`LandType` value for Water:** Multiple older docs claimed Water=4 (from a mislabeled enum in `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` §5). The actual binary value is 2 (Water at slot 2 in the terrain section name table; live consumer code `cell.LandType == 2` in Ship Process). The 16-entry TMP table at `0x008288E4` was correctly read in §13 of the IsoTileTypeClass doc but the named labels in §5 were wrong — `terrain_type=9 → LandType=2` means Water, not Road.
- **`OverlayType + 0x2A8` flag:** Earlier docs called it `IsWater`. Ghidra's annotation in `RecalcZoneType` confirms it's `IsWall` (sets ZoneType=Wall=2). There is no IsWater overlay flag in stock YR.

---

*End of report. The "sea tile" story is, in one sentence: a `CellClass` whose `IsoTileTypeIndex` is in `[g_WaterSet, g_WaterSet + 14)` and whose `LandType` (derived from TMP byte 9 → table lookup → 2) drives a ZoneType=4 (Water) classification, gating passability to naval/amphibious zones only, with wake animations spawned every 8 ticks on movement and no per-frame water graphic cycling.*
