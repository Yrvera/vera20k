# Psychic Reveal Superweapon — Ghidra Research Report

**Primary Addresses:**
- `MapClass::RevealAroundCell` — `0x005678E0`
- `FUN_00653830` (RevealFogCell wrapper) — `0x00653830`
- `MapClass::RevealFogCell` — called from wrapper
- `MapClass::UpdateFogOfWarCell` — called for fog mode
- `SuperClass::Launch` case 11 — `0x006CC390`

**Confidence:** HIGH (all functions decompiled from binary)
**Active in YR:** Yes — launched via Psychic Sensor (NAPSIS building)

---

## 1. Overview

Psychic Reveal (Type=11, `PsychicReveal`) is a superweapon that reveals shroud in a
configurable radius around a target cell. It delegates to a general-purpose
`MapClass::RevealAroundCell` function that handles the circular reveal logic, ownership
checks, and cell iteration via the CellSpread table.

This is one of the simpler superweapons — no persistent state machine, no damage, no
ongoing effects. It reveals shroud once and plays a sound.

---

## 2. Launch Dispatch (Case 11)

From `SuperClass::Launch` at `0x006CC390`:

```
case 0xb:  // PsychicReveal
    if NOT IsReady: return

    cell = MapClass::Get_CellClass(target)
    coords = cell->GetCenterCoords()

    // Reveal shroud (called twice with identical params)
    MapClass::RevealAroundCell(coords, PsychicRevealRadius, owner, 0, 0, 0)
    MapClass::RevealAroundCell(coords, PsychicRevealRadius, owner, 0, 0, 0)

    // Play activation sound
    VocClass::PlayAtCoord(coords)
```

**Double call:** The function is called twice with identical parameters. Both calls use
param_7=0 (shroud reveal mode, not fog update). The purpose is unclear — possibly a
safety measure for complete reveal, or to handle both normal and bridge cell layers.

---

## 3. MapClass::RevealAroundCell (0x005678E0)

General-purpose shroud/fog reveal function. Used by Psychic Reveal and potentially
other systems (unit vision, spy satellite, etc.).

### Parameters

| Param | Type | Purpose |
|-------|------|---------|
| `this` (ECX) | MapClass* | Map instance |
| `param_2` | int* (CoordStruct) | Center coords (X, Y, Z in leptons) |
| `param_3` | int | Radius in cells (**clamped to max 10**) |
| `param_4` | HouseClass* | Owner (whose shroud to reveal) |
| `param_5` | char | Fog-edge behavior flag |
| `param_6` | undefined4 | Unused |
| `param_7` | char | 0 = reveal shroud, nonzero = update fog cells |
| `param_8` | char | Height-based reveal flag |
| `param_9` | undefined4 | Additional context |

### Algorithm

```
// 1. Height conversion
heightLevel = coords.Z / CellHeight (DAT_00abde88)

// 2. Perspective adjustment
adjustedX = coords.X + AdjustForZ(heightLevel) * 256
adjustedY = coords.Y + AdjustForZ(heightLevel) * 256

// 3. Cell conversion
centerCellX = adjustedX >> 8  // leptons to cells
centerCellY = adjustedY >> 8

// 4. Radius clamping
if radius > 10: radius = 10

// 5. Cell count from CellSpread table
cellCount = CellSpread[radius]  // table at 0x007ED3D0

// 6. Optional fog-edge reduction
if param_5 AND Rules+0x17EE (RevealByHeight) AND radius > 2:
    // Skip inner cells already revealed, start from edge
    startIdx = CellSpread[radius - offset]
    cellCount -= startIdx

// 7. Ownership check (CRITICAL — controls who sees the reveal)
if owner != NULL AND PlayerPtr != NULL:
    if owner == PlayerPtr: proceed
    else if owner has spied on PlayerPtr's radar: treat as PlayerPtr
    else if allied with PlayerPtr AND Rules+0x17E7 (ShareReveal): treat as PlayerPtr
if owner != PlayerPtr: return  // only reveal for the local player

// 8. Cell iteration (spiral pattern)
for each cell offset in CellOffsetPairs (0x00ABD490):
    cellX = centerCellX + offset.X
    cellY = centerCellY + offset.Y

    // Bounds check (diamond-shaped map bounds)
    if cellX + cellY >= mapDim1 (this+0xF4): skip
    if cellX - cellY >= mapDim1: skip
    if cellY - cellX >= mapDim1: skip
    if cellX + cellY > mapDim1 + mapDim2*2 (this+0xF8): skip

    // X-axis range check
    if |cellX - centerCellX| > radius: skip

    // Euclidean distance check
    dist = Sqrt_Approx((cellX - centerX)^2 + (cellY - centerY)^2)
    if dist > radius: skip

    // Height-based LOS check (optional)
    if param_8 AND Rules+0x17EE:
        check mirror cell height against heightLevel + 3
        if blocked: skip

    // Get cell object
    cellIdx = cellY * 0x200 + cellX
    cell = CellArray[cellIdx]  // g_CellArray_Base

    // Reveal or update fog
    if param_7 == 0:
        FUN_00653830(cell, owner, param_9)  // → MapClass::RevealFogCell
    else:
        if (cell.flags & 1) AND (cell.flags & 2) AND (cell[300] & 8):
            MapClass::UpdateFogOfWarCell(cell, owner)
```

### FUN_00653830 (RevealFogCell Wrapper — 0x00653830)

Trivial 10-line wrapper:
```c
bool FUN_00653830(cell, owner, context) {
    return MapClass::RevealFogCell(cell, owner, context) != 0;
}
```

---

## 4. Rules Offsets

| Offset | INI Key | Type | Default | Purpose |
|--------|---------|------|---------|---------|
| 0x0FEC | `PsychicRevealRadius` | int | 15 | Reveal radius in cells |
| 0x17E7 | (ShareReveal) | bool | — | Allies share shroud reveal |
| 0x17EE | (RevealByHeight) | bool | — | Height-based line of sight for reveal |

From `rulesmd.ini`:
```ini
PsychicRevealRadius=15    ; radius in cells that the PsychicReveal Super should clear
PsychicRevealActivateSound=PsychicRevealActivate
```

---

## 5. INI Configuration

### [PsychicRevealSpecial] Section (rulesmd.ini line 31032)

```ini
UIName=Name:PsyReveal
Name=Psychic Reveal
IsPowered=false               ; Does NOT require power
RechargeTime=4                ; 4 minutes
Type=PsychicReveal
Action=PsychicReveal
SidebarImage=PSYRICON
ShowTimer=no
DisableableFromShell=no        ; Cannot be disabled in lobby
FlashSidebarTabFrames=120      ; Flash sidebar for 120 frames when ready
```

**Building:** NAPSIS (Yuri Psychic Sensor) — YR only

### [AudioVisual] Section

```ini
PsychicRevealActivateSound=PsychicRevealActivate
```

---

## 6. Key Data Tables

| Address | Type | Purpose |
|---------|------|---------|
| 0x007ED3D0 | int[11] | CellSpread table — cumulative cell counts per radius |
| 0x00ABD490 | CellStruct[] | Cell offset pairs for spiral iteration |
| g_CellArray_Base | ptr | Base of cell array (cellY * 0x200 + cellX indexing) |

**CellSpread table values (cumulative cells within radius):**
Radius 0=1, 1=5, 2=13, 3=21, 4=29, 5=37, ... up to radius 10.

---

## 7. Integration Points

- **Caller:** `SuperClass::Launch` case 11
- **Shared function:** `MapClass::RevealAroundCell` is likely used by other reveal
  systems (unit vision, spy satellite, shroud regrowth) — not exclusive to Psychic Reveal
- **No persistent state:** Unlike Lightning Storm or Psychic Dominator, this has no
  ongoing state machine. It reveals once and is done.
- **Ownership matters:** Only reveals for the local player or their allies/spies.
  The reveal function will silently no-op if the owner is not the local player.

---

## 8. Edge Cases

- **Radius clamped to 10:** Even though `PsychicRevealRadius=15` in INI, the
  `RevealAroundCell` function clamps `param_3` to max 10. This means the effective
  reveal radius is 10 cells, not 15. **This may be an engine limitation or the
  CellSpread table may extend beyond 10** — needs further investigation.

- **Height-based reveal:** When Rules+0x17EE is true and param_8 is set, tall
  terrain can block reveal. For Psychic Reveal specifically, param_8=0 so this
  check is bypassed — Psychic Reveal ignores terrain height.

- **Fog vs shroud:** param_7=0 for Psychic Reveal means it reveals shroud (black
  unexplored areas), not fog. Fog-of-war updates use a different code path
  (param_7 != 0).

---

## 9. Open Questions

1. **Double call purpose** — Why is RevealAroundCell called twice with identical params?

2. **Radius 15 vs clamp 10** — PsychicRevealRadius=15 in INI but the function clamps
   to 10. Does the CellSpread table actually support radius > 10? If not, the INI value
   is effectively capped. Need to verify CellSpread table size.

3. **PsychicDetectionRadius=15** — Found on PSYCHINT (Psychic Sensor building, line 13353
   in rulesmd.ini). This may be a separate reveal mechanism (passive detection) vs the
   superweapon active reveal. Relationship needs investigation.

---

## Sources

**Ghidra functions decompiled:**
- 0x005678E0 (MapClass::RevealAroundCell — 180 lines)
- 0x00653830 (RevealFogCell wrapper — 10 lines)
- 0x006CC390 case 11 (Launch dispatch — ~15 lines)

**INI files checked:** ini/rulesmd.ini

**Date:** 2026-04-02
