# Timing Systems Index

Exhaustive reference docs for every timing/rate surface in `gamemd.exe`. Each
doc covers a single timing surface end-to-end: every INI key feeding it, every
hardcoded constant, every tick loop the value flows through, every multiplier
that modifies it, and every player-visible effect.

Foundation docs (1–4) must come first because every later doc cross-references
their tick definitions.

Mark progress as `TODO`, `IN-PROGRESS`, or `DONE`.

---

## Foundations

| # | Status | Doc | One-line |
|---|--------|-----|----------|
| 1 | DONE | [game-speed-master-clock.md](game-speed-master-clock.md) | `[General] GameSpeed`, 0..6 speed slider, per-tick wait derivation, the multiplier every game-tick timing flows through |
| 2 | DONE | [logic-vs-render-loop.md](logic-vs-render-loop.md) | Separation of deterministic logic ticks from interpolated render frames; which subsystems live on which loop |
| 3 | DONE | [animation-rate-delay.md](animation-rate-delay.md) | artmd `Rate=`, `Delay=`, `Start=`, `End=`, `LoopStart=`, `LoopEnd=`; whether Rate ties to game-tick or anim-tick; global anim driver |
| 4 | DONE | [multiplayer-frame-step.md](multiplayer-frame-step.md) | Lockstep batching, network buffer, replay determinism, network frame-budget event 0x20 |

## Unit / weapon timings

| # | Status | Doc | One-line |
|---|--------|-----|----------|
| 5 | DONE | [weapon-rof-burst.md](weapon-rof-burst.md) | `ROF`, `Burst`, `BurstDelay0/1`, `Reload`, `Ammo`, `[General] ReloadRate`, FireTimer, GetROF mid-burst vs end-of-burst |
| 6 | DONE | [weapon-charge-and-muzzle.md](weapon-charge-and-muzzle.md) | `IsAnimDelayedFire`/`DelayedFireDelay` (Prism/Tesla 28-tick charge), FLH parsing, `LaserDuration`, muzzle anim selection, `MultiBarrelIndex` cycling, FiringSyncFrame infantry gating |
| 7 | DONE | [movement-speed-turn-rate.md](movement-speed-turn-rate.md) | `Speed`, `TurnRate`, `ROT`, `Accelerates`, `GameSpeedBias` (RulesClass+0x1418), HouseSpeedBonus, FootClass::GetCurrentSpeed pipeline, per-locomotor variance |
| 8 | DONE | [unit-build-time.md](unit-build-time.md) | 54-step constant, `Rate = cost/54` clamped [1,255], `MultipleFactory=0.8` cumulative discount, power penalty, AI Progress headstart, full refund on cancel, FactoryClass per-tick AI |
| 9 | TODO | building-construction-anim.md | BuildupAnim frame rate, deploy transition, MakeAnim cadence |
| 10 | TODO | superweapon-recharge.md | `RechargeTime` per super, AI accel, power-off pause behavior, charge-while-disabled rules |

## Combat / damage timings

| # | Status | Doc | One-line |
|---|--------|-----|----------|
| 11 | TODO | radiation-tick-rate.md | Rad damage cadence, decay, RadSiteClass tick |
| 12 | TODO | fire-burn-duration.md | `InfDeath=4` burn animation timing |
| 13 | TODO | parasite-cycle.md | Incubation, expel, host-death timing |
| 14 | TODO | mind-control-duration.md | Control-range polling rate, `InfiniteMindControl=no` decay path |
| 15 | TODO | iron-curtain-duration.md | Duration field + on-unit vs. on-building behavior + expiry tick |
| 16 | TODO | chrono-warp-cooldown.md | Chronoshift duration, warpout/warpin animation timing, `ChronoTrigger` field, `ChronoSphereDelay` |
| 17 | TODO | emp-stun-duration.md | EMPulse warhead duration field; recovery tick |

## Animation

| # | Status | Doc | One-line |
|---|--------|-----|----------|
| 18 | TODO | infantry-sequence-timing.md | Sequence= block (Idle1/Idle2/Walk/Stand) frame rates and loop ranges |
| 19 | TODO | cameo-flash-pulse.md | Sidebar cameo ready-flash, low-power pulse, capture-progress pulse |
| 20 | TODO | damage-pip-fade.md | Health bar pip blink, damage number lifetime |

## Audio

| # | Status | Doc | One-line |
|---|--------|-----|----------|
| 21 | TODO | voice-cooldown-overlap.md | EVA queue cadence, unit voice min interval, VoiceFollowsTalk overlap suppression |
| 22 | TODO | ambient-loop-poll.md | Building idle sound poll, shore/water ambient |

## Production / economy

| # | Status | Doc | One-line |
|---|--------|-----|----------|
| 23 | TODO | ore-growth-spread.md | Tiberium growth tick rate, regrowth chance, OreGrowthRate global |
| 24 | TODO | harvester-dock-cycle.md | Bail-time, dock-load/unload, refinery process duration |
| 25 | TODO | repair-rate-cost-tick.md | Building repair tick rate, credits/tick, sell-progress tick |
| 26 | TODO | self-heal-tick.md | SelfHealing, infantry/vehicle heal cadence, HospitalSelfHealRate, RepairBaseSelfHealRate |

## UI / feedback

| # | Status | Doc | One-line |
|---|--------|-----|----------|
| 27 | TODO | cursor-scroll-poll.md | Auto-scroll rate near screen edge |
| 28 | TODO | selection-box-flash.md | Selection bracket flash interval |
| 29 | TODO | tooltip-delay.md | Hover-to-tooltip latency |
| 30 | TODO | waypoint-rally-pulse.md | Rally line pulse, waypoint marker cadence |

## Game-state timings

| # | Status | Doc | One-line |
|---|--------|-----|----------|
| 31 | TODO | veterancy-decay.md | Does veterancy decay? TS-legacy check; promotion XP threshold poll |
| 32 | TODO | cloak-uncloak-delay.md | Cloakable, CloakingSpeed, DecloakToFire re-cloak interval |
| 33 | TODO | shroud-reveal-decay.md | Confirm TS shroud decay is dead in YR; cell fade-to-explored timing |
| 34 | TODO | power-state-machine.md | Powered= poll, low-power degradation cadence (turret slowdown, radar blackout transition) |
| 35 | TODO | defeat-detection-tick.md | Last-MCV/last-building defeat poll rate |
| 36 | TODO | ai-threat-scan.md | AI retaliation poll, threat reacquire interval |
| 37 | TODO | turret-target-reacquire.md | Auto-fire target re-pick interval |

## Misc

| # | Status | Doc | One-line |
|---|--------|-----|----------|
| 38 | TODO | spy-infiltration-effect-timing.md | Duration of stolen-tech grant, per-building effect timings |
| 39 | TODO | drop-pod-arrival.md | Drop-pod warning to impact frame count |
| 40 | TODO | paradrop-spawn-cadence.md | Multi-infantry drop staggering |
| 41 | TODO | terror-drone-grace.md | Terror drone host destruction timing |
| 42 | TODO | magnetron-lift-cycle.md | Magnetron beam grab + drop frames |

---

## Cross-cutting cheat sheet (extend as iterations discover more)

### Confirmed entry points
- `Main_Game` @ `0x0048ccc0` — outer game loop; calls `Main_Tick` repeatedly
- `Main_Tick` @ `0x0055d360` — per-tick orchestration; ends with `g_CurrentFrameCounter += 1`
- `LogicClass::AI` @ `0x0055dee0` — **misnamed** — actually input/network-event dispatch (NOT the per-entity AI driver)
- `LogicClass::PerTickUpdate` @ `0x0055afb0` — late-tick housekeeping (ore growth, bombs, lasers, factories, houses) **AND** the per-entity vtable-`+0x5c` AI loop (runs unconditionally, including during pause)
- `Map::Logic` @ `0x004d2370` — per-tick map dispatch (cell flag-marking)
- `RenderFrame_main` @ `0x004f4480` — render dispatch (inside the gameplay-block gate, so suppressed during pause)
- `FUN_0055e160` @ `0x0055e160` — **the actual frame-pacing loop** — sleeps `Sleep(DAT_00887350)` in SP/replay, spinwaits with `Sleep(0)` + `Network_ServiceLoop` in MP
- `House_AI_Tick` @ `0x0055f47d` (entry) — per-house housekeeping including FPS tracking
- `EventClass::Execute` @ `0x004c807d` — network event dispatch (case 0x0d = SET_GAMESPEED, case 0x20 = NETWORK_FRAME_BUDGET)
- `State_Machine` @ `0x0048c8b0` — modal-state dispatcher for menu / save / load / score screens (runs when `g_GameState != 0`)
- `FUN_0055cfd0` @ `0x0055cfd0` — session-end handler; checks the four session-end flags
- `FUN_005fb2e0` @ `0x005fb2e0` — **GameSpeed-normalize table lookup** — converts a game-tick interval into a wall-clock-stable interval; used by anim Rate, building anims, house housekeeping, infantry actions, techno extras
- `FUN_006475f0` @ `0x006475f0` — **per-tick network turn manager** — measures RTT every 128 frames, emits event 0x20/0x21, drains event ring via `FUN_0064c380` → `EventClass::Execute`
- `FUN_0064c380` @ `0x0064c380` — event-ring drainer; calls `EventClass::Execute` on due events
- `EventClass::Execute` @ `0x004c6cb0` — single dispatcher for all network/local events; ~50 case arms by type
- `FUN_00649ca0` — outbound packet sender (returns frames-just-sent count)
- `FUN_00648710` — "Wait For Players" blocking call (returns 0=ok, 2=NoConnection, 3=Timeout, 7=Disconnected, 8/9=Other)
- `FUN_0064d9e0` — replay writer (when `_DAT_00a8d5f8 & 1`)
- `Desync_Handler` @ `0x0048dc90` — checksum-mismatch reaction (deselect all + reset selection mode)
- `Network_Keepalive` @ `0x00542520` — per-peer RTT measurement (every 8 frames in Internet MP)

### Confirmed globals
- `g_CurrentFrameCounter` — master logic frame counter; incremented at end of `Main_Tick`
- `DAT_00a8eb60` — local `OptionsClass::GameSpeed` slider value (0..6, **0=fastest**, **6=slowest**)
- `DAT_00a8eb70` — `OptionsClass::ScrollRate` slider value (0..6, default 3)
- `DAT_00a8eb7f` — `ExtraAnimations` toggle (default 0)
- `DAT_00a8b558` — multiplayer network frame budget / "FPS divisor" (set by event 0x20)
- `g_NetworkFrameBudget` — network frame budget (set by event 0x1b and 0x20)
- `DAT_00887348` — last-tick `GetRadarTimer()` snapshot for wait pacing
- `DAT_00887350` — wait interval in **`GetRadarTimer` units (= ms × 16)**; in SP/replay equals `DAT_00a8eb60`, in MP derived from `DAT_00a8b558`
- `DAT_00887328` / `DAT_00887330` — analogous wall-clock-ms snapshot/interval pair used by MP-only spinwait
- `DAT_00a8b560` / `DAT_00a8b564` — running ms/tick totals for FPS measurement
- `g_GameState` — modal state (0 = gameplay; 1/3/4/5/6/7/8/9 = various menu/dialog/score modes); when nonzero, gameplay block in `Main_Tick` is skipped — this is the in-game pause
- `_DAT_00a8d5f8` — transition/replay flag set (bit 1 = scenario-end transition, bit 0 = replay-record)
- `DAT_00a83d49` / `DAT_00a8ecd0` / `DAT_008b41c0` / `DAT_00a83d48` — four session-end flags (victory / defeat / quit / disconnect)
- `g_ScenarioClass_Instance[0x18B]` (= `+0x62c`) — intro-cinematic gate that causes `Main_Tick` to render once and skip logic

### Confirmed ReadINI scopes
- `RulesClass::ReadGeneral` @ `0x0066d530` — reads `GameSpeedBias` (RulesClass offset stored as `double`)
- `RulesClass::ReadMultiplayerDialogSettings` @ `0x00671eb0` — reads `[MultiplayerDialogSettings] GameSpeed` → `RulesClass + 0x14a0`
- `OptionsClass::ReadFromINI` @ `0x005fa646` — reads `[Options] GameSpeed` → `OptionsClass + 0x00` (`DAT_00a8eb60`)
- `OptionsClass::SetDefaults` @ `0x005fa350` — sets default `GameSpeed=3`
- `OptionsClass::ApplyFromInGameDialog` @ `0x004e1de0` — slider→speed `internal = 6 - slider`
- `SessionClass::ReadSkirmishSettings` @ `0x00697f10` — reads `GameSpeed` for skirmish save state

### TS-legacy timing traps (verify gated off in YR)
- Shroud re-decay / fog-of-war tick (gated by `SpecialFlags & 0x1000`; YR default off)
- TS credit interest / decay
- Tunnel locomotor timing (TS subterranean locomotor)
- `ImmuneToVeins` counter (TS veins)
- TS-era veterancy decay
- TS visceroid / cyborg-specific timers
- TS shadow re-grow (`ShadowGrow` defaults `no` in YR)
