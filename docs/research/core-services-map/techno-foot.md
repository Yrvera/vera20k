# Core Service Profile — TechnoClass / FootClass (object-AI dispatch + locomotion)

**Slug:** `techno-foot`
**Layer:** object (per-object update layer; the AI/update stage LogicClass drives)
**Tick phase:** the per-object **AI/update stage** of the tick. In gamemd this is one combined stage inside `Main_Tick`; in the Rust `advance_tick` phase table it maps to the **AI phase** (between scatter+production+repairs+docks+ore and defeat detection). Within the object body the verified order is: pre-mission common work → `+0xC4` increment → `Mission_Dispatch` → post-mission common work → locomotor `Process` (in `FootClass::AI`, after dispatch).
**Primary doc:** `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` (STUDY+DESIGN, 2026-06-02; §0 is the live-binary re-verification of record and authoritative over the body).
**Companions:** `TECHNOCLASS_AI_MIGRATION_BOUNDARY_GHIDRA_REPORT.md`, `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md`, `READYTOCOMMENCE_*` reports, `BUILDINGCLASS_UPDATE_AI_TICK`.

---

## Purpose

The per-object update layer. For every live object, each tick this service runs the verified AI spine
`<Leaf>::AI → FootClass::AI (0x004DA530) → TechnoClass::AI_Update (0x006F9E50) → MissionClass::Mission_Dispatch (0x005B3060) → locomotor ILocomotion::Process (vtable+0x40)`.
It owns the common-Techno per-tick body (cloak, EMP, health-smoothing, target validation, voice, damage-particle RNG, accumulators), drives the mission FSM through `Mission_Dispatch`, and runs the locomotor *after* mission dispatch in the same per-object pass. `AbstractClass → ObjectClass → MissionClass → RadioClass → TechnoClass → FootClass → {Unit/Infantry/Aircraft}` is treated as a *behavior contract*, not a Rust class tree — the Rust target is `ObjectSubstrate` + `EntityCategory` + `CapabilityFlags` + per-component `Option<T>`, `match category` dispatch, no vtable/dyn/COM.

The governing rule: **Rust-native structure, gamemd-native semantics.**

---

## Owns

- **Common-Techno per-object AI body** (`TechnoClass::AI_Update 0x006F9E50`): pre-mission steps 1–20, `+0xC4` increment (step 21), `Mission_Dispatch` (step 22), post-mission steps 23–42; three early-return death points (self-heal step 12, IsAlive step 27, building-EMP-restore step 42).
- **Per-object AI tick counter** `+0xC4` — incremented immediately before `Mission_Dispatch` (call site `0x006FA655`); distinct from the global frame counter; consumers are mission-side.
- **Smoothed display-health** `+0x70` — lerps +1 toward real Health on `frame&4`, snaps down on damage (render-side; must NOT be hashed).
- **Cloak state** `+0x220/+0x224`, CloakState reveal/conceal via `vtable+0x420` (DoUncloak `0x6F4EB0`).
- **EMP-stun countdown** `+0x298/+0x29c`, EMPLockRemaining `+0x504`.
- **Passive/opportunity-scan timer** `+0x180/+0x188` (45-frame), gated missions {2,10,5} + CanPassiveAcquire + OpportunityFire `+0x6AF`.
- **Units-only periodic accumulator** `+0xf8 += +0x110` on the `+0x100/+0x108` timer (miner-unload accumulator); buildings (RTTI 6) skip it.
- **Voice/Voc queue** `+0x4f0`; damage-fire particle slot `+0x308` (the ONLY RNG consumer in `AI_Update`, `Random__RandomRanged` ×2).
- **Manager sub-objects** (CONDITIONAL): SpawnManager `+0x2d0`, SlaveManager `+0x2d8`, CaptureManager `+0x2bc` — each ticked via its own `vtable+0x5C`.
- **FootClass navigation state**: NavCom (live single-target), NavQueue `+0x588/+0x598` (cap 10, no runtime push producer — DORMANT, save-load only), formation speed `+0x578`, idle-scatter every `0x3f` frames.
- **FootClass sub-AI dispatch** `+0x694` — ticks a live sub-object via `(*(*(this+0x694)+0x69C))[+0x5C]()` every frame; identity = `WrapAttachClass*` (chrono-warp attach), CONDITIONAL(chrono-warp), NOT a parasite/Terror-Drone. Dispatch is ACTIVE_YR; writer site UNVERIFIABLE.
- Rust-side homes (in-flight): `sim/world/substrate.rs` `ObjectSubstrate`; the planned `sim/ai/` shell (`object_ai_stage` + per-leaf step fns); `InfantryRuntime{fear_level,is_prone}` (`game_entity.rs:48`) + `tick_fear_for_entities` (`infantry.rs:130`).

State that lives in *sibling* services (NOT owned here): the mission selector/timer/control config (mission-radio + rules-class), radio contacts (mission-radio), presence/active-vector/pending-delete (abstract-object), occupancy grid (cell-map).

---

## Key functions & globals (addresses)

| Symbol | Address | Role |
|---|---|---|
| `TechnoClass::AI_Update` | `0x006F9E50` | Common per-object body; sole caller of `Mission_Dispatch` (call site `0x006FA655`). |
| `FootClass::AI` | `0x004DA530` | Calls `AI_Update` (`0x004DA539`, first call), then locomotor `Process` (vtable+0x40 @ ~`0x004DA877`); idle-scatter every 0x3f; sub-AI `+0x694` dispatch at tail. |
| `UnitClass::AI` | `0x007360C0` | Leaf shell: pre-Foot deploy/tube/warp → FootClass::AI → TurretAI → Fire→Facing→Harvest→Anim/Ammo(vtable+0x424)→Spawn→auto-hunt. |
| `InfantryClass::AI` | `0x0051BAB0` | Tube early-return → death-force → FootClass::AI → garrison → Mission_Capture → Fear_Decay → Fire → DoType_Sequencer. |
| `AircraftClass::AI` | `0x00414BB0` | Thin shell: clears one-shot mission byte; state machines run under Mission_Dispatch. |
| `FootClass::Mission_Eaten` | `0x004D4CB0` | Case 9 handler (mind-control follower; consumes `Random(0,2)`). CONDITIONAL. |
| `FootClass::Mission_Rescue` / `AircraftClass::Mission_Rescue` | `0x004DDF90` / `0x00415960` | Case 21 (slot +0x258). CONDITIONAL, AI-only. |
| `Fear_Decay_Handler` (Infantry) | `0x005200B0` | Thresholds 49/50/199. ACTIVE_YR. |
| `DoType_Sequencer` (Infantry) | `0x00520AE0` | Sequencer self-Destroy. |
| `SpawnManagerClass::AI` | `0x006B7230` | Spawner sub-object AI (`+0x2d0`). CONDITIONAL. |
| `DriveLocomotionClass::Process` | `0x004b0500` | Locomotor `Process` identity at ILocomotion vtable+0x40 (verified `read_memory 0x007e7ef0`). |
| `DriveLocomotionClass::Set_Destination` | `0x004afd40` | vtable+0x44 (label drift: NOT Head_To_Coord). |
| FootClass `+0x694` dispatch | tail of `0x004DA530` | WrapAttach sub-AI; `decompile 0x0062a4a0/0x004deae4/0x004d9960`. |
| LogicClass AI consumer | `0x0055AFB0` (in `Main_Tick 0x0055D360`) | Re-reads live active-object count each iteration (same-pass). Doc-sourced. |
| Per-object dispatch slot `vtable+0x5C` | `read_memory 0x7E8C94`[+0x5C]=`0x004DA530` | LogicClass calls leaf AI here. |

(Mission-FSM verbs, mission-name/control tables, `Mission_Dispatch` switch map, ReadyToCommence overrides, radio contacts — see the **mission-radio**, **rules-class**, and **abstract-object** profiles; addresses are in primary doc §3.)

---

## Tick / render position

In gamemd, one combined AI/update stage (`0x0055AFB0` inside `Main_Tick 0x0055D360`) walks the active-object vector, re-reading the live count each iteration (mid-tick spawn acts the same tick), and calls each leaf `vtable+0x5C`. Within the per-object body the order is load-bearing and PROVEN:

```
LogicClass active-vector loop  (vtable +0x5C per object)
  → <Leaf>::AI  (Unit 0x007360C0 / Infantry 0x0051BAB0 / Aircraft 0x00414BB0 / Building::Update phase 11)
    → FootClass::AI 0x004DA530
      → TechnoClass::AI_Update 0x006F9E50
          steps 1-20 (pre-mission)  →  +0xC4++  →  Mission_Dispatch 0x005B3060 (case→vtable handler)  →  steps 23-42 (post-mission)
      → ILocomotion::Process  vtable+0x40  (~0x004DA877, AFTER dispatch)   [PROVEN]
      → FootClass +0x694 sub-AI dispatch (tail, chrono-warp only)
    → Unit post-Foot: Fire_At_Target → Facing_Update → HarvestBrain → Anim/Ammo(vtable+0x424) → SpawnManager   [fire-before-facing PROVEN]
```

Global brackets (commands, vision, power, production/repair/ore, defeat, building-anim+cleanup/flush_pending_delete, state hash) stay OUTSIDE the object body (Invariant G1) — owned by other services. In the Rust port this service is the **AI phase** of `advance_tick`; currently the work is scattered across separate global sweeps (movement Phase 1, combat, turret rotation, aircraft missions) — the design folds them under a per-object `object_ai_stage` (UnitClass first, Buildings last). Not a render service (only `+0x70` smoothed health touches the render path, render-side, never hashed).

---

## Depends-on (outgoing edges)

| Target slug | Via (symbol/field) | Evidence |
|---|---|---|
| **logicclass** | called from the LogicClass AI consumer `0x0055AFB0` via per-object `vtable+0x5C` (=`FootClass::AI 0x004DA530`); membership flag `+0x98`, add `FUN_0055BAA0` / remove `FUN_0055BAE0` | This service is the body LogicClass dispatches each tick (`read_memory 0x7E8C94`[+0x5C]; `disassemble 0x0055AFB0` CALL [EDX+0x5c] @ `0x0055b610`). Same-pass re-read is a LogicClass-stage property. |
| **mission-radio** | `MissionClass::Mission_Dispatch 0x005B3060` (call site `0x006FA655`); verbs Queue/Assign/Override/Restore/Commence; radio `transmit()/receive_radio() 0x0065A820`; contacts `+0xE4/+0xE8` | `AI_Update` calls `Mission_Dispatch` per object every tick; FSM gates (IsActive +0x90, Health +0x6C, frame-anchored timer +0xC8/+0xD0) live there. Foot docking calls synchronous radio. (`decompile 0x006F9E50`, `0x005B3060`) |
| **abstract-object** | `substrate.uninit/conceal/reveal/unlimbo/flush_pending_delete`; IsActive `+0x90`, IsAlive, Health `+0x6C`; per-object self-removal exits enqueue to `pending_delete` | Every mid-pass death (deploy timed-death, sinking, crash, sequencer death, zero-health) routes through the ObjectClass lifecycle/presence owner; the AI body never frees synchronously. (Invariant T5; `decompile 0x006F9E50/0x007360C0/0x0051BAB0/0x00414BB0`) |
| **random-scenario** | `Random__RandomRanged` ×2 in `AI_Update` step 40 (damage-fire particle); `Random(0,2)` in `Mission_Eaten 0x004D4CB0`; `RandomRanged(0,2)` in handler-rate compute; `RandomRanged(0,99)` in Rescue assigner | The only `AI_Update` RNG consumer is the damage-particle pick (gated ConditionYellow + DamageParticleSystems + `+0x308==0`); lockstep-relevant — must consume at the same per-object position. Per-callsite ECX selects RNG instance. (`decompile 0x006F9E50`) |
| **rules-class** | `MissionControl` rate (`Rate*900.0`); `Rules+0x30/0x38` power heal/drain, `+0x314/0x318` Thief, `+0x1700/0x1708` ConditionYellow, `+0x558..0x564` particle coords; ConstructionYardTypes `Rules+0x8b0` | Frame-modulo gates and dispatch-rate computation inside `AI_Update` read RulesClass globals. (`decompile 0x006F9E50`; `Read_INI 0x005B3760`) |
| **cell-map** | cloak reveal uses cell `IsVisibleToHouse`; passive-scan/garrison/stuck-rescue read occupancy; cell relocation (no `move_cell` substrate API yet) | CloakState reveal/conceal keys on cell visibility (`decompile 0x006F9E50`); leaf AI queries occupancy for garrison-enter and stuck-harvester rescue. (Invariant N1; primary doc §7.4) |
| **damage-helpers** | `TechnoClass::ReceiveDamage 0x00701900` is the trigger surface for the Rescue(21) AI assigner (`FUN_00708080 → Queue_Mission(0x15,0)`); health gate `+0x6C` | Retaliation/Rescue assignment originates in the ReceiveDamage family; the AI body reads Health for its dispatch gate and self-heal death. (§0.4; `decompile 0x00701900/0x00708080`) |
| **target-scoring** | passive/opportunity acquisition post-dispatch (missions {2,10,5}, OpportunityFire `+0x6AF`, scan `+0x180/+0x188`, suppress `vtable+0x4c4`); target-validation/clear suite via `+0x3c8` | Grizzly/War-Miner opportunity fire and per-tick target sanity run inside `AI_Update`. (Invariant T7; `decompile 0x006FA699..0x006FA6C1`) |
| **pathfinding-helpers** | locomotor `ILocomotion::Process` (vtable+0x40, `0x004b0500`) consumes NavCom/path after mission dispatch | FootClass::AI drives the locomotor, which reads the active path (split from NavCom owner-state). (Invariant N1/T2; `disassemble 0x004DA530`) |
| **factory-house** | post-mission reads HouseClass power surplus (`HasPowerOutput/GetTotalPowerOutput`) for wall/structure heal-or-drain; AI auto-deploy of ConYard types; Thief credit drain | CONDITIONAL common work touches HouseClass economy globals; building leaf has high House/Factory blast radius (kept as wrapping bracket). (§2.4, §6.4; `decompile 0x006F9E50`) |
| **drawing-helpers** | smoothed display-health `+0x70`, cloak/temporal/gap-gen visuals, voice/EVA cue, damage-particle spawn | Visual catch-up + special-fx ticks are computed in the AI body but realized render-side (`+0x70` never hashed). (§2.4; render-side, out of `sim/`) |

Edges deliberately NOT created (TS-legacy/dormant — see primary doc §10.2): bridge-helpers (no live bridge predicate from the AI body in stock YR beyond building leaf), gadget-dialog/shell-dialog (UI, invariant #1 forbids `sim/`→UI), lookup-tables, ini-parsing (INI is read at load via mission-radio/rules-class, not per tick).

---

## Used-by (incoming edges)

| Source slug | Via | Evidence |
|---|---|---|
| **logicclass** | drives this service every tick: walks the active vector and calls each object's `vtable+0x5C` (=`FootClass::AI`) | LogicClass is the tick spine; `techno-foot` is the per-object body it dispatches. (`0x0055AFB0`; `read_memory 0x7E8C94`[+0x5C]) |
| **mission-radio** | mission handlers (vtable +0x204..+0x270) are invoked BY `Mission_Dispatch` which is called from `AI_Update`; handlers re-enter Techno/Foot state (NavCom, ammo, target). Bidirectional with the dependency above. | `Mission_Dispatch` is reached only via `TechnoClass::AI_Update` (sole caller, `get_function_callers 0x005B3060`); the dispatched handlers mutate Techno/Foot fields. |
| **factory-house** | production unlimbo/spawn puts new objects into the active vector; HouseClass AI commands (auto-deploy/hunt) flow into this object body's leaf AI | Newly produced units are ticked by this service the same tick they enter the vector (T4 same-pass). |
| **frontier-objects** | leaf mission state machines (Attack/Move/Carryall/Paradrop, Harvest/Unload, Deploy) and manager sub-objects (Spawn/Slave/Capture) are ticked through this body | Per-object dispatch is the entry point for every Techno-derived gameplay object's per-tick behavior. |

---

## Open / unverified edges

- **Rescue(21) FootClass assigner → damage-helpers.** Handler/slot CONFIRMED live (slot +0x258); the FootClass-side assigner via `ReceiveDamage 0x00701900 → FUN_00708080 → Queue_Mission(0x15,0)` (gated `IsPlayerControl()==0`, RandomRanged(0,99)>65 split) is RESOLVED in §0.4 but doc-sourced (V4), not re-decompiled this session — treat the edge as CONDITIONAL until traced.
- **`+0x694` sub-AI → frontier-objects (WrapAttachClass).** Per-tick dispatch ACTIVE_YR/PROVEN; the field **writer** instruction (in the un-analyzed `WrapAttachClass` ctor after `operator_new(0x1c8)`) is UNVERIFIABLE. Identity = chrono-warp attach, CONDITIONAL.
- **Leaf `ReadyToCommence` busy-flag bytes → mission-radio.** Field roles (`+0x6DD` building; `+0x6D2/+0x6D4` aircraft; `+0x6E1/+0x6E2/+0x6D1/+0x68D/+0x8D` unit/infantry) and the locomotor `vtable+0x80` idle predicate are INFERRED from constructor init, not traced setters — DRIFT/UNCHECKED.
- **cell-map cell-move edge.** No `move_cell` substrate API exists; cell relocation currently lives in the movement tick. Routing it through the substrate is part of this service's migration (§0.2).
- **AI/update-stage same-pass iteration.** Rust still uses a frozen per-phase snapshot (`live_object_order_snapshot()` world/mod.rs:1741); the same-pass re-read (`for_each_live_object`) for the AI/update stage is designed-not-done — the LogicClass-consumer addresses (`0x0055AFB0`, `FUN_0055BAA0/AE0`) are doc-sourced, not re-decompiled.
- **Type-byte labels** `+0x5ed` (Thief), `+0xc8f` (DamageParticleSystems), drain triad `+0x1cc/0x1d0/0x1d4` rest on field-map labels, not fresh xrefs — exact byte identity is DRIFT.
