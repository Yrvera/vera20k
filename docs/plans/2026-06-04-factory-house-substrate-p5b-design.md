---
title: Factory/House Production+Economy Substrate — P5b (the authority flip) — APPROVED Design
date: 2026-06-04
status: design — APPROVED (synthesized from 3 competing lanes + 3 judge score-sets). WINNER = D-PARITY-MIN
        (minimum-hash-delta / smallest authoritative footprint), mean total 24/25 across all three judges, a
        clean sweep. Grafts the D-RISK 4-micro-step authoring order + stale-test reconciliation, and the
        D-SUBSTRATE §3.3 double-hash audit format + P5d retirement seam. Designs WITHIN the two LOCKED
        decisions (D1 drop next_insertion_seq, D2 defer Ship) + all P5 locked items. NO Rust written; NO cargo.
scope: P5b ONLY — the atomic authority flip + SNAPSHOT_VERSION 17→18 + the C1 fold. The FIRST hashed-state
       change in the whole program; the milestone; the riskiest slice. IN: serde+un-skip the five shadow types;
       hash fold (ADD Factory+Economy fields, REMOVE remaining_base_frames+progress_carry from the per-item
       fold, DROP next_insertion_seq+seq_carry FIELDS); flip the per-step charge to the REAL wallet via
       FactoryRegistry::step_all in advance_tick Phase 7 BEFORE the house tail (C1 fold); persist Factory
       progress/balance across ticks (kill the per-tick rebuild clobber); bind start_next_queued at the C7
       delivery commit; swap set_rate's input to the build_step_time producer; retire upfront-charge /
       .rev()-full-refund cancel / frames timer / the credits_entry_for_owner auto-create-house hazard.
       OUT (clean seams): P6 prereq 3-way; P7 purifier-count/IncomeMult/HarvestedCredits economy fix; P5c (P9)
       replay/parity acceptance gate; the Ship category (D2 deferred follow-up); full queues_by_owner retirement
       into Factory.queue (P5d follow-up seam).
source: docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md (C1 §5, C3, C4, C5, C6, C7, C8, C12, C15;
        §6.4 hash-set; §7 retire); docs/plans/2026-06-04-factory-house-substrate-p5-design.md (§2.2 the P5b seam,
        §5.1 C7); docs/plans/2026-06-04-factory-house-substrate-p5a-plan-review.md (CONCERN-2);
        committed P1–P5a code (live-tree reads this session, anchored on quoted text — the tree shifts;
        world/mod.rs is co-edited by a concurrent session).
verification: every current-code claim below is quoted file:TEXT from a live Read/Grep this session. Central
        divergence VERIFIED LIVE this session: active_producer_by_owner is WRITTEN by find_spawn_selection
        (production_spawn.rs:111-117), READ by the placement focus/rotation paths (production_placement.rs:88,
        production_spawn.rs:98), and HASHED at world_hash.rs:210-216 — it is a live authoritative producer-focus
        binding, NOT a throwaway-rebuild artifact. Mirror direction VERIFIED: house.economy.credits = house.credits
        (world/mod.rs:979, house→economy). Anchors VERIFIED: upfront charge (production_queue.rs:218), .rev()
        full-refund cancel (811/864 + .max(0) refunds 783/837/876), auto-create-house (86), Phase-7 head
        tick_production_with_overlay_registry (world/mod.rs:2559), house tail run_late_region (2656), tail
        refresh_production_shadow (2677), P5a inversion assert (1028), SNAPSHOT_VERSION=17 + pin test
        (snapshot.rs:24, 374-375). C3/C5/C7/C12 are VERIFIED-LIVE v2 in the study — cited, not re-decoded. cargo NOT run.
rule: Rust-native structure, gamemd-native semantics. sim/ never depends on render/ui/sidebar/audio/net. All sim
      math integer/fixed-point; EntityStore+BTreeMap keyed by InternedId; 30-player/20k scale.
---

# P5b Authority Flip — APPROVED Design

## 0. TL;DR (verdict-first)

**Flip the charge with the SMALLEST authoritative footprint: keep `queues_by_owner` as the serialized+hashed
queue-of-record and temporal-order source (`enqueue_order`), make the registry's per-step charge authoritative,
persist a MINIMAL progress set (`progress, balance, step_timer, on_hold, suspended` + identity/round-trip fields)
across ticks in the now-serialized `Factory`, and keep `house.credits` the ONE hashed wallet (charged once,
`economy.credits` demoted to a non-hashed sweep shim).** This is the lowest-risk path for the first hashed-state
change, and it is the only one of the three lanes that catches — with verified writers/readers — that
`active_producer_by_owner` is a LIVE authoritative producer-focus field the literal scope would wrongly remove
from the hash.

The load-bearing move: today `refresh_production_shadow` (world/mod.rs:1006) does `std::mem::take(&mut
factory_shadow)` + `rebuild_shadow` every tick, re-deriving `progress`/`balance` from the legacy frames timer +
cost. That **rebuild-every-tick clobber MUST stop**. P5b replaces it with a **reconcile-membership + persist** pass:
a factory whose `(type_id, enqueue_order)` front is unchanged keeps its authoritative progress UNTOUCHED (the
PERSIST arm); a new/changed front seeds `progress=0, balance=full_cost` ONCE (the SEED arm). The per-step charge
`advance_one_step(&mut Economy)` (shipped + tested P3/P5a) becomes the single writer of those fields against the
real wallet.

Authored as four bisectable micro-steps inside ONE commit (grafted from D-RISK): **M1 INVERT** (persist the
registry, proven hash-neutral with serde still skipped) → **M2 ROUND-TRIP** (serde + un-skip + hash fold) → **M3
CHARGE-FLIP** (step_all authoritative + retire legacy charge/cancel/frames atomically) → **M4 DELIVERY+C1** (bind
start_next_queued at the C7 delivery commit, fold the C1 ordering lock). One 17→18 bump.

---

## 1. The synthesis: scoring, winner, grafts

### 1.1 Aggregated ranking (mean total across 3 judges)

| Rank | Lane | Mean total | n | One-line bias |
|---|---|---|---|---|
| **1 (WINNER)** | **D-PARITY-MIN** (parity-fidelity / minimum-hash-delta) | **24 / 25 — 23.0+** | 3 | Smallest authoritative footprint; queues_by_owner stays hashed queue-of-record + temporal source; registry adds only the minimal progress set; one wallet (house.credits) charged once; uniquely catches the live active_producer hash-hole. |
| 2 | D-RISK (risk-isolation / test-first incremental) | 22.33 | 3 | Best bisectability via 4 guarded micro-steps; but flips credits authority to economy.credits with house.credits a tail mirror and HASHES BOTH — a fragile last-writer double-hash; also drops the live active_producer from the hash. |
| 3 | D-SUBSTRATE (cleanest end-state) | 21.0 | 3 | Cleanest registry-authoritative end-state + exhaustive hash table; but largest one-shot (full data-flow reversal), removes house.credits from the hash (biggest reader blast radius), and removes the still-written active_producer per scope. |

Per-judge: J-parity scored D-PARITY-MIN 24, D-RISK 22, D-SUBSTRATE 21. J-substrate scored 24/22/21.
J-risk scored 24/23/21. **D-PARITY-MIN is the unanimous winner on every judge and every axis.** Tie-break rule
("safer flip for the first hashed-state change") is moot — there is no tie; the winner is also the safest.

### 1.2 Why D-PARITY-MIN wins (the decisive divergence)

All three judges independently verified against the live tree that the literal scope-B instruction "REMOVE
active_producer_by_owner from hash" is **REFUTED**: it is written by `find_spawn_selection`
(production_spawn.rs:111-117) and `cycle_active_producer_for_owner_category` (production_placement.rs), read by the
producer-focus + exit-rotation paths (production_placement.rs:88, production_spawn.rs:98), and hashed at
world_hash.rs:210-216. **None of these writers/readers is retired by P5b** (P5b retires the *queue-advance* half of
`tick_production`; the placement-geometry + producer-focus binding STAYS — the C7 seam "placement-geometry half
stays"). Removing a still-written authoritative field from the hash is a DRIFT/hash-hole under the burden-of-proof
default. D-PARITY-MIN is the only lane that catches this and keeps the field hashed. This decision OVERRIDES the
literal scope (see §3.4 and §10).

The second decisive divergence: D-PARITY-MIN keeps `house.credits` the single hashed wallet and demotes
`economy.credits` to a non-hashed sweep shim — the **only** lane of the three with NO double-hash of the balance
and NO "last-writer discipline" caveat. D-RISK and D-SUBSTRATE both flip authority to `economy.credits`; D-RISK
then hashes BOTH `house.credits` and `economy.credits` (a fragile invariant only safe because the tail mirror is
the last writer — its own U-CREDITS-MIRROR flags this), and D-SUBSTRATE removes `house.credits` from the hash (the
largest reader blast radius on the first flip). The winner avoids both.

### 1.3 Grafts (the best concrete ideas from the runners-up, folded into the winner)

| Graft | From | Where it lands in this design |
|---|---|---|
| **4-micro-step authoring order** (M1 INVERT hash-neutral → M2 ROUND-TRIP → M3 CHARGE-FLIP → M4 DELIVERY+C1), each guarded green before the next | D-RISK | §2.1 — makes the first hashed change bisectable WITHOUT enlarging the footprint. M1 (prove the registry persists hash-neutral BEFORE any serde/hash move) is a strictly better de-risking of the winner's load-bearing PERSIST arm than single-commit staging. |
| **Stale-test reconciliation** — the P3/P4 `factory_*_does_not_change_state_hash` tests assumed serde-skip; under the flip they INVERT (the registry now DOES change the hash). They must be intentionally inverted, not silently broken. | D-RISK | §9 (reconcile list) — the winner omitted this; it is a real green-landing risk. |
| **§3.3 double-hash AUDIT format** — enumerate every field hashed in both registry and queue mirror (object.type_id vs front.type_id; insertion_seq vs front.enqueue_order) and prove the redundancy is deterministic-and-safe because the mirror is one-way-derived. | D-SUBSTRATE | §3.3 — makes the no-hole/no-double-hash claim airtight. |
| **P5d clean-seam framing** for fully retiring queues_by_owner into Factory.queue (move enqueue_order storage into the registry) — gives the winner's mirror-not-retire a named exit. | D-SUBSTRATE | §11 — the explicit follow-up that erases the mirror/registry value-redundancy. |
| **total_base_frames disposition** — D-SUBSTRATE argues it now feeds only the retired frames timer + progress bridge, so remove it too. | D-SUBSTRATE | §3.2 — RESOLVED: KEEP it hashed (it survives as the sidebar ETA basis via `effective_time_to_build_frames_for_type`; verify the reader at plan time). NOT removed in P5b — keeping it is the smaller-footprint call and the scope did not list it for removal. |

### 1.4 Grafts explicitly REJECTED (would enlarge blast radius beyond the flip)

- D-SUBSTRATE's `house.credits` removal from the hash — REJECTED. Keep `house.credits` authoritative (§3.3).
- D-RISK's / D-SUBSTRATE's `economy.credits` authority flip + tail mirror — REJECTED. One wallet, no direction-flip, no double-hash (§3.3).
- D-SUBSTRATE's `total_base_frames` removal — REJECTED for P5b (§3.2; it has a surviving sidebar reader).
- Both rivals' literal scope-B removal of `active_producer_by_owner` — REJECTED (§3.4; verified-live authoritative field).

---

## 2. THE SEQUENCING — four micro-steps inside ONE commit + how progress PERSISTS

### 2.1 The one-commit authoring order (M1→M4, each guarded green before the next)

The flip is atomic at the commit level. The committed tree is the COMPLETE flip — M1–M4 are the authoring order
within the diff, each with a falsifiable guard test that must pass on the work-in-progress tree before the next
edit is written. This is the P5a T1→T7 discipline applied to the riskier flip; it makes the first hashed change
bisectable (if M3's no-double-charge fails, you know it is the charge-flip, not the hash fold M2 already proved).

| Step | What flips | Guard test (green before next step) | Hash touched? |
|---|---|---|---|
| **M1 — INVERT** | Convert `refresh_production_shadow` from `std::mem::take`+`rebuild_shadow` (clobber) into reconcile-membership + persist (§2.2). Registry still `#[serde(skip)]`; legacy frames timer is still the authority the hash sees. | `factory_registry_persists_across_ticks_hash_neutral` — registry `progress` advances across ticks WITHOUT rebuild; `state_hash` STILL bit-identical (registry still serde-skip, so the hash sees no change). Isolates the single riskiest mechanical change — losing the rebuild — from the hash move. | **NO** (still skip) |
| **M2 — ROUND-TRIP** | serde derives on the five types; un-skip `factory_shadow` + `economy`; the `hash_production`/`hash_houses` fold (ADD Factory+Economy, REMOVE remaining_base_frames+progress_carry, DROP next_insertion_seq+seq_carry FIELDS). Legacy charge still authoritative one more step. | `snapshot_version_is_18`; `snapshot_roundtrip_factory_registry`; `production_authoritative_hash_includes_factory_fields`; `legacy_progress_carry_removed_from_hash` | **YES** (hash moves; charge unchanged) |
| **M3 — CHARGE-FLIP** | `step_all` becomes the authoritative per-step charge against the real `house.credits`; retire upfront charge + `.rev()`-full-refund cancel + frames timer atomically; close the auto-create-house hazard; swap `set_rate`'s input to the producer. | `no_upfront_charge_at_enqueue`; `single_wallet_charged_once_no_double_debit`; `cancel_one_partial_refund_to_house_credits`; `step_cadence_respects_step_rate_frames` | YES (charge path) |
| **M4 — DELIVERY+C1** | Bind `start_next_queued` at the delivery commit (replace completion→`ready_by_owner` queue-advance); place `step_all` at Phase-7 head before the house tail (C1 fold). | `reconcile_seed_arm_re_arms_on_new_front`; `c1_factories_step_before_house_tail`; `factory_flip_determinism_over_scripted_commands` | YES (ordering + delivery) |

**Safe-order invariant (the never-uncharged rule):** within the commit, ADD the real charge (M3 step_all live)
BEFORE removing the legacy charge, so no built revision is double-charged or uncharged. The detailed retirement
order is §6.2.

### 2.2 How progress/balance PERSIST (the replacement for the std::mem::take rebuild) — M1

**Today (world/mod.rs:1006 `refresh_production_shadow`, anchor `let mut registry = std::mem::take(&mut
self.production.factory_shadow);`):**
```
self.refresh_economy_shadow(rules);
let mut registry = std::mem::take(&mut self.production.factory_shadow);
match rules { Some(r) => registry.rebuild_shadow(self, r), None => registry.rebuild_shadow_no_rules(self) }
self.production.factory_shadow = registry;
```
`rebuild_shadow_inner` (factory.rs) builds a FRESH `BTreeMap` every tick and seeds each factory's `progress` from
the legacy frames timer and `balance` from `remaining_balance_after(full_cost, progress)`. Once the registry is
authoritative, that re-derivation would clobber the authoritative progress every tick.

**P5b (the minimum-footprint reconcile):** split the rebuild into a **reconcile** pass that PRESERVES the surviving
factory's progress fields and only syncs membership + the queue tail + the object identity:
```
fn reconcile_from_queues(&mut self, sim_queues, rules):
  for (owner, category, queue) in queues_by_owner:               // BTreeMap<owner>→BTreeMap<category>, deterministic
    front = queue.front()  (skip if empty)
    key = (owner, category)
    seq  = front.enqueue_order                                   // temporal insertion_seq (D1; P5a mint, unchanged)
    match self.factories.get_mut(&key):
      Some(f) if f.object.is_some()
                 && f.object.type_id == front.type_id
                 && f.insertion_seq == seq:                      // SAME build still active
          // PERSIST: do NOT touch progress/balance/step_timer/on_hold/suspended.
          f.queue   = tail(queue)                                // refresh the FIFO tail only (cancel/enqueue may change it)
          f.manual  = (front.state == Paused)                   // §2.3 pause bridge (the one front.state read kept)
          // object/progress are authoritative-in-registry now; do NOT overwrite from front.state.
      _otherwise:                                                // NEW build OR a changed front (delivery advanced / cancel)
          // a fresh active object begins: seed ONCE from cost, progress 0.
          insert/replace Factory { owner, category, object: Some(front.type_id),
                                   progress: 0, balance: full_cost, original_balance: full_cost,
                                   step_rate_frames: 0, step_timer: 0, on_hold:false, suspended:false,
                                   manual:(front.state==Paused), special:NoneNeg1, queue: tail, insertion_seq: seq }
  self.factories.retain(|key,_| queues_by_owner has a non-empty queue at key)   // drop emptied factories
```

The two arms ARE the whole design:
- **PERSIST arm** (same `(type_id, enqueue_order)` front as last tick): the active build is unchanged, so its
  authoritative `progress`/`balance`/`step_timer`/`on_hold`/`suspended` carry forward UNTOUCHED. Only the FIFO tail
  + the pause bridge are refreshed. This is the common case every stepping tick.
- **SEED arm** (no factory, or the front changed identity): a new active object begins — seed `progress=0`,
  `balance=original_balance=full_cost` ONCE. Fires exactly when gamemd would Begin a fresh object: first enqueue, or
  after a delivery/cancel advanced the FIFO front to a new `enqueue_order`.

**Why `(type_id, enqueue_order)` is the right identity test:** `enqueue_order` is strictly monotonic
(production_queue.rs `next_enqueue_order` saturating_add from 1), so a delivered-then-restarted `(owner,category)`
gets a NEW front with a HIGHER `enqueue_order` → the test fails → the SEED arm re-arms. Two MTNK in a row are two
distinct builds with distinct `enqueue_order` (NOT distinguished by `type_id` alone). This is the faithful analog
of gamemd's destroy-recreate → array-tail-re-append (C7).

**Crucial subtlety:** in the legacy world the frames timer (`remaining_base_frames`) drives progress and
`front.state` carries Building/NoFunds/Paused/Done. After the flip the registry's `progress`/`on_hold`/`suspended`
are authoritative and the frames timer is RETIRED (§6) — so the reconcile must NOT read `front.state` for
progress/hold/suspend. `front.state` remains hashed (it is the sidebar's queued/building label and still drives
`refresh_queue_states`), but it is no longer the progress source. The ONE place the reconcile honors `front.state`
is **Paused** (§2.3).

### 2.3 Pause/resume bridging (keeps the legacy pause command working without re-authoring it)

`toggle_pause_for_owner_category` (production_placement.rs) flips `front.state` Building↔Paused — the live pause
command, NOT re-authored by P5b. The reconcile bridges it: `f.manual = (front.state == Paused)` (PERSIST arm
included — pause can toggle mid-build). `advance_one_step`'s ARMED GATE already idles a `manual` factory without
losing progress (returns `Idle` before touching `progress`). One-line bridge, not a new command path. (Prereq-driven
resume — C9 — is P6's job and OUT of P5b; a manual unpause simply clears `f.manual` next reconcile.)

---

## 3. THE QUEUES_BY_OWNER RELATIONSHIP + THE EXACT HASH FIELD SET

### 3.1 Authority roles (minimum-footprint)

| State | Role after P5b | Hashed? | Why |
|---|---|---|---|
| `queues_by_owner` (VecDeque per owner/category) | **AUTHORITATIVE-OF-RECORD** for FIFO membership + temporal order | YES (already) | Queue-of-record / cancel target / `enqueue_order` temporal source (C6). Unchanged container. The flip does NOT move ordering/membership/cancel-target authority into the registry. |
| `BuildQueueItem.{owner, type_id, queue_category, state, total_base_frames, enqueue_order}` | queue identity + sidebar label + ETA basis + temporal stamp | **YES (keep)** | type/category/state/enqueue_order stay hashed. `total_base_frames` STAYS (it is the sidebar ETA basis via `effective_time_to_build_frames_for_type`; verify the reader at plan time — §3.2). |
| `BuildQueueItem.{remaining_base_frames, progress_carry}` | **RETIRED** (the frames timer; progress now lives in `Factory`) | **NO (REMOVE)** | Frames timer retired (§6); progress is `Factory.progress`. Leaving them hashed would hash a dead, no-longer-advanced field (world_hash.rs:198-199). |
| `Factory.{progress, balance, step_timer, on_hold, suspended}` | **AUTHORITATIVE-PERSISTENT** (the charge engine's state) | **YES (ADD)** | The per-step charge writes these; they persist across ticks (§2.2). The minimal progress set. |
| `Factory.{owner, category, object(type_id+entity_id), queue, insertion_seq, original_balance, step_rate_frames, manual, special}` | identity / round-trip / cancel-refund basis / temporal key / ARMED-GATE inputs | **YES (ADD)** | Needed for the round-trip AND they feed authoritative behavior: `original_balance` is the C8 refund basis; `insertion_seq` is the sweep order (D1); `manual` gates stepping; `special` is the 0/-1 discriminant; `object.entity_id` becomes non-None at delivery; `step_rate_frames` is the cadence gate. All authoritative, not derived. |
| `Economy.{spent_credits, harvested_credits, purifier_count}` | hashed statistics / accumulators | **YES (ADD)** | `spent_credits` written by `spend`; harvested/purifier wired in P7. Hash them so a desync is caught and they round-trip. |
| `Economy.credits` | **DEMOTED — no longer the wallet** | **NO (do NOT hash)** | `house.credits` is the one wallet (§3.3). Hashing both would double-represent the balance and invite the derived/authoritative ambiguity the scope warns against. Keep the field for the `add_credits`/`spend`/`available` API but route the charge through `house.credits`; do NOT hash `economy.credits`. |
| `house.credits` | **AUTHORITATIVE WALLET** (unchanged) | YES (already, `hash_houses`) | The single wallet integer. The charge debits it; cancel refunds it; deposit credits it. No change to `hash_houses`'s credits line. |
| `active_producer_by_owner` | sidebar producer-focus / exit-rotation binding (NOT progress) | **KEEP hashed** — see §3.4 | **OVERRIDES literal scope-B.** Verified-live authoritative field (production_spawn.rs:111-117 write, production_placement.rs:88 read, world_hash.rs:210 hash); NOT retired by P5b. Removing it = a hash hole. KEEP it hashed; defer its removal to the producer-focus retirement slice. |
| `next_insertion_seq`, `seq_carry` (FactoryRegistry fields) | **DROP + remove the fields** (D1) | **NO** | After the P5a temporal mint, `insertion_seq = front.enqueue_order`; the counter is dead. Remove both fields (revises STUDY §6.4 + P5a-review CONCERN-2 — §3.5). |
| `next_enqueue_order` (ProductionState) | the monotonic stamp source — KEEP | YES (already, world_hash.rs:217) | Mints `enqueue_order`; must round-trip so post-load enqueues don't collide. Unchanged. |

### 3.2 `hash_production` — exact ADD / REMOVE (anchored on the live world_hash.rs text)

Live `hash_production` (world_hash.rs) folds, per queue item (lines 193-200): `owner, type_id, queue_category,
state, total_base_frames, remaining_base_frames, progress_carry, enqueue_order`; then `ready_by_owner` (204-208);
then `active_producer_by_owner` (210-216); then `next_enqueue_order` (217); then resources/ore/terrain/docks.

**REMOVE from the per-item fold:** `item.remaining_base_frames` (line 198), `item.progress_carry` (line 199) —
frames timer retired.
**KEEP** `item.total_base_frames` (line 197) — it survives as the sidebar ETA basis (verify
`effective_time_to_build_frames_for_type` still reads it at plan time; if a live reader exists, keep it hashed and
say so — §10 U-QFRAMES). This REJECTS D-SUBSTRATE's removal (smaller footprint; not in the scope's remove list).
**KEEP** `owner, type_id, queue_category, state, enqueue_order`; KEEP `ready_by_owner`; KEEP the
`active_producer_by_owner` block (lines 210-216) per §3.4; KEEP `next_enqueue_order`; KEEP all resource/ore/
terrain/dock folds.

**ADD a `hash_factory_registry` fold** (new helper called from `hash_production`, iterating
`self.production.factory_shadow.iter_insertion_ordered()` so the fold order is the deterministic temporal sweep
order — NOT BTreeMap key order — because the fold order is part of the hash contract and must equal the order the
charge runs in):
```
for f in factory_shadow.iter_insertion_ordered():
    f.owner.hash; (f.category as u8).hash; f.insertion_seq.hash;
    f.progress.hash; f.step_rate_frames.hash; f.step_timer.hash;
    f.balance.hash; f.original_balance.hash;
    match f.object { Some(o)=>{1u8.hash; o.type_id.hash; match o.entity_id {Some(e)=>{1u8;e.hash} None=>0u8}} None=>0u8 }
    f.on_hold.hash; f.suspended.hash; f.manual.hash;
    match f.special { NoneNeg1=>0u8; NoneZero=>1u8; Item(v)=>{2u8.hash; v.hash} }   // 0/-1 distinct — NEVER collapse
    f.queue.len().hash; for t in &f.queue { t.hash }
```
Use explicit-field folding (the project's `category as u8` / Option presence-tag idiom in `hash_entities`/
`hash_mission_com`), NOT `#[derive(Hash)]`, so `SpecialItem`'s three states fold distinctly and `entity_id`'s
Option is a presence tag — consistent with the rest of `world_hash.rs`.

`step_rate_frames` IS hashed: it is authoritative (set_rate writes it from the producer; it persists across ticks
and gates the cadence) and must round-trip.

### 3.3 `hash_houses` + the credits/economy authority (no double-charge, no double-hash, no ambiguity)

`hash_houses` currently folds `house.credits` (+ side/flags/counts/rally/base). **Keep `house.credits` EXACTLY
as-is** — it is THE wallet, charged once by `step_all`. **ADD an `economy` sub-fold for the STATISTICS fields only:**
```
house.economy.spent_credits.hash; house.economy.harvested_credits.hash; house.economy.purifier_count.hash;
// do NOT hash house.economy.credits — house.credits is the authority.
```

**The double-charge resolution, stated crisply:** there is exactly ONE debit per step, into `house.credits`, via
`step_all`. `Economy.credits` is no longer mirrored from `credits` (DELETE the `house.economy.credits =
house.credits` line at world/mod.rs:979), no longer the spend source, and not hashed — so there is no second
balance that could disagree or be charged. This is the single-wallet answer; NO direction-flip (rejecting D-RISK's
`house.credits = economy.credits`), NO double-hash (rejecting D-RISK's hash-both), NO `house.credits` removal
(rejecting D-SUBSTRATE).

**§3.3 double-hash AUDIT (grafted from D-SUBSTRATE) — proving no field is hashed twice:**

| Field hashed in registry | Field hashed in mirror | Same value? | Safe (no double-hash of one authority)? |
|---|---|---|---|
| `Factory.object.type_id` | `BuildQueueItem.type_id` (front) | equal-valued for the front item | YES — distinct ROLES (active-object identity vs queue-of-record front); the mirror is one-way derived (queues_by_owner is authoritative-of-record; the registry's object is reconciled FROM the front in the SEED arm). They cannot disagree → hashing both is redundant-but-consistent, not a desync hazard. Pins both the queue front AND the resolved active object. |
| `Factory.insertion_seq` | `BuildQueueItem.enqueue_order` (front) | equal by construction (`insertion_seq = front.enqueue_order`, the P5a mint) | YES — distinct ROLES (per-factory sweep key vs per-item temporal stamp). Pinned equal by `factory_insertion_seq_equals_front_enqueue_order`. Hashing both catches a mint regression. (If strict no-redundancy is ever required, `insertion_seq` could be dropped from the hash as a pure function of `front.enqueue_order` — but it is KEPT as a regression guard.) |
| `Factory.progress` | (nothing — `remaining_base_frames` REMOVED) | n/a | YES — progress hashed exactly once. |
| `Factory.balance` | (nothing) | n/a | YES — once. |
| `house.credits` | (nothing — `economy.credits` NOT hashed) | n/a | YES — the balance hashed exactly once. |

The only value-redundancy is registry↔mirror on `type_id`/`enqueue_order`, and it is DETERMINISTIC (the registry is
reconciled one-way from the queue-of-record, so they can never disagree) and HASH-SAFE. It is the explicit price of
mirror-not-retire, surfaced here, cleanly erased by the P5d seam (§11). No field of any single authority is hashed
twice.

### 3.4 active_producer_by_owner — KEEP hashed (OVERRIDES literal scope-B) — verified-live DRIFT-default

`active_producer_by_owner` is **not** a progress field and **not** a throwaway. Verified live this session:
- WRITTEN by `find_spawn_selection_for_owner_with_type` (production_spawn.rs:111-117: `…active_producer_by_owner
  .entry(owner_id).or_default().insert(queue_category, first.0)`) — seeds the focused producer; and by
  `cycle_active_producer_for_owner_category` (production_placement.rs) — rotates it.
- READ by the exit-rotation + sidebar-focus paths: production_placement.rs:88 (`active_producer_by_owner.get(&id)`
  → selects which physical building the cameo/exit focuses) and production_spawn.rs:98 (`get(&owner_id)…rotate_left`
  → exit-cell rotation).
- HASHED at world_hash.rs:210-216.

**The legacy spawn/placement path that writes it is NOT retired by P5b** — P5b retires the *queue-advance* half of
`tick_production`; the placement geometry, including this focus binding, STAYS (the C7 seam "placement-geometry
half stays"). Removing a still-WRITTEN, still-authoritative field from the hash is a DRIFT unless it is also retired
or proven derived. **None of the three lanes proves it derived.** Under the burden-of-proof default, the honest
verdict is: **KEEP `active_producer_by_owner` hashed in P5b** (it is orthogonal to the charge flip — leaving it
hashed has zero interaction with the registry and keeps the round-trip honest). Scope-B's "remove" was premised on
it being a throwaway-rebuild artifact, which it is NOT.

**RESOLVED for the plan:** scope-B line "REMOVE active_producer_by_owner from hash" is **deferred to whichever later
slice retires the producer-focus binding** (P6/placement cleanup), NOT bundled into the charge flip. **Scope-F test
`legacy_active_producer_removed_from_hash` is DROPPED from the P5b test list** (it conflicts with reality and would
codify a hash hole). All three judges concur with this override. (If a future lead insists on removing it, it needs
its own determinism proof — a test that the producer-focus binding is reconstructed identically post-load, which
does NOT exist today.)

### 3.5 next_insertion_seq drop REVISES STUDY §6.4 (resolves CONCERN-2) — explicit

D1 drops `next_insertion_seq` + `seq_carry` and removes the FIELDS. After the P5a temporal mint
(`insertion_seq = front.enqueue_order`, factory.rs `rebuild_shadow_inner`), the counter is never the ordering
source and is dead. **This REVISES STUDY §6.4** (which listed adding `next_insertion_seq` to the hashed/serialized
set) **and the original P5b seam in the P5 design §2.2** (which planned hashing the counter) — **resolving P5a
plan-review CONCERN-2.** Stated explicitly so the contradiction is surfaced, not shipped silently. The flip KEEPS
the per-queue `enqueue_order` (and queue membership) in the hashed/serialized set as the temporal ordering source —
the lower-risk path P5a already proved hash-neutral. The originally-planned test
`registry_next_insertion_seq_is_serialized_and_hashed` is SWAPPED for the already-passing
`factory_insertion_seq_equals_front_enqueue_order`.

---

## 4. THE step_all PLACEMENT (C1 fold) + the rate swap + the wallet adapter — M3/M4

### 4.1 Where in advance_tick Phase 7 (C1 fold)

Verified live `advance_tick` Phase 7: `tick_production_with_overlay_registry` (world/mod.rs:2559) → `tick_repairs`
→ `tick_building_docks` → … → ore growth; then AFTER the block: `run_late_region` (the house tail —
defeat/AI/anims, 2656); then the tick tail `refresh_mission_shadow` → `refresh_production_shadow` (2677) → asserts
→ `state_hash`.

**C1 requires every factory to step BEFORE any house tick** (study C1 §5: PerTickUpdate factory loop precedes the
house loop). `run_late_region` is the house tail. **Place `FactoryRegistry::step_all` at the START of Phase 7's
production work — the FIRST thing in the production block, before the (now charge-free)
`tick_production_with_overlay_registry` placement/spawn pass, unconditionally before `run_late_region`.**
Concretely, replace the charge-bearing role of `tick_production_with_overlay_registry` with:
```
// Phase 7, first production step — the authoritative factory sweep (C1: factories step before the house tail).
self.production.factory_shadow.step_all(&mut self.houses, rules);   // reconcile ran last tick's tail; charge now
spawned_entities |= production::tick_production_with_overlay_registry(self, rules, height_map, path_grid,
                                                                      overlay_registry, tick_ms);  // spawn/placement only — charge removed
```

**The cadence wrinkle, resolved precisely:** today the reconcile (`refresh_production_shadow`) runs at the TICK
TAIL (after all systems), so at the START of the next tick's Phase 7 the registry membership already reflects last
tick's settled queues. That is the correct cadence for `step_all`: the tail reconcile at tick N prepares the
registry; `step_all` at tick N+1's Phase-7 head charges it; the delivery/spawn pass + the C7 `start_next_queued`
mutate `queues_by_owner`; the tail reconcile folds those mutations back in. **Keep the reconcile at the tail** (§2.2
replaces its BODY, not its POSITION at 2677) and **add `step_all` at Phase-7 head** (before 2559). This gives the
gamemd ordering: registry settled → factories step (charge) → delivery advances queue → reconcile.

`step_all` iterates the registry in `iter_insertion_ordered()` (temporal `insertion_seq`) order and calls
`advance_one_step(wallet)` once per factory per cadence-tick (the `step_timer` countdown that gates whether THIS
tick steps is §4.4). This reproduces gamemd PerTickUpdate walking the factory array in registration order (C1),
then the house tail (`run_late_region`) runs after — the C1 fold, in one 17→18 bump (the locked fold). The
`EntityCategory::Structure` arm of `object_ai_stage`/`techno_ai_shell` STAYS no-op (LOCKED; techno_ai.rs
`EntityCategory::Structure => {}`) — `step_all` is a standalone Phase-7 registry step, NOT per-building dispatch
(study FIT-(a) explicitly NOT taken; justified by same-frame-completion output parity).

### 4.2 The wallet adapter (the &mut Economy boundary — smallest form)

The shipped `advance_one_step(&mut self, economy: &mut Economy)` takes `&mut Economy`. The flip makes the wallet
`house.credits` (an `i32` on `HouseState`). The minimum adapter: `step_all` borrows `&mut self.houses`, and for
each factory steps it against an `Economy` whose `credits` is loaded from `house.credits` at sweep entry and stored
back after:
```
// step_all is a method on a std::mem::take-n registry (the borrow-checker pattern proven in refresh_production_shadow):
let mut order: Vec<(u64, InternedId, ProductionCategory)> =
    self.factories.iter().map(|(&(o,c), f)| (f.insertion_seq, o, c)).collect();
order.sort_by_key(|&(seq, _, _)| seq);            // stable; ties impossible (enqueue_order strictly monotonic)
for (_, owner, category) in order {
    let Some(f) = self.factories.get_mut(&(owner, category)) else { continue };
    let Some(house) = houses.get_mut(&owner) else { continue };   // defensive: skip a vanished house (NO auto-create)
    let mut wallet = std::mem::take(&mut house.economy);          // economy holds spent/harvested/purifier
    wallet.credits = house.credits;                              // load the authoritative balance into the spend-API shim
    // (set_rate from the producer here — §4.3 — then the cadence gate — §4.4 — then:)
    let _ = f.advance_one_step(&mut wallet);                     // spends wallet.credits; accumulates wallet.spent_credits
    house.credits  = wallet.credits;                            // store the debited balance back to the ONE wallet
    house.economy  = wallet;                                    // keep spent_credits/etc.
}
```
This keeps `advance_one_step`'s BODY UNCHANGED (the locked "P5b flips WHO is passed, not the algorithm" — the
per-step `balance/steps_left` charge, strict-`<` stall rewind, completion-suspends-with-object-held). It routes the
debit through `house.credits` (the single wallet) and lets `spent_credits` accumulate in `economy`.
`economy.credits` is a transient shim inside the sweep, never the authority, never hashed (§3.3). Borrow-checker
note for the plan: `step_all` must be a method on a `std::mem::take`-n registry (mirroring how
`refresh_production_shadow` already takes the registry out at world/mod.rs:1006 to satisfy the borrow checker) so
it does not double-borrow `&self.production.factory_shadow` while holding `&mut self.houses`.

### 4.3 The rate swap

`set_rate` already takes the build-step TOTAL and owns `/PRODUCTION_STEPS + clamp[STEP_RATE_MIN, STEP_RATE_MAX]`
(factory.rs `set_rate`, verified). P5b feeds it the `build_step_time(&BuildStepTimeInputs)` producer (shipped +
tested in P5a) instead of the legacy frames math. In the sweep (or the SEED arm when an object is freshly armed)
gather `BuildStepTimeInputs` from `rules` + the owner's `power_states` ratio + the per-category factory count (from
the registry key count, retiring `matching_factory_count_for_owner`'s full-store rescan) and call
`f.set_rate(build_step_time(&inputs))` before stepping. The producer is x0.9-free (v2 correction honored); the
legacy `production_tech` build-time family is DRIFT and is one of the retirements (§6) once nothing reads it. The
producer's `factory_count` is the per-category count; the Ship-collapse means naval folds into the Vehicle count —
the D2 known DRIFT, pinned by the existing regression test, NOT fixed here (§10 U-SHIP).

### 4.4 One step per cadence-tick vs the step_rate_frames timer

gamemd steps a factory when its per-step CDTimer expires (every `step_rate_frames` frames), not every tick.
Minimum-footprint model: `step_all` decrements `f.step_timer` each tick; when it reaches 0, call `advance_one_step`
and reset `step_timer = step_rate_frames`. `step_timer`/`step_rate_frames` are hashed (§3.2), so the countdown
round-trips. This is the faithful cadence; charging every tick regardless of rate would complete a build in
`PRODUCTION_STEPS` ticks instead of `PRODUCTION_STEPS × step_rate_frames` frames (a player-visible cadence DRIFT).
**The plan must wire the `step_timer` countdown in `step_all`** — it is the one piece of new per-tick logic the
flip adds beyond the shipped `advance_one_step` (which does NOT itself consume `step_timer`; the countdown gate is
`step_all`'s responsibility). The exact CDTimer reset semantics (reset-to-rate vs reset-to-rate-minus-overshoot)
are §10 U-STEPRATE; model reset-to-rate and flag for a C5-adjacent spot-check if the determinism test surfaces a
cadence drift.

---

## 5. THE DELIVERY BIND (C7) — start_next_queued at the delivery commit; what it replaces — M4

**C7 (verified-live v2; cited from the study, not re-decoded):** queue advance is bound to the successful delivery
command (FUN_004FAA10 post-delivery `StartNextQueued`), NOT the completion tick. `CompletedProduction` has no
begin/next call; the factory holds its object (progress at completion, suspended) until the delivery command clears
it. The shipped `start_next_queued` (factory.rs, `pub(crate)`, front-pop + held-object guard) is the bind point.

**The legacy path it replaces:** in `tick_production_with_overlay_registry`, when the frames timer hits 0 the item
is marked `Done` and (for buildings) pushed to `ready_by_owner` + `pop_completed_front`; for units it spawns then
`pop_completed_front`. That **completion→pop / completion→ready_by_owner** half is the queue-advance the flip
retires. **The placement geometry STAYS:** `find_spawn_selection_*`, `spawn_object`, helipad reserve, rally-move,
and `place_ready_building` (the building placement path) are untouched.

**P5b wiring** — on a successful delivery (a unit spawned, or a building placed via `place_ready_building`):
1. clear the producing factory's `object` (`f.object = None`) — optionally stamp `entity_id` (the delivered child;
   left None for minimum footprint, hashed as the 0u8 presence tag — §10 U-ENTITYID);
2. call `f.start_next_queued()` — front-pop the next FIFO type into a fresh `object`, `progress=0`;
3. the next tick's reconcile SEED arm seeds the new `balance` from cost (the front's `enqueue_order` is now higher,
   so the reconcile identity test fails → SEED — §2.2).

The completion→`ready_by_owner` push for **buildings** STAYS as the "awaiting placement" signal (a building
completes held, the player places it — C12); on `place_ready_building` success, the delivery bind fires. For
**units**, delivery is the spawn itself (immediate); the bind fires at spawn success, replacing
`pop_completed_front`. This is the C12 split (factory-complete vs delivery) made authoritative: the factory holds
the object at completion (`suspended` + object attached, the shipped `advance_one_step` completion state) until
delivery clears it.

**Same-frame correctness (C7 abandon path):** the post-AbandonProduction auto-`StartNextQueued` (C7: the same path
fires on cancel-with-remaining-queue) binds at the SAME seam — `cancel_one` / the cancel command, after refund,
calls `start_next_queued` if a queue tail remains. Wiring the post-cancel `start_next_queued` there keeps abandon
and delivery on one bind point.

---

## 6. THE RETIREMENTS (table) + the safe order — M3/M4

### 6.1 Retirement table (anchored on quoted text)

| Legacy (file:symbol — quoted anchor) | Replacement | Micro-step |
|---|---|---|
| **Upfront charge** — `enqueue_by_type`: `*credits_entry_for_owner(sim, owner) -= obj.cost;` (production_queue.rs:218) | NO upfront debit. The per-step `advance_one_step` charges `⌊balance/(PRODUCTION_STEPS − step)⌋` into `house.credits` over the build (C3/C15). `enqueue_by_type` only checks affordability (the can-afford-to-START gate stays, the C20 begin precondition) + appends the queue item. | M3 — remove the `-=` AFTER `step_all` is live. |
| **.rev()+full-refund cancel** — `cancel_by_type_for_owner`: `.rev()` last-match (production_queue.rs:811) + `*credits_entry_for_owner(sim, owner) += obj.cost.max(0)` (783/837) | The shipped `FactoryRegistry::cancel_one`: FIRST-match front-to-back queued removal (C6), or active-abandon with **partial** refund `original_balance − balance` (C8) into `house.credits`. Route the cancel COMMAND to `cancel_one`; for a queued-tail cancel also remove the matching `queues_by_owner` item (keep the queue-of-record in sync); after refund, call `start_next_queued` if a tail remains (C7 abandon). The `.rev()` full-refund is the verified DRIFT — its removal is the C8/C15 fix. | M3 |
| **Completed-building full-refund** — `cancel_completed_building_from_ready`: `.rev()` (864) + `+= obj.cost.max(0)` (876) | Completed builds are now `Factory` complete-held objects; cancel routes through the registry path (placement geometry stays). | M4 |
| **Frames timer** — `tick_production_with_overlay_registry` PPM: `advance_queue_item` (`remaining_base_frames` decrement) + completion→`ready_by_owner`/`pop_completed_front` | `advance_one_step` (progress/charge) + `step_timer` countdown (§4.4) + the C7 delivery bind (§5). The PLACEMENT/SPAWN half of `tick_production_with_overlay_registry` STAYS. | M3 (charge/frames half) + M4 (completion→ready half) — AFTER `step_all` + the C7 bind are live. Then remove `remaining_base_frames`/`progress_carry` from the hash (§3.2). |
| **`credits_entry_for_owner` auto-create-house hazard** — `sim.houses.insert(… HouseState::new(… is_human=true …))` (production_queue.rs:86) | The charge no longer flows through this getter. After upfront-charge + full-refund are retired, the remaining callers are spawn-fail refund paths; replace those with a `houses.get_mut(&owner)` that NO-OPs if the house is absent. Fabricating an `is_human=true` house mutates HASHED state (`hash_houses`) — closing this is a correctness fix, not cleanup. | M3 — retire (or make non-fabricating) LAST, after both charge paths are off it. |
| **Legacy build-time family** — `production_tech` build-time math + `matching_factory_count_for_owner` full-store rescan | The `build_step_time` producer (§4.3); `factory_count` from the registry key count. | M3 — once nothing reads them. |
| **The mirror line** — `house.economy.credits = house.credits;` (world/mod.rs:979) | DELETE it. `economy.credits` is no longer mirrored (§3.3). `refresh_economy_shadow`'s purifier-count pass STAYS (a legit per-tick recompute, untouched). | M3 |
| **The rebuild** — `rebuild_shadow`/`rebuild_shadow_inner`/`rebuild_shadow_no_rules`/`remaining_balance_after` (factory.rs) | `reconcile_from_queues` (the persist+membership reconcile, §2.2). The registry persists. | M1 |

**Safe-order invariant:** never leave a tick where nothing charges. Order: `step_all` live (M3, charges) → retire
upfront → retire full-refund cancel (route to `cancel_one`) → bind C7 delivery (M4) → retire frames timer → close
the auto-create hazard → fold the hash → bump version. Each step is compilable; the determinism test (§8) brackets
the whole commit.

---

## 7. THE SNAPSHOT BUMP (17→18) + serde + round-trip — M2

- **serde derives:** add `Serialize, Deserialize` to `Economy` (economy.rs), `Factory`, `FactoryRegistry`,
  `PendingObject`, `SpecialItem` (factory.rs). `SpecialItem` is a 3-variant enum (`NoneNeg1`/`NoneZero`/`Item(u32)`)
  — derive serde directly; the 0/-1/Item three-state stays distinct (NEVER collapse — v2 locked).
  `StepOutcome`/`CancelOutcome`/`BuildStepTimeInputs`/`BuildEligibility` are transient return/input types — NOT
  stored, so NO serde. `VecDeque<InternedId>` and `BTreeMap<key, Factory>` serialize once their elements do.
- **un-skip:** remove `#[serde(skip)]` from `HouseState.economy` (house_state.rs) and `ProductionState.factory_shadow`
  (production_types.rs). Remove the `next_insertion_seq` + `seq_carry` FIELDS from `FactoryRegistry` (D1) — so they
  neither serialize nor hash, and remove the `new_carry`/`next_insertion_seq` writes in the (now-retired) rebuild.
- **`SNAPSHOT_VERSION` 17→18** (snapshot.rs:24) + flip the pin test `snapshot_version_is_17_in_shadow_phase`
  (snapshot.rs:374) → `snapshot_version_is_18`. Update the doc-comment history line: "17→18: Factory/Economy
  authority flip — registry+economy now serialized+hashed; frames-timer per-item fields (remaining_base_frames/
  progress_carry) removed from the hash; next_insertion_seq+seq_carry fields removed; C1 ordering lock folded in."
- **round-trip:** the registry must serialize→deserialize to identical content AND the post-load reconcile must NOT
  perturb it. The PERSIST arm (§2.2) is the guarantee: a loaded factory whose `(type_id, enqueue_order)` matches
  its queue front is left untouched by the first post-load reconcile, so `round_trip_preserves_state_hash` holds.
  Verify `rebuild_caches_after_load` (if it exists) does NOT re-run the retired rebuild over the loaded
  authoritative registry. Add the focused `snapshot_roundtrip_factory_registry` (§9).

---

## 8. DETERMINISM / REPLAY — preserving lockstep across the bump; the P5c seam

**The flip INTENTIONALLY breaks the no-hash contract (that IS the flip).** It preserves lockstep/replay
determinism across the 17→18 bump:
- **No RNG in the charge path.** `advance_one_step`, `step_all`, `set_rate`, `build_step_time`, `cancel_one`,
  `start_next_queued`, and the reconcile are integer/`i128`-only, no `f32`/`f64` in committed math. No
  `scenario_rng`/`main_rng` touch.
- **Deterministic iteration only.** `step_all` rides `iter_insertion_ordered()` (a STABLE sort over
  `BTreeMap::values()` by `insertion_seq`); `insertion_seq = front.enqueue_order` is strictly monotonic
  (`next_enqueue_order` saturating_add from 1) → **no ties → total order → deterministic sweep**. The hash fold uses
  the SAME `iter_insertion_ordered()` order, so charge order == fold order. Reconcile iterates `queues_by_owner`
  (BTreeMap<owner>→BTreeMap<category>, deterministic). No `HashMap`, no fixed-size player array.
- **One wallet, charged once** (§3.3) — no double-debit, no order-dependent dual-balance.
- **Snapshot determinism:** the PERSIST-arm reconcile (§7) keeps a loaded sim bit-identical to its pre-save self.

**Near-term P5b determinism guard:** `factory_flip_determinism_over_scripted_commands` — two `Simulation`s run
`advance_tick` over the SAME scripted command stream (enqueue several types across ≥2 owners and ≥2 categories incl.
a same-tick two-Begin, a cancel-one, run enough ticks for ≥1 completion+delivery) and MUST produce an identical
per-tick `state_hash()` sequence. This is the flip's own guard, distinct from the full P5c replay/parity gate.

**The P5c (P9) seam — leave CLEAN, do NOT design internals:** P5c is the global replay/parity ACCEPTANCE gate
(study §8 P9): a recorded command stream (begin/suspend/cancel-one/cancel-all/place/deposit) replayed twice AND
against a pre-flip baseline yields bit-identical per-tick `state_hash()`, plus `economy_conservation_over_replay`
(C15 global), plus pre-flip-baseline-vs-post-flip observable-output equivalence (the x0.9-free producer correction
documented as the ONE intended difference, not a regression). It reuses the existing replay harness. P5b leaves it
a seam: the determinism guard above is the near-term proxy; P5c is the acceptance gate that ratifies the flip. (If a
same-frame two-Begin ever diverges, the intra-frame command-execute dispatch order is the place to re-verify —
§10 U-ORDER — but P5b's strictly-monotonic `enqueue_order` makes two same-tick Begins distinct, so the sweep order
is total.)

---

## 9. THE P5b TEST LIST (scope F + grafted reconciliation + lane guards), each tied to a contract

**Scope F (required, with the §3.4 override applied):**
1. `production_authoritative_hash_includes_factory_fields` — mutating each newly-hashed `Factory` field (progress,
   balance, step_timer, on_hold, suspended, object, insertion_seq, original_balance, step_rate_frames, manual,
   special, queue) changes `state_hash()`; mutating `economy.{spent_credits, harvested_credits, purifier_count}`
   changes it. (C12/C15.) **[M2]**
2. `snapshot_version_is_18` — pins `SNAPSHOT_VERSION == 18` (replaces the 17 pin at snapshot.rs:374). **[M2]**
3. `snapshot_roundtrip_factory_registry` — a sim with a mid-build factory (progress>0, balance>0, a queue tail)
   saves→loads→ identical `state_hash()`, and the loaded registry's progress/balance survive a post-load reconcile
   unchanged (the PERSIST arm). (C15; §7.) **[M2]**
4. `legacy_progress_carry_removed_from_hash` — mutating `remaining_base_frames`/`progress_carry` on a queue item
   does NOT change `state_hash()` (they are out of the fold; progress lives in `Factory`). (frames timer retired.)
   **[M2]**
5. `factory_insertion_seq_equals_front_enqueue_order` — the already-passing P5a test (replacing the dropped
   `registry_next_insertion_seq_is_serialized_and_hashed`); pins `insertion_seq == front.enqueue_order` after the
   reconcile. (D1 / C6 / C1.) **[M2]**
6. `factory_flip_determinism_over_scripted_commands` — the §8 determinism guard (two sims, scripted stream,
   identical per-tick hash sequence). (lockstep determinism across the bump.) **[M4]**

   **DROPPED from scope F (per §3.4 override):** `legacy_active_producer_removed_from_hash` — it conflicts with the
   verified-live reality that `active_producer_by_owner` is a still-written authoritative field. It is deferred to
   the producer-focus retirement slice. All three judges concur.

**Grafted reconciliation (from D-RISK) — STALE tests that must be intentionally INVERTED, not silently broken:**
7. The P3/P4 `factory_*_does_not_change_state_hash` family (and `snapshot_roundtrip_ignores_shadow` if present)
   ASSUMED serde-skip. Under the flip they INVERT — the registry now DOES change the hash and DOES round-trip. They
   are REPLACED by tests #1/#3 (the inverse assertions). **List them explicitly in the plan as intentional
   inversions** so they don't read as regressions at green-landing.

**Lane guards (the minimum-footprint risks this design must pin):**
8. `factory_registry_persists_across_ticks_hash_neutral` — the M1 guard: registry `progress` advances across ticks
   WITHOUT rebuild; `state_hash()` still bit-identical (registry still serde-skip). Proves the std::mem::take rebuild
   is gone in isolation, BEFORE any serde/hash move. (the load-bearing PERSIST arm.) **[M1]**
9. `progress_persists_across_ticks_not_re_derived` — step a build to progress 20, advance one more cadence-tick
   WITHOUT a queue mutation, assert `progress == 21` (per the `step_timer` cadence), NOT re-seeded to a cost-derived
   value. (C2.) **[M3]**
10. `reconcile_seed_arm_re_arms_on_new_front` — deliver/cancel a build so the front `enqueue_order` advances; assert
    the registry SEEDs a fresh `progress=0, balance=full_cost` for the new front and does NOT carry the old build's
    balance. (C7/C12.) **[M4]**
11. `single_wallet_charged_once_no_double_debit` — over a full build, total debit to `house.credits` == full cost;
    `economy.credits` is never the source and never diverges; `economy.spent_credits == full cost`. (§3.3; C15.) **[M3]**
12. `cancel_one_partial_refund_to_house_credits` — mid-build cancel routes `original_balance − balance` back to
    `house.credits` (NOT full cost, NOT via a fabricated house), first-match (not `.rev()`), and removes the matching
    `queues_by_owner` item. (C8.) **[M3]**
13. `pause_front_maps_to_manual_idle` — a `front.state == Paused` factory does not step (`advance_one_step → Idle`)
    and keeps its progress; unpause clears `manual` and resumes. (§2.3 pause bridge.) **[M3]**
14. `step_cadence_respects_step_rate_frames` — a build with `step_rate_frames > 1` does NOT advance every tick; it
    steps every `step_rate_frames` ticks (the §4.4 countdown). (C5.) **[M3]**
15. `c1_factories_step_before_house_tail` — `step_all` runs (charge applied) before `run_late_region` within one
    `advance_tick`. (C1 fold.) **[M4]**
16. `no_upfront_charge_at_enqueue` — enqueuing does NOT debit `house.credits` (the affordability gate still blocks
    an unaffordable START, but no money moves until the first step). (C3.) **[M3]**
17. `stall_on_no_funds_holds` — an underfunded house: the factory sets `on_hold`, no progress, nothing spent that
    step. (C4.) **[M3]**

---

## 10. UNKNOWN / UNCHECKED (DRIFT-default) — what P5b leaves open

- **`active_producer_by_owner` removal — DEFERRED / DRIFT-resolved.** It is a still-written, still-authoritative
  sidebar producer-focus binding (production_spawn.rs:111-117 write, production_placement.rs:88 read,
  world_hash.rs:210 hash), NOT a throwaway. KEPT hashed in P5b; its hash removal is deferred to whichever later
  slice retires the producer-focus binding. Scope-F's `legacy_active_producer_removed_from_hash` test is DROPPED
  (§3.4). DEFAULT VERDICT honored: do not silently remove a live authoritative field.
- **U-SHIP (D2 deferred) — KNOWN DRIFT, NOT fixed.** No `Ship` ProductionCategory; naval collapses into `Vehicle`
  (production_types.rs `ProductionCategory` Building<Defense<Infantry<Vehicle<Aircraft, NO Ship). A house owning a
  War Factory + Naval Yard collapses two gamemd factories into one Vehicle key → diverges the MultipleFactory
  `factory_count` (§4.3) and same-frame completion order on water maps. **Frequency: every naval water-map match.**
  PINNED by the existing `category_for_object_naval_collapses_to_vehicle_documented` regression test; Ship is a
  focused follow-up slice (its own hash-key change + version bump). DEFAULT: documented DRIFT.
- **U-ORDER — same-frame two-Begin command-dispatch order — UNCHECKED.** `enqueue_order` makes two same-tick Begins
  distinct (total SWEEP order), so the charge is deterministic. But whether the command-execute dispatch assigns
  `enqueue_order` in the SAME relative order gamemd's `EventClass::Execute` does for two same-frame Begins from
  different players is UNCHECKED here (it is the P5c U-ORDER spot-check). DEFAULT: surface, not asserted-equal.
- **U-QFRAMES — `total_base_frames` kept hashed.** Kept on the per-item fold as the sidebar ETA basis (via
  `effective_time_to_build_frames_for_type`). It no longer drives progress (frames timer retired). **Verify the
  reader survives at plan time;** if a live reader exists, keep it hashed (deterministic) — REJECTS D-SUBSTRATE's
  removal. If NO reader survives, remove it per D-SUBSTRATE and add a `legacy_total_base_frames_removed_from_hash`
  test. DEFAULT: keep hashed pending the reader check.
- **U-STEPRATE — per-step cadence vs gamemd CDTimer.** The one-step-per-CDTimer-expiry model (decrement
  `step_timer`, step at 0, reset to `step_rate_frames`) is the faithful cadence, but the exact reset semantics
  (reset-to-rate vs reset-to-rate-minus-overshoot) were not re-decoded this run (C5 covers the ÷PRODUCTION_STEPS
  magic, not the per-tick countdown reset). DEFAULT: reset-to-rate; flag for a C5-adjacent spot-check if the
  determinism test surfaces a cadence drift.
- **U-RATEINPUTS — `build_step_time` inputs sourcing at SEED/step.** The producer needs the owner's live
  `power_states` ratio + the per-category `factory_count`. P5b recomputes `set_rate` each step from current inputs
  (the producer is pure, cheap) — a superset of gamemd's per-change RecalcAllRates (each step ≥ each change), so the
  VALUE matches whenever inputs match. DEFAULT: recompute each step; no DRIFT vs gamemd's per-change recompute.
- **U-ENTITYID — `object.entity_id` at delivery — optional in P5b.** Whether the factory remembers its delivered
  child (`entity_id = Some(spawned)`) or clears to None is a P5b choice with no current consumer; left None for
  minimum footprint (hashed as the 0u8 presence tag). Flag if a later slice (radio link to the exiting vehicle)
  wants the back-reference.
- **U-AFFORD — affordability read == write wallet.** `economy.available() == credits` and `spend()` both touch the
  one `credits` shim loaded from `house.credits` → holds by construction in Rust. The engine-side proof (credit
  sub-object read-slot vs `Spend_Money` write) was asserted (study §9.4 H1) but not decompiled. DEFAULT: holds in
  Rust; engine-side proof incomplete.

---

## 11. THE P5c / P5d CLEAN SEAMS (NOT designed here)

- **P5c — the P9 replay/parity acceptance gate** (the flip's ACCEPTANCE gate). §8 names it: recorded command stream,
  two-replay bit-identity, `economy_conservation_over_replay` (C15), pre-flip-baseline-vs-post-flip observable-output
  equivalence (the x0.9-free producer correction documented as the one intended difference). Reuses the existing
  replay harness. Internals NOT designed here.
- **P5d — full `queues_by_owner` retirement into `Factory.queue`** (grafted from D-SUBSTRATE; the mirror-not-retire
  follow-up). Moves `enqueue_order` storage into `Factory` (a `VecDeque<(InternedId, u64)>` for the queue + an
  active-object stamp), retires the `BuildQueueItem` mirror entirely, and erases the U-QFRAMES / registry↔mirror
  value-redundancy (§3.3). Its own hash-key change + version bump (18→19). Out of scope for P5b — P5b leaves the
  clean seam.
- **P6** — prereq revalidation 3-way (incl. C9 prereq-driven resume). **P7** — purifier-count/IncomeMult/
  HarvestedCredits economy fix (the hashed `purifier_count`/`harvested_credits` are PLACED now, WIRED in P7).

---

## 12. Files touched (anchor on TEXT; world/mod.rs is co-edited — keep edits minimal + text-anchored)

| File | Edit (anchor) |
|---|---|
| `src/sim/economy.rs` | add `Serialize, Deserialize` to the `Economy` derive (anchor `#[derive(Debug, Clone, Default, PartialEq, Eq)]` on `pub struct Economy`); keep `credits` but it is no longer the wallet/hashed (re-point `spend`/`available`/`add_credits` to operate on the shim loaded from `house.credits` inside `step_all`). |
| `src/sim/production/factory.rs` | add serde to `Factory, FactoryRegistry, PendingObject, SpecialItem` (anchor each `// NO serde` derive line); REMOVE the `next_insertion_seq` + `seq_carry` fields (D1, anchor `next_insertion_seq: u64,` / `seq_carry: BTreeMap<…>`); split the rebuild into a persist-preserving `reconcile_from_queues` (§2.2) + retire `rebuild_shadow`/`rebuild_shadow_inner`/`rebuild_shadow_no_rules`/`remaining_balance_after`; add `step_all(&mut self, houses, rules)` (§4) + the `step_timer` countdown (§4.4); the C7 delivery+cancel `start_next_queued` bind helpers. Keep `advance_one_step`/`cancel_one`/`start_next_queued` BODIES UNCHANGED. |
| `src/sim/house_state.rs` | remove `#[serde(skip)]` from `economy` (anchor `#[serde(skip)] pub economy: Economy`). |
| `src/sim/production/production_types.rs` | remove `#[serde(skip)]` from `factory_shadow` (anchor `#[serde(skip)] pub factory_shadow`); NO `active_producer_by_owner` field change in this slice (§3.4 keeps it). |
| `src/sim/world/world_hash.rs` | `hash_production`: REMOVE `item.remaining_base_frames` (line 198) + `item.progress_carry` (199); KEEP `item.total_base_frames` (197, §3.2/U-QFRAMES); KEEP the `active_producer_by_owner` block (210-216, §3.4); ADD `hash_factory_registry` (iter_insertion_ordered fold, §3.2). `hash_houses`: ADD the `economy.{spent_credits, harvested_credits, purifier_count}` fold; KEEP `house.credits`. |
| `src/sim/world/mod.rs` (CO-EDITED — minimal, text-anchored) | replace `refresh_production_shadow` body with reconcile-not-rebuild (§2.2, anchor `let mut registry = std::mem::take(&mut self.production.factory_shadow);` at 1006); DELETE the mirror line `house.economy.credits = house.credits;` (979); add `self.production.factory_shadow.step_all(&mut self.houses, rules)` at Phase-7 head before `tick_production_with_overlay_registry` (anchor 2559); retire/repurpose the P5a `debug_assert_factory_step_matches_legacy(None)` call (1028) — it compared model-vs-legacy; post-flip there is no legacy charge to compare, so it becomes an authoritative-invariant assert in M1 or is retired (confirm at plan time). |
| `src/sim/production/production_queue.rs` | retire upfront `-= obj.cost` (218, keep the affordability gate); retire `.rev()`+full-refund (811/864 + 783/837/876 → route to `cancel_one`); retire `advance_queue_item` + `remaining_base_frames` decrement + completion→`ready_by_owner`/`pop_completed_front` half (keep spawn/placement); make `credits_entry_for_owner` non-fabricating (86, §6). |
| `src/sim/production/production_tech.rs` | retire the legacy build-time family + `matching_factory_count_for_owner` rescan once the producer + registry-key count replace them (§4.3/§6). |
| `src/sim/snapshot.rs` | `SNAPSHOT_VERSION` 17→18 (24) + history comment; flip `snapshot_version_is_17_in_shadow_phase` (374) → `snapshot_version_is_18`. |
| `src/sim/production/mod.rs` | re-export any new public surface (`step_all` if pub). |
| (tests) `src/sim/production/*tests*.rs` + `src/sim/world/*tests*.rs` + snapshot.rs tests | the §9 list + the §9.7 stale-test inversions. |

**NOT touched:** miner/combat/movement/unit_post (concurrent session); `place_ready_building` placement geometry;
the P5c harness. **Verify (separate foreground pass the human runs):** `cargo test -p vera20k` — read the literal
`test result:` line; confirm `SNAPSHOT_VERSION == 18` and the hash table matches §3.

---

*End of APPROVED P5b design. Winner D-PARITY-MIN (minimum-hash-delta), unanimous across all three judges
(mean 24/22.33/21). The flip keeps `queues_by_owner` the hashed queue-of-record + temporal source; the registry
adds only the minimal persistent progress set {progress, balance, step_timer, on_hold, suspended} (+ identity/
round-trip fields); one wallet (`house.credits`) charged once, `economy.credits` demoted to a non-hashed shim
(no double-hash, no last-writer caveat). Progress PERSISTS via the `(type_id, enqueue_order)` identity test (PERSIST
arm) instead of re-deriving from cost. Authored as four bisectable micro-steps inside one commit (grafted from
D-RISK), with the stale-test inversion reconciliation and the §3.3 double-hash audit + P5d retirement seam grafted
from D-SUBSTRATE. Two explicit overrides of the literal scope, both verified-live: (1) KEEP
`active_producer_by_owner` hashed (a live authoritative producer-focus binding, not a throwaway — scope-F test
dropped, hash removal deferred); (2) the `step_rate_frames` cadence countdown is new per-tick logic the flip adds.
D1 (drop next_insertion_seq, REVISES STUDY §6.4 + resolves CONCERN-2) and D2 (defer Ship, documented DRIFT pinned
by the regression test) designed-within. All v2 corrections honored: x0.9-free producer; Primary_For* Aircraft@53AC/
Infantry@53B0; SetRate-takes-total; purifier=count; SpecialItem 0-vs-(-1) distinct; Structure-arm no-op; C1 folded
into the one 17→18 bump. P5c (replay/parity) left a clean seam, not designed here.*
