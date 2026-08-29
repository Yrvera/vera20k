# HouseClass AI Production Tail Formulas - Ghidra Report

> **2026-08-29 correction:** any defeat/`ScatterAllUnits` wording retained here
> for `0x004FC6D0` names the ordering slot only and must not be read as movement
> behavior. The callee is the shared destructive House Techno/C4 receiver sweep
> documented in
> `docs/gap-scans/2026-08-29-disparity-scan-action-119-house-destruction.md`.

**Date:** 2026-05-28  
**Target:** `HOUSECLASS_AI_PRODUCTION_TAIL_FORMULAS`  
**Investigation Mode:** coverage-map  
**Address(es):** `HouseClass::Update @ 0x004F8440`, `HouseClass::AI_Building_Strategy @ 0x004FD500`, `HouseClass::AI_Check_Build_Need @ 0x004FD9A0`, `HouseClass::AI_Manage_Build_Queue @ 0x004FDD10`, `HouseClass::AI_Choose_Building @ 0x004FE3E0`, `HouseClass::AI_Choose_Unit @ 0x004FEA60`, `HouseClass::AI_Choose_Infantry-like @ 0x004FEEE0`, `HouseClass::AI_Choose_Aircraft-like @ 0x004FF210`, `HouseClass::AI_ManageProduction @ 0x0050AF10`, `HouseClass::AI_ResumeProduction @ 0x0050B1D0`, `HouseClass::AI_DispatchProduction @ 0x005098F0`  
**Confidence:** High for call order, gate predicates, and factory-tail interactions listed as verified; Medium/Partial for full building placement formulas because `AI_ChooseNextProduction @ 0x00506EF0` and base-plan helpers were touched but not exhausted.  
**Active in YR:** Yes. These functions are reached from active `HouseClass::Update` in standard YR skirmish/multiplayer AI houses; branches labeled conditional below require AI/non-passive houses, frame/timer gates, or dirty production state.

## Working Notes

Target question: Verify HouseClass AI production tail order and formula/gate details relevant to Factory/House tail order contracts, including `AI_ManageProduction`, `AI_ResumeProduction`, AI chooser gates, factory queue interactions, and exact call order relative to `HouseClass::Update` tail.
Non-goals: Do not redo FactoryClass OLE save/load order, general production UI behavior, full sidebar pixel rendering, or the complete `AI_ChooseNextProduction` base-placement search.
Evidence needed to mark COMPLETE: Decompile plus assembly context for `HouseClass::Update` AI/tail calls, decompile plus assembly context for build-strategy/build-queue/factory interactions, decompile of manage/resume gates, and current Rust surface scan with acceptance tests.
Stop conditions: Stop when the requested tail gates and factory interactions are verified or when remaining formula depth clearly requires a separate narrow investigation; write only this report plus the shared claims file.

## 1. Overview

The active YR house tail separates three concepts that current docs and Rust can easily blur: periodic AI build choice, dirty-gated superweapon manage/resume, and global FactoryClass progress. Global factories have already ticked before `HouseClass::Update`; then each AI house may run building strategy on its own timer, may run category choosers on an 8-frame cadence, and finally may consume `House+0x1FC` for `AI_ManageProduction`/`AI_ResumeProduction`.

The most important correction is naming: `AI_ManageProduction @ 0x0050AF10` and `AI_ResumeProduction @ 0x0050B1D0` are not the same thing as the build-queue chooser formulas. They are dirty-gated superweapon grant/suspend/deactivate/resume/cameo functions. The build-choice formulas live mainly in `AI_Building_Strategy`, `AI_Check_Build_Need`, `AI_Manage_Build_Queue`, `AI_Choose_Building`, and the unit/infantry/aircraft choosers.

## 2. Key Offsets / State

| Offset / global | Verified role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `House+0x1E4` | AI chooser mode/state: `0`, `1`, `2` choose different order/recheck paths. | `0x004F9087..0x004F9265`, `0x004FE109`, `0x004FE3AB` | Yes, AI/non-passive only |
| `House+0x1EC/+0x1ED` | Current/player-control gate used by AI checks. | `0x004F8FE1`, `0x004FD9A0`, `0x005098F0` | Yes |
| `House+0x1F5` | Defeated byte; gates `AI_ResumeProduction`. | `0x0050B1D0` | Yes |
| `House+0x1FC` | Dirty manage/resume gate consumed at the end of `HouseClass::Update`. | `0x004F92ED..0x004F92FD` | Yes |
| `House+0x24` | Wallet/storage-like object; vtable `+0x18` returns balance used for affordability gates. | `0x004FD500`, `0x004FD9A0`, `0x004FEA60` | Yes |
| `House+0x250` | AI urgency/money/under-attack state. | `0x004FD6F9..0x004FD848` | Yes, AI strategy |
| `House+0x564C` | Building type choice slot, `-1` means empty. | `0x004FE3E0`, `0x004F90B3` | Yes, AI/non-passive |
| `House+0x5650/+0x5654/+0x5658` | Unit/infantry/aircraft choice slots, `-1` means empty. | `0x004FEA70`, `0x004FEEED`, `0x004FF21D` | Yes, AI/non-passive |
| `House+0x5708/+0x5714` | AI base-plan/build queue, 16 bytes per entry, count at `+0x5714`. | `0x004FDD10`, `0x004FE3E0`, `0x00506EF0` | Yes, AI base building |
| `g_FactoryClass_Array/count` | Build-queue manager can suspend/abandon all owned factories when plan cost exceeds budget. | `0x004FE05B..0x004FE097` | Yes |
| `Rules+0x13F4[difficulty]` | Probability table used by unit/infantry/aircraft choosers for nearest/highest-priority choice versus random max-need candidate. | `0x004FEDF2`, `0x004FF190`, `0x004FF4C0` | Yes |

## 3. Core Logic And Ordering

### 3.1 `HouseClass::Update` chooser and tail order

After superweapon-ready and multiplayer defeat/scatter sections, `HouseClass::Update` runs AI logic only when the house is not current-player/player-controlled and its HouseType is not `MultiplayPassive` (`Type+0x1A6 == 0`). Active in YR: Yes; conditional on AI/non-passive house.

The ordering is:

1. A separate strategy timer at `House+0x5634/+0x563C` gates `AI_Building_Strategy @ 0x004FD500`; the function's return value becomes the next duration. Evidence: decompile `0x004F8FE1..0x004F9043`; assembly `0x004F8FE1`.
2. On `g_CurrentFrameCounter % 8 == 0` using the signed-mask pattern `g_CurrentFrameCounter & 0x80000007`, native runs chooser calls. Evidence: decompile `0x004F904F..0x004F9265`.
3. If `House+0x1E4` is `0` or single-player mode, call order is building, unit, aircraft, infantry. Evidence: `0x004F9092..0x004F90A1`.
4. If `House+0x1E4 == 1`, choose building first, then check `House+0x564C` via `BuildingType` vtable `+0x94` with `(1,1,1,this)`; if buildable check fails, continue to unit/aircraft/infantry. Evidence: assembly `0x004F90B3..0x004F90E2`.
5. If `House+0x1E4 == 2`, choose unit first, run a side/random rules scan, then choose aircraft and infantry if the current unit target does not match `House+0x5650`; after candidate validation, building can be retried if no category choice exists or any existing choice fails vtable `+0x94`. Evidence: decompile `0x004F90E7..0x004F9265`; assembly `0x004F9196`.
6. Only after those chooser gates does `House+0x1FC` tail run. The current-player branch clears `+0x1FC`, loops buildings to expel/flush stale orders, calls `FUN_006A7D20`, then calls `AI_ManageProduction` and `AI_ResumeProduction`; non-player houses clear `+0x1FC` and jump directly to the same two calls. Evidence: decompile `0x004F9265..0x004F92FD`; assembly `0x004F92F4`.

### 3.2 `AI_Building_Strategy @ 0x004FD500`

Active in YR: Yes for AI/non-passive houses when `HouseClass::Update` strategy timer expires.

Verified behavior:

- If `House+0x5600 == -1`, `g_GameMode != 0`, and the house type is not passive, it selects a nearest non-self, non-passive, non-defeated house by 3D distance and calls `HouseClass::Update_Threat_Score(1, target)`. Evidence: decompile `0x004FD538..0x004FD660`.
- If the current enemy index points at a defeated house, it removes that house's grudge score from the `House+0x5608` vector and resets `House+0x5600 = -1`. Evidence: decompile `0x004FD666..0x004FD6D6`.
- It calls `AI_DispatchProduction @ 0x005098F0` when `g_GameMode != 0` or `Rules+0x1438 <= House+0x24C`. Evidence: decompile `0x004FD6DE..0x004FD6EE`.
- It updates urgency state `House+0x250`: state `0` goes to `1` when wallet vtable `+0x18` returns `< 25`; state `1` returns to `0` when wallet is `> 24`; state `3` clears after `House+0x54D8 + 900 < g_CurrentFrameCounter`, otherwise current frame before that deadline forces state `3`. Evidence: decompile `0x004FD70A..0x004FD848`.
- Multiplayer-only priority array uses two slots: slot 0 contributes priority `4` only when urgency is not `3` and no owned operational building has type byte `+0xEB8 != 0`; slot 1 is `AI_Check_Build_Need`. It then loops priority values from `4` down to `1`, dispatching matching slot 0 wallet action or slot 1 `AI_Manage_Build_Queue`. Evidence: decompile `0x004FD848..0x004FD913`; assembly `0x004FD848`, `0x004FD922`.
- Return value is `Random::RandomRanged(1,7) + 0x69`, so the next strategy delay is 106..112 frames inclusive if native `RandomRanged` is inclusive as used elsewhere. Evidence: assembly `0x004FD91C..0x004FD928` pushes `1`, `7`, calls random, then adds `0x69`.

### 3.3 `AI_Check_Build_Need @ 0x004FD9A0`

Active in YR: Yes for AI/non-passive houses through `AI_Building_Strategy`.

Verified behavior:

- Current/player-controlled houses return `0` immediately. Evidence: `0x004FD9A0` decompile.
- The function chooses a side/faction indexed candidate through `FUN_005117D0`, `Rules+0xB40/+0xB4C`, and a bit test against type `+0x6CC`. Evidence: decompile `0x004FDA03..0x004FDA46`; assembly context `0x004FDA5D`.
- If `House+0x15C < 1`, it looks for fallback buildings through `FUN_005051E0(Rules+0x8E4)` then `FUN_005051E0(Rules+0x938)`. If the chosen building is already in `House+0x564C`, it scans the global factory array for an owned factory producing that same building object and compares factory field `+0x60` against wallet balance. If factory remaining/cost field `+0x60` is greater than balance, it returns `1`; if affordable, `0`; if no matching owned factory reaches the end, it returns `1`. Evidence: decompile `0x004FDA46..0x004FDB3B`.
- If `House+0x15C >= 1`, it checks owned buildings for refinery-like rules entries (`Rules+0x8E8` first two entries) on mission/status `0x12` and returns `0` if present; otherwise it performs the same candidate/factory affordability logic for unit-related choice at `House+0x5650`. Evidence: decompile `0x004FDBB3..0x004FDCEF`.

### 3.4 `AI_Manage_Build_Queue @ 0x004FDD10`

Active in YR: Yes for AI houses when `AI_Check_Build_Need` contributes a matching priority in `AI_Building_Strategy`.

Verified behavior:

- It derives whether both `House+0x15C` and `House+0x160` are positive, then chooses candidate base/refinery/conyard objects through `FUN_005051E0` and the same `FUN_005117D0`/`Rules+0xB40` bit-test path. Evidence: decompile `0x004FDD10..0x004FDE79`.
- It walks the AI base-plan queue backwards from `House+0x5714 - 1`, each entry being 16 bytes. If `FUN_0042E820(entry_index)` returns null or cannot pass vtable `+0x98`, it tries `FUN_0042E780` and may call `HouseClass::Sell_Building_At_Cell(entry+4, 0)`, then shifts remaining 16-byte entries down. Evidence: decompile `0x004FDEAA..0x004FE01E`; assembly `0x004FDFEA`.
- If an entry resolves to a building and `Building+0x702 == 0`, it calls vtable `+0x1A0` with `1`. If the building type byte `+0x16B7` is set, it may remove a plan entry; then it adds the building's vtable `+0x2BC` result into a local budget accumulator. Evidence: decompile `0x004FE01E..0x004FE04A`.
- If `Building+0x702 != 0`, it treats the entry as an upgrade case, calls an upgrade vtable path, adds refund/cost through vtable `+0xB8`, calls `BuildingClass::RemoveLastUpgrade`, sets `House+0x5778/+0x5779 = 1`, and calls `FUN_00454CE0`. Evidence: decompile `0x004FE026..0x004FE04A`.
- If the computed budget limit `local_1C` is less than `House+0x30C + accumulated_building_value`, it iterates the global FactoryClass array in reverse. For each owned factory it calls `FactoryClass::Suspend(true)`, `FactoryClass::AbandonProduction`, and vtable `+0x20(1)`; then it clears all four choice slots `+0x564C/+0x5650/+0x5654/+0x5658` to `-1`. Evidence: decompile `0x004FE05B..0x004FE0D1`; assembly `0x004FE05B`.
- After budget cancellation, if the unit-needed flag is set, it writes `House+0x1E4 = 2` and `House+0x5650 = candidate_type_index`. Otherwise it may insert refinery/conyard entries into the 16-byte plan vector, writes `House+0x1E4 = 1`, and calls `AI_Choose_Building`. Evidence: decompile and assembly `0x004FE103`, `0x004FE3A9`.

This is the main verified factory queue/tail interaction in this slot: AI build queue pressure can suspend and abandon all owned factories after the global FactoryClass tick has already run this frame, then the later `HouseClass::Update` dirty/superweapon tail can still run.

### 3.5 Category chooser functions

Active in YR: Yes for AI/non-passive houses every 8 frames when the chooser gate runs.

`AI_Choose_Building @ 0x004FE3E0`:

- Returns `0xF` immediately if `House+0x564C != -1` or if `House+0x60 == 0`. Evidence: decompile `0x004FE3E0`.
- Pulls the first base-plan entry through `FUN_0042EB20(-1)`. Null returns `0xF`. Evidence: `0x004FE407`.
- If the entry's building type has naval byte `+0xCCE != 0` and `House+0x1F0 == 0`, it removes that queue entry and fetches a new first entry. Evidence: `0x004FE41C..0x004FE482`.
- Entry type `-3` removes the entry, calls `AI_ScanBasePerimeter`, and returns `1`. Evidence: `0x004FE49C..0x004FE4FE`.
- Entry type `-1` or a conyard entry at invalid cell uses a random 0..99 roll against `Rules+0xDD8[difficulty]`; if the roll succeeds and `FUN_0050C340(index)` succeeds, it removes the matching entry and returns `1`. Otherwise it calls `AI_ChooseNextProduction @ 0x00506EF0`. Evidence: `0x004FE507..0x004FE638`.
- For normal entries, power/budget gate is `House+0x53A8 + BuildingType+0xEE4 <= House+0x53A4 - House+0x160B4`, with conyard/rules exceptions before a timer/dirty fallback. Evidence: decompile `0x004FE693..0x004FE953`.
- If no offensive units (`House+0x577B == 0`), it can insert a side-specific base defense object from `Rules+0x89C` for side 0, `Rules+0x8A8` for side 2, or `Rules+0x8A0/+0x8A4` logic otherwise. Evidence: `0x004FE76C..0x004FE8F5`.
- The final normal action writes selected building type index to `House+0x564C`. Evidence: assembly before `0x004FEA54`.

`AI_Choose_Unit @ 0x004FEA60`:

- Early returns when `House+0x5650 != -1`. Evidence: assembly `0x004FEA70`.
- It has an early-game special candidate path using `Rules+0x135C[difficulty] * House+0x15C`, fallback `Rules+0x1340[difficulty]`, `Rules+0x1458 <= House+0x24C`, `House+0x242 == 0`, current/player gate false, affordability `type+0x634 <= House+0x1D4`, and `FUN_005051E0(Rules+0x8E4)`/`type+0x408`. Evidence: decompile `0x004FEA7E..0x004FEBDC`.
- Main scoring zeros a 100-entry needed-count array and fills a 100-entry nearest-priority array with `0x7FFFFFFF`, scans active teams owned by the house, counts requested RTTI `0x28` unit types, subtracts already-existing matching `UnitClass` objects that pass `FUN_004DA230`, then builds candidates that pass `HouseClass::CanBuild` and wallet affordability. Evidence: decompile `0x004FEBDC..0x004FEDF2`.
- Random chooser: roll `RandomRanged(0,0x7FFFFFFE)`; if `roll * const < Rules+0x13F4[difficulty] * 0.01`, store nearest/high-priority type, else choose random among max-need candidates. Evidence: decompile `0x004FEDF2..0x004FEEC5`.

`0x004FEEE0` and `0x004FF210`:

- The labels are misleading in current Ghidra/doc wording. `0x004FEEE0` checks/writes `House+0x5654`, scans `InfantryClass`/`InfantryTypeClass`, and counts RTTI `0x10`; `0x004FF210` checks/writes `House+0x5658`, scans `AircraftClass`/`AircraftTypeClass`, and counts RTTI `3`. Evidence: decompile `0x004FEEE0`, `0x004FF210`; assembly entries `0x004FEEED`, `0x004FF21D`.
- Both use the same 100-entry count/nearest arrays, subtract existing owned objects passing `FUN_004DA230`, filter by `HouseClass::CanBuild` and wallet affordability, then use the same `Rules+0x13F4[difficulty]` probability roll to pick nearest/high-priority versus random max-need. Evidence: decompile `0x004FEEE0`, `0x004FF210`.

### 3.6 Dirty-gated `AI_ManageProduction` and `AI_ResumeProduction`

Active in YR: Yes, conditional on `House+0x1FC` in `HouseClass::Update` tail or other direct callers/power paths.

`AI_ManageProduction @ 0x0050AF10`:

- Early-out unless `g_GameActive != 0`.
- Loops entries at `House+0x258/+0x264`.
- Only processes enabled superweapon entries that are active/needing management or when `House+0x1F5 != 0`.
- Scans global BuildingClass array for live, non-limbo buildings owned by the same house; checks three upgrade slots at `Building+0x5EC` plus `BuildingClass::GetSuperWeaponIndex1/2` against the current superweapon index.
- Applies SuperWeaponsAllowed gate: if `SuperWeaponType+0xE7 != 0` and `DAT_00A8B263 == 0`, force grant false.
- Applies low-power gate: if `PowerOutput < PowerDrain`, `PowerDrain != 0`, and `(PowerOutput == 0 || PowerOutput / PowerDrain < 1.0)`, force powered-building byte false.
- Calls type vtable `+0x40`, then either `SuperClass::Suspend(1/0)` or `SuperClass::Deactivate`. On state change, current player clears selected cameo index `DAT_008809A0` if it matches, refreshes sidebar tab, and writes `House+0x1FC = 1`.
- Evidence: decompile `0x0050AF10`; prior assembly contexts `0x0050B10F..0x0050B1A5`.

`AI_ResumeProduction @ 0x0050B1D0`:

- Early-out when `House+0x1F5 != 0`.
- Loops the same `House+0x258/+0x264` entries.
- Processes disabled entries or enabled/post-clicked entries, then scans owned building list in reverse (`House+0x78 - 1` down) for live non-limbo buildings with matching upgrade-slot or primary/secondary superweapon indices.
- Rechecks SuperWeaponsAllowed and computes low-power flag from `House+0x53A4/+0x53A8`.
- Calls `FUN_006CB560(0, this == g_PlayerPtr, low_power_flag)`. For current player it calls `SidebarClass::AddCameo(0x1F, index)`, type vtable `+0x40`, and sidebar tab refresh.
- Evidence: decompile `0x0050B1D0`.

## 4. INI Keys / Defaults Touched

This pass did not trace every parser. The following keys were confirmed as relevant source data from repo INI/defaults and binary consumers, but parser-address proof is deferred unless listed.

| Key / data | Default / examples | Binary consumer in this report | Active in YR |
|---|---|---|---|
| `[General] AIDifficulty=` | `rulesmd.ini:3027 AIDifficulty=0` | difficulty index selects Rules arrays used by AI timers/probabilities | Yes |
| `[General] BuildRefinery=` | `rulesmd.ini:3068 NAREFN,GAREFN,YAREFN` | `Rules+0x8E8` in build-need/refinery checks | Yes |
| `[General] AIBaseSpacing=` | `rulesmd.ini:3132 AIBaseSpacing=1` | base-plan helpers touched through `AI_ChooseNextProduction`; exact reader deferred | Yes |
| `[BuildingType] AIBuildThis=` | many stock YR buildings | feeds AI base-plan/type availability; exact parser/reader not drained here | Yes |
| `[BuildingType] AIBasePlanningSide=` | many stock YR buildings | base-plan side filtering; exact reader not drained here | Yes |
| `[BuildingType] Factory=` | Building/Infantry/Unit/Aircraft factories | current Rust uses it; native factory object/type behavior not rederived here | Yes |
| `[General] BuildSpeed`, `MultipleFactory`, `LowPowerPenaltyModifier` | `BuildSpeed=.7`, `MultipleFactory=0.8`, `LowPowerPenaltyModifier=1` | FactoryClass progress formulas are settled by separate reports, not this slot | Yes |

## 5. Current Rust Implementation Status

| Surface | Current shape | Delta |
|---|---|---|
| `src/sim/world/mod.rs:1427` | `tick_superweapons` runs before production and AI. | DRIFT for dirty-gated manage/resume placement. Native `AI_ManageProduction`/`AI_ResumeProduction` are house-tail calls after chooser gates. |
| `src/sim/world/mod.rs:1690` | `tick_production_with_overlay_registry` runs owner/category production before Rust AI. | DRIFT/unchecked versus native global FactoryClass progress plus later AI build queue cancellation/suspend logic. |
| `src/sim/world/mod.rs:1777..1814` | Rust AI runs and applies commands before defeat detection. | DRIFT versus native defeat-before-later-house-AI ordering already proven in sibling report. |
| `src/sim/ai.rs:64..140` | Simple command generator gated by 30-tick interval and queue emptiness. | Not native `HouseClass::AI_Building_Strategy`/chooser formulas. |
| `src/sim/ai.rs:221..329` | Chooses broad building/unit commands from Rust build options. | Missing native `House+0x1E4` modes, 8-frame chooser cadence, AI base-plan queue, factory-abandon budget path, and native random formulas. |
| `src/sim/production/production_types.rs:198..202` | Production queues are `BTreeMap<owner, BTreeMap<category, VecDeque<BuildQueueItem>>>`. | Deterministic but not native global factory array or House AI base-plan queue. |
| `src/sim/production/production_queue.rs:429..638` | Production pass advances/pops queue fronts and immediately handles completion/spawn/ready placement. | Missing native AI build queue pressure path that can suspend/abandon owned factories and clear choice slots. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `HouseClass::Update` AI/tail call order | verified | `0x004F8FE1..0x004F92FD`, assembly `0x004F92F4` | none for scoped order |
| 8-frame chooser cadence and current/passive gates | verified | `0x004F904F..0x004F9265` | exact signed-mask runtime edge for negative frame counters irrelevant after frame zero |
| `AI_Building_Strategy` urgency/priority/return delay | verified | `0x004FD500`, assembly `0x004FD848`, `0x004FD922` | none for listed gates |
| `AI_Check_Build_Need` factory affordability checks | verified | `0x004FD9A0` | exact meanings of `FUN_004F6540` and `FUN_005117D0` deferred |
| `AI_Manage_Build_Queue` base-plan reverse walk and factory abandon | verified | `0x004FDD10`, assembly `0x004FE05B`, `0x004FE3A9` | exact base-plan helper semantics deferred |
| `AI_Choose_Building` top-level gates | verified | `0x004FE3E0` | `AI_ChooseNextProduction @ 0x00506EF0` not exhausted |
| `AI_Choose_Unit` formulas | verified for visible gates | `0x004FEA60` | exact team member extraction helper `FUN_006EF4D0` deferred |
| `0x004FEEE0` / `0x004FF210` label correction | verified | decompile + assembly entries | update stale docs later |
| `AI_ManageProduction` / `AI_ResumeProduction` dirty-gated superweapon behavior | verified | `0x0050AF10`, `0x0050B1D0` | type vtable `+0x40` and `FUN_006CB560` internals deferred to superweapon docs |
| `AI_DispatchProduction` | touched-not-exhausted | `0x005098F0` | full per-category rally/dispatch semantics out of scope |
| INI parser addresses | deferred | INI grep only | parser proof for AI keys requires separate parser-focused target |
| Current Rust contrast | verified | `src/sim/world/mod.rs`, `src/sim/ai.rs`, `src/sim/production/*` rg scan | no Rust edits made |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Where do AI chooser calls sit relative to dirty manage/resume? -> Choosers run first; `+0x1FC` manage/resume is last in the investigated tail.` (evidence: `0x004F904F..0x004F92FD`)
- `[RESOLVED] OQ-2 - Are `AI_ManageProduction` and `AI_ResumeProduction` build-choice formulas? -> No; they are dirty-gated superweapon manage/resume/cameo functions over `House+0x258/+0x264`.` (evidence: `0x0050AF10`, `0x0050B1D0`)
- `[RESOLVED] OQ-3 - What cadence gates normal category choosers? -> `g_CurrentFrameCounter % 8 == 0` via signed-mask code, AI/non-current/non-passive only.` (evidence: `0x004F904F..0x004F906E`)
- `[RESOLVED] OQ-4 - What cadence gates building strategy? -> `House+0x5634/+0x563C` timer; return delay is `RandomRanged(1,7)+0x69`.` (evidence: `0x004F8FDD..0x004F9043`, `0x004FD91C..0x004FD928`)
- `[RESOLVED] OQ-5 - Can AI build-queue pressure touch factories? -> Yes; `AI_Manage_Build_Queue` can reverse-scan all factories, suspend/abandon owned ones, and clear choice slots.` (evidence: `0x004FE05B..0x004FE0D1`)
- `[RESOLVED] OQ-6 - What is the main AI build queue storage? -> `House+0x5708` entries, 16 bytes each, count at `+0x5714`.` (evidence: `0x004FDD10`, `0x004FE3E0`, `0x00506EF0`)
- `[RESOLVED] OQ-7 - Does Rust currently implement these formulas? -> No; Rust has a simple 30-tick command generator and owner/category queues.` (evidence: `src/sim/ai.rs:30`, `src/sim/ai.rs:64`, `src/sim/production/production_types.rs:198`)
- `[RESOLVED] OQ-8 - Are 0x004FEEE0 and 0x004FF210 labels trustworthy? -> No; evidence shows 0x004FEEE0 handles Infantry arrays/slot +0x5654, while 0x004FF210 handles Aircraft arrays/slot +0x5658.` (evidence: `0x004FEEE0`, `0x004FF210`)
- `[DEFERRED] OQ-9 - Exact `AI_ChooseNextProduction @ 0x00506EF0` placement formula` (category: bounded-cost-too-high; reason: very large helper with multiple grid/threat/weighted-random paths; next-step-if-pursued: target only `AI_ChooseNextProduction` and `FUN_005060B0/FUN_00505FD0`.)
- `[DEFERRED] OQ-10 - Exact semantics of base-plan helpers `FUN_0042EB20`, `FUN_0042EB50`, `FUN_0042E820`, `FUN_0042E780`` (category: bounded-cost-too-high; reason: helper cluster requires separate queue-structure slice; next-step-if-pursued: investigate House AI base-plan vector helpers.)
- `[DEFERRED] OQ-11 - Parser addresses for every AI-related INI key` (category: out-of-scope; reason: current target is tail formulas and runtime gates; next-step-if-pursued: parser-key investigation for AIBuildThis/AIBasePlanningSide/AIBaseSpacing.)
- `[DEFERRED] OQ-12 - Full rally/dispatch semantics in `AI_DispatchProduction`` (category: out-of-scope; reason: dispatch targets and rally behavior are not Factory/House tail order blockers; next-step-if-pursued: trace `AI_DispatchProduction` category cases.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native AI category chooser is a per-house, 8-frame-gated tail after defeat and before `+0x1FC` manage/resume; it uses `House+0x1E4` modes to choose building/unit/infantry/aircraft order. Active in YR: Yes, AI/non-passive. | `0x004F904F..0x004F92FD`, assembly `0x004F9196`, `0x004F92F4` | Rust `ai::tick_ai` is a 30-tick command generator applied before defeat. | `src/sim/world/mod.rs`, `src/sim/ai.rs` | Add a native house-AI tail surface separate from current skirmish helper; gate by native frame modulo and house passive/current flags. | AI house due on frame 8 with `House+0x1E4=1` chooses building, validates it, then only falls through to unit/infantry/aircraft if native buildable check fails. Proposed test: `test_house_ai_tail_mode1_building_validation_before_unit_choices`. | Do not treat "AI after production" as sufficient; the per-house position and choice-slot validation order are observable through RNG, queues, and factory cancels. |
| `AI_Manage_Build_Queue` can suspend/abandon all owned factories and clear all four choice slots when queued building plan cost exceeds the computed budget threshold. Active in YR: Yes. | `0x004FDD10`; assembly `0x004FE05B..0x004FE0D1` | Rust production queues advance/pop; no native AI base-plan pressure path that cancels owned factories. | `src/sim/production/production_queue.rs`, future native House AI queue module | Model AI base-plan queue separately from player production queue; when budget pressure triggers, suspend/abandon owned factory items in global factory order and clear choice slots. | AI with two owned factories and over-budget base plan abandons both factories in reverse global factory array order before choosing next building. Proposed test: `test_ai_build_queue_budget_abandons_owned_factories_and_clears_choices`. | Do not cancel only the current owner/category front item; native scans the global FactoryClass array in reverse and touches every owned factory. |
| Dirty-gated `AI_ManageProduction`/`AI_ResumeProduction` are superweapon manage/resume functions and run after native chooser gates in `HouseClass::Update`. Active in YR: Conditional on `House+0x1FC` or other callers. | `0x004F92ED..0x004F92FD`, `0x0050AF10`, `0x0050B1D0` | Rust superweapon refresh/tick runs early and is not integrated as a house-tail dirty consumer. | `src/sim/superweapon/mod.rs`, `src/sim/world/mod.rs` | Keep factory production progress and build-choice formulas separate from superweapon grant/suspend/resume/cameo dirty tail; consume dirty flag after chooser pass. | Capturing/losing a SW-granting building sets dirty; same owner's house tail first runs due AI chooser, then manage/resume updates SW enabled/suspended/cameo state. Proposed test: `test_house_dirty_superweapon_resume_runs_after_ai_chooser_tail`. | Do not implement `AI_ManageProduction` as generic build queue management; that name is misleading for this verified body. |

## 9. Negative Facts / Do Not Do

- Do not use `AI_ManageProduction @ 0x0050AF10` as the AI build chooser; it loops `House+0x258/+0x264` superweapon entries and building SW slots, not the 16-byte AI base-plan queue. Evidence: `0x0050AF10`.
- Do not use `AI_ResumeProduction @ 0x0050B1D0` as FactoryClass resume; it skips defeated houses, scans owned buildings for superweapon grants, and calls `FUN_006CB560` plus cameo refresh. Evidence: `0x0050B1D0`.
- Do not run native AI build decisions before defeat or outside the house tail; `HouseClass::Update` reaches chooser gates after defeat/scatter and before dirty manage/resume. Evidence: `0x004F8E86..0x004F92FD`.
- Do not assume Rust owner/category `BTreeMap` queue order is equivalent to native AI/factory queue interactions; `AI_Manage_Build_Queue` uses a 16-byte base-plan vector and a reverse global FactoryClass scan. Evidence: `0x004FDD10`, `src/sim/production/production_types.rs:198`.
- Do not trust stale labels saying `0x004FEEE0` is aircraft and `0x004FF210` is infantry without checking fields/arrays; this pass found the opposite by array and choice-slot usage. Evidence: `0x004FEEE0`, `0x004FF210`.

## 10. Remaining Uncertainty

- `AI_ChooseNextProduction @ 0x00506EF0` is touched but not exhausted. It contains grid allocation, direction/threat weighting, building-type weighting, and placement helper calls; it deserves a standalone target.
- Base-plan helper semantics (`FUN_0042EB20`, `FUN_0042EB50`, `FUN_0042E820`, `FUN_0042E780`) remain unresolved beyond their caller effects in this report.
- Parser-address proof for `AIBuildThis`, `AIBasePlanningSide`, `AIBaseSpacing`, and related AI planning keys remains deferred.
- `AI_DispatchProduction @ 0x005098F0` was decompiled to identify its tail placement but its per-category rally/dispatch formulas were not drained.

## 11. Stale Docs / Follow-up Wording

- `docs/contracts/2026-05-28-factory-house-tail-order-implementation-contract.md`: replace "exact AI chooser and `AI_ManageProduction` / `AI_ResumeProduction` formulas" with "exact AI chooser and `AI_Manage_Build_Queue` / `AI_ChooseNextProduction` formulas; `AI_ManageProduction @ 0x0050AF10` and `AI_ResumeProduction @ 0x0050B1D0` are verified dirty-gated superweapon manage/resume functions."
- `docs/research/HOUSECLASS_GHIDRA_REPORT.md`: replace "`AI_Choose_Aircraft (0x4feee0)` and `AI_Choose_Infantry (0x4ff210)`" with "`0x004FEEE0` writes `House+0x5654` and scans Infantry arrays; `0x004FF210` writes `House+0x5658` and scans Aircraft arrays. Update labels only after a dedicated verification pass if naming convention differs."
- `docs/research/BUILDING_SYSTEMS_GHIDRA_REPORT.md`: refine "AI_Manage_Build_Queue applies upgrades if available" with "AI_Manage_Build_Queue can also suspend/abandon every owned factory in reverse global FactoryClass order and clear `+0x564C/+0x5650/+0x5654/+0x5658` when the base-plan budget threshold is exceeded."

## Sources

- Ghidra decompiled/read-only: `0x004F8440`, `0x004FD500`, `0x004FD9A0`, `0x004FDD10`, `0x004FE3E0`, `0x004FEA60`, `0x004FEEE0`, `0x004FF210`, `0x0050AF10`, `0x0050B1D0`, `0x005098F0`, `0x00506EF0`.
- Ghidra assembly context/read-only: `0x004F8FE1`, `0x004F90B3`, `0x004F90D4`, `0x004F9196`, `0x004F92F4`, `0x004FD848`, `0x004FD922`, `0x004FDA5D`, `0x004FE05B`, `0x004FE103`, `0x004FE3A9`, `0x004FE3C2`, `0x004FEA60`, `0x004FEEE0`, `0x004FF210`.
- Prior docs: `docs/research/FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI_GHIDRA_REPORT.md`, `docs/research/HOUSECLASS_MPLAYER_DEFEATED_SCATTER_PRODUCTION_TAIL_RESWARM_20260528.md`, `docs/research/HOUSECLASS_GHIDRA_REPORT.md`, `docs/research/BUILDING_SYSTEMS_GHIDRA_REPORT.md`, `docs/contracts/2026-05-28-factory-house-tail-order-implementation-contract.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: `src/sim/world/mod.rs`, `src/sim/ai.rs`, `src/sim/production/production_queue.rs`, `src/sim/production/production_types.rs`.

## Status

PARTIAL. The slot verifies the requested tail ordering, dirty manage/resume meaning, several chooser gates, and the highest-risk factory-abandon queue interaction. It does not claim full completion of `AI_ChooseNextProduction`, base-plan helper semantics, AI planning key parser addresses, or per-category rally dispatch formulas.
