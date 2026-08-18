---
name: Garrison Visual OccupantAnim Reswarm 20260527
description: Re-swarm slot 5 verification of occupied civilian body-frame/BState visuals and shot-triggered WeaponType.OccupantAnim timing/depth against current Rust.
type: ghidra-report
date: 2026-05-27
---

# Garrison Visual OccupantAnim Reswarm 20260527 - Ghidra Research Report

**Addresses:** `BuildingClass::GetCurrentFrame @ 0x0043EF90`, `TechnoClass::Fire_At @ 0x006FDD50`, `AnimClass::Constructor @ 0x00421EA0`, `AnimTypeClass::ReadINI @ 0x00427D00`, `AnimClass::AI @ 0x00423AC0`, `AnimClass::DrawIt @ 0x00422CA0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** occupied `CanBeOccupied` civilian building body-frame/BState gate plus ordinary shot-triggered `WeaponType.OccupantAnim` render spawn, timing, and depth semantics.
**Non-Scope:** garrison targeting, fire-index weapon fallback, kill credit, sell/ejection, ownership transfer, tank bunker, and chrono/temporal sparkle visuals except as a negative contrast.
**Confidence:** High for binary mechanisms and current Rust deltas; Medium for exact final pixel fixture ordering because no runtime screenshot/hash was captured.
**Active in YR:** Yes for ordinary occupied civilian buildings and occupied-building shots; conditional notes appear per finding.

## 0. Investigation Contract

Target question: Verify occupied civilian building body-frame/BState gate and shot-triggered `WeaponType.OccupantAnim` timing/depth/event semantics against current Rust render/app surfaces.

Non-goals: Do not investigate combat targeting, sell/ejection, ownership, or bunker behavior; do not redo settled `OccupantAnim != ChronoSparkle1` unless contradicted.

Evidence needed to mark COMPLETE: decompile plus disassembly/address ranges for `GetCurrentFrame`, `Fire_At`, `AnimClass::Constructor`, `AnimTypeClass::ReadINI`, `AnimClass::AI`, and `AnimClass::DrawIt`; INI defaults for `CAGAS01`, `UCFLASH`, and `OccupantAnim`; current Rust scan of `src/app_instances/shp.rs`, `src/app_building_anim.rs`, `src/app_sim_tick.rs`, `src/app_instances/overlays.rs`, `src/rules/art_data.rs`, and `src/sim/combat/mod.rs`; implementation handoff with concrete test names.

Stop conditions: Stop after verifying the two scoped visual surfaces and their Rust handoff; record any wider AnimClass, wall/overlay sorting, or BState-lifecycle questions as follow-up rather than expanding scope.

## 1. Overview

Current Rust no longer applies the occupied civilian body-frame formula unconditionally: it now calls `rendered_garrison_body_frame_index` with `entity.building_damage_state_active` and returns raw frame `0` when that gate is false. That matches the healthy occupied static case, but the Rust state is still a scoped zero/nonzero proxy rather than the full native `BuildingClass+0x534` BState table.

Ordinary occupied-building shot flashes are active YR `TechnoClass::Fire_At` behavior: `Fire_At` overrides normal weapon `Anim=` with `WeaponType+0x110 OccupantAnim`, constructs one `AnimClass` with delay `0`, loop multiplier `1`, and draw flags `0x600`, then for occupied buildings writes `anim+0x100 = -200`. Current Rust has the shot-triggered event path, rate parsing, and z-adjust value, but still lacks exact generic `AnimClass` lifecycle fields and can lose shot events when multiple fixed simulation ticks are batched before the render/app flash spawner runs.

## 2. Key Offsets And Fields

| Offset / field | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `+0x534` | `BuildingClass` | Nonzero gate before `CanBeOccupied` body-frame formula in `GetCurrentFrame` | decompile `0x0043EF90`; disasm `0x0043EFC6..0x0043F0B7` | Yes |
| `+0x157B` | `BuildingTypeClass` | `CanBeOccupied` body-frame branch flag | decompile `0x0043EF90`; `rulesmd.ini` active garrison buildings | Yes |
| `+0x634` | `BuildingTypeClass` | `TechLevel`; `-1` triggers civilian red frame `3 -> 1` collapse | decompile `0x0043EF90`; `rulesmd.ini:19305` | Yes |
| `Rules+0x1700/+0x1708` | `RulesClass` | `ConditionYellow` / `ConditionRed` thresholds | decompile `0x0043EF90`; `rulesmd.ini:752..753` | Yes |
| `WeaponType+0x110` | `WeaponTypeClass` | `OccupantAnim` selected for occupied-building shots | decompile `0x006FDD50`; asm context `0x006FF394..0x006FF41D` | Yes |
| `Anim+0x100` | `AnimClass` | Per-instance ZAdjust; occupied shot writes `-200` | decompile `0x006FDD50`, `0x00422CA0`; asm `0x006FF411..0x006FF41D` | Yes |
| `AnimType+0x2B0` | `AnimTypeClass` | Native frame delay from `Rate`, default `1`, explicit `900 / Rate` | decompile `0x00427530`, `0x00427D00` | Yes |
| `AnimType+0x2B4/+0x2B8/+0x2BC/+0x2C0/+0x2C4/+0x2C8` | `AnimTypeClass` | `Start`, `LoopStart`, `LoopEnd`, `End`, `LoopCount`, `Next` lifecycle fields | decompile `0x00427D00`, `0x00423AC0` | Yes |

## 3. Core Logic

### 3.1 Body frame gate

`BuildingClass::GetCurrentFrame @ 0x0043EF90` reads `BuildingClass+0x534` before the `CanBeOccupied` branch. If `+0x534 == 0`, it returns the current body frame (`+0xF8`) after the ordinary laser/firestorm/gate/mission checks and does not inspect garrison occupancy. If `+0x534 != 0` and `Type+0x157B CanBeOccupied` is true, it computes the garrison body frame: occupant count positive sets base frame `2`, red health increments it, buildable non-civilian yellow health can increment it, and `TechLevel == -1 && frame == 3` returns frame `1`.

Evidence: decompile `0x0043EF90`; disassembly range verified `0x0043EFC6..0x0043F0B7`. Active in YR: Yes, standard building body rendering uses this function and stock `CAGAS01` has `CanBeOccupied=yes`.

Current Rust: `src/app_instances/shp.rs:141..162` calls `rendered_garrison_body_frame_index(0, entity.building_damage_state_active, ...)`; `src/app_instances/shp.rs:736..757` returns raw frame when the gate is false; `src/app_instances/shp.rs:764..789` implements the BState-gated formula. `src/sim/game_entity.rs:142..147` documents the field as scoped and not a full BState table; `src/sim/game_entity.rs:473..483` refreshes it from `health <= ConditionYellow`.

Rust verdict: healthy occupied static civilian frame no longer applies the formula too broadly. The remaining drift is byte/mechanism exactness: Rust models a boolean threshold proxy, not native `BuildingClass+0x534` state values/writer cadence.

### 3.2 Shot `OccupantAnim` spawn and event timing

`TechnoClass::Fire_At @ 0x006FDD50` first selects normal weapon anim(s), then calls `IsOccupied` and overwrites the anim pointer with `WeaponType+0x110`. If non-null, it allocates one `AnimClass` and calls `AnimClass::Constructor(animType, coord, delay=0, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)`. For buildings (`GetAbsType() == 6`), it computes a building z adjust and then, if `GetOccupantCount() > 0`, overwrites `anim+0x100` with `0xFFFFFF38` (`-200`).

Evidence: decompile `0x006FDD50`; assembly context at `0x006FF394` shows allocation/constructor branch; `0x006FF411..0x006FF41D` calls vtable `+0x408`, tests occupant count, and writes `-200`. Active in YR: Yes for ordinary occupied-building shots with a weapon `OccupantAnim`.

Current Rust: `src/sim/combat/mod.rs:2051..2056` emits `garrison_muzzle_index` and `occupant_anim` for garrison shots; `src/sim/world/mod.rs:253..258` carries both fields; `src/app_building_anim.rs:702..758` creates a `GarrisonMuzzleFlash` from pending fire effects; `src/app_fire_effects.rs:183..192` resolves garrison muzzle origin.

Timing delta: native constructs the `AnimClass` inside the shot tick. Rust drains sim fire events during `advance_fixed_simulation`, but `src/app_sim_tick.rs:270` clears `state.pending_fire_effects` inside every fixed step and `src/app_sim_tick.rs:198..201` calls `tick_garrison_muzzle_flashes` once after the batched simulation advance. If a render frame processes multiple fixed sim ticks, only the last tick's pending garrison shot events survive to the app-layer flash spawner. Active in YR: Yes as a Rust-facing mismatch against native immediate construction.

### 3.3 `OccupantAnim` rate and lifecycle

`AnimTypeClass::Constructor @ 0x00427530` initializes `Rate`/frame delay to `1`, `Start`, `LoopStart`, `LoopEnd`, `End`, and `LoopCount` to `0`, and `Next` to null. `AnimTypeClass::ReadINI @ 0x00427D00` reads `Rate`; if the INI value is greater than zero it stores integer `900 / Rate`, otherwise it stores `0`. It also reads `Start`, `End`, `LoopStart`, `LoopEnd`, `LoopCount`, `Next`, `RandomRate`, `Shadow`, `PingPong`, and other generic anim metadata. `AnimClass::Constructor @ 0x00421EA0` copies the type rate into the instance countdown fields, computes remaining loops from `type->LoopCount * loopCount`, clamps the remaining-loop byte to at least `1`, and calls `Middle` immediately when constructor delay is zero. `AnimClass::AI @ 0x00423AC0` advances frames on the countdown, honors loop/end/next/shadow/pingpong/random-rate semantics, and deletes/transitions the anim when complete.

Evidence: decompile `0x00427530`, `0x00427D00`, `0x00421EA0`, `0x00423AC0`; disassembly `0x00421EA0..0x0042207F` and `0x00427F50..0x0042801F`. Active in YR: Yes for all ordinary `AnimClass` instances, including occupied-shot `OccupantAnim`.

INI evidence: stock `UCFLASH`, `UCCONS`, and `UCINIT` in `artmd.ini:16131..16141` define only `Layer=ground` and `Translucent=yes`; they do not define `Rate`, `Start`, `End`, `LoopStart`, `LoopEnd`, `LoopCount`, `Next`, or `Shadow`. Stock default rate therefore remains the constructor's `1` native logic frame. Weapons such as `UCPara` set `OccupantAnim=UCFLASH` in `rulesmd.ini:22944..22953`.

Current Rust: `src/rules/art_data.rs:203..212` matches native `900 / Rate` and `Rate <= 0 -> 0`; `src/rules/art_data.rs:227..232` has default one native logic frame; `src/app_building_anim.rs:784..797` requires the anim art section and fetches rate logic frames. Rust still stores only frame, total SHP frames, rate, and elapsed time in the app-layer flash path (`src/app_building_anim.rs:739..754`, `767..781`), and comments at `src/app_building_anim.rs:792..795` explicitly state `End/Loop/Next/Shadow` are not represented. Active in YR: Yes; this is a real mod and generic-anim parity gap, even if stock `UCFLASH` mostly exercises defaults.

### 3.4 ZAdjust/depth semantics

For occupied shots, native writes `anim+0x100 = -200` after construction. `AnimClass::DrawIt @ 0x00422CA0` standard non-tiled/non-flat branch passes screen position separately from depth. Screen Y is `screen_y + AnimType.YDrawOffset`; depth passed to `CC_Draw_Shape` is `(AnimType.YDrawOffset + Anim.ZAdjust) - Tactical_AdjustForZ - 2`. Flat branch uses `-3`; tiled branch has separate repeated-shape logic. Thus `-200` is a depth/z-sort input, not a screen-Y displacement.

Evidence: decompile `0x00422CA0`; disassembly range `0x00422CA0..0x0042357F`; prior `PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md` section "ZAdjust Math" independently records the same `DrawIt` formula. Active in YR: Yes for standard non-tiled `UCFLASH`/`UCCONS`/`UCINIT` shot anims, which are not `Flat` or `Tiled` in stock `artmd.ini`.

Current Rust: `src/app_building_anim.rs:21` and `:749` use `-200`. `src/app_instances/overlays.rs:508..517` keeps screen position from the resolved origin plus sprite offsets and passes z-adjust only into depth; `src/app_instances/overlays.rs:531..547` applies it through a normalized float bias with `1000` as neutral. This fixes the older screen-row-shift failure, but exact gamemd depth arithmetic is still not equivalent: native uses the integer `CC_Draw_Shape` depth formula above, while Rust uses `base_depth + (1000 - z_adjust) * 0.000001`.

## 4. Visual Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `BuildingClass::GetCurrentFrame @ 0x0043EF90` | `Building+0x534 == 0` bypasses garrison formula; `!=0 && Type+0x157B` enters | body SHP frame `+0xF8` or computed `0..3` | building body draw path | normal building body palette | Yes | body frame selection |
| 2 | `TechnoClass::Fire_At @ 0x006FDD50` | successful occupied-building shot; `IsOccupied` true | `WeaponType+0x110` anim type | fire coordinate / garrison muzzle origin | normal `AnimClass` path | Yes | shot flash spawn |
| 3 | `AnimClass::Constructor @ 0x00421EA0` | delay `0`, loop multiplier `1`, flags `0x600` | selected `OccupantAnim`, frame starts via `Middle` | constructor coordinate | `drawFlags=0x600` stored | Yes | instance creation |
| 4 | `Fire_At` post-constructor branch `0x006FF411..0x006FF41D` | building and occupant count > 0 | same anim | same coord | writes `anim+0x100=-200` | Yes | depth adjustment |
| 5 | `AnimClass::AI @ 0x00423AC0` | generic anim tick | type frames, loops, next/shadow | instance state | generic lifecycle | Yes | timing/lifetime |
| 6 | `AnimClass::DrawIt @ 0x00422CA0` | visible/non-hidden/non-filtered anim | current frame | screen position plus `YDrawOffset`; depth uses `YDrawOffset + ZAdjust - TacticalZ - const` | `CC_Draw_Shape` | Yes | final draw |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Overlay | Inactive / negative role | Evidence |
|---|---|---|---|---|---|---|
| `CAGAS01` body SHP | Yes | Yes | Yes | No | no generic occupied overlay for healthy static body | `rulesmd.ini:19302..19325`, `artmd.ini:8019..8041`, `0x0043EF90` |
| `UCFLASH` | Yes when weapon references it | Yes on GI/Allied occupied shots | Yes | Shot flash | not `BuildingClass::Update` ambient flash | `rulesmd.ini:22931..22964`, `artmd.ini:16131..16133`, `0x006FDD50` |
| `UCCONS` / `UCINIT` | Yes when weapon references them | Yes on Conscript/Initiate occupied shots | Yes | Shot flash | same generic lifecycle as `UCFLASH` | `rulesmd.ini:22868..22920`, `artmd.ini:16135..16141` |
| `CAWA19_AG` | Data exists | No for standard YR garrison | No in stock standard target | Building anim variant only if live slot and garrisonable type | not proof of stock static garrison overlay | `artmd.ini:8976..8978`, `rulesmd.ini:14609..14612` commented garrison flags |
| `CHRONOSK` / `ChronoSparkle1` | Yes | Conditional chrono/warp branch | Not ordinary garrison shot | Chrono visual | do not use as garrison shot flash | `CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md`, `rulesmd.ini:554` |

## 5. Current Rust Implementation Status

- Body frame gate: mostly aligned for healthy/damaged visible cases. `src/app_instances/shp.rs:141..162`, `736..789`, and tests at `1056..1060` prove current Rust does not render healthy occupied `CAGAS01` as frame `2` when `building_damage_state_active` is false.
- Native BState byte model: incomplete. `src/sim/game_entity.rs:142..147` says the field models only the proven zero/nonzero gate, and `473..483` derives it directly from current health vs `ConditionYellow`.
- Shot event source: implemented. `src/sim/combat/mod.rs:2051..2056` emits `occupant_anim`; `src/app_building_anim.rs:702..758` spawns flashes.
- Batched event retention: mismatching. `src/app_sim_tick.rs:270` clears pending effects each fixed tick; `198..201` consumes them once after batched advancement.
- Rate parsing: aligned for `Rate` storage. `src/rules/art_data.rs:203..212` and `src/app_building_anim.rs:784..797`.
- Generic `AnimClass` lifecycle: missing for garrison flashes. `src/app_building_anim.rs:792..795` names the gap.
- ZAdjust value and non-screen displacement: value is aligned; exact depth formula is approximate. `src/app_building_anim.rs:21`, `749`; `src/app_instances/overlays.rs:531..547`.

## 6. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `GetCurrentFrame` only applies the occupied body-frame formula when native `Building+0x534 != 0`; healthy occupied static civilians keep raw frame `0`. | decompile `0x0043EF90`; disasm `0x0043EFC6..0x0043F0B7`; `rulesmd.ini:19302..19325` | mostly aligned for boolean gate; still not full BState byte/table cadence | `src/sim/game_entity.rs`, `src/app_instances/shp.rs` | Preserve raw-frame healthy output, then replace/prove the proxy with native-equivalent BState writer timing/values when BState lifecycle is implemented | healthy occupied `CAGAS01` renders body frame `0`; red occupied `CAGAS01` after BState activation renders frame `1` | Do not remove the gate just because formula tests return frame `2` for occupied healthy BState-active inputs |
| Native constructs one `AnimClass` immediately in the shot tick for each non-null occupied-shot `WeaponType+0x110`, then writes `anim+0x100=-200` when occupant count > 0. | decompile `0x006FDD50`; asm context `0x006FF394`, `0x006FF411..0x006FF41D` | event path exists, but batched fixed ticks can drop earlier pending shot events before app flash spawn | `src/app_sim_tick.rs`, `src/app_building_anim.rs`, `src/sim/world/mod.rs` | Accumulate or spawn all shot flashes per fixed tick so no fire event is lost when `schedule.steps > 1`; keep one flash per shot event | two fixed ticks in one render frame, each with one garrison shot, produce two `GarrisonMuzzleFlash` instances in tick order | Do not key flashes only by current render frame or last pending event batch |
| Occupied-shot `OccupantAnim` is a generic `AnimClass`; `Rate`, `Start`, `End`, `LoopStart`, `LoopEnd`, `LoopCount`, `Next`, `Shadow`, `PingPong`, and `RandomRate` drive timing/lifetime. | decompile `0x00427530`, `0x00427D00`, `0x00421EA0`, `0x00423AC0`; `artmd.ini:16131..16141` | `Rate` is parsed, but garrison flash state still lacks full lifecycle fields and deletes at raw SHP frame count | `src/rules/art_data.rs`, `src/app_building_anim.rs`, `src/sim/components.rs` | Use generic AnimType metadata for garrison flashes or route them through a shared AnimClass-like app model | modded `OccupantAnim=MYUC` with `Rate=300`, `LoopStart=0`, `LoopEnd=3`, `LoopCount=1`, `Next=MYNEXT` advances and chains per native logic | Do not hardcode stock `UCFLASH` frame count as the general lifetime rule |
| `Anim+0x100=-200` is a depth/ZAdjust input; standard draw depth is `YDrawOffset + ZAdjust - Tactical_AdjustForZ - 2`, while screen Y uses `YDrawOffset` only. | decompile `0x00422CA0`; disasm `0x00422CA0..0x0042357F`; `PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md` ZAdjust section | Rust stores `-200` and no longer shifts screen Y, but depth uses an approximate normalized float bias | `src/app_instances/overlays.rs`, render sorting/depth helpers | Replace or prove the float depth bias against native integer draw-depth formula for anim sprites | occupied `UCFLASH` overlapping a building body/wall sorts using the same depth sign and integer ordering as `CC_Draw_Shape` for `ZAdjust=-200` | Do not convert `-200` into screen-pixel displacement or an arbitrary layer override |

Acceptance test-name proposals:

- `healthy_occupied_static_civilian_garrison_body_frame_stays_zero_without_bstate`
- `batched_fixed_ticks_preserve_all_garrison_occupant_anim_fire_events`
- `garrison_occupant_anim_uses_animtype_loop_end_next_lifecycle`
- `garrison_occupant_anim_z_adjust_uses_native_draw_depth_formula`

## 7. Negative Facts / Do Not Do

- Do not render healthy occupied static `CAGAS01` body frame `2` just because it has occupants. Evidence: `GetCurrentFrame @ 0x0043EF90` bypasses the formula while `Building+0x534 == 0`; Active in YR: Yes.
- Do not treat `ActiveAnimGarrisoned` as a universal occupied overlay. Evidence: `FUN_00458330` only swaps live anim slots in prior report; `CAGAS01` has no active anim slot, and `CAWASH19` garrison flags are commented out; Active in YR: Conditional only for live-slot/garrisonable entries.
- Do not use `[General] ChronoSparkle1` for ordinary garrison shots. Evidence: continuous-flash report proves `BuildingClass::Update` branch is chrono/warp gated; shot path uses `WeaponType+0x110`; Active in YR: `ChronoSparkle1` branch conditional, ordinary shot path Yes.
- Do not apply `anim+0x100=-200` as `screen_y -= 200`. Evidence: `AnimClass::DrawIt @ 0x00422CA0` separates screen Y from depth; Active in YR: Yes.
- Do not delete `OccupantAnim` after raw SHP frame count as a general rule. Evidence: `AnimClass::AI @ 0x00423AC0` honors generic type lifecycle fields; Active in YR: Yes.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::GetCurrentFrame` BState gate | verified | `0x0043EF90`; disasm `0x0043EFC6..0x0043F0B7` | full BState writer lifecycle is adjacent follow-up |
| Healthy occupied static body output | verified | `0x0043EF90`; `rulesmd.ini:19302..19325`; Rust `shp.rs:141..162`, `736..757` | none for selected frame |
| Civilian red collapse | verified | `0x0043EF90`; existing traces; Rust `shp.rs:764..789` | none for selected frame |
| `Fire_At` `OccupantAnim` override | verified | `0x006FDD50`; asm `0x006FF394..0x006FF41D` | targeting/weapon fallback out of scope |
| Shot `AnimClass` constructor args | verified | `0x006FDD50`, `0x00421EA0` | none for scoped args |
| `Rate` read/default | verified | `0x00427530`, `0x00427D00`; `artmd.ini:16131..16141` | none |
| Generic lifecycle | verified native, Rust incomplete | `0x00423AC0`; Rust `app_building_anim.rs:792..795` | implement/prove shared AnimClass-like lifecycle |
| ZAdjust draw semantics | verified native, Rust approximate | `0x00422CA0`; Rust `overlays.rs:531..547` | exact integer-depth fixture |
| Batched app event retention | verified Rust delta | Rust `app_sim_tick.rs:266..324`, `198..201` | implement/prove no dropped events |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does latest Rust still apply occupied body formula to healthy static garrisons? -> No, `building_damage_state_active=false` returns raw frame 0.` (evidence: `src/app_instances/shp.rs:141..162`, `736..757`)
- `[RESOLVED] OQ-02 - Is the Rust BState gate exact? -> No, it is a scoped zero/nonzero proxy derived from `health <= ConditionYellow`, not the full native BState table.` (evidence: `src/sim/game_entity.rs:142..147`, `473..483`)
- `[RESOLVED] OQ-03 - Is `OccupantAnim` shot-triggered and active in YR? -> Yes, `Fire_At` uses `WeaponType+0x110` after `IsOccupied`.` (evidence: `0x006FDD50`, `0x006FF394..0x006FF41D`)
- `[RESOLVED] OQ-04 - Does native use `Rate`/loop metadata? -> Yes through generic `AnimTypeClass`/`AnimClass` fields and `AnimClass::AI`.` (evidence: `0x00427530`, `0x00427D00`, `0x00423AC0`)
- `[RESOLVED] OQ-05 - What is the occupied-shot ZAdjust value? -> `-200` after positive occupant-count check.` (evidence: `0x006FF411..0x006FF41D`)
- `[RESOLVED] OQ-06 - Is ZAdjust a screen-position offset? -> No, `DrawIt` uses it in the `CC_Draw_Shape` depth argument.` (evidence: `0x00422CA0`)
- `[RESOLVED] OQ-07 - Can current Rust lose shot flashes under batched fixed ticks? -> Yes, pending events are cleared each fixed step and consumed once after the batch.` (evidence: `src/app_sim_tick.rs:270`, `198..201`)
- `[DEFERRED] OQ-08 - Pixel-perfect order against every wall/building overlap` (category: bounded-cost-too-high; reason: needs a concrete runtime fixture or exhaustive draw-stack comparison; next-step-if-pursued: compute one occupied-shot `UCFLASH` overlap case through native integer depth and Rust render sort)

## 10. Stale Docs / Follow-up Docs

- `docs/research/traces/GARRISON_SHOT_OCCUPANTANIM_RENDER_POSTFIX_TRACE.md`: replace the stale z-adjust Rust failure wording with: "Current Rust no longer applies `-200` as a screen-row shift: `src/app_instances/overlays.rs:508..517` keeps screen position from the fire origin and applies `z_adjust` only through `garrison_flash_depth`. The remaining drift is exact arithmetic: native `AnimClass::DrawIt @ 0x00422CA0` passes `YDrawOffset + Anim.ZAdjust - Tactical_AdjustForZ - 2` to `CC_Draw_Shape`, while Rust uses a normalized float bias in `src/app_instances/overlays.rs:542..547`."
- `docs/research/traces/GARRISON_SHOT_OCCUPANTANIM_RENDER_POSTFIX_TRACE.md`: replace the stale runtime cadence wording with: "Current Rust advances garrison flash elapsed time by fixed simulation ticks via `src/app_sim_tick.rs:198..201`, not wall-clock render delta. The remaining timing drift is event retention under batched fixed ticks and missing generic `AnimClass` lifecycle fields."
- `docs/research/traces/GARRISON_SHOT_Z_ADJUST_DEPTH_POSTFIX_TRACE.md`: replace the "PASS for sign and neutral only" current-status wording with: "Current Rust stores `-200` and applies it as a depth bias rather than screen displacement, but the float-depth scale is not the native integer `CC_Draw_Shape` depth formula."

## Sources

- Ghidra decompile/read-only: `0x0043EF90`, `0x006FDD50`, `0x00421EA0`, `0x00427530`, `0x00427D00`, `0x00423AC0`, `0x00422CA0`, `0x004509D0`, `0x00451EE0`
- Disassembly/address ranges: `0x0043EFC6..0x0043F0B7`, `0x006FF394..0x006FF41D`, `0x00421EA0..0x0042207F`, `0x00422CA0..0x0042357F`, `0x00427F50..0x0042801F`
- INI: `ini/rulesmd.ini:752..753`, `14589..14612`, `19302..19325`, `22925..22964`; `ini/artmd.ini:8019..8041`, `8970..8982`, `16131..16141`
- Prior docs: `GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md`, `CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md`, `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md`, `traces/GARRISON_SHOT_OCCUPANTANIM_RENDER_POSTFIX_TRACE.md`, `traces/GARRISON_SHOT_Z_ADJUST_DEPTH_POSTFIX_TRACE.md`
- Rust scan: `src/app_instances/shp.rs`, `src/app_building_anim.rs`, `src/app_sim_tick.rs`, `src/app_instances/overlays.rs`, `src/rules/art_data.rs`, `src/sim/game_entity.rs`, `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`

## Status

COMPLETE for the scoped visual mechanisms. One bounded follow-up remains for a concrete pixel-order fixture, but the binary mechanism and Rust deltas needed for implementation are verified.
