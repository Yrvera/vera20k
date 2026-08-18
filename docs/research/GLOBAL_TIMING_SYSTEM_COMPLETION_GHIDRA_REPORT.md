# Global Timing System Completion - Ghidra Research Report

**Date:** 2026-05-18

**Scope:** Completion pass for the RA2/YR global timing model: main tick order, speed selection, frame counter semantics, timer primitives, animation normalization, and major timing consumers.

**Primary addresses decompiled:**

- `0055D360` `Main_Tick`
- `0055E160` frame throttle / wait service
- `006C8C40` `GetRadarTimer`
- `005FA350` `OptionsClass__SetDefaults`
- `005FA620` `OptionsClass__ReadFromINI`
- `004E1DE0` in-game options dialog apply
- `00671EA0` `RulesClass__ReadMultiplayerDialogSettings`
- `00697F10` `SessionClass__ReadSkirmishSettings`
- `005B67F0` network/session game option apply
- `0055AFB0` `LogicClass__PerTickUpdate`
- `0055DEE0` `LogicClass__AI`
- `0046B640` `CDTimerClass__Start`
- `00426630` `CDTimerClass__GetTimeRemaining`
- `004C9480` CDTimer/RateTimer remaining helper
- `004C9220` `RateTimer__Set`
- `004C93D0` `RateTimer__Current`
- `004C9300` `FacingClass__UpdateFacing`
- `00427D00` `AnimTypeClass__ReadINI`
- `005FB2E0` normalized delay helper
- `00421EA0` `AnimClass__Constructor`
- `00423AC0` `AnimClass__AI`
- `00424CE0` `AnimClass__Middle`
- `00424F00` `AnimClass__Start`
- `004255B0` `AnimClass__Destroy`
- `0051D6F0` `InfantryClass__Do_Action`
- `0051CDB0` `InfantryClass__UpdateIdleAction`
- `00736990` `UnitClass__Facing_Update`
- `004DB1A0` `FootClass__GetCurrentSpeed`
- `00520F40` `FootClass__Locomotion_AI`
- `004DA530` `FootClass__AI`
- `0070E5A0` `TechnoClass__UpdateTemporalVisual`
- `0070E920` `TechnoClass__UpdateGapVisual`
- `0070B570` `TechnoClass__RockingUpdate`
- `0043FB20` `BuildingClass::Update`
- `00450630` `BuildingClass__UpdateRepairAndPower`
- `0043E7B0` `BuildingClass__UpdateGarrisonFire`
- `00454DB0` `BuildingClass__UpdateGapGenerator_Tick`
- `004C9B20` `FactoryClass::AI`
- `004C9C70` `FactoryClass__StartProduction`
- `004C9EA0` `FactoryClass__SetRate`
- `004CA6E0` `FactoryClass__RecalcAllRates`
- `004C9FB0` `FactoryClass__CalcRate`
- `006F47A0` `FactoryClass__GetBuildStepTime`
- `006CAF90` `SuperClass__Constructor`
- `006CBEE0` `SuperClass__AnimStage`
- `006CC390` `SuperClass__Launch`
- `0071A760` `TemporalClass__Update`
- `006297F0` `TemporalClass__AI`
- `006A7780` `SidebarClass__Action`

**Confidence:** High for the core clock spine, speed source chain, CDTimer/RateTimer semantics, animation delay normalization, and the listed direct timing consumers. Medium for exact arithmetic inside `006F47A0` factory build-time calculation, late `00454DB0` gap-generator locals, and the full superweapon launch variants inside `006CC390`, because those functions are large or have decompiler register/local artifacts.

**Active in YR:** Yes for the local skirmish timing spine and the named gameplay/visual consumers. Network timing branches are active for multiplayer modes. Some fog/gap/superweapon branches are conditional on rules, object state, map state, or game mode.

## Executive Summary

The timing model is not a single global FPS constant. GameMD has one authoritative frame counter, `g_CurrentFrameCounter`, but the visible game pace is controlled by a separate wait/throttle layer and many systems consume the counter in different ways.

The important correction is that local skirmish timing uses a speed byte from the session/rules path and a coarse `timeGetTime() >> 4` bucket clock. In standard YR skirmish, `[rulesmd.ini] [MultiplayerDialogSettings] GameSpeed=1` is the absent-skirmish fallback. `[Options] GameSpeed=3` is the UI/default options value, not the local skirmish fallback once a skirmish session is created.

The main simulation frame increments late in `Main_Tick`, after input, logic, map logic, rendering, side services, and per-tick update have already run. Any code called before the increment observes the old frame number. This matters because most CDTimer-style checks use `elapsed = g_CurrentFrameCounter - start` and expire only when `elapsed >= duration`.

Current Rust timing is structurally different: it has a 45 Hz fixed app simulation tick, a synthetic `binary_frame = (total_sim_ms * 15) / 1000`, and many animation/effect systems driven by elapsed milliseconds. That may be workable as an internal engine scheduler, but it is not a faithful model of GameMD's timing semantics unless all GameMD frame-counter consumers are explicitly mapped onto the same late-increment frame and speed-throttle behavior.

## Clock Taxonomy

| Clock / timer | Verified GameMD behavior | Main consumers | Active in YR | Confidence |
| --- | --- | --- | --- | --- |
| Game frame counter | `g_CurrentFrameCounter` increments once at the end of the normal active `Main_Tick` path, after logic/render/service calls. | CDTimer, RateTimer, anims, factories, buildings, visual state machines, many modulo checks. | Yes | High |
| Local speed throttle | Uses `GetRadarTimer() = timeGetTime() >> 4`, initial bucket in `DAT_00887348`, and speed budget in `DAT_00887350`. | Local skirmish/campaign pacing. | Yes | High |
| Local speed byte | `DAT_00A8EB60`; session/rules/options paths write this byte. | Local speed throttle budget. | Yes | High |
| Network frame pacing | Uses millisecond `timeGetTime` fields `DAT_00887328` and `DAT_00887330`, with requested/network FPS logic and backlog adjustment. | Network games. | Multiplayer only | High |
| CDTimer | Stores frame start and duration. Remaining time is frame-count based. | Superweapons, factories, animation frames, visuals, object timers. | Yes | High |
| RateTimer | Interpolates a low 16-bit value over a frame duration at an integer rate. | Facing, rocking/sinking support, visual transitions. | Yes | High |
| Per-AI-call decrement | Some systems decrement counters once per object AI call rather than using CDTimer directly. | Initial anim delay, temporal damage progression, sidebar counters, gap generator fade byte. | Yes | High |
| Render/app elapsed milliseconds in Rust | Rust currently uses `tick_ms`, `elapsed_ms`, `rate_ms`, and a synthetic `binary_frame`. | Current engine rendering/effects/sim subsystems. | Rust only | High |

## Core Clock Spine

### `Main_Tick` order

Verified at `0055D360`.

If `g_GameActive == 0`, `Main_Tick` returns early. If the game is active but not currently running, it waits in a loop. Local modes sleep for 500 ms in the inactive-running wait; network-ish modes sleep 10 ms and continue servicing the network.

For normal local modes, `Main_Tick` sets:

- `DAT_00887348 = GetRadarTimer()`
- `DAT_00887350 = DAT_00A8EB60`

There is a special mode-0 override when `DAT_00A8EDDC == 0`: it temporarily writes `DAT_00A8EB60 = 2`, sets `DAT_00887348 = GetRadarTimer()`, and sets `DAT_00887350 = 2`.

For nonlocal/network timing, the function uses requested/network FPS globals. If `_DAT_00A8D5F8 & 2` is set, the wait budget can be forced to zero. If `DAT_00A8B558 == 0`, it uses speed budget `2` and network millisecond budget `0x21` (33 ms). Otherwise, it derives `DAT_00887350 = 0x3c / DAT_00A8B558` and `DAT_00887330 = 1000 / DAT_00A8B558`.

Network mode 4 can add up to three 10 ms increments to the network budget when remote-frame backlog crosses one-quarter, one-half, and three-quarter thresholds of `g_NetworkFrameBudget`.

There is a scenario-delay/render-only branch: when `Scenario + 0x62C != 0`, the function services network/input/render-like work and returns without advancing the game frame.

The normal active branch order is:

1. `GScreenClass__Input`
2. `LogicClass__AI`
3. optional house AI tick
4. network keepalive every frame where `(g_CurrentFrameCounter & 7) == 7` in network mode 4
5. `Map__Logic`
6. main render/tactical frame
7. side services
8. `LogicClass__PerTickUpdate`
9. more services and network loop
10. if stop/pause gates are clear, increment `g_CurrentFrameCounter`
11. wait/throttle service at `0055E160`
12. trailing service calls

The frame increment is gated by several global stop/pause/status flags: `DAT_00A83D49`, `DAT_00A8ECD0`, `DAT_008B41C0`, and `DAT_00A83D48`.

**Finding:** Logic code called from `Main_Tick` sees the pre-increment frame number. A timer started during a frame stores the current old value, and does not see one elapsed frame until the next successful active tick increments and re-enters logic.

### Wait / throttle service

Verified at `0055E160`.

For local modes 0 and 5, the wait service uses the `GetRadarTimer()` bucket clock and the budget set by `Main_Tick`. It subtracts elapsed 16-ish ms buckets from the budget. If time remains, it calls a service function and sleeps while waiting. The decompiled call passes the remaining bucket count directly to `Sleep`; the source-level unit intent is not recovered, so this report treats the observed logic as a coarse bucket wait rather than a precise millisecond conversion.

For network/nonlocal modes, it uses `timeGetTime()` directly with `DAT_00887328` and `DAT_00887330`, services network during the wait, and may run input/logic/tactical work if there is enough remaining budget.

The same wait service also updates timing telemetry: `DAT_00A8E314` accumulates remaining time, and every `0x3C` radar buckets it copies and resets frame-count telemetry including `DAT_00ABCD40`.

### `GetRadarTimer`

Verified at `006C8C40`.

`GetRadarTimer` returns `timeGetTime() >> 4`.

This is a coarse 16 ms bucket source. It is not the game frame counter, and it is not the same as the network millisecond path.

## Speed Source Chain

### Options default and read path

Verified at `005FA350` and `005FA620`.

`OptionsClass__SetDefaults` initializes `[Options] GameSpeed` to `3`.

`OptionsClass__ReadFromINI` reads `Options/GameSpeed` into the options object and does not clamp that value in the decompiled path. Difficulty is clamped later, but `GameSpeed` was not.

This confirms that Options `GameSpeed=3` exists as an options/UI default, but it is not by itself the standard YR skirmish fallback.

### Rules multiplayer dialog setting

Verified at `00671EA0`.

`RulesClass__ReadMultiplayerDialogSettings` reads `[MultiplayerDialogSettings] GameSpeed` into the rules object at offset `+0x14A0` when the section exists. It also reads related multiplayer defaults such as fog of war, tiberium growth, and superweapon allowance.

INI evidence:

- `ini/rulesmd.ini` has `[MultiplayerDialogSettings] GameSpeed=1`.
- `ini/rules.ini` has base RA2 `GameSpeed=0`.

YR `rulesmd.ini` patches base RA2 data, so the standard YR default for absent skirmish setting is speed byte `1`.

### Skirmish session setting

Verified at `00697F10`.

`SessionClass__ReadSkirmishSettings` reads skirmish `GameSpeed` with default `RulesClass + 0x14A0`. Therefore, if the user's skirmish settings do not explicitly provide `GameSpeed`, standard YR falls back to rulesmd speed `1`.

### In-game slider mapping

Verified at `004E1DE0`.

The in-game options dialog maps the speed slider as:

`speed_byte = 6 - TBM_GETPOS`

If changed during an active nonlocal game (`g_GameActive == 1`, `g_GameMode != 0`, `g_GameMode != 5`), it queues command type `0x0D` if there is room in the command queue, timestamped with `timeGetTime`.

The function writes `DAT_00A8EB60` to the current or newly selected speed byte at the end.

The same dialog maps a second slider with `6 - TBM_GETPOS` into `DAT_00A8EB70`, and another control updates extra animation settings.

### Network/session game option apply

Verified at `005B67F0`.

The function decodes network/session game options. It compares a packet byte at offset `+0xA2` to `DAT_00A8B268`. When applying, it writes:

- `DAT_00A8B268 = packet_byte_at_0xA2`
- `DAT_00A8EB60 = DAT_00A8B268`

This is a second live writer into the same local speed byte used by the throttle path.

## Timer Primitives

### CDTimer

Verified at `0046B640`, `00426630`, and `004C9480`.

`CDTimerClass__Start` writes:

- `start = g_CurrentFrameCounter`
- `duration = param_2`

`CDTimerClass__GetTimeRemaining` behavior:

- If `start == -1`, it returns the raw duration field.
- If `start != -1` and `g_CurrentFrameCounter - start < duration`, it returns `duration - elapsed`.
- If elapsed is greater than or equal to duration, it returns `0`.

The timer expires on `elapsed >= duration`, not `elapsed > duration`.

The remaining helper at `004C9480` is used by RateTimer-like structures. It returns false/no remaining time when the rate word is not positive, or when elapsed has reached the duration.

### RateTimer

Verified at `004C9220` and `004C93D0`.

RateTimer is frame based. It interpolates the low 16 bits of a target value over a duration derived from integer difference divided by integer rate.

Observed layout from decompiled access pattern:

| Offset | Meaning |
| --- | --- |
| `+0x00` | target dword |
| `+0x04` | start/current dword for interpolation |
| `+0x08` | timer start frame |
| `+0x10` | timer duration |
| `+0x14` | rate word |

`RateTimer__Set`:

- If the low 16 bits of the target are unchanged, it returns `0`.
- If a previous interpolation is still running, it first samples the current interpolated value and uses that as the new start value.
- It writes `start_frame = g_CurrentFrameCounter`.
- It computes `duration = abs(new_low16 - start_low16) / rate` using integer division when rate is positive.
- If rate is not positive, no active timer duration is started.

`RateTimer__Current`:

- If rate is less than 1, elapsed has reached duration, or no remaining duration exists, it returns the target dword.
- Otherwise, it returns the target high 16 bits and an interpolated low 16-bit value.
- If the computed divisor/step would be less than 1, it returns the target directly.

### Facing

Verified at `004C9300` and `00736990`.

Facing update paths are not free-running floats. Units and other technos sample RateTimer current values and set new target values against the global frame counter. `UnitClass__Facing_Update` uses `RateTimer__Current` and `RateTimer__Set` for body/turret facing. Some turret/status cases sample CDTimer remaining time and write visual state bytes.

`FacingClass__UpdateFacing` has decompiler ambiguity around an unresolved CDTimer call, but the visible behavior is still frame-counter based: it compares current/requested values, starts at `g_CurrentFrameCounter`, and writes duration fields accordingly.

## Animation Timing

### AnimType INI fields

Verified at `00427D00`.

Important fields:

| INI key / field | Internal field | Verified behavior |
| --- | --- | --- |
| `Rate` | `+0x2B0` | Read default `-1`; if value `< 1`, internal delay becomes `0`; otherwise `900 / Rate` integer. |
| `Start` | `+0x2B4` | Start frame. |
| `LoopStart` | `+0x2B8` | Loop start frame. |
| `LoopEnd` | `+0x2BC` | Loop end frame; defaults later to End if `-1`. |
| `End` | `+0x2C0` | End frame; constructor fills from SHP frame count if `-1`. |
| `LoopCount` | `+0x2C4` | Loop count byte used by `AnimClass`. |
| `Next` | `+0x2C8` | Next anim type. |
| `SpawnParticle` | `+0x2CC` | Particle type spawned at start. |
| `NumParticles` | `+0x2D0` | Particle count. |
| `RandomLoopDelay` | `+0x2DC/+0x2E0` | Random delay range. |
| `RandomRate` | `+0x2E4/+0x2E8` | Each endpoint converted by the same `900 / rate` rule or `0` for values `< 1`; max clamped nonnegative, min clamped down to max. |
| `TrailerAnim` | `+0x308` | Trailer animation type. |
| `TrailerSeperation` | `+0x30C` | Trailer spawn modulo/separation. |
| `Normalized` | `+0x362` | Enables speed-normalized delay helper. |
| `Layer` | `+0x364` | Draw layer. |
| `Flat` | `+0x369` | Flat flag. |
| `Translucent` | `+0x36A` | Translucency flag. |
| `Shadow` | `+0x372` | If true, constructor halves frame count for End fill. |

INI scan confirms many `art.ini` and `artmd.ini` anims use `Normalized=yes`, `Rate=200/300/400/450`, `RandomRate`, `TrailerSeperation`, and `LoopCount`. Several comments around delayed-fire anims explicitly warn that some `Normalized=no` delays must match hard frame delay rather than scaling.

### Normalized delay helper

Verified at `005FB2E0`, with supporting prior doc evidence from `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`.

Behavior:

- If delay is `0`, return `0`.
- If delay is less than `5`, use a small-delay table keyed by speed and delay.
- Otherwise return `(delay << 3) / (speed + 1)`.

Known table facts from previous verified timing report:

- `Rate=200` converts to internal delay `4`.
- `Rate=300` converts to internal delay `3`.
- `Rate=400` converts to internal delay `2`.
- `Rate=450` converts to internal delay `2`.
- At `Options.GameSpeed=3`, small delays normalize as `1->1`, `2->2`, `3->3`, `4->4`.
- If speed is forced to `2`, small delays normalize as `1->1`, `2->3`, `3->4`, `4->5`.

The helper proves that normalized animation delay is not simple milliseconds and not a universal 15 FPS transform. It is a speed-byte-dependent frame-delay transform with a special table for delays 1 through 4.

### AnimClass lifecycle

Verified at `00421EA0`, `00423AC0`, `00424CE0`, `00424F00`, and `004255B0`.

`AnimClass__Constructor`:

- Stores timer/start frame from `g_CurrentFrameCounter`.
- Initializes current frame, frame step, type pointer, caller initial delay, and loop state.
- If type End is `-1`, reads SHP frame count and halves it when `Shadow` is true.
- If type LoopEnd is `-1`, copies End.
- Chooses delay/reload from type Rate or RandomRate range.
- Applies the normalized helper only when the type has `Normalized=yes`.
- Reverse flags can set current frame to `End - 1` and negate frame step.
- Multiplies type loop count by caller loop count and clamps values below 2 to 1.
- If caller initial delay is zero, it calls `AnimClass__Middle` immediately.

`AnimClass__AI`:

- Handles looping sound and visibility/removal checks.
- Handles special type behavior such as bouncer, meteor, no-ore hiding, and psi warning.
- Trailer anim spawn gate is frame counter based: if trailer is active and type has a trailer anim, it spawns when `TrailerSeperation == 1` or `g_CurrentFrameCounter % TrailerSeperation == 0`.
- Caller initial delay is a per-AI-call decrement. It decrements once per call and calls `Middle` when it reaches zero.
- Core frame advance waits until the CDTimer-style remaining time is zero and reload is nonzero.
- On frame advance, it adds the frame step, writes the last-frame start to `g_CurrentFrameCounter`, and reloads the delay fields.
- Loop/end handling includes loop byte decrement, random loop delay, Next animation transfer, and expiration/self-destruction.

`AnimClass__Middle` starts the active animation effects. It plays start sound if configured, calls `AnimClass__Start` if no delayed Start frame is needed, and handles tiberium chain reaction. It does not introduce an independent wall-clock timer.

`AnimClass__Start` spawns configured particles immediately and handles scorch/crater/debris side effects.

`AnimClass__Destroy` detaches owner, releases looping sound, plays stop sound if configured, and uninitializes. The decompiled body did not show a separate independent ExpireAnim timer here; Next/expiration behavior is visible in `AnimClass__AI`.

### Infantry action animation

Verified at `0051D6F0` and `0051CDB0`.

`InfantryClass__Do_Action` starts action sequences at `g_CurrentFrameCounter` and stores delay/reload values from the infantry sequence metadata.

Only these action IDs pass their delay through the normalized helper:

- `9`
- `10`
- `0x12`
- `0x13`
- `0x17`
- `0x20`

Other action delays use the raw sequence delay byte.

`InfantryClass__UpdateIdleAction` sets an idle timer at `g_CurrentFrameCounter`, randomizes duration from a rules field, and can trigger idle fidget actions 9 or 10. Those idle fidgets therefore use the normalized subset above.

## Major Timing Consumers

### Logic tick service

Verified at `0055AFB0`.

`LogicClass__PerTickUpdate` begins by incrementing `DAT_00ABCD40`, a frame-count telemetry counter later consumed by the wait service.

It uses frame-counter timers throughout:

- Scenario timers use `g_CurrentFrameCounter - start < duration`.
- Bridge shroud recalculation runs when `g_CurrentFrameCounter % 0x78 == 0`.
- Growth/spread/bomb/lightning/EMP/tactical/factory/house/team systems are stepped from this per-tick function.

Because this function is called before the late `Main_Tick` increment, every timer check inside it observes the old frame number for the current active frame.

`LogicClass__AI` at `0055DEE0` is not a simple object-AI loop despite its label. The decompiled body shows command/action dispatch and UI/action toggles, including writes to `DAT_00ABCE14` bits. Its signature is decompiler-ambiguous in the direct output.

### Mobile objects

Verified at `004DB1A0`, `00520F40`, and `004DA530`.

`FootClass__GetCurrentSpeed` computes speed from type/house/ability data and returns half speed for a particular infantry/state case. It is a speed sample provider, not a frame timer.

`FootClass__Locomotion_AI` is a per-frame bridge into locomotor methods and action animation selection. It calls locomotor methods and chooses infantry actions when not moving. The function is active YR locomotion dispatch, but it does not itself define a CDTimer model.

`FootClass__AI` contains several frame-counter gates:

- self-heal/tiberium check when `g_CurrentFrameCounter % Rules+0x1808 == 0`
- fog-border timer at fields `+0x197/+0x199`, duration `0x0F`
- cell-action check every 16 frames with `(g_CurrentFrameCounter & 0x8000000F) == 0`
- weapon/rate gates using `g_CurrentFrameCounter % type+0x294` and `% type+0x298`
- idle scatter every 64 frames when low byte masked by `0x3F` equals `0x3F`

### Techno temporal and gap visuals

Verified at `0070E5A0` and `0070E920`.

`TechnoClass__UpdateTemporalVisual` is a frame-counter state machine. It has states with durations:

- state 0 to 1: 6 frames
- state 1 to 2: 6 frames, then duration 4
- state 2 to 3: 4 frames, then random duration `RandomRanged(-5, 5) + 0x14`, so 15 to 25 frames
- state 3 to 4: that random duration, then 8
- state 4 to 5: 8 frames, then `0x10`
- state 5 loops back to 4 after `0x10` unless an external CDTimer remaining value is below `0x36`, then it goes to state 6
- state 6 waits for external remaining below `0x1F`, then state 7 for 6 frames
- state 7 to 8: 6 frames, then 4
- state 8 to 9: 4 frames, then `0x14`
- state 9 to 10: after `0x14`

`TechnoClass__UpdateGapVisual` has the same style of frame-counter state machine. It is active only for eligible building/object state and owner checks. State 3/4/5 use `0x40` frame durations. State 5 waits until an external CDTimer remaining value is below `0x9E`; state 6 waits until remaining is below `0x1F`; then it follows the 6, 4, `0x14` tail.

### Rocking and sinking

Verified at `0070B570`.

`TechnoClass__RockingUpdate` is per-AI-call visual motion with float angles and deltas. It samples RateTimer current values for sinking/facing support but does not use wall-clock timing. Its cadence is the object update cadence.

### Buildings

Verified at `0043FB20`, `00450630`, `0043E7B0`, and `00454DB0`.

`BuildingClass::Update` contains multiple frame timing consumers:

- CDTimer remaining for type flag `+0xCA1`, writing visual/status byte `field_0x4A0`.
- Damage-fire animation creation/removal when health thresholds change.
- Animation production every `0x18` frames for certain mission/status states.
- Building animation change at `field_0x538`; anim index 0 or 1 applies normalized helper to type animation delay and writes timer start `g_CurrentFrameCounter`, while other indexes use raw delay.
- Death/deferred-destroy timer uses CDTimer-style fields.

`BuildingClass__UpdateRepairAndPower` uses modulo frame timing for repair:

- It computes a frame interval with `Math__ftol`.
- It executes only when `g_CurrentFrameCounter % interval == 0`.
- It charges credits, repairs by a frame amount, clamps health, and updates damage animation slots.
- AI low-power/building repair delay writes owner timer start from `g_CurrentFrameCounter` and a randomized duration.

`BuildingClass__UpdateGarrisonFire` is primarily render/draw-ish occupant shape logic. No independent frame timer was visible in that function body.

`BuildingClass__UpdateGapGenerator_Tick` is a per-call state tick. It increments or decrements a byte fade value and dirties drawing at specific byte values:

- opening state increments up to `0x0F`, dirtying at `1`, `6`, and `11`
- closing state decrements down to `0`, dirtying at `0`, `5`, and `10`
- it writes the fade value to linked anims at offset `+0x178`

The latter half of this function has unreliable register-carried decompiler locals, so only these visible state/fade facts should be treated as high confidence.

### Factories and production

Verified at `004C9B20`, `004C9C70`, `004C9EA0`, `004CA6E0`, `004C9FB0`, and `006F47A0`.

`FactoryClass::AI` uses a CDTimer-style production timer:

- If the factory is not suspended and has an object/special item, it checks remaining time.
- If remaining time is nonzero or duration is zero, it sets `Production_HasChanged=false` and returns.
- On expiry it increments `Production_Value` by `Production_Step`, sets changed, and restarts the timer at `g_CurrentFrameCounter` with the same duration.
- Completion occurs at progress `0x36` (54).
- If credits are insufficient, it sets OnHold and rolls `Production_Value--`.

`FactoryClass__StartProduction` starts/switches production by setting suspended, clearing timer duration/time-left, zeroing progress, and assigning the object. If the factory is busy and queue limits allow, it queues instead.

`004C9EA0` decompiled as `FactoryClass__SetRate`, not as the planned name "CompletionStep". It unsuspends valid production and computes a build step time as:

`FactoryClass__GetBuildStepTime() / 0x36`

The result is clamped to `1..0xFF`, then stored as timer duration/time-left with `start = g_CurrentFrameCounter`. A manual/retaddr flag path can re-suspend and zero the timer.

`FactoryClass__RecalcAllRates` recomputes the same clamped duration for factories owned by a given house/class pointer and updates duration if it changed. It does not restart the start frame in the visible path.

`FactoryClass__CalcRate` is a pure rate calculation using the same `GetBuildStepTime() / 0x36` clamp.

`006F47A0` decompiled as `FactoryClass__GetBuildStepTime`, not as the planned name "GetProductionSpeed". It calculates build time from object type, house build bonus, power ratio, and factory count. The exact arithmetic is medium confidence due to decompiler complexity, but the integration point and per-step timing are high confidence.

### Superweapons

Verified at `006CAF90`, `006CBEE0`, and `006CC390`.

`SuperClass__Constructor` initializes a timer/start field from `g_CurrentFrameCounter`, stores type and owner, initializes flags, and registers the superweapon object.

`SuperClass__AnimStage` returns a stage based on active/ready flags and a ratio converted with `Math__ftol`, clamping above `0x34` to `0x35`.

`SuperClass__Launch` is a very large variant switch over superweapon type. Timing findings from this pass:

- Most launch cases are gated by a ready/active byte before side effects.
- Chronosphere/warp-like cases spawn `AnimClass` instances and write locomotor/timer fields.
- One locomotor-associated path writes a timer start from `g_CurrentFrameCounter` and duration `500`.
- A force-shield-like case writes a duration-like value from rules fields `Rules+0x17BC - Rules+0x17C4`, stores target coordinates, and calls a power sabotage function.

Full per-superweapon launch timing should be a dedicated follow-up slice. This report only classifies the clock dependencies.

### Temporal logic

Verified at `0071A760` and `006297F0`.

`TemporalClass__Update` is not a CDTimer countdown. It performs per-update damage/progress:

- It clears target/linkage if owner/target state no longer matches.
- It detaches if source-target distance exceeds `Rules+0xF60 * 0x100`.
- It sums chain damage, stores a weapon/warhead-derived value at `+0x4C`, and decrements `+0x48` by source plus chain damage.
- When `+0x48 < 1`, it spawns warp animation and proceeds with erasure/teleport/cleanup behavior.

`TemporalClass__AI` handles a separate five-state visual/attachment machine. It writes a target visual timer using `g_CurrentFrameCounter` and a rules/weapon value, advances anim frames, spawns warp sparkles, samples RateTimer current values, and uses AnimClass delays for spawned effects.

### Sidebar/UI action cadence

Verified at `006A7780`.

`SidebarClass__Action` calls strip AI and power animation logic. It uses per-call counters to increment/decrement UI frame state until SHP frame count limits. No direct `g_CurrentFrameCounter` or `GetRadarTimer` use was visible in the decompiled body. Its cadence therefore comes from how often `Main_Tick` and service paths call sidebar action, not from a standalone wall-clock timer in this function.

## INI Timing Inputs

| INI source | Keys / examples | Timing role | Evidence |
| --- | --- | --- | --- |
| `ini/rulesmd.ini` | `[MultiplayerDialogSettings] GameSpeed=1` | Standard YR absent-skirmish speed fallback. | `RulesClass__ReadMultiplayerDialogSettings`, `SessionClass__ReadSkirmishSettings` |
| `ini/rules.ini` | `[MultiplayerDialogSettings] GameSpeed=0` | Base RA2 fallback, overridden by YR md rules. | INI scan |
| `ini/rules.ini`, `ini/rulesmd.ini` | `LightningHitDelay=10`, `LightningScatterDelay=5` | Per-tick lightning timing inputs. | `LogicClass__PerTickUpdate` calls lightning processing |
| `ini/rules.ini`, `ini/rulesmd.ini` | `[Tiberiums] Growth`, `GrowthPercentage`, `Spread`, `SpreadPercentage` | Tiberium growth/spread timing and probability inputs. | `LogicClass__PerTickUpdate` calls growth/spread drivers |
| `ini/rules.ini`, `ini/rulesmd.ini` | `[Repair] Rate=.08` and related repair fields | Building repair interval/amount calculation. | `BuildingClass__UpdateRepairAndPower` |
| `ini/rules.ini`, `ini/rulesmd.ini` | superweapon `RechargeTime`, `ShowTimer` | Superweapon readiness/stage display inputs. | `SuperClass` functions |
| `ini/art.ini`, `ini/artmd.ini` | `Rate`, `RandomRate`, `Normalized`, `LoopCount`, `TrailerSeperation` | Anim frame delay and trailer cadence. | `AnimTypeClass__ReadINI`, `AnimClass__AI` |
| `ini/art.ini`, `ini/artmd.ini` | `DelayedFireDelay`, often with `Normalized=no` comments | Weapon/building delayed fire visual timing. | INI scan and building/animation consumers |

## Rust Implementation Status

This is a source scan, not a judgment that each difference is wrong. It identifies places that need an explicit mapping decision against the verified GameMD timing model.

| Rust path | Current timing behavior found | Parity risk |
| --- | --- | --- |
| `src/app_types.rs` | Defines `SIM_TICK_HZ`, `SIM_TICK_MS`, `DEFAULT_YR_SKIRMISH_GAME_SPEED=1`, `GAME_SPEED_BUCKET_MS=16`, and `tps_for_game_speed`. | Good recognition of YR speed byte and bucket, but must align with late GameMD frame counter semantics. |
| `src/app_sim_tick.rs` | `advance_fixed_simulation` scales elapsed by `sim_speed_tps / SIM_TICK_HZ`; scheduler uses fixed steps. | GameMD local speed throttles frame advance rather than scaling all elapsed time consumers independently. |
| `src/sim/world/mod.rs` | Tracks `total_sim_ms`; computes `binary_frame = (total_sim_ms * 15) / 1000`; many systems receive `tick_ms`; combat samples `barrel.current(binary_frame)` at start of tick; several effects use `rate_ms: 67`. | Synthetic 15 FPS frame may diverge from GameMD frame counter, especially around late increment and speed byte normalization. |
| `src/sim/animation.rs` | Uses `elapsed_ms`, `tick_ms`, `dt_ms`, sequence `tick_ms`, and ms-based harvest/voxel animation. | GameMD AnimClass and infantry actions are frame-delay based with selective speed normalization. |
| `src/app_building_anim.rs` | Uses elapsed-ms timers for crane, idle anim, damage fire overlays, radar state, and garrison muzzle flashes; common `rate_ms: 67`. | Building anims in GameMD use frame counters, modulo checks, CDTimer, and selective normalized helper. |
| `src/app_chute_anim.rs` | Parachute animation uses `elapsed_ms/rate_ms`. | Needs validation against actual GameMD chute/AnimClass frame delay path. |
| `src/app_fire_effects.rs` | Muzzle flashes use `elapsed_ms/rate_ms`. | GameMD muzzle/anim effects often use AnimClass or frame gates; ms may drift. |
| `src/app_instances/overlays.rs` | Terrain overlay frame uses `idle_anim_elapsed_ms / TERRAIN_ANIM_RATE_MS`, often `67` ms. | Overlay animation should be checked against frame-counter or art/rules timing path. |
| `src/app_instances/units.rs` | Facing samples `current(sim.binary_frame)`. | RateTimer should sample the same authoritative GameMD frame counter used by simulation timers. |
| `src/sim/movement/mod.rs` | Movement update is passed `tick_ms`. | Locomotion can remain modern internally only if observable per-frame movement cadence matches GameMD. |

## Findings Matrix

| ID | Finding | Evidence | Active in YR | Confidence |
| --- | --- | --- | --- | --- |
| T-001 | The authoritative game frame increments late in `Main_Tick`, after logic/render/service calls. | `0055D360` | Yes | High |
| T-002 | Local modes use `GetRadarTimer() = timeGetTime() >> 4` for throttle buckets. | `006C8C40`, `0055D360`, `0055E160` | Yes | High |
| T-003 | Standard YR absent-skirmish speed fallback is rulesmd `GameSpeed=1`, not options default `3`. | `00671EA0`, `00697F10`, INI scan | Yes | High |
| T-004 | In-game speed slider maps as `6 - TBM_GETPOS`. | `004E1DE0` | Yes | High |
| T-005 | Network/session option packets can write the same live speed byte used by the throttle. | `005B67F0` | Multiplayer | High |
| T-006 | `CDTimer` expires when `elapsed >= duration`; `start == -1` returns raw duration. | `00426630` | Yes | High |
| T-007 | `RateTimer` interpolates low 16-bit target values over frame duration using integer division by rate. | `004C9220`, `004C93D0` | Yes | High |
| T-008 | Anim `Rate` and `RandomRate` convert from INI by `900 / rate`, with values below 1 becoming 0. | `00427D00` | Yes | High |
| T-009 | `Normalized=yes` uses a speed-byte-dependent helper with a special table for delays 1 through 4. | `005FB2E0`, prior timing report | Yes | High |
| T-010 | Anim trailer spawning is frame modulo based on `g_CurrentFrameCounter % TrailerSeperation`. | `00423AC0` | Yes | High |
| T-011 | Infantry action normalization applies only to action IDs 9, 10, 0x12, 0x13, 0x17, and 0x20. | `0051D6F0` | Yes | High |
| T-012 | Bridge shroud recalculation runs every `0x78` frames from per-tick logic. | `0055AFB0` | Yes | High |
| T-013 | Foot idle scatter gate runs every 64 frames at low-byte mask value `0x3F`. | `004DA530` | Yes | High |
| T-014 | Temporal visual state transitions use fixed frame durations and external CDTimer thresholds. | `0070E5A0` | Conditional | High |
| T-015 | Gap visual state transitions use fixed frame durations and external CDTimer thresholds. | `0070E920` | Conditional | High |
| T-016 | Building repair uses `g_CurrentFrameCounter % interval == 0`, not elapsed milliseconds. | `00450630` | Yes | High |
| T-017 | Some building animation production runs every `0x18` frames. | `0043FB20` | Conditional | High |
| T-018 | Factory production progress has 54 steps (`0x36`) and each step is frame-timer driven. | `004C9B20`, `004C9EA0` | Yes | High |
| T-019 | Factory step duration is `GetBuildStepTime() / 0x36`, clamped to `1..0xFF`. | `004C9EA0`, `004C9FB0`, `004CA6E0` | Yes | High |
| T-020 | Temporal damage progression decrements per update by source/chain damage rather than a CDTimer countdown. | `0071A760` | Conditional | High |
| T-021 | Sidebar action animation counters are per-call counters in `SidebarClass__Action`, not direct frame-counter timers in that function. | `006A7780` | Yes | High |
| T-022 | Scenario delay/render-only branch can run services/render without advancing `g_CurrentFrameCounter`. | `0055D360` | Conditional | High |
| T-023 | Network mode can increase wait budget by remote backlog thresholds. | `0055D360` | Multiplayer | High |
| T-024 | Gap-generator fade tick increments/decrements one byte per call and dirties draw at fixed byte values. | `00454DB0` | Conditional | High for visible early state machine |

## Integration Guidance

This report does not prescribe a full implementation, but it narrows the necessary parity decisions:

1. Treat the GameMD frame counter as a first-class simulation concept, separate from render delta time and app scheduler time.
2. Preserve the late-increment behavior: systems called during frame N should observe frame N until the end of the active main tick.
3. Model local speed as frame-throttle budget sourced from session/rules speed byte and `GetRadarTimer` buckets, not as a universal elapsed-ms multiplier.
4. Implement CDTimer and RateTimer in frame units and migrate consumers that currently use elapsed milliseconds where they map to GameMD timers.
5. Implement the normalized animation helper exactly, including the small-delay table, and apply it only on the verified paths.
6. Classify each animation/effect as one of: AnimClass delay, infantry sequence delay, building anim timer, CDTimer/RateTimer, modulo frame gate, per-call decrement, or true render-only interpolation.
7. Audit all Rust `rate_ms: 67`, `elapsed_ms`, and synthetic `binary_frame` uses against the taxonomy above before trusting visible cadence.

## Open Follow-Up Slices

The global model is now substantially clearer, but a few areas deserve focused slices before implementation:

1. **Superweapon timing:** `SuperClass__Launch` is too broad for one timing pass. Each superweapon should get a launch/recharge/visual timing trace.
2. **Factory build-time formula:** The integration points are clear, but exact `006F47A0` arithmetic should be isolated and verified with example objects/houses/power states.
3. **Gap generator behavior:** `00454DB0` should be rechecked with tighter decompilation and call-site context because the latter half has unreliable locals.
4. **Overlay and terrain animation:** Current Rust uses elapsed-ms terrain overlay animation; GameMD overlay animation path should be traced separately.
5. **Locomotor cadence:** `FootClass__Locomotion_AI` confirms per-tick dispatch, but individual locomotor classes need their own timing traces for movement parity.
6. **Network timing:** Multiplayer frame budget and command queue timing should be researched separately if deterministic netplay parity is a target soon.

## Plan Coverage Notes

The planned address set was covered. Two planned labels resolved under different decompiler names:

- `004C9EA0` resolved as `FactoryClass__SetRate`, not "CompletionStep".
- `006F47A0` resolved as `FactoryClass__GetBuildStepTime`, not "GetProductionSpeed".

Additional caution notes:

- `0055DEE0` `LogicClass__AI` has a decompiler-ambiguous signature and is not the full object AI loop implied by the name.
- `00454DB0` `BuildingClass__UpdateGapGenerator_Tick` has unreliable decompiler locals in the latter half.
- `006CC390` `SuperClass__Launch` is a large switch; this report classifies timing dependencies but does not fully document every launch variant.

## Related Existing Docs

- `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`
- `docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
- `docs/research/VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md`

