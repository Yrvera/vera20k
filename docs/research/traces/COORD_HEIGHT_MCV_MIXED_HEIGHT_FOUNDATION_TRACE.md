# Coordinate/Height Trace: MCV or Ready Building over Mixed-Height Foundation

Date: 2026-05-21

Scope: one concrete scenario only: an Allied MCV attempts to deploy into `GACNST`, or a ready `GACNST` is placed, with the resulting `GACNST` foundation spanning mixed terrain heights.

Concrete fixture used for value tracing:

- Unit/building data: retail YR `AMCV -> GACNST`.
- `ini/rulesmd.ini`: `[AMCV] DeploysInto=GACNST`; `[GACNST] UndeploysInto=AMCV`.
- `ini/artmd.ini`: `[GACNST] Foundation=4x4`, `Height=4`.
- Rust MCV deploy fixture: MCV center cell `(20,22)`.
- Rust computed `GACNST` origin for a 4x4 foundation: `(20 - 4/2, 22 - 4/2) = (18,20)`.
- Mixed-height footprint: top-left/reference cell `(18,20)` has height `0`; cell `(19,20)` has height `1`; all other cells clear, in bounds, unoccupied, and otherwise buildable.

Verdict rule: `PASS` requires literal numerical equality between Rust output and gamemd output. If both were not computed, the stage is `UNCHECKED`.

## Pipeline

Player click or deploy command -> command scheduling -> foundation origin resolution -> per-cell placement validation -> placement/deploy accept/reject -> entity mutation -> placement/deploy visual and audio feedback.

## Entry Points Covered

- Ready-building click: `place_ready_building_at_cursor` reads the current preview cell and queues `Command::PlaceReadyBuilding`.
- Ready-building sim command: `Command::PlaceReadyBuilding` calls `production::place_ready_building`.
- MCV deploy command: `Command::DeployMcv` calls `Simulation::deploy_mcv`.
- gamemd ready placement: `HouseClass::Place_Production @ 0x004FB0E0` calls building `vtable+0xD8`, `BuildingClass::Unlimbo @ 0x00440580`.
- gamemd MCV deploy: `UnitClass::Deploy @ 0x007393C0` constructs a `BuildingClass`, calls placement validation, then calls building `vtable+0xD8`.
- gamemd commit validator: `BuildingTypeClass::CanBePlacedAt @ 0x0045EE70`, active in standard YR through `UnitClass::Deploy`; callers also include AI deploy and build-queue exit paths.

## Stage Trace

### Stage 1 - Click/command cell

Rust ready-building placement:

- `src/app_sim_tick.rs:879` converts screen cursor to `(rx, ry)`.
- `src/app_commands.rs:215-227` uses the preview's stored `(rx, ry)` if available; otherwise it converts the cursor again.
- `src/app_commands.rs:245-254` queues `Command::PlaceReadyBuilding { owner, type_id, rx, ry }`.

Concrete output: for a ready `GACNST` preview anchored at `(18,20)`, the queued origin is `(18,20)`.

gamemd:

- The active ready-placement commit path is `HouseClass::Place_Production @ 0x004FB0E0`; it receives the clicked cell and forms coordinates as `cell_x * 256 + 128`, `cell_y * 256 + 128` before calling `BuildingClass::Unlimbo`.
- Cursor preview validation is explicitly not `CanBePlacedAt`; prior report `FIND_NEAREST_VARIANTS_SPIRAL_COMPARISON_GHIDRA_REPORT.md` says build-cursor preview uses a different DisplayClass routine and was out of scope.

Verdict: `UNCHECKED` - the Rust origin number was computed, but the gamemd cursor-to-origin output for the same screen click was not computed.

### Stage 2 - Foundation data

Rust:

- `production::foundation_dimensions` delegates to `rules::foundation::foundation_dimensions`.
- Retail `artmd.ini` gives `[GACNST] Foundation=4x4`.
- Output: width `4`, height `4`.

gamemd:

- `BuildingTypeClass::GetFoundationWidth @ 0x0045EC90` returns `g_FoundationWidthTable[Type+0xEF0]`.
- `BuildingTypeClass::GetFoundationHeight @ 0x0045ECA0` returns `g_FoundationHeightTable[Type+0xEF0]`; no stock YR caller in this path uses the bib-extension argument.
- For retail `GACNST Foundation=4x4`, output is width `4`, height `4`.

Verdict: `PASS`.

### Stage 3 - MCV center to foundation origin

Rust:

- `src/sim/world/world_spawn.rs:514-518` computes deploy origin from the MCV entity cell and the target building foundation.
- `src/sim/world/world_spawn.rs:680-685`: `origin = (center_rx - width / 2, center_ry - height / 2)`.
- Concrete output for MCV `(20,22)` and `GACNST 4x4`: origin `(18,20)`.

gamemd:

- `UnitClass::Deploy @ 0x007393C0` gets a deploy cell through a virtual call, builds `CoordStruct(cell_x * 256 + 128, cell_y * 256 + 128, 0)`, then calls the new building's `vtable+0xD8`.
- I did not compute the exact source of that deploy cell from the MCV location for this fixture; whether the virtual returns raw unit cell or a foundation-adjusted cell remains unverified in this run.

Verdict: `UNCHECKED`.

### Stage 4 - Ready-building per-cell mixed-height validation

Rust:

- `src/sim/production/production_placement.rs:300-314` sets `ref_height = height_map[(rx, ry)]` and checks every foundation cell against it.
- `src/sim/production/production_placement.rs:381` computes `same_height = height_map[(cx, cy)] == ref_height`.
- `src/sim/production/production_placement.rs:411` requires `same_height`.
- Concrete output: `ref_height=0`; first mismatching cell `(19,20)` has `1`; placement returns `Err(BlockedTerrain)`, so `place_ready_building` returns `false`.

gamemd:

- `BuildingTypeClass::CanBePlacedAt @ 0x0045EE70` walks `GetFoundation(1)` offsets, checks in-bounds, overlay presence, first object/terrain object, upgrade/ally overlap, and scatter side effects. The decompiled active function contains no terrain height read and no equality comparison against a reference cell.
- `HouseClass::Place_Production @ 0x004FB0E0` then calls `BuildingClass::Unlimbo @ 0x00440580`; the decompiled Unlimbo body calls `TechnoClass::Unlimbo`, increments occupancy over `(W+2)*(H+2)`, and has no all-cells-same-height gate.
- For the concrete otherwise-clear mixed-height footprint, the active gamemd commit-side code inspected here has no branch that rejects solely because `(19,20).height=1` while `(18,20).height=0`.

Verdict: `FAIL`.

Player-visible difference: Rust turns the placement red/rejects the click and leaves the building ready; gamemd's inspected commit path does not reject solely for mixed height and proceeds to place the building if the cells are otherwise legal.

### Stage 5 - MCV deploy per-cell mixed-height validation

Rust:

- `src/sim/world/world_spawn.rs:535` sets `ref_height` from the deploy origin.
- `src/sim/world/world_spawn.rs:536-579` iterates the footprint cells and rejects if any cell height differs from the reference.
- `src/sim/world/world_spawn.rs:570-577` returns `false` on height mismatch.
- Concrete output: origin `(18,20)`, `ref_height=0`, cell `(19,20)=1`; `deploy_mcv` returns `false`, MCV remains, no ConYard spawns.

gamemd:

- `UnitClass::Deploy @ 0x007393C0` calls active `BuildingTypeClass::CanBePlacedAt @ 0x0045EE70` before the create/place branch. The decompiled validator has no terrain-height equality check.
- The create/place branch calls the new building's `vtable+0xD8`; `BuildingClass::Unlimbo @ 0x00440580` has no all-cells-same-height check in the normal placement branch.
- This path is active in standard YR: `UnitClass::Deploy` is the documented MCV deploy path and directly calls `CanBePlacedAt` and `Unlimbo`; no TS-only flag gates the inspected checks.

Verdict: `FAIL`.

Player-visible difference: Rust refuses to deploy the MCV on mixed-height but otherwise legal footprint cells; gamemd's inspected active deploy path does not reject solely for mixed terrain height.

### Stage 6 - Accept/reject state mutation

Rust ready-building:

- `src/sim/production/production_placement.rs:205-209` returns `false` before spawn if validation fails.
- `src/sim/production/production_placement.rs:255-267` removes the ready item only after successful spawn; rejected mixed-height placement keeps the ready building.

Rust MCV deploy:

- `src/sim/world/world_spawn.rs:581-587` despawns the MCV and spawns the ConYard only after all checks pass.
- On mixed-height failure, those lines are not reached.

gamemd:

- In `HouseClass::Place_Production @ 0x004FB0E0`, a failed `Unlimbo` plays EVA and keeps placement mode; a successful `Unlimbo` calls `FactoryClass::CompletedProduction`.
- In `UnitClass::Deploy @ 0x007393C0`, failed building placement destroys the newly constructed building and re-enables the deploy interface; success transfers state and destroys the MCV.
- I did not compute the complete ready queue/sidebar state for the concrete mixed-height fixture because gamemd's cursor preview routine was not decompiled here.

Verdict: `UNCHECKED`.

### Stage 7 - Visible feedback and audio

Rust:

- Ready placement rejection is local and silent in this slice: `src/app_commands.rs:228-241` logs and returns if preview is invalid.
- MCV deploy rejection in `src/sim/world/world_spawn.rs:570-577` logs and returns false; no `SimSoundEvent` or EVA event is emitted for "cannot deploy here" in this path.

gamemd:

- `MCV_DEPLOY_GHIDRA_REPORT.md` records `EVA_CannotDeployHere` references from `HouseClass::Place_Production @ 0x004FB372`, deploy command failure, and `UnitClass::Deploy @ 0x00739502`.
- The live `HouseClass::Place_Production @ 0x004FB0E0` decompile shows `VoxClass::PlayEVA` on placement failure for the player.

Verdict: `NOT-IMPLEMENTED`.

Player-visible difference: on a rejected deploy/placement, Rust currently provides no matching EVA/audio feedback from the sim path; gamemd plays the cannot-deploy/cannot-place feedback for the local player.

### Stage 8 - Final screen outcome

Rust:

- Mixed-height ready placement: invalid preview/click rejection; no new `GACNST`.
- Mixed-height MCV deploy: MCV remains at `(20,22)`; no `GACNST`; only log output.

gamemd:

- The inspected active commit-side placement/deploy code does not reject solely on terrain-height mismatch; if the cells are otherwise legal, the visible result is a placed/deployed `GACNST`.
- The exact sprite screen pixel anchor for the resulting `GACNST` was not recomputed in this run; prior `FOUNDATION_CENTER_INVESTIGATION.md` leaves a known open question around building sprite anchoring.

Verdict: `FAIL`.

## Failures

1. Ready-building mixed-height rejection is stricter than gamemd's inspected active commit path. Rust rejects because every cell must match the top-left height; gamemd `CanBePlacedAt`/`Unlimbo` inspected here has no same-height gate.
2. MCV mixed-height rejection is stricter than gamemd's inspected active deploy path. Rust rejects before despawn/spawn; gamemd `UnitClass::Deploy -> CanBePlacedAt -> Unlimbo` has no same-height gate in the inspected functions.
3. Rejection feedback is incomplete. Rust logs rejection but does not emit gamemd's cannot-deploy/cannot-place EVA/audio event from this path.

## Adjacent Findings

- `src/sim/world/world_spawn.rs:536-537` iterates `for dy in 0..fw` and `for dx in 0..fh`; width/height are swapped. For retail `GACNST 4x4` this has no visible effect, so it is not counted as a failure for this concrete MCV scenario.
- The ready-placement preview validates with `screen_point_to_world_cell` and a top-left anchor. The exact gamemd DisplayClass cursor-preview routine was not decompiled in this run, so preview cell coloring remains unchecked.
- `CanBePlacedAt` has a scatter side effect for overlapping allied movable objects. This concrete mixed-height trace used no occupants, so scatter is out of scope.

## Verdict Tally

PASS: 1 | FAIL: 3 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

## Evidence Notes

- Active YR, not TS legacy: `HouseClass::Place_Production @ 0x004FB0E0`, `UnitClass::Deploy @ 0x007393C0`, `BuildingTypeClass::CanBePlacedAt @ 0x0045EE70`, and `BuildingClass::Unlimbo @ 0x00440580` are normal production/deploy placement paths and are not gated by TS fog/fog-of-war branches for the height decisions discussed here.
- TS-legacy branch observed in Unlimbo: the fogged snapshot branch is gated by `ScenarioClass SpecialFlags & 0x1000`; it was not used as evidence for placement acceptance.
- No Rust code, INI files, or in-repo docs were modified for this trace.
