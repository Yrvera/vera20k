# GSI-07.15 Harvest mission + GSI-07.17 Return mission — disparity scan

Read-only scan, 2026-09-05. Binary: `gamemd.exe` (Ghidra, image base 0x400000). Rust: worktree
`clean-slate-system-impl-891469` @ 4b89ef52. Retail data: worktree `ini/rulesmd.ini`.
Prior research was spot-checked against the binary; two prior claims are refuted below (§2.4, §2.7).

Vtable slot identities used throughout (MISSIONCLASS_BASE_PROTOTYPES report, re-read at
0x007EDEA8): `+0x1E8` = `Queue_Mission(mission, commence_now)`, `+0x1EC` = `Commence`,
`+0x1F0` = `Assign_Mission`. Mission ids: 5 Guard, 7 Enter, 10 Harvest, 12 Return, 13 Stop.

---

## 1. Scope enumeration (native mechanisms, from bodies + active callers)

| # | Mechanism | Native address | Active callers / reach |
|---|---|---|---|
| M1 | Dispatch + post-handler epilogue write | `MissionClass::Mission_Dispatch @ 0x005B3060`, case 10 → vtable `+0x224`; UnitClass slot `0x007F5E94` = `0x0073E5E0` | `TechnoClass::AI_Update` (0x006FA655) every frame, timer-gated |
| M2 | `UnitClass::Mission_Harvest` preamble (non-harvester 450; slave host; no dock type owned → Guard) | `0x0073E5E0` head | every Harvest dispatch |
| M3 | State 0 LOOKING (full check, archive consume, teleport-dest cancel, long scan, no-ore → state 4) | `0x0073E5E0` case 0; `FootClass::Search_For_Tiberium_And_Move @ 0x004DCFE0`; `Scan_For_Tiberium @ 0x004DD0A0` (vtable +0x338); `Is_Cell_Harvestable @ 0x004DCE80` | HARV/CMIN |
| M4 | State 1 HARVESTING (9-stage StepTimer, `Harvest_Ore_Tick`, continuation short scan, full → state 2 + archive) | case 1; `UnitClass::Harvest_Ore_Tick @ 0x0073D450`; `CellClass::Reduce_Tiberium @ 0x00480A80` | HARV/CMIN |
| M5 | State 2 FINDING HOME (NavCom gate, `Find_Docking_Bay`, TooFar thresholds, HELLO radio, QueueingCell staging) | case 2; `FootClass::Find_Docking_Bay @ 0x004DF040`; HELLO at `0x0073EE51` | HARV/CMIN |
| M6 | State 3 → `Queue_Mission(Enter)` | case 3 (`0x0073EF71` region) | hand-off to Mission_Enter lane |
| M7 | State 4 GOTO-IDLE (RepairBay probe, move off refinery, `Queue_Mission(Guard)`) | case 4 | after the 105-frame no-ore wait |
| M8 | Harvester Guard override (AI re-Harvest; human chrono adjacency / full-teleport re-Harvest) | `UnitClass::Mission_Guard @ 0x00740810` → tail-calls `FootClass::Mission_Guard @ 0x004D5070` | miner on Guard |
| M9 | Assignment sites for mission 10 | `BuildingClass::ExitObject_Main 0x00444ED4` (factory exit), `BuildingClass::OnConstructionComplete 0x00446EA3` (refinery FreeUnit), `ScenarioClass::Read_Units_Section 0x00743623` (map INI), `UnitClass::Enter_Idle_Mode @ 0x00738970`, `EventClass::Execute` MEGAMISSION `0x004C73B9` (mission byte from event via slot +0x4A4 = `0x004DF0E0`), IDLE/Stop arm `0x004C7665..0x004C769C`, `Mission_Deploy` state 4 post-unload (prior report) | all stock |
| M10 | `[Harvest]` MissionControl consumers | `MissionControlClass::Read_INI @ 0x005B3760` (layout: +4 NoThreat, +5 Zombie, +6 Recruitable, +7 Paralyzed, +8 Retaliate, +9 Scatter, +0x10 Rate, +0x18 AARate); readers: `GetMissionTimerEntry @ 0x005B3A00` callers — `TechnoClass::ShouldRetaliate @ 0x007087C0` (+8), `UnitClass::Scatter @ 0x00743A50` (+9, +7), `TechnoClass::Evaluate_Candidate @ 0x006F7CA0` (+4), `Enter_Idle_Mode` (+5, +7) | stock values: `Retaliate=no Recruitable=no Scatter=no Rate=.016` |
| M11 | Passive acquire / retaliation while on Harvest | `TechnoClass::AI_Update` passive block (missions {2,10,5}); `ShouldRetaliate` | HARV `OpportunityFire=yes`, `Primary=20mmRapid` |
| M12 | Harvest visual flag | `UnitClass+0x6D2` written in `0x0073E5E0` (set on state 0→1, cleared on state-0 entry and Harvest_Ore_Tick failure); no facing/turret writes anywhere in the handler | render `DrawExtras 0x0073CEC0` (not inspected) |
| M13 | RNG in the handler | one `Random::RandomRanged(0,2)` in the default epilogue, ECX = `[0x00A8B230]+0x218` (Scenario RNG) at `0x0073EF97` | every epilogue exit |
| M14 | Chrono-miner differences | case 0 CLSID_TeleportLocomotion cancel (`0x0073E7FF..0x0073E851`); case 2 `Stop_Moving` when NavCom && teleporter && dock found; `ChronoHarvTooFarDistance` (Rules+0xD7C); staging via `+0x1618/+0x161C` QueueingCell | CMIN only |
| R1 | Mission_Return (GSI-07.17) | slot `+0x234` = `0x005B2ED0` (`return 0x1C2`) in all 8 vtables; assigners: none in code | see §3 |

Rust production owner and reach (all from `advance_tick`): `sim/world/techno_ai.rs:463 techno_ai_shell`
→ `unit_techno_bracket` (techno_ai.rs:1004) → `sim/miner/harvest_mission.rs:66 dispatch_harvest_for_object`
→ `miner_system.rs:584 process_miner_with_resource_authority` → `handle_search_ore` (:746),
`handle_move_to_ore` (:895), `handle_harvest` (:1022), `handle_return` (:1157), `handle_wait_no_ore` (:1343),
`handle_forced_return` (:1350), `miner_dock_sequence.rs` (Dock cursor). Guard-mission miners go instead to
`techno_ai/mission_handlers.rs:23 dispatch_supported_foot_mission_cadence` (line 54 gate).
`outbound_drive_tests.rs` and `miner_tests.rs` are tests only (not production).

---

## 2. Per-mechanism findings

### 2.1 M1 Dispatch and epilogue — MATCH (bounded)
Native: `Mission_Dispatch` runs the handler only when `start==-1 || frame-start >= delay`, then writes
`+0xC8 = frame`, `+0xD0 = return`. Rust: `harvest_mission.rs:106-160` (`dispatch_timer().due(now)`,
`commit_miner_snapshot` → `write_dispatch_epilogue(now, delay)`). Checked: gate shape, write shape,
`DISPATCH_NEXT_FRAME=1`, Rate epilogue `ftol(.016*900)=14 + RandomRanged(0,2)` on the scenario stream
(`miner_system.rs:355 arm_rate_epilogue`, `world/mod.rs:3432 miner_jitter_rng = scenario_rng`) vs native ECX
`Scenario+0x218` at `0x0073EF97`. **Residual (recorded in the Rust doc, confirmed here):** the Rust gate is
"has Miner component and current != Guard", not "current == Harvest" — a miner on Move/Attack keeps being
Harvest-dispatched. Native routes strictly on `+0xAC`. Trigger: player Move/Attack order on a miner; effect:
VERA miner resumes harvesting after the order where retail decides in `Enter_Idle_Mode` (see 2.9).

### 2.2 M2 Preamble — DRIFT (rare) / EXCLUDED
- Non-harvester → `return 0x1C2`: unreachable in Rust (dispatch is Miner-gated). No effect.
- Slave-host branch (`Type+0x5EC/+0x5ED` && `+0x2D8`): `SlaveManagerClass::HandleReturnedSlaves` then epilogue —
  owned by `slave_miner.rs`, out of scope.
- **No dock building type owned** (`HouseClass::CountOwnedInstances` over `Type+0x3EC` list, count `+0x3F8`,
  every entry 0) → `Queue_Mission(Guard,0)`, `return 1`. Applies to humans too (the
  `IsControlledByHuman` gate only guards the `+0x3F8==0` "no Dock= at all" case). Rust: no equivalent; the
  miner keeps searching/harvesting and `handle_return` parks in `WaitNoOre` when `find_nearest_refinery`
  is `None` (`miner_system.rs:1207`), then `handle_wait_no_ore` re-enters `SearchOre` every 105 frames.
  DRIFT. Trigger: player loses every refinery (a few times per match at most). Player effect: retail miner
  goes idle on Guard immediately (and, for a human, stays idle until re-ordered — see 2.8); VERA miner keeps
  mining and cycling. Downstream: none deterministic beyond the miner itself.

### 2.3 M3 State 0 — MATCH (bounded) with VERA-internal cursor
Native order (case 0): (a) non-weeder && `Get_Storage_Percentage() >= 1.0` → state 2, `return 1`;
(b) `+0x6D2 = 0`; if archive `+0x218` set → `Set_Destination(archive,1)`, clear archive, zone-flag 0;
(c) if locomotor CLSID == TeleportLocomotion && NavCom → `Set_Destination(0,1)`; (d)
`Search_For_Tiberium_And_Move(TiberiumLongScan=48, zoneflag)`: returns FALSE immediately if NavCom set;
scan; if found cell == own cell → TRUE; else `Set_Destination(cell,1)`, FALSE. (e) TRUE → `+0x6D2=1`,
StepTimer `{stage 0, rate 2, dur 2}`, state 1, `return 1`. (f) NavCom set → `+0x3D0=0`, epilogue. (g) no
NavCom && archive → `Set_Destination(archive)`, epilogue. (h) no NavCom && no archive → state 4,
`+0x3D0=1`, `House+0x242=1` (Harvester), `return 0x69` (105) — no RNG draw.

Rust: `handle_search_ore` (full → Return; archive → MoveToOre + epilogue; found → drive + epilogue,
own-cell → per-frame; NoOre → WaitNoOre, delay 105, no draw) and `handle_move_to_ore` (NavCom/movement
present → epilogue; arrival → Harvest; re-scan on re-entry). Checked and matching: scan radius 48 (`rings
1..48` exclusive both sides), ring/arm order `(x+i,y-r),(x+i,y+r),(x-r,y+i),(x+r,y+i)`, strict `<` first-seen
tie-break (`search_local_ore` :2086, `search_local_tiberium` :2029 — note the 2026-07 chrono trace's S0-E
"last-seen wins" verdict is wrong against the body: `if (iVar6 < iVar7)`), centre-cell LandType shortcut
without harvestability, `Is_Cell_Harvestable` = playfield + `Can_Reach_Zone` + LandType 5 +
`Can_Enter_Cell(...)==0` vs `build_scan_filter` (:693). `MoveToOre` (cursor 5) is VERA-internal; native
stays in state 0 while driving — hash-visible only through `handler_state`, not behavior.
Not modelled: `House+0x242` ore-depleted flag (consumer is the AI arm of 2.8 → EXCLUDED, no AI).
UNRESOLVED: the zone flag passed to the scan (0 after archive consume) — Rust always filters by zone.

### 2.4 M4 State 1 — DRIFT (demonstrated, high frequency): extraction amount per bite
Native `Harvest_Ore_Tick @ 0x0073D450`, disassembly `0x0073D556..0x0073D599`:
```
FILD  [Type+0x800]        ; Storage (HARV 40, CMIN 20)
FSTP  float [ESP+0x10]
CALL  StorageClass::GetTotalAmount
FSUBR float [ESP+0x10]    ; st0 = Storage - total
FCOMP float [0x007E2AC8]  ; constant = 0x3F800000 = 1.0f (read_memory)
FNSTSW AX ; TEST AH,0x41 ; JNZ recompute(Storage-total)   ; taken when remaining <= 1.0
FLD   float [0x007E2AC8]  ; otherwise request = 1.0
CALL  Math::ftol ; PUSH EAX ; CALL CellClass::Reduce_Tiberium
```
So the request is `ftol(min(1.0, Storage - total))` = **1 density level per bite**; `Reduce_Tiberium(1)`
decrements `OverlayData` by 1 (partial branch, returns 1) and `AddAmount(1.0, type)`. A density-11 cell
therefore feeds 11 bites and the 12th bite (data 0) takes the full-removal branch, returns 0, and
`Harvest_Ore_Tick` reports failure → continuation scan. The prior report
`HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md` §3.5 ("Empty CMIN: request 20")
and the chrono trace row S1-E are **wrong**; the Ghidra plate comment on `0x0073D450` (2026-07-28) already
states `min(1.0, ...)`.

Rust `handle_harvest` (`miner_system.rs:1056-1090`) requests `empty = capacity_bales - cargo.len()` (40 for a
fresh HARV) and `sim/tiberium/mod.rs:441 reduce_tiberium` removes `min(amount, data+1)` levels — a full cell
in one bite, credited as `removed_amount` bales.

Concrete difference (stock HARV, dense field): native fills in 40 bites × 19 frames = **760 frames
(~51 s)** on ~4 cells thinning one level per bite; VERA fills in ~4 bites (~76 frames) plus short drives, each
bite clearing a cell outright. Per-miner income is several times retail; ore patches vanish cell-by-cell
instead of thinning. Trigger: every harvest cycle of every miner. Corroboration: the harness harvester docks
by tick 599 (memory note), which is impossible at the native rate from spawn.

Other state-1 checks: 9-stage timer × `HarvesterLoadRate` (absent in stock ini → default 2) + 1 dispatch
frame = 19 frames per bite, and the stage counter **is** reset on success (`0x0073D5BE MOV [ESI+0xF8],0`;
the 2026-07 trace row S1-C claiming 2-frame subsequent bales is wrong) — Rust `harvest_tick_interval+1 =
19` MATCH. Full check before `Reduce_Tiberium` (native c) vs Rust `is_full()` first — MATCH. Full →
`+0x6D2=0`, state 2, archive via zone-aware `Scan_For_Tiberium(ShortScan=6)` (vtable +0x338) — Rust
`save_archive_via_short_scan` radius 6 with filter — MATCH bounded. Failure not full → HARV uses
`Search_For_Tiberium_And_Move(ShortScan=6, 0)` (the `Short_And_Move` NoZone variant is weeder-only →
EXCLUDED); hit → stays state 1 (`+0x6D2=1`, `return 1`), miss with no NavCom → state 2 — Rust hit →
`MoveToOre`, miss → `begin_return` — MATCH bounded. Native (a) "NavCom set → TRUE without extraction" has
no Rust guard — rare (needs a destination while in state 1). Full-check asymmetry (`>=1.0` state 0,
`==1.0` state 1) is inert with integer storage.

### 2.5 M5 State 2 — MATCH (chrono) / DRIFT (HARV close return) / UNRESOLVED (refused HELLO)
Native (case 2): (a) CMIN: NavCom && `Find_Docking_Bay(Type+0x3E8,0,0)` → `Stop_Moving`; (b) any NavCom →
epilogue; (c) `bay = Find_Docking_Bay(…,0,0)`; HARV: `dist(bay) <= HarvesterTooFarDistance*256` (5 cells) →
`Transmit_Radio(2 /*HELLO*/, bay)`; reply 1 → state 3 (`0x0073EE51..0x0073EE68`); CMIN: same with
`ChronoHarvTooFarDistance` (50); (d) otherwise `g_MapEditorMode++`, relaxed `Find_Docking_Bay(…,0,1)`,
`g_MapEditorMode--`; if bay && (`dist > 0x300` || teleporter): staging = bay NW cell +
`(+0x1618,+0x161C)` QueueingCell → `Find_Nearby_Passable_Cell` → `Set_Destination(cell)` (or
`Set_Destination(0,1)` if none); (e) always the Rate epilogue (never `return 1`).
Rust `handle_return`: NavCom/movement gate (War only, :1174), chrono far-teleport (>50) / close radio
(≤50) / War far drive (>5) helpers, else adjacency-or-CloseEnough → `Dock` cursor; epilogue armed by the
caller (:626) — MATCH on cadence, thresholds and staging cell.
- HARV within 5 cells: native sends HELLO on that dispatch and hands off to Mission_Enter on acceptance;
  Rust `try_begin_close_return_radio` is chrono-only (:1519 comment) and the HARV path waits for
  adjacency/`CloseEnough` before entering `Dock` (owned by `miner_dock_sequence.rs`, GSI Enter/dock lane).
  DRIFT in contact timing (up to ~4 cells of drive under Harvest instead of Enter); every HARV cycle.
- HELLO refused (refinery busy): native HARV > 3 cells drives to the staging cell, ≤ 3 cells just retries
  every 14-16 frames; native CMIN always `Set_Destination(staging)` = teleport. Rust: `dock_queued` +
  `issue_move_if_idle(staging)` for chrono (:1580) — whether that warps or drives is UNRESOLVED here.
- `Find_Docking_Bay` picks, over the `Dock=` type list, the per-type nearest (slot +0x52C, distance out-param)
  preferring a bay with `+0x3D3` set; Rust `find_nearest_refinery` (:1730) is Euclidean² to the dock cell over
  all friendly refineries. Tie/ordering equivalence UNRESOLVED (needs +0x52C body and +0x3D3 identity).

### 2.6 M6 State 3 — boundary
Native: `Queue_Mission(Enter,0)`, `return 1`. Rust: `Dock` cursor with internal phases; commits Enter at
`miner_dock_sequence.rs:1402 mission_queue_exact`. Everything after belongs to the Mission_Enter/dock lane.

### 2.7 M7 State 4 — DRIFT (demonstrated): idle tail
Native (case 4), reached 105 frames after the no-ore return: if `+0x3D0` (set by the no-ore path):
`Find_Docking_Bay(Rules+0x850 /*RepairBay list*/,0,1)` → `Queue_Mission(Hunt)` if none else
`Queue_Mission(Repair)`; then if the unit's own cell holds a Refinery (`+0x16BB`/`+0x16BC`) →
`Set_Destination(exit cell via 0x00703590)`; then **`Queue_Mission(Guard,0)`** (the last queue wins —
Queue_Mission overwrite semantics assumed, UNCHECKED); Rate epilogue. Net: the miner leaves Harvest for
Guard. What brings it back is 2.8.
Rust `handle_wait_no_ore` (:1343): after 105 frames re-enters `SearchOre` and repeats forever; the Rust doc
already labels this VERA-internal. Player effect: retail human miner with no ore in 48 cells parks on
Guard (idle, selectable "guard" state) until re-ordered; VERA keeps re-scanning every 7 s and resumes the
moment ore grows back. Frequency: late game / depleted fields; AI n/a.

### 2.8 M8 Harvester Guard override — MISSING (human chrono arms) / EXCLUDED (AI)
`UnitClass::Mission_Guard @ 0x00740810`: (i) slave-recall gate; (ii) Harvester/Weeder && **AI house** && owns a
dock type && !(Harvester && `House+0x242`) → `Queue_Mission(Harvest)`, `return 1` — EXCLUDED (no AI yet);
(iii) **human && Teleporter**: any of the 8 neighbour cells holds a Refinery owned by this house →
`Queue_Mission(Harvest)`, `return 1`; else `Get_Storage_Percentage()==1.0` && teleport locomotor `+0x10`
query true → `Queue_Mission(Harvest)` — MISSING in Rust (`mission_handlers.rs:50-56` records it); (iv)
`Type+0x404` DeploysInto in `[General]` list (Rules+0x8B0) && house `+0x1F3` && AI → Unload — EXCLUDED;
(v) Weeder latch — EXCLUDED; then `FootClass::Mission_Guard @ 0x004D5070` (target/retaliate/scan, own lane).
Note: a human **war** miner on Guard never re-Harvests on its own natively. Rust's Unit Guard arm
(`evaluate_foot_guard_cadence`) also never re-queues Harvest → MATCH for HARV, MISSING for CMIN.
Trigger for (iii): Stop/park a chrono miner next to its refinery, or the 2.7 tail — several times a match.

### 2.9 M9 Assignment — mixed
- Factory exit `0x00444ED4`: after `Unlimbo` succeeds, `Queue_Mission(10, EBX)`. Rust
  `world_spawn.rs:802 commit_spawn_harvest_mission` → `mission_assign_exact(Harvest)` on every spawn path
  (:765, :1078, :1256, :1457). Equivalent outcome; commit shape differs (Assign vs Queue) — inert for the
  first dispatch (timer zeroed either way). MATCH bounded.
- Refinery FreeUnit `0x00446E9F`: `Queue_Mission(10,0)` + `Commence`. Rust `production_refinery.rs` spawns
  through the same `commit_spawn_harvest_mission`. MATCH bounded.
- Map `[Units]` mission name (`0x00743623`, `Mission_From_Name`): Rust forces Harvest for placed miners
  (`world_spawn.rs:802` doc). A map-placed miner with `Guard`/`Sleep` would differ — UNRESOLVED (stock
  skirmish maps do not place miners; campaign only).
- `UnitClass::Enter_Idle_Mode @ 0x00738970` harvester arm: path still has steps → return; current or
  queued == Harvest → return; else mission = Harvest, but a **human** house with `param_2==0` and the unit's
  cell LandType != Tiberium(5) → Guard; then `Assign_Target(0)`, `Set_Destination(0,1)`, `Queue_Mission`.
  Reached from Move arrival, Unlimbo, ChangeOwner (84 `+0x484` callers). Rust: no arm; `retask.rs:130-175`
  records the Move-arrival residual accurately. MISSING (Track A2/B1 prerequisite).
- Player Harvest order: MEGAMISSION carries the client's mission byte through `+0x4A4`
  (`FootClass 0x004DF0E0`, passthrough unless 0x1D) into `Queue_Mission(EBX,0)` at `0x004C73B9`; with a
  cell target the destination is set before the queue, so state 0 drives there and scans on arrival. Rust
  `Command::HarvestCell` (`world_commands.rs:1788`) assigns Harvest + `MoveToOre` cursor → drives → scans on
  arrival. MATCH bounded (native target/destination write in the arm not re-read this pass).
- Stop: `0x004C7665..0x004C769C` — `What_Am_I()==1` (Unit), `Type+0xE0E` Harvester, `+0xAC ∈ {10,12}` →
  `Queue_Mission(Guard,0)`, `Commence`. Rust `retask.rs:183 commit_stop_miner_guard` MATCH (verified bytes).
- Player "return to refinery" (right-click refinery): Rust `Command::MinerReturn` → Harvest +
  `ForcedReturn` cursor (VERA-internal, doc says UNCHECKED). Native mission for that action was not
  established this pass (the event byte is client-chosen; expected Enter=7). UNRESOLVED.
- Post-unload: `Mission_Deploy` state 4 → `SetMission(10)` + Rate epilogue (prior report, not re-read).

### 2.10 M10 `[Harvest]` MissionControl consumers
| Key (stock) | Native reader | Rust | Disposition |
|---|---|---|---|
| `Rate=.016` | epilogue `ftol(Rate*900)` = 14 | `mission_base_frames(Harvest)` | MATCH |
| `AARate` (absent → copies Rate, `0x005B3760` tail) | building handlers only | n/a | EXCLUDED for units |
| `Retaliate=no` | `ShouldRetaliate 0x007087C0`: `entry+8 == 0` → no retaliation | `combat_targeting.rs:379` `!entry.retaliate` | MATCH (gate only) |
| `Scatter=no` | `UnitClass::Scatter 0x00743A50`: `entry+9==0 && !forced` → bail | `bump_crush.rs:1038/:1227` | MATCH bounded |
| `Recruitable=no` | reader not located this pass (team recruiting) | none | EXCLUDED (no AI) |
| `NoThreat` (absent) | `Evaluate_Candidate 0x006F7CA0` rejects candidates whose mission has `+4` set | none | EXCLUDED by data |
| `Zombie`/`Paralyzed` (absent) | `Enter_Idle_Mode` +5/+7; `Scatter` +7 | none | EXCLUDED by data |

### 2.11 M11 Passive acquire / retaliation on Harvest — MATCH (bounded)
Native passive block admits missions {Move, Harvest, Guard} with `OpportunityFire` (HARV yes) → a war
miner shoots while harvesting; `Retaliate=no` blocks damage-triggered retaliation. Rust
`techno_ai.rs passive_acquire_gate` admits Harvest; 2.10 covers retaliate. Neither lane's target-scan body
was compared here.

### 2.12 M12 Visuals — MATCH (bounded, render side not inspected)
Native writes only `+0x6D2` (1 from state 0→1 and on continuation hit; 0 on state-0 entry, on
`Harvest_Ore_Tick` failure, and on full). No facing, turret or animation writes in `0x0073E5E0`/`0x0073D450`.
Rust `commit_miner_snapshot` (:517) derives `is_harvesting = cursor == Harvest` — identical truth table
except the 19-frame first-bite wait (native flag already 1, Rust cursor already Harvest → same).

### 2.13 M13 RNG — MATCH
Native: exactly one scenario-stream `RandomRanged(0,2)` per epilogue exit; none on `return 1` or the 105
return. Rust: `arm_rate_epilogue` single draw on `scenario_rng`; per-frame and no-ore paths draw none.
`Find_Nearby_Passable_Cell`, `Scan_For_Tiberium`, `Reduce_Tiberium` partial path draw none (full-removal
spread re-seeding is the tiberium lane).

### 2.14 M14 Chrono miner — MATCH (bounded) + one UNRESOLVED
Inbound-only warp, staging = QueueingCell, sounds both ends, depart shimmer only: see memory note
`reference_chrono_miner_warp_facts` and `docs/research/miner/CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` §14,
`CHRONO_MINER_MISSION_HARVEST_STATE2_RETURN_BRANCH_COORDS_GHIDRA_REPORT.md`. Rust
`try_issue_chrono_far_return_teleport` (>50 cells) then `try_begin_close_return_radio` (≤50) partition the
same way as native (c)/(d). Open: refused-HELLO staging (2.5) and the native case-2 `Stop_Moving` for a
driving CMIN (Rust chrono path has no NavCom gate, :1174 is War-only) — UNRESOLVED, low frequency.

---

## 3. GSI-07.17 Return mission — PROVEN dead (EXCLUDED)

- Slot: `Mission_Dispatch` case `YR_MISSION_RETURN` (12) calls vtable `+0x234`.
- Slot contents (read_memory): FootClass `0x007E8EC8`, InfantryClass `0x007EB28C`, AircraftClass
  `0x007E24D8`, UnitClass `0x007F5EA4` all = `0x005B2ED0`; xrefs to `0x005B2ED0` are exactly the 8 vtables
  (`0x007E24D8, 0x007E40F0, 0x007E8EC8, 0x007EB28C, 0x007EDEF4, 0x007F073C, 0x007F4B94, 0x007F5EA4`) — no
  class overrides it. Body: `return 0x1C2` (450 frames, no side effects).
- Assigners: all 259 `CALL [reg+0x1E8]` (Queue) and 29 `CALL [reg+0x1F0]` (Assign) sites were scanned with
  4-instruction context: zero `PUSH 0xc`. Direct callers of `Queue_Mission 0x005B35E0` / `Assign_Mission
  0x005B2FD0` are only the aircraft wrappers. Variable-mission passthroughs: MEGAMISSION (client action →
  mission byte; no stock action maps to Return — not exhaustively proven, no `PUSH 0xc` anywhere on the
  producer side was searched), `Read_Units_Section` (map INI mission names), TeamClass script action 11
  (`aimd.ini`: every `=11,` line is `11,11` Area Guard, 16 occurrences). `rulesmd.ini:30538 [Return]` is
  an empty `; <unused>` section.
- Readers of the id: IDLE/Stop compares `+0xAC` against 10 or 12 (`0x004C767B/0x004C7680`) — mirrored by
  `retask.rs:197`. `mission_handlers.rs:1313 base_mission_handler_delay` returns the 450-frame stub for
  Return — MATCH.
Verdict: no active-YR code path assigns Return; only map/script data could name it, and then it is a
450-frame no-op. No implementation needed.

---

## 4. Ranked implementation candidates

**Group A — extraction economy (no prerequisite; touches goldens)**
1. **One density level per bite** (§2.4). Change `handle_harvest` to request `min(1, capacity - cargo)`
   (and treat a full-removal return of 0 as failure → continuation scan). Files: `src/sim/miner/miner_system.rs`
   (:1050-1095, `extract_bales_max` test helper :1435), `src/sim/miner/miner_tests.rs` (fixtures assuming
   whole-cell bites, e.g. :5278, :5521), harness/stream hash constants (docking tick moves), plan
   `docs/plans/2026-07-24-gsi-07-15-harvest-filling-return-gate-*` premise. Also fix the wrong §3.5 in
   `docs/research/HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md` and trace row
   S1-E/S1-C. Size: S code, M tests/goldens. Highest player visibility of anything in this lane.

**Group B — idle/guard hand-off (prerequisite: a Unit Guard arm that can re-queue Harvest)**
2. State-4 tail → `Queue_Mission(Guard)` (+ move off refinery cell) instead of perpetual re-scan (§2.7);
   preamble no-dock-type → Guard (§2.2). Files: `miner_system.rs` (`handle_wait_no_ore`, `handle_search_ore`
   NoOre arm, `handle_return` no-refinery arm), `harvest_mission.rs` (dispatch gate stays).
3. Harvester Guard override human-chrono arms (§2.8 iii) in `techno_ai/mission_handlers.rs` Unit Guard arm.
4. `Enter_Idle_Mode` harvester arm + strict `current == Harvest` dispatch gate (§2.1, §2.9) — Track A2/B1;
   files `retask.rs`, `harvest_mission.rs:120-150`, Move handler arrival. Size M each; do 2→3→4 together or
   the miner strands.

**Group C — return contact (prerequisite: Mission_Enter/dock lane)**
5. HARV close-return HELLO at ≤5 cells → Enter (§2.5) and refused-HELLO staging semantics for HARV/CMIN.
   Files: `miner_system.rs` `try_begin_close_return_radio`, `miner_dock_sequence.rs`. Size M.
6. `Find_Docking_Bay` selection order/`+0x3D3` preference vs `find_nearest_refinery` — needs +0x52C body
   first (RE task, S).

**Group D — player return order (RE first)**
7. Establish the native mission for right-click-refinery (expected Enter) and retire the `ForcedReturn`
   cursor if so. Files: `world_commands.rs:1437`, `miner_system.rs:1350`, `app/input/context_order.rs:642`.

---

## 5. Uninspected / unknown
- `Mission_Enter` (state 3 onward), `Mission_Deploy` unload, `BuildingClass::Receive_Radio` dock protocol —
  other lanes; only the hand-off shape was checked.
- `+0x52C` per-type bay search body, `+0x3D3` flag identity, `+0x3D0` identity (labelled "IsUseless").
- MEGAMISSION arm: which target/destination writes precede `0x004C73B9` for a Harvest-with-cell order;
  the client-side action→mission table (Return exclusion on the producer side is by absence of `PUSH 0xc`
  at consumers only).
- `[Harvest] Recruitable` reader; TeamClass/TriggerAction mission-id sources (variable pushes).
- Attack-move onto ore with miners selected — not traced natively.
- Save/load of `+0xBC` handler state and the StepTimer; capture (`ChangeOwner` → `Enter_Idle_Mode`) only
  noted as a caller.
- `Queue_Mission` overwrite semantics for the state-4 Hunt/Repair-then-Guard sequence (assumed last wins).
- Render side of `+0x6D2` (`DrawExtras 0x0073CEC0`) vs Rust `voxel_animation`/`harvest_overlay`.
- Weeder (`Type+0xE0F`) paths: EXCLUDED, no `Weeder=yes` in stock inis (not re-grepped this pass).
