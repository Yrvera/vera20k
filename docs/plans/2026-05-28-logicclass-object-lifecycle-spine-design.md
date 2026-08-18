# LogicClass Scheduler + ObjectClass Lifecycle Spine — Design

## Goal

Reproduce gamemd's active-object spine in Rust — a live, insertion-ordered,
membership-gated object vector with reveal/conceal/unlimbo/uninit lifecycle and
direct save/load of the order — using Rust-native ownership, keeping the existing
phased `advance_tick` but making the vector the single authority on iteration
order.

## Scope (decided with user, 2026-05-28)

- **In:** the object-vector spine — membership byte, tail-append/compacting-remove
  ops, reveal/conceal/unlimbo/uninit lifecycle hooks, save/load of the order,
  membership derived from vector presence on load, and the vector becoming the
  iteration-order authority for AI/parity-sensitive phases.
- **Scheduler shape:** *order-authority over the phased tick* (chosen). Keep
  Rust's phased `advance_tick`; phases iterate vector order instead of
  `keys_sorted()`. Do **not** rewrite to a single per-object AI walk.
- **Deferred (named DRIFT, not silently cut):**
  - **Inter-phase interleaving DRIFT (the big one).** gamemd runs a single
    interleaved walk: object A's *entire* AI (move → mission → fire) commits before
    object B's begins, so A sees the this-tick mutations of every earlier-in-vector
    object and the *last-tick* state of every later one. Rust's phased
    `advance_tick` does all-move-then-all-fire, so a unit firing in the combat phase
    sees the post-move position of *every* other unit, including ones that come
    after it in the vector. This changes targeting leads, facing, and
    who-hits-whom-first. It is **DRIFT — unproven equivalence**, per the
    burden-of-proof rule; it is **pre-existing** (the phased tick already exists)
    and this design does not make it worse. Closing it requires the true
    single-object AI walk (Option B in Alternatives), a large rewrite with heavy
    borrow-checker cost — out of scope here. Tracked follow-up: measure the
    observable divergence (e.g. two tanks crossing while both firing) and decide
    whether to close it.
  - Global subsystem reorder (claim 12: tiberium→bombs→teams→lasers→lightning→
    EMP→objects→tactical→factories→houses). Blocked partly on unresolved RE
    (unnamed `FUN_*` callees) and is a separable sequencing problem.
  - Late pre-increment frame-counter contract (claim 11). Orthogonal timing
    concern; separate pass.
  - PendingDeleteList deferred *free* (claim 7) — see ledger item 16; Rust's
    immediate unregister already reproduces the AI-stop, inline free is
    output-equivalent unless a same-tick read-after-uninit consumer is found.

## Verification preflight (this session, live Ghidra)

All load-bearing binary claims re-verified directly, not trusted from docs:

- **Adder `0x0055BAA0`** — `+0x98` early-return guard; `DynamicVector__Insert`
  (tail); set `+0x98=1` only on insert success. (`decompile_function 0x0055BAA0`)
- **Remover `0x0055BAE0`** — gated on `+0x98`; finds index via vtable+0x10;
  order-preserving left-shift compaction `items[i]=items[i+1]`; decrement count
  (`+0x10`); clear `+0x98`. Not swap-remove. (`decompile_function 0x0055BAE0`)
- **Reveal `0x005F4EC0`** — clears InLimbo `+0x81`; gate `piVar5[0x8d]`
  (= type+0x234) plus `g_GameMode`; calls `FUN_0055baa0(obj,0)`; failed path sets
  `+0x81=1` and returns 0. (`decompile_function 0x005F4EC0`)
- **Save `0x00551B20` / Load `0x00551B90`** — Save writes count + each element
  pointer in order; Load reads count, tail-appends in saved order, then swizzles
  each slot via `FUN_006cf240(&DAT_00b0c110,…)`.
  (`decompile_function 0x00551B20`, `0x00551B90`)
- **`+0x98` not serialized** — `ObjectClass::Save 0x005F6250` serializes
  `+0x74/+0x80/+0x83/+0x81/+0x84/+0x8c/+0x8d/+0x8f/+0x90` and coords, **not
  `+0x98`**; `search_byte_patterns 88 86 98 00 00 00` (`MOV [ESI+0x98],AL`)
  matches only `0x0055bac6` inside the adder. (`decompile_function 0x005F6250`)

## Architecture Context

Current Rust spine (`src/sim/world/`):

- `Simulation.live_object_order: Vec<u64>` (mod.rs:289, `#[serde(default)]`) is the
  active-order surrogate.
- `register_live_object` (mod.rs:612) — dedup-guarded tail-append (`contains` then
  `push`). **Matches native add shape.**
- `unregister_live_object` (mod.rs:618) — `retain(|id| id != x)`, i.e.
  order-preserving compacting remove. **Matches native remove shape.**
- `live_object_order_snapshot` (mod.rs:622) — returns the order, then appends a
  `keys_sorted()` fallback for any unregistered entity. **DRIFT 1.**
- Spawn paths register: `spawn_from_map_with_resolved` (world_spawn.rs:260),
  `spawn_object_at_height` (world_spawn.rs:438) — correct (active spawns);
  `spawn_object_limbo_at_height` (world_spawn.rs:588) — **DRIFT 2** (limbo objects
  should not be active). Sole caller: `paradrop.rs:182`.
- `despawn_entity` (mod.rs:~696) removes from store then unregisters — correct
  shape, but not routed through a named lifecycle helper.
- `advance_tick` (mod.rs:1187) is phased; ~55 `keys_sorted()` call sites across 30
  files. Only AI/parity-committing phases need vector order; owner-queries and
  render helpers stay as-is.
- Sole consumer of the order today: `passenger.rs:355` (garrison reconciliation).

State mapping (clean Rust encoding of the two native bytes):

| Native | Rust |
|---|---|
| InLimbo `+0x81=1`, not in vector | in `EntityStore`, `in_logic_vector=false`, absent from order |
| revealed, `+0x98=1`, in vector | in `EntityStore`, `in_logic_vector=true`, present in order |
| uninit'd / freed | removed from `EntityStore` + unregistered |

So Rust does not need a separate InLimbo flag for the spine: *store-presence ∧
¬membership* encodes limbo; *store-presence ∧ membership* encodes active.

## Impact Analysis

**Touched:**

- `src/sim/world/mod.rs` — wrap order ops; drop sorted fallback; add
  `rebuild_logic_membership`; route despawn through `uninit`.
- `src/sim/world/logic_vector.rs` — **new** `LogicVector` type owning the order
  Vec + invariant-enforcing ops.
- `src/sim/game_entity.rs` — add `in_logic_vector: bool` (`#[serde(skip)]`).
- `src/sim/world/world_spawn.rs` — limbo spawn stops registering; active spawns go
  through `reveal`.
- `src/sim/superweapon/paradrop.rs` — register paradropped passenger at *reveal*
  (landing), not at limbo creation.
- `src/sim/passenger.rs` — consumes pure vector order (no behavior change beyond
  removal of the sorted fallback).
- AI/parity phases in `advance_tick` — migrate `keys_sorted()` → vector snapshot,
  **incrementally** (see migration plan).

**Risk areas / blast radius:**

1. **Removing the sorted fallback (DRIFT 1) is the highest-risk single change.**
   If any active object is not registered, it silently vanishes from the order
   (and from garrison reconciliation). Mitigation: a debug assertion that
   `order.len() == count of revealed (non-limbo, in-store) entities`, plus an audit
   that every active spawn path calls `reveal`.
2. **`keys_sorted()` migration** must be selective. Converting an owner-query or a
   determinism-sensitive sort to vector order could change unrelated behavior.
   Mitigation: phase-by-phase migration gated on a full-skirmish state-hash
   regression; a phase flips only when the hash is unchanged or changes in the
   expected parity-improving direction.
3. **Paradrop limbo→reveal** is the one limbo site; must verify the passenger gets
   a reveal when it lands, else it never receives AI.
4. **Determinism:** vector order is deterministic iff spawn/reveal order is
   deterministic (it is — commands applied in sorted order, map load in fixed
   section order). Membership flag is `#[serde(skip)]`, rebuilt deterministically.

## Tiny-Detail Ledger (parity constraints carried to /write-plan)

1. Register = tail-append, no sort. `[GHIDRA 0x0055BAA0, verified]`
2. Re-register idempotent via membership-flag guard. `[GHIDRA 0x0055BAA5, verified]`
3. Unregister = order-preserving left-shift compaction, **not** swap-remove.
   `[GHIDRA 0x0055BAE0, verified]`
4. Unregister decrements count; logical membership = count, no tail-zeroing.
   `[GHIDRA 0x0055BAE0; doc PERTICKUPDATE §3.3]`
5. Unregister clears membership flag even when the id is absent/invalid.
   `[GHIDRA 0x0055BB00; doc]` — Rust `retain` + unconditional flag clear.
6. Membership flag (`+0x98`) is distinct from InLimbo (`+0x81`); never collapse.
   `[GHIDRA 0x005F6250 saves +0x81 not +0x98, verified]`
7. Insertion point is **reveal**, gated by `type+0x234` + game-mode — not
   construction. `[GHIDRA 0x005F4EC0 piVar5[0x8d], verified]`
8. Failed reveal/unlimbo → no live entry (sets InLimbo, returns 0).
   `[GHIDRA 0x005F4EC0 else-branch, verified]`
9. Limbo-created objects are absent from the live order until revealed.
   `[doc ACTIVE_OBJECT_ORDER §3.3; DRIFT 2]`
10. Same-tick append: an object appended before the walk reaches the old tail runs
    the same pass (count reloaded each iteration).
    `[GHIDRA 0x0055B5FB..0x0055B619; AAHeatSeeker2]` — Rust: append-sensitive
    phases use a live-reload loop; cross-phase appends are already honored when the
    appending phase precedes the consuming phase.
11. Self-removal mid-pass + compaction can skip the shifted object (no index
    repair). `[scheduler+remover; claim 9]` — preserve mechanics, do not
    special-case; low-frequency, document only.
12. Map-load seed order: Terrain → Units → Aircraft → Infantry → Structures →
    Smudge, then per-section INI key order, then reveal timing. Not sort-by-ID.
    `[GHIDRA 0x00686B20; claim 10]`
13. Save serializes the vector directly (count + ordered ids); restore in saved
    order; no re-derive, no sort. `[GHIDRA 0x00551B20/0x00551B90, verified]`
14. `+0x98` not serialized; membership derived from vector presence on load.
    `[GHIDRA 0x005F6250, verified; §3.4 hazard]` — Rust rebuilds the flag from the
    restored vector, avoiding the native stale/double-add hazard.
15. Vector cleared before the load stream is applied. `[GHIDRA 0x006851f0; doc]`
16. uninit → conceal removes from the vector immediately (AI stops); free is
    deferred (PendingDeleteList). `[GHIDRA 0x005F65F0; claim 7]` — Rust immediate
    unregister reproduces the AI-stop; inline store removal is output-equivalent
    **unless** a same-tick consumer reads the freed object (named risk, deferred).

## Chosen Approach

A small `LogicVector` owner type enforces the native order contract; the membership
byte lives on the entity; the existing phased tick consumes vector order; save/load
serializes the order directly and rebuilds membership on load.

### Components

**`LogicVector` (new, `src/sim/world/logic_vector.rs`)** — owns `order: Vec<u64>`,
nothing else. Enforces the contract in one place; the membership flag is passed in
by reference so the type stays storage-agnostic and unit-testable in isolation:

```rust
pub struct LogicVector { order: Vec<u64> }

impl LogicVector {
    /// Native adder: +0x98 guard → tail-append → set flag. Idempotent.
    fn register(&mut self, id: u64, member: &mut bool) {
        if *member { return; }       // ledger 2
        self.order.push(id);         // ledger 1 (tail, no sort)
        *member = true;              // ledger 1
    }
    /// Native remover: gate → order-preserving compaction → clear flag.
    fn unregister(&mut self, id: u64, member: &mut bool) {
        if !*member { /* still clear, ledger 5 */ *member = false; return; }
        self.order.retain(|&x| x != id);  // ledger 3,4 (compacting, not swap)
        *member = false;
    }
    /// No sorted fallback — the vector is the whole truth. (ledger 1, fixes DRIFT 1)
    fn snapshot(&self) -> Vec<u64> { self.order.clone() }
    fn len(&self) -> usize { self.order.len() }
    fn get(&self, i: usize) -> Option<u64> { self.order.get(i).copied() }
    fn clear(&mut self) { self.order.clear() }   // ledger 15
}
```

Serialization: `LogicVector` serializes as its inner `Vec<u64>` directly (ledger
13) — newtype `serde` like `EntityStore`. `order` is the only persisted field.

**`GameEntity.in_logic_vector: bool`** — `#[serde(skip)]` (ledger 14), default
false. Mirrors `+0x98`. Rebuilt on load.

**`Simulation` wrappers** keep the current method names; they fetch the entity flag
and delegate:

```rust
fn register_live_object(&mut self, id) {  // = native Reveal's append
    if let Some(e) = self.entities.get_mut(id) {
        self.logic.register(id, &mut e.in_logic_vector);
    }
}
fn unregister_live_object(&mut self, id) { /* symmetric */ }
fn live_object_order_snapshot(&self) -> Vec<u64> { self.logic.snapshot() }
```

**Lifecycle helpers (Simulation methods, native-named):**

- `reveal(id)` → `register_live_object` (and clears any limbo marker). Active
  spawns call this.
- `conceal(id)` → `unregister_live_object` (object stays in store = limbo).
- `unlimbo(id)` → `reveal(id)`.
- `uninit(id)` → `conceal(id)` then remove from store. `despawn_entity` routes
  through here so the AI-stop ordering is centralized (ledger 16).

Spawn wiring:
- `spawn_from_map_with_resolved`, `spawn_object_at_height` → `reveal` (active).
- `spawn_object_limbo_at_height` → **no register** (DRIFT 2 fix). Stays in store,
  `in_logic_vector=false`.
- `paradrop.rs` → call `reveal` on the passenger when it lands, not at spawn.

**`rebuild_logic_membership(&mut self)`** — post-load, set
`in_logic_vector=true` for every id in the restored order (ledger 14). Called from
the Simulation deserialize finalizer alongside `rebuild_owner_index`.

### Order-authority migration (the bulk of the work, incremental)

`advance_tick` stays phased. Each AI/parity-committing phase replaces
`keys_sorted()` with `live_object_order_snapshot()`:

- **Snapshot phases** (most): take the vector snapshot at phase start, iterate with
  `get_mut(id)`, skip ids no longer present. Covers movement, combat, mission,
  docks, production order, retaliation, passengers, etc.
- **Append-sensitive phases** (ledger 10): the phase that ticks freshly-appended
  objects (projectiles/anims) iterates with a **live-reload loop** — re-check
  `logic.len()` each iteration so a within-phase append runs the same pass.
  Cross-phase appends (unit fires in combat phase → bullet ticked by a later
  movement phase) are already same-tick because the later phase snapshots after the
  append.
- **Leave as `keys_sorted()`**: owner-indexed queries, render/UI helpers, and any
  pass where stable-id order is the intended contract (not AI order).

Migration is one phase per step, each gated by a full-skirmish **state-hash
regression** (hash unchanged, or changed only in the expected parity-improving
direction). This is tracked as a checklist in the implementation plan, not a single
mechanical sweep.

### Data Flow

```
spawn (active) ──reveal──▶ register(id, &mut flag) ──▶ order tail += id, flag=true
spawn (limbo)  ──────────▶ (store only, flag=false, not in order)
landing/unlimbo ─unlimbo─▶ reveal ─▶ register
death/destroy  ──uninit──▶ conceal ─▶ unregister (compacting) ─▶ store.remove
advance_tick phase ──────▶ snapshot()/live-reload ─▶ get_mut(id) per id in order
save ────────────────────▶ serialize logic.order (Vec) verbatim
load ────────────────────▶ logic.clear → deserialize order → rebuild_logic_membership
```

### Error Handling

No new error types. The contract is enforced by the `LogicVector` API (private
`order`, ops are the only mutators). Invariant guards as `debug_assert!`:
`order` has no duplicates; `order.len()` equals the count of in-store entities with
`in_logic_vector=true`. Release builds trust the invariant.

### Testing Strategy

`LogicVector` unit tests (no engine spin-up):
- `register_appends_to_tail_no_sort`
- `reregister_is_idempotent`
- `unregister_preserves_order_compacting` (register A,B,C; remove B; expect [A,C])
- `unregister_absent_id_is_safe_and_clears_flag`
- `snapshot_is_order_verbatim_no_sorted_fallback`

Spine integration tests:
- `limbo_object_registers_only_on_reveal_tail_append` (ledger 9)
- `saveload_restores_live_object_order_verbatim` (order [B,A,C] with mismatched
  creation ids survives) (ledger 13)
- `saveload_clears_active_order_before_restore` (ledger 15)
- `saveload_restored_member_removes_cleanly` (after load, dying a restored unit
  unregisters exactly once — no stale, no double-add) (ledger 14)
- `map_load_live_object_order_follows_native_section_sequence` (ledger 12)
- `logic_scheduler_append_during_pass_ticks_new_tail_same_tick` (ledger 10)

Regression gate: full-skirmish replay state-hash unchanged across each phase
migration step.

### Determinism

- Vector order deterministic given deterministic spawn/reveal order (preserved).
- Membership flag `#[serde(skip)]`, rebuilt from the ordered Vec — deterministic.
- No `HashMap`/`HashSet` in the order path; `LogicVector` is a `Vec`, membership is
  a `bool` on the entity. Lockstep-safe.

## Architectural Decisions

- **Follows CLAUDE.md** "a scheduler or subsystem owner owns native order" and
  "lifecycle helper APIs own reveal/conceal/limbo/unlimbo/uninit/delete effects":
  `LogicVector` is the order owner; `reveal/conceal/unlimbo/uninit` are the helpers.
- **Deviation (pragmatic, with a named unclosed DRIFT):** keeps the phased
  `advance_tick` instead of a literal single per-object AI walk. This design fixes
  the *intra-phase* iteration order (vector vs. sorted) — a strict parity
  improvement — and preserves register/remove/lifecycle/same-tick-append faithfully.
  It does **not** close the *inter-phase interleaving* DRIFT (all-move-then-all-fire
  vs. per-object full AI; see Scope → Deferred). That equivalence is **unproven**
  and must not be asserted; it is pre-existing and separable. Rationale for not
  closing it now: the true interleaved walk is a multi-session rewrite with heavy
  borrow-checker cost (each object's AI needs `&mut self` + read-all-others +
  spawn) and high regression risk.
- **Split-invariant wart (honest tradeoff):** the active-set truth is split across
  two owners — the order `Vec` in `LogicVector` and `in_logic_vector` on the entity.
  This faithfully mirrors gamemd (vector + `+0x98`) and matches the chosen
  flag-on-entity model, but the two can desync silently if an entity leaves the
  store without going through `unregister`/`uninit`. Guarded by the debug invariant;
  a single-source-of-truth design would be cleaner but less faithful to the binary.
- **Tech debt:** the `keys_sorted()` → vector-order migration is incremental; until
  every AI phase is converted, the engine mixes vector-order and sorted-order
  passes. Tracked as an explicit migration checklist; both orders are deterministic
  so this is a parity-progress state, not a correctness hazard.

## Alternatives Considered

- **True single-vector AI walk** — most literal port; *does* close the inter-phase
  interleaving DRIFT (its one real output advantage). Deferred, not dismissed:
  massive rewrite, high regression risk, fights Rust borrow/batching. Revisit if
  the interleaving divergence proves observable.
- **Hybrid projectile-only live walk** — folded into the chosen approach: phase
  ordering already gives cross-phase same-tick append; the live-reload rule covers
  the within-phase case without a separate sub-scheduler.
- **Parallel `HashSet` membership** — rejected (Q3): non-deterministic iteration
  risk, and the flag-on-entity mirrors `+0x98` more faithfully and serializes
  cleanly (by being skipped).
- **Keep linear `contains()` dedup** — rejected (Q3): O(n) per register and no
  explicit invariant anchor; the `bool` flag is O(1) and matches native exactly.
- **Including the global subsystem reorder now** — deferred: blocked on unresolved
  RE (unnamed `FUN_*` callees) and is a separable sequencing problem (named DRIFT,
  not a silent cut).
