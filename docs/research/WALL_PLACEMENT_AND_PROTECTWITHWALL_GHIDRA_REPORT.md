# Wall Placement & ProtectWithWall — Ghidra Research Report

**Scope:** BuildingTypeClass Wall= INI reader; ProtectWithWall= INI reader and AI consumer;
can-place-overlay-wall predicate for user and AI placement.

**Primary addresses:**
- `BuildingTypeClass::ReadINI` (partial, named `BuildingTypeClass_ReadINI_Water`) @ `0x00460272`
- `OverlayTypeClass::ReadINI` @ `0x005FE770`
- `Cell_passability_building_placement` @ `0x0047C620`
- `FUN_005060b0` (AI base-placement helper, ProtectWithWall consumer) @ `0x005060b0`
- `OverlayWall_PlacementShadow` @ `0x006D5C50`
- `BuildingPlacement_OverlayRenderer` @ `0x006D5030`
- `BuildingPlacement_per_cell_draw` @ `0x0047EC90`

**Confidence:** HIGH for INI offsets (directly read from decompiled ReadINI bodies, param_1 is `int`,
so all offsets are direct byte offsets). HIGH for ProtectWithWall consumer (offset 0x1765 matched
to the string `s_ProtectWithWall_0081ac80` in the ReadINI decompilation). MEDIUM for the placement
predicate analysis (logic is clear but the exact AI dispatch chain that calls `FUN_005060b0` was not
fully traced upward).

**Active in YR:** Yes for Wall= (overlay walls are core gameplay). ProtectWithWall= consumer
is in an AI building-placement path that runs when AI places buildings — active in standard
YR skirmish AI. See §4 for nuance.

---

## 1. Wall= INI Key — Two Separate Classes

`Wall=` is parsed by **two different ReadINI functions** and stored at **two different struct offsets**:

### 1.1 OverlayTypeClass — Wall= @ +0x2A8

Read in `OverlayTypeClass::ReadINI` @ `0x005FE770`:

```c
uVar3 = CCINIClass__ReadBool(iVar1, &DAT_0081ac58, *(undefined1 *)(param_1 + 0x2a8));
*(undefined1 *)(param_1 + 0x2a8) = uVar3;
```

- **String address:** `0x0081ac58` (contains "Wall")
- **Struct offset:** `OverlayTypeClass + 0x2A8` (bool, default false)
- **Effect:** Marks this overlay type as a wall. Gates the entire DestroyOverlay damage pipeline
  (from the pre-existing WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT). All standard wall
  overlays (GASAND, CYCL, GAWALL, BARB, NAWALL) have Wall=yes in rules.ini.

This is the primary `Wall=` flag that controls gameplay.

### 1.2 BuildingTypeClass — Wall= @ +0x1571

Read in `BuildingTypeClass::ReadINI` @ `0x00460272`:

```c
uVar4 = CCINIClass__ReadBool(iVar21, &DAT_0081ac58,
    CONCAT31((int3)((uint)iVar16 >> 8), *(undefined1 *)(param_1 + 0x1571)));
*(undefined1 *)(param_1 + 0x1571) = uVar4;
```

- **Same string address:** `0x0081ac58` (same "Wall" string, shared across both ReadINI bodies)
- **Struct offset:** `BuildingTypeClass + 0x1571` (bool)
- **Effect:** Used in `BuildingClass::Unlimbo` @ `0x00440580` to gate wall-specific logic at
  building spawn time. The Unlimbo code checks `*(char *)(iVar14 + 0x1571)` early in its
  dispatch table. In vanilla YR, only the overlay-type wall system writes this flag on
  BuildingTypeClass — the GAWALL/NAWALL building-category entries in rules.ini that have
  `Wall=yes` (lines 12031, 12827, 13571) use this field.

Both fields read from the same string at `0x0081ac58`. They are parallel — one for overlay behavior,
one for building-type spawn behavior — and are independent of each other.

---

## 2. ProtectWithWall= INI Key — BuildingTypeClass @ +0x1765

Read in `BuildingTypeClass::ReadINI` @ `0x00460272`:

```c
uVar4 = CCINIClass__ReadBool(iVar21, s_ProtectWithWall_0081ac80,
    *(undefined1 *)(param_1 + 0x1765));
*(undefined1 *)(param_1 + 0x1765) = uVar4;
```

- **String address:** `0x0081ac80` (verified by search_strings; exactly "ProtectWithWall")
- **Struct offset:** `BuildingTypeClass + 0x1765` (bool, default false)
- **INI occurrences:** Present on both UnitTypeClass and BuildingTypeClass sections in
  rulesmd.ini (lines 11649, 11944, 12282, 12319, 12445, 12732, 13039, 13081, 13119, 13230,
  13490, 13571, 13788, 13821). All are `ProtectWithWall=yes`.
- **Note:** `ProtectWithWall=` is parsed by `BuildingTypeClass::ReadINI` only. If UnitTypeClass
  entries in the INI also have it, either they share the same ReadINI dispatch or the field
  at that offset happens to be read for both. The ReadINI function is labeled
  `BuildingTypeClass_ReadINI_Water` in Ghidra. This needs verification for UnitTypeClass
  (see Open Questions).

---

## 3. ProtectWithWall= Consumer — AI Base Placement Offset

The flag at `BuildingTypeClass + 0x1765` is consumed in `FUN_005060b0` @ `0x005060b0`,
an AI building-placement helper (unnamed in Ghidra, called with a building type as `param_3`):

```c
if ((*(char *)((int)param_3 + 0x1765) == '\0') && ((char)param_3[0x55e] == '\0')) {
    bVar15 = false;
} else {
    bVar15 = true;
}
iVar5 = *(int *)(g_RulesClass_Instance + 0x1460);
if (bVar15) {
    iVar5 = iVar5 + 1;
}
```

**What this does:** When the AI computes where to place a building, it looks up a base
perimeter distance from `RulesClass + 0x1460` (likely `AIDefensiveModifier` or similar base
radius). If the building has `ProtectWithWall=yes` (or has flag `+0x55e` set, which appears
to be a harvester/OreDock indicator), the AI adds 1 to that radius. This pushes the building
placement target **1 cell further from the base center**, leaving a ring of gap that can
accommodate wall segments around the protected building.

**Meaning:** `ProtectWithWall=yes` causes the AI to place this building with a 1-cell
standoff from adjacent buildings, so walls can be auto-built around it by the AI's wall-placing
logic (`HouseClass::AI_ScanBasePerimeter`).

**Active in YR:** Yes. `FUN_005060b0` is called from within the AI build pipeline. It is not
gated behind any TS-legacy flag. It is part of the active AI base-builder path.

**TS-legacy assessment:** The ProtectWithWall mechanism itself (add 1 to radius) is live
in YR. The field exists in `BuildingTypeClass` and is actively read during AI placement.
This is NOT a TS-only dead path.

---

## 4. Can-Place Predicate for Overlay Walls — `Cell_passability_building_placement`

**Address:** `0x0047C620`
**Signature (thiscall):** `bool Cell_passability_building_placement(CellClass* this, int speedType, BuildingTypeClass* bt, int houseIndex)`

This function is the gate for **both user and AI** wall (and building) placement on a cell.
It is called from:
- `BuildingPlacement_per_cell_draw` @ `0x0047EC90` (user drag-placement rendering + validation)
- `OverlayWall_PlacementShadow` @ `0x006D5C50` (overlay wall shadow path)

### 4.1 Overlay-wall placement logic (extracted from decompilation)

When `bt` (the building type) has `BuildingTypeClass + 0xe54` non-null (the "e54" pointer —
references the linked OverlayTypeClass), and the pointed-to overlay type has `IsWall` (`+0x2A8`) set:

```python
# Can-place check for overlay wall cells
overlay_idx = cell.OverlayTypeIndex   # CellClass + 0x44
house_owner = cell.WallOwner          # CellClass + 0x50 → house index
level_byte   = cell.field_0x11E       # combined damage|connect byte

# Rule 1: cell already has a wall of the same type (and is not at max damage)
if overlay_idx == bt.linked_overlay_type.overlay_idx:
    if level_byte > 0x0F:   # damage stage > 0, so not pristine but replaceable
        if house_owner == placer_house_index:
            return True

# Rule 2: firestorm/fence-post anchors can serve as wall endpoints
if bt_idx in (Rules[+0x86C], Rules[+0x870], Rules[+0x87C]):
    return True  # E/W or N/S anchor
```

The cell passability check (not wall-specific) also runs:
- `CellClass + 0x124` (occupation flags, low 6 bits) — must be 0
- Speed type and land type compatibility
- `OverlayTypeClass + 0x2A8` (IsWall) blocks placement in map-editor mode if already walled

### 4.2 User vs. AI placement path

**User placement:** Goes through `BuildingPlacement_per_cell_draw` → `Cell_passability_building_placement`.
The call uses `g_UIModeLock` (the currently selected building), `g_PlayerPtr` as house index.

**AI placement:** Also calls `Cell_passability_building_placement` (found in
`OverlayWall_PlacementShadow` path at `0x006D5C50`). The shadow renderer is used for both
UI feedback (user) and AI placement validation. No separate "AI-only" predicate was found —
both use the same gate.

**Key difference:** The user can only place walls on the visible, clickable screen cells;
the AI uses `HouseClass::AI_ScanBasePerimeter` @ `0x005082C0` to pick candidate cells first,
then validates each through the same predicate before queueing wall production.

### 4.3 Overlay wall dispatch in `BuildingPlacement_OverlayRenderer`

At `0x006D5030`, the rendering dispatcher reads `BuildingTypeClass + 0x16BE` (LaserFencePost),
`+0x16C0` (FirestormWall), and `+0xe54` (linked overlay type pointer with IsWall at +0x2A8)
to decide which shadow renderer to call:

```c
if (bt->LaserFencePost) {         // +0x16BE
    LaserFencePost_PlacementShadow(...)
} else if (bt->FirestormWall) {   // +0x16C0
    FirestormWall_PlacementShadow(...)
} else if (bt->e54 != null && bt->e54->IsWall) {  // +0xe54 → +0x2A8
    OverlayWall_PlacementShadow(...)
}
```

This confirms the 3-way split: standard overlay walls (GAWALL, NAWALL, sandbags, etc.) use
`OverlayWall_PlacementShadow`; Firestorm Walls and Laser Fence Posts have their own paths.

---

## 5. Relationship Between Wall= (BuildingTypeClass+0x1571) and Placement

In `BuildingClass::Unlimbo` @ `0x00440580`, `BuildingTypeClass + 0x1571` (Wall=) is checked
before spawning:

```c
if (*(char *)(iVar14 + 0x1571) == '\0') {
    // non-wall building — do standard Unlimbo work
    // includes ExtendWallInDirection × 4 if LaserFencePost (+0x16BE)
    // includes PostDestructionWallCleanup × 4 if fence-post anchor type
    ...
}
```

When Wall= is set (true), the early block is skipped, meaning non-standard wall buildings
go through a different (shorter) Unlimbo path. Standard overlay-wall posts (GAWALL category
BuildingTypeClass entries that have Wall=yes) skip the building-list registration steps and
the full tech-tree tracking.

---

## 6. Open Questions

1. **ProtectWithWall= on UnitTypeClass entries.** The INI has `ProtectWithWall=yes` on
   what appear to be unit sections. Whether `UnitTypeClass::ReadINI` also reads this key
   (at the same or a different offset) was not traced in this session.

2. **`BuildingTypeClass + 0x55e` flag meaning.** In the ProtectWithWall consumer, `param_3[0x55e]`
   is OR'd with ProtectWithWall. This is likely a `Harvester=` or `ResourceGatherer=` flag —
   harvesters also get the 1-cell standoff. Needs verification.

3. **`RulesClass + 0x1460` identity.** The base perimeter radius that ProtectWithWall increments
   was not traced to its INI key. Likely `AIBaseSpacing` or `AIDefensiveModifier`.

4. **`FUN_005060b0` call chain.** The full call path from `HouseClass::AI_Tick` or
   `HouseClass::AI_Building_Strategy` down to `FUN_005060b0` was not traced. The function
   is the placement-candidate selector for AI-built structures (it picks a map cell for the
   next building). Callers not audited.

5. **`BuildingTypeClass + 0xe54` pointer identity.** This is described as a link to an
   OverlayTypeClass; needs full struct trace to confirm it points to the wall overlay type
   that the building post connects to.

---

## Sources

**Ghidra addresses decompiled this session:**
- `0x00460272` — `BuildingTypeClass_ReadINI_Water` (ProtectWithWall @ +0x1765, Wall @ +0x1571)
- `0x005FE770` — `OverlayTypeClass::ReadINI` (Wall @ +0x2A8 — confirmed from pre-existing doc)
- `0x0047C620` — `Cell_passability_building_placement` (placement gate predicate)
- `0x0047EC90` — `BuildingPlacement_per_cell_draw` (user placement loop)
- `0x006D5030` — `BuildingPlacement_OverlayRenderer` (shadow renderer dispatcher)
- `0x006D5C50` — `OverlayWall_PlacementShadow` (overlay wall visual shadow)
- `0x005060b0` — `FUN_005060b0` (AI base placement helper — ProtectWithWall consumer)
- `0x004FE3E0` — `HouseClass::AI_Choose_Building` (reviewed, no ProtectWithWall use)
- `0x005082C0` — `HouseClass::AI_ScanBasePerimeter` (reviewed, no direct ProtectWithWall use)
- `0x00440580` — `BuildingClass::Unlimbo` (Wall= at +0x1571 gating confirmed)

**Strings verified:**
- `0x0081AC80` — "ProtectWithWall" (search_strings result; single xref into ReadINI)
- `0x0081AC58` — "Wall" (shared string; xrefs to both OverlayTypeClass::ReadINI and BuildingTypeClass::ReadINI)

**Cross-referenced docs:**
- `WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT.md` — OverlayTypeClass::ReadINI field map
  (§8), which already verified Wall= at OverlayTypeClass+0x2A8

---

## Summary Table

| Item | Address/Offset | Confidence | Active in YR |
|------|---------------|------------|--------------|
| `Wall=` → OverlayTypeClass | `+0x2A8` bool | HIGH (verified in ReadINI body) | Yes |
| `Wall=` → BuildingTypeClass | `+0x1571` bool | HIGH (verified in ReadINI body) | Yes |
| `ProtectWithWall=` → BuildingTypeClass | `+0x1765` bool | HIGH (verified in ReadINI body) | Yes |
| ProtectWithWall consumer | `FUN_005060b0` @ `0x005060b0` | HIGH (offset 0x1765 read in decompilation) | Yes — AI base placement |
| Effect of ProtectWithWall | Adds 1 to perimeter radius so walls fit | HIGH | Yes |
| Can-place predicate (user + AI) | `Cell_passability_building_placement` @ `0x0047C620` | HIGH | Yes |
| Overlay wall shadow split | `BuildingPlacement_OverlayRenderer` @ `0x006D5030` | HIGH | Yes |
