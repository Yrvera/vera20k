# Ion Cannon — TS-Legacy Dormancy

This doc is the canonical reference for the **Ion Cannon system** in gamemd.exe.

**Top-line verdict:** the Ion Cannon C++ class (`IonBlastClass`) is **FULLY DORMANT
in YR** — no code references it, no instances are ever constructed, no detonation
branch is reachable. The only piece of "Ion" infrastructure that remains live is the
`IonBlast=` INI AnimType key, repurposed by the **Genetic Mutator superweapon** as
its overhead ring animation.

This doc exists primarily to **prevent re-implementation of dead code** when scanning
for "Ion" references. There is no Ion Cannon damage formula to document because
there is no live execution path.

Out-of-scope:
- Lightning Storm (which uses `IonWH` warhead, separate from Ion Cannon) → [`warheads/IonWH.md`](../warheads/IonWH.md) when written
- Genetic Mutator superweapon (which uses `Rules.IonBlast` AnimType) → [`../../PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md`](../../PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md)-style — separate
- Tiberian Sun's actual Ion Cannon — out of scope for this YR-focused project

---

## 1. What's in the binary (verified)

### IonBlastClass C++ class — DORMANT

| Evidence | Result |
|---|---|
| `search_functions "IonBlast"` | **0 matches** — no constructor, no destructor, no methods |
| RTTI strings | `.?AV?$VectorClass@PAVIonBlastClass@@@@` at `0x008280D8`, `.?AV?$DynamicVectorClass@PAVIonBlastClass@@@@` at `0x00828108` |
| Xrefs to RTTI strings | `get_xrefs_to 0x008280D8`: **0 refs**. `get_xrefs_to 0x00828108`: **0 refs** |
| Global array (count/capacity/buffer triple) | **Not present.** Compare to `g_DiskLaserClass_Array @ 0x008A020C` which IS live |
| Code that constructs IonBlastClass | **None** |
| Code that references IonBlastClass | **None** |

**Verdict: VERY HIGH confidence the IonBlastClass C++ class is fully dead.** Only
compile-time RTTI for its containers remains, and even those are unreferenced.

### Confidence

- **Content: VERY HIGH** — multiple negative searches verified (existing canonical doc Round 2 reverified all checks).
- **Identity: VERY HIGH** — RTTI strings name the class explicitly; their unreferenced state is a strong signal of complete dormancy.
- **Binding: N/A (negative result)** — the absence of bindings IS the verified finding.

---

## 2. Ion Cannon superweapon — REGISTERED BUT NOT ENABLED

From `ini/rulesmd.ini` `[SuperWeaponTypes]` enumeration (lines ~2848+):

```ini
[SuperWeaponTypes]
;4=IonCannonSpecial            ← COMMENTED OUT
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

`[IonCannonSpecial]` itself IS defined further down (`ini/rulesmd.ini:30847`):

```ini
[IonCannonSpecial]
ImpatientVoice=
SuspendVoice=
RechargeTime=8.5
Type=IonCannon
Action=IonCannon
SidebarImage=IONCICON
ShowTimer=no
```

But because the `;4=IonCannonSpecial` line is commented out, the section is never
loaded into the active SuperWeaponTypes index. No player gets an Ion Cannon SW.

### Even if re-enabled by a mod

`SuperClass::Launch` at `0x006CC390` switches on the SW Type enum (`SuperWeaponTypeClass+0xB4`). Verified case enumeration:

| Case | Type | Effect |
|---|---|---|
| 0 | MultiMissile (Nuke) | Bullet + warhead flow |
| 1 | IronCurtain | `Rules+0x348` |
| 2 | LightningStorm | `LightningStorm::Start` |
| 3 | ChronoSphere | — |
| 4 | ChronoWarp | `Rules+0x32C/+0x328` |
| 5 | ParaDrop | — |
| 6 | AmericanParaDrop | — |
| 7 | PsychicDominator | `FUN_0053AE50` |
| 8 | SpyPlane | — |
| 9 | **GeneticConverter** | **AnimClass(Rules+0x298 = IonBlast), then iterate 3x3 for MutateExplosion damage** |
| 10 | ForceShield | `HouseClass::SpyPowerSabotage` |
| 11 | PsychicReveal | `MapClass::RevealAroundCell` |
| — | (no case 12 / no default) | — |

**There is NO `case IonCannon` in the switch.** Even if a mod re-enabled the SW
registration, the launch handler would fall off the end of the switch and do nothing
(or hit an empty default). Full reactivation would require an executable patch.

---

## 3. IsIonCannon weapon flag — NOT PARSED

`search_strings "IsIonCannon"` returns **0 matches** in gamemd.exe.

The `WeaponTypeClass::ReadINI` parser does **not** look up an `IsIonCannon=` key. So:
- No weapon can ever set `weapon+0x144 (IsIonCannon)` via INI.
- The matching dispatch branch in `TechnoClass::Fire_At` at `0x6FDDD4` (which would
  fire an IonCannonWarhead via `Rules+0xFA8`) is **unreachable dead code**.

---

## 4. Rules-class fields that ARE parsed (but dispatch is dormant)

These keys ARE parsed from `[General]` / `[CombatDamage]` because the shared parser
still handles them — they just have no live consumer:

| INI key | Section | Rules offset | Status |
|---|---|---|---|
| `IonBlast=` | `[General]` | `+0x298` | **LIVE** — Genetic Mutator overhead ring anim (Case 9 in SuperClass::Launch) |
| `IonBeam=` | `[General]` | `+0x29C` | **DORMANT** — parsed but never read after setup |
| `IonCannonWarhead=` | `[General]` | `+0xFA8` | **DORMANT** — stored but never dispatched (no weapon can set IsIonCannon to trigger the path) |
| `IonCannonDamage=` | `[General]` | (other offset) | **DORMANT** — never applied |
| `AIIonCannonConYardValue=` | `[AI]` etc. | various | **DORMANT** — AI threat-score tables for Ion Cannon target priority. Since no AI ever has Ion Cannon, these are never queried. |
| `AIIonCannonWarFactoryValue=` etc. (11 keys total) | `[AI]` | various | **DORMANT** — same |

### Confidence

- **Content: HIGH** — each parse and read site verified in existing canonical doc.
- **Identity: HIGH** — INI key strings have known parser xrefs.
- **Binding: HIGH** — `Rules+0x298` has ONE live consumer (Genetic Mutator); everything else has zero.

---

## 5. The one live consumer — Genetic Mutator (Case 9)

From `SuperClass::Launch` Case 9 (Genetic Converter) at approximately `0x006CD89E`:

```c
case 9:  // GeneticConverter
    if (param_1->byte+0x6F != 0):
        cell = MapClass::Get_CellClass(...)
        coord = cell->Get_Coords()
        // bridge height adjustment

        // Spawn the overhead ring anim
        new AnimClass(Rules+0x298 /* IonBlast */, coord, owner=NULL, ...)

        VoxClass::PlayEVA(...)
        VocClass::PlayAtCoord(...)
        CreateRadarEvent(...)

        // Then iterate 3x3 cells, killing infantry with GeneticMutatorWH (Rules+0xF98)
        for cell in 3x3 around impact:
            for inf in cell.infantry_occupants:
                inf->ReceiveDamage(huge, 0, Rules.GeneticMutatorWH, ...)
```

The anim spawned is a **standard `AnimClass` instance (0x1C8 bytes)** — NOT an
`IonBlastClass` object. The `Rules.IonBlast` field is just used as the AnimType
pointer, treated identically to any other anim spawn.

Per existing canonical doc:
> Nothing else in the binary references `Rules+0x298`. The only `mov eax,
> [g_RulesClass_Instance+0x298]` in live code is the Genetic Mutator call at
> `0x006CD89E`; stray matches are sub-field accesses on unrelated structs.

---

## 6. Other "Ion"-named features that are NOT Ion Cannon

Easy to confuse — these are separate systems:

| Feature | Status | Notes |
|---|---|---|
| `IonStorm` (string at `0x0081D994`) | TS-legacy (related vtable at `0x007E5278`) | The TS Ion Storm event — unrelated to Ion Cannon |
| `IonLevel`, `IonGround`, `IonBlue`, `IonGreen`, `IonRed`, `IonAmbient` | Light-tint params for IonStorm renderer | Unrelated to Ion Cannon |
| `IonStorms` (string at `0x008401F8`) | Scenario/Rules multi-bool flag | Unrelated |
| `IonImmune` (TypeClass flag) | LIVE — type-level immunity to ion storm | Unrelated |
| `IonSensitive` (WeaponTypeClass flag) | LIVE — weapon-level ion-storm sensitivity | Unrelated |
| `[LightningStorm]` (warhead `LightningWarhead=IonWH`) | LIVE — Lightning Storm SW uses `[IonWH]` warhead | Lightning is YR; Ion Cannon is TS |
| `[IonWH]` warhead | LIVE — referenced by `LightningWarhead=` | Repurposed TS asset for Lightning Storm |
| `[IonCannonWH]` warhead | DEFINED in `[Warheads]` enum (line 2890) | But no live weapon references it |

---

## 7. Practical implications

For implementers / parity testers:

1. **Do not implement IonBlastClass as a C++ object.** There is no live sim object. The TS-era class was stripped of all code in YR — only RTTI ghosts remain.
2. **Do not implement an IonCannon superweapon.** It's not reachable in vanilla YR.
3. **Do not implement an IsIonCannon weapon flag.** The string is not in the binary; no weapon parser can read it.
4. **Do not implement an IonCannonWH dispatch path.** Even though the warhead is parsed and referenced by Rules+0xFA8, no execution path uses it.
5. **DO implement the `IonBlast=` AnimType reference at `Rules+0x298`** — used by Genetic Mutator. Treat it as a standard AnimType pointer.
6. **DO implement `IonWH` warhead** — used by Lightning Storm. (Documented separately when that warhead doc is written.)

---

## 8. TS-legacy filter — explicit declarations

| Item | Status |
|---|---|
| `IonBlastClass` C++ class | **TS-legacy, FULLY DORMANT in YR** |
| `IonCannonSpecial` superweapon | **TS-legacy, registered-but-disabled in YR** |
| `IsIonCannon` weapon flag | **TS-legacy, NOT PARSED in YR** |
| `IonCannonWarhead`/`IonCannonDamage` Rules fields | **TS-legacy, PARSED-BUT-DORMANT in YR** |
| `IonBeam` Rules AnimType | **TS-legacy, PARSED-BUT-DORMANT in YR** |
| `AIIonCannon*Value` AI tables (11 keys) | **TS-legacy, PARSED-BUT-DORMANT in YR** |
| `IonBlast` Rules AnimType | **LIVE in YR** — repurposed by Genetic Mutator |
| `IonWH` warhead | **LIVE in YR** — used by Lightning Storm |
| `IonStorm` rendering / lighting infrastructure | Separate system, partially live for atmospheric storms |

---

## 9. Edge cases

| Case | Behavior |
|---|---|
| Mod uncomments `4=IonCannonSpecial` in `[SuperWeaponTypes]` | SW is registered, sidebar icon appears, RechargeTime applies, but **the launch handler does nothing** (no case in SuperClass::Launch switch). The button is decorative; nothing fires. |
| Mod sets `IsIonCannon=yes` on a weapon | INI key is **not parsed**; no field is set. The flag has no effect. |
| Mod sets `Warhead=IonCannonWH` on a weapon | Warhead reference resolves normally. Damage application uses the warhead's normal Verses/CellSpread/AnimList — the `IsIonCannon` dispatch is never triggered. Behaves as a normal damage warhead. |
| Genetic Mutator launch | Anim of type `Rules.IonBlast` is spawned at target cell. This is the ONE live consumer. |
| Lightning Storm launch | Uses `IonWH` warhead. Damage path is standard (not Ion Cannon). |

---

## 10. Open follow-ups

1. **Verify `Rules+0xFA8 IonCannonWarhead` is truly dormant.** The existing canonical doc says the only code path that would dispatch is via `weapon+0x144 IsIonCannon`, which can't be set because the key isn't parsed. Cross-check by searching for ALL reads of `[Rules+0xFA8]` in the binary. Priority: LOW (overwhelming evidence already).
2. **`AIIonCannon*Value` tables (11 keys) — confirm AI never reads them.** The AI threat scoring functions should not query Ion-related tables. Verify by inspecting AI target-priority code paths. Priority: LOW.
3. **`[IonCannonWH]` warhead INI section — what's in it?** Worth quoting verbatim once when the warhead doc is written (in `warheads/IonCannonWH.md`), even though no weapon references it. Priority: LOW.
4. **`IonBeam=` — what does it look like in rulesmd.ini?** Quote the line for completeness. Priority: LOW.

---

## 11. Sources

- Existing canonical doc: [`../../ION_BLAST_CLASS_GHIDRA_REPORT.md`](../../ION_BLAST_CLASS_GHIDRA_REPORT.md) (302 lines) — exhaustive dormancy verification, including Round 2 RTTI xref re-checks. Confidence: VERY HIGH. This systems doc summarizes its findings.
- Live xrefs (2026-05-17):
  - `"IonCannon"` standalone — **0 matches in binary** (verified)
  - `"IonBlast"` at `0x0083CED0` — exists, parsed into Rules
  - INI grep of `ini/rulesmd.ini`: `IonCannonSpecial` SW definition present at line 30847 but commented out in `[SuperWeaponTypes]` enumeration at line 2852
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md), [`superweapon_dispatch.md`](superweapon_dispatch.md) (when written).
- Per-warhead docs to be written:
  - [`warheads/IonCannonWH.md`](../warheads/IonCannonWH.md) — defined but dead; will document the section verbatim
  - [`warheads/IonWH.md`](../warheads/IonWH.md) — live, used by Lightning Storm
