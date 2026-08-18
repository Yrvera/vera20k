---
title: Factory/House Economy Substrate — P7 (ore-deposit income wiring) — DONE
date: 2026-06-06
status: implemented + committed (c220e2d0); full suite green. Produced by the p7-economy-income-wiring
        workflow (6 Ghidra/INI/live-tree understand lanes + synthesis + adversarial review). Review found
        2 blockers + 1 high — all folded into the shipped code (see below).
branch: factory-house-substrate-p1p2
---

# P7 — ore-deposit income wiring

## Scope (what actually changed)
On **stock YR this is credit-hash-NEUTRAL**: every country's `IncomeMult` is 1.0 (commented out in
rulesmd), so the multiplier is the identity, and the base deposit + OrePurifier bonus already matched
gamemd. The live deltas:
1. **Per-country `IncomeMult`** — parsed + applied at deposit time (real for mods, identity on stock).
2. **`HarvestedCredits` statistic** — placed + hashed in P5b but never wired to a live path; now accrues
   on every deposit. (This makes the tick hash diverge from any pre-P7 baseline once a deposit occurs —
   intended; the P5c gates are relative so they still pass.)

NO `SNAPSHOT_VERSION` bump (no layout change). `house.credits` stays the one wallet (P5b).

## The gamemd deposit formula (binary-verified this run)
Per non-empty tib slot, gamemd makes **two `Add_Tiberium_Credits` calls** (base, then bonus), each a
single accumulate-then-`ftol` (truncate-toward-zero), each gated on its credit `> 0.0`:

```
count        = OrePurifierCount(owner)            [House+0x538C, building count] (+ AI virtual if !human && GameMode!=0)
amount       = slot bale count
bonus_amount = count × 0.25 × amount              [PurifierBonus = Rules+0xF3C = 0.25]
# base:  Balance += trunc(TibValue × IncomeMult × amount)       ; Harvested += trunc(amount × 5.0)
# bonus: Balance += trunc(TibValue × IncomeMult × bonus_amount) ; Harvested += trunc(bonus_amount × 5.0)   (if >0)
```
Verified constants: PurifierBonus 0.25 (`decompile_function 0x00522D50`, `Rules+0xF3C`); count `House+0x538C`;
`5.0f` (`read_memory 0x007EAA00`); truncate control word `0x00822D80 = 0x0E7F`; two-call + `>0.0` guards
(`0x00522D50`); ftol `0x004F9610 → 0x007C5F00`. TibValue Ore=25 / Gem=50 from `ini/rulesmd.ini`
`[Riparius]/[Cruentus] Value=` (offset `TiberiumClass+0xB8` confirmed present in the live image).

## Rust mapping (the value-baked subtlety)
Rust `CargoBale.value` already = TibValue, so `slot_value = Σ bale.value = amount × TibValue`. Therefore:
- base credits = `trunc(IncomeMult × slot_value)` = `economy::apply_income_mult(slot_value, income_ppm)`.
- bonus credits = `trunc(IncomeMult × count × 0.25 × slot_value)` = `economy::purifier_bonus_credits(slot_value, count, bonus_pct, income_ppm)` — **ONE i64 truncation** (the BLOCKER fix: a separate `/100` truncation first drifts ±1 when IncomeMult≠1.0).
- HarvestedCredits stat = `amount × 5` (base, `add_harvested(slot_bales)`) + `trunc(count × 0.25 × amount × 5)` (bonus, `economy::purifier_bonus_harvested(bales, count, bonus_pct)` → `add_harvested_raw`). The ×5 multiplies the BALE COUNT, not credits, so `slot_bales` is counted in the drain loop.

`IncomeMult` stored as parts-per-million `i64` (`CountryRules.income_ppm`, default `INCOME_PPM_SCALE=1_000_000`).
Integer/i64 only — lockstep-safe.

## Files
- `src/rules/ruleset.rs` — `CountryRules.income_ppm` + hand-written `Default` (derived would zero it) + `from_ini_section` parse via `get_f32` (not the nonexistent `get_f64`) + `RuleSet::country_income_ppm` (case-insensitive) + `INCOME_PPM_SCALE`.
- `src/sim/economy.rs` — `apply_income_mult`, `purifier_bonus_credits`, `purifier_bonus_harvested` (pure, std-only, single-truncation), `add_harvested_raw`.
- `src/sim/house_state.rs` — `income_ppm_for_owner` (owner → country → income_ppm).
- `src/sim/miner/miner_dock_sequence.rs` — deposit site A (refinery unload): `slot_bales` counter + the helpers.
- `src/sim/slave_miner.rs` — deposit site B (slave, 1 bale/tick) + the helpers.

## Review fixes folded in
- BLOCKER single-truncation bonus (45 not 44 for gem/count3/IncomeMult1.2) — `purifier_bonus_credits` + test.
- BLOCKER `get_f64` doesn't exist → `get_f32`, round in f64.
- HIGH derived-Default zeroing → hand-written `Default`.
- stat single-truncation + `add_harvested_raw` (not floor-then-×5).

## Deferred / flagged (NOT in P7)
- `TiberiumType.value` → `MinerConfig` wiring (hash-neutral; hardcoded 25/50 already match).
- Silo/credit cap — gamemd does NOT cap the ore deposit (verified unconditional drain at 0x00522D50). Do not add.
- AI virtual-purifier `g_GameMode != 0` (non-campaign) gate — missing from `effective_purifier_count`
  (miner_system.rs, concurrent-session-owned). Pre-existing, out of income-formula scope; fix when
  campaign-solo is modelled.
