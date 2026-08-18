---
title: LAT Groups & Slope Fixup — Ghidra Research Report
---

# LAT Groups, Exemption Ranges & Slope Fixup — Ghidra Research Report

**Primary addresses:**
- `CellClass::ApplyLAT_and_SlopeFixup` @ `0x0047CA80` — full algorithm
- `TheaterData::LoadTileSets` (labeled `CDFileClass__Constructor`) @ `0x005455B5` — theater-INI parser + global init
- `IsometricTileTypeClass::TMP_ReadSlopeType` @ `0x005471B0` — slope byte fetch
- `FUN_00544C80` @ `0x00544C80` — lazy TMP loader (called post-LAT if tile changed)

**Confidence:** HIGH. Every claim below is read directly from the disassembly of these four functions.

**Active in YR:** Yes — LAT runs on every cell at map load and on every cell mutation at runtime.

---

## 1. Scope

This report closes the two remaining LAT gaps from the prior coverage audit:

1. **All LAT group exemption tables verified from binary** — previously only Pave was fully covered.
2. **`ApplyLAT_and_SlopeFixup` internals** — verified by direct decompile. The slope-fixup half was only peripherally documented before.

Also includes the theater-INI key/global mapping (lines read at theater load) and the map-edge sentinel behavior.

---

## 2. The LAT algorithm — complete, verified

### 2.1 Processing order

`ApplyLAT_and_SlopeFixup` processes **four LAT groups in strict fixed order**:

```
Rough → Sand → Green → Pave → (Slope fixup)
```

Each group runs independently. A cell whose tile falls in multiple group ranges is processed by each matching group in order; the last match wins (because each block writes `cell.IsoTileTypeIndex`).

In practice ranges are disjoint — but the order still matters for map-edge and partially-initialized cases.

### 2.2 Per-group skeleton (identical logic for all 4)

```python
def lat_pass(cell, base_tile, lat_base, lat_max, *exemptions):
    # Match: cell is in this group?
    if cell.tile != base_tile and not (lat_base <= cell.tile <= lat_max):
        return   # not this group's concern

    mask = 0
    for i, dir in enumerate([N, E, S, W]):           # dirs 0, 2, 4, 6
        neighbor = map.get_cell(cell.coord + g_DirectionOffsets[dir])
        nt = neighbor.IsoTileTypeIndex
        # Set bit i if neighbor is NOT in this group's range
        # AND not in any exempted range
        if (nt != base_tile and not (lat_base <= nt <= lat_max)
            and all(not (exlo <= nt <= exhi) for (exlo, exhi) in exemptions)):
            mask |= 1 << i

    if mask == 0:
        cell.IsoTileTypeIndex = base_tile                 # isolated — pure base tile
    else:
        cell.IsoTileTypeIndex = lat_base + mask            # variant 1..15
```

**Bit assignment — verified from decompile:**

| Bit | Direction | Offset idx in `g_DirectionOffsets` | Table entry |
|-----|-----------|------------------------------------|-------------|
| 0 | N | 0 | (0, −1) |
| 1 | E | 2 | (+1, 0) |
| 2 | S | 4 | (0, +1) |
| 3 | W | 6 | (−1, 0) |

Loop iterates `uVar12 = (uVar12 + 2) & 7`, so the cycle is 0 → 2 → 4 → 6 → 0. Only the 4 cardinals — no diagonals.

### 2.3 Group-specific parameters (VERIFIED — all four)

Read directly from the decompiled constants at the head of `ApplyLAT_and_SlopeFixup`:

| Group | Base tile | LAT variant range | Exemption ranges | Enable guard |
|-------|-----------|-------------------|------------------|--------------|
| **Rough** | `g_RoughTile` | `g_ClearToRoughLat .. g_ClearToRoughLat + 0xF` | **none** | always runs (no `!= -1` guard) |
| **Sand** | `g_SandTile` | `g_ClearToSandLat .. g_ClearToSandLat + 0xF` | **none** | `g_ClearToSandLat != -1` |
| **Green** | `g_GreenTile` | `g_ClearToGreenLat .. g_ClearToGreenLat + 0xF` | `g_ShorePieces .. g_ShorePieces + 0x29` (42 tiles), `g_WaterBridge .. g_WaterBridge + 1` (2 tiles) | `g_ClearToGreenLat != -1` |
| **Pave** | `g_PaveTile` | `g_ClearToPaveLat .. g_ClearToPaveLat + 0xF` | `g_MiscPaveTile .. g_MiscPaveTile + 0xD` (14 tiles), `g_Medians .. g_Medians + 0xD` (14 tiles), `g_PavedRoads .. g_PavedRoads + 0x14` (21 tiles) | `g_ClearToPaveLat != -1` |

Each exemption range individually guards `!= -1`; if the global is `-1` (disabled), its range local is forced to `-1` and the `<= -1` check always fails, effectively skipping that exemption.

**Asymmetry: Rough has no `!= -1` guard.** The Rough block always executes. Sand, Green, and Pave have explicit `if (iVar{X} != -1)` outer guards so they skip entirely when their `ClearTo*Lat` global is undefined in the theater. Rough will still try to process but all its range checks will fail (since `-1 <= tile <= -1+0xF == 14` is false for any real tile_id ≥ 0) — so it's effectively self-disabling too.

### 2.4 Rationale for the exemptions (inferred from ranges, not from code comments)

- **Green exempts ShorePieces (42 tiles) and WaterBridge (2 tiles):** shoreline transitions and water-bridge connectors contain pixels that blend into grass, so when a grass cell's neighbor is shore/water-bridge, the engine treats that neighbor "as if it were grass" (no edge). The grass LAT run continues through the shoreline without drawing a grass-edge variant.
- **Pave exempts MiscPaveTile (14 tiles), Medians (14 tiles), and PavedRoads (21 tiles):** these are all "pave-like" tiles that shouldn't trigger pave-edge variants when adjacent to a pave cell. The pavement LAT treats any adjacent road, median, or misc-pave as a continuous paved surface.

Rough and Sand have no exemptions because there are no "rough-like" or "sand-like" transition tiles in the stock theaters — you either have that ground type or you don't.

### 2.5 Why `*ConnectTo` INI keys aren't involved

The Rust implementation currently parses `*ConnectTo` INI keys (flagged in `PAVEMENT_AND_TILE_PROPAGATION_GHIDRA_REPORT.md` as incorrect). **Verified:** there is no `ConnectTo` parsing anywhere in `ApplyLAT_and_SlopeFixup`, nor in the theater-INI loader. The exemption ranges are entirely derived from the theater-INI `[General]` `PavedRoads=`, `MiscPaveTile=`, `Medians=`, `ShorePieces=`, `WaterBridge=` tileset pointers and the hardcoded `+0xF`/`+0xD`/`+0x14`/`+0x29` lengths in the decompile.

---

## 3. Slope fixup — the second half of the function

After the 4 LAT passes, `ApplyLAT_and_SlopeFixup` runs slope fixup on cells whose tile is now in the ramp range.

### 3.1 Guard

```python
tile = cell.IsoTileTypeIndex
if not ((g_RampBase <= tile <= g_RampBase + 0x13) or
        (g_RampSmooth <= tile <= g_RampSmooth + 0xB)):
    goto LAB_0047d1cf   # skip slope fixup entirely
```

Ramp base covers 20 variants (0x14). Ramp smooth covers 12 (0xC). So slope fixup only touches cells already known to be ramp tiles — it doesn't try to promote a flat tile into a ramp.

### 3.2 Per-slope-index logic

`cell.field_0x11C` holds the **slope type byte** (0 = flat, 1..4 = four cardinal slope orientations). This byte is baked into the TMP per-cell data at offset `+0x2A` and fetched by `TMP_ReadSlopeType` (see §5). `RecalcAttributes` calls `TMP_ReadSlopeType` and writes the result to `SlopeIndex` before calling `ApplyLAT_and_SlopeFixup`.

For each non-zero `SlopeIndex`, the function checks **two specific perpendicular neighbors** and builds a 2-bit mask:

| Slope | Neighbor A (bit 0) | Neighbor B (bit 1) | Formula on mask != 0 |
|-------|--------------------|--------------------|----------------------|
| 1 | W | E | `RampSmooth + (mask − 1)` → 0..2 |
| 2 | N | S | `RampSmooth + mask + 2` → 3..5 |
| 3 | E | W | `RampSmooth + mask + 5` → 6..8 |
| 4 | S | N | `RampSmooth + mask + 8` → 9..11 |

Each bit is set if the checked neighbor has `SlopeIndex == 0` (flat neighbor).

If `mask == 0` (both checked neighbors are also sloped), falls through to the fallback:

```python
cell.IsoTileTypeIndex = g_RampBase + (SlopeIndex − 1)
```

So the fallback maps slopes 1..4 to RampBase+{0,1,2,3}. The 20-tile-long RampBase range (0x14 variants) has room for all 4 orientations plus 16 LAT-style variants, but the code here only uses the first 4.

### 3.3 `RampSmooth` block layout (derived)

The `mask + offset` formulas produce these deterministic mappings:

| Slope | mask=1 | mask=2 | mask=3 |
|-------|--------|--------|--------|
| 1 (W/E) | `RampSmooth + 0` | `RampSmooth + 1` | `RampSmooth + 2` |
| 2 (N/S) | `RampSmooth + 3` | `RampSmooth + 4` | `RampSmooth + 5` |
| 3 (E/W) | `RampSmooth + 6` | `RampSmooth + 7` | `RampSmooth + 8` |
| 4 (S/N) | `RampSmooth + 9` | `RampSmooth + 10` | `RampSmooth + 11` |

So the RampSmooth tile set has exactly 12 variants arranged as 4 slope orientations × 3 "flat-neighbor" configurations (A only / B only / both). That matches the `+0xB` (12 tiles) upper bound on the guard.

### 3.4 Final side effect

```python
if old_tile != cell.IsoTileTypeIndex:
    if new_tile_type.vtable[0x9C]() == 0:       # TMP data not loaded
        FUN_00544c80(new_tile_type)              # lazy-load TMP
return (old_tile != cell.IsoTileTypeIndex)       # "did tile change" → caller uses for dirty-mark decisions
```

`FUN_00544C80` @ `0x00544C80` — gated lazy TMP loader. Only loads if `tile_type + 0xA4 == 0` (no TMP cached) AND `tile_type + 0x2F4 != 0` (filename was found on disk at init). Returns whether TMP is now loaded. This is the on-demand TMP load path for tile sets that weren't loaded at startup.

---

## 4. Theater-INI parser — `0x005455B5` (labeled `CDFileClass__Constructor` — MISLABELED)

**Ghidra labeling error.** The function at `0x005455B5` is labeled `CDFileClass__Constructor`, but it's actually the **theater initialization and tileset loader**. Called at theater load with `param_1` = theater index (0..5), it:

1. Reads the theater INI's `[General]` section — all the tile-pointer keys below.
2. Loads every TileSet (`[TileSetNNNN]` sections), assigning each a base IsoTileTypeIndex.
3. For each tileset, checks whether its index matches any of the `[General]` pointers; if so, snapshots the first tile's IsoTileTypeIndex into the corresponding global.
4. Loads TMP files for each tile.

### 4.1 All LAT-relevant `[General]` keys read

From the decompile, read from `[General]` of the active theater INI (`TEMPERAT.INI` / `SNOW.INI` / `URBAN.INI` / etc.):

| INI key | Global | Purpose |
|---------|--------|---------|
| `ClearTile` | `g_ClearTile` | Base tile for the clear/grass layer |
| `RoughTile` | `g_RoughTile` | Rough LAT base |
| `SandTile` | `g_SandTile` | Sand LAT base |
| `GreenTile` | `g_GreenTile` | Green LAT base |
| `PaveTile` | `g_PaveTile` | Pave LAT base |
| `MiscPaveTile` | `g_MiscPaveTile` | Pave exemption 1 (misc-pave tiles) |
| `ClearToRoughLat` | `g_ClearToRoughLat` | Rough LAT variant range base (16 tiles) |
| `ClearToSandLat` | `g_ClearToSandLat` | Sand LAT variant range base |
| `ClearToGreenLat` | `g_ClearToGreenLat` | Green LAT variant range base |
| `ClearToPaveLat` | `g_ClearToPaveLat` | Pave LAT variant range base |
| `ShorePieces` | `g_ShorePieces` | Green exemption 1 (42 shore tiles) |
| `WaterBridge` | `g_WaterBridge` | Green exemption 2 (2 water-bridge tiles) |
| `PavedRoads` | `g_PavedRoads` | Pave exemption 3 (21 road tiles) |
| `Medians` | `g_Medians` | Pave exemption 2 (14 median tiles) |
| `RampBase` | `g_RampBase` | Ramp base tile range (20 tiles) |
| `RampSmooth` | `g_RampSmooth` | Ramp smooth variant range (12 tiles) |
| `MMRampBase` | `DAT_00AA109C` | Marble Madness ramp base (for `MarbleMadness=yes` tileset overrides) |

Every global defaults to `-1` if the INI key is absent or the tileset index doesn't exist.

### 4.2 Other `[General]` keys read in the same pass (for context — non-LAT)

`BridgeSet`, `WoodBridgeSet`, `CliffSet`, `WaterSet`, `SlopeSetPieces`, `SlopeSetPieces2`, `MonorailSlopes`, `Tunnels`, `TrackTunnels`, `DirtTunnels`, `DirtTrackTunnels`, `WaterfallEast/West/North/South`, `CliffRamps`, `PavedRoadEnds`, `RoughGround`, `DirtRoadJunction/Curve/Straight/Slopes`, `DestroyableCliffs` (default `-2`, not `-1`), `WaterCaves`, `WaterCliffs`, `PavedRoadSlopes`, `Rocks`, `HeightBase`, `BlackTile`, plus 10 `BridgeTopLeft1..BridgeMiddle2` keys.

**`DestroyableCliffs` defaults to `-2`** (all other keys default to `-1`). This is a sentinel value — when the theater INI specifies `DestroyableCliffs=no` or omits the key entirely, a `-2` probably means "explicitly disabled" vs `-1` meaning "not present in this theater's INI". Behavioral impact is downstream of this report.

### 4.3 Snow-theater override

When theater index `== 5` (Snow), after all tilesets are loaded:

```python
g_ShorePieces       = -1
DAT_00aa0738        = -1   # WaterSet
DAT_00aa1020        = -1   # CliffSet
DAT_00aa101c        = -1   # WaterCliffs
g_WaterBridge       = -1
DAT_00aa0e28        = -1   # BridgeSet
DAT_00abad1c        = -1   # WoodBridgeSet
```

Snow maps disable all shore/water/bridge/cliff-related globals. This means snow maps can't have bridges or shorelines. That's actually consistent with stock YR — snow is the polar/arctic theater and doesn't have water bodies.

**YR impact:** the Green LAT group's exemptions effectively disable themselves in snow theater (since `g_ShorePieces` and `g_WaterBridge` become -1). Snow grass LAT has no exemptions at all.

---

## 5. TMP slope type — `TMP_ReadSlopeType` @ `0x005471B0`

```c
int TMP_ReadSlopeType(IsometricTileTypeClass* this, int sub_tile_idx) {
    int* tmp_header = this->vtable[0x9C]();    // fetch TMP data header
    if (tmp_header == null) return 0;
    int cell_idx = sub_tile_idx % (tmp_header[1] * tmp_header[0]);   // % (width*height)
    int cell_data = tmp_header[4 + cell_idx];
    if (cell_data == 0) return 0;
    return (int)*(char*)(cell_data + 0x2A);      // signed byte at +0x2A
}
```

**TMP per-cell byte `+0x2A` = slope type** (0 = flat, 1..4 = cardinal slope). Baked into the `.tem` / `.sno` / `.urb` TMP files at authoring time.

Cross-reference: the prior `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` §11.3 mentions the TMP cell flag at `+0x24` (bit 2 = `FLAG_HAS_DAMAGED_DATA`). `+0x2A` is the adjacent slope-type byte — they live in the same per-cell struct. Full TMP cell struct relevant fields:

| Offset | Type | Purpose |
|--------|------|---------|
| `+0x00..+0x23` | mixed | pixel data / metadata (see isometric tile report) |
| `+0x24` | u32 flags | bit 0 = `HAS_EXTRA_DATA`, bit 1 = `HAS_Z_DATA`, bit 2 = `HAS_DAMAGED_DATA` |
| `+0x2A` | i8 | **slope type** (0=flat, 1..4=slope orientations) |

---

## 6. Map-edge sentinel — `DAT_00abdc50`

### 6.1 Read of static binary image

```
00abdc50: 00000000 00000000 00000000 00000000
00abdc60: 00000000 00000000 00000000 00000000
00abdc70: 00000000 00000000 00000000 00000000
00abdc80: 00000000 00000000 00000000 00000000
```

All zero. The sentinel is zero-initialized. Runtime may or may not modify other fields, but for LAT purposes only `+0x38` (IsoTileTypeIndex) and `+0x11C` (SlopeIndex) matter.

### 6.2 Effect on LAT

`MapClass::Get_CellClass(off_map_coord)` returns `&DAT_00abdc50`. Its `IsoTileTypeIndex` reads as 0.

For a LAT group whose `base_tile != 0` (all 4 LAT groups have base tiles far above 0 — ClearTile is 0, but that's a separate group not in the LAT loop), the edge-check evaluates:

- `0 != base_tile` → TRUE
- `0 < lat_base` → TRUE (lat_base is usually also > 0)
- `0 < exemption_low` → TRUE for all exemption ranges

Result: **every off-map neighbor sets its bit in the LAT mask.** A cell at the map edge always gets the "bordered" variant in the direction of the edge.

This is the correct and intended behavior — a Rough cell at the east edge of the map should show its east-edge variant (bit 1 set), not act like it has a Rough neighbor to the east.

### 6.3 Effect on slope fixup

Sentinel `+0x11C = 0` → treated as a flat neighbor → sets the bit. So a ramp cell at the map edge whose off-map side is checked gets the "smooth" variant (RampSmooth range) as if the neighbor were flat ground. Also intended.

---

## 7. Invocation & retrigger chain (cross-reference)

Established in `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`:

```
[71 callers] → CellClass::RecalcAttributes (0x0047D2B0)
                   → TMP_ReadSlopeType  (refresh cell.SlopeIndex)
                   → ApplyLAT_and_SlopeFixup  (this report)
                   → RecalcZoneType
                   → (zone-cache write-back)
```

`ApplyLAT_and_SlopeFixup` is a **leaf function** in this chain — it only reads the globals set by the theater loader and writes `cell.IsoTileTypeIndex`. No side effects on other cells (the flood-fills live in `SetOverlayAndPropagate` and `ToggleBridgePavement`, not here).

---

## 8. Current Rust implementation status — corrected against binary

Rust: [src/map/lat.rs](src/map/lat.rs).

| Aspect | Rust today | Binary truth | Gap |
|--------|-----------|--------------|-----|
| Source of exemption ranges | `*ConnectTo` INI key parsing | Hardcoded in `ApplyLAT_and_SlopeFixup` | **Wrong source — needs rewrite** |
| LAT groups | partial | 4 groups: Rough, Sand, Green, Pave | Verify all 4 present in Rust |
| Rough exemptions | (whatever the `ConnectTo` path produces) | **none** | Remove exemptions for Rough |
| Sand exemptions | (likewise) | **none** | Remove exemptions for Sand |
| Green exemptions | (likewise) | ShorePieces+42, WaterBridge+2 | Hardcode these 2 ranges |
| Pave exemptions | (likewise) | MiscPaveTile+14, Medians+14, PavedRoads+21 | Hardcode these 3 ranges |
| Processing order | unknown | Rough → Sand → Green → Pave | Must match for determinism |
| Bit assignment | unknown | N=0, E=1, S=2, W=3 | Verify |
| Slope fixup | missing | RampBase+RampSmooth 2-neighbor picker | Needs implementation (blocks cliffs/ramps parity) |
| Map-edge handling | unknown | off-map forces bit set | Verify |
| `ClearToXxxLat` disable via -1 | unknown | Sand/Green/Pave disable if base is -1; Rough runs regardless | Match |
| Lazy TMP load on tile change | missing | `FUN_00544C80` after tile change | Deferred — not strictly required for parity if all TMPs are pre-loaded |

### 8.1 Minimal correctness checklist for the Rust port

1. **Delete `*ConnectTo` parsing** in `src/map/lat.rs`. These keys don't exist in the binary's LAT.
2. **Parse the theater-INI `[General]` globals directly:** `RoughTile`, `SandTile`, `GreenTile`, `PaveTile`, `MiscPaveTile`, `ClearToRoughLat`, `ClearToSandLat`, `ClearToGreenLat`, `ClearToPaveLat`, `ShorePieces`, `WaterBridge`, `Medians`, `PavedRoads`, `RampBase`, `RampSmooth`. All default to `-1` (Option::None).
3. **Hardcode the 5 exemption ranges:**
   - Green: `[ShorePieces, ShorePieces+0x29]`, `[WaterBridge, WaterBridge+1]`
   - Pave: `[MiscPaveTile, MiscPaveTile+0xD]`, `[Medians, Medians+0xD]`, `[PavedRoads, PavedRoads+0x14]`
4. **Process groups in order Rough → Sand → Green → Pave.**
5. **Cardinal-only neighbor check** (4 bits, N/E/S/W at offsets 0/2/4/6 of the 8-dir table).
6. **Map-edge sentinel:** return a cell with IsoTileTypeIndex == 0 or a sentinel that fails all group matches. (Current Rust's behavior needs audit.)

### 8.2 Slope fixup priority

Slope fixup is only meaningful when the map has ramp tiles (cliffs/height transitions). Stock YR maps do have these, so parity requires it — but the impact is visual on cliff ramps only, not on flat gameplay. Can be deferred until destructible cliffs or cliff-traversal work is in scope.

### 8.3 RMG in-generation LAT port (2026-07-20)

**Re-verified `0x0047CA80` live** (`decompile_function`) — this Apr-24 report matches the binary exactly: 4 groups in order, cardinal offsets `[0,2,4,6]` stepping `(uVar12+2)&7`, `cell.tile = lat_base + mask` (direct, no remap table), the exemption ranges in §2.3, Rough unguarded / Sand·Green·Pave `!= -1` guarded, and the tile-0 map-edge sentinel forcing edge bits.

The RMG runs this fixup between LAT-patch painting and tree scatter (`FUN_005a3ae0`'s `ApplyLAT_and_SlopeFixup` cell-iterator loop). Ported faithfully as **`src/map/rmg/phases/lat_fixup.rs`** (`run(grid, ids)`, RNG-free, 12 tests) — the LAT half only. This is a **separate implementation** from the map-load `src/map/lat.rs` (which still uses the isometric-diamond mask + `*ConnectTo` path flagged in §8; note the two are algebraically equivalent for the *offset* — the `MASK_TO_LAT_INDEX` remap is exactly the permutation from the diamond bit-convention to the binary's direct cardinal mask — but differ on exemptions and map-edge handling). `WaterBridge` was added to `RmgTileKeys`/`TileIds` for Green's exemption.

**Slope-fixup half deferred** in the RMG port: it dispatches on the `+0x11C` slope-type byte (0..4), which is not the port's `GridCell.slope` (0..18 ramp-variant index). Resolving the slope-type source needs its own RE pass. It is RNG-free, so deferral cannot desync the generator's draw stream.

---

## 9. Summary of new findings (beyond prior reports)

1. **`ApplyLAT_and_SlopeFixup` decompile is direct** — previously the algorithm description came partly from inference. Now every constant, exemption range, and branch is read out of the binary.
2. **Rough and Sand have NO exemptions.** Prior reports implied or were silent on this. Confirmed: those two groups are simple 4-cardinal LAT with no special-cases.
3. **Green has exactly 2 exemptions:** ShorePieces (+0x29 = 42 tiles) and WaterBridge (+1 = 2 tiles).
4. **Pave has exactly 3 exemptions:** MiscPaveTile (+0xD = 14 tiles), Medians (+0xD = 14 tiles), PavedRoads (+0x14 = 21 tiles). **PavedRoadEnds and PavedRoadSlopes are NOT LAT exemptions** — they're parsed as separate theater globals but never referenced by the LAT algorithm. Prior reports listed them alongside, which was inaccurate.
5. **Processing order is Rough → Sand → Green → Pave**, strictly sequential.
6. **Rough is unguarded** — runs without a `!= -1` check on its Clear-to-lat global. The other 3 groups have explicit guards.
7. **Slope fixup is a 4-state dispatch** on `cell.SlopeIndex` (1..4), each state picks 2 perpendicular neighbors and produces 3 RampSmooth variants + 1 RampBase fallback. Total RampSmooth = 12 tiles (3 × 4 orientations), RampBase = 20 tiles (only first 4 used by fixup; rest presumably for other slope paths).
8. **TMP slope byte lives at per-cell offset +0x2A** (signed byte). Baked into the TMP file at authoring time.
9. **Theater-INI parser at `0x005455B5` is mislabeled `CDFileClass__Constructor`.** It's the theater/tileset loader. A rename to `TheaterData__LoadTheater` or similar would be valuable for future work — I did not apply a Ghidra rename this pass because there's some uncertainty about whether the naming should reflect its role as loader vs constructor-of-tileset-cache (the function does both).
10. **Snow theater forcibly disables shore/water/bridge/cliff globals** after tileset load. So Green LAT in snow has no exemptions — which matches the fact that snow theaters have no shorelines.
11. **Map-edge sentinel at `DAT_00abdc50` is zero-initialized.** Its effective tile_id of 0 correctly fails all 4 LAT group matches, so off-map neighbors always set their LAT bit. This produces correct map-edge variants "for free".
12. **`DestroyableCliffs` defaults to -2, not -1.** All other theater globals default to -1. The -2 is a distinct sentinel that downstream code presumably distinguishes from "not set".

---

## 10. Open questions / remaining gaps

1. **`DestroyableCliffs = -2` vs `-1` meaning** — not investigated. Relevant for when cliff destruction lands.

2. **The 20-tile `RampBase` range** — only the first 4 variants (slope 1..4) are referenced by `ApplyLAT_and_SlopeFixup`'s fallback. The remaining 16 tiles are presumably used by a different path (possibly the slope variant of LAT, or directly by authored ramp placements in map files). Not investigated.

3. **Why does `FUN_00544C80` call `TMP_Loader()` only when `cell.vtable[0x9C]() == 0` succeeds post-change?** The condition is "if tile type exists but TMP isn't cached". This suggests some tilesets are lazy-loaded on first display — verify with a runtime trace when porting.

4. **`MMRampBase`** — the Marble Madness ramp base. Set from INI at `0x008295c4`. Not used by `ApplyLAT_and_SlopeFixup` (only `g_RampBase` and `g_RampSmooth` are). Likely an alternate ramp set for Marble Madness-flagged TileSets. Tangential.

5. **Snow-theater index `== 5`** — wasn't verified as actually being the snow theater. The theater index mapping (0=Temperate, 1=Snow, 2=Urban, 3=New Urban, 4=Lunar, 5=Desert?) would need a theater-ID-table lookup. The override behavior (disabling bridges/water) sounds more like Lunar than Snow to me — Lunar has no water. Worth confirming but orthogonal to LAT correctness.

---

## Sources

**Ghidra addresses decompiled this pass:**
- `0x0047CA80` — `CellClass::ApplyLAT_and_SlopeFixup` (full algorithm)
- `0x005455B5` — `TheaterData::LoadTileSets` (labeled `CDFileClass__Constructor`)
- `0x005471B0` — `IsometricTileTypeClass::TMP_ReadSlopeType`
- `0x00544C80` — `FUN_00544C80` (lazy TMP loader, post-LAT tile-change hook)

**String search:** 14 matches on LAT-related ASCII strings (`RoughTile`, `GreenTile`, `SandTile`, `PaveTile`, `MiscPaveTile`, `ClearToRoughLat`, `ClearToSandLat`, `ClearToGreenLat`, `ClearToPaveLat`, `ShorePieces`, `WaterBridge`, `RampBase`, `RampSmooth`, `MMRampBase`). All resolved to INI reads inside `0x005455B5`.

**Memory read:** `DAT_00abdc50` — 64 bytes, all zero in static image (map-edge sentinel).

**Cross-referenced docs:**
- `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` — TMP cell struct, variant picker interaction, CellClass Flags 0x2000
- `PAVEMENT_AND_TILE_PROPAGATION_GHIDRA_REPORT.md` — RecalcAttributes, tile flood-fill, ConnectTo-path flagged as wrong
- `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md` — the full retrigger pipeline; this doc is the leaf function

**Rust source audited:**
- [src/map/lat.rs](src/map/lat.rs) (load-only, uses `*ConnectTo` INI — needs rewrite)
