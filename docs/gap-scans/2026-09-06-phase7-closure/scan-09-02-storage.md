# GSI-09.02 — Storage, silos and resource loss (disparity scan, read-only)

Date: 2026-09-05. Worktree: `.claude/worktrees/clean-slate-system-impl-891469`
(branch `feature/phase-7-parity-close-284594`). Binary: gamemd.exe (Ghidra, base 0x400000).
Retail data: worktree `ini/rulesmd.ini`, `ini/artmd.ini`, `ini/evamd.ini`, `ini/aimd.ini`.

## 0. Headline (decisive evidence first)

**Stock YR never holds ore in any building or house storage.** Every credit-bearing unload
path converts the *unit's* cargo straight into house credits:

- `UnitClass::Mission_Unload` 0x0073D630 state 3, drain block 0x0073E451..0x0073E4D0:
  `LEA ECX,[ESI+0x33C]` (the **harvester's** StorageClass) → `StorageClass::RemoveAmount`
  0x006C96B0 → gate `MOV AL,[UnitType+0xE0F]` (`Weeder=`) at 0x0073E474 → **Weeder=no** falls
  to 0x0073E4A2: `HouseClass::Add_Tiberium_Credits` 0x004F9610 (amount, slot) then the
  purifier-bonus call at 0x0073E4C9. The `Weeder=yes` branch (0x0073E48E →
  `HouseClass::Add_Tiberium_To_Storage` 0x004F9700, which fills **house** storage +0x2FC up
  to `Rules+0x17D0`) is dead: no `Weeder=` key exists in `rulesmd.ini` (grep, 0 hits) and
  `Rules+0x17D0` is `[General] WeedCapacity` (string at 0x0083B760, read at 0x00671962), a TS
  chem-missile counter.
- Slave miner: `SlaveManagerClass::AI_Update` 0x006AFBD2 calls
  `BuildingClass__DepositOreFromStorage` 0x00522D50 with **ECX = the slave** (ESI, the
  FootClass whose `+0x5A4` was just tested) and stack arg = `SlaveManager+0x24` (the master
  building). Body 0x00522D55 `LEA EBP,[ECX+0x33C]` drains the **slave's** storage; the
  building is only used for `+0x21C` owner (0x00522D75) and the smoke callback vtable+0x468
  (0x00522E55). The Ghidra label is misleading; the master building's StorageClass is never
  written.
- The only writers of any StorageClass (`StorageClass::AddAmount` 0x006C9690 callers):
  `UnitClass::Harvest_Ore_Tick` 0x0073D450 (unit self), `FUN_00522E70` slave harvest tick
  (`LEA EDI,[ESI+0x33C]` at 0x00522F0A, ESI = slave infantry; cap = its own
  `Type+0x800`), `Add_Tiberium_To_Storage` (house; Weeder-only, dead), and `FUN_0065DD30`
  team-member creation (pre-fills created **units** with `Type+0x800` bales when
  `TeamType+0xA5` is set — `Full=yes`; `aimd.ini` has **0** `Full=yes` lines).
  `StorageClass::AddFrom` 0x006C9740 has one caller, `HouseClass::Added_To_Game` 0x00502A80,
  which adds the *building's* (always-empty) +0x33C into the house mirror +0x2FC.

Consequence: house storage `+0x2FC`, every building `+0x33C`, and everything downstream
(silo-drain spending, sale/destruction ore loss, silo fill art, "silos needed") is
data-dead in stock YR. Only the **capacity counter** `House+0x310` and the **purifier count**
`House+0x538C` are live, and neither has a stock consumer that changes player-visible output.

## 1. Scope enumeration (native mechanisms, addresses)

| # | Mechanism | Native identity | Stock-YR status |
|---|---|---|---|
| M1 | Per-house storage mirror | `House+0x2FC` StorageClass (float[4]); `+0x310` capacity; `+0x30C` Balance; `+0x54E8` score; `+0x2DC` spent | +0x2FC always 0 |
| M2 | Capacity adjust on add/remove/capture | `HouseClass::Added_To_Game` 0x00502A80 (`+0x310 += Type+0x800`, then `AddFrom(bldg+0x33C)`); `HouseClass::Removed_From_Game` 0x005025F0 (`+0x310 -= Type+0x800` at 0x00502807..0x00502815, then `SubtractFrom` at 0x005028C2) | live, no visible consumer |
| M3 | Removal "cash out" path | `Removed_From_Game` param_3≠0 branch 0x00502821..0x005028A5 (drain bldg+0x33C → Balance/score inline) | dead: both callers pass 0 — `TechnoClass::Limbo` 0x006F6BD1 `PUSH EBX` (EBX==0, used as null in `CMP EDI,EBX`), `TechnoClass::ChangeOwner` 0x0070159A `PUSH 0x0` |
| M4 | Spend with silo drain | `HouseClass::Spend_Money` 0x004F9790: cash first; shortfall drains each owned building's +0x33C one bale at a time (0x004F9862 `PUSH 0x3f800000`), mirrors into house +0x2FC (0x004F9878), values it `TiberiumClass+0xB8 × HouseType+0x148 × 1.0` ftol; then `Update_Silo_Damage_Frames`; `+0x2DC += paid` | drain never entered (storage 0); cash path live |
| M5 | Unload when "full" | Weeder branch of Mission_Unload caps at `WeedCapacity`; HARV/CMIN branch has **no capacity check** | HARV/CMIN: unlimited credits |
| M6 | Sale ore refund | `BuildingClass::Sell` 0x00449C30 state 2: after `Add_Credits(refund)`, loop `FindFirstNonEmptySlot/GetAmount/RemoveAmount → ftol → Add_Tiberium_Credits` | dead (bldg storage 0) |
| M7 | Destruction ore spill | `BuildingClass::DestructionEffects` 0x004415F0: while `GetTotalAmount ≥ 1.0`: remove 1 bale, `RandomRanged(0x100,0x300)` offset, `CellClass::PlaceTiberium(type,1)` | dead (bldg storage 0) — and NOT entered, so no RNG draws |
| M8 | Silo fill art | `HouseClass::Update_Silo_Damage_Frames` 0x004F9970 (quantise round(4×ratio) on house +0x2FC/+0x310; marks buildings whose `Type+0x16A8`=`SiloDamage`); `BuildingClass::UpdateAnimation` 0x00450CBD SiloDamage block | dead: `SiloDamage=yes` only under `[GASILO]` in artmd.ini:2008 and GASILO has no `rulesmd.ini` BuildingType section (only anim list entries 182–185) |
| M9 | Refinery ore-pile tier art | `UpdateAnimation` 0x00450D96..0x00450E47: gate `Type+0x16BB` (`Refinery=`), `LEA EDI,[ESI+0x33C]` (building's own storage), tier = `(4×ftol(total)) / Type+0x800` (IDIV), cached in `+0x6F0`, slot 3/4/5/6 = ActiveAnim..Four | live, tier is always 0 → only `ActiveAnim` (GAREFNL1/NAREFNL1) ever plays |
| M10 | Storage pips on refinery | `FUN_0044D700` (building pip count): `PipScale`≠2 → generic; else needs `Type+0x16BC` (`Weeder=`) → `Get_Storage_Fraction` 0x004F9750 (÷ WeedCapacity) | stock GAREFN/NAREFN/YAREFN have `PipScale=Tiberium` but no `Weeder=` → 0 pips |
| M11 | "Silos needed" | No `EVA_SilosNeeded`/`SilosNeeded` string in gamemd (search_strings "Silo" → only `EVA_NuclearSiloDetected`, `SiloDamage`, `NukeSilo`, `FillSilos`); `evamd.ini` has no silo entry. `HouseClass::Update` 0x004F8BFF..0x004F8C3F: `if (capacity − ftol(house storage)) < 30 && capacity > 50` → arms a timer (0x00A8EB60, `Rules+0x16A8 × 900`) | never true (storage 0 ⇒ 600 − 0 ≥ 30) |
| M12 | Purifier count | `House+0x538C`: INC at `OnConstructionComplete` 0x0044637C and 0x004491EB (gated `Type+0x16CC`), DEC at `ChangeOwner` 0x00448AC2 and `Limbo` 0x00445925 (gated `+0x16CC` and bldg `+0x6E4`, clamped ≥0). `+0x16CC` is bound to the `"OrePurifier"` string: xref 0x004604ED → write 0x004604FA in `BuildingTypeClass::ReadINI` | live (deposit lane owns the consumer) |
| M13 | Harvester death cargo | `UnitClass::Death_Explosion` 0x00738680: cargo is not spilled; if `GetTotalAmount>0` and `Rules+0x17E5` (read in `RulesClass::ReadCombatDamage` 0x0066CE91 = `[CombatDamage] TiberiumExplosive`, rulesmd.ini:805 `=no`) adds Σ amount×`TiberiumClass+0xBC` damage | dead (flag = no); cargo simply vanishes |
| M14 | Scenario `FillSilos=` | `[Basic] FillSilos` (string 0x0083E0BC read by `ScenarioClass::Read_INI_Basic` 0x0068A25A) → `FUN_00684C30` one-shot loop calling `Add_Tiberium_Credits(1.0, 0)` per house | map flag; skirmish maps: not inspected (see §4) |
| M15 | Save/load | `House+0x2FC/+0x310/+0x538C` and building `+0x33C` are plain fields inside the object blobs | state always 0/derivable |

Corrections to prior research relied on here:
- `REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md` §1 says slaves "deposit ore into the Slave Miner
  building's StorageClass via FUN_00522E70" and the building drains itself. **Wrong**:
  0x00522F0A and 0x00522D55 both address the *slave's* +0x33C (see §0).
- Same report / plan list `+0x16D8` as the OrePurifier flag and call `+0x538C` a "storage
  facility count". Binary: `+0x16CC` ↔ `"OrePurifier"` (0x004604ED/0x004604FA); `+0x538C` is
  the OrePurifier building count. `src/sim/economy.rs:28` already states this correctly.
- `Update_Silo_Damage_Frames` plate comment (Ghidra) says "quantises Storage(+0x2FC)/
  Capacity(+0x310)" — confirmed by 0x004F9798 `LEA ESI,[EBX+0x2FC]` in Spend_Money and the
  `+0x310` read at 0x004F97A5.

## 2. Per-mechanism findings

### M1/M2 — house storage mirror and capacity counter
- Native: above. Capacity += `Storage=` (200 per refinery; `[SLAV]` 4, `[SMIN]`/`[CMIN]` 20,
  `[HARV]` 40 are unit-side and not summed into the house) on Added_To_Game; −= on
  Removed_From_Game (Limbo and ChangeOwner both route through it, so capture moves capacity).
- Rust: no house storage, no capacity field (`src/sim/house_state.rs` `HouseState`,
  `src/sim/economy.rs` `Economy` — credits/spent/harvested/purifier_count only). `Storage=` is
  parsed into `ObjectType.storage` (`src/rules/object_type.rs:610,1708`) and consumed only as
  unit cargo capacity.
- Disposition: **EXCLUDED (state)** — the mirror is provably 0 for every house in stock YR;
  the capacity counter has no stock consumer that reaches player-visible output (M8 needs
  SiloDamage buildings; M10 needs Weeder buildings; M11 needs nonzero storage; M4 needs
  nonzero building storage). Nothing to port. Trigger frequency of a visible effect: none.

### M3 — removal cash-out
- Native: dead branch (both callers pass 0). Even if reached, building storage is 0.
- Rust: n/a. Disposition: **EXCLUDED (dead code, gate evidence 0x006F6BD0/0x0070159A)**.

### M4 — Spend_Money
- Native cash path: `if Balance ≥ amount: Balance −= amount; else Balance = 0, paid = old
  Balance` (0x004F98B9 / 0x004F97C7), then `+0x2DC += paid`. Storage drain never entered.
- Rust: `Economy::spend` `src/sim/economy.rs:60` = `min(credits, amount)`, `spent_credits +=
  paid`; production debits `house.credits` through the wallet shim
  (`src/sim/production/factory.rs:1044..1100`). Same cash arithmetic; the shortfall semantics
  (whether a factory step is charged partially or waits) belongs to the factory lane.
- Disposition: **MATCH (bounded: cash-only path, storage = 0)**.

### M5 — unload with "full" storage
- Native HARV/CMIN: no cap check anywhere in the state-3 drain (0x0073E451..0x0073E4D0);
  every bale becomes credits. No EVA. Chrono/War Miner never wait for "silo space".
- Rust: `src/sim/miner/miner_dock_sequence.rs:1214..1304` credits the refinery owner per
  slot with no capacity gate. Slave: `src/sim/slave_miner.rs:342..385` same.
- Disposition: **MATCH (bounded: absence of a gate; per-slot value/cadence/order is the
  deposit lane GSI-09.01's scope, not re-verified here)**.

### M6 — sale ore refund
- Native: dead for stock (building storage 0). Refund itself (`Add_Credits(vtable+0x2BC)`)
  is the sell lane's scope.
- Rust: `src/sim/production/production_sell.rs:46` refund only; no storage term.
- Disposition: **EXCLUDED (data: no writer of building +0x33C in stock YR)**.

### M7 — destruction ore spill
- Native: loop at DestructionEffects only when `GetTotalAmount ≥ 1.0`; with 0 storage the
  loop and its `RandomRanged(0x100,0x300)` draws are skipped, so the RNG stream is
  unaffected in stock.
- Rust: building death path (`src/sim/combat/mod.rs` ~2796) has no spill; the note there
  records the separate GSI-08.11 `Explosion=` residual (out of this lane).
- Disposition: **EXCLUDED (data)**. Note for a mod-capable future: the spill would need
  `PlaceTiberium` (already ported in `src/sim/ore_growth.rs` as `place_tiberium`) plus RNG
  order inside DestructionEffects.

### M8 — silo fill-level art (SiloDamage)
- Native: only `[GASILO]` sets `SiloDamage=yes` (artmd.ini:2008) and GASILO is not a
  BuildingType in rulesmd.ini (only `[Animations]` 182–185 `GASILO_A/AD/B/BD`), so the
  `Update_Silo_Damage_Frames` loop marks nothing and the 0x00450CBD block never runs.
- Rust: no `SiloDamage` parse (`grep -i silo_damage src/rules` = 0 hits).
- Disposition: **EXCLUDED (data)**.

### M9 — refinery ore-pile tier art (ActiveAnim..Four)
- Native: tier = `4×ftol(building storage) / Storage` on the **building's** +0x33C, cached in
  +0x6F0; storage is always 0 ⇒ tier 0 ⇒ only slot 3 (`ActiveAnim` = GAREFNL1 / NAREFNL1)
  is ever created; slots 4–6 never. YAREFN has no `Refinery=` (rulesmd.ini `[YAREFN]`) and
  uses `IdleAnim=YAREFN_A`, so the block is skipped for it.
- Rust: `src/app/presentation/instances/shp.rs:839..851` suppresses non-primary Active
  slots for `obj.refinery` buildings and renders the primary loop; the comment cites the same
  reason. `src/sim/anim_class.rs:300` `building_storage_fill_level` (round-half-up quantiser)
  is **test-only** (callers: only the tests at :1877) — it mirrors the
  `Update_Silo_Damage_Frames` `+0.5` quantiser (0x007E1738), not the IDIV tier of M9; it
  is dead surface, harmless.
- Disposition: **MATCH (bounded: tier-0 case, the only reachable one; damaged-variant
  selection and the low-power gating of slot 3 are other lanes)**.

### M10 — storage pips on a selected refinery
- Native: 0 pips for stock refineries (no `Weeder=`).
- Rust: `src/app/presentation/ui_overlays.rs:785..818` draws Tiberium pips only for
  vehicles with a miner component.
- Disposition: **MATCH (bounded: buildings draw none in both)**.

### M11 — "silos needed"
- Native: no such EVA in gamemd/evamd; the HouseClass::Update near-full check can never fire.
- Rust: `src/audio/events.rs` has no silo event (grep 0 hits).
- Disposition: **EXCLUDED (data: string absent; gate unreachable)**.

### M12 — purifier count (+0x538C)
- Native: increment/decrement at construction complete / list-add / change-owner / limbo,
  clamped ≥ 0, gated by `Type+0x16CC` (`OrePurifier=`), `[GAOREP]` is the only stock
  `OrePurifier=yes` (rulesmd.ini:11978).
- Rust: `src/sim/miner/miner_system.rs:2362` recounts live, non-dying Structure entities
  with `ore_purifier` per deposit; `effective_purifier_count` :2390 adds
  `AIVirtualPurifiers[difficulty]` for non-human houses (native adds it only when
  `g_GameMode != 0` — 0x00522D8F / same shape in Mission_Unload; skirmish is non-zero).
- Disposition: **UNRESOLVED → deposit lane (GSI-09.01)**: the recount equals the native
  counter whenever the `+0x6E4` "counted" flag, limbo state and Dying timing agree; the
  0x004491EB site is in a function Ghidra labels `BuildingClass__OnSold` but which appends
  the building to the owner's list (0x004491D2) — label suspect; a capture-during-
  construction or sold-then-recount edge could differ by one purifier for one deposit.
  Not chased here (out of lane).

### M13 — harvester death cargo
- Native: cargo is discarded with the unit; extra explosion damage only if
  `TiberiumExplosive=yes` (stock: no). No ground spill.
- Rust: miner cargo dies with the entity; no spill. Disposition: **MATCH (bounded)**.

### M15 — save/load
- Nothing to carry: every storage field is 0 in stock and the capacity/purifier counters are
  derivable from the building set. Rust `HouseState`/`Economy` serialize credits, spent,
  harvested, purifier_count. Disposition: **MATCH by exclusion**.

## 3. Ranked implementation candidates

None are required for stock-YR parity. Listed for completeness, grouped by prerequisite.

A. **No prerequisite / documentation only (size XS)**
   1. Correct `docs/research/miner/REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md` §1/§2
      (slave path drains the slave's storage; `+0x16CC` = OrePurifier, `+0x538C` = purifier
      count) and the Ghidra plate/label on 0x00522D50 (it is a TechnoClass-storage drain
      taking the building as an argument). Files: that doc; `docs/plans/2026-05-12-refinery-
      storage-flow-investigation-plan.md` (mark M6/M7/M8 questions resolved as data-dead).
   2. Optionally delete the dead `building_storage_fill_level` helper + its tests
      (`src/sim/anim_class.rs:300`, `:1877`) or re-label it as the SiloDamage quantiser.

B. **Prerequisite: per-building StorageClass model (only if mod support beyond stock YR is
   ever in scope) (size M–L, blocked on a scope decision; do not build for stock)**
   - Building `+0x33C` storage + house `+0x2FC` mirror + `+0x310` capacity; then M4 drain
     order (per building in house list order, per bale, ftol per bale), M6 sale refund loop,
     M7 spill (RNG draws inside DestructionEffects), M9 tiers 1–3 (IDIV, cache +0x6F0), M8
     SiloDamage marks, M11 near-full timer. Files a builder would touch:
     `src/sim/house_state.rs`, `src/sim/economy.rs`, `src/sim/production/factory.rs` (spend),
     `src/sim/production/production_sell.rs`, `src/sim/combat/mod.rs` (building death),
     `src/app/presentation/instances/shp.rs` (tier slots), `src/rules/art_data.rs`
     (SiloDamage). Every writer that would fill building storage is data-dead in stock, so
     this is a mod feature, not parity work.

C. **Adjacent, other lane (GSI-09.01 deposit)**: purifier-count edge parity (M12).

## 4. Uninspected / unknown

- `FillSilos=` (M14): whether any stock skirmish map sets it was not checked (maps live in
  mix files; `asset` tooling not used here). If set, `FUN_00684C30` converts some
  house-money quantity into `Add_Tiberium_Credits(1.0, 0)` calls at scenario load —
  a one-shot credit/score effect. Rust has no equivalent (`grep -ri fillsilos src` not run).
- `Rules+0x16A8` (double, the near-full timer length in M11) INI identity not resolved
  (not in ReadGeneral); irrelevant while M11 is unreachable.
- `FUN_004F6E70` (house storage ÷ `+0x310` fraction) and `FUN_004589C0` (building
  4×storage/Storage) have no code xrefs in Ghidra — probably vtable slots; not traced
  because their inputs are 0 in stock.
- `HouseType+0x148` factor in Add_Tiberium_Credits (IncomeMult) and the per-slot ftol
  behaviour are the deposit lane's; not re-verified here.
- TS `Weeder`/`WeedCapacity` mechanics deliberately not traced beyond the gate evidence.
