# Phase 7 disparity scan — GSI-09.03 Ore/gem value, cargo, unload conversion + GSI-09.05 Miner work-site and return decisions

Read-only scan, 2026-09-05. Worktree `clean-slate-system-impl-891469` @ `4b89ef52` (branch `feature/phase-7-parity-close-284594`).
Binary: gamemd.exe (Ghidra project, image base 0x400000). Every address below was decompiled/disassembled
in this session unless marked "(research, not re-derived)". Rust line numbers are from this worktree.

Legend for dispositions: MATCH (bounded), DRIFT (demonstrated), MISSING, EXCLUDED, UNRESOLVED.

---

## 1. Scope enumeration (native mechanisms, from bodies + active callers)

| # | Mechanism | Native owner | Active caller chain (stock YR) |
|---|---|---|---|
| M1 | TiberiumType data (`Value`, `Power`, `Growth`, `Image`) | `TiberiumClass::ReadINI_Fields` 0x00721A90 (+0xB8 Value, +0xBC Power, +0xA8 Growth, +0xB0 GrowthPercentage dbl, +0xA0 SpreadPercentage dbl, +0xE0 overlay bind by `Image`) | rules load (`ReadINI_All` 0x00721D10) |
| M2 | Per-cell scan score | `CellClass::Get_Tiberium_Value` 0x00485020 = `Tib[idx].Value * (OverlayData + 1)` | `Scan_For_Tiberium` |
| M3 | Harvester cargo tally | `StorageClass` float[4] at Unit+0x33C, one slot per TiberiumClass index; `AddAmount` 0x006C9690, `RemoveAmount` 0x006C96B0, `GetTotalAmount` 0x006C9650, `FindFirstNonEmptySlot` 0x006C9820 | Harvest_Ore_Tick, Mission_Unload |
| M4 | Fullness | `UnitClass::Get_Storage_Percentage` 0x007414A0 (= UnitClass vtable+0x2B4, verified `read_memory 0x7F5F24` → `a0 14 74 00`) = total / `UnitType+0x800` (`Storage=`) | Mission_Harvest state 0 (`>= 1.0`), state 1 (`== 1.0`), Harvest_Ore_Tick (`>= 1.0`) |
| M5 | Per-bite collection | `UnitClass::Harvest_Ore_Tick` 0x0073D450 → `CellClass::Reduce_Tiberium` 0x00480A80 | Mission_Harvest 0x0073E5E0 state 1 only |
| M6 | Harvest cadence | Mission_Harvest state-1 StepTimer gate `[+0xF8] >= 9`; timer re-armed with `HarvesterLoadRate` (RulesClass+0x1520, stock absent → 2) | Mission_Harvest state 1 |
| M7 | Ore search | `FootClass::Search_For_Tiberium_And_Move` 0x004DCFE0 → vtable+0x338 `Scan_For_Tiberium` 0x004DD0A0 → `Is_Cell_Harvestable` 0x004DCE80; radii RulesClass+0x177C `TiberiumLongScan` (48) state 0, +0x1778 `TiberiumShortScan` (6) state 1 | Mission_Harvest states 0/1 |
| M8 | Archive ("last harvested") | `TechnoClass::Set_ArchiveTarget` from Mission_Harvest state 1 full branch (short scan result) / state 1 no-ore-no-NavCom; consumed at state 0 entry (`+0x218`) | Mission_Harvest |
| M9 | Refinery selection | `FootClass::Find_Docking_Bay` 0x004DF040 → vtable+0x52C `FUN_004DEE80` (per-type nearest) → `FUN_0065ADF0` (contact-slot probe) + `BuildingClass::Receive_Radio` 0x0043C2D0 case 0xF + `FUN_005F6500` (2-D lepton² distance) | Mission_Harvest state 2 (narrow pass `arg3=0`, then wide pass `arg3=1` bracketed by `g_MapEditorMode++/--`), state 4 (RepairBay list) |
| M10 | Return decision (close vs far, chrono) | Mission_Harvest state 2: 3-D `Sqrt_Approx`+ftol vs `HarvesterTooFarDistance*256` (RulesClass+0xD78) / `ChronoHarvTooFarDistance*256` (+0xD7C); close → `Transmit(2)` → state 3; else wide pass → drive/warp to building cell + `(Type+0x1618,+0x161C)` via `Find_Nearby_Passable_Cell` when `dist > 0x300` or Teleporter | Mission_Harvest state 2 |
| M11 | No-ore go-home | Mission_Harvest state 0 miss with NavCom==0 && archive==0 → state 4, `Unit+0x3D0=1`, `House+0x242=1` (Harvester=yes), return 0x69 (105 frames); state 4 → `Find_Docking_Bay(RulesClass+0x850 RepairBay, 0, 1)` → Queue_Mission(0x14 Repair) else (0x0F); if standing on a Refinery/Weeder-dock building → Set_Destination(exit cell); Queue_Mission(5 Guard) | Mission_Harvest; then `UnitClass::Mission_Guard_Harvester` (research, not re-derived) |
| M12 | Unload conversion | `UnitClass::Mission_Unload` 0x0073D630 dump state 3: gate `HarvesterDumpRate(RulesClass+0x1528 dbl) * 900.0 <= [+0xF8]`; `FindFirstNonEmptySlot`; `purifiers = House+0x538C (+ AIVirtualPurifiers[House+0x184] when !House+0x1EC && g_GameMode!=0)`; `bonus = purifiers * PurifierBonus(RulesClass+0xF3C float) * slotAmount`; `RemoveAmount` whole slot; `HouseClass::Add_Tiberium_Credits` 0x004F9610 twice (base, bonus) on the refinery owner (`building vtable+0x3C`) | Mission_Unload (mission 16) after dock |
| M13 | Credit arithmetic + stat | `Add_Tiberium_Credits`: `House+0x54E8 = ftol(amount*5.0 + old)`; `House+0x30C = ftol(Tib[idx].Value * HouseType+0x148 IncomeMult * amount + old)`; ftol = truncate (0x0E7F) (research §2-3, consistent with body read) | M12 |
| M14 | 'Harvesting' animation | No sim-side facing/frame writes in Harvest_Ore_Tick/Mission_Harvest; OREGATH drawn render-side in `DrawExtras` 0x0073CEC0 (plate comment; not re-derived) | render |

Rust owners: `src/sim/miner/miner_system.rs` (handlers, scan, refinery select), `src/sim/miner/mod.rs` (Miner/MinerConfig), `src/sim/miner/miner_dock_sequence.rs` (unload), `src/sim/tiberium/mod.rs` (reduce_tiberium, cell view), `src/sim/economy.rs`, `src/rules/tiberium_type.rs`.
Production chain: `Simulation::advance_tick` (`src/sim/world/mod.rs:6748`) → techno AI shell → `techno_ai.rs:1004 dispatch_harvest_for_object` → `harvest_mission.rs:106..203` → `miner_system.rs:584 process_miner_with_resource_authority` → per-cursor handlers; dock/unload via `miner_dock_sequence.rs:734 handle_dock_sequence` → `:822 Unloading` → `phase_unloading` (:1185).

---

## 2. Per-mechanism findings

### M5 — Per-bite collection amount  **DRIFT (critical, every harvest gate)**

**Native (0x0073D450, disassembly 0x0073D541..0x0073D5A1):**
```
FILD  [type+0x800]        ; Storage
FSTP  [esp+0x10]
CALL  GetTotalAmount       ; total
FSUBR [esp+0x10]           ; ST0 = Storage - total
FCOMP [0x007E2AC8]         ; 1.0f  (read_memory → 00 00 80 3f)
FNSTSW AX ; TEST AH,0x41 ; JNZ recompute   ; (Storage-total) <= 1.0 → request = Storage-total
FLD   [0x007E2AC8]         ; else request = 1.0
CALL  Math__ftol           ; truncate
PUSH  EAX ; CALL Reduce_Tiberium
```
Request per bite = `ftol(min(1.0, Storage − total))` = **one density level per gate**. `Reduce_Tiberium(1)` on OverlayData=c (c ≥ 1) is the partial path (`1 < c+1`): byte −1, return 1; on c = 0 it is the full-removal path returning 0 → `removed <= 0` → helper returns FALSE without re-arming → state 1 runs the short continuation scan. So an 11-density cell yields 11 bales over 11 gates, and the 12th gate on the now-zero cell clears it and triggers retargeting. Success re-arms StepTimer with rate `HarvesterLoadRate`, 9 steps → next bite 19 frames later (F+19, mission-before-timer order; research ledger + this body).

**Rust (`miner_system.rs:1022 handle_harvest`, :1055-1068):** `empty = capacity_bales − cargo.len()` and `reduce_tiberium_at_with_native_context(cell, empty, …)` → `tiberium::reduce_tiberium` (:441) with `amount = empty` (40 for an empty HARV). `amount < current+1` is false for any cell with density ≤ 39 → **full-removal path every gate**: the whole cell (up to 11 levels) is taken in one 19-frame gate and the overlay is cleared. The unit test `miner_tests.rs:5414 harvester_drains_full_cell_in_one_extraction_tick` enshrines this ("matching gamemd's Harvest_Ore_Tick") — it is wrong. The research doc `HARVEST_ORE_TICK_…_GHIDRA_REPORT.md` §9 row "Request amount is truncated remaining capacity in tiberium units" is also wrong (contradicted by the `FCOMP 1.0f` / `FLD 1.0f` branch and by the 2026-07-28 plate comment on 0x0073D450, which already says "1 density level per bite").

**Player-visible effect:** Rust fills a War Miner from a max-density field in ~4 gates (≈76 frames) vs native 40 gates (≈760 frames ≈ 51 s): roughly **10× native harvest throughput**, ore fields vanish cell-by-cell instantly instead of thinning frame by frame, and gem/ore field lifetime is cut by an order of magnitude. Trigger: every harvest gate of every miner all game.

**Downstream:** the unload/cadence timings (M12) and the harvest_timer +1 are correct, so the fix is confined to the request amount; the `is_full` transition after a filling bite (Rust re-arms and observes fullness at the next gate) already matches native's "positive fill is still success".

### M1/M2 — TiberiumType data and scan score  **MATCH (bounded)**
`ReadINI_Fields` reads `Value` int → +0xB8, `Power` → +0xBC, `Growth` → +0xA8, `GrowthPercentage`/`SpreadPercentage` doubles, `Image` (1..4 → overlay bind; `Image=2` also sets +0xE4/+0xE8 = 12). Rust `rules/tiberium_type.rs:101` parses Value/Growth/percentages/Image; `Power` is not parsed (only consumed by tiberium explosion damage — out of lane, stock `Power=0` on all four types; EXCLUDED-by-data here). Score `Value*(OverlayData+1)` = `tiberium_cell_view.nominal_value` (`tiberium/mod.rs:83`, `wrapping_mul`). Stock: Riparius 25 / Cruentus 50 / Vinifera 25 / Aboreus 25.

### M3/M4 — Cargo tally and fullness  **MATCH (bounded, stock) / DRIFT (minor, TIB2/TIB3 maps)**
Native: float per TiberiumClass index (4 slots); full when `total / Storage >= 1.0`. Rust: `Vec<CargoBale>` with `ResourceType::{Ore,Gem}` (`mod.rs:214`), `is_full = cargo.len() >= capacity_bales` (`mod.rs:499`), capacity = `Storage=` from `ObjectType.storage` (`world_spawn.rs:571-573`, `:1037`, `:1224`), stock HARV 40 / CMIN 20 (`rulesmd.ini:8236`, `:7374`). Integer-equivalent for stock.
Minor DRIFT: `resource_type_for_tiberium_image` (`tiberium/mod.rs:86`) folds Vinifera/Aboreus (`Image=3/4`, overlays TIB2/TIB3) into `Ore`, so a mixed Riparius+Vinifera load drains as ONE unload slot (one 15-frame gate) where native drains two slots (two gates), and bale value is taken from `TiberiumTypeId(0).Value` (`mod.rs:327 from_rules`) at extraction rather than the cell's own type `Value` at unload. Stock values are equal (25/25/25), so only the extra ~15-frame unload gate is visible, and only on maps that place TIB2/TIB3 overlays. Frequency: map-dependent, low.

### M6 — Harvest cadence  **MATCH (bounded)**
Native: state 0→1 transition at 0x0073E879 arms rate 2 / step 0; state 1 gates on `[+0xF8] >= 9`; success re-arms with `HarvesterLoadRate`. Rust: `harvest_tick_interval = 9*HarvesterLoadRate = 18`, armed `+1` (`mod.rs:265`, `miner_system.rs:1065`, `:989`). Checked only for the stock rate (2) and the arrival→first-bite / bite→bite paths. Note the native transition to state 1 occurs only when `Search_For_Tiberium_And_Move` returns 1, which (0x004DCFE0) happens only when the miner is already standing on Tiberium — so anchoring on arrival is the right shape; the residual is the state-0 dispatch latency after arrival (Rate epilogue, 14-16 frames) which Rust reproduces via `arm_rate_epilogue` in `handle_move_to_ore` (:1017).
Unchecked: the native "destination present → return TRUE, no extraction, no re-arm" branch (0x0073D488) has no explicit Rust gate in `handle_harvest`; VERA's Harvest cursor is only entered after arrival, so it is reachable only via retask races. UNRESOLVED (low).

### M7 — Ore search  **MATCH (bounded) with two filter residuals**
Native `Scan_For_Tiberium` (0x004DD0A0): ring 0 = own cell `LandType == 5` with NO harvestability filter; rings `1..radius-1` (exclusive upper bound), square (Chebyshev) perimeter, arm order top/bottom/left/right per offset `-r..=r`, corners visited twice, strict `>` (first-seen wins), first non-empty ring wins. No RNG, no gems preference beyond `Value*(data+1)`, no threat avoidance. `Is_Cell_Harvestable` (0x004DCE80): in-playfield; (SP-only shroud gate on `Unit+0x41A`); `Can_Reach_Zone` for the unit's movement zone; `LandType == 5`; `Can_Enter_Cell(cell,-1,-1,0,1) == 0` (fully clear).
Rust `search_local_tiberium` (`miner_system.rs:2029`): same ring bounds (`1..radius`), same arm order, same strict compare, same ring-0 shortcut. Radii from `[General]` (`mod.rs:298`), 48/6 stock.
Residuals (VERA-internal, gamemd equivalent UNCHECKED): (a) reachability is `ore_reachable` (any 8-neighbour of the ore cell in the miner's zone, `:1908`) because VERA's path grid marks ore cells `ZONE_INVALID`; native tests the ore cell itself; (b) occupancy filter `is_cell_path_clear_for_scan` (`:720`) ignores infantry blockers, native requires `Can_Enter_Cell == 0` (infantry-occupied cells are not 0). Both differ only in crowded/edge fields; no stock frequency estimate attempted.
Search radius unit: native `(v + (v>>31 & 0xFF)) >> 8` on the rules value — i.e. the RulesClass field is stored in leptons and converted to cells at the call (research; the stock 48/6 read straight from ini therefore means the parser multiplies by 256 — not re-derived, no Rust impact since Rust uses cells directly).

### M8 — Archive target  **MATCH (bounded), one ordering nit**
Native state 0: if `+0x218` archive set → `Set_Destination(archive,1)`, clear archive, then the scan call returns 0 immediately (NavCom busy) → Rate epilogue; it does NOT check the archive cell still has ore. Rust `handle_search_ore` (:775-802) drops a depleted/unreachable archive and scans instead. Visible only when the archived patch was exhausted by another miner during the trip (native drives there anyway then rescans; Rust rescans first). Low.

### M9 — Refinery selection  **DRIFT (demonstrated, three independent deltas; every full-cargo return)**
Native `FUN_004DEE80` (read this session) iterates **only `this->Owner`'s building list** (`param_1[0x87]` = TechnoClass+0x21C Owner; vector at House+0x6C/+0x78), per `Dock=` type in list order, filters: non-null, `+0x81 == 0`, `Type == dockType`, narrow pass requires `FUN_0065ADF0(building, miner)` = a free radio-contact slot (`building+0xE4[i] == 0`) or the miner already tracked; if `WhatAmI() != Aircraft` (vtable+0x2C; 2 = AbstractType::Aircraft — resolves the research's "GetMission()==2 unresolved") → `Can_Reach_Zone` must pass; then `Receive_Radio(0xF)` must return 1; distance `FUN_005F6500` = `(dy)² + (dx)²` in leptons², **Z ignored**, on `GetCoords()` of both objects; replace-if `best==-1 || d < best || candidate.IsPrimaryFactory(+0x3D3)`.
`Receive_Radio` case 0xF (0x0043C2D0, read this session) for a Refinery (`Type+0x16B3`) and a Unit with `Harvester=yes`: rejects mission 0x12 Construction / 0x13 Selling (`return 10`), rejects `building+0x534 == 0`, rejects when `g_MapEditorMode == 0` and no free/own contact slot, rejects MovementZone/Naval mismatch, rejects `!HasPower`; then returns 1 only if `g_MapEditorMode != 0` (wide pass) or `building+0x118 == 0` (no unit currently occupying the bay). So: **narrow pass = own house, free contact slot, bay not occupied; wide pass = own house, any refinery** (contact/bay gates bypassed). Ally check (`Is_Ally_ByObject`) is redundant because only own buildings are iterated.

Rust `find_nearest_refinery` (`miner_system.rs:1730-1778`), callers `handle_return :1189`, `begin_return :1476`, `handle_forced_return`:
1. **Accepts allied refineries** (`are_houses_friendly`, :1743) → credits land in the ally's wallet (`phase_unloading` credits the refinery owner, :1240) whenever the ally's refinery is nearer. Native: never. Trigger: every team game with a nearer allied refinery.
2. **No narrow/wide pass**: no contact-slot / bay-occupied preference → all of a house's miners convoy to the single nearest refinery; native spreads them to the nearest *free* one and falls back to busy ones only when none is free. Trigger: every return with ≥2 miners per refinery cluster.
3. **No zone-reachability filter**: an unreachable (other island/behind cliffs) refinery can be chosen; native `Can_Reach_Zone` rejects it. Map-dependent.
4. Distance: Rust squared 2-D **cell** distance from the miner cell to the computed dock cell (`refinery_dock_cell`), native squared 2-D **lepton** distance between object coords (building centre) — same metric family, tie order and near-equidistant picks can differ. Low.
5. `IsPrimaryFactory` override absent (no primary designation on refineries in VERA). Low.
Extra Rust gates `entity.dying`, `health.current == 0`, `in_limbo`, `building_up` map onto native `+0x81`, Construction/Selling missions; not contradicted.
Status: `docs/plans/2026-07-28-refinery-selection-parity-plan.md` covers 1-2 (own-house filter + `would_admit` two-pass) and was **not implemented** (no `would_admit` in `miner_dock.rs`; :1743 still allied). It does not cover 3.

### M10 — Close/far return decision and chrono special-casing  **MATCH (bounded) / cross-lane residual**
Native state 2 (read this session): Teleporter (`Type+0xCD4`) uses `ChronoHarvTooFarDistance` (+0xD7C), HARV uses `HarvesterTooFarDistance` (+0xD78); distance = `ftol(Sqrt_Approx(dx²+dy²+dz²))` in leptons, 3-D; `<=` threshold → `Transmit(2)` → 1 → state 3. Rust `return_exceeds_too_far_threshold` (:88-115): 3-D squared lepton distance including terrain Z vs `(threshold*256)²`, thresholds 5/50 from `[General]` (`mod.rs:311-312`). Equivalent except at the exact boundary (`Sqrt_Approx` is approximate, ftol truncates; Rust compares squares exactly) — negligible.
Residual (dock lane, not this scan): Rust's HARV close path uses adjacency/CloseEnough contact instead of the radio HELLO at ≤5 cells (documented at `:1517-1522`); the native wide-pass "drive to `Type+0x1618/0x161C` staging when `dist > 0x300` (3 cells) or Teleporter" is modelled only beyond the too-far threshold (`try_issue_standard_far_return_drive`, :1651). Chrono: teleport only when far, staging cell via `chrono_return_staging_cell_for_sid` — shape matches.

### M11 — No-ore go-home  **DRIFT (documented VERA-internal), plus MISSING house flag**
Native: scan miss with no NavCom and no archive → state 4, `Unit+0x3D0 = 1`, **`House+0x242 = 1` (sticky "ore short")**, return 105; the state-4 dispatch queues Guard (after a RepairBay probe), i.e. the harvester parks. Recovery is owned by `Mission_Guard_Harvester` (research `MISSION_GUARD_HARVESTER_GHIDRA_REPORT.md`, not re-derived): AI harvesters re-enter Harvest only while `House+0x242 == 0`; player chrono miners re-enter when an own refinery is in one of the 8 adjacent cells or storage is full; player HARV has no automatic re-entry.
Rust `handle_wait_no_ore` (:1343): re-scans every 105 frames forever (`rescan_cooldown_ticks`), documented as VERA-internal at `:1310-1330`; no `House+0x242` equivalent anywhere (`grep ore_short` none). Player-visible: a Rust miner that exhausted its area resumes on its own when ore regrows/spreads within 48 cells; a gamemd player HARV stays parked until re-ordered. Trigger: once per exhausted patch per miner (late game). AI consumer of +0x242 is EXCLUDED until the AI lane exists.

### M12/M13 — Unload conversion  **MATCH (bounded)**
Native dump state 3 (0x0073D630 body read this session): gate `HarvesterDumpRate*900.0 <= StepTimer` with rate-1 StepTimer → first drain when the counter reaches 15 (0.016×900 = 14.4); `FindFirstNonEmptySlot` (index order 0..3 = ore before gems for stock); purifier count = `House+0x538C` plus `AIVirtualPurifiers[House+0x184]` for non-human in-game houses; `bonus = float(count) * PurifierBonus * slotAmount`; `RemoveAmount(slotAmount)` (whole slot); `Add_Tiberium_Credits(removed, slot)` then `(bonus, slot)` if `bonus > 0`; counter reset; next gate drains the next slot; `-1` slot → state 4. Credits and stats go to the **refinery owner** (`this_00->vtable+0x3C`).
Rust `phase_unloading` (`miner_dock_sequence.rs:1185-1323`): `unload_tick_interval = ceil(rate*900) = 15` (`ruleset.rs:2184`, `mod.rs:288`), one slot per gate ore-then-gem (:1219-1223), whole slot, credits to refinery owner (:1240-1250), `apply_income_mult` single truncation (`economy.rs:82`), `HarvestedCredits += bales*5` (`economy.rs:42`), purifier bonus `trunc(slot_value*count*PB*IM)` (`economy.rs:94`) and stat `trunc(count*PB*bales*5)` (:116), `effective_purifier_count` (`miner_system.rs:2390`) = live non-dying OrePurifier count + AI table for non-human. Worked stock values: 40 ore bales, 1 purifier → 1000 + 250 credits, stat 200 + 50 — identical to native float math (all intermediates exact). Checked inputs: stock Value/IncomeMult/PurifierBonus, integer bales; fractional storage not representable in Rust (cannot arise in stock).
Nit: Rust adds the base `HarvestedCredits` stat only inside `if base_credits > 0` (:1234); native adds `amount*5` whenever `removed > 0`. Differs only for `Value=0` mods.
Native `purifier count` source `House+0x538C` is maintained by construction-complete/limbo (research); Rust recounts live structures each drain — equivalent unless a purifier is in limbo/under construction (Rust counts `building_up`? `count_purifiers_for_owner` :2362 does not exclude `building_up`) → UNRESOLVED minor: a purifier still under construction may pay bonus in Rust; native increments +0x538C only at `OnConstructionComplete` (research `BUILDINGCLASS_SPECIAL_BUILDINGS` §7, not re-derived).

### M14 — Harvesting animation cadence  **UNRESOLVED (render-side)**
Native sim writes no facing/frame during harvesting; OREGATH is drawn by `DrawExtras` (0x0073CEC0, plate comment only). Rust draws OREGATH from `src/app/presentation/instances/units.rs:880-952` with its own frame counter. Not inspected against the binary in this scan (render lane).

### Lifecycle entry points
- Map spawn / factory delivery: `Miner::new(kind, cfg, Storage=)` at `world_spawn.rs:573/1037/1224` — capacity data-driven. `world_commands.rs:3353` and `techno_ai.rs:1885/5412` construct with `MinerConfig::default()` and storage 0 (kind default 40) — test/debug spawn paths only (not the production spawn), acceptable.
- Capture / mind control: credits follow the refinery owner (M12) — matches.
- Refinery destruction/sale mid-return: `handle_return` re-selects (`:1224-1240`); native state 2 with no bay falls into the wide pass and, finding none, waits in state 2 (never scans ore while full) — Rust `handle_search_ore` full-check (:757) preserves that.
- Save/load: `Miner` is serialized; the pending harvest_timer/unload cluster are frame-anchored — not inspected further.

---

## 3. Ranked implementation candidates (grouped by prerequisite)

### Group A — no prerequisite (standalone, high value)

**A1. One density level per harvest bite (M5).** Size: small (S). In `handle_harvest` request `min(1, empty)` instead of `empty`; the `is_full` check already runs before the call. Rewrite the tests that pin whole-cell drains (`miner_tests.rs:5405-5460` `harvester_drains_full_cell_in_one_extraction_tick`, `harvester_caps_extraction_at_remaining_capacity`, plus any `extract_bales_max`-shaped expectations at `:5263-5405`) to the per-bite contract: 11-density cell → 11 gates × 19 frames, 12th gate clears and retargets. Correct the stale sentence in `docs/research/HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md` §9 ("request = ftol(min(1.0, Storage−total))", cite 0x0073D569 `FCOMP 1.0f`). Expect the global harness hash constants to move (harvester docking time shifts by ~700 frames). Files: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_tests.rs`, harness pins.

**A2. Own-house-only refinery selection (M9.1).** Size: S. Replace `are_houses_friendly` with owner equality in `find_nearest_refinery` (:1743). Files: `miner_system.rs`, `miner_tests.rs` (check for tests spawning only cross-house refineries). Plan already exists (Task 2 of the 2026-07-28 plan).

### Group B — prerequisite: A2 (same helper)

**B1. Narrow/wide two-pass selection (M9.2).** Size: M. Add `RefineryDockContacts::would_admit(ref, miner, capacity)` (mirror of `hello_or_wait`, `miner_dock.rs:47-68`) and a bay-occupied probe (Rust equivalent of `building+0x118 != 0` — the pad-link `is_on_pad`/`pad_occupied`, `miner_dock.rs:100-104`); narrow pass filters on both, wide pass ignores them. Files: `miner_dock.rs`, `miner_system.rs` (thread `miner_sid` to the 3 callers), `miner_tests.rs`. Plan Tasks 1/3.

**B2. Zone-reachability filter in selection (M9.3).** Size: S-M. Reuse `zone_grid.can_reach` (as `ore_reachable`, :1908) between the miner's anchor cell and the refinery dock/queue cell. Files: `miner_system.rs`.

**B3. Lepton² building-centre distance + list-order tie semantics (M9.4).** Size: S. Optional exactification after B1; only tie order changes.

### Group C — prerequisite: Guard-mission harvester arm (cross-lane)

**C1. Native go-home tail (M11).** Size: M-L. Requires the Unit Guard override's harvester arm (`techno_ai.rs` Guard dispatch residual) so a parked miner can be recovered the native way (adjacent own refinery / full storage / AI flag); then replace the 105-frame re-scan loop in `handle_wait_no_ore` with Queue_Mission(Guard) and add the sticky `House+0x242` flag on `HouseState` (needed by the future AI lane). Files: `miner_system.rs`, `src/sim/world/techno_ai.rs`, `src/sim/house_state.rs`.

### Group D — low priority / data-gated

**D1. Per-type cargo slots (M3 minor):** carry `TiberiumTypeId` on `CargoBale`, drain per type in index order, value from the cell's type at unload. Size: M. Only visible on TIB2/TIB3 maps or `Value` mods. Files: `mod.rs`, `miner_system.rs`, `miner_dock_sequence.rs`.
**D2.** Exclude `building_up` purifiers from `count_purifiers_for_owner` (:2362) after confirming `+0x538C` maintenance in `OnConstructionComplete`/`Limbo`. Size: XS.
**D3.** Destination-present harvest gate (M6 residual). Size: XS; only if a retask race is demonstrated.

---

## 4. Uninspected / unknown

- `BuildingClass+0x534` (case 0xF reject when zero) and `+0x118` (bay occupant) semantics were read from the body only; their maintenance sites were not walked (assumed "placed/ready" and "current docker" — treat as UNCHECKED labels).
- `Mission_Guard_Harvester` (0x00740922 region) and `House+0x242` consumers: cited from research, not re-derived.
- `HouseClass+0x538C` maintenance (`OnConstructionComplete`/`Limbo`): research only.
- Native `Can_Enter_Cell` return classes vs Rust `is_cell_path_clear_for_scan` (infantry) — not compared in detail.
- `TiberiumShortScan`/`LongScan` lepton storage in `RulesClass::ReadGeneral` — not re-derived (Rust reads cells directly; no impact).
- OREGATH frame cadence (`DrawExtras` 0x0073CEC0) and cargo pip drawing — render lane, not inspected.
- Save/load rehydration of `harvest_timer` / unload cluster — not inspected.
- The state-2 HARV close-path radio and the 3-cell (`0x300`) wide-pass staging drive belong to the dock lane; only their selection inputs were checked here.
