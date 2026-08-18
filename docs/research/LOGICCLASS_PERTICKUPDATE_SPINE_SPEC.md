# LogicClass::PerTickUpdate — Spine Spec (per-tick rung ladder, lockstep contract)

**Status:** VERIFIED from binary. Authority order = **binary → Ghidra → docs**. The
**disassembly** at `disassemble_function 0x0055AFB0` is ground truth; the decompiler
**reordered and mislabeled** several tail calls and **omitted** at least two
(`0x00554d50`, `0x004c54a0`). Every rung below is keyed to a disassembly address and a
per-rung verified record.

**Target:** `LogicClass::PerTickUpdate` @ `0x0055AFB0` (Ghidra label
`LogicClassPerTickUpdateLiveVector`). Sole caller `Main_Tick` @ `0x0055D360`. Image base
`0x400000`.

**What this doc is:** the synthesized, authoritative per-tick spine — prelude → 28 ordered
rungs → postlude — plus the **RNG-draw order** (the multiplayer lockstep contract). It
supersedes the rung list in `CORE_ENGINE_SERVICES_MAP.md §4` (which now points here) and is
built from `core-services-map/_spine-anchor.md` + the per-rung records in
`core-services-map/_spine-rung-*.md`.

**Per-rung evidence:** each rung cites its driver address; full Ghidra-call evidence lives in
`docs/research/core-services-map/_spine-rung-<N>.md` and `_spine-anchor.md`. Addresses below
were verified via `disassemble_function 0x0055AFB0` (body) plus per-driver
`decompile_function` / `disassemble_function` / `get_function_callers` /
`get_xrefs_to` calls recorded in those rung files.

---

## 1. Where the live command/event stage runs (Main_Tick prelude)

`Main_Tick @ 0x0055D360` calls `PerTickUpdate @ 0x0055AFB0` exactly once per frame and bumps
the gameplay frame counter **late** (postlude), so the entire rung ladder reads the
**pre-increment** frame clock `g_CurrentFrameCounter` (`0x00a8ed84`).

The **live gameplay** command/event stage runs **before** PerTickUpdate, inside the block
guarded by `((DAT_00a8d5f8 & 2)==0) && g_GameState==0 && g_GameRunning!=0`:

1. `GScreenClass__Input(...)` — collect local input.
2. `Process_Command()` — translate local input into queued game commands.
3. `Network_Keepalive()` when `(g_CurrentFrameCounter & 7)==7 && g_GameMode==4`.
4. **`Map__Logic()`** — **this is where the live command/event queue is executed** (commands
   committed into world state) for the current frame, plus map-level per-frame logic.
5. `RenderFrame_main()`.

Then the replay/desync state-hash record/verify block (`DAT_00a8d5f8 & 1` = record hash to
stream; `& 2` = read+compare, `Desync_Handler()` on mismatch), then `FUN_00551a30()`, then a
one-time mission-0 scenario-name setup, then **`PerTickUpdate()`** (the rung ladder below).

> **Command-queue framing (corrected):** `Process_QueuedEvents @ 0x0053B560` is **not** the
> per-frame live command stage in the normal path. In the live path the command/event queue
> is executed inside **`Map__Logic()`**, before PerTickUpdate. The only direct
> `Process_QueuedEvents` call in Main_Tick sits in the **offline-spectator early-return
> branch** (`Scen+0x62c != 0` inside the `g_GameMode==0||==5` block), which returns before
> ever reaching PerTickUpdate — so that call is **not** the live-gameplay command stage.
> (`Process_QueuedEvents` also appears nested inside Rung P's storm driver; not a per-tick
> command stage either.)

Per-frame order: **Input → Process_Command → (keepalive) → Map__Logic (command/event
execution) → RenderFrame → [state-hash record/verify] → PerTickUpdate (this ladder).**

### Function prelude (inside PerTickUpdate, before Rung A)
- `0055afb3 INC [0x00abcd40]` — **profiling/perf tick counter**, NOT the gameplay frame
  counter (rolled into `DAT_00abcd44` in `FUN_0055e160`). Do not confuse with
  `g_CurrentFrameCounter`.
- `0055afdb MOV [0x00a83cdc],0` — resets the tag/cell-action scan iterator used by Rung A.

---

## 2. Rung ladder (true disassembly order)

`vt+0x5c` = virtual call through slot 0x5c (the per-object AI update slot). "RNG" column =
streams drawn and the per-tick draw count, where **Scen->Random** = the synchronized
lockstep stream at `ScenarioClass+0x218` (`0x00a8b230 → +0x218`), **g_MainRng** = the
non-synchronized UI/cosmetic stream at `0x00886b88`.

| # | Label | Driver @ addr | Walks | Gate | Active-in-YR | TS-legacy | Draws RNG (stream · count) |
|---|---|---|---|---|---|---|---|
| 1 | A. Tag/map-trigger event scan + SW-ready poll | `0x006e53a0` (TagClass per-event trigger eval — label "TechnoClass__ProcessCellAction" is DRIFT) | global TagClass array `DAT_008b40cc` count `DAT_008b40d8`; event codes 0x32/0x1b/0x1c/0x24/0x25/0x2d/0x2e/0xd/0x33/0xe | entry: tag count `DAT_008b40d8 > 0`; per-event sub-gates on Scen placement/mode bytes (+0x34be,+0x34aa,+0x34ab,+0x11e8 timer) | conditional — empty in skirmish (no tags), full on campaign/scripted maps | no | none directly (transitive only if a fired trigger action is RNG-drawing; 0 in skirmish) |
| 2 | B. SW recharge/redraw timer #1 + redraw | `0x004f42f0` (generic sidebar/tactical redraw helper) | inline scenario timer slot Scen+0x11e8/+0x11f0; sets redraw flags + `0x00578ac0` counter bump | runs iff `*(u32*)(Scen+0x11e8) != 0xFFFFFFFF` (slot armed) | conditional — fires the tick the slot elapses, refreshes sidebar/tactical | no | none |
| 3 | C. Clear placement-mode flags | `0x0055AFB0` inline `0055b1d8-0055b1fe` (no callee) | 4 byte stores to Scen +0x34aa/+0x34a9/+0x34ab/+0x34be = 0 | unconditional | yes (bookkeeping; one-shot flags consumed by Rung A) | no | none |
| 4 | D. Shroud-regrowth (shroud creep) pass | `0x004acac0` | __fastcall on g_Map; two 512×512 cell sweeps + reveal-notify finalize | `Rules+0x17f0 (ShroudGrow)!=0` AND `Rules+0x1640 (ShroudRate)!=0.0` AND Scen timer (+0x1218/+0x1220) elapsed | **no** — stock `ShroudGrow=no`, first gate byte 0, skipped every tick | **yes** | none |
| 5 | E. RecalcBridgeShroudFlags (frame % 120) | `0x00578100` | bridge cells, two-pass; queue changed cells for redraw | `g_CurrentFrameCounter % 0x78 == 0` (every 120 frames) | yes — runs every match on 120-frame cadence; real work where bridges exist | no | none |
| 6 | F. FogOfWar re-shroud / lighting 2nd channel | `0x004acbc0` | all cells via CellIterator, two passes; recursive re-shroud `0x004acc50`; re-cloak pass | `(Scen & 0x1000)!=0` (FogOfWar Special bit) AND `Rules+0x1648 != 0.0`; Scen timer +0x1224/+0x122c | **conditional** — FogOfWar Special 0x1000 defaults OFF in YR, skipped every tick (single TEST AH,0x10/JZ) | **yes** | none |
| 7 | G. IonStorm/weather color interpolation | `0x004ae4c0` (+ `0x004f42f0(MapClass,1)` redraw; storm gates 0x0053a110/0053a120/0053bad0/0053b400) | inline `0055b33d-0055b4d2`; walks every cell via CellIterator, `Cell_ComputeZAdjust 0x00484680` per cell | `Scen[0xd4c] != Scen[0xd4b]` (target≠current color) AND `Rules+0x1668 != 0.0` AND timer (Scen[0x492]/[0x494]) elapsed | yes (conditional) — fires when a storm/SW color tween is active (Psychic Dominator / Lightning Storm) | no | none |
| 8 | H. Tiberium GROWTH driver (all types) | `0x00722C40` | `g_TiberiumClass_Array` base `0x00b0f4ec` count `0x00b0f4f8`; per-type timer +0x11c/+0x124; `GrowthProcessor 0x00722f00` | `Scen+0x34a6 != 0` (ore-growth enabled) AND array count > 0 | yes — standard ore regrowth, player-visible | no | **Scen->Random** · data-dependent: per fired type, 1 (grow-count) + 1/grown cell (next-frame jitter) + 1/spreadable cell (spread-queue jitter); 0 on ticks where no type's timer due |
| 9 | I. Tiberium SPREAD driver (all types) | `0x007221B0` | same array; per-type timer +0x100/+0x108; `SpreadProcessor 0x00722440` | `Scen+0x34a6 != 0` (same gate as H); call site unconditional, gating internal | yes — ore/gem field spread, player-visible economy | no | **Scen->Random** · 1 draw per due type per tick (step-budget pick); 0 when queue empty / density too low |
| 10 | J. BombClass update-all (Ivan bombs / demo charges) | `0x00438BF0` | this=`0x0087f5d8`; bomb array +0x04 count +0x10; 3 reverse passes; 45-frame defuse-visibility throttle | unconditional (empty-list no-op) | yes — Crazy Ivan / Demo Truck: ticking sound + bomb indicator | no | none |
| 11 | K. Periodic spawn re-anchor / retreat driver | `0x0054e4d0` | spawn-retreat list `0xabc5f8`: timer +0/+4/+8, entries +0x10 count +0x1c; per-entry facing-step / set-position | internal 30-frame self-timer (`frame - this[0] >= this[2]`, re-arm 0x1e); no-op when count ≤ 0; call site unconditional | conditional — works only on ~30-frame interval AND when a spawn is queued (carrier hornets, dreadnought/boomer missiles) | no | none |
| 12 | L. TeamClass cull-and-tick | `0x006e9140` (TeamClass::AI, vt+0x5c) | build temp DynamicVector from `g_TeamClass_Array 0x008b40ec` count `0x008b40f8` (builder `0x0055bb40`), then vt+0x5c per surviving team | count-gated only (array count > 0 to build, temp count > 0 to tick); NO game-mode gate | yes — AI/triggers create teams; drives visible AI behavior | no | **Scen->Random** · 0 or 1 per ticked team: only script opcode 0x36 (Convoy random move) with no enemy-house ref → 1 RandomRanged(0,0xff) facing (`0x006efadc`); atan2-toward-enemy branch draws 0 |
| 13 | M. DiskLaserClass update (reverse walk) | `0x004a7340` (DiskLaserClass::AI, vt+0x5c) | `DAT_008a020c[]` count `DAT_008a0218`, reverse | unconditional reverse loop (emptiness check only) | yes — Floating Disc ring-laser; visible whenever a disc attacks | no | **Scen->Random** · 0 common case; transitive only on FIRE tick via `Apply_area_damage 0x00489280` when disc lands on a destroy-on-damage overlay cell (debris/particle RandomRanged(0,99)); bridge rolls NOT reached ([DiskWH] Wall=no) |
| 14 | N. Laser/draw-segment timer purge | `0x005FF390` | DynamicVector `DAT_00ac167c` count `DAT_00ac1688`, reverse; ages each +0xc timer by 8; deletes once > 0x4f | unconditional reverse loop (no-op when count 0) | yes — list filled by particle spark + laser/lightning draw path (Tesla/Prism beams, sparks); no-op only on empty ticks | no | none |
| 15 | O. LaserDrawClass::UpdateAllAI | `0x00550150` | `g_LaserDraw_Array 0x00abc87c` count `0x00abc888`, reverse; advance anim-step/repeat, blink flag, cull expired | unconditional | yes — backing renderer for stock IsLaser/IsBigLaser (Prism, Mirage, IFV, Tank Destroyer, Disk, railgun) | no | none |
| 16 | P. LightningStorm / PsychicDominator process | `0x0053A6C0` | 3 bolt-array reverse walks (`0x00a9fa1c`/`0x00a9fa64`/`0x00a9f9d4`); nests PsychicDominator process + Process_QueuedEvents; cloud-bolt spawn branch | unconditional call; internal: bolt walks unconditional, spawn branch gated storm-active (`0x00a9fab4 != 0 && 0x00a9fad0 == 0`), scatter sub-branch gated `frame % LightningScatterDelay == 0` | yes — Weather/Lightning Storm (Soviet) + Psychic Dominator (Yuri) stock SWs | no | **Scen->Random** · 2 per scatter attempt (X,Y offset of scatter bolt), up to 3 attempts on a qualifying tick → **0/2/4/6** draws; each rejected candidate still costs its 2; 0 otherwise. Nested PD/queued-events draw 0 |
| 17 | Q. RadSiteClass list tick (reverse walk) | `0x0065b800` (RadSiteClass::AI, vt+0x5c; "EMPulseClass" label is DRIFT) | `DAT_00b04bd4[]` count `DAT_00b04be0`, reverse | unconditional reverse loop | yes — RadSites from radiation warheads (Desolator); radiation puddle + green glow + DoT | no | none (rad damage adjusts CellClass rad level, not a unit damage roll) |
| 18 | R. Deferred cell-lighting recalc flush | `0x00554D50` | deferred relight queue (base `0x00abca44`, count `0x00abca50`); snapshot phase then time-budgeted apply phase | call unconditional; snapshot gated `0x00abca50 != 0 && 0x00abca84 == 0`; apply gated `0x00abca84 != 0 && (force OR 6ms budget)` | yes — dynamic per-cell lighting (production glow on/off, sold/destroyed building un-light) | no | none |
| 19 | S. EMPulseClass expiry purge | `0x004C54A0` (EMPulseClass__UpdateAll — expiry/destroy sweep) | `DAT_008a3874[]` count `DAT_008a3880`, reverse; destroy expired via vt+0x20 | unconditional reverse loop (per-entry expiry `[+0x30]+[+0x2c] <= frame`) | **no** — EMP warhead `;gs disabled` in stock rulesmd.ini; [EMPulseWeapon] unassigned; list stays empty | **yes** | none (the family's only draw — RandomRanged(0,0x19) in EMPulseClass__Apply `0x004c54e0` — is at construction, NOT this expiry sweep) |
| 20 | T. MAIN object vector tick (universal per-object AI fan-out) | `0x005F3E70` base (ObjectClass::AI, vt+0x5c; polymorphic: UnitClass `0x007360C0`, InfantryClass `0x0051BAB0`, AircraftClass `0x00414BB0`, FootClass `0x004DA530`→TechnoClass::AI_Update `0x006F9E50`, BuildingClass, bullets/voxelanims/particles) | LogicClass live vector: base `param_1[0x04]` count `param_1[0x10]` (`param_1 = 0x0087f778`), FORWARD, count re-read each iter; members = all ObjectClass-derived registered via Reveal | unconditional in practice (count > 0; always >0 in a loaded skirmish); NOT mode-gated | yes — advances every unit/building/projectile/effect (the bulk of gameplay) | no | **Scen->Random | g_MainRng** · NOT statically enumerable — scales with live-object count and which AI branches fire. Gameplay AI draws → Scen->Random (e.g. AI_Update RandomRanged `0x0065c7e0`); cosmetic voice/sound → g_MainRng (FootClass::AI `0x004daac0`). Base ObjectClass::AI draws 0 |
| 21 | U. AnimClass-subset vector tick (MODE-GATED, separate from T) | `0x00423ac0` (AnimClass::AI, vt+0x5c) | secondary anim DynamicVector `DAT_00a83e00`: base `DAT_00a83e04` count `DAT_00a83e10` (cap 0xa); FORWARD, count re-read; occupants = MoveFlash anims | `g_GameMode (0x00a8b238) != 0 && != 5` AND count > 0 | yes — MoveFlash click-feedback created on essentially every move/attack order; visible every match | no | **Scen->Random** · all 6 RandomRanged sites bind Scen+0x218, but stock occupants (MoveFlash) set none of the triggering AnimType fields → **expected 0 draws/tick** from this vector's stock contents |
| 22 | V. Wave-splash (psychic-wave ripple) driver | `0x0053d310` → worker `0x0053cbe0` | reverse loop over `DAT_00aa0128` wave entries; frame-0 epicentre burst + area damage + -3..+3 ring jitter; expire at frame > 78 | call site unconditional (count is internal loop bound); cheap no-op at 0 waves | yes — Psychic Dominator SW + map "create wave" trigger action (case 0x5e) | no | **Scen->Random** · driver itself draws 0 (deterministic RateTimer/Cos/Sin/Sqrt); transitive only via `Apply_area_damage(Rules+0xff0)` debris rolls when an overlay is destroyed (RandomRanged(0,99)); bridge/wall rolls bypassed (warhead==Rules+0xff0); common case 0 |
| 23 | W. AlphaShapeClass::PurgeDisabled (+ one-time LUT init) | `0x00420E90` | global AlphaShape array `DAT_0088a0f4[]` count `DAT_0088a100`, reverse; delete entries with +0x3c flag set via vt+0x20; one-time 0x10000-entry gradient LUT on first call | unconditional; one-time LUT gated `DAT_0089a134 == 0`; purge no-op at count 0 | yes — AlphaImage= overlays (light posts); non-empty only when such an object is disabled/limboed that tick | no | none |
| 24 | X. MapClass::UpdateCrateRegenTimers | `0x0056BBE0` | 256 crate slots at MapClass+0x158 stride 0x10, forward; due-slot → `PlaceCrateAtRandomCell 0x0056bd40` | `g_GameMode (0x00a8b238) != 0` AND crates-enabled `DAT_00a8b261 != 0` | conditional — only with Crates option on; respawned crate is player-visible | no | **Scen->Random** · driver itself 0; per due slot, `PlaceCrateAtRandomCell` does up to 1000 attempts × **2 draws/attempt** (random X then Y), stops on first valid → 2·N per due slot (min 2), 0 if no free slot |
| 25 | Y. Tactical/DisplayClass per-tick | `0x006d2540` (TacticalClass::AI, vt+0x5c) | single object g_Tactical (`0x00887324`); camera-scroll interp + view commit + radar-refresh timer | unconditional at body site; internal early-out when `DAT_00a8d5f8 & 2` set; per-frame dedup via this+0xa8 == frame | yes — tactical view scrolls + radar refreshes every frame; only the display-suppress guard pauses it | no | none (wall-clock timeGetTime only) |
| 26 | Z. FactoryClass tick (production / build progress) | `0x004C9B20` (FactoryClass::AI, vt+0x5c) | `g_FactoryClass_Array 0x00A83E34` count `0x00A83E40`, FORWARD, count re-read; 54-step pay-as-you-go state machine | count > 0; NO game-mode gate; per-factory internal short-circuits (suspended / empty queue / complete / CDTimer throttle) | yes — every-match production: build-bar fill cadence, On Hold stall, credit drain, completion flash | no | none (per-step cost is integer IDIV; callees = GetTimeRemaining + Spend_Money) |
| 27 | AA. HouseClass tick (economy/power/SW/AI; null-checked) | `0x004F8440` (HouseClass::AI/Update, vt+0x5c) | `g_HouseClass_Array 0x00A8022C` count `0x00A80238`, FORWARD, per-entry non-null guard | count > 0 AND entry non-null; NO game-mode gate at body site | yes — power/EVA/SW-ready/AI production/defeat detection for every house | no | **Scen->Random | g_MainRng** · up to 3 draws, ALL gated to local player house AND `g_GameMode==3||4` (network) AND non-spectating: (1) g_MainRng (0,1) one-time, (2) g_MainRng (0,2) per-tick, (3) **Scen->Random (0,2)** only when picked cell holds a live occupant (result discarded). The Scen->Random draw is **local-player-gated → 0 synchronized draws on non-local/AI houses**; g_MainRng draws are non-lockstep |
| 28 | AB. Last-ref-object camera follow + temp-vector teardown | `0x004AEB10` (gate GetLastRefObject) → `0x006D6070` (camera follow); `0x007C8B3D` (free Rung-L temp) | single object: read last-ref obj coords +0x9c/+0xa0/+0xa4, recenter camera + minimap; then free Rung-L temp vector | last-ref non-null (`Display+0x119c != 0 && Display+0x11a0 != 0`); temp-free gated owned-buffer | yes — camera recenters every tick a last-ref object is set (player-visible); temp-free whenever Rung L allocated | no | none |

---

## 3. RNG-draw order across the tick (the lockstep contract)

Rung **order AND per-draw order within a rung are the multiplayer lockstep contract** —
reordering any drawing rung, or any individual draw inside it, shifts every later RNG result
and desyncs. The ordered subsequence of rungs that **can** draw, with the stream each uses:

```
H  (8)  Tiberium GROWTH          → Scen->Random   (data-dependent; 0 if no type due)
I  (9)  Tiberium SPREAD          → Scen->Random   (1 per due type; 0 if none due)
L  (12) TeamClass AI             → Scen->Random   (0 or 1 per team; only opcode 0x36)
M  (13) DiskLaser AI             → Scen->Random   (0 common; transitive area-damage debris only)
P  (16) LightningStorm/PD        → Scen->Random   (0/2/4/6 on qualifying ticks)
T  (20) MAIN object vector       → Scen->Random AND g_MainRng (bulk; per-callsite binding)
U  (21) AnimClass subset         → Scen->Random   (0 with stock MoveFlash occupants)
V  (22) Wave-splash              → Scen->Random   (0 common; transitive overlay-destroy debris)
X  (24) Crate regen              → Scen->Random   (2·N per due slot; 0 if no slot due)
AA (27) HouseClass               → g_MainRng (×2 UI, local-only) AND Scen->Random (×1, local-only → 0 synchronized on AI/remote)
```

**Synchronized (Scen->Random) lockstep order is H → I → L → M → P → T → U → V → X** (AA's
Scen->Random draw is local-player-gated, so it consumes **0 synchronized draws** on
non-local/AI houses; including it on the local house would still place it last). The two ore
rungs (H growth before I spread) share the stream, so their relative order is itself part of
the draw order. **g_MainRng** (cosmetic/UI, non-synchronized) is touched only inside **T**
(voice/sound) and **AA** (local-player UI rolls).

Rungs **A, B, C, D, E, F, G, J, K, N, O, Q, R, S, W, Y, AB draw zero RNG** and are
RNG-neutral — they hold their slot in the order but never advance any cursor.

---

## 4. Active-in-YR vs legacy/gated split

**Active every match (run + do real work in a normal YR skirmish):**
- C (clear placement flags), E (bridge-shroud %120), H/I (ore growth/spread), J (Ivan/demo
  bombs — with a Crazy Ivan present), L (team AI), M (disk laser — disc attacks), N (laser
  segment purge), O (LaserDraw — all laser beams), P (storm/dominator — when an SW is up), Q
  (RadSite — radiation content), R (deferred relight), T (main object vector — always), U
  (AnimClass MoveFlash — every move/attack order), V (wave-splash — Dominator/trigger), W
  (alpha-shape purge), Y (tactical view), Z (factories), AA (houses), AB (camera follow).

**Conditional (run every tick but only do work under a session/option/state condition;
NOT TS-legacy):**
- A (tags — empty in skirmish, full on scripted maps), B (SW recharge slot — only the tick
  it elapses), G (storm color tween — when active), K (spawn retreat — ~30-frame interval +
  queued spawn), X (crate regen — Crates option on).

**Gated / TS-legacy (skipped in a normal YR skirmish; kept in the ORDER for lockstep):**
- **D. Shroud-regrowth** — TS-legacy. Stock `ShroudGrow=no` → first gate byte 0 → skipped
  every tick. (`ini/rulesmd.ini` ShroudGrow=no.)
- **F. FogOfWar re-shroud / 2nd lighting channel** — TS-legacy. Gated on Special `0x1000`
  (FogOfWar), which defaults OFF in YR → skipped every tick.
- **S. EMPulseClass expiry purge** — effectively TS-legacy in stock: EMP warhead
  `;gs disabled` and [EMPulseWeapon] unassigned, so the list stays empty (loop never
  iterates).

> All three remain ordered rungs of the spine: their **position** is part of the lockstep
> contract even though they consume zero RNG and produce no visible effect in stock YR. Do
> not drop them from the ladder.

---

## 5. Main_Tick postlude (after PerTickUpdate, before return)

After PerTickUpdate returns, Main_Tick:
- Builds an audio/ambient sound-volume value (`FUN_0054f5c0` + `Math__ftol`), fires up to 4
  `FUN_004a9840` ambient-loop updates gated by `DAT_00abce14` bits, `FUN_00637550()`,
  `FUN_005d4430()`, an optional `Random__RandomRanged(0,2)` cell-anim flutter when
  `g_GameMode==3||4` and the current cell qualifies, accumulates frame-time stats,
  `FUN_00647260()`, `FUN_00637550()` again, then `Network_ServiceLoop()`.
- **Frame-counter bump** (guarded by no pause/reconnect/desync flags:
  `DAT_00a83d49==0 && DAT_00a8ecd0==0 && DAT_008b41c0==0 && DAT_00a83d48==0`):
  - **`g_CurrentFrameCounter += 1`** (`0x00a8ed84`) — the late gameplay-frame increment; the
    whole tick read the pre-increment value.
  - mission-time-limit check (`DAT_00b07784 → FUN_00684290`),
  - **`FUN_0055e160()`** — frame-pacing/timing throttle (rolls perf counter
    `DAT_00abcd40 → DAT_00abcd44`),
  - **`FUN_00725c70()`** — deferred object-destruction/cleanup purge over `DAT_00b0f69c`
    (vt+0x44 ready-to-remove → vt+0x20 delete),
  - **`FUN_00637270()`** — waypoint / plan-manager flush over `DAT_00ac4c7c`/`DAT_00ac4c9c`,
  - clears `DAT_00abcd58 = 0`, returns.

> The postlude's optional `Random__RandomRanged(0,2)` cell-anim flutter is in **Main_Tick**,
> not in the PerTickUpdate ladder, and is gated `g_GameMode==3||4` — it is outside the
> §3 PerTickUpdate draw order.

---

## 6. Open / unverified

- **Rung A transitive RNG.** If an author-placed trigger action fires `TriggerAction__Execute
  0x006dd8b0`, an RNG-drawing action type could draw; the action dispatcher's per-action draw
  set was not exhaustively enumerated. Conditional on a trigger firing; 0 in skirmish. Stream
  of any such draw is per-callsite ECX — **UNCHECKED**.
- **Rung L opcode coverage.** Only the ~0x40-opcode dispatch body + opcodes 0x36 / 0x2c /
  0x2d were checked for draws; other team-script opcode helpers were not exhaustively swept.
  No draw found in the dispatch body itself; remaining helpers **UNCHECKED**.
- **Rung T draw enumeration.** The per-tick draw count is data-dependent (live-object count ×
  which AI branches fire) and is **not statically enumerable**; only the stream binding rule
  (gameplay→Scen->Random, cosmetic→g_MainRng) and a few concrete call sites are verified. The
  full set of drawing AI branches under the polymorphic vt+0x5c subtree is **UNCHECKED**.
- **Rung U non-stock occupants.** The non-zero RandomRanged paths in AnimClass::AI exist but
  are unreachable from this vector's stock MoveFlash contents; whether any non-stock anim ever
  enters `DAT_00a83e04` and triggers them is **UNCHECKED** (fires under Rung T / at
  construction for general anims).
- **Two-stream cursor parity in Rust.** The `g_MainRng` vs `Scen->Random` split is the
  lockstep boundary; Rust must keep the synchronized cursor draws on Scen->Random in exactly
  the §3 order. Whether the port currently splits the two streams is **out of scope here**
  (design caution, not a binary fact).
```
