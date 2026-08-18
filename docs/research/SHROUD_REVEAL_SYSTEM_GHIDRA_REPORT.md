# Shroud Reveal System — Ghidra Research Report

Reverse-engineered via Ghidra MCP (live decompilation of Yuri's Revenge `gamemd.exe`).

---

## 1. Reveal Spiral Table Generation

### Address & Init Function
- **Spiral table (dx,dy pairs):** `DAT_00abd490` (in .bss, populated at runtime)
- **Mirror/midpoint table:** `DAT_00abcf60` (in .bss, populated at runtime)
- **Init function for spiral table:** `FUN_00561910` (8102 bytes, at 0x00561910)
- **Init function for mirror table:** `FUN_005638d0` (3438 bytes, at 0x005638d0)
- **Called from:** `0x00561900` (a trampoline JMP) which is referenced from `0x00813a88`

### Table Format

Both tables store entries as pairs of `short` values (dx, dy) packed into 32-bit words:
```
Each 4-byte entry = (short dx, short dy) where low 16 bits = dx, high 16 bits = dy
```

### Spiral Pattern

The table at `DAT_00abd490` contains cells sorted by distance from center, forming
concentric "rings" around the origin. The pattern starts at the center (0,0) and spirals
outward. Each ring contains cells at increasing Euclidean distance.

**First entries (decoded from the init function):**
```
Index 0:  (0, 0)      -- center
Index 1:  (1, -1)      -- ring 1 starts
Index 2:  (0, -1)
Index 3:  (-1, -1)
Index 4:  (-1, 0)
Index 5:  (1, 0)
Index 6:  (1, 1)
Index 7:  (-1, 1)
Index 8:  (0, 1)       -- ring 1 ends (8 cells)
Index 9:  (-1, -2)      -- ring 2 starts
Index 10: (-2, -1)
...
```

The entries are NOT a true spiral; they're an **approximate distance-sorted** list.
Within each "ring" (same distance band), cells are enumerated in a specific order.

### Cumulative Ring Size Table

**Address:** `DAT_007ed3d0` (in .rdata, constant)

This table stores the cumulative number of cells for each sight range 0..10:

| Sight Range | Cumulative Cells | Ring Size |
|-------------|-----------------|-----------|
| 0           | 1               | 1         |
| 1           | 9               | 8         |
| 2           | 21              | 12        |
| 3           | 37              | 16        |
| 4           | 61              | 24        |
| 5           | 89              | 28        |
| 6           | 121             | 32        |
| 7           | 161             | 40        |
| 8           | 205             | 44        |
| 9           | 253             | 48        |
| 10          | 369*            | 116*      |

(*Ring 10 extends to the end of the table including entries computed via `FUN_0042d470`
which converts isometric cell offsets to lepton coordinates.)

### Maximum Sight Range

The code clamps sight range to 10:
```c
if ((10 < param_3) || (10 < param_3)) {
    param_3 = 10;
}
```

### Inner Ring Skip (RevealByHeight=false optimization)

When `RevealByHeight` is false (the common case) AND `param_5 != 0` AND `sight > 2`,
the code skips the inner rings and only reveals the outer ring:

```c
if ((rules+0x17EE == 0) && (param_5 != 0) && (2 < param_3)) {
    iVar6 = cumulative_table[param_3];  // DAT_007ed3c4 + param_3*4
    iVar5 = iVar5 - iVar6;             // reduce count
    local_44 = &DAT_00abd490 + iVar6 * 2;  // skip spiral entries
    param_2 = &DAT_00abcf60 + iVar6 * 4;   // skip mirror entries
}
```

This is a performance optimization: if height-based LOS is disabled, inner cells are
assumed already revealed from previous ticks, so only the outermost ring needs updating.

**Confidence: 95%** — directly verified from decompilation.

---

## 2. Height-Based Sight Obstruction (RevealByHeight)

### Rules INI Key
- **INI key:** `RevealByHeight` in `[General]` section
- **Rules offset:** `rules+0x17EE` (confirmed: `0x0083cb80` string xref at `0x0066eaf0`)
- **Type:** bool (char)

### How Height Blocks Line of Sight

In `MapClass__RevealShroud` (0x005673a0) and `FUN_005678e0` (0x005678e0), when
`RevealByHeight` is true, each cell in the spiral is checked against a "mirror cell"
to determine if terrain height blocks the view:

```c
if ((param_8 != 0) && (rules+0x17EE != 0)) {
    // Compute mirror cell coordinates
    sVar11 = psVar9[2] + (local_24 - local_14);   // mirror dx + cell offset
    sVar3  = psVar9[3] + (sStack_22 - local_12);  // mirror dy + cell offset

    // Look up the mirror cell
    mirror_cell = CellArray[sVar3 * 0x200 + sVar11];

    // Compare viewer height level against mirror cell's Level
    if (iVar4 + 3 < (int)(char)mirror_cell[0x11B]) {
        goto LAB_005678a8;  // SKIP this cell — LOS blocked!
    }
}
```

### The "Mirror Cell"

The mirror cell is looked up from the **second table** at `DAT_00abcf60`. This table
has the same number of entries as the spiral table. For each (dx, dy) entry in the
spiral, the mirror table stores a corresponding (mdx, mdy) offset that represents the
**cell between the viewer and the target cell** — approximately the midpoint of the
line of sight.

The mirror table offsets are relative to the viewer's position but are adjusted by
`(local_24 - local_14, sStack_22 - local_12)` where local_14/local_12 are offsets
computed from the Z-adjusted viewer position.

**How the mirror table differs from the spiral table:**
- Spiral table entry 0: (0,0) -> Mirror entry 0: (0,0) — center, no midpoint
- Spiral table entry 1-8 (ring 1): Mirror entries are (0,0) or (1,0)/(0,1)/(-1,0)/(0,-1)
  — neighboring cells along the line
- For outer rings, mirror entries point to cells ~halfway between viewer and target

**Confidence: 85%** — The mirror table contents are verified from `FUN_005638d0`. The
interpretation as "midpoint for LOS check" is inferred from the usage pattern. The
exact geometric meaning of each mirror entry was not independently verified cell-by-cell.

### Height Comparison

- `iVar4` = viewer's Z coordinate / `DAT_00abde88` — this converts the viewer's Z
  position (in leptons) to a terrain level number
- `DAT_00abde88` = leptons per height level, computed at init time by `FUN_005617e0`
  from a trig lookup (converts between lepton Z and cell Level units)
- `mirror_cell[0x11B]` = **CellClass::Level** at offset 283 (byte) — the mirror cell's
  terrain height level (0-15 typically)
- The check: `if (viewer_level + 3 < mirror_level)` — if the midpoint cell's terrain
  level is more than 3 levels above the viewer's level, line of sight is blocked

The `+3` provides some tolerance — a cliff needs to be significantly higher than the
viewer to actually block sight. This means units on the ground (level 0) can see past
level-3 terrain but are blocked by level-4+ terrain at the midpoint.

**Confidence: 90%** — Field offsets and comparison logic directly verified.

---

## 3. The vtable+0x48C Call (TechnoClass Reveal Shroud)

### Identity

- **vtable+0x488:** `TechnoClass::UpdateReveal` at `0x0070af50`
  (labeled `TechnoClass__UpdateVeterancyAnim` in Ghidra — MISNAMED)
- **vtable+0x48C:** `TechnoClass::ReReveal` at `0x0070b1d0`
  (labeled `TechnoClass__ClearVeterancyAnim` in Ghidra — MISNAMED)

Both are inherited by all TechnoClass subclasses (BuildingClass, UnitClass, InfantryClass,
AircraftClass) — none override these vtable slots.

### What vtable+0x488 Does (UpdateReveal)

This is the main "update shroud reveal" function. It:

1. Checks if the object is allowed to reveal (`this+0x3D5` != 0)
2. Checks if the owner house type has `this+0x1A6` set (spectator flag — skip if so)
3. Gets the object's type class via `vtable+0x84` (GetType)
4. Computes the effective sight range from the type's Sight value
5. Applies veteran sight bonus if applicable (`rules+0x680` != 0.0)
6. Stores the reveal state: coordinates at `this[0x95..0x97]`, sight range at `this[0x98]`
7. Calls `FUN_005678e0` (the fog-aware reveal function) with the object's XYZ coordinates
   and computed sight range

### What vtable+0x48C Does (ReReveal)

This is the "clear and re-reveal" function, called when an object needs to refresh its
reveal (e.g., after moving). It:

1. Checks the same preconditions as UpdateReveal
2. Gets the stored sight range from `this[0x260]`
3. Calls `FUN_005678e0` with the stored coordinates and sight range

### Usage in FUN_004adee0 / FUN_004adcd0

These are the "paranoid" reveal/unreveal functions that iterate ALL TechnoClass objects:

```c
for (i = 0; i < g_TechnoClass_Count; i++) {
    techno = g_TechnoClass_Array[i];
    if (techno != NULL && techno+0x81 == 0) {
        type = techno->vtable->WhatAmI();  // vtable+0x2C
        if (type != 6 || param_1 == 0) {   // type 6 = Building
            if (IsPlayerControlled()) {
                techno->vtable[0x48C](...);  // ReReveal
            } else if (allied && RevealAlliedShroud) {
                techno->vtable[0x48C](...);  // ReReveal for allies
            }
        }
    }
}
```

**FUN_004adee0** (reveal): Calls vtable+0x48C for player-controlled and allied units.
**FUN_004adcd0** (unreveal): Calls vtable+0x488 and also calls `MapClass__UpdateFogBorder`
afterward with `sight + 3` as the radius.

**Confidence: 95%** — vtable offsets verified by reading BuildingClass and UnitClass
vtables at those slots. Both resolve to the same TechnoClass implementations.

---

## 4. CellClass Field +0x10C

### Identity

**CellClass offset 0x10C** (268 decimal) is a **short** (2 bytes).

### Usage in Rendering

In `FUN_00480180` (0x00480180), this field is passed as the 11th parameter to
`TMP_TileBlitter` (0x00547cf0):

```c
TMP_TileBlitter(
    *DAT_0087f69c,      // TMP image data
    0,                   // sub-tile index
    g_PrimarySurface,    // destination surface
    *param_2, param_2[1], // screen x, y
    *param_3, param_3[1], param_3[2], param_3[3], // clip rect
    (int)*(char *)(cell + 0x11B),   // param 10: Level (terrain height)
    (int)*(short *)(cell + 0x10C),  // param 11: THIS FIELD
    1, 0, 0, 1, 0, 0    // flags
);
```

### Likely Purpose

Based on context:
- It's a `short` value (16 bits)
- It's passed alongside `Level` (cell+0x11B) to the tile blitter
- It's in the CellClass struct between RadSite (248) and Height (282)
- It's copied during the fog-of-war save/restore in `FUN_00565c10`

This is most likely the **shroud/fog overlay tile frame index** — the index into the
shroud TMP tileset that determines which shroud edge graphic to draw for this cell.
It would be computed from the shroud edge bitmask (cell+0x120, cell+0x121) via a
lookup table.

The CellClass struct layout around this area (partially mapped):
```
Offset 0x108 (264): short — (unknown, related to shroud)
Offset 0x10A (266): short — (unknown, related to shroud)
Offset 0x10C (268): short — shroud tile frame index (passed to TMP_TileBlitter)
Offset 0x10E (270): short — (unknown, related to fog)
Offset 0x110 (272): short — (unknown)
...
Offset 0x11A (282): byte  — Height (sub-tile index from TMP)
Offset 0x11B (283): byte  — Level (terrain height level, 0-15)
Offset 0x11C (284): byte  — SlopeIndex
```

**Confidence: 75%** — The field's use as a tile blitter parameter is verified. The
interpretation as "shroud tile frame index" is inferred from context but not confirmed
by tracing the exact write path.

---

## 5. How Buildings Reveal Shroud

### Same Mechanism as All Technos

Buildings use the **exact same** reveal mechanism as all other TechnoClass objects.
The vtable entries at +0x488 and +0x48C are NOT overridden by BuildingClass — they
inherit directly from TechnoClass.

### Reveal Uses Center Coordinates

The reveal function uses the building's primary XYZ coordinates (TechnoClass offsets
`[0x27]`, `[0x28]`, `[0x29]` = `this+0x9C`, `this+0xA0`, `this+0xA4` in byte offsets),
which represent the building's **center/anchor point**. The foundation size does NOT
affect the reveal calculation directly.

### Single-Point Reveal, Not Per-Foundation-Cell

Buildings reveal from a **single point** (their center) with their Sight range, NOT
from each foundation cell individually. A 3x3 building with Sight=8 reveals the same
pattern as a 1x1 building with Sight=8 at the same location.

### Building Placement Reveal (FUN_0043f180)

When a building is placed (in `FUN_0043f180`, the BuildingClass enter-cell handler),
there is special code that calls `FUN_005678e0` with sight=1 for individual foundation
cells — but this is specifically for the case where a building is REPLACING an existing
structure (clearing terrain), not the normal reveal. The normal reveal happens through
the vtable+0x488/0x48C mechanism.

### Gap Generator Special Case

Buildings with `GapGenerator=yes` have separate handling through
`BuildingClass__UpdateGapGenerator_Tick` (0x00454db0) which uses `GapRadiusInCells`
(string at 0x00843e84) for the re-shroud radius. This is a separate system from the
normal reveal.

**Confidence: 90%** — vtable inheritance verified. Single-point reveal confirmed from
decompilation of TechnoClass::UpdateReveal. The placement reveal at sight=1 was observed
in FUN_0043f180 but its exact trigger conditions need further verification.

---

## Key Addresses Summary

| Address      | Name/Purpose |
|-------------|-------------|
| `0x005673a0` | `MapClass__RevealShroud` — main reveal function |
| `0x005678e0` | Fog-aware variant of RevealShroud (also updates fog) |
| `0x00561910` | Spiral table init (populates DAT_00abd490) |
| `0x005638d0` | Mirror table init (populates DAT_00abcf60) |
| `0x00abd490` | Reveal spiral table (short dx,dy pairs, max ~370 entries) |
| `0x00abcf60` | Mirror/midpoint table for height LOS checks |
| `0x007ed3d0` | Cumulative ring size table (int[11]) |
| `0x00abde88` | Leptons-per-height-level divisor (computed at init) |
| `0x0070af50` | `TechnoClass::UpdateReveal` (vtable+0x488) |
| `0x0070b1d0` | `TechnoClass::ReReveal` (vtable+0x48C) |
| `0x004adee0` | Paranoid reveal (iterate all technos, call vtable+0x48C) |
| `0x004adcd0` | Paranoid unreveal (iterate all technos, call vtable+0x488) |
| `0x00653830` | Wrapper for `MapClass__RevealFogCell` |
| `0x006d8700` | `Shroud_EdgeBitmask_Calculator` |

### CellClass Shroud-Related Fields

| Offset | Type  | Name/Purpose |
|--------|-------|-------------|
| 0x10C  | short | Shroud tile frame index (passed to TMP_TileBlitter) |
| 0x11A  | byte  | Height (sub-tile index from TMP data) |
| 0x11B  | byte  | Level (terrain height level, controls rendering Y offset) |
| 0x11C  | byte  | SlopeIndex |
| 0x120  | byte  | Shroud edge bitmask (8 neighbors, shroud layer) |
| 0x121  | byte  | Fog edge bitmask (8 neighbors, fog layer) |
| 0x12C  | int   | Shroud/fog flags |
| 0x130  | int   | Shroud reveal counter |
| 0x138  | byte  | Needs-redraw flag |
| 0x140  | int   | Extended cell flags (bit 1=fog revealed, bit 6=temp, etc.) |

### RulesClass Shroud-Related Fields

| Offset | Type | INI Key |
|--------|------|---------|
| 0x17E7 | bool | `RevealAlliedShroud` (inferred from usage context) |
| 0x17EE | bool | `RevealByHeight` (confirmed from string xref) |
