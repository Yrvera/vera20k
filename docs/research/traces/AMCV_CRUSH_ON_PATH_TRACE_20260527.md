# AMCV Crush-On-Path Locomotion Trace — 2026-05-27

## Scope

Trace-swarm slot 5 only: AMCV drives from `(40,40)` to `(45,40)` on flat ground with one infantry unit standing on the route. I resolved the otherwise-unspecified infantry as stock `E1` GI at route cell `(42,40)`, ground layer, enemy to the AMCV owner, not deployed, not prone, not warped/temporal, not in limbo, centered in its cell. Adjacent infantry types and deployed infantry are out of scope.

Hard constraints followed: Ghidra use was read-only only; this is the only file written.

## Evidence

- INI, active YR data: `ini/rulesmd.ini:6969-6998` defines `AMCV` with `Speed=4`, `ROT=5`, `Crusher=yes`, drive locomotor `{4A582741-9839-11d1-B709-00A024DDAFD1}`, and `MovementZone=Normal`.
- INI, active YR data: `ini/rulesmd.ini:3713-3757` defines `E1` with `CrushSound=InfantrySquish`, `DieSound=GIDie`, and `Crushable=yes`.
- Verified research: `docs/research/CRUSH_SYSTEM_GHIDRA_REPORT.md:198-307` documents active `UnitClass::PerCellProcess @ 0x741700` crush application.
- Verified research: `docs/research/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md:619-653` documents active `UnitClass::Can_Enter_Cell @ 0x73F0A0` returning code `2` for crush/friendly infantry presence, not hard impassable.
- Read-only Ghidra spot-check: `TechnoClass__CanCrushCheck @ 0x005F6CD0` confirmed the regular path tests victim `Crushable`, on-map, not deployed, not allied, and not warped.
- Read-only Ghidra spot-check: `UnitClass__PerCellProcess @ 0x00741700` confirmed active YR checks `UnitType+0xD28 Crusher` or veteran `CRUSHER`, scatters on `entering != 0`, and kills on `entering == 0`.

## Pipeline

Move command -> path request -> entity-block path weighting -> per-step deferred occupancy -> cell-entry classification -> crush victim selection -> occupancy removal -> deferred kill removal -> sound event drain -> render/audio.

## Stage Findings

### Stage 1 — Rules Data

Expected gamemd: AMCV type has `Crusher=yes` at TechnoType offset `+0xD28`, so it satisfies the first crush-capability gate in `UnitClass::PerCellProcess`.

Rust: `ObjectType` parses victim-side `Crushable`, `DeployedCrushable`, `OmniCrusher`, and `OmniCrushResistant` at `src/rules/object_type.rs:1011-1015`, but there is no parsed `Crusher=` field. `rg "Crusher"` in `src/rules/object_type.rs` finds no parser for `Crusher=`.

Verdict: NOT-IMPLEMENTED. The AMCV's active `Crusher=yes` bit is not represented.

### Stage 2 — Move Order / Crusher Flag Propagation

Expected gamemd: AMCV can be a crusher even while `MovementZone=Normal`; the crush gate reads `Crusher=yes`, not the movement-zone string.

Rust: `resolve_move_info` sets `movement_zone` from INI but computes `mover_is_crusher` only from `e.omni_crusher` or locomotor movement zones `Crusher`, `AmphibiousCrusher`, `CrusherAll` at `src/sim/world/world_commands.rs:91-100`. AMCV has `MovementZone=Normal`, so the concrete value is `mover_is_crusher=false`.

Verdict: FAIL. Numeric branch value differs: gamemd crusher gate true, Rust command/path option false.

### Stage 3 — Path Planning Over the Route Infantry

Expected gamemd: active vehicle pathing and runtime checks treat crushable infantry as a temporary/crush situation; the AMCV is not supposed to route as a non-crushing vehicle around a crushable GI solely because `MovementZone=Normal`.

Rust: A* soft-block cost is skipped only when `options.mover_is_crusher` is true at `src/sim/pathfinding/core.rs:1264-1266`. For AMCV, that option is false, so the E1 route cell is treated as an entity soft block.

Verdict: FAIL. The path planner consumes the wrong crusher bit before the runtime cell-entry stage.

### Stage 4 — Runtime Cell-Entry / Crush Decision

Expected gamemd: on the route cell, `UnitClass::PerCellProcess` sees AMCV `Crusher=yes`; for E1, `TechnoClass::CanCrushCheck` returns `1` because E1 is crushable, on map, not deployed, enemy, and not warped. With centered occupants, distance squared is `0 <= 0x3FFF`.

Rust: `classify_occupied_cell_with_layers` asks `collect_crush_victims` and `cell_passable_after_crush` using `snap.movement_zone` plus `snap.omni_crusher` at `src/sim/pathfinding/cell_entry.rs:395-423`. `can_crush` returns false for `MovementZone::Normal` at `src/sim/movement/bump_crush.rs:415-453`. Concrete victim count is `0`, not `1`.

Verdict: FAIL. Concrete crush decision differs: gamemd `CanCrushCheck=1`; Rust `victims.len()=0`.

### Stage 5 — Victim State Changes

Expected gamemd: `UnitClass::PerCellProcess` stores `next`, plays `CrushSound`, frees mind-control captures, records the kill with AMCV as killer, marks for deletion, destroys/uninitializes, and removes the E1 from game.

Rust: Because Stage 4 produces no victims, the deferred kill block at `src/sim/movement/movement_tick.rs:1052-1065` does not run. E1 remains in `EntityStore` with unchanged health for this crush scenario.

Verdict: FAIL. Concrete state differs: gamemd removes `1` victim; Rust removes `0`.

### Stage 6 — AMCV Movement Continuation / Final State

Expected gamemd: after the centered E1 is crushed, the AMCV continues its drive toward `(45,40)`; the victim no longer blocks the route.

Rust: with no crush victims, the occupied enemy path goes through `CellEntryResult::OccupiedEnemy`, setting `attack_target` and blocked/repath handling at `src/sim/movement/movement_occupancy.rs:433-506`. Exact final AMCV cell/tick was not simulated, but the branch is already different from gamemd's crush-through path.

Verdict: FAIL. Player-visible result differs: the AMCV does not crush-through the infantry route cell.

### Stage 7 — Sound / Event Timing

Expected gamemd: on the kill pass, `VocClass::PlayAt` plays the victim's `CrushSound` at the crusher's current coordinates before kill finalization (`docs/research/CRUSH_SYSTEM_GHIDRA_REPORT.md:291-302`).

Rust: no crush means no sound event. Additionally, the implemented helper would emit `EntityCrushed` and `EntityDied` from victim position if reached (`src/sim/movement/bump_crush.rs:511-535`), while gamemd's verified PerCellProcess directly plays `CrushSound` at crusher coordinates in this stage.

Verdict: FAIL. Concrete event count differs in this scenario: gamemd crush sound count `1`; Rust crush sound count `0`.

### Stage 8 — Kill Attribution / Mind Control Cleanup

Expected gamemd: crush death calls FreeAllMindControlCaptures and RecordKill before deletion.

Rust: the deferred removal block sets health to `0`, clears radio contacts, removes the entity, and increments stats at `src/sim/movement/movement_tick.rs:1052-1065`; there is no equivalent RecordKill or mind-control cleanup in this path.

Verdict: NOT-IMPLEMENTED. For this scenario it is masked by the earlier no-crush failure, but the gamemd kill-side effects are absent.

### Stage 9 — Distance Gate

Expected gamemd: object must be within `0x3FFF` squared leptons of the crusher position before crush death. For the centered route-cell assumption, expected distance squared is `0`, so the gate passes.

Rust: crush collection is cell-based and does not compute the gamemd distance gate (`src/sim/movement/bump_crush.rs:475-504`).

Verdict: FAIL. Mechanism differs even though the chosen centered E1 would pass both if AMCV crusher eligibility were fixed.

### Stage 10 — Tilt / Render

Expected gamemd: after any crush, PerCellProcess checks `TiltsWhenCrushes`; if true and tilt is zero, it writes approximately `-0.05`.

Rust: this exact AMCV scenario did not reach a crush, and I did not verify AMCV's `TiltsWhenCrushes` default or any Rust equivalent.

Verdict: UNCHECKED.

## Player-Visible Failures

1. AMCV does not crush the route infantry because `Crusher=yes` is not parsed or propagated.
2. AMCV may stop, attack, wait, or repath around a GI it should drive over.
3. E1 remains alive in Rust where gamemd removes it immediately on the crush pass.
4. `InfantrySquish` does not play in Rust for this AMCV scenario.
5. Score/kill attribution and mind-control release side effects for crush are absent from the Rust kill path.

## Adjacent Findings

- Rust's current sound helper emits both crush and death sounds from victim cell position if a crush is reached; gamemd's verified PerCellProcess plays the crush sound from crusher coordinates. This report did not trace all death-sound side effects outside PerCellProcess.
- Rust's `can_crush` treats `MovementZone::Destroyer`, `AmphibiousDestroyer`, and `InfantryDestroyer` as regular crusher zones, but `mover_is_crusher` propagation only exempts `Crusher`, `AmphibiousCrusher`, and `CrusherAll` in path options. That broader inconsistency is outside this AMCV-only trace.

## Verdict Tally

PASS: 0 | FAIL: 7 | UNCHECKED: 1 | NOT-IMPLEMENTED: 2

## Status

COMPLETE
