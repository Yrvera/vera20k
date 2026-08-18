# EXTRALIGHT_DRAWBODY_Z_DELTA_DETAILS_GHIDRA_REPORT

Date: 2026-05-22

Target: `EXTRALIGHT_DRAWBODY_Z_DELTA_DETAILS`

Status: COMPLETE

## Target Question

How does the signed `ExtraLight=` value stored at `BuildingTypeClass+0x1548` flow through `BuildingClass_DrawBody`, which draw/depth argument does it affect, what is its sign behavior, does it interact with cell lighting fields `CellClass+0x10A/+0x10C`, and what is the concrete Rust render handoff?

## Non-goals

- Do not re-prove the `ExtraLight=` reader except where needed for signedness context.
- Do not investigate `LightSourceClass`, `LightConvertClass`, map `[Lighting]`, or lamp ambience.
- Do not investigate `BuildingLightClass` spotlights.
- Do not edit Ghidra, Rust, INI files, existing reports, or `.swarm-claims.md`.

## Evidence Needed To Mark COMPLETE

- Decompile evidence from `BuildingClass_DrawBody @ 0x0043D290`.
- Assembly/address evidence for each handoff-critical signed load/add/push site.
- Identification of whether the summed value uses `CellClass+0x10A` or `CellClass+0x10C`.
- Identification of the downstream draw call(s) reached by the summed value.
- Rust-facing handoff with concrete affected files and test proposal.

## Stop Conditions

- Stop after classifying `BuildingClass_DrawBody` consumers of `BuildingTypeClass+0x1548`.
- Stop if the target expands into general building rendering, lighting, or fogged-object rendering.
- Stop if only decompiler prose is available for signedness or argument flow.

## Verified Findings

### 1. Main SHP body depth is `signed(Cell+0x10A) + signed(Type+0x1548)`

Active in YR: Yes. `BuildingClass_DrawBody @ 0x0043D290` is the standard visible building body draw path, reached through the building draw dispatcher.

Evidence:

- Decompile at `0x0043D290` computes `iVar16 = (int)*(short *)(cell + 0x10a) + (int)*(short *)(param_1->Type + 0x1548)` in the main body branch.
- Assembly confirms signed extension and raw add:
  - `0x0043D81D`: `MOVSX ECX, word ptr [EAX + 0x10A]`
  - `0x0043D824`: `MOVSX EAX, word ptr [EDX + 0x1548]`
  - `0x0043D82D`: `ADD ECX, EAX`
  - `0x0043D82F..0x0043D836`: pushes summed `ECX`, `1`, `2`, then calls vtable slot `+0x1D0`

Claim: `ExtraLight` is used as a signed 16-bit delta added to the cell Z/depth base at `CellClass+0x10A`. Positive values increase the depth argument; negative values decrease it. There is no divide, multiply, clamp, or RGB conversion in this DrawBody slice.

### 2. The same summed value feeds both the pre-draw Z adjustment path and `TechnoClass_DrawSHP`

Active in YR: Yes for the main SHP body branch.

Evidence:

- After `0x0043D82F..0x0043D836` calls vtable slot `+0x1D0` with the summed depth, DrawBody calls `Tactical__AdjustForZ @ 0x006D20E0` at `0x0043D83E`.
- The adjusted result is used as a Y offset at `0x0043D843..0x0043D84D` (`screen_y - AdjustForZ`), then DrawBody pushes the remaining draw arguments and calls `TechnoClass_DrawSHP @ 0x00705E00` at `0x0043D85F`.
- Decompile shows the same `iVar16` value passed to `TechnoClass_DrawSHP(...)` as the body depth/Z argument after layer `2` and sort flag `1`.

Claim: The field affects render ordering/depth and the derived screen-Y Z adjustment path. The immediate vtable `+0x1D0` call receives layer `2`, sort flag `1`, and the summed depth. The following `TechnoClass_DrawSHP` call receives the same body-depth value in the building body draw parameter bundle.

### 3. Damaged/body-overlay branches repeat the same signed addition

Active in YR: Conditional. These branches require optional building art pointers and state gates, such as `Type+0x1518` plus `BuildingClass+0x534 != 0`, or factory/overlay art at `Type+0x14EC` / `Type+0x1504`.

Evidence:

- Damaged auxiliary body branch:
  - `0x0043D89D`: `MOVSX ECX, word ptr [EAX + 0x10A]`
  - `0x0043D8A4`: `MOVSX EAX, word ptr [EDX + 0x1548]`
  - `0x0043D8AD`: `ADD ECX, EAX`
  - `0x0043D8AF..0x0043D8B6`: pushes the sum, sort flag `1`, layer `0`, then calls vtable `+0x1D0`
- Normal factory/overlay art branch:
  - `0x0043D97D`: `MOVSX EDX, word ptr [EAX + 0x10A]`
  - `0x0043D98A`: `MOVSX ECX, word ptr [EAX + 0x1548]`
  - `0x0043D991`: `ADD EDX, ECX`
  - `0x0043D995..0x0043D99C`: pushes the sum, sort flag `1`, layer `0`, then calls vtable `+0x1D0`
- Alternate factory/overlay art branch:
  - `0x0043DA21`: `MOVSX ECX, word ptr [EAX + 0x10A]`
  - `0x0043DA28`: `MOVSX EAX, word ptr [EDX + 0x1548]`
  - `0x0043DA31`: `ADD ECX, EAX`
  - `0x0043DA33..0x0043DA3A`: pushes the sum, sort flag `1`, layer `0`, then calls vtable `+0x1D0`

Claim: DrawBody consistently treats `Type+0x1548` as a signed depth delta anywhere it uses the field. The main body uses layer `2`; auxiliary/overlay body art uses layer `0` in the observed branches.

### 4. `CellClass+0x10C` is not part of the ExtraLight calculation in DrawBody

Active in YR: Yes as a negative fact for this DrawBody slice.

Evidence:

- Every `Type+0x1548` DrawBody consumer listed above combines it with `MOVSX ..., word ptr [cell + 0x10A]`, not `cell + 0x10C`.
- `TechnoClass_DrawSHP @ 0x00705E00` does read `CellClass+0x10C` in separate lighting/color paths, including cases where it refreshes `CellClass+0x34` via `FUN_00483E30` and then loads `*(short *)(cell + 0x10C)`.
- No observed `0x0043D290` instruction routes `Type+0x1548` into `CellClass+0x10C`, `CellClass+0x34`, `LightConvertClass`, or RGB arguments.

Claim: `Cell+0x10A` is the depth/Z base used with `ExtraLight`. `Cell+0x10C` remains a lighting/color scalar consumed elsewhere and should not be blended with `ExtraLight`.

### 5. Construction/gate animation branch does not apply ExtraLight

Active in YR: Conditional. This branch is live when a building is in the relevant gate/construction mission/state, but not for normal idle body draw.

Evidence:

- In the special branch before the main body path, decompile computes `iVar15 = (int)*(short *)(cell + 0x10A)` and passes that to vtable `+0x1D0` / `TechnoClass_DrawSHP`.
- No `Type+0x1548` load appears in that branch before it returns.

Claim: Do not blindly apply `ExtraLight` to every building-related SHP draw. The verified standard main body and auxiliary body-art branches apply it; the earlier special gate/construction-style branch does not.

## Implementation Handoff

1. Verified behavior: `ExtraLight=` is a raw signed 16-bit draw-depth delta added to signed `CellClass+0x10A`.
   Rust delta: keep parsing the INI key, but reinterpret the stored field away from ambience and apply it to building body sprite depth/order generation.
   Affected surface: `src/rules/art_data.rs`, building sprite/depth emission surface, and any building-body selection/bracket depth coupling.
   Acceptance scenario: `GAICBM` or `GADPSA` with `ExtraLight=-100` produces the same RGB lighting as `ExtraLight=0`, but its body depth key is lower by 100 relative to the cell depth base.
   Proposed test name: `test_building_extra_light_offsets_body_depth_not_rgb`
   Risk: High screenshot/sort risk for the few stock buildings that use `ExtraLight`.

2. Verified behavior: main body branch uses layer `2`, sort flag `1`, and `cell_z + extra_light`; auxiliary body-art branches use the same signed sum with layer `0`.
   Rust delta: model the depth delta at the building-body draw-item layer, not in map lighting setup.
   Affected surface: likely `src/render/batch.rs` `SpriteInstance.depth`, terrain/entity instance construction, and any future building-body renderer.
   Acceptance scenario: a building with `ExtraLight=350` changes only its render order/depth key relative to adjacent terrain/objects; cell tint and point-light accumulation are unchanged.
   Proposed test name: `test_extra_light_positive_value_raises_building_depth_key`
   Risk: Medium until Rust has a single building body depth-key helper.

3. Verified behavior: `CellClass+0x10C` is separate from this calculation.
   Rust delta: do not use LightConvert/light-profile outputs as the base for `ExtraLight`; use the terrain/cell depth or render Z-adjust equivalent.
   Affected surface: render/depth helper code, not `src/map/lighting.rs`.
   Acceptance scenario: changing `Cell+0x10C` equivalent lighting profile does not change the ExtraLight depth delta, and changing ExtraLight does not alter a cell light profile.
   Proposed test name: `test_extra_light_uses_cell_depth_not_light_profile`
   Risk: Medium if LightConvert and depth profiles are introduced together.

## Negative Facts / Do Not Do

- Do not apply `ExtraLight` to RGB lighting, `LightingGrid`, `LightSourceClass`, or `LightConvertClass`. Active in YR: Yes as a negative fact; DrawBody uses `Cell+0x10A + Type+0x1548`, while `TechnoClass_DrawSHP` reads `Cell+0x10C` only through separate lighting/color paths.
- Do not divide `ExtraLight` by `1000`. Active in YR: Yes; DrawBody uses `MOVSX` and integer `ADD` directly at `0x0043D824/0x0043D82D`, `0x0043D8A4/0x0043D8AD`, `0x0043D98A/0x0043D991`, and `0x0043DA28/0x0043DA31`.
- Do not treat the field as unsigned. Active in YR: Yes; every DrawBody use is `MOVSX word ptr`, so negative stock values like `-100` remain negative.
- Do not apply `ExtraLight` automatically to the special gate/construction-style branch. Active in YR: Conditional; that branch uses `Cell+0x10A` without a `Type+0x1548` load before returning.
- Do not couple `ExtraLight` to `CellClass+0x10C`. Active in YR: Yes as a negative fact; no `Type+0x1548` consumer in DrawBody reads or writes `+0x10C`.

## Remaining Uncertainty

- The exact Rust insertion point depends on the still-evolving building body renderer. This report verifies the binary behavior and handoff shape, not the final Rust call site.
- The symbolic name/signature of vtable slot `+0x1D0` is not established here. The argument flow is verified at the DrawBody call sites, but this slot should be named only after a separate bounded investigation if needed.
- The fogged/cached-object consumer reported elsewhere remains outside this target. This report only completes `BuildingClass_DrawBody`.

