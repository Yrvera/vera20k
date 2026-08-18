# Core Service Profile — frontier-ai-team

**Slug:** `frontier-ai-team`
**Service:** TeamClass / TaskForceClass / ScriptClass — the AI team / mission-script engine
(recruit members per TaskForce, run a ScriptType opcode list — move/attack/guard/patrol/SW-launch/
convoy — over the team's member units).
**Status:** FRONTIER (promoted from catalog stub **F2** in `_frontier.md`). Profiled
**STRUCTURALLY ONLY** per the project AI-deferral rule — its tick-spine position, ownership,
dependency edges, and RNG/lockstep relevance. The AI *decision logic* (per-opcode target
heuristics, recruit scoring, convoy step semantics) is deliberately **NOT decoded** here.

**Primary docs (existing, Ghidra-verified):**
- `docs/research/core-services-map/_spine-rung-12.md` — the authoritative rung-L (TeamClass
  cull-and-tick) profile; build-loop/tick-loop disassembly, array globals, RNG draw site.
- `docs/research/LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` — rung L (#12) row + the lockstep
  RNG-order list (TeamClass = Scen->Random, 0 or 1 per team, opcode 0x36 only).
- `docs/research/LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md` §3 step 11/28 — the cull-into-temp
  pattern and the 2026-05-29 label-drift correction.
- `docs/research/CONVOY_FORMATION_SYSTEM_GHIDRA_REPORT.md` — team-level convoy movement helpers
  (`0x6E9050+`), member-selection-for-pathfinding criteria, straggler management.
- `docs/research/bridges/07-cross-system-consumers/AI_BRIDGE_INTERACTION_GHIDRA_REPORT.md` §(a) —
  full 64-opcode (0x00–0x40) ScriptClass dispatch table, verified via `decompile_function 0x006E9380`.
- `docs/research/FOOTCLASS_COMPLETE_GHIDRA_REPORT.md` §9.1 — `FootClass+0x5D4 = TeamClass*`
  back-pointer (Add_Member `0x6EA500` / Remove_Member `0x6EA870`).
- `docs/research/NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` §5 — team script → member
  movement routes through vtable `+0x480` (Set_Destination), NOT NavQueue push.
- `docs/research/MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md` §2 — `Destroy_All_Owned` iterates
  the team registry to tear down a defeated house's teams (an alternate array-global pair, see caveat).

**This profile:** edge/graph extract for the core-services map. Detailed body lives in the docs above.

**Evidence base / re-verification note:** No Ghidra instance was reachable this session
(`list_instances` → none; `connect_instance gamemd` → connection refused; `debugger_status` →
server not running). The addresses below were **NOT re-run live this session**. They are carried
from the `[ghidra/verified]` reports listed above, each of which cites exact
`decompile_function` / `disassemble_function` / `get_xrefs_to` evidence inline (notably
`_spine-rung-12.md`, which re-verified the whole rung against the binary). Treat every address as
**VERIFIED-VIA-CITED-DOC**, not VERIFIED-LIVE-THIS-SESSION. Re-run the representative
`TeamClass::AI @ 0x006e9140` and the cross-service edges against the binary before implementation.

---

## Stub corrections (representative address + plug point)

The seed stub (F2 in `_frontier.md`) is mostly correct on addresses but **WRONG on the spine
rung letter**. Corrected here against `_spine-rung-12.md` + `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`:

1. **Plug-point rung label — CORRECTED. The stub says "PerTickUpdate rung F". That is wrong.**
   The verified spine places TeamClass cull-and-tick at **Rung L (#12)** — driver
   `TeamClass::AI @ 0x006e9140` (`vt+0x5c` on each TeamClass), dispatched from
   `LogicClass::PerTickUpdate @ 0x0055AFB0` (build loop `0x0055b4f5–0x0055b582`, tick loop
   `0x0055b582–0x0055b5a1`). Rung **F** in the same spec is a different system entirely
   (F = FogOfWar re-shroud / lighting 2nd channel, `0x004acbc0`, a TS-legacy fog pass).
   The stub's `g_TeamClass_Array +0x5C` shorthand is right in spirit (it IS a `vt+0x5c` AI head
   over the team array), just mis-lettered.

2. **Representative function — CONFIRMED.** Stub `TeamClass__AI @ 0x006E9140` is correct: it is
   the slot-23 (`vt+0x5c`) AI head the spine dispatches per surviving team
   (`_spine-rung-12.md`: vtable write `*this = 0x007f4730`, slot `+0x5c` = `0x006e9140`).

3. **Other stub addresses — CONFIRMED.** `TeamClass__Recruit_Or_Add @ 0x006E9380` is the **sole
   ScriptClass opcode dispatcher** (64-opcode table 0x00–0x40), verified via
   `decompile_function 0x006E9380` in `AI_BRIDGE_INTERACTION`. The stub's
   `Convoy_Script_Attack_Nearest @ 0x006EE310` falls inside the verified convoy-script helper
   range (`0x006EC7D0`–`0x006EFA10`); `CONVOY_FORMATION` / `NAVCOM_NAVQUEUE_PUSH_PRODUCERS`
   enumerate the neighbours (Attack_Nearest = opcode 0x2E per the opcode table).

---

## Purpose (one line + minimal structural elaboration)

The **AI team / mission-script engine**: a `TeamClass` is a roster of member units recruited per a
`TaskForceClass` composition, executing a `ScriptTypeClass` opcode list (a `ScriptClass` is the
live cursor into that list) — Patrol / Attack-Building / Move-to-Cell / Move-to-Waypoint / Follow /
Guard / convoy-move / Attack-Nearest|Farthest|Production / random-move / SW-launch / start-storm /
para-drop / reveal-map, etc. Each tick (rung L) every live team advances its current opcode, which
drives its member units' missions/destinations. Created by the HouseClass AI brain (via AITriggers),
by map `[Teams]`/triggers, and by the script's own "create new team" opcode (0x12). **Decision
logic deliberately NOT decoded** — this profile is the spine connectivity only.

---

## Owns (globals / structs / state, with addresses where known)

- **The global team registry** (the rung-L walk source): array base
  `g_TeamClass_Array @ 0x008b40ec`, count `g_TeamClass_Array_Count @ 0x008b40f8`.
  Confirmed by `_spine-rung-12.md` via `get_xrefs_to 0x008b40ec` (TeamClass__Constructor xref)
  and the disassembled loop bounds (`MOV EAX,[0x008b40f8]` at `0x0055b502`/`0x0055b577`).
  **NOTE on label history:** `LOGICCLASS_VS_MAPCLASS` (2026-05-29) flagged "`DAT_008B40EC /
  DAT_008B40F8` = GLOBAL OBJECTS POOL" as RTTI_LABEL_DRIFT — that correction was about the symbol
  *name* (it is the *team* array, not a generic object pool), **not** the address; the address
  `0x008b40ec` is the team registry. **UNRESOLVED address discrepancy (flag for live re-check):**
  `MULTIPLAYER_DEFEAT_VICTORY` §2 `Destroy_All_Owned` reads the team array as
  `DAT_00a8ec7c` (array) / `DAT_00a8ec88` (count) — a *different* global pair. Either (a) these
  are two aliases/views of the same vector resolved by different symbol scripts, or (b) one is the
  master TeamClass `DynamicVectorClass` and the other its raw-array shadow. Could not be reconciled
  live this session; do not assume one without a Ghidra `get_xrefs_to` on both.
- **The `TeamClass` instance struct** (offsets from cited docs, partial):
  `+0x3C` current primary (convoy) target, `+0x40` secondary/previous target, `+0x54` first-member
  pointer (head of the member list, iterated via member `+0x5D8`), `+0x77/0x79/0x7a/0x7b/0x7d/0x7f`
  active/suspended/disbanded/recruiting flag bits, `+0x7f` movement-complete, `+0x80` "step done"
  (all members arrived), `+0x82` off-screen target-building flag, `+0x87` Owner `HouseClass*`,
  the script CD-timer at `+0x19/+0x1b` (un-elapsed → opcode skipped this tick). vtable `0x007f4730`,
  slot `+0x5c` = `0x006e9140`.
- **The member back-pointer it writes onto FootClass:** `FootClass+0x5D4 = TeamClass*`
  (`FOOTCLASS_COMPLETE` §9.1): `Add_Member @ 0x006EA500` writes it, `Remove_Member @ 0x006EA870`
  clears it. This is how a unit knows which team it belongs to (consumed by mission/retaliation code).
- **The rung-L temp snapshot vector** (transient, owned for one tick): a stack-local
  `DynamicVectorClass<ObjectClass*>` built by `FUN_0055bb40` (capacity `local_4 = 10`), filled
  from the registry, ticked, then freed at rung AB (`0x007C8B3D`). Ownership is per-tick, not
  persistent state — listed because it is the "snapshot-before-mutation" structure the rung depends on.
- **The TaskForce / ScriptType / TeamType type-data** (`TaskForceClass`, `ScriptTypeClass`,
  `TeamTypeClass`): the INI-loaded recipes a TeamClass instantiates from. `TeamTypeClass` carries
  e.g. `IsBaseDefense` at `+0xEC3` (`TARGET_ACQUISITION` §8) and the `DiscreteDistribution`-weighted
  AITrigger linkage; `MindControlDecision` is read into TeamType/TaskForce AI data at global
  `0x008430D8` (`MIND_CONTROL_SYSTEM` §12, flagged live-YR AI). Their loaders/constructors were
  not re-verified this session (AI-loader scope deferred).

---

## Key functions & globals (addresses)

All VERIFIED-VIA-CITED-DOC (not re-run live this session — see evidence-base note above).

| Symbol | Address | Role |
|---|---|---|
| `TeamClass::AI` | 0x006e9140 | **Representative fn** — the slot-23 (`vt+0x5c`) per-tick AI head; reads team flags + script CD-timer, then dispatches the current ScriptClass opcode. Rung L driver. |
| `LogicClass::PerTickUpdate` | 0x0055AFB0 | The spine. Rung-L build loop `0x0055b4f5–0x0055b582` (cull registry → temp), tick loop `0x0055b582–0x0055b5a1` (`vt+0x5c` per team). |
| `FUN_0055bb40` (temp DynVec ctor) | 0x0055bb40 | Constructs the per-tick snapshot vector (capacity 10). Plain ctor — walks nothing, draws no RNG (`_spine-rung-12.md`). |
| `TeamClass::Recruit_Or_Add` | 0x006E9380 | **Sole ScriptClass opcode dispatcher** — 64-opcode table 0x00–0x40 (recruit/run-script). Verified via `decompile_function 0x006E9380`. |
| `TeamClass::Add_Member` | 0x006EA500 | Adds a unit to the team; writes `FootClass+0x5D4 = this`. |
| `TeamClass::Remove_Member` | 0x006EA870 | Removes a unit; clears `FootClass+0x5D4 = 0`. |
| `TeamClass::Set_Convoy_Target` | 0x006E9050 | Central convoy coordination — sets team target fields, calls member `vt+0x480` to reset. Called by nearly all convoy script ops. |
| `TeamClass::Convoy_Move_With_Target` | 0x006EB490 | Move when target known; picks best member as pathfinder (highest `ThreatPosed`). |
| `TeamClass::Convoy_Move_Without_Target` | 0x006EBAD0 | Move toward current target; straggler handling. |
| `TeamClass::Convoy_Guard_Members` | 0x006EBF50 | Guard members; send stragglers to catch up. |
| `TeamClass::Convoy_Script_Move_To_Cell` | 0x006EC7D0 | Script op 0x03 helper. |
| `TeamClass::Convoy_Script_Move` | 0x006ECCE0 | Script op 0x10 helper. |
| `TeamClass::Convoy_Script_Patrol` | 0x006ED090 | Script op 0x00 helper. |
| `TeamClass::Convoy_Script_Attack_Nearest` | 0x006EE310 | Script op 0x2E helper (stub's rep recruit-step example). |
| `TeamClass::Find_Best_Target_Building` | 0x006EEBD0 | 4-mode building target scoring for attack ops (no CABHUT/bridge weighting). |
| `TeamClass::Convoy_Script_Attack_Move` | 0x006EF700 | Script op 0x35 helper. |
| `TeamClass::Convoy_Script_Random_Move` | 0x006EFA10 | Script op 0x36 — **the rung's only verified RNG consumer** (see RNG section). |
| `Random__RandomRanged` | 0x0065c7e0 | The RNG draw callee (lagged-Fibonacci over the receiver state object). |
| `Destroy_All_Owned` | 0x004FB920 | HouseClass teardown — walks the team registry to destroy a defeated house's teams (`vt+0x20`, arg=1). |
| **Scheduler infra (not owned; shared):** | | |
| `ObjectClass::AI` (base slot 23) | 0x005F3E70 | Base `vt+0x5c`; TeamClass overrides it with `0x006e9140`. |
| Rung-AB temp free | 0x007C8B3D | Frees the rung-L snapshot vector at end of the tick (`LOGICCLASS_PERTICKUPDATE_SPINE_SPEC` rung AB). |

**Globals / fields:**
- `g_TeamClass_Array @ 0x008b40ec`, count `@ 0x008b40f8` — the team registry walked by rung L
  (confirmed-by-xref). Alternate pair `DAT_00a8ec7c`/`DAT_00a8ec88` cited by `Destroy_All_Owned`
  — UNRESOLVED, see Owns/caveat.
- `g_ScenarioClass_Instance @ 0x00a8b230`; the synchronized RNG is at `Scen+0x218` (`Scen->Random`).
- `FootClass+0x5D4` — the per-unit `TeamClass*` membership back-pointer.
- `TeamTypeClass+0xEC3` — `IsBaseDefense`; AI-team type-data global `0x008430D8` (`MindControlDecision`).

---

## Tick position (the spine plug point)

**Plug point: the per-tick spine, Rung L (#12) — TeamClass cull-and-tick.** Out-of-render, in-sim.

- **Driver:** `LogicClass::PerTickUpdate @ 0x0055AFB0`. Two sequential phases (per
  `_spine-rung-12.md`, disassembly-verified):
  1. **Build temp list** (`0x0055b4f5–0x0055b582`): construct a stack-local DynamicVector
     (`FUN_0055bb40`, capacity 10), then walk the registry (`g_TeamClass_Array_Count > 0` gate)
     copying each team pointer into the temp. This is the classic **snapshot-before-mutation**: a
     team can disband members or spawn a new team (script op 0x12) during its own AI tick, so the
     live registry is unsafe to iterate directly.
  2. **Tick temp list** (`0x0055b582–0x0055b5a1`): forward over the temp (temp count > 0 gate),
     `vt+0x5c` (= `TeamClass::AI`) per surviving team.
- **Gate:** **count only — NO `g_GameMode` gate** (contrast rung U / AnimClass which IS
  `g_GameMode != 0 && != 5`). Per-team gating happens *inside* `TeamClass::AI` (active/suspended/
  disbanded flags + script CD-timer at `+0x19/+0x1b`; un-elapsed timer early-returns).
- **Order within the tick (lockstep-relevant):** rung L sits immediately after rung K
  (`FUN_0054e4d0`, 30-frame scripted re-anchor, `0x0055b4f0`) and immediately before rung M
  (DiskLaser reverse-walk, `0x0055b5a1`). The rung-L temp vector is **freed at rung AB**
  (`0x007C8B3D`) at the end of the same tick. The MAIN per-object AI fan-out (rung T, `0x005F3E70`,
  where the team's member *units* actually move/fire) runs *later* in the same tick — so a team's
  rung-L opcode sets member destinations/missions that the member units then act on in rung T.
- **Member movement does NOT go through NavQueue push:** team scripts reissue member destinations
  via member vtable `+0x480` (Set_Destination), not by appending to `FootClass` NavQueue
  (`NAVCOM_NAVQUEUE_PUSH_PRODUCERS` §5). Structural note for the movement edge.
- **Render:** NONE owned by this service. A team is not a drawable object; only its member units
  (techno-foot) draw. (Convoy formation is a sim concept, not a render layer.)

---

## RNG / lockstep relevance

**Draws RNG: YES (conditionally) — stream = `Scen->Random` (the synchronized lockstep stream).**

- The only verified draw on this rung: script **opcode 0x36** (random-move) →
  `TeamClass::Convoy_Script_Random_Move @ 0x006efa10`, draw site `0x006efacb–0x006efadc`:
  receiver ECX = `g_ScenarioClass_Instance + 0x218` (`Scen->Random`), `RandomRanged(0,0xff)`,
  one draw, **only** on the branch where the team has no enemy-house reference (the enemy-house
  branch uses `Math::atan2` toward the enemy and draws 0). The byte becomes a random facing/angle
  for the move destination. Verified via `disassemble_function 0x006efa10` (`_spine-rung-12.md`).
- **Per-tick count:** **0 or 1 Scen->Random draw per ticked team**, only for teams executing
  opcode 0x36. All other opcode helpers inspected (e.g. `FUN_00747370`, the TRUCKA/TRUCKB type
  pick at ops 0x2c/0x2d) are deterministic string/scored lookups with no RNG. (Not all ~64 opcode
  helpers were exhaustively decompiled; opcode 0x36 is the confirmed consumer, others UNCHECKED but
  none observed drawing in the dispatch body.)
- **Lockstep contract:** rung L is item #3 in the spine's RNG-draw subsequence
  (after H Tiberium-growth, I Tiberium-spread; before M DiskLaser, P LightningStorm, T main object).
  Because the draw count is data-dependent (how many teams run opcode 0x36) but **deterministic
  given identical lockstep state**, it is lockstep-safe *iff* team iteration order (registry
  insertion order) and the cull are reproduced exactly. Reordering teams, or reordering rung L
  relative to its neighbours, shifts every later RNG result and desyncs. This is the determinism
  reason the rung must keep its exact position and iteration order even though AI is deferred.

---

## Depends-on (outgoing edges)

Each edge: target slug + via-symbol + evidence. (Evidence = cited verified docs; not re-run live.)

1. **logicclass** (the scheduler / spine) — structural driver edge
   - via: `LogicClass::PerTickUpdate @ 0x0055AFB0` rung L builds the temp from the team registry and
     dispatches `TeamClass::AI @ 0x006e9140` via `vt+0x5c`; frees the temp at rung AB.
   - evidence: `_spine-rung-12.md` (full build/tick disassembly); `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC`
     rung L row. This is the "where it ticks" edge.

2. **techno-foot** (TechnoClass + FootClass) — the units it commands — STRONGEST gameplay edge
   - via: `TeamClass::Add_Member @ 0x006EA500` / `Remove_Member @ 0x006EA870` write/clear the
     `FootClass+0x5D4` `TeamClass*` back-pointer; the convoy/attack script helpers issue member
     missions/destinations through member vtable `+0x480` (Set_Destination) and select the "best"
     member for pathfinding by `type->ThreatPosed (+0x5FC)`, alive/health/cloak/arrived flags.
   - evidence: `FOOTCLASS_COMPLETE` §9.1 (the +0x5D4 back-pointer, both writers);
     `CONVOY_FORMATION` (member-selection criteria, straggler mission reassignment);
     `NAVCOM_NAVQUEUE_PUSH_PRODUCERS` §5 (member moves via +0x480, not NavQueue).

3. **target-scoring** (target acquisition / threat evaluation)
   - via: attack opcodes score targets — `TeamClass::Find_Best_Target_Building @ 0x006EEBD0`
     (4-mode: closest-by-slope-cost / closest-by-Euclidean), Attack-Nearest/Farthest/Production
     helpers, and `Set_Convoy_Target` target selection; `TeamTypeClass::ComputeZoneCategory` feeds
     compatible-movement-zone scoring.
   - evidence: `AI_BRIDGE_INTERACTION` §(b) (`Find_Best_Target_Building` scoring modes);
     `TARGET_ACQUISITION` §8 (TeamType `IsBaseDefense` targeting); opcode table 0x2E/0x2F/0x30/0x3A.

4. **pathfinding-helpers** (movement / zone pathfinding)
   - via: move/patrol/attack-move convoy ops path the chosen pathfinder member toward the target;
     `TeamTypeClass::RecomputeAllZoneCategories @ 0x006F2040` → `ComputeZoneCategory @ 0x006F1FA0`
     → `ZoneMap::FindBestCompatibleMovementZone @ 0x005889F0` resolves a team's movement zone.
   - evidence: `ZONE_PASSABILITY_MATRIX_READERS` §5 (the TeamType zone-category chain);
     `CONVOY_FORMATION` (CloseEnough `Rules+0x171C` / Stray `Rules+0x1720` straggler distances).

5. **random-scenario** (RandomClass + ScenarioClass) — lockstep RNG
   - via: opcode 0x36 random-move `RandomRanged(0,0xff)` on `Scen->Random` (`Scen+0x218`) via
     `Random__RandomRanged @ 0x0065c7e0`; reads `g_ScenarioClass_Instance @ 0x00a8b230`.
   - evidence: `_spine-rung-12.md` RNG section (draw site `0x006efacb–0x006efadc`, receiver
     `Scen+0x218`); spine RNG-order list item #3. Lockstep-critical.

6. **factory-house** (HouseClass — Owner + creator) — type/ownership edge
   - via: each team has an Owner `HouseClass*` (`TeamClass+0x87`); the HouseClass AI brain creates
     and dispatches teams; `Destroy_All_Owned @ 0x004FB920` (HouseClass teardown) destroys a
     house's teams on defeat.
   - evidence: `MULTIPLAYER_DEFEAT_VICTORY` §2 (`team->Owner == this` walk + `vt+0x20` destroy);
     opcode 0x13–0x15 "house calls". (Creation/dispatch direction is also an incoming edge — see below.)

7. **mission-radio** (MissionClass — member mission assignment) — weak/structural
   - via: script ops reassign member *missions* (e.g. Follow→Capture(8), Guard(2), Move) and read
     member mission state (Guard mission gate in pathfinder selection); the team's commands surface
     as MissionClass state changes on the members.
   - evidence: `CONVOY_FORMATION` (Follow→mission-8 reassignment; Guard-mission pathfinder gate).
     Weak edge — the team issues missions; MissionClass owns their execution.

8. **rules-class** (RulesClass) — tuning globals + type-data origin — weak
   - via: convoy straggler distances `CloseEnough (Rules+0x171C)` / `Stray (Rules+0x1720)`; the
     TaskForce/ScriptType/TeamType recipes originate from the rules INI parse (AI-team loader,
     not re-verified here).
   - evidence: `CONVOY_FORMATION` (Rules+0x171C/0x1720). The TeamType INI loader is AI-loader scope
     (deferred); the type-data dependency is structural.

(Weak/transitive) **cell-map** — convoy move-to-cell / zone reads consult CellClass occupancy via
the pathfinding chain; listed transitively (the direct cell reads are inside pathfinding-helpers /
the member units, not TeamClass itself).

---

## Used-by (incoming edges)

Other services that create / drive / consume teams:

1. **frontier-ai-house** (HouseClass AI brain) — the principal creator/dispatcher [FRONTIER]
   - via: the per-house AI economy/production brain (rung AA) builds TaskForces and dispatches
     `TeamClass` instances to attack/scout/defend; teams are the AI brain's "units of intent."
   - evidence: `_frontier.md` F3 (the house brain "dispatches teams"); `MULTIPLAYER_DEFEAT_VICTORY`
     §2 (house owns its teams). Structural — the brain is the upstream producer.

2. **frontier-ai-trigger** (AITriggerTypeClass) — skirmish team production [FRONTIER]
   - via: weighted, condition-gated AI triggers (`DiscreteDistributionClass`) decide *which*
     TaskForce/Team to produce and send; evaluated within the house brain.
   - evidence: `_frontier.md` F4 ("produces teams"); `MIND_CONTROL_SYSTEM` §12
     (`MindControlDecision` in TeamType/TaskForce AI data at `0x008430D8`, live-YR AI).

3. **frontier-trigger** (map triggers / TActionClass) — map-script team creation [FRONTIER]
   - via: map `[Teams]` and trigger actions create/reinforce teams (campaign + some skirmish maps);
     `TriggerAction::Execute @ 0x006DD8B0` calls team/waypoint helpers (`0x006E0AA0`, `0x006E0FE0`,
     `0x006E11C0`, `0x006E2050`); the 30-frame rung-K driver (`FUN_0054e4d0`) handles reinforcement
     / team-event cadence.
   - evidence: `NAVCOM_NAVQUEUE_PUSH_PRODUCERS` §5 (trigger team helpers);
     `LOGICCLASS_VS_MAPCLASS` §3a (rung-K 30-frame scripted-action trigger).

4. **logicclass** (the scheduler) — bidirectional (also depends-on)
   - via: rung L *is* the LogicClass per-tick driver of this service; listed both ways because the
     spine both owns the iteration and is the thing the service plugs into.
   - evidence: `_spine-rung-12.md`.

5. **factory-house** (HouseClass teardown) — bidirectional (also depends-on)
   - via: `Destroy_All_Owned @ 0x004FB920` destroys a defeated house's teams; team Owner is a
     HouseClass. (Creation is via the AI-house/AI-trigger frontier services above.)
   - evidence: `MULTIPLAYER_DEFEAT_VICTORY` §2.

---

## Active in YR / Tiberian Sun legacy

**Active in YR: YES — TeamClass is the live AI team/scripting engine.** In a normal YR skirmish,
computer (AI) houses create teams from TaskForce/TeamType recipes via AITriggers; map
triggers/`[Teams]` also create them (campaign + scripted maps). Rung L fires every tick any team
exists. Player-visible effect: AI attack waves, scouting, harvester/engineer micro, convoy
behaviors, super-weapon launches — all flow through these ScriptClass opcodes. The opcode set is
current YR (e.g. 0x37/0x38/0x39 single/dual SW-launch, 0x20/0x21 LightningStorm start/stop, the
0x2e–0x36/0x3a/0x3b convoy attack/move variants), **not** dead TS branches.

**Project scope caveat (NOT a parity statement):** per project memory `feedback_no_ai_yet`,
AI-system *implementation* is deferred at the current stage. That is a **scheduling** decision —
gamemd unambiguously runs rung L in standard YR skirmish, so it remains a real rung in the lockstep
order and a future parity obligation. **It must NOT be dropped from the per-tick ORDER**, and the
rung-L → rung-AB temp-vector free is part of the spine even when the temp is empty. This profile is
therefore structural-only by directive, not because the system is inactive.

**No TS-legacy dead branches identified in the rung itself** (unlike rungs D/F fog passes). Within
the ~64-opcode table some individual actions may be campaign-only / never used in YR skirmish maps
(stub F1/F2 note) — that per-opcode liveness audit is part of the deferred AI decode, not done here.

---

## Open / unverified items

- **NO live Ghidra re-verification this session** — no instance reachable (`list_instances` none;
  `connect_instance` refused). Every address is carried from `[ghidra/verified]` docs (chiefly
  `_spine-rung-12.md`, which itself re-verified the rung against the binary). Re-run
  `TeamClass::AI @ 0x006e9140` and the techno-foot / target-scoring / pathfinding edges live before
  any implementation. **Top caveat.**
- **Team-array global discrepancy (must reconcile live):** rung L reads
  `g_TeamClass_Array 0x008b40ec` / count `0x008b40f8` (xref-confirmed in `_spine-rung-12.md`), but
  `Destroy_All_Owned` reads `DAT_00a8ec7c` / `DAT_00a8ec88`. Resolve whether these are the same
  vector (alias) or two distinct globals via `get_xrefs_to` on both before trusting either in a port.
- **ScriptClass / ScriptTypeClass cursor mechanics** — the live opcode-cursor advance, jump (op
  0x06), load-new-script (op 0x11), and timer (op 0x05) semantics are AI decision logic, deferred.
- **TaskForce recruit scoring + TeamType INI loader** — recruit member selection, min/max counts,
  and the rules INI parse for `[TaskForces]`/`[ScriptTypes]`/`[TeamTypes]`/`[AITriggerTypes]` were
  NOT decoded (AI-loader scope). The `RulesClass::Read_INI` step previously misidentified as the
  AI-team/TaskForce loader is actually the advanced-command-bar reader (`RULESCLASS_GHIDRA_REPORT`
  §5 step 33) — so the real TaskForce loader address is **UNKNOWN this session**.
- **Per-opcode RNG completeness** — only opcode 0x36 confirmed as an RNG consumer; the other ~63
  opcode helpers were not exhaustively decompiled for hidden draws (none observed in the dispatch
  body). A full per-opcode RNG audit is part of the deferred AI decode.
- **Rust port status:** per `feedback_no_ai_yet`, no TeamClass/ScriptClass equivalent exists in the
  Rust sim yet. The rung must nonetheless be reserved in the tick ORDER (and its empty-temp build/
  free) so that when AI lands, the lockstep RNG sequence and tick position are already correct.
