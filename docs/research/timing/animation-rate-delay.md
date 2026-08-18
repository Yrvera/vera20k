# Animation Rate / Delay

## Overview

**Player-visible effect:** every animated SHP in the game (explosions, muzzle
flashes, idle animations on buildings, infantry walk cycles, weapon
projectile sprites, sidebar cameo pulses) plays back at a frame cadence
determined by its INI `Rate=` value. Some animations also wait a `Delay=`
before the first frame appears, loop a sub-range repeatedly, or randomize
their rate within a min/max band.

**Mechanism in plain terms:** each `AnimClass` instance owns a countdown
timer. Every game tick it ticks down by 1; when it hits 0 the animation
advances one frame and the timer reloads from a per-type "ticks per frame"
value. That per-type value is derived from the INI `Rate=` by the formula
`internal_ticks_per_frame = 900 / Rate`. So `Rate=900` means "1 game tick
per frame" (= as fast as the game ticks); `Rate=300` means 3 ticks per
frame; `Rate=120` means 7 ticks per frame; `Rate=1` means 900 ticks per
frame (= roughly once a minute at game-speed Medium).

There are **three knobs** that adjust animation speed beyond the per-type
`Rate=`:

1. **`Delay=`** — a one-shot countdown (in game ticks) before the first
   frame is drawn. Used by trigger-spawned animations that need a beat
   before they appear.
2. **`RandomRate=min,max`** — overrides `Rate=` with a per-instance random
   value drawn at construction time. Used to make crowd animations feel
   organic rather than synchronized.
3. **`Normalized=yes`** — re-maps the per-frame tick count through a
   GameSpeed-dependent table so the animation plays at roughly the same
   **wall-clock** rate regardless of the player's GameSpeed slider. Without
   `Normalized=`, animations play in game-ticks (and therefore speed up
   when the slider is set faster).

The animation clock is the **game-tick clock** — every animation timer
counts in `g_CurrentFrameCounter` units. There is **no separate animation
clock**. Pause behavior depends on whether the per-entity AI runs during
pause (see [logic-vs-render-loop.md](logic-vs-render-loop.md): the
per-entity vtable-`+0x5c` loop runs in `LogicClass::PerTickUpdate`, which is
unconditional — so animations *do* continue ticking during pause).

---

## INI surface

### `artmd.ini` — per-`AnimTypeClass` keys

All values quoted are example sections from the in-repo `artmd.ini`. Scope:
**per-AnimTypeClass section** (one block per animation type).

```ini
[GACNST_A]
Normalized=yes
Start=0
LoopStart=0
LoopEnd=3
LoopCount=-1
Rate=200
Layer=ground
NewTheater=yes
```

```ini
[CARYLAND]
Image=PODRING
Normalized=no
Rate=900
```

| Key | Type | Default | Read by `AnimTypeClass::ReadINI` (`0x00427d00`) into byte offset |
|---|---|---|---|
| `Rate=` | int | `1` | `0x2B0` (`[0xAC]`) — converted via `900 / INI_Rate` (or `0` if `Rate<=0`) |
| `Start=` | int | `0` | `0x2B4` (`[0xAD]`) — index of first frame |
| `End=` | int | `0` (auto-detect from SHP) | `0x2C0` (`[0xB0]`) — index of last frame |
| `LoopStart=` | int | `0` | `0x2B8` (`[0xAE]`) — re-entry frame after each loop |
| `LoopEnd=` | int | `0` | `0x2BC` (`[0xAF]`) — clamped to `End` if larger |
| `LoopCount=` | int | `0` | `0x2C4` (`[0xB1]`) — `-1` = infinite; otherwise number of loops |
| `RandomLoopDelay=min,max` | int×2 | `{0,0}` | `0x2DC..0x2E3` (`[0xB7..0xB8]`) — extra ticks waited between loops |
| `RandomRate=min,max` | int×2 | `{-1,-1}` (= not set) | `0x2E4..0x2EB` (`[0xB9..0xBA]`) — converted via `900/x`; clamped `>=0`; `min<=max` |
| `Normalized` | bool | `false` | `0x362` — when `yes`, the per-frame tick count is re-mapped through a GameSpeed table at runtime (see "Hardcoded constants") |
| `Reverse` | bool | `false` | `0x371` — play frames in reverse (`FrameStep = -1`) |
| `PingPong` | bool | `false` | `0x370` — bounce between `LoopStart` and `LoopEnd` |
| `Shadow` | bool | `false` | `0x372` — second half of SHP is shadow frames; `End` halved if `yes` |
| `Image=` | str | section name | `0x238` (file basename, used for SHP load) — does not affect timing |
| `Layer=` | enum | `3` (Ground) | `0x364` — render layer, not a timing field |

The `Damage=`, `Warhead=`, `DamageRadius=`, `Spawns=`, `BounceAnim=`,
`ExpireAnim=`, `TrailerAnim=`, `TrailerSeperation=` fields all exist on the
type but feed combat / chaining behavior rather than the animation clock; see
[fire-burn-duration.md](fire-burn-duration.md) and other combat docs.

### `Delay=` is a **constructor parameter, not an INI key on `AnimTypeClass`**

`Delay=` appears in `[InfantryTypes]`/sequence blocks (per-Sequence frame
delays) and as a constructor argument when an animation is spawned by code
(`AnimClass::Constructor(type, coords, delay, loopCount, ...)`). It is **not**
an INI key on `[AnimTypeClass]` sections. The per-instance `Delay` lives at
`AnimClass + 0x184` (`[0x61]`) — that's the runtime field — but it is not
populated from the artmd `Rate=` block. **Caveat:** infantry sequence
`Delay=` values (the per-frame waits inside `[InfantrySequence]` blocks)
flow through a different read path; see
[infantry-sequence-timing.md](infantry-sequence-timing.md).

### `rulesmd.ini` — no global animation rate

There is **no `[General] AnimRate=` or `[AudioVisual] AnimSpeed=`** key
that multiplies all animation rates globally. The only RA2-side knobs are
per-type (`Rate=`, `Normalized=`) and the global GameSpeed slider (which
affects animations through both game-tick advance and the `Normalized=`
remap table). The closest cosmetic-side knob is `ExtraAnimations` (the
launcher checkbox, persisted as `OptionsClass + 0x47` = `DAT_00a8eb7f`,
checked at `AnimClass::AI` `0x00423b76` for one specific anim type) and
`DetailLevel` (per-type). These gate *which* animations draw, not their
rate.

### Pointer-arithmetic note for `AnimTypeClass`

Per `CLAUDE.md`: `AnimTypeClass::ReadINI` uses `param_1` as `int *` — so
`param_1[N]` is a **DWORD offset** (multiply N by 4 for byte offset).
The byte offsets listed above (`0x2B0`, `0x2B4`, …) are the actual byte
positions in the class layout, already corrected. The `[0xAC]`, `[0xAD]`,
… in brackets are the `int *` indices (= byte offset / 4). Both forms come
from [ANIM_CLASS_GHIDRA_REPORT.md](../ANIM_CLASS_GHIDRA_REPORT.md).

---

## Hardcoded constants

### Rate conversion formula

`AnimTypeClass::ReadINI` @ `0x00427d00`:

```
internal_rate = 900 / INI_Rate     (if INI_Rate > 0)
internal_rate = 0                  (if INI_Rate <= 0)
```

The `900` constant is a fixed scaling factor — equivalent to saying "Rate
is measured in 1/900ths of a game-tick reciprocal". Effective tick-per-frame
table for common values seen in `artmd.ini`:

| INI `Rate=` | `internal_rate` (ticks per frame) |
|---|---|
| `900` | 1 (one frame per game tick) |
| `600` | 1 (900/600=1.5 → 1 via int division) |
| `500` | 1 |
| `450` | 2 |
| `400` | 2 |
| `375` | 2 |
| `350` | 2 |
| `320` | 2 |
| `300` | 3 |
| `250` | 3 |
| `225` | 4 |
| `220` | 4 |
| `200` | 4 |
| `180` | 5 |
| `175` | 5 |
| `170` | 5 |
| `150` | 6 |
| `120` | 7 |
| `100` | 9 |
| `60` | 15 |
| `50` | 18 |
| `20` | 45 |
| `10` | 90 |
| `0` | 0 (special: anim does not advance — used for static decorations) |

Quoted from `AnimTypeClass::ReadINI` at the assignment site for `Rate`
(byte offset `0x2B0`); verified in [ANIM_CLASS_GHIDRA_REPORT.md](../ANIM_CLASS_GHIDRA_REPORT.md)
at "Rate Conversion Formula".

### Normalized-rate GameSpeed table

`FUN_005fb2e0` @ `0x005fb2e0` — invoked when `Normalized=yes` to convert an
internal_rate value into a wall-clock-adjusted value:

```c
undefined * __thiscall FUN_005fb2e0(int *param_1, int param_2) {
    // param_1 = pointer to GameSpeed (effectively DAT_00a8eb60, range 0..6)
    // param_2 = internal rate to be normalized (in game ticks per frame)
    if (param_2 != 0) {
        if (param_2 < 5) {
            return (&PTR_s_Name_Sov07MD_00832cec)[*param_1 + param_2 * 8];
        }
        return (param_2 << 3) / (*param_1 + 1);   // (rate * 8) / (GameSpeed + 1)
    }
    return 0;
}
```

The lookup table for `param_2 < 5` lives at `0x00832cec` (verified via
`read_memory`). Reading 256 bytes from that address and indexing as
`base[GameSpeed + InRate*8]` gives:

| Input `internal_rate` (param_2) | GS=0 (Fastest) | GS=1 | GS=2 | GS=3 (Medium) | GS=4 | GS=5 | GS=6 (Slowest) |
|---|---|---|---|---|---|---|---|
| 1 | 2 | 2 | 1 | 1 | 1 | 1 | 1 |
| 2 | 3 | 3 | 3 | 2 | 2 | 2 | 1 |
| 3 | 5 | 4 | 4 | 3 | 3 | 2 | 2 |
| 4 | 7 | 6 | 5 | 4 | 4 | 4 | 3 |

For `internal_rate >= 5`, the formula `(rate * 8) / (GameSpeed + 1)` is
used:

| Input `internal_rate` | GS=0 | GS=3 | GS=6 |
|---|---|---|---|
| 5 | 40 | 10 | 5 |
| 6 | 48 | 12 | 6 |
| 7 | 56 | 14 | 7 |
| 15 | 120 | 30 | 15 |

Effect: at the **Fastest** GameSpeed, the engine *inflates* the per-frame
tick count by up to 8× — the animation appears to play at roughly the same
wall-clock pace as it does at Slowest. At **Slowest**, the table is roughly
identity (or even shrinks slightly: input 4 → 3). The intent is "the
animation looks the same to the player regardless of GameSpeed setting" — a
direct counter to GameSpeed scaling the underlying tick rate.

**Confidence (content): HIGH** — function decompiled, memory dumped, table
indices computed and cross-checked against the formula branch.
**Confidence (binding to GameSpeed slider): HIGH** — `*param_1` is the
GameSpeed value because callers pass `DAT_00a8eb60` (the slider) or a copy
held in a struct field at the same offset. **Confidence (other consumers
identified): HIGH** — `get_function_callers` returned: `AnimClass::AI`,
`AnimClass::Constructor`, `BuildingClass::GrandOpening`,
`BuildingClass::Update`, `BuildingClass::UpdateAnimation`,
`HouseClass::Update`, `InfantryClass::Do_Action`, `TechnoClass::DrawExtras`.
This table therefore normalizes **all** of: animations, building animations
(active/idle), house housekeeping cadences, infantry action animations, and
techno extras (selection brackets / pip flashes). It is essentially the
"keep this thing's wall-clock pace constant across GameSpeed" function.

### AnimClass field layout (timing-relevant)

From [ANIM_CLASS_GHIDRA_REPORT.md](../ANIM_CLASS_GHIDRA_REPORT.md):

| Byte offset | Index | Field | Purpose |
|---|---|---|---|
| `0x0AC` | `[0x2B]` | `CurrentFrame` | Current frame index |
| `0x0B0` | — | `LastFrameTime` (`g_CurrentFrameCounter` snapshot) | Set when timer expires |
| `0x0BC` | `[0x2F]` | `FrameDelay` | Current countdown to next frame (CDTimerClass) |
| `0x0C0` | `[0x30]` | `FrameDelayReload` | Reload value for the CDTimerClass = `internal_rate` |
| `0x184` | `[0x61]` | `Delay` | One-shot ticks-until-anim-starts countdown |
| `0x195` | — | `LoopCountRemaining` (byte) | Loops left; `0xFF` = infinite |

Frame advancement (from `AnimClass::AI` @ `0x00423ac0`):

```
1. If Delay > 0:
     Delay -= 1
     return  (animation hasn't started yet)
2. CDTimerClass timer (FrameDelay) ticks down
3. When timer == 0:
     CurrentFrame += FrameStep   (FrameStep = +1 normal, -1 if Reverse)
     timer reloads from FrameDelayReload (= internal_rate)
     LastFrameTime = g_CurrentFrameCounter
4. If CurrentFrame >= End:
     If LoopCountRemaining > 1 and != 0xFF:
         LoopCountRemaining -= 1
         CurrentFrame = LoopStart
         if RandomLoopDelay set:
             Delay = Random(RandomLoopDelay.Min, RandomLoopDelay.Max)
     Else if LoopCountRemaining == 1:
         If type->Next != NULL:
             type = type->Next; restart with new type
         Else:
             mark for deletion (calls Destroy via vtable+0xF8)
```

Verified directly in the decompiled `AnimClass::AI`. The
"`type` swap to `Next`" path implements animation chaining — when one
animation finishes, it transparently switches to the next type in the
chain. Used heavily for muzzle-flash → smoke-puff sequences.

### `PingPong` (frame direction reversal)

From `AnimClass::AI`:

```c
if (*(char *)(iVar9 + 0x370) != '\0') {     // PingPong = yes
    ...
    if ((*(int *)(iVar9 + 0x2c0) <= iVar8) || (iVar8 == 0)) {
        param_1[0x31] = -param_1[0x31];     // flip FrameStep direction
        return;
    }
}
```

When `PingPong=yes`, on reaching either `End` or `0`, the `FrameStep` field
(`AnimClass + 0xC4`/`[0x31]`) is negated. Animation oscillates indefinitely
unless `LoopCount` is also set.

### `RandomLoopDelay` extra wait between loops

```c
if ((*(int *)(iVar8 + 0x2dc) == 0) && (*(int *)(iVar8 + 0x2e0) == 0)) {
    return;
}
iVar8 = Random__RandomRanged(*(int *)(iVar8 + 0x2dc), *(undefined4 *)(iVar8 + 0x2e0));
param_1[0x61] = iVar8;   // Delay field — re-engages the start-up wait
return;
```

After a loop completes, if `RandomLoopDelay` is set (both fields > 0), the
engine populates `Delay` with a random value in `[min..max]` ticks. The
animation pauses for that many ticks before starting its next loop. Used
for randomized idle building animations (e.g. radar pulse, smoke puff
cadence) so multiple buildings of the same type don't all blink in
lockstep.

### `RandomRate` (per-instance random rate override)

```c
iVar9 = *(int *)(iVar8 + 0x2b0);   // type's internal_rate
if ((*(int *)(iVar8 + 0x2e4) != 0) || (*(int *)(iVar8 + 0x2e8) != 0)) {
    iVar9 = Random__RandomRanged(*(int *)(iVar8 + 0x2e4),
                                 *(undefined4 *)(iVar8 + 0x2e8));
}
```

At loop-restart, if `RandomRate` is set (default `{-1,-1}` means "not
set"; conversion in `ReadINI` clamps min/max ≥ 0), the engine picks a new
`internal_rate` in `[min..max]` for this single loop. The values stored
are already post-`900/x` converted at INI read time.

### Per-tick driver: `AnimClass::AI` is called via the vtable-`+0x5c`/`+0x60` loop

From the existing report: `AnimClass::AI` is **vtable[24]** (offset `0x60`).
The per-entity loop in `LogicClass::PerTickUpdate` calls `vtable+0x5c` —
which on `AnimClass` is `AbstractClass::Update_Override` (or similar), and
that in turn calls `AI()` at slot `0x60`. **Confidence (binding): MEDIUM**
— the report says `+0x60`, the loop in `PerTickUpdate` uses `+0x5c`. The
intervening vtable slot is likely the `AnimClass`-specific update wrapper.
This needs cross-verification by reading an `AnimClass` vtable at a known
address; deferred.

What is **verified**: animations advance on every game tick (i.e., every
`Main_Tick` iteration where `LogicClass::PerTickUpdate` runs — which is
**always**, even during pause).

---

## Tick / frame topology

| Stage | Clock | Where |
|---|---|---|
| `AnimClass::AI` invocation | game-tick | `LogicClass::PerTickUpdate` per-entity loop, vtable-`+0x5c` |
| `Delay` countdown | game-tick (decrements by 1 per `AnimClass::AI` call) | `AnimClass::AI` early-out branch |
| `FrameDelay` (CDTimerClass) | game-tick | `AnimClass::AI` post-Delay branch |
| `LastFrameTime` snapshot | game-tick | `AnimClass::AI`: `param_1[0x2d] = g_CurrentFrameCounter` |
| `Damage` accumulator | game-tick (per frame) | `AnimClass::AI`: `param_1[0x62] += type->Damage * (mind-control bonus)` |
| `TrailerAnim` spawn | game-tick: `g_CurrentFrameCounter % TrailerSeperation == 0` | `AnimClass::AI`: trailer spawn branch |
| `RandomLoopDelay` re-wait | game-tick (uses `Delay` field) | `AnimClass::AI`: loop-restart branch |
| `Normalized` remap | game-tick (input), tick-budget-adjusted (output) | `FUN_005fb2e0` invoked on Rate access |

### `Damage` accumulator and the mind-control bonus multiplier

Within the per-frame damage path:

```c
if (((int *)param_1[0x33] == (int *)0x0) || ...) {
    dVar1 = *(double *)(param_1[0x32] + 0x2a8);              // Damage
} else {
    dVar1 = *(double *)(param_1[0x32] + 0x2a8) * _DAT_007e3568;  // Damage * bonus
}
*(double *)(param_1 + 0x62) = dVar1 + *(double *)(param_1 + 0x62);
```

`_DAT_007e3568` is a `double` constant scaling factor applied when the
animation's `param_1[0x33]` (something like "attached object" or "owner
techno of type 0x24" based on the `(**(*0x2c))() != 0x24` check). The
constant's value is not read in this iteration; flag for follow-up if a
combat/damage-anim doc needs it.

### TrailerAnim cadence

```c
if ((iVar8 == 1) || (g_CurrentFrameCounter % iVar8 == 0)) {
    ...spawn trailer anim copy...
}
```

`iVar8 = TrailerSeperation` (note Westwood's original typo "Seperation"
preserved at byte offset `0x30C`). If `TrailerSeperation=1`, spawn every
tick. Otherwise spawn when `g_CurrentFrameCounter % TrailerSeperation == 0`.
This binds the trailer cadence to the **master frame counter**, not to the
parent animation's own frame index — so if multiple trailer-spawning anims
of the same type exist, they all spawn trailers on the same global tick,
not staggered per-instance.

---

## Multipliers and modifiers

### `Normalized=yes` (per-AnimTypeClass)

Re-maps `internal_rate` through `FUN_005fb2e0`. See "Hardcoded constants"
above for the table. Net effect: the higher the GameSpeed (Fastest = 0),
the more ticks per frame the animation actually takes, so wall-clock pace
stays roughly constant.

### `Reverse=yes` (per-AnimTypeClass)

Sets `FrameStep = -1` so frame index decrements each tick instead of
incrementing. End-condition is `CurrentFrame <= 0` instead of `>= End`. No
effect on rate.

### `PingPong=yes` (per-AnimTypeClass)

Flips `FrameStep` sign at each end. Combined with `LoopCount=-1` produces
an indefinite oscillation. No effect on rate.

### `Shadow=yes` (per-AnimTypeClass)

The SHP has 2× the frames — the second half are shadow frames. `End` is
halved at INI-read time so the timing math sees only the real frame count.
`LoopEnd` clamps to the halved `End`.

### `RandomRate=min,max` (per-AnimTypeClass)

Per-loop random rate override (converted via `900/x` at INI read). Selected
fresh at each loop restart.

### `RandomLoopDelay=min,max` (per-AnimTypeClass)

Per-loop random extra `Delay` between iterations.

### Constructor parameter `delay` (call-site)

When code spawns an `AnimClass`, it passes a `delay` parameter that
populates the per-instance `Delay` field. Use cases include weapon
projectile impacts that want a half-second hang before the splash anim
plays. Verified in [ANIM_CLASS_GHIDRA_REPORT.md](../ANIM_CLASS_GHIDRA_REPORT.md):
the constructor signature shows `int delay` as parameter 4.

### Constructor parameter `loopCount` (call-site)

Multiplies the type's `LoopCount` to give `LoopCountRemaining`. Use case:
spawn an idle anim that loops `type->LoopCount * 3` times before chaining
to the next type.

### `ExtraAnimations` (launcher checkbox / Options dialog)

`DAT_00a8eb7f` — toggled via `OptionsClass::ApplyFromInGameDialog` /
`OptionsClass::ApplyFromLauncherDialog`. Default `0`. Read in
`AnimClass::AI` @ `0x00423b76` for the special-case branch:

```c
if (param_1[0x32] == *(int *)(g_RulesClass_Instance + 0xb8)) {
    if (DAT_00a8eb7f == '\0') {
        *(undefined1 *)((int)param_1 + 0x19d) = 1;   // invisible
    } else {
        *(undefined1 *)((int)param_1 + 0x19d) = 0;   // visible
    }
}
```

Gates **visibility** (the `0x19d` "invisible" flag), not rate. The
`RulesClass + 0xb8` field is the AnimType pointer affected — it's a single
specific cosmetic anim (possibly the rules-listed `ExtraExplosionFlare` or
similar; check from a combat-anim doc). The cosmetic checkbox does **not**
slow down all animations.

### `DetailLevel=` (per-AnimTypeClass) and `TranslucencyDetailLevel=`

Per the existing report fields at `0x2D4` / `0x2D8`. These compare against
a global "current detail level" to decide whether the anim draws at all /
whether it draws translucent. Visibility gates, not rate.

### `g_GameMode` interaction

Animations advance the same in SP, replay, LAN, and Internet MP. None of
the timing fields are MP-specific.

---

## Edge cases

### Pause behavior — animations continue ticking

`LogicClass::PerTickUpdate` runs unconditionally (see
[logic-vs-render-loop.md](logic-vs-render-loop.md)). The per-entity
vtable-`+0x5c` loop iterates every entity in the LogicClass entity array,
which includes `AnimClass` instances. Therefore `AnimClass::AI` is called
every tick — including during the in-game menu pause. **Player-visible
effect:** open the in-game menu and a long building idle animation will
continue advancing underneath the menu. Verified by tick-topology
inspection; not yet observed in live binary.

### Save / load mid-animation

`AnimClass::Load` @ `0x00425280` restores `CurrentFrame`, `FrameDelay`,
`Delay`, `LoopCountRemaining`, `LastFrameTime`, and the type pointer.
Animation resumes from exactly where it was. `g_CurrentFrameCounter`
itself is part of the save state (per
[game-speed-master-clock.md](game-speed-master-clock.md)), so all
"start-frame"-based math (`g_CurrentFrameCounter - LastFrameTime`) remains
coherent.

### Replay determinism

`Random__RandomRanged` is called for `RandomRate` and `RandomLoopDelay`.
The RNG is deterministic (seeded from scenario start and advanced
identically per tick on every peer), so animations evolve identically in
replay and across MP peers. Therefore even cosmetic flicker patterns are
lockstep.

### `Rate=0` static animations

When INI `Rate <= 0`, `internal_rate = 0` is stored. Reading the
`AnimClass::AI` timer-tick block: if `param_1[0x30] == 0` (no reload
value), the early-out path returns immediately, leaving `CurrentFrame`
unchanged. Used for static decorations (e.g., building debris that just
sits on screen).

### `End = 0` auto-detect from SHP header

If the INI does not set `End=`, `AnimTypeClass::Constructor` defers; at
`AnimClass::AI` the first time the type is touched (`if (((int
*)param_1[0x32])[0xb0] == -1)`) it pulls the frame count out of the
SHPData header via vtable `+0x9c` and writes it into `End`. If `Shadow=yes`,
the count is halved. **Side effect:** `End` is mutated on the *shared
type*, not the instance. So the first AnimClass to use a type forces the
auto-detect for everyone.

### Iron Curtain / EMP / Mind Control / Chrono / Stasis freezes

These freeze the **affected unit's AI** by way of per-unit state flags
checked inside `TechnoClass::AI` / `FootClass::AI`. They do **not** freeze
animations on those units — explosion / cloak / iron-curtain visual
effects continue to play. Specifically, if Iron Curtain is applied, the
unit can't move, but its turret-fire muzzle animation (if it's already
firing) and its idle-animation (e.g. tracks) continue. Each freeze type's
own doc owns the precise gating logic:
- [iron-curtain-duration.md](iron-curtain-duration.md)
- [emp-stun-duration.md](emp-stun-duration.md)
- [mind-control-duration.md](mind-control-duration.md)
- [chrono-warp-cooldown.md](chrono-warp-cooldown.md)

### Mid-tick session end

If a session-end flag is set inside `LogicClass::PerTickUpdate` (e.g.,
`HouseClass::Update` declares the player defeated), the current tick still
finishes its full entity iteration. Any animations spawned in that final
tick will appear on the **next** render but their `AI()` call won't fire
again because `g_CurrentFrameCounter` won't increment. Effect: the final
frame of the game can show partially-advanced animations.

### `Damage > 0.0` animations (e.g. fire, gas cloud)

The per-frame damage accumulator (`type->Damage` per game tick, applied
every time the accumulator crosses 1.0) ticks on the master clock. **Not
GameSpeed-normalized**: at Fastest, damage-anims deal damage faster in
wall-clock terms; at Slowest, slower. This is opposite to the
`Normalized=yes` cosmetic behavior. Cross-ref
[fire-burn-duration.md](fire-burn-duration.md).

---

## TS-legacy filter

| Field / branch | TS-legacy? | Notes |
|---|---|---|
| Core `Rate=` / `Start=` / `End=` / `LoopStart=` / `LoopEnd=` / `LoopCount=` | **Live in YR** | Every animation uses these. |
| `Normalized=` + GameSpeed table | **Live in YR** | Active for many `[GACNST_A]`-style YR anims. |
| `RandomRate=` / `RandomLoopDelay=` | **Live in YR** | Used by idle building anims. |
| `PingPong=` | **Live in YR** | Used by some warp/charge effects. |
| `Reverse=` | **Live in YR** | Used by retract/deactivate anims. |
| `Shadow=` | **Live in YR** | Building idle anim shadows. |
| `Damage=` / `Warhead=` / `DamageRadius=` | **Live in YR** | Burn anims, radiation. |
| `TrailerAnim=` / `TrailerSeperation=` | **Live in YR** | Smoke trails on damaged units. |
| `Spawns=` / `SpawnCount=` / `BounceAnim=` / `ExpireAnim=` | **Live in YR** | Meteor / explosion behavior. |
| `IsTiberium` / `IsVeins` / `IsMeteor` / `IsFlamingGuy` | **Mostly TS** | `IsVeins` is dead (no veins in YR). `IsTiberium` is live (ore overlay anim hook). `IsMeteor` is live (meteor SW). `IsFlamingGuy` is live (`InfDeath=4` burn). |
| `TiberiumChainReaction` | **Live in YR but rare** | Only fires from a few warhead types. |
| `IsAnimatedTiberium` | **Live in YR** | Animated ore overlay. |
| `PsiWarning` | **Live in YR** | Psychic Dominator warning circle. |
| `HideIfNoOre` | **Live in YR** | Used by certain harvester/ore anims. |
| `ShouldUseCellDrawer` | **Live in YR** | Default `yes`. |
| `Bouncer=` + bounce physics | **Live in YR** | Meteor anims, etc. |
| `Tiled=` | **Live in YR** | Tiled wall/floor decals. |
| `Flat=` / `YDrawOffset` / `ZAdjust` | **Live in YR** | Warp ring drawing. |
| `Scorch=` / `Crater=` / `ForceBigCraters=` / `Sticky=` | **Live in YR** | Permanent smudge spawn. |
| `Flamer=` | **Live in YR** | Flame-thrower variant tagging. |
| `DoNotSimplify=` | **Not a real key in YR** | Per the existing report — string not present in binary, ignored. |
| `EndSound=` | **Not a real key in YR** | Same — string not present. |
| `Report=` | **Real key, aliases `StartSound=`** | Same struct offset. |
| `Translucent=` / `Translucency=25/50/75` | **Live in YR** | Warp / chrono ring. |

---

## Cross-references

- [game-speed-master-clock.md](game-speed-master-clock.md) — defines the
  master `g_CurrentFrameCounter` that the animation timer counts in;
  defines the GameSpeed slider that `Normalized=yes` compensates for
- [logic-vs-render-loop.md](logic-vs-render-loop.md) — confirms the
  per-entity `AI()` loop runs inside `LogicClass::PerTickUpdate` (which is
  unconditional, including during pause)
- [infantry-sequence-timing.md](infantry-sequence-timing.md) — per-Sequence
  block `Delay=` values for infantry walk/idle/death cycles
- [building-construction-anim.md](building-construction-anim.md) —
  `BuildupAnim` cadence and the buildup-anim-finish handoff
- [cameo-flash-pulse.md](cameo-flash-pulse.md) — sidebar cameo ready-flash
  cadence (likely a separate timer, but may reuse the `Normalized` table)
- [fire-burn-duration.md](fire-burn-duration.md) — the `Damage=` field's
  per-frame damage accumulator
- [chrono-warp-cooldown.md](chrono-warp-cooldown.md) — warp-in/warp-out
  animation timing (uses Rate=120/150/300 examples)
- [magnetron-lift-cycle.md](magnetron-lift-cycle.md) — Magnetron beam
  grab/drop animation frames

---

## Coverage audit

| Item | Disposition |
|---|---|
| `[AnimTypeClass] Rate=` | Owned here |
| `[AnimTypeClass] Start=` / `End=` / `LoopStart=` / `LoopEnd=` / `LoopCount=` | Owned here |
| `[AnimTypeClass] RandomLoopDelay=` / `RandomRate=` | Owned here |
| `[AnimTypeClass] Reverse=` / `PingPong=` / `Shadow=` / `Normalized=` | Owned here |
| `[AnimTypeClass] Damage=` / `Warhead=` / `DamageRadius=` | Owned by [fire-burn-duration.md](fire-burn-duration.md) and warhead-specific docs; cross-referenced |
| `[AnimTypeClass] TrailerAnim=` / `TrailerSeperation=` | Owned here (cadence is timing) |
| `[AnimTypeClass] Spawns=` / `SpawnCount=` / `BounceAnim=` / `ExpireAnim=` / `Next=` | Animation chaining — chain semantics owned here; combat semantics in respective combat docs |
| `[AnimTypeClass] StartSound=` / `Report=` / `StopSound=` | Owned by [voice-cooldown-overlap.md](voice-cooldown-overlap.md) or an animation-sound doc; cross-referenced |
| `[AnimTypeClass] DetailLevel=` / `TranslucencyDetailLevel=` | Visibility gates, not rate — owned by a render-detail-throttle doc if one becomes needed |
| `Delay` constructor parameter | Owned here |
| `loopCount` constructor parameter | Owned here |
| Per-instance `LoopCountRemaining` | Owned here |
| `FUN_005fb2e0` normalize table | Owned here |
| `_DAT_007e3568` mind-control damage multiplier | Flagged for follow-up in [fire-burn-duration.md](fire-burn-duration.md) or [mind-control-duration.md](mind-control-duration.md) |
| `ExtraAnimations` checkbox (`DAT_00a8eb7f`) | Owned here (visibility toggle for one specific anim type) |
| `vtable + 0x60` (`AnimClass::AI` slot) vs. `vtable + 0x5c` (loop slot) | Identified here as needing cross-verification; deferred |

---

## Ghidra queries log (this iteration)

| Query | Result |
|---|---|
| Read `ANIM_CLASS_GHIDRA_REPORT.md` lines 155–500 | Confirmed full `AnimTypeClass` field-offset table, `Rate=900/INI_Rate` formula, `AnimClass::AI` flow, constructor signature |
| `search_functions "AnimClass"` | Confirmed `AnimClass::AI @ 0x00423ac0`, `AnimClass::Constructor @ 0x00421ea0`, `AnimClass::Middle @ 0x00424ce0`, plus virtual stubs |
| `decompile_function 0x00423ac0` (AnimClass::AI) | Confirmed `Delay`-countdown → CDTimerClass-tick → `CurrentFrame += FrameStep` → end-condition / PingPong / Reverse / Loop / Next handling; verified `param_1[0x2d] = g_CurrentFrameCounter` (LastFrameTime); verified `RandomRate` at `iVar8 + 0x2e4/0x2e8`; verified `RandomLoopDelay` at `iVar8 + 0x2dc/0x2e0`; verified `Normalized` branch calls `FUN_005fb2e0` |
| `get_xrefs_to 0x00a8eb7f` | Confirmed `ExtraAnimations` is read in `AnimClass::AI` for one specific anim type |
| `search_strings "Normalized"` | One hit at `0x00818610`, xref'd from `VoxelAnimTypeClass`, `AnimTypeClass`, `ParticleTypeClass`, `TechnoTypeClass` — confirms `Normalized` exists on multiple type classes; same semantics as anim |
| `decompile_function 0x005fb2e0` | Confirmed `(param_2 < 5) ? table[GameSpeed + Rate*8] : (Rate*8) / (GameSpeed+1)` |
| `get_function_callers 0x005fb2e0` | 8 consumers — confirms this table normalizes anim + building + house + infantry-action + techno-extras timings, not just animations |
| `read_memory 0x00832cec len=256` | Confirmed table contents → reconstructed `[Rate, GameSpeed]` lookup as shown above |
| `search_functions "AnimTypeClass"` | Confirmed `AnimTypeClass::ReadINI @ 0x00427d00`, `Constructor @ 0x00427530` |
| `grep ^Rate= ini/artmd.ini \| sort -u` | Confirmed the practical INI Rate range (`0..900`) and that `900` is the maximum used (= 1 tick/frame) |
| Inspected `[GACNST_A]` and `[CARYLAND]` for representative `Normalized=yes` / `=no` examples | Confirmed both forms are in production INI |
