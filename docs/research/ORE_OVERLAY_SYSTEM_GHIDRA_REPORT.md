# Ore Overlay System — Ghidra RE Report

**Date:** 2026-03-25 (audited 2026-04-27)
**Binary:** gamemd.exe (Yuri's Revenge)
**Confidence:** HIGH — all findings verified from direct decompilation via Ghidra MCP

**Audit notes (2026-04-27):** Sections 4, 6, 8, 9, 11 patched against the live binary —
+0x122 was misidentified as OreNeighborCount (it's a wall-neighbor counter), the
Reduce_Tiberium 8-neighbor loop pseudocode was wrong (re-seeds the spread queue, doesn't
decrement a counter), the harvest-timer ×3 only applies to the dead Weeder branch,
and §9.1 / §9.2 sub-sections were swapped (Spread / Growth). The §16 "Critical Bug in
Current Code" claim about Rust overlay-index compaction is **unverified** — needs the
[OverlayTypes] section reader traced before relying on it.

**Audit notes (2026-05-20, /re-swarm --area tiberium overlays):** Sections 6, 10, 11
patched against the live binary —
- §6 TIBTRE mechanism rewritten: TIBTRE does **not** create an AnimClass for ore
  spawning. The `PUSH 0x1C8` at `0x0071b9a7` is the destruction explosion inside
  `TerrainClass::Take_Damage`, not the spawn path. The real spawn path is
  `TerrainClass::AI` (`0x0071C730`) → `CellClass::SpreadTiberium` (`0x00483780`)
  driven by `IsAnimated=yes` + `SpawnsTiberium=yes`. See
  `TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md`.
- §6 `AnimType+0x338 TiberiumSpawnType` / `+0x33C TiberiumSpreadRadius` are part of
  a **separate** system (AnimClass::AI bouncer-landing block, `0x00423ac0`) used by
  `METDEBRI` (meteor impact) and `CRYSTAL1-4` (gem bouncers). Moved into a new §6a.
- §6 TerrainTypeClass `+0x2B2` and `+0x2B3` resolved: `IsFlammable` (TS-dead) and
  `IsAnimated` (active in YR). See `TERRAINTYPECLASS_2B2_2B3_FLAGS_GHIDRA_REPORT.md`.
- §10 PlaceTiberium pseudocode replaced: 6 Branch-A pre-flight gates added (most
  importantly `ScenarioClass+0x34A6` TiberiumGrowthEnabled and
  `TibClass+0xB0 GrowthPercentage ≥ 0` — the latter is what suppresses gem growth).
  `+0x140` bit-7 and `+0xEC` LandType are **not** written by PlaceTiberium —
  `RecalcAttributes` (`0x0047d2b0`) owns both. `RadarMarkDirty` only fires in
  Branch B (germinate). Sloped-cell variant formula added. See
  `CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md`.
- §11 density-11 detour clarified as no-op (`AddToGrowthQueue` at `0x007235A0`
  has an internal `density < 11` guard; density is still 11 at call time).
  `OverlayToTiberiumIndex` (IsWallOverlay) fallback returns **0**, not -1, for a
  `Tiberium=yes` overlay outside any range — Reduce_Tiberium's guard does not
  catch this. See `CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md`.

**Audit notes (2026-07-24, live Ghidra correction):** Sections 2, 3, 14, and 16
were corrected after tracing both the `[OverlayTypes]` loader and
`TiberiumClass::ReadINI`.

- The engine compacts `[OverlayTypes]` by section-entry ordinal; numeric INI
  keys are not runtime array indices. Runtime bases are GEM01=27, TIB01=102,
  TIB2_01=127, and TIB3_01=147.
- `OverlayToTiberiumIndex @ 0x005FDD20` checks twelve primary images plus
  `NumExtraImages`; stock TIB13 through TIB20 are therefore Riparius, not
  unmapped overlays.
- `OverlayTypeClass::ReadINI @ 0x005FE770` forces parsed Land to tiberium
  LandType 5 only when `Tiberium=yes` and the parsed Land value is zero.
  Explicit nonzero `Land=` survives.
- The full signed `TiberiumClass::ReadINI Image=` switch is now decoded.
  `-1` performs no image/count writes; `2` does not write `NumExtraImages`;
  all other integers except `3` and `4` use the Riparius default. Because the
  reader does not reset these fields, omitted writes preserve prior values on
  a reread.

---

## 1. Architecture Overview

The ore/resource system in YR uses four interconnected class hierarchies:

| Class | Purpose | Key Address |
|-------|---------|-------------|
| **TiberiumClass** | Defines a resource type (ore/gems) — value, growth rules, overlay mapping | Constructor: `0x007216c0` |
| **OverlayTypeClass** | Defines a single overlay type — SHP image, flags (Tiberium, Wall, etc.) | Constructor: `0x005fe250`, ReadINI: `0x005fe7a0` |
| **CellClass** | Per-cell state — which overlay is present, density, land type | DrawOverlay: `0x0047f6a0` |
| **TerrainClass / TerrainTypeClass** | "Ore wells" — TIBTRE terrain objects that seed ore around them | Constructor: `0x0071bb90` |

**Data flow:**
```
[Tiberiums] section          → TiberiumClass instances (4 types)
  ├─ Image= key              → maps to a range of OverlayTypeClass entries
  ├─ Growth/Spread settings   → timers + priority queues for periodic updates
  └─ Value= key              → credit value per density unit

[OverlayTypes] section       → OverlayTypeClass instances (~250 types)
  └─ Tiberium=yes flag       → marks as harvestable ore

[Terrain] map section        → TerrainClass instances (TIBTRE01-03 = ore wells)
  └─ SpawnsTiberium=yes      → periodically seeds new ore around itself

[OverlayPack] map data       → CellClass.OverlayTypeIndex (which overlay)
[OverlayDataPack] map data   → CellClass.OverlayData (density 0-11 for ore)
```

### Ore Lifecycle in YR

1. **Map load:** Initial ore patches placed from `[OverlayPack]` + TIBTRE terrain objects placed from `[Terrain]`
2. **Runtime seeding:** TIBTRE trees periodically spawn new ore cells within their radius via AnimClass + `TiberiumSpawnType`
3. **Growth:** Existing ore cells increase density (0→11) via TiberiumClass growth priority queue
4. **Spread:** High-density ore cells propagate to empty neighbor cells via TiberiumClass spread priority queue
5. **Harvesting:** Miners reduce cell density; at density 0, overlay is removed entirely

**All retail YR maps** have `TiberiumGrowthEnabled=yes` in `[Basic]` and contain TIBTRE terrain objects (typically 8-38 per map). This is NOT dormant TS code — it is the active ore regeneration system.

---

## 2. TiberiumClass Struct Layout

Source: Constructor at `0x007216c0`, ReadINI at `0x00721a90`.
**IMPORTANT:** `param_1` type is `undefined4 *` (int pointer), so `param_1[N]` = byte offset `N × 4`.

| Offset | Type | INI Key | Default | Description |
|--------|------|---------|---------|-------------|
| +0x98 | int | — | -1 | ArrayIndex in global TiberiumClass array |
| +0x9C | int | Spread | 0 | Spread interval in frames (timer between spread ticks) |
| +0xA0 | double | SpreadPercentage | 0.1 | Fraction of queued cells to spread per tick |
| +0xA8 | int | Growth | 0 | Growth interval in frames |
| +0xB0 | double | GrowthPercentage | 0.1 | Fraction of queued cells to grow per tick |
| +0xB8 | int | Value | 0 | Credit value per density unit |
| +0xBC | int | Power | 0 | Radiation power (TS legacy, unused in YR) |
| +0xC0 | int | Color | — | Radar minimap color |
| +0xC4 | TypeList | Debris | — | Debris AnimType names (comma-separated) |
| +0xE0 | OverlayTypeClass* | Image | — | Pointer to first overlay type for this tiberium |
| +0xE4 | int | — | 12 | MaxDensity (number of density frames in SHP) |
| +0xE8 | int | — | 12 | NumImages (primary overlay type count in range) |
| +0xEC | int | — | 8 or 0 | NumExtraImages (extra overlay variants; 8 for ore, 0 for gems) |
| +0xF0 | int | — | 0 | SpreadQueue cell count |
| +0xF4 | Heap* | — | — | Spread priority queue (min-heap by frame timing) |
| +0xF8 | byte* | — | — | Spread per-cell flag array (prevents duplicate queueing) |
| +0xFC | void* | — | — | Spread cell data buffer (8 bytes per entry: coords + timing) |
| +0x100 | int | — | — | LastSpreadFrame (frame counter when spread last ran) |
| +0x108 | int | — | — | SpreadInterval (frames until next spread, reset to +0x9C each tick) |
| +0x10C | int | — | 0 | GrowthQueue cell count |
| +0x110 | Heap* | — | — | Growth priority queue (min-heap by frame timing) |
| +0x114 | byte* | — | — | Growth per-cell flag array |
| +0x118 | void* | — | — | Growth cell data buffer |
| +0x11C | int | — | — | LastGrowthFrame |
| +0x124 | int | — | — | GrowthInterval (frames until next growth tick) |

### Image → Overlay Index Mapping (Hardcoded Switch at `0x00721c55`)

The `Image=` INI key selects a compact runtime `OverlayTypeClass` range.
`RulesClass::Process @ 0x00668CF9..0x00668D32` appends entries by section
ordinal, so the numeric keys in `[OverlayTypes]` are lookup names rather than
the runtime array indices stored in map overlay bytes.

| Signed `Image=` value | Switch result | Array Offset | Runtime Base | Counts written |
|-------------|-------------|-------------|--------------|-----------|
| -1 | no image branch | — | preserved; 0 on fresh construction | none; all prior `+E0/+E4/+E8/+EC` values survive |
| 2 | Cruentus | 0x6C (÷4=27) | 27 / GEM01 | `MaxDensity=12`, `NumImages=12`; `NumExtraImages` is preserved (0 on fresh construction) |
| 3 | Vinifera | 0x1FC (÷4=127) | 127 / TIB2_01 | `12/12/8` |
| 4 | Aboreus | 0x24C (÷4=147) | 147 / TIB3_01 | `12/12/8` |
| every other signed integer, including 0 and 1 | default Riparius | 0x198 (÷4=102) | 102 / TIB01 | `12/12/8` |

Evidence: `ReadInt` default `-1`, signed increment, and unsigned jump-table
dispatch at `0x00721C3F..0x00721C55`; jump-table bytes at `0x00721CF8`;
case writes at `0x00721C5C..0x00721CD6`. The constructor zeros all four fields
at `0x0072173A..0x0072174C`, but `ReadINI` does not reset them before dispatch.

There are no non-tiberium prefixes inside these native ranges. The difference
between a raw numeric key and runtime slot is ordinal compaction, including
skipped numeric keys 40 and 41 before TIB01.

### Overlay Index Ranges Per Tiberium Type (from `rulesmd.ini`)

| Tiberium | Image | First Overlay | Range (Primary 12) | Range (Extra 8) | Total |
|----------|-------|---------------|--------------------|--------------------|-------|
| Riparius (Ore) | 1 | 102 | 102-113 (TIB01-TIB12) | 114-121 (TIB13-TIB20) | 20 |
| Cruentus (Gems) | 2 | 27 | 27-38 (GEM01-GEM12) | — on fresh construction; a reread preserves prior `NumExtraImages` | 12 fresh |
| Vinifera | 3 | 127 | 127-138 (TIB2_01-TIB2_12) | 139-146 (TIB2_13-TIB2_20) | 20 |
| Aboreus | 4 | 147 | 147-158 (TIB3_01-TIB3_12) | 159-166 (TIB3_13-TIB3_20) | 20 |

Only Riparius and Cruentus are actively used in YR. Vinifera and Aboreus are TS legacy.

---

## 3. OverlayTypeClass Struct Layout

Source: Constructor at `0x005fe250`, ReadINI at `0x005fe7a0`.
**IMPORTANT:** Constructor uses `undefined4 *` param (multiply index by 4), but ReadINI uses direct byte offsets via `unaff_ESI`.

| Offset | Type | INI Key | Default | Description |
|--------|------|---------|---------|-------------|
| +0x294 | int | — | -1 | ArrayIndex (position in global OverlayTypeClass array) |
| +0x298 | int | Land | 0 | Parsed LandType; if `Tiberium=true` and this remains 0, ReadINI forces tiberium LandType 5. Explicit nonzero Land is preserved. |
| +0x29C | AnimTypeClass* | CellAnim | NULL | Cell animation on this overlay |
| +0x2A0 | int | DamageLevels | 1 | Number of damage stages (for destructible walls) |
| +0x2A4 | int | Strength | 1 | Hit points |
| +0x2A8 | bool | Wall | false | Is a wall overlay |
| +0x2A9 | bool | Tiberium | false | **Is a tiberium/ore overlay** (checked by IsWallOverlay) |
| +0x2AA | bool | Crate | false | Is a crate |
| +0x2AB | bool | CrateTrigger | false | Triggers crate logic |
| +0x2AC | bool | NoUseTileLandType | true | Don't inherit tile's land type |
| +0x2AD | bool | IsVeinholeMonster | false | TS veinhole monster (disabled in YR) |
| +0x2AE | bool | IsVeins | false | TS veins (disabled in YR) |
| +0x2B0 | bool | Explodes | false | Explodes when destroyed |
| +0x2B1 | bool | ChainReaction | false | Chain-reacts with neighbors |
| +0x2B2 | bool | Overrides | false | Overrides existing overlays |
| +0x2B3 | bool | DrawFlat | true | Draw without Z-height adjustment |
| +0x2B4 | bool | IsRubble | false | Is rubble overlay |
| +0x2B5 | bool | IsARock | false | Is a rock overlay |
| +0x2B6 | 3 bytes | RadarColor | 0,0,0 | RGB color for radar minimap |

### Key Logic in ReadINI (`0x005fe7a0`):
When `Tiberium=true`:
- Land type is forced to 6 (Tiberium land)
- If `Land=` was 0, it's set to 5 instead
- The `0x9C` field (in ObjectTypeClass) is set to 6

---

## 4. CellClass Overlay Fields

Source: DrawOverlay_Body at `0x0047f6a0`, Reduce_Tiberium at `0x00480a80`, RecalcAttributes at `0x0047d2b0`.

| Offset | Type | Description |
|--------|------|-------------|
| +0x24 | short[2] | MapCoord_X, MapCoord_Y |
| +0x34 | void* | IsoTile pointer |
| +0x38 | int | IsoTileTypeIndex |
| +0x44 | int | **OverlayTypeIndex** (-1 = no overlay) — index into OverlayTypeClass array |
| +0x11B | byte | Level (cell elevation for height calculation) |
| +0x11C | byte | DamageState (wall damage level; 0 = undamaged) |
| +0x11E | byte | **OverlayData** — for tiberium, this is **density (0-11)** |
| +0x122 | byte | WallNeighborCount — decremented on adjacent **wall** removal (CellClass::DestroyOverlay + PostDestructionWallCleanup, both gated on OverlayTypeClass.Wall +0x2A8). NOT used by ore code; SpreadCellGerminate recounts ore neighbors at runtime. |
| +0x140 | uint | Flags — bit 7 (0x80) = cell has tiberium overlay |
| +0xEC | int | LandType — set to 5 for tiberium cells |

---

## 5. IsWallOverlay — Overlay-to-Tiberium Mapping

**Address:** `0x005fdd20`

Despite its misleading name (TS legacy), this function maps an overlay type index to its owning TiberiumClass index:

```c
// Pseudocode
int IsWallOverlay(int overlayTypeIndex) {
    if (overlayTypeIndex == -1) return -1;
    if (!OverlayTypeArray[overlayTypeIndex]->Tiberium) return -1;  // +0x2A9

    for (int i = 0; i < TiberiumCount; i++) {
        TiberiumClass* tib = TiberiumArray[i];
        int firstIdx = tib->Image->ArrayIndex;  // +0xE0 -> +0x294

        // Primary range: [firstIdx, firstIdx + NumImages)
        if (overlayTypeIndex >= firstIdx && overlayTypeIndex < firstIdx + tib->NumImages)
            return tib->ArrayIndex;  // +0x98

        // Extra range: [firstIdx + NumImages, firstIdx + NumImages + NumExtraImages)
        if (overlayTypeIndex >= firstIdx + tib->NumImages &&
            overlayTypeIndex < firstIdx + tib->NumImages + tib->NumExtraImages)
            return tib->ArrayIndex;
    }

    // "Overlay %s not really tiberium" warning
    return 0;
}
```

---

## 6. TIBTRE Ore Wells (TerrainClass Spawning)

**This is the primary ore seeding mechanism in YR.** Every retail map places TIBTRE01-03 terrain objects as "ore wells" that continuously regenerate ore.

### TerrainTypeClass Fields (Ore-Related)

| Offset | Type | INI Key | Description |
|--------|------|---------|-------------|
| +0x2B1 | bool | SpawnsTiberium | Periodically spread ore from this terrain (gated by IsAnimated) |
| +0x2B2 | bool | IsFlammable | TS "Forest Fires" legacy — **dead code in YR, no readers** (verified via `decompile_function 0x0071DEA0` + binary string search) |
| +0x2B3 | bool | IsAnimated | Drives the per-tick animation cycle AND the mid-anim SpreadTiberium call (verified via `decompile_function 0x0071C730` + `decompile_function 0x0071C1B0`) |

INI write sites: `TerrainTypeClass::ReadINI_Full` at `0x0071DEA0` — `IsFlammable`
string at `0x00844668` (verified via `inspect_memory_content 0x00844668`),
`IsAnimated` string at `0x0084465C` (verified via `inspect_memory_content 0x0084465C`).

### Runtime Behavior — Corrected (2026-05-20)

**Prior versions of this section claimed TIBTRE creates an AnimClass with
`TiberiumSpawnType` / `TiberiumSpreadRadius` to spawn ore. That is WRONG.**
TIBTRE spreads ore by calling `CellClass::SpreadTiberium` directly from
`TerrainClass::AI`. The bouncer-impact ore-spawn path that DOES read
`AnimType+0x338/+0x33C` is a separate system, documented in §6a below.

The actual TIBTRE spread chain (verified via `decompile_function 0x0071C730`):

```
TerrainClass::AI (0x0071C730) called every game tick per TerrainClass instance
  │
  ├── IF (TerrainTypeClass+0x2B3 IsAnimated == true):
  │     advance animation timer; on frame-roll event:
  │       │
  │       └── IF (TerrainTypeClass+0x2B1 SpawnsTiberium == true):
  │             call CellClass::SpreadTiberium (0x00483780) on this->cell
  │
  └── (other AI logic: nothing else ore-related)
```

`CellClass::SpreadTiberium` (`0x00483780`) is one of the three callers of
`CellClass::PlaceTiberium` (`0x00487190`) — see §10. It picks an in-radius empty
neighbor cell, validates it via `CanPlaceTiberium`, and germinates new ore at
density 3.

**Key consequence:** TIBTRE only spreads ore when **both** `SpawnsTiberium=yes`
**and** `IsAnimated=yes` are set. TIBTRE01/02/03 set both in stock `rulesmd.ini`.
The "PUSH 0x1C8" at `0x0071b9a7` cited in prior versions of this doc is inside
`TerrainClass::Take_Damage` (destruction explosion), not the spawn path.

### Gating Flags (TIBTRE-specific)

For TIBTRE to spread ore, ALL of these must be true:
1. **TerrainTypeClass+0x2B1 `SpawnsTiberium`** (yes on TIBTRE01-03)
2. **TerrainTypeClass+0x2B3 `IsAnimated`** (yes on TIBTRE01-03 — gates the AI tick)
3. **`TiberiumGrowthEnabled`** (`ScenarioClass+0x34A6`) — per-map flag from `[Basic]` section
4. **`SpecialFlags.TiberiumSpreads`** (bit 7 / 0x80) — from multiplayer dialog settings

Without TIBTRE trees, existing ore still grows in density and spreads via the
TiberiumClass spread queue (see §9), but no NEW ore is seeded from terrain features.

### Map Placement Evidence

**All 54 retail YR maps** contain TIBTRE objects:
- amazon.mmx: 14 TIBTRE instances
- PowdrKeg.mmx: 38 TIBTRE instances (highest)
- MoonPatr.yro: 0 TIBTRE (only exception — lunar map)
- Standard template (StdMapRA2.ini): `TiberiumGrowthEnabled=yes` default

### INI Definitions

```ini
[TIBTRE01]
Name=Tiberium Tree
SpawnsTiberium=yes
IsAnimated=yes
RadarColor=192,192,0
; ... (Immune=no, WaterBound=no default)

[TIBTRE02]
Name=Tiberium Tree
SpawnsTiberium=yes
IsAnimated=yes
; ...

[TIBTRE03]
Name=Tiberium Tree
SpawnsTiberium=yes
IsAnimated=yes
; ...
```

## 6a. Bouncer-Impact Ore Drop (METDEBRI / CRYSTAL1-4)

**This is a separate, second ore-placement system.** It is what actually reads
the AnimTypeClass `TiberiumSpawnType` / `TiberiumSpreadRadius` fields.
Documented standalone in `TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md`.

### Path

`AnimClass::AI` (`0x00423ac0`) inline bouncer/meteor-landing block
(approx. `0x00423f00–0x00424235`):

- Fires once on landing (not periodic), gated by `AnimType+0x358 IsTiberium`
  inside the `Bouncer=yes` branch.
- Iterates a `[-radius, +radius]²` square around the impact, skipping cells
  with `sqrt(dx²+dy²) > TiberiumSpreadRadius`. `Sqrt_Approx` at `0x004cac40`.
- Per cell: runs `CellClass::CanPlaceTiberium` (`0x004838e0`).
- Placement calls `OverlayClass::Constructor` (`0x005fc380`) **directly** — NOT
  `CellClass::PlaceTiberium`. Variant = `TiberiumSpawnType->ArrayIndex + Random(0,3)`.
  Density = `Random(0,2)` written to `CellClass+0x11E`.
- No growth/spread queue entry at placement time.

### Stock YR consumers (artmd.ini)

| AnimType | Bouncer | IsTiberium | TiberiumSpawnType | TiberiumSpreadRadius | Trigger |
|----------|---------|-----------|--------------------|----------------------|---------|
| METDEBRI | yes | yes | TIB01 | (per art) | Meteor impact (map trigger) |
| METSMALL | yes | yes | TIB01 | (per art) | Small meteor impact |
| CRYSTAL1 | yes | yes | TIB2_01 | 0 | Gem-class bouncer |
| CRYSTAL2 | yes | yes | TIB2_01 | 0 | Gem-class bouncer |
| CRYSTAL3 | yes | yes | TIB2_01 | 0 | Gem-class bouncer |
| CRYSTAL4 | yes | yes | TIB2_01 | 0 | Gem-class bouncer |

CRYSTAL1-4 have radius 0 — they only place on the landing cell, which often
fails `CanPlaceTiberium`, so visible effect is sparse.

### AnimTypeClass Tiberium-Related Field Offsets (verified)

Source: AnimTypeClass::ReadINI at `0x00427d00` (verified via
`decompile_function 0x00427d00`).

| Offset | Type | INI Key | Default | Consumer / Behavior |
|--------|------|---------|---------|---------------------|
| +0x338 | OverlayTypeClass* | TiberiumSpawnType | NULL | Overlay type for bouncer-impact spawn (§6a) |
| +0x33C | int | TiberiumSpreadRadius | 0 | Radius for bouncer-impact spawn (§6a) |
| +0x357 | bool | TiberiumChainReaction | false | Reads in `AnimClass::Middle` (`0x00424CE0`): clears the underlying cell's ore + 1-in-3 spawns `TiberiumClass.Debris` anim + area damage. Stock use: `TWLT070T`. |
| +0x358 | bool | IsTiberium | false | Inside Bouncer block only (§6a) — without `Bouncer=yes`, this flag does **nothing**. |
| +0x359 | bool | HideIfNoOre | false | Reads in `AnimClass::AI` every tick: sets `AnimClass+0x19D IsInvisible` based on `CellClass::Get_Tiberium_Value`. Anim still advances; only draw is suppressed. Stock use: `TWNK1` (ore sparkle). |
| +0x360 | bool | IsAnimatedTiberium | false | Reads in `AnimClass::AI` every tick: checks cell at offset `(-0x180, -0x180)` leptons; if that cell's overlay's `CellAnim (+0x29C)` doesn't match this AnimType, sets `AnimClass+0x19B IsInactive` and the anim **destroys itself**. Stock use: `BIGBLUE`. |

**Note:** `VoxelAnimTypeClass+0x300` is a **separate** `IsTiberium` field — same INI
key string, different class, different consumer. `VoxelAnimClass::AI` at
`0x00749F30` places ore in 8 neighbors on voxel-anim expiry. Stock uses in
`rules.ini`: CRYSTAL01, CRYSTAL02, METEOR01.

See `ANIMTYPECLASS_TIBERIUM_FLAG_CONSUMERS_GHIDRA_REPORT.md` for full per-flag
consumer decompilation.

---

## 7. Ore Value & Credit Calculation

### CellClass::Get_Tiberium_Value (`0x00485020`)

```c
int Get_Tiberium_Value(CellClass* cell) {
    int tibIdx = IsWallOverlay(cell->OverlayTypeIndex);
    if (tibIdx == -1) return 0;
    return TiberiumArray[tibIdx]->Value * (cell->OverlayData + 1);
}
```

- With Value=25 (ore) and density 0-11: cell value = **25 to 300 credits**
- With Value=50 (gems) and density 0-11: cell value = **50 to 600 credits**

### Density Neighbor Lookup Table (`0x0081cd28`)

When placing ore on a cell (via `FUN_004818e0` at `0x004818e0`), the initial density depends on how many of the 8 neighboring cells also contain the same tiberium type:

```
Neighbors:  0  1  2  3  4  5  6  7  8  9 10 11
Density:    0  1  3  4  6  7  8 10 11  7  0  1  (repeats mod 12)
```

This creates natural-looking ore patches where cells with more ore neighbors are richer.

---

## 8. Ore Drawing System

### DrawOverlay_Body (`0x0047f6a0`)

The body frame of a tiberium overlay SHP is selected by:

```c
uint frame = cell->OverlayData;  // density 0-11

// Visual variety at sparse (0) and dense (9) levels
if (frame == 0 || frame == 9) {
    frame += variety_table[((MapCoord_Y & 3) << 2) | (MapCoord_X & 3)];
}

CC_Draw_Shape(shp, frame, &pos, clipRect, 0x4E00, 0,
              z_input * -15 - 2, // Z offset (see formula below)
              0,                 // flags
              zBuffer_value,     // *(short*)(cell + 0x10E)
              0, 0, 0, 0, 0);
```

### Variety Table (`0x0081cc30`)

4×4 table indexed by `((Y & 3) << 2) | (X & 3)`:

```
Row 0: [0, 1, 2, 3]
Row 1: [3, 2, 1, 0]
Row 2: [2, 3, 0, 1]
Row 3: [1, 0, 3, 2]
```

At density 0: frame is 0-3 (4 sparse variants).
At density 9: frame is 9-12 (4 dense variants).
Other densities (1-8, 10-11): frame equals density directly.

### SHP Frame Layout

Each ore overlay SHP (e.g., `tib01.shp`) has:
- **Frames 0-12**: Body frames (13 frames for density + variety)
- **Frames 13-25**: Shadow frames (matching, accessed at `totalFrames/2 + overlayData`)

Total: 26 frames per ore SHP.

### DrawOverlay_Shadow (`0x0047f510`)

```c
uint shadowFrame = (shp->frameCount / 2) + cell->OverlayData;
```

Special handling: at densities 9-17 (0x09-0x11), shadow is offset by (-15, +7) pixels.

### Z-Offset Formula

```
z_input  = cell->Level + ((cell->Flags >> 7) & 1) * 4
z_offset = z_input * -15 - 2
```

Where Level (+0x11B) is the cell elevation (0-based) and Flags bit 7 (0x80 in +0x140) means
"cell has tiberium overlay". When that bit is set, the Z input is biased by +4, producing
an additional −60 unit Z offset — i.e. tiberium cells draw at a different depth than
non-tiberium overlays at the same Level.

---

## 9. Growth & Spread System

Two independent timer-driven systems operate on priority queues. Both are gated by the `TiberiumGrowthEnabled` flag at `DAT_00a8b230 + 0x34A6`.

### Global Control Flags

**The SpreadDriver and GrowthDriver check ONLY `ScenarioClass+0x34A6` (TiberiumGrowthEnabled).
Neither function reads any SpecialFlags bitfield.** The `TiberiumGrows` and `TiberiumSpreads`
SpecialFlags bits previously documented here as gates are WRONG — the binary contains no such
checks in `TiberiumClass__SpreadDriver_AllTypes` (`0x007221b0`) or `TiberiumClass__GrowthDriver_AllTypes`
(`0x00722c40`). (corrected 2026-05-29: prior table listed SpecialFlags bit 6 TiberiumGrows and
bit 7 TiberiumSpreads as gates; binary shows neither is checked — ROOT_CAUSE: INFERENCE_HARDENED;
verified via `decompile_function 0x007221b0` and `decompile_function 0x00722c40`)

### 8.1 Spread System

**Tick Driver:** `TiberiumClass__SpreadDriver_AllTypes` at `0x007221b0` (iterates all TiberiumClass instances)
**Spread Logic:** `TiberiumClass__SpreadProcessor` at `0x00722440`
**Cell Germination:** `CellClass::SpreadTiberium` → `CellClass::PlaceTiberium` at `0x00487190`

**Algorithm per TiberiumClass:**
1. Check if enough frames elapsed since `LastSpreadFrame` (+0x100)
2. If timer expired: call growth execution function
3. Reset timer: `LastSpreadFrame = CurrentFrame`, `SpreadInterval = TiberiumClass->Spread` (+0x9C)

**Growth Execution (`FUN_00722440`):**
1. Calculate cells to process: `count = clamp(ceil(SpreadPercentage * queueSize), 5, 25)`
2. Add random variance: `actual = Random(0, count-1) + 1`
3. Pop cells from spread priority queue (min-heap, sorted by frame timing)
4. For each cell: check 8 neighbors (random starting direction)
5. If a valid empty neighbor exists: place new ore (density=3) via `FUN_00487190`
6. If >1 valid neighbor: re-add source cell to spread queue for future spreading

### 8.2 Growth System

**Tick Driver:** `TiberiumClass__GrowthDriver_AllTypes` at `0x00722c40` (iterates all TiberiumClass instances)
**Growth Logic:** `TiberiumClass__GrowthProcessor` at `0x00722f00`

**Algorithm per TiberiumClass:**
1. Check if enough frames elapsed since `LastGrowthFrame` (+0x11C)
2. If timer expired: call density growth execution
3. Reset timer with interval from Math::ftol (frame-based)

**Density Growth Execution (`FUN_00722f00`):**
1. Calculate cells to process: `count = clamp(ceil(GrowthPercentage * queueSize), 5, 50)`
2. Pop cells from growth priority queue
3. For each cell: if `OverlayData < 11`, increase density by 1
4. If density reaches 11 (max): remove from growth queue
5. Otherwise: re-add to growth queue AND add to spread queue

### 8.3 Eligible Cell Check (`FUN_004838e0`)

A cell is valid for tiberium placement if:
- Cell is within playfield
- Cell flags don't include 0x500 (occupied by building/terrain object)
- No blocking objects (buildings of type 6, terrain objects with flag at +0x2B1)
- Cell's LandType allows tiberium (checked via `DAT_0089ea60` table)
- Cell has no existing overlay (`OverlayTypeIndex == -1`)
- Cell has no damage indicator (`+0x11C == 0`)
- Tile type allows tiberium (checked via IsometricTileTypeClass flag at +0x306)

### 8.4 Priority Queue Implementation

Both queues use min-heaps keyed on frame timing:
- **Entry size:** 8 bytes (4-byte cell coords + 4-byte float timing value)
- **Timing jitter:** Each entry gets `CurrentFrame + Random(0, 49)` as its timing
- **Per-cell dedup:** Flag arrays prevent the same cell from being queued multiple times
- **Initialization:** `FUN_00722240` allocates queues sized to total map cell count

---

## 10. Ore Placement — `CellClass::PlaceTiberium` (`0x00487190`)

**Patched 2026-05-20 against the live binary.** Prior pseudocode (Path 1 / Path 2)
inverted the branch labels, omitted the 6 Branch-A pre-flight gates, and
incorrectly attributed cell-flag bit-7 and LandType writes to this function.

Source: `decompile_function 0x00487190`. Three callers exist (verified via
`get_function_callers 0x00487190`):

| Caller | Address | Context |
|--------|---------|---------|
| `CellClass::SpreadTiberium` | `0x00483780` | Spread to empty neighbor (TIBTRE + spread-queue driver) |
| `CellClass::GrowTiberium` | `0x00483710` | Increase density on existing ore |
| `BuildingClass::DestructionEffects` | `0x004415F0` | Drops ore on destroyed building (mod-able paths) |

Note: bouncer-impact spawn (METDEBRI/CRYSTAL — see §6a) does **NOT** call this
function; it constructs `OverlayClass` directly.

### Branch A — Grow existing ore (cell already has matching tiberium)

Gates (in order — all must pass):

1. `ScenarioClass+0x34A6 TiberiumGrowthEnabled` == true (per-map [Basic] flag).
2. `OverlayToTiberiumIndex(cell->OverlayTypeIndex)` returns a valid index
   (≠ -1; note: returns 0 on type mismatch — see §11).
3. `TibClass->GrowthPercentage (+0xB0, double) > ~8e-6`. The actual check is
   `if (GrowthPercentage <= _DAT_007e3810) return` where `_DAT_007e3810 = 0x3ee4f8b588e368f1`
   (≈ 7.96e-6, a small positive epsilon). Cruentus has `GrowthPercentage=0`, which satisfies
   `0.0 <= ~8e-6` and causes an **early return** — gems do NOT pass this gate, they bail here.
   (corrected 2026-05-29: prior text said comparison is `>= 0.0` and gems pass; binary shows
   gate is `<= epsilon` early-return, gems bail; ROOT_CAUSE: INFERENCE_HARDENED; verified via
   `decompile_function 0x00722f00` + `read_memory 0x007e3810`)
4. `cell->OverlayData < (TibClass->MaxDensity - 1)` (i.e. density < 11).
5. Cell-flags do not include the "no-grow" mask bits checked in PlaceTiberium
   prologue.
6. Random-variant write only when the cell's existing overlay is at a "variety"
   density; otherwise body simply increments density.

Effect:
```c
cell->OverlayData += amount;             // typically +1 from growth
if (cell->OverlayData >= 11)
    cell->OverlayData = 11;
TiberiumClass::AddToSpreadQueue(cellCoord);  // FUN_00722AF0
```

**Does NOT call**: `RecalcAttributes`, `RadarClass::MarkTerrainDirty`, or any
LandType / cell-flag writer.

### Branch B — Germinate new ore (cell currently has no overlay)

Variant selection (verified):

```c
if (cell->SlopeType == 0) {
    // Flat cell — uniform pick from primary range
    variant = Random__RandomRanged(0, 0xB);  // [0, 11]
    overlay_idx = tib->Image->ArrayIndex + variant;
} else {
    // Sloped cell — pick from a 2-frame sub-range biased by slope
    variant = Random__RandomRanged(0, 1);
    overlay_idx = tib->Image->ArrayIndex
                + tib->NumImages          // +0xE8
                + (slopeIdx * 2)
                + variant
                - 2;
}
```

Effect:

```c
new OverlayClass(OverlayTypeArray[overlay_idx], &cellCoord, -1);
cell->OverlayData = density;             // from caller (3 for spread, table-driven for map-load)
TiberiumClass::AddToGrowthQueue(cellCoord);  // FUN_007235A0
RadarClass::MarkTerrainDirty(&cellCoord);    // ONLY in Branch B
```

### Fields that PlaceTiberium does **NOT** write

These are commonly mis-attributed to this function:

- `CellClass+0x140` bit 7 ("has tiberium" flag) — owned by `CellClass::RecalcAttributes` (`0x0047D2B0`).
- `CellClass+0xEC` LandType — owned by `RecalcAttributes`.

`RecalcAttributes` is the function that synthesizes both from the overlay type's
`Tiberium` (+0x2A9) and `Land` (+0x298) fields. PlaceTiberium itself only writes
`+0x44 OverlayTypeIndex` (via the OverlayClass constructor) and `+0x11E OverlayData`.

In normal flow, `RecalcAttributes` runs because the OverlayClass constructor
ultimately invokes it — but if you're tracing field writes, attribute them to
`RecalcAttributes`, not PlaceTiberium.

See `CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md` for full verified
decompilation.

---

## 11. Ore Reduction & Harvesting

### CellClass::Reduce_Tiberium (`0x00480a80`)

**Verified 2026-05-20** — all four §11 load-bearing claims confirmed against the
live binary. See `CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md` for
full verified decompilation.

```c
uint Reduce_Tiberium(CellClass* cell, uint amount) {
    int tibIdx = OverlayToTiberiumIndex(cell->OverlayTypeIndex);  // alias: IsWallOverlay (0x005FDD20)
    if (amount <= 0 || tibIdx == -1) return 0;
    // NOTE: OverlayToTiberiumIndex returns **0**, not -1, when the overlay
    // has Tiberium=yes but falls outside all TiberiumClass ranges. The
    // tibIdx != -1 guard does NOT catch this — a mismatched Tiberium=yes
    // overlay is processed as Riparius (index 0). Only a concern for mods
    // that introduce stray Tiberium=yes overlays; stock YR is unaffected.

    if (cell->OverlayData == 11) {
        // Density-11 detour: call AddToGrowthQueue (0x007235A0).
        // NO-OP in practice: AddToGrowthQueue has an internal `density < 11`
        // guard, and the density is still 11 at this call site (decrement has
        // not happened yet). Net effect: zero. Vestigial.
        TiberiumClass::AddToGrowthQueue(&cell->MapCoord);
    }

    uint current = cell->OverlayData;
    if (amount < current + 1) {
        // Partial reduction
        cell->OverlayData -= amount;
        return amount;
    } else {
        // Full removal
        cell->OverlayTypeIndex = -1;
        cell->OverlayData = 0;
        RecalcAttributes(cell);                           // synthesizes +0x140 bit-7 + +0xEC
        RadarClass::MarkTerrainDirty(&cell->MapCoord);

        // Clear this cell from every tiberium type's spread bitmap.
        // Scope confirmed = ALL tiberium types (`0x00722AB0` iterates
        // g_TiberiumClass_Array_Count). On large maps with multiple
        // tib types this is O(N_tib_types) per harvest tick.
        TiberiumClass::ClearSpreadBitmaps_AllTypes();

        // Re-seed the 8 neighbors into THIS tiberium's spread queue only,
        // so the patch can grow back into the void this cell just left
        // behind. EBP holds the TiberiumClass* resolved from tibIdx; no
        // outer loop over other tib types.
        for (dir = 0; dir < 8; dir++) {
            CellCoord n = cell->MapCoord + DirectionOffsets[dir];
            if (Cell_in_bounds(n) && tib->SpreadBitmap[cellIdx(n)] == 0) {
                TiberiumClass::AddToSpreadQueue(&n);    // 0x00722AF0
            }
        }
        // NOTE: the +0x122 byte (a wall-neighbor counter, see Section 4)
        // is NOT touched here — disassembly contains zero accesses to
        // base+0x122. It is decremented only by PostDestructionWallCleanup
        // (0x00480838), gated on OverlayTypeClass.Wall (+0x2A8).
        return current;
    }
}
```

### UnitClass::Harvest_Ore_Tick (`0x0073d450`)

Per-frame harvesting logic. Two type flags on UnitTypeClass gate which path runs:
- `+0xE0E` = `Harvester` (set on War Miner, Chrono Miner — standard YR ore harvesters)
- `+0xE0F` = `Weeder` (TS-legacy weed-collector flag; not set on any standard YR unit)

1. Bail if `UnitType.Harvester == 0`, the harvester's facing speed is over the threshold,
   or `cell.LandType != 5` (not a tiberium cell).
2. **Weeder branch** (`UnitType.Weeder != 0`): add a fixed 1.0 to slot 0,
   set timer = `RulesClass->HarvestRate * 3`, return. (Dead path in standard YR.)
3. **Standard ore harvester branch** (Harvester=yes, Weeder=no):
   a. Get tiberium type index via `OverlayToTiberiumIndex` (used as the StorageClass slot)
   b. Compute remaining capacity = `UnitType.Storage - StorageClass::GetTotalAmount()`
   c. Call `CellClass::Reduce_Tiberium(amount)` — returns levels actually removed
   d. Call `StorageClass::AddAmount((float)removed, tibIdx)`
   e. Set timer = `RulesClass->HarvestRate` (×1, NOT ×3) — `RulesClass + 0x1520`

---

## 12. Ore on Radar/Minimap

### OverlayClass::GetRadarColor (`0x005fed00`)

- For tiberium overlays in ranges 0x7F-0x8A or 0x93-0x9E (indices 127-138 or 147-158): swaps RGB channels
- Otherwise: uses `GetTiberiumRadarColor` which reads color from the overlay's SHP frame header

### GetTiberiumRadarColor (`0x0069e860`)

Reads the average color directly from the SHP frame header data (offset +0x0C in frame header, 3 bytes RGB).

---

## 13. Map Data Format

### OverlayPack
- INI section: `[OverlayPack]`
- Format: numbered keys → concatenate values → base64 decode → LCW decompress
- Result: 262,144 bytes (512×512 grid)
- Each byte: **overlay type index** (0xFF = no overlay)
- Grid position: `index = ry * 512 + rx`

### OverlayDataPack
- INI section: `[OverlayDataPack]`
- Same format as OverlayPack
- Each byte: **overlay frame/density data**
- For ore: density 0-11
- For walls: connectivity bitmask

**IMPORTANT:** The overlay type indices in OverlayPack correspond to the numeric keys in `[OverlayTypes]`, NOT sequential zero-based indices. Gaps in the INI numbering (e.g., 40-41 are missing) create empty slots in the engine's array. The current Rust implementation compacts these indices, which may cause incorrect overlay mapping.

---

## 14. INI Data Reference

### [Tiberiums] — Global Tiberium Type Registry

```ini
[Tiberiums]
0=Riparius    ; Green ore (standard)
1=Cruentus    ; Blue gems (valuable)
2=Vinifera    ; TS legacy — not used in YR maps
3=Aboreus     ; TS legacy — not used in YR maps
```

### Per-Tiberium INI Values

| Key | Riparius | Cruentus | Vinifera | Aboreus |
|-----|----------|----------|----------|---------|
| Image | 1 | 2 | 3 | 4 |
| Value | 25 | 50 | 25 | 25 |
| Growth | 2200 | 10000 | 2200 | 2200 |
| GrowthPercentage | 0.06 | 0 | 0.06 | 0.06 |
| Spread | 2200 | 10000 | 2200 | 2200 |
| SpreadPercentage | 0.06 | 0 | 0.06 | 0.06 |
| Power | 0 | 0 | 0 | 0 |
| Color | NeonGreen | NeonBlue | NeonBlue | NeonBlue |
| Debris | — | CRYSTAL1-4 | CRYSTAL1-4 | CRYSTAL1-4 |

**Key insight:** Gems (Cruentus) have `GrowthPercentage=0` and `SpreadPercentage=0`, meaning they **never grow or spread**. Only standard ore (Riparius) grows and spreads.

At 15fps: Growth/Spread interval of 2200 frames = ~146.7 seconds (~2.4 minutes between ticks).

### Per-Overlay-Type INI Values (TIB01-TIB20, GEM01-GEM12)

```ini
[TIB01]
Image=TIB01         ; SHP filename
Name=Tiberium
Tiberium=yes         ; Marks as harvestable ore
LegalTarget=false
RadarInvisible=false
RadarColor=220,200,0  ; YR addition (rulesmd.ini)

[GEM01]
Image=GEM01
Name=Gems
Tiberium=yes
LegalTarget=false
RadarInvisible=false
```

### OverlayTypes Index Map (runtime slots after section-order compaction)

```
Runtime slots 27-38:   GEM01-GEM12
Runtime slots 102-121: TIB01-TIB20
Runtime slots 127-146: TIB2_01-TIB2_20
Runtime slots 147-166: TIB3_01-TIB3_20

Raw numeric INI keys are respectively 28-39, 105-124, 130-149, and 150-169.
They are not the stored runtime indices.
```

---

## 15. CellClass::RecalcAttributes Tiberium Handling (`0x0047d2b0`)

When an overlay is present, RecalcAttributes:

1. Sets `cell->LandType` from `OverlayTypeClass->Land` (+0x298)
2. If LandType is Road(4), Clear(9), or `NoUseTileLandType=true`:
   - Recalculates SlopeIndex from terrain tile
   - **If slope ≥ 5 and overlay is tiberium: REMOVES the overlay** (ore can't exist on steep slopes)
3. If cell has tiberium and slope < 5:
   - If OverlayTypeClass->Land == 0: forces LandType to 5 (Tiberium)

This explains why ore never appears on steep cliff faces.

---

## 16. Implementation Notes for Rust Engine

### Current Implementation Status
- `src/map/overlay.rs`: OverlayPack/OverlayDataPack parsing ✓
- `src/map/overlay_types.rs`: OverlayTypeRegistry with flags ✓
- `src/sim/ore_growth.rs`: Growth/spread simulation ✓
- `src/sim/miner/`: Harvester cycle ✓

### Corrected Runtime-Index Finding

The previous claim that numeric `[OverlayTypes]` keys are direct runtime array
indices was false. `RulesClass::Process @ 0x00668CF9..0x00668D32` enumerates
the section in entry order and appends one `OverlayTypeClass` per entry.
Sequential compaction in Rust is therefore the correct storage model.

The remaining Rust drift is classification breadth:
`flat_tiberium_variant_ids` and `tiberium_overlay_mapping` recognize only the
twelve primary images, while native `OverlayToTiberiumIndex @ 0x005FDD20`
also checks `NumExtraImages`. Stock TIB13-TIB20 must map to Riparius.

### Key Constants for Implementation

```rust
const MAX_ORE_DENSITY: u8 = 11;       // OverlayData max value
const MAX_DENSITY_FRAMES: u8 = 12;    // +0xE4 field, = MaxDensity
const NUM_PRIMARY_OVERLAYS: u8 = 12;  // +0xE8 field
const NUM_EXTRA_OVERLAYS: u8 = 8;     // +0xEC field (ore only, 0 for gems)
const VARIETY_DENSITIES: [u8; 2] = [0, 9];  // Densities that get visual variety
const ORE_SHP_BODY_FRAMES: u8 = 13;   // Frames 0-12
const ORE_SHP_SHADOW_OFFSET: u8 = 13; // Shadow frame = body frame + 13
const SPREAD_JITTER_FRAMES: u32 = 50; // Random 0-49 frames added to queue timing
const SPREAD_INITIAL_DENSITY: u8 = 3; // Density when ore spreads to new cell

// Neighbor-density lookup table
const DENSITY_FROM_NEIGHBORS: [u8; 12] = [0, 1, 3, 4, 6, 7, 8, 10, 11, 7, 0, 1];

// Variety table (4×4, indexed by ((Y&3)<<2) | (X&3))
const VARIETY_OFFSETS: [u8; 16] = [
    0, 1, 2, 3,
    3, 2, 1, 0,
    2, 3, 0, 1,
    1, 0, 3, 2,
];
```
