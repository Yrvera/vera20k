# Infantry Sub-Cell Positioning — Ghidra Research Report

> **Verified 2026-03-19** against live Ghidra MCP decompilation of gamemd.exe.
> All function addresses, table values, occupancy logic, and bit fields confirmed
> by decompiling every referenced function and reading raw memory. Preference
> tables decoded from static .rdata. Isometric formula cross-checked against
> TacticalClass::CellToPixel (FUN_006d1fe0). Occupancy bit semantics traced to
> their setter functions across decompiled C files 087 and 141.

## Overview

RA2 places up to 3 infantry per cell at distinct sub-cell positions within the
isometric diamond. The original game uses 5 sub-cell indices (0–4), a quadrant
detection function, preference-order tables, and a lepton offset lookup table to
determine exact positions. Only indices 2, 3, 4 are ever used for placement —
index 0 is "center/unassigned" and index 1 exists in the table but is dead code.

## Sub-Cell Coordinate System

### Quadrant Function — FUN_004810a0

Determines which sub-cell quadrant a lepton position falls in.

```c
// Decompiled from gamemd.exe 0x004810a0
char __fastcall GetSubCell(uint *coords)
{
    uint x = coords[0];
    uint y = coords[1];
    double dy = (double)(int)((y & 0xFF) - 0x80);
    double dx = (double)(int)((x & 0xFF) - 0x80);
    int dist = (int)sqrt(dy * dy + dx * dx);

    if (dist < 0x3C)          // Within 60 leptons of center
        return 0;             // → sub-cell 0 (center)

    byte bits = (0x80 < (x & 0xFF));   // bit0 = X > 128
    if (0x80 < (y & 0xFF))
        bits |= 2;                     // bit1 = Y > 128

    if (bits == 0)
        return 0;             // NW quadrant (X<=128, Y<=128) → ALSO returns 0!

    return bits + 1;          // NE=2, SW=3, SE=4
}
```

**Critical detail:** When both X<=128 and Y<=128 (NW quadrant, bits=0), the
function returns 0 — the same as center. Sub-cell index 1 is **never returned**
by this function. The `bits + 1` formula only applies when at least one of X or Y
exceeds 128.

Return value mapping:

| Condition | bits | Return | Meaning |
|-----------|------|--------|---------|
| distance < 60 | — | 0 | Center |
| X<=128, Y<=128 | 0 | 0 | NW quadrant → merged with center |
| X>128, Y<=128 | 1 | 2 | NE quadrant |
| X<=128, Y>128 | 2 | 3 | SW quadrant |
| X>128, Y>128 | 3 | 4 | SE quadrant |

### Sub-Cell Index Layout

| Index | Lepton (X, Y) | Screen Offset (dx, dy) | Notes |
|-------|---------------|------------------------|-------|
| 0 | (128, 128) | (0, 0) | Center — returned for center AND NW quadrant |
| 1 | (64, 64) | (0, -7.5) | **Dead entry** — exists in table, never assigned |
| 2 | (192, 64) | (+15, 0) | NE — shifts right along iso axis |
| 3 | (64, 192) | (-15, 0) | SW — shifts left along iso axis |
| 4 | (192, 192) | (0, +7.5) | SE — shifts down along iso axis |

The three usable sub-cells (2, 3, 4) form a **diamond/triangle pattern**, not a
rectangular grid. Sub-cells 2 and 3 are horizontally opposed; sub-cell 4 is below.

### Isometric Screen Conversion

Sub-cell leptons → screen pixels. Confirmed from TacticalClass::CellToPixel at
`0x006d1fe0` which implements:

```c
pixelX = (cellX * 60/2 + cellY * -60/2) >> 8;   // = (X - Y) * 30 / 256
pixelY = (cellX * 30/2 + cellY * 30/2) >> 8;     // = (X + Y) * 15 / 256
```

For sub-cell offsets from center (128, 128):

```
dx_lep = sub_x - 128   (offset from cell center in leptons)
dy_lep = sub_y - 128
screen_dx = (dx_lep - dy_lep) * 30 / 256
screen_dy = (dx_lep + dy_lep) * 15 / 256
```

Worked examples:

| Index | dx_lep | dy_lep | screen_dx | screen_dy |
|-------|--------|--------|-----------|-----------|
| 0 | 0 | 0 | 0 | 0 |
| 1 | -64 | -64 | 0 | -7.5 |
| 2 | +64 | -64 | +15 | 0 |
| 3 | -64 | +64 | -15 | 0 |
| 4 | +64 | +64 | 0 | +7.5 |

## Sub-Cell Offset Table — DAT_0089e9f0

Address `0x0089e9f0` in .data segment (BSS — all zeros in the binary image).
Initialized at runtime by inline code at `0x0048e480`:

```asm
; Decoded from raw bytes at 0x0048e480
MOV EAX, 0x80           ; 128
XOR EDX, EDX            ; 0
MOV ECX, EAX            ; 128
MOV [0x0089e9f0], EAX   ; table[0].x = 128
MOV EAX, 0x40           ; 64
MOV [0x0089e9f4], ECX   ; table[0].y = 128
MOV ECX, EAX            ; 64
MOV [0x0089e9fc], EAX   ; table[1].x = 64
MOV EAX, 0xC0           ; 192
MOV [0x0089ea00], ECX   ; table[1].y = 64
MOV [0x0089ea08], EAX   ; table[2].x = 192
MOV [0x0089ea0c], ECX   ; table[2].y = 64
MOV EAX, ECX            ; EAX = 64
MOV ECX, 0xC0           ; ECX = 192
MOV [0x0089ea14], EAX   ; table[3].x = 64
MOV EAX, ECX            ; EAX = 192
MOV [0x0089e9f8], EDX   ; table[0].z = 0
MOV [0x0089ea04], EDX   ; table[1].z = 0
MOV [0x0089ea10], EDX   ; table[2].z = 0
MOV [0x0089ea18], ECX   ; table[3].y = 192
MOV [0x0089ea1c], EDX   ; table[3].z = 0
MOV [0x0089ea20], EAX   ; table[4].x = 192
MOV [0x0089ea24], ECX   ; table[4].y = 192
MOV [0x0089ea28], EDX   ; table[4].z = 0
RET
```

Resulting table (5 entries × 3 ints = 60 bytes):

| Index | X | Y | Z |
|-------|-----|-----|---|
| 0 | 128 | 128 | 0 |
| 1 | 64 | 64 | 0 |
| 2 | 192 | 64 | 0 |
| 3 | 64 | 192 | 0 |
| 4 | 192 | 192 | 0 |

Usage pattern (from FUN_00481180 and FUN_0051fb00):
```c
world_x = (lepton_x / 256) * 256 + table[subcell * 3 + 0];
world_y = (lepton_y / 256) * 256 + table[subcell * 3 + 1];
world_z = table[subcell * 3 + 2] + base_z;
```

The division uses a signed rounding idiom: `(x + (x >> 31 & 0xFF)) >> 8` which
truncates to cell base. The table value adds the sub-cell offset (0–255 range).

Only 3 callers reference this table:
- `FUN_00481180` — PlaceInfantryInCell (reads during placement)
- `FUN_0051fb00` — InfantryClass::Load (reads during save file deserialization)
- `0x0048e480` — runtime initialization (writes)

## Sub-Cell Preference Tables

When an infantry needs a sub-cell, the engine doesn't just use the quadrant
result directly — it consults **preference-order tables** to find the first
unoccupied sub-cell.

### Preference table — DAT_0081cc84

5 entries × 4 bytes (static .rdata), indexed by the quadrant result from
GetSubCell. Each entry lists 4 sub-cell indices to try in order. Raw memory
confirmed: `01 02 03 04  00 02 03 04  00 01 04 03  00 01 04 02  00 02 03 01`.

| Quadrant result | Try order | Effective (skip 0,1) |
|-----------------|-----------|----------------------|
| 0 (center/NW) | [1, 2, 3, 4] | 2, 3, 4 |
| 1 (unused) | [0, 2, 3, 4] | 2, 3, 4 |
| 2 (NE) | [0, 1, 4, 3] | 4, 3 |
| 3 (SW) | [0, 1, 4, 2] | 4, 2 |
| 4 (SE) | [0, 2, 3, 1] | 2, 3 |

Note: for quadrants 2–4, the preferred sub-cell is tried FIRST via a fast-path
check before entering the preference loop (see placement algorithm below). The
preference table is the fallback when the preferred sub-cell is already occupied.

### Random rotation table — DAT_0081cc98

When the quadrant result is 0 (center/NW), a random rotation is chosen instead
of the fixed preference list above. 4 rotations × 4 bytes (static .rdata),
selected via `FUN_0065c7e0(0, 3)` — random int in [0, 3]. Raw memory confirmed:
`01 02 03 04  02 03 04 01  03 04 01 02  04 01 02 03`.

| Rotation | Try order | Effective |
|----------|-----------|-----------|
| 0 | [1, 2, 3, 4] | 2, 3, 4 |
| 1 | [2, 3, 4, 1] | 2, 3, 4 |
| 2 | [3, 4, 1, 2] | 3, 4, 2 |
| 3 | [4, 1, 2, 3] | 4, 2, 3 |

This randomizes which of {2, 3, 4} gets tried first when entering from center.

## Placement Function — FUN_00481180

This is the primary function that assigns a sub-cell to infantry entering a cell.
20 callers throughout the engine. Signature:

```c
int* __thiscall PlaceInfantryInCell(
    CellClass* cell,       // param_1 (this)
    int* out_coords,       // param_2 — result: world lepton X, Y, Z
    uint* in_coords,       // param_3 — infantry's current lepton position
    char force_subcell,    // param_4 — if true, use subcell directly (no search)
    char is_bridge,        // param_5 — check bridge-level occupancy
    char use_cell_coords   // param_6 — if true, get base coords from cell vtable+0x48
);
```

### Algorithm (non-forced path, param_4 == 0):

1. Compute quadrant from `in_coords` (same distance/quadrant logic as GetSubCell)
2. Compute base coordinates: strip low byte from input coords (`x & ~0xFF`) to
   get cell base, or call cell's vtable+0x48 if `use_cell_coords` is set
3. Read occupancy byte from `cell+0x124` (ground) or `cell+0x128` (bridge)
4. **Pre-checks on occupancy byte:**
   - If **bit 5 (0x20)** is set → return failure coords. A non-infantry unit
     (vehicle) occupies this cell (set by UnitClass, file 141 at `0x0074b920`)
   - If **bit 6 (0x40)** is set at ground level → building present. Call
     `FUN_0047c4d0(6, 0)` to find the building (searches cell+0xE4 linked list
     for RTTI type 6). Then call `FUN_004525f0` to check if the building supports
     garrison (checks `BuildingTypeClass+0x16B7` flag). If not garrisonable →
     return failure coords
5. **Sub-cell selection:**
   - If quadrant == 0: pick random rotation from `DAT_0081cc98`
   - If quadrant == 2, 3, or 4: **fast-path** — check if that sub-cell is free.
     If free, skip directly to coordinate computation (goto LAB_00481437).
     If occupied, fall through to preference table `DAT_0081cc84[quadrant * 4]`
   - If quadrant == 1: always use preference table (no fast-path)
6. **Preference search loop** (max 4 iterations):
   - Read next sub-cell index from preference list
   - **Skip indices 0 and 1** (`if (uVar11 != 0 && uVar11 != 1)`)
   - Check occupancy bit: `cell_byte & (1 << subcell)`
   - If unoccupied: compute world coords from offset table → return
   - If all 4 entries exhausted: return failure coords
7. **Coordinate computation** (LAB_00481437):
   - `world_x = base_x + table[subcell * 3 + 0]`
   - `world_y = base_y + table[subcell * 3 + 1]`
   - `world_z = GetGroundHeight()` (+ bridge offset if on bridge)

### Forced path (param_4 != 0):

Skips all occupancy checks and preference search. Uses the quadrant result
directly — computes coordinates from the offset table with whatever sub-cell
index the quadrant detection returned.

### Failure coordinates — DAT_0089e778

Initialized to `(0, 0, 0)` by inline code at `0x0047b300`:
```asm
XOR EAX, EAX
MOV [0x0089e778], EAX   ; x = 0
MOV [0x0089e77c], EAX   ; y = 0
MOV [0x0089e780], EAX   ; z = 0
RET
```

Cell (0, 0) is outside the playable map area, so returning (0, 0, 0) effectively
signals "no valid placement found" to callers.

## Walk Locomotor Integration — FUN_0075c240

The WalkLocomotionClass calls `PlaceInfantryInCell` as part of its movement
processing chain. This is where sub-cell positions are actually assigned during
gameplay — not in PerCellProcess.

### Call chain:

```
WalkLocomotionClass::Process (vtable+0x40, FUN_0075ac80)
  → WalkLocomotionClass::ProcessMovement (FUN_0075aec0)
    → WalkLocomotionClass::FindSubCellDest (FUN_0075c240)
      → CellClass::PlaceInfantryInCell (FUN_00481180)
```

### FUN_0075c240 algorithm:

1. Call **UnmarkCellOccupancy** (entity vtable+0xF4 = `FUN_00521850`) on the
   entity's current position — clears the current sub-cell bit
2. Determine if on bridge (checks `cell[0x140] & 0x100` and height comparison)
3. Call **PlaceInfantryInCell** (`FUN_00481180`) to find available sub-cell at
   the destination cell — returns world lepton coordinates
4. Store result as locomotor destination (`loco+0x28/0x2C/0x30`)
5. Call **MarkCellOccupancy** (entity vtable+0xF0 = `FUN_005217c0`) on the new
   destination — sets the sub-cell bit at the target cell
6. The walk locomotor then **smoothly walks** the infantry toward the sub-cell
   position over subsequent frames — there is no instant snap

**Key insight:** The original engine does NOT snap infantry to sub-cell positions.
It assigns the sub-cell coordinates as a **walk destination** and the walk
locomotor interpolates the movement. The infantry smoothly walks from the cell
edge to its assigned sub-cell spot within the cell.

### vtable+0xF0 / vtable+0xF4 (virtual MarkCellOccupancy / UnmarkCellOccupancy)

These are virtual functions at the same vtable slot in different classes:
- **InfantryClass** vtable offset 0xF0/0xF4 → `FUN_005217c0` / `FUN_00521850`
  (bridge threshold `DAT_00a8f234`)
- **AnimClass** vtable offset 0xF0/0xF4 → `FUN_00426270` / `FUN_00426300`
  (bridge threshold `DAT_0089a1b4`)

Both override the same base class virtual slot (likely defined in ObjectClass).
The different bridge height thresholds reflect that AnimClass (explosions,
effects) and InfantryClass use independent bridge detection constants.

## Cell Occupancy

### CellClass Occupancy Fields

| Offset | Size | Purpose |
|--------|------|---------|
| `+0x054` | 4 | Ground-level **owner house ID** — set to `entity->Owner->field_0x30`; -1 when empty |
| `+0x058` | 4 | Bridge-level **owner house ID**; -1 when empty |
| `+0x0E4` | 4 | Ground-level occupier linked list head (used by FUN_0047c4d0 for building lookup) |
| `+0x0E8` | 4 | Bridge-level occupier linked list head |
| `+0x124` | 1 | Ground-level occupancy byte (bit field — see below) |
| `+0x128` | 1 | Bridge-level occupancy byte (bit field — see below) |
| `+0x140` | 4 | Cell flags (bit 0x100 = bridge present) |

### vtable+0x38 — Owner House ID Getter

The value stored in cell+0x054/0x058 comes from infantry vtable+0x38
(`0x006f9db0`), a tiny 3-instruction function:

```asm
MOV EAX, [ECX+0x21C]   ; entity->Owner (HouseClass* at TechnoClass+0x21C)
MOV EAX, [EAX+0x30]    ; Owner->field_0x30 (house array index / ID)
RET
```

This stores the **owning player's house ID** in the cell, enabling fast "who
occupies this cell?" queries without walking the linked list at cell+0xE4.

### Occupancy Byte Bit Field (cell+0x124 / cell+0x128)

| Bit | Hex | Meaning | Set by |
|-----|-----|---------|--------|
| 0 | 0x01 | Sub-cell 0 occupied | MarkCellOccupancy |
| 1 | 0x02 | Sub-cell 1 occupied | MarkCellOccupancy |
| 2 | 0x04 | Sub-cell 2 occupied | MarkCellOccupancy |
| 3 | 0x08 | Sub-cell 3 occupied | MarkCellOccupancy |
| 4 | 0x10 | Sub-cell 4 occupied | MarkCellOccupancy |
| 5 | 0x20 | **Non-infantry unit present** | UnitClass (file 141, `0x0074b920`) |
| 6 | 0x40 | **Building present** | TechnoClass (file 087, `0x005f6790`) |
| 7 | 0x80 | **Not used** | No setter found in decompiled files |

- Bits 0–4 are individual infantry sub-cell bits
- **0x1C** = bits 2+3+4 combined — the mask for "any functional infantry sub-cell"
- **0x20** blocks all infantry placement immediately (pre-check in FUN_00481180)
- **0x40** triggers a building garrison capability check before placement proceeds

The clear check in UnmarkCellOccupancy (`FUN_00521850`) tests `(byte & 0x1C) == 0`
to determine if all infantry sub-cells are empty, then resets the owner house ID
field (`+0x054` or `+0x058`) to -1.

### Bridge Detection

Both mark/unmark functions determine ground vs bridge level by checking:
```c
if (ground_height + bridge_offset <= entity_z
    && (cell[0x140] & 0x100) != 0)  // cell has bridge flag
    → use bridge level (+0x128, +0x058)
else
    → use ground level (+0x124, +0x054)
```

Bridge height threshold: `DAT_00a8f234` (runtime-initialized from rules.ini).

### Mark/Unmark Virtual Functions

These are virtual overrides at vtable offset 0xF0/0xF4, not standalone functions.
Each class that can occupy cells provides its own implementation:

**InfantryClass** (bridge threshold `DAT_00a8f234`):
- vtable+0xF0 → `FUN_005217c0` — MarkCellOccupancy: sets `1 << subcell` in
  occupancy byte, stores owner house ID (vtable+0x38) in cell+0x054/0x058
- vtable+0xF4 → `FUN_00521850` — UnmarkCellOccupancy: clears bit, resets house
  ID to -1 when 0x1C mask is empty

**AnimClass** (bridge threshold `DAT_0089a1b4`):
- vtable+0xF0 → `FUN_00426270` — MarkCellOccupancy (identical logic)
- vtable+0xF4 → `FUN_00426300` — UnmarkCellOccupancy (identical logic)

Confirmed via RTTI: `0x007eb058` = `.?AVInfantryClass@@`, `0x007e3354` =
`.?AVAnimClass@@`. Both at the same vtable slot offset (0xF0 from primary vtable).

### Occupancy Check — FUN_00481130

```c
// Returns 1 if sub-cell is free, 0 if occupied or reserved
uint IsSubCellFree(CellClass* cell, int subcell, char is_bridge) {
    if (subcell == 0 || subcell == 1)
        return 0;                     // Always "unavailable"
    if (is_bridge)
        return (cell[0x128] & (1 << subcell)) == 0;
    return (cell[0x124] & (1 << subcell)) == 0;
}
```

### Cell Occupier Lookup — FUN_0047c4d0

```c
// Walk cell's occupier linked list, find object of given RTTI type
Object* FindOccupierByType(CellClass* cell, int rtti_type, char is_bridge) {
    Object* obj = is_bridge ? cell[0xE8] : cell[0xE4];
    while (obj != NULL) {
        if (obj->WhatAmI() == rtti_type)   // vtable+0x2C
            return obj;
        obj = obj[0xC];                     // next in linked list (obj+0x30)
    }
    return NULL;
}
```

Called in the placement function with `rtti_type = 6` (BuildingClass) to check
if a building at the cell supports infantry garrison.

## FootClass Sub-Position Fields

From TECHNO_CLASS_FIELD_MAP.md (FootClass region, offset from object base):

| Offset | Size | Type | Field | Evidence |
|--------|------|------|-------|----------|
| +0x568 | 4 | int | Sub-position coord 1 | Report 040: CRC'd for sync |
| +0x56C | 4 | int | Sub-position coord 2 | Report 040: CRC'd for sync |
| +0x570 | 4 | int | Sub-position coord 3 | Report 040: CRC'd for sync |

**Confidence: LOW.** The only evidence is that these 12 bytes are included in the
sync CRC calculation (report 040). The label "sub-position" is inferred, not
confirmed — they could be velocity, intermediate coordinates, or other movement
state. The entity's actual world coordinates (used for rendering and collision)
are stored elsewhere in ObjectClass.

## Ghidra Addresses

| Address | Function | Verified |
|---------|----------|----------|
| `0x004810a0` | GetSubCell — quadrant from lepton position | Yes |
| `0x00481130` | IsSubCellFree — occupancy check for one sub-cell | Yes |
| `0x00481180` | PlaceInfantryInCell — full placement with preference search (20 callers) | Yes |
| `0x004525f0` | BuildingGarrisonCheck — checks if building supports infantry | Yes |
| `0x0047c4d0` | FindOccupierByType — walk cell occupier list by RTTI type | Yes |
| `0x0047b300` | Failure coords init — sets DAT_0089e778 to (0,0,0) | Yes |
| `0x0048e480` | Sub-cell offset table runtime initialization | Yes |
| `0x0089e9f0` | Sub-cell offset table (BSS, 5×3 ints, runtime-init) | Yes |
| `0x0081cc84` | Sub-cell preference order table (5×4 bytes, static .rdata) | Yes |
| `0x0081cc98` | Random rotation tables for center entry (4×4 bytes, static .rdata) | Yes |
| `0x0089e778` | Failure coordinates (BSS, 3 ints, init to 0,0,0) | Yes |
| `0x005217c0` | MarkCellOccupancy (YR bridge threshold) | Yes |
| `0x00521850` | UnmarkCellOccupancy (YR bridge threshold) | Yes |
| `0x00426270` | MarkCellOccupancy (alternate bridge threshold) | Yes |
| `0x00426300` | UnmarkCellOccupancy (alternate bridge threshold) | Yes |
| `0x0051fb00` | InfantryClass::Load — reads sub-cell from save format | Yes |
| `0x0054c550` | InfantryClass::PerCellProcess — drowning, scatter, crates | Yes |
| `0x0065c7e0` | Random(min, max) — RNG used for preference table rotation | Yes |
| `0x006d1fe0` | TacticalClass::CellToPixel — isometric projection formula | Yes |
| `0x0075c240` | WalkLoco::FindSubCellDest — calls PlaceInfantryInCell, sets walk target | Yes |
| `0x0075aec0` | WalkLoco::ProcessMovement — main walk processing, calls FindSubCellDest | Yes |
| `0x0075ac80` | WalkLoco::Process — ILocomotion vtable+0x40, entry point | Yes |
| `0x006f9db0` | InfantryClass::GetOwnerHouseID — vtable+0x38, returns entity->Owner->0x30 | Yes |
| `0x007eb058` | InfantryClass primary vtable (RTTI: `.?AVInfantryClass@@`) | Yes |
| `0x007e3354` | AnimClass primary vtable (RTTI: `.?AVAnimClass@@`) | Yes |
| `0x007f69f8` | WalkLocomotionClass ILocomotion vtable | Yes |

## Current Rust Engine Status

### Already Implemented
- `sub_cell: Option<u8>` field on `GameEntity` — tracks assigned sub-cell index
- `subcell_lepton_offset()` in `util/lepton.rs` — maps index 0–4 to lepton coords
  (values match original: 128/128, 64/64, 192/64, 64/192, 192/192)
- `lepton_sub_to_screen_offset()` in `util/lepton.rs` — formula-based conversion,
  matches original isometric projection. **This is the active code path.**
- `allocate_sub_cell()` / `allocate_sub_cell_with_reserved()` in `bump_crush.rs`
- Sub-cell occupancy tracking in `CellOccupancy.infantry: Vec<(u64, u8)>`
- **Sub-cell snapping on movement completion** — implemented in `movement.rs:1862`
  in the `finished_entities` loop: calls `subcell_lepton_offset(entity.sub_cell)`
  and sets `sub_x`/`sub_y` accordingly. Infantry visually spread out when idle.
  **Note:** The original engine does NOT snap — it assigns sub-cell coordinates as
  a walk destination and the walk locomotor smoothly interpolates. The Rust engine's
  instant snap is a simplification that works but isn't authentic behavior.

### Dead Code
- `sub_cell_screen_offset()` in `app_instances/helpers.rs` — hardcoded pixel
  offsets, **never called**. Also has a computation error: sub-cell 2 is listed
  as `(15.0, -7.5)` but the correct value from the isometric formula is
  `(15.0, 0.0)`. The `-7.5` is the screen_dy for sub-cell 1 (the dead entry),
  not sub-cell 2.

### Authenticity Bug: Wrong Functional Sub-Cells
`FUNCTIONAL_SUB_CELLS` in `bump_crush.rs` is `[0, 3, 4]` but should be
**`[2, 3, 4]`**. The original engine uses sub-cells 2 (NE), 3 (SW), 4 (SE) —
never sub-cell 0 (center). The Rust engine assigns infantry to cell center
instead of the NE corner, causing wrong visual spread pattern.

The comment at `bump_crush.rs:27` says "Matches the original engine's 3 usable
spots (center + 2 corners)" — this is incorrect. The original uses 3 corners,
not center + 2 corners. Additionally, the comment at line 172 says "first infantry
gets spot 2" but `FUNCTIONAL_SUB_CELLS[0]` is 0, not 2 (code/comment mismatch).

Infantry spawn in `game_entity.rs:190` defaults to sub-cell 2 (correct), but the
runtime allocation during movement uses the wrong `FUNCTIONAL_SUB_CELLS` array.

### Missing: Preference Table Logic
The Rust engine's `allocate_sub_cell()` does a simple linear search through
`FUNCTIONAL_SUB_CELLS`. The original engine:
1. Determines entry quadrant from infantry position
2. If the preferred sub-cell (matching quadrant) is free, uses it immediately
3. Otherwise consults a directional preference table biased toward the entry side
4. For center/NW entries, randomizes which sub-cell gets tried first

This affects infantry visual distribution — the original engine places infantry
closer to their entry direction, while the Rust engine always fills in the same
order regardless of approach direction.

---

## Tier 8 application record (2026-08-17, Claude Code session)

Corridor row 8. Snapshot: `C:/Users/enok/Documents/ghidra-backups/2026-08-17-pre-tier8`
(17 files, 243,605,513 bytes, verified with the program closed).

Structs created: `/UnitClass` **2280 B (0x8E8)**, 12 fields; `/InfantryClass` **1776 B (0x6F0)**,
7 fields; `/CDTimerClass` 12 B (`StartTime`, `AccumTime`, `Duration`). Both class sizes are
critic-confirmed from `PUSH <size>; CALL operator_new` sites — four for UnitClass, one for
InfantryClass. UnitClass's size is independently corroborated by its last member: a 512-byte
wide-char scratch buffer at 0x6E8 that closes the layout exactly on 0x8E8, which is also why the
constructor initialises nothing above 0x6E4.

### The headline: sub-cell position is NOT stored anywhere

`CellClass__GetSubCell` 0x004810A0 derives the index from the low bytes of X and Y every time it
is needed. The decisive argument (the critic supplied a better one than the applier had):
`InfantryClass__UnmarkCellOccupancy` 0x00521850 **recomputes** the index in order to clear its
own occupancy bit. Unmark is precisely where a cached index would be read if one existed. The
constructor initialises no such field, and `MarkCellOccupancy` 0x005217C0 recomputes on the way
in as well.

**Only {0, 2, 3, 4} are reachable — index 1 is dead.** `GetSubCell`:
`b = (x&0xFF) > 0x80; if ((y&0xFF) > 0x80) b |= 2; if (b == 0) return 0; return b + 1;`
Slot 1 at (64,64) inverts to `b == 0` and returns 0, so nothing can land there.
`CellClass__PlaceInfantryInCell` 0x00481180 rejects both 0 and 1 in its scan loop, so placement
assigns only 2, 3 or 4. **A five-slot sub-cell array in a port will drift against the occupancy
bitmask.**

Offsets, taken from the *writer* `CellClass__InitSubCellOffsets` (0x0048E489 onward) because the
table at 0x0089E9F0 is runtime-filled and reads as zeros in the static image — the same trap the
direction table sets:
slot 0 (128,128), 1 (64,64), 2 (192,64), 3 (64,192), 4 (192,192), leptons from the cell NW
corner, +X east / +Y south, Z always 0.

Determinism: a centre-coordinate placement consumes exactly one `Random__RandomRanged(0,3)` to
pick a rotated scan row; a corner placement consumes none. Tables verified byte-for-byte at
0x0081CC84 (fallback rows) and 0x0081CC98 (rotated rows).

**Sub-cell assignment runs through floating point.** `GetSubCell` computes distance from centre
via `Sqrt_Approx` on doubles through `Math__ftol`, returning 0 below 60 leptons. That is a second
float path in the sim after BulletClass's `atan2` facing.

### Two claims from this session that were REFUTED and must not propagate

1. **"The map-save writer calls `GetSubCell` rather than reading a field."** `get_xrefs_to
   0x004810A0` returns six callers: `AnimClass__MarkCellOccupancy` 0x0042627C,
   `AnimClass__ClearCellOccupancy` 0x00426309, `InfantryClass__MarkCellOccupancy` 0x005217CC,
   `InfantryClass__UnmarkCellOccupancy` 0x00521859,
   `JumpjetLocomotionClass__State4_Descend` 0x0054C679, and `FUN_0051FEF0` 0x0052001C. Whether
   that last one is the `[Infantry]` INI writer is DISPUTED between two agents and was not
   settled. Do not lean on it — the unmark-recomputes argument above is the load-bearing one.
2. **"The cell-full test is `(occupancy & 0x1C) == 0x1C`."** No such comparison exists.
   `PlaceInfantryInCell` exhausts via a loop counter (`if (3 < counter)` -> return invalid coord).
   The 0x1C mask appears in `UnmarkCellOccupancy` as `(b & 0x1C) == 0` — the **opposite** test,
   detecting *cell now empty* to reset the +0x54/+0x58 metadata to -1. Porting `== 0x1C` as a
   fullness gate would be a bug. (The mask does independently corroborate the {2,3,4} set: bits
   0x04|0x08|0x10, with 1<<1 = 0x02 never present.)

### Other negative findings that change the port

- **Ore cargo is not a UnitClass field.** `UnitClass__Get_Storage_Percentage` 0x007414A0:
  `LEA ECX,[ESI+0x33C]; CALL 0x006C9650; MOV EAX,[ESI+0x6C4]; FIDIV [EAX+0x800]`. The
  `StorageClass` lives at **TechnoClass+0x33C** and `StorageClass__GetTotalAmount` 0x006C9650
  loops **exactly 4 times** — one slot per tiberium type, each priced separately on unload.
  Capacity is `UnitTypeClass+0x800`. A single scalar "credits carried" collapses four types.
- **The harvester state machine is `MissionClass+0xBC`** (`dwMissionSubstate`, already named in
  tier 1), not a UnitClass field — `Mission_Harvest` has eight accesses to it, one dispatch read
  and seven state writes.
- **The docking refinery is not stored** on UnitClass for the harvest loop; it is re-derived by
  stepping one cell and looking the building up, with the persistent link being the RadioClass
  contact. Confirmed for the harvest loop specifically (`Mission_Harvest` never touches the one
  unmodelled pointer at UnitClass+0x6C8); unproven in general.
- **The deploy triad** `IsDeployed` 0x6E0 / `IsDeployAnimPlaying` 0x6E1 / `IsUndeployAnimPlaying`
  0x6E2 is what UnitClass adds beyond `FootClass::bIsDeploying` 0x6AD — and the separate
  *deploy-requested* byte is a fourth thing, at FootClass+0x68C. Four consumers gate on different
  members (Is_Ready_To_Commence, Scatter, DrawExtras, Facing_Update).

### Holes

UnitClass: 0x6C0 (int, ctor -1, decremented in AI, no reader found), **0x6C8** (applied as
`void *` — `PointerExpired` nulls it alongside Type, so it is provably an object reference, but
its identity is UNSETTLED and it is NOT the harvester dock link), 0x6D0, 0x6D3, 0x6DC
(bidirectional transient attachment, pointee type unknown), 0x6E3 padding.
InfantryClass: 0x6CC is the unused middle word of the `SequenceTimer` (the read pattern touches
only +0x00 and +0x08), plus 0x6D8, 0x6D9, 0x6DA, 0x6DC, 0x6DD, 0x6DE, 0x6DF, 0x6E0, 0x6E4 — all
ctor-zeroed with no reader found in any labelled InfantryClass body.

Flagged possibly dormant: `WaterSequenceState` 0x6E8 and the 0x10-0x16 water sequence family are
gated on `TechnoTypeClass+0x5B4 == 3`; whether any stock YR InfantryType sets that is UNCHECKED.
Do not port the water remap until that is settled. `IdleWanderDirection` 0x6D4 and
`DelayedDeathTimer` 0x6D8 are gated on `Type+0xE18`/`+0xE19` and `Type+0xE20` respectively, whose
stock values are likewise UNCHECKED.

### Corrections applied to this tier's own metadata

The `SequenceTimer` was first applied as two separate ints at 0x6C8 and 0x6D0. The critic showed
it is one 12-byte `CDTimerClass` with an unused middle word, matching the same shape already
found at TechnoClass+0x180. Rebuilt as a single `CDTimerClass` member. Also: `TypeClass+0xE3C` is
a **pointer to** the 0x24-byte sequence-record table, not the table itself — `Do_Action`
double-dereferences it. Porting it as an inline array would be wrong.
