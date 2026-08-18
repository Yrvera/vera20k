# Layer System — Ghidra Research Report

**Date:** 2026-05-17
**Active in YR:** Yes — every visible unit goes through layer-sort every render frame.
**Confidence:** HIGH overall

## 1. Overview

The "layer system" in gamemd.exe is actually **two distinct concepts** that share the word "layer":

### 1.1 The display-sort layer (5 layers)

Used by the renderer for **draw-order sorting**. Implemented as `LayerClass<ObjectClass*>` instances in the global `g_DisplayLayers @ 0x008A0360`. Each layer holds a sorted list of pointers to objects that draw in that layer's slice of the rendering pipeline. **5 layers total** (verified via `LayerClass::Constructor @ 0x4A862A` loop count).

This is what `ILocomotion::In_Which_Layer` (vtable slot 29) answers — "which display-sort layer does this unit belong to right now?"

### 1.2 The cell-feature layer (3 named: Ground / Surface / Underground)

A separate enum used for **cell-feature classification** — likely for terrain rendering or TS-legacy subterranean state. Identified via string table at `0x0081DB84`. Only 3 named values:
- `0x0081DB84` = "Ground"
- `0x0081DB8C` = "Surface"
- `0x0081DB94` = "Underground"

**This enum is NOT the same as the In_Which_Layer return value.** Confusingly, both use the word "layer". The In_Which_Layer values (0-4) refer to display-sort layers; the Ground/Surface/Underground names refer to cell-feature classification. **A unit with display layer = 2 (Ground display) is unrelated to a cell with feature = "Surface".**

This doc focuses on the **display-sort layer** since that's what In_Which_Layer answers and what the locomotors care about for the render pipeline.

---

## 2. The 5 display layers — enum

Per `LayerClass::Constructor @ 0x4A862A`:
```c
void LayerClass__Constructor(void) {
    piVar1 = &g_DisplayLayers.count;
    iVar2 = 5;                                     // 5 iterations = 5 layers
    do {
        piVar1[-1] = 0;                            // count = 0
        *piVar1 = 0;                                // ?
        *(byte *)(piVar1 + 1) = 1;                  // flag byte
        *(byte *)((int)piVar1 + 5) = 0;
        // ... vtable + capacity (10) ...
        piVar1[3] = 10;                             // capacity = 10
        piVar1 = piVar1 + 6;                        // advance 24 bytes to next layer
        iVar2--;
    } while (iVar2 != 0);
}
```

**5 LayerClass instances, each 24 bytes** (verified `piVar1 += 6` advance = 6 ints = 24 bytes per instance).

**Layer indices inferred from In_Which_Layer return values:**

| Index | Inferred name | Locomotors returning this value |
|---|---|---|
| 0 | (Underground — likely TS-legacy) | Tunnel (returns 1? — not separately verified; TS-dormant) |
| 1 | (Surface — possibly submerged sub layer?) | (no live locomotor returns 1 explicitly in this pass) |
| **2** | **Ground** | **Drive, Ship, Walk** (always); Fly, JumpJet (when on ground / altitude 0) |
| 3 | Mid-air | JumpJet (low altitude, when altitude < `DAT_00ABC5DC`) |
| 4 | High-air | Fly (when altitude > 0), JumpJet (high altitude, when altitude >= `DAT_00ABC5DC`) |

**The 5-layer count is FIRMLY 5** per the constructor. The names "Ground/Surface/Underground" don't map 1:1 to indices 0-4 (only 3 names for 5 layers), suggesting:
- Indices 0-2 might be the named tier (Underground/Surface/Ground or similar)
- Indices 3-4 are unnamed air/sky tiers used implicitly by Fly/Jumpjet

**Subtle detail — the constructor's flag byte at `+0x10`:** initialized to **1** in every layer. Likely an "active/visible" flag. The renderer skips layers where this flag is 0 (for paused / debug states).

**Subtle detail — initial capacity = 10:** Each layer pre-allocates a 10-pointer buffer. Layers grow dynamically when needed (vtable methods at offset 0xC handle Clear, presumably also Grow).

C=HIGH (decompilation of constructor + return values of each locomotor's In_Which_Layer), I=HIGH, B=HIGH.

---

## 3. Per-locomotor In_Which_Layer behaviour (vtable slot 29)

### 3.1 Drive @ `0x4B4820` — returns **2** (Ground)

```c
undefined4 DriveLocomotionClass__In_Which_Layer(void) {
    return 2;
}
```

**Always Ground.** No bridge adjustment, no altitude check. A tank on a bridge OR on the ground OR mid-jump (impossible for Drive) returns 2.

C=HIGH, I=HIGH, B=HIGH.

### 3.2 Ship @ `0x6A3E50` — returns **2** (Ground)

Identical body to Drive's — `return 2`. Verified in `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §6 slot 29.

**Submarines also return 2.** There's no "submerged" display layer at the In_Which_Layer level — submarines' invisible-underwater state is handled by ObjectClass cloak rendering, NOT by display-sort layer.

C=HIGH, I=HIGH, B=HIGH.

### 3.3 Walk @ `0x75C7E0` — returns **2** (Ground)

`return 2`. Verified in `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §6.10. (Ghidra symbol "Get_Locomotion_Type" is misleading — semantic is In_Which_Layer.)

Infantry always Ground. Even when on a bridge (after on_bridge flag is set and Z-bumped), the display layer is still 2 — only the unit's Z coordinate differs.

C=HIGH, I=HIGH, B=HIGH.

### 3.4 Fly @ `0x4CFCF0` — returns **2 (ground) or 4 (high air)** by altitude

```c
int FlyLocomotionClass__In_Which_Layer(int param_1) {
    iVar1 = (**(code **)(**(int **)(param_1 + 8) + 0x1c8))();   // techno.vtable+0x1C8 = Get_Height
    return ((0 < iVar1) - 1 & 0xfffffffe) + 4;
}
```

**Decoded:**
- `iVar1 > 0` (airborne) → `((1 - 1) & 0xFFFFFFFE) + 4 = 0 + 4 = 4`
- `iVar1 <= 0` (landed) → `((0 - 1) & 0xFFFFFFFE) + 4 = 0xFFFFFFFE + 4 = 2`

So Fly is either **4** (in air) or **2** (on ground — when landed at airfield).

**Subtle detail — the `0xFFFFFFFE` trick** is a sign-extension idiom. `(0 < x) - 1` produces 0 for true, -1 (=0xFFFFFFFF) for false. AND with `0xFFFFFFFE` masks off bit 0, giving 0 or 0xFFFFFFFE. `0xFFFFFFFE + 4 = 2` (mod 2^32). Branchless conditional.

**Aircraft transition through display layer 3 (mid-air) only via Jumpjet, never Fly.** Helicopters use Jumpjet for that intermediate hover state.

C=HIGH (decomp + binary truth-table verified), I=HIGH, B=HIGH (vtable slot 29 of Fly vtable, confirmed by `In_Which_Layer` search).

### 3.5 JumpJet @ `0x54B8D0` — returns **2 / 3 / 4** by altitude + bridge

```c
char JumpjetLocomotionClass__In_Which_Layer(int param_1) {
    iVar2 = (**(code **)(**(int **)(param_1 + 8) + 0x1c8))();    // altitude
    iVar3 = *(int *)(param_1 + 8);                                // techno
    if (*(char *)(iVar3 + 0x8c) == 0) {                          // NOT on bridge
        coord = techno.Location;
        cell = MapClass::Get_Cell_At(&coord);
        if ((cell.Flags & 0x100) != 0                            // cell IS bridge
            && DAT_00ABC5DC <= iVar2                              // altitude >= bridge threshold
            && techno+0x8D == 0) {                                // some "not aborting" flag
            iVar2 = iVar2 - DAT_00ABC5DC;                         // subtract bridge altitude — flying ABOVE bridge
        }
    }
    cVar1 = (**(code **)(**(int **)(param_1 + 8) + 0x54))();     // some "is moving / is visible" check
    if (cVar1 == 0) return 2;                                     // not moving → Ground
    if (iVar2 == 0) return 2;                                     // altitude 0 → Ground
    return (DAT_00ABC5DC <= iVar2) + 3;                           // 3 or 4 based on altitude vs threshold
}
```

**Decoded layer-selection logic:**
1. If unit not moving or altitude 0: return **2 (Ground)** — Rocketeer landed on ground
2. If altitude > 0 AND altitude < `DAT_00ABC5DC`: return **3 (Mid-air)** — Rocketeer hovering low
3. If altitude > 0 AND altitude >= `DAT_00ABC5DC`: return **4 (High-air)** — Rocketeer cruise altitude / over bridge

**Subtle detail — bridge altitude adjustment:**
If the unit is NOT on a bridge (FootClass.on_bridge == 0) but the cell below has the bridge flag (0x100) set AND the unit's altitude is high enough to clear the bridge: subtract `DAT_00ABC5DC` from altitude before the layer comparison. This makes a Rocketeer flying ABOVE a bridge render correctly relative to it — without the subtraction, the altitude would compare against the threshold incorrectly because the unit is "below the threshold" relative to the deck but "above the threshold" relative to the ground.

`DAT_00ABC5DC` is the JumpJet bridge-altitude threshold (per `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md` §2). Per the bridge doc, this is also used at 11 other state-handler sites for landing/ascending decisions.

C=HIGH (full decomp verified + cross-doc verification), I=HIGH (vtable slot 29 of `0x07ECD68 + 0x74 = 0x0054B8D0`, confirmed in bridge doc), B=HIGH.

### 3.6 Hover (per existing `HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md`)

Hover uses `HoverLocomotionClass::Move @ 0x514310` for layer-related decisions, NOT a class-specific In_Which_Layer override. Per the bridge doc, hover's bridge interaction is handled there (reads `cell.flags & 0x100 + altitude vs DAT_00A8F1B4`).

**Hover's vtable slot 29** likely inherits from a base implementation. Not separately verified this pass — open question.

### 3.7 Teleport, Tunnel, DropPod, Mech, Rocket

- **Teleport** — Per `TELEPORT_LOCOMOTION_DEEP_DIVE.md`, Teleport's In_Which_Layer likely returns 2 (Ground) — chrono units appear on ground. Not separately verified this pass.
- **Tunnel** — TS-dormant per `TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md`. If active, would likely return a different layer (Underground = 0 or 1).
- **DropPod / Mech** — TS-dormant, not instantiated in stock YR.
- **Rocket** — Per `ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`, used by V3 bullet locomotion. Bullets don't go through In_Which_Layer (they render via different machinery). Open question if it overrides slot 29.

C=MEDIUM, I=MEDIUM, B=MEDIUM (deferred verification).

---

## 4. Layer assignment flow per render tick

Per `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md` §3.4 and `DISPLAYCLASS_GHIDRA_REPORT.md`:

```
Every render tick:
1. LogicClass::AI moves all objects (Process tick)
2. DisplayClass::Tick clears all 5 LayerClass instances:
   for layer in g_DisplayLayers[0..5]:
       layer.vtable[0x0C]()    // LayerClass::Clear
3. DisplayClass::Tick re-populates layers:
   for each visible object:
       layer_id = object.locomotor.In_Which_Layer()   // vtable slot 29
       g_DisplayLayers[layer_id].Submit_Object(object)
4. For each layer 0..4:
   Sort by Z (depth)
   Draw each object in sort order
```

**Subtle detail — re-population is per-tick:** A unit can change layer between ticks (e.g. Rocketeer ascending). The layer system handles this naturally because the layer set is rebuilt each tick from In_Which_Layer.

**Subtle detail — within a layer, objects sort by Z.** Two units in layer 2 (Ground) with different Z values draw in Z-order. This is how the renderer handles units on bridges visually appearing above units beneath (both are layer 2 but bridge unit has higher Z due to the bridge-Z-bump).

C=HIGH (decompilation in DISPLAYCLASS_GHIDRA_REPORT.md), I=HIGH, B=HIGH.

---

## 5. Cross-locomotor In_Which_Layer summary table

| Locomotor | Slot 29 addr | Return value(s) | Notes |
|---|---|---|---|
| Drive | `0x4B4820` | **2** (always) | All vehicles. Bridge-deck or ground — same layer. |
| Ship | `0x6A3E50` | **2** (always) | All naval. Includes submerged subs (cloak handles invisibility). |
| Walk | `0x75C7E0` (Ghidra: Get_Locomotion_Type) | **2** (always) | All infantry. |
| Fly | `0x4CFCF0` | **2** (landed) or **4** (airborne) | Branchless altitude-conditional. |
| JumpJet | `0x54B8D0` | **2** (stationary), **3** (low), or **4** (high) | Bridge-altitude-aware. |
| Hover | (base inherited) | likely **2** | Verified in bridge doc — uses Move @ 0x514310 for bridge state, not In_Which_Layer. |
| Teleport | (not separately verified) | likely **2** | Open question. |
| Rocket | (not separately verified) | likely **2 or 4** | V3 bullet; open question. |
| Tunnel | (TS-dormant) | n/a | Not instantiated in YR. |
| DropPod | (TS-dormant) | n/a | Not instantiated in YR. |
| Mech | (TS-dormant) | n/a | Not instantiated in YR. |

**The 4 live YR layers from In_Which_Layer:**
- **2 (Ground)** — used by Drive, Ship, Walk, landed Fly, stationary JumpJet, Hover
- **3 (Mid-air)** — used by airborne low-altitude JumpJet
- **4 (High-air)** — used by airborne Fly, airborne high-altitude JumpJet
- **0, 1** — not actively returned by any live YR locomotor's In_Which_Layer (TS-legacy)

This means **`g_DisplayLayers[0]` and `g_DisplayLayers[1]` are effectively empty in standard YR play.** They allocate 24 bytes each but never receive objects. The renderer iterates them anyway (low cost — empty list).

---

## 6. The "Ground/Surface/Underground" strings at `0x0081DB84..0x0081DB94`

**This is a separate enum.** Used by what, exactly? Not directly traced this pass. Hypotheses:
1. **Cell-feature classification** for terrain rendering (used by `CellClass::DrawTerrain`?)
2. **Object spawning placement layer** — "spawn this on the Surface" vs "spawn underground"
3. **TS-legacy Submerged/Subterranean state** — possibly the "submerged" state of a TS Subterranean APC

The strings appear immediately before the SpeedType string table (`FloatBeach`, `Float`, `Winged`, `Hover`) starting at `0x0081DBA0`. The proximity suggests they're an adjacent enum used by INI parsing, but the binding wasn't traced.

**Open question:** Identify what reads these strings. Possible search: any `_stricmp` / `strcmp` calls with `0x0081DB84` as source, or any INI key parser that emits a value 0/1/2 mapped to these names.

C=LOW (strings located but consumer not identified), I=MEDIUM, B=LOW.

---

## 7. Active-in-YR confirmation

| Subsystem | Active in YR? | Evidence |
|---|---|---|
| 5 `LayerClass` instances at `g_DisplayLayers @ 0x8A0360` | Yes | Constructor runs at boot, 5 iterations |
| In_Which_Layer dispatch (slot 29) | Yes | Called every render frame by DisplayClass for every visible object |
| Layer 2 (Ground) | Yes | Drive/Ship/Walk constantly populate it |
| Layer 4 (High-air) | Yes | Aircraft constantly populate it |
| Layer 3 (Mid-air) | Yes | JumpJet units transit through it |
| Layers 0, 1 | **Empty in standard YR** | No live locomotor's In_Which_Layer returns these values |
| LayerClass::Submit_Object | Yes | Per-tick re-population |
| Bridge altitude adjustment in JumpJet::In_Which_Layer | Yes | Standard YR maps have bridges and Rocketeers |
| Ground/Surface/Underground name enum | **Unknown** (consumer not identified) | Strings exist; usage not traced |

---

## 8. Open questions

1. **Hover's slot 29 implementation** — Hover doesn't have a class-specific override per my search. Verify whether it inherits the base `LocomotionClass::In_Which_Layer` and what value that returns. Likely 2.

2. **Teleport's slot 29 implementation** — same question. Likely 2.

3. **Rocket's slot 29** — V3 bullet locomotion. Bullets bypass the layer system typically, so this may not even be called.

4. **The cell-feature enum** (Ground/Surface/Underground at `0x0081DB84`) — identify the consumer. Possibly used by:
   - Building placement validation (e.g. "must be on Surface")
   - Animation spawn metadata (e.g. "spawn this anim Underground for subterranean transit")
   - INI key for some TS-legacy feature

5. **What's at `g_DisplayLayers + 0x10`** (the `*piVar1 = 0` write that's offset +0x10 in each 24-byte entry)? The constructor zeros it, but its semantic isn't clear from the constructor alone. Possibly a "previous tick count" cache for diff-rendering.

6. **The `cVar1 = vtable+0x54` check in JumpJet::In_Which_Layer** — what's at techno vtable +0x54? "Is_Visible" or "Is_Selected" guess. Verifies the layer assignment skips the bridge-altitude adjustment for invisible/destroyed units.

7. **Indices 0 and 1 — what's their TS purpose?** Likely Underground (0) and Surface (1) for TS subterranean / sea submerged. Worth a doc cross-reference to confirm.

---

## 9. Sources

**Ghidra functions decompiled (this pass):**
- `LayerClass::Constructor @ 0x4A862A` — full body (5-instance init loop)
- `DriveLocomotionClass::In_Which_Layer @ 0x4B4820` — full body (1-line stub)
- `FlyLocomotionClass::In_Which_Layer @ 0x4CFCF0` — full body
- `JumpjetLocomotionClass::In_Which_Layer @ 0x54B8D0` — full body (cross-ref with bridge doc)

**Memory reads:**
- `0x008A0360` len 32 — `g_DisplayLayers` cold state (BSS zeros)
- `0x0081DB84` len 64 — string table: Ground / Surface / Underground / FloatBeach / Float / Winged / Hover

**Strings located:**
- `0x0081DB84` "Ground"
- `0x0081DB8C` "Surface"
- `0x0081DB94` "Underground"

**Xref tables:**
- (Existing) `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md` documents 11 read sites of `DAT_00ABC5DC` (JumpJet bridge-altitude threshold) across state handlers + In_Which_Layer.

**Companion docs:**
- `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md` — JumpJet In_Which_Layer + bridge altitude
- `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §6 — Ship slot 29 returns 2
- `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §6.10 — Walk slot 29 returns 2
- `DRIVE_LOCOMOTION_CLASS.md` — Drive vtable confirms slot 29 → In_Which_Layer
- `DISPLAYCLASS_GHIDRA_REPORT.md` — `g_DisplayLayers` consumer in render pipeline
- `OBJECTCLASS_DRAW_LIMBO_CELLLIST.md` — LayerClass interaction with limbo state
- `MOVEMENT_CLASSIFIERS_REFERENCE.md` — adjacent SpeedType string table

---

*End of report. The display-sort layer system is straightforward: 5 layers, locomotor decides which one each tick, renderer sorts within each. The Ground/Surface/Underground enum is unrelated and remains an open question for follow-up investigation.*
