# BuildingClass::DepositOreFromStorage (0x522D50) — Ghidra Research Report

**Date:** 2026-05-19
**Binary:** gamemd.exe (Yuri's Revenge)
**Target:** Chrono Miner / Harvester ore-to-credits unload mechanism
**Confidence:** HIGH — all findings from direct live decompilation this session

---

## CRITICAL FINDING: DepositOreFromStorage is Slave Miner-only

`BuildingClass::DepositOreFromStorage` (0x522D50) has **exactly one call site** (verified via
`get_xrefs_to`): `SlaveManagerClass::AI_Update` at 0x6AFB D2, state 4 of the slave state machine.

**It is NOT called during regular harvester or chrono miner docking.** The chrono miner / War
Miner / standard HARV credits come from a completely separate path: `UnitClass::Mission_Deploy_Building`
(0x73D630) handles the dump inline, without calling `DepositOreFromStorage` at all.

This changes the framing for all items (a)–(g) below.

---

## Scope Resolution per Question

### (a) Once-per-bale or once-for-all?

**Two separate answers by unit type:**

**Slave Miner path — BuildingClass::DepositOreFromStorage (0x522D50):**
Called **once-for-all-storage** per invocation. Internally loops over all non-empty tiberium
slots and drains the entire building's StorageClass in a single call:

```c
while (tibType = StorageClass__FindFirstNonEmptySlot() != -1) {
    amount = StorageClass__GetAmount(tibType);
    removed = StorageClass__RemoveAmount(amount, tibType);   // ALL of that type
    if (removed > 0) {
        HouseClass__Add_Tiberium_Credits(removed, tibType);
        if (purifierBonus > 0) HouseClass__Add_Tiberium_Credits(purifierBonus, tibType);
    }
}
```

**Harvester / Chrono Miner path — UnitClass::Mission_Deploy_Building state 3:**
Called **once-per-bale per frame**. The timer check fires every 14.4 frames; on each
fire, a single `StorageClass__FindFirstNonEmptySlot` + `StorageClass__RemoveAmount` removes
**one slot's worth** (the full amount of the first non-empty type, not one unit). After
removal the step counter resets to 0 and the loop exits. The next bale fires 14.4 frames later.

Call-site assembly (from decompiled Mission_Deploy_Building state 3):
```c
if (*(double *)(g_RulesClass_Instance + 0x1528) * _DAT_007e27f8 <= (double)param_1[0x3e]) {
    // fire vtable+0x468 (silo update)
    iVar3 = StorageClass__FindFirstNonEmptySlot();   // on harvester (param_1)
    // ... purifier calc, RemoveAmount, Add_Tiberium_Credits
    param_1[0x3e] = 0;   // reset step counter
}
```

### (b) Per-bale credit calculation

Both paths use the same formula via `HouseClass__Add_Tiberium_Credits` (0x4F9610):

```
credits_added = (int)(TiberiumClass[tibType]->Value * HouseTypeClass->IncomeMult * amount)
```

- Each "bale" is a `float` amount stored in a StorageClass slot indexed by tiberium type (0–3)
- The bale is NOT tagged with its type at storage time — the type IS the slot index
- TiberiumClass->Value is at TibClass+0xB8 (int): Ore=25, Gems=50
- HouseTypeClass->IncomeMult at HouseType+0x148 (float, default 1.0)

Purifier bonus (same in both paths):
```
purifierBonus = (float)storageCapacity * RulesClass[0xF3C] * amount
```
Where `storageCapacity = HouseClass+0x538C`, optionally +AIVirtualPurifiers[difficulty]
for non-human players when `g_GameMode != 0`.

### (c) Owner-credit-add call

**Function:** `HouseClass__Add_Tiberium_Credits` at **0x004F9610**
**Signature:** `__thiscall void(float amount, int tibType)` (ECX = HouseClass*)

Called twice per dump event (base + purifier bonus if > 0). See ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md §3 for full assembly. No MaxCash check inside this function — it unconditionally adds to `HouseClass+0x30C` (Balance). There is no credit cap enforced at deposit time.

### (d) StorageClass layout and location

**StorageClass** is a 16-byte struct: 4 `float` slots indexed [0..3] for tiberium types:

| Offset | Field     | Tiberium type       |
|--------|-----------|---------------------|
| +0x00  | Amount[0] | Riparius / Ore      |
| +0x04  | Amount[1] | Cruentus / Gems     |
| +0x08  | Amount[2] | Vinifera            |
| +0x0C  | Amount[3] | Aboreus             |

**Storage location by unit path:**
- **Slave miner path:** Storage is on the **BuildingClass** (refinery) at BuildingClass+0x33C.
  Slaves deposit ore into the building's buffer; `DepositOreFromStorage` drains it.
- **Harvester / Chrono Miner path:** Storage is on the **harvester unit** itself at
  UnitClass+0x33C. Ore is added during `Harvest_Ore_Tick` directly to the unit's own
  StorageClass; `Mission_Deploy_Building` drains it while the unit is on the dock cell.

CMIN has `Storage=500` (credits capacity) at TechnoTypeClass+0x800 → 500/25 = 20 bales of ore
(type 0), or 10 bales of gems (type 1). The STORAGE IS IN CREDITS, not bale count directly.

### (e) HarvesterDumpRate location

**RulesClass+0x1528** — a `double` read via `CCINIClass::ReadDouble` at 0x670CD4.
INI key: `[General] HarvesterDumpRate=0.016` (minutes per bale).

Verified from Mission_Deploy_Building state 3 timer check:
```c
*(double *)(g_RulesClass_Instance + 0x1528) * _DAT_007e27f8 <= (double)param_1[0x3e]
// 0.016 * 900.0 = 14.4 frames threshold
```

This is a **RulesClass field**, NOT a TechnoTypeClass field. All harvesters share the same
rate regardless of type. The constant 900.0 is at `0x007E27F8` = 60 sec × 15 fps.

The harvester's step counter is `param_1[0x3e]` (UnitClass+0xF8), incremented every frame
(CDTimer at +0x40/0x42/0x43 configured to fire every 1 frame during the dump phase).

### (f) Animation triggering

**DepositOreFromStorage does NOT trigger animations.** It only calls `vtable+0x468` on the
building if `anyDeposited == true`.

`vtable+0x468` resolves to `HouseClass__Notify_Credit_State_Change` (0x4F9970), which iterates
all the house's buildings and triggers animation slot updates (SetAnimSlot command 2) on silo
buildings whose `Type+0x16A8` (PowersUpBuilding/SiloContributes flag) is set.

The **dock animations** (smoke puff, DOORAG, refinery animation slots 7, 8, 10) are triggered
separately in `UnitClass::Mission_Deploy_Building`:
- **Slot 7** (approach anim) set on entry to dump state (harvester-only path)
- **Slot 10** (active dump anim) set each dump tick when `building->field_0x584 == 0`
- **Slot 8** (empty/idle anim) restored when storage is fully drained
- These are BuildingClass::SetAnimSlotImage calls directly in Mission_Deploy_Building,
  conditioned on `Type+0x16BB` (Refinery flag) and building health ratio vs `Rules+0x1700`

The dock entry animation (`[General] DockAnim`) is created in `BuildingClass::EnterTransport`
(0x70FD70), not in DepositOreFromStorage.

### (g) Edge cases

**Empty storage:** `StorageClass__FindFirstNonEmptySlot` returns -1 immediately; the while
loop never executes; `bVar3` stays false; `vtable+0x468` not called. For the harvester path:
the `iVar3 == -1` check skips credit deposits and proceeds to state 4 (undock). No crash.

**MaxCash:** No MaxCash enforcement at deposit time. `HouseClass__Add_Tiberium_Credits`
unconditionally adds to Balance. No cap check present in the function (verified from assembly
in ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md §3).

**Refinery destroyed mid-unload:** The building pointer `this_00` comes from
`Look_up_building_in_cell()`. If the building is destroyed, `Look_up_building_in_cell()` 
returns null. The null check `if (this_00 != NULL)` causes the dump branch to be skipped;
`PathType__Has_Valid_Steps` is checked instead, and the harvester is sent to Guard/Harvest.
Ore remaining in the harvester's StorageClass is preserved — it is NOT lost. The harvester
will carry remaining ore to the next refinery it docks at. (Confirmed consistent with
MINER_DOCK_GAPS_RESEARCH.md Case C, but clarified: partial ore is retained on the harvester,
not lost.)

---

## Key Verified Facts (load-bearing)

1. **0x522D50 is slave-miner-only.** Xref check: one caller only (SlaveManagerClass::AI_Update
   at 0x6AFB D2 state 4). Chrono miner / HARV use `UnitClass::Mission_Deploy_Building` inline.

2. **HarvesterDumpRate = RulesClass+0x1528 (double, 0.016 min/bale).** Confirmed from
   Mission_Deploy_Building timer: `RulesClass[0x1528] * 900.0 ≤ param_1[0x3e]` → 14.4 frames.

3. **Credit formula verified:** `Balance += (int)(TiberiumClass[type]->Value * IncomeMult * amount)`
   via HouseClass__Add_Tiberium_Credits (0x4F9610). No MaxCash cap at deposit.

4. **Harvester storage is on the unit (UnitClass+0x33C).** Mission_Deploy_Building calls
   `StorageClass__FindFirstNonEmptySlot` with ECX = param_1 (the harvester). Storage is NOT
   transferred to the building before crediting; it is drained directly from the unit.

5. **Refinery destroyed mid-unload: harvester retains remaining ore.** `Look_up_building_in_cell()`
   returns null; dump branch skipped; unit transitions to Guard; StorageClass on unit is intact.

---

## Relation to Existing Docs

- **ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md §5** contains a pseudocode of DepositOreFromStorage
  that is accurate for the slave miner path. It incorrectly implies this is the main harvester
  path; the call-site verification in this report corrects that.
- **HARVESTER_DOCK_UNLOAD.md §2.3** correctly identifies `UnitClass::Mission_Deploy_Building`
  as the harvester dump path. The "CORRECTION" in that doc is verified correct.
- **MINER_DOCK_GAPS_RESEARCH.md §Case C** is confirmed with the clarification that remaining
  ore stays on the harvester (not lost).

---

## Status: COMPLETE
