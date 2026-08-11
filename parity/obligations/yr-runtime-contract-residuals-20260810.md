# YR runtime contract integration residuals

Authority: the pinned `yr_1001` contracts in `RA2-GAME.EXE-IDB`, especially the
2026-08-09 CellClass completion audit and runtime-contracts goal audit.

## Integrated

- `IsClearToMove` is the shared CellRect passability leaf. Ordered normal and
  alternate object-list queries, infantry owner planes, native-order scatter,
  and normal-list projectile building lookup are represented.
- Foot/path terrain entry, production preview/commit, parachuted payload
  landing, aircraft pad admission, and DropPod virtual-Unlimbo admission now
  route through their native Cell wrappers rather than bespoke emptiness tests.
- Persistent projectile flight uses the floor/bridge/building/overlay collision
  kernel, target clamp/slope response, and ordered special-detonation selection.
- Airburst/Cluster executes in native detonation/RNG order, and ordinary
  Shrapnel targets the normal-list head before constructing synchronized-random
  fallback children.
- Type-0 Wave damage consumes recorded cells in wave, cell-vector, then selected
  Cell-list order, including the final lifetime tick.
- Point-light updates enumerate changed source areas and atomically commit the
  reverse-gathered batch. Complete Ion lighting inputs are selected while the
  represented Lightning Storm state is active.
- Type-16 Spotlight masks are generated procedurally and uploaded to an exact
  destination-factor GPU pipeline without external assets.
- Fogged-object shared footprint lifetime, signed per-house sensor counters,
  and per-cell cloak-owner words are serialized and hashed; SHP/VXL draw state
  consumes represented sensor coverage.
- Snapshot schema 52 hashes and round-trips the new projectile, Wave, infantry
  owner, fogged-object, sensor, and cloak-owner state.

## Explicit residuals

- Nearby CellRect callers without a raw grid still project list occupancy to
  generic bit `0x40`. A* and runtime `+0x1AC` retain their later class-specific
  raw/list blocker arms. Parsed overlays lack the crusher byte at `+0x22D`.
- Jumpjet and legacy aircraft landing lack the required path/raw/fog inputs;
  general aircraft occupant-alliance/type and per-house shroud gates remain
  unrepresented. Production lacks editor/global bypass and `+0xE58` inputs.
- The two raw Building collision exemptions have no represented runtime
  producers. `ConnectsToOverlay` is currently limited to the represented wall
  family.
- Special-detonation effect bodies are unsupported by design. Their ordered
  first-true branches still shadow later branches and ordinary damage.
- Cluster cannot stop early after an unsupported special body removes its
  parent bullet, and Shrapnel's native double vector is projected into the
  existing integer velocity/gravity representation.
- Wave `UpdateCells` production, wall damage, and cliff tails lack represented
  inputs. The ordered consumer accepts only evidence-backed recorded cells.
- Spotlight child coordinate/angle production is absent, so no instance is
  synthesized at the parent. The BGRA/RGBA sRGB target cannot reproduce native
  RGB565 extraction/repack rounding.
- Psychic Dominator and Nuclear Flash scenario-light states are not represented.
- FreezeInFog draw payload/producer, cloak-generator and sensor activation
  producers, and sensor occupant/building notifications remain unwired.

No residual above is approximated with a semantically unrelated flag, stable-ID
sort, alpha blend, or unsynchronized RNG source.
