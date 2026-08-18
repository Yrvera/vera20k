# Garrison Combat Implementation — Session Prompt

## Context

We've completed extensive reverse engineering of the garrison system from gamemd.exe
via Ghidra and audited all existing garrison code in this Rust engine. Two comprehensive
docs exist:

- `docs/GARRISON_SYSTEM_GHIDRA_REPORT.md` — Full RE report with 16 sections, 22 decompiled
  functions, verified formulas, confidence assessments. This is the ground truth for how
  gamemd.exe implements garrison.

- `docs/GARRISON_IMPLEMENTATION_PLAN.md` — Maps every gamemd behavior to exact file/line
  in this codebase. Has code sketches, priority ordering, and architecture constraints.

## What's already implemented

- Cursor/action (Enter cursor for Occupier infantry on CanBeOccupied buildings)
- Boarding (pathfind → arrive → board via PassengerRole::Boarding/Inside)
- Ownership transfer (neutral building → infantry's owner on entry, revert on empty)
- Occupant pips (pips.shp frames 6-12)
- ActiveAnimGarrisoned (loops while building has occupants)
- All INI keys parsed (OccupyWeapon, OccupyDamageMultiplier, OccupyROFMultiplier,
  OccupyWeaponRange, MuzzleFlash positions, etc.) — but combat values are UNUSED

## What needs implementing

**The building cannot fire.** Garrison is visual-only right now. The entire combat
system needs garrison awareness. The implementation plan has 10 steps:

1. Data model (garrison_fire_index on PassengerCargo, garrison_muzzle_index on SimFireEvent)
2. Fire gating (empty CanBeOccupied building cannot fire — even with own weapons)
3. Garrison weapon selection (OccupyWeapon → EliteOccupyWeapon → primary fallback)
4. Auto-target acquisition (scan range = halfFoundation + 1 + OccupyWeaponRange)
5. **Garrison combat in tick_combat_with_fog** (the big one — range/damage/ROF/round-robin)
6. Muzzle flash at fire ports (MuzzleFlash pixel offsets from art.ini)
7. Turret suppression when garrisoned
8. Eject on destruction (instead of kill — LIFO, scatter, parachute fallback)
9. Sound/EVA (EVA_StructureGarrisoned, EVA_StructureAbandoned)
10. Kill credit to occupant infantry (not building)

## Task

Read `docs/GARRISON_IMPLEMENTATION_PLAN.md` fully first. Then enter plan mode and
create a step-by-step implementation plan. Start with steps 1-5 (critical path for
playable garrison combat, ~250 lines). Follow the exact gamemd formulas documented
in the plan — do not improvise or simplify the math. Key formulas:

- **IsOccupied:** `can_be_occupied AND can_occupy_fire AND cargo.count() > 0`
- **Range:** `(half_foundation + occupy_weapon_range) * 256` leptons
- **Damage:** `base_damage * verses / 100 * occupy_damage_multiplier` (float multiply, truncate)
- **ROF:** `rof_to_cooldown_ticks(weapon.rof) / occupant_count / occupy_rof_multiplier`
- **Round-robin:** `garrison_fire_index = (idx + 1) % count` after each shot
- **Weapon:** occupant's OccupyWeapon (elite variant if veteran), fallback to primary

The combat code is in `sim/combat/mod.rs` (tick_combat_with_fog, ~800 lines, 7 phases).
Weapon selection is in `sim/combat/combat_weapon.rs`. Fire gating in `combat_fire_gate.rs`.
Auto-targeting in `combat_targeting.rs`. All garrison rules in `rules/ruleset.rs` (GarrisonRules).
