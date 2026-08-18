# IFV Mode-Switching & Open-Topped Transport Passenger Weapons — Ghidra Research Report

**Address(es):**
- `0x0070DC70` — `TechnoClass__SetGunnerWeapon(slot)` *(labelled in Ghidra)*
- `0x00710470` — `TechnoClass__SetInOpenTransport(unit)` *(labelled)* — sets `field_0x82 = 1`
- `0x007104A0` — `TechnoClass__ClearInOpenTransport(unit)` *(labelled)* — sets `field_0x82 = 0`
- `0x007104C0` — `CargoClass__ClearAllInOpenTransport()` *(labelled)* — walks cargo list, clears every passenger
- `0x006F3330` — `TechnoClass__SelectWeaponAgainst` (the OpenTransportWeapon override lives at `0x006F3400`)
- `0x007012C0` — `TechnoClass::GetEffectiveMinRange?` (uses passenger min-ranges when OpenTopped)
- `0x00746810` — `UnitClass__InitFromType` (calls SetGunnerWeapon(0) on init when Gunner=yes)
- `0x004733A0` — `CargoClass__AddPassenger` (linked-list insert; **does NOT update IFV slot**)
- `0x00714000+` — `TechnoTypeClass__ReadINI` (parses `IFVMode`, `Gunner`, `OpenTopped`, `OpenTransportWeapon`, `TurretCount`)
- `0x00772080` — `WeaponTypeClass__ReadINI` (parses `FireInTransport=` at +0x143 — but it's never read by anyone)

**Confidence:** HIGH for the IFV slot mechanism, the OpenTransportWeapon path, and the `field_0x82` set/clear functions. HIGH for the dead-key finding on `FireInTransport=`. MEDIUM for the duplicate min-range loop in `0x007012C0`. LOW for the meaning of the special `SetGunnerWeapon(7)` call site at `0x0074647C`.

**Active in YR:** Yes (live). IFV (`[FV]`) uses the Gunner path; Battle Fortress (`[BFRT]`), Allied Nighthawk (`[NHAW]` if OpenTopped), and any other `OpenTopped=yes` transport use the OpenTransportWeapon path.

---

## 1. Overview

YR has **two distinct mechanisms** for "transports whose weapons depend on passengers":

1. **`Gunner=yes` path (IFV / FV)** — the *transport* fires; its current weapon
   slot is overridden by whichever single passenger is inside. Driven by
   `IFVMode=` on the passenger and `Weapon1..N=` on the transport. The
   transport caches `current_slot` and `cached_weapon_ptr` per-instance and
   refreshes them on board / leave events.

2. **`OpenTopped=yes` path (BFRT, etc.)** — the *passengers* fire,
   independently of each other and of the transport, using their own attack
   logic and ROF. Each passenger's `SelectWeaponAgainst` returns
   `OpenTransportWeapon` (an index into its OWN type's weapon list, typically
   0 = Primary or 1 = Secondary). The transport's own weapon (`Primary=`)
   continues to fire as a separate concern. A per-instance flag
   (`TechnoClass+0x82`) on each passenger gates this override; it's set when
   the passenger enters an open-topped transport and cleared when it leaves.

The two systems are **mutually exclusive in INI practice** (`[FV]` is `Gunner=`,
`[BFRT]` is `OpenTopped=`), but nothing in the binary forbids combining them.
The Battle Fortress also has a non-passenger-dependent base weapon
(`Weapon=20mmRapid`) that fires from `PrimaryFireFLH=`, while passengers fire
from `AlternateFLH0..4`.

---

## 2. Class Layout / Key Offsets

### TechnoTypeClass (per-type, set from INI)

| Byte offset | Field | Type | INI key | Notes |
|---|---|---|---|---|
| `+0x5E4` | `OpenTopped` | bool | `OpenTopped=` | Vehicle-side flag enabling passenger fire |
| `+0x688` | `IFVMode` | int | `IFVMode=` | Infantry-side: which IFV slot this infantry occupies (0..16) |
| `+0x805` | `Gunner` | bool | `Gunner=` | Vehicle-side flag enabling IFV slot system |
| `+0x808` | `TurretCount` | int | `TurretCount=` | Visual turret variants (4 for FV → 4 voxel turrets) |
| `+0x80C` | `WeaponCount` | int | `WeaponCount=` | Length of `Weapon1..N` list (17 for FV) |
| `+0x810` | `IsChargeTurret` | bool | `IsChargeTurret=` | Prism-Tank-style charging; when true, gates out the IFV slot setter |
| `+0xD50` | `OpenTransportWeapon` | int | `OpenTransportWeapon=` | Infantry-side: weapon idx (0/1/...) to use when firing from inside open-topped. Default `-1` (sentinel for "not set") |

### TechnoClass (per-instance runtime state)

| Byte offset | Field | Type | Notes |
|---|---|---|---|
| `+0x82` | `is_in_open_transport` | byte | Set to 1 by `SetInOpenTransport`, 0 by `ClearInOpenTransport`. Gates the OpenTransportWeapon override in `SelectWeaponAgainst` |
| `+0x114` | (cargo-related; `LEA EDI, [ESI+0x114]` in cargo-board calls) | various | First passenger pointer or related cargo-list head |
| `+0x124` | `cached_gunner_weapon_ptr` | `WeaponStruct*` | Set by `SetGunnerWeapon` to `WeaponList[slot]`; consumed by general weapon-fire code |
| `+0x138` | `current_gunner_slot` | int | Current IFV slot (0..17). Set by `SetGunnerWeapon(slot)` |

### WeaponTypeClass

| Byte offset | Field | Notes |
|---|---|---|
| `+0x143` | `FireInTransport` | bool | **CORRECTION (third pass): NOT DEAD.** Read exactly once at `0x006FC587` inside `TechnoClass__GetFireError` via `MOV CL, [EDI+0x143]` (the EDI form, which my earlier byte-pattern search missed). If the firer has `field_0x82 != 0` (in open-topped transport) AND the chosen weapon has `FireInTransport == 0`, `GetFireError` returns `FIRE_CANT (0x05)` — denying the fire. Default value is whatever the WeaponType ctor sets (likely `true`, so most weapons CAN fire from inside). |

---

## 3. Core Logic

### 3.1 `SetGunnerWeapon` — the IFV slot setter (`0x0070DC70`)

```c
void TechnoClass::SetGunnerWeapon(this, int slot):
    type = this.vtable.GetType()                    // vtable+0x84
    if (type.IsChargeTurret) return                 // +0x810 gate

    if (slot < 0 || slot >= 0x12) {                 // out of range → fall back to slot 0
        slot = 0
    }
    weapon_ptr = type.WeaponList[slot]              // FUN_007178B0(slot)
    this.current_gunner_slot = slot                 // +0x138
    this.cached_gunner_weapon_ptr = weapon_ptr      // +0x124
```

Slot range is `[0, 17]` (checked as `slot < 0x12`). The fall-back-to-slot-0
is the "no passenger" / "default gunner" weapon.

**Callers (all 5):**
| Caller | Slot arg | Purpose |
|---|---|---|
| `UnitClass__Constructor @ 0x007353C0` | constant 0 | Initial state (corrected 2026-05-28: was `0x73568C`; `0x73568C` is a call site inside the constructor, not the function start; binary shows constructor starts at `0x007353C0` via `decompile_function 0x007353C0`) |
| `UnitClass__InitFromType @ 0x00746810` | constant 0 | After type-init, gated by `Type.Gunner != 0` (corrected 2026-05-28: was `0x7468AF`; verified via `decompile_function 0x00746810`) |
| Function near `0x0074647C` | constant **7** | Mystery (see §7 — possibly Crazy-Ivan-spawn special case?) |
| Function near `0x007464CE` (cargo-board) | `passenger.IFVMode` | When passenger enters Gunner transport, set IFV slot to passenger's IFVMode |
| `UnitClass__Save @ 0x00746520` | (saved value) | Restore on save/load (corrected 2026-05-28: was `0x746596`; verified via `get_function_callers 0x0070DC70` returning `UnitClass__Save @ 00746520`) |

**Important:** `CargoClass__AddPassenger` at `0x004733A0` does NOT call
`SetGunnerWeapon`. The slot update happens in a separate boarding handler
(unlabelled in Ghidra, around `0x007464CE`) that runs alongside the
linked-list insert.

### 3.2 `SetInOpenTransport` / `ClearInOpenTransport` (`0x00710470` / `0x007104A0`)

Trivial flag setters/clearers for the per-instance gate:

```c
SetInOpenTransport(unit):
    if (unit) {
        unit.is_in_open_transport = 1               // +0x82
        unit.vtable[0x3D0](unit)                    // some virtual notification
        // also pushes unit into a global vector at 0x87F778 via FUN_0055BAA0
    }

ClearInOpenTransport(unit):
    if (unit) unit.is_in_open_transport = 0
```

`CargoClass__ClearAllInOpenTransport` (`0x007104C0`) walks the cargo
linked-list (via `+0x30` next-pointer) and clears `+0x82` on every passenger
in one pass:

```c
ClearAllInOpenTransport():
    p = FUN_00473450()                              // get_first_passenger
    while (p && (p.flags & 4)) {                    // flag 4 = "is in cargo"
        p.is_in_open_transport = 0
        p = p.next                                  // +0x30
    }
```

**Callers of `SetInOpenTransport`:** `InfantryClass__PerCellProcess @ 0x00519630`
and `UnitClass__PerCellProcess @ 0x00739EC0` — both fire on the PerCellProcess
path when the unit arrives at its transport's cell. (corrected 2026-05-28:
was `InfantryClass__Mission_Enter` and `UnitClass__Mission_Enter`; binary
`get_function_callers 0x00710470` returns only these two PerCellProcess
functions; `0x005196A0` decompiles as `InfantryClass__PerCellProcess`, not
Mission_Enter — ROOT_CAUSE: RTTI_LABEL_DRIFT). Both call sites are gated on
`OpenTopped` check at `+0x5E4` ✓ — see §7 resolution note below.

**Callers of `ClearInOpenTransport`:** `UnitClass__Mission_Deploy_Building`.
Other clear sites likely exist for the unload path.

**Caller of `ClearAllInOpenTransport`:** `UnitClass__ReceiveDamage` (when
the open-topped transport takes damage — possibly during destruction or area
damage that hits passengers).

### 3.3 `SelectWeaponAgainst` — the OpenTransportWeapon override (`0x006F3400`)

In `TechnoClass__SelectWeaponAgainst @ 0x006F3330`, after the gattling-stage
return path, the function reaches an early-priority block:

```c
piVar5 = this.vtable.GetWeapon(1)                   // GetWeapon(1) — secondary
weapon1 = *piVar5
if (weapon1) {
    piVar5 = this.vtable.GetWeapon(0)               // GetWeapon(0) — primary
    weapon0 = *piVar5
    if (weapon0 && !weapon1.NeverUse                // weapon1.NeverUse at +0x136
                  && target_valid) {
        if (this.is_in_open_transport               // +0x82
            && this.GetType().OpenTransportWeapon != -1) {       // +0xD50
            return this.GetType().OpenTransportWeapon
        }
        // ... other checks (gattling, mind-control, building-firing-port, etc.)
    }
}
```

So **inside an open-topped transport, an infantry's normal weapon-selection
is short-circuited** to return its `OpenTransportWeapon` index directly,
provided both Primary and Secondary exist on its own type.

This drops out of the normal "verses table → primary or secondary" decision.
For example a Guardian GI inside a BFRT will always fire its Secondary
(missile launcher, `OpenTransportWeapon=1`) regardless of target type.

### 3.3a `GetFireError` — the FireInTransport gate (`0x006FC587`)

After `SelectWeaponAgainst` picks a weapon index, `TechnoClass__GetFireError`
at `0x006FC0B0` validates that the firer can actually fire that weapon.
The relevant block:

```c
if (this.field_0x82 != 0) {                          // in open-topped transport
    if (weapon.FireInTransport == 0) {               // weapon+0x143
        return FIRE_CANT  // 0x05
    }
    if (this.MyTransport != 0
        && this.MyTransport.vtable[0x1D4]() != 0) {  // transport "is busy"-ish check
        return FIRE_CANT
    }
}
if (this.field_0x82 != 0
    && this.MyTransport != 0
    && this.MyTransport.MyTransport != 0) {          // nested transports
    return FIRE_CANT
}
```

Three layered conditions:
1. **`FireInTransport=no` blacklist** — the chosen weapon's
   `FireInTransport` flag must be true. Modders use this to mark certain
   weapons as un-fireable from inside (e.g., if a weapon doesn't make sense
   from a passenger position).
2. **Transport-is-busy gate** — `vtable[0x1D4]` on the transport (likely
   "is performing some non-interruptible action") suppresses passenger fire.
3. **Nested-transport gate** — if the open-topped transport is itself
   inside another transport, passengers can't fire (sensible: a BFRT
   loaded into a Carryall shouldn't fire from inside the Carryall).

### 3.4 OpenTopped min-range adjustment (`0x007012C0`)

Helper called by some range/targeting code (caller list returned empty;
probably called via xref in another function we haven't traced). For
`OpenTopped` units, it returns the **minimum** of:
- The transport's own weapon's `MinimumRange` (+0xB4)
- The minimum `MinimumRange` across all passengers' weapons (via
  `vtable[0x3F4]` per-passenger lookup)

Two near-identical loops in the function — likely one for ground-target
weapon and one for air-target weapon. (Confirming this would require
more decompilation.)

The effect: an open-topped transport's effective close-range capability is
extended by whichever passenger has the lowest `MinimumRange`. So a BFRT
loaded with infantry that have melee/short-range weapons effectively has
those passengers' min-range, not the BFRT's chaingun's min-range.

### 3.5 Architecture summary

| Aspect | IFV (Gunner=yes) | OpenTopped (BFRT) |
|---|---|---|
| Who fires? | The transport | The passengers (independently) |
| Whose weapon? | Transport's `Weapon[slot]` (slot from passenger's IFVMode) | Passenger's `Weapon[OpenTransportWeapon]` (passenger's own weapon list) |
| Multi-passenger behavior | One slot at a time (1-passenger transports only — `Passengers=1` for FV) | All passengers fire simultaneously, each at own target with own ROF |
| Per-instance state | Transport caches slot + weapon ptr | Each passenger has `+0x82` flag |
| Triggered when | Passenger boards (cargo handler reads IFVMode) / Init / Load | Passenger enters via Mission_Enter |
| Cleared when | Passenger leaves (slot reset to 0) | Passenger leaves / transport damaged |
| Transport's own weapon | Replaced (no separate "base" weapon while loaded) | Continues to fire (`Weapon=20mmRapid` on BFRT) |
| Visual | Turret swaps via `TurretCount=` voxel variant (one per logical turret type) | No turret swap; passengers fire from `AlternateFLH%d` art hard points |

---

## 4. INI Keys

| Key | Section | Type | Default | Effect |
|---|---|---|---|---|
| `Gunner=` | TechnoType (vehicle) | bool | false (assumed) | Enables IFV slot system. Used by `[FV]`. |
| `IFVMode=` | TechnoType (infantry) | int | 0 | Which IFV slot this infantry triggers (0..16). |
| `TurretCount=` | TechnoType (vehicle) | int | 1 | Number of visual turret voxel variants (FV uses 4: standard / repair / chaingun / hi-tech). |
| `WeaponCount=` | TechnoType (vehicle) | int | 0 | Total weapons in `Weapon1..N` list (17 for FV; 6 for gattling units). |
| `Weapon1=`..`Weapon17=` | TechnoType | id | — | Weapon for IFV slot N–1 (Weapon1 = slot 0 = default gunner; Weapon2 = slot 1 = engineer's repair gun; etc.). |
| `EliteWeapon1=`..`EliteWeapon17=` | TechnoType | id | — | Elite variants. |
| `OpenTopped=` | TechnoType (vehicle) | bool | false | Enables passenger-fire-from-inside. Used by `[BFRT]`, possibly Nighthawk in some configurations. |
| `OpenTransportWeapon=` | TechnoType (infantry) | int | **-1** (sentinel for "not set") | Which weapon idx (Primary=0 / Secondary=1) this infantry uses when firing from inside an open-topped. |
| `FireInTransport=` | WeaponType | bool | true (assumed from ctor) | **CORRECTION: live key.** Used in `TechnoClass__GetFireError`: when firer has `field_0x82 != 0`, the chosen weapon must have `FireInTransport != 0` or `FIRE_CANT` is returned. Modders use this to whitelist which weapons are usable from inside open-topped. |
| `NormalTurretIndex=`, `RepairTurretIndex=`, `MachineGunTurretIndex=`, `FlakTurretIndex=`, `PistolTurretIndex=`, `SniperTurretIndex=`, `ShockTurretIndex=`, `ExplodeTurretIndex=`, `BrainBlastTurretIndex=`, `RadCannonTurretIndex=`, `ChronoTurretIndex=`, `TerroristExplodeTurretIndex=`, `CowTurretIndex=`, `InitiateTurretIndex=`, `VirusTurretIndex=`, `YuriPrimeTurretIndex=`, `GuardianTurretIndex=` | TechnoType (vehicle) | int | 0 (assumed) | **17 per-vehicle keys**, one per IFVMode slot (0..16). Each maps an IFVMode infantry to a voxel turret index in `[0, TurretCount-1]`. Read by render path to pick which turret variant to draw when a given passenger occupies the IFV. |
| `IsChargeTurret=` | TechnoType (vehicle) | bool | false | Prism-Tank-style charging mechanism. **Gates out** the `SetGunnerWeapon` slot setter — IsChargeTurret + Gunner is incompatible. |
| `OpenToppedDamageMultiplier=` | RulesClass `[CombatDamage]` | double | 1.0 | Multiplier applied to damage dealt by passengers firing from inside (per `RulesClass+0xF58`). Confirmed in GARRISON_SYSTEM doc. |
| `OpenToppedRangeBonus=` | RulesClass `[CombatDamage]` | int | 0 | Range bonus added to passengers firing from inside (per `RulesClass+0xF5C`). |
| `OpenToppedAnim=` | WeaponType | string | — | Muzzle anim override when firing from inside open-topped. (Parsed at WeaponType+0x118 — consumer not yet traced.) |
| `AlternateFLH0..4=` | art.ini (vehicle voxel) | coords | — | Fire-location hard points for the up-to-5 passenger firing ports on `[BFRT]`. |
| `Weapon1FLH..Weapon17FLH=` | art.ini (vehicle voxel) | coords | — | Per-slot fire-location hard points for IFV (one per turret/weapon mode). |

YR retail values for `[FV]`:
```
Gunner=yes  TurretCount=4  WeaponCount=17
Passengers=1  SizeLimit=1  Turret=yes
Weapon1=HoverMissile (default — no passenger / GI)
Weapon2=RepairBullet (Engineer)
Weapon3=CRM60 (GI rocket — IFVMode=2)
Weapon4=CRFlakGuyGun (Flak Trooper — IFVMode=3)
... 17 total ...
```

YR retail values for `[BFRT]`:
```
OpenTopped=yes  Passengers=5  SizeLimit=2
Weapon=20mmRapid  PipScale=Passengers
PrimaryFireFLH=220,0,130
AlternateFLH0..4 = (5 passenger firing ports)
```

Per-infantry `OpenTransportWeapon=`:
```
[E1] OpenTransportWeapon=0       (GI fires Primary from BFRT)
[GHOST] OpenTransportWeapon=0    (SEAL fires Primary)
[TANY] OpenTransportWeapon=0     (Tanya fires Primary)
[BORIS] OpenTransportWeapon=0    (Boris fires Primary)
[GGI] OpenTransportWeapon=1      (Guardian GI fires Secondary — missile launcher)
[YURIPR] OpenTransportWeapon=1   (Yuri Prime fires Secondary)
```

Most infantry default to no `OpenTransportWeapon=` set → `-1` → cannot fire
from inside.

---

## 5. Integration Points

**Read by:**
- `TechnoClass__SelectWeaponAgainst` (the central weapon-index router; both IFV and OpenTransportWeapon paths short-circuit it).
- `TechnoClass__SetGunnerWeapon` writes the cache; the cache (`+0x124`, `+0x138`) is consumed by general weapon-fire code (cached pointer avoids repeated GetWeapon dispatch).
- `FUN_007012C0` — extends min-range using passenger min-ranges for OpenTopped.
- `UnitClass__ReceiveDamage` — uses `OpenTopped` flag to clear all passengers' fire flag (likely on transport destruction).

**Tick ordering:**
- `SetGunnerWeapon` runs at boarding / unboarding events, not per tick — the
  cached weapon pointer makes this a one-shot update.
- `SelectWeaponAgainst` is called per-fire-attempt during Mission_Attack
  (which itself runs per-frame for units, per-mission-rate for buildings).
- The `is_in_open_transport` flag is flipped on Mission_Enter / Mission_Deploy
  events, so it's stable for the duration of a passenger's stay.

**Control flow (IFV passenger boarding):**
1. Player commands infantry to enter IFV → `Mission_Enter` runs on infantry.
2. When close enough, infantry is removed from map (added to limbo) and
   appended to IFV's cargo linked-list via `CargoClass__AddPassenger`.
3. **In a separate handler** (around `0x007464CE`), the IFV reads
   `passenger.IFVMode` from the new passenger's TypeClass and calls
   `SetGunnerWeapon(IFVMode)` on itself.
4. IFV's turret graphic + active weapon both update.
5. On unload: passenger is removed from cargo list; IFV calls
   `SetGunnerWeapon(0)` to revert to default gunner weapon.

**Control flow (BFRT passenger boarding + firing):**
1. Player commands infantry to enter BFRT → `Mission_Enter` runs on infantry.
2. When close enough, infantry is added to BFRT's cargo list AND
   `SetInOpenTransport(infantry)` is called → `infantry.field_0x82 = 1`.
3. Each tick, infantry's own AI / Mission_Attack runs. On any
   `SelectWeaponAgainst` call, the OpenTransportWeapon override returns the
   pre-set index from infantry's TypeClass.
4. The infantry's weapon fires from the BFRT's location (presumably picking
   one of the `AlternateFLH0..4` hard points; selection logic not yet
   traced).
5. On unload: infantry is removed from cargo, `ClearInOpenTransport` runs,
   `field_0x82 = 0`, override is no longer applied.

---

## 6. Current Rust Implementation Status

**IFV / Gunner path: MOSTLY IMPLEMENTED** (per Rust-impl scan)
- [src/sim/game_entity.rs:187](../ra2-rust-game/src/sim/game_entity.rs#L187) — `ifv_weapon_index: Option<u32>` (matches binary's `+0x138`).
- [src/sim/passenger.rs:382](../ra2-rust-game/src/sim/passenger.rs#L382) — sets index when passenger boards Gunner transport.
- [src/sim/passenger.rs:554](../ra2-rust-game/src/sim/passenger.rs#L554) — clears index when all passengers exit.
- [src/sim/combat/combat_weapon.rs:79-118](../ra2-rust-game/src/sim/combat/combat_weapon.rs#L79-L118) — `select_weapon_with_ifv()` consumes the index.
- [src/sim/combat/mod.rs:985](../ra2-rust-game/src/sim/combat/mod.rs#L985) — wired into combat tick.

**Differences from binary:**
- Rust caches the **index** only; binary also caches the **weapon pointer** at `+0x124`. Cosmetic / perf detail.
- Rust does NOT model the `IsChargeTurret` exclusion. Worth adding a guard: if `is_charge_turret`, ignore IFV updates (a Prism Tank with `Gunner=yes` would be misbehaving in our impl).
- Rust does NOT cover the mystery `SetGunnerWeapon(7)` call site at `0x0074647C` — but neither does YR INI seem to depend on it.

**OpenTopped / BFRT path: NOT IMPLEMENTED (only stubs).**
- [src/rules/object_type.rs:437-438](../ra2-rust-game/src/rules/object_type.rs#L437-L438) — `open_topped: bool` parsed but never consumed.
- [src/rules/weapon_type.rs:81](../ra2-rust-game/src/rules/weapon_type.rs#L81) — `open_topped_anim: Option<String>` parsed.
- [src/rules/ruleset.rs:487-489](../ra2-rust-game/src/rules/ruleset.rs#L487-L489) — `open_topped_damage_multiplier`, `open_topped_range_bonus` parsed.
- No `OpenTransportWeapon` parsing on InfantryType.
- No `is_in_open_transport` flag on entity.
- No code that lets a passenger fire while in cargo.

**Minimum viable BFRT plug-in plan** (for a future implementation conversation):
1. Parse `OpenTransportWeapon=` (int, default `-1`) on InfantryType.
2. Parse `FireInTransport=` (bool, default `true`) on WeaponType.
3. Parse `AlternateFLH0..4=` (5 × XYZ) on UnitType.
4. Add `is_in_open_transport: bool` to `GameEntity`.
5. In passenger Mission_Enter / boarding code: if target transport's
   `open_topped`, set passenger's `is_in_open_transport = true`. Clear on
   unload / transport death.
6. In `select_weapon_*`: if `is_in_open_transport && open_transport_weapon != -1`,
   return `open_transport_weapon` directly, skipping verses logic.
7. In `can_fire` / fire-error code: if `is_in_open_transport`, deny fire
   when chosen weapon's `fire_in_transport == false`, OR when the transport
   is itself in another transport.
8. While `is_in_open_transport`, the passenger's normal Mission_Attack
   should still tick (own target acquisition, own ROF), but use the
   transport's coordinates for fire-location and the transport's
   `AlternateFLH%d[passenger_slot_index]` for muzzle position.
9. Apply `OpenToppedDamageMultiplier` / `OpenToppedRangeBonus` from rules.
10. The transport's OWN weapon fire is unchanged — it's a separate concern.

**For IFV, also:**
- Parse the 17 `*TurretIndex=` keys on UnitType (NormalTurretIndex,
  RepairTurretIndex, etc.).
- In the IFV draw path: when picking the turret voxel, look up the keyed
  TurretIndex for the current `gunner_slot` value and render that turret
  variant.

---

## 7. Open Questions

**Resolved in second follow-up pass (2026-04-19):**

- ~~Mission_Enter call sites for `SetInOpenTransport`~~ — **CONFIRMED gated.
  CORRECTED 2026-05-28: the function is `InfantryClass__PerCellProcess @ 0x00519630`,
  NOT `Mission_Enter`. Address `0x005196A0` decompiles as `InfantryClass__PerCellProcess`
  (ROOT_CAUSE: RTTI_LABEL_DRIFT). The call site is:**
  ```c
  iVar3 = (**(code **)(*piVar10 + 0x84))();       // transport.GetType()
  if (*(char *)(iVar3 + 0x5e4) != '\0') {         // OpenTopped check (+0x5E4)
      TechnoClass__SetInOpenTransport(param_1);   // only set if OpenTopped
  }
  param_1[0x47] = (int)piVar10;                   // store transport ref at +0x11C
  ```
  The flag is correctly only set when entering an `OpenTopped=yes` transport.
  `UnitClass__PerCellProcess @ 0x00739EC0` has the identical pattern (verified
  via `decompile_function 0x00739EC0`).

- ~~Duplicate min-range loop in `FUN_007012C0`~~ — **CONFIRMED identical.**
  Re-read the decompilation: both loops call the same `vtable[0x3F4]` with
  no args, walk the same cargo linked-list, compute the min the same way.
  Functionally redundant — either a Westwood code-duplication bug or the
  second loop was originally intended to do something different (AA weapon?
  different filter?) and never updated. Real impact: the second loop wastes
  cycles re-computing a value already known. Doesn't affect correctness.

**Resolved in third follow-up pass (2026-04-19):**

- ~~`FireInTransport=` is dead~~ — **CORRECTED — NOT DEAD.** Read at
  `0x006FC587` in `TechnoClass__GetFireError` via the `MOV CL, [EDI+0x143]`
  encoding (`8A 8F 43 01 00 00`). My original byte-pattern search missed
  the EDI form. The semantics: passengers in open-topped transports
  (`field_0x82 != 0`) can ONLY fire weapons whose `FireInTransport != 0`.
  If a weapon has `FireInTransport=no`, `GetFireError` returns `FIRE_CANT
  (0x05)` and the fire is suppressed. Default value (from ctor) is
  presumably `true` — most weapons can fire from inside. Modders use this
  to whitelist/blacklist specific weapons for OpenTopped behavior. **This
  changes the implementation plan in §6** — the Rust port needs to read
  `FireInTransport=` and gate the OpenTransportWeapon path on it.

- ~~TurretCount → turret-voxel mapping~~ — **RESOLVED.** The mapping is
  expressed as **17 explicit per-UnitType INI keys**, all parsed in
  `UnitTypeClass__ReadINI` (xrefs to the strings at `0x008459D4..0x00845C80`):
  `NormalTurretIndex`, `RepairTurretIndex`, `MachineGunTurretIndex`,
  `FlakTurretIndex`, `PistolTurretIndex`, `SniperTurretIndex`,
  `ShockTurretIndex`, `ExplodeTurretIndex`, `BrainBlastTurretIndex`,
  `RadCannonTurretIndex`, `ChronoTurretIndex`, `TerroristExplodeTurretIndex`,
  `CowTurretIndex`, `InitiateTurretIndex`, `VirusTurretIndex`,
  `YuriPrimeTurretIndex`, `GuardianTurretIndex` — one per IFVMode slot
  (0..16). Each maps that IFVMode to a turret-voxel index in `[0,
  TurretCount-1]`. So when `[FV] TurretCount=4` and an Engineer (IFVMode=1)
  boards, the IFV draw path looks up `[FV] RepairTurretIndex=2` and
  renders voxel turret #2. **Per-vehicle**, not global — different
  `Gunner=yes` vehicles can map slots to different voxels. Render-side
  consumer not traced (out of scope for combat investigation).

**Still open (lower confidence / lower priority):**

1. **`SetGunnerWeapon(7)` mystery site** at `0x0074647C`. Inspecting the
   surrounding asm: it's in a function that handles a **passenger transfer
   between two transports** (writes `[ESI+0x274] = EBX` and `[EBX+0x24] = ESI`,
   bidirectional re-link). When the OLD passenger is moved to the NEW
   transport, `SetGunnerWeapon(slot=7)` is called on the new transport
   (slot 7 = Crazy Ivan IFV slot). The call argument really IS `7`, not
   `passenger.IFVMode`. **Most likely a Westwood bug or undocumented
   special-case** — the developers either intended to push the passenger's
   IFVMode but hardcoded 7, or 7 has a special meaning here. Worth a quick
   in-game test to see what happens when an IFV passenger is transferred
   between two IFVs (if that's even possible).

2. **`ClearInOpenTransport` for the normal unload path** — **NOT FOUND.**
   - `TechnoClass__ClearInOpenTransport @ 0x007104A0` is **only called from
     `UnitClass__Mission_Deploy_Building`** (the MCV-deploy path).
   - `CargoClass__ClearAllInOpenTransport @ 0x007104C0` is **only called
     from `UnitClass__ReceiveDamage`** (transport destruction).
   - `UnitClass__Mission_Unload @ 0x00740EF0` decompiled — does NOT call
     either clear function. It dispatches via `vtable[0x528]` (find unload
     cell) and `vtable[0x278](2, target)` (set MISSION_MOVE on the unloaded
     passenger), but does not touch `field_0x82`.
   - `FootClass__Unlimbo @ 0x004D7170` — also does not touch `field_0x82`.
   - Binary-wide byte-pattern search for `MOV byte ptr [reg+0x82], 0`
     finds **only the `ClearInOpenTransport` site** plus one false positive
     in `TeamClass__Set_Convoy_Target` (different struct, coincidental
     offset).
   - **This appears to be a Westwood bug:** passengers exited from a BFRT
     via normal unload retain `field_0x82 = 1`. After they're back on the
     map, their `SelectWeaponAgainst` would still see the override
     condition (flag set + OpenTransportWeapon != -1) and return
     OpenTransportWeapon as their weapon idx — possibly causing them to
     fire the "wrong" weapon (e.g., a Guardian GI dropped from BFRT might
     keep firing its missile launcher instead of the regular GI rifle).
     **Needs in-game verification** before claiming it as a bug — there
     could be a clear path via a vtable call we didn't trace, or the
     surrounding logic happens to mask the effect.

3. **Firing-port FLH selection.** `AlternateFLH0..4` parsed at
   `0x715FAF` as 5 × 3 ints stored at TechnoTypeClass+0x85C..+0x894 (15
   ints, 5 slots). Consumer (per-passenger fire-position lookup) not traced.
   Likely happens in `InfantryClass::Fire_At` or `TechnoClass::Get_Fire_Position`
   when the firer is `is_in_open_transport`. The mapping from passenger
   to slot index is presumably by cargo-list order (first passenger →
   AlternateFLH0, second → AlternateFLH1, etc.).

4. **`OpenToppedAnim=` consumer.** Parsed at WeaponType+0x118 but consumer
   not yet traced. Many false positives in byte-pattern search because the
   passenger linked-list head on TechnoClass is also at byte +0x118
   (different struct). Would require per-function disambiguation.

5. **TurretCount → turret-voxel selection.** `TurretCount=4` on `[FV]`
   means 4 voxel turret variants, but the IFV slot is in `[0,17]`. There
   must be a slot→turret-index mapping somewhere. Per the INI doc, several
   slots reuse the same turret (all "exotic" infantry use turret index 3).
   Likely handled in the voxel draw path — out of scope for the combat
   investigation. Worth its own `/re-investigate` if visual fidelity
   matters.

6. ~~**`FireInTransport=` is dead.**~~ **CORRECTED — NOT DEAD. See §3.3a.**
   (corrected 2026-05-28: this item is stale and contradicts the §3.3a
   correction already in the document body — ROOT_CAUSE: INFERENCE_HARDENED.
   The key IS live: read at `0x006FC587` in `TechnoClass__GetFireError` via
   `*(char *)(iVar4 + 0x143)`, confirmed by `decompile_function 0x006FC0B0`.)

---

## Sources

**Ghidra functions decompiled / inspected:**
- `0x0070DC70` `TechnoClass__SetGunnerWeapon` *(labelled this session)*
- `0x00710470` `TechnoClass__SetInOpenTransport` *(labelled this session)*
- `0x007104A0` `TechnoClass__ClearInOpenTransport` *(labelled this session)*
- `0x007104C0` `CargoClass__ClearAllInOpenTransport` *(labelled this session)*
- `0x006F3330` `TechnoClass__SelectWeaponAgainst` (re-read for OpenTransportWeapon path)
- `0x007012C0` (`FUN_007012C0`) — OpenTopped min-range helper
- `0x004733A0` `CargoClass__AddPassenger` (linked-list insert; no IFV update)
- `0x00746810` `UnitClass__InitFromType` (calls SetGunnerWeapon(0) when Gunner)
- `0x00714000+` `TechnoTypeClass__ReadINI` (string xrefs at `0x71401D..0x714E5C`)
- `0x00772080` `WeaponTypeClass__ReadINI` (FireInTransport at WeaponType+0x143)

**Binary-wide byte-pattern searches:**
- `MOV CL, [reg+0x5E4]` (OpenTopped) — 14 hits across combat/AI/damage code
- `MOV CL, [reg+0x805]` (Gunner) — 7 hits
- `MOV reg, [reg+0xD50]` (OpenTransportWeapon) — 2 hits (one in SelectWeaponAgainst)
- `MOV reg, [reg+0x688]` (IFVMode) — 1 hit (in cargo-board handler near `0x007464CE`)
- `MOV byte ptr [ESI+0x82], 1` — 1 hit (the SetInOpenTransport function itself)
- `MOV/MOVZX/CMP byte ptr [reg+0x143]` (FireInTransport) — **zero hits → dead key**

**Related docs in `ra2-rust-game-docs/`:**
- `GARRISON_SYSTEM_GHIDRA_REPORT.md` — building occupant system; provides architectural pattern reference and the `OpenToppedDamageMultiplier` / `OpenToppedRangeBonus` rules offsets.
- `TECHNOCLASS_SYSTEMS_GHIDRA_REPORT.md` §10 — passenger / transport struct layout, IFVMode noted but not behavior.
- `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` — weapon offsets including `+0x143 FireInTransport` and `+0x118 OpenToppedAnim`.
- `GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md` — sister report; covers another per-instance weapon-index override system on the same `vtable[0x3F8] GetWeapon` path.

**INI files checked:**
- `ini/rulesmd.ini` — `[FV]`, `[BFRT]`, all `IFVMode=` infantry, all `OpenTransportWeapon=` infantry
- `ini/artmd.ini` — IFV `Weapon1FLH..17FLH`, BFRT `AlternateFLH0..4`
- `ini/rules.ini` / `ini/art.ini` — confirmed YR-only system (no IFV in base RA2)
