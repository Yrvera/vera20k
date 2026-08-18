# IonBlastClass — Ghidra Research Report

**Top-line verdict: DORMANT (TS-legacy C++ class) + the `IonBlast` INI AnimType IS live in YR via Genetic Mutator.**

- `IonBlastClass` (the C++ class) is **completely dormant** in YR: no constructor function is emitted, no instances are ever created, its array globals are never written to, and the only RTTI evidence is stray compile-time type descriptors for `VectorClass<IonBlastClass*>` / `DynamicVectorClass<IonBlastClass*>` that are never referenced by any code. It is pure Tiberian Sun dead code inherited by gamemd.exe.
- The `IonBlast=` INI key in `[General]` IS still parsed into `RulesClass+0x298` and IS used as the overhead ring AnimType spawned by the **Genetic Mutator** superweapon (SW Type enum 9 = GeneticConverter) — not by any Ion Cannon (Ion Cannon SW is itself disabled in YR's `[SuperWeaponTypes]` list).

Confidence: HIGH for the dormancy of the C++ class. HIGH for `IonBlast` AnimType usage by Genetic Mutator (verified via `SuperClass::Launch` case 9).

---

## Evidence: IonBlastClass C++ class is dead in YR

### 1. No constructor, no code

- `search_functions "IonBlast"` → **no functions matched**.
- `search_functions "Ion"` in the binary — no result that creates or manipulates an IonBlastClass object.
- The only strings tied to the class in the binary are compile-time RTTI for the containers:
  - `0x008280D8` : `.?AV?$VectorClass@PAVIonBlastClass@@@@`
  - `0x00828108` : `.?AV?$DynamicVectorClass@PAVIonBlastClass@@@@`
- Both of those RTTI strings have **zero cross-references** (`get_xrefs_to 008280d8` → no refs, `get_xrefs_to 008280108` → no refs). Nothing in the image instantiates a `VectorClass<IonBlastClass*>` either — so not even the container for this class is ever constructed in YR.
- No global array (count/capacity/buffer triple) for IonBlastClass exists in gamemd.exe. Compare to DiskLaser, which has `g_DiskLaserClass_Array` (`0x008A020C`) actively used — IonBlast has no equivalent.

### 2. Ion Cannon SW is itself disabled in YR

`ini/rulesmd.ini` `[SuperWeaponTypes]`:

```ini
; active list — note IonCannonSpecial is commented out
;4=IonCannonSpecial
...
1=NukeSpecial
2=IronCurtainSpecial
3=LightningStormSpecial
4=ChronoSphereSpecial
5=ChronoWarpSpecial
6=ParaDropSpecial
7=AmericanParaDropSpecial
8=PsychicDominatorSpecial
9=SpyPlaneSpecial
10=GeneticConverterSpecial
11=ForceShieldSpecial
12=PsychicRevealSpecial
```

The `[IonCannonSpecial]` section still exists further down (30847..30859) but is NOT registered in `[SuperWeaponTypes]`, so it never gets constructed and its `Type=IonCannon` is unreachable in any normal YR game.

Even if `[IonCannonSpecial]` were re-enabled (mod scenario), `SuperClass::Launch` does not contain a case handler for an `IonCannon` type in YR builds — the switch at `*(SuperWeaponTypeClass+0xB4)` only dispatches on enum values 0..11 (Nuke..PsychicReveal) and NONE of those code paths construct an `IonBlastClass` object. The C++ class is truly dead.

### 3. `RulesClass+0xFA8` (`IonCannonWarhead`) still exists as a field, but its `IonCannonClass` detonation path never runs

The field at `Rules+0xFA8` is the AnimType-pointer that `TechnoClass::Fire_At` uses via `this->vtable+0x16C` at `0x6FDDD4` when `weapon+0x144` is set. That code path would trigger with a weapon having `IsIonCannon=yes`. HOWEVER:
- `search_strings "IsIonCannon"` → **no matches** in the binary. `IsIonCannon` is NOT parsed from INI by gamemd.exe. So no weapon can ever set `weapon+0x144`.
- The nearest matching strings (`IonCannonWarhead`, `IonCannonDamage`, and the `AIIonCannon...ValueX` score tables) are parsed in `RulesClass::ReadGeneral` but none of them trigger any IonBlastClass construction.

Net: the only Ion-cannon-visual code path that actually runs in YR is the Genetic Mutator launch handler reusing `Rules+0x298` (`IonBlast` AnimType).

---

## IonBlast AnimType (LIVE usage)

### 4. Rules+0x298 = IonBlast (AnimType*)

Parsed in `RulesClass::ReadGeneral` (function `0x0066DCB0`):

```c
uVar4 = *(undefined4 *)(param_1 + 0x298);                    // default = existing ptr
iVar2 = CCINIClass__ReadString(&s_General, &s_IonBlast_0083ced0, ...);
if (iVar2 != 0) {
    uVar4 = AnimTypeClass__FindByName();
}
*(undefined4 *)(param_1 + 0x298) = uVar4;
```

INI:
```ini
[General]
IonBlast=RING1          ; initial anim when ion cannon hits (ini/rulesmd.ini:530)
```

(The comment in `rulesmd.ini` is a leftover from TS — in YR the anim is used by Genetic Mutator, not the Ion Cannon.)

### 5. Single live consumer in YR: Genetic Mutator

Verified via `SuperClass::Launch` at `0x006CC390`, switching on `*(SuperWeaponTypeClass+0xB4)` (Type enum):

| Case | Enum | INI Section | IonBlast used? |
|------|------|-------------|----------------|
| 0 | MultiMissile | NukeSpecial | no (uses NukeDown/NukeUp) |
| 1 | IronCurtain | IronCurtainSpecial | no |
| 2 | LightningStorm | LightningStormSpecial | no |
| 3 | ChronoSphere | ChronoSphereSpecial | no (uses ChronoBlast/other) |
| 4 | ChronoWarp | ChronoWarpSpecial | no |
| 5 | ParaDrop | ParaDropSpecial | no |
| 6 | AmerParaDrop | AmericanParaDropSpecial | no |
| 7 | PsychicDominator | PsychicDominatorSpecial | no |
| 8 | SpyPlane | SpyPlaneSpecial | no |
| **9** | **GeneticConverter** | **GeneticConverterSpecial** | **YES — creates AnimClass(Rules+0x298) at target** |
| 10 | ForceShield | ForceShieldSpecial | no |
| 11 | PsychicReveal | PsychicRevealSpecial | no |

Relevant slice of `SuperClass::Launch` case 9 (addr ≈ `0x006CD89E`):

```c
case 9:  // GeneticConverter
    if (*(char *)((int)param_1 + 0x6f) != '\0') {
        piVar20 = (int *)MapClass__Get_CellClass();
        piVar20 = (int *)(**(code **)(*piVar20 + 0x48))();   // CellClass::Get_Coords
        iVar21 = *piVar20; iVar16 = piVar20[1]; local_1cc = piVar20[2];
        // bridge height adjustment ...
        pvVar14 = operator_new(0x1c8);                        // AnimClass = 0x1C8 bytes
        if (pvVar14 != (void *)0x0) {
            ...
            AnimClass__Constructor(*(undefined4 *)(g_RulesClass_Instance + 0x298)); // <-- IonBlast
        }
        VoxClass__PlayEVA();
        VocClass__PlayAtCoord();
        CreateRadarEvent();
        // then iterate 3x3 cells killing infantry with GeneticMutatorWH (Rules+0xF98) ...
```

Notes:
- The anim is spawned as a standard `AnimClass` instance (0x1C8 bytes), owner = none, NOT an `IonBlastClass` object.
- The anim drops onto the normal AnimClass update pipeline (`g_AnimClass_Array`, `AnimClass::AI`, etc.).
- Nothing else in the binary references `Rules+0x298` (verified: the only byte pattern `mov eax, [g_RulesClass_Instance+0x298]` in live code is the Genetic Mutator call at `0x006CD89E`; stray matches are sub-field accesses on unrelated structs).

### 6. Rules+0x29C = IonBeam (AnimType*) — parsed but UNUSED

Parsed in the same `RulesClass::ReadGeneral` block immediately after IonBlast:

```c
*(undefined4 *)(param_1 + 0x29c) = uVar4;  // IonBeam
iVar2 = CCINIClass__ReadString(&s_General, s_IonBeam_0083cec8, ...);
```

There is NO call in gamemd.exe that reads `Rules+0x29C` after setup (no xref from code). This is likely the TS ion-cannon vertical-beam anim, kept only because the shared `[General]` parser still reads the key. Report as **DORMANT** (TS leftover).

---

## Call graph for the `IonBlast` AnimType

```
ini/rules(md).ini [General] IonBlast=RING1
        │
        ▼  RulesClass::ReadGeneral @ 0x0066DCB0  (stores AnimTypeClass* at Rules+0x298)
        │
        ▼  SuperClass::Launch case 9 (GeneticConverter) @ ≈ 0x006CD89E
        │    operator_new(0x1C8)
        │    AnimClass::Constructor(Rules+0x298 /* IonBlast */, target_coord, owner=NULL, ...)
        │
        ▼  AnimClass joins g_AnimClass_Array
        │
        ▼  AnimClass::AI (0x423AC0) → normal animation update+draw
```

Per-caller YR-reachability verdict:

| Caller | YR reachable? | Evidence |
|---|---|---|
| `RulesClass::ReadGeneral` (stores) | YES — always at startup | every skirmish reads `[General]` |
| `SuperClass::Launch` case 9 (GeneticConverter) | YES | `GeneticConverterSpecial` is slot 10 in `[SuperWeaponTypes]` and the Yuri faction gets Genetic Mutator |
| `SuperClass::Launch` cases 0..8, 10, 11 | n/a | none reference Rules+0x298 |
| Anything using `IonBlastClass` C++ class | **NO** | no code constructs or manipulates it anywhere |

---

## INI keys

| Key | Section | RulesClass offset | Type | Status |
|---|---|---|---|---|
| `IonBlast` | `[General]` | 0x298 | AnimType name | LIVE — Genetic Mutator overhead ring |
| `IonBeam` | `[General]` | 0x29C | AnimType name | DORMANT — TS vertical beam, never read after setup |
| `IonCannonDamage` | `[General]` | (other offset) | integer | DORMANT — no weapon reads it (IsIonCannon is not a parsed flag) |
| `IonCannonWarhead` | `[General]` | 0xFA8 | WarheadType name | DORMANT — stored but never dispatched (weapon+0x144 IsIonCannon is never set because the flag is not parsed) |

The default in both `rules.ini` and `rulesmd.ini` is `IonBlast=RING1`. In standard YR the visible Genetic Mutator ring uses the `RING1` animation (see `[RING1]` section in `artmd.ini`).

---

## Open questions

1. Does any map-trigger action (scripted mission event) spawn an IonBlastClass C++ object? Checked: no. `TriggerAction` handlers in the binary never reference the RTTI strings at `0x008280D8` / `0x00828108`, and no mission-action case in the big `TriggerClass::Execute` switch creates one. The C++ class is fully removed from all live execution paths in YR.
2. Is there any way a mod re-enables the ion cannon detonation? Only by re-enabling `[IonCannonSpecial]` in `[SuperWeaponTypes]` — BUT there is no `case IonCannon` in `SuperClass::Launch` (the Type enum stops at 11 = PsychicReveal), so even a re-enabled Ion Cannon SW would hit an empty default branch and not fire visuals. Full reactivation is outside the scope of YR and would require an executable patch.

---

## Ghidra functions labeled (for this report)

None — no IonBlastClass code exists. No functions were renamed for this investigation.

Verified dormant. Nothing to label.

---

## Follow-up investigation (round 2)

Exhaustive dormancy verification per Round-2 request. All evidence below re-verified
via Ghidra MCP in this session.

### Q3 — RESOLVED: IonBlastClass C++ class is FULLY DORMANT. Confidence raised from HIGH to VERY HIGH.

Every potential live-path check comes back negative:

**1. RTTI / container references — zero in both cases.**
- `get_xrefs_to 0x008280d8` (`.?AV?$VectorClass@PAVIonBlastClass@@@@`) → **"No references found"**.
- `get_xrefs_to 0x00828108` (`.?AV?$DynamicVectorClass@PAVIonBlastClass@@@@`) → **"No references found"**.
- No code anywhere constructs, iterates, or references an `IonBlastClass` object or its containers.

**2. Strings exhaustively scanned.**
All `Ion*`-style substrings in the binary:

| Address | String | Use site |
|---|---|---|
| `0x008280d8` | `.?AV?$VectorClass@PAVIonBlastClass@@@@` | **unused RTTI** (0 xrefs) |
| `0x00828108` | `.?AV?$DynamicVectorClass@PAVIonBlastClass@@@@` | **unused RTTI** (0 xrefs) |
| `0x0083ced0` | `IonBlast` | `RulesClass::ReadGeneral` → stores at Rules+0x298 (Genetic Mutator anim) |
| `0x0083cec8` | `IonBeam` | `RulesClass::ReadGeneral` → stores at Rules+0x29C (never read after) |
| `0x0083aecc` | `IonCannonWarhead` | `FUN_0066bbb0` (Rules sub-reader) → stored at Rules+0xFA8, dispatched only when weapon's `IsIonCannon` flag is set — see below |
| `0x0083b284` | `IonCannonDamage` | `FUN_0066bbb0` → stored but never applied |
| `0x0083bf40..0x0083c06c` | `AIIonCannonXxxValue` (11 keys) | Rules score tables for AI target scoring; no code path constructs IonBlastClass |
| `0x0081d994` | `IonStorm` | Data xref from `0x007e5278` (likely IonStormClass vtable, separate TS class, not IonBlastClass) |
| `0x0082bf28..0x0082bf5c` | `IonLevel`, `IonGround`, `IonBlue`, `IonGreen`, `IonRed`, `IonAmbient` | Light-tint parameters for an IonStorm-related renderer — unrelated to IonBlastClass |
| `0x008401f8` | `IonStorms` | Parsed by `FUN_006b8b30` / `FUN_006b8ca0` (likely Scenario/Rules multi-bool) — unrelated |
| `0x00842ffc` | `IonImmune` | `FUN_006f1550` — TechnoType ion-storm immunity flag, unrelated |
| `0x00849300` | `IonSensitive` | `WeaponTypeClass::ReadINI` at `0x00772080` — weapon ion-storm sensitivity flag, unrelated |

- No `Ion_Cannon`, `IsIonCannon`, `IonCannonClass`, or `IonBlastClass` strings exist.
  Search: `search_strings "Ion_Cannon"` → 0 matches. `search_strings "IsIonCannon"` → 0 matches.

**3. `SuperClass::Launch @ 0x006cc390` exhaustive switch enumeration.**
Re-decompiled in full this session. The switch dispatches on
`*(undefined4 *)(param_1[10] + 0xb4)` (SuperWeaponTypeClass `Type` enum) with
cases 0–0xB (11) handling all YR super weapons:

| Case | Effect | IonBlast touched? |
|---|---|---|
| 0 | Nuke | no (BulletClass + Warhead flow) |
| 1 | Iron Curtain | no (uses `Rules+0x348`) |
| 2 | Lightning Storm | no (`LightningStorm::Start`) |
| 3 | Chrono Sphere | no |
| 4 | Chrono Warp | no (uses `Rules+0x32c`, `Rules+0x328`) |
| 5 | Para Drop | no |
| 6 | American Para Drop | no |
| 7 | Psychic Dominator | no (`FUN_0053ae50`) |
| 8 | Spy Plane | no |
| **9** | **Genetic Converter** | **YES — `AnimClass::Constructor(Rules+0x298)`** |
| 10 | Force Shield | no (`HouseClass::SpyPowerSabotage`) |
| 0xB | Psychic Reveal | no (`MapClass::RevealAroundCell`) |

- No `case 0xC`, no `default:`, no fall-through to an IonCannon branch. The
  switch simply ends after case 0xB.
- Case 9 is the SOLE consumer of `Rules+0x298` (IonBlast AnimType ref). It instantiates
  a standard `AnimClass` (0x1C8 bytes) — NOT an `IonBlastClass`. Confirms Round-1
  finding.

**4. `IsIonCannon` on WeaponTypeClass: unreachable.**
- `IsIonCannon` is not a parsed INI key (0 string matches).
- `WeaponTypeClass::ReadINI` at `0x00772080` parses `IonSensitive` but NOT `IsIonCannon`.
- Therefore `weapon+0x144` (the `IsIonCannon` flag consumed by `TechnoClass::Fire_At` at
  `0x6FDDD4`) is never set, and the `IonCannonWarhead` detonation branch via
  `Rules+0xFA8` is dead code in YR.

**5. `SpecialFlags`-gated paths — none construct IonBlastClass.**
- Searched for any byte-pattern gate on IonBlast and re-scanned all consumers of the
  `Rules+0x298` slot. The only code that reads this offset is Case 9 of
  `SuperClass::Launch` (Genetic Mutator). There is no SpecialFlags condition gating
  IonBlastClass anywhere.

**6. Trigger / mission scripts — none reach IonBlastClass.**
- The `TriggerAction::Execute` family in gamemd.exe has no case that constructs an
  IonBlastClass (no code anywhere refs the RTTI strings above).

### Definitive conclusion

There is **NO path, behind any flag or gate, that constructs an `IonBlastClass`
instance in the YR gamemd.exe binary.** The class is pure Tiberian Sun dead code
that survived compilation only as:
- Two unreferenced RTTI descriptor strings (VectorClass / DynamicVectorClass of IonBlastClass*).
- No array globals (the count/capacity/buffer triple that every live gamemd.exe
  DynamicVectorClass would have is absent — compare to DiskLaser at `0x008A020C`
  which IS present and actively populated).

**Confidence:** VERY HIGH. Fully dormant, bit-for-bit verified.

### Labels applied

None. Nothing to label — the class has no live functions in the binary.

Ghidra `save_program` called at end of this session.

---

## Implementation guidance (for the Rust engine)

- **Do NOT implement IonBlastClass as a C++ object.** There is no sim object to maintain; IonBlast is just an AnimType reference.
- When the Genetic Mutator SW launches, the engine should:
  1. Look up `RulesClass.IonBlast` (AnimTypeClass pointer) — Rust equivalent: `rules.general.ion_blast` → AnimTypeId.
  2. Spawn a standard `Anim` entity at the target cell (same as any other AnimClass instance), owner = None.
  3. Apply `MutateExplosion` logic (Rules+0x17C8) using the standard Warhead/damage path.
- Ion Cannon SW itself: **do not implement**. It is not reachable in YR, and the implementation is not needed for parity with retail YR skirmish.
- IonBeam (Rules+0x29C): do not implement. It parses the key (for compat) but never draws.
- IsIonCannon weapon flag / IonCannonWarhead dispatch (weapon+0x144 / Rules+0xFA8): **do not implement**. The source flag is never parsed into `WeaponTypeClass` in YR — no weapon can set it, so the matching `TechnoClass::Fire_At` branch at `0x6FDDD4` is unreachable.
