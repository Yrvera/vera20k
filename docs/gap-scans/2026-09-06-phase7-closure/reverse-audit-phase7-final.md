# Phase 7 reverse audit — final (merged main d7733d37; GSI-09.01 + PR #257 recheck)

Date 2026-09-06. Read-only; no cargo run, no edits. Worktree `.claude/worktrees/phase7-audit`
(detached at d7733d37 = merge of PR #257). Binary: gamemd.exe, Ghidra image base 0x400000,
10073 functions (confirmed with `get_current_program_info`). Inputs: `scan-09-01-credits.md`,
`reverse-audit-phase7.md` (first audit, rows 154–165), PR #244 (6d8958fa), PR #256 (b1dca5c1),
PR #257 (eff37282) commit messages + diffs, production code on main, and the binary reads listed
below.

Dispositions as in the first audit: **IMPL** = IMPLEMENTED+REACHED (production path from
`advance_tick`/command/event verified), **RES** = RESIDUAL-RECORDED, **EXCL** = EXCLUDED-WITH-EVIDENCE,
**ADJ** = assigned to another row and recorded there, **OMIT / REGR / UNRES** as named.

---

## Part A — GSI-09.01 Credits, transactions, displayed money, transaction ordering

### Fresh binary probes (this session)

1. `get_xrefs_to 0x004F9950` (Add_Credits): 23 call sites — every one is in the scan's §1.3 table.
   Only label note: 0x004575E4 sits in the function Ghidra names `BuildingClass__EjectOccupants`
   (the scan called it an "upgrade-refund loop"; the containing-function identity was not re-derived).
2. `get_xrefs_to 0x004F9790` (Spend_Money): 7 sites (`Receive_Radio` 0x006F4D48, `BuildingClass::Update`
   0x0043FDD1, `UpdateRepairAndPower` 0x004508A3, `AI_Update` 0x006FA1AE, `FactoryClass::AI` ×2,
   `OnSpyInfiltrate` 0x00457453) — matches the scan. No unlisted money writer.
3. `read_memory 0x00843CA4` = `"ResourceDestination\0"`, `"ResourceGatherer"`, `"OpenTopped"`,
   `"Drainable"` at 0x00843CD8 — supports #256's correction of the scan (§2.17 below).

### Native bodies spot-checked myself

- `BuildingClass::Update` 0x0043FB20 (decompile): the ProduceCash block sits in the not-warping arm
  (`vtable+0x1D4`/`+0x1D8` both false); fire test `+0x6D0 == -1 ? dur : (elapsed < dur ? dur-elapsed : skip)`,
  fires iff remaining `== 1`; re-arm `+0x6D0 = frame, +0x6D8 = Type+0x1560` BEFORE the
  `HouseType+0x1A6` passive skip and the `vtable+0x350` operational gate; `Type+0x155C < 1 → Spend_Money(-x)`
  else `Add_Credits`. `credit_income.rs::ProduceCashTimer::fires_now` and `produce_cash_step` match this
  order exactly.
- `BuildingClass::ChangeOwner` 0x004482AA..0x004482F9 (bytes): `EDX=[ESI+0x21C] (old Owner)`,
  `CL=[[EDX+0x34]+0x1A6]` passive test; `EAX=[Type+0x1558]`; `PUSH EAX; MOV ECX,EBX; CALL 0x004F9950`
  where EBX is the new-owner argument (`MOV byte [EBX+0x56F8],1` at 0x004482A3); then `+0x6D0=frame`,
  `+0x6D4=[ESP+0x14]` scratch, `+0x6D8=[Type+0x1560]`. `produce_cash_on_owner_change` matches.
  The decompile also shows `+0x538C` DEC on the old owner and INC on the new owner at the tail —
  consistent with #257's purifier writers.
- `TechnoClass::AI_Update` 0x006FA14B..0x006FA224 (bytes): `CMP [ESI+0x1D0],EBP(0)`; `[Type+0x5ED]`
  test; `EAX=frame; CDQ; IDIV [Rules+0x314]; TEST EDX`; `EDI=[Rules+0x318]`; `Available_Money`
  via slot `+0x18` on `owner+0x24`; `min`; `Spend_Money(owner, EDI)`; `Add_Credits([[+0x1D0]+0x21C], EDI)`;
  second block: `[+0x1CC]` set and `IsAlliedWith 0x004F9A90` → clear anim `+0x1D4`, `target+0x1D0=0`,
  `target->Owner+0x5778=1`, `+0x1CC=0`. `drain_common_step` matches both blocks.
- `TechnoTypeClass::ReadINI` 0x007143EA..0x007143FE (bytes): `PUSH 0x843CA4 ("ResourceDestination")`
  → stored to `+0x5ED`; `+0x5EC` from 0x843CB8. So the money block gates on ResourceDestination, not
  Drainable — the scan's §2.17 ("every Drainable=yes type") was wrong and #256 is right.

### Mechanism table

| § | Mechanism | Disposition | Evidence |
|---|---|---|---|
| 2.1 | wallet model, `Available_Money` | IMPL (bounded, storage = 0) | `credit_income.rs:116-121` cites slot `+0x18`/0x004F6990; house storage EXCL per 09.02 (Weeder-only writer) |
| 2.2 | starting credits | IMPL | unchanged `HouseState::new` |
| 2.3 | AI opening grant `ftol(AICM[diff]×0.01×money)` | IMPL | #244; `scenario_bootstrap.rs:1578-1631` (`X87Chop53` chain, comment now names the FMUL/FIMUL and no longer says "returns the current balance" unqualified); `ruleset.rs:1928` parses `MultiplayerAICM`; reached `scenario_post_map.rs:160` in the Post_Map_Init step order and `app/frontend/skirmish.rs:1156`; xref 0x00686A73 present |
| 2.4 | leftover starting-unit budget | IMPL | #244; `scenario_bootstrap.rs:1966-1985` (`remaining > 0 && !multiplay_passive → credits += remaining`, cites 0x005D6F91..9C); ordering before the grant stated at :1610-1614. The "no eligible candidate aborts generation" residual is in the commit message; no code comment located in the lines read (minor) |
| 2.5 | harvester deposit | IMPL (bounded, stock values) | unchanged `miner_dock_sequence.rs:1386-1422`; #257's `counts_as_purifier` now feeds it |
| 2.6 | Slave Miner deposit | IMPL | #256; `slave_miner.rs:346-436` drains the slave's own storage per whole slot, one truncation per call, cites 0x00522D50/0x006AFBCF/0x00522DCD..DE3 (corrects the scan's "building storage" reading); reached `tick_slave_harvesters → process_slave → handle_slave_deposit`; test :996 |
| 2.7 | factory per-step drain | IMPL (bounded) | unchanged `factory.rs:196-251`; `step_all` at `world/mod.rs:8105` |
| 2.8 | cancel refund | IMPL (bounded) + UNRES (low, unrecorded) | `factory.rs:264-283` refunds `original_balance − balance`; native `GetCost(house) − Balance` at cancel time — differs only when the cost multiplier changed mid-build; no code note |
| 2.9 | sell refund (RefundPercent, human-only, no health term) | ADJ → GSI-09.14 (plan row 177) | code still `SELL_REFUND_PERCENT=50 × health%` (`production_sell.rs:26,55`); **stale System Map claim**: `mechanisms.v1.json:249,341` say "RefundPercent is rules-driven … VERIFIED" — the code uses a constant (see R7) |
| 2.10 | building repair cadence/cost | ADJ → GSI-09.13/07.25 (plan rows 202/203) | `production_sell.rs:828-830` constants unchanged; not absorbed |
| 2.11 | depot repair spend | IMPL (bounded, money side) | `building_dock.rs:724-730` `(credits − cost).max(0)` = Spend_Money cash arm; cost math recorded adjacent :42-43,156-157 |
| 2.12 | FreeUnit unlimbo-fail refund | IMPL (bounded) | unchanged `production_refinery.rs` |
| 2.13 | oil-derrick ProduceCash | IMPL | #256; `credit_income.rs:127-271`; reached `techno_ai.rs:509` (Structure arm of the object-AI shell, every building every frame) and `world/mod.rs:5226` (owner change, before the Techno swap); keys parsed `rules/object_type.rs`; retail `[CAOILD]` rulesmd.ini:13949-13951; residuals recorded in code (EMP, warp gates, `+0x67C`, `PoweredSpecial`) |
| 2.14 | `FUN_00684C30` overflow loop | EXCL (capacity 0 at post-map) | unchanged; `+0x34B2` identity still UNRES |
| 2.15 | removed building with stored ore | EXCL (stock) | unchanged |
| 2.16 | Grinder | ADJ → GSI-09.18 (plan row 248) | no grinder consumer; not absorbed |
| 2.17 | Floating Disc money drain | IMPL | #256; `credit_income.rs:277-472`; link from `Fire_At` arm `combat/mod.rs:8124-8142` → `:6717 install_drain_link` + `Assign_Target(NULL)`; `GetFireError` refusal `:7423`; transfer from all four class arms `techno_ai.rs:487/503/562/817`; cell recheck `:1048`; expiry `lifecycle.rs:2582`; hashed v135 `world_hash.rs:1395-1401`; bytes re-read above; power cutoff `+0x5778`/DrainAnim recorded residuals. Note the scan's "refineries and power plants alike lose money" is corrected: only `ResourceDestination=yes` types (rulesmd.ini 9106, 11766, 12557, 13293) pay |
| 2.18 | spy money steal | ADJ → spy-infiltration family (plan :656-659) | none in src; not absorbed |
| 2.19 | crate money | ADJ → Phase 14 crate authority (`docs/plans/2026-09-01-phase14-crate-authority-foundation-plan.md`) | pickup absent; not absorbed |
| 2.20 | unit sold at depot (`FootClass::OnSold`) | ADJ → 09.14 / 09.13; reachability UNRES | `eva_producers.rs:59` records "no unit selling" |
| 2.21 | map-trigger credit events | UNRES (low, trigger-runtime lane) | no credits event in `trigger_runtime.rs`; stock skirmish maps do not use it |
| 2.22 | displayed counter value + CreditTicks sound | IMPL (app layer) + RES (cadence) | #256; `sidebar_projection.rs:84-111` (`Some(CreditTick)` only on a changed step, cites 0x004A2740..51), `sidebar_render.rs:41-83` emits one `GameSoundEvent::CreditTick` per changed step, `building_anim.rs:333` plays at 0.5 volume (0x004A2519 `PUSH 0x3f000000`), `ruleset.rs:2062` parses `CreditTicks`; no sim dependency; display-dispatch cadence residual at `sidebar_projection.rs:120` |
| 2.23 | same-tick ordering (object-loop spend before factory) | UNRES (DRIFT, low, **unrecorded**) | `advance_tick`: object AI → `step_all` :8105 → `tick_repairs` :8119 → `tick_building_docks` :8120; native runs repair/depot spend in the object loop before `FactoryClass::AI`. Only observable at a near-zero balance. No comment names it; scan Group A #5 said "land with the repair item" (09.13) |
| 2.24 | statistics (`+0x54E8`, `+0x2DC`) | IMPL (bounded) + UNRES (TotalSpent consumer) | unchanged |
| 2.25 | IncomeMult | IMPL (bounded, 1.0 stock) | unchanged |

Counts: IMPL 15 · EXCL 2 · ADJ 6 · UNRES 2 (2.21, 2.23; plus sub-residuals on 2.8/2.22/2.24) · OMIT 0 · REGR 0.

Two scan corrections landed by #256 and confirmed here: slave deposit drains the slave's storage
(not the building's); drain money gates on `ResourceDestination` (+0x5ED), not `Drainable` (+0x5EF).

---

## Part B — PR #257 recheck (eff37282, 9 files, +949/−123)

Diff read in full except `miner_tests.rs` (names and count only). No `sim/` file imports
`app|audio|ui|render|net|sidebar`. `git diff … | grep -c '^-.*#\[test\]'` = 0 (no test removed);
`deploy_tests.rs` change is visibility only.

| # | Item | Verdict | Notes |
|---|---|---|---|
| 1 | depot re-probe in the unit's own dispatch slot | IMPL, but **REGR for harvesters** | `mission_handlers.rs:172` arm `(Unit, Enter) if depot_dock_state` runs behind the `timer_due` gate (mission dispatch timer, hashed v29); `tick_building_docks` :585-612 only reads `linked`. Test `waiter_probe_draw_sits_in_the_units_own_ai_slot_in_object_order` pins draw order. **But** the miner gate at `mission_handlers.rs:57-61` (`entity.miner.is_some() && mission ∉ {Guard, Move} → return`) precedes the arm, so a harvester on `Command::RepairAtDepot` (`world_commands.rs:1526`) never HELLOs: it drives to the depot and parks in `WaitForDock` (:614). Pre-#257 the probe in `tick_building_docks` was miner-agnostic. No test sends a HARV to a depot. Caveat: the Harvest handler ignores `dock_state` (no reference in `src/sim/miner/`), so a non-idle miner was already contended before #257; the exact prior outcome is UNCHECKED, but the HELLO is now provably unreachable. `DockState.enter_retry` kept as an unhashed mirror (gate is the mission timer) — acceptable, recorded :92-97 |
| 2 | refinery destroyed while a miner unloads | IMPL | `undock_refinery_unit_on_death` called at `BeforeDeathEffects` (`world/mod.rs:1977`) and both `immediate_uninit_ids` sites (:3038, :7798); reuses `interrupt_refinery_docked_miners` (`miner_dock_sequence.rs:582-665`): contact break on the transitional `dock_reservations` authority, cargo kept, `start_refinery_exit_force_track` only when on pad, no RNG. Second call is a no-op (reservation already cancelled). VERA-internal phase reset recorded in the doc comment |
| 3 | purifier under construction | IMPL | `counts_as_purifier` (`miner_system.rs:3025-3050`) shared by `count_purifiers_for_owner` and `refresh_economy_shadow`; native writers (OnConstructionComplete INC, ChangeOwner DEC/INC, Limbo DEC) cited and consistent with the ChangeOwner decompile read above. Economy `purifier_count` is hashed, so the change is deterministic |
| 4 | `Enter_Idle_Mode` harvester arm | IMPL (Move arrival only) | `harvester_enter_idle_mode_evaluation` (`mission_handlers.rs:677-760`): radio-contact early-out, Harvest current/queued early-out, `IsControlledByHuman` via `is_controlled_by_human(game_mode_nonzero)`, LandType from resolved terrain; Move gate added in `harvest_mission.rs:157-163` and the foot dispatch; Move retask releases the refinery reservation (`world_commands.rs:1488`). Residual: only the Move-arrival caller is modelled; the `Unlimbo`/`ChangeOwner` callers (first audit M9) — a mind-controlled or captured miner — remain UNRES and are not recorded (R2) |
| 5 | stale `+0xE8` provenance | IMPL | `miner_dock.rs:1-12`, `miner_system.rs:2223-2233` rewritten with the ctor bytes; "UNCHECKED" removed |
| 6 | per-type cargo slots | RES | `miner_system.rs:1290-1300` names trigger/effect/frequency |

---

## Part C — Consolidated remaining findings (all thirteen rows), ranked

First-audit items **closed by #257**: (1) stale radio-capacity provenance, (2) depot re-probe RNG order
(dock phase/timer still unhashed — recorded), (3) refinery destroyed mid-unload, (4) purifier under
construction, (5) per-type cargo slots (now recorded), (6) `Enter_Idle_Mode` harvester arm — Move
arrival only; the other callers stay open (R2).

| Rank | Class | Finding | Trigger / effect / frequency | Where |
|---|---|---|---|---|
| R1 | **REGR (new, #257)** | Harvester ordered to a service depot never HELLOs the depot | Trigger: `Command::RepairAtDepot` on a damaged war/chrono miner. Effect: no repair; miner parks in `WaitForDock` (or is pulled off by the Harvest handler, which ignores `dock_state`). Frequency: routine player action. Downstream: money-side depot spend for miners never exercised | `src/sim/world/techno_ai/mission_handlers.rs:57-61` (miner gate) vs `:172` (Enter arm); `src/sim/docking/building_dock.rs:585-614`; `src/sim/world/world_commands.rs:1526` |
| R2 | UNRES (moderate) | `Enter_Idle_Mode` harvester arm on `ChangeOwner`/`Unlimbo` callers | Trigger: Yuri mind control / engineer-capture of a miner, or a fresh miner leaving the factory. Effect: native re-queues Harvest for the new owner; VERA relies on other paths. Frequency: every mind-controlled miner in Yuri games. Not recorded | `mission_handlers.rs:677-760` (Move caller only); 0x00738970 |
| R3 | RES (structural) | Two admission authorities for refinery contacts (`dock_reservations` + radio bus) while the depot decides on the bus from the dispatch slot | Every refinery/depot session; determinism unaffected, but duplicate authority persists | `miner_dock.rs:9-13`, `miner_dock_sequence.rs:959/1895`, `building_dock.rs:399-425` |
| R4 | UNRES (DRIFT, low, unrecorded) | Same-tick money ordering: repair/depot spend after factory step (native before) | Only at near-zero balance; native starves the factory, VERA the repair. Every tick with both active | `src/sim/world/mod.rs:8105-8120` (09.01 §2.23) |
| R5 | UNRES (low) | Chrono miner HELLO while driving (NavCom gate) | CMIN return path; 07.37 M6 | `miner_dock_sequence.rs:917-990` |
| R6 | UNRES (low, unrecorded) | Cancel refund uses paid amount, native `GetCost(house)` at cancel time | Only when a FactoryPlant is built/lost mid-build; rare | `factory.rs:264-283` (09.01 §2.8) |
| R7 | stale claim (doc) | System Map says sell refund is RefundPercent-driven and VERIFIED; code uses `SELL_REFUND_PERCENT=50 × health%` | Every reader; the sell DRIFT itself is GSI-09.14's | `docs/system-map/mechanisms.v1.json:249,341`; `production_sell.rs:26,55` |
| R8 | RES | Credit-counter display-dispatch cadence vs committed sim frames | Visible only when render rate ≠ logic rate / paused | `sidebar_projection.rs:120` |
| R9 | UNRES (low) | `TotalSpent (+0x2DC)` consumer; `+0x54E8` kill/capture score terms; `+0x34B2` identity | statistics/score screen only | 09.01 §2.14/§2.24 |
| R10 | UNRES (low) | Map-trigger credit events | stock skirmish maps essentially never | `sim/trigger_runtime.rs` |
| R11 | RES / recorded minor | SuspendEVA on nuke flash; unload `+0xF8` incrementer ±1 frame; sender-full HELLO eviction BREAK (dormant); legacy ore-growth dead hashed state; depot dock phase/timer unhashed; sale-path interrupt resets dock phase; Attack retask keeps the miner's old cursor; AI-house miner re-scan bridge; airfield/war-factory contact authorities (pre-existing); theme in savegames; drained building power cutoff `+0x5778` + DrainAnim; derrick EMP/warp/`+0x67C`/`PoweredSpecial` gates; ProduceCash-after-drain order (no stock type carries both); leftover-budget "no candidate aborts" (commit only) | each names its trigger in code except where marked | see rows above |
| — | EXCL (evidence) | storage/silo family (09.02), `FUN_00684C30`, removed-building ore, Return mission, VoxelAnim/Weeder growth | — | first audit |

Remaining count: **1 REGR (new), 0 OMIT, 6 UNRES (R2, R4, R5, R6, R9, R10), 3 RES/structural-or-doc (R3, R7, R8), plus the R11 group of recorded minor residuals.**

---

## Global parity harness coverage (unchanged from the first audit)

Fixture has no derrick, DrainWeapon, capture, depot, slave miner, purifier or AI house, so #256's
mechanisms are exercised only as composition (the v135 folds of dead timers/`None` links); the
pre-v135 probes reproduce the prior finals, as the commit states. #244's grant is not exercised
(no bootstrap grant path). Read the moved finals as "fixture unaffected", not as phase coverage.
