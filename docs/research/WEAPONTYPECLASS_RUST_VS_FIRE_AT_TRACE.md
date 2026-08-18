---
name: WeaponTypeClass Rust vs Fire_At — Parity Trace
description: Side-by-side comparison of Rust combat firing behavior vs TechnoClass::Fire_At at 0x006fdd50, focused on visual chain, directional muzzle anim, and RevealOnFire.
type: research
date: 2026-04-24
binary: gamemd.exe (Yuri's Revenge 1.001)
---

# WeaponTypeClass — Rust vs `Fire_At` Parity Trace

Scope: three observable behaviors the player sees every time a unit fires. Verified in the binary at `TechnoClass::Fire_At` (`0x006fdd50`) and cross-checked against the live Rust code.

**HARD-GATE:** Research only. No implementation in this document.

| Gap | Binary behavior present | Rust implemented | Player-visible impact |
|---|---|---|---|
| A — Exclusive visual chain (IsLaser / IsElectricBolt / IsRadBeam / IsRadEruption / IsMagBeam) | Yes | **No (fields parsed, zero consumers)** | High — Prism, Mirage, Rad gun, Magnetron, Tesla all silent visually |
| B — Directional muzzle anim `Anim=` | Yes (8-way pick + elite/transport override) | **No for non-garrison fire** (garrison uses `OccupantAnim` only) | High — no muzzle flashes on infantry/vehicle shots |
| C — `RevealOnFire` gating + owner | Yes (gated on TARGET being human, reveals for TARGET's house) | **Partial — wrong owner semantics** | Medium — "see incoming shots" mechanic does not function |

---

## A. Exclusive Visual-Effect Chain

### Binary (`Fire_At` @ `0x006fdd50`, post-bullet-spawn block)

Decompilation (simplified, original pseudocode at ~0x006ff1a0):

```c
if (weapon->IsLaser) {               // byte 0x149
    if (is_deploy_vehicle) {
        laser = TechnoClass::SpawnLaser(target, this, ..., &null_coord);
        if (gattling_level > 0) { laser[0x21] = 1; laser[0x1c] = 5; }
        else { laser[0x1c] = 3; }
    } else {
        laser = TechnoClass::SpawnLaser(target, this, weapon, &null_coord);
        if (laser && weapon->IsHouseColor) {     // byte 0x14D
            laser[0x1c] = 2;                     // color-code = 2 (house color)
        }
    }
}
else if (weapon->IsElectricBolt) {   // byte 0x151
    TechnoClass::SpawnElectricBoltEffect(target);       // @ 0x006fd570
}
else if (weapon->IsRadBeam) {        // byte 0x154
    if (weapon->Warhead == 0 || warhead[0x15A] == 0) {
        TechnoClass::SpawnRadBeam(target, 0);           // @ 0x006fd620, flavor=0
    } else {
        TechnoClass::SpawnRadBeam(target, 1);           // flavor=1
    }
}
else if (weapon->IsRadEruption) {    // byte 0x155
    TechnoClass::SpawnRadEruption(ftol_result);         // @ 0x006fd800
}
else if (weapon->IsMagBeam) {        // byte 0x15C
    if (!this->Wave && (!target || !target->IsCell())) {
        wave_ptr = operator_new(0x240);
        if (wave_ptr) {
            this->Wave = WaveClass::Constructor(&iStack_54, &uStack_98, this, 3, target);
        }
    }
}
```

**Properties confirmed from binary:**
- **Strictly exclusive**: `if/else if/...`. First matching flag fires, the rest are skipped even if set.
- **Spawner functions exist** — `SpawnLaser` @ `0x006fd210`, `SpawnElectricBoltEffect` @ `0x006fd570`, `SpawnRadBeam` @ `0x006fd620`, `SpawnRadEruption` @ `0x006fd800`, `WaveClass::Constructor` (type 3 = magnetron).
- **`IsSonic` is separate** and fires *unconditionally* before this chain (line ~0x006ff0b0): `if (weapon->IsSonic) this->Wave = WaveClass::Constructor(..., type=0, target);`. It is NOT in the exclusive chain and can co-exist with one of the five above. Exception: the `IsMagBeam` branch is *skipped* if a Wave is already present (from the earlier `IsSonic` path).
- **`DiskLaser`** (byte 0x14A) is handled **earlier** in `Fire_At` (line ~0x006fecf0), *before* the bullet spawns, and `return`s early. It is NOT in the same chain — it's a pre-empting branch that replaces the whole fire path with a `DiskLaserClass` allocation.

### Rust

```
src/rules/weapon_type.rs:151  pub is_laser: bool,
src/rules/weapon_type.rs:165  pub is_electric_bolt: bool,
src/rules/weapon_type.rs:171  pub is_rad_beam: bool,
src/rules/weapon_type.rs:173  pub is_rad_eruption: bool,
src/rules/weapon_type.rs:175  pub is_mag_beam: bool,
```

Grep across `src/` confirms **all 16 occurrences of these five fields live in `weapon_type.rs` alone** — 10 parse/struct declarations plus 6 test assertions. Zero production-code readers in `src/sim/`, `src/render/`, or `src/app*`. The fields are parsed, stored, and ignored.

### Gap

Implementing this requires (in order of visibility):
1. **SpawnLaser equivalent** — the most common: Prism Tower, Prism Tank, Mirage Tank, GI elite, Navy SEAL, Kirov explosion, laser-Guardian-GI.
2. **SpawnElectricBoltEffect** — Tesla Trooper/Coil/Trooper-Elite, Tesla Tank.
3. **SpawnRadBeam** + **SpawnRadEruption** — Desolator, rad-infused IFV, eruption impact.
4. **IsMagBeam** — Magnetron (Wave type 3).
5. **Exclusivity** — enforce the if/else chain; don't spawn multiple effects per shot.

`IsSonic` (Sonic Tank) is a sixth, always-on effect spawned earlier in `Fire_At` — currently also ungated in Rust.

---

## B. Directional Muzzle Anim `Anim=`

### Binary (`Fire_At` @ ~`0x006ff018`, pre-effect-spawn)

```c
// uVar18 = weapon ptr
iVar9 = 0;

if (*(int *)(uVar18 + 0x104) == 8) {                  // weapon->Anim.ActiveCount == 8
    // 8-way directional pick
    facing = this->vtable[0x308 / 4](&piStack_cc);    // IsElite/GetFacing? returns coord with facing in hi bits
    idx = (((facing >> 0xC) + 1) >> 1) & 7;           // standard 8-way facing bin
    idx = (idx + 1) & 0x80000007;                     // rotation offset to align with gamemd convention
    iVar9 = *(AnimTypeClass **)(weapon->Anim.Buffer + idx * 4);
}
else if (*(int *)(uVar18 + 0x104) > 0) {              // Anim.ActiveCount > 0 but != 8
    iVar9 = *(AnimTypeClass **)(weapon->Anim.Buffer); // always entry 0
}

// Elite-override (vtable+0x400 on TechnoClass subclass — likely "IsOccupant" or "InGarrison")
if (this->vtable[0x400 / 4]()) {
    iVar9 = *(AnimTypeClass **)(uVar18 + 0x110);      // weapon->OccupantAnim
}

// Transport-override (when firing FROM an open-topped transport, field_0x82 is set)
if (iVar9 == 0 && this->field_0x82 && *(int *)(uVar18 + 0x118) != 0) {
    iVar9 = *(AnimTypeClass **)(uVar18 + 0x118);      // weapon->OpenToppedAnim
}

// Sound + anim spawn
if (weapon->Report.ActiveCount > 0 && !this->field_0xCD5) { VocClass::PlayAt(...); }
if (iVar9) {
    anim = operator_new(0x1C8);
    if (anim) AnimClass::Constructor(iVar9, &uStack_98, 0, 1, 0x600, 0, 0);
    // iVar9 == 6 → AircraftClass branch (Z-offset adjustment)
    if (this->GetRTTI() != 6) AnimClass::SetOwnerObject(this);
}
```

**Properties confirmed:**
- **`Anim.ActiveCount == 8`** → strict 8-way directional pick. Any count other than 8 (including 1, 2, 4, 16) falls back to entry 0.
- **Indexing formula** `(((facing >> 0xC) + 1) >> 1) & 7` with `+1` rotation — standard gamemd 8-way binning (the `+1` maps facing-0 (N) to index 0 after the half-step shift).
- **OccupantAnim** (0x110) acts as **elite/in-occupant override**, *not* garrison-only as the field name suggests. A vtable+0x400 dispatch decides — this is likely `TechnoClass::IsInAir` or `Is_In_Air()`; InfantryClass and UnitClass may override it differently.
- **OpenToppedAnim** (0x118) is **only** used when `iVar9 is still 0` (i.e., no primary Anim picked) AND `field_0x82` is set. `field_0x82` on TechnoClass is likely "in an open-topped transport" passenger state.

### Rust

```
src/rules/weapon_type.rs:~95   pub anim: Vec<String>,     // parsed
src/rules/weapon_type.rs:~200  anim: section.get_list("Anim", ","),
```

Grep of `src/sim/`, `src/render/`, `src/app*` for `weapon.anim`, `weapon_anim`, or `.anim[`: **no matches** in the fire path. Only the garrison muzzle-flash path ([src/app_building_anim.rs:492](src/app_building_anim.rs#L492) `tick_garrison_muzzle_flashes`) spawns any fire-side anim, and it uses `OccupantAnim` exclusively — not `Anim=`.

### Gap

Non-garrison unit fire currently shows **no muzzle flash**. The missing implementation:
1. After a `SimFireEvent` for a non-garrison unit, pick the anim from `weapon.anim` using entity facing:
   - `len == 8` → `anim[facing_index_8way(entity.facing)]`
   - `len > 0 && len != 8` → `anim[0]`
   - `len == 0` → skip
2. Optionally override to `weapon.occupant_anim` when the firer is an occupant (our equivalent of the vtable+0x400 check — in Rust terms that's probably "is this entity in `GarrisonOccupant` state").
3. Optionally fall back to `weapon.open_topped_anim` when the firer is a passenger of an open-topped transport.
4. Render side spawns the anim at the muzzle FLH position (same resolver as garrison muzzle flash).

Not studied yet: the exact facing→index mapping convention in our Rust. Before implementing, verify our facing encoding (0..256 vs 0..8 vs radians) matches the gamemd `(facing >> 0xC) + 1) >> 1) & 7` formula.

---

## C. `RevealOnFire` — Wrong Owner, Missing Gate

### Binary (`Fire_At` @ `LAB_006ff6d0`, post-fire)

```c
LAB_006ff6d0:
if (target != NULL &&
    (target->AbstractFlags_0x14 & 2) != 0 &&          // target flag bit 1 (active/selected)
    (target_house = target->GetHouse()) != NULL &&
    HouseClass::IsHumanPlayer(target_house) &&
    weapon->RevealOnFire)                             // byte 0x137
{
    // GetCoordinates of *this* (the firer, not the target)
    this->vtable[0x48 / 4](&firer_coords, 3, target_house, 0, 0, 0, 1, 0);
    MapClass::RevealShroud(&firer_coords, 3, target_house, 0, 0, 0, 1, 0);
    MapClass::UpdateFogBorder(&firer_coords, 4);
}
```

**Properties confirmed:**
- **Position**: firer's coordinates (reveal shroud around *where the shot came from*).
- **Owner who sees the reveal**: the **target's** house, and *only if that house is a human player*. This is the "see incoming shot" mechanic — an enemy shooting you exposes their position on your map.
- **Radius**: 3 cells for `RevealShroud`, 4 cells for `UpdateFogBorder`. In standard YR (FogOfWar off) `UpdateFogBorder` is effectively a no-op — safe to skip.
- **Additional flag check**: `target[0x14] & 2` — the `AbstractClass::Flags` bit for "active" or "valid." Probably always true for a living target; this is defensive.
- **A second reveal path** exists earlier in the `Spawner`-weapon branch (~0x006fdef3) with the same gating pattern. Covers Aircraft Carrier Dreadnought-style spawned-unit shots.

### Rust

[src/sim/combat/mod.rs:1173-1180](src/sim/combat/mod.rs#L1173-L1180):

```rust
if weapon.reveal_on_fire {
    reveal_events.push(RevealEvent {
        owner: snap.owner,          // ← FIRER's owner (wrong)
        rx: snap.pos_rx,
        ry: snap.pos_ry,
        radius: REVEAL_ON_FIRE_RADIUS,  // = 3, matches binary
    });
}
```

Consumed at [src/sim/world/mod.rs:1170](src/sim/world/mod.rs#L1170):

```rust
for ev in &combat_result.reveal_events {
    vision::reveal_radius(&mut self.fog, ev.owner, ev.rx, ev.ry, ev.radius);
}
```

### Gap

**Three mismatches vs binary:**

1. **Owner is the firer's, not the target's.** In binary, `RevealShroud` is called with `target_house` as the "whose shroud gets cleared" argument. Our Rust uses `snap.owner` (firer's owner). Net effect: we reveal the firer's *own* shroud at the firer's position — which is a no-op because the firer already has LOS to their own position. **The mechanic does not function.**
2. **No human-player gate.** Binary only fires the reveal if `target_house->IsHumanPlayer()`. Our Rust always fires. With the owner correction above, this matters because in a 30-player skirmish with many AIs, AI-vs-AI fire would spuriously reveal AI shroud (no observable effect) or crash if target owner is invalid.
3. **No target-null / target-flag-bit-2 guard.** Minor robustness — our code already filters by "weapon fired" which implies a target exists, so this is likely safe to omit. Document it but don't add guards speculatively.

**Correction shape (for a future implementation task, not this doc):**

```rust
// Pseudocode only — do not apply from this doc.
if weapon.reveal_on_fire {
    if let Some(target) = entities.get(snap.target) {
        if house_is_human(target.owner) {
            reveal_events.push(RevealEvent {
                owner: target.owner,      // target's house
                rx: snap.pos_rx,          // firer's position (already correct)
                ry: snap.pos_ry,
                radius: REVEAL_ON_FIRE_RADIUS,
            });
        }
    }
}
```

---

## Combined Summary

| # | Gap | Fix size | Dependencies | Test pattern |
|---|---|---|---|---|
| A | Visual chain (Laser / ElectricBolt / RadBeam / RadEruption / MagBeam) | Large — needs 5 render-side effect spawners + sim→render event plumbing + exclusivity rule | `SpawnLaser` needs laser draw system; `SpawnRadBeam` needs warhead `0x15A` flag (rad variant); `IsMagBeam` / `IsSonic` need `WaveClass` | Fire Prism Tank at Rhino; expect blue beam. Fire Tesla at IFV; expect lightning bolt. |
| B | Directional muzzle anim | Medium — add a `weapon_anim_index_8way(facing, anim.len())` helper, extend `SimFireEvent` with a `muzzle_anim: Option<InternedId>`, spawn in the existing `world_effects` pipeline | Needs facing→8-way index formula that matches gamemd; verify with a unit test against `(facing >> 0xC + 1) >> 1 & 7` | Fire a Conscript; expect GUNFIRE_N/NE/... frame; rotate and confirm it updates. |
| C | RevealOnFire owner + human gate | Small — 5-line change in combat firing block | Need `house_is_human` helper; our `fog` already supports per-owner reveals | AI shoots at your unit with a `RevealOnFire=yes` weapon; expect shroud to clear around the AI. |

**Recommended ordering if implementing:**
1. **C** first — smallest, most targeted fix; easy to verify; currently actively wrong.
2. **B** second — visually obvious, bounded scope, builds on the existing `SimFireEvent` / `world_effects` plumbing.
3. **A** last — largest and most coupled to the render pipeline; splits naturally into 5 effects; worth its own brainstorm before implementation.

---

## Open Items (for future investigation)

1. **vtable+0x400 on TechnoClass** — what subclass method is this? Confirms whether `OccupantAnim` override triggers on "firing from garrison", "is elite", or "is in the air". Decompile `InfantryClass`/`UnitClass`/`BuildingClass` v-tables at slot index 0x100.
2. **`field_0x82`** — which Techno state bit? Likely "passenger in open-topped vehicle" but not verified.
3. **Facing encoding** — `Fire_At` uses `(facing >> 0xC + 1 >> 1) & 7`. What is the bit-width of `facing` here (16-bit? 32-bit?), and what does our Rust use for facing? A mismatch here would silently pick the wrong anim frame every shot.
4. **`target[0x14] & 2`** — which AbstractClass flag bit is this? Likely "active/valid"; not critical for Rust since we filter via `EntityStore.get()`.
5. **`Spawner` sub-path RevealOnFire** — separately gated inside the Spawner branch. Only matters when implementing `Spawner=yes` weapons (Aircraft Carrier). Note and defer.
6. **`DiskLaser`** — pre-empts the entire fire path (returns before bullet allocate). Separate implementation from the exclusive chain. Only used by Floating Disc.

---

## Sources

- **Ghidra addresses decompiled:**
  - `0x006fdd50` `TechnoClass::Fire_At` (primary)
  - `0x006fd210` `TechnoClass::SpawnLaser` (confirmed exists, not decompiled)
  - `0x006fd570` `TechnoClass::SpawnElectricBoltEffect` (exists)
  - `0x006fd620` `TechnoClass::SpawnRadBeam` (exists)
  - `0x006fd800` `TechnoClass::SpawnRadEruption` (exists)
- **Rust files traced:**
  - [src/rules/weapon_type.rs](src/rules/weapon_type.rs) — field declarations, parsing
  - [src/sim/combat/mod.rs:1162-1180](src/sim/combat/mod.rs#L1162-L1180) — SimFireEvent push + RevealEvent push
  - [src/sim/world/mod.rs:120-136](src/sim/world/mod.rs#L120-L136) — SimFireEvent definition
  - [src/sim/world/mod.rs:1170-1172](src/sim/world/mod.rs#L1170-L1172) — RevealEvent consumer
  - [src/app_building_anim.rs:485-540](src/app_building_anim.rs#L485-L540) — garrison-only muzzle flash
- **Prior companion doc:** [docs/WEAPONTYPECLASS_VERIFICATION_AND_CONSUMERS_GHIDRA_REPORT.md](docs/WEAPONTYPECLASS_VERIFICATION_AND_CONSUMERS_GHIDRA_REPORT.md)
