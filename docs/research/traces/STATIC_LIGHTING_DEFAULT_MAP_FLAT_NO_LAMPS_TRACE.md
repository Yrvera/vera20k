# Static Lighting Default Map Flat No Lamps Trace

Scenario: load a standard YR map area with no point-light emitters and default or missing `[Lighting]` keys. Compare gamemd.exe terrain/object tint behavior at ground level and at one raised cell against current Rust `src/map/lighting.rs` default `LightingConfig`, `cell_tint`, `cell_light_scalar`, and `CellLightGrid` construction.

Status: COMPLETE for the scoped static/default no-lamp scenario. No Rust, INI, or existing docs were modified.

## Pipeline

`ScenarioClass reset -> [Lighting] missing-key/default read -> CellClass lighting compute -> cell light bundle/cache -> terrain/object draw consumers -> Rust parse/build grid/render tint accessors`

## Concrete Inputs

- Map `[Lighting]`: section missing or keys missing.
- Point-light emitters: none.
- Special lighting modes: inactive; ordinary branch only.
- Checked cells:
  - Ground cell: `CellClass+0x11B = 0`.
  - Raised cell: `CellClass+0x11B = 4`.

## Evidence Summary

- `SCENARIO_LIGHTING_DEFAULT_RESET_PATH_GHIDRA_REPORT.md` verifies ordinary reset defaults: `Ambient=100`, `Red=100`, `Green=100`, `Blue=100`, `Ground=50`, `Level=8`, active in normal YR scenario loads.
- `SCENARIO_LIGHTING_FIELDS_00689E90_GHIDRA_REPORT.md` verifies missing keys preserve reset defaults through `ReadDouble` defaults, and `Ambient` writes `+0x3528/+0x352C/+0x3530`.
- Read-only Ghidra spot-check of `FUN_00484180 @ 0x00484180` confirms the ordinary no-special-mode formula:
  - base ambient output starts as `Scenario+0x352C * 1000 / 100`.
  - RGB starts as `Scenario+0x3534/+0x3538/+0x353C * 1000 / 100`.
  - no point-light sources means additive intensity remains `0`.
  - top/common branch adds `Scenario+0x3544 * signed_cell_level - Scenario+0x3540`.
  - bottom branch adds `Scenario+0x3544 * (signed_cell_level + 4) - Scenario+0x3540`.
  - high clamp is `>1999 -> 2000`; low clamp is `<1 -> 0`.
- `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md` verifies terrain tiles, overlays, terrain objects, Techno SHPs, and some anims consume the cell light bundle/scalars in active standard YR draw paths.
- Current Rust evidence:
  - `C:/Users/enok/Documents/ra2-rust-game/src/map/lighting.rs:59` default `ambient=1.0`, `red=1.0`, `green=1.0`, `blue=1.0`, `ground=0.20`, `level=0.032`.
  - `C:/Users/enok/Documents/ra2-rust-game/src/map/lighting.rs:326` computes `ambient + level * z - ground`.
  - `C:/Users/enok/Documents/ra2-rust-game/src/map/lighting.rs:348` builds `CellLightGrid` with RGB profile `[red, green, blue]` and `common_scalar = cell_light_scalar(config, z)`.
  - `C:/Users/enok/Documents/ra2-rust-game/src/app_init.rs:392` also writes a uniform `terrain_tint(config)` into resolved terrain cells before render setup.

## Stage Verdicts

| Stage | Boundary Output | gamemd.exe | Current Rust | Verdict |
|---|---|---:|---:|---|
| 1. Missing `[Lighting]` defaults | Public ordinary defaults | Ambient/R/G/B `1.0`; Ground `0.20`; Level `0.032` | Same public defaults in `LightingConfig::default` and `parse_lighting` | PASS |
| 2. No point-light contribution | Additive point-light intensity | `0` because no active `LightSourceClass` contributes | `0` because point-light list is empty and `accumulate_point_lights` returns | PASS |
| 3. Ground cell scalar | Ordinary top/common cell scalar at `z=0` | `(100*1000/100) + 0 + (8*0 - 50) = 950`, external `0.950` | `1.0 + 0.032*0 - 0.20 = 0.800` | FAIL |
| 4. Raised cell scalar | Ordinary top/common cell scalar at `z=4` | `(100*1000/100) + 0 + (8*4 - 50) = 982`, external `0.982` | `1.0 + 0.032*4 - 0.20 = 0.928` | FAIL |
| 5. Raised cell bottom scalar | Bottom/alternate scalar at `z=4` | `(100*1000/100) + 0 + (8*(4+4) - 50) = 1014`, external `1.014` | No separate bottom scalar; `CellLight::bottom_scalar` is initialized equal to common scalar `0.928` | FAIL |
| 6. Render-facing cell bundle | Terrain/object lighting inputs | Active draw paths consume `LightConvertClass*` plus scalar fields such as `+0x10A/+0x10C/+0x10E` | Current accessors collapse to one RGB tint `[profile.rgb * common_scalar]` | FAIL |
| 7. Terrain tile raised-cell behavior | TMP terrain at raised cell | TMP tile draw consumes the raised cell's live `+0x34` and `+0x10C`; for this input scalar is `982` before final palette/blitter details | `app_init` pre-tints terrain cells uniformly with ground `terrain_tint = [0.8,0.8,0.8]` | FAIL |
| 8. Exact final pixels | Literal screen RGB after LightConvert/blitter | Not computed in this trace | Not computed in this trace | UNCHECKED |

## Player-Visible Findings

1. Ground default lighting is too dark in Rust. For a missing/default `[Lighting]` no-lamp cell, gamemd computes scalar `0.950`; Rust computes `[0.800, 0.800, 0.800]`. A normal map without authored lighting will render visibly darker.

2. Raised default lighting is also too dark and the height delta is smaller. At `z=4`, gamemd top/common scalar is `0.982`; Rust computes `0.928`. Raised terrain and objects will not brighten by the same amount.

3. Rust has no separate bottom/alternate scalar for the raised cell. Gamemd computes bottom scalar `1.014` for `z=4`; Rust stores `bottom_scalar = common_scalar = 0.928`. Any branch that should consume bottom/alternate lighting cannot match.

4. Rust's terrain tile path applies uniform ground-level tint before render setup. Gamemd terrain tile draw consumes per-cell lighting; a raised cell in this scenario should use the raised cell's scalar, not the ground cell scalar.

5. Rust currently collapses the binary cell-light bundle into one `[f32; 3]` tint. Even when RGB is neutral, the active YR renderer uses separate LightConvert profile and scalar fields, so literal parity remains unproven and currently mismatched at scalar boundaries.

## Adjacent Findings

- The docs and current source comments should be treated carefully: public default `Ground=0.20` is not equivalent to subtracting external `0.20` directly in the final 1000-scale cell scalar. The live binary stores Ground as `50` and subtracts that integer in the `0x00484180` cell formula.
- Exact `LightConvertClass` palette table generation and final blitter RGB remain out of scope for this trace. The scalar mismatch alone is enough to mark visible parity FAIL at the render-input boundary.

## Verdict Tally

PASS: 2 | FAIL: 5 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

