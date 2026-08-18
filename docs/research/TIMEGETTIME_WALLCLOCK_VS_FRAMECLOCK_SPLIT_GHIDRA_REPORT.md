# timeGetTime() Wall-Clock vs Frame-Clock Split — Ghidra Research Report

**Date:** 2026-05-28
**Slot:** 5 of /re-swarm 2026-05-28 (timer classes, anim staging, frame-cadence inventory)
**Addresses verified in this session:**
- `GetRadarTimer` @ `0x006C8C40`
- `Main_Tick` @ `0x0055D360`
- `FUN_0055E160` @ `0x0055E160`
- `FUN_004A3C30` @ `0x004A3C30`
- `FUN_0048D1E0` @ `0x0048D1E0`
- `Main_Game` @ `0x0052E6C3`-surrounding
- `FUN_0055CFD0` @ `0x0055CFD0`
- `FUN_00542620` @ `0x00542620`
- `FUN_005BDC80` @ `0x005BDC80`
- `FUN_005D3BA0` @ `0x005D3BA0`
- `FUN_005D4430` @ `0x005D4430`
- `FUN_005F0070` @ `0x005F0070`
- `FUN_006523A0` @ `0x006523A0`
- `FUN_006639D0` @ `0x006639D0`
- `FUN_00663DB0` @ `0x00663DB0`
- `FUN_00664530` @ `0x00664530`
- `FUN_00683610` @ `0x00683610`
- `ScenarioClass__Constructor` @ `0x006832DA`
- `ScenarioClass__Start_Scenario` @ `0x00683E3A`
- `FUN_00685670` @ `0x00685670`
- `FUN_00692B60` @ `0x00692B60`
- `FUN_00648350` @ `0x00648350`
- `FUN_0055CF10` @ `0x0055CF10`
- `MapSelect__Constructor` @ `0x005ADDE0`
- `FUN_005AE220` @ `0x005AE220`

**Confidence:** High for all PRESENTATION-PACED and LOGIC-PACED classifications in the table; High for the FPS counter identification; High for the wall-clock chat-message-timeout and scroll-acceleration subsystems; Medium for the "AMBIGUOUS: scroll acceleration state" entry — the timer controls when the per-tick acceleration step fires, which is gameplay-adjacent.

**Active in YR:** Yes for the skirmish-live subsystems. No for the multiplayer network throttle/router/WOL subsystems in a local skirmish (flagged below).

---

## 0. Investigation Frame

**Target question:** Which subsystems reachable in a live YR skirmish frame read `timeGetTime()` (directly or via `GetRadarTimer`) and are they PRESENTATION-PACED (wall-clock) or LOGIC-PACED (`g_CurrentFrameCounter`)?

**Non-goals:** Re-deriving the main throttle helper `FUN_0055E160` (already documented), re-deriving `CDTimerClass`/`RateTimer`/`AnimClass` (already LOGIC-PACED in prior docs).

**Evidence needed to mark COMPLETE:** All unique callers of `GetRadarTimer @ 0x006C8C40` enumerated and classified; ambiguous subsystems identified with at least one Rust-facing handoff note.

**Stop conditions:** All `get_xrefs_to 0x006C8C40` callers decompiled and classified. No active-in-YR-skirmish caller left unclassified.

---

## 1. Full Call-Site Inventory

`GetRadarTimer @ 0x006C8C40` wraps `timeGetTime() >> 4` (verified via `decompile_function 0x006C8C40`). All `timeGetTime` calls in the binary go through `GetRadarTimer` except:

1. **Direct `timeGetTime()` in `Main_Tick`:** Two direct calls — one stores `_DAT_00a8b55c = timeGetTime()` at tick entry, one computes `iVar4 = DVar10 - _DAT_00a8b55c` near the end and accumulates `DAT_00a8b560 += iVar4; DAT_00a8b564 += 1`. These two form an FPS/tick-duration performance counter (verified via `decompile_function 0x0055D360`).

2. **Direct `timeGetTime()` in `Main_Tick` (network path):** Multiple direct calls in the non-mode-5 `DAT_00a8b558` network throttle branch of `Main_Tick`, storing into `local_1b4 → DAT_00887328` and `local_1ac → DAT_00887330`. This path is **not active in local YR skirmish** (mode-5 skips it).

All other `timeGetTime` usage in the binary runs via `GetRadarTimer`.

---

## 2. Split Map

| Subsystem | Function(s) | What wall-clock value it reads | What it drives | Classification | Active in YR skirmish |
|---|---|---|---|---|---|
| **Main tick throttle** | `Main_Tick @ 0x0055D360`, `FUN_0055E160 @ 0x0055E160` | `GetRadarTimer()` buckets (`timeGetTime() >> 4`, 16 ms each) into `DAT_00887348/DAT_00887350` | Paces how fast main-tick frames are allowed to run; budget = speed byte; work time subtracted before sleep | PRESENTATION-PACED (throttle/sleep) | Yes |
| **Per-tick FPS/duration counter** | `Main_Tick @ 0x0055D360` | Direct `timeGetTime()` delta from tick entry to near-end | `DAT_00a8b560` (cumulative tick ms), `DAT_00a8b564` (count); diagnostic perf counter | PRESENTATION-PACED (FPS counter) | Yes |
| **Tactical auto-scroll acceleration** | `FUN_00692B60 @ 0x00692B60` | `GetRadarTimer()` → `DAT_00B05638/DAT_00B05640` | Ramps scroll speed one step per 16 ms bucket; acceleration timer for edge-scroll velocity curve | **AMBIGUOUS** (wall-clock timer drives per-tick gameplay scroll increment) | Yes |
| **Screen-fade / loading transition** | `FUN_004A3C30 @ 0x004A3C30` | `GetRadarTimer()` with budget `param_2` (bucket count) | Cross-fade alpha `((elapsed × 256) / total_budget)`, loops until budget exhausted | PRESENTATION-PACED (blend animation) | Conditional (loading/exit only, not inside a running skirmish tick) |
| **Network lobby pre-game timing** | `FUN_0048D1E0 @ 0x0048D1E0` | `GetRadarTimer()` → `DAT_0089E920/DAT_0089E928` | Network frame ready wait, 0x78-bucket (1920 ms) resync window | PRESENTATION-PACED (network ready/wait) | No (mode-4 net lobby, not mode-5 skirmish) |
| **Network FPS/frames-per-second display** | `FUN_00542620 @ 0x00542620` | `GetRadarTimer()` against `DAT_00AC6648` every 128 frames | Formats `FSPS=N` diagnostic display; rate = `DAT_00B0487c / elapsed_buckets` | PRESENTATION-PACED (perf display) | No (g_GameMode==4 only, internet/LAN) |
| **Chat message timeout** | `FUN_005D3BA0 @ 0x005D3BA0`, `FUN_005D4430 @ 0x005D4430` | `GetRadarTimer()` → `piVar6[9]` (message expiry bucket) | Chat label display duration; labels expire when `GetRadarTimer() + DAT_00887340 > expiry_bucket` | **AMBIGUOUS** (wall-clock bucket drives when in-game text disappears; player-visible) | Conditional (mode-3/4 LAN/net; in mode-5 skirmish only if chat overlay fires) |
| **Load progress timer** | `FUN_005F0070 @ 0x005F0070` | `GetRadarTimer()`, 29-bucket (464 ms) window | Loading-screen delay at post-connection; waits for short bucket window while calling render | PRESENTATION-PACED (loading delay) | No (pre-game setup only) |
| **Network disconnect flush** | `FUN_006523A0 @ 0x006523A0`, `FUN_0055CF10 @ 0x0055CF10` | `GetRadarTimer()`, 300-bucket (4800 ms) timeout | Waits for outgoing network queue to drain before tearing down connection | PRESENTATION-PACED (network shutdown wait) | No (disconnect/exit only) |
| **WOL router master-player-list send** | `FUN_006639D0 @ 0x006639D0`, `FUN_00664530 @ 0x00664530` | `GetRadarTimer()`, 900-bucket (14.4 s) timeout | Waits for TALK_HELLO ack from router process after `CreateProcess(mphmd.exe)` | PRESENTATION-PACED (network setup) | No (WOL/internet mode only) |
| **Router process startup wait** | `FUN_00663DB0 @ 0x00663DB0` | `GetRadarTimer()`, 0x708-bucket (28.9 s) timeout | Waits for router process to start and reply TALK_HELLO | PRESENTATION-PACED (process launch wait) | No (WOL/internet mode only) |
| **Scenario loading timers init** | `ScenarioClass__Constructor @ 0x006832DA`, `FUN_00683610 @ 0x00683610` | `GetRadarTimer()` snapshots into `ScenarioClass+0x614` and `+0x620` | Elapsed-bucket accumulators (`+0x61c`, `+0x628`) for scenario load profiling; reset on each scenario reset | PRESENTATION-PACED (load-time profiling, not gameplay logic) | Conditional (called at game-exit / scenario end; not mid-skirmish) |
| **ScenarioClass::Start_Scenario** | `ScenarioClass__Start_Scenario @ 0x00683E3A` | `GetRadarTimer()` snapshot into `ScenarioClass+0x614` | Re-arms the load-time elapsed accumulator if it was cleared; feeds the same profiling counters | PRESENTATION-PACED (load profiling) | Conditional (called once at scenario start) |
| **End-game network wait / framesync** | `FUN_00648350 @ 0x00648350` | `GetRadarTimer()`, 0xf-bucket (240 ms) resend, 600-bucket (9600 ms) outer timeout | Waits for all players to acknowledge final frame before exiting game loop | PRESENTATION-PACED (end-game handshake) | No (mode-3/4 only) |
| **Movie/vox audio drain** | `FUN_00685670 @ 0x00685670` | `GetRadarTimer()` baseline from `DAT_00887338/DAT_00887340`; 300-bucket (4800 ms) drain | Waits up to 4.8 s for VOX audio to finish playing after game-over before teardown | PRESENTATION-PACED (audio drain) | Conditional (game-exit only) |
| **Map-select screen timer** | `FUN_005AE220 @ 0x005AE220`, `MapSelect__Constructor @ 0x005ADDE0` | `GetRadarTimer()` snapshot into `param_1+0xcc` | Inter-option navigation timer on campaign map select UI | PRESENTATION-PACED (UI animation) | No (campaign/skirmish map select only, not inside running skirmish) |
| **Bink / CD-loading progress** | `CDFileClass__Constructor @ 0x004B7554` et al., `Pipe__Constructor @ 0x005E7EC4` et al., `Read_Theater_TileSets_INI @ 0x005451CE` et al. | `GetRadarTimer()` snapshots against `DAT_00887338/DAT_00887340` baseline | Loading progress / chunk timing during theater/asset load | PRESENTATION-PACED (asset load profiling) | No (load phase only) |
| **All CDTimerClass / RateTimer / AnimClass consumers** | Multiple (see GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md) | `g_CurrentFrameCounter` exclusively | Weapon cooldown, facing interpolation, anim frame advancement, scenario timers, ore growth, etc. | LOGIC-PACED (frame-counter) | Yes |

---

## 3. The DAT_00887338 / DAT_00887340 Global Pair

Multiple callers (chat timeout, scroll, audio drain, loading) read these two globals together in a `if (DAT_00887338 != -1) { elapsed = GetRadarTimer() - DAT_00887338; accumulated = DAT_00887340 + elapsed; }` idiom. They are a **shared wall-clock baseline** (start bucket + accumulated bucket count) used across subsystems for elapsed-time bookkeeping. They are distinct from `DAT_00887348 / DAT_00887350` (main-tick local throttle). `DAT_00887338 == -1` disables elapsed subtraction and causes the raw accumulated value to be used unchanged — the same sentinel idiom as `CDTimerClass.start_frame == -1`.

Evidence: `decompile_function 0x005D3BA0`, `0x005D4430`, `0x00685670`, `0x00692B60`, and the CDFileClass/Pipe callers.

---

## 4. AMBIGUOUS Subsystems — Parity Hazards

### 4.1 Tactical Auto-Scroll Acceleration (`FUN_00692B60 @ 0x00692B60`)

**What it does:** Uses `GetRadarTimer()` against `DAT_00B05638/DAT_00B05640` to fire one scroll acceleration step per ~16 ms bucket. Each step increments `param_1[0x1552]` (scroll level) up toward `8 - (DAT_00a8eb70 + 1)`. The deceleration path also fires one step per bucket.

**Why ambiguous:** The scroll acceleration step count is controlled by wall-clock bucket count, but the resulting scroll level directly drives how many tactical cells the viewport moves per tick — a gameplay-adjacent quantity. If Rust paces this on sim ticks instead of real-time, the scroll rate will change with game speed. If Rust paces it on wall-clock ms but at wrong resolution, acceleration/deceleration feel will differ.

**Verdict:** PRESENTATION-PACED (the velocity ramp is a presentation/feel effect, not a deterministic gameplay state). The scroll position itself is per-tick, but the ramp rate is wall-clock.

Evidence: `decompile_function 0x00692B60`, `get_xrefs_to 0x006C8C40` confirms FUN_00692B60 calls GetRadarTimer multiple times.

### 4.2 Chat Message Timeout (`FUN_005D3BA0`, `FUN_005D4430`)

**What it does:** `FUN_005D3BA0` stores `expiry = GetRadarTimer() + DAT_00887340 + param_7` in `piVar6[9]`. `FUN_005D4430` expires labels when `GetRadarTimer() + DAT_00887340 > expiry`. The label lifetime is thus measured in 16 ms buckets.

**Why ambiguous:** In a multiplayer game, chat labels appear at wall-clock rate regardless of game speed. In local skirmish (mode-5), chat is typically not shown but the same bucket-expiry path would apply if triggered. The expiry is wall-clock, not frame-based, which is correct presentation behavior. But `DAT_00887340` accumulates across scenarios and its reset discipline is non-obvious.

**Verdict:** PRESENTATION-PACED. The player observes the label disappearing at a real-time interval, not at a game-speed-dependent interval.

Evidence: `decompile_function 0x005D3BA0`, `0x005D4430`.

---

## 5. Call-Site Classification by Liveness in Local YR Skirmish

| Group | Functions | Active in local skirmish (mode-5)? |
|---|---|---|
| Main throttle + FPS counter | `Main_Tick` | Yes |
| Tactical scroll acceleration | `FUN_00692B60` | Yes |
| Chat timeout | `FUN_005D3BA0`, `FUN_005D4430` | Conditional (chat overlay if triggered) |
| Screen fade | `FUN_004A3C30` | Loading/exit only |
| Scenario load-time profiling | `ScenarioClass__Constructor`, `FUN_00683610`, `ScenarioClass__Start_Scenario` | Once at scenario start/end |
| Movie/audio drain | `FUN_00685670`, `FUN_0055CF10` | Game-exit only |
| Network throttle, lobby, router | `FUN_0048D1E0`, `FUN_006639D0`, `FUN_00663DB0`, `FUN_00664530`, `FUN_006523A0`, `FUN_00542620` | No (mode-3/4 only) |
| End-game framesync | `FUN_00648350` | No (mode-3/4 only) |
| Map-select UI | `FUN_005AE220`, `MapSelect__Constructor` | No (campaign/menu only) |
| Asset/tileset load profiling | `CDFileClass__Constructor`, `Pipe__Constructor`, `Read_Theater_TileSets_INI`, others | Load phase only |

---

## 6. The Definitive Boundary

```
PRESENTATION-PACED (wall-clock / GetRadarTimer-driven):
  - Main tick throttle (how fast frames are allowed to execute)
  - FPS/tick-duration performance counter
  - Tactical scroll velocity ramp (16 ms bucket per step)
  - Screen fades / loading transitions
  - Chat message display timeout (bucket-expiry)
  - All network setup/teardown waits
  - Scenario load-time elapsed profiling
  - Movie/audio drain timeouts
  - Map-select UI animation timer

LOGIC-PACED (g_CurrentFrameCounter-driven):
  - CDTimerClass (weapon cooldown, build timers, scenario timers, all gameplay delays)
  - RateTimer / FacingClass (unit rotation interpolation)
  - AnimClass (all INI-Rate-based animation frame advancement)
  - LogicClass__PerTickUpdate gates (ore growth, team AI, factory, house updates, etc.)
  - All modulo gates using (g_CurrentFrameCounter % N == 0)
  - InfantryClass action timers
  - FootClass WalkRate/IdleRate body frame advancement
  - UnitClass ammo/reload frame timers
```

The **boundary** is: anything that controls how many game frames fire (throttle) or how UI presentation feels (scroll ramp, chat labels, fades, audio drain) is wall-clock. Anything that controls what happens during those frames (all damage, movement, animation, timers, AI) is frame-counter.

---

## 7. Implementation Handoff

### 7.1 Tactical auto-scroll acceleration — potential DRIFT

**Verified behavior:** `FUN_00692B60` increments the scroll level (`TacticalClass+0x5548`) once per 16 ms bucket. With `GetRadarTimer() = timeGetTime() >> 4`, one bucket = ~16 ms of real time, independent of game speed.

**Rust delta:** `src/app_camera.rs` (auto-scroll) and the scroll input handler need to use real wall-clock ms, not sim tick count, for the acceleration ramp. If Rust uses sim ticks (which scale with game speed), scroll acceleration will be game-speed-dependent while gamemd always uses real time.

**Affected surface:** Tactical viewport scroll rate when holding the mouse at the screen edge.

**Acceptance scenario:** At game speed 1 AND game speed 6, the time to reach maximum scroll velocity from a standing start should be the same number of real-time milliseconds. At speed 6 the sim runs faster but scroll ramp time does not shrink.

**Proposed test name:** `test_scroll_acceleration_is_wallclock_invariant_across_game_speeds`

**Risk:** Medium. Every player uses edge-scroll in every skirmish. Speed-6 play (common in speedruns/observers) would show a noticeable difference if scroll ramp uses sim ticks.

Evidence: `decompile_function 0x00692B60`; `get_xrefs_to 0x006C8C40` confirms multiple GetRadarTimer calls inside the scroll handler.

### 7.2 Building/muzzle/parachute presentation effects — correct to use wall-clock, but resolution matters

**Verified behavior:** In gamemd, these are NOT on the main `Main_Tick` wall-clock throttle path; they are either `AnimClass` (LOGIC-PACED, frame-counter) or not present in gamemd as a separate subsystem (muzzle flashes are `AnimClass`-driven). The Rust path in `src/app_sim_tick.rs:176-210` passes `sim_elapsed.min(MAX_UPDATE_DELTA_MS)` (capped wall-clock ms) to building anim, muzzle flash, and parachute ticks. This is presentation-paced, which is the correct zone for these, BUT the gamemd equivalents are `AnimClass`-frame-paced. The rate conversion (`900 / Rate * 1000 / 15` in art_data.rs) collapses them to ms, which is only correct at 15 fps wall-clock equivalence.

**Rust delta:** AnimClass-driven effects should use `binary_frame` (derived from `total_sim_ms * 15 / 1000` at `src/sim/world/mod.rs:229-233`) not raw `sim_elapsed` ms.

**Affected surface:** Every explosion, muzzle flash, building damage fire, and parachute animation in the game.

**Acceptance scenario:** A known `Rate=400` anim effect advances frames at the same visible speed at game speeds 1, 3, and 6 — it should advance once every `900/400 = 2` game frames regardless of wall-clock speed of play.

**Proposed test name:** `test_anim_class_rate_is_frame_counter_not_wall_clock`

**Risk:** High. Fires every match on every anim instance.

Evidence: `decompile_function 0x00423AC0` (AnimClass__AI), `0x00427D00` (AnimTypeClass__ReadINI); prior docs confirm frame-counter basis.

### 7.3 Chat message expiry — already wall-clock in gamemd, verify Rust matches

**Verified behavior:** In gamemd, chat labels expire after `param_7` 16 ms buckets via `GetRadarTimer()`. This is wall-clock, independent of game speed.

**Rust delta:** If Rust implements chat label lifetimes using sim ticks, fast game speeds make labels disappear faster and slow game speeds make them linger. The correct implementation uses real-time ms, not tick count.

**Affected surface:** In-game chat in LAN/net modes; also any text overlays that share this label system.

**Acceptance scenario:** A chat message sent at game speed 1 and game speed 6 should both disappear after the same number of real-time seconds.

**Proposed test name:** `test_chat_label_expiry_uses_wallclock_not_sim_ticks`

**Risk:** Low for local skirmish (chat rarely fires). Medium for multiplayer.

Evidence: `decompile_function 0x005D3BA0`, `0x005D4430`.

---

## 8. Negative Facts / Do Not Do

1. **Do NOT model the DAT_00887338/DAT_00887340 global pair as a second throttle.** These are a shared elapsed-bucket baseline used by chat, scroll, audio drain, and loading subsystems. They are NOT a second game-tick rate and NOT a frame-counter. Evidence: `decompile_function 0x005D3BA0`, `0x00685670`, `0x00692B60`.

2. **Do NOT make scroll acceleration speed-dependent.** gamemd's scroll ramp fires on real-time 16 ms buckets, not game frames. Making it game-speed-dependent would be a drift from gamemd behavior. Evidence: `decompile_function 0x00692B60`.

3. **Do NOT classify the per-tick FPS counter (`DAT_00a8b560/DAT_00a8b564`) as a frame-pacing mechanism.** It is a diagnostic accumulator. It does not influence how fast frames fire. Evidence: `decompile_function 0x0055D360` (the counter is written after `FUN_00637550()` and before `Network_ServiceLoop()` — far from the throttle path).

4. **Do NOT implement the network mode timeGetTime branches for local skirmish.** `DAT_00887328/DAT_00887330` (direct ms network budget) and all per-player latency buckets (`FUN_0048D1E0`, `FUN_00542620`, `FUN_006639D0`, etc.) are gated on `g_GameMode != 5`. In local skirmish mode-5, none of these fire. Evidence: `decompile_function 0x0055D360` (mode-5 branch skips the `DAT_00a8b558` block and goes directly to `DAT_00887348 = GetRadarTimer(); DAT_00887350 = uVar19` at LAB_0055d79e).

5. **Do NOT confuse router/WOL timeouts for skirmish-active subsystems.** `FUN_00663DB0` (router process startup, 28 s timeout), `FUN_006639D0` (master-player-list send), and `FUN_00664530` (router goodbye) are WOL-only and never fire in a local YR skirmish. Evidence: callers check `g_GameMode == 4` before entering these functions.

---

## 9. Remaining Uncertainty

1. **Exact units and reset discipline for `DAT_00887338 / DAT_00887340`:** These appear to be a wall-clock baseline (start bucket) and accumulated elapsed (bucket count) but the full reset path — where they are initialized to `-1` / `0` and when `DAT_00887340` is accumulated — was only partially traced. Callers treat `DAT_00887338 == -1` as "not started." Full reset discipline requires tracing write sites (not done in this pass).

2. **Exact scroll level table:** `FUN_00692B60` drives `param_1[0x1552]` (scroll level 0–8 or similar) and the table at `0x0083E748` (9 × int32 speed values) was identified in prior docs but the complete mapping from bucket count to pixel-per-tick was not verified in this pass.

3. **Whether `FUN_005D3BA0` chat labels fire in mode-5 local skirmish:** The function itself does not gate on `g_GameMode`; only its callers might. If a caller fires in local skirmish (e.g., a mission-text overlay sharing this label path), the chat-timeout wall-clock dependence becomes active in skirmish. Not verified in this pass.

---

## 10. Sources

- Ghidra MCP `decompile_function 0x006C8C40` — GetRadarTimer implementation
- Ghidra MCP `get_xrefs_to 0x006C8C40` — all 100 GetRadarTimer callers enumerated
- Ghidra MCP `list_imports` offset 200 — confirmed `timeGetTime` at `EXTERNAL:0000013f`
- Ghidra MCP `decompile_function 0x0055D360` — Main_Tick (FPS counter, throttle, game modes)
- Ghidra MCP `decompile_function 0x004A3C30` — screen fade, wall-clock blend
- Ghidra MCP `decompile_function 0x0048D1E0` — network lobby ready-wait
- Ghidra MCP `decompile_function 0x00542620` — network FSPS diagnostic display
- Ghidra MCP `decompile_function 0x005BDC80` — bink/display chunk with GetRadarTimer snapshot
- Ghidra MCP `decompile_function 0x005D3BA0` — chat label construction with bucket expiry
- Ghidra MCP `decompile_function 0x005D4430` — chat label expiry check
- Ghidra MCP `decompile_function 0x005F0070` — loading-screen bucket wait
- Ghidra MCP `decompile_function 0x006523A0` — network disconnect flush
- Ghidra MCP `decompile_function 0x006639D0` — WOL master-player-list send
- Ghidra MCP `decompile_function 0x00663DB0` — router process startup wait
- Ghidra MCP `decompile_function 0x00664530` — WOL router goodbye
- Ghidra MCP `decompile_function 0x00683610` — ScenarioClass reset with GetRadarTimer init
- Ghidra MCP `decompile_function 0x006832DA` — ScenarioClass__Constructor
- Ghidra MCP `decompile_function 0x00683E3A` — ScenarioClass__Start_Scenario
- Ghidra MCP `decompile_function 0x00685670` — FUN_00685670 (teardown audio drain wait)
- Ghidra MCP `decompile_function 0x00692B60` — tactical auto-scroll acceleration
- Ghidra MCP `decompile_function 0x00648350` — end-game framesync wait
- Ghidra MCP `decompile_function 0x0055CF10` — disconnect gracefully helper
- Ghidra MCP `decompile_function 0x005ADDE0` — MapSelect__Constructor
- Ghidra MCP `decompile_function 0x005AE220` — map select stage timer
- Prior docs: `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`, `VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md`, `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
- Rust files: `src/app_sim_tick.rs`, `src/sim/world/mod.rs`, `src/util/fixed_math.rs`
