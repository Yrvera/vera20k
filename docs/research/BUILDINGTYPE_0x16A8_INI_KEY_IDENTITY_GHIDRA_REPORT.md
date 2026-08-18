# BuildingTypeClass+0x16A8 — INI Key Identity: SiloDamage

**Target:** Settle the identity of the boolean byte at `BuildingTypeClass+0x16A8` and resolve the HasStorage vs HasTurretAnim conflict from REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.2.

**Verdict: Hypothesis B is correct. `Type+0x16A8 = SiloDamage` flag. GAREFN does NOT have this flag set. Slot-10 SetAnimSlotImage always fires unsuppressed during refinery dock unload.**

**Active in YR:** Yes — the `SiloDamage` block in `BuildingClass::UpdateAnimation` is live code. However, it only executes on buildings with `SiloDamage=yes` (in YR: only `GASILO`). For all refineries it is unconditionally skipped.

**Confidence:** HIGH (content), HIGH (identity), HIGH (binding) — verified via direct decompilation of `BuildingTypeClass_ReadINI_Water` (0x460A5B) and `BuildingClass::UpdateAnimation` (0x4509D0/0x450CBD).

---

## 1. Identification of `BuildingTypeClass+0x16A8`

### ReadINI mapping (verified)

Decompiled `BuildingTypeClass_ReadINI_Water` at `0x460A5B` (verified via `decompile_function 0x460A5B`). The relevant sequential ReadBool calls in the 0x16A0–0x16B0 range are:

| Offset | INI Key | String address |
|--------|---------|---------------|
| +0x16A4 | `Radar` | `0x0081ae60` |
| +0x16A5 | `SpySat` | `0x0081ae58` |
| +0x16A6 | `ChargeAnim` | `0x0081a774` |
| +0x16A7 | `IsAnimDelayedFire` | `0x0081a760` |
| **+0x16A8** | **`SiloDamage`** | **`0x0081a780`** |
| +0x16A9 | `UnitRepair` | `0x0081aaf0` |
| +0x16AA | `UnitReload` | `0x0081aae4` |
| +0x16AB | `Bunker` | `0x0081aadc` |
| +0x16AC | `Cloning` | `0x0081aad4` |
| +0x16AD | `Grinding` | `0x0081aac8` |
| +0x16AE | `UnitAbsorb` | `0x0081aabc` |
| +0x16AF | `InfantryAbsorb` | `0x0081aaac` |
| +0x16B0 | `SecretLab` | `0x0081aaa0` |
| +0x16BB | `Refinery` | `0x0081aa5c` |

**Exact decompilation quote for +0x16A8:**
```c
uVar4 = CCINIClass__ReadBool(iVar15, s_SiloDamage_0081a780, *(undefined1 *)(param_1 + 0x16a8));
*(undefined1 *)(param_1 + 0x16a8) = uVar4;
```
(verified via `decompile_function 0x460A5B`, reading the sequential ReadBool call immediately after the `Flat`/`DoubleThick` reads)

### INI string confirmed in binary

`search_strings "SiloDamage"` → `0x0081a780` — single match in the binary, xref confirmed to the ReadINI call above.

### INI default and where it appears

From `artmd.ini` comment (line 53): `; SiloDamage = Is damage image based on base Tiberium storage level (def=no)?`

`SiloDamage=yes` appears **only** on `[GASILO]` (artmd.ini line 2008 and art.ini line 1252). No other building in stock RA2/YR sets this flag. `GAREFN` does not define it.

---

## 2. What `Type+0x16A8 (SiloDamage)` gates in `BuildingClass::UpdateAnimation`

Decompiled `BuildingClass::UpdateAnimation` at `0x450CBD` (verified via `decompile_function 0x450CBD`). The conditional:

```c
if (this->Type[0x16a8] != '\0') {
    // --- SiloDamage / silo fill-level anim block ---
    iVar6 = 0;
    if (*(int *)(this->Type + 0x800) >= 1) {
        StorageClass__GetTotalAmount();
        // ... compute storage tier (0..3) from stored amount vs capacity
        iVar6 = tier;
    }
    if (tier == 0) {
        if (*(int *)&this->field_0x584 != 0)
            BuildingClass__ClearAnimSlot(this);   // clear slot 10 if empty
    } else {
        if (*(int *)&this->field_0x584 == 0)
            BuildingClass__CreateAnimForSlot(this); // create slot 10 if full
        *(int *)(*(int *)&this->field_0x584 + 0xac) = tier;  // write tier to anim
    }
}
```

**This block:**
- Is the **silo fill-level animation** display (showing how full a Tiberium Silo is)
- Operates on `this->field_0x584` = the slot-10 (`SpecialAnim`) anim pointer
- Selects art from `this->Type + 0x11F4` (slot anim name, verified in context) based on health state
- Is **entirely about managing the always-visible fill-level display for GASILO**, NOT about the per-bale GAREFNOR pulse

**Crucially:** for any building where `SiloDamage=no` (default, including all refineries), the entire block is skipped. `field_0x584` is never touched by `BuildingClass::UpdateAnimation` for GAREFN.

---

## 3. Hypothesis Resolution

| Hypothesis | Claim | Verdict |
|------------|-------|---------|
| A: HasStorage | Type+0x16A8 set on refineries → slot-10 always non-null → per-bale SetAnimSlotImage(10) suppressed | **WRONG** |
| B: HasTurretAnim | Type+0x16A8 unset on refineries → slot-10 null → per-bale SetAnimSlotImage(10) fires | **CORRECT (but label is also wrong — it's SiloDamage, not HasTurretAnim)** |

**The correct statement is:**

> `BuildingTypeClass+0x16A8` = `SiloDamage` INI key. It is `false` (0) on `GAREFN`. Therefore `BuildingClass::UpdateAnimation` never touches `building->field_0x584` (the slot-10 pointer) for any refinery. When `Mission_Deploy_Building` checks `building->field_0x584 == 0` before firing the per-bale SetAnimSlotImage(10) call, that check is against a field that is **only managed by the harvester FSM itself** (CreateAnimForSlot on bale start, ClearAnimSlot on bale end). The slot-10 GAREFNOR animation fires on every bale unsuppressed.

Neither hypothesis A nor B correctly named the field — "HasStorage" and "HasTurretAnim" are both wrong labels. The actual key is `SiloDamage`.

---

## 4. Context: TurretAnim Strings

`search_strings "TurretAnim"` → 8 matches for keys `TurretAnim`, `TurretAnimIsVoxel`, `TurretAnimYSort`, etc. These are legitimate INI keys but map to **different offsets** (in the 0x1900+ range or similar — not verified in this session). `TurretAnim` has nothing to do with 0x16A8. The "HasTurretAnim" hypothesis was a misidentification.

---

## 5. Downstream Impact on Rust Port

From REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.2, the concern was:

> If `building->field_0x584` is always non-null (Hypothesis A), per-bale SetAnimSlotImage(10) is suppressed during normal unloading.

**This concern is resolved.** For stock refineries (GAREFN, NAREFN), `SiloDamage=no`, so `field_0x584` is managed only by the harvester FSM. Per-bale SetAnimSlotImage(10) fires freely — confirmed consistent with the player-observed behavior (GAREFNOR pulses each bale).

The existing §7 "What needs to change" in REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md remains valid and is **not affected** by this finding. No suppression path exists for GAREFN's slot-10. The Rust port simply needs to fire the per-bale SpecialAnim trigger.

---

## 6. Verified Offset Table (adjacent range)

All from `decompile_function 0x460A5B` — single source, direct ReadBool calls:

| Offset | Key | Notes |
|--------|-----|-------|
| +0x16A4 | `Radar` | radar dome flag |
| +0x16A5 | `SpySat` | spy satellite flag |
| +0x16A6 | `ChargeAnim` | fire-effect charge anim |
| +0x16A7 | `IsAnimDelayedFire` | weapon fire delay |
| +0x16A8 | `SiloDamage` | silo fill-level anim (GASILO only) |
| +0x16A9 | `UnitRepair` | repairs docked units |
| +0x16AA | `UnitReload` | reloads docked units |
| +0x16AB | `Bunker` | garrison bunker |
| +0x16AC | `Cloning` | Yuri clone vat |
| +0x16AD | `Grinding` | Alliance (mod) grinder |
| +0x16AE | `UnitAbsorb` | absorb units on entry |
| +0x16AF | `InfantryAbsorb` | absorb infantry on entry |
| +0x16B0 | `SecretLab` | secret lab power-up |
| +0x16B3 | `DockUnload` | accepts ore unloads |
| +0x16BB | `Refinery` | gates slot-8 call |

---

*Investigation by: re-swarm slot 1, 2026-05-19. Ghidra MCP read-only.*
