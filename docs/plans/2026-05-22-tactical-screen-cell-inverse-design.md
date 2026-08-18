# Tactical Screen-To-Cell Inverse Design

## Goal

Make Rust tactical cursor-to-cell resolution match active YR `gamemd.exe` behavior for height, bridge, viewport, and off-map edge cases.

## Architecture Context

The current Rust input path is already centralized enough for a narrow parity fix. UI/input callers pass screen coordinates to `app_sim_tick::screen_point_to_world_cell`, which applies `screen / zoom + camera`, then delegates to `world_point_to_cell`. That wrapper calls `map::terrain::screen_to_iso_with_height_and_bridges`.

The current mismatch is concentrated in `src/map/terrain.rs`: `screen_to_iso_with_height_and_bridges` does a 3-pass height refinement and then a 7x7 closest-bridge search. Verified YR behavior in `TACTICAL_SCREEN_PIXEL_TO_CELL_INVERSE_RECHECK_GHIDRA_REPORT.md` is a vertical pixel scan with an effective 180 failed-attempt cap, sentinel fallback behavior, cardinal bridge checks, orientation-gated bridge adjustment, and strict `>15` edge thresholds.

The design must not disturb forward render helpers such as `iso_to_screen` and `lepton_to_screen`. Those helpers are broadly used by terrain rendering, entities, overlays, minimap/debug views, and tests. The parity target here is the tactical inverse only.

## Impact Analysis

Primary files:

- `src/map/terrain.rs`: add a tactical inverse context and replace the height/bridge inverse implementation used for tactical input.
- `src/app_sim_tick.rs`: pass app-level camera/viewport input into the new inverse contract and keep screen-to-cell ownership centralized.
- `src/map/resolved_terrain.rs` or existing bridge-map construction surfaces: expose enough bridge metadata for the inverse branch, not only deck height.
- Tests in `src/map/terrain.rs` and `src/app_sim_tick.rs`: cover scan cap, bridge cardinal behavior, viewport boundary behavior, and invalid/negative inputs.

Player-visible dependent paths:

- cursor cell hover,
- right-click move/attack orders,
- building placement preview and click fallback,
- superweapon target cell,
- object/foundation hit testing after cell resolution.

Risk areas:

- Changing `screen_to_iso` directly would affect render/minimap assumptions; avoid that.
- The current `bridge_height_map` lacks `CellClass+0x140` style flags and orientation bit `0x800`; exact bridge parity needs richer metadata.
- Rust currently clamps negative inverse output to `(0,0)`. YR has caller-specific negative guards and a sentinel cell fallback. The app wrapper must make that contract explicit.

## Chosen Approach

Use Approach A: add a tactical inverse context rather than patching the existing approximation in place.

The new API should keep forward projection helpers stable and introduce a tactical-only inverse owner, conceptually:

```rust
pub struct TacticalInverseContext<'a> {
    pub height_map: &'a BTreeMap<(u16, u16), u8>,
    pub bridge_cells: Option<&'a BTreeMap<(u16, u16), TacticalBridgeCell>>,
    pub viewport_offset: glam::Vec2,
}

pub struct TacticalBridgeCell {
    pub deck_z: u8,
    pub structural: bool,
    pub orientation_north_south: bool,
}
```

Exact names can follow local style during implementation. The important contract is that bridge inverse receives structural/orientation data, not only deck height.

## Tiny-Detail Ledger

- `0x006D6590` is active in YR and is called by tactical pick, radar update, cursor/cell wrappers, and mouse/display paths. Source: `TACTICAL_SCREEN_PIXEL_TO_CELL_INVERSE_RECHECK_GHIDRA_REPORT.md`.
- The inverse subtracts `g_RadarViewportOffsetX/Y` internally; some callers pre-add it before calling. Source: same report, section 3.1.
- The initial fallback cell is computed before scan; scan-cap failure returns that fallback. Source: same report, section 3.2.
- Height behavior is a vertical pixel scan, not 3-pass convergence. Source: same report, section 3.3.
- Scan starts from `(input_y - viewport_offset_y) + Tactical+0xB4`. Source: same report, section 3.3.
- Each failed scan decrements by exactly one screen pixel. Source: same report, section 3.3.
- Loop bound compares the incremented counter against `0xB4`; effective cap is 180 failed attempts. Source: same report, section 3.3.
- Cell height uses `CellClass+0x11B`, multiplied by 15 screen pixels. Source: same report, section 3.3.
- Signed cell conversion uses the `value + (sign & 0xFF) >> 8` pattern before packing shorts. Source: same report, section 3.3.
- Bridge branch runs only when `CellClass+0x140 & 0x100` is set. Source: same report, section 3.4.
- Bridge branch uses cardinal directions `2`, `4`, and conditionally `0` or `6`, not a radial search. Source: same report, section 3.4.
- `CellClass+0x140 & 0x800` selects the direction-zero/N-S bridge orientation behavior. Source: same report and `BRIDGE_AXIS_AND_CARDINAL_POLARITY_GHIDRA_REPORT.md`.
- Bridge edge threshold is strict `> 0xF`, not `>= 0xF`. Source: tactical inverse recheck, section 3.4.
- Direct bridge neighbor returns are only `+Y` or `+X`; dir0/dir6 edge flags gate extra 60-pixel bridge adjustment. Source: tactical inverse recheck, section 3.4.
- Direction ids are `0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW`; `8` is tube special, not a normal neighbor. Source: `CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md`.
- Off-map cell lookup returns a sentinel `DAT_00ABDC50`; negative-input guard is caller-specific. Source: tactical inverse recheck, section 3.5.

## Design

### Components

`map::terrain` owns the pure inverse math. It should expose a tactical inverse function that accepts world/client pixel coordinates plus a context object. Existing `screen_to_iso` can remain a low-level helper for simple z=0 math and tests, but tactical input should route through the new parity path.

`app_sim_tick` remains the app-level owner for converting window/screen coordinates to tactical/world coordinates. It should decide whether the input is full-window, tactical-local, radar-local, or already world-space. The wrapper should pass an explicit viewport offset into the terrain inverse instead of leaving offset behavior implicit.

Bridge metadata should be derived from existing resolved bridge state where possible. The tactical inverse does not need full bridge damage logic; it needs a per-cell view of:

- deck height,
- structural bridge body flag equivalent to `0x100`,
- orientation bit equivalent to `0x800`.

### Interfaces / Contracts

The tactical inverse should return a float or cell result plus enough status for wrappers to decide clamping/fallback:

```rust
pub enum TacticalInverseResult {
    Cell { rx: f32, ry: f32 },
    Fallback { rx: f32, ry: f32 },
}
```

If implementation simplicity favors returning `(f32, f32)`, the fallback behavior must still be covered by tests and comments.

The app-level `world_point_to_cell` should be the only place converting the result to `(u16, u16)` for gameplay/UI callers. Negative and off-map behavior should not be buried inside the low-level math helper.

### Data Flow

1. UI caller provides screen pixel.
2. `screen_point_to_world_cell` converts via zoom/camera and applies explicit tactical viewport contract.
3. `world_point_to_cell` calls the tactical inverse with height and bridge context.
4. Terrain inverse computes fallback cell.
5. Terrain inverse performs up to 180 failed vertical scan attempts:
   - solve candidate cell,
   - read height,
   - apply bridge branch if structural bridge cell,
   - return when adjusted scan Y reaches the input threshold,
   - otherwise decrement scan Y by 1.
6. On cap exhaustion, return fallback cell.
7. App wrapper converts to `u16` cell for downstream systems.

### Bridge Branch

Do not search a 7x7 neighborhood. For the candidate structural bridge cell:

- inspect E (`2`) and S (`4`),
- inspect N (`0`) when orientation bit `0x800` is set,
- inspect W (`6`) when orientation bit `0x800` is clear,
- only direct-return `+Y` or `+X` candidates as verified,
- use dir0/dir6 open-edge flags only for the extra 60-pixel bridge adjustment,
- use strict `> 15` thresholds.

This branch should use explicit cardinal constants in the tactical inverse module or a tiny local helper. Do not reuse a generic helper that wraps `direction & 7` without first validating the direction, because direction `8` has special tube meaning elsewhere.

### Error Handling

No panics for off-map inputs. Low-level inverse should compute a fallback and allow sentinel-like behavior to be represented by the app wrapper. If the app wrapper must clamp to map bounds for current Rust safety, that clamp should be documented as the Rust app contract and tested separately from binary sentinel behavior.

### Testing Strategy

Unit tests in `terrain.rs`:

- height scan uses more than 3 attempts and can still resolve the intended cell;
- cap/fallback returns the initial cell when no scan candidate satisfies the threshold;
- scan decrements by one pixel per failed attempt, tested with a small instrumented height map if needed;
- bridge branch uses cardinal candidates, not radial closest;
- strict `>15` behavior differs from `>=15`;
- direction `8` is not accepted as a tactical bridge neighbor.

App wrapper tests in `app_sim_tick.rs`:

- `screen_point_to_world_cell` preserves the tactical viewport contract at sidebar boundary;
- negative/full-window edge inputs do not silently become ordinary tactical cells without the documented guard;
- building placement preview and fallback click use the same inverse result path.

Regression tests should prefer small synthetic maps over full app startup. No sim determinism risk is expected because this is input/UI conversion, not a simulation tick mutation path, but the output cell feeds deterministic commands after player input.

## Architectural Decisions

- Keep forward render helpers unchanged. The verified patch scope is tactical inverse parity, not a render-origin rewrite.
- Add richer bridge metadata rather than encoding bridge orientation into the existing deck-height-only map. The existing map is too weak to preserve the ledger.
- Keep `sim/` isolated. The app/map layer may consume resolved bridge metadata, but `sim/` must not depend on rendering/UI.
- Prefer named constants for `180`, `15`, `60`, and cardinal direction ids.
- Add small explicit types over booleans if the bridge orientation naming becomes unclear.

## Alternatives Considered

### Patch Existing Function In Place

This is tempting because it touches fewer call sites, but the current signature only receives height maps and optional bridge deck height. It cannot express the verified structural/orientation bridge branch without either hidden side channels or another parameter. That makes exact parity easy to fake and hard to test.

### Two-Phase Fix

Implementing height scan first and bridge branch later would reduce review size, but it leaves known player-visible bridge endpoint click drift. This is acceptable only if explicitly chosen as a temporary parity gap. The chosen design avoids that gap by giving bridge metadata a first-class home.

## Hand-Off Notes

Implementation should start with the terrain inverse tests, then add the context type and bridge metadata adapter, then switch `world_point_to_cell` to the new path. Do not refactor render coordinate helpers or building coordinate helpers as part of this task.
