# Cell Validation for Tiberium Placement (FUN_004838e0)

**Date:** 2026-04-03
**Method:** Ghidra MCP decompilation of gamemd.exe
**Confidence:** High — all findings from direct binary analysis
**Function:** `0x004838e0` — CellClass::CanPlaceTiberium

---

## Overview

Called before placing ore via spread (`FUN_00483780`) or TIBTRE spawning. Takes a
CellClass pointer (`param_1` typed as `int`, so all offsets are direct byte offsets).
Returns 1 if tiberium can be placed, 0 otherwise.

All 8 checks must pass (short-circuit on first failure).

---

## Check 1: Cell in playfield

```c
MapClass__Is_Cell_In_Playfield(param_1 + 0x24, 1)
```

CellClass+0x24 = cell coordinates. Cell must be within playable map bounds.

**Our status:** Implemented via bounds check in `can_germinate()`.

---

## Check 2: No bridge/impassable flags

```c
(*(uint *)(param_1 + 0x140) & 0x500) == 0
```

CellClass+0x140 = cell flags bitfield.
- Bit 0x100 = bridge structural cell
- Bit 0x400 = bridge rail/guard post

Combined mask 0x500: ore cannot be placed on any bridge cell.

**Our status:** NOT IMPLEMENTED directly, but data available.

**Engine data:** `ResolvedTerrainCell.has_bridge_deck: bool` — accessible via
`self.resolved_terrain.cell(rx, ry)`. One-line check: `if cell.has_bridge_deck { return false; }`

---

## Check 3: Building exclusion

Iterates cell's object list at CellClass+0xE4, looking for RTTI type 6 (BuildingClass).

If a building is found with health > 0:
- Gets BuildingTypeClass pointer from building (object+0x520)
- Checks **offset 0xC9A** on BuildingTypeClass = `Invisible` (bool)
- Checks **offset 0x1701** on BuildingTypeClass = `InvisibleInGame` (bool)

**Logic:** If `Invisible == false` AND `InvisibleInGame == false` → **REJECT**.
Only invisible buildings (like spawn markers) allow ore on their cells.

Note: This is NOT `AllowTiberium` or `TiberiumProof` as originally assumed in the
design doc. Those flags exist but are checked elsewhere (ore damage context, not
placement context). The placement check only cares about building visibility.

**Our status:** NOT IMPLEMENTED.

**Engine data:** `OccupancyGrid.has_blockers_on(cell, Ground)` — available via `self.occupancy`.
No standard YR building has `Invisible=yes` or `InvisibleInGame=yes`, so checking for any
building blocker on the cell is functionally identical to the binary's visibility check.
Implementation: pass `&self.occupancy` to `can_germinate()`, reject if building present.

---

## Check 4: Terrain object (SpawnsTiberium) exclusion

Second loop through object list at CellClass+0xE4, looking for RTTI type 0x24
(36 = TerrainClass).

If a terrain object is found:
- Gets TerrainTypeClass pointer (object+0xC8)
- Checks **offset 0x2B1** on TerrainTypeClass = `SpawnsTiberium` (bool)

**Logic:** If `SpawnsTiberium != 0` → **REJECT**.

Ore cannot be placed on the cell occupied by a tiberium-spawning tree. The tree
spawns ore on *adjacent* cells, not its own cell.

**Our status:** NOT IMPLEMENTED — planned in Task 6.

**Engine data:** Task 6 creates `terrain_objects: Vec<TerrainObjectState>` with per-object
`cell` and `spawns_tiberium` fields. Pass slice to `can_germinate()`, check
`terrain_objects.iter().any(|t| t.cell == (rx, ry) && t.spawns_tiberium)`.
For performance, a `HashSet<(u16,u16)>` of spawner cells could be pre-built at map load.

---

## Check 5: Land type Buildable flag

```c
(&DAT_0089ea60)[*(int *)(param_1 + 0xec) * 0x24] != '\0'
```

CellClass+0xEC = LandType (int enum).

The table base is at 0x0089ea40 with 36-byte (0x24) entries per land type.
DAT_0089ea60 = base + 0x20 = the `Buildable` bool field within each entry.
Table populated at runtime from `[SpeedType]` section via
`RulesClass::ReadSpeedTypeLandTypeTable`.

### Land Type → Buildable Mapping

| Index | Land Type | Buildable | Ore Allowed? |
|-------|-----------|-----------|--------------|
| 0 | Clear | **yes** | **YES** |
| 1 | Road | **yes** | **YES** |
| 2 | Water | no | NO |
| 3 | Rock | no | NO |
| 4 | Wall | no | NO |
| 5 | Tiberium | no | NO |
| 6 | Beach | no | NO |
| 7 | Rough | **yes** | **YES** |
| 8 | Ice | no | NO |
| 9 | Railroad | no | NO |
| 10 | Tunnel | no | NO |
| 11 | Weeds | no | NO |

Only Clear, Road, and Rough terrain allows ore placement.

Note: LandType=5 (Tiberium) is the land type that cells WITH ore already have.
Since check 6 (no existing overlay) would already reject these cells, this check
is redundant for that case but provides defense-in-depth.

**Our status:** NOT IMPLEMENTED.

**Engine data:** `ResolvedTerrainCell.terrain_class: TerrainClass` — directly available.
Implementation: `matches!(cell.terrain_class, TerrainClass::Clear | TerrainClass::Road | TerrainClass::Rough)`.
These three match the binary's Buildable=yes land types exactly.

---

## Check 6: No existing overlay

```c
*(int *)(param_1 + 0x44) == -1
```

CellClass+0x44 = OverlayTypeIndex. Must be -1 (no overlay present).

Ore cannot be placed on cells with any existing overlay — walls, existing ore,
crates, etc.

**Our status:** IMPLEMENTED — `resource_nodes.contains_key(&cell)` check.

---

## Check 7: No terrain slope (SlopeIndex == 0)

```c
*(char *)(param_1 + 0x11c) == '\0'
```

CellClass+0x11C = SlopeIndex/SlopeType. Must be 0 (flat ground).

**IMPORTANT:** Earlier research docs incorrectly identified this as `DamageState`.
It is actually the cell's slope type. Ore cannot be placed on ramped/sloped cells.

**Our status:** NOT IMPLEMENTED.

**Engine data:** `ResolvedTerrainCell.has_ramp: bool` (derived from `slope_type != 0`).
Implementation: `if cell.has_ramp { return false; }`. One-line check.

---

## Check 8: Tile type AllowTiberium flag

```c
uVar3 = *(uint *)(param_1 + 0x38);  // tile type index
if ((int)uVar3 < 0 || DAT_00a8ed38 <= (int)uVar3) {
    // Invalid index → pass (safety fallback)
} else {
    char flag = *(char *)(*(int *)(DAT_00a8ed2c + uVar3 * 4) + 0x306);
    if (flag == '\0') → REJECT
}
```

- CellClass+0x38 = IsometricTileTypeClass array index
- DAT_00a8ed2c = IsometricTileTypeClass pointer array
- DAT_00a8ed38 = tile type count
- **Offset 0x306** on IsometricTileTypeClass = `AllowTiberium` (bool)

Per-tileset flag from theater INI — `[TileSetNNN] AllowTiberium=yes/no`.
Only certain tile types (grass, dirt) allow ore. Pavement, roads, water tiles
block it.

**Our status:** NOT IMPLEMENTED — **DEFERRED**.

**Engine data:** NOT PARSED. `AllowTiberium` is per-tileset in theater INI (e.g.,
`[TileSet0000] AllowTiberium=true` in `temperatmd.ini`). ~28 of ~150 tilesets have it.
Requires: parse flag during theater loading → propagate to `ResolvedTerrainCell`.
Deferred because: most gameplay-relevant cells (grass, dirt) allow tiberium, and
pavement/concrete already blocked by land type check (Check 5). Main gap: decorative
concrete tiles that are TerrainClass::Clear but have `AllowTiberium=false`.

---

## Engine Data Availability (verified 2026-04-03)

Per-cell data in our engine, mapped to each check:

| Check | Binary Field | Our Data Source | Available in sim/? |
|-------|-------------|----------------|-------------------|
| 2. Bridge | CellFlags & 0x500 | `ResolvedTerrainCell.has_bridge_deck` | YES — `self.resolved_terrain.cell(rx,ry)` |
| 3. Building | Object list RTTI==6, Invisible/InvisibleInGame | `OccupancyGrid.has_blockers_on(cell, Ground)` | YES — `self.occupancy` |
| 4. TIBTRE | Object list RTTI==0x24, SpawnsTiberium | Task 6 `terrain_objects: Vec<TerrainObjectState>` | Planned — Task 6 creates this |
| 5. Land type | `(&DAT_0089ea60)[land_type * 0x24]` Buildable flag | `ResolvedTerrainCell.terrain_class` | YES — match against Clear/Road/Rough |
| 7. Slope | `CellClass+0x11C == 0` | `ResolvedTerrainCell.has_ramp` | YES |
| 8. AllowTiberium | `IsometricTileTypeClass+0x306` | **NOT PARSED** from theater INI | NO — needs theater parser change |

### Key findings:

- **Check 3 (Building):** Binary checks `Invisible` and `InvisibleInGame` flags. No standard
  YR building has either flag set, so checking `occupancy.has_blockers_on()` is functionally
  identical. No need to parse those INI fields.
- **Check 5 (Land type):** Binary uses a `Buildable` field in the SpeedType/LandType table.
  Our `TerrainClass` enum maps directly: Clear, Road, Rough = allow ore. Water, Rock, Cliff,
  Beach, Ice, Tiberium, Weeds, Wall, Railroad, Tunnel = block ore.
- **Check 8 (AllowTiberium):** Per-tileset in theater INI (e.g., `[TileSet0000] AllowTiberium=true`).
  ~28 of ~150 tilesets have it (grass, dirt). Not currently parsed. Main effect: prevents ore
  on decorative concrete/pavement tiles. Rare gameplay impact — deferred.

## Summary: Implementation Priority

| Check | Description | Implemented | Engine Data | Effort | Priority |
|-------|------------|-------------|-------------|--------|----------|
| 1 | Playfield bounds | YES | — | — | — |
| 2 | Bridge flags (0x500) | PARTIAL | `has_bridge_deck` | Trivial (1 line) | Medium |
| 3 | Building exclusion | NO | `OccupancyGrid` | Easy (pass to fn) | Medium |
| 4 | SpawnsTiberium exclusion | NO | Task 6 terrain_objects | Already planned | **High** |
| 5 | Land type Buildable | NO | `terrain_class` | Trivial (1 match) | **High** |
| 6 | No existing overlay | YES | `resource_nodes` | — | — |
| 7 | Slope type == 0 | NO | `has_ramp` | Trivial (1 line) | Medium |
| 8 | Tile AllowTiberium | NO | **Not parsed** | Medium (theater parser) | Low (defer) |

Recommended: Add checks 2, 3, 5, 7 to `can_germinate()` in Task 4 (all data available).
Check 4 comes from Task 6. Check 8 deferred (theater parser change, rare edge case).

---

## Density-from-Neighbors Lookup Table (0x0081CD28)

**12 entries (int32), 48 bytes:**

| Neighbor Count | Initial Density |
|---------------|----------------|
| 0 | 0 |
| 1 | 1 |
| 2 | 3 |
| 3 | 4 |
| 4 | 6 |
| 5 | 7 |
| 6 | 8 |
| 7 | 10 |
| 8 | 11 |
| 9 | 7 |
| 10 | 0 |
| 11 | 1 |

**Usage:** Map load seeding ONLY (called from `FUN_00568bb0`). Counts how many of 8
neighboring cells have the same overlay type, then looks up `table[count]` to set
initial overlay density. **NOT used during runtime spread** — runtime always uses
density 3 (hardcoded in `FUN_00483780` → `FUN_00487190(type, 3)`).

This table affects initial map appearance but not gameplay growth/spread mechanics.
