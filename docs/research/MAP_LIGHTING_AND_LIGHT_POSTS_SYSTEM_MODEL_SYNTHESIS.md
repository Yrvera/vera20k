# Map Lighting And Light Posts — System Model Synthesis

Date: 2026-05-22
Scope: map `[Lighting]` ambience, building/lamp point lights, `LightSourceClass`, `BuildingLightClass` spotlights, current Rust lighting surface.
Non-scope: Lightning Storm superweapon lifecycle/damage, EBOLT/Tesla weapon visuals, sound ambience.
Output type: conflict-map with implementation-safe islands.

## Current Model

YR has two separate "lighting" mechanisms that are easy to conflate:

1. **Map ambience** comes from the scenario/map `[Lighting]` section. `ScenarioClass::Read_INI_Basic` at `0x00689E90` reads `Ambient`, `Red`, `Green`, `Blue`, `Ground`, and `Level` into ScenarioClass lighting fields, plus Ion/Nuke/Dominator variants for superweapon transitions.
2. **Radius light emitters** are `LightSourceClass` objects, usually created by lamp-post-style buildings whose `BuildingTypeClass` has nonzero `LightIntensity`. Their type data is `LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, and `LightBlueTint`.
3. **Spotlights** are not the same system. `BuildingClass+0x600` is a `BuildingLightClass*`, allocated from `HasSpotlight=` (`Type+0x154B`). `BuildingClass+0x614` is a separate `LightSourceClass*` for ambient/radius light.
4. Cell drawing uses cached per-cell light conversion data. `FUN_00483E30` initializes/sets a cell's LightConvert pointer and light fields; `FUN_00484180` computes the full set including map ambience and active light-source contributions.

The strongest checked binary evidence supports a real point-light model: live `LightSourceClass` entries are scanned for each cell, distance is measured from cell center to light source in leptons, contribution falls off linearly from the light radius, and values are clamped to the 0..2000 lighting range.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| `[Lighting]` `Ambient/Red/Green/Blue/Ground/Level` is scenario-level ambience data | Ghidra `0x00689E90`; `CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Lamp posts use `LightVisibility/LightIntensity/*Tint` from building type data | Ghidra `0x00440580`, `0x00554760`; `rulesmd.ini` lamp sections | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Runtime ambient light pointer is `BuildingClass+0x614`, not `+0x600` | `BUILDINGCLASS_FIELD_VERIFICATION_ROUND_2.md`; Ghidra `0x00440580` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `+0x600` is `BuildingLightClass` spotlight allocated by `HasSpotlight=` | same plus `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md` | confirmed | high | conditional | IMPLEMENTATION_SAFE |
| LightSource allocation in Unlimbo is gated by `Type+0xE34 != 0` (`LightIntensity`), not by `LightVisibility` alone | Ghidra `0x00440580` | confirmed | high | yes | DOC_PATCH_READY |
| LightSource falloff is linear by lepton radius | Ghidra `0x00484180` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| LightSource enable/disable dirties or recomputes affected cells | Ghidra `0x00554A60`, `0x00554A80`, `0x00554AF0`; LogicClass doc | confirmed | medium | yes | IMPLEMENTATION_SAFE for concept, needs details for scheduling |
| Exact LightConvert palette/cache reference-count behavior is fully modeled in Rust | Ghidra `0x00483E30`; Rust inspection | contradicted/unknown | medium | yes | NEEDS_REINVESTIGATE |
| Rust point-light math exactly matches original engine | Rust comment; Ghidra partial check | partially confirmed | medium | yes | DOC_PATCH_READY only unless exact scaling tests are added |
| `ExtraLight=` is a flat own-cell brightness adjustment at scale 1000 | Rust code/comment; ctor default doc says type field exists | unknown | low | probably | NEEDS_REINVESTIGATE |

## Implementation-Safe Facts

- Map ambience should be parsed from `[Lighting]`: `Ambient`, `Red`, `Green`, `Blue`, `Ground`, and `Level`. Ghidra `0x00689E90` confirms these are read by `ScenarioClass::Read_INI_Basic`.
- The point-light building keys are real YR data. `rulesmd.ini` comments define `LightVisibility` as lepton distance, and lamp sections such as `TSTLAMP`, `NEGLAMP`, `REDLAMP`, `GRENLAMP`, `BLUELAMP`, `YELWLAMP`, and `PURPLAMP` carry those keys.
- `LightVisibility` is in leptons. `FUN_00484180` compares squared lepton distance to `radius * radius`, using cell centers `(cell * 256 + 128)`.
- The falloff formula is effectively `(radius - distance) / radius`, scaled by 1000, applied to intensity and RGB tint fields.
- Per-cell brightness and RGB values are clamped. `FUN_00484180` and `FUN_005558E0` clamp below 0 and above 2000.
- Light sources participate only when their active flag at `LightSource+0x48` is set and their detail/quality field at `+0x34` is allowed by `g_ExtraAnimationsEnabled`.
- `LightSourceClass` constructor `0x00554760` stores the source coordinate, visibility radius, intensity, RGB tint values, active/detail fields, and inserts the object into global light-source vector `DAT_00ABCA14`/`DAT_00ABCA20`.
- Building placement creates `LightSourceClass*` at `BuildingClass+0x614`; spotlights use `BuildingLightClass*` at `+0x600`.

## Doc-Patch-Ready Facts

- Older docs saying `+0x614` exists on all buildings or is a "default ambient light" should be corrected. BuildingType defaults include `LightVisibility=5000`, but `LightIntensity=0`, so ordinary buildings do not get an active radius light from visibility alone.
- Older wording saying Unlimbo allocates `LightSourceClass` when any `Type+0xE30..0xE40` light field is nonzero is too broad. The checked Unlimbo branch gates on `Type+0xE34 != 0`, i.e. nonzero `LightIntensity`.
- `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md` line 451 appears to call `Type+0xE34` "has `LightVisibility=`"; the offset is `LightIntensity`, while `LightVisibility` is `+0xE30`.
- Rust comments in `src/map/lighting.rs` and `src/app_init.rs` can safely say the high-level model is radius light with linear falloff, but should not imply the full LightConvert/cache behavior is fully reproduced.
- Rust `ObjectType` currently defaults `LightVisibility` to 0. Binary constructor defaults it to 5000 while `LightIntensity` defaults to 0. That likely does not change stock ordinary-building lighting, but it is a data-model mismatch for mods/inheritance tests.

## Stale Or Superseded Claims

- `BUILDINGCLASS_MASTER_GHIDRA_REPORT.md` says `+0x614` is ambient light "on ALL buildings". Superseded by later reports and live Unlimbo spot-check.
- `BUILDINGCLASS_VERIFICATION_ROUND_GHIDRA_REPORT.md` says `LightSourceClass` is created for `LightVisibility > 0` and describes it as default ambient light. Superseded by Ghidra `0x00440580`: branch checks `Type+0xE34 != 0`.
- `BUILDINGCLASS_FIELD_VERIFICATION_ROUND_2.md` is correct about `+0x600` vs `+0x614`, but its allocation summary "any of `Type+0xE30..0xE40` non-zero" is too broad for the checked Unlimbo site.

## Cross-Doc Conflicts

- Several building master/verification docs use "ambient light" for `LightSourceClass`, while map `[Lighting]` also uses ambient-light terminology. The corrected vocabulary should be:
  - map ambience = ScenarioClass `[Lighting]` fields;
  - radius/point light = `LightSourceClass` from building type light keys;
  - spotlight/searchlight = `BuildingLightClass` from `HasSpotlight=`.
- Some docs treat `LightVisibility` as the allocation gate because its default is conspicuous (`5000`). Binary allocation uses `LightIntensity` at the checked Unlimbo site.
- `ExtraLight=` appears in both Rust and `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`, but no checked binary consumer in this synthesis proves Rust's flat-own-cell behavior.

## Needs Re-Investigation

- `/re-investigate LightSourceClass cell recompute and LightConvert cache path`  
  Drain `0x00554AF0`, `0x00554D50`, `0x00483E30`, `0x00484180`, `0x00544E70`, and `0x00555AC0` enough to implement cache/refcount scheduling and exact palette conversion.
- `/re-investigate BuildingTypeClass ExtraLight rendering effect`  
  Verify who reads `BuildingTypeClass+0x1548`, whether it affects the building's own cell, sprites, palette choice, or only some special rendering path.
- `/re-investigate BuildingLightClass spotlight behavior`  
  Needed only if implementing searchlights/spotlights such as Hollywood spotlight behavior; it is separate from lamp-post ambience.
- `/verify-doc BUILDINGCLASS_MASTER_GHIDRA_REPORT.md` and `/verify-doc BUILDINGCLASS_VERIFICATION_ROUND_GHIDRA_REPORT.md`  
  The stale "all buildings/default ambient/LightVisibility gate" claims are narrow and patchable.

## Do-Not-Implement Notes

- Do not implement map lamp posts as a Lightning Storm or weather feature. They are building type light emitters plus the cell lighting/LightConvert pipeline.
- Do not allocate lights for every building just because binary `LightVisibility` defaults to 5000. With `LightIntensity=0`, the checked live branch does not allocate.
- Do not merge `BuildingLightClass` (`+0x600`) and `LightSourceClass` (`+0x614`). They have different gates, lifetimes, and player-visible roles.
- Do not treat Rust's current `f32` grid as exact parity for final rendering until fixed-point scaling, LightConvert cache behavior, and palette application are verified end-to-end.

## Source Ledger

- Ghidra live spot-checks: `0x00440580` (`BuildingClass::Unlimbo`), `0x00554760` (`LightSourceClass` constructor), `0x00689E90` (`ScenarioClass::Read_INI_Basic`), `0x00484180` (cell light computation with light sources), `0x00483E30` (cell LightConvert initializer/setter), `0x00554A60`/`0x00554A80`/`0x00554AF0` (LightSource enable/disable dirtying), `0x005558E0` (RGB/brightness clamp/normalize helper).
- Research docs: `BUILDINGCLASS_FIELD_VERIFICATION_ROUND_2.md`, `BUILDINGCLASS_VERIFICATION_ROUND_GHIDRA_REPORT.md`, `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`, `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md`, `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md`, `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md`, `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md`, `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md`, `CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md`, `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`, `RULESCLASS_FIELDS.csv`, `RULESCLASS_GHIDRA_REPORT.md`.
- INI data: `ini/rulesmd.ini` `[BuildingTypes]`, lamp sections around `TSTLAMP`/`NEGLAMP`/colored lamps, `AmbientChangeRate`, `AmbientChangeStep`; `ini/artmd.ini` `ExtraLight=` examples.
- Rust surfaces: `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs`, `src/rules/art_data.rs`.

## Classification

Doc-patch-ready with implementation-safe islands. The high-level map ambience plus lamp `LightSourceClass` model is safe to use, including the allocation gate and linear radius falloff. Exact parity for final rendering/cache/palette integration and `ExtraLight=` remains investigation-blocked.
