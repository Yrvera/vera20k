# OverlayClass & OverlayTypeClass — Ghidra Research Report

**Primary Addresses:** OverlayClass::Constructor `0x005FC380`, OverlayTypeClass::Constructor `0x005FE250`, OverlayTypeClass::ReadINI `0x005FE770`, ObjectTypeClass::ReadINI `0x005F92D0`
**Confidence:** HIGH (offsets verified from binary; re-verified 2026-04-22)
**Active in YR:** Yes — overlays are core to YR maps (ore, gems, walls, bridges, tracks, crates, rocks)

**Re-investigation 2026-04-22 corrections** — this revision fixes several inherited-field offsets in the prior report (ObjectTypeClass offsets were partially mislabeled), resolves multiple LOW-confidence open questions, and re-verifies the chain-reaction, draw-variant, and harvest formulas. Changes are flagged **[corrected]** in-line below.

**Live re-verification 2026-07-24:** The complete signed
`TiberiumClass::ReadINI Image=` switch and reread behavior were decoded. The
reader uses default `-1`; omitted case writes preserve the existing object
fields. The corrected table appears in section 3a.

## 1. Overview

Overlays are map-level visual and logical objects: ore/gem fields, destructible walls, bridge
decks, railroad tracks, crates, rocks, fences, and rubble. Unlike units or buildings, overlays
follow a **stamp-and-forget** pattern: an `OverlayClass` instance is constructed, its placement
routine writes the overlay type index and data byte into the target `CellClass`, and the
`OverlayClass` instance itself is not retained as a persistent per-cell object during gameplay.
The cell owns the runtime overlay state via `OverlayTypeIndex` (+0x44) and `OverlayData` (+0x11E).

`OverlayTypeClass` is the type definition loaded from INI. It inherits from `ObjectTypeClass`
and holds all per-type properties (Wall, Tiberium, strength, flags). A global array of overlay
type instances is created at load from `[OverlayTypes]` (253+ entries in YR).

Walls are placed as `BuildingClass` instances that write overlay data into cells. Wall
connectivity and destruction are driven by `BuildingClass::ConnectWalls` and
`CellClass::DestroyOverlay` respectively.

## 2. OverlayClass Layout

**Instance size:** 0xB0 (176 bytes)
**RTTI:** 0x14 (20)
**Inherits:** ObjectClass → AbstractClass
**Primary vtable:** `0x7EF3D4`
**Global array:** pointer at `0x00A8EC54` (`g_OverlayClass_Array`), count at `0x00A8EC60`, capacity at `0x00A8EC58`

| Offset | Type | Field | Evidence |
|--------|------|-------|----------|
| +0x00 | ptr | vtable (primary) | Constructor sets 0x7EF3D4 |
| +0x04 | ptr | vtable (secondary: IRTTITypeInfo) | Constructor sets 0x7EF3B0 |
| +0x08 | ptr | vtable (secondary: INoticeSink) | Constructor sets 0x7EF3CC |
| +0x0C | ptr | vtable (secondary: INoticeSource) | Constructor sets 0x7EF3A8 |
| +0x10–0xAB | — | Inherited ObjectClass fields | See ABSTRACTCLASS / OBJECTCLASS reports |
| +0xAC | ptr | OverlayTypeClass* (type reference) | `GetType` returns `*(this+0xAC)` |

OverlayClass is minimal — almost all state lives in CellClass after placement.

### OverlayClass Methods (from vtable at 0x7EF3D4)

| Address | Method | Vtable Slot | Notes |
|---------|--------|-------------|-------|
| `0x005FC380` | Constructor | — | Calls ObjectClass::Constructor; sets type at +0xAC; registers in global array |
| `0x005FDF70` | Destructor | [8] | Removes from global array, clears +0xAC |
| `0x005FDF10` | GetClassID | [3] | Returns CLSID GUID from 0x7E96B0 |
| `0x005FD8F0` | Load | [5] | Restores from save, re-sets vtable ptrs |
| `0x005FD950` | Save | [6] | Delegates to ObjectClass::Save |
| `0x005FDF50` | What_Am_I | [11] | Returns 0x14 (20 = RTTI_OVERLAY) |
| `0x005FDDE0` | GetType | [34] | Returns *(this+0xAC) |
| `0x005FD270` | Unlimbo | [54] | Lepton→cell conversion, blocking check, delegate to ObjectClass::Reveal. **[corrected]** — prior revision said slot [53], but that slot holds the inherited `ObjectClass::Conceal` (0x005F4D30); OverlayClass::Unlimbo override is at slot [54]. Verified by memory read of vtable 0x7EF3D4. |
| `0x005FED00` | GetRadarColor | — | Reads OverlayTypeClass ArrayIndex; byte-swaps radar color for ArrayIndex in [0x7F-0x8A] = TIB2_01-12 (Vinifera) or [0x93-0x9E] = TIB3_01-12 (Aboreus). **[corrected]** prior report said "bridge byte-swap" — actual purpose is to distinguish the four Tiberium variants on radar. |

### OverlayClass Placement Flow (verified)

```
1. OverlayTypeClass::CreateInstance (0x005FE530) allocates 0xB0 bytes
2. OverlayClass::Constructor(typePtr, cellCoords, frame) called
3. Constructor → ObjectClass::Constructor
4. AbstractClass::AssignUniqueID (sets AbstractClass+0x10 UniqueID)
5. Register in global array at g_OverlayClass_Array (0x00A8EC54):
     if count < capacity or vector can grow, append pointer
6. Convert cell coords to leptons:  leptonXY = cellXY * 256 + 128
7. MapClass::Get_CellClass(packed leptons) → target cell
8. FUN_0047c550(0) searches cell's FirstObject/AltObject linked list
   for an existing object of RTTI 0x24 (blocking) → returns non-zero if blocked
9. If NOT blocked: ObjectClass::Reveal(leptonCoords, 0)
     Reveal is the virtual that actually writes the cell fields:
       CellClass +0x44  = OverlayTypeIndex (type's ArrayIndex)
       CellClass +0x11E = OverlayData (density / wall frame / bridge state / crate type)
10. Cell now owns the overlay; OverlayClass instance is ephemeral
```

## 3. OverlayTypeClass Layout

**Instance size:** 700 bytes (0x2BC)
**RTTI:** 0x15 (21)
**Inherits:** ObjectTypeClass → AbstractTypeClass → AbstractClass
**Primary vtable:** `0x7EF600`
**Global array:** pointer at `0x00A83D84` (`g_OverlayTypeClass_Array`), count at `0x00A83D90`, capacity at `0x00A83D88`

### Inherited Fields from ObjectTypeClass / AbstractTypeClass **[corrected]**

These offsets were partially mislabeled in the prior revision. The authoritative source is
`ObjectTypeClass::ReadINI` at `0x005F92D0`, decompiled and cross-checked 2026-04-22.

| Offset | Type | Field | INI Key | Default (overlay) | Evidence |
|--------|------|-------|---------|-------------------|----------|
| +0x24 | char[32] | Name | — | section name | AbstractTypeClass — used as INI section key |
| +0x98 | byte[3] | RadialColor | `RadialColor=` | 0,0,0 | ObjectTypeClass::ReadINI |
| +0x9C | int | Armor | `Armor=` | — (forced to 6=Special when Tiberium=true) | ObjectTypeClass::ReadINI |
| +0xA0 | int | Strength (base-class) | `Strength=` | — | Overlay-specific Strength lives at +0x2A4 |
| +0xA4 | ptr | SHP image pointer | — | — | Filled by LoadFileFromMIX |
| +0x1E8 | bool | NoSpawnAlt | `NoSpawnAlt=` | false | ObjectTypeClass::ReadINI |
| +0x1F0 | int | CrushSoundIdx | `CrushSound=` | -1 | VocClass index |
| +0x1F4 | int | AmbientSoundIdx | `AmbientSound=` | -1 | VocClass index |
| +0x1F8 | char[25] | Image name | `Image=` | section name | Overrides section name for SHP lookup |
| +0x211 | bool | AlternateArcticArt **[corrected]** | `AlternateArcticArt=` | false | Previously placed at +0x22C |
| +0x213 | char[25] | AlphaImage name | `AlphaImage=` | "" | Alpha shadow variant |
| +0x22C | bool | Theater **[corrected]** | `Theater=` | false | Previously mislabeled "AlternateArcticArt" |
| +0x22D | bool | Crushable | `Crushable=` | false | |
| +0x22E | bool | Bombable | `Bombable=` | false | |
| +0x22F | bool | RadarInvisible **[resolved]** | `RadarInvisible=` | **true** for overlays | Overlay ctor defaults to 1; previously "?" |
| +0x230 | bool | Selectable **[resolved]** | `Selectable=` | **false** for overlays | Overlay ctor defaults to 0 |
| +0x231 | bool | LegalTarget **[resolved]** | `LegalTarget=` | false | |
| +0x232 | bool | Insignificant **[resolved]** | `Insignificant=` | **true** for overlays | Overlay ctor defaults to 1; previously "?" |
| +0x233 | bool | Immune | `Immune=` | false | |
| +0x235 | bool | (unknown, default 0) | — | false | Still LOW confidence — see Open Questions |
| +0x236 | bool | Voxel | `Voxel=` | false | If false, ObjectTypeClass calls FUN_005F9070 (SHP load helper) |
| +0x237 | bool | NewTheater | `NewTheater=` | false | |
| +0x239 | bool | IgnoresFirestorm | `IgnoresFirestorm=` | false | |
| +0x23A | bool | UseLineTrail | `UseLineTrail=` | false | |
| +0x23B | byte[3] | LineTrailColor | `LineTrailColor=` | 0,0,0 | |
| +0x240 | int | LineTrailColorDecrement | `LineTrailColorDecrement=` | 0 | |

Note on YR overlay defaults: the overlay constructor overrides three inherited defaults:
`RadarInvisible=true`, `Selectable=false`, `Insignificant=true`. Real overlay INI sections
therefore commonly write `RadarInvisible=false` (e.g., `[TIB01]`, `[LOBRDG01]`) to expose
the overlay on the minimap.

### OverlayTypeClass-Specific Fields (verified)

| Offset | Type | Field | INI Key | Default | Evidence |
|--------|------|-------|---------|---------|----------|
| +0x294 | int | ArrayIndex | — | set in ctor | Self-referencing index in global array |
| +0x298 | int | Land | `Land=` | 0 (Clear) | FUN_004754b0 (LandType enum parser); forced to 5 if Tiberium==true AND was 0 |
| +0x29C | ptr | CellAnim (AnimTypeClass*) | `CellAnim=` | NULL | AnimTypeClass::FindByName |
| +0x2A0 | int | DamageLevels | `DamageLevels=` | 1 | Read from **art** section (+0x1F8) |
| +0x2A4 | int | Strength | `Strength=` | 1 | Read from rules section |
| +0x2A8 | bool | Wall | `Wall=` | false | |
| +0x2A9 | bool | Tiberium | `Tiberium=` | false | Forces Armor=6 and Land=5-if-0 |
| +0x2AA | bool | Crate | `Crate=` | false | |
| +0x2AB | bool | CrateTrigger | `CrateTrigger=` | false | |
| +0x2AC | bool | NoUseTileLandType | `NoUseTileLandType=` | **true** | Constructor default 1 |
| +0x2AD | bool | IsVeinholeMonster | `IsVeinholeMonster=` | false | TS-legacy |
| +0x2AE | bool | IsVeins | `IsVeins=` | false | TS-legacy |
| +0x2AF | bool | (unknown — never written in traced code) | — | 0 | Checked in ReadINI to gate SHP load but nothing ever sets it |
| +0x2B0 | bool | Explodes | `Explodes=` | false | |
| +0x2B1 | bool | ChainReaction | `ChainReaction=` | false | |
| +0x2B2 | bool | Overrides | `Overrides=` | false | Bridges use this |
| +0x2B3 | bool | DrawFlat | `DrawFlat=` | **true** | Constructor default 1 |
| +0x2B4 | bool | IsRubble | `IsRubble=` | false | |
| +0x2B5 | bool | IsARock | `IsARock=` | false | |
| +0x2B6 | byte[3] | RadarColor (R,G,B) | `RadarColor=` | 0,0,0 | FUN_00474b50 color parser |

### Tiberium=true Side Effects in ReadINI (verified)

```
if (*(char *)(type + 0x2a9) != 0):       // Tiberium=yes
    *(int  *)(type + 0x9c) = 6           // Armor := 6 (Special)
    if (*(int *)(type + 0x298) == 0):    // if Land == Clear
        *(int *)(type + 0x298) = 5       // Land := 5 (Tiberium)
```

### SHP Load Gate in ReadINI (verified)

```
if (type+0x22C == 0 AND type+0x2AF == 0):   // NOT Theater AND NOT-unknown-flag
    FUN_007c9ff0(buf, 0, 0, type+0x1F8, ".SHP")
    type+0xA4 = LoadFileFromMIX(...)
```

Theater-specific overlays (e.g., `[TIB01]` with `Theater=yes` in art.ini) are loaded via a
separate theater-aware path elsewhere. The +0x2AF gating is not used by any parser in YR
that has been traced — see Open Questions.

### OverlayTypeClass Methods

| Address | Method | Vtable Slot | Notes |
|---------|--------|-------------|-------|
| `0x005FE250` | Constructor | — | Inits defaults, registers in global array, assigns UniqueID |
| `0x005FE770` | ReadINI | [25] | Reads all fields from rules/art INI |
| `0x005FEC70` | FindOrCreate | — | Lookup by name; allocates 700 bytes if new |
| `0x005FEF00` | What_Am_I | [11] | Returns 0x15 (21 = RTTI_OVERLAYTYPE) |
| `0x005FEF10` | Size_Of | [12] | Returns 700 |
| `0x005FE530` | CreateInstance | [32] | Allocates 0xB0, calls OverlayClass ctor |
| `0x005FE570` | CreateInstanceAtDefault | [35] | Same with default coords (0x00AC1608) |
| `0x005FE4C0` | GetDimensions | [36] | Returns {0, 0x7FFF7FFF} (lazy-init) |
| `0x005FEDE0` | GetRadarColor | [39] | Returns +0x2B6 RGB color |

## 3a. TiberiumClass Layout (verified 2026-04-22)

Offsets verified from `TiberiumClass::Constructor` (0x007216C0) and `TiberiumClass::ReadINI`
(0x00721A50). Only the fields relevant to overlay/cell mechanics are listed.

| Offset | Type | Field | INI Key | Notes |
|--------|------|-------|---------|-------|
| +0x98 | int | ArrayIndex | — | Set in ctor from `g_TiberiumClass_Count` |
| +0x9C | int | Spread | `Spread=` | Frame interval for spread (default 2200) |
| +0xA0 | double | SpreadPercentage | `SpreadPercentage=` | |
| +0xA8 | int | Growth | `Growth=` | Frame interval for growth |
| +0xB0 | double | GrowthPercentage | `GrowthPercentage=` | 0 = no growth (e.g., Cruentus/gems) |
| +0xB8 | int | **Value** | `Value=` | Credits per density level |
| +0xBC | int | Power | `Power=` | |
| +0xC0 | int | Color | `Color=` | Enum |
| +0xC4 | DynVec | Debris | `Debris=` | AnimTypeClass* list for crystal explosion bits |
| +0xE0 | ptr | **ImagePtr** | `Image=` | Selected base OverlayTypeClass pointer; constructor zero, preserved when `Image=-1` |
| +0xE4 | int | **MaxDensity** | — | Written `12` by every image branch except `-1`; constructor zero and preserved on the no-write branch |
| +0xE8 | int | **NumImages** | — | Written `12` by every image branch except `-1`; constructor zero and preserved on the no-write branch |
| +0xEC | int | **NumExtraImages** | — | Written `8` by Riparius/default, Vinifera, and Aboreus; `Image=2` and `Image=-1` do not write it, so fresh Cruentus has 0 but rereads preserve a prior value |

### Image= → OverlayTypeClass base pointer switch (hardcoded in ReadINI)

`TiberiumClass::ReadINI` reads signed `Image=` with default `-1`, increments
the value, and dispatches through an unsigned six-entry jump table. Exact
results:

| Signed `Image=` | Result | Overlay base index | Counts written |
|----------|----------|--------------------|------|
| -1 | no writes; prior state survives | preserved; 0 on fresh construction | none |
| 2 | Cruentus/gems | **27** / GEM01 | MaxDensity 12, NumImages 12; extras preserved |
| 3 | Vinifera | **127** / TIB2_01 | 12/12/8 |
| 4 | Aboreus | **147** / TIB3_01 | 12/12/8 |
| every other signed integer, including 0 and 1 | default Riparius/ore | **102** / TIB01 | 12/12/8 |

Evidence: constructor zero writes `0x0072173A..0x0072174C`; selector read and
dispatch `0x00721C3F..0x00721C55`; jump table `0x00721CF8`; case/common writes
`0x00721C5C..0x00721CD6`.

These are the bases used by `OverlayToTiberiumIndex` (§5.11) and by `DrawOverlay_Body`'s
position-based variant selection (§5.1). The TIB2/TIB3 ranges match the GetRadarColor
byte-swap ranges `[0x7F-0x8A]` and `[0x93-0x9E]` exactly.

## 4. CellClass Overlay Fields (verified)

These fields in CellClass store the runtime overlay state after an OverlayClass stamps:

| Offset | Type | Field | Purpose |
|--------|------|-------|---------|
| +0x44 | int | OverlayTypeIndex | Index into g_OverlayTypeClass_Array. -1 = no overlay |
| +0x11C | byte | **SlopeIndex [corrected]** | 0 = flat cell; 1-N = slope tile orientation. Drives slope-variant SHP selection for tiberium and Z-offset lookup in g_OverlaySlopeZOffset (DAT_00AA105C) |
| +0x11E | byte | OverlayData | Multi-purpose: ore density (0-11), wall frame+damage, bridge damage state, or (for crates) crate-type index if < 0x13 |
| +0x122 | byte | OreNeighborCount | Count of adjacent cells with ore. Decremented on neighbor ore removal |
| +0x140 bit 0x80 | bit | HasBridgeOverlay | Adds +4 to effective height in DrawOverlay_Body; set by SetBridgeDirection |

### OverlayData Encoding (verified)

**Tiberium/Ore overlays** (`Tiberium=true`, flat cells):
- Value 0-11 = density level (0=sparse, 11=maximum)
- SHP frame = OverlayData
- At densities 0 and 9: variety offset 0-3 added from Latin square (see §6)
- SHP variant (rendered type) is position-deterministic: `(MapY * MapX) % NumImages + baseIndex`

**Tiberium on sloped cells** (`Tiberium=true`, +0x11C != 0):
- Rendered type = `(MapY * MapX) % quarterExtra + (slopeIdx - 1) * quarterExtra + baseIndex + NumImages`
- Where `quarterExtra = NumExtraImages / 4` (= 2 for ore)
- This is **slope-variant rendering**, not damage — previous report's "smudge/damaged tiberium" interpretation **[corrected]**

**Wall overlays** (`Wall=true`):
- Upper nibble (bits 4-7) = damage level (0 to DamageLevels-1)
- Lower nibble (bits 0-3) = connectivity bitmask (4-bit cardinal: N=1, E=2, S=4, W=8)
- Combined byte: `(damageLevel << 4) | connectivityFrame`
- Example: OverlayData=0x25 → damage level 2, connected E+N

**Bridge overlays:**
- Values 0-8: EW bridge damage states (0 = healthy)
- Values 9-17: NS bridge damage states (9 = healthy)
- Healthy frames (0 and 9) add Latin-square variety 0-3

**Crate overlays:**
- If OverlayData < 0x13 (19): use as direct crate-type index into the [Powerups] table
- Else: random weighted selection (see CRATE_SYSTEM_GHIDRA_REPORT.md §8)

**Other overlays** (rocks, tracks, rubble): frame index directly.

## 5. Core Logic

### 5.1 Tiberium Rendering — `CellClass::DrawOverlay_Body` (0x0047F6A0, re-verified)

```
idx = cell.OverlayTypeIndex
if idx == 0xA7 or idx == 0xB2: return           // two hardcoded skip indices

type = g_OverlayTypeClass_Array[idx]
heightLevel = cell.field_0x11B + ((cell.field_0x140 >> 7) & 1) * 4   // +4 if bridge

if (cell.field_0x140 & 0x80):                    // bridge overlay
    // (see 5.3 bridge path)

elif type.Tiberium:                              // tiberium / ore / gems
    tibIdx = OverlayToTiberiumIndex(cell)
    if tibIdx == -1: return
    tib = g_TiberiumClass_Array[tibIdx]

    if cell.SlopeIndex == 0:                     // FLAT
        variantIdx = (cell.MapY * cell.MapX) % tib.NumImages + tib.Image.ArrayIndex
    else:                                        // SLOPED
        quarterExtra = tib.NumExtraImages / 4
        variantIdx = (cell.MapY * cell.MapX) % quarterExtra
                   + (cell.SlopeIndex - 1) * quarterExtra
                   + tib.Image.ArrayIndex + tib.NumImages

    shp = g_OverlayTypeClass_Array[variantIdx].GetSHP()
    frame = cell.OverlayData
    CC_Draw_Shape(shp, frame, screenPos, viewport, 0x4E00, 0,
                  heightLevel * -15 - 2, zAdjust, remap, ...)

else:                                            // non-tiberium
    if type.Wall:
        // (see 5.4 wall path)
    elif type.Crate:
        // render crate frame
    elif type.IsRubble:
        // foundation-cell lookup render
    else:
        // standard overlay: slope Z-offset from g_OverlaySlopeZOffset if slope, frame = OverlayData
```

### 5.2 Tiberium Shadow — `CellClass::DrawOverlay_Shadow` (0x0047F510)

```
frame = OverlayData
shadowFrame = frame + (shp.TotalFrames / 2)      // shadows are second half of SHP
if bridge overlay AND OverlayData in (8, 0x12):  // NS bridge shadow
    screenX -= 15
    screenY += 7
CC_Draw_Shape(shp, shadowFrame, screenPos, viewport, 0x4601, 0,
              heightLevel * -15 - 2, ...)
```

### 5.3 Bridge Path in DrawOverlay_Body (verified)

```
if (cell.field_0x140 & 0x80):                    // HasBridgeOverlay
    // Cache-check: skip redraw if same frame counter AND same viewport AND same shroud byte
    if (cell.field_0x64 == g_CurrentFrameCounter
        AND cell.field_0x118 == DAT_00880940
        AND cell.field_0x68..0x74 == viewport[0..3]):
        return

    frame = cell.OverlayData
    if frame == 0 or frame == 9:                 // healthy EW or healthy NS
        frame += g_OverlayVarietyLatinSquare[((Y & 3) << 2) | (X & 3)]

    CC_Draw_Shape(shp, frame, screenPos, viewport, 0x4E00, 0,
                  heightLevel * -15 - 2, 0, cell.field_0x10E, 0, 0, 0, 0, 0)

    // Cache current frame counter + viewport rect into cell.field_0x64..0x74
```

### 5.4 Wall Path in DrawOverlay_Body **[corrected z-offset]**

```
drawFlags = DrawFlat ? 0 : 2                     // 2 = upright draw mode
zOffset   = (IsARock OR DrawFlat) ? 0 : -15      // upright walls shift up 15px
frame     = OverlayData                          // upper nibble=damage, lower=connectivity

CC_Draw_Shape(shp, frame, screenPos, viewport, 0x4E00, 0,
              zOffset + heightLevel * -15 - 2, drawFlags, frameYAdjust, remap, ...)
```

(The prior report's "`zOffset = IsARock ? 0 : -15`" was incomplete — DrawFlat also forces zOffset to 0. A wall with `DrawFlat=yes` draws flat on the ground; DrawFlat=no draws upright.)

### 5.5 Wall Destruction — `CellClass::DestroyOverlay` (0x00480CB0, re-verified)

```
function DestroyOverlay(cell, damage):
    if cell.OverlayTypeIndex == -1: return 0
    type = g_OverlayTypeClass_Array[cell.OverlayTypeIndex]
    if !type.Wall: return 0

    // Random damage check (only if damage is real, not forced -1)
    if damage != -1 AND damage < type.Strength AND !g_MapEditorMode:
        if RandomRanged(0, type.Strength) > damage:
            return 0

    TacticalClass::DirtyScreenRect(bounding_rect)        // redraw region
    cell.OverlayData += 0x10                              // bump damage level (upper nibble)

    // Chain reaction at penultimate damage level (requires DamageLevels > 2)
    if (cell.OverlayData >> 4) == type.DamageLevels - 1 AND type.DamageLevels > 2:
        for dir in [N, E, S, W]:   // step by 2 in uVar14, mask & 7
            neighbor = GetAdjacentCell(cell, dir via g_DirectionOffsets)
            if neighbor.OverlayTypeIndex != -1
               AND g_OverlayTypeClass_Array[neighbor.OverlayTypeIndex].Wall
               AND neighbor.OverlayTypeIndex == cell.OverlayTypeIndex
               AND neighbor.OverlayData < 0x10:           // still undamaged
                DestroyOverlay(neighbor, 200)              // 0xC8 = 200 chain damage

    // Destruction gating
    if damage != -1:
        damageLevel = cell.OverlayData >> 4
        if damageLevel < type.DamageLevels:
            return 0
        if damageLevel == type.DamageLevels - 1 AND (cell.OverlayData & 0xF) != 0:
            return 0                                      // still connected, can't finalize

    // Full destruction
    cell.field_0x50 = -1
    cell.OverlayTypeIndex = -1
    cell.OverlayData = 0
    CellClass::RecalcAttributes(cell)
    MapClass::AssignOrphanedCellZone(cell.MapCoord)
    FUN_00584550(cell.MapCoord)
    RadarClass::MarkTerrainDirty(cell.MapCoord)

    // Update wall-connection frames on 4 cardinal neighbors (FUN_00480630)
    for dir in [N, E, S, W]:
        neighbor = GetAdjacentCell(cell, dir)
        FUN_00480630(neighbor)                           // recompute neighbor's connectivity nibble

    FUN_007258d0()                                       // likely voxel/building notification

    // Decrement OreNeighborCount on ALL 8 neighbors
    for dir in ALL_8_DIRECTIONS:
        neighbor = GetAdjacentCell(cell, dir)
        neighbor.OreNeighborCount -= 1

    return 1
```

Key constants (verified): chain damage = **200** (0xC8), chain fires at penultimate level
(`DamageLevels - 1`), minimum `DamageLevels > 2` required for chain to engage.

### 5.6 Wall Connectivity — `BuildingClass::ConnectWalls` (0x00452A40)

```
g_WallConnectionBitmask_NESW[4] at 0x00818CA0 = {1, 2, 4, 8}    // N, E, S, W

for dir in [N, E, S, W]:
    neighbor = GetAdjacentCell(this, dir)
    building = LookupBuildingInCell(neighbor)
    if building AND building.IsWall AND building.OwnerType == this.OwnerType:
        this.LaserFenceFrame |= g_WallConnectionBitmask_NESW[dir]
        BuildingClass::AdjustWallConnections(neighbor, dir)    // 0x00453060
```

Produces the 4-bit connectivity frame (0-15) stored in the lower nibble of OverlayData.

### 5.6.1 Post-Destruction Wall Cleanup — `CellClass::PostDestructionWallCleanup` (0x00480630) **[resolved Open Q #4]**

**Hardcoded destruction rules verified against rulesmd.ini [OverlayTypes]:** array indexes
map to `0=GASAND`, `1=CYCL`, `2=GAWALL`, `3=BARB`, `22=FENC` — all five are wall/fence-type
overlays. GASAND (DamageLevels=2) auto-destroys at isolated data 0x10 or 0x20; GAWALL
(DamageLevels=3) at isolated 0x20 or 0x30; BARB (DamageLevels=1) at isolated 0x10; CYCL/FENC
similarly. All patterns represent "fully damaged AND fully isolated."


Called by `CellClass::DestroyOverlay` on each of 4 cardinal neighbors after a wall is destroyed. Despite taking `CellClass*` as param_1, the function iterates a driver array of **5 direction entries** at `DAT_0081CC70..DAT_0081CC84` (first entry `0xFFFFFFFF` = "this cell", next four = cardinal directions). For each visited cell:

```
1. Dirty the tactical rect and radar for the cell
2. If cell has a wall-type overlay (+0x2A8):
   a. Recompute the 4-bit connectivity mask by testing 8 directions with
      FUN_00480510(cellOverlayIdx, direction) — each direction that returns
      true contributes a bit (ORed into local_cc)
   b. Replace the lower nibble: field_0x11E = (field_0x11E & 0xF0) | new_mask
   c. Apply HARDCODED post-destruction destruction rules (for isolated,
      fully-damaged walls of specific types):
        OverlayTypeIndex==0 (GASAND)  and OverlayData in {0x10, 0x20}  → destroy
        OverlayTypeIndex==1 (CYCL)    and OverlayData == 0x20          → destroy
        OverlayTypeIndex==2 (GAWALL)  and OverlayData in {0x20, 0x30}  → destroy
        OverlayTypeIndex==3 (BARB)    and OverlayData == 0x10          → destroy
        OverlayTypeIndex==0x16 (22)   and OverlayData in {0x10, 0x20}  → destroy
      These combine max-damage (upper nibble == DamageLevels) with
      lower-nibble=0 (no neighbors) → wall with no remaining connections
      dies automatically
   d. CellClass::RecalcAttributes
   e. If ZoneType changed:
        if destroyed this pass → AssignOrphanedCellZone + decrement
           OreNeighborCount on 8 neighbors
        else → MergeAdjacentCellZone
```

### 5.6.2 Wall Direction-Reachable Test — `FUN_00480510`

Used by 5.6.1 to test if a wall-type is present/connecting in a given direction from a cell:

```
function FUN_00480510(cell, targetOverlayIdx, dir):
  if cell.OverlayTypeIndex == targetOverlayIdx AND != -1:
      return true                                    // same wall already placed

  if targetOverlayIdx == -1:
      // "any wall?" test (used during placement)
      return cell.OverlayTypeIndex in {2 /*GAWALL*/, 0x1A /*NAWALL*/, 0xF3}

  if targetOverlayIdx in {0 /*GASAND*/, 2 /*GAWALL*/}:
      // check in-progress BuildingClass in cell
      for obj in cell.FirstObject list:
          if obj.What_Am_I() == 6 /*BUILDING*/ AND obj.HP > 0:
              bti = obj.BuildingType.ArrayIndex
              if (bti == RulesClass+0x86C AND dir in {2, 6})  // AlliedWall E/W
                 OR (bti == RulesClass+0x870 AND dir in {0, 4}) // AlliedWall N/S
                 OR (bti == RulesClass+0x87C):                  // Any-wall
                  return true

  if targetOverlayIdx == 0x1A /*NAWALL*/:
      for obj in cell.FirstObject:
          if obj.What_Am_I() == 6 AND obj.HP > 0:
              bti = obj.BuildingType.ArrayIndex
              if (bti == RulesClass+0x874 AND dir in {2, 6})    // SovietWall E/W
                 OR (bti == RulesClass+0x878 AND dir in {0, 4}): // SovietWall N/S
                  return true

  return false
```

**RulesClass wall-building offsets fully resolved** (from `RulesClass::ReadGeneral` at 0x0066D530):

| RulesClass Offset | INI Key in `[General]` | Role |
|-------------------|------------------------|------|
| `+0x86C` | `GDIGateOne=` | Allied gate, N-S orientation |
| `+0x870` | `GDIGateTwo=` | Allied gate, E-W orientation |
| `+0x874` | `NodGateOne=` | Soviet gate, N-S orientation |
| `+0x878` | `NodGateTwo=` | Soviet gate, E-W orientation |
| `+0x87C` | `WallTower=` | Watchtower building (connects to any wall) |

So the function is really testing **"does the adjacent cell's building act as a wall?"** —
gates and wall towers count as wall-connectors. This is why wall segments visually link across
a gate or wall tower in the original game. Direction encoding per RA2 8-direction standard:
`{0, 4}` = N/S, `{2, 6}` = E/W.

### 5.7 Tiberium Placement Eligibility — `CellClass::CanPlaceTiberium` (0x004838E0)

Returns true if ore/gems can germinate on this cell. Used as the gate for `PlaceTiberium`'s
"new overlay" branch (see §5.8).

```
function CanPlaceTiberium(cell):
  if !MapClass::Is_Cell_In_Playfield(cell.MapCoord, 1): return false
  if cell.field_0x140 & 0x500 != 0: return false            // bridge (0x100) or rail (0x400)

  // Blocking objects: a BuildingClass (RTTI 6) with HP>0 blocks, UNLESS
  // its BuildingTypeClass has +0xC9A == 0 AND +0x1701 == 0 (unknown flags,
  // possibly "BaseNormal" / "InvisibleInGame")
  for obj in cell.FirstObject:
      if obj.What_Am_I() == 6 AND obj.HP > 0:
          if BuildingTypeClass+0xC9A != 0 OR BuildingTypeClass+0x1701 != 0:
              continue
          return false

  // A pre-existing overlay (RTTI 0x24) blocks if its +0x2B1 (ChainReaction) is true
  for obj in cell.FirstObject:
      if obj.What_Am_I() == 0x24:
          if obj.Type+0x2B1 != 0: return false
          break

  // Land-type gate: DAT_0089EA60 is a 36-byte-stride table indexed by LandType
  if DAT_0089EA60[LandType * 0x24] == 0: return false

  // Other gates
  if cell.OverlayTypeIndex != -1: return false              // already has overlay
  if cell.SlopeIndex != 0: return false                     // must be flat

  // IsoTile-accepts-tib gate
  iso = cell.IsoTileTypeIndex
  if iso < 0 OR iso >= g_IsoTileTypeArray_Count: return false
  if g_IsoTileTypeArray[iso]+0x306 == 0: return false

  return true
```

### 5.8 Ore Spread Germination — `FUN_004818E0` (xref'd from spread-queue tick)

Called from the ore spread-queue processor to seed a freshly-germinated cell with an
initial density that matches its surroundings. Returns the credit value deposited
(`(density + 1) * TiberiumValue`).

```
function SpreadCellGerminate(cell, randomizeType):
  if cell.OverlayTypeIndex == -1: return 0
  tibIdx = OverlayToTiberiumIndex(cell)
  if tibIdx == -1: return 0
  tib = g_TiberiumClass_Array[tibIdx]
  value = tib.Value  (+0xB8)

  if randomizeType:
      // Pick a random OverlayTypeClass pointer within this tib's primary-image range
      cell.OverlayTypeIndex = Random(ptr(tib.Image),
                                     ptr(tib.Image) + (tib.NumImages - 1) * 700)
      // NOTE: relies on contiguous allocation of OverlayTypeClass instances

  // Count neighbors of same tib type (8 directions)
  matching = 0
  for dir in 0..7:
      neighbor = GetAdjacentCell(cell, dir)
      if OverlayToTiberiumIndex(neighbor) == tibIdx:
          matching += 1

  // Look up initial density from the neighbor-count table
  cell.OverlayData = g_OreDensityByNeighborCount[(matching % MaxDensity) * 4]

  return (cell.OverlayData + 1) * value
```

The g_OreDensityByNeighborCount table (§6) is keyed by neighbor count modulo
MaxDensity (12 for ore). A fully-surrounded cell (8 same-type neighbors) seeds at
density 11; isolated cells seed at 0.

### 5.9 Ore Harvest — `CellClass::Reduce_Tiberium` (0x00480A80, verified)

```
function Reduce_Tiberium(cell, densityLevels):
    tibIdx = OverlayToTiberiumIndex(cell)
    if densityLevels <= 0 OR tibIdx == -1: return 0
    tib = g_TiberiumClass_Array[tibIdx]

    if cell.OverlayData == 11:                    // max density → trigger regrowth later
        TiberiumClass::AddToGrowthQueue(cell.MapCoord)

    currentDensity = cell.OverlayData
    if densityLevels < currentDensity + 1:        // partial harvest
        cell.OverlayData -= densityLevels
        TacticalClass::DirtyScreenRect(...)
        return densityLevels

    // Full removal
    cell.OverlayTypeIndex = -1
    cell.OverlayData = 0
    CellClass::RecalcAttributes(cell)
    RadarClass::MarkTerrainDirty(cell.MapCoord)
    TiberiumClass::ClearSpreadBitmaps_AllTypes()

    // Seed spread from 8 neighbors (those not already in tib's spread bitmap)
    for dir in ALL_8_DIRECTIONS:
        neighborCoord = cell.MapCoord + g_DirectionOffsets[dir]
        if in_bounds(neighborCoord):
            cellIdx = FUN_0042b1c0()              // map-cell index
            if *(char *)(cellIdx + *(int *)(tib + 0xF8)) == 0:   // not already in spread bitmap
                TiberiumClass::AddToSpreadQueue(neighborCoord)

    TacticalClass::DirtyScreenRect(...)
    return densityLevels                          // reports full removal amount
```

**Key finding**: `densityLevels` parameter is **levels** (1 per SHP frame), not raw credit value.
At density 11, a harvest triggers regrowth queue. On full removal, 8 neighbors seed the
spread queue — this is how harvested patches refill from surrounding ore.

### 5.10 Ore Placement — `CellClass::PlaceTiberium` (0x00487190, verified)

```
function PlaceTiberium(cell, tibTypeIdx, amount):
    tib = g_TiberiumClass_Array[tibTypeIdx]
    if amount >= tib.MaxDensity (+0xE4 = 12): return 0

    if CellClass::CanPlaceTiberium(cell, tib):
        // Branch A: empty cell — germinate
        if cell.SlopeIndex == 0:                  // flat
            variantRand = RandomRanged(0, 11)
            type = g_OverlayTypeClass_Array[tib.Image.ArrayIndex + variantRand]
        else:                                     // sloped
            variantRand = RandomRanged(0, 1)
            type = g_OverlayTypeClass_Array[tib.Image.ArrayIndex
                                          + tib.NumImages
                                          + cell.SlopeIndex * 2 + variantRand - 2]
        new OverlayClass(type, cell.MapCoord, -1)          // ephemeral, stamps into cell
        TiberiumClass::AddToGrowthQueue(cell.MapCoord)
        cell.OverlayData = amount
    else if cell already has this tibTypeIdx's overlay
         AND ScenarioClass+0x34A6 is set      // map [Basic] TiberiumGrowthEnabled= (see §13a)
         AND cell.SlopeIndex == 0
         AND cell.OverlayData < tib.MaxDensity - 1
         AND tib.Growth >= g_MinGrowthThreshold (0x007E3810):
        // Branch B: already-filled cell — densify further
        cell.OverlayData = min(cell.OverlayData + amount, tib.MaxDensity - 1)
        TiberiumClass::AddToSpreadQueue(cell.MapCoord)

    return 1
```

On new placement, variant is **randomly chosen** and stored in cell.OverlayTypeIndex. On draw,
a **deterministic position-based formula** picks the SHP variant (see §5.1). Because all
variants resolve to the same TiberiumClass via `OverlayToTiberiumIndex`, the economic/land-type
behavior is identical regardless of which variant was stored.

### 5.11 OverlayToTiberiumIndex (0x005FDD20, verified)

### 5.12 Map Overlay-Pack Parser — `ReadMapOverlayPacks` (0x005FD2E0) **[corrected address]**

Reads the `[OverlayPack]` and `[OverlayDataPack]` sections from a .map file and stamps
them into cells. Previously mislabeled `BSurface__Constructor` in Ghidra (renamed 2026-04-22).

**Pass 1 — `[OverlayPack]`** (512×512 bytes after base64 + LCW decompress):

```
for y in 0..512:
  for x in 0..512:
    idx = pack_byte(x, y)
    if idx == 0xFF: continue                              // empty
    type = g_OverlayTypeClass_Array[idx]

    // Validation gate:
    if type.GetSHP() == NULL AND type.CellAnim == NULL: skip    // no art, no anim
    if g_GameMode != 0 AND type.Crate: skip                     // no pre-placed crates in MP

    if !Cell_in_bounds_check(x, y): skip
    cell = MapClass::Get_CellClass(x, y)
    saved_data = cell.OverlayData                               // snapshot

    new OverlayClass(type, (x, y), -1)                          // stamps into cell

    // Bridge overlays preserve pre-existing OverlayData (bridge damage was set earlier)
    if idx in {0x18 BRIDGE1, 0x19 BRIDGE2, 0xED BRIDGEB1, 0xEE BRIDGEB2}:
        cell.OverlayData = saved_data
```

**Pass 2 — `[OverlayDataPack]`** (also 512×512 bytes after base64 + LCW):

```
for y in 0..512:
  for x in 0..512:
    data_byte = pack_byte(x, y)
    if Cell_in_bounds_check(x, y):
        cell = MapClass::Get_CellClass(x, y)
        cell.OverlayData = data_byte     // blind write (after Pass 1 placement)
```

Key findings:
- **Bridge-overlay damage state is preserved** across OverlayPack — indices `0x18/0x19/0xED/0xEE`
  have their data byte snapshotted/restored. This is because bridge damage was written elsewhere
  earlier (from the map's [Basic] or bridge-specific structural records).
- **Multiplayer skips pre-placed crates.** `g_GameMode != 0` filters out `Crate=yes` overlays,
  which in MP are placed instead by the CrateSlot timer system (see CRATE_SYSTEM report).
- **Pass 2 is unconditional** — it overrides any OverlayData set in Pass 1 (except bridge damage).
  Map editors (FinalSun, FinalAlert, WAE) write both packs so their values match semantically.

Maps an overlay type index to a TiberiumClass index. Despite the misleading legacy name
(often seen as "IsWallOverlay" in some reports), this function is **tiberium-only**:

```
function OverlayToTiberiumIndex(overlayTypeIdx):
    if overlayTypeIdx == -1: return -1
    type = g_OverlayTypeClass_Array[overlayTypeIdx]
    if !type.Tiberium: return -1                  // NOT tiberium overlay

    for tib in g_TiberiumClass_Array:
        baseIdx  = tib.Image.ArrayIndex           // +0xE0 → ObjectTypeClass* → +0x294
        primary  = tib.NumImages                  // +0xE8
        extras   = tib.NumExtraImages             // +0xEC
        if overlayTypeIdx in [baseIdx, baseIdx + primary):
            return tib.ArrayIndex                 // +0x98
        if overlayTypeIdx in [baseIdx + primary, baseIdx + primary + extras):
            return tib.ArrayIndex

    Log("Overlay: %s not really tiberium")
    return 0                                      // fallback — should never happen on live data
```

## 6. Static Data Tables (verified)

### Overlay Variety Latin Square — `g_OverlayVarietyLatinSquare` (`0x0081CC30`)

16 × int32, indexed by `((Y & 3) << 2) | (X & 3)`:

```
Row 0: [0, 1, 2, 3]
Row 1: [3, 2, 1, 0]
Row 2: [2, 3, 0, 1]
Row 3: [1, 0, 3, 2]
```

Applied to ore/bridge at densities 0 and 9 to add visual variety (0-3 frame offset).

### Ore Density Initial Seed — `g_OreDensityByNeighborCount` (`0x0081CD28`)

12 × int32, indexed by neighbor count, giving initial density:

```
Neighbors:  [ 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11]
Density:    [ 0, 1, 3, 4, 6, 7, 8,10,11, 7, 0, 1]
```

Referenced by `FUN_004818E0` and `FUN_004814F0` during ore spread tick.

### Wall Connection Bitmask — `g_WallConnectionBitmask_NESW` (`0x00818CA0`)

```
[0]=1 (North), [1]=2 (East), [2]=4 (South), [3]=8 (West)
```

### Overlay Slope Z-Offset Table — `DAT_00AA105C` **[resolved]**

Indexed by `cell.SlopeIndex` (+0x11C). Referenced from:
- `0x0047F91D` & `0x0047F9EE` in `CellClass::DrawOverlay_Body` (per-slope Y-offset)
- `0x00547404` in `FUN_00547370` (bridge rail / related render)
- `0x00549B13` in `FUN_00549AE0`
- Written from `0x00543E02` (runtime initialization — slope-to-Y offset table is populated
  as the TMP tile set is loaded)

Purpose: Y-shift applied when drawing overlays on sloped cells, so the sprite sits correctly
on the ramp surface. **This is NOT a smudge / damaged-overlay lookup table** (the prior
report's Open Question #3 was based on a misinterpretation of +0x11C as "smudge"; +0x11C is
actually SlopeIndex per CELLCLASS_STRUCT_GHIDRA_REPORT.md).

## 7. Global Variables

| Address | Type | Name | Notes |
|---------|------|------|-------|
| `0x00A8EC50` | ptr | OverlayClass DynVec vtable | DynamicVectorClass management |
| `0x00A8EC54` | ptr | `g_OverlayClass_Array` | Pointer to array of OverlayClass* |
| `0x00A8EC58` | int | `g_OverlayClass_Capacity` | |
| `0x00A8EC5D` | byte | OverlayClass_Array_Initialized | |
| `0x00A8EC60` | int | `g_OverlayClass_Count` | Active overlay instances |
| `0x00A83D80` | ptr | OverlayTypeClass DynVec vtable | |
| `0x00A83D84` | ptr | `g_OverlayTypeClass_Array` | Pointer to array of OverlayTypeClass* |
| `0x00A83D88` | int | `g_OverlayTypeClass_Capacity` | |
| `0x00A83D8D` | byte | OverlayTypeClass_Array_Initialized | |
| `0x00A83D90` | int | `g_OverlayTypeClass_Count` | 253+ in YR |
| `0x00A83D94` | int | OverlayTypeClass_Array_Growth | |
| `0x00B0F4EC` | ptr | g_TiberiumClass_Array | Array of TiberiumClass* |
| `0x00B0F4F8` | int | g_TiberiumClass_Count | 2-4 in YR (Riparius, Cruentus, optionally Vinifera/Aboreus) |
| `0x00AC1608` | packed | Default cell coords | Used by CreateInstanceAtDefault |
| `0x0081CC30` | int[16] | `g_OverlayVarietyLatinSquare` | Labeled 2026-04-22 |
| `0x0081CD28` | int[12] | `g_OreDensityByNeighborCount` | Labeled 2026-04-22 |
| `0x00818CA0` | int[4] | `g_WallConnectionBitmask_NESW` | Labeled 2026-04-22 |
| `0x00AA105C` | int[] | OverlaySlopeZOffset table | Runtime-populated; exact length TBD |

## 8. INI Keys

### Per OverlayType (rules section)

| Key | Offset | Type | Default | Notes |
|-----|--------|------|---------|-------|
| `Land=` | +0x298 | int (enum) | 0=Clear | Forced to 5 if Tiberium=true & was 0 |
| `Strength=` | +0x2A4 | int | 1 | HP for destructible overlays |
| `Wall=` | +0x2A8 | bool | false | Destructible wall |
| `Tiberium=` | +0x2A9 | bool | false | Harvestable ore/gem (94 sections in rulesmd.ini) |
| `Crate=` | +0x2AA | bool | false | 2 sections: CRATE, WCRATE |
| `CrateTrigger=` | +0x2AB | bool | false | CRATE, WCRATE |
| `NoUseTileLandType=` | +0x2AC | bool | **true** | Don't inherit underlying tile's land |
| `IsVeinholeMonster=` | +0x2AD | bool | false | **TS-legacy** |
| `IsVeins=` | +0x2AE | bool | false | **TS-legacy** |
| `Explodes=` | +0x2B0 | bool | false | 0 occurrences in YR overlays |
| `ChainReaction=` | +0x2B1 | bool | false | 0 occurrences in YR overlays |
| `Overrides=` | +0x2B2 | bool | false | Bridges: RAILBRDG1/2, BRIDGE1/2, BRIDGEB1/2 |
| `DrawFlat=` | +0x2B3 | bool | **true** | Affects both alpha flag AND z-offset |
| `IsRubble=` | +0x2B4 | bool | false | RUBBLE_OVERLAY only |
| `IsARock=` | +0x2B5 | bool | false | SROCK/TROCK set this in practice (no YR section writes `IsARock=yes`; defaulted in constructor for rock overlays by IDE convention) |
| `RadarColor=` | +0x2B6 | RGB(3B) | 0,0,0 | Minimap color |
| `CellAnim=` | +0x29C | ptr | NULL | AnimTypeClass* |

### Per OverlayType (inherited from ObjectTypeClass — rules section)

| Key | Offset | Type | Default (overlay) | Notes |
|-----|--------|------|-------------------|-------|
| `Image=` | +0x1F8 | char[25] | section name | Art section name |
| `AlphaImage=` | +0x213 | char[25] | "" | |
| `AlternateArcticArt=` | +0x211 | bool | false | **[corrected]** |
| `Theater=` | +0x22C | bool | false | **[corrected]** |
| `NewTheater=` | +0x237 | bool | false | |
| `Crushable=` | +0x22D | bool | false | |
| `Bombable=` | +0x22E | bool | false | |
| `RadarInvisible=` | +0x22F | bool | **true** | **[resolved]** overlay-specific default |
| `Selectable=` | +0x230 | bool | **false** | **[resolved]** |
| `LegalTarget=` | +0x231 | bool | false | **[resolved]** — on TIB01/GEM01 explicitly set false |
| `Insignificant=` | +0x232 | bool | **true** | **[resolved]** overlay-specific default |
| `Immune=` | +0x233 | bool | false | |
| `Voxel=` | +0x236 | bool | false | When false, ObjectTypeClass calls SHP helper FUN_005F9070 |
| `Armor=` | +0x9C | int enum | — | Forced to 6=Special if Tiberium=true |

### Per OverlayType (art section, at +0x1F8)

| Key | Offset | Type | Default | Notes |
|-----|--------|------|---------|-------|
| `DamageLevels=` | +0x2A0 | int | 1 | Wall destruction stages |

### Globals

| Key | Source | Notes |
|-----|--------|-------|
| `TiberiumGrows=` | rules `[General]`, map `[SpecialFlags]` | Default yes in YR |
| `TiberiumSpreads=` | rules `[General]`, map `[SpecialFlags]` | Default yes |
| `GrowthRate=` | rules `[General]` | 5 (minutes) in YR stock |
| `TiberiumGrowthEnabled=` | map `[Basic]` | Per-map override |
| `TiberiumStrength=` | rules `[CombatDamage]` | -1 in YR (no chain ignition) |
| `TiberiumExplosive=` | rules `[CombatDamage]` | no in YR |
| `TiberiumExplosionDamage=` | rules `[CombatDamage]` | 0 in YR |
| `BridgeStrength=` | rules `[CombatDamage]` | 1500 |

## 9. Overlay Categories in YR

### Active in YR skirmish

| Category | Index Range | Count | Flags | Notes |
|----------|-------------|-------|-------|-------|
| Ore (Riparius) | ~102-121 (TIB01-20) | 20 | `Tiberium=yes` | 12 primary + 8 slope variants |
| Gems (Cruentus) | ~27-38 (GEM01-12) | 12 | `Tiberium=yes` | 12 primary, no slope extras (`NumExtraImages=0`) |
| Allied Wall | GAWALL (id 3) | 1 | `Wall=yes` | Strength=300, DamageLevels=3 |
| Soviet Wall | NAWALL (id 27) | 1 | `Wall=yes` | Strength=300, DamageLevels=3 |
| Sandbags | GASAND (id 1) | 1 | `Wall=yes` | Strength=100, DamageLevels=2 |
| Fences (brick/wood/cyber) | various CAFN*, GAFWLL, YAWALL | ~45 | `Wall=yes` | Various strengths |
| Low wood bridges | LOBRDG01-28 / LOBRDB1-28 | ~56 | `Land=Road` | 28 pieces × 2 damage variants |
| High concrete bridges | BRIDGE1-2, BRIDGEB1-2 | 4 | `Overrides=yes` | |
| Railroad bridges | RAILBRDG1-2 | 2 | `Overrides=yes` | |
| Railroad tracks | TRACKS01-16, TRACKTUNNEL01-04 | 20 | `Land=Railroad` | |
| Crates | CRATE, WCRATE | 2 | `Crate=yes`, `CrateTrigger=yes` | Land=Clear / Water |
| Rocks | SROCK01-05, TROCK01-05 | 10 | `IsARock=true` | Decorative |
| Rubble | RUBBLE_OVERLAY | 1 | `IsRubble=yes` | Uses SROCK01 image |

### TS Legacy (dormant in YR)

| Category | Notes |
|----------|-------|
| Veins (VEINS, id 129 region) | `IsVeins=true` — TS tiberium veins, inactive in YR |
| Veinhole (VEINHOLE, id 170 region) | `IsVeinholeMonster=true` — TS only |
| Tib3/Tib4 slots (unused in YR) | Available but not referenced by [Tiberiums] in standard YR |

**Tiberium count in YR:** stock rulesmd.ini defines 4 [Tiberiums] entries (Riparius,
Cruentus, Vinifera, Aboreus) but only Riparius + Cruentus appear on standard maps.

## 10. Integration Points

### Who Creates Overlays

| Caller | Address | When |
|--------|---------|------|
| **`ReadMapOverlayPacks`** **[corrected]** | `0x005FD2E0` | Loading [OverlayPack]+[OverlayDataPack] from .map. (Prior report's 0x00568E40 was incorrect — that address is a bridge helper.) |
| `AnimClass::AI` | `0x0042413B` | Tiberium deposition from terrain anims (TIBTRE) |
| `CellClass::PlaceTiberium` | `0x00487190` | TiberiumClass spread/growth tick |
| `CellClass::GrowTiberium` | `0x00483710` | Density increment (calls PlaceTiberium with amount=1) |
| Map/scenario crate placement | multiple | CRATE/WCRATE placement |
| `BuildingClass::Place_OccupyMap` | tail call to `FUN_0056BEC0` | CrateBeneath building destruction |

### Who Reads Cell Overlay State

| Consumer | Key Fields | Purpose |
|----------|------------|---------|
| `CellClass::RecalcAttributes` | +0x44, +0x2A9 | Sets LandType from overlay |
| `CellClass::DrawOverlay_Body` | +0x44, +0x11C, +0x11E, +0x140 | Renders overlay SHP |
| `CellClass::DrawOverlay_Shadow` | +0x44, +0x11E | Renders overlay shadow |
| `CellClass::Reduce_Tiberium` | +0x44, +0x11E | Harvesting / warhead damage |
| `CellClass::Get_Tiberium_Value` | +0x44, +0x11E | Credit-value calculation |
| `CellClass::DestroyOverlay` | +0x44, +0x11E | Wall destruction + chain |
| `CellClass::GetEffectiveHeight` | +0x140 bit 7 | Bridge height offset |
| `UnitClass::Can_Enter_Cell` | +0xEC (LandType) | Speed / passability |
| Pathfinding | +0xEC, +0x4C | Route cost evaluation |
| `CellClass::OverlayToTiberiumIndex` | +0x44, +0x2A9 | Overlay → TiberiumClass mapping |
| Can_Enter_Cell_General crate dispatch | +0x11E | Direct crate-type index if < 0x13 |

### Tick Order

Overlays don't have their own tick in `World::advance_tick`. They are affected by:
- **Ore growth/spread:** `Map::Logic` (0x004D2370) processes spread+growth queues
- **Combat:** `Apply_area_damage` calls `Reduce_Tiberium`; `DestroyOverlay` handles wall chain
- **Wall damage:** BuildingClass hit processing → `DestroyOverlay`
- **Crate pickup:** `CellClass::Can_Enter_Cell_General` (0x00481A00) dispatches based on OverlayData

## 11. Current Rust Implementation Status (updated 2026-08-14)

### Implemented

| Feature | Path |
|---------|------|
| OverlayPack / OverlayDataPack parsing | [src/map/overlay.rs](src/map/overlay.rs) |
| Overlay type registry (core flags) | [src/map/overlay_types.rs](src/map/overlay_types.rs) |
| GPU atlas for overlay SHPs | [src/render/overlay_atlas.rs](src/render/overlay_atlas.rs) |
| Bridge-specific atlas | [src/render/bridge_atlas.rs](src/render/bridge_atlas.rs) |
| Overlay instance rendering + bridge Latin-square variety | [src/app_instances/overlays.rs](src/app_instances/overlays.rs) |
| Ore growth + spread | [src/sim/ore_growth.rs](src/sim/ore_growth.rs) |
| Reduce_Tiberium (harvest) | [src/sim/miner/mod.rs](src/sim/miner/mod.rs) |
| Overlay grid (runtime mutable state) | [src/sim/overlay_grid.rs](src/sim/overlay_grid.rs) |
| Wall connectivity bitmask (N=1 E=2 S=4 W=8) | [src/map/overlay.rs](src/map/overlay.rs) |
| Wall damage levels (upper nibble) + chain reaction | [src/sim/overlay_grid.rs](src/sim/overlay_grid.rs) |
| Bridge frame 0-8 / 9-17 encoding | [src/app_instances/overlays.rs](src/app_instances/overlays.rs) |
| Bridge height offsets (EW -16, NS -31, low 0) | [src/app_instances/overlays.rs](src/app_instances/overlays.rs) |
| Flat Tiberium body identity | Uses the live stored overlay only to select `TiberiumClass`, then applies signed `(MapY * MapX) % NumImages + base`; the density byte remains the SHP frame |
| Flat Tiberium atlas coverage | Preloads every configured primary image across every parsed density frame, independent of map-seeded variants |

### Missing / Needs Work

| Gap | Note |
|-----|------|
| Sloped-tiberium extra-image variant selection | Slope tiles don't yet pick from the 8 extra variants |
| `Tiberium=true` forcing Armor=Special, Land=Tiberium | Engine-level flags not propagated |
| Flag parsing: `IsRubble`, `IsARock`, `Explodes`, `ChainReaction`, `Overrides`, `DrawFlat`, `NoUseTileLandType` | Not parsed into registry |
| Flag parsing: `RadarInvisible`, `LegalTarget`, `Selectable`, `Insignificant`, `Theater`, `AlternateArcticArt` | Inherited ObjectTypeClass fields not in registry |
| Persistent `OreNeighborCount` field | Currently on-demand counted, not cached in cell state |
| Wall DrawFlat z-offset toggle | Rust may not gate -15 z-shift by `DrawFlat=false` |
| Crate pickup + CrateTrigger | Not implemented |
| Crate regen timer (uniform random `[CrateRegen*450, CrateRegen*1800]`) | Not implemented |
| Overlay registry mod-input ordering | **CORRECTED 2026-07-23:** compacting missing numeric keys is native. `RulesClass` counts `[OverlayTypes]` entries, fetches each key by section-entry ordinal, reads that key's string value, and calls `OverlayTypeClass__FindOrCreate`; the numeric key value is never used as the array index. This matches the independently verified mappings key 1 `GASAND` → array index 0 and key 23 `FENC` → array index 22, so stock gaps (40/41/183) do not reserve slots. Rust's additional numeric sort/dedup policy may still differ for deliberately out-of-order or duplicate mod entries and remains UNCHECKED. Evidence: live Ghidra `decompile_function(0x00668BF0)` on 2026-07-23 (`Section_Entry_Count` → ordinal `Get_Entry_Key_Name` → `CCINIClass__ReadString` → `OverlayTypeClass__FindOrCreate`). |

## 12. Open Questions

1. **OverlayTypeClass +0x2AF flag** — Read in ReadINI to gate the generic SHP load, but nothing
   in any YR code path traced so far writes it. It appears to be either dead code inherited from
   TS or a runtime "already-loaded" cache flag that nothing currently flips. **Impact for parity:
   likely zero.** **Confidence: LOW.**

2. **OverlayTypeClass +0x235 flag** — Default 0 in constructor. Not read in any traced function.
   Similar to +0x2AF, may be dead code. **Confidence: LOW.**

   ~~**Propagation from TiberiumGrows INI to the runtime gate at `DAT_00A8B230+0x34A6`**~~
   **[RESOLVED]** — The flag at `DAT_00A8B230+0x34A6` is not `TiberiumGrows` at all. It is
   **`TiberiumGrowthEnabled=` from the map's `[Basic]` section**, loaded directly into
   `ScenarioClass+0x34A6` by `FUN_00689E90` (the scenario `[Basic]` parser). There is no
   propagation step — the value is read from the map file into ScenarioClass at scenario
   load. See §13a for the full disambiguation and a summary table of the three
   unrelated "TiberiumGrows"-shaped flags.

3. **DAT_00AA105C length** — Indexed by SlopeIndex (+0x11C) which is 0-20 per
   CELLCLASS_STRUCT_GHIDRA_REPORT.md, suggesting 21 int32 entries. Runtime-populated from
   `0x00543E02` (TMP tile load path). Exact table length and entry semantics (pixel offset? lepton
   offset?) need confirmation via dynamic trace. **Confidence: MEDIUM** (use confirmed, data
   layout inferred).

4. ~~FUN_00480630~~ **[RESOLVED]** — Now documented as §5.6.1 (Post-Destruction Wall Cleanup).
   It walks 5 cells (self + 4 cardinals), recomputes each wall's connectivity nibble from 8
   directions via `FUN_00480510`, and applies hardcoded destruction rules for isolated walls
   of specific overlay-type indices {0 GASAND, 1 CYCL, 2 GAWALL, 3 BARB, 0x16} at specific
   damage patterns.

5. **Crate overlay OverlayData>=0x13 dispatch** — Full weighted random selection table referenced
   in `CellClass::Can_Enter_Cell_General` (0x00481A00). Details in CRATE_SYSTEM_GHIDRA_REPORT.md
   §8; not duplicated here.

6. **BuildingTypeClass +0xC9A / +0x1701** — Flags checked by `CanPlaceTiberium` (§5.7) to decide
   whether a building in the cell blocks tib placement. Likely "BaseNormal" / "UndeployInto"
   or similar but not confirmed. **Confidence: LOW.**

   Update 2026-04-22: `RulesClass+0x86C…+0x87C` **[resolved]** — they are `GDIGateOne=`,
   `GDIGateTwo=`, `NodGateOne=`, `NodGateTwo=`, `WallTower=` from `[General]`.
   See §5.6.2. So `IsWallConnectableInDirection` is literally testing "is this adjacent
   building a gate or watchtower that visually connects walls?"

7. ~~RTTI 0x24 vs 0x14 discrepancy~~ **[RESOLVED]** — `TerrainClass::What_Am_I` (0x0071D300)
   returns 0x24. The RTTI-0x24 check in `FUN_0047C550` and `CellClass::CanPlaceTiberium` is
   searching for `TerrainClass` instances (trees, rocks, terrain objects placed via the
   `[Terrain]` map section). There is no actual discrepancy — OverlayClass returns 0x14,
   TerrainClass returns 0x24, and cell-placement gates block when a `TerrainClass` occupies
   the cell. OverlayClass does NOT live in the cell's object list at all (stamp-and-forget).

8. ~~Post-destruction overlay-index 0x16~~ **[RESOLVED]** — Array index 22 (= 0x16) in YR
   is **FENC** (the `FENC` fence overlay, [OverlayTypes] key 23). Confirmed by dumping the
   rulesmd.ini [OverlayTypes] list. So all five hardcoded destruction rules target
   consistent wall/fence-type overlays: GASAND (0), CYCL (1), GAWALL (2), BARB (3), FENC (22).
   (`FENC` has no section body in stock YR rulesmd.ini but is still allocated an array slot
   because its name appears in [OverlayTypes]; the destruction rule exists but is effectively
   dormant unless a mod activates FENC.)

## 13a. Clarifying note — `ScenarioClass+0x34A6` is `TiberiumGrowthEnabled` (map-level)

**[Resolved 2026-04-22]** — The gate that `CellClass::GrowTiberium` checks is neither
`TiberiumGrows` from `[MultiplayerDialogSettings]` nor the `SpecialFlags` bitfield. It
is **`TiberiumGrowthEnabled=` from the map's `[Basic]` section**, stored in
`ScenarioClass+0x34A6`.

### Verified identification

The function `FUN_00689E90` (the scenario `[Basic]`-section loader, called during map
init) contains:

```c
uVar2 = CCINIClass__ReadBool([Basic], "TiberiumGrowthEnabled", default);
*(undefined1 *)(param_1 + 0x34a6) = uVar2;
```

where `param_1 == DAT_00A8B230 == &ScenarioClass_instance`. Confirmed by the surrounding
fields parsed by the same function:

| Map [Basic] key | ScenarioClass offset |
|-----------------|----------------------|
| `NextScenario=` | +0x004 |
| `AltNextScenario=` | +0x108 |
| `FreeRadar=` | +0x34A4 |
| `TrainCrate=` | +0x34A5 |
| **`TiberiumGrowthEnabled=`** | **+0x34A6** |
| `VeinGrowthEnabled=` | +0x34A7 |
| `IceGrowthEnabled=` | +0x34A8 |
| `SkipScore=` | +0x34AE |
| `OneTimeOnly=` | +0x34AF |
| `SkipMapSelect=` | +0x34B0 |
| `TruckCrate=` | +0x34B1 |
| `FillSilos=` | +0x34B2 |
| `TiberiumDeathToVisceroid=` | +0x34B3 |
| `IgnoreGlobalAITriggers=` | +0x34B4 |
| `MultiplayerOnly=` | +0x34BC |

### The three unrelated "TiberiumGrows"-shaped flags (now disambiguated)

| Flag | INI location | Destination | Semantic |
|------|-------------|-------------|----------|
| `TiberiumGrows=` | rules `[MultiplayerDialogSettings]` | `RulesClass+0x14B0` | MP dialog default |
| `TiberiumGrows` | rules `[SpecialFlags]` | `SpecialFlags` bit 6 (uint32 bitfield, TS-legacy) | Special-flags-bits bitmap |
| **`TiberiumGrowthEnabled=`** | **map `[Basic]`** | **`ScenarioClass+0x34A6`** | **Per-map gate used by `GrowTiberium`** |

### Implications for a fidelity Rust implementation

- The runtime growth gate is **per-map**, not per-rules. Parsing `TiberiumGrows=` from
  rules [MultiplayerDialogSettings] into a game-level switch alone is **insufficient
  for parity** — the engine actually reads the per-map `[Basic] TiberiumGrowthEnabled=`
  flag on every `GrowTiberium` call.
- The doc's §8 table already lists `TiberiumGrowthEnabled=` as a "map [Basic]
  per-map override"; this note confirms it is the **primary** gate the engine checks,
  not a mere override.

### What this renames

- `g_SpecialFlags+0x34A6` in the pseudocode → `ScenarioClass+0x34A6 (TiberiumGrowthEnabled)`
- The "§12 Open Question" about propagation → **RESOLVED**; there is no propagation to
  trace. The flag is read directly from the map file into ScenarioClass at scenario load.

## 13. Ghidra Annotations Applied (2026-04-22)

Labels created:

| Address | Label |
|---------|-------|
| 0x0081CC30 | `g_OverlayVarietyLatinSquare` |
| 0x00818CA0 | `g_WallConnectionBitmask_NESW` |
| 0x0081CD28 | `g_OreDensityByNeighborCount` |
| 0x00A8EC54 | `g_OverlayClass_Array` |
| 0x00A8EC58 | `g_OverlayClass_Capacity` |
| 0x00A8EC60 | `g_OverlayClass_Count` |
| 0x00A83D84 | `g_OverlayTypeClass_Array` |
| 0x00A83D88 | `g_OverlayTypeClass_Capacity` |
| 0x00A83D90 | `g_OverlayTypeClass_Count` |

Functions renamed or with plate comments added:

| Address | Function | Action |
|---------|----------|--------|
| 0x00487190 | `CellClass__PlaceTiberium` | Renamed from `FUN_00487190`; plate comment on branch logic |
| 0x005FE770 | `OverlayTypeClass__ReadINI` | Plate comment documenting full field map incl. inherited offsets |
| 0x005FC380 | `OverlayClass__Constructor` | Plate comment documenting stamp-and-forget flow |
| 0x00480CB0 | `CellClass__DestroyOverlay` | Plate comment documenting chain reaction (0xC8 damage, `DamageLevels > 2` gate) |
| 0x00480A80 | `CellClass__Reduce_Tiberium` | Plate comment documenting density-level parameter + spread-queue seeding |
| 0x00480630 | `CellClass__PostDestructionWallCleanup` | Round 2 rename; plate comment documenting 5-cell walk and hardcoded destruction rules |
| 0x00480510 | `CellClass__IsWallConnectableInDirection` | Round 2 rename; helper for 5.6.1 |
| 0x004818E0 | `CellClass__SpreadCellGerminate` | Round 2 rename; density = g_OreDensityByNeighborCount[neighbors] |
| 0x005FD2E0 | `ReadMapOverlayPacks` | Round 4 rename (was `BSurface__Constructor` — Ghidra mis-labeled the OverlayPack parser) |

## Sources

### Ghidra Addresses Decompiled (2026-04-22 re-verification + round-2)

Round 1:
- `0x005FC380` — OverlayClass::Constructor
- `0x005FD270` — OverlayClass::Unlimbo (thin wrapper around ObjectClass::Reveal)
- `0x005FDD20` — CellClass::OverlayToTiberiumIndex
- `0x005FED00` — OverlayClass::GetRadarColor (TIB2/TIB3 byte-swap, not bridge)
- `0x005FE250` — OverlayTypeClass::Constructor
- `0x005FE770` — OverlayTypeClass::ReadINI (field-by-field)
- `0x005F92D0` — ObjectTypeClass::ReadINI (inherited offsets — CRITICAL for correction)
- `0x00480A80` — CellClass::Reduce_Tiberium
- `0x00480CB0` — CellClass::DestroyOverlay
- `0x00487190` — CellClass::PlaceTiberium
- `0x0047F6A0` — CellClass::DrawOverlay_Body
- `0x0047C550` — Find blocking object in cell (RTTI 0x24 lookup)
- `0x0047E8A0` — CellClass::AddContent

Round 2:
- `0x00480630` — CellClass::PostDestructionWallCleanup (resolves prior Open Q #4)
- `0x00480510` — CellClass::IsWallConnectableInDirection (helper for round-2)
- `0x004818E0` — CellClass::SpreadCellGerminate (density from g_OreDensityByNeighborCount)
- `0x004814F0` — Ore-spread random-variant generator (deterministic 8×8 permutation at DAT_0089E620)
- `0x004838E0` — CellClass::CanPlaceTiberium
- `0x005FDF70` — OverlayClass::Destructor (shifts array on removal, resets base class)
- `0x005FD8F0` — OverlayClass::Load (re-sets vtables, swizzles type pointer)

Round 3 (verification):
- `0x005FDF50` — OverlayClass::What_Am_I → returns **0x14** confirmed
- `0x0071D300` — TerrainClass::What_Am_I → returns **0x24** (resolves RTTI-0x24 mystery)
- `0x007216C0` — TiberiumClass::Constructor (field offsets verified)
- `0x00721A50` — TiberiumClass::ReadINI (Image→baseIdx switch verified, TIB2/TIB3 ranges confirmed)

Round 4 (final verification):
- `0x005FD2E0` — `ReadMapOverlayPacks` (the real OverlayPack/OverlayDataPack parser, was
  mislabeled `BSurface__Constructor` — prior report's 0x00568E40 was wrong)
- `0x00483710` — CellClass::GrowTiberium (confirms SlopeIndex gate; growth = PlaceTiberium(1))
- `0x00722F00` — TiberiumClass::GrowthProcessor (growth queue = min-heap by frame,
  ftol-clamped [5, 50] cells/tick, reschedule jitter = Random() % 50 frames)
- `0x0066D530` — RulesClass::ReadGeneral (resolves wall-pointer INI keys at +0x86C..+0x87C)

### Cross-Referenced Documents

- `ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md` — TiberiumClass struct, ore mechanics
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` — CellClass field layout (confirms +0x11C = SlopeIndex)
- `BRIDGE_RENDERING_GHIDRA_REPORT.md` — Bridge overlay draw pipeline
- `BRIDGE_SYSTEM.md` — Bridge flags and height system
- `SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md` — Harvest mechanics
- `ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md` — Ore value formula
- `OBJECTCLASS_GHIDRA_REPORT.md` — ObjectClass field map
- `ABSTRACTCLASS_GHIDRA_REPORT.md` — AbstractClass field map
- `CRATE_SYSTEM_GHIDRA_REPORT.md` — Crate pickup dispatch, regen timer formula

### INI Files Checked

- `ini/rulesmd.ini` — [OverlayTypes] (253 entries), per-type sections, [General], [CombatDamage], [CrateRules], [Tiberiums]
- `ini/artmd.ini` — per-type art sections, DamageLevels, Theater
