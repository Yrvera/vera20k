---
title: Factory/House Production+Economy Substrate — P3 (per-step charge / SetRate) Design Spec
date: 2026-06-04
status: design (winner = D2 substrate-fit, grafted with D3 structural-no-hash + D1 fidelity ledger; P3 oracle/hash-neutral scope is the only implement-now boundary)
scope: P3 ONLY — `Factory::advance_one_step` (per-step charge / rollback / completion settlement, C2/C3/C4/C12/C15)
       and `Factory::set_rate` (C5). HASH-NEUTRAL: charges an ORACLE (clone) economy, never the hashed wallet.
       The legacy upfront-charge stays AUTHORITATIVE. world_hash.rs UNTOUCHED; SNAPSHOT_VERSION STAYS 17.
       OUT OF SCOPE (seams only): authority flip + 17->18 (P5), cancel/refund (P4), prereq revalidation (P6),
       purifier/IncomeMult (P7), delivery/exit (P5+), full GetBuildStepTime pipeline (low-power C10 / multi-factory C11).
source: docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md (v2-verified; C2-C5, C12, C15; §6.2; §6.3 FIT-a; §8 P3; §9.1)
        + the committed P1/P2 code (factory.rs, economy.rs, world/mod.rs, techno_ai.rs, production_shadow_tests.rs).
verification: every current-code claim is quoted file:line from a live read this session. P0 charge math is
              VERIFIED-LIVE v2 (study §9.1: GetBuildStepTime 0x006F47A0 R1 — NO x0.9; FactoryClass::AI 0x004C9B20 V1).
              Ghidra read-only; cargo NOT run (separate build pass).
rule: Rust-native structure, gamemd-native semantics. sim/ never depends on render/ui/sidebar/audio/net.
---

# Factory/House Substrate — P3 Design Spec (per-step charge + SetRate, hash-neutral oracle)

## 0. TL;DR

Three competing P3 designs were scored against the v2-verified study and the committed P1/P2 code.
**The winner is the substrate-fit-first design (D2)**, grafted with the test-first design's (D3)
**structural** no-hash proof (the method is dead-code until P5; the oracle is a clone, never the wallet)
and the parity-fidelity design's (D1) decisive **SetRate scope** finding + complete tiny-detail ledger.

The result: `Factory::advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome` and
`Factory::set_rate(&mut self, build_step_time: i32)`, both **pure methods on `Factory`** with the
exact gamemd integer charge math. P3 charges only a **CLONED/throwaway oracle economy** — never
`HouseState.economy`, never the legacy `credits`. The legacy upfront-charge stays authoritative; the
new method is `#[allow(dead_code)]` (no `advance_tick` call site) until P5. Because `Factory`/`Economy`
carry **no serde derive** and no new call enters the tick, `state_hash()` is bit-identical by
construction — the strongest available no-hash proof (D3), not a leak-freedom argument.

**The three P3 DECISIONS (rationale in §3-§5):**
- **(a) E1 cost-source:** `rebuild_shadow` gains `&RuleSet` and seeds a **cost-based** oracle balance
  (`balance = original_balance = rules.object(type).cost`, remaining derived by replaying the exact
  per-step charge ladder). Chosen over D3's oracle-cost-in-tests-only so the conservation shadow-assert
  runs against EVERY live tick's real shadow, not just synthetic fixtures.
- **(b) SetRate scope:** `set_rate(build_step_time: i32)` applies only the **verified ÷54 + clamp[1,255]
  + rate-0-no-object sentinel** on a *given* total. It does NOT compute the full `GetBuildStepTime`
  pipeline (low-power C10 / multi-factory C11 — P-later) and it MUST NOT derive the total from the
  legacy `build_time_base_frames`, which bakes in a verified-REFUTED ×0.9 (`production_tech.rs:334`,
  `* 9 / 10000`). This is D1's decisive parity call.
- **(c) Oracle wiring (FIT-a, hash-neutral):** the `EntityCategory::Structure` arm stays a literal
  no-op. The oracle step runs as a debug-only probe **beside** the existing `factory_shell_trace`
  (techno_ai.rs:252), walking live Structures in LogicVector order and stepping a CLONE of each
  factory against a CLONE of the owner's economy — `&Simulation`-read-only, so the existing
  `techno_ai_shell_is_passthrough_no_hash_change` (techno_ai.rs:331) stays valid verbatim.

---

## 1. Scoring the three competing designs

Each lens scored 1-5 (5 best) on five axes, with a one-line justification per cell.

| Axis | D1 (parity-fidelity) | D2 (substrate-fit) | D3 (test-first / risk) |
|---|---|---|---|
| **Parity-fidelity** | **5** — exact charge ladder + the decisive SetRate-source finding (legacy ×0.9 is wrong); replays the exact balance ladder in the shadow so conservation is meaningful on live factories | **5** — same charge math + same SetRate finding; frame-choice (increment-first) mirrors gamemd's PV++ then `54-Value` ordering verbatim | **4** — charge math correct, but the BEFORE-increment frame is a clean re-derivation, not the gamemd order; defers the cost-based shadow so conservation is fixture-only |
| **Substrate-program fit** | **4** — FIT-(a) honored via a Structure-arm shadow trace, but layers the conservation probe across both the arm and rebuild; more surface in the spine | **5** — extends the existing `factory_shell_trace` probe (the P2 "proof beside the no-op arm"); threads `rules` through the already-`Option<&RuleSet>` `refresh_production_shadow`; reuses `iter_insertion_ordered` | **3** — explicitly adds NO driver and NO `&RuleSet`; cleanest blast radius but deliberately does NOT advance FIT-(a) past P2 (seam only), so the substrate spine gains nothing this slice |
| **Testability / determinism** | **5** — full §8-P3 matrix + boundary sub-cases (exactly-affordable, cost-0); live conservation assert each tick | **5** — same matrix; conservation assert runs on every live shadow tick (the strongest guard); BTreeMap/integer throughout | **4** — full matrix as pure unit tests (lowest-friction), but the conservation assert is test-only, never on a live tick |
| **Risk / blast-radius (HASH-NEUTRAL)** | **3** — largest surface: cost-based rebuild + Structure-arm probe + conservation assert; more to keep hash-neutral | **4** — moderate: one probe beside an existing one + a `rules` thread; arm stays literal no-op so the passthrough test is untouched | **5** — smallest: factory.rs-only, no tick call site, no rebuild change; structural no-hash proof ("no call exists") is trivially auditable |
| **Buildability (lands green fast)** | **3** — most plumbing (rules thread + ladder replay + probe) | **4** — reuses P2 scaffolding; the `rules` thread is a one-line change on an existing `Option` | **5** — two method bodies + tests; fewest moving parts |
| **TOTAL** | **20** | **23** | **21** |

**Winner: D2 (substrate-fit-first), total 23.** It best honors the master directive — "slot into the
substrate program, do not invent a parallel architecture" — by extending the *existing* P2
`factory_shell_trace` probe rather than adding a parallel driver, and it makes the conservation
invariant checkable on every live tick (the strongest parity guard), which D3's fixture-only approach
cannot. Its charge math and SetRate finding are identical to D1's at lower spine cost.

### 1.1 What was grafted from the runners-up

- **From D3 (test-first / risk):** the **structural** no-hash proof is adopted as the primary guarantee.
  The no-hash argument rests on three independently-sufficient structural facts (§5.1), each auditable
  from the diff: (1) no serde derive on `Factory`/`Economy`/`FactoryRegistry`; (2) the new method has
  NO authoritative `advance_tick` call site — it is `#[allow(dead_code)]` until P5; (3) the oracle is
  always a clone, never the wallet. We keep D2's debug-only probe (so FIT-(a) advances and the
  conservation assert is live), but the *release-path* hash neutrality is D3's "no call exists" —
  stronger than "the call provably touches no hashed state." We also adopt D3's full §8-P3 matrix
  mapping (§7) and its cost-1 conservation corner analysis (the load-bearing tiny-cost case).
- **From D1 (parity-fidelity):** the decisive **SetRate-source** finding — the legacy
  `build_time_base_frames` is the WRONG source because it carries a verified-REFUTED ×0.9
  (`production_tech.rs:334` `cost * speed_x1000 * 9 / 10000`); SetRate must take the build-step total
  as an input, not derive it from the legacy frames balance. We also adopt D1's complete tiny-detail
  ledger (§6) and its decision to replay the EXACT per-step charge ladder when seeding the cost-based
  shadow balance (so the conservation assert is not spuriously off by accumulated rounding).

### 1.2 Why D2 over D3 specifically (the E1 fork)

D3's "oracle takes cost only in tests, no `&RuleSet` on `rebuild_shadow`" is the smallest blast radius,
but it leaves the live shadow `balance` **frames-based** (the P2 placeholder, `factory.rs:245`), so the
per-tick conservation shadow-assert can only run against synthetic test fixtures, never against the
real per-house factories the substrate is supposed to prove out. The substrate program's discipline is
"new authority added SHADOWED first; divergence SURFACED on every tick." A cost-based live shadow
(D2/D1) is what makes that discipline meaningful here; D3's frames-based shadow cannot carry it. The
cost-based rebuild is a one-line `rules` thread on an already-`Option<&RuleSet>` function
(`refresh_production_shadow`, world/mod.rs:1006), so the cost is small and the parity guard is real.

---

## 2. The `advance_one_step` algorithm (C2/C3/C4/C12/C15)

### 2.1 Signature & charge target (the hash-neutrality core)

```rust
impl Factory {
    /// Advance one step against an ORACLE economy (a clone / throwaway), NOT the
    /// hashed wallet. Charges balance/(54-progress) per step; the LAST step charges
    /// the entire remaining balance ONCE (div-by-zero guard at stepsLeft==0); on
    /// shortfall sets on_hold + rewinds progress by 1 (net-zero, no spend); on
    /// reaching 54 suspends with the object STILL attached and balance zeroed.
    /// The legacy upfront-charge stays authoritative; this is shadow-only until P5.
    fn advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome
}
```

The `&mut Economy` is **always an oracle** in P3 — never the real `HouseState.economy`. The method body
is hash-agnostic; neutrality is enforced at the **call site** (§4): P3 only ever passes a `clone()` of
an economy or a test-local one. The `&mut Economy` param (not `&mut self` alone) is kept deliberately so
the method shape is exactly what P5 makes authoritative — P5 flips *who* is passed, not the algorithm
(study §6.2 line 612: `advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome`; study §8 P3
line 724: "CLONED/throwaway economy (oracle) — does NOT call the real `economy.spend` on the hashed
wallet").

### 2.2 The exact step algorithm (integer-only; `progress: u16`, `balance/original_balance: i32`)

Frame choice: **increment progress FIRST, then compute `54 - Value`** — the verified gamemd order
(study C3/F4: `Production_Value` advances, then the cost reads `Balance/(54-Value)`, hitting the
divide-by-zero guard at the final step). This reproduces the exact charge sequence including the
single-remainder-on-the-last-step behavior. (D3's BEFORE-increment frame yields the same observable
output but re-derives a different control flow; we mirror gamemd's order for fidelity, per the winner.)

```
fn advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome:

  // (G0) ARMED GATE — not stepping this call -> Idle.
  //   No object, OR suspended (complete-held / paused), OR on_hold latched, OR manual.
  if self.object.is_none() || self.suspended || self.on_hold || self.manual:
      return StepOutcome::Idle
  // (G1) already at completion (defensive; a settled factory is suspended so G0 catches it).
  if self.progress >= PRODUCTION_STEPS:           // 54
      return StepOutcome::Idle

  // --- take one tentative step (C2: step = 1 per timer expiry) ---
  self.progress += 1;                              // Production_Value advances first
  let steps_left = PRODUCTION_STEPS - self.progress;   // 54 - NEW Value

  // (C3) per-step charge, signed-truncate toward zero (= floor for non-negative balance):
  //   normal step:   charge = balance / (54 - Value)
  //   last step (steps_left == 0): SKIP the IDIV (div-by-0 guard) and charge the
  //                                entire remaining balance, ONCE.
  let charge = if steps_left == 0 {
      self.balance                                 // whole remainder (may be 0)
  } else {
      self.balance / (steps_left as i32)           // i32 / : truncate toward zero
  };

  // (C4) affordability — PRE-CHECK (no spend on a stall, keeps oracle spent_credits clean):
  if economy.available() < charge {                // strict < : exactly-affordable PROCEEDS
      self.progress -= 1;                          // REWIND the tentative step (net-zero)
      self.on_hold = true;                         // UI "On Hold"
      return StepOutcome::Stalled                  // nothing spent, balance unchanged
  }

  // pay-as-you-go: spend exactly `charge`, decrement balance by the same.
  self.on_hold = false;                            // a successful step clears a prior hold
  let paid = economy.spend(charge);
  debug_assert_eq!(paid, charge, "afforded charge must be paid in full");
  self.balance -= charge;                          // charge <= balance always (k>=1) -> no underflow

  // (C12) completion settlement on reaching 54:
  if self.progress >= PRODUCTION_STEPS:            // == 54
      // The last-step charge already zeroed balance; gamemd's completion Spend_Money(Balance)
      // runs as Spend_Money(0). DO NOT charge again (C3/C15 double-charge guard).
      debug_assert_eq!(self.balance, 0, "last-step charge must zero the balance");
      self.balance = 0;                            // idempotent; the contract value
      self.suspended = true;                       // complete-but-not-delivered
      self.step_timer = 0;                         // engine zeroes the CDTimer on completion
      // object STAYS Some(..); delivery (P5+) clears it and advances the queue.
      return StepOutcome::Completed

  StepOutcome::Stepped
```

### 2.3 StepOutcome mapping (already declared, factory.rs:98)

- `Idle` — not armed (no object / suspended / on_hold / manual / already 54).
- `Stepped` — a normal step paid, progress < 54.
- `Stalled` — shortfall: rewound, on_hold set, **nothing** spent.
- `Completed` — reached 54, settled (suspended, object attached, balance 0).

**One step per call.** The per-tick `step_timer` countdown gating is a SEPARATE concern (§2.4); the
§8-P3 tests call `advance_one_step` directly 54× with no clock, so the method must be the pure stepper.

### 2.4 Timer gating — scope boundary (DECISION)

**P3 implements the pure stepper only; the `step_timer` countdown is NOT wired into an authoritative
per-tick driver in P3.** Rationale:
- §8-P3's tests (`factory_54_steps_to_complete`, `factory_exact_cost_conservation`) drive
  `advance_one_step` directly 54×, no timer interposed — the contract is "one step per call."
- The per-tick driver (decrement `step_timer`, reload to `step_rate_frames`, call `advance_one_step`
  on expiry) belongs to the Structure-arm authority flip and is exercised as a debug shadow probe
  (§4), at most once per tick per factory, until P5.
- Wiring the timer into authority now would risk touching tick cadence and is unnecessary to prove the
  charge math.

**UNKNOWN/deferred:** the exact `step_timer` reload trigger (on-step vs on-stall vs on-completion) is a
P5 timing detail (F3 says the timer holds `GetBuildStepTime()/54`; the reload point is not pinned in
the contract). Marked deferred, not guessed.

---

## 3. SetRate algorithm + scope (C5) — DECISION (b)

### 3.1 The chosen scope — verified ÷54 + clamp on a GIVEN total (NOT the legacy frames path)

```rust
impl Factory {
    /// Resume + (re)compute the per-step frame rate from a GIVEN build-step total.
    ///   no object  -> step_rate_frames = 0  (sentinel; clamp does NOT apply)
    ///   else        -> step_rate_frames = clamp(build_step_time / 54, 1, 255)
    /// build_step_time is the VERIFIED total (NO x0.9); /54 is signed-truncated.
    fn set_rate(&mut self, build_step_time: i32)
}
```

Algorithm:
```
fn set_rate(&mut self, build_step_time: i32):
    // SetRate resumes a system-suspend (F8); manual user-pause is untouched.
    if !self.manual:
        self.suspended = false;
    // (C5) rate-0-no-object sentinel: (Object ? GetBuildStepTime() : 0) / 54.
    if self.object.is_none():
        self.step_rate_frames = 0;                 // NOT clamped to 1 — the sentinel 0
        return;
    let per_step = build_step_time / (PRODUCTION_STEPS as i32);   // i32/54, truncate toward zero
    let clamped  = per_step.clamp(STEP_RATE_MIN as i32, STEP_RATE_MAX as i32);  // [1, 255]
    self.step_rate_frames = clamped as u16;
```

### 3.2 Why NOT the legacy `build_time_base_frames` (the decisive parity finding, grafted from D1)

`production_tech.rs:317-337` computes `base = trunc(cost * BuildSpeed * 0.9)` — line 334 is
`(cost * speed_x1000 * 9 / 10000)`, i.e. the **×0.9 factor**. The study REFUTES the ×0.9 for
`GetBuildStepTime` in three places: §0 line 120 ("**NO ×0.9** … `0x007e2ac8` is **1.0f**"), §8 P0
line 702 ("the `×0.9` (REFUTED — does not exist)"), and C5/§9.3 (the verified base is
`trunc(HouseBuildTimeBonus × Cost)`, no ×0.9). Feeding the legacy frames total into `÷54` would bake
a verified-refuted drift straight into the rate. A faithful SetRate must NOT inherit it — so SetRate
takes the build-step total as an input and the verified-exact ÷54/clamp/sentinel is what P3 owns.

### 3.3 Where `build_step_time` comes from — the input boundary (DECISION)

**`set_rate(build_step_time: i32)` takes the total as a parameter; P3 does NOT compute the
low-power/multi-factory pipeline.** The full `GetBuildStepTime` =
`BuildSpeed/power(C10)/MultipleFactory(C11)` pipeline (per-iteration MF truncation; continuous
low-power divisor floored to 0.01) is its own large surface, explicitly P-later (study §8 P3 lists only
C2/C3/C4/C5; C10/C11 are later slices). So:
- **P3 production code:** `set_rate(build_step_time)` does the ÷54-clamp-sentinel shape on a *given*
  total — the verified-exact part (the ÷54 magic, clamp, rate-0 sentinel).
- **P3 tests:** `set_rate_total_over_54_truncates_clamps` passes totals {0, 53, 54, 661, 14000}
  directly, asserting {1, 1, 1, 12, 255} **with an object** (0/54→0→clamp 1; 53/54→0→clamp 1; 54→1;
  661→12; 14000→259→255). `set_rate_zero_when_no_object` asserts the rate-0 sentinel (no object → 0,
  bypassing the clamp). Matches §8 P3 (661→12) and C5 exactly.

The seam is a single `i32` total; the pipeline producer (power C10 + MF C11) plugs in at its own slice.

---

## 4. E1 cost-source + oracle wiring — DECISIONS (a) and (c)

### 4.1 DECISION (a): cost-based oracle balance via `&RuleSet` on `rebuild_shadow`

**`rebuild_shadow` gains a `&RuleSet` parameter and sets `balance`/`original_balance` to the type COST
(in credits), sourced from `sim.object_type(type_ref, rules).cost`.** This replaces the P2 frames-based
placeholder (E1 deferred, `factory.rs:244-246`: `balance = remaining_base_frames`,
`original_balance = total_base_frames`).

Why cost-based is mandatory in P3: the per-step charge is in **credits** (C3: `Balance/(54-Value)` in
cost units; C15 conservation == full `GetCost`). The P2 balance is **frames**. Charging a frame count
through `economy.spend` is meaningless and breaks `factory_exact_cost_conservation` (its costs
{1,25,700,99991} are CREDITS). The cost is already resolvable: `ObjectType.cost: i32`
(object_type.rs:155); `Simulation::object_type(type_ref, rules) -> Option<&ObjectType>` already exists
(world/mod.rs:496) and resolves via the type-handle table with a name-path fallback. So P3 only threads
`rules` and reads `.cost`.

Concrete change in `rebuild_shadow`:
```
// E1 (P3): cost-based oracle balance. original_balance = full type cost (snapshot for
// conservation); balance = the not-yet-charged remainder, computed by replaying the
// EXACT per-step charge ladder for `progress` steps (NOT a one-shot proportion — see below).
let full_cost = sim.object_type(front.type_id, rules).map(|o| o.cost.max(0)).unwrap_or(0);
let original_balance = full_cost;
let balance = remaining_balance_after(full_cost, progress);   // ladder replay, integer-only
// step_rate_frames stays 0 in the rebuild (the probe calls set_rate separately, §4.2).
```

`remaining_balance_after(cost, p)` replays the charge ladder: start `b = cost`, for `value` in `1..=p`
let `k = 54 - value`; `b -= if k == 0 { b } else { b / k }`. ≤54 integer iterations. This yields the
EXACT balance the authoritative stepper would hold at that progress, so a freshly-stepped factory and
the rebuilt shadow agree and the conservation assert (§6) is meaningful.

**REJECTED simpler alternative:** `balance = full_cost * (54 - progress) / 54` (one division). It does
NOT match the per-step floor-division running balance (off by accumulated rounding), which would make
the conservation shadow-assert spuriously diverge. Fidelity-first ⇒ replay the exact ladder.

Threading `rules` (the `refresh_production_shadow` tail already carries `Option<&RuleSet>`,
world/mod.rs:1006):
```
pub(crate) fn refresh_production_shadow(&mut self, rules: Option<&RuleSet>) {
    self.refresh_economy_shadow(rules);
    let mut registry = std::mem::take(&mut self.production.factory_shadow);
    match rules {
        Some(r) => registry.rebuild_shadow(self, r),
        None    => registry.rebuild_shadow_no_rules(self),  // cost=0 fallback (P2 behavior)
    }
    self.production.factory_shadow = registry;
}
```
The `None` arm preserves the P2 cost-free path so `advance_tick`'s `None` callers
(`production_shadow_preserves_advance_tick_phase_order`) never panic. **This holds hash-neutrality
unconditionally** — the registry is `#[serde(skip)]` + no-serde-derive either way.

**REJECTED alternative (oracle-cost-in-tests-only, D3):** keep `rebuild_shadow` cost-free and feed cost
only in unit tests. Rejected: it leaves the live shadow frames-based, so the per-tick conservation
shadow-assert can only run on synthetic fixtures, never on live factories — the weaker parity guard.

### 4.2 DECISION (c): the oracle step runs as a debug probe BESIDE the no-op arm (FIT-a, hash-neutral)

**The `EntityCategory::Structure` arm (techno_ai.rs:107) stays a LITERAL no-op.** FIT-(a) (study §6.3
line 646 — preferred) is honored by the **shadow probe walking the same LogicVector order** and
stepping each structure's factory CLONE — the "proof lives beside, not inside, the no-op arm" shape the
P2 comment already mandates (techno_ai.rs:225-236). This is the S1 template
(`unit_ai_shadow_step`, techno_ai.rs:162: READ-ONLY `&Simulation`, mutates nothing).

A new debug-only `factory_oracle_step_trace` (`&Simulation`), extending the existing
`factory_shell_trace` (techno_ai.rs:252):
```
// debug/test only; READ-ONLY w.r.t. all hashed state:
for id in self.live_object_order_snapshot():                  // LogicVector order = FIT-(a)
    if structure is live, non-dying, and resolves to a factory owner+category in the registry:
        let mut oracle_factory = registry.lookup(key).clone();    // throwaway (registry is a LOOKUP)
        let mut oracle_econ    = house.economy.clone();           // CLONE, not the wallet
        oracle_factory.set_rate(build_step_time_placeholder);     // exercise SetRate (rate non-stale)
        let outcome = oracle_factory.advance_one_step(&mut oracle_econ);
        record the (outcome, charged delta) into a local trace; NEVER write back.
```

This honors FIT-(a) (step driven from the Structure arm's LogicVector walk; registry = lookup) WITHOUT
making it authoritative: it operates on clones, surfaces divergence, never commits. The arm stays a
literal no-op, so `techno_ai_shell_is_passthrough_no_hash_change` (techno_ai.rs:331) stays valid
verbatim. The authority flip (arm becomes a real `&mut` step on the hashed economy/registry, with the
timer driver) is P5 — this design leaves that exact seam.

**Building-type → ProductionCategory binding — UNKNOWN at P3 precision (§9).** Which Structure owns
which `(owner, category)` factory (engine `Primary_For*` slots) is not modeled at P3. The probe is
gated to a BOUNDED scope (like S1): step each live Structure's owner's factories via the registry
lookup, or fall back to iterating `registry.iter_insertion_ordered()` (factory.rs:167) cloning each.
Both are hash-neutral. The full per-building routing is a P5 ordering concern; P3 proves the charge math
+ conservation, not the routing. If the bounded-Structure form cannot resolve a clean per-building
binding this slice, use the insertion-order iteration and mark the FIT-(a) LogicVector-order claim
UNPROVEN for the probe (study §6.3 option b), never asserted.

---

## 5. No-hash proof + its test

### 5.1 The structural argument (D3 graft — strongest available)

The no-hash guarantee rests on three independently-sufficient structural facts, each auditable from the
diff:

1. **No serde derive.** `Factory`/`FactoryRegistry` (factory.rs:42,52,68,127) and `Economy`
   (economy.rs:17) carry NO `Serialize`/`Deserialize` — only `derive(Debug, Clone, [Default,]
   PartialEq, Eq)`. None of their fields enter bincode or the hash. `advance_one_step`/`set_rate`
   mutate only a `Factory` + an `Economy`, neither of which `state_hash()` visits.
2. **No new authoritative `advance_tick` call site.** `refresh_production_shadow` (world/mod.rs:1006)
   still only calls `refresh_economy_shadow` + `registry.rebuild_shadow`. P3 adds NO `step_all` /
   `advance_one_step` call into the running sim. The methods are `#[allow(dead_code)]`
   (factory.rs:26) until P5 — "no call site = no behavioral change = the per-tick hash sequence is
   byte-for-byte P2's."
3. **The oracle is always a clone.** P3 only ever steps a CLONE of a factory against a CLONE of an
   economy (§4.2) or a test-local `Economy`. `HouseState.economy` and the legacy `credits` are never
   the receiver of `spend`. The legacy upfront-charge (`production_queue.rs:~218`) stays authoritative.

The debug probe (§4.2) is `#[cfg(any(test, debug_assertions))]`, `&Simulation`, writes only to local
clones. `world_hash.rs` is UNTOUCHED; `SNAPSHOT_VERSION` STAYS 17 (snapshot.rs:24). ⇒ `state_hash()` is
bit-identical before/after any P3 stepping.

### 5.2 The acceptance test (required — §8 P3 `factory_advance_step_does_not_change_state_hash`)

Mirrors `economy_shadow_does_not_change_state_hash` (production_shadow_tests.rs:91) and
`techno_ai_shell_is_passthrough_no_hash_change` (techno_ai.rs:331), reusing
`empty_rules()`/`queued_item`/`insert_queue` helpers (production_shadow_tests.rs:20-61):
```
#[test]
fn factory_advance_step_does_not_change_state_hash() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(&mut sim, owner, ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1));
    sim.refresh_production_shadow(Some(&rules));        // cost-based shadow built
    let before = sim.state_hash();

    // Step a CLONE of the shadow factory against a CLONE of the wallet, 54 times.
    let mut f = sim.production.factory_shadow.iter_insertion_ordered()[0].clone();
    let mut oracle = sim.houses[&owner].economy.clone();
    for _ in 0..PRODUCTION_STEPS { let _ = f.advance_one_step(&mut oracle); }
    sim.refresh_production_shadow(Some(&rules));        // rebuild again

    assert_eq!(before, sim.state_hash(),
        "P3 oracle stepping must not perturb the state hash (serde-skip + clone)");
}
```
Plus a per-tick variant mirroring `production_shadow_preserves_advance_tick_phase_order`
(production_shadow_tests.rs:337): run `advance_tick` 5× twice, assert identical hash sequences (the
debug probe must not introduce nondeterminism).

---

## 6. Conservation shadow-assert (surface, never equalize)

A debug-only assert wired into `debug_assert_production_shadow` (world/mod.rs:1021) beside
`debug_assert_factory_shell_trace`, mirroring the S1 divergence-surfacing discipline
(`debug_assert_s1_shadow`, techno_ai.rs:193 — surface tick+id, never write back):

```
#[cfg(any(test, debug_assertions))]
fn debug_assert_factory_conservation(&self):
    // For each live shadow factory with an object: step a CLONE to completion against
    // a CLONE economy seeded with exactly original_balance; assert exact-cost
    // conservation (C15). Divergence is SURFACED (tick+owner+category), NEVER equalized.
    for factory in registry.iter_insertion_ordered():
        if factory.object.is_none() { continue; }
        let cost = factory.original_balance;
        let mut f = factory.clone();
        f.balance = cost; f.progress = 0; f.on_hold = false; f.suspended = false; f.manual = false;
        let mut econ = Economy { credits: cost, ..Default::default() };
        let mut steps = 0;
        loop {
            match f.advance_one_step(&mut econ) {
                StepOutcome::Stepped   => steps += 1,
                StepOutcome::Completed => { steps += 1; break; }
                _ => break,   // Stalled/Idle cannot happen with exact funds -> the asserts below fire
            }
        }
        debug_assert_eq!(steps, PRODUCTION_STEPS as i32,
            "C2: tick {} {:?}/{:?}: full build must take 54 steps", ...);
        debug_assert_eq!(econ.spent_credits, cost,
            "C15: tick {} {:?}/{:?}: total spent {} must equal full cost {}", ...);
        debug_assert_eq!(f.balance, 0, "C12: completion zeroes balance");
        debug_assert!(f.suspended && f.object.is_some(),
            "C12: completion suspends with the object attached");
```

This checks the INTRINSIC conservation invariant of `advance_one_step` (Σ charge == original_balance,
true by telescoping, C15). It SURFACES any drift (tick+owner+category in the message) and NEVER writes
back — same contract as `debug_assert_s1_shadow`. The boundary-cost cases ({1,25,700,99991}) are pinned
by the dedicated `factory_exact_cost_conservation` unit test (§7), never re-derived in the runtime
assert (the P2 intrinsic-only-at-runtime pattern, techno_ai.rs:289).

---

## 7. Acceptance tests (the full §8 P3 set, in `factory.rs mod tests`)

| Test | Asserts | Contract |
|---|---|---|
| `factory_54_steps_to_complete` | start→Completed = exactly 53 `Stepped` + 1 `Completed`; 54 increments. | C2 |
| `factory_exact_cost_conservation` | for cost ∈ {1, 25, 700, 99991}: Σ oracle spend over full build == cost; balance ends 0. | C3/C15 |
| `factory_last_step_charges_full_remainder` | at progress 53→54 charge == remaining balance; no div-by-0 at `steps_left==0`; remainder charged ONCE (completion adds 0). | C3/C5 |
| `factory_stall_on_no_funds_rewinds` | oracle one credit below a step charge ⇒ `on_hold==true`, `progress` unchanged, oracle `spent_credits` unchanged. | C4 |
| `set_rate_total_over_54_truncates_clamps` | totals {0,53,54,661,14000} (with object) ⇒ rate {1,1,1,12,255}. | C5 (661→12) |
| `set_rate_zero_when_no_object` | no object ⇒ rate 0 (NOT 1); a suspended/queued-only factory `advance_one_step` ⇒ `Idle`. | C5 (rate-0 sentinel) |
| `factory_advance_step_does_not_change_state_hash` | §5.2. | no-hash guarantee |

Plus fidelity-first boundary sub-cases: exactly-affordable step proceeds (`available == charge`, strict
`<` boundary); cost-0 type completes free (every charge 0; conservation Σ=0=B₀ holds trivially); charge
truncation direction at cost {25} (the 25/54-ladder sums to exactly 25); the cost-1 corner (charges 0
for steps 1..52 since `1/k=0` for `k>=2`, then `1/1=1` on the last step — proves tiny-cost conservation
depends on the full-remainder last-step mechanism).

---

## 8. Out-of-scope seams (left clean, NOT implemented)

| Concern | Status | Seam |
|---|---|---|
| Authority flip (oracle→real wallet), SNAPSHOT_VERSION 17→18 | P5 | `advance_one_step(&mut Economy)` signature already P5-ready; flip the call site (arm body) + add the timer driver, not the method. |
| Cancel / partial refund (`original_balance − balance`) | P4 | `original_balance` + `balance` already hold the spent split (C8). |
| Prereq revalidation (3-way) / `on_hold` auto-unstick on resume | P6 | `BuildEligibility` declared (factory.rs:108); `on_hold`/`suspended`/`manual` present. P3 does NOT auto-clear `on_hold` (a stalled factory stays stalled). |
| Full `GetBuildStepTime` pipeline (low-power C10, MultipleFactory C11) | P-later | `set_rate(build_step_time: i32)` takes the total as input; the pipeline producer plugs in. |
| Purifier / IncomeMult / HarvestedCredits | P7 | `Economy` fields present; not exercised by P3. |
| Delivery / queue advance / object clear | P5+ | completion leaves `object: Some(..)`, `suspended=true`; delivery is command-bound (C7). |
| `step_timer` per-tick authoritative driver | P3 stepper is timer-free | a `tick_step_timer`-style helper may be provided but is NOT wired authoritative. |
| Building-type → ProductionCategory routing | P5 | the probe uses a bounded scope / insertion-order iteration; full `Primary_For*` routing is P5. |

---

## 9. UNKNOWN / UNCHECKED (marked, not guessed)

- **U1 — Building-type → ProductionCategory binding** for the §4.2 probe: the engine `Primary_For*`
  per-category slot mapping is not modeled at P3; the probe uses a bounded per-owner scope or
  insertion-order iteration. Full per-building routing is a P5 ordering concern. **UNKNOWN.**
- **U2 — `step_timer` reload trigger** (on-step vs on-stall vs on-completion): F3 says the timer holds
  `GetBuildStepTime/54`; the reload point is not pinned in the contract. Deferred to P5. **UNCHECKED.**
- **U3 — `build_step_time` total scale vs `ObjectType.cost` scale.** SetRate's `build_step_time` is a
  frame total; `advance_one_step` charges in cost credits — two independent quantities (rate vs money),
  kept separate, so no scale reconciliation is needed in P3. The `cost` ×100 internal-scale question
  (study §6.2 line 516 "x100 internal scale" vs current Rust `i32` credits at face value,
  object_type.rs:155) is a P5 authority-scale concern; P3's oracle uses whatever scale `ObjectType.cost`
  already is, consistently on both sides of the conservation assert — so conservation holds at any
  scale. **Flagged for P5, not blocking P3.**
- **U4 — read==write-wallet equivalence** for the affordability query vs `Spend_Money` (study §9.4):
  the engine reads availability through a +0x24/+0x18 sub-object and writes via Spend_Money(+0x30C);
  P3's oracle uses one `Economy` for both `available()` and `spend()`, so the read/write target is the
  same word by construction. Not load-bearing for P3 (single oracle). **Noted.**

---

## 10. Files touched (P3)

- `src/sim/production/factory.rs` — add `Factory::advance_one_step(&mut self, &mut Economy) -> StepOutcome`
  and `Factory::set_rate(&mut self, build_step_time: i32)`; change `rebuild_shadow` to take `&RuleSet`
  and set a cost-based `balance`/`original_balance` (ladder-replay remainder) + add a
  `rebuild_shadow_no_rules` cost-0 fallback + the `remaining_balance_after` helper; add the §8-P3 unit
  tests (all but the world-level no-hash one) + the boundary sub-cases.
- `src/sim/world/mod.rs` — thread `rules` into `rebuild_shadow` (`refresh_production_shadow`, :1006);
  add `debug_assert_factory_conservation` to `debug_assert_production_shadow` (:1021).
- `src/sim/world/techno_ai.rs` — add the debug-only `factory_oracle_step_trace` BESIDE
  `factory_shell_trace` (:252); the `EntityCategory::Structure` arm STAYS a literal no-op.
- `src/sim/world/production_shadow_tests.rs` — add `factory_advance_step_does_not_change_state_hash`
  + the per-tick determinism variant (reuse `empty_rules`/`queued_item`/`insert_queue`).

**NOT touched:** `world_hash.rs`, `SNAPSHOT_VERSION` (stays 17), `economy.rs` core (`spend`/`add_credits`/
`available` reused as-is), the legacy upfront-charge (`production_queue.rs:~218`, stays authoritative),
any miner/combat/movement file (concurrent session). **Verify:** `cargo test -p vera20k` (separate
foreground pass, per the build-discipline memory).

---

## 11. P3 TASK OUTLINE (for the planner to expand)

- **P3-T1** `factory.rs`: implement `Factory::set_rate(&mut self, build_step_time: i32)` per §3.1
  (resume non-manual; rate-0-no-object sentinel; `clamp(total/54, 1, 255)`). Unit tests
  `set_rate_total_over_54_truncates_clamps` + `set_rate_zero_when_no_object`.
- **P3-T2** `factory.rs`: implement `Factory::advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome`
  per §2.2 (armed gate; increment-first frame; per-step charge with last-step full remainder; pre-check
  stall + rewind; completion settlement). Unit tests `factory_54_steps_to_complete`,
  `factory_last_step_charges_full_remainder`, `factory_stall_on_no_funds_rewinds`, plus the
  exactly-affordable / cost-0 boundary sub-cases.
- **P3-T3** `factory.rs`: add `remaining_balance_after(cost, progress)` (ladder replay) and switch
  `rebuild_shadow` to `(&mut self, sim, rules: &RuleSet)` with cost-based `balance`/`original_balance`;
  add `rebuild_shadow_no_rules` (cost-0 fallback). Unit test `factory_exact_cost_conservation`
  (costs {1,25,700,99991} + the cost-25 ladder sum). Update the P2 `rebuild_shadow` doc-comment (the
  E1-deferred note becomes the E1-resolved note).
- **P3-T4** `world/mod.rs`: thread `rules` into `rebuild_shadow` in `refresh_production_shadow` (Some/None
  arms); add `debug_assert_factory_conservation` to `debug_assert_production_shadow`. Confirm the P2
  tests passing `Some(&rules)` still build the (now cost-based) shadow without panics.
- **P3-T5** `techno_ai.rs`: add the debug-only `factory_oracle_step_trace` beside `factory_shell_trace`
  (clone factory + clone economy, LogicVector-order walk, bounded scope or insertion-order fallback;
  record outcomes, never write back). Arm stays no-op; confirm `techno_ai_shell_is_passthrough_no_hash_change`
  unchanged.
- **P3-T6** `production_shadow_tests.rs`: add `factory_advance_step_does_not_change_state_hash` (§5.2)
  + a per-tick determinism variant. Confirm `snapshot_roundtrip_ignores_shadow` and
  `production_shadow_preserves_advance_tick_phase_order` still pass (None-arm + serde-skip intact).
- **P3-T7 (verify, separate foreground pass):** `cargo test -p vera20k` — read the literal `test result:`
  line; confirm SNAPSHOT_VERSION still 17 and no `world_hash.rs` diff.

---

*End of P3 design. The slice is additive and oracle-only: `advance_one_step`/`set_rate` are pure
`Factory` methods exercised against a cloned economy; the legacy upfront-charge stays authoritative;
`world_hash.rs`/`snapshot.rs` are untouched and `SNAPSHOT_VERSION` stays 17. The authority flip (oracle
→ real wallet), the timer-gated per-tick driver, and the 17→18 bump land at P5 (out of scope). SetRate
deliberately takes the build-step total as an input rather than deriving it from the legacy
`build_time_base_frames`, which carries the verified-REFUTED ×0.9 — the decisive parity choice this
slice makes.*
