# BuildingClass Verification Round — Ghidra Report

Verification date: 2026-04-19
Source: live Ghidra MCP decompilation of `gamemd.exe`

This report directly verifies (or debunks) nine contested claims from prior BuildingClass
research. Each claim is reconciled against the actual binary. Per-claim confidence level:
HIGH = confirmed by raw disassembly or explicit Ghidra labels; MEDIUM = strongly implied
but setter site not located; LOW = insufficient evidence.

---

## Claim 1 — ChargeFlags 21-byte array at BuildingClass+0x5B0

**Verdict: DEBUNKED (HIGH confidence).**

No function in the BuildingClass code path (constructor, destructor, Update, UpdateAnimation,
GoOnline, GoOffline, RestoreOnlineEffects, ApplyOfflineEffects, SetAnimSlotImage,
CreateAnimForSlot, ClearAnimSlot, UpdateRepairAndPower, PowerCheck_Upgrade,
UpdateGapGenerator_Tick, ReadFromINI, OnConstructionComplete, OnDestroyed, ChangeOwner,
GetWeapon, GetOccupantCount, Mission_RepairAndProduce, ProcessDelayedFire) reads or writes
the byte range +0x5B0 through +0x5C4 as a 21-byte array.

The "21 slots" idea appears to come from the `Anims_0` array of 21 animation slots. The real
layout in the BuildingClass constructor at 0x0043B740 is:
- `param_1 + 0x157` (byte +0x55C), zero 21 DWORDs → +0x55C..+0x5AF covers the 21 Anim slots,
  not +0x5B0.
- `param_1 + 0x172` (byte +0x5C8), zero 8 DWORDs → +0x5C8..+0x5E7 is an 8-slot pointer array
  released in the destructor via `(**(code **)(*piVar6 + 0xf8))()`.

The region +0x5B0..+0x5C7 appears to be scattered scalar fields, not a cohesive byte array.
Anim "PoweredEffect charge" state, if tracked at all, is not via a 21-byte array at +0x5B0.

---

## Claim 2 — BuildingClass+0x524 is FactoryClass pointer

**Verdict: CONFIRMED (HIGH confidence for identity, MEDIUM for assignment path).**

Destructor at 0x0043BCF0 shows the exact pattern of releasing a dynamically-allocated owned
object:
```
0043bf40: MOV ECX, dword ptr [ESI + 0x524]
0043bf46: CMP ECX, EBP                           ; EBP = 0
0043bf48: JZ  ...                                ; skip if null
0043bf4e: CALL dword ptr [EDX + 0x20]            ; virtual destructor (slot 0x20)
0043bf51: MOV dword ptr [ESI + 0x524], EBP       ; null it out
```

Ghidra's pre-annotated struct labels this field `Factory`. In the decompiled destructor it
appears as `param_1->Factory`. This matches the classic FactoryClass-owned-by-BuildingClass
teardown pattern.

The assignment site (where `SomeFactory*` is written into +0x524) was NOT found in the primary
BuildingClass paths inspected (OnConstructionComplete, Unlimbo, MissionRepairAndProduce).
The assignment almost certainly happens in a HouseClass production-dispatch path that calls
out to this building (not sampled in this round). `UpdateGarrisonFire` at 0x0043E7B0 reads
`this->Factory` directly, confirming the runtime field is live.

**Docs need update**: state that +0x524 IS `FactoryClass*`, owned by the building, destroyed
via virtual slot 0x20 in the destructor, but flag the assignment path as "unverified — likely
from HouseClass production dispatcher."

---

## Claim 3 — BuildingLight at +0x600

**Verdict: CONFIRMED WITH CORRECTION (HIGH confidence).** Two different light objects exist.

### BuildingClass+0x600 — `BuildingLightClass*` (spotlight)

- Created in `BuildingClass::Unlimbo` at 0x00441187 **only if** `Type+0x154B (HasSpotlight) != 0`.
- Allocated with `operator_new(0xE8)` (232 bytes), then `BuildingLightClass__Constructor`
  (0x00435820) is called.
- Stored at `[ESI + 0x600]`.
- This is the directional spotlight seen on prism towers / guard towers / oil derricks.
- **Not every building has one** — only those with `HasSpotlight=yes` in `art(md).ini`.

### BuildingClass+0x614 — `LightSourceClass*` (ambient radiosity light)

- Created in `BuildingClass::Unlimbo` at 0x00440DE6 via `LightSourceClass__Constructor`
  (0x00554760). This is unconditional for all buildings with a non-default
  `LightVisibility > 0` (rulesmd.ini BuildingLight keys at Type+0xE30/+0xE34/+0xE38/+0xE3C).
- Ghidra labels the field `LightSource` in the struct; raw offset is +0x614.
- Destroyed in the destructor at 0x0043BD3A:
  ```
  MOV ECX, dword ptr [ESI + 0x614]
  PUSH 0 ; CALL FUN_00554a80   ; detach from global light list
  CALL dword ptr [EAX + 0x20]  ; virtual destructor
  MOV dword ptr [ESI + 0x614], 0
  ```
- The fact that the destructor only cleans +0x614 (not +0x600) suggests
  `BuildingLightClass*` at +0x600 is either torn down earlier (e.g. in OnDestroyed) or owned
  elsewhere — worth a follow-up trace but out of scope here.

### INI data lives on BuildingTypeClass, not BuildingClass

The `BuildingLight` INI section (`LightVisibility`, `LightIntensity`, `LightRedTint`,
`LightGreenTint`, `LightBlueTint`) reads into `BuildingTypeClass+0xE30..+0xE3C+`. These are
the per-type parameters used when constructing the runtime LightSource at +0x614.

**Docs need update**: distinguish `BuildingLightClass*` at +0x600 (conditional, HasSpotlight)
from `LightSourceClass*` at +0x614 (default ambient light). Prior docs appear to have
conflated these two.

---

## Claim 4 — Bio-reactor occupant count at BuildingClass+0x114 vs +0x66C vector

**Verdict: +0x114 IS the count (HIGH confidence). +0x66C is NOT involved in this formula.**

`BuildingClass__GetPowerOutput` (0x0044E7B0) decompiles to:
```c
iVar5 = *(int *)(this->Type + 0xee0);           // base Power= from TypeClass
// [HasExtraPowerBonus branch skipped]
if ((((puVar1[0x16ae] != '\0') ||               // Type.UnitAbsorb
      (puVar1[0x16af] != '\0')) &&              // Type.InfantryAbsorb
     (0 < *(int *)(puVar1 + 0xee8))) &&         // Type.ExtraPower > 0
    (0 < *(int *)&this->field_0x114)) {         // BuildingClass+0x114 > 0
    iVar5 = iVar5 + *(int *)(puVar1 + 0xee8)    // ExtraPower *
                    * *(int *)&this->field_0x114;  // BuildingClass+0x114
}
```

- **BuildingTypeClass+0xEE0 = Power** (base output)
- **BuildingTypeClass+0xEE8 = ExtraPower** (per-occupant multiplier)
- **BuildingTypeClass+0x16AE = UnitAbsorb** (bool, enables bio-reactor for vehicles)
- **BuildingTypeClass+0x16AF = InfantryAbsorb** (bool, enables bio-reactor for infantry)
- **BuildingClass+0x114 = scalar occupant count** used as the ExtraPower multiplier.

The +0x66C DynamicVector hypothesis is incorrect for this formula. The destructor shows +0x66C
is actually a VectorClass/DynamicVectorClass base (seen clearing via
`*(undefined ***)puVar1 = &PTR_FUN_007e43e8;` where `puVar1 = &param_1->field_0x66c`). The
precise content of that vector was not identified in this round but it is not the bio-reactor
occupant count.

---

## Claim 5 — GarrisonFireIndex at +0x664 vs +0x69C

**Verdict: +0x69C is correct (HIGH confidence). +0x664 is WRONG.**

`BuildingClass::GetWeapon` (0x004526F0) decompiles to:
```c
// param_1 is int *  (DWORD-indexed)
if (param_1[0x1a5] <= param_1[0x1a7]) {                   // count <= index ? fall through
    piVar3 = TechnoClass__GetWeapon(param_2);
} else {
    piVar2 = *(int **)(param_1[0x1a2] + param_1[0x1a7] * 4);  // occupants[index]
    // ...pick weapon from that occupant's TechnoTypeClass...
}
```

Byte offsets:
- `param_1[0x1a2]` → +0x688 → pointer to occupant array (DynamicVector base)
- `param_1[0x1a5]` → +0x694 → occupant **count**
- `param_1[0x1a6]` → +0x698 → likely capacity
- `param_1[0x1a7]` → **+0x69C → garrison fire round-robin index**

The field named `UpdateGarrisonFire` at 0x0043E7B0 is actually the factory queue preview
DRAWING function — it calls `FactoryClass__GetObject` then `CC_Draw_Shape`. This is a
misnomer in the existing Ghidra labels and is unrelated to garrison firing. The real fire
path goes through `GetWeapon` (verified) and `TechnoClass::Fire_At` (not re-examined here
but should use the same occupant-indexing logic).

**Docs need update**: GarrisonFireIndex is at **BuildingClass+0x69C**, not +0x664. Also,
consider renaming `BuildingClass__UpdateGarrisonFire` at 0x0043E7B0 — it doesn't update
garrison fire.

---

## Claim 6 — SecretLab stores TechnoTypeClass* at BuildingClass+0x6F4

**Verdict: UNVERIFIED (LOW confidence). Field exists and is zeroed at construction, but no
writer was found.**

- Constructor at 0x0043B9B5 zeroes `[ESI + 0x6F4]` (EBX = 0).
- No scanned BuildingClass function (Unlimbo, OnConstructionComplete, Update,
  MissionRepairAndProduce, ChangeOwner, ReadFromINI, GoOnline, GoOffline,
  RestoreOnlineEffects, ApplyOfflineEffects, OnSpyInfiltrate, IronCurtain,
  GetSuperWeaponIndex1/2, etc.) writes to +0x6F4.
- `SecretLab` INI key at Type+0x16B0 (confirmed by string 0x81AAA0) gates the Unlimbo
  registration (HouseClass secret-lab list push), but that does NOT write to BuildingClass+0x6F4.
- The three Secret pool configuration fields are on **BuildingTypeClass** (NOT BuildingClass):
  - Type+0xEA4 = `SecretInfantry` (InfantryTypeClass*, looked up via FUN_00524cb0)
  - Type+0xEA8 = `SecretUnit` (UnitTypeClass*, looked up via FUN_007480d0)
  - Type+0xEAC = `SecretBuilding` (BuildingTypeClass*, looked up via FUN_00466000)

The runtime "chosen secret" (if it even resides on BuildingClass rather than being computed
on-reveal from the Type's pools + a Random roll) needs a dedicated trace. Candidate functions
to investigate next: callers of the Secret pool lookup helpers, RulesClass at +0x???
(SecretBuildings/SecretUnits vectors), and HouseClass Init_Random.

**Docs need update**: Do NOT claim +0x6F4 holds a TechnoTypeClass* without a verified setter.
Label it `unknown_dword_6f4` until a write site is located.

---

## Claim 7 — Default value of CloakRadiusInCells (Type+0x1707)

**Verdict: DEFAULT IS 20 (HIGH confidence). The "0" default claim is WRONG.**

BuildingTypeClass constructor at 0x0045DD90 initializes:
```c
*(undefined1 *)((int)param_1 + 0x1707) = 0x14;   // 20
```
Note: `param_1` is `undefined4 *`; the `(int)` cast means this is a **direct byte offset**.

BuildingTypeClass::ReadINI at 0x0045FE50 reads the key via CCINIClass::ReadInt, passing the
current signed-byte value as the default:
```
00460c19: MOVSX EAX, byte ptr [EBP + 0x1707]    ; current byte, sign-extended
00460c20: PUSH EAX
00460c21: PUSH 0x81a978                          ; "CloakRadiusInCells"
00460c26: PUSH EBX                               ; INI section
00460c27: CALL 0x005276d0                        ; CCINIClass::ReadInt
00460c32: MOV byte ptr [EBP + 0x1707], AL        ; store low byte of result
```

The field IS a **signed byte**. Reading is done as int then truncated. Default = 20. Docs
saying 0 are wrong.

---

## Claim 8 — Gap generator flag

**Verdict: CORRECT FIELD IDENTIFIED (HIGH confidence). Neither +0x40C nor +0x16C8.**

The field that gates gap generator behavior in `BuildingClass::UpdateGapGenerator_Tick`
(0x00454DB0) is:

- **Type+0x16C7 = CloakGenerator** (bool) — INI key "CloakGenerator" (string 0x81A998).
- **Type+0x1707 = CloakRadiusInCells** (signed byte) — the gap/cloak radius in cells.

For clarity on adjacent flags:
- **Type+0x16C6** = (prior flag, not checked)
- **Type+0x16C7** = CloakGenerator
- **Type+0x16C8** = SensorArray (INI key "SensorArray", string 0x81A98C) — separate detect-cloak flag.

Evidence: UpdateGapGenerator_Tick line 163 checks `*(char *)(param_1[0x148] + 0x16c7) != '\0'`
before running the gap radius logic; the radius arg passed to `FUN_007bb920` is
`*(char *)(unaff_EBX[0x148] + 0x1707)` = CloakRadiusInCells.

The destructor also gates cloak/gap teardown on `Type[0x16c7] != '\0'` and reads
CloakRadiusInCells (Type+0x1707) into `BuildingClass+0x6EC` before calling
`UpdateGapGenerator_Tick(1)`.

**Docs need update**:
- `+0x40C` references for gap generator are TS-legacy (likely robot-tank UnitType*); do not
  use in YR.
- `+0x16C7 = CloakGenerator`, NOT SensorArray.
- `+0x16C8 = SensorArray`, NOT CloakGenerator.
- Gap radius = `CloakRadiusInCells` = Type+0x1707 (signed byte, default 20).

---

## Claim 9 — Type+0x16A9 is UnitRepair, not CanC4 or garrison fire

**Verdict: +0x16A9 is UnitRepair (HIGH confidence). Prior "CanC4" label is WRONG.**

BuildingTypeClass::ReadINI at 0x0045FE50:
```
00460906: MOV CL, byte ptr [EBP + 0x16a9]       ; current value
0046090c: PUSH ECX
         PUSH 0x81aaf0                           ; "UnitRepair"
         ...CCINIClass::ReadBool...
00460929: MOV byte ptr [EBP + 0x16a9], AL        ; store result
```

Confirmed offsets in this block:
- **Type+0x16A9 = UnitRepair** (INI key "UnitRepair", string 0x81AAF0) — used by service depots
- **Type+0x16AA = UnitReload** (INI key "UnitReload", string 0x81AAE4)

For completeness, CanC4 actually lives at:
- **Type+0x1577 = CanC4** (INI key "CanC4", string 0x81ADFC) — in an earlier bool block.

**Docs need update**: Correct the +0x16A9 label to `UnitRepair`. CanC4 is a completely different
offset.

---

## Summary table

| Claim | Field | Verdict | Confidence |
|---|---|---|---|
| 1 | BuildingClass+0x5B0..+0x5C4 ChargeFlags | **DEBUNKED** — no such array | HIGH |
| 2 | BuildingClass+0x524 = FactoryClass* | **CONFIRMED** (Ghidra labeled) | HIGH identity / MED assignment path |
| 3 | BuildingClass+0x600 = BuildingLight | **CONFIRMED WITH CORRECTION** — +0x600 is `BuildingLightClass*` (HasSpotlight only); +0x614 is `LightSourceClass*` (default) | HIGH |
| 4 | BuildingClass+0x114 = bio-reactor count | **CONFIRMED** | HIGH |
| 5 | BuildingClass+0x69C = GarrisonFireIndex | **CONFIRMED** (+0x664 is wrong) | HIGH |
| 6 | BuildingClass+0x6F4 = SecretLab choice | **UNVERIFIED** — field is zeroed at ctor, no writer found | LOW |
| 7 | BuildingTypeClass+0x1707 CloakRadiusInCells default | **20** (not 0) | HIGH |
| 8 | Gap generator flag: +0x16C7 = CloakGenerator; radius = +0x1707 | **CONFIRMED** (+0x40C is TS legacy) | HIGH |
| 9 | BuildingTypeClass+0x16A9 = UnitRepair | **CONFIRMED** (not CanC4) | HIGH |

## Confirmed BuildingTypeClass offsets (for cross-reference)

| Offset | Field | INI Key | Notes |
|---|---|---|---|
| +0x154B | HasSpotlight | HasSpotlight | gates spotlight at BuildingClass+0x600 |
| +0x1577 | CanC4 | CanC4 | default 1 |
| +0xEA4 | SecretInfantry | SecretInfantry | InfantryTypeClass* |
| +0xEA8 | SecretUnit | SecretUnit | UnitTypeClass* |
| +0xEAC | SecretBuilding | SecretBuilding | BuildingTypeClass* |
| +0xE30 | LightVisibility | LightVisibility | int |
| +0xE34 | LightIntensity | LightIntensity | int (scaled) |
| +0xE38 | LightRedTint | LightRedTint | int (scaled) |
| +0xE3C | LightGreenTint | LightGreenTint | int (scaled) |
| +0xEE0 | Power | Power= | base power output |
| +0xEE8 | ExtraPower | ExtraPower= | per-occupant bonus (bio-reactor) |
| +0x16AE | UnitAbsorb | UnitAbsorb | bio-reactor for vehicles |
| +0x16AF | InfantryAbsorb | InfantryAbsorb | bio-reactor for infantry |
| +0x16B0 | SecretLab | SecretLab | bool, gates secret-lab list registration |
| +0x16A9 | UnitRepair | UnitRepair | service depot |
| +0x16AA | UnitReload | UnitReload | |
| +0x16C7 | CloakGenerator | CloakGenerator | gates gap generator tick |
| +0x16C8 | SensorArray | SensorArray | detect cloak |
| +0x16A4 | Radar | Radar | |
| +0x16A5 | SpySat | SpySat | |
| +0x1707 | CloakRadiusInCells | CloakRadiusInCells | signed byte, default 20 |

## Confirmed BuildingClass offsets

| Offset | Field | Notes |
|---|---|---|
| +0x114 | bio-reactor occupant count | multiplier for ExtraPower |
| +0x148 (as int* index) / +0x520 | Type (BuildingTypeClass*) | |
| +0x524 | Factory (FactoryClass*) | destructor virtual-releases via slot 0x20 |
| +0x55C | Anims_0..Anims_20 (21 DWORD slots) | anim pointer array |
| +0x5C8 | 8-DWORD pointer array | released in destructor via virtual slot 0xF8 |
| +0x600 | BuildingLightClass* (spotlight) | only if Type.HasSpotlight |
| +0x614 | LightSourceClass* | default ambient light (all buildings) |
| +0x688 | occupants array (DynamicVector base) | param_1[0x1a2] |
| +0x694 | occupant count | param_1[0x1a5] |
| +0x698 | occupant capacity (likely) | param_1[0x1a6] |
| +0x69C | GarrisonFireIndex | round-robin index param_1[0x1a7] |
| +0x6EC | runtime cloak radius (copy of Type+0x1707) | set in cloak-gen destroy path |
| +0x702 | Occupants count (char) | used in GetWeapon early-exit |
| +0x6F4 | unknown DWORD | zeroed at ctor; NO confirmed writer found |

## Recommended Ghidra label corrections

- `BuildingClass__UpdateGarrisonFire` (0x0043E7B0) → should be renamed
  `BuildingClass__DrawFactoryQueuePreview` — this function draws the queued-unit preview via
  `FactoryClass__GetObject` + `CC_Draw_Shape`, not garrison firing.
- BuildingClass.LightSource at +0x614 — already labeled correctly.
- BuildingClass+0x600 — should be labeled `Spotlight` or `BuildingLight`.
- BuildingTypeClass+0x16A9 — if not yet labeled, set to `UnitRepair`.
- BuildingTypeClass+0x16C7 — label as `CloakGenerator`.
- BuildingTypeClass+0x16C8 — label as `SensorArray`.
- BuildingTypeClass+0x1707 — label as `CloakRadiusInCells` (signed byte).
