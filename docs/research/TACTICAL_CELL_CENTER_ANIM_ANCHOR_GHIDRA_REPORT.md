# Tactical Cell-Center Animation Anchor - Ghidra Research Report

**Address(es):** `0x0041BE00`, `0x00422BE0`, `0x00422CA0`, `0x00469A90`, `0x005F4B10`, `0x005F65A0`, `0x006D1EB0`, `0x006D1FE0`, `0x006D20E0`, `0x006D2140`, `0x006D6D10`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Active Yuri's Revenge conversion and draw-anchor path for a flat cell-centered `CoordStruct` used by projectile destinations and ordinary unowned `AnimClass` impact animations, contrasted with terrain-tile origin positioning.  
**Non-Scope:** tactical screen-to-cell inverse, full projectile flight simulation, every `AnimClass` material/palette branch, building foundation targeting, noncanonical camera matrices, and global draw-order exactification.  
**Confidence:** High for the flat-ground center/origin distinction and impact-animation owner chain; medium for elevation outside this slice because the runtime `AdjustForZ` multiplier is not captured here.  
**Active in YR:** Yes.

## 0. Investigation Question

The Rust runtime trace showed a projectile endpoint and the `WorldEffect` spawned at
that exact endpoint disagreeing by `(0,+15)` screen pixels. It also showed four miners
entering harvest at subcell `(128,128)` and clearing the intended resource/overlay cell,
not the cell behind them. The question was whether `gamemd.exe` projects impact
animations from the exact lepton coordinate, or first converts them to a terrain-tile
origin that would justify Rust's extra half-tile.

## 1. Verdict

`gamemd.exe` does **not** move an ordinary unowned impact animation to a tile origin.
`BulletClass::Detonate` passes the exact impact `CoordStruct` into the `AnimClass`
constructor. The normal object draw owner asks the animation for that exact coordinate,
projects it through `TacticalClass::CoordsToClient2`, and passes the resulting point to
`AnimClass::DrawIt`.

For a flat cell center `(cx*256+128, cy*256+128, 0)`, the native point is:

```text
x = 30 * (cx - cy)
y = 15 * (cx + cy) + 15
```

Terrain-cell content uses a distinct path. `Cell_ContentRendering` deliberately
truncates a center coordinate back to `(cell*256, cell*256)`, projects that origin, then
subtracts 30 from X to obtain the tile bounding-box origin. There is no extra +15 in Y.

Rust has both concepts, but `build_world_effect_instances` converts the exact effect
coordinate with `map::terrain::lepton_to_screen`, whose formula already includes a
center baseline and then adds the absolute subcell contribution again. At subcell
`(128,128)`, that helper returns native center Y plus another 15 pixels. This is the
reported one-tile-row visual disparity. The mining mutation is not displaced.

## 2. Native Owner and Class Evidence

### 2.1 Concrete `AnimClass` vtable identity

- `vtable__AnimClass` begins at `0x007E3354`.
- The complete-object locator at `0x007E3350` points to `0x007FBA60`.
- Its type descriptor is `0x008182C8`, containing `.?AVAnimClass@@`.
- Vtable slot `+0x114` is stored at `0x007E3468` and points to
  `AnimClass::DrawIt @ 0x00422CA0`.

This ties the draw function to the concrete active `AnimClass` type rather than relying
on a label alone.

### 2.2 Coordinate fields and virtuals

| Item | Address / offset | Verified behavior |
|---|---:|---|
| `ObjectClass::Location` | `+0x9C/+0xA0/+0xA4` | Exact stored X/Y/Z dwords |
| `ObjectClass::GetCoords` | `0x005F65A0` | Returns those exact dwords |
| `AnimClass::GetCoords_WithOwnerOffset` | `0x00422BE0` | If `AnimClass+0xCC` has no owner, returns `ObjectClass::GetCoords`; otherwise adds owner coordinates |
| `ObjectClass::GetRenderCoords` | `0x0041BE00` | Calls virtual `+0x48` and copies the returned coordinate |
| `ObjectClass::DrawIt` | `0x005F4B10` | Calls virtual `+0xAC`, then `CoordsToClient2`, then virtual `+0x114` |

On the `AnimClass` vtable, `+0x48` resolves to `0x00422BE0`, `+0xAC` resolves to
`0x0041BE00`, and `+0x114` resolves to `0x00422CA0`. Therefore the complete ordinary
path is:

```text
AnimClass stored Location
  -> AnimClass::GetCoords_WithOwnerOffset
  -> ObjectClass::GetRenderCoords
  -> ObjectClass::DrawIt
  -> TacticalClass::CoordsToClient2
  -> AnimClass::DrawIt
```

No cell conversion or tile-origin helper occurs in this chain.

## 3. Impact Animation Construction

`BulletClass::Detonate @ 0x00469A90` copies its incoming impact coordinate to a local
`CoordStruct` at `0x00469AA4`. When `BulletType.Inviso` is set, the optional branch calls
the verified radius-`0x20` scatter helper and replaces that local coordinate. Otherwise
it remains exact.

At `0x00469C61..0x00469C79`, the function copies that same local X/Y/Z onto the
constructor argument area. At `0x00469C7C..0x00469C93`, it supplies:

```text
AnimClass(type, &impact_coords, delay=0, loop_count=1,
          draw_flags=0x2600, z_adjust=-15, reverse=false)
```

The `z_adjust=-15` argument is an ordering/depth input. It is not a screen-space Y
translation. This is consistent with
`INVISIBLELOW_DETONATION_COORDSTRUCT_GHIDRA_REPORT.md` and
`ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`.

## 4. Projection Math

### 4.1 Raw world-to-screen numerator

`Tactical__WorldToScreenSub @ 0x006D1EB0` computes:

```text
raw_x = (world_x * 60) / 2 + (world_y * -60) / 2
raw_y = (world_x * 30) / 2 + (world_y *  30) / 2
```

### 4.2 Full tactical projection

`TacticalClass::CoordsToClient2 @ 0x006D2140` uses the same raw X/Y, converts from
leptons to pixels with a signed bias before `>>8`, subtracts
`Tactical__AdjustForZ(z)`, then subtracts tactical viewport scroll:

```text
pixel_x = signed_trunc_256(raw_x)
pixel_y = signed_trunc_256(raw_y) - AdjustForZ(z)
client_x = pixel_x - camera_x
client_y = pixel_y - camera_y
```

The signed conversion is:

```text
(value + ((value >> 31) & 255)) >> 8
```

It truncates negative values toward zero rather than using Euclidean/floor division.
`TacticalClass::CellToPixel @ 0x006D1FE0` is the no-Z/no-scroll form.

### 4.3 Center and origin samples

For `(cx*256, cy*256, 0)`:

```text
x = 30 * (cx - cy)
y = 15 * (cx + cy)
```

For `(cx*256+128, cy*256+128, 0)`:

```text
x = 30 * (cx - cy)
y = 15 * (cx + cy) + 15
```

The center-origin delta is exactly `(0,+15)` for any ordinary cell whose arithmetic
does not overflow. Checked examples include `(0,0)`, `(10,10)`, `(23,20)`, and
`(41,17)`. At center subcells, the +128 terms cancel in X and add to one half-tile in Y.

### 4.4 Elevation

Native `CoordsToClient2` subtracts `Tactical__AdjustForZ(z)`, not an unconditional
`z*15`. A height level is represented elsewhere through a lepton-height conversion
whose ordinary result is approximately 15 screen pixels per level. The exact helper
uses a runtime multiplier, a threshold adjustment beginning at `z >= 728`, `+0.5`, and
`Math__ftol`.

The reported mismatch is present at `z=0`, where `AdjustForZ(0)=0`; elevation cannot
explain or cancel the extra 15 pixels. This report does not authorize a broad rewrite of
the particle/elevation projector.

## 5. `AnimClass::DrawIt` Position Versus Depth

In the standard non-tiled branch of `AnimClass::DrawIt @ 0x00422CA0`:

```text
screen_x = projected_input.x
screen_y = projected_input.y + AnimType.YDrawOffset
```

The instance `AnimClass.ZAdjust` is consumed separately in the shape depth expression:

```text
depth = AnimType.YDrawOffset
      + AnimClass.ZAdjust
      - Tactical__AdjustForZ(AnimClass::GetZAdjust())
      - 2
```

Flat and tiled branches vary the final depth constant, not the basic conclusion.
Neither `ZAdjust=-15` nor the low-level SHP canvas centering adds a hidden half-cell to
the world point. `YDrawOffset` is art-controlled and deliberately moves the sprite
relative to that world anchor.

## 6. Terrain Cell Content Is a Different Reference Point

`Cell_ContentRendering @ 0x006D6D10` begins with a cell center:

```text
cell_x * 256 + 128
cell_y * 256 + 128
```

It then applies the signed cell conversion and shifts back by 8, deliberately producing:

```text
cell_x * 256
cell_y * 256
```

After projection it subtracts `0x1E` (30) from screen X and leaves Y unchanged. Thus the
tile diamond's bounding-box origin is:

```text
x = 30 * (cell_x - cell_y) - 30
y = 15 * (cell_x + cell_y)
```

That is not an animation anchor and should not be reused as though it were one.

## 7. INI and Legacy Status

- `256` leptons per cell, center `128`, and projection scales `60/30` are hardcoded.
- `AnimType.YDrawOffset` can alter sprite placement after projection.
- `AnimClass.ZAdjust` affects the depth/order expression, not screen Y.
- No INI key selects center versus terrain origin for the core transform.
- The inspected `TacticalClass`, `ObjectClass`, `AnimClass`, and bullet detonation paths
  are active Yuri's Revenge paths, not dormant Tiberian Sun legacy branches.

## 8. Current Rust Status

| Rust surface | Current reference point | Native comparison |
|---|---|---|
| `src/util/lepton.rs::lepton_to_screen` | Cell + subcell coordinate; center `(128,128)` gives `+15` Y | Matches flat native `CoordsToClient2` center projection |
| `src/map/terrain.rs::iso_to_screen` | Documented tile bounding-box origin but returns `15*(x+y)+15` | Its Y is 15 px below native `Cell_ContentRendering` tile origin; existing atlas offsets compensate in some paths |
| `src/map/terrain.rs::lepton_to_screen(IVec3)` | Adds a `+15` center baseline, then adds absolute subcell contribution | Center `(128,128)` becomes `+30`, 15 px below native center |
| `src/app_fire_effects.rs` projectile endpoint | Uses `util::lepton::lepton_to_screen` | Correct flat center/subcell anchor |
| `src/app_instances/overlays.rs` `WorldEffect` | Converts exact effect coordinate through `terrain::lepton_to_screen` | Wrong by `(0,+15)` at center; this is the traced defect |
| `src/app_instances/particles.rs` particles | Also uses `terrain::lepton_to_screen` | Potentially related, but particle Z/negative-coordinate behavior is a separate slice |
| Miner extraction state | Mutates the arrived cell and clears its overlay | Runtime trace matches intended cell; no simulation-coordinate fix indicated |

Only `WorldEffect` and particles currently call the absolute-`IVec3` terrain helper.
Changing that helper globally would alter particle presentation, including negative
coordinates and elevation semantics that this slice did not fully verify. The
native-backed minimal repair is therefore to route `WorldEffect` through the existing
cell-plus-subcell projector already used by the projectile endpoint.

## 9. Coverage Ledger

| Question | Evidence | State |
|---|---|---|
| Active draw owner | `ObjectClass::DrawIt @ 0x005F4B10`, vtable chain | Covered |
| Exact animation coordinates | `0x00422BE0`, `0x005F65A0`, detonation `0x00469A90` | Covered |
| Projection and rounding | `0x006D1EB0`, `0x006D1FE0`, `0x006D2140` | Covered |
| Draw-time Y adjustments | `AnimClass::DrawIt @ 0x00422CA0` | Covered |
| Depth-only `ZAdjust` | `0x00422CA0`, prior audited depth report | Covered |
| Tile-origin contrast | `Cell_ContentRendering @ 0x006D6D10` | Covered |
| INI authority | Existing coordinate/AnimClass reports and stock INI scan | Covered |
| Rust callers | `rg` callsite inventory and runtime trace | Covered |
| Flat-ground boundary samples | Algebra plus runtime trace at three cells | Covered |
| Negative coordinates | Native rounding identified; current particle behavior compared | Deferred from implementation because ordinary map effects use non-negative cells |
| Exact elevated-particle behavior | `AdjustForZ` mechanism identified | Deferred to a particle/elevation slice |
| Pause/save/replay alternate transform | No alternate projector in object draw owner chain | Covered for rendering; lifecycle serialization is non-scope |

## 10. Adversarial Checks

1. **Could `z_adjust=-15` intentionally counter the 15-pixel shift?** No. The verified
   draw code uses it in depth, while screen Y uses only projected Y plus
   `YDrawOffset`.
2. **Could the impact animation be snapped to a cell first?** No on the ordinary path.
   The exact local `CoordStruct` is copied into the constructor, and the unowned
   animation returns its stored location.
3. **Could tile rendering prove +15 is native everywhere?** No. Tile content explicitly
   truncates to origin and subtracts 30 X; it is a separate reference-point contract.
4. **Could changing the simulation target fix both reports?** No. The miner trace
   removes the requested cell and the projectile/animation records share an identical
   world coordinate. The divergence appears only during projection.
5. **Could the shared absolute helper be changed safely?** Not from this evidence.
   Particle coordinates can be negative and use absolute lepton Z; their exact native
   elevation/render path needs a separate investigation.

## 11. Open Questions - Final State

- `[RESOLVED] OQ-01 - ObjectClass::DrawIt is the active tactical owner.`
- `[RESOLVED] OQ-02 - It calls full CoordsToClient2, not a cell-origin helper.`
- `[RESOLVED] OQ-03 - Flat center projects to (30*(cx-cy), 15*(cx+cy)+15).`
- `[RESOLVED] OQ-04 - Native uses sign-bias 255 then arithmetic >>8.`
- `[RESOLVED] OQ-05 - AnimClass::DrawIt adds no hidden half-cell shift.`
- `[RESOLVED] OQ-06 - YDrawOffset changes sprite screen Y after projection.`
- `[RESOLVED] OQ-07 - Instance ZAdjust changes depth/order, not screen Y.`
- `[RESOLVED] OQ-08 - Cell_ContentRendering is the active tile-origin owner.`
- `[RESOLVED] OQ-09 - Tile rendering projects cell origin, then subtracts 30 X.`
- `[RESOLVED] OQ-10 - Core 256/128 and 60/30 constants are hardcoded.`
- `[RESOLVED] OQ-11 - Inspected owner chains are active in YR.`
- `[RESOLVED] OQ-12 - Rust center, tile, and mixed paths are identified in section 8.`
- `[RESOLVED] OQ-13 - Only WorldEffect and particles consume the absolute terrain helper; the change is presentation-only.`
- `[RESOLVED] OQ-14 - Native truncates negatives toward zero; Rust absolute helper uses Euclidean decomposition.`
- `[RESOLVED] OQ-15 - Native elevation uses AdjustForZ; flat z=0 proves elevation is immaterial to this defect.`
- `[RESOLVED] OQ-16 - Center-origin delta is invariant `(0,+15)` for ordinary coordinates.`
- `[RESOLVED] OQ-17 - The normal object rendering owner provides no alternate pause/replay/save transform.`
- `[RESOLVED] OQ-18 - Low-level shape drawing does not create a WorldEffect-specific half-cell translation.`

Zero-add pass: no remaining open question is required to repair the proven flat-ground
`WorldEffect` disparity. Negative/elevated particle projection is explicitly residual.

## 12. Visual Composition Ledger

| Layer | Native anchor | Rust before repair | Required result |
|---|---|---|---|
| Projectile endpoint | Exact impact `CoordStruct` center/subcell | Existing util projector | Keep unchanged |
| Impact `AnimClass` / `WorldEffect` | Same exact `CoordStruct` | Extra +15 Y from mixed absolute helper | Use same projector as endpoint |
| Resource overlay | Terrain-cell content/art offset contract | Existing terrain/atlas path | Keep unchanged in this slice |
| Miner mutation | Arrived simulation cell | Correct in runtime trace | Keep unchanged |
| Art `YDrawOffset` / atlas offset | Applied after world projection | Atlas sprite offset | Keep separate from coordinate transform |
| Sort depth / `ZAdjust` | Separate depth parameter | Separate renderer depth | Do not convert to screen Y |

## 13. Implementation Handoff

Implement the narrow presentation correction in
`src/app_instances/overlays.rs::build_world_effect_instances`:

1. Project `fx.rx`, `fx.ry`, `fx.sub_x`, `fx.sub_y`, and `fx.z` through
   `crate::util::lepton::lepton_to_screen`, exactly like projectile endpoints and other
   cell/subcell entities.
2. Do not change the stored `WorldEffect` coordinate, combat/miner simulation, terrain
   `iso_to_screen`, or particle projector in this slice.
3. Add a regression covering multiple non-symmetric cells and center/subcell points,
   asserting that a `WorldEffect` anchor equals the projectile/native center projection
   rather than landing 15 pixels below it.
4. Retain the four-direction miner trace as evidence that extraction mutates the target
   cell and does not require an offset.

Expected player-visible result: ordinary impact/explosion/warp `WorldEffect` sprites
land on the projectile or action coordinate rather than the next isometric row.

## Sources

- Live read-only Ghidra decompile/disassembly and memory reads for the addresses listed
  in this report, including a cold re-decompile of `0x005F4B10`, `0x006D1EB0`,
  `0x006D2140`, and `0x006D6D10`.
- `docs/research/COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`
- `docs/research/COORDINATE_SYSTEM_GAMEMD.md`
- `docs/research/ANIMCLASS_DRAWIT_ZADJUST_DEPTH_GHIDRA_REPORT.md`
- `docs/research/CELL_REFERENCE_POINTS_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`
- `docs/research/INVISIBLELOW_DETONATION_COORDSTRUCT_GHIDRA_REPORT.md`
- Current Rust source and the scoped `coordinate_runtime_trace` tests.
