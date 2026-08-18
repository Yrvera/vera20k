# Factory / House / AI Tick Order vs Rust Production AI - Ghidra Report

**Date:** 2026-05-28  
**Target:** `FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI`  
**Address(es):** `LogicClass::PerTickUpdate @ 0x0055AFB0`, `FactoryClass::AI @ 0x004C9B20`, `HouseClass::Update @ 0x004F8440`  
**Active in YR:** Yes. `Main_Tick` reaches `LogicClass::PerTickUpdate`, and the factory/house arrays are standard active game-state arrays in normal Yuri's Revenge.  
**Status:** COMPLETE for factory-before-house ordering and current Rust comparison. Exact AI build-choice formulas and `HouseClass::MPlayer_Defeated` side effects remain separate topics.

## Target Question

Verify native ordering of `FactoryClass::AI` vs `HouseClass::AI` and adjacent tactical/object/global loops inside `LogicClass::PerTickUpdate`, then compare with current Rust ordering for production, repairs/docks, `ai::tick_ai`, defeat, and superweapon charge/refresh. Focus on production/house/AI/defeat consequences visible to players.

## Non-goals

- Do not re-derive factory build-speed formulas beyond citing existing reports.
- Do not reverse every `HouseClass::Update` AI chooser branch.
- Do not audit exact war-factory exit placement, blocked delivery, or queue restart formulas beyond using existing reports.
- Do not modify Rust, INI, or non-research docs.

## Evidence Needed To Mark COMPLETE

- Decompile evidence for `LogicClass::PerTickUpdate` tail ordering.
- Assembly/range evidence for tactical, factory-array, and house-array order.
- Decompile evidence that `FactoryClass::AI` is production-step/completion logic, not house AI.
- Decompile evidence for `HouseClass::Update` placement of superweapon ready checks, multiplayer defeat, AI chooser, and `AI_ManageProduction` / `AI_ResumeProduction`.
- Current Rust source evidence for `advance_tick` ordering around superweapons, production/repairs/docks, AI, and defeat.

## Stop Conditions

- Stop once native factory-vs-house order is proven by `0x0055AFB0` decompile plus assembly context.
- Stop once Rust source ordering is identified from `Simulation::advance_tick`; do not implement.
- Stop before exact AI build-selection formulas.

## Verified Binary Facts

### `LogicClass::PerTickUpdate` Tail Order

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` executes the relevant late-gameplay tail in this order:

1. `TiberiumClass__GrowthDriver_AllTypes()`
2. `TiberiumClass__SpreadDriver_AllTypes()`
3. `BombClass__UpdateAll()`
4. team array copy/AI loop
5. disk-laser reverse loop
6. `FUN_005ff390()`
7. `LaserDrawClass__UpdateAllAI()`
8. `LightningStorm__Process()`
9. reverse loop over another global vtable `+0x5C` array
10. `FUN_00554d50()`
11. `EMPulseClass__UpdateAll()`
12. main live `LogicClass` object vector loop
13. optional non-local/network vtable `+0x5C` loop
14. `FUN_0053d310()`
15. `AlphaShapeClass__PurgeDisabled()`
16. `MapClass__UpdateCrateRegenTimers()`
17. `Tactical` vtable `+0x5C`
18. global `FactoryClass` array vtable `+0x5C` loop
19. global `HouseClass` array vtable `+0x5C` loop
20. last-ref-object display housekeeping

Assembly context pins the critical handoff ordering:

```text
0055b65f: MOV ECX,dword ptr [0x00887324]
0055b667: CALL dword ptr [EAX + 0x5c]        ; Tactical vtable +0x5C
0055b66a: MOV EAX,[0x00a83e40]
0055b675: MOV ECX,dword ptr [0x00a83e34]
0055b680: CALL dword ptr [EDX + 0x5c]        ; FactoryClass array item AI
0055b683: MOV EAX,[0x00a83e40]
0055b688: INC ESI
0055b689: CMP ESI,EAX
0055b68b: JL 0x0055b675
0055b68d: MOV EAX,[0x00a80238]
0055b698: MOV EAX,[0x00a8022c]
0055b69d: MOV ECX,dword ptr [EAX + ESI*0x4]
0055b6a0: TEST ECX,ECX
0055b6a6: CALL dword ptr [EDX + 0x5c]        ; HouseClass array item AI
0055b6a9: MOV EAX,[0x00a80238]
0055b6ae: INC ESI
0055b6af: CMP ESI,EAX
0055b6b1: JL 0x0055b698
```

The order is therefore **Tactical AI -> all factories -> all houses**. Factory AI is not nested inside each house update.

### `FactoryClass::AI` Role

`FactoryClass__AI @ 0x004C9B20` is the production stepper:

- early-outs when suspended, empty, complete, or timer-not-expired;
- increments `Production_Value` by `Production_Step`;
- restarts the CDTimer from `g_CurrentFrameCounter`;
- charges the owner for the step or rolls progress back on insufficient credits;
- when `Production_Value == 0x36`, sets `IsSuspended = true`, clears timer duration/time-left, spends remaining balance, and leaves the completed object for later delivery paths.

This confirms the `0x0055B675..0x0055B68B` loop advances factory progress before any `HouseClass` instance gets its per-frame update.

### `HouseClass::Update` Internal Order Relevant Here

`HouseClass__Update @ 0x004F8440` contains several player-visible systems after it is reached by the global house loop:

- Early per-house timer work can call `HouseClass__CheckSuperweaponReady` and `HouseClass__CheckLowPower`.
- Superweapon instances are iterated at `this+0x258/count +0x264`; each calls `SuperClass__AI_Ready @ 0x006CBCA0`.

```text
004f8e28: MOV EAX,dword ptr [ESI + 0x264]
004f8e34: MOV EAX,dword ptr [ESI + 0x258]
004f8e3a: MOV EDI,dword ptr [EAX + EBX*0x4]
004f8e47: PUSH ECX
004f8e48: MOV ECX,EDI
004f8e4a: CALL 0x006cbca0                  ; SuperClass::AI_Ready
004f8e7b: MOV EAX,dword ptr [ESI + 0x264]
004f8e81: INC EBX
004f8e82: CMP EBX,EAX
004f8e84: JL 0x004f8e34
```

- Multiplayer defeat checks follow the superweapon loop.

```text
004f8e86: CMP dword ptr [0x00a8b238],EBP
...
004f8f79: MOV ECX,ESI
004f8f7b: CALL 0x004fc6d0                  ; scatter all units
004f8f80: MOV ECX,ESI
004f8f82: CALL 0x004fc0b0                  ; multiplayer defeated
```

- AI building/unit/aircraft/infantry choice logic runs later, gated by frame modulo and AI/player flags.
- If house field `+0x1FC` is set, the tail calls production-management helpers:

```text
004f92f4: MOV ECX,ESI
004f92f6: CALL 0x0050af10                  ; HouseClass::AI_ManageProduction
004f92fb: MOV ECX,ESI
004f92fd: CALL 0x0050b1d0                  ; HouseClass::AI_ResumeProduction
```

The important order is: **global factories finish their AI first; then each house performs superweapon ready/low-power checks, defeat handling, AI choosing, and production-management/resume work inside `HouseClass::Update`.**

## Current Rust Evidence

Current `Simulation::advance_tick` is a staged Rust pipeline:

- Commands are applied at tick start around `src/sim/world/mod.rs:1224`.
- Power is updated around `src/sim/world/mod.rs:1397`.
- `superweapon::tick_superweapons` runs early around `src/sim/world/mod.rs:1408`; it both advances charge/suspend state and processes active Lightning Storm.
- Combat and particle systems run before production.
- `production::tick_production_with_overlay_registry` runs around `src/sim/world/mod.rs:1670`, then `tick_repairs`, `tick_building_docks`, and `tick_aircraft_docks` around `1678..1680`.
- Ore growth/spread runs after production/docks in the same phase.
- `ai::tick_ai` runs around `src/sim/world/mod.rs:1764`; returned commands are applied immediately around `1775`.
- `check_defeat` runs after AI around `src/sim/world/mod.rs:1794`.

Rust production currently combines several behaviors in `src/sim/production/production_queue.rs`:

- `tick_production_with_overlay_registry` starts with resource economy work, then scans `queues_by_owner`.
- It advances the front item via `advance_queue_item`, marks it `Done`, immediately handles ready buildings or unit/aircraft spawn attempts, and then pops completed fronts on successful delivery.
- Recent code now preserves blocked vehicle pending state better than older reports, but the active production model remains a Rust queue pass rather than native `FactoryClass::AI` objects followed by `HouseClass::Update`.

Rust AI in `src/sim/ai.rs` is not native `HouseClass::Update` AI:

- It is a simple deterministic command generator.
- It applies AI commands immediately in the same `advance_tick` after production.
- It is not `HouseClass::AI_Choose_*`, `AI_ManageProduction`, or `AI_ResumeProduction` running inside the per-house vtable loop.

## Parity Assessment

### Matches / Lower-Risk Shape

- Rust does run production before its high-level AI pass, which broadly matches the native fact that factories tick before houses.
- Rust defeat runs after production, also broadly compatible with the native house-loop fact that defeat is downstream of factory progress.

### DRIFT / Unchecked Gaps

| Area | Native ordering | Current Rust ordering | Player-visible risk |
|---|---|---|---|
| Superweapon charge/ready | Per-house `SuperClass::AI_Ready` runs inside `HouseClass::Update`, after global factory loop and after tactical update. Active `LightningStorm__Process` is a separate global call before the main object loop. | `tick_superweapons` runs early after power, before combat and before production; it also processes Lightning Storm. | One-tick differences in ready EVA/sidebar state, low-power suspend/resume, and storm damage/effects relative to production/combat/death. |
| Defeat vs AI decisions | Multiplayer defeat checks occur inside `HouseClass::Update` before later AI choose/production-management tail for that house. | `ai::tick_ai` runs before `check_defeat`. | A house that should be defeated this frame can still generate or apply AI commands before Rust marks defeat. |
| House production management | `AI_ManageProduction` and `AI_ResumeProduction` run in the house tail after factory progress and after native AI chooser gates. | No equivalent native house production-management tail; simple `ai::tick_ai` queues commands after production and applies immediately. | Queue resume, new build selection, and superweapon/building grant side effects can happen in the wrong tick or under the wrong house state. |
| Factory loop ownership | Native iterates a global `FactoryClass` array before the global `HouseClass` array, not per-owner queue categories. | Rust iterates collected `(owner, ProductionCategory)` queue pairs from maps. | Multi-house/multi-factory completion order can differ when several factories complete on the same frame. |
| Adjacent resource order | Tiberium growth/spread is before teams/object vector/tactical/factory/house in `0x0055AFB0`. | Ore growth/spread runs after Rust production/repairs/docks. | Resource density/harvester/economy timing can drift when production, ore growth, and combat-side ore reduction interact. |

## Implementation Handoff

1. Split superweapon timing into native roles instead of one early Rust pass: active `LightningStorm__Process` belongs in the global pre-object tail of `0x0055AFB0`, while per-house `SuperClass::AI_Ready` charging/ready presentation belongs inside the house update phase after factories.
2. Preserve a native-order phase contract for: tactical update -> all factories -> all houses. Within each house, defeat handling should occur before native AI chooser / production-management tail effects.
3. Treat `AI_ManageProduction` / `AI_ResumeProduction` as a `HouseClass::Update` parity target, not as equivalent to the current simple `ai::tick_ai` command generator. Factory completion should set/clear the house/factory dirty state that the later house tail consumes.
4. Add multi-owner acceptance tests if production ordering is tightened: two owners' factories complete on the same frame; one completion grants/revokes a powered superweapon; one house loses its last required object before its house update; AI must not produce commands after native defeat ordering would mark it defeated.
5. Keep Rust's recent blocked-vehicle pending behavior, but evaluate it under the native factory-before-house tail: completed factory state is visible to the subsequent house update in the same `0x0055AFB0` pass.

## Negative Facts / Do Not Do

- Do not say `HouseClass::Update` owns the global `FactoryClass::AI` production step. `0x0055B66A..0x0055B68B` iterates the global factory array before the house array.
- Do not place all superweapon behavior in one early tick phase. Native `LightningStorm__Process` and per-house `SuperClass::AI_Ready` occupy different positions.
- Do not run ordinary AI command generation before defeat if the target is native `HouseClass::Update` ordering. Native multiplayer defeat checks are inside the house update before later AI chooser / production-management tail work.
- Do not assume owner/category map iteration is equivalent to the native global `FactoryClass` array order.
- Do not use stale wording that puts Lightning Storm/EMP after houses in `0x0055AFB0`; the decompile shows those calls before the main live object vector and before factories/houses.

## Remaining Uncertainty

- Exact native construction/insertion order of the global `FactoryClass` array was not re-investigated here. The order matters for simultaneous multi-factory completions.
- Exact side effects of `HouseClass__MPlayer_Defeated @ 0x004FC0B0` and `HouseClass__ScatterAllUnits @ 0x004FC6D0` were not expanded beyond call placement.
- Exact AI chooser formulas and production-selection priorities are out of scope.
- Exact interaction between `HouseClass::CheckSuperweaponReady`, `SuperClass::AI_Ready`, sidebar flash/EVA, and current-player gating needs a separate UI/superweapon trace if the implementation changes.

## Stale Doc Wording

### `docs/research/FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md`

The `LogicClass::AI - Global Tick Order` section currently implies a simplified tail and says `// ... superweapons, EMP, lightning storm ...` after the house loop. Replace with:

> In `LogicClass::PerTickUpdate @ 0x0055AFB0`, active `LightningStorm__Process` and `EMPulseClass__UpdateAll` run before the main live object vector. The late tail then runs tactical update, the global `FactoryClass` array loop, and finally the global `HouseClass` array loop. Per-house `SuperClass::AI_Ready` charging/ready checks occur inside `HouseClass::Update`, not as a separate tail after all houses.

### `docs/research/HOUSECLASS_GHIDRA_REPORT.md`

The older "Execution order each frame" line saying `Production tick: Iterate all factories -> FactoryClass::Update per queue` inside `HouseClass::Update` is misleading. Replace with:

> `FactoryClass::AI` is driven by `LogicClass::PerTickUpdate` through the global factory array before the global house array. `HouseClass::Update` later performs per-house superweapon checks, defeat handling, AI choose logic, and `AI_ManageProduction` / `AI_ResumeProduction`; it does not own the global factory production-step loop.
