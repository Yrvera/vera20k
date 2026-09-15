# Jumpjet flight: native facts behind the cruise port

Binary: YR 1.001 `gamemd.exe`, SHA-256 `1cdd1180…4298c`. Read 2026-09-15/16 from
disassembly; the cruise body is **parity demonstrated** for the scope of
`tools/spatial_oracle/jumpjet_flight.json` (see its `.meta.json`). Everything
else here is **native behaviour established** from bodies and callers, or marked
otherwise. Rust owners: `src/sim/movement/jumpjet_flight.rs` (kernel) and
`src/sim/world/jumpjet_cruise.rs` (production host).

## Process model

- `JumpjetLocomotionClass::Process @ 0x0054AEC0` calls
  `Update_Coordinates_And_Altitude @ 0x0054D0F0` only while the interface's
  `Is_Moving` (+0x10) or `Is_Moving_Now` (+0x80) answers true, then dispatches
  the state at receiver `+0x50`: 0 ground `0x0054B980`, 1 ascend `0x0054BA30`,
  2 hold `0x0054BD30`, 3 translate `0x0054BFF0`, 4 descend `0x0054C550`,
  5 touchdown/crash `0x0054CA90`.
- Receiver-relative fields: `+0x1C..+0x3C` type block, `+0x40` destination XYZ,
  `+0x4C` moving byte, `+0x50` state, `+0x54` the locomotor's own FacingClass,
  `+0x70` current speed (double), `+0x78` target speed (double), `+0x80` target
  height (int), `+0x88` bob phase (double).

## Link and facing

- `Link_To_Object @ 0x0054AD30` (COM, `RET 8`) copies `TechnoTypeClass+0xD70..
  +0xD90`: turn rate, speed (int), climb, crash, height floored at two cell
  levels (`0x0054AD9B`), accel, wobbles, deviation, no-wobbles (`+0xD8C`). It then
  rebuilds `+0x54` with `FUN_004C91E0(type JumpjetTurnRate)` and snaps it to
  `0x4000`. The constructor's `[JumpjetControls] TurnRate` (Rules `+0x40C`)
  facing is therefore replaced; the body turns at the **type's**
  `JumpjetTurnRate=`. Stock misspells that key in every jumpjet section, so all
  stock jumpjets turn at the constructor's 4.
- State 0 (`0x0054B980`) snaps the locomotor facing to the body facing
  (`+0x388`) at takeoff; every Update copies the locomotor facing back to the
  body (`0x0054D67B..D692`), except Infantry with `JumpJetTurn=`
  (`InfantryTypeClass+0xECB`, key at `0x008258FC`) holding a target in state 2,
  which faces the target instead.

## Update (0x0054D0F0)

1. Speed ramp: if target > current, current += accel, capped at speed; then if
   target < current, current -= 1.5 × accel, floored at 0. The second test reads
   the updated value, so a target below the cap overshoots and is pulled back in
   the same frame. `SetSpeedFraction(current / speed)`.
2. Bob: states 2 and 3 without no-wobbles add `2pi / (15 / wobbles)`; otherwise
   the phase is zeroed. Target height `T = ftol(sin(bob) × deviation + +0x80)`.
3. Ground `G` = floor height at the owner; on a high-bridge cell, `G += 416`
   once `Z >= G + 4 levels − crash`.
4. Reference `R`: states 0/4 use `G`; otherwise `0x0054D820` unless the owner is
   in the destination cell without `BalloonHover=`. `0x0054D820` samples the
   current cell's top (`CellClass 0x00485080`: centre ground, plus the first
   building's `Dimension2` height, art `Height=` at `+0xEF4` times
   `g_HeightFactor` = 104 from the startup chain `0x0045AFA0..0x0045B070`
   (`tools/spatial_oracle/height_factor.json`), or 85 for any other techno) and,
   while moving,
   the cell one step ahead along the facing (direction deltas at `0x0089F6D8`,
   filled by the CRT initializer `0x0049F3A0`); it returns the ahead value when
   higher, else the average. Both of its bridge tests add the deck to the
   *current* sample.
5. Height: `A = Z − R`. Below `T`: climb by `JumpjetClimb=` or snap to `T`
   (with a grounded reset through vtable `+0xF4` at height 0). Above `T`: descend
   by climb or snap, never below `G`.
6. Outside the destination cell, `A < T/2` or `A < T/4` zeroes the speed.
7. XY step along `Current(+0x54)`: `Y −= sin × trunc(speed)`,
   `X += cos × trunc(speed)`, committed through `SetLocation`.

## State3_Translate (0x0054BFF0)

- Desired facing `ftol((atan2(curY−destY, destX−curX) − pi/2) × −65536/2pi)`,
  set gradually. Turn error `((diff16 >> 7) + 1) >> 1 & 0xFF` over the unsigned
  destination-minus-current difference: a quarter turn left reads 192, right 64.
- Distance `ftol(Sqrt_Approx(dx² + dy²))`. Arrival below 20 leptons: zero both
  speeds, snap XY to the destination, then state 4 (+0x80 = 0) for an ordinary
  owner; a `BalloonHover=` owner or one with a target claims the cell's air slot
  and holds (state 2) or scatters to a random neighbour when the slot is taken;
  a Unit with `IsSimpleDeployer=` (`UnitTypeClass+0xE13`) and `DeployToLand=`
  (`TechnoTypeClass+0x6AD`) holds at full height. The arrival notify `0x00705D60`
  returns early unless `TechnoClass+0x514` (a planning-code manager) is set.
- Zones with `S = JumpjetSpeed`: `d < S` → S/8, height/2; `d < 2S` → S/4,
  height/2, S/10 (floor 1.0) when the error exceeds the turn rate;
  `d < 50S/turn rate` → S/2, height × 0.75, S/5 (floor 1.0) when the error exceeds
  5 × turn rate; else S at full height. Height writes are skipped with a target.
- Tail: `BalloonHover=`, a Water or Beach destination (LandType 2/6), or a Unit
  with `DeployToLand=` keeps full height.

## Numeric model

`WinMain` calls `_controlfp(0x300, 0x300)` (`0x006BBFB7..BFC1`) and
`0x007C5EE4` captures the word at `0x00822D80`; `Math::ftol @ 0x007C5F00`
reloads it without restoring. The process runs x87 at 53-bit precision rounding
toward zero, which the oracle confirms: truncating replay matches every bob sum,
round-to-nearest diverges after 13–14 frames. Sine and cosine read the table at
`0x0084F084` with the binary32 scale at `0x008223B0` (`0x4522F983`), the
arctangent reads the 4097-entry table at `0x008610B4` (step `0x3CC7FE84`,
FNV-1a `4056c36f7f1eab9c`), and the distance uses the `Sqrt_Approx` LUT.

## Production hand-off

The adapter's takeoff stands in for State 0, so entering the cruise from any
state other than 3 or 2 applies State 0's seed: the locomotor facing snaps to
the body facing, both speed doubles and the bob phase are zeroed, and `+0x80`
takes `JumpjetHeight=`. A re-order from a hold keeps them. A cruise whose move
target drops leaves state 3 for a stopped hold; native `Stop_Moving` would
re-target a nearby cell and keep flying.

## Not yet ported

States 0, 1, 2 and 4 (takeoff translation while climbing, idle hold and bob,
descent and landing admission), state 5 crash, the cell `AltObject` air slot
(`0x004135A0` query, `0x00487D70` set/clear), `JumpJetTurn=`, the planning-token
notify and the grounded reset's `+0xF4` target. The production host hands those
back to VERA's air adapter.
