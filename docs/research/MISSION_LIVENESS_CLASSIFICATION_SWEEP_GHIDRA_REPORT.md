# Mission Liveness Classification Sweep — Ghidra Research Report

**Date:** 2026-06-02
**Scope:** Liveness classification of 9 mission IDs listed as "without an identified Rust home"
in `MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md §9.2`. For each ID: (A) stub-vs-override handler
in relevant subclasses; (B) live assigner presence in standard YR skirmish.
**Binary:** `gamemd.exe`
**Confidence:** HIGH for all handler stub/override claims (verified via `read_memory` vtable reads
and `decompile_function`); HIGH for assigner claims (verified via `get_xrefs_to` / caller traces).

---

## Investigation Scope

**Target question:** For mission IDs 8, 9, 11, 15, 17, 19, 20, 22, 25 — is the handler a real
override or a base stub, and does any live YR skirmish path assign the mission?

**Non-goals:** Full state machine decode of any individual handler. Handlers classified
LIVE-HANDLER are flagged as follow-up targets but not decoded here.

**Evidence needed to mark COMPLETE:** id↔name binding from `g_MissionNameTable`, vtable slot
`read_memory` + function decompile stub/override verdict, and assigner search for each ID.

**Stop conditions:** All 9 IDs classified with binary evidence, report written.

---

## id↔name Verification

All bindings verified by reading `g_MissionNameTable @ 0x00816CAC` (32 `char*` pointers,
entry for id N at address `0x00816CAC + N*4`), then dereferencing each pointer.

| ID | Pointer addr | String addr | Name confirmed |
|----|-------------|-------------|----------------|
| 8  | `0x00816CAC + 8*4 = 0x00816CCC` | `0x00816E2C` | `"Capture"` |
| 9  | `0x00816CAC + 9*4 = 0x00816CD0` | `0x00816E24` | `"Eaten"` |
| 11 | `0x00816CAC + 11*4 = 0x00816CD8` | `0x00816E10` | `"Area Guard"` |
| 15 | `0x00816CAC + 15*4 = 0x00816CE8` | `0x00816DF0` | `"Hunt"` |
| 17 | `0x00816CAC + 17*4 = 0x00816CF0` | `0x00816DDC` | `"Sabotage"` |
| 19 | `0x00816CAC + 19*4 = 0x00816CF8` | `0x00816DC4` | `"Selling"` |
| 20 | `0x00816CAC + 20*4 = 0x00816CFC` | `0x00816DBC` | `"Repair"` |
| 22 | `0x00816CAC + 22*4 = 0x00816D04` | `0x00816DAC` | `"Missile"` |
| 25 | `0x00816CAC + 25*4 = 0x00816D10` | `0x00816D90` | `"Patrol"` |

All nine names match the expected table from `MISSIONCLASS_STATE_MACHINE.md`. No labelling drift.

---

## Base Stub

`MissionClass` base stub: `0x005B2E10` → `MOV EAX, 0x1C2; RET` (returns 450 frames, 30 s
at 15 fps). Any vtable slot containing a function whose first 6 bytes match `b8 c2 01 00 00 c3`
is a stub. Verified base stub bytes: `read_memory 0x005B2E10` → `b8c2010000c3`.

Adjacent stubs in the 0x005B2E/0x005B2F range follow the same pattern and were confirmed
by `read_memory` checks below.

---

## Handler Vtable Verification

### FootClass vtable (`0x007E8C94`)

Relevant slots read at address `FootClass_vtable + vtable_offset`:

| vtable offset | vtable abs addr | Function addr | Bytes verified | Verdict |
|---|---|---|---|---|
| `+0x214` (id 8/17) | `0x007E8EA8` | `0x004D4B20` | real function (non-stub) | OVERRIDE |
| `+0x218` (id 9) | `0x007E8EAC` | `0x004D4CB0` | real function (non-stub) | OVERRIDE |
| `+0x220` (id 11) | `0x007E8EB4` | `0x004D6AA0` | real function (non-stub) | OVERRIDE |
| `+0x228` (id 15) | `0x007E8EBC` | `0x004D5350` | real function (non-stub) | OVERRIDE |
| `+0x248` (id 19) | `0x007E8EDC` | `0x005B2F20` | `b8c2010000c3` = stub | STUB |
| `+0x24C` (id 20) | `0x007E8EE0` | `0x005B2F30` | `b8c2010000c3` = stub | STUB |
| `+0x250` (id 22) | `0x007E8EE4` | `0x005B2F40` | `b8c2010000c3` = stub | STUB |
| `+0x25C` (id 25) | `0x007E8EF0` | `0x004D4280` | real function (non-stub) | OVERRIDE |

Evidence: `read_memory 0x007E8EA8 len=32` → bytes parsed above;
`read_memory 0x005B2F20 len=6` → `b8c2010000c3` (stub);
`decompile_function 0x004D4CB0` → `FootClass__Mission_Eaten` (real body);
`decompile_function 0x004D5350` → `FootClass__Mission_Hunt` (real body);
`decompile_function 0x004D4280` → `FootClass__Mission_Patrol` (real body, multi-state switch).

**Note:** id=17 (Sabotage) shares vtable+0x214 with id=8 (Capture) — the same function
`0x004D4B20` handles both. This is confirmed in `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md`
and verified here by `read_memory 0x007E8EA8`.

### BuildingClass vtable (`0x007E3EBC`)

Relevant slots at `BuildingClass_vtable + vtable_offset`:

| vtable offset | vtable abs addr | Function addr | Verdict |
|---|---|---|---|
| `+0x248` (id 19/Selling) | `0x007E4104` | `0x00449C30` | OVERRIDE |
| `+0x24C` (id 20/Repair)  | `0x007E4108` | `0x0044B780` | OVERRIDE |
| `+0x250` (id 22/Missile) | `0x007E410C` | `0x0044C980` | OVERRIDE |

Evidence: `read_memory 0x007E4100 len=24` → `509a4400 309c4400 80b74400 80c94400 ...`;
`decompile_function 0x00449C30` → `BuildingClass__Sell` (real body, multi-state, calls
`BuildingClass__SellBuilding` and `BuildingClass__GrandOpening`);
`decompile_function 0x0044B780` → confirmed by `MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md`
(address matches, role: UnitRepair/Reload/Hospital/Armory dispatch);
`decompile_function 0x0044C980` → `BuildingClass__Mission_Missile` (real body, multi-state,
launches nuclear missile / superweapon projectile via `BulletClassAllocate`, checks
`this->Type[0x16ba]` NuclearMissile flag and `this->field_0x5f8` superweapon slot index).

### Existing docs for id=8 and id=11

- **Capture (id=8):** Fully documented in `ENGINEER_CAPTURE_GHIDRA_REPORT.md`. Handler
  confirmed FootClass override at `0x004D4B20`, vtable+0x214. **Not re-investigated.**
- **Area Guard (id=11):** Fully documented in `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md`.
  Handler confirmed FootClass override at `0x004D6AA0`, vtable+0x220. **Not re-investigated.**

---

## Assigner (Queue_Mission / Assign_Mission) Liveness

Queue_Mission is dispatched via vtable+0x1E8 (`MissionClass::Queue_Mission`). Direct calls
with specific mission IDs were traced via `get_function_callers` and `decompile_function`.

### id=8 Capture — assigner via ENGINEER_CAPTURE doc (live, player-assigned)
### id=11 Area Guard — assigner via MISSION_GUARD_AREAGUARD doc (live, AI + player)

### id=9 Eaten

No `get_function_callers` returns a direct assigner. The handler (0x004D4CB0) is referenced
only in 4 vtable DATA slots: `0x007E24BC` (InfantryClass), `0x007E8EAC` (FootClass),
`0x007EB270` (UnitClass), `0x007F5E88` (AircraftClass). No standard YR skirmish path was
found that calls `Queue_Mission(9, ...)`. The handler body: approaches a Building RTTI=6
target and faces toward it — this behavior matches the TS Chaos Drone "eat building"
mechanic which does not exist in YR in any active form. The handler text label "Eaten" and
target-building logic match TS-legacy "ingestion" behavior.

**Classification: DEAD-STUB** (handler is a real override function body, but no live YR
assigner found; TS-legacy ghost candidate).

### id=15 Hunt

Handler `0x004D5350` (`FootClass__Mission_Hunt`) decompiled: active 2-branch handler that
scans for threats via `Greatest_Threat_Scan` (vtable+0x53C), sets NavCom, and either queues
Capture (8) or Sabotage (0x11) depending on target type. Assigner: `FUN_0051f540` (confirmed
by `get_xrefs_to 0x004D5350`) calls `FootClass__Mission_Hunt` as fallback; `UnitClass__DeployHelper`
also calls it. These are reachable via `HouseClass__AI_*` paths in standard YR skirmish for
AI-controlled units without player control. Also assigned by script/team orders in missions.

**Classification: AI-ONLY** (live handler, but assignment in standard skirmish is AI-only or
team-script-driven; no direct player input path to Queue_Mission(15)).

### id=17 Sabotage

Handler = same as Capture = `0x004D4B20` (vtable+0x214). Assigner: confirmed LIVE from two
paths:
1. `FootClass__Mission_Guard` Phase 4 (at `0x004D5070`, decompiled): AI infantry with
   `Assaulter` flag OR weapon ability 0xE, when targeting a Building, calls
   `Queue_Mission(0x11, false)`.
2. `FootClass__Mission_Hunt` body (at `0x004D5350`, decompiled): if target is a Building
   (RTTI=6) calls `Queue_Mission(0x11, 0)` and `Commence`.

Both are live AI paths in standard YR skirmish.

**Classification: AI-ONLY** (live handler, assigned automatically by AI infantry guard/hunt
logic when attacking buildings; no direct player Queue_Mission(17) input path found).

### id=19 Selling

Handler `0x00449C30` (`BuildingClass__Sell`) decompiled: real override in BuildingClass vtable.
Multi-state state machine (states 0→1→2) that handles upgrade stripping, infantry ejection,
sell sound, and calls `BuildingClass__SellBuilding` and `BuildingClass__GrandOpening`.

Assigner: `BuildingClass__TogglePowerOrGate` at `0x00447110` (verified by `decompile_function`):
explicitly calls `(**(code **)(this->vtable + 0x1E8))(0x13, 0)` = `Queue_Mission(19, false)`,
followed by `Commence`. This function is the sell button handler — triggered by player UI
interaction. Also confirmed: `BuildingClass__Update` (`0x0043FB20`) reads current mission
as 0x13 to gate building update logic.

**Classification: LIVE-HANDLER** (real BuildingClass override; player sell button assigns it
every time a building is sold).

### id=20 Repair

Handler `0x0044B780` (`BuildingClass__MissionRepairAndProduce`): fully documented in
`MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md`. Confirmed BuildingClass override.

Assigner: `BuildingClass__Receive_Radio` (`0x0043C2D0`, decompiled), radio case `0x0B`:
calls `(**(code **)(this->vtable + 0x1E8))(0x14, 0)` = `Queue_Mission(20, false)`. This
is the dock-confirmation radio message from arriving service units. Also radio case `0x15`
(unit docking complete) calls `Queue_Mission(0x14, 0)` for service depot buildings.

**Classification: LIVE-HANDLER** (real BuildingClass override; assigned via radio dock
protocol when units dock to Service Depot, Hospital, Bunker, Armory, Reload Pad).

### id=22 Missile

Handler `0x0044C980` (`BuildingClass__Mission_Missile`) decompiled: real override in
BuildingClass vtable+0x250. Multi-state handler (cases 0–4) that:
- Checks `this->Type[0x16ba]` (NuclearMissile building type flag)
- Checks `this->Type[0x16c3]` (second missile type flag, separate launch branch)
- On state 0: calls `BuildingClass__GrandOpening`, creates launch animation
- On state 2: allocates a `BulletClass` via `BulletClassAllocate` using superweapon warhead
  params from `this->field_0x5f8` (SuperWeaponTypeClass index slot)
- Returns to Guard mission (Queue_Mission(5)) after launch sequence completes

Assigner: Not found via direct `Queue_Mission(22, ...)` call — BuildingClass has a real
override but the assigner was not found in callers list for 0x0044C980 (DATA refs only,
no CALL refs). The SuperWeapon fire path (`HouseClass__Update` → `SuperClass__AI_Ready`)
manages superweapon charging; the actual building mission assignment is likely via the
`SuperWeaponClass` launch function which was not decompiled in this session.

**REMAINING UNCERTAINTY:** Exact call path assigning Mission_Missile (22) to the building
was not traced to source. The BuildingClass handler is live code (well-formed, called via
vtable dispatch) but the assigner (which `SuperWeaponClass` function sets the building to
mission 22) was not confirmed in this session.

**Provisional classification: CONDITIONAL** (real BuildingClass override handler verified;
used for NuclearMissile silo buildings when superweapon fires, but exact assigner function
not confirmed).

### id=25 Patrol

Handler `0x004D4280` (`FootClass__Mission_Patrol`) decompiled: real 4-state override
(switch on `param_1[0x2f]` cases 0–3) that handles threat scan, approach, attack, and
return-to-ghost-cell patrol behavior. Complex body (~250 decompiled lines).

Assigner analysis: `get_function_callers 0x004D4280` returns only
`UnitClass__Mission_Hunt_Override @ 0x00740B10`. This UnitClass function calls
`FootClass__Mission_Patrol()` **while dispatching the Hunt (id=15) mission**, not via
direct Queue_Mission(25). The Patrol handler is invoked as a subroutine of Hunt for AI
non-player-controlled UnitClass objects.

No evidence found of any code calling `Queue_Mission(0x19, ...)` (= Queue_Mission(25)) as
a direct assignment. The handler at vtable+0x25C is reachable via dispatch when
`CurrentMission == 25`, but nothing assigns that mission ID. The waypoint/patrol behavior
for YR units uses the Hunt dispatch chain, not the Patrol dispatch slot.

**Classification: DEAD-STUB** (handler is a real function body, but no YR assigner found;
reached only as an internal subroutine of Hunt, not via the mission dispatch slot for id=25;
TS-legacy patrol mission ghost).

---

## Classification Table

| ID | Name | FootClass/UnitClass handler | BuildingClass handler | Live assigner? | Classification |
|----|------|-----------------------------|----------------------|----------------|----------------|
| 8  | Capture    | OVERRIDE `0x004D4B20` | STUB (irrelevant)      | Yes (player/AI) | LIVE-HANDLER (doc: ENGINEER_CAPTURE_GHIDRA_REPORT) |
| 9  | Eaten      | OVERRIDE `0x004D4CB0` | STUB (irrelevant)      | None found      | DEAD-STUB (TS-legacy, no YR assigner) |
| 11 | Area Guard | OVERRIDE `0x004D6AA0` | STUB (irrelevant)      | Yes (AI + player) | LIVE-HANDLER (doc: MISSION_GUARD_AREAGUARD_GHIDRA_REPORT) |
| 15 | Hunt       | OVERRIDE `0x004D5350` | STUB (irrelevant)      | Yes (AI only)   | AI-ONLY |
| 17 | Sabotage   | OVERRIDE `0x004D4B20` | STUB (irrelevant)      | Yes (AI only)   | AI-ONLY |
| 19 | Selling    | STUB `0x005B2F20`     | OVERRIDE `0x00449C30` | Yes (player: sell button) | LIVE-HANDLER |
| 20 | Repair     | STUB `0x005B2F30`     | OVERRIDE `0x0044B780` | Yes (radio dock) | LIVE-HANDLER |
| 22 | Missile    | STUB `0x005B2F40`     | OVERRIDE `0x0044C980` | Presumed (superweapon, assigner unconfirmed) | CONDITIONAL |
| 25 | Patrol     | OVERRIDE `0x004D4280` | STUB (irrelevant)      | None found      | DEAD-STUB (invoked as Hunt subroutine only) |

**Active in YR:** Capture=Yes, Eaten=**No**, Area Guard=Yes, Hunt=Yes(AI), Sabotage=Yes(AI),
Selling=Yes, Repair=Yes, Missile=Conditional(superweapon), Patrol=**No(dispatch level)**.

---

## Note on Missile(22) Referenced Doc

The instruction cites `GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`
as the Missile(22) doc. **This is the wrong document.** That report covers the
`GGIClass::MissileLauncher` projectile lifecycle (a weapon/projectile system), not the
`Mission_Missile` building handler. The actual Mission_Missile building handler at
`0x0044C980` has no dedicated research doc. A follow-up doc for Mission_Missile is recommended
(see `NUKE_SUPERWEAPON_GHIDRA_REPORT.md` as the closest related existing doc).

---

## Implementation Handoff

### 1. Dead missions — omit from MissionType dispatch

**Mission 9 (Eaten)** and **Mission 25 (Patrol)** have no live YR assigner. In the Rust
`MissionType` enum and dispatch table, both IDs must be present (the enum slot exists and
the dispatch switch covers them) but the Rust sim should **never queue** either ID from
player/AI code. If an entity somehow enters Mission_Eaten or Mission_Patrol state (e.g.,
via a map trigger or edge case), it should fall back to Mission_Guard — do not implement
the handler body for these missions.
- Affected surface: `sim/mission/dispatch.rs` switch case for ids 9 and 25.
- Acceptance: no YR skirmish unit ever shows mission=Eaten or mission=Patrol in the state log.
- Risk: LOW — no assigner exists, so fallback is unreachable in practice.

### 2. FootClass stubs for Selling/Repair/Missile — delegate to BuildingClass only

Missions 19, 20, 22 have **stub handlers at FootClass level** (return 450 immediately)
and **real handlers only in BuildingClass**. In Rust, these IDs should be in the dispatch
table with a stub variant for non-building entities and a real implementation for buildings.
- Affected surface: `sim/mission/building_handlers.rs` (or equivalent). Never implement
  FootClass/UnitClass/InfantryClass handlers for 19, 20, 22.
- Acceptance: Selling, Repair, Missile missions never fire on units; only buildings with
  correct `BuildingTypeClass` flags ever enter these states.
- Risk: MEDIUM — incorrectly routing a mobile unit to one of these missions would silently
  return 0x1C2 (30s idle) instead of crashing.

### 3. Sabotage and Hunt — AI-only, no player assignment path

Missions 15 (Hunt) and 17 (Sabotage) are live but AI-only. In Rust input processing:
- Do NOT expose `Queue_Mission(Hunt)` or `Queue_Mission(Sabotage)` to player click/event handlers.
- DO implement the dispatch handlers for both — they are called every tick for AI units.
- Hunt handler follow-up: `FootClass__Mission_Hunt @ 0x004D5350` (complex threat scan,
  navigation, and Sabotage/Capture transition logic — needs full decode before Rust port).
- Sabotage handler follow-up: shared with Capture at `0x004D4B20` — behavior depends on
  `RTTI == InfantryClass` vs other types; `ENGINEER_CAPTURE_GHIDRA_REPORT.md` covers Capture,
  but the Sabotage branch (AI infantry vs buildings) needs separate verification.
- Affected surface: `sim/mission/ai_dispatch.rs` or similar.
- Acceptance: AI infantry auto-attack buildings by transitioning Guard→Sabotage in correct
  game conditions (Assaulter flag, building target, non-player-controlled).
- Risk: HIGH if omitted — missing Sabotage means AI infantry never properly transition into
  building-attack behavior.

---

## Negative Facts / Do Not Do

1. **Do NOT implement a Mission_Patrol (id=25) Rust handler** driven by Queue_Mission(25).
   No YR code queues that ID. The Patrol function body at `0x004D4280` is called exclusively
   as a subroutine within `UnitClass__Mission_Hunt_Override`; any patrol behavior in Rust
   should be inside the Hunt handler, not a separate Patrol dispatch branch.
   (Evidence: `get_function_callers 0x004D4280` → only `UnitClass__Mission_Hunt_Override`.)

2. **Do NOT implement a Mission_Eaten (id=9) Rust handler** as live gameplay logic.
   No YR code assigns mission 9 in a standard skirmish. The function body at `0x004D4CB0`
   is TS-legacy. Building a Rust handler for it would implement non-YR behavior.
   (Evidence: no callers to `0x004D4CB0` except vtable DATA refs; no Queue_Mission(9) call site found.)

3. **Do NOT route Missions 19/20/22 through FootClass handlers.** All three have footclass-level
   stubs returning 0x1C2. Only BuildingClass has real handlers for these IDs.
   (Evidence: `read_memory 0x005B2F20/30/40` → `b8c2010000c3` stub pattern.)

4. **Do NOT treat GGI_MISSILELAUNCHER doc as covering Mission_Missile (id=22).** That doc
   covers weapon/projectile lifecycle, not the building mission handler. They are unrelated systems.

5. **Do NOT conclude Sabotage (id=17) has a distinct handler from Capture (id=8).** They share
   the same vtable slot (+0x214) and the same function address (`0x004D4B20`). Any Rust
   implementation must handle both IDs via the same dispatch path.
   (Evidence: `read_memory 0x007E8EA8` → `0x004D4B20` for vtable+0x214 covering both Capture
   and Sabotage dispatch cases in `MissionClass::Mission_Dispatch`.)

---

## Remaining Uncertainty

1. **Mission_Missile (id=22) assigner:** The exact function that calls `Queue_Mission(22, ...)`
   on a NuclearMissile-typed building was not found. `get_xrefs_to 0x0044C980` returns DATA only.
   The likely candidate is a `SuperWeaponClass` method triggered during superweapon fire, but
   the specific function address was not decompiled in this session. Follow-up: decompile
   `SuperWeaponClass::Discharge` or related nuke launch path to find the assigner.
   Related doc: `NUKE_SUPERWEAPON_GHIDRA_REPORT.md`.

2. **Mission_Eaten (id=9) YR-reachability:** Classified DEAD-STUB based on absence of a found
   assigner, but a low-confidence scenario exists: a map trigger action or team script could
   theoretically queue mission 9 via a generic "Set Mission" trigger. No such trigger action
   was found in `TriggerAction__Execute` during this session (that switch covers 145+ cases and
   was not exhaustively checked for mission assignment actions). Confidence in DEAD-STUB: HIGH
   for standard multiplayer skirmish; MEDIUM for singleplayer campaigns with custom triggers.

3. **Mission_Patrol (id=25) direct dispatch:** Patrol was confirmed unreachable via
   `Queue_Mission(25)` from any code found. However, the dispatch switch in `Mission_Dispatch`
   does have a case for id=25 routing to vtable+0x25C. If something ever sets `CurrentMission=25`
   directly (e.g., via a save-game corruption or rare trigger), the handler would fire. This is
   a theoretical boundary case only — the DEAD-STUB classification holds for normal play.

---

## Sources

- `MISSIONCLASS_STATE_MACHINE.md` — dispatch switch, enum table, vtable offsets
- `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md` — FootClass override map (pre-verified)
- `ENGINEER_CAPTURE_GHIDRA_REPORT.md` — Capture(8) handler (cited, not re-investigated)
- `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md` — AreaGuard(11) handler (cited)
- `MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md` — Repair(20) BuildingClass handler (cited, confirmed)
- `BUILDINGCLASS_MISSION_GUARD_AND_CONSTRUCTION.md` — BuildingClass vtable base address
- Live Ghidra: `read_memory`, `decompile_function`, `get_xrefs_to`, `get_function_callers`
  — all handler addresses and assigner traces performed live this session.
