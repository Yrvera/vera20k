# Engineer Capture Mechanics — Ghidra Analysis of gamemd.exe

**Source:** Live Ghidra decompilation of `gamemd.exe` (YR 1.001)
**Confidence:** HIGH for capture flow (decompiled from Mission_Capture at 0x5202f0).
MEDIUM for MultiEngineer (runtime global not found in gameplay code — see analysis).

---

## Overview

Engineers are infantry with `Engineer=yes` in rules.ini. When ordered to enter an
enemy building with `Capturable=yes`, they walk to it, instantly transfer ownership,
and are consumed. The entire process is one function: `InfantryClass::Mission_Capture`
at 0x5202f0.

There is NO damage-based capture in vanilla YR. Engineers always capture instantly.
The `MultiEngineer` game option was inherited from Tiberian Sun and is marked
"DESUPPORTED" in the INI comments. The `EngineerDamage` key does NOT exist in the
vanilla binary — it's an Ares mod extension.

---

## 1. InfantryClass::Mission_Capture (0x5202f0)

**Size:** ~1000 bytes. **Labeled in Ghidra.**

### Preconditions

```
1. Check TypeClass+0xEC5 != 0       — infantry has Engineer=yes flag
2. Check this+0x5A4 != NULL         — has a target assigned
3. Check target->GetRTTI() == 1     — target is a BuildingClass
4. Check !IsAlliedWith(target)      — target is an ENEMY building
```

If any check fails → return 0 (do nothing).

### Distance Check

```
distance = Distance3D(infantry.GetCoords(), building.GetCoords())
if distance >= 0x80:    → too far, keep moving
if distance < 0x80:     → close enough to capture (128 leptons ≈ 0.5 cells)
```

If too far but within 0x200 (512 leptons ≈ 2 cells), the infantry issues a
movement order toward the building's dock point and retries next tick.

### Capture Sequence (distance < 0x80)

```c
// 1. If building is an active target of another unit, clear that targeting
if (building->field_0xD != 0) {
    TechnoClass::ProcessCellAction(1, infantry, DAT_00a8f1e0, 0, 0);
}

// 2. Set building mission to Guard (3)
building->vtable_0x274(3);

// 3. Limbo the building temporarily (hides from game world briefly)
building->vtable_0xDC(0);

// 4. Notify house of capture event (if human, play EVA)
if (infantry->HouseIndex != 0) {
    if (FUN_006e57c0()) {          // check if EVA system active
        FUN_005f5b50(infantry->HouseIndex);  // "EVA_StructureCaptured"
    }
}

// 5. TRANSFER OWNERSHIP — the core call
building->SetOwner(infantry->Owner, 1);  // vtable+0x3D4 = ChangeOwner
// param_2 = 1 means "announce" (EVA notification)

// 6. Set building type tag from engineer's type data
building->field_0xCE = infantry->TypeClass+0xDF8;

// 7. DESTROY THE ENGINEER — consumed on capture
infantry->vtable_0xF8();  // Destroy/Delete
```

### Key Observations

- **Instant capture** — no health check, no damage, no gradual process
- **Engineer is destroyed** — the infantry is consumed upon successful capture
- **Single function** — no separate "can capture" validation beyond the 4 preconditions
- **Building is limboed** — briefly hidden during the ownership swap, then un-limboed
- **ChangeOwner does the heavy lifting** — updates power, tracking lists, radar, EVA,
  sidebar, base center, wall connections (see BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md)

---

## 2. MultiEngineer Game Option

### INI Definition

```ini
[MultiplayerDialogSettings]
MultiEngineer=no  ; "DESUPPORTED" according to INI comments
```

### Runtime Global

`DAT_00a8b26c` — cached from RulesClass+0x14B4 at scenario load.

### Where It's Read

| File/Context | Usage |
|---|---|
| File 057 (init) | Copied from Rules+0x14B4 to global |
| File 075 (lobby UI) | Read for checkbox state, encoded as bit 7 of network byte 0x8E |
| File 076 (lobby UI) | Checkbox labeled "Crap Engineers" (item 0x551) |
| File 115 (skirmish setup) | Reset to 0 for skirmish |
| **Gameplay code** | **NOT FOUND** — no file in the 0x40xxxx-0x55xxxx range reads DAT_00a8b26c |

### What This Means

**MultiEngineer is parsed and displayed in the lobby but has NO runtime gameplay
effect in vanilla YR.** The INI comments confirm this — "DESUPPORTED". When
enabled in the lobby, the checkbox shows as "Crap Engineers" but the capture
logic at Mission_Capture (0x5202f0) does not check this flag at all.

In Tiberian Sun, MultiEngineer made engineers only capture buildings below a
health threshold (`EngineerCaptureLevel`). This mechanic was removed/disabled
in RA2/YR — the code to check it no longer exists in the capture path.

---

## 3. EngineerCaptureLevel (Rules+0x17F8)

### INI Parsing

```ini
[General]
EngineerCaptureLevel=0.xxx   ; float, parsed into Rules+0x17F8 and Rules+0x17FC
```

This value IS parsed from rules.ini into RulesClass at two float offsets:
- `Rules+0x17F8` — first EngineerCaptureLevel
- `Rules+0x17FC` — second EngineerCaptureLevel (same key read twice — likely
  neutral vs enemy distinction)

### Runtime Usage

The value is stored on RulesClass but **no gameplay code reads Rules+0x17F8
or Rules+0x17FC during runtime** based on search of all 172 decompiled files.
It appears to be a vestigial Tiberian Sun field that's parsed but never consumed.

---

## 4. Engineer Repair (friendly building)

When an engineer enters a **damaged friendly building**, instead of capturing it
the engineer repairs it to full health and is consumed. This is handled in the
cursor logic (app_cursor.rs already implements the cursor feedback) but the
actual repair action is a separate mission — `InfantryClass::Mission_Enter`
(0x5196A0), which checks if the target is allied and the building is damaged.

---

## 5. Related Systems

### C4 / Sabotage (Tanya, Crazy Ivan)

Infantry with `C4=yes` can plant explosives on buildings. This is a separate
mission (`Mission_Sabotage`) that does NOT transfer ownership — it deals a
large amount of damage via the `C4Warhead` from [CombatDamage]. Tanya and
Navy SEAL use `SabotageCursor=yes` for the cursor feedback.

### Spy Infiltration

Infantry with `Infiltrate=yes` enters buildings for special effects (steal
tech, steal money, reset superweapons, power sabotage) via
`BuildingClass::OnSpyInfiltrate` (0x4571E0). This is separate from engineer
capture — the spy is NOT consumed and the building is NOT transferred.

### Mind Control (Yuri Prime)

Mind control uses `SetOwner` with a stored `OldOwner` reference, enabling
`ReclaimUnitsFrom` if the controller dies. Engineers don't use this mechanism —
they call `SetOwner` without storing the old owner, making capture permanent.

---

## 6. INI Keys Reference

### On Infantry Types

| Key | Purpose | Example |
|-----|---------|---------|
| `Engineer=yes` | Unit behaves as engineer (capture + repair) | [ENGINEER], [SENGINEER] |
| `Infiltrate=yes` | Unit behaves as spy | [SPY] |
| `C4=yes` | Unit can plant demolition charges | [TANYA], [SEAL] |
| `SabotageCursor=yes` | Shows sabotage cursor instead of attack | [TANYA] |

### On Building Types

| Key | Purpose |
|-----|---------|
| `Capturable=yes` | Building can be captured by engineers |
| `CanBeOccupied=yes` | Building can be garrisoned (separate from capture) |

### On [General]

| Key | Offset | Status in YR |
|-----|--------|-------------|
| `EngineerCaptureLevel=` | Rules+0x17F8/17FC | **Vestigial** — parsed but never read |

### On [MultiplayerDialogSettings]

| Key | Offset | Status in YR |
|-----|--------|-------------|
| `MultiEngineer=` | Rules+0x14B4 | **Desupported** — UI only, no gameplay effect |

---

## 7. Implementation Guide for ra2-rust-game

### What to implement

1. **Engineer capture command** — when an engineer is ordered to enter a
   capturable enemy building:
   - Move engineer toward building
   - When within ~0.5 cells (128 leptons):
     - Change `building.owner` to `engineer.owner`
     - Update HouseState owned counts for both old and new owner
     - Remove engineer entity (consumed)
   - No health check needed (instant capture always)

2. **Engineer repair** — when ordered to enter a damaged friendly building:
   - Move engineer toward building
   - When close enough:
     - Set `building.health.current = building.health.max`
     - Remove engineer entity (consumed)

3. **Cursor feedback** — already implemented in app_cursor.rs

### What NOT to implement

- `MultiEngineer` health check — desupported, no gameplay effect in vanilla YR
- `EngineerCaptureLevel` — vestigial TS field, never read
- `EngineerDamage` — does not exist in vanilla, it's an Ares mod extension
- C4/Sabotage — separate system, implement later
- Spy infiltration — separate system, implement later

### Key functions to create

```rust
// In sim command handler:
Command::CaptureBuilding { engineer_id, building_id } => {
    // Validate: engineer has Engineer=yes, building has Capturable=yes
    // Validate: building is enemy (not allied)
    // Set engineer movement target to building position
    // In movement/mission tick: check distance < capture_range
    // On arrival: transfer ownership, destroy engineer
}
```

The ChangeOwner equivalent in our engine is:
```rust
fn change_building_owner(sim: &mut Simulation, building_id: u64, new_owner: &str) {
    if let Some(entity) = sim.entities.get_mut(building_id) {
        let old_owner = entity.owner.clone();
        entity.owner = new_owner.to_string();
        // Update owned counts
        sim.decrement_owned_count(&old_owner, EntityCategory::Structure);
        sim.increment_owned_count(new_owner, EntityCategory::Structure);
        // Power will be recalculated next tick automatically
    }
}
```
