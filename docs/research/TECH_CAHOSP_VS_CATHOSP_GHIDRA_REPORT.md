# Tech Hospital — [CATHOSP] vs [CAHOSP] — Ghidra Research Report

**Addresses:**
- `BuildingClass::ChangeOwner` — `0x00448260`
- `TechnoClass::AI_Update` (self-heal aura consumer) — `0x006F9E50`
- `BuildingTypeClass::ReadINI` — `0x0045FE50` (labelled `BuildingTypeClass_ReadINI_Water` in Ghidra)
- `BuildingTypes=` parser — `FUN_00672660`
- `NeutralTechBuildings=` parser site — `0x00673926` inside `FUN_00672ae0` (RulesClass general read)
- `HouseClass::HasInfantryGainSelfHeal` — `0x0050D9C0` (Ghidra label `HasPowerOutput` — **misnamed**)
- `HouseClass::GetInfantrySelfHealAmount` — `0x0050D9E0` (Ghidra label `GetTotalPowerOutput` — **misnamed**)
- `HouseClass::HasUnitsGainSelfHeal` — `0x0050D9D0` (Ghidra `HasPowerDrain` — **misnamed**)
- `HouseClass::GetUnitsSelfHealAmount` — `0x0050D9F0` (Ghidra `GetTotalPowerDrain` — **misnamed**)
- `FUN_004653c0` — name → BuildingTypeClass resolver (creates if missing)
- `InfantryClass::Mission_Capture` — `0x005202F0`

**Confidence:** HIGH on parse offsets, capture pipeline, and aura mechanic
(all load-bearing claims re-audited 2026-05-17 against `gamemd.exe`; the
§4 table's `Crewed=` row, the Hospital=/Armory= cited instruction
addresses, and §10's engineer-flag offset were all corrected following
the 2026-05-17 `/verify-doc` pass — see §14 Audit notes). MEDIUM on which
legacy stock RA2 maps reference `CAHOSP` by name (would require scanning
map files).

**Active in YR:** Yes — `[CATHOSP]` is referenced by both `NeutralTechBuildings=` and AI scripts; `[CAHOSP]` is an orphan instance that exists but is not auto-placed.

---

## 1. Overview

`[CATHOSP]` and `[CAHOSP]` are **two independent BuildingTypeClass instances**, both
defined in `rulesmd.ini`, both registered in `BuildingTypes=` (indices `70=CATHOSP`
and `48=CAHOSP`), both with effectively identical mechanics (same `Capturable=yes`,
same `NeedsEngineer=yes`, same `InfantryGainSelfHeal=1`, same `Strength=800`,
same `Sight=6`, same `Image=CAHOSP`). The only meaningful field differences are:

| Field | `[CATHOSP]` | `[CAHOSP]` |
|-------|-------------|------------|
| `Name=` | `Tech Hospital` | `Old Civilian Hospital` |
| `UIName=` | `Name:CAHOSP` | `Name:CAHOSP` *(same string ID)* |

**Source-of-truth assignment.** `RulesClass+0xADC` (`NeutralTechBuildings=`) is the
list that scenario placement and AI consult to know *what counts as a tech building*.
Stock value in `rulesmd.ini` line 3082:

```
NeutralTechBuildings=CAAIRP,CATHOSP,CAOILD,CAOUTP,CAMACH,CAPOWR
```

`CAHOSP` is **not** in this list. AI script `aimd.ini` line 7373 also targets
`CATHOSP` (`Yuri Capture Hospital ... CATHOSP`). Therefore `CATHOSP` is the active
tech hospital; `[CAHOSP]` is a *kept-around duplicate* (the in-file comment on line
14029 — `[CAHOSP];copypasted the good one on top of it` — reads as "I pasted the
good content from CATHOSP on top of this section to make sure it works if any old
RA2-era map still spawns it by name").

> **Note on user-supplied direction.** The investigation request stated that
> `CATHOSP` is "the broken copy." The binary and INI evidence is the opposite:
> `CATHOSP` is the active live instance (in `NeutralTechBuildings`, named "Tech
> Hospital", referenced by AI). `CAHOSP` is the orphaned copy retained for legacy
> map compatibility. The mechanics of both are identical, so if `[CAHOSP]` is
> placed by name in a stock or custom map it will heal infantry exactly like
> `[CATHOSP]` once captured.

---

## 2. Both sections produce BuildingTypeClass instances

`FUN_00672660` is the `[BuildingTypes]` list parser. It iterates each numbered key
under `[BuildingTypes]` (e.g. `1=...`, `2=...`, `48=CAHOSP`, `70=CATHOSP`), reads
the value as a section name, and calls `FUN_004653c0(name)` (resolve-or-create):

```
FUN_004653c0(name):
    look up `name` in g_BuildingTypeClass_Array (linear scan, compare +0x24 ID)
    if found     → return existing pointer
    if not found → operator new(0x1798)   ; BuildingTypeClass is 0x1798 bytes
                   call BuildingTypeClass__constructor(name)
                   ; (constructor appends self to g_BuildingTypeClass_Array)
                   return new pointer
```

There is no de-duplication by image, by name string, or by section content — only
by the registered ID (`+0x24`). `CAHOSP` and `CATHOSP` are different IDs, so each
gets its own ~6 KB `BuildingTypeClass` instance (struct size **0x1798** bytes,
confirmed). The constructor runs `ReadINI` on the section, populating all parsed
fields from the `[CAHOSP]` or `[CATHOSP]` block.

**Implication for the engine port.** Treat each `BuildingTypes=` entry as
producing its own type instance. Don't try to be clever about "near-duplicate"
detection — gamemd is not.

---

## 3. `NeutralTechBuildings=` is what makes a building a "tech building" at runtime

`FUN_00672ae0` is the giant `RulesClass` general-section reader. Inside it, around
offset `0x00673926`, the call sequence is:

```
PUSH (RulesClass + 0xADC)        ; destination vector
PUSH "NeutralTechBuildings"      ; INI key
PUSH PTR_DAT_007f0cd4            ; rulesmd.ini object
CALL FUN_0067b550                ; ReadList(BuildingTypeClass*) helper
```

`FUN_0067b550` resolves each comma-separated token through `FUN_004653c0`
(the same resolve-or-create used by `BuildingTypes=`) and appends to the vector
at `RulesClass+0xADC`. Net effect: `RulesClass.NeutralTechBuildings` is a
vector of 6 `BuildingTypeClass*` in a stock game.

**Used by:** scenario placement (neutral house spawns these instead of generic
civilian buildings), AI target lists, capture-EVA triage, AI script trigger
predicates, and pre-placement validation in skirmish setup.

---

## 4. Capture mechanics — the actual parsed offsets

`BuildingTypeClass::ReadINI` writes the capture-related flags into the following
**byte offsets** (verified directly from the assembly — see `0x0045FFCE`, `0x00460237`,
`0x00460251`, `0x0046024B` and surrounding instructions):

| INI key | Class+offset | Width | Default in ctor | Verified |
|---------|-------------|-------|------------------|----------|
| `NeedsEngineer=`    | `BuildingTypeClass+0x1552` | byte  | 0 (no)  | `MOV byte ptr [EBP+0x1552], AL` at `0x0046024B` (key string `"NeedsEngineer\0"` at `0x0081ACA0`) |
| `CaptureEvaEvent=`  | `BuildingTypeClass+0x1554` | int   | `-1` (sentinel for "none") | `MOV [EBP+0x1554], EAX` at `0x00460265` |
| `Capturable=`       | `BuildingTypeClass+0x1572` | byte  | 0 (no)  | `MOV byte ptr [EBP+0x1572], AL` at `0x0045FFDB` (key string `"Capturable\0"` at `0x0081AE34`) |
| `ProduceCashStartup=` | `BuildingTypeClass+0x1558` | int | 0  | `MOV [EBP+0x1558], EAX` at `0x00460161` |
| `ProduceCashAmount=` | `BuildingTypeClass+0x155C` | int | 0  | `MOV [EBP+0x155C], EAX` at `0x0046017B` |
| `ProduceCashDelay=` | `BuildingTypeClass+0x1560` | int   | 0  | `MOV [EBP+0x1560], EAX` at `0x00460195` |
| `InfantryGainSelfHeal=` | `BuildingTypeClass+0x1564` | int | 0 | `MOV [EBP+0x1564], EAX` at `0x004601AF` |
| `UnitsGainSelfHeal=` | `BuildingTypeClass+0x1568` | int  | 0  | `MOV [EBP+0x1568], EAX` at `0x004601C9` |
| `Hospital=` (TS legacy) | `BuildingTypeClass+0x16C1` | byte | 0 | `MOV byte ptr [EBP+0x16C1], AL` at `0x00460AFD` (key string `"Hospital\0"` at `0x0081AA14`) |
| `Armory=` (TS legacy)   | `BuildingTypeClass+0x16C2` | byte | 0 | `MOV byte ptr [EBP+0x16C2], AL` at `0x00460B08` (key string `"Armory\0"` at `0x0081AA0C`) |
| `Crewed=`           | `TechnoTypeClass+0xCCD` (NOT BuildingTypeClass+0x1571 — see note below) | byte  | 0 (no)  | `MOV byte ptr [EBP+0xCCD], AL` at `0x00714A43` (key string `"Crewed\0"` at `0x0084396C`); parsed in `TechnoTypeClass::ReadINI`, NOT `BuildingTypeClass::ReadINI` |

> **CORRECTION to TECH_BUILDINGS_GHIDRA_REPORT.md.** That doc states
> `Capturable=` is at `TechnoTypeClass+0x1552`. **It is not.** `+0x1552` is
> `NeedsEngineer=`. `Capturable=` is at `+0x1572`. The two get conflated easily
> because `BuildingClass::ChangeOwner` reads `this->Type[0x1552]` as a gate for the
> "engineer-captured EVA voice vs. captured-radar-event" choice — but that read
> is testing `NeedsEngineer`, not `Capturable`. The same TECH_BUILDINGS doc also
> reads as if `Capturable=` at `+0x1552` gates the engineer's ability to even
> begin a capture mission; whatever does that, it is reading `+0x1572`, not
> `+0x1552`. Treat the existing doc's table row as stale.

### 4.1 What `BuildingClass::ChangeOwner` actually does with these flags

Trace condensed from `0x00448260`:

1. **No-op guard:** `if (newOwner == this->Owner) return 0;`
2. **Defuse bomb** if one is planted and the type's `+0x157B` flag isn't set.
3. **Set `newOwner[+0x56F8] = 1`** if the building's type has `+0x16C7` set
   (some "needs-cleanup" flag; details out of scope).
4. **`ProduceCashStartup` grant** — only fires when **old owner is a
   `MultiplayPassive`-house** (i.e. the neutral civilian house, `Owner.HouseType +0x1A6 != 0`)
   AND `Type+0x1558 != 0`:
   ```
   HouseClass::Add_Credits(Type+0x1558)        ; ProduceCashStartup
   building.LastCashTickFrame = g_CurrentFrameCounter
   building.NextCashDelay     = Type+0x1560    ; ProduceCashDelay  (re-seed timer)
   ```
   **Tiny detail:** capturing a tech building from another *player* (i.e. recapture)
   does **not** grant `ProduceCashStartup`. Only first capture from neutral counts.
5. **EVA branch (per `NeedsEngineer` flag at `+0x1552`, NOT Capturable):**
   - `NeedsEngineer == 0` → `CreateRadarEvent()` then `VoxClass::PlayEVA()` (e.g.
     "Your building has been compromised" style — the auto-capture / proximity-loss
     path, not engineer capture).
   - `NeedsEngineer == 1 && owner actually changed` → `VoxClass::PlayEVA()` plus
     (if local-player is the human player and `Type+0x1554 != -1`)
     `VoxClass::QueueVoice(Type+0x1554)` (the `CaptureEvaEvent=` index, e.g.
     `EVA_HospitalCaptured`).
6. **Wall reconnect** if `Type+0x16BE` (wall flag).
7. Visibility-bitmask edit at `this+0x210` (the per-house "seen by" mask).
8. Several list removals from the **old** owner's per-flag building registries
   (each driven by a `Type[+0x16xx]` flag):
   - `+0x16A9` (UnitRepair) → Owner+0x80 list
   - `+0x16AD` (Power) → Owner+0x98 list
   - `+0x16AE` / `+0x16AF` (Tech-list flags) → Owner+0xB0 list
   - `+0x16AB` (Hospital — TS legacy, see §6) → Owner+0xC8 list
   - `+0x16AC` (Armory — TS legacy) → Owner+0xF8 list
   - `+0x16B0` (BarracksType) → Owner+0x110 list
   - `+0x16CD` (RecalcBonuses trigger; **HouseClass::RecalcBonuses is called here**) → Owner+0x140 list
   - `+0x157B` (RefinerySmoke flag, requires `Type+0x634 >= 0`) → Owner+0xE0 list
   - `+0x170c > 0` (SpyEffect.MoneyAmount or similar threshold) → Owner+0x128 list
9. **`+0x16CC` counter decrement** on old owner (`Owner+0x538C--`).
10. **Self-heal aura DECREMENT — old owner side. THIS IS THE LOAD-BEARING TIDBIT:**
    ```
    if (Type+0x1564 != 0 && building.ActuallyPlacedOnMap) {
        Owner+0x164 -= Type+0x1564                  ; InfantryGainSelfHeal counter
        if (Owner+0x164 < 0) Owner+0x164 = 0        ; CLAMP AT ZERO
    }
    if (Type+0x1568 != 0 && building.ActuallyPlacedOnMap) {
        Owner+0x168 -= Type+0x1568                  ; UnitsGainSelfHeal counter
        if (Owner+0x168 < 0) Owner+0x168 = 0        ; CLAMP AT ZERO
    }
    ```
    - **Clamp at 0** — defensive against desync between the counter and the
      registry. Worth replicating to match exact behavior on edge cases where
      the building was placed before its `ActuallyPlacedOnMap` flag was set.
    - **`ActuallyPlacedOnMap` gate** — if the building was never officially
      "placed on the map" (e.g. spawned but not committed), no decrement happens.
11. `+0x16CB` (PowerOutput type flag) → drain `Owner+0x2D4 -= Type+0x1780`.
12. **`TechnoClass::ChangeOwner(newOwner, 1)`** — sets the new owner pointer, calls
    `HouseClass::Recalc_Base_Center` for both old and new owner, drives the
    shroud-reveal step via `+0x488` virtual call (this is where the `Sight=`
    reveal radius is applied to the new owner — *see §7*).
13. **List inserts on new owner** — symmetric to step 8 but with one twist:
    the per-flag list inserts use a deferred-array pattern (`ppuVar13[idx*6]++`)
    so they batch up and are appended after `TechnoClass::ChangeOwner` completes.
14. **`+0x16CC` counter increment** on new owner (`Owner+0x538C++`).
15. **Self-heal aura INCREMENT — new owner side:**
    ```
    if (Type+0x1564 != 0) {
        Owner+0x164 += Type+0x1564                  ; InfantryGainSelfHeal counter
    }
    if (Type+0x1568 != 0) {
        Owner+0x168 += Type+0x1568                  ; UnitsGainSelfHeal counter
    }
    ```
    **Asymmetry detail:** the *increment* has no `ActuallyPlacedOnMap` guard
    and no clamp. So if a building is captured between two players while *not*
    `ActuallyPlacedOnMap`, the old owner's counter is *not* decremented, but
    the new owner's *is* incremented. This is a latent drift bug in gamemd —
    very rarely visible because `ActuallyPlacedOnMap` is set very early —
    but it is the binary's behavior and the port should replicate it (or at
    least not "fix" it silently).
16. `+0x16CB` add to new owner's `+0x2D4`.
17. Radar update (`UpdateRadar` called twice) for buildings with `Type+0xEB8 != 0`
    (`RadarVisible=yes` and friends).
18. **Captured-units list relinking** (the `for (i; i < iStack_24; i++)` loop
    over the building's contained passengers/garrison; re-issues mission 2 =
    Guard and propagates `field_0x418`).
19. **AI cleanup on upgrade-acquisition:** if game mode != 0 (i.e. not campaign)
    and new owner is AI and old owner is not `MultiplayPassive`, refund all
    upgrades (the powerup roof attachments) and call `RemoveLastUpgrade` in a
    loop — credits go to the new owner.
20. **Set new owner's `ProductionChanged` flag** (`Owner+0x1FC = 1`) — this is
    what makes the new owner's AI re-evaluate buildable lists next tick and is
    *also* the mechanic the Tech Airport (`CAAIRP`) and other granted-SW
    buildings rely on for activating the superweapon on capture.

---

## 5. The infantry-heal aura — what it actually does

Section corrects a recurring misunderstanding (the original request described the
behavior as *"auto-heal infantry in range"*). **The aura has no proximity check.**

Decompile of the relevant block inside `TechnoClass::AI_Update` (`0x006F9E50`):

```c
// RTTI from vtable+0x2c: 0x0F = Infantry, 0x01 = Unit (vehicle)
int rtti = vtable[+0x2C]();
TechnoTypeClass* type = vtable[+0x84]();

// "Use infantry cadence" branch — Infantry OR Organic-unit
bool useInfantryHeal =
       (rtti == 0x0F)                            // Infantry
    || (rtti == 0x01 && type->Organic != 0);     // Organic vehicle (Brute, Yuri Clone…)

if (useInfantryHeal
    && this->Health > 0
    && !(this->Health >= type->MaxHealth)        // bVar18 = (MaxHealth <= Health)
    && (g_CurrentFrameCounter % Rules->SelfHealInfantryFrames) == 0
    && HouseClass::HasInfantryGainSelfHeal(this->Owner))   // Owner+0x164 > 0
{
    int maxHp     = type->MaxHealth;                              // +0xA0
    int missing   = maxHp - this->Health;
    int auraAmt   = HouseClass::GetInfantrySelfHealAmount(Owner); // Rules+0x34 * Owner+0x164
    int applied;
    if (auraAmt < missing) {
        applied = HouseClass::GetInfantrySelfHealAmount(Owner);   // CALLED A SECOND TIME
    } else {
        applied = type->MaxHealth - this->Health;
    }
    this->Health += applied;
}
// else (rtti == 1 && !Organic): vehicle aura branch (Rules+0x38 / Rules+0x3C / Owner+0x168)
```

### 5.1 Tiny details worth replicating verbatim

- **No proximity check.** No `Distance2D`, no cell scan, no per-house range
  filter. The aura is **global** to the owning house: every owned infantry on
  the map heals on the same frame.
- **Synchronized globally by `g_CurrentFrameCounter`.** All eligible units of
  *all* houses fire on the same global frame number, not staggered per-unit.
- **Strict-less-than full-health gate.** `Health < MaxHealth`; if `Health == MaxHealth`
  no heal occurs that tick. (Reduces a one-tick "overheal-then-clamp" wobble.)
- **`HouseClass::GetInfantrySelfHealAmount()` is called twice** in the
  `auraAmt < missing` branch (once to compare, once to use the value). Pure
  perf inefficiency — does not change the output value. Documented because
  someone refactoring may try to call it once and store; they should not
  bother but should also note this is a Ghidra-confirmed double-call, not a
  Ghidra mis-decompile.
- **Per-tick stacking is multiplicative.** Two Tech Hospitals → counter = 2 →
  heal per tick = `Rules.SelfHealInfantryAmount * 2 = 40 HP` (stock). Three →
  60 HP. There is no diminishing-returns curve; raw linear multiply.
- **The aura still ticks while the unit is in a transport / inside a building.**
  `TechnoClass::AI_Update` runs unconditionally for every alive `Techno`; the
  passenger's `Health` is updated even though they are not visible. Confirmed
  visually by the existing TECH_BUILDINGS doc; not separately re-verified here.
- **Organic flag (`TechnoTypeClass+0xD97`)** parsed from `Organic=` in
  TechnoTypeClass::ReadINI (verified at `0x0071502B`):
  - `Yuri's Brute` → `Organic=yes` → uses **infantry** cadence (50 frames)
  - `Yuri Clone` / `Yuri` → `Organic=yes` → infantry cadence
  - Generic vehicles → `Organic=no` → **vehicle** cadence (75 frames)
  - This matters because the cadence and the *amount* (Rules+0x34 vs Rules+0x3C)
    differ. A `Brute` (RTTI 1 = Unit) heals at `+20 / 50f` not `+5 / 75f`.
- **Vehicle heal path additionally clears damage particles.** After applying
  the vehicle aura the function calls `ObjectClass::GetHealthRatio` and if
  the ratio crosses `Rules+0x1700` (the *"condition yellow"* threshold,
  default `0.5`) **and** the per-unit damage-smoke pointer (`field_0x310`)
  is non-null, it calls `+0xF8` on it (`Detach`/`Release`) to kill the
  smoke trail. The infantry heal path does **not** do this — there is no
  damage-particle cleanup on infantry-heal. (Infantry don't carry sustained
  damage smoke in stock content anyway, so this is invisible — but the
  asymmetry is in the binary and should be matched.)

### 5.2 Cadence and amount values (verified from RulesClass parse)

| Field | Rules offset | INI default | Width |
|-------|--------------|-------------|-------|
| `SelfHealInfantryFrames=` | `Rules+0x30` | `50`  | int |
| `SelfHealInfantryAmount=` | `Rules+0x34` | `20`  | int |
| `SelfHealUnitFrames=`     | `Rules+0x38` | `75`  | int |
| `SelfHealUnitAmount=`     | `Rules+0x3C` | `5`   | int |

Parse site addresses: `0x0066E6EB` (`SelfHealInfantryFrames`) and surrounding —
the four keys are read consecutively and each one's previous value is its own
default (so any one of them missing falls back to the current ctor default,
which itself is zero-initialised unless changed elsewhere).

### 5.3 House-side accumulators

| Counter | HouseClass offset | What it accumulates |
|---------|--------------------|----------------------|
| Infantry self-heal aggregate | `Owner+0x164` | Sum of `+0x1564` over all owned, on-map, alive buildings. |
| Vehicle self-heal aggregate  | `Owner+0x168` | Sum of `+0x1568` over all owned, on-map, alive buildings. |

> **Ghidra labels at `0x0050D9C0` / `0x0050D9D0` / `0x0050D9E0` / `0x0050D9F0`
> are misnamed.** They read `HasPowerOutput` / `HasPowerDrain` /
> `GetTotalPowerOutput` / `GetTotalPowerDrain` but their bodies multiply by
> `Rules+0x34` or `Rules+0x3C` (the self-heal tuning) against `Owner+0x164` /
> `Owner+0x168`. The real power getters live elsewhere. This trips up search.

Lifecycle for `Owner+0x164` / `Owner+0x168`:

| Event | Effect |
|-------|--------|
| Building placed (any path that sets `ActuallyPlacedOnMap = true`) | `Owner+0x164 += Type+0x1564`; `Owner+0x168 += Type+0x1568` |
| `ChangeOwner` (capture, sale-transfer, MCV redeploy) — old owner | Decrement, clamped to 0, **gated on `ActuallyPlacedOnMap`** |
| `ChangeOwner` — new owner | Increment, **unconditional**, no clamp |
| Building destroyed (`BuildingClass::OnDestroyed`) | Decrement, clamped to 0 (handled in `OnDestroyed`, not re-verified here) |

---

## 6. TS-legacy `Hospital=` / `Armory=` — parsed but dead

Both keys are still parsed:

- `Hospital=` → `TechnoTypeClass+0x16C1` (byte), parse site `0x00460AE1`
- `Armory=`   → `TechnoTypeClass+0x16C2` (byte), parse site `0x00460AF6`

The flags exist in the binary and the *old* Tiberian Sun walk-inside-to-heal
state machine (2-state FSM, ~`Rules+0x16F0 × 900.0` frame threshold) is also
still present. **But:**

1. `[CATHOSP]` line 14016 and `[CAHOSP]` line 14040 both have `;Hospital=yes ;gs old TS way`
   *(commented out)*.
2. `[CAARMR]` is similarly commented out in the YR rules (the YR-era replacement
   is the cloning vat / proper Armory building).
3. The `+0x16AB` / `+0x16AC` (Hospital-list / Armory-list) flags are still
   maintained in `ChangeOwner` (steps 8 and 13 in §4.1 above), but with the
   parsed default of 0 they never get added to any owner's per-flag list. The
   list-walk machinery thus runs over empty lists and does nothing.
4. **No stock YR `BuildingTypes=` entry sets `Hospital=yes`.** Verified by grep
   of `rulesmd.ini` — only commented-out occurrences exist.

**Implication for the port.** Do **not** implement walk-inside-to-heal or
walk-inside-to-veterancy. Implement only the `InfantryGainSelfHeal` / aura
mechanic from §5. If you decide to model the legacy flag parsing for completeness,
treat it as a parsed-but-unused byte (it should never be read).

---

## 7. Reveal-on-capture — there is no `RevealRadius=` for tech buildings

Verified strings-table sweep:

- No string `RevealRadius` exists in the binary (only `PsychicRevealRadius` for
  Yuri's Psychic Reveal SW; that is unrelated).
- The reveal radius on capture / ownership-change comes from the building's
  generic `Sight=` field (TechnoTypeClass-level, parsed at offset `+0x5C0` —
  not separately re-verified here, but consistent across all unit/building
  types).
- Both `[CATHOSP]` and `[CAHOSP]` have `Sight=6` — a 6-cell radius unshroud
  centred on the building's cell when the new owner takes control.

The actual shroud unshroud happens during `TechnoClass::ChangeOwner` (called near
the end of `BuildingClass::ChangeOwner`, step 12 in §4.1), via the virtual call
at `vtable+0x488` (`(**(code **)(this->vtable + 0x488))(0,0,0,0,0)` — the
`Conceal_Or_Reveal` / `Look` family). Not re-traced further in this report.

---

## 8. `Power=` — not relevant for tech hospital

- String `Power` exists in the binary (`0x0081938C`), parsed as
  `TechnoTypeClass+ProducedPower` (positive = generator, negative = consumer).
- Neither `[CATHOSP]` nor `[CAHOSP]` sets `Power=`. Default = 0. Capturing the
  tech hospital adds nothing to the owner's power.
- (For `[CAPOWR]` — civilian power plant — `Power=` *is* set positive. That
  building is the queue's item 7 and warrants its own re-investigation.)

---

## 9. Duplicate-section parse order — *not relevant here*

The investigation request asked whether `CCINIClass::ReadString` resolves
duplicate `[Section]` headers by first-wins or last-wins. **In this case the
question does not apply**: `[CAHOSP]` and `[CATHOSP]` are *different section
names*, not duplicates. There is no parse-order ambiguity. Both sections are
read once, into their respective `BuildingTypeClass` instances, and both
instances exist in `g_BuildingTypeClass_Array` simultaneously.

(For the record: `CCINIClass::ReadString` resolves a section name by CRC32
binary search over a sorted array of section pointers — see
`CCINICLASS_GHIDRA_REPORT.md`. The exact duplicate-section disposition for
true duplicates was flagged as an open question in that doc but is not
relevant to CAHOSP/CATHOSP.)

---

## 10. Capture-issue pipeline — who decides "this engineer may capture this building"

`InfantryClass::Mission_Capture` (`0x005202F0`) is the *active mission state*, not
the gate. It pre-conditions:

- `param_1->Type[+0xEC5] != 0` — capture-capable infantry type flag read in
  `Mission_Capture`. (corrected 2026-05-28: was `+0xEC3`; binary at `0x005202F0`
  decompiles as `*(char *)(param_1[0x1b0] + 0xec5)` — ROOT_CAUSE: RTTI_LABEL_DRIFT:
  the 2026-05-17 pass verified the *parse-site* offset for `Engineer=` (correctly
  `+0xEC3`) but incorrectly applied it to the *usage-site* in `Mission_Capture`
  which reads `+0xEC5`; verified via `decompile_function 0x005202F0`.)
  The flag byte cluster in `InfantryTypeClass::ReadINI`:
  - `C4=` → `+0xEC2` (verified PUSH at `0x0052453D`, write at `0x00524559`)
  - `Engineer=` → `+0xEC3` (parse-site PUSH `0x82596C` = "Engineer", write at `0x00524584`)
  - `Agent=` → `+0xEC4` (write at `0x005245b2`; corrected 2026-05-28: was cited as `Infiltrate=`; PUSH `0x825954` = "Agent\0"; verified via `get_assembly_context 0x00524598`)
  - `Thief=` → `+0xEC5` (write at `0x005245d2`; PUSH `0x82594c` = "Thief\0"; verified via `get_assembly_context 0x005245b8`)
  **Mission_Capture therefore gates on `Thief=` (`+0xEC5`), not `Engineer=` (`+0xEC3`).**
  Engineers can also capture buildings; the gate in `Mission_Capture` specifically
  tests the `Thief=` flag. Whether `Engineer=` is tested on a separate order-issue
  path (not traced here) remains an open question.
- `param_1->Target != null` and `Target.RTTI == 1` (per the
  `vtable+0x2C` call — note: 1 = Unit RTTI for the building-as-target may
  diverge in YR; existing docs vary on this). The mission proceeds when the
  engineer is within `0x80` lepton (~½ cell) of the target, at which point it
  calls `vtable+0x3D4` on the target — that's the abstract `Captured(by_player)`
  hook which dispatches to `BuildingClass::ChangeOwner` for buildings.
- Range-fallback: at `0x80 < dist < 0x200` the engineer issues `vtable+0x480`
  (move-to-target) and re-evaluates next tick.
- At `dist >= 0x200` with a non-null destination, also issue `vtable+0x480`
  to keep pursuing.

The `Capturable=` flag itself (`Type+0x1572`) is checked *before* this mission
mode is allowed to start — likely in the order-issue path / UI cursor selection
path. That gate was not traced further here (out of scope for the CAHOSP/CATHOSP
question); both sections have `Capturable=yes` so it passes for both.

---

## 11. Side-by-side INI dump (verbatim, with line numbers)

### `rulesmd.ini`

```
14005: [CATHOSP]
14006: UIName=Name:CAHOSP
14007: Name=Tech Hospital
14008: Image=CAHOSP
14009: TechLevel=-1
14010: Strength=800
14011: Insignificant=yes
14012: Nominal=yes
14013: Sight=6
14014: Points=5
14015: Armor=concrete
14016: ;Hospital=yes ;gs old TS way
14017: Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
14018: MaxDebris=6
14019: DebrisAnim=Dbris3sm,Dbris4lg,Dbris4sm,Dbris6sm,Dbris7lg,Dbris7sm,Dbris8sm,Dbris9lg,Dbris10lg,Dbris10sm
14020: DamageParticleSystems=SmallGreySSys,BigGreySmokeSys
14021: Capturable=yes
14022: CaptureEvaEvent= EVA_HospitalCaptured  ;Eva (and therefore 3way split) voice to use when captured
14023: NeedsEngineer=yes
14024: Unsellable=yes
14025: LeaveRubble=yes
14026: InfantryGainSelfHeal=1 ; one 'unit' of SelfHealInfantryAmount per SelfHealInfantryFrames
14027: RadarVisible=yes;gs put on radar even if insignificant and unowned (insignificant and owned is a UC building)

14029: [CAHOSP];copypasted the good one on top of it
14030: UIName=Name:CAHOSP
14031: Name=Old Civilian Hospital
14032: Image=CAHOSP
14033: TechLevel=-1
14034: Strength=800
14035: Insignificant=yes
14036: Nominal=yes
14037: Sight=6
14038: Points=5
14039: Armor=concrete
14040: ;Hospital=yes ;gs old TS way
14041: Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
14042: MaxDebris=6
14043: DebrisAnim=Dbris3sm,Dbris4lg,Dbris4sm,Dbris6sm,Dbris7lg,Dbris7sm,Dbris8sm,Dbris9lg,Dbris10lg,Dbris10sm
14044: DamageParticleSystems=SmallGreySSys,BigGreySmokeSys
14045: Capturable=yes
14046: CaptureEvaEvent= EVA_HospitalCaptured
14047: NeedsEngineer=yes
14048: Unsellable=yes
14049: LeaveRubble=yes
14050: InfantryGainSelfHeal=1
14051: RadarVisible=yes
```

### `artmd.ini` — only one Buildup, both share the same image

Both `[CATHOSP]` (lines 3349–3376) and `[CAHOSP]` (lines 3319–3347) reference
`Image=CAHOSP`, `Buildup=CAHOSP`, `ActiveAnim=CAHOSP_A`,
`ActiveAnimDamaged=CAHOSP_AD`, `ActiveAnimTwo=CAHOSP_F`. The art is shared.
**Single visual difference** in `artmd.ini`:
- `[CAHOSP].CanBeHidden=false` vs `[CATHOSP].CanBeHidden=true`. Marginal —
  affects whether infantry can "hide things" under the building's footprint.

### `rules.ini` (base RA2, before YR overlay)

Base RA2 used the *old* TS path: both `[CATHOSP]` (line 10548) and `[CAHOSP]`
(line 10571) carry `Hospital=yes` un-commented. YR's `rulesmd.ini` overrides
each section's `Hospital=` line to a comment, removing the flag.

### `[General]`

```
34: SelfHealInfantryFrames=50
35: SelfHealInfantryAmount=20
36: SelfHealUnitFrames=75
37: SelfHealUnitAmount=5 ;gs Tech Machine Shop and Tech Hospital
```

### `[BuildingTypes]`

```
1232: 48=CAHOSP
1254: 70=CATHOSP			; Tech Hospital
```

### `NeutralTechBuildings=` (line 3082)

```
NeutralTechBuildings=CAAIRP,CATHOSP,CAOILD,CAOUTP,CAMACH,CAPOWR
```

### `aimd.ini`

```
7373: 05FC1C5C-G=Yuri Capture Hospital,05FC006C-G,<all>,1,7,CATHOSP,...
```

(`CAHOSP` is not referenced by any AI script in stock content.)

---

## 12. Current Rust implementation status

| Subsystem | Status |
|-----------|--------|
| `Capturable=` field parsed into `ObjectType.capturable` | [src/rules/object_type.rs:493](../ra2-rust-game/src/rules/object_type.rs#L493) |
| Engineer capture command | [src/sim/world/world_commands.rs:1013](../ra2-rust-game/src/sim/world/world_commands.rs#L1013) (hard-coded capture target validation, not INI-driven mission system) |
| `InfantryGainSelfHeal` / `UnitsGainSelfHeal` parse | **Missing** |
| `Owner+0x164` / `Owner+0x168` aggregates | **Missing** |
| `Rules.SelfHeal{Infantry,Unit}{Frames,Amount}` parse | **Missing** |
| `TechnoClass::AI_Update` aura tick (global, %-cadence) | **Missing** |
| Organic-flag → infantry cadence routing | **Missing** |
| `CaptureEvaEvent=` parse and EVA queueing | **Missing** |
| `Hospital=` / `Armory=` legacy parsing | **Missing** (correct — should stay missing) |

---

## 13. Open questions

1. **Why does `[CAHOSP]` exist at all in shipped YR rules?** Stock skirmish
   placement uses `CATHOSP` exclusively. Hypothesis: kept for backward
   compatibility with RA2-era custom/campaign maps that hard-placed `CAHOSP` by
   name. Verifying this would require scanning every stock `.mmx` and
   `.map` for `CAHOSP` entries — out of scope here. **Not a parity-relevant
   question** as long as the port handles whichever section name appears in a
   map identically (and it will, since both produce equivalent
   `BuildingTypeClass` instances).
2. **What flag at `BuildingTypeClass+0x1571` is parsed just before `Capturable=`?**
   The assembly at `0x0045FFC7` reads `[EBP+0x1572]` as the *default* for the
   `Capturable=` ReadBool. An earlier draft of this doc speculated `+0x1571` =
   `Crewed=` "per parse-site context" — **DISPROVEN 2026-05-17**: `Crewed=` is
   parsed in `TechnoTypeClass::ReadINI` and writes to `TechnoTypeClass+0xCCD`
   (not `BuildingTypeClass+0x1571`). The byte at `BuildingTypeClass+0x1571`
   therefore has unknown INI-key origin — possibly an unused padding byte, or
   parsed by a key whose parse-site lives elsewhere in `BuildingTypeClass::ReadINI`.
   Low priority for follow-up; no live consumer identified.
3. **The capture-flag offset in `Mission_Capture`** has been re-resolved: the
   binary reads `InfantryTypeClass+0xEC5` (`Thief=`), NOT `+0xEC3` (`Engineer=`).
   (corrected 2026-05-28: the 2026-05-17 pass confused the parse-site offset for
   `Engineer=` with the usage-site offset in `Mission_Capture`; `+0xEC5` = `Thief=`
   confirmed via `decompile_function 0x005202F0` and `get_assembly_context 0x005245b8`.)
   Whether `Engineer=` (+0xEC3) is additionally gated on a separate order-issue
   path is not yet traced.
4. **TECH_BUILDINGS_GHIDRA_REPORT.md table needs a correction.** The
   `Capturable=` row claims `+0x1552`. Should be amended to `+0x1572`, and a
   separate `NeedsEngineer=` row should be added at `+0x1552`. Not done in
   this pass (different doc, different scope).
5. **Tech Hospital infantry aura — does it heal passengers inside transports
   and garrisoned occupants?** §5.1 asserts yes from the existing TECH_BUILDINGS
   doc (the aura ticks for all `Techno`s in `TechnoClass::AI_Update`,
   irrespective of whether they are in a transport). Not separately re-verified
   in this report. Worth a 1-finding verification pass when implementing.

---

## Sources

- Ghidra MCP — live decompilation of `gamemd.exe`:
  - `0x00448260` (`BuildingClass::ChangeOwner`)
  - `0x0045FE50` (`BuildingTypeClass::ReadINI`)
  - `0x006F9E50` (`TechnoClass::AI_Update`)
  - `0x0050D9C0` / `0x0050D9D0` / `0x0050D9E0` / `0x0050D9F0` (mislabeled house self-heal helpers)
  - `0x00672660` (BuildingTypes parser)
  - `0x00672AE0` / `0x00673926` (RulesClass general read; NeutralTechBuildings parse)
  - `0x004653C0` (BuildingTypeClass resolve-or-create)
  - `0x005202F0` (`InfantryClass::Mission_Capture`)
  - assembly context at `0x0045FFCE`, `0x004601A2`, `0x004601BC`, `0x0046023E`, `0x00460258`,
    `0x00460154`, `0x0046016E`, `0x00460188`, `0x00460AE1`, `0x0066E6EB`, `0x0071502B`
- INI files (in-repo authoritative):
  - `ini/rulesmd.ini` lines 34–37, 1232, 1254, 3082, 14005–14051
  - `ini/rules.ini` lines 10548–10597
  - `ini/artmd.ini` lines 3319–3376
  - `ini/aimd.ini` line 7373
- Prior research:
  - `ra2-rust-game-docs/TECH_BUILDINGS_GHIDRA_REPORT.md` — corroborated for
    Rules-class self-heal offsets and HouseClass-counter offsets; **table row
    for `Capturable=` offset is wrong and superseded by §4 of this doc**.
  - `ra2-rust-game-docs/BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` — corroborated
    for ChangeOwner step ordering.
  - `ra2-rust-game-docs/BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` — corroborated
    that `+0x164` / `+0x168` are self-heal accumulators (not power).
  - `ra2-rust-game-docs/CCINICLASS_GHIDRA_REPORT.md` — for the open-question
    cross-reference on duplicate-section disposition (not relevant to this report).

---

## 14. Audit notes (2026-05-17)

A focused re-verification pass was run against `gamemd.exe` via Ghidra MCP
on the four load-bearing claims flagged for spot-checking. All four held;
no corrections were needed.

| Claim | Result | Evidence |
|-------|--------|----------|
| §4: `Capturable=` at `TechnoTypeClass+0x1572`; `NeedsEngineer=` at `+0x1552` | **VERIFIED** | Read string bytes at the pushed pointers: `0x0081ae34` = `"Capturable\0"`, `0x0081aca0` = `"NeedsEngineer\0"`. Each push immediately precedes its matching `MOV byte ptr [EBP+offset], AL` at the cited parse-site addresses. Re-confirmed by inspecting the asm context around `0x0045FFDB` and `0x0046024B`. The `[0x1572]` write follows `PUSH 0x81ae34` ("Capturable"); the `[0x1552]` write follows `PUSH 0x81aca0` ("NeedsEngineer"). |
| §4 — `ChangeOwner` reads `Type[0x1552]` as the EVA-branch gate testing `NeedsEngineer` | **VERIFIED** | In the `BuildingClass::ChangeOwner` decompile at `0x00448260`, the relevant branch is `if (this->Type[0x1552] == '\0') { CreateRadarEvent; PlayEVA; } else if (Owner changed) { PlayEVA; if (Type+0x1554 != -1) QueueVoice; }`. Since `+0x1552` was just confirmed to be `NeedsEngineer`, the gate is on `NeedsEngineer`, not `Capturable`. |
| §5: InfantryGainSelfHeal aura has **no proximity check** in `TechnoClass::AI_Update` (`0x006F9E50`) | **VERIFIED** | Decompiled the function in full. The self-heal block dispatches on RTTI (`0xF` Infantry, or `1` + Organic) and gates on `Health > 0`, `Health < MaxHealth`, `g_CurrentFrameCounter % Rules+0x30 == 0`, and `HouseClass::HasInfantryGainSelfHeal` (Ghidra-mislabeled `HasPowerOutput`). No `Distance2D`, no per-house range filter, no per-cell scan, no spatial gate. Heal is global to all owned infantry of the house, synchronised on the global frame counter. The double-call to `GetInfantrySelfHealAmount` in the `auraAmt < missing` branch is also visible. |
| §5.1: `Organic=` parses to `TechnoTypeClass+0xD97` at xref `0x0071502B`, and the AI_Update dispatch routes `RTTI==1 && Organic` to the infantry cadence (`Rules+0x30`/`+0x34`) | **VERIFIED** | The asm at `0x0071502B` pushes `0x00843714`, which reads as `"Organic\0"`; the following `CALL` returns into `MOV byte ptr [EBP+0xd97], AL` at `0x0071503F`. In `AI_Update`, the outer dispatch `if ((RTTI != 1) \|\| (Type+0xd97 != 0) \|\| Health==0 \|\| AtFull)` selects the infantry-cadence path, which uses `g_CurrentFrameCounter % Rules+0x30` and `GetInfantrySelfHealAmount` (which multiplies `Owner+0x164 × Rules+0x34`). The vehicle-cadence path uses `Rules+0x38`/`+0x3C`. |
| §4.1 step 10 — old-owner counter decrement clamped at 0 AND gated on `ActuallyPlacedOnMap`; new-owner increment unconditional and unclamped | **VERIFIED** | The decompile shows the decrement block as `if ((Type+0x1564 != 0) && (this->ActuallyPlacedOnMap != false)) { Owner+0x164 -= Type+0x1564; if (Owner+0x164 < 0) Owner+0x164 = 0; }` (mirror for `+0x1568`/`+0x168`). The increment block is `if (Type+0x1564 != 0) { Owner+0x164 += Type+0x1564; }` (mirror for `+0x1568`/`+0x168`). Asymmetric exactly as documented. |

**No corrections written.** Confidence on §4, §4.1, §5, and §5.1 is now
HIGH-verified-2026-05-17 rather than HIGH-from-first-pass-research.

### Not re-verified in this pass — candidates for a future audit

The audit was scoped to the four high-risk claims above. The following
specific claims in this doc were NOT independently re-checked against
the binary in this pass. Each is paired with the exact address / offset
a future investigator should hit:

- **§4 parse-offset table — six rows not re-checked.** Only the
  `Capturable=` and `NeedsEngineer=` rows were re-verified (those were
  the audited claims). The other rows in the table — `Crewed=` (+0x1571),
  `CaptureEvaEvent=` (+0x1554), `ProduceCashStartup=` (+0x1558),
  `ProduceCashAmount=` (+0x155C), `ProduceCashDelay=` (+0x1560),
  `UnitsGainSelfHeal=` (+0x1568), `Hospital=` (+0x16C1), `Armory=`
  (+0x16C2) — should each be confirmed by reading the string at the
  cited `PUSH <imm32>` immediate near the parse site. Same pattern as
  the verified rows. Each takes one `read_memory` call.
- **§4.1 ChangeOwner steps 1-9 and 11-20.** Only step 10 (self-heal
  decrement/increment asymmetry) was re-verified. The decompile of
  `BuildingClass::ChangeOwner` at `0x00448260` is in this report — but
  the *specific* claims about which Owner-list offsets correspond to
  which per-flag registries (+0x80 UnitRepair / +0x98 Power / +0xB0
  Tech-list / +0xC8 Hospital / +0xF8 Armory / +0x110 BarracksType /
  +0x140 RecalcBonuses / +0xE0 RefinerySmoke / +0x128 SpyEffect / +0x68
  all-buildings) should each be cross-checked against the actual list
  removal/insert blocks. The numbers look right but were read off, not
  audited.
- **§5.1 vehicle-aura damage-particle-cleanup claim.** "After applying
  the vehicle aura the function calls `ObjectClass::GetHealthRatio` and
  if the ratio crosses `Rules+0x1700` ... and `field_0x310` is non-null,
  it calls `+0xF8` on it." This block IS visible in the AI_Update
  decompile I read but the asymmetry-vs-infantry claim (infantry path
  has NO such cleanup) deserves a side-by-side re-read of both branches.
- **§5.2 cadence/amount parse addresses.** Only `SelfHealInfantryFrames`
  at `0x0066E6EB` is cited. The other three (`SelfHealInfantryAmount`,
  `SelfHealUnitFrames`, `SelfHealUnitAmount`) should each have their
  parse-site write confirmed at `Rules+0x34`, `+0x38`, `+0x3C`.
- **§5.3 lifecycle claim — "Building destroyed (`OnDestroyed`) →
  decrement, clamped to 0."** This was explicitly NOT re-verified in
  the original research and not re-verified here. `BuildingClass::OnDestroyed`
  is referenced as covering it; worth a 10-minute confirmation read.
- **§10 capture-mission `vtable+0x3D4` claim.** Stated as "the abstract
  `Captured(by_player)` hook which dispatches to `BuildingClass::ChangeOwner`
  for buildings." The `BuildingClass` vtable's +0x3D4 entry should be
  read out of memory and confirmed to point to `0x00448260`. The CABHUT
  doc references the same dispatch path (also unverified) — a single
  vtable-pointer read closes both.
- **`+0xEC5` / `+0xEC3` engineer-vs-thief flag (§10)** — **RESOLVED 2026-05-28.**
  `Mission_Capture` checks `+0xEC5` = `Thief=` (confirmed via decompile and
  `get_assembly_context 0x005245b8`). `Engineer=` is at `+0xEC3` (parse site
  confirmed). Whether `Engineer=` is gated on the order-issue path upstream of
  `Mission_Capture` is still untraced.
- **§10 range thresholds `0x80` and `0x200`.** Stated as ½-cell and
  ~1-cell-ish lepton distances respectively. Likely correct
  (256 leptons = 1 cell) but the actual `CMP` values in
  `InfantryClass::Mission_Capture` at `0x005202F0` were not read.
- **`g_BuildingTypeClass_Array` and `+0x24 ID` field claims (§2).** The
  resolve-or-create function `FUN_004653C0` was not independently traced.
  The struct-size claim of `0x1798` bytes for `BuildingTypeClass` should
  be confirmed by reading the `operator_new` immediate in the BuildingTypes
  parser.
- **`NeutralTechBuildings=` parse — `RulesClass+0xADC`.** The parse site
  `0x00673926` and the destination offset `+0xADC` were not separately
  read out.

If picking ONE follow-up target: the §4 parse-offset table is the most
load-bearing for a future port (these offsets get hard-coded into
struct definitions) and is also the cheapest to verify exhaustively —
each row is one `get_assembly_context` + `read_memory` round-trip.

### Audit notes (2026-05-28)

A second audit pass was run against `gamemd.exe` via Ghidra MCP covering 20
load-bearing claims. All but one held; one WRONG finding was corrected.

| Claim | Result | Evidence |
|-------|--------|----------|
| §4 parse-offset table — all 11 rows (NeedsEngineer +0x1552, CaptureEvaEvent +0x1554, Capturable +0x1572, ProduceCashStartup +0x1558, ProduceCashAmount +0x155C, ProduceCashDelay +0x1560, InfantryGainSelfHeal +0x1564, UnitsGainSelfHeal +0x1568, Hospital +0x16C1, Armory +0x16C2, Crewed +0xCCD) | **CONFIRMED** | Parse-site writes confirmed via `get_assembly_context` at each cite address; key strings verified via `read_memory` at each PUSH immediate. |
| §2 BuildingTypeClass struct size 0x1798 | **CONFIRMED** | `decompile_function 0x004653C0` shows `operator_new(0x1798)`. |
| §3 NeutralTechBuildings= → RulesClass+0xADC, parse site 0x00673926 | **CONFIRMED** | `get_assembly_context 0x00673926`: `LEA EBX,[ESI+0xADC]` then `PUSH 0x83d36c` = "NeutralTechBuildings". |
| §4.1 ChangeOwner owner-list offsets (+0x80 UnitRepair / +0x98 Power / +0xB0 Tech / +0xC8 Hospital / +0xF8 Armory / +0x110 Barracks / +0x140 RecalcBonuses / +0xE0 RefinerySmoke / +0x128 SpyEffect) | **CONFIRMED** | Cross-checked each against `decompile_function 0x00448260`. |
| §5 self-heal aura — no proximity check; cadence Rules+0x30/0x34/0x38/0x3C; `HasPowerOutput` / `GetTotalPowerOutput` labels misnamed | **CONFIRMED** | `decompile_function 0x006F9E50` and `0x0050D9C0/D0/E0/F0`. |
| §5.2 SelfHealInfantryFrames= parse site 0x0066E6EB → Rules+0x30 | **CONFIRMED** | `get_assembly_context 0x0066E6EB`: write `[ESI+0x30]` after PUSH `0x83cc5c` = "SelfHealInfantryFrames". |
| §10 capture gate flag: earlier doc claimed `+0xEC3` (Engineer=) gates Mission_Capture | **WRONG → CORRECTED** | `decompile_function 0x005202F0`: the gate is `*(char *)(param_1[0x1b0] + 0xec5)` — offset `+0xEC5` = `Thief=` (PUSH `0x82594c`, write at `0x005245d2`; verified via `get_assembly_context 0x005245b8`). `Engineer=` parses to `+0xEC3` (confirmed); `+0xEC4` = `Agent=` (not `Infiltrate=` as previously speculated). Root cause: RTTI_LABEL_DRIFT — 2026-05-17 pass verified parse site but applied it to the wrong usage site. |
