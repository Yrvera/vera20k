# Implementation Trace: `High=yes` Map Unit Parse Parity

**Scenario:** Load one `[Units]` entry equivalent to:

```ini
0=Americans,MTNK,256,5,5,64,Guard,None,0,-1,yes,-1,false,false
```

The cell `(5,5)` has a bridge deck available. This trace checks only the implemented `High=` parse parity path for literal `yes`.

**Scope:** parser output -> map spawn z/layer/occupancy -> first screen anchor/status-bar anchor. Adjacent `High=1`, infantry `High=`, and movement-after-spawn cases are out of scope.

## Evidence Used

- Rust parser: `src/map/entities.rs:244-296`, test coverage at `src/map/entities.rs:403-419`.
- Rust spawn: `src/sim/world/world_spawn.rs:48-68`, `:177-193`, `:234-268`.
- Rust screen anchor: `src/sim/game_entity.rs:307-318`, `src/util/lepton.rs:136-144`, `src/app_instances/units.rs:140`, `:238-240`.
- Rust status-bar anchor: `src/app_ui_overlays.rs:427-456`, `:521-548`, `src/render/selection_overlay.rs:160-162`, `:176-179`.
- INI data: `[MTNK]` in `ini/rulesmd.ini:6603-6625`; no `PixelSelectionBracketDelta`, so Rust default is `0` via `src/rules/object_type.rs:856-858`.
- Ghidra read-only: `ScenarioClass__Read_Units_Section @ 0x00743270`, `CRT__atoi @ 0x007C9B72`, `ScenarioClass__Full_Init @ 0x00686B20`, `CellClass__AddContent @ 0x0047E8A0`, `CoordsToClient`, `ObjectClass__Mark_Occupation`, `ObjectClass__GetHeight`, `TechnoClass__DrawHealthBar @ 0x006F64A0`.

## Active Path Check

`ScenarioClass__Full_Init @ 0x00686B20` calls `ScenarioClass__Read_Units_Section @ 0x00743270` during the standard scenario load sequence after map/overlay initialization and before buildings are read. This is active in standard YR, not a dormant TS-only path.

## Stage Table

| Stage | Rust output | gamemd/YR output | Verdict |
|---|---:|---:|---|
| 1. Unit field split | `fields[10] = "yes"` for `High` | 11th parsed token after owner/type/health/x/y/facing/mission/tag/veterancy/group is `"yes"` | PASS |
| 2. `High` boolean parse | `parse::<i32>("yes") = Err`, `high = false` (`0`) | `CRT__atoi("yes") = 0`, `iVar5 != 0` is false | PASS |
| 3. Bridge deck gate | `map_ent.high == false`, so `bridge_spawn = None`; available deck is ignored | High branch skipped; `OnBridge` byte is not set and z bump is not applied | PASS |
| 4. Spawn z for this literal token | With no supplied nonzero ground height, Rust uses ground z `0` instead of deck z | `local_8c` remains `0`; only numeric nonzero High would set `ground + bridge offset` | PASS |
| 5. `on_bridge` / bridge occupancy | `on_bridge = false`, `bridge_occupancy = None` | object `OnBridge` remains `0` | PASS |
| 6. Occupancy list/layer | `MovementLayer::Ground`; `occupancy.add(..., Ground, ...)` | `CellClass__AddContent` selects ground `FirstObject` list when bridge flag argument is `0` | PASS |
| 7. First screen anchor | `lepton_to_screen(5,5,128,128,0) = (0,165)` | `CoordsToClient` for `(5*256+128, 5*256+128, 0)` gives `(0,165)` | PASS |
| 8. Unit sprite anchor | Body sprite base uses cached `(sx,sy) = (0,165)` before sprite atlas image offsets | Draw path consumes the same projected object location for the unit body anchor; exact GTNK frame atlas offsets not recomputed here | UNCHECKED |
| 9. Vehicle status-bar fill anchor | Rust full-health selected vehicle pips start at `(-15,140)` from `(0,165) + (-15,-25) + delta 0` | `TechnoClass__DrawHealthBar` non-infantry branch draws filled pips at `pLocation + (-15, type_delta -25)`; MTNK delta is `0`, so `(-15,140)` | PASS |
| 10. Vehicle status-bar background anchor | Rust vehicle `pipbrd` background starts at `(1,139)` from `(0,165) + (1,-26) + delta 0` | `TechnoClass__DrawHealthBar` selected non-infantry background uses `pLocation + (1, type_delta -26)`; MTNK delta is `0`, so `(1,139)` | PASS |

## Findings

No FAIL or NOT-IMPLEMENTED findings for this concrete implementation trace.

The previous player-visible bug, where literal `High=yes` placed a map unit on the bridge deck, is fixed for this scenario. The current Rust parser now matches gamemd's `atoi` semantics for the unit `High` field: nonnumeric `yes` is false, so the bridge deck is ignored, the unit spawns on the ground layer, and the initial projected anchor remains the ground-layer coordinate.

## Unchecked / Residual Risk

- Stage 8 is `UNCHECKED` because I did not compute the exact GTNK voxel/body render frame offsets in both engines. The base coordinate feeding the render path is numerically equal.
- This trace does not validate numeric `High=1`, `High=-1`, infantry `High=`, or nonzero terrain-height cells. Those are adjacent scenarios.

## Verdict Tally

PASS: 9 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
