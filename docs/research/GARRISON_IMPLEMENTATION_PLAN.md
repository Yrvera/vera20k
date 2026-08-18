# Garrison System — Implementation Plan

Maps every gamemd.exe garrison behavior to exact code locations in this Rust engine.
All gamemd details sourced from verified Ghidra decompilation (see `GARRISON_SYSTEM_GHIDRA_REPORT.md`).

---

## Status Overview

| System | Status | Notes |
|--------|--------|-------|
| Cursor & action | ✅ Complete | `app_cursor.rs:188-210`, `app_context_order.rs:194-252` |
| Boarding & pathfinding | ✅ Complete | `Command::EnterTransport`, `tick_boarding`, `tick_unloading` |
| Ownership transfer | ✅ Complete | Immediate in `tick_boarding`/`tick_unloading` |
| Occupant pips | ✅ Complete | `app_ui_overlays.rs:259-346`, pips.shp frames 6-12 |
| ActiveAnimGarrisoned | ✅ Complete | `shp.rs:458-467`, loops while garrisoned |
| INI parsing | ✅ Complete | All keys parsed but combat values UNUSED |
| **Garrison combat** | ❌ Not implemented | **P0 — the biggest gap** |
| Muzzle flash at fire ports | ❌ Not implemented | Data parsed, render not hooked |
| Fire gating (empty garrison) | ❌ Not implemented | |
| Auto-target for garrison | ❌ Not implemented | |
| Eject on destruction | ⚠️ Different | We kill occupants; original ejects them |
| Turret suppression | ❌ Not implemented | Minor visual |
| Sound/EVA | ❌ Not implemented | |
| Kill credit to occupant | ❌ Not implemented | |

---

## 1. Data Model Changes

### 1a. `garrison_fire_index` on PassengerCargo

**gamemd:** `BuildingClass+0x69C` (CurrentFireIdx). Initialized to 0. Advanced by
`Fire_At` after each shot: `++idx % GetOccupantCount()`. Reset to 0 in `SellBuilding`
before occupant ejection.

**File:** `sim/passenger.rs` → `PassengerCargo`
```rust
pub garrison_fire_index: u8,  // Init to 0 in PassengerCargo::new()
```

### 1b. `garrison_muzzle_index` on SimFireEvent

**gamemd:** `GetFireCoords` (0x00453840) reads `MuzzleFlash[CurrentFireIdx]` from
`BuildingTypeClass+0x1588` (stride 8 = X,Y int pairs). The muzzle flash position
is per-fire-port, NOT per-weapon-slot.

**File:** `sim/world/mod.rs` → `SimFireEvent`
```rust
pub garrison_muzzle_index: Option<u8>,  // None = normal FLH, Some(idx) = fire port
```

---

## 2. IsOccupied Query

**gamemd:** `BuildingClass::IsOccupied` (0x00458DD0, report §13c)

```
CanBeOccupied (TypeClass+0x157B) AND CanOccupyFire (TypeClass+0x157C) AND count > 0
```

All three must be true. `CanBeOccupied` alone is NOT enough — `CanOccupyFire` controls
whether garrisoned infantry can shoot. A building can be garrisonable but not fireable.

**Our code:** Both flags are parsed in `object_type.rs` (`can_be_occupied`, `can_occupy_fire`).
Use this three-way check everywhere garrison fire logic is added.

---

## 3. Fire Gating — Empty Garrisonable Buildings Cannot Fire

**gamemd:** `FUN_007091D0` (report §12e)
```
IF type == Building AND CanBeOccupied=yes AND GetOccupantCount() == 0:
    CANNOT FIRE — return 0
```

Even if the building has its own Primary weapon, a `CanBeOccupied` building with
no occupants is defenseless. This is a hard fire-gate.

**File:** `sim/combat/combat_fire_gate.rs` → `collect_fire_blocked_entities()`

Add after the existing power check (~line 90):
```rust
// Garrison fire gate: CanBeOccupied buildings with no occupants cannot fire.
if entity.category == EntityCategory::Structure {
    if let Some(obj) = rules.object(&entity.type_ref) {
        if obj.can_be_occupied
            && entity.passenger_role.cargo().map_or(true, |c| c.is_empty())
        {
            blocked.insert(entity.stable_id);
        }
    }
}
```

---

## 4. Garrison Weapon Selection

**gamemd:** `BuildingClass::GetWeapon` (0x004526F0, report §13b)

Full priority chain:
```
1. Fire port slots (Building+0x5EC, count +0x702) — checked FIRST
   → If port has infantry, return its weapon
   → (Fire ports are separate from garrison; skip for now — P3)

2. IsOccupied() check — if false OR fire_index out of bounds:
   → Fall back to building's own weapon (TechnoClass::GetWeapon)

3. Get current occupant: Items[garrison_fire_index]
   → occupant = *(*(building+0x688) + fire_index * 4)

4. Check occupant veterancy:
   → NOT elite: use OccupyWeapon (InfantryTypeClass+0xE04)
   → Elite:     use EliteOccupyWeapon (InfantryTypeClass+0xE20)

5. If OccupyWeapon/EliteOccupyWeapon is NULL:
   → Fall back to occupant's primary weapon (GetWeapon(0))
```

**File:** `sim/combat/combat_weapon.rs`

New function:
```rust
pub fn select_garrison_weapon<'a>(
    rules: &'a RuleSet,
    occupant_type_ref: &str,
    occupant_veterancy: u16,  // 0-200, >=100 = veteran, >=200 = elite
    target_category: EntityCategory,
    target_armor: &str,
) -> Option<SelectedWeapon<'a>> {
    let occupant_obj = rules.object(occupant_type_ref)?;
    let is_elite = occupant_veterancy >= 200;

    // 1. Try elite/normal OccupyWeapon
    let occupy_weapon_id = if is_elite {
        occupant_obj.elite_occupy_weapon.as_deref()
    } else {
        occupant_obj.occupy_weapon.as_deref()
    };

    // 2. Try resolve the occupy weapon
    if let Some(wid) = occupy_weapon_id {
        if let Some(sw) = try_weapon(rules, wid, target_category, target_armor, WeaponSlot::Primary) {
            return Some(sw);
        }
    }

    // 3. Fallback: occupant's primary weapon
    if let Some(ref primary) = occupant_obj.primary {
        return try_weapon(rules, primary, target_category, target_armor, WeaponSlot::Primary);
    }
    None
}
```

---

## 5. Auto-Target Acquisition

**gamemd:** `TechnoClass::Greatest_Threat` (0x006F8DF0, report §15a-b)

Buildings run standard `TechnoClass::AI_Update` which dispatches Guard mission.
Guard mission calls `Greatest_Threat` to find targets. For garrisoned buildings:

```
scanRange_cells = GetHalfFoundationSize() + 1 + OccupyWeaponRange
```

The `+1` makes the scan range 1 cell LARGER than firing range, so buildings detect
approaching enemies before they're in range. Buildings use **cell-based scanning**
(iterate all cells within radius), evaluating each candidate via `Evaluate_Candidate`.

**File:** `sim/combat/mod.rs` — add before Phase 1 snapshot

```rust
// Garrison auto-acquire: idle garrisoned buildings scan for targets
for &id in &keys {
    let entity = match entities.get(id) { Some(e) => e, None => continue };
    if entity.category != EntityCategory::Structure { continue; }
    if entity.attack_target.is_some() { continue; }  // already attacking
    if entity.dying || !entity.is_alive() { continue; }

    let obj = match rules.object(&entity.type_ref) { Some(o) => o, None => continue };
    if !obj.can_be_occupied || !obj.can_occupy_fire { continue; }

    let cargo = match entity.passenger_role.cargo() { Some(c) => c, None => continue };
    if cargo.is_empty() { continue; }

    // Scan range = OccupyWeaponRange + 1 (buffer, matching original +1)
    let scan_range = SimFixed::from_num(rules.garrison.occupy_weapon_range + 1);

    // Get current occupant's weapon for verses validation
    let occ_idx = cargo.garrison_fire_index as usize % cargo.count() as usize;
    let occ_id = cargo.passengers[occ_idx];

    if let Some(target_id) = acquire_best_target(entities, rules, /*snap*/, obj, fog) {
        // Set attack target on the building
        // (need to build appropriate snapshot or pass garrison range override)
    }
}
```

The existing `acquire_best_target()` can be reused but needs a range override parameter
for the garrison scan range instead of weapon range.

---

## 6. Garrison Combat in tick_combat_with_fog

### 6a. Range Formula

**gamemd:** `TechnoClass::InRange` (0x006F7220, report §14a)

```
garrison_range_leptons = (GetHalfFoundationSize() + OccupyWeaponRange) * 256
```

- `GetHalfFoundationSize()` = `min(foundation_width, foundation_height) / 2`
  (0x00458E00, report §14e). Integer division, truncating.
- Range is in **leptons** (256 per cell)
- This REPLACES the weapon's native range entirely
- **Target foundation bonus:** If target is a building, add `(target_height + target_width) * 64` leptons

**Our code:** Foundation dimensions are available from art.ini foundation parsing.
Need to compute `half_foundation` for each garrisoned building. The range check
in `tick_combat_with_fog` (line ~634) must use this formula instead of `weapon.range`.

### 6b. Damage Multiplier

**gamemd:** `TechnoClass::Fire_At` (0x006FDD50, report §12b)

Applied AFTER base damage and veterancy bonuses, BEFORE projectile creation:
```
IF IsOccupied():
    damage = ftol((float)damage * OccupyDamageMultiplier)  // RulesClass+0xF40
```

`OccupyDamageMultiplier` is a float (default 1.0). The multiplication uses FPU
float math, then converts back to int via `ftol`.

**Our code:** In `tick_combat_with_fog` Phase 2 damage calc (line ~685):
```rust
let mut damage = weapon.damage * selected.verses_pct / 100;
if is_garrison_attacker {
    // Apply as float then truncate, matching original ftol behavior
    damage = (damage as f64 * rules.garrison.occupy_damage_multiplier) as i32;
}
```

Note: using `f64` for this single multiply is acceptable — this is NOT sim-critical
math (damage is already an integer), and the original uses FPU floats too.

### 6c. ROF Formula

**gamemd:** `TechnoClass::GetROF` (0x006FCFA0, report §5d, verified in decompiled code)

```
1. base_rof = weapon.ROF (in game frames at 15 fps)
2. IF IsOccupied() AND occupant_count > 0:
      rof = rof / occupant_count     // Integer division — MORE occupants = FASTER fire
3. IF OccupyROFMultiplier > 0.0:   // RulesClass+0xF44, FLOAT_007e1748 = 0.0f
      rof = ftol((float)rof / OccupyROFMultiplier)
```

**Critical:** ROF is divided by occupant count FIRST, then by the multiplier.
More occupants = proportionally faster aggregate fire rate. With 5 occupants,
the building fires 5× faster than a single infantry would.

**Our code:** In `tick_combat_with_fog` Phase 2 ROF calc (line ~730):
```rust
let mut cooldown = rof_to_cooldown_ticks(weapon.rof, tick_ms);
if is_garrison_attacker {
    let count = cargo.count().max(1) as u16;
    cooldown = cooldown / count;  // Integer division
    if rules.garrison.occupy_rof_multiplier > 0.0 {
        cooldown = (cooldown as f64 / rules.garrison.occupy_rof_multiplier) as u16;
    }
    cooldown = cooldown.max(1);  // Minimum 1 tick
}
```

### 6d. Round-Robin Advancement

**gamemd:** `Fire_At` (0x006FDD50, report §12a)

```c
IF IsOccupied() AND type == Building:
    CurrentFireIdx++                    // Building+0x69C
    CurrentFireIdx %= GetOccupantCount()
```

Runs after EVERY successful shot. Each shot cycles to the next occupant.

**Our code:** In Phase 3 (apply updates), after setting cooldown for garrison:
```rust
if let Some(cargo) = entity.passenger_role.cargo_mut() {
    let count = cargo.count() as u8;
    if count > 0 {
        cargo.garrison_fire_index = (cargo.garrison_fire_index + 1) % count;
    }
}
```

### 6e. Fire Event with Muzzle Port

**Our code:** When pushing `SimFireEvent` for garrison fire:
```rust
sim.fire_events.push(SimFireEvent {
    attacker_id: building_id,
    weapon_slot: WeaponSlot::Primary,
    target_id,
    garrison_muzzle_index: Some(fire_index),
});
```

### 6f. Kill Credit to Occupant

**gamemd:** `TechnoClass::RegisterDestruction` (0x00702D40, report §15d)

Kill credit goes to `Items[CurrentFireIdx]→InfantryTypeClass`, NOT the building.
The occupant infantry earns veterancy from garrison kills.

**Our code:** When tracking `last_attacker_id` on the target, set it to the
**occupant's stable_id** (not the building's). This way retaliation targets
the building (since the occupant is hidden), but kill credit flows correctly.

Actually, simpler approach: store the occupant_id alongside the building's fire
event. When the target dies, award experience to the occupant entity.

---

## 7. Muzzle Flash Rendering

### 7a. Fire Port Positions

**gamemd:** `BuildingClass::GetFireCoords` (0x00453840, report §14a)

```
pixel_offset = MuzzleFlash[CurrentFireIdx]  // TypeClass+0x1588, stride 8 bytes
world_offset = IsometricPixelToWorld(pixel_offset)
fire_coords = building_center + world_offset
fire_coords.z = building_z  // No Z offset from fire ports
```

**Our data:** `art_data.rs:91-94` parses `MuzzleFlash0..9` as `Vec<(i32, i32)>`.
These are screen-space pixel offsets (X, Y). Already loaded per building type.

### 7b. No Ambient Garrison Flash in Update

**gamemd:** `BuildingClass::Update` (0x0043F900, report §14f)

Correction from `CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md`:
do not implement a normal occupied-garrison ambient muzzle flash from
`BuildingClass::Update`. The previously suspected 24-frame branch is chrono /
temporal sparkle rendering, gated by `TechnoClass` warp flags `+0x270/+0x271`.
It uses `[General] ChronoSparkle1` at `RulesClass+0x344`; when
`MaxNumberOccupants > 0`, it reuses `MuzzleFlashN` offsets only as chrono
sparkle anchors with `(g_CurrentFrameCounter + port) % 24 == 0`.

Normal garrison combat flashes remain shot-triggered through `Fire_At` and
`WeaponType+0x110` (`OccupantAnim`).

### 7c. Garrison-Specific Muzzle Flash Anim

**gamemd:** In `Fire_At` (report §12c), when IsOccupied:
```
muzzle_flash_anim = WeaponType+0x110  // Garrison muzzle flash anim type
// (Instead of standard WeaponType+0x104)
```

**Our render code:** When processing `SimFireEvent` with `garrison_muzzle_index: Some(idx)`:
1. Look up `art_entry.muzzle_flash_positions[idx]` → screen-space (X, Y)
2. Position flash at `building_screen_pos + (X, Y)`
3. Z-offset = -200 (draw in front of building)

---

## 8. Turret Suppression

**gamemd:** Report 129 — VXL turret only renders if `GarrisonAnim` field == 0.
When garrisoned, the turret is hidden.

**File:** VXL turret render code in `app_instances/`

Skip turret for buildings where `cargo()` is non-empty and `can_be_occupied`.
Minor visual — most garrisonable buildings don't have turrets.

---

## 9. Ejection on Destruction

**gamemd:** `BuildingClass::SellBuilding` (0x00457DE0, report §14c)

Full flow:
1. Reset `CurrentFireIdx` to 0
2. Search foundation edges in gamemd order: east column SE->NE, south row
   SE->SW, north row west->east, then west row north->south. The probe uses
   occupant slot 0 only via `Can_Enter_Cell(cell,-1,-1,0,1)`.
3. If ALL edges fail, behavior is caller-argument dependent: destruction/red-HP
   callers pass zero and take `SpawnUnitsWithParachute(0)`'s null remove branch;
   normal player sell passes nonzero and uses an inside-foundation fallback.
4. Select one coordinate once, then iterate occupants BACKWARDS (LIFO,
   high-to-low index)
5. For each: try `Unlimbo(exitCoords)`
   - Success: clear archive target, call occupant Scatter with building coord
   - Failure: DESTROY/REMOVE infantry via vtable `+0xF8`
6. Clear occupant vector, recalculate power

**Key details:**
- Backwards iteration (LIFO), not FIFO
- One selected coordinate is reused for every occupant; do not rescan per occupant
- No normal parachute visual fallback for destruction/red-HP no-exit; the null
  branch removes occupants
- Infantry Scatter uses scenario `RandomRanged(0,4)` after Scatter gates, not
  immediate raw `% 8`
- The later mission `0xF` block exists but is first-argument gated and was not
  active for the direct callers checked in the 2026-05-27 garrison swarm
- Occupants are NOT harmed by building damage — only by destruction

**File:** `sim/combat/mod.rs` → death handling (line ~341)

For `CanBeOccupied` buildings: unlimbo passengers to adjacent cells (reuse
`tick_unloading` exit cell search). Kill if no exit cell.
Reset `garrison_fire_index` to 0.

---

## 10. Sound & EVA

| Event | gamemd | Our code | Location |
|-------|--------|----------|----------|
| First occupant enters | `EVA_StructureGarrisoned` + `BuildingGarrisonedSound` | Not implemented | `tick_boarding()`, if `count == 1` after board |
| Last occupant leaves | `EVA_StructureAbandoned` | Not implemented | `tick_unloading()`, when cargo empty |
| Garrison fire | Weapon report at building position | Automatic via existing `SimSoundEvent::WeaponFired` | Combat tick |

---

## 11. Eligibility Corrections (from CanDock decompilation)

**gamemd:** `BuildingClass::CanDock` (0x00457CE0, report §13a)

Details our `can_enter_transport()` is missing:

| Check | gamemd | Our code | Priority |
|-------|--------|----------|----------|
| `CanBeOccupied` | Yes | Yes | — |
| Mission ≠ Construction (0x12) / Selling (0x13) | Yes | No | P3 |
| Building in map bounds + visible | Yes | No | P3 |
| Not being warped/chrono'd (`FUN_007105e0`: +0x2C0/+0x2C4) | Yes | No | P3 |
| `MultiplayPassive` on house's CountryType (+0x34→+0x1A6) | Yes | Hardcoded "neutral"/"special" | P3 |
| Count < `MaxNumberOccupants` (+0x1580) | Yes | Yes (via cargo capacity) | — |
| Not at red HP | Yes | Yes | — |
| `Occupier` flag | Yes | Yes | — |
| `Assaulter` path (not allied + building HAS occupants) | Yes | Not implemented | P3 |

The `Assaulter` path is specifically for clearing enemy garrisons — infantry
must NOT be allied with the building's owner, and the building must already
have occupants inside. This is separate from normal garrison entry.

---

## 12. Architecture Constraints

### Sim/render boundary
All garrison combat logic in `sim/`. Fire events flow to render via `SimFireEvent`.
**sim/ must NEVER depend on render/.**

### Determinism
- Damage multiply uses `f64` (matching original's FPU floats) — acceptable since
  the original also uses float for this single operation, not sim-critical fixed-point
- ROF division is integer — deterministic
- Round-robin index stored in `PassengerCargo`, advanced in sorted entity order
- `BTreeMap` iteration = sorted by stable_id = deterministic

### Snapshot-then-mutate pattern
Combat uses read-only snapshots in Phase 1, applies mutations in Phase 3.
Garrison snapshots must follow the same pattern. Never mutate entities while iterating.

### Fire event pipeline
`SimFireEvent` → `sim.fire_events` → `app_sim_tick.rs:271` drains to
`state.pending_fire_effects` → render consumes. Add `garrison_muzzle_index`
field; render checks it to position muzzle flash at fire port vs FLH.

### Foundation size for range
`GetHalfFoundationSize()` = `min(width, height) / 2` (integer division).
Foundation dimensions from art.ini. Used in both scan range and fire range formulas.
Need access to foundation data in the combat tick — may need to add foundation
dimensions to `ObjectType` or look up from `ArtRegistry` during combat.

---

## 13. Implementation Priority

| Step | What | Complexity | Prereqs |
|------|------|-----------|---------|
| **1** | Data model (`garrison_fire_index`, `SimFireEvent` field) | ~10 lines | None |
| **2** | Fire gating (empty garrison cannot fire) | ~10 lines | None |
| **3** | Garrison weapon selection function | ~40 lines | None |
| **4** | Auto-target acquisition for garrisoned buildings | ~40 lines | Step 3 |
| **5** | Garrison combat in tick_combat_with_fog | ~150 lines | Steps 1-4 |
| **6** | Muzzle flash at fire ports | ~40 lines | Step 5 |
| **7** | Turret suppression | ~5 lines | None |
| **8** | Eject on destruction | ~30 lines | None |
| **9** | Sound/EVA | ~20 lines | Step 5 |
| **10** | Kill credit to occupant | ~20 lines | Step 5 |

Steps 1-5 are the critical path for playable garrison combat (~250 lines).
Steps 6-10 are polish (~115 lines).

---

*All gamemd function addresses, offset values, and formulas verified via live Ghidra
decompilation. See `GARRISON_SYSTEM_GHIDRA_REPORT.md` for full decompiled code and
confidence assessments.*
