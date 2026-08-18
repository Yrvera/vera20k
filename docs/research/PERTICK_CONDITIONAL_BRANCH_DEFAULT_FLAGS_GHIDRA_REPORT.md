# PerTick Conditional Branch Default Flags -- Ghidra Research Report

**Address(es):** `0x0055D360` (`Main_Tick`), `0x0055AFB0` (`LogicClassPerTickUpdateLiveVector`), `0x0054E4D0`, `0x00554D50`, `0x0053D310`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** default YR liveness and tick-spine position for the TS fog branch, scenario-delay/render-only branch, non-local animation pool branch, and the three named PerTick helper calls.  
**Non-Scope:** `DAT_00AC167C` light/glow list, `FUN_00551A30` display YSort, full scenario parser bit layout, full AnimClass semantics, and complete UI/tactical rendering internals.  
**Confidence:** High for branch order and default/conditional liveness; Medium for human semantic names of `0x0054E4D0` because class ownership remains unnamed.  
**Active in YR:** Conditional overall; individual branches below state `Yes / No / Conditional`.

## Working Notes Gate

- **Target question:** Which remaining conditional/legacy branches on `Main_Tick` / `LogicClass::PerTickUpdate` are active by default in YR, where do they sit, and what should Rust not treat as default behavior?
- **Non-goals:** Do not redo `DAT_00AC167C`, `FUN_00551A30`, AnimClass constructor mapping, or full fog/render internals.
- **Evidence needed to mark COMPLETE:** direct decompile plus assembly context for the branch/call position; INI/default source plus binary reader address for `FogOfWar`; current Rust surface scan for affected files.
- **Stop conditions:** stop after these scoped branch positions and default gates are resolved; defer unrelated helper internals and mode/session parser details.

## 1. Overview

The scoped branches are not missing pieces of the normal local YR tick path. `Main_Tick` always reaches the active PerTick ladder in normal running gameplay, but scenario render-only and replay paths can return before the late frame increment. Inside `LogicClassPerTickUpdateLiveVector`, the TS-style fog tick is conditional and default-off, the non-local animation pool is skipped for local modes `0` and `5`, and the three named helpers are unconditional PerTick calls with verified positions.

## 2. Branch / Helper Ledger

| Branch / helper | Position | Active in YR | Evidence | Default standard YR effect |
|---|---:|---|---|---|
| Scenario delay timer pre-branch (`Scenario+0x630`, timer at `+0x620/+0x628`) | `Main_Tick`, before scenario render-only branch | Conditional | Decompile `0x0055D360`; assembly `0x0055D7C2..0x0055D821` | Not a normal local gameplay tick effect unless scenario delay fields are set. |
| Scenario render-only return (`Scenario+0x62C != 0`) | `Main_Tick`, before normal input/logic block and before `g_CurrentFrameCounter++` | Conditional | Decompile `0x0055D360`; assembly `0x0055D821..0x0055D859` | Renders/services and returns without `LogicClassPerTickUpdateLiveVector` or frame increment. |
| TS-style fog branch (`Scenario.SpecialFlags & 0x1000`, `Rules+0x1648 != 0.0`) | PerTick after bridge-shroud `% 0x78`, before scenario lighting interpolation and ore drivers | Conditional; default off | Decompile `0x0055AFB0`; assembly `0x0055B2C4..0x0055B326`; `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` reads `FogOfWar` into `Rules+0x14B7`; `rulesmd.ini:3040` says `FogOfWar=no`; skirmish start assembly `0x006AD88F` forces `DAT_00A8B31F=0`. | Do not run as default YR shroud. It only calls `0x004ACBC0` when the flag and interval are enabled. |
| Non-local animation pool (`DAT_00A83E04/+0x10`) | PerTick after main live object vector, before `0x0053D310` | Conditional | Decompile `0x0055AFB0`; assembly `0x0055B61B..0x0055B649` checks `g_GameMode != 0 && g_GameMode != 5` | Skipped in local modes `0` and `5`; active for non-local/network-ish modes when count > 0. |
| `FUN_0054E4D0(0x00ABC5F8)` | PerTick after `BombClass__UpdateAll`, before team temp-vector AI | Yes | Decompile `0x0054E4D0`; call assembly `0x0055B4EB..0x0055B4F0` | Every tick checks its own 30-frame timer; may no-op until period expires. |
| `FUN_00554D50(ECX=6, DL=0)` | PerTick after RadSite/reverse loop and before `EMPulseClass__UpdateAll` | Yes | Decompile `0x00554D50`; call assembly `0x0055B5EA..0x0055B5F6` | Every tick drains light/cell dirty queue with a 6 ms preparation budget; no-op if queue empty. |
| `FUN_0053D310()` | PerTick after optional non-local pool, before alpha/crate/tactical/factory/house tail | Yes | Decompile `0x0053D310`; call assembly `0x0055B64B..0x0055B650` | Every tick loops `DAT_00AA0128` wave/splash entries backwards; no-op if count is zero. |

## 3. Core Logic Details

### Main_Tick Scenario Branches

`Main_Tick @ 0x0055D360` first configures timing budget, then for local modes `0` or `5` checks scenario timer fields. If `Scenario+0x630` is nonzero, it compares `GetRadarTimer() - *(Scenario+0x620)` against `*(Scenario+0x628)` unless the start field is `-1`; when the delay expires it clears `+0x630` and calls `0x00684180`. Active in YR: Conditional. Evidence: decompile `0x0055D360`, assembly `0x0055D7D4..0x0055D821`.

Immediately after that, `Main_Tick` reads `Scenario+0x62C`. If nonzero, it calls `Process_NetworkMessages`, `Network_ServiceLoop`, `Process_QueuedEvents`, `Tactical vtable+0x5C`, `RenderFrame_main`, and `FUN_0055E160`, then returns without reaching the normal input/logic block, `LogicClassPerTickUpdateLiveVector`, or the late frame increment. Active in YR: Conditional. Evidence: decompile `0x0055D360`, assembly `0x0055D821..0x0055D859`.

### TS Fog Branch

Inside `LogicClassPerTickUpdateLiveVector`, the fog branch starts after the bridge-shroud `% 0x78` recalculation. It tests `(*Scenario & 0x1000) != 0` via `TEST AH,0x10`, then checks `Rules+0x1648 != 0.0`. If both pass and the timer at `Scenario+0x1224/+0x122C` has expired, it reloads `+0x1224 = current frame`, `+0x1228 = local high word`, `+0x122C = ftol(Rules+0x1648 * const)`, then calls `0x004ACBC0`. Active in YR: Conditional, default off. Evidence: decompile `0x0055AFB0`; assembly `0x0055B2C4..0x0055B326`; `rulesmd.ini:[MultiplayerDialogSettings] FogOfWar=no`; reader `0x00671EA0`; skirmish start force `0x006AD88F`.

### Non-Local Animation Pool

After the main live LogicClass vector loop, PerTick checks `g_GameMode`. It skips the pool when mode is `0` or `5`; otherwise it loops forward over `DAT_00A83E04` for `DAT_00A83E10` entries and calls each entry's vtable `+0x5C`. Active in YR: Conditional. Evidence: decompile `0x0055AFB0`; assembly `0x0055B61B..0x0055B649`.

### Named Helpers

`0x0054E4D0` is timer-owned. It stores `last_frame = g_CurrentFrameCounter`, preserves a high/stack companion word, reloads period `0x1E`, then iterates entries at `param+0x10` for count `param+0x1C`. For each entry it writes object `+0x2FC = 1`; when entry mode is zero it uses `RateTimer__Current`, shifts by `0xC`, computes `((value >> 12) + 1) >> 1 & 7`, calls vtable `+0x1BC`, then `Pathfinding_update_continued`; it always calls vtable `+0x3C8` and `+0x1E8(1,0)`. Active in YR: Yes, but work is cadence-gated. Evidence: decompile `0x0054E4D0`; assembly `0x0054E4D0..0x0054E584`, call `0x0055B4EB..0x0055B4F0`.

`0x00554D50` is the light/cell dirty queue processor. Normal PerTick passes `ECX=6`, `DL=0`. Preparation walks queued records backward, checking elapsed time only on indices where `(index & 0x0F) == 0x0F`; commit is all-or-nothing after preparation completes. Active in YR: Yes; queue contents are conditional. Evidence: decompile `0x00554D50`; assembly `0x00554D50..0x00554EE6`, call `0x0055B5EA..0x0055B5F1`; prior report `LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md`.

`0x0053D310` is a narrow wave/splash loop: it snapshots `DAT_00AA0128`, starts at `count - 1`, and calls `Wave_splash_forces()` while index remains nonnegative. Active in YR: Yes; effective body count may be zero. Evidence: decompile `0x0053D310`; assembly `0x0053D310..0x0053D32F`, call `0x0055B64B`.

## 4. INI Keys

| Key | Source | Default | Binary read / consumer | Active in YR |
|---|---|---:|---|---|
| `[MultiplayerDialogSettings] FogOfWar` | `ini/rulesmd.ini:3040`, `ini/rules.ini:2520` | `no` | `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` reads into `Rules+0x14B7`; later session init stages `Scenario.SpecialFlags & 0x1000`; PerTick consumes the flag at `0x0055B2C4`. | Conditional; default off |
| `[General] ShroudRate` / fog interval source candidate | `ini/rulesmd.ini:762`, `ini/rules.ini:621` | `4` | PerTick consumes `Rules+0x1648`, but this slice did not re-prove the parser offset. | Conditional; parser mapping deferred |

## 5. Current Rust Implementation Status

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/sim/world/mod.rs::advance_tick` | Advances `binary_frame` at tick entry and runs Rust staged simulation phases. | Native conditional render-only branch and late frame commit are not represented as a first-class spine gate. |
| `src/sim/world/mod.rs::refresh_fog` / `src/app_sim_tick.rs` | Maintains standard Rust fog/shroud visibility every sim tick and pre-merges local fog after `advance_tick`. | This must not be justified by the default-off TS `SpecialFlags & 0x1000` PerTick fog branch. |
| `src/sim/world/mod.rs::world_effects` and app animation files | World effects/building animations tick in Rust phases/app code. | No proven conditional equivalent of the non-local native pool gated by `g_GameMode != 0 && != 5`. |
| `src/map/lighting.rs` / app init lighting | Static lighting grid setup; dynamic dirty queue not modeled. | Missing tick-integrated `0x00554D50`-style dirty cell/light queue drain. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Scenario `+0x630` timer branch | verified | `0x0055D7D4..0x0055D821` | exact scenario field names/parser owners |
| Scenario `+0x62C` render-only return | verified | `0x0055D821..0x0055D859` | runtime scenario cases that set the flag |
| TS fog PerTick branch | verified for default/liveness/order | `0x0055B2C4..0x0055B326`, `0x00671EA0`, `rulesmd.ini:3040`, `0x006AD88F` | parser offset for `Rules+0x1648` |
| Non-local animation pool branch | verified | `0x0055B61B..0x0055B649` | exact class inventory of `DAT_00A83E04` entries |
| `0x0054E4D0` | verified for order/timer mechanics | decompile `0x0054E4D0`; call `0x0055B4EB` | exact class/name for queued objects |
| `0x00554D50` | verified via existing dedicated report plus spot-check | decompile `0x00554D50`; call `0x0055B5EA`; light dirty report | none for tick placement/default liveness |
| `0x0053D310` | verified | decompile `0x0053D310`; call `0x0055B64B` | producer set for wave/splash entries |

## 7. Open Questions -- Final State

- `[RESOLVED] OQ-01 -- Is the scenario render-only branch before or after normal logic? -> Before normal input/logic and before PerTick; it returns before frame increment.` (evidence: `0x0055D821..0x0055D859`)
- `[RESOLVED] OQ-02 -- Is the TS fog branch default YR behavior? -> No; Active in YR: Conditional, gated by `Scenario.SpecialFlags & 0x1000` and interval nonzero; standard `FogOfWar=no` leaves it off.` (evidence: `0x0055B2C4..0x0055B326`, `0x00671EA0`, `ini/rulesmd.ini:3040`)
- `[RESOLVED] OQ-03 -- Is the non-local animation pool active in local modes? -> No for modes `0` and `5`; active only when `g_GameMode` is neither value and count > 0.` (evidence: `0x0055B61B..0x0055B649`)
- `[RESOLVED] OQ-04 -- Are the three named helpers on the active PerTick spine? -> Yes, all have direct calls inside `0x0055AFB0` after the default-off TS fog branch.` (evidence: `0x0055B4EB`, `0x0055B5EA`, `0x0055B64B`)
- `[DEFERRED] OQ-05 -- Which exact scenario/map/script states set `Scenario+0x62C`?` (category: requires-different-system-context; reason: this slice only needed branch order/default liveness; next-step-if-pursued: trace writers to `Scenario+0x62C`)
- `[DEFERRED] OQ-06 -- What exact class family owns `DAT_00A83E04`?` (category: requires-different-system-context; reason: branch gate/order resolved but class census is an AnimClass/pool target; next-step-if-pursued: xref constructors/destructors for `DAT_00A83E04`)
- `[DEFERRED] OQ-07 -- What is the parser source for `Rules+0x1648`?` (category: out-of-scope; reason: branch default is already blocked by `FogOfWar=no` and `SpecialFlags & 0x1000`; next-step-if-pursued: rules parser offset audit)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Scenario render-only branch returns before normal logic, PerTick, and late frame increment. Active in YR: Conditional. | `0x0055D821..0x0055D859` | Missing/unchecked | app-level tick orchestrator around `src/app_sim_tick.rs` and `src/sim/world/mod.rs::advance_tick` | Model a branch that can service/render without advancing native gameplay frame when the native-equivalent scenario gate is set. | `global_tick_scenario_render_only_skips_logic_and_frame_commit` | Do not treat every render pass as a gameplay tick. |
| TS fog PerTick branch is conditional and default-off in standard YR. Active in YR: Conditional. | `0x0055B2C4..0x0055B326`; `rulesmd.ini:3040`; `0x00671EA0`; `0x006AD88F` | Rust fog exists but is not this branch | `src/sim/world/mod.rs::refresh_fog`, `src/app_sim_tick.rs` fog merge | Keep standard shroud/vision separate from the TS fog decay tick; only enable TS branch under its verified flag/default path. | `ts_fog_branch_default_off_does_not_decay_explored_cells` | Do not use `SpecialFlags & 0x1000` code as justification for default YR fog/shroud behavior. |
| Non-local pool runs only when `g_GameMode != 0 && g_GameMode != 5`, after main live object vector and before wave/alpha/crate/tactical/factory/house tail. Active in YR: Conditional. | `0x0055B61B..0x0055B649` | Missing/unchecked | world/app animation effect scheduling | If modeled, gate this pool by session mode and preserve placement after object AI. | `nonlocal_anim_pool_skipped_for_local_modes_0_and_5` | Do not tick this pool unconditionally in local skirmish/single-player. |
| `0x00554D50` light/cell dirty queue drain is an unconditional PerTick call with args `6,false`, before EMP and object AI. Active in YR: Yes. | `0x0055B5EA..0x0055B5F1`, decompile `0x00554D50` | Missing dynamic dirty queue | `src/map/lighting.rs`, future render-light bridge | Place dynamic light dirty queue drain before EMP/object AI in native PerTick order. | `light_dirty_queue_drain_before_emp_and_object_ai` | Do not batch it after app effects if claiming native tick parity. |
| `0x0054E4D0` and `0x0053D310` are live unconditional PerTick callsites, but their internal work is timer/count-gated. Active in YR: Yes. | `0x0055B4EB`, `0x0055B64B`, decompiles | Missing/unchecked | future global tick spine ledger | Reserve explicit slots for 30-frame scripted/action helper and wave/splash forces even when they often no-op. | `pertick_helper_slots_preserve_noop_order_under_zero_counts` | Do not delete or reorder no-op-looking helper slots; later systems may observe their side effects when nonempty. |

## 9. Negative Facts / Do Not Do

- Active in YR: No by default. Do not enable the TS `SpecialFlags & 0x1000` fog tick as standard YR shroud/visibility decay.
- Active in YR: Conditional. Do not tick the non-local animation pool in local modes `0` and `5`.
- Active in YR: Conditional. Do not increment the native frame counter on the `Scenario+0x62C` render-only path.
- Active in YR: Yes for callsite, conditional for body work. Do not skip `0x0054E4D0` or `0x0053D310` just because their current timer/count can make them no-op.
- Active in YR: Yes. Do not move `0x00554D50` after EMP/object AI; native order is before `EMPulseClass__UpdateAll` and before the main object vector.

## 10. Stale Docs / Follow-up Docs

- Replace global tick contract wording `conditional TS fog, non-local anim pool, and scenario-delay branches require verified flag writers/default values` with: `TS fog default/liveness is resolved: PerTick branch is gated by Scenario.SpecialFlags bit 0x1000 plus Rules+0x1648, and standard YR MultiplayerDialogSettings has FogOfWar=no. Non-local pool liveness is resolved for the top-level gate: it runs only when g_GameMode is neither 0 nor 5. Scenario render-only branch order is resolved: Scenario+0x62C returns after service/render and before PerTick/frame increment; exact writers remain separate follow-up.`
- Replace `FUN_00554D50 unknown global` with: `PerTick light/cell dirty queue processor called with ECX=6, DL=0 after RadSite/reverse loop and before EMPulseClass::UpdateAll.`
- Replace `FUN_0053D310 unknown global` with: `Wave/splash forces loop over DAT_00AA0128 entries, placed after optional non-local pool and before alpha/crate/tactical/factory/house tail.`
- Keep `FUN_0054E4D0` as `timer-owned 30-frame scripted/action helper` unless a separate class-ownership report proves a better name.

## Sources

- Ghidra decompile: `0x0055D360`, `0x0055AFB0`, `0x0054E4D0`, `0x00554D50`, `0x0053D310`, `0x00671EA0`.
- Ghidra assembly context: `0x0055D7C2`, `0x0055D821`, `0x0055B2C4`, `0x0055B304`, `0x0055B326`, `0x0055B4EB`, `0x0055B5EA`, `0x0055B61B`, `0x0055B64B`, `0x006AD88F`.
- INI: `ini/rulesmd.ini`, `ini/rules.ini`.
- Prior docs checked: `docs/contracts/2026-05-28-global-tick-spine-order-implementation-contract.md`, `docs/research/PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`, `docs/research/LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`, `docs/research/LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`.

**Status:** COMPLETE for the scoped branch/default/order question.
