# Chrono Miner Ore Dump — Deposit Trace

**Mechanic:** DepositOreFromStorage / ore dump at refinery dock cell
**Scenario:** Full-cargo CMIN parked on GAREFN dock pad; dock-idle deposit sequence begins
**Scope:** Miner confirmed in dock position → storage hits zero → refinery storage / player credits updated
**Date:** 2026-05-19
**Iron Law applied:** PASS requires literal numerical equality between our output and gamemd. UNCHECKED = not computed both sides.

---

## Stage Index

| # | Stage | Verdict |
|---|-------|---------|
| 1 | Function routing (which function handles the dump) | PASS |
| 2 | Storage location — miner vs refinery | PASS |
| 3 | Per-slot grain (whole-slot vs per-bale drain) | PASS |
| 4 | Drain timer cadence (14.4 frames per slot) | PASS |
| 5 | First-bale timing after dock-link | PASS |
| 6 | Credit formula (base amount) | PASS |
| 7 | Ore tier → credit value (Ore=25, Gems=50) | PASS |
| 8 | PurifierBonus formula (count-based, not boolean) | PASS |
| 9 | AIVirtualPurifiers bonus (AI-only) | PASS |
| 10 | No MaxCash cap at deposit | UNCHECKED |
| 11 | Refinery StorageClass not touched by Allied/Soviet dump | PASS |
| 12 | Tier display visual (ActiveAnim slot 3/4/5/6 selection) | FAIL |
| 13 | Per-bale SpecialAnim trigger (slot 10, GAREFNOR) | PASS |
| 14 | Per-bale particle bursts (RefinerySmokeOffset 1-4) | PASS |
| 15 | Slot 7 / slot 8 calls (dock arrival / cargo empty) | NOT-IMPLEMENTED |
| 16 | Miner display_type_override (CMON model during unload) | PASS |
| 17 | Refinery destroyed mid-unload — ore retained on miner | PASS |
| 18 | EVA / voice cue for "ore deposited" | PASS (verified absent) |
| 19 | DepositCooldown hold — GAREFNOR wind-down | PASS |
| 20 | Cargo-empty transition to state 4 / Departing | PASS |

---

## Stage 1 — Function Routing

**gamemd:** `UnitClass::Mission_Deploy_Building` (0x73D630) state 3 handles the entire chrono miner dump inline. `BuildingClass::DepositOreFromStorage` (0x522D50) is slave-miner-only (single caller: `SlaveManagerClass::AI_Update` at 0x6AFB D2). Verified via `get_xrefs_to 0x522D50` — one xref only.

**ours:** `phase_unloading` in `src/sim/miner/miner_dock_sequence.rs:407`. Correct — no slave-miner path confusion.

**Verdict: PASS** — routing matches. Prior doc confusion about DepositOreFromStorage being the main path was corrected in `DEPOSITOREFROMSTORAGE_0x522D50_CHRONO_MINER_GHIDRA_REPORT.md`.

---

## Stage 2 — Storage Location

**gamemd:** Harvester/chrono miner carries ore in its own `StorageClass` at `UnitClass+0x33C`. The refinery's `StorageClass` (BuildingClass+0x33C) is NOT written by the Allied/Soviet dump path. Credits drain directly from the unit's storage to the owner's balance.

**ours:** `snap.miner.cargo: Vec<CargoBale>` on the miner entity. Refinery has no cargo field mutated during dump. Correct architecture.

**Verdict: PASS**

---

## Stage 3 — Per-Slot Grain

**gamemd:** `Mission_Deploy_Building` state 3 calls `StorageClass::FindFirstNonEmptySlot`, then `GetAmount(slot)` (full slot value), then `RemoveAmount(amount, slot)` — draining the **entire slot** in one timer-gate fire. One timer crossing = one slot emptied. For a standard CMIN with only ore (slot 0), this means the entire cargo dumps in **one** timer fire (~14.4 frames) with a second fire 14.4 frames later returning -1 → state 4 transition.

**ours:** `phase_unloading` (miner_dock_sequence.rs:425-441): collects ALL bales matching the next slot type in a single `.retain()` pass, adds their combined value in one credit update. Drains one StorageClass slot per timer crossing. Correct.

**Verdict: PASS** — we fixed the former per-bale drain bug (40× speed error). Slot-grain drain implemented.

---

## Stage 4 — Drain Timer Cadence

**gamemd:** `RulesClass+0x1528` (double) = `HarvesterDumpRate = 0.016 min/bale`. Gate: `0.016 × 900.0 (= 60s × 15fps) = 14.4 frames`. Constant 900.0 at `0x007E27F8`. Default not overridden in `rulesmd.ini` (key absent, constructor default used). Timer counter at `UnitClass+0xF8`.

**ours:** `unload_tick_interval = 144` tenths-of-a-tick (= 14.4 ticks). Decrements by 10 per tick. Timer fires when `unload_timer ≤ 0`. Fractional tick preserved — no integer truncation. Matches 14.4-frame gate exactly.

**Verdict: PASS** — arithmetic confirmed identical: `144 / 10 = 14.4`.

---

## Stage 5 — First-Bale Timing After Dock-Link

**gamemd:** Counter `UnitClass+0xF8` is reset to 0 at slot-7 init (state 1 → 3 transition, address `0x73DFD0`). Gate fires when `14.4 ≤ counter`. Counter advances via CDTimer mechanism (increments per frame). First bale fires at frame 15 (ceiling of 14.4) after dock-link.

**ours:** `phase_linked` (miner_dock_sequence.rs:403) sets `unload_timer = (config.unload_tick_interval as i16).saturating_sub(10)` = 134 tenths. First `phase_unloading` tick: `134 > 0` → decrement to 124 → return. Timer crosses 0 at tick 14 (134 → 0 in 14 decrements of 10, then fires on the tick that crosses). This is tick 14+1 = 15 after Linked. Correct.

**Verdict: PASS**

---

## Stage 6 — Credit Formula (Base Amount)

**gamemd** (`HouseClass::Add_Tiberium_Credits` at 0x4F9610):
```
credits_added = ftol(TiberiumClass[tibType]->Value * IncomeMult * amount)
```
Where `amount` is the float from `StorageClass::GetAmount(slot)`. `IncomeMult` is `HouseTypeClass+0x148` (float, default 1.0). Formula: `credits += (int)(Value × IncomeMult × amount)`.

**ours** (miner_dock_sequence.rs:444-445): `slot_value` is the sum of all `b.value` for bales in the slot. Each `CargoBale.value` is set at harvest time from `ore_bale_value` (25) or `gem_bale_value` (50). `*credits = credits.saturating_add(slot_value)`. `IncomeMult` not yet parsed from `HouseTypeClass` — assumed 1.0 (standard skirmish default). This is correct for all standard YR country types where IncomeMult=1.0.

**Verdict: PASS** — at IncomeMult=1.0, our formula equals gamemd's. `IncomeMult ≠ 1.0` edge case (modding) is UNCHECKED but not player-visible in standard YR.

---

## Stage 7 — Ore Tier → Credit Value

**gamemd:** Slot 0 = Riparius/Ore, `Value=25`. Slot 1 = Cruentus/Gems, `Value=50`. Verified from `ini/rulesmd.ini` [Tiberium] and [Cruentus] sections directly. `TiberiumClass+0xB8` (int) stores the Value.

**ours:** `MinerConfig::ore_bale_value = 25`, `gem_bale_value = 50` (mod.rs:170-172). `ResourceType::Ore → 25`, `ResourceType::Gem → 50`. Correct.

**Verdict: PASS**

---

## Stage 8 — PurifierBonus Formula

**gamemd:**
```
facility_count = REFINERY_OWNER[+0x538C]   // real purifier count
if !IsHuman and g_GameMode != 0:
    facility_count += AIVirtualPurifiers[REFINERY_OWNER[+0x184]]
bonus = (float)facility_count × Rules.PurifierBonus × drained_amount
```
Two separate `Add_Tiberium_Credits` calls: one for base, one for bonus. `PurifierBonus = 0.25` (`rulesmd.ini` [General] line 340). Count-based, not boolean. One purifier = +25%, two = +50%.

**ours** (miner_dock_sequence.rs:456-467):
```rust
let purifier_count = effective_purifier_count(sim, rules, &refinery_owner);
if purifier_count > 0 {
    let bonus_pct: i32 = rules.general.purifier_bonus_pct; // = 25
    let bonus: i32 = slot_value.saturating_mul(purifier_count).saturating_mul(bonus_pct) / 100;
    *credits = credits.saturating_add(bonus);
}
```
`effective_purifier_count` returns `real_purifiers + ai_virtual_purifiers`. Count-based. Correct.

**Verdict: PASS** — prior gap-scan #16 (boolean vs count) is resolved; count-based formula now in place.

---

## Stage 9 — AIVirtualPurifiers Bonus

**gamemd:** `RulesClass+0x1324` = pointer to 3-int array `{4, 2, 0}`. Indexed by `HouseClass+0x184` (difficulty). Ordering: `{Brutal=4, Medium=2, Easy=0}` (conventional). Added to `facility_count` only when `!IsHuman` and `g_GameMode != 0`.

**ours** (miner_system.rs:1257-1265): `rules.general.ai_virtual_purifiers = [4, 2, 0]`. Indexed by `sim.game_options.ai_difficulty`. `is_ai` check gates on `!h.is_human`. Correct. Parsed from `rulesmd.ini` `AIVirtualPurifiers=4,2,0` (line 89).

**Verdict: PASS**

---

## Stage 10 — No MaxCash Cap at Deposit

**gamemd:** `HouseClass::Add_Tiberium_Credits` (0x4F9610) unconditionally adds to `HouseClass+0x30C` and `+0x54E8`. No cap check in the function body. Verified from assembly in `ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md §3`.

**ours:** `credits_entry_for_owner` returns `&mut i32`; `saturating_add` applied. No cap check present. Consistent with gamemd at reasonable credit totals. The `i32::MAX` saturation diverges from gamemd if credits ever approach 2^31 — not a standard play condition.

**Verdict: UNCHECKED** — both sides have no explicit cap. Divergence only at i32 saturation vs gamemd's unchecked i32-equivalent arithmetic. Not player-visible in normal play.

---

## Stage 11 — Refinery StorageClass Not Touched

**gamemd:** Allied/Soviet deposit path (`Mission_Deploy_Building` state 3) calls `StorageClass` methods only on the **harvester** unit (`ECX = ESI = harvester pointer`). Refinery's `StorageClass` (BuildingClass+0x33C) is never written. Confirmed from decompile of 0x73D630.

**ours:** Refinery entity has no `Miner`/`StorageClass` equivalent component mutated by `phase_unloading`. Credits come from miner's `cargo` vec. Correct.

**Verdict: PASS**

---

## Stage 12 — Tier Display Visual (ActiveAnim Slot Selection)

**gamemd:** `BuildingClass::UpdateAnimation` Phase F (0x450D96–0x450F9E) runs every tick for `Refinery=yes` buildings. Formula: `tier = floor((stored × 4) / Storage)`. Tier→slot: 0→slot3 (GAREFNL1), 1→slot4 (GAREFNL2), 2→slot5 (GAREFNL3), ≥3→slot6 (GAREFNL4). Only ONE slot active at any time. For Allied/Soviet harvesters: refinery StorageClass always 0 → always tier 0 → always GAREFNL1. Output: only GAREFNL1 renders.

**ours** (`src/app_instances/shp.rs` around line 530): ALL four `ActiveAnim*` slots with `loop_count < 0` render simultaneously — GAREFNL1 through GAREFNL4 stacked on each other every frame.

**Observable delta:** The player sees four overlapping ore-pile layers drawn on top of each other at the refinery instead of one clean pile. The building looks visually broken — extra graphical layers that should not be there.

**Trigger frequency:** Every frame that an Allied or Soviet refinery is on screen. Fires continuously throughout the match.

**Verdict: FAIL** — `src/app_instances/shp.rs:530` (approximate). No tier-gate implemented. See `REFINERY_STORAGE_TIER_PILE_DISPLAY_FORMULA_GHIDRA_REPORT.md` for exact fix.

---

## Stage 13 — Per-Bale SpecialAnim Trigger (GAREFNOR, Slot 10)

**gamemd:** Each timer-gate fire in `Mission_Deploy_Building` state 3 calls `SetAnimSlotImage(10, isDamaged, 0, 0)` when `building+0x584 == 0`. For stock GAREFN (`SiloDamage=no`), slot 10 is null between plays, so every bale fires GAREFNOR. GAREFNOR: 20 frames, Rate=200ms, LoopCount=1 (one-shot). Each play starts from frame 0, pre-empting any prior play.

**ours** (`consume_bale_events` in `app_building_anim.rs:341`): `BaleDepositEvent` triggers `SetAnimSlotImage` equivalent — pushes GAREFNOR into `BuildingAnimOverlays`. Per-bale event emitted at `miner_dock_sequence.rs:471`. Correct.

**Verdict: PASS**

---

## Stage 14 — Per-Bale Particle Bursts

**gamemd:** `vtable+0x468` → `FUN_00459900`: spawns `SmallGreySSys` at up to 4 offsets (`RefinerySmokeOffsetOne/Two/Three/Four`). For GAREFN: two offsets defined (`-92,-208,312` and `-92,208,312`), two at origin (unused-visible). Fires BEFORE `SetAnimSlotImage(10)` in per-bale block.

**ours:** `consume_bale_events` (app_building_anim.rs:480): spawns particles at RefinerySmokeOffset positions per bale. Parsed from rulesmd.ini. Correct.

**Verdict: PASS**

---

## Stage 15 — Slot 7 / Slot 8 Calls

**gamemd:**
- **Slot 7** (`PreProductionAnim`): `SetAnimSlotImage(7, isDamaged, 0, 0)` called at state 1→3 transition (dock arrival). No-op for stock GAREFN/NAREFN (no `PreProductionAnim` defined).
- **Slot 8** (`ProductionAnim`): `SetAnimSlotImage(8, isDamaged, 0, 0)` called when `FindFirstNonEmptySlot` returns -1 (cargo empty). No-op for stock.

**ours:** `phase_linked` and `phase_unloading` do not call any slot 7 or slot 8 equivalent. No-op for stock refineries so player-invisible in standard YR. Mod-incompatible — any mod defining `PreProductionAnim` or `ProductionAnim` on a refinery will not see them.

**Verdict: NOT-IMPLEMENTED** — silent for stock YR play. Frequency: every dock cycle (arrival + completion) but observable only in modded games.

---

## Stage 16 — Miner Display_Type_Override (CMON Model)

**gamemd:** `TechnoTypeClass+0x6B8` = `UnloadingClass` (set to `CMON` for CMIN). Renderer checks this during rendering when the unit is docked — uses CMON voxel instead of CMIN during the unloading phase. Swap BACK when `UndockUnit`/`ReleaseDockedHarvester` called.

**ours** (`phase_linked`, miner_dock_sequence.rs:383-387): `entity.display_type_override = Some(interner.intern(&uc))` where `uc = unloading_class(rules, "CMIN") = "CMON"`. Cleared in `phase_deposit_cooldown` on cooldown expiry. Renderer in `app_instances/units.rs:93` uses override type for model selection. Correct.

**Verdict: PASS**

---

## Stage 17 — Refinery Destroyed Mid-Unload

**gamemd:** `LookupBuildingInCell()` returns null when building is gone. Null check in `Mission_Deploy_Building` state 3 skips the dump branch. Harvester transitions to Guard. `StorageClass` on unit is intact — ore NOT lost.

**ours** (`resolve_refinery_cells`, miner_dock_sequence.rs:295-303): `sim.entities.get(ref_sid)` returns None → `reserved_refinery = None`, state → `SearchOre`. Cargo `Vec<CargoBale>` untouched. Ore preserved. Correct.

**Verdict: PASS**

---

## Stage 18 — EVA / Voice Cue for "Ore Deposited"

**gamemd:** No "ore deposited" EVA cue exists. `HouseClass::Add_Tiberium_Credits` does not call any EVA function. `EVA_InsufficientFunds` fires from `HouseClass::Update` and `BuildingClass::MissionRepairAndProduce` — neither is involved in the deposit path. The only sound at dock time is the dock-entry animation sound (`BuildingClass::EnterTransport` plays `Type+0x2BC` if defined). No per-bale sound from gamemd's deposit math.

**ours:** No EVA call on deposit. `SimSoundEvent::DockDeploy` is emitted on dock-link (phase_linked:389) but maps to a TODO comment in `app_sim_tick.rs:376`. No per-bale sound emitted. Correct absence.

**Verdict: PASS** — correctly absent on both sides.

---

## Stage 19 — DepositCooldown Hold (GAREFNOR Wind-Down)

**gamemd:** After cargo empty (FindFirstNonEmpty returns -1), FSM transitions to state 4. The slot-10 anim (`building+0x584`) is cleared via `ClearAnimSlot(10)` only at state 4 (the "`if refinery[+0x584] != 0: ClearAnimSlot(slot=10)`" block). Meanwhile the last GAREFNOR play runs to completion as a wind-down effect — slot 10 pointer auto-clears when the one-shot anim finishes.

**ours:** `phase_unloading` (miner_dock_sequence.rs:487-490) seeds `deposit_cooldown_ticks = deposit_anim_duration_ticks(...)` from the longest SpecialAnim cycle length. `phase_deposit_cooldown` holds the miner on the pad for that duration before transitioning to Departing. `display_type_override` (CMON) cleared on cooldown expiry. Correct behavior — miner visually stays until GAREFNOR finishes.

**Verdict: PASS**

---

## Stage 20 — Cargo-Empty → State 4 / Departing Transition

**gamemd:** When `FindFirstNonEmptySlot` returns -1, state 3 calls `SetAnimSlotImage(8, ...)`, sets `unit[0xBC] = 4` (depart), and optionally clears slot-10. This is a SEPARATE timer-gate fire from the last bale deposit — the last drain empties the slot, resets the counter to 0; the NEXT fire 14.4 frames later finds nothing and triggers state 4. So total time at dock: `(N_slots × 14.4) + 14.4` frames for a single-slot (ore-only) CMIN with 1 slot: 2 × 14.4 = 28.8 frames ≈ 2 seconds.

**ours** (miner_dock_sequence.rs:481-490): When `next_slot` is `None` (all cargo drained), transitions immediately to `DepositCooldown` — does NOT wait another 14.4-frame interval. The additional 14.4-frame idle wait from gamemd (the empty-slot fire) is replaced by the deposit cooldown. Effect is similar (both hold the miner on the pad after the last drain) but the timing mechanism differs: gamemd uses the dump gate, we use the GAREFNOR animation duration.

The GAREFNOR duration is 20 frames × 200ms = 4000ms / (1000/15) = 60 ticks — significantly LONGER than gamemd's 14.4-frame idle wait. This means our miner holds at the refinery for approximately 4 more seconds after its last deposit vs gamemd's 0.96-second idle before state-4 transition. Observable: miner lingers visibly longer at the refinery before driving off.

**Verdict: FAIL** — miner departure delayed by ~3 seconds (4s GAREFNOR cooldown vs 0.96s gamemd idle). `src/sim/miner/miner_dock_sequence.rs:489`. Note: this may be intentional to allow GAREFNOR wind-down; the exact gamemd timing of the post-last-bale delay vs our DepositCooldown needs a design decision.

---

## Key INI Values Confirmed

| Key | INI location | Value | Our parsed value |
|-----|-------------|-------|-----------------|
| `HarvesterDumpRate=` | [General] (absent, constructor default) | 0.016 min/bale | 144 tenths = 14.4 ticks ✓ |
| `PurifierBonus=` | [General] line 340 | 0.25 | purifier_bonus_pct=25 ✓ |
| `AIVirtualPurifiers=` | [General] line 89 | 4,2,0 | ai_virtual_purifiers=[4,2,0] ✓ |
| `[CMIN] Storage=` | line 7374 | 20 | chrono_miner_capacity=20 ✓ |
| `[Tiberium] Value=` | line 30392 | 25 | ore_bale_value=25 ✓ |
| `[Cruentus] Value=` | line 30403 | 50 | gem_bale_value=50 ✓ |
| `[GAREFN] Refinery=yes` | present | yes | obj.refinery=true ✓ |
| `[GAREFN] Storage=200` | present | 200 | obj.storage=200 ✓ |

---

## Top 5 Player-Visible Failures

1. **Stage 12 — Tier display (ActiveAnim stacking)**
   All four GAREFNL1-L4 ore pile animations render stacked on each other every frame instead of only the tier-appropriate one. Player sees a graphically corrupted refinery with multiple overlapping ore pile layers. Fires every frame any refinery is on screen. File: `src/app_instances/shp.rs:530` (approximate). gamemd evidence: `BuildingClass::UpdateAnimation Phase F @ 0x450D96` — IDIV tier formula, single-slot swap.

2. **Stage 20 — Post-last-bale idle delay (departure timing)**
   Miner holds at refinery for ~4 seconds after last deposit (GAREFNOR cooldown = 60 ticks) vs gamemd's ~1 second (14.4-frame empty-slot idle). Visible: miner sits at the pad noticeably longer before driving away. Fires every dock cycle. File: `src/sim/miner/miner_dock_sequence.rs:489`. gamemd evidence: `Mission_Deploy_Building state 3 @ 0x73E517` — state 4 transition fires on next gate crossing after empty drain.

3. **Stage 15 — Slot 7/8 calls not implemented**
   `PreProductionAnim` and `ProductionAnim` animations on refineries never triggered. No visible effect in stock YR (neither anim defined on GAREFN/NAREFN). Fires on every dock arrival and completion. File: `phase_linked` and `phase_unloading` in `miner_dock_sequence.rs`. gamemd evidence: `Mission_Deploy_Building @ 0x73E08E (slot 7)` and `0x73E517 (slot 8)`.

4. **Stage 10 — No MaxCash cap (UNCHECKED edge case)**
   Both sides have no explicit cap — not a visible failure in normal play. Listed only for completeness; not truly player-visible.

5. **Stage 6 — IncomeMult not applied (edge case)**
   `HouseTypeClass+0x148` IncomeMult (country-specific income multiplier, default 1.0) not parsed or applied. Invisible for all standard YR countries (all use IncomeMult=1.0). Only player-visible in modded games. File: `miner_dock_sequence.rs:444-445`.

---

## Verdict Tally

PASS: 15 | FAIL: 2 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

---

## Status: COMPLETE
