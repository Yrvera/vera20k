# GSI-09.01 — Credits, transactions and displayed money (disparity scan)

Read-only scan, 2026-09-05. Binary: gamemd.exe (Ghidra, image base 0x400000). Rust: worktree
`clean-slate-system-impl-891469` @ 4b89ef52. Retail data: `ini/rulesmd.ini` in the worktree.
Every native claim below was re-read from the binary this session unless marked "doc-only".

Evidence labels: **[bin]** = decompile/disassembly read this session; **[doc]** = research report
consulted, spot-checked where stated; **[rust]** = current Rust read this session.

---

## 1. Scope enumeration (native mechanisms, from bodies + active callers)

### 1.1 HouseClass money storage [bin]

| Field | Offset | Writer / reader evidence |
|---|---|---|
| Ore storage `StorageClass` (float[4], one slot per Tiberium type) | `+0x2FC..+0x30B` | `Spend_Money` 0x004F9798 `LEA ESI,[EBX+0x2fc]`; `Available_Money` 0x004F69A3 `LEA ECX,[ESI+0x2d8]` (ESI = house+0x24). **The research docs' "+0x314" is wrong.** |
| Cash credits | `+0x30C` | `Add_Credits`, `Spend_Money`, `Add_Tiberium_Credits`, `Set_Credits_And_Color` |
| Silo capacity (Σ BuildingType `Storage=`) | `+0x310` | `Removed_From_Game` 0x005025F0 case 6 subtracts `Type+0x800`; space-left slot 0x004F69D0 |
| TotalSpent statistic | `+0x2DC` | `Spend_Money` tail 0x004F992D |
| InitialCredits | `+0x1DC` | `Set_Credits_And_Color` 0x004FCE00, `FUN_004C6210` (campaign) |
| Score accumulator (income×5 + kill/capture value) | `+0x54E8` | `Add_Tiberium_Credits` 0x004F961D; also `TechnoClass__RecordKill`, `ChangeOwner` (per DB comment) |
| OrePurifier count | `+0x538C` | read by both deposit paths |
| Difficulty index (Hard=0, Medium=1, Easy=2) | `+0x184` | indexes `AIVirtualPurifiers` (rules+0x1324) and `MultiplayerAICM` (rules+0x1308) |

**Money interface vtable** at 0x007EA834 (pointer stored at `HouseClass+0x24`) [bin, read_memory]:
slot +0x18 = **0x004F6990 `Available_Money`** = `ftol( StorageTotalAsInt(+0x2FC) × HouseType.IncomeMult(+0x148) + credits(+0x30C) )`;
slot +0x1C = 0x004F69D0 = `capacity(+0x310) − ftol(storage total)`. (The DB label
`HouseClass__ComputeAIOpeningCredits` on 0x004F6990 is a misnomer: it is the generic
Available_Money used by FactoryClass::AI, CreditsClass::AI, repair, drain, spy and Post_Map_Init.)

### 1.2 Primitive transactions [bin]

- **`Add_Credits` 0x004F9950**: `credits += amount` (no clamp, no stats).
- **`Add_Tiberium_Credits(float amount, int slot)` 0x004F9610**:
  `score(+0x54E8) = ftol(amount × 5.0f [0x007EAA00] + score)`;
  `credits = ftol( TiberiumType[slot].Value(+0xB8) × HouseType.IncomeMult(+0x148) × amount + credits )`.
  One ftol per call; `amount` is a float32 argument.
- **`Spend_Money(int amount)` 0x004F9790**:
  1. `old = StorageTotal(house +0x2FC)`, `cap = +0x310`.
  2. If `credits >= amount`: `credits -= amount`, `spent = amount`.
  3. Else `deficit = amount − credits; credits = 0; spent = old credits`; if `deficit > 0` and house
     storage total > 0: for each owned building (+0x6C array, +0x78 count) whose **building**
     storage (`bld+0x33C`) is non-empty: repeatedly `RemoveAmount(1.0, slot)` from the building
     storage, mirror-remove the same from the house storage, value it as
     `ftol(Value[slot] × IncomeMult × removed)`, `deficit −= value; spent += value`; on
     over-drain `credits −= deficit` (refund the excess), stop.
  4. `Update_Silo_Damage_Frames(ftol(old), cap)` 0x004F9970 — dead in stock YR (marks only
     `SiloDamage=yes` types; only GASILO in artmd, no rules section) [DB comment + bin].
  5. `TotalSpent(+0x2DC) += spent`.
- **`Add_Tiberium_To_Storage` 0x004F9700**: fills the house storage one bale at a time while
  total < rules+0x17D0; **only caller** is `UnitClass__Mission_Unload` 0x0073E48E behind
  `Type+0xE0F` (Weeder flag, `JZ` at 0x0073E47C skips it). No stock YR unit is a weeder that
  can reach a refinery in skirmish → house storage stays 0.0 in stock skirmish.
- **`Set_Credits_And_Color` 0x004FCE00**: `+0x1DC = +0x30C = credits`.

### 1.3 Every active caller of the primitives (xrefs, this session) [bin]

| Site | Function | Transaction |
|---|---|---|
| 0x004C9BF5 / 0x004C9C2F | `FactoryClass__AI` 0x004C9B20 | per-step drain / completion remainder |
| 0x004CA046 | `FactoryClass__AbandonProduction` 0x004C9FF0 | `Add_Credits(Type.GetCost(house) − Balance)` |
| 0x0073E4A9 / 0x0073E4C9 | `UnitClass__Mission_Unload` 0x0073D630 (harvester state 3) | `Add_Tiberium_Credits(slotAmount)` then `(bonus)` |
| 0x00522E11 / 0x00522E31 | `BuildingClass__DepositOreFromStorage` 0x00522D50 ← `SlaveManagerClass__AI_Update` 0x006AFBD2 | same pair, from the Slave Miner building storage |
| 0x0044AAEF, 0x0044A272 | `BuildingClass__Sell` 0x00449C30 state 2 | `Add_Credits(refund)`, then stored ore → `Add_Tiberium_Credits(ftol(amt), slot)` |
| 0x0044A176 / 0x0044A1B0 / 0x0044A222 | `BuildingClass__Sell` MCV-undeploy branch | full refund when the UnitClass alloc/unlimbo fails |
| 0x004575E4, 0x00449332 | upgrade-refund loops (`+0x702` upgrade level, `+0x5E8` upgrade types) | `Add_Credits(UpgradeType.GetCost(house,1))` |
| 0x004D9FC6 | `FootClass__OnSold` 0x004D9F70 | `Add_Credits(vtable+0x2BC refund)` |
| 0x00446E8F / 0x00446EDD | `BuildingClass__OnConstructionComplete` 0x00445F80 | FreeUnit unlimbo-fail refund |
| 0x0043FDC1 / 0x0043FDD1 | `BuildingClass__Update` 0x0043FB20 (ProduceCash timer) | `Add_Credits(ProduceCashAmount)` or `Spend_Money(−amount)` |
| 0x004482D0 | `BuildingClass__ChangeOwner` 0x00448260 | `Add_Credits(ProduceCashStartup +0x1558)` when old owner's HouseType `+0x1A6` (MultiplayPassive) is set and value ≠ 0; also arms the ProduceCash timer (+0x6D0..+0x6D8 ← `Type+0x1560` delay) |
| 0x004508A3 | `BuildingClass__UpdateRepairAndPower` 0x00450630 | building repair `Spend_Money(RepairCost)` |
| 0x006F4D48 | `TechnoClass__Receive_Radio` 0x006F4AB0 (radio 0x1C) | service-depot repair `Spend_Money(cost)` |
| 0x006FA1AE / 0x006FA1C0 | `TechnoClass__AI_Update` 0x006F9E50 | **Drain**: drained object's owner `Spend_Money(min(DrainMoneyAmount, available))`, drainer's owner `Add_Credits(same)` every `DrainMoneyFrameDelay` frames |
| 0x00457453 / 0x0045745B | `BuildingClass__OnSpyInfiltrate` 0x004571E0 | spy steals `ftol(Available(victim) × SpyMoneyStealPercent)` |
| 0x0073A0B7/0FB/125/157, 0x0051987B | `UnitClass__PerCellProcess` 0x00739EC0, `InfantryClass__PerCellProcess` 0x00519630 | **Grinder**: `Add_Credits(vtable+0x2BC)` for the unit, each passenger, and +0x694 link |
| 0x004824D4 | crate pickup body (undefined fn around 0x00481A00) | money crate `Add_Credits` |
| 0x005D6F9C | `MultiplayerGameMode__Generate_Starting_Units` 0x005D6D80 | leftover starting-unit budget → `Add_Credits` |
| 0x00686A73 | `ScenarioClass__Post_Map_Init` 0x00686890 | AI opening grant `Add_Credits(ftol(MultiplayerAICM[diff] × 0.01 × Available))` |
| 0x004C6273 | `FUN_004C6210` | campaign difficulty money (`+0x1DC` also bumped) — campaign only |
| 0x00684F58 | `FUN_00684C30` (post-map setup) | gated by `Scenario+0x34B2` and storage space > 0 — see §2.14 |

`FUN_006B0AE0` (the "chrono death credit" lead) calls none of these; it is kill-**attribution**
for passengers of a chrono-anim unit (doc FUN_006B0AE0_CHRONO_DEATH_CREDIT confirms). Not a money
mechanism — out of this lane.

### 1.4 Refund value chain [bin]

`TechnoClass vtable+0x2BC` = **0x0070ADA0** = `Type->vtable+0xB8(Owner, 0)`. `TechnoTypeClass
vtable+0xB8` = **0x00711F60** (shared by 5 type vtables incl. BuildingType at 0x007E4628):

```
pct = (float) Rules.RefundPercent (double @ rules+0x1738); if (flag) pct = 1.0
if (house) {
  costBonus = HouseClass__GetCostBonus(house, type)        // HouseType +0x114/+0x118/+0x11C/+0x120/+0x124 per category
  accum     = HouseClass__GetAccumulatedBonus(house, type) // house +0x5390.. (FactoryPlant products)
  if (Type.Soylent(+0x614) != 0) return ftol(Soylent × costBonus)        // NO RefundPercent
  v = ftol(GetCost() × accum × costBonus)
  if (HouseClass__IsControlledByHuman(house)) v = ftol(v × pct)           // AI houses: 100 %
  return v
} else return ftol(GetCost() × pct)
```
No health term anywhere (a Type method; the object is not passed).

### 1.5 Displayed money (CreditsClass) [bin]

- `CreditsClass__AI(force)` 0x004A2600, called from `CommandBar_Dispatch` 0x006D0680 ←
  `DisplayClass__Dispatch` 0x006922E0 (virtual; slot data at 0x007EDFDC / 0x007E198C).
  `target = Available_Money(PlayerPtr)`, clamped `<1 → 0`; if `target == displayed` return
  (unless force); force → snap; else `step = |diff| >> 3`, clamp `[1, 0x8F=143]`, sign toward
  target, `displayed += step`; if changed: `animating(+0xA)=1`, `counting_up(+0x9) = step>0`.
  `+0xC` (1/3 "direction") is decrement-then-overwrite scratch, never read elsewhere.
  Also marks dirty when a scenario timer (`Scenario+0x11E8/+0x11F0`) is running.
- `CreditsClass__Draw` 0x004A2370: if `animating && Rules.CreditTicks count(+0x6DC) >= 2` →
  `0x00750920(sound = CreditTicks[counting_up ? 0 : 1] (rules+0x6D0), pan 0x2000, vol 0.5f, 0)`
  at 0x004A2500..0x004A2533, then draws CREDITS.SHP + `"%ld"` of `displayed` at
  `(surface_width/2, 2)`, flags 0x4108; clears dirty and animating → at most one tick sound per
  AI step. Observer branch shows elapsed time instead.

### 1.6 Frame ordering [doc FACTORY_HOUSE_AI_ORDER…, LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER…; spot-checked callers]

`LogicClass::PerTickUpdate` 0x0055AFB0: main object vector (harvester `Mission_Unload`
deposits, `BuildingClass::Update` → repair spend + ProduceCash, `TechnoClass::AI_Update` drain,
depot `Receive_Radio` spend) → **global FactoryClass array** (`FactoryClass::AI` per-step spend,
vtable data 0x007E892C) → global HouseClass array (`HouseClass::Update`). Sidebar credit AI is
in the display dispatch, outside the logic tick.

---

## 2. Per-mechanism findings

### 2.1 Wallet model: cash vs stored ore, `Available_Money`
- Native: two pools (§1.1); `Available_Money = credits + trunc(storage_int × IncomeMult)`.
- Rust: `HouseState.credits: i32` (house_state.rs:276) is the only wallet; `Economy.credits` is a
  per-sweep shim (house_state.rs:373-378, world_hash.rs:887-890); `Economy::available()` =
  credits (economy.rs:68). No house or building ore storage exists.
- Disposition: **EXCLUDED for stock skirmish** (house storage only fed by the Weeder path,
  §1.2; building storage only on YAREFN and drained in the same object-loop tick by
  `SlaveManagerClass__AI_Update`, so `FactoryClass::AI` never sees it). **MATCH bounded** to
  storage == 0. Trigger frequency of the excluded silo-drain path in stock skirmish: never.

### 2.2 Starting credits
- Native: `Set_Credits_And_Color` sets `+0x1DC = +0x30C`. Rust: `HouseState::new(.., credits, ..)`
  (house_state.rs:488-499); `credits_for_owner` falls back to `STARTING_CREDITS` for unknown
  owners (production_queue.rs:44-49; test-only fallback). **MATCH** for the lobby value.

### 2.3 AI opening grant (`Post_Map_Init`) — **DRIFT (demonstrated)**
- Native [bin 0x00686A2E..0x00686A73]: for each house with `+0x1EC == 0` (not human) and
  HouseType `+0x1A6 == 0` (not MultiplayPassive):
  `Add_Credits( ftol( MultiplayerAICM[house+0x184] × 0.01 (double @0x007E3808) × Available_Money ) )`.
  `rules+0x1308` is the `MultiplayerAICM` list data pointer (ReadGeneral 0x00670512
  `LEA EBX,[ESI+0x1304]` right after `PUSH "MultiplayerAICM"` 0x00670526; the next key
  `AIVirtualPurifiers` sits at +0x1320/+0x1324). Retail `rulesmd.ini:88`
  `MultiplayerAICM=400,0,0` (hard, medium, easy).
  → Brutal AI: +400 % (5× lobby credits). Medium/Easy AI: **+0**.
- Rust: `apply_skirmish_ai_opening_credits` (scenario_bootstrap.rs:1580-1593) adds
  `house.credits` once to every non-human, non-passive house → **2× for every difficulty**;
  test scenario_post_map.rs:774-775 pins 5 000 → 10 000.
- Effect: with 10 000 lobby credits, native Brutal opens at 50 000 / Medium 10 000 / Easy 10 000;
  Rust opens every AI at 20 000. Every skirmish with any AI. Also the `country_…` IncomeMult term
  is irrelevant (storage 0).
- Rust doc comment at scenario_bootstrap.rs:1582-1587 states the vslot "returns the current
  balance" — the multiply at 0x00686A5E..0x00686A6B was missed.

### 2.4 Leftover starting-unit budget — **MISSING**
- Native [bin 0x005D6F91..0x005D6F9C]: after the per-house `+0xC8`/`+0xCC` starting-force
  callbacks, `if (remaining_budget > 0) Add_Credits(house, remaining_budget)`; budget =
  `((n/2 + Σcost)/n) × UnitCount` (0x005D6E55..0x005D6E7A).
- Rust: `spawn_starting_units…` (scenario_bootstrap.rs:1830-1935) tracks `spent` vs `budget`
  and never credits the remainder. Rust default `unit_count = 10` (game_options.rs:89; retail
  `[MultiplayerDialogSettings] UnitCount=10`).
- Effect: every default-lobby skirmish starts each house short by `budget − spent`
  (0 .. one unit's cost). Fix touches scenario_bootstrap.rs only.

### 2.5 Harvester deposit (`Mission_Unload` state 3) — **MATCH bounded** (stock values)
- Native [bin 0x0073E3F3..0x0073E4CE]: purifiers = `house+0x538C` (+ `AIVirtualPurifiers[diff]`
  when non-human and `g_GameMode != 0`); `amount = GetAmount(slot)` of the **unit's** storage
  (`ESI+0x33C`); `bonus_f32 = (float)purifiers × PurifierBonus(rules+0xF3C) × amount`
  (FSTP to a float32 local); `removed = RemoveAmount(amount, slot)`; if `removed > 0`:
  `Add_Tiberium_Credits(removed, slot)`; if `bonus > 0`: `Add_Tiberium_Credits(bonus, slot)`.
- Rust: miner_dock_sequence.rs:1216-1300 drains one whole slot per threshold crossing (ore
  first, then gems), `base = apply_income_mult(Σ bale values)`, then
  `purifier_bonus_credits(slot_value, effective_purifier_count, bonus_ppm, income_ppm)`; stats
  via `add_harvested(bales)` / `add_harvested_raw(trunc(bales×count×bonus×5))` (economy.rs:40-52, 94-122).
- Checked inputs: Value 25/50, PurifierBonus .25, IncomeMult 1.0, 0..4 purifiers, ≤40 bales:
  all products are exact in float32, so the single-ftol results coincide. Not checked: modded
  fractional `PurifierBonus`/`IncomeMult` (Rust folds ppm in i128 with one truncation; native
  rounds the bonus to float32 first — can differ by 1 credit).
- Owner: both credit the refinery owner (Rust reads the building's owner; doc-verified vtable+0x3C).

### 2.6 Slave Miner deposit — **DRIFT (bonus truncation + timing)**
- Native [bin 0x00522D50]: on slave arrival (`SlaveManagerClass__AI_Update` 0x006AFBB2..0x006AFBD2
  cell match), `DepositOreFromStorage` drains the **building** storage per slot:
  `bonus = purifiers × PurifierBonus × GetAmount(slot)` on the whole slot, then the same two
  `Add_Tiberium_Credits` calls. (How the slave's cargo enters the building storage before this
  call was not traced — UNRESOLVED detail; the arithmetic is per whole slot.)
- Rust: slave_miner.rs:341-390 pops **one bale per tick** and applies the purifier bonus per
  bale.
- Concrete: Medium AI Yuri (`AIVirtualPurifiers[1] = 2` → 0.5) with a 4-bale slave
  (`[SLAV] Storage=4`): native `trunc(0.5×4×25) = 50` bonus, Rust `4 × trunc(12.5) = 48`;
  credits also arrive over 4 ticks instead of one. Brutal (4 → 1.0) and Easy (0) are exact.
  Frequency: every slave return of a Medium-AI Yuri house; human Yuri only with a captured
  Ore Purifier.

### 2.7 Factory per-step drain — **MATCH bounded**
- Native [bin `FactoryClass__AI`]: on cadence expiry `Production_Value += step`;
  `charge = (54 − value == 0) ? Balance : Balance / (54 − value)`, `min(Balance)`;
  `if Available < charge → OnHold, value −= 1` else `Spend_Money(charge); Balance −= charge`;
  at 54: suspend, `Spend_Money(Balance)` (0), `Balance = 0`.
- Rust: factory.rs:196-251 `advance_one_step` mirrors this exactly (steps_left divide, strict
  `<`, rewind on stall, completion charge 0); `step_all` factory.rs:1055-1102 loads/stores the
  wallet shim and accumulates `spent_credits` = native `+0x2DC`.
- Bounded to `Production_Step == 1` and to Balance = house-adjusted cost (cost-bonus lane not
  re-checked here).

### 2.8 Cancel refund (`AbandonProduction`) — **MATCH bounded**
- Native: `Add_Credits(Type.GetCost(house) − Balance)` — recomputes the adjusted cost **at cancel
  time**. Rust: `original_balance − balance` (factory.rs:264-283; production_queue.rs:963-1017).
- Differs only if the house cost multiplier changed between start and cancel (FactoryPlant
  built/lost mid-build): native refunds the new cost minus balance; Rust refunds what was paid.
  Rare; noted, not ranked.

### 2.9 Building sell refund — **DRIFT (demonstrated, three axes)**
- Native (§1.4): `refund = ftol(cost × accum × costBonus)`, then `× RefundPercent` **only for
  human-controlled houses**; no health scaling; stored ore added on top; upgrades refunded at
  100 % first (dead in stock: no `PowersUpBuilding=` in rulesmd).
- Rust: production_sell.rs:24-57 `sell_refund_for_building = cost × 50 (const) × health% / 10000`,
  applied in `sell_building` (:748, :772-774).
- Concrete: GAPOWR (800) at 50 % HP — native human 400, Rust 200. AI-owned sale (the
  low-credit sale in `tick_ai_low_credit_sell_decisions`, production_sell.rs:816-860, and
  native `UpdateRepairAndPower`'s AI arm): native 800, Rust 400 (or less when damaged).
  Also `RefundPercent` is hard-coded 50 (`SELL_REFUND_PERCENT`), not parsed (`rules::ruleset`
  only has `[IQ] SellBack`).
- Frequency: every player sell of a damaged building; every AI emergency sale.

### 2.10 Building repair spend — **DRIFT (demonstrated)**
- Native [bin 0x00450821..0x004508D0]: only when `frame % ftol(RepairRate(rules+0x16E0) × 900.0
  [0x007E27F8]) == 0` (retail `.016` → every **14** frames): `cost = BuildingType vtable+0xB0 =
  0x007120D0 = max(1, ftol( (Cost / (Strength / RepairStep(rules+0x16CC))) × RepairPercent(rules+0x16D0) ))`,
  `step = vtable+0xB4 = 0x00712120 = RepairStep`; if `Available < cost` → repair flag off, else
  `Spend_Money(cost); Health += step` (clamped to Strength).
- Rust: production_sell.rs:876-935 runs **every tick**, `heal = 4 HP` (`REPAIR_HP_PER_TICK`),
  `cost = ceil(cost × 25 % / max_hp) × heal` (`REPAIR_COST_PERCENT = 25`).
- Concrete GAPOWR (800 / 750 HP): native 1 credit per 8 HP every 14 frames (≈ 94 credits for a
  full repair, 1 313 frames); Rust 4 credits per 4 HP every frame (≈ 750 credits, 188 frames).
  Retail keys `RepairPercent=15%`, `RepairStep=8`, `RepairRate=.016` (rulesmd.ini:27-29) are not
  used. Frequency: every repair.

### 2.11 Service-depot repair (radio 0x1C) — **MATCH bounded** (money side)
- Native: `Available >= cost` else radio 0x20; `Spend_Money(cost)` [doc SERVICE_REPAIR_RADIO…,
  xref 0x006F4D48]. Rust: building_dock.rs:94-110 gate, :378-384 `credits − cost` (clamped ≥ 0).
  Cost/step formula belongs to the docking lane; not re-derived here.

### 2.12 FreeUnit unlimbo-fail refund — **MATCH bounded**
- Native 0x00446E8F/0x00446EDD `Add_Credits(FreeUnitType.GetCost(house,1))` [bin context; doc
  GACNST_UNDEPLOY…]. Rust `refund_failed_free_unit` production_refinery.rs:265-268. Amount
  derivation (flag=1 → full cost, no RefundPercent) not re-checked against Rust's `refund` input.

### 2.13 Oil-derrick money — **MISSING**
- Native: capture from a MultiplayPassive owner → `Add_Credits(ProduceCashStartup=1000)` and arm
  the timer; then every `ProduceCashDelay=100` frames `Add_Credits(ProduceCashAmount=20)` while
  operational (`BuildingClass__Update` 0x0043FD28..0x0043FDD6; negative amounts spend).
- Rust: no producer (`grep ProduceCash|derrick` → only RMG placement of `CAOILD`,
  scenario_bootstrap.rs:3419ff). Rust has `Command::CaptureBuilding` (command.rs:583-586), so
  captures happen and yield nothing. Frequency: every map with derricks (most stock maps);
  1 000 on capture + 12 credits/s per derrick is a visible economy swing.

### 2.14 Post-map storage-overflow loop (`FUN_00684C30`) — **EXCLUDED for skirmish / UNRESOLVED identity**
- Native [bin 0x00684EFD..0x00684F69]: if `Scenario+0x34B2` and `Available > Tiberium[0].Value`:
  `while (space_left() > 0) { Add_Tiberium_Credits(1.0, 0); money −= Value }`.
- `space_left = capacity(+0x310) − storage`; at post-map in skirmish no refinery exists yet →
  capacity 0 → loop never runs. The INI key behind `+0x34B2` was not identified.

### 2.15 Cancelled/removed building with stored ore — **EXCLUDED (stock)**
- `Removed_From_Game` 0x005025F0 case 6 and `Sell` 0x0044A272 convert building storage to
  credits; only YAREFN carries storage transiently. Rust: none. Trigger: selling/losing a Slave
  Miner on the exact tick a slave has just deposited — negligible.

### 2.16 Grinder — **MISSING**
- Native (§1.3): unit/infantry entering a `Grinding=yes` building (YAGRND, rulesmd.ini:13510)
  credits `ftol(Soylent × costBonus)` for the unit, each passenger, and the +0x694 link, then
  destroys them. Rust: no grinder consumer at all (audio/events.rs:575-580 confirms). Frequency:
  Yuri players/AI only; Yuri AI grinds captured/surplus units routinely.

### 2.17 Floating-Disc money drain — **MISSING**
- Native [bin 0x006FA14B..0x006FA1C5]: for a Techno with `+0x1D0` (drainer) set and
  `Type+0x5ED` (`Drainable=yes`), every `DrainMoneyFrameDelay` (rules+0x314 = 30) frames:
  `amt = min(DrainMoneyAmount (rules+0x318 = 30), Available(owner))`; `Spend_Money(owner, amt)`;
  `Add_Credits(drainer.owner, amt)`. Applies to every `Drainable=yes` type (refineries and
  power plants alike, rulesmd.ini:11684,11767,12134,12180,12409,12478,12558,12766).
- Rust: combat_weapon.rs:573-582 arms the DrainWeapon; no money transfer. Frequency: any Yuri
  disc parked on an enemy building — 1 credit/frame stolen.

### 2.18 Spy money steal — **MISSING**
- Native [bin 0x0045741C..0x0045745B]: `amt = ftol(Available(victim) × SpyMoneyStealPercent
  (rules+0xD68 = .5))`; `Spend_Money(victim, amt)`; `Add_Credits(spy owner, amt)`.
- Rust: no spy infiltration path (`grep infiltrat` in src/sim → none). Allied spy vs refinery
  is a common human tactic; AI spies are rare.

### 2.19 Crate money — **MISSING**
- Native: money crate → `Add_Credits` at 0x004824D4 (multiplayer amount per doc
  CRATE_SYSTEM §"Type 0": base + random 0..900 — **doc-only, not re-read**). Rust: `sim/crates`
  places/regenerates crates; nothing picks one up (audio/events.rs:579-580). Frequency: every
  crate touched when crates are enabled.

### 2.20 Unit sold at a depot (`FootClass__OnSold`) — **UNRESOLVED reachability, Rust MISSING**
- Native body verified (EVA + sound for humans, `Add_Credits(vtable+0x2BC)`, then removal);
  the caller of its vtable slot (data at 0x007E2444/0x007E8E34/0x007EB1F8/0x007F5E10) was not
  traced, so whether stock YR lets a vehicle be sold on GADEPT/NADEPT is not established here.
  Rust has no unit-sell command (command.rs has only `SellBuilding`/`SellWallAtCell`).

### 2.21 Map-trigger money conditions — **MISSING (low)**
- Native: trigger events "Credits ≥ N"/"≤ N" read `Available_Money` [doc FACTORY_CREDIT_SYSTEM;
  not re-read]. Rust `map/retail_trig.rs`, `sim/trigger_runtime.rs` contain no credits event.
  Stock skirmish maps essentially never use them.

### 2.22 Displayed credits counter — **MATCH bounded (value); MISSING (sound); UNRESOLVED (cadence binding)**
- Rust app/sidebar_projection.rs:38-63 reproduces `|diff|/8` clamped [1,143] with the same
  sign rule and no overshoot; `format_credits` = `to_string()` (render/sidebar_text.rs:353) =
  `"%ld"`. Negative clamp: Rust wallets never go negative, so the `<1 → 0` clamp is moot.
- Rust steps once per **committed Ordinary sim frame** (sidebar_render.rs:36-53,
  `credits_advance_for_frame`). Native steps once per display dispatch (§1.5), whose relation
  to logic frames (render-rate vs logic-rate, paused/menu behaviour) was not established —
  UNRESOLVED; at 1 render : 1 logic frame they coincide.
- **CreditUp/CreditDown tick sound: MISSING** — `CreditTicks` is not parsed and no producer
  exists (audio/events.rs:589). Native plays `CreditTicks[0]` while counting up, `[1]` while
  counting down, once per AI step that changed the value, volume 0.5, centre pan. This is the
  most player-audible gap in the lane (audible on every income/spend event).

### 2.23 Same-tick transaction ordering — **DRIFT (ordering)**
- Native (§1.6): object loop (deposits, repair spend, depot spend, drain, derrick) → factory
  steps → houses.
- Rust `advance_tick` (world/mod.rs): object AI (deposits) → `step_all` factory charge (:7807)
  → `tick_repairs` (:7821) → `tick_building_docks` (:7822).
- Effect: when the balance cannot cover both the factory step and a repair/depot step in the
  same tick, native starves the factory (repair already took the money), Rust starves the repair.
  Only observable under a near-zero balance; deterministic in both, but not the same.

### 2.24 Statistics — **MATCH (score income term); UNRESOLVED (TotalSpent consumer)**
- `+0x54E8` income×5 term ↔ `Economy.harvested_credits` (score.rs:76-88 consumer);
  `+0x2DC` ↔ `Economy.spent_credits` (hashed, world_hash.rs:890) — native consumer of `+0x2DC`
  not located this session. `+0x1DC` has no Rust field (campaign-only reader).
- Save/load: Rust serialises `credits` and `economy` (house_state.rs:234-378). Native save is
  a raw HouseClass dump; no behavioural difference to report.

### 2.25 IncomeMult — **MATCH bounded (stock 1.0)**
- Native applies `HouseType+0x148` inside every `Add_Tiberium_Credits` and in the storage term
  of `Available_Money`/`Spend_Money`. Rust applies `income_ppm_for_owner` in both deposit paths
  (house_state.rs:619-629). All stock countries 1.0 (rulesmd.ini:3195, :3323 commented).

---

## 3. Ranked implementation candidates (grouped by shared prerequisite)

**Group A — no prerequisite, pure economy arithmetic (small, high frequency)**
1. **AI opening grant = `trunc(MultiplayerAICM[diff] × 0.01 × credits)`** (§2.3). Size S.
   Files: `src/sim/scenario_bootstrap.rs` (apply_skirmish_ai_opening_credits), `src/rules/ruleset.rs`
   (parse `MultiplayerAICM`), tests in `src/sim/scenario_post_map.rs:774-775` (rebaseline).
2. **Leftover starting-unit budget → credits** (§2.4). Size S. `src/sim/scenario_bootstrap.rs`
   starting-unit loop; hash/snapshot goldens that pin opening credits.
3. **Sell refund = house-adjusted cost × (human ? RefundPercent : 1), no health term** (§2.9).
   Size S–M. `src/sim/production/production_sell.rs` (sell_refund_for_building, sell_building,
   AI low-credit sale), `src/rules/ruleset.rs` (parse `RefundPercent`), reuse the cost-bonus
   producer already used by the factory (`production_types.rs`/factory cost inputs).
4. **Building repair cadence/cost** (§2.10): 14-frame gate, `max(1, trunc(cost/(strength/step) × RepairPercent))`,
   heal `RepairStep`. Size M (touches damage-state refresh timing). `production_sell.rs:806-935`,
   `ruleset.rs` (RepairRate/RepairStep/RepairPercent). Coordinate with the repair/docking lane.
5. **Repair/depot spend before factory steps** (§2.23). Size S but phase-ordering sensitive
   (`world/mod.rs` advance_tick; hash goldens). Best landed with 4.

**Group B — needs the mechanism host, then the money line is trivial**
6. **Oil-derrick ProduceCash** (§2.13). Size M: building per-tick timer (+0x6D0) and
   ProduceCashStartup on capture; `src/sim/world/techno_ai/*` building arm,
   `src/sim/capture_manager.rs`/capture command, `ruleset.rs` (three keys), `house_state.rs`
   credit entry.
7. **Floating-Disc drain money** (§2.17). Size S once drain state exists (combat_weapon.rs
   already tracks `drain_target_active`): 30-frame modulo, `min(30, available)`, transfer.
8. **Credit tick sound** (§2.22). Size S: parse `CreditTicks`, add `counting_up/animating`
   to `SidebarProjectionState`, emit `GameSoundEvent` per changed step from
   `advance_sidebar_credits_after_frame` (app layer; sim untouched).
9. **Slave Miner whole-slot deposit** (§2.6). Size S: buffer the slave cargo and credit per
   slot on arrival (`slave_miner.rs:341-390`); Medium-AI Yuri parity.

**Group C — blocked on absent mechanisms (money line is the last step)**
10. Grinder (§2.16) — needs Enter-mission handling for `Grinding=yes`.
11. Spy refinery infiltration (§2.18) — needs the spy infiltration branch table.
12. Crate pickup money (§2.19) — needs crate pickup; amount must be re-read at 0x004824D4.
13. Trigger credit events (§2.21), unit sell at depot (§2.20) — resolve reachability first.

---

## 4. Uninspected / unknown

- `CommandBar_Dispatch`/`DisplayClass::Dispatch` call cadence relative to the logic tick (and
  behaviour while paused) — needed to certify the counter's timing; only the arithmetic is matched.
- The consumer of `TotalSpent (+0x2DC)` (score screen?) and whether `+0x54E8`'s kill/capture
  terms match Rust `MatchStatistics::score`.
- `Scenario+0x34B2` INI identity (§2.14) and `FUN_00684C30`'s intent.
- Slave → building storage transfer step before `DepositOreFromStorage` (timing within the tick).
- `FootClass__OnSold` slot callers (depot vehicle sell reachability).
- Crate money amount (doc-only).
- `FactoryClass::AI` with `Production_Step != 1` and cost-multiplier changes mid-build (§2.7/2.8 bounds).
- Modded fractional `PurifierBonus`/`IncomeMult` float32 rounding vs Rust ppm (§2.5 bound).
- Research docs with wrong offsets found in passing: FACTORY_CREDIT_SYSTEM ("StorageClass at
  +0x314", "Notify_Credit_State_Change"), BUILDING_CHANGE_OWNER ("refund power credits" for
  +0x1558 = ProduceCashStartup), CREDITS_COUNTER_SYSTEM (sound "every frame" — it is once per
  changed AI step), scenario_bootstrap.rs comment on 0x004F6990 (missed the AICM multiply).
