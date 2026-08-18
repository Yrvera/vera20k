# OccupantAnim AnimClass Lifecycle and DrawIt Depth - Ghidra Research Report

**Address(es):** `TechnoClass::Fire_At @ 0x006FDD50`, `AnimTypeClass::Constructor @ 0x00427530`, `AnimTypeClass::ReadINI @ 0x00427D00`, `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::AI @ 0x00423AC0`, `AnimClass::DrawIt @ 0x00422CA0`, `AnimClass::Middle @ 0x00424CE0`, `AnimClass::Start @ 0x00424F00`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** shot-triggered occupied-building `WeaponType+0x110` `OccupantAnim` construction, default/INI anim metadata consumed by that path, first-tick/lifecycle behavior, and standard non-flat/non-tiled `DrawIt` depth for garrison shot flashes.  
**Non-Scope:** full `AnimClass` special effects outside this render contract: bouncer/meteor physics, damage/scorch/crater area effects, particle chains, sound ownership, RING1 warp ring rendering, global render list traversal, and exact `Tactical__AdjustForZ` internals.  
**Confidence:** High for this slice.  
**Active in YR:** Yes. Standard YR UC weapons in `rulesmd.ini` define `OccupantAnim=` and standard UC anims in `artmd.ini` define stock draw flags.

## 1. Overview

Occupied-building shots use the normal `TechnoClass::Fire_At` muzzle animation path, but occupied shooters replace the ordinary `Anim=` selection with `WeaponType+0x110` (`OccupantAnim`). The resulting object is a normal `AnimClass` with constructor arguments `delay=0`, `loop multiplier=1`, `drawFlags=0x600`, and later `ZAdjust=-200` when the shooter is an occupied building.

For stock `UCFLASH`, `UCCONS`, and `UCINIT`, art data does not define `Rate`, `Start`, `End`, `LoopStart`, `LoopEnd`, `LoopCount`, or `Next`; the native defaults therefore matter. The subtle lifecycle detail is that zero-delay construction calls `Middle()` immediately, but `AnimClass::AI` still has a constructor-set first-AI guard at instance byte `+0x19C` that returns before frame/timer advancement on the first AI call.

## 2. Class Layout / Key Offsets

| Owner | Offset | Meaning in this slice | Verified source |
|-------|--------|-----------------------|-----------------|
| `WeaponTypeClass` | `+0x110` | `OccupantAnim` pointer used for occupied shots | `TechnoClass::Fire_At @ 0x006FDD50`; prior weapon layout docs |
| `AnimTypeClass` | `+0x2B0` | frame delay, stored as `900 / Rate` when `Rate > 0`; default `1` | constructor `0x00427530`, ReadINI `0x00427D00` |
| `AnimTypeClass` | `+0x2B4` | `Start`, default `0` | constructor/ReadINI |
| `AnimTypeClass` | `+0x2B8` | `LoopStart`, default `0` | constructor/ReadINI |
| `AnimTypeClass` | `+0x2BC` | `LoopEnd`, default `0`; only auto-filled from `End` when value is `-1` | constructor/AI |
| `AnimTypeClass` | `+0x2C0` | `End`, default `0`; only auto-filled from SHP frame count when value is `-1` | constructor/AI |
| `AnimTypeClass` | `+0x2C4` | `LoopCount`, default `0` | constructor/ReadINI |
| `AnimTypeClass` | `+0x2C8` | `Next` `AnimTypeClass*`, default `0` | ReadINI |
| `AnimTypeClass` | `+0x344` | `YDrawOffset`, default `0` | constructor/ReadINI/DrawIt |
| `AnimTypeClass` | `+0x348` | type `ZAdjust`, default `0` | constructor/ReadINI |
| `AnimTypeClass` | `+0x35B` | `Tiled`, default false | constructor/DrawIt |
| `AnimTypeClass` | `+0x369` | `Flat`, default false | constructor/DrawIt |
| `AnimTypeClass` | `+0x36A` | `Translucent`; stock UC sets true in `artmd.ini` | ReadINI/DrawIt/INI |
| `AnimTypeClass` | `+0x371` | `Reverse`, default false | constructor/AI |
| `AnimTypeClass` | `+0x372` | `Shadow`, default false; if true, frame count/end handling halves SHP frames | ReadINI/AI/DrawIt |
| `AnimClass` | `+0xAC` | current frame index, before adding type `Start` in DrawIt | constructor/AI/DrawIt |
| `AnimClass` | `+0xC4` | frame step, normally `+1`, reverse path uses `-1` | constructor/AI |
| `AnimClass` | `+0x100` | instance `ZAdjust`; occupied-building shot overwrites to `-200` | constructor/Fire_At/DrawIt |
| `AnimClass` | `+0x184` | initial/random loop delay counter | constructor/AI |
| `AnimClass` | `+0x190` | draw flags stored from constructor argument `0x600` | constructor/DrawIt |
| `AnimClass` | `+0x195` | loop count remaining byte; `0xFF` is not decremented | constructor/AI |
| `AnimClass` | `+0x19B` | destroy/expired flag checked before first-AI guard | constructor/AI |
| `AnimClass` | `+0x19C` | first-AI guard set to `1` by constructor, cleared with immediate return by AI | constructor/AI |

## 3. Verified Control Flow

### Occupied shot creation

`TechnoClass::Fire_At @ 0x006FDD50` first chooses the ordinary weapon animation, including direction-specific `Anim=` handling. If the shooter reports occupied via the virtual call at `+0x400`, it replaces that normal animation with `WeaponType+0x110`. If the selected animation is non-null, `Fire_At` allocates `0x1C8` bytes and calls:

`AnimClass::Constructor(type=OccupantAnim, coords=fire origin, delay=0, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)`.

After construction, the building-specific branch checks `WhatAmI == 6`, calls the occupied-building occupant-count virtual at `+0x408`, and when the count is positive writes instance `AnimClass+0x100 = 0xFFFFFF38` (`-200`). The non-building branch calls `AnimClass::SetOwnerObject(this)`; the building occupied-shot branch does not take that owner attachment call after the `-200` write.

Active in YR: Yes. `rulesmd.ini` stock UC weapons define `OccupantAnim=UCCONS`, `OccupantAnim=UCINIT`, or `OccupantAnim=UCFLASH`, and these are standard occupied-building combat weapons.

### Stock UC art data

Scoped INI scan:

| Section | Source | Keys relevant to this slice |
|---------|--------|-----------------------------|
| `[UCFLASH]` | `ini/artmd.ini` | `Layer=ground`, `Translucent=yes` |
| `[UCCONS]` | `ini/artmd.ini` | `Layer=ground`, `Translucent=yes` |
| `[UCINIT]` | `ini/artmd.ini` | `Layer=ground`, `Translucent=yes` |
| `[UCFLASH]` | `ini/art.ini` fallback | `Layer=ground`, `Translucent=yes` |

No stock UC section in the scoped set defines `Rate`, `Start`, `End`, `LoopStart`, `LoopEnd`, `LoopCount`, `Next`, `YDrawOffset`, `ZAdjust`, `Flat`, `Tiled`, `Shadow`, `PingPong`, or `Reverse`.

### Type defaults and INI conversion

`AnimTypeClass::Constructor @ 0x00427530` sets:

- `Rate/+0x2B0 = 1`.
- `Start/+0x2B4 = 0`.
- `LoopStart/+0x2B8 = 0`.
- `LoopEnd/+0x2BC = 0`.
- `End/+0x2C0 = 0`, not `-1`.
- `LoopCount/+0x2C4 = 0`.
- `Next/+0x2C8 = 0`.
- `YDrawOffset/+0x344 = 0`.
- `ZAdjust/+0x348 = 0`.
- `Layer/+0x364 = 3` before INI override.
- `Flat/+0x369 = false`, `Translucent/+0x36A = false`, `Reverse/+0x371 = false`, `Shadow/+0x372 = false`.
- `ShouldUseCellDrawer/+0x35C = true`.

`AnimTypeClass::ReadINI @ 0x00427D00` only overwrites `Rate` when the key exists. Present `Rate < 1` stores `0`; present `Rate >= 1` stores integer `900 / Rate`. `RandomRate` endpoints are separately converted through the same `900 / value` rule, absent endpoints stay `-1`, negative max is clamped to `0`, and if converted max is less than converted min then min is set to max.

Tiny but important: because stock UC sections omit `End`, `End` remains constructor default `0`. The generic SHP-frame-count fill only runs when `End == -1`; stock UC does not reach that fill from the default constructor state.

### Constructor and first tick

`AnimClass::Constructor @ 0x00421EA0`:

- initializes current frame at `AnimClass+0xAC = 0`;
- initializes frame step `+0xC4 = 1`;
- stores type pointer at `+0xC8`;
- stores neutral z-ish value `+0xFC = 1000` and instance `ZAdjust +0x100 = 0`;
- stores delay `+0x184 = constructor delay`;
- stores draw flags `+0x190 = constructor drawFlags`;
- sets loop byte `+0x195 = 1` before type loop processing;
- sets destroy/expired byte `+0x19B = 0`;
- sets first-AI guard byte `+0x19C = 1`;
- registers the object in `g_AnimClass_Array`;
- sets active byte at object `+0x90` true;
- if `End == -1`, fills `End` from SHP frame count and halves it when `Shadow=yes`;
- if `LoopEnd == -1`, fills `LoopEnd = End`;
- if constructor `zAdjust` argument is zero, copies type `ZAdjust`; otherwise stores constructor `zAdjust`;
- chooses frame delay from type `Rate`, unless `RandomRate` min/max are enabled and ordered;
- stores the chosen delay into timer fields `+0xBC` and `+0xC0`;
- if type `Reverse` or constructor reverse flag is true, starts at `LoopEnd - 1` and makes frame step negative;
- for normal non-bouncer/non-meteor stock UC, calls `ObjectClass::Reveal(coords, 0)`;
- computes loop remaining as `(byte)type.LoopCount * (byte)loopCount`, then clamps any result `< 2` to `1`;
- because `delay=0` for `Fire_At`, calls `AnimClass::Middle()` immediately.

`AnimClass::Middle @ 0x00424CE0` marks the object dirty through vtable `+0x124`, handles start/looping sound setup, detaches sound handles, and calls `AnimClass::Start()` only when the start-sound/frame offset field is zero. It does not advance `CurrentFrame`.

`AnimClass::AI @ 0x00423AC0` checks `+0x19B` before the first-AI guard. Then, when `+0x19C != 0`, it writes `+0x19C = 0` and returns immediately. That return occurs before delay countdown, active-state check, `End`/`LoopEnd` fill, timer check, `CurrentFrame += FrameStep`, loop resolution, `Next`, or deletion.

This corrects an over-strong implementation assumption from prior cadence notes: a newly inserted zero-delay `AnimClass` may be visited by the logic pass, but its first `AI` call does not advance the frame because of the constructor-set `+0x19C` guard.

### Frame advance, loop, end, and next

After the first-AI guard has been cleared, `AnimClass::AI @ 0x00423AC0` proceeds in this order:

1. If delay `+0x184` is nonzero, decrement it. If it reaches zero, call `Middle()` and return; if it stays nonzero, return.
2. If object active byte `+0x90` is false, return.
3. If type `End == -1`, fill it from SHP frame count and halve for `Shadow=yes`.
4. If type `LoopEnd == -1`, set `LoopEnd = End`.
5. Dirty/invalidate via vtable `+0x124`.
6. If bytes `+0x19E` or `+0x11A` are set, return.
7. If `CDTimerClass::GetTimeRemaining()` is nonzero, or reload delay `+0xC0` is zero, set frame-advanced byte `+0xB0 = 0` and return.
8. Otherwise set frame-advanced byte `+0xB0 = 1`, add frame step to current frame, update frame counter/timer fields, and then evaluate start sound, ping-pong, loop/end, random loop delay, `Next`, `MakeInfantry`, and delete.

Loop/end details verified in the decompile:

- `LoopCount=0` with constructor loop multiplier `1` becomes loop remaining byte `1`, not `0`.
- A loop remaining byte of `0xFF` is not decremented at loop boundary.
- When loop remaining is less than `2`, normal forward end testing uses `CurrentFrame < End`.
- When loop remaining is at least `2`, loop testing uses `CurrentFrame < LoopEnd - Start`.
- At loop boundary, if loop remaining is neither `0` nor `0xFF`, it decrements by one.
- If loop remaining remains nonzero, forward non-reverse resets current frame to `LoopStart - Start`; reverse/ping-pong-related paths reset to `LoopEnd`.
- If random loop delay min/max are nonzero, the reset path sets delay `+0x184` to `RandomRanged(min,max)` and returns.
- If loop remaining reaches zero and `Next` is non-null, the same `AnimClass` switches its type pointer to `Next`, clears expired state, sets loop remaining from the next type `LoopCount` byte, resets damage accumulator/timers/rate/current frame, calls `Middle()`, and returns.
- If no `Next` path applies, it marks byte `+0x179 = 1` and calls the destroy vtable at `+0xF8`.

For stock UC defaults, `Rate=1`, `Start=0`, `End=0`, `LoopStart=0`, `LoopEnd=0`, `LoopCount=0`, and constructor loop multiplier `1` mean: initial frame is `0`, first AI call after construction returns due to `+0x19C`, and the first later eligible timer advancement increments current frame to `1`, immediately reaches the `End=0` boundary, decrements loop remaining from `1` to `0`, and deletes if no `Next`. The visible stock flash therefore depends on the draw(s) before that first eligible advancement/deletion, not on raw SHP frame-count expiry.

### DrawIt standard depth

For stock UC, `Tiled=false`, `Flat=false`, `Shadow=false`, `YDrawOffset=0`, and instance `ZAdjust=-200` after `Fire_At`. `AnimClass::DrawIt @ 0x00422CA0` standard branch:

- computes drawn frame as `AnimType.Start + AnimClass.CurrentFrame`;
- starts with draw flags from `AnimClass+0x190`, then translucency/detail logic may OR in translucency bits;
- if bit `0x1` is not set, ORs `0x800`;
- passes `drawFlags | 0x2000` to `CC_Draw_Shape`;
- uses screen position `param_2.x`, `param_2.y + YDrawOffset`; instance `ZAdjust` is not a screen-Y displacement;
- calls vtable `+0x1D0` and `Tactical__AdjustForZ()`;
- passes integer draw depth `YDrawOffset + AnimClass.ZAdjust - Tactical__AdjustForZ() - 2` to `CC_Draw_Shape`;
- passes layer/depth bucket argument `2` in the standard non-flat/non-tiled call.

The flat branch differs by using `... - 3`. The tiled branch differs by starting at `... - 0x32` and decrementing depth each tiled repeat. Shadow drawing, when enabled, uses a separate second draw with shadow flags and depth `-2 - Tactical__AdjustForZ()`, not the normal `YDrawOffset + ZAdjust` expression.

For the stock occupied-shot `UCFLASH`/`UCCONS`/`UCINIT` path, the standard depth expression reduces to:

`-200 - Tactical__AdjustForZ() - 2`

because stock type `YDrawOffset` is `0` and `Fire_At` writes instance `ZAdjust=-200`.

## 4. Edge Cases / Tiny Details Ledger

| Detail | Why it matters | Evidence | Active in YR |
|--------|----------------|----------|--------------|
| `Fire_At` overrides normal `Anim=` with `WeaponType+0x110` only when the occupied virtual returns true. | Prevents using generic muzzle anims for garrison shots. | `0x006FDD50` | Yes |
| Constructor arg `zAdjust=0` does not mean final zero; it means copy type `ZAdjust`, then building branch may overwrite instance `+0x100=-200`. | Prevents screen-position hacks and preserves per-instance depth. | `0x00421EA0`, `0x006FDD50` | Yes |
| Building occupied-shot branch does not call `SetOwnerObject` after the `-200` write. | Do not require owner attachment for palette/depth behavior. | `0x006FDD50` | Yes |
| `End` default is `0`, not `-1`. | Stock UC omitted `End` does not auto-fill from SHP frame count. | `0x00427530`, `0x00427D00`, `0x00423AC0` | Yes |
| `Rate` default is `1`, and absent `Rate=` keeps that exact value. | Stock UC frame delay is native one logic frame before timer gating. | `0x00427530`, `artmd.ini` | Yes |
| Present `Rate=0` stores delay `0`, and AI then refuses to advance because reload delay `+0xC0 == 0`. | Modded static anims must not clamp to one tick if exact generic parity is required. | `0x00427D00`, `0x00423AC0` | Conditional |
| Constructor always sets first-AI guard `+0x19C=1`. | Newly spawned anim may be AI-visited without advancing/deleting. | `0x00421EA0`, `0x00423AC0` | Yes |
| `Middle()` is called immediately for `delay=0` but does not advance frame. | Separates start side effects from frame cadence. | `0x00421EA0`, `0x00424CE0` | Yes |
| `LoopCount=0` becomes one remaining loop due to byte clamp. | A naive `0` means no loops model is wrong. | `0x00421EA0` | Yes |
| `LoopCount=-1` becomes byte `0xFF` and is not decremented at loop boundary. | Infinite-loop modded anims need sentinel semantics. | `0x00421EA0`, `0x00423AC0` | Conditional |
| `Next` reuses the same object rather than spawning a separate object in the verified branch. | Chained garrison flashes need continuous identity/timing behavior. | `0x00423AC0` | Conditional |
| Standard `DrawIt` uses `ZAdjust` only in integer depth, not screen coordinates. | Prevents shot flashes from visually shifting from the fire port. | `0x00422CA0` | Yes |
| Standard depth subtracts a hard `2`; flat subtracts `3`; tiled starts at `0x32`. | Sorting against buildings/walls can change by one/tens of depth units. | `0x00422CA0` | Yes, standard stock |
| `DrawIt` ORs `0x2000` into draw flags before `CC_Draw_Shape`. | Shape flags are not just constructor `0x600`. | `0x00422CA0` | Yes |
| `Translucent=yes` in stock UC affects draw flags/alpha-style path before draw. | Stock flashes are not opaque palette-only sprites. | `artmd.ini`, `0x00427D00`, `0x00422CA0` | Yes |
| `Shadow=yes` changes both `End` filling and draw behavior, with a separate shadow draw depth. | Modded `OccupantAnim` with shadows cannot be modeled as raw frame count. | `0x00427D00`, `0x00423AC0`, `0x00422CA0` | Conditional |

## 5. Rust Comparison

Current Rust has an app-layer garrison flash path rather than a generic `AnimClass` object for occupied-shot `OccupantAnim`:

- `src/app_sim_tick.rs:176..200` measures fixed ticks elapsed and calls `tick_garrison_muzzle_flashes` once after the batched fixed simulation update.
- `src/app_sim_tick.rs:266..270` clears `state.pending_fire_effects` at the start of each fixed step. Because `tick_garrison_muzzle_flashes` consumes pending fire effects once after the loop, earlier fixed-step shot events can be overwritten by later fixed steps in the same render frame.
- `src/app_building_anim.rs:702..764` spawns `GarrisonMuzzleFlash` objects from `pending_fire_effects`, then advances all flashes by the batched elapsed fixed ticks.
- `src/app_building_anim.rs:767..781` advances by elapsed milliseconds and deletes when `frame >= total_frames`.
- `src/app_building_anim.rs:792..795` explicitly notes that `End`, `Loop`, `Next`, and `Shadow` are not represented yet.
- `src/sim/components.rs:675..701` stores only `shp_name`, position, `z_adjust`, `frame`, `total_frames`, `rate_logic_frames`, and elapsed milliseconds for garrison flashes.
- `src/rules/art_data.rs:203..212` implements the verified `Rate -> 900 / Rate` conversion and `Rate <= 0 -> 0`.
- `src/app_instances/overlays.rs:508..517` keeps screen position independent of `z_adjust`, matching the native separation of position and sort depth.
- `src/app_instances/overlays.rs:531..546` applies `z_adjust` as a normalized float bias around `1000`, not the verified integer `YDrawOffset + ZAdjust - Tactical__AdjustForZ() - 2` expression.

Net deltas:

1. Batched retention remains a direct risk: only the final fixed step's pending fire events are visible to the post-loop flash spawner.
2. Generic lifecycle is still missing: stock UC deletion by raw SHP frame count is not native; native stock UC has `End=0`, first-AI guard, one eligible timer advance, and deletion through generic loop/end logic.
3. Same-pass frame advancement should not be used as the implementation target for this path. Native may first visit the new object in the same pass, but the constructor-set `+0x19C` guard returns before advancement.
4. Depth is directionally fixed versus older screen-shift behavior, but exact integer ordering against buildings/walls is unproven because current Rust maps the native integer depth expression into a float bias.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| `TechnoClass::Fire_At` occupied `OccupantAnim` selection | verified | `0x006FDD50`; `rulesmd.ini` `OccupantAnim=` lines | none for this slice |
| Building occupied-shot `ZAdjust=-200` write | verified | `0x006FDD50` | none for this slice |
| `SetOwnerObject` exclusion for building branch | verified | `0x006FDD50` | none for this slice |
| Stock UC INI keys | verified | `ini/artmd.ini`, `ini/art.ini` scoped scan | actual SHP bitmap frame contents not dumped in this slice |
| `AnimTypeClass` constructor defaults | verified | `0x00427530` | none for listed fields |
| `AnimTypeClass::ReadINI` key conversion | verified | `0x00427D00` | not every unrelated AnimType key explained |
| `AnimClass::Constructor` occupied-shot relevant fields | verified | `0x00421EA0` | bouncer/meteor branches intentionally non-scope |
| `AnimClass::Middle` zero-delay behavior | verified | `0x00424CE0` | sound side effects not implementation-scoped here |
| `AnimClass::AI` first-AI guard | verified | `0x00423AC0` | exact CDTimer internals touched only through call-site effects |
| `AnimClass::AI` loop/end/next ordering | verified | `0x00423AC0` | MakeInfantry side path non-scope |
| `AnimClass::DrawIt` standard non-flat/non-tiled depth | verified | `0x00422CA0` | exact global render traversal and z-buffer comparator outside scope |
| `AnimClass::DrawIt` flat/tiled/shadow contrast | touched-not-exhausted | `0x00422CA0` | formulas captured, but not full visual contract for every special branch |
| `Tactical__AdjustForZ` internals | deferred | helper called in `0x00422CA0` | investigate if exact render integer depth needs its internal formula |
| Current Rust batched event retention | verified | `src/app_sim_tick.rs:176..200`, `266..270`, `320..324` | implementation fix and tests |
| Current Rust generic lifecycle gap | verified | `src/app_building_anim.rs:702..797`, `src/sim/components.rs:675..701` | implementation fix and tests |
| Current Rust depth approximation | verified | `src/app_instances/overlays.rs:508..546` | exact render-depth contract/test against building/wall ordering |

## 7. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is shot `OccupantAnim` active in normal YR garrison fire? -> Yes; occupied `Fire_At` replaces normal weapon anim with `WeaponType+0x110`, and stock UC weapons define `OccupantAnim`.` (evidence: `0x006FDD50`; `rulesmd.ini`)
- `[RESOLVED] OQ-02 - What constructor arguments are used? -> `delay=0`, `loopCount=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.` (evidence: `0x006FDD50`, `0x00421EA0`)
- `[RESOLVED] OQ-03 - Does occupied building fire write `ZAdjust=-200`? -> Yes, after construction when occupant count is positive.` (evidence: `0x006FDD50`)
- `[RESOLVED] OQ-04 - Does the building branch attach owner object? -> No observed post-constructor `SetOwnerObject(this)` call on the building branch; the non-building branch has that call.` (evidence: `0x006FDD50`)
- `[RESOLVED] OQ-05 - What are stock UC art metadata keys? -> only `Layer=ground` and `Translucent=yes` in scoped stock sections; lifecycle keys omitted.` (evidence: `ini/artmd.ini`, `ini/art.ini`)
- `[RESOLVED] OQ-06 - What is default `End` for omitted stock UC `End=`? -> `0`, not `-1`; SHP-frame-count fill only runs for `-1`.` (evidence: `0x00427530`, `0x00423AC0`)
- `[RESOLVED] OQ-07 - Does zero-delay construction advance the first frame? -> No; constructor calls `Middle()`, but `Middle()` does not increment current frame.` (evidence: `0x00421EA0`, `0x00424CE0`)
- `[RESOLVED] OQ-08 - Can first AI after construction advance the frame? -> No; constructor sets `+0x19C=1`, and AI clears it and returns before timer/frame logic.` (evidence: `0x00421EA0`, `0x00423AC0`)
- `[RESOLVED] OQ-09 - What happens to `LoopCount=0`? -> Constructor converts it to one remaining loop byte.` (evidence: `0x00421EA0`)
- `[RESOLVED] OQ-10 - Is `LoopCount=-1` a special infinite sentinel? -> Yes at the loop boundary, byte `0xFF` is not decremented.` (evidence: `0x00423AC0`)
- `[RESOLVED] OQ-11 - Does `Next=` spawn or switch? -> In the verified AI branch, it switches the same object's type pointer and reinitializes timing/current-frame fields.` (evidence: `0x00423AC0`)
- `[RESOLVED] OQ-12 - Does `ZAdjust` move screen position? -> No for standard `DrawIt`; it participates in draw-depth expression, while screen Y uses `param_2.y + YDrawOffset`.` (evidence: `0x00422CA0`)
- `[RESOLVED] OQ-13 - What is the standard non-flat/non-tiled depth expression? -> `YDrawOffset + AnimClass.ZAdjust - Tactical__AdjustForZ() - 2`.` (evidence: `0x00422CA0`)
- `[DEFERRED] OQ-14 - What exactly does `Tactical__AdjustForZ()` compute for every building/wall overlap?` (category: requires-different-system-context; reason: this slice verified the caller's depth expression but not the helper internals or global sort comparator; next-step-if-pursued: focused render-depth contract covering `Tactical__AdjustForZ`, object/building/wall draw calls, and final sort/z-buffer order)
- `[DEFERRED] OQ-15 - What are the exact stock UC SHP frame dimensions/contents?` (category: out-of-scope; reason: lifecycle/depth contract did not require bitmap dumping; next-step-if-pursued: asset dump for `UCFLASH`, `UCCONS`, `UCINIT` frame counts and offsets)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|-------------------|----------|--------------------|-----------------------|--------------------------------|---------------------|------------------|
| Every occupied shot with non-null `WeaponType+0x110` constructs one normal `AnimClass` object immediately in the shot tick. | `0x006FDD50`; `rulesmd.ini` `OccupantAnim=` | batched events can be lost because `pending_fire_effects` is cleared each fixed step and consumed once after the batch | `src/app_sim_tick.rs`, `src/app_building_anim.rs` | Retain/spawn every garrison `SimFireEvent` across batched fixed ticks in tick order. | Two fixed ticks in one render frame, each with one garrison shot, produce two flash instances/events in deterministic order. | Do not keep only the final fixed step's pending fire events. |
| Zero-delay constructor calls `Middle()` immediately, but first `AI` clears `+0x19C` and returns before frame advancement. | `0x00421EA0`, `0x00424CE0`, `0x00423AC0` | current comments/flow model still imply same-call advance is native target; current app state has no first-AI guard | `src/app_building_anim.rs`, `src/sim/components.rs`, future generic anim model | Add/native-model a first-AI guard or equivalent lifecycle phase before timer/frame advancement. | A newly spawned stock `UCFLASH` visited once by the app anim update remains on frame `0` after that first visit and is only eligible to advance on the next native-equivalent AI. | Do not advance a just-constructed zero-delay `OccupantAnim` in the same logical AI visit. |
| Stock UC defaults are `Rate=1`, `Start=0`, `End=0`, `LoopStart=0`, `LoopEnd=0`, `LoopCount=0`, `Next=null`; omitted `End` does not auto-fill from SHP count. | `0x00427530`, `0x00427D00`, `0x00423AC0`, `artmd.ini` | current deletion uses raw `total_frames` and rate-derived cadence | `src/app_building_anim.rs`, `src/sim/components.rs`, `src/rules/art_data.rs` | Use generic `AnimClass` end/loop/next fields for garrison `OccupantAnim`; SHP count only fills when native `End == -1`. | Stock `UCFLASH` with omitted lifecycle keys follows first-guard then generic `End=0` deletion behavior, not `frame < total_frames` lifetime. | Do not assume omitted `End` means play all SHP frames. |
| `Rate=` conversion is integer `900 / Rate`; `Rate <= 0` stores zero, and AI with reload delay zero does not advance. | `0x00427D00`, `0x00423AC0` | Rust conversion exists, but garrison delay wrapper clamps `rate_logic_frames.max(1)` | `src/rules/art_data.rs`, `src/app_building_anim.rs`, generic anim tick | Preserve zero-delay-static semantics when exact generic anim lifecycle is implemented. | Modded `OccupantAnim=MYUC` with `Rate=0` does not advance frames under generic AI. | Do not clamp `Rate=0` to one tick for the generic path. |
| `LoopCount=0` becomes one remaining loop; `0xFF` is an undecremented sentinel; `Next` switches the same object. | `0x00421EA0`, `0x00423AC0` | current `GarrisonMuzzleFlash` has no loop remaining byte, random loop delay, or `Next` type pointer | `src/sim/components.rs`, `src/app_building_anim.rs`, rules art metadata | Represent loop byte semantics, boundary resets, optional random loop delay, and same-object `Next` transition. | Modded `MYUC` with `LoopStart=0`, `LoopEnd=3`, `LoopCount=2`, `Next=MYNEXT` advances/resets/switches per native order. | Do not model `Next` as an unrelated spawn that loses timing/current object state. |
| Occupied building shot standard draw depth is `YDrawOffset + ZAdjust - Tactical__AdjustForZ() - 2`, with `ZAdjust=-200`; screen position is not shifted by `ZAdjust`. | `0x006FDD50`, `0x00422CA0`, `artmd.ini` | Rust preserves unshifted screen position, but depth is a normalized float bias around neutral `1000` | `src/app_instances/overlays.rs`, render sort/depth model | Match/prove integer native depth ordering against building and wall draws, using the exact standard branch expression for stock UC. | Occupied `UCFLASH` overlapping a building body and adjacent wall sorts identically to native for `ZAdjust=-200`. | Do not encode `ZAdjust` as screen-Y displacement or an arbitrary float nudge without proving ordering. |
| Stock UC has `Translucent=yes`; `DrawIt` also ORs `0x2000` into flags and ORs `0x800` when low bit is unset. | `artmd.ini`, `0x00422CA0` | app sprite instance uses alpha/tint pipeline, not verified native draw flags | `src/app_instances/overlays.rs`, sprite material/render flags | Preserve translucent visual treatment and any final draw-flag-equivalent behavior needed for pixel parity. | Stock UC flash over dark/light cells matches native translucency and lighting frame-by-frame. | Do not treat stock UC as an opaque ordinary sprite. |

### Stale Docs / Follow-up Docs

- `docs/research/traces/GARRISON_SHOT_CADENCE_FIRST_ADVANCE_POSTFIX_TRACE.md` and comments derived from it should replace any wording that says a newly inserted `AnimClass` can advance on its first same-pass AI visit with: "A newly constructed zero-delay `AnimClass` may be visited by AI after insertion, but constructor byte `+0x19C` is set to `1`; the first `AnimClass::AI` call clears that byte and returns before timer/frame advancement. `Middle()` is still called immediately by the zero-delay constructor."
- `docs/research/CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md` line-level handoff about "generic SHP/default end behavior" should be refined with: "Constructor default `End` is `0`; the SHP-frame-count fill is gated by `End == -1`, so stock UC sections that omit `End=` do not automatically play to SHP frame count through that fill path."
- Existing implementation comments in `src/app_building_anim.rs` are not patched by this research pass, but the future implementation should update them while fixing lifecycle parity.

## Sources

- Ghidra decompiled: `TechnoClass::Fire_At @ 0x006FDD50`.
- Ghidra decompiled: `AnimTypeClass::Constructor @ 0x00427530`.
- Ghidra decompiled: `AnimTypeClass::ReadINI @ 0x00427D00`.
- Ghidra decompiled: `AnimClass::Constructor @ 0x00421EA0`.
- Ghidra decompiled: `AnimClass::AI @ 0x00423AC0`.
- Ghidra decompiled: `AnimClass::DrawIt @ 0x00422CA0`.
- Ghidra decompiled: `AnimClass::Middle @ 0x00424CE0`.
- Ghidra decompiled: `AnimClass::Start @ 0x00424F00`.
- INI checked: `ini/artmd.ini`, `ini/art.ini`, `ini/rulesmd.ini`, `ini/rules.ini`.
- Prior docs checked: `docs/research/CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md`, `docs/research/GARRISON_VISUAL_OCCUPANTANIM_RESWARM_20260527.md`, `docs/research/traces/GARRISON_SHOT_CADENCE_FIRST_ADVANCE_POSTFIX_TRACE.md`, `docs/research/traces/GARRISON_SHOT_Z_ADJUST_DEPTH_POSTFIX_TRACE.md`, `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `docs/contracts/2026-05-27-building-garrison-reswarm-implementation-contract.md`.
- Rust scanned: `src/app_sim_tick.rs`, `src/app_building_anim.rs`, `src/app_instances/overlays.rs`, `src/sim/components.rs`, `src/rules/art_data.rs`.
