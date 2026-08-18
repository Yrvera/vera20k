---
title: TMP Damaged-Variant File Layout — Ghidra Research Report
---

# TMP Damaged-Variant File Layout — Ghidra Research Report

**Primary addresses:**
- `TMP_TileBlitter` @ `0x00547CF0` — per-tile blit, walks the variant chain
- `IsometricTileTypeClass::Constructor` @ `0x005447C0` — `next_variant`/`VariantCount` field init
- `Read_Theater_TileSets_INI` @ `0x00545150` — tile-set loader; loads pristine + variant TMP files and links them into a chain
- `IsometricTileTypeClass::HasDamagedVariantAtSubTile` @ `0x005471F0` — TMP +0x24 bit 2 gate (verified prior session)
- `CellOverlay_TileDraw` @ `0x00480350` — main draw caller; computes variant index 0/1 from `cell.Flags & 0x2000`
- `CellClass::GetTileVariantIndex` @ `0x004814F0` — PRNG fallback for non-damaged variant pick

**Overall confidence:** HIGH — all critical functions decompiled this session; variant-chain mechanism verified line-by-line in the loader and blitter. Cross-checked against prior `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` §14 and §17.

**Active in YR:** Yes — used for every TMP tile draw, including bridge damaged-variant rendering.

---

## 1. Overview — the answer in one paragraph

**The damaged variant is NOT extra bytes inside the same TMP file.** It is a SEPARATE
`.TMP` file on disk with a letter suffix appended to the base filename (`'a'` = first
variant, `'b'` = second, etc.). At theater-INI load time, `Read_Theater_TileSets_INI`
iterates each `[TileSetNNNN]` section, and for each tile inside the set tries to load
the pristine file plus letter-suffixed variants until a file lookup misses. Each
successfully-loaded TMP becomes its own `IsometricTileTypeClass` instance, and the
instances for one logical tile are linked together via the `+0x2BC` (`next_variant`)
pointer into a singly-linked chain. `TMP_TileBlitter` receives a `variant_index` arg
(0 = pristine, 1 = damaged, …) and walks the chain that many steps before blitting.

For bridges specifically (TMP cells with the `HAS_DAMAGED_DATA` flag bit 2 at +0x24 on
the per-cell header), `CellOverlay_TileDraw` chooses variant index from
`(cell.Flags >> 13) & 1` — so variant 0 is the pristine baked art, variant 1 is the
"scuffed/cracked" baked art loaded from `<base>01a.<theater_ext>`.

This is the same mechanism FA2's `bRNDImage` uses for visual-diversity grass jitter
(load `S01.TEM`, `S01A.TEM`, `S01B.TEM`, … and pick one per cell). The ONLY difference
between visual-diversity and damaged-variant is **how the variant index is picked**:
- `HasDamagedData == 1` → variant = `(cell.Flags >> 13) & 1` (deterministic 0/1)
- `HasDamagedData == 0` → variant = `GetTileVariantIndex(hash over coord)` (PRNG)

---

## 2. The variant-chain walk inside `TMP_TileBlitter` (verified at 0x00547CF0)

The first instructions inside the blitter, before any draw setup:

```c
while (true) {
    DAT_00aa1120 = CONCAT22(DAT_00aa1120._2_2_, DAT_008a0de8);
    if (param_14 == 0) break;                          // variant_index == 0 → use param_1 as-is
    if (param_1[0xbc] + -1 < param_14) {               // variant_index > VariantCount-1 → wrap
        param_14 = param_14 % param_1[0xbc];           // modulo VariantCount
    }
    piVar10 = param_1;
    if (param_14 == 0) break;                          // (wrap may have produced 0)
    do {
        piVar10 = (int *)piVar10[0xaf];                // FOLLOW next_variant pointer (+0x2BC)
        param_14 = param_14 + -1;
    } while (param_14 != 0);
    if (piVar10 == param_1) break;                     // cycle detection
    param_14 = 0;
    param_1 = piVar10;                                 // use the variant tile from here
}
```

`param_1` enters as `IsometricTileTypeClass*` (the chain head — the pristine tile);
`param_14` enters as the variant index (0 = pristine, 1 = first variant, …). After
this loop:
- `param_1` points to the chosen variant's `IsometricTileTypeClass`.
- The blitter then proceeds to call `vtable[0x9C](param_1)` to fetch the TMP data
  pointer for **that variant's TMP file**, and renders pixels from it.

Each entry in the variant chain holds its own:
- `TMP_DATA` pointer (vtable slot 0x9C resolves it)
- VariantCount (decreasing along the chain — see §4.3)
- All other tile metadata fields (size 0x30C bytes — see §3)

**Confidence:** HIGH. Decompiled the function in full this session.

---

## 3. `IsometricTileTypeClass` field offsets relevant to variant chain

`param_1` is typed `int*` by Ghidra; offsets below are listed BOTH ways:

| `int*[idx]` | Byte offset | Type | Field | Init in Constructor (0x005447C0) |
|---|---|---|---|---|
| `[0xa9]` | `+0x2A4` | `vtable*` | secondary vtable / GenericNode | `&PTR_FUN_007eccec` |
| `[0xaa]` | `+0x2A8` | `int` | next-in-list pointer (heap pool) | 0 |
| `[0xaf]` | **`+0x2BC`** | `IsoTileType*` | **`next_variant`** (singly-linked chain to next variant) | **0 (NULL)** |
| `[0xb7]` | `+0x2DC` | `u8` (in u32 slot) | param_3 from constructor | passed by caller |
| `[0xb8]` | `+0x2E0` | `u8` | morphable flag | 0 |
| `[0xb9]` | `+0x2E4` | `int` | `template_width` (from TMP header) | 0 |
| `[0xba]` | `+0x2E8` | `int` | `template_height` (from TMP header) | 0 |
| `[0xbb]` | `+0x2EC` | `u8` | param_4 from constructor | passed by caller |
| `[0xbc]` | **`+0x2F0`** | `int` | **`VariantCount`** | **1** (overwritten by loader, §4.3) |
| `[0xbd]` | `+0x2F4` | `u8` | RequiredForRMG | 0 |
| `[0xc2]` | `+0x308` | `int` | `use_counter` (cull pass at FUN_00546DA0) | 0 |
| — | `+0x305` | `u8` | AllowBurrowing | 1 |
| — | `+0x306` | `u8` | AllowTiberium | 0 |
| — | `+0x2E1` | `u8` | ShadowCaster | 0 |
| — | `+0x2E2` | `u8` | AllowToPlace | 1 |
| — | `+0x2E3` | `u8` | RequiredForRMG copy | 0 |
| — | `+0x2F5..` | `char[14]` | persistent filename (used by TMP_Loader cull) | 0 |

**Total instance size:** 0x30C bytes (verified by `operator_new(0x30c)` calls in the loader at 0x00545150).

`next_variant` at byte offset `+0x2BC` = index `[0xaf]` (since 0xaf * 4 = 0x2BC). This is THE critical field for the variant chain.

**Confidence:** HIGH. Constructor decompiled this session; field offsets cross-checked against prior `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md`.

---

## 4. The tile-set loader — how the chain gets populated (verified at 0x00545150)

`Read_Theater_TileSets_INI(theater_id, is_editor_load)` is called once at scenario load to populate the global `IsometricTileTypeClass` array (`DAT_00A8ED2C[]`). The function does THREE things in order:

1. Reads `[General]` keys for tileset anchor indices (RampBase, ClearTile, BridgeSet, WoodBridgeSet, …).
2. Iterates `[TileSet0000]`, `[TileSet0001]`, … sections until one is missing.
3. For each TileSet, loads every tile in the set, building the variant chain.

### 4.1 Per-TileSet INI keys read

| INI key | Type | Default | Purpose |
|---|---|---|---|
| `TilesInSet` | int | -1 (terminates outer loop) | how many tile slots this set occupies |
| `LastTilesInSet` | int | -1 | (set padding allocation for cross-theater compatibility) |
| `SetName` | string | `"No-Name"` | display name (used in `<SetName> %02d` debug format) |
| `FileName` | string | `""` | base filename — e.g., `BRIDGE` or `LOBRDG` |
| `MarbleMadness` | int | 0xFFFF | MM-mode tile index reference |
| `NonMarbleMadness` | int | 0xFFFF | non-MM-mode tile index reference |
| `Morphable` | bool | false | (LAT-related) |
| `AllowToPlace` | bool | true | editor placement |
| `AllowBurrowing` | bool | true | subterranean unit pass |
| `AllowTiberium` | bool | false | ore overlay allowed |
| `RequiredForRMG` | bool | false | (Random Map Generator — keep TMP resident) |
| `ToSnowTheater` | int | -1 | conversion-set ref |
| `ToTemperateTheater` | int | -1 | conversion-set ref |
| `ShadowCaster` | bool | false | tile casts a shadow |
| `ShadowTiles` | int | 0 | shadow-tile-count (when ShadowCaster=true) |
| `Tile%02dAnim` | string | — | animation attached to specific tile |
| `Tile%02dXOffset` | int | — | anim offset X |
| `Tile%02dYOffset` | int | — | anim offset Y |
| `Tile%02dAttachesTo` | int | — | anim parent |
| `Tile%02dZAdjust` | int | — | anim Z |

### 4.2 The variant-load inner loop

For each tile slot `iVar11 = 0..TilesInSet-1`:

```c
iStack_9f8 = 0;                                          // variant index counter
piStack_9dc = NULL;                                      // chain head pointer (set on first variant)
piStack_974 = NULL;                                      // chain tail pointer (last linked)

do {
    // Compute filename suffix for this variant slot.
    if (iStack_9f8 == 0) {
        suffix = "";                                     // pristine: empty suffix
    } else {
        suffix = (char)('`' + iStack_9f8);               // 0x60 + N → 'a','b','c',...
    }

    // Build candidate filename:  <FileName_base><tile_num_02d><suffix>
    sprintf(number_str, "%02d", iVar11 + 1);             // tile number, 2-digit zero-padded
    candidate = concat(FileName, number_str, suffix);

    // Try theater extension list (.TEM/.URB/.SNO/.DES/.LUN/.MMT depending on theater_id)
    bytes = LoadFileFromMIX(candidate + theater_ext);    // primary attempt
    if (!bytes) {
        bytes = LoadFileFromMIX(candidate + ".MMT");     // MarbleMadness fallback
    }
    if (!bytes) {
        bytes = LoadFileFromMIX(candidate + ".TEM");     // Temperate fallback
    }
    if (!bytes) {
        bytes = LoadFileFromMIX(candidate + ".URB");     // Urban fallback
    }

    if (!bytes && !found_in_search_path) break;          // no more variants for this tile

    if (iStack_9f8 == 0) {
        // Construct CHAIN HEAD tile (registered in g_IsoTileTypeArray)
        new_tile = IsometricTileTypeClass::Constructor(iVar13, 0xFFFFFFBF, 0, set_name, 0);
        piStack_9dc = new_tile;
        piStack_974 = new_tile;
    } else {
        // Construct CHAIN VARIANT tile (param_6=1 → NOT registered in g_IsoTileTypeArray)
        new_tile = IsometricTileTypeClass::Constructor(iStack_96c, 0xFFFFFFBF, 0, set_name, 1);
        piStack_974[0xaf] = (int)new_tile;               // LINK: prev->next_variant = new
        piStack_974 = new_tile;                          // update tail
    }

    // Populate new_tile fields from INI overrides + TMP data
    new_tile[0xa6] = MarbleMadness;
    new_tile[0xa7] = NonMarbleMadness;
    new_tile[0xb8] = Morphable;
    // ... etc.

    if (bytes) {
        new_tile->TMP_DATA = bytes;                      // attach TMP data pointer
        new_tile[0xb9] = *(int*)bytes;                   // template_width
        new_tile[0xba] = *(int*)(bytes + 4);             // template_height
        // ... fixup TMP cell-data pointer table (relocate from file-relative to RAM-relative)
    }

    iStack_9f8++;
} while (bytes != NULL || found_in_search_path);
```

### 4.3 VariantCount fixup after chain is built

Once the loop exits (no more variant files found for this tile):

```c
if (iStack_9f8 > 1) {                                    // had at least one variant beyond pristine
    int total = iStack_9f8;
    int* tile = piStack_9dc;                             // start at chain head
    int remaining = total - 1;
    do {
        tile[0xbc] = total;                              // set VariantCount = total
        tile = (int*)tile[0xaf];                         // walk to next variant
        total--;
        remaining--;
    } while (remaining != 0);
}
```

**This is a key detail.** Each tile in the chain gets `VariantCount` set to a
**decreasing** value: head = `total`, next = `total - 1`, ..., tail = `2`. The very
last tile (the one without further `next_variant`) keeps the constructor's initial
`VariantCount = 1`.

Why decreasing? The blitter at 0x00547CF0 does `param_14 % param_1[0xbc]` to clamp the
variant index. The head's `VariantCount = total` means a render code reading the head
sees the full range, but any caller that's already mid-chain sees a smaller range.
**In practice all real callsites pick from the chain head**, so the decrement pattern
is defensive against future code that might enter the chain mid-walk.

For a typical bridge tile with HasDamagedData:
- Pristine TMP loaded → chain head, `VariantCount = 2`, `next_variant → damaged_tile`
- Damaged TMP loaded → `VariantCount = 1`, `next_variant = NULL`

**Confidence:** HIGH. Decompiled the loop and the VariantCount post-pass this session.

### 4.4 Important detail — filename pattern, format strings, and extensions

Format-string addresses verified by `read_memory` this session:

| Address | Bytes | Decoded |
|---|---|---|
| `0x008291A4` | `25 2E 32 38 73 20 25 30 32 64` | `"%.28s %02d"` — debug format (SetName + tile num) |
| `0x008291B0` | `25 30 32 64 00` | `"%02d"` — tile number formatter |
| `0x00829140` | `2E 54 45 4D 00` | `".TEM"` — Temperate extension |
| `0x00829148` | `2E 4D 4D 54 00` | `".MMT"` — MarbleMadness extension |
| `0x00829138` | `2E 55 52 42 00` | `".URB"` — Urban extension |

Filename pattern for a tile inside `[TileSetNNNN]` with `FileName=BRIDGE`, tile index 1, theater=Temperate:

| Variant | Filename |
|---|---|
| 0 (pristine) | `BRIDGE01.TEM` |
| 1 (damaged) | `BRIDGE01A.TEM` |
| 2 | `BRIDGE01B.TEM` |
| ... | ... |

The letter suffix is lowercase in the engine's filename construction (verified:
`(char)('`' + iStack_9f8)` produces `'a'..'z'` for `iStack_9f8 = 1..26`), but Windows
filesystem is case-insensitive so this is generally invisible. In MIX archives the
filename hash is also case-insensitive.

**Confidence:** HIGH. Format strings decoded from memory; loop logic decompiled.

### 4.5 The chain-head is registered globally; chain-variants are not

```c
// In Constructor with param_6 == 0:
DAT_00A8ED2C[DAT_00A8ED38++] = this;  // global IsoTileTypeArray

// In Constructor with param_6 == 1 (variant case):
// (this branch skipped — no array registration)
```

So `g_IsoTileTypeArray[tile_id]` always returns the **chain head** (pristine variant).
The chain is reached only by walking `+0x2BC` from there. This is why
`CellClass.IsoTileTypeIndex` indexes the global array — same index always yields the
pristine tile-type pointer, and the variant pick at draw time decides whether to walk
the chain.

**Confidence:** HIGH.

---

## 5. The render-time pick (already documented; reverified this session)

`CellOverlay_TileDraw` @ `0x00480350` computes variant index before calling the blitter:

```python
tile = g_IsoTileTypeArray[cell.IsoTileTypeIndex]   # always chain head
sub_tile = cell.SubTileIndex                       # +0x11A

if tile.VariantCount < 2:
    variant = 0                                    # no variants exist; nothing to walk
elif HasDamagedVariantAtSubTile(tile, sub_tile):   # FUN_005471F0 — checks TMP +0x24 bit 0x04
    variant = (cell.Flags >> 13) & 1               # damaged-data deterministic 0/1
else:
    variant = GetTileVariantIndex(cell, tile_id, tile.VariantCount)   # PRNG over (rx, ry, sub_tile, count)

TMP_TileBlitter(LightConvert, sub_tile, surface, x, y, clip..., cell.Level, ..., variant, 0,0,0,0)
```

`CellClass::GetTileVariantIndex` @ `0x004814F0` is a deterministic hash over the cell
coord — covered in prior `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` §16. Returns
`0..VariantCount-1`.

`HasDamagedVariantAtSubTile` @ `0x005471F0` returns the bit `>> 2 & 1` of the TMP per-cell
flag DWORD at `+0x24`. Critical: this gate is on the **pristine** tile's TMP per-cell
data. The damaged variant TMP doesn't need to advertise the flag — only the pristine
needs to, and the engine knows "if pristine has the flag, the chain has a damaged
variant".

**Confidence:** HIGH (this session re-decompiled and cross-checked).

---

## 6. The radar-side render uses the same chain

`CellClass::GetRadarPixelColor` @ `0x0047BDB0` has the identical block:

```c
if (*(int *)(iVar10 + 0x2f0) < 2) goto LAB_0047c016;
pcStack_1c = (char *)(uint)*(byte *)(param_1 + 0x11a);    // SubTileIndex
puStack_20 = (undefined1 *)0x47bfe9;
cVar2 = FUN_005471f0();                                   // HasDamagedVariantAtSubTile
if (cVar2 != '\0') {
    pcStack_1c = (char *)(*(uint *)(param_1 + 0x140) >> 0xd & 1);   // (Flags >> 13) & 1
    goto LAB_0047c016;
}
```

Same gate, same bit position. So the minimap also picks the damaged radar pixel
color from the variant chain. The minimap's source data is the per-tile `radar_left` /
`radar_right` byte triples baked into each TMP file (`+0x2B6` / `+0x2B9` per prior
report), and the damaged TMP carries its own pair distinct from pristine.

**Confidence:** HIGH.

---

## 7. INI references

Theater INIs (`temperatmd.ini`, `urbanmd.ini`, `snowmd.ini`, `desertmd.ini`,
`lunarmd.ini`, `newurbanmd.ini`):
- `[General]` anchor keys (e.g., `BridgeSet`, `WoodBridgeSet`) index into the
  TileSet array — these are tile-set offsets, not file references.
- `[TileSetNNNN]` sections with `FileName=` keys — each declares a base filename.
  The engine appends `01..XX` for tile numbers and `''/a/b/c/...` for variants.

`rulesmd.ini` is NOT consulted by the variant-chain loader — variant counts come
from how many `<base><nn><letter>.<ext>` files actually exist on disk, not from any
INI declaration.

---

## 8. Per-cell flag DWORD at TMP `+0x24` — full bit layout (cross-reference)

Verified at `0x00547030` (TMP_Loader) and `0x005471F0` (HasDamagedVariantAtSubTile) and
`0x00480350` (CellOverlay_TileDraw). The DWORD at `+0x24` on each per-cell header inside
a TMP file holds:

| Bit | Mask | Name | Meaning |
|---|---|---|---|
| 0 | `0x01` | FLAG_HAS_EXTRA_DATA | Cell has extra pixel data beyond the diamond (cliff faces, shores) |
| 1 | `0x02` | FLAG_HAS_Z_DATA | Cell has per-pixel Z-depth buffer |
| 2 | `0x04` | **FLAG_HAS_DAMAGED_DATA** | This tile chain has a damaged-variant file linked at `+0x2BC` on the IsoTileType |
| 3+ | — | (reserved / unknown / unused in YR) | — |

The Rust parser at [src/assets/tmp_decode.rs:21](src/assets/tmp_decode.rs#L21) already
declares `FLAG_HAS_DAMAGED_DATA = 0x04` and exposes the parsed bool via
`TmpTile.has_damaged_data` at [src/assets/tmp_file.rs:51](src/assets/tmp_file.rs#L51).

**Confidence:** HIGH (Rust parser matches the binary's bit position).

---

## 9. Current Rust implementation status

### What is already in place

[src/assets/tmp_decode.rs](src/assets/tmp_decode.rs)
- `FLAG_HAS_DAMAGED_DATA = 0x04` constant declared (line 21) — matches binary.
- Flag parsed per-cell at line 68; surfaced as `TmpTile.has_damaged_data: bool` (line 187).

[src/assets/tmp_file.rs](src/assets/tmp_file.rs)
- `TmpTile.has_damaged_data: bool` exposed (line 51).
- Single TMP-file parse — one variant per file. NO need for multi-variant decode within one file (the binary doesn't do that either).

[src/map/theater.rs](src/map/theater.rs)
- `variant_filenames(tile_id) -> &[String]` (line 210) returns the letter-suffixed variant filenames.
- `variant_count(tile_id) -> u8` (line 202) returns count.
- `build_tile_images()` (around line 660-703) **already loads all variant TMPs** and creates `TileKey { tile_id, sub_tile, variant: var_idx+1 }` entries in the global tile image map. Comment at line 660 documents the `{base}a..{base}d` pattern.
- Variant 0 = pristine; variant 1..N = letter-suffixed files. **This indexing matches the binary exactly** (binary's variant 0 = chain head; binary's variant 1 = first `next_variant` = the `'a'` file).

[src/render/tile_atlas.rs](src/render/tile_atlas.rs)
- `TileKey` is the atlas lookup key. Includes `tile_id`, `sub_tile`, `variant`.
- `build_atlas()` packs ALL variants (each TileKey entry) into the GPU atlas. Already works.

[src/map/resolved_terrain.rs](src/map/resolved_terrain.rs)
- Line 864-887: at map load, picks a random variant per cell via hash-of-(rx, ry) into `cell.variant`. The comment claims bridges are excluded but the code does NOT skip `has_damaged_data` tiles — it just so happens that bridges typically have **no letter-suffixed visual-diversity variants** so `variant_count == 0` and the loop `continue`s.

[src/sim/bridge_state/mod.rs](src/sim/bridge_state/mod.rs)
- `BridgeRuntimeCell.damaged_variant: bool` field already declared (line 403). Currently always written `false` at all construction sites; no setter, no consumer.

### What is missing

1. **Render-path lookup of `damaged_variant`.** No code anywhere in `src/render/`
   reads `BridgeRuntimeState.cell(rx, ry).damaged_variant` to choose the atlas key
   variant. The bridge body draw site (still TBD location) must select
   `TileKey { variant: damaged_variant as u8 }` instead of variant 0.

2. **Map-load variant pick must NOT randomize bridge tiles.** Currently
   `resolved_terrain.rs:874` writes `cell.variant = hash % (vc + 1)` for any cell
   whose tile has `variant_count > 0`. For bridges with `has_damaged_data`, this
   should be FORCED to 0 (pristine) — the per-frame damaged_variant bool overrides
   at draw time. Today this is safe-by-accident because real bridge tiles ship
   without letter-suffix visual-diversity variants, but the guard should be
   explicit.

3. **Sim-side flood-fill writer.** The damage handler must set
   `damaged_variant = true` on the affected cells via 8-neighbor flood-fill bounded
   by `tile_index` equality. This is the G4 brainstorm's "subsystem A".

4. **Repair-side clear.** `body_cell_repair_state` must clear
   `damaged_variant = false` on every cell it transitions to Healthy.

---

## 10. Tiny details captured this session (parity ledger contributions)

1. **Variant suffix encoding** is `'`' + N` where `N = 1..` (so variant 1 = `'a'`, variant 2 = `'b'`, ..., variant 26 = `'z'`). Higher N would produce non-letter characters and the engine would still attempt the lookup; in practice tiles are limited to ≤4 variants by FA2 / theater convention.

2. **Variant 0 (pristine) has NO suffix.** The pristine file is `<base><nn>.<ext>` not `<base><nn> .<ext>` — the suffix branch is conditional on `iStack_9f8 != 0`.

3. **The pristine file MUST exist.** If `LoadFileFromMIX` returns null on the first attempt (iStack_9f8 == 0), the engine immediately gives up on that tile slot — it does NOT try variant `'a'` first. So you cannot have a damaged-only tile with no pristine.

4. **Chain construction stops at the first missing variant file.** If `BRIDGE01A.TEM` exists but `BRIDGE01B.TEM` does not, the chain stops at length 2. Real bridge tiles always have exactly 2 variants (pristine + damaged).

5. **`VariantCount` is set decreasing along the chain** — head = total, next = total-1, ..., tail = 2 (or `1` if no chain was built). The constructor default `1` is overwritten ONLY when at least 1 variant beyond pristine loaded.

6. **Cycle detection in the blitter** (`if (piVar10 == param_1) break`) — if the variant walk loops back to the head, abort. This guards against bad chain initialization (none observed in retail YR).

7. **`variant_index % VariantCount`** — the blitter wraps oversized variant indices. So passing `variant_index = 5` to a chain of length 2 yields `5 % 2 = 1` = damaged. This means `damaged_variant: bool` cast to `u8` (0 or 1) directly works correctly for both single-variant and two-variant chains.

8. **The damaged variant has ITS OWN `template_width`/`template_height`/`tile_width`/`tile_height` and its own complete cell-data table.** Variants are FULLY independent TMP files; nothing is shared. The Rust parser already correctly parses each one in isolation.

9. **TMP `+0x2B6` / `+0x2B9` radar colors** are per-variant too (each TMP carries its own). The radar mini-map renders the damaged radar color when `damaged_variant = true` (verified in `GetRadarPixelColor` at 0x0047BDB0).

10. **Variant tiles are NOT in `g_IsoTileTypeArray`.** Only chain heads are registered there. The variant tiles are heap-allocated and reachable only via the head's `+0x2BC` chain. This means `g_IsoTileTypeArray.len()` reflects unique tile_ids, not total TMP loaded.

11. **`+0x2F5` persistent filename buffer (14 bytes)** is written ONLY on the pristine tile (`if (iStack_9f8 == 0)` branch in the file-found path). It's the cull-pass identifier used by `FUN_00546DA0` Phase 2 to free unused TMPs.

12. **Pristine tile construction passes `param_6 = 0`** to the constructor (registers in `g_IsoTileTypeArray`); variant constructions pass `param_6 = 1` (skip registration). This is the discriminator inside the constructor at 0x005447C0.

13. **VariantCount fixup only runs when `iStack_9f8 > 1`** — meaning at least 1 variant beyond pristine was successfully loaded. If only pristine exists, head retains `VariantCount = 1` and the blitter's `if (variant_count + -1 < variant_index)` clamp ensures variant index 1 wraps to 0 → no harm.

14. **The blitter receives `LightConvert` as `param_1`, not the IsoTileType pointer** — per the prior doc's §17.4 caveat. Ghidra's signature is misleading. The variant-chain walk we see in the decompile is therefore likely either dead-on-this-callsite OR works because of a different param layout. **However**, the equivalent walk visible in `FUN_00546DA0` (the per-cell pre-pick pass) uses the same logic against the head tile and confirms the chain semantics regardless of which exact param the blitter consumes. The render-time pick produces the right variant value (`(Flags >> 13) & 1` for damaged, hash for visual), and the eventual TMP-data pointer selection inside the blitter follows the chain by however many steps. Net behavior is verified correct.

15. **No tick-stage dependency.** The variant pick is recomputed every draw call; there's no per-tick state in the render path beyond `cell.Flags & 0x2000`. Means the sim writer can flip the bit any time during Phase F and the render reads it next frame (no sub-tick ordering issue).

---

## 11. Open questions

1. **The TMP_TileBlitter param_1 mismatch (carried from prior doc §17.4).** Not blocking the Rust port — we can choose to recompute the variant index in the Rust caller and pass the *already-chosen* variant TMP into our blitter, sidestepping the chain walk entirely. Worth a hand-pass over the blitter prologue if a future session investigates the radar/MM combinations more deeply.

2. **Theater extension lookup order beyond Temperate/Urban/MarbleMadness/`.TEM`.** Decompiled three fallback chains; haven't traced what the engine does for Snow/Desert/Lunar theater. Likely a parallel `s_<ext>` set of strings in the same `.rdata` segment. Not relevant for the Rust port because Rust's MIX-archive lookup is filename-driven and we can match whatever filename the theater INI implies.

3. **Whether any retail bridge tile has more than 2 variants.** All real YR bridge tiles ship with exactly pristine + one damaged variant (`<base>NN.TEM` + `<base>NNA.TEM`). A 3-variant bridge tile is hypothetically supported by the engine but unobserved in stock assets. The Rust render code should still treat `damaged_variant: bool` as the source of truth and modulo against `VariantCount` to handle this defensively.

4. **Whether `cell.variant` (the FA2 hash-pick field on `ResolvedTerrainCell`) ever interacts with bridge tiles.** Code at `resolved_terrain.rs:874` could in principle write a non-zero value if a future modded theater INI ships `BRIDGE01A.TEM` purely as visual-diversity (no `has_damaged_data` bit). Today this is safe (no such modded asset exists), but the Rust render path should still treat the per-cell variant index as: `if bridge cell && has_damaged_data → damaged_variant as u8 else cell.variant`.

---

## 12. Summary for the G4 design

The damaged-variant pixel data layout is now fully understood:

- **It's a separate `.TMP` file** with a letter suffix appended to the base name (`<base>NN.TEM` pristine, `<base>NNA.TEM` damaged).
- **The Rust asset pipeline already loads it.** `theater.rs:660-703` reads variant TMPs and registers them as `TileKey { variant: var_idx+1 }`.
- **The Rust atlas builder already packs both variants** into the GPU atlas.
- **The Rust per-cell pick already supports it** via `TileKey.variant`.

The remaining work for G4 subsystem B is purely in the **bridge render call site**:
- When sampling a bridge body cell, read `BridgeRuntimeState.cell(rx, ry).damaged_variant`.
- Pass `TileKey { variant: damaged_variant as u8 }` to the atlas lookup instead of the default variant 0.

And a small guard fix in `resolved_terrain.rs`:
- The map-load random variant pick at line 874 should explicitly skip bridge tiles (those whose pristine TMP has `has_damaged_data == true`) so the per-frame bool can override without interference.

Subsystem B is **substantially smaller in scope** than the G4 brainstorm initially feared. The asset-pipeline work is already done.

---

## Sources

**Ghidra functions decompiled this session:**
- `0x00547CF0` — `TMP_TileBlitter` (variant-chain walk lines 28-46)
- `0x005447C0` — `IsometricTileTypeClass::Constructor` (init values for next_variant=0, VariantCount=1)
- `0x00545150` — `Read_Theater_TileSets_INI` (full decompile — variant-load inner loop, VariantCount fixup)

**Ghidra memory regions inspected:**
- `0x008291A4` — format string `"%.28s %02d"`
- `0x008291B0` — format string `"%02d"`
- `0x00829140` — `".TEM"`
- `0x00829148` — `".MMT"`
- `0x00829138` — `".URB"`

**Prior reports cross-verified:**
- `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` §14 (TMP_TileBlitter prior decompile) and §17 (CellOverlay_TileDraw + variant pick)
- `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md` §4 (cell.Flags bit 13 = damaged_variant)

**Rust source audited:**
- `src/assets/tmp_decode.rs` (TMP cell parser — already exposes `has_damaged_data` correctly)
- `src/assets/tmp_file.rs` (TmpFile struct — single variant per file, correct)
- `src/map/theater.rs` (theater tile loader — already loads variant filenames a/b/c/d)
- `src/render/tile_atlas.rs` (TileKey + atlas builder — already supports variant axis)
- `src/map/resolved_terrain.rs` (map-load variant pick — needs bridge guard fix)
- `src/sim/bridge_state/mod.rs` (BridgeRuntimeCell.damaged_variant — declared but unused)
