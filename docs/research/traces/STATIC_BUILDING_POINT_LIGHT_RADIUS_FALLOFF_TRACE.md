# Static Building Point Light Radius/Falloff Trace

Date: 2026-05-24

Scenario: one stock YR `GALITE` structure on an otherwise flat map, no other point lights. The concrete source is placed at cell `(10,10)` and sampled on the +X axis at offsets `0`, `10`, `19`, and `20` cells.

Scope: static building point-light radius and falloff only. Adjacent lifecycle, power toggles, radiation lights, terrain-object non-emitters, spotlight beams, and LightConvert palette construction are not traced here.

Verdict summary: Rust creates a point light from `LightIntensity != 0` and uses lepton-centered radius checks, so the broad source allocation and radius edge are close for this simple placed-lamp case. The visible lighting values are not parity: gamemd keeps additive intensity and RGB tint as separate milli-unit outputs, normalizes RGB through `0x005558E0`, and exposes multiple cell-light fields; Rust multiplies intensity by tint into one RGB tint and clamps that directly.

## Pipeline

`rulesmd.ini GALITE` -> BuildingType light fields -> map/spawned structure -> Rust `PointLight` / gamemd `LightSourceClass` -> affected-cell radius scan -> per-cell contribution -> cell-light output tuple / Rust `CellLightGrid` tint -> SHP/terrain tint consumers.

## Concrete Data

Stock YR `GALITE` from `ini/rulesmd.ini:17233`:

- `LightVisibility=5000`
- `LightIntensity=0.2`
- `LightRedTint=0.05`
- `LightGreenTint=0.05`
- `LightBlueTint=0.01`

Internal gamemd units from `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`:

- radius `5000` leptons
- intensity `200`
- RGB tints `50,50,10`

Rust source construction at `src/map/lighting.rs:419` to `src/map/lighting.rs:448` produces the same stored source fields for these explicit values:

- radius `5000`
- intensity `200`
- RGB tints `50,50,10`
- center `(rx*256+128, ry*256+128)`
- active/detail placeholders both true

Flat default Rust base tint at height `0` is `[0.8,0.8,0.8]` from `src/map/lighting.rs:326`.

## Stage Results

### Stage 1 - INI Source Fields

gamemd: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` reads the five building light keys from the rules section. Double values store as `ftol(value * 1000.0 + 0.1)`.

Rust: `src/rules/object_type.rs:1112` to `src/rules/object_type.rs:1116` parses the same explicit `GALITE` values through `get_light_f32`; `src/map/lighting.rs:502` stores them as `value * 1000 + 0.1`.

Concrete output: radius `5000`, intensity `200`, tint `50,50,10` in both.

Verdict: PASS for this concrete explicit `GALITE` source.

### Stage 2 - Light Source Allocation / Collection

gamemd: `BuildingClass__Unlimbo @ 0x00440580` allocates `LightSourceClass` when `BuildingTypeClass+0xE34` (`LightIntensity`) is nonzero, then enables it through `0x00554A60(0)`.

Rust: `src/map/lighting.rs:389` to `src/map/lighting.rs:415` collects map structures; `src/app_init.rs:184` to `src/app_init.rs:213` collects live sim structures. Both call `point_light_from_object`, which returns `None` only when intensity or radius is zero.

Concrete output: `GALITE LightIntensity=0.2` creates one active point light in both for a live non-dying placed structure.

Verdict: PASS for the static one-building collection gate.

### Stage 3 - Radius Cell Set

gamemd: `0x00484180` computes cell centers as `cell*256+128`, compares squared distance against `radius*radius`, then accepts `distance <= radius`. `0x00554AF0` dirty scan uses `floor(radius/256)+1` square and the same circular center-distance guard.

Rust: `src/map/lighting.rs:463` to `src/map/lighting.rs:481` computes the same center coordinates, skips only if `distance_sq > radius*radius`, then uses integer square root.

Concrete +X samples for radius `5000`:

| Offset | Distance | gamemd inside | Rust inside |
|---:|---:|---|---|
| 0 | 0 | yes | yes |
| 10 | 2560 | yes | yes |
| 19 | 4864 | yes | yes |
| 20 | 5120 | no | no |

Verdict: PASS for these axis-aligned sample cells.

### Stage 4 - Falloff Contribution

gamemd: `0x00484180` computes `factor = ((radius - distance) * 1000) / radius`, then separately adds:

- additive intensity: `intensity * factor / 1000`
- red tint: `red * factor / 1000`
- green tint: `green * factor / 1000`
- blue tint: `blue * factor / 1000`

Rust: `src/map/lighting.rs:483` to `src/map/lighting.rs:487` computes one per-channel contribution:

`falloff * intensity * tint[channel] / radius / 1000`

Concrete +X outputs:

| Offset | gamemd factor | gamemd add intensity | gamemd RGB add | Rust RGB add |
|---:|---:|---:|---:|---:|
| 0 | 1000 | 200 | `50,50,10` | `10,10,2` |
| 10 | 488 | 97 | `24,24,4` | `4,4,0` |
| 19 | 27 | 5 | `1,1,0` | `0,0,0` |
| 20 | skipped | 0 | `0,0,0` | `0,0,0` |

Verdict: FAIL. The radius/falloff distance is aligned for this sample, but the output value model is different and numerically unequal at every in-radius sample.

### Stage 5 - Cell-Light Output Shape

gamemd: `0x00484180` outputs at least a 16.16 brightness scale, additive intensity, top/common/bottom ambient values, and RGB keys. It then calls `0x005558E0`, which clamps/normalizes RGB and may scale additive intensity. For the center `GALITE` sample before RGB normalization, the ordinary flat-map intermediate tuple is:

- additive intensity `200`
- RGB before normalization `1050,1050,1010`
- top ambient before/after high clamp `1000`
- bottom ambient before RGB scale `1128`

Rust: `src/map/lighting.rs:490` to `src/map/lighting.rs:498` stores only one compatibility RGB tint for the same center:

- base `800,800,800`
- contribution `10,10,2`
- stored tint `810,810,802`

Verdict: FAIL. Rust has no equivalent output tuple for gamemd's separate additive intensity, top/bottom ambient, RGB normalization, and 16.16 scale.

### Stage 6 - Runtime Update Timing

gamemd: `BuildingClass__Unlimbo @ 0x00440580` enables the source immediately and `0x00554AF0(0)` recomputes affected cells immediately. Online/offline/destruction paths also use immediate mode in the lighting lifecycle report.

Rust: `src/app_init.rs:167` to `src/app_init.rs:181` rebuilds lighting during map/app setup. It also rebuilds after at least one app input path at `src/app_input.rs:782`, but this trace did not find a gamemd-equivalent per-building LightSource handle or dirty-cell immediate recompute service.

Verdict: NOT-IMPLEMENTED for binary-style live dirty-cell scheduling. Static load-time collection exists, but the immediate LightSource recompute pipeline is absent.

### Stage 7 - Screen Result

gamemd: The in-radius cells feed `0x00483E30` / LightConvert-backed cell fields. The exact final palette pixels were not rendered in this trace; the numeric pre-LightConvert cell tuple is verified.

Rust: Render consumers read `CellLightGrid` tint through surfaces such as `terrain_object_tint_at`, `building_body_tint_at`, and `techno_tint_at`.

Verdict: UNCHECKED for literal screenshot equality. The upstream numeric cell-light tuple already fails, so final visible parity is expected to fail.

## Failures

1. Falloff contribution model differs. Rust multiplies `intensity * tint` into per-channel RGB, while gamemd adds intensity and RGB tint as separate fields before normalization. Player-visible result: lamp color/brightness around every lit cell is too weak and shaped differently. Rust evidence: `src/map/lighting.rs:483`; gamemd evidence: `0x00484180`.

2. Output shape differs. Rust stores one RGB tint; gamemd stores a richer cell-light tuple with scale, additive intensity, top/common/bottom ambient, and normalized RGB. Player-visible result: sprites/terrain cannot match YR LightConvert-driven shading. Rust evidence: `src/map/lighting.rs:490`; gamemd evidence: `0x00484180`, `0x005558E0`.

3. Dynamic dirty scheduling is missing. Static load-time collection exists, but no LightSource-style immediate affected-cell recompute service exists for live building light changes. Player-visible result: lamp lighting will not reliably update with power/death/sell/capture timing. Rust evidence: `src/app_init.rs:167`; gamemd evidence: `0x00554AF0`.

## Not Implemented

- LightSource active/detail state equivalent with `source+0x48` and `source+0x34 <= DetailLevel`.
- Immediate/queued affected-cell recompute equivalent to `0x00554AF0` / `0x00554D50`.
- LightConvert RGB key quantization and palette construction.
- Separate top/bottom/common cell-light outputs consumed by renderer.

## Adjacent Findings

- This trace did not cover terrain objects with stray Light* keys; the terrain-object non-emitter question belongs to another trace slot.
- This trace did not cover power/offline lifecycle beyond noting that gamemd's static source is not just a one-time bake.
- This trace did not cover superweapon ambient transitions or transient screen-space combat/particle lights.

## Verdict Tally

PASS: 3 | FAIL: 2 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Sources

- `docs/research/BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md`
- `docs/research/LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`
- `docs/research/MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`
- `docs/research/LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md`
- Read-only Ghidra spot checks: `0x00484180`, `0x005558E0`, `0x00554AF0`, `0x00440580`.
- `ini/rulesmd.ini:17233`
- `src/map/lighting.rs`
- `src/rules/object_type.rs`
- `src/app_init.rs`

## Status

COMPLETE.
