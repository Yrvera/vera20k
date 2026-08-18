# EMP — TS-Legacy Dormant System

This doc is the canonical reference for the **EMP (electromagnetic pulse) system** in
gamemd.exe.

**Top-line verdict:** EMP is **functionally dormant in YR**.

- The only warhead with `EMEffect=yes` is `[EMPuls]`, explicitly marked
  `;gs disabled in code` in retail rulesmd.ini.
- The Rules `EMPulseWarhead=EMPuls` reference (intended for "warhead used by falling
  nuke missile") points to the disabled warhead.
- `EMPulseClass` C++ class exists in the binary with full functionality
  (constructor, destructor, Apply, recovery loop) but has no live caller in YR.
- `EMPulseSparkles` AnimType IS live, but it's used by the **RadSite visuals** (see
  [`radiation.md`](radiation.md) §9), not by an actual EMP event.

This doc exists primarily to **prevent re-implementation of dead EMP code** AND to
document the mechanism in case a future mod re-enables it. Similar in spirit to
[`ion_cannon.md`](ion_cannon.md).

Out-of-scope:
- Radiation system (shares the `ImmuneToRadiation` flag for building immunity, plus the `EMPulseSparkles` anim asset) → [`radiation.md`](radiation.md)
- Iron Curtain (different mechanism, but conceptually related — also disables units, but lives in different code) → separate; see `[IronCurtain]` SW
- Nuke superweapon → existing canonical [`../../NUKE_SUPERWEAPON_GHIDRA_REPORT.md`](../../NUKE_SUPERWEAPON_GHIDRA_REPORT.md)

---

## 1. Live INI survey — what's actually using EMP

### `EMEffect=yes` warheads in retail rulesmd.ini

```ini
; EM Pulse cannon warhead.
[EMPuls];gs disabled in code             ← only retail warhead with EMEffect
;Spread=11       ; Spread is radius of EM pulse effect.
EMEffect=yes
```

**ONE** warhead has `EMEffect=yes`. Its section header literally says "disabled in code".
The `;gs` prefix (Greg's signature? — Westwood comment convention) confirms a Westwood
engineer left this annotation indicating the EMP cannon was disabled before YR shipped.

### `EMPulseWarhead=` and `EMPulseProjectile=` Rules keys

```ini
; from rulesmd.ini line 587-588:
NukeProjectile=NukeUp   ; nuclear missile (from silo) projectile to launch
EMPulseWarhead=EMPuls   ; warhead used by falling nuke missile
EMPulseProjectile=PulsPr ; nuclear missile (from silo) projectile to launch
```

The INI comment claims `EMPuls` warhead is used by the falling nuke missile — but
since `[EMPuls]` itself is "disabled in code", the nuke does NOT actually deliver
EMP in YR. Verify against [`../../NUKE_SUPERWEAPON_GHIDRA_REPORT.md`](../../NUKE_SUPERWEAPON_GHIDRA_REPORT.md)
which traces the nuke detonation path.

### `EMPulseSparkles=` (LIVE — radiation visual)

```ini
; from rulesmd.ini line 567:
EMPulseSparkles=EMP_FX01	; Anim to play over units disabled by an EM Pulse.
```

This IS parsed and IS used — but **not for EMP**. The RadSite uses this anim for
units inside the radiation cloud (per [`radiation.md`](radiation.md) §9 and existing
canonical doc). The "disabled by EM Pulse" comment is the INI author's misleading
holdover.

---

## 2. Flag layout (verified)

### WarheadTypeClass

| Offset | INI key | String addr | Effect (if mechanism were live) |
|---|---|---|---|
| `wh+0x154` | `EMEffect=` | `0x00847D60` (verified live 2026-05-17) | Detonation triggers EMP on target |

Parsed at `WarheadTypeClass::ReadINI 0x0075D7B8`.

**Live consumer in YR:** None reachable, because no live warhead has `EMEffect=yes`.

### Rules `[SpecialWeapons]` constants

| Offset | INI key | Type | String addr |
|---|---|---|---|
| `Rules+0xFA0` | `EMPulseWarhead=` | string → WarheadType pointer | (per existing doc) |
| `Rules+0xFA4` | `EMPulseProjectile=` | string → BulletType pointer | (per existing doc) |

Both point to disabled-in-code entities.

### Rules `[General]` constants

| Offset | INI key | String addr | Status |
|---|---|---|---|
| `Rules+0x17F4` | `EMPulseSparkles=` | `0x0083CCA4` (verified live 2026-05-17) | **LIVE** (used by RadSite visuals — NOT by EMP) |

### TechnoTypeClass (immunity, shared with radiation)

| Offset | INI key | Effect |
|---|---|---|
| `type+0xD37` | `ImmuneToRadiation=` | If true, unit immune to radiation AND EMP (despite the name) — but irrelevant if EMP doesn't fire |
| `BuildingType+0x1701` | (mirror of ImmuneToRadiation for buildings) | Same — would block EMP if mechanism were active |

### TechnoClass instance

| Offset | Field |
|---|---|
| `+0x504` | `EMPLockRemaining` (int) — decremented per tick; non-zero = unit is EMP-locked |

`TechnoClass::IsUnderEMP @ 0x0070EFD0` returns `EMPLockRemaining > 0`. The field is
read by various consumers (movement gates, fire gates, etc.) — but the field is
never SET in YR because no path writes to it.

### Confidence

- **Content: HIGH** — three core string xrefs verified live 2026-05-17.
- **Identity: HIGH** — single INI key strings.
- **Binding: HIGH for the parser** (each flag has a known parse site). **Effectively N/A for runtime** — the consumer code paths exist but have no live entry point.

---

## 3. `EMPulseClass` struct (dead code, documented for reference)

**Size: 0x34 bytes**, allocated via constructor at `0x004C52B0`. Destructor at `0x004C5370`.

| Offset | Type | Field |
|---|---|---|
| `0x00-0x0F` | ptr × 4 | vtables |
| `0x10-0x23` | — | AbstractClass base |
| `0x24` | short | `CellX` |
| `0x26` | short | `CellY` |
| `0x28` | int | `Range` (cells) |
| `0x2C` | int | `StartFrame` |
| `0x30` | int | `Duration` (frames) |

Global container at `0x008A3870..0x008A3884`:
- Array vtable, data ptr, capacity, count, growth fields

**Status:** the class CAN be instantiated, but nothing in YR ever instantiates it.
The C++ code is dead code that survived compilation.

---

## 4. The (would-be-live-if-enabled) EMP application logic

For completeness — the dispatch logic that WOULD run if a `[EMPuls]`-style warhead
detonated. Documented here per [`../../RADIATION_EMP_GHIDRA_REPORT.md`](../../RADIATION_EMP_GHIDRA_REPORT.md) Part 2.

### `EMPulseClass::Apply @ 0x004C54E0` (DORMANT — no caller in YR)

Two loops:

**Loop 1 — iterate all Technos:**
```c
for each techno in g_TechnoClass_Array:
    if (techno.IsAlive && !techno.InLimbo && !techno.IsCrashing && techno.Health > 0):
        dist = CoordStruct::Distance3D(techno.coords, emp_center)
        if (dist < Range * 256):
            techno->vtable[0x3DC](duration)    // FootClass::ReceiveEMP
```

**Loop 2 — iterate cells in range (for buildings):**
```c
for y in (CellY-Range, CellY+Range):
    for x in (CellX-Range, CellX+Range):
        if (x² + y² <= Range²):
            cell = MapClass::Get_CellClass(x, y)
            building = LookUpBuildingInCell(cell)
            if (building):
                if (building is at foundation origin):
                    if (!building.Type.ImmuneToRadiation) (BuildingType+0x1701):
                        BuildingClass::ApplyOfflineEffects()  // 0x00452480
                        building.EMPLockRemaining = duration
                        if (building.Type has radar):
                            house.NeedsRadarRecalc = true
```

### `FootClass::ReceiveEMP @ 0x004DEBB0` (DORMANT)

```c
// For FootClass units (infantry / vehicles):
this.PlayEVA(0x26)                              // "EMP hit" voice
this.PlayEVA(0x29)                              // "unit disabled" voice
this.StopLocomotor(duration)                    // vtable+0xE0
this.SetMission(Guard, 3)                       // vtable+0x274 — stop orders
this.ClearOrders()                              // vtable+0x3A0
// Recursive EMP on passengers via FUN_00707CB0 @ 0x00707CB0
for each passenger in this.Passengers (+0x118):
    passenger.ReceiveEMP(duration)
// Cosmetic random rocking angles
```

### `BuildingClass::ApplyOfflineEffects @ 0x00452480` (DORMANT — only Apply caller)

- `StuffEnabled = false` (offset 0x6EA) — disables production / power contribution
- LightSource turned off (if building has one)
- Power-dependent anims removed
- Sensor capability deactivated (type+0xCD1)
- Wall connections recalculated (broken appearance for wall types)
- Radar building → house radar power state update

### EMP recovery in `TechnoClass::AI_Update`

```c
// At end of TechnoClass::AI_Update:
if (EMPLockRemaining > 0):
    EMPLockRemaining--
    if (EMPLockRemaining == 0):
        if (whatAmI == 6): // Building
            if (!type.ImmuneToRadiation):
                BuildingClass::RestoreOnlineEffects()  // 0x00452410
                if (type has radar): house.NeedsRadarRecalc = true
        else: // Foot
            this.Locomotor->Unlock()    // vtable+0x58
            // Clear EMPulseSparkles anim attached to this
```

This recovery loop runs every tick. If `EMPLockRemaining` is never set, the
conditional never enters. The code is fully dead in normal play.

---

## 5. Why the Tesla Coil doesn't disable units

A common misconception: Tesla Coils "lock up" their targets. In YR, this is **not**
EMP — the Tesla Coil's effect is:
1. Targeting animation (the visual "lock-on" before firing).
2. Standard damage on impact via `Electric` warhead.
3. No EMP, no movement lockout, no shutdown.

The `Electric` warhead doesn't have `EMEffect=yes` (verified by the survey above —
only `[EMPuls]` does, and that's disabled).

Tesla weapons do high damage. The illusion of "disabling" comes from the target's
death animation freezing it mid-pose, or from the targeting animation slowing the
visual fire cadence. There is no actual EMP lock applied to the target.

---

## 6. EMPulse Cannon superweapon

The existing canonical doc claims "EMPulse superweapon" is `SuperClass::Launch` case 3.
**This is incorrect for YR.** Per the verified enumeration in [`ion_cannon.md`](ion_cannon.md) §2:

| Case | Type |
|---|---|
| 0 | MultiMissile (Nuke) |
| 1 | IronCurtain |
| 2 | LightningStorm |
| **3** | **ChronoSphere** ← NOT EMP |
| 4 | ChronoWarp |
| ... | ... |
| 11 | PsychicReveal |

There is **no EMP superweapon case** in YR's SuperClass::Launch switch. The
EMPulseClass instantiation that would happen from such a case never occurs.

The existing canonical doc's claim of "case 3 in SuperClass::Launch" appears to be a
documentation error inherited from TS reference material (TS's case enumeration may
have differed).

---

## 7. TS-legacy filter

| Component | Status in YR |
|---|---|
| `EMEffect=yes` warhead flag (`wh+0x154`) | **PARSED-BUT-DORMANT** — only `[EMPuls]` sets it, and that warhead is "disabled in code" |
| `EMPulseClass` C++ class | **DEAD CODE** — no live caller, but bytes exist in binary |
| `EMPulseClass::Apply 0x004C54E0` | **DEAD** — single caller is EMPulseClass::Constructor, which has no live caller |
| `FootClass::ReceiveEMP 0x004DEBB0` | **DEAD** — single caller is Apply (dormant) |
| `BuildingClass::ApplyOfflineEffects 0x00452480` | **DEAD** — single caller is Apply (dormant) |
| `TechnoClass::EMPLockRemaining (+0x504)` | **NEVER SET** — read by various consumers but never written in YR |
| `IsUnderEMP 0x0070EFD0` predicate | **ALWAYS RETURNS FALSE** in YR (no path sets EMPLockRemaining) |
| `EMPulseWarhead` Rules ptr | **PARSED-BUT-DORMANT** — points to disabled warhead |
| `EMPulseProjectile` Rules ptr | **PARSED-BUT-DORMANT** |
| `EMPulseSparkles` Rules AnimType | **LIVE** — but used by RadSite, not EMP |
| `ImmuneToRadiation` building flag (`+0x1701`) | **LIVE** for radiation; would also gate EMP if EMP were live |
| EMP recovery in `TechnoClass::AI_Update` | **DEAD CONDITIONAL** — runs every tick but never enters the body since EMPLockRemaining stays 0 |

---

## 8. The lone "EMP-adjacent" effect that IS live

The closest thing to an EMP effect in retail YR is the **building offline state
during a Tesla Coil power outage**. When a player's power goes below the threshold
needed to run a Tesla Coil, the Tesla Coil enters an "offline" state visually similar
to an EMP-disabled state. But this is the standard power-management code path, NOT
the EMP system.

`StuffEnabled = false` (BuildingClass+0x6EA) is set both by `ApplyOfflineEffects`
(dormant EMP path) AND by power-failure logic (live). The same flag, two writers —
one dead, one live.

---

## 9. Practical implications

For implementers:

1. **Do not implement EMPulseClass as a sim object.** No live caller in YR.
2. **Do not implement an EMP warhead-special branch** in WarheadTypeClass::Detonate's mutually-exclusive cascade. No live warhead would trigger it.
3. **DO parse `EMEffect=` from INI** for compat (so mods can read their own warhead INIs back), but **do not implement the runtime mechanism**.
4. **DO parse `EMPulseWarhead=` and `EMPulseProjectile=`** for compat but treat as no-ops.
5. **DO implement `EMPulseSparkles` AnimType loading** — it's used by RadSite (per radiation.md).
6. **DO implement `EMPLockRemaining` field on TechnoClass** if other consumers in YR check `IsUnderEMP` (which they may, even if no path writes the field) — set it to 0 always.
7. **Power-failure offline state for buildings** IS live and needs its own implementation, separate from any EMP-disabled state.

---

## 10. Edge cases

| Case | Behavior |
|---|---|
| Mod un-comments `;` lines in `[EMPuls]` warhead to enable it | Warhead loads with `EMEffect=yes`. But the actual EMP dispatch code path (if any) needs investigation — the comment "disabled in code" suggests the dispatch is also stripped, not just the INI definition. **Mod investigation needed.** |
| Mod sets `EMEffect=yes` on a custom warhead | Warhead loads with the flag set. Whether the runtime mechanism fires depends on whether `wh+0x154` has a live read in YR's `WarheadTypeClass::Detonate`. Per the existing canonical doc, the read site exists but only via `EMPulseClass::Apply` which is dormant. **Mod compatibility risk.** |
| Mod uses Tesla weapons expecting EMP-like effect | Tesla weapons don't have `EMEffect=yes` and don't currently trigger EMP. Mod would need to ALSO add the flag to the Tesla warhead AND verify the dispatch fires. |
| `RadiationSite + ImmuneToRadiation` interaction | This works correctly (per radiation.md). The flag's "also gates EMP" property is irrelevant because EMP doesn't fire. |
| Nuke detonation | The INI suggests EMPulseWarhead=EMPuls is "used by falling nuke missile" but since [EMPuls] is disabled, the nuke effectively delivers no EMP component. Standard nuke damage and radiation apply. Verify via NUKE_SUPERWEAPON doc. |

---

## 11. Open follow-ups

1. **Confirm EMP dispatch in `WarheadTypeClass::Detonate` is dead in YR.** The existing canonical doc's `EMEffect=` read at `0x0075D7B8` is the PARSER. The Detonate-time read site — if any — needs separate trace. Priority: MEDIUM (the "disabled in code" comment hints the dispatch is also gone, but should be verified).
2. **Find what the SOLE consumer of `wh+0x154` is in live combat code.** If `WarheadTypeClass::Detonate` doesn't read it, what does? May be a leftover read in some pre-impact path that's safely gated by the absence of `EMEffect=yes` warheads. Priority: MEDIUM.
3. **Verify nuke does NOT deliver EMP in YR.** Trace `NukeGroundZero::ApplyDamage` (called from `Apply_area_damage` per its caller list in splash_cellspread.md) to confirm no EMP-side-effect call. Priority: LOW.
4. **Correct the existing canonical RADIATION_EMP_GHIDRA_REPORT.md.** Its claim "case 3 in `SuperClass::Launch`" for EMPulse is incorrect per ion_cannon.md's enumeration (case 3 is ChronoSphere). The doc should be updated to mark EMP system as dormant. Priority: LOW (documentation accuracy).
5. **Determine if `EMPulseSparkles` is also used in any OTHER live context.** RadSite is one consumer. Cross-check for other AnimType references. Priority: LOW.
6. **Power-failure offline state — is it truly separate from EMP?** Both write `StuffEnabled = false`. Cross-check the two code paths. Priority: LOW.

---

## 12. Sources

- Live xrefs (2026-05-17):
  - `"EMEffect"` at `0x00847D60`
  - `"EMPulse"` at `0x0081721C`
  - `"EMPulseSparkles"` at `0x0083CCA4`
- INI quotes from `ini/rulesmd.ini`:
  - line 567: `EMPulseSparkles=EMP_FX01` (LIVE)
  - lines 586-588: `EMPulseWarhead=EMPuls`, `EMPulseProjectile=PulsPr` (DORMANT — points to disabled warhead)
  - lines 26412-26415: `[EMPuls];gs disabled in code` warhead with `EMEffect=yes` — ONLY warhead with this flag in retail
- Existing canonical doc: [`../../RADIATION_EMP_GHIDRA_REPORT.md`](../../RADIATION_EMP_GHIDRA_REPORT.md) Part 2 (lines 233+) — primary source for EMPulseClass struct + dispatch logic. **Note: the doc's claim "case 3 in SuperClass::Launch" is incorrect for YR; see open follow-up #4.**
- Existing canonical doc: [`../../NUKE_SUPERWEAPON_GHIDRA_REPORT.md`](../../NUKE_SUPERWEAPON_GHIDRA_REPORT.md) — for cross-checking whether nuke actually delivers any EMP.
- Sister system docs: [`radiation.md`](radiation.md), [`ion_cannon.md`](ion_cannon.md) (sister "TS-legacy dormancy" doc), [`damage_formula.md`](damage_formula.md), [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md) (when written, will document the warhead cascade priority).
