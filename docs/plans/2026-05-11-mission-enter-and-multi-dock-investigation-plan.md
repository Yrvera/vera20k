# Mission_Enter + Multi-Pad Docking — Investigation Plan

> **For Claude:** This plan scopes a `/verify-doc` pass on existing load-bearing
> docs PLUS a narrow `/re-investigate` pass on the genuine remaining gaps.
> Execute Phase 0 (Agent D pre-flight) ONLY after `gamemd.exe` is loaded in
> Ghidra MCP. Then run the two-stage execution per Section 10.

**Topic:** (A) Mission_Enter (mission 7) as a first-class mission across building consumers, and (B) the multi-pad `DockingOffset`/`NumberOfDocks` system — pad-index assignment, per-pad occupancy, and how each consumer (refinery, helipad, service depot, garrison, engineer capture, spy infiltration, C4) hooks into them.

**Scope Size:** Small/Medium — most binary research is already documented; this pass is mostly verification + 4 targeted gap fills. Function inventory below: 12 functions in scope (3 verify-only, 5 gap-targeted, 4 contextual). INI keys: ~25 already mapped by Agent B.

**Est. Effort:** ~3–5 hours of mixed `/verify-doc` + narrow `/re-investigate` work (10–20 min per FULL function, 5 min per LIGHT). Most of the time is /verify-doc on three multi-page docs.

**Prior Research:** 15+ relevant reports in `ra2-rust-game-docs/`. See Section 2 for the inventory.

**Expected Output:**
1. Three /verify-doc audit notes (one per load-bearing doc) appended to the existing reports.
2. One new research doc at `docs/research/MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md` covering only the gaps.

**Next Pipeline Step:** `/brainstorm Mission_Enter + multi-dock pad system in Rust` — design the unified entry-contract abstraction in `sim/docking/` and the multi-pad `pads: Vec<DockPad>` field on `ObjectType`. Implementation is a follow-up session.

---

## 1. Goal

When this investigation finishes, we must be able to answer:

1. **For each consumer that calls Mission_Enter (mission 7),** which class override executes (`FootClass` base, `UnitClass`, `InfantryClass`, `AircraftClass`), what the early-exit conditions are, and what handler runs on arrival at the building? Single side-by-side table across all four classes.
2. **For multi-pad buildings (GAAIRC = 4 pads, refinery = 1, others?),** is the pad-index assignment definitively first-empty-slot via `RadioClass::FindDockSlot @ 0x65AD90`, or are there caller-side overrides (Find_Docking_Bay picks a preferred index)? Confirm by tracing one full handshake for an aircraft entering GAAIRC.
3. **For Spy infiltration**, what is the exact trigger site inside `InfantryClass::Mission_Enter @ 0x5196A0` that dispatches to `BuildingClass::OnSpyInfiltrate @ 0x4571E0`? Is `Agent=yes` checked unit-side or building-side?
4. **For FreeUnit-at-construction** (refinery spawns a free harvester), does the spawn ever invoke Mission_Enter on the newly spawned unit (e.g., to make it auto-enter the parent refinery), or is it a clean spawn with `facing=0xC0` only?
5. **Did any pre-2026-04-06 doc still carry the old `0x4D9290 = Mission_Harvest` mislabel** that needs correcting in place?

The answers feed directly into the /brainstorm design — specifically the choice between (a) a single `Mission` enum on `GameEntity` that mirrors gamemd's vtable dispatch, or (b) a trait-driven "entry contract" per building type that owns its own state.

---

## 2. Prior Research Inventory

### HIGH-confidence docs that COVER the topic (candidates for /verify-doc)

| Report | Scope | Confidence | Status |
|--------|-------|------------|--------|
| `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` | Master dock report: `NumberOfDocks`/`DockingOffset`, `BuildingTypeClass+0x1780/+0x1788`, `RadioClass::FindDockSlot @ 0x65AD90`, dock array `+0xE4/+0xE8`, GAAIRC 4 pads, **answers pad-index question** | HIGH | **PRIMARY /verify-doc target** |
| `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` | UnitClass::Mission_Enter @ 0x739EC0 full 534-line decompile, refinery path, radio cmd 8/0xE, states 0/2 | HIGH (with §5.3 correction) | **PRIMARY /verify-doc target** |
| `MISSION_ENTER_REFINERY_DOCK_VERIFICATION_NOTES.md` | §5.3 branch-swap correction + FUN_00500200 AI wander | HIGH | Already a verification doc — read and absorb |
| `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md` | Vtable dispatch map, locks in `0x4D9290 = FootClass::Mission_Enter` (mission 7, vtable+0x240). Resolves 8 prior mislabels | HIGH | **PRIMARY /verify-doc target** |
| `MISSIONCLASS_STATE_MACHINE.md` | Mission_Dispatch switch (0x5B3060), 32 mission enums, table at 0x816CAC | HIGH | Read-only reference, no verify needed |
| `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` | Full RadioClass protocol (cmds 2/3/0xE/0x22/0x23) | HIGH | Read-only reference |
| `FOOTCLASS_ENTER_QUEUE_AND_NAVCOM_SYSTEM.md` | Enter queue DVec +0x5AC, NavCom +0x588, TarCom +0x5C4 | HIGH | Read-only reference |
| `HARVESTER_DOCK_UNLOAD.md` | 8-section overview: CanDock, dock pad coords, radio protocol | 95% header | Already-named anchors; light cross-check |
| `HARVESTER_DOCK_UNLOAD_SEQUENCE.md` | End-to-end harvester lifecycle | HIGH | Read-only reference |
| `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md` | Mission_Harvest @ 0x73E5E0 state machine, links into Mission_Enter at state 3 | HIGH (99%) | Read-only reference |
| `ENGINEER_CAPTURE_GHIDRA_REPORT.md` | `InfantryClass::Mission_Capture @ 0x5202F0` — engineer uses **Mission 8**, not 7 | HIGH | Confirms engineer is OUT of Mission_Enter scope |
| `NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` | `InfantryClass::Mission_Enter @ 0x5196A0` for C4 plant, CanC4 (+0x1577), C4Warhead, C4Delay | HIGH | Read-only reference; covers C4 branch of InfantryClass override |
| `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md` | `BuildingClass::OnSpyInfiltrate @ 0x4571E0` — effects only, NOT the dispatch site | HIGH for effects | **Gap: dispatch site not traced** |
| `GARRISON_SYSTEM_GHIDRA_REPORT.md`, `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md` | Occupier→CanBeOccupied DVec +0x684. Single-slot interior, no DockingOffset | HIGH | Confirms garrison is OUT of multi-pad scope |
| `BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md`, `MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md` | Hospital/Armory/UnitRepair/Bunker/UnitReload timer machines at 0x44B780 | HIGH | Service depot + helipad reload covered |
| `SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md`, `SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md`, `SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md` | Slave Miner deploy goes through `Mission_Deploy_Building`, NOT Mission_Enter | HIGH | Confirms Slave Miner is OUT of Mission_Enter scope |
| `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` | `UnitClass::Mission_Deploy_Building @ 0x73D630` (MCV deploy + refinery dump) | HIGH | Read-only reference — runs *after* Mission_Enter links the dock |
| `TECH_BUILDINGS_GHIDRA_REPORT.md` | Tech captures; flags Hospital/Armory as DEAD in YR (TS legacy) | HIGH | TS-legacy filter reference |

### Conflicts and corrections already resolved (do NOT re-litigate)

- **`0x4D9290`** = `FootClass::Mission_Enter` (vtable+0x240 base). Pre-2026-04-06 docs may still call it `Mission_Harvest`. Light-check during /verify-doc.
- **`MISSION_ENTER_REFINERY_DOCK §5.3`** had inverted harvester/non-harvester branches. Fixed in verification notes. No re-litigation needed.
- **Hospital flag offset `+0x16C1`** was once confused with Refinery `+0x16BB`. Fixed in `BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md`.
- **C4 offsets** triple-conflict resolved by `NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` §0.

### Genuine GAPS (no doc covers these — these become /re-investigate scope)

1. **`AircraftClass::Mission_Enter` decompile** — the canonical 4-pad GAAIRC consumer. We know `BuildingClass::Receive_Radio` handles command 0xE (CAN_DOCK?) and the building side returns a pad cell, but the aircraft-side handler that calls `FindDockSlot` and parks on the assigned pad is NOT in any doc. Address TBD — discover in Phase 0.
2. **Side-by-side Mission_Enter cross-walk** — no single table compares FootClass (0x4D9290) vs UnitClass (0x739EC0) vs InfantryClass (0x5196A0) vs AircraftClass (TBD). Useful for picking the right Rust abstraction.
3. **Spy infiltration trigger site inside `InfantryClass::Mission_Enter @ 0x5196A0`** — what conditional dispatches to `BuildingClass::OnSpyInfiltrate @ 0x4571E0`? Is the `Agent=yes` flag checked on entry, or via `Spyable=yes` on the building?
4. **FreeUnit-at-construction** — `BuildingClass::OnConstructionComplete @ 0x00445F80` per HARVESTER_DOCK_UNLOAD.md spawns the free harvester with facing 0xC0 and no Mission_Enter queued. Re-verify in Phase 1 with one targeted decompile — confirm no Mission_Enter side-call.
5. **C4 scatter formula re-derive** — our `world_orders.rs` comment claims `(tick >> 12 + 1) >> 1 & 7`. Re-derive from binary at the Mission_Enter post-detonation block to confirm shift order isn't `(tick >> (12+1)) >> 1 & 7` vs `((tick >> 12) + 1) >> 1 & 7`.

---

## 3. Function Inventory

Execute in three phases. After Phase 1 checkpoint, summarize findings; revise the plan if early findings invalidate Phase 2/3 assumptions.

| # | Phase | Address | Current Name | Scope Reason | Depth Target | TS-Legacy Risk |
|---|-------|---------|--------------|--------------|--------------|----------------|
| 1 | 0 | `0x00457CE0` | `BuildingClass__CanDock` | Anchor verification + read first ~20 lines to re-confirm flag matrix | LIGHT | LOW — verified live in retail |
| 2 | 0 | `0x004DFCB0` | `FootClass__Find_Nearest_Dock` | Anchor verification + caller side pad-selection check | LIGHT | LOW |
| 3 | 0 | `0x004DF040` | `FootClass__Find_Docking_Bay` | Anchor verification — does this pre-select a pad index, or always return building only? | LIGHT | LOW |
| 4 | 1 | `0x0065AD90` | `RadioClass__FindDockSlot` | Spec-anchor: confirm scan loop on `RadioClass+0xE4` array, returns 0-based index | FULL | LOW |
| 5 | 1 | `BuildingClass::Receive_Radio` cmd 0x0E branch @ `0x0043C2D0` | (within Receive_Radio) | Confirm CAN_DOCK? returns dock cell + the chosen pad index (or just cell). Trace return value flow | FULL | LOW — live in YR |
| 6 | 1 | `0x00419C80` (Ghidra-labeled `Mission_Sticky` — mislabel; rename to `AircraftClass__Mission_Enter` during /re-investigate) | `AircraftClass__Mission_Enter` | **GAP #1 — address discovered.** AircraftClass vtable `0x007E22A4`, slot +0x240 = `0x007E24E4` → `0x00419C80`. Canonical multi-pad consumer | FULL | MEDIUM — verify pad-index logic isn't TS-only (Helipad= flag is YR-live) |
| 7 | 1 | `0x005196A0` | `InfantryClass__Mission_Enter` | Spy dispatch site (gap #3) + C4 dispatch site cross-check. We already have C4 side from NAVY_SEAL doc | FULL | LOW |
| 8 | 2 | `0x004571E0` | `BuildingClass__OnSpyInfiltrate` | Read entry conditions — what does Mission_Enter pass through? | MEDIUM | LOW |
| 9 | 2 | `0x00739EC0` | `UnitClass__Mission_Enter` | /verify-doc cross-check: re-confirm states 0/2, branch matrix; spot stale `0x4D9290` references | MEDIUM | LOW |
| 10 | 2 | `0x00445F80` | `BuildingClass__OnConstructionComplete` | Gap #4: confirm FreeUnit spawn does NOT queue Mission_Enter on the spawned unit | LIGHT | LOW |
| 11 | 2 | Mission_Enter post-detonation block (TBD address within `0x004D9290` or related) | C4 scatter formula | Gap #5: re-derive `(tick >> 12 + 1) >> 1 & 7` shift order from binary | LIGHT | LOW |
| 12 | 3 | `0x006AF6C0` (**Agent D verification: this is `SlaveManagerClass__AI_Update`, NOT a refinery dock-queue processor — see Section 9 open question #10**) | (re-source) | Re-find the correct refinery dock-queue processor address during /verify-doc on BUILDING_DOCKING_SYSTEM; the existing doc's claim is suspect | LIGHT | LOW |
| 13 | 3 | `0x005B2F00` (no defined function — likely `Mission_Sleep` no-op thunk) | BuildingClass vtable +0x240 target | Cross-walk completeness: confirms BuildingClass does NOT meaningfully override Mission_Enter | LIGHT | LOW |

**Phase 1 checkpoint rule:** After functions #4–#7 are decompiled, write a one-paragraph status note ("AircraftClass::Mission_Enter is at 0xXXXXX, dispatches to <radio cmd chain>; spy dispatch happens via <condition>") and STOP. Confirm the gap-fill is producing what we need before continuing to Phase 2.

**Vtable +0x240 cross-walk targets:** When function #6 is found, also resolve vtable +0x240 for `BuildingClass` (if present — likely no-op or default-FootClass). Final cross-walk table goes in the new MISSION_ENTER_CROSSWALK doc.

---

## 4. Detail Checklist

### Magic numbers to extract from #6 (AircraftClass::Mission_Enter)

- Approach distance threshold (in leptons / cells) — equivalent of `HarvesterTooFarDistance`?
- Descent altitude / landing trigger
- Radio command IDs sent on arrival (likely 2 DOCK_LINK, 0xE CAN_DOCK?, 0x15 PREPARE)
- Reload-trigger condition (calls `MissionRepairAndProduce` UnitReload branch?)

### Bit flags / state encoding

- `MissionSubState` byte at instance +0xBC: confirm aircraft uses same state-byte slot as units
- Per-pad occupancy: is there a per-pad bool on BuildingClass beyond the dock array, or is array-slot-empty the only signal?

### State machines

- Aircraft Mission_Enter sub-states (confirm count — likely 0/1/2/3 mapping to ApproachAirfield / WaitForPad / Descend / Reload)
- Spy Mission_Enter sub-states (probably reuses 0/1/2 with infiltrate trigger at sub-state 2)

### INI keys to verify (from Agent B's inventory)

All already mapped — see Agent B's table. Specifically re-confirm in /verify-doc:
- `NumberOfDocks` defaults: 1 for refineries/depots, 4 for `GAAIRC`/`AMRADR`
- `DockingOffset0..3` parsed for max N=3 in retail
- `QueueingCell` is art.ini-only
- `Helipad=yes` is paired with `UnitReload=yes`
- `Capturable` defaults `true`
- `Spyable` defaults `false` per-building
- `CanBeOccupied` only on garrison-able structures (e.g., NABRCK)
- `MaxNumberOccupants` defaults 10

### Struct offsets to extract (from Phase 1)

- `RadioClass` (`AbstractClass` base): `+0xE4` dock array, `+0xE8` count, stride per slot
- `BuildingClass`: `+0xE4` (dock array — same as RadioClass base?), `+0x2E4` (docked unit pointer per HARVESTER_DOCK_UNLOAD), `+0x620` accumulator (per BUILDING_DOCK_AND_HEAL doc)
- `BuildingTypeClass`: `+0x1780` NumberOfDocks (int), `+0x1788` DockingOffset array (3*int per pad, stride 12)
- `AircraftClass` vtable: address TBD; `+0x240` is the Mission_Enter slot
- All `param_1` types: confirm direct byte offset vs *4 int-array indexing

### Clamps / rounding / off-by-ones to look for in #6

- Approach-cell rounding from lepton → cell space (256 leptons per cell, watch for +128 half-cell offset bug)
- Pad index 0 vs 1-based — DockingOffset is 0-indexed in INI ("DockingOffset0")

### Edge cases to test (input → expected output)

- 2 aircraft simultaneously trigger ammo=0 with one free pad → first-empty-slot tiebreak (per `FindDockSlot`)
- Helipad destroyed mid-descent: aircraft Mission_Enter should detect via dock-link CLEAR_LINK (radio cmd 3) and exit
- Spy enters allied building: should be no-op (alliance check before infiltrate fires)
- Engineer enters Captured-flag=false building: should be rejected at `Receive_Radio` cmd 0xF
- 4-pad GAAIRC with all 4 occupied + 5th aircraft arrives: aircraft hovers (waitfordock equivalent)
- DockingOffset0 specified but NumberOfDocks=2: behavior when index 1 is read from array — is index 1's bytes zero-initialized or garbage?

### Timing / ordering

- Where does Mission_Enter run relative to other missions in `Mission_Dispatch @ 0x005B3060`? Already known: vtable dispatch, current mission read from instance +0xAC.
- Per-tick: does Mission_Enter return a frame delay (1-3 jitter per HARVESTER_MISSION_HARVEST §3), or 0?

### TS-legacy filter

- **Bunker=yes** — only on YABRKS (YR-only). Live in YR.
- **Hospital=yes / Armory=yes** — commented out in `rulesmd.ini` per Agent B; TS-legacy, do not implement in Mission_Enter consumer set.
- **Cloning=yes** — live in YR (NACLONE/GACLONE/YACLONE); but cloning vat receives unit-cargo from production, no Mission_Enter dispatch needed.
- **Grinding=yes** — live in YR (YAGRNR). May use Mission_Enter — flag as low-priority follow-up if discovered during #6.
- **SecretLab capture** — `+0x16B0` flag. Capture path is Mission 8 (engineer), not Mission_Enter.

### Vtable dispatches to resolve

- AircraftClass vtable +0x240 (function #6)
- BuildingClass vtable +0x240 (sanity — buildings shouldn't enter anything; expect default-FootClass or no-op)
- Cross-walk all 4 in the new doc.

---

## 5. INI Keys in Scope

See Agent B's full table (above this plan in conversation). The investigation must re-verify the following set in /verify-doc:

| Key | Section default | Currently parsed in Rust? | Notes for /re-investigate |
|-----|-----------------|---------------------------|---------------------------|
| `NumberOfDocks` | 1 / 4 | YES — `ObjectType.number_of_docks` ([object_type.rs:576](src/rules/object_type.rs#L576)) | Used by airfield path only; not multi-pad-aware |
| `DockingOffset0..3` | varies, lepton offset | **PARTIAL** — only `DockingOffset0` ([art_data.rs:272-279](src/rules/art_data.rs#L272-L279)) | **GAP for implementation**, not for research |
| `QueueingCell` | art.ini | YES ([art_data.rs:265-271](src/rules/art_data.rs)) | Art-only, no rules side |
| `Helipad` | yes (NAHPAD/GAHPAD/AMRADR) | YES ([object_type.rs:949](src/rules/object_type.rs#L949)) | Always paired with UnitReload=yes |
| `UnitReload` | yes (helipads + airfields) | YES ([object_type.rs:948](src/rules/object_type.rs#L948)) | |
| `UnitRepair` | yes (NAREPAIR/GAREPAIR/YAREPAIR/NATECH) | YES ([object_type.rs:947](src/rules/object_type.rs#L947)) | Service depot |
| `Refinery` | yes (NAREFN/GAREFN/YAREFN) | YES | |
| `DockUnload` | yes (refineries) | Verify | Always paired with Refinery=yes |
| `Capturable` | true (most) | YES via engineer path | Engineer path uses Mission_Capture (8), not Mission_Enter |
| `Spyable` | varies | Verify | Drives spy infiltration; check in #7/#8 trace |
| `CanBeOccupied` | yes (NABRCK) | YES (garrison path) | Garrison out of Mission_Enter scope per Agent A |
| `Bunker` | yes (YABRKS) | Verify | YR-only |
| `Grinding` | yes (YAGRNR) | Verify | Possible Mission_Enter consumer — low priority |
| `Cloning` | yes (NACLONE/GACLONE/YACLONE) | Verify | NOT Mission_Enter — cargo from production |

---

## 6. Caller & Integration Map

### Mission-7 queue sites (verified by Agent D pre-flight)

`FootClass::Mission_Enter @ 0x004D9290` has **0 direct callers** — it's exclusively dispatched via vtable from `Mission_Dispatch @ 0x005B3060` case 7. Dispatch chain: subclass vtable +0x240 → falls back to FootClass if not overridden. Inheriting vtables found at `0x007E8ED4` (FootClass self), `0x007EB298` (intermediate, likely VehicleClass), `0x007F5EB0` (another inheritor).

The interesting question is **who writes mission code 7 into instance+0xAC.** Agent D's `byte_pattern 6A 07 8B` scan (PUSH 7; MOV) returned 38 hits. Top 12 by relevance:

| Address | Containing function | Significance |
|---------|---------------------|--------------|
| `0x00416EBF` | `AircraftClass::Mission_Move_Carryall` | Carryall paratrooper/cargo pickup handoff to Mission_Enter |
| `0x00417C21` | (inside aircraft mission region, no function defined) | Adjacent to AircraftClass mission code; investigate during /re-investigate |
| `0x0041A7F4` | `AircraftClass::Mission_Guard` | Idle aircraft queues Mission_Enter when ammo depleted — the rearm trigger |
| `0x0043CC7D` | `BuildingClass::Receive_Radio` | **Building → unit handshake** that puts a passenger/dock-target into mission 7. This is the canonical "enter me" radio command path |
| `0x004DFC87` | `FUN_004dfb70` | In the Find_Nearest_Dock neighborhood — dock-finding helper that queues Mission_Enter on caller |
| `0x004E0063` | `FUN_004dff40` | Same dock-finding region |
| `0x00510F2C` | `FUN_00510ed0` | Infantry helper — possibly the spy/engineer/C4 selector or `Find_Nearest_Repair_Bay` analog |
| `0x0051AC1B` | `FUN_0051aa40` | Immediately after `InfantryClass::Mission_Enter` ends — likely split/inlined infantry helper that re-queues mission 7 |
| `0x00520154` | (near `InfantryClass::Mission_Capture` or `Fear_Decay_Handler`) | Infantry mission-7 queue site; investigate origin |
| `0x0073E08A` | `UnitClass::Mission_Deploy_Building` | MCV/refinery-dump path re-queues Mission_Enter at end of dump |
| `0x0073EE8F` | `UnitClass::Mission_Harvest` | **State 3 → Mission_Enter handoff** (already documented in HARVESTER_MISSION_HARVEST §2 state 3) |
| `0x00738664` | `UnitClass::ReceiveDamage` | Unit-takes-damage handler queues Mission_Enter (possibly retreat-to-repair logic) |

### Direct callers of `MissionClass::Queue_Mission @ 0x005B35E0`

Only **1 direct caller**: `AircraftClass::Queue_Mission_Override @ 0x0041BA90`. All other mission-queueing happens via direct instance write (the `byte_pattern 6A 07 8B` hits above) rather than the Queue_Mission helper.

### Rust integration points

- **`sim/docking/`** is the natural home for unified Mission_Enter. Currently has `aircraft_dock.rs` (pad-anonymous multi-slot) and `building_dock.rs` (single-cell service depot).
- **`sim/miner/miner_dock.rs`** has single-slot `DockReservations` (used by refinery). Candidate to merge into a multi-pad slot manager in `sim/docking/`.
- **`sim/miner/miner_dock_sequence.rs`** is the conflated approach+dock FSM. Future refactor: harvester becomes a consumer of generic Mission_Enter instead of owning its FSM.
- **`sim/aircraft/mod.rs`** has `AircraftMission::Docking { sub_state }` — already mission-aware. This is the closest existing analogue to a unified Mission enum.
- **`sim/world/world_orders.rs`** has `tick_capture_orders`, `tick_c4_plants` — ad-hoc Mission_Enter analogues. Capture stays separate (Mission 8); C4 plant may move into a unified Mission_Enter consumer.
- **`sim/command.rs`** has `EnterTransport`, `RepairAtDepot`, `CaptureBuilding`, `PlantC4`, `MinerReturn` as separate variants. Brainstorm should propose whether to unify under a single `EnterBuilding { target_id, intent: EnterIntent }`.
- **`sim/passenger.rs`** has `PassengerRole { None, Boarding, Inside, Transport }`. Orthogonal to Mission_Enter per Agent C — passengers are inside transports, dockers stay on the map.

---

## 7. TS-Legacy Risk Register

Consolidated from Agent B and prior docs. Verify each during /verify-doc + /re-investigate:

- **`Hospital=yes`** — commented out in YR `rulesmd.ini`. TS legacy. Mission_Enter path inside `MissionRepairAndProduce` may still reference it; **do not implement** even if found.
- **`Armory=yes`** — same as Hospital. TS legacy.
- **`Bunker=yes`** — YR-live (YABRKS). Implement.
- **`Grinding=yes`** — YR-live (YAGRNR). May use Mission_Enter — flag in #6/#7 if discovered.
- **`Cloning=yes`** — YR-live but does NOT use Mission_Enter — verify path is via production.
- **`SpecialFlags`** — Agent B confirms `[SpecialFlags]` section is NOT in retail INI. Any function gated behind `SpecialFlags & 0x1000` is TS-only and dormant.
- **`FogOfWar`** — confirmed default-false in YR (memory). Any Mission_Enter logic gated behind it is dormant.
- **TS-era mission slots** — Mission table at 0x816CAC includes slots like Ambush, Patrol, Construction that may be unused in YR maps. Per MISSIONCLASS_STATE_MACHINE doc.

---

## 8. Current Rust Implementation Surface

Per Agent C, six entry-into-building FSMs exist today, each with its own state field on `GameEntity`:

1. **`miner: Option<Miner>`** ([src/sim/miner/mod.rs:213](src/sim/miner/mod.rs#L213)) — `MinerState::Dock` + `RefineryDockPhase` (Approach/Linked/Unloading/Departing). Single-slot reservation via [`DockReservations`](src/sim/miner/miner_dock.rs).
2. **`dock_state: Option<DockState>`** ([src/sim/docking/building_dock.rs:41](src/sim/docking/building_dock.rs#L41)) — `DockPhase` (Approach/WaitForDock/EnterDock/Servicing/ExitDock). Service depot only. Single-slot via `depot_dock_reservations`.
3. **`aircraft_ammo.dock_phase: Option<AircraftDockPhase>`** ([src/sim/docking/aircraft_dock.rs:62](src/sim/docking/aircraft_dock.rs#L62)) — ReturnToBase/WaitForDock/Descending/Reloading/Launching. Multi-slot **but pad-anonymous** via [`AirfieldDocks`](src/sim/docking/aircraft_dock.rs#L85).
4. **`aircraft_mission: Option<AircraftMission>`** ([src/sim/aircraft/mod.rs](src/sim/aircraft/mod.rs)) — high-level: Idle/Move/Attack/Guard/ReturnToBase/Docking{sub_state}/DockedIdle/ParaDropApproach/ParaDropOverfly. Coexists with #3.
5. **`capture_target: Option<u64>`** + `tick_capture_orders` ([src/sim/world/world_orders.rs:147](src/sim/world/world_orders.rs#L147)) — engineer capture, Mission 8 path, not Mission_Enter.
6. **`c4_plant: Option<C4PlantState>`** + `tick_c4_plants` ([src/sim/world/world_orders.rs:228](src/sim/world/world_orders.rs#L228)) — Tanya/SEAL plant intent.

Plus passenger orthogonal: **`passenger_role: PassengerRole`** ([src/sim/game_entity.rs:39](src/sim/game_entity.rs#L39)).

**Parser side** ([src/rules/art_data.rs:272-279](src/rules/art_data.rs#L272-L279)) reads ONLY `DockingOffset0`. `number_of_docks` is parsed but unused for multi-pad addressing. [Ruleset art→rules merge at ruleset.rs:1630-1640](src/rules/ruleset.rs#L1630-L1640).

**Test coverage that will constrain the refactor:**
- `src/sim/miner/miner_tests.rs` — full harvester FSM (~400 lines)
- `src/sim/docking/aircraft_dock.rs` tests — multi-slot airfield dock reservations
- `src/sim/world/world_orders_c4_tests.rs` — C4 plant integration
- No end-to-end test for full harvest-capture-dock-repair-garrison cycle.

---

## 9. Deferred Open Questions

Resolve during /re-investigate. If unresolvable from binary, document explicitly as unresolved in the new doc.

1. **AircraftClass vtable address** — TBD via Ghidra symbol search.
2. **AircraftClass::Mission_Enter address** — TBD at the +0x240 slot.
3. **Pad-selection policy when caller knows preferred index** — does Find_Docking_Bay ever return a pad index (not just building), or is index always picked by FindDockSlot at link time?
4. **Per-pad approach-cell occupancy** — does gamemd reserve approach cells per pad, or just the pad cell itself? Probable answer: only the pad cell, but verify.
5. **Helipad dock-link timeout / queue behavior** — does an aircraft hover indefinitely if all pads occupied, or does it eventually return to base / re-queue with a cooldown?
6. **DockingOffset stride for an N-pad building where art.ini only specifies <N offsets** — is the unspecified pad's offset zero-initialized (visible center-overlap bug) or does the parser bail at first missing index?
7. **Save-game serialization of pad-index** — confirm pad index is preserved (likely via the dock array slot, not a per-unit field).
8. **Cross-pad facing during dock** — gamemd's harvester faces 0x4000 before docking (radio cmd 0x16 FACE_DOCK). Does aircraft Mission_Enter set a per-pad facing, or fixed?
9. **Mission_Enter completion return** — does Mission_Enter clear itself to Guard/Sleep on success, or does the building broadcast something? Verify in #6.
10. **The `0x006AF6C0` mystery** — `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` cites this as a refinery dock-queue processor (slot array at `dock_manager+0x3C`, count `+0x48`). Agent D's pre-flight check found `0x006AF6C0` is actually `SlaveManagerClass__AI_Update`. Either the prior doc is wrong, the address moved, or the dock_manager and slave_manager share the +0x3C/+0x48 layout coincidentally. **Re-source the correct refinery dock-queue processor address during Stage 1 /verify-doc on BUILDING_DOCKING_SYSTEM.** This is the most load-bearing surprise from the pre-flight.
11. **Ghidra mislabel: `0x00419C80`** is currently labeled `AircraftClass__Mission_Sticky` (mission code 0x11 ≠ 7). It sits in vtable slot +0x240 (Mission_Enter), so the label is wrong. The /re-investigate executor may rename it to `AircraftClass__Mission_Enter` after decompiling to confirm — but only after explicit user approval per the "label what you understand with ~90% confidence" rule in CLAUDE.md.
12. **The `byte_pattern 6A 07 8B` hits at `0x00417C21` and `0x00510F2C`** sit inside undefined-function regions (`FUN_*` placeholders). Phase 1 should `create_function` boundaries here before decompiling — these may be the spy/repair-bay or carryall helpers we need to fully trace.
13. **`UnitClass::ReceiveDamage @ 0x00738664` queues mission 7** on damage — is this a "retreat to repair" trigger (driver flees to depot when HP drops)? Or unrelated? Worth investigating during Stage 2 since it surfaces a Mission_Enter consumer we hadn't anticipated.

---

## 10. Execution Strategy

Two-stage execution:

### Stage 0 (Pre-flight) — **COMPLETED 2026-05-11**

Agent D pre-flight ran against the live binary. Results:

- All 12 of 13 known anchor addresses verified ✓ (function #12 `0x006AF6C0` flagged as wrong — see Section 9 open question #10).
- **AircraftClass::Mission_Enter discovered at `0x00419C80`** (vtable `0x007E22A4`, slot +0x240 → `0x007E24E4` → `0x00419C80`). Currently Ghidra-mislabeled `Mission_Sticky`. Function #6 in Section 3 updated.
- **BuildingClass vtable +0x240** points at `0x005B2F00` (no defined function, likely Mission_Sleep no-op). Buildings effectively don't override Mission_Enter. Function #13 in Section 3 records this.
- **12 mission-7 queue sites** discovered across AircraftClass / BuildingClass::Receive_Radio / dock-finding helpers / Infantry helpers / UnitClass::Mission_Harvest / Mission_Deploy_Building / ReceiveDamage. Section 6 caller map fleshed out.
- **`RadioClass::Transmit_Radio_Impl @ 0x0065A970` cmd 2 (HELLO/DOCK_LINK)** confirmed: scans `Contacts[] @ this+0xE4` for first empty slot, writes target only if target's `Receive_Radio(HELLO)` returns ROGER (1). If full, evicts slot 0 via recursive `Transmit_Radio(BREAK=3, Contacts[0])`. Allocation is internal to RadioClass — caller does not pre-compute index. Cross-confirms BUILDING_DOCKING_SYSTEM §2 + §7.

### Stage 1 — /verify-doc pass (parallel, ~1.5 hours)

Three /verify-doc invocations against the load-bearing docs (Section 2 PRIMARY targets):
- `/verify-doc BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md`
- `/verify-doc MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`
- `/verify-doc FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md`

Goal: re-anchor every claim in these docs against the live binary; flag any stale `0x4D9290 = Mission_Harvest` labels left over.

### Stage 2 — Narrow /re-investigate (single doc, ~2 hours)

`/re-investigate AircraftClass::Mission_Enter + Mission_Enter cross-walk + spy/C4/FreeUnit gap-fills`

Produces one new research doc at `ra2-rust-game-docs/MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md` covering:
- Function inventory rows #6–#11 (the gap fills)
- One side-by-side cross-walk table of FootClass/UnitClass/InfantryClass/AircraftClass Mission_Enter overrides
- Resolutions for the deferred open questions in Section 9

### Stage 3 — /brainstorm

After Stage 2 doc is written and approved, run `/brainstorm Mission_Enter + multi-dock pad system in Rust` against the combined verified picture. Produces the design spec for the actual refactor.

---

## 11. Success Criteria

The Stage 2 research doc must:

- Decompile `AircraftClass::Mission_Enter @ 0x00419C80` (currently Ghidra-mislabeled as `Mission_Sticky`) to FULL depth (every state, every branch, every magic number, every radio command it sends). After decompile, rename the function in Ghidra after user approval.
- Produce a 5-row vtable +0x240 cross-walk: FootClass (`0x004D9290`) / UnitClass (`0x00739EC0`) / InfantryClass (`0x005196A0`) / AircraftClass (`0x00419C80`) / BuildingClass (`0x005B2F00`, no-op). For each: address, primary states, key branch points, mission-7-queue-site callers, edge-exit conditions.
- Resolve open question #10: re-source the correct refinery dock-queue processor address (the BUILDING_DOCKING_SYSTEM doc cites `0x006AF6C0` which is actually `SlaveManagerClass__AI_Update`).
- Trace the spy infiltration dispatch: exact site in `InfantryClass::Mission_Enter @ 0x5196A0` that calls `BuildingClass::OnSpyInfiltrate @ 0x4571E0`. Confirm `Spyable=yes` check is building-side.
- Confirm or correct the C4 scatter formula `(tick >> 12 + 1) >> 1 & 7`.
- Verify FreeUnit spawn at refinery construction does NOT auto-queue Mission_Enter.
- For every claim, cite Ghidra address and quote ≤3 lines of decompilation.
- Mark each finding "Active in YR: Yes/No/Conditional".
- Answer Section 1's five goal questions explicitly.

The Stage 1 verification notes must:

- Re-verify every address-bearing claim in the three primary docs against the live binary.
- Flag any address that has been renamed since the doc was written.
- Flag any struct-offset claim that doesn't match the current Ghidra struct layout.
- Be appended (not destructively edit) to the source docs, dated.

---

## Sources

- Ghidra addresses sampled (pre-Phase 0, from prior docs; all ✓-verified by Agent D unless noted):
  - `0x004D9290` (FootClass::Mission_Enter), `0x00739EC0` (UnitClass::Mission_Enter), `0x005196A0` (InfantryClass::Mission_Enter)
  - `0x00457CE0` (BuildingClass::CanDock), `0x004DFCB0` (Find_Nearest_Dock), `0x004DF040` (Find_Docking_Bay)
  - `0x0065AD90` (RadioClass::FindDockSlot), `0x0043C2D0` (BuildingClass::Receive_Radio), `0x00737430` (UnitClass::Receive_Radio)
  - `0x005B3060` (Mission_Dispatch), `0x00816CAC` (mission name table)
  - `0x00445F80` (OnConstructionComplete)
  - `0x004571E0` (BuildingClass::OnSpyInfiltrate), `0x005202F0` (InfantryClass::Mission_Capture)
  - `0x006AF6C0` (claimed as refinery dock-queue processor by BUILDING_DOCKING_SYSTEM doc — Agent D found this is `SlaveManagerClass__AI_Update`; **re-source during Stage 1**)
- Ghidra addresses discovered by Agent D pre-flight (Stage 0):
  - `0x00419C80` (AircraftClass::Mission_Enter — currently mislabeled `Mission_Sticky`)
  - `0x007E22A4` (AircraftClass primary vtable), `0x007E24E4` (its +0x240 slot)
  - `0x007E3EBC` (BuildingClass primary vtable), `0x005B2F00` (its +0x240 target — no-op)
  - `0x007E8ED4` / `0x007EB298` / `0x007F5EB0` (vtable slots that inherit FootClass's Mission_Enter)
  - `0x0041BA90` (AircraftClass::Queue_Mission_Override — only direct caller of Queue_Mission@0x005B35E0)
  - Mission-7 queue sites: `0x00416EBF`, `0x00417C21`, `0x0041A7F4`, `0x0043CC7D`, `0x004DFC87`, `0x004E0063`, `0x00510F2C`, `0x0051AC1B`, `0x00520154`, `0x0073E08A`, `0x0073EE8F`, `0x00738664`
- Docs searched (Agent A inventory): 19 reports in `ra2-rust-game-docs/`
- INI files checked: `rules.ini`, `rulesmd.ini`, `art.ini`, `artmd.ini`
- Related plans: `2026-05-06-refinery-dock-gamemd-parity-{design,plan}.md`, `2026-05-10-navy-seal-c4-{design,plan,investigation-plan}.md`, `2026-04-27-refinery-undock-bypass-grid-{design,plan}.md`
