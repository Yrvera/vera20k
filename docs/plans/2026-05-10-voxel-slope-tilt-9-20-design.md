# Voxel Slope Tilt Rendering — Slopes 9-20 Design

## Goal

Extend the VXL slope-tilt renderer to cover gamemd.exe's full populated slope-matrix
table (slopes 9-16) and apply a defensive identity clamp for the unpopulated entries
(slopes 17-20), so ground vehicles tilt correctly on every ramp variant a YR map can
emit.

## Architecture Context

The VXL slope-tilt pipeline is layered as follows:

1. **Sim:** `cell.slope_type: u8` (`src/map/resolved_terrain.rs:79`) is populated at
   map-load time directly from the parsed TMP tile's `ramp_type` byte
   (`resolved_terrain.rs:1132`), with no transformation. Sim never reads it back —
   it is render-side data carried on the cell record.
2. **Consumer (render hand-off):** `src/app_instances/units.rs:81-87` reads
   `cell.slope_type` per voxel entity per frame and currently clamps `> 8 → 0`
   before passing it to the atlas key. Aircraft are forced to 0 (no tilt).
3. **Atlas pre-render:** `src/render/unit_atlas.rs` builds one pre-rendered sprite
   per `(type_id, facing, house_color, layer, frame, slope_type)` tuple at map-load
   time. Three call sites (`:210-211`, `:333-334`, `:432-433`) pre-render `0..=8` for
   ground vehicles. Cached sprites are keyed by `slope_type` in `UnitSpriteKey`
   (`unit_atlas.rs:67`).
4. **Atlas fallback:** `src/app_instances/units.rs:281-298` provides a runtime
   safety net — if the atlas misses on `(slope_type=N)`, it falls back to
   `slope_type=0`. This means a slope variant that isn't pre-rendered renders flat
   instead of disappearing.
5. **Render core:** `compute_slope_rotation` (`src/render/vxl_raster.rs:255-273`)
   builds `Rz(compass) · Rx(tilt) · Rz(-compass)` per slope_type. Currently
   covers slopes 0-8; returns `Mat4::IDENTITY` for 9+.

The sim/render layering invariant is preserved: `slope_type` is read off the cell
sim-side, but the matrix construction is render-only. No determinism impact —
slope rendering does not feed back into sim.

The full populated slope-matrix table from gamemd.exe is documented in
`ra2-rust-game-docs/VOXEL_SLOPE_TILT_SYSTEM.md` under the "Slope Matrix Table —
Full Entry List" addendum (verified 2026-05-10).

## Impact Analysis

| File | Change | Risk |
|---|---|---|
| `src/render/vxl_raster.rs:255-273` | Extend `compute_slope_rotation` match with 8 new arms (slopes 9-16). Update doc comment. Update `slope_type` field doc on `VxlRenderParams` (`:67-68`). | Pure addition. Slopes 1-8 untouched. Defensive `_ => Mat4::IDENTITY` covers 17+. |
| `src/app_instances/units.rs:81-87` | Widen consumer clamp from `<= 8` to `<= 16`. Bytes 17-20 fold to 0 (flat) at the boundary. | Low. The atlas-fallback safety net at `:281-298` already handles unknown slope keys. |
| `src/render/unit_atlas.rs:210-211, 333-334, 432-433` | Widen pre-render slope range from `0..=8` to `0..=16` at all three call sites. Update `slope_type` field doc on `UnitSpriteKey` (`:65-67`). | Atlas grows by ~89% in slope variants per ground vehicle. Existing single-texture retry-grow logic at `unit_atlas.rs:1094-1132` may double atlas width. If overflow warning at `:1121-1130` fires, multi-page packing for the unit atlas becomes a follow-up task. |
| `src/render/vxl_raster.rs` (tests) | Add `test_slope_9_uses_corner_tilt_at_nw` and `test_slope_13_uses_edge_tilt_at_nw` mirroring the existing `test_slope_4_geometry_locks_current_direction` lockdown style. | Low. |
| Diagnostic log | Emit a single throttled `warn!` the first time a `slope_type >= 17` byte is observed in `units.rs`'s consumer clamp, naming the cell coords. Aids deferred TMP empirical scan. | Low. One-shot via `OnceCell` or `AtomicBool` to keep the per-tick path branch-free. |

**No sim impact.** No tick-order changes, no determinism impact, no state-hash. The
slope_type byte already flows sim → render; we are only widening the value range
that the render side accepts.

## Chosen Approach

**Q1 (slopes 9-16 code shape):** extend the match arm in `compute_slope_rotation`,
parallel to the existing 1-8 entries. 8 new arms, ~12 lines added. Keeps a 1:1
mapping between the addendum's per-entry table and the Rust match.

**Q2 + Q3 (slopes 17-20 + consumer clamp):** clamp `slope_type > 16 → 0` at the
consumer in `units.rs`. Slopes 17-20 will render the unit flat (no tilt) instead
of invisible. `compute_slope_rotation` keeps its `_ => Mat4::IDENTITY` arm as
defense in depth.

**Q4 (atlas range):** pre-render `0..=16`. Three call sites updated. Bytes 17-20
never reach the atlas key because the consumer clamp folds them to 0.

**Q6 (TMP empirical scan):** deferred. A throttled `warn!` log on first
`slope_type >= 17` contact will surface the question at runtime if a real map
emits those bytes.

### Why option (b) over option (a) for slopes 17-20

The all-zero matrix at `DAT_00b454B8` is unfilled BSS, not a deliberate output
(see ledger item L10). The CLAUDE.md parity bar reads as "internals modernized,
outputs preserved" — and the bar applies to outputs gamemd.exe *intends*.
Reproducing an unintended bug whose visible failure mode is "unit disappears"
is exactly the kind of player-visible regression the parity bar exists to
prevent. If a real YR map ever emits 17-20, gamemd shows nothing and we show a
flat unit; our deviation is strictly better. If no real map ever emits them,
both renderers produce nothing observable and the deviation never matters.
Identity-clamp is no worse than zero-matrix in any scenario, and strictly
better in the worst case.

## Tiny-Detail Ledger

Constraints carried into `/write-plan` and implementation. Each item must have a
home in the implementation; if any drops out during coding, that's a parity bug.

| # | Detail | Source | Implementation home |
|---|---|---|---|
| L1 | Slope 9: compass=5π/4 (NW, 225°), tilt=CORNER (0.385_882_7 rad). Byte-identical to slope 5. | doc: `VOXEL_SLOPE_TILT_SYSTEM.md` "Per-entry breakdown" row 9 | match arm in `compute_slope_rotation` |
| L2 | Slope 10: compass=3π/4 (NE, 135°), tilt=CORNER. Byte-identical to slope 6. | doc row 10 | match arm |
| L3 | Slope 11: compass=π/4 (SE, 45°), tilt=CORNER. Byte-identical to slope 7. | doc row 11 | match arm |
| L4 | Slope 12: compass=7π/4 (SW, 315°), tilt=CORNER. Byte-identical to slope 8. | doc row 12 | match arm |
| L5 | Slope 13: compass=5π/4 (NW), tilt=EDGE (0.521_476_7 rad). NEW combo not in 1-8. | doc row 13 | match arm |
| L6 | Slope 14: compass=3π/4 (NE), tilt=EDGE. NEW combo. | doc row 14 | match arm |
| L7 | Slope 15: compass=π/4 (SE), tilt=EDGE. NEW combo. | doc row 15 | match arm |
| L8 | Slope 16: compass=7π/4 (SW), tilt=EDGE. NEW combo. | doc row 16 | match arm |
| L9 | Same `Rz(compass) · Rx(tilt) · Rz(-compass)` composition for 9-16 as 1-8 — same sign convention, no negated/half angles. | doc: "Matrix builder" §; "sign convention applies uniformly to all 16 populated slope entries" | inherited from existing builder line `Mat4::from_rotation_z(c) * Mat4::from_rotation_x(t) * Mat4::from_rotation_z(-c)` |
| L10 | Slopes 17-20: gamemd has all-zero matrix (BSS-zero) → unit invisible. Decision: identity-clamp instead. | doc: "DAT_00b454B8 ... NOT POPULATED"; GHIDRA `inspect_memory_content @ 0x00B454B8` | consumer clamp in `units.rs` folds 17+ to 0; defensive `_ => Mat4::IDENTITY` in `compute_slope_rotation` |
| L11 | Compass literals are exactly 8 IEEE-754 values; no half-angles, no negated forms; same set used by 1-8 and 9-16. | doc: "Compass angle constants" | reuse the 4 corner-direction literals (3.9270, 2.3562, 0.7854, 5.4978) already present for slopes 5-8 |
| L12 | TS++ enum names for 9-12 (`MidNW/NE/SE/SW`) and 13-16 (`SteepSE/SW/NW/NE`) are misleading — binary's actual ordering for 13-16 is NW/NE/SE/SW. | doc: "TS++ enum semantics vs. binary behavior" | comments use the binary's actual NW/NE/SE/SW ordering, not TS++ "Mid"/"Steep" names; never name a Rust constant after the TS++ enum |
| L13 | gamemd applies no bounds clamp at the matrix lookup site — relies on TMP data staying ≤16. | doc: "VXL_GetFacingMatrix … no bounds check, no mask" | our consumer-side clamp at `units.rs:81-87` is the equivalent boundary; defensive `_` arm in match is the inner safety net |
| L14 | slope_type=0 must early-return identity (matches gamemd's caller-side early-out). | `vxl_raster.rs:257`; doc: "both Draw_Matrix and Turret_barrel_tilt early-out on slope_type == 0" | preserved — first match arm `0 => return Mat4::IDENTITY` stays |

## Design

### Components

No new components. All changes are in-place edits to three existing files
(`vxl_raster.rs`, `units.rs`, `unit_atlas.rs`).

### Interfaces / Contracts

`UnitSpriteKey.slope_type`: documented range widens from `0..=8` to `0..=16`.
The Rust type stays `u8`. Existing `Eq`/`Hash` derives don't need changing.

`VxlRenderParams.slope_type`: same — doc widens from `0..=8` to `0..=16`.

`compute_slope_rotation(slope_type: u8) -> Mat4`: signature unchanged. Behavior
extended to handle 9-16 with explicit matrices; unchanged for 17+ (identity).

The cell→render flow stays identical. The only externally observable change is
that ground vehicles on slope_type 9-16 cells now visually tilt instead of
rendering flat.

### Data Flow

```
TMP tile +0x2A (parsed at map load)
   └→ ResolvedCell.slope_type: u8                         [sim-side]
        └→ units.rs build_unit_instances loop (per frame, per voxel entity)
             └→ clamp: slope_type > 16 → 0                [render-side hand-off]
                  └→ UnitSpriteKey { slope_type, ... }    [atlas lookup key]
                       ├→ atlas hit  → entry → SpriteInstance
                       └→ atlas miss → fallback to slope_type=0 (flat)

Atlas build (map load):
   for each ground vehicle: pre-render slopes 0..=16 × facings × frames × layers
        └→ compute_slope_rotation(slope_type) inside vxl_raster::prepare_limb_data
```

### Error Handling

No new error paths. The defensive `_ => Mat4::IDENTITY` in
`compute_slope_rotation` covers any value that slips past the consumer clamp.
The runtime atlas-miss → `slope_type=0` fallback in `units.rs:281-298` covers
any cache-miss path (e.g. mid-rebuild).

### Testing Strategy

Unit tests in `vxl_raster.rs::tests`:

1. `test_slope_9_aliases_slope_5_geometry` — verify slope 9 produces the same
   matrix as slope 5 (byte-identical aliases per L1).
2. `test_slope_13_uses_edge_tilt_at_nw` — verify slope 13's `+Y` and `-Y`
   transforms expose `EDGE_TILT_RAD.sin()` magnitudes at the NW compass
   (mirrors existing `test_slope_4_geometry_locks_current_direction` style).
3. `test_slopes_17_to_20_return_identity` — verify the defensive arm.

No integration / GPU test changes. Visual smoke check: place a ground vehicle
on each of slopes 9-16 in a sandbox map and confirm tilt direction matches
gamemd.exe (eyeball comparison against retail screenshots).

### Determinism Considerations

None. Slope rendering is render-only — no sim state, no tick ordering, no state
hash. The change cannot affect lockstep determinism.

## Architectural Decisions

**Patterns followed:**
- Match-arm extension parallels existing slopes 1-8 — same code shape, same
  comment conventions, same compass-degree annotations.
- Consumer-side clamp at the sim/render boundary is the established pattern
  (the existing `> 8 → 0` clamp at `units.rs:81-87` is what we're widening).
- Atlas key seeding mirrors the existing 9-slope range already at three call
  sites; we just widen the range, no new call sites.

**Patterns deviated from:**
- None.

**Tech debt introduced:**
- Atlas memory grows ~89% in slope variants per ground vehicle. If atlas
  overflow warning fires, follow-up work is to extend the unit atlas to
  multi-page packing (sprite atlas already has it per `feedback_multi_atlas`).
- Body vs. turret read-width asymmetry (cached locomotor+0x18 dword vs. fresh
  cell+0x11C byte read) is a separate parity gap noted in the addendum's "Body
  vs. turret read width" §. Not addressed here.

**Known deferred follow-ups:**
1. **TMP empirical scan (Q6 from `/plan-investigation`)** — tally byte +0x2A
   distribution across retail YR map TMPs. Will surface if standard maps emit
   slope_type ≥ 17 in practice. Until then, the `warn!` log on first contact
   serves as a runtime tripwire.
2. **Body vs. turret slope-source asymmetry** — separate investigation; tied
   to slope-transition smoothing, not slope magnitude.
3. **Slope transition interpolation** (`Force_New_Slope` zeroes the transition
   timer; some other code path must drive the interpolated path). Not in
   scope; tracked in addendum's "Open questions" §2.
4. **Multi-page unit atlas packing** — only if the post-change atlas overflow
   warning fires.

## Alternatives Considered

**For slopes 17-20: match gamemd's zero matrix (option a).** Rejected —
reproduces an unintended bug whose visible failure mode (unit disappears) is
exactly what the parity bar exists to prevent. CLAUDE.md "internals modernized,
outputs preserved" applies to outputs gamemd intends; BSS-zero is not an output,
it's an unfilled slot.

**For slopes 17-20: extrapolate to a new EDGE-at-corner pattern (option c).**
Rejected — speculation that diverges from gamemd in the opposite direction
without evidence. If real maps DO emit 17-20, our guess could be more wrong
than identity.

**For code shape: lookup table indexed by slope_type.** Rejected — buys nothing
for a fixed table this small; less greppable than a match arm; needs special-
case for slope 0's identity early-out anyway.

**For code shape: helper that decomposes (compass_dir, tilt_kind).** Rejected —
over-engineered for 17 fixed entries that aren't extensible.

**For atlas range: pre-render 0..=20.** Rejected — wastes ~24% of atlas slots
on slope_type values that visually equal slope 0 (since `compute_slope_rotation`
returns identity for 17-20).

**For atlas range: keep 0..=8 + incremental rebuild on first 9-16 contact.**
Rejected — incremental rebuild causes a per-encounter stutter; better to pay
the memory cost upfront at map load.

**For TMP scan: run inline.** Rejected — does not change the chosen approach
(option b is correct regardless of frequency); only changes severity
classification. Better to land the fix and surface the data via runtime warn-log.
