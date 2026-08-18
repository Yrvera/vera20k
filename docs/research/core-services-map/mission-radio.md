# Core Service Profile — mission-radio (MissionClass + RadioClass)

**Slug:** `mission-radio`
**Service:** MissionClass (mission FSM: mission cycle / MissionType / dispatch timers) +
RadioClass (synchronous radio links: dock / load / enter contacts).
**Primary doc:** `docs/research/MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md` (substrate design,
Ghidra-verified 2026-06-01). Cross-edges confirmed via `research_search`/`research_related`
over the verified-ghidra corpus this session.
**Authority:** binary → Ghidra → docs. Substrate doc is Ghidra-verified and cites live
addresses; treated as primary evidence base.

---

## Purpose

Two cooperating per-object substrate services that the **entire active-techno population**
flows through:

1. **MissionClass = the mission scheduler.** Owns "what is this object doing right now": a
   single `CurrentMission` selector + a `Mission_Dispatch` switch, a frame-anchored
   self-throttling per-object timer (handler integer return `N` = defer N frames), and a
   verb API (`Assign`/`Queue`/`Commence`/`Override`/`Restore`) with a 1-deep suspend stack.
   There is **no global scheduler** — throttling is entirely per-object.
2. **RadioClass = the contact RPC bus.** A synchronous (no-queue) message protocol —
   `Transmit_Radio_Impl` calls the target's `Receive_Radio` inline and returns a response
   code — plus a sparse, capacity-bounded `Contacts[]` array with HELLO/BREAK bookkeeping.
   Carries dock/board/tether/repair/reload handshakes.

gamemd implements both as adjacent inheritance layers in the
`AbstractClass → ObjectClass → MissionClass (0xAC–0xD3) → RadioClass (0xD4–0xF3) →
TechnoClass` chain. We reproduce the observable contract, not the class tree.

---

## Owns (state, globals, tables)

### MissionClass per-object state (byte offsets, verified live)
- `+0xAC` CurrentMission (committed) · `+0xB0` SuspendedMission · `+0xB4` QueuedMission
  (pending) · `+0xB8` reset/just-started byte · `+0xBC` MissionState (substate index) ·
  `+0xC0` mission-start frame · `+0xC4` MissionTickCounter (++ every AI tick) ·
  `+0xC8` DispatchTimer.Start · `+0xCC` scratch (dead/uninitialized) ·
  `+0xD0` DispatchTimer.Rate (= handler return).

### MissionClass globals / static tables
- `g_MissionNameTable @ 0x00816CAC` — 32 `char*` mission names (e.g. "Ambush" @ 0x00816DF8,
  "Rescue" @ 0x00816DB4).
- `g_MissionControl array @ 0x00A8E3A8` — 32 × 0x20-byte INI records
  (Rate/AARate/NoThreat/Zombie/Recruitable/Paralyzed/Retaliate/Scatter per mission).
- `g_CurrentFrameCounter @ 0x00A8ED84` — the time base all dispatch timers snapshot.
- No singletons/registries beyond the two static tables; dispatch throttling is per-object.

### RadioClass per-object state
- `+0xD4/+0xD8/+0xDC` RadioHistory[0..2] (3-deep push-down dedup log; base writes only, no
  proven base reader) · `+0xE4` Contacts.data (`TechnoClass**`) · `+0xE8` Contacts.Capacity
  (signed loop bound) · `+0xEC` CanGrow · `+0xED` Initialized.
- Dock-flag bytes used by the protocol but **owned by TechnoClass**: `+0x418` dock-entered
  flag (0x18 sets, 0x19 clears) · `+0x2E4` reciprocal two-sided link pointer (tank bunker /
  service-depot install).

### RadioClass globals
- `g_RadioScratchBuffer @ 0x00A8EC30` — single static payload scratch (safe only because
  gamemd is single-threaded; a concurrent port must use per-call scratch).

---

## Key functions & globals (addresses)

### MissionClass — methods + vtable slots
| Symbol | Address | vtable slot | Role |
|---|---|---|---|
| `Mission_Dispatch` (AI) | `0x005B3060` | +0x05C | per-tick dispatch + rate throttle |
| `GetCurrentMission` | `0x005B3040` | +0x184 | return current, else queued |
| `Queue_Mission` | `0x005B35E0` | +0x1E8 | set queued; optional commence-now |
| `Commence` | `0x005B3570` | +0x1EC | promote queued→current, zero timer |
| `Assign_Mission` | `0x005B2FD0` | +0x1F0 | force current, reset timer (bypass queue) |
| `Override_Mission` | `0x005B3650` | +0x1F4 | suspend + switch |
| `Restore_Mission` | `0x005B36B0` | +0x1F8 | pop suspended → current |
| `Is_Mission_Suspended` | `0x005B3A10` | +0x1FC | suspended != -1 |
| `ReadyToCommence` (base) | `0x004E0140` | +0x200 | base stub `return 1`; subclass-overridden |
| `GetMissionTimerEntry` | `0x005B3A00` | — | `&MissionControl[current*0x20]` |
| `Read_INI` | `0x005B3760` | — | parse per-mission `[MissionName]` INI block |
| `Mission_From_Name` / `Mission_Name` | `0x005B3910` / `0x005B3950` | — | name↔id |
| `Constructor` | `0x005B2DA0` | — | inits +0xAC..+0xD0 (current/queued/susp = -1) |
| Mission handler base stub | `0x005B2E10` | +0x204..+0x270 | 32 slots; base = `return 0x1C2` |
| Ambush(14) stub slot | `0x005B2E30` | FootClass +0x20C | `mov eax,0x1C2; ret` — dead TS stub |

### RadioClass — methods + vtable slots
| Symbol | Address | vtable slot | Role |
|---|---|---|---|
| `Receive_Radio` (base tail) | `0x0065A820` | +0x194 | HELLO/BREAK bookkeeping + history shift |
| `Transmit_Radio_Impl` | `0x0065A970` | +0x27C | core send; inline `target->Receive_Radio` |
| `Transmit_Radio` | `0x0065AAA0` | +0x278 | wrapper supplying `g_RadioScratchBuffer` |
| `Transmit_Radio_ToFirst` | `0x0065ACB0` | +0x274 | send to `Contacts[0]` only |
| `Broadcast_Radio_ToAll` | `0x0065ACE0` | +0x280 | send to every non-null contact |
| `FindDockSlot` | `0x0065AD90` | — | linear scan → contact-slot index, else -1 |
| `Set_Contact_Count` | `0x0065AE60` | — | grow-only capacity (sole caller: BuildingClass ctor) |
| `DynamicVectorClass::Contains` | `0x0065AD50` | — | membership bool (used by Can_Enter_Cell) |
| `Constructor` | `0x0065A750` | — | capacity=1, 1-slot Contacts, history zeroed |
| `Filter_AbstractType_InMap` | `0x0040DD70` | — | RTTI sender filter (Unit/Aircraft/Building/Infantry) |
| AircraftClass radio-deaf gate | `0x004190B0` | — | mission-state gate dropping all radio msgs |

### Response codes (return values, not messages)
ROGER=`0x01` · NEGATORY=`0x0A` · CELL_ACCEPTED=`0x14` · QUEUED=`0x17` ·
INSUFFICIENT_FUNDS=`0x20` · REPAIR_COMPLETE=`0x21` · silent/no-op=`0`.

---

## Tick / render position

**Not a tick spine itself; runs once per active techno per logic frame, inside object AI.**

Verified caller chain (research-index, ghidra/verified):
`UnitClass::AI @ 0x007360C0 → FootClass::AI @ 0x004DA530 → TechnoClass::AI_Update @ 0x006F9E50 → MissionClass::Mission_Dispatch @ 0x005B3060`.

- `TechnoClass::AI_Update @ 0x006F9E50` is the **sole** static caller of `Mission_Dispatch`
  (verified `get_function_callers`). One dispatch per active object per frame.
- Dispatch order inside the handler: `ObjectClass::AI()` first → active gate (`+0x90 != 0`)
  → due gate (`start == -1 || g_CurrentFrame - start >= dur`, inclusive) → health gate
  (`HP > 0`) → handler → commit timer (`+0xC8 = now`, `+0xD0 = N`). Tick counter `+0xC4`
  increments every AI tick regardless of the due gate.
- `Mission_Dispatch` runs **before** the unload-accumulator / locomotor `Process` block in
  `AI_Update` (verified `TECHNOCLASS_AI_UPDATE_UNLOAD_ACCUMULATOR_ORDERING`).
- **RadioClass has no tick slot** — it is synchronous RPC invoked from whatever caller is
  mid-handshake (mission handlers, dock choreography, command handlers, limbo/death cleanup).
- In the Rust port this maps to phase wiring in `World::advance_tick` (commands → movement →
  vision → power → turrets+combat → retaliation → scatter/production/repairs/docks/ore → AI
  → defeat → anims → hash). The substrate doc requires preserving that phase order; the
  shared timer/verb API is adopted *within* existing slots, not by collapsing phases.

---

## Depends-on (outgoing edges)

| Target slug | Via symbol / field | Evidence |
|---|---|---|
| `lookup-tables` | `g_MissionControl @ 0x00A8E3A8` (32×0x20 records) via `GetMissionTimerEntry @ 0x005B3A00`; `g_MissionNameTable @ 0x00816CAC` via `Mission_From_Name`/`Mission_Name` (0x005B3910/0x005B3950) | Static read-only tables the scheduler indexes per mission. §2.1 substrate doc. |
| `ini-parsing` | `MissionClass::Read_INI @ 0x005B3760` parses per-mission `[MissionName]` blocks into the MissionControl records | §2.1, §5.1.8 substrate doc; MissionControl is INI-driven (reset-per-entry, 32 slots). |
| `rules-class` | MissionControl tunables (Rate/AARate/NoThreat/Zombie/Recruitable/Paralyzed/Retaliate/Scatter) sourced from the merged rules INI; handlers read their own Rate to choose deferral N | §1.1, §5.1.8; per-mission INI config feeds the scheduler's throttle. |
| `random-scenario` | `g_CurrentFrameCounter @ 0x00A8ED84` is the time base every dispatch timer snapshots; mission *handlers* (not the scheduler) consume RNG via per-callsite ECX (e.g. Mission_Enter 14–16 jitter) | §5.1.2 (frame-anchored due gate), §5.1.7 (no RNG in scheduler; handlers draw via Scen->Random/per-callsite). Frame counter is scenario/global state. |
| `cell-validation` | A radio contact gates `UnitClass::Can_Enter_Cell @ 0x0073F0A0` passability via `DynamicVectorClass::Contains @ 0x0065AD50` — the `NumberImpassableRows` west-column skip for a contacted building occupant | §3.3 idiom 2; `NUMBER_IMPASSABLE_ROWS_RADIO_CONTACT_VECTOR` + `WAR_FACTORY_EXIT_CONTACT_ROW_SKIP` (verified). Radio membership is the input to the validator. |
| `factory-house` | War-factory exit: `BuildingClass::ExitObject_Main @ 0x00443C60` sends HELLO(0x02)+ENTER_DOCK(0x18) and `Queue_Mission(0x10)` on the vehicle; refinery deposit drives credits via `Mission_Deploy_Building`; Rescue assignment reads `HouseClass::IsPlayerControl` | §3.3 idioms 1–2; `WAR_FACTORY_EXIT_CONTACT_ROW_SKIP`, `MISSION_RESCUE_21_HANDLER_AND_AMBUSH_14_STUB`. Production/economy/house drive and consume mission+radio. |
| `damage-helpers` | Rescue(21) mission is assigned through `FootClass::ReceiveDamage @ 0x004D7330` → (IsPlayerControl()==0 + team gate) → `FUN_00708080` → `Queue_Mission(0x15,0)` on idle teammates | `MISSION_RESCUE_21_HANDLER_AND_AMBUSH_14_STUB` (verified). ReceiveDamage is the live trigger of a mission transition. |
| `abstract-object` | Limbo/death cleanup: `Broadcast_Radio_ToAll(BREAK) @ 0x0065ACE0` runs every contact's `Receive_Radio` teardown; ObjectClass::AI is the first step in Mission_Dispatch; missions read `+0x90` active byte and HP | §5.2.8 (broadcast-on-limbo), §5.1.1 (ObjectClass::AI first). `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP`. Lifecycle owner. |
| `techno-foot` | Radio sender RTTI filter `Filter_AbstractType_InMap @ 0x0040DD70`; dock-flag/reciprocal-link fields (`+0x418`, `+0x2E4`) are TechnoClass-owned; locomotor piggyback missions (QMove etc.) live on FootClass | §2.2 (filter), §2.3 (TechnoClass-owned dock bytes). Bidirectional — see Used-by. |

---

## Used-by (incoming edges)

| Source slug | Via symbol | Evidence |
|---|---|---|
| `techno-foot` | `TechnoClass::AI_Update @ 0x006F9E50` is the **sole** static caller of `Mission_Dispatch @ 0x005B3060`, once per active techno per frame; FootClass/UnitClass/AircraftClass/InfantryClass override mission-handler vtable slots (e.g. FootClass::Mission_Attack 0x4D4DC0, AircraftClass QMove 0x00415A50 at Retreat slot) | `MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS`, `TECHNOCLASS_AI_MIGRATION_BOUNDARY` (verified). Object-AI dispatch is the entry point. |
| `logicclass` | The per-tick update scheduler drives the AI pass (`...->FootClass::AI->TechnoClass::AI_Update`) that invokes Mission_Dispatch each logic frame; the global frame counter advanced by the tick spine is the mission timer's time base | Caller chain + §5.1.2 frame-anchoring. Tick spine is the upstream pump. |
| `factory-house` | BuildingClass/FactoryClass ctor calls `Set_Contact_Count @ 0x0065AE60` to size dock capacity; war-factory exit and refinery economy invoke HELLO/0x18/`Queue_Mission(0x10)`; receivers gate on `WeaponsFactory`/dock flags | §3.3 idioms 1–2; `RADIO_0X08_TO_0X17_FACTORY_REPAIR_BUNKER_CLEARANCE`, `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH`. |
| `cell-validation` | `UnitClass::Can_Enter_Cell @ 0x0073F0A0` reads the radio Contacts membership (`DynamicVectorClass::Contains`) to apply the `NumberImpassableRows` exit-lane relaxation | `NUMBER_IMPASSABLE_ROWS_RADIO_CONTACT_VECTOR`, `WAR_FACTORY_EXIT_CONTACT_ROW_SKIP` (verified). |
| `damage-helpers` | `FootClass::ReceiveDamage @ 0x004D7330` queues the Rescue(21) mission on AI teammates (retaliation/rescue is a mission transition) | `MISSION_RESCUE_21_HANDLER_AND_AMBUSH_14_STUB` (verified). |
| `abstract-object` | Despawn/limbo/death entry points call `Broadcast_Radio_ToAll(BREAK)` to tear down contacts; mission state is reset on (un)limbo | `GENERIC_DESPAWN_LIMBO_CLEANUP_ENTRY_POINTS`, `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP` (verified). |
| `target-scoring` (weak) | Combat/target-acquisition code reads "current mission / is-busy-idle" to decide retasking and retaliation gating (Rust mirror: combat_targeting busy predicates) | §4.1, §7 retire-list #2/#4 (Rust-side); gamemd reads via `GetCurrentMission`. Lower-confidence edge — see Open. |

---

## Open / unverified edges

- **`random-scenario` RNG attribution** — the scheduler draws ZERO RNG (§5.1.7, HIGH).
  RNG consumed only by individual handlers via per-callsite ECX; per
  `reference_rng_instance_routing_truth`, instance binding is per-callsite (Scen->Random vs
  g_MainRng), so the mission↔RNG edge is real but lives in handlers, not the dispatcher.
  Do not attribute a draw to the scheduler.
- **`target-scoring` ← mission "is-busy"** — the gamemd read site for `GetCurrentMission`
  by target-acquisition was not re-decompiled this session; edge inferred from the Rust
  mirror (combat_targeting.rs busy predicates) + §4.1. Mark MEDIUM until the binary
  call site is named.
- **`ReadyToCommence` subclass overrides (+0x200)** — base = `return 1` (verified
  0x004E0140); the per-type promotion gates that would create additional outgoing edges
  (e.g. unit-at-deploy-cell) are NOT decompiled. Resolve before the verb-API slice.
- **AircraftClass `+0x294` radio-deaf latch setter** — the gate *read* (0x004190B0) is
  verified; *when the latch clears* (candidate: ParaDrop/SpyPlane handlers) is UNCHECKED.
  This is the load-bearing Radio↔Mission coupling.
- **Inferred radio names** (0x07/0x0B/0x0C/0x11/0x16/0x1A/0x1B/0x1D/0x1E) — behavior-guessed,
  not string-confirmed; only 0x01/0x02/0x13/0x24 survive as binary string literals.
- **RadioHistory readers** (+0xD4/D8/DC) — base writes only; no external/base reader found,
  but subclass overrides not exhaustively scanned. UNCHECKED-omittable, not proven-inert.
- **Dead missions** — Ambush(14) confirmed dead TS stub (slot 0x005B2E30 = `return 0x1C2`);
  Rescue(21) is live but AI-only (IsPlayerControl gate). AttackMove(29) has NO dispatch
  case — resolved upstream as a command, never a CurrentMission. Hard parity requirements.
