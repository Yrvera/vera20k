---
title: Factory/House Production+Economy Substrate — P5 (authority-flip prep) Design Spec
date: 2026-06-04
status: design (winner = D2 substrate-fit, grafted with D1 tiny-detail ledger + D3 structural-no-hash; the
        only implement-now boundary is P5a, the LAST hash-neutral slice before the flip. Judge panel: D2 wins
        23/23/23 across all three judges; D1 second (20/20/20), D3 third (21/22/22) — D3 deferred the Lane-A
        insertion_seq mint fix on an inaccurate "the mint is hashed state in P5a" premise.)
scope: P5a ONLY — the last hash-neutral prep slice. Lands every flip-enabling piece that can be proven WITHOUT
       touching the hash: (1) a PURE x0.9-FREE `build_step_time` producer (C5/C10/C11) feeding the already-shipped
       `Factory::set_rate` (which takes the build-step TOTAL); (2) a PURE `category_for_object` routing helper
       (a tested delegate over the existing `production_category_for_object`) with the Ship-vs-Vehicle collapse
       SURFACED not folded; (3) the C7 delivery-command seam IDENTIFIED + the already-dormant `start_next_queued`
       confirmed as the bind point, NO new authoritative call site; (4) the Lane-A `insertion_seq` mint
       CORRECTION to temporal `enqueue_order` order (still serde-skip, hash-neutral now) + a blocking
       temporal-order assert; (5) the INVERSION-READINESS shadow assert — each tick, prove the authoritative
       MODEL (registry-sweep step + real producer + delivery) WOULD match the legacy per-tick result, SURFACED
       never equalized. STRICTLY HASH-NEUTRAL: NO serde derive added, NO un-skip of `economy`/`factory_shadow`,
       NO `world_hash.rs` change, `SNAPSHOT_VERSION` STAYS 17, the legacy `production_queue` path stays
       AUTHORITATIVE. Proven by a `*_does_not_change_state_hash` acceptance test mirroring P3/P4.
       OUT OF SCOPE (seams only, NOT implemented): the authority flip itself + serde derives + un-skip + hash
       fold + 17→18 + the C1 ordering lock (ALL P5b); the P9 global parity/replay acceptance gate (P5c).
source: docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md (v2-verified; C1 line 411, C5 line 419,
        C7 line 423, C10 line 429, C11 line 431, C12 line 433, C15 line 439, C19 line 447; §6.3 FIT line 625-649;
        §6.4 hash-set line 651-655; §7 retire list line 678-690; §8 P5 line 743-752, P8 line 774, P9 line 782)
        + this run's Lane A/Lane B research-gate findings (g_FactoryClass_Array temporal-append order +
        compacting removal; GetBuildStepTime 0x006F47A0 full truncation order no ×0.9; GetBuildTimeBonus
        0x0050c0a0 per-category dispatch; Primary_For* binding; GetFactoryCount 0x00500910 naval split; SetRate
        0x004C9EA0 ÷54 magic) + the committed P1-P4 code.
verification: every current-code claim is quoted file:TEXT from a live read this session (the tree shifts —
        anchor on text, not line numbers). Ghidra read-only this run (decompile/disassemble/read_memory/
        get_function_callers only). cargo NOT run (separate foreground pass). Research-gate Lane A/B verdicts
        folded into §3/§4/§5.
rule: Rust-native structure, gamemd-native semantics. sim/ never depends on render/ui/sidebar/audio/net.
      All sim math fixed-point/integer; EntityStore+BTreeMap keyed by InternedId; 30-player/20k scale.
---

# Factory/House Substrate — P5 Design Spec (authority-flip prep: pure producers + inversion-readiness assert)

## 0. TL;DR

Three competing P5a designs were scored against the v2-verified study and the committed P1-P4 code by a
three-judge panel. **The winner is the substrate-fit-first design (D2)** — a clean 23/23/23 sweep — grafted
with the parity-fidelity design's (D1) **complete tiny-detail ledger** and its full routing-table fidelity, and
the test-first design's (D3) **structural** no-hash proof (no serde derive + no authoritative call site +
clone/oracle-only receiver, proven by construction not by a sampled trace).

P5 is decomposed into three committable slices (LOCKED, §2). **This doc details P5a** — the last hash-neutral
prep slice — and scopes P5b (the atomic authority flip + version bump) and P5c (the P9 parity/replay gate) as
clean seams, mirroring how the P4 design scoped P5+ as seams.

P5a lands four pieces, all `#[serde(skip)]` / no-serde-derive / oracle-or-clone-only, leaving `state_hash()`
bit-identical and `SNAPSHOT_VERSION` at 17:

1. **A PURE `build_step_time` producer** — the real `GetBuildStepTime` TOTAL, **x0.9-free**, per-category
   `BuildTimeBonus`, ratio<1.0-gated Max clamp + 0.01 divisor floor (C10), **per-iteration** MultipleFactory
   truncation (C11), wall branch — feeding the **already-shipped** `Factory::set_rate(build_step_time)`
   (factory.rs `set_rate`). The `/54 + clamp[1,255]` stays in `set_rate`, NOT the producer (the engine's three
   callers all apply it there). The legacy `production_tech.rs` build-time family is audited **DRIFT — not
   reusable** (it bakes the verified-REFUTED `×0.9` and models build time as a rate-domain division with a
   single end-truncate, not the engine's multiply-then-per-step-truncate divisor).
2. **A PURE `category_for_object` routing helper** — a thin tested delegate over the existing
   `production_category_for_object` (production_tech.rs), keyed on the verified RTTI→Primary_For* table
   (Aircraft@+0x53AC / Infantry@+0x53B0; the refuted inverse NEVER reintroduced). It surfaces the
   **Ship-vs-Vehicle collapse** (Rust has no Ship category; naval folds into Vehicle) as an explicit P5b-or-later
   structural DRIFT — never silently folded.
3. **The C7 delivery seam IDENTIFIED + the dormant `start_next_queued` confirmed as the bind point** — P4 already
   ships `start_next_queued` proven-but-dormant; P5a names the exact `advance_tick` location where P5b binds it
   and the legacy ready/delivery path it replaces, plus a single dormant test-only `Simulation` probe. NO new
   authoritative call site.
4. **The Lane-A `insertion_seq` mint CORRECTION + the inversion-readiness shadow assert** — the load-bearing
   de-risk. P5a switches the shadow's `insertion_seq` derivation from BTreeMap-iteration order to **temporal
   first-Begin order via `BuildQueueItem.enqueue_order`** (factory.rs `rebuild_shadow_inner`), and adds a
   blocking assert that the registry sweep order equals the legacy temporal order per house. This converts the
   study §6.1/§6.3-flagged UNPROVEN same-frame-completion ordering into a **proven invariant before P5b makes
   it hashed** — hash-neutral now because the registry is `#[serde(skip)]`. The inversion-readiness assert then
   proves the authoritative MODEL (registry-sweep step + real producer + delivery) WOULD match the legacy
   per-tick result every tick, SURFACED (tick+owner+category), NEVER equalized.

**The whole point of P5a:** when the inversion-readiness assert and the temporal-order assert hold across the
suite, **P5b flips WHO is passed (oracle clone → real wallet) and what is hashed — not the algorithm, not the
order, not the rate.**

---

## 1. Scoring the three competing designs (P5a)

Each lens scored 1-5 (5 best) on five axes; totals are the mean across the three-judge panel.

| Axis | D1 (parity-fidelity) | **D2 (substrate-fit)** | D3 (test-first / risk) |
|---|---|---|---|
| **Parity-fidelity** | **5** — exact producer + per-category BuildTimeBonus + per-iter MF trunc + wall branch; fullest 18-row ledger; fixes the Lane-A mint in P5a | **5** — same producer + same routing-table fidelity + the same Lane-A temporal-mint correction; identical arithmetic; surfaces the Ship + BuildTimeBonus-field gaps | **4** — ships the verified producer but DEFERS the routing helper + delivery seam, so two verified gaps stay un-surfaced this slice; defers the Lane-A mint fix |
| **Substrate-program fit** | **4** — faithful, but adds the producer as a module + a parallel `step_all` sweep + an economy-clone-map for the assert — more new spine than the flip needs | **5** — smallest new surface that still de-risks: reuses the existing `set_rate(total)` caller, keeps `category_for_object` a thin delegate (slot-in, don't fork), the mint fix is one block in `rebuild_shadow_inner`, the assert is a verbatim sibling of `debug_assert_factory_conservation` | **4** — leanest, but defers the delivery seam, leaving P5b to author the bind point under flip pressure (the exact thing P4's dormant-`start_next_queued` discipline avoids) |
| **Testability / determinism** | **5** — full §8-P5 set + boundary sub-cases; integer throughout; 4-mode inversion assert | **5** — same set + the per-tick inversion-readiness assert (the strongest live guard) + the temporal-order assert + a determinism variant | **4** — full set as pure unit tests; the inversion assert's frames↔step tolerance is the most honestly framed of the three, but it proves weaker per-tick properties |
| **Risk / blast-radius (HASH-NEUTRAL)** | **3** — largest new surface (producer + routing layer + `step_all` + economy-clone plumbing) = more to keep provably serde-skip | **4** — one pure module + one debug-assert sibling + one dormant probe + one mint-source swap on already-skipped state; all clone/oracle-only; `world_hash.rs` untouched | **5** — smallest (producer + one assert); trivially auditable no-hash |
| **Buildability (lands green fast)** | **3** — most plumbing | **4** — producer is a translation of a verified formula; the mint swap is a one-block change; the assert mirrors P3/P4 verbatim | **5** — fewest moving parts |
| **TOTAL (panel mean)** | **20** | **23** | **~21.7** |

**Winner: D2 (substrate-fit-first), 23/23/23.** It honors the master directive — "slot into the substrate
program, do not invent a parallel architecture" — by **reusing** the `set_rate(total)` caller, the
`rebuild_shadow` mint plumbing, and the `debug_assert_production_shadow` chain rather than adding parallel
structures. It de-risks P5b as completely as D1 (the inversion-readiness + temporal-order asserts make the flip
a swap) at materially less new hash-neutral surface, and unlike D3 it surfaces the two verified gaps (Ship
collapse, delivery binding point) **now**, dormant, rather than letting them ambush the flip.

### 1.1 What was grafted from the runners-up

- **From D1 (parity-fidelity):** the complete **tiny-detail ledger** (§6) and the full **routing-table
  fidelity** (§4: the RTTI→category table incl. the surfaced Ship-split DRIFT and the per-category
  `GetBuildTimeBonus` field gap). D2 carries D1's honesty on the two structural gaps — it does NOT silently fold
  naval into Vehicle or assume a single scalar house build-bonus. D1's per-helper audit of
  `owner_effective_production_speed_ppm` (the C10 divisor sub-step) is adopted (§3.4): that one sub-step's clamp
  logic is reusable in spirit, the rest of the legacy family is DRIFT.
- **From D3 (test-first / risk):** the **structural** no-hash proof as the primary guarantee (§5), resting on
  three independently-sufficient facts auditable from the diff (no serde derive; no authoritative call site;
  clone/oracle-only receiver), and D3's exhaustive **write-set verification** framing as the burden-of-proof
  evidence (CLAUDE.md "default to DRIFT unless provably no-hash"). We also adopt D3's honest framing that the
  frames↔step bridge is NOT bit-identical (the legacy frames model is x0.9-baked) so the inversion assert must
  SURFACE the x0.9 delta, never demand bit-equality with the wrong legacy value.

### 1.2 Why D2 over D3 specifically (the delivery-seam + mint fork)

D3 ships only the producer + the inversion assert, arguing the routing helper, the delivery seam, and the
Lane-A mint fix are "P5b's problem." Two of those are wrong calls for P5a:

1. **The Lane-A `insertion_seq` mint fix is hash-neutral in P5a and is the single highest-value de-risk.** D3's
   ledger defers it on the premise that "changing the mint touches `insertion_seq`, which P5b hashes." But in
   P5a the registry lives in `#[serde(skip)]` `factory_shadow` with NO serde derive, and `hash_production`
   (world_hash.rs) never reads it — so the mint change is provably hash-neutral NOW, exactly like the P3/P4
   oracle methods. Landing it now converts the §6.3-flagged UNPROVEN same-frame ordering into a proven invariant
   *before* authority flips; deferring it leaves P5b an order-correctness gamble.
2. **The delivery seam should be identified + dormant now**, mirroring P4's discipline: P4 shipped
   `start_next_queued` proven-but-dormant so the flip *binds* an existing thing rather than authoring it under
   pressure. P5a identifying the C7 bind point and naming the legacy path it replaces is the same zero-hash-risk
   move.

D2 pays a small surface cost for a materially smaller P5b surprise surface. (D3's structural no-hash proof and
its frames↔step honesty are still adopted as grafts.)

---

## 2. The P5 decomposition (LOCKED) + P5b/P5c seams

### 2.0 The three LOCKED decisions (user-confirmed; designed WITHIN, not relitigated)

1. **DECOMPOSE P5 into three committable slices:** **P5a** (hash-neutral prep — this doc) → **P5b** (the atomic
   authority flip + `SNAPSHOT_VERSION` 17→18) → **P5c** (the P9 parity/replay acceptance gate). This workflow
   details P5a ONLY; P5b/P5c are scoped seams (§2.2/§2.3), mirroring how the P4 design scoped P5+ as clean seams.
2. **STEP ORDER = REGISTRY SWEEP in `insertion_seq` order**, reproducing the engine's `PerTickUpdate` walking
   `g_FactoryClass_Array` in registration order then `g_HouseClass_Array` (C1/G2). This is a deliberate departure
   from study §6.3 FIT-(a) per-building `LogicVector` dispatch: the study itself admits "insertion_seq order ==
   LogicVector order" is an UNPROVEN equivalence (a building revealed→concealed→revealed keeps its factory
   `insertion_seq` but gets a new `LogicVector` position). The departure is justified by gamemd same-frame-
   completion output parity (the sweep reproduces the array-index = temporal step order — Lane A). **The
   `EntityCategory::Structure` arm of `object_ai_stage` STAYS a no-op for factories** (techno_ai.rs Structure arm).
3. **FOLD C1 (factory-step-before-house-tick ordering lock) INTO the P5b flip** = ONE version bump 17→18.
   Avoids a second 18→19 bump at P8 (study §8 P5 note line 745 default recommendation: fold C1 into P5).

### 2.1 P5a boundary (this doc — hash-neutral prep)

**IN (hash-neutral, all serde-free, oracle-or-clone-only):**

| Piece | Surface | Why it belongs in P5a |
|---|---|---|
| `build_step_time` producer | new pure fn in `factory.rs` (or a sibling `factory_rate.rs` if `factory.rs` crosses ~600 lines — planner's call) | The TOTAL fed to the already-shipped `set_rate`; the single largest correctness item the flip needs. Pure, tested vs C5/C10/C11; touches no hashed state. |
| `category_for_object` | new pure fn delegating to `production_category_for_object` | The sweep needs one tested routing mapping + the surfaced Ship gap. Pure. |
| `enqueue_order`-based `insertion_seq` mint | edit `rebuild_shadow_inner` in `factory.rs` | Bakes the gamemd-temporal sweep order in NOW so P5b's authoritative charge order is correct; hash-neutral (registry is serde-skip). |
| C7 delivery seam identification + dormant probe | doc + the existing dormant `start_next_queued` + a `#[cfg(test)]` `Simulation` probe | Names WHERE P5b binds + the legacy path it replaces; no new authoritative call site. |
| Inversion-readiness assert | `debug_assert_factory_step_matches_legacy` in `world/mod.rs`, chained into `debug_assert_production_shadow` | Proves the authoritative model == legacy per-tick on clones; the strongest de-risk. |
| Temporal-order assert | folded into the inversion assert (A) + a positive world-level test | Converts §6.3 UNPROVEN into a proven invariant. |

**OUT (P5b seams §2.2):** serde derives + un-skip of `economy`/`factory_shadow`; `world_hash.rs` field add/remove;
`SNAPSHOT_VERSION` 17→18; making the registry sweep authoritative against the real wallet; binding the C7
delivery to `start_next_queued`; the C1 ordering lock fold; retiring legacy upfront-charge / `.rev()` cancel /
frames timer. **OUT (later):** prereq revalidation (P6), purifier/IncomeMult/HarvestedCredits (P7), the P9
replay harness (P5c).

### 2.2 P5b seam — the atomic authority flip + version bump (scope only, NOT detailed)

The first hashed-state change in the whole program. Scope (do NOT detail here):

- **serde + un-skip:** add `Serialize/Deserialize` to `Economy`/`Factory`/`FactoryRegistry`/`PendingObject`/
  `SpecialItem`; remove `#[serde(skip)]` from `HouseState.economy` and `ProductionState.factory_shadow`.
- **hash fold** in `world_hash.rs` `hash_houses`/`hash_production`: ADD `Factory` fields (owner, category,
  progress, step_rate_frames, step_timer, balance, original_balance, object(type_id+entity_id), on_hold,
  suspended, manual, queue, insertion_seq) + `Economy` (credits, spent_credits, harvested_credits,
  purifier_count) + `FactoryRegistry` next_insertion_seq (study §6.4 line 653). **D2/D1 recommendation
  (decided in P5a §5.2):** with the temporal `enqueue_order`-derived mint, `next_insertion_seq` is no longer the
  ordering source — **DROP it from the hashed/serialized set** (the order is carried by `enqueue_order`, already
  hashed via the queue) and replace the planned `registry_next_insertion_seq_is_serialized_and_hashed` test with
  `factory_insertion_seq_equals_front_enqueue_order`. REMOVE the retired legacy fields `active_producer_by_owner`
  + per-item `remaining_base_frames` + `progress_carry` from `hash_production` (study §7 retire list line 681-684).
- **flip authority:** add the ONE `FactoryRegistry::step_all` call to `advance_tick` Phase 7 (charging the real
  wallets via the per-step `advance_one_step(&mut Economy)`, in `insertion_seq` = temporal order — the body is
  unchanged, P5a proved it); bind the already-shipped `start_next_queued` at the C7 delivery commit (the seam P5a
  identified, §5.3); replace `set_rate`'s input source with the producer (the producer is already correct +
  tested). Retire the legacy upfront-charge (production_queue.rs `enqueue_by_type` `*credits -= obj.cost`),
  `.rev()`+full-refund cancel (`cancel_by_type_for_owner`), and the frames timer (`tick_production_with_overlay_
  registry` PPM `remaining_base_frames`/`progress_carry` integration).
- **fold C1** (factory-step-before-house-tail): place `step_all` before the house tail (the §6.3 placement P5a
  documents).
- **bump `SNAPSHOT_VERSION` 17→18.**
- **Tests:** `production_authoritative_hash_includes_factory_fields`, `snapshot_version_is_18`,
  `snapshot_roundtrip_factory_registry`, `legacy_active_producer_removed_from_hash`,
  `legacy_progress_carry_removed_from_hash`, `factory_insertion_seq_equals_front_enqueue_order` (replacing the
  `next_insertion_seq` hash test per the mint decision).

### 2.3 P5c seam — the P9 global parity/replay harness (scope only, NOT detailed)

The required end-to-end determinism gate (study §8 P9 line 782): a recorded command stream (begin, suspend,
cancel-one, cancel-all, place) replayed twice + against the pre-flip baseline yields a bit-identical per-tick
`state_hash()` sequence; `economy_conservation_over_replay` (C15 global invariant). Reuses the existing replay
harness. If the replay ever shows a same-frame two-Begin divergence, that intra-frame `EventClass::Execute`
dispatch order is the place to re-verify (§8 U-ORDER).

---

## 3. The `build_step_time` producer (C5/C10/C11) — the real GetBuildStepTime, x0.9-free

### 3.1 What it is and where the /54 lives

A **pure function** computing the build-step TOTAL (the un-divided `GetBuildStepTime` return), fed to the
**already-shipped** `Factory::set_rate(build_step_time: i32)` (factory.rs `set_rate`), which already does
`clamp(total/54, 1, 255)` with the no-object→0 sentinel. **The producer does NOT divide by 54 and does NOT
clamp** — that is the caller's job. Lane B verified all three engine callers (CalcRate / RecalcAllRates /
SetRate) apply the `÷54 clamp[1,255]` after `GetBuildStepTime`; SetRate 0x004C9EA0's `MOV EAX,0x4BDA12F7;
IMUL; SAR EDX,4; +sign` is the exact signed ÷54 (the `0x4BDA12F7` magic, C5 line 419). The producer returns the
total; `set_rate` is unchanged.

### 3.2 Signature

```rust
/// Produce the build-step TOTAL (the engine's GetBuildStepTime return), BEFORE the
/// caller's /54 + clamp[1,255]. PURE: integer/fixed-point throughout, no &mut, no RNG,
/// no hashed-state read, no float in the committed math. Fed to Factory::set_rate
/// (which owns the /54). Built from the R1-verified contract; the legacy
/// production_tech build-time family is a verified DRIFT (bakes a REFUTED x0.9,
/// rate-domain single-truncate, generic BuildSpeed) and is NOT reused.
pub(crate) fn build_step_time(inp: &BuildStepTimeInputs) -> i32;

/// Resolved inputs the caller (the P5b begin path / the dormant probe in P5a) gathers
/// from rules + the depositing house's country type + the owner's power + factory
/// count. A transient param struct (no serde, no storage) so the producer is a pure
/// function of explicit inputs (testable in isolation; no Simulation handle).
pub(crate) struct BuildStepTimeInputs {
    pub cost: i32,                       // GetCost of the object under construction
    pub build_time_bonus_ppm: u64,       // per-CATEGORY GetBuildTimeBonus (default 1.0 = PRODUCTION_RATE_SCALE)
    pub build_time_multiplier_ppm: u64,  // per-TYPE BuildTimeMultiplier (Type+0x608; rules build_time_multiplier_x1000)
    pub power_ratio_ppm: u64,            // owner GetPowerRatio, clamped [0, SCALE]; SCALE (1.0) if not under-powered
    pub low_power_penalty_modifier_ppm: u64, // Rules LowPowerPenaltyModifier (already parsed)
    pub min_clamp_ppm: u64,              // Rules MinLowPowerProductionSpeed
    pub max_clamp_ppm: u64,              // Rules MaxLowPowerProductionSpeed (applied ONLY when ratio < 1.0)
    pub multiple_factory_ppm: u64,       // Rules MultipleFactory (loop gate > 0)
    pub factory_count: u32,              // per-category GetFactoryCount (the (n-1) loop count)
    pub is_wall: bool,                   // RTTI==building AND the wall flag
    pub wall_build_speed_ppm: u64,       // Rules BuildSpeed (the wall double, pre-converted to ppm; used only if is_wall)
}
```

`build_time_bonus_ppm` is the **per-category** `GetBuildTimeBonus` (Lane B refinement: `0x0050c0a0` is
RTTI-dispatched over `HouseTypeClass+0x34`, default 1.0 for land-vehicle/building, per-side multipliers for
infantry/naval/aircraft/defense), NOT a single house scalar and NOT the generic `BuildSpeed`. **No rules field
backs this today** (§8 U-BONUS); for stock YR with no per-side build-speed bonus it is `PRODUCTION_RATE_SCALE`
(1.0), so the producer reduces to `trunc(Cost) × multiplier …`. The seam is present so P5b/P7 can wire real
per-side values without reshaping the producer. `wall_build_speed_ppm` is pre-converted to ppm by the caller
(the legacy `wall_build_speed_coefficient` is an `f32`, production_tech.rs `effective_time_to_build_frames_for_
object` consumes it via `(f32 as f64 * 1e6) as u64`) so the producer body stays integer-only.

### 3.3 The exact pipeline (PPM fixed-point, truncation-faithful)

PPM scale = `PRODUCTION_RATE_SCALE` = 1_000_000 = 1.0 (the existing scale, so the parsed `*_ppm` rules fields
feed the producer directly — only the *formula* is rebuilt, the INI plumbing is shared). Every multiply-truncate
rounds toward zero (the engine FPU control word @0x00822d80 = 0x0E7F, RC=truncate, C5 line 419; for non-negative
values = floor). The per-iteration truncation in step T4 is observable (C11), so each step floors at every
gamemd ftol point:

```
fn build_step_time(inp: &BuildStepTimeInputs) -> i32 {
    const SCALE: i128 = 1_000_000;
    let cost = inp.cost.max(0) as i128;
    if cost == 0 { return 0; }                                          // no work -> rate-0 path in set_rate

    // T1: base = trunc(BuildTimeBonus x Cost)   (NO x0.9 — the legacy x0.9 is REFUTED)
    let s1 = cost * inp.build_time_bonus_ppm as i128 / SCALE;           // floor

    // T2: x per-type BuildTimeMultiplier (Type+0x608), trunc
    let s2 = s1 * inp.build_time_multiplier_ppm as i128 / SCALE;        // floor

    // T3: low-power divide. divisor d = 1 - (1 - ratio) x LPPM, clamped (C10).
    //   - lower clamp to Min ALWAYS
    //   - upper clamp to Max ONLY when ratio < 1.0
    //   - if d <= 0 then d = 0.01  (= SCALE/100)
    let deficit = SCALE - (inp.power_ratio_ppm.min(1_000_000) as i128); // (1 - ratio), >= 0
    let penalty = deficit * inp.low_power_penalty_modifier_ppm as i128 / SCALE;
    let mut d = SCALE - penalty;                                        // (1 - (1-ratio)xLPPM) in PPM
    d = d.max(inp.min_clamp_ppm as i128);                              // Min clamp ALWAYS
    if (inp.power_ratio_ppm as i128) < SCALE {                         // ratio < 1.0 gate
        d = d.min(inp.max_clamp_ppm as i128);                         // Max clamp ONLY when under-powered
    }
    if d <= 0 { d = SCALE / 100; }                                    // 0.01 divisor floor
    let mut acc = s2 * SCALE / d;                                      // trunc(s2 / d): s2 frames / a PPM fraction

    // T4: MultipleFactory loop — (factory_count - 1) iterations, PER-ITERATION trunc (C11).
    //   gate: multiple_factory_ppm > 0  (strict; skip on 0)
    if inp.multiple_factory_ppm > 0 && inp.factory_count > 1 {
        for _ in 0..(inp.factory_count - 1) {
            acc = acc * inp.multiple_factory_ppm as i128 / SCALE;     // trunc EACH iteration (NOT MF^(n-1) once)
        }
    }

    // T5: wall branch — RTTI==building wall only, trunc(acc x BuildSpeed)
    if inp.is_wall {
        acc = acc * inp.wall_build_speed_ppm as i128 / SCALE;         // trunc
    }

    acc.clamp(0, i32::MAX as i128) as i32   // the TOTAL; set_rate does /54 + clamp[1,255]
}
```

The verified order (Lane B, `disassemble 0x006F47A0`): `s1 = trunc(GetBuildTimeBonus × GetCost)` (no ×0.9; the
`this` is the object under construction, reads its own house at `[ESI+0x21c]`) → `s2 = trunc(s1 ×
[Type+0x608])` → `s3 = trunc(s2 / divisor)` with the C10 clamps → MultipleFactory `n = GetFactoryCount`, gate
`MultipleFactory > 0 && (n-1) > 0`, loop `(n-1)` times `acc = trunc(acc × MF)` → wall `if RTTI==building &&
[obj+0x520]→[+0x1571] != 0 { return trunc(acc × BuildSpeed_double) }`. Constants `read_memory`: `0x007e2ac8 =
1.0f` (the `1.0` in `1.0-ratio`, NOT a 0.9), `0x007e1748 = 0.0f` (the `d <= 0` test), `0x007f4e34 = 0.01f` (the
divisor floor). **There is NO ×0.9 anywhere** (V2 (b), R1 confirmed).

### 3.4 Why the legacy `production_tech.rs` family is NOT reused (DRIFT, per the default verdict)

Per-helper audit (Lane B + a live read of production_tech.rs this session):

| Legacy helper (file:text) | Verdict | Reason |
|---|---|---|
| `build_time_base_frames` (`cost * speed_x1000 * 9 / 10000`) | **DRIFT — reimplement** | bakes the REFUTED `×0.9` (the `*9/10000`); uses generic `build_speed_x1000`, NOT per-category `GetBuildTimeBonus`. Two-axis drift. |
| `effective_time_to_build_frames_for_object` (`base_frames * SCALE / speed_ppm`) | **DRIFT — reimplement** | models build time as a **rate-domain division** (divide by a *speed*), not the engine's multiply-then-divide-by-clamped-divisor; truncation points differ. |
| `owner_effective_production_speed_ppm` (the C10 clamp: `1-(1-ratio)×LPPM`, Min always, Max only when ratio<1, 0.01 floor) | **clamp logic reusable in SPIRIT; reimplemented inline** | The clamp sequence matches C10 — but the legacy version produces a *speed* the time is then DIVIDED by, whereas the producer keeps the divisor in one truncation-faithful chain (T3). D1's verdict was "blanket reimplement"; D2's per-helper audit is more accurate: this one sub-step's clamp is the C10 logic and is the model for T3, but it is reimplemented inline (not called) so the producer stays a single chain. |
| `apply_multiple_factory_scaling_ppm` (loop multiply on the *time* domain) | **DRIFT — reimplement** | truncates per iteration but on `time_to_build` in the rate domain, NOT on the build-step total after the low-power divide (C11). Wrong domain + point. |
| `matching_factory_count_for_owner` (full-store rescan) | **reusable conceptually, replaced structurally at P5b** | the COUNT is the right input; the producer takes `factory_count` as a param. P5b reads it from the registry by key (study §7 retire line 687). |
| INI plumbing (`low_power_penalty_modifier_ppm`, `min/max_low_power_production_speed_ppm`, `multiple_factory_ppm`, `build_time_multiplier_x1000`, `wall_build_speed_coefficient`) | **REUSE** | already parsed (ruleset.rs); the producer consumes them as PPM inputs. |

**The legacy functions stay authoritative** (they feed the legacy frames timer) until P5b retires them. P5a does
not touch them; the producer coexists dormant.

### 3.5 Output check (C5/C10/C11)

With `cost=700, bonus=1.0, multiplier=1.0, ratio=1.0, factory_count=1, no wall`, the result is `700`;
`set_rate(700)` → `700/54 = 12`. The "MTNK total 661 → rate 12" example in the prompt is one concrete case (661
also gives `661/54 = 12`); the producer is tested for both the **total** (700, 661) and the **resulting rate**
through `set_rate`. The existing `set_rate_total_over_54_truncates_clamps` test (factory.rs) already pins
`set_rate(661) -> 12` and `set_rate(14000) -> 255`, so the producer test only needs to pin the TOTAL.

---

## 4. The `category_for_object` routing helper + the SURFACED Ship gap

### 4.1 Signature + the verified RTTI→category table

```rust
/// Map an object type to the ProductionCategory whose factory produces it — the Rust
/// analog of the engine's Begin_Production Primary_For* slot resolution (RTTI -> slot).
/// A thin tested delegate over the existing production_category_for_object: one routing
/// source, not a fork. NOTE the Ship gap (§4.3). Pure; no sim state.
pub(crate) fn category_for_object(obj: &ObjectType) -> ProductionCategory {
    production_category_for_object(obj) // delegate; the helper is the named seam, not new logic
}
```

Verified binding (Lane B, `decompile 0x004FA350` Begin_Production + `0x0048DCD0` RTTI_To_TypeArray; study §2b;
honor the v2 binding — Aircraft@+0x53AC / Infantry@+0x53B0; the inverse is REFUTED):

| gamemd RTTI / split | Primary_For* slot | Rust `ProductionCategory` |
|---|---|---|
| 2,3 (aircraft) | +0x53AC Aircraft | `Aircraft` |
| 0xf,0x10 (infantry) | +0x53B0 Infantry | `Infantry` |
| 1,0x28 naval-flag==0 (land vehicle) | +0x53B4 Vehicles | `Vehicle` |
| 1,0x28 naval-flag!=0 (naval) | +0x53B8 Ships | **`Vehicle`** (NO Ship category — DRIFT, §4.3) |
| 6,7 `+0xE08 != 5` (building) | +0x53BC Buildings | `Building` |
| 6,7 `+0xE08 == 5` (defense) | +0x53CC Defenses | `Defense` |

### 4.2 Verdict: a NEW pure helper, but the legacy enqueue path already routes the four non-naval categories

The Rust `ProductionCategory{Building, Defense, Infantry, Vehicle, Aircraft}` maps 1:1 onto FIVE of the six
gamemd slots. `production_category_for_object` (production_tech.rs) already does the Building↔Defense split via
`Some(BuildCategory::Combat) => Defense` (the Rust analog of `+0xE08==5`). **No new routing is needed for the
legacy enqueue path** (`queues_by_owner` keyed by `ProductionCategory` already routes the four non-naval
categories). D2 deliberately makes `category_for_object` a thin named delegate rather than a parallel
re-implementation — *slot in, don't fork*. Its value is being the **single call site the P5b sweep uses** and
the place the two DRIFTs are documented + pinned by tests:
- a test pinning the mapping against the verified RTTI table (so the refuted Aircraft/Infantry inverse can never
  silently return), and
- the explicit Ship-gap surface.

### 4.3 The two surfaced routing DRIFTs (default DRIFT, do NOT silently resolve)

- **Ship-vs-Vehicle (ledger #12, §8 U-SHIP).** gamemd keeps a 6th factory slot `Primary_ForShips (+0x53B8)`
  AND a separate `GetFactoryCount` field (Lane B: `0x00500910` is RTTI-dispatched over +0x5378 aircraft,
  +0x537c infantry, +0x5380 vehicle, +0x5384 building/defense, **+0x5388 ship**). Rust has no Ship
  `ProductionCategory` — naval maps to Vehicle. Under the P5b flip, a single Rust `Vehicle` factory key
  **collapses two gamemd factories** (War Factory + Naval Yard), which **diverges in the MultipleFactory
  `factory_count` (§3.3 T4) and same-frame completion ordering** when a player owns both. **Frequency: every
  water-map match with naval (Allied/Soviet naval).** P5a surfaces this as the routing helper's documented gap
  + a regression-guard test pinning the current collapse; whether to add a Ship category is a P5b/later
  structural decision (a hash-set change) requiring sign-off — NOT silently folded.
- **Defense-vs-Building equivalence (ledger #13, §8 U-DEFENSE).** `BuildCategory::Combat ⇔ BuildingType+0xE08==5`
  is the existing `production_category_for_object` assumption; not re-proven here. Default DRIFT; `category_for_
  object` reuses it as-is + adds a pinning test (surfaces if it ever diverges).

---

## 5. The C7 delivery seam + the Lane-A mint fix + the inversion-readiness assert

### 5.1 The C7 delivery seam — IDENTIFIED + dormant (NO new authoritative call site)

`Factory::start_next_queued` already exists, is `pub(crate)`, and is proven (P4: front-pop + held-object guard,
factory.rs). P5a adds NO authority. It (a) documents the bind point and (b) adds a single dormant test-only
probe.

- **Binding point in `advance_tick` (for P5b):** the post-delivery queue advance is the engine's `FUN_004FAA10`
  post-delivery `StartNextQueued`, bound to the successful place/delivery command (C7 line 423). In `advance_tick`
  this is Phase 7 (Scatter+Production+…), AFTER `FactoryRegistry::step_all` (which P5b adds) and AFTER the
  delivery command is processed (study §6.3 line 638 places "delivery commit (command-bound)" there). P5a
  **documents** the P5b call sequence: `step_all` (charge) → delivery command clears `Factory.object` →
  `start_next_queued` advances the FIFO front. P5a moves NO code into `advance_tick`.
- **Legacy path it replaces (named, not touched):** `tick_production` / `tick_production_with_overlay_registry`
  (production_queue.rs) drives completion → `ready_by_owner`; `place_ready_building` (production_placement)
  consumes the ready item; `ready_buildings_for_owner` exposes it. P5b retires the **queue-advance half** of this
  (completion→ready becomes `Factory` completion-holds-object + delivery-driven `start_next_queued`); the
  placement-geometry half of `place_ready_building` stays. The post-AbandonProduction auto-`StartNextQueued`
  (the `heapId = -1` path, C7 line 423) binds at the same seam.
- **The dormant probe (P5a code):** a `#[cfg(test)]` `Simulation` method that operates on a CLONE of the
  registry (never the hashed shadow) and proves the post-delivery mechanics end-to-end against the legacy
  ready→next transition — mirroring P4's `queue_advances_only_after_delivery` test, extended to compare against
  the legacy path. Wiring `start_next_queued` into a live `advance_tick` path in P5a would make a queued item
  start charging (a sim-state + cadence change), breaking the P5a invariant. P5a keeps abandon/advance/delivery
  separable; P5b binds them. This is the exact P4 discipline.

### 5.2 The Lane-A `insertion_seq` mint CORRECTION (do this in P5a — still hash-neutral)

**This is the load-bearing P5a change.** The committed `rebuild_shadow_inner` (factory.rs) mints `insertion_seq`
via `next_insertion_seq++`/`seq_carry` on the first tick a `(owner, category)` key appears, iterating
`sim.production.queues_by_owner` — a `BTreeMap<owner> → BTreeMap<category>`. BTreeMap iteration is **key-sorted**
→ the mint order is **sorted owner, then sorted category** (`Building < Defense < Infantry < Vehicle <
Aircraft`, the `ProductionCategory` `Ord` in production_types.rs). gamemd's `g_FactoryClass_Array` order is
**temporal first-Begin_Production order** (Lane A Verdict 2: `disassemble 0x004C98F0` ctor `slot = old Count;
Count++; array[slot] = this` strict tail-append; `decompile 0x004CA790` dtor `Count--` + a shift-left
compaction; `disassemble 0x0055AFB0` PerTickUpdate sweeps strict ascending array index).

**They diverge whenever first-Begin temporal order ≠ sorted (owner,category) order** (Lane A Verdict 3): one
house queues Aircraft then Vehicle the same rebuild tick → gamemd charges Aircraft first (earlier registration);
current Rust charges Vehicle first (`Vehicle < Aircraft` enum order). Or: House B begins before House A in
command order but A's `InternedId` sorts first → gamemd charges B first; current Rust charges A first.
**Player-observable:** the sweep order decides which factory's per-step charge wins when a house can't afford two
same-tick steps — different build stalls, different completion frame, different `state_hash` once P5b is
authoritative. This is the §6.1/§6.3 UNCHECKED equivalence; the honest verdict is DRIFT.

**Fix (P5a, hash-neutral):** derive the ordering key from `BuildQueueItem.enqueue_order` (production_types.rs,
already serialized + hashed via `hash_production` world_hash.rs) — the front (earliest still-live) item's
`enqueue_order` is the temporal stamp of when that `(owner, category)` first began producing (the faithful
analog of ctor tail-append position). `enqueue_order` is stamped monotonically from `next_enqueue_order` on
every enqueue (production_queue.rs, the Begin-command analog).

```
// in rebuild_shadow_inner, replacing the seq_carry / next_insertion_seq first-appearance block:
let seq = front.enqueue_order;   // temporal first-Begin stamp (replaces next_insertion_seq++ / seq_carry)
// the factory's insertion_seq = seq; iter_insertion_ordered() sorts by it (sort unchanged, source changed).
```

- **Drop `seq_carry` and `next_insertion_seq` as the ordering source.** `iter_insertion_ordered()` (sort by
  `insertion_seq`) now reproduces gamemd array-index = temporal order across both the two-category and two-house
  cases, with zero new globals. Ties cannot occur (`next_enqueue_order` is strictly monotonic).
- **Destroy-recreate fidelity:** reading the *current front's* `enqueue_order` each rebuild is exactly right.
  gamemd moves a recreated factory to the array tail (compaction + re-append); a lapsed-then-restarted category
  gets a fresh, higher `enqueue_order` (new Begin → new `next_enqueue_order`) → a higher `insertion_seq` → tail
  position. The carry-by-key `seq_carry` was the wrong mechanism (it carried a stale seq across a queue-empty
  gap or re-minted in sorted position) and is removed.
- **Hash-neutrality:** the registry is `#[serde(skip)]` with no serde derive, so changing how `insertion_seq` is
  minted changes NO hashed bit in P5a. The mint change affects only the (dormant) sweep order, observable solely
  via the debug assert/tests. The `next_insertion_seq` removal from the *hash set* is deferred to P5b (§2.2):
  P5b should DROP it (the order is now `enqueue_order`-carried) — flagged so the P5b hash-field list drops it.

### 5.3 The inversion-readiness shadow assert (the strongest de-risk)

A debug-only assert, chained into `debug_assert_production_shadow` (world/mod.rs) beside
`debug_assert_factory_conservation`, mirroring the P3 clone-only template **exactly** (clone factory + clone
economy; surface tick+owner+category; never write back). It proves the **authoritative model** (registry-sweep
step in temporal order + real `build_step_time`→`set_rate` + delivery) WOULD produce the same per-tick result as
the **legacy path** (charge/progress/completion/ready) — so P5b's flip is verified-equivalent before it happens.

```rust
#[cfg(debug_assertions)]
pub(crate) fn debug_assert_factory_step_matches_legacy(&self, rules: Option<&RuleSet>) {
    // (A) ORDER: the registry sweep order (temporal insertion_seq) must equal the legacy
    //     per-house temporal order (enqueue_order-sorted front items). For each owner,
    //     collect (category, front.enqueue_order) from queues_by_owner, sort by
    //     enqueue_order, and assert iter_insertion_ordered() visits that owner's
    //     factories in the same sequence. SURFACE (tick+owner) on mismatch.
    //
    // (B) RATE: for each live factory, build BuildStepTimeInputs from `rules` + the
    //     owner's power_state + factory_count, run the producer, run set_rate on a CLONE,
    //     and assert the producer is INTERNALLY consistent (total/54 clamp). SURFACE
    //     (never equalize) the producer-vs-legacy effective-rate divergence so the
    //     x0.9/truncation DRIFT is VISIBLE in the log, not silently reconciled. The
    //     legacy value is the wrong one; this assert RECORDS the gap, it does NOT force a
    //     match (frames<->step is NOT bit-identical — the legacy frames model bakes x0.9).
    //
    // (C) CHARGE/PROGRESS/COMPLETION/STALL: drive a CLONE of the registry's per-step
    //     model over one tick against CLONE economies seeded from the shadow economies;
    //     assert the model WOULD match the legacy per-tick result where they map by
    //     construction (the per-step ladder conserves exact cost — proven by
    //     debug_assert_factory_conservation; the model's on_hold maps to legacy NoFunds;
    //     completion == Done with the object held; ready == object-held-awaiting-delivery).
    //     SURFACE divergence with tick+owner+category; NEVER write back.
    //
    // (D) DELIVERY: on a CLONE, simulate clearing a completed factory's object then
    //     start_next_queued; assert the FIFO front advances and matches the legacy
    //     ready->next transition. SURFACE only.
}
```

Wired with one added line (anchor on the P3 `debug_assert_factory_conservation` text):

```rust
#[cfg(debug_assertions)]
pub(crate) fn debug_assert_production_shadow(&self) {
    self.debug_assert_economy_shadow();
    self.debug_assert_factory_shell_trace();
    self.debug_assert_factory_conservation();                          // P3
    self.debug_assert_factory_step_matches_legacy(/* rules */ None);   // P5a  <-- added
}
```

**Design notes (the honest-tolerance discipline, D3 graft):**
- **(B) records the DRIFT, does not equalize it.** The legacy frames model is the verified-wrong `×0.9`/
  rate-domain one; the assert SURFACES the producer-vs-legacy gap (so the magnitude is logged and the
  planner/user can confirm the producer is the correct one) but never forces the model to match the legacy wrong
  value. This is the burden-of-proof default in action.
- **(C) CAN be asserted equal where the legacy and model agree by construction:** the per-step charge ladder
  (`advance_one_step`) already conserves exact cost (proven by `debug_assert_factory_conservation`), and the
  model's stall (`on_hold`) maps to the legacy `NoFunds` state. Where they map, assert equality; where the legacy
  upfront-charge differs (it never stalls — the verified DRIFT, study §7 retire line 683), SURFACE the difference.
- **rules threading:** `debug_assert_production_shadow` currently takes no `rules`. The call site at the
  `advance_tick` tail (world/mod.rs, `self.refresh_production_shadow(rules)` then `self.debug_assert_production_
  shadow()`) has the `Option<&RuleSet>` in scope. P5a passes the same `Option<&RuleSet>` into the assert so (B)
  can compute the producer inputs. Anchor the call-site edit on the existing `refresh_production_shadow(rules)` /
  `debug_assert_production_shadow()` text; world/mod.rs is co-edited by a concurrent session, so the edit is the
  minimal "add the rules arg + the new assert call." If threading `rules` collides, fall back to the `None` arm
  (the producer skips (B) when `rules` is `None`, exactly like `rebuild_shadow_no_rules`).

### 5.4 The temporal-order assert (the §6.3 UNCHECKED → proven invariant)

The (A) order check, exposed as a positive blocking world-level test (§7): for an owner with Aircraft begun
before Vehicle (temporal `[Aircraft, Vehicle]` but enum-sorted `[Vehicle, Aircraft]`), assert
`iter_insertion_ordered()` follows `enqueue_order`, NOT category-sort. This converts the study "insertion_seq
order == array order UNPROVEN" into a proven invariant before P5b makes it hashed — so P5b is an
order-correctness swap, not a gamble.

---

## 6. No-hash proof + the tiny-detail ledger

### 6.1 The structural argument (D3 graft — strongest available, by construction)

Hash-neutrality is **structural, not behavioral**, inheriting the P2/P3/P4 property. Four independently-
sufficient facts, each auditable from the diff:

1. **No serde derive added.** The producer is a free function returning `i32`; `BuildStepTimeInputs` is a
   transient param struct (no derive beyond `Debug`, never stored); `category_for_object` returns a
   `ProductionCategory` (which IS serde, but the function adds no field anywhere); the `insertion_seq` mint edit
   mutates a `#[serde(skip)]`, no-serde-derive `FactoryRegistry` field; the dormant delivery probe + the
   inversion assert touch only clones. `Factory`/`FactoryRegistry`/`Economy`/`PendingObject`/`SpecialItem`/
   `StepOutcome`/`CancelOutcome` stay serde-free (factory.rs / economy.rs); `HouseState.economy` and
   `ProductionState.factory_shadow` stay `#[serde(skip)]` (house_state.rs / production_types.rs). NO new type
   enters bincode or the hash.
2. **No new authoritative call site.** The registry sweep is NOT made authoritative (LOCKED-decision 2 — the
   `EntityCategory::Structure` arm stays no-op, techno_ai.rs). `refresh_production_shadow` (world/mod.rs) still
   only calls `refresh_economy_shadow` + `rebuild_shadow`. The producer / `set_rate` / `start_next_queued` /
   delivery model are invoked ONLY from the debug-only assert and `#[cfg(test)]` code, against clones. The legacy
   `production_queue` charge/refund/ready path stays authoritative, untouched.
3. **The mint change touches only serde-skip state.** `insertion_seq` / `seq_carry` / `next_insertion_seq` all
   live in `FactoryRegistry`, which is `#[serde(skip)]` on `ProductionState.factory_shadow` AND has no serde
   derive. `hash_production` (world_hash.rs) never reads them. Changing the mint changes the (dormant) sweep
   order only — observable solely via the debug assert/tests, never via `state_hash()`.
4. **`world_hash.rs` and `snapshot.rs` untouched.** `SNAPSHOT_VERSION` STAYS 17 (snapshot.rs; the existing pin
   test still passes). The 17→18 fold is P5b.

The write set of every P5a addition is `{free-fn return value, non-serde Factory/FactoryRegistry/Economy clone
fields, debug-assert/test locals}` — exhaustively disjoint from the hashed set. By construction, **no input can
change a hashed bit**. This is exhaustive write-set verification, the burden-of-proof bar (not a sampled trace).

### 6.2 Tiny-detail ledger (D1 graft)

| # | Detail | Resolution | Grounding |
|---|---|---|---|
| 1 | **×0.9 in the base** | NONE. `base = trunc(BuildTimeBonus × Cost)`. The legacy `*9/10000` is the REFUTED ×0.9 — not in the producer. | C5 line 419; V2 (b); production_tech.rs `build_time_base_frames` `cost*speed_x1000*9/10000` |
| 2 | **Base factor identity** | Per-CATEGORY `GetBuildTimeBonus` (HouseTypeClass), default 1.0; NOT the generic `BuildSpeed`, NOT a single house scalar. Land-vehicle/building = 1.0; infantry/naval/aircraft/defense = per-side. | Lane B `0x0050c0a0` |
| 3 | **Truncation order** | trunc after T1, T2, T3 (the divide), EACH T4 iteration, and T5. Round toward zero (FPU RC=truncate; = floor for non-neg). | C5 line 419; R1 |
| 4 | **Low-power Max clamp gate** | Max clamp applied **only when `ratio < 1.0`**; Min clamp applied always. | C10 line 429 |
| 5 | **Divisor floor** | If `d <= 0` then `d = 0.01` (= `SCALE/100`). The `<= 0` test is vs 0.0, replaced with 0.01. | C10; R1 (`0x007e1748`=0.0f, `0x007f4e34`=0.01f) |
| 6 | **MultipleFactory loop count + truncation** | `(factory_count - 1)` iterations, **trunc after each**, NOT `MF^(n-1)` with one truncate. Gate `multiple_factory_ppm > 0` strict (skip on 0). | C11 line 431; R1; production_tech.rs `apply_multiple_factory_scaling_ppm` (DRIFT) |
| 7 | **Wall branch** | `trunc(acc × BuildSpeed)` **only** when RTTI==building AND the wall flag set. BuildSpeed is the engine `double` here (passed pre-converted as PPM). | C5 line 419; R1 |
| 8 | **`/54` + clamp[1,255] location** | In the CALLER `set_rate` (factory.rs, already shipped), NOT the producer. Signed `/54` truncates toward zero (`0x4BDA12F7` magic). | C5; Lane B SetRate 0x004C9EA0 |
| 9 | **No-object → rate 0** | `set_rate` already returns rate 0 with no object (factory.rs); the producer is not called in that case. The producer also returns 0 for `cost <= 0`. | C5; factory.rs `set_rate` |
| 10 | **`insertion_seq` mint order** | Temporal `front.enqueue_order` (first-Begin order), NOT BTreeMap-iteration `next_insertion_seq++`. Drops `seq_carry` as the ordering source. | Lane A Verdict 3; production_types.rs `enqueue_order`; factory.rs `rebuild_shadow_inner` |
| 11 | **Destroy-recreate ordering** | Re-read front `enqueue_order` each rebuild → a restarted category gets a fresh higher stamp → tail position (matches gamemd compaction + re-append). | Lane A Verdict 2/3 |
| 12 | **Ship-vs-Vehicle routing** | SURFACED as DRIFT (no Ship category; naval collapses into Vehicle → divergent MF count + same-frame order). Not fixed in P5a; explicit P5b decision. | Lane B §4; GetFactoryCount +0x5388 ship vs +0x5380 vehicle |
| 13 | **Defense-vs-Building split** | `category_for_object` reuses `production_category_for_object` (`BuildCategory::Combat ⇔ +0xE08==5`); the equivalence is DRIFT-default and gets a pinning test. | Lane B §4; production_tech.rs `production_category_for_object` |
| 14 | **PPM scale reuse** | Producer uses `PRODUCTION_RATE_SCALE = 1_000_000` PPM, so the parsed rules `*_ppm` fields feed the producer directly (only the formula is new). | production_types.rs `PRODUCTION_RATE_SCALE` |
| 15 | **i128 intermediates** | All multiplies in i128 to avoid overflow (cost ~50k × bonus ~1e6 overflows i32); final `clamp(0, i32::MAX)`. | mirrors production_tech.rs i64 discipline |
| 16 | **`step_all`/probe/producer are dormant** | No `advance_tick` call; exercised only on clones inside the assert + tests. P5b adds the authoritative call site. | P3/P4 dormant discipline |
| 17 | **Assert surfaces, never equalizes** | (B) records the producer-vs-legacy DRIFT magnitude; (C/D) surface charge/progress/delivery divergence with tick+owner+category; nothing written back. | world/mod.rs `debug_assert_factory_conservation` P3 template |
| 18 | **`build_time_bonus_ppm` default** | Stock YR with no side build-speed bonus = `PRODUCTION_RATE_SCALE` (1.0); producer reduces to `trunc(Cost) × multiplier …`. The seam is present for P5b/P7 per-side wiring; the field is MISSING from rules today (§8 U-BONUS). | Lane B (default 1.0 branch); ruleset.rs (no per-category bonus field) |

---

## 7. Acceptance + boundary + determinism + no-hash test list

### In `factory.rs mod tests` (pure value-type):

| Test | Asserts | Contract |
|---|---|---|
| `build_step_time_no_x09_base` | `cost=700, bonus=1.0, mult=1.0, ratio=1.0, count=1, no wall` → total `700` (NOT `630` = the ×0.9 result); `set_rate(700)` → `12`. | C5; ledger #1 |
| `build_step_time_mtnk_example` | the C5 reference case → rate `12` (both the 700 and 661 totals divide to 12). | C5 |
| `build_step_time_build_time_multiplier` | `mult=1.15` (PRISM-like) truncates at T2 (e.g. base 67 → `trunc(67×1.15)=77`). | ledger #2/#3 |
| `build_step_time_low_power_max_clamp_gated` | ratio<1.0 applies Max clamp; ratio>=1.0 does NOT; ratio 0 hits Min clamp; divisor `<=0` floors to 0.01. | C10; ledger #4/#5 |
| `build_step_time_multiple_factory_per_iteration_trunc` | `count=3, MF=0.8`, small `acc` → result DIFFERS from `acc×MF²` single-truncate (the load-bearing per-iteration proof). | C11; ledger #6 |
| `build_step_time_multiple_factory_gate_skips_on_zero` | `MF=0` → loop skipped (total unchanged regardless of count); `count=1` → loop skipped. | C11; ledger #6 |
| `build_step_time_wall_branch_only_for_walls` | `is_wall=true` applies BuildSpeed; `is_wall=false` does not. | C5; ledger #7 |
| `build_step_time_zero_cost_is_zero` | `cost<=0` → `0`. | ledger #9 |
| `build_step_time_overflow_safe` | `cost=50000, bonus=1e6, mult=1e6` does not overflow (i128 intermediates), clamps to `i32::MAX`. | ledger #15 |
| `category_for_object_matches_rtti_table` | each `ObjectCategory`/`BuildCategory` maps to the verified `ProductionCategory` (aircraft→Aircraft, infantry→Infantry, vehicle→Vehicle, building→Building, combat-building→Defense); the Aircraft/Infantry binding is pinned (refuted inverse cannot return). | Lane B §4; ledger #13 |
| `category_for_object_naval_collapses_to_vehicle_documented` | a naval unit type maps to `Vehicle` (the DOCUMENTED collapse) — the test pins the current behavior AND its doc names the DRIFT (regression guard so a future silent Ship-fold is caught). | ledger #12 |

### In `production_shadow_tests.rs` (world-level, hash-neutral — reuse `empty_rules`/`queued_item`/`insert_queue`/`HouseState::new`):

| Test | Asserts | Contract |
|---|---|---|
| `factory_flip_prep_does_not_change_state_hash` | build the producer + step a CLONE registry + CLONE economies + route a type + run the inversion assert; `state_hash()` bit-identical before/after; legacy `credits` untouched. (Mirrors `factory_advance_step_does_not_change_state_hash` / `factory_cancel_one_does_not_change_state_hash`.) | no-hash |
| `factory_insertion_seq_equals_front_enqueue_order` | after `refresh_production_shadow`, each factory's `insertion_seq == queue.front().enqueue_order`; a fixture where two categories of one owner have temporal order opposite to enum order proves the sweep follows temporal, NOT sorted-category, order. | Lane A; §5.2 |
| `factory_step_order_matches_legacy_temporal_order` | the inversion assert's (A) order check as a positive test: for an owner with Aircraft begun before Vehicle, the sweep visits Aircraft first (the DRIFT-fix vs the old sorted mint). | Lane A; §5.4 |
| `factory_step_matches_legacy_shadow_holds` | drive `advance_tick` over N ticks with a scripted queue; `debug_assert_factory_step_matches_legacy` fires no divergence every tick (the inversion-readiness assert holds). | §5.3 |
| `production_delivery_probe_is_dormant` | the delivery probe is test-only; a tick run leaves queue fronts unchanged absent a delivery (no `advance_tick` path invokes `start_next_queued`). | §5.1 |
| `production_flip_prep_is_deterministic` | a per-tick closure that builds the producer + runs the inversion model on clones; two runs → identical hash sequences (mirror `production_shadow_with_oracle_is_deterministic` / `production_shadow_with_cancel_is_deterministic`). | determinism |
| `snapshot_version_is_17_in_shadow_phase` (existing) | still passes — `SNAPSHOT_VERSION == 17`, no `world_hash.rs` diff. | no-hash |
| `snapshot_roundtrip_ignores_shadow` (existing) | still passes — skipped economy/factory_shadow come back Default; hash unchanged across round-trip. | no-hash |

---

## 8. UNKNOWN / UNCHECKED (marked, not guessed — DRIFT-default)

- **U-SHIP — Ship-vs-Vehicle routing collapse (ledger #12).** Rust has no `Ship` `ProductionCategory`; naval
  collapses into `Vehicle`, diverging MultipleFactory count and same-frame order whenever a house owns both a War
  Factory and a Naval Yard (every naval match). SURFACED, not fixed in P5a. P5b-or-later structural decision (add
  `Ship` vs accept collapse) needs user sign-off — gamemd keeps +0x53B8/+0x5388 as separate slot+count. NOT
  silently folded.
- **U-BONUS — per-category `GetBuildTimeBonus` field not parsed (ledger #2/#18).** The producer's
  `build_time_bonus_ppm` input has no backing rules field (`HouseTypeClass+0x34` per-category side multipliers).
  Stock YR default is 1.0, so stock parity holds with `build_time_bonus_ppm = PRODUCTION_RATE_SCALE`;
  modded/per-side bonuses (infantry/naval/aircraft/defense) are unmodeled. Flagged as a parser slice; P5a passes
  1.0. NOT invented. If a stock-YR side has a non-1.0 build-time bonus, that becomes a fidelity gap until wired.
- **U-ORDER — intra-frame two-Begin registration order (Lane A residual, §2.3).** The gamemd array order is
  verified temporal-append/compacting and the sweep is array-index order. NOT verified: the dispatch order of two
  Begin commands in the SAME frame (`EventClass::Execute 0x004C6CB0` → Begin_Production → ctor). `enqueue_order`
  reproduces command-dispatch order, the faithful default; if P5c's replay harness ever shows a same-frame
  two-Begin divergence, re-verify the intra-frame `EventClass::Execute` dispatch order. The one residual UNCHECKED
  on the ordering axis.
- **U-DEFENSE — `BuildCategory::Combat ⇔ +0xE08==5` (ledger #13).** The Defense-vs-Building split equivalence is
  DRIFT-default; P5a adds a pinning test but does not prove the equivalence from the binary this slice. If a
  building is Combat-categorized in Rust but `+0xE08 != 5` in gamemd (or vice versa), it routes to the wrong
  factory. Flagged for a separate verification lane.
- **U-AFFORD — affordability read==write-wallet equivalence.** The engine's per-step affordability READ goes via
  a credit sub-object (vtable+0x18); `Spend_Money` WRITES the wallet. That both reference the same wallet word is
  asserted (study §9.4 H1) but the read-slot target was not decompiled. The Rust `Economy.available() == credits`
  and `spend()` both touch the one `credits` field, so the equivalence holds in Rust by construction; flagged
  because the engine-side proof is incomplete and the inversion assert (C) relies on it.

---

## 9. Files touched (P5a)

(Anchor on TEXT; the tree shifts. A concurrent session edits miner/combat/movement/unit_post AND world/mod.rs —
design NO edits to the miner/combat/movement/unit_post files; anchor world/mod.rs edits on code TEXT, not line
numbers.)

- `src/sim/production/factory.rs` — add `build_step_time(&BuildStepTimeInputs) -> i32` + `BuildStepTimeInputs`
  (§3) and `category_for_object` (§4); change `rebuild_shadow_inner`'s `insertion_seq` derivation from the
  `seq_carry`/`next_insertion_seq` first-appearance block to front-`enqueue_order` temporal derivation (§5.2),
  retiring `seq_carry` as the ordering source; extend `mod tests` with the §7 pure-function cases. (Module is
  `#![allow(dead_code)]` — new dormant items are fine.) If `factory.rs` crosses ~600 lines, the producer +
  `BuildStepTimeInputs` may live in a sibling `factory_rate.rs` (planner's call; anchor on the function, not the
  file).
- `src/sim/production/mod.rs` — re-export `build_step_time` / `BuildStepTimeInputs` / `category_for_object`
  (mirror the existing `pub use self::factory::{...}` block that re-exports `CancelOutcome`/`StepOutcome`/etc).
- `src/sim/world/mod.rs` — add `debug_assert_factory_step_matches_legacy(&self, rules: Option<&RuleSet>)` (§5.3,
  clone-only, surface tick+owner+category, never write back) + one call line in `debug_assert_production_shadow`
  (anchor on the `self.debug_assert_factory_conservation(); // P3` text); thread `rules` from the existing
  `refresh_production_shadow(rules)` / `debug_assert_production_shadow()` tail call (fall back to the `None` arm
  if the concurrent session collides); add the `#[cfg(test)]` dormant delivery probe (§5.1). Anchor on TEXT
  (co-edited file).
- `src/sim/world/production_shadow_tests.rs` — add the world-level tests (§7): `factory_flip_prep_does_not_
  change_state_hash`, `factory_insertion_seq_equals_front_enqueue_order`, `factory_step_order_matches_legacy_
  temporal_order`, `factory_step_matches_legacy_shadow_holds`, `production_delivery_probe_is_dormant`,
  `production_flip_prep_is_deterministic`. Reuse the existing helpers; confirm
  `snapshot_version_is_17_in_shadow_phase`, `snapshot_roundtrip_ignores_shadow`, and the P2/P3/P4
  `*_does_not_change_state_hash` tests still pass.

**NOT touched:** `world_hash.rs`, `snapshot.rs` (SNAPSHOT_VERSION stays 17), `economy.rs` core
(`add_credits`/`spend`/`available` reused as-is), the legacy `production_queue.rs` / `production_tech.rs`
authoritative build-time family (they stay authoritative + DRIFT, replaced at P5b/P7 — the producer coexists
dormant), any miner/combat/movement/unit_post file (concurrent session). **Verify:** `cargo test -p vera20k`
(separate foreground pass, per the build-discipline memory) — read the literal `test result:` line; confirm
SNAPSHOT_VERSION still 17 and no `world_hash.rs` diff.

---

## 10. P5a TASK OUTLINE (for the planner to expand)

- **P5a-T1** `factory.rs` (or `factory_rate.rs`): implement `build_step_time` + `BuildStepTimeInputs` per §3.3
  (x0.9-free, per-category bonus input, Max-clamp-only-when-ratio<1 + 0.01 floor, per-iteration MF truncation,
  wall branch; NO /54, NO clamp). Pure-function tests §7 (no-×0.9 base, MTNK→rate-12, BTM, low-power clamps,
  per-iteration-MF divergence, MF gate, wall, cost-0, overflow-safe).
- **P5a-T2** `factory.rs`: add `category_for_object` delegate + `category_for_object_matches_rtti_table` /
  `category_for_object_naval_collapses_to_vehicle_documented` tests (§4).
- **P5a-T3** `factory.rs`: switch `rebuild_shadow_inner` `insertion_seq` to front-`enqueue_order` temporal
  derivation; retire `seq_carry` as the ordering source (§5.2). Confirm the existing
  `insertion_seq_stable_across_rebuild` / `factory_registry_iteration_is_insertion_ordered` tests still pass (or
  adjust to the new source if they encoded the old mint).
- **P5a-T4** `production/mod.rs`: re-export the new items (mirror the factory re-export block).
- **P5a-T5** `world/mod.rs`: add `debug_assert_factory_step_matches_legacy` (§5.3) + the one call line + the
  `rules` threading; add the `#[cfg(test)]` dormant delivery probe (§5.1). Anchor on TEXT.
- **P5a-T6** `production_shadow_tests.rs`: add the six world-level tests (§7); confirm
  `production_shadow_preserves_advance_tick_phase_order`, `snapshot_version_is_17_in_shadow_phase`,
  `snapshot_roundtrip_ignores_shadow`, and the P3/P4 no-hash tests still pass.
- **P5a-T7 (verify, separate foreground pass):** `cargo test -p vera20k` — read the literal `test result:` line;
  confirm SNAPSHOT_VERSION still 17 and no `world_hash.rs` diff.

---

*End of P5a design. The slice is additive and hash-neutral: the x0.9-free `build_step_time` producer feeds the
already-shipped `set_rate(total)` caller (legacy build-time family stays authoritative + DRIFT, retired at P5b);
`category_for_object` is a tested delegate that surfaces the Ship-collapse + per-category-bonus gaps; the C7
delivery seam is identified + proven-but-dormant (no authoritative call site); and the load-bearing de-risk —
the Lane-A temporal `insertion_seq` mint correction + the inversion-readiness assert — converts the §6.3
same-frame-ordering UNPROVEN into a proven invariant. NO serde derive is added, `economy`/`factory_shadow` stay
serde-skip, `world_hash.rs`/`snapshot.rs` are untouched, and `SNAPSHOT_VERSION` stays 17 — proven by
`factory_flip_prep_does_not_change_state_hash`. When the inversion + temporal-order asserts hold across the
suite, P5b is a near-mechanical swap: add derives, fold the hash (dropping `next_insertion_seq`, removing the
retired legacy fields), add the `step_all` + delivery call sites, fold C1, bump 17→18. P5c is the global
replay/parity gate. The Ship collapse, the per-category build-time bonus field, the intra-frame two-Begin order,
the Defense split, and the affordability read-slot are surfaced as the five open UNKNOWNs, per the burden-of-
proof default.*
