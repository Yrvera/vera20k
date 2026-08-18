# Damage Substrate Service — Design Spec (Brainstorm)

> **⚠ 2026-06-04 post-plan verification overturned several facts in this spec.** A
> refute-oriented Ghidra re-pass corrected: leptons/cell = **256** not 128
> (`0x007e2224 = 0x43800000`); running MaxDamage = **10000** not 1000; the Red
> threshold is a **double** compare (no ftol); the attacker chain is FirePower →
> VeteranCombat(0x670) → Occupy(0xf40) → TankBunker(0xf4c) → OpenTopped(0xf58)
> with **no deploy/gattling** mult; TypeImmune is gated **after** the divides;
> building min-1 runs before the zero-check. See **"⚠ VERIFICATION CORRECTIONS"**
> at the top of `2026-06-04-damage-substrate-service-implementation-plan.md` for
> the cited evidence. The CORE service was implemented against the corrected facts.

**Status:** DESIGN SPEC (brainstorm). Doc work only — no `src/` edits. Not an approved implementation plan.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics. `sim/` never depends on `render/ui/audio/net`.
**Bar:** indistinguishable on player-observable OUTPUT — damage numbers to the last decimal, armor-vs-warhead multipliers exact.
**Slot:** master-TODO item #5 (combat/projectile/warhead pipeline) of the engine-substrate program (`docs/plans/2026-05-29-core-engine-substrate-todo.md`).

## Provenance posture

Load-bearing facts below are bit-VERIFIED in the four cited docs (all 2026-06-04). I do **not** re-run Ghidra in this design pass; every address/offset/order carries the inline citation from those docs. Where a doc and the parent study disagree, the **gate doc wins** (it is the later, ASM-verified resolution) and I flag the discrepancy.

Sources:
- `docs/research/DAMAGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (parent study; Pass-2 verified).
- `docs/research/GATE_DAMAGE_VERSES_F64_RESOLUTION_GHIDRA_REPORT.md` (D1 — Verses f64 read + parse).
- `docs/research/GATE_DAMAGE_MAXDAMAGE_CLAMP_RESOLUTION_GHIDRA_REPORT.md` (D2 — clamps).
- `docs/research/GATE_DAMAGE_COUNTRY_ARMOR_ORDER_RESOLUTION_GHIDRA_REPORT.md` (D3 — divide + order + ftol).

**Two parent-study constants are now CORRECTED by the gates and are authoritative below:**
1. **MaxDamage default = 1000 (0x3E8)**, NOT 10000. (`GATE_…_MAXDAMAGE` §a, `read_memory 0x006674d0`.) Every "10000" in the parent study and gate D3 is stale.
2. **leptons-per-cell inside the kernel's CellSpread→lepton conversion = 128.0f**, NOT 256.0. (`GATE_…_VERSES_F64` §D1c, `read_memory 0x007e2224 = 0x43800000`.) This is the single most dangerous cross-gate thread — see OPEN QUESTIONS Q1.

---

## 1. Goal & non-goals

### Goal
One **pure, fixed-point damage-math service** in `sim/combat/damage/` that converts
`(raw weapon damage + warhead + target armor + distance + attacker/defender vet/country/per-unit state + immunity inputs)`
into a final integer HP delta plus a death-state classification, reproducing gamemd's `ftol`-truncated, multi-stage pipeline **to the last decimal**. It replaces the current single-multiply shortcut and its two duplicate copies, retires the dead ProneDamage application, and adds the entire missing attacker-mult and receiver-divide chains.

### Non-goals (adjacent systems this service does NOT own)
- Projectile flight, target acquisition, retaliation scheduling, weapon selection (only the Verses **gate** for selection lives nearby — keep existing `verses_gate`).
- Warhead **side-effects**: Temporal/EMP/MindControl/IvanBomb/Parasite state machines, rocking/Deform, poison-tick, radiation field, money transfer, animation/debris spawn. The service touches these only where the **damage number** depends on them (the immunity gates, and the MindControl "apply 0 HP, return 1" path).
- `[CombatDamage]` particle defaults (`rules/combat_damage.rs`) and bridge-warhead names (`rules/bridge_warheads.rs`) — out of scope, leave untouched.
- AI threat-score feedback, ammo decrement on hit, EstimatedHealth resync scheduling — they *consume* the final clamped damage but are not damage-number stages.
- AoE **target collection** (`Apply_area_damage` ring-walk / eligibility / layer selection) stays in `combat_aoe.rs`; the service owns only the per-target **number** computed inside that loop.

---

## 2. Service boundary

### Where it lives
`src/sim/combat/damage/` — a new submodule of `sim/combat`.

- **Depends on:** `rules/` (`WarheadType`, `ObjectType` for `Armor=`/`Strength=`), `util/fixed_math` (`SimFixed`, truncate-toward-zero `sim_to_i32`). Read-only over value-types; no `EntityStore`/`GameEntity` reach-in — callers extract inputs into value-types. **Never** depends on render/ui/audio/net.
- **Consumed by:** `combat_aoe.rs` (per-target inside its target loop, replacing `aoe_damage_at_distance`), the `mod.rs` direct-hit path (replacing the inline `base*verses/100`), and the `mod.rs` Phase-4 HP-apply site (replacing the coarse `is_invulnerable` + bare `saturating_sub`).

### Data vs apply split (the layering invariant)
- **DATA (rules layer):** Verses table, CellSpread, PercentAtMax, immunity bools, MaxDamage, VeteranArmor/VeteranCombat, country/per-unit mults are parsed in `rules/` and passed in. **Verses specifically is the only INI value gamemd keeps as full f64 through apply** — it MUST consume the slice-1 typed accessor's f64 read path, not the f32-narrowing `get_f32`. (See §4.)
- **APPLY (sim layer):** the ordered `ftol`-truncated pipeline lives in `sim/combat/damage/`. All math `SimFixed`; the only f64 is the parsed Verses value converted once at the rules→sim boundary.

### Types (sketch — not final)
```rust
// sim/combat/damage/mod.rs

/// 0..10 armor class index (none..special_2). Newtype to stop raw-int confusion.
pub struct ArmorClass(pub u8);

/// Verses as fixed-point (gamemd double[11] @ warhead+0xA0). NOT u8 percent.
pub struct VersesTable { values: [SimFixed; 11] }

/// Attacker (Fire_At) + defender (ReceiveDamage) modifiers, gathered by caller.
/// All default 1.0 => no-op (no country / no vet / no per-unit mult).
pub struct CombatMods {
    // Attacker side, each stage ftol-truncated, in this order:
    pub attacker_country_firepower: SimFixed, // HouseClass+0x188
    pub attacker_unit_firepower:    SimFixed, // TechnoClass+0x160 (FirepowerMultiplier) — folded with country in ONE ftol stage
    pub attacker_vet_combat:        SimFixed, // VeteranCombat (Rules+0x670, ~1.1) if vet/elite firepower ability, else 1.0
    pub attacker_deploy:            SimFixed, // Rules+0xf40 if vtable+0x400, else 1.0
    pub attacker_garrison:          SimFixed, // garrison occupy mult, else 1.0
    pub attacker_gattling:          SimFixed, // gattling mult, else 1.0
    // Defender side — DIVIDE, each stage ftol-truncated:
    pub defender_country_armor:     SimFixed, // GetArmorMultForType(target) — incoming DIVIDED by (this × unit_armor)
    pub defender_unit_armor:        SimFixed, // TechnoClass+0x158 (ArmorMultiplier), folded into the same divide
    pub defender_vet_armor:         SimFixed, // VeteranArmor (Rules+0x688, ~1.5) if vet/elite armor ability, else 1.0
}

/// Receiver-side gate inputs (warhead bools + target flags + ally relationship).
pub struct ImmunityInputs { /* type_immune, same_owner, same_whatami, warping_out,
    force_shield, bunkered+whatami, radiation+immune, psychic+immune, poison+immune,
    affects_allies, is_allied, psychedelic+immune+is_building, ... */ }

/// Caller-built target view — decouples the service from GameEntity.
pub struct TargetDamageView {
    pub armor: ArmorClass, pub strength: i32, pub current_hp: i32,
    pub is_building: bool, pub can_c4: bool, /* vet level, immunity flags ... */
}

pub enum DamageGate { Pass, Nullified, MindControlled } // MindControlled => 0 HP, return-code 1
pub enum DamageState { Unaffected, Damaged, Yellow, Red, Dead, PostMortem }
pub struct DamageOutcome { pub hp_delta: i32, pub state: DamageState } // hp_delta<0 = heal
```

### Key function signatures (sketch)
```rust
/// gamemd ApplyWarheadDamage kernel (R1, D1–D6). Pure. The ONE copy (folds both
/// current Rust duplicates).
pub fn apply_warhead_damage(
    damage: i32, wh: &WarheadType, armor: ArmorClass, distance_leptons: i32,
    scenario_no_damage: bool, max_damage: i32,
) -> i32;

/// Attacker Fire_At damage build (R8, D10). Returns the integer stored on the bullet.
pub fn fire_damage(weapon_damage: i32, mods: &CombatMods, wave_or_spawn: bool) -> i32;

/// Receiver pipeline: D7→D9 divides → D11 gates → D13 kernel → D14 building-min1
/// → D17 overkill clamp → D18 classify. Pure over the caller-built view.
pub fn receive_damage(
    incoming: i32, wh: &WarheadType, target: &TargetDamageView,
    mods: &CombatMods, gates: &ImmunityInputs, distance_leptons: i32,
    scenario_no_damage: bool, max_damage: i32,
) -> DamageOutcome;
```

### Consuming the INI accessor service (slice 1) for Verses f64
`rules/warhead_type.rs` parses Verses via a dedicated `parse_verses` that mirrors gamemd's branch (NOT `get_f32`, NOT `read_double` directly — Verses has its own per-token `%`/no-`%` split):
- 11 comma tokens (default `1.0` ×11 if `Verses=` absent — gate D1(b) "Defaults").
- **Per token, branch on `'%'` (byte 0x25) presence** (`strchr` @ `0x007caf30`):
  - has `%`: `value = (atoi(token) as f64) * 0.01` — **integer-truncating** `atoi` BEFORE ×0.01. `"50.5%"`→0.5, `"0.5%"`→0.0, `"-50%"`→-0.5. (`atoi` `0x007c9bfd`, `0.01` const `0x007e3808`.)
  - no `%`: `value = strtod(token)` — full f64. `"0.505"`→0.505. (`strtod` `0x007c9d66`.)
- Stored as **f64-precision** (`SimFixed` with ≥ f64-equivalent mantissa, or carry f64 to the sim boundary and convert once). **Never `u8`.**

The bare-decimal `strtod` path is exactly what slice-1's `read_double` already does for the no-`%` case (sscanf `%f` widened). The `%` path differs (slice-1 `read_double` does float-parse-then-×0.01; Verses uses integer `atoi`-then-×0.01) — so `parse_verses` must NOT just call `read_double`; it reuses slice-1's `atoi_lenient` for the `%` branch and slice-1's `parse_leading_f32`/equivalent for the bare branch. **Reference slice-1 (`rules/ini_value.rs`) helpers; do not re-implement parsing primitives.**

---

## 3. The gamemd behavior CONTRACT (ordered pipeline, ftol + fixed-point/f64 split)

`ftol` = `Math__ftol @ 0x007c5f00` = **truncate toward zero** (FPU CW 0x0E7F). Every "→ftol" is one truncation to int. `SimFixed`'s `sim_to_i32` already truncates toward zero — use it as the `ftol` analog (NOT `.round()`, NOT floor-toward-−∞).

This is the exact ordered formula the service must reproduce for a positive, non-immune hit. Each citation is to the verified gate doc.

### 3a. Attacker side — `Fire_At @ 0x006fdd50` (D3 §c, D10)
Gate: if `weapon Wave(+0x130) || weapon+0x129` → whole chain skipped, **stored damage = 0** (a Wave/spawn carries no bullet damage). Otherwise:
```
d = weapon.Damage (weapon+0xa4)                               # int
d = ftol(d × country_firepower(House+0x188) × unit_firepower(Techno+0x160))   # A1, ONE ftol stage
if vet/elite firepower ability:  d = ftol(d × VeteranCombat[Rules+0x670])     # A2  (~1.1)
if deploy(vtable+0x400):         d = ftol(d × Rules+0xf40)                     # A3
if garrison:                     d = ftol(d × garrison_mult)                   # A4
if gattling:                     d = ftol(d × gattling_mult)                   # A5
# result = integer base damage stored on the bullet
```
Ability gates: FIREPOWER vet `type+0x29e` / elite `type+0x2b0`. (D3 §c, `get_assembly_context 0x006fe337/3c8/3f1`.)

### 3b. Receiver pre-pipeline — `TechnoClass::ReceiveDamage @ 0x00701900` (D3 §a/§c, D7–D9)
```
d = ftol(d / (country_armor(GetArmorMultForType) × unit_armor(Techno+0x158)))  # R1 DIVIDE, ONE ftol
if vet/elite armor ability:  d = ftol(d / VeteranArmor[Rules+0x688])           # R2 DIVIDE (~1.5)
d = max(d, 1)                                                                  # R3 defender min-1 (positive only)
```
- **R1 is a DIVIDE** (`FDIVR` @ `0x0070195d`) — tougher country/unit = `damage ÷ mult`. Country mult source = defender's HouseTypeClass switched on target `WhatAmI()`: Inf +0x108, Air +0x100, Bldg +0x104, flying-unit +0x110, ground-unit +0x10c, default 1.0. (`decompile 0x0050bd30`.)
- Armor ability gates: ARMOR vet `type+0x29d` / elite `type+0x2af`.
- **R3 min-1 sits AFTER the two divides, BEFORE immunity gates and BEFORE the Verses kernel** (`GATE_…_MAXDAMAGE` §b1, order is load-bearing — do NOT move it after Verses).

### 3c. Immunity gates — `TechnoClass::ReceiveDamage` (D11), short-circuit to 0
Checked in this order (each → return 0 unless noted). **TypeImmune is checked first, before the armor divides** (`0x007019e3`); the rest run after R3:
1. **TypeImmune** (`0x007019e3`): attacker present, `type+0xc8c` set, attacker WhatAmI == target WhatAmI, same owner `+0x21c` → 0.
2. **WarpingOut** `vtable+0x160` → 0.
3. **ForceShield/invuln** `vtable+0x1d4` → 0.
4. **Bunker/wall**: occupying-building `field_0x2e4` + `WhatAmI==6` → `warhead+0x146` (Wall) gate; non-building cell-match → 0.
5. **Radiation** `warhead+0x177` && `type+0xd37` (ImmuneToRadiation) → 0.
6. **PsychicDamage** `warhead+0x178` && `type+0xd36` → 0.
7. **Poison** `warhead+0x156` && `type+0xd3b` → 0.
8. **!AffectsAllies** (`warhead+0x179==0`) && attacker present && `IsAlliedWith` → 0. (AffectsAllies default TRUE.)
9. **Psychedelic/MindControl** `warhead+0x16d`: if allied → 0; if ImmuneToPsionics → 0; if building → 0; else MC bookkeeping (calls kernel with NULL warhead → 0 HP) and **returns 1** (damaged-marker, no HP delta).

### 3d. Kernel — `ApplyWarheadDamage @ 0x00489180` (D1–D6, D13), runs via `ObjectClass::ReceiveDamage @ 0x005f5390`
```
# early-outs: return 0 if damage==0 OR ScenarioFlags&0x20 OR warhead==NULL
# healing (damage < 0): return (armor >= 8) ? 0 : damage   # bypasses falloff+Verses; special_1/2 cannot heal
csL  = ftol(CellSpread × 128.0)              # <-- 128, NOT 256 (gate D1c). ONE ftol.
fall = (damage×PAM != damage  &&  csL != 0)  # float-exact branch guard (PAM==1.0 => flat)
         ? ftol(damage × lerp(PAM, 1.0, (csL − dist)/csL))   # interior ftol #2; lerp in 80-bit x87
         : damage
fall = max(fall, 0)                          # zero-crossing floor
d    = ftol((double)fall × Verses_f64[armor])   # interior ftol #3; Verses = double @ wh+0xA0+armor*8
d    = min(d, MaxDamage[Rules+0x16C8])       # default 1000 (NOT 10000); signed, inclusive-on-equal
```
Contract is exactly **`ftol( ftol(lerp) × Verses_f64 )`** — two truncations on the damage value plus one on `csL`. (D1(a), `disassemble 0x00489180`: three ftol @ `0x004891e4/0x00489220/0x00489244`.)

### 3e. HP apply + classify — `ObjectClass::ReceiveDamage @ 0x005f5390` (D14, D16–D19)
```
if building && !CanC4(+0x1577==0):  d = max(d, 1)        # D14 building min-1, post-Verses
d = min(d, currentHealth)                                # D17 overkill clamp
Health -= d
state: Yellow if integer (Strength>>1) crossing;         # D18 Yellow = integer Strength>>1, NOT Rules+0x1700
       Red   if (double)Strength × Rules+0x1708 crossing;
       Dead  if HP==0; PostMortem if IsAlive==false
# D19 death credit: RegisterDestruction(attacker or attacker.Owner) then MarkForDeath
```

### Fixed-point vs f64 split (to the last decimal)
- **f64 only:** the parsed Verses value (kept full f64 through the kernel multiply), `PercentAtMax`/`CellSpread` read as **f32** in gamemd, lerp intermediates in 80-bit x87. In Rust: Verses → `SimFixed` (≥f64 precision) converted once at parse; PAM/CellSpread already `SimFixed`/`u8` from parse. The kernel does **one** `SimFixed × SimFixed` then `sim_to_i32`.
- **integer/fixed everywhere else:** every stage boundary truncates to int (`sim_to_i32`), so the only place 80-bit x87 vs f64-vs-`SimFixed` could diverge is the lerp intermediate — and the ftol at each boundary collapses it for all sampled inputs (study §9 residual; see Q3).
- **No `f32`/`f64` in the apply path** beyond the single Verses conversion — matches the project float rule.

### Worked example (acceptance anchor)
~~100 dmg, Verses 0.5 (Heavy), CellSpread 1.0, PAM 0.25, dist 128 leptons → **31**.~~ **SUPERSEDED — see Design-review correction C-4.** The "31" was computed under the stale 256 leptons/cell assumption (`DAMAGE_MATH_GHIDRA_REPORT §1`). Under the verified `csL = ftol(1.0×128) = 128`, dist 128 → falloff `ftol(100×0.25)=25` → `ftol(25×0.5)=12`, i.e. **12** (and that still assumes dist shares the 128-scaled unit — the open Q1). Do NOT use 31 as the P1 acceptance value; recompute the anchor after Q1 resolves the distance unit.

---

## 4. Tiny-detail ledger (every observable detail)

| # | Detail | Contract | Source |
|---|---|---|---|
| 1 | **ProneDamage = DEAD** | Never read in YR. Do NOT apply; retire `apply_prone_damage_modifier`. | Study §3 / Pass-2 byte sweep |
| 2 | ftol order | `ftol(ftol(lerp)×Verses)` — two damage truncations + one on csL | D1(a) |
| 3 | Verses type | f64 `double[11]` @ wh+0xA0; keep full precision, not u8 | D1(a) |
| 4 | Verses `%` parse | `atoi(token)×0.01` (integer-trunc) for `%`; `strtod` for bare | D1(b) |
| 5 | leptons/cell in kernel | `CellSpread × 128.0` (NOT 256) | D1c |
| 6 | MaxDamage default | **1000** (0x3E8), NOT 10000; signed `>=` cap on kernel output, per target | MAXDAMAGE §a |
| 7 | MinDamage key | parsed at Rules+0x16C4 but **DEAD** (zero reads) — never apply | MAXDAMAGE §c |
| 8 | Defender min-1 | `max(d,1)` AFTER armor+vet divide, BEFORE gates & Verses; positive only | MAXDAMAGE §b1 |
| 9 | Building min-1 | `max(d,1)` post-Verses, buildings w/o CanC4 only | MAXDAMAGE §b2 / D14 |
| 10 | Overkill clamp | `d = min(d, currentHP)` before subtract (affects kill-credit/EstimatedHealth) | MAXDAMAGE §b3 / D17 |
| 11 | Country armor = DIVIDE | `d ÷ (GetArmorMultForType × Techno+0x158)`, NOT multiply | D3 §a |
| 12 | Country mult by type | Inf+0x108 / Air+0x100 / Bldg+0x104 / flying+0x110 / ground+0x10c / default 1.0 (defender HouseType) | D3 §a |
| 13 | Per-unit +0x158 | ArmorMultiplier (receiver, folded into divide) | D3 §b |
| 14 | Per-unit +0x160 | FirepowerMultiplier (attacker, folded into FirePower stage) | D3 §b |
| 15 | VeteranArmor | Rules+0x688 (~1.5) DIVIDE; gates 0x29d/0x2af | D3 §c |
| 16 | VeteranCombat | Rules+0x670 (~1.1) MUL; gates 0x29e/0x2b0 | D3 §c |
| 17 | Deploy FP mult | Rules+0xf40 if vtable+0x400 | D3 §c |
| 18 | Healing | `damage<0` bypasses falloff+Verses; armor index ≥8 cannot heal | D1(a)/D2 |
| 19 | PAM==1.0 branch | float-exact `damage×PAM != damage` guard → flat damage | D1(a)/D3 |
| 20 | Zero-floor | `falloff = max(falloff,0)` before Verses | D1(a) |
| 21 | ScenarioFlags&0x20 | global no-damage → kernel returns 0 (also in Apply_area_damage) | D1(a) |
| 22 | Wave/spawn weapon | `weapon Wave(+0x130) || +0x129` → attacker chain skipped, stored 0 | D3 §c |
| 23 | TypeImmune order | checked BEFORE armor divides (same-type+same-owner → 0) | D3 §c |
| 24 | Immunity gate order | Warp→ForceShield→Bunker→Rad→Psychic→Poison→AffectsAllies→Psychedelic | D11 |
| 25 | AffectsAllies default | TRUE (warhead+0x179=1) | D11 |
| 26 | MindControl HP | applies 0 HP, returns code 1 (2nd kernel call w/ NULL warhead) | Study P2.1 |
| 27 | Yellow classify | integer `Strength>>1` crossing, NOT Rules+0x1700 double | D18 |
| 28 | Red classify | `Strength × Rules+0x1708` (~0.25) double crossing | D18 |
| 29 | AoE same base/per-dist | every target gets same base damage, its own distance; falloff per-target inside kernel | D20 |
| 30 | In-air aircraft | `WhatAmI==2 && IsInAir` → distance halved before eligibility | D21 |
| 31 | Verses select gate | Verses[armor]==0 → weapon switch; ≤0.01 suppresses auto-acquire/retaliation | D23 (keep existing `verses_gate`) |
| 32 | VeinholeMonster clamp | `WhatAmI==0xF` HP clamp = TS-legacy, do NOT model | D3 note |
| 33 | Deform/rocking | terrain crater + rocker impulse = separate subsystem, not HP | Study P2.3 |

---

## 5. Shadow-first rollout

Mirrors the proven Mission/Radio rhythm: shadow (read-only assert) → invert hash-invariant → drop shadow asserts → authoritative → `SNAPSHOT_VERSION` bump → parity harness. **No math becomes authoritative before its slice's shadow proves equality-or-intended-divergence.**

### Additive / read-only (NOT hash-relevant)
- P1 — the pure kernel module behind a shadow flag asserting equality with the existing path for matched inputs.
- P3/P4/P6 shadow phases — compute the new pipeline alongside the old, assert/log, do not flip.
- The Verses-precision parse migration (P5) is **additive** as long as nothing reads the new field yet.

### Hash-relevant (require `SNAPSHOT_VERSION` bump, integration-owned)
Any change to the **damage number** or **death-state classification** is hash-relevant: kernel formula flip, ProneDamage retirement (changes prone-infantry damage), MaxDamage cap, country/vet/per-unit mults, overkill clamp, immunity-gate set, state enum. These flip the per-tick `state_hash`.

**`SNAPSHOT_VERSION` is CONTENDED across substrate programs** (currently 17, last bumped by Mission/MissionCom slice 8 T4). Multiple in-flight programs (factory/house, bridge-occupancy, mission) all queue bumps. **Do NOT hard-code a version number in this plan.** The bump is **integration-owned**: the damage authoritative slice (P7) requests a bump from whoever sequences `snapshot.rs`, takes whatever the next free integer is at merge time, and updates the `assert_eq!(SNAPSHOT_VERSION, …)` test together with the parity baseline in the same commit. Treat the number as a merge-time fill-in, not a design constant.

### Slice order (dependency-ordered, each shippable)
- **P0 — Research gate: CLOSED** (all four gate docs verified). Authoritative changes unblocked except the optional MaxDamage-INI exceedance check (Q4).
- **P1 — Pure kernel + fold both copies** (shadow). Tests: worked-example (**12 under the 128 kernel, NOT 31 — see C-4**; gate on Q1 for the dist unit), double-ftol divergence, healing-blocked-special-armor, PAM==1.0 flat, MaxDamage cap (default **1000**).
- **P2 — Retire ProneDamage application.** Drop `apply_prone_damage_modifier` from both paths. Test: prone GI takes same damage as standing GI. Sharpest player-visible fix — ship early (hash-relevant).
- **P3 — Receiver immunity gates** (D11), value-type pipeline, shadow vs coarse `is_invulnerable`. Tests: TypeImmune same-owner zeroes; AffectsAllies default-hits / no-blocks ally; radiation/poison/psionic immune zeroes; MindControl applies 0 HP / returns marker.
- **P4 — Overkill clamp + state classify** (D17/D18). Tests: 500 dmg vs 50-HP reports 50; Yellow uses integer Strength>>1; Yellow/Red/Dead transitions.
- **P5 — Verses precision migration** u8→`SimFixed`/f64 (consumes slice-1 parse). Test: `Verses=0.005` / `1.5%` preserved vs u8-rounded; `"50.5%"`→0.5 vs `"0.505"`→0.505.
- **P6 — Country + vet/elite + per-unit mults** (`CombatMods`, D7–D10). Tests: VeteranArmor divides (`ftol(d/1.5)`); country ArmorUnitsMult scales; per-unit Firepower/Armor mult; defender min-1 floor; attacker FirePower chain.
- **P7 — Authoritative + SNAPSHOT_VERSION bump** (integration-owned, §5). Drop shadow asserts; service is the only damage path; CI grep-gate proves no remaining call to retired formulas.
- **P8 — Global parity harness.** Deterministic replay of a scripted skirmish (mixed warheads × mixed armors × distances × vet tiers × country bonuses), golden per-tick state-hash (mirrors the Slice-8 global parity harness in recent commits).

---

## 6. Ad-hoc Rust to retire (file:symbol)

| # | Retire / fold | Location | Why |
|---|---|---|---|
| R-1 | `aoe_damage_at_distance` | `src/sim/combat/combat_aoe.rs:327` | Wrong truncation order (single `base×verses×falloff/10000`), no MaxDamage, no country/vet/per-unit mults. Fold into `apply_warhead_damage`. |
| R-2 | **`apply_prone_damage_modifier`** + `prone_damage_basis_points` + `is_prone_for_damage` callers | `src/sim/combat/mod.rs:152`; `src/rules/warhead_type.rs:69,212` (`parse_prone_damage_basis_points`); `combat_aoe.rs:207,315`; `mod.rs:2225` | **WRONG.** ProneDamage dead in YR. Deals 50–70% wrong damage to prone infantry. Stop applying; keep parse only if save round-trip needs it. |
| R-3 | Inline direct-hit `base_damage × verses_pct / 100` | `src/sim/combat/mod.rs:2223` | Duplicate of R-1; fold into the single kernel. |
| R-4 | Coarse immunity nullify | `src/sim/combat/mod.rs:1644` (`is_invulnerable` → full nullify) | Replace with ordered D11 gate set; keep IronCurtain/ForceShield, add TypeImmune/WarpingOut/Radiation/Psychic/Poison/AffectsAllies/Psychedelic. |
| R-5 | Bare `saturating_sub` HP apply | `src/sim/combat/mod.rs:1653` | Add D17 overkill clamp + D18 state return so kill-credit/EstimatedHealth/condition transitions are gamemd-shaped. |
| R-6 | `verses: Vec<u8>` (+ `verses_pct: u8`) | `src/rules/warhead_type.rs:36`, `combat_weapon.rs` | Migrate to `SimFixed` f64-precision Verses (D5). Keep `verses_gate` 0/1/>1 thresholds (derive percent from the fixed value). |
| — | `aoe_damage_at_distance` AoE spread_leptons `×256` | `combat_aoe.rs:96` | Quick-reject radius uses 256 leptons/cell; kernel's falloff csL uses 128. Reconcile under Q1 — the reject radius (target *collection*) may legitimately differ from the kernel's falloff denominator; do not blindly change both. |

`src/rules/combat_damage.rs` (particle defaults) and `src/rules/bridge_warheads.rs` (warhead names) — **out of scope, leave as-is.**

---

## 7. OPEN QUESTIONS (for design review)

**Q1 — leptons-per-cell: 128 (kernel falloff) vs 256 (AoE distance/collection). CROSS-GATE, BLOCKS the worked-example anchor.**
Gate D1c proved the kernel computes `csL = ftol(CellSpread × 128.0)`. But the Rust AoE collection (`combat_aoe.rs:96`, `lepton_distance_sq_raw`) and RA2's canonical lepton are 256/cell. Two unknowns: (a) what unit is the **distance argument** the kernel receives — is it 256-leptons (so the falloff is genuinely steeper, reaching PAM at half the geometric radius), or does `Apply_area_damage` feed a 128-scaled distance so the ratio `(csL−dist)/csL` is unit-consistent? (b) The worked-example "31" assumed dist 128 — does that hold under the real unit? **Must reconcile `Apply_area_damage @ 0x00489280`'s distance scale against the kernel's `×128` before P1's acceptance test is trustworthy.** This is the highest-risk item; flagged open by gate D1 itself.

**Q2 — `CombatMods` plumbing source.** The service is pure over value-types, but the caller (`combat/mod.rs`) must gather country FirePower (HouseClass+0x188), per-unit Firepower/Armor mults (Techno+0x158/+0x160), vet level + ability bits, deploy/garrison/gattling state, and the defender's HouseType per-type armor mult. Which of these are already on `GameEntity`/`HouseState`/`ObjectType` today, and which need new fields parsed/tracked? (E.g. per-unit FirepowerMultiplier/ArmorMultiplier — are these even parsed from INI yet?) Needs a field-availability audit before P6.

**Q3 — last-ULP Verses parity (80-bit x87 lerp).** The kernel's lerp runs in 80-bit x87 between ftol boundaries; an f64/`SimFixed` pipeline cannot bit-reproduce x87 in adversarial cases. Study §9 says the per-boundary ftol collapses this for all sampled inputs, but it is unproven across the full input space. Accept "ftol-collapsed equality" as the bar, or invest in a boundary-spanning bit test on the lerp? (Recommend: accept; revisit only if a parity-replay mismatch points here.)

**Q4 — MaxDamage exceedance in stock YR (non-blocking).** Cap default is 1000. Is it ever a no-op for parity, or does some stock weapon × Verses>1 × small-base reach 1000 on a single hit? Cheap follow-up: enumerate `rulesmd.ini` per-weapon Damage × max Verses vs `[CombatDamage] MaxDamage`. Keep the cap regardless; this only tells us whether P1's cap test uses a real or synthetic input.

**Q5 — `DamageState` vs existing condition gates.** Current Rust has `refresh_building_damage_state_gate` + `apply_fear_from_damage` reading `condition_yellow_x1000`/`condition_red_x1000`. The new D18 classify uses integer `Strength>>1` for Yellow (NOT the ratio) and `Rules+0x1708` for Red. How do the existing condition-ratio consumers (building damage state, fear) coexist with the new integer-Yellow classify — is the ratio still used for the smoke-particle gate (it is, per D18: Rules+0x1700 gates the particle, not the state), and does the fear system need the integer crossing instead? Needs reconciliation in P4 so we don't keep two disagreeing Yellow definitions.

**Q6 — Healing path scope.** Current Rust has no heal path (damage clamps to 0). gamemd heals on negative damage (bypassing falloff/Verses, armor≥8 blocked). Is any stock-YR warhead negative-damage (self-heal warheads, hospital)? If yes, P-which-slice owns wiring `hp_delta<0` through to `Health += |delta|` clamped to Strength? Currently out of the slice list — needs a home or an explicit defer.

---

## Design-review corrections (2026-06-04, adversarial review pass)

**Verdict: YELLOW.** Architecture, layering, data/apply split, shadow-first rollout, and the tiny-detail ledger are sound and grounded in the cited gate docs. Five corrections below must be folded before this becomes a plan; none invalidates the chosen boundary. Every retire target was grep-verified this run.

**C-1 — `get_f32` does not exist in slice-1 (`rules/ini_value.rs`); fabricated method name.** §2 "Consuming the INI accessor" and the DESIGNER SUMMARY say Verses must avoid the "f32-narrowing `get_f32`". Grep of `src/rules/ini_value.rs` (this run) shows the public accessors are `read_int / read_bool / read_double / read_string / read_3int / read_minmax / read_point / read_rect / read_color_rgb / read_speed / read_range` — **there is no `get_f32`**. The real precision contract: `read_double` (ini_value.rs:77–95) already parses `%` tokens via `parse_leading_f32` **first** (line 82, f32-narrowed) then ×0.01 — i.e. it is a *float*-parse-then-scale for `%`. The design's underlying reasoning is still correct (Verses' `%` branch needs **integer-truncating `atoi_lenient`**, NOT `read_double`'s float path), but every "`get_f32`" reference must be rewritten to "`read_double`'s f32-narrowed `%` path" or the plan will reference a nonexistent symbol. `atoi_lenient` (ini_value.rs:263) and `parse_leading_f32` (ini_value.rs:289) DO exist and are the correct primitives to reuse. (Grep: `grep -n "pub fn|get_f32|atoi_lenient|parse_leading_f32" src/rules/ini_value.rs`.)

**C-2 — Missing third damage-apply consumer (`mod.rs:1066–1072`).** §2 "Consumed by" and §6.5 name only the `combat_aoe.rs` per-target path, the direct-hit path, and "the `mod.rs` Phase-4 HP-apply site" (cited as 1644/1653/1669). Grep (this run) shows a **separate AoE-apply block at `mod.rs:1064–1080`**: `is_invulnerable` (1066) → `health.current.saturating_sub(aoe_dmg)` (1072) → `refresh_building_damage_state_gate` (1073) → `apply_fear_from_damage` (1075). This is a second coarse-nullify + bare-subtract site with the exact R-4/R-5 DRIFT the service is meant to retire. The plan must list it as a consumer and a retire target, or P7's "no remaining call to retired formulas" CI gate will pass while this path stays divergent. (Grep: `grep -n "is_invulnerable|saturating_sub" src/sim/combat/mod.rs`.)

**C-3 — Line numbers in §6 retire table and §2.6.5 are stale/wrong; cite symbol names, not lines.** Verified actual locations this run: R-1 `aoe_damage_at_distance` = `combat_aoe.rs:327` (OK); R-2 `apply_prone_damage_modifier` = `mod.rs:152` (OK), but its only *apply* call site is `mod.rs:2206` (not "2225"); R-3 inline direct-hit formula `base_damage * selected.verses_pct as i32 / 100` = **`mod.rs:2204`** (design says 2223/2225 — WRONG); R-4 `is_invulnerable` nullify = **`mod.rs:1066` and `mod.rs:1625`** (design says 1644 — WRONG, and misses 1066 per C-2); R-5 `saturating_sub` HP apply = **`mod.rs:1072` and `mod.rs:1634`** (design says 1653 — WRONG, misses 1072); R-6 `verses: Vec<u8>` = `warhead_type.rs:36` (OK), `verses_pct: u8` = `combat_weapon.rs:67` (OK). The write-plan must re-grep and pin every site by symbol, not by the inherited line numbers (which already drifted from the parent study's own numbers).

**C-4 — Cross-gate contradiction inside the gate corpus is real and unresolved; the "31" anchor is UNCHECKED, not just Q1-flagged.** The design correctly adopts 128 (D1) and MaxDamage 1000 (D2). But **gate D3 (`GATE_DAMAGE_COUNTRY_ARMOR_ORDER`) still writes `csL = ftol(CellSpread × 256)` (its line 112) and "MaxDamage … [10000]" (lines 88, 116)** — i.e. D3 carries the stale constants D1/D2 corrected, despite all three being same-day docs. The design's "gate doc wins" rule is ambiguous when two gate docs disagree. Resolution: **D1's `read_memory 0x007e2224 = 0x43800000 = 128.0f` and D2's `read_memory 0x006674d0 = MOV [ESI+0x16C8], 0x3E8 = 1000` are the bit-reads and win; D3's 256/10000 are stale copy-forward and must be treated as WRONG.** The worked-example "31" originates from `DAMAGE_MATH_GHIDRA_REPORT.md §1`, which was computed under the **256** assumption (study §9 line 514 still sources it from that DOC). Therefore the "31" is **NOT a verified anchor under the 128 kernel** — it is UNCHECKED. P1's `kernel_matches_worked_example` must NOT assert 31 until recomputed under `csL = ftol(1.0 × 128) = 128`, dist 128 → `(128−128)/128 = 0` → falloff = `ftol(100 × lerp(0.25,1.0,0)) = ftol(100×0.25) = 25` → `ftol(25 × 0.5) = 12`. Under the 128 kernel the example yields **12, not 31** (and even this assumes dist is in the same 128-scaled unit — the genuine Q1 unknown). Do not ship a test asserting 31.

**C-5 — `SNAPSHOT_VERSION` is currently 17 (verified `src/sim/snapshot.rs:24`); the "integration-owned, don't hard-code" framing is correct.** No correction needed — flagging only that the contended-at-17 claim is accurate (factory.rs, bridge_occupancy_shadow.rs, harvest_mission.rs headers all reference pending bumps). Keep the merge-time fill-in posture.

**Not corrections (confirmed sound):** ftol order `ftol(ftol(lerp)×Verses)`, Verses kept f64, country-armor DIVIDE folding Techno+0x158, ordered immunity gates with TypeImmune-first, defender min-1 before Verses / building min-1 after, overkill clamp before subtract, integer-Yellow `Strength>>1`, ProneDamage DEAD (parse `basis_points` at `warhead_type.rs:212`, applied at `mod.rs:2206` — confirmed live, retire the apply). Q2 (CombatMods field-availability audit) is correctly flagged as a pre-P6 blocker — none of country FirePower / per-unit Firepower-Armor mults are parsed today (grep shows no such fields), so P6 carries hidden parse work.
