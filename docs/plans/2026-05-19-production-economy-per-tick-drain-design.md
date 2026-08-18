# Production Economy Refactor — Per-Tick Drain, NoFunds Stall, Partial Refund — Design

## Goal

Bring the production credit-flow into observable parity with gamemd.exe: per-step
cost drain across 54 discrete progress steps, NoFunds stall when funds run out
mid-build, and partial refund on cancel equal to the amount actually spent.

## Architecture Context

### Current Rust implementation

[src/sim/production/production_queue.rs](src/sim/production/production_queue.rs)
owns the per-owner queue loop driven by `tick_production`. Today's flow:

- `enqueue_by_type` deducts `obj.cost` from the owner's credits at enqueue time
  ([production_queue.rs:215](src/sim/production/production_queue.rs#L215)).
- `advance_queue_item` advances `remaining_base_frames` against a power-/multi-factory-
  scaled `progress_rate_ppm`. No money interaction after enqueue.
- All cancel paths (`cancel_last_for_owner`, `cancel_by_type_for_owner`,
  `cancel_ready_by_type_for_owner`) refund the full `obj.cost`.
- `BuildQueueState` is `Queued | Building | Paused | Done`; there is no NoFunds variant.
- Spawn-fail (no helipad / no spawn cell at completion) full-refunds.

### gamemd.exe model (verified via Ghidra reports)

Sourced from `ra2-rust-game-docs/BUILD_QUEUE_GHIDRA_REPORT.md`,
`FACTORYCLASS_PRODUCTION_DEEP_DIVE.md`, `FACTORY_CREDIT_SYSTEM_GHIDRA_REPORT.md`.

- `FactoryClass` has `Progress: 0..=54`, `Balance` (remaining cost), `Rate` (frames
  per step), `Object` (active TechnoClass), `QueueArray` of `TechnoTypeClass*`.
- Active item is the `Object`; trailing items in `QueueArray` are type pointers
  with **no money committed**. Money is only attached when an item is promoted
  to active head (`StartProduction` sets `Balance = type->GetCost(Owner)`).
- Per-step tick (`FactoryClass::AI`, 0x004C9B20): tentatively advance Progress,
  compute `costThisStep = Balance / (54 - Progress)` (int div, last step pays
  `Balance`), check `GetAvailableCredits() >= costThisStep`. If insufficient
  → set `OnHold=true`, roll back `Progress -= 1`, no spend. Else → Spend_Money,
  decrement Balance.
- Completion (Progress == 54): `Spend_Money(Balance); Balance = 0;` pays
  integer-division remainder.
- Cancel via `AbandonProduction` (0x004CA0E0):
  `Add_Credits(fullCost - Balance)` — refund only the amount already spent.

### Constraints from CLAUDE.md

Per the parity bar, the spec is observable output, not gamemd's internals. We
reproduce the per-step drain math (rounding-equivalent on inputs the player can
see in the credit display) but do not need to mirror the `FactoryClass` /
`Object` / `QueueArray` split internally. Implicit-active-head over a
single `VecDeque<BuildQueueItem>` produces the same outputs.

## Impact Analysis

**Touched files:**

- [src/sim/production/production_types.rs](src/sim/production/production_types.rs):
  `BuildQueueItem` field set changes; `BuildQueueState` gains `NoFunds`.
- [src/sim/production/production_queue.rs](src/sim/production/production_queue.rs):
  `enqueue_by_type`, `tick_production`, `advance_queue_item`, all three cancel
  paths, `queue_view_for_owner`, `refresh_queue_states`.
- [src/sim/production/production_tests.rs](src/sim/production/production_tests.rs)
  and [src/sim/production/production_queue_tests.rs](src/sim/production/production_queue_tests.rs):
  many assertions about credit balance and refund amounts need to be rewritten.

**Out of scope (unchanged):**

- `production_economy.rs` — harvester ticking / ore deposits, unrelated.
- `production_placement.rs`, `production_sell.rs`, `production_spawn.rs`,
  `production_tech.rs` — placement, sell, spawn-cell-finding, tech-tree.
- `effective_progress_rate_ppm_for_type` and friends — the power/multi-factory
  rate scaling continues to work; we just change what we accumulate against it.
- Sidebar overlay rendering for the "On Hold" cameo — separate follow-up.
- Spawn-fail refund path — left as-is per user direction; flagged as a known
  open question (see Alternatives Considered).
- Storage pool (HouseClass+0x314 in gamemd) — cash-pool-only per user direction;
  vestigial in standard YR.

**Risk areas:**

- **Save-load schema**: `BuildQueueItem` is `Serialize/Deserialize`. Field
  rename/add breaks old saves. Pre-1.0 — acceptable.
- **AI build picker**: any code that calls `credits_for_owner` to decide whether
  to queue more items sees a slower drain (over the build duration instead of
  instant). gamemd AI handles the same shape, so this is parity-correct. In-game
  observation will confirm AI doesn't pathologically over-queue.
- **Determinism**: all new math is integer (`i32` for `balance_remaining`,
  `u8` for `progress_steps`, `u64` for step-time carry). Same lockstep contract
  as today.
- **Tick ordering**: `tick_production` already runs in the existing slot. No
  reordering needed.

## Tiny-Detail Ledger (constraints carried into implementation)

Each item cites its source from the Ghidra reports.

1. **Progress is 54 discrete steps.** `progress_steps: u8`, range `0..=54`.
   [doc: BUILD_QUEUE §6, FACTORYCLASS_PRODUCTION_DEEP_DIVE §2]
2. **Frames-per-step rate = `GetBuildStepTime() / 54`, clamped `[1, 255]`.**
   We re-use `effective_progress_rate_ppm_for_type` for the live scale and
   derive `frames_per_step` from `total_base_frames / 54`, clamped `[1, 255]`.
   [doc: FACTORYCLASS_PRODUCTION_DEEP_DIVE §9]
3. **`costThisStep = balance / stepsLeft` (integer div).** Where
   `stepsLeft = 54 - progress_steps`. Special-case `stepsLeft == 0 → balance`.
   Then `min(costThisStep, balance)`. [doc: BUILD_QUEUE §6,
   FACTORYCLASS_PRODUCTION_DEEP_DIVE §2]
4. **NoFunds rollback after tentative advance.** Order: progress += 1 →
   compute cost → check funds → on insufficient, progress -= 1, set
   `BuildQueueState::NoFunds`, no spend. Retry next step interval.
   [doc: BUILD_QUEUE §6]
5. **Balance is set to full cost at active-head start.** Set in
   `start_active_head()` when an item becomes the head (either enqueued into
   empty queue or promoted after the previous head completes/cancels).
   [doc: BUILD_QUEUE §5 path A, FACTORYCLASS_PRODUCTION_DEEP_DIVE §10]
6. **Final-step remainder payment.** On `progress_steps == 54`: spend the
   remaining `balance_remaining` (flushes int-div remainder, up to ~53
   credits), set balance to 0. [doc: FACTORYCLASS_PRODUCTION_DEEP_DIVE §2]
7. **Cancel refund formula.**
   - Active head: refund `obj.cost - balance_remaining`.
   - Queued (not head): refund 0.
   - Ready buildings (Progress==54 awaiting placement): refund `obj.cost`
     (balance_remaining is 0 by completion, so formula yields full cost).
   [doc: FACTORY_CREDIT_SYSTEM §Add_Credits, FACTORYCLASS_PRODUCTION_DEEP_DIVE §11]
8. **Queued items hold no money.** No deduction at `enqueue_by_type`. Money
   is only debited as the active head drains step-by-step.
   [doc: BUILD_QUEUE §3 path B, FACTORYCLASS_PRODUCTION_DEEP_DIVE §10]
9. **NoFunds distinct from Paused.** Manual pause (`Paused`) only resumes by
   player input. NoFunds auto-resumes the moment funds are available. New
   `BuildQueueState::NoFunds` variant. [doc: FACTORYCLASS_PRODUCTION_DEEP_DIVE §8]
10. **Cash pool only.** Credits stays as `i32` on `HouseState`. No storage
    pool struct. [doc: FACTORY_CREDIT_SYSTEM §GetAvailableCredits + CLAUDE.md
    internal-modernization rule]
11. **Sidebar credit counter geometric-decay animation** — out of scope here;
    sidebar render layer.
12. **Refund credits go to cash pool.** `credits_entry_for_owner(...) += refund`.
    [doc: FACTORY_CREDIT_SYSTEM §Add_Credits]
13. **Spawn-fail behavior — UNKNOWN — needs RE.** Current full-refund-on-spawn-
    fail kept as-is per user direction. Open follow-up: investigate whether
    gamemd holds, deletes, or refunds the unit when no exit cell is available.

## Chosen Approach (Approach A)

**Per-item Balance/Progress, head-of-queue is implicitly active.**

Single `VecDeque<BuildQueueItem>` per (owner, category) — unchanged from today.
The front item is the active producer; trailing items are reservations with
`balance_remaining = 0`. A small helper `start_active_head` runs whenever the
head slot transitions to a new item (enqueue into empty queue, head completes,
head cancels) and writes `balance_remaining = obj.cost`.

Rejected: Approach B (split `active: Option<ActiveProduction>` from
`queued: VecDeque<InternedId>`) reproduces gamemd's `Object`+`QueueArray` split
literally but adds churn across every consumer (sidebar, AI picker, tests) with
no observable output difference. See Alternatives Considered.

## Design

### Components

No new modules. All changes live inside `src/sim/production/`:

- `production_types.rs` — field set on `BuildQueueItem`, new `BuildQueueState`
  variant.
- `production_queue.rs` — rewritten `advance_queue_item`, modified
  `tick_production`, modified `enqueue_by_type`, modified cancel functions,
  new private `start_active_head` helper, modified `refresh_queue_states` to
  handle NoFunds carry-over.

### Data Structures

`BuildQueueItem` (after refactor):

```rust
pub struct BuildQueueItem {
    pub owner: InternedId,
    pub type_id: InternedId,
    pub queue_category: ProductionCategory,
    pub state: BuildQueueState,
    /// Base build time in RA2 production frames before live scaling.
    /// Used for UI ms estimation and to derive frames_per_step.
    pub total_base_frames: u32,
    /// Current progress, 0..=54. Item completes at 54.
    pub progress_steps: u8,
    /// Remaining cost owed by the active head. Zero on queued (non-head)
    /// items. Set to obj.cost when promoted to head; drained per step.
    pub balance_remaining: i32,
    /// Sub-step time accumulator in PPM units (matches existing rate scale).
    /// When carry crosses a frames_per_step boundary, attempt one step.
    pub step_time_carry_ppm: u64,
    pub enqueue_order: u64,
}
```

Removed fields: `remaining_base_frames`, `progress_carry` (replaced by
`progress_steps` and `step_time_carry_ppm`). Kept: `total_base_frames` (drives
both UI ms and `frames_per_step`).

`BuildQueueState`:

```rust
pub enum BuildQueueState {
    Queued,
    Building,
    NoFunds,   // <-- new
    Paused,
    Done,
}
```

`NoFunds` slots in between `Building` and `Paused` semantically. `refresh_queue_states`
treats `NoFunds` as a head-state alongside `Building`: only the front item may be
in `Building`, `NoFunds`, or `Paused`; trailing items stay `Queued`.

### Interfaces / Contracts

Public surface stays the same:

- `enqueue_by_type` — still returns `bool`; semantics change: returns `true`
  on successful enqueue but money is NOT debited.
- `cancel_last_for_owner` / `cancel_by_type_for_owner` — still return `bool`;
  refund amount changes per the formula in Ledger §7.
- `tick_production` — signature unchanged; internal step logic rewritten.
- `queue_view_for_owner` — `QueueItemView.remaining_ms` and `total_ms` are
  still derived from `total_base_frames` and the live rate; the surfaced UI
  state now includes `NoFunds`.
- `credits_for_owner` — unchanged.

### Data Flow

**Enqueue path** (`enqueue_by_type`):
1. Resolve `obj`, validate build option (existing logic).
2. Affordability gate: `credits_for_owner(...) >= obj.cost`. This is a UI
   gate, not a debit — preserves "can't queue what you can't afford to start"
   user expectation. (gamemd has this check inside `CanBuild`, same effect.)
3. Push `BuildQueueItem` with `balance_remaining = 0, progress_steps = 0,
   step_time_carry_ppm = 0`.
4. Call `refresh_queue_states(queue)`.
5. If the new item is at the head (queue was empty before), call
   `start_active_head(queue, rules)` to set `balance_remaining = obj.cost`.

**Per-tick path** (`tick_production` → `advance_queue_item`):

```text
for each (owner, category) with non-empty queue:
    head = queue.front_mut()
    if head.state == Paused: continue
    rate_ppm = effective_progress_rate_ppm_for_type(...)
    frames_per_step = (head.total_base_frames / 54).clamp(1, 255)
    step_threshold = frames_per_step * RA2_QUEUE_FRAME_MS * PRODUCTION_RATE_SCALE
    head.step_time_carry_ppm += (tick_ms as u64) * rate_ppm

    // Attempt as many steps as the carry covers, one at a time
    loop:
        if head.step_time_carry_ppm < step_threshold: break
        // Try one step
        head.progress_steps += 1
        steps_left = 54 - head.progress_steps
        cost_this_step = if steps_left == 0 {
            head.balance_remaining
        } else {
            head.balance_remaining / (steps_left + 1) // pre-decrement view
        };
        // Note: matches gamemd: "stepsLeft = 54 - Progress" AFTER the
        // tentative increment, where new Progress is what we just set.
        // gamemd computes stepsLeft after Progress+=1, so stepsLeft can
        // be 0 (the final step). We mirror this.
        cost_this_step = cost_this_step.min(head.balance_remaining).max(0)

        if credits < cost_this_step:
            head.progress_steps -= 1  // rollback
            head.state = NoFunds
            // Drop the carry that covered this step so we retry next interval
            head.step_time_carry_ppm -= step_threshold
            break
        else:
            credits -= cost_this_step
            head.balance_remaining -= cost_this_step
            head.step_time_carry_ppm -= step_threshold
            if head.state == NoFunds: head.state = Building
            if head.progress_steps == 54:
                // Completion: balance_remaining already zero or carries
                // the int-div remainder, which we just paid via the
                // stepsLeft==0 branch. Done.
                head.state = Done
                break
```

After loop, normal completion handling proceeds as today (pop, dispatch to
ready_by_owner / spawn / refund-on-fail path).

**Stepping math verification (cost rounding):**

For `obj.cost = 1000`, the per-step cost trace is:
- step 1: `balance=1000, stepsLeft=53, cost=18` → balance=982
- step 2: `balance=982,  stepsLeft=52, cost=18` → balance=964
- ...
- step 54: `stepsLeft=0` → `cost=balance` (pays all rounding drift)

For `obj.cost = 100`:
- step 1: `balance=100, stepsLeft=53, cost=1` → balance=99
- steps 2..53: most are cost=1, balance counts down
- step 54: `cost=balance` flushes whatever remains

Matches gamemd's `Balance / stepsLeft` semantics.

**Cancel path** (`cancel_last_for_owner`, `cancel_by_type_for_owner`):
1. Locate target item.
2. If it's the head (`queue.front()` after lookup): refund
   `obj.cost - item.balance_remaining`. Pop it. Then call
   `start_active_head` on the new front if any.
3. If it's a queued non-head item: refund 0. Remove it.
4. Call `refresh_queue_states`.

**Cancel of ready building** (`cancel_ready_by_type_for_owner`):
- The completed building already drained all `obj.cost` (balance was 0 at
  completion). Refund `obj.cost` (matches the formula
  `fullCost - balance_remaining = fullCost - 0`). Behavior unchanged from today.

**Promote-next path** (`start_active_head` on existing items):
After head completes (in `tick_production`'s pop) or is cancelled, the new
front item — which currently has `balance_remaining = 0` — needs promotion:

```rust
fn start_active_head(queue: &mut VecDeque<BuildQueueItem>, rules: &RuleSet) {
    let Some(head) = queue.front_mut() else { return; };
    if head.balance_remaining != 0 || head.progress_steps != 0 {
        return; // Already started (defensive)
    }
    let type_str = /* resolve via interner — caller passes it */;
    let Some(obj) = rules.object(type_str) else { return; };
    head.balance_remaining = obj.cost.max(0);
    // progress_steps and step_time_carry_ppm start at 0
}
```

(In practice `start_active_head` will need an interner reference; the helper
signature lives inside `production_queue.rs` with access to whatever the
caller already holds.)

### Error Handling

- `obj.cost <= 0` items (free units, e.g., grants): `balance_remaining = 0`
  at promotion. Every step computes `cost_this_step = 0`. Funds check passes
  trivially. Behavior unchanged.
- `total_base_frames < 54`: `frames_per_step` clamps to 1; build completes
  in ≤54 sub-tick intervals. Same observable outcome as today.
- `tick_ms == 0`: early-return as today.
- Owner with no `HouseState` entry: `credits_entry_for_owner` already
  auto-creates with defaults. No new error path.

### Testing Strategy

Unit tests added in `production_queue_tests.rs`:

1. `enqueue_does_not_debit_credits` — queue item, assert
   `credits_for_owner` unchanged.
2. `tick_drains_per_step` — queue a known-cost item, run ticks, assert
   credits decrease incrementally across steps.
3. `final_step_pays_remainder` — pick a cost not divisible by 54 (e.g.,
   1000), assert total drained == cost at completion (no off-by-one).
4. `nofunds_stalls_then_resumes` — queue an item, drain credits to mid-build
   threshold via direct mutation, run tick, assert state==NoFunds and
   progress doesn't advance. Add credits back, run tick, assert state
   returns to Building and progress resumes.
5. `cancel_head_refunds_partial` — queue, tick N times to spend X credits,
   cancel, assert refund == cost - X.
6. `cancel_queued_nonhead_refunds_zero` — enqueue two items, cancel the
   second, assert credits unchanged.
7. `promote_next_starts_balance` — queue two items, complete head, assert
   new head has `balance_remaining == obj.cost`.
8. `nofunds_zero_cost_item` — enqueue a `cost=0` item, drain credits to 0,
   assert build still progresses (no NoFunds triggered).
9. `int_div_rounding_drift_within_bounds` — total drained across full build
   equals `obj.cost` exactly, regardless of cost value (1, 53, 100, 999,
   50000).

Integration tests in `production_tests.rs`:

- Existing happy-path "queue + tick to completion + place" tests: update
  credit-balance assertions but expect the same final outcome (unit spawns).
- New: "queue two infantry, cancel first mid-build" — verify the second
  promotes correctly.

### Determinism Considerations

All math is integer:
- `progress_steps: u8` — exact step count.
- `balance_remaining: i32` — exact remaining cost.
- `step_time_carry_ppm: u64` — fixed-point accumulator at the existing
  `PRODUCTION_RATE_SCALE` (1e6).
- Step threshold computed as `u128` to avoid overflow during multiplication,
  then converted back.

No floats, no platform-specific intrinsics. Lockstep contract preserved.

## Architectural Decisions

**Patterns followed:**
- Single `VecDeque<BuildQueueItem>` per (owner, category) — existing pattern.
- Implicit-active-head (queue front is the producer) — existing pattern.
- `BuildQueueState` as a closed enum — existing pattern, extended by one
  variant.
- All math integer; `i32` for credits, `u64` for time carries — existing
  determinism rule.

**Patterns deviated from / introduced:**
- None new. The refactor moves *toward* simpler invariants (queued items
  have predictable zero-balance) and away from the upfront-debit pattern
  which was a parity divergence.

**Tech debt:**
- Spawn-fail refund path remains "full refund" pending RE investigation.
  Tracked as a known open question, not introduced by this refactor.

## Alternatives Considered

**Approach B — Split `active: Option<ActiveProduction>` + `queued: VecDeque<InternedId>`.**
Mirrors gamemd's `Object` + `QueueArray` split 1:1. Identical observable
output to A. Rejected: more invasive — every consumer that touches
`BuildQueueItem` (sidebar, AI picker, snapshots, tests) has to traverse a
new two-level structure. CLAUDE.md is explicit that internals should be
modernized when they don't change output; the head-of-VecDeque pattern is
the natural Rust shape here.

**Approach C — Continuous proportional drain (no discrete 54 steps).**
Track a `balance_remaining` that drains proportionally each tick by
`tick_ms / total_time_ms`. Cleaner internally. Rejected by user direction:
deviates from the per-step rounding signature gamemd produces, and the
NoFunds stall model is intrinsically per-step (gamemd doesn't drain a
fractional credit during a stalled step — it either fully spends or fully
holds).

**Spawn-fail behavior: investigate now vs defer.** Deferred per user
direction. Logged in Ledger §13 as an open question. Current
full-refund behavior preserved as-is; no regression.
