# BuildingClass Field Verification — Round 2

Re-verification of critical BuildingClass field offsets by independent cross-checking
across multiple call sites in gamemd.exe. Each field was verified by decompiling at
least 2 independent functions that read/write it (plus disassembly inspection where
struct metadata disagreed with claims).

Ghidra has an applied `BuildingClass` struct (size 1820 bytes) with labelled fields —
this report cross-references that struct against the original claims and, where they
disagree, trusts the raw disassembly.

---

## Field 1 — `+0x114` — Claim: "Bio-reactor occupant count"

**Verdict: HIGH confidence, but mis-named.**

The field IS used as the bio-reactor occupant multiplier by `GetPowerOutput`, but
it is NOT a bio-reactor-specific field. It is the **embedded `CargoClass` sub-object**
inherited from `TechnoClass` — specifically the `NumPassengers` count field at offset 0
inside the embedded CargoClass.

### Evidence

| Site | Address | Access | Interpretation |
|------|---------|--------|----------------|
| `BuildingClass::GetPowerOutput` | `0x0044e7b0` | READ `[ESI+0x114]` as int | Multiplies `Type+0xee8` (PowerBonus) when `Type+0x16ae` or `Type+0x16af` set |
| `BuildingClass::Receive_Radio` | `0x0043c2d0` | READ `[ESI+0x114]` | Capacity check: `*(int*)(this+0x114) + 1 <= Type+0x5e0` to accept entering infantry |
| `FUN_0065d8e0` (parachute-drop / absorb path) | `0x0065dad7`, `0x0065dce3` | `LEA ECX, [r+0x114]` then `CALL CargoClass::AddPassenger (0x004733a0)` | Passes `this+0x114` as the `CargoClass*` `this` pointer to increment NumPassengers |
| `AircraftClass::Carryall_Pickup`, `Mission_Deploy_Building`, etc. | many | `LEA r, [ESI+0x114]` then `CALL 0x004733a0` | Same pattern — `this+0x114` IS the `CargoClass*` |

### Disassembly confirmation at `0x0044e813`
```
MOV ECX, dword ptr [ESI + 0x114]
TEST ECX, ECX
JLE ...
```
Read as signed int, used as a multiplier — consistent with `CargoClass::NumPassengers`.

### Structural interpretation
`CargoClass` is a 2-field struct `{ int NumPassengers; TechnoClass* FirstPassenger; }`.
Embedded in `TechnoClass` at offset `+0x114`, so:
- `BuildingClass + 0x114` = `CargoClass.NumPassengers`
- `BuildingClass + 0x118` = `CargoClass.FirstPassenger`

### Is this TechnoClass or BuildingClass-specific?
**Inherited from TechnoClass.** `CargoClass::AddPassenger` at `0x004733a0` is called from
14+ sites across `AircraftClass`, `BuildingClass`, `UnitClass` and `InfantryClass` —
confirming the field layout is a base-class mechanism, not a BuildingClass extension.

### Bio-reactor write path
No direct instruction writes to `[r+0x114]` as an integer. Infantry enter bio-reactors
via the generic `CargoClass::AddPassenger` helper (`LEA r, [building+0x114]; CALL 0x004733a0`),
which increments `*param_1` (the NumPassengers field). `FUN_0044db00` (the
`InfantryClass::Mission_Enter` path for Bio Reactor / Absorber) hands off to this
cargo-add mechanism via vtable calls rather than touching +0x114 directly.

### Difference vs +0x694
| Field | Used by | Meaning |
|-------|---------|---------|
| `+0x114` | Bio-Reactor (`Type+0x16ae`), Absorber (`Type+0x16af`) | `CargoClass::NumPassengers` — reused for occupant count |
| `+0x694` | Garrisonable civilian buildings (gated by `InfantryType+0xeb4 = Occupier`) | DynamicVector<InfantryClass*>::Count at `+0x684..+0x69C` |

These are **completely separate systems**. Bio-reactor uses CargoClass (shared with
transport helos), garrison uses a dedicated DynamicVector.

---

## Field 2 — `+0x69C` — Claim: "GarrisonFireIndex"

**Verdict: HIGH confidence.**

### Evidence

| Site | Address | Access | Interpretation |
|------|---------|--------|----------------|
| `BuildingClass::GetWeapon` | `0x004526f0` | READ `this[0x1a7]` (= `+0x69c`) as array index | `if (this->Count <= this->FireIdx) fall through to Techno weapon; else pick `Occupants[FireIdx]`'s primary/elite weapon` |
| `TechnoClass::Fire_At` | `0x006fdd50` lines 630-632 | `this[0x1a7]++; this[0x1a7] %= vtable[0x408]();` | Post-fire: increment, wrap modulo `GetWeaponCount()` |
| `BuildingClass::Constructor` | `0x0043b740` | `param_1[0x1a7] = 0` | Init to 0 |

### Layout (confirmed via DynamicVector pattern in disassembly)
```
+0x684  DynamicVector<InfantryClass*> vtable
+0x688  array pointer
+0x68C  Capacity
+0x690  IsInitialized (byte)
+0x691  IsAllocated (byte)          (also touched by AddGarrisonOccupant)
+0x694  Count                       ← GetOccupantCount returns this
+0x698  CapacityIncrement
+0x69C  CurrentFireIdx (int)        ← This field
```

The adjacency to the DynamicVector makes any alternative offset implausible —
`+0x69C` is the next int after the vector's CapacityIncrement, and `GetWeapon` uses
it as a bounds-checked array index into `[EBP+0x688]` (the occupant array).

### Candidate alternative offsets near +0x69C?
None — the DynamicVector footprint fixes the layout up to `+0x69B`, and the next
aligned int location IS `+0x69C`. Both GetWeapon and Fire_At consistently target it.

---

## Field 3 — `+0x660`/`+0x661`/`+0x662` — Claim: "HasPower / HasExtraPowerBonus / HasExtraPowerDrain"

**Verdict: PARTIALLY WRONG. The layout is off by one grouping.**

### Correct layout (verified from Ghidra struct + raw disassembly)
| Offset | Field | Verified where |
|--------|-------|----------------|
| `+0x660` | `HasPower` (bool) | GoOnline `0x00452263/0x00452287`, GoOffline `0x00452363/0x00452393`, GetPowerOutput `0x0044e831`, GetPowerDrain `0x0044e88f`, PowerCheck_Upgrade `0x00450605` |
| `+0x661` | `IsOverpowered` (bool) | PowerCheck_Upgrade `0x00450614`/`0x0045061e` writes 0/1; constructor `0x0043b740` inits to 0 |
| `+0x662` | (unlabeled byte — cleared by constructor; no read site found in power path) | Constructor writes 0 at `+0x662`; no read use in GetPower*/GoOnline/GoOffline/PowerCheck_Upgrade |
| `+0x668` | `HasExtraPowerBonus` (bool) | GetPowerOutput `0x0044e7d5`: `MOV AL, byte ptr [ESI + 0x668]` — then adds `Type+0xee8` (PowerBonus) |
| `+0x669` | `HasExtraPowerDrain` (bool) | GetPowerDrain `0x0044e89f`: `MOV DL, byte ptr [ESI + 0x669]` — then adds `Type+0xeec` (PowerDrain) |

### Disassembly extracts

**`GetPowerOutput` (`0x0044e7d5`)**
```
MOV AL,byte ptr [ESI + 0x668]     ; HasExtraPowerBonus
TEST AL,AL
JZ ...
ADD EDI, dword ptr [EAX + 0xee8]  ; add Type.PowerBonus
```

**`GetPowerDrain` (`0x0044e89f`)**
```
MOV DL,byte ptr [ESI + 0x669]     ; HasExtraPowerDrain
TEST DL,DL
JZ ...
ADD EAX, dword ptr [ECX + 0xeec]  ; add Type.PowerDrain
```

**`PowerCheck_Upgrade` (`0x0045060f..0x0045061e`)**
```
MOV byte ptr [ESI + 0x661], 0x0   ; ← writes +0x661 (IsOverpowered), NOT +0x668
...
MOV byte ptr [ESI + 0x661], 0x1
```

**`GoOnline` (`0x00452287`)**
```
MOV byte ptr [ESI + 0x660], AL    ; writes HasPower ← confirms +0x660
```

### Correction
The original claim that `+0x661 = HasExtraPowerBonus` and `+0x662 = HasExtraPowerDrain`
is **wrong**. The ExtraPowerBonus/Drain flags are at `+0x668`/`+0x669`. Offset `+0x661`
is `IsOverpowered` (a PowerCheck_Upgrade-maintained flag indicating the building has
≥3 power-up upgrades AND the house power ratio is 1.0). Offset `+0x662` is a byte
cleared by the constructor but otherwise unused in the power-management paths checked.

---

## Field 4 — `+0x600` (spotlight) vs `+0x614` (LightSource)

**Verdict: HIGH confidence — two separate pointer fields of different types.**

### Evidence

| Site | Address | +0x600 access | +0x614 access |
|------|---------|---------------|---------------|
| `BuildingClass::Unlimbo` | `0x00440580` | line 325: `param_1[0x180] = BuildingLightClass__Constructor(param_1)` — allocates when `Type+0x154b != 0` | line 209: `param_1[0x185] = LightSourceClass__Constructor(...)` — allocates for lamp-post-style buildings |
| `BuildingClass::PointerExpired` (mislabelled "GetType" at `0x0044e8f0`) | — | `if (field_0x600 == expired) field_0x600 = 0` | `if (LightSource == expired) LightSource = 0` |
| `BuildingClass::GoOnline` | `0x004522b1` | — | `MOV ECX, dword ptr [ESI + 0x614]; TEST ECX, ECX; JZ ...; PUSH 0; CALL 0x00554a60` — enables LightSource |
| `BuildingClass::ApplyOfflineEffects` | `0x00452480` | — | `if (this->LightSource != 0) FUN_00554a80(0)` — disables |
| `BuildingClass` destructor | `0x0043bcf0` | — | `FUN_00554a80(0); (**(vtable+0x20))(1); LightSource = 0` — releases via vtable slot 0x20 |

### Types
- `+0x600` → `BuildingLightClass*` — allocated via `BuildingLightClass__Constructor` at `0x00435820`. BuildingLightClass has `AI`, `Draw_It`, `Detach`, `FindTarget`, `RecalcArcPositions`, `DistanceToIntensity`, `Load`, `Save` methods — this is the **spotlight/rotating-arc light** class (used by Prism Tower / searchlight buildings).
- `+0x614` → `LightSourceClass*` — allocated via `LightSourceClass__Constructor` at `0x00554760`. LightSourceClass is the **radius-based ambient light emitter** (lamp posts, light-emitting buildings) with tint/intensity/radius fields from `Type+0xe30..0xe40`.

### Allocation gating
- `+0x614` (LightSource): allocated in Unlimbo when the building's Type has any of `Type+0xe30..0xe40` non-zero (LightIntensity, LightRedTint, LightGreenTint, LightBlueTint, LightVisibility). NOT universal — only lamp-like buildings.
- `+0x600` (BuildingLight/Spotlight): allocated in Unlimbo when `Type+0x154b != 0` (the `HasSpotlight=` flag — Prism Tower etc.). Much more selective.

Neither is universal — both are conditional on Type flags. Most buildings will have both NULL.

---

## Field 5 — `+0x524` — Claim: "`FactoryClass*` pointer"

**Verdict: HIGH confidence.**

Confirmed via Ghidra struct (`Factory` at byte 1316 = `0x524`) and cross-referenced
with writers:

### Assignment
`FUN_004500f0` (Mission_RepairAndProduce / BuildingClass production driver), lines
103-107:
```c
pvVar5 = operator_new(0x74);
if (pvVar5 == 0) pFVar6 = 0;
else pFVar6 = FactoryClass__Constructor();   // 0x004c98b0
param_1[0x149] = (int)pFVar6;                // ← write to +0x524
```

Clears on completion/abandonment (lines 42, 143):
```c
FactoryClass__AbandonProduction((FactoryClass*)param_1[0x149]);
(**(code **)(*(int *)param_1[0x149] + 0x20))(1);   // release via FactoryClass vtable+0x20
param_1[0x149] = 0;
```

### Destructor
`BuildingClass::~BuildingClass` at `0x0043bcf0`:
```c
if ((int *)param_1->Factory != 0) {
    (**(code **)(*(int *)param_1->Factory + 0x20))(1);   // FactoryClass::vtable[0x20] destructor
}
param_1->Factory = 0;
```

### Lifecycle
- Set ONLY when a production order actively uses this building as the primary factory (see `HouseClass::GetPrimaryFactoryBuilding` call at line 85 of `FUN_004500f0`).
- NULL otherwise — including for factories that are idle.
- NULL for non-factory buildings (Repair Depot's "produce repair tickets" does NOT create a FactoryClass here — Repair Depot uses different mission code).
- Also referenced by `BuildingClass::UpdateGarrisonFire` at `0x0043e7b0` (reads `this->Factory` for prerequisite-side shape drawing).

### Is it always non-null for factory buildings?
**No.** It is ephemeral — allocated on production start, freed on production complete.
A War Factory with an empty queue has `Factory = NULL`.

### Is it ever set on non-factory buildings?
Not in any path checked. Set only via `FactoryClass__Constructor` called from
production-start code gated by `HouseClass::GetPrimaryFactoryBuilding`.

---

## Field 6 — `+0x520` — Claim: "BuildingTypeClass* Type pointer"

**Verdict: HIGH confidence.**

Confirmed in Ghidra struct (`Type` at byte 1312 = `0x520`) and cross-verified across
3 functions:

| Site | Disassembly |
|------|-------------|
| `GetPowerOutput` `0x0044e7b5` | `MOV EAX, dword ptr [ESI + 0x520]` → reads `Type`, then `[EAX + 0xee0]` (Type.PowerBonus base) |
| `GetPowerDrain` `0x0044e899` | `MOV ECX, dword ptr [ESI + 0x520]` → reads `Type`, then `[ECX + 0xee4]` (Type.PowerDrain) |
| `GoOffline` `0x00452371` | `MOV EAX, dword ptr [ESI + 0x520]` → reads `Type`, then `[EAX + 0xee4]` |
| `GoOnline` `0x004522c8` | `MOV EDX, dword ptr [ESI + 0x520]` → reads `Type`, then `[EDX + 0x16be]` |
| `Constructor` `0x0043b740` | `param_1[0x148] = param_2` (byte `0x148*4 = 0x520`) — direct write from constructor argument |

All consistent — Type is at **exactly `+0x520`** (4-byte pointer). Not `+0x521` or
adjacent. The struct-applied name in the decompiler (`this->Type`) matches every
raw-disassembly read.

---

## Field 7 — `+0x694` — Claim: "Occupant Count (garrison)"

**Verdict: HIGH confidence.**

### Evidence

**`GetOccupantCount` at `0x004581f0`** — one-liner:
```
MOV EAX, dword ptr [ECX + 0x694]
RET
```
Returns `+0x694` directly.

**`AddGarrisonOccupant` at `0x00522910`** — post-add increment:
```
MOV EBP, dword ptr [ESP + 0x20]       ; EBP = building (2nd arg)
MOV EAX, dword ptr [EBP + 0x68c]      ; Capacity
MOV ECX, dword ptr [EBP + 0x694]      ; Count
LEA EDI, [EBP + 0x684]                ; DynamicVector base
...
MOV dword ptr [EDI + 0x10], ECX       ; writes Count via EDI+0x10 = EBP+0x694
MOV [array + iVar1*4], param_1        ; occupants[old_count] = infantry
```
And in the decompilation: `param_2[0x1a5] = iVar1 + 1;` — `0x1a5 * 4 = 0x694`.

**`BuildingClass::PointerExpired` at `0x0044e8f0`** — decrements count on
occupant-pointer expiry:
```c
if (*(int *)&param_1->field_0x694 > 0 && array.contains(expired)) {
    iVar6 = *(int *)&param_1->field_0x694 - 1;
    *(int *)&param_1->field_0x694 = iVar6;
    // shift remaining elements left
}
```

All three functions treat `+0x694` as the DynamicVector's `Count` field.

### Conflict with CurrentFireIdx placement?
No — `+0x694` and `+0x69C` are 8 bytes apart. The DynamicVector layout occupies
`+0x684..+0x69B` (28 bytes = 6 4-byte + 4-byte flag block), leaving `+0x69C` as the
next aligned int. No overlap.

---

## Summary Table

| Field | Claim | Verdict | Actual |
|-------|-------|---------|--------|
| `+0x114` | Bio-reactor occupant count | HIGH (with clarification) | Embedded `CargoClass.NumPassengers` inherited from TechnoClass; re-purposed by bio-reactor via Type+0x16ae/0x16af flags |
| `+0x69C` | GarrisonFireIndex | HIGH | Confirmed — next int after the occupant DynamicVector at +0x684 |
| `+0x660` | HasPower | HIGH | Confirmed |
| `+0x661` | HasExtraPowerBonus | **WRONG** | Actually `IsOverpowered` (written by PowerCheck_Upgrade) |
| `+0x662` | HasExtraPowerDrain | **WRONG** | Unlabeled; constructor-cleared byte, no verified consumer |
| `+0x668` | — | — | Actual `HasExtraPowerBonus` (read by GetPowerOutput) |
| `+0x669` | — | — | Actual `HasExtraPowerDrain` (read by GetPowerDrain) |
| `+0x600` | Spotlight `BuildingLightClass*` | HIGH | Confirmed — conditionally allocated when `Type+0x154b != 0` |
| `+0x614` | Ambient `LightSourceClass*` | HIGH | Confirmed — conditionally allocated for lamp-like buildings (Type+0xe30..0xe40) |
| `+0x524` | `FactoryClass*` | HIGH | Confirmed — ephemeral, allocated on production start, freed on completion |
| `+0x520` | `BuildingTypeClass*` Type | HIGH | Confirmed across 5 functions |
| `+0x694` | Occupant Count | HIGH | Confirmed — DynamicVector.Count (garrison occupant list) |

---

## Recommendations

1. **Do not use `+0x661`/`+0x662` as "HasExtraPowerBonus/Drain"** in the Rust port.
   Use `+0x668` and `+0x669` respectively, matching Ghidra's applied struct and the
   raw disassembly of GetPowerOutput / GetPowerDrain.
2. **Document `+0x114` as `CargoClass::NumPassengers`** (not "bio-reactor count") to
   make clear it is the same field used for transport-helo passenger counts.
   Bio-reactor behaviour is achieved via the Type-side flags `+0x16ae` (BioReactor)
   and `+0x16af` (Absorber), not a dedicated count.
3. **Treat `+0x524` (`Factory*`) as ephemeral and nullable** — NOT a static
   "is-a-factory" marker. Presence of this pointer means "currently producing",
   not "can produce".
4. **`+0x600` and `+0x614` are both nullable** — only specific Type flags cause
   allocation. Most buildings have both NULL. Always null-check before dereferencing.
