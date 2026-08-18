# TechnoClass / FootClass — Engine Substrate Service Design

**Status:** STUDY + DESIGN (not an approved implementation plan). Read-only research; no Rust written.
**Date:** 2026-06-02
**Rule:** Rust-native structure, gamemd-native semantics.
**Provenance:** assembled by a workflow (7 Ghidra decode + 5 Rust-map + 3 adversarial-verify lanes -> 10 synthesized sections + completeness critic). **6 of the 15 research lanes returned structured findings — D2/D4/D6/D7 (gamemd contract), M5 (plan corpus), V2 (classifications); the other 9 did their Ghidra/file work but failed to emit the final structured output.** The sections were therefore reconciled and gap-filled by the author against ground truth: the object-AI spine was re-confirmed by a fresh `decompile_function 0x004DA530` (FootClass::AI — `TechnoClass::AI_Update` is its first call, locomotor `Process` via the +0x674 ILocomotion vtable+0x40 runs after it), and the landed Rust substrate symbols were spot-verified against the live tree (`for_each_live_object` world/mod.rs:911, `flush_pending_delete` :1024, `ObjectSubstrate` substrate.rs:48, `LogicVector` logic_vector.rs:13, `live_object_order_snapshot` :893). Every address/offset/file:line is cited inline; default verdict for unproven equivalence is DRIFT; residual UNCHECKED items are enumerated in §10.
**Companion research:** docs/research/TECHNOCLASS_AI_MIGRATION_BOUNDARY_GHIDRA_REPORT.md, FOOTCLASS_COMPLETE_GHIDRA_REPORT.md. Migration program: docs/plans/2026-05-29-core-engine-substrate-todo.md + the mission/radio substrate plan.

## 0. Round-2 Re-verification (2026-06-02) — Upgrades & Corrections

The 9 lanes that failed to emit structured output in round 1 were re-run free-text and all returned. This section is the **live-binary re-verification of record**: where it conflicts with the body sections below, **this section is authoritative** (the body was written from the 6 round-1 survivors + the author's reconciliation; this is the fresh Ghidra/file pass that resolved the gaps). Every item carries its verifying call.

### 0.1 Upgraded — previously UNCHECKED / doc-sourced, now binary-verified this session

| Claim | New status | Evidence (this session) |
|---|---|---|
| Scheduler **same-pass re-read** (T4) — mid-tick-revealed object acts the same tick | **VERIFIED** | `disassemble 0x0055AFB0`: live-object loop `0x0055b601/0x0055b608/0x0055b613` re-reads BOTH count `[EDI+0x10]` and data ptr `[EDI+0x4]` every iteration; tail-append (`FUN_0055BAA0`) is processed same pass |
| Active-vector **add 0x0055BAA0** (idempotent, append-only) / **remove 0x0055BAE0** (compacting shift-down); **+0x98** is the sole membership flag | **VERIFIED** | `decompile 0x0055BAA0` (early `return 1` if `+0x98`, else Insert, set `+0x98=1`), `decompile 0x0055BAE0` (find idx → decrement count → order-preserving shift → clear `+0x98`) |
| Per-object dispatch **vtable+0x5C = FootClass::AI 0x004DA530** | **VERIFIED** | `read_memory 0x7E8C94` dword[+0x5C] = `0x004DA530`; `disassemble 0x0055AFB0` `CALL [EDX+0x5c]` @ `0x0055b610` |
| **Global-service bracket (G1)** — ore growth/spread pre-object; Tactical→factories→houses tail; LightningStorm + particles global; Mission_Dispatch/cloak/SpawnManager/CaptureManager/passive-acquire are per-object INSIDE AI_Update | **VERIFIED** | `decompile 0x0055AFB0` (`TiberiumClass__Growth/SpreadDriver_AllTypes` @ `0x0055B4D7` precede the `+0x10` object loop; post-loop `g_Tactical`→`g_FactoryClass_Array`→`g_HouseClass_Array`; `LightningStorm__Process`/`0x005ff390` global); `get_function_callers 0x005B3060` → only `TechnoClass::AI_Update` |
| **Spine + ordering (V1)** — Unit→Foot @`0x0073647B`; Foot→AI_Update @`0x004DA539` (first call); `+0xC4` store @`0x006fa64f` then Mission_Dispatch @`0x006FA655`; locomotor Process via `+0x674` vtable+0x40 @`0x004DA877` AFTER | **VERIFIED to the byte** | `disassemble 0x007360C0 / 0x004DA530 / 0x006F9E50`; mission-dispatch-before-locomotor PROVEN |
| Locomotor **slot +0x40 identity = `DriveLocomotionClass::Process 0x004b0500`** (resolves V1's "slot identity UNVERIFIABLE") | **VERIFIED** | `read_memory 0x007e7ef0` (ILocomotion/Drive vtable +0x40) |
| Rust substrate symbols (Presence 3 variants; `for_each_live_object`:911; `flush_pending_delete`:1024; `live_object_order_snapshot`:893) | **VERIFIED in-tree** | `substrate.rs:48-77`, `game_entity.rs:144`, `mod.rs:893/911/1024` |
| **Unload-accumulator parity** (was NEEDS-PROOF, §8 #8 / §10.3) | **RESOLVED — Rust matches** | `tick_unload_accumulator` (`miner_dock_sequence.rs:194`) called at `:802` AFTER `phase_unloading` `:792`, mirroring native increment-after-Mission_Dispatch (`decompile 0x006F9E50`); units-only |
| **MissionControl reset-per-entry** (was a flagged conflict) | **CONFIRMED** | `control.rs:1-12` + `Read_INI 0x005B3760` |
| **FootClass `+0x694` identity** (was INFERRED/UNCHECKED; round-1 doc wrongly said ParasiteClass) | **RESOLVED = `WrapAttachClass*` (chrono-warp attach), CONDITIONAL(chrono-warp)** | bidirectional proof `WarpAttach+0x28 ↔ Foot+0x694` (`decompile 0x0062a4a0, 0x004deae4, 0x004d9960`). It is the chrono-warp attachment (e.g. Chrono Legionnaire), **NOT** a parasite/Terror-Drone. The per-tick dispatch from the warped unit is verified; the field **writer** site is UNVERIFIABLE this session |

### 0.2 Corrections — body claims that were WRONG or imprecise

- **RadioHistory is NOT literally write-only.** `+0xD4` **is read** inside `Receive_Radio` as a most-recent-message duplicate-suppression guard (`CMP EBX,[ESI+0xD4]` @ `0x0065a82f` gating the ring shift); `+0xD8/+0xDC` have no verifiable reader. There is no **gameplay/subclass** consumer, so **omitting the history is still behavior-safe**, but §2.3/§6.3-R4/§10.2 "write-only / zero readers" is corrected to "self-read for dedup only." (`disassemble 0x0065A820`)
- **Receiver contact insert** is **first-NULL-slot insert returning NEGATORY(0xA) when full**, not a stack push-down; contacts ptr `+0xE4`, count `+0xE8`. (matches the slot model; `disassemble 0x0065A820`, `decompile 0x0065A970`)
- **Locomotor `Process` runs MID-`FootClass::AI`** (~`0x004da877`), not "near the tail." The actual tail (`0x004daee1`) is the `+0x694` WrapAttach sub-AI dispatch. (Ordering vs AI_Update unchanged.) (`disassemble 0x004DA530`)
- **Conceal/Destructor logic-vector removal is GATED**, not unconditional: conceal on type flag `+0x234`, destructor on `+0x26`. (`decompile 0x005F4D30 / 0x005F3B80`)
- **Set_Destination_Internal labels:** `+0x82` is a **Foot-level lock flag, NOT the TechnoClass InLimbo byte**; **vtable+0x44 = `DriveLocomotionClass::Set_Destination 0x004afd40`**, not "Head_To_Coord" (Ghidra label drift — Head_To_Coord is vtable+0x18 @ `0x004afcc0`); `+0x5A8` "SuspendedNavCom" is **UNVERIFIABLE this session** (retained from FOOTCLASS_COMPLETE). (`decompile 0x004D94B0`)
- **advance_tick snapshots do NOT sort.** movement/combat/retaliation iterate the **live logic order verbatim** (`live_object_order_snapshot()` = `logic.snapshot()`, no sort — `logic_vector.rs:34`); vision (`refresh_fog` `mod.rs:1374`) + power (`:1933`) iterate `entities.values()` = **BTreeMap id-ascending**. Wherever the body says "snapshots+sorts entity ids," read "snapshots the live logic order (no sort)." Turret rotation runs after combat in Phase-5 (`mod.rs:2008`). No per-tick Presence rebuild (inline-maintained shadow; full rebuild only on deserialize `:1180`). (M2)
- **Retire-ledger citation fixes (M4):** combat attacker snapshot is inside `pub fn tick_combat_with_fog` (`combat/mod.rs:1183`; snapshot vec `:1376`; `AttackerSnapshot` push `:1494`) — the body's "`combat/mod.rs:1174..1549`" is off (`:1174` is a closing brace). `tick_retaliation` is `combat_targeting.rs:325` (not `combat/mod.rs`). Confirmed: `tick_movement_with_grids` `movement_tick.rs:820`; `tick_turret_rotation` `turret.rs:82`; `tick_aircraft_missions` `aircraft/mod.rs:152`; `tick_deploy_state` `deploy.rs:80`; **`tick_idle_scatter` `scatter.rs:71` is currently DISABLED** (commented at `world/mod.rs:2235-2243`).
- **Dock wait-queue retire target:** there is **no `RefineryDockContacts.waiting_retry_queue`** in current code (already gone). The only stored dock queue is **`AirfieldDocks.queues` (`aircraft_dock.rs:113`)** — that is the NEEDS-PROOF retire candidate. (M4)
- **Infantry fear is NOT unimplemented.** Rust has `InfantryRuntime { fear_level, is_prone }` (`game_entity.rs:48`) + `tick_fear_for_entities` (`infantry.rs:130`, called `world/mod.rs:1960`). The gap is exact gamemd parity (Fear_Decay thresholds 49/50/199, prone/crawl-fire selection, sequencer self-Destroy) → **NEEDS-PROOF, not "missing."** Wherever the body says infantry fear is "unimplemented," read "partially implemented; parity unproven." (M4 + M2)
- **Substrate API:** **`move_cell` does NOT exist** — drop it from the §7.1/§10.2 API sketch. `unlimbo` is `Simulation::unlimbo` at `world_spawn.rs:554` (not `mod.rs`). One removal **bypass remains**: `remove_wall_entity_at` (`mod.rs:1344`) calls `substrate.entities.remove` directly, skipping uninit/conceal/pending_delete — a known DRIFT to route through the substrate. (M1)

### 0.3 Still UNVERIFIABLE — updated by §0.4 (planset round-3)

Round-3 (the planset verification, 2026-06-02) resolved several of these — see §0.4. **Genuinely still open:** the `+0x694` non-zero **writer** instruction (identity confirmed, install site not pinned — lives in the un-analyzed `WrapAttachClass` ctor); existence of any frame-level deferred kill-queue beyond synchronous destruction. **Resolved in §0.4:** the Rescue(21) assigner, `+0x5A8` SuspendedNavCom, and the commence verb structure.

> **0.3-bis RESOLVED (2026-06-02, S5/L6 blocker pass).** The leaf `ReadyToCommence` busy-flag **byte internals** are no longer open: read-sites + predicates in `READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md`; Unit/Infantry setter lifecycles (+0x6D1/+0x6E1/+0x6E2, +0x2B4) in `READYTOCOMMENCE_UNIT_INFANTRY_FLAG_LIFECYCLES_GHIDRA_REPORT.md`; locomotor `vtable+0x80` idle body = `DriveLocomotionClass::Is_Moving_Now 0x004afc20` (body-verified) in `READYTOCOMMENCE_S5_BLOCKER_CLOSURE_AND_FEAR_SEQUENCE_GATE_GHIDRA_REPORT.md`. **Downgrade the §10.3 / §6.2-M5 "DRIFT until traced" flag to RESOLVED.** Corrections that pass surfaced: excluded-set is **mission 6 = Sticky, 21 (0x15) = Rescue** (the M5 "Sleep(6)/0x15" wording is wrong); Infantry `+0x2B4` is the **attack-target pointer** (not a counter); Aircraft `+0x6D4` inits to **1** (born ready). **Separately, the infantry fear-prone Down/Up "27-30" gate (§6.2/§2.6) is a current-SEQUENCE (`Doing`, +0x6C4) gate, NOT a CurrentMission gate** — see the same report; `DAT_007eaf7c` is a per-sequence table, not per-type. Still deferred (low priority): Infantry `+0x68D` set-site, `+0x8D` ObjectClass semantic, exact names of sequences 27-30, aircraft `+0x6D2/+0x6D4` runtime flip sites, the Unit garrison/spyplane-pad `FUN_004a51d0`/`+0x16BD` branch (not load-bearing for standard YR).

### 0.4 Round-3 (planset) verification — resolutions (2026-06-02)

A pre-planning verification pass (workflow `ai-shell-migration-planset`) closed these; this section supersedes the corresponding §10.1/§10.3 rows.

- **Rescue(21) assigner — RESOLVED (was UNVERIFIABLE, §10.1).** `TechnoClass::ReceiveDamage 0x00701900` (AI-victim only: `IsPlayerControl()==0`, attacker non-null, type `+0xc96` / instance `+0x3cf`) → `FUN_00708080(attacker)` → `Queue_Mission(0x15, 0)` (vtable+0x1E8) on the threat-sorted AI teammates of the victim's owner, with a **`RandomRanged(0,99) > 65` → AreaGuard(11) else Rescue(21)** split (a carried passenger forces AreaGuard). The earlier `6A 15` "conflation" was a real but unrelated radio dock-command-0x15 site. AI-only, never player-assignable — `MissionType::Rescue=21` stays AI-gated. **Lockstep note:** this path consumes `RandomRanged(0,99)` — model it at the matching per-object position. (`decompile 0x00701900, 0x00708080`)
- **`+0x5A8` = SuspendedNavCom — CONFIRMED, and it is HASHED.** Clean write at `Set_NavCom_With_Suspend 0x004d8f40` (`+0x5A8 = +0x5A4` before the override), zero-init in ctor, cleared in `PointerExpired`, and **folded into `FootClass::ComputeChecksum 0x004dbad0`**. → the Rust `suspended_nav_com` must be **hashed once authority flips** (not serde-skip-forever) — added to the NavCom-suspend slice's acceptance criteria.
- **Commence verb structure — CONFIRMED (one correction).** `Queue_Mission 0x005B35E0` writes the **queued slot `+0xB4`** (+ substate) and promotes only via the gated `Commence`; `Commence 0x005B3570` promotes `+0xB4 → +0xAC`, resets substate + **both** timers (dispatches next tick), consumes no RNG; `Assign_Mission 0x005B2FD0` force-writes `+0xAC` and bypasses the gate (only a Repair(0x1C)+Guard(5) guard). Wherever the body implies `Queue_Mission` writes `+0xAC` directly, read "writes the queued `+0xB4`; `Commence` does the `+0xB4→+0xAC` promotion." The L6 plan ships the **structural** gate only and keeps player commands on the ungated `assign_mission` force-promote path until the busy-flag internals (still §0.3) are traced.
- **`+0x694` identity re-confirmed = `WrapAttachClass*` (chrono-warp); the non-zero WRITER instruction is still UNVERIFIABLE** (lives in the un-analyzed `WrapAttachClass` ctor after `operator_new(0x1c8)`). Does not block the plan — model as `Option<WrapAttachHandle>` set on warp attach.

## 1. Overview, Scope & the Inheritance Chain

gamemd.exe inherits the Tiberian-Sun authority chain `AbstractClass -> ObjectClass -> MissionClass -> RadioClass -> TechnoClass -> FootClass -> {UnitClass, InfantryClass, AircraftClass}`, with `BuildingClass` branching off `TechnoClass` directly (it does NOT inherit `FootClass` — Building owns its own stationary mission set and has no locomotor). This study treats that chain as a **substrate service**: not a Rust trait/`dyn`/vtable hierarchy to be ported literally, but the verified *behavior contract* — field-layout offsets, mission/radio state, native ordering, RNG consumption, timer visibility, registration/removal — that every Techno carries. The intermediate bases (`MissionClass`, `RadioClass`, `TechnoClass`, `FootClass`) are never instantiated directly; they are inherited slices of state. The mission state machine, for example, is **common-Techno state** living at `+0xAC..+0xD0` (`CurrentMission +0xAC`, `QueuedMission +0xB4`, `SuspendedMission +0xB0`, dispatch timer `+0xC8/+0xD0`), driven by one common dispatcher with per-leaf vtable handler overrides — not per-leaf machinery. The Rust-native target is therefore `EntityStore` + `ObjectSubstrate` (the single lifecycle/presence owner, `src/sim/world/substrate.rs`, `Simulation.substrate` at `world/mod.rs:321`) + per-component `Option<T>` (`MissionCom`, `Contacts`, `NavigationState`) + `match category`/`CapabilityFlags` dispatch — explicitly **no** `AbstractClass`/`ObjectClass`/`TechnoClass` Rust trait tree (design §6; verified `src/sim/mod.rs:18` #1 invariant). The governing rule for the whole study: **Rust-native structure, gamemd-native semantics**, and the parity bar is *indistinguishable-from-gamemd observable output*, so any unproven structural equivalence defaults to **DRIFT**.

The central per-object spine is `UnitClass::AI (0x007360C0) -> FootClass::AI (0x004DA530) -> TechnoClass::AI_Update (0x006F9E50) -> MissionClass::Mission_Dispatch (0x005B3060) -> locomotor ILocomotion::Process`, all verified from the live binary this session (lanes D4, D6). The V1-relevant ordering verdict — **mission-dispatch-before-locomotor is PROVEN** — comes from the migration-boundary chain: `FootClass::AI` calls `TechnoClass::AI_Update` (which itself runs `Mission_Dispatch` near the end of its body, after the `+0xC4` per-object AI-tick-counter increment) and only *then* runs the locomotor `Process` (vtable+0x40) at `~0x004DA877`, i.e. mission dispatch resolves before the locomotor moves the unit in the same per-object pass (D6 confirmed; cited `decompile_function 0x004DA530`). Within `TechnoClass::AI_Update` the boundary is sharp: steps 1-20 are pre-mission common Techno work, then `+0xC4++`, then `Mission_Dispatch`, then steps 23-42 are post-mission common work (D4, `decompile_function 0x006F9E50`). This is ACTIVE_YR and load-bearing — the Rust port currently has **no single per-object AI shell** owning this order (movement is a separate global phase before combat; D6 marks the split as DRIFT against `src/sim/world/mod.rs` `advance_tick`).

```text
LogicClass main object loop  (vtable +0x5C per object)
        |
        v
  UnitClass::AI                 0x007360C0   [leaf shell: pre-Foot deploy/tube/warp]
        |  (Infantry 0x0051BAB0 / Aircraft 0x00414BB0 / Building::Update 0x0043FB20 phase 11 are the peer shells)
        v
  FootClass::AI                 0x004DA530   [parent; runs locomotor AFTER the call below returns]
        |
        v
  TechnoClass::AI_Update        0x006F9E50
        |    steps 1-20  : pre-mission common Techno work
        |    +0xC4++     : per-object AI-tick counter (NOT g_CurrentFrameCounter)
        v
  MissionClass::Mission_Dispatch 0x005B3060  (call site 0x006FA655)
        |    IsActive(+0x90) gate -> frame-anchored timer gate (CurrentFrame - +0xC8 >= +0xD0)
        |    -> Health(+0x6C)>0 gate -> switch(CurrentMission +0xAC) -> (*vtable[slot])()
        |    [<-- returns to AI_Update; steps 23-42 post-mission common work run here]
        v
  ILocomotion::Process          vtable +0x40 (~0x004DA877, back in FootClass::AI)
        |    [PROVEN: locomotor Process runs AFTER mission dispatch, same per-object pass]
        v
  UnitClass post-Foot: TurretAI -> Fire_At_Target -> Facing_Update -> HarvestBrain
                       -> Anim/Ammo(vtable+0x424) -> SpawnManager   [fire-before-facing PROVEN]
```

The mission state-machine handlers (vtable `+0x204..+0x270`) live in the dispatched handlers, NOT in the leaf AI bodies — `AircraftClass::AI` only clears a one-shot mission byte, and the aircraft Attack/Move/Carryall machines run under `Mission_Dispatch` (D6). Mission classification flowing through this spine is itself mixed-status and is detailed in later sections: Rescue (21) is CONDITIONAL (AI-house only, `IsPlayerControl()==0`); Ambush (14) is TS_LEGACY (dead 450-frame stub at `0x005B2E30`, V2 CONFIRMED — note the lane's `0x005B2E10` was the Sleep slot, corrected); Eaten (9) is CONDITIONAL with a real handler at `0x004D4CB0` but a TS enum-index-shift trap; AttackMove (29) is representable-but-never-*committed* (`case 0x1d` absent from the explicit switch → would route via `default` to Sleep `+0x204` + timer rewrite exactly like QMove if dispatched, but assign-side anti-churn keeps it off `+0xAC` so dispatch never sees it; verified `decompile_function 0x005B3060`).

## 2. Verified Active-YR Responsibilities

The Techno/Foot substrate is an inheritance chain — `ObjectClass → MissionClass(+0xAC..+0xD0) → RadioClass → TechnoClass`, with `FootClass` and the four instantiated leaves (`Building/Unit/Infantry/Aircraft`) below — that VERA20k models as Rust-native components, not a C++ class tree. The per-object AI body is `TechnoClass::AI_Update` (`0x006F9E50`), reached from each leaf's `vtable+0x5C` shell via `FootClass::AI` (`0x004DA530`); buildings reach it from `BuildingClass::Update` phase 11 (`0x0043FE36`). Only responsibilities reachable in a stock YR skirmish are listed; AI-only / type-gated work is tagged CONDITIONAL.

### 2.1 Lifecycle / scheduler membership (ObjectClass / LogicClass layer)

- **Active-object scheduler membership, same-pass re-read.** The LogicClass AI consumer (`0x0055AFB0`, inside `Main_Tick 0x0055D360`) re-reads the live active-object count each iteration; a unit revealed mid-tick acts the same tick. Add/remove gated by the 1-byte `+0x98` in-logic flag (`FUN_0055BAA0` add-once, `FUN_0055BAE0` compacting-remove). **ACTIVE_YR (membership semantics); the cited addresses (`0x0055AFB0`, `FUN_0055BAA0`, `FUN_0055BAE0`) are doc-sourced (M5/C9) and UNCHECKED this session** — corroborated, not independently re-decompiled. The mid-tick-spawn-acts-this-tick observable applies to the **AI/update stage specifically**; phase-split passes may snapshot independently.
- **`IsActive` (+0x90) gate before mission processing; `Health` (+0x6C) gate before dispatch.** `Mission_Dispatch` returns immediately if `(char)+0x90==0` (dead/inactive skip is on IsActive, **not** health) and only enters the switch when `Health>0`. **ACTIVE_YR** — decompile `0x005B3060`.
- **Self-removal / deferred-death exits.** Multiple AI bodies enqueue destruction mid-pass: `TechnoClass::AI_Update` has three early-return death points (SelfHealing step death, post-capture `IsAlive`, building EMP-restore); each leaf adds its own (`UnitClass` deploy timed-death, sinking, Guard-impassable; `InfantryClass` 3 destroy paths; `AircraftClass` crash + bounds-kill; `BuildingClass` zero-health phase 20). **ACTIVE_YR** — decompile `0x006F9E50`, `0x007360C0`, `0x0051BAB0`, `0x00414BB0`. The Rust contract enqueues to `pending_delete` with synchronous conceal/unmark/detach and deferred slot-free (one-tick `Dying` window), drained at the cleanup phase.

### 2.2 Mission dispatch + timers (MissionClass layer)

- **Frame-anchored dispatch timer gate.** Due iff `g_CurrentFrameCounter - DispatchStart(+0xC8) >= DispatchRate(+0xD0)`; the handler's return value (frames) re-arms `+0xC8/+0xD0` after each dispatch. This is **not** a per-tick decrement. **ACTIVE_YR** — decompile `0x005B3060`. Rust must use the frame-anchored snapshot model (`MissionTimer` `(start_frame,duration)`), or save/load and variable-rate paths drift (DRIFT-RISK until confirmed).
- **Mission verb layer (base impls on the common MissionClass vtable).** `Queue_Mission` (`0x005B35E0`, +0x1E8) consults `ReadyToCommence` (+0x200) then `Commence` (+0x1EC) — a **gated** promotion; `Assign_Mission` (`0x005B2FD0`, +0x1F0) writes `+0xAC` directly and **bypasses** the gate — a force-promotion. `Commence` (`0x005B3570`) resets sub-state + both timers for immediate next-tick dispatch. **ACTIVE_YR** — decompile both. A flat "commence always promotes" Rust verb is **DRIFT** (one-tick-early promotion to a still-driving unit / not-landed aircraft / not-ready building, vs silent non-promotion in gamemd).
- **`ReadyToCommence` commence gate — base + 4 leaf overrides.** Base (`0x004E0140`) = `return 1`, inherited unchanged by the non-instantiated intermediate bases; all four leaves override with real predicates: Building `0x00454250` (`+0x6DD!=0`), Unit `0x00744270`, Infantry `0x00521B60`, Aircraft `0x0041B5E0` (locomotor-idle / mission-state / busy-flag predicates). **ACTIVE_YR** — decompile all five (V2 CONFIRMED). The leaf busy-flag **byte-field semantics remain INFERRED (DRIFT)** from constructor init, not traced setters — do not treat the field offsets as field-accurate until each setter is verified.
- **Override / Restore single-depth suspend stack (+0xB0).** `Override_Mission` (`0x005B3650`) pushes the **queued** mission if one is pending, else the current mission; `Restore_Mission` (`0x005B36B0`) pops (returns false if empty). Neither resets timers/sub-state. **ACTIVE_YR** — decompile both. A naive "always save current" Override is **DRIFT** when a queued mission is pending.
- **`MissionControl` INI config (separate 32-entry global array, NOT entity state).** `GetMissionTimerEntry` (`0x005B3A00`): `base 0x00A8E3A8 + CurrentMission*0x20` (stride **0x20 = 32 bytes**, `shl eax,5`). `Read_INI` (`0x005B3760`) reads per-mission bools (NoThreat/Zombie/Recruitable/Paralyzed/Retaliate/Scatter) + `Rate`/`AARate` doubles; **AARate==0 copies Rate**; reset-per-entry. Handlers compute dispatch return as `ftol(Rate*900.0)+RandomRanged(0,2)`. **ACTIVE_YR** — decompile + byte-read. The canonical doc's "8 bytes per entry" is a stale stride claim (it meant 8 dwords); size the Rust table at 32 bytes/entry.
- **`+0xC4` per-object AI-tick counter** incremented immediately before `Mission_Dispatch` (call site `0x006FA655`), distinct from `g_CurrentFrameCounter`. **ACTIVE_YR** — decompile `0x006F9E50`.
- **Mission Rescue (21).** Dispatch case `0x15→+0x258`; real handlers `FootClass 0x004DDF90` + `AircraftClass 0x00415960`. **CONDITIONAL** — gate `IsPlayerControl()==0` (AI-owned units only; fires every AI skirmish, never on human units). V2 CONFIRMED the handler/slot is live; the specific FootClass `ReceiveDamage`-family assigner was **UNVERIFIABLE** this session (a `6A 15` site inspected was a radio-command-0x15 dock-unload transmit, not a mission-21 assign) — do not assert the FootClass assigner chain as fact.
- **Mission Eaten (9).** Dispatch case `0x9→+0x218` = real handler `FootClass::Mission_Eaten 0x004D4CB0` (mind-control follow, building-entry, consumes `RandomRanged(0,2)`). **CONDITIONAL** — gate Yuri slave/mind-control presence. Index 9 is correct; the TS artifact is the **enum-numbering shift** (Eaten retained at 9 shifts Harvest=10, AreaGuard=11, Ambush=14 vs the clean YRpp enum), **not** a dead handler.

### 2.3 Radio / contacts (RadioClass layer)

- **Synchronous radio link model.** Contacts are a capacity-bounded sparse slot array (`+0xE4/+0xE8`), capacity = `max(NumberOfDocks,1)`; first-null insert, null-hole removal with **no compaction**, sender self-evicts its own slot-0. Receiver **never** evicts; a saturated dock replies NEGATORY to every HELLO. **ACTIVE_YR** — V3 RESOLVED (doc-sourced). There is **no stored dock wait-queue/FIFO**; next docker = whoever re-probes and wins by distance-then-deterministic order. A Rust wait-queue is proven **DRIFT** — remove it.
- **`RadioHistory` (+0xD4/+0xD8/+0xDC) 3-slot push-down log.** Maintained by `Receive_Radio` (`0x0065A820`), zeroed by ctor (`0x0065A750`); duplicate messages don't push. **DORMANT** — binary-wide scan found **zero reader/consumer** (V2 CONFIRMED, exhaustive); save/load serializes contacts only, not history. Rust may omit; do **not** branch gameplay on prior radio messages.

### 2.4 Common-Techno object AI (TechnoClass layer)

These run inside `TechnoClass::AI_Update`, ordered pre-mission (steps 1–20) → `+0xC4` increment → `Mission_Dispatch` → post-mission (23–42); all verified via decompile `0x006F9E50` this session.

- **Cloak reveal/conceal.** CloakState 0 (uncloaked) + cell visible → `vtable+0x420` discovery-mark; CloakState 2 (cloaked) + cell not visible → conceal. **ACTIVE_YR** (visibility/discovery for every object). Full auto-cloak cycle / cloaking progress (`+0x220`/`+0x224`) is **CONDITIONAL** on `Cloakable=`/stealth-ability types.
- **Gattling stage helper** (`FUN_0070ed10` ×2) + turret-anim looping sound. **CONDITIONAL** — gate `TechnoType+0xCA2` (IsGattling) / `+0xCD5` (has turret-anim).
- **SpawnManager AI** (`+0x2D0→vtable+0x5C`, `SpawnManagerClass::AI 0x006B7230`). **CONDITIONAL** — `Spawns=` types (carrier/dreadnought/V3/etc.).
- **SlaveManager AI** (`+0x2D8→vtable+0x5C`). **CONDITIONAL** — `Enslaves=`/slaver types (Yuri).
- **CaptureManager (mind-control) Update** (`+0x2BC`). **CONDITIONAL** — mind-controller types (Yuri/Psi).
- **Passive/opportunity target acquisition.** Runs **after** `Mission_Dispatch`, only for missions `{2,10,5}` (Guard / Harvest / Guard), gated on `CanPassiveAcquire`+`OpportunityFire(+0x6AF)` and the `+0x180/+0x188` 45-frame scan timer + `vtable+0x4c4` suppress check. **CONDITIONAL** — covers War Miner (mission 10) and Grizzly opportunity fire. Rust lacks this (no `OpportunityFire`/`CanPassiveAcquire` parse, no 45-frame timer) → **DRIFT**.
- **Target validation/clear suite.** Ally-turned-friendly clear (pre-mission block, step 14), periodic ally recheck (`frame & 0xF`), FireError 5/6 clear, out-of-range clear (skirmish/MP human, step 35), general target-still-valid recheck (step 37). **ACTIVE_YR** (target sanity each tick). Campaign-only AI auto-clear (`g_GameMode==0`, step 34) is **CONDITIONAL** (single-player only, skipped in skirmish/MP).
- **EMP handling.** EMP-stun countdown (`+0x298`byte/`+0x29c`, restores MISSION_GUARD(5) vs MISSION_HUNT(0xF) by human-player gate) and EMPLockRemaining (`+0x504`) countdown + online-effects/anim restore. **CONDITIONAL** — gate EMP applied.
- **Health visual smoothing.** Displayed health `+0x70` snaps down on damage, lerps +1/qualifying-frame (`frame&4`) toward real Health. **ACTIVE_YR** (every damaged unit; pure visual). Rust health-bar may snap instead of lerp → **DRIFT-RISK (visual)**.
- **Voice/Voc queue + low-power/low-health EVA cue.** `+0x4F0` queued-voice playback; volume-category crossing → `VoxClass::PlayEVA` if human-owned. **ACTIVE_YR**.
- **Damage-fire particle spawn.** Below ConditionYellow + `DamageParticleSystems` type + `+0x308` empty → spawn, consuming `Random__RandomRanged` ×2 — **the only RNG consumption inside AI_Update**. **CONDITIONAL** — lockstep-relevant; Rust must consume RNG at the identical per-object position under the identical gate (**DRIFT-RISK**).
- **Other CONDITIONAL common work:** SelfHealing/organic per-tick regen + low-health anim; power-plant wall/structure heal-or-drain by house power surplus; Thief steal-credits drain; Bomb (Ivan/demo) detonation check; temporal/chrono-erase + gap-generator visuals; drain-link teardown. IronCurtain/ForceShield/Temporal timers are **passive** (checked on demand via `CurrentFrame-Start<Duration`), **not** decremented in AI_Update — do **not** model them as per-tick countdowns (**DRIFT-RISK**).

### 2.5 Navigation / locomotion (FootClass layer)

- **Locomotor `ILocomotion::Process` runs AFTER mission dispatch.** `FootClass::AI` calls `TechnoClass::AI_Update` (→ `Mission_Dispatch`) first, then drives the locomotor (`vtable+0x40`) near its tail. **ACTIVE_YR** — migration-boundary doc + decompile `0x004DA530`.
- **Sub-AI dispatch at `+0x694`** — at the tail of `FootClass::AI`, `if (this+0x694 != 0) (*(*(*(this+0x694)+0x69C))[+0x5C])()` ticks a live sub-object every frame from the host. The **dispatch is ACTIVE_YR** (mechanism VERIFIED this session, `decompile_function 0x004DA530`). Its **target identity is RESOLVED (§0.1): `WrapAttachClass*` — the chrono-warp attachment, CONDITIONAL(chrono-warp)** — proven bidirectionally `WarpAttach+0x28 ↔ Foot+0x694` (`decompile 0x0062a4a0, 0x004deae4, 0x004d9960`); **NOT** a parasite/Terror-Drone. The field **writer** site is still UNVERIFIABLE. (It is NOT in the do-not-implement set: the per-tick dispatch is real and must be reproduced.)
- **Idle scatter** every `0x3F` frames. **ACTIVE_YR** — decompile `0x004DA530`.
- **NavCom / NavQueue.** NavCom is the live single-target nav field; consumers (`OnArrival 0x004D82B0`, `Mission_Enter 0x004D9290`, `PointerExpired 0x004D9960`) decrement/shift the NavQueue (`+0x588/+0x598`, cap 10). The only **positive** populator is `FootClass::Load 0x004DB3C0` (save reconstruction); no standard YR player/team/trigger runtime push producer exists (verified-negative across `EventClass::Execute`, TeamClass convoy scripts, `TriggerAction::Execute`). **NavQueue runtime push = DORMANT.** Storage/readers stay CONDITIONAL (must tolerate nonzero from save); do **not** implement shift-click waypoint chaining or AI-patrol as Foot NavQueue appends.
- **Formation speed** (`FootClass +0x578`, propagated leader→follower). **ACTIVE_YR**. The convoy **chain-link** fields (`0x6C0–0x6D2`, follower list `+0x6C8`) are **UnitClass**-scoped, not FootClass — model convoy links on UnitClass only.

### 2.6 Leaf post-Foot work (Unit / Infantry / Aircraft / Building)

Each leaf `vtable+0x5C` is a shell: pre-Foot work → `FootClass::AI` → post-Foot work. Verified call orders, decompile `0x007360C0`/`0x0051BAB0`/`0x00414BB0` this session; Building from BUILDINGCLASS_UPDATE_AI_TICK.

- **UnitClass post-Foot order: Fire → Facing → HarvestBrain → Anim/Ammo (`vtable+0x424`) → SpawnManager.** Fire-then-Facing is load-bearing: `Fire_At_Target` reads previous-tick facing, so a single target order cannot start rotation and fire the same pass. **ACTIVE_YR** — decompile `0x007360C0`. (Corrects the "fire-then-facing-then-ammo" brief: the ammo/anim wrapper sits after HarvestBrain, not immediately after facing.) Rust currently splits fire and facing across two global phases → **DRIFT** (ordering).
- **UnitClass CONDITIONAL leaf work:** TurretAI idle scan (gate `+0xd2f!=0 && +0xd30==0`); deploy-countdown timed-death (rocket-loco `DeathFrames`); AI auto-deploy of ConYard types, AI auto-hunt, AI stuck-harvester rescue (all `IsPlayerControl()==0`); HarvestBrain (harvester/weeder types); sinking descent. Tube/tunnel traversal (`-1 < (char)+0x684`) is **DORMANT/TS_LEGACY** — `g_TubeArray` is empty in stock YR maps, branch never fires.
- **InfantryClass:** fear/prone/panic decay (`Fear_Decay_Handler 0x005200B0`, thresholds 49/50/199) **ACTIVE_YR**; death-sequence force + sequencer self-Destroy (`DoType_Sequencer 0x00520AE0`) **ACTIVE_YR**; `Fire_At_Target` (`0x005206B0`) and `Mission_Capture` (consumes the tick if nonzero) **ACTIVE_YR**; garrison-enter check **CONDITIONAL** (Occupier infantry near occupiable building). Rust has no FearLevel/prone/crawl-fire system → **DRIFT** (whole subsystem unimplemented). Tube branch (`FUN_0051B350`) **DORMANT/TS_LEGACY**.
- **AircraftClass:** AI body is a thin shell that only clears a one-shot mission byte; the aircraft mission **state machines** (Attack/Move/Guard/Carryall/Paradrop) run in the dispatched mission handlers under `FootClass::AI` (`vtable+0x210..0x270`), **not** in `AircraftClass::AI`. **ACTIVE_YR** — decompile `0x00414BB0`. Crash descent + map-bounds strafe kill **ACTIVE_YR**; Carryall position-sync to carried unit **CONDITIONAL** (`Carryall=yes`). Rust runs aircraft missions as a global sweep, not per-object dispatch → **DRIFT** (ordering).
- **BuildingClass::Update** brackets `AI_Update` (phase 11/13) with 26 building-specific phases — power-state transitions, damage-fire anims, docked-object update (`vtable+0x5C`, phase 6), ProduceCash, power charge, SAM gate, anim state machine, turret-fire counter + ROF + burst fire, zero-health destruction (phase 20, OnDestroyed/SpawnSurvivors/Limbo + return), delayed fire, overpower cleanup, auto-sell, repair+power AI, auto-production, bridge destruction, factory/transport gate. **ACTIVE_YR** except fog-of-war darkening branches (**TS_LEGACY**, `SpecialFlags & 0x1000` off by default). High blast radius (HouseClass/Factory globals) — not the first migration slice.

### 2.7 DEAD / DORMANT — not Techno/Foot substrate responsibilities in stock YR

For completeness, the following are **not** active substrate responsibilities and must not be implemented as default: Ambush mission 14 (dispatch `0xe→+0x20c` = stub `0x005B2E30` `return 0x1c2`, no leaf override, no live assigner — **TS_LEGACY**, V2 CONFIRMED, the stub address is `0x005B2E30`, not `0x005B2E10`); AttackMove 29 (`case 0x1d` absent → routes via the explicit `default` to Sleep `+0x204` **+ timer rewrite**, identical to QMove — **not** a silent skip; never *committed* because assign-side anti-churn keeps it off `+0xAC` — **ACTIVE_YR** as an assign-side selector with no dedicated handler; verified `decompile_function 0x005B3060`); QMove 3 (no case → routes to Sleep `+0x204` for all classes — **ACTIVE_YR**); Tunnel/Mech/DropPod locomotors (never instantiated — **TS_LEGACY**); aircraft AreaGuard (inherited 450-frame stub — **TS_LEGACY**); fog-of-war "previously seen" darkening and ShroudGrow (default OFF — **CONDITIONAL/DORMANT**).

## 3. Full Surface Inventory (methods, helpers, state, registries, tables, vtable/COM slots, TS paths)

This inventory is the engine-substrate map for a Rust-native TechnoClass/FootClass service: *what state and dispatch surface exists*, where it is bound, and which entries are dead in YR. Per the burden-of-proof rule, addresses/offsets are cited inline exactly as the findings give them; any equivalence not algebraically or empirically proven is left as DRIFT, and WRONG/UNVERIFIABLE verify-lane verdicts are reflected, not overridden.

### (a) Key class methods per layer (with addresses)

Inheritance chain (verified D2): `ObjectClass → MissionClass(+0xAC..+0xD3) → RadioClass → TechnoClass`, with `FootClass` an intermediate non-instantiated base and the four instantiated leaves (Building/Unit/Infantry/Aircraft) below. Each leaf `AI` is the `vtable+0x5C` slot called by the LogicClass object loop.

| Layer | Method | Address | Role / status |
|---|---|---|---|
| MissionClass | Mission_Dispatch | `0x005B3060` (vtable +0x5C) | Frame-anchored timer gate + 32-case switch → handler slot. ACTIVE_YR. Sole caller `TechnoClass::AI_Update 0x006F9E50` (call site `0x006FA655`). |
| MissionClass | Queue_Mission | `0x005B35E0` (+0x1E8) | Gated promotion: consults ReadyToCommence(+0x200) then Commence(+0x1EC). ACTIVE_YR. |
| MissionClass | Assign_Mission | `0x005B2FD0` (+0x1F0) | Force-promote; bypasses ReadyToCommence. Only the Repair(0x1C)+Guard(5) guard. ACTIVE_YR. |
| MissionClass | Override_Mission | `0x005B3650` (+0x1F4) | Push queued-if-pending else current onto 1-deep suspend slot +0xB0. ACTIVE_YR. |
| MissionClass | Restore_Mission | `0x005B36B0` (+0x1F8) | Pop +0xB0; false if -1. ACTIVE_YR. |
| MissionClass | Commence | `0x005B3570` (+0x1EC) | Promote queued, reset substate + both timers. ACTIVE_YR. |
| MissionClass | GetCurrentMission | `0x005B3040` (+0x184) | `+0xAC` first, `+0xB4` fallback. ACTIVE_YR. |
| MissionClass | Is_Mission_Suspended | `0x005B3A10` (+0x1FC) | `+0xB0 != -1`. ACTIVE_YR. |
| MissionClass | Mission_Default (Sleep/QMove/stub) | `0x005B2E10` | `mov eax,0x1C2; ret` (450f idle). Backs Sleep slot +0x204. ACTIVE_YR (as default). |
| MissionClass | Ambush stub | `0x005B2E30` | `return 0x1c2` — twin of Default; backs +0x20c. **TS_LEGACY** (V2: CORRECTION — Ambush stub is `0x005B2E30`, **not** `0x005B2E10` as the D2 lane wrote). |
| MissionClass | ReadyToCommence (base) | `0x004E0140` | `return 1`. ACTIVE_YR. Inherited unchanged by Foot/Radio/Techno bases. |
| MissionClass | GetMissionTimerEntry | `0x005B3A00` | `&MissionControl + CurrentMission*0x20`. ACTIVE_YR. |
| MissionClass | Mission_Name / Mission_From_Name | `0x005B3950` / `0x005B3910` | Name-table round-trip over `0x00816CAC` (32 entries). ACTIVE_YR. |
| MissionControlClass | Read_INI | `0x005B3760` | Per-entry Rate/AARate (AARate==0 copies Rate) + 6 bools. ACTIVE_YR. |
| TechnoClass | AI_Update | `0x006F9E50` | Common-object per-tick body; pre-mission work → +0xC4 increment → Mission_Dispatch → post-mission work. ACTIVE_YR. |
| FootClass | AI | `0x004DA530` | Calls `TechnoClass::AI_Update` (at `0x004DA539`) then locomotor `ILocomotion::Process` (vtable +0x40). ACTIVE_YR. |
| UnitClass | AI | `0x007360C0` | Pre-Foot deploy/tube/warp → FootClass::AI → TurretAI → Fire→Facing→Harvest→Anim/Ammo(+0x424)→Spawn→auto-hunt. ACTIVE_YR. |
| InfantryClass | AI | `0x0051BAB0` | Tube early-return → death-force seq → FootClass::AI → garrison → Fear_Decay → Fire → DoType_Sequencer → Locomotion_AI. ACTIVE_YR. |
| AircraftClass | AI | `0x00414BB0` | One-shot mission byte clear → docked update → FootClass::AI (in non-warp guard) → crash/bounds/Carryall. ACTIVE_YR. |
| BuildingClass | Update | `0x0043FB20` | 27-phase pipeline; AI_Update is phase 11 (`0x0043FE36`). ACTIVE_YR (fog branches TS_LEGACY). |

Leaf mission-handler bodies of note: `FootClass::Mission_Eaten 0x004D4CB0` (case 9, **ACTIVE_YR** real handler — *not* a dead stub; see (f)); `FootClass::Mission_Rescue 0x004DDF90` + `AircraftClass::Mission_Rescue 0x00415960` (case 21, both bound to slot +0x258, CONDITIONAL AI-only). FootClass overrides 12 of 28 handler slots (Attack 0x4D4DC0, Move 0x4D4200, Retreat 0x4DA2C0, Guard/Sticky 0x4D5070, Enter 0x4D9290, Capture/Sabotage 0x4D4B20, Eaten 0x4D4CB0, AreaGuard 0x4D6AA0, Hunt 0x4D5350, Rescue 0x4DDF90, Patrol 0x4D4280; Unload slot 0x4DA2B0 is a stub-return). Harvest(10) and Unload(16) are leaf-only (Unit/Infantry/Building), not FootClass.

### (b) Global helpers + singleton state

| Surface | Address / offset | Role / notes |
|---|---|---|
| LogicClass AI-stage consumer | `0x0055AFB0` (in `Main_Tick 0x0055D360`) | Active-object iteration; **re-reads** the live count each iteration from the embedded DynamicVectorClass (data +0x04, count +0x10/+0x0C) → mid-tick spawn acts same tick. ACTIVE_YR. Doc-sourced (M5), not re-decompiled this session. |
| Active-vector add | `FUN_0055BAA0` | Add-once, gated by 1-byte in-logic bool `+0x98`. ACTIVE_YR. |
| Active-vector remove | `FUN_0055BAE0` | Compacting remove, same `+0x98` gate. ACTIVE_YR. Order is tail-append, no sort (verified, not DRIFT). |
| MissionControl array base | `0x00A8E3A8` | 32 entries, **stride 0x20 (32 bytes)** per `GetMissionTimerEntry` `shl eax,5`. ACTIVE_YR. |
| g_CurrentFrameCounter | (sim `binary_frame` analogue) | Time base for all frame-anchored gates (`+0xC8`/`+0xD0`, health smooth `& 4`, ally recheck `& 0xF`, power/steal `% Rules+0x30/0x38/0x314`). ACTIVE_YR — these are **global-frame-phased**, NOT per-object-`+0xC4`-phased. |
| Per-object AI tick counter | `+0xC4` | Incremented immediately before Mission_Dispatch; its in-body consumers are mission-side. ACTIVE_YR. |
| RulesClass timers (handler rate) | `Rate*900.0` → `ftol(...)+RandomRanged(0,2)` | 900.0 = 15fps×60 (seconds→frames). Stored as Mission_Dispatch return into `+0xD0`. ACTIVE_YR. |
| RulesClass periodic intervals | `Rules+0x30` (power heal), `+0x38` (power drain), `+0x314`/`+0x318` (Thief steal interval/amount), `+0x1700`/`+0x1708` (ConditionYellow), `+0x558/0x55c`/`+0x560/0x564` (ConditionRed/Yellow particle coords) | Frame-modulo gates inside AI_Update. CONDITIONAL by feature. |
| RNG routing | — | **Only step 40 of AI_Update consumes RNG** (`Random__RandomRanged` ×2 for damage-particle pick), gated on ConditionYellow + DamageParticleSystems + `+0x308==0`. Lockstep-relevant; must consume at the same per-object position/gate. `Mission_Eaten` and handler-rate paths also consume `Random__RandomRanged(0,2)`. Per-callsite ECX selects the RNG instance (per memory `reference_rng_instance_routing_truth`); RNG_SYSTEM §3.1 stale — do not treat its routing as authoritative. |

### (c) Registries

| Registry | Location / offset | Admission / removal semantics |
|---|---|---|
| HouseClass building lists | (HouseClass; consumed by AI_Update auto-deploy via `Rules.ConstructionYardTypes 0x8b0`/count `0x8bc`) | Power heal/drain reads `HasPowerOutput/HasPowerDrain`, `GetTotalPowerOutput/Drain`. ConYard auto-deploy AI-only. CONDITIONAL. |
| Radio contacts array | RadioClass `+0xE4`/`+0xE8` (Contacts), distinct from RadioHistory `+0xD4/+0xD8/+0xDC` | **Contacts** is the live link store (slot model: first-null insert, null-hole removal NO compaction, slot-0 sender self-evict; capacity = max(NumberOfDocks,1)). **RadioHistory** is write-only push-down — **DORMANT, omit-safe** (V2 CONFIRMED: sole writer `Receive_Radio 0x0065A820`, zero reader; save/load serializes Contacts only). |
| Dock / airfield admission | `Find_Docking_Bay 0x004DF040`, `Receive_Radio 0x0065A820`, `Transmit_Radio_Impl 0x0065A970` | **NO stored wait-queue / FIFO** (M5 V3, proven DRIFT to model one). Saturated dock replies NEGATORY to every HELLO; next docker wins by distance-then-deterministic-order on re-probe. Receiver never evicts; only a full sender self-evicts its own slot-0. |

### (d) Static tables

| Table | Base / extent | Layout |
|---|---|---|
| MissionControl config | `0x00A8E3A8`, 32 entries | Stride **0x20** (32 bytes / 8 dwords) per entry — verified via `GetMissionTimerEntry 0x005B3A00` `shl eax,5`. Rate at dword[4/5] (+0x10), AARate at dword[6/7] (+0x18) and the bool placement are **INFERRED from Read_INI usage, NOT byte-verified**. **Do NOT size from `MISSIONCLASS_STATE_MACHINE.md` "8 bytes per entry"** — that is 8 *dwords*; the byte stride is 32 (retire candidate noted). The leading +0..+3 int and exact C field names are UNVERIFIED beyond Read_INI usage. |
| Mission-name table | `0x00816CAC`, end `0x00816D2C` | 32 `char*`. Verified: [0]=Sleep `0x00816e6c`, [1]=Attack, **[14]=Ambush `0x00816df8`** (off-by-one "table[15]=Ambush" claim is a self-admitted misread — index 14 is CORRECT, retire candidate), [13]=Stop, [21]=Rescue `0x00816DB4`, [31]=Spyplane Overfly `0x00816d2c`. None ptr = `DAT_00817474` ("None"). |
| Vtable mission-handler region | per-class vtable `+0x204..+0x270` | Mission-handler slots, one per dispatched mission (see (e) switch map). FootClass base region anchored at `0x007E8C94`; +0x20c = `0x007E8EA0`, +0x218 = `0x007E8EAC`. |

### (e) Vtable / COM slots

Mission_Dispatch (`0x005B3060`) switch coverage — case → vtable slot (offset from per-class vtable base), verified exhaustively:

| Case (mission) | Slot | Case | Slot | Case | Slot |
|---|---|---|---|---|---|
| 0 | +0x204 | 8 (Capture) | +0x214 | 0x14 | +0x24c |
| 1 | +0x210 | 9 (Eaten) | +0x218 | 0x15 (Rescue) | +0x258 |
| 2 | +0x22c | 10 (Harvest) | +0x224 | 0x16 | +0x250 |
| 4 | +0x230 | 0xb | +0x220 | 0x17 | +0x208 |
| 5 | +0x21c | 0xc | +0x234 | 0x18 | +0x254 |
| 6 | +0x21c | 0xd | +0x238 | 0x19 | +0x25c |
| 7 | +0x240 | 0xe (Ambush) | +0x20c | 0x1a | +0x260 |
| 0x10 | +0x23c | 0xf | +0x228 | 0x1b | +0x264 |
| 0x11 (Sabotage) | +0x214 | 0x12 | +0x244 | 0x1c | +0x268 |
| | | 0x13 | +0x248 | 0x1e | +0x26c |
| | | | | 0x1f | +0x270 |

`case 3 (QMove)` and `case 0x1d (29 AttackMove)` are **ABSENT**: both hit the **explicit `default` case** → +0x204 (Sleep) **with the full timer rewrite every case performs** (`+0xC8 = CurrentFrame`, `+0xCC`, `+0xD0 = handler return`). 29 is therefore **identical to QMove at dispatch** — *not* a silent fall-through and *not* a dispatcher skip; it is simply never a **committed** CurrentMission (assign-side anti-churn keeps it off `+0xAC`), so dispatch never sees it. *(Corrected from the prior "29 falls off the switch, no timer rewrite" claim — verified `decompile_function 0x005B3060`.)* Cases 8 and 0x11 both → +0x214. **M5 DRIFT correction (verify-reflected):** for Aircraft, `Mission_QMove 0x00415A50` is at slot **+0x230 (Retreat)**, not +0x204 — but QMove(3) still routes to the Sleep slot for all classes via the absent-case default.

Named substrate / locomotor slots:

| Slot | Method | Address (where bound) | Status |
|---|---|---|---|
| +0x5C | `<Leaf>::AI` / Mission_Dispatch (MissionClass) | leaf AI bodies; SpawnManager/SlaveManager AI also via their own +0x5C (`SpawnManagerClass::AI 0x006B7230`) | ACTIVE_YR |
| +0x200 | ReadyToCommence | base `0x004E0140`; Building `0x00454250`, Unit `0x00744270`, Infantry `0x00521B60`, Aircraft `0x0041B5E0` | ACTIVE_YR (V2 CONFIRMED all 4 leaves are real predicates) |
| +0x1E8/+0x1EC/+0x1F0/+0x1F4/+0x1F8 | Queue/Commence/Assign/Override/Restore | MissionClass base impls | ACTIVE_YR |
| +0x480 | SetDestination / Assign_Destination | invoked from AI_Update target/nav paths and leaf auto-hunt | ACTIVE_YR |
| +0x420 | DoUncloak / discovery | `0x6F4EB0` | ACTIVE_YR |
| +0x3c8 | Assign_Target / clear | `0x006FCDB0` | ACTIVE_YR |
| +0x410 | UpdateGapGenerator / special-fx tick | Building `0x00454DB0` | CONDITIONAL |
| ILocomotion +0x40 | Process | per-loco vtable; called by FootClass::AI **after** Mission_Dispatch | ACTIVE_YR |
| ILocomotion +0x44 | Head_To_Coord | per-loco vtable | ACTIVE_YR |
| IPiggyback | (piggyback locomotor interface) | per-loco | CONDITIONAL (warp/temporal/parasite piggyback) |

> **DRIFT note — leaf ReadyToCommence busy-flag bytes.** The predicate *structure* is verified (Building `+0x6DD!=0`; Aircraft `+0x6D2`/`+0x6D4`; Unit/Infantry locomotor-idle slot+0x80 + busy bytes `+0x6E1/+0x6E2/+0x6D1/+0x68D/+0x8D`), but the **field-role semantics of those busy-flag bytes are INFERRED from constructor init, not from traced setters** — DRIFT until each setter is decompiled. The locomotor `slot+0x80` "idle" predicate is UNCHECKED (not decompiled). Do not treat these byte roles as proven.

### (f) Legacy / dormant TS paths (do-not-implement / conditional)

| Surface | Status | Gate / evidence |
|---|---|---|
| Ambush mission (14) handler | **TS_LEGACY** | case 0xe → +0x20c = `0x005B2E30` `return 0x1c2` stub; no leaf override; no live assigner. Model as inert no-op (name round-trip only). |
| AttackMove (29 / 0x1D) | ACTIVE_YR; `case 0x1d` absent → **hits the explicit `default`** | **Not** a silent fall-through: routes via `default` → +0x204 (Sleep) **+ timer rewrite**, identical to QMove. Never *committed* as a CurrentMission — assign-side anti-churn (refuses Guard reassign) keeps it off `+0xAC`; the dispatcher has **no special skip**. Verified `decompile_function 0x005B3060`. |
| QMove (3) | ACTIVE_YR | No case 3; default → Sleep slot +0x204 for all classes. |
| Eaten (9) handler `0x004D4CB0` | **ACTIVE_YR** (CONDITIONAL on Yuri slave/clone) | **NOT a dead stub** — real mind-control follower logic (RNG `Random(0,2)`). The TS artifact is only the **enum index-shift** (Eaten retained at 9 shifts Harvest→10, AreaGuard→11, Ambush→14 vs the "clean" YRpp enum). Match gamemd's shifted numbering, not YRpp's. |
| Rescue (21) handler | **CONDITIONAL** (AI-only) | case 0x15 → +0x258; FootClass `0x004DDF90` + Aircraft `0x00415960` are real bodies. Live in AI skirmish (and aircraft Paradrop path), never player-assigned. The specific FootClass ReceiveDamage-family assigner is **UNVERIFIABLE** this session (V2: the `6A 15`/radio-cmd-0x15 conflation trap — `0x0051a29a` is a radio transmit of dock-unload cmd 0x15 via vtable+0x274, *not* a mission-21 assign). Treat the AI-assigner mechanism as unproven; the handler/slot LIVE verdict stands. |
| Aircraft AreaGuard (11) | TS_LEGACY | Aircraft inherits MissionClass +0x220 stub (returns 450); only Foot/Unit override. Inherited dead stub. |
| Spyplane/Paradrop (0x1B/0x1C/0x1E/0x1F) | CONDITIONAL (aircraft-only) | Slots +0x264/+0x268/+0x26C/+0x270 → AircraftClass overrides; ground leaves inherit 450-stubs. |
| Tunnel/Mech/DropPod locomotors | TS_LEGACY | CLSIDs `{4A582743}`/`{55D141B8}`/`{4A582745}`; ctors `0x00728A00`/`0x005AFEF0`/`0x004B5AB0`; zero active INI `Locomotor=` use, never instantiated (no `CoCreateInstance`). Slot-index→address mapping APPROXIMATE — do not cite slot indices for implementation. |
| InfantryClass tube sub-AI `FUN_0051B350` | DORMANT | InfantryClass::AI gate `-1 < (char)param_1[0x1a1]` (byte `+0x684`, init 0xFF); reads empty `g_TubeArray`. Gate never fires in stock YR (no tube entities). g_TubeArray emptiness is **inferred** (not enumerated map-by-map). |
| Walk/Hover locomotor path/dir-code 8 (tube teleport) | DORMANT | Requires populated `g_TubeArray`. |
| RadioHistory readers (`+0xD4/+0xD8/+0xDC`) | DORMANT | Writes active (`Receive_Radio 0x0065A820`); zero consumer (binary-wide scan = 8 RadioClass instructions only). Omit from port; do not branch gameplay on prior radio msgs. |
| NavQueue runtime PUSH producers (Foot `+0x588`/`+0x598`) | DORMANT | Only `FootClass::Load 0x004DB3C0` populates; EventClass/TeamClass/TriggerAction verified-negative. Storage/readers stay CONDITIONAL (save-load tolerance); do not reintroduce shift-click/AI-patrol appends. |
| FootClass field `0x694` sub-AI (`param_1[0x1a5]`) | dispatch **ACTIVE_YR**; identity **`WrapAttachClass` (chrono-warp), RESOLVED §0.1** | Mechanism verified this session (`decompile 0x004DA530`): `(*(*(this+0x694)+0x69C))[+0x5C]()` ticks a live sub-object every frame from the host — must be reproduced, NOT omitted. Identity = `WrapAttachClass*` (chrono-warp attach), CONDITIONAL(chrono-warp), proven bidirectionally (`decompile 0x0062a4a0, 0x004deae4, 0x004d9960`) — NOT ParasiteClass. Writer site UNVERIFIABLE. |
| Convoy chain link fields (`0x6C0–0x6D2`, list `+0x6C8`) | ACTIVE_YR but **UnitClass-scoped** | Only `formation_speed +0x578` is a real FootClass field; convoy chain is UnitClass (ctor `0x007353C0`), player-convoy only. Do NOT model convoy link on Foot/Infantry/Aircraft (mis-attributed historically). |
| Fog-of-war "previously seen" darkening / FoggedObjectClass / fog-border maintenance | CONDITIONAL (default OFF) → effectively TS_LEGACY in stock YR | Gated `SpecialFlags & 0x1000` (FogOfWar=no default, `Full_Init 0x00686B20` clears bit 0xC). The `MapClass__UpdateFogBorder` block at ~`0x004DA6C0` inside FootClass::AI is *reached* but moot when the bit is clear — do not port fog-border maintenance as default. Implement shroud only. |
| ShroudGrow regrowth | DORMANT | `ShroudGrow=no`; outer gate `Rules+0x17F0` fails by default (`ShroudRate Rules+0x1640` moot). |

## 4. Active YR vs Inactive/Legacy Separation

This section is **Deliverable #3**: a single consolidated separation table for the Techno/Foot tree, plus the explicit DO-NOT-IMPLEMENT set. Default verdict for any unproven equivalence is DRIFT; statuses are taken from the `active_vs_legacy` rows across decode lanes D2/D4/D6/D7 and the V2 adversarial verdicts, with corrections applied where the verify lane overruled a decode claim.

### 4.1 Status legend

- **ACTIVE_YR** — fires in a normal stock-YR skirmish; the Rust shell must reproduce it.
- **CONDITIONAL** — live, but only when its gate holds (type byte, AI ownership, attached subobject, game mode). Reproduce, gated exactly.
- **TS_LEGACY** — Tiberian Sun inheritance that is dead/stubbed in YR. DO NOT implement as live behavior.
- **DORMANT** — code path exists and is reachable in principle, but its trigger never occurs in stock YR (empty global, write-only field, no producer). Omit or keep storage-only.

Time base for every "frame % N" / "frame & N" gate is the `g_CurrentFrameCounter` analogue `sim.binary_frame`, not the per-object `+0xC4` tick counter (verified `0x006F9E50`: `+0xC4` is incremented but its in-body consumers are mission-side).

### 4.2 Consolidated separation table — Techno/Foot tree

| Behavior | Status | Gate | Evidence (on/off by default) |
|---|---|---|---|
| **Mission substrate (MissionClass layer)** | | | |
| Mission_Dispatch frame-anchored timer gate (due iff `binary_frame - +0xC8 >= +0xD0`; rate = handler return) | ACTIVE_YR | — | decompile `0x005B3060`; sole caller AI_Update `0x006F9E50` |
| IsActive (`+0x90`) skip-gate + Health (`+0x6C`) dispatch-gate | ACTIVE_YR | — | `0x005B3060`: `if((char)[0x24]==0) return;` then `if(0<[0x1b])` before switch |
| Queue_Mission consults ReadyToCommence (`+0x200`); Assign_Mission force-promotes ignoring it | ACTIVE_YR | — | decompile `0x005B35E0` (calls `+0x200` then `+0x1EC`) vs `0x005B2FD0` (direct `+0xAC` write) |
| ReadyToCommence base = `return 1`; all 4 leaf types override (Building `0x00454250`, Unit `0x00744270`, Infantry `0x00521B60`, Aircraft `0x0041B5E0`) | ACTIVE_YR | per-leaf, only on `queue(commence=true)` | base `0x004E0140`=`return 1`; V2 **CONFIRMED** all four are real predicates, not stubs |
| Override/Restore single-depth suspend stack (`+0xB0`); Override saves queued-if-pending else current | ACTIVE_YR | — | decompile `0x005B3650` (two-branch save) + `0x005B36B0` (pop, false if -1) |
| MissionControl Read_INI per-mission Rate/AARate (AARate==0 copies Rate); 32-entry array stride **0x20**, base `0x00A8E3A8` | ACTIVE_YR | — | decompile `0x005B3760`; byte-decode `0x005B3A00` `shl eax,5`. **Stride is 0x20 (32 bytes), not the canonical doc's "8 bytes"** |
| Mission **Rescue (21)** — AI threat-response handler bound at slot `+0x258` | CONDITIONAL | `IsPlayerControl()==0` (AI-owned only); never player-assigned | V2 **CONFIRMED** handler live: FootClass `0x004DDF90` (passenger-eject) + Aircraft `0x00415960` (paradrop). **Assigner via ReceiveDamage family is UNVERIFIABLE** — V2 could not confirm a FootClass-side mission-21 assigner (the `6A 15` sites inspected were radio-cmd-0x15 dock-unload, not mission assigns). Treat handler as live AI-only; do not hard-depend on the ReceiveDamage assigner path |
| Mission **Eaten (9)** — slave/mind-control follower handler `0x004D4CB0` | CONDITIONAL | Yuri mind-control / abduction state present | case 9→`+0x218`=`0x004D4CB0`, a **real** handler (follow controller, building-entry, RNG `Random(0,2)`). Index 9 is the TS enum-shift artifact, **not** a dead stub |
| Mission **Ambush (14)** — dispatched to a 450-frame idle stub | TS_LEGACY | none | case 0xe→`+0x20c`=**`0x005B2E30`** = `return 0x1c2`; no leaf override; no live assigner. V2 **CONFIRMED**; corrects decode lane's `0x005B2E10` (that address backs Sleep `+0x204`) |
| Mission **AttackMove (29 / 0x1D)** — `case 0x1d` absent → hits `default`; never a committed CurrentMission | ACTIVE_YR (selector) | representable; assign-side anti-churn keeps it off `+0xAC` | `decompile_function 0x005B3060`: case `0x1d` ABSENT → **explicit `default` → +0x204 (Sleep) + timer rewrite**, identical to QMove (no silent skip). Resolved upstream as a queued command, so dispatch never sees it. |
| Mission **QMove (3)** routes to Sleep (`+0x204`) via default case for all classes | ACTIVE_YR | — | `0x005B3060`: no case 3, `default→+0x204` |
| **TechnoClass::AI_Update common work (`0x006F9E50`)** | | | |
| `+0xC4` per-object AI tick counter increment before Mission_Dispatch | ACTIVE_YR | — | decompile: `[0xc4]++` immediately before dispatch call `0x006FA655` |
| Health visual smoothing (`+0x70` lerps +1/frame on `frame&4` toward Health; snaps down on damage) | ACTIVE_YR | — | decompile body; pure display catch-up |
| Cloak reveal/conceal via CloakState 0/2 + cell IsVisibleToHouse → `+0x420` (DoUncloak `0x6F4EB0`) | ACTIVE_YR | — | decompile: CloakState==0 reveal, ==2 conceal |
| Voice/Voc queue (`+0x4f0`) + low-power/health EVA cue | ACTIVE_YR | human-owned for EVA | decompile: `if([0x4f0]!=-1)` play; category change → `VoxClass::PlayEVA` if human |
| Target validation/clear suite (ally-turned, out-of-range, FireError 5/6, bridge/terrain, frame&0xF recheck) | ACTIVE_YR | has Target | decompile: multiple `+0x3c8(0)` clears |
| Timer-cluster periodic accumulator (`+0xf8 += +0x110`; miner unload etc.) — runs AFTER Mission_Dispatch | CONDITIONAL | non-building (`RTTI!=6`) AND `+0x10c` rate set | bytes `0x006FABC4..0x006FAC2A`; building branch skips it |
| Passive/opportunity target acquisition | CONDITIONAL | mission ∈ {2,10,5} AND CanPassiveAcquire+OpportunityFire(`+0x6AF`) AND scan timer `+0x180/+0x188` (45f) expired AND `!+0x4c4` | decompile `0x006FA699..0x006FA6C1`; covers Grizzly/War-Miner opp-fire |
| Auto-cloak full cycle / cloaking progress | CONDITIONAL | `Cloakable=` / veteran StealthAbility types | CLOAKING doc; `+0x2A0` CanAutoCloak `0x6FBDC0`, `+0x220/+0x224` |
| Gattling stage helper (`FUN_0070ed10` ×2) + turret-anim looping sound | CONDITIONAL | TechnoType `+0xCA2` / `+0xCD5` | decompile gates; GATTLING doc |
| SpawnManager AI (`+0x2d0`→`+0x5c`) | CONDITIONAL | `Spawns=` types (CARRIER/DEST/DRED/BSUB/V3) | decompile; SpawnManagerClass::AI `0x006B7230` |
| SlaveManager AI (`+0x2d8`→`+0x5c`) | CONDITIONAL | `Enslaves=`/slaver types (Yuri SLAV) | decompile; SLAVE_MANAGER doc |
| CaptureManager (mind-control) Update (`+0x2bc`) | CONDITIONAL | mind-controller types (Yuri/Psi) | decompile: `if(CaptureManager) Update()` |
| EMP-stun countdown (`+0x298`/`+0x29c`) + mission restore; EMPLockRemaining (`+0x504`) + RestoreOnlineEffects | CONDITIONAL | EMP applied to object | decompile: countdown→`+0x3c8(0)`+`+0x1e8(5 or 0xf,0)`; final block clears anim/restore |
| SelfHealing per-tick (organic regen + low-health anim release) | CONDITIONAL | `SelfHealing=`/organic types | decompile: `+0x298()` step + later `+0x294()` regen, ConditionYellow `Rules+0x1700` |
| Power-plant wall/structure heal-or-drain by house power surplus | CONDITIONAL | power-tied wall (RTTI 0xf)/unit (Type `+0xD97`); `frame % Rules+0x30`/`+0x38` | decompile post-self-regen block |
| Thief steal-credits per-tick drain | CONDITIONAL | Thief mission active + TechnoType `+0x5ed` + `frame % Rules+0x314` | decompile (field-map label, not fresh setter trace → DRIFT on exact byte) |
| Damage-fire particle system spawn (consumes RNG `Random__RandomRanged` ×2) | CONDITIONAL | TechnoType `+0xc8f` DamageParticleSystems + HealthRatio<ConditionYellow + `+0x308==0` | decompile; **lockstep-relevant — must consume RNG at same per-object position under same gate** |
| Bomb (Ivan/demo) detonation timer check | CONDITIONAL | attached BombClass `+0x38` and `!+0x81` safe-flag | decompile: `if(+0x38 && !+0x81 && IsTimerExpired()) Detonate()` |
| Temporal/chrono-erase visual; gap-generator visual (`+0x410` tick) | CONDITIONAL | temporal state `+0x198/+0x1a4` (init 10=off) / gap-generator owner | decompile: unconditional call, no-ops when inactive |
| IronCurtain/ForceShield/Temporal **timers** | CONDITIONAL (passive) | iron-curtained object | IRONCURTAIN doc: checked on-demand (`CurrentFrame-Start<Duration`); **AI_Update does NOT decrement these** — do not add a per-tick countdown |
| Campaign-only AI target auto-clear | CONDITIONAL | `g_GameMode==0` (single-player campaign), AI house | decompile: skipped in skirmish/MP |
| Skirmish/MP human out-of-range target clear | ACTIVE_YR | `g_GameMode!=0` + human owner | decompile: `+0x3c8(0)` when GetWeaponRange(-1)<0 |
| RadioHistory / Ambush(14) / Eaten(9) special path **inside AI_Update** | TS_LEGACY | — | not present in AI_Update body; passive-acquire set is {2,10,5}, no 9/14 path here |
| Fog-of-war darkening of seen-not-visible cells | TS_LEGACY | `SpecialFlags & 0x1000` (FogOfWar) | AI_Update cloak block uses IsVisibleToHouse for reveal/discovery only; fog default OFF |
| **Leaf-class AI shells (D6)** | | | |
| UnitClass::AI Fire_At_Target **before** Facing_Update (fire reads previous-tick facing) | ACTIVE_YR | — | decompile `0x007360C0`; post-Foot order Fire→Facing→HarvestBrain→Anim/Ammo(`+0x424`)→Spawn |
| UnitClass deploy-countdown timed-death self-destruct | ACTIVE_YR | DeathFrames set (rocket-loco / V3 types) | decompile: `+0x1b6` vs Type `+0xe38` → Death_Explosion+Destroy |
| InfantryClass fear/prone/panic decay (Fear_Decay_Handler `0x005200B0`) | ACTIVE_YR | — | decompile `0x0051BAB0`; thresholds 49/50/199 |
| InfantryClass death-sequence force + sequencer self-Destroy | ACTIVE_YR | — | decompile: Health<1 force (exempt set), DoType_Sequencer death→`+0xf8` |
| AircraftClass mission state machines (Attack/Move/Guard/Carryall/Paradrop) | ACTIVE_YR | — | decompile `0x00414BB0`: AI body only clears a one-shot byte; state machines run via Mission_Dispatch under FootClass::AI |
| AircraftClass crash descent + map-bounds strafe kill | ACTIVE_YR | — | decompile: crash-flag Z descent w/ -400 destroy; FlyBy/FlyBack==0 + IsStrafe OOB → Destroy |
| UnitClass TurretAI idle scan | CONDITIONAL | `TurretNotHidden`(`+0xd2f`)≠0 AND `!TurretLocked`(`+0xd30`==0) | decompile gates |
| UnitClass auto-hunt + stuck-harvester rescue | CONDITIONAL | AI-controlled (`IsPlayerControl()==0`) | decompile both gated AI-only |
| InfantryClass garrison-enter check from AI | CONDITIONAL | Occupier infantry near occupiable building | decompile: not-selected + Guard(5)/Sleep(0xB) + CanGarrison |
| AircraftClass Carryall position sync to carried unit | CONDITIONAL | `Carryall=yes` (Type `+0xdfc`) | decompile: copy 24B to carried `+0x388/+0x3a0` |
| BuildingClass::Update 27-phase pipeline wrapping AI_Update (phase 11 @ `0x0043FE36`) | ACTIVE_YR | — | BUILDINGCLASS_UPDATE_AI_TICK doc; fog-darkening branches within are TS_LEGACY |
| UnitClass / InfantryClass tube-traversal sub-AI branch | DORMANT (TS) | `byte+0x684 != 0xFF` AND populated `g_TubeArray` | decompile `0x007360C0` / `0x0051BAB0`+`0x0051B350`: reads `g_TubeArray`; no tube entities in stock YR maps → never fires |

### 4.3 DO-NOT-IMPLEMENT set (explicit)

These are the items a TechnoClass/FootClass shell must NOT implement as live behavior. Each is verified DEAD/DORMANT in stock YR.

| Surface | Status | Why dead/off | Native owner / evidence |
|---|---|---|---|
| **Tunnel / subterranean locomotor** (TunnelLocomotionClass) | TS_LEGACY | CLSID `{4A582743}` **zero** INI refs; ctor `0x00728A00` only xref is WinMain factory + own QI; no `CoCreateInstance`; never instantiated | TS_DORMANT_LOCOMOTORS §4 |
| **Mech / DropPod locomotors** | TS_LEGACY | Mech `{55D141B8}` 8 INI refs all commented; DropPod `{4A582745}` zero refs; ctors `0x005AFEF0`/`0x004B5AB0` never invoked | TS_DORMANT_LOCOMOTORS §2-3 |
| **Tube branches in Walk/Hover locomotors + Infantry/Unit tube sub-AI** | DORMANT | gated on `byte+0x684 != 0xFF` + populated `g_TubeArray`, which is empty in stock YR maps | decompile `0x0051B350`; WALK §6.2.3 / HOVER §9 |
| **Fog-of-war "previously seen" darkening** (FoggedObjectClass, fog A-buffer, fog-border maintenance) | TS_LEGACY / CONDITIONAL-off | `SpecialFlags & 0x1000` (FogOfWar) never set; `[MultiplayerDialogSettings] FogOfWar=no` (rulesmd.ini:205, :3040); `Full_Init 0x00686B20` clears bit 0xC. V2 **CONFIRMED** off. Implement shroud only (black for unexplored) | OBJECT_FOG_VISIBILITY; `RulesClass__ReadMultiplayerDialogSettings 0x00671EA0` |
| **ShroudGrow regrowth** | DORMANT | `ShroudGrow=no` (rulesmd.ini:677); PerTick outer gate `Rules+0x17F0` fails by default | PERTICKUPDATE_UNNAMED_CALLEE_RESOLUTION |
| **Ambush mission (14)** real behavior | TS_LEGACY | dispatches to `0x005B2E30` = `return 0x1c2` 450-frame idle stub; no leaf override; no live assigner. Keep only as an inert no-op enum variant for INI name round-trip | V2 CONFIRMED; case 0xe→`+0x20c` |
| **RadioHistory readers** (RadioClass `+0xD4/+0xD8/+0xDC`) | DORMANT (write-only) | sole writer is `Receive_Radio 0x0065A820` (3-deep push-down); binary-wide scan = 8 instructions, **zero consumer**; save/load serializes contacts (`+0xE4/+0xE8`) only, not history. V2 **CONFIRMED** omit-safe. Do NOT branch gameplay on prior radio messages | RADIOHISTORY_READ_USE_SCAN |
| **Dead WaypointQueue / NavQueue runtime producers** (Foot `+0x588/+0x598`) | DORMANT | only `FootClass::Load 0x004DB3C0` (save reconstruction) populates it; player commands, TeamClass convoy scripts, and TriggerAction::Execute all verified-negative for pushes. Keep storage/readers for save-load tolerance only; do NOT reintroduce shift-click waypoint chaining or AI-patrol as NavQueue appends | NAVCOM_NAVQUEUE_PUSH_PRODUCERS |
| **Dock/radio wait-queue or FIFO** | (do not design) | gamemd has NO stored wait-queue; saturated refinery/airfield replies NEGATORY, next docker wins by distance-then-deterministic re-probe; receiver never evicts, only a full sender self-evicts slot-0. Remove any `waiting_retry_queue`/`AirfieldDocks.queues` | M5 V3 proven DRIFT |
| **C++ AbstractClass/ObjectClass/TechnoClass/FootClass trait/vtable tree** | (architecture) | dispatch stays `match category` + `Option::is_some()` + capability flags, not inheritance/`dyn`/COM | M5 invariant #3, design §6 |

#### Field 0x694 — sub-AI dispatch ACTIVE_YR (implement); identity = WrapAttachClass (chrono-warp)

The brief flagged FootClass field **`0x694`** as a candidate for the DO-NOT-IMPLEMENT set "if unresolved." The **dispatch is ACTIVE_YR and must be implemented**, not omitted. Mechanism verified at the tail of `FootClass::AI 0x004DA530`: `if (param_1[0x1a5] != 0) (**(code**)(**(int**)(param_1[0x1a5] + 0x69c) + 0x5c))();` — `+0x694` holds a pointer to an object; at `+0x69C` within it is a pointer to an `AbstractClass`-derived object whose `AI()` (`vtable+0x5C`) is invoked every tick from the host. **Round-2 resolved the identity (see §0.1): `+0x694` is a `WrapAttachClass*` — the chrono-warp attachment (e.g. Chrono Legionnaire warp), CONDITIONAL(chrono-warp)** — proven bidirectionally via `WarpAttach+0x28 ↔ Foot+0x694` (`decompile 0x0062a4a0, 0x004deae4, 0x004d9960`). It is **NOT** a parasite/Terror-Drone (the round-1 "ParasiteClass" claim was wrong). The field **writer** site remains UNVERIFIABLE this session; `FOOTCLASS_COMPLETE §9.2/§10` ("unknown large-object pointer") is superseded by this resolution.

Rust handoff caveat (DRIFT-RISK): the warped unit's AI drives the WrapAttach sub-AI tick from the warped unit in this ordering slot (FootClass::AI tail, after TryEnterTransport, before piggyback release) — reproduce the dispatch from the host side; CONDITIONAL on a chrono-warp being attached.

### 4.4 Carry-over UNCHECKED / DRIFT flags affecting this separation

- **ReadyToCommence leaf busy-flag byte semantics** (`+0x6DD` building; `+0x6D2/+0x6D4` aircraft; `+0x6E1/+0x6E2/+0x6D1/+0x68D/+0x8D` unit/infantry) are INFERRED from constructor init, not from decompiled setters → **DRIFT** until each setter is traced before the Slice-6 Rust hook is field-accurate. The base=`return 1` and all-four-override facts are CONFIRMED (V2); only the predicate field internals are unproven.
- **Rescue (21) FootClass assigner** via the ReceiveDamage family is **UNVERIFIABLE** (V2): the handler/slot are confirmed live AI-only, but no FootClass-side mission-21 assigner was confirmed this session. Do not build the design on a FootClass-side Rescue assigner without tracing `TechnoClass::ReceiveDamage 0x00701900`.
- **Locomotor "idle" predicate** (`loco slot+0x80`, consumed by Unit/Infantry ReadyToCommence) was not decompiled → exact idle semantic **UNCHECKED**.
- **Type byte labels** `+0x5ed` (Thief), `+0xc8f` (DamageParticleSystems), and the drain triad `+0x1cc/+0x1d0/+0x1d4` rest on field-map labels, not fresh per-field xrefs → treat exact byte identity as **DRIFT** until re-verified.
- **`g_TubeArray` emptiness** in every stock YR map is **inferred** (no tube INI/map entities), not enumerated map-by-map — the tube DORMANT verdict is HIGH but rests on that inference.

## 5. Comparison Against Current Rust Architecture

This section contrasts the gamemd-native contract decoded in §1–§4 against the current VERA20k Rust implementation, names where the two already align, and enumerates the deltas a TechnoClass/FootClass substrate-service design must close. Per CLAUDE.md burden-of-proof, every unproven equivalence defaults to **DRIFT**; ordering and lifecycle differences are surfaced regardless of trigger frequency.

### 5.1 Structural model: inheritance chain vs flat GameEntity + capability components

gamemd's object identity is a C++ inheritance chain `AbstractClass → ObjectClass → MissionClass(+0xAC..+0xD3) → RadioClass(+0xD4..+0xE8) → TechnoClass(size 0x520) → {Unit/Infantry/Aircraft/Building}` with per-class vtable dispatch (mission handlers at vtable +0x204..+0x270, `ReadyToCommence` at +0x200, leaf `AI` at +0x5C). State is split across the chain by layer: MissionClass owns `+0xAC` CurrentMission, `+0xB0` SuspendedMission, `+0xB4` QueuedMission, `+0xB8` IsCommenced, `+0xBC` MissionState, the dispatch timer at `+0xC8/+0xD0`; TechnoClass owns the ~80 fields touched by `AI_Update` (`+0x70` smoothed health, `+0xC4` AI tick counter, `+0xac` mission mirror, `+0x180/+0x188` passive-scan timer, `+0x220` CloakState, `+0x2bc` CaptureManager, `+0x2d0` SpawnManager, `+0x2d8` SlaveManager, `+0x504` EMPLockRemaining, etc.; per `get_struct_layout TechnoClass`/`decompile_function 0x006F9E50`).

The Rust target is the inverse: a **flat `GameEntity` in a `BTreeMap<u64, GameEntity>` `EntityStore`**, with `match category` + `Option<T>::is_some()` capability dispatch and **no trait hierarchy / no `dyn` / no COM/vtable plumbing**. This is a deliberate, invariant-backed decision (`ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md §6` lines 168-205; `EntityCategory` lives in `map/entities.rs`, consumed one-way by `sim/`). A substrate design that re-introduces an `AbstractClass/ObjectClass/TechnoClass/FootClass` Rust trait tree contradicts this and is rejected (M5 collision #1, #4). The verified behavior to preserve is the *layer-split state* and *per-category dispatch points*, not the chain — the layering becomes capability components (`MissionCom`, `Contacts`, `NavigationState`) gated by an `EntityCategory` + a spawn-time `CapabilityFlags` bitset.

| gamemd contract | Rust today | Gap |
|---|---|---|
| 5-layer inheritance chain; identity = class | flat `GameEntity` in `BTreeMap` `EntityStore`; identity = `(EntityCategory, Option<T> components)` | **ALIGNED on philosophy** — the flat model is the intended end-state, not a gap. Do NOT port the chain. |
| Per-leaf vtable dispatch (`AI` +0x5C, mission handlers +0x204..+0x270, `ReadyToCommence` +0x200) | `match category` + `Option::is_some()`; planned `dispatch_category(id) -> (EntityCategory, CapabilityFlags)` once at spawn | Capability-flag bitset (Slice 8) **not landed**; scattered `category==X && opt.is_some()` checks not yet routed through one audited surface. |
| MissionClass state at fixed offsets `+0xAC..+0xD0` | `MissionCom` component (current/queued/suspended/substate/timer/tick_counter), shadow → authoritative | **ALIGNED on shape**; authority flip is Slice 6, not done (shadow only, landed ff1d2a32). |
| TechnoClass `+0x520` field block driving `AI_Update` | fields scattered across `GameEntity` + per-system structs; several unmodeled (smoothed health `+0x70`, opportunity-fire, fear) | Multiple **DRIFT/DRIFT-RISK** items (5.4). |

### 5.2 Execution model: per-object AI spine vs global phased advance_tick

This is the largest structural delta. gamemd drives every object through one **per-object AI spine** invoked from `LogicClass`'s active-vector consumer (`0x0055AFB0` in `Main_Tick 0x0055D360`): `LogicClass → <Leaf>::AI (+0x5C) → FootClass::AI (0x004DA530) → TechnoClass::AI_Update (0x006F9E50) → MissionClass::Mission_Dispatch (0x005B3060)`, with the locomotor `ILocomotion::Process` (vtable +0x40) running **after** mission dispatch inside `FootClass::AI`. Within `AI_Update` the order is load-bearing and verified (`decompile_function 0x006F9E50`): pre-mission common work (steps 1–20) → **`+0xC4` increment** → `Mission_Dispatch` → post-mission common work (steps 23–42), with three early-return death points. Each leaf wraps this: `UnitClass::AI (0x007360C0)` runs pre-Foot deploy/tube/warp → `FootClass::AI` → TurretAI → **Fire_At_Target → Facing_Update** → HarvestBrain → ammo/anim (vtable +0x424) → SpawnManager (verified `decompile 0x007360C0`; fire-then-facing means fire reads previous-tick facing — a single target order cannot start rotation and fire the same pass). `BuildingClass::Update (0x0043FB20)` brackets `AI_Update` (its phase 11/13) inside a 27-phase pipeline.

Rust today runs a **global phased `advance_tick`** (the M2 phase table): commands → ground movement → air/special movement → vision → power → turrets+combat → retaliation+passengers → scatter+production+repairs+docks+ore → AI → defeat → building anims+cleanup → state hash (`src/sim/world/mod.rs`; tick order matches CLAUDE.md and M5). There is **no single per-object AI body** that owns the pre-mission → +0xC4 → dispatch → post-mission ordering; combat, turret rotation, cloak, target-validation, and aircraft missions are each separate global sweeps over independently-taken snapshots.

| gamemd contract | Rust today | Gap |
|---|---|---|
| Per-object AI spine; `AI_Update` runs pre-mission → +0xC4++ → `Mission_Dispatch` → post-mission, all in one body per object | global phased `advance_tick`; no per-object shell; mission/combat/turret/cloak are separate phases | **DRIFT (ordering).** No host for the per-object sequence with its 3 early-return death/EMP points. Affects `world/mod.rs advance_tick` + all combat/movement phases. |
| `UnitClass::AI`: Fire_At_Target **then** Facing_Update (coupled, fire uses prior facing) | combat snapshots attackers (`combat/mod.rs:1174..1549`, dispatched `world/mod.rs:1732..1777`); turret rotation a separate later sweep (`movement/turret.rs:82..95`, `world/mod.rs:1778`) | **DRIFT.** Fire and facing split across two global phases vs coupled per-object fire-before-facing. (`retire_candidates`: `tick_turret_rotation`, global attacker snapshot.) |
| Locomotor `Process` runs **after** mission dispatch inside `FootClass::AI` | movement (Phase 1) runs **before** aircraft missions/combat | **DRIFT (ordering).** Movement precedes mission dispatch in Rust; native dispatches mission then moves. |
| Aircraft mission state machines dispatched via `Mission_Dispatch` under a thin `AircraftClass::AI` shell | global aircraft mission sweep snapshots all aircraft (`aircraft/mod.rs:144..183`) | **DRIFT.** Missions are a global phase, not per-object dispatch under a Foot-equivalent shell. (`retire_candidate`.) |
| `BuildingClass::Update` 27-phase wrapper owns pre/post-`AI_Update` ordering incl. zero-health destruction + IsAlive guard | power/production/repair/gates remain separate global phases (`world/mod.rs:1704..2078`) | **DRIFT (ordering).** No per-building Update shell; high blast radius — buildings are explicitly **not** the first migration leaf (Unit is safest first slice). |

A subtlety the design already records (and a substrate shell must honor): the **iteration model is per-phase, not blanket**. gamemd's combined AI stage re-reads the live active-vector count each iteration (`0x0055AFB0`), so a unit revealed mid-tick acts the same tick (verified-as-doc-sourced, C9). Rust currently uses a **frozen per-phase snapshot** (`live_object_order_snapshot()` at `world/mod.rs:1741`). The planned resolution is same-pass re-read for the AI/update stage specifically, frozen snapshots acceptable for the phase-split movement/combat passes — core-engine-substrate TODO #1, **still open**. A shell must not hardcode one iteration model for all updates (M5 collision #5). (`live_object_order_snapshot()` is *defined* at `world/mod.rs:893`; `:1741` is the AI-phase call site. The substrate's same-pass iterator `for_each_live_object` already exists at `:911` — it is the alternative the AI/update stage wants.)

### 5.3 Existing substrate (M1) and in-flight mission/radio substrate (M3) vs what is missing

The object-lifecycle substrate (M1) has materially landed: `ObjectSubstrate` is the single owner of `EntityStore` + `LogicVector` (active-object order, tail-append no-sort, verified) + `OccupancyGrid` + `pending_delete` + monotonic no-reuse `ids` (`world/mod.rs:321`; `for_each_live_object` at `:911`; `uninit` enqueues with a one-tick Dying window at `:974-1009`; `flush_pending_delete` at `:1024` drained at command boundary, Phase 9, and load). The single `Presence` FSM (`Limbo | InCell | Dying` — three variants; conceal transitions `InCell → Limbo`, there is no separate `Concealed` state) exists as a **shadow** (`game_entity.rs:144`, `derived_presence()` at `:467`, serde-skip, asserted not-hashed). The mission/radio substrate (M3) has landed Slices 0–3: `sim/mission/{mod,timer,control}.rs` and `sim/radio/{mod,contacts,receive}.rs` exist, with gate timers migrated onto `MissionTimer` (792d6051), `MissionCom` shadow (ff1d2a32), and `Contacts` replacing the `radio_contacts: Vec` (6943e8ed).

| gamemd contract | Rust today | Gap |
|---|---|---|
| `LogicClass` active vector, +0x98 in-logic flag, deferred PendingDeleteList free | `ObjectSubstrate` owns store/logic/occupancy/pending_delete; `uninit`→enqueue, conceal/unmark synchronous, slot-free deferred | **ALIGNED (M1 landed, Slices 1/2/6).** Reveal gate-chain + Presence rollback on Mark failure = Slice 7, **open**. |
| Single Presence FSM (Limbo/InCell/Dying), one transition per state | `Presence` enum present as **shadow** (3 variants — conceal → Limbo, no `Concealed` state), shadows the old scattered limbo gates | **ALIGNED on shape**; authority flip not done (shadow only). |
| Frame-anchored `DispatchTimer` (`+0xC8` start, `+0xD0` rate; due iff `frame - start >= rate`); handler return re-arms; never decrements | `MissionTimer` = frame-anchored `(start_frame, duration)` delta gate, never decrements, SENTINEL=u32::MAX, base=`sim.binary_frame` | **ALIGNED** by design (`mission/timer.rs`). Must verify the landed timer is the snapshot model, not a per-tick decrement (5.4 confirms the requirement). |
| `MissionControl` 32-slot array, stride 0x20, base 0x00A8E3A8, AARate==0 copies Rate, reset-per-entry | `MissionControl` INI table, 32 slots, reset-per-entry, AARate-absent copies Rate (`mission/control.rs`) | **ALIGNED.** Note: canonical `MISSIONCLASS_STATE_MACHINE.md` mis-states stride as "8 bytes" — that is 8 *dwords*; true byte stride is 0x20 (verified `read_memory 0x005B3A00`, `shl eax,5`). Size the table from the verified layout, not that doc. |
| Override/Restore single-depth suspend slot `+0xB0`; Override saves **queued-if-pending else current** | `MissionCom` models exactly one suspended slot (Slice 2) | **ALIGNED on capacity**; the queued-takes-priority save rule must be honored — a naive "always save current" diverges on the suspend/restore round-trip (verified `decompile 0x005B3650`). |
| `Contacts` sparse radio links; first-null insert, null-hole removal no-compaction, slot-0 sender self-evict; capacity max(NumberOfDocks,1); **no wait-queue** | `Contacts` slot model replaces `radio_contacts: Vec` (Slice 3); synchronous `transmit()`/`receive_radio()` RadioBus | **ALIGNED.** V3 proves gamemd has **no stored dock wait-queue/FIFO** — a saturated dock replies NEGATORY, next docker wins by distance-then-deterministic re-probe, receiver never evicts. Any `waiting_retry_queue`/`AirfieldDocks.queues` in Rust is **DRIFT to remove**. |

**Still missing (designed, not done):** per-EntityCategory `ready_to_commence()` hook (4 leaf impls; base = return 1) — Mission/Radio Slice 6, **open**; the verb API (`assign_mission`/`queue_mission`/`commence_queued`/`override_mission`/`restore_mission`/`get_current_mission`/`is_busy`) — Slice 6, open; RadioBus refinery idiom (Slice 4, possibly mid-landing — untracked `radio/receive.rs` + modified `miner_dock*`/`world_hash.rs` in the working tree); `MissionCom`→authoritative flip (Slice 6); `TypeHandleTable` one-hop type resolution (Slice 8); two RNG streams (TODO #2); the same-pass AI/update iteration (TODO #1).

### 5.4 Behavioral DRIFT / DRIFT-RISK the shell must close

These are output-affecting gaps surfaced by the `AI_Update` and leaf-AI decode, defaulting to DRIFT per burden-of-proof:

- **Queue+commence gating — DRIFT.** gamemd `Queue_Mission(commence=true)` is gated by per-leaf `ReadyToCommence` (+0x200; base `0x004E0140`=return 1; Building `0x00454250`, Unit `0x00744270`, Infantry `0x00521B60`, Aircraft `0x0041B5E0`); `Assign_Mission (0x005B2FD0)` **force-promotes** ignoring it (verified `decompile 0x005B35E0` vs `0x005B2FD0`; CONFIRMED by V2). Until the per-type hook lands (Slice 6), a flat `commence()` that unconditionally promotes diverges: a queued-commence-now to a still-driving unit / not-landed aircraft / not-ready building promotes one tick early in the port but silently fails to promote in gamemd. **Note:** leaf busy-flag byte semantics (`+0x6DD` building; `+0x6D2/+0x6D4` aircraft; unit/infantry locomotor-idle `loco slot+0x80` + busy bytes) are **INFERRED from constructor init, not from traced setters — DRIFT until each setter is traced** before the Slice-6 hook can be field-accurate.

- **Opportunity / passive target acquisition — DRIFT.** gamemd runs passive acquisition only for missions `{2,10,5}` after `Mission_Dispatch`, gated on CanPassiveAcquire + OpportunityFire (`+0x6AF`) + the `+0x180/+0x188` 45-frame scan timer + a suppress predicate (verified `decompile 0x006FA699..0x006FA6C1`). Rust has no `OpportunityFire`/`CanPassiveAcquire` parse, no 45-frame scan timer, and only auto-acquires for AttackMove/Guard intents — **Grizzly / War-Miner opportunity fire is missing**.

- **Timer-cluster unload accumulator ordering — DRIFT-RISK.** gamemd's `+0xf8 += +0x110` accumulator fires on the `+0x100/+0x108` periodic timer **after** `Mission_Dispatch`, is **units-only** (buildings RTTI 6 skip it), and `Mission_Deploy` state-3 samples `+0xf8` *during* dispatch, before this tick's increment (verified bytes `0x006FABC4..0x006FAC2A`). The Rust miner unload accumulator exists (`miner_dock_sequence`) but must be confirmed to increment **after** mission sampling and to remain units-only — cross-check `src/sim/miner/*`.

- **Health-bar visual smoothing — DRIFT-RISK (visual).** gamemd lerps a displayed-health field `+0x70` up toward real Health at +1 per `frame&4` qualifying frame, snapping down instantly on damage (verified `decompile 0x006F9E50`). Unknown whether Rust has a `+0x70`-equivalent; without it the health bar snaps instead of lerping after repair/regen.

- **Damage-particle RNG position — DRIFT-RISK (lockstep).** The only `AI_Update` RNG consumption is the damage-fire particle spawn (`Random__RandomRanged ×2`) gated on DamageParticleSystems type + HealthRatio < ConditionYellow + `+0x308==0` (verified). If Rust consumes RNG for damage particles at a different per-object tick position or under a different gate, the shared stream desyncs — must match position and gate exactly.

- **Passive/implicit timers must not decrement — DRIFT-RISK.** IronCurtain/ForceShield/Temporal are checked on demand (`CurrentFrame - Start < Duration`), **not** decremented in `AI_Update` (verified; `IRONCURTAIN_FORCESHIELD` doc). Do not model these as per-tick countdowns. Same rule applies to any timer a shell adds: frame-anchored `MissionTimer`, never a per-tick `u8/u16/i16`.

- **Infantry fear/prone/panic — DRIFT (partial; parity unproven).** `InfantryClass::AI` calls `Fear_Decay_Handler (0x005200B0)` (thresholds 49/50/199), prone/crawl-fire sequence selection, and a sequencer self-Destroy path (verified `decompile 0x0051BAB0`). Round-2 correction: Rust **has** `InfantryRuntime { fear_level, is_prone }` (`game_entity.rs:48`) + `tick_fear_for_entities` (`infantry.rs:130`, called `world/mod.rs:1960`) — the subsystem is **present, not missing**. The DRIFT is whether it reproduces gamemd exactly (the 49/50/199 thresholds, prone/crawl-fire selection, the sequencer self-Destroy) — **NEEDS-PROOF**.

### 5.5 TS-legacy / dormant — do NOT implement in the shell

The shell must encode these as inert, not as live behavior (per §D7; all CONFIRMED by V2 where re-verified):

- **Ambush (mission 14) — TS_LEGACY.** Dispatch case 0xe → vtable +0x20c → dead 450-frame stub returning `0x1C2` (verified the stub is at `0x005B2E30`, a twin of Mission_Default `0x005B2E10` which backs Sleep +0x204 — **not** the same address; V2 correction). No leaf override, no live assigner. Model as an inert no-op enum variant for INI name round-trip only.
- **Eaten (mission 9) — ACTIVE_YR handler but TS enum-index-shift trap.** Case 9 → FootClass `0x004D4CB0` is a **real** handler (mind-control follower; consumes `Random(0,2)`), live for Yuri mind-control. The TS artifact is only the **enum numbering** — Eaten retained at index 9 shifts every mission at/after Harvest by +1 vs the clean YRpp enum (so Harvest=10, AreaGuard=11, Ambush=14; name-table index 14=Ambush is correct, **no off-by-one**). Rust's mission enum must match gamemd's shifted numbering wherever it cross-references binary/INI mission codes — **UNCHECKED**, verify.
- **Rescue (mission 21) — CONDITIONAL (AI-only), real handler.** Case 0x15 → vtable +0x258; FootClass `0x004ddf90` and Aircraft `0x00415960` are real handlers (CONFIRMED, V2). Live only for AI-owned units (`IsPlayerControl()==0`), fires every AI skirmish, never on human units — include a handler but gate it AI-only. **Caveat:** the specific FootClass ReceiveDamage-family assigner is **UNVERIFIABLE** this session (the inspected `6A 15` site was a radio-command-0x15 dock-unload transmit via vtable+0x274, not a mission-21 assign — do not conflate). Do not design around a FootClass-side Rescue assigner until traced.
- **AttackMove (mission 29) — CONDITIONAL (representable; never a *committed* mission).** `case 0x1d` is absent, so 29 hits the **explicit `default` → +0x204 (Sleep) + timer rewrite**, identical to QMove — **not** a silent fall-through and **not** a dispatcher skip. It is never executed only because assign-side anti-churn (e.g. refuses Guard reassign) keeps it from ever being committed to `+0xAC`; the dispatcher needs **no special skip**. The selector must be representable. Verified `decompile_function 0x005B3060`.
- **QMove (mission 3) — ACTIVE_YR routes to Sleep.** No case 3; default → vtable +0x204 (Sleep slot) for all classes.
- **RadioHistory (`+0xD4/+0xD8/+0xDC`) — DORMANT, omit-safe.** Write-only push-down maintained by `Receive_Radio (0x0065A820)`, **no reader/consumer** anywhere (CONFIRMED, V2; exhaustive scan). Do not branch any gameplay on prior radio messages; not serialized (save/load carries Contacts `+0xE4/+0xE8` only).
- **NavQueue runtime push — DORMANT.** No standard YR runtime producer (only `FootClass::Load` reconstructs on save-load). Append already removed in Rust (`movement_commands.rs`, 2026-05-28) — **aligned**; do NOT reintroduce shift-click waypoint chaining or AI-patrol NavQueue appends. Keep storage/readers for save-load tolerance.
- **Tunnel/subterranean & fog-of-war darkening — TS_LEGACY/CONDITIONAL-default-off.** Tunnel locomotors (Tunnel/Mech/DropPod) never instantiated; infantry tube sub-AI (`+0x684` gate / `g_TubeArray`) dormant. Fog darkening gated on `SpecialFlags & 0x1000` (FogOfWar=no default, CONFIRMED V2) — implement shroud only. The `FootClass` `+0x694` sub-AI is **not** dormant: its per-tick dispatch is verified ACTIVE_YR (`decompile 0x004DA530`) and must be reproduced — and the pointed-to object's identity is now **RESOLVED (§0.1): `WrapAttachClass` (chrono-warp attach), CONDITIONAL(chrono-warp)**, NOT the parasite/Terror-Drone of the round-1 draft.

## 6. The gamemd-native Behavior Contract

This is the semantic contract the Rust substrate must honor for the Techno/Foot tree. It is phrased as invariants on **observable behavior** — ordering, lifecycle visibility, timer cadence, RNG consumption, link semantics — not on C++ class shape. The Rust-native structure (`ObjectSubstrate` + `EntityCategory` + capability flags + per-component `Option<T>`, no vtable/COM/dyn tree) is settled elsewhere; what follows is what that structure must *do*.

### 6.1 Per-object tick contract

**Invariant T1 — single per-object AI body, three-segment shape.** Each live object's update runs as one ordered body: pre-mission common work → increment the per-object AI tick counter (`+0xC4`) → `Mission_Dispatch` → post-mission common work. The counter increment sits immediately before dispatch (`decompile 0x006F9E50`, increment at the call site preceding `Mission_Dispatch 0x005B3060` at `0x006FA655`); it is **not** the global frame counter. ACTIVE_YR. The BUILDINGCLASS a–z phase paraphrase is *not* execution order; the decompile order is authoritative.

**Invariant T2 — leaf shell brackets the parent call.** The per-category body is `<Leaf>::AI → FootClass::AI (0x004DA530) → TechnoClass::AI_Update (0x006F9E50) → Mission_Dispatch (0x005B3060)`, with the locomotor `ILocomotion::Process` (vtable `+0x40`) running **after** mission dispatch inside FootClass::AI. Mission *state machines* live in the dispatched handlers (vtable `+0x204..+0x270`), never in the leaf AI shell. ACTIVE_YR (UnitClass `0x007360C0`, InfantryClass `0x0051BAB0`, AircraftClass `0x00414BB0`, BuildingClass::Update `0x0043FB20` all re-decompiled). Aircraft mission state machines specifically are in dispatched handlers, not in `AircraftClass::AI` (which only clears a one-shot mission byte).

**Invariant T3 — UnitClass post-Foot order is Fire → Facing → HarvestBrain → Anim/Ammo → Spawn.** `UnitClass__Fire_At_Target` runs immediately before `UnitClass__Facing_Update`, so **fire reads the previous tick's facing** — a single target order cannot both start rotation and fire in the same pass. The ammo/anim wrapper (`vtable+0x424`) sits *after* the harvest brain, not immediately after facing. ACTIVE_YR (`decompile 0x007360C0`). The Rust split (combat snapshot at `world/mod.rs:1732..1777`, turret rotation as a later sweep at `:1778` / `movement/turret.rs`) is **DRIFT** against this per-object fire-before-facing coupling — proven different ordering, not proven-equivalent.

**Invariant T4 — same-pass append/remove visibility for the AI/update stage.** The active-object consumer re-reads the live vector count each iteration (LogicClass AI stage `0x0055AFB0` within `Main_Tick 0x0055D360`; membership gated by the 1-byte `+0x98` flag via add-once / compacting-remove), so an object revealed mid-stage acts the **same** tick. The Rust substrate must iterate the AI/update stage with a re-read length (`for_each_live_object`), **not** a frozen snapshot. CONDITIONAL by stage: gamemd has one combined AI stage; the Rust spine is phase-split and each phase may snapshot independently — same-pass is load-bearing *for the AI/update stage specifically*, not necessarily for the phase-split movement/combat passes. Current code still uses a frozen snapshot for the AI/update path (`world/mod.rs:1741`) — that is open, designed-not-done (DRIFT until migrated). *(Active-vector consumer addresses are doc-sourced, not re-decompiled this session.)*

**Invariant T5 — deferred death, synchronous teardown.** Object removal enqueues to a pending-delete list; it does **not** free synchronously. Link teardown, conceal, and occupancy-unmark happen synchronously at `uninit`; only the slot free defers, draining at the cleanup phase to reproduce the one-tick `Dying` window. Each leaf body has multiple mid-pass self-removal exits that must honor this enqueue-not-free rule (UnitClass: timed-death `Death_Explosion`, sinking descent, Guard-terrain destroy; InfantryClass: death-force sequence, garrison-terrain destroy, sequencer death-completion — 3 paths; AircraftClass: crash descent + map-bounds strafe kill; BuildingClass: zero-health destruction at phase 20). ACTIVE_YR. A shell `drop`/synchronous-free breaks the verified deferred-death semantics.

**Invariant T6 — RNG is consumed at a fixed per-object position.** Inside `AI_Update` the only RNG consumption is the damage-fire particle pick (`Random__RandomRanged` ×2), gated on HealthRatio < ConditionYellow AND `DamageParticleSystems` type AND the particle slot empty (`+0x308==0`). Other frame-phased work (`& 4` health smoothing, `& 0xF` ally recheck, `% Rules` power/steal) consumes no RNG. The Rust substrate must consume the shared RNG stream at the same per-object position under the same gate or the lockstep stream desyncs (DRIFT-RISK). ACTIVE_YR/CONDITIONAL by gate.

**Invariant T7 — passive/opportunity acquisition is mission-gated and post-dispatch.** Passive target acquisition runs **after** `Mission_Dispatch`, only for current-mission ∈ {2, 10, 5} (Guard/area, Harvest, Guard), gated on CanPassiveAcquire + OpportunityFire + the `+0x180/+0x188` 45-frame scan timer + the suppress predicate (`vtable+0x4c4`). This is the War-Miner (mission 10) and Grizzly opportunity-fire path. ACTIVE_YR/CONDITIONAL. Rust currently lacks an `OpportunityFire`/`CanPassiveAcquire` parse and the {2,10,5}-gated post-mission scan — DRIFT.

**Invariant T8 — the units-only periodic accumulator runs after dispatch.** The `+0xf8 += +0x110` accumulator fires on its `+0x100/+0x108` frame-anchored periodic timer **after** `Mission_Dispatch`; buildings (RTTI 6) skip it entirely. `Mission_Deploy_Building` state-3 samples `+0xf8` *during* dispatch, i.e. before this tick's increment. CONDITIONAL (non-building, `+0x10c` rate set; this is the miner-unload accumulator, step init `+0x110=1`). The Rust miner-unload accumulator must increment after mission sampling, not before, and stay units-only (DRIFT-RISK; cross-check `src/sim/miner/*`).

### 6.2 Mission contract

**Invariant M1 — one current selector, queued fallback.** Mission identity is a single `CurrentMission` selector (`+0xAC`, init -1) read by the dispatch switch; `GetCurrentMission` returns CurrentMission, else the QueuedMission (`+0xB4`) fallback (`0x005B3040`). The dispatch switch keys on `CurrentMission` exclusively. ACTIVE_YR.

**Invariant M2 — frame-anchored timer gate, never a decrement.** A mission is due iff `g_CurrentFrameCounter - DispatchTimer_Start(+0xC8) >= DispatchTimer_Rate(+0xD0)`; the rate is the handler's return value, re-armed by writing `+0xC8 = frame` after each dispatch (`decompile 0x005B3060`). It is **not** a per-tick decrementing counter, so skipped/variable-rate ticks do not drift cadence and save/load is exact. ACTIVE_YR. The Rust `MissionTimer` must be the snapshot `(start_frame, duration)` model against `sim.binary_frame`, never a `u8/u16/i16` decrement (DRIFT-RISK; this is the cadence-drift class).

**Invariant M3 — the gates before dispatch.** Mission processing is skipped entirely when the object is inactive (`IsActive +0x90 == 0`); the switch handler is invoked only when Health (`+0x6C`) > 0. The IsActive gate is checked first, after the base `ObjectClass__AI()` tick; the Health gate guards the switch specifically. ACTIVE_YR (`decompile 0x005B3060`).

**Invariant M4 — queue/assign/commence is a gated-vs-forced contrast.** `Queue_Mission(commence=true)` (`0x005B35E0`) consults `ReadyToCommence` (vtable `+0x200`) and **skips** `Commence` when it returns false; `Assign_Mission` (`0x005B2FD0`) writes `CurrentMission` directly and **force-promotes**, ignoring `ReadyToCommence`. `Commence` (`0x005B3570`) promotes the queued mission and resets sub-state + both timers so the new handler dispatches next tick. ACTIVE_YR/CONFIRMED (V2). A flat "commence always promotes" Rust verb is **DRIFT**: queue+commence-now to a still-driving unit / not-landed aircraft / not-ready building would promote one tick early in the port yet silently fail to promote in gamemd.

**Invariant M5 — ReadyToCommence base = 1, four real leaf overrides.** Base `ReadyToCommence` (`0x004E0140`) returns 1, inherited unchanged by the three non-instantiated intermediate bases (Foot/Radio/Techno). All four instantiated leaves override with real predicates (CONFIRMED V2): Building `0x00454250` (`*(char*)(this+0x6DD)!=0`); Unit `0x00744270` and Infantry `0x00521B60` (locomotor-idle AND not Sleep(6)/0x15 AND not mid-Attack/Guard-with-target AND busy-flags clear; Infantry adds a recruit-table gate); Aircraft `0x0041B5E0` (not Sleep(6)/0x15 AND busy `+0x6D2` clear-unless-0x1E AND ready `+0x6D4` set). The Rust per-EntityCategory `ready_to_commence()` hook (Slice 6, **not yet landed**) must reproduce these four predicates. The **exact busy-flag byte semantics** (`+0x6DD`, `+0x6D2/+0x6D4`, unit/infantry `+0x6E1/+0x6E2/+0x6D1/+0x68D/+0x8D`) and the locomotor-idle predicate (`loco slot+0x80`) are **INFERRED from constructor init, not from traced setters** — treat as **DRIFT/UNCHECKED** until each setter is traced before the hook is claimed field-accurate.

**Invariant M6 — single-depth override/restore stack with queued-priority save.** `Override_Mission` (`0x005B3650`) pushes onto the **one** SuspendedMission slot (`+0xB0`): it saves the **queued** mission if one is pending, otherwise the **current** mission. `Restore_Mission` (`0x005B36B0`) pops it (returns false if `+0xB0==-1`). Neither resets timers or sub-state. ACTIVE_YR. A naive "always save current" Override is **DRIFT** when a queued mission is pending — it diverges on the suspend/restore round-trip. The Rust `MissionCom` must model exactly one suspended slot with the queued-takes-priority rule.

**Invariant M7 — MissionControl is a separate 32-entry config array, reset-per-entry.** Per-mission rate config lives in a separate `MissionControlClass` global array, **not** entity state: base `0x00A8E3A8`, **stride `0x20` (32 bytes)**, 32 entries (`GetMissionTimerEntry 0x005B3A00`: `shl eax,5; add 0x00A8E3A8`). `Read_INI` (`0x005B3760`) reads NoThreat/Zombie/Recruitable/Paralyzed/Retaliate/Scatter bools plus `Rate` and `AARate` doubles; **AARate==0 copies Rate**. Handlers compute their dispatch return as `ftol(Rate*900.0)+RandomRanged(0,2)` (900.0 = 15fps×60). The canonical doc's "8 bytes per entry" is **WRONG** (it is 8 *dwords* = 32 bytes) — do not size the Rust table from it. ACTIVE_YR. *(RESOLVED: the landed `control.rs:1-12` implements reset-per-entry — "no carry-forward between missions" — matching the verified `Read_INI 0x005B3760`; the mission/radio plan's earlier P0 "carry-forward" wording is superseded by the shipped code.)*

**Invariant M8 — mission status classification (do not over-implement TS-legacy).**
- **Sleep(0)** dispatched; **QMove(3)** has *no* case → routes to the Sleep slot (`+0x204`) via `default` for all classes (ACTIVE_YR). **AttackMove(29/0x1D)** also has *no* case → it hits the **same explicit `default` → +0x204 (Sleep) + timer rewrite**, identical to QMove — **not** a silent fall-through and **not** a dispatcher skip. 29 is never *executed* only because assign-side anti-churn (refuses Guard reassign) keeps it from being committed to `+0xAC`. The Rust selector must be representable-but-never-committed; **no dispatcher skip is needed**. ACTIVE_YR — verified `decompile_function 0x005B3060`.
- **Rescue(21)** is a **real** handler at slot `+0x258` (FootClass `0x004DDF90`, Aircraft `0x00415960`) and is **CONFIRMED LIVE**, gated **AI-only** (`IsPlayerControl()==0`); fires every AI skirmish, never on human units. Include a Rust handler. *(The specific FootClass-side ReceiveDamage-family assigner is **UNVERIFIABLE** this session — the `6A 15`/`PUSH 0x15` byte-pattern hits include radio-command-0x15 dock-unload transmits via `vtable+0x274`, not mission assigns; do **not** assert a FootClass Rescue assigner as fact, and beware the radio-cmd-0x15 vs mission-0x15 conflation.)*
- **Ambush(14)** is **CONFIRMED TS_LEGACY dead stub**: case `0xe → +0x20c → 0x005B2E30` (`return 0x1c2`, a 450-frame idle twin of Mission_Default, **not** literally `0x005B2E10`), no leaf override, no live assigner. Model as an inert no-op for INI name round-trip only.
- **Eaten(9)** is **not** a dead stub: case `9 → +0x218 → FootClass 0x004D4CB0` is a **real** handler (mind-control follower behavior, consumes `Random(0,2)`). It is **CONDITIONAL** on Yuri slave/mind-control presence. The only TS artifact here is the **enum index-shift** — `Eaten` retained at index 9 shifts every mission at/after Harvest by +1 vs the "clean" YRpp enum, so the Rust enum must match gamemd's shifted numbering (Harvest=10, AreaGuard=11, Ambush=14, Rescue=21). The substrate-doc "table[15]=Ambush off-by-one" worry is **resolved**: name-table index 14 = "Ambush" is correct, no shift in the *string* table (`read_memory 0x00816CAC`, entry[14]=`0x00816df8`).

### 6.3 Radio contract

**Invariant R1 — synchronous transmit/receive.** Radio messages are delivered by synchronous `transmit()`/`receive_radio()` within the tick (RadioClass `Transmit`/`Receive_Radio 0x0065A820`), not queued for a later phase. The Rust `RadioBus` (`sim/radio/`) models this synchronous call directly. ACTIVE_YR.

**Invariant R2 — slot model, no compaction.** Contacts are a capacity-bounded **sparse slot** structure (capacity = `max(NumberOfDocks, 1)`): insert at first-null, remove by nulling the hole **without** compaction. The Rust `Contacts` type (Slice 3, landed — replaces the old `radio_contacts: Vec<u64>`) must keep this sparse no-compaction semantics; compacting would renumber slots and change slot-0 semantics. ACTIVE_YR.

**Invariant R3 — receiver never evicts; only a full sender self-evicts slot-0.** A saturated dock replies NEGATORY to every HELLO and **never evicts** an existing contact; the receiver does not bump anyone. The next docker is whoever re-probes and wins by distance-then-deterministic-order. Only a **full sender** self-evicts its own **slot-0**. This is **PROVEN DRIFT** against any stored wait-queue/FIFO: gamemd has no dock wait-queue. The Rust port must **remove** `RefineryDockContacts.waiting_retry_queue` and `AirfieldDocks.queues` and must **not** design a FootClass docking wait-queue. ACTIVE_YR/V3.

**Invariant R4 — RadioHistory has only a self-dedup reader; omit it (behavior-safe).** The 3-deep RadioHistory log (`+0xD4/+0xD8/+0xDC`) is self-maintained by `Receive_Radio`: it reads `+0xD4` (`CMP EBX,[ESI+0xD4]` @ `0x0065a82f`) as a most-recent-message duplicate-suppression guard, then push-shifts only on a *differing* head. So it is **not literally write-only** (round-2 correction to the round-1 "zero readers" claim) — but its **only** reader is that internal dedup; **no gameplay or subclass consumer reads it**, and save/load serializes contacts `+0xE4/+0xE8`, not history. Omitting it is therefore still **behavior-safe**: the Rust port omits it and must **not** branch any gameplay decision on prior radio messages. DORMANT-for-gameplay.

### 6.4 Navigation contract

**Invariant N1 — NavCom owner-state vs active path/locomotor split.** Navigation owner-state (NavCom destination intent) is distinct from the active path and the locomotor process; the locomotor `Process` (vtable `+0x40`) runs after mission dispatch (see T2), reading NavCom rather than owning it. The Rust substrate keeps `NavigationState` (NavCom) as a separate component from the mission selector and from locomotor execution; a FootClass shell must **not** fold "current mission"/"is busy"/NavCom into one struct. ACTIVE_YR.

**Invariant N2 — no runtime NavQueue push producer.** The FootClass NavQueue (`+0x588/+0x598`, capacity 10) has **no standard YR runtime producer**: the only positive populator is `FootClass::Load 0x004DB3C0` (save-game reconstruction). Player commands (`EventClass::Execute 0x004C6CB0`), TeamClass convoy scripts, and `TriggerAction::Execute 0x006DD8B0` were all verified-negative (no `+0x598` push). Storage/readers/save-load tolerance stay CONDITIONAL (must accept nonzero from a save); the runtime push producer is **DORMANT/absent**. The Rust port (append already removed; `movement_commands.rs` only clears) must **not** reintroduce a NavQueue push on shift-click waypoint chaining or AI patrol. DORMANT.

**Invariant N3 — warp/parasite guards bracket the navigation body.** The main locomotor body is guarded by the warp-out/being-warped vtable predicates (`+0x1d4`/`+0x1d8`); under warp the body short-circuits to a locomotor `Process` + IsAlive guard and skips normal navigation. The FootClass tail runs a sub-AI dispatch (`+0x694` → object at `+0x69C` → its AI via `vtable+0x5C`) — the dispatch is **ACTIVE_YR** (verified `decompile 0x004DA530`); the pointed-to object's identity is **RESOLVED (§0.1): `WrapAttachClass` (chrono-warp attach), CONDITIONAL(chrono-warp)**, proven bidirectionally (`decompile 0x0062a4a0, 0x004deae4`). The tube-traversal branch (`InfantryClass +0x684` / `g_TubeArray`) is **DORMANT/TS_LEGACY** — no tube entities in stock YR maps, gate stays 0xFF. *(Whether Rust drives the WrapAttach sub-AI tick from the **warped unit's** AI body in this ordering slot is **UNCHECKED**.)*

### 6.5 Global-service bracket (outside the per-object loop)

**Invariant G1 — V3 stays out of the object body.** The per-object tick contract above is bracketed by global pre/post-loop stages that **do not** migrate into the object loop: the advance_tick phase order is preserved verbatim (commands → ground movement → air/special movement → vision → power → turrets+combat → retaliation+passengers → scatter+production+repairs+docks+ore → AI → defeat → building anims + cleanup/flush_pending_delete → state hash). House/Factory/superweapon services, defeat detection, and `flush_pending_delete` (the deferred-death drain, T5) are global-bracket stages, not per-object work. ACTIVE_YR. No slice collapses or reorders these phases; only state *representation* and teardown *call sites* change — there is **no monolithic per-object dispatch rewrite**.

**Invariant G2 — passive timers are checked on demand, never ticked in the object body.** IronCurtain / ForceShield / Temporal durations are evaluated on demand (`CurrentFrame - Start < Duration`), **not** decremented inside `AI_Update`. The Rust port must model these as `(start_frame, duration)` compared against the frame counter, never as per-tick countdowns (DRIFT-RISK; same cadence class as M2). CONDITIONAL by effect presence.

**Invariant G3 — single time base.** All frame-anchored gates (M2, T8, G2) read one time base — the `g_CurrentFrameCounter` analogue `sim.binary_frame`, committed late at end-of-tick so consumers observe the pre-increment frame. `tick` and `binary_frame` must never be mixed. ACTIVE_YR.

**Invariant G4 — building work stays a wrapping bracket, not object-body migration.** `BuildingClass::Update` brackets `AI_Update` (phase 11/13) with 26 building-specific phases (power transitions, ProduceCash, gates, delayed fire, auto-sell, repair, auto-production, bridge destruction, zero-health destruction) that touch House/Factory globals. These stay as the building's wrapping bracket; Buildings are explicitly **not** the first migration leaf for exactly this entanglement reason (UnitClass is the safest first slice — fully verified shell order, behavior-bearing Fire-before-Facing and locomotor-after-dispatch orderings, separable existing Rust phases to fold under it). ACTIVE_YR; fog-of-war darkening branches inside building special-effects remain **TS_LEGACY** (off by default, `SpecialFlags & 0x1000`).

## 7. Rust-native Replacement Boundary (the design)

This section proposes the Rust-native module boundary that implements the verified TechnoClass/FootClass behavior contract **without** porting the C++ class tree. It is the structural payload for Deliverable #6. It is deliberately consistent with the already-planned object-substrate + mission/radio-substrate end-state (M5); it adds the per-object **AI shell** that those plans leave open, not a parallel ownership scheme. Per the project invariant: *Rust-native structure, gamemd-native semantics.* No `AbstractClass`/`ObjectClass`/`TechnoClass`/`FootClass` trait tree, no `dyn`/vtable dispatch, no raw pointer vectors, no global-singleton mutation. Dispatch stays `match category` + `Option::is_some()` (M5 design §6; ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md §6 lines 168-205).

### 7.1 Ownership: one owner, three step-functions, no new class tree

The substrate that already exists is the home; the shell is a **scheduler that walks substrate-owned order and dispatches plain step functions**. Nothing in the shell owns lifecycle, presence, mission, radio, or active-vector state.

```
sim/
  world/
    substrate.rs        ObjectSubstrate  (EXISTING owner — DO NOT duplicate)
                          store: EntityStore (BTreeMap<u64,GameEntity> + by_owner)
                          logic: LogicVector (active-object order; in-logic flag)
                          occupancy: OccupancyGrid
                          pending_delete: Vec<u64>; ids (monotonic)
                          API: unlimbo/reveal/conceal/uninit/
                               flush_pending_delete/change_owner  (ONLY presence mutators;
                               NB no move_cell method exists today — see §0.2)
    mod.rs              Simulation { substrate, binary_frame, rng… } ; advance_tick
    ai/                 <— NEW: the AI shell (the design)
      mod.rs             object_ai_stage(sim)  — the scheduler/owner of native order
      techno_common.rs   techno_common_pre / techno_common_post  (plain fns)
      foot_nav.rs        foot_locomotor_step                      (plain fn)
      leaf_unit.rs       unit_pre / unit_post                     (plain fns)
      leaf_infantry.rs   infantry_pre / infantry_post
      leaf_aircraft.rs   aircraft_pre / aircraft_post
      leaf_building.rs   building_pre / building_post  (wraps; NOT first slice)
  mission/  { mod, timer, control }   EXISTING — MissionCom authoritative (Slice 6)
  radio/    { mod, contacts, receive } EXISTING — Contacts + RadioBus
```

`sim/ai/` depends only on `sim/` siblings and reads/commits through the substrate API and the mission/radio verb API. It NEVER references `render/`, `ui/`, `sidebar/`, `audio/`, `net/` (invariant #1, `src/sim/mod.rs:18`). `EntityCategory` stays in `map/entities.rs`, consumed one-way; the shell derives a sim-side `CapabilityFlags` at spawn (M5 collision #4) — it does **not** re-home category as a sim-side class identity.

### 7.2 The scheduler: `object_ai_stage` walks live-object order, dispatches a per-category shell

This is the Rust stand-in for the LogicClass AI consumer (`0x0055AFB0` inside `Main_Tick 0x0055D360`, doc-sourced via M5/C9) and the leaf `vtable+0x5C` bodies (`UnitClass::AI 0x007360C0`, `InfantryClass::AI 0x0051BAB0`, `AircraftClass::AI 0x00414BB0`, `BuildingClass::Update 0x0043FB20` — all D6, HIGH, re-decompiled). It runs as the **AI phase** of `advance_tick` (between docks/production and defeat detection) and must **not** reorder or collapse any other phase (M5 invariant #2).

```
fn object_ai_stage(sim: &mut Simulation) {
    // SAME-PASS re-read for THIS stage (D6/M5 C9): a unit revealed mid-stage
    // acts this stage. NOT a frozen snapshot — that snapshot model is correct
    // for the phase-split movement/combat passes, NOT for the AI/update stage.
    let mut i = 0;
    while i < sim.substrate.logic.len() {          // re-read len each iteration
        let id = sim.substrate.logic.get(i);       // active-vector ORDER, not BTreeMap order
        i += 1;
        if !sim.substrate.is_active(id) { continue; } // IsActive +0x90 gate (D2 step 2)
        match sim.category(id) {
            Unit      => unit_ai(sim, id),
            Infantry  => infantry_ai(sim, id),
            Aircraft  => aircraft_ai(sim, id),
            Building  => building_ai(sim, id),
        }
    }
}
```

Active-vector order is authoritative tail-append (M5 C8, resolved NOT-DRIFT); it is distinct from `EntityStore` BTreeMap order and both must stay distinct. The `i < len()` re-read (not a cached length) is the verified mid-tick-spawn-acts-this-tick semantic for the AI stage specifically (D6/M5; the design §6 line 198 caveat: decide per-phase, AI/update stage is the same-pass one).

### 7.3 The per-category shell = pre → +0xC4 → Mission_Dispatch → Foot locomotor → post

Each `*_ai(sim, id)` is a thin shell that reproduces the verified leaf call order (D6). The parent chain `<Leaf> → FootClass::AI (0x004DA530) → TechnoClass::AI_Update (0x006F9E50) → MissionClass::Mission_Dispatch (0x005B3060)`, with the locomotor `ILocomotion::Process` (vtable+0x40) running **after** mission dispatch (D6, HIGH). The Techno-common body is **not** top-of-function: the `+0xC4` per-object AI-tick counter increment and `Mission_Dispatch` happen near the END of the common work, after a large pre-mission block (D4, verified `0x006F9E50`; the BuildingClass a–z paraphrase is NOT the true order).

```
fn unit_ai(sim, id) {
    // PRE-FOOT (UnitClass::AI 0x007360C0, verified order):
    //   warp/parasite early-process, deploy-countdown timed death (self-remove),
    //   tube branch (DORMANT, TS), AI auto-deploy, then →
    techno_common_pre(sim, id);          // D4 steps 1–20  (TechnoClass::AI_Update head)
    sim.entity_mut(id).ai_tick += 1;     // +0xC4  (D4 step 21) — BEFORE dispatch
    mission::dispatch(sim, id);          // Mission_Dispatch  (D2/D4 step 22)
    if !sim.is_alive(id) { return; }     // early-return: died in mission (D4 step 27)
    foot_locomotor_step(sim, id);        // ILocomotion::Process AFTER dispatch (D6)
    techno_common_post(sim, id);         // D4 steps 23–42 (accumulator, cloak, EMP…)
    unit_post(sim, id);                  // TurretAI → Fire → Facing → HarvestBrain
                                         //   → Anim/Ammo(+0x424) → Spawn → auto-hunt
}
```

Key behavior-bearing orderings the shell MUST preserve (all D6 HIGH, all default-DRIFT against the current Rust split):

- **Fire-then-Facing, same pass** (`UnitClass::Fire_At_Target` then `Facing_Update`): fire reads the *previous-tick* facing; a single target order cannot both start rotation and fire on the same pass. Current Rust splits fire and turret rotation into two global sweeps (`src/sim/combat/mod.rs:1174..1549`, `src/sim/movement/turret.rs:82..95`, `world/mod.rs:1778`) — **DRIFT** until folded into `unit_post`.
- **Unit post-Foot order** is Fire → Facing → HarvestBrain → Anim/Ammo (`vtable+0x424`) → SpawnManager (D6 correction: the ammo/anim wrapper sits *after* the harvest brain, not immediately after facing).
- **Mission_Dispatch is the boundary**: everything in `techno_common_pre` is pre-mission Techno-common work; everything in `techno_common_post` is post-mission. Three early-return death/EMP points (D4 steps 12 self-heal-death, 27 IsAlive, 42 building EMP-restore).
- **Timer-cluster accumulator** (`+0xf8 += +0x110`, the miner unload accumulator) runs in `techno_common_post` (D4 step 38), **after** Mission_Dispatch and **units-only** (buildings RTTI 6 skip it). Mission_Deploy state-3 samples `+0xf8` *during* dispatch, i.e. before this tick's increment — verify `src/sim/miner/*` increments after sampling, not before (D4 DRIFT-RISK).

`infantry_ai`/`aircraft_ai` follow the same skeleton with their verified specifics (infantry: death-force sequence, `Mission_Capture`, `Fear_Decay_Handler`, `DoType_Sequencer` self-Destroy — 3 self-removal exits; aircraft: mission one-shot byte clear, crash descent, map-bounds strafe kill, Carryall position sync — D6). `building_ai` wraps the common step in its 27-phase pipeline (`BuildingClass::Update 0x0043FB20`, AI_Update at phase 11 `0x0043FE36`) and is **explicitly NOT the first migration slice** (high blast radius into HouseClass/Factory; D6 §3.4). **UnitClass is the safest first leaf slice** (fully verified call order, strongly-proven fire-before-facing and locomotor-after-dispatch orderings, Rust already has separable movement/turret/combat phases to fold under it).

### 7.4 Consuming the existing substrate (no new owners)

The shell is a **pure consumer** of the four substrate services; every mutation routes through their APIs:

- **MissionCom (becoming authoritative, Slice 6).** `mission::dispatch(sim, id)` reads `MissionCom { current, queued, suspended, substate, timer, tick_counter }` and runs the frame-anchored gate: due iff `binary_frame - timer.start >= timer.duration` (D2 §2; `MissionTimer` is start+duration, **never decrements** — M5 timer rule, collision #10). The dispatch re-arms by writing the handler's returned frame count back into the timer (D2). The shell calls mission **verbs**, never raw field writes: `assign_mission` force-promotes (bypasses the gate, D2 §3); `queue_mission(commence=true)` is GATED — it consults the per-`EntityCategory` `ready_to_commence()` hook and skips promotion when false (base = `return 1`, four leaf overrides Building/Unit/Infantry/Aircraft — D2 §4, V2 CONFIRMED). A flat "commence always promotes" is **DRIFT** (still-driving unit / not-landed aircraft / not-ready building) and must wait for the Slice-6 per-type hook (M5 collision #3). `override_mission` saves the **queued** mission if one is pending, else the current, onto the single-depth suspend slot (D2 §3; a naive "always save current" is DRIFT). The shell does **not** add a parallel `mission_state:` field (M5 collision #9).

- **Contacts + RadioBus.** Docking/link steps in `foot_*` call the synchronous `transmit()`/`receive_radio()` and the `Contacts` slot model (first-null insert, null-hole removal with no compaction, sender self-evicts slot-0; capacity `max(NumberOfDocks,1)`). The shell **must not** introduce a dock wait-queue/FIFO — V3 proved gamemd has none (NEGATORY-when-full, distance-then-deterministic re-probe, receiver never evicts; M5 collision #7). `DockTeardown {All,Depot,AircraftOnly,IdleOnly,None}` is load-bearing for the 9 retask sites.

- **Presence + lifecycle.** Any death inside a shell step (deploy-countdown, garrison-terrain destroy, crash, sequencer death, zero-health) calls `substrate.uninit(id)` — detach-links → conceal → IsAlive=0 → **enqueue** to `pending_delete` (synchronous teardown, deferred slot-free; one-tick Dying window). The shell never frees synchronously and never touches `store.insert`/`logic.push`/`occupancy.add` (M5 collision #2, #6; invariant #6). `flush_pending_delete` stays at the cleanup phase, not inside the AI stage.

- **OccupancyGrid / Presence FSM.** Cell-registering eligibility is the `Type+0x234` class/category gate (no INI key — D-corpus); cell moves must route through the substrate (note: **no `move_cell` method exists today** — cell relocation currently lives in the movement tick; routing it through a substrate cell-move API is part of this work, §0.2). The shell reads occupancy for passive-scan/garrison/stuck-rescue but mutates only via the API.

### 7.5 Lockstep, RNG, and DRIFT discipline at the boundary

- **RNG position is load-bearing.** The only RNG consumed inside `techno_common` is the damage-fire particle pick (`Random__RandomRanged` ×2, D4 step 40), gated on ConditionYellow + `DamageParticleSystems` type + `+0x308==0`. The shell must consume RNG at the **same per-object position** under the **same gate** or the shared stream desyncs (D4 DRIFT-RISK). `Mission_Eaten` (CONDITIONAL, Yuri mind-control) and the Rescue handler also consume `Random__RandomRanged(0,2)` — preserve their call sites.
- **Do NOT add per-tick decrements** for IronCurtain/ForceShield/Temporal — those are passive (start-frame + duration compared on demand), AI_Update does **not** tick them (D4 DRIFT-RISK). Same rule for `MissionTimer` (frame-anchored).
- **Health smoothing** (`+0x70` lerp +1 on `frame&4` toward real Health, snaps down on damage, D4 step 7) is a per-object visual catch-up; confirm a `+0x70`-equivalent exists in the render health-bar path (DRIFT-RISK, render-side, out of `sim/`).
- **Frame-phased vs counter-phased:** `frame&4`/`frame&0xF`/`% Rules+0x30/0x38/0x314` are **global-frame**-phased (use `binary_frame`), while `+0xC4` is the per-object counter incremented but consumed mission-side (D4). Single time base = `binary_frame`, committed late (M5 invariant #5).

### 7.6 TS-legacy / dormant: representable, never live (D7, V2)

The shell must keep these **inert**, not implemented as live behavior:

| Behavior | Status | Shell rule |
|---|---|---|
| Tunnel/subterranean locomotor + Infantry tube sub-AI (`+0x684`/`g_TubeArray`) | TS_LEGACY / DORMANT | no tube branch; gate never fires (empty `g_TubeArray`) |
| Ambush mission 14 (`case 0xe → +0x20c`, stub `0x005B2E30` `return 0x1C2`) | TS_LEGACY | inert no-op enum variant; name round-trip only (V2: stub addr is `0x005B2E30`, not `0x005B2E10`) |
| Rescue mission 21 (`case 0x15 → +0x258`, real handler) | CONDITIONAL | LIVE but AI-only (`IsPlayerControl()==0`); include the handler, never player-assignable. The "ReceiveDamage-family FootClass assigner" is **UNVERIFIABLE** (V2) — do not bake a specific FootClass assigner into the design until traced |
| Eaten mission 9 (`case 9 → +0x218`, real handler `0x004D4CB0`) | CONDITIONAL / ACTIVE handler | LIVE for Yuri mind-control; the index-9 numbering is the TS enum-shift trap (Harvest=10, AreaGuard=11, Ambush=14), **not** a dead stub. Mission enum must match gamemd's shifted numbering, not the clean YRpp enum |
| AttackMove 29 (`case 0x1d` ABSENT → `default`) | CONDITIONAL/ACTIVE | representable selector; routes via **`default` → +0x204 (Sleep) + timer rewrite**, identical to QMove (**no dispatcher skip**); never *committed* — assign-side anti-churn keeps it off `+0xAC`. Verified `decompile_function 0x005B3060` |
| QMove 3 (no case → default `+0x204` Sleep) | ACTIVE_YR | routes to Sleep slot for all classes |
| RadioHistory `+0xD4/+0xD8/+0xDC` | DORMANT | write-only, no reader (V2 CONFIRMED) — **omit**; never branch gameplay on prior radio messages |
| WaypointQueue/NavQueue runtime push | DORMANT | no runtime producer; keep storage/readers for save-load tolerance only; do NOT reintroduce push on shift-move/AI-patrol |
| Fog-of-war "previously-seen" darkening / fog-border maintenance | TS_LEGACY (CONDITIONAL, `SpecialFlags & 0x1000`, default OFF) | shroud only; do NOT port `UpdateFogBorder` as default |
| Aircraft AreaGuard | TS_LEGACY | inherited 450-stub; never real |

The one D7 item the shell must honor: **FootClass `+0x694` drives a live sub-object AI every tick from the host** (`foot_*` tail, verified `decompile 0x004DA530`) — reproduce that dispatch, do NOT omit it. The pointed-to object's identity (likely an attached attacker — parasite/Terror-Drone or chrono — but **INFERRED/UNCHECKED**; `FOOTCLASS_COMPLETE §9.2` still unresolved) must be traced before binding it to a specific Rust system.

### 7.7 M5 consistency checklist (what this design does NOT change)

- One owner (`ObjectSubstrate`); the shell adds **no** second owner of lifecycle/presence/active-vector/mission/radio state.
- `advance_tick` phase order preserved; `object_ai_stage` slots into the existing AI phase only (no monolithic dispatch rewrite, no phase collapse — M5 invariant #2).
- Shadow-first discipline holds: the shell flips authority only as MissionCom/Presence flip (Slices 6/2→6), never ahead of them; hash must not move on a shadow slice.
- Dispatch is `match category` + capability flags + `Option<T>` components — never a trait/`dyn` class tree (invariant #3, M5 collision #1).
- State-hash remains a determinism oracle, not a gamemd-parity oracle; any new gamemd-matching behavior needs a gamemd-side evidence artifact before re-baselining a golden (M5 invariant #8).

**Open items carried into implementation (do not assert as settled):** leaf `ready_to_commence()` busy-flag byte semantics are INFERRED from constructor init, not traced setters — **DRIFT until traced** before the Slice-6 hook is field-accurate (D2 open question); the locomotor "idle" predicate (`loco slot+0x80`) consumed by Unit/Infantry `ready_to_commence` is **UNCHECKED**; the FootClass-side Rescue assigner is **UNVERIFIABLE** this pass (V2); whether `object_ai_stage` same-pass migration lands inside the object substrate or as a separate native-tick-spine contract is still open (M5).

## 8. Ad-hoc Rust Logic to Retire / Absorb

Deliverable #7. The keep/retire ledger for every current global sim sweep. Verdict vocabulary: **ABSORB-INTO-OBJECT-AI** (the sweep's behavior must move under a per-object `TechnoClass::AI_Update`-equivalent shell, sited explicitly relative to the verified `+0xC4` increment → `Mission_Dispatch` → locomotor `Process` spine), **KEEP-AS-GLOBAL-SERVICE** (stays a pre-/post-object bracket in the LogicClass-equivalent tick spine), or **NEEDS-PROOF** (verdict blocked on an unresolved binding/ordering from the verify lane).

**Hard precondition on every ABSORB.** Nothing moves until an acceptance test pins the order. The single proven per-object spine is, top→bottom of `TechnoClass::AI_Update` (`0x006F9E50`, this session): pre-mission common work (steps 1–20) → **`+0xC4` increment (per-object AI-tick counter, NOT `g_CurrentFrameCounter`)** → **`MissionClass::Mission_Dispatch` (`0x005B3060`, call site `0x006FA655`)** → post-mission common work (steps 23–42), with three EARLY-RETURN death points (self-heal step 12, IsAlive step 27, building EMP-restore step 42). For Foot leaves the locomotor `ILocomotion::Process` (vtable `+0x40`) runs inside `FootClass::AI` (`0x004DA530`) **after** `Mission_Dispatch`. The leaf shells wrap this: `UnitClass::AI` (`0x007360C0`) post-Foot order is **Fire_At_Target → Facing_Update → HarvestBrain → Anim/Ammo (`vtable+0x424`) → SpawnManager → auto-hunt**. These orderings ARE the acceptance test; any absorb that cannot reproduce its slot in this spine is **NEEDS-PROOF**, not ABSORB.

### Keep/Retire ledger

| # | Current global sweep | Verdict | Target site in the per-object spine (or bracket) | Evidence / caveat |
|---|---|---|---|---|
| 1 | Ground movement phase (`world/mod.rs` Phase 1, frozen `live_object_order_snapshot` at `:1741`) | **ABSORB-INTO-OBJECT-AI** | Locomotor `Process` (`vtable+0x40`) inside `FootClass::AI` (`0x004DA530`), **after** `Mission_Dispatch` — not a leading global phase | DRIFT (ordering): native runs movement after mission dispatch per object; Rust runs it as the first global phase before mission work. C9 caveat: AI/update stage wants same-pass re-read, movement phase may keep a per-phase snapshot. |
| 2 | Air + special movement (teleport/tunnel/rocket/droppod) phase | **ABSORB-INTO-OBJECT-AI** (special-loco subset DORMANT/TS_LEGACY) | Same locomotor `Process` slot, per leaf (`AircraftClass::AI` `0x00414BB0`) | Tunnel/Mech/DropPod locomotors are TS_LEGACY, never instantiated (zero INI `Locomotor=` refs; Tunnel `0x00728A00`, Mech `0x005AFEF0`, DropPod `0x004B5AB0`) — do NOT absorb those branches as live; absorb only the active drive/hover/fly/jumpjet/teleport Process. |
| 3 | Turret rotation (`movement/turret.rs:82..95`, post-combat sweep at `world/mod.rs:1778`) | **ABSORB-INTO-OBJECT-AI** | `UnitClass::Facing_Update` (`0x00736990`) — immediately **after** `Fire_At_Target`, inside `UnitClass::AI` IsAlive guard | DRIFT (ordering): Fire-then-Facing is verified per-object-coupled (fire reads previous-tick facing); Rust splits fire and facing across two global phases. Gate: turreted (`TechnoType+0xd2f` TurretNotHidden, `+0xd30==0` !TurretLocked). |
| 4 | Combat fire / facing (`combat/mod.rs:1174..1549`; attacker snapshot+sort `world/mod.rs:1732..1777`) | **ABSORB-INTO-OBJECT-AI** | `UnitClass::Fire_At_Target` (`0x00736DF0`) inside `UnitClass::AI`, **before** Facing_Update; for infantry `InfantryClass::Fire_At_Target` (`0x005206b0`) | DRIFT (ordering): native fires per-object in active-vector order then rotates; the global snapshot+sort is a substituted ordering — RNG-consuming damage paths must land at the same per-object position. Couples with #3 (fire→facing single pass). |
| 5 | Aircraft missions (global aircraft sweep `aircraft/mod.rs:144..183`) | **ABSORB-INTO-OBJECT-AI** | `MissionClass::Mission_Dispatch` (`0x005B3060`) reached under `FootClass::AI` from `AircraftClass::AI`; state machines live in dispatched handlers (vtable `+0x210..+0x270`), NOT in the AI body | DRIFT: missions are a global phase in Rust; native dispatches per-object. AircraftClass::AI itself is a thin shell (clears one mission one-shot byte, switch over `+0xAC`); the Attack/Move/Carryall/Paradrop state machines are mission handlers. |
| 6 | Retaliation + passengers (`world/mod.rs` Phase 6 slot `:2187/:2189`) | **KEEP-AS-GLOBAL-SERVICE** (post-combat bracket) | Stays a post-combat bracket; tick-order Phase 6 retaliation slot is invariant #2 (no phase collapse/reorder) | M5 invariant #2: advance_tick phase order is PRESERVED — the pre/post-combat order-intent split and Phase-6 retaliation slot stay put; only state representation + teardown call sites change. Retaliation assigner is the trigger surface (e.g. Rescue-21 issued from the ReceiveDamage family), but the bracket itself is a service. |
| 7 | Scatter (per-object idle scatter every 0x3f frames in `FootClass::AI`) | **ABSORB-INTO-OBJECT-AI** | Inside `FootClass::AI` (`0x004DA530`) idle-scatter block (every 0x3f frames), part of the Foot common body | Verified this session: FootClass::AI does idle-scatter every 0x3f frames. Frame-phased on `g_CurrentFrameCounter`, not the per-object `+0xC4` counter — preserve the global-frame phasing. |
| 8 | Miner FSM (`miner/*`, miner_dock_sequence) | **ABSORB-INTO-OBJECT-AI** (state machine) + **NEEDS-PROOF** (unload-accumulator ordering) | Harvest mission handler under `Mission_Dispatch` (`UnitClass` Harvest leaf `0x73E5E0`); the unload accumulator (`+0xf8 += +0x110`) runs **post-Mission_Dispatch** in `AI_Update` step 38 (bytes `0x006FABC4..0x006FAC2A`), units-only (buildings RTTI 6 skip it) | DRIFT-RISK: Mission_Deploy state-3 samples `+0xf8` DURING dispatch, before that tick's increment — so the accumulator must increment AFTER mission sampling. Cross-check `miner/*` against `TECHNOCLASS_AI_UPDATE_UNLOAD_ACCUMULATOR_ORDERING` doc before moving; ordering is NEEDS-PROOF until the acceptance test pins sample-before-increment. |
| 9 | Dock state machines (refinery/airfield docking) | **ABSORB-INTO-OBJECT-AI** (mission-side) + **retire FIFO** | Enter/Harvest/Unload mission handlers under `Mission_Dispatch`; admission via synchronous `transmit/receive_radio` RadioBus | V3 proven DRIFT: gamemd has NO stored dock wait-queue/FIFO — saturated dock replies NEGATORY to every HELLO, next docker wins by distance-then-deterministic re-probe, receiver never evicts, only a full sender self-evicts its own slot-0. **Retire** `RefineryDockContacts.waiting_retry_queue` + `AirfieldDocks.queues`. Do NOT design a dock wait-queue into the Foot docking shell. |
| 10 | Deploy (ConYard / unit deploy) | **ABSORB-INTO-OBJECT-AI** | Pre-Foot deploy-countdown block in `UnitClass::AI` (`0x007360C0`): timed-death self-destruct (`+0x1b6` vs `type+0xe38` DeathFrames), DeployCountdown decrement (`+0x1b0`), AI auto-deploy for ConYard types (`type+0x404`); Deploy_Building handler (`0x73D630`) under dispatch | Verified UnitClass::AI pre-Foot order. Timed-death and tube branches are SELF-REMOVAL exits — must enqueue to `pending_delete` (one-tick Dying window), NOT immediate-free. Tube branch (`-1 < (char)+0x1a1`) is DORMANT/TS_LEGACY (empty `g_TubeArray` in stock YR). |
| 11 | Infantry fear / prone / panic | **ABSORB-INTO-OBJECT-AI** | `InfantryClass::Fear_Decay_Handler` (`0x005200b0`) inside `InfantryClass::AI` (`0x0051BAB0`), after FootClass::AI, before `Fire_At_Target`; then DoType sequencer (`0x00520AE0`) | ACTIVE_YR (thresholds 49/50/199). **Partially implemented** in Rust (`InfantryRuntime{fear_level,is_prone}` `game_entity.rs:48` + `tick_fear_for_entities` `infantry.rs:130`, called `world/mod.rs:1960`) — absorb the existing tick into the leaf shell and **prove gamemd parity** (thresholds, prone/crawl-fire, sequencer self-Destroy); NEEDS-PROOF, not implement-from-scratch. Infantry has the most self-removal exits (death-force seq, garrison-terrain destroy, sequencer death completion) — each must route through deferred delete. |

### Bracket services that explicitly STAY global (not in the table above, for completeness)

These are the LogicClass-equivalent pre/post brackets the object shell sits between; they are **KEEP-AS-GLOBAL-SERVICE** by invariant #2 and are not absorbed: commands intake, vision, power, production/repairs/ore-growth, defeat detection, building-anim + cleanup (`flush_pending_delete` at the cleanup phase reproducing the one-tick Dying window), and state hash. `BuildingClass::Update` (`0x0043FB20`) is the wrinkle — it wraps `AI_Update` (phase 11, `0x0043FE36`) in 26 building-specific phases (power transitions, ProduceCash, gates, delayed fire, auto-sell, repair, auto-production, bridge destruction, zero-health destruction). Per the D6 migration rationale, **Buildings are NOT the first absorb slice** (high blast radius into HouseClass/Factory globals); **UnitClass is the safest first leaf** (fully verified shell order, separable existing Rust phases).

### Cross-cutting absorb constraints (apply to every ABSORB row)

- **No C++ class tree.** Absorbing into "object AI" means a per-object body dispatched by `match category` + `CapabilityFlags` + `Option<T>` components — NOT an `AbstractClass/ObjectClass/TechnoClass/FootClass` trait/vtable/`dyn` hierarchy (M5 invariant #3; design §6 "no vtables/COM/dyn"). `EntityCategory` stays in `map/entities.rs`, consumed one-way.
- **Single presence owner.** Any absorbed sweep that creates/removes/limbos entities routes through the `ObjectSubstrate` API only (`unlimbo/reveal/conceal/uninit/flush_pending_delete/change_owner` — a `move_cell` API does not exist yet, §0.2); no absorbed body touches `store.insert`/`logic.push`/`occupancy.add` (M5 invariant #6).
- **Deferred death.** Every SELF-REMOVAL exit inside an absorbed body (timed-death #10, garrison/terrain/sequencer #11, sinking/crash/bounds-kill for #2/#5) enqueues to `pending_delete` with synchronous conceal/unmark/detach but deferred slot-free — never an immediate `drop` (M5 constraint #6 / C7).
- **Timers are frame-anchored.** Any countdown an absorbed body owns is a `MissionTimer` `(start_frame, duration)` delta gate against `sim.binary_frame`, never a per-tick decrement (M5 timer constraint; Mission_Dispatch gate is `g_CurrentFrameCounter - +0xC8 >= +0xD0`). IronCurtain/ForceShield/Temporal are passive checked-on-demand timers — do NOT absorb them as per-tick decrements (DRIFT-RISK per D4).
- **RNG position.** The only RNG-consuming site inside `AI_Update` proper is the damage-fire particle spawn (step 40, `Random__RandomRanged` ×2, gated ConditionYellow + DamageParticleSystems + `+0x308==0`); an absorbed combat/particle path must consume RNG at the same per-object position in active-vector order or lockstep desyncs (DRIFT-RISK per D4).

### NEEDS-PROOF items carried forward (block the absorb until resolved)

- **#8 unload-accumulator ordering** — the gamemd ordering itself is verified (the `+0xf8 += +0x110` accumulator and the Deploy state-3 sample both sit inside the `0x006F9E50` decompile); what is NEEDS-PROOF is the **Rust-side** parity: an acceptance test must pin that the Rust miner reads `+0xf8` in Deploy state-3 *before* the post-`Mission_Dispatch` increment, and stays units-only.
- **Leaf `ReadyToCommence` busy-flag byte semantics** (`+0x6DD` building / `+0x6D2`/`+0x6D4` aircraft / `+0x6E1`/`+0x6E2`/`+0x6D1`/`+0x68D`/`+0x8D` unit/infantry) are INFERRED from constructor init, not from decompiled setters — DRIFT until each setter is traced; the per-category commence gate (Slice 6) cannot be field-accurate until then.
- **Locomotor `vtable+0x80` idle predicate** (consumed by Unit/Infantry `ReadyToCommence`) was not decompiled — exact idle semantic UNCHECKED; blocks proving the commence gate that orders queued-mission promotion.
- **FootClass Rescue (21) assigner** — the verify lane marked the "AI-only via ReceiveDamage family" claim **UNVERIFIABLE** this session (the inspected `6A 15` site was a radio-command-0x15 dock-unload transmit via `vtable+0x274`, NOT a mission-21 assign). The Rescue handler/slot is CONFIRMED live, but the retaliation-bracket trigger that issues it is unproven — do not bind #6's Rescue trigger to a FootClass assigner until traced (e.g. via `TechnoClass::ReceiveDamage 0x00701900`).

## 9. Migration Slices & Acceptance Tests

This section extends the in-flight object/mission/radio substrate program (object-substrate Slices 1–2/6 landed; mission/radio Slices 0–3 landed per `git log` `d41352b7`/`792d6051`/`ff1d2a32`/`6943e8ed`). The shell migration is **additive on top of those slices**, not a parallel track. Hard constraints carried from the M5 corpus and design invariants:

- **`advance_tick` phase order is PRESERVED** until a slice *explicitly* changes it (invariant #2; current order at `world/mod.rs` commands → ground move → air/special move → vision → power → turrets+combat → retaliation+passengers → scatter+production+repairs+docks+ore → AI → defeat → building anims+cleanup(`flush_pending_delete`) → state hash).
- **Shadow-first** (invariant #4): each behavior-bearing slice lands first as a shadow that `debug_assert`s agreement with the existing machine and is **not hashed** (no `state_hash` movement; prove with a no-change test) before authority flips.
- **No new owner of presence/lifecycle/active-vector state** (M5 collision #2): the shell routes membership through `ObjectSubstrate` only.
- **No C++ class tree / no `dyn`/vtable** (invariant #3, M5 collision #1): dispatch stays `match category` + capability flags + `Option<T>`.
- **State-hash is a self-replay determinism oracle, not a gamemd-parity oracle** (invariant #8): every slice that claims a *new* gamemd-matching ordering needs a gamemd-side evidence artifact (cited address) before its golden is baselined.

The boundary the slices converge on is the verified per-object shell shape from lane D6: `<Leaf>::AI → FootClass::AI (0x004DA530) → TechnoClass::AI_Update (0x006F9E50) → MissionClass::Mission_Dispatch (0x005B3060)`, with the locomotor `ILocomotion::Process` (vtable+0x40) running **after** mission dispatch inside `FootClass::AI`, and the `+0xC4` per-object tick counter incremented immediately before dispatch (lane D4 step 21–22, call site `0x006FA655`). UnitClass is the safest first leaf (D6 §3.4); buildings are last.

---

### Slice S0 — Instrumented no-op shell (order/membership preservation harness)

**Goal.** Stand up a per-object `techno_ai_step(id)` shell that is called from the existing AI phase and currently does **nothing but call the same work the phase already does, in the same order**, plus assert invariants. No behavior change, no hash change. This is the harness that every later slice mutates one ordering at a time.

**Files/surfaces.** `src/sim/world/mod.rs` (AI phase ~`:1741`+, `for_each_live_object` at `:911`); a new `src/sim/world/techno_ai.rs` (shell entry, instrumentation only). Reuses `ObjectSubstrate` API; touches no presence mutators.

**Becomes authoritative.** Nothing. The shell is a pass-through. Membership iteration still uses the existing per-phase snapshot (`live_object_order_snapshot()` at `:1741`); the shell does **not** yet adopt the re-read model (that is the C9 caveat — deferred, AI/update-stage-only, and not part of S0).

**Parity risk.** Near-zero — the only risk is accidentally reordering the work it wraps. Mitigation: the shell calls the *identical* phase bodies; the slice is rejected if `state_hash` moves on a replay.

**Acceptance tests.**
- `techno_ai_shell_is_passthrough_no_hash_change` — full-replay golden over a fixed skirmish seed; `state_hash` per tick bit-identical to pre-S0.
- `techno_ai_shell_membership_matches_phase_snapshot` — the set of ids the shell visits per tick equals the existing AI-phase visited set, in the same order.
- `techno_ai_shell_preserves_advance_tick_phase_order` — assert the phase sequence around the shell is unchanged (commands→…→AI→…→cleanup→hash).

---

### Slice S1 — First behavior-bearing ordering: locomotor-Process-after-mission-dispatch (one UnitClass scenario, shadow)

**Goal.** Land the verified D6/D4 ordering for **one narrow UnitClass scenario**: mission dispatch runs, *then* the locomotor processes movement, within the same per-object pass — as a **shadow** that asserts agreement with the current phase-split movement-then-mission ordering, before any flip. Scope: a single moving UnitClass executing `Mission_Move`/`Mission_Guard` with no combat, no docking.

**Files/surfaces.** `src/sim/world/techno_ai.rs`, `src/sim/movement/` (locomotor process entry), `src/sim/world/mod.rs` (Phase 1 ground-movement vs AI-phase relationship — **read-only at this slice**, shadow compares but does not move the phase).

**Becomes authoritative.** Nothing yet — shadow only. The shell computes the would-be post-dispatch locomotor result and `debug_assert`s it matches the current tick's actual movement output for the in-scope scenario. Not hashed.

**Parity risk.** This is the highest-leverage ordering in the whole migration and a known DRIFT (D4 rust_delta: "no single per-object AI_Update shell … movement (Phase 1) before aircraft missions/combat"). Risk is that flipping it later changes *when* a freshly-dispatched `Mission_Move` first advances position — a 1-tick movement-start slip that is player-visible. Keeping S1 as a shadow surfaces the divergence count before committing.

**Acceptance tests.**
- `unit_ai_mission_dispatch_precedes_locomotor_process` — for the scoped UnitClass, dispatch is observed to run before the locomotor `Process` step in the shadow trace.
- `unit_move_dispatch_then_process_shadow_agrees` — over the scoped scenario the shadow's post-dispatch movement matches the live phase-split result every tick (zero `debug_assert` failures); if it diverges, the divergence is logged with tick + id, **not** silently equalized.
- `s1_no_hash_change_shadow` — `state_hash` unmoved (shadow not hashed).

---

### Slice S2 — Flip the UnitClass locomotor/dispatch ordering authoritative (scoped), establish the +0xC4 increment point

**Goal.** Promote the S1 ordering to authoritative **for the scoped UnitClass path only**: the shell increments the per-object tick counter (`+0xC4` analogue, D4 step 21), calls `Mission_Dispatch`, then runs the locomotor `Process`. This is the first slice that *explicitly* changes the relationship between the ground-movement work and the AI phase for those units, so it carries a `SNAPSHOT_VERSION` bump and a fresh golden.

**Files/surfaces.** `src/sim/world/techno_ai.rs`, `src/sim/world/mod.rs` (route scoped UnitClass movement through the shell instead of Phase 1), `src/sim/movement/`. Retire candidate begins here: the global per-unit movement step for in-scope units (D6 retire list — global combat/turret/movement sweeps eventually fold under the shell).

**Becomes authoritative.** The per-object dispatch→process ordering and the `+0xC4` increment-before-dispatch for scoped UnitClass entities.

**Parity risk.** The 1-tick movement-start timing established in S1 now affects the hash. Because the hash is only a self-replay oracle (invariant #8), the new golden must be justified by the cited gamemd evidence (`FootClass::AI 0x004DA530` runs `Process` after `TechnoClass::AI_Update`→`Mission_Dispatch`; `+0xC4` increment at the call site preceding `0x006FA655`). Out-of-scope units still use the old phase ordering, so cross-interaction (a scoped unit and an unscoped unit colliding for the same cell) is a watch item — test it explicitly.

**Acceptance tests.**
- `unit_move_start_slip_matches_dispatch_then_process` — a freshly-ordered Move advances position on the tick predicted by dispatch-then-process, not the prior phase-split tick.
- `unit_c4_counter_increments_before_dispatch` — the per-object counter is incremented immediately before `Mission_Dispatch` is invoked (D4 step 21→22).
- `scoped_vs_unscoped_unit_cell_contention_deterministic` — a scoped and an unscoped unit racing for one cell resolve deterministically and identically across replays.
- `s2_snapshot_version_bumped_golden_rebaselined` — golden regenerated; replay determinism holds.

---

### Slice S3 — Post-Foot UnitClass ordering: Fire → Facing → HarvestBrain → Anim/Ammo → Spawn

**Goal.** Reproduce the verified UnitClass post-Foot sequence (D6 §UnitClass step 7, **corrected** order: `Fire_At_Target` → `Facing_Update` → `HarvestBrain_Idle` → ammo/anim wrapper `vtable+0x424` → SpawnManager — note the ammo/anim wrapper sits *after* the harvest brain, not immediately after facing; the lane brief's "fire-then-facing-then-ammo" is this fuller order). Land as shadow, then flip. **Fire-before-Facing is load-bearing**: fire reads the previous-tick facing, so a single target order cannot both start rotation and fire on the same pass.

**Files/surfaces.** `src/sim/world/techno_ai.rs`, `src/sim/combat/mod.rs` (`:1174..1549` global attacker snapshot — retire candidate, D6), `src/sim/movement/turret.rs` (`:82..95` global turret sweep — retire candidate, native owner `UnitClass::Facing_Update 0x00736990`), `src/sim/world/mod.rs` (`:1778` turret-rotation sweep site).

**Becomes authoritative.** Per-object Fire→Facing coupling for scoped UnitClass; the global combat-attacker snapshot and the global turret-rotation sweep are removed for in-scope units (they remain for out-of-scope categories until their slices land).

**Parity risk.** Current Rust **splits fire and facing across two global phases** (D6 rust_delta: combat at `world/mod.rs:1760..1783`, turret in `movement/turret.rs`) — coupling them per-object with fire-first is a confirmed DRIFT fix and **will** change turret-vs-fire interleave on the tick a target is first acquired. This is player-visible (turret lag / first-shot timing) and hash-affecting → `SNAPSHOT_VERSION` bump, gamemd-evidence-backed golden.

**Acceptance tests.**
- `fire_then_facing_then_ammo_order` — the scoped unit's post-Foot calls occur in the order Fire→Facing→(Harvest)→Anim/Ammo→Spawn.
- `unit_fire_reads_previous_tick_facing` — on the tick a target is first assigned, the unit fires using last-tick facing and rotation only *begins* that tick (no same-tick rotate-and-fire).
- `harvest_brain_between_facing_and_ammo` — for a War Miner, `HarvestBrain_Idle` runs after Facing and before the ammo/anim wrapper.
- `turret_sweep_retired_for_scoped_units_no_drift` — removing the global turret sweep for scoped units leaves out-of-scope turret behavior bit-identical.

---

### Slice S4 — TechnoClass common pre/post-mission work + the three early-return death points (units)

**Goal.** Fold the verified `TechnoClass::AI_Update` common body (lane D4) into the shell for UnitClass: pre-mission block (steps 1–20) → `+0xC4` increment → `Mission_Dispatch` → post-mission block (steps 23–42), honoring the **three EARLY-RETURN points** (D4 step 12 self-heal death, step 27 IsAlive, step 42 building-EMP-restore — the third is building-only and deferred). Two lockstep-sensitive items land here precisely:

1. **Damage-fire particle RNG** (D4 step 40): consumes `Random__RandomRanged` ×2, gated on `HealthRatio < ConditionYellow` + `DamageParticleSystems` type + `+0x308` empty. Must consume RNG at the **same per-object position** or the shared stream desyncs (D4 rust_delta DRIFT-RISK).
2. **Health visual smoothing** (D4 step 7, `+0x70` lerp +1/`frame&4`-qualifying tick toward `Health`, instant snap-down on damage) — this is render-side and **must not be hashed**; verify a `+0x70`-equivalent exists or is added on the render path only.

Passive/opportunity acquisition (D4 step 23, missions **{2,10,5}** only, `+0x180/+0x188` 45-frame scan timer, `CanPassiveAcquire`+`OpportunityFire`+0x6AF gate) lands here as a shadow — it is a known missing system (D4 + D6 rust_delta: Grizzly/War-Miner opportunity fire absent) and needs `OpportunityFire`/`CanPassiveAcquire`/`CanRetaliate` parsed first.

**Files/surfaces.** `src/sim/world/techno_ai.rs`, `src/sim/combat/` (passive scan), RNG-stream plumbing, `map/entities.rs`/rules parse for the new INI keys (one-way map→sim), render-side health-bar path for `+0x70`.

**Becomes authoritative.** The pre/post-mission common-work ordering and the death early-returns for units; damage-particle RNG position. Passive-acquire stays shadow until S5.

**Parity risk.** RNG position is the dangerous one — wrong placement is a full-match lockstep desync, not a cosmetic drift. Default DRIFT; the gate must match exactly (`+0x308==0` + ConditionYellow + `DamageParticleSystems`). Iron-curtain/force-shield/temporal timers must **not** be added as per-tick decrements (D4 rust_delta DRIFT-RISK; they are passive `CurrentFrame-Start<Duration` checks) — this slice asserts the absence.

**Acceptance tests.**
- `techno_ai_pre_then_dispatch_then_post_order` — the shell runs pre-mission → `+0xC4` → dispatch → post-mission for units.
- `damage_particle_rng_consumed_at_native_position` — RNG draw count and position per tick match the gamemd gate; a unit that is *not* below ConditionYellow consumes zero draws.
- `health_smoothing_not_hashed_render_only` — `+0x70` smoothing changes never affect `state_hash`; snap-down-on-damage + lerp-up-on-`frame&4` verified on the render value only.
- `iron_curtain_temporal_timers_not_decremented_per_tick` — assert no per-tick countdown; on-demand `frame - start < duration` only.
- `passive_acquire_only_missions_2_10_5_shadow` — shadow scan fires only for missions 2/10/5 with the 45-frame `+0x180/+0x188` cadence; zero divergence asserted before any flip.

---

### Slice S5 — Align with mission/radio Slice 6 (MissionCom authority + ReadyToCommence) handoff

**Goal.** This is the explicit handoff to the mission/radio program's **Slice 6** (MissionCom becomes authoritative; verb API `assign_mission`/`queue_mission`/`commence_queued`/`override_mission`/`restore_mission`/`get_current_mission`/`is_busy` + per-`EntityCategory` `ready_to_commence()` hook). The Techno shell stops reading the shadow `mission_com` and reads the authoritative `mission` field; the commence gate is enforced through the per-category hook. Passive-acquire (S4 shadow) flips authoritative here, since it is keyed on the now-authoritative mission selector.

**Verified contract this slice must honor (lanes D2/V2):**
- `Queue_Mission` (`0x005B35E0`) **consults `ReadyToCommence` (+0x200)** and *skips* `Commence` when false; `Assign_Mission` (`0x005B2FD0`) **force-promotes, ignoring it** (D2 §4, V2 CONFIRMED). A flat "commence always promotes" Rust verb is DRIFT.
- Base `ReadyToCommence` = `return 1` (`0x004E0140`); **all four leaves override** (Building `0x00454250`, Unit `0x00744270`, Infantry `0x00521B60`, Aircraft `0x0041B5E0`) — V2 CONFIRMED these are real predicates, not stubs. Implement four `ready_to_commence()` impls via `match category`, **not** a vtable.
- **Override/Restore is a single-depth suspend slot (`+0xB0`)**; Override saves the **queued** mission if one is pending, else the **current** mission (D2 §3, rust_delta gap — "always save current" is wrong on the suspend/restore round-trip).
- `MissionTimer` is **frame-anchored** (`(start,duration)` delta gate, never decrements; sentinel `u32::MAX`; base `sim.binary_frame`) — D2 rust_delta DRIFT-RISK: a per-tick decrement drifts on save/load and variable-rate paths.
- `MissionControl` is a **32-slot** table, stride confirmed; `AARate==0` copies `Rate` (D2 §5). **Do not size from `MISSIONCLASS_STATE_MACHINE.md`** — its "8 bytes per entry" is wrong (true byte stride `0x20`; the doc figure is 8 *dwords*). This is a flagged retire-candidate (`MISSIONCLASS_STATE_MACHINE.md:341-373,546`).
- **Busy-flag byte semantics inside the leaf `ReadyToCommence` predicates remain INFERRED → DRIFT** (D2 open question: `+0x6DD` building; `+0x6D2/+0x6D4` aircraft; unit/infantry busy bytes from constructor init, *not* decompiled setters). Land the hook structure now; mark the busy-flag field reads UNCHECKED and trace each setter before claiming field-accuracy.

**Mission-status facts the shell must encode (D2 §7 / D7 / V2):**
- **Rescue (21)** — CONDITIONAL, **AI-only** (`IsPlayerControl()==0`), real handlers (`FootClass 0x004DDF90`, `Aircraft 0x00415960`); include an AI threat-response handler. The specific FootClass live assigner via the ReceiveDamage family is **UNVERIFIABLE per V2** — do **not** assert a FootClass-side Rescue assigner as fact; gate the handler's existence on the confirmed slot, and treat the assigner path as UNCHECKED pending a `TechnoClass::ReceiveDamage 0x00701900` trace.
- **Ambush (14)** — TS_LEGACY dead 450-frame stub (slot `+0x20c` → `0x005B2E30 = return 0x1C2`, **not** `0x005B2E10`, per V2 correction); model as inert no-op, name round-trip only. Do **not** implement real behavior.
- **Eaten (9)** — CONDITIONAL (Yuri slave/clone); the handler `0x004D4CB0` is **real, not a stub** (D7 §6). The "Eaten=9" concern is an **enum index-shift trap**, not a dead handler: gamemd retains TS `Eaten` at index 9, so mission codes at/after Harvest are +1 vs the "clean" YRpp enum (Harvest=10, AreaGuard=11, Ambush=14). The string-table off-by-one ("table[15]=Ambush") is **refuted** — `0x00816CAC` entry[14]="Ambush" is correct (D2 §6, retire-candidate `MISSION_RADIO_SUBSTRATE_BINARY_VERIFICATIONS.md:8,87`). The Rust mission enum must match gamemd's shifted numbering wherever it cross-references binary/INI codes (D7 rust_delta UNCHECKED).
- **AttackMove (29)** — representable selector; `case 0x1d` absent → hits the **explicit `default` → +0x204 (Sleep) + timer rewrite**, identical to QMove (**not** a silent fall-through, **no** dispatcher skip). Never *committed* — assign-side anti-churn keeps it off `+0xAC`; do not synthesize a handler. Verified `decompile_function 0x005B3060`.
- **QMove (3)** — routes to the **Sleep** slot (`+0x204`) for all classes via `default` (V5/D2).
- **No dock/radio wait-queue** (M5 V3 DRIFT): a saturated refinery/airfield replies NEGATORY; next docker wins by distance-then-deterministic re-probe; receiver never evicts, only a full sender self-evicts slot-0. A FootClass docking shell must **not** add a FIFO.
- **RadioHistory** (`+0xD4/D8/DC`) is write-only, no reader (D7 §7, V2 CONFIRMED) → **omit**; never branch gameplay on prior radio messages.

**Files/surfaces.** `src/sim/mission/{mod,timer,control}.rs`, `src/sim/radio/{mod,contacts,receive}.rs`, `src/sim/world/techno_ai.rs`, `src/sim/game_entity.rs` (rename shadow `mission_com`→authoritative `mission`, type-swap already done for `radio_contacts: Contacts`).

**Becomes authoritative.** `MissionCom` (mission selector + frame-anchored timer + substate), the four `ready_to_commence()` category hooks, Override/Restore single-depth suspend, the passive-acquire mission gate.

**Parity risk.** The commence-gate divergence is real and player-visible: without the per-type hook, queue+commence-now to a still-driving unit / not-landed aircraft / not-ready building promotes one tick early in the port and silently fails to promote in gamemd (D2 rust_delta). The busy-flag INFERRED fields are a live DRIFT until setters are traced — do not baseline a golden that depends on their exact values.

**Acceptance tests.**
- `queue_commence_gated_by_ready_to_commence` — `queue_mission(commence=true)` to a unit whose `ready_to_commence()` is false does **not** promote that tick; `assign_mission` to the same unit **does** (force-promote).
- `ready_to_commence_base_returns_true_four_leaf_overrides` — base hook true; Building/Unit/Infantry/Aircraft each route through their own predicate via `match category`, no `dyn`.
- `override_saves_queued_when_pending_else_current` — with a queued mission pending, Override suspends the *queued* one; with none, it suspends the *current* one; Restore pops the single slot and returns false when empty.
- `mission_timer_frame_anchored_no_decrement` — a mission whose ticks are skipped (e.g. paused) re-arms on the same absolute frame; no per-tick countdown drift across save/load.
- `mission_control_32_slots_aarate_copies_rate` — table has 32 entries at stride `0x20`; an entry with `AARate` absent/0 reads back `AARate==Rate`.
- `rescue_21_ai_only_never_player_assigned` — a player-issued order never produces mission 21; an AI-owned unit can enter it; assigner path marked UNCHECKED in the test comment (V2).
- `ambush_14_inert_noop_name_roundtrip` — mission 14 dispatches to a no-op (~450-frame idle / next-dispatch); the name "Ambush" round-trips through the 32-entry table; no behavior.
- `eaten_9_real_handler_enum_shift_preserved` — Eaten(9) handler runs (not a stub); mission enum numbering is the shifted gamemd one (Harvest=10/AreaGuard=11/Ambush=14), verified against the name table.
- `attackmove_29_never_committed_routes_like_qmove` — mission 29 is never committed to `+0xAC` (assign-side anti-churn); there is **no** dispatcher skip — if 29 *were* dispatched it would route via the explicit `default` to the Sleep handler `+0x204` + timer rewrite, exactly like QMove(3). Verified `decompile_function 0x005B3060`.
- `qmove_3_routes_to_sleep_slot` — mission 3 dispatches via default to the Sleep handler for every category.
- `no_dock_wait_queue_negatory_reprobe` — a full refinery returns NEGATORY; the next docker is chosen by distance-then-deterministic order on re-probe; no FIFO state retained.
- `s5_mission_authority_flip_golden_rebaselined` — `SNAPSHOT_VERSION` bumped; replay determinism holds; busy-flag-dependent assertions explicitly excluded from the golden until setters are traced.

---

### Slice S6 — Infantry leaf shell

**Goal.** Bring InfantryClass under the shell (verified order, lane D6 `InfantryClass::AI 0x0051BAB0`): tube early-return (DORMANT — `+0x684`/`g_TubeArray`, TS-legacy, never reached in stock YR, assert-and-skip) → falling/warp → `FootClass::AI` → garrison-enter check → `Mission_Capture` (returns-consumes-tick) → `Fear_Decay_Handler` (0x005200B0, ACTIVE_YR) → `Fire_At_Target` → `DoType_Sequencer` (death-seq completion self-Destroys). Infantry has the **most self-removal exits** (death-force seq, garrison-terrain destroy, sequencer death completion) — all must enqueue to `pending_delete` (synchronous conceal/unmark, deferred free), never immediate-free (M5 collision #6).

**Files/surfaces.** `src/sim/world/techno_ai.rs`, `src/sim/combat/`, `src/sim/movement/`, infantry fear/prone state (currently absent — D6 rust_delta: no `FearLevel`, no prone trigger, `SequenceKind::Panic` never entered).

**Becomes authoritative.** Per-object Infantry AI ordering and self-removal-via-`pending_delete`. The fear/prone/panic system is a **new** behavior (DRIFT — entirely unimplemented per D6) and lands as shadow → flip with its own gamemd-evidence golden.

**Parity risk.** Fear/prone is net-new gameplay; multiple mid-pass self-removal points raise the risk of a unit being freed before a later consumer reads it (deferred-death window must hold). TS-legacy tube branch must be a no-op, not implemented.

**Acceptance tests.**
- `infantry_ai_order_capture_fear_fire_sequencer` — the verified call order is reproduced.
- `infantry_self_removal_enqueues_pending_delete` — each of the three destroy paths enqueues (conceal/unmark synchronous), with the one-tick Dying window preserved; no synchronous free.
- `infantry_tube_branch_is_noop_ts_legacy` — `+0x684`/`g_TubeArray` path is asserted unreachable in stock maps and does nothing.
- `infantry_fear_decay_thresholds` — FearLevel decays; prone/stand at the verified thresholds; panic scatter above threshold (shadow-validated before flip).

---

### Slice S7 — Aircraft leaf shell

**Goal.** Bring AircraftClass under the shell (lane D6 `AircraftClass::AI 0x00414BB0`): the AI body is a **thin shell that only clears a one-shot mission byte** — the real Attack/Move/Guard/Carryall/Paradrop state machines run in the **dispatched mission handlers** under `FootClass::AI` (vtable `+0x210..0x270`), **not** in the AI body (D6 rust_delta: current Rust runs a global aircraft mission sweep at `aircraft/mod.rs:144..183` — retire candidate, native owner `Mission_Dispatch 0x005B3060` under `FootClass::AI`). Honor crash-descent and map-bounds-strafe self-removal (both → `pending_delete`), and Carryall position sync (CONDITIONAL).

**Files/surfaces.** `src/sim/aircraft/mod.rs` (`:144..183` global sweep — retired here), `src/sim/world/techno_ai.rs`, mission handlers for aircraft slots.

**Becomes authoritative.** Per-object aircraft dispatch (replacing the global sweep); aircraft mission state machines move under mission dispatch.

**Parity risk.** Moving missions from a global snapshot sweep to per-object dispatch changes interleave with other aircraft and with ground units; aircraft has the most state-machine surface to relocate. Aircraft `+0x294` airstrike-owner radio-deaf latch is a back-pointer (M5 V2), not a bool — deferred detail, mark UNCHECKED if not traced. Aircraft never AreaGuard (inherited dead stub, D7 §11) — do not implement.

**Acceptance tests.**
- `aircraft_ai_body_is_thin_shell` — the AI body only clears the one-shot byte; no state-machine logic runs outside dispatch.
- `aircraft_missions_dispatched_not_global_sweep` — the global aircraft mission sweep is retired; missions run per-object under dispatch with identical observable behavior on a Paradrop/Carryall scenario.
- `aircraft_crash_and_bounds_kill_pending_delete` — crash-descent (<−400) and out-of-bounds strafe kill enqueue to `pending_delete`.
- `aircraft_never_areaguard_inherited_stub` — assigning AreaGuard to an aircraft is the inherited 450-stub no-op.

---

### Slice S8 — Building leaf shell (LAST)

**Goal.** Bring BuildingClass under the shell. Buildings are **last** by design (D6 §3.4): `BuildingClass::Update (0x0043FB20)` wraps `AI_Update` (phase 11, `0x0043FE36`) in **26 building-specific phases** (power-state transition + looping sound, damage-fire anims, occupant counter, docked-object update via `vtable+0x5c`, warp early-return, ProduceCash, power charge, SAM gate, anim state machine, turret-fire counter + ROF + burst, anim-slot change, health→sidebar redraw, **zero-health destruction** with destruction-delay timer + Limbo, delayed fire, overpower cleanup, auto-sell/civilian, repair+power AI, auto-production, bridge destruction, factory/transport gate). High blast radius into HouseClass/Factory/superweapon globals — the migration boundary deliberately keeps those as pre/post-LogicClass stages. Buildings **skip the timer-cluster unload accumulator** (D4 step 38, RTTI 6 → `goto`); assert units-only. The third early-return (building EMP-restore, D4 step 42) lands here.

**Files/surfaces.** `src/sim/world/techno_ai.rs`, `src/sim/world/mod.rs` (`:1704..2078` power/production/repair global phases — these remain separate stages; the shell owns only the per-building Update-bracket ordering around `AI_Update`, not the house/factory services). `ready_to_commence()` Building hook (`+0x6DD`, INFERRED — UNCHECKED until setter traced) finalized here.

**Becomes authoritative.** The per-building Update-bracket ordering around `AI_Update` and the building EMP-restore early-return. House/factory/superweapon services stay as their current global phases (no collapse — invariant #2).

**Parity risk.** Highest blast radius of any slice — touches power transitions, ProduceCash, auto-sell, repair, auto-production, bridge destruction, zero-health destruction. Any reorder risks cascading into HouseClass/Factory globals. Mitigation: shadow the entire 27-phase bracket first, flip only after a full-skirmish replay shows zero divergence; keep the global house/factory phases untouched. Fog-of-war darkening branches inside building gap/special-fx are **TS_LEGACY** (off by default) — do not implement.

**Acceptance tests.**
- `building_update_27_phase_bracket_order` — the verified phase sequence around `AI_Update` (phase 11) is reproduced; house/factory global phases unchanged.
- `building_skips_unload_accumulator_units_only` — buildings (RTTI 6) never run the `+0xf8`/`+0x110` accumulator; a co-located miner still does.
- `building_zero_health_destruction_pending_delete` — zero-health destruction runs OnDestroyed/SpawnSurvivors/Limbo and enqueues with the destruction-delay timer; IsAlive guard returns post-`AI_Update`.
- `building_emp_restore_early_return` — the EMP-lock-expiry restore path sets the online-effects flag and early-returns as the final block.
- `building_fog_darkening_not_implemented` — fog-of-war darkening branches are absent (TS_LEGACY, default off); only shroud is modeled.
- `s8_full_building_bracket_shadow_zero_divergence` — full-skirmish shadow replay shows zero `debug_assert` divergence before the authority flip; golden rebaselined with `SNAPSHOT_VERSION` bump.

---

### Slice ordering & dependency summary

| Slice | Leaf/scope | Authority flip | Hash-affecting | Depends on |
|---|---|---|---|---|
| S0 | all (passthrough) | none | no | object-substrate S1–2 |
| S1 | UnitClass (1 move scenario) | none (shadow) | no | S0 |
| S2 | UnitClass (scoped) | dispatch→process + `+0xC4` | yes | S1 |
| S3 | UnitClass | Fire→Facing→…→Spawn | yes | S2 |
| S4 | UnitClass | pre/post common + RNG pos | yes | S3 |
| S5 | all categories | MissionCom + ReadyToCommence | yes | S4 + mission/radio Slice 6 |
| S6 | InfantryClass | infantry AI order + fear | yes | S5 |
| S7 | AircraftClass | per-object dispatch | yes | S5 |
| S8 | BuildingClass | Update-bracket | yes | S5, S6, S7 |

Every authority flip (S2 onward) carries a `SNAPSHOT_VERSION` bump and a gamemd-evidence-cited golden (invariant #8); every shadow slice (S0, S1, the shadow phase of S4/S6) must prove zero `state_hash` movement before its flip. The unresolved busy-flag byte semantics (D2/V2 INFERRED — `+0x6DD`, `+0x6D2/+0x6D4`, unit/infantry busy bytes) and the FootClass Rescue assigner (V2 UNVERIFIABLE) remain **UNCHECKED** and must be traced from the binary before any golden depends on their values.

## 10. Open Questions, Risks & Negative Facts

This section consolidates the cross-lane open questions, the UNVERIFIABLE/WRONG verify verdicts, and the explicit do-not-do negative facts that constrain the Techno/Foot substrate design. Every item is grounded in the findings JSON; default verdict on any unproven equivalence is DRIFT.

### 10.1 Verify-lane verdicts that limit what may be asserted

- **UNVERIFIABLE — FootClass-side Rescue (21) assigner.** The "AI-only via the ReceiveDamage family" assigner for mission `0x15` on FootClass was *not* confirmed from the binary this session. The inspected `6A 15` (PUSH 0x15) site at `0x0051a29a` inside `InfantryClass::Per_Cell_Process @0x00519630` is a **radio transmit of command 0x15 (dock-unload) via `vtable+0x274`, NOT a mission-21 assign** — the radio-command-0x15 vs mission-0x15 conflation trap. The Rescue *handler* and *slot* are CONFIRMED LIVE (`Mission_Dispatch @0x005B3060` case `0x15` → `vtable+0x258`; FootClass `0x004ddf90`, Aircraft `0x00415960`); only the FootClass-side *assigner* is unproven. **Do NOT design a FootClass Rescue assigner on the ReceiveDamage assumption** until traced. The decode-lane D2 framing of `FUN_00708080` issuing `Queue_Mission(0x15,0)` gated `IsPlayerControl()==0` is itself doc-sourced (MISSION_RADIO_SUBSTRATE_BINARY_VERIFICATIONS.md V4), not re-decompiled — treat as CONDITIONAL pending the trace.
- **WRONG / address corrections to carry forward.** (1) Ambush(14) stub address is **`0x005B2E30`**, not `0x005B2E10`; `0x005B2E10` (Mission_Default) backs the Sleep slot `+0x204`. Both return `0x1C2` (450 frames) — same behavior, distinct address. (2) Per the M5 V5 correction, Aircraft `Mission_QMove 0x00415A50` is at slot **`+0x230` (Retreat)**, NOT `+0x204`; QMove(3) routes to the **Sleep** slot for all classes via the dispatch `default` case. (3) Convoy chain link fields `0x6C0–0x6D2` / follower list `+0x6C8` are **UnitClass**, not FootClass (CONVOY_FORMATION_SYSTEM doc was WRONG; corrected in FOOTCLASS_COMPLETE §8) — only `formation_speed +0x578` is a FootClass field.
- **Doc-sourced, not re-decompiled this session (treat as DRIFT, not equivalence).** All gamemd addresses cited from the M5 plan/design corpus — `0x0055AFB0`, `FUN_0055BAA0`/`FUN_0055BAE0`, `FUN_007C8D20`, the ReadyToCommence leaf slots, and the mission-handler slot map — are pre-accepted verifications quoted from docs, not independently re-decompiled. D4/D6 helper FUNs (`FUN_0070ed10` gattling, `FUN_004a5150`/`FUN_004a5360`, `FUN_00709290`) and several field-map labels (`+0x1cc/+0x1d0/+0x1d4` drain triad, `+0x5ed` Thief byte, `+0xc8f` DamageParticleSystems byte) carry label-only confidence — not fresh per-field xrefs.

### 10.2 Do-NOT-do negative facts (designer constraints)

- **Do NOT migrate ore/spread (or production/repair/scatter) under a per-object Techno AI shell.** advance_tick phase order is PRESERVED — no slice collapses or reorders phases; scatter+production+repairs+docks+ore stay in their existing global phase. Only state REPRESENTATION and teardown CALL SITES change; there is NO monolithic dispatch rewrite (M5 invariant #2).
- **Do NOT collapse the per-tick / PerTickUpdate global loops, nor hardcode one iteration model.** The choice is PER-PHASE: same-pass re-read for the AI/update stage (the `0x0055AFB0` consumer re-reads the live count, so a mid-tick spawn acts the same tick), frozen snapshot acceptable for the phase-split movement/combat passes. A shell that hardcodes one iteration model for all Techno updates contradicts the C9 caveat (design §6 line 198). Global frame-phased gates (`g_CurrentFrameCounter & 4` health smooth, `& 0xF` ally recheck, `% Rules+0x30/0x38/0x314` power/steal) are global-frame-phased, NOT per-object-counter phased — the per-object `+0xC4` counter is incremented pre-Mission_Dispatch but its consumers are mission-side.
- **Do NOT start the leaf migration with BuildingClass.** UnitClass is the safest first leaf (fully verified shell order: pre-Foot deploy/tube/warp → FootClass::AI → TurretAI → Fire→Facing→Harvest→Anim/Ammo→Spawn→auto-hunt; Fire-then-Facing and locomotor-after-mission-dispatch are the strongly-proven orderings). `BuildingClass::Update` wraps `AI_Update` (phase 11 @ `0x0043FE36`) in 26 building-specific phases (power transitions, ProduceCash, gates, delayed fire, auto-sell, repair, auto-production, bridge destruction, zero-health destruction) — high blast radius into HouseClass/Factory globals the boundary report keeps as pre/post-LogicClass stages. Infantry/Aircraft are intermediate (more self-removal exits; aircraft mission state machines must move under mission dispatch).
- **Do NOT implement TS-legacy / dormant behavior as live:**
  - **Ambush(14)** — inert 450-frame stub (`0x005B2E30`), no leaf override, no live assigner. Model as a no-op enum variant for name round-trip only. *TS_LEGACY.*
  - **Tunnel/subterranean** — TunnelLocomotionClass (`0x00728A00`), Mech (`0x005AFEF0`), DropPod (`0x004B5AB0`) never instantiated (zero/commented INI refs); InfantryClass tube sub-AI (`FUN_0051B350`, gate byte `+0x684 != 0xFF`) and Walk/Hover dir-code-8 branches are DORMANT (empty `g_TubeArray`). Do NOT implement. *TS_LEGACY / DORMANT.*
  - **Fog-of-war "previously seen" darkening / FoggedObjectClass / fog-border maintenance** (`MapClass__UpdateFogBorder` block in FootClass::AI ~`0x004DA6C0`) — gated `SpecialFlags & 0x1000` (FogOfWar=no default, rulesmd.ini:3040). Implement shroud only. *CONDITIONAL, default OFF.*
  - **ShroudGrow** — gate `Rules+0x17F0` default off (ShroudGrow=no, rulesmd.ini:677). *DORMANT.*
  - **RadioHistory** (`+0xD4/+0xD8/+0xDC`) — write-only, exhaustive scan found zero reader/consumer; not serialized in save/load. Omit from the port; do NOT branch gameplay on prior radio messages. *DORMANT.*
  - **WaypointQueue/NavQueue runtime push** — no standard YR producer (only `FootClass::Load 0x004DB3C0` reconstructs on save-load); player commands, TeamClass scripts, and TriggerAction verified-negative. Do NOT reintroduce shift-click waypoint chaining or AI-patrol NavQueue appends; keep storage/readers only for save-load tolerance. *DORMANT producer.*
  - **Dock/radio wait-queue or FIFO** — V3 proved gamemd has NONE. Saturated refinery/airfield replies NEGATORY to every HELLO; next docker = whoever re-probes and wins by distance-then-deterministic order; receiver NEVER evicts (only a full sender self-evicts its own slot-0). Remove `RefineryDockContacts.waiting_retry_queue` + `AirfieldDocks.queues`; do NOT design a dock wait-queue into a FootClass docking shell. *DRIFT to remove.*
  - **Aircraft AreaGuard** — inherited `+0x220` 450-stub; aircraft never AreaGuard.
- **Do NOT port the C++ class tree, add a second presence owner, or duplicate mission/radio state on a shell struct.** Dispatch stays `match category` + `Option::is_some()` (no AbstractClass/ObjectClass/TechnoClass trait hierarchy, no `dyn`/vtable/COM). `ObjectSubstrate` is the ONE presence/lifecycle/active-vector owner; mission/radio state already lives in `MissionCom`/`Contacts` (canonical `mission` / `radio_contacts: Contacts`). Death must enqueue to `pending_delete` (synchronous conceal/unmark/detach, deferred slot-free), never synchronous free. Any countdown must be a frame-anchored `MissionTimer` (start_frame+duration, never-decrement), never a per-tick `u8`/`u16` decrement. `EntityCategory` stays in `map/entities.rs` (one-way into sim/) — derive a sim-side `CapabilityFlags` at spawn rather than re-owning identity in the shell.

### 10.3 Residual unknowns needing a follow-up Ghidra pass before the corresponding slice

- **Before Slice 6 (verb API / ReadyToCommence hook):** the leaf ReadyToCommence **busy-flag byte semantics are INFERRED from constructor init, not from decompiled setters** — Building `+0x6DD`; Aircraft `+0x6D2`/`+0x6D4`; Unit/Infantry `+0x6E1`/`+0x6E2`/`+0x6D1`/`+0x68D`/`+0x8D`. DRIFT until each setter is traced. The exact **ReadyToCommence excluded-mission set** is likewise unconfirmed. Also unchecked: the **locomotor vtable `slot+0x80` "idle" predicate** (consumed by Unit/Infantry ReadyToCommence) was not decompiled — exact idle semantic UNCHECKED.
- **MissionControl table — RESOLVED (was a flagged conflict).** The landed `control.rs:1-12` implements **reset-per-entry** ("no carry-forward between missions"), matching `reference_mission_control_ini_reset_per_entry` and the verified `Read_INI 0x005B3760`; the mission/radio plan's earlier P0 "carry-forward" wording is superseded by the shipped code. Confirmed: stride **0x20 (32 bytes), base `0x00A8E3A8`, 32 entries** (the canonical doc's "8 bytes" = 8 *dwords*; do NOT size from that doc). Rate at `param_1[4/5]`, AARate at `param_1[6/7]` (AARate==0 copies Rate) are **INFERRED from Read_INI usage, not byte-verified**; precise C-struct field names and the leading `+0..+3` int remain unverified.
- **Before any AI/update-stage same-pass migration (core-substrate TODO #1):** the Rust tick spine is **still phase-split with frozen per-phase snapshots** (`live_object_order_snapshot()` at world/mod.rs:1741), NOT the re-read `for_each_live_object`. Undecided whether the same-pass migration is part of the object substrate or a separate native-tick-spine contract.
- **Unload-accumulator ordering — RESOLVED (§0.1).** Round-2 confirmed the Rust `tick_unload_accumulator` (`miner_dock_sequence.rs:194`) is called at `:802` **after** `phase_unloading` (`:792`), matching the native post-`Mission_Dispatch` increment (`+0xf8 += +0x110`); units-only. The earlier DRIFT-RISK is cleared; only an acceptance test pinning the order in CI remains.
- **Lockstep / RNG (verify at the matching tick position):** damage-fire particle spawn consumes `Random__RandomRanged` ×2 inside `AI_Update` (step 40), gated on ConditionYellow + DamageParticleSystems + `+0x308==0`; FootClass `Mission_Eaten` consumes `Random(0,2)`. RNG must be consumed at the same per-object position under the same gate or the shared stream desyncs. Separately, **do NOT add per-tick decrements for IronCurtain/ForceShield/Temporal** — those are passive (CurrentFrame−Start<Duration on demand), not ticked in AI_Update.
- **Render-side / out-of-lane:** confirm a `+0x70`-equivalent **smoothed display-health** with `frame&4` gating exists in the health-bar path (snaps down on damage, lerps +1/qualifying-frame up) — DRIFT-RISK (visual) if the bar snaps instead of lerping.
- **Lower-priority Ghidra confirmations:** identity/semantics of `FUN_0070ed10` (×2, gattling stage), `FUN_004a5150`/`FUN_004a5360` (team/formation tick), `FUN_00709290` (passive-acquire gate); `field_0x11c` pointer identity in the step-14 target-clear context; AircraftClass one-shot byte `param_1[10].field_0x1a` meaning; the dispatch switch lacking `case 0x1d` (only `0x1f` present for the last Spyplane entry — flagged re-check, not load-bearing for ground units); whether `g_TubeArray` is provably empty in every stock YR map (inferred, not enumerated); and the `+0xCC` post-dispatch write sourcing an uninitialized stack slot (decompiler artifact, confirm observably inert — the rate is the lo-dword in `+0xD0`).
