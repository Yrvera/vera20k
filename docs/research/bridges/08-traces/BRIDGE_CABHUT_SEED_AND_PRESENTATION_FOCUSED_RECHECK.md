# Bridge CABHUT Seed Canonicalization And Presentation Focused Recheck

Date: 2026-05-22

Scope: focused recheck of two CABHUT collapse gaps from the deeper trace swarm:

- `DestroyBridgeFromCell_Low/High` canonical seed selection before `CollapseBridge_*`.
- CABHUT collapse presentation/RNG, split between walker pre-destroy explosions and per-cell `BlowUpBridge` debris.

This is analysis only. No Rust, INI, or existing docs were modified.

## Verdict

Both findings are confirmed.

Rust's `dispatch_bridge_collapse_from_hut` uses the first 5x5 overlay hit directly as the bounded walker seed. `gamemd.exe` does not. It passes the hit through `DestroyBridgeFromCell_Low/High`, which classifies the overlay family and probes one/two cells behind before choosing the canonical seed. On edge-first scans this can shift the collapse footprint by one cell.

Presentation/RNG is also split incorrectly in Rust. `gamemd.exe` has two independent presentation layers:

1. CABHUT `CollapseBridge_*` walker pre-destroy `BridgeExplosions`: three perpendicular TWLT animations per walker step before `DestroyBridge_*` retries.
2. `CellClass::BlowUpBridge` per-cell fallout debris: optional metallic debris plus delayed `BridgeExplosions` after ground kill, deck DropIn, and collapsed-cell queue append.

Rust has only the aggregate post-collapse `spawn_bridge_debris` path, with wrong RNG ranges, centered coordinates, `BridgeVoxelMax` gating, and no TWLT report sound.

## Seed Canonicalization

Live read-only Ghidra rechecked:

- `MapClass__DestroyBridgeFromCell_High @ 0x005749C0`
- `MapClass__DestroyBridgeFromCell_Low @ 0x00574780`
- `FUN_00588C60 @ 0x00588C60`

The high and low functions are compiled twins with different overlay ranges.

High overlay subranges:

- NS subrange: `0xCD..=0xD5`, `0xDF..=0xE2`, `0xE7` -> `CollapseBridge_EW_High`
- EW subrange: `0xD6..=0xDE`, `0xE3..=0xE6`, `0xE8` -> `CollapseBridge_NS_High`

Low overlay subranges:

- NS subrange: `0x4A..=0x52`, `0x5C..=0x5F`, `0x64` -> `CollapseBridge_EW_Low`
- EW subrange: `0x53..=0x5B`, `0x60..=0x63`, `0x65` -> `CollapseBridge_NS_Low`

Canonicalization pattern:

- For the NS subrange, probe `(x, y - 1)` and `(x, y - 2)`.
- For the EW subrange, probe `(x - 1, y)` and `(x - 2, y)`.
- If the first back probe is outside the bridge band, call the collapse walker at `matched + 1`.
- If one back probe is inside but the second is outside, call at `matched`.
- If both back probes are inside, call at `matched - 1` or the equivalent helper-computed coordinate.

Player-visible implication: if the hut 5x5 scan first sees the edge cell of a bridge, gamemd walks one cell inward before the bounded four-step collapse. Rust starts on the edge cell, so the destroyed footprint can shift by one row/column.

Current Rust delta:

- `src/sim/world/bridge_orchestrator.rs:202` calls `find_destroy_overlay_seed`.
- `src/sim/world/bridge_orchestrator.rs:256` returns the first scan hit directly.
- `src/sim/world/bridge_orchestrator.rs:205` passes that uncanonicalized coordinate to `run_hut_collapse_bounded`.

## CABHUT Walker Presentation

Live read-only Ghidra rechecked:

- `MapClass__CollapseBridge_NS_High @ 0x00575BA0`
- `MapClass__CollapseBridge_EW_High @ 0x00575870`

Per walker step, before the `DestroyBridge_High` retry loop:

- If the center cell is not the terminal destroyed cap for that walker, the binary spawns three `BridgeExplosions` animations on perpendicular cells.
- For each of those three animations it consumes:
  - `RandomRanged(0, 0x7FFFFFFE)` for X jitter,
  - `RandomRanged(0, 0x7FFFFFFE)` for Y jitter,
  - `RandomRanged(1, 5)` for delay,
  - `RandomRanged(0, BridgeExplosions.ActiveCount - 1)` for the TWLT animation type.
- This happens before the per-step `DestroyBridge_*` retry loop.
- It uses `BridgeExplosions`; it does not spawn `MetallicDebris`.

Current Rust delta:

- `run_hut_collapse_bounded` goes straight from the four-step loop into `call_destroy_per_family`.
- No pre-destroy walker animation function exists.
- The first presentation event comes later from aggregate `spawn_bridge_debris`.

## BlowUpBridge Debris

Live read-only Ghidra rechecked:

- `CellClass__BlowUpBridge @ 0x0047DD70`

Binary order per actual `BlowUpBridge` cell:

1. Walk `FirstObject` and force-kill ground occupants with `C4Warhead`.
2. Walk `AltObject` and call `DropIn` for deck occupants.
3. Append the cell coordinate to the collapsed-cell queue.
4. If `BridgeExplosions.ActiveCount > 0`, run the debris block.
5. Outer 95 percent gate uses `RandomRanged(0, 0x7FFFFFFE)`.
6. Two jitter draws use `RandomRanged(0, 0x7FFFFFFE)`.
7. Metallic 50 percent gate uses `RandomRanged(0, 0x7FFFFFFE)`.
8. If metallic passes, select `MetallicDebris` with `RandomRanged(0, MetallicDebris.ActiveCount - 1)`.
9. Always attempt one delayed `BridgeExplosions` animation with `RandomRanged(1, 5)` delay and `RandomRanged(0, BridgeExplosions.ActiveCount - 1)` slot.

Negative facts:

- `BridgeVoxelMax` does not gate standard YR `BlowUpBridge` debris.
- Metallic debris alone does not enable the debris block; the outer active-count gate is `BridgeExplosions.ActiveCount > 0`.
- TWLT sounds are not hardcoded bridge-collapse sounds. They come from the selected TWLT anim's `StartSound` / fallback `Report` when the delayed animation starts.

Focused follow-up, 2026-05-22:

- Raw constants in `gamemd.exe` are:
  - `0x007E3570 = 1 / 2^31`,
  - `0x007E4F58 = 0.95`,
  - `0x007E1738 = 0.5`,
  - `0x007E4F50 = 50.0`.
- The outer debris gate is strict: `RandomRanged(0, 0x7FFFFFFE) * (1 / 2^31) < 0.95`.
  - Integer equivalent for Rust: pass when the inclusive draw is `<= 2_040_109_465` (`draw < 2_040_109_466`).
- The metallic debris gate is strict: `RandomRanged(0, 0x7FFFFFFE) * (1 / 2^31) < 0.5`.
  - Integer equivalent for Rust: pass when the inclusive draw is `< 0x4000_0000`.
- Both `CellClass__BlowUpBridge @ 0x0047DD70` and the CABHUT walker pre-destroy path use the same jitter transform:
  - base X = `cell_x * 256 + 128`,
  - base Y = `cell_y * 256 + 128`,
  - jittered coordinate = `Math__ftol(base + ((draw * (1 / 2^31)) - 0.5) * 50.0)`.
- `Math__ftol @ 0x007C5F00` sets x87 control word `0x0E7F` and uses `fistp`, so this conversion truncates toward zero.
  - For bridge/map coordinates, the base is positive, so an integer equivalent is:
    - `base - 25 + floor(draw * 50 / 2^31)`.
- Walker pre-destroy TWLTs and `BlowUpBridge` metallic/TWLT debris therefore use the same +/-25-lepton cell-center jitter shape.

Current Rust delta:

- `spawn_bridge_debris` runs over `destroyed_set`, not actual `blow_up_cells`.
- It gates no-op on both explosion and metallic lists being empty.
- It uses `next_range_u32(20)`, `next_range_u32(0xFFFF)` twice, and `next_range_u32(2)`.
- It reads `rules.bridge_rules.voxel_max`.
- It places effects at `CELL_CENTER_LEPTON`.
- `WorldEffect` has no selected anim report/start sound.

## Implementation Shape For Later

These should be separate fixes:

1. Add seed canonicalization before `run_hut_collapse_bounded`.
2. Add CABHUT walker pre-destroy `BridgeExplosions` in the bounded walker, before `call_destroy_per_family`.
3. Scope `DropIn` and `spawn_bridge_debris` to actual `BlowUpBridge` cells.
4. Fix `spawn_bridge_debris` RNG/gates/jitter and remove `BridgeVoxelMax` from the YR path.
5. Route selected TWLT `StartSound` / `Report` when the delayed world effect starts.

Do not fold these into `DestroyableBridges`, `[CombatDamage] DestroyableBridges`, or the skirmish `BridgeDestruction` option. Those are only entry gates, not collapse execution semantics.
