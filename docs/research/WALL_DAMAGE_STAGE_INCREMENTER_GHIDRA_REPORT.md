# Wall Damage-Stage Incrementer — Ghidra Research Report

**Target:** CellClass+0x11E upper nibble (0xF0) incrementer — the code path that advances a
wall overlay's damage stage.

**Scope:** Filling the gap noted in `WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT.md §2.2`:
"damage-stage incrementer was not traced this pass."

**Session date:** 2026-05-19

**Confidence:** HIGH (all key findings verified from live Ghidra decompilation this session).
**Active in YR:** Yes — walls are a standard skirmish element; incrementer fires every time a
weapon that can damage walls strikes a walled cell.

---

## 1. The Incrementer — Verified

**Function:** `CellClass::DestroyOverlay` @ `0x00480CB0` (same function as the destructor;
handles both per-stage progression and final removal).

**Exact instruction** (verified from decompilation):
```c
bVar5 = param_1->field_0x11e + 0x10;   // add 0x10 to upper nibble
param_1->field_0x11e = bVar5;           // write back
```

This fires unconditionally once the function's entry gates are cleared (see §2). The lower nibble
(connectivity) is not touched — the field is treated as a raw u8 add of 0x10, which increments
only the upper nibble when the lower nibble does not carry (connectivity 0..15, 15 + 0x10 = 0x1F;
max lower nibble is 0x0F, so carries happen but are survivable — destruction gate catches them).

---

## 2. Entry Gates (what triggers the incrementer)

`CellClass::DestroyOverlay(damage: i32)` has two entry gates before the increment fires:

### Gate 1 — IsWall check
```c
if (cell.OverlayTypeIndex == -1) return 0;
overlay_type = g_OverlayTypeClass_Array[cell.OverlayTypeIndex];
if (overlay_type[+0x2A8] == 0) return 0;   // not a wall overlay → exit
```
Only wall overlays (OverlayTypeClass.Wall == true, read from INI key `Wall=` in art section)
proceed. Verified from `OverlayTypeClass::ReadINI` @ `0x005FE770`.

### Gate 2 — Probabilistic damage roll (bypassed when damage == -1)
```c
if (damage != -1) {                               // -1 = forced destroy (engineer/crusher)
    if (damage < overlay_type[+0x2A4]) {          // Strength=
        if (Random::RandomRanged(0, Strength) > damage) return 0;  // miss this tick
    }
}
```
- `damage == -1`: forced destroy — no roll, always increments (all stages at once → destroyed).
- `damage < Strength`: probabilistic gate. Probability of stage advance per tick = damage/Strength.
- `damage >= Strength`: always advances (no roll needed).

**Source of Strength field:** `OverlayTypeClass::ReadINI` @ `0x005FE770`:
- `OverlayTypeClass + 0x2A4` = `Strength=` (read from rules INI section, default 1).

---

## 3. Callers of CellClass::DestroyOverlay (verified xrefs @ 0x00480CB0)

| Caller address | Caller name | damage arg | Notes |
|---|---|---|---|
| `0x004896AD` | `Apply_area_damage` | weapon damage | Conditional on warhead flags (§4) |
| `0x00445B69` | `BuildingClass__Limbo` | varies | Building demolition clears overlays on its footprint |
| `0x00480EAF` | `CellClass__DestroyOverlay` | `0xC8` (200) | Self-recursion: concrete wall chain reaction |
| `0x0073B056` | `UnitClass__PerCellProcess` | `0xFFFFFFFF` (−1) | Engineer/crusher entering cell (forced destroy) |
| `0x0075F477` | `FUN_0075F330` | raw weapon damage | Weapon-hit-on-cell path (no warhead flag check) |

All 5 callers confirmed via `get_xrefs_to(0x00480CB0)` this session.

---

## 4. Warhead Flags That Gate DestroyOverlay in Apply_area_damage

`Apply_area_damage` @ `0x00489280` contains this gate around the `CellClass__DestroyOverlay`
call (verified from decompilation this session):

```c
// iStack_60 = OverlayTypeClass* for this cell's overlay
// param_4   = WarheadTypeClass*
if ((*(char *)(iStack_60 + 0x2a8) != '\0') &&        // overlay.Wall == true
   (((*(char *)(param_4 + 0x145) != '\0' ||           // warhead.WallAbsoluteDestroyer
     (*(char *)(param_4 + 0x144) != '\0')) ||          // warhead.Wall
    ((*(char *)(param_4 + 0x147) != '\0' &&            // warhead.Wood
      (*(int *)(iStack_60 + 0x9c) == 6)))))) {         // AND overlay.Armor == Special(6)
  CellClass__DestroyOverlay(damage_value);
}
```

**Warhead offsets verified from `WarheadTypeClass::ReadINI_Body` @ `0x0075D4E1`:**

| WarheadTypeClass offset | INI key | Verified string address |
|---|---|---|
| `+0x144` | `Wall=` | `0x0081AC58` (string "Wall\0") |
| `+0x145` | `WallAbsoluteDestroyer=` | `0x00847E1C` |
| `+0x147` | `Wood=` | `0x00847E00` (string "Wood\0") |

**Note:** The existing `WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT.md §4.0` listed
the third flag as "Incendiary" — this is incorrect. The string at `+0x147` is `Wood=`, not
`Incendiary=`. Verified by reading memory at `0x00847E00`.

**What this means for wall damage routing:**
- `Wall=true` warhead → always damages any wall overlay.
- `WallAbsoluteDestroyer=true` warhead → always damages any wall overlay.
- `Wood=true` warhead AND overlay's `Armor=Special` (armor enum 6) → damages that overlay.
- None of the above → wall is immune to this warhead's area damage.

**Active in YR:** Yes — standard weapons use `Wall=yes` on the `HE` warhead (`[HE]` in rulesmd.ini).

---

## 5. DamageLevels — Per-Wall-Type Stage Counts

Stage count is controlled by `OverlayTypeClass + 0x2A0` = `DamageLevels=` (read from the
**art section** identified by the overlay's `Image=` key, not from rules.ini).

Verified from `OverlayTypeClass::ReadINI` @ `0x005FE770`:
```c
uVar4 = CCINIClass__ReadInt(param_1 + 0x1f8,   // art section name (Image=)
                             s_DamageLevels_..., // "DamageLevels"
                             current_value);      // default = prior value (1 from ctor)
*(int *)(param_1 + 0x2a0) = uVar4;
```

Per-type stage counts (from `PostDestructionWallCleanup` cleanup thresholds, confirmed in the
existing doc §5.2 which was verified in an earlier session):

| Overlay index | Name | DamageLevels | Stages (upper nibble values) | Max byte |
|---|---|---|---|---|
| 0 | GASAND | 3 | 0, 1, 2 → `0x00`, `0x10`, `0x20` | `0x20` |
| 1 | CYCL | 3 | 0, 1, 2 → `0x00`, `0x10`, `0x20` | `0x20` |
| 2 | GAWALL | 4 | 0, 1, 2, 3 → `0x00`, `0x10`, `0x20`, `0x30` | `0x30` |
| 3 | BARB | 2 | 0, 1 → `0x00`, `0x10` | `0x10` |
| 0x16 | (unknown) | 3 | 0, 1, 2 | `0x20` |
| 0x1A | NAWALL | 4 | 0, 1, 2, 3 → `0x00`, `0x10`, `0x20`, `0x30` | `0x30` |

All walls share **the same single incrementer** (`CellClass::DestroyOverlay`); stage count
differences come entirely from the per-type `DamageLevels` field at `+0x2A0`.

---

## 6. Threshold Formula — HP to Stage Progression

There is **no HP-deduction-then-threshold** formula. The engine does not track fractional HP on
wall overlays. Instead:

1. Each call to `DestroyOverlay` is a **single stage increment** (adds 0x10 to field_0x11E).
2. Whether the call fires this tick is determined by the **probabilistic roll** in Gate 2 (§2).
3. Effective "HP" is entirely emergent from the roll probability:
   - A wall with `Strength=400` takes on average `400/damage` ticks per stage at a given
     damage value (because each tick has `damage/400` chance of advancing).
   - There is no stored HP counter per cell — only the stage nibble.

**The formula linking damage to expected stage advance time:**
```
E[ticks per stage] = Strength / damage     (when damage < Strength)
E[ticks per stage] = 1                     (when damage >= Strength, guaranteed advance)
```

**Destruction gate** (from `DestroyOverlay` decompilation, verified):
```c
stage = cell.field_0x11e >> 4;            // upper nibble
if (stage < DamageLevels - 1) return 0;  // not at final stage → keep going
if (stage == DamageLevels - 1 &&
    (cell.field_0x11e & 0x0F) != 0) return 0;  // at max stage but still connected
// ... proceed to remove overlay
```
Final removal requires: upper nibble == `DamageLevels - 1` AND lower nibble (connectivity) == 0.

**Chain reaction gate** (concrete walls only, verified):
```c
new_stage = bVar5 >> 4;                         // just-incremented stage
if (new_stage == DamageLevels - 1 && DamageLevels > 2) {
    // cascade 0xC8 (200) damage to 4 cardinal same-type pristine neighbors
}
```
Fires when wall reaches final stage AND `DamageLevels > 2` (only GAWALL/NAWALL with
DamageLevels=4 qualify; 2-stage/3-stage walls do not cascade).

---

## 7. Open Questions

1. **`UnitClass::PerCellProcess` crusher condition:** The `damage=-1` call at `0x0073B056` is
   gated on `overlay_type.Crushable` (`+0x22D`) OR `overlay_type.Wall` AND unit has
   `Crusher` ability. The exact gate for which unit types trigger this was not fully traced
   this session — it involves `TechnoClass::HasWeaponAbility(0x11)` and a `BuildingType`
   flag at `+0xD28`. Scope deferred.

2. **`FUN_0075F330` caller identity:** This function calls `DestroyOverlay` with raw damage,
   no warhead-flag guard. Its own callers were not traced this session. Likely a
   bullet/warhead direct-hit path separate from area damage. Scope deferred.

3. **OverlayType index 0x16 identity:** The INI name for overlay index 0x16 was not determined
   this session. It appears in `PostDestructionWallCleanup` thresholds alongside GASAND/GAWALL.

4. **`BuildingClass__Limbo` damage value:** The exact damage argument passed when a building
   demolishes its footprint overlays was not read this session.

5. **Warhead `Wall=` key vs `WallAbsoluteDestroyer=` behavioral difference:** Both trigger
   `DestroyOverlay`, but `WallAbsoluteDestroyer` is listed separately — whether it bypasses
   the probabilistic roll (sends `-1` instead of the weapon damage) was not confirmed this
   session. The decompilation showed both passing `damage_value` (not `-1`) to `DestroyOverlay`,
   so the difference may be purely in the OR logic (either flag alone suffices).

---

## 8. Rust Implementation Gap (from prior doc, unchanged)

`src/map/overlay.rs` has no damage-stage tracking. The frame is computed as connectivity-nibble
only. Required:

1. Add `damage_stage: u8` per cell overlay entry. Draw uses `(damage_stage << 4) | connectivity`.
2. Implement `DestroyOverlay(cell, damage)` with the two gates (§2) and the incrementer.
3. Implement `DamageLevels` parsing from art INI into `OverlayTypeClass + 0x2A0`.
4. Implement `Strength` parsing from rules INI into `OverlayTypeClass + 0x2A4`.
5. Implement the destruction gate and connectivity-zero requirement.
6. Implement the concrete-wall chain reaction (`DamageLevels > 2` gate).

---

## Sources (this session)

**Ghidra addresses decompiled:**
- `0x00480CB0` — `CellClass__DestroyOverlay` (incrementer confirmed: `field_0x11e + 0x10`)
- `0x00489280` — `Apply_area_damage` (warhead gate confirmed: `+0x144`, `+0x145`, `+0x147`)
- `0x005FE770` — `OverlayTypeClass__ReadINI` (field layout: `+0x2A0` DamageLevels, `+0x2A4` Strength)
- `0x0075D4E1` — `WarheadTypeClass__ReadINI_Body` (warhead field offsets confirmed)
- `0x0073B056` — `UnitClass__PerCellProcess` (engineer/crusher forced-destroy path: damage=-1)
- `0x0075F477` — `FUN_0075F330` (direct weapon-hit path: passes raw damage, no warhead guard)

**Xrefs queried:**
- `get_xrefs_to(0x00480CB0)` — confirmed 5 callers
- `get_xrefs_to(0x0083B3C0)` — DestroyWalls string (RulesClass only, not warhead)
- `get_xrefs_to(0x00847E1C)` — WallAbsoluteDestroyer string → WarheadTypeClass__ReadINI_Body
- `get_xrefs_to(0x00847E34)` — Conventional string → WarheadTypeClass__ReadINI_Body

**Memory inspected:**
- `0x0081AC58` → "Wall\0" (confirms WarheadTypeClass+0x144 = `Wall=`)
- `0x00847E00` → "Wood\0" (confirms WarheadTypeClass+0x147 = `Wood=`, not "Incendiary")

**Cross-referenced docs:**
- `WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT.md` — base doc this fills
