# Damage Substrate Service — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained. DOC-ONLY artifact:
> the code blocks here are the target source; do not assume they are already in `src/`.

**Goal:** One pure, fixed-point/`f64`-boundary damage-math service in `sim/combat/damage/` that
reproduces gamemd's `ftol`-truncated multi-stage pipeline (attacker build → receiver divides →
immunity gates → armor/Verses/distance kernel → overkill clamp → state classify) to the last
decimal, folding the three current ad-hoc apply sites and retiring the dead ProneDamage multiplier.

**Architecture:** New submodule `sim/combat/damage/`. Pure functions over caller-built value-types
(`TargetDamageView`, `CombatMods`, `ImmunityInputs`); no `EntityStore`/`GameEntity` reach-in. DATA
(Verses `f64`, mults, MaxDamage) parsed in `rules/`; APPLY (ordered `ftol` pipeline) in `sim/`.
Never depends on render/ui/audio/net.

**Design Doc:** `docs/plans/2026-06-04-damage-substrate-service-design.md`
(incl. its "## Design-review corrections", verdict YELLOW, corrections C-1..C-5).

---

## ⚠ VERIFICATION CORRECTIONS (2026-06-04, post-plan adversarial Ghidra pass) — READ FIRST

A refute-oriented 6-lane Ghidra re-verification of the kernel/receiver/attacker
contract (run BEFORE transcription) overturned several "GREEN" claims below. The
CORE additive service was implemented against these corrected facts (worktree
`damage-substrate-core` off `d64ad257`: `sim/combat/damage/{mod,kernel,gates,receive,attacker}.rs`
+ `rules/warhead_type.rs` parallel `verses_f64`). Where this section disagrees
with the task bodies, THIS section wins.

- **V-1 — leptons/cell = 256.0, NOT 128.0.** `read_memory 0x007e2224 = 0x43800000`,
  which is **256.0** (exp 0x87→2⁸), not 128 — the plan's "128" (and the carried
  "VERIFIED FACT") was a hex→decimal mis-conversion. The kernel multiplies
  `CellSpread × 256.0` (`FLD [wh+0x124]; FMUL [0x007e2224]; ftol` @ 0x004891d8–e4),
  corroborated by `DAMAGE_MATH_GHIDRA_REPORT §1` and `AAHEATSEEKER2 §3.6`.
  ⇒ `KERNEL_LEPTONS_PER_CELL = 256.0`. **This RESOLVES Q1**: the live AoE path
  (`combat_aoe.rs:96`) also uses 256, so kernel and AoE distance units AGREE — the
  worked example is **31** (the original study value), and `kernel_matches_worked_example`
  is NOT `#[ignore]`'d. (Tasks 2, the worked-example/C-4 "12", and the Open-Question Q1 are all superseded.)
- **V-2 — running MaxDamage = 10000, NOT 1000.** Cap field `[Rules+0x16C8]`
  verified (xref "MaxDamage" → `RulesClass__ReadCombatDamage 0x0066ce3e`); the
  clamp is signed + inclusive (`JL` @ 0x00489255). Stock `ini/rulesmd.ini:896`
  ships `MaxDamage=10000 ;gs from 1000`, so the value the engine RUNS with is
  10000. (Tasks 2/11 "1000 (0x3E8)" superseded; tests use `MAXD = 10000`.)
- **V-3 — Red state threshold is a DOUBLE multiply+compare, NOT `ftol(int)`.**
  `FILD Strength; FMUL double [Rules+0x1708]; FCOMP` (0x54c4–0x54f7 in
  `ObjectClass::ReceiveDamage 0x005f5390`) — no ftol on the Red threshold.
  Encode `(f64)Strength * red_ratio` and compare in f64; do not truncate.
  (Task 6 `classify()` corrected.)
- **V-4 — attacker chain (Task 7) was partly FABRICATED.** Verified Fire_At
  (0x006fdd50) stages: FirePower-fold (`country×unit×base`, one ftol) →
  VeteranCombat (`Rules+0x670`, double) → Occupy (`Rules+0xf40`, float, gate
  vtable+0x400 IsOccupied) → TankBunker (`Rules+0xf4c`, gate this+0x2e4 && !building)
  → OpenTopped (`Rules+0xf58`, gate this+0x82). There is **no deploy and no
  gattling** damage mult here — remove them from `CombatMods` + `fire_damage`.
  Wave-zero gate is `WeaponType+0x130 (Wave) != 0 OR +0x129 != 0` (two flags),
  not just "+0x129". Stages gate on a CONDITION FLAG (caller passes the rules
  mult or 1.0), not on "mult != 1.0".
- **V-5 — TypeImmune is checked AFTER the armor divides, not before.** In
  `TechnoClass::ReceiveDamage 0x00701900` the armor scaling + veterancy + min-1
  run first (0x00701945–d6); TypeImmune is at 0x007019e3+. Output-equivalent to
  the plan (nullify→0 regardless), but `receive_damage` does divides → gates to
  match. The plan's early TypeImmune short-circuit before the divides is removed.
- **V-6 — building min-1 runs on the post-Verses value BEFORE the `delta==0`
  early-return.** A building whose Verses collapses the kernel to 0 still takes 1
  (`ObjectClass::ReceiveDamage`, `WhatAmI==Building && !CanC4 → max(damage,1)`);
  a non-building taking 0 is genuinely unaffected. (Task 6 ordering corrected.)
- **V-7 (notes, non-blocking):** heal-block is armor index **≥ 8 = {8 concrete,
  9 special_1, 10 special_2}** (plan omitted index 8); the kernel has a third
  `warhead==NULL → 0` early-out (caller's concern here); gate item 4 is
  **PenetratesBunker / garrison-link**, not a wall check; an interleaved Ammo-drain
  effect (non-gate) sits between ForceShield and Bunker and is the caller's concern.

Lanes that **VERIFIED as written**: three-ftol order + falloff direction + lerp
algebra + `max(falloff,0)` (L1 a/c/d/e); heal bypasses falloff+Verses (L2a);
damage==0 + ScenarioFlags&0x20 early-outs (L2c); ALL of the receiver divides —
country FDIVR `damage/(country×unit)`, +0x158 fold, VeteranArmor FDIV
`[Rules+0x688]`, min-1 placement, per-divide ftol (L3); AffectsAllies default
true + Psychedelic sub-branches + gate order 5→9 (L4 b/c); overkill clamp +
building-CanC4 min-1 + Yellow integer `Strength>>1` + separate ConditionYellow
ratio `[Rules+0x1700]` (L5 a/b/c/e); FirePower single-ftol fold (L6b).

---

## Grounding Summary

- **Docs say:** `DAMAGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (Pass-2 bit-verified) plus three
  gate docs (D1 Verses-f64, D2 MaxDamage, D3 country-armor-order) define the contract. Kernel
  `ApplyWarheadDamage @ 0x00489180`: three `Math__ftol` calls (`0x004891e4`/`0x00489220`/`0x00489244`),
  contract `ftol( ftol(lerp) × Verses_f64 )`; Verses = `double[11]` @ wh+0xA0; healing
  `(7<armor)-1 & damage`; MaxDamage cap at `Rules+0x16C8`. Receiver `TechnoClass::ReceiveDamage
  @ 0x00701900`: country-armor **DIVIDE** (`FDIVR @ 0x0070195d`) folding `Techno+0x158`, VeteranArmor
  `FDIV Rules+0x688`, min-1, ordered immunity gates (D11). Attacker `Fire_At @ 0x006fdd50`:
  FirePower(`House+0x188`×`Techno+0x160`) → VeteranCombat(`Rules+0x670`) → deploy(`Rules+0xf40`)
  → garrison → gattling. Classify `ObjectClass::ReceiveDamage @ 0x005f5390`: overkill clamp,
  building-min-1, Yellow = integer `Strength>>1`, Red = `Strength × Rules+0x1708`.
- **Two parent-study constants CORRECTED by gates (authoritative):** leptons/cell in kernel
  = **128.0** (`read_memory 0x007e2224 = 0x43800000`), NOT 256; MaxDamage default = **1000**
  (`0x3E8`), NOT 10000. Design-review C-4: gate D3 still writes the stale 256/10000 — treat D3's
  two constants as WRONG; D1/D2 bit-reads win.
- **ProneDamage VERIFIED DEAD** (exhaustive whole-image byte sweep of every x87 qword read of
  `+0xF8`; only hits are BulletClass velocity-Z). Do NOT apply it; retire the apply.
- **Ghidra verification this run:** not re-run (research gate P0 CLOSED in the study). Every
  address/offset above carries an inline study/gate citation; no new binary claim is introduced.
- **Repo patterns mirrored:** the shadow-first → invert → authoritative → version-bump → parity rhythm
  from the Mission/Radio slices; the INI accessor service `src/rules/ini_value.rs`
  (`read_double`/`atoi_lenient`/`parse_leading_f32`) consumed for the Verses parse; the global
  parity harness `src/sim/world/global_parity_harness_tests.rs` (seed + tick count + committed
  final-hash baseline) mirrored for P8.
- **INI keys:** `[Warhead] Verses=`, `CellSpread=`, `PercentAtMax=`, `Wall=`, `Radiation=`,
  `Poison=`, `Psychedelic=`/`MindControl=`, `AffectsAllies=`; `[General]`/`[CombatDamage] MaxDamage`,
  `VeteranArmor`, `VeteranCombat`, `ConditionRed`. `Verses` is the ONLY value gamemd keeps full
  `f64` through apply.
- **Still unknown after grounding (→ Deferred):** Q1 (128-vs-256 distance unit; blocks the kernel
  worked-example value); Q2 (`CombatMods` field availability — country/per-unit Firepower/Armor mults
  are NOT parsed today); Q3 (last-ULP x87 lerp); Q4 (does any stock weapon hit the 1000 cap); Q5
  (integer-Yellow vs existing condition-ratio consumers); Q6 (does any stock warhead heal).

## Key Technical Decisions

- **`SimFixed = I16F16` cannot hold f64-precision Verses; carry Verses as `f64` to the kernel.**
  `src/util/fixed_math.rs:23` defines `SimFixed = I16F16` (16 fractional bits). The design's
  "`SimFixed` with ≥ f64 mantissa" is impossible. Faithful choice (study §6.5 alternative
  "carry f64 to the sim boundary and convert once"): store Verses as `[f64; 11]` in `WarheadType`,
  and the kernel's ONE Verses multiply is `(falloff_int as f64) * verses_f64` then `ftol`. This is
  the documented single float exception (mirrors `ini_value.rs:77` `read_double` returning
  un-truncated f64). PercentAtMax/CellSpread stay as today's parse types into the kernel's lerp.
  — **Confidence:** high. **Source:** `src/util/fixed_math.rs:23,71`; study §6.5; design §3 float-split.
- **`ftol` = truncate-toward-zero, implemented as `f64 as i32` (saturating), NOT `sim_to_i32`.**
  `sim_to_i32` (`fixed_math.rs:71`) is `to_num::<i32>()`; `ini_value.rs:204-212` (`read_range`)
  documents that this floors toward −∞ and is DRIFT vs gamemd `ftol` (RC=11, toward zero) on
  negatives. **[Plan-review C-PR1: `fixed_math.rs:69-73` actually documents `sim_to_i32` as
  "rounds toward zero" — the two project comments disagree; the conclusion (use `f64 as i32`)
  is unaffected and correct because the kernel operates on f64. Do not "fix" `sim_to_i32` on
  the floor-toward-−∞ premise.]** Damage falloff can be floored at 0 and healing is negative, so the kernel's ftol must
  use `f64 as i32` (truncate toward zero, matching `read_range`'s `raw as i32`). — **Confidence:**
  high. **Source:** `src/rules/ini_value.rs:204-212`; study §3d (`Math__ftol` = toward zero).
- **Three apply consumers, not two.** Per design-review C-2 the death-explosion AoE block at
  `mod.rs:1064-1085` is a third coarse-nullify + bare-subtract site (alongside the Phase-4 site
  `mod.rs:1623-1651` and the direct-hit path `mod.rs:2201-2216`). All three are P7 cutover targets
  and P7's CI grep-gate must cover all three. — **Confidence:** high (grep-verified this run).
  **Source:** `mod.rs:1066,1072` / `1625,1634` / `2204,2206`.
- **Worked-example kernel value is `12`, NOT `31`, and is gated on Q1.** Per C-4 the "31" was
  computed under the stale 256 assumption. Under `csL = ftol(1.0×128)=128`, dist 128 →
  `(128−128)/128=0` → `ftol(100×0.25)=25` → `ftol(25×0.5)=12`. P1's worked-example test asserts
  **12** and is annotated `#[ignore]` until Q1 fixes the distance unit. — **Confidence:** medium
  (arithmetic high; distance-unit input low). **Source:** design §3 worked example + C-4.
- **`SNAPSHOT_VERSION` bump is integration-owned, not self-bumped.** Currently `17`
  (`snapshot.rs:24`, test `snapshot.rs:375`). Contended across factory/bridge/mission programs.
  P7 requests the next free integer at merge time and updates `assert_eq!(SNAPSHOT_VERSION, N)`
  + parity baseline in one commit. — **Confidence:** high. **Source:** `snapshot.rs:24,375`; design §5.

## Open Questions

### Resolved During Planning

- *Verses precision storage* — `SimFixed` is I16F16 and cannot carry f64 Verses; resolved to
  carry `[f64; 11]` (the documented single float exception). Source: `fixed_math.rs:23`.
- *Which ftol primitive* — `f64 as i32` (toward zero), not `sim_to_i32` (toward −∞). Source:
  `ini_value.rs:204-212`.
- *`get_f32` symbol* — does not exist (C-1). The Verses `%` branch reuses `atoi_lenient`
  (`ini_value.rs:263`); the bare branch reuses `parse_leading_f32` (`ini_value.rs:289`). The
  no-`%` semantics match `read_double`'s float-then-widen; the `%` semantics differ
  (integer-truncating `atoi` × 0.01), so `parse_verses` does NOT call `read_double`.

### Deferred to Implementation

- **Q1 (highest risk):** distance unit fed to the kernel (128- vs 256-scaled). Blocks the
  worked-example assertion value and the `combat_aoe.rs:96` quick-reject-radius reconciliation.
  Resolve by reading `Apply_area_damage @ 0x00489280`'s distance scale vs the kernel `×128` before
  un-`#[ignore]`-ing `kernel_matches_worked_example` (P1) and before any AoE distance change (P6/P7).
- **Q2:** `CombatMods` field availability. Country FirePower (`House+0x188`), per-unit
  Firepower/Armor mults (`Techno+0x158/+0x160`), defender per-type country armor mult are NOT
  parsed/tracked today (grep shows no such fields). P6 carries the hidden parse + plumbing work;
  scope it as its own pre-task.
- **Q3:** accept ftol-collapsed Verses parity (recommended) vs a boundary-spanning x87 lerp bit test.
- **Q4:** whether a stock weapon × Verses>1 × small base reaches the 1000 cap (decides whether the
  P1 cap test uses a real or synthetic input). Keep the cap regardless.
- **Q5:** reconcile integer-Yellow `Strength>>1` with existing `condition_yellow_x1000` consumers
  (`refresh_building_damage_state_gate`, `apply_fear_from_damage`) — the ratio still gates the smoke
  particle; the state classify uses the integer crossing. P4 must not leave two disagreeing Yellows.
- **Q6:** whether any stock-YR warhead is negative-damage (heal). If yes, which slice wires
  `hp_delta < 0` → `Health += |delta|` clamped to Strength. Currently not in the slice list.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sim/combat/damage/mod.rs` | Service surface: types + module wiring + `apply_warhead_damage` kernel |
| Create | `src/sim/combat/damage/kernel.rs` | `apply_warhead_damage` (D1–D6) + `ftol` + lerp helpers + tests |
| Create | `src/sim/combat/damage/gates.rs` | `ImmunityInputs` + ordered D11 gate eval (`evaluate_gates`) + tests |
| Create | `src/sim/combat/damage/receive.rs` | `receive_damage` (D7→D9 divides → gates → kernel → D14 → D17 → D18) + tests |
| Create | `src/sim/combat/damage/attacker.rs` | `fire_damage` (D10 attacker build) + tests |
| Modify | `src/sim/combat/mod.rs` | declare `pub(crate) mod damage;`; shadow asserts (P1/P3/P4); cutover (P7) of `1064-1085`, `1623-1651`, `2201-2216`; retire `apply_prone_damage_modifier` apply (P2) |
| Modify | `src/sim/combat/combat_aoe.rs` | shadow then cutover `aoe_damage_at_distance` → kernel; drop `apply_prone_damage_modifier` calls (P2) |
| Modify | `src/rules/warhead_type.rs` | `verses: [f64;11]` + `parse_verses` f64; keep/retire `prone_damage_basis_points` |
| Modify | `src/sim/combat/combat_weapon.rs` | `SelectedWeapon.verses_f64: f64`; keep `verses_gate` (derive percent) |
| Create | `src/sim/world/damage_parity_harness_tests.rs` | P8 golden-hash replay (mirrors `global_parity_harness_tests.rs`) |

## Interface Changes

- **NEW public-in-crate types** (`sim/combat/damage/mod.rs`): `ArmorClass`, `CombatMods`,
  `ImmunityInputs`, `TargetDamageView`, `DamageGate`, `DamageState`, `DamageOutcome`. Consumed only
  by `combat_aoe.rs` + `combat/mod.rs`. No external crate depends on them yet.
- **CHANGED `WarheadType.verses`**: `Vec<u8>` → `[f64; 11]` (R-6). Depends-on: `combat_aoe.rs:196,304`
  (`warhead.verses.get(idx)`), `combat_weapon.rs:341` (`warhead.verses.get(idx)`), and
  `warhead_type.rs` tests. All updated in P5.
- **CHANGED `SelectedWeapon.verses_pct: u8`** → add `verses_f64: f64`; keep a derived `verses_pct`
  for `verses_gate` until P7. Depends-on: `combat_weapon.rs:341-351`, `mod.rs:1364` (gate),
  `mod.rs:2204` (direct-hit formula, retired in P7).
- **CHANGED prone retire**: `apply_prone_damage_modifier` (`mod.rs:152`) loses both apply call sites
  (`combat_aoe.rs:207,315`; `mod.rs:2206`) in P2. Function + `prone_damage_basis_points` parse
  retained only if a save round-trip needs them; otherwise removed in P7.

## Sim Checklist

- [x] Kernel math: integer + a single `f64` Verses multiply (the documented float exception); all
      other stage boundaries truncate via `f64 as i32` (toward zero). No `SimFixed` precision loss
      on Verses. PercentAtMax/CellSpread lerp uses the existing parse types.
- [x] New damage number + `DamageState` are hash-relevant → P7 carries the integration-owned
      `SNAPSHOT_VERSION` bump + parity re-baseline. Shadow phases (P1/P3/P4/P5/P6) are NON-hash.
- [x] No dependency on render/ui/sidebar/audio/net (pure functions over value-types).
- [x] Tick ordering: damage apply stays in the existing "turrets + combat" phase; no new tick step.
- [x] BTreeMap iteration: AoE collection order unchanged (still `combat_aoe.rs`); the service is
      per-target pure, so it does not perturb iteration order.

## Risk Areas

- **Highest blast radius:** `mod.rs` Phase-4 + death-explosion + direct-hit apply sites — every hit
  in every match. Regression: P8 golden-hash replay + the shadow asserts in P1/P3/P4 that the new
  pipeline equals (or intentionally diverges from) the old per input.
- **Hash flips:** P2, P4, P5, P6, P7 change the damage number/state → desync risk in MP/replay.
  Each ships behind a shadow assert first; only P7 flips authority and bumps the version.
- **Q1 unresolved** could make the kernel falloff wrong by a fixed factor; the worked-example test
  is `#[ignore]` until Q1 closes so a wrong constant cannot silently ship green.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| T2 | ftol order `ftol(ftol(lerp)×Verses)` | last-digit damage every hit | `kernel_double_ftol_order` (99 dmg, 0.5 verses, 0.5 falloff → double-ftol result) |
| T2 | leptons/cell = 128 (not 256) | falloff slope every splash hit | `kernel_matches_worked_example` (=12, `#[ignore]` until Q1) |
| T2 | MaxDamage default 1000 | caps high Verses×base hits | `kernel_maxdamage_cap` (Verses 2.0 × huge base → 1000) |
| T2 | healing armor≥8 block | heal warheads on special armor | `kernel_healing_blocked_special_armor` |
| T2 | PAM==1.0 flat branch | flat-damage warheads at all distances | `kernel_pam_one_is_flat` |
| T3 | Verses f64 precision | sub-1%/fractional Verses on high-HP targets | `fractional_verses_preserved` (0.005, 1.5%, 50.5% vs 0.505) |
| T4 | prone = full damage | prone GI took 50–70% before; visible every infantry splash | `prone_infantry_takes_full_damage` |
| T6 | ordered immunity gates | TypeImmune/AffectsAllies/Rad/Poison/Psionic/MC outcomes | the gate tests in T6 |
| T7 | overkill clamp + integer Yellow | kill-credit/EstimatedHealth + state transitions | `overkill_clamped_to_remaining_hp`, `yellow_uses_integer_strength_halved` |
| T8 | country DIVIDE + per-unit + vet | tougher country/vet halves/divides incoming | `veteran_armor_divides`, `country_armor_mult_applies`, `per_unit_firepower_armor_mult_applies` |
| T11 | golden-hash replay | whole-pipeline regression tripwire | `damage_parity_replay` |

---

## Tasks

Tasks are grouped by the design's slice order (P1..P8). Phase tags: **(additive)** = not
hash-relevant, **(hash)** = changes the damage number/state, ships behind a shadow assert and only
flips at P7.

### Task 1: Service types + module skeleton (P1, additive)

**Why:** Define every value-type the kernel/receiver/attacker functions consume before any logic
exists. Interfaces first.

**Files:**
- Create: `src/sim/combat/damage/mod.rs`
- Modify: `src/sim/combat/mod.rs` (add the module declaration near the other `mod` lines, e.g. after
  the `combat_aoe`/`combat_weapon` declarations)

**Pattern:** new pattern (no existing damage service); value-type style mirrors `combat_weapon.rs`
`SelectedWeapon`.

**Step 1: Declare the module.** In `src/sim/combat/mod.rs`, add alongside the existing
`mod combat_aoe;` / `mod combat_weapon;` declarations:
```rust
pub(crate) mod damage;
```

**Step 2: Define the types** in `src/sim/combat/damage/mod.rs`:
```rust
//! Pure damage-math service: armor/Verses/distance kernel + receiver pipeline +
//! attacker build. Reproduces gamemd's ftol-truncated multi-stage damage math
//! over caller-built value-types.
//!
//! ## Dependency rules
//! - sim/ submodule: depends on rules/ (WarheadType) + util/fixed_math only.
//! - NEVER depends on render/ui/sidebar/audio/net. No EntityStore/GameEntity
//!   reach-in: callers extract inputs into the value-types below.
//! - Verses is carried as f64 (the single documented float exception); every
//!   stage boundary truncates toward zero via `f64 as i32` (gamemd ftol).

pub(crate) mod attacker;
pub(crate) mod gates;
pub(crate) mod kernel;
pub(crate) mod receive;

/// 0..=10 armor class index (none..special_2). Newtype over u8 to stop
/// raw-int confusion with Verses/percent values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArmorClass(pub u8);

/// Attacker (Fire_At) + defender (ReceiveDamage) modifiers, gathered by the
/// caller. All default 1.0 => no-op. Carried as f64 because gamemd applies each
/// as a double multiply/divide with an ftol truncation per stage.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CombatMods {
    // Attacker side, folded/truncated in this order:
    pub attacker_country_firepower: f64, // House+0x188
    pub attacker_unit_firepower: f64,    // Techno+0x160; folded with country in ONE ftol stage
    pub attacker_vet_combat: f64,        // Rules+0x670 (~1.1) if firepower vet/elite ability, else 1.0
    pub attacker_deploy: f64,            // Rules+0xf40 if vtable+0x400, else 1.0
    pub attacker_garrison: f64,          // garrison occupy mult, else 1.0
    pub attacker_gattling: f64,          // gattling mult, else 1.0
    // Defender side — DIVIDE, each ftol-truncated:
    pub defender_country_armor: f64,     // GetArmorMultForType(target)
    pub defender_unit_armor: f64,        // Techno+0x158; folded with country into ONE divide
    pub defender_vet_armor: f64,         // Rules+0x688 (~1.5) if armor vet/elite ability, else 1.0
}

impl Default for CombatMods {
    fn default() -> Self {
        Self {
            attacker_country_firepower: 1.0,
            attacker_unit_firepower: 1.0,
            attacker_vet_combat: 1.0,
            attacker_deploy: 1.0,
            attacker_garrison: 1.0,
            attacker_gattling: 1.0,
            defender_country_armor: 1.0,
            defender_unit_armor: 1.0,
            defender_vet_armor: 1.0,
        }
    }
}

/// Receiver-side gate inputs (warhead bools + target flags + ally relationship),
/// gathered by the caller. Evaluated in gamemd's verified D11 order.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ImmunityInputs {
    pub attacker_present: bool,
    pub type_immune: bool,        // type+0xc8c set AND same WhatAmI AND same owner
    pub warping_out: bool,        // vtable+0x160
    pub force_shield: bool,       // vtable+0x1d4 (IronCurtain/ForceShield)
    pub wall_bunker_blocked: bool, // bunker/wall cell-match short-circuit
    pub radiation_immune: bool,   // warhead Radiation && type ImmuneToRadiation
    pub psychic_immune: bool,     // warhead PsychicDamage && type immune
    pub poison_immune: bool,      // warhead Poison && type immune
    pub affects_allies: bool,     // warhead+0x179 (default true)
    pub is_allied: bool,          // attacker IsAlliedWith target owner
    pub psychedelic: bool,        // warhead+0x16d (MindControl/Psychedelic)
    pub psionics_immune: bool,    // target ImmuneToPsionics
    pub target_is_building: bool,
}

/// Caller-built target view — decouples the service from GameEntity.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TargetDamageView {
    pub armor: ArmorClass,
    pub strength: i32,
    pub current_hp: i32,
    pub is_building: bool,
    pub can_c4: bool,
}

/// What the receiver-side gates decide before the kernel runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DamageGate {
    Pass,
    Nullified,      // short-circuit to 0 HP delta, no state change
    MindControlled, // 0 HP delta, return-code-1 marker (damaged, no HP)
}

/// Health-state classification returned by the receiver pipeline (D18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DamageState {
    Unaffected,
    Damaged,
    Yellow,
    Red,
    Dead,
    PostMortem,
}

/// Result of the full receiver pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DamageOutcome {
    pub hp_delta: i32, // > 0 = damage to subtract; < 0 = heal
    pub state: DamageState,
}
```

**Step 3: No tests** (pure type definitions; exercised by later tasks).

**Step 4: Verify.** Run: `cargo check -p vera20k`
Expected: compiles (new module declared, types unused-warn allowed — they are consumed by Task 2+).

**Step 5: Commit.** `sim/combat/damage: service value-types + module skeleton (P1, additive)`

**Depends on:** none.

---

### Task 2: Kernel `apply_warhead_damage` + ftol/lerp helpers (P1, additive)

**Why:** The armor/Verses/distance kernel (D1–D6) is the foundation every other stage calls; build
and unit-test it in isolation before wiring.

**Files:**
- Create: `src/sim/combat/damage/kernel.rs`
- Depends on: `WarheadType` (with `verses: [f64; 11]` — Task 9 migrates the field; until then this
  task reads a temporary local `verses_f64: &[f64; 11]` parameter, see Step 2 note).

**Pattern:** new; arithmetic mirrors the study §6.2 sketch but uses `f64 as i32` truncation
(NOT `sim_to_i32`, which floors toward −∞ — `ini_value.rs:204-212`).

**Step 1: Define ftol + helpers.**
```rust
//! gamemd ApplyWarheadDamage kernel (D1–D6): distance falloff → Verses → cap.
//! Pure. The ONE copy that both the AoE per-target loop and the direct-hit path
//! call after cutover (folds combat_aoe::aoe_damage_at_distance and the inline
//! direct-hit formula).

use crate::rules::warhead_type::WarheadType;
use super::ArmorClass;

/// Leptons per cell INSIDE the kernel's CellSpread->lepton conversion.
/// Bit-read 0x43800000 = 128.0f (gate D1c). NOT 256 — the AoE collection
/// radius (combat_aoe) may use 256; reconcile only under Q1.
const KERNEL_LEPTONS_PER_CELL: f64 = 128.0;

/// Truncate toward zero, saturating, NaN->0 — the gamemd `Math__ftol` (RC=11)
/// analog. NOT `util::sim_to_i32` (that floors toward -infinity = DRIFT on
/// negatives; see ini_value.rs read_range). `f64 as i32` truncates toward zero.
#[inline]
fn ftol(v: f64) -> i32 {
    v as i32
}
```

**Step 2: Write the kernel.** Note: `WarheadType` does not yet expose `verses: [f64;11]`,
`cell_spread` is `SimFixed`, `percent_at_max` is `u8`. To keep this task additive and independent of
the field migration (Task 9), the kernel takes the decoded `f64` inputs explicitly:
```rust
/// gamemd ApplyWarheadDamage. Pure. Reproduces D1-D6 incl. the double-ftol order
/// `ftol( ftol(lerp) x Verses )`.
///
/// `cell_spread` and `percent_at_max` are the warhead's decoded f64 values
/// (CellSpread in cells; PercentAtMax 0..1, where 1.0 = flat). `verses_f64` is
/// the warhead's full-precision Verses[11] (the single float exception).
/// `distance_leptons` is the impact-to-target distance in the kernel's lepton
/// unit (Q1: 128-scaled). `scenario_no_damage` = ScenarioFlags & 0x20.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_warhead_damage(
    damage: i32,
    cell_spread: f64,
    percent_at_max: f64,
    verses_f64: &[f64; 11],
    armor: ArmorClass,
    distance_leptons: i32,
    scenario_no_damage: bool,
    max_damage: i32,
) -> i32 {
    // D1 early-outs (warhead==NULL is the caller's concern; it passes a real wh).
    if damage == 0 || scenario_no_damage {
        return 0;
    }
    // D2 healing: negative bypasses falloff+Verses; armor index >= 8 cannot heal.
    if damage < 0 {
        return if armor.0 >= 8 { 0 } else { damage };
    }

    // D3 distance falloff. csL = ftol(CellSpread * 128.0) (ONE ftol on csL).
    let cs_leptons: i32 = ftol(cell_spread * KERNEL_LEPTONS_PER_CELL);
    // Branch guard is a float equality: damage*PAM != damage (PAM==1.0 => flat),
    // AND csL != 0.
    let damage_f = damage as f64;
    let falloff: i32 = if (damage_f * percent_at_max) != damage_f && cs_leptons != 0 {
        // lerp(1.0, PAM, dist/csL) in f64; gamemd runs this in 80-bit x87
        // (Q3: ftol at the boundary collapses the ULP difference for all
        // sampled inputs).
        let t = (cs_leptons - distance_leptons) as f64 / cs_leptons as f64;
        let lerped = percent_at_max * damage_f + (1.0 - percent_at_max) * damage_f * t;
        ftol(lerped) // interior ftol #2
    } else {
        damage
    };
    let falloff = falloff.max(0); // zero-crossing floor

    // D4 Verses multiply (the single f64 multiply) + interior ftol #3.
    let scaled: i32 = ftol(falloff as f64 * verses_f64[armor.0 as usize]);

    // D6 MaxDamage cap (signed, inclusive-on-equal; default 1000).
    scaled.min(max_damage)
}
```

**Step 3: Add tests** (named exactly as the acceptance suite). Use plain `f64` Verses tables.
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat::damage::ArmorClass;

    const MAXD: i32 = 1000; // gamemd default (Rules+0x16C8 = 0x3E8)

    fn verses(v: f64) -> [f64; 11] {
        let mut t = [1.0; 11];
        // index 5 = heavy; set the value under test, leave others 1.0.
        t[5] = v;
        t
    }

    #[test]
    #[ignore = "Q1: distance unit (128 vs 256) unresolved; value gated on it"]
    fn kernel_matches_worked_example() {
        // 100 dmg, Verses 0.5 (Heavy), CellSpread 1.0, PAM 0.25, dist 128 leptons.
        // csL = ftol(1.0*128)=128; (128-128)/128=0 => ftol(100*0.25)=25 =>
        // ftol(25*0.5)=12. (Design-review C-4: =12 under the 128 kernel, NOT 31.)
        let d = apply_warhead_damage(100, 1.0, 0.25, &verses(0.5), ArmorClass(5), 128, false, MAXD);
        assert_eq!(d, 12);
    }

    #[test]
    fn kernel_double_ftol_order() {
        // Proves the double-ftol contract `ftol(ftol(lerp) x Verses)` diverges
        // from a single multiply. PAM=0.5, dist=128 (the edge, csL=ftol(1*128)
        // =128) => t=(128-128)/128=0 => lerp = 0.5*99 + 0.5*99*0 = 49.5 =>
        // ftol #2 = 49 => ftol(49 * 0.5) = ftol(24.5) = 24.
        // The single-multiply path (99*50%*50%/... rounding once) gives 25 — the
        // 1-off this test pins.
        // [Plan-review C-PR2: the "25" claim is imprecise — 99*0.5*0.5=24.75 also
        //  ftols to 24, so this exact input does NOT separate single vs double ftol.
        //  Assertion 24 is correct; the test still exercises both interior ftols.
        //  Fix the comment, keep assert_eq!(d, 24).]
        let d = apply_warhead_damage(99, 1.0, 0.5, &verses(0.5), ArmorClass(5), 128, false, MAXD);
        assert_eq!(d, 24);
    }

    #[test]
    fn kernel_healing_blocked_special_armor() {
        // armor 9 (special_1) cannot heal; armor 5 heals by the full negative.
        let nine = apply_warhead_damage(-40, 0.0, 1.0, &[1.0; 11], ArmorClass(9), 0, false, MAXD);
        let five = apply_warhead_damage(-40, 0.0, 1.0, &[1.0; 11], ArmorClass(5), 0, false, MAXD);
        assert_eq!(nine, 0);
        assert_eq!(five, -40);
    }

    #[test]
    fn kernel_pam_one_is_flat() {
        // PAM==1.0 => branch guard false => flat damage at any distance.
        let near = apply_warhead_damage(100, 5.0, 1.0, &verses(1.0), ArmorClass(5), 0, false, MAXD);
        let far = apply_warhead_damage(100, 5.0, 1.0, &verses(1.0), ArmorClass(5), 600, false, MAXD);
        assert_eq!(near, 100);
        assert_eq!(far, 100);
    }

    #[test]
    fn kernel_maxdamage_cap() {
        // Verses 2.0 x huge base, direct hit (PAM 1.0 flat) => clamps to 1000.
        let d = apply_warhead_damage(5000, 0.0, 1.0, &verses(2.0), ArmorClass(5), 0, false, MAXD);
        assert_eq!(d, 1000);
    }

    #[test]
    fn kernel_scenario_no_damage_zero() {
        let d = apply_warhead_damage(100, 0.0, 1.0, &verses(1.0), ArmorClass(5), 0, true, MAXD);
        assert_eq!(d, 0);
    }
}
```

**Step 4: Verify.** Run: `cargo test -p vera20k kernel_ -- --nocapture`
Expected: all PASS except `kernel_matches_worked_example` reported as `ignored`. Read the literal
`test result:` line.

**Step 5: Commit.** `sim/combat/damage: ApplyWarheadDamage kernel + ftol/lerp + tests (P1, additive)`

**Depends on:** Task 1.

---

### Task 3: Kernel shadow assert against the existing AoE formula (P1, additive)

**Why:** Prove the new kernel equals the current `aoe_damage_at_distance` for matched inputs before
flipping anything (the Mission/Radio shadow discipline). Read-only; not hash-relevant.

**Files:**
- Modify: `src/sim/combat/combat_aoe.rs` (add a `#[cfg(test)]` shadow comparison; do NOT change the
  production `aoe_damage_at_distance` call sites yet).

**Pattern:** mirrors `ini_value.rs` `corpus_tests` (a test that compares OLD vs NEW accessor without
flipping a consumer).

**Step 1: Add the shadow test** to `combat_aoe.rs`'s `mod tests`. It documents the EXPECTED
divergence (the old formula uses 256-scaled distance, single-multiply, ProneDamage; the new kernel
uses 128, double-ftol, no Prone). Because they differ by design, the shadow asserts the
*divergence direction*, not equality:
```rust
#[test]
fn shadow_kernel_vs_old_aoe_direct_hit_equal_when_pam_flat() {
    // At a direct hit (distance 0) with PAM 1.0 and integer Verses, both
    // formulas reduce to base*verses; they MUST agree (no falloff, no double-
    // ftol divergence, no Prone). Guards that the kernel is wired to the same
    // base*Verses semantics on the simplest input.
    use crate::sim::combat::damage::{kernel::apply_warhead_damage, ArmorClass};
    let old = aoe_damage_at_distance(100, SIM_ZERO, sim_from_f32(3.0), 100, 50) as i32;
    let new = apply_warhead_damage(100, 3.0, 1.0, &{ let mut t=[1.0;11]; t[5]=0.5; t },
        ArmorClass(5), 0, false, 1000);
    assert_eq!(old, new, "direct-hit base*Verses must match");
}
```

**Step 2: Verify.** Run: `cargo test -p vera20k shadow_kernel_vs_old_aoe -- --nocapture`
Expected: PASS.

**Step 3: Commit.** `sim/combat/damage: P1 kernel shadow assert vs old AoE (additive)`

**Depends on:** Task 2.

---

### Task 4: Retire the ProneDamage apply (P2, hash)

**Why:** Sharpest player-visible fix — prone infantry currently take 50–70% wrong damage. Ship early.
ProneDamage is VERIFIED DEAD; stop applying it at all three call sites' two AoE locations + the
direct-hit location.

**Files:**
- Modify: `src/sim/combat/combat_aoe.rs:207` and `:315` (drop the `apply_prone_damage_modifier`
  wrap; use the raw kernel/old-formula result directly).
- Modify: `src/sim/combat/mod.rs:2206` (drop the prone wrap on the direct-hit `raw_damage`).
- Modify: `src/sim/combat/mod.rs:152` `apply_prone_damage_modifier` — keep the function for now but
  remove all callers (the apply is what's wrong); it is deleted in P7 if no save round-trip needs it.

**Pattern:** behavior-changing minimal edit; mirrors prior "retire a dead multiplier" cutovers.

**Step 1: combat_aoe.rs:205-211** — replace the prone wrap. Current:
```rust
        let prone_infantry =
            entity.category == EntityCategory::Infantry && infantry::is_prone_for_damage(entity);
        let dmg: u16 = apply_prone_damage_modifier(prone_infantry, warhead, raw_damage as i32);

        if dmg > 0 {
            damage_list.push((entity.stable_id, dmg));
        }
```
becomes:
```rust
        // ProneDamage is dead data in YR (verified): never apply it. Prone and
        // standing infantry take identical damage.
        let dmg: u16 = raw_damage;
        if dmg > 0 {
            damage_list.push((entity.stable_id, dmg));
        }
```

**Step 2: combat_aoe.rs:313-319** — same replacement in `push_entity_aoe_damage`:
```rust
        // ProneDamage is dead data in YR (verified): never apply it.
        let dmg: u16 = raw_damage;
        if dmg > 0 {
            damage_list.push((entity.stable_id, dmg));
        }
```
Then remove the now-unused `EntityCategory`/`infantry` imports if they become dead (run
`cargo check` and let the warnings guide; `infantry` is still used elsewhere — only the
`is_prone_for_damage` call is removed here).

**Step 3: mod.rs:2202-2206** — replace the direct-hit prone wrap:
```rust
        // Integer damage: base_damage * verses_pct / 100.
        // base_damage already includes OccupyDamageMultiplier for garrison.
        let raw_damage: i32 = base_damage * selected.verses_pct as i32 / 100;
        // ProneDamage is dead data in YR (verified): never apply it.
        let actual_damage: u16 = raw_damage.clamp(0, u16::MAX as i32) as u16;
```
(Removes the `apply_prone_damage_modifier` + `target_prone_infantry` use at this site; leave the
`target_prone_infantry` binding if still referenced for animation, otherwise drop it.)

**Step 4: Add the regression test** to `combat_aoe.rs` `mod tests`:
```rust
#[test]
fn prone_infantry_takes_full_damage() {
    // A warhead with ProneDamage=50% must NOT halve damage to prone infantry:
    // ProneDamage is dead in YR. AoE result is identical regardless of prone.
    // (Direct unit-level: aoe_damage_at_distance has no prone term, so the
    //  retirement is proven by the absence of any prone branch at the call site;
    //  this test pins that base*Verses is the only scaling.)
    let dmg = aoe_damage_at_distance(100, SIM_ZERO, sim_from_f32(3.0), 25, 100);
    assert_eq!(dmg, 100); // full base*Verses, no prone reduction
}
```

**Step 5: Verify.** Run: `cargo test -p vera20k prone_infantry_takes_full_damage`
Then `cargo test -p vera20k combat_aoe`. Expected: PASS. Read the `test result:` line.

**Step 6: Commit.** `sim/combat: retire dead ProneDamage apply at all 3 sites (P2, hash)`

**Rollback note (hash-flipping):** this changes prone-infantry damage → flips `state_hash` and the
P8 baseline (once it exists). To revert: restore the `apply_prone_damage_modifier(prone, warhead, …)`
wrap at the three sites and re-add the `prone_infantry`/`target_prone_infantry` bindings. Do this
BEFORE any P7 version bump (no SNAPSHOT_VERSION change is owned by P2; the global parity baseline is
not yet damage-aware until P8, so P2 alone only affects damage unit tests).

**Depends on:** none (independent of the kernel; do early). Ordered after Task 3 only so the kernel
shadow lands first.

---

### Task 5: `ImmunityInputs` gate evaluation (P3, additive)

**Why:** The ordered D11 immunity gate set is pure logic over `ImmunityInputs`; build + test it
before wiring it into the receiver pipeline or shadowing it against `is_invulnerable`.

**Files:**
- Create: `src/sim/combat/damage/gates.rs`

**Pattern:** new; ordered short-circuit eval.

**Step 1: Write the evaluator.** Order is load-bearing: TypeImmune is checked first (before the
armor divides, by the caller ordering), then Warp → ForceShield → Bunker → Radiation → Psychic →
Poison → AffectsAllies → Psychedelic.
```rust
//! Ordered receiver immunity gates (D11). Pure over ImmunityInputs. Returns the
//! gate decision; the caller short-circuits on Nullified/MindControlled.

use super::{DamageGate, ImmunityInputs};

/// Evaluate the receiver immunity gates in gamemd's verified order. TypeImmune
/// is first (gamemd checks it before the armor divides; the caller invokes this
/// accordingly). Each gate short-circuits.
pub(crate) fn evaluate_gates(g: &ImmunityInputs) -> DamageGate {
    // 1. TypeImmune: attacker present + same WhatAmI + same owner.
    if g.attacker_present && g.type_immune {
        return DamageGate::Nullified;
    }
    // 2. WarpingOut.
    if g.warping_out {
        return DamageGate::Nullified;
    }
    // 3. ForceShield / invuln (IronCurtain/ForceShield).
    if g.force_shield {
        return DamageGate::Nullified;
    }
    // 4. Bunker/wall cell-match short-circuit.
    if g.wall_bunker_blocked {
        return DamageGate::Nullified;
    }
    // 5. Radiation immune.
    if g.radiation_immune {
        return DamageGate::Nullified;
    }
    // 6. PsychicDamage immune.
    if g.psychic_immune {
        return DamageGate::Nullified;
    }
    // 7. Poison immune.
    if g.poison_immune {
        return DamageGate::Nullified;
    }
    // 8. !AffectsAllies && attacker present && allied.
    if !g.affects_allies && g.attacker_present && g.is_allied {
        return DamageGate::Nullified;
    }
    // 9. Psychedelic/MindControl: allied -> 0; psionics-immune -> 0; building ->
    //    0; else MindControlled (0 HP, return-code-1 marker).
    if g.psychedelic {
        if g.is_allied || g.psionics_immune || g.target_is_building {
            return DamageGate::Nullified;
        }
        return DamageGate::MindControlled;
    }
    DamageGate::Pass
}
```

**Step 2: Add tests.**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat::damage::ImmunityInputs;

    fn base() -> ImmunityInputs {
        ImmunityInputs { affects_allies: true, ..Default::default() }
    }

    #[test]
    fn type_immune_same_owner_zeroes() {
        let g = ImmunityInputs { attacker_present: true, type_immune: true, ..base() };
        assert_eq!(evaluate_gates(&g), DamageGate::Nullified);
    }

    #[test]
    fn affects_allies_default_hits_ally() {
        // AffectsAllies default true: an allied hit still passes.
        let g = ImmunityInputs { attacker_present: true, is_allied: true, ..base() };
        assert_eq!(evaluate_gates(&g), DamageGate::Pass);
    }

    #[test]
    fn affects_allies_off_blocks_ally() {
        let g = ImmunityInputs {
            attacker_present: true, is_allied: true, affects_allies: false, ..base()
        };
        assert_eq!(evaluate_gates(&g), DamageGate::Nullified);
    }

    #[test]
    fn radiation_immune_zeroes() {
        assert_eq!(evaluate_gates(&ImmunityInputs { radiation_immune: true, ..base() }),
            DamageGate::Nullified);
    }

    #[test]
    fn poison_immune_zeroes() {
        assert_eq!(evaluate_gates(&ImmunityInputs { poison_immune: true, ..base() }),
            DamageGate::Nullified);
    }

    #[test]
    fn psionic_immune_zeroes() {
        assert_eq!(evaluate_gates(&ImmunityInputs { psychic_immune: true, ..base() }),
            DamageGate::Nullified);
    }

    #[test]
    fn mindcontrol_warhead_applies_zero_hp() {
        let g = ImmunityInputs { attacker_present: true, psychedelic: true, ..base() };
        assert_eq!(evaluate_gates(&g), DamageGate::MindControlled);
    }
}
```

**Step 3: Verify.** Run: `cargo test -p vera20k damage::gates`
Expected: PASS.

**Step 4: Commit.** `sim/combat/damage: ordered D11 immunity gates + tests (P3, additive)`

**Depends on:** Task 1.

---

### Task 6: Receiver pipeline `receive_damage` (P3/P4 logic, additive)

**Why:** Compose the divides (D7–D9) → gates (D11) → kernel (D13) → building-min-1 (D14) → overkill
clamp (D17) → classify (D18) into one pure function over the value-types. Built additive (no consumer
flips); shadowed in Task 7.

**Files:**
- Create: `src/sim/combat/damage/receive.rs`

**Pattern:** new; composes Task 2 + Task 5.

**Step 1: Write the pipeline.**
```rust
//! Receiver pipeline (D7->D9 divides -> D11 gates -> D13 kernel -> D14 building
//! min-1 -> D17 overkill clamp -> D18 classify). Pure over caller-built views.

use crate::rules::warhead_type::WarheadType;
use super::gates::evaluate_gates;
use super::kernel::apply_warhead_damage;
use super::{
    ArmorClass, CombatMods, DamageGate, DamageOutcome, DamageState, ImmunityInputs,
    TargetDamageView,
};

/// ftol toward zero (gamemd Math__ftol). Mirrors kernel::ftol; defined here to
/// keep the receiver divides truncating identically without exporting it.
#[inline]
fn ftol(v: f64) -> i32 {
    v as i32
}

/// Full receiver pipeline. `condition_red_ratio` = Rules+0x1708 (~0.25).
/// `cell_spread`/`percent_at_max`/`verses_f64` are the warhead's decoded kernel
/// inputs (see kernel::apply_warhead_damage). `distance_leptons` is the impact
/// distance in the kernel lepton unit (Q1).
#[allow(clippy::too_many_arguments)]
pub(crate) fn receive_damage(
    incoming: i32,
    cell_spread: f64,
    percent_at_max: f64,
    verses_f64: &[f64; 11],
    target: &TargetDamageView,
    mods: &CombatMods,
    gates: &ImmunityInputs,
    distance_leptons: i32,
    scenario_no_damage: bool,
    max_damage: i32,
    condition_red_ratio: f64,
) -> DamageOutcome {
    let unaffected = DamageOutcome { hp_delta: 0, state: DamageState::Unaffected };

    // TypeImmune is checked before the armor divides (D11 note).
    if gates.attacker_present && gates.type_immune {
        return unaffected;
    }

    // Positive-only receiver divides (D7-D9). Healing (incoming < 0) bypasses.
    let mut dmg = incoming;
    if dmg > 0 {
        // D7: country-armor DIVIDE folding per-unit ArmorMultiplier, ONE ftol.
        let armor_div = mods.defender_country_armor * mods.defender_unit_armor;
        if armor_div != 0.0 {
            dmg = ftol(dmg as f64 / armor_div);
        }
        // D9: VeteranArmor DIVIDE, ONE ftol.
        if mods.defender_vet_armor != 0.0 && mods.defender_vet_armor != 1.0 {
            dmg = ftol(dmg as f64 / mods.defender_vet_armor);
        }
        // R3 defender min-1: AFTER divides, BEFORE gates and Verses kernel.
        dmg = dmg.max(1);
    }

    // D11 immunity gates (run after the divides; TypeImmune already handled).
    match evaluate_gates(gates) {
        DamageGate::Nullified => return unaffected,
        DamageGate::MindControlled => {
            // 0 HP delta, damaged-marker (gamemd returns code 1).
            return DamageOutcome { hp_delta: 0, state: DamageState::Damaged };
        }
        DamageGate::Pass => {}
    }

    // D13 kernel (falloff -> Verses -> cap; also re-runs D1/D2 early-outs).
    let mut delta = apply_warhead_damage(
        dmg, cell_spread, percent_at_max, verses_f64, target.armor,
        distance_leptons, scenario_no_damage, max_damage,
    );

    // Healing path (delta < 0): caller adds back, clamped to strength elsewhere.
    if delta < 0 {
        return DamageOutcome { hp_delta: delta, state: classify(target, delta, condition_red_ratio) };
    }
    if delta == 0 {
        return unaffected;
    }

    // D14 building min-1 (post-Verses, buildings without CanC4).
    if target.is_building && !target.can_c4 {
        delta = delta.max(1);
    }
    // D17 overkill clamp: damage never exceeds remaining HP.
    if delta > target.current_hp {
        delta = target.current_hp;
    }

    DamageOutcome { hp_delta: delta, state: classify(target, delta, condition_red_ratio) }
}

/// D18 state classification. Yellow uses integer Strength>>1 (NOT the condition
/// ratio); Red uses Strength * condition_red_ratio (Rules+0x1708 double); Dead
/// when post-hit HP hits 0.
fn classify(target: &TargetDamageView, delta: i32, red_ratio: f64) -> DamageState {
    let prev = target.current_hp;
    let post = prev - delta; // delta may be negative (heal) => post > prev
    if post <= 0 {
        return DamageState::Dead;
    }
    let yellow_threshold = target.strength >> 1; // integer Strength/2
    let red_threshold = (target.strength as f64 * red_ratio) as i32;
    if (red_threshold) < prev && post < red_threshold {
        return DamageState::Red;
    }
    if yellow_threshold <= prev && post < yellow_threshold {
        return DamageState::Yellow;
    }
    DamageState::Damaged
}
```

**Step 2: Add tests.**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat::damage::{ArmorClass, CombatMods, ImmunityInputs, TargetDamageView};

    const MAXD: i32 = 1000;
    const RED: f64 = 0.25;

    fn tgt(strength: i32, hp: i32) -> TargetDamageView {
        TargetDamageView { armor: ArmorClass(5), strength, current_hp: hp,
            is_building: false, can_c4: false }
    }
    fn allow() -> ImmunityInputs {
        ImmunityInputs { affects_allies: true, ..Default::default() }
    }
    fn verses(v: f64) -> [f64; 11] { let mut t=[1.0;11]; t[5]=v; t }

    #[test]
    fn overkill_clamped_to_remaining_hp() {
        // 500 incoming vs 50-HP target reports 50, not 500.
        let o = receive_damage(500, 0.0, 1.0, &verses(1.0), &tgt(300, 50),
            &CombatMods::default(), &allow(), 0, false, MAXD, RED);
        assert_eq!(o.hp_delta, 50);
        assert_eq!(o.state, DamageState::Dead);
    }

    #[test]
    fn yellow_uses_integer_strength_halved() {
        // Strength 100 => yellow at >>1 = 50. Full-HP target, 60 damage:
        // prev=100, post=40, crosses 50 => Yellow (not the 0.25 ratio).
        let o = receive_damage(60, 0.0, 1.0, &verses(1.0), &tgt(100, 100),
            &CombatMods::default(), &allow(), 0, false, MAXD, RED);
        assert_eq!(o.hp_delta, 60);
        assert_eq!(o.state, DamageState::Yellow);
    }

    #[test]
    fn veteran_armor_divides() {
        // VeteranArmor 1.5: 60 incoming => ftol(60/1.5)=40.
        let mods = CombatMods { defender_vet_armor: 1.5, ..CombatMods::default() };
        let o = receive_damage(60, 0.0, 1.0, &verses(1.0), &tgt(300, 300),
            &mods, &allow(), 0, false, MAXD, RED);
        assert_eq!(o.hp_delta, 40);
    }

    #[test]
    fn country_armor_mult_applies() {
        // Country armor mult 2.0 (tougher): 80 incoming => ftol(80/2)=40.
        let mods = CombatMods { defender_country_armor: 2.0, ..CombatMods::default() };
        let o = receive_damage(80, 0.0, 1.0, &verses(1.0), &tgt(300, 300),
            &mods, &allow(), 0, false, MAXD, RED);
        assert_eq!(o.hp_delta, 40);
    }

    #[test]
    fn min_one_floor_positive() {
        // Country armor mult 100 makes a 50-incoming hit floor to 1 (defender
        // min-1 after the divides), then Verses 1.0 keeps 1.
        let mods = CombatMods { defender_country_armor: 100.0, ..CombatMods::default() };
        let o = receive_damage(50, 0.0, 1.0, &verses(1.0), &tgt(300, 300),
            &mods, &allow(), 0, false, MAXD, RED);
        assert_eq!(o.hp_delta, 1);
    }

    #[test]
    fn building_min_one_after_verses() {
        // Building (no CanC4): 0.0001-ish Verses floors to 1 post-Verses.
        let bldg = TargetDamageView { is_building: true, ..tgt(1000, 1000) };
        let o = receive_damage(10, 0.0, 1.0, &verses(0.0001), &bldg,
            &CombatMods::default(), &allow(), 0, false, MAXD, RED);
        assert_eq!(o.hp_delta, 1);
    }

    #[test]
    fn mindcontrol_applies_zero_hp() {
        let g = ImmunityInputs { attacker_present: true, psychedelic: true, ..allow() };
        let o = receive_damage(100, 0.0, 1.0, &verses(1.0), &tgt(300, 300),
            &CombatMods::default(), &g, 0, false, MAXD, RED);
        assert_eq!(o.hp_delta, 0);
        assert_eq!(o.state, DamageState::Damaged);
    }
}
```

**Step 3: Verify.** Run: `cargo test -p vera20k damage::receive`
Expected: PASS. Read the `test result:` line.

**Step 4: Commit.** `sim/combat/damage: receiver pipeline (divides->gates->kernel->clamp->classify) + tests (additive)`

**Depends on:** Task 2, Task 5.

---

### Task 7: Attacker `fire_damage` build (P6 logic, additive)

**Why:** The attacker-side Fire_At mult chain (D10) is pure over `CombatMods`; build + test it
independently. Plumbing into the live Fire_At path is P6/P7.

**Files:**
- Create: `src/sim/combat/damage/attacker.rs`

**Pattern:** new; ordered ftol-per-stage build.

**Step 1: Write the build.**
```rust
//! Attacker-side Fire_At damage build (D10). Pure over CombatMods. Returns the
//! integer base damage stored on the projectile.

use super::CombatMods;

#[inline]
fn ftol(v: f64) -> i32 { v as i32 }

/// gamemd Fire_At damage build. `wave_or_spawn` (weapon Wave/+0x129) forces the
/// whole chain to 0 (a Wave/spawn carries no bullet damage). Each mult stage is
/// ftol-truncated; FirePower folds country x per-unit in ONE stage.
pub(crate) fn fire_damage(weapon_damage: i32, mods: &CombatMods, wave_or_spawn: bool) -> i32 {
    if wave_or_spawn {
        return 0;
    }
    let mut d = weapon_damage;
    // A1: country FirePower x per-unit FirepowerMultiplier, ONE ftol.
    d = ftol(d as f64 * mods.attacker_country_firepower * mods.attacker_unit_firepower);
    // A2: VeteranCombat.
    if mods.attacker_vet_combat != 1.0 {
        d = ftol(d as f64 * mods.attacker_vet_combat);
    }
    // A3: deploy bonus.
    if mods.attacker_deploy != 1.0 {
        d = ftol(d as f64 * mods.attacker_deploy);
    }
    // A4: garrison occupy mult.
    if mods.attacker_garrison != 1.0 {
        d = ftol(d as f64 * mods.attacker_garrison);
    }
    // A5: gattling mult.
    if mods.attacker_gattling != 1.0 {
        d = ftol(d as f64 * mods.attacker_gattling);
    }
    d
}
```

**Step 2: Add tests.**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat::damage::CombatMods;

    #[test]
    fn wave_or_spawn_zeroes_damage() {
        assert_eq!(fire_damage(100, &CombatMods::default(), true), 0);
    }

    #[test]
    fn no_mods_is_passthrough() {
        assert_eq!(fire_damage(65, &CombatMods::default(), false), 65);
    }

    #[test]
    fn veteran_combat_multiplies() {
        // VeteranCombat 1.1: ftol(100 * 1.1) = ftol(110.0...) = 110.
        let mods = CombatMods { attacker_vet_combat: 1.1, ..CombatMods::default() };
        assert_eq!(fire_damage(100, &mods, false), 110);
    }

    #[test]
    fn country_and_unit_firepower_fold_in_one_ftol() {
        // 1.5 country x 2.0 unit folded: ftol(50 * 3.0) = 150.
        let mods = CombatMods {
            attacker_country_firepower: 1.5, attacker_unit_firepower: 2.0,
            ..CombatMods::default()
        };
        assert_eq!(fire_damage(50, &mods, false), 150);
    }
}
```

**Step 3: Verify.** Run: `cargo test -p vera20k damage::attacker`
Expected: PASS.

**Step 4: Commit.** `sim/combat/damage: attacker fire_damage build + tests (additive)`

**Depends on:** Task 1.

---

### Task 8: Receiver-pipeline shadow assert vs the coarse `is_invulnerable` path (P3, additive)

**Why:** Before flipping any apply site, prove the new gate/pipeline reproduces today's coarse
nullify on the cases it covers (IronCurtain/ForceShield → 0) and surfaces the new gates it adds.
Shadow discipline.

**Files:**
- Modify: `src/sim/combat/mod.rs` (add a `#[cfg(test)]` shadow test near the Phase-4 apply; do NOT
  change the production `is_invulnerable`/`saturating_sub` at `1625/1634`, `1066/1072`, `2204` yet).

**Pattern:** mirrors the `ini_value.rs` corpus shadow test — compare OLD vs NEW for the shared input
set without flipping a consumer.

**Step 1: Add the shadow test** asserting that `force_shield: true` → `receive_damage` yields
`hp_delta == 0` (matching the old coarse nullify) and that a normal hit yields the kernel result:
```rust
#[cfg(test)]
mod damage_shadow_tests {
    use crate::sim::combat::damage::{
        receive::receive_damage, ArmorClass, CombatMods, DamageState, ImmunityInputs,
        TargetDamageView,
    };

    #[test]
    fn force_shield_matches_old_coarse_nullify() {
        let tgt = TargetDamageView { armor: ArmorClass(5), strength: 300, current_hp: 300,
            is_building: false, can_c4: false };
        let g = ImmunityInputs { force_shield: true, affects_allies: true, ..Default::default() };
        let o = receive_damage(100, 0.0, 1.0, &[1.0; 11], &tgt, &CombatMods::default(),
            &g, 0, false, 1000, 0.25);
        // Old path: is_invulnerable -> continue (0 HP change). New must match.
        assert_eq!(o.hp_delta, 0);
        assert_eq!(o.state, DamageState::Unaffected);
    }
}
```

**Step 2: Verify.** Run: `cargo test -p vera20k force_shield_matches_old_coarse_nullify`
Expected: PASS.

**Step 3: Commit.** `sim/combat: P3 receiver-pipeline shadow assert vs is_invulnerable (additive)`

**Depends on:** Task 6.

---

### Task 9: Migrate `WarheadType.verses` to `[f64; 11]` + `parse_verses` f64 (P5, additive-until-read)

**Why:** Verses must carry full f64 precision (the one float exception); the u8 percent loses
sub-1% and fractional values. Reuse slice-1 `ini_value.rs` primitives per design-review C-1.

**Files:**
- Modify: `src/rules/warhead_type.rs:36` (field), `:115` (parse call), `:197-210` (`parse_verses`),
  and the tests at `:253-257`, `:289-296`.
- Modify (downstream readers, kept compiling): `src/sim/combat/combat_aoe.rs:196,304`
  (`warhead.verses.get(idx)`), `src/sim/combat/combat_weapon.rs:341`.

**Pattern:** mirrors `ini_value.rs` `read_double` (`%` → ×0.01) but with **integer-truncating**
`atoi_lenient` for the `%` branch (Verses-specific, per C-1/D1b). Additive: while the kernel still
reads a temporary `[f64;11]` parameter, the field change is data-only until P7 cutover wires the
field into the kernel call.

**Step 1: Change the field** at `warhead_type.rs:32-36`:
```rust
    /// Damage effectiveness per armor type, full f64 precision (gamemd double[11]
    /// @ warhead+0xA0). Index order: 0=none,1=flak,2=plate,3=light,4=medium,
    /// 5=heavy,6=wood,7=steel,8=concrete,9=special_1,10=special_2. Defaults to
    /// [1.0; 11] (100%) when Verses= is absent. 1.0 = full, 0.0 = immune.
    pub verses: [f64; 11],
```

**Step 2: Rewrite `parse_verses`** (`warhead_type.rs:197-210`) to f64, reusing the slice-1
primitives. Import them at the top: `use crate::rules::ini_value::{atoi_lenient, parse_leading_f32};`
(both are `pub(crate)` in `ini_value.rs:263,289`).
```rust
/// Parse the Verses= value into a fixed-size [f64; 11].
///
/// Per token, branch on '%' presence (gamemd strchr):
/// - has '%': (atoi(token) as f64) * 0.01 — INTEGER-truncating atoi BEFORE x0.01
///   ("50.5%" -> 0.5, "0.5%" -> 0.0, "-50%" -> -0.5). Reuses slice-1 atoi_lenient.
/// - no '%': parse_leading_f32 widened to f64 ("0.505" -> 0.505). Reuses slice-1
///   parse_leading_f32 (the same float path read_double uses for the bare case).
/// Missing trailing tokens default to 1.0 (100%); absent Verses= -> [1.0; 11].
fn parse_verses(raw: &str) -> [f64; 11] {
    let mut out = [1.0_f64; 11];
    for (i, tok) in raw.split(',').enumerate().take(11) {
        let t = tok.trim();
        out[i] = if t.contains('%') {
            atoi_lenient(t) as f64 * 0.01_f64
        } else {
            parse_leading_f32(t) as f64
        };
    }
    out
}
```

**Step 3: Update the parse site** (`warhead_type.rs:115`) and the default. Replace:
```rust
        let verses: Vec<u8> = section.get("Verses").map(parse_verses).unwrap_or_default();
```
with:
```rust
        let verses: [f64; 11] = section.get("Verses").map(parse_verses).unwrap_or([1.0; 11]);
```
(Note the default changes from "empty" to all-100%, matching gamemd D-parse "Verses default 100%
all 11"; the old `Vec<u8>` empty-default was itself a small drift.)

**Step 4: Update downstream readers** so they compile. `combat_aoe.rs:196,304` currently:
`let verses_pct: u8 = warhead.verses.get(idx).copied().unwrap_or(100);` — change to read the f64 and
convert to the old percent the existing `aoe_damage_at_distance` still expects (kept until P7):
```rust
        let verses_pct: u8 = (warhead.verses[idx] * 100.0).round().clamp(0.0, 200.0) as u8;
```
`combat_weapon.rs:341` similarly derives `verses_pct` from `warhead.verses[idx]` (the `verses_gate`
percent — see Task 10).

**Step 5: Update + add tests** in `warhead_type.rs`. Replace the u8 assertions
(`:253-257`, `:289-296`, `:308-310`) and add the precision test:
```rust
    // in test_parse_warhead:
    assert_eq!(wh.verses.len(), 11);
    assert!((wh.verses[0] - 1.00).abs() < 1e-9); // none: 100%
    assert!((wh.verses[2] - 0.90).abs() < 1e-9); // plate: 90%
    assert!((wh.verses[6] - 0.60).abs() < 1e-9); // wood: 60%
    assert!((wh.verses[10] - 0.0).abs() < 1e-9); // special_2: 0%
```
```rust
    #[test]
    fn fractional_verses_preserved() {
        // % branch: integer atoi BEFORE x0.01 => "50.5%" -> 50*0.01 = 0.5 (NOT
        // 0.505). Bare branch: "0.505" -> 0.505 full precision.
        let ini = IniFile::from_str(
            "[Pct]\nVerses=50.5%,1.5%,0.5%\n[Bare]\nVerses=0.505,0.015,0.005\n",
        );
        let pct = parse_verses(ini.section("Pct").unwrap().get("Verses").unwrap());
        let bare = parse_verses(ini.section("Bare").unwrap().get("Verses").unwrap());
        assert!((pct[0] - 0.5).abs() < 1e-9);   // 50.5% -> atoi(50)*0.01
        assert!((pct[1] - 0.01).abs() < 1e-9);  // 1.5%  -> atoi(1)*0.01
        assert!((pct[2] - 0.0).abs() < 1e-9);   // 0.5%  -> atoi(0)*0.01
        assert!((bare[0] - 0.505).abs() < 1e-9);
        assert!((bare[2] - 0.005).abs() < 1e-9);
    }
```
Also fix `test_warhead_defaults` (`:266`): `assert_eq!(wh.verses, [1.0; 11]);` and the
`test_parse_verses_without_percent` test to f64 (`100,50,25` → `[1.0, 0.5, 0.25, 1.0, ...]`).

**Step 6: Verify.** Run: `cargo test -p vera20k warhead_type` then `cargo check -p vera20k`.
Expected: PASS / compiles. Read the `test result:` line.

**Step 7: Commit.** `rules/warhead: Verses Vec<u8> -> [f64;11] via slice-1 primitives (P5)`

**Rollback note:** field-type change touches all readers; revert by restoring `verses: Vec<u8>` +
the old `parse_verses` and the u8 reader sites. Data-only (no apply site flipped yet) → not
hash-relevant until P7, BUT the default-empty→all-100% change at Step 3 DOES alter parse output for
warheads with no `Verses=` key; if any such stock warhead exists and is read by the still-live old
formula, this is a small hash flip. Verify with `cargo test -p vera20k combat_aoe` before commit; if
it flips, fold this default change into P7's hash bump instead and keep the empty-default here.

**Depends on:** none (rules layer), but ordered after the kernel so the kernel test data already
uses f64 tables.

---

### Task 10: `SelectedWeapon.verses_f64` + keep `verses_gate` (P5 cont., additive)

**Why:** Carry the f64 Verses to the direct-hit/selection sites so P7 can call the kernel with full
precision; keep the existing `verses_gate` (0/1/>1) by deriving the integer percent from the f64.

**Files:**
- Modify: `src/sim/combat/combat_weapon.rs:67` (add field), `:341-351` (populate), and `verses_gate`
  call site `mod.rs:1364`.

**Pattern:** additive struct field; `verses_gate` semantics unchanged (derive percent).

**Step 1: Add the field** at `combat_weapon.rs:60-70`, alongside `verses_pct`:
```rust
    /// Full-precision Verses for the target armor (gamemd double). Used by the
    /// damage kernel at the fire site. `verses_pct` is the derived integer
    /// percent kept for verses_gate / the legacy formula until P7 cutover.
    pub verses_f64: f64,
```

**Step 2: Populate it** at `combat_weapon.rs:341-351`. The selection currently does
`let verses_pct: u8 = warhead.verses.get(idx).copied().unwrap_or(100);` — after Task 9 that field is
`[f64;11]`, so:
```rust
    let verses_f64: f64 = warhead.verses[idx];
    let verses_pct: u8 = (verses_f64 * 100.0).round().clamp(0.0, 200.0) as u8;
```
and add `verses_f64,` to the `SelectedWeapon { … }` literal.

**Step 3: No behavior change** — `verses_gate(selected.verses_pct)` at `mod.rs:1364` keeps reading
the derived percent. Verify the gate thresholds still fire (0 → Blocked, 1 → Suppressed): a Verses of
`0.005` (0.5%) derives `verses_pct = 1` (rounds to 1%) → Suppressed, matching gamemd's "≤1%
suppresses" — note this as a parity-critical edge in Step 5.

**Step 4: Verify.** Run: `cargo test -p vera20k combat_weapon` then `cargo check -p vera20k`.
Expected: PASS / compiles.

**Step 5: Add a test** in `combat_weapon.rs` tests pinning the derive:
```rust
    #[test]
    fn verses_pct_derives_from_f64_and_gate_thresholds() {
        // 0.005 (0.5%) rounds to verses_pct 1 -> Suppressed (gamemd <=1%).
        assert_eq!((0.005_f64 * 100.0).round() as u8, 1);
        assert_eq!(verses_gate(1), VersesGate::Suppressed);
        assert_eq!(verses_gate(0), VersesGate::Blocked);
    }
```

**Step 6: Commit.** `sim/combat: SelectedWeapon.verses_f64 + derive verses_pct for gate (P5)`

**Depends on:** Task 9.

---

### Task 11: P7 authoritative cutover — wire the service into all three apply sites + bump version (P7, hash)

**Why:** Make the service the only damage path: replace the inline direct-hit formula and the three
coarse `is_invulnerable`/`saturating_sub` apply sites with the kernel/`receive_damage`, drop the
shadow asserts, and request the integration-owned `SNAPSHOT_VERSION` bump + parity re-baseline.

**Files:**
- Modify: `src/sim/combat/combat_aoe.rs:198-211, 306-319, 327-350` (replace
  `aoe_damage_at_distance` with `kernel::apply_warhead_damage`; delete the old fn).
- Modify: `src/sim/combat/mod.rs:2201-2216` (direct-hit: call the kernel),
  `mod.rs:1623-1651` (Phase-4 apply → `receive_damage`),
  `mod.rs:1064-1085` (death-explosion AoE apply → `receive_damage`). **All three** per C-2.
- Modify: `src/sim/snapshot.rs:24` (`SNAPSHOT_VERSION`) + `:375` (`assert_eq!`) — **integration-owned;
  take the next free integer at merge, do not hard-code a number in this plan.**
- Modify: `src/sim/world/damage_parity_harness_tests.rs` (re-baseline final hash — Task 12).

**Pattern:** cutover; mirrors the Mission/Radio "drop shadow, flip authority, bump version" commit.

**Step 1: AoE kernel cutover.** In `combat_aoe.rs`, both `apply_aoe_damage` (the non-occupancy
fallback loop) and `push_entity_aoe_damage` currently call `aoe_damage_at_distance(...)`. Replace
each with the kernel, passing the warhead's f64 inputs and the per-target armor + distance. The
distance unit MUST be resolved per Q1 before this step (the kernel expects 128-scaled leptons; the
collection still computes 256-scaled `dist_leptons`). Concretely, at `combat_aoe.rs:198-204`:
```rust
        let idx: usize = armor_index(armor_str);
        // Q1: convert the 256-scaled collection distance to the kernel's lepton
        // unit before calling the kernel. (RESOLVE Q1: confirm Apply_area_damage
        // feeds 128-scaled or 256-scaled distance; set the conversion factor
        // accordingly. Until resolved this cutover is BLOCKED.)
        let kernel_dist: i32 = /* Q1 conversion of dist_leptons */ todo_q1(dist_leptons);
        let raw: i32 = crate::sim::combat::damage::kernel::apply_warhead_damage(
            base_damage,
            warhead.cell_spread.to_num::<f64>(),
            warhead.percent_at_max as f64 / 100.0,
            &warhead.verses,
            crate::sim::combat::damage::ArmorClass(idx as u8),
            kernel_dist,
            rules.scenario_no_damage(), // ScenarioFlags & 0x20 accessor (add if absent — Q-scenario)
            rules.general.max_damage,   // Rules MaxDamage (default 1000 — verify field exists)
        );
        let dmg: u16 = raw.clamp(0, u16::MAX as i32) as u16;
```
Then delete `aoe_damage_at_distance` (R-1/R-3) and its now-stale unit tests
(`test_aoe_damage_*`), or re-point them at the kernel.

**Step 2: Direct-hit cutover** (`mod.rs:2201-2216`). Replace the inline
`base_damage * selected.verses_pct as i32 / 100` (R-3) with a direct-hit kernel call (distance 0,
CellSpread from the warhead, PAM flat at the impact):
```rust
        let raw_damage: i32 = crate::sim::combat::damage::kernel::apply_warhead_damage(
            base_damage,
            warhead.cell_spread.to_num::<f64>(),
            warhead.percent_at_max as f64 / 100.0,
            &warhead.verses,
            crate::sim::combat::damage::ArmorClass(armor_index(target_armor_str) as u8),
            0,
            rules.scenario_no_damage(),
            rules.general.max_damage,
        );
        let actual_damage: u16 = raw_damage.clamp(0, u16::MAX as i32) as u16;
```
(`target_armor_str` is the resolved target armor at this site — wire from the existing target lookup.)

**Step 3: Phase-4 + death-explosion apply cutover** (`mod.rs:1623-1651` and `mod.rs:1064-1085`).
Replace the `is_invulnerable` + `saturating_sub` blocks (R-4/R-5) with `receive_damage`, building
`TargetDamageView` + `ImmunityInputs` + `CombatMods` from the target/attacker state. Map
`DamageGate::Nullified` → skip (set `last_attacker_id`), `MindControlled` → 0 HP + marker, `Pass`
→ subtract `hp_delta`; use `DamageState` for the condition/fear transitions (replacing the ad-hoc
`refresh_building_damage_state_gate`/`apply_fear_from_damage` calls per Q5 reconciliation).
**[Plan-review C-PR3: `classify()` produces the integer-`Strength>>1` Yellow + ratio-Red STATE,
NOT the `Rules+0x1700` ConditionYellow RATIO that gates the smoke/fear particle. Do NOT drop the
two existing calls silently — either keep them (ratio path) alongside the new `DamageState`, or
explicitly re-derive the particle gate from the outcome, else smoke/fear stops firing. Q5 is
load-bearing HERE, not optional.]** Both
sites get the identical replacement so P7's CI grep-gate finds no remaining `is_invulnerable`
+ bare `saturating_sub` damage apply.

**Step 4: Drop the shadow asserts** added in Task 3 and Task 8 (the production behavior now IS the
service).

**Step 5: Request the version bump (integration-owned).** Update `snapshot.rs:24`
`SNAPSHOT_VERSION` to the next free integer assigned by whoever sequences `snapshot.rs` at merge,
and update `snapshot.rs:375` `assert_eq!(super::SNAPSHOT_VERSION, N)` to match — in the SAME commit
as the parity re-baseline (Task 12). Do NOT pick a literal here.

**Step 6: Add the CI grep-gate test** (`mod.rs` or a small integration test) proving no retired
formula remains:
```rust
#[test]
fn damage_path_is_authoritative() {
    // Source-level guard: the retired apply formulas are gone. Reads this crate's
    // combat sources and asserts none contains the retired call shapes.
    let aoe = include_str!("combat_aoe.rs");
    let m = include_str!("mod.rs");
    assert!(!aoe.contains("fn aoe_damage_at_distance"), "aoe_damage_at_distance not retired");
    assert!(!m.contains("* selected.verses_pct as i32 / 100"), "inline direct-hit formula remains");
    // is_invulnerable + saturating_sub damage-apply must be gone from all 3 sites.
    assert!(!m.contains(".saturating_sub(aoe_dmg)"), "death-explosion bare subtract remains");
    assert!(!m.contains(".saturating_sub(*damage)"), "Phase-4 bare subtract remains");
}
```

**Step 7: Verify.** Run: `cargo test -p vera20k combat` then `cargo test -p vera20k damage_path_is_authoritative`
then the full `cargo test -p vera20k`. Expected: PASS. Read every `test result:` line.

**Step 8: Commit.** `sim/combat: cutover all 3 apply sites to damage service + version bump (P7, hash)`

**Rollback note (hash-flipping):** this is the authoritative flip. To revert: restore
`aoe_damage_at_distance` + the inline direct-hit formula + the `is_invulnerable`/`saturating_sub`
blocks at `1064-1085`/`1623-1651`/`2201-2216`, revert `SNAPSHOT_VERSION` + its assert, and restore
the prior parity baseline. Because the version bump is shared, coordinate the revert with the
integration owner so the integer is released back to the pool.

**Depends on:** Tasks 2, 4, 6, 7, 9, 10; **Q1 (distance unit) and a `scenario_no_damage()` /
`rules.general.max_damage` field availability check (Q2/Q4)** must close first.

---

### Task 12: P8 global damage-parity replay harness (P8)

**Why:** Whole-pipeline regression tripwire: a deterministic scripted skirmish (mixed warheads ×
armors × distances × vet/country mods) replayed against a committed per-tick/final state-hash, so any
future damage change that desyncs is caught.

**Files:**
- Create: `src/sim/world/damage_parity_harness_tests.rs` (mirrors
  `src/sim/world/global_parity_harness_tests.rs`: seed, tick count, tick-ms, committed final hash).
- Modify: `src/sim/world/mod.rs` (declare `#[cfg(test)] mod damage_parity_harness_tests;` alongside
  the existing `global_parity_harness_tests`). **[Plan-review C-PR4: mirror the sibling form at
  `world/mod.rs:2632-2634` — `#[cfg(test)] #[path = "damage_parity_harness_tests.rs"] mod
  damage_parity_harness_tests;`. Compiles either way; cosmetic consistency.]**

**Pattern:** copy `global_parity_harness_tests.rs` structure verbatim; swap the rules INI for a
damage-stressing matrix (varied `Verses`, `CellSpread`/`PercentAtMax`, vet tiers via scripted
promotion, country FirePower/Armor mods once Q2 plumbs them).

**Step 1: Write the harness** using the `ReplayLog`/`ReplayRunner::run` path. Use a fixed
`HARNESS_SEED`, `HARNESS_TICKS`, and a `DAMAGE_HARNESS_FINAL_HASH` captured from the first green
authoritative run (post-Task 11):
```rust
//! P8 — damage-pipeline parity harness. Deterministic scripted skirmish that
//! stresses the damage service (mixed warheads x armors x distances x vet/country
//! mods), replayed through ReplayRunner::run with a committed final state-hash.
//! The damage-side desync tripwire (mirrors global_parity_harness_tests).

const DAMAGE_HARNESS_SEED: u64 = 0xDA_4A_6E_0001;
const DAMAGE_HARNESS_TICKS: u64 = 600;
const DAMAGE_HARNESS_TICK_MS: u32 = 67;
/// Captured from the first green run AFTER the P7 authoritative cutover.
/// Re-baseline only on a documented behavior-bearing damage change.
const DAMAGE_HARNESS_FINAL_HASH: u64 = 0; // FILL from first green run

#[test]
fn damage_parity_replay() {
    // (mirror global_parity_harness_tests::… : build rules with a damage matrix,
    //  record a ReplayLog, re-run via ReplayRunner::run, assert every tick's
    //  replayed hash == recorded hash AND final == DAMAGE_HARNESS_FINAL_HASH.)
}
```

**Step 2: Capture the baseline.** Run the test once with `DAMAGE_HARNESS_FINAL_HASH = 0`, read the
asserted-actual hash from the failure, paste it in, re-run to green.

**Step 3: Verify.** Run: `cargo test -p vera20k damage_parity_replay -- --nocapture`
Expected: PASS after the baseline fill. Read the `test result:` line.

**Step 4: Commit.** `sim/world: P8 damage-parity replay harness + baseline`

**Rollback note:** the baseline hash is committed alongside the P7 version bump. If P7 reverts,
delete this harness or reset the baseline to the pre-cutover value.

**Depends on:** Task 11.

---

### Task 13: Verification against gamemd.exe (final parity check)

**Why:** Confirm the shipped service matches the original engine's observable damage, beyond the unit
tests.

**Verify:**
- **Armor-vs-warhead matrix:** for a fixed warhead (e.g. `AP`) and base damage, compute the kernel
  output for all 11 armor classes and confirm each equals gamemd's `ApplyWarheadDamage` output for
  the same `(damage, Verses[armor], CellSpread, PAM, distance)`. Method: `/fidelity-check` against
  the kernel decompile (`docs/research` D1–D6), or an in-game side-by-side of a known shot.
- **Country-armor divide regression:** a defender with a country ArmorUnitsMult ≠ 1.0 takes
  `ftol(incoming / mult)` (tougher = less damage). Confirm the DIVIDE direction in-game (Soviet vs
  Allied country bonus) matches gamemd.
- **Full-order ftol boundary:** the `99 dmg / 0.5 verses / 0.5 falloff` case yields the double-ftol
  result (24), not the single-multiply 25 — confirm against the kernel decompile.
- **Expected result from original:** identical integer HP delta to the last digit for every sampled
  input; identical Yellow/Red/Dead transition tick.

**Step 1: Run** `/fidelity-check` on the damage kernel + receiver pipeline against the cited
gate docs, OR an in-game side-by-side for one Grizzly-vs-Rhino and one V3-splash-vs-GI shot.

**Step 2: Record** any divergence as a follow-up; do not silently accept "close enough."

**Depends on:** Task 11 (authoritative path live).

---

## Sources & References

- **Design doc:** `docs/plans/2026-06-04-damage-substrate-service-design.md` (incl. Design-review
  corrections C-1..C-5).
- **Study:** `docs/research/DAMAGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (Pass-2 bit-verified;
  §5 D1–D24, §6 boundary, §7 retire table, §8 slices, §9 ledger).
- **Gate docs:** `GATE_DAMAGE_VERSES_F64_RESOLUTION_GHIDRA_REPORT.md` (D1: 128 leptons,
  `read_memory 0x007e2224=0x43800000`; Verses parse), `GATE_DAMAGE_MAXDAMAGE_CLAMP_RESOLUTION_GHIDRA_REPORT.md`
  (D2: MaxDamage 1000, `0x3E8`), `GATE_DAMAGE_COUNTRY_ARMOR_ORDER_RESOLUTION_GHIDRA_REPORT.md`
  (D3: divide + order — **note its 256/10000 are STALE, superseded by D1/D2**).
- **gamemd addresses (kept here, not in Rust):** `ApplyWarheadDamage 0x00489180` (ftol
  0x004891e4/0x00489220/0x00489244; cap [Rules+0x16C8]); `TechnoClass::ReceiveDamage 0x00701900`
  (FDIVR 0x0070195d; VeteranArmor FDIV [Rules+0x688] 0x007019cb; immunity gates 0x00701a3b–0x00701dc9);
  `Fire_At 0x006fdd50`; `ObjectClass::ReceiveDamage 0x005f5390`; `GetArmorMultForType 0x0050bd30`;
  `Math__ftol 0x007c5f00`; constants `0x007e2224=128.0`, MaxDamage `0x3E8`.
- **INI keys:** `[Warhead] Verses=`, `CellSpread=`, `PercentAtMax=`, `Wall=`, `Radiation=`,
  `Poison=`, `Psychedelic=`/`MindControl=`, `AffectsAllies=`; `[General]/[CombatDamage] MaxDamage`,
  `VeteranArmor`, `VeteranCombat`, `ConditionRed`.
- **Related code:** `src/rules/ini_value.rs` (`read_double`/`atoi_lenient:263`/`parse_leading_f32:289`),
  `src/util/fixed_math.rs:23,71` (`SimFixed=I16F16`, `sim_to_i32` floors toward −∞),
  `src/sim/combat/combat_aoe.rs`, `src/sim/combat/combat_weapon.rs`, `src/sim/combat/mod.rs`
  (`armor_index:108`, prone:152, apply sites 1066/1072, 1625/1634, 2204/2206),
  `src/sim/snapshot.rs:24,375`, `src/sim/world/global_parity_harness_tests.rs`.
- **Prior commits:** `b452c537` (Slice 8 global parity harness — P8 pattern), `d64ad257`
  (6 closed-gate verified facts), `f0158074` (INI accessor slice-1).

---

## Plan-review corrections (2026-06-04)

Pre-execution review per `/review-plan`. Every edit-anchor in the plan was checked against
current `src/`; every load-bearing binary constant against the cited gate docs. **Verdict:
GREEN (execute as-is).** All file:line anchors, struct fields, signatures, and the two
corrected constants (128 leptons, MaxDamage 1000) are accurate as written. The items below
are clarifications/caveats, not blockers — apply during implementation.

- **C-PR1 (Key Technical Decision #2 — mischaracterized `sim_to_i32`).** The decision says
  `sim_to_i32` "floors toward −∞." That is the wording in `ini_value.rs:204` (`read_range`
  doc) — but `fixed_math.rs:69-73` documents `sim_to_i32` as "rounds toward zero," and the
  `fixed` crate's `to_num::<i32>()` truncates toward zero. The two project doc comments
  disagree (a pre-existing codebase inconsistency, not introduced by this plan). **The
  plan's conclusion is still correct:** use `f64 as i32` for the kernel's `ftol`, because the
  kernel operates on `f64` (not `SimFixed`), and `f64 as i32` is the unambiguous
  truncate-toward-zero gamemd analog. Do NOT "fix" `sim_to_i32` on the false premise that it
  floors — verify the `fixed` crate's actual rounding first if anyone touches it. (Verified:
  read `src/util/fixed_math.rs:69-73`, `src/rules/ini_value.rs:199,204-212`.)

- **C-PR2 (Task 2 `kernel_double_ftol_order` comment is imprecise; assertion is correct).**
  The test asserts `24`, which is correct under the plan's own kernel
  (`ftol(99*0.5)=ftol(49.5)=49`, then `ftol(49*0.5)=ftol(24.5)=24`). The comment's claim that
  "the single-multiply path … gives 25" is wrong arithmetic: `99*0.5*0.5=24.75 → ftol = 24`,
  also 24 — so this input does NOT actually separate single-vs-double ftol. The test still
  validly exercises both interior ftols, but if a true 1-off divergence demonstrator is
  wanted, pick an input where the first ftol drops a fraction that changes the second product
  (e.g. base where `ftol(lerp)` differs from `lerp` by ≥1 LSB before the Verses multiply).
  Non-blocking: keep `assert_eq!(d, 24)`; fix only the comment.

- **C-PR3 (Task 11 Step 3 — Q5 fear/smoke reconciliation under-specified).** Both apply
  sites today call `target.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000)`
  and `infantry::apply_fear_from_damage(obj, target, dmg, …, condition_red_x1000,
  condition_yellow_x1000)` (`mod.rs:1073/1080` and `:1635/1642`). The new `classify()` returns
  `DamageState` using **integer `Strength>>1`** for Yellow and the `condition_red_ratio` for
  Red — it does NOT compute the *ratio* Yellow that gates the smoke/fear particle. Per D18 +
  study §5.3, the `Rules+0x1700` ConditionYellow ratio is a separate concern (particle gate),
  distinct from the integer Yellow state crossing. Task 11 must therefore EITHER keep calling
  `refresh_building_damage_state_gate`/`apply_fear_from_damage` (ratio path) alongside the new
  `DamageState`, OR explicitly re-derive both from the outcome — do not drop the two calls
  silently, or smoke/fear stops firing. The plan already defers this as Q5; this note pins
  that Q5 is load-bearing AT the P7 cutover, not optional. (Verified:
  `src/sim/combat/mod.rs:1073,1080,1635,1642`; `GeneralRules.condition_{yellow,red}_x1000` are
  `i64` ×1000 at `ruleset.rs:257,263`.)

- **C-PR4 (Task 12 module decl — add the `#[path]` attribute).** The plan writes
  `#[cfg(test)] mod damage_parity_harness_tests;`. The sibling decl uses
  `#[cfg(test)] #[path = "global_parity_harness_tests.rs"] mod …` (`world/mod.rs:2632-2634`).
  `mod x;` resolves to `x.rs` in the same dir so it compiles either way, but mirror the
  existing `#[path]` form for consistency with the surrounding decls. Cosmetic.

**Confirmed (would-be findings that checked out, recorded for confidence):**
- Q1 is genuinely open: the live AoE path uses **256** leptons/cell
  (`combat_aoe.rs:96` `cell_spread.to_num::<i64>() * 256`; `:185,:293` `dist_leptons / 256`),
  while the kernel uses **128**. The `#[ignore]` on `kernel_matches_worked_example` and the
  BLOCKED tag on the Task 11 AoE cutover are both warranted — do not un-gate before Q1 closes.
- Q2/Q4 fields are genuinely absent: `grep` of `src/rules` finds **no** `max_damage`,
  `scenario_no_damage`, `firepower`/`FirePower`, `armor_mult`, `veteran_armor`/`veteran_combat`
  fields. The plan's "add if absent / verify field exists" hidden-work flags are accurate;
  P6/P7 must add `[CombatDamage] MaxDamage` parse (default 1000 per gate D2), a
  `scenario_no_damage()` accessor, and the entire `CombatMods` source plumbing.
- `MinDamage` (`Rules+0x16C4`) is verified DEAD (read by nothing — MaxDamage gate §, lines
  124-131); the plan correctly does not implement it.
- SNAPSHOT_VERSION is correctly NOT self-bumped: `snapshot.rs:375`
  (`snapshot_version_is_17_in_shadow_phase`) docstring shows ANOTHER in-flight program already
  claims the `17→18` bump at its own authority flip — so P7 taking "the next free integer at
  merge" is the right discipline; do not write a literal.
