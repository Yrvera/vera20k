# Phase 7 reverse audit — merged main 409ea3aa (rows 154–165; GSI-09.01 excluded)

Date 2026-09-06. Read-only; no cargo run. Worktree `.claude/worktrees/phase7-audit` (detached at 409ea3aa).
Binary: gamemd.exe (Ghidra `testProsjekt`, image base 0x400000, 10073 functions — confirmed via
`get_current_program_info`). Inputs: the nine `scan-*.md` reports, the 13 PR commit messages/stats
(`git show <merge>^2 --stat`), production code on main, and my own binary reads listed per row.

Dispositions: **IMPL** = IMPLEMENTED+REACHED (production path from `advance_tick`/command/event verified),
**RES** = RESIDUAL-RECORDED (code/doc comment with evidence), **EXCL** = EXCLUDED-WITH-EVIDENCE,
**OMIT** = OMISSION, **REGR** = REGRESSION, **UNRES** = UNRESOLVED. Critic passes were not treated as proof;
each row has at least one native body re-read by me ("audit read").

---

## GSI-09.02 Storage, silos, resource loss

Audit reads: `HouseClass::Removed_From_Game` 0x005025F0 decompile (capacity `-= Type+0x800` at
`nSiloCapacity`; the `param_3 != 0` cash-out loop `RemoveAmount(1.0)→ftol→nScore/Balance` is present and,
per the scan, both callers pass 0); callers of `Add_Tiberium_To_Storage` 0x004F9700 = only
`UnitClass::Mission_Unload` (Weeder branch) — confirms the headline; `Add_Tiberium_Credits` 0x004F9610
callers = `DepositOreFromStorage`, `Sell`, `FUN_00684C30` (FillSilos), `Mission_Unload` — no unlisted writer.

| # | Mechanism | Disposition | Evidence |
|---|---|---|---|
| M1/M2 | house storage mirror + capacity counter | EXCL | storage provably 0 in stock; no visible consumer (scan §2); `Removed_From_Game` audit read |
| M3 | removal cash-out | EXCL | dead branch, callers pass 0 (0x006F6BD1 / 0x0070159A) |
| M4 | Spend_Money | IMPL (bounded, cash path) | `economy.rs:60` |
| M5 | unload capacity gate | IMPL (absence) | `miner_dock_sequence.rs` phase_unloading |
| M6/M7/M8/M11 | sale refund ore, spill, SiloDamage, silos-needed | EXCL | no stock writer of building +0x33C; no `EVA_SilosNeeded` string |
| M9/M10 | refinery tier art / storage pips | IMPL (tier 0 only) | `shp.rs`, `ui_overlays.rs` |
| M12 | purifier count +0x538C | see 09.03 D2 | — |
| M13/M15 | death cargo, save/load | IMPL by exclusion | — |
| M14 | `FillSilos=` | EXCL | checkpoint: 16 retail map hits, all `=no` |
| doc | REFINERY_STORAGE_FLOW report corrections | IMPL | doc lines 21/56/83 carry "corrected 2026-09-06" (slave drains own +0x33C; +0x16CC OrePurifier; +0x538C count) |

Nothing implemented; nothing reachable was missed. Dead helper `anim_class.rs building_storage_fill_level`
(test-only) still present — cosmetic.

---

## GSI-09.03 / 09.05 Ore value, cargo, unload conversion; miner work-site/return decisions

Audit reads: `Harvest_Ore_Tick` 0x0073D556..0x0073D5A1 bytes (FILD Storage; GetTotalAmount; FSUBR;
FCOMP 1.0f @0x007E2AC8; FLD 1.0f; ftol) — one level per bite confirmed; callers of 0x0073D450 = only
`Mission_Harvest`; `Get_Storage_Percentage` 0x007414A0 referenced only from the vtable slot;
`OnConstructionComplete` 0x0044636C..0x0044637C (`Type+0x16CC` gate → `INC [House+0x538C]`).
`BuildingClass` ctor 0x0043BCB3..0x0043BCD0: `MOV EAX,[Type+0x1780]; CMP 1; JGE; MOV 1; PUSH; CALL Set_Contact_Count`.

| # | Mechanism | Disposition | Evidence |
|---|---|---|---|
| M5 | one density level per bite | IMPL | PR #242; `miner_system.rs:1262-1278` (`request = empty.min(1)`), reached via `techno_ai → harvest_mission → process_miner_with_resource_authority → handle_harvest` |
| M9.1-9.5 | own-house, narrow/wide, zone, centre lepton², IsPrimaryFactory | IMPL | PR #245; `find_docking_bay` `miner_system.rs:2229` (`wide` flag, `can_reach` :2424), `are_houses_friendly` no longer referenced in miner_system; IsPrimaryFactory recorded VERA-absent :2227 |
| M9 residual | NumberOfDocks capacity | **REGR (stale claim)** | `miner_system.rs:2214-2219` and `miner_dock.rs:3-8` state "+0xE8 is written only by the RadioClass ctor (=1), never from NumberOfDocks= (UNCHECKED)". The ctor bytes above set capacity = max(`Type+0x1780` NumberOfDocks, 1); PR #253's message and `building_dock.rs:496-503` say so. Inert for stock refineries (NumberOfDocks=1) but the provenance text is wrong and contradicts sibling code. |
| M10 | close/far, chrono, Sqrt_Approx edge | IMPL | #245 `return_exceeds_too_far_threshold` :137 with sqrt_approx table test :249 |
| M11 | no-ore go-home + `House+0x242` | IMPL | PR #248; write mirrored at :1086 (`0x0073E911` re-read: `MOV byte [ECX+0x242],1` after `+0x3D0=1`, return 0x69); latch hashed v132 (`world_hash.rs:995`) |
| M11 AI | AI-house re-scan bridge | RES | `miner_system.rs:1571-1585` VERA-internal label |
| M12/M13 | unload conversion + stat | IMPL (bounded) | unchanged; stock values exact |
| M3/M4 D1 | per-type cargo slots (TIB2/TIB3 maps) | **OMIT (low)** | `CargoBale` still Ore/Gem only (`miner/mod.rs:333-348` values by TiberiumTypeId 0/1); no residual comment names the Vinifera/Aboreus fold. Trigger: maps placing TIB2/TIB3 overlay; effect: one fewer 15-frame unload gate. |
| M12 D2 | purifier under construction pays bonus | **OMIT (low)** | native increments +0x538C only in `OnConstructionComplete` (audit read above); `count_purifiers_for_owner` (`miner_system.rs` ~3010) excludes `dying` only, never `building_up` (`game_entity.rs:613` exists). Trigger: a deposit during a purifier's build-up (once per purifier built, ~0-2 per game); effect: +25% on one or two unloads early. Not recorded anywhere. |
| M6 | destination-present harvest gate | UNRES (low) | retask race only |
| M8 | archive re-check ordering | UNRES (low) | — |
| M14 | OREGATH cadence | UNRES (render lane) | — |

---

## GSI-07.15 Harvest mission / GSI-07.17 Return mission

Audit reads: 0x0073E8F0..0x0073E924 (state-4 entry, `+0x3D0=1`, `Harvester` gate, `+0x242=1`, return 105);
`UnitClass::Enter_Idle_Mode` 0x00738970 decompile (harvester arm: `!In_Radio_Contact`, current/queued ≠ 10 →
Harvest, or Guard for a human when the cell LandType ≠ 5 with `param_2 == 0`); `FootClass::Mission_Enter`
0x004D9290 decompile (epilogue draw `RandomRanged(0,2)` after `GetMissionTimerEntry/ftol`).
07.17: `read_memory 0x007F5EA4` = `d0 2e 5b 00`; `get_xrefs_to 0x005B2ED0` = exactly the 8 vtable DATA refs
(0x007E24D8, 0x007E40F0, 0x007E8EC8, 0x007EB28C, 0x007EDEF4, 0x007F073C, 0x007F4B94, 0x007F5EA4); no code xref.

| # | Mechanism | Disposition | Evidence |
|---|---|---|---|
| M1 | dispatch gate (`current == Harvest`) | RES | `harvest_mission.rs:42-44,142-148` (Miner-present && not Guard) |
| M2 | no Dock= type owned → Guard | IMPL | #248; O(1) owner-type count `entity_store.rs` |
| M3/M4 | states 0/1 | IMPL | #242/#248 |
| M5 | HARV close HELLO ≤ 5 cells; refused-HELLO 0x300 split | IMPL | #248 `try_begin_close_return_radio` :1860 both kinds; staging seed from narrow result recorded :1932 |
| M7 | state-4 tail → Guard, drive off refinery | IMPL | #248 (`Queue_Mission` pure overwrite cited) |
| M8 ii/iii | Guard override arms | IMPL | `mission_handlers.rs:1298-1362`, dispatch at :418; arm (ii) non-human bridge recorded |
| M9 Enter_Idle_Mode harvester arm | Move-arrival / Unlimbo / ChangeOwner → Harvest-or-Guard | **UNRES** | no Rust arm; the old `retask.rs` note is gone and no current comment cites 0x00738970's harvester branch (only the dispatch-gate residual). Behaviour overlap (VERA keeps dispatching Harvest after a Move) masks it except the human "moved onto non-ore → Guard" case. Track A2/B1 prerequisite. |
| M9 right-click-refinery | ForcedReturn cursor | RES | `world_commands.rs:1511` UNCHECKED |
| M10-M13 | MissionControl consumers, passive acquire, visuals, RNG | IMPL (bounded) | unchanged |
| M14 | chrono refused-HELLO / Stop_Moving | UNRES (low) | see 07.37 M6 |
| R1 | Return mission | EXCL | slot/xref evidence above; `mission_handlers.rs` base delay 450; System Map row COMPILED_INACTIVE/ABSENT (#255). Nothing implements it. |

---

## GSI-07.37 / 07.38 Radio contact; docking reservations, queues, handoff

Audit reads: `BuildingClass::Receive_Radio` 0x0E UnitRepair path 0x0043C7E9..0x0043C8E0, 0x0043C93F..0x0043C9F5,
0x0043CCF2: `+0x660` off → 10; linked && 0x22 → 10 → 10; Hospital/Armory (`+0x16C1/+0x16C2`) → 0x0043CB0C
(the 0x22/0x17 loop — not the depot path); `Has_Free_Or_Own_Contact_Slot` → HELLO(2); when full and the sender
is not a contact: occupant/pad distance test, 0x13 to sender, then **return 1 at 0x0043CCF2**. So a waiter at a
saturated depot gets 1 and stays in `Mission_Enter` (whose reply≠1 path would otherwise BREAK + `Enter_Idle_Mode`).
PR #253's model holds on this path. `BuildingClass` ctor capacity read (see 09.03).

| # | Mechanism | Disposition | Evidence |
|---|---|---|---|
| M1 | HELLO/BREAK bookkeeping | IMPL (bounded) | `radio/mod.rs` |
| M1 | sender-full eviction BREAK before HELLO | UNRES (dormant) | not in `transmit_hello`; no comment records it |
| M1 | `Is_Ally_ByObject` receiver gate | UNRES (dormant) | owner-equality; own-house selector makes it unreachable |
| M2 | receiver class effects (Techno/Foot/Unit cases) | RES | `receive.rs:120..` "transitional" |
| M3 | capacity at construction | RES / **REGR (stale text)** | grow-only at first HELLO (`miner_dock_sequence.rs:207`, `building_dock.rs:503`); `miner_dock.rs:3-8` provenance wrong (see 09.03) |
| Group A | bus as decision authority | RES (duplicate authority persists) | `miner_dock.rs:9-13` "transitional mirror … retired in a later slice"; refinery admission still decided by `dock_reservations.hello_or_wait` (`phase_approach` :959, `try_begin_close_return_radio` :1895) with the bus mirroring, while the depot (#253) decides on the bus directly (`radio::transmit` `building_dock.rs:507`). Two admission authorities for the same `radio_contacts` state — recorded but now asymmetric. |
| M4 | Destroy(false) non-ally BREAK; PointerExpired silent null | UNRES (dormant, no stock effect) | — |
| M5 | two-sided +0x418 | RES | — |
| M6 | CMIN HELLO while NavCom set | UNRES | `phase_approach` (:917-990) has no movement gate; whether #248's `handle_return` gates the chrono path on NavCom was not re-verified here |
| M7 | 3+ waiter staging spread | UNRES | needs `Find_Nearby_Passable_Cell` flags |
| M8/M9 | movement authority, release fallbacks | IMPL (bounded) | — |
| M10 | 0x0E `+0x660` gate | RES | `building_dock.rs:433` DRIFT label; EXCL for refineries |
| M12 | depot admission by contact + Enter re-probe, no FIFO, Move-to-exit release | IMPL | #253; `DockReservations` deleted (no refs); `tick_building_docks` reached `world/mod.rs:8063` |
| M12 | depot re-probe RNG draw order | **REGR (RNG order, low)** | native draws `RandomRanged(0,2)` inside each waiter's `Mission_Enter` dispatch in `TechnoClass::AI_Update` object order; Rust draws in `tick_building_docks` (post-production pass, stable-id order, `building_dock.rs:359/516`) — a new Scenario-stream draw site outside object-AI order. Deterministic, but interleaving vs other per-object draws differs whenever a depot waiter coexists with any other Scenario consumer (every repair session). Not recorded as an order residual. |
| M12 | dock phase + re-probe timer unhashed | REGR (recorded) | commit message admits it; relies on hashed RNG cursor/contacts to surface within one cadence |
| M12 | no-funds eject; +0x660 | RES | `building_dock.rs:42-45,433` |
| M13 | airfield parallel `AirfieldDocks` authority | UNRES (pre-existing) | — |
| M14 | war-factory one-sided contact; `mark_live_contact_with` writers | UNRES (pre-existing) | — |
| label | 0x0065AE30 `In_Radio_Contact` | IMPL | Ghidra renamed; docs swept (#255); one "formerly mislabeled" note in `miner_dock_sequence.rs:1271` |

---

## GSI-07.39 / 07.21 Refinery docking/transfer/release; Unload mission

Audit reads: `Mission_Unload` 0x0073E355..0x0073E3BF (dump gate `HarvesterDumpRate*900.0 <= +0xF8`; `vtable+0x468`
particle burst BEFORE the `+0x584` SpecialAnim test; ConditionYellow `FCOMP [Rules+0x1700]`; `SetAnimSlotImage(10, damaged, 0, 0)`);
callers of `UndockUnit` 0x004593A0 = `BuildingClass::ReceiveDamage`, `Sell`, `TemporalClass::Update`.

| # | Mechanism | Disposition | Evidence |
|---|---|---|---|
| A1/A2/A3 | admission chain, refinery mission, pad cell | IMPL (bounded) | unchanged |
| A4 | contacts-gone abort at unload start | IMPL | #246 (`In_Radio_Contact` gate) |
| A5 | credit cadence/value | IMPL (bounded) | — |
| A5 | `+0xF8` incrementer site (±1 frame) | UNRES | `tick_unload_accumulator` :243 has no note on the native incrementer |
| A6 | smoke every due gate, SpecialAnim start-once/cut, damaged variant | IMPL | #246; `building_anim.rs consume_bale_events` reached from finalize; bytes re-read above |
| A7 | release ordering; `Is_Ready_To_Commence` false path | UNRES (low) | — |
| A8 | FreeUnit | RES | `production_refinery.rs:296-335` |
| A10 | refinery destroyed by damage mid-unload → `UndockUnit` (Force_Track push-off) | **OMIT (low-moderate)** | native caller `ReceiveDamage` confirmed; Rust only `abort_missing_unload_building` on the next gate (`miner_dock_sequence.rs:1352`) and sale interrupt (`production_sell.rs:754`); no comment cites the ReceiveDamage caller. Trigger: refinery killed while a miner is on the pad (occasional per skirmish); effect: miner left standing on the rubble footprint vs pushed out along track 0x47. |
| A10 | capture mid-unload | UNRES | `ChangeOwner` not decoded |
| B2 | vehicle transport Unload FSM | IMPL | #251; `transport_unload.rs` reached via `mission_handlers.rs:390-393` (object AI) and `techno_ai.rs:560/1030`; commands from `transport_orders.rs`, `dispatch.rs:2686`, `context_order.rs:717/1041` → `world_commands.rs:1774` |
| B3 | aircraft Unload slot | IMPL | same PR; residuals (+0x418 early-out, carryall, team arms, IsTrain, refused-eject keeps passenger, jumpjet auto-land) recorded `transport_unload.rs:902-953,1013-1057` |
| B1/B4-B9 | +0x2E4 link, Weeder, MCV/deployer/absorb/factory-exit branches | EXCL / adjacent | — |

---

## GSI-09.04 Resource growth and spread

Audit reads: 0x00722C9E..0x00722CE3 (`TEST byte [Scen],0x40` → `FLD [0x007E5138]` 0.3 else `FLD [0x007E1718]` 1.0;
`FILD [+0xA8]`; `FMUL`; ftol; `+0x11C=frame`, `+0x124=result`); callers of `AddToGrowthQueue` 0x007235A0 =
`PlaceTiberium`, `Reduce_Tiberium`, `VoxelAnimClass::AI` (matches table row 6).

| # | Mechanism | Disposition | Evidence |
|---|---|---|---|
| M1/M4-M7/M9-M12 | drivers, processors, predicates, Place/Reduce, twinkle | IMPL (bounded) | production path `tick_ore_growth_rungs` `world/mod.rs:7148` |
| M2 | growth reload `ftol(Growth×0.3)` under bit 0x40 | IMPL | #243; `ore_growth.rs:73-79,178-188` chop helpers, 659 pinned |
| M3 | flag sources (Basic +0x34A6, session bits; General/SpecialFlags gates removed) | IMPL | #243 `scenario_session.rs`; campaign default residual recorded in commit |
| M8 | array-counter rebuild triggers | RES | `ore_growth.rs:690,709` (OQ-38) |
| M13 | legacy scan/reservoir + dead hashed queues | UNRES (pre-existing cleanup) | `tick_ore_growth` :1972 still reachable only via `world/mod.rs:4193` no-overlay fallback |
| M14/M15 | VoxelAnim, Weeder | EXCL | — |

---

## GSI-15.06 EVA queueing and house voice

Audit reads: `PlayEVA` 0x00752700 callers (41 functions) cross-checked against the #254 census: all present,
including `HouseClass::Removed_From_Game` 0x00502776 (decompiled: robot-tank offline line behind `Type+0x40C`/`+0x662` —
census "prerequisite missing"), `LostPoweredCenter`, `RobotTanksBackOnline`, `GoOnline/GoOffline`, `LightningStorm::Process`.
`SuspendEVA` 0x00753570 callers = `NukeFlash::StartScreenFlash`, `RadarClass::PlayRadarMovie`.
`Next_Song` not relevant here. sim/ dependency sweep: no `crate::app|audio|ui|render|net|sidebar` in `src/sim`.

| # | Mechanism | Disposition | Evidence |
|---|---|---|---|
| M1 | evamd-only registry, Type/Priority/Volume | IMPL | #249 `sound_ini.rs:911`, `transitions.rs:777-891` |
| M2/M3/M4 | pending slot, 4 FIFOs, critical/interrupt, 500 ms gap, interrupt-only-when-playing | IMPL | `audio/vox.rs`, pumped every app frame |
| M5 | SuspendEVA producers (nuke flash, radar movie) | RES | commit #249; nuke flash IS stock-skirmish reachable — minor |
| M6 | side column once per match | IMPL | — |
| M8 | queue persistence | RES | — |
| E1 | UnitLost from kill sites, Spawned gate, radar 7 | IMPL | #250 `world/mod.rs:3194`, iron curtain, bridge; aircraft self-destruct silent (corrected) |
| E2 | under-attack: no cooldown, Insignificant, ally branch, own-fire announces | IMPL | `combat/mod.rs:4578`, `radar.rs:63`, cooldown constant gone |
| E3/E4 | low power (BuildPower gate/guard), funds nag (900.0, SpeedNormalize) | IMPL | `house_eva.rs:125-160`, reached `world/mod.rs:7555`, hashed v133; "all human houses" residual recorded |
| E5-E12 | sidebar/placement/sold/repair/capture/SW/defeat/rally/promotion | IMPL | #254 `sidebar_eva.rs`, `eva_producers.rs`; residuals recorded (sync sell, Slave Miner deploy line, UseChargeDrain) |
| E13 | spectator flag | IMPL (SW detected gate) | commit #254 |
| census | primary factory, depot unit sell, spy, beacon, alliance, robot tanks, crates, temporal | RES | `eva_producers.rs:52-85` with addresses |

---

## GSI-15.07 Music

Audit reads: `Next_Song` 0x00720AA0..0x00720ADE (`CMP byte [ESI+0x12],1`; `MOV ECX,0x886B88`; `RandomRanged(0,count-1)`;
1000-try loop; `Is_Allowed` 0x00721140); callers of `Queue_Song` 0x00720B20 = 13 as tabled; `Set_Volume` 0x00721290
callers = options only.

| # | Mechanism | Disposition | Evidence |
|---|---|---|---|
| 2.1/2.2 | THEMEMD-only, index-keyed, per-entry Repeat | IMPL | #247 `theme.rs:880-911` tests |
| 2.3 | Start_Scenario LOADING → Stop(fade) / Queue(-2), Main_Tick head | IMPL | commit; SpawnPick silence residual recorded |
| 2.4 | Next_Song cyclic + shuffle + Is_Allowed; presentation RNG | IMPL | no `main_rng` use in `src/audio` (frontend copy only) |
| 2.5 | ScoreVolume gate in AI/Play | RES | `theme.rs:623` |
| 2.6 | AI poll every screen | IMPL | `sim_tick.rs:614-621` in `pump_audio_service` |
| 2.9 | in-game Sound dialog | RES (out of scope) | commit |
| 2.10 | InGameMusic hold | IMPL (Main_Tick head) | commit |
| 2.11 | theme in savegames | UNRES | save/load lane |
| 2.12 | trigger/WOL/campaign | EXCL | — |

---

## Ranked omissions / regressions

1. **REGR — stale, wrong provenance on radio capacity** (`src/sim/miner/miner_system.rs:2214-2219`,
   `src/sim/miner/miner_dock.rs:3-8`). Claims `+0xE8` is never written from `NumberOfDocks=`; ctor
   0x0043BCBD..0x0043BCD0 does exactly that (`max([Type+0x1780],1) → Set_Contact_Count`), as #253 and
   `building_dock.rs:501-503` state. Trigger: every reader; effect: contradictory evidence in tree
   (ENGINE.md provenance rule). Fix: rewrite both comments, drop "UNCHECKED".
2. **REGR — depot re-probe RNG draw outside object-AI order** (`src/sim/docking/building_dock.rs:359,516`,
   `src/sim/world/mod.rs:8063`). Native: `Mission_Enter` epilogue draw per waiter inside `TechnoClass::AI_Update`
   (0x004D9290 audit read). Rust: separate pass after production, stable-id order. Trigger: any repair-depot
   session with other Scenario-stream consumers (every multi-unit repair); effect: Scenario cursor interleaving
   diverges from native (deterministic, but not native order — parity blocker for a future stream golden).
   Plus dock phase/timer unhashed (recorded).
3. **OMIT — refinery destroyed by damage while a miner is on the pad** (07.39 A10). Native
   `BuildingClass::ReceiveDamage → UndockUnit 0x004593A0` (caller list audit read). Rust: next-gate
   `abort_missing_unload_building` (`miner_dock_sequence.rs:1352`), no track-0x47 push-off, no comment citing
   the ReceiveDamage caller. Frequency: occasional per skirmish. Downstream: miner parked on the footprint.
4. **OMIT — purifier under construction pays the deposit bonus** (09.03 D2). Native `INC [House+0x538C]`
   only at `OnConstructionComplete` 0x0044637C; `count_purifiers_for_owner` never excludes `building_up`.
   Frequency: one or two unloads per purifier built. No residual note.
5. **OMIT — per-type cargo slots** (09.03 D1). Vinifera/Aboreus folded into Ore; map-dependent (TIB2/TIB3
   overlays), one extra 15-frame gate. No residual note.
6. **UNRES — `Enter_Idle_Mode` harvester arm** (07.15 M9). Decompiled 0x00738970 shows Harvest-or-Guard
   selection on Move arrival / Unlimbo / ChangeOwner for harvesters; no Rust arm and no current comment
   cites it (the old retask note is gone). Human Move onto non-ore → native Guard, VERA resumes Harvest.
7. **RES (structural) — two admission authorities for `radio_contacts`**: refinery via
   `ProductionState.dock_reservations`, depot via the bus. Recorded as transitional; now asymmetric after #253.
8. **UNRES — chrono HELLO while driving** (07.37 M6): `phase_approach` has no NavCom gate; not re-verified.
9. Minor recorded-but-stock-reachable residuals: SuspendEVA on nuke flash; A5 `+0xF8` ±1 frame; sender-full
   HELLO eviction BREAK (dormant); legacy ore-growth dead hashed state.

No sim→app/audio dependency introduced. No `#[ignore]` added or removed in the phase diff
(`git diff 4b89ef52..409ea3aa -- src`); 18 test fns removed vs 135 `#[test]` added (removed ones are the
whole-cell-drain and FIFO-depot pins the PRs replaced — consistent with the new contracts, not sampled further).

---

## Global parity harness coverage (`src/sim/world/global_parity_harness_tests.rs`, read only)

Fixture: two houses ("Americans", "Soviet"), both created by `spawn_from_map` as **human**
(`world_spawn.rs:2295` `is_human=true`); GAWEAP + GAREFN(Refinery=yes, 3x3) + HARV(Storage=28, Dock=GAREFN)
+ MTNK + E1 per side; four TIB01 cells at density 11 four cells from the harvester; 600 ticks; scenario
session flags clear (#243: "fixture builds the world with the flags clear"). No depot, no transport/passengers,
no purifier, no AI house, no ore regrowth bit, no music/EVA app layer.

| Mechanism | Exercised? |
|---|---|
| #242 bite size | yes (Scenario pins moved with it) |
| #245 refinery selection | trivially (one own refinery; no allied/unreachable candidate) |
| #248 idle tail / House+0x242 / HELLO ≤5 cells | HELLO path plausibly on return; idle tail no (ore present); latch stays clear |
| #246 SpecialAnim/smoke gates | only if the 28-bale load docks before tick 600 (28×19 f + drives ≈ 550+ f) — not established here |
| #243 growth reload ×0.3 | **no** (bit 0x40 clear) |
| #244 opening credits | no (no bootstrap grant path) |
| #250 house_eva | yes: both houses human, credits 0 < 100, GAWEAP counts as a factory → funds-nag timer state is hashed; UnitLost/under-attack fire if the scripted engagement kills |
| #251 transport unload, #253 depot | **no** |
| #247 music, #249/#254 EVA queue/producers | no (app side) |
| 07.17 Return | no |

Read an unchanged harness hash as "this fixture is unaffected", not as phase coverage.
