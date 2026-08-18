---
title: Factory/House Production+Economy Substrate — P1+P2 Shadow Design Spec
date: 2026-06-04
status: design (unified from 3 competing lenses; P1+P2 shadow scope is the only implement-now boundary)
scope: P1 (`Economy` value-type shadow) + P2 (`Factory`/`FactoryRegistry` derived shadow) ONLY.
       Both additive, `#[serde(skip)]`, ZERO state_hash change, no SNAPSHOT_VERSION bump.
       P3-P9 (per-step charge, cancel/refund authority, prereq revalidation, purifier fix,
       authority flip, ordering lock, parity harness) are OUT of scope — seams only.
source doc: docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md (v2-verified, C1-C20)
verification: every current-code claim below is quoted file:line from a live read this session.
              Ghidra read-only; cargo NOT run (separate build pass).
rule: Rust-native structure, gamemd-native semantics. sim/ never depends on render/ui/sidebar/audio/net.
---

# Factory/House Substrate — P1+P2 Shadow Design Spec

## 0. TL;DR

Three competing P1+P2 designs were scored against the v2-verified study. **The winner is the
substrate-fit-first design (D2)**, grafted with the minimal-risk design's (D3) trivially-provable
no-hash discipline and the parity-fidelity design's (D1) `SpecialItem` 3-state guard and complete
ledger. The result: two new value-types (`Economy`, `Factory`) plus one derived registry
(`FactoryRegistry`), held as `#[serde(skip)]` shadow state, rebuilt-from-scratch each tick from the
authoritative `queues_by_owner` + `HouseState.credits`, asserted to track the legacy items, with
divergence **surfaced** (tick+owner+category) and never equalized — exactly mirroring the proven
`unit_ai_shadow_step` + `techno_ai_shell_is_passthrough_no_hash_change` rhythm.

The **FIT §6.3 fork is decided: option (a)** — the per-(house,category) factory step is driven from
the `object_ai_stage()` Structure arm in live-object/LogicVector order, and `FactoryRegistry` is a
**lookup**, not a tick-loop owner. In P1+P2 (no authoritative step exists) the Structure arm records
a read-only LogicVector-order trace; the authority flip at P3+ swaps the arm body, not the iteration
source. Rationale and the runner-up grafts are in §1-§3.

P1+P2 ship **zero hashed bits**: `world_hash.rs` is untouched, `SNAPSHOT_VERSION` stays 17, and the
no-hash guarantee is reduced to its trivial case (no serialized field can change the hash).

---

## 1. Scoring the three competing designs

Each lens scored 1-5 (5 best) on five axes, with a one-line justification per cell.

| Axis | D1 (parity-fidelity) | D2 (substrate-fit) | D3 (minimal-risk) |
|---|---|---|---|
| **Parity-fidelity** | **5** — full FactoryClass-shaped state machine + SpecialItem 3-state + step_timer; every C1-C20 has a home from day one | **4** — study-exact field set, honest monotone-tracking, surfaces the balance-is-constant truth; no SpecialItem field yet (constraint noted) | **3** — honest but proves *less* (balance constant-0, harvested/spent oracle-only); collapses files, defers SpecialItem to "don't add a collapse" |
| **Substrate-program fit** | **4** — option (a) wired correctly, but stands up 3 files + a `next_insertion_seq` identity carried across rebuilds (more surface than the program needs at P2) | **5** — minimum new containers (one `#[serde(skip)]` field on ProductionState + one value-field on HouseState); reuses `refresh_*_shadow`/`debug_assert_*_shadow` verbatim; extends the existing no-hash test | **3** — transient locals (no field on any struct) is clean but DEFERS the §6.3 fork toward (b) behind an `#[ignore]` guard — sets the trajectory away from the program's LogicVector invariant |
| **Testability / determinism** | **5** — full test matrix; insertion_seq stability test; option-(a) trace order is a TRUE assertion | **5** — same matrix, BTreeMap+InternedId throughout, no float, no RNG; snapshot_roundtrip_ignores_shadow proves skip is honored | **4** — strong, but rebuild-from-sorted-source makes insertion_seq a pure function (good) while the FIT guard is `#[ignore]`-pending (a tripwire, not a live test) |
| **Risk / blast-radius** | **3** — largest surface; Structure-arm wiring + carried identity counter is the most code touching the substrate spine in a pure-shadow slice | **4** — one container, asserts beside existing ones; Structure-arm read-only trace is hash-neutral by the existing test | **5** — smallest reversible change (2 files, 2 builder fns, one `#[cfg]` call, zero serialized fields); no struct edited that is serialized |
| **Buildability (lands green fast)** | **3** — most types/wiring to get compiling and green | **4** — moderate; reuses existing shadow scaffolding so less novel plumbing | **5** — fewest moving parts; the no-hash proof is the trivial case |
| **TOTAL** | **20** | **22** | **20** |

**Winner: D2 (substrate-fit-first), total 22.** It best honors the master directive — "slot into the
substrate program, do not invent a parallel architecture" — by reusing the exact `refresh_*_shadow`
+ `debug_assert_*_shadow` + extend-the-existing-no-hash-test machinery, and it decides the FIT fork
toward the program's own LogicVector invariant rather than deferring it.

### 1.1 What was grafted from the runners-up

- **From D3 (minimal-risk):** the no-hash guarantee is strengthened past D2's `#[serde(skip)]`-field
  framing toward the trivial case wherever it costs nothing — the new types **do not derive
  `Serialize`/`Deserialize` in P1+P2** (added only at the P5 authority flip). A `#[serde(skip)]`
  field still mutates the in-memory struct and risks a Default-on-deserialize footgun; omitting the
  derive entirely means the bincode layout is provably byte-identical. We keep D2's *placement* (the
  registry IS a field on `ProductionState`, the economy IS a field on `HouseState` — the study's §6.1
  ownership diagram), but the field is `#[serde(skip)]` AND the type carries no serde derive yet, so
  both guards hold. We also adopt D3's explicit `economy_no_house_created_on_missing_owner` /
  `production_shadow_does_not_create_houses` test as a first-class §4.3-hazard guard, and D3's honest
  UNKNOWN flagging that `harvested_credits`/`spent_credits` have **no legacy mirror** (oracle-asserted,
  not legacy-tracked).
- **From D1 (parity-fidelity):** the `SpecialItem` 3-state enum (`NoneNeg1`/`NoneZero`/`Item`) — the
  study §9.4 is explicit the +0x68 writer was never located, so value 0 cannot be proven unreachable;
  collapsing 0 and -1 is exactly the "edge case, probably equivalent" downgrade CLAUDE.md forbids. We
  graft it now (cost: one enum) so the later SW slice cannot silently fuse them. We also adopt D1's
  fuller tiny-detail ledger (L-rows below) and its explicit "L7 monotone-not-bit-equal" framing for
  the progress map.

### 1.2 Why D2 over D1 specifically

D1's full state machine is the *correct end-state*, but standing up a `next_insertion_seq` identity
counter carried across per-tick rebuilds, a third file, and the Structure-arm dispatch wiring is more
surface than a pure-shadow slice warrants — and the parity payoff (C3 pay-as-you-go, C8 partial
refund, etc.) is unreachable in P1+P2 because the legacy upfront-charge is still authoritative. D2
reaches the same end-state at P3+ via a body swap, with less to revert if the model needs adjustment.
D1's best ideas (SpecialItem guard, ledger completeness) are grafted without its cost.

---

## 2. FIT §6.3 decision — option (a), with a P2 read-only-trace ramp

**Decision: option (a).** The per-(house,category) factory step is driven from the **Structure arm of
`object_ai_stage()`** (`src/sim/world/techno_ai.rs:107`, currently a strict no-op) in
live-object/LogicVector order. `FactoryRegistry` is a **lookup keyed by `(InternedId,
ProductionCategory)`** that the Structure arm indexes into — NOT an iteration owner that runs its own
tick loop.

**Rationale, tied to the substrate program:**

1. **It IS the program's invariant.** The whole substrate program dispatches per-object behavior
   through `object_ai_stage()` in LogicVector order — the Structure arm at `techno_ai.rs:107` is the
   reserved S8 home ("absorb the BuildingClass::Update bracket"). A standalone Phase-7 registry sweep
   is the per-owner free-function-scan anti-pattern the study (§6.3, Digest G #1) and §4 of the study
   flag as the structural DRIFT this substrate exists to retire. Choosing (b) reintroduces the thing
   the program is built to remove.

2. **(b)'s equivalence is UNPROVEN and the study proves it false in general.** Option (b) requires
   asserting "insertion_seq order == LogicVector order," but a building revealed→concealed→revealed
   gets a new LogicVector position while keeping its factory `insertion_seq` (study §6.3 verbatim).
   Under the burden-of-proof rule (default DRIFT), we do not bake a known-false ordering equivalence
   into the trajectory. Option (a) makes the question moot — the order IS LogicVector order because
   the building drives the step.

3. **The registry-as-lookup is the smaller container** — exactly the fit lens's bias. Under (a),
   `FactoryRegistry` is a keyed store the Structure arm indexes, not a thing that owns a loop.

**The P2 read-only-trace ramp (honest, not a hedge):** in P1+P2 there is no authoritative step (the
legacy upfront-charge stays authoritative until P5). So the Structure arm in P2 does NOT step or
charge — it performs the **shadow observation**: for each live Structure that is its category's active
producer, it looks up the derived `Factory` and records a `FactoryShellTrace { owner, category,
progress, insertion_seq, step_seq }` in LogicVector order — exactly as `unit_ai_shadow_step` records a
`ShellTrace`. The P2 test then asserts the trace order IS LogicVector order **by construction** (a TRUE
assertion, not the option-(b) guard against a false assumption). When P3 makes stepping authoritative,
the same Structure-arm slot flips from "record trace" to "step this factory against the economy" — the
iteration source is unchanged. No re-architecture, and **no `factory_step_order_matches_logic_vector_
order` UNPROVEN guard is ever needed** — (a) makes it vacuous.

**One flagged correctness concern (graft from D2's T-FIT-1), deferred to P3:** a building can route
production to a category it is not the live producer of. The lookup is keyed `(owner, category)`; the
Structure arm performs a category's step only when it visits that category's **active producer**
building (the existing `active_producer_by_owner` tells us which that is — `production_types.rs:201`).
A non-producer building's Structure-arm visit is a no-op for the factory step (one step per category
per tick). This routing is not load-bearing in P1+P2 (trace only) but is recorded for the P3 author to
verify against the active-producer binding.

---

## 3. Chosen architecture (types, fields, modules, target paths)

### 3.1 New files

| Type(s) | Target file | Rationale |
|---|---|---|
| `Economy` value-type + `Simulation::refresh_economy_shadow` + `debug_assert_economy_shadow` + tests | `src/sim/economy.rs` (NEW) | Study §8 P1 §Files names `src/sim/economy.rs`. A *value-type*, like `HouseState` is — held BY `HouseState`, the existing economy home. Declared in `src/sim/mod.rs` as `pub mod economy;`. |
| `Factory`, `PendingObject`, `SpecialItem`, `StepOutcome`, `BuildEligibility`, `FactoryRegistry`, `FactoryView<'_>` + `rebuild_factory_shadow` free fn + tests | `src/sim/production/factory.rs` (NEW) | Study §8 P2 lists `factory.rs` + `factory_registry.rs`; for pure-shadow scope the registry is a ~100-line derived rebuild — one cohesive file (~350 lines, under the 600 guideline) reads like `production_types.rs` (which co-locates `ProductionState` + all its value-types). **Split out `factory_registry.rs` at P4** when `cancel_one`/`revalidate` add bulk. Declared in `production/mod.rs` as `mod factory;` + re-exports. |

### 3.2 New fields (the only two struct edits)

```rust
// src/sim/house_state.rs — additive shadow field on HouseState:
/// Per-house wallet/storage/statistics shadow. Tracks the authoritative
/// `credits` field; non-serialized, non-hashed until the authority flip.
#[serde(skip)]
pub economy: Economy,
```

```rust
// src/sim/production/production_types.rs — ProductionState gains ONE container:
/// Per-(house,category) factory shadow, rebuilt each tick from `queues_by_owner`.
/// Derived; non-serialized, non-hashed until the authority flip.
#[serde(skip)]
pub factory_shadow: FactoryRegistry,
```

`HouseState` derives `Default`; `Economy: Default`, so `economy` defaults cleanly with no `new()`
change. `ProductionState` has a hand-written `Default` (`production_types.rs:240`) — add
`factory_shadow: FactoryRegistry::default()` to it. Both fields are `#[serde(skip)]` AND their types
carry **no `Serialize`/`Deserialize` derive in P1+P2** (graft from D3 — the bincode layout is
provably byte-identical; the derive is added at P5 when they become authoritative). `#[serde(skip)]`
is kept on the field as belt-and-suspenders so the field stays inert even after the P5 derive lands
until its skip is removed in lockstep with the hash add.

### 3.3 `Economy` (P1)

```rust
//! Per-house wallet/storage/statistics value-type. Shadow-first: introduced as a
//! non-serialized field on HouseState that tracks the authoritative `credits`.
//! The purifier-bonus base is the OrePurifier *building count* (NOT silo storage
//! capacity); IncomeMult is NOT stored here — it is read per-deposit from the
//! country type. Depends only on std. NEVER on render/ui/sidebar/audio/net.

#[derive(Debug, Clone, Default, PartialEq, Eq)]   // NO serde derive in P1+P2 (added at P5)
pub struct Economy {
    /// Spendable balance. Tracks the legacy `HouseState.credits` exactly in P1
    /// (same i32 scale — P1 introduces no rescale; the engine's internal x100 vs
    /// raw scale is a P5/lifecycle DRIFT, out of scope).
    pub credits: i32,
    /// Running total spent (statistics). NO legacy field exists — oracle-only in P1.
    pub spent_credits: i32,
    /// Deposit x5.0 statistics accumulator. NO legacy field exists — oracle-only in P1.
    pub harvested_credits: i32,
    /// OrePurifier building count; purifier-bonus base. NEVER silo storage capacity.
    pub purifier_count: i32,
}

impl Economy {
    pub fn add_credits(&mut self, amount: i32);
    /// Spend up to `amount`; the silo-drain fallback body is P3+. In P1 the body is
    /// the trivial `min(credits, amount)` deduction so the type unit-tests, but
    /// `advance_tick` NEVER calls it on a real economy (legacy charge stays authoritative).
    pub fn spend(&mut self, amount: i32) -> i32;
    pub fn available(&self) -> i32 { self.credits }
}
```

**No new getter.** The §4.3 hazard (`credits_entry_for_owner`, `production_queue.rs:74-92`,
auto-creates an `is_human=true` house on miss → mutates the hashed `houses` map from a getter) is
forbidden by construction: the shadow build takes `&Simulation` (or iterates `&mut self.houses`
read-style) and **only mirrors houses that already exist** — a missing owner simply has no `economy`
update. No path inserts into `houses`. This is ledger L-ECON-1 and a dedicated test.

### 3.4 `Factory` / `PendingObject` / `SpecialItem` / `FactoryRegistry` (P2)

```rust
pub const PRODUCTION_STEPS: u16 = 54;
pub const STEP_RATE_MIN: u16 = 1;
pub const STEP_RATE_MAX: u16 = 255;

#[derive(Debug, Clone, Default, PartialEq, Eq)]   // NO serde in P1+P2
pub struct PendingObject {
    pub type_id: InternedId,
    /// None in shadow; the produced entity is created by the legacy path at
    /// completion. Held distinct so the complete-but-not-delivered state is
    /// representable now (study C12).
    pub entity_id: Option<u64>,
}

/// Engine special/SW discriminator. The study §9.4 proves the +0x68 writer was
/// never located, so value 0 cannot be proven unreachable and 0-vs-(-1) MUST NOT
/// be collapsed. 3 states keep them distinct. In P1+P2 (normal builds) always NoneNeg1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]   // NO serde in P1+P2
pub enum SpecialItem { NoneNeg1, NoneZero, Item(u32) }
impl Default for SpecialItem { fn default() -> Self { SpecialItem::NoneNeg1 } }

/// One production state machine per (house, category). Value-type owned by the
/// FactoryRegistry. Mirrors the engine FactoryClass on the hash-relevant fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]   // NO serde in P1+P2
pub struct Factory {
    pub owner: InternedId,
    pub category: ProductionCategory,
    /// 0..=54; completion at PRODUCTION_STEPS (study C2).
    pub progress: u16,
    /// Per-step frame rate = GetBuildStepTime()/54 clamp[1,255]; 0 when no object (C5).
    pub step_rate_frames: u16,
    /// Frames remaining in the current step (engine CDTimer).
    pub step_timer: u16,
    /// Remaining cost still owed (charged down per step at P3). Shadow value in P2.
    pub balance: i32,
    /// Full-cost snapshot at start, for exact-cost conservation (study C15).
    pub original_balance: i32,
    pub object: Option<PendingObject>,
    pub on_hold: bool,
    pub suspended: bool,
    pub manual: bool,
    pub special: SpecialItem,
    /// FIFO type ids waiting behind the active object (study C6).
    pub queue: VecDeque<InternedId>,
    /// Deterministic registration order for same-frame completion sequencing.
    pub insertion_seq: u64,
}

pub enum StepOutcome { Idle, Stepped, Stalled, Completed }

/// 3-way prereq eligibility (P6 consumer; defined now so the registry surface is
/// stable). Active object runs BOTH (1,0,1) and (1,1,1) gates; queued items only
/// (1,0,1) (study C19).
pub enum BuildEligibility { Buildable, TemporarilyBlocked, PermanentlyBlocked }

#[derive(Debug, Clone, Default, PartialEq, Eq)]   // NO serde in P1+P2
pub struct FactoryRegistry {
    factories: BTreeMap<(InternedId, ProductionCategory), Factory>,
    next_insertion_seq: u64,
    /// Carried across the per-tick rebuild so a surviving (owner,category) keeps a
    /// stable insertion_seq (study §6.1 same-frame ordering). Skipped + unhashed.
    seq_carry: BTreeMap<(InternedId, ProductionCategory), u64>,
}
```

`Defense` stays a distinct `ProductionCategory` key in P1+P2 (study §7 note — collapsing it is a
hash-set change owned by a later slice; ledger L-FAC-7).

### 3.5 Registry API (P2 needs only the first three; the rest are documented seams)

```rust
impl FactoryRegistry {
    /// P2 SHADOW BUILD: (re)derive the whole registry from the legacy queues each
    /// tick. READ-ONLY w.r.t. all hashed state. Reuses seq_carry to keep
    /// insertion_seq stable for surviving factories.
    pub(crate) fn rebuild_shadow(&mut self, sim: &Simulation);

    /// Read-only sidebar projection (render seam). Never mutates.
    pub fn view(&self, owner: InternedId, category: ProductionCategory) -> Option<FactoryView<'_>>;

    /// Native-order step service. In P2 it is exercised ONLY by the
    /// insertion-order test; it does NOT charge a real economy.
    pub fn step_all(&mut self, economies: &mut BTreeMap<InternedId, Economy>);

    // ---- declared for later slices; documented seams, NOT called in P1-P2 ----
    pub fn begin(&mut self, owner: InternedId, category: ProductionCategory, type_id: InternedId);
    pub fn cancel_one(&mut self, owner: InternedId, category: ProductionCategory,
                      type_id: InternedId, economy: &mut Economy);
    pub fn revalidate(&mut self, owner: InternedId,
                      can_build: &dyn Fn(InternedId) -> BuildEligibility);
}
```

`FactoryView<'_>` is a borrow-only struct in the same file (progress %, on_hold, suspended, queue
contents, ready). The `IsDifferent`/`HasChanged` dirty bit (study F11) is **render-only and never a
`Factory` field** — computed sidebar-side from a per-tick change set, never hashed (ledger L-FAC-6).

---

## 4. Shadow build + derive/assert discipline

### 4.1 P1 — `Economy` shadow (tracks, does not recompute)

Derive direction is **legacy → shadow** (legacy `credits` authoritative through P4). `refresh_economy_
shadow` runs at end-of-tick AFTER the legacy charge/deposit/refund for the tick:

```
for (owner_id, house) in &mut self.houses {        // BTreeMap, sorted, deterministic
    house.economy.credits = house.credits;          // mirror the authoritative wallet
    house.economy.purifier_count = <count owned OrePurifier buildings>;
    // spent_credits / harvested_credits have NO legacy mirror (study §4.3 MISSING) —
    // exercised only by the P1 oracle unit tests, not accumulated from a live path here.
}
```

This iterates the existing `houses` map only; it never calls `credits_entry_for_owner` and never
inserts a house. **Asserts** (debug-only, mirror `debug_assert_s1_shadow`): `economy.credits ==
house.credits` for every house; divergence reported with `tick + owner` and asserted, never written
back.

### 4.2 P2 — `Factory`/`FactoryRegistry` shadow (derived from `queues_by_owner`)

`rebuild_factory_shadow` walks `self.production.queues_by_owner` (the authoritative
`BTreeMap<owner, BTreeMap<category, VecDeque<BuildQueueItem>>>`) and, for each (owner, category) with
a non-empty queue, derives one `Factory`:

| `Factory` field | Derived from the legacy `BuildQueueItem` (`production_types.rs:23-34`) |
|---|---|
| `owner`, `category` | the `queues_by_owner` key |
| `object` | `Some(PendingObject{ type_id: front.type_id, entity_id: None })` if the front item is `Building`/active; else `None` |
| `queue` | the tail items' `type_id`s, FIFO order preserved |
| `progress` (0..=54) | `((front.total_base_frames - front.remaining_base_frames) * 54) / front.total_base_frames` (integer, guard `total_base_frames != 0`) — the monotone bridge |
| `step_rate_frames` | from the existing build-time path = `clamp(GetBuildStepTime()/54, 1, 255)`; 0 when `object.is_none()` |
| `step_timer` | best-effort from `front.progress_carry`; only monotonicity is asserted, not bit-equality (study line 351 — different timer model) |
| `balance` / `original_balance` | legacy pre-pays full cost at enqueue (`production_queue.rs:218`), so legacy "remaining-to-pay" is 0. Shadow records `original_balance = cost`, `balance = cost - (progress*cost/54)` as the gamemd-semantics projection; the assert is **monotone-tracking** (balance decreases as progress increases), NOT equality with a (nonexistent) legacy balance |
| `on_hold` | `front.state == NoFunds` (`BuildQueueState::NoFunds`, label "On Hold", `production_types.rs:148`) |
| `suspended` | `front.state == Paused` OR `Done` (awaiting placement) |
| `manual` | `true` if `Paused` by user; else default-resume |
| `special` | `SpecialItem::NoneNeg1` (normal builds) |
| `insertion_seq` | reuse `seq_carry[(owner,category)]` if it existed last tick; else `next_insertion_seq++` and record into the new carry |

**The legacy PPM timer model and the 54-step model are NOT bit-equivalent** (study line 351:
"different timer model; equivalence unproven"). The shadow does not pretend they are. It asserts the
**weaker, true invariant** (study P2 test): shadow `progress` maps **monotonically** to legacy
`remaining_base_frames` (as remaining decreases, progress is non-decreasing) and reaches 54 exactly
when the legacy item reaches `Done`. Divergence in the exact step is **surfaced** (tick + owner +
category), never equalized — the same discipline as `unit_ai_shadow_step` returning the OBSERVED value
(`techno_ai.rs:159` "a divergence is surfaced, never silently equalized").

**`insertion_seq` stability across rebuild:** a naive from-scratch rebuild would reassign seqs every
tick and the iteration-order test would be meaningless. `rebuild_shadow` therefore reuses `seq_carry`
for any (owner,category) that survives, minting new seqs only for newly-appearing factories — making
`insertion_seq` a stable identity from first appearance (the exact property P5 will need when the
counter becomes hashed). (This is D2's carried-map; D3's pure-function-of-sorted-source alternative is
also valid but loses identity stability across a factory's pause/resume, so we keep the carry.)

### 4.3 Surfacing, never equalizing

Both shadows follow the program's discipline verbatim: a divergence is reported with `tick + owner
[+ category]` via `debug_assert!`, never written back to the legacy authority. This mirrors
`debug_assert_s1_shadow` (`techno_ai.rs:193-222`) and `unit_ai_shadow_step` (`techno_ai.rs:159-181`).

---

## 5. Exact no-hash-change guarantee + its test

### 5.1 Discipline

- `HouseState.economy` and `ProductionState.factory_shadow` are both `#[serde(skip)]` AND their types
  carry no `Serialize`/`Deserialize` derive in P1+P2 — so the bincode layout is provably byte-identical
  and `SNAPSHOT_VERSION` stays **17** (`snapshot.rs:24`). The 17→18 bump is a P5 concern, explicitly
  out of scope.
- `world_hash.rs` is **NOT touched**. `hash_houses` (157-184) hashes `credits, side_index, is_human,
  is_defeated, has_won, has_lost, owned_building_count, owned_unit_count, tech_level, rally_point,
  base_center` — `economy` is never referenced. `hash_production` (187-271) hashes `queues_by_owner`
  items, `ready_by_owner`, `active_producer_by_owner`, `next_enqueue_order`, resources, ore growth,
  terrain, dock contacts — `factory_shadow` is never referenced. The new fields are invisible to
  `state_hash()`.
- No shadow path calls `credits_entry_for_owner` or any function that inserts into
  `houses`/`queues_by_owner`. The derive iterates existing maps only (§4.3 hazard guard).
- All shadow writes target the new non-hashed fields; all shadow reads are of hashed state. No RNG
  draw, no float in sim logic (the deposit ×5.0 is integer `× 5` since bales are integral), no
  live-membership change mid-pass.

### 5.2 The proving tests (mirror / extend `techno_ai_shell_is_passthrough_no_hash_change`)

- `economy_shadow_does_not_change_state_hash` — snapshot `state_hash()`, run `refresh_economy_shadow()`,
  assert bit-identical (analog of the existing test at `techno_ai.rs:243-270`).
- `factory_registry_shadow_no_hash_change` — insert entities + queues, snapshot, run
  `object_ai_stage()` (now rebuilding + tracing in the Structure arm) + `rebuild_factory_shadow`,
  assert `state_hash()` bit-identical. **Extends** the existing
  `techno_ai_shell_is_passthrough_no_hash_change` guarantee with the same fixture shape, rather than
  cloning a parallel proof.
- `production_shadow_does_not_create_houses` — run the shadow over a sim whose queued owner has NO
  `houses` entry; assert `houses.len()` and `queues_by_owner` unchanged and `state_hash()` bit-identical
  (the §4.3 auto-create-hazard guard).
- `snapshot_roundtrip_ignores_shadow` — serialize→deserialize, assert the skipped `economy`/
  `factory_shadow` come back `Default` and `state_hash()` is unchanged across the boundary (proves the
  skip is honored and the shadow is not load-bearing).
- `production_shadow_preserves_advance_tick_phase_order` — two identical fixtures over N ticks produce
  identical per-tick `state_hash()` sequences (mirrors `techno_ai_shell_preserves_advance_tick_phase_
  order`, `techno_ai.rs:297`).
- `snapshot_version_is_17_in_shadow_phase` — assert `SNAPSHOT_VERSION == 17` (locks "no bump until P5").

---

## 6. The `advance_tick` hook

Two hooks, both inside the existing pipeline, no new phase:

1. **Per-building shadow observation (P2):** the existing `object_ai_stage()` call at
   `world/mod.rs:1788` already walks live order and dispatches the Structure arm. P2 fills the
   `EntityCategory::Structure => {}` no-op arm (`techno_ai.rs:107`) with a read-only
   `factory_shadow_step(sim, structure_id, &mut seq, &mut traces)` that (a) checks the structure is its
   category's active producer, (b) looks up the derived `Factory` via `factory_shadow.view(owner,
   category)`, (c) pushes a `FactoryShellTrace`. It is `&Simulation`-shaped reads + a local trace vec
   — no hashed-state mutation, no RNG — so the existing
   `techno_ai_shell_is_passthrough_no_hash_change` still passes.

2. **End-of-tick shadow build + asserts**, placed exactly beside the existing S1 shadow at
   `world/mod.rs:2426-2433`:

```
self.refresh_mission_shadow();              // existing (2426)
self.refresh_production_shadow();           // NEW: (i) refresh_economy_shadow (mirror credits +
                                            //      purifier_count); (ii) factory_shadow.rebuild_shadow(self)
#[cfg(debug_assertions)]
self.debug_assert_s1_shadow();              // existing (2432)
#[cfg(debug_assertions)]
self.debug_assert_production_shadow();      // NEW: (a) economy.credits == credits per house;
                                            //      (b) progress monotone vs legacy remaining;
                                            //      (c) Structure-arm trace order == LogicVector order
let state_hash = self.state_hash();         // existing (2433) — MUST be unchanged
```

`refresh_production_shadow` runs AFTER Phase-7 `tick_production` (the legacy charge/spawn/refund for
the tick) so the derive sees settled legacy state. Both sub-steps are read-derives of hashed state
writing only the non-hashed shadow fields. The Structure arm at line 1788 reads the registry built at
the *previous* tick's tail — deterministic, and the P2 shadow needs only internal consistency, not
same-pass freshness. The legacy Phase-7 `tick_production_with_overlay_registry` (`world/mod.rs:2314`)
is **completely untouched** in P1+P2.

This places ZERO behavior between the legacy systems and the hash — the shadows run after all
authoritative systems, before the hash, exactly where the mission/S1 shadows run.

---

## 7. Determinism + 30-player scale

- **Keying:** `FactoryRegistry.factories: BTreeMap<(InternedId, ProductionCategory), Factory>` —
  `InternedId` and `ProductionCategory` (`production_types.rs:136`, derives `Ord`) give the tuple key
  `Ord`, so sorted iteration for replay/lockstep. `Economy` lives on `HouseState` inside the existing
  `houses: BTreeMap<InternedId, _>`. No `HashMap`, no `1<<(idx&0x1f)` bitmask, no fixed 8/32-player
  array anywhere — satisfies the 30-player scale target (MEMORY `project_scale_target`; study §6.1).
- **`insertion_seq`:** monotonic `u64`, assigned only to newly-appearing factories and carried via
  `seq_carry` across the per-tick rebuild. The Structure-arm trace iterates in the order live buildings
  appear (option a) → reproduces native registration order without a separate sweep. Same-frame
  completion order is therefore deterministic and equal to live-object order.
- **`next_insertion_seq` / `seq_carry` serde (P1+P2):** the registry is `#[serde(skip)]` and carries no
  serde derive, so the counter and carry are rebuilt deterministically each tick from the (hashed,
  serialized) legacy queues; they do NOT round-trip through save. **This is acceptable ONLY because the
  registry is non-authoritative shadow** — a load rebuilds it from the serialized legacy queues on the
  next tick, so two peers reconverge. The study's `registry_next_insertion_seq_is_serialized_and_hashed`
  requirement (§6.1) is a **P5 obligation** (the trigger when the field stops being skipped), recorded
  here as the desync tripwire but explicitly out of P1+P2 scope (ledger L-DET-1).
- **No float in sim math:** `step_rate_frames` from the existing integer/PPM build-time path
  (`production_tech.rs`), clamped `[1,255]`; `progress` integer ratio; `Economy` is `i32`; deposit ×5.0
  is integer `× 5`. No `f32`/`f64` in the shadow.
- **No RNG** consumed by either shadow (matches the `object_ai_walk` "consumes no RNG" property).

---

## 8. Seams (declared now, inert in P1+P2)

- **Sidebar (render, no sim dep):** `FactoryRegistry::view` → `FactoryView<'_>` borrow-only projection.
  In P1+P2 the sidebar keeps reading the legacy queue view; `view` is defined but not repointed until
  the authority flip (P5). The dirty/`HasChanged` flag stays render-side, never a `Factory` field,
  never hashed (study F11).
- **Per-step authority (P3):** `factory.advance_one_step(&mut economy)` — declared signature, body
  deferred to P3; the Structure arm flips from trace to this call. P3 runs against a cloned/throwaway
  oracle economy, NOT the hashed wallet (study §8 P3).
- **Cancel/refund (P4), prereq revalidation (P6):** `cancel_one`/`revalidate` declared; bodies
  deferred. `SpecialItem` 3-state enum already prevents the 0/-1 collapse a later SW slice needs.
- **Purifier economy (P7):** `Economy.purifier_count` (count base, CORRECT per v2 — NOT silo capacity);
  `Economy` stores NO `IncomeMult` (read per-deposit from country type, study C18) and NO
  `storage_capacity`. The AIVirtualPurifiers term is P7-gated on the open index-field identity (study
  §9.4).
- **AI (deferred):** `FactoryRegistry::begin` is the single documented entry an AI chooser would call;
  never invoked by the human path in P1+P2. No AI internals designed (DEFERRED-AI per study §3).
- **House-lifecycle / diplomacy:** out of scope; the `InternedId` keying already honors the §6.6 "no
  `1<<idx` bitmask" scale constraint so the future sub-program is not boxed in.

---

## 9. Tiny-detail ledger — every P1+P2 observable/correctness detail this design must get right

| # | Detail | Source | How honored |
|---|---|---|---|
| L1 | Shadow adds ZERO hashed bits; `SNAPSHOT_VERSION` stays 17 | task; study §6.4 | both fields `#[serde(skip)]` + types carry no serde derive; `world_hash.rs` untouched; `snapshot_version_is_17` test |
| L-ECON-1 | No house auto-creation in any shadow/derive path | study §4.3; `production_queue.rs:74-92` | shadow iterates existing `houses` only; never calls `credits_entry_for_owner`; `production_shadow_does_not_create_houses` test |
| L-ECON-2 | `purifier_count` = OrePurifier building COUNT, never silo storage capacity; no `storage_capacity` field | study C14 (v2 REFUTED v1); task | `Economy.purifier_count` named/commented as building count; NO storage field |
| L-ECON-3 | IncomeMult NOT stored on the wallet (per-deposit country-type read) | study C18 | `Economy` has no income_mult field; documented deposit caller passes it (P7) |
| L-ECON-4 | `credits` same i32 scale as legacy (no x100 in P1) | study line 352; `house_state.rs:28` | `Economy.credits` mirrors `house.credits` verbatim; scale DRIFT deferred to lifecycle sub-program |
| L-ECON-5 | `harvested_credits`/`spent_credits` have NO legacy mirror → oracle-asserted, marked UNKNOWN-legacy | study §4.3 MISSING (graft D3) | fields present; P1 tests are isolated method unit tests, NOT legacy-tracking asserts |
| L-FAC-1 | `progress` is 0..=54 inclusive; completion at exactly 54 | study C2 | `PRODUCTION_STEPS=54`; `progress: u16` 0..=54; derive maps to 54 at legacy `Done` |
| L-FAC-2 | shadow `progress` maps MONOTONICALLY to legacy `remaining_base_frames`, NOT bit-equal | study line 351, P2 | monotonicity + completion-coincidence asserted; divergence surfaced (tick+owner+cat), never equalized |
| L-FAC-3 | FIFO queue; front = active object, rest = queue, order preserved | study C6 | `object` split from `queue: VecDeque`; derive preserves legacy `VecDeque` order |
| L-FAC-4 | pay-as-you-go modeled: `balance` remaining-to-pay, `original_balance` full cost | study C3/C15 | both fields present; shadow `balance = cost - progress*cost/54`, `original_balance = cost`; monotone-tracking, NOT bit-equal (legacy pre-pays, has no remaining balance) |
| L-FAC-5 | step_rate = clamp(total/54, 1, 255); 0 when no object | study C5 | `STEP_RATE_MIN/MAX`; derive sets rate via existing build-time path, 0 when `object.is_none()` |
| L-FAC-6 | `Factory` carries NO dirty/IsDifferent flag (render-only, never hashed) | study F11 | `FactoryView` borrow-only; dirty bit computed render-side |
| L-FAC-7 | `Defense` stays a distinct category key for now (render-origin split documented) | study §7 | `ProductionCategory::Defense` stays a key; commented to revisit at a hash-set slice |
| L-FAC-8 | SpecialItem 0-vs-(-1) NOT collapsed | study §9.4 (graft D1) | `SpecialItem::{NoneNeg1, NoneZero, Item}` enum; `special_item_none_zero_and_neg1_distinct` test |
| L-FAC-9 | `on_hold`/`suspended`/`manual` distinct | study F4/F8 | three separate bools; derive maps `NoFunds`→on_hold, `Paused`→suspended+manual, `Done`→suspended |
| L-FAC-10 | `object` distinct from `queue` so complete-but-not-delivered is representable | study C12 | `object: Option<PendingObject>` with `entity_id` distinct from `type_id` |
| L-FAC-11 | `step_timer` separate field, not folded into progress | study §2a CDTimer | `step_timer: u16` distinct from `step_rate_frames` and `progress` |
| L-FIT-1 | FIT = (a): step driven from Structure arm in LogicVector order; registry is a lookup; trace order test is a TRUE assertion | study §6.3 (§2) | Structure arm (`techno_ai.rs:107`) drives the shadow trace; `factory_shadow_trace_order_matches_logic_vector` is true-by-construction, no UNPROVEN guard |
| L-FIT-2 | Structure-arm step routes only via the active producer (one step/category/tick) | study §6.3 (graft D2 T-FIT-1) | trace keyed by active_producer_by_owner; non-producer visit is a no-op; flagged for P3 verification |
| L-DET-1 | `insertion_seq` stable across rebuild (carried, not reassigned); `next_insertion_seq` becomes serialized+hashed at P5 | study §6.1 | `seq_carry` reuses prior seq; serde obligation flagged as P5 desync tripwire |
| L-DET-2 | Deterministic keying for 30-player scale; no `1<<idx`, no fixed array; no RNG, no float | study §6.1; scale memory | `BTreeMap<(InternedId, category), _>`; `Economy` on `houses` map; integer math only |
| L-SHADOW-1 | Divergence SURFACED (tick+owner+category), never equalized | task; `techno_ai.rs:159` | `debug_assert_production_shadow` reports + asserts; never writes back |
| L-HOOK-1 | Shadow build runs AFTER legacy Phase-7, BEFORE the hash | study §6.3; `world/mod.rs:2314/2426/2433` | `refresh_production_shadow()` inserted just after `refresh_mission_shadow()`, before `state_hash()` |

---

## 10. P1 / P2 task outline (for the planner to expand)

### P1 — `Economy` value-type (shadow)
- **P1-T1** Create `src/sim/economy.rs`: `Economy` struct (no serde derive), `add_credits`/`spend`/
  `available` methods (trivial bodies; `spend` silo-drain deferred to P3). Declare `pub mod economy;`
  in `src/sim/mod.rs`.
- **P1-T2** Add `#[serde(skip)] pub economy: Economy` to `HouseState` (`house_state.rs`). Confirm
  `Default` still derives; no `new()` change.
- **P1-T3** Add `Simulation::refresh_economy_shadow` (mirror `credits` + `purifier_count` over existing
  `houses`; no house creation) and `#[cfg(debug_assertions)] debug_assert_economy_shadow`.
- **P1-T4** Tests: `economy_shadow_tracks_legacy_credits`, `economy_shadow_does_not_change_state_hash`,
  `production_shadow_does_not_create_houses`, `economy_purifier_count_is_building_count`,
  `economy_add_harvest_truncates_x5` (isolated method, marked not-a-shadow-track),
  `economy_spend_silo_drain_and_no_cap` (isolated method).

### P2 — `Factory` + `FactoryRegistry` (derived shadow) + FIT option (a) trace
- **P2-T1** Create `src/sim/production/factory.rs`: `PRODUCTION_STEPS`/`STEP_RATE_*` consts;
  `PendingObject`, `SpecialItem`, `Factory`, `StepOutcome`, `BuildEligibility`, `FactoryRegistry`,
  `FactoryView<'_>` (no serde derives). Declare `mod factory;` + re-exports in `production/mod.rs`.
- **P2-T2** Add `#[serde(skip)] pub factory_shadow: FactoryRegistry` to `ProductionState`
  (`production_types.rs`); add to its hand-written `Default`.
- **P2-T3** Implement `FactoryRegistry::rebuild_shadow(&mut self, sim: &Simulation)` deriving from
  `queues_by_owner` per the §4.2 mapping table, with `seq_carry` insertion_seq stability. Add
  `view` and a P2-only `step_all` (insertion-order iterator, no economy charge).
- **P2-T4** Fill the `EntityCategory::Structure` arm (`techno_ai.rs:107`) with read-only
  `factory_shadow_step` recording `FactoryShellTrace` in LogicVector order (active-producer gated).
- **P2-T5** Add `Simulation::refresh_production_shadow` (calls `refresh_economy_shadow` +
  `factory_shadow.rebuild_shadow`) wired at `world/mod.rs:2426`; extend `debug_assert_production_shadow`
  with the monotone-progress and trace-order asserts.
- **P2-T6** Tests: `factory_shadow_progress_tracks_legacy_remaining` (monotone + completion-coincidence,
  divergence surfaced), `factory_registry_iteration_is_insertion_ordered` (3 owners × 2 categories),
  `factory_registry_shadow_no_hash_change`, `insertion_seq_stable_across_rebuild`,
  `special_item_none_zero_and_neg1_distinct`, `factory_shadow_trace_order_matches_logic_vector`,
  `snapshot_roundtrip_ignores_shadow`, `production_shadow_preserves_advance_tick_phase_order`,
  `snapshot_version_is_17_in_shadow_phase`.

### Declared-but-inert seams (no bodies in P1+P2)
`Economy::spend` silo-drain (P3); `Factory::advance_one_step` (P3); `FactoryRegistry::begin` (AI seam),
`cancel_one` (P4), `revalidate` (P6); `FactoryView` sidebar repoint (P5); `next_insertion_seq` serde +
hash (P5 tripwire).

---

## 11. UNKNOWN / UNCHECKED (marked, not guessed)

- **U1 — `step_timer` legacy mapping.** No clean 1:1 from `progress_carry`/`remaining_base_frames` (PPM)
  onto the engine CDTimer `step_timer`. P2 derives best-effort and asserts only monotone `progress`;
  exact per-frame `step_timer` equivalence is UNPROVEN (study line 351). Fine for shadow; authoritative
  timer lands at P3.
- **U2 — `balance` shadow shape.** `balance = cost - progress*cost/54` is a modeling choice; legacy
  pre-pays full cost so there is no legacy remaining-balance to mirror bit-for-bit. The shadow asserts
  conservation (`balance`→0 at completion, `original_balance == cost`), not bit-equality. Exact per-step
  charge is P3 (study C3).
- **U3 — `harvested_credits`/`spent_credits` legacy analog.** None exists in `HouseState`
  (`house_state.rs:18-49`, confirmed). Oracle-asserted in P1; authority at P5/P7. UNKNOWN whether the
  project wants them hashed at P5.
- **U4 — active-producer routing (L-FIT-2).** The exact set of buildings that may step a given
  category's factory depends on `active_producer_by_owner` rotation semantics not fully traced in
  P1+P2 scope. UNCHECKED — flagged for the P3 author.
- **U5 — `credits` scale at runtime (x100 vs raw).** Study flags this DRIFT; lifecycle-sub-program
  concern. P1 mirrors the live value whatever it is. Not load-bearing for P1 shadow.

---

*End of P1+P2 design. Both slices are additive, `#[serde(skip)]` (no serde derive), zero-hash-change
shadow; `world_hash.rs`/`snapshot.rs` are untouched and `SNAPSHOT_VERSION` stays 17. The authority
flip and 17→18 bump land at P5 (out of scope). FIT §6.3 is decided as option (a): the per-building
step is driven from the `object_ai_stage()` Structure arm in LogicVector order, with P1+P2 recording a
read-only LogicVector-order trace that the P3 authority flip turns into the live step by a body swap.*
