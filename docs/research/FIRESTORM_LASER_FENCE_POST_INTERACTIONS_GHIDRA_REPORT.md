# Firestorm Wall / Laser Fence Post — Ghidra Investigation Report

**Date:** 2026-05-19  
**Target:** BuildingTypeClass+0x16BE/+0x16BF flags, RulesClass+0x86C..+0x87C 5-index array, wall-connectivity scan mechanism, YR-active status  
**Source:** Live Ghidra decompilation of gamemd.exe

---

## 1. Resolving the +0x16BE / +0x16BF Conflict

The two prior docs (WALL_CONNECTION and GATE_MECHANIC) contradict each other. The decompilation of `BuildingTypeClass_ReadINI_Water` (which references all three string addresses 0x0081aa20, 0x0081aa30, 0x0081aa3c) resolves the conflict definitively.

**Verified from decompile at 0x00460A93..0x00460AC7:**

```c
// offset +0x16BD:
uVar4 = CCINIClass__ReadBool(iVar21, s_WeaponsFactory_0081aa4c, *(param_1 + 0x16bd));
*(param_1 + 0x16bd) = uVar4;

// offset +0x16BE:
uVar4 = CCINIClass__ReadBool(iVar21, s_LaserFencePost_0081aa3c, *(param_1 + 0x16be));
*(param_1 + 0x16be) = uVar4;

// offset +0x16BF:
uVar4 = CCINIClass__ReadBool(iVar21, s_LaserFence_0081aa30, *(param_1 + 0x16bf));
*(param_1 + 0x16bf) = uVar4;

// offset +0x16C0:
uVar4 = CCINIClass__ReadBool(iVar21, s_FirestormWall_0081aa20, *(param_1 + 0x16c0));
*(param_1 + 0x16c0) = uVar4;
```

**Definitive offset map:**

| Offset | INI Key | Purpose |
|--------|---------|---------|
| +0x16BD | `WeaponsFactory=` | BuildingType flag (unrelated to fence) |
| **+0x16BE** | **`LaserFencePost=`** | **Boolean: this building is a fence-post anchor** |
| **+0x16BF** | **`LaserFence=`** | **Boolean: this building is a fence wall segment** |
| **+0x16C0** | **`FirestormWall=`** | **Boolean: this building is a Firestorm Wall segment** |

**Verdict on prior docs:**
- WALL_CONNECTION doc's claim "+0x16BE = FirestormWall" is **WRONG**. FirestormWall is at +0x16C0.
- GATE_MECHANIC doc's claim "+0x16BE = LaserFencePost, +0x16BF = LaserFence" is **CORRECT**.

The INI key names as strings in the binary: `LaserFencePost` @ 0x0081AA3C, `LaserFence` @ 0x0081AA30, `FirestormWall` @ 0x0081AA20. All have exactly one xref each, all pointing into `BuildingTypeClass_ReadINI_Water`.

**Active in YR:** Conditional. `LaserFencePost` and `LaserFence` are parsed for any building that declares them. No stock building in rulesmd.ini sets `LaserFencePost=yes` or `LaserFence=yes` (verified by grep of ini/rulesmd.ini — no such keys appear anywhere). `FirestormWall=yes` is also absent from rulesmd.ini. These are parsed-but-unused flags in a standard YR skirmish.

---

## 2. RulesClass+0x86C..+0x87C — 5-Index Gate/Tower Array

These 5 slots are populated in `RulesClass__ReadGeneral` @ 0x0066D530 from the `[General]` INI section. They are **not** fence-post related — they hold gate and wall-tower BuildingType pointers used in the building-placement passability check.

**Verified from decompile at 0x0066D530 (ReadGeneral):**

| RulesClass Offset | INI Key | rules.ini default | rulesmd.ini value |
|---|---|---|---|
| **+0x86C** | `GDIGateOne=` | `GAGATE_A` | `GADUMY` |
| **+0x87 0** | `GDIGateTwo=` | `GAGATE_B` | `GADUMY` |
| **+0x874** | `NodGateOne=` | `NAGATE_A` | `GADUMY` |
| **+0x878** | `NodGateTwo=` | `NAGATE_B` | `GADUMY` |
| **+0x87C** | `WallTower=` | `GACTWR` | `GADUMY` |

All 5 are read via `CCINIClass__ReadString` + `BuildingTypeClass__FindOrAllocate()`, storing a BuildingType pointer (not an index). They store **pointers to BuildingTypeClass objects**, not integer indices.

In rulesmd.ini all 5 are set to `GADUMY` — the generic dummy building. The comments in rulesmd.ini (lines 374–379) explicitly name the intended RA2 values and note "these buildings affect nearby walls, so I need to know what they are." This was a deliberate YR override that neutralized the gate-type passability logic by routing all 5 slots to the same dummy type.

**How they are used (Cell_passability_building_placement @ 0x0047C620):**

```c
// Terrain overlay types 0 and 2 (concrete wall, etc.) — gate passability:
if ((param_3 == *(int *)(g_RulesClass_Instance + 0x87c) ||
    (param_3 == *(int *)(g_RulesClass_Instance + 0x86c))) ||
   (param_3 == *(int *)(g_RulesClass_Instance + 0x870))) {
    return 1;  // allow placement
}
// Terrain type 0x1A — gate passability:
if (param_3 == *(int *)(g_RulesClass_Instance + 0x874) ||
   (param_3 == *(int *)(g_RulesClass_Instance + 0x878))) {
    return 1;
}
```

The 5 slots gate whether a building can be placed on a cell with certain overlay terrain types. Since all 5 point to `GADUMY` in YR, this effectively means only the dummy building type gets the pass-through benefit, not real gate buildings.

---

## 3. Wall-Connectivity Scan Mechanism

Examined: `BuildingClass__ConnectWalls` @ 0x00452A40, `BuildingClass__AdjustWallConnections` @ 0x00453060, `BuildingClass__RecalculateWallConnections` @ 0x004533A0, `BuildingClass__ExtendWallInDirection` @ 0x00452DC0, `BuildingClass__OnWallDestroyed` @ 0x00453240.

### Scan Layer

The wall-connectivity functions scan the **building layer**, not the overlay layer. They call `Look_up_building_in_cell()` to find buildings in adjacent cells — this is a per-cell lookup of the building occupying that cell, not an overlay scan. Laser fence posts and segments are placed as buildings, not as overlay tiles.

### ConnectWalls Logic (0x00452A40)

```c
// Only runs if this building has LaserFencePost flag:
if (g_MapEditorMode == 0 && this->Type[0x16be] != '\0') {
    // Scan 4 cardinal directions (direction offsets 0,2,4,6 % 8):
    for each direction {
        // Find building in adjacent cell:
        building = Look_up_building_in_cell(neighbor_cell);
        // Check if neighbor has LaserFence flag (+0x16BF):
        if (building != null && building->Type[0x16bf] != '\0') {
            // Check same owner and RateTimer phase match:
            if (same_owner && timer_phase == direction) {
                this->LaserFenceFrame |= g_WallConnectionBitmask[dir];
                BuildingClass__AdjustWallConnections(direction, ...);
            }
        }
    }
}
```

Key findings:
- `ConnectWalls` is called **on the fence-post building** (the anchor/tower). It checks its own `LaserFencePost` flag (+0x16BE).
- It then scans neighbors for buildings with `LaserFence` flag (+0x16BF) — the fence wall segments.
- The `LaserFenceFrame` field at **BuildingClass+0x618** (= `param_1[0x186]` where param_1 is int*, byte offset = 0x186 × 4 = 0x618) stores the connectivity bitmask for which directions this post connects.

### RecalculateWallConnections (0x004533A0)

- Checks `param_1[0x148] + 0x16be` (= `BuildingClass+0x520` = Type pointer, then +0x16BE = LaserFencePost).
- Iterates adjacent cells in each direction.
- For each cell, checks if building at that cell has `+0x16BF` (LaserFence).
- If connected, writes frame index to `piVar6[0x186]` (= BuildingClass+0x618, the LaserFenceFrame field).
- Calls `BuildingClass__FindNearestFencePost` to detect online/offline fence-post state.

### ExtendWallInDirection (0x00452DC0)

- Only runs if caller has LaserFencePost flag.
- Scans `g_BuildingTypeClass_Array` linearly to find the first BuildingType with `+0x16BF` (LaserFence) set.
- Uses that type as the wall segment to place in cells between fence posts.
- Does NOT use RulesClass+0x86C..0x87C — those are gate arrays, completely separate.

### OnWallDestroyed (0x00453240)

- When a LaserFence segment is destroyed, checks the post-cell for a LaserFencePost building and calls `AdjustWallConnections` to recalculate.
- Uses direct field access `*(int *)(iVar2 + 0x520)` (BuildingClass+0x520 = Type*) then `+0x16be`/`+0x16bf`.

### BuildingClass+0x618 = LaserFenceFrame

Confirmed by `piVar6[0x186]` assignments in `RecalculateWallConnections` where `piVar6` is `int*`:  
byte offset = 0x186 × 4 = 0x618. This field stores the directional connectivity bitmask (frame index), NOT a simple bool. The WALL_CONNECTION doc's claim "BuildingClass+0x618 = fence frame index" is **CORRECT**.

---

## 4. YR-Active Status

### LaserFencePost / LaserFence Wall Functions

**Active in YR: Conditional (code is live, but no stock building triggers it)**

Caller chain evidence:
- `RecalculateWallConnections` is called from: `BuildingClass__ApplyOfflineEffects`, `BuildingClass__ChangeOwner`, `BuildingClass__GoOnline` (← called from `EventClass__Execute` @ 0x004C6CB0, which is the main game event pipeline), `BuildingClass__OnConstructionComplete`, `BuildingClass__ReadFromINI`, `BuildingClass__RestoreOnlineEffects`, `BuildingClass__Sell`, `BuildingClass__Unlimbo` (all virtual dispatch, confirmed YR-active).
- `ConnectWalls` is called from: `BuildingClass__ChangeOwner`, `BuildingClass__DestructionEffects`, `BuildingClass__Limbo` (virtual dispatch targets, confirmed YR-active building lifecycle).
- `OnWallDestroyed` is called from: `BuildingClass__Unlimbo` (virtual dispatch, YR-active).
- None of these functions are gated behind a TS-specific flag. The early guard `if (g_MapEditorMode == 0 && this->Type[0x16be] != '\0')` is straightforward — it just requires a building with `LaserFencePost=yes`.

**Conclusion:** The wall-connectivity system is live code that will execute whenever a building with `LaserFencePost=yes` is placed. In stock YR, no such building exists in rulesmd.ini. If a modder adds `LaserFencePost=yes` and `LaserFence=yes` to buildings, the system activates immediately. This is **not** TS-gated — it's an unused YR feature, not dead TS legacy.

### FirestormWall Flag (+0x16C0)

**Active in YR: No (unused in stock YR)**

The flag is parsed (`FirestormWall=` INI key at +0x16C0) but no stock YR building sets it. The flag's consumer functions were not found in this investigation scope (searching for reads of +0x16C0 was out of scope). Firestorm Wall is associated with the GDI Firestorm Generator from TS/Firestorm expansion — likely dormant in YR per the TS-legacy rule.

### Gate Array (RulesClass+0x86C..+0x87C)

**Active in YR: Technically active, but effectively neutralized**

The `Cell_passability_building_placement` function reads these 5 slots in normal game operation (building placement logic). However, all 5 are set to `GADUMY` in rulesmd.ini, so only the dummy building type gets the passability bypass benefit. Real gate buildings (GAGATE_A, GAGATE_B, NAGATE_A, NAGATE_B, GACTWR) in YR would NOT get the wall-terrain passability bypass unless the INI keys are restored to their rules.ini values.

---

## 5. Summary of Findings

### Fact Table

| Claim | Status | Evidence |
|-------|--------|----------|
| BuildingType+0x16BE = LaserFencePost | **VERIFIED** | `CCINIClass__ReadBool(…, s_LaserFencePost_0081aa3c, *(param_1 + 0x16be))` @ 0x00460A93 |
| BuildingType+0x16BF = LaserFence | **VERIFIED** | `CCINIClass__ReadBool(…, s_LaserFence_0081aa30, *(param_1 + 0x16bf))` @ 0x00460AA9 |
| BuildingType+0x16C0 = FirestormWall | **VERIFIED** | `CCINIClass__ReadBool(…, s_FirestormWall_0081aa20, *(param_1 + 0x16c0))` @ 0x00460AC7 |
| WALL_CONNECTION doc claim "+0x16BE = FirestormWall" | **WRONG** | FirestormWall is at +0x16C0 |
| GATE_MECHANIC doc claim "+0x16BE = LaserFencePost" | **CORRECT** | Confirmed from binary |
| RulesClass+0x86C = GDIGateOne | **VERIFIED** | `CCINIClass__ReadString(…, s_GDIGateOne_0083c80c, …)` → `*(param_1 + 0x86c)` in ReadGeneral |
| RulesClass+0x870 = GDIGateTwo | **VERIFIED** | Same function, sequential writes |
| RulesClass+0x874 = NodGateOne | **VERIFIED** | Same function, sequential writes |
| RulesClass+0x878 = NodGateTwo | **VERIFIED** | Same function, sequential writes |
| RulesClass+0x87C = WallTower | **VERIFIED** | Same function, sequential writes |
| All 5 set to GADUMY in rulesmd.ini | **VERIFIED** | grep of ini/rulesmd.ini lines 374–379 |
| Wall scan uses building layer, not overlay layer | **VERIFIED** | `Look_up_building_in_cell()` calls in ConnectWalls |
| BuildingClass+0x618 = LaserFenceFrame (connectivity bitmask) | **VERIFIED** | `piVar6[0x186]` writes in RecalculateWallConnections (0x186 × 4 = 0x618) |
| Fence functions are YR-active code | **VERIFIED** | Called through standard building lifecycle vtable dispatch (GoOnline→EventClass__Execute etc.) |

---

## 6. Implications for Rust Port

1. **Don't implement FirestormWall behavior** as LaserFencePost — they are distinct flags at distinct offsets (+0x16BE vs +0x16C0). The WALL_CONNECTION doc was wrong; fix any code that treats +0x16BE as FirestormWall.

2. **The wall-connectivity system is entirely building-layer, not overlay-layer.** Fence posts and segments are placed as buildings. The scan calls `Look_up_building_in_cell()` — implement as a building-cell lookup, not an overlay scan.

3. **LaserFenceFrame field is BuildingClass+0x618** — a directional bitmask (not bool). It controls which frame to draw for the fence post (showing connected directions).

4. **The gate/wall-tower passability logic (RulesClass+0x86C..+0x87C)** uses 5 BuildingType pointers read from `[General]` section. In YR these all point to GADUMY so the passability bypass effectively doesn't apply to any real gate building. Implement as pointer comparisons, not index comparisons.

5. **No stock YR building uses LaserFencePost or LaserFence** — the system is inert in vanilla play. It is not TS-legacy (not gated behind TS-only flags), but it requires modder opt-in via INI. Implement it correctly but it won't fire in vanilla tests.

6. **FirestormWall (+0x16C0) consumers were not located in this investigation** — skip for now, likely TS-legacy (Firestorm expansion only).
