---
title: Factory/House Production+Economy Substrate — P4 (queue + cancel + partial refund) Design Spec
date: 2026-06-04
status: design (winner = D2 substrate-fit, grafted with D3 structural-no-hash + D1 tiny-detail ledger; P4 oracle/hash-neutral scope is the only implement-now boundary)
scope: P4 ONLY — `FactoryRegistry::cancel_one` (first-match queued removal C6 / active AbandonProduction
       partial refund C8), `Factory::cancel_active` (the refund + reset primitive), `Factory::start_next_queued`
       (FIFO front-pop with the C7/C12 held-object guard, PROVEN in isolation, NOT wired to delivery), and the
       cancel-conservation shadow-assert. HASH-NEUTRAL: cancel/refund/advance run against an ORACLE (clone)
       economy + clone registry, never the hashed wallet. The legacy `cancel_by_type_for_owner` stays
       AUTHORITATIVE (its `.rev()` last-match + full-cost refund are the verified DRIFTs P4 models CORRECTLY in
       the shadow). world_hash.rs UNTOUCHED; SNAPSHOT_VERSION STAYS 17.
       OUT OF SCOPE (seams only): authority flip + fixing legacy cancel + 17->18 (P5), the delivery command
       that drives start_next_queued (P5+), per-step charge (DONE P3), prereq revalidation (P6),
       purifier/IncomeMult (P7).
source: docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md (v2-verified; C6 line 421, C7 line 423,
        C8 line 425, C12 line 433, C15 line 439; §6.2 line 593-595; §8 P4 line 735-741; §9.1)
        + the committed P1/P2/P3 code (factory.rs, economy.rs, world/mod.rs, production_shadow_tests.rs).
verification: every current-code claim is quoted file:line from a live read this session. The cancel/queue
              primitives are VERIFIED-LIVE v2 (study §9.1: RemoveFromQueue 0x004CA620 first-match V2;
              StartNextQueued 0x004CA5A0 front-pop V2; AbandonProduction 0x004C9FF0 Add_Credits(GetCost-Balance)
              F9/V2; FUN_004FAA10 heapId routing line 224). Ghidra read-only; cargo NOT run (separate pass).
rule: Rust-native structure, gamemd-native semantics. sim/ never depends on render/ui/sidebar/audio/net.
---

# Factory/House Substrate — P4 Design Spec (FIFO queue + cancel + partial refund, hash-neutral oracle)

## 0. TL;DR

Three competing P4 designs were scored against the v2-verified study and the committed P1/P2/P3 code.
**The winner is the substrate-fit-first design (D2)**, grafted with the test-first design's (D3)
**structural** no-hash proof (the cancel/queue methods are dead-code against clones until P5; the oracle is a
clone, never the wallet) and the parity-fidelity design's (D1) **complete tiny-detail ledger** and its honest
surfacing of the completed-build-cancel UNKNOWN.

The result: `FactoryRegistry::cancel_one(owner, category, type_id, &mut Economy) -> CancelOutcome`, a private
`Factory::cancel_active(&mut Economy) -> i32` (the AbandonProduction refund+reset primitive), and
`Factory::start_next_queued(&mut self) -> Option<InternedId>` (FIFO front-pop with the held-object guard),
all **pure methods on `Factory`/`FactoryRegistry` + `&mut Economy`**. P4 runs them only against a CLONE of the
registry and a CLONE of the economy — never `HouseState.economy`, never the legacy `credits`. The legacy
`cancel_by_type_for_owner` (`.rev()` last-match + full-cost refund) stays authoritative; the new methods are
`#[allow(dead_code)]` (no `advance_tick`/command call site) until P5. Because `Factory`/`FactoryRegistry`/
`Economy`/`CancelOutcome` carry **no serde derive** and no new call enters the tick, `state_hash()` is
bit-identical by construction — the strongest available no-hash proof (D3), not a leak-freedom argument.

**The three P4 DECISIONS (rationale in §3-§5):**
- **(a) Cancel precedence + no-match:** `cancel_one` removes a matching **QUEUED tail** copy FIRST
  (front-to-back, FIRST match — the C6 RemoveFromQueue path); only when **no queued copy** of that type exists
  does it AbandonProduction the **active** object (the C8 refund path). No factory / type-absent-and-active-
  differs → `NoMatch`, a true no-op (zero mutation, zero refund). The completed-but-held active object is a
  `NoMatch` for the active branch (the "AbandonProduction no-op after completion" study line 224), surfaced as
  the single most-uncertain ledger row.
- **(b) Queue-advance proves vs defers:** P4 ships `start_next_queued` and PROVES (1) it pops the **FRONT**
  (FIFO), (2) the **held-object guard** (no-op while `object.is_some()`), and (3) the C7/C12 negative invariant
  end-to-end with the P3 stepper (completion holds the object; `start_next_queued` is a no-op on a completed
  factory; the queue front does NOT advance). It DEFERS the **delivery command binding** (WHO calls it, C7) and
  the post-AbandonProduction auto-StartNextQueued to P5 — `cancel_active` leaves the queue tail intact and does
  NOT auto-advance.
- **(c) Refund + hash-neutrality:** the refund is `original_balance − balance` (the already-paid spent
  portion, C8), credited via `economy.add_credits` to an **oracle** (clone) economy. Hash-neutrality is
  STRUCTURAL: the write set of every P4 method is `{non-serde Factory/FactoryRegistry fields, &mut Economy
  clone}`, exhaustively disjoint from the hashed set; no `advance_tick` call site exists; `world_hash.rs` is
  untouched; `SNAPSHOT_VERSION` stays 17.

---

## 1. Scoring the three competing designs

Each lens scored 1-5 (5 best) on five axes, with a one-line justification per cell.

| Axis | D1 (parity-fidelity) | D2 (substrate-fit) | D3 (test-first / risk) |
|---|---|---|---|
| **Parity-fidelity** | **5** — exact RemoveFromQueue first-match + C8 spent-only refund; the ONLY design that surfaces the completed-build-cancel UNKNOWN (ledger #5: "no-op after completion") instead of silently refunding full cost | **5** — same first-match + spent-only refund; queued-first precedence grounded in the FUN_004FAA10 heapId routing; correctly notes queued-removal refund = 0 by the same formula | **4** — same arithmetic, but treats the active-abandon branch as "provisional/lower-precedence" and leans on "refund arithmetic is proven, routing deferred" — correct but it under-commits the precedence the study's §6.2 OR signature already implies |
| **Substrate-program fit** | **4** — faithful, but adds a `start_next_queued` that re-seeds `balance`/`rate` inline AND defers to rebuild (two-source ambiguity it flags but carries) + a 12-row ledger; more surface in the spine | **5** — smallest new surface that still advances the substrate: reuses the existing `Factory.queue` `rebuild_shadow` fills (factory.rs:370-372), mirrors P3's `cancel_active(&mut Economy)` ↔ `advance_one_step(&mut Economy)` shape exactly, one sibling debug-assert beside `debug_assert_factory_conservation` | **3** — leanest blast radius, but provides NO public `start_next_queued` (only a commented seam), so the substrate spine gains no FIFO-pop primitive this slice — the queue-advance proof is a pure negative, leaving P5 to build the pop from scratch |
| **Testability / determinism** | **5** — full §8-P4 set + boundary sub-cases (progress 0, completed, cost-1); VecDeque/integer throughout | **5** — same set; adds the per-tick cancel-conservation assert (runs on every live shadow tick, the strongest guard) + a determinism variant; BTreeMap/VecDeque/integer | **4** — full §8-P4 set as pure unit tests (lowest-friction), but the conservation assert is "optional/droppable" — weaker live coverage |
| **Risk / blast-radius (HASH-NEUTRAL)** | **3** — largest: cancel + start_next_queued (with inline seeding) + a per-tick assert that itself must handle the queued-first precedence branch correctly | **4** — moderate: three methods + one outcome enum + one debug-assert sibling; no tick call site; the assert branches on precedence (queued-vs-active) faithfully | **5** — smallest: factory.rs cancel + reset + a commented seam + one acceptance test; "no call exists" is trivially auditable |
| **Buildability (lands green fast)** | **3** — most plumbing (inline-seed start_next_queued + the precedence-aware assert) | **4** — reuses P3 scaffolding; `cancel_one` is a get_mut + position + remove; `cancel_active` is the refund formula + a reset; the assert mirrors P3's verbatim | **5** — two method bodies + tests; fewest moving parts |
| **TOTAL** | **20** | **23** | **21** |

**Winner: D2 (substrate-fit-first), total 23.** It best honors the master directive — "slot into the
substrate program, do not invent a parallel architecture" — by reusing the `Factory.queue` the P3
`rebuild_shadow` already populates and by mirroring P3's `&mut Economy` method shape exactly (so P5 flips WHO
is passed, not the algorithm). It makes cancel-conservation checkable on every live tick (the strongest parity
guard), which D3's "optional/droppable" assert weakens, and it commits the queued-first precedence the study's
§6.2 OR signature implies — whereas D3 hedges it as "provisional." Its arithmetic and ledger match D1's at
lower spine cost, and unlike D1 it does not carry the two-source `start_next_queued` balance-seed ambiguity
into P4 (it defers the seed cleanly to the next rebuild / P5 begin path).

### 1.1 What was grafted from the runners-up

- **From D3 (test-first / risk):** the **structural** no-hash proof is adopted as the primary guarantee
  (§5.1), resting on three independently-sufficient facts auditable from the diff: (1) no serde derive on
  `Factory`/`FactoryRegistry`/`Economy`/`CancelOutcome`; (2) no authoritative `advance_tick`/command call
  site — the methods are `#[allow(dead_code)]` until P5; (3) the oracle is always a clone, never the wallet.
  We keep D2's live cancel-conservation assert (so the C8/C15 invariant is checked each tick), but the
  *release-path* hash neutrality is D3's "no call exists." We also adopt D3's exhaustive **write-set
  verification** framing as the burden-of-proof evidence (CLAUDE.md "default to DRIFT unless provably no-hash").
- **From D1 (parity-fidelity):** the complete **tiny-detail ledger** (§6) AND its decisive honesty on the
  completed-build-cancel case (ledger #5): the study line 224 phrase "AbandonProduction (no-op after
  completion)" means cancelling a complete-but-undelivered active object via the active branch is a **no-op**,
  NOT a full-cost refund. D1 is the only lens that refuses to silently refund full cost here; we adopt its
  decision (`cancel_one` active branch is `NoMatch` on a completed factory) and surface the UNKNOWN (§9).

### 1.2 Why D2 over D3 specifically (the start_next_queued fork)

D3 ships NO public `start_next_queued` — only a commented seam — arguing the §8-P4 `queue_advances_only_after_
delivery` test is a pure NEGATIVE assertion (the queue must NOT advance), so the positive pop is wholly P5.
That is the smallest blast radius, but it leaves the substrate with no FIFO-pop primitive and forces P5 to
build the pop + its guard from scratch under the authority-flip pressure. D2 ships `start_next_queued` as a
tested-in-isolation primitive (front-pop + held-object guard), **dormant** in the tick (no call site), so P5
binds an already-proven pop to the delivery command rather than authoring it. This is the same discipline P3
used: `advance_one_step` shipped as a pure method with no authoritative call site, so P5 flips WHO calls it.
Shipping the primitive proven-but-dormant is strictly more substrate progress at the same (zero) hash risk —
the pop touches only clones in P4. (D3's negative invariant is still proven; D2 just also proves the positive
pop mechanics it will need.)

---

## 2. The `cancel_one` / `cancel_active` algorithms (C6/C8)

### 2.1 Signatures & the result enum (the hash-neutrality core)

```rust
impl FactoryRegistry {
    /// Cancel one production of `type_id` for (owner, category) — the substrate analog
    /// of the engine's cancel-one command. PURE on the registry + an ORACLE (clone)
    /// economy in P4 (never the hashed wallet). Precedence: a QUEUED tail copy is
    /// removed FIRST (front-to-back, FIRST match — C6); ONLY when no queued copy of
    /// `type_id` matches AND the ACTIVE object is `type_id` is the active build
    /// abandoned (refund = original_balance - balance, C8). No match -> NoMatch.
    pub fn cancel_one(
        &mut self,
        owner: InternedId,
        category: ProductionCategory,
        type_id: InternedId,
        economy: &mut Economy,        // ORACLE clone in P4; P5 flips WHO is passed, not this body
    ) -> CancelOutcome;
}

impl Factory {
    /// AbandonProduction the ACTIVE object (C8): refund the ALREADY-PAID portion
    /// (original_balance - balance) to the (oracle) economy, then reset to the
    /// empty-but-registered state (the partial object is destroyed). Returns the
    /// refunded amount. No-op (returns 0) when there is no active object OR the active
    /// object is complete-but-held (the "no-op after completion" rule). Leaves the
    /// queue tail intact — the next-queue advance (start_next_queued) is command-bound.
    fn cancel_active(&mut self, economy: &mut Economy) -> i32;   // private; the C8 primitive
}
```

```rust
/// Outcome of a cancel-one (consumer: tests + the P4 cancel-conservation assert).
/// Serde-free — the same no-hash discipline as `StepOutcome` (factory.rs:217).
#[derive(Debug, Clone, Copy, PartialEq, Eq)] // NO serde
pub enum CancelOutcome {
    /// No factory, OR the type matched neither a queued tail copy nor an abandonable
    /// active object (including the complete-but-held case). True no-op.
    NoMatch,
    /// A queued tail copy of `type_id` was removed (first match, front-to-back). No
    /// refund — a queued item was never charged (its spent portion is 0).
    QueuedRemoved,
    /// The active object was AbandonProduction'd; `refund` credits returned (C8).
    AbandonedActive { refund: i32 },
}
```

The `&mut Economy` is **always an oracle** in P4 — never the real `HouseState.economy`. The method body is
hash-agnostic; neutrality is enforced at the **call site** (§5): P4 only ever passes a `clone()` of an
economy or a test-local one, on a `clone()` of the registry. This is the exact P3 shape (study §6.2 line 595:
`cancel_one(&mut self, owner, category, type_id, &mut Economy)`; study §8 P4 line 740: the refund is exercised
so "total house credits return to pre-build value" — checked on the ORACLE, not the hashed wallet).

### 2.2 `cancel_one` — the exact routing (integer/VecDeque ops)

```
fn cancel_one(self, owner, category, type_id, economy) -> CancelOutcome:

  // (R0) look up the one factory for this (owner, category). None -> NoMatch.
  let Some(f) = self.factories.get_mut(&(owner, category)) else {
      return CancelOutcome::NoMatch;
  };

  // (R1) QUEUED TAIL FIRST — RemoveFromQueue (C6): the FIRST front-to-back match.
  //   VecDeque::iter().position() scans front-to-back, returns the FIRST index;
  //   VecDeque::remove(idx) removes it and shifts survivors down (order preserved).
  //   This is the DRIFT fix vs the legacy `.rev()`/last-match (production_queue.rs:811).
  if let Some(idx) = f.queue.iter().position(|&t| t == type_id) {
      f.queue.remove(idx);
      return CancelOutcome::QueuedRemoved;          // no refund: a queued item is uncharged
  }

  // (R2) ELSE the ACTIVE object, if it is this type AND abandonable: AbandonProduction.
  //   cancel_active itself no-ops (returns 0 / no reset) on a complete-but-held object,
  //   in which case we fall through to NoMatch (the "no-op after completion" rule).
  if f.object.as_ref().map(|o| o.type_id) == Some(type_id) {
      let refund = f.cancel_active(economy);
      // cancel_active returns < 0 NEVER; it returns the refund (possibly 0) when it
      // acted, and signals "did not act" via the completed/no-object guard. We model
      // "did it act" with a bool the primitive returns (see §2.3) rather than overload 0.
      // -> if it acted: AbandonedActive { refund }; if it no-op'd (completed): NoMatch.
      return /* see §2.3: acted ? AbandonedActive{refund} : NoMatch */;
  }

  // (R3) no queued copy, active object is a different type (or none) -> true no-op.
  CancelOutcome::NoMatch
```

**Precedence — QUEUED FIRST, ACTIVE FALLBACK (DECISION a).** Grounding:
- The committed `rebuild_shadow` puts the active object in `Factory.object` (factory.rs:361-368) and the
  **tail behind it** in `Factory.queue` (`queue.iter().skip(1)`, factory.rs:370-372). So the active object is
  NOT an element of `queue` — exactly the gamemd split: `RemoveFromQueue 0x004CA620` scans the `QueuedObjects`
  vector (F6, study line 64), and the in-flight `Object` (+0x58, F7 line 65) is a separate slot Abandon-
  Production touches (F9 line 67).
- The dispatcher `FUN_004FAA10 (house, rtti, heapId, naval, removeAll)` routes by heapId: `heapId ≥ 0 →
  RemoveFromQueue`; `heapId = −1 → AbandonProduction + StartNextQueued` (study line 224). The two primitives
  are mutually exclusive call paths, not a fallthrough.
- The study's own §6.2 signature (line 593-595) states cancel_one removes "the FIRST matching queued type
  (front-to-back), **OR** abandon the active object with partial refund" — an OR with the queued path named
  first. The faithful single-entry reproduction is therefore: prefer the queued removal; abandon the active
  only when no queued copy of that type remains. This reproduces the observable cadence — a right-click on a
  cameo with a count badge drops the queue count before the in-progress build's progress bar resets.
- **Duplicate-type handling:** `queue.iter().position(...)` returns the FIRST (front-most) match → `[A,B,A,C]`
  cancel A → remove index 0 → `[B,A,C]` (C6, study line 739). The legacy `.rev()` gives `[A,B,C]` — the DRIFT
  this proves correct in the shadow.

### 2.3 `cancel_active` — the AbandonProduction primitive (C8 refund + reset)

`cancel_active` must distinguish "acted (refunded + reset)" from "no-op (completed / no object)" without
overloading the `0` refund (a progress-0 active object legitimately refunds 0). It returns the refund AND a
"did-act" signal. Cleanest: return `Option<i32>` (`Some(refund)` = acted; `None` = no-op), and `cancel_one`
maps `Some(r) -> AbandonedActive { refund: r }`, `None -> NoMatch`.

```
fn cancel_active(self, economy) -> Option<i32>:
  // No active object -> no-op.
  let Some(obj) = self.object.as_ref() else { return None; };

  // "No-op after completion" (study line 224): a complete-but-held active object is
  // NOT abandoned via this path (it is delivered/awaiting placement; cancelling a
  // completed build goes through the ready-queue path, P5+). progress >= 54 OR (the
  // settled state) suspended-with-balance-0 marks completion.
  if self.progress >= PRODUCTION_STEPS {
      return None;                                  // surfaced UNKNOWN — see §9 U1, ledger #5
  }

  // C8: refund the already-paid (spent) portion. balance is remaining-unpaid, so
  // original_balance - balance is exactly what the per-step ladder removed. Clamp >= 0
  // defensively (the invariant balance <= original_balance holds; the stepper only
  // decrements balance, factory.rs:197).
  let refund = (self.original_balance - self.balance).max(0);
  economy.add_credits(refund);                      // ORACLE economy in P4 (saturating, economy.rs:35)

  // Reset to the empty-but-registered state; the partial object is destroyed.
  self.object = None;                               // partial object destroyed (in P4 shadow entity_id was None)
  self.progress = 0;
  self.balance = 0;
  self.original_balance = 0;
  self.step_rate_frames = 0;                         // no-object => rate-0 sentinel (matches set_rate, factory.rs:136)
  self.step_timer = 0;
  self.on_hold = false;
  self.suspended = false;
  self.manual = false;
  self.special = SpecialItem::NoneNeg1;              // canonical "none"; do NOT collapse 0/-1 (factory.rs:76)
  // self.queue is LEFT INTACT — StartNextQueued is command-bound (C7), deferred to P5.
  Some(refund)
```

**Refund formula `original_balance − balance` (C8, study line 425 / F9 line 67):**
- `original_balance` = the type's full `GetCost` snapshot (set in `rebuild_shadow`, factory.rs:384).
- `balance` = remaining-unpaid, charged down per step (factory.rs:103, 197).
- `original_balance − balance` = the already-paid spent portion = `GetCost − Balance` — exactly `AbandonProduction
  0x004C9FF0`'s `Add_Credits(GetCost − Balance)` (F9, study line 67). NOT the full cost (the legacy DRIFT,
  production_queue.rs:837, study line 684).
- **Refund rounding: NONE.** Both terms are exact `i32` credits; the subtraction is exact. The per-step floor
  rounding lives in `advance_one_step` (factory.rs:179-183); the refund is the exact arithmetic complement of
  what was spent, so `Σ per-step charge + refund = original_balance` exactly (telescoping, C15 line 439). The
  `.max(0)` clamp documents intent and guards a malformed shadow; it is never expected to fire.

**Object-destroyed modeling:** in the P4 shadow `object.entity_id` is always `None` (the legacy path owns the
produced entity, factory.rs:64-66, 364), so "destroy the partial object" is exactly `object = None`. No entity
is despawned (there is none in the shadow). When P5 makes this authoritative and `entity_id` becomes `Some`,
this reset is the seam where the real partial-object despawn (and the F9 AI-tracking-field clear, currently
out of scope per `feedback_no_ai_yet`) hook in — documented, not wired.

---

## 3. FIFO `start_next_queued` — what P4 PROVES vs DEFERS (DECISION b)

### 3.1 The contract split (C7/C12)

- **C12 (study line 433):** completion sets the factory **suspended with the object still attached** (`balance
  = 0`); the object stays pending. `advance_one_step` ALREADY does this (factory.rs:202-208): on reaching 54
  it sets `suspended = true`, leaves `object = Some`, zeroes balance, returns `Completed`. **P3 already
  implements the "completion holds the object" half of C12.**
- **C7 (study line 423):** the queue advances ONLY after a **successful delivery command** — `CompletedProduction
  0x004CA1A0` has NO begin/next call; the advance is `FUN_004FAA10`'s post-delivery `StartNextQueued`.
  Delivery is command-bound (study §4.2 #5 line 364). **Delivery wiring is P5+ (out of P4 scope).**

### 3.2 What P4 PROVES — the pop primitive + the negative invariant

P4 ships `start_next_queued` and proves only its pure pop-front mechanics + the gating guard, NOT a delivery-
driven invocation:

```
fn start_next_queued(self) -> Option<InternedId>:
  // GUARD (C7/C12): a held object blocks the advance. A completed-but-held factory
  // (progress 54, suspended, object attached) is a no-op here — the queue does not
  // advance on completion alone; the delivery command (P5) clears the object first.
  if self.object.is_some() {
      return None;                                  // "Object null required" precondition (study line 117)
  }
  let next = self.queue.pop_front()?;               // FIFO FRONT pop (C6); None on empty queue
  self.object = Some(PendingObject { type_id: next, entity_id: None });
  self.progress = 0;
  // balance/original_balance/step_rate are LEFT for the next rebuild_shadow to seed
  // from the type cost (factory.rs:377-385) — the single source of the cost-based
  // balance in P4. (P5's authoritative Begin_Production(resume=1) decides whether the
  // pop seeds the cost inline; that is a P5 wiring choice, not this P4 algorithm.)
  self.balance = 0;
  self.original_balance = 0;
  self.suspended = false;
  self.on_hold = false;
  Some(next)
```

**P4 proves (all against clones / hand-seeded factories):**
1. `start_next_queued` pops the **FRONT** (FIFO): `[X,Y,Z]` with `object = None` → active = X, queue = `[Y,Z]`
   (C6 StartNextQueued front-pop, study line 117/421).
2. The **held-object guard**: returns `None` (no-op, queue unchanged) when `object.is_some()` — an in-flight
   OR a completed-held object is never displaced (the "Object null required" precondition, study line 117).
3. The **C7/C12 negative invariant end-to-end** with the P3 stepper: drive a factory to `Completed`; it stays
   `suspended && object.is_some()`; `start_next_queued` returns `None` and the queue front is unchanged; THEN
   manually clear the object (simulating the P5 delivery commit) and assert `start_next_queued` now pops the
   front. This proves "completion holds the object; the queue does not advance until delivery" WITHOUT wiring
   delivery (study line 741).

### 3.3 What P4 DEFERS to P5

- **The delivery command binding** (C7): NO `advance_tick`/command path calls `start_next_queued`. The
  "object cleared → queue advance" transition is invoked only inside the P4 test (manually, to prove the
  post-delivery mechanics) and by the P5 delivery commit (real wiring). The method is `#[allow(dead_code)]`,
  dormant in the tick.
- **The post-AbandonProduction auto-StartNextQueued** (study line 224: `heapId = −1 → AbandonProduction +
  StartNextQueued`): `cancel_active` LEAVES the queue intact (§2.3) and does NOT auto-call `start_next_queued`.
  Reason: that auto-advance is part of the same command-bound `FUN_004FAA10` path that is P5. Wiring it in P4
  would make a queued item start charging (a sim-state change) outside the delivery command — the cadence
  DRIFT (study line 364). P4 keeps abandon and advance **separable**; P5 binds them.
- **Inline balance/rate seeding of the popped front**: P4 leaves `balance`/`rate` to the next `rebuild_shadow`
  (the single source of the cost-based balance). P5's authoritative begin path decides whether the pop seeds
  inline. Flagged (§9 U2), not asserted.

---

## 4. The cancel-conservation shadow-assert (surface, never equalize)

A debug-only assert wired into `debug_assert_production_shadow` (world/mod.rs:1024) beside the P3
`debug_assert_factory_conservation` (world/mod.rs:1035), mirroring its template **exactly** (clone factory +
clone economy, never write back):

```
#[cfg(debug_assertions)]
fn debug_assert_factory_cancel_refund(self):
  for factory in self.production.factory_shadow.iter_insertion_ordered():
      let Some(obj) = factory.object.as_ref() else { continue; };
      // Drive a fresh CLONE forward ~half the build against a CLONE economy seeded with
      // exactly original_balance, record spent, then cancel the active object. The
      // refund must equal the spent portion and return the oracle to its start (C8/C15).
      let cost = factory.original_balance;
      let mut f = factory.clone();
      f.progress = 0; f.balance = cost; f.on_hold = false; f.suspended = false; f.manual = false;
      let mut econ = Economy { credits: cost, ..Economy::default() };
      let target = (PRODUCTION_STEPS / 2).max(1);
      while f.progress < target {
          if !matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped) { break; }
      }
      let spent = econ.spent_credits;                 // already removed by the ladder
      let mut reg = self.production.factory_shadow.clone();   // throwaway 1-lookup registry clone
      // overwrite the clone's factory with the mid-build `f` so cancel_one targets it:
      // (reg holds a copy; cancel against it, never the live shadow)
      let outcome = reg.cancel_one_on(/* the cloned key, obj.type_id */, &mut econ);
      // C8: an active cancel (no queued copy of this type) refunds exactly the spent portion.
      debug_assert!(matches!(outcome, CancelOutcome::AbandonedActive { refund } if refund == spent),
          "C8: tick {} {:?}/{:?}: active cancel must refund the spent portion {}",
          self.tick, factory.owner, factory.category, spent);
      // C15: the oracle returns to its starting credits (granted - spent + refund == granted).
      debug_assert_eq!(econ.credits, cost,
          "C15: tick {} {:?}/{:?}: post-cancel oracle balance must equal start {}",
          self.tick, factory.owner, factory.category, cost);
```

> **Precedence-aware caveat (D2 graft):** because §2.2 prefers a queued copy, the assert must NOT pick a type
> that also sits in the tail (else `cancel_one` returns `QueuedRemoved`, refund 0, not the abandon path). The
> assert therefore branches: if `factory.queue` contains `obj.type_id`, expect `QueuedRemoved` (refund 0);
> else build the mid-build clone and expect `AbandonedActive { refund == spent }`. This keeps the assert
> faithful to the chosen precedence. (Simplest robust form: drive the assert on the `f` clone directly via
> `f.cancel_active(&mut econ)` so there is no queued-vs-active ambiguity — the registry-level `cancel_one` is
> covered by the unit tests.) Divergence is **SURFACED** (tick + owner + category), **NEVER equalized** — the
> P3 discipline (world/mod.rs:1033).

Wired into the umbrella, one added line:
```
fn debug_assert_production_shadow(self):
    self.debug_assert_economy_shadow();
    self.debug_assert_factory_shell_trace();
    self.debug_assert_factory_conservation();   // P3
    self.debug_assert_factory_cancel_refund();  // P4  <-- added
```

---

## 5. No-hash proof + its test (DECISION c)

### 5.1 The structural argument (D3 graft — strongest available)

The no-hash guarantee is **structural, not behavioral**, and inherits P3's already-proven property — P4 adds
methods of the identical shape. Three independently-sufficient facts, each auditable from the diff:

1. **No serde derive.** `Factory`/`FactoryRegistry` (factory.rs:93, 247), `PendingObject` (factory.rs:67),
   `SpecialItem` (factory.rs:77), `Economy` (economy.rs:17), and the new `CancelOutcome` carry NO
   `Serialize`/`Deserialize` — `CancelOutcome` follows the same `#[derive(Debug, Clone, Copy, PartialEq, Eq)]
   // NO serde` line as `StepOutcome` (factory.rs:217). None of their fields enter bincode or the hash.
   `cancel_one`/`cancel_active`/`start_next_queued` mutate only a `Factory`/`FactoryRegistry` + an `Economy`,
   none of which `state_hash()` visits. The registry lives in `#[serde(skip)]` `factory_shadow`.
2. **No new authoritative call site.** `refresh_production_shadow` (world/mod.rs) still only calls
   `refresh_economy_shadow` + `rebuild_shadow`. P4 adds NO `cancel_one`/`start_next_queued` call into the
   running sim. The methods are `#[allow(dead_code)]` (factory.rs:28) until P5 — "no call site = no behavioral
   change = the per-tick hash sequence is byte-for-byte P3's."
3. **The oracle is always a clone.** P4 only ever cancels/pops against a CLONE of the registry and a CLONE of
   an economy (§4) or a test-local `Economy`. `HouseState.economy` and the legacy `credits` are never the
   receiver of `add_credits`. The legacy `cancel_by_type_for_owner` (production_queue.rs:794, the `.rev()` +
   full-refund DRIFT) stays authoritative, untouched.

Because the write set of every P4 method is `{non-serde Factory/FactoryRegistry fields, &mut Economy clone}`
— exhaustively disjoint from the hashed set — **no input distribution to `cancel_one` can change a hashed
bit**. The proof is by construction (no hashed field is in the write set), the strongest acceptance evidence
under the CLAUDE.md "default to DRIFT unless provably no-hash" bar — exhaustive write-set verification, not a
sampled trace. `world_hash.rs` is UNTOUCHED; `SNAPSHOT_VERSION` STAYS 17 (snapshot.rs; pin test
`snapshot_version_is_17_in_shadow_phase`). The 17→18 authority flip is P5.

### 5.2 The acceptance test (required — mirrors `factory_advance_step_does_not_change_state_hash`)

In `production_shadow_tests.rs`, structurally identical to the P3 acceptance test
(production_shadow_tests.rs:357), reusing `empty_rules()`/`queued_item()`/`insert_queue()`:

```rust
#[test]
fn factory_cancel_one_does_not_change_state_hash() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(&mut sim, owner, ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1));
    sim.refresh_production_shadow(Some(&rules));
    let before = sim.state_hash();
    let legacy_credits = sim.houses[&owner].credits;

    // Cancel (active abandon, mid-build) against a CLONE of the registry + a CLONE of
    // the wallet; prove the real hash + the legacy wallet are bit-identical.
    let mut reg = sim.production.factory_shadow.clone();
    let mut oracle = sim.houses[&owner].economy.clone();
    // seed a mid-build cost on the clone so the refund is nonzero:
    // (mutate the clone's factory: progress 20, balance 300, original_balance 700)
    let _ = reg.cancel_one(owner, ProductionCategory::Vehicle, ty, &mut oracle);

    assert_eq!(before, sim.state_hash(),
        "P4 cancel on a clone must not perturb the state hash (serde-skip + clone)");
    assert_eq!(sim.houses[&owner].credits, legacy_credits,
        "the legacy wallet is untouched by oracle cancel");
}
```

Plus a determinism variant mirroring `production_shadow_with_oracle_is_deterministic`
(production_shadow_tests.rs:398): a per-tick closure that calls `cancel_one`/`start_next_queued` on clones;
two runs produce identical hash sequences.

---

## 6. Tiny-detail ledger (D1 graft)

| # | Detail | Resolution | Grounding |
|---|---|---|---|
| 1 | **Duplicate-type in queue** (`[A,B,A,C]`, cancel A) | Remove the FIRST (front-most) match → `[B,A,C]` via `queue.iter().position()` + `VecDeque::remove`. NOT the last A (the legacy `.rev()` DRIFT). | C6 line 421; test line 739; legacy production_queue.rs:811 |
| 2 | **Cancel active vs a queued copy of the SAME type** | Queued copy removed FIRST (§2.2); active abandoned only when no tail copy of that type remains. | F6/F7 lines 64-65; dispatcher line 224; §6.2 OR signature line 593-595 |
| 3 | **Refund of a not-yet-charged build** | `original_balance − balance`. Queued tail item: never charged → `QueuedRemoved`, refund 0 (RemoveFromQueue never refunds). Active object never stepped (progress 0): `balance == original_balance` → refund 0 (falls out of the formula, no special case). | C8 line 425; F9 line 67; factory.rs:384 |
| 4 | **Refund rounding** | NONE. Both terms are exact `i32` credits; `original_balance − balance` is exact integer subtraction; `economy.add_credits` is saturating. The refund is the exact arithmetic complement of `Σ spent` (telescoping), so `Σ spent + refund = original_balance` exactly. | C15 line 439; economy.rs:35; factory.rs:48-61 ladder |
| 5 | **Completed-but-held active cancel** | `cancel_active` is a **no-op** (returns `None` → `cancel_one` returns `NoMatch`) when `progress >= 54`. Does NOT refund full cost. Matches "AbandonProduction (no-op after completion)" (study line 224); cancelling a completed build goes through the ready-queue path (P5+). The single most-uncertain row — surfaced as §9 U1, NOT silently full-refunded. | C12 line 433; dispatcher line 224; legacy ready path production_queue.rs:849 |
| 6 | **start_next_queued on an object-present / completed-held factory** | No-op, returns `None`; queue unchanged — the "Object null required" guard. An in-flight OR completed-held object is never displaced. | C6/StartNextQueued line 117 |
| 7 | **start_next_queued on an empty queue** | `pop_front()? → None`; no object created. | line 117 ("queue non-empty" precondition) |
| 8 | **Empty factory after cancel** (no object, no queue) | gamemd self-deletes the empty factory (C9). P4 LEAVES the empty `Factory` in the registry; the next `rebuild_shadow` drops empty-queue categories anyway (`queue.front()` continue, factory.rs:329). Self-delete is C9/P6 — a documented seam, not modeled in P4. | C9; rebuild factory.rs:329 |
| 9 | **Negative refund** | Impossible under `balance ≤ original_balance` (the stepper only decrements balance, factory.rs:197). `.max(0)` clamp documents intent + guards a malformed shadow; never expected to fire. | factory.rs:197 |
| 10 | **add_credits saturation** | `economy.add_credits` saturates (economy.rs:35-37); a refund near `i32::MAX` saturates rather than overflows. Faithful for the oracle; the hashed wallet is untouched. | economy.rs:35 |
| 11 | **NoMatch is a true no-op** | Zero economy mutation, zero state change — a mis-targeted cancel is observably inert (RemoveFromQueue Find-fail / AbandonProduction null-object no-op). | §2.2 R3; line 224 |
| 12 | **Queue order preserved on removal** | `VecDeque::remove(idx)` shifts subsequent items down, preserving relative order — the "shift down" of RemoveFromQueue 0x004CA620. | C6 line 421 |
| 13 | **`special` after cancel** | reset to `SpecialItem::NoneNeg1` (the canonical "none"); do NOT collapse 0/(−1). | factory.rs:76; study §3 |
| 14 | **`cancel_active` did-act signal vs refund 0** | `cancel_active` returns `Option<i32>` (`Some(refund)` = acted, including `Some(0)`; `None` = no-op). Avoids overloading `0` (a progress-0 active object legitimately refunds 0 but DID act). | §2.3 |

---

## 7. Acceptance tests (the §8 P4 set + the ledger guards)

In `factory.rs mod tests` (pure value-type):

| Test | Asserts | Contract |
|---|---|---|
| `cancel_one_removes_first_matching` | queue `[A,B,A,C]` (all queued, object None), cancel A → `[B,A,C]`, returns `QueuedRemoved`, refund 0. | C6 (study line 739) |
| `cancel_one_queued_preferred_over_active_same_type` | active = A, tail = `[A]`; cancel A → tail copy removed (`QueuedRemoved`), active build untouched. | precedence §2.2 |
| `cancel_active_refunds_spent_only` | armed factory cost 700 stepped to progress 20, cancel active → `AbandonedActive { refund == original_balance − balance }`; oracle returns to pre-build credits; factory reset, object None. | C8/C15 (study line 740) |
| `cancel_active_at_progress_zero_refunds_nothing` | progress 0 → `AbandonedActive { refund: 0 }` (acted, refund 0); factory reset. | ledger #3/#14 |
| `cancel_one_completed_active_is_noop` | drive to completion (held), cancel active type → `NoMatch`, factory unchanged (still suspended + object held). | ledger #5; line 224 |
| `cancel_one_no_match_is_noop` | type absent from both active and tail → `NoMatch`, no economy/state change. | §2.2 R3 |
| `start_next_queued_pops_front` | queue `[X,Y,Z]`, object None → active = X, queue `[Y,Z]`. | C6 |
| `start_next_queued_blocked_while_object_held` | object Some → `None`, queue unchanged. | C7/C12 (study line 117) |
| `cancel_active_round_trip_conserves` | C15 cancel-side telescoping over costs {1, 25, 700, 99991} × progress {0, 1, 20, 53}: step k against an oracle, cancel, assert oracle returns to start. | C15 line 439 |

In `production_shadow_tests.rs` (world-level, hash-neutral):

| Test | Asserts | Contract |
|---|---|---|
| `factory_cancel_one_does_not_change_state_hash` | §5.2 — the acceptance test (mirrors `factory_advance_step_does_not_change_state_hash`). | no-hash guarantee |
| `queue_advances_only_after_delivery` | completion held; `start_next_queued` no-op while object attached; manual object-clear (simulated delivery) then `start_next_queued` pops the front; queue front unchanged until the clear. | C7/C12 (study line 741) |
| `production_shadow_with_cancel_is_deterministic` | per-tick closure calls `cancel_one`/`start_next_queued` on clones; two runs identical hash sequences (mirror production_shadow_tests.rs:398). | determinism |

---

## 8. Out-of-scope seams (left clean, NOT implemented)

| Concern | Status | Seam |
|---|---|---|
| Authority flip (oracle→real wallet), fixing legacy `cancel_by_type_for_owner` `.rev()`+full-refund, SNAPSHOT_VERSION 17→18 | P5 | `cancel_one(&mut Economy)` signature already P5-ready; flip WHO is passed + replace the legacy cancel call, not this method. |
| The delivery command that drives `start_next_queued` (C7) | P5+ | `start_next_queued` shipped proven-but-dormant; P5 binds it to the delivery commit. |
| Post-AbandonProduction auto-StartNextQueued (`heapId = −1` path) | P5 | `cancel_active` leaves the queue tail intact; P5 binds abandon + advance in the same command. |
| Completed-build cancel (ready-queue path) | P5+ | `cancel_active` no-ops on a completed factory (ledger #5); the ready-queue cancel is P5's. |
| Per-step charge / SetRate | DONE P3 | `advance_one_step` / `set_rate` (factory.rs:130, 160). |
| Empty-factory self-delete (C9) | P6 | `cancel_active` leaves the empty factory; the next rebuild drops empty-queue categories. |
| Prereq revalidation's abandon-active path (C9/C19) | P6 | reuses `cancel_active` + the refund formula, wired to building-lifecycle. |
| Inline balance/rate seed of the popped front | P5 | `start_next_queued` leaves the seed to the next rebuild; P5's begin path decides. |
| Purifier / IncomeMult | P7 | `Economy` fields present; not exercised by P4. |

---

## 9. UNKNOWN / UNCHECKED (marked, not guessed)

- **U1 — Completed-but-held active cancel routing (ledger #5).** The study line 224 phrase "AbandonProduction
  (no-op after completion)" indicates AbandonProduction does nothing once the object is complete; P4 models the
  active-abandon branch as a `NoMatch` no-op on a completed factory rather than refunding full cost. This is
  the honest DRIFT-default position. **UNCHECKED:** confirming the early-out on completion needs
  `decompile 0x004C9FF0`, and the completed-build cancel path (ready-queue) is P5+. NOT silently full-refunded.
- **U2 — `start_next_queued` balance/rate seeding (§3.3).** P4 leaves the popped front's balance/rate to the
  next `rebuild_shadow` (the single source of the cost-based balance) rather than recomputing inline via the
  engine's `Begin_Production(resume=1)`→SetRate path. Equivalent in the shadow (rebuild is the single source),
  but P5 must decide whether the authoritative pop seeds inline. Flagged, not asserted.
- **U3 — Sidebar→heapId selection policy (precedence §2.2).** The queued-first precedence reproduces the
  count-decrement cadence and matches the §6.2 OR signature, but the exact sidebar→`heapId` mapping (tail-
  while-copies-remain, then −1 for the last) was NOT decompiled in this study (the study verifies the two
  primitives + the dispatcher heapId branch line 224, not the sidebar's selection policy). P4 implements
  queued-first and documents this as the one open behavioral assumption; P5's command binding (which carries
  the real heapId) settles it. **UNCHECKED on the selection policy.**

---

## 10. Files touched (P4)

(Anchor on TEXT; the tree shifts. A concurrent session is editing miner/combat/movement/unit_post — none
touched here.)

- `src/sim/production/factory.rs` — add `enum CancelOutcome`, `FactoryRegistry::cancel_one`,
  `Factory::cancel_active` (private, `Option<i32>`), `Factory::start_next_queued`; extend `mod tests` with the
  pure-method §7 cases. (Module is `#![allow(dead_code)]`, factory.rs:28 — new dormant methods are fine.)
- `src/sim/production/mod.rs` — add `CancelOutcome` to the `pub use self::factory::{...}` re-export (mirror the
  `StepOutcome`/`PRODUCTION_STEPS` re-exports the world tests already use, production_shadow_tests.rs:17).
- `src/sim/world/mod.rs` — add `debug_assert_factory_cancel_refund`; one call line in
  `debug_assert_production_shadow` (world/mod.rs:1024).
- `src/sim/world/production_shadow_tests.rs` — add the world-level no-hash acceptance test, the
  `queue_advances_only_after_delivery` test, and the determinism variant (reuse the existing helpers).

**NOT touched:** `world_hash.rs`, `snapshot.rs` (SNAPSHOT_VERSION stays 17), `economy.rs` core
(`add_credits`/`spend`/`available` reused as-is), the legacy `production_queue.rs` cancel (stays authoritative
+ wrong, fixed P5), any miner/combat/movement/unit_post file (concurrent session). **Verify:** `cargo test -p
vera20k` (separate foreground pass, per the build-discipline memory).

---

## 11. P4 TASK OUTLINE (for the planner to expand)

- **P4-T1** `factory.rs`: implement `Factory::cancel_active(&mut self, economy: &mut Economy) -> Option<i32>`
  per §2.3 (no-object → None; completed-held → None; else refund `original_balance − balance`, reset to idle,
  destroy object, leave queue). Unit tests `cancel_active_refunds_spent_only`,
  `cancel_active_at_progress_zero_refunds_nothing`, `cancel_active_round_trip_conserves`.
- **P4-T2** `factory.rs`: add `enum CancelOutcome` (serde-free, mirror `StepOutcome`) and implement
  `FactoryRegistry::cancel_one(owner, category, type_id, &mut Economy) -> CancelOutcome` per §2.2 (get_mut →
  None=NoMatch; queued first via `position()` + `remove`; else `cancel_active` → `Some`=AbandonedActive /
  `None`=NoMatch). Unit tests `cancel_one_removes_first_matching`,
  `cancel_one_queued_preferred_over_active_same_type`, `cancel_one_completed_active_is_noop`,
  `cancel_one_no_match_is_noop`.
- **P4-T3** `factory.rs`: implement `Factory::start_next_queued(&mut self) -> Option<InternedId>` per §3.2
  (held-object guard → None; `pop_front` into a fresh active object, progress 0, balance/rate left for
  rebuild). Unit tests `start_next_queued_pops_front`, `start_next_queued_blocked_while_object_held`.
- **P4-T4** `production/mod.rs`: re-export `CancelOutcome`.
- **P4-T5** `world/mod.rs`: add `debug_assert_factory_cancel_refund` per §4 (clone-only, precedence-aware or
  driven on the `f` clone via `cancel_active` directly; surface tick+owner+category; never write back); add the
  one call line to `debug_assert_production_shadow`.
- **P4-T6** `production_shadow_tests.rs`: add `factory_cancel_one_does_not_change_state_hash` (§5.2),
  `queue_advances_only_after_delivery` (§7), and `production_shadow_with_cancel_is_deterministic`. Confirm
  `snapshot_roundtrip_ignores_shadow` and `production_shadow_preserves_advance_tick_phase_order` still pass
  (None-arm + serde-skip intact).
- **P4-T7 (verify, separate foreground pass):** `cargo test -p vera20k` — read the literal `test result:`
  line; confirm SNAPSHOT_VERSION still 17 and no `world_hash.rs` diff.

---

*End of P4 design. The slice is additive and oracle-only: `cancel_one`/`cancel_active`/`start_next_queued` are
pure `Factory`/`FactoryRegistry` methods exercised against cloned registries + economies; the legacy
`cancel_by_type_for_owner` stays authoritative (its `.rev()` last-match + full-cost refund are the verified
DRIFTs P4 models CORRECTLY in the shadow so P5 can adopt them); `world_hash.rs`/`snapshot.rs` are untouched
and `SNAPSHOT_VERSION` stays 17. P4 PROVES the first-match queued removal (C6), the spent-only refund (C8/C15),
and the C7/C12 held-object queue-advance guard; it DEFERS the delivery command that drives the advance and the
authority flip to P5. The completed-build cancel is modeled as a no-op (not a full refund) and surfaced as the
one open UNKNOWN, per the burden-of-proof default.*
