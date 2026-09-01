# Wall Connection, Damage & Destruction — Ghidra Research Report

**Primary addresses:**
- `CellClass::IsWallConnectableInDirection` @ `0x00480510`
- `CellClass::PostDestructionWallCleanup` @ `0x00480630`
- `CellClass::DestroyOverlay` @ `0x00480CB0` (caller — not fully decompiled)
- `BuildingClass::ConnectWalls` @ `0x00452A40`
- `BuildingClass::AdjustWallConnections` @ `0x00453060`
- `BuildingClass::OnWallDestroyed` @ `0x00453240`
- `BuildingClass::RecalculateWallConnections` @ `0x004533A0`
- `BuildingClass::ExtendWallInDirection` @ `0x00452DC0`
- `OverlayWall_PlacementShadow` @ `0x006D5C50`
- ~~`WallOverlay_HeightAdjust` @ `0x0056BEC0`~~ — mislabeled; function is a crate/overlay placement helper (calls `CrateSlot__PlaceOverlayAndInitTimer` + `FootClass__Find_Nearby_Passable_Cell`), not wall-related. See [OVERLAYWALL_PLACEMENTSHADOW_AND_HEIGHTADJUST_GHIDRA_REPORT.md].

**Data tables:**
- `DAT_0081CC70` — 5×i32 direction table: `[0, 2, 4, 6, -1]` (N/E/S/W, then self)
- `DAT_0089F688` — `g_DirectionOffsets[8]` (N, NE, E, SE, S, SW, W, NW — i16[2] pairs)
- `g_RulesClass_Instance + 0x86C..0x87C` — 5 BuildingType indices linking overlay walls to fence-post building types

**Confidence:** HIGH for the overlay-wall frame/damage/destruction logic and `IsWallConnectableInDirection`. MEDIUM for the BuildingClass fence-wall interactions (firestorm walls, laser fences — `ConnectWalls`/`AdjustWallConnections` decompiled but not fully exercised).
**Active in YR:** Yes — walls are a core gameplay system.

---

## 1. Overview

RA2/YR has **two parallel wall systems** sharing a common connectivity algorithm:

1. **Overlay-layer walls** — the standard Sandbag / Chain-link / Wood-fence / Concrete walls.
   Stored on `CellClass.OverlayTypeIndex` (+0x44); wall frame byte at `CellClass + 0x11E`
   packs [damage nibble | connectivity nibble] into a single u8.

2. **Building-layer fence posts** — Laser Fence Posts (`BuildingType + 0x16BE` = `LaserFencePost=yes`)
   and Laser Fence segments (`BuildingType + 0x16BF` = `LaserFence=yes`). Firestorm Wall is a separate
   flag at `BuildingType + 0x16C0` (`FirestormWall=yes`). All three are code-live but data-inert in
   stock YR — no stock building sets any of them. Stored as BuildingClass instances; frame at
   `BuildingClass + 0x618`. See [FIRESTORM_LASER_FENCE_POST_INTERACTIONS_GHIDRA_REPORT.md] for
   verified offsets and the 5 `RulesClass+0x86C..0x87C` gate-building indices.

The two systems interact at connection sites: a fence-post BuildingType listed in
`RulesClass + 0x86C..0x87C` can register as a wall-endpoint for certain overlay-wall
types, so walls can link up to firestorm endpoints and vice versa.

---

## 2. Overlay-Wall Frame Byte Layout (`CellClass + 0x11E`, u8)

| Nibble | Bits | Meaning |
|--------|------|---------|
| Lower | `0x0F` | Connectivity bitmask — which cardinal neighbors have a connecting wall |
| Upper | `0xF0` | Damage stage |

### 2.1 Lower nibble — connectivity bitmask

Bits set by testing the 4 cardinal neighbors via `IsWallConnectableInDirection`:

| Bit | Mask | Direction (cell-coord) |
|-----|------|-----------------------|
| 0 | `0x01` | N (Y − 1) |
| 1 | `0x02` | E (X + 1) |
| 2 | `0x04` | S (Y + 1) |
| 3 | `0x08` | W (X − 1) |

Bit assignment derived from `DAT_0081CC70 = [0, 2, 4, 6]` (indices into the 8-dir
`g_DirectionOffsets` table; with that table laid out as `[N, NE, E, SE, S, SW, W, NW]`
even indices are cardinals).

Lower nibble ranges 0..15 → 16 connectivity variants:
- `0x00` isolated pillar
- `0x01` N-stub, `0x02` E-stub, `0x04` S-stub, `0x08` W-stub
- `0x03` NE-L, `0x05` NS-straight, `0x06` ES-L, `0x09` NW-L, `0x0A` EW-straight, `0x0C` SW-L
- `0x07` NES-T, `0x0B` NEW-T, `0x0D` NSW-T, `0x0E` ESW-T
- `0x0F` four-way cross

### 2.2 Upper nibble — damage stage

Empirically observed values: `0x00`, `0x10`, `0x20`, `0x30`. Concrete walls
(Allied GAWALL idx 2, Soviet NAWALL idx 0x1A) use all 4 stages; simpler walls use
fewer. The upper nibble is INCREMENTED by damage events (not traced in detail this
pass — presumably in damage-dispatch code that deducts HP and moves to next stage).

### 2.3 Frame index into wall SHP

The combined byte is the frame index directly: `shp_frame = cell.field_0x11E`.
Wall SHPs pack 16 frames per damage stage → total frames = 16 × (number of stages).

---

## 3. Connectivity Predicate — `CellClass::IsWallConnectableInDirection` @ `0x00480510`

Signature (`__thiscall`):
```c
bool IsWallConnectableInDirection(
    this: CellClass*,
    target_overlay_idx: u32,   // what wall type are we looking for? -1 = any
    dir: int                    // 0=N, 2=E, 4=S, 6=W
);
```

Return `true` if a wall of `target_overlay_idx` is reachable from `this` cell in
the given direction. Logic:

```python
def is_wall_connectable(this_cell, target_overlay_idx, dir):
    own = this_cell.OverlayTypeIndex  # +0x44

    # Same type already placed
    if own == target_overlay_idx and own != 0xFFFFFFFF:
        return True

    # Wildcard: any wall?
    if target_overlay_idx == 0xFFFFFFFF:
        if own in (2, 0x1A, 0xF3):   # GAWALL, NAWALL, wildcard
            return True

    # Cross-system: scan cell's object list for fence-post buildings
    if target_overlay_idx in (0, 2):   # GASAND or GAWALL
        for obj in this_cell.objects_in_cell:   # +0xE4 linked list via +0x30 next
            if obj.rtti == 6 and obj.HP > 0:    # BuildingClass with HP
                bt_idx = obj.BuildingType_Index
                if ((bt_idx == Rules[+0x86C] and dir in (2, 6)) or
                    (bt_idx == Rules[+0x870] and dir in (0, 4)) or
                    bt_idx == Rules[+0x87C]):
                    return True

    if target_overlay_idx == 0x1A:     # NAWALL
        for obj in this_cell.objects_in_cell:
            if obj.rtti == 6 and obj.HP > 0:
                bt_idx = obj.BuildingType_Index
                if ((bt_idx == Rules[+0x874] and dir in (2, 6)) or
                    (bt_idx == Rules[+0x878] and dir in (0, 4))):
                    return True

    return False
```

### 3.1 Compact overlay indices and active-retail status

| Idx | Name | Retail status | artmd DamageLevels | Notes |
|-----|------|---------------|--------------------|-------|
| `0` | GASAND | Active, `Wall=yes` | 2 | Allied sandbags |
| `1` | CYCL | Dormant/mod-conditional | no section | Hardcoded cleanup row exists, but retail constructor default `Wall=false` remains |
| `2` | GAWALL | Active, `Wall=yes` | 3 | Allied concrete wall; connects via Rules+0x86C/0x870/0x87C |
| `3` | BARB | Dormant/mod-conditional | no section | Hardcoded cleanup row exists, but no retail type section activates it |
| `0x16` | FENC | Dormant/mod-conditional | no section | Compact slot exists; no retail type section activates it |
| `0x1A` | NAWALL | Active, `Wall=yes` | 3 | Soviet concrete wall; connects via Rules+0x874/0x878 |
| `0xF3` | — | Connection sentinel, not a retail wall type row | — | Acts as any-wall when target=-1 |

Retail proof: `rulesmd.ini` sets `Wall=yes` only for GASAND, GAWALL, and NAWALL.
CYCL, BARB, and FENC have no section anywhere under retail `ini/`;
`OverlayTypeClass::Constructor @ 0x005FE250` initializes `Wall=false` at `0x005FE296`, and
`ReadINI @ 0x005FE770` retains that value when the key is absent. YR does not merge the base TS
INI set, so the three hardcoded rows are code-present but inactive in active retail.

### 3.2 RulesClass wall-building references

| Offset | Purpose |
|--------|---------|
| `+0x86C` | BuildingType idx that links to GASAND/GAWALL in E/W direction (fence-post anchor) |
| `+0x870` | BuildingType idx that links to GASAND/GAWALL in N/S direction |
| `+0x874` | BuildingType idx that links to NAWALL in E/W direction |
| `+0x878` | BuildingType idx that links to NAWALL in N/S direction |
| `+0x87C` | Fallback BuildingType that links to any wall in any direction |

These correspond to the fence-post BuildingTypes (Firestorm Wall anchors and
similar). If not set, the fallback is `-1` and no cross-system linking occurs.

---

## 4. Wall Damage Pipeline — `CellClass::DestroyOverlay` @ `0x00480CB0`

**This is the primary damage + destruction entry point for walls.** Named
`DestroyOverlay` but handles both per-tick damage accumulation AND final destruction.

### 4.0 Callers (where wall damage flows in)

| Caller | Purpose |
|--------|---------|
| `Apply_area_damage` @ `0x004896AD` | Area-damage weapons, including active-retail Lightning Storm: `[IonWH]` has `Wall=yes` and `Wood=yes` |
| `BuildingClass::OnDestroyed` @ `0x00445B69` | Building demolition — destroys overlays on vacated cells |
| `UnitClass::Mission_Enter` @ `0x0073B056` | Unit entering cell (engineers, hover units clearing paths) |
| `FUN_0075F330` @ `0x0075F477` | Unidentified unit-activity path |
| `CellClass::DestroyOverlay` itself (self-recursion) @ `0x00480EAF` | Concrete-wall chain reaction |

Retail-data exclusion: Genetic Mutator also reaches the generic area-damage
plumbing, but active-retail `[MutateExplosion]` has no `Wall=`, `Wood=`, or
`WallAbsoluteDestroyer=` flag. Its wall callback is therefore dormant; this is
not authority to omit the generic callback from a future mod-support owner.

### 4.1 Algorithm — per-tick damage accumulation

```python
def DestroyOverlay(cell, damage):
    # 0. Wall-gate: only operates on wall overlays
    if cell.OverlayTypeIndex == -1: return 0
    overlay_type = g_OverlayTypeClass[cell.OverlayTypeIndex]
    if not overlay_type.Wall:       # +0x2A8
        return 0

    # 1. Probabilistic damage gate (damage==-1 bypasses → "forced destroy")
    if damage != -1:
        if damage < overlay_type.Strength and not map_editor_mode:    # +0x2A4
            if Random.RandomRanged(0, overlay_type.Strength) >= damage:
                return 0     # this tick no damage — roll failed

    # 2. Dirty screen rect (combines GetOccupyList + GetOverlapList)
    TacticalClass.dirty_screen_rect(...)

    # 3. Increment damage stage (upper nibble)
    cell.field_0x11E += 0x10

    # 4. Chain reaction (only for multi-stage walls, i.e. concrete)
    new_stage = cell.field_0x11E >> 4
    max_stage = overlay_type.DamageLevels - 1                         # +0x2A0
    if new_stage == max_stage and overlay_type.DamageLevels > 2:
        # Cascade 200 damage into 4 cardinal neighbors of same overlay type
        # that are still at damage stage 0 (pristine)
        for dir in [0, 2, 4, 6]:    # N, E, S, W
            neighbor = map.get_cell(cell.coord + g_DirectionOffsets[dir])
            if (neighbor.OverlayTypeIndex == cell.OverlayTypeIndex and
                neighbor.field_0x11E >> 4 == 0):
                DestroyOverlay(neighbor, damage=200)   # 0xC8 chain payload

    # 5. Destruction gate
    if damage != -1:
        stage = cell.field_0x11E >> 4
        if stage < max_stage:
            return 0              # not fully damaged yet
        if stage == max_stage and (cell.field_0x11E & 0x0F) != 0:
            return 0              # max-damaged but still connected → stay visible

    # 6. Remove overlay
    cell.field_0x50 = 0xFFFFFFFF       # clear (AttachedTag? owner?)
    cell.OverlayTypeIndex = -1
    cell.field_0x11E = 0
    CellClass.RecalcAttributes(cell)   # re-runs LAT

    # 7. Zone + radar housekeeping
    MapClass.AssignOrphanedCellZone(cell)
    FUN_00584550(cell)
    RadarClass.mark_terrain_dirty(cell)

    # 8. Update 4 cardinal receivers in literal N,W,S,E order. Each receiver
    #    performs its own N,E,S,W,self cleanup walk before the next receiver.
    for dir_offset in [N, W, S, E]:
        neighbor = map.get_cell(cell.coord + dir_offset)
        CellClass.PostDestructionWallCleanup(neighbor, flag=0)

    FUN_007258D0()    # internal cleanup

    # 9. Broadcast expiry for this exact CellClass pointer after the complete
    #    cleanup fan-out and before the retained-count tail.
    PointerExpired(cell)

    # 10. Decrement OreNeighborCount on all 8 neighbors
    for dir in range(8):
        neighbor = map.get_cell(cell.coord + g_DirectionOffsets[dir])
        neighbor.field_0x122 -= 1

    return 1          # destroyed
```

### 4.2 Two key behaviors that emerge

1. **Walls are not torn down by a single shot.** When `damage < Strength`, every
   damage tick rolls the inclusive range `RandomRanged(0, Strength)` and advances
   only when `roll < damage`; equality is a no-op. A wall with `Strength=400` hit
   by `damage=100` therefore advances on 100 of 401 possible results.

2. **Walls stay visible while connected even at max damage.** The §5 destruction
   gate requires the connectivity nibble == 0. A mid-segment wall at max damage
   will keep its frame as `(max << 4) | bitmask` until neighbors fall; only when
   isolated does it vanish. This is the observed in-game behavior — damaged walls
   appearing "cracked" mid-segment, collapsing only when the segment breaks.

3. **Concrete walls (GAWALL/NAWALL) chain-react.** With retail artmd
   `DamageLevels=3`, reaching stage 2 (`0x20`) enters the chain branch and deals 200 damage
   to 4 pristine concrete neighbors, which usually destroys them outright given
   typical wall Strength values. This is why concrete walls often collapse in
   cascading chains rather than segment-by-segment.

---

## 5. Post-destruction Cleanup — `CellClass::PostDestructionWallCleanup` @ `0x00480630`

Called **by** `DestroyOverlay` (§4 step 8) on each of the 4 cardinal receivers in
literal `N,W,S,E` order. Each call completes before the next receiver and rebuilds
connectivity against the live state left by prior calls.

Walks a 5-entry table (`DAT_0081CC70 = [0, 2, 4, 6, -1]`) covering
`N,E,S,W,self`. Every lookup uses the signed fixed-map `Get_CellClass` seam. A
true miss stamps and returns the persistent shared dummy; it is not skipped.
For each visited CellClass, including the dummy, tactical and radar dirty work
runs before the wall gate:

### 4.1 Per-cell steps

```python
for dir_entry in [0, 2, 4, 6, -1]:
    cell = self if dir_entry == -1 else map.get_cell(self.coord + dir_offset(dir_entry))

    # 1. Mark tactical + radar dirty
    TacticalClass.dirty_screen_rect(cell)
    RadarClass.mark_terrain_dirty(cell)

    # 2. Only if cell has a wall overlay (OverlayTypeClass +0x2A8 IsWall flag)
    if cell.OverlayTypeIndex == -1: continue
    if not OverlayType[cell.OverlayTypeIndex].is_wall: continue

    # 3. Rebuild connectivity nibble
    connectivity = 0
    for bit, d in enumerate([0, 2, 4, 6]):          # N, E, S, W
        neighbor = map.get_cell(cell.coord + dir_offset(d))
        if IsWallConnectableInDirection(neighbor, cell.OverlayTypeIndex, d):
            connectivity |= 1 << bit

    cell.field_0x11E = (cell.field_0x11E & 0xF0) | connectivity

    # 4. Apply auto-destruct rules: isolated + max-damage → remove
    destroyed = False
    data = cell.field_0x11E       # full byte: [damage | connect]

    if cell.OverlayTypeIndex in (2, 0x1A):       # GAWALL, NAWALL
        if data in (0x20, 0x30):                 # fully damaged or destroyed
            destroyed = True
    elif cell.OverlayTypeIndex == 0:              # GASAND
        if data in (0x10, 0x20):
            destroyed = True
    elif cell.OverlayTypeIndex == 1:              # CYCL
        if data == 0x20:
            destroyed = True
    elif cell.OverlayTypeIndex == 0x16:
        if data in (0x10, 0x20):
            destroyed = True
    elif cell.OverlayTypeIndex == 3:              # BARB
        if data == 0x10:
            destroyed = True

    if destroyed:
        cell.OverlayTypeIndex = -1
        cell.field_0x11E = 0
        cell.field_0x50 = 0xFFFFFFFF   # clear owner?
        FUN_007258d0()                 # internal cleanup

    old_zone = cell.nZoneType

    # 5. Re-run RecalcAttributes → triggers ApplyLAT_and_SlopeFixup
    CellClass.RecalcAttributes(cell)

    # 6. Zone recomputation runs only when Recalc changed nZoneType
    if old_zone != cell.nZoneType:
        if destroyed:
            MapClass.AssignOrphanedCellZone(cell)
            # Only this changed-zone auto-removal decrements the 8 neighbors.
            for dir in range(8):
                neighbor = map.get_cell(cell.coord + dir_offset(dir))
                neighbor.field_0x122 -= 1   # OreNeighborCount
        else:
            MapClass.MergeAdjacentCellZone(cell)
```

### 5.2 Destruction thresholds summary (cleanup safety-net path)

| Overlay | Full byte → destroy in cleanup | Active-retail status |
|---------|-------------------------------|----------------------|
| GASAND (0) | `0x10` or `0x20` | Active |
| CYCL (1) | `0x20` | Dormant/mod-conditional |
| GAWALL (2) | `0x20` or `0x30` | Active |
| BARB (3) | `0x10` | Dormant/mod-conditional |
| FENC (`0x16`) | `0x10` or `0x20` | Dormant/mod-conditional |
| NAWALL (`0x1A`) | `0x20` or `0x30` | Active |

Note: these are the **cleanup safety-net** destruction checks, not the primary
destruction gate. The primary path is in `CellClass::DestroyOverlay` (§4 step 5),
which requires damage-stage == `DamageLevels - 1` AND connectivity nibble == 0.
`PostDestructionWallCleanup` also enforces a similar check as a safety net in case
a neighbor's damage stage moved past max without triggering the primary destroy
(e.g., for walls damaged by area-damage on non-center cells).

**Active-retail DamageLevels (verified from artmd.ini and
OverlayTypeClass::ReadINI @ 0x005FE770, field +0x2A0):** GASAND is 2; GAWALL and
NAWALL are 3. CYCL, BARB, and FENC have no retail art section, so their hardcoded
threshold rows remain dormant unless a mod supplies both an active wall type and art data.

DamageLevels is read by `OverlayTypeClass::ReadINI` from the **art.ini** section
named by the overlay's `Image=` key, not from rules.ini. Defaults to 1 if not set
(single-stage, destroys on first hit at random).

---

## 5. Building-Wall Connection Logic

Fence-post buildings (`BuildingType + 0x16BE` = `LaserFencePost=yes`)
auto-connect to adjacent fence-posts of the same type and owner.

### 5.1 `BuildingClass::ConnectWalls` @ `0x00452A40`

Entry point when a fence-post building is built. Iterates 4 cardinal directions
(uVar8 += 2, masked to 7). For each direction:
- Gets adjacent cell, looks up any BuildingClass in that cell
- If adjacent building has the `+0x16BF` "connectable" flag AND same House:
  - Checks a timer-based hash (`RateTimer::Current() >> 0xC + 1 >> 1 & 3 == dir & 3`) to rate-limit
  - ORs bit into `this->LaserFenceFrame` (`BuildingClass + 0x618` equiv, though
    field offset varies by build — confirmed in binary as single u32)
  - Calls `BuildingClass::AdjustWallConnections(dir, neighbor)`

### 5.2 `BuildingClass::RecalculateWallConnections` @ `0x004533A0`

Called on existing fence-posts when a neighbor is added/removed. Loops 4 cardinals,
for each matching neighbor (same House, same BuildingType, timer check):
- If `param_2 != 0` (forced recompute) OR the extension logic applies:
  - Computes frame indices via `BuildingClass::FindNearestFencePost` (partially traced)
  - Switch on direction to pick frame numbers 0-6 for directional variants
  - Writes `neighbor->Frame = frame_id` and `neighbor->field_0x80 = 1` (dirty)
- Special "firestorm active" variant (checked via `building->field_0x6EA` != 0 AND
  `vtable[0x184]` return not in {0x12, 0x13}) applies different frame numbers

### 5.3 Frame encoding for fence-posts

Observed base frames:
| Frame | Meaning |
|-------|---------|
| `0..7` | Directional + active variants |
| `8` | Connected segment mid (type 1) |
| `12` (`0xC`) | Default / unconnected pillar (type 2) |
| `3`, `7` | Used for `piStack_24 == piStack_20` path in active-mode |

Switch cases per direction (uStack_40):
| Direction | Base | Mid | End |
|-----------|------|-----|-----|
| 0 (N) | 5 | 4 | 6 |
| 2 (E) | 1 | 0 | 2 |
| 4 (S) | 6 | 4 | 5 |
| 6 (W) | 2 | 0 | 1 |

Exact semantics of "Base/Mid/End" not fully traced — these are assignments to
`piVar6[0x186]` (frame field) on chained fence-posts.

---

## 6. Integration with RecalcAttributes / LAT

`PostDestructionWallCleanup` ends each cell visit with `CellClass::RecalcAttributes`,
which in turn runs `ApplyLAT_and_SlopeFixup`. So **wall destruction automatically
retriggers ground LAT** on up to 5 cells (self + 4 cardinal neighbors).

This means any ground-LAT system that uses pavement-under-walls exemption (`Pave`
LAT group exempts `PavedRoads..+0x14` per the IsometricTileTypeClass report §4.3)
will re-flow correctly when walls come down — the Pave LAT re-tiles the exposed
cells to match their current neighborhood.

---

## 7. Current Rust Implementation Status

The live runtime owner is now `src/sim/overlay_grid.rs`; the older
`src/map/overlay.rs::compute_wall_connectivity` remains a map-side helper and is not the mutation
authority.

| System | Rust coverage | Gap |
|--------|---------------|-----|
| Full damage/connectivity byte | Implemented in `OverlayCell::overlay_data` | N/E/S/W are bits 0/1/2/3 and damage remains in the upper nibble. |
| Same-ID runtime connection | Implemented by `recompute_wall_connectivity_at_with_terrain` | Runtime probes use the signed fixed-grid CellClass lookup, including real aliases such as west of `(0,1)` selecting `(511,0)`. |
| Damage gate, penultimate chain, and direct removal | Implemented by `damage_wall_overlay_with_runtime_host` | The inclusive roll uses `roll < damage`; chain recursion is N/E/S/W. Each terminal recursive removal Recalcs and repairs navigation before its cleanup fan-out, expires the represented Cell pointer after that fan-out, and decrements its retained source last. |
| Post-destruction cleanup | Implemented in N/W/S/E outer order and N/E/S/W/self inner order | Every visit dirties tactical/radar before the wall gate; every live wall recomputes and Recalcs synchronously. GASAND/GAWALL/NAWALL thresholds are active retail code/data, but no shipped-map or ordinary invariant-preserving placement witness for the pre-existing isolated-damaged input is established. Dormant CYCL/BARB/FENC rows are deliberately excluded. |
| Conditional cleanup count reversal | Implemented through `recalc_wall_mutation_passability` | A cleanup-removed source reverses its eight wrapping count writes only when Recalc changed reduced zone type. House sale intentionally leaves the sold source stale. |
| Fixed-grid alias and shared-dummy runtime | Implemented | Chain, cleanup, sale, and all eight retained-count probes use the stamping lookup. Real aliases mutate real cells; a true dummy retains live overlay identity/state and emits its captured packed coordinate to tactical/radar callbacks while remaining absent from the exported real count plane. Dummy identity/state joins the v114 current hash but remains process-global rather than Scenario-serialized. |
| Runtime dirty, navigation, and pointer-expiry publication | Critic-1 correction implemented; fresh criticism pending | The first fresh critic rejected `95f77159` because production replayed terminal radar after return, omitted damage tactical dirty, and left placement/sale observers outside native order. The correction publishes tactical/radar through the synchronous wall host for standard combat, persistent projectile/death paths, ambient Wave, movement crush/world events, sale, placement, and active-retail Lightning Storm. Pointer expiry covers represented Techno Cell targets in native forward clear-first order. The broader native non-entity listener roster is not represented and remains an explicit residual. |
| Authored-load wall effects | Infrastructure only | `LiveOverlayCells` and `FinalizedOverlayPayload` retain the ordered authored count plane, but no production authored-row reader calls the helper or consumes the payload yet. |
| Wildcard `0xF3` and building-anchor matching | Missing | The direction-aware `IsWallConnectableInDirection` building branches are outside this retained-plane slice and keep the wider wall mechanism open. |
| Firestorm/Laser Fence building behavior | Missing | No active-retail Rust owner implements the BuildingClass connect/extend system. Do not infer it from plain overlay matching. |
| Full RecalcAttributes/LAT equivalence | Partial | Runtime wall paths synchronously update represented overlay land/passability/zone facts; the broader live LAT/Recalc transaction remains owned by authored-overlay finalization. |

### 7.1 Priority fixes

1. Complete focused validation and obtain a new fresh read-only critic verdict on the corrected
   runtime count/dummy/dirty/navigation/pointer/placement slice. The new critic must recheck every
   finding from the `95f77159` rejection; keep the slice open on any unresolved ordering, callsite,
   reachability, or represented-listener finding.
2. Wire the verified authored wall helper into the one production authored-row transaction and move
   its finalized identity/state/count payload into every production/headless runtime builder.
3. Close the current-version persistence gate: production state must never save or restore legacy
   `None` retained authority after migration (or the snapshot version must advance if such a save
   escapes).
4. Investigate and implement the separate direction-aware building-anchor branches before claiming
   the complete wall-connection mechanism closed. Their absence does not invalidate the active
   overlay-to-overlay runtime/count transaction, but it keeps this broader document open.

Correction-focused validation on 2026-09-01 passed: the 107-test `wall` filter, 59 overlay-grid
tests, 12 Lightning Storm tests, exact host-order, placement, autofill, persistent-projectile, sale,
radar-rearm, ordinary-warhead, and crusher fixtures, the shared-dummy v114 hash test, and all three
live-object detach-sweep tests. A new fresh critic must still recheck every critic-1 finding.
The full `--lib` suite remains reserved for the final PR gate.

---

## 8. OverlayTypeClass Field Map (verified from `ReadINI` @ `0x005FE770`)

All offsets confirmed. Key fields for the wall system bolded:

| Offset | Type | INI Key | Default | Purpose |
|--------|------|---------|---------|---------|
| `+0x298` | i32 | `Land=` | 0 (Clear) | LandType override (bypassed if +0x2AC is true) |
| `+0x29C` | `AnimType*` | `CellAnim=` | null | Spawning animation |
| `+0x2A0` | i32 | `DamageLevels=` (art.ini) | 1 | **Number of damage stages** |
| `+0x2A4` | i32 | `Strength=` | 1 | **HP — probabilistic damage gate** |
| `+0x2A8` | bool | `Wall=` | false | **IsWall flag** |
| `+0x2A9` | bool | `Tiberium=` | false | Is ore; forces Land=5 + Armor=6 |
| `+0x2AA` | bool | `Crate=` | false | Pickup crate |
| `+0x2AB` | bool | `CrateTrigger=` | false | Crate activation |
| `+0x2AC` | bool | `NoUseTileLandType=` | **true** | When true, overlay's +0x298 supersedes tile LandType |
| `+0x2AD` | bool | `IsVeinholeMonster=` | false | [TS legacy] |
| `+0x2AE` | bool | `IsVeins=` | false | [TS legacy] |
| `+0x2AF` | bool | — | false | Never written by ReadINI (TS legacy / dead) |
| `+0x2B0` | bool | `Explodes=` | false | Explodes on destroy |
| `+0x2B1` | bool | `ChainReaction=` | false | Chain explosions (damage ripple — separate from wall-chain in §4 step 4) |
| `+0x2B2` | bool | `Overrides=` | false | Can override existing overlay |
| `+0x2B3` | bool | `DrawFlat=` | **true** | Draw flat on ground (no Z) |
| `+0x2B4` | bool | `IsRubble=` | false | Post-destruction rubble |
| `+0x2B5` | bool | `IsARock=` | false | Rock/decorative (ignores Z) |
| `+0x2B6` | byte[3] | `RadarColor=` | — | Minimap RGB |

**Note:** The wall cascade chain-reaction in §4 step 4 is gated by `DamageLevels > 2`,
not by `ChainReaction=`. The `ChainReaction=` key fires the generic damage-ripple
logic used for Tiberium/ammo overlays (explode → damage neighbors → explode), a
completely separate system from wall cascading.

---

## 9. Open Questions

1. **Exact semantics of `RulesClass + 0x87C`** — the "fallback any-direction"
   fence-post ref. Used in the same-idx-in-cell test; purpose of a direction-agnostic
   fence-post is unclear — possibly a 1x1 hub/pillar that serves as a universal
   connector.

2. **`OverlayType 0xF3` identity** — wildcard-wall, but its actual INI name and
   purpose weren't identified. Likely a Tiberian Sun holdover.

3. **Firestorm-active path** in `RecalculateWallConnections` — the `bVar2` branch
   selects alternate frames when the building's `+0x6EA` flag is set (Firestorm on?)
   and vtable[0x184] returns not 0x12/0x13. Exact enum meaning of 0x12/0x13 needs
   verification (likely animation states).

4. **`BuildingClass::OnWallDestroyed` @ `0x00453240`** is actually called from
   `BuildingClass::Unlimbo` @ `0x0044075F` — i.e., when a fence-post building
   SPAWNS. It looks for adjacent fence-posts in a timer-randomized direction and
   calls `AdjustWallConnections(dir | 4, 0)`. The naming from Ghidra is misleading —
   this is the **spawn-time auto-connect** logic, not a destruction response. The
   `| 4` flag in the dir arg probably means "extend-mode" (vs. normal connect).

---

## Sources

**Ghidra addresses decompiled:**
- `0x00452A40` — BuildingClass::ConnectWalls
- `0x00452DC0` — BuildingClass::ExtendWallInDirection (player-command wall build)
- `0x00453060` — BuildingClass::AdjustWallConnections
- `0x00453240` — BuildingClass::OnWallDestroyed (actually called from Unlimbo — see §9.4)
- `0x004533A0` — BuildingClass::RecalculateWallConnections
- `0x00480510` — CellClass::IsWallConnectableInDirection
- `0x00480630` — CellClass::PostDestructionWallCleanup
- `0x00480CB0` — CellClass::DestroyOverlay (primary wall damage + destroy)
- `0x005FE770` — OverlayTypeClass::ReadINI (verified OverlayTypeClass field layout)
- `0x0056EB80` — MapClass::SetOverlayAndPropagate (referenced — tile flood-fill)

**Xref audit:**
- DestroyOverlay callers: Apply_area_damage, BuildingClass::OnDestroyed,
  UnitClass::Mission_Enter, FUN_0075F330, (self-recursive for chain reaction)
- OnWallDestroyed callers: BuildingClass::Unlimbo only

**Memory tables dumped:**
- `0x0081CC70` — `[0, 2, 4, 6, -1]` direction table (verified 20 bytes)

**Cross-referenced docs:**
- `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` — LAT retrigger via RecalcAttributes
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` — cell field offsets (+0x11E, +0x44, +0xE4)
- `OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md` — OverlayTypeClass IsWall flag (+0x2A8)

**Rust source audited:**
- [src/map/overlay.rs](src/map/overlay.rs) — has basic connectivity but no damage or cleanup
- `src/map/overlay_types.rs` — overlay registry
- `src/rules/ruleset.rs` — no RulesClass wall-ref fields yet
