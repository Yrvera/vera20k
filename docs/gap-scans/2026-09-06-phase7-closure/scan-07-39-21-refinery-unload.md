# Disparity scan — GSI-07.39 Refinery docking/transfer/release + GSI-07.21 Unload mission

Date: 2026-09-05. Read-only. Binary: gamemd.exe (image base 0x400000), Ghidra MCP live.
Worktree: `.claude/worktrees/clean-slate-system-impl-891469` (branch feature/phase-7-parity-close-284594).
Retail data: `ini/rulesmd.ini`, `ini/artmd.ini` (YR standalone).

Evidence labels: **BIN** = read live this session (decompile/disassembly/vtable bytes);
**DOC** = prior research doc relied on, spot-checked where stated; **INI** = retail key read this session.

## 1. Scope enumeration (from the binary)

### 1.1 Mission "Unload" (id 16 / 0x10) vtable occupants — slot +0x23C, dispatched by `MissionClass::Mission_Dispatch @ 0x005B3060`

| Class | vtable base (derivation) | +0x23C occupant | Ghidra label | Verdict |
|---|---|---|---|---|
| UnitClass | `0x007F5C70` (+0x194 = `0x00737430` UnitClass::Receive_Radio, BIN) | `0x0073D630` (BIN read `0x007F5EAC`) | `UnitClass__Mission_Unload` (renamed 2026-08-18; old name Mission_Deploy_Building) | the whole unit-side lane |
| BuildingClass | `0x007E3EBC` (Receive_Radio `0x0043C2D0` xref at `0x007E4050` = +0x194, BIN) | `0x0044D880` (BIN read `0x007E40F8`) | `FUN_0044d880` (plate: "slot 26 … likely Mission_Unload") | UnitAbsorb/InfantryAbsorb eject + WeaponsFactory exit; never reached by GAREFN/NAREFN (§2.A2) |
| AircraftClass | `0x007E22A4` (Receive_Radio `0x004190B0` xref at `0x007E2438`; +0x240 = `0x00419C80` Mission_Enter cross-checks) | `0x004151E0` (BIN read `0x007E24E0`; sole xref) | **mislabeled `AircraftClass__Mission_Hunt`** | Nighthawk/carryall unload (§2.B8) |
| FootClass/InfantryClass | — | not overridden (thunk returning 0x1C2 per `MISSION_UNLOAD_GHIDRA_REPORT.md` §3) | — | infantry never unload |

Label traps confirmed live: `0x00740EF0` is slot +0x24C (mission 20 Repair, RepairBay search) — not Unload;
`0x004151E0` sits in the Aircraft Unload slot although labeled Mission_Hunt.

### 1.2 `UnitClass::Mission_Unload @ 0x0073D630` branch map (BIN, disassembly 0x0073D63B–0x0073E5BD)

| Gate (order) | Branch | Stock reach |
|---|---|---|
| `unit+0x2E4 != 0` | reciprocal dock link → `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`, then falls to LAB_0073d672 | not stock DockUnload (DOC `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY`) |
| `Type+0x5E0 (Passengers) > 0` | transport unload FSM on `unit+0xBC` states 0/1/3/4, jump table `0x0073E5C0` | FV, BFRT, LCRF, SAPC, YHVR, HTNK-class carriers… (INI `Passengers=`) |
| `Type+0x6AC (CanPassiveAcquire) && !Type+0xE13 (IsSimpleDeployer)` | one-shot idle/scatter branch, returns `14 + RandomRanged(0,2)` | non-transport, non-harvester units given Unload — edge |
| `Type+0xE0E (Harvester) \|\| Type+0xE0F (Weeder)` | **refinery unload FSM** (`0x0073DEE0`) | HARV/CMIN (INI `Harvester=yes`); Weeder: no stock unit (INI grep `^Weeder=` → none) → EXCLUDED |
| `Type+0x404 (DeploysInto) != 0` | MCV/slave-miner deploy | AMCV/SMIN — adjacent lane, not scanned |
| `Type+0xE13 (IsSimpleDeployer)` | `FUN_00739CD0/FUN_00739AC0` deploy toggle | SCHP — adjacent lane, not scanned |
| else | `thunk_FUN_005b2ef0` (450-frame hold) | — |

### 1.3 Refinery side (BuildingClass)

| Mechanism | Address | Evidence |
|---|---|---|
| Radio admission/handoff | `BuildingClass::Receive_Radio @ 0x0043C2D0` cases 0x0E (accepted cell = NW+(3,1) hardcoded `CONCAT22(psVar5[1]+1, *psVar5+3)`), 0x18, 0x16, 0x15 (`if Type+0x16B3 DockUnload: sender->Queue_Mission(0x10, 0)`) | BIN |
| Refinery mission during unload | unchanged (case 0x15 touches only the sender; building Unload slot `0x0044D880` gated on +0x16AE/+0x16AF/+0x16BD) | BIN |
| Credit transfer | inside unit FSM state 3: `HouseClass::Add_Tiberium_Credits @ 0x004F9610` ×2 (base, purifier bonus) on the **building owner** (`this_00->vtable+0x3C`) | BIN |
| Unload animation | `BuildingClass::SetAnimSlotImage @ 0x00451750` slots 7/10/8, `BuildingClass::ClearAnimSlot @ 0x00451E40`, particle emit `vtable+0x468` (`0x00459900` per DOC anim-slots) | BIN call sites `0x0073E08E/0x0073E3BA/0x0073E517/0x0073E534`, `0x0073E37E` |
| Release | unit FSM state 4 (`0x0073E17F`): no building call except optional radio 3 BREAK | BIN |
| FreeUnit on completion | `BuildingClass::OnConstructionComplete @ 0x00445F80` (`Type+0xEA0` FreeUnit) | BIN |
| Interrupt release | `BuildingClass::UndockUnit @ 0x004593A0` (callers Sell/ReceiveDamage/Temporal per DOC), `ReleaseDockedHarvester @ 0x004595C0` | DOC |

### 1.4 Rust owners (production reachability)

- Miner dock FSM: `src/sim/miner/miner_dock_sequence.rs` via `miner_system.rs:637` ← `harvest_mission.rs::dispatch_harvest_for_object` (Unit arm of the per-object AI shell; timer-gated). Reached.
- Contacts/pad registry: `src/sim/miner/miner_dock.rs::RefineryDockContacts` (+ radio bus mirror `src/sim/radio/mod.rs`).
- Building anim/smoke: `src/sim/world/building_anim.rs::consume_bale_events` (Phase finalize) ← `BaleDepositEvent` pushed at `miner_dock_sequence.rs:1304`.
- HORV/CMON swap: `display_type_override` set `miner_dock_sequence.rs:1127`, read `src/app/presentation/instances/units.rs:314`.
- FreeUnit: `src/sim/production/production_refinery.rs::spawn_completed_refinery_free_units` ← `src/sim/world/mod.rs:6597` (build-up completion). Reached.
- Transport unload: `src/sim/passenger.rs::tick_unloading` / `process_unloading_transport` ← `tick_passenger_system` (`src/sim/world/mod.rs:7719`). **Order source `Command::UnloadPassengers` is produced only for `EntityCategory::Structure` with `can_be_occupied`** (`src/app/input/context_order.rs:704,1009`, `src/app/input/dispatch.rs:2279`). No vehicle or aircraft path produces it.
- `src/sim/docking/building_dock.rs` = repair depot only (mission 20 family); not part of this lane.

## 2. Per-mechanism findings

### A. Refinery side

**A1. Admission → accepted cell → contact → 0x16 → 0x15 → queued mission 0x10.**
Native (BIN case 0x0E/0x15 + DOC canonical synthesis): HELLO admits into Contacts[] (capacity = NumberOfDocks); CAN_DOCK replies 0x12 with NW+(3,1); already-there reply sends 0x18 then 0x16; a later gated 0x16/PerCellProcess sends 0x15; case 0x15 queues mission 0x10 (flag 0) on the sender only — no building state, no +0x2E4 link, no sound.
Rust: phases Approach→MissionEnter→AwaitingAcceptedCell→FaceSync→MissionQueued (`miner_dock_sequence.rs:844-1097`); accepted cell `refinery_can_dock_queue_cell` = (rx+3, ry+1) (`:299`); Enter retry `ftol([Enter]Rate×900)+RandomRanged(0,2)` (`:102`); no queue promotion (`miner_dock.rs:44`).
Disposition: **MATCH (bounded)** — checked inputs: stock GAREFN/NAREFN (`DockUnload=yes, Refinery=yes, NumberOfDocks=1`, INI lines 11726-11729/12519-12521), single miner. Two-miner takeover frame order not re-checked (DOC EDGE).
Frequency: every dock cycle. Player-visible: dock approach/waiting.

**A2. Refinery's own mission while a miner unloads.**
Native (BIN `0x0044D880`): the building Unload slot ejects cargo only for `Type+0x16AE`(UnitAbsorb)/`+0x16AF`(InfantryAbsorb) with cargo, or runs the WeaponsFactory (`+0x16BD`) exit FSM; otherwise `Queue_Mission(5)`; case 0x15 never queues anything on a DockUnload refinery. Stock refineries keep their current mission (Guard) throughout.
Rust: no refinery-side mission change. Disposition: **EXCLUDED for refineries / MATCH by absence.** (Absorb/WeaponsFactory branches are other lanes.)

**A3. Dock pad and refinery rediscovery cell.**
Native (BIN): miner unloads standing on the accepted cell NW+(3,1) (art `RemoveOccupy1=3,1`, INI artmd 1795); state 1/3/4 re-find the refinery at miner cell + `g_dwDirectionOffset_W` = (-1,0) via `Look_up_building_in_cell @ 0x0047C520` (BIN `0x0073E01C-0x0073E05A`). `GetDockCoord` is not consulted in this FSM.
Rust: `refinery_pad_cell` = (rx+3, ry+1) when no `DockingOffset` (`:310`; stock NAREFN's `DockingOffset0` is commented out, INI artmd 1725), `mission_deploy_unload_building` looks up (rx-1, ry) on the miner's occupancy layer (`:501-526`).
Disposition: **MATCH (bounded to stock art).** Residual: a modded `Refinery=yes` building with a live `DockingOffset0` would move the Rust pad while native ignores it (DOC `MISSION_ENTER_REFINERY_DOCK_RESWARM` §5) — mod-only.

**A4. Unload start (mission 0x10 first entry).**
Native (BIN `0x0073DEE0-0x0073E09D`): `PathType__Has_Valid_Steps` (contacts non-empty) else abort: `Enter_Idle_Mode(0,1)` (+0x484), `+0x6D1=0`, locomotor `Is_Moving` → `+0x500`, `Is_Ready_To_Commence` → `Commence`, return 1. Facing gate `((facing>>7)+1)&0x1FE == 0x80` (East 0x4000) else `Do_Turn(0x4000)` when `+0x6AF==0`, return 5. First entry: `+0xF8=0`, `+0x6D1=1`, timer `+0x100..+0x10C = (frame, ?, 1)`, slot 7 `PreProductionAnim` (stock undefined → no-op), `+0xBC=3`; epilogue `ftol([Unload]Rate×900)+RandomRanged(0,2)` = 14..16 (INI `[Unload] Rate=.016`, line 30557).
Rust: `phase_pivoting` (`:1147`): facing gate `dock_pivot_accepts` identical formula (`:77`), wait 5 frames (`MISSION_DEPLOY_FACING_WAIT_FRAMES`), `start_unload_deploy` sets `unload_active`, accumulator 0, display override; delay `mission_base_frames(Unload)=14 + RandomRanged(0,2)` (`:1163-1172`).
Disposition: **MATCH (bounded)** for the healthy path. **DRIFT (minor):** the contacts-empty abort (`Has_Valid_Steps == 0`) has no Rust counterpart in `phase_pivoting`/`phase_unloading`; Rust only aborts when the refinery entity is gone. Trigger: refinery loses the contact between 0x15 and unload start (e.g. sold same frame) — rare.

**A5. Credit transfer cadence and the moment credits are spendable.**
Native (BIN `0x0073E2BF-0x0073E4DA`): state 3 returns 1 every frame; gate `HarvesterDumpRate×900.0 <= (double)unit+0xF8` (14.4 → integer accumulator crosses at 15); on a due gate: particle emit, slot 10 if null, `StorageClass::FindFirstNonEmptySlot`, `amount = GetAmount(slot)` (whole slot), `RemoveAmount`, `Add_Tiberium_Credits(amount, slot)` → `credits(+0x30C) = ftol(TiberiumValue[slot] × HouseType+0x148 × amount)` written synchronously (spendable the same frame), then `bonus = (float)(purifiers(+0x538C) [+ AIVirtualPurifiers[difficulty] for non-human, g_GameMode≠0]) × PurifierBonus(+0xF3C, float32) × amount` and a second `Add_Tiberium_Credits(bonus)` if > 0; `+0xF8=0`. Owner = refinery owner (`this_00->vtable+0x3C`). Ore slot before gems (slot order).
Rust: `phase_unloading` (`:1185-1321`): gate `unload_accumulator >= unload_tick_interval` (15 = ceil(14.4), `mod.rs:305`, `ruleset.rs:2184`), whole-slot drain, SLOT_ORDER Ore→Gem, credits to refinery owner, `apply_income_mult` single trunc, `purifier_bonus_credits` single trunc in i128 (`economy.rs:94`), `effective_purifier_count` (real+virtual), credits entry incremented immediately.
Disposition: **MATCH (bounded)** — checked: stock Value 25/50, PurifierBonus .25 (exact in f32), IncomeMult 1.0, 0..3 purifiers; Rust exact rational equals native f32×f64→ftol for these. Not checked: modded fractional `PurifierBonus`/`Value` where f32 rounding could differ by 1 credit. **UNRESOLVED (±1 frame):** the site that increments `unit+0xF8` each frame was not located; Rust increments after the handler in the same call (`tick_unload_accumulator`, `:833`), which may shift the first drain by one frame relative to native.

**A6. Refinery unloading animation and smoke.**
Native (BIN `0x0073E37A-0x0073E3BF`, `0x0073E4DC-0x0073E534`): on EVERY due gate (including the final one that finds no cargo): `vtable+0x468` particle burst first; then slot 10 `SpecialAnim` (`GAREFNOR`/`NAREFNOR`, INI artmd 1787) is created only while `building+0x584 == NULL` (a running SpecialAnim is never restarted); on the empty gate: slot 8 `ProductionAnim` (stock undefined → no-op), `+0xBC=4`, and `ClearAnimSlot(10)` if the SpecialAnim is still alive. ActiveAnim tiers (slots 3–6) follow house storage in `UpdateAnimation`; stock ore refineries credit directly, so tier stays 0 (`GAREFNL1`) — DOC anim-slots §1, not re-derived.
Rust: one `BaleDepositEvent` per successful slot drain (`:1304`); `consume_bale_events` (`building_anim.rs:184-299`) **replaces** an existing SpecialAnim overlay with a fresh one (`:262-266`) and spawns smoke per event; nothing fires on the empty gate; no slot-10 clear at state 4.
Disposition: **DRIFT (demonstrated).** Concrete: a HARV carrying ore+gems → native: smoke ×3 (ore gate, gem gate, empty gate), SpecialAnim started once at the ore gate and cut at the empty gate (~30 frames later); Rust: smoke ×2, SpecialAnim restarted at frame 0 on the gem gate, plays to its natural end. Ore-only cargo (the common case): native smoke ×2, Rust ×1; native cuts `GAREFNOR` ~15 frames after start, Rust plays it out.
Frequency: every unload (~every 60-90 s per miner). Player-visible: refinery smoke/ore-pile animation cadence.

**A7. Release: exit cell, facing, next mission.**
Native (BIN `0x0073E17F-0x0073E289`): state 4 runs on the dispatch after the empty gate (state 3 returned 1): wait while refinery `+0x57C` (slot 8) is live (stock never); `+0x6D1=0`; if `+0x5A4==0 || queued==-1 || queued==10`: `Queue_Mission(10,0)`, `UnitClass::Is_Ready_To_Commence` (+0x200), BREAK radio 3 to first contact if any (+0x274), `MissionClass::Commence` (+0x1EC); epilogue `ftol([Harvest]Rate×900)+RandomRanged(0,2)`. No exit destination, no facing, no `Force_Track`, no sound.
Rust: `phase_departing` (`:1333-1410`): release pad/contact, BREAK on bus, clear override/movement, `dispatch_delay = 14 + RandomRanged(0,2)`, `mission_queue_exact(Harvest,0)` + `mission_commence_exact`.
Disposition: **MATCH (bounded)** — same-frame effects and one RNG draw. Ordering differs (Rust BREAK before Queue; native Queue→Ready→BREAK→Commence); no consumer found that observes the miner's mission during BREAK (`Receive_Radio` case 3 → `GrandOpening`/Techno base), so no visible difference established. The `Is_Ready_To_Commence` false path (Rust always commences) — UNRESOLVED, not decoded.

**A8. FreeUnit spawn on refinery completion.**
Native (BIN `0x00445F80`, `0x00446B..-0x00446EE2`): after placement, if `Type+0xEA0` (FreeUnit) and human-controlled queue limit not hit: construct UnitClass; primary `Unlimbo(center cell + DAT_0089f698, facing 0xC0)`; on failure two `Find_Nearby_Passable_Cell` searches from the building coordinate (second differs by one flag) each followed by `Unlimbo(..., 0xA0)`; on total failure refund `vtable+0xB8(owner,1)` and delete; on success `Queue_Mission(10,0)`+`Commence`.
Rust: `production_refinery.rs:95-232` (primary (rx+w/2, ry+h/2+1) = (rx+2, ry+2) for 4x3; two fallback searches differing only in overlay rejection; refund; Harvest via spawn path).
Disposition: **MATCH (bounded)** to the design/plan docs; stated residuals in code (`:296-335`): speed-type index 2 substitution, zone requirement dropped, occupancy column reduced. Not re-verified here: `DAT_0089f698` = (0,+1), the `+0x5B4` speed-type argument, and whether the Rust free unit is committed to Harvest with a Commence-equivalent timer.

**A9. War miner vs chrono miner on the refinery side.**
Native: no Teleporter branch in `Receive_Radio` or the unload FSM; differences are `UnloadingClass` (HORV/CMON, INI 8215ff/7351ff) drawn while `+0x6D1` (DOC display timing), `Storage` 40 vs 20 (one ore slot either way), and the inbound warp (unit-side Mission_Harvest/Enter, out of lane).
Rust: `display_type_override` set at unload start, cleared in Departing; `teleport_state` wait in `phase_departing` (`:1334`).
Disposition: **MATCH (bounded).**

**A10. Interrupts (sold/destroyed/captured mid-unload).**
Native: sale/damage/temporal → `UndockUnit @ 0x004593A0` (DOC callers); a refinery vanishing between gates → state 3 finds no building: BREAK if contacts, `Queue_Mission(10, commence_now=1)`, epilogue (`0x0073E30D-0x0073E350`); `+0x6D1` is NOT cleared there (stale HORV frame, DOC).
Rust: sale → `interrupt_refinery_docked_miners` (`production_sell.rs:753`); destruction → next gate `abort_missing_unload_building` (`:699`) (keeps `display_type_override` — consistent with native); capture → only `lifecycle.rs:2513` clears `reserved_refinery/home_refinery` on expiry.
Disposition: **UNRESOLVED** for destruction-by-damage (`ReceiveDamage → UndockUnit` track 0x47 push-off vs Rust waiting for the next gate) and for capture (`ChangeOwner` behavior not decoded). Trigger: refinery killed/captured while a miner is on the pad — occasional in stock skirmish.

### B. Unit side — other Unload occupants

**B1. `+0x2E4` reciprocal link branch** → `ReleaseDockedHarvester`. **EXCLUDED** for stock DockUnload (no stock writer; DOC inventory). Rust keeps `start_refinery_exit_force_track` only for the sale interrupt.

**B2. Transports (`Passengers > 0`), states 0/1/3/4 (BIN `0x0073D70E-0x0073DCCE`).**
Native:
- state 0: locomotor `Is_Moving` → return 10 (wait). If no NavCom dest (+0x5A4==0) and current cell `LandType(+0xEC)==2` (water): `Find_Nearby_Passable_Cell` + `Set_Destination`, return 10 (hover transports drive to land first). Else `FUN_00740B60` (+0x304: octant scoring — passable, no enemy occupant, cost vs current facing, octant 4 penalised −100; returns direction and writes cell, `Empty` = `DAT_00B1CFB8` = (0,0)); if cargo≠0 and cell≠Empty: if `Type+0x808 (TurretCount) > 0` set keep-count `+0x6E4 = (cargo==1 ? 0 : 1)`; locomotor `Do_Turn(dir<<13)`; `+0xBC=1`; return 1. Else `Queue_Mission(5)` and epilogue.
- state 1: when `+0x6AF==0` (turn done) → `+0xBC=3`, return 1.
- state 3: if `cargo(+0x114) > keep(+0x6E4)`: pop cargo HEAD (`FUN_004DE710`→`0x00473430`); scan 8 octants starting from `((facing+0x7FFF)>>12+1)>>1` using `g_DirectionOffsets @ 0x0089F688`, validating each with passenger `Can_Enter_Cell` (+0x1AC, `0x0073F0A0`) at the octant cell and the cell beyond; infantry (RTTI 0xF) sub-cell via `FUN_004ACA10`, vehicles via `Find_Nearby_Passable_Cell`; `Unlimbo` (+0xD8) with facing from the octant; on success: `ClearInOpenTransport` if OpenTopped, `passenger+0x11C=0`, `Queue_Mission(2)`, `Set_Destination(cell,1)`, `LeaveTransportSound (+0x568)` at the transport; on failure `CargoClass::AddPassenger` re-insert. **Every outcome exits through the epilogue** (`ftol([Unload]Rate×900)+RandomRanged(0,2)` = one passenger per 14..16 frames). Else `+0xBC=4`.
- state 4: `Queue_Mission(5)`, `+0xB8=1`, epilogue.
- Cargo order: `CargoClass::AddPassenger @ 0x004733A0` prepends (`passenger->next = head; head = passenger`, BIN) → head-pop unloads the LAST boarded first.
Rust: `Command::UnloadPassengers` sets `OrderIntent::Unloading` (`world_commands.rs:1770-1786`); `tick_unloading` (`passenger.rs:1096-1249`) ejects one passenger **per tick**, first free `NEIGHBORS` cell by entity occupancy only (no terrain/passability, no water check), FIFO (`unload_first` pops front; `board` pushes back, `:83,117`), scatter via `scatter_rng().next_u32()%8` + `issue_direct_move`, no turn, no sound, moves blocked while unloading (`movement_commands.rs:71`).
Disposition: **MISSING in production for vehicles and aircraft** — no input path emits `UnloadPassengers` for a Vehicle/Aircraft entity (`context_order.rs:696-710, 1001-1015`; `dispatch.rs:2270-2298` Vehicle arm only `DeployMcv`). The sim helper exists but is unreachable for them. If wired as-is it would be **DRIFT** on: cadence (5 passengers in 5 frames vs ~70-80 frames), pre-turn, LIFO vs FIFO, water/land and passability validation, Move-mission + Set_Destination vs RNG scatter (extra scenario-RNG draws), `LeaveTransportSound` (`ExitTransport`/`IFVTransform`, INI; no Rust emitter — grep empty), Move-order override (native `Assign_Mission(Move)` interrupts unload; Rust refuses the move).
Frequency: every APC/Flak Track/IFV/BFRT/LCAC/SAPC/YHVR unload — common in stock play. Player-visible: transports cannot be unloaded by deploy/self-click at all today.

**B3. Aircraft Unload slot `0x004151E0` (Nighthawk SHAD `Passengers=5`, INI).**
Native (BIN): state 0: no target (+0x1C8) and altitude float `+0x2E8 == 0.0` → state 3 (else team/airfield handling → state 2 waits for locomotor stop). State 3: if `+0x418` clear: cargo empty → `Enter_Idle_Mode(0,1)`; not carryall (`Type+0xDFC==0`): pop head, `ExitCell_RemoveFromMultiCells` (aircraft), place passenger via `+0x100` at the aircraft's cell, `EnterCell_AddToMultiCells`; team re-add; epilogue per passenger. Carryall → `Carryall_Pickup`.
Rust: none reachable (same missing command; `tick_unloading` has no landed/altitude gate).
Disposition: **MISSING.** Frequency: Nighthawk transport use — moderate.

**B4. Weeder** — EXCLUDED (no stock `Weeder=yes`).
**B5. MCV/slave deploy (`DeploysInto`)**, **B6. IsSimpleDeployer**, **B7. CanPassiveAcquire idle branch**, **B9. Building Unload slot (Bio Reactor/Grinder eject, WeaponsFactory exit)** — adjacent lanes, addresses recorded above; not compared.

## 3. Ranked implementation candidates

**1. Vehicle/aircraft transport unload — command surface + native-shaped Unload FSM (prereq: none beyond existing cargo/passenger components).** Size: L.
- Emit `UnloadPassengers` for Vehicles/Aircraft with non-empty cargo on deploy key and self-click (`src/app/input/dispatch.rs` Vehicle arm, `src/app/input/context_order.rs` both self-click sites); consider committing mission Unload through `src/sim/mission/` instead of `OrderIntent`.
- `src/sim/passenger.rs::tick_unloading`: replace per-tick with per-dispatch cadence (`[Unload] Rate` + `RandomRanged(0,2)` on the scenario stream, no destination RNG), add state 0/1 (wait for stop, water→land move for hover, `FUN_00740B60` octant choice, `Do_Turn`), timer-seeded 8-octant scan with `Can_Enter_Cell`-grade validation (reuse `find_nearby_cell`), LIFO head-pop (`PassengerCargo::board` prepend or `unload_last`), `Queue_Mission(2)` + destination handoff, `TurretCount` keep-1 rule, Guard on completion.
- Audio: `LeaveTransportSound` event (`src/audio/events.rs` + rules field on ObjectType).
- `src/sim/movement/movement_commands.rs::can_accept_destination`: allow a Move order to override unloading.
- Aircraft: landed gate (altitude 0) and same-cell placement.
Validation: seed-fixed test for 5-passenger BFRT (frames between ejections 14..16, order reversed), hover LCRF on water moves to shore first; live smoke for APC deploy.

**2. Refinery SpecialAnim/smoke gate parity (prereq: none).** Size: S.
- `miner_dock_sequence.rs::phase_unloading`: emit one event per due gate (including the empty gate), tagged `{drained: bool, empty: bool}`; `src/sim/components.rs::BaleDepositEvent`.
- `src/sim/world/building_anim.rs::consume_bale_events`: spawn smoke on every event; start SpecialAnim only when no live SpecialAnim overlay; on the empty event remove the SpecialAnim overlay (slot-10 clear).
Validation: overlay-state test for ore-only and ore+gem cargo (smoke counts 2/3, single anim start, cut on empty gate).

**3. Unload-start abort when contacts are gone (prereq: radio bus contact truth).** Size: S. `phase_pivoting`: if the miner has no live contact with the refinery, run the native abort (idle, clear latch/override, resume Harvest via Commence). Files: `miner_dock_sequence.rs`, `src/sim/radio/mod.rs` query.

**4. Refinery destroyed/captured mid-unload (prereq: decode `UndockUnit` damage/temporal callers and `ChangeOwner`).** Size: M, research-gated. Files: `src/sim/combat/mod.rs` destruction hook, `src/sim/world/lifecycle.rs`, `miner_dock_sequence.rs::interrupt_refinery_docked_miners`.

**5. FreeUnit residuals already recorded in code** (zone requirement, speed-type index, Harvest commit check). Size: M, blocked on `find_nearby_cell` zone semantics. Files: `production_refinery.rs`, `src/sim/find_nearby_cell.rs`.

## 4. Uninspected / unknown

- Sender side of 0x16/0x15 (`UnitClass::Receive_Radio @ 0x00737430`, `PerCellProcess @ 0x00739EC0`) and `FootClass::Mission_Enter @ 0x004D9290` retry — relied on DOC (PRIMARY set in `STOCK_REFINERY_DOCK_UNLOAD_LIFECYCLE_DOC_MAP.md`), not re-decompiled.
- The per-frame incrementer of `unit+0xF8` and the `+0x100..+0x10C` timer semantics (A5 ±1 frame).
- `UnitClass::Is_Ready_To_Commence @ 0x00744270` false path in state 4.
- Rust radio bus (`src/sim/radio/mod.rs`) building-side handlers were not read; the registry mirror in `miner_dock.rs` was treated as the decision source per its own doc comment.
- Two-miner contention frame order, save/load of `dock_phase`, capture (`ChangeOwner`) of a refinery with a docked miner.
- Aircraft Unload full body (`0x004151E0`) beyond the state map above; carryall branch.
- Building Unload slot `0x0044D880` (Bio Reactor/Grinder eject, WeaponsFactory exit) and garrison evacuation (not in this slot) — separate lanes.
- `DockingOffset`/`GetDockCoord @ 0x00447B20` interactions for modded refineries.
