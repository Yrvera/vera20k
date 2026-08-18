# Implementation Trace: MCV Deploy over Mixed Terrain Heights

Date: 2026-05-21

Scope: one concrete implementation verification only. An Allied `AMCV` deploys into
`GACNST` with the resulting foundation spanning mixed terrain heights, no
structures, no terrain blockers, and the normal `Command::DeployMcv` path.

Concrete fixture:

- Unit/building data: retail YR `AMCV -> GACNST`.
- INI evidence: `ini/rulesmd.ini` has `[AMCV] DeploysInto=GACNST`;
  `ini/artmd.ini` has `[GACNST] Foundation=4x4`.
- Rust MCV cell: `(20,22)`.
- Rust `GACNST` foundation origin: `(19,21)` from `deploy_origin_from_center`
  subtracting `1` for foundations larger than `2x2`.
- Mixed-height footprint used for the decision trace: `(19,21)=0`,
  `(20,21)=1`, all other `4x4` foundation cells otherwise clear, in bounds,
  unoccupied, and buildable.
- Verdict rule: `PASS` requires literal numerical equality between Rust and
  gamemd outputs. If both numbers were not computed, the stage is `UNCHECKED`.

## Pipeline

Deploy command -> ownership/rules gate -> target building lookup -> foundation
origin -> footprint validation -> MCV removal/building spawn -> visible result.

## Entry Points Covered

- Player/AI deploy command in Rust: `Command::DeployMcv`.
- Sim command dispatch: `Simulation::apply_command` in
  `src/sim/world/world_commands.rs`.
- MCV conversion implementation: `Simulation::deploy_mcv` in
  `src/sim/world/world_spawn.rs`.
- gamemd active standard YR path: `UnitClass__Deploy @ 0x007393c0`.
- gamemd placement gate: `BuildingTypeClass__CanBePlacedAt @ 0x0045ee70`.
- gamemd placement commit: `BuildingClass__Unlimbo @ 0x00440580`.

## Stage Trace

### Stage 1 - Deploy Command Dispatch

Rust:

- `src/sim/command.rs:58-59` defines `Command::DeployMcv { entity_id }`.
- `src/sim/world/world_commands.rs:477-482` requires rules, verifies the
  command owner owns the entity, then calls `deploy_mcv`.
- Concrete output: if `entity_owned_by_id("Americans", mcv)` is true, the
  command reaches `deploy_mcv(mcv, rules, height_map)`.

gamemd:

- `MCV_DEPLOY_GHIDRA_REPORT.md` identifies standard deploy input as the D key
  or deploy button sending event type `0x1E`.
- Active decompile of `UnitClass__Deploy @ 0x007393c0` shows the conversion
  body used for MCV deploy. This is not a dormant TS-only path.

Verdict: `UNCHECKED` - both paths reach their deploy handler, but the exact
network command tick/value for the same player input was not numerically
computed on both engines.

### Stage 2 - Deploy Target and Foundation Data

Rust:

- `src/sim/world/world_spawn.rs:502-519` resolves the source type, gets
  `DeploysInto`, gets the target object, and stores the target foundation.
- `src/rules/foundation.rs:126-130` maps `4x4` to width `4`, height `4`.
- Concrete output: `yard_type="GACNST"`, foundation `4x4`, `(fw,fh)=(4,4)`.

gamemd:

- Retail INI data gives `[AMCV] DeploysInto=GACNST` and `[GACNST] Foundation=4x4`.
- Active decompile of `UnitClass__Deploy @ 0x007393c0` reads the deploy target
  from the unit type and calls `BuildingTypeClass__GetFoundationWidth`; for
  large foundations it also checks foundation height.
- Existing trace evidence records `BuildingTypeClass::GetFoundationWidth` and
  `GetFoundationHeight` reading the original foundation dimension tables.

Verdict: `PASS` - both sides use `GACNST` with a `4x4` footprint.

### Stage 3 - MCV Cell to Building Origin

Rust:

- `src/sim/world/world_spawn.rs:506-510` calls `deploy_origin_from_center`.
- `src/sim/world/world_spawn.rs:672-678` returns
  `(center_rx.saturating_sub(1), center_ry.saturating_sub(1))` for any
  foundation wider than `2` or taller than `2`.
- Concrete output: MCV `(20,22)` -> `GACNST` origin `(19,21)`.

gamemd:

- Active decompile of `UnitClass__Deploy @ 0x007393c0` calls a virtual that
  writes a cell-like value into the local coordinate buffer, then builds
  `CoordStruct(cell_x * 256 + 128, cell_y * 256 + 128, 0)` before calling the
  new building's `vtable+0xD8`.
- I did not compute the exact virtual-returned cell for an MCV at `(20,22)`.

Verdict: `UNCHECKED` - Rust origin is numerically computed as `(19,21)`, but
the matching gamemd origin for this same MCV cell was not computed.

### Stage 4 - Footprint Occupancy and Terrain Blockers

Rust:

- `src/sim/world/world_spawn.rs:525-560` iterates `dy in 0..fh`,
  `dx in 0..fw`, checks for existing structures except the deploying MCV,
  and checks `effective_build_blocked`.
- Concrete fixture has no structures and no build-blocked terrain.
- Concrete output through this stage: no rejection before the height check.

gamemd:

- Active decompile of `BuildingTypeClass__CanBePlacedAt @ 0x0045ee70` walks
  `GetFoundation(1)` offsets, checks in-bounds cells, overlay/building content,
  terrain-object content, upgrade placement, allied movable overlap/scatter, and
  returns a placement result.
- For the concrete fixture with no occupants and no terrain blockers, no
  rejection branch from those checks was identified.

Verdict: `UNCHECKED` - the no-blocker fixture is explicit on the Rust side, but
the exact gamemd per-cell result code for every `4x4` cell was not computed as
literal numbers.

### Stage 5 - Mixed-Height Footprint Validation

Rust:

- Current implementation still contains a same-height gate:
  `src/sim/world/world_spawn.rs:527` sets
  `ref_height = height_map[(rx,ry)].unwrap_or(z)`.
- `src/sim/world/world_spawn.rs:562-568` rejects if any footprint cell's
  height differs from `ref_height`.
- Concrete output: origin `(19,21)`, `ref_height=0`,
  `(20,21)=1`, so `deploy_mcv` logs height mismatch and returns `false`.

gamemd:

- Active decompile of `UnitClass__Deploy @ 0x007393c0` reaches
  `BuildingTypeClass__CanBePlacedAt @ 0x0045ee70` before creating/placing the
  building.
- Active decompile of `BuildingTypeClass__CanBePlacedAt @ 0x0045ee70` has no
  terrain-height read and no equality comparison against a reference cell in
  the inspected placement checks.
- Active decompile of `BuildingClass__Unlimbo @ 0x00440580` calls
  `TechnoClass__Unlimbo`, then performs standard building registration and
  occupancy work. The normal standard-YR branch inspected here has no
  all-foundation-cells-same-height reject.
- This is the normal YR MCV deploy path; the inspected height decision is not
  gated by the TS/fog legacy branch in `Unlimbo`.

Verdict: `FAIL`.

Player-visible difference: Rust rejects a clear mixed-height deploy before
spawning the Construction Yard; gamemd's active deploy/placement path has no
same-height gate and proceeds if the cells are otherwise legal.

### Stage 6 - MCV Despawn and Construction Yard Spawn

Rust:

- Because Stage 5 returns `false`, `src/sim/world/world_spawn.rs:573-590` is not
  reached.
- Concrete output: MCV remains at `(20,22)`, no `GACNST` entity is spawned,
  selected/building-up transfer does not occur.

gamemd:

- Active decompile of `UnitClass__Deploy @ 0x007393c0` constructs a
  `BuildingClass`, calls the new building's `vtable+0xD8`, transfers state, and
  destroys the source unit when placement succeeds.
- For the otherwise-clear mixed-height fixture, the inspected placement path
  has no same-height reject before this success branch.
- Exact resulting object IDs and health transfer numbers were not computed for
  this fixture.

Verdict: `FAIL`.

Player-visible difference: Rust leaves the player with an undeployed AMCV;
gamemd produces a deployed `GACNST` for the same otherwise-clear placement.

### Stage 7 - Visible Feedback and Audio on Failure

Rust:

- Stage 5 rejection logs only from `src/sim/world/world_spawn.rs:563-568`.
- No sim sound/EVA event is emitted on this MCV deploy rejection path in the
  inspected code.

gamemd:

- `MCV_DEPLOY_GHIDRA_REPORT.md` records `EVA_CannotDeployHere` references at
  `0x004FB372`, `0x004ABC7B`, and `0x00739502`.
- Active `UnitClass__Deploy @ 0x007393c0` decompile shows the failure branch can
  call `VoxClass__PlayEVA` for human players before restoring deploy state.

Verdict: `NOT-IMPLEMENTED`.

Player-visible difference: if Rust rejects the command, the player gets no
matching cannot-deploy EVA/audio feedback from this path.

### Stage 8 - Final Screen Result

Rust:

- Concrete result after the normal deploy command: AMCV remains at `(20,22)`;
  no `GACNST` is spawned; no build-up animation starts; no matching EVA/audio
  feedback is emitted.

gamemd:

- Concrete expected result from inspected active placement gates: no
  same-height rejection; the Construction Yard placement proceeds if all other
  checks pass.
- Exact screen pixel anchor and build-up frame timing for the spawned `GACNST`
  were not recomputed for this trace.

Verdict: `FAIL`.

Player-visible difference: the implementation still shows "MCV did not deploy"
where standard YR accepts the deploy and shows the Construction Yard.

## Failures

1. Stage 5 - current MCV deploy implementation still rejects mixed-height
   `GACNST` footprints. The intended parity fix is absent or overwritten in
   `src/sim/world/world_spawn.rs`.
2. Stage 6 - because of the height rejection, Rust does not despawn the AMCV or
   spawn `GACNST`; gamemd proceeds through `Unlimbo` for the otherwise-clear
   footprint.
3. Stage 8 - final visible outcome diverges: Rust leaves an undeployed AMCV;
   gamemd yields a deployed Construction Yard.

## Not Implemented

1. Stage 7 - cannot-deploy feedback is not implemented on this Rust rejection
   path; gamemd has `EVA_CannotDeployHere` behavior on placement/deploy failure.

## Adjacent Findings

- Ready-building mixed-height placement is out of scope for this slot.
- Exact building sprite anchor and build-up animation cadence for a successful
  mixed-height deploy remain unchecked.
- Exact gamemd origin-cell calculation for MCV `(20,22)` remains unchecked;
  this does not affect the main finding because Rust rejects after its own
  origin calculation and gamemd has no same-height gate in the active placement
  functions.

## Verdict Tally

PASS: 1 | FAIL: 3 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

## Evidence Notes

- Ghidra use was read-only: decompiled `UnitClass__Deploy`,
  `BuildingTypeClass__CanBePlacedAt`, and `BuildingClass__Unlimbo`; no mutating
  tools were used.
- No Rust files, INI files, in-repo docs, or other paths were modified for this
  trace.
