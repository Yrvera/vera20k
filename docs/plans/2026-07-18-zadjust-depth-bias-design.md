# ZAdjust Depth-Bias Design (building anims, turret VXL, world effects, parachute)

2026-07-18. Brainstorm output; approach A approved by user.

## Goal

Apply art.ini/rules.ini `ZAdjust` values as pixel-exact depth-sort biases in every
sprite-emit path that has the data but ignores it, and fix the one existing consumer
whose bias direction is inverted relative to the binary.

## Architecture Context

- `SpriteInstance.depth` is the painter's sort key: sorted **descending**, drawn
  back-to-front; **lower depth = closer to camera**
  ([merge_passes.rs:223](../../src/app_render/merge_passes.rs), particles.rs:106 —
  "passthrough pipeline does no GPU depth read/write, this field only feeds
  sort_by_depth_desc").
- Base depth: `compute_sprite_depth_params` = `1 − (iso_row − origin_y)/world_height −
  z·0.0001`, where `iso_row = screen_y + z·HEIGHT_STEP`
  ([helpers.rs:43](../../src/app_instances/helpers.rs)).
- Existing z_adjust consumers: garrison muzzle flash + damage fires via
  `garrison_flash_depth_apply_z_adjust` (overlays.rs:564) — **sign-inverted and
  1000-neutral, both wrong for anim ZAdjust** (see ledger).
- Parsed but unused: `BuildingAnimConfig.z_adjust` (art_data.rs:287, from
  `<Slot>ZAdjust=` in the building's art section), `AnimMetadataEntry.z_adjust`
  (art_data.rs:216, from the anim type's own `ZAdjust=`),
  `rules_obj.turret_anim_z_adjust` (threaded to `emit_building_turret_vxl` as
  `_z_adjust`, ignored).
- Sim already models AnimClass z_adjust including the damage-fire computed value
  (anim_class.rs:465, `(scaled>>1) − 10, min 0`; test asserts −192).

## Impact Analysis

Render-only; no sim state, tick order, or state-hash changes (world-effect ZAdjust is
looked up render-side, NOT stored in sim). Touched files:
`src/app_instances/helpers.rs` (new shared helper),
`src/app_instances/shp.rs` (emit_building_anims, emit_building_turret_vxl, stale
comment at :377), `src/app_instances/overlays.rs` (world effects, parachute,
garrison/damage-fire helper + its test). Blast radius: sprite ordering near
buildings/effects — precisely the thing we're changing. Risk: the garrison-helper sign
fix changes two already-shipped visuals (garrison flash, damage fires); both must be
eyeballed after the change.

## Chosen Approach (A — pixel-exact row bias)

gamemd's anim depth expression is in the same pixel units as the Y-based sort value:
`depth = YDrawOffset + ZAdjust − AdjustForZ(height) − 2` with **smaller = closer**
(AdjustForZ is subtracted so higher objects pull toward the camera). The Rust depth
axis has the same direction (lower = closer), so the exact translation of a ZAdjust
bias of N pixels is:

```
depth' = clamp(depth + z_adjust_px / world_height, 0.001, 0.999)
```

(z' = z + ZAdjust ⇔ effective iso_row' = iso_row − ZAdjust ⇔ depth += ZAdjust/h.)
Negative ZAdjust → smaller depth → closer, matching every native consumer
(damage fire −192, occupied flash −200, arrows −300, DEMTEXP −1000, PARACH −10).

One shared helper `apply_shape_z_adjust(depth, z_adjust_px, world_height)` in
helpers.rs, used by all five sites. The existing 1e-6/1000-neutral helper and its unit
test are replaced by it.

## Tiny-Detail Ledger (constraint set)

1. ZAdjust modifies **depth-sort only**, never screen-Y. [doc:
   PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md §5, decompile_function 0x00422CA0]
2. Sign: **negative = toward camera**. [ini: art.ini PARACH SJM comment "infantry are
   fudged by 10 towards camera so we must match this here"; AdjustForZ subtraction in
   0x00422CA0]
3. Units: ZAdjust shares pixel units with YDrawOffset/AdjustForZ in the depth
   expression. [doc: PARACHUTE §5 formula]
4. **1000 is NOT neutral** for AnimClass ZAdjust; that convention belongs to the
   per-cell terrain z path (Cell_ComputeZAdjust, default 1000, clamp [0,2000]) which
   is a separate mechanism and out of scope here. [doc:
   ANIMCLASS_DRAWIT_ZADJUST_DEPTH_GHIDRA_REPORT.md §4 Negative Facts;
   CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md]
5. Effective anim ZAdjust precedence: constructor param if nonzero, else the
   AnimType's own `ZAdjust=` (+0x348). For building anims the slot key
   (`ActiveAnimZAdjust=` etc., 116 keys in artmd.ini) is the constructor param → slot
   value if nonzero, else anim type value. [doc: ANIMCLASS_DRAWIT §2.3,
   0x0042219E..0x004221BF]
6. Standard anim draws carry an extra **−2** constant in the depth expression; applied
   to all AnimClass-derived SHP draws (building anims, world effects, damage fires,
   muzzle flashes, parachute), NOT to the building body or turret VXL. [doc:
   PARACHUTE §5, 0x00422CA0]
7. Damage-fire ZAdjust is the computed non-positive value already produced by
   sim/anim_class.rs (mirrors the binary post-construction overwrite); render must
   consume it with the corrected sign. [doc: ANIMCLASS_DRAWIT §1]
8. Occupied-building muzzle flash: ZAdjust −200. [doc:
   ANIMCLASS_CONSTRUCTOR_CALLER_TAXONOMY_RUNTIME_SPAWNS_GHIDRA_REPORT.md]
9. PARACH: YDrawOffset=0, ZAdjust=−10 — must come from art.ini via the registry, not
   hardcoded. [doc: PARACHUTE §5; ini: art.ini]
10. TurretAnimZAdjust: 10 buildings, −20..−150 [ini: rulesmd.ini; string 0x00819630
    read in BuildingTypeClass ReadINI at 0x0046460B via get_xrefs_to]. Draw-family
    evidence (spot-checked 2026-07-18): BuildingClass_DrawBody (decompile_function
    0x0043D290) composes its z argument as `CellClass+0x10A (ZAdjust_Ground,
    1000-base) + BuildingTypeClass+0x1548` and subtracts `Tactical__AdjustForZ()` —
    signed pixel biases on the cell base, smaller z = closer. The turret draw is
    assumed to use TurretAnimZAdjust identically (same family); **exact consumer
    still UNVERIFIED** — the convenient labels are polluted (the function labeled
    `BuildingClass__Draw` at 0x004E0240 is actually a 2-line array accessor —
    RTTI_LABEL_DRIFT, recorded). Follow-up RE item; bias ships flagged.
11. Infantry are fudged 10px toward camera in gamemd [ini: art.ini SJM comment —
    **UNKNOWN — needs RE** for the binary constant/site]. Gate for the parachute
    site: applying PARACH −10 without the infantry −10 would make chute-vs-own-GI
    relative bias −10 where gamemd has 0. **Resolution this pass: parachute site
    DEFERRED** — the existing epsilon reproduces gamemd's relative chute-vs-GI
    result (0); a half-applied −10 would be worse. Lands together with the infantry
    fudge after RE.
14. (added post-spot-check) Building BODY draw carries its own per-type z bias:
    `BuildingTypeClass+0x1548`, INI key unidentified [decompile_function 0x0043D290
    — UNKNOWN — needs RE]. Body draw is untouched this pass; if +0x1548 is nonzero
    for stock buildings this is a separate parity item.
15. (added post-spot-check) Sign/units/base CONFIRMED from the binary this session:
    z argument = cell 1000-base + signed pixel bias, AdjustForZ subtracted, smaller
    = closer [decompile_function 0x0043D290]. The Rust base depth already encodes
    the cell/row term, so the Rust bias is exactly `z_adjust_px / world_height` with
    no 1000 term — confirming the helper design and the inversion in the current
    garrison helper.
12. Trigger frequency (for review priority): building-slot keys fire for most powered
    buildings every match (21× `ActiveAnimZAdjust=-100` alone); turret bias on every
    defense tower; arrows/explosions/flash per event.
13. Stale claim to delete: shp.rs:377 comment "ZAdjust affects screen Y position
    (verified: AnimClass::DrawIt reads it 4 times…)" — refuted (depth-only; 3 draw
    variants). [doc: PARACHUTE §5; COORDINATE_SYSTEM_GAMEMD.md corrected 2026-07-18]

## Design

### Components
- `helpers.rs`: `pub(crate) fn apply_shape_z_adjust(depth: f32, z_adjust_px: i32,
  world_height: f32) -> f32` + `const ANIM_DRAW_DEPTH_BIAS_PX: i32 = -2;`
- `shp.rs::emit_building_anims`: effective = slot-else-type (ledger #5) +
  ANIM_DRAW_DEPTH_BIAS_PX; needs the AnimMetadataEntry lookup for the selected anim
  type and `world_height` (already derivable from existing params or threaded in).
- `shp.rs::emit_building_turret_vxl`: use `_z_adjust` (rename), no −2; body order
  preserved (turret still pushed after body).
- `overlays.rs::build_world_effect_instances`: look up
  `art_reg.resolve_metadata_entry(shp_name).z_adjust` render-side; apply + −2.
- `overlays.rs` parachute + garrison/damage-fire: switch to the shared helper
  (sign fix); parachute gated on ledger #11.
- Delete `garrison_flash_depth_apply_z_adjust` + rewrite its test.

### Testing Strategy
- Unit tests on `apply_shape_z_adjust`: negative → smaller depth; magnitude =
  px/world_height; clamps.
- Rewrite `garrison_flash_depth_applies_native_z_adjust_as_depth_bias`: −200 must
  move the flash **toward** the camera.
- Precedence test: slot ZAdjust nonzero wins over type ZAdjust; zero falls back.
- Eyeball pass (user): Prism/SAM turret sorting, powered-building active anims,
  garrison flash, damage fires, one paradrop — vs gamemd side-by-side.
- UNVERIFIED-pending-instrument: no pixel-golden gate exists yet; visual parity claim
  stays eyeball-level until the pixel oracle lands.

## Architectural Decisions
- Follows the existing shared-helper + per-site emit pattern; no new abstractions.
- Render-side art lookup for world effects avoids sim/state-hash churn (deliberate).
- Building anim/turret keep base depth = building_depth + bias (differential model);
  the body draw's own depth expression is unresearched — folding YDrawOffset into
  position only is a known micro-approximation, flagged for the future pixel oracle.

## Alternatives Considered
- **B — sign fix only, keep 1e-6 arbitrary scale:** rejected; unit-less magnitude makes
  cross-object ordering diverge from gamemd within ~1 row (DRIFT).
- **Emission-order-only:** handles same-building stacking but cannot express
  cross-object bias at all; rejected.

## Pre-implementation checklist
1. Ghidra spot-check: TurretAnimZAdjust consumer in BuildingClass turret draw (ledger #10).
2. Ghidra spot-check: infantry −10 draw fudge constant/site (ledger #11) — gates the
   parachute change.
