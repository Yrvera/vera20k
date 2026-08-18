# Can-Target Gates — `GetFireError` Inventory

This doc is the canonical reference for the **target-eligibility gate set** in
gamemd.exe. Every check that decides whether a weapon can fire at a candidate
target lives here.

The primary dispatcher is **`TechnoClass::GetFireError`** at `0x006FC0B0`.
Subclass overrides exist for `BuildingClass` at `0x00447F10`. Both return a
`FireError` code (an integer in 0..9 mapping to "OK / blocked-because-...").

Out-of-scope:
- The **range** check inside the FireError pipeline → [`range_min_max.md`](range_min_max.md)
- The **AA dispatch** (primary/secondary swap based on target locomotor + IsAntiAir) → [`anti_air_dispatch.md`](anti_air_dispatch.md)
- The damage transform after firing → [`damage_formula.md`](damage_formula.md)
- ROF/Burst cadence → [`rof_burst_timing.md`](rof_burst_timing.md)
- Veterancy weapon swap → [`veterancy_weapon_swap.md`](veterancy_weapon_swap.md)

---

## 1. Function identity

| Field | Value |
|---|---|
| Address | `0x006FC0B0` (`TechnoClass::GetFireError`) |
| Subclass override | `0x00447F10` (`BuildingClass::GetFireError`) |
| Calling convention | `__thiscall(this: TechnoClass*, target: AbstractClass*, weaponIndex: int /* on stack */, _: bool /* on stack */) → char (FireError code)` |
| Caller (live, partial) | `UnitClass::Fire_At_Target @ 0x00736df0` — dispatch is mostly via `vtable+0x314` (an indirect entry) so the `get_function_callers` index returns only 1 site; the actual call sites are at every per-tick `Mission_Attack` handler (UnitClass / InfantryClass / BuildingClass / AircraftClass). |

### Building wrapper structure

```
BuildingClass::GetFireError(this, target, weaponIdx, _):
    if BuildingType.byte+0x157B != 0:                       // 'PowerUp'-only flag
        if BuildingType.byte+0x157C == 0:                   // not currently powered
            return 5
        if this.vtable+0x408() == 0:                        // GetBarrelCount/Power check
            return 5
    if !TechnoClass::IsDeploying(this):
        if BuildingType.byte+0x16C3 != 0:                   // 'CanFire' override flag
            return 6
        anim_state = this.vtable+0x184()                    // current anim state
        if anim_state != 0x12 && anim_state != 0x13:        // not in fire-start or fire-active
            if !this.vtable+0x350():                         // CanFireFromCurrentState
                return 6
            if this.field_0x1C5 == 0:                        // not in cooldown
                err = TechnoClass::GetFireError(this, target, weaponIdx, _)
                if err == 0 && this.vtable+0x3FC():          // power-gated rate-limit
                    rearmDelay = vtable+0x4E8(weaponIdx)
                    minDelay = (PowerDoubleRearm ? 0x800 : 0x800)  // 8s default
                    elapsed = currentFrame - lastFireFrame
                    if elapsed < minDelay: return 2          // REARM
                return err
            return 3
    return 5
```

So a building's GetFireError applies a few **building-specific** gates (power, deploy
state, anim state, rearm rate-limit) and otherwise delegates to TechnoClass.

### Confidence

- **Content: HIGH** for the dispatcher structure (both functions decompiled live 2026-05-17).
- **Identity: HIGH** — both functions are named in the Ghidra annotation set.
- **Binding: MEDIUM** — the function is dispatched via `vtable+0x314` from every per-tick `Mission_Attack`/`Fire_At_Target` handler, but `get_function_callers` reports only 1 caller because the dispatch is vtable-indirect. Caller pattern is well-known from the existing `FIRE_AT_ANALYSIS.md` doc.

---

## 2. FireError code mapping (inferred from return values and context)

| Code | Likely semantic | When returned |
|---:|---|---|
| `0` | **OK** — fire allowed | end of function, all gates passed |
| `1` | **AMMO** — out of ammo | `this.Ammo == 0` |
| `2` | **REARM** — building power-gated rearm wait | building wrapper |
| `3` | **BUSY / ROTATING** — mission/anim/locomotor not ready | many places (mission state, fire timer, particle active, etc.) |
| `5` | **ILLEGAL** — target ineligible | by far the most common reject code |
| `6` | **CANT / no-valid-weapon** | weapon=NULL, target requires AA and weapon is AG, etc. |
| `8` | **MUST-DEPLOY** | `TechnoType.byte+0xD27` set ("must deploy to fire"); also the `vtable+0x3A8` final check |
| `9` | **CLOAKED** | attacker is stealth-firing-blocked (`weapon.byte+0x133 != 0`) |

Notes on `5` vs `6`:
- `5` is "this target is not a valid target for this weapon" (cell-flag, friendly, immune, locomotor-incompat, etc.).
- `6` is "this weapon's setup is unusable here" (weapon=NULL, in flight while not allowed, particle attached, etc.).

Notes on code `4`/`7` not seen:
- Standard Westwood TS/RA2 enum has 9-10 codes; the values not used by this function may be reserved or used by sibling subclasses. Open follow-up: enumerate the full enum from the FireError consumer side (`Fire_At_Target` handlers).

---

## 3. Gate inventory (all gates in dispatch order)

The function reads many fields. Below is the full gate sequence as decompiled.
Format: **[order] return-code — condition — fields tested**.

### Phase A — Target sanity & attacker state

| # | Code | Condition |
|---:|---:|---|
| 1 | `5` | `target == NULL` |
| 2 | `5` | `attacker.field_0x2DC != 0` — likely "spawned but not ready" or "in pre-deploy state" |
| 3 | `3` | `attacker.vtable+0x1D8()` true — attacker is rotating/aiming-not-yet-ready |
| 4 | `3` | `attacker.LocomotorTarget == target` — already moving to attack this target (re-aim in progress) |
| 5 | `5` | `attacker.vtable+0x1D4()` true — attacker is in a "no-fire" locomotion state |
| 6 | `5` | `attacker.field_0x1C8 != 0` — mission lock (BUSY mission step) |
| 7 | `5` | `attacker.IsSinking != 0` — ship sinking |
| 8 | `5` | `target == attacker.field_0x1CC` — "do not re-fire at this object" (some no-retarget flag) |
| 9 | `5` | `target == attacker.field_0x11C` — target is attacker's transport/bunker host (cannot fire at own transport) |
| 10 | `3` | `attacker.vtable+0x1DC()` and `attacker.field_0x274 != 0` and `target == *(field_0x274 + 0x28)` — already engaged via sniper-like state |
| 11 | `5` | `attacker.field_0x14 & 4` AND `attacker[1].field_0x18D != 0` — combination flag; appears in InfantryClass-specific check (NoTrans? DeathHandWeapon?) |

### Phase B — Target is a Techno (gated on target.IsTechno = bit 0x1 of target+5)

The `piVar7` variable is set to `target` if `target` IS a Techno, NULL otherwise:
```
piVar7 = ((target.byte+5 & 1) != 0) ? target : NULL
```

| # | Code | Condition (only when target is Techno) |
|---:|---:|---|
| 12 | `5` | `target.InLimbo != 0` (target+0x81) — target is in limbo |
| 13 | `5` | `attacker.field_0x298 != 0` AND `target.Type.byte+0x690 != 0` — "TS-era special anti-target flag", details unclear |
| 14 | `5` | `attacker.Type.byte+0x693 != 0` AND `target.Type.byte+0x694 != 0` — paired "cannot target X" flags |
| 15 | `5` | `target[0x9F] != 0` — some bool flag on target instance (probably IsCloaked-but-not-detected) |
| 16 | `5` | `attacker.Owner.byte+0x1EC == 0` AND `target.vtable+0x160()` (IsOnBridge) true — non-bridge-aware AI can't fire at bridge targets |
| 17 | `6` | `target.vtable+0x68(1, attacker.Owner) == 5` AND sensor-by-attacker-house is false AND attacker has nonzero range AND `!IsAllied(target.House, attacker.House)` — i.e., target is sensor-stealthed and we can't see it via friendly sensors |
| 18 | `5` | `target[0x106] != 0` AND `target.Type.byte+0xD6A != 0` AND `target.vtable+0x54() == 0` — some special "target-not-airborne-and-flagged-X" rule |

### Phase C — General attacker state

| # | Code | Condition |
|---:|---:|---|
| 19 | `5` | `attacker.field_0x8D != 0` — general "cannot fire" flag |
| 20 | `6` | `attacker.vtable+0x37C()` true AND `attacker.WhatAmI() != 1 (UnitClass)` — special non-unit attack-restriction; UnitClass has a passthrough but only if specific InfantryType flags `+0xE18/0xE19` set |
| 21 | `6` | `weapon == NULL` (`vtable+0x3F8` returned 0) |

### Phase D — Sticky-beam particle active block

The four "sticky beam" flags on the weapon disable re-firing while the corresponding
particle system on the attacker is still active. Each block returns code `3` (BUSY).

| # | Weapon flag | Attacker field | Code |
|---:|---|---|---:|
| 22 | `weapon.UseSparkParticles (+0x129)` | `+0x304` (sparkParticleSys) | `3` |
| 23 | `weapon.IsRailgun (+0x12D)` | `+0x314` (railgunParticleSys) | `3` |
| 24 | `weapon.UseFireParticles (+0x12A)` | `+0x308` (fireParticleSys) | `3` |
| 25 | `weapon.IsSonic (+0x130)` | `+0x???` (Wave pointer) | `3` |

These prevent overlapping particle effects from the same firer.

### Phase E — Lightning Storm world block

| # | Code | Condition |
|---:|---:|---|
| 26 | `6` | `weapon.byte+0x14F != 0` AND `FUN_0053a130()` returns true — likely "Lightning Storm active OR strike imminent" gate blocking firing during the storm |

### Phase F — Naval-vs-target sanity (Branch G in some Westwood docs)

| # | Code | Condition |
|---:|---:|---|
| 27 | `5` | `weapon.byte+0x142 != 0` ("NavalTargeting=") AND target.byte+0x74 (IsAirborne) — target is airborne but weapon can't target air-from-naval |
| 28 | `5` | `weapon.byte+0x142 != 0` AND target.Type.byte+0x5EF == 0 — target's type doesn't permit naval-targeting |

### Phase G — Anti-bunker-occupant check

| # | Code | Condition |
|---:|---:|---|
| 29 | `5` | `attacker.WhatAmI() == 1 (UnitClass)` AND `FUN_00746db0()` true — "this unit cannot fire because of crawl/transport rules"; details in `UnitClass` helper |

### Phase H — Warhead Psychedelic / MindControl / IvanBomb / BombDisarm gates

These read `weapon.Warhead` (`weapon+0xAC`).

| # | Code | Condition |
|---:|---:|---|
| 30 | `5` | `warhead.Psychedelic (+0x16D) != 0` AND target.Type.ImmuneToPsionics (`+0xD35`) — psy can't affect immune targets |
| 31 | `5` | `warhead.Psychedelic != 0` AND `target.field_0x2E4 != 0` — target is in a bunker (psy doesn't reach inside bunkers) |
| 32 | `5` | `warhead.IsLocomotor (+0x15B) != 0` AND target.WhatAmI() == 1 (UnitClass) AND target type has the "no-Magnetron" flag (`FUN_00746db0` true) — Magnetron can't lift this type |
| 33 | `5` | `warhead.IsLocomotor != 0` AND target IsTechno AND target.Type.byte+0xD94 != 0 — secondary "Magnetron-immune" type-flag, with extra weapon-spawn check via attacker `vtable+0x80` |
| 34 | `5` | `warhead.IsLocomotor != 0` AND target.WhatAmI() == 1 AND target type byte+0xE13 != 0 AND target.warhead.Locomotor == 0x10 — anti-double-Magnetron-lift |
| 35 | `5` | `warhead.IsLocomotor != 0` AND target.Type.byte+0xD97 != 0 — third "can't be Magnetron'd" flag |
| 36 | `5` | `attacker.field_0x82 (IsOpenTopped) != 0` AND `weapon.byte+0x143 (?) == 0` — open-topped attackers can only use weapons flagged `+0x143` |
| 37 | `5` | `attacker.IsOpenTopped != 0` AND `attacker.Transport != NULL` AND `attacker.Transport.vtable+0x1D4()` true — passenger of a transport in a "no-fire" loco state |
| 38 | `5` | `attacker.IsOpenTopped != 0` AND `attacker.Transport != NULL` AND `Transport.field_0x11C != 0` — transport has its own transport (nested) |
| 39 | `5` | target IsTechno AND `attacker.LocomotorTarget != NULL` AND `target.vtable+0x1D4()` true AND `warhead.IsLocomotor == 0` — target locomotor blocks non-Magnetron fire |

### Phase I — Spawner weapon (LimboLaunch) gates

`weapon.byte+0x131` is the LimboLaunch / spawner-weapon flag.

| # | Code | Condition |
|---:|---:|---|
| 40 | `6` | `weapon.LimboLaunch (+0x131) != 0` AND `TechnoClass::IsOnBridge_ForFiring(attacker)` — can't launch from bridges |
| 41 | `6` | `weapon.LimboLaunch != 0` AND `attacker.vtable+0x380()` true — attacker is in a state preventing spawner launch |
| 42 | `3` | `weapon.LimboLaunch != 0` AND `SpawnManagerClass.CountAliveSpawns() == 0` — all spawn slots in cooldown |

### Phase J — Naval-targeting target-type gate

| # | Code | Condition |
|---:|---:|---|
| 43 | `5` | `warhead.NavalGunboat (+0x15A) != 0` AND target IsTechno AND target.WhatAmI() == 2 (AircraftClass) AND target.Type.byte+0xD54 != 0 — naval gunboat can't target this specific aircraft type |

### Phase K — Sticky-beam still-active (re-checked after weapon load)

Same gates as Phase D, with a slightly different weapon pointer source — covers the
case where weapon was re-fetched after intermediate vtable calls.

### Phase L — Anti-Air dispatch

| # | Code | Condition |
|---:|---:|---|
| 44 | `3` (with locomotor mode) | target.IsAirborne (vtable+0x54) true AND `weapon.Projectile.AA (+0x2A4) == 0` — projectile is not AA-capable for this air target; return depends on whether attacker.LocomotorTarget == target (the locomotor-mode call) |
| 45 | `5` | target.byte+5 bit 0x4 (IsAirborne flag) AND target.vtable+0x78() != 2 (not Aircraft locomotor mode 2) AND `weapon.Projectile.AA == 0` — similar AA mismatch |

### Phase M — Anti-Ground dispatch (target = cell, no Techno)

When `piVar7 == NULL` (target is a cell or non-Techno):

| # | Code | Condition |
|---:|---:|---|
| 46 | `5` | target.vtable+0x54() == 0 (NOT airborne) AND `weapon.Projectile.AG (+0x2A5) == 0` — projectile can't target ground (typical of AA-only weapons) |
| 47 | (flow to LAB_006fc857) | target.WhatAmI() == 0xB (CellClass) AND `target.field_0xEC != 2 && != 6` — target cell is not navigable for this type |

### Phase N — Bridge-vs-ground attacker/target tier check

The function checks whether attacker and target are on different bridge tiers
(`attacker.OnBridge != target.byte+0x23`). If so, several sub-gates examine cell flags
to determine if the firer-target geometry is legal:

| # | Code | Condition |
|---:|---:|---|
| 48 | `5` | Different bridge tier AND both cells are bridge cells AND attacker not airborne AND warhead is `NavalGunboat` — can't fire across bridge tiers when both on bridges |
| 49 | `5` | Different bridge tier AND `weapon.Warhead.NavalGunboat != 0` AND `|attacker.Z - target.Z| > DAT_00B0EB34 × 2` — Z-delta too large for naval gunboat to fire across |

### Phase O — Type-level MissileSpawn / "must deploy" check

| # | Code | Condition |
|---:|---:|---|
| 50 | `5` | `attacker.Type.byte+0xD97 != 0` AND `attacker.vtable+0x380()` true — type requires deploy and attacker is not in deployed state |

### Phase P — Burst-and-FiringFrame infantry-only gate

For UnitClass attackers with a current burst in progress, check the
`InfantryType.FiringSyncFrame%d` against current animation:

```
if !iStack_4 && WhatAmI()==1 && (CurrentBurstIndex % weapon.Burst) < 2:
    sync_frame = InfantryType.FiringSyncFrame[burstIdx]   // +0xE40 + burstIdx*4
    if sync_frame != -1:
        if attacker.AnimationFrame != sync_frame:
            return 3       // BUSY (waiting for sync frame)
```

This is the "infantry must reach the right animation frame to fire" gate. Otherwise:

| # | Code | Condition |
|---:|---:|---|
| 51 | `3` | Fire-timer not yet elapsed: `(currentFrame - this.lastFireFrame) < this.fireTimer.InitialValue` |

### Phase Q — Sticky-beam ENDING re-check (third occurrence)

Same as Phase D, third time — apparently the function paranoid-checks particle state
at multiple points. All return `3` (BUSY) if particle still active.

### Phase R — Ammo

| # | Code | Condition |
|---:|---:|---|
| 52 | `1` | `attacker.Ammo == 0` — out of ammo |

### Phase S — Cloak

| # | Code | Condition |
|---:|---:|---|
| 53 | `9` | `weapon.byte+0x133 != 0` (DecloakToFire flag) AND `attacker.CloakState != 0` AND NOT (WhatAmI==2 (Aircraft) AND CloakState==2) — cloaked attacker, weapon would decloak it, and it's not a fully-cloaked aircraft. CLOAKED reject. |

### Phase T — Type-level "must deploy"

| # | Code | Condition |
|---:|---:|---|
| 54 | `8` | `attacker.Type.byte+0xD27 != 0` — type requires deploy and isn't currently deployed |

### Phase U — Building-attacking-Naval-targeting cell special

| # | Code | Condition |
|---:|---:|---|
| 55 | `5` | `warhead.NavalGunboat != 0` AND attacker.field_0x14 & 4 (IsAttachedToTransport?) AND `FUN_0062a8e0(target)` returns false — naval-targeting fire is blocked when attacker is attached and target isn't a valid naval target |

### Phase V — Building-attacking-bridge-target rate gate

| # | Code | Condition |
|---:|---:|---|
| 56 | `5` | `warhead.NavalGunboat != 0` AND target.byte+5 & 4 (IsAttached) AND `currentFrame < target.field_0x698` — target is in invulnerability frames |

### Phase W — Techno-specific gates (target IsTechno)

| # | Code | Condition |
|---:|---:|---|
| 57 | `5` | `warhead.NavalGunboat != 0` AND target.vtable+0x160() (IsOnBridge) — can't naval-target bridge units |
| 58 | `5` | `warhead.MindControl (+0x155) != 0` AND `CaptureManagerClass::CanCapture(target) == false` — target can't be mind-controlled (already maxed, immune, etc.) |
| 59 | `5` | `warhead.Verses[target.Armor] == 0` (`warhead+0xA0 + target.Armor*8`) — Verses 0 against target's armor type. **This is the engine's "weapon can't damage this armor → switch weapons" gate.** |
| 60 | `5` | `warhead.BombDisarm (+0x16E) != 0` AND target has no bomb attached (`target[0xE] == 0`) — disarm weapon can't fire at non-bombed target |
| 61 | `5` | `warhead.IvanBomb (+0x157) != 0` AND target already has a bomb (`target[0xE] != 0`) — can't double-bomb |
| 62 | `5` | `target.byte+0x3CD != 0` — generic "untargetable now" flag (mind-control charge phase, etc.) |
| 63 | `5` | attacker.OnBridge != target.byte+0x23 AND both cells are bridge cells (`cell.byte+0x140 & 0x100`) AND attacker not airborne — bridge-tier rejection (Phase N variant for different geometry) |
| 64 | `5` | attacker.OnBridge != target.byte+0x23 AND warhead.NavalGunboat AND `|attacker.Z - target.Z| > DAT_00B0EB34 × 2` — Z-delta too large |

### Phase X — Final type-deploy + Verses re-check

| # | Code | Condition |
|---:|---:|---|
| 65 | `5` | `attacker.Type.byte+0xD97 != 0` AND `attacker.vtable+0x380()` true — same as #50 but at end of pipeline (caller may have changed state) |
| 66 | `8` | If `(stack-arg)` set AND `attacker.vtable+0x3A8(target, weaponIdx)` returns false — final "CanFire this specific weapon at this target right now" check. Returns MOVING/MUST-DEPLOY |

### Phase Y — Final OK

If none of the above fired, return `0` (OK).

---

## 4. Important flag offsets (warhead side)

| Offset | Flag | Effect |
|---|---|---|
| `wh+0x155` | `MindControl` | uses `CaptureManagerClass::CanCapture` gate |
| `wh+0x157` | `IvanBomb` | requires target has no bomb |
| `wh+0x15A` | `NavalGunboat` | aircraft/bridge/Z-delta restrictions |
| `wh+0x15B` | `IsLocomotor` | Magnetron — many per-target immunity flags |
| `wh+0x16D` | `Psychedelic` | requires not-ImmuneToPsionics, not-bunker |
| `wh+0x16E` | `BombDisarm` | requires target has bomb |
| `wh+0xA0 + armor*8` | `Verses[armor]` | 0 → cannot target this armor |

## 5. Important flag offsets (weapon side)

| Offset | Flag | Effect |
|---|---|---|
| `wp+0x129` | `UseSparkParticles` | particle-active block |
| `wp+0x12A` | `UseFireParticles` | particle-active block |
| `wp+0x12D` | `IsRailgun` | particle-active block |
| `wp+0x130` | `IsSonic` | wave-active block |
| `wp+0x131` | `LimboLaunch` | spawner-weapon gates |
| `wp+0x133` | `DecloakToFire` | cloaked-attacker block |
| `wp+0x134` | `CellSnapAA` | read by CanFireAt for AA cell-snap |
| `wp+0x142` | (NavalTargeting?) | naval-targeting target-type gate |
| `wp+0x143` | (OpenTopped allowed?) | open-topped attacker filter |
| `wp+0x14F` | (LightningStorm-related?) | Lightning-storm-active block |
| `wp+0x15C` | (Wave/Sonic-target?) | overlap with sonic-wave gate |
| `wp+0x9C` | `Burst` | used in Phase P burst-frame gate |
| `wp+0xA0` | `Projectile` ptr | `+0x2A4` AA / `+0x2A5` AG |
| `wp+0xAC` | `Warhead` ptr | for Phase H/W warhead-flag gates |

## 6. Important flag offsets (attacker / target instance state)

| Offset | Field | Class |
|---|---|---|
| `attacker+0x82` | `IsOpenTopped` | TechnoClass |
| `attacker+0x8D` | "cannot fire" generic | TechnoClass |
| `attacker+0x11C` | `Transport` (or Bunker host) | TechnoClass |
| `attacker+0x14 bit 0x4` | `IsAttachedToTransport` | ObjectClass |
| `attacker+0x1C8` | mission-busy lock | TechnoClass |
| `attacker+0x1CC` | "no-retarget" exclusion | TechnoClass |
| `attacker+0x274` | sniper-engaged ref | TechnoClass |
| `attacker+0x298` | TS-special hit flag | TechnoClass |
| `attacker+0x2DC` | "pre-deploy lock" | TechnoClass |
| `attacker+0x2E4` | `IsInBunker` | TechnoClass |
| `attacker+0x2EC` | `lastFireFrame` | TechnoClass |
| `attacker+0x2F4` | `fireTimer.InitialValue` | TechnoClass |
| `attacker+0x304` | `sparkParticleSys` (pointer) | TechnoClass |
| `attacker+0x308` | `fireParticleSys` | TechnoClass |
| `attacker+0x314` | `railgunParticleSys` | TechnoClass |
| `attacker.Wave` | sonic-wave pointer | TechnoClass |
| `attacker+0x3B8` | `CurrentBurstIndex` | TechnoClass |
| `attacker.Ammo` | `int` | TechnoClass |
| `attacker.CloakState` | `int` | TechnoClass |
| `attacker.Owner+0x1EC` | "bridge-aware AI" | HouseClass |
| `target+0x81` | `InLimbo` | ObjectClass |
| `target+0x9F` | (cloaked/undetected?) | ObjectClass |
| `target+0xE` | bomb-attached ref | ObjectClass |
| `target+0x23` | `OnBridge` (byte 0xCD on TechnoClass — `IsOnBridge` byte) | ObjectClass |
| `target+0x106` | TS-special target flag | ObjectClass |
| `target+0x3CD` | "untargetable now" | ObjectClass |
| `target+0x5 bit 0x1` | `IsTechno` | AbstractClass |
| `target+0x5 bit 0x4` | (IsAircraft locomotor?) | AbstractClass |
| `target+0x698` | invulnerability-end frame | TechnoClass |
| `target.Type+0xCA0` | `IsSelfHealing` | TechnoTypeClass |
| `target.Type+0xD27` | "must deploy to fire" (attacker side) | TechnoTypeClass |
| `target.Type+0xD35` | `ImmuneToPsionics` | TechnoTypeClass |
| `target.Type+0xD54` | naval-target immunity (specific aircraft) | TechnoTypeClass |
| `target.Type+0xD6A` | TS-special target restriction | TechnoTypeClass |
| `target.Type+0xD94` | Magnetron-immune (secondary) | TechnoTypeClass |
| `target.Type+0xD97` | "MissileSpawn must deploy" | TechnoTypeClass |
| `target.Type+0xE13` | "Magnetron-double-lift block" | TechnoTypeClass |
| `target.Type+0xE18/0xE19` | UnitClass-specific attack permission | TechnoTypeClass |
| `target.Type+0x5EF` | "naval-targetable" | TechnoTypeClass |
| `target.Type+0x690/0x693/0x694` | paired TS-flag block | TechnoTypeClass |

These should be cross-verified against the TechnoTypeClass/TechnoClass struct doc.
Many are TS-legacy candidates — see TS-legacy section below.

---

## 7. Verses as a target-gate (important)

Gate #59 — `warhead.Verses[target.Armor] == 0` → return `5` — means **a weapon whose
warhead has 0% Verses against the target's armor type is BLOCKED from firing at that
target**. This is the engine-side mechanism that drives:

- "Anti-armor weapons can't auto-fire at infantry" (Verses against `none`/`flak` = 0).
- "Anti-air weapons can't fire at ground" (handled via AA flag, but also via Verses).
- "Naval guns can't hit aircraft" (Verses against airborne armors = 0).

Together with the **SelectWeaponAgainst** logic (in
[`anti_air_dispatch.md`](anti_air_dispatch.md)), Verses=0 produces the automatic
primary/secondary weapon swap when the target's armor doesn't match the active weapon.

**ForceFire bypasses this gate** — see §8 below.

---

## 8. ForceFire and target-gate bypass

When the player Ctrl-clicks (force-fire), most of these gates are skipped at a
**higher dispatch level** — specifically in `Mission_Attack` or the input-command
handler that determines the weapon. The path is roughly:

```
[ForceFire input]
   → DetermineAction skip target-validity check
   → set Target = picked cell/object
   → Mission_Attack → calls GetFireError
```

GetFireError itself does NOT have a "ForceFire" parameter — the bypass happens
**upstream** by ensuring the weapon picked is the right one for ForceFire intent.
However, the **fire-timer / particle-active / ammo / cloak** gates (Phases D/Q/R/S)
are still applied even on ForceFire — the player can't bypass cooldown or ammo.

**Verses=0 specifically:** documented behavior is that ForceFire shoots but deals 0
damage. The implementation likely bypasses gate #59 by ForceFire setting a flag in
`Mission_Attack` that suppresses the Verses-check. Open follow-up: trace where this
bypass actually lives.

---

## 9. AffectsAllies / friendly-fire gate location

GetFireError does **NOT** check `wh->AffectsAllies (+0x179)` — that gate fires
**downstream** in `TechnoClass::ReceiveDamage` (zeroes the damage if same-house and
AffectsAllies=false). So a friendly target IS targetable by GetFireError, but the
damage is nullified at impact. See [`receive_damage_pipeline.md`](receive_damage_pipeline.md) §7.

The exception is the **auto-target / AI threat scan** which does pre-filter friendly
targets before invoking GetFireError. Documented in [`target_acquisition.md`](target_acquisition.md).

---

## 10. TS-legacy filter

Most gates are live in YR. Known/probable TS-legacy:

- **`target.Type.byte+0xD6A`** (#18) — paired with `target[0x106]` and `target.vtable+0x54()==0`. The combination is rare in retail YR — flag for follow-up.
- **`target.Type.byte+0x690/0x693/0x694`** (#13, #14) — paired flags. Likely a TS-era "cannot target this type" mechanism with no shipping YR INI setting them. **Flag for follow-up** — could be a TS-only path.
- **`attacker.field_0x298`** (#13) — TS-era hit flag. Suspect TS-legacy.
- **Lightning-storm block** (#26) — active during Weather Storm SW, which IS a YR feature. Active.
- **Bridge-tier checks** (#48, #49, #63, #64) — all live (bridges are core YR).

The function as a whole IS live; only a few specific flag pairings are suspect.

---

## 11. Open follow-ups

1. **Enumerate the full FireError enum** — the function uses codes 0/1/2/3/5/6/8/9. Codes 4 and 7 are not used here but may be returned by sibling subclasses (InfantryClass / AircraftClass). Trace by examining `Fire_At_Target` consumers. Priority: MEDIUM.
2. **`weapon.byte+0x142/0x143/0x14F/0x15C` identities** — these don't have firmly verified names. Cross-reference against `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`. Priority: MEDIUM.
3. **`TechnoType.byte+0xD27/0xD94/0xD97/0xE13` identities** — verify which TechnoTypeClass INI keys map to these. Priority: MEDIUM.
4. **TS-legacy verification** on the suspect gates (#13/#14/#18) — set up a parity test, check whether any vanilla YR target hits these flags. Priority: LOW (the suspected dead paths don't change observable behavior).
5. **ForceFire bypass mechanism** — trace where ForceFire suppresses gate #59 (Verses-zero block). Priority: MEDIUM (parity behavior depends on this).
6. **Sticky-beam particle-active triple-check (Phases D / K / Q)** — three different points in the function check the same particle flags. Determine why three checks exist (probably defensive against intervening state changes). Priority: LOW.
7. **Phase Y `vtable+0x3A8(target, weaponIdx)` identity** — the final "can I really fire this weapon at this target right now" check. Looks like `CanFire`/`CanShoot`. Priority: MEDIUM.
8. **`piVar7[0x9F]` and `piVar7+0x3CD`** — generic "untargetable" flags on the target instance. Trace writers to determine which game states set them. Priority: MEDIUM.

---

## 12. Sources

- Live decompilation of `TechnoClass::GetFireError` at `0x006FC0B0` (2026-05-17).
- Live decompilation of `BuildingClass::GetFireError` at `0x00447F10` (2026-05-17).
- Live decompilation of `TechnoClass::CanFireAt` at `0x006F77B0` (2026-05-17) — confirmed it's a range-check wrapper that delegates to InRange after weapon-flag `+0x134` (CellSnapAA) and IsHighFlying coord handling.
- Caller list for `GetFireError` returned only `UnitClass::Fire_At_Target @ 0x00736df0` — vtable-indirect dispatch (`vtable+0x314`) means the caller index is incomplete. See [`../../FIRE_AT_ANALYSIS.md`](../../FIRE_AT_ANALYSIS.md) for the per-tick Mission_Attack call pattern that invokes this function.
- WeaponTypeClass struct: [`../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md).
- WarheadTypeClass struct: [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md).
- Cross-reference: [`damage_formula.md`](damage_formula.md) §6 for the `Verses[armor]==0` semantics that gate #59 in this function uses as a hard target-eligibility filter.
- Cross-reference: [`verses_armor_matrix.md`](verses_armor_matrix.md) for the armor-type index that gate #59 looks up.
