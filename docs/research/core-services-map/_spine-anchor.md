# LogicClass::PerTickUpdate — Spine Anchor (per-tick rung ladder)

**Status:** VERIFIED from binary this session.
**Target:** `LogicClass::PerTickUpdate` @ `0x0055AFB0` (Ghidra label
`LogicClassPerTickUpdateLiveVector`). Image base 0x400000.
**Authority:** binary -> Ghidra. The **disassembly** at `disassemble_function 0x0055AFB0`
is ground truth; the decompiler **reordered and mislabeled** several tail calls and
**omitted** at least two calls entirely (`0x00554d50`, `0x004c54a0` appear swapped/elided in
decomp). Every rung below is keyed to a disassembly address.

Verification calls used: `decompile_function 0x0055AFB0`, `disassemble_function 0x0055AFB0`,
`decompile_function 0x0055D360` (Main_Tick), plus per-driver decompiles cited inline.

---

## 0. Function prelude (before any rung)

`disassemble_function 0x0055AFB0`:
- `0055afb3 INC [0x00abcd40]` — **`DAT_00abcd40` += 1**. This is a *profiling/perf tick
  counter* (consumed in `FUN_0055e160` @ `0055e39b` as `DAT_00abcd44 = DAT_00abcd40; reset`),
  **NOT** the gameplay frame counter. The gameplay frame counter `g_CurrentFrameCounter`
  (`0x00a8ed84`) is bumped late in **Main_Tick**, not here.
- `0055afdb MOV [0x00a83cdc],0` — resets the cell-action scan index used by Rung A.

---

## RUNG LADDER (true body order)

Notation: each rung = an ordered call or inline loop the body executes. `vt+0x5c` = virtual
call through slot 0x5c (the per-object AI update slot used by every object-vector tick).

### A. Sidebar / placement cell-action scan + super-weapon ready poll
- **Site:** `0055afe6`–`0055b177` (inline `do/while` over `DAT_008b40d8` count, list
  `DAT_008b40cc`).
- **Driver:** `TechnoClass__ProcessCellAction` @ `0x006e53a0`, dispatched with action codes
  `0x32,0x1b,0x1c,0x24,0x25,0x2d,0x2e,0x0d,0x33,0x0e` depending on
  `g_ScenarioClass_Instance+0x34be/0x34aa/0x34ab` placement-mode flags.
- **Gate:** loop only runs if `DAT_008b40d8 > 0` (count of pending cell-action entries; was
  0 at read time — empty in idle). Inner dispatch gated by placement-mode bytes.
- **Active in YR:** YES — drives build-placement ghost / super-weapon target cursor cell
  evaluation. Empty when no placement/SW targeting is in progress.

### B. Super-weapon recharge timer #1 (Scen+0x47a/0x47c) + redraw
- **Site:** `0055b17d`–`0055b1d2`.
- **Driver:** decrements `g_ScenarioClass_Instance[0x47c]` against `g_CurrentFrameCounter`,
  clears `[0x47a]=-1`, then `FUN_004f42f0(2)` @ `0x004f42f0` (sidebar/flag redraw helper).
- **Gate:** `g_ScenarioClass_Instance[0x47a] != 0xffffffff`.
- **Active in YR:** YES (super-weapon timing).

### C. Clear placement-mode flags
- **Site:** `0055b1d8`–`0055b1fe`. Writes 0 to Scen+0x34aa,0x34a9,0x34ab,0x34be.
- Bookkeeping, not a driver. (Listed for completeness; consumes no RNG.)

### D. Map lighting fade — ambient (Rules+0x1640, RedTint?) 
- **Site:** `0055b205`–`0055b28c`.
- **Driver:** `Math__ftol` (@ `0x007c5f00`) computes new tween value, then
  `FUN_004acac0` @ `0x004acac0` (palette/lighting recalc).
- **Gate:** `*(char*)(g_RulesClass_Instance+0x17f0) != 0` **AND**
  `*(double*)(Rules+0x1640) != 0.0`. Per-target Scen timer at +0x486/0x488.
- **Active in YR:** conditional on a Rules lighting flag; commonly off — record as
  **gated**, do not drop.

### E. RecalcBridgeShroudFlags (frame % 0x78)
- **Site:** `0055b29a`–`0055b2be`.
- **Driver:** `MapClass__RecalcBridgeShroudFlags` @ `0x00578100`.
- **Gate:** `g_CurrentFrameCounter % 0x78 == 0` (every **120** frames). Confirms seed fact.
- **Active in YR:** YES (periodic).

### F. Map lighting fade — second channel (Scen flag 0x1000, Rules+0x1648)
- **Site:** `0055b2c4`–`0055b337`.
- **Driver:** `Math__ftol` then `FUN_004acbc0` @ `0x004acbc0`.
- **Gate:** `(*g_ScenarioClass_Instance & 0x1000) != 0` **AND** `*(double*)(Rules+0x1648) != 0.0`.
  Scen flag 0x1000 is the FogOfWar/Special bit — **TS-legacy-adjacent**; this lighting
  channel is opt-in. Record as **gated**.

### G. IonStorm / weather color interpolation block (Scen+0xd4b/0xd4c)
- **Site:** `0055b33d`–`0055b4d2`. Large inline block.
- **Drivers / gates queried:** `FUN_0053a110`@`0x0053a110` (`DAT_00a9fabc==1`),
  `FUN_0053a120`@`0x0053a120` (`==2`), `FUN_0053bad0`@`0x0053bad0` (`DAT_00a9fab0!=0`),
  `FUN_0053b400`@`0x0053b400` (`DAT_00a9fac0!=0`) — lightning-storm / weather state checks.
  On the chosen branch: `FUN_004ae4c0`@`0x004ae4c0` then `FUN_004f42f0(1)` (lighting redraw).
- **Gate:** `Scen[0xd4c] != Scen[0xd4b]` (current != target color) **AND**
  `*(double*)(Rules+0x1668) != 0.0`, plus Scen+0x492 timer.
- **Active in YR:** YES when a Lightning Storm / weather color tween is in progress; idle
  otherwise.

### H. Tiberium (ore) GROWTH driver
- **Site:** `0055b4d7 CALL 0x00722c40`.
- **Driver:** `TiberiumClass__GrowthDriver_AllTypes` @ `0x00722C40` (confirms seed).
  Walks `g_TiberiumClass_Array` (count `g_TiberiumClass_Array_Count`); per-cell timer at
  +0x11c/+0x124; calls `TiberiumClass__GrowthProcessor`.
- **Gate:** `*(char*)(Scen+0x34a6) != 0` (ore-growth-enabled scenario flag).
- **Active in YR:** YES.

### I. Tiberium (ore) SPREAD driver
- **Site:** `0055b4dc CALL 0x007221b0`.
- **Driver:** `TiberiumClass__SpreadDriver_AllTypes` @ `0x007221B0`. Same array; timer
  +0x100/+0x108; calls `TiberiumClass__SpreadProcessor`.
- **Gate:** `*(char*)(Scen+0x34a6) != 0`.
- **Active in YR:** YES.

### J. BombClass (Ivan bombs / demo charges) update-all
- **Site:** `0055b4e1` (ECX=`0x87f5d8`) `CALL 0x00438bf0`.
- **Driver:** `BombClass__UpdateAll` @ `0x00438BF0`. Walks the bomb vector
  (`this+0x4`/`this+0x10`); detonates entries whose `[0xb]==0`, plays `VocClass__PlayAt`,
  proximity-defuse logic every 0x2d (45) frames via `this+0x30` counter.
- **Gate:** unconditional (loop empty if no bombs).
- **Active in YR:** YES (Crazy Ivan / Terrorist demo).

### K. Spark/sparkle periodic driver (FUN_0054e4d0)
- **Site:** `0055b4eb` (ECX=`0xabc5f8`) `CALL 0x0054e4d0`.
- **Driver:** `FUN_0054e4d0` @ `0x0054E4D0`. Self-timed (30-frame interval via this+0/+2);
  walks `this+0x4` count `this+0x1c`(?); per-entry calls vt+0x1bc / vt+0x3c8 / vt+0x1e8,
  consuming `RateTimer__Current` -> `MapCoord_StepByDir_GetCell`. Appears to be a periodic
  particle/animation re-anchor driver on a small registered list.
- **Gate:** internal timer (`this+0x2`); fires every ~30 frames.
- **Active in YR:** conditional on list being non-empty; record as a real rung.

### L. TeamClass cull-and-tick (two-phase)
- **Site (build temp list):** `0055b4f5`–`0055b582` — inline loop over
  `g_TeamClass_Array` @ `0x008b40ec` count `0x008b40f8`, filtered through a
  `DynamicVector`-style functor (`PTR_FUN_007e9f64`, vt+0x8 predicate) into a stack temp
  vector built by `FUN_0055bb40`@`0x0055bb40`.
- **Site (tick):** `0055b582`–`0055b5a1` — loop the temp list, call **vt+0x5c** on each
  TeamClass.
- **Driver:** `TeamClass::AI` via slot 0x5c.
- **Gate:** count > 0.
- **Active in YR:** YES (team/AI script objects). NOTE: AI gameplay itself is out of current
  project scope, but the rung's ORDER is part of the lockstep contract.

### M. DiskLaserClass update (reverse walk)
- **Site:** `0055b5a1`–`0055b5be` — `for(i = DAT_008a0218-1; i>=0; i--)` over
  `DAT_008a020c[i]`, call **vt+0x5c**. (Decomp mislabeled this `g_DiskLaserClass_Array`; the
  true globals are `0x008a020c`/`0x008a0218`.)
- **Driver:** disk-laser per-object AI (slot 0x5c).
- **Gate:** unconditional reverse loop.
- **Active in YR:** YES (Prism/Disk effects when present).

### N. **FUN_005FF390 — the previously-missing rung** (laser-fence/segment timer purge)
- **Site:** `0055b5be CALL 0x005ff390`.
- **Driver:** `FUN_005FF390` @ `0x005FF390`. Walks `DAT_00ac167c[]` count `DAT_00ac1688`,
  advances each entry's `+0xc` timer by **8**, purges (via functor vt+0x10 on `DAT_00ac1678`
  + `FUN_007c8b3d` free) once the timer passes **0x4f (79)**. **Confirms the seed fact: this
  is an ordered driver sitting between the disk-laser loop (M) and LaserDrawClass (O).**
- **Gate:** unconditional reverse loop (empty if list empty).
- **Active in YR:** YES (timed laser/draw segments).

### O. LaserDrawClass::UpdateAllAI
- **Site:** `0055b5c3 CALL 0x00550150`.  **(decomp showed this as a separate `0x00550150`
  but mislabeled order)**
- **Driver:** `LaserDrawClass__UpdateAllAI` @ `0x00550150`. Reverse-walks
  `g_LaserDraw_Array` count `g_LaserDraw_Count`; advances each laser's draw timer, toggles
  visibility, removes-and-frees expired (functor `DAT_00abc878` vt+0x10).
- **Gate:** unconditional.
- **Active in YR:** YES (every drawn laser/beam).

### P. LightningStorm / PsychicDominator process
- **Site:** `0055b5c8 CALL 0x0053a6c0`.
- **Driver:** `LightningStorm__Process` @ `0x0053A6C0`. Runs storm state machine
  (`DAT_00a9fabc`), calls `PsychicDominator__Process`, then **`Process_QueuedEvents()`**
  (note: a queued-event flush is nested *inside* this driver), then ticks three bolt arrays
  (`DAT_00a9fa1c`, `DAT_00a9fa64`, `DAT_00a9f9d4`), and spawns new cloud bolts.
  **RNG: consumes `Random__RandomRanged`** (bolt scatter `+/-` range from Rules+0x17a8) when
  spawning bolts.
- **Gate:** internal storm-active state.
- **Active in YR:** YES when a Lightning Storm is active; idle otherwise.

### Q. EMPulseClass update-all (reverse walk)
- **Site:** `0055b5cd`–`0055b5ea` — inline `for(i = DAT_00b04be0-1; i>=0; i--)` over
  `DAT_00b04bd4[i]`, call **vt+0x5c**. (This is the EMP-pulse list; NOT `EMPulseClass__UpdateAll`
  @ 0x004c54a0 — see note.)
- **Gate:** unconditional reverse loop.
- **Active in YR:** YES (EMP weapon pulses).
- **NOTE:** the decompiler labeled `0x004c54a0` as `EMPulseClass__UpdateAll`; the real EMP
  list tick here is the inline reverse loop on `DAT_00b04bd4`. `0x004c54a0` is a separate
  driver (see Rung S).

### R. FUN_00554d50 — shroud / cell-lighting recalc flush
- **Site:** `0055b5ea` (ECX=0x6, DL=0) `CALL 0x00554d50`.
- **Driver:** `FUN_00554D50` @ `0x00554D50`. Time-budgeted (`FUN_005b1e40` timer) flush over
  `DAT_00abca44[]` (count `DAT_00abca50`): saves/restores CellClass lighting via
  `MapClass__Get_CellClass` + `FUN_00484050`/`FUN_00483e30`; periodic re-trigger gated on
  `DAT_00829ae8`/`DAT_00abca78`. **Runs BEFORE the AlphaShape driver (Rung S).** (Decomp
  swapped R and S in its output.)
- **Gate:** `DAT_00abca50 != 0` (queued cells) + internal time budget.
- **Active in YR:** YES (deferred cell relight after shroud/terrain changes).

### S. AlphaShapeClass / cloak-shape purge (FUN_004c54a0)
- **Site:** `0055b5f6 CALL 0x004c54a0`.  **(this call is OMITTED from the decompiler output —
  found only in disassembly)**
- **Driver:** `0x004C54A0` (decomp label `EMPulseClass__UpdateAll`, but body reverse-walks
  `DAT_008a3874` count `DAT_008a3880`, detonating entries whose `[0xc]+[0xb] <=
  g_CurrentFrameCounter` via vt+0x20). Functionally an expiry-purge over a timed-effect list.
  **Label identity is suspect; trust the address + body, not the name.**
- **Gate:** unconditional reverse loop.
- **Active in YR:** YES (timed effect expiry).

### T. MAIN object vector tick (bullets / voxel-anims / particles / etc.)
- **Site:** `0055b5fb`–`0055b61b` — `for(i=0; i<param_1[0x10]; i++)` over `param_1[0x4][i]`,
  call **vt+0x5c**. `param_1` = the LogicClass `this`. This is the **primary live object
  AI fan-out** (bullets, voxel anims, particle systems registered in the main logic vector).
- **Gate:** **unconditional** (count>0). Confirms seed: the main vector is NOT mode-gated.
- **Active in YR:** YES — every bullet/particle/voxelanim ticks here.

### U. AnimClass vector tick — **mode-gated, SEPARATE from T**
- **Site:** `0055b61b`–`0055b64b` — `for(i=0; i<DAT_00a83e10; i++)` over `DAT_00a83e04[i]`,
  call **vt+0x5c**.
- **Gate:** `g_GameMode != 0 && g_GameMode != 5`. **Confirms seed fact: AnimClass list is a
  separate rung AFTER the main vector and is mode-gated; bullets/particles (Rung T) are not.**
- **Active in YR:** YES in skirmish/MP (mode != 0/5). Mode 0/5 are menu/special-render modes
  where standalone anims are skipped.

### V. Wave-splash driver
- **Site:** `0055b64b CALL 0x0053d310`.
- **Driver:** `FUN_0053d310` @ `0x0053D310`. Loops `DAT_00aa0128` times calling
  `Wave_splash_forces` @ `0x0053cbe0`. Confirms seed.
- **Gate:** count>0.
- **Active in YR:** YES (shore/water wave anims).

### W. AlphaShapeClass::PurgeDisabled (+ one-time gradient table init)
- **Site:** `0055b650 CALL 0x00420e90`.
- **Driver:** `AlphaShapeClass__PurgeDisabled` @ `0x00420E90`. One-time builds gradient LUT
  `DAT_0088a118` (gated by `DAT_0089a134`), then reverse-walks `DAT_0088a0f4` count
  `DAT_0088a100`, removing entries flagged disabled (`[0xf]!=0`) via vt+0x20.
- **Gate:** unconditional reverse loop.
- **Active in YR:** YES (cloak/alpha shapes).

### X. MapClass::UpdateCrateRegenTimers
- **Site:** `0055b65a` (ECX=`0x87f7e8`) `CALL 0x0056bbe0`.
- **Driver:** `MapClass__UpdateCrateRegenTimers` @ `0x0056BBE0`. Walks 0x100 crate slots
  (this+0x158 stride 0x10), regenerating expired crates
  (`CrateSlot__ClearAndPreserveTimer` + `MapClass__PlaceCrateAtRandomCell`).
- **Gate:** `g_GameMode != 0 && DAT_00a8b261 != 0` (crates-enabled).
- **Active in YR:** YES when crates enabled.
- **RNG:** `MapClass__PlaceCrateAtRandomCell` consumes RNG on regen.

### Y. Tactical/Display per-tick (g_Tactical vt+0x5c)
- **Site:** `0055b65f`–`0055b667` — `(**(code**)(*(g_Tactical@0x00887324)+0x5c))()`.
- **Driver:** the DisplayClass/TacticalClass per-tick AI (slot 0x5c).
- **Gate:** unconditional.
- **Active in YR:** YES (camera/tactical map logic tick).

### Z. FactoryClass tick
- **Site:** `0055b66a`–`0055b68d` — `for(i=0;i<DAT_00a83e40;i++)` over `DAT_00a83e34[i]`,
  call **vt+0x5c**. (Decomp label `g_FactoryClass_Array`; true globals
  `0x00a83e34`/`0x00a83e40`.)
- **Driver:** `FactoryClass::AI` (production/build progress) slot 0x5c.
- **Gate:** count>0.
- **Active in YR:** YES (all production queues advance here).

### AA. HouseClass tick (null-checked)
- **Site:** `0055b68d`–`0055b6b3` — `for(i=0;i<DAT_00a80238;i++)` over `DAT_00a8022c[i]`,
  null-check then call **vt+0x5c**. (Decomp label `g_HouseClass_Array`; true globals
  `0x00a8022c`/`0x00a80238`.)
- **Driver:** `HouseClass::AI` (economy, power, super-weapon charge, AI build) slot 0x5c.
- **Gate:** count>0, per-entry non-null.
- **Active in YR:** YES (every house ticks).

### AB. DisplayClass last-ref-object follow + temp-vector teardown
- **Site:** `0055b6b3`–`0055b71c`.
- **Driver:** `DisplayClass__GetLastRefObject` @ `0x004AEB10`; if non-null, copies the
  object's coord (+0x9c/0xa0/0xa4) and calls `FUN_006d6070` (camera/audio follow). Then
  `local_18=&PTR_FUN_007e9f84` and `FUN_007c8b3d` frees the Rung-L temp vector.
- **Gate:** last-ref object non-null; temp-vector owned.
- **Active in YR:** YES (selected-object camera/audio tracking). Bookkeeping teardown after.

---

## Seed-fact verdicts

1. **`FUN_005FF390` is a real ordered rung between the disk-laser loop and
   LaserDrawClass** — **CONFIRMED** (Rung N, site `0055b5be`; body loops `DAT_00ac167c`,
   +0xc += 8, purge > 0x4f). Verified `decompile_function 0x005ff390`.
2. **Main vector (param_1+0x4 / +0x10) ticks as one unconditional rung, then a SEPARATE
   AnimClass list (`DAT_00a83e04`/`DAT_00a83e10`) AFTER it, gated `g_GameMode != 0 && != 5`**
   — **CONFIRMED** (Rungs T and U, sites `0055b5fb` and `0055b61b`). Verified in disasm.
3. **PerTickUpdate addr = 0x0055AFB0, Main_Tick = 0x0055D360** — CONFIRMED.
4. **Late g_CurrentFrameCounter+1 + RecalcBridgeShroudFlags @ frame%0x78** — CONFIRMED
   (Rung E for the %0x78 recalc; frame bump is in Main_Tick postlude, see below).
5. **CORRECTION to seed:** the decompiler's *order* in the laser/EMP/shroud region is
   UNRELIABLE. True disasm order is M -> N(005ff390) -> O(LaserDraw 00550150) ->
   P(LightningStorm 0053a6c0) -> Q(EMP inline 00b04bd4) -> R(shroud relight 00554d50) ->
   S(alpha/effect purge 004c54a0) -> T(main vector) -> U(AnimClass). The decomp listed
   EMPulse before 00554d50 and dropped 004c54a0; do not trust it.

---

## Main_Tick (`0x0055D360`) — prelude & postlude

Verified `decompile_function 0x0055D360`.

### Where the LIVE command/event stage runs (relative to PerTickUpdate)

The **only** `Process_QueuedEvents()` call directly in Main_Tick sits inside the
**offline-spectator branch** (`*(int*)(g_ScenarioClass_Instance+0x62c) != 0`, inside the
`g_GameMode==0||==5` block at `LAB_0055d821`): it runs `Process_NetworkMessages();
Network_ServiceLoop(); Process_QueuedEvents(); g_Tactical.vt+0x5c; RenderFrame_main();
FUN_0055e160(); return;` — i.e. that branch **returns early** and never reaches
PerTickUpdate. So that is NOT the live-gameplay command stage. **Confirms seed.**

The **live gameplay** command/event stage is the block guarded by
`((DAT_00a8d5f8 & 2)==0) && g_GameState==0 && g_GameRunning!=0`, which runs **before**
`LogicClassPerTickUpdateLiveVector()`:
1. `GScreenClass__Input(...)` — collect local input.
2. `Process_Command()` @ — translate input into queued game commands.
3. (debug overlay if `DAT_00a8b8b4`)
4. `Network_Keepalive()` if `(g_CurrentFrameCounter & 7)==7 && g_GameMode==4`.
5. **`Map__Logic()`** — this is where the **live command/event queue is executed**
   (commands/events committed into world state) for the current frame, plus map-level
   per-frame logic.
6. `RenderFrame_main()`.

Then, separately, the replay/desync save+verify block runs (`DAT_00a8d5f8 & 1` = record
state hash to stream; `& 2` = read+compare, calling `Desync_Handler()` on mismatch and
`Process_QueuedEvents()`+render in the playback path).

Then `FUN_00551a30()`, then (mission-0 one-time scenario-name setup), then:
**`LogicClassPerTickUpdateLiveVector()`** — i.e. the whole rung ladder above.

So per frame the order is: **Input -> Process_Command -> (keepalive) -> Map__Logic
(command/event execution) -> RenderFrame -> [state-hash record/verify] -> PerTickUpdate
(object/world simulation)**.

### Postlude (after PerTickUpdate, before return)

After PerTickUpdate returns, Main_Tick: builds an audio/ambient sound-volume value
(`FUN_0054f5c0` checks + `Math__ftol`), fires up to 4 `FUN_004a9840(...)` ambient-loop
updates gated by `DAT_00abce14` bits (0x100/0x1000/0x1/0x10), `FUN_00637550()`,
`FUN_005d4430()`, an optional `Random__RandomRanged(0,2)` for a cell-anim flutter when
`g_GameMode==3||4` and conditions on the current cell hold, accumulates frame-time stats
(`DAT_00a8b560`/`DAT_00a8b564`), `FUN_00647260()`, `FUN_00637550()` again, then
`Network_ServiceLoop()`.

**Frame-counter bump:** guarded by
`DAT_00a83d49==0 && DAT_00a8ecd0==0 && DAT_008b41c0==0 && DAT_00a83d48==0` (no
pause/reconnect/desync flags), it executes:
- **`g_CurrentFrameCounter = g_CurrentFrameCounter + 1`** (`0x00a8ed84`) — the late gameplay
  frame-counter increment. Confirms seed.
- mission-time-limit check (`DAT_00b07784` -> `FUN_00684290`),
- **`FUN_0055e160()`** — frame-pacing / timing throttle (waits out the frame budget,
  rolls the perf counter `DAT_00abcd40` into `DAT_00abcd44`),
- **`FUN_00725c70()`** — deferred object-destruction/cleanup purge over `DAT_00b0f69c`
  (vt+0x44 "ready to remove" check then vt+0x20 delete),
- **`FUN_00637270()`** — waypoint / plan-manager (build-queue planning UI) flush over
  `DAT_00ac4c7c` / `DAT_00ac4c9c`,
- clears `DAT_00abcd58=0`, returns.

(The render trigger for the live path is `RenderFrame_main()` called *within* the
input/logic block above, not in the postlude.)
