---
name: Find-nearest variants — spiral / search-order comparison
description: Cross-cutting comparison of every gamemd.exe routine that searches outward for a "nearby" cell or object, with exact iteration order, tie-break, RNG hookup, and parity-relevant differences between variants. Aggregator over per-system docs.
type: reference
---

# Find-Nearest Variants — Spiral / Search-Order Comparison

**Scope:** Every gamemd.exe routine that picks a "nearby" cell, building, or
object via outward search. Built on top of the existing per-system reports
(`FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`,
`SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md`,
`UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`,
`BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md`,
`MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`,
`BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md`). This document only adds: the
unified comparison table, the exact tie-break rules per variant verified
in Ghidra, and the cross-system parity hazards that fall out when several
variants run on the same tick.

**Confidence:** HIGH for every variant covered (each was decompiled or
re-decompiled for this report — see "Per-Variant Verification" section).

**Active in YR:** All variants are active in standard YR skirmishes. None
of these are TS-legacy. (Per-cell occupancy bitmask `0xDC` and AltOccupy
bridge variant are TS-era field semantics but live in the YR code paths.)

---

## 1. The Eight Variants

| # | Function | Address | Pattern | Triggered by |
|---|----------|---------|---------|--------------|
| A | `FootClass::Find_Nearby_Passable_Cell` | `0x56DC20` | Diamond-ring spiral, up to 24 candidates, then closest-to-target or frame-modulo random | MCV deploy, scatter destination, harvester reposition, chrono teleport landing, rally point, AI wander, naval exit fallback |
| B | `UnitClass::Scatter` | `0x743A50` | 8-direction sequential scan from a heading-derived start | Locomotor blocked, crusher entering occupied cell, factory bib clear (when targeting vehicles) |
| C | `InfantryClass::Scatter` | `0x51D0D0` | 8-direction sequential scan, **wider** random offset | Same triggers as B but for infantry |
| D | `FootClass::Find_Nearest_Dock` | `0x4DFCB0` | **No spiral** — linear backward scan over `owner.Buildings[]`, picks nearest by 3-D Euclidean distance | Garrison auto-target, engineer auto-target (dispatched via `BuildingClass::CanDock`) |
| E | `BuildingClass::GetDockCellForObject` foundation perimeter scan | `0x44EFB0` | Fixed-order perimeter walk: top/bottom-row interleaved X-sweep, then right/left-col interleaved Y-sweep | War factory / barracks / hospital exit when no `ExitList` is set, or building is Hospital |
| F | `FUN_005060B0` (non-Naval branch — find building placement-clear cell) | `0x5060B0` | `ExitList` table sorted by index×1000 + (distance × 1000) for upgrades, then 8-direction probe + 3-step push per probe, with bridge retry pass | Defense exit cell, base-defense placement clearance |
| F′ | `FUN_005060B0` (Naval branch — same function, naval flag) | `0x5060B0` (entry at `0x506239`) | Defers to A with `(W+2, H+2)` foundation, +1 SpeedType=5 (Float) | Naval Yard ship exit |
| G | `BuildingTypeClass::CanBePlacedAt` | `0x45EE70` | **No outward search** — iterates the foundation `OccupyList` in INI order; first failing cell short-circuits | MCV deploy, AI deploy scheduler, build-queue exit (placement-commit only — see §10 RQ5; cursor preview uses a separate routine) |

Garrison **entry** itself (`InfantryClass::Mission_Enter`, `0x5196A0`) and WF bib
clear (`0x449540`) are NOT search variants — they are addressed in §11 because
the user often associates them with this family.

---

## 2. The Two Direction-Offset Tables (the rosetta stone)

There are **two physically distinct memory regions** holding the same logical
direction-offset values. A re-impl needs to know about both — they are NOT
the same address.

| Region | Address | Initialized by | Read by |
|--------|---------|----------------|---------|
| **Static `g_DirectionOffsets`** | `0x89F688` | (static / link-time, region appears zero in the disk image — actual runtime values are populated by the loader or a one-time init not in `FUN_005060B0`) | `UnitClass::Scatter`, `InfantryClass::Scatter`, `FootClass::Mission_AreaGuard`, ~30+ other consumers across pathfinding, bridges, walls, animations, `RevealCell`, etc. |
| **Runtime cache** | `0xA8EF78` (Table 1) and `0xA8EFA8` (Table 2) | `FUN_005060B0` only, on first call, gated by `DAT_00A8F004 & 1` and `& 2` | `FUN_005060B0` only |

The contents of both regions match (same 8 entries in the same order — verified
because the game's behavior is consistent across all consumers and the
`FUN_005060B0` init writes match what other readers expect). The only reason
the runtime cache exists is `FUN_005060B0` re-deriving the table locally.

**Re-impl note:** define ONE direction-offset table and have all consumers read
from it. The two-region split in gamemd.exe is a build-time artifact, not a
behavioral feature.

### Table 1 — `g_DirectionOffsets` / `DAT_00A8EF78` (CW from West)

8 entries × `{short dx, short dy}` (4 bytes each, total 32 bytes).

| idx | dx | dy | direction |
|-----|-----|-----|-----------|
| 0   | -1  |  0  | W         |
| 1   | -1  | -1  | NW        |
| 2   |  0  | -1  | N         |
| 3   |  1  | -1  | NE        |
| 4   |  1  |  0  | E         |
| 5   |  1  |  1  | SE        |
| 6   |  0  |  1  | S         |
| 7   | -1  |  1  | SW        |

Used by:
- `UnitClass::Scatter` 8-direction probe loop (indexed `aiStack_3c[0] + (uVar14 & 7) & 7`)
- `InfantryClass::Scatter` 8-direction probe loop (indexed `iVar7 + (iVar3 >> 8 & 0xffff) & 7`)
- `FUN_005060B0` outer probe (one offset per candidate cell)

### Table 2 — `DAT_00A8EFA8` (CW from East — Table 1 rotated by +4 mod 8)

| idx | dx | dy | direction |
|-----|-----|-----|-----------|
| 0   |  1  |  0  | E         |
| 1   |  1  |  1  | SE        |
| 2   |  0  |  1  | S         |
| 3   | -1  |  1  | SW        |
| 4   | -1  |  0  | W         |
| 5   | -1  | -1  | NW        |
| 6   |  0  | -1  | N         |
| 7   |  1  | -1  | NE        |

Used by:
- The 3-step "push further" inner loop inside `FUN_005060B0`
- The "bridge retry" path in `FUN_005060B0`
- The `&DAT_0089F68A` reference inside `UnitClass::Scatter` and `InfantryClass::Scatter`
  (it's `&Table1[i] + 2`, i.e. picks the dy element when stepping)

**Tiny detail (load-bearing for parity):**
the literal expression `&DAT_0089F68A` that appears inside `UnitClass::Scatter`
(`uStack_40 = CONCAT22((&DAT_0089f68a)[uVar6 * 2] + ..., ...)`) is **not** a
separate table — it's `(short*)&g_DirectionOffsets + 1`, i.e. the 2nd short of
each Table 1 entry, used because the pair is loaded as two separate halves.
Re-implementations should NOT define a third table.

A second runtime gate `DAT_00A8F004 & 2` initializes a second region
(`DAT_00A8EFA8 .. DAT_00A8EFC4`) with Table 2 values. The two gates are
independent — either or both may be initialized at any point.

---

## 3. Per-Variant Algorithm — the Tiny Details

### 3.1 Variant A — `Find_Nearby_Passable_Cell` (0x56DC20)

**Already documented exhaustively in [FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md](FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md).**

Tiny details that matter for the **comparison table** (added here, not in the parent doc):

- **Ring 0 is *not* iterated.** The loop in the parent doc reads "for r in 0..radius",
  but at r=0 both segment loops are empty (`-r..+r` with r=0 is just delta=0; segment 2
  loops over `1..-1` which is empty). So the search starts at the cells **adjacent**
  to the origin, not at the origin itself. The origin is only reached by the per-tick
  passability check upstream of this function, never by Find_Nearby_Passable_Cell.
- **Ring traversal interleaves N/S then W/E pairs.** First the top row pair `(ox+δ, oy-r)`
  and the bottom row pair `(ox+δ, oy+r)` are tested for each `δ ∈ [-r, +r]` (so NW corner
  comes before NE, but only after testing SW). Then segment 2 walks left/right columns.
  → For tied distances on a ring, the NW quadrant is tried before NE before W/E columns.
- **`skip_first_quad` (param_15) skips the south-row test only when `δ == -r`.** That
  is one cell on each ring (the SW corner). It is *not* a 90°-quadrant skip — the name
  `skip_first_quad` is misleading. A faithful re-impl needs the off-by-one.
- **`local_1d5` (the "found-direct" flag) is set inside the per-candidate accept path.**
  Once it's set, the *current ring* still completes; only at the ring boundary does the
  outer loop stop. So the candidate count after a single direct hit on ring `r` may be
  anywhere from 1 to `4·r` (one per cell on the ring). That spread directly affects the
  modulo-frame "random" tie-break.
- **The frame-modulo random is `g_CurrentFrameCounter % count`.** It is **not**
  `Random__RandomRanged`. This means two Find_Nearby_Passable_Cell calls on the same
  frame for the same candidate count return the same index — which is desired for
  determinism but also means a re-run of the same scenario is bit-identical even
  without any seeded RNG state.
- **Distance metric for the "closest to target" branch is `sqrt`-based.** The code
  literally calls `Sqrt_Approx` (a custom approximate sqrt at `0x4CAC40`) and uses
  `Math__ftol` to truncate. Two candidates with identical `dx²+dy²` produce identical
  truncated distances; in that case the **first-tested** candidate wins (because the
  comparison is strict `<`). Since the ring traversal order is fixed, ties are
  deterministic and break NW-first.

### 3.2 Variant B — `UnitClass::Scatter` (0x743A50)

8-direction sequential probe. Re-decompiled for this report.

Steps (the parent `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md` covers gating; this
section adds the spatial details):

1. Compute *seed direction* `uVar14`:
   - If the threat coord is the **null coord** `DAT_00B1CFE8` (no direction hint),
     fall through to the alternate path which uses Find_Nearby_Passable_Cell directly
     (variant A). No 8-direction scan in that branch.
   - Else: `angle = atan2(coord.Y - my.Y, my.X - coord.X)` (note: y is dy normal,
     x is `-dx`, which rotates the vector 180° — i.e. the seed direction is
     *away from* the threat, not toward it).
   - `seed = ((angle_int >> 12) + 1) >> 1` — quantize the binary-radian angle to
     the 16 sub-bins, round to nearest of the 8 bins.
   - **Add `Random__RandomRanged(0, 2) - 1` ∈ {-1, 0, +1}**, so the seed direction
     can wobble ±1 from the away-from-threat ideal. (Compare: variant C uses ±2.)
2. Probe loop (`aiStack_3c[0] = 0; aiStack_3c[0] < 8`):
   - `dir_idx = (loop_i + seed) & 7` — clockwise rotation through Table 1.
   - candidate = `current_cell + Table1[dir_idx]`.
   - **Reject** via `Is_Cell_In_Playfield` then `vtable+0x1AC` (`Can_Enter_Cell`,
     param 4 = the dir_idx itself, param 5 = `CellClass::Get_Effective_Height(0)`).
     Note that the Can_Enter_Cell call passes the *direction index* as param 4 —
     this is used by the cliff/ramp-aware passability check, so **direction matters
     for accept/reject** even when the destination cell is identical.
   - On the FIRST passing candidate, record into `uStack_4` as the *fallback* cell
     (the "any direction works" answer).
   - Then run the **height/footprint match check** via `FUN_006D6410` — if the
     height-corrected cell at `(cx*256+128, cy*256+128, +z_bias)` equals the input
     cell AND the input cell does NOT have bridge bit `0x100` set, store this
     into `uStack_48` as the *preferred* cell and **break** the loop.
   - `z_bias` in this call is `iStack_1C = DAT_00B1D0B8 * (-(uint)((uVar6 & 0xff) + Level != 0) & 4)`
     — i.e. either 0 or `4 * DAT_00B1D0B8`. `DAT_00B1D0B8` is `0x80` (128 leptons,
     half a cell). So z_bias is +512 leptons on a non-zero-Level cell. This compensates
     for terrain height when projecting to the screen-aligned cell.
3. After the loop: prefer `uStack_48` (height-matched), fall back to `uStack_4` (first
   passable), else give up. Then issue `SetMission(2)` (Move) and `SetDestination`.

**Tiny details that matter:**
- The probe loop iterates exactly **8 times** every call. There is no early exit on
  bad seed direction — even if seed=0 and loop_i=0 fails passability, the loop
  continues with loop_i=1 testing seed+1. (Compare: Find_Nearby_Passable_Cell can
  expand its radius dynamically.)
- The break on a height-matched candidate happens *inside* the for-loop body, not
  as a `for` condition — so `loop_i` still increments before the break is taken.
  This doesn't affect correctness but matters for any re-impl that tries to be
  clever with the loop control.
- **The `+Random(0,2)-1` seed wobble is taken even when the seed is already perfect**
  — the random call is unconditional. Two calls on the same tick with the same
  inputs produce different results (RNG is consumed twice).
- **The `else` branch on the early-out (around `LAB_00744076`)** silently leaves
  the unit's mission untouched — meaning a scatter that finds no valid cell does
  NOT set `MISSION_GUARD` or `MISSION_AREA_GUARD`. The unit keeps doing whatever
  it was already doing. This is a real behavioral detail: a fully surrounded unit
  responds to a scatter trigger by doing nothing visible.
- The `param_1[0xad] != 0` test (target exists) combined with `Random__RandomRanged(1,4) != 1`
  is a **75%-skip-the-scatter** gate when the unit already has a target. So 3 out of
  4 Random rolls cause the scatter call to be a no-op when the unit is engaged.

### 3.3 Variant C — `InfantryClass::Scatter` (0x51D0D0)

Same 8-direction shape as B, but with these differences (verified in this
report's decompilation):

| Aspect | Variant B (Unit) | Variant C (Infantry) |
|--------|------------------|----------------------|
| Random offset on seed | `Random(0, 2) - 1` ∈ {-1, 0, +1} | `Random(0, 4) - 2` ∈ {-2, -1, 0, +1, +2} |
| Mission post-scatter | `SetMission(2)` (Move) | `SetMission(2)` then `Locomotor::Force_Move()` |
| Crawl-state early-out | none | sequences `0x1B, 0x1C, 0x1D, 0x1E` block scatter for player-owned (and force-stop animation if `force` and `param_4` both set) |
| Locomotor-busy override | none | downgrades `force` to false if `Locomotor::IsBusy()` |
| RetaliationOverlap pre-check (`unaff_retaddr` table at `&DAT_007EAF7C`) | none | per-mission scatter-allowed table indexed by sequence number; if 0, skip entirely |
| Z-bias (height comp) | `0` or `4·0x80` | `0` or `4·0x80` (same, via `DAT_00A8F240`) |
| Direction passed to Can_Enter_Cell | dir_idx | `(iVar3 >> 8 & 0xFFFF) & 7` from `Get_Effective_Height` return — **subtly different**: variant C masks 0xFFFF first, then & 7. The result is identical for the inputs that occur, but the bit-level expression differs. |

The **±2 RNG range** is the player-visible parity hazard: on a single scatter
event triggered for both an infantry and a vehicle on the same cell, infantry
deviates further from the away-from-threat ideal direction. This is one of the
"feels random but isn't" sources the user flagged.

### 3.4 Variant D — `FootClass::Find_Nearest_Dock` (0x4DFCB0)

Misnamed for this report's purpose — despite the name, it is **not** the
harvester-finds-refinery routine. It is the auto-target picker for any
TechnoClass that has been told "go enter a building" without a specific
target, and the candidate test is `BuildingClass::CanDock` (`0x457CE0`)
which gates on `Type+0x157B = CanBeOccupied` (garrison) and the
infantry's `Occupier`/`C4` flags. Real harvesters use Mission_Harvest's
own refinery search (a separate function).

**Algorithm:**

```
best_dist = INT_MAX           // local_34 = 0x7fffffff
best_b = NULL                 // local_30
i = owner.Buildings.count - 1 // iterates BACKWARD
while i >= 0:
    b = owner.Buildings[i]
    delta = my_coord - b.GetCoords()  // 3-D vector
    dist_f = Sqrt_Approx(dx² + dy² + dz²)
    dist = ftol(dist_f)        // truncate to int
    if dist < best_dist:       // STRICT <
        if CanDock(b, this):
            best_dist = dist
            best_b = b
    i -= 1
if best_b == NULL: return 0
this+0x690 = 1
SetDestination(best_b, 1)
SetMission(MISSION_ENTER, 1)
return 1
```

**Tiny details (parity-load-bearing):**

- **Backward iteration + strict `<` ⇒ ties favor the higher-indexed building.**
  If two buildings are at identical Euclidean distance (after `ftol`), the one
  added LATER to the owner's building list wins. Build order = creation order,
  so a refinery built later wins ties over an older one of the same distance.
- **CanDock is called only when distance is already a new best.** This is an
  optimization that produces a behavioral artifact: if the closest building is
  not dockable (e.g., its occupant slots are full), the function does NOT keep
  looking for a *farther* dockable building — it only keeps looking for one
  that's *both farther* and dockable. Since `local_30` only updates when CanDock
  passes, the next iteration's `iVar5 < local_34` is comparing against the last
  *dockable* best, not the geometric best. So the algorithm correctly finds the
  nearest dockable, just with a layered comparison.
  Wait — re-reading: `local_34` is updated ONLY inside the CanDock-true branch,
  so `local_34` is always the distance of the best *dockable* building. The
  CanDock test itself happens before `local_34` updates. ✓ correct.
- **Distance is 3-D, not 2-D.** This includes the Z component. For two refineries
  on different bridges or at different terrain heights, the Z difference can flip
  the choice. Most other find-nearby variants use 2-D ortho.
- **`Sqrt_Approx` is a custom approximation (at `0x4CAC40`)**, not `sqrtf`. Re-impl
  must use the same approximation or the truncated int distances will diverge for
  large `dx²+dy²+dz²` values. (For typical map distances under ~20 cells = 5120 leptons,
  the approximation matches IEEE sqrt to within ULP, but at 100+ cells it can drift.)
- The **search has no early exit on "good enough"**. Every owner-building is tested.
  For 8-player full-base scenarios this is `O(N)` per dispatch, acceptable; for the
  20k-unit / 30-player scale target it would be a hot-path concern (but Find_Nearest_Dock
  fires on *individual unit-into-building* events, not per-tick, so still fine).

### 3.5 Variant E — Foundation Perimeter Scan (`GetDockCellForObject`, 0x44EFB0)

This runs when no `Type->ExitList` is set, OR the building is `Hospital=yes`.

**Iteration order (verified):**

```
w = GetFoundationWidth()
h = GetFoundationHeight()
o = building.MapCell  // (local_18, sStack_16) = top-left

// X-axis sweep: x from -1 to w (inclusive, w+2 iterations)
for x in -1..=w:
    test cell (o.x + x, o.y + h)        // BOTTOM row first
    test cell (o.x + x, o.y - 1)        // TOP row second

// Y-axis sweep: y from -1 to h (inclusive, h+2 iterations)
for y in -1..=h:
    test cell (o.x + w, o.y + y)        // RIGHT column first
    test cell (o.x - 1, o.y + y)        // LEFT column second
```

Each `test cell` does:
- `Cell_in_bounds_check` (map clipping)
- `MapClass::Get_CellClass` → pointer
- Unit's `Can_Enter_Cell` via `vtable+0x1AC` with `(cell, -1, -1, 0, 1)` — the
  trailing `1` is the strict-occupancy flag.
- On first pass, return that cell.

**Tiny details:**
- The corners (e.g. `(o.x-1, o.y-1)`) are tested **twice**: once during the
  X-sweep at `x=-1, top` and again during the Y-sweep at `y=-1, left`. Both
  tests produce identical results, so this is wasted work but harmless.
- The **bottom row is preferred over top** (south before north) at any `x`, and
  the **right column is preferred over left** (east before west) at any `y`. This
  produces a SE-bias on the perimeter walk.
- The X-sweep happens BEFORE the Y-sweep, so for an L-shaped non-foundation
  conflict, the function prefers an exit on the long side (foundation width
  matters for which axis is iterated first, but per-axis is fixed: X then Y).
- The "barracks special-case" tests at the top of `GetDockCellForObject` (GDIBarracks
  → `(+1,+2)`, NODBarracks → `(+2,+2)`, YuriBarracks → `(+2,+1)`) run BEFORE this
  scan. So infantry from a barracks bypass the perimeter scan entirely if the
  hardcoded exit is passable.

### 3.6 Variant F — `FUN_005060B0` (defense / non-naval exit)

Used for base-defense placement and similar where the building has an `ExitList`
but the standard cells fail. Two phases:

**Phase 1 — Sort exit-list candidates.** Iterate the `ExitList` at offset `+0x5724`
(count at `+0x5730`). For each entry, compute:
- A weighted "distance" = `index + (sqrt_distance_to_origin × 1000)`, where
  `sqrt_distance` uses `Sqrt_Approx` and `Math__ftol`.
- The `+ 1000`-scale on the distance is critical: it makes index a fine-grained
  tie-break (positions 0..999 within the same outer distance band sort by ExitList
  order). This matches the qsort comparator at `LAB_005108F0` (referenced inline,
  not a separate function — it compares the int field at offset 0 of each entry).

The sort uses `FUN_007C8B48` which is qsort-style with the comparator at the
above label. Stable sort is **not** guaranteed, but because the keys include
the index in the low bits, ties at the int level are impossible.

**Phase 2 — For each sorted candidate, 8-direction probe + 3-step push.**

```
seed_dir = atan2 → angle index, NO random offset (compare: variants B/C add ±)
for outer_iter in 0..2:        // 2 passes: bridge-aware then bridge-blind
  for each_sorted_candidate:
    // first pass: original direction; second pass: opposite (seed - 4 & 7)
    target_cell = candidate + Table1[seed_dir]   // only on second pass: also DirectionOffsets
    // 3-step push along Table 2 in the same direction:
    for step in 0..3:
      test target_cell against CheckOccupancy + footprint
      if accepted: return target_cell
      target_cell = target_cell + Table2[seed_dir]
```

**Tiny details (verified):**
- The outer loop runs at most **2** times (`local_d4 < 2`). If both passes fail,
  return INVALID.
- The inner step loop runs **exactly 3** times per direction. So each candidate gets
  3 chances on each of the 2 passes = 6 cells tested per ExitList entry per outer
  iteration.
- The `iVar5` in the rect check `iStack_18 = iStack_2c + iVar5 * 2` expands the
  footprint by `2 * iVar5` cells (where `iVar5` defaults to `Rules+0x1460`, with +1
  if `Type+0x1765` or `Type[0x55E]` is set — the "spacious base defense" mode).
  This means a defense placed near other buildings has a configurable buffer band.
- The function clones `Find_Nearby_Passable_Cell`-shaped behavior for the Naval branch
  (`Type+0xCCE != 0`): it bypasses Phase 1+2 entirely and calls `Find_Nearby_Passable_Cell`
  with `(W+2, H+2)` foundation, `SpeedType=5` (Float), and a target-distance check
  against `Rules+0xE0C` (max build distance). So Naval Yards inherit Variant A's spiral.

### 3.7 Variant G — `BuildingTypeClass::CanBePlacedAt` (0x45EE70)

This is an object/scatter validator used only by three verified callers:
`UnitClass::Deploy` (MCV/unit deploy), `FUN_006ED4D0` (AI deploy scheduler), and
`BuildingClass::ExitObject_Main` (factory object exit). It is **not** the normal
human sidebar-ready placement commit used by `HouseClass::Place_Production`;
that path reaches `BuildingClass::Unlimbo` and the
`BuildingClass::Can_Enter_Cell` / building-type `+0xA8` /
`Cell_passability_building_placement` chain. Cursor cell tint is also computed
by a different routine. Decompiled for this report.

**Algorithm:**

```
foundation_list = GetFoundation(1)   // vtable+0x90, returns short[][2]
                                     // terminated by {0x7FFF, 0x7FFF}
flag_can_overlap = false             // (uint)unaff_EBX >> 0x18

for each (dx, dy) in foundation_list:
    cell = (cursor.x + dx, cursor.y + dy)
    if !Cell_in_bounds(cell): continue
    if cell.OverlayType != -1:
        if not (this is RulesClass+0x87C and overlay == 2):
            return BLOCKED   // 2
    obj = cell.FirstObject (cell+0xE4)
    if obj is null: continue
    if obj.RTTI == 0x24 (TerrainObject?): return BLOCKED
    if obj.flags bit 0 (foundation flag) is clear: continue
    if obj.RTTI == 6 (Building):
        if !CanAcceptUpgrade(this, cursor): return BLOCKED
        else: flag_can_overlap = true
    else:
        if !IsAlliedWith(obj.Owner): return BLOCKED
        if obj.flags bit 2 is clear:    return BLOCKED
        flag_can_overlap = true
        // Trigger scatter on the obj — see below
        if obj.NavTarget == cell: scatter
        Scatter_Objects(obj.Cell, ..., DAT_0089C8D0)

if flag_can_overlap: return OK_OVERLAPS  // 1
else:                return OK_FREE      // 0
```

**Tiny details:**
- **No outward search.** This is a footprint-iteration only. The "spiral" feeling
  during placement preview comes from the cursor moving, not from any code-side
  search.
- **Scatter side-effect at placement-commit, NOT during preview.** Verified
  via xrefs (see RQ5 in §10): `CanBePlacedAt` has only three callers
  — `UnitClass::Deploy`, `FUN_006ED4D0` (AI deploy scheduler), and
  `BuildingClass::ExitObject_Main`. None of them is the build-cursor
  hover handler. So the scatter call fires only on the actual placement
  tick, not while the cursor moves over a busy area. (Build-cursor preview
  uses a different validation routine; cell tint comes from there.) A
  re-impl that omits the scatter on the placement tick produces a
  "I just placed and the unit didn't move" UX bug.
- **Foundation walk order is whatever the INI says.** `GetFoundation(1)` returns
  the table at the building's `+0xEF0` cookie indexed into `g_FoundationData`. The
  short pairs are stored in INI (`Foundation=NxN` macro-expanded to coord list).
  Scan order matches that INI order — typically row-major top-to-bottom, but for
  irregular foundations it is the order Westwood specified.
- **Three different return values: 0, 1, 2.** `0` = clear (no overlaps), `1` = OK
  but overlaps something the placement may consume (like an upgrade slot), `2` =
  blocked. Cursor color depends on this distinction. A re-impl returning a bool
  loses the distinction between "place sound + no overlap" and "place sound + overlap-is-fine".

---

## 4. Cross-Variant Comparison Matrix

| Property | A: Find_Nearby | B: Unit Scatter | C: Inf Scatter | D: Find_Nearest_Dock | E: Foundation Perim | F: Defense Exit | G: CanBePlacedAt |
|----------|---------------|-----------------|----------------|----------------------|---------------------|-----------------|------------------|
| Pattern | Diamond-ring spiral | 8-dir CW | 8-dir CW | Linear (backward over building list) | Fixed perimeter walk | Sorted ExitList + 8-dir probe + 3-step push | Foundation-cell walk (INI order) |
| Max cells tested | 32-radius diamond ≤ 4N² ≈ 4096 (typical 200-500) | 8 | 8 | (= owner.building count, typ. 10-50) | 2(W+2) + 2(H+2) (typ. 8-20) | (ExitList × 2 × 3) ≈ 30-60 | OccupyList size (typ. 4-9) |
| Tie-break | Ring order: NW corner → CW perimeter; then `g_FrameCounter % count` | First-in-loop wins (loop_i = 0..7 from seed) | First-in-loop wins | **Higher index wins ties** (backward iter + strict `<`) | First passable in fixed order: bottom-x, top-x, right-y, left-y | Phase 1 sort by `idx + dist*1000`; then first passable in 8-dir probe (CW from seed) | First failure short-circuits; INI order on success |
| RNG hookup | `g_FrameCounter` (deterministic counter, NOT Random) | `Random(0,2)-1` on seed | `Random(0,4)-2` on seed | None | None | None | None |
| Includes Z (3-D)? | No (2-D + height-correction lookup) | No (2-D + Z-bias offset) | No (2-D + Z-bias offset) | **Yes** (true 3-D distance) | No | No | No |
| Distance metric | `Sqrt_Approx` of `dx²+dy²` truncated | N/A (no distance comparison) | N/A | `Sqrt_Approx` of `dx²+dy²+dz²` truncated | N/A | `Sqrt_Approx` × 1000 + index | N/A |
| Edge of map handling | `IsOnScreen` + bounds index check | `Is_Cell_In_Playfield` | `Is_Cell_In_Playfield` | None (always uses building's own coords) | `Cell_in_bounds_check` | `Cell_in_bounds_check` | `Cell_in_bounds_check` |
| Bridge cells | param_13 toggle + `+0x100` flag | Implicit via `Can_Enter_Cell` flags | Implicit via `Can_Enter_Cell` flags | N/A | Implicit via `Can_Enter_Cell` strict | Phase 2 outer loop has bridge retry via `bVar15` | Implicit via `+0xE4` first-obj check |
| Returns on failure | `{0,0}` (NULL_CELL = `DAT_00ABD480`) | Silent no-op (mission unchanged) | Silent no-op (mission unchanged) | sets `+0x690 = 0`, returns 0 | `INVALID_CELL` (`DAT_0089C818`) | `INVALID_CELL` | `0` (return code) |
| Side effects during search | None | None | Force_Move at end | Sets `+0x690`, calls SetDestination + SetMission | None | None | **Scatter_Objects called on conflicting friendly unit** |

---

## 5. Quantization & RNG Detail Cross-Reference

The seed-direction calculation is the same in B, C, and F:

```
angle_int = Math__ftol(atan2(dy, -dx))   // y points down on screen
seed_4bit = (angle_int >> 12) & 0xF      // 16 bins → 4-bit quadrant
seed_3bit = ((seed_4bit + 1) >> 1) & 7   // round to nearest 8-bin (0..7)
```

Then variants add the random:
- B (unit): `final = (seed_3bit + Random(0,2) - 1) & 7` → ±1 wobble
- C (infantry): `final = (seed_3bit + Random(0,4) - 2) & 7` → ±2 wobble
- F (defense exit): `final = seed_3bit` (no wobble)

**Why the wobble difference matters:** in a tight pack of mixed unit/infantry,
the same scatter trigger sends infantry on a wider angular distribution. Over
many ticks this produces visibly different post-scatter formations. A re-impl
that uses the same wobble for both produces visually-correct individual units
but a noticeably-different aggregate flow, which is exactly the "feels random
but isn't" parity bug class the user flagged.

**The atan2 input convention `(dy, -dx)`** rotates the away-from-threat direction
by 180°, so `seed` is the direction pointing *from threat toward me*. Then the
scan starts at that direction and rotates CW. This means the **first probe is
typically the cell directly opposite the threat** — exactly the cell the unit
"should" flee to, except the random wobble can shift it ±1 (or ±2 for infantry)
clockwise/counter-clockwise.

---

## 6. The "Same Tick, Different Variant" Hazards

These are the parity-relevant interactions between variants when multiple
fire on the same frame:

### 6.1 Frame-counter aliasing in Variant A

`Find_Nearby_Passable_Cell` uses `g_CurrentFrameCounter % candidate_count` for
the no-target tie-break. **Two A-calls on the same tick with the same candidate
count return the same index.** So if MCV deploy and chrono teleport landing
both fire on the same frame and both have, say, 3 candidates, both pick
`frame % 3 = 0` — i.e., both go to the spatially-first candidate. This is a
**deterministic-but-not-uniform** behavior.

Re-impl note: do NOT replace `g_FrameCounter % count` with a real RNG, even
if the call site looks like it wants randomness. Lockstep correctness depends
on this exact formula.

### 6.2 Random-counter advances differ between B and C

`UnitClass::Scatter` consumes 1 RNG roll (the seed wobble). `InfantryClass::Scatter`
consumes 1 RNG roll too. But the gates BEFORE the RNG call differ:
- B has a `Random(1,4)` early-out gate (75%-skip-when-targeted); this consumes
  a roll even on the skip path.
- C has no such gate; it goes straight to the seed roll.

So a tick that scatters one tank + one infantry consumes potentially 2 rolls
(B early-out + C seed) or 2 rolls (B seed + C seed) depending on whether the
tank had a target. This RNG-cursor drift is invisible per call but accumulates.

### 6.3 Variant D's CanDock can fire Variant E

`Find_Nearest_Dock` → `BuildingClass::CanDock`. CanDock for some building types
consults the building's `GetDockCellForObject` (vtable+0x4D4 inside CanDock)
which is variant E. So a single Find_Nearest_Dock call may invoke 8-20 inner
cell tests per candidate building, multiplying the apparent O(N) into O(N×P)
where P is the perimeter size. For a 30-player large base, this is the kind
of cost that compounds.

### 6.4 Variant G's scatter loops back to Variant B/C — at placement-commit only

Building placement-commit (variant G) calls `CellClass::Scatter_Objects` on
any friendly unit on the foundation. `Scatter_Objects` dispatches to per-class
Scatter (variant B for vehicles, variant C for infantry). So a single placement
that overlaps N friendly units fires N variants B/C calls — and each consumes
1 RNG roll.

**Resolved (post-investigation, see RQ5 §10):** the call only fires at
placement-commit (`UnitClass::Deploy`, AI deploy scheduler, build-queue exit),
NOT during cursor preview. The original draft warned about per-cursor-tick
RNG drift, which is incorrect — cursor-tick advancement of RNG is not a
concern.

**Real parity hazard that remains:** the placement tick must be aligned across
clients in lockstep multiplayer. The placement event itself is replicated via
Network::Place_Production — already lockstep-correct. But the order in which
the foundation cells are walked (and thus which overlapping unit gets
scattered FIRST) must match `Foundation=` INI order exactly, since each scatter
consumes one RNG roll and shifts the seed for the next.

### 6.5 Variant F's stable-sort assumption

Phase 1 of variant F sorts by `index + 1000 × dist`. Because the index is unique
per ExitList entry, ties at the int comparison level are impossible — so the
sort is effectively stable even though the underlying qsort is not stable in
the general case. A re-impl that uses an unstable sort with a different tie-break
(e.g., raw distance only) WILL pick different exit cells.

---

## 7. Hot-Path Frequency Reference

To frame the player-visibility for each variant:

| Variant | Trigger frequency in normal play | Player-visible? |
|---------|-----------------------------------|-----------------|
| A: Find_Nearby_Passable_Cell | Many times per minute (every scatter, every harvester reposition, every chrono landing, every MCV deploy attempt) | Yes — "where exactly does the unit appear?" |
| B: Unit Scatter | Every time a vehicle path-blocks another (every move command in a base produces several) | Yes — visible per-unit movement |
| C: Infantry Scatter | Every time infantry path-blocks (very frequent in boxed-in infantry) | Yes — visible per-unit movement |
| D: Find_Nearest_Dock | Once per "enter any" command (rare; infantry garrison auto-target) | Yes — picks which building gets the engineer |
| E: Foundation Perimeter Scan | Once per building production exit when ExitList missing (mostly hospitals, some special buildings) | Yes — sets where unit appears |
| F: Defense Exit | Once per defense placement attempt + once per defense production exit | Mostly placement-validation, rare runtime |
| G: CanBePlacedAt | Once per placement-commit event (MCV deploy, building queue completes, AI deploy schedule fires) | Yes — placement success/fail and one-shot scatter on overlapping unit |

A is by far the highest-frequency variant. B and C are high in busy bases.
G is one-shot per placement event (corrected post-investigation; see RQ5 in §10).

---

## 8. Per-Variant Verification

Each variant was decompiled or re-decompiled live for this report. Confidence
is HIGH for everything in section 3 unless explicitly noted.

| Variant | Address | Re-decompiled here? | Lines decompiled |
|---------|---------|---------------------|------------------|
| A | `0x56DC20` | No (relied on existing high-confidence report) | — |
| B | `0x743A50` | Yes | 200+ |
| C | `0x51D0D0` | Yes | 230+ |
| D | `0x4DFCB0` | Yes | 50 (full body) |
| E | `0x44EFB0` | Yes | 200+ (full body) |
| F | `0x5060B0` | Yes | 300+ (full body, both branches) |
| G | `0x45EE70` | Yes | 80 (full body) |
| Direction tables | `0x89F688`, `0x89F6A8` | Yes (read raw memory + traced init in F) | 32 + 32 bytes |
| `BuildingClass::CanDock` (used by D) | `0x457CE0` | Yes | 50 |
| `CellClass::Scatter_Objects` (used by G+other) | `0x481670` | Yes | 80 |
| `BuildingClass::ClearBibArea` (FUN_00449540) | `0x449540` | Yes | 60 |

The only LOW-confidence claim in this report is the inferred semantics of
`+0x6E1`, `+0x6E2` (deploying-flag/unloading-flag in B's gate chain) — these
follow naming from existing UnitClass docs but I did not decompile their
write sites here.

---

## 9. INI Keys That Affect Search Behavior

| Key | Section | Default | Affects |
|-----|---------|---------|---------|
| `[CombatDamage] PlayerScatter` | rules | no | Forces dispatch through B/C even without elite/IQ trigger |
| `[IQ] Scatter` | rules | 2 | AI-only gate before B/C; humans always = 0 |
| `[General] FreeUnit` | rules | (per-house) | Whether refinery spawn uses A or static formula |
| `[Mission name] Scatter` | rules | varies | Per-mission allow-scatter bool gates B/C entirely |
| `Foundation=` | art | — | Variant G's iteration order |
| `ExitList=` | art (via Foundation) | — | Variant E's exit-list / order |
| `DockingOffset0..N` | art | — | Variant E's dock pad selection (multi-pad helipads) |
| `QueueingCell` | art | (0,0) | Refinery harvester wait cell — independent from any spiral, just a fixed offset |
| `[General] MaxBaseDistance` | rules | — (`Rules+0xE0C`) | Variant F-Naval distance cap (in cells × 256) |

---

## 10. Resolved Questions (post-investigation closure)

The five LOW-confidence items in the initial draft are all resolved.
Recorded here for the audit trail.

### RQ1 — `g_CurrentFrameCounter` advance timing

**Resolved: simulation tick counter; frozen during pause.**

- Address: `0x00A8ED84`.
- Two write sites: `Main_Game` (`0x52DA08`, init/reset) and `Main_Tick`
  (`0x55DE81`, the per-tick increment).
- Increment in `Main_Tick` is gated by FOUR pause/sync flags (`DAT_00A83D49`,
  `DAT_00A8ECD0`, `DAT_008B41C0`, `DAT_00A83D48`). All zero ⇒ tick advances
  and `LogicClass::PerTickUpdate` runs. Any non-zero ⇒ early exit, counter
  unchanged, no logic.
- **Implication for Variant A**: the frame-modulo "random" tie-break is fully
  deterministic and lockstep-safe. Pause does NOT advance the counter, so a
  paused-then-resumed game produces identical bit-for-bit picks. A re-impl
  using a sim-tick counter (NOT wall-clock, NOT render-frame) matches gamemd.exe.

### RQ2 — Sqrt_Approx accuracy

**Resolved: ~12-bit mantissa precision via 8192-entry lookup; deterministic.**

- Function at `0x4CAC40`. Decompilation shows: cast `double → float`, extract
  IEEE float exponent and mantissa, look up sqrt mantissa in 8192-entry table
  at `DAT_008650BC`, reconstruct the result.
- Indexing: `(mantissa >> 10)` gives 13-bit table index; the `(exp & 1)`
  parity bit shifts in `0x800000` (selects upper or lower mantissa half).
- Special cases: input == 0.0 → return 0.0; input < 0 → multiply by
  `_DAT_007E4900` (= -1.0f) and proceed (so `Sqrt_Approx(-x) = sqrt(|x|)`,
  NOT NaN). None of the Find-Nearest variants pass negative input.
- Precision: ~12 bits of mantissa = error ~1 in 4096 of the magnitude. For
  map distances up to ~120 cells (30720 leptons), well within float32 range
  and the truncated `ftol` output is bit-stable across calls.
- **Two truly equidistant points produce identical `ftol(Sqrt_Approx(...))`
  outputs**, so Variant D's tie-break (backward-iter strict `<`) is reliable
  even at long range. A re-impl using IEEE `f32::sqrt` would produce *the same*
  truncated int outputs for distances under ~10000 cells (the precision floor
  of the lookup is below float32's ulp at typical distances) — so for parity
  on integer-truncated outputs, IEEE sqrt is OK. The "do NOT use f32::sqrt"
  warning in §12 is only relevant if the un-truncated float result is
  consumed elsewhere (it isn't, in any Find-Nearest variant).

### RQ3 — `Rules+0x1460` is `[General] AIBaseSpacing`

**Resolved: `AIBaseSpacing`, default = 1 in `rulesmd.ini`.**

- Verified via existing report `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md`
  (entry T13 B13) and `rules.ini` line 2602: `AIBaseSpacing=1`.
- Variant F uses this as `iVar5 = Rules+0x1460` (= 1 by default), with a +1
  boost when `Type+0x1765` (`WantsExtraSpace=yes`) or `Type[0x55E]` is set. So
  the buffer band is "1 cell" by default, "2 cells" with WantsExtraSpace.
- The `rules.ini` comment at line 8451/8489 confirms: *"will look for a
  space AIBaseSpacing+1 when the computer places, but will settle for
  AIBaseSpacing"*. The +1 fallback is exactly what F's `bVar15` outer loop
  implements.

### RQ4 — `DAT_007EAF7C` per-mission scatter table

**Resolved: hardcoded link-time constant array, 4 bytes per entry, indexed
by sequence index. First byte is "scatter allowed".**

Raw bytes (read from live image at `0x7EAF7C`, 32 entries × 4 bytes = 128 B):

| Idx | Bytes | byte0 | Likely sequence |
|-----|-------|-------|-----------------|
| 0x00 | `01 00 00 00` | ✓ allow | Ready |
| 0x01 | `01 00 00 00` | ✓ | Guard |
| 0x02 | `01 00 00 06` | ✓ | Walk |
| 0x03 | `01 01 01 03` | ✓ | Firing primary |
| 0x04 | `01 00 00 01` | ✓ | Firing secondary |
| 0x05 | `00 01 00 01` | ✗ block | Hit |
| 0x06 | `01 01 01 01` | ✓ | Lying-down |
| 0x07 | `00 00 00 01` | ✗ | Lying-firing |
| 0x08 | `01 00 00 01` | ✓ | Crawling-base |
| 0x09 | `01 00 00 03` | ✓ | Up |
| 0x0A | `01 00 00 03` | ✓ | Down |
| 0x0B | `00 00 00 01` | ✗ | Cheer |
| 0x0C–0x0F | `00 00 00 01` ×4 | ✗ ×4 | Dies-1 / Dies-2 / Dies-3 / Dead |
| 0x10 | `01 00 00 03` | ✓ | Idle1 |
| 0x11 | `01 01 01 01` | ✓ | Idle2 |
| 0x12 | `01 00 00 03` | ✓ | Paradrop |
| 0x13 | `01 00 00 03` | ✓ | Paradrop secondary |
| 0x14 | `00 00 00 01` | ✗ | Tumble |
| 0x15 | `00 00 00 01` | ✗ | Sinking |
| 0x16 | `01 00 00 01` | ✓ | DeployAttack? |
| 0x17 | `01 01 00 02` | ✓ | Panic |
| 0x18 | `01 01 00 01` | ✓ | Sleep |
| 0x19 | `01 01 00 01` | ✓ | Crash |
| 0x1A | `01 01 00 01` | ✓ | Wake |
| **0x1B** | **`00 00 00 01`** | **✗** | **Crawl-prone-1** |
| 0x1C | `01 00 00 01` | ✓ | Crawl-prone-2 |
| 0x1D | `01 00 00 01` | ✓ | Crawl-prone-3 |
| 0x1E | `01 00 00 01` | ✓ | Crawl-prone-4 |
| 0x1F | `00 00 00 01` | ✓ (dead byte — see note) | Tumble-final |

**Note on 0x1F (and seq=−1):** the table byte is `00`, but the runtime check is
`if ((iVar3 != -1) && (iVar3 != 0x1f) && ((&DAT_007EAF7C)[iVar3 * 4] == 0)) return;`
— meaning the function **explicitly skips the table for indices `-1` and `0x1F`**.
So at runtime, sequence 0x1F is **allowed** (scatter falls through to the rest
of the function), regardless of what the table byte says. The `00` at offset
0x1F * 4 is a dead value because of the skip-condition. Re-impl: hardcode the
exemption for `-1` and `0x1F`; do NOT consult the table for those two indices.

(InfantryClass sequence labels above are inferred from typical Westwood SHP
sequence enums; idx → name mapping is HIGH-confidence for 0x00–0x07 and
0x1B–0x1E since those align with explicit code paths, MEDIUM for the others.)

**Tiny-detail correction to my draft**: I stated 0x1B–0x1E ALL block scatter
via the table. The dump shows **only 0x1B** has `byte0=0`; 0x1C–0x1E have
`byte0=1` (table allows). The early-out in `InfantryClass::Scatter`
**explicitly tests `seq == 0x1B || 0x1C || 0x1D || 0x1E`** and either
force-stops the prone animation (when forced + param_4) or returns without
scattering (when player-owned). So:

- 0x1B is blocked at BOTH layers (table + explicit early-out).
- 0x1C / 0x1D / 0x1E: table allows; the explicit code-check still blocks them.

**Re-impl:** implement BOTH gates. The table is a fast-path; the explicit
sequence-index check in `InfantryClass::Scatter` is the authoritative source
for 0x1C–0x1E. Re-impl using only the table would let infantry scatter
mid-prone-animation — visible parity bug.

The table is **not** INI-driven; it is a static C array compiled into the
binary at link time. Re-impl should hardcode these 128 bytes.

### RQ5 — `CanBePlacedAt` callers (cursor vs place-confirm)

**Resolved: only THREE callers, none of them per-cursor-tick.**

| Caller | Address | Trigger |
|--------|---------|---------|
| `UnitClass::Deploy` | `0x739536` | MCV deploy command actually executes |
| `FUN_006ED4D0` (AI deploy scheduler) | `0x6ED6E0` | AI per-tick deploy attempt for owner's units |
| `BuildingClass::ExitObject_Main` | `0x445210` | Production build queue completes and places |

**Build-cursor preview does NOT call `CanBePlacedAt`.** Cursor cell-tint is
computed by a different cell-validation routine (out of scope here, invoked
from DisplayClass per cursor tick). The scatter side-effect inside
`CanBePlacedAt` therefore fires only at placement-commit moment.

**Correction to my draft (§3.7 was overstated)**: The original draft warned
"every cursor tick during build mode" fires Scatter_Objects and "dragging
the build cursor across a busy area advances the RNG." Both wrong. The
scatter side-effect is a **one-shot per placement event**, not a per-tick
stream. Lockstep replay safety still requires placement-tick alignment but
no cursor-input replay concern.

The §7 frequency table for Variant G has been updated below to reflect this.

---

## 11. NOT spiral / search variants — clarifications

For completeness, these systems are often confused with the family above but
do NOT use any spiral / outward search:

- **`InfantryClass::Mission_Enter` (0x5196A0)** — garrison entry. Just an
  equality check at the unit's current cell vs. the destination building's
  foundation. The infantry walked there via standard pathfinding. The
  "find which cell to walk to" was Variant E (the building's exit cell, used
  in reverse).
- **`BuildingClass::ClearBibArea` (FUN_00449540 at 0x449540)** — WF bib clear.
  Uses `CellClass::Find_Nearest_Object` + `Scatter_Objects` with a fixed cell
  computed from `Type->ExitList[0x28]`. No outward search; just a fixed cell
  + retry up to 8 times. The retries are TIME retries, not spatial — they
  give scattered units time to actually move out.
- **`UnitClass::Mission_Harvest` ore-cell selection** — uses a separate
  patch-coverage scan, not in this family. Documented elsewhere.
- **AI building-placement (cell selection for AI bases)** — uses a totally
  different system (`HouseClass::Place_Object`) that scans available zones
  and tries Foundation::CanBePlacedAt at each. Out of scope here.

---

## 12. Re-Implementation Guidance — Summary

When porting these to Rust, the eight variants must remain **distinct**.
Tempting "unifications" that break parity:

- ❌ Replacing `g_FrameCounter % count` with a real RNG in Variant A.
- ❌ Using a single direction table (Table 1 only) for both the probe and
  the push step in Variant F.
- ❌ Using a 2-D distance for Variant D (drops Z, changes ties).
- ❌ Sharing the seed wobble between B and C (drops the ±2 vs ±1 distinction).
- ❌ Forward-iterating the building list in Variant D (flips the tie-break).
- ❌ Iterating Y before X in Variant E (changes which corner cell wins ties).
- ⚠️ Using `f32::sqrt` instead of `Sqrt_Approx`: SAFE for the variants here
  because all sites truncate via `ftol` and table precision is below float32
  ulp at typical distances. Becomes UNSAFE if any future code path consumes
  the un-truncated float — see RQ2 in §10.
- ❌ Skipping the Variant G scatter side-effect.
- ❌ Treating Variant G's three-state return (`0`/`1`/`2`) as a `bool`.

Each of the above produces a behavior that compiles and runs, plays a single
match without crashing, but produces visibly-different outcomes from
gamemd.exe in repeatable scenarios — exactly the parity-rot pattern the
project's CLAUDE.md warns about.

---

## Sources

### Decompiled in Ghidra MCP for this report

- `0x4DFCB0` `FootClass::Find_Nearest_Dock` (full body)
- `0x44EFB0` `BuildingClass::GetDockCellForObject` (full body)
- `0x5060B0` `FUN_005060B0` (defense / naval exit) (full body, both branches)
- `0x500200` `FUN_00500200` (AI wander cell)
- `0x449540` `BuildingClass::ClearBibArea` (full body)
- `0x743A50` `UnitClass::Scatter` (full body, ~200 lines)
- `0x51D0D0` `InfantryClass::Scatter` (full body, ~230 lines)
- `0x457CE0` `BuildingClass::CanDock` (full body)
- `0x481670` `CellClass::Scatter_Objects` (full body)
- `0x45EE70` `BuildingTypeClass::CanBePlacedAt` (full body)
- `0x5196A0` `InfantryClass::Mission_Enter` (full body, for §11)
- `0x506B90` `FUN_00506B90` (upgrade-slot finder, for cross-check)

### Raw memory inspection

- `0x89F688` direction-offset table 1 region (file is zero-init; runtime fill verified inside `FUN_005060B0`)
- `0x89F6A8` direction-offset table 2 region (same)

### Cross-references — existing reports relied upon

- `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md` (variant A — primary deep-dive)
- `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md` (B+C dispatch + gating)
- `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md` (B specifics)
- `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` (D / E / F caller context)
- `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` (D dispatch context, queue cell math)
- `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md` (G caller context)
- `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md` (CanDock gating in D)

### INI files

- `ini/rulesmd.ini` (PlayerScatter, IQ.Scatter, MaxBaseDistance, FreeUnit)
- `ini/artmd.ini` (Foundation, ExitList, DockingOffset, QueueingCell)
