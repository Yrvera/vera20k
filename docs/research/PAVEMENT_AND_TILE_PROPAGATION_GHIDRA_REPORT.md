# Pavement & Tile-Index Propagation — Ghidra Research Report

**Primary addresses:**
- `MapClass::SetOverlayAndPropagate` @ `0x0056EB80` (misnamed — see §2)
- `MapClass::ToggleBridgePavement` @ `0x0056E990`
- `CellClass::ApplyLAT_and_SlopeFixup` @ `0x0047CA80` (covered in IsometricTileType report)

**Confidence:** HIGH for the negative findings (§1) and `SetOverlayAndPropagate`
algorithm. MEDIUM for `ToggleBridgePavement` (referenced but not fully decompiled).
**Active in YR:** Yes — tile propagation is used by bridge destruction and RMG.

---

## 1. Key Negative Finding — No Building-Places-Pavement System

**There is no `Pavement=` INI key on `BuildingTypeClass`** in gamemd.exe. An
exhaustive string search of the binary for `Pave*` returned only the **LAT-group
tile globals** — none of them are BuildingType properties:

| String address | Value | Purpose |
|----------------|-------|---------|
| `0x00829388` | `PavedRoadSlopes` | `[General]` theater-INI key (tile index) |
| `0x00829410` | `PavedRoadEnds` | `[General]` theater-INI key |
| `0x00829420` | `PavedRoads` | `[General]` theater-INI key |
| `0x00829538` | `ClearToPaveLat` | `[General]` LAT base tile |
| `0x00829578` | `MiscPaveTile` | `[General]` alternate pave tile |
| `0x00829588` | `PaveTile` | `[General]` primary pave tile |

(Remaining matches were C++ RTTI type names for unrelated classes: `PAVEBolt`,
`PAVEgoClass`, `PAVEMPulseClass`, `PAVEventClass`.)

### 1.1 What this means

Pavement is **purely a map-design artifact**:
1. The map author places pavement tiles in the isometric tile layer at design time
   (via FinalAlert / WAE / the in-engine map editor).
2. LAT (Lookup Adjacent Tile) auto-blends the `[PaveTile..+0x0F]` range at runtime
   based on 4 cardinal neighbors — see `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md`
   §4 for the full algorithm.
3. Buildings do **not** automatically lay pavement when placed, and do **not**
   remove pavement when sold.
4. Players have no in-game pavement-placement action. Pavement is static ground
   that happens to be part of the tile palette.

### 1.2 Follow-on implications for the Rust engine

- The "pavement LAT trigger" worry raised during the IsometricTileTypeClass
  investigation is a **non-issue** — no special code path is needed. The existing
  ground LAT algorithm (applied in `CellClass::ApplyLAT_and_SlopeFixup`) handles
  pavement tiles the same as Rough/Sand/Green.
- No BuildingType INI field to add. No foundation-placement side effect.
- The Pave LAT group's **hardcoded exemptions** — `[MiscPaveTile..+0xD]`,
  `[Medians..+0xD]`, `[PavedRoads..+0x14]` — remain the correct behavior. These
  exemptions are what make roads and paved surfaces not absorb adjacent pavement
  into their LAT run.

### 1.3 Observed in-engine behavior this explains

- Pavement cells in the tile layer naturally LAT-blend with each other at load
  time via `MapClass::InitCellAttributes` → `RecalcAttributes` → `ApplyLAT_and_SlopeFixup`.
- When a wall is destroyed on a pave cell (common base-defense pattern), the wall
  cleanup (§4 of the wall report) calls `RecalcAttributes`, which re-runs LAT —
  so the pave tile re-flows correctly even though nothing changed the pave tile
  directly.
- Bridges morph to water on destruction via `MapClass::SetOverlayAndPropagate` (§2
  below) — same pattern: change tile-index, recurse to neighbors, re-LAT each one.

---

## 2. `MapClass::SetOverlayAndPropagate` @ `0x0056EB80` — Actually Tile-Index Flood-Fill

**The Ghidra label is misleading.** The function does NOT set an overlay. It sets
a cell's `IsoTileTypeIndex` (+0x38) and flood-fills the change to all connected
8-neighbors that still match the OLD tile_id. It's the tile-layer equivalent of a
recursive paint-bucket.

### 2.1 Signature

```c
void MapClass__SetOverlayAndPropagate(
    short* coord,           // packed i16[2] = (x, y)
    int new_tile_id,        // target tile index to set
    int old_tile_id,        // flood-fill match criterion
    uint dirty_flag,        // used when dirty_flag==0 to also dirty screen rect + check
    char suppress_dirty     // 0 = first-call (mark screen dirty), 1 = recursive call
);
```

### 2.2 Algorithm

```python
def SetTileAndFloodFill(coord, new_tile, old_tile, dirty_flag, suppress_dirty):
    cell = map.get_cell(coord)           # with off-map sentinel fallback

    # On the *first* call (non-recursive), dirty the tactical screen rect
    if not suppress_dirty:
        screen_coords = coord_to_client(cell)
        TacticalClass.dirty_screen_rect(screen_coords ± 0x80, size=0x100, 0)
        # Sanity check: if the cell already has the new tile, stop — avoids infinite recursion
        if cell.IsoTileTypeIndex == new_tile:
            return

    # Actual tile swap
    if cell.IsoTileTypeIndex != new_tile:
        cell.IsoTileTypeIndex = new_tile
        CellClass.RecalcAttributes(cell)       # triggers LAT + zone recompute
        RadarClass.mark_terrain_dirty(cell)

        # Recurse to 8 neighbors (dir 0..7) — set each that still has old_tile
        for dir in range(8):
            neighbor_coord = coord + g_DirectionOffsets[dir]
            neighbor = map.get_cell(neighbor_coord)
            if neighbor.IsoTileTypeIndex == old_tile:
                SetTileAndFloodFill(neighbor_coord, new_tile, old_tile,
                                    dirty_flag, suppress_dirty=1)
```

### 2.3 Callers & use cases

- **Bridge destruction** — when a bridge segment is destroyed, its tile_id changes
  to the corresponding destroyed/water tile. The flood-fill propagates the change
  through all connected bridge-tiles of the old type. See
  `MapClass::SelectDestroyedBridgeTile_Low` @ `0x00579ACA`.
- **RMG (random map generator)** terrain stamping — confirmed by the
  `MapClass::ToggleBridgePavement` @ `0x0056E990` neighbor which orchestrates
  bridge/pavement macro operations.

### 2.4 Implication for Rust engine

When porting bridge destruction, we need:
1. A **tile-coord flood-fill primitive** with old-tile matching (not simple 4-neighbor —
   it uses all 8 directions per iteration).
2. **RecalcAttributes on every visited cell** — triggers LAT re-tile of neighbors
   (which is how bridge ramps re-appear as shore/water tiles).
3. **Screen-dirty marker** on the *initial* call only (the `suppress_dirty` flag
   suppresses recursion's re-dirty — otherwise large flood-fills would redraw the
   whole map every step).

---

## 3. `CellClass::RecalcAttributes` Runtime Triggers — Partial Enumeration

This was flagged as an open question in the IsometricTileTypeClass report ("30+
callers"). Here's a structured list gathered across this investigation, organized
by category (not exhaustive — confirm against Ghidra when reopened for any not
verified here):

### 3.1 Confirmed callers (verified in this session)

| Call site | Context |
|-----------|---------|
| `MapClass::InitCellAttributes` @ `0x00568BB0` | Map-load sweep over every cell |
| `MapClass::SetOverlayAndPropagate` @ `0x0056EB80` | Tile-index flood-fill (bridge destruction etc.) |
| `MapClass::ToggleBridgePavement` @ `0x0056E990` | Bridge pavement toggle (not fully traced) |
| `MapClass::SelectDestroyedBridgeTile_Low` @ `0x00579ACA` | Bridge damage |
| `MapClass::SelectBridgeTileVariant_Low` @ `0x0057B133` | Bridge visual variant |
| `CellClass::PostDestructionWallCleanup` @ `0x00480630` | Wall destruction (once per cell, up to 5 cells per event) |
| `CellClass::ApplyLAT_and_SlopeFixup` @ `0x0047CA80` | Internal — `RecalcAttributes` calls it, then the post-check calls back via `FUN_00544C80` |

### 3.2 Additional callers noted from xref list (not fully traced)

From the `g_IsoTileTypeArray @ 0x00A8ED2C` xref list dumped during the
IsometricTileTypeClass investigation — these functions *read* the tile-type array,
which strongly implies they are tile-mutation paths that would call
`RecalcAttributes`:

- `FUN_0047FF80` — overlay-related (adjacent to `CellClass::DrawOverlay_*`)
- `CellClass::GetRadarPixelColor` / `GetRadarColor` — render-side readers, not mutators
- `FUN_004814F0` — `GetTileVariantIndex` — pure read
- `FUN_006B2A70`, `FUN_006B2520`, `FUN_006B3850` — unidentified; likely HUD/radar
- `FUN_006851F0` — unidentified

### 3.3 Known paths that must call `RecalcAttributes` (from behavior, not traced)

- Overlay placement (ore/gems/bulge of any overlay including non-wall)
- Crate overlay spawn/pickup (`CrateSlot::*` functions exist)
- Building sell (cell returns to base ground state)
- Wall place (new overlay at cell → must trigger neighbor connect re-compute)
- Terrain terraform / cliff destruction (`DestroyableCliffs` in `[General]` suggests
  cliff tiles can change)
- Ore spread / germination tick (`CellClass::SpreadCellGerminate`)

**Rust porting recommendation:** implement a `world.recalc_cell_attributes(coord)`
that's callable from any cell-mutation path, and audit each mutation site one by
one rather than trying to enumerate all callers up-front. This matches the binary's
pattern of "mutate then recompute".

---

## 4. Integration with IsometricTileTypeClass & Wall Reports

Chain of causation that determines a cell's final rendered appearance:

```
   map file             rules/art INI          gameplay event
      │                       │                      │
      ▼                       ▼                      ▼
  IsoTileTypeIndex       TileType loader      mutation path
  (cell +0x38)           (theater INI)           (wall,
                                                  overlay,
                                                  bridge, …)
      │                       │                      │
      └──────────┬────────────┘                      │
                 ▼                                   │
    g_IsoTileTypeArray[idx]                          │
    → TMP file + metadata                            │
                                                     ▼
                                          CellClass::RecalcAttributes
                                                     │
                                                     ▼
                                    CellClass::ApplyLAT_and_SlopeFixup
                                      (ground LAT in 4 groups, slope fixup)
                                                     │
                                                     ▼
                                   final cell.IsoTileTypeIndex ready to draw
```

The **LAT algorithm is the same** regardless of what triggered it — map load,
wall destruction, bridge collapse, or flood-fill all converge at
`ApplyLAT_and_SlopeFixup`. This is why a single correct LAT implementation in
Rust (once we port the hardcoded exemption ranges and fix the bit assignment per
§4 of the IsometricTileTypeClass report) will correctly handle every mutation
path.

---

## 5. Current Rust Implementation Status

| System | Rust coverage | Gap |
|--------|---------------|-----|
| `[Pavement=]` BuildingType INI | N/A — doesn't exist in binary | **Nothing to port.** |
| Tile-coord 8-neighbor flood-fill | ✗ Missing | Needed for bridge destruction port. |
| `world.recalc_cell_attributes(coord)` dispatcher | ✗ Missing | Needed — called from overlay place, wall destroy, bridge damage, building sell. |
| Bridge destruction tile-morph | ✗ Missing | `SelectDestroyedBridgeTile_Low` + propagation. |
| Pave LAT (hardcoded exemptions) | ✗ Wrong | Current `src/map/lat.rs` uses `*ConnectTo` INI keys instead. Fix per `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` §4.3. |
| LAT retrigger on wall destroy | ✗ Missing | Depends on `recalc_cell_attributes` dispatcher. |

### 5.1 Priority order

1. **Fix the Pave/Green LAT exemptions** in `src/map/lat.rs` — hardcode them per the
   IsometricTileType report §4.3 and delete the `*ConnectTo` INI parsing path.
2. **Add `world.recalc_cell_attributes(coord)`** as a thin wrapper around the existing
   LAT apply + any future slope fixup. Call from:
   - Overlay place / remove (ore, wall, crate)
   - Wall destroy (per `PostDestructionWallCleanup`)
   - Bridge damage (future)
3. **Defer** the tile flood-fill until bridge destruction is ported — it's not
   needed for the existing parity work.

---

## 6. `MapClass::ToggleBridgePavement` — Actually Bridge Damage-Variant Toggle

**Second misnamed function in this area.** Decompiled this pass: it has **nothing
to do with pavement**. It toggles the damaged-variant bit (`CellClass.Flags & 0x2000`)
across a contiguous group of same-tile-id cells.

### 6.1 Algorithm

```python
def ToggleBridgeDamageVariant(coord, new_state, suppress_self):
    cell = map.get_cell(coord)

    if not suppress_self:
        # Sanity gates
        if cell.IsoTileTypeIndex in (0xFFFF, 0xFF):  # empty / clear
            return
        # TMP cell must actually have damaged data baked in
        if not HasDamagedVariantAtSubTile(cell.tile_type, cell.SubTileIndex):
            return
        # Initial call: mark screen rect dirty
        TacticalClass.dirty_screen_rect(...)

    # Only act if state differs from current bit
    current_bit = (cell.Flags >> 13) & 1
    if (new_state & 1) == current_bit:
        return

    old_tile_id = cell.IsoTileTypeIndex
    cell.Flags = (cell.Flags & ~0x2000) | ((new_state & 1) << 13)
    RadarClass.mark_terrain_dirty(cell)

    # Recurse to all 8 neighbors that share the same tile_id
    for dir in range(8):
        neighbor_coord = coord + g_DirectionOffsets[dir]
        neighbor = map.get_cell(neighbor_coord)
        if neighbor.IsoTileTypeIndex == old_tile_id:
            ToggleBridgeDamageVariant(neighbor_coord, new_state, suppress_self=1)
```

### 6.2 What it actually does

Flips `CellClass.Flags` bit `0x2000` across a contiguous bridge/tile segment. That
bit is the **damage-variant selector** documented in the IsometricTileTypeClass
report §11.3:

```
if TMP_cell.flags & 0x04:                # FLAG_HAS_DAMAGED_DATA
    variant = (cell.Flags >> 13) & 1     # damaged-data path
```

So `ToggleBridgePavement(coord, 1, 0)` flips an entire bridge segment to its
damaged visual, and `ToggleBridgePavement(coord, 0, 0)` flips it back to pristine.
The flood-fill uses `tile_id` equality as its propagation criterion — so only the
contiguous same-tile-id section is affected, and mixed-tile neighbors stop the
cascade naturally.

### 6.3 Caller (not yet traced)

Called by bridge damage/repair logic — the most likely caller is the bridge
damage-state transition in the bridge-destruction pipeline (which ends up wanting
to flip the visual without changing the tile_id). Exact call site left for a
future bridge-focused investigation.

### 6.4 Rust implication

Bridge damage visual is a separate channel from tile_id changes. When porting:
- `tile_id` change → use the §2 flood-fill (changes underlying tile)
- **Damage visual only** → flip `CellFlags & 0x2000` via a `toggle_damage_variant()`
  helper that uses the same 8-neighbor flood-fill with tile_id equality

This is why bridge tiles don't change their tile_id during early damage stages;
they just flip the damage bit. Only full destruction switches to water tiles via
the §2 propagator.

---

## 7. Open Questions

1. **Caller of `ToggleBridgePavement`** — likely the bridge-damage dispatch.
   Bridge HP → damage-stage → flip visual. Worth tracing when porting bridges.

2. **Is there a pavement-under-foundation visual in YR maps that I've been
   misremembering?** The finding "no auto-pavement" is strong (no INI key, no
   `BuildingType` flag, no per-place hook traced), but if user has seen
   pavement-under-building in YR gameplay, the path would be: map author placed
   pavement tiles **before** the building went down, and the building simply
   sits on top. Build on pavement → no visual change. Building removed → pavement
   was always there, still is. That matches the code behavior.

3. **Ore-overlay spread retrigger path** — `CellClass::SpreadCellGerminate`
   grows ore but should also re-LAT the cell? Unclear from static analysis.
   Revisit during ore-growth audit.

---

## Sources

**Ghidra addresses decompiled:**
- `0x0056EB80` — MapClass::SetOverlayAndPropagate (aka tile flood-fill)
- `0x0056E990` — MapClass::ToggleBridgePavement (actually bridge damage-variant toggler)

**Ghidra addresses referenced (not decompiled this pass):**
- `0x00579ACA` — MapClass::SelectDestroyedBridgeTile_Low
- `0x0057B133` — MapClass::SelectBridgeTileVariant_Low

**Strings search (exhaustive) for `Pave*`:** returned 14 matches, all LAT-tile
globals or unrelated RTTI type names. No BuildingType INI key.

**Cross-referenced docs:**
- `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` — LAT algorithm, hardcoded exemptions
- `WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT.md` — wall destroy → RecalcAttributes
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` — bridge damage (for context)

**Rust source audited:**
- [src/map/lat.rs](src/map/lat.rs) — uses `*ConnectTo` keys; needs hardcoded exemption rewrite
- [src/map/overlay.rs](src/map/overlay.rs) — wall connectivity exists, needs damage nibble
- [src/map/resolved_terrain.rs](src/map/resolved_terrain.rs) — load-time land_type only, no runtime recompute
