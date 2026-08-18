# FootClass `+0x388` Locomotor-Facing Timer -- Ghidra Report

**Date:** 2026-07-18  
**Status:** COMPLETE for the failed-A* retry consumer, initialization, and active Unit locomotor writers examined here  
**Active in YR:** Yes

## Question

The failed-A* retry plan needs the two `FacingClass::Current` values compared by
`UnitClass::Can_Enter_Cell`. Existing research called `TechnoClass+0x388`
`TurretFacing`, while current Rust has only an 8-bit body `facing` plus a separate
timer-backed `barrel_facing`. This pass determines which Rust state must supply the
retry comparison.

## Verified Findings

1. `UnitClass::Can_Enter_Cell @ 0x0073F0A0` compares the animated values of
   `blocker+0x388` and `self+0x388` in the moving-allied deadlock branch. The two
   receivers are proven by disassembly at `0x0073F8E5..0x0073F90B`, which executes
   `LEA ECX,[EBX+0x388]`, calls `FacingClass::Current @ 0x004C93D0`, then repeats
   the call with `LEA ECX,[ESI+0x388]`.
2. `DriveLocomotionClass::Do_Turn @ 0x004B0EF0` is a direct writer of the linked
   object's `+0x388` timer. Disassembly `0x004B0EF0..0x004B0F0F` loads the linked
   object from locomotor `+0x8`, adds `0x388`, and calls
   `FacingClass::Set @ 0x004C9220` with the requested 16-bit facing.
3. `HoverLocomotionClass::SpeedUpdate @ 0x00515ED0` also writes the linked object's
   `+0x388`. At `0x00516292..0x005162B1` it computes a 16-bit facing, loads the
   linked object from hover locomotor `+0xC`, adds `0x388`, and calls
   `FacingClass::Set @ 0x004C9220`. The same function's internal hover-facing timer
   at locomotor `+0x30` is separate.
4. `TechnoClass::Unlimbo @ 0x006F6CA0` initializes object `+0x388` from the reveal
   facing with the snap variant `FacingClass::UpdateFacing @ 0x004C9300`.
   Disassembly `0x006F6D94..0x006F6DAA` forms the 16-bit facing and passes
   `ESI+0x388` as the receiver.
5. `UnitClass::Constructor @ 0x007353C0` treats `+0x388` and `+0x3A0` as distinct.
   At `0x007354AE..0x007354C9` it samples `Current(+0x388)` and snaps `+0x3A0` to
   that value. At `0x00735570..0x0073558D` it applies the type's `ROT` value to
   both timers separately.
6. `UnitClass::Facing_Update @ 0x00736990` does not make `+0x388` an alias of
   `+0x3A0`. Its assembly reads `Current(+0x388)` at
   `0x00736A09..0x00736A20` and `0x00736BCA..0x00736BDD` to choose targets for
   `+0x3A0`; other branches write `+0x3A0` directly. This is synchronization by
   explicit reads/writes, not shared storage.

## Corrected Interpretation

For the failed-A* retry and movement consumers, `Foot/Techno+0x388` is the
timer-backed **locomotor/body-facing state**. The old generic `TurretFacing` label
is unsafe for this field: active Drive and Hover locomotors write it, Unlimbo seeds
it from the object's reveal facing, and Unit cell-entry deadlock logic compares it.

`+0x3A0` remains a distinct Unit aiming/barrel timer. Current Rust
`GameEntity::barrel_facing` is therefore not a valid substitute for `+0x388`.

The semantic name should remain conservative in Rust, for example
`locomotor_facing`, because this pass did not reclassify every neighboring
TechnoClass facing field or every non-Unit rendering consumer.

## Current Rust Delta

- `GameEntity::facing: u8` stores only a quantized current body direction.
- `GameEntity::facing_target: Option<u8>` plus
  `movement_step::handle_vehicle_rotation` advances that byte by a per-tick delta.
- `GameEntity::barrel_facing: Option<FacingClass>` is the separate aiming timer and
  cannot supply native `+0x388`.
- `FacingClass` already implements the required 16-bit timer primitive, but no
  `GameEntity` field currently owns the locomotor-facing instance.

Therefore the retry oracle cannot reconstruct the native equality test from the
ordered occupant list or from `barrel_facing`. It needs an authoritative
timer-backed locomotor-facing field, initialized and retargeted through the same
spawn and movement ownership boundaries that currently write `facing` or
`facing_target`. The 8-bit `facing` may remain a compatibility/render projection,
but it must not be the retry oracle's authority.

## Implementation Handoff

1. Add one serialized and hashed `FacingClass` instance to `GameEntity` for the
   native `+0x388` role; initialize it from `facing << 8` and the object's parsed
   `ROT`.
2. Centralize snap/retarget/current-projection operations so spawn, in-place turns,
   Drive/track, Hover/Ship/other active locomotor paths cannot update the 8-bit
   projection without updating the timer.
3. Make the failed-A* retry oracle compare
   `locomotor_facing.current(binary_frame)` for mover and blocker. Do not compare
   `barrel_facing`, `facing_target`, or only the high byte.
4. Bump the Rust snapshot version and include the new timer in the deterministic
   state hash. The timer is durable entity state, unlike CellClass occupancy caches.
5. Acceptance must include two facings with the same 8-bit high byte but different
   16-bit animated values; native equality is false and a high-byte-only comparison
   would be wrong.

## Negative Facts

- Do not call `+0x388` the retry's turret-aim state.
- Do not reuse Rust `barrel_facing` for `+0x388`.
- Do not derive the native equality from current direction, target direction, or
  movement-path direction alone.
- Do not serialize the per-cell occupancy shadow merely because the entity-facing
  timer is serialized; gamemd rebuilds CellClass occupancy attributes through
  object Unlimbo order.

## Sources

Live read-only Ghidra MCP calls against `gamemd.exe` on 2026-07-18:

- `disassemble_function(0x00736990)` -- `UnitClass::Facing_Update`
- `decompile_function(0x00736990)` -- branch-level `+0x388/+0x3A0` use
- `disassemble_function(0x007353C0)` -- `UnitClass::Constructor`
- `decompile_function(0x007353C0)` -- constructor flow
- `disassemble_function(0x004B0EF0)` -- `DriveLocomotionClass::Do_Turn`
- `disassemble_function(0x00515ED0)` -- `HoverLocomotionClass::SpeedUpdate`
- `disassemble_function(0x006F6CA0)` -- `TechnoClass::Unlimbo`
- `get_xrefs_to(0x004C9220)` -- active `FacingClass::Set` callers
- `get_xrefs_to(0x004C93D0)` -- active `FacingClass::Current` consumers, including
  `UnitClass::Can_Enter_Cell @ 0x0073F8EB/0x0073F906`

