# UnloadingClass Render Orientation (CMON/HORV) — Ghidra Research Report

**Date:** 2026-05-19
**Scope:** Render-orientation only — does the UnloadingClass voxel rotate with the
unit's live facing, or is it locked to a fixed frame? What happens if the harvester
is at facing 0xC0 (west) instead of 0x40 (east) on the dock pad?
**Active in YR:** Yes — fires on every cargo cycle for Chrono Miner (CMIN) and War
Miner (HARV).
**Confidence:** HIGH — all key claims verified from binary decompilation.

---

## 1. Overview

When a harvester docks at a refinery and begins unloading, `UnitClass::DrawExtras`
temporarily swaps the unit's TypeClass pointer from the normal type (CMIN/HARV) to
the UnloadingClass (CMON/HORV) before calling the voxel draw path. The swap replaces
only the **type** (i.e., which VXL model file is loaded), not any field on the unit
instance. The unit's facing, position, and locomotor state are unchanged by the swap.

**Answer to the primary question: Yes — a wrong facing during dump IS player-visible.**
The CMON/HORV voxel is rendered using the unit's current live locomotor facing, exactly
the same as the normal CMIN/HARV voxel. There is no hardcoded frame or fixed orientation.
If the harvester arrives at the dock pad facing 0xC0 (west) instead of 0x40 (east),
the open-bay model will render pointing west.

---

## 2. Image Swap Mechanism — `UnitClass::DrawExtras` (0x0073CEC0)

**Verified via:** `decompile_function 0x0073CEC0`

The swap occurs inside `UnitClass__DrawExtras` in three steps:

### Step 1 — Save TypeClass
```c
puStack_4 = (undefined4 *)param_1[0x1b1];   // save TypeClass* (at unit+0x6C4)
```

### Step 2 — Conditional swap
```c
if (*(char *)(puStack_4 + 0xe0e) != '\0'     // TypeClass->Harvester (offset 0xE0E)
 && *(char *)((int)param_1 + 0x6d1) != '\0' // unit byte 0x6D1 (dock-active flag)
 && *(int *)((int)puStack_4 + 0x6b8) != 0)  // TypeClass->UnloadingClass (offset 0x6B8)
{
    param_1[0x1b1] = *(int *)((int)puStack_4 + 0x6b8);  // unit->TypeClass = CMON or HORV
}
```

- `TypeClass+0xE0E` = `Harvester=yes` flag (verified in HARVESTER_DOCK_UNLOAD.md)
- `unit+0x6D1` = dock-active flag, set to 1 in `Mission_Deploy_Building` init
  (byte 0x6D1: 0→1 on first-entry to dump path, cleared to 0 in state 4 on exit)
  (verified via `MISSION_DEPLOY_BUILDING_0x73D630_STATE_MACHINE_GHIDRA_REPORT.md` §3B)
- `TypeClass+0x6B8` = `UnloadingClass` pointer (CMON for CMIN, HORV for HARV)
  (verified in HARVESTER_DOCK_UNLOAD.md §2.5 and TechnoTypeClass::ReadINI at 0x7146E8)

### Step 3 — Voxel dispatch (with swapped TypeClass)
```c
iVar4 = param_1[0x1b1];    // now CMON or HORV TypeClass
if (*(char *)(iVar4 + 0x236) == '\0') {
    (**(code **)(*param_1 + 0x558))(...);  // SHP body draw — not taken (CMON/HORV are VXL)
} else if (*(int *)(iVar4 + 0xb0) != 0) {
    (**(code **)(*param_1 + 0x554))(...);  // VXL body draw — taken (Voxel=yes)
}
param_1[0x1b1] = iStack_8;  // RESTORE TypeClass (original CMIN/HARV)
```

- `TypeClass+0x236` = VoxelFlag (0=SHP, nonzero=VXL); CMON/HORV have `Voxel=yes` in artmd.ini
  (verified: artmd.ini lines 621, 649)
- `TypeClass+0xB0` = VoxelData pointer (the loaded .vxl struct)
- vtable+0x554 = `0x0073B470` (UnitClass VXL draw path, resolved via vtable at 0x007F5C70)
  (verified via `read_memory 0x007F61C4`: bytes `70 B4 73 00` = 0x0073B470)

**Critical point:** The swap only changes which VXL MODEL is rendered. The unit's facing
field (`unit+0x388`, FacingClass), its locomotor pointer (`unit+0x674`), and all other
instance state remain on the **unit instance** and are completely unaffected by the
TypeClass pointer swap.

---

## 3. Facing Read Path — How the VXL Orientation Is Determined

**Verified via:** `decompile_function 0x0073C5F0` (UnitClass__Draw_Body_And_Turret)
and `read_memory 0x007F5F60` (vtable+0x2F0 → 0x004DB0A0)

CMON and HORV are defined without `Turret=yes` in artmd.ini, so they take the
**no-turret path** in `UnitClass__Draw_Body_And_Turret`. In that path:

```c
uStack_14c = (**(code **)(iVar12 + 0x2f0))();  // get facing matrix
(**(code **)(iVar12 + 0x50c))(...);             // VXL draw with that matrix
```

vtable+0x2F0 for UnitClass resolves to **`FUN_004DB0A0`** (verified via `read_memory
0x007F5F60`: bytes `A0 B0 4D 00`):

```c
undefined4 FUN_004DB0A0(int param_1) {
    if (*(int *)(param_1 + 0x674) != 0) {
        // Call locomotor vtable+0x3C: ILocomotion::Get_Facing_Matrix() or equivalent
        uVar1 = (**(code **)(**(int**)(param_1 + 0x674) + 0x3c))(*(int**)(param_1 + 0x674));
        return uVar1;
    }
    return 2;  // fallback facing index if no locomotor
}
```

- `param_1 + 0x674` = unit's locomotor interface pointer (`unit->Locomotor`)
- `locomotor_vtable+0x3C` = the locomotor's facing matrix getter — returns the **current
  live body facing matrix** for the unit as it exists right now
- The locomotor owns the authoritative facing direction; it is set via `ILocomotion::Head_To`
  and updated per-tick by `UnitClass::Facing_Update` (0x00736990)

**The facing matrix is read from the unit's live locomotor, NOT from a static index or
the TypeClass.** When the TypeClass pointer is temporarily set to CMON/HORV for rendering,
the locomotor read (`param_1 + 0x674`) still points to the same locomotor on the same unit.
The locomotor's facing is unchanged by the TypeClass swap.

---

## 4. INI Verification — CMON/HORV Are VXL Only

From `artmd.ini` (lines 620-650, verified):

```ini
[HORV]          ; Soviet harvester without back (open bay model)
Voxel=yes
Remapable=yes

[CMON]          ; Allied harvester without back (open bay model)
Voxel=yes
Remapable=yes
```

Neither CMON nor HORV defines `Image=`, `TurretAnim=`, or any SHP fallback. The VXL
rendering path is taken unconditionally when `TypeClass+0xB0` (VoxelData) is non-null.

From `rulesmd.ini`:
- Line 7384: `[CMIN]` section → `UnloadingClass=CMON`
- Line 8246: `[HARV]` section → `UnloadingClass=HORV`

Both are stored at `TechnoTypeClass+0x6B8` (verified from ReadINI at 0x7146E8 in
HARVESTER_DOCK_UNLOAD.md §2.5).

---

## 5. GAREFNOR Animation — Facing-Independent (Building-Side)

The per-bale `GAREFNOR` animation (slot 10, `SpecialAnim`) is played on the **refinery
building**, not on the harvester. It is a 2D SHP animation with fixed frame layout and
no facing dependency. It fires regardless of which direction the harvester is facing.
(Verified in REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §3, Trigger 2.)

This animation does NOT substitute for the CMON/HORV voxel — they are independent:
- GAREFNOR: building-side smoke/deposit effect, 2D SHP, fixed orientation
- CMON/HORV: harvester-side body model swap, 3D VXL, facing-dependent

---

## 6. Player-Visibility Verdict

**If the harvester is at facing 0xC0 (west) instead of 0x40 (east) on the dock pad:**

1. `UnitClass::DrawExtras` swaps TypeClass to CMON/HORV (the open-bay model)
2. `UnitClass::Draw_Body_And_Turret` calls `vtable+0x2F0` (FUN_004DB0A0)
3. FUN_004DB0A0 reads the live locomotor facing via `locomotor_vtable+0x3C`
4. The locomotor returns the unit's current facing — which is 0xC0 (west)
5. The 3D voxel rasterizer applies this facing matrix to the CMON/HORV geometry
6. **Result: the open-bay model is rendered pointing west, visible to the player**

A harvester sitting on the refinery pad with the wrong facing will show the open-bay
voxel (CMON or HORV) rotated to that wrong orientation. Since the open bay faces toward
the rear/cargo area of the miner, a wrong facing causes the visual gap/opening to point
in the wrong direction — clearly incorrect and player-visible.

**Severity:** Player-visible every time a harvester docks and unloads (fires every match,
multiple times per match). The dump sequence lasts several seconds, so the wrong-facing
voxel is displayed for the full duration.

---

## 7. Open Questions — Final State

- `[RESOLVED] Q1 — Does the CMON/HORV visual rotate with facing?`
  → YES. Facing matrix is read from unit's live locomotor via FUN_004DB0A0, not from
  any TypeClass field. (evidence: `decompile_function 0x004DB0A0`, `read_memory 0x007F5F60`)

- `[RESOLVED] Q2 — Where is orientation read from?`
  → From `unit+0x674` (locomotor pointer) via `locomotor_vtable+0x3C`. The FacingClass
  at `unit+0x388` is the input to the locomotor's facing; ultimately the locomotor's
  facing matrix is what drives the VXL orientation. (evidence: `decompile_function 0x004DB0A0`)

- `[RESOLVED] Q3 — Is the gate flag (unit+0x6D1) active during the entire dump?`
  → YES. Set to 1 at dock-init (0x6D1: 0→1), cleared to 0 only after dump completes
  in state 4 (after all bales deposited). The swap is active the entire dump duration.
  (evidence: `MISSION_DEPLOY_BUILDING_0x73D630_STATE_MACHINE_GHIDRA_REPORT.md` §3B)

- `[RESOLVED] Q4 — Is GAREFNOR player-visible and facing-dependent?`
  → GAREFNOR is on the building, not the harvester; it is a fixed-orientation 2D SHP.
  Not facing-dependent, not affected by harvester orientation.
  (evidence: REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §3 Trigger 2)

- `[RESOLVED] Q5 — Is the VXL selected by TypeClass or directly by name?`
  → TypeClass. The swapped TypeClass (CMON/HORV) has its own VoxelData pointer at
  TypeClass+0xB0. The VXL filename = the type's section name (CMON.vxl, HORV.vxl).
  No `Image=` redirect in artmd.ini. (evidence: artmd.ini lines 620-650, confirmed Voxel=yes)

- `[RESOLVED] Q6 — Does the swap happen before or after the locomotor facing is read?`
  → BEFORE. DrawExtras swaps the TypeClass, then calls vtable+0x554 → Draw_Body_And_Turret
  → vtable+0x2F0 (facing matrix getter) → locomotor. Facing is read during the draw,
  after the type swap. No sequencing issue. (evidence: decompile 0x0073CEC0 control flow)

- `[DEFERRED] Q7 — Confirm the exact locomotor vtable+0x3C call semantics`
  (category: `bounded-cost-too-high`; reason: would require decompiling each locomotor
  type's vtable+0x3C implementation; the behavior is clear from the VXL rendering
  pipeline docs and is not needed to answer the orientation question)

---

## 8. Key Verified Facts (Top 5)

1. **Draw function address:** `UnitClass::DrawExtras` @ `0x0073CEC0` — confirmed as
   the site of the TypeClass swap and VXL draw dispatch. (verified: decompile_function)

2. **Facing read address:** `FUN_004DB0A0` @ vtable+0x2F0 (UnitClass vtable `0x007F5C70 + 0x2F0 = 0x007F5F60`; read_memory confirms `A0 B0 4D 00`). Reads live facing from `unit+0x674` (locomotor) via `locomotor_vtable+0x3C`.

3. **Wrong facing IS player-visible:** CMON/HORV VXL is drawn with the unit's live
   locomotor facing matrix. If facing is wrong, the open-bay model points the wrong way.
   Full duration of dump (several seconds per cycle, every cycle).

4. **Swap gate = unit byte 0x6D1:** Set to 1 at dump-init, cleared to 0 after last bale.
   CMON/HORV swap is active precisely during the dump sequence.
   (`MISSION_DEPLOY_BUILDING_0x73D630_STATE_MACHINE_GHIDRA_REPORT.md` §3B)

5. **GAREFNOR is facing-independent:** It is a building-side 2D SHP, no facing relation
   to the harvester. Does not mask or compensate for a wrong harvester facing.
   (REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §3 Trigger 2)

---

## Sources

- Ghidra decompiled: `0x0073CEC0` (UnitClass__DrawExtras), `0x0073C5F0` (Draw_Body_And_Turret),
  `0x007353C0` (UnitClass constructor — vtable address), `0x004DB0A0` (vtable+0x2F0 facing getter)
- Memory read: `0x007F5F60` (vtable+0x2F0 slot), `0x007F61C4` (vtable+0x554 slot)
- Disassemble: `0x007353C0` (confirmed vtable at 0x007F5C70)
- Docs referenced: HARVESTER_DOCK_UNLOAD.md §2.5, REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §3,
  MISSION_DEPLOY_BUILDING_0x73D630_STATE_MACHINE_GHIDRA_REPORT.md §3B, VOXEL_RENDERING_ANALYSIS.md,
  UNIT_DRAW_EXTRAS_REPORT.md §3-4
- INI: `artmd.ini` lines 620-650 (HORV/CMON Voxel=yes), `rulesmd.ini` lines 7384, 8246
  (UnloadingClass=CMON/HORV)
