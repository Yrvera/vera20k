# Bridge Locomotor — JumpJet & Hover — Ghidra Research Report

**Phase:** Phase 3 of approved plan `docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`
**Plan items covered:** #24 (JumpJet bridge interaction via vtable), #25 (HoverLocomotionClass::Move + SpeedUpdate bridge layer reads)
**Companion docs:** `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md`
**Phase 1+2 dependencies:** `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`, `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
**Date:** 2026-05-13
**Active in YR:** **Yes for both** — JumpJet drives Rocketeer/Siege Chopper/Hornet (9 INI declarations), Hover drives Robot Tank/LCRF/SAPC/YHVR (4 active INI declarations + 1 commented).

> Every claim cites a Ghidra address + decompilation excerpt or `read_memory` byte dump.
> Confidence axes: **C**=content / **I**=identity / **B**=binding (caller path verified).

---

## 1. Overview — air locomotors don't follow the bridge layer

Both JumpJet and Hover are **non-following** layers — they do NOT join `cell.AltObject` (+0xE8) occupancy lists, and they do NOT add `g_BridgeZOffset_*` to their destination Z. They float at their own altitude regimes.

What they **do** with bridge data is purely **layer-sort and on_bridge-flag bookkeeping**:

| Locomotor | Bridge-cell read? | Z-offset added? | `FootClass+0x8C` (on_bridge) updated? | Layer-sort (`LayerClass`) adjusted? |
|-----------|-------------------|------------------|---------------------------------------|------------------------------------|
| **JumpJet** | Yes — In_Which_Layer @ 0x54B8D0 | **No** | **No** (only reads it) | Yes — altitude offset by `DAT_00ABC5DC` for sort decision |
| **Hover** | Yes — Move @ 0x514310 | **No** | **Yes** — set/cleared at runtime per cell transition | No (uses its own bob/dampen system) |

The asymmetry is by design:
- **JumpJet** flies in the air-layer (`TopLow` / `TopHigh`). The bridge's effect is to push the unit "down" one bridge-deck level for sort-tie purposes — i.e., when a Rocketeer flies right over a bridge deck, the deck is treated as if it were the ground floor for the purpose of choosing which sprite layer the Rocketeer renders in.
- **Hover** hugs the terrain — the deck IS the ground when it passes over. So it sets `on_bridge = 1` so that subsequent passes through the cell-occupancy system know the unit is "on the deck" rather than "on the ground below the bridge". But Hover does NOT add a Z-offset because the visual altitude is decoupled from the deck via the bob math (Robot Tank floats a few pixels above whatever surface it's on).

---

## 2. JumpJet — In_Which_Layer @ `0x54B8D0` (Item #24)

This is the **sole site** where JumpJet reads `cell.flags & 0x100`. The Constructor (`0x54AC40`), `Process` (`0x54AEC0`), `Set_Destination` (`0x54B1C0` via vtable), and `Do_Turn` (`0x54B4D0` via vtable) do NOT read bridge state.

### 2.1 Full decompilation

`param_1` is the ILocomotion sub-object (= instance + 0x04) because In_Which_Layer is dispatched via ILocomotion vtable slot 29 (offset 0x74 in the vtable @ 0x07ECD68 + 0x74). Therefore `[param_1 + 8]` reads instance+0x0C = LinkedTo TechnoClass.

```c
char FUN_0054B8D0(int param_1) {   // (Ghidra-unlabeled; this is JumpjetLocomotionClass::In_Which_Layer)
    char    cVar1;
    int     iVar2;
    int     iVar3;
    CoordStruct unit_coord;

    // Step 1: get unit altitude via vtable+0x1C8
    iVar2 = (**(code **)(**(int **)(param_1 + 8) + 0x1c8))();   // = unit altitude

    iVar3 = *(int *)(param_1 + 8);                              // LinkedTo*
    if (*(char *)(iVar3 + 0x8c) == 0) {                         // ON_BRIDGE flag is 0 (unit not flagged as on bridge)
        unit_coord = LinkedTo.Coord (+0x9C..+0xA4);
        iVar3 = CellClass__Get_Cell_At(&unit_coord);
        if (((cell.Flags (+0x140) & 0x100) != 0)                // cell IS bridge cell
            && (DAT_00abc5dc <= iVar2)                          // unit altitude >= bridge-altitude threshold
            && (*(char *)(LinkedTo + 0x8D) == 0))               // a SEPARATE byte at +0x8D (not on_bridge)
        {
            iVar2 = iVar2 - DAT_00abc5dc;                       // ALTITUDE -= bridge_height (for sort calc only)
        }
    }
    cVar1 = (**(code **)(**(int **)(param_1 + 8) + 0x54))();    // vtable+0x54 = Is_Visible?
    if (cVar1 == 0) return 2;                                   // → Ground layer (2)
    if (iVar2 == 0)  return 2;                                  // grounded → Ground layer
    return (char)((iVar2 >= *(int *)(param_1 + 0x28)) + 3);     // 3 if below cruise alt, 4 if at/above
}
```

### 2.2 The three-way return code

- **2** = Ground layer (rendered with terrain). Used when invisible OR altitude == 0.
- **3** = TopLow layer (low air). Returned when altitude > 0 AND altitude < cached_cruise_alt.
- **4** = TopHigh layer (high air). Returned when altitude >= cached_cruise_alt.

The cached cruise altitude is at `*(param_1 + 0x28)` = instance + 0x2C — **this is the field with the open question** in `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md §11.1` (CruiseHeight cache vs Speed cache). In_Which_Layer reads it as an altitude comparator, so under the "CruiseHeight cache" hypothesis it makes sense; under the "Speed cache" hypothesis it would be a coincidence. **This report leaves the open question unresolved** — it is item-#24 scope to identify bridge interaction, which IS confirmed here regardless.

### 2.3 Bridge interaction details

**The altitude adjustment** `iVar2 = iVar2 - DAT_00abc5dc` is a **pure layer-sort tweak**, NOT a position change. The unit's actual altitude in world space is unchanged. The function returns a different layer code because the **subtracted altitude** is compared against the cruise threshold. Effect: a Rocketeer that is `altitude = bridge_height + 50` flying over a bridge deck is reported as `altitude = 50` for sort purposes — which puts it in TopLow (= 3) instead of TopHigh (= 4) only if 50 < cruise.

The three gates:
1. `FootClass+0x8C (on_bridge) == 0` — if the unit is already flagged on_bridge, the adjustment is SKIPPED. (This means a Rocketeer dropping onto a bridge cell from below would skip the adjustment until it lands.)
2. `cell.Flags & 0x100` (cell IS bridge) — without this, no adjustment.
3. `DAT_00ABC5DC <= altitude` — altitude must be AT or ABOVE the bridge-deck height. Sub-deck altitudes don't trigger.
4. `LinkedTo+0x8D == 0` — separate byte flag. Likely "in transition" or "on landing pad". When set, skip the adjustment.

### 2.4 DAT_00ABC5DC — the JumpJet bridge altitude threshold

Read sites (via `get_xrefs_to 0x00abc5dc`):

```
From 0054d314 in FUN_0054d0f0   [READ]   (Process_Movement helper)
From 0054d3f6 in FUN_0054d0f0   [READ]   (Process_Movement helper)
From 0054d875 in FUN_0054d820   [READ]   (state-5/abort helper)
From 0054d906 in FUN_0054d820   [READ]   (state-5/abort helper)
From 0054d771 in FUN_0054d6d0   [READ]   (state helper)
From 0054d7f4 in FUN_0054d6d0   [READ]   (state helper)
From 0054b657 [READ]                       (Process — vtable-slot 16 body, indirect bridge reference)
From 0054b926 in FUN_0054b8d0   [READ]   (← THIS function, In_Which_Layer)
From 0054ba85 in FUN_0054ba30   [READ]   (state-1/ascend helper)
From 0054c7fa in FUN_0054c550   [READ]   (state-4/land helper)
From 0054cb4f in FUN_0054ca90   [READ]   (state-5/abort helper)
From 0054abc0 [WRITE]                     (init site — outside any labeled function)
```

11 read sites across most state handlers — meaning **JumpJet's bridge-altitude threshold is consulted during every state transition**, not only In_Which_Layer. The state handlers use it for landing/ascending vertical-clearance checks.

The WRITE site at `0x54ABC0` is **not in any labeled function** but is near the constructor. Likely an inline init that runs after the JumpJet vtable is registered.

### 2.5 What JumpJet does NOT do with bridges

- Does **not** add any Z-offset to its destination coord (no `Set_Destination + bridge bump` mirror).
- Does **not** set `FootClass+0x8C` (on_bridge flag) — only reads it. The flag is set by ground locomotors (Drive/Walk) when they land on a bridge; JumpJet, while in flight, never lands on a bridge layer.
- Does **not** participate in the dual-occupancy list (`cell.+0xE4` ground vs `cell.+0xE8` bridge). JumpJet stores its occupancy in `cell.+0xE0` (Jumpjet list — see Phase 2 `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` §2). Bridge state has no effect on Jumpjet occupancy.

### 2.5b JumpJet::Process @ `0x54AEC0` — **freshly decompiled (cleanup pass)** — confirmed bridge interaction is indirect

Original draft asserted "The Constructor, Process, Set_Destination, and Do_Turn do NOT read bridge state" without verifying Process directly. Cleanup pass decompiled the full Process body (~0x2D0 bytes from 0x54AEC0 to LAB_0054B193 at function exit).

**Direct reads of `cell.flags & 0x100` in Process: zero.**
**Direct reads of `DAT_00ABC5DC` (JumpJet bridge threshold) in Process: zero.**

But Process **DOES dispatch to state handlers** via a jump table (state 0-5 → FUN_0054B980 / B/A30 / BD30 / BFF0 / C550 / CA90), and 9 of those state-handler entry sites read `DAT_00ABC5DC` per the xref list documented in §2.4. So bridge state IS consulted, but **transitively via state handlers**, not directly in Process.

**Newly found TS-legacy gate in Process:**

```c
if ((*(char *)(iVar5 + 0x83) != '\0') && (*(int *)(iVar5 + 0x21c) != g_PlayerPtr)) {
    // ... coord setup ...
    cVar4 = IsShrouded();
    if (cVar4 == '\0') {
        if ((*g_ScenarioClass_Instance & 0x1000) != 0) {       // ← FOG-OF-WAR GATE
            // ... coord setup ...
            cVar4 = FUN_005865e0();                              // ← fog-revealed check
            if (cVar4 != '\0') goto LAB_0054b107;
        }
    }
    else {
LAB_0054b107:
        (**(code **)(*(int *)param_1[2] + 0x150))();              // vtable+0x150 (reveal/redraw?)
    }
}
```

The condition `*g_ScenarioClass_Instance & 0x1000` is the **fog-of-war SpecialFlag bit (0x1000)**. Per CLAUDE.md and the project's MEMORY:

> Fog of war — `[MultiplayerDialogSettings] FogOfWar` defaults to `false` in YR. The semi-transparent darkening of "previously seen but not visible" cells is NOT used in standard YR. All fog-specific code is gated behind `SpecialFlags & 0x1000`.

So this entire branch is **TS-legacy / dormant in standard YR**. The `IsShrouded()`-without-fog path also takes the LAB_0054B107 jump, so the shroud path IS live — but the fog path is dead.

This gate was present in the prior JUMPJET_LOCOMOTION_CLASS doc §10 (which correctly flagged "Fog-of-war branch in Process step 8 (SpecialFlags & 0x1000) — per CLAUDE.md, fog defaults OFF in YR. This path is dormant.") but the cleanup pass independently re-verified it from fresh decompilation.

Confidence after cleanup: **C=HIGH (full body decompiled), I=HIGH, B=HIGH.**

### 2.5c JumpJet::Set_Destination @ `0x54B1C0` — entry-point verified

Original draft asserted "Set_Destination does NOT read bridge state". Ghidra does not label `0x54B1C0` as a function (auto-analysis miss), but raw memory read at that address shows a function prologue:

```
0054b1c0: 83 ec 10                ; SUB ESP, 0x10
0054b1c3: 8b 15 a8 c5 ab 00       ; MOV EDX, [0x00ABC5A8]   ← NullCoord_Jumpjet_X
0054b1c9: 53 55 56                ; PUSH EBX/EBP/ESI
0054b1cc: 8b 74 24 20             ; MOV ESI, [ESP+0x20]
0054b1d0: 57                      ; PUSH EDI
0054b1d1: 8b 4e 3c                ; MOV ECX, [ESI+0x3C]     ← reads instance+0x40 = dest X
0054b1d4: 8d 46 3c                ; LEA EAX, [ESI+0x3C]
0054b1d7: 3b ca                   ; CMP ECX, EDX
0054b1d9: 75 1a                   ; JNZ +0x1A
...                              ; NullCoord guards continue
```

**First 64 bytes show only NullCoord guards** (comparing each of X/Y/Z against `g_NullCoord_Jumpjet_*` at `0x00ABC5A8/AC/B0`). **No `cell.flags & 0x100` read in the entry block.** The function body past 64 bytes wasn't fully analysed in this cleanup (Ghidra refused to decompile because it doesn't recognise this as a function start), but the structural intent is clear: JumpJet's Set_Destination just stores the dest coord with NullCoord short-circuits.

Confidence after cleanup: **C=MEDIUM (entry verified, body not exhaustively read), I=MEDIUM (function exists but unlabeled in Ghidra DB), B=HIGH (vtable slot 17 of JumpJet vtable points here).**

A full upgrade to HIGH would require either Ghidra-labeling the function with `create_function 0x54B1C0` and re-decompiling, or reading the full byte range and decoding manually. Deferred to follow-up.

### 2.6 Caller binding

In_Which_Layer is at ILocomotion vtable slot 29 of JumpjetLocomotionClass (vtable @ `0x007ECD68` + slot 29×4 = +0x74; address read = `0x0054B8D0`). Confirmed by `read_memory 0x007ECD68 length 128` showing the slot at offset 0x74 holds `0x0054B8D0`.

Active in YR: **Yes** — called by `LayerClass::Submit_Object` and similar render pipeline functions every tick for every visible Rocketeer/Siege Chopper/Hornet/Allied paradrop carrier.

Confidence: C=HIGH, I=HIGH (vtable slot resolved by memory read), B=HIGH (slot owner is the JumpJet vtable, single-class binding).

---

## 3. Hover — Move @ `0x514310` (Item #25)

The per-tick driver. ~2.2 KB body. The bridge-relevant region is concentrated in the **cell-transition phase** at the end of a successful step.

### 3.1 The on_bridge transition site

Located in the position-step block (after a successful `vtable+0x1B4` Set_Coord call):

```c
iVar9 = CellClass__Get_Cell_At();         // new cell after step

if (((char)((int *)param_1[2])[0x23] == 0)               // FootClass+0x8C (on_bridge) is currently 0
    && (((*(uint *)(iVar9 + 0x140) & 0x100) != 0          // new cell IS bridge cell
        && (iVar10 = (**(code **)(*(int *)param_1[2] + 0x1c8))(),
            DAT_00a8f1b4 <= iVar10))))                    // unit altitude >= Hover bridge threshold
{
    *(undefined1 *)(param_1[2] + 0x8c) = 1;               // SET on_bridge = 1
}

if ((*(char *)(param_1[2] + 0x8c) == 1)                   // on_bridge is now (or was) 1
    && ((*(uint *)(iVar9 + 0x140) & 0x100) == 0))         // new cell NOT bridge
{
    *(undefined1 *)(param_1[2] + 0x8c) = 0;               // CLEAR on_bridge = 0
}
```

### 3.2 The transition table

| Currently `on_bridge` | `new.flags & 0x100` | `Get_Height() >= DAT_00A8F1B4`? | After |
|----------------------|----------------------|----------------------------------|-------|
| 0 (no) | clear (no bridge) | (don't care) | **0** (unchanged) |
| 0 | set (bridge cell) | **no** | **0** (unchanged — below threshold) |
| 0 | set | **yes** | **1** (SET) |
| 1 (yes) | clear (no bridge) | (don't care) | **0** (CLEAR) |
| 1 | set | (don't care) | **1** (unchanged) |

**Subtle:** when `on_bridge` is already 1 AND new cell is bridge AND altitude drops below the threshold (`Get_Height() < DAT_00A8F1B4`), the flag stays 1. The first `if` only SETS when on_bridge was 0; it never SETS when on_bridge was 1 — and the second `if` only CLEARS when leaving bridge cells. So Hover's `on_bridge` flag is **one-way upward** at a single threshold but only clears at a cell-flag transition.

### 3.3 `DAT_00A8F1B4` — the Hover bridge altitude threshold

```
read_memory 0x00A8F1B4 length 16 → all zeros (BSS, runtime-init)
get_xrefs_to 0x00A8F1B4 →
  From 00514939 in HoverLocomotionClass__Move        [READ]   ← THIS site
  From 005153cd in FUN_00514f70                       [READ]   (StartNextStep)
  From 00514e2b [READ]                                          (mid-Move adjacent code)
  From 00516f56 [READ]                                          (another Hover helper)
  From 00513ba0 [WRITE]                                          (init site)
```

5 read sites, 1 write site. The Hover threshold is consulted at multiple sites in Move, FUN_00514F70, and elsewhere — meaning Hover's bridge-state evaluation is more pervasive than just the on_bridge transition. Other reads relate to step-direction validation and arrival detection. The WRITE site at `0x513BA0` is not in any labeled function (inline init in the constructor's preamble or the post-construction class registration code).

### 3.4 `DAT_00A8F1C0` — the Hover "force float up" threshold (NOT bridge-related)

Used in Move at the very end:

```c
iVar9 = CellClass__GetGroundHeight(&unit_coord);
if (unit.Z < iVar9 + DAT_00a8f1c0) {                       // below threshold above ground
    (**(code **)(*(int *)param_1[2] + 0xec))();             // vtable+0xEC = Force_Float_Up
}
```

Only fires when `cell.LandType (+0xEC) == 2` (water). This makes the Hover skirt lift the unit clear of water surface — has nothing to do with bridges, but is the adjacent constant. Worth noting because the two constants `0x00A8F1B4` and `0x00A8F1C0` differ by 12 bytes (one is `+0xC` from the other) — they may be part of a struct laid out as `{bridge_alt_thresh, low_water_thresh, ...}`. Not pursued further in this phase.

### 3.5 What Hover does NOT do with bridges

- Does **not** add bridge_offset to destination Z. The Hover has no `Set_Destination`-style hook with a Z-bump.
- Does **not** participate in the bridge occupancy list. Hover joins `cell.+0xE4` (ground list).
- Does **not** have a `Compute_BridgeZOffset` style init function (the value `DAT_00A8F1B4` is an altitude THRESHOLD, not a Z-OFFSET added to coord).

### 3.6 SpeedUpdate @ `0x515ED0` — **freshly decompiled (cleanup pass)** — confirmed no bridge interaction

`SpeedUpdate` (called by Move) modulates `SpeedRequest` and `SpeedCurrent` based on path direction and rules constants.

**Cleanup pass verification:** Original draft of this report claimed "no bridge reads, confirmed by inspection" but did NOT actually decompile this function fresh. The cleanup pass decompiled it directly. **Result: zero reads of `cell.flags & 0x100`. Zero reads of `DAT_00A8F1B4` (Hover bridge threshold). Zero reads of `FootClass + 0x8C` (on_bridge flag).**

What SpeedUpdate DOES read:
- `DAT_00A8F180 / 0x184 / 0x188` (Hover NullCoord sentinels — coord-related, NOT bridge)
- `Rules.HoverBoost` (RulesClass+0x5D8/0x5DC double)
- `Rules.HoverAcceleration` (RulesClass+0x5E0)
- `Rules.HoverBrake` (RulesClass+0x5E8)
- `_DAT_007E27F8` (frame-rate normalisation constant)
- `g_DirectionDeltaX_Table` / `g_DirectionDeltaY_Table` (for next-cell offset)
- `RateTimer__Set` / `RateTimer__Current` for facing
- `FacingClass__UpdateFacing` for facing interpolation
- `_g_Const_0_0` and `_g_Const_1_0` (literal 0.0 and 1.0 doubles)

Hover's speed math is **layer-agnostic** — the unit accelerates and decelerates identically whether on ground, on bridge deck, or on water. **The bridge layer never enters the speed calculation.**

Confidence after cleanup: **C=HIGH (full body decompiled), I=HIGH, B=HIGH.**

### 3.7 Caller binding

Move is at ILocomotion vtable slot 17 of HoverLocomotionClass (vtable @ `0x007EACFC`). Called per-tick.

Active in YR: **Yes** — drives all 4 active hover unit types (LCRF, ROBO, SAPC, YHVR).

Confidence: C=HIGH, I=HIGH, B=HIGH.

---

## 4. Cross-doc contradictions resolved

### 4.1 Prior `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md §6` reading

The prior report described the `cell.flags & 0x100` check as a "water flag" check ("0x100 = is water flag"). **Refuted.** Per Phase 1+2 docs and direct re-verification, `cell.flags & 0x100` is the **on-bridge structural flag**, NOT water. Water is identified via `cell.LandType (+0xEC) == 2`.

The semantic in In_Which_Layer is **definitively bridge-related**: it subtracts a bridge altitude offset (`DAT_00ABC5DC`) when the cell has the bridge flag set. This is the layer-sort tweak that makes Rocketeers render correctly relative to a bridge they're flying over.

### 4.2 Prior `HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md §4.7`

The prior Hover report noted: "if the move would change cells, the new cell is fetched via `CellClass__Get_Cell_At` and `LinkedTo[0x140]` overlay flags are examined to toggle `LinkedTo[0x8C]` (`IsOnBridgeOrCellOverlay`)". **Confirmed.** The mechanism is the dual-conditional documented in §3.1 of this report — the prior doc was correct, this report adds the explicit threshold-vs-altitude formula.

### 4.3 The "DAT_00A8F1B4" vs "g_BridgeZOffset_Hover" naming

The Hover threshold at `0x00A8F1B4` is used as an **altitude threshold for layer detection**, not as a Z-offset that's added to a coord. Naming it `g_BridgeZOffset_Hover` would be misleading. Suggested label: `g_HoverBridgeAltitudeThreshold` or `g_HoverOnBridgeMinAltitude`.

This contrasts with Drive's `g_BridgeZOffset_Drive` which IS a Z-offset added to dest Z.

---

## 5. Active-in-YR confirmation per function

| Function | Active in YR? | Evidence | Gating |
|----------|---------------|----------|--------|
| `JumpjetLocomotionClass::In_Which_Layer @ 0x54B8D0` | Yes | ILocomotion vtable slot 29 of `0x07ECD68` | Bridge-adjust branch gated by `cell.flags & 0x100` AND `altitude >= DAT_00ABC5DC` AND `FootClass+0x8C == 0` AND `FootClass+0x8D == 0` |
| `HoverLocomotionClass::Move @ 0x514310` | Yes | ILocomotion vtable slot ~17 of `0x07EACFC` | Bridge transition gated by `cell.flags & 0x100` AND `Get_Height() >= DAT_00A8F1B4` |
| `HoverLocomotionClass::SpeedUpdate @ 0x515ED0` | Yes | Called from Move | No bridge gates (always runs) |

No SpecialFlags gates. No fog gates. Standard YR skirmish.

---

## 6. Current Rust Implementation Status

| Binary feature | Rust file | Status |
|----------------|-----------|--------|
| JumpJet altitude-offset for layer sort (subtract `DAT_00ABC5DC` when over bridge) | [src/render/layer_sort.rs](../../ra2-rust-game/src/render/layer_sort.rs) — if it exists | **Audit** — Rust likely doesn't have this layer-sort tweak; means jumpjet sprites may sort incorrectly relative to bridge deck. **Player-visible** when Rocketeer flies over a bridge with units on it. |
| JumpJet's 4-gate condition (on_bridge==0, cell.bridge, altitude>=threshold, +0x8D==0) | none | **Missing**. |
| Hover's on_bridge transition (set when entering bridge cell at altitude >= threshold; clear when leaving) | [src/sim/movement/movement_bridge.rs](../../ra2-rust-game/src/sim/movement/movement_bridge.rs) | **Partial** — Rust has bridge-occupancy state on Hover units but the altitude-threshold gate is unclear. Audit needed. |
| Hover's `DAT_00A8F1B4` altitude threshold | none | **Missing/audit** — the exact value depends on runtime init from theater data. |
| Hover's NOT-bumping the dest Z (unlike Drive/Ship) | [src/sim/movement/hover_movement.rs](../../ra2-rust-game/src/sim/movement/hover_movement.rs) — if it exists | **Audit** — Rust should NOT add a bridge Z-offset to hover destinations. |

(Severity assessment deferred to Phase 7 synthesis.)

---

## 7. Open Questions

1. **JumpJet `instance+0x2C` semantic** — CruiseHeight cache vs Speed cache. The In_Which_Layer reads it as altitude comparator; state 0 copies it to instance+0x80. Unresolved (was open in prior JumpJet doc).
2. **JumpJet `LinkedTo+0x8D`** byte — the "skip bridge altitude adjustment" gate. What sets it? Likely a transient flag during landing/takeoff.
3. **Hover `DAT_00A8F1B4`'s runtime value** — never observed at runtime in this report. Could be retrieved by attaching debugger or by reading after a game session start.
4. **Hover's 5 read sites of `DAT_00A8F1B4`** — only the Move-site is fully characterised here. The other 4 (in StartNextStep and helpers) likely use the same threshold for adjacent purposes (e.g., "is this neighbour a bridge cell I can step onto from current altitude") but were not decompiled in detail.
5. **The structural layout at `0x00A8F1B4..0x00A8F1C0+`** — looks like part of a Hover-rules struct. Worth a follow-up to map adjacent fields.

---

## 8. Sources

**Ghidra functions decompiled:**
- `JumpjetLocomotionClass::In_Which_Layer` (FUN_0054B8D0) @ 0x0054B8D0 (~70 bytes body)
- `JumpjetLocomotionClass::Constructor` @ 0x0054AC40 (~150 bytes body)
- `HoverLocomotionClass::Move` @ 0x00514310 (~2.2 KB body — bridge sites at §3.1 only)
- `HoverLocomotionClass::Compute_BridgeZOffset` not present (the relevant constant is initialised inline at `0x00513BA0`)

**Memory reads:**
- 0x007ECD68 + 128 bytes (JumpJet ILocomotion vtable; confirmed slot 29 = `0x0054B8D0`)
- 0x00A8F1B4 (Hover bridge alt threshold; BSS, cold dump 0)
- 0x00A8F1C0 (Hover Force_Float threshold; BSS, cold dump 0)
- 0x00ABC5DC (JumpJet bridge alt threshold; BSS, cold dump 0)

**Xrefs traced:**
- `get_xrefs_to 0x00ABC5DC` → 11 reads (across multiple JumpJet state handlers), 1 write @ 0x54ABC0
- `get_xrefs_to 0x00A8F1B4` → 5 reads, 1 write @ 0x513BA0

**Callers traced:**
- JumpJet::In_Which_Layer ← LayerClass::Submit_Object (per-tick render call)
- Hover::Move ← ILocomotion::Process dispatch per tick

**INI verification:**
- `Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5}` (JumpJet) — 9 declarations in `ini/rulesmd.ini`
- `Locomotor={4A582742-9839-11d1-B709-00A024DDAFD1}` (Hover) — 4 active + 1 commented declarations

**Companion docs:**
- `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` (the bridge Z-offset family table is in §1 of that doc)
- `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` (Phase 2 — cell.flags 0x100 semantic)
- `HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md` (prior — Hover struct layout, SpeedUpdate, Bob math)
- `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` (prior — JumpJet state machine, R3 update on Parachute non-existence)
- `LOCOMOTION_MATH_AND_CONSTANTS.md` (prior — locomotor CLSID table)

---

## 9. Cleanup pass — 2026-05-13 (post-initial-draft)

Summary of fixes applied:

| Item | Original status | Cleanup verdict |
|------|-----------------|-----------------|
| Hover::SpeedUpdate "no bridge reads" | Claimed HIGH, not verified | **Verified HIGH (fresh decompilation)** — see §3.6 |
| JumpJet::Process "no bridge reads" | Claimed without decomp | **Verified HIGH** — see §2.5b. Also flagged the fog-of-war TS-legacy gate at 0x54B107. |
| JumpJet::Set_Destination "no bridge reads" | Inferred | **MEDIUM (entry verified)** — Ghidra doesn't auto-label 0x54B1C0; full body not decompiled. See §2.5c. |
| `DAT_00ABC5DC` is JumpJet's bridge altitude threshold | HIGH | HIGH (unchanged) |
| `DAT_00A8F1B4` is Hover's bridge altitude threshold | HIGH | HIGH (unchanged) |
| Hover Bob math at 0x513D20 | (referenced from prior doc, not decompiled here) | Unchanged — prior HOVER doc covers this |

**No new bridge globals discovered for JumpJet/Hover** in this cleanup pass. (Two new globals were discovered for Walk and Teleport — see Drive/Ship doc §1 and Walk/Teleport doc §13.)

**Net effect:** the negative claims in this report ("Hover::SpeedUpdate has no bridge reads", "JumpJet::Process has no direct bridge reads") were ASSERTED in the original draft but only LIGHTLY verified. Cleanup pass independently re-verified each one by full decompilation. **Claims confirmed.**
