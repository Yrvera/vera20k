# Tick Speed and Animation Play Speed - Ghidra Research Report

**Addresses:** `0x0055D360` (`Main_Tick`), `0x0055E160` (main throttle wait helper), `0x0055AFB0` (`LogicClass__PerTickUpdate`), `0x00423AC0` (`AnimClass__AI`), `0x00421EA0` (`AnimClass__Constructor`), `0x00427D00` (`AnimTypeClass__ReadINI`), `0x005FB2E0` (normalized-rate helper), `0x00426630` (`CDTimerClass__GetTimeRemaining`), `0x004C9220` (`RateTimer__Set`), `0x004C93D0` (`RateTimer__Current`), `0x006C8C40` (`GetRadarTimer`), `0x004DA530` (`FootClass__AI`), `0x007360C0` (`UnitClass__AI`), `0x00736CA0` (UnitClass vtable `+0x424` ammo/reload tick wrapper), `0x006FB010` (TechnoClass vtable `+0x424` ammo/reload tick), `0x006FB080` (ammo reload helper), `0x00736DF0` (`UnitClass__Fire_At_Target`), `0x0073C5F0` (`UnitClass__Draw_Body_And_Turret`), `0x00747620` (`UnitTypeClass__ReadINI`), `0x0051BAB0` (`InfantryClass__AI`), `0x0051D6F0` (`InfantryClass__Do_Action`), `0x00520AE0` (`InfantryClass__DoType_Sequencer`), `0x0070E5A0` (`TechnoClass__UpdateTemporalVisual`), `0x0070E920` (`TechnoClass__UpdateGapVisual`)
**Confidence:** High for `g_CurrentFrameCounter`, `CDTimerClass`, `AnimClass` rate conversion, normalized small-rate lookup values, `RateTimer` interpolation math, SHP vehicle `WalkRate`/`IdleRate` frame-counter gating, infantry action timer setup/consumption, ammo/reload frame timers, Techno temporal/gap visual state machines, and current Rust divergence. Medium for the final infantry SHP draw-frame mapping; this pass verified action timer setup, timer consumption, sequence completion, and weapon-fire trigger frames, but did not fully reduce every facing-mode token into final image-frame formulas.
**Active in YR:** Yes. These are standard game-loop, timer, options, and `AnimClass` paths used in normal Yuri's Revenge skirmish.

## 1. Overview

`gamemd.exe` has one authoritative gameplay frame counter: `g_CurrentFrameCounter` at `0x00A8ED84`. Most countdown timers store a start frame and duration, then compute remaining time from this counter. The counter increments once near the end of `Main_Tick`, after logic, map logic, render, per-tick side work, and network/service processing.

Animation playback for `AnimClass` is frame-counter driven, not render-delta driven. `Rate=` in art INI is converted to an internal delay in game frames using integer division `900 / Rate`. If `Normalized=yes`, the delay is further adjusted by the current game-speed setting through `0x005FB2E0`, so normalized animations try to maintain more stable wall-clock speed across the game-speed slider.

## 2. Key Offsets and Globals

| Field/global | Offset/address | Purpose | Evidence | Confidence | Active in YR |
|---|---:|---|---|---|---|
| `g_CurrentFrameCounter` | `0x00A8ED84` | Global gameplay frame counter used by timers and modulo gates | `Main_Tick`, `CDTimerClass__GetTimeRemaining`, `AnimClass__AI` | High | Yes |
| `DAT_00A8EB60` | global | Current game-speed setting used by the main tick throttle and normalized animation helper | `Main_Tick`, options dialog, `0x005FB2E0` via `ECX` | High | Yes |
| `AnimClass::CurrentFrame` | `this+0x0AC` (`[0x2B]`) | Current anim-relative frame | `AnimClass__AI`, `AnimClass__Constructor` | High | Yes |
| `AnimClass::FrameAdvanced` | `this+0x0B0` (`[0x2C]`) | Set true only on a frame-advance tick | `AnimClass__AI` | High | Yes |
| `AnimClass::LastFrameTime` | `this+0x0B4` (`[0x2D]`) | Written to `g_CurrentFrameCounter` when frame advances | `AnimClass__AI`, constructor | High | Yes |
| `AnimClass::FrameDelay` | `this+0x0BC` (`[0x2F]`) | CDTimer duration currently counting down | `AnimClass__AI`, constructor | High | Yes |
| `AnimClass::FrameDelayReload` | `this+0x0C0` (`[0x30]`) | Reload delay after every frame advance | `AnimClass__AI`, constructor | High | Yes |
| `AnimClass::FrameStep` | `this+0x0C4` (`[0x31]`) | `+1` normally, negated for reverse/ping-pong | `AnimClass__AI`, constructor | High | Yes |
| `AnimType::Rate` | `type+0x2B0` | Internal frame delay, not raw INI milliseconds | `AnimTypeClass__ReadINI` | High | Yes |
| `AnimType::RandomRate` | `type+0x2E4/+0x2E8` | Internal min/max frame delays after conversion | `AnimTypeClass__ReadINI`, constructor | High | Yes |
| `AnimType::Normalized` | `type+0x362` | Enables `0x005FB2E0` game-speed normalization | `AnimTypeClass__ReadINI`, constructor | High | Yes |
| `AnimType::LoopEnd` | `type+0x2BC` (`700` decimal in decompile) | If `-1`, filled from `End` during AI/constructor | `AnimClass__AI`, constructor | High | Yes |
| `FootClass::BodyFrameCounter` | `this+0x538` (`param_1[0x14E]`) | Shared per-foot animation frame counter used by SHP vehicles and body draw code | `FootClass__AI`, `UnitClass__Draw_Body_And_Turret` | High | Yes |
| `TechnoType::WalkRate` | `type+0x294` | Modulo divisor for moving body-frame increments | `TechnoTypeClass__ReadINI`, `FootClass__AI` | High | Yes |
| `TechnoType::IdleRate` | `type+0x298` | Optional modulo divisor for idle body-frame increments | `TechnoTypeClass__ReadINI`, `FootClass__AI` | High | Yes |
| `UnitType::WalkFrames` | `type+0xE5C` byte | Number of walk frames per facing for SHP unit bodies | `UnitTypeClass__ReadINI`, `UnitClass__Draw_Body_And_Turret` | High | Yes for SHP units |
| `UnitType::FiringFrames` | `type+0xE5D` byte | Number of firing frames per facing for SHP unit bodies | `UnitTypeClass__ReadINI`, `UnitClass__Draw_Body_And_Turret`, `UnitClass__Fire_At` docs | High | Yes for SHP units |
| `InfantryType::SequenceTable` | `type+0xE3C` | 24-byte per-action sequence entries parsed from art `Sequence=` section | `FUN_00523D00`, `InfantryClass__DoType_Sequencer`, `InfantryClass__Do_Action` | Medium | Yes |
| `InfantryClass::DoingAction` | `this+0x6C4` (`param_1[0x1B1]`) | Current infantry action/sequence id | `InfantryClass__Do_Action`, `DoType_Sequencer` | High | Yes |
| `InfantryClass::DoingFrame` | `this+0x0F8` (`param_1[0x3E]`) | Current frame index within the current infantry action sequence | `InfantryClass__Do_Action`, `DoType_Sequencer` | Medium | Yes |
| `InfantryClass::ActionTimer` | `this+0x100..0x10C` | CDTimer-style start/duration/reload for infantry action frame cadence | `InfantryClass__Do_Action` | High | Yes |

## 3. Game Tick Cadence

### 3.1 Counter ordering

`Main_Tick` calls gameplay and render work before incrementing `g_CurrentFrameCounter`:

1. Input is processed by `GScreenClass__Input`.
2. Command/event input is processed by `LogicClass__AI`.
3. `House_AI_Tick` may run before render in some modes.
4. `Map__Logic` marks occupied cells dirty.
5. `RenderFrame_main` draws the frame.
6. `FUN_00551A30`, `LogicClass__PerTickUpdate`, radar/audio/network/service routines, and cleanup run.
7. If not paused or blocked by stop flags (`DAT_00A83D49`, `DAT_00A8ECD0`, `DAT_008B41C0`, `DAT_00A83D48`), `g_CurrentFrameCounter++`.

**Finding:** `CDTimerClass` users inside the tick see the old frame number. A timer started during tick `N` with `start_frame=N` will not observe `N+1` until the next `Main_Tick` completes. Evidence: `Main_Tick @ 0x0055D360`; increment appears at the end after `Network_ServiceLoop`.

### 3.2 Time units

`GetRadarTimer @ 0x006C8C40` returns `timeGetTime() >> 4`, so its unit is 16 ms buckets. `Main_Tick` stores the game-speed setting into `DAT_00887350`; later `FUN_0055E160` sleeps against `GetRadarTimer`, so a game-speed value of `N` corresponds to approximately `N * 16 ms` of throttle in the single-player path.

**Important detail:** the in-game options dialog writes `DAT_00A8EB60 = 6 - slider_position` for control `0x529`. Higher UI slider positions therefore produce lower stored delay values. Evidence: `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0`.

**Default detail:** `OptionsClass__SetDefaults @ 0x005FA350` initializes `Options.GameSpeed` at offset `+0x00` to `3`. `OptionsClass__ReadFromINI @ 0x005FA620` reads `[Options] GameSpeed=` directly into that same field.

### 3.3 `CDTimerClass`

`CDTimerClass__GetTimeRemaining @ 0x00426630`:

```text
duration = timer[2]
if timer[0] != -1:
    elapsed = g_CurrentFrameCounter - timer[0]
    if elapsed < duration:
        return duration - elapsed
    return 0
return duration
```

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| `start_frame == -1` means paused/not-started; remaining time is raw `duration` | `0x00426630` | High | Yes |
| `duration == 0` is immediately expired | `0x00426630` | High | Yes |
| No self-decrement occurs; all countdown behavior is computed from global frame delta | `0x00426630` | High | Yes |
| Comparison is `elapsed < duration`, so `elapsed == duration` returns zero | `0x00426630` | High | Yes |

### 3.4 Main throttle wait helper

`FUN_0055E160 @ 0x0055E160` is called from `Main_Tick` after the gameplay frame counter increments, and also from the scenario-delay render-only branch. It waits against two different timer units:

```text
DAT_00887348 / DAT_00887350: GetRadarTimer() buckets, 16 ms each
DAT_00887328 / DAT_00887330: timeGetTime() milliseconds, used by network/multiplayer pacing
```

Single-player/menu style waiting uses `GetRadarTimer()`, therefore `DAT_00887350=3` means roughly three 16 ms buckets, not three milliseconds. Network modes use `timeGetTime()` and a millisecond frame budget in `DAT_00887330`.

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| `Main_Tick` sets `DAT_00887348 = GetRadarTimer()` before the wait helper, then `FUN_0055E160` subtracts elapsed buckets from `DAT_00887350` | `Main_Tick`, `FUN_0055E160` | High | Yes |
| In game mode `0`, if `DAT_00A8EDDC == 0`, `Main_Tick` temporarily writes both `DAT_00A8EB60` and `DAT_00887350` to `2` | `Main_Tick @ 0x0055D44F..0x0055D492` | High | Conditional |
| In non-mode-0/non-mode-5 with `DAT_00A8B24C == 2`, replay/network style path can set `DAT_00887350 = 0`, `2`, or `0x3c / DAT_00A8B558` | `Main_Tick @ 0x0055D4FA..0x0055D585` | High | Conditional |
| Network game mode `4` can add `10` ms to `DAT_00887330` up to three times based on remote frame budget thresholds `1/4`, `1/2`, and `3/4` | `Main_Tick @ 0x0055D5D0..0x0055D756` | High | Multiplayer |
| `FUN_0055E160` calls `Sleep(0)` inside the network wait loop, but `Sleep(DVar)` in the single-player bucket wait loop | `FUN_0055E160` | High | Yes |
| When `DAT_00887348 == -1`, the single-player helper sleeps the raw `DAT_00887350` value as milliseconds; the normal initialized path uses bucket-relative subtraction first | `FUN_0055E160` | High | Edge/initialization |

## 4. AnimType Rate Parsing

`AnimTypeClass__ReadINI @ 0x00427D00` reads `Rate=` as an integer. If present:

```text
if ini_rate < 1:
    internal_rate = 0
else:
    internal_rate = 900 / ini_rate     // integer division
type->Rate = internal_rate
```

The same conversion is applied to `RandomRate=min,max` independently for each endpoint, except an endpoint of `-1` means "not specified" and is not converted. After conversion:

```text
RandomRate.Max = max(RandomRate.Max, 0)
if RandomRate.Max < RandomRate.Min:
    RandomRate.Min = RandomRate.Max
```

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| The magic `900` is frames per minute at the engine's intended 15 game frames/sec (`60 * 15`) | `ReadINI` conversion and many rules timers using minute-to-frame conversions | High | Yes |
| Conversion truncates, it does not round | integer division in `0x00427D00` | High | Yes |
| `Rate=200` becomes `4` frames, not `4.5` | `900 / 200` integer division | High | Yes |
| `Rate=400` becomes `2` frames, not `2.25` | `900 / 400` integer division | High | Yes |
| `Rate=120` becomes `7` frames | `900 / 120` integer division | High | Yes |
| `Rate<=0` stores `0`; in `AnimClass__AI`, `FrameDelayReload == 0` blocks normal frame advancement | `0x00427D00`, `0x00423AC0` | High | Yes |
| Constructor default before INI parsing is `Rate=1` internal frame, not raw `Rate=1` INI | `AnimTypeClass__Constructor @ 0x00427530` | High | Yes |

## 5. Normalized Animation Rate

When `AnimType::Normalized` (`type+0x362`) is true, both `AnimClass__Constructor` and the in-place `Next=` transition path call `0x005FB2E0` on the already-converted internal frame delay.

Assembly at `0x005FB2E0`:

```text
rate = [esp+4]
if rate == 0:
    return 0
game_speed = [ecx]          // Options/GameSpeed-style value
if rate < 5:
    return small_rate_table[game_speed + rate * 8]
return (rate << 3) / (game_speed + 1)
```

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| Normalization is applied after `900 / Rate`, not to raw INI `Rate=` | `AnimTypeClass__ReadINI`, constructor | High | Yes |
| Normalization is applied to random rates too, because constructor first selects `RandomRate` then normalizes the selected delay | `AnimClass__Constructor`, `AnimClass__AI` `Next=` path | High | Yes |
| `rate == 0` stays zero | `0x005FB2E0` | High | Yes |
| For `rate >= 5`, formula is `(rate * 8) / (game_speed + 1)`, integer division | `0x005FB2E0` assembly | High | Yes |
| For `rate < 5`, a hardcoded lookup table is used instead of the division formula | `0x005FB2E0` assembly and data at `0x00832CEC` | High | Yes |
| `Normalized=no` anims are not compensated for game speed; their wall-clock speed changes with the tick throttle | Constructor branch skips `0x005FB2E0` | High | Yes |

The small-rate table matters because many common art rates convert below 5 frames: `Rate=200 -> 4`, `Rate=300 -> 3`, `Rate=400 -> 2`, `Rate=450 -> 2`. A faithful implementation needs the table, not only the `rate >= 5` formula.

Dumped normalized-delay table at `0x00832CEC`, indexed as:

```text
if internal_delay == 0: return 0
if internal_delay < 5: return table[internal_delay][stored_game_speed]
else: return (internal_delay * 8) / (stored_game_speed + 1)
```

| Internal delay before normalization | Stored game speed 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 2 | 2 | 1 | 1 | 1 | 1 | 1 | 1 |
| 2 | 3 | 3 | 3 | 2 | 2 | 2 | 1 | 1 |
| 3 | 5 | 4 | 4 | 3 | 3 | 2 | 2 | 1 |
| 4 | 7 | 6 | 5 | 4 | 4 | 4 | 3 | 2 |

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| Row 0 at `0x00832CEC` is not used by the helper because `rate == 0` returns before indexing | `0x005FB2E0` branch order | High | Yes |
| The helper allows stored game-speed index 0..7; no clamp exists inside `0x005FB2E0` | `MOV ECX,[ECX]`, direct indexed load | High | Yes, caller/options clamp separately |
| At `OptionsClass` constructor default `GameSpeed=3`, small delays normalize as `1->1`, `2->2`, `3->3`, `4->4` | `OptionsClass__SetDefaults`, table dump | High | Yes |
| If the scenario path temporarily forces stored game speed to `2`, small delays normalize as `1->1`, `2->3`, `3->4`, `4->5` | `FUN_0069BAB0`, table dump | High for table, Medium for standard skirmish reachability |

## 6. AnimClass Frame Advance

`AnimClass__AI @ 0x00423AC0` frame-advance sequence:

1. Early special systems run first: looping sound update, flaming-guy bounce AI, psi warning visibility, `HideIfNoOre`, `MakeInfantry` coordinate capture, bouncer collision handling, trailer spawning, tiberium overlay validation, and End/LoopEnd auto-fill.
2. Visibility is updated through vtable `+0x124`.
3. If `this+0x19E` or `this+0x11A` is set, frame advancement returns early.
4. `CDTimerClass__GetTimeRemaining` is called on the timer embedded at `this+0x0B4`.
5. If remaining is nonzero, or `FrameDelayReload == 0`, `FrameAdvanced=false` and AI returns.
6. Otherwise:

```text
FrameAdvanced = true
CurrentFrame += FrameStep
LastFrameTime = g_CurrentFrameCounter
FrameDelay = FrameDelayReload
```

7. Only after this does per-frame damage accumulate, start-frame side effects trigger, ping-pong reverse, loop/end handling, `Next=`, `MakeInfantry`, or destruction run.

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| The first AI tick after construction returns without advancing when `this+0x19C` is set; constructor sets it to 1 and AI clears it | `AnimClass__AI`, constructor | High | Yes |
| Constructor calls `AnimClass__Middle()` immediately only when `Delay == 0` | constructor | High | Yes |
| Trailer anims spawn before normal frame advancement, gated by `g_CurrentFrameCounter % TrailerSeperation == 0` or separation `== 1` | `AnimClass__AI` | High | Yes |
| Trailer constructor uses delay `1`, not `0`, so the trailer waits one tick before `Middle()` | `AnimClass__AI` constructor call arguments | High | Yes |
| Per-frame damage is tied to frame advancement, not every game tick | `AnimClass__AI` order | High | Yes |
| `LastFrameTime` is written before damage and loop/end processing | `AnimClass__AI` | High | Yes |
| `FrameDelay` reload writes `FrameDelayReload` into timer duration but does not change `LastFrameTime` again later in the tick | `AnimClass__AI` | High | Yes |
| `LoopEnd == -1` is lazily filled from `End`; `End == -1` is filled from SHP frame count, halved if `Shadow=yes` | `AnimClass__AI`, constructor | High | Yes |
| `LoopCountRemaining` is a byte. Constructor multiplies `(byte)type->LoopCount * (byte)param_loop_count`, then clamps values `<2` up to `1` | constructor | High | Yes |
| `LoopCountRemaining == 0xFF` is infinite and is not decremented | `AnimClass__AI` | High | Yes |
| On a `Next=` transition, the same `AnimClass` object is reused; accumulated damage is reset, type changes, loop count and rate are reloaded, `CurrentFrame` becomes the next type's `Start`, and `Middle()` is called | `AnimClass__AI` | High | Yes |

## 7. SHP Unit Body Animation

SHP vehicles and other foot-derived sprite bodies are not driven by `AnimType::Rate`. Their visible body frame is based on `FootClass::BodyFrameCounter` (`this+0x538`) and type-level `WalkRate`/`IdleRate` fields.

`TechnoTypeClass__Constructor @ 0x00710AF0` initializes:

```text
TechnoType+0x294 WalkRate = 1
TechnoType+0x298 IdleRate = 0
```

`TechnoTypeClass__ReadINI @ 0x00712170` reads `WalkRate=` and `IdleRate=` as raw integers into those same fields. There is no `900 / value` conversion and no normalized-rate helper on this path.

`FootClass__AI @ 0x004DA530` increments `this+0x538` only after locomotor processing and only if the unit is eligible for body animation. The moving path is gated by:

```text
if g_CurrentFrameCounter % type->WalkRate == 0
    and not moving/falling/other blocked states:
        BodyFrameCounter += 1
```

The idle path is separate:

```text
if type->IdleRate != 0
    and locomotor is not currently moving
    and g_CurrentFrameCounter % type->IdleRate == 0
    and not firing/falling/other blocked states:
        BodyFrameCounter += 1
```

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| `WalkRate=1` means eligible moving SHP body frames can advance every game frame | `TechnoTypeClass__Constructor`, `FootClass__AI` modulo | High | Yes |
| `IdleRate=0` disables the idle-specific modulo path entirely; it does not mean "every frame" | `FootClass__AI` checks `type+0x298 != 0` before modulo | High | Yes |
| `WalkRate`/`IdleRate` are raw frame modulo divisors, not art `Rate=` values and not wall-clock milliseconds | `TechnoTypeClass__ReadINI`, `FootClass__AI` | High | Yes |
| There is no zero guard around `WalkRate` before modulo in the decompiled moving path; retail defaults and INI comments imply content must keep it positive | `FootClass__AI`, rules comments | Medium | Yes |
| The old body counter value is saved before the increment; sound/start effects later compare old vs new counter | `iVar3 = param_1[0x14E]` before increment, then `if (iVar3 == param_1[0x14E]) ... else ...` | High | Yes |
| Rank/falling/looping sound side effects in `FootClass__AI` run after the body-frame increment decision, not before it | `FootClass__AI` order | High | Yes |

`UnitTypeClass__Constructor @ 0x007470D0` initializes SHP unit frame layout fields:

```text
StandingFrames   type+0xE1C = 0
DeathFrames      type+0xE20 = 0
DeathFrameRate   type+0xE24 = 1
StartStandFrame  type+0xE28 = -1
StartWalkFrame   type+0xE2C = -1
StartFiringFrame type+0xE30 = -1
StartDeathFrame  type+0xE34 = -1
MaxDeathCounter  type+0xE38 = -1
Facings          type+0xE3C = 8
WalkFrames       type+0xE5C = 12
FiringFrames     type+0xE5D = 0
```

`UnitTypeClass__ReadINI @ 0x00747620` then computes defaults:

```text
if FiringFrames > 0:
    StandingFrames = 1

if DeathFrameRate < 1:
    DeathFrameRate = 1

if FiringFrames == 0 and Turret == false:
    Facings = 1

if StartWalkFrame == -1:
    StartWalkFrame = 0

if StartStandFrame == -1:
    if StandingFrames == 0:
        StartStandFrame = StartWalkFrame
    else:
        StartStandFrame = WalkFrames * Facings

if StartFiringFrame == -1:
    if FiringFrames == 0:
        StartFiringFrame = StartStandFrame
    else:
        StartFiringFrame = (WalkFrames + StandingFrames) * Facings

if StartDeathFrame == -1:
    if DeathFrames == 0:
        StartDeathFrame = -1
    else:
        StartDeathFrame = (FiringFrames + 1 + WalkFrames) * Facings
    MaxDeathCounter = StartDeathFrame + DeathFrames
```

`UnitClass__Draw_Body_And_Turret @ 0x0073C5F0` consumes those fields:

```text
if firing_counter >= 0:
    frame = StartFiringFrame + FiringFrames * facing + firing_counter / 2
else if not moving and StandingFrames == 0:
    if death_counter >= 0:
        frame = StartDeathFrame + min(death_counter / DeathFrameRate, DeathFrames - 1)
    else if special standing flag:
        frame = StartStandFrame + standing_frame_group
    else:
        frame = StartWalkFrame + WalkFrames * facing
else:
    frame = StartWalkFrame + (BodyFrameCounter % WalkFrames) + WalkFrames * facing
```

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| `FiringFrames` and `WalkFrames` are stored as signed bytes in the decompile (`char` reads) even though INI values are read through `ReadInt` | `UnitTypeClass__ReadINI`, `UnitClass__Draw_Body_And_Turret` | High | Yes |
| Firing body animation uses `firing_counter / 2`, so each firing SHP frame persists for two counter steps | `UnitClass__Draw_Body_And_Turret` | High | Yes |
| `UnitClass__Fire_At` sets the firing animation counter to `FiringFrames * 2 - 1` when the type has firing frames | `FIRE_AT_PIPELINE_GHIDRA_REPORT.md`, `UnitClass__Draw_Body_And_Turret` | High | Yes |
| Death animation frame clamps at `DeathFrames - 1`, so it never indexes past the declared death range | `UnitClass__Draw_Body_And_Turret` | High | Yes |
| `DeathFrameRate < 1` is clamped up to `1` during `ReadINI` | `UnitTypeClass__ReadINI` | High | Yes |
| Non-turret unit types with `FiringFrames==0` force `Facings=1` before `Facings=` is read, then `Facings=` can override it | `UnitTypeClass__ReadINI` order | High | Yes |

Retail INI hits confirmed in this pass:

| Section | INI | Values |
|---|---|---|
| SHP vehicle examples | `artmd.ini` | `WalkFrames=6`, `FiringFrames=6`; `WalkFrames=6`, `FiringFrames=4`; `WalkFrames=20`, `FiringFrames=16` |
| SHP unit hacks | `rulesmd.ini` | `WalkRate=4`, `IdleRate=8`; `WalkRate=2`, `IdleRate=4` |

## 8. Infantry Sequence Timing

Infantry use a third timing path. Their `Sequence=` name in art INI points to a section containing named action entries such as `Ready`, `Walk`, `FireUp`, `FireProne`, `Prone`, `Crawl`, `Up`, and `Down`.

`InfantryTypeClass__ReadSequenceData @ 0x00523D00` reads the art `Sequence=` key from the infantry image section, then reads each named action from the sequence section. Each sequence entry is parsed with:

```text
"%d,%d,%d,%s"
```

The three integers are written into the 24-byte action entry at:

```text
entry+0x00
entry+0x04
entry+0x08
```

The trailing string is an optional completion-facing hint stored at `entry+0x0C`; the constructor initializes it to `-1`. Live string bytes at `0x008258BC` and the reader's comparison order give this mapping:

| Token | Stored value | Evidence |
|---|---:|---|
| `N` | 0 | native direction index; snap value `0x0000` |
| `NE` | 1 | native direction index; snap value `0x2000` |
| `E` | 2 | native direction index; snap value `0x4000` |
| `SE` | 3 | native direction index; snap value `0x6000` |
| `S` | 4 | native direction index; snap value `0x8000` |
| `SW` | 5 | native direction index; snap value `0xA000` |
| `W` | 6 | native direction index; snap value `0xC000` |
| `NW` | 7 | native direction index; snap value `0xE000` |

On default action completion, `InfantryClass__DoType_Sequencer @ 0x00520AE0`
(`0x00520CEB..0x00520D16`) reads the completed entry's hint, skips `-1`, calls
`FacingClass__UpdateFacing(hint << 13)` on the infantry body-facing receiver, and only then
dispatches the next/default action. `FacingClass__UpdateFacing` resets its timer epoch and
clears duration even when the destination already matches.

`InfantryClass__Do_Action @ 0x0051D6F0` starts a new action and configures an action timer:

```text
DoingAction = action_id

if action_id in {9, 10, 0x12, 0x13, 0x17, 0x20}:
    delay = Normalized(action_delay_table[action_id])
else:
    delay = action_delay_table[action_id]

ActionTimer.start = g_CurrentFrameCounter
ActionTimer.duration = delay
ActionTimer.reload = delay

if random_start == false:
    DoingFrame = 0
else:
    DoingFrame = RandomRanged(0, max(sequence_frame_count, 1) - 1)
```

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| Only six infantry actions use the normalized-rate helper: `9`, `10`, `0x12`, `0x13`, `0x17`, `0x20` | `InfantryClass__Do_Action @ 0x0051D9CF..0x0051DA02` | High | Yes |
| Other infantry actions use the raw one-byte action delay table value directly | `InfantryClass__Do_Action` else path | High | Yes |
| `Do_Action(-1, ...)` returns false immediately | `InfantryClass__Do_Action` first branch | High | Yes |
| If the target sequence's frame count is zero, `Do_Action` returns false and does not change the current action | `InfantryClass__Do_Action` reads `SequenceTable[action]+0x04` | High | Yes |
| Action `0x21` refuses to start if the infantry is currently cloaked/hidden via byte `this+0x8D` | `InfantryClass__Do_Action` | Medium | Conditional |
| `random_start` clamps frame count below 2 up to 1 before `RandomRanged(0, count - 1)`, preventing a negative upper bound | `InfantryClass__Do_Action @ 0x0051DA52..0x0051DA84` | High | Yes |
| Sequence sound triggers are limited to two entries per action; extra parsed sound pairs are ignored | `FUN_00523D00` checks `puVar7[1] < 2` | High | Yes |
| Sequence sound frames use modulo against sequence frame count, but if frame count is below 2 they use 1 | `InfantryClass__DoType_Sequencer` sound loop | High | Yes |

The action delay table near `0x007EAF7C` is byte-packed into 4-byte action records. The delay byte read by `Do_Action` is at `0x007EAF7F + action_id * 4`; the action-blocking flag read earlier is at `0x007EAF7C + action_id * 4`. This table is independent from art `Rate=`.

Additional infantry timing findings from this reinvestigation:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| `InfantryClass__AI` calls `FootClass__AI` before `InfantryClass__Fire_At_Target`, then `InfantryClass__DoType_Sequencer`, then `FootClass__Locomotion_AI` | `InfantryClass__AI @ 0x0051BAB0` | High | Yes |
| `DoType_Sequencer` returns to normal sequencing only while `DoingFrame < sequence.frame_count`; equal or greater is treated as action-complete | `InfantryClass__DoType_Sequencer` first branch | High | Yes |
| Completed death actions `0x0B..0x0F` may spawn a death anim before object destruction; custom corpse anim list at `type+0xE54/+0xE60` overrides rules fallback | `InfantryClass__DoType_Sequencer` cases `0x0B..0x0F` | High | Yes |
| Completed actions `0x14`, `0x15`, and `0x24` clear/destroy through vtable `+0xF8` once their sequence frame count is reached | `InfantryClass__DoType_Sequencer` | High | Yes |
| Action `0x21` is force-returned to action `0` if byte `this+0x8D` is clear after sequencing | `InfantryClass__DoType_Sequencer` tail | Medium | Conditional |
| Weapon firing for infantry is keyed to a sequence frame, not action start. `InfantryClass__Fire_At_Target` fires only when `DoingFrame == selected_fire_frame` | `InfantryClass__Fire_At_Target @ 0x005206B0` | High | Yes |
| Primary/secondary/elite/prone fire actions pick fire-frame fields from the infantry type area around `type+0xE40..0xE4C`; crouched/alternate path switches between those fields based on byte `this+0x6EF` and weapon slot | `InfantryClass__Fire_At_Target` | Medium | Yes |
| Some infantry fire setup writes `DoingFrame=0` directly instead of starting a new `Do_Action` when already in selected actions `0x28/0x29` and a timer/target condition is active | `InfantryClass__Fire_At_Target` | Medium | Conditional |
| `InfantryClass__UpdateIdleAction` starts idle actions `9` and `10` through vtable `+0x558`, so those idle fidgets are in the six-action normalized set | `InfantryClass__UpdateIdleAction @ 0x0051CDB0`, `Do_Action` normalization branch | High | Yes |

## 9. RateTimer, Facing, and Ammo Reload Timing

`RateTimer__Set @ 0x004C9220` and `RateTimer__Current @ 0x004C93D0` are another important timing family. They are not `AnimType::Rate` timers. They interpolate facing-like 16-bit values over a duration derived from angular distance and a per-timer rate field at `timer+0x14`.

Observed layout from the two functions:

```text
timer+0x00 current target packed value
timer+0x04 previous/source packed value
timer+0x08 CDTimer start frame
timer+0x0C CDTimer side field / high dword residue
timer+0x10 CDTimer duration
timer+0x14 rate/divisor
```

`RateTimer__Set(new_target)`:

```text
if current_target == new_target:
    return 0

source = Current(timer) if existing timer still has time, else current_target
previous/source = source
current_target = new_target

if rate > 0:
    start = g_CurrentFrameCounter
    duration = abs(new_target.low16 - source.low16) / rate
return 1
```

`RateTimer__Current(out)`:

```text
if rate < 1:
    return current_target

remaining = CDTimerRemaining(start, duration)
if remaining == 0:
    return current_target

delta = target.low16 - source.low16
step_count = abs(delta) / rate
if step_count < 1:
    return current_target

out.low16 = target.low16 - (delta / step_count) * remaining
out.high16 = target.high16
```

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| `RateTimer` interpolates only the low 16 bits in the decompiled math; the high 16 bits are copied from the target | `RateTimer__Current`, `RateTimer__Set` | High | Yes |
| If `rate < 1`, `RateTimer__Current` snaps to the target and `RateTimer__Set` does not start a new CDTimer | `RateTimer__Current`, `RateTimer__Set` | High | Yes |
| Duration is `abs(delta) / rate`, integer division; small rotations below one rate unit can become zero-duration snaps | `RateTimer__Set` | High | Yes |
| `RateTimer__Set` calls the current interpolation first when changing target mid-turn, so retargeting begins from the visible interpolated angle, not from the old target | `RateTimer__Set` | High | Yes |
| `RateTimer__Current` uses the same `elapsed < duration` boundary as `CDTimerClass`; at `elapsed == duration`, it returns the final target | `RateTimer__Current` | High | Yes |
| Callers include unit facing update, aircraft/building aiming, locomotion turn helpers, rocking/sinking visuals, and scatter direction selection | xrefs to `RateTimer__Set`/`Current` | High | Yes |

`UnitClass__Facing_Update @ 0x00736990` consumes `RateTimer` for turret/body target facing. Important ordering:

1. `UnitClass__Fire_At_Target` may set a facing target/timer first.
2. `UnitClass__Facing_Update` runs immediately after firing logic in `UnitClass__AI`.
3. `UnitClass__AI` then calls vtable slot `+0x424`, which for UnitClass is the function entry at `0x00736CA0`.

The UnitClass vtable `+0x424` entry at `0x007F6094` points to `0x00736CA0`. Ghidra did not name this as a separate function, but the bytes decode as a small wrapper around the shared Techno ammo/reload tick. It conditionally freezes/reloads a timer at `this+0x1FC..0x204`, then tail-calls `FUN_006FB010`.

Tiny details from `0x00736CA0`:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| UnitClass vtable base is `0x007F5C70`; AI is at base `+0x5C`; ammo/reload slot `+0x424` points to `0x00736CA0` | vtable data dump and `UnitClass__AI` call | High | Yes |
| If byte `unit_type+0x6AE` is nonzero, the wrapper skips its local timer-freeze logic and still calls `FUN_006FB010` | disassembly `0x00736CA7..0x00736D3E` | Medium | Conditional |
| If a manager/object at `this+0x674` exists and its vtable `+0x10` returns true, the wrapper rewrites timer `this+0x1FC..0x204` to preserve `remaining + 1` frames from the current frame | disassembly `0x00736CCF..0x00736D36` | Medium | Conditional |
| The preserved timer duration is explicitly `remaining + 1`, not raw remaining | `LEA EAX,[ECX+1]` at `0x00736D27` | High | Conditional |
| After the local timer adjustment, the wrapper always calls `FUN_006FB010` | `CALL 0x006FB010` at `0x00736D3E` | High | Yes |

`FUN_006FB010` and `FUN_006FB080` are shared Techno ammo/reload helpers used by multiple vtables, including InfantryClass and UnitClass:

```text
if type->Ammo == -1:
    return
if current_ammo >= type->Ammo:
    return
if reload_timer remaining != 0:
    return

current_ammo += 1
Mark/Redraw object
reload timer through FUN_006FB080
```

`FUN_006FB080` reloads the next delay:

```text
if current_ammo == 0 and type->EmptyReload != -1:
    duration = type->EmptyReload
else:
    if type->PipWrap == 0:
        group = 1
    else:
        group = current_ammo / type->PipWrap
    duration = type->Reload + type->ReloadIncrement * group * group
```

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| Current ammo is `this+0x2FC`; `TechnoClass__InitFromType` initializes it from `InitialAmmo`, falling back to `Ammo` when `InitialAmmo == -1` | `TechnoClass__InitFromType`, `TechnoTypeClass__ReadINI` | High | Yes |
| Type `Ammo == -1` disables the reload helper entirely | `FUN_006FB010` | High | Yes |
| Reload delay is quadratic by ammo group: `Reload + ReloadIncrement * group * group` | `FUN_006FB080` | High | Yes |
| Group uses integer division by `PipWrap`; if `PipWrap == 0`, group is forced to `1` | `FUN_006FB080`, `TechnoTypeClass__ReadINI @ 0x00714179..0x00714198` | High | Yes |
| Empty units can use a distinct `EmptyReload`, but only when `current_ammo == 0` and `EmptyReload != -1` | `FUN_006FB080`, `TechnoTypeClass__ReadINI` | High | Yes |
| The helper marks/redraws through vtable `+0x124` with argument `2` immediately after ammo increments | `FUN_006FB010` | High | Yes |
| `MobileFire=yes` (`type+0x6AE`) skips the UnitClass wrapper's local timer-preservation branch before the shared ammo reload tick | `0x00736CA0`, `TechnoTypeClass__ReadINI @ 0x0071481C..0x00714830` | Medium | Conditional |

## 10. Other Frame-Counter Visual State Machines

`TechnoClass__UpdateTemporalVisual @ 0x0070E5A0` and `TechnoClass__UpdateGapVisual @ 0x0070E920` are fixed frame-count state machines. They use CDTimer-style fields, not `AnimType::Rate`, and their durations are hardcoded frame counts.

Temporal visual phases (`this+0x198..0x1A4`, state at `this+0x1A4`):

| State transition | Duration / condition | Evidence | Confidence |
|---|---:|---|---|
| `0 -> 1` | `6` frames | `TechnoClass__UpdateTemporalVisual` | High |
| `1 -> 2` | `4` frames | same | High |
| `2 -> 3` | `RandomRanged(-5, 5) + 20` frames | same | High |
| `3 -> 4` | `8` frames | same | High |
| `4 -> 5` | `16` frames | same | High |
| `5 -> 6` | when external CDTimer remaining `< 0x36` | same | High |
| `6 -> 7` | when external CDTimer remaining `< 0x1F` | same | High |
| `7 -> 8` | `6` frames | same | High |
| `8 -> 9` | `4` frames | same | High |
| `9 -> 10` | `20` frames | same | High |

Gap visual phases (`this+0x1B4..0x1C0`, state at `this+0x1C0`) are almost the same, except the middle hold durations and thresholds differ:

| State transition | Duration / condition | Evidence | Confidence |
|---|---:|---|---|
| `0 -> 1` | `6` frames | `TechnoClass__UpdateGapVisual` | High |
| `1 -> 2` | `4` frames | same | High |
| `2 -> 3` | `RandomRanged(-5, 5) + 20` frames | same | High |
| `3 -> 4` | `0x40` frames | same | High |
| `4 -> 5` | `0x40` frames | same | High |
| `5 -> 6` | when external CDTimer remaining `< 0x9E` | same | High |
| `6 -> 7` | when external CDTimer remaining `< 0x1F` | same | High |
| `7 -> 8` | `6` frames | same | High |
| `8 -> 9` | `4` frames | same | High |
| `9 -> 10` | `20` frames | same | High |

Tiny details:

| Detail | Evidence | Confidence | Active in YR |
|---|---|---|---|
| If the temporal/gap condition is not active, the state is reset directly to `0`; timers are not allowed to continue invisibly | `TechnoClass__UpdateTemporalVisual`, `TechnoClass__UpdateGapVisual` first branches | High | Conditional |
| The random middle duration can be as low as `15` and as high as `25` frames because the call is `RandomRanged(-5, 5) + 20` | both functions | High | Yes |
| State `5` can loop back to `4` while the external effect has too much time remaining | both functions | High | Yes |
| These timings are game frames, because every phase uses `g_CurrentFrameCounter` CDTimer fields | both functions | High | Yes |

`TechnoClass__RockingUpdate @ 0x0070B570` is also frame-step based. Rocking/sinking changes `AngleRotatedForwards`, `AngleRotatedSideways`, and per-frame deltas once per AI/update call. It uses floating-point for visual tilt, but the cadence is still game-frame driven. When sinking, it samples `RateTimer__Current` and uses the facing bucket to choose whether the forward angle increases or decreases.

## 11. INI Keys

| Key | Scope | Binary field | Behavior | Evidence | Confidence |
|---|---|---:|---|---|---|
| `[Options] GameSpeed=` | user options | `Options+0x00`, copied/used through `DAT_00A8EB60` | Tick throttle and normalized animation divisor source | `OptionsClass__ReadFromINI`, `Main_Tick`, `0x005FB2E0` | High |
| `Rate=` | `[AnimType]` in art INI | `AnimType+0x2B0` | `900 / Rate` internal frame delay | `AnimTypeClass__ReadINI` | High |
| `RandomRate=` | `[AnimType]` | `AnimType+0x2E4/+0x2E8` | Min/max converted with same `900 / value` rule, selected at construction/Next | `ReadINI`, constructor | High |
| `Normalized=` | `[AnimType]` | `AnimType+0x362` | Applies game-speed normalization to internal frame delay | `ReadINI`, constructor | High |
| `RandomLoopDelay=` | `[AnimType]` | `AnimType+0x2DC/+0x2E0` | Random delay assigned to `AnimClass::Delay` after a loop wraps; values are frame counts, not `900/Rate` converted | `ReadINI`, `AnimClass__AI` | High |
| `LoopStart=` | `[AnimType]` | `AnimType+0x2B8` | Reset frame when looping forward | `ReadINI`, `AnimClass__AI` | High |
| `LoopEnd=` | `[AnimType]` | `AnimType+0x2BC` | End of looping range; if `-1`, defaults to `End` | `ReadINI`, `AnimClass__AI` | High |
| `LoopCount=` | `[AnimType]` | `AnimType+0x2C4` | Copied through byte multiplication into `LoopCountRemaining` | `ReadINI`, constructor | High |
| `Start=` / `End=` | `[AnimType]` | `AnimType+0x2B4/+0x2C0` | Frame range; `End=-1` auto-detects from SHP | `ReadINI`, constructor, AI | High |
| `WalkRate=` | `[TechnoType]` / rules | `TechnoType+0x294` | Raw game-frame modulo divisor for moving body frame increments | `TechnoTypeClass__ReadINI`, `FootClass__AI` | High |
| `IdleRate=` | `[TechnoType]` / rules | `TechnoType+0x298` | Raw game-frame modulo divisor for idle body frame increments; zero disables idle path | `TechnoTypeClass__ReadINI`, `FootClass__AI` | High |
| `WalkFrames=` | `[UnitType]` art section | `UnitType+0xE5C` byte | Frames per facing for SHP unit walking/body loop | `UnitTypeClass__ReadINI`, `UnitClass__Draw_Body_And_Turret` | High |
| `FiringFrames=` | `[UnitType]` art section | `UnitType+0xE5D` byte | Frames per facing for SHP firing overlay/body sequence | `UnitTypeClass__ReadINI`, `UnitClass__Draw_Body_And_Turret` | High |
| `StandingFrames=` | `[UnitType]` art section | `UnitType+0xE1C` | Standing frames count; auto-set to 1 when `FiringFrames>0` before explicit read | `UnitTypeClass__ReadINI` | High |
| `DeathFrames=` / `DeathFrameRate=` | `[UnitType]` art section | `UnitType+0xE20/+0xE24` | Death frame count and frame-rate divisor; rate clamped to at least 1 | `UnitTypeClass__ReadINI`, draw path | High |
| `StartStandFrame=` / `StartWalkFrame=` / `StartFiringFrame=` / `StartDeathFrame=` | `[UnitType]` art section | `UnitType+0xE28/+0xE2C/+0xE30/+0xE34` | Override auto-computed SHP unit frame offsets | `UnitTypeClass__ReadINI` | High |
| `Facings=` | `[UnitType]` art section | `UnitType+0xE3C` | Facing count used to stride SHP unit frame groups | `UnitTypeClass__ReadINI`, draw path | High |
| `Sequence=` | infantry art image section | `InfantryType+0xE3C` parsed table | Names the sequence section used for infantry actions | `FUN_00523D00`, art/artmd INI | High |
| `InitialAmmo=` | TechnoType rules/art | `type+0x680` | Initial current ammo; if `-1`, current ammo starts at `Ammo` | `TechnoTypeClass__ReadINI`, `TechnoClass__InitFromType` | High |
| `Ammo=` | TechnoType rules/art | `type+0x684` | Maximum ammo; `-1` disables shared reload helper | `TechnoTypeClass__ReadINI`, `FUN_006FB010` | High |
| `Reload=` | TechnoType rules/art | `type+0x698` | Base reload duration in game frames | `TechnoTypeClass__ReadINI`, `FUN_006FB080` | High |
| `EmptyReload=` | TechnoType rules/art | `type+0x69C` | Optional distinct reload duration when current ammo is zero | `TechnoTypeClass__ReadINI`, `FUN_006FB080` | High |
| `ReloadIncrement=` | TechnoType rules/art | `type+0x6A0` | Quadratic reload increment multiplier by ammo group | `TechnoTypeClass__ReadINI`, `FUN_006FB080` | High |
| `PipWrap=` | TechnoType rules/art | `type+0x3E4` | Divisor for ammo group in reload-increment formula; zero forces group `1` | `TechnoTypeClass__ReadINI`, `FUN_006FB080` | High |

## 12. Current Rust Implementation Status

The current Rust code is not parity-correct for this area yet.

| Area | Rust status | Parity issue |
|---|---|---|
| Global tick rate | `src/util/fixed_math.rs:51` sets `SIM_TICK_HZ = 45`; `src/app_types.rs:27` computes `SIM_TICK_MS = 1000 / 45 = 22` despite the stale `// 66ms` comment | Many systems still treat one Rust tick as one `gamemd` frame in comments or code. That is incompatible unless every frame-based timer is explicitly mapped to a synthetic binary frame. |
| Synthetic binary frame | `src/sim/world/mod.rs:1002` computes `binary_frame = total_sim_ms * 15 / 1000` | Good direction for frame-based parity, but only systems wired to `binary_frame` get it. Many systems still tick every Rust sim tick. |
| Entity sprite animation | `src/sim/animation.rs:375` advances by `dt_ms`; app passes `SIM_TICK_MS` from `src/app_sim_tick.rs:281` | Infantry/unit sprite sequences use hardcoded milliseconds, not `gamemd` `Sequence` timing verified here. This is a separate but adjacent parity gap. |
| Infantry sequence timing | `src/rules/infantry_sequence.rs:210` uses hardcoded `DEFAULT_*_TICK_MS` | No binary evidence in this report that infantry sequence frame rates use these values. Needs a separate `InfantryClass`/sequence timing investigation. |
| Infantry action normalization | `src/rules/infantry_sequence.rs` and `src/sim/animation.rs` do not model the action delay table at `0x007EAF7F` or the six normalized infantry action ids | Current implementation cannot match game-speed-dependent infantry action cadence. |
| `RateTimer` interpolation | No matching frame-counter interpolation model was found in the animation paths checked | Facing/turning visual interpolation uses `RateTimer` math with integer division and mid-turn retargeting; wall-clock interpolation will not match exact visual facing frames. |
| SHP vehicle body animation | `src/rules/shp_vehicle_sequence.rs` builds sequences from `WalkFrames`/`FiringFrames`, but current advancement appears tied to generic milliseconds | Binary uses `FootClass+0x538` with `WalkRate`/`IdleRate` modulo gates, then draw-time modulo by `WalkFrames`. |
| AnimType `Rate=` conversion | `src/rules/art_data.rs` — `art_rate_to_delay_ms` (line ~266) converts to milliseconds via `(900 / Rate) * 1000 / 15` (corrected 2026-05-28: was `:153`; actual function starts at line 266 — verified by Read) | This matches unnormalized wall-clock at a 15 fps assumption for some render-side overlays, but it collapses game-frame timers into wall-clock milliseconds and does not model `Normalized=yes` game-speed compensation. |
| Building/crane/damage-fire overlays | `src/app_building_anim.rs:25` advances render-side overlays by wall-clock `dt_ms`; `src/app_sim_tick.rs:184` passes capped wall-clock elapsed | This intentionally avoids render FPS over-advancement, but `gamemd` `AnimClass` advances on game frames and `Normalized=yes` changes the frame delay by game speed. |
| Special/chrono/wake/fires | `src/rules/ruleset.rs:1066` and related calls load art `Rate=` into `rate_ms`; several sim events hardcode `rate_ms: 67` | Missing the small-rate normalized table and the difference between normalized and non-normalized anims. |
| Ammo reload timing | No matching frame-counter reload formula was found in the animation paths checked | Reload cadence uses `Ammo`, `InitialAmmo`, `Reload`, `EmptyReload`, `ReloadIncrement`, and `PipWrap` with CDTimer frame math; it is adjacent to animation because UnitClass calls it after firing/facing. |
| Temporal/gap visual stages | No matching state-machine timings were found in the checked Rust animation paths | These visuals are hardcoded CDTimer phases, not `Rate=` animations. |

## 13. Parity Implications

1. `SIM_TICK_HZ = 45` is not automatically wrong, but it is only safe if every `gamemd` frame-based system uses a mapped 15-ish Hz binary frame counter. Current code mixes 45 Hz per-tick updates, synthetic `binary_frame`, and wall-clock animation timers.

2. `Rate=` should not be interpreted as milliseconds. The binary stores frame delays. A render-side millisecond approximation can match one speed setting, but it will not match `Normalized=yes` behavior across the game-speed slider.

3. `Normalized=yes` is load-bearing. Common building/refinery/fire/chrono anim rates (`Rate=200`, `300`, `400`, `450`) convert to internal delays below 5 frames, exactly where the binary uses a lookup table instead of the simple `(delay * 8) / (game_speed + 1)` formula.

4. `AnimClass` damage, trailer spawning, `Start()` effects, `Next=` transitions, and loop handling are all tied to frame-advance ticks and exact ordering. Advancing render overlays independently can make visible frames look close while gameplay side effects occur on the wrong tick.

5. The binary increments `g_CurrentFrameCounter` at the end of `Main_Tick`. Code that increments a frame counter at the start of a Rust tick will be off by one for timers that are started and checked in the same tick.

6. SHP vehicle/body animation is not `Rate=` driven. `WalkRate` and `IdleRate` gate increments of a body-frame counter, and `WalkFrames`/`FiringFrames` only decide how that counter maps to asset frames.

7. Infantry sequence timing is also not purely INI-derived. The INI supplies frame ranges and counts, but action cadence comes from a binary action-delay table; only selected action ids are normalized for game speed.

8. `RateTimer` is a separate parity requirement. Turret/body facing and several visual systems retarget from the current interpolated value and use integer frame math. Matching only final facings while using smooth wall-clock interpolation will produce different intermediate frames.

9. Ammo reload has its own frame-counter cadence. It is easy to miss because UnitClass reaches it through the same per-frame AI neighborhood as facing and animation, but it is controlled by ammo/reload INI fields, not art `Rate=`.

10. Temporal/gap effects have hardcoded frame-count phase machines. They cannot be derived from `AnimType::Rate` and should not be folded into generic sprite animation timing.

## 14. Open Questions

1. Verify the intended default game-speed value for the standard YR skirmish path after scenario setup, not just the `OptionsClass` constructor default and `[Options] GameSpeed=` read. `FUN_0069BAB0` can temporarily force `DAT_00A8EB60 = 2` when `g_GameActive != 0`, scenario byte `+0x30D8` is clear, and the dword at the scenario object base is zero; this must be tied to scenario/skirmish state before treating it as normal gameplay.

2. Finish the infantry render-side SHP index formula. This pass verified sequence parsing, action timer setup, action completion, sound triggers, and weapon fire-frame gates, but did not fully reduce every facing-mode token into final image-frame formulas.

3. Audit every Rust system that treats "tick" as a `gamemd` frame. With `SIM_TICK_HZ = 45`, those should use either `binary_frame` gates or explicit conversion; otherwise they run 3x too often.

4. Build a full action-id name map for infantry ids `0..0x29` from the sequence-name pointer table around `0x008255C8`; only selected names were needed for this timing pass.

5. Follow the ammo/reload helper into weapon-fire side effects if implementing firing cadence. This report only documents the reload timer math because it was discovered while tracing animation-adjacent vtable timing.

## Sources

- Ghidra decompile: `Main_Tick @ 0x0055D360`
- Ghidra decompile: `FUN_0055E160 @ 0x0055E160`
- Ghidra decompile: `LogicClass__PerTickUpdate @ 0x0055AFB0`
- Ghidra decompile: `Map__Logic @ 0x004D2370`
- Ghidra decompile: `RenderFrame_main @ 0x004F4480`
- Ghidra decompile: `GetRadarTimer @ 0x006C8C40`
- Ghidra decompile: `CDTimerClass__GetTimeRemaining @ 0x00426630`
- Ghidra decompile: `RateTimer__Set @ 0x004C9220`
- Ghidra decompile: `RateTimer__Current @ 0x004C93D0`
- Ghidra decompile: `OptionsClass__SetDefaults @ 0x005FA350`
- Ghidra decompile: `OptionsClass__ReadFromINI @ 0x005FA620`
- Ghidra decompile: `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0`
- Ghidra decompile/assembly: normalized-rate helper `0x005FB2E0`
- Ghidra data dump: normalized small-rate table `0x00832CEC`
- Ghidra decompile: `AnimTypeClass__Constructor @ 0x00427530`
- Ghidra decompile: `AnimTypeClass__ReadINI @ 0x00427D00`
- Ghidra decompile: `AnimClass__Constructor @ 0x00421EA0`
- Ghidra decompile: `AnimClass__AI @ 0x00423AC0`
- Ghidra decompile: `FootClass__AI @ 0x004DA530`
- Ghidra decompile: `TechnoTypeClass__Constructor @ 0x00710AF0`
- Ghidra decompile: `TechnoTypeClass__ReadINI @ 0x00712170`
- Ghidra decompile: `UnitTypeClass__Constructor @ 0x007470D0`
- Ghidra decompile: `UnitTypeClass__ReadINI @ 0x00747620`
- Ghidra decompile: `UnitClass__AI @ 0x007360C0`
- Ghidra disassembly: UnitClass vtable `+0x424` entry `0x00736CA0`
- Ghidra decompile: `UnitClass__Facing_Update @ 0x00736990`
- Ghidra decompile: `UnitClass__Fire_At_Target @ 0x00736DF0`
- Ghidra decompile: `FUN_006FB010 @ 0x006FB010`
- Ghidra decompile: `FUN_006FB080 @ 0x006FB080`
- Ghidra decompile: `UnitClass__Draw_Body_And_Turret @ 0x0073C5F0`
- Ghidra decompile: `TechnoClass__RockingUpdate @ 0x0070B570`
- Ghidra decompile: `TechnoClass__UpdateTemporalVisual @ 0x0070E5A0`
- Ghidra decompile: `TechnoClass__UpdateGapVisual @ 0x0070E920`
- Ghidra decompile: `InfantryClass__AI @ 0x0051BAB0`
- Ghidra decompile: `InfantryClass__UpdateIdleAction @ 0x0051CDB0`
- Ghidra decompile: `InfantryClass__Fire_At_Target @ 0x005206B0`
- Ghidra decompile: `InfantryClass__Do_Action @ 0x0051D6F0`
- Ghidra decompile: `InfantryClass__DoType_Sequencer @ 0x00520AE0`
- Ghidra decompile: `FUN_00523D00` infantry `Sequence=` parser
- Ghidra data dump: infantry action delay/flags table around `0x007EAF7C`
- Existing docs checked: `ANIM_CLASS_GHIDRA_REPORT.md`, `ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`, `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md`, `BUILDING_ANIM_STATE_MACHINE.md`, `REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md`
- Existing docs checked: `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`, `VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md`, `FOOTCLASS_AI_GHIDRA_REPORT.md`, `FIRE_AT_PIPELINE_GHIDRA_REPORT.md`, `UNITCLASS_GHIDRA_REPORT.md`
- Rust files checked: `src/util/fixed_math.rs`, `src/app_types.rs`, `src/app_sim_tick.rs`, `src/sim/world/mod.rs`, `src/sim/animation.rs`, `src/rules/art_data.rs`, `src/rules/infantry_sequence.rs`, `src/app_building_anim.rs`, `src/rules/ruleset.rs`
