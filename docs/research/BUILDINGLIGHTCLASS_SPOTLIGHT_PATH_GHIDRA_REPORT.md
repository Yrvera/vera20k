# BuildingLightClass Spotlight Path -- Ghidra Research Report

**Address(es):** `0x00435820` constructor, `0x004361D0` AI, `0x00435BE0`/`0x00435C10` draw, `0x00436BE0` target selection, `0x00436E80` distance/intensity helper  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `BuildingLightClass` lifecycle and behavior for building spotlights/searchlights allocated at `BuildingClass+0x600` from `BuildingTypeClass+0x154B HasSpotlight=`.  
**Non-Scope:** `LightSourceClass` radius lamps at `BuildingClass+0x614`, map `[Lighting]` ambience math, `ExtraLight=`, Lightning Storm, EBOLT/Tesla weapon visuals, and full trigger-action naming tables.  
**Confidence:** High for allocation, parser keys, class fields, AI/draw branch behavior, save/load, and teardown. Medium for exact visible beam primitive rasterization because the low-level surface/primitive helpers were identified but not drained.  
**Active in YR:** Conditional. The parser and runtime paths are live in `gamemd.exe`, but shipped `rules.ini`/`rulesmd.ini` in this repo contain no `HasSpotlight=` assignments; activation requires a map/mod/rules override that sets `HasSpotlight=yes` on a building type and places that building.

## 1. Overview

`BuildingLightClass` is the directional spotlight/searchlight object attached to a building at `BuildingClass+0x600`. It is separate from the colored radius-light system: lamp posts use `LightSourceClass` at `BuildingClass+0x614` and `LightVisibility/LightIntensity/*Tint`, while `BuildingLightClass` owns a moving beam endpoint, draws screen-space spotlight geometry, and can run a search/target mode against nearby enemy objects.

The class does not behave like a point light. In the functions decompiled for this slice, it does not allocate `LightSourceClass`, does not call the LightConvert pipeline, and does not write per-cell RGB ambience. Its player-visible output is an overlay beam drawn by `Draw_It`; its gameplay-adjacent output is a `TechnoClass__ProcessCellAction(0x23, ...)` call when the sweeping/search mode finds an enemy in the tested area.

## 2. Class Layout / Key Offsets

Offsets below are byte offsets in `BuildingLightClass` unless otherwise stated.

| Offset | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `BuildingClass+0x600` | ptr | `BuildingLightClass*` spotlight pointer | `BuildingClass__Unlimbo @ 0x00441187` allocates `0xE8` and stores `param_1[0x180]` | Conditional: only if `Type+0x154B != 0` |
| `BuildingClass+0x614` | ptr | `LightSourceClass*` radius/ambient light, not this system | `BuildingClass__Unlimbo @ 0x00441057..0x004410A2`; separate constructor `0x00554760` | Conditional; non-scope |
| `BuildingTypeClass+0x154B` | bool | `HasSpotlight=` | `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` reads `s_HasSpotlight_0081AEA0`; ctor default clears it at `0x0045DFDA` | Conditional; default false |
| `+0xB0/+0xB4` | double | angular accumulator/current sweep angle | constructor zeroes `param_1[0x2C..0x2D]`; AI updates `*(double *)(param_1+0x2C)` | Conditional |
| `+0xB8..+0xC0` | coord | one arc endpoint / beam endpoint | constructor writes `param_1[0x2E..0x30]` (indices 0x2E, 0x2F, 0x30 = bytes 0xB8, 0xBC, 0xC0; 3 ints) (corrected 2026-05-29: was `+0xB8..+0xC4`; binary shows 3-int range ending at 0xC0 via decompile_function 0x00435820 — OFFSET_RETYPED_WRONG) | Conditional |
| `+0xC4..+0xCC` | coord | second arc endpoint / beam endpoint | constructor writes `param_1[0x31..0x33]` (indices 0x31, 0x32, 0x33 = bytes 0xC4, 0xC8, 0xCC; 3 ints) (corrected 2026-05-29: was `+0xC8..+0xD4`; binary shows this range starts at 0xC4 not 0xC8 via decompile_function 0x00435820 — OFFSET_RETYPED_WRONG) | Conditional |
| `+0xD0/+0xD4` | double | angular velocity | constructor zeroes `param_1[0x34..0x35]`; AI accelerates/decelerates using Rules `+0x798` | Conditional |
| `+0xD8` | bool | sweep side / direction toggle | constructor clears byte at `param_1+0x36`; AI flips when velocity crosses zero | Conditional |
| `+0xDC` | int | mode | `FindTarget @ 0x00436BE0` writes `param_1[0x37] = param_2`; AI switches on it | Conditional |
| `+0xE0` | ptr | target object | `FindTarget @ 0x00436BE0` writes nearest non-ally in mode 3; `Detach @ 0x00436A00` clears if detached | Conditional |
| `+0xE4` | ptr | owning building | constructor stores `param_2` at `param_1[0x39]`; load fixes this pointer | Conditional |
| size | `0xE8` | serialized/runtime class size | `BuildingLightClass__GetSize @ 0x00436900` returns `0xE8`; allocation uses `operator_new(0xE8)` | Conditional |

## 3. Core Logic

### Allocation and construction

Verified behavior:

- `BuildingTypeClass` constructor default sets `Type+0x154B` false. Active in YR: Yes as default data initialization. Evidence: constructor write at `0x0045DFDA`, also `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`.
- `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` reads `HasSpotlight` with `CCINIClass__ReadBool(section, "HasSpotlight", old_value)` and stores it at `Type+0x154B`. Active in YR: Yes for rules/map override parsing.
- `BuildingClass__Unlimbo @ 0x0044117A..0x0044118C` checks `*(char *)(Type+0x154B) != 0`, allocates `0xE8`, calls `BuildingLightClass__Constructor(building)`, and stores the result at `BuildingClass+0x600`. Active in YR: Conditional on `HasSpotlight=yes`.
- Constructor `0x00435820` registers the object in global vector `DAT_008B4194` / count `DAT_008B41A0`, stores the owner building at `+0xE4`, initializes mode with `FindTarget(1)`, and sets an alternating/toggle byte from the object's index parity in the vector. Active in YR: Conditional.

Constructor coordinate initialization uses the current `RateTimer__Current` value, subtracts `0x3FFF`, multiplies by a global radian-scale constant, then applies `Cos_lookup`/`Sin_lookup`. This means two spotlights created at different global timer values can start with different beam endpoints; they are not initialized from a fixed static angle. Active in YR: Conditional.

### Modes

`FindTarget @ 0x00436BE0` is the mode setter. It always resets angle/velocity (`+0xB0/+0xB4` and `+0xD0/+0xD4`) to zero and stores the passed mode at `+0xDC`. Active in YR: Conditional.

| Mode | Verified behavior | Evidence | Active in YR |
|---:|---|---|---|
| `1` | Default sweeping/search mode. Constructor and AI invalid-target fallback call `FindTarget(1)`. AI accelerates/decelerates a sweep between `-SpotlightAngle` and `+SpotlightAngle` using `SpotlightSpeed` and `SpotlightAcceleration`. It can call `TechnoClass__ProcessCellAction(0x23, ...)` when a nearby enemy is found in the tested cells. | constructor `0x00435B06..0x00435B0A`; AI `0x0043622x..0x004365xx`, enemy loop later in AI | Conditional |
| `2` | Continuous rotating mode. AI increments the angle by `Rules.SpotlightSpeed * constant` and wraps when the angle exceeds a full-circle constant. Uses building position as origin and transforms endpoint from Rules radius fields. | AI branch for `param_1[0x37] == 2` at `0x0043632F..0x004364xx` | Conditional |
| `3` | Target-tracking/search mode. `FindTarget(3)` scans a 3x3 cell box around the spotlight cell, selects the nearest non-ally object of RTTI `0xF` or `1`, and stores it in `+0xE0`. AI keeps tracking while target is alive and within `SpotlightMovementRadius`; otherwise it falls back to `FindTarget(1)`. | `FindTarget @ 0x00436BE0`; AI mode 3 branch at `0x0043627x..0x00436325` | Conditional |

Tiny details:

- Target scanning uses offsets `-1..1` on both axes, so only the 3x3 cell neighborhood around the spotlight's cell is searched. Evidence: nested `do` loops in `FindTarget @ 0x00436BE0`.
- The target filter calls virtual RTTI/type function `+0x2C` and accepts only IDs `0xF` or `1`, then rejects allied objects via `HouseClass__Is_Ally_ByObject`. Evidence: `FindTarget @ 0x00436C4x..0x00436D0x`; same pattern in AI enemy check.
- Nearest-target distance is full 3D coordinate distance, not Manhattan or 2D-only cell distance. Evidence: `Sqrt_Approx(dx^2 + dy^2 + dz^2)` in `FindTarget @ 0x00436CCx..0x00436D0x`.
- If no target is found in mode 3, `+0xE0` is not overwritten to a new object. The later AI invalid-target path resets the mode to `1`. Evidence: `FindTarget @ 0x00436D8x..0x00436DBx`; AI invalid target branch calls `FindTarget(1)` at `0x00436321..0x00436325`.
- `Detach @ 0x00436A00` clears both `+0xE0` target and `+0xE4` owner if the detached pointer matches. Active in YR: Yes for pointer-expiration/listener cleanup.

### Draw path

`BuildingLightClass__Draw_It @ 0x00435BE0` draws only when all of these gates pass:

1. mode `+0xDC != 0`;
2. owner building pointer `+0xE4 != NULL`;
3. owner RTTI/type virtual `+0x2C` returns `6` (building);
4. owner is alive/active (`owner+0x90` byte nonzero in decompiler field terms);
5. owner virtual `+0x350` returns true; decompiled `BuildingClass__CanSellOrUndeploy @ 0x004555D0` acts as an operational gate: power/EMP/health/power-ratio/engineer/special-mission checks;
6. owner byte `+0x6E7` is zero;
7. if `ScenarioClass flags & 0x1000` is set, `FUN_005865E0(coords)` must not reject drawing.

When drawing, the function creates a small heap primitive via `operator_new(0x18)` and `FUN_005FF250(..., 0x10)`, computes distance from owner to beam endpoint, chooses an intensity/color-like value, calls `FUN_005FF850` / `FUN_005FF2D0`, then emits two clipped beam segments through the primary surface virtuals `+0x78` and `+0x38` after converting coordinates through `TacticalClass__CoordsToClient2` and `Tactical__AdjustForZ`.

Negative fact: in the decompiled draw path, there is no `LightSourceClass__Constructor`, no `LightConvertClass`, no map cell RGB/ambient field write, and no `LightVisibility/LightIntensity/*Tint` read. Active in YR: Conditional. This is strong evidence that `BuildingLightClass` is a drawn overlay/searchlight path, not a lamp-post ambience path.

### Distance/intensity helper

`DistanceToIntensity @ 0x00436E80`:

- gets owner coordinates through owner virtual `+0x48`;
- gets spotlight coordinates through its own virtual `+0x48`;
- computes 3D distance via `Sqrt_Approx`;
- returns `0` when `distance < Rules+0x78C SpotlightLocationRadius`;
- otherwise returns `(distance - SpotlightLocationRadius) / ((SpotlightMovementRadius - SpotlightLocationRadius) / 10)`.

Active in YR: Conditional. The integer division and `/10` bucket are verified and matter: if `SpotlightMovementRadius == SpotlightLocationRadius` or their difference is less than `10`, the denominator can become zero. Stock rules avoid this with `2000` and `1000`.

## 4. INI Keys

| Key | Scope | Binary storage | Shipped value/default | Effect | Active in YR |
|---|---|---:|---|---|---|
| `HasSpotlight=` | building type | `BuildingTypeClass+0x154B` bool | ctor default false; no matches in repo `rules.ini`/`rulesmd.ini` | Gates allocation of `BuildingLightClass` at `BuildingClass+0x600` | Conditional: live parser, no stock assignment found |
| `SpotlightMovementRadius=` | `[General]` | `RulesClass+0x788` int | `rulesmd.ini:446 = 2000`; ctor default 2000 | Outer movement/target distance radius; also denominator for intensity bucket | Yes if any spotlight exists |
| `SpotlightLocationRadius=` | `[General]` | `RulesClass+0x78C` int | `rulesmd.ini:447 = 1000`; ctor default 1000 | Inner radius / near-origin cutoff | Yes if any spotlight exists |
| `SpotlightSpeed=` | `[General]` | `RulesClass+0x790` double | `rulesmd.ini:445 = .015`; ctor default appears as double `0.05` | Max angular speed / continuous rotation speed | Yes if any spotlight exists |
| `SpotlightAcceleration=` | `[General]` | `RulesClass+0x798` double | `rulesmd.ini:448 = .0025`; ctor default appears as double `0.005` | Accel/decel step for sweep mode | Yes if any spotlight exists |
| `SpotlightAngle=` | `[General]` | `RulesClass+0x7A0` double | `rulesmd.ini:449 = .5`; ctor default appears as double `20.0` | Sweep bound in mode 1 | Yes if any spotlight exists |
| `SpotlightRadius=` | `[General]` | `RulesClass+0x7A8` int | not present in repo INI; ctor default `0xAF` / 175 | Added to detection/draw intensity radius in AI/draw | Yes if any spotlight exists, but only default unless map/mod sets it |

`RulesClass__ReadGeneral @ 0x0066D530` reads all six spotlight rule keys in sequence: movement radius, location radius, speed, acceleration, angle, and radius. The repo INI comment block lists the first five only; the binary also reads `SpotlightRadius`.

The building named `CAURB03` has `Name=Hollywood Spotlight` in `rulesmd.ini`, but it does not set `HasSpotlight=` in the repo INI data. Active in YR: its building type is active data, but it is not binary evidence of this `BuildingLightClass` path being used by stock content.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Building placement | Allocates `+0x600` after normal placement/registration when `Type+0x154B` is true | `BuildingClass__Unlimbo @ 0x0044117A..0x0044118C` | Conditional |
| Save/load | `BuildingLightClass` is serialized and pointer-fixed; owner `+0xE4` and target `+0xE0` are registered in the global fixup dictionary | `Load @ 0x00436950`, `Save @ 0x00436F40` | Conditional |
| Object detach | Clears target and owner pointer slots when the detached pointer matches | `Detach @ 0x00436A00` | Conditional |
| Runtime AI | Vtable entry points to `AI @ 0x004361D0`; updates sweep/target/rotation and can fire cell action `0x23` | vtable data xref `0x007E3B2C`; AI decompile | Conditional |
| Runtime draw | Vtable entry points to `Draw_It @ 0x00435BE0`; direct screen-space beam drawing | vtable data xref `0x007E3BE4`; draw decompile | Conditional |
| Trigger action | Trigger action case `0x34` scans buildings and calls `FindTarget(action_param)` on powered, alive, non-limbo, `HasSpotlight` buildings in the matching trigger house/filter | `TriggerAction__Execute @ 0x006DEFxx..0x006DF036`, split helper `0x006E2990` | Conditional; requires trigger/action data |
| Map `[Structures]` load | After a structure is unlimboed, if `+0x600` exists, calls `FindTarget(parsed_field)` | `BuildingClass__ReadFromINI @ 0x0044FC07..0x0044FC16`; assembly shows `ECX=[ESI+0x600]`, `PUSH EDX` | Conditional; requires placed HasSpotlight building |
| Teardown | `BuildingClass__Limbo` calls spotlight vtable `+0xF8` when `+0x600 != NULL`; class destructor removes object from global vector | `BuildingClass__Limbo @ 0x00445D6x`; `BuildingLightClass__Destructor @ 0x004370C0` | Conditional |

Tick-order note: `BuildingLightClass` is an `ObjectClass`-derived object inserted into the global object vector during construction, so its AI/draw are reached through normal object/vtable dispatch. This report verified the class methods and vtable references but did not drain the global object scheduler.

## 6. Current Rust Implementation Status

Current Rust has map ambience and point lights but no `BuildingLightClass` spotlight model:

- `src/map/lighting.rs:1-9` describes map `[Lighting]` and point-light falloff. It does not define a directional spotlight/searchlight class.
- `src/map/lighting.rs:148-179` collects only `LightVisibility/LightIntensity/*Tint` point lights from structure entities.
- `src/map/lighting.rs:182-217` accumulates point lights into an RGB lighting grid. This should remain separate from `BuildingLightClass`.
- `src/app_init.rs:339-366` builds the lighting grid, accumulates point lights, and applies `ExtraLight`. It has no directional beam overlay or spotlight AI path.
- `src/rules/object_type.rs:700-708` and `1095-1108` parse lamp fields, but no `HasSpotlight=` or `[General]` spotlight rule keys are parsed.

Rust delta: missing if mods/maps need `HasSpotlight=` parity; not a blocker for lamp-post ambience. Do not add spotlight behavior by extending `PointLight`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingTypeClass+0x154B HasSpotlight` ctor default | verified | `0x0045DFDA`; `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` | none |
| `HasSpotlight=` parser | verified | `0x0045FE50` reads `s_HasSpotlight_0081AEA0` into `+0x154B` | none |
| Spotlight `[General]` rule parser | verified | `RulesClass__ReadGeneral @ 0x0066D530`; `RULESCLASS_FIELDS.csv` | none for key ownership |
| `BuildingClass+0x600` allocation | verified | `BuildingClass__Unlimbo @ 0x0044117A..0x0044118C` | none |
| Constructor | verified | `BuildingLightClass__Constructor @ 0x00435820` | exact meaning of two initial endpoint coordinate pairs could be named better with runtime visualization |
| AI modes 1/2/3 | verified | `BuildingLightClass__AI @ 0x004361D0`; `FindTarget @ 0x00436BE0` | exact low-level `ProcessCellAction(0x23)` consequence deferred to trigger/cell-action system |
| Draw path | touched-not-exhausted | `Draw_It @ 0x00435BE0`, split `FUN_00435C10` | low-level primitive helpers `FUN_005FF250/850/2D0` and surface raster details not drained |
| Cell ambience contribution | verified negative for this slice | no LightSource/LightConvert/map RGB writes in decompiled constructor/AI/draw/target/helper methods | full renderer audit could search low-level primitive helpers, but class methods show no ambience writes |
| Target selection | verified | `FindTarget @ 0x00436BE0` | exact RTTI names for accepted IDs `0xF` and `1` not resolved here |
| Save/load | verified | `Load @ 0x00436950`, `Save @ 0x00436F40` | none |
| Detach | verified | `Detach @ 0x00436A00` | none |
| Teardown | verified | `BuildingClass__Limbo @ 0x00445D6x`, destructor `0x004370C0` | whether Limbo should null `+0x600` after vtable call is not visible in this decompilation; future lifecycle test should catch dangling-reuse cases |
| Stock INI activation | verified negative in repo data | repo `rg -i "HasSpotlight"` finds no INI assignments | retail install/custom maps outside repo not exhaustively searched |
| Current Rust spotlight support | verified | `rg` over `src/`; `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs` | implementation absent |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] Q1 -- Is `+0x600` the lamp radius light or spotlight? -> Spotlight/searchlight; lamp radius light is `+0x614 LightSourceClass*`.` (evidence: `0x0044117A`, `0x00441057`; prior docs)`
- `[RESOLVED] Q2 -- What allocates `BuildingLightClass`? -> `BuildingClass__Unlimbo` allocates size `0xE8` and stores it at `BuildingClass+0x600` only when `Type+0x154B != 0`.` (evidence: `0x0044117A..0x0044118C`)`
- `[RESOLVED] Q3 -- Which INI key controls allocation? -> `HasSpotlight=` parsed into `BuildingTypeClass+0x154B`, default false.` (evidence: `0x0045FE50`, `0x0045DFDA`)`
- `[RESOLVED] Q4 -- Do shipped repo rules set `HasSpotlight=`? -> No matches in `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, or `ini/artmd.ini`.` (evidence: repo `rg -i "spotlight"` / `rg "HasSpotlight"`)`
- `[RESOLVED] Q5 -- Which global spotlight rules exist? -> Movement radius, location radius, speed, acceleration, angle, and hidden/default-only `SpotlightRadius` are read from `[General]`.` (evidence: `0x0066D530`, `RULESCLASS_FIELDS.csv`)`
- `[RESOLVED] Q6 -- Does `BuildingLightClass` use `LightVisibility/LightIntensity/*Tint`? -> No in the decompiled class methods; those belong to `LightSourceClass`/lamp point lights.` (evidence: `0x00435820`, `0x004361D0`, `0x00435BE0`, `0x00436E80`)`
- `[RESOLVED] Q7 -- Does it write map cell ambience? -> No cell RGB/LightConvert/LightSource writes were found in constructor, AI, draw, target, detach, save/load, or distance helper.` (evidence: same decompile set)`
- `[RESOLVED] Q8 -- Is it drawn visually? -> Yes; `Draw_It` emits beam geometry through tactical coordinate conversion and primary-surface calls.` (evidence: `0x00435BE0`, `0x00435C10`)`
- `[RESOLVED] Q9 -- What gates drawing? -> mode nonzero, owner building exists/is alive, RTTI building, owner operational via vtable `+0x350`, owner byte `+0x6E7 == 0`, and optional scenario flag `0x1000` visibility check.` (evidence: `0x00435BE0`, `0x004555D0`)`
- `[RESOLVED] Q10 -- What are the modes? -> `1` sweeping default/search, `2` continuous rotation, `3` nearest-target tracking; set by `FindTarget`.` (evidence: `0x00436BE0`, `0x004361D0`)`
- `[RESOLVED] Q11 -- How are targets selected? -> Mode 3 scans 3x3 cells, accepts RTTI `0xF` or `1`, rejects allies, chooses shortest 3D distance.` (evidence: `0x00436BE0`)`
- `[RESOLVED] Q12 -- What happens if the target is invalid? -> AI calls `FindTarget(1)` and returns to sweeping mode.` (evidence: `0x00436321..0x00436325`)`
- `[RESOLVED] Q13 -- Does it have gameplay side effects? -> In mode 1, if enemy objects are found close enough in the tested cells, AI calls `TechnoClass__ProcessCellAction(0x23, ...)`.` (evidence: `0x0043660A..0x004368xx`)`
- `[RESOLVED] Q14 -- Is `BuildingLightClass` saved/loaded? -> Yes; save writes custom fields and serializes target/owner object IDs when non-null; load registers `+0xE4` and `+0xE0` for pointer fixup.` (evidence: `0x00436F40`, `0x00436950`)`
- `[RESOLVED] Q15 -- How is it torn down? -> `BuildingClass__Limbo` calls vtable `+0xF8` on `+0x600`; class destructor conceals object, removes it from global vector, and optionally frees memory.` (evidence: `0x00445880`, `0x004370C0`)`
- `[RESOLVED] Q16 -- Does current Rust implement this? -> No; Rust implements map lighting, point lights, and ExtraLight only.` (evidence: `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs`)`
- `[DEFERRED] Q17 -- What exact visible pixels do `FUN_005FF250/850/2D0` draw?` (category: `bounded-cost-too-high`; reason: low-level primitive/raster helpers are separate from identifying the spotlight path; next-step-if-pursued: investigate spotlight primitive rendering with screenshots and helper decompilation)`
- `[DEFERRED] Q18 -- What exact effect does `TechnoClass__ProcessCellAction(0x23)` have for spotlight detection?` (category: `requires-different-system-context`; reason: cell actions and trigger/action semantics are broader than this class slice; next-step-if-pursued: trace cell action `0x23` from `TechnoClass__ProcessCellAction`)`
- `[DEFERRED] Q19 -- Are any retail install maps outside this repo setting `HasSpotlight=` via map rule overrides?` (category: `out-of-scope`; reason: repo INI was checked, retail map archive scan was not part of this slot; next-step-if-pursued: scan extracted retail `.map/.yrm/.mpr` files for `HasSpotlight`)`

Adversarial corner cases answered:

- Null owner: AI destructor path calls vtable `+0xF8` and returns; Draw does nothing. Evidence: `AI @ 0x004361D0`, Draw gates.
- Owner unpowered/EMP/zero health/low power: operational gate `0x004555D0` returns false, so Draw does nothing. Evidence: `0x004555D0`.
- Target destroyed/invalid: Detach clears target; AI falls back to mode 1 when target invalid/out of range. Evidence: `0x00436A00`, `0x004361D0`.
- Save/restore with target and owner pointers: both pointer slots are fixup-registered. Evidence: `0x00436950`.
- Stock maps/rules with no `HasSpotlight`: no allocation occurs; `+0x600` remains null. Evidence: `0x0044117A`, repo INI search.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `HasSpotlight=` is a building-type bool at `+0x154B`, default false, and gates `BuildingLightClass` allocation | `0x0045DFDA`, `0x0045FE50`, `0x0044117A` | missing | `src/rules/object_type.rs` or building-type rules model | Parse and carry `HasSpotlight` separately from point-light fields | Unit test `building_type_has_spotlight_defaults_false_and_parses_true` | Do not infer it from building names like "Hollywood Spotlight" |
| Spotlight rules are global `[General]` keys, including binary-read `SpotlightRadius` | `0x0066D530`, `RULESCLASS_FIELDS.csv` | missing | rules parser / global rules model | Parse `SpotlightMovementRadius`, `SpotlightLocationRadius`, `SpotlightSpeed`, `SpotlightAcceleration`, `SpotlightAngle`, `SpotlightRadius` | Test `general_spotlight_rules_parse_with_rulesmd_defaults` | Do not omit `SpotlightRadius` just because it is absent from comments in repo INI |
| Spotlights are not lamp ambience and do not consume `LightVisibility/LightIntensity/*Tint` | `0x00435820`, `0x00435BE0`, `0x00436E80`; Rust point-light code | absent but cleanly separate | future render overlay + sim/object visual state | Implement a directional spotlight overlay/search object, not a `PointLight` | Mod fixture with `HasSpotlight=yes` should draw a beam without changing surrounding lamp tint grid | Do not merge `+0x600` into `src/map/lighting.rs::PointLight` |
| Draw requires owner operational gate (`vtable+0x350`) | `0x00435BE0`, `0x004555D0` | missing | building power/EMP/render eligibility integration | Beam disappears when owner is off/EMP/zero health/low power as binary gate dictates | Test `spotlight_not_drawn_when_owner_offline_or_emp_locked` | Do not draw spotlights unconditionally for placed HasSpotlight buildings |
| Mode 1 sweep accelerates/decelerates between angle bounds; mode 2 rotates; mode 3 tracks nearest non-ally in 3x3 cells | `0x004361D0`, `0x00436BE0` | missing | sim visual-state update or deterministic render-state update | Maintain mode, angle, velocity, direction, and optional target object | Tests `spotlight_mode1_sweeps_between_angle_bounds`, `spotlight_mode3_selects_nearest_enemy_in_3x3` | Do not use frame-rate floating drift; gameplay-facing state should be deterministic |
| Search mode can trigger `TechnoClass__ProcessCellAction(0x23)` when an enemy is close enough | `0x004361D0` | missing/unchecked | future cell-action/trigger interaction | Preserve side effect or explicitly defer until cell action `0x23` is implemented | Test proposal `spotlight_search_triggers_cell_action_23_for_enemy_in_beam` after cell-action research | Do not model the beam as purely cosmetic if maps rely on this trigger action |
| Save/load preserves spotlight object, owner, and target pointer slots | `0x00436950`, `0x00436F40` | missing | save/load system | Serialize spotlight mode/angle/velocity/endpoints/target/owner or reconstruct with equivalent observable state | Test `spotlight_save_load_preserves_mode_and_target_pointer` | Do not recreate all spotlights from type data on load if mode/target state matters |

### Negative Facts / Do Not Do

- Do not implement `HasSpotlight` as a colored radius light.
- Do not use `LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, or `LightBlueTint` for `BuildingLightClass`.
- Do not assume stock `CAURB03` activates this system merely because its display name is "Hollywood Spotlight"; repo INI does not set `HasSpotlight=`.
- Do not treat prior doc wording "NATESLA spotlight" as stock-active without a data source that sets `HasSpotlight=yes`; no repo INI match was found.
- Do not claim exact pixel parity for the beam until the low-level primitive helpers and screenshots are audited.

### Stale Docs / Follow-up Docs

- `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md` line 674 says "HasSpotlight=yes for NATESLA" and line 675 says the spotlight updates the cell light map. Replacement wording: "`BuildingLightClass` is allocated for buildings whose parsed `HasSpotlight=` flag is true; no shipped repo INI assignment was found in this investigation. Its decompiled class methods draw a directional overlay and run search logic; they do not show `LightSourceClass`/LightConvert cell-ambience writes."
- `BUILDINGCLASS_FIELD_VERIFICATION_ROUND_2.md` says used by "Prism Tower / searchlight buildings." Replacement wording: "used by buildings with `HasSpotlight=yes`; stock activation was not found in repo INI data."

## Evidence Needed to Mark COMPLETE

This report is complete for identifying and separating the `BuildingLightClass` path from map/lamp ambience. To mark the broader player-visible spotlight feature complete for implementation, a follow-up should verify:

- the exact beam primitive rasterization and blend/color behavior in `FUN_005FF250`, `FUN_005FF850`, `FUN_005FF2D0`, and primary-surface virtual calls;
- the exact downstream effect of `TechnoClass__ProcessCellAction(0x23)`;
- whether any extracted retail maps or mission INI overrides set `HasSpotlight=`;
- screenshot-confirmed behavior for a minimal custom map with one `HasSpotlight=yes` building.

## Stop Conditions

The investigation stopped after decompiling constructor, AI, draw, target selection, distance helper, detach, save/load, destructor, allocation caller, parser, operational gate, trigger activation hook, map structure-load hook, and teardown. It did not descend into low-level draw primitive helpers or general cell-action semantics because those are separate systems and would exceed this slot's scoped target.

## Sources

- Ghidra decompilations: `0x00435820`, `0x004361D0`, `0x00435BE0`, `0x00435C10`, `0x00436A00`, `0x00436BE0`, `0x00436E80`, `0x00436900`, `0x00436950`, `0x00436F40`, `0x004370C0`, `0x00440580`, `0x00445880`, `0x0044FC16`, `0x004555D0`, `0x0045DFDA`, `0x0045FE50`, `0x0066D530`, `0x006DF031`, `0x006E2990`.
- Ghidra xrefs: constructor xrefs from `BuildingClass__Unlimbo` and object factory; method vtable data refs for AI/draw/detach/load/save/destructor; `FindTarget` xrefs from constructor, AI, `BuildingClass__ReadFromINI`, and trigger action.
- Repo INI: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`.
- Prior docs: `BUILDINGCLASS_FIELD_VERIFICATION_ROUND_2.md`, `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md`, `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md`, `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md`, `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`, `BUILDINGTYPECLASS_FIELDS.csv`, `RULESCLASS_FIELDS.csv`, `RULESCLASS_CONSTRUCTOR_DEFAULTS.csv`.
- Rust scan: `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs`.
