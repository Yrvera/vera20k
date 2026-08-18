# AnimClass AI Lifecycle Exact Subset - Reswarm Report

**Address(es):** `AnimClass::AI @ 0x00423AC0`, `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::Middle @ 0x00424CE0`, `AnimTypeClass::Constructor @ 0x00427530`, `AnimTypeClass::ReadINI @ 0x00427D00`, `CCINIClass::ReadMinMax @ 0x00529880`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact lifecycle subset ordinary garrison `OccupantAnim` needs: first-AI guard, delay-zero/Middle ordering, `Rate` and `Rate=0`, `Start`, `End` including `End=0` and `End=-1`, `LoopStart`, `LoopEnd`, loop byte/sentinel, `Next`, `RandomRate`, `RandomLoopDelay`, and carry fields for `PingPong`/`Reverse`.  
**Non-Scope:** global `AnimClass` object pool identity/order, `DrawIt` depth/order, bouncer/meteor physics, damage/scorch/crater effects, particles, sound fidelity, `MakeInfantry`, save/load serialization, and full render traversal.  
**Confidence:** High for the claimed lifecycle slice.  
**Active in YR:** Yes for ordinary garrison `OccupantAnim` because stock YR occupied weapons define `OccupantAnim=` and the shot path constructs a normal `AnimClass`. Conditional for modded lifecycle keys that stock UC anims omit.

## Working Notes

Target question: what exact app-layer `AnimRuntime` state machine is needed for ordinary occupied-building shot `OccupantAnim`, and does lifecycle evidence by itself force a full app-layer `AnimClass` object pool?

Non-goals: do not decide global render pool/order, do not redo occupied-shot weapon selection unless a contradiction appears, do not implement Rust.

Evidence needed to mark COMPLETE: Ghidra decompile for constructor, AI, Middle, AnimType defaults/readers, ReadMinMax; executable-range disassembly coverage for the primary AI function; INI proof that stock UC anims are active and omit lifecycle keys; Rust scan of `GarrisonMuzzleFlash` and art metadata; implementation handoff with tests.

Stop conditions: stop after the lifecycle state machine is implementation-ready and remaining questions are only object-pool/render-order or non-garrison `AnimClass` branches.

## 1. Overview

Ordinary occupied-building shot flashes use a normal active-YR `AnimClass` lifecycle. For stock `UCFLASH`/`UCCONS`/`UCINIT`, the lifecycle keys are mostly omitted, so the constructor defaults are the spec: `Rate=1`, `Start=0`, `End=0`, `LoopStart=0`, `LoopEnd=0`, `LoopCount=0`, and `Next=null`.

The most important implementation detail is the first-AI guard. `delay=0` construction calls `Middle()` immediately, but `AnimClass::AI` still clears byte `+0x19C` and returns before delay countdown, timer checks, frame advance, loop/end, `Next`, or deletion.

## 2. Lifecycle Fields Needed by an App-Layer Runtime

| Owner | Offset | Field | Native value/source | Active in YR |
|---|---:|---|---|---|
| `AnimClass` | `+0xAC` | `CurrentFrame` | constructor sets `0`; AI adds `FrameStep`; `DrawIt` later adds type `Start` | Yes |
| `AnimClass` | `+0xB0` | frame-advanced byte | AI writes `0` when timer blocks, `1` when it advances | Yes |
| `AnimClass` | `+0xBC` | frame delay/timer reload-ish field | set from `Rate` or `RandomRate`; zero blocks advancement | Yes |
| `AnimClass` | `+0xC0` | frame delay reload | same chosen delay; AI refuses advance when zero | Yes |
| `AnimClass` | `+0xC4` | `FrameStep` | constructor `+1`; reverse sets `-1`; ping-pong flips sign | Yes/conditional |
| `AnimClass` | `+0xC8` | `Type` pointer | constructor type; `Next` mutates this in-place | Yes/conditional |
| `AnimClass` | `+0x184` | `Delay` | constructor delay; garrison shot uses `0`; loop reset may set random delay | Yes/conditional |
| `AnimClass` | `+0x195` | loop count remaining byte | `(byte)type.LoopCount * (byte)ctorLoop`, then clamp `<2` to `1`; `0xFF` sentinel | Yes/conditional |
| `AnimClass` | `+0x19B` | expired/inactive flag | checked before first-AI guard; set before destroy paths | Yes |
| `AnimClass` | `+0x19C` | first-AI guard | constructor sets `1`; AI clears and returns | Yes |
| `AnimTypeClass` | `+0x2B0` | `Rate` | default `1`; `ReadINI Rate>0` stores `900/Rate`, `Rate<=0` stores `0` | Yes |
| `AnimTypeClass` | `+0x2B4` | `Start` | default/read int; used as draw/frame base and loop math | Yes/conditional |
| `AnimTypeClass` | `+0x2B8` | `LoopStart` | default/read int; reset uses `LoopStart - Start` | Yes/conditional |
| `AnimTypeClass` | `+0x2BC` | `LoopEnd` | default `0`; only filled from `End` if `-1` | Yes/conditional |
| `AnimTypeClass` | `+0x2C0` | `End` | default `0`; only filled from SHP frame count if `-1`; `Shadow` halves fill | Yes/conditional |
| `AnimTypeClass` | `+0x2C4` | `LoopCount` | default/read int, consumed as byte | Yes/conditional |
| `AnimTypeClass` | `+0x2C8` | `Next` | default null; read via `AnimTypeClass::FindOrCreate` | Conditional |
| `AnimTypeClass` | `+0x2DC/+0x2E0` | `RandomLoopDelay` min/max | default `{0,0}`; parsed by `ReadMinMax`; nonzero range sets `Delay` after loop reset | Conditional |
| `AnimTypeClass` | `+0x2E4/+0x2E8` | `RandomRate` min/max | default read seed `{-1,-1}`; converted with `900/x`, clamped max >= 0, min <= max | Conditional |
| `AnimTypeClass` | `+0x370` | `PingPong` | default false; flips `FrameStep` at boundary and returns | Conditional |
| `AnimTypeClass` | `+0x371` and `AnimClass+0x120` | type/constructor reverse | default false; constructor reverse arg for garrison is false | Conditional |
| `AnimTypeClass` | `+0x372` | `Shadow` | default false; changes `End` fill and read-time halving/doubling | Conditional |

## 3. Core State Machine

### Construction

`AnimClass::Constructor @ 0x00421EA0` initializes frame/lifecycle state, registers the object, and then type-initializes it. Load-bearing order:

1. `CurrentFrame=0`, `FrameStep=1`, `Type=param_2`, `Delay=param_4`, draw flags from param_6.
2. `LoopCountRemaining` byte is first set to `1`.
3. expired byte `+0x19B=0`; first-AI guard `+0x19C=1`.
4. Object is registered in `g_AnimClass_Array` and active byte `+0x90` is set.
5. If `type.End == -1`, fill `End` from SHP header frame count; if `Shadow`, halve.
6. If `type.LoopEnd == -1`, set `LoopEnd = End`.
7. Choose frame delay from `Rate` or enabled `RandomRate`.
8. If type `Reverse` or constructor reverse is set, set `CurrentFrame = LoopEnd - 1` and negate `FrameStep`.
9. Compute `LoopCountRemaining = (byte)type.LoopCount * (byte)max(ctorLoop, 1)`, then if result `<2`, clamp to `1`.
10. If `Delay==0`, call `AnimClass::Middle()`.

Active in YR: Yes for garrison shot construction. The parent-settled occupied-shot path calls constructor with `delay=0`, loop multiplier `1`, draw flags `0x600`, z-adjust arg `0`, reverse arg `0`.

### Middle

`AnimClass::Middle @ 0x00424CE0` marks visibility/dirty state, handles start/report sound plumbing, may call `AnimClass::Start()` when no SHP pointer is loaded, and may run unrelated tiberium-chain side effects. It does not change `CurrentFrame` or clear the first-AI guard.

Active in YR: Yes. Ordinary garrison `OccupantAnim` with delay zero reaches `Middle()` immediately from constructor, but stock UC does not use the tiberium-chain branch.

### AI tick order

`AnimClass::AI @ 0x00423AC0` is large, but the ordinary non-bouncer garrison lifecycle path reaches this order:

1. Special visibility and owner/sound checks run first.
2. Trailer/bouncer/overlay special branches are outside this slice for stock UC.
3. If expired byte `+0x19B` is set, jump to destroy.
4. If first-AI guard `+0x19C` is set, clear it and return.
5. If `Delay > 0`, decrement. If it becomes zero, call `Middle()` and return; otherwise return.
6. If active byte `+0x90` is false, return.
7. If `End == -1`, fill it from SHP frame count and halve for `Shadow`.
8. If `LoopEnd == -1`, set `LoopEnd = End`.
9. Mark visibility/dirty via vtable `+0x124`.
10. If pause bytes `+0x19E` or `+0x11A` are set, return.
11. If timer remaining is nonzero or frame-delay reload `+0xC0` is zero, write frame-advanced byte `0` and return.
12. Otherwise write frame-advanced byte `1`, add `FrameStep` to `CurrentFrame`, stamp current frame counter, reload delay, then evaluate per-frame side effects, ping-pong, loop/end, `Next`, and destroy.

Active in YR: Yes. The first-AI guard and timer gating apply to every ordinary garrison `OccupantAnim` because it is a normal `AnimClass`.

### Boundary, loop, and next

After a successful frame advance, native tests boundaries with two main modes:

- If loop byte `<2`, forward end test uses `CurrentFrame < End`.
- If loop byte `>=2`, forward loop test uses `CurrentFrame < LoopEnd - Start`.

If boundary is reached:

1. If loop byte is neither `0` nor `0xFF`, decrement it by one.
2. If loop byte remains nonzero, reset `CurrentFrame` to `LoopStart - Start` for forward non-reverse, or to `LoopEnd` for reverse/constructor-reverse paths. If either `RandomLoopDelay` endpoint is nonzero, set `Delay = RandomRanged(min,max)` and return.
3. If loop byte is zero and `Next != null`, mutate the same object: `Type = Next`, apply next type `End == -1` / `LoopEnd == -1` fills, clear expired state, set loop byte from next type `LoopCount`, clear side state, choose rate from next type `Rate` or enabled `RandomRate`, reload timer fields, set `CurrentFrame = next.Start`, call `Middle()`, and return.
4. If no `Next`, set deletion marker byte `+0x179=1` and call destroy vtable `+0xF8`.

Active in YR: Yes for the generic object created by garrison shots. Conditional for `LoopCount`, `RandomLoopDelay`, and `Next` content because stock UC sections omit those keys, but mods can set them and the runtime path is live.

### PingPong and reverse carry fields

`PingPong` is checked after frame advance and before ordinary loop/end resolution. If set, native flips `FrameStep` and returns at the boundary instead of immediately falling through to loop/end. With loop byte `<2`, boundary is `CurrentFrame >= End || CurrentFrame == 0`. With loop byte `>=2`, boundary is `CurrentFrame >= LoopEnd - Start || CurrentFrame == Start`.

Reverse exists both on the type (`AnimType+0x371`) and instance (`AnimClass+0x120`, constructor reverse arg). Garrison shot constructor reverse arg is false, but a generic runtime must carry the instance bit because AI checks it alongside type reverse.

## 4. INI Keys

| Key | Stock UC value | Native default / conversion | Active in YR |
|---|---|---|---|
| `Rate` | omitted | constructor default `1`; if present `Rate>0` -> `900/Rate`, `Rate<=0` -> `0` | Yes |
| `Start` | omitted | `0` | Yes/conditional |
| `End` | omitted | `0`; SHP fill only if value is `-1` | Yes/conditional |
| `LoopStart` | omitted | `0` | Conditional |
| `LoopEnd` | omitted | `0`; fill from `End` only if value is `-1` | Conditional |
| `LoopCount` | omitted | `0`, then constructor loop byte becomes `1` | Conditional |
| `Next` | omitted | null | Conditional |
| `RandomLoopDelay` | omitted | `{0,0}` | Conditional |
| `RandomRate` | omitted | absent read defaults `{-1,-1}`, then max clamped to `0` and min <= max; random chosen only if either stored endpoint nonzero and min<=max in constructor | Conditional |
| `PingPong` | omitted | false | Conditional |
| `Reverse` | omitted | false | Conditional |
| `Shadow` | omitted | false; if toggled by INI, read path adjusts `End` and clamps `LoopEnd` | Conditional |

Scoped INI proof:

- `ini/artmd.ini:16131..16141` defines `[UCFLASH]`, `[UCCONS]`, `[UCINIT]` with only `Layer=ground` and `Translucent=yes`.
- `ini/art.ini:11583..11586` fallback `[UCFLASH]` likewise only sets `Layer=ground` and `Translucent=yes`.
- `ini/rulesmd.ini:22868..22964`, `24686..24696`, and `25290` define stock `OccupantAnim=UCCONS`, `UCINIT`, or `UCFLASH`.

## 5. Current Rust Implementation Status

Current Rust app-layer garrison flashes are lifecycle-incomplete:

- `src/sim/components.rs` `GarrisonMuzzleFlash` carries `frame`, `total_frames`, `rate_logic_frames`, and elapsed milliseconds, but no first-AI guard, native `End`, `LoopStart`, `LoopEnd`, loop byte, `Next`, random delay/rate, ping-pong, reverse, shadow, or type pointer transition.
- `src/app_building_anim.rs::tick_garrison_muzzle_flashes` spawns one app flash per pending event and then advances all flashes by batched elapsed fixed ticks.
- `src/app_building_anim.rs::advance_garrison_muzzle_flash` advances until `frame >= total_frames`; native lifecycle does not use raw SHP count unless `End == -1`.
- `src/app_building_anim.rs::garrison_occupant_anim_delay_ms` clamps `rate_logic_frames.max(1)`, which is wrong for exact `Rate=0` semantics once the generic path is implemented.
- `src/rules/art_data.rs` already has verified `Rate -> 900 / Rate` and `Rate<=0 -> 0`, but generic anim metadata is not represented as a reusable `AnimType` lifecycle record.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AnimClass::Constructor` lifecycle init | verified | decompile `0x00421EA0` | none for listed fields |
| first-AI guard | verified | constructor `0x00421EA0`; AI decompile `0x00423AC0`; AI disassembly range `0x00423AC0..0x00424B3F` | none |
| `Middle()` no frame advance | verified | decompile `0x00424CE0` | sound/particle details out of scope |
| `Rate` and `Rate=0` | verified | `AnimTypeClass::ReadINI @ 0x00427D00`, AI timer branch `0x00423AC0` | exact `CDTimerClass` internals out of scope because call-site effect is enough |
| `End=0` vs `End=-1` fill | verified | `AnimTypeClass::Constructor @ 0x00427530`; constructor and AI fills | none |
| `LoopStart`/`LoopEnd`/`LoopCount` boundary behavior | verified | `AnimClass::AI @ 0x00423AC0` | none for ordinary non-bouncer branch |
| `0xFF` loop sentinel | verified | loop decrement branch in `AnimClass::AI @ 0x00423AC0` | none |
| `Next` transition | verified | `AnimClass::AI @ 0x00423AC0` | full object-pool identity/order is slot 1/3 scope |
| `RandomRate` conversion and use | verified | `AnimTypeClass::ReadINI @ 0x00427D00`; constructor and next transition | RNG stream integration test during implementation |
| `RandomLoopDelay` parse/use | verified | `CCINIClass::ReadMinMax @ 0x00529880`; loop reset in `0x00423AC0` | RNG stream integration test during implementation |
| `PingPong`/`Reverse` carry fields | verified | `AnimTypeClass::ReadINI @ 0x00427D00`; constructor and AI | stock UC inactive unless modded |
| stock UC lifecycle keys | verified | `ini/artmd.ini:16131..16141`, `ini/art.ini:11583..11586` | bitmap frame contents not dumped |
| Rust current garrison flash lifecycle | verified | `src/sim/components.rs`, `src/app_building_anim.rs`, `src/rules/art_data.rs` scan | implementation pending |
| Full global `AnimClass` pool/order | deferred | separate swarm slots | required before choosing full approach 3 for render order |
| Full draw/depth behavior | deferred | separate swarm slots | required for sprite ordering parity |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is ordinary garrison OccupantAnim on a live YR path? -> Yes, stock weapons define OccupantAnim and the parent-settled Fire_At path constructs normal AnimClass objects.` (evidence: `ini/rulesmd.ini:22868..22964`; parent context `TechnoClass::Fire_At @ 0x006FDD50`)
- `[RESOLVED] OQ-02 - Does delay=0 skip Middle or call it? -> Constructor calls Middle immediately when Delay is zero.` (evidence: `0x00421EA0`)
- `[RESOLVED] OQ-03 - Does Middle advance CurrentFrame? -> No CurrentFrame write or FrameStep add appears in Middle.` (evidence: `0x00424CE0`)
- `[RESOLVED] OQ-04 - Can first AI advance a just-created anim? -> No; AI clears first-AI guard +0x19C and returns before delay/timer/frame logic.` (evidence: `0x00421EA0`, `0x00423AC0`)
- `[RESOLVED] OQ-05 - What does omitted Rate mean? -> Constructor default Rate is 1 logic frame.` (evidence: `0x00427530`; `ini/artmd.ini:16131..16141`)
- `[RESOLVED] OQ-06 - What does Rate=0 mean? -> ReadINI stores 0 and AI refuses advancement when reload delay +0xC0 is zero.` (evidence: `0x00427D00`, `0x00423AC0`)
- `[RESOLVED] OQ-07 - What does omitted End mean? -> End remains 0, not SHP frame count; SHP fill only occurs for End == -1.` (evidence: `0x00427530`, `0x00421EA0`, `0x00423AC0`)
- `[RESOLVED] OQ-08 - What happens to LoopEnd=-1? -> Constructor/AI set LoopEnd to End when LoopEnd is -1.` (evidence: `0x00421EA0`, `0x00423AC0`)
- `[RESOLVED] OQ-09 - What does LoopCount=0 do with garrison ctor loop multiplier 1? -> Constructor loop byte is clamped to 1.` (evidence: `0x00421EA0`)
- `[RESOLVED] OQ-10 - Is LoopCount=-1 infinite? -> As byte 0xFF, it is not decremented at loop boundary.` (evidence: `0x00423AC0`)
- `[RESOLVED] OQ-11 - Does Next allocate a new AnimClass? -> The verified AI branch mutates the same object's Type pointer and resets timing/current frame.` (evidence: `0x00423AC0`)
- `[RESOLVED] OQ-12 - Are RandomRate endpoints converted like Rate? -> Yes when present; absent -1 endpoints are preserved until clamp normalization.` (evidence: `0x00427D00`)
- `[RESOLVED] OQ-13 - Where does RandomLoopDelay apply? -> After a loop reset when either endpoint is nonzero, Delay is set with RandomRanged and AI returns.` (evidence: `0x00423AC0`, parser `0x00529880`)
- `[RESOLVED] OQ-14 - Does PingPong need runtime state beyond metadata? -> Yes; it flips instance FrameStep and returns at boundary.` (evidence: `0x00423AC0`)
- `[RESOLVED] OQ-15 - Does constructor reverse need an instance bit? -> Yes; constructor reverse arg sets AnimClass+0x120, and AI checks it alongside type Reverse.` (evidence: `0x00421EA0`, `0x00423AC0`)
- `[DEFERRED] OQ-16 - Does lifecycle alone require full global object pool identity?` (category: requires-different-system-context; reason: this slot proves same-object Next state but not global render/update ordering; next-step-if-pursued: consume slots 1 and 3)
- `[DEFERRED] OQ-17 - What is exact draw/depth ordering for garrison flashes?` (category: requires-different-system-context; reason: not an AI lifecycle question; next-step-if-pursued: consume slots 3, 4, and 5)
- `[DEFERRED] OQ-18 - What are retail UC SHP frame counts?` (category: out-of-scope; reason: lifecycle rule uses type End/Loop fields; bitmap dump belongs to visual asset verification; next-step-if-pursued: dump UCFLASH/UCCONS/UCINIT headers)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| delay-zero constructor calls `Middle()` immediately, but first `AI` clears `+0x19C` and returns before frame advancement. | `0x00421EA0`, `0x00424CE0`, `0x00423AC0` | missing first-AI guard; current app advancement can advance spawned flashes during the same post-tick call | `src/app_building_anim.rs`, `src/sim/components.rs` or app-layer anim runtime | Add a first-AI/no-advance guard to garrison `OccupantAnim` runtime. | Newly spawned stock `UCFLASH` remains frame 0 after the first native-equivalent update visit. | Do not equate delay-zero `Middle()` with first-frame advancement. |
| omitted stock `End` remains `0`; SHP frame-count fill only runs when `End == -1`. | `0x00427530`, `0x00421EA0`, `0x00423AC0`, `ini/artmd.ini:16131..16141` | current deletion is `frame < total_frames` | `src/rules/art_data.rs`, `src/app_building_anim.rs`, future `AnimRuntime` | Parse/carry `End` and only use SHP frame count for explicit native `End=-1`. | Stock omitted-end UC anim follows native first-guard plus `End=0` boundary, not full SHP-count playback. | Do not treat omitted `End` as "play all frames". |
| `Rate<=0` stores zero and AI does not advance while reload delay is zero. | `0x00427D00`, `0x00423AC0` | `garrison_occupant_anim_delay_ms` clamps `max(1)` | `src/app_building_anim.rs`, `src/rules/art_data.rs` | Preserve zero as a static/no-advance timer state in generic runtime. | Modded `OccupantAnim=MYUC` with `Rate=0` does not advance/delete by timer. | Do not clamp `Rate=0` to one tick in generic anim logic. |
| loop remaining is a byte, constructor multiplies by ctor loop count and clamps values `<2` to `1`; `0xFF` is not decremented. | `0x00421EA0`, `0x00423AC0` | no loop byte exists | `src/sim/components.rs` or app-layer anim runtime | Use byte semantics, constructor multiplier, clamp, and sentinel. | `LoopCount=-1` modded UC loops indefinitely without decrement; `LoopCount=0` gets one loop pass. | Do not store loop count only as signed `i32` and decrement `-1`. |
| `Next` mutates the same object/type pointer and resets timing/current frame; it is not a separate unrelated spawn in the verified branch. | `0x00423AC0` | no `Next` metadata/runtime transition | `src/rules/art_data.rs`, `src/app_building_anim.rs`, app-layer anim runtime | Carry type identity/name and perform in-place transition preserving object-local identity where the app surface has one. | `MYUC End=1 Next=MYNEXT` changes the same flash runtime to `MYNEXT` and sets current frame to `MYNEXT.Start`. | Do not model `Next` as an independent event that reorders relative to other anims. |
| `PingPong` and reverse are instance-state behaviors because AI flips `FrameStep` and checks instance reverse. | `0x00421EA0`, `0x00423AC0`, `0x00427D00` | no frame step or direction state | app-layer anim runtime | Store signed frame step and instance reverse bit, even if stock UC defaults false. | Modded ping-pong UC flips direction at the native boundary and returns without loop/end deletion that tick. | Do not recompute direction from frame number only. |
| RandomRate and RandomLoopDelay consume RNG only when configured and in specific lifecycle positions. | `0x00427D00`, `0x00529880`, `0x00421EA0`, `0x00423AC0` | no random lifecycle fields or RNG consumption | app-layer anim runtime plus deterministic RNG policy | Preserve conversion, gating, and consumption points if modded `OccupantAnim` enables these keys. | `RandomRate=300,450` chooses native converted reload on construction/Next; loop delay consumes RNG only at loop reset. | Do not consume RNG for stock UC omitted random keys. |

## Negative Facts / Do Not Do

- Do not claim lifecycle parity from `Rate` alone; `End`, loop byte, first-AI guard, and `Next` are load-bearing.
- Do not clamp generic `Rate=0` to one logic tick; native stores zero and AI refuses advancement.
- Do not fill omitted stock `End` from SHP frame count; only `End=-1` triggers the fill.
- Do not implement `Next` as an unrelated newly spawned flash if object-local ordering/identity is later relevant.
- Do not force a full global `AnimClass` pool from lifecycle evidence alone. This slot proves a reusable runtime state machine is necessary; pool/order must be decided by the separate registration/render-order slots.

## Proposed Rust Tests

- `garrison_occupant_anim_first_ai_guard_does_not_advance_on_spawn_tick`
- `garrison_occupant_anim_omitted_end_does_not_use_shp_frame_count`
- `garrison_occupant_anim_rate_zero_never_advances`
- `garrison_occupant_anim_loopcount_zero_clamps_to_one_loop`
- `garrison_occupant_anim_loopcount_ff_is_infinite_sentinel`
- `garrison_occupant_anim_next_switches_same_runtime`
- `garrison_occupant_anim_random_loop_delay_applies_only_after_loop_reset`
- `garrison_occupant_anim_pingpong_flips_frame_step_before_end_delete`

## Stale Docs / Replacement Wording

- `docs/research/traces/GARRISON_SHOT_CADENCE_FIRST_ADVANCE_POSTFIX_TRACE.md`: replace any claim that a newly inserted zero-delay `AnimClass` can advance on its first same-pass AI visit with: "A newly constructed zero-delay `AnimClass` calls `Middle()` during construction, but constructor byte `+0x19C` is set to `1`; the first `AnimClass::AI` visit clears that byte and returns before timer/frame advancement. First visible advancement is only eligible on a later AI visit after the guard has cleared."
- `docs/research/CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md`: replace "generic SHP/default end behavior" wording with: "Constructor default `End` is `0`; SHP frame-count fill is gated by `End == -1`. Stock UC sections omit `End=`, so they do not auto-play to SHP frame count through that fill path."
- `src/app_building_anim.rs` comments should be updated during implementation, not by this research slot, to remove the implication that same-pass advance is the native target.

## Sources

- Ghidra decompiled: `AnimClass::AI @ 0x00423AC0`; disassembly range checked: `0x00423AC0..0x00424B3F`.
- Ghidra decompiled: `AnimClass::Constructor @ 0x00421EA0`.
- Ghidra decompiled: `AnimClass::Middle @ 0x00424CE0`.
- Ghidra decompiled: `AnimTypeClass::Constructor @ 0x00427530`.
- Ghidra decompiled: `AnimTypeClass::ReadINI @ 0x00427D00`.
- Ghidra decompiled: `CCINIClass::ReadMinMax @ 0x00529880`.
- INI checked: `ini/artmd.ini`, `ini/art.ini`, `ini/rulesmd.ini`, `ini/rules.ini`.
- Prior docs read for duplication/stale checks: `docs/research/OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md`, `docs/research/ANIM_CLASS_GHIDRA_REPORT.md`, `docs/research/ANIM_CLASS_DEEP_DIVE.md`, `docs/research/CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md`, `docs/research/traces/GARRISON_SHOT_CADENCE_FIRST_ADVANCE_POSTFIX_TRACE.md`.
- Rust scanned: `src/sim/components.rs`, `src/app_building_anim.rs`, `src/rules/art_data.rs`.

## Status

COMPLETE for the scoped `AnimClass::AI` lifecycle subset. Remaining uncertainty is outside this slot: global object pool/order and draw/depth contracts.
