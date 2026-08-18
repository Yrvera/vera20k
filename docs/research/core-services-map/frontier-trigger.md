# frontier-trigger — Map triggers/tags (TActionClass / TEventClass / TagClass)

**Slug:** `frontier-trigger`
**Status:** promoted from catalog stub (F1 in `_frontier.md`) to STRUCTURAL profile.
**Scope:** STRUCTURAL ONLY per project AI-deferral rule. This profile fixes the service's
owner/global, its place in the verified per-tick spine, its dependency edges, and its
RNG/lockstep relevance. It deliberately does **not** decode the trigger-action decision
logic (which action does what, the condition-matching semantics of each event code, the AI
team/script side). One paragraph of purpose is the cap; the value here is spine
connectivity.

> **Verification provenance (read this).** No Ghidra instance was reachable this session
> (`list_instances` returned `{"instances": []}`; TCP 127.0.0.1:8089 not listening;
> `connect_instance` actively refused). I therefore could **not** independently re-run
> `decompile_function` / `get_function_by_address` on these addresses *this session*. Every
> address below is carried from a recent **Ghidra-verified doc-of-record** that cites its own
> verifying call inline — primarily `core-services-map/_spine-rung-1.md` (Rung A, the per-tick
> driver), `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` (the 28-rung ladder), and the
> trigger-system reports listed in §Sources. The stub's main gap — "per-tick trigger
> evaluation entry unlabeled" — is **resolved by those docs**, not by a live read this
> session. Re-verify live before any implementation.

---

## PURPOSE (one paragraph)

The map scripting engine. Mappers attach **TagClass** instances to cells/objects/houses; each
tag links a **TriggerTypeClass** whose **TEventClass** condition list (entered-by, attacked-by,
destroyed, elapsed-time, credits-exceed, etc.) gates a **TActionClass** action list
(reinforce, play EVA/voice, fire superweapon, reveal map area, win/lose, place overlay,
spawn particle system, etc.). Each game tick the engine scans the global tag array, evaluates
each tag's conditions for a fixed set of event-type codes, and — when a condition passes —
plays the trigger's voice and executes its actions. **Inert in a normal skirmish** (the
global tag array is empty: no author tags), **fully live and player-visible on
campaign/custom scripted maps** (mission objectives, reinforcement waves, EVA narration,
scripted superweapons). It is the live RA2/YR mission-scripting system, **not** TS-legacy —
though some individual *action indices* dispatched downstream were never used by YR maps.

---

## OWNER / GLOBAL (what it owns)

| Thing | Address / offset | Role | Evidence (doc-of-record) |
|---|---|---|---|
| `g_TagClass_Array` (live tags) | base `DAT_008b40cc`, count `DAT_008b40d8` | the global TagClass pointer array the per-tick scan walks | `_spine-rung-1.md` (`get_xrefs_to 0x008b40cc` via `TagClass__Destructor`; `disassemble_function 0x0055AFB0`) |
| Tag scan iterator | `DAT_00a83cdc` | per-tick scan cursor; **reset to 0 in the PerTickUpdate prelude** (`MOV [0x00a83cdc],0 @ 0055afdb`) | `_spine-rung-1.md`, `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md §1` |
| Trigger-event "subject" context | `DAT_00abccd8` | read-only "by whom/on whom" pointer passed into condition eval; written in the LogicClass-AI prologue (`WRITE @ 0x0055aef2`) | `_spine-rung-1.md` (`get_xrefs_to 0x00abccd8`) |
| `g_TagTypeClass_Array` | (array; count `g_TagTypeClass_Array_Count`) | the *type* table (parsed `[Tags]`), classified at map load | `FUN_006E61F0_BRIDGE_LINKED_PREDICATE_GHIDRA_REPORT.md §4/§7` |
| `g_DestroyedEventTagList` | `DAT_008b41a8` (`DynamicVectorClass<TagClass*>`) | map-load bookkeeping list of tags carrying a Destroyed-category event (bit 4) | `FUN_006E61F0_*` §5; `UNREGISTERBRIDGEREPAIRHUT_*` §2.2 |
| Category-E DynVec | `DAT_008b40c8` | secondary per-category tag vector (bit 0x10); populated at load | `FUN_006E61F0_*` §8 (open) |

**Struct field anchors (verified offsets, byte-relative):**

| Struct | Offset | Field | Evidence |
|---|---|---|---|
| TagClass | `+0x24` | linked TriggerType (gate: non-null required to fire) | `_spine-rung-1.md`; `TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_*` |
| TagClass | `+0x34`, `+0x35` | disabled / re-entrancy guard bytes (set `+0x35=1` during walk) | `_spine-rung-1.md` |
| TagClass | `+0x2c` | one-time/AND-fire latch | `_spine-rung-1.md` |
| TagTypeClass | `+0xA0` | head of attached TriggerTypeClass linked list | `FUN_006E61F0_*` §1 |
| TriggerTypeClass | `+0xAC` | event-list head (TEventClass nodes; next at `+0x28`) | `FUN_006E61F0_*` §2 |
| TriggerTypeClass | `+0xB0` | action-list head (TActionClass nodes) | `FUN_006E61F0_*` §2 |
| TriggerTypeClass | `+0xA8` | next-TriggerType (compound triggers) | `FUN_006E61F0_*` §1/§2 |
| TriggerType | `+0x9c` | repeat/persist type (0 OR-fire / 1 AND-fire / 2 repeating) — drives queueing | `_spine-rung-1.md` |

---

## KEY FUNCTIONS (re-stated from docs-of-record; live re-verify pending)

| Function | Address | Role | Evidence |
|---|---|---|---|
| **Per-tick trigger driver** (TagClass per-event trigger eval) | **`0x006e53a0`** | for one Tag, walks its trigger condition list, plays voice + queues/executes actions if a condition for the given event-code passes; sets `+0x35` recursion guard; gated `g_IsMapEditor==0 && Tag+0x34==0 && Tag+0x35==0 && Tag+0x24!=0` | `_spine-rung-1.md` (`decompile_function 0x006e53a0`, `get_function_callees 0x006e53a0`); `TECHNOCLASS_PROCESSCELLACTION_*` |
| Condition evaluator | `0x007264c0` | `TriggerActionEntry__EvaluateConditions` — matches stored condition-event vs the passed event code; calls `HouseClass__Find_By_Country_Index` + `TriggerCondition__Evaluate` | `_spine-rung-1.md` (`decompile_function 0x007264c0`) |
| Voice/action player | `0x007265c0` | `TriggerActionEntry__PlayVoiceForObjects` — loops action list, calls `TriggerAction__Execute` | `_spine-rung-1.md` (`decompile_function 0x007265c0`) |
| **Action dispatcher** | `0x006dd8b0` | `TriggerAction__Execute` — the big switch over action codes (the per-action effect bodies) | `_spine-rung-1.md` (`get_function_callees 0x006dd8b0`); many action reports below |
| Detach helper | `0x007258d0` | `Detach_From_All_Lists` on loop end | `_spine-rung-1.md` |
| TagType event-category classifier | `0x006e61f0` | `TagTypeClass::GetEventCategoryBitmask` (load-time tag categorization) | `FUN_006E61F0_*` §1 |
| TriggerType event classifier (recursive) | `0x007271e0` | `TriggerTypeClass::GetEventCategoryBitmask` | `FUN_006E61F0_*` §2 |
| TriggerClass constructor | `0x00725fa0` (stub-claimed) | runtime trigger instance ctor — **UNRE-VERIFIED this session** (stub value, no live read available) | stub F1 only |
| TEventClass constructor | `0x0071e6a0` (stub-claimed) | event node ctor — **UNRE-VERIFIED this session** | stub F1 only |

> **Stub correction.** The stub listed `TriggerClass__Constructor`/`TEventClass__Constructor`
> as the representative functions and flagged "per-tick trigger evaluation entry unlabeled."
> The constructors are load-time, not the spine entry. The **actual representative function
> for spine connectivity is the per-tick driver `0x006e53a0`** (Rung A), now located. The
> Ghidra label on `0x006e53a0` is `TechnoClass__ProcessCellAction` — **drifted/misleading**:
> it is not a TechnoClass method and is unrelated to sidebar/placement cell-action scanning;
> it is a TagClass per-event trigger-evaluation method (`this = TagClass`). Trust the body,
> not the name.

---

## TICK PLUG POINT — Rung A (first rung of the 28-rung spine)

**Rung A** of `LogicClass::PerTickUpdate @ 0x0055AFB0` (the leading rung, `0055afe6`–`0055b177`),
sole caller `Main_Tick @ 0x0055D360`. Per the spine spec table:

> | # | Label | Driver | Walks | Gate |
> |---|---|---|---|---|
> | 1 | **A. Tag/map-trigger event scan + SW-ready poll** | `0x006e53a0` | global TagClass array `DAT_008b40cc` count `DAT_008b40d8`; event codes 0x32/0x1b/0x1c/0x24/0x25/0x2d/0x2e/0xd/0x33/0xe | entry: `DAT_008b40d8 > 0`; per-event sub-gates on Scen bytes `+0x34be/+0x34aa/+0x34ab` and the `+0x11e8` SW timer |

Mechanics (verified, `disassemble_function 0x0055AFB0`): the prelude resets the scan iterator
(`MOV [0x00a83cdc],0 @ 0055afdb`); the rung gate is the tag count `DAT_008b40d8 > 0` (the whole
scan is `JLE`-skipped when zero — i.e. **skipped every tick in skirmish**). Per tag, a
short-circuit cascade of driver calls runs, each pushing a different **event-type code** (0x32,
then 0x1b/0x1c/0x24/0x25, then 0x2d/0x2e, then unconditional 0xd/0x33, plus a `0xe` SW-ready
poll gated on the `Scen+0x11e8` timer — the *same* timer slot Rung B consumes). Receiver is the
TagClass (`ECX`), context arg is `DAT_00abccd8`.

**This places `frontier-trigger` at the very top of the gameplay tick, before all object AI
(Rung T) and economy (Rungs Z/AA).** It is the first thing the lockstep ladder does after the
two-line prelude.

**Render-side note:** distinct from Rung A, the global function `Map__Logic @ 0x004D2370`
(NOT a MapClass method) runs in the **Main_Tick prelude** (before PerTickUpdate) and walks a
trigger/tag/waypoint object array to OR cell highlight flag `0x400000` into `cell.flags[0x140]`
for tag/area-trigger cells (object type 6 → expand point list) — i.e. the tag/area-trigger
**editor/visual highlight** pass, not the firing path. Evidence: `LOGICCLASS_VS_MAPCLASS_*` §6.

---

## OUTGOING EDGES (this service → others)

All via `TriggerAction__Execute @ 0x006dd8b0` action cases (the firing path) plus the per-tick
driver's direct callees. Each edge cites the action code + the verifying doc.

| → Service | via (symbol / action) | Evidence |
|---|---|---|
| `logicclass` | runs as **Rung A** of `LogicClass::PerTickUpdate`; firing order is the lockstep contract | `_spine-rung-1.md`; `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` |
| `factory-house` | `HouseClass__Find_By_Country_Index` (condition + action house resolution); sell/online/offline + credit/production actions | `_spine-rung-1.md` (callees); `SIDECLASS_*` §10.2 (TEvent house-owned-count predicate `0x0041ec90`) |
| `cell-map` | `MapClass__Get_CellClass @ 0x005657a0` (driver callee); "change visible map area" action `FUN_006e21e0` rewrites playfield rect + RecalcAttributes | `_spine-rung-1.md`; `CELLCLASS_PLAYFIELD_BOUNDS_*` §5 |
| `cell-validation` / overlay | action 0x6C places an overlay at a waypoint cell (`WallOverlay_HeightAdjust @ 0x0056BEC0`) | `OVERLAYWALL_PLACEMENTSHADOW_*` §2.2 |
| `damage-helpers` | trigger area-damage actions `FUN_006e0490` (case 0x3F) / `FUN_006e2050` / `FUN_006e2390` (case 0x2A) → `Apply_area_damage`; bridge-destroy via direct ApplyDamageToCell | `SUPERWEAPON_BRIDGE_AOE_IMPACT_Z_THREADING_*` §3; `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_*` §8 |
| `bridge-helpers` | broadcasts trigger **events** 0x1F (BridgeDestroyed) / 0x30 (BridgeRepaired) — the *consumer* side of bridge state changes (these are events delivered TO triggers) | `TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_*`; `BRIDGE_DESTROYED_TRIGGER_EVENT_0X1F_*` |
| `frontier-audio-voc` | action 0x13 "Play Sound Effect" → `VocClass__PlayAtPos(idx,1.0,0)` | `SOUND_TRIGGERS_COMPLETE_*` §22 |
| `frontier-audio-eva` | action 0x15 "Play Speech/EVA" → `VoxClass__QueueVoice(idx,-1)` | `SOUND_TRIGGERS_COMPLETE_*` §22; `EVA_SYSTEM_DEEP_DIVE_*` §4 (EVA_MissionAccomplished etc. via trigger action) |
| `frontier-particle` | action 0x58 "Particle System at Waypoint" — the only PSC-spawning action case | `PARTICLESYSTEMCLASS_*` §11.8.B |
| `frontier-super` | superweapon-launch trigger actions (Ion/PD area helpers case 0x2A/0x3F; chrono warp via `ChronoSphere__WarpUnitsAtCell` in the callee set) | `SUPERWEAPON_BRIDGE_AOE_IMPACT_Z_THREADING_*` §3; `_spine-rung-1.md` (callee set) |
| `frontier-radar` | `CreateRadarEvent` in the action callee set (radar ping actions) | `_spine-rung-1.md` (callee set of `0x006dd8b0`) |
| `frontier-ai-team` | reinforcement / team-create / waypoint trigger actions call team+waypoint helpers (`0x006e0aa0`, `0x006e0fe0`, `0x006e11c0`, `0x006e2050`) | `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_*` §5 |
| `random-scenario` | **transitive only** — RNG drawn iff a fired action is itself RNG-drawing (chrono warp, spawn, area-damage debris roll); 0 draws in skirmish (rung inert) | `_spine-rung-1.md §RNG` |

---

## INCOMING EDGES (others → this service)

| From service | via | Evidence |
|---|---|---|
| `logicclass` | drives Rung A each tick (the firing scan) | `_spine-rung-1.md` |
| `frontier-mix-vfs` / `ini-parsing` | `[Tags]` / `[Triggers]` / `[CellTags]` / `[Events]` / `[Actions]` map sections parsed at map load populate `g_TagClass_Array`, `g_TagTypeClass_Array`, and classify into `g_DestroyedEventTagList` (`FUN_00684C30` post-`.MAP` parse) | `BRIDGE_DESTROYED_TRIGGER_EVENT_0X1F_*` §4; `FUN_006E61F0_*` §4/§7 |
| `bridge-helpers` | bridge-span collapse broadcasts **event 0x1F** to each tagged cell via the per-techno/per-cell path `0x006e53a0` (`RepairBridgeSegment @ 0x00575EE0`) | `BRIDGE_DESTROYED_TRIGGER_EVENT_0X1F_*`; `TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_*` |
| `techno-foot` (infantry) | engineer entering bridge-repair hut broadcasts **event 0x30** via `0x006e53a0` from `InfantryClass::PerCellProcess @ 0x00519630` | `TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_*` §4 |
| object cell-action callers (e.g. BuildingLightClass) | the same driver `0x006e53a0` is the generic attached-trigger/event processor invoked with object cell action `0x23` | `BUILDINGLIGHTCLASS_BEAM_RASTERIZATION_AND_CELLACTION_0X23_*` §3.4 |
| `cell-map` | `[CellTags]` binds cells to tag IDs (`CellClass+0x3C` attached-tag pointer); the "cells with attached object" registry (MapClass `+0x115C` DynVec) is the index used to find tagged cells | `BRIDGE_DESTROYED_TRIGGER_EVENT_0X1F_*` §4; `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §5 |

> Note `0x006e53a0` has **two caller classes**: (1) the **Rung-A periodic scan** over the
> whole tag array each tick, and (2) **event broadcasters** (bridge collapse 0x1F, engineer
> hut 0x30, building-light cell action 0x23) that fire it on demand for a single tagged
> entity. Same driver, two delivery paths. The periodic scan is the spine plug point.

---

## RNG / LOCKSTEP RELEVANCE

- **Rung A draws zero RNG in the common case.** The driver and its direct condition/voice
  callees (`0x007264c0`, `0x007265c0`) draw **no** RNG (verified `decompile_function` per
  `_spine-rung-1.md §RNG`).
- RNG is consumed **transitively only** when a fired action inside `TriggerAction__Execute @
  0x006dd8b0` is itself RNG-drawing (chrono warp, spawn helpers, area-damage debris rolls).
  That is conditional on an author-placed trigger firing this tick — **0 draws in a normal
  skirmish** because the rung is skipped entirely (`DAT_008b40d8 == 0`).
- **Lockstep position is load-bearing on scripted maps:** because Rung A is the *first* rung,
  any RNG a fired action draws is drawn **before** all object-AI (Rung T) and economy (Rung
  AA) draws. The firing order — tag-array order, then per-tag event-code cascade order, then
  per-tag action-list order — is part of the lockstep contract. Which stream a fired action
  uses is per-callsite ECX inside `0x006dd8b0` (not the driver's own stream) and is out of
  scope for this structural profile.

---

## ACTIVE-IN-YR / TS-LEGACY

- **Mechanism: live in YR, not TS-legacy.** The TagClass/TriggerType/TEvent/TAction
  scripting engine is the standard RA2/YR mission system. Rung A is reachable and
  player-visible whenever a map defines tags.
- **Skirmish: inert.** Vanilla multiplayer/skirmish maps carry no author tags →
  `DAT_008b40d8 == 0` → Rung A `JLE`-skipped every tick. The whole service contributes
  nothing observable in a stock skirmish. **Trigger-frequency for parity prioritization:
  every tick on campaign/scripted maps; never on stock skirmish maps.**
- **Per-action liveness varies.** Individual action indices dispatched by `0x006dd8b0`
  include some TS-era codes never used by YR maps (e.g. `IonBlastClass` is fully removed from
  the live switch — `ION_BLAST_CLASS_*` §OQ1). Per-action liveness is out of scope here
  (AI/scripting decode is deferred); flag for the eventual full study.

---

## Sources (docs-of-record; each cites its own verifying Ghidra call inline)

- `docs/research/core-services-map/_spine-rung-1.md` — Rung A driver `0x006e53a0`, tag array,
  iterator reset, gate, callee set, RNG analysis (primary).
- `docs/research/LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` — the 28-rung ladder; Rung A row.
- `docs/research/LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md` §3/§6 — trigger-action dispatch loop
  inside PerTickUpdate; `Map__Logic @ 0x004D2370` highlight pass.
- `docs/research/bridges/07-cross-system-consumers/FUN_006E61F0_BRIDGE_LINKED_PREDICATE_GHIDRA_REPORT.md`
  — TagTypeClass/TriggerTypeClass struct offsets; `g_DestroyedEventTagList`; load-time classify.
- `docs/research/TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_GHIDRA_REPORT.md` — driver as
  FireTriggerAction; events 0x1F/0x30; label-drift confirmation.
- `docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_DESTROYED_TRIGGER_EVENT_0X1F_GHIDRA_REPORT.md`
  — INI map keys (`[CellTags]`/`[Tags]`/`[Triggers]`), cell attached-tag gate.
- `docs/research/SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md` §22 — action 0x13 (Voc) / 0x15 (Vox).
- `docs/research/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §11.8.B — action 0x58 (particle system).
- `docs/research/OVERLAYWALL_PLACEMENTSHADOW_AND_HEIGHTADJUST_GHIDRA_REPORT.md` §2.2 — action 0x6C (overlay).
- `docs/research/CELLCLASS_PLAYFIELD_BOUNDS_FROM_LOCALSIZE_GHIDRA_REPORT.md` §5 — change-map-area action.
- `docs/research/SUPERWEAPON_BRIDGE_AOE_IMPACT_Z_THREADING_GHIDRA_REPORT.md` §3 — Ion/PD area trigger actions.
- `docs/research/NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` §5 — team/waypoint trigger helpers.
- `docs/research/SIDECLASS_GHIDRA_REPORT.md` §10.2 — TEvent house-owned-count predicate `0x0041ec90`.
- `docs/research/BUILDINGLIGHTCLASS_BEAM_RASTERIZATION_AND_CELLACTION_0X23_GHIDRA_REPORT.md` §3.4 — driver as generic cell-action 0x23 processor.

**LIVE-VERIFY TODO (next session with Ghidra up):** re-confirm `0x006e53a0` body + receiver,
`get_function_callers 0x006e53a0` (Rung-A scan vs event broadcasters), the `0x006dd8b0` switch
extent, and re-verify the two stub-only constructor addresses `0x00725fa0` / `0x0071e6a0`.
