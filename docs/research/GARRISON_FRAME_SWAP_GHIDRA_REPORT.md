---
name: Garrison Frame Swap (CanBeOccupied branch of GetCurrentFrame)
description: BuildingClass::GetCurrentFrame branch 4 — frame index for CanBeOccupied (garrisonable) buildings based on occupancy + damage tier. Includes Type+0x634 identification (TechLevel), the (base==3 && TechLevel==-1) collapse rule, and the BState gating that controls when this branch fires.
type: reference
---

# Garrison Frame Swap — Ghidra Research Report

**Primary address:** `0x0043EF90` — `BuildingClass::GetCurrentFrame`
**Confidence:** HIGH (decompiled, helper calls resolved, all magic numbers identified)
**Active in YR:** Yes — runs every frame for every CanBeOccupied building drawn.

This report scopes ONLY the `CanBeOccupied` branch (branch 4) of GetCurrentFrame.
For the full GetCurrentFrame branch tree (LaserFence / FirestormWall / Gate /
Selling / damaged-Anim-max), see `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md` §3.2.

---

## 1. Overview

For garrisonable civilian buildings (CABHUT, CABARN01-04, CAGAS01, CALA01-10,
CABUNK01-04, etc. — **95 garrisonable types found in rulesmd.ini**), gamemd
chooses the building body SHP frame using a small formula based on occupant
count and current damage tier. The result selects between the SHP's four
canonical frames:

- frame 0 = empty + healthy
- frame 1 = empty + red-damaged (also: civilian collapse target for occupied+red)
- frame 2 = occupied + healthy
- frame 3 = occupied + yellow-damaged (only reachable on buildable garrisons)

The civilian-specific collapse rule (frame 3 → frame 1 when `TechLevel == -1`)
matches the typical 3-frame layout of civilian SHPs.

**Important gating:** branch 4 only runs when `BuildingClass+0x534 != 0`
(i.e., the building is in a non-zero anim BState). For most healthy idle
buildings, BState = 0 and GetCurrentFrame returns the raw `+0xF8` anim phase
instead. See §3.3 below.

---

## 2. Class layout / key offsets

### BuildingClass (instance) fields read by branch 4

| Offset | Type | Field | Notes |
|---|---|---|---|
| `+0x520` | ptr | `Type` (BuildingTypeClass*) | Used both as `this->Type` and via vtable+0x84 trampoline |
| `+0x534` | int | `CurrentAnimState` (BState index) | Gates which branch of GetCurrentFrame fires; `0`= idle/healthy, `1` = damaged-idle, `2+` = production states. Master report calls this "DamagedState"; UpdateAnimation report calls it "CurrentAnimState" — same field, different functional name. |
| `+0x538` | int | `PendingAnimState` | -1 = none. When set, BuildingClass::Update commits `+0x534 = +0x538` and resets `+0xF8` to the new anim's start frame. |
| `+0x694` | int | Occupant `Count` (returned by `GetOccupantCount` vtable+0x408 = `0x004581F0`) | DynamicVectorClass<InfantryClass*>.Count at building+0x694 |
| `+0xF8` | int | `CurrentFrame` within active BState | Returned by branch 3 for healthy idle buildings |
| `+0x6E6` | bool | `IsDamaged` | **Separate** field, written by `ReceiveDamage` at the ConditionYellow threshold. Not the same as +0x534. |

### BuildingTypeClass (Type) fields read by branch 4

| Offset | Type | Field | Default | Notes |
|---|---|---|---|---|
| `+0x157B` | byte | `CanBeOccupied` | 0 (no) | Gates branch 4 entry. INI: `CanBeOccupied=yes`. |
| `+0x634` | int | **`TechLevel`** (TechnoTypeClass-inherited) | 255 | -1 = civilian/uncraftable; ≥ 0 = buildable. INI: `TechLevel=`. **This is what was previously labeled "TechnoType factory reference" in earlier docs — that label was wrong.** Verified at `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md:371`. |

### RulesClass (g_RulesClass_Instance) fields

| Offset | Type | Field | Default | INI |
|---|---|---|---|---|
| `+0x1700` | double | `ConditionYellow` | 0.5 | `[AudioVisual] ConditionYellow=50%` |
| `+0x1708` | double | `ConditionRed` | 0.25 | `[AudioVisual] ConditionRed=25%` |

Verified directly: `RulesClass__ReadAudioVisual` at `0x0066B34B-0x0066B372`
calls `CCINIClass::ReadDouble` for `ConditionRed` writing to
`param_1[0x5C2..0x5C3]` (= byte offset `0x1708`) and for `ConditionYellow`
writing to `param_1[0x5C0..0x5C1]` (= byte offset `0x1700`). String pointers
at `0x0083A370` ("ConditionYellow") and `0x0083A380` ("ConditionRed") xref
exclusively from this function.

---

## 3. Core logic

### 3.1 Formula (verified from decompilation at 0x0043EF90)

```
// Pre-conditions: BuildingClass+0x534 != 0 (BState non-zero)
//                 AND Type+0x16BF (LaserFence) == 0
//                 AND Type+0x16C0 (FirestormWall) == 0
//                 AND Type+0x157B (CanBeOccupied) != 0

base = 0
if (GetOccupantCount() > 0):                  // vtable+0x408 → returns this+0x694
    base = 2

health_ratio = ObjectClass::GetHealthRatio()  // = Health / Type+0xA0 (Strength), as double

if (health_ratio <= ConditionRed)             // Rules+0x1708 (0.25)
   OR
   (TechLevel > 0 AND health_ratio <= ConditionYellow):  // TechLevel = Type+0x634
                                                          // ConditionYellow = Rules+0x1700 (0.5)
    base += 1

if (TechLevel == -1 AND base == 3):
    return 1                                  // civilian collapse: occupied+red → frame 1

return base
```

### 3.2 Frame table by (TechLevel, occupancy, health)

For **civilian** garrisons (`TechLevel == -1`, the default for all 95 types
in rulesmd.ini that have `CanBeOccupied=yes`):

| Occupied? | Health | base computation | returned frame |
|---|---|---|---|
| No | > 50% (above yellow) | base=0, +0 | **0** |
| No | 25%–50% (yellow tier) | base=0, +0 (yellow gate `TechLevel>0` fails) | **0** |
| No | ≤ 25% (red tier) | base=0, +1 | **1** |
| Yes | > 50% | base=2, +0 | **2** |
| Yes | 25%–50% | base=2, +0 | **2** |
| Yes | ≤ 25% | base=2, +1 = 3 → collapse | **1** |

For **buildable** garrisons (`TechLevel ≥ 0`, default 255 if INI omits the
key — none of the standard YR garrisonable buildings appear to use this
path, which is a TS-era leftover):

| Occupied? | Health | returned frame |
|---|---|---|
| No | > 50% | 0 |
| No | ≤ 50% | 1 |
| Yes | > 50% | 2 |
| Yes | ≤ 50% | 3 |

**Key observations:**
- The `OR` between red and yellow checks means at most `+1` total. There's
  no "+2" path even when both conditions trip.
- For civilian buildings, the yellow-tier check is gated on `TechLevel > 0`
  and never fires (TechLevel is -1). So civilians have only two health
  buckets visually: above-red and at-red.
- The frame-3 collapse rule means civilian SHPs only need 3 distinct frames
  in practice (0, 1, 2). Some art may have a frame 3 anyway, but the engine
  will never reference it.

### 3.3 BState gating — when does this branch actually run?

Branch 4 sits below the `+0x534 == 0` test. The function structure is:

```
if (Type+0x16BF) return LaserFenceFrame                       // [1]
if (Type+0x16C0) return FirestormWallFrame                    // [2]

if (BuildingClass+0x534 == 0) {                               // [3] BState 0 (idle)
    if (Gate=yes) iVar3 = (Anim1.Start + Anim1.Frames - iVar3) - 1
    if (Mission != 0x13/SELLING) return iVar3                 //   → returns +0xF8
    return ...selling decay formula...
}

// BState != 0 reaches here:
if (Type+0x157B CanBeOccupied) { ...branch 4 formula... }     // [4]
if (Gate=yes) ...                                              // [5]
...normal damaged Anim-max formula...                          // [6]
```

So the CanBeOccupied frame swap only fires when the building's anim BState
is non-zero. BState transitions are committed by `BuildingClass::Update`
(`0x0043FB20`) via the deferred-state path:

```
int next = this->field_0x538;
if (next != -1) {
    if (this->field_0x534 != next) {
        this->field_0x534 = next
        // re-read Anim slot at Type + next*0xC + 0xF04
        // reset CurrentFrame +0xF8 to first_frame of that slot
    }
    this->field_0x538 = -1
}
```

`BuildingClass::ReceiveDamage` (`0x00442230`) sets `IsDamaged` (the bool at
+0x6E6, *not* +0x534) when health crosses ConditionYellow, then calls
`CreateAnimForSlot` which is what eventually queues a BState transition via
`+0x538`. So:

- A building at full HP → BState 0 → branch 3 → returns `+0xF8` (typically 0).
  **Branch 4 is NOT consulted.**
- A building at ≤ ConditionYellow HP → BState eventually transitions to 1 →
  branch 4 fires for CanBeOccupied buildings.

### 3.4 Caveat — visual "garrisoned" effect on healthy buildings

Per the binary, a healthy occupied civilian garrison's body SHP frame
returned by GetCurrentFrame is whatever `+0xF8` is (typically 0) — not 2.
So a healthy garrisoned CABHUT does **not** swap its body SHP frame to the
"occupied" frame via this path.

However, CanBeOccupied buildings have a separate **anim-overlay swap**
mechanism in `FUN_00458330` (called every tick by
`BuildingClass::CheckAutoSellOrCivilian` at `0x00458200` whenever
`Type+0x157B`). For each anim slot that's currently active, the helper
re-images it based on `(occupants > 0) × (health vs ConditionYellow)`,
choosing between three name variants per slot stored in the BuildingType
struct:

```
slot 1 (+0x5A4):  empty/healthy = Type+0x1414, occupied/healthy = Type+0x1434, damaged = Type+0x1424
slot 2 (+0x568):  empty/healthy = Type+0x1018, occupied/healthy = Type+0x1038, damaged = Type+0x1028
slot 3 (+0x56C):  empty/healthy = Type+0x105C, occupied/healthy = Type+0x107C, damaged = Type+0x106C
slot 4 (+0x570):  empty/healthy = Type+0x10A0, occupied/healthy = Type+0x10C0, damaged = Type+0x10B0
slot 5 (+0x574):  empty/healthy = Type+0x10E4, occupied/healthy = Type+0x1104, damaged = Type+0x10F4
```

Each variant slot is 0x20 bytes (32-byte string). These map to art.ini
keys like `ActiveAnim=` / `IdleAnim=` / `ActiveAnimDamaged=` etc., but the
specific INI key → BuildingTypeClass offset mapping for these
occupancy-aware variants needs separate reading of `BuildingTypeClass::ReadINI`.
This anim-overlay system is **out of scope** for the present report — see
Open Questions.

For standard YR civilian garrisons in `artmd.ini` (CABHUT, CABARN01-04,
CAGAS01, CALA01-10, CABUNK01-04), spot-check shows none of these civilian
entries set `ActiveAnim=` / `IdleAnim=`. Their SHPs are static-frame, so
the overlay-swap path is mostly inert for them — implying the body-SHP
frame-swap path (branch 4) is the only mechanism that visibly changes
the building's appearance, and that path only fires when damaged.

If parity of healthy "lit windows" on civilian garrisons is observed in
the original game, the mechanism is either (a) an art.ini-side technique
not represented in the in-repo INIs, (b) an overlay anim configured in
some art file we haven't traced, or (c) the muzzle flashes from
OccupantAnim drawn during fire (which ARE implemented in our engine).
This is flagged as the highest-priority Open Question.

---

## 4. INI keys

| Key | Type | Default | Section | Field | Effect |
|---|---|---|---|---|---|
| `CanBeOccupied` | bool | no | rules.ini per-building | BuildingTypeClass+0x157B | Gates branch 4 entry |
| `TechLevel` | int | 255 | rules.ini per-building | TechnoTypeClass+0x634 | -1 = civilian (collapse rule + skip yellow-tier increment) |
| `MaxNumberOccupants` | int | 0 | rules.ini per-building | BuildingTypeClass+0x1580 | Caps DynamicVectorClass capacity at +0x694; not used by branch 4 directly but bounds occupant count |
| `ConditionYellow` | percent | 50% | `[AudioVisual]` rules.ini | RulesClass+0x1700 | Yellow-tier health threshold (only reached for TechLevel>0 buildings in branch 4) |
| `ConditionRed` | percent | 25% | `[AudioVisual]` rules.ini | RulesClass+0x1708 | Red-tier health threshold (always-on damage trigger) |

For garrisonable list, see the 95 `CanBeOccupied=yes` entries in
`rulesmd.ini` — the standard skirmish set is all map-placed civilian
buildings (TechLevel=-1).

---

## 5. Integration points

**Callers of `GetCurrentFrame` (0x0043EF90):**
- `BuildingClass::DrawBody` (`0x0043D290`) — primary tactical render every frame
- `FUN_0043d030` (`0x0043D030`) — split-pass body draw (Y-clip wrapper around DrawBody)
- `BuildingClass::CreateFoggedSnapshot` (`0x004D0EF0`) — caches frame for the fogged "last seen" snapshot
- `BuildingClass::ReceiveDamage` (`0x00442230`) — pre/post damage frame compare to set the `+0x80` dirty flag
- `BuildingClass::UpdateRepairAndPower` (`0x00450630`) — also reads current frame

**Helpers called from branch 4:**
- `vtable+0x84` → `TechnoClass::GetTechnoType_Trampoline` (`0x006F3270`) → `vtable+0x88` → `*(this+0x520)` (the Type pointer). The trampoline call result is the same as `this->Type` directly, but the binary uses both forms.
- `vtable+0x408` → `BuildingClass::GetOccupantCount` (`0x004581F0`) → returns `*(this+0x694)`.
- `ObjectClass::GetHealthRatio` — returns `(double)Health / (double)Type[+0xA0]`. Health at `this+0x6C`. Type+0xA0 is `Strength` from TechnoTypeClass.
- `vtable+0x184` → `MissionClass::GetCurrentMission` (`0x005B3040`) → returns `*(this+0xAC)` (or `*(this+0xB4)` if Mission == -1). Used in branch 3 to detect SELLING (mission code 0x13).

**When during the tick:** GetCurrentFrame is called from the rendering pass
(Layer 4 buildings in `Tactical_ObjectRenderingLoop`). It's purely a getter
— no state mutation.

---

## 6. Current Rust implementation status

**Body SHP frame selection:**
- File: [src/app_instances/shp.rs:138-139](../src/app_instances/shp.rs#L138-L139)
- Current code: `EntityCategory::Structure => (0, None)` — hardcoded frame 0 for all buildings.
- Missing: every branch of GetCurrentFrame, including the CanBeOccupied frame swap.

**INI parsing (already done):**
- `can_be_occupied` parsed at [src/rules/object_type.rs:842](../src/rules/object_type.rs#L842) → `ObjectType.can_be_occupied: bool`.
- `max_occupants` parsed at [src/rules/object_type.rs:472](../src/rules/object_type.rs#L472).
- `ConditionYellow`/`ConditionRed` parsed at [src/rules/ruleset.rs:597-632](../src/rules/ruleset.rs#L597-L632) → `GeneralRules.condition_yellow: f32`, `condition_red: f32`. Currently used only for fire-overlay spawning and UI health bars, not frame selection.
- `TechLevel` parsing — needs verification: is it currently parsed on `ObjectType`?

**Garrison occupant count (already done):**
- `Cargo.count()` on `GameEntity.passenger_role` returns occupant count for buildings with cargo. Wired correctly through the garrison combat logic at [src/sim/combat/mod.rs:693+](../src/sim/combat/mod.rs#L693).

**Health-ratio computation (partial):**
- Health stored as `Health { current: u16, max: u16 }` at [src/sim/components.rs:87-92](../src/sim/components.rs#L87-L92).
- Ratio is computed ad-hoc in a few places (`current as f32 / max as f32`) but no shared helper exists.

**Damage state / BState transitions:**
- Not implemented. No analog of `+0x534 CurrentAnimState` or `+0x6E6 IsDamaged` in the entity model.

**Net gap to close for body-SHP frame swap:**
1. Parse `TechLevel=` onto `ObjectType` (likely already there as part of TechnoTypeClass parsing — verify).
2. Compute `health_ratio` for entities (helper or inline).
3. In [src/app_instances/shp.rs:138](../src/app_instances/shp.rs#L138), replace the hardcoded `(0, None)` for `Structure` with a call to a new helper that implements the formula above.
4. **Resolved by `GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md`:**
   do not apply the occupied body-frame formula while `BState/+0x534` is zero.
   Healthy idle occupied buildings keep the raw body frame. Full-HP occupied
   visuals, when present, come from active/idle anim-slot variant replacement.

---

## 7. Open questions

1. **Resolved - healthy occupied body frame.** Healthy occupied
   `CanBeOccupied` buildings do not use `GetCurrentFrame` body frame 2 while
   `BState/+0x534` is zero. Full-HP occupied visual changes, when present, are
   active/idle anim-slot variant swaps performed by `FUN_00458330` from
   `CheckAutoSellOrCivilian`.

2. **Resolved - active/idle garrisoned variants.** `ActiveAnimGarrisoned` and
   `IdleAnimGarrisoned` are runtime replacement variants for existing anim
   slots, not extra overlays. Damaged variants override garrisoned variants at
   `ConditionYellow` or below. Stock `artmd.ini` has explicit
   `ActiveAnimGarrisoned=CAWA19_AG`; other types either inherit normal active
   anims or have no active slot.

3. **`Type+0x634` cross-doc mislabels.** Earlier docs (`BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md:524`, `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md:108`) call this field "TechnoType factory reference" / "no primary weapon slot." Both labels are wrong — the field is `TechLevel`. Those docs should be corrected at their next touch.

4. **BState 2+ for CanBeOccupied buildings.** Does any garrisonable building ever transition to BState 2 or higher (e.g., from a production state)? If not, branch 4 only ever fires with BState=1, simplifying the implementation. If yes, we need to ensure BState != 0 doesn't have other side-effects via branch 4 we haven't considered.

5. **Hot-path frequency.** Branch 4 fires every render frame for every visible CanBeOccupied building. With the engine's 30-player / 20k-unit scale target, garrisonable count per match could be in the 30-100 range in late-game; this is a per-frame call. Negligible cost — a ratio compare and a few branches — but worth noting.

---

## Sources

**Ghidra functions decompiled (gamemd.exe):**
- `BuildingClass::GetCurrentFrame` @ `0x0043EF90` (primary)
- `BuildingClass::GetOccupantCount` @ `0x004581F0`
- `TechnoClass::GetTechnoType_Trampoline` @ `0x006F3270`
- `MissionClass::GetCurrentMission` @ `0x005B3040`
- `ObjectClass::GetHealthRatio` (per master report)
- `BuildingClass::SetDamagedState` @ `0x00451EE0`
- `BuildingClass::AddGarrisonOccupant` @ `0x00522910`
- `BuildingClass::CheckAutoSellOrCivilian` @ `0x00458200`
- `FUN_00458330` (anim-overlay swap helper called from CheckAutoSellOrCivilian)
- `BuildingClass::Update` @ `0x0043FB20` (BState commit path + ambient muzzle-flash spawn)
- `BuildingClass::ReceiveDamage` @ `0x00442230` (IsDamaged write at ConditionYellow)
- `RulesClass::ReadAudioVisual` (full body — verified ConditionYellow/Red offsets and string addresses)
- `FUN_00459EE0` (BuildingClass vtable+0x88 = `return *(this+0x520)`)
- `FUN_0043D030` (split-pass body draw wrapper around GetCurrentFrame)

**Memory reads:**
- vtable_BuildingClass @ `0x007E3EBC`: confirmed entries 0x84/0x184/0x408 resolve to GetTechnoType / GetCurrentMission / GetOccupantCount.
- ConditionYellow string @ `0x0083A370`, ConditionRed string @ `0x0083A380` — exclusive xref from `RulesClass::ReadAudioVisual`.

**Docs cross-referenced:**
- `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md` §3.2 (parent doc — formula summary corrected here)
- `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md` (BuildingClass field offsets)
- `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` (BuildingTypeClass offset map)
- `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md` (TechLevel @ +0x634 confirmed)
- `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` (IsDamaged ↔ ConditionYellow transition)
- `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md` (BState anim slot machinery)
- `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` (Phase 16 BState commit path)
- `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md` (occupant count at +0x694)

**INI files checked:**
- `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` — 95 `CanBeOccupied=yes` entries; ConditionYellow=50%, ConditionRed=25% in `[AudioVisual]`
- `c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini` — civilian garrison entries (CABHUT, CALA10, CAMOV02, etc.) verified to lack ActiveAnim/IdleAnim keys

**Rust files referenced:**
- `src/app_instances/shp.rs:138-139` — current building frame selection stub
- `src/rules/ruleset.rs:597-632` — ConditionYellow/Red parsing
- `src/rules/object_type.rs:472,842` — can_be_occupied + max_occupants parsing
- `src/sim/components.rs:87-92` — Health field
- `src/sim/combat/mod.rs:693+` — garrison combat (occupant count consumer)
