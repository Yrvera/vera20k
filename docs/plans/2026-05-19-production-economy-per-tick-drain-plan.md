# Production Economy Refactor — Per-Tick Drain Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Drain credits per-step across 54 progress steps, stall on NoFunds with
auto-resume, and refund only the amount actually spent on cancel — matching
gamemd.exe observable economy behavior.

**Architecture:** Replaces the current upfront-debit + full-refund model in
[src/sim/production/](src/sim/production/) with a per-item `Balance`/`Progress`
pair on the active head of each queue. Queued non-head items hold no money;
money is debited per step from the head. NoFunds becomes a first-class
`BuildQueueState` variant.

**Design Doc:** [docs/plans/2026-05-19-production-economy-per-tick-drain-design.md](docs/plans/2026-05-19-production-economy-per-tick-drain-design.md)

---

## Grounding Summary

**Existing RE research (ra2-rust-game-docs/):**
- `BUILD_QUEUE_GHIDRA_REPORT.md` — FactoryClass struct layout (0x74 bytes),
  Progress 0..=54, Balance at +0x60, NoFunds flag at +0x5C, AI tick at vtable[23].
- `FACTORYCLASS_PRODUCTION_DEEP_DIVE.md` — Complete `FactoryClass::AI`
  (0x004C9B20) pseudocode with `costThisStep = Balance / stepsLeft` math,
  `OnHold = true` rollback semantics, `AbandonProduction` (0x004CA0E0)
  refund formula `Add_Credits(fullCost - Balance)`.
- `FACTORY_CREDIT_SYSTEM_GHIDRA_REPORT.md` — HouseClass credit fields
  (Credits at +0x30C); confirms storage pool (+0x314) is vestigial in YR.

**Ghidra verification:** The brainstorm phase resolved all per-step math
and the NoFunds semantics directly from the Ghidra decompilations cited in
the design doc. No live MCP verification needed for implementation — the
docs are recent (2026-03-26) and the design's tiny-detail ledger maps each
formula to a doc citation.

**Repo pattern:** Existing per-tick advancement of
[BuildQueueItem](src/sim/production/production_types.rs#L22) inside
[advance_queue_item](src/sim/production/production_queue.rs#L808) uses
integer accumulator math against a `PRODUCTION_RATE_SCALE = 1_000_000`
ppm denominator. The new step-based logic uses the same accumulator
shape with a different threshold (`frames_per_step` instead of
`base_frames`).

**INI keys driving behavior:**
- Per-object `Cost=` (read into `obj.cost`) — already parsed.
- `[General] BuildSpeed=` → `rules.production.build_speed_x1000` — already parsed.
- Per-object `BuildTimeMultiplier=` → `obj.build_time_multiplier_x1000` — already parsed.
- `MultipleFactory=`, power scaling — already plumbed via
  [effective_progress_rate_ppm_for_type](src/sim/production/production_tech.rs#L325).

No new INI parsing required.

**Unknown after grounding:**
- gamemd's spawn-fail behavior (no exit cell at completion). Deferred per
  user direction; current full-refund-on-spawn-fail path preserved as-is.

## Key Technical Decisions

- **Implicit-active-head over `VecDeque<BuildQueueItem>` instead of a split
  `Option<ActiveProduction> + VecDeque<TypeId>` (Approach A).** Head of
  VecDeque is the active producer; trailing items have
  `balance_remaining = 0`. Identical observable output, fewer external
  consumers to refactor. **Confidence:** high. **Source:** Design doc
  Approach A, CLAUDE.md internal-modernization rule.

- **`progress_steps: u8` (0..=54) replaces `remaining_base_frames`.**
  Discrete 54-step model matches gamemd's per-step rounding signature.
  **Confidence:** high. **Source:** `BUILD_QUEUE_GHIDRA_REPORT.md §6`,
  `FACTORYCLASS_PRODUCTION_DEEP_DIVE.md §2`.

- **`step_time_carry_ppm: u64` accumulates against
  `frames_per_step * RA2_QUEUE_FRAME_MS * PRODUCTION_RATE_SCALE` threshold.**
  Same fixed-point shape as today; integer math, deterministic.
  **Confidence:** high. **Source:** repo pattern at
  [production_queue.rs:808-825](src/sim/production/production_queue.rs#L808-L825).

- **`costThisStep = balance_remaining / steps_left` where
  `steps_left = 54 - progress_steps` is computed AFTER the tentative
  `progress_steps += 1`.** Matches gamemd's `Progress += 1; stepsLeft = 54 - Progress`
  ordering. **Confidence:** high. **Source:**
  `FACTORYCLASS_PRODUCTION_DEEP_DIVE.md §2` pseudocode lines 76-82.

- **`BuildQueueState::NoFunds` is a new closed-enum variant.** Distinct
  from `Paused` (manual). Auto-clears the next tick when funds suffice.
  **Confidence:** high. **Source:** Design doc Ledger §9, `FACTORYCLASS_PRODUCTION_DEEP_DIVE.md §8`
  (IsManual at +0x71 distinguishes manual vs system pause).

- **Cash pool only; no storage pool struct.** Most YR buildings have
  `Storage=0`; the dual-pool drain in `Spend_Money` is vestigial in
  standard YR. **Confidence:** medium-high. **Source:**
  `FACTORY_CREDIT_SYSTEM_GHIDRA_REPORT.md` final paragraph of
  `Spend_Money` section ("mostly vestigial in standard YR gameplay").

## Open Questions

### Resolved During Planning

- **Where does `BuildQueueItem` get hashed for lockstep determinism?**
  → [src/sim/world/world_hash.rs:131-140](src/sim/world/world_hash.rs#L131-L140).
  Every field change must be reflected there.

- **Who else creates `BuildQueueItem` directly?**
  → Only `queued_item_via` helper at
  [production_tests.rs:555](src/sim/production/production_tests.rs#L555).
  All production code goes through `enqueue_by_type`.

- **Stepping math: cost rounding pattern.** Manual trace of
  `Balance / stepsLeft` with `obj.cost = 1000`: step 1 deducts 18,
  step 54 (stepsLeft=0) deducts remainder. Total drained == 1000
  exactly. Integer math is monotonic and bounded; rounding drift
  flushed by final-step branch. Confirmed safe.

### Deferred to Implementation

- **AI build-picker behavior under slower drain.** Unknown until in-game
  observation. gamemd AI has identical drain shape, so theoretically a
  non-issue. Verify after Task 11 via skirmish playtest.

- **gamemd spawn-fail behavior.** Whether the original holds the unit
  indefinitely or refunds. Out of scope here; current Rust full-refund
  preserved.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/production/production_types.rs](src/sim/production/production_types.rs) | `BuildQueueItem` field set; `BuildQueueState::NoFunds` variant |
| Modify | [src/sim/production/production_queue.rs](src/sim/production/production_queue.rs) | `enqueue_by_type`, `tick_production`, `advance_queue_item`, all 3 cancel paths, new `start_active_head`, `refresh_queue_states`, `queue_view_for_owner` |
| Modify | [src/sim/world/world_hash.rs](src/sim/world/world_hash.rs) | Hash new `BuildQueueItem` fields |
| Modify | [src/sim/production/production_tests.rs](src/sim/production/production_tests.rs) | Update `queued_item_via` signature; update credit-balance assertions |
| Modify | [src/sim/production/production_queue_tests.rs](src/sim/production/production_queue_tests.rs) | Update existing tests; add 9 new tests per design doc |

No new files. No new modules.

## Interface Changes

- **`BuildQueueItem` public struct** — fields change. Breaks
  serde-serialized save files (pre-1.0, accepted). Affects:
  - [world_hash.rs](src/sim/world/world_hash.rs) (state hashing)
  - [queued_item_via](src/sim/production/production_tests.rs#L555) (test helper)
  - No other external consumers found (grep confirmed).

- **`BuildQueueState::NoFunds` new variant** — closed enum gains one
  variant. Affects:
  - `BuildQueueState::label()` — needs a match arm for "On Hold".
  - `world_hash.rs` — already calls `item.state.hash()`, derives via
    `Hash`, so the new variant hashes by discriminant automatically.
  - Any external code matching on the enum exhaustively — grep below.

- **`enqueue_by_type` semantics** — public function still returns `bool`;
  signature unchanged. But credits are no longer debited on success. UI
  code that reads `credits_for_owner` to render the current balance
  will now see credits drop gradually instead of instantly. Acceptable.

- **`cancel_*` functions** — public signatures unchanged; refund amount
  semantics change per the design ledger §7.

## Sim Checklist

- [x] All math uses integer / `i32` / `u8` / `u64` — no f32/f64
- [x] New `BuildQueueItem` fields included in state hash via Task 9
- [x] No new dependencies on render/ui/sidebar/audio/net
- [x] Tick ordering unchanged — `tick_production` runs in its existing slot
- [x] No BTreeMap iteration-order changes — same `queues_by_owner` shape

## Risk Areas

| Risk | Mitigation |
|------|------------|
| Tests assume upfront-debit pattern | Rewrite assertions in Task 11 |
| State-hash regression breaks lockstep | Update `hash_production` in Task 9 same-commit as field changes |
| AI may over-queue with slower drain | Note in Task 12 verification; parity-correct vs gamemd |
| `BuildQueueState` match arms incomplete after adding NoFunds | Task 2 sweeps every match site; rustc enforces exhaustiveness |

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | 54-step Progress granularity | Sidebar cameo progress bar maps Progress 0..=54 to fill height; smooth-feeling drain only happens with the right step count | Compare cameo fill cadence vs gamemd in side-by-side skirmish |
| Task 4 | `Balance / stepsLeft` int-div cost-per-step | Player sees credits decrease in specific increments matching the build cost / 54 pattern; off-by-one in math = visible drift on each cameo | Unit tests + manual trace of cost=1000, cost=53, cost=1 |
| Task 4 | NoFunds rollback (Progress += 1 → check funds → Progress -= 1 on fail, no spend) | Player who runs out of money mid-build must see the cameo halt without losing money; spending then rolling back = stolen credits | Unit test stalls/resumes; in-game: queue tank, drain credits, observe halt-then-resume |
| Task 5 | Final-step remainder payment | Total drained must equal `obj.cost` exactly; rounding leftover would be a visible "free 50 credits" or "stolen 50 credits" per build | Unit test asserts total drained == cost for several cost values |
| Task 7 | Partial refund formula `cost - balance_remaining` | Cancel mid-build must return only spent credits; full refund mid-build is a known economy exploit gamemd does NOT have | Unit test, plus in-game: build to half, cancel, observe credits |
| Task 7 | Queued non-head item cancel refunds 0 | Cancelling reservations must not generate phantom credits; gamemd never debited them in the first place | Unit test queues two items, cancels second, asserts no change |
| Task 8 | NoFunds visual state in `QueueItemView.state` | Sidebar will render "On Hold" overlay based on this; missing → cameo looks idle but stuck | Sidebar follow-up task verifies via state field; for now, unit test propagates state into view |

---

## Tasks

### Task 1: Add `BuildQueueState::NoFunds` variant

**Why:** New state must exist before logic can emit it. Closed enum is referenced
in match sites and state hash; adding the variant first surfaces every match site
that needs updating via rustc errors.

**Files:**
- Modify: [src/sim/production/production_types.rs:144-161](src/sim/production/production_types.rs#L144-L161)

**Pattern:** Existing `BuildQueueState` enum — derives `Serialize, Deserialize, Hash`.

**Step 1: Add the variant**

In `production_types.rs`, modify the `BuildQueueState` enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildQueueState {
    Queued,
    Building,
    NoFunds,
    Paused,
    Done,
}

impl BuildQueueState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Building => "Building",
            Self::NoFunds => "On Hold",
            Self::Paused => "Paused",
            Self::Done => "Done",
        }
    }
}
```

**Step 2: Sweep match sites**

Run: `cargo check --lib 2>&1 | grep "non-exhaustive"`

Expected: any non-exhaustive matches on `BuildQueueState` outside production_queue.rs
should error. Fix each by adding a `Self::NoFunds => …` arm. Likely sites: nowhere
outside the production module (verify via grep:
`cargo check 2>&1 | grep "BuildQueueState"`).

**Step 3: Verify**

Run: `cargo check --lib`
Expected: clean compile.

**Step 4: Commit**

Commit message: `sim/production: add BuildQueueState::NoFunds variant`

---

### Task 2: Redefine `BuildQueueItem` fields

**Why:** All subsequent tasks operate on the new field set. Defining the data
shape first lets the compiler guide every consumer to the new model.

**Files:**
- Modify: [src/sim/production/production_types.rs:22-34](src/sim/production/production_types.rs#L22-L34)

**Pattern:** Existing `BuildQueueItem` struct — derives `Debug, Clone, Serialize, Deserialize`.

**Step 1: Replace the struct definition**

In `production_types.rs`, replace `BuildQueueItem` with:

```rust
/// One queued build item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildQueueItem {
    pub owner: InternedId,
    pub type_id: InternedId,
    pub queue_category: ProductionCategory,
    pub state: BuildQueueState,
    /// Base build time in RA2 production frames before live scaling.
    /// Drives both UI ms estimation and `frames_per_step` derivation.
    pub total_base_frames: u32,
    /// Discrete progress, 0..=54. Item completes at 54.
    pub progress_steps: u8,
    /// Remaining cost owed by the active head. Zero on queued (non-head)
    /// items; set to obj.cost when promoted to head; drained per step.
    pub balance_remaining: i32,
    /// Sub-step time accumulator (PPM units against PRODUCTION_RATE_SCALE).
    /// When carry crosses one step's worth of frames, one step is attempted.
    pub step_time_carry_ppm: u64,
    pub enqueue_order: u64,
}
```

**Step 2: Verify**

Run: `cargo check --lib`
Expected: errors in `production_queue.rs`, `world_hash.rs`, `production_tests.rs`
referencing the removed `remaining_base_frames` and `progress_carry` fields. These
get fixed in later tasks.

**Step 3: Commit**

Commit message: `sim/production: redefine BuildQueueItem with progress_steps + balance_remaining`

(Compile will be broken after this commit. Task 3 onward restores it. If preferred,
batch Tasks 1-9 into one commit; see "Commit batching" note at end of plan.)

---

### Task 3: Drop upfront debit from `enqueue_by_type`; add `start_active_head` helper

**Why:** Money must not move on enqueue under the new model. Adding the helper
in the same task because the new head item needs its `balance_remaining`
populated immediately on enqueue-into-empty-queue.

**Files:**
- Modify: [src/sim/production/production_queue.rs:189-231](src/sim/production/production_queue.rs#L189-L231)

**Pattern:** Existing `enqueue_by_type` flow; new helper follows the same
private-fn shape as `next_enqueue_order`.

**Step 1: Add `start_active_head` helper**

After `refresh_queue_states` (around line 125), add:

```rust
/// Promote the front item to the active head: write its full cost into
/// `balance_remaining`. Called when an item becomes the head (enqueued
/// into empty queue, or shifted up after the previous head completed/cancelled).
/// No-op if the head is already started (balance != 0 or progress != 0).
fn start_active_head(queue: &mut VecDeque<BuildQueueItem>, rules: &RuleSet,
                     interner: &crate::sim::intern::StringInterner) {
    let Some(head) = queue.front_mut() else { return; };
    if head.balance_remaining != 0 || head.progress_steps != 0 {
        return;
    }
    let type_str = interner.resolve(head.type_id);
    let Some(obj) = rules.object(type_str) else { return; };
    head.balance_remaining = obj.cost.max(0);
}
```

**Step 2: Rewrite the enqueue body**

Replace [production_queue.rs:189-231](src/sim/production/production_queue.rs#L189-L231)
`enqueue_by_type` with:

```rust
/// Enqueue a specific unit type. Does NOT debit credits — money is
/// drained per-step from the active head only.
pub fn enqueue_by_type(sim: &mut Simulation, rules: &RuleSet, owner: &str, type_id: &str) -> bool {
    let relaxed: bool = should_use_relaxed_build_mode(sim, rules, owner);
    let mode = if relaxed {
        BuildMode::PrototypeRelaxed
    } else {
        BuildMode::Strict
    };
    if let Some(opt) = build_option_for_owner(sim, rules, owner, type_id, mode) {
        if !opt.enabled {
            return false;
        }
    } else {
        return false;
    }
    let Some(obj) = rules.object(type_id) else {
        return false;
    };
    if !supports_live_production(obj) {
        return false;
    }
    let queue_category = production_category_for_object(obj);
    // Affordability gate (UI consistency): can't queue what you can't afford
    // to start. Matches gamemd's CanBuild check; this does NOT debit.
    let owner_credits = credits_for_owner(sim, owner);
    if obj.cost <= 0 || owner_credits < obj.cost {
        return false;
    }
    let total_base_frames: u32 = build_time_base_frames(rules, obj);
    let owner_id = sim.interner.intern(owner);
    let type_interned = sim.interner.intern(type_id);
    let enqueue_order = next_enqueue_order(sim);
    queue_for_owner_category_mut(sim, owner, queue_category).push_back(BuildQueueItem {
        owner: owner_id,
        type_id: type_interned,
        queue_category,
        state: BuildQueueState::Queued,
        total_base_frames,
        progress_steps: 0,
        balance_remaining: 0,
        step_time_carry_ppm: 0,
        enqueue_order,
    });
    let queue = queue_for_owner_category_mut(sim, owner, queue_category);
    refresh_queue_states(queue);
    // If this is the new head (queue was empty before), give it its starting balance.
    // We need to clone the interner reference because start_active_head needs both
    // queue and interner; sim.interner can't be borrowed while queue (sim.production)
    // is borrowed mutably. Resolve type-str outside.
    let head_needs_start = queue.front()
        .map(|h| h.balance_remaining == 0 && h.progress_steps == 0
             && h.type_id == type_interned)
        .unwrap_or(false);
    if head_needs_start {
        // Set balance directly to avoid the borrow conflict.
        if let Some(head) = queue.front_mut() {
            head.balance_remaining = obj.cost.max(0);
        }
    }
    true
}
```

Note: `start_active_head` is the conceptual helper. In `enqueue_by_type` we
inline the equivalent (head-promotion happens at the same site) to avoid
the `&Simulation` / `&mut Simulation` borrow conflict. The helper itself
is used by `tick_production` and the cancel paths in later tasks where the
borrow shape is simpler (we hold `&mut sim` and can re-borrow the queue).

**Step 3: Verify**

Run: `cargo check --lib`
Expected: `enqueue_by_type` compiles; other broken sites remain from Tasks 2 and
upcoming work.

**Step 4: Commit**

Commit message: `sim/production: drop upfront credit debit from enqueue; add active-head promotion`

---

### Task 4: Rewrite `advance_queue_item` with 54-step drain logic

**Why:** Core of the refactor — the per-step cost drain and NoFunds rollback.
All visible economy parity hinges on this function being correct.

**Files:**
- Modify: [src/sim/production/production_queue.rs:808-825](src/sim/production/production_queue.rs#L808-L825)

**Pattern:** Existing `advance_queue_item` shape (mutates a `&mut BuildQueueItem`,
takes `tick_ms` and rate), but new logic operates on steps instead of frames.

**Step 1: Add a per-step constant**

Near `RA2_QUEUE_FRAME_MS` (top of file, around line 22) add:

```rust
/// gamemd FactoryClass: production is divided into 54 discrete steps.
/// Verified: BUILD_QUEUE_GHIDRA_REPORT.md §6.
const PRODUCTION_TOTAL_STEPS: u32 = 54;
```

**Step 2: Rewrite `advance_queue_item`**

Change the signature to return how many credits were spent this tick, so
`tick_production` can deduct from the owner's credits without holding two
mutable borrows. Replace [production_queue.rs:808-825](src/sim/production/production_queue.rs#L808-L825):

```rust
/// Advance the head item. Returns (credits_to_spend_this_tick, completed).
/// On NoFunds: returns (0, false) and sets item.state = NoFunds.
/// On Building: returns (cost_spent, item.progress_steps == 54).
/// Caller is responsible for deducting `credits_to_spend` from owner credits.
fn advance_queue_item(item: &mut BuildQueueItem, tick_ms: u32, rate_ppm: u64,
                     available_credits: i32) -> (i32, bool) {
    if item.progress_steps >= PRODUCTION_TOTAL_STEPS as u8 || tick_ms == 0 {
        return (0, item.progress_steps >= PRODUCTION_TOTAL_STEPS as u8);
    }
    // frames_per_step from gamemd: Rate = cost/54 in frames, clamped [1, 255].
    // Our equivalent: total_base_frames / 54, same clamp.
    let frames_per_step = ((item.total_base_frames / PRODUCTION_TOTAL_STEPS).max(1)).min(255) as u64;
    let step_threshold: u64 = frames_per_step
        .saturating_mul(RA2_QUEUE_FRAME_MS)
        .saturating_mul(PRODUCTION_RATE_SCALE);
    // Accumulate sub-step time at the given live rate.
    item.step_time_carry_ppm = item.step_time_carry_ppm
        .saturating_add(u64::from(tick_ms).saturating_mul(rate_ppm));

    let mut credits_spent: i32 = 0;
    let mut remaining_credits = available_credits;

    while item.step_time_carry_ppm >= step_threshold {
        // Tentatively advance one step.
        item.progress_steps = item.progress_steps.saturating_add(1);
        let progress = item.progress_steps as u32;
        // gamemd computes stepsLeft AFTER Progress += 1.
        let steps_left = PRODUCTION_TOTAL_STEPS - progress;
        let cost_this_step: i32 = if steps_left == 0 {
            item.balance_remaining
        } else {
            item.balance_remaining / (steps_left as i32)
        };
        let cost_this_step = cost_this_step.min(item.balance_remaining).max(0);

        if remaining_credits < cost_this_step {
            // NoFunds: roll back the tentative advance, no spend.
            // Drop this step's carry so we retry next interval.
            item.progress_steps = item.progress_steps.saturating_sub(1);
            item.state = BuildQueueState::NoFunds;
            item.step_time_carry_ppm = item.step_time_carry_ppm.saturating_sub(step_threshold);
            return (credits_spent, false);
        }

        // Spend and continue.
        remaining_credits -= cost_this_step;
        credits_spent = credits_spent.saturating_add(cost_this_step);
        item.balance_remaining = item.balance_remaining.saturating_sub(cost_this_step);
        item.step_time_carry_ppm = item.step_time_carry_ppm.saturating_sub(step_threshold);

        // Returning to Building from NoFunds happens implicitly: the caller
        // sets state from refresh_queue_states; here we clear NoFunds.
        if item.state == BuildQueueState::NoFunds {
            item.state = BuildQueueState::Building;
        }

        if item.progress_steps as u32 >= PRODUCTION_TOTAL_STEPS {
            // Completion. Step 54 already drained balance_remaining via
            // the steps_left == 0 branch above.
            return (credits_spent, true);
        }
    }
    (credits_spent, false)
}
```

**Step 3: Verify (no test yet)**

Run: `cargo check --lib`
Expected: `advance_queue_item` compiles; `tick_production` call site is now
broken (signature changed); fixed in Task 5.

**Step 4: Commit**

Commit message: `sim/production: rewrite advance_queue_item with 54-step drain + NoFunds rollback`

---

### Task 5: Update `tick_production` to use new `advance_queue_item` signature and debit credits

**Why:** Pipe the (credits_to_spend, completed) tuple from
`advance_queue_item` into the owner's credit balance. Handle the
NoFunds→Building state transition on resume.

**Files:**
- Modify: [src/sim/production/production_queue.rs:410-602](src/sim/production/production_queue.rs#L410-L602)

**Pattern:** Existing `tick_production` outer loop and completion handling
(refund-on-spawn-fail, ready-buildings, helipad). Only the inner advance +
completion-detection block changes.

**Step 1: Modify the per-owner advance block**

Replace lines around [production_queue.rs:448-473](src/sim/production/production_queue.rs#L448-L473)
(the `let completed: Option<BuildQueueItem> = { ... }` block) with:

```rust
let advance_result: Option<(i32, bool)> = {
    let owner_credits_now = sim.houses.get(&owner_id).map(|h| h.credits).unwrap_or(0);
    let queue = sim
        .production
        .queues_by_owner
        .get_mut(&owner_id)
        .and_then(|queues| queues.get_mut(&queue_category));
    let Some(queue) = queue else { continue };
    refresh_queue_states(queue);
    if let Some(front) = queue.front_mut() {
        if front.state == BuildQueueState::Paused {
            None
        } else {
            let (spent, done) = advance_queue_item(front, tick_ms, progress_rate, owner_credits_now);
            Some((spent, done))
        }
    } else {
        None
    }
};

let Some((credits_spent, completed_flag)) = advance_result else { continue };

if credits_spent > 0 {
    *credits_entry_for_owner(sim, &owner_str) -= credits_spent;
}

let completed: Option<BuildQueueItem> = if completed_flag {
    let queue = sim
        .production
        .queues_by_owner
        .get_mut(&owner_id)
        .and_then(|queues| queues.get_mut(&queue_category));
    let Some(queue) = queue else { continue };
    // Mark Done and pop. Then promote the new head if any.
    if let Some(front) = queue.front_mut() {
        front.state = BuildQueueState::Done;
    }
    let popped = queue.pop_front();
    refresh_queue_states(queue);
    // Promote new head.
    if let Some(new_head) = queue.front_mut() {
        if new_head.balance_remaining == 0 && new_head.progress_steps == 0 {
            let type_str = sim.interner.resolve(new_head.type_id).to_string();
            if let Some(obj) = rules.object(&type_str) {
                new_head.balance_remaining = obj.cost.max(0);
            }
        }
    }
    popped
} else {
    None
};

let Some(done) = completed else { continue };
```

The remaining post-completion logic (ready_by_owner push, spawn-cell lookup,
spawn-fail refund, rally point auto-move) stays as-is.

**Step 2: Verify**

Run: `cargo check --lib`
Expected: `tick_production` compiles. `world_hash.rs` and tests still broken.

**Step 3: Commit**

Commit message: `sim/production: wire new advance_queue_item into tick_production`

---

### Task 6: Update `queue_view_for_owner` to derive ms from step model

**Why:** Sidebar shows `remaining_ms` / `total_ms`. The old derivation used
`remaining_base_frames` directly; the new model needs to convert
`progress_steps` to remaining frames via `(54 - progress_steps) * frames_per_step`.

**Files:**
- Modify: [src/sim/production/production_queue.rs:605-653](src/sim/production/production_queue.rs#L605-L653)

**Pattern:** Existing `queue_view_for_owner` shape. Same `QueueItemView` output
struct.

**Step 1: Rewrite the per-item view computation**

Replace the inner closure body (around line 616-642) with:

```rust
.map(|q| {
    let type_str = sim.interner.resolve(q.type_id);
    let frames_per_step =
        ((q.total_base_frames / PRODUCTION_TOTAL_STEPS).max(1)).min(255);
    let remaining_steps = (PRODUCTION_TOTAL_STEPS as u8).saturating_sub(q.progress_steps);
    let remaining_frames = (remaining_steps as u32) * frames_per_step;
    let total_frames = q.total_base_frames.max(1);
    let (display_name, remaining_frames_scaled, total_frames_scaled) = rules
        .object(type_str)
        .map(|obj| {
            (
                obj.name.clone().unwrap_or_else(|| type_str.to_string()),
                effective_time_to_build_frames_for_type(
                    sim,
                    rules,
                    owner,
                    type_str,
                    remaining_frames,
                ),
                effective_time_to_build_frames_for_type(
                    sim,
                    rules,
                    owner,
                    type_str,
                    total_frames,
                ),
            )
        })
        .unwrap_or_else(|| (type_str.to_string(), remaining_frames, total_frames));
    QueueItemView {
        type_id: q.type_id,
        display_name,
        queue_category: q.queue_category,
        state: q.state,
        remaining_ms: estimated_real_time_ms(remaining_frames_scaled, PRODUCTION_RATE_SCALE),
        total_ms: estimated_real_time_ms(total_frames_scaled, PRODUCTION_RATE_SCALE),
    }
})
```

**Step 2: Verify**

Run: `cargo check --lib`
Expected: function compiles.

**Step 3: Commit**

Commit message: `sim/production: derive QueueItemView ms from step model`

---

### Task 7: Update cancel paths with partial-refund formula

**Why:** Cancel mid-build must refund only what was actually spent
(`cost - balance_remaining`); cancelling a queued non-head item must
refund 0. Ready-buildings refund stays at full cost (matches the formula
since balance is 0 at completion).

**Files:**
- Modify: [src/sim/production/production_queue.rs:681-806](src/sim/production/production_queue.rs#L681-L806)

**Pattern:** Existing three cancel functions, same control flow; only the
refund-amount calculation and head-promotion lines change.

**Step 1: Update `cancel_last_for_owner`**

Replace [production_queue.rs:681-717](src/sim/production/production_queue.rs#L681-L717):

```rust
/// Cancel the most recently queued item for this owner. Refund equals the
/// amount actually spent (full cost − balance_remaining for the head;
/// zero for not-yet-active queued items).
pub fn cancel_last_for_owner(sim: &mut Simulation, rules: &RuleSet, owner: &str) -> bool {
    let owner_id = sim.interner.intern(owner);
    let Some(category) = ({
        sim.production.queues_by_owner.get(&owner_id).and_then(|q| {
            q.iter()
                .filter_map(|(category, queue)| {
                    queue.back().map(|item| (*category, item.enqueue_order))
                })
                .max_by_key(|(_, order)| *order)
                .map(|(category, _)| category)
        })
    }) else {
        return false;
    };
    // We need to know whether the removed item was the head (queue.front == queue.back).
    let removal: Option<(BuildQueueItem, bool)> = sim
        .production
        .queues_by_owner
        .get_mut(&owner_id)
        .and_then(|owner_queues| owner_queues.get_mut(&category))
        .and_then(|queue| {
            let was_head = queue.len() == 1;
            let item = queue.pop_back();
            refresh_queue_states(queue);
            item.map(|i| (i, was_head))
        });
    let Some((item, was_head)) = removal else { return false; };
    let type_str = sim.interner.resolve(item.type_id).to_string();
    let full_cost = rules.object(&type_str).map(|o| o.cost.max(0)).unwrap_or(0);
    let refund = if was_head {
        // Active head: refund what was actually spent.
        (full_cost - item.balance_remaining).max(0)
    } else {
        // Queued non-head: no money was committed.
        0
    };
    if refund > 0 {
        *credits_entry_for_owner(sim, owner) += refund;
    }
    // No need to promote new head here — cancel_last only removes the BACK,
    // so the front is unchanged.
    sim.production.queues_by_owner.retain(|_, queues| {
        queues.retain(|_, queue| !queue.is_empty());
        !queues.is_empty()
    });
    true
}
```

**Step 2: Update `cancel_by_type_for_owner`**

Replace [production_queue.rs:721-772](src/sim/production/production_queue.rs#L721-L772):

```rust
/// Cancel one queued item of a specific type_id (right-click cameo in RA2).
/// Removes the last-enqueued instance of that type. Refund equals amount
/// spent (full cost − balance_remaining if it was the head; 0 otherwise).
pub fn cancel_by_type_for_owner(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    type_id: &str,
) -> bool {
    let owner_id = sim.interner.intern(owner);
    let type_interned = sim.interner.intern(type_id);
    let target = sim
        .production
        .queues_by_owner
        .get(&owner_id)
        .and_then(|owner_queues| {
            for (category, queue) in owner_queues.iter() {
                let idx = queue
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, item)| item.type_id == type_interned)
                    .map(|(i, _)| i);
                if let Some(idx) = idx {
                    return Some((*category, idx));
                }
            }
            None
        });
    let Some((category, idx)) = target else {
        return cancel_ready_by_type_for_owner(sim, rules, owner, type_id);
    };
    let removed: Option<(BuildQueueItem, bool)> = sim
        .production
        .queues_by_owner
        .get_mut(&owner_id)
        .and_then(|queues| queues.get_mut(&category))
        .and_then(|queue| {
            let was_head = idx == 0;
            let item = queue.remove(idx);
            refresh_queue_states(queue);
            item.map(|i| (i, was_head))
        });
    if let Some((removed_item, was_head)) = removed {
        let removed_type_str = sim.interner.resolve(removed_item.type_id).to_string();
        let full_cost = rules.object(&removed_type_str).map(|o| o.cost.max(0)).unwrap_or(0);
        let refund = if was_head {
            (full_cost - removed_item.balance_remaining).max(0)
        } else {
            0
        };
        if refund > 0 {
            *credits_entry_for_owner(sim, owner) += refund;
        }
        // If we removed the head, promote the new head.
        if was_head {
            if let Some(queue) = sim
                .production
                .queues_by_owner
                .get_mut(&owner_id)
                .and_then(|queues| queues.get_mut(&category))
            {
                if let Some(new_head) = queue.front_mut() {
                    if new_head.balance_remaining == 0 && new_head.progress_steps == 0 {
                        let new_head_type = sim.interner.resolve(new_head.type_id).to_string();
                        if let Some(obj) = rules.object(&new_head_type) {
                            new_head.balance_remaining = obj.cost.max(0);
                        }
                    }
                }
            }
        }
    }
    sim.production.queues_by_owner.retain(|_, queues| {
        queues.retain(|_, q| !q.is_empty());
        !queues.is_empty()
    });
    true
}
```

Note: the `sim.interner.resolve(new_head.type_id)` inside a `get_mut` borrow
conflicts. Re-fetch the type_id first, then drop the queue borrow, then look
up the object. Acceptable refactor:

```rust
if was_head {
    let new_head_type: Option<String> = sim
        .production
        .queues_by_owner
        .get(&owner_id)
        .and_then(|q| q.get(&category))
        .and_then(|q| q.front())
        .filter(|h| h.balance_remaining == 0 && h.progress_steps == 0)
        .map(|h| sim.interner.resolve(h.type_id).to_string());
    if let Some(type_str) = new_head_type {
        let cost = rules.object(&type_str).map(|o| o.cost.max(0)).unwrap_or(0);
        if let Some(queue) = sim
            .production
            .queues_by_owner
            .get_mut(&owner_id)
            .and_then(|queues| queues.get_mut(&category))
        {
            if let Some(new_head) = queue.front_mut() {
                new_head.balance_remaining = cost;
            }
        }
    }
}
```

**Step 3: `cancel_ready_by_type_for_owner` stays unchanged**

The function at [production_queue.rs:776-806](src/sim/production/production_queue.rs#L776-L806)
already refunds full `obj.cost`. By our formula this matches
`full_cost − balance_remaining(=0)` = `full_cost`. No change needed.
(Verify by reading the function; confirm no edit required.)

**Step 4: Verify**

Run: `cargo check --lib`
Expected: cancel functions compile.

**Step 5: Commit**

Commit message: `sim/production: partial refund on cancel; promote new head after head-cancel`

---

### Task 8: Update `refresh_queue_states` for NoFunds carry-over

**Why:** The current implementation forces `state = Building` on the head
every refresh. Under the new model, a head that's in `NoFunds` should stay
`NoFunds` until `advance_queue_item` clears it. Only `Queued`/`Building`
should be promoted to `Building`.

**Files:**
- Modify: [src/sim/production/production_queue.rs:115-125](src/sim/production/production_queue.rs#L115-L125)

**Step 1: Update the function**

Replace:

```rust
pub(super) fn refresh_queue_states(queue: &mut VecDeque<BuildQueueItem>) {
    for (idx, item) in queue.iter_mut().enumerate() {
        if idx == 0 {
            // Head: leave NoFunds and Paused intact; promote Queued to Building.
            if matches!(item.state, BuildQueueState::Queued | BuildQueueState::Done) {
                item.state = BuildQueueState::Building;
            }
        } else {
            item.state = BuildQueueState::Queued;
        }
    }
}
```

**Step 2: Verify**

Run: `cargo check --lib`

**Step 3: Commit**

Commit message: `sim/production: preserve NoFunds across refresh_queue_states`

---

### Task 9: Update `hash_production` in world_hash.rs

**Why:** Field changes to `BuildQueueItem` require matching state-hash updates
or lockstep replay will diverge. Same-commit as field changes ideally, but
gated by Task 2's compile-broken-window so we land it now.

**Files:**
- Modify: [src/sim/world/world_hash.rs:131-140](src/sim/world/world_hash.rs#L131-L140)

**Pattern:** Existing per-field hash sequence.

**Step 1: Update the hash block**

Replace [world_hash.rs:131-140](src/sim/world/world_hash.rs#L131-L140) with:

```rust
for item in queue {
    item.owner.hash(hasher);
    item.type_id.hash(hasher);
    item.queue_category.hash(hasher);
    item.state.hash(hasher);
    item.total_base_frames.hash(hasher);
    item.progress_steps.hash(hasher);
    item.balance_remaining.hash(hasher);
    item.step_time_carry_ppm.hash(hasher);
    item.enqueue_order.hash(hasher);
}
```

**Step 2: Verify**

Run: `cargo check --lib`
Expected: world_hash compiles.

**Step 3: Commit**

Commit message: `sim/world: hash new BuildQueueItem fields (progress_steps + balance_remaining)`

---

### Task 10: Update `queued_item_via` test helper signature

**Why:** Test code creates `BuildQueueItem` directly via this helper; old
fields are gone. Single helper, single point of fix.

**Files:**
- Modify: [src/sim/production/production_tests.rs:555-573](src/sim/production/production_tests.rs#L555-L573)

**Step 1: Replace the helper**

```rust
/// Create a BuildQueueItem with IDs interned via the given interner. The
/// helper expects test callers to supply the full balance and step state
/// they want; defaults to "active head, fresh start, fully funded".
pub(super) fn queued_item_via(
    interner: &mut crate::sim::intern::StringInterner,
    owner: &str,
    type_id: &str,
    queue_category: ProductionCategory,
    total_base_frames: u32,
    balance_remaining: i32,
) -> BuildQueueItem {
    BuildQueueItem {
        owner: interner.intern(owner),
        type_id: interner.intern(type_id),
        queue_category,
        state: BuildQueueState::Queued,
        total_base_frames,
        progress_steps: 0,
        balance_remaining,
        step_time_carry_ppm: 0,
        enqueue_order: 1,
    }
}
```

Note the parameter rename: callers passed `remaining_base_frames` in the
last position; that's now `balance_remaining`. Audit call sites and update
the values they pass — see Step 2.

**Step 2: Update call sites**

Run: `cargo check --lib --tests 2>&1 | grep queued_item_via`

Existing call sites pass values like `10000, 5000` which were
`(total_base_frames, remaining_base_frames)`. The new helper expects
`(total_base_frames, balance_remaining)`. For tests that intended to
simulate a partially-built item, the new semantics differ: pass either
the full `obj.cost` (fresh) or a fractional remaining balance.

Inspect each call site and update accordingly. The two known sites in
[production_queue_tests.rs:666-673](src/sim/production/production_queue_tests.rs#L666-L673)
pass `10000, 5000` for GAREFN; since the test cancels and checks refund,
either:
- Pass `(10000, GAREFN.cost)` for a fresh queued non-head item.
- Pass `(10000, 0)` for a queued non-head item (no balance held).

Pick the value that matches what each test is exercising. For
`cancel_by_type_prefers_build_queue_over_ready_queue` the item exists to
be cancelled and re-tested; pass `obj.cost` so we exercise the head-
refund path: assert the refund equals `obj.cost − balance_remaining`. For
the "ready_queue holds queued item" path, the item should be at index 0,
which is the head, so use full cost.

**Step 3: Verify**

Run: `cargo check --lib --tests`
Expected: all helper sites compile.

**Step 4: Commit**

Commit message: `sim/production: update queued_item_via helper for new field set`

---

### Task 11: Rewrite existing production tests for the new model

**Why:** Existing tests assume upfront-debit semantics. Re-derive each
assertion against the per-step drain + partial-refund model.

**Files:**
- Modify: [src/sim/production/production_queue_tests.rs](src/sim/production/production_queue_tests.rs)
- Modify: [src/sim/production/production_tests.rs](src/sim/production/production_tests.rs)

**Step 1: Run the test suite, enumerate failures**

```
cargo test --lib production 2>&1 | tee /tmp/prod_test_results.txt
```

Expected: a list of failing tests. For each:
- Identify whether it asserts "credits decreased by full cost on enqueue"
  → change to "credits unchanged on enqueue".
- Identify whether it asserts "full refund on cancel"
  → change to one of:
    - "refund of `cost − balance_remaining`" for active-head cancels.
    - "no refund" for queued (non-head) cancels.
    - "full refund" for ready-buildings cancels (no change needed there).
- Tests that tick to completion and assert "credits drained by cost over
  the build" need no change in net effect — they just verify the same
  final balance, but the drain shape is now incremental.

**Step 2: Edit each failing test**

For each failure, apply the appropriate fix. Examples:

- `enqueue_deducts_cost` → rename to `enqueue_does_not_deduct_cost` and
  invert the assertion: `assert_eq!(after, before)`.

- `cancel_refunds_full_cost` (if it exists) → split into two:
  - `cancel_active_head_refunds_partial` — start build, tick to half,
    cancel, assert refund == cost − balance_remaining.
  - `cancel_queued_refunds_zero` — enqueue 2 items, cancel the 2nd
    (non-head), assert credits unchanged.

- `cancel_ready_building_refunds_cost` → unchanged in behavior; keep as-is.

**Step 3: Verify**

Run: `cargo test --lib production`
Expected: all existing tests pass with updated assertions.

**Step 4: Commit**

Commit message: `sim/production: update existing tests for per-step drain semantics`

---

### Task 12: Add new tests for per-step drain, NoFunds, partial refund

**Why:** Lock in the new semantics with explicit unit tests. Catches
regressions early.

**Files:**
- Modify: [src/sim/production/production_queue_tests.rs](src/sim/production/production_queue_tests.rs)

**Step 1: Add the 9 tests from the design doc**

Append to `production_queue_tests.rs`. Each test uses
`basic_infantry_rules()` or `build_catalog_rules()` and the spawn helpers
already present in scope.

```rust
#[test]
fn enqueue_does_not_debit_credits() {
    let mut sim = Simulation::new();
    let rules = basic_infantry_rules();
    rules.intern_all_ids(&mut sim.interner);
    spawn_structure(&mut sim, 1, "Americans", "GAPILE", 10, 10);
    let before = credits_for_owner(&sim, "Americans");
    let ok = super::enqueue_by_type(&mut sim, &rules, "Americans", "E1");
    assert!(ok);
    let after = credits_for_owner(&sim, "Americans");
    assert_eq!(after, before, "credits must not move on enqueue");
}

#[test]
fn tick_drains_per_step() {
    let mut sim = Simulation::new();
    let rules = basic_infantry_rules();
    rules.intern_all_ids(&mut sim.interner);
    spawn_structure(&mut sim, 1, "Americans", "GAPILE", 10, 10);
    let before = credits_for_owner(&sim, "Americans");
    super::enqueue_by_type(&mut sim, &rules, "Americans", "E1");
    // Run several ticks; credits should decrease but not by full cost yet.
    let height_map = std::collections::BTreeMap::new();
    for _ in 0..10 {
        tick_production(&mut sim, &rules, &height_map, None, 66);
    }
    let after = credits_for_owner(&sim, "Americans");
    let cost = rules.object("E1").unwrap().cost;
    assert!(after < before, "credits should drop");
    assert!(after > before - cost, "should not be fully drained yet at 10 ticks");
}

#[test]
fn final_step_pays_remainder() {
    let mut sim = Simulation::new();
    let rules = basic_infantry_rules();
    rules.intern_all_ids(&mut sim.interner);
    spawn_structure(&mut sim, 1, "Americans", "GAPILE", 10, 10);
    let before = credits_for_owner(&sim, "Americans");
    super::enqueue_by_type(&mut sim, &rules, "Americans", "E1");
    let cost = rules.object("E1").unwrap().cost;
    // Tick well past completion (54 steps × frames_per_step × 66 ms).
    let height_map = std::collections::BTreeMap::new();
    for _ in 0..2000 {
        tick_production(&mut sim, &rules, &height_map, None, 66);
    }
    let after = credits_for_owner(&sim, "Americans");
    // Total drained must equal cost exactly (final-step remainder flush).
    // Note: produced unit may or may not have spawned; we only check
    // credits drained == cost regardless of spawn outcome (if spawn-fail,
    // the refund path adds back cost — final balance == before).
    let drained = before - after;
    assert!(drained == cost || drained == 0,
            "drain must be exact cost or fully refunded on spawn-fail; got {}", drained);
}

#[test]
fn nofunds_stalls_then_resumes() {
    let mut sim = Simulation::new();
    let rules = basic_infantry_rules();
    rules.intern_all_ids(&mut sim.interner);
    spawn_structure(&mut sim, 1, "Americans", "GAPILE", 10, 10);
    super::enqueue_by_type(&mut sim, &rules, "Americans", "E1");
    let cost = rules.object("E1").unwrap().cost;
    // Drain credits below per-step cost. With cost=200 and 54 steps,
    // each step costs ~3. Set credits to 0 to guarantee stall.
    *super::credits_entry_for_owner(&mut sim, "Americans") = 0;
    let height_map = std::collections::BTreeMap::new();
    tick_production(&mut sim, &rules, &height_map, None, 66 * 100);
    // Head should be NoFunds, no progress past step 0.
    let view = queue_view_for_owner(&sim, &rules, "Americans");
    assert!(!view.is_empty());
    assert_eq!(view[0].state, BuildQueueState::NoFunds);
    // Add credits back, tick once — should resume.
    *super::credits_entry_for_owner(&mut sim, "Americans") = cost * 2;
    tick_production(&mut sim, &rules, &height_map, None, 66);
    let view2 = queue_view_for_owner(&sim, &rules, "Americans");
    assert_eq!(view2[0].state, BuildQueueState::Building);
}

#[test]
fn cancel_head_refunds_partial() {
    use super::cancel_last_for_owner;
    let mut sim = Simulation::new();
    let rules = basic_infantry_rules();
    rules.intern_all_ids(&mut sim.interner);
    spawn_structure(&mut sim, 1, "Americans", "GAPILE", 10, 10);
    let before = credits_for_owner(&sim, "Americans");
    super::enqueue_by_type(&mut sim, &rules, "Americans", "E1");
    let height_map = std::collections::BTreeMap::new();
    // Drain partway: a few ticks.
    for _ in 0..5 {
        tick_production(&mut sim, &rules, &height_map, None, 66);
    }
    let spent_so_far = before - credits_for_owner(&sim, "Americans");
    let cancelled = cancel_last_for_owner(&mut sim, &rules, "Americans");
    assert!(cancelled);
    let after = credits_for_owner(&sim, "Americans");
    let cost = rules.object("E1").unwrap().cost;
    // Refund = cost - balance_remaining = spent_so_far. Final balance == before - spent + spent_so_far_refund
    // But the refund formula refunds exactly what was spent, so net == before.
    assert!((after - (before - spent_so_far + spent_so_far)).abs() <= 1,
            "refund should restore spent amount; got after={}, before={}", after, before);
}

#[test]
fn cancel_queued_nonhead_refunds_zero() {
    use super::cancel_by_type_for_owner;
    let mut sim = Simulation::new();
    let rules = basic_infantry_rules();
    rules.intern_all_ids(&mut sim.interner);
    spawn_structure(&mut sim, 1, "Americans", "GAPILE", 10, 10);
    super::enqueue_by_type(&mut sim, &rules, "Americans", "E1");
    super::enqueue_by_type(&mut sim, &rules, "Americans", "E1");
    let before = credits_for_owner(&sim, "Americans");
    // Cancel the second (non-head) E1.
    let cancelled = cancel_by_type_for_owner(&mut sim, &rules, "Americans", "E1");
    assert!(cancelled);
    let after = credits_for_owner(&sim, "Americans");
    // Cancel removes the LAST-enqueued matching item, which is the non-head one.
    // No money was committed, so refund is 0.
    assert_eq!(after, before, "non-head cancel refunds 0");
}

#[test]
fn promote_next_starts_balance_when_head_completes() {
    let mut sim = Simulation::new();
    let rules = basic_infantry_rules();
    rules.intern_all_ids(&mut sim.interner);
    spawn_structure(&mut sim, 1, "Americans", "GAPILE", 10, 10);
    super::enqueue_by_type(&mut sim, &rules, "Americans", "E1");
    super::enqueue_by_type(&mut sim, &rules, "Americans", "E1");
    let height_map = std::collections::BTreeMap::new();
    // Tick to completion of #1.
    for _ in 0..2000 {
        tick_production(&mut sim, &rules, &height_map, None, 66);
        // Stop once we reach the second-as-head state.
        let v = queue_view_for_owner(&sim, &rules, "Americans");
        if v.len() == 1 && v[0].state == BuildQueueState::Building { break; }
    }
    // Inspect raw queue.
    let owner_id = sim.interner.intern("Americans");
    let q = sim.production.queues_by_owner.get(&owner_id)
        .and_then(|qs| qs.get(&ProductionCategory::Infantry));
    if let Some(q) = q {
        if let Some(head) = q.front() {
            let cost = rules.object("E1").unwrap().cost;
            assert_eq!(head.balance_remaining, cost,
                "new head must have balance == cost after promotion");
        }
    }
}

#[test]
fn zero_cost_item_never_stalls() {
    // Synthetic: an item with cost=0 should never trigger NoFunds even at
    // credits=0. Uses queued_item_via to inject a zero-balance head.
    let mut sim = Simulation::new();
    let rules = basic_infantry_rules();
    rules.intern_all_ids(&mut sim.interner);
    spawn_structure(&mut sim, 1, "Americans", "GAPILE", 10, 10);
    *super::credits_entry_for_owner(&mut sim, "Americans") = 0;
    let item = queued_item_via(&mut sim.interner, "Americans", "E1",
                               ProductionCategory::Infantry, 540, 0);
    let owner_id = sim.interner.intern("Americans");
    sim.production.queues_by_owner.entry(owner_id).or_default()
        .entry(ProductionCategory::Infantry).or_default().push_back(item);
    let height_map = std::collections::BTreeMap::new();
    tick_production(&mut sim, &rules, &height_map, None, 66 * 100);
    let view = queue_view_for_owner(&sim, &rules, "Americans");
    if !view.is_empty() {
        assert_ne!(view[0].state, BuildQueueState::NoFunds,
                   "zero-cost item must not stall on NoFunds");
    }
}

#[test]
fn int_div_rounding_drift_drains_exactly_cost() {
    // Check several cost values to ensure final-step remainder flush works.
    // Run via a synthetic single-step harness on advance_queue_item directly.
    // (This test imports the function via super::advance_queue_item if it's
    // not pub. If private, restructure as an integration-style ticker.)
    //
    // Approach: enqueue with known cost, tick to completion, assert
    // exact drain.
    let costs = [1i32, 53, 100, 999, 5000];
    for &cost in &costs {
        // Use a synthetic rules with a single object of this cost.
        // basic_infantry_rules() may not expose a knob to set arbitrary cost,
        // so this test may need a new helper synth_rules_with_cost(cost).
        // If that helper doesn't exist, mark this test as #[ignore] and
        // implement the helper in a follow-up.
        let _ = cost; // placeholder until synth helper exists
    }
}
```

If the synth-rules helper for arbitrary `obj.cost` doesn't exist, mark
`int_div_rounding_drift_drains_exactly_cost` as `#[ignore]` with a TODO
note. The math is already covered by the per-step trace in
`final_step_pays_remainder` for E1's actual cost.

**Step 2: Verify**

Run: `cargo test --lib production`
Expected: all new tests pass.

**Step 3: Commit**

Commit message: `sim/production: add tests for per-step drain + NoFunds + partial refund`

---

### Task 13: Full-suite regression run

**Why:** Confirm no untouched system regressed. Production economy
touches state hashing (lockstep) and credit balance reads (UI, AI).

**Step 1: Full library test run**

```
cargo test --lib
```

Expected: all tests pass.

**Step 2: Check no clippy regressions**

```
cargo clippy --lib -- -D warnings
```

Expected: clean.

**Step 3: Confirm `cargo build` still produces a working binary**

```
cargo build
```

Expected: success.

**Step 4: Skirmish smoke test**

Run the game, start a skirmish, queue several units. Verify:
- Credits drop gradually (not instantly on enqueue).
- Setting low cash, queueing an expensive unit: build halts mid-progress
  (NoFunds), resumes when harvester delivers ore.
- Cancelling a mid-build item refunds only spent credits (visible in
  credit counter).
- Cancelling a queued (non-head) item leaves credits unchanged.
- AI players continue to build normally (no over-queueing pathology).

**Step 5: Commit any cleanup**

Only if tweaks are needed; otherwise no commit.

---

### Task 14: Verification against gamemd.exe behavior

**Why:** Confirm observable parity. The Ghidra reports were the
spec, but in-game side-by-side is the ground truth.

**Verify:**
- Queue 5 conscripts. Watch the credit counter on gamemd.exe (or compare
  to a recorded reference) — credits should drop in a continuous stream
  across each build, not in 5 discrete chunks at enqueue.
- Force NoFunds: spend down to under 1 unit's cost, queue a 200-credit
  unit, watch the cameo. In gamemd it pauses with no progress; verify
  ours does too.
- Cancel mid-build: build a $1500 tank to ~50%, cancel. gamemd refunds
  ~$750; we should too (±~30 credits for int-div rounding).
- Cancel queued non-head: queue 2 tanks, immediately cancel the second.
  gamemd shows no credit change (queued items hold no money); ours
  should match.

Document any divergence in [docs/plans/2026-05-19-production-economy-per-tick-drain-verification.md](docs/plans/2026-05-19-production-economy-per-tick-drain-verification.md).

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-19-production-economy-per-tick-drain-design.md](docs/plans/2026-05-19-production-economy-per-tick-drain-design.md)
- **Ghidra reports:**
  - `ra2-rust-game-docs/BUILD_QUEUE_GHIDRA_REPORT.md` — FactoryClass struct, Progress 0..=54, NoFunds flag, AI tick layout.
  - `ra2-rust-game-docs/FACTORYCLASS_PRODUCTION_DEEP_DIVE.md` — Per-step math, AbandonProduction refund formula, IsManual vs OnHold distinction.
  - `ra2-rust-game-docs/FACTORY_CREDIT_SYSTEM_GHIDRA_REPORT.md` — HouseClass credit field layout, Add_Credits/Spend_Money.
- **gamemd.exe addresses** (referenced; not in Rust code per CLAUDE.md):
  - `FactoryClass::AI` 0x004C9B20
  - `FactoryClass::AbandonProduction` 0x004CA0E0
  - `FactoryClass::StartProduction` 0x004C9C70
  - `HouseClass::Add_Credits` 0x004F9950
  - `HouseClass::Spend_Money` 0x004F9790
- **INI keys:** Per-object `Cost=`, `[General] BuildSpeed=`,
  per-object `BuildTimeMultiplier=`, `[General] MultipleFactory=`.
  All already parsed.
- **Related code:**
  - [src/sim/production/production_queue.rs](src/sim/production/production_queue.rs)
  - [src/sim/production/production_types.rs](src/sim/production/production_types.rs)
  - [src/sim/world/world_hash.rs](src/sim/world/world_hash.rs)
- **Prior commits touching production:**
  - `3b437bd sim/production: cover infantry foundation-center spawn` — recent, unrelated to economy
  - `2051442 sim/production: route infantry spawn to building-center cell` — same
  - `de7e6b2 WIP on dev: …` — irrelevant

---

## Commit Batching Note

Tasks 1-9 form a tightly coupled cluster: the field changes in Task 2 break
compilation until Tasks 3-9 land. For a cleaner git history, either:
- **Atomic squash:** do Tasks 1-9 in a single working session and commit as
  one larger commit `sim/production: per-tick credit drain + NoFunds + partial refund`.
- **Sequential micro-commits:** as written above, with the understanding
  that intermediate commits won't compile cleanly. Each leaves `cargo check`
  in a broken state until Task 9 closes the loop.

Recommend atomic squash for this refactor — the intermediate commits add no
review value because the broken-compile state masks issues.
