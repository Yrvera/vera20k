# Garrison Shot Z-Adjust Depth Postfix Trace

Scope: one occupied building fires one garrison shot and spawns the weapon `OccupantAnim` muzzle flash. Concrete stock case: `UCPara` in an occupied building selects `OccupantAnim=UCFLASH`; this trace only checks whether the flash's occupied-shot `ZAdjust=-200` is applied as render depth bias rather than screen-Y displacement, and whether the resulting ordering is proven equal to active YR.

Run mode: `/trace-action` on the exact scenario requested by trace-swarm slot 2.

## Pipeline

Occupied building shot -> `TechnoClass::Fire_At` selects weapon `OccupantAnim` -> native `AnimClass` is constructed with draw flags `0x600` -> occupied-building branch writes `anim+0x100 = -200` -> `AnimClass::DrawIt` / `CC_Draw_Shape` consume shape position and z-adjust/depth input -> Rust stores `GarrisonMuzzleFlash.z_adjust = -200` -> Rust builds a `SpriteInstance` with unshifted screen position and adjusted float depth -> object pass sorts/draws the flash with building and overlay sprites.

## Stages

| Stage | gamemd evidence | Rust evidence | Verdict |
| --- | --- | --- | --- |
| 1. Active occupied-shot path | Active standard YR ordinary garrison shots use `TechnoClass::Fire_At`; occupied buildings replace normal muzzle anim with `WeaponType+0x110 OccupantAnim` (`docs/research/CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md:108..116`, `120..131`; `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md:75..102`). | Pending fire effects with `occupant_anim` spawn `GarrisonMuzzleFlash` entries (`src/app_building_anim.rs:727..754`). | PASS: same scoped trigger path for this visual. |
| 2. Occupied-shot ZAdjust value | Active YR writes `anim+0x100 = -200` after constructing the occupied-shot anim (`docs/research/CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md:112..114`). | Rust stores `z_adjust: GARRISON_OCCUPANT_ANIM_Z_ADJUST` at spawn (`src/app_building_anim.rs:738..749`); component comment identifies occupied shots as `-200` (`src/sim/components.rs:526..528`). | PASS: `-200 == -200`. |
| 3. Screen-Y displacement | `CC_Draw_Shape` decompile shows centering/shape-frame offsets adjust X/Y, while the z/depth parameter is stored/used as draw depth state; existing ZAdjust report identifies the argument as Z-sort depth bias, not a screen-position offset (`docs/research/CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md:254..266`). | Rust final position is `fy = flash.screen_y + entry.offset_y`; `flash.z_adjust` is not added to `fy` (`src/app_instances/overlays.rs:508..519`). | PASS: Rust no longer applies `-200` as a `screen_y - 200` displacement. |
| 4. Depth-bias direction and neutral point | Research states `1000` is neutral, values below `1000` push away, values above pull forward (`docs/research/CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md:254..266`). | Rust computes base depth from real `screen_y`, then applies `neutral_delta = 1000 - z_adjust`; `z_adjust=1000` is unchanged and `z_adjust=-200` adds `0.0012` to depth (`src/app_instances/overlays.rs:531..547`, test at `src/app_instances/overlays.rs:841..870`). | PASS for sign and neutral only: `-200` pushes away without moving the sprite. |
| 5. Exact depth-bias arithmetic | Native passes/consumes an integer ZAdjust through `CC_Draw_Shape` / blitter Z-buffer state. This trace did not decompile the final blitter arithmetic that maps `-200` to the concrete per-pixel depth comparison against the building body/walls. | Rust uses an ad-hoc normalized float scale: `(base_depth + (1000 - z_adjust) * 0.000001).clamp(0.001, 0.999)` (`src/app_instances/overlays.rs:542..547`). For `-200`, the delta is exactly `+0.0012` in Rust. | FAIL: exact gamemd numerical equality is not proven, and the Rust scalar is not sourced from active YR arithmetic. |
| 6. Ordering relative to building body | Native occupied-shot anim is a normal `AnimClass` in the tactical draw path; its `ZAdjust=-200` participates in native shape/depth sorting. Exact ordering for a stock flash pixel overlapping the occupied building body was not numerically computed in this trace. | Building body depth uses structure YSort row `sy - TILE_HEIGHT/2` (`src/app_instances/shp.rs:214..227`); garrison flash depth uses `flash.screen_y` plus the current scalar bias (`src/app_instances/overlays.rs:511..547`); all SHP pages are sorted by float depth descending (`src/app_render/build_instances.rs:261..263`, `802..808`). | UNCHECKED: no literal side-by-side building-body order number was computed. |
| 7. Ordering relative to nearby overlays/walls | Native shape ordering against walls/nearby overlays depends on the same tactical draw/depth path. This trace did not compute active YR order for a concrete wall/overlay adjacent to the firing port. | Wall overlays get separate depth treatment and a local nudge (`src/app_instances/overlays.rs:341..357`); garrison flashes are inserted into `shp_paged` and sorted afterward (`src/app_render/build_instances.rs:249..263`). | UNCHECKED: player-visible wall/overlay order is not proven numerically equal. |

## Verdict Tally

PASS: 4 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Top Player-Visible Findings

1. Stage 5: exact depth-bias arithmetic. Player-visible difference: the flash can sort one layer too far behind/in front of building pixels or nearby sprites because Rust uses `+0.0012` normalized depth for `ZAdjust=-200`, not a verified active-YR Z-buffer formula. Rust: `src/app_instances/overlays.rs:542..547`. gamemd evidence: occupied shot writes `anim+0x100=-200` and `CC_Draw_Shape` consumes ZAdjust as Z-sort/depth input (`docs/research/CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md:112..114`; `docs/research/CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md:254..266`).

## Adjacent Findings

- Garrison shot cadence and generic `AnimClass` lifecycle remain separate mechanics and were not traced here.
- The current fix addresses the previous screen-row-shift failure for this scenario.
- A follow-up exactness trace should choose one map coordinate, one occupied building frame, and one nearby wall/overlay, then compute gamemd and Rust final draw/depth numbers for the same pixels.

Status: COMPLETE
