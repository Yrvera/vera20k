# FLH, Turret, and Visual Offsets -- Ghidra Research Report

**Address(es):** `0x006F3AD0`, `0x00453840`, `0x00453BF0`, `0x006D2070`, `0x00451890`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Coordinate axes/reference points for `PrimaryFireFLH`, `SecondaryFireFLH`, elite FLH slots, `TurretOffset`/building turret draw position, `PrimaryFirePixelOffset`/`SecondaryFirePixelOffset`, building active anim X/Y/YSort/ZAdjust storage/use, projectile/muzzle source positions.
**Non-Scope:** Full weapon damage logic, full building animation lifecycle/tier switching, projectile ballistics after launch, Ares/YRpp extensions.
**Confidence:** High for fire-origin coordinate spaces and sign conventions; Medium for complete active-anim lifecycle.
**Active in YR:** Yes. All verified functions are on live YR Techno/Building fire or render/animation paths.

## 0. Investigation Setup

**Target question:** Which coordinate space and reference point does gamemd use for FLH, turret, fixed building fire pixel offsets, building active anim offsets, and projectile/muzzle origins?

**Non-goals:** Do not re-document burst ROF, projectile physics, all building animation slot state transitions, or Rust implementation patches.

**Evidence needed to mark COMPLETE:** Direct decompilation of live fire-origin functions; exact sign convention for FLH lateral/height/forward; proof whether fixed building fire offsets are screen-pixel or lepton/facing-relative; proof of projectile/muzzle source read; Rust-facing deltas and tests.

**Stop conditions:** Stop if function boundary is missing and read-only Ghidra cannot inspect it, if an offset path requires runtime-only debugger evidence, or if a related lifecycle branch is outside the coordinate-space question.

## 1. Overview

`TechnoClass::GetFLH` computes the world/lepton coordinate where most unit/techno weapons originate. It reads an FLH triplet in leptons, transforms it by a 32-way facing matrix, adds the result to `GetRenderCoords`, and returns a world `CoordStruct` used by bullets, muzzle anims, sounds, lasers, waves, and special effects.

Buildings override the same virtual at `BuildingClass::GetFLH`. Turretless/fixed-origin building fire uses `PrimaryFirePixelOffset`/`SecondaryFirePixelOffset` as isometric screen pixel offsets converted back to world leptons, not facing-relative FLH.

## 2. Key Offsets / Fields

| Field | Offset | Space | Verified use |
|---|---:|---|---|
| Normal weapon FLH slot | `TechnoType + 0x898 + weapon_idx*0x1C + 4/8/0xC` | Leptons | `TechnoClass::GetFLH @ 0x006F3AD0`, via `GetWeapon @ 0x0070E140` and slot helpers `0x007177C0/0x007177E0` |
| Elite weapon FLH slot | `TechnoType + 0xA94 + weapon_idx*0x1C + 4/8/0xC` | Leptons | `GetWeapon @ 0x0070E140` selects elite slot when elite and slot has weapon |
| Alternate negative FLH slots | `TechnoType + 0x850 + (-weapon_idx)*0x0C` | Leptons | `GetFLH @ 0x006F3AD0`, negative `-1..-5` branch |
| `TurretOffset` / render turret location | type-derived; unit art parser maps to Rust `ArtEntry::turret_offset` | Leptons | Unit Rust currently screen-transforms it; binary building equivalent uses `GetTurretDrawPosition @ 0x00453BF0` with lepton-like X/Y/Z fields |
| Building `PrimaryFirePixelOffset` | `BuildingType + 0xE44/+0xE48` | Isometric screen pixels | `BuildingClass::GetFLH @ 0x00453840`; ctor sentinel `0xFFFF,0xFFFF` at `0x0045DE40/46` |
| Building `SecondaryFirePixelOffset` | `BuildingType + 0xE4C/+0xE50` | Isometric screen pixels | same branch family; ctor sentinel at `0x0045DE4C/52` |
| `PrimaryFireDualOffset` | `BuildingType + 0x1764` | bool | `BuildingClass::GetFLH @ 0x00453840`, conditionally adds `PrimaryFirePixelOffset` to `TechnoClass::GetFLH` |
| Garrison muzzle flash offsets | `BuildingType + 0x1588 + CurrentFirePort*8` | Isometric screen pixels | `BuildingClass::GetFLH @ 0x00453840`, converted via `IsometricPixelToWorld` |
| Building anim X/Y/ZAdjust/YSort | per slot table, e.g. `ActiveAnimX/Y/ZAdjust/YSort = +0x1048/+0x104C/+0x1050/+0x1054` | X/Y screen pixels, Z/YSort draw offsets | `CreateAnimForSlot @ 0x00451890` copies slot `+0xF84/+0xF88` to `AnimClass +0x100/+0x104`; field map in `BUILDINGTYPECLASS_FIELDS.csv` |

## 3. Core Logic

### `TechnoClass::GetFLH @ 0x006F3AD0`

1. Gets `TechnoType` through vtable `+0x84`.
2. If `weapon_idx >= 0`, calls vtable `+0x3F8` (`TechnoClass::GetWeapon @ 0x0070E140`), then reads `slot+4`, `slot+8`, `slot+0xC` as the FLH triplet.
3. If `weapon_idx < 0`, only `-1..-5` are accepted by the branch `weapon_idx == -5 || -weapon_idx < 5`; it reads `TechnoType + 0x850 + (-weapon_idx)*0x0C`.
4. Adds caller extra X only to the first FLH component before matrix construction.
5. Chooses a facing angle from the current body facing. The formula is `((((facing >> 10) + 1) >> 1) & 0x1F) - 8`, multiplied by `PI/16` (`_DAT_007e4408`).
6. If a locomotor matrix path is active, it subtracts a second 32-way `RateTimer::Current` angle from the body angle.
7. Matrix order is translate by `Type + 0x720` on X, rotate Z by the quantized angle, then translate by `(flhZ + extraZ, sign * (flhY + extraY), 0)`.
8. The sign for the second translate component comes from `CurrentBurstIndex` (`param_1[0xEE]` = byte `+0x3B8`): odd current burst uses positive lateral, even uses negative lateral.
9. The transformed origin is rounded by `Math__ftol` and added to vtable `+0xAC` (`GetRenderCoords`).

Important sign result: the INI triplet is not projected directly as `(forward,lateral,height)` in world XY. The binary feeds `FLH.Z` into the first transformed axis and `FLH.Y` into the second transformed axis after the initial type translation. The ordinary player-facing convention still holds at the data level: first value = forward/barrel length, second = lateral/barrel side, third = height, but implementation maps them through the voxel matrix axes.

### `TechnoClass::GetWeapon @ 0x0070E140`

`GetWeapon(-1)` returns null. For elite technos, the function probes the elite table helper `0x007177E0` (`type + 0xA94 + idx*0x1C`) and returns it only if the slot and weapon pointer are non-null; otherwise it falls back to normal helper `0x007177C0` (`type + 0x898 + idx*0x1C`). This proves elite FLH does not require a separate GetFLH path.

### `BuildingClass::GetFLH @ 0x00453840`

The building override branches before falling back to generic `TechnoClass::GetFLH`:

1. If garrison fire positions are active (`Type + 0x157B`) and fire-port count vtable `+0x408` is positive, it converts `Type + 0x1588 + CurrentFirePort*8` from isometric pixels to world leptons and adds to building `GetRenderCoords`.
2. If both primary pixel-offset ints are sentinel `0xFFFF,0xFFFF`, it either:
   - returns `GetTurretDrawPosition` for voxel barrel/turret buildings when `Type + 0x16C6` is true, or
   - calls `TechnoClass::GetFLH`; if `Type + 0x16C5` is true, adds the turret anim pixel offset `Type + 0x11E0/+0x11E4` after `IsometricPixelToWorld`.
3. If primary pixel offset exists and `PrimaryFireDualOffset` (`Type + 0x1764`) is true, it converts `Type+0xE44/+0xE48` and adds that world delta to generic `TechnoClass::GetFLH`.
4. Otherwise, it converts the fixed primary pixel offset and adds it to `GetRenderCoords`, preserving Z from render coords.

The fixed building pixel offset is therefore screen/isometric-pixel data that becomes a world/lepton delta before the projectile is spawned. It is not facing-relative and not an already-final screen-space-only decoration.

### `IsometricPixelToWorld @ 0x006D2070`

Input is two ints `{pixel_x, pixel_y}` plus implicit `z=0.0`. The function runs `Matrix3x4_TransformPoint`, rounds X/Y through `Math__ftol`, writes only output X/Y, and leaves no Z component. This helper is shared by damage fire, garrison ports, and fixed building fire offsets.

The exact active matrix and conversion are now verified. `IsometricPixelToWorld` uses `TacticalClass+0xDE4`; `TacticalClass::Constructor @ 0x006D1C20` overwrites the full 3x4 matrix with float words `{0x408888CE,0x410888CE,0,0, 0xC08888CE,0x410888CE,0,0, 0,0,0x3F800000,0}`. `Matrix3x4_TransformPoint @ 0x005AFB80` therefore produces, for `A=f32::from_bits(0x408888CE)` and `B=f32::from_bits(0x410888CE)=2*A`, `world_x=f32(A*pixel_x+B*pixel_y)` and `world_y=f32(-A*pixel_x+B*pixel_y)`. The outputs are stored as f32 before `Math__ftol @ 0x007C5F00`; its forced control word is `0x0E7F`, so `FISTP` truncates toward zero. An exact Rust implementation may equivalently compute one f32-rounded product `A*(pixel_x+2*pixel_y)` and `A*(-pixel_x+2*pixel_y)` during validated metadata initialization, then store only the resulting signed integer lepton deltas for simulation. Evidence: read-only live calls `mcp__ghidra_mcp__disassemble_function(0x006D2070)`, `mcp__ghidra_mcp__decompile_function(0x006D1C20)`, `mcp__ghidra_mcp__decompile_function(0x005AFB80)`, `mcp__ghidra_mcp__disassemble_function(0x007C5F00)`, and `mcp__ghidra_mcp__read_memory(0x00822D80,4)` on 2026-07-18.

### `GetTurretDrawPosition @ 0x00453BF0`

For building voxel turrets/barrels, the function reads `Type+0x1754` forward, `+0x1758` lateral, `+0x175C` vertical. It alternates lateral by burst: first alternate uses `+lateral`, second uses `-lateral`, otherwise `0`. It builds the VXL turret matrix (`0x00458810`), transforms the vector, adds building `GetRenderCoords`, then additionally converts a pixel-like stack vector through `IsometricPixelToWorld` before returning final world coords. Active in YR for building voxel turrets/barrels.

### `BuildingClass::CreateAnimForSlot @ 0x00451890`

This helper creates the `AnimClass` for active/idle/special/production building anim slots. It calls building `GetRenderCoords`, constructs the anim, then copies per-slot type fields into `AnimClass +0x100` and `+0x104` from `Type + slot*0x44 + 0xF84/+0xF88`. Existing field maps identify these as the slot's `ZAdjust` and `YSort` pair; the string/field table maps ActiveAnim slot 3 X/Y/ZAdjust/YSort at `+0x1048/+0x104C/+0x1050/+0x1054`, with subsequent ActiveAnimTwo/Three/Four entries stride `0x44`.

## 4. INI Keys

| Key | Type/default | Space | Effect |
|---|---|---|---|
| `PrimaryFireFLH`, `SecondaryFireFLH` | triplet, default `0,0,0` | leptons | normal weapon hardpoint; selected by weapon slot |
| `ElitePrimaryFireFLH`, `EliteSecondaryFireFLH` | triplet optional | leptons | selected by elite weapon-slot table when present |
| `WeaponNFLH` | triplet optional | leptons | multi-weapon slot table, same `GetWeapon` slot path |
| `AlternateFLH0..4` | triplet optional | leptons | negative index `-1..-5` path |
| `TurretOffset` | int, default `0` | leptons | unit/building turret pivot offset, body/turret-reference data |
| `PrimaryFirePixelOffset`, `SecondaryFirePixelOffset` | pair, ctor sentinel `0xFFFF,0xFFFF` | isometric screen pixels | fixed building fire origin converted by `IsometricPixelToWorld` |
| `PrimaryFireDualOffset` | bool, default false | bool | adds primary pixel offset to generic FLH path for alternating building barrels |
| `ActiveAnimX/Y` | ints, default `0` | screen pixels | building overlay placement relative to building render point |
| `ActiveAnimZAdjust/YSort` | ints, default `0` | draw/depth metadata | copied to `AnimClass +0x100/+0x104`; affects anim draw offset/sort behavior |

## 5. Integration Points

`TechnoClass::Fire_At @ 0x006FDD50` calls vtable `+0xB0` at `0x006FE260`-range to fill `iStack_8C/88/84` with the muzzle source before bullet allocation/init. Later in the same function, muzzle report sound (`VocClass__PlayAt`), muzzle anim (`AnimClass__Constructor`), waves, lasers, rad beams, EBolts, particle systems, and bullet trajectory use that same computed source coordinate or directly related local copies.

`ObjectClass::GetRenderCoords @ 0x0041BE00` delegates to `GetCoords`; `BuildingClass::GetRenderCoords @ 0x00459EF0` returns object coords with X and Y each shifted by `-0x80` leptons. Building fixed pixel offsets are added to this shifted render reference, not raw foundation origin.

## 6. Current Rust Implementation Status

Scanned Rust surfaces:

| Surface | Current status |
|---|---|
| `src/rules/flh.rs` | Parses FLH triplets and resolves primary/secondary/elite; does not represent negative `AlternateFLH0..4` or `WeaponNFLH` slot table as a generic binary-equivalent source. |
| `src/util/flh_transform.rs` | Uses screen-space approximation from FLH and facing. It includes 32-way quantization but maps first FLH component as forward in screen projection; binary world path maps through matrix axes and adds to world `GetRenderCoords`. |
| `src/app_fire_effects.rs` | Resolves muzzle anim/sound with screen offsets only; `FireOrigin.rx/ry/z` remains entity position rather than binary source world coords. Missing building fixed pixel-origin path. |
| `src/app_instances/units.rs` | TurretOffset screen transform exists for unit rendering. Needs validation against binary axis/sign, especially negative offsets and body-facing rotation. |
| `src/rules/art_data.rs` / `src/app_instances/shp.rs` | Parses building anim X/Y/ZAdjust/YSort and places overlays at screen X/Y. `YSort` is currently not used to alter depth; report scope confirms it is draw metadata copied to AnimClass. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::GetFLH` normal/elite/negative FLH | verified | `0x006F3AD0`, `0x0070E140`, `0x007177C0`, `0x007177E0` | none for coordinate contract |
| FLH 32-way facing quantization | verified | `0x006F3AD0` formula `>>10`, `&0x1F`, `PI/16` | exact render-vs-sim rounding visual diff test |
| Lateral alternation | verified | `0x006F3AD0` reads `CurrentBurstIndex` via `param_1[0xEE]` and flips sign | none |
| Building fixed pixel fire origin | verified | `0x00453840`, `0x006D2070`, ctor sentinel `0x0045DE40..52` | secondary branch exact selector not separately decompiled; same field family |
| Building turret draw position | verified | `0x00453BF0`, `0x00458810` | full VXL matrix internals not needed for fire-origin contract |
| Projectile/muzzle source use | verified | `0x006FDD50` vtable `+0xB0` source before bullet/anim/sound | none for source coordinate |
| Building active anim offsets | touched-not-exhausted | `0x00451890`, `BUILDINGTYPECLASS_FIELDS.csv` | full lifecycle and every slot variant out of scope |
| Rust parity comparison | verified-by-scan | listed files from repo scan | exact line-by-line implementation fix out of scope |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- Does unit/techno FLH produce screen offsets or world coordinates? -> World `CoordStruct` in leptons, added to `GetRenderCoords`.` (evidence: `0x006F3AD0`)`
- `[RESOLVED] OQ-2 -- What facing granularity does FLH use? -> 32-way quantized facing, `((((facing>>10)+1)>>1)&0x1F)-8` times `PI/16`.` (evidence: `0x006F3AD0`)`
- `[RESOLVED] OQ-3 -- Is lateral positive always right? -> Data convention is lateral; binary flips sign by `CurrentBurstIndex` before matrix translate, so burst parity alternates side.` (evidence: `0x006F3AD0`)`
- `[RESOLVED] OQ-4 -- Are building fire pixel offsets screen or world? -> Stored as isometric pixel pairs, converted to world X/Y through `IsometricPixelToWorld`.` (evidence: `0x00453840`, `0x006D2070`)`
- `[RESOLVED] OQ-5 -- What is the missing-pixel-offset default? -> Both ints initialized to `0xFFFF`; branch treats `0xFFFF,0xFFFF` as absent.` (evidence: `0x0045DE40..52`, `0x00453840`)`
- `[RESOLVED] OQ-6 -- Does Fire_At use GetFLH before projectile creation? -> Yes, vtable `+0xB0` fills muzzle source before bullet allocation/init and visual/audio effects.` (evidence: `0x006FDD50`)`
- `[RESOLVED] OQ-7 -- Are active anim X/Y world-space? -> No evidence of world/facing transform; building anim slot creation uses render coords plus per-slot draw metadata and active anim X/Y field family is pixel offset in art data.` (evidence: `0x00451890`, `BUILDINGTYPECLASS_FIELDS.csv`)`
- `[DEFERRED] OQ-8 -- Exact draw-depth arithmetic for `ActiveAnimYSort` across every draw pass` (category: `out-of-scope`; reason: this slot only verifies coordinate/reference spaces; next-step-if-pursued: audit `AnimClass::Draw` and tactical animation sort list)`

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| FLH source is a world `CoordStruct` from 32-way matrix transform plus `GetRenderCoords`, with burst lateral sign flip. | `0x006F3AD0`, `0x006FDD50` | mismatch/partial: Rust computes screen-only fire origin and keeps `rx/ry/z` at entity position | `src/util/flh_transform.rs`, `src/app_fire_effects.rs`, `src/sim/combat/combat_weapon.rs` | Provide a deterministic world-coordinate muzzle source for bullets/sounds/anims, then project to screen for presentation | Two-burst Grizzly/Rhino fire alternates muzzle side while projectile source and report sound originate from matching world point | `flh_world_source_32way_burst_lateral_alternates` | Do not implement FLH as a pure render-only screen offset. |
| Fixed building fire pixel offsets are isometric pixel pairs converted to world X/Y and added to building `GetRenderCoords`; absent sentinel is `0xFFFF,0xFFFF`. | `0x00453840`, `0x006D2070`, `0x0045DE40..52` | missing: Rust parses FLH but not fixed building fire pixel origin path | `src/rules/art_data.rs`, `src/app_fire_effects.rs`, future building weapon-source surface | Parse/store `PrimaryFirePixelOffset`/`SecondaryFirePixelOffset`; for turretless/fixed buildings convert pixel pair to world delta before projectile source | Prism Tower/Grand Cannon-style fixed muzzle uses art pixel origin independent of building facing | `building_primary_fire_pixel_offset_converts_iso_pixels_to_world_source` | Do not treat these keys as already-final screen decorations or facing-relative FLH. |
| Building active anim offsets are building-render-point pixel/draw offsets, not FLH/facing-relative weapon hardpoints. | `0x00451890`, `BUILDINGTYPECLASS_FIELDS.csv` | partial: X/Y used; `YSort` currently ignored for depth | `src/rules/art_data.rs`, `src/app_instances/shp.rs` | Keep X/Y as pixel placement; audit and model `YSort`/`ZAdjust` in anim draw ordering separately | GAWEAP/GAREFN active overlays sit at pixel offset and sort consistently with body and walls | `building_active_anim_xy_pixel_offsets_and_ysort_depth` | Do not route active anim X/Y through lepton or facing transforms. |
| Building voxel turret/barrel fire position uses `GetTurretDrawPosition` and building type turret vector fields when barrel/turret flags are set. | `0x00453840`, `0x00453BF0`, `0x00458810` | unchecked/missing for building VXL turrets | `src/app_render/build_instances.rs`, weapon fire origin builder | Add building-turret fire-origin path before generic FLH fallback for barrel/turret buildings | Voxel-turret building projectile starts at turret/barrel, not center/fixed pixel origin | `building_voxel_turret_fire_origin_uses_turret_draw_position` | Do not reuse unit `TurretOffset` screen helper blindly for building turret fire. |

## Negative Facts / Do Not Do

- Do not treat `PrimaryFirePixelOffset` or `SecondaryFirePixelOffset` as FLH triplets or facing-relative offsets.
- Do not store only screen-space fire origins; gamemd computes and consumes world/lepton source coordinates first.
- Do not skip the 32-way FLH quantization and replace it with 8-way anim-facing buckets.
- Do not ignore `CurrentBurstIndex` lateral sign flip; this is the twin-barrel visual/source alternation.
- Do not implement Ares-style per-burst FLH arrays for vanilla YR; stock gamemd reuses the same slot FLH and alternates side by sign.

## Remaining Uncertainty

The exact `ActiveAnimYSort` arithmetic in the final tactical animation sort pass was not re-decompiled here. Coordinate-space claim is sufficient for this slot, but draw-depth parity needs a separate focused animation-sort investigation.

## Proposed Rust Tests

- `flh_world_source_32way_burst_lateral_alternates`
- `flh_elite_slot_falls_back_to_normal_when_elite_slot_missing`
- `building_primary_fire_pixel_offset_converts_iso_pixels_to_world_source`
- `building_voxel_turret_fire_origin_uses_turret_draw_position`
- `building_active_anim_xy_pixel_offsets_and_ysort_depth`

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/timing/weapon-charge-and-muzzle.md`: replace "PrimaryFirePixelOffset: add screen-space pixel delta to the FLH at draw time" with "PrimaryFirePixelOffset/SecondaryFirePixelOffset are stored as isometric pixel pairs; `BuildingClass::GetFLH` converts them through `IsometricPixelToWorld` and adds the resulting world X/Y delta to either building `GetRenderCoords` or the generic FLH source depending on the building branch."
- `C:/Users/enok/Documents/ra2-rust-game-docs/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`: replace "Matrix3x4_Translate(flhZ + offsetZ, ySign * (flhY + offsetY), /*unaff_ESI*/ 0)" commentary with "The decompiler shows the third argument as an unrecovered register; the verified output contract is the transformed world source. Future audits should verify the third-axis source from assembly before claiming it is zero."

## Sources

- Ghidra decompiled: `0x006F3AD0`, `0x006F3D60`, `0x0070E140`, `0x007177C0`, `0x007177E0`, `0x00453840`, `0x00453BF0`, `0x00458810`, `0x006D2070`, `0x00451890`, `0x006FDD50`, `0x0041BE00`, `0x00459EF0`.
- Ghidra assembly context: `0x00715DA1`, `0x0045DE40`, `0x0045DE46`, `0x0045DE4C`, `0x0045DE52`.
- Docs referenced: `ABSTRACTCLASS_GHIDRA_REPORT.md`, `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`, `BUILDINGTYPECLASS_FIELDS.csv`, `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`, `DAMAGE_FIRE_ANIMS_GHIDRA.md`, `TURRET_ON_SLOPE_TILT_ACTUAL_PATH_GHIDRA_REPORT.md`, `TECHNOCLASS_VTABLE_COMPLETE.md`, `BUILDINGCLASS_VTABLE_COMPLETE.md`.
- Rust scanned: `src/rules/flh.rs`, `src/util/flh_transform.rs`, `src/app_fire_effects.rs`, `src/app_instances/units.rs`, `src/rules/art_data.rs`, `src/app_instances/shp.rs`.
