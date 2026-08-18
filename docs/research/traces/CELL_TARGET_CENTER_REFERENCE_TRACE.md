# Cell Target Center Reference Trace

Scenario: a projectile or force-fire cell target resolves target cell `(50,50)` on flat ground. This trace compares gamemd `CellClass` coordinate virtuals `+0x48` and `+0x58` against current Rust combat, homing, render-only projectile, and AoE cell-target reference points.

## Verdict

PASS: 5 | FAIL: 1 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Concrete Values

- Cell input: `(rx, ry) = (50,50)`.
- Leptons per cell: `256`.
- Cell center sub-lepton: `128`.
- Expected center XY: `50 * 256 + 128 = 12928`, so `(x,y) = (12928,12928)`.
- Flat-ground Z for this scenario: `0`.
- No structural bridge in this scenario, so `CellClass +0x58` must equal `+0x48`: `(12928,12928,0)`.

## Pipeline

`Ctrl force-fire / projectile cell target -> TargetKind::Cell(50,50) -> combat target resolution -> range / facing / fire event destination -> optional AoE detonation -> render-only projectile destination`

## Stage Trace

### Stage 1 - gamemd CellClass `+0x48` center coordinate

- Rust surface: `src/sim/combat/mod.rs:342`, `src/util/lepton.rs:20`.
- Rust output: `cell_center_coords(50,50)` returns `(50,50,128,128)`, absolute XY `(12928,12928)`. Flat cell-target impact Z is `0` via `attack_impact_z` at `src/sim/combat/mod.rs:736`.
- gamemd output: `CellClass__Get_Center_Coords @ 0x00480A30` computes `x = MapCoord_X * 256 + 128`, `y = MapCoord_Y * 256 + 128`, `z = ground height`; existing report also verifies vtable slot `+0x48 @ 0x00486840`.
- Active in standard YR: yes; direct callers include tactical/action visuals and movement callers, and vtable slot is core `CellClass` coordinate behavior.
- Verdict: PASS for flat `(50,50)`: both are `(12928,12928,0)`.

### Stage 2 - gamemd CellClass `+0x58` target coordinate

- Rust surface: `src/sim/combat/in_range.rs:245`, `src/sim/combat/in_range.rs:267`, `src/sim/combat/in_range.rs:334`.
- Rust output: `resolve_target_coords_3d(TargetKind::Cell(50,50))` computes `tx=12928`, `ty=12928`, `tz=ground_z_with_bridge_offset`; on flat no-bridge ground this is `0`.
- gamemd output: existing report verifies vtable slot `+0x58 @ 0x00486890` delegates to `+0x48` and only adds bridge Z when `CellClass+0x140 & 0x100`. No bridge flag here, so `(12928,12928,0)`.
- Active in standard YR: yes; `BulletClassAiHomingDetonationPath @ 0x004666E0` calls target vtable `+0x58` for non-null targets in the normal bullet AI path.
- Verdict: PASS for flat `(50,50)`: both are `(12928,12928,0)`.

### Stage 3 - force-fire command target binding

- Rust surface: `src/app_context_order.rs:628`, `src/sim/world/world_commands.rs:357`, `src/sim/combat/mod.rs:495`.
- Rust output: empty-cell force-fire dispatches `Command::ForceAttackCell { target_rx: 50, target_ry: 50 }`, then `AttackTarget::for_cell(50,50)`.
- gamemd output: this trace did not re-walk the mouse/order dispatch binary to compute tick order and command object contents; the reference-point comparison begins once the cell target exists.
- Active in standard YR: force-fire is normal gameplay, but exact order dispatch was out of scope for this slot.
- Verdict: UNCHECKED for order-dispatch parity. Reference-point output after binding is checked in later stages.

### Stage 4 - combat/range/facing target reference

- Rust surface: `src/sim/combat/mod.rs:354`, `src/sim/combat/in_range.rs:278`, `src/sim/movement/turret.rs:105`.
- Rust output: `resolve_target_coords` and `resolve_target_xy_2d` use `(50,50,128,128)` / `(12928,12928)` for `TargetKind::Cell`; turret facing also uses sub `(128,128)`.
- gamemd output: cell target coordinate reads use center XY from `+0x48` or `+0x58`; for flat ground both are `(12928,12928,0)`.
- Active in standard YR: yes for combat targeting and action visuals; CellClass coordinate slots are core active paths.
- Verdict: PASS for reference point XY.

### Stage 5 - direct combat fire and render-only projectile destination

- Rust surface: `src/sim/combat/mod.rs:1535`, `src/sim/combat/mod.rs:1857`, `src/sim/combat/mod.rs:1948`, `src/app_fire_effects.rs:197`.
- Rust output: synthetic cell-target data uses `(50,50,128,128)`; simulation impact Z for cell target is `0`; render-only projectile destination uses `lepton_to_screen(50,50,128,128,0)`.
- gamemd output: flat cell target `+0x48/+0x58` both resolve `(12928,12928,0)`.
- Active in standard YR: yes; `BulletClassBulletDetonationImpactDamage @ 0x00468D80` and normal fire/detonation paths are active.
- Verdict: PASS for flat cell target impact/reference point.

### Stage 6 - AoE detonation reference point

- Rust surface: `src/sim/combat/combat_aoe.rs:68`, `src/sim/combat/combat_aoe.rs:153`, `src/sim/combat/combat_aoe.rs:261`.
- Rust output: `apply_aoe_damage(..., impact_rx=50, impact_ry=50, ...)` measures from `CELL_CENTER_LEPTON` `(128,128)`, absolute `(12928,12928)`.
- gamemd output: `Apply_area_damage @ 0x00489280` converts the impact coord to a containing cell and constructs `centerX = cellX * 256 + 128`, `centerY = cellY * 256 + 128`, `centerZ = 0`.
- Active in standard YR: yes; Ghidra xrefs include `WarheadTypeClass__Detonate @ 0x00469A83`, superweapon launches, bomb detonation, and normal animation/damage paths.
- Verdict: PASS for blast reference point on flat `(50,50)`.

### Stage 7 - homing projectile cell-target reference

- Rust surface: `src/sim/movement/homing_movement.rs:193`, `src/sim/movement/homing_movement.rs:223`, `src/sim/movement/homing_movement.rs:299`, `src/sim/movement/homing_movement.rs:409`.
- Rust output if the helper is given `target_pos=(50,50)`: target storage is only `last_known_rx=50`, `last_known_ry=50`; distance math computes `(50 - pos_cells) * 256`, so the target point is whole-cell `(12800,12800)` in lepton terms, not center `(12928,12928)`.
- gamemd output: `BulletClassAiHomingDetonationPath @ 0x004666E0` calls target vtable `+0x58`; for a `CellClass` target on flat ground that is `(12928,12928,0)`.
- Active in standard YR: yes; this is the normal per-tick bullet AI function. Current Rust production code did not show a live call wiring `TargetKind::Cell` into `attach_homing_state`; `attach_homing_state` only accepts an entity `target_id` plus whole-cell `target_pos`.
- Verdict: FAIL for the homing helper's target point if used for this cell; NOT-IMPLEMENTED for a live homing `TargetKind::Cell` binding.

## Top Player-Visible Findings

1. Stage 7: homing cell-target helper would aim 128 leptons northwest in both axes, so missiles force-fired at terrain can visibly steer toward a cell corner instead of the center; Rust `src/sim/movement/homing_movement.rs:409`; gamemd evidence `BulletClassAiHomingDetonationPath @ 0x004666E0` target vtable `+0x58`.
2. Stage 7: no live Rust homing cell-target binding was found, so CellClass `+0x58` semantics are not represented for homing projectiles targeting a map cell; Rust `src/sim/movement/homing_movement.rs:193`; gamemd evidence `BulletClassAiHomingDetonationPath @ 0x004666E0`.

## Adjacent Findings

- Existing AAHeatSeeker2/GUARDWH reports verify a separate AoE candidate-radius detail for `CellSpread=.5`: gamemd adds `0.99` before integer conversion and can inspect the first ring. This is adjacent to reference points and not traced here.
- Bridge behavior is not exercised by this flat-ground scenario. Existing research says `+0x58` adds bridge Z only for structural bridge cells; current Rust has bridge-aware helpers, but exact bridge globals remain outside this trace.
- Building/foundation target anchors are intentionally out of scope; this trace covers `CellClass` cell targets only.

## Sources

- `docs/research/CELL_REFERENCE_POINTS_GHIDRA_REPORT.md`
- `docs/research/AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md`
- `docs/research/AAHEATSEEKER2_GUARDWH_DETONATION_PARAMETERS_GHIDRA_REPORT.md`
- Ghidra read-only spot checks: `CellClass__Get_Center_Coords @ 0x00480A30`, `CellClass__GetGroundHeight @ 0x00578080`, `CellClass__Get_Cell_At @ 0x00565730`, `BulletClassAiHomingDetonationPath @ 0x004666E0`, `BulletClassBulletDetonationImpactDamage @ 0x00468D80`, `Apply_area_damage @ 0x00489280`.

Status: COMPLETE
