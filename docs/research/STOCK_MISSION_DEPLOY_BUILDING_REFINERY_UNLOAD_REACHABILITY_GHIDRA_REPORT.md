# Stock Mission Deploy Building Refinery Unload Reachability - Ghidra Research Report

**Address(es):** `0x0073D630` primary, plus `0x0065AE30`, `0x004595C0`, `0x004593A0`, `0x0043C2D0`, `0x00739EC0`, `0x005B3060`, `0x005B3A00`, `0x0049F2F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock YR `HARV/CMIN -> GAREFN/NAREFN` reachability for `UnitClass::Mission_Deploy_Building @ 0x0073D630`, with emphasis on zero-link versus nonzero `UnitClass+0x2E4`, unload states 3/4, `PathType::Has_Valid_Steps` branch polarity, `ReleaseDockedHarvester`, mission queue/commence behavior, `UnitClass+0x6D1`, and direct returns.  
**Non-Scope:** multi-miner contact saturation and exact frame timing from close-return state 3 into first `Mission_Enter` dispatch; those are separate requested follow-ups.  
**Confidence:** High for static branch reachability and field effects in this slice; medium only for runtime frequency of modded animation waits.  
**Active in YR:** Yes for stock `HARV/CMIN` unloading at `GAREFN/NAREFN`; conditional branches are called out below.

## 1. Overview

Fresh read-only Ghidra checks confirm that the normal stock refinery unload path is the zero `UnitClass+0x2E4` path. The function drains cargo through state 3, transitions to state 4, clears `UnitClass+0x6D1` after any refinery door-animation wait, sets mission `0x0A` (Harvest), optionally sends radio `3`, queues the next mission, and reaches the mission timer epilogue.

The nonzero `UnitClass+0x2E4` branch is a separate conditional branch that calls `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`. That helper contains the `Force_Track(0x47)` exit behavior, but it is not reached by the stock zero-link `GAREFN/NAREFN` completion path.

## 2. Class Layout / Key Offsets

| Offset / field | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Unit `+0x2E4` | top-level branch selector; zero enters normal unload/deploy dispatcher, nonzero calls release helper | `0x0073D63B`, `0x0073D641`, `0x0073D66D` | Conditional; stock refinery path uses zero |
| Unit `+0xBC` | mission substate; stock unload uses states 3 and 4 here | writes at `0x0073E093`, `0x0073E51C`, `0x0073E594` | Yes |
| Unit `+0xB4` | queued mission id tested for override/forced-undock cases | reads at `0x0073E201`, `0x0073E543` | Yes |
| Unit `+0xF8` | dump-rate accumulator | gate `0x0073E35B..0x0073E374`, reset `0x0073E4D0` | Yes |
| Unit `+0x5A4` | Foot/NavCom destination pointer used by state-4 override tests | reads `0x0073E1F0`, `0x0073E539` | Yes |
| Unit byte `+0x6D1` | unload-active latch | set `0x0073DFDA`, clear `0x0073E1F6` and no-valid-steps cleanup `0x0073DEF8` | Yes |
| UnitType `+0x5E0` | SizeLimit/Storage-size style branch before harvester gate | `0x0073D6EC` | Yes, but stock HARV/CMIN do not need it positive |
| UnitType `+0xE0E` | `Harvester=yes` | `0x0073D678`, `rulesmd.ini:[CMIN]/[HARV]` | Yes |
| UnitType `+0xE0F` | `Weeder=yes` | `0x0073D686`; stock HARV/CMIN do not set it | No for stock HARV/CMIN |
| BuildingType `+0x16B3` | `DockUnload=yes`; radio `0x15` queues sender mission `0x10` | `0x0043C2D0` | Yes for GAREFN/NAREFN |
| BuildingType `+0x16BB` | `Refinery=yes`; gates state-4 animation wait | `0x0073E1D5` | Yes for GAREFN/NAREFN |
| Building `+0x57C` | dock close animation pointer tested before clearing `+0x6D1` | `0x0073E1DF` | Conditional; normally null for stock GAREFN/NAREFN |
| Building `+0x2E4` | reciprocal docked-unit pointer used by `ReleaseDockedHarvester` | `0x004595C0` | Conditional; not written by stock DockUnload arrival |
| `DAT_0089F6A0/2` | signed adjacent lookup initialized as `(-1,0)` | `0x0049F2F0`, uses `0x0073E195`, `0x0073E2D5` | Yes |

## 3. Core Logic

### Entry split: zero-link versus nonzero-link

The first branch compares `UnitClass+0x2E4` to zero:

| Branch | Target behavior | Evidence |
|---|---|---|
| `UnitClass+0x2E4 == 0` | jumps to `0x0073D6E6`, then reaches the normal deploy/harvester FSM | `0x0073D63B CMP [ESI+0x2E4],0`; `0x0073D641 JZ 0x0073D6E6` |
| `UnitClass+0x2E4 != 0` | looks up a building and calls `BuildingClass::ReleaseDockedHarvester` | `0x0073D647..0x0073D66D` |

`get_function_callers` for `0x004595C0` found only `UnitClass__Mission_Deploy_Building @ 0x0073D630`, and the xref is the call at `0x0073D66D`. This makes the release helper conditional on entering the nonzero `+0x2E4` branch.

### Stock harvesters reach the harvester block even with SizeLimit <= 0

`0x0073D6EC` checks `UnitType+0x5E0` and jumps to `0x0073DCD3` when the value is `<= 0`. That path can still reach `LAB_0073D672`, where the function tests `UnitType+0xE0E` and `+0xE0F`. Stock `[CMIN]` and `[HARV]` have `Harvester=yes`, so they reach the harvester unload block at `0x0073DEE0`.

This corrects older wording that implied stock miners required the positive `+0x5E0` path to unload.

### PathType helper semantics and branch polarity

`PathType::Has_Valid_Steps @ 0x0065AE30` scans path entries at `param+0xE4` for `param+0xE8` count and returns true if any entry is nonzero. Empty count or all-zero entries return false.

The first harvester-path guard is:

| Condition | Branch | Effect |
|---|---|---|
| `Has_Valid_Steps() != 0` | `0x0073DEE9 JNZ 0x0073DF56` | proceed to RateTimer/facing/state dispatch |
| `Has_Valid_Steps() == 0` | fall through `0x0073DEEB` | force-scatter cleanup, clear `+0x6D1`, optionally stop/queue, direct-return `1` |

The direct `return 5` belongs to the later RateTimer/facing branch, not the no-valid-steps branch. Evidence: `0x0073DF56..0x0073DFBC`.

### State 3 entry and dump loop

When valid steps exist and the RateTimer/facing gate is satisfied, the function checks `UnitClass+0x6D1`.

If `+0x6D1 == 0`, state initialization runs:

| Step | Effect | Evidence |
|---|---|---|
| reset dump accumulator | `Unit+0xF8 = 0` | `0x0073DFD0` |
| set unload-active latch | `Unit+0x6D1 = 1` | `0x0073DFDA` |
| initialize animation timer fields | writes `+0x100..+0x10C` | `0x0073DFE0..0x0073DFFC` |
| if `Harvester=yes`, find refinery at current cell plus `(-1,0)` | `DAT_0089F6A0/2` lookup | `0x0073E013..0x0073E05A`; init `0x0049F2F0` |
| request open-door slot | `SetAnimSlotImage(7, damaged, 0)` if building found | `0x0073E065..0x0073E08E` |
| enter dump state | `Unit+0xBC = 3` | `0x0073E093` |

In state 3, the refinery is re-found by the same `(-1,0)` adjacent lookup. The dump gate compares `HarvesterDumpRate * 900.0` with `Unit+0xF8` at `0x0073E35B..0x0073E374`. On threshold crossing it closes/updates building animations, finds the first non-empty storage slot, removes that slot amount, deposits credits, and resets `+0xF8`.

If no slot exists or no positive amount is removed, the function requests slot 8 close animation if `Refinery=yes`, sets state 4, clears slot 10 if occupied, and direct-returns `1` through `0x0073E5B1..0x0073E5BD`. It does not immediately use the timer epilogue on that empty-cargo transition.

### State 4 stock exit

For stock non-weeder HARV/CMIN, state 4 begins at `0x0073E17F`.

| Order | Behavior | Evidence |
|---|---|---|
| 1 | find refinery at current cell plus `(-1,0)` | `0x0073E181..0x0073E1C6` |
| 2 | if building exists, `Refinery=yes`, and `building+0x57C != 0`, direct-return `1` and keep `+0x6D1` set | `0x0073E1CB..0x0073E1EA` |
| 3 | clear unload-active latch | `0x0073E1F6 MOV [ESI+0x6D1],0` |
| 4 | normal stock branch when no override mission is pending: set mission `0x0A` with parameter `0` | `0x0073E24D..0x0073E254` |
| 5 | if vtable `+0x200` succeeds and `Has_Valid_Steps()` is true, send radio `3` | `0x0073E25A..0x0073E279` |
| 6 | queue/commence the next mission through vtable `+0x1EC` | `0x0073E27F..0x0073E283` |
| 7 | use mission timer epilogue: `GetMissionTimerEntry`, multiply entry `+0x10` by `900.0`, add random `0..2` | `0x0073E289..0x0073E2BE`; `0x005B3A00` |

Important ordering detail: `+0x6D1` is cleared only after the `building+0x57C` wait guard passes. A refinery close animation can therefore keep the unit in dock-active rendering state for additional dispatches. Stock GAREFN/NAREFN usually do not populate this slot, but the branch is live for modded art/rules.

### Direct returns

Not every path converges on the timer epilogue.

| Return | Path | Evidence |
|---|---|---|
| `1` | no-valid-steps cleanup after optional queueing | `0x0073DF49..0x0073DF55` |
| `5` | valid steps, RateTimer/facing gate not satisfied | `0x0073DFB0..0x0073DFBC` |
| `1` | state 4 waiting on `building+0x57C` | branch to `0x0073E5B1` |
| `1` | state 3 post-deposit / empty transition / forced state-4 setup | `0x0073E5B1..0x0073E5BD` |
| timer + random `0..2` | normal state init and normal state-4 handoff | `0x0073E289..0x0073E2BE` |

### ReleaseDockedHarvester conditionality

`BuildingClass::ReleaseDockedHarvester @ 0x004595C0` clears animation slots 10 and 11, may spawn a rules sound/anim, creates slot 12/13 visuals if defined, reads `BuildingClass+0x2E4`, and if that points at a unit:

- clears `UnitClass+0x2E4`;
- stops the unit locomotor;
- calls locomotor slot `+0x70` with track `0x47` and exit coordinate offset `(-0x80,+0x80)`;
- sets unit speed to `1.0`;
- finds/scatters to a passable cell;
- sets mission `2` (Move);
- clears `BuildingClass+0x2E4` and `+0x718`;
- sets the building to Guard and sends radio `3`.

This remains a valid conditional branch. It is not the stock zero-link unload completion path because stock arrival reports and the direct decompile of `0x0043C2D0`/`0x00739EC0` show radio `0x15` queues sender mission `0x10` and does not create reciprocal `+0x2E4` links.

## 4. INI Keys

| INI key | Stock value | Effect in this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[CMIN] Dock` | `NAREFN,GAREFN` | chrono miner refinery candidates | Yes |
| `rulesmd.ini:[CMIN] Harvester` | `yes` | reaches `UnitType+0xE0E` harvester branch | Yes |
| `rulesmd.ini:[CMIN] Storage` | `20` | cargo capacity for state 3 storage drain | Yes |
| `rulesmd.ini:[CMIN] Teleporter` | `yes` | chrono movement identity; not a `Mission_Deploy_Building` unload gate | Yes |
| `rulesmd.ini:[HARV] Dock` | `NAREFN,GAREFN` | war miner refinery candidates | Yes |
| `rulesmd.ini:[HARV] Harvester` | `yes` | same unload branch as CMIN | Yes |
| `rulesmd.ini:[HARV] Storage` | `40` | cargo capacity | Yes |
| `rulesmd.ini:[GAREFN] DockUnload` | `yes` | radio `0x15` queues sender mission `0x10` | Yes |
| `rulesmd.ini:[GAREFN] Refinery` | `yes` | state-4 wait and close-animation branch | Yes |
| `rulesmd.ini:[NAREFN] DockUnload` | `yes` | same as GAREFN | Yes |
| `rulesmd.ini:[NAREFN] Refinery` | `yes` | same as GAREFN | Yes |
| `[General] HarvesterDumpRate` | `0.016` | dump gate threshold: `0.016 * 900 = 14.4` | Yes |
| `[General] PurifierBonus` | `.25` | refinery-owner bonus credit calculation | Yes |
| `[General] ConditionYellow` | `50%` | damaged art variant selection for slots 7/8/10 | Yes |
| `artmd.ini:[GAREFN]/[NAREFN] QueueingCell` | `4,1` | queue/staging context elsewhere; not the state 3/4 adjacent refinery lookup | Conditional data; not used here |

## 5. Integration Points

| Function | Role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `MissionClass::Mission_Dispatch @ 0x005B3060` | mission ID dispatch; mission `0x10` uses vtable slot `+0x23C` | decompile `0x005B3060` | Yes |
| `MissionClass::GetMissionTimerEntry @ 0x005B3A00` | computes current mission timer table entry | decompile `0x005B3A00` | Yes |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | case `0x15` sends sender mission `0x10` for `DockUnload=yes` | decompile `0x0043C2D0` | Yes |
| `UnitClass::PerCellProcess @ 0x00739EC0` | pad arrival sends radio `0x15`; no reciprocal `+0x2E4` write | decompile `0x00739EC0` | Yes |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | stock unload state machine | decompile/disassembly | Yes |
| `PathType::Has_Valid_Steps @ 0x0065AE30` | path-step predicate and state-4 radio `3` condition | decompile `0x0065AE30` | Yes |
| `Foundation_direction_table_init @ 0x0049F2F0` | initializes adjacent lookup as `(-1,0)` | decompile `0x0049F2F0` | Yes |
| `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` | conditional nonzero-link release and `Force_Track(0x47)` | decompile/xref | Conditional |

## 6. Current Rust Implementation Status

Relevant Rust surfaces found by scan:

- `src/sim/miner/mod.rs`: `RefineryDockPhase` high-level dock/unload phases.
- `src/sim/miner/miner_dock_sequence.rs`: approach, mission-enter, pivot, unload, cooldown, and depart behavior.
- `src/sim/miner/miner_dock.rs`: contact/reservation state, including Rust-owned waiting queue and pad/contact maps.
- `src/rules/ruleset.rs` and `src/rules/object_type.rs`: harvester/refinery flags, `Dock=`, `QueueingCell`, `NumberOfDocks`, `HarvesterDumpRate`.

No Rust or repo source files were edited. The current Rust direction already appears to avoid using `ReleaseDockedHarvester` as the normal stock zero-link completion path, but exact state-3 empty-check timing and multi-miner handoff remain adjacent risks.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x0073D630` entry split on unit `+0x2E4` | verified | decompile and disassembly `0x0073D63B..0x0073D66D` | none |
| stock zero-link reachability for HARV/CMIN | verified | `0x0073D641`, `0x0073D672`, INI `Harvester=yes` | none |
| nonzero-link release helper branch | verified | `0x0073D66D`, `0x004595C0`, xref caller list | runtime frequency outside stock refineries |
| `PathType::Has_Valid_Steps` helper semantics | verified | decompile `0x0065AE30` | none |
| first PathType guard polarity | verified | `0x0073DEE2..0x0073DEE9` | none |
| no-valid-steps cleanup | verified | `0x0073DEEB..0x0073DF55` | exact vtable names are not needed for branch polarity |
| RateTimer direct return `5` | verified | `0x0073DF56..0x0073DFBC` | none |
| state 3 initialization and `+0x6D1` set | verified | `0x0073DFBD..0x0073E09D` | none |
| state 3 dump gate and empty transition | verified | `0x0073E355..0x0073E5BD` | exact runtime frame handoff to next miner deferred |
| state 4 wait on `building+0x57C` | verified | `0x0073E1CB..0x0073E1EA` | modded animation lifetime |
| state 4 `+0x6D1` clear | verified | `0x0073E1F6` | none |
| state 4 normal `SetMission(0x0A)` and `QueueMission` | verified | `0x0073E24D..0x0073E283` | mission-table runtime base timing deferred to trace task |
| radio `0x15` handoff to mission `0x10` | verified | `0x0043C2D0` | none |
| pad arrival no reciprocal `+0x2E4` write | verified | `0x00739EC0` | none for stock path |
| multi-miner contact saturation | deferred | out of scope | separate `/re-investigate chrono miner refinery contact saturation...` |
| exact close-return dispatch frame timing | deferred | out of scope | separate `/trace-action chrono miner full cargo close return...` |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is stock refinery unload zero-link or nonzero-link? -> Zero-link; nonzero-link calls release helper.` (evidence: `0x0073D63B`, `0x0073D641`, `0x0073D66D`)
- `[RESOLVED] OQ-02 - Is `ReleaseDockedHarvester` reached by normal stock cargo-empty completion? -> No, it is only the nonzero `+0x2E4` entry branch.` (evidence: `0x0073D66D`, `0x004595C0`, xrefs)
- `[RESOLVED] OQ-03 - Does stock radio `0x15` create reciprocal `+0x2E4` links? -> No; it queues sender mission `0x10` for `DockUnload=yes`.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-04 - Does pad arrival create reciprocal `+0x2E4` links? -> No; it sends radio `0x15` and performs locomotor/contact work.` (evidence: `0x00739EC0`)
- `[RESOLVED] OQ-05 - What does `PathType::Has_Valid_Steps` mean? -> True when any stored step entry is nonzero.` (evidence: `0x0065AE30`)
- `[RESOLVED] OQ-06 - Which way does the first PathType guard branch? -> True proceeds to RateTimer/state dispatch; false cleans up and returns `1`.` (evidence: `0x0073DEE2..0x0073DF55`)
- `[RESOLVED] OQ-07 - Which branch returns `5`? -> RateTimer/facing not-ready branch with valid steps.` (evidence: `0x0073DF56..0x0073DFBC`)
- `[RESOLVED] OQ-08 - When is `+0x6D1` set? -> On first unload init before state 3 is written.` (evidence: `0x0073DFD0..0x0073E093`)
- `[RESOLVED] OQ-09 - When is `+0x6D1` cleared? -> No-valid-steps cleanup clears it immediately; state 4 clears it only after any `building+0x57C` wait passes.` (evidence: `0x0073DEF8`, `0x0073E1CB..0x0073E1F6`)
- `[RESOLVED] OQ-10 - How does normal state 4 resume play? -> Set mission `0x0A`, optional radio `3`, queue mission, timer epilogue.` (evidence: `0x0073E24D..0x0073E2BE`)
- `[RESOLVED] OQ-11 - What is the adjacent refinery lookup? -> current cell plus signed `(-1,0)` from `DAT_0089F6A0/2`.` (evidence: `0x0049F2F0`, `0x0073E181`, `0x0073E2C8`)
- `[DEFERRED] OQ-12 - Exact multi-miner handoff timing after state 4?` (category: `out-of-scope`; reason: separate contact saturation and frame trace follow-ups are already requested; next-step-if-pursued: run the two listed follow-up tasks)
- `[DEFERRED] OQ-13 - Runtime lifetime of modded slot-8 `building+0x57C` animation wait?` (category: `requires-different-system-context`; reason: stock GAREFN/NAREFN normally do not exercise it; next-step-if-pursued: trace `BuildingClass::CreateAnimForSlot` and animation clear for a modded refinery)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock refinery unload completion is zero-link and does not call `ReleaseDockedHarvester` | `0x0073D63B`, `0x0073D641`, `0x0073D66D`, `0x004595C0` | none observed in comments/direction | `src/sim/miner/miner_dock_sequence.rs::phase_departing`, `src/sim/miner/miner_dock.rs` | keep normal GAREFN/NAREFN completion independent from reciprocal `+0x2E4` release semantics | full CMIN unload completes without `Force_Track(0x47)` or release-helper sound/effects | Do not reuse `ReleaseDockedHarvester` as the standard post-unload exit |
| `Has_Valid_Steps()==true` proceeds to RateTimer/state dispatch; false cleans up and returns `1` | `0x0065AE30`, `0x0073DEE2..0x0073DF55` | unchecked exact low-level equivalent | future mission-level miner parity | preserve polarity if porting this state machine | valid path steps should not trigger the no-steps cleanup path | Do not copy older inverted PathType wording |
| Direct `return 5` is the RateTimer/facing wait, not PathType false | `0x0073DF56..0x0073DFBC` | Rust has higher-level pivot/timer phases | `phase_pivoting`, mission timing scheduler | wait until facing/rate gate before first state-3 init | miner turns/waits before unload rather than dumping while misaligned | Do not attach delay `5` to a no-valid-steps condition |
| First state-3 entry sets `+0x6D1=1`, initializes animation timer fields, optionally opens slot 7, then writes state 3 | `0x0073DFBD..0x0073E093` | mostly modeled by dock/unloading display state | `phase_unloading`, rendering display override | keep dock-active visual latch through unload | unloading class/render state begins only after pivot/facing gate | Do not clear dock-active before first dump |
| Empty-cargo transition sets state 4 and direct-returns `1`; normal state-4 handoff happens on a later dispatch | `0x0073E4DC..0x0073E5BD` | approximated by deposit cooldown; exact frame handoff pending trace | `phase_unloading`, `phase_deposit_cooldown`, `phase_departing` | preserve one dispatch separation between last positive drain and state-4 exit | single-slot full miner does not depart on the same dispatch that removes final slot | Defer timing-sensitive changes until the frame trace finishes |
| State 4 waits on `building+0x57C` before clearing `+0x6D1` | `0x0073E1CB..0x0073E1F6` | likely missing for modded slot-8 anims | building anim integration, `phase_departing` | for modded active close animation, keep unit dock-active until slot clears | custom refinery with close production anim holds miner in dock visual state | Do not treat `+0x57C` as a movement/NavCom field |
| Normal state 4 sets mission `0x0A`, optionally radios `3` only if `Has_Valid_Steps()` is true, then queues mission | `0x0073E24D..0x0073E283` | Rust explicitly releases reservation and searches ore | `RefineryDockContacts`, `phase_departing`, miner scheduling | ensure contact/retry release and next harvest scheduling are ordered consistently | after unload, miner resumes ore search/harvest and next queued miner can be admitted | Do not send release/contact messages unconditionally without checking queue handoff behavior |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`: patch any claim that stock `HARV/CMIN` require `UnitType+0x5E0 > 0`; they can reach the harvester branch through the `<= 0` path and `Harvester=yes`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`: patch the `PathType::Has_Valid_Steps` polarity if it says valid steps take cleanup or direct `return 5`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`: narrow any language that frames `ReleaseDockedHarvester` / `Force_Track(0x47)` as normal stock refinery completion.
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md`: its earlier uncertainty about `DAT_0089F6A0` value is superseded by `0x0049F2F0`, which initializes the lookup as `(-1,0)`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/CHRONO_MINER_NAVCOM_RADIO_SYSTEM_MODEL_SYNTHESIS.md`: remains broadly correct; link this report as the canonical branch-polarity and state-4 evidence.

## Sources

- Ghidra read-only decompile/disassembly: `0x0073D630`, `0x0065AE30`, `0x004595C0`, `0x004593A0`, `0x0043C2D0`, `0x00739EC0`, `0x005B3060`, `0x005B3A00`, `0x0049F2F0`, `0x006F4AB0`, `0x004D9290`.
- Ghidra read-only xrefs/callers: `0x004595C0` callers/xrefs; `0x0065AE30` callers.
- Existing reports read/reconciled: `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`, `CHRONO_MINER_NAVCOM_RADIO_SYSTEM_MODEL_SYNTHESIS.md`, `UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`, `miner/STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`, `miner/MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md`, `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scan only: `src/sim/miner/`, `src/rules/`, `src/sim/world/world_hash.rs`.

