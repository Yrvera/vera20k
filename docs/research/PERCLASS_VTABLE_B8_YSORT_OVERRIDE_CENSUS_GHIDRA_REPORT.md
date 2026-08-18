# Per-Class VTable +0xB8 YSort Override Census - Ghidra Research Report

**Address(es):** `ObjectClass::GetYSort @ 0x005F6BD0`, `AnimClass::GetYSortWithAdjust @ 0x00422BC0`, `BuildingClass::GetYSort @ 0x00449410`, `ObjectClass::YSortComparator @ 0x005F6220`
**Investigation Mode:** coverage-map
**Claimed Scope:** bounded census of proved active ObjectClass-family vtables whose slot `+0xB8` can supply the signed YSort key used by Ground `LayerClass` ordering.
**Non-Scope:** proof that `FUN_00551A30` is Ground `LayerClass`, its one-adjacent-pass mechanics, LogicClass active-vector ordering, full draw body/blitter behavior, and runtime screenshot fixtures.
**Confidence:** High for the named vtable slot targets and the two override formulas; Medium for "no other override" because the census is based on found vtable slot data xrefs and existing object-family reports, not a full RTTI table dump.
**Active in YR:** Yes / Conditional. The virtual slot is live for standard object rendering; Ground-layer consumption is conditional on the object being submitted to `g_DisplayLayers[2]`.

## 0. Working Notes Contract

**Target question:** Which active ObjectClass-derived classes override vtable slot `+0xB8`, what signed key do they return, and what does Rust render ordering need to preserve?
**Non-goals:** Do not re-prove `FUN_00551A30` vector identity, one-pass behavior, or LogicClass non-involvement; do not inspect every draw routine or every INI key that can spawn each class.
**Evidence needed to mark COMPLETE:** vtable table evidence for representative active classes, decompile plus assembly for every override body, base key formula, YR activity label, and Rust-facing acceptance scenarios.
**Stop conditions:** Stop after class-slot census and Rust handoff are complete enough for render-order contracts; defer unknown vtables or runtime layer-membership fixtures instead of expanding into all rendering.

## 1. Overview

Ground display ordering uses a virtual signed integer key at object vtable offset `+0xB8`. The default implementation is `ObjectClass::GetYSort`, which calls `GetRenderCoords` and returns world/render `X + Y`. The only proved active overrides in this census are `AnimClass` and `BuildingClass`.

Most object-family classes keep the base key. That matters for Rust because Unit, Infantry, Aircraft while landed, Terrain, Bullet if layer-2, Overlay/Smudge, Particle, VoxelAnim, BuildingLight, Wave, and related classes should not receive class-specific ad hoc render offsets unless their vtable actually overrides `+0xB8`.

## 2. Key Functions And Offsets

| Function / field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `ObjectClass::GetYSort @ 0x005F6BD0` | default key: `GetRenderCoords().X + GetRenderCoords().Y` | decompile `0x005F6BD0`; assembly context `0x005F6BD0..0x005F6BE0` | Yes |
| `ObjectClass::YSortComparator @ 0x005F6220` | calls `b.vtable+0xB8`, then `a.vtable+0xB8`; compares only signed keys | decompile `0x005F6220`; assembly context `0x005F6227..0x005F6234` | Yes |
| `AnimClass+0x104` | per-instance YSort adjust copied from `AnimTypeClass+0x340` | decompile `0x00422BC0`; prior `ANIMCLASS_DRAW_TRAVERSAL_LAYER_ORDERING_RESWARM_20260527.md` | Yes for AnimClass |
| `BuildingClass+0x520` | `BuildingTypeClass*` used by building override | decompile `0x00449410`; assembly `0x00449418` | Yes for BuildingClass |
| `BuildingType+0x16C5` | `TurretAnimIsVoxel=` / VXL turret gate in current field docs | assembly `0x0044941E`; `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md`; `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` | Conditional by building data |
| `BuildingType+0x16B7` | `Gate=` in current gate/pathing docs | assembly `0x00449424`; `GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md`; `rulesmd.ini` stock gate rows | Conditional by building data |

## 3. Core Logic

Default `ObjectClass::GetYSort`:

```text
coords_a = this.vtable[+0xAC].GetRenderCoords()
coords_b = this.vtable[+0xAC].GetRenderCoords()
return coords_a.Y + coords_b.X
```

The function calls `GetRenderCoords` twice. In normal implementations the two calls are equivalent, but Rust should not collapse this into a cached coordinate if the future object model introduces a side-effecting override without proof.

`AnimClass::GetYSortWithAdjust @ 0x00422BC0`:

```text
return ObjectClass::GetYSort(this) + this[0x104]
```

`BuildingClass::GetYSort @ 0x00449410`:

```text
base = ObjectClass::GetYSort(this)
type = this[0x520]
delta = (type[0x16C5] != 0 ? 0x20 : 0) - (type[0x16B7] != 0 ? 0x10 : 0)
return base + delta
```

Assembly details for the building override: bytes `Type+0x16C5` and `Type+0x16B7` are read before boolean expansion with `NEG`/`SBB`, masked to `0x20` and `0x10`, then `SUB ECX,EDX; ADD EAX,ECX; RET` at `0x0044941E..0x0044943D`. Active in YR: Yes for all BuildingClass objects; the two adjustments are conditional on type data.

## 4. VTable Slot Census

The slot table below uses read-only Ghidra byte-pattern evidence:

- `D0 6B 5F 00` found slot entries that bind `+0xB8` to base `ObjectClass::GetYSort`.
- `C0 2B 42 00` found the AnimClass override slot at `0x007E340C`.
- `10 94 44 00` found the BuildingClass override slot at `0x007E3F74`.

| Class / vtable | Slot address | Slot target | Override? | Key used when consumed by Ground layer | Active in YR |
|---|---:|---:|---|---|---|
| `AircraftClass @ 0x007E22A4` | `0x007E235C` | `0x005F6BD0` | No | base `X + Y`; flying aircraft normally leave Ground layer | Conditional: grounded/landed only for Ground sort |
| `AnimClass @ 0x007E3354` | `0x007E340C` | `0x00422BC0` | Yes | base `X + Y + AnimClass+0x104` | Yes; Ground if `AnimType.Layer=ground` or attached owner returns Ground |
| `BuildingLightClass @ 0x007E3AD0` | `0x007E3B88` | `0x005F6BD0` | No | base `X + Y` | Conditional: `HasSpotlight=yes` / spotlight object |
| `BuildingClass @ 0x007E3EBC` | `0x007E3F74` | `0x00449410` | Yes | base `X + Y + 0x20 if Type+0x16C5 - 0x10 if Type+0x16B7` | Yes; flag effects conditional |
| `BulletClass @ 0x007E46E4` | `0x007E479C` | `0x005F6BD0` | No | base `X + Y` | Conditional: only Ground-layer bullet/projectile variants |
| `FootClass @ 0x007E8C94` | `0x007E8D4C` | `0x005F6BD0` | No | base `X + Y` | Base class; inherited by mobile leaf classes |
| `InfantryClass @ 0x007EB058` | `0x007EB110` | `0x005F6BD0` | No | base `X + Y` | Yes for infantry in Ground layer |
| `IsometricTileClass @ 0x007EC258` | `0x007EC310` | `0x005F6BD0` | No | base `X + Y` | Conditional; constructed from map tile/type path |
| `MissionClass @ 0x007EDCC0` | `0x007EDD78` | `0x005F6BD0` | No | base `X + Y` | Base class, not a normal concrete rendered leaf |
| `ObjectClass @ 0x007EF060` | `0x007EF118` | `0x005F6BD0` | No | base `X + Y` | Base class |
| `OverlayClass @ 0x007EF3D4` | `0x007EF48C` | `0x005F6BD0` | No | base `X + Y` | Conditional; overlay render path varies by overlay type |
| `ParticleClass @ 0x007EF954` | `0x007EFA0C` | `0x005F6BD0` | No | base `X + Y` | Conditional: when particles exist |
| `ParticleSystemClass @ 0x007EFB9C` | `0x007EFC54` | `0x005F6BD0` | No | base `X + Y` | Conditional: particle systems / one-frame spark path |
| `RadioClass @ 0x007F0508` | `0x007F05C0` | `0x005F6BD0` | No | base `X + Y` | Base class |
| `SmudgeClass @ 0x007F32FC` | `0x007F33B4` | `0x005F6BD0` | No | base `X + Y` | Yes/Conditional: map smudges, render path not fully revisited |
| `TechnoClass @ 0x007F4960` | `0x007F4A18` | `0x005F6BD0` | No | base `X + Y` | Base class |
| `TerrainClass @ 0x007F522C` | `0x007F52E4` | `0x005F6BD0` | No | base `X + Y` | Yes for terrain objects |
| `UnitClass @ 0x007F5C70` | `0x007F5D28` | `0x005F6BD0` | No | base `X + Y` | Yes for vehicles/ships in Ground layer |
| `VoxelAnimClass @ 0x007F6318` | `0x007F63D0` | `0x005F6BD0` | No | base `X + Y`; `GetLayer @ 0x0074A960` returns `3`, so not normal Ground prepass input | Conditional; active object, generally not Ground |
| `VeinholeMonsterClass @ 0x007F66A8` | `0x007F6760` | `0x005F6BD0` | No | base `X + Y` | No for standard stock YR play; TS/vein legacy unless map/mod creates it |
| `WaveClass @ 0x007F6BF4` | `0x007F6CAC` | `0x005F6BD0` | No | base `X + Y` | Conditional: stock sonic/mag-beam WaveClass paths |

## 5. Integration Points

| Point | Finding | Evidence | Active in YR |
|---|---|---|---|
| Ground layer sorted insertion | sorted insert and the prepass use the same virtual slot `+0xB8` | `LAYER_CLASS_GHIDRA_REPORT.md`; decompile `0x005F6220` | Yes |
| Anim layer selection | `AnimClass::GetLayer` returns Ground when attached or `AnimType.Layer` when typed | decompile `0x00424CB0` | Yes/Conditional |
| Aircraft layer selection | flying aircraft are not Ground-sorted; landed/height <= 0 can be Ground | `LAYER_CLASS_GHIDRA_REPORT.md` FlyLocomotion section | Conditional |
| VoxelAnim layer selection | `VoxelAnimClass::GetLayer @ 0x0074A960` returns `3` | decompile `0x0074A960` | Yes for class, no for Ground prepass normally |
| Building override flags | `TurretAnimIsVoxel` adds 32, `Gate` subtracts 16 | decompile/assembly `0x00449410..0x0044943D`; field docs | Conditional |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/app_render/build_instances.rs` | sorts rebuilt instance vectors by floating `SpriteInstance.depth` using `sort_by` | does not model persistent native Ground layer, one-pass repair, or class virtual key table |
| `src/app_render/merge_passes.rs` | merges object draw groups by depth and uses group-priority tie behavior | tie behavior differs from native single display vector unless explicitly modeled |
| `src/app_instances/shp.rs` | comments model building YSort as `X+Y` from NW-ish origin and building anim `YSortAdjust` separately | missing BuildingClass `+0x20/-0x10` type-byte delta unless implemented elsewhere |
| `src/app_instances/units.rs` | unit depth includes local rendering offsets, including docking bias | Unit/Infantry native `+0xB8` is just base render `X+Y`; any extra bias must be proven as draw-depth, not layer key |
| `src/app_instances/overlays.rs` | garrison flash/projectile depths use bespoke depth formulas | AnimClass layer key should be `base X+Y + YSortAdjust`; ZAdjust is not the layer key |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| default key body | verified | decompile `0x005F6BD0`; assembly context | none |
| comparator virtual slot | verified | decompile `0x005F6220`; assembly context | none |
| Anim override | verified | vtable slot `0x007E340C`; decompile `0x00422BC0` | none |
| Building override | verified | vtable slot `0x007E3F74`; decompile/assembly `0x00449410..0x0044943D` | stock fixture inventory for both flags |
| base-key vtable matches | verified for listed addresses | Ghidra `search_byte_patterns D0 6B 5F 00`; sibling constructor/vtable docs | full RTTI universe outside listed object family |
| layer activation per class | touched-not-exhausted | `LAYER_CLASS_GHIDRA_REPORT.md`; class-specific docs | runtime fixture for every class in Ground layer |
| Rust render ordering | touched-not-exhausted | focused `rg` over `src/app_render`, `src/app_instances`, `src/render` | exact implementation design |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-B8-001 - What is the default `+0xB8` key? -> `ObjectClass::GetYSort` returns render coordinate `X + Y`.` (evidence: `0x005F6BD0`)
- `[RESOLVED] OQ-B8-002 - Does the comparator use class/id tiebreakers? -> No; it calls only `+0xB8` and compares signed key results.` (evidence: `0x005F6220`)
- `[RESOLVED] OQ-B8-003 - Which proved active classes override the slot? -> `AnimClass` and `BuildingClass`.` (evidence: vtable slots `0x007E340C`, `0x007E3F74`; override decompiles)
- `[RESOLVED] OQ-B8-004 - What does AnimClass add? -> `AnimClass+0x104`, copied from `AnimTypeClass+0x340` YSortAdjust in prior traversal report.` (evidence: `0x00422BC0`; `ANIMCLASS_DRAW_TRAVERSAL_LAYER_ORDERING_RESWARM_20260527.md`)
- `[RESOLVED] OQ-B8-005 - What does BuildingClass add? -> `+32` for `Type+0x16C5`, `-16` for `Type+0x16B7`.` (evidence: `0x00449410..0x0044943D`)
- `[RESOLVED] OQ-B8-006 - Do UnitClass and InfantryClass override the key? -> No, both vtable slots point to `0x005F6BD0`.` (evidence: slots `0x007F5D28`, `0x007EB110`)
- `[RESOLVED] OQ-B8-007 - Does TerrainClass override the key? -> No, slot `0x007F52E4` points to `0x005F6BD0`.` (evidence: byte-pattern match; `TERRAIN_CLASS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-B8-008 - Does VoxelAnimClass normally enter Ground layer? -> No for the checked helper; `GetLayer @ 0x0074A960` returns `3`.` (evidence: decompile `0x0074A960`)
- `[DEFERRED] OQ-B8-009 - Are there object-family vtables outside this table with a non-base `+0xB8` target?` (category: `bounded-cost-too-high`; reason: requires exhaustive RTTI/vtable universe dump; next-step-if-pursued: script all ObjectClass-derived constructor vtable writes and inspect `base+0xB8`)
- `[DEFERRED] OQ-B8-010 - Which stock scenes exercise both BuildingClass +32 and -16 deltas visibly?` (category: `needs-runtime-debugger`; reason: static flag/key proof is complete, but screenshot fixture requires stock scene setup and capture; next-step-if-pursued: place one `TurretAnimIsVoxel` building, one `Gate=yes`, one normal building with equal base YSort)
- `[DEFERRED] OQ-B8-011 - Do BulletClass/OverlayClass ever contribute to Ground `FUN_00551A30` in stock scenes?` (category: `requires-different-system-context`; reason: class slot is known, but per-type layer selection belongs to projectile/overlay render investigations)

## 9. Visual Composition Ledger

| Order role | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| Ground-layer key comparator | `ObjectClass::YSortComparator @ 0x005F6220` | Ground sorted insert / repair only | none | object render coordinate | none | Yes | ordering predicate |
| Default object key | `ObjectClass::GetYSort @ 0x005F6BD0` | any class inheriting `+0xB8` | none | `GetRenderCoords` X/Y | none | Yes/Conditional | layer key |
| Anim adjusted key | `AnimClass::GetYSortWithAdjust @ 0x00422BC0` | AnimClass in Ground layer | anim asset depends on AnimType | anim render coord plus `YSortAdjust` | draw path later | Yes/Conditional | layer key |
| Building adjusted key | `BuildingClass::GetYSort @ 0x00449410` | all BuildingClass objects; type bytes conditional | building asset later | building render coord plus small type delta | draw path later | Yes | layer key |

Asset role matrix is not applicable for this report because it investigates ordering keys, not a concrete asset draw path. Asset-specific frames remain owned by class draw reports.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| UnitClass, InfantryClass, TerrainClass, BulletClass, OverlayClass, SmudgeClass, ParticleClass, ParticleSystemClass, VoxelAnimClass, BuildingLightClass, WaveClass, and base classes bind `+0xB8` to `ObjectClass::GetYSort`, not custom per-class offsets. Active in YR: Yes/Conditional by layer and object existence. | vtable slot addresses listed in section 4; `0x005F6BD0` decompile | Rust uses multiple per-surface depth formulas and biases | `src/app_instances/*`, `src/app_render/merge_passes.rs` | Future native render-order model should centralize a virtual-key table and make base classes use `X+Y` unless a verified override exists. | Unit, infantry, and terrain at equal native `X+Y` keep insertion/tie order rather than receiving class-priority bias. Proposed test: `ground_layer_base_ysort_has_no_class_priority_bias` | Do not add class/id/type tiebreakers to emulate pleasing depth; native key table does not provide them. |
| AnimClass key is `base X+Y + AnimClass+0x104`, where `+0x104` is `YSortAdjust`. Active in YR: Yes for Ground anims. | `0x00422BC0`; prior constructor copy evidence | Rust garrison flashes/projectiles use bespoke depth formulas | `src/app_instances/overlays.rs`, anim/runtime rendering surface | Put Ground `AnimClass`-equivalent sprites into the same native layer key stream using `YSortAdjust`, and keep `ZAdjust` as draw-depth/blitter metadata. | A ground anim with `YSortAdjust=2000` sorts by `base+2000` without changing sim order or treating `ZAdjust` as layer order. Proposed test: `anim_ground_layer_ysort_uses_y_sort_adjust_not_z_adjust` | Do not use `ZAdjust` or anim pool index as the layer key. |
| BuildingClass key is `base X+Y +32 if Type+0x16C5 -16 if Type+0x16B7`. Active in YR: Yes; deltas conditional on building data. | `0x00449410..0x0044943D`; vtable slot `0x007E3F74` | Rust building depth comments model only base YSort/foundation, not these deltas | `src/app_instances/shp.rs`, building rules/art data model | Add or preserve building YSort-key deltas separately from draw-body depth and foundation occupancy. | Three buildings with identical base render coord but flags normal, `Gate=yes`, and `TurretAnimIsVoxel=yes` order by `base-16`, `base`, `base+32`. Proposed test: `building_ysort_applies_gate_and_voxel_turret_deltas` | Do not conflate `Gate=` passability with an overlay gate, and do not fold these small deltas into world position. |

## 11. Negative Facts / Do Not Do

- Do not sort `UnitClass` or `InfantryClass` with a special vehicle/infantry class priority. Evidence: `UnitClass 0x007F5D28` and `InfantryClass 0x007EB110` both point to `0x005F6BD0`. Active in YR: Yes.
- Do not treat aircraft as having an aircraft-specific `+0xB8` key. Evidence: `AircraftClass 0x007E235C -> 0x005F6BD0`; flying aircraft usually avoid Ground layer by layer selection. Active in YR: Conditional.
- Do not use `ZAdjust` as the Ground layer order key for AnimClass. Evidence: `0x00422BC0` adds `AnimClass+0x104` YSortAdjust; prior draw traversal report separates ZAdjust. Active in YR: Yes for AnimClass.
- Do not omit BuildingClass small deltas once exact render parity is pursued. Evidence: `0x00449410` reads `Type+0x16C5/+0x16B7` and returns `base+delta`. Active in YR: Yes/Conditional.
- Do not claim every ObjectClass-derived vtable was exhaustively dumped here. Evidence: coverage-map mode; remaining RTTI-universe dump is deferred. Active in YR: N/A.

## 12. Remaining Uncertainty

- Exhaustive ObjectClass-derived vtable universe proof is deferred. The listed set covers the active/common classes surfaced by existing vtable reports and Ghidra byte-pattern hits, including two non-base overrides.
- Stock-scene visual fixture for BuildingClass `+32/-16` tie cases is not captured.
- BulletClass and OverlayClass layer-2 participation by stock type is not drained here; their `+0xB8` target is base if they do enter Ground.
- The exact Rust design for a persistent native display-layer vector remains a separate implementation contract/design task.

## 13. Stale Docs / Follow-up Docs

- `docs/research/ANIM_CLASS_DEEP_DIVE.md`: replace row `| 46 | 0x0B8 | 0x422BC0 | (O) AnimClass::GetRenderColor | Uses +0x184 or inherited |` with: `| 46 | 0x0B8 | 0x00422BC0 | (O) AnimClass::GetYSortWithAdjust | Returns ObjectClass::GetYSort() + AnimClass+0x104 YSortAdjust |`.
- Any implementation contract or synthesis doc that says `FUN_00551A30` sorts an active/LogicClass vector should use: "`FUN_00551A30` repairs Ground display `LayerClass` order by signed virtual `+0xB8` YSort keys. It is not LogicClass active-object maintenance."
- Any render-order doc saying "all ground objects sort by `X+Y`" should be qualified: "base objects sort by `X+Y`; AnimClass adds `YSortAdjust`; BuildingClass adds `+32` for `Type+0x16C5` and `-16` for `Type+0x16B7`."

## Sources

- Ghidra read-only decompiles: `0x005F6BD0`, `0x005F6220`, `0x00422BC0`, `0x00449410`, `0x0041BE00`, `0x00447AC0`, `0x00459EF0`, `0x00424CB0`, `0x0074A960`, `0x00543780`, `0x0074C650`, `0x005B2DB0`.
- Ghidra read-only assembly contexts: `0x005F6220`, `0x005F6BD0`, `0x00422BC0`, `0x00449410..0x0044943D`.
- Ghidra read-only byte-pattern searches: `D0 6B 5F 00`, `C0 2B 42 00`, `10 94 44 00`, constructor vtable-base patterns for `IsometricTileClass`, `VeinholeMonsterClass`, `BuildingLightClass`, and `MissionClass`.
- Prior reports: `FUN_00551A30_ACTIVE_ORDER_MAINTENANCE_KEY_RESWARM_20260528.md`, `ANIMCLASS_DRAW_TRAVERSAL_LAYER_ORDERING_RESWARM_20260527.md`, `LAYER_CLASS_GHIDRA_REPORT.md`, `OBJECTCLASS_GHIDRA_REPORT.md`, `BUILDINGCLASS_VTABLE_COMPLETE.md`, `TECHNOCLASS_VTABLE_COMPLETE.md`, `FOOTCLASS_VTABLE_COMPLETE.md`, `AIRCRAFTCLASS_GHIDRA_REPORT.md`, `TERRAIN_CLASS_GHIDRA_REPORT.md`, `VOXELANIMCLASS_GHIDRA_REPORT.md`, `DETACH_FROM_ALL_LISTS_LISTENER_ROSTER_CENSUS_RESWARM_20260528.md`, `BUILDINGLIGHT_HASSPOTLIGHT_REGISTRATION_RESWARM_20260528.md`.
- Rust scan: `src/app_render/build_instances.rs`, `src/app_render/merge_passes.rs`, `src/app_instances/shp.rs`, `src/app_instances/units.rs`, `src/app_instances/overlays.rs`, `src/app_instances/helpers.rs`.

**Status:** COMPLETE for bounded per-class `+0xB8` override census; PARTIAL only for exhaustive RTTI-universe and runtime screenshot fixture coverage.
