# FootClass::Find_Nearby_Passable_Cell -- Deep Dive

**Address:** `0x56dc20`
**Size:** 381 decompiled lines, 797 instructions
**Confidence:** High (verified from binary, all branches traced)

## Purpose

Given a starting cell coordinate, searches outward in an expanding diamond/ring
pattern to find up to 24 candidate passable cells, then selects the best one
based on distance to a target or random selection. Used by MCV deploy, unit
scatter, harvester repositioning, and many other systems that need to place a
unit at a valid nearby cell.

---

## 1. Parameters (16 total)

This is a `__thiscall` on FootClass. `this` (ECX) = the FootClass instance.

| # | Decompiled Name | Type | Meaning |
|---|----------------|------|---------|
| 0 | `this` (implicit) | FootClass* | The unit searching for a cell. Search radius is derived from `this->Speed + this->SightRange` (offsets +0xF4, +0xF8 on instance). |
| 1 | `param_2` | CellStruct* (out) | **Output**: receives the chosen cell coordinate (X,Y packed as two shorts). |
| 2 | `param_3` | CellStruct* | **Origin cell**: the cell to search outward from (X at [0], Y at [1]). |
| 3 | `param_4` | int | **SpeedType**: locomotor speed category (0=Foot, 1=Track, 2=Wheel, etc.). Passed to CheckPassability for terrain cost lookups. Value 4 = "Fly" which bypasses all passability checks. |
| 4 | `param_5` | int | **Zone ID**: pathfinding zone the unit belongs to. -1 means "don't check zones". If 0xFFFF, converted to -1. Cells in different zones are rejected. |
| 5 | `param_6` | int | **Locomotor type**: movement class ID (0=Drive, 1=Walk, etc.). Used in zone and overlay checks. |
| 6 | `param_7` | bool | **Bridge-aware mode**: if true, adjusts the origin cell's reference height by +4 when the origin is a bridge cell (flag 0x100 in CellClass+0x140). Also controls whether `FUN_006d6410` (height-corrected cell lookup) is called to classify candidates as "direct" vs "indirect". |
| 7 | `param_8` | int | **Foundation width**: number of cells wide the object's foundation is (e.g., 2 for a 2x2 building). Passed to CheckPassability to verify all cells in the rect. |
| 8 | `param_9` | int | **Foundation height**: number of cells tall the foundation is. |
| 9 | `param_10` | int | **Overlay check flag**: passed into CheckPassability. When nonzero, cells with any overlay are rejected (OverlayTypeIndex != -1 causes failure). |
| 10 | `param_11` | bool | **Check height match**: if true, candidate cells must be within +/-2 height levels of the origin cell (accounting for bridge height offset). |
| 11 | `param_12` | bool | **Check cell occupants**: if true, calls `FUN_00486ff0` (cell occupant safety check) on each candidate. Rejects cells containing certain object types (buildings, specific locomotor types). |
| 12 | `param_13` | bool | **Reject bridge cells**: if true (nonzero), candidate cells that are bridge structural cells (flag 0x100) are REJECTED. If false, bridge cells are allowed. Note: the logic is inverted -- `param_13 != 0` means "allow bridge cells", `param_13 == 0` means "reject bridge cells". |
| 13 | `param_14` | CellStruct* | **Target cell**: the cell we ultimately want to reach. Used in the final selection phase to pick the candidate closest to this target. If equal to the null cell `{0,0}`, random selection is used instead. |
| 14 | `param_15` | bool | **Skip first quadrant**: when true, the first two candidate positions (north/south in each ring) are skipped. Used to produce a different search pattern. |
| 15 | `param_16` | bool | **Check occupancy rect**: if true, calls `CellRect::CheckOccupancy` on each candidate cell rect. Rejects cells that already contain objects occupying that cell. |

---

## 2. Search Radius Computation

```c
int search_radius = *(int*)(this + 0xF4) + *(int*)(this + 0xF8);
if (search_radius > 32) {
    search_radius = 32;  // hard cap
}
```

Offsets +0xF4 and +0xF8 on the FootClass instance are the unit's **cached Speed** and
**Sight range** values (derived from TechnoTypeClass at init time). The sum is capped
at 32 cells. A typical tank (Speed=6, Sight=8) searches up to 14 cells out; a fast
scout (Speed=10, Sight=10) searches 20. Slow units search less, reducing computation.

---

## 3. Search Pattern -- Expanding Diamond/Ring

The function searches outward in concentric diamond-shaped rings of increasing
radius, from ring 0 (the origin) to ring `search_radius - 1`.

For each ring at radius `r`, it visits cells in 4 segments that together form a
diamond perimeter:

### Ring traversal (radius `r`, origin = `(ox, oy)`)

**Segment 1 -- Top-right to bottom-right edge (N->S on the east side):**
```
for delta = -r to +r:
    candidate = (ox + delta, oy - r)   // "north" half
    candidate = (ox + delta, oy + r)   // "south" half  (skipped if delta == -r)
```

**Segment 2 -- Left and right columns (W and E sides, excluding corners):**
```
for delta = (1 - r) to (r - 1):
    candidate = (ox - r, oy + delta)   // west column
    candidate = (ox + r, oy + delta)   // east column
```

In each segment, for each candidate cell, the full validation pipeline runs (see
section 4). The `param_15` flag can skip certain positions in segments 1 & 2.

The inner loop iterates `delta` from `-r` to `+r` for segments 1-2, then from
`(1-r)` to `(r-1)` for segments 3-4. This produces a complete diamond perimeter:

```
Ring r=2 example (relative to origin O):
         .  2  .
        2  .  .  2
       .  .  O  .  .
        2  .  .  2
         .  2  .
```

### Early termination conditions

The search stops when ANY of these is true:
1. 24 candidates have been collected (`local_1d4 == 0x18`)
2. At least one "direct" candidate was found (`local_1d5 != 0`) AND the current
   ring is complete
3. The ring radius exceeds `search_radius`

Condition 2 means: once ANY valid "direct" candidate (one that maps to itself after
height correction -- see section 6) is found on a ring, the algorithm finishes that
ring and stops. It does NOT search further rings. This biases toward closer cells.

---

## 4. Per-Candidate Validation Pipeline

For each candidate cell `(cx, cy)`, these checks run in order. ALL must pass for
the cell to be accepted as a candidate.

### 4a. Map bounds / Cell lookup

The cell coordinate is converted to a linear index `cy * 512 + cx`. If out of
range `[0, 0x3FFFF]` or the CellClass pointer is null, a dummy cell is used.

### 4b. `TechnoClass::IsOnScreen` (0x578540)

**What it checks:** whether the candidate cell falls within the visible/valid map
rectangle. Despite the name, this is really `MapClass::IsCellInPlayfield` -- it
verifies the cell coordinates against the tactical viewport/map bounds. If the cell
is outside the playable map area, it is rejected.

Called as: `TechnoClass__IsOnScreen(cell_ptr, 1)`

### 4c. `CellRect::CheckPassability` (0x56E7C0)

Verifies the entire foundation rectangle (width x height cells) starting at the
candidate cell is passable for the given movement type.

**Full signature (reconstructed from stack params):**
```c
bool CellRect::CheckPassability(
    CellStruct* top_left,      // candidate cell
    int foundation_width,       // param_8
    int foundation_height,      // param_9
    int speed_type,             // param_4
    int zone_id,                // param_5 (-1 = skip zone check)
    int locomotor_type,         // param_6
    int required_height,        // always -1 here (any height)
    bool bridge_aware,          // param_7
    int overlay_check           // param_10
)
```

It iterates over the full `width x height` rectangle. For each sub-cell, it calls
`CellClass::CheckCellPassability` (0x4834A0) which checks:

1. **Zone connectivity**: if `zone_id != -1`, the cell's zone must match
2. **Height/bridge matching**: if `required_height != -1`, the cell's height level
   must match (with bridge height +4 handling)
3. **Occupation flags**: checks `OccupationFlags` (or `AltOccupationFlags` for
   bridge cells). Infantry-only and vehicle-only flags can be masked out via
   `check_infantry` / `check_vehicles` params.
4. **Terrain cost**: looks up `SpeedType x LandType` in the global speed table.
   If cost is -1.0 (impassable), rejects the cell.
5. **Wall/overlay passability**: if the cell has an overlay marked as `IsWall`,
   only certain locomotor types can pass (Hover=2, Mech=3, Amphibious=8,
   Ship=1, Float=4, or locomotors with `IsWallBuster`).

Additionally, the wrapper checks: if `overlay_check` is true and the cell has
any overlay (`OverlayTypeIndex != -1`), the cell is immediately rejected before
calling CheckCellPassability.

Returns `true` only if ALL sub-cells pass.

### 4d. Height/elevation matching (the +/-2 check)

**Gated by:** `param_11` (check height match)

```c
if (param_11) {
    int origin_height = cell_at_origin.Level;  // +0x11B, signed byte
    // If bridge-aware and origin is a bridge cell, add 4
    if (param_7 && (origin_cell.Flags & 0x100))
        origin_height += 4;

    int candidate_height = candidate_cell.Level;
    // Adjust for bridge: subtract 4 if candidate IS a bridge cell
    int adjusted = origin_height + ((candidate_cell.Flags >> 8) & 1) * -4;
    int diff = adjusted - candidate_height;

    if (abs(diff) >= 2)
        reject;  // height difference too large
}
```

This ensures candidate cells are within 2 height levels of the origin. Bridge cells
get a +4 height adjustment. This prevents units from being placed on cliff tops when
searching from a valley, or vice versa.

### 4e. `TechnoClass::Is_Current_Cell_Obstacle_Free` -- Cell occupant safety check (0x486FF0)

<!-- corrected 2026-05-28: was labelled `FUN_00486ff0`; binary name is TechnoClass__Is_Current_Cell_Obstacle_Free via get_function_by_address — RTTI_LABEL_DRIFT -->

**Gated by:** `param_12` (check cell occupants)

This function checks whether the candidate cell is safe to move into by examining
what objects are currently on the cell. It returns `true` (safe) or `false` (blocked).

**Logic:**
1. Call `TechnoClass::IsOnScreen(cell, 1)` -- if cell is off-screen, return `true`
   (allow; don't block on invisible cells).
2. Check if the cell's `AbstractType` (offset +0x38) refers to a type whose
   `IsInsignificant` flag (type+0x305) is set -- if so, return `false` (blocked).
3. Check if `cell+0x11C != 0` (cell has special flag) OR `cell+0x140 & 0x500`
   (bridge-related flags) -- if so, return `false`.
4. Walk the cell's object linked list (`cell+0xE4`). For each object:
   - Call `object->WhatAmI()` (vtable+0x2C). If result == 6 (BuildingClass),
     return `false` (building blocks the cell).
5. Walk the list again checking for locomotor type 0x24 (36 = teleport/chrono).
   If any object is teleporting, return `false`.
6. Otherwise return `true`.

### 4f. Bridge cell rejection

**Gated by:** `param_13` (inverted logic)

```c
if (param_13 == false) {  // param_13 == 0 means "reject bridge cells"
    if (candidate_cell.Flags & 0x100)  // is bridge structural cell
        reject;
}
```

When `param_13` is false/zero, any cell with the bridge structural flag (0x100 in
CellClass+0x140) is rejected. When true/nonzero, bridge cells are allowed.

### 4g. `CellRect::CheckOccupancy` (0x586780)

**Gated by:** `param_16` (check occupancy rect)

Checks the entire foundation rectangle for object occupation. For each cell in the
rect:

1. Call `FUN_0047c550(cell, 0)` -- searches the cell's object list for locomotor
   type 0x24 (teleport). If found, reject.
2. Check if the cell's `GapGenBitmask` (offset +0xDC; NOT OccupationFlags, which is at +0x124) has any bits matching the
   caller's bitmask. If `param_5 == -1`, the mask is 0 (skip). Otherwise
   `mask = 1 << (param_5 & 0x1F)`.
3. Check `cell+0x44 != -1` (cell has terrain object) -- if so, reject.
4. Check `cell+0x4C != 0` (cell has overlay object) or `cell+0x11C != 0`
   (special cell flag) -- if so, reject.
5. Call `Look_up_building_in_cell` (0x47C520) -- walks the object list looking
   for `WhatAmI() == 6` (building). If found, reject.
6. Finally, call `MapClass__IsRectInPlayfield` (0x578390) which verifies all 4 corners of the foundation
   rectangle are within the playable map area. (corrected 2026-05-28: was `FUN_00578390`; binary name confirmed via get_function_by_address)

Returns nonzero on ANY rejection (the first failing check's result), zero if all clear.

---

## 5. `FUN_006d6410` -- Height-Corrected Cell Lookup (0x6D6410)

This function resolves the "true" cell at a given lepton position after accounting
for terrain height differences. It is used to classify candidates as "direct" vs
"indirect".

**Inputs:**
- `param_1`: output CellStruct*
- `param_2`: lepton coordinate (X, Y, Z=0) -- cell * 256 + 128 (cell center)

**Algorithm:**
1. Convert lepton to cell coords: `cell_x = lepton_x >> 8`, `cell_y = lepton_y >> 8`
2. Get the reference cell's height level (`+0x11B`) and flags (`+0x140`)
3. Starting from the lepton position, iteratively subtract 8 leptons from both X and Y
   (walking "up" the isometric projection):
   - At each step, compute the cell at the new position
   - Calculate height difference: `delta_h = new_cell.Level - origin.Level`
   - If origin has bridge flag bit 12 (`flags >> 0xC & 1`), add bridge height:
     `delta_h += (new_cell.Flags >> 8 & 1) * 4`
   - Offset the lepton position by `-delta_h * 128` (height displaces the visual
     position by 128 leptons per level, i.e., half a cell)
   - If the offset cell coords are <= the input cell coords, we've overshot: return
     the last intermediate cell
   - If the intermediate cell equals the input cell, return the input (identity)
4. Returns the cell that "visually" corresponds to the lepton position after height
   projection

**Purpose in Find_Nearby_Passable_Cell:** A candidate cell is converted to lepton
center coordinates, then run through this function. If the result cell matches the
candidate cell, it is a "direct" cell (the cell is exactly where it appears to be
visually). If not, it is "indirect" (the cell's visual position is displaced by
height, meaning something else appears at that screen location).

---

## 6. Final Selection Algorithm

After the search loop collects up to 24 candidates in `local_120[]` (array of
CellStruct, 48 shorts = 24 entries), the selection phase runs.

### 6a. Split candidates into "direct" vs "indirect" groups

Each candidate cell is converted to lepton center coords `(cell_x * 256 + 128,
cell_y * 256 + 128, 0)` and run through `FUN_006d6410` (height-corrected lookup).

- If the returned cell **matches** the input cell: **direct** candidate.
  Stored in `local_c0[]` (up to 24 entries). Count: `direct_count`.
- If the returned cell **differs**: **indirect** candidate.
  Stored in `local_60[]` (up to 24 entries). Count: `indirect_count`.

Direct cells are preferred because they are at the expected visual position.

### 6b. Selection when NO target specified

If `param_14` (target cell) equals the null cell `{0, 0}`:

```
if (direct_count > 0):
    result = local_c0[g_CurrentFrameCounter % direct_count]  // pseudo-random from directs
    // NOTE: code has a quirk -- accesses local_60[frame % direct_count - 24]
    // which is actually local_c0[frame % direct_count] due to array layout
else:
    result = local_60[g_CurrentFrameCounter % indirect_count]  // from indirects
```

Uses the global frame counter modulo candidate count for deterministic pseudo-random
selection. This ensures lockstep-safe "random" picks.

### 6c. Selection when target IS specified

If a target cell is provided:

```
candidate_pool = direct_count > 0 ? direct_candidates : indirect_candidates;
pool_size = direct_count > 0 ? direct_count : indirect_count;

best_distance = 100000.0;
best_cell = null_cell;

for each candidate in pool:
    dx = candidate.X - target.X
    dy = candidate.Y - target.Y
    dist = sqrt(dx*dx + dy*dy)
    if (dist < best_distance):
        best_distance = dist
        best_cell = candidate

result = best_cell;
```

Picks the candidate closest to the target cell (Euclidean distance). Direct
candidates are preferred; indirect ones are only used if no direct candidates exist.

### 6d. Fallback

If zero candidates were collected (nothing passable found), the output is set to
the null cell `DAT_00abd480` = `{0, 0}`.

---

## 7. Maximum Candidates

The maximum number of candidates is **24** (`0x18`). The candidate array
`local_120[48]` holds 24 CellStruct entries (each CellStruct = 2 shorts = 4 bytes,
stored at 2 short-array positions each). Once 24 are collected, the search
immediately jumps to the selection phase.

---

## 8. Complete Pseudocode

```
fn find_nearby_passable_cell(
    this: &FootClass,
    origin: CellStruct,
    speed_type: i32,
    zone_id: i32,           // 0xFFFF -> -1
    locomotor_type: i32,
    bridge_aware: bool,
    foundation_w: i32,
    foundation_h: i32,
    overlay_check: i32,
    check_height: bool,
    check_occupants: bool,
    reject_bridge: bool,     // inverted: false = reject bridges
    target: CellStruct,
    skip_first_quad: bool,
    check_occupancy_rect: bool,
) -> CellStruct {

    // --- Setup ---
    if zone_id == 0xFFFF { zone_id = -1; }

    let origin_cell = get_cell(origin);
    let mut ref_height = origin_cell.Level;  // +0x11B, signed byte

    if bridge_aware && (origin_cell.Flags & 0x100) != 0 {
        ref_height += 4;  // bridge adds 4 height levels
    }

    let search_radius = min(this.speed + this.sight, 32);  // +0xF4 + +0xF8, cap 32

    let mut candidates: [CellStruct; 24];
    let mut candidate_count = 0;
    let mut found_direct = false;

    // --- Expanding diamond search ---
    for ring in 0..search_radius {
        // Segment 1: top and bottom edges
        // delta goes from -ring to +ring
        for delta in -ring..=ring {
            if !skip_first_quad {
                // North candidate: (origin.x + delta, origin.y - ring)
                let cell = (origin.x + delta, origin.y - ring);
                if try_candidate(cell, &mut candidates, &mut candidate_count, &mut found_direct) {
                    break;
                }
            }
            if candidate_count == 24 { goto select; }

            if !skip_first_quad || delta > -ring {
                // South candidate: (origin.x + delta, origin.y + ring)
                let cell = (origin.x + delta, origin.y + ring);
                if try_candidate(cell, &mut candidates, &mut candidate_count, &mut found_direct) {
                    break;
                }
            }
            if candidate_count == 24 { goto select; }
        }
        if candidate_count == 24 { goto select; }

        // Segment 2: left and right edges (excluding corners)
        // delta goes from (1 - ring) to (ring - 1)
        for delta in (1-ring)..=(ring-1) {
            if !skip_first_quad {
                // West candidate: (origin.x - ring, origin.y + delta)
                let cell = (origin.x - ring, origin.y + delta);
                try_candidate(cell, ...);
            }
            if candidate_count == 24 { goto select; }

            // East candidate: (origin.x + ring, origin.y + delta)
            let cell = (origin.x + ring, origin.y + delta);
            try_candidate(cell, ...);
            if candidate_count == 24 { goto select; }
        }

        // End of ring: if we found a direct candidate, stop searching
        if candidate_count == 24 || found_direct { goto select; }
    }
    // If 0 candidates found, return null cell {0,0}
    if candidate_count == 0 { return NULL_CELL; }

select:
    // --- Classify candidates ---
    let mut direct: Vec<CellStruct>;   // max 24
    let mut indirect: Vec<CellStruct>; // max 24

    for each candidate in candidates[0..candidate_count] {
        let lepton = (candidate.x * 256 + 128, candidate.y * 256 + 128, 0);
        let resolved = height_corrected_lookup(lepton);

        if resolved == candidate {
            direct.push(candidate);
        } else {
            indirect.push(candidate);
        }
    }

    // --- Select best ---
    if target == NULL_CELL {
        // No target: pseudo-random selection using frame counter
        if direct.len() > 0 {
            return direct[g_FrameCounter % direct.len()];
        } else {
            return indirect[g_FrameCounter % indirect.len()];
        }
    } else {
        // Target specified: pick closest candidate
        let pool = if direct.len() > 0 { &direct } else { &indirect };
        let mut best_dist = 100000.0;
        let mut best = NULL_CELL;

        for cell in pool {
            let dx = cell.x - target.x;
            let dy = cell.y - target.y;
            let dist = sqrt((dx*dx + dy*dy) as f64);
            if dist < best_dist {
                best_dist = dist;
                best = cell;
            }
        }
        return best;
    }
}

fn try_candidate(cell, candidates, count, found_direct) -> bool {
    // 1. Map bounds check (cell index in [0, 0x3FFFF])
    let cell_obj = get_cell(cell);

    // 2. Is cell within playable map area?
    if !is_on_screen(cell_obj) { return false; }

    // 3. Is full foundation rect passable?
    if !check_passability(cell, foundation_w, foundation_h,
                          speed_type, zone_id, locomotor_type,
                          -1 /*any height*/, bridge_aware, overlay_check) {
        return false;
    }

    // 4. Height match (+/-2 levels)
    if check_height {
        let candidate_height = cell_obj.Level;
        let bridge_adj = ((cell_obj.Flags >> 8) & 1) as i32 * -4;
        let diff = (ref_height + bridge_adj) - candidate_height;
        if diff.abs() >= 2 { return false; }
    }

    // 5. Cell occupant safety
    if check_occupants {
        if !cell_occupant_check(cell_obj) { return false; }
    }

    // 6. Bridge rejection
    if !reject_bridge_param {  // param_13 == 0 means reject bridges
        if (cell_obj.Flags & 0x100) != 0 { return false; }
    }

    // 7. Foundation occupancy
    if check_occupancy_rect {
        if !check_rect_occupancy(cell, -1) { return false; }
    }

    // --- Accepted! ---
    candidates[count] = cell;
    count += 1;

    // Classify as direct/indirect (bridge-aware mode only)
    if !bridge_aware {
        let lepton = (cell.x * 256 + 128, cell.y * 256 + 128, 0);
        let resolved = height_corrected_lookup(lepton);
        if resolved != cell {
            return false;  // indirect: DON'T set found_direct, skip to next
        }
    }
    found_direct = true;
    return false;  // continue searching
}
```

---

## 9. Key Constants and Addresses

| Item | Address/Value |
|------|--------------|
| `FootClass::Find_Nearby_Passable_Cell` | `0x56DC20` |
| `CellRect::CheckPassability` | `0x56E7C0` |
| `CellClass::CheckCellPassability` | `0x4834A0` |
| `CellRect::CheckOccupancy` | `0x586780` |
| `TechnoClass::Is_Current_Cell_Obstacle_Free` (cell occupant safety) | `0x486FF0` |
| `FUN_006D6410` (height-corrected cell lookup) | `0x6D6410` |
| `TechnoClass::IsOnScreen` (map bounds) | `0x578540` |
| `Look_up_building_in_cell` | `0x47C520` |
| `FUN_0047C550` (find teleporting object) | `0x47C550` |
| `MapClass__IsRectInPlayfield` (rect in playfield check) | `0x578390` |
| `g_CellArray_Base` | `0x87F924` |
| `DAT_00abd480` (null cell {0,0}) | `0xABD480` |
| `g_CurrentFrameCounter` | global (frame-based RNG seed) |
| Max candidates | 24 (`0x18`) |
| Max search radius | 32 (`0x20`) |
| CellClass.Level | offset `+0x11B` (signed byte) |
| CellClass.Flags | offset `+0x140` (u32 bitfield) |
| CellClass.Flags bit 0x100 | Bridge structural cell |
| CellClass.GapGenBitmask | offset `+0xDC` (u32) — **not** OccupationFlags (those are at +0x124) |
| CellClass.OverlayTypeIndex | named field (via struct) |
| CellClass.LandType | offset after overlay checks |
| SpeedType x LandType table | `g_SpeedType_LandType_Table` |

---

## 10. Callers (notable)

- `UnitClass::Mission_Deploy_Building` (0x73D630) -- MCV finding deploy spot (corrected 2026-05-28: was 0x73D7B0; binary entry via get_function_by_address — GHIDRA_ADDRESS_SHIFT, prior value was a call-site offset)
- `MapClass__PlaceCrateAtRandomCell` (0x56BD40) -- random passable cell for spawning (corrected 2026-05-28: was labelled `FUN_0056BD40`; function is now named MapClass__PlaceCrateAtRandomCell)
- `UnitClass::Mission_Harvest` (0x73E5E0) -- harvester repositioning (corrected 2026-05-28: was `FootClass::Mission_Harvest @ 0x4D6CE1`; binary shows UnitClass__Mission_Harvest @ 0x73E5E0 — GHIDRA_ADDRESS_SHIFT + wrong class name; 0x4D6CE1 is inside FootClass__Mission_AreaGuard)
- `TechnoClass::Set_Destination` (0x741970) -- destination correction (corrected 2026-05-28: was 0x742042; binary entry via get_function_by_address — GHIDRA_ADDRESS_SHIFT)
- `InfantryClass::Scatter` (0x51D0D0) -- infantry scatter (corrected 2026-05-28: was 0x51D41D; binary entry via get_function_by_address — GHIDRA_ADDRESS_SHIFT)
- `UnitClass::Scatter` (0x743A50) -- vehicle scatter (corrected 2026-05-28: was 0x743C6B; binary entry via get_function_by_address — GHIDRA_ADDRESS_SHIFT)
- `TeleportLocomotionClass::Process` (0x718B70) -- chrono teleport landing (corrected 2026-05-28: was 0x71900D; binary entry via get_function_by_address — GHIDRA_ADDRESS_SHIFT)
- `BuildingClass::OnConstructionComplete` (0x445F80) -- rally point finding (corrected 2026-05-28: was 0x446A14; binary entry via get_function_by_address — GHIDRA_ADDRESS_SHIFT)
- Many team/convoy script functions
