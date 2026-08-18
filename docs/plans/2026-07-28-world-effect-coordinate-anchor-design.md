# WorldEffect Coordinate Anchor Fix

**Status:** Approved by the user's explicit request to design and implement the fix.  
**Research authority:** `docs/research/TACTICAL_CELL_CENTER_ANIM_ANCHOR_GHIDRA_REPORT.md`

## Problem

Projectile endpoints and the `WorldEffect` spawned at the same world coordinate are
drawn 15 pixels apart. The simulation coordinate is correct; only the effect renderer
uses the wrong projection contract.

`gamemd.exe` projects an unowned impact `AnimClass` from its exact `CoordStruct`. The
current Rust projectile path already matches that flat cell/subcell formula. The
`WorldEffect` path instead converts the same coordinate with
`map::terrain::lepton_to_screen`, which adds a center baseline and then adds the
absolute `(128,128)` subcell contribution a second time.

## Player-Experience Ledger

| Trigger | Required result |
|---|---|
| A projectile detonates at a cell center | Explosion/impact animation is centered on the endpoint |
| A projectile detonates at a non-center subcell | Animation preserves the exact subcell anchor |
| A warp or other fixed `WorldEffect` starts | Effect uses the same native cell/subcell coordinate contract |
| A miner reaches ore from any direction | The requested resource cell is removed; no simulation offset is introduced |
| Terrain/resource tiles render | Existing terrain and atlas anchoring remains unchanged |
| Particles drift outside the map or above ground | Existing behavior remains unchanged pending its separate signed/elevation audit |

## Architecture Fit

The project already has the correct abstraction for this data shape:

```text
util::lepton::lepton_to_screen(rx, ry, sub_x, sub_y, z)
```

`WorldEffect` stores exactly those five fields. Its renderer should consume them
directly instead of first flattening them into an absolute `IVec3` and passing them to a
terrain/particle-oriented helper.

This is a presentation-only dependency:

```text
WorldEffect coordinate record
  -> canonical cell/subcell projector
  -> viewport culling and sprite instance
```

Combat, mining, resource overlays, pathfinding, and effect lifecycle remain outside the
change.

## Considered Approaches

### A. Route only `WorldEffect` through the canonical cell/subcell projector

Change `build_world_effect_instances` to call the same helper used by projectile
endpoints.

- Matches the verified native impact-animation mechanism.
- Removes the observed `(0,+15)` disagreement.
- Does not broaden into particle elevation or signed off-map coordinates.
- Smallest blast radius and easiest exact regression.

**Selected.**

### B. Rewrite `map::terrain::lepton_to_screen`

This would also alter particle rendering. Particle coordinates use signed absolute
leptons and absolute Z, while the native signed rounding and `AdjustForZ` behavior differ
from the current helper. The present research does not prove that migration.

**Rejected for this slice.**

### C. Migrate `iso_to_screen` and all tile consumers

Native terrain origin Y differs from the helper's documented formula, but current
terrain/overlay atlas paths contain compensating offsets. A global migration would
touch many unrelated tile, click, camera, and UI consumers.

**Rejected as architecture drift.**

## Implementation

1. Replace the temporary absolute `IVec3` construction in
   `build_world_effect_instances`.
2. Add a small `world_effect_screen_position` helper so the production projection can
   be tested without constructing renderer/atlas state.
3. Project through `crate::util::lepton::lepton_to_screen`.
4. Convert the diagnostic endpoint/effect test into a regression that compares the
   production helper across center and asymmetric subcell samples.
5. Retain the four-direction miner trace as a simulation regression.

## Acceptance

- At `(10,10,128,128,0)`, both endpoint and effect project to `(0,315)`.
- At `(23,20,128,128,0)`, both project to `(90,660)`.
- At `(41,17,128,128,0)`, both project to `(720,885)`.
- At asymmetric subcells, both paths remain identical.
- Four-direction miner trace still clears only the target resource/overlay.
- Scoped library tests pass.

## Residuals

- Signed negative absolute particle coordinates do not use native truncation today.
- Particle Z uses a simplified height projection rather than the exact runtime
  `AdjustForZ` mechanism.
- Terrain `iso_to_screen` has a compensated origin convention that needs a dedicated
  migration design if player-visible terrain misalignment remains after this fix.
