# Timing / Scheduler / Global Tick Spine - System Model Synthesis

**Date:** 2026-05-28  
**Scope:** native frame-counter visibility, `Main_Tick`, `LogicClass::PerTickUpdate`, the live object scheduler, and the global PerTick subsystem ladder.  
**Non-scope:** exact retail wall-clock frame pace by speed/mode, full pause/menu/replay branch matrix, exact `Scenario+0x62C` writer taxonomy, save/load active-list reconstruction, and the mechanism of every unknown `FUN_*` helper.  
**Output type:** model-synthesis with an implementation-blocker queue.  
**Overall status:** implementation-safe for the active standard tick order, pre-increment frame contract, main live-object scheduler semantics, and known PerTick ladder positions; investigation-blocked for pacing constants, increment-gate globals, and unknown helper bodies.

## Evidence Ladder Used

| Rank | Meaning in this synthesis |
|---|---|
| BINARY_HIGH | Direct Ghidra report with addresses, active caller, and active YR status checked |
| RESEARCH_HIGH | Recent focused report with Rust handoff and exact function/range citations |
| DOC_SYNTHESIS | Older overview prose; useful only when not contradicted |
| INFERENCE | Plausible connection across systems, unsafe without owner-specific proof |

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| Standard `Main_Tick` runs input, `LogicClass::AI`, optional house AI, map logic, render, `PerTickUpdate`, service/network, then late frame increment. | `MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `g_CurrentFrameCounter` is visible as the pre-increment value during gameplay and PerTick work. | `FRAME_COUNTER_PREINCREMENT_VS_RUST_BINARY_FRAME_GHIDRA_REPORT.md`; `MAIN_TICK...` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Rust currently computes `binary_frame` at the start of `Simulation::advance_tick`. | `MAIN_TICK...` Rust evidence | confirmed | high | n/a | IMPLEMENTATION_SAFE as mismatch fact |
| `LogicClass::PerTickUpdate` owns a live forward object vector and reloads count after each object AI call. | `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`; `LOGICCLASS_LIVE_VECTOR_VS_RUST_ENTITY_PASSES_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Registration tail-appends by object-local membership and removal compacts by shifting left. | `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| PerTick top-level order puts ore growth/spread before bombs/teams/object vector, and tactical/factories/houses after object work. | `PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`; `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE for order placement |
| TS fog PerTick branch is conditional and default-off in standard YR. | `PERTICK_CONDITIONAL_BRANCH_DEFAULT_FLAGS_GHIDRA_REPORT.md` | confirmed | high | conditional | IMPLEMENTATION_SAFE as default-off gate |
| Non-local animation pool is skipped in local modes `0` and `5`. | `PERTICK_CONDITIONAL_BRANCH_DEFAULT_FLAGS_GHIDRA_REPORT.md` | confirmed | high | conditional | IMPLEMENTATION_SAFE as mode gate |
| `Scenario+0x62C` render-only path returns before the live-object vector and before frame increment. | `PERTICK_CONDITIONAL_BRANCH_DEFAULT_FLAGS_GHIDRA_REPORT.md` | confirmed for order | medium | conditional | IMPLEMENTATION_SAFE for branch effect; writer taxonomy unknown |
| Helper slots `0x00554D50`, `0x0053D310`, and `0x0054E4D0` have resolved top-level identities/positions. | `PERTICK_CONDITIONAL_BRANCH_DEFAULT_FLAGS_GHIDRA_REPORT.md` | partially confirmed | medium | yes/body may no-op | IMPLEMENTATION_SAFE for placement only |
| All PerTick arrays share the same mutation semantics as the main object vector. | `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS_GHIDRA_REPORT.md`; full ladder report | contradicted | high | yes | DOC_PATCH_READY |
| Default retail wall-clock pace by speed/mode is fully proven. | timing reports | unknown | low | yes | NEEDS_REINVESTIGATE |
| Four late-increment gate globals + pause/menu/replay non-advance matrix mapped. Four globals are session-end flags (victory/defeat/quit/disconnect; `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md`). Non-advance states: `g_GameActive==0` full-freeze, `g_GameRunning==0` network-only loop, `Scenario+0x62C` render/net-only return, `g_GameState!=0` skips gameplay block but still advances frame counter + PerTickUpdate. | `FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md` | confirmed | high | conditional | IMPLEMENTATION_SAFE |

## Current Model

The native tick is not a single Rust-style simulation phase. The standard active path enters `Main_Tick`, processes input and early gameplay, renders, then calls `LogicClass::PerTickUpdate`. Only after late service/network work does the native code conditionally increment `g_CurrentFrameCounter`.

That placement is load-bearing. Any gamemd-mapped timer, modulo gate, animation delay, refinery unload accumulator, invulnerability timer, or production/tail update that reads `g_CurrentFrameCounter` during the tick sees the old frame value. A timer started and read later in the same tick has elapsed `0`.

`LogicClass::PerTickUpdate` has two distinct responsibilities:

1. It runs a fixed global subsystem ladder: scenario/timer work, ore growth/spread, bombs, teams, disk lasers, laser/lightning/radiation/light/EMP, the main live object vector, conditional anim pool, wave/alpha/crate/tactical, factories, houses, last-ref-object cleanup.
2. Its main object scheduler walks a live appendable vector, not `EntityStore` storage. The scheduler reloads count after every object `vtable+0x5C` call, so tail-appended objects can run in the same pass. Removal compacts the vector and the loop does not repair the current index, so a current-object self-removal can skip the immediate shifted successor.

Rust currently has deterministic `EntityStore` storage and many sorted snapshot phases. That is valuable storage infrastructure, but it is not the native scheduler model.

The conditional PerTick branches now have a narrower model. The TS fog branch is not standard YR shroud behavior: it requires `Scenario.SpecialFlags & 0x1000` and a nonzero `Rules+0x1648`, while standard `rulesmd.ini` has `FogOfWar=no`. The non-local animation pool is gated out for local modes `0` and `5`. The `Scenario+0x62C` path is a render/service-only path that returns before the live-object vector and before the late frame increment; exact writers for that flag remain a follow-up.

## Implementation-Safe Facts

- Introduce a native-frame view where gamemd-mapped tick work reads the pre-increment frame and commits the next frame only at the late native-equivalent boundary.
- Keep Rust app pacing and fixed-step scheduling separate from the native frame counter contract.
- Model the main object scheduler as a live active list with object-local membership, insertion order, count reload after each object, and compacting removal.
- Place PerTick-equivalent systems according to the verified ladder unless a per-system proof shows byte/pixel/result equivalence.
- Treat `EntityStore` as deterministic storage, not as the active object-AI order.
- Preserve ore growth before ore spread; current Rust preserves local order but not global placement.
- Place factories before houses in the PerTick tail; do not run HouseClass-equivalent work before the global factory loop.
- Keep the TS fog branch disabled for standard YR defaults unless both native gates are proven true.
- Gate any non-local animation pool by native game mode; local modes `0` and `5` skip it.
- Treat `0x00554D50` as the light/cell dirty queue slot before EMP, `0x0053D310` as the wave/splash force loop slot, and `0x0054E4D0` as the 30-frame timer-owned scripted/action helper slot. These identities are placement-safe, not full mechanism contracts.

## Doc-Patch-Ready Facts

- Replace any prose saying one Rust fixed sim tick equals one native RA2 frame. Current Rust uses a 45 Hz fixed step; native frame-counter visibility is a late-commit contract.
- Replace broad "LogicClass::AI iterates entities" wording with `LogicClass::PerTickUpdate` live-object vector semantics.
- Replace "production before AI" as a sufficient parity statement with the stricter tactical -> factories -> houses tail order.
- Mark old docs that collapse LightningStorm/EMP into house/superweapon tail work as stale for global order.

## Stale Or Superseded Claims

- Any doc or comment equating `SIM_TICK_HZ` with native `g_CurrentFrameCounter` is stale.
- Any claim that sorted `EntityStore` iteration proves active ObjectClass order is superseded by live-vector reports.
- Any claim that all PerTick arrays mutate like the main object vector is unsafe; teams, disk lasers, anim pools, factories, and houses have their own loop shapes.

## Cross-Doc Conflicts

No broad conflict remains for standard active tick order, pre-increment frame visibility, or main live object scheduler mechanics. The main unresolved issue is not disagreement but scope: several helper bodies and conditional branches are known by position but not yet mechanism-complete.

## Needs Re-Investigation

- `/re-investigate retail frame counter wall-clock pacing by game speed and mode`
  - Needed before choosing exact pacing constants or claiming wall-clock parity.
- `[RESOLVED 2026-05-28]` Main_Tick late-increment gate globals + pause/menu/replay matrix — decoded in `FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md` (pause/scenario/replay non-advance states) and `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` (four session-end flags). Only the retail wall-clock pace probe remains open (needs live attach).
- `/re-investigate PerTickUpdate remaining unknown helper <address>`
  - Needed for unnamed or unresolved helper bodies before implementing their mechanics. `0x00554D50`, `0x0053D310`, and `0x0054E4D0` have placement-safe identities but still need body-specific contracts before detailed mechanics.
- `/re-investigate Scenario+0x62C writers and render-only branch triggers`
  - Needed before exposing the render/service-only path outside a guarded ledger.
- `/re-investigate DAT_00A83E04 non-local animation pool inventory`
  - Needed before implementing the class contents and mutation rules of the non-local pool. The top-level local-mode gate is already resolved.
- `/re-investigate LogicClass active list save load replay reconstruction`
  - Needed for persistence/replay parity.

## Do-Not-Implement Notes

- Do not compensate for early `binary_frame` by subtracting one at individual timer sites.
- Do not implement active object order from sorted IDs.
- Do not use pass-entry snapshots for projectile/anim/unit AI paths that are meant to be live-vector objects.
- Do not put tactical/UI camera work inside `sim/`; preserve ordering through app-level orchestration or a split boundary.
- Do not enable TS fog behavior for standard YR merely because the ladder contains a conditional TS-fog branch.
- Do not run the non-local animation pool in local modes `0` or `5`.
- Do not increment `g_CurrentFrameCounter` or run the live-object vector on the `Scenario+0x62C` render-only path.

## Source Ledger

- `docs/research/MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md`
- `docs/research/FRAME_COUNTER_PREINCREMENT_VS_RUST_BINARY_FRAME_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md`
- `docs/research/PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`
- `docs/research/PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_LIVE_VECTOR_VS_RUST_ENTITY_PASSES_GHIDRA_REPORT.md`
- `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
- `docs/research/PERTICK_CONDITIONAL_BRANCH_DEFAULT_FLAGS_GHIDRA_REPORT.md`
- `src/sim/world/mod.rs`
- `src/sim/entity_store.rs`
- `src/app_sim_tick.rs`
- `src/app_types.rs`
- `src/util/fixed_math.rs`
