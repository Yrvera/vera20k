# Phase 3 Building Destruction `Explosion=` / `DestroyAnim=` — Ghidra Report

**Date:** 2026-08-29
**Program:** active retail Yuri's Revenge `gamemd.exe` (`image base 0x00400000`)
**Primary address:** `BuildingClass::DestructionEffects @ 0x004415F0`
**Supporting addresses:** `BuildingClass::ReceiveDamage @ 0x00442230`, `BuildingClass::SpawnSurvivors @ 0x00442D90`, `BuildingClass::GetRenderCoords @ 0x00459EF0`, `ObjectClass::GetCellFootprintOffsets @ 0x005F5B90`, `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::AI @ 0x00423AC0`, `AnimClass::Middle @ 0x00424CE0`, `AnimClass::Start @ 0x00424F00`, `FUN_0049F420 @ 0x0049F420`, `Random::Next @ 0x0065C780`, `Random::RandomRanged @ 0x0065C7E0`
**Investigation mode:** exhaustive slice
**Confidence:** High for the scoped emission order, coordinates, RNG stream/draw order, constructor rows, active-retail data, and Rust mismatch.

## 1. Scope and completion boundary

This report exactifies the two building-destruction animation mechanisms that the Phase 3 House-destruction critic identified as the largest remaining receiver prerequisite:

1. one `Explosion=` `AnimClass` construction for every ordered foundation cell; and
2. one optional `DestroyAnim=` `AnimClass` construction after the intervening destruction work and before destruction particle-system selection and survivor spawning.

The slice includes every RNG draw and synchronous `AnimClass::Middle/Start` consequence owned by those two constructors. It also pins their position relative to the already-present building-center and survivor-smudge work.

This report does **not** claim that all of `BuildingClass::DestructionEffects` is parity-closed. The zero-add pass confirmed additional active mechanisms between or after these two animation arms: the `Explodes=yes` overlay-cell animation loop, stored-resource spill, cost/threshold callback, timer state, destruction particle-system selection, and other non-animation teardown. Those remain explicit parent-row residuals and must be separately investigated, implemented, and criticized before GSI-08.11 or the Action119 receiver chain can close.

## 2. Verdict

The active Rust receiver omits both scoped building animation arms. This is a high-severity mismatch whenever a building dies, including four shipped Action119 rows that can sweep 71 authored live buildings.

The native per-foundation-cell sequence is not equivalent to emitting presentation-only explosions after combat. It consumes Scenario RNG in the exact order

`scatter raw Next -> allocation -> delay RandomRanged(0,3) -> list-index raw Next -> constructor-owned RandomRate if authored -> delay-zero Middle/Start side effects`.

For stock building `Explosion=` animations, `RandomRate` is absent, but four of the five common entries have both `Scorch=yes` and `Crater=yes`. A delay-zero selection therefore can synchronously consume more Scenario RNG and mutate smudge/ore state inside the constructor before the next foundation cell is visited. Delays 1–3 defer `Middle/Start` through normal `AnimClass` AI timing. The animation objects themselves are global scheduler-owned objects with constructor fields `(loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`.

`DestroyAnim=` consumes one raw Scenario draw, uses the building render origin coordinate rather than a generic cell center, and constructs a normal delay-zero scheduler animation. Active retail `DestroyAnim` art uses `NewTheater=yes`, `Shadow=yes`, and `AltPalette=yes`; most rows use `Layer=ground`. It is not safely representable as a short anonymous `WorldEffect`.

## 3. Identity and offset ledger

### 3.1 Building vtable identity

The primary Building vtable begins at `0x007E3EBC`. Its preceding MSVC complete-object locator at `0x007FC360` resolves the type descriptor at `0x00818D60`; the descriptor name is `.?AVBuildingClass@@`. Vtable slot `+0x4EC` at `0x007E43A8` contains `0x004415F0`, proving the receiver callback is `BuildingClass::DestructionEffects`.

Relevant slots:

| Slot | Function | Scoped meaning |
|---|---|---|
| `+0x84` | type getter trampoline | returns the BuildingType/TechnoType object |
| `+0xAC` | `BuildingClass::GetRenderCoords @ 0x00459EF0` | returns `(LocationX-128, LocationY-128, LocationZ)` |
| `+0x108` | `ObjectClass::GetCellFootprintOffsets @ 0x005F5B90` | returns the sentinel-terminated ordered foundation offsets |
| `+0x1E4` | `FUN_00705D70` | selects remap/color scheme, including a custom type palette when present |
| `+0x4EC` | `BuildingClass::DestructionEffects @ 0x004415F0` | destruction callback called by the Building damage wrapper |

### 3.2 Fields used by the scoped arms

| Object | Offset | Evidence-backed meaning |
|---|---:|---|
| `BuildingClass` | `+0x9C/+0xA0/+0xA4` | native object Location X/Y/Z |
| `BuildingClass` | `+0x520` | Building/Techno type pointer |
| `TechnoTypeClass` | vector base `+0x72C`; items `+0x730`; count `+0x73C` | `Explosion=` resolved `AnimTypeClass*` vector |
| `TechnoTypeClass` | vector base `+0x748`; items `+0x74C`; count `+0x758` | `DestroyAnim=` resolved `AnimTypeClass*` vector |
| `TechnoTypeClass` | `+0xDD0` | 32-byte `Palette=` string buffer, not a die-sound name |
| `TechnoTypeClass` | `+0xDF0` | loaded custom palette table pointer derived from `Palette=` |
| `AnimClass` | `+0xD4` | selected remap/color-scheme pointer |
| `AnimClass` | `+0xDC` | copied custom palette name buffer |
| `ScenarioClass` | `+0x218` | Scenario RNG object used by every scoped draw |

`FUN_00717820` proves the palette identity: it tests the first byte at type `+0xDD0`, loads the palette via `FUN_006263D0`, and stores the result at type `+0xDF0`. The older `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` description of `+0xDD0` as a custom die-sound name is wrong.

## 4. Entry and ordering

`BuildingClass::ReceiveDamage @ 0x00442230` obtains the foundation offset pointer through vtable `+0x108` before applying damage. On a native `NowDead` result it performs the Building wrapper cleanup and then calls vtable `+0x4EC`, passing the captured offset pointer into `DestructionEffects`.

Within `DestructionEffects`, the scoped ordering is:

1. clear eight owned damage-fire animation slots;
2. perform gap/sensor/wall/special teardown and building die sound gates;
3. emit the existing large-building center smudge branch;
4. run the ordered per-foundation-cell `Explosion=` loop;
5. run `Explodes=yes`, storage, callback, and timer branches (outside this report but explicitly live/residual);
6. select and construct one `DestroyAnim=`;
7. collect/select a destruction particle system (outside this report and active/residual);
8. force Health to zero;
9. call `BuildingClass::SpawnSurvivors`;
10. call `FootClass::EMPPassengers`.

This order is load-bearing. Existing Rust already dispatches the center-smudge request before its survivor-smudge requests. The new per-cell constructors must be inserted after the center branch and before survivor work; `DestroyAnim` must remain after the intervening native branches and before the particle/survivor tail.

## 5. Ordered foundation authority

The Building damage wrapper passes a sentinel-terminated list of signed `(short dx, short dy)` pairs. Termination is exactly `(0x7FFF, 0x7FFF)`. The destruction loop advances by one pair and preserves list order; it does not sort, deduplicate, or derive a rectangle at death time.

Standard native foundation tables are row-major. Rust's `rules::foundation::foundation_cell_offsets` already preserves the needed row-major special tables, including the `3x3Refinery` hole and zero-sized case. `production::building_base_foundation_cells` is not an acceptable ordered source because its loop/set representation can produce column-major or sorted set order even when occupancy membership is correct.

## 6. Exact per-foundation-cell `Explosion=` mechanism

For each ordered foundation offset `(dx,dy)`:

1. Call Building vtable `+0xAC` (`GetRenderCoords`).
2. If `Explosion` count at type `+0x73C` is non-positive, continue with no coordinate effect and no RNG.
3. Form the foundation-cell center:
   - `X = (((GetRenderX >> 8) + dx) * 256) + 128`
   - `Y = (((GetRenderY >> 8) + dy) * 256) + 128`
   - `Z = LocationZ`
4. Call `FUN_0049F420(radius=0x40, snapFlag=0)`.
5. Allocate `0x1C8` bytes for an `AnimClass`.
6. If allocation fails, continue to the next foundation cell. The scatter draw has already been consumed; the delay and list-index draws are not consumed.
7. Call Scenario `RandomRanged(0,3)` for the constructor delay.
8. Call Scenario raw `Random::Next`; select `Explosion[raw % count]`.
9. Call `AnimClass::Constructor(selected, scatteredCoord, delay, 1, 0x600, 0, 0)`.
10. Complete any synchronous constructor/Middle/Start work before the loop advances to the next foundation pair.

Assembly anchors:

| Address | Fact |
|---:|---|
| `0x0044199C` | call `FUN_0049F420`; allocation follows at `0x004419BC` |
| `0x004419C8` | null allocation branches back to the loop without delay/index draws |
| `0x004419D6` | ECX becomes `Scenario + 0x218` |
| `0x004419DC` | `RandomRanged(0,3)` |
| `0x004419ED` | ECX becomes the same `Scenario + 0x218` again |
| `0x004419FB` | raw `Random::Next` |
| `0x00441A13` | unsigned `DIV` by list count; remainder is the index |
| `0x00441A1F` | `AnimClass::Constructor` |

### 6.1 Scatter helper

`FUN_0049F420 @ 0x0049F420` consumes exactly one raw Scenario `Random::Next`, uses its low byte as direction, transforms `((byte << 8) as short) - 0x3FFF` through native trig/x87 conversion, and applies the requested magnitude. With `snapFlag=0`, it preserves the exact scattered coordinate. If either converted cell is greater than 511, it falls back to the unscattered base coordinate.

Rust already has the exact reusable implementation and 256-direction fixture at `src/sim/combat/inviso_scatter.rs::random_direction_coord`; the building arm must call it with magnitude `0x40` and the Scenario stream.

### 6.2 Constructor and `Start` interleave

`AnimClass::Constructor @ 0x00421EA0` registers the object, initializes rate/lifecycle fields, reveals it, and stores the caller delay. It can consume Scenario RNG for `RandomRate=` during construction. It calls `Middle()` synchronously only when caller delay is zero.

`Middle @ 0x00424CE0` handles `StartSound`/`Report` and calls `Start @ 0x00424F00` when the type's `Start` frame is zero. `Start` owns particle/scorch/crater/start-damage side effects. For both `Scorch` and `Crater`, it calls Scenario `RandomRanged(0,0x7FFFFFFE)` and selects scorch only when the accepted value is below `0x40000000`; the crater arm reduces tiberium by 6 before its smudge placement.

For a nonzero constructor delay, the first Anim AI visit clears the first-AI guard and returns. Later visits decrement the delay. The visit that decrements it to zero calls `Middle()` and returns without frame advancement. Thus delay 1 starts on the second AI visit, delay 2 on the third, and delay 3 on the fourth, subject to native global scheduler insertion order.

Current Rust `spawn_anim_at_world` correctly performs constructor-time `Middle` for delay zero and stores a first-AI guard, but `visit_anim` currently decrements a positive delay and returns even when it reaches zero. It never calls `anim_middle` on that transition and has no scheduler-owned `AnimClass::Start` smudge/ore hook. Both defects are directly load-bearing for this mechanism.

## 7. Exact `DestroyAnim=` mechanism

After the intervening native destruction branches:

1. If type `+0x758` count is non-positive, emit nothing and consume no draw.
2. Call raw Scenario `Random::Next` first.
3. Select `DestroyAnim[raw % count]` from type `+0x74C`.
4. If the selected pointer is null, stop this arm; the selection draw remains consumed.
5. Allocate `0x1C8` bytes. Allocation failure suppresses the animation while retaining the selection draw.
6. On success, call Building `GetRenderCoords` and construct `(selected, renderCoord, delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`.
7. If the Building type has a loaded custom `Palette=` pointer at `+0xDF0`, set the Anim remap/color field through vtable `+0x1E4` and copy the type's `Palette=` buffer from `+0xDD0` to Anim `+0xDC`.

Assembly anchors:

| Address | Fact |
|---:|---|
| `0x00441CB2` | positive count gate at type `+0x758` |
| `0x00441CC4` | ECX is `Scenario + 0x218` |
| `0x00441CCA` | raw `Random::Next` occurs before pointer/null/allocation checks |
| `0x00441CDD` | unsigned `DIV` by vector count |
| `0x00441CEA` | allocate `0x1C8` |
| `0x00441D10` | Building `GetRenderCoords` call |
| `0x00441D1A` | `AnimClass::Constructor` |
| `0x00441D2F..0x00441D66` | optional custom type-palette/remap copy |

The coordinate is the Building render origin `(LocationX-128, LocationY-128, LocationZ)`. For an ordinary Rust structure whose stored `Position` is a foundation-origin cell center (`sub_x=sub_y=128`), the native `DestroyAnim` coordinate decomposes to that same cell with `sub_x=sub_y=0`, not to the cell center and not to the foundation's geometric center.

## 8. Active-retail data census

The authoritative active-YR INI corpus used here is standalone `rulesmd.ini` and `artmd.ini`; Yuri's Revenge does not merge the RA2 base `rules.ini` or `art.ini` into those files. The census also checks 184 extracted `.map`/`.mpr` files under `target/phase3-retail-census/extract` for scenario overrides. A corrected standalone-MD rescan preserves the headline reach and art results below. Its parser follows native INI section syntax through the first `]`, so section headers with trailing comments remain active.

### 8.1 Building type reach

- 403 retail `BuildingTypes` registry rows representing 402 unique names; `NAPSYA` occurs twice.
- 397 registry rows (396 unique BuildingTypes) author `Explosion=`.
- 13 registry rows/types author `DestroyAnim=`.
- 398 registry rows (397 unique BuildingTypes) author at least one of the two.
- Five author neither.
- No extracted retail map overrides `Explosion=`, `DestroyAnim=`, `RandomRate=`, `Scorch=`, `Crater=`, `ForceBigCraters=`, `Foundation=`, or `Palette=` in a way that changes the Action119 target mechanism; the four Action119 maps were checked directly as well.

### 8.2 Active `Explosion=` lists and art

There are ten distinct referenced animation names across retail building `Explosion=` lists:

`TWLT070`, `S_BANG48`, `S_BRNL58`, `S_CLSN58`, `S_TUMU60`, `BRRLEXP1`, `BRRLEXP2`, `MININUKE`, `gtpowexp`, and `tstlexp`.

Stock distribution is dominated by the common five-entry list: 389 registry rows (388 unique BuildingTypes) use `TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60`; three add `gtpowexp`; one uses each barrel list; one uses `MININUKE`; one combines `MININUKE` with the common list; one adds `tstlexp`.

Active art facts:

- `TWLT070`, `S_BANG48`, `S_BRNL58`, and `S_TUMU60`: `Scorch=yes`, `Crater=yes`, a `Report=` sound, `Translucent=yes`, `UseNormalLight=yes`.
- `S_CLSN58`: `Crater=yes`, `Report=Explosion14`, `Translucent=yes`, `UseNormalLight=yes`.
- `BRRLEXP1/2`: `Crater=yes`, `Report=ExplosionBarrel`, `Rate=400`, `Translucent=yes`, `UseNormalLight=yes`.
- `MININUKE`: `Report=ExplosionBarrel`, no scorch/crater flag.
- `gtpowexp` and `tstlexp`: no art section; native `FindOrAllocate` therefore supplies AnimType constructor defaults while their retail SHPs are real assets.
- None of the ten authors `RandomRate=`, `Start=`, `SpawnsParticle=`, `NumParticles=`, `Damage=`, `Warhead=`, `Next=`, or `TrailerAnim=` in active retail art. The omitted `Start` defaults to zero.

Retail asset lookup confirms `gtpowexp.shp` (29 frames) and `tstlexp.shp` (33 frames) in `ra2.mix -> conquer.mix`; the other generic explosion SHPs are also present. Therefore the two missing art sections are not dead references and must receive native default AnimType metadata rather than being rejected as missing configuration.

### 8.3 Active `DestroyAnim=` lists and art

Thirteen BuildingTypes author a one-entry `DestroyAnim=` list. Ten of those types occur in the extracted retail maps: 23 instances across 11 maps. Examples include six `CAEAST01` instances in `sov05umd.map`, `CATRAN03` in `sov07tmd.map`, and civilian `CAWASH*`/`CAPARS*` instances in stock multiplayer maps.

All 13 referenced DestroyAnim art sections use `NewTheater=yes`, `Shadow=yes`, and `AltPalette=yes`; 11 explicitly use `Layer=ground`, while `CACHIG04D` and `CAPR11DM` retain the AnimType layer default. Rates range from 200 to 320. None authors `RandomRate`, `Start`, scorch/crater flags, particle/start-damage keys, `Next`, or trailer keys.

The active theater-specific SHPs exist in retail archives. Examples: `CTEAST01DM.SHP` and `CTTR03DM.SHP` in `ra2md.mix -> isotemmd.mix`; base civilian rows fall back through the `NewTheater` candidate chain to generic `CG...DM.SHP` assets where a theater-specific row is absent. Asset binding must use the established `anim_shp_candidates` theater/generic fallback, not append `.SHP` to the literal AnimType name only.

None of the 13 owning BuildingTypes authors `Palette=` and no extracted map adds it. Therefore the custom type-palette branch at `0x00441D2F..0x00441D66` is an evidence-backed zero-occurrence active-retail exclusion. This does **not** exclude the separate, active AnimType `AltPalette=yes` behavior.

### 8.4 Action119 concrete reach

The seven shipped `Destroy Tag`/Action119 rows are campaign-only ParamType 0 rows. Four rows target Houses with live structures in the authored map state:

| Map / action row | Target | Structures | With `Explosion=` |
|---|---|---:|---:|
| `sov01umd.map` / `096879AC` | Arabs | 1 (`YAPPET`) | 1 |
| `sov06lmd.map` / `0782720C` | YuriCountry | 40 | 40 |
| `sov06lmd.map` / `09A0EC1C` | Americans | 15 | 15 |
| `sov06lmd.map` / `09A0C36C` | Alliance | 15 | 15 |

The other three Action119 rows have zero target-owned authored structures at map load. Runtime ownership changes can still alter reach, but the four positive rows alone prove the mechanism is active and player-visible. None of these 71 authored target structures has `DestroyAnim=`; `DestroyAnim` remains independently active through ordinary destruction of the 23 mapped civilian instances.

## 9. Current Rust status

### Preserved and reusable

- `ObjectType` already parses `Explosion=` and `DestroyAnim=` in document order.
- `rules::foundation::foundation_cell_offsets` supplies the correct ordered special foundation lists.
- `inviso_scatter::random_direction_coord` already matches the native 256-direction/x87 helper and accepts arbitrary magnitude.
- `Simulation::scenario_rng` is the correct stream owner for destruction/smudge/particle work.
- `AnimClassSpawnDescriptor`, `AnimWorldCoord`, and scheduler-owned `AnimStore` can preserve exact constructor rows and world-lepton coordinates.
- `spawn_anim_at_world` already handles registration, stable identity, reveal, rate choice, first-AI guard, delay-zero `Middle`, report/start sound, draw flags, z-adjust, loop count, reverse, and normal serialized/hashable lifetime.
- `try_dispatch_anim_smudge` already implements the active `AnimClass::Start` scorch/crater/ore rules against Scenario RNG.
- Existing building center/survivor smudge requests already use row-major `foundation_cell_offsets` and are placed in the receiver postlude.

### Wrong or missing

- `src/sim/combat/mod.rs` explicitly retains the GSI-08.11 Building animation residual and emits type explosions only for Unit/Aircraft.
- Both `src/sim/world/mod.rs` `ExplosionEffect` consumers erase constructor metadata and produce `WorldEffect { anim_spawn: None }`.
- `ExplosionEffect` is presentation-transient and lacks delay, draw flags, exact world Z, scheduler identity, and an attached start hook.
- Current building destruction emits neither per-cell `Explosion=` nor the single `DestroyAnim=`.
- Current `AnimClass::visit_anim` decrements positive delay but never calls `anim_middle` when the decrement reaches zero.
- Scheduler `anim_middle` only emits start/report sound; it does not invoke the active smudge/ore `AnimClass::Start` subset.
- `scheduler_anim_roots` does not include building destruction animation roots. It also cannot currently synthesize native default AnimType metadata for valid rules references such as `gtpowexp`/`tstlexp` that have a real SHP but no art section.
- The existing `ObjectType::explosion_anims` comment overgeneralizes Unit semantics. Buildings select once per ordered foundation cell with scatter and random delay.

## 10. Implementation handoff

### 10.1 Required deltas

1. Add a building-only destruction animation producer at the concrete Building receiver postlude. Do not reuse the Unit/Aircraft `main_rng` loop.
2. Use the captured pre-damage ordered foundation offsets from the type's exact foundation table.
3. For each foundation pair, execute the exact Scenario draw/constructor order from section 6.
4. Construct real scheduler-owned `AnimClass` records through the existing `AnimStore`/LogicVector registration seam at exact world-lepton coordinates.
5. Preserve the constructor row `(delay=random 0..3, loop=1, flags=0x600, zAdjust=0, reverse=false)` for `Explosion=` and `(0,1,0x600,0,false)` for `DestroyAnim=`.
6. Make delay transition to zero call `Middle` exactly once after the first-AI guard, with no same-visit frame advancement.
7. Bind an `AnimClass::Start` smudge/ore hook to these spawned building Explosion anims. Delay zero must run it synchronously inside the per-cell producer before the next foundation cell. Delays 1–3 must run it only when normal Anim AI reaches `Middle`.
8. Do not globally fire smudge hooks for every existing scheduler anim without proving their prior producer behavior; attach explicit start-work identity or route through a shared exact `Start` implementation whose callers are audited.
9. Add building explosion/destroy roots to scheduler asset binding with native default AnimType creation for rules-resolved references lacking art sections. Retain theater/generic `NewTheater` resolution and Shadow bounds.
10. Use `GetRenderCoords` semantics for `DestroyAnim`: foundation-origin cell at sub `(0,0)`, exact Building Location Z.
11. Keep native custom `Palette=` behavior representable, but active-retail tests may assert the proven zero-occurrence branch. Preserve AnimType `AltPalette` rendering.
12. Remove or narrow only the scoped Building animation residual. Do not erase the adjacent `Explodes`, storage, particle-system, or other `DestructionEffects` residuals.

### 10.2 Architectural constraint

Do not solve this by extending `ExplosionEffect` with enough presentation fields while leaving it outside scheduler state. Native constructs real `AnimClass` objects before survivor spawning. Their stable registration, delay/first-AI state, frame lifetime, report sound, translucent/layer/shadow/AltPalette art, delayed Start side effects, Scenario RNG, snapshot, and replay hash must be owned by simulation.

The combat/receiver boundary must support synchronous delay-zero start work without reordering it after all deaths or after survivor-smudge dispatch. A returned batch that is instantiated only by the later world consumer is insufficient unless it carries and commits the exact inline points.

### 10.3 Acceptance tests

At minimum:

1. `building_explosion_walk_uses_row_major_special_foundation_offsets`
   - Use a `3x3Refinery` or another holed foundation.
   - Assert one spawn per listed pair, in list order, with no hole spawn.
2. `building_explosion_rng_order_is_scatter_delay_index_then_zero_delay_start`
   - Seed values that make the first delay zero and select a both-flags anim.
   - Assert Scenario cursor/state after scatter, ranged delay, modulo index, and Start 50/50/smudge work before cell two.
3. `building_explosion_nonzero_delay_defers_middle_past_first_guard`
   - Delay 1: constructor no Start, first AI only clears guard, second AI calls Middle/Start once and does not advance frame.
   - Cover delay 2/3 visit counts.
4. `building_explosion_allocation_boundary_preserves_native_draws`
   - If Rust exposes an allocation-failure seam, scatter remains consumed and delay/index do not. If allocation failure is structurally impossible, document that Rust memory model exclusion and test normal allocation only.
5. `building_explosion_uses_scenario_not_main_rng`
   - Assert main RNG unchanged and Scenario state changes.
6. `building_explosion_scatter_matches_256_direction_fixture_at_radius_0x40`
   - Reuse the existing exact helper; include boundary fallback.
7. `building_explosion_constructor_row_is_scheduler_owned_and_snapshotted`
   - Verify stable Anim ID, LogicVector membership, descriptor fields, snapshot/restore, and hash.
8. `building_destroy_anim_draw_precedes_alloc_and_uses_render_origin`
   - One Scenario raw draw, `% count`, coordinate sub `(0,0)`, exact Z, delay zero, flags `0x600`.
9. `building_destroy_anim_newtheater_shadow_altpalette_asset_lifecycle`
   - Bind a retail-style `NewTheater/Shadow/AltPalette` row and verify effective frame bounds/palette/layer metadata survive into the scheduler object.
10. `building_effect_missing_art_section_uses_native_animtype_defaults`
    - `gtpowexp`-style referenced type with a real SHP and no art section constructs with default Start/rate/lifecycle instead of failing load.
11. `house_destroy_action_runs_building_explosion_before_survivor_smudges`
    - Drive the Action119 House sweep through concrete damage and assert center smudge -> per-cell constructor/start interleave -> survivor sequence.
12. `building_destroy_anim_palette_branch_active_retail_exclusion`
    - Retail corpus assertion: all 13 DestroyAnim owners have empty `Palette=` after standalone RULESMD plus map overrides, while their AnimTypes retain `AltPalette=yes`.

Focused validation must follow `ENGINE.md`: every Cargo test command carries `--lib`; run the full `cargo test -p vera20k --lib` exactly once after the slice is stable and before its PR is declared ready for `main`.

## 11. Visual composition ledger

| Surface | Native composition | Active-retail consequence |
|---|---|---|
| Foundation explosions | one scheduler Anim per ordered foundation cell, center scattered by radius `0x40`; frame/start timing from the selected AnimType | large buildings produce many staggered, translucent explosions rather than one generic puff |
| Explosion audio | `Report=`/`StartSound=` at `Middle`, immediate only for delay zero | common list reports Explosion09/11/12/14/15 at staggered native start times |
| Explosion smudge/ore | `AnimClass::Start` on the selected type, not at a generic post-death batch boundary | four common entries choose scorch/crater with Scenario RNG; `S_CLSN58` always takes crater path |
| `DestroyAnim` | one scheduler animation at Building render origin; art controls layer, Shadow, NewTheater, AltPalette, and rate | authored civilian buildings visibly collapse after the earlier destruction effects |
| Draw flags | caller supplies `0x600`, z-adjust 0, loop multiplier 1, reverse false | must retain normal AnimClass material/depth behavior, not a bespoke opaque sprite |
| Coordinate Z | exact Building Location Z | bridge/elevated positions must not be reconstructed only from a coarse presentation byte |

## 12. Evidence-backed exclusions and negative facts

- Empty/non-positive `Explosion` count: foundation pairs are still walked, but the scoped arm consumes no RNG and creates no anim.
- Empty/non-positive `DestroyAnim` count: no draw and no anim.
- Active retail has no `RandomRate` on any building `Explosion` or `DestroyAnim` type, so constructor rate choice adds no stock draw. The runtime must still preserve the generic constructor rule.
- Active retail has no nonzero `Start=` on these types; all scoped Start work begins at `Middle`.
- Active retail DestroyAnim owners have no `Palette=` and no map override; the type-custom-palette branch has zero retail occurrences. AnimType `AltPalette=yes` remains active and is not excluded.
- No Action119 target structure authors `DestroyAnim`; this does not make `DestroyAnim` dead because 23 authored retail map instances use it under ordinary destruction.
- Native allocation failure is conditional low-memory behavior. Rust's ordinary allocator does not expose a recoverable null allocation path. This can be an implementation-model exclusion only if explicitly documented; it cannot justify changing normal-path draw order.
- `gtpowexp` and `tstlexp` are not missing assets. They are real retail SHPs with no art sections and therefore use native AnimType defaults.
- `DestroyAnim` does not use the Building cell center or foundation geometric center. It uses `GetRenderCoords`.
- Per-cell `Explosion` selection does not use `main_rng`; every direct draw uses `Scenario+0x218`.
- `production::building_base_foundation_cells` membership equality is insufficient for RNG order.
- A delayed Anim object is still constructed/registered immediately. Delay controls `Middle/Start` and frame progression; it is not permission to postpone object creation.

## 13. Open Questions log — final state

1. `[RESOLVED]` Which Building virtual owns the callback? `+0x4EC -> 0x004415F0`, proven by vtable and RTTI.
2. `[RESOLVED]` Is the foundation captured before or after damage? Before damage via `ReceiveDamage` vtable `+0x108`.
3. `[RESOLVED]` What terminates the footprint list? Exact pair `(0x7FFF,0x7FFF)`.
4. `[RESOLVED]` Is foundation order regenerated/sorted? No; the caller-supplied pair order is consumed directly.
5. `[RESOLVED]` What coordinate seeds each explosion? `GetRenderCoords` cell plus signed foundation offset, recentered to `+128`, with Location Z.
6. `[RESOLVED]` Which scatter helper and magnitude? `FUN_0049F420(0x40,0)`.
7. `[RESOLVED]` Which RNG stream feeds scatter, delay, selection, and Start? Scenario `+0x218`.
8. `[RESOLVED]` What is the direct draw order? scatter -> allocation -> ranged delay -> raw modulo index -> constructor.
9. `[RESOLVED]` What happens on allocation failure? scatter retained; delay/index/constructor skipped for that cell.
10. `[RESOLVED]` What are the per-cell constructor args? `(delay 0..3, loop 1, flags 0x600, zAdjust 0, reverse 0)`.
11. `[RESOLVED]` Can constructor add RNG? Yes for `RandomRate`; active scoped retail types author none.
12. `[RESOLVED]` Does delay zero run Start synchronously? Yes, through Constructor -> Middle -> Start when Start frame is zero.
13. `[RESOLVED]` How does delay one start? First AI clears guard; second AI decrements to zero and calls Middle, then returns.
14. `[RESOLVED]` Are stock Start frames nonzero? No; every scoped active type omits Start and retains zero.
15. `[RESOLVED]` Do stock Start side effects matter? Yes; common explosion art has Report and scorch/crater flags.
16. `[RESOLVED]` When is `DestroyAnim` selected? After the intervening Explodes/storage/callback/timer work, before destruction particle selection and survivors.
17. `[RESOLVED]` What draw does `DestroyAnim` consume? One raw Scenario Next before pointer/allocation checks.
18. `[RESOLVED]` What coordinate does `DestroyAnim` use? Building `GetRenderCoords`, equivalent to origin-cell sub `(0,0)` for ordinary centered structure storage.
19. `[RESOLVED]` What are `+0xDD0/+0xDF0`? Building type `Palette=` name buffer and loaded palette table, not a die sound.
20. `[RESOLVED]` Is that custom palette branch active in retail DestroyAnim owners? No; zero of 13 owner types and zero map overrides author Palette.
21. `[RESOLVED]` Is `AltPalette` therefore dead? No; it is a distinct AnimType art flag and active on all 13 DestroyAnim rows.
22. `[RESOLVED]` Are `gtpowexp`/`tstlexp` dead missing-art names? No; retail SHPs exist and native defaults apply.
23. `[RESOLVED]` Can Action119 actually reach the per-cell mechanism in shipped data? Yes; four rows have 71 authored target structures, all with Explosion lists.
24. `[RESOLVED]` Is DestroyAnim independently active? Yes; 23 authored instances across 11 maps.
25. `[RESOLVED]` Does current Rust have a scheduler primitive suitable for this? Partially: AnimStore/descriptor/world coordinate exist, but delayed Middle and Start-smudge integration are missing.

No material question remains open inside the scoped two-animation mechanism.

## 14. Adversarial review

1. **Could the existing Unit/Aircraft loop be generalized to Buildings?** No. It uses `main_rng`, selects once per list, preserves no scatter/delay/scheduler identity, and uses the object's stored coordinate rather than the ordered foundation/render-origin rules.
2. **Could delay be applied only to rendering while smudges dispatch immediately?** No. Nonzero delay suppresses `Middle/Start`; delay-zero Start can interleave Scenario draws before the next foundation cell.
3. **Could all per-cell draws be performed first, then all Anim objects spawned?** No. Constructor `RandomRate` and delay-zero Start are inside each iteration and may consume/mutate before the next cell.
4. **Could active stock ignore scheduler state because these are short effects?** No. DestroyAnim rows have long theater-specific Shadow SHPs and rate/layer/palette behavior; all rows are real global `AnimClass` objects.
5. **Could missing art sections be rejected as invalid data?** No. `gtpowexp` and `tstlexp` have real active retail SHPs; native allocates default AnimTypes for them.

## 15. Cold spot-checks and zero-add pass

Cold spot-check A re-read assembly at `0x004419D6..0x00441A1F` without relying on the decompiler's local naming. It independently confirmed ECX=`Scenario+0x218` before both RNG calls, `RandomRanged(0,3)` before raw `Next`, unsigned remainder selection, and constructor pushes `reverse=0,zAdjust=0,flags=0x600,loop=1,delay=<range result>`.

Cold spot-check B re-read `0x00441CB2..0x00441D66`. It independently confirmed the DestroyAnim positive-count gate, raw Scenario draw before modulo/pointer/allocation, `GetRenderCoords` before constructor, fixed constructor row, and the conditional type-palette/remap copy.

The final zero-add scan of the complete `0x004415F0` decompile found no additional producer, coordinate, RNG, allocation, lifecycle, palette, or active-data branch inside the scoped `Explosion=`/`DestroyAnim=` mechanisms. It did find the adjacent active `Explodes=yes`, storage, and destruction particle-system branches. They are explicitly retained as parent-row residuals rather than being incorrectly absorbed into or excluded from this report.

## 16. Coverage ledger

| Coverage dimension | Evidence exercised | Result |
|---|---|---|
| Native identity and entry | Building RTTI/vtable, `ReceiveDamage`, complete `DestructionEffects` decompile | Covered; callback identity, caller, captured foundation pointer, and entry order established |
| Native control/data flow | Both animation arms, allocation branches, constructor calls, Scenario RNG, `Middle/Start` | Covered; every scoped branch, draw, coordinate, argument, and synchronous/deferred side effect established |
| Active retail rules/maps/assets | Standalone RULESMD/ARTMD, 184 maps, Action119 owners, MIX/SHP lookup | Covered; positive reach and zero-occurrence exclusions enumerated |
| Current Rust producer/consumer paths | Combat receiver, world effects, AnimStore/scheduler, smudge dispatch, loading roots | Covered; preserved seams, wrong behavior, and missing behavior listed |
| Ordering, persistence, and determinism | Foundation order, inline delay-zero work, delayed AI, stable Anim ownership, snapshot/hash needs | Covered; implementation invariants and acceptance tests specified |
| Visual/audio composition | Layer, Shadow, NewTheater, AltPalette, report timing, smudge/ore, exact Z | Covered in the visual composition ledger |
| Boundary and exclusions | Allocation-model exception, absent custom Palette/RandomRate/Start, adjacent native branches | Covered; scoped exclusions are evidence-backed and adjacent residuals remain open |
| Adversarial completeness | 25 resolved questions, five adversarial challenges, two cold disassembly reads, zero-add pass | Covered; no scoped unresolved or unverified item remains |

## 17. Stale-document corrections

- `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` section 2l should replace “custom die-sound name” for type `+0xDD0` with “`Palette=` string buffer”; type `+0xDF0` is the loaded custom palette table.
- Any wording that calls `DestroyAnim`'s coordinate `center_coord` should be replaced with “Building `GetRenderCoords` = `(LocationX-128, LocationY-128, LocationZ)`.”
- `ObjectType::explosion_anims` documentation must distinguish categories: Unit/Aircraft select once, while Building destruction selects once per ordered foundation cell with scatter and random delay.
- The GSI-08.11 Rust residual's “sub-order ... UNCHECKED” wording is now stale. The exact order is established in section 6.

## 18. Sources

- Live read-only Ghidra decompile/disassembly of all primary/supporting addresses named at the top.
- Vtable/RTTI memory reads at `0x007E3EBC`, `0x007E43A8`, `0x007FC360`, and `0x00818D60`.
- `docs/research/ANIMCLASS_CONSTRUCTOR_MIDDLE_SOUND_TIMING_GHIDRA_REPORT.md`.
- `docs/research/ANIMCLASS_AI_LIFECYCLE_EXACT_SUBSET_RESWARM_20260527.md`.
- `docs/research/SMUDGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md`.
- `docs/research/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md`.
- `docs/research/BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md`, corrected where noted.
- Authoritative active-YR `ini/rulesmd.ini` and `ini/artmd.ini`; base RA2 `ini/rules.ini` and `ini/art.ini` were used only for non-authoritative comparison.
- 184 extracted retail `.map`/`.mpr` files in `target/phase3-retail-census/extract`.
- Read-only retail asset queries through the existing `target/release/asset.exe` tool.
- Current Rust inspection in `src/rules/object_type.rs`, `src/rules/foundation.rs`, `src/rules/art_data.rs`, `src/app/loading/init_helpers.rs`, `src/sim/combat/mod.rs`, `src/sim/combat/inviso_scatter.rs`, `src/sim/combat/smudge_dispatch.rs`, `src/sim/anim_class.rs`, `src/sim/components.rs`, and `src/sim/world/mod.rs`.

**Research verdict:** COMPLETE for the scoped Building `Explosion=` and `DestroyAnim=` emission mechanisms. The parent Building destruction row remains OPEN for the explicitly listed adjacent active branches.
