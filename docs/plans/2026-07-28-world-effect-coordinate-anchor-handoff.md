# WorldEffect Coordinate Anchor Handoff

**Branch:** `feature/coordinate-runtime-trace-20260728`  
**Worktree:** `<local>/Documents/ra2-rust-game-coordinate-runtime-trace`  
**Base:** `6f6ec58e`  
**Commit:** `3b4ad180eb25eaefe7f66b0ae1d833aa4c17cf76`

## Result

Fixed the 15-pixel half-tile mismatch between projectile/action coordinates and fixed
`WorldEffect` animations. Native evidence shows unowned `AnimClass` effects project
their exact `CoordStruct`; Rust now routes `WorldEffect` through the same cell/subcell
projector as projectile endpoints.

Runtime tracing also proved miner extraction already mutates and clears the intended
resource/overlay cell from west, east, north, and south approaches. No miner simulation
offset was introduced.

## Changed Files

- `src/app_instances/overlays.rs`
- `src/app_fire_effects.rs`
- `src/sim/miner/miner_tests.rs`

## Validation

```text
cargo test -p vera20k --lib coordinate_runtime_trace -- --nocapture
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 4984 filtered out
```

```text
cargo test -p vera20k --lib world_effect_projection_preserves_exact_subcell_anchor
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4985 filtered out
```

The fire trace now reports `(0.0,0.0)` delta at `(10,10)`, `(23,20)`, and `(41,17)`.
The miner trace reports the target removed/overlay cleared and the behind resource and
overlay preserved for all four approaches.

The existing 48 library warnings remain; this slice introduced no new warning.
The feature worktree is clean and no Cargo/rustc process remains.

## Residuals

- Signed/elevated particle projection still uses a separate simplified absolute helper.
- Terrain `iso_to_screen` retains its compensated origin convention; migrating it needs
  a separate bounded design.

## Next Safe Action

After the current `dev` owner finishes and the main checkout is clean, merge or
cherry-pick `3b4ad180` into `dev`, resolve only genuine upstream overlap, then run the
full suite exactly once as the merge-to-`dev` certification step.
