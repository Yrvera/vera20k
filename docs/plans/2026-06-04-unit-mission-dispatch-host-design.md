# Per-object Unit Mission_Dispatch Host (shadow) Design

**Status:** DESIGN — `/design-review` cleared (REVISE findings P1–P3 folded in, see
Design-Review Resolution). Ready for `/write-plan`. No Rust written this session.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics. Shadow-first, hash-neutral.
**Ladder position:** the prerequisite that gates **S2** in
`docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §9. Sits between the
landed **S1** (dispatch-before-locomotor shadow) and the hash-affecting **S2** flip
(run locomotor `Process` after per-object dispatch + relocate the `+0xC4` increment).

## Goal

Fill the no-op `EntityCategory::Unit` arm of `techno_ai_shell` with **one authoritative
per-object Mission_Dispatch router** — read-only, hash-neutral — that consolidates the
scattered per-Unit mission-routing decision into a single `match mission.current` switch,
proven (debug-only) to agree with the scattered legacy dispatch, so the later S2 flip is a
small, safe change.

## Architecture Context

How the area works **today** (re-verified against current code this session; the
auto-memory and several code comments are stale — see Stale-Doc Follow-ups):

- **`object_ai_stage` runs at the TOP of the tick.** `advance_tick`
  ([src/sim/world/mod.rs:1928](../../src/sim/world/mod.rs)) calls it right after
  command-apply + `flush_pending_delete` (1904/1914), **before** Phase-1 ground movement
  (1934). This is the S2a relocation — the per-object dispatch site must precede the
  locomotor. The design doc §7.2 ("between docks and defeat") is stale on placement; the
  code is authoritative.
- **The shell is a strict no-op.** `techno_ai_shell`
  ([src/sim/world/techno_ai.rs:107](../../src/sim/world/techno_ai.rs)) matches all four
  categories to `{}`. `EntityCategory::Unit => {}` is what this slice fills.
- **MissionCom is hashed (Slice 8), but `current`/`substate` are a projection.**
  `hash_mission_com` is folded into the state hash
  ([src/sim/world/world_hash.rs:656](../../src/sim/world/world_hash.rs)). However
  `mission.current`/`mission.substate` are written each tick by `refresh_mission_shadow`
  ([src/sim/world/mod.rs:910](../../src/sim/world/mod.rs), called at the tail line 2575)
  as a deterministic projection of the legacy `Option<T>` machines via `derived_mission()`
  ([src/sim/game_entity.rs:523](../../src/sim/game_entity.rs)). So the **selector is
  authoritative-as-hashed-state, but the legacy machines still drive behavior.** That is
  precisely the gap this host begins to close.
- **`derived_mission` priority:** miner → aircraft → dock → attack → move → idle. For a
  Unit it can only produce `{Move, Attack, Enter, Harvest, None}` (and aircraft variants on
  the Aircraft category). It can **never** produce `AttackMove`.
- **Freshness window.** At `object_ai_stage` (top of tick) `mission.current` reflects
  end-of-(N−1) and does **not** yet include this tick's command mutations; it is fresh only
  at the tail after `refresh_mission_shadow`. The landed **S1 proof runs end-of-tick**
  (`debug_assert_s1_shadow`, [src/sim/world/mod.rs:2587](../../src/sim/world/mod.rs)).
- **Scattered legacy dispatch + their iteration sets:**
  - `tick_order_intents_pre_combat` / `_post_combat`, `tick_attack_pursuit` →
    `keys_sorted()` (all entities) ([src/sim/world/world_orders.rs:52/94/895](../../src/sim/world/world_orders.rs)).
  - combat `tick_combat_with_fog` → attacker snapshot in live order; turret rotation a
    separate post-combat sweep.
  - miner `tick_miners` → live-object order, inside the production phase (~mod.rs:2463),
    via `harvest_mission_step` ([src/sim/miner/harvest_mission.rs:46](../../src/sim/miner/harvest_mission.rs)).
  - aircraft → global sweep (`tick_aircraft_missions`, mod.rs:2066).
  - The **L2 facing pass deliberately used `keys_sorted()`** to match the legacy turret
    sweep's SET ([src/sim/world/unit_post.rs:46](../../src/sim/world/unit_post.rs)).
- **Mission verb API (Slice 6) exists** in `src/sim/mission/verb.rs`
  (`assign_mission`/`queue_mission`/`commence_queued`/`override_mission`/`restore_mission`/
  `get_current_mission`/`is_busy`/`ready_to_commence`). The host **reads** the selector
  this slice; the verb/commence-gate authority flip is S5.
- **Miner overlap:** the other session already shipped `harvest_mission_step` — the exact
  routing-seam pattern (debug-only `derived_mission()==Harvest` agreement, delegates to
  unchanged `process_miner`). The host must **not** re-own Harvest.

## Impact Analysis

**Files touched (this slice):**
- NEW `src/sim/mission/dispatch.rs` — pure router + slot table (no `sim` access).
- `src/sim/mission/mod.rs` — `pub mod dispatch;`.
- `src/sim/world/techno_ai.rs` — fill the `Unit` arm; add the debug-only proof method.
- `src/sim/world/mod.rs` — one call to the proof method end-of-tick (beside
  `debug_assert_s1_shadow` at 2587).

**Depends on:** the landed S0/S1 shell, MissionCom (Slice 8, hashed), `derived_mission`,
`mission::verb`. **No dependency** on the miner session's in-flight files.

**Blast radius:** near-zero. The host is **read-only** (no machine/mission/timer/RNG
mutation, no `tick_counter` touch), so the lockstep hash is bit-identical. The only risk
is the proof itself being wrong (false assert) — mitigated by deriving the proof from
`derived_mission` (the same projection the hash already trusts).

**Determinism / tick-order:** no phase reorder; `advance_tick` order preserved (invariant
#2). The host walks `live_object_order_snapshot()` (a frozen copy) — safe because the pass
mutates no membership. (The native same-pass `for_each_live_object` re-read is only needed
once the host mutates membership, S2+.)

## Chosen Approach

**A category-agnostic pure router in `mission/dispatch.rs`, called from the Unit arm, with
a debug-only end-of-tick divergence proof.** Chosen over inlining everything in
`techno_ai.rs` because gamemd's `Mission_Dispatch` is on the **common MissionClass vtable**
(shared switch, per-leaf handler overrides) — a shared router in `mission/` is the faithful
shape and sets up the S5 all-category flip for free, while keeping `techno_ai.rs` as the
scheduler. It also respects the ~600-line file convention (`techno_ai.rs` is already 649).

### Decision: the host routes by a FRESH re-derivation at host time, not by stale `mission.current`

gamemd's `Mission_Dispatch` routes each object by its **post-command** `CurrentMission`
(commands are applied before the LogicClass object loop). In Rust, commands are applied at
mod.rs:1904 (before `object_ai_stage` at 1928), but `mission.current` is only re-projected
from the machines at the tail (`refresh_mission_shadow`, 2575). So at host time
`mission.current` is **stale** — it reflects end-of-(N−1), missing this tick's commands.

Therefore the host (this slice and the authoritative S2 host it sets up) routes by
**`entity.derived_mission()` evaluated fresh at host time** — the faithful stand-in for
"post-command current mission" until the verb-API authority flip (S5) makes
`mission.current` itself post-command-fresh. Reading the stale `mission.current` was the
rejected alternative (see Alternatives Considered); it would bake a one-tick command-lag
into the very ordering S2 is meant to get right.

This makes the slice's proof **non-vacuous**: the host records its fresh top-of-tick routing
and the end-of-tick proof compares it against the fresh **end-of-tick** derivation,
*surfacing* (counting, with tick+id) the divergence caused by mid-tick machine churn — never
asserting a value equal to itself. The divergence count is the slice's headline metric for
the S2 go/no-go (how much a Unit's settled mission moves within one tick).

### Components

The router carries **two distinct concerns**, which must not be conflated (see [P2]):
the gamemd switch→slot *table fidelity*, and the coarse *Unit handler-family* routing.

1. **`mission/dispatch.rs` (pure, no `sim`):**
   - `fn dispatch_slot_offset(mission: MissionType) -> Option<u16>` — the verified 32-case
     switch→slot **offset** mapping from §3(e) (`+0x204..+0x270`), the directly gamemd-cited,
     fully table-testable artifact. Total over all 32 ids + `None`. Encodes: `QMove`→default
     `+0x204` (Sleep); `Ambush(14)`→`+0x20c` (inert stub); `Guard(5)`==`Sticky(6)`→`+0x21c`;
     `Capture(8)`==`Sabotage(17)`→`+0x214`; `None`→`+0x204`. (Refined in `/write-plan` to a
     family-granularity `unit_dispatch_family` for this Unit-only slice; the full per-slot
     table is deferred to S5. Verified note: `AttackMove(29)` is NOT a dispatcher skip — the
     binary routes it via `default` to Sleep `+0x204` + timer rewrite; it is simply never a
     committed CurrentMission, so the router models it defensively. See the Tiny-Detail Ledger.)
   - `enum DispatchSlot { Move, Attack, Guard, Enter, Harvest, Unload, Hunt, Sleep, Skip,
     OtherInert }` — the coarse **Unit handler family** (the families a Unit's behavior
     actually uses), NOT a 1:1 of the 28 gamemd slots. `Skip` = AttackMove (never
     dispatched). `OtherInert` = any mission with no Unit handler family this slice
     (Capture/Sabotage/Eaten/AreaGuard/Return/Repair/Rescue/… — real gamemd slots, but no
     *Unit* leaf behavior is routed here yet; represented-but-inert for the Unit arm).
     TS-legacy Ambush also lands in `OtherInert`.
   - `fn unit_dispatch_family(mission: MissionType) -> DispatchSlot` — the Unit-arm routing
     used by the shell. The reachable-Unit set `{Move, Attack, Enter, Harvest, Guard, None}`
     maps to live families (`None`→`Sleep`); everything else → `Skip`/`OtherInert`.

2. **`techno_ai.rs` Unit arm:** a thin `unit_mission_dispatch_shadow(sim, &mut trace, id)`
   that evaluates **`entity.derived_mission().0` fresh** (per the Decision above — NOT the
   stale `mission.current`), computes `unit_dispatch_family(..)`, and (debug-only) records
   `(id, fresh_mission, family)` into a per-pass trace. **No handler body runs.** Miner Units
   (`miner.is_some()`) are skipped (Q3). Other category arms stay `{}`.

3. **Proof method** `debug_assert_unit_dispatch_shadow(&self, trace)` — runs end-of-tick at
   mod.rs:2587 beside `debug_assert_s1_shadow`. For each recorded `(id, fresh_mission,
   family)` it re-evaluates `derived_mission().0` **now** (end-of-tick fresh) and **logs**
   any family divergence with `tick + id + both missions` (the slice's churn metric) — it
   does **not** assert equality (host-time and tail-time derivations legitimately differ when
   a Unit's machines change mid-tick). The hard asserts it *does* make are the non-vacuous
   invariants: slot-table fidelity, reachable-missions-route-live, live-set coverage, and
   miner/non-Unit skip (see Testing Strategy). Read-only; never hashed; never silently
   equalized (S1/L5 discipline).

### Interfaces / Contracts

- `dispatch_slot_offset` is total over all 32 `MissionType` ids + `None`; pure; no panics;
  returns `None` only for `AttackMove`. `unit_dispatch_family` is total and pure.
- The Unit arm is read-only w.r.t. all hashed state. Signature of `techno_ai_shell` is
  unchanged (the `sim`/`id`/`category` threading already exists).
- The host routes by `derived_mission().0` evaluated **fresh at host time** (not the stale
  `mission.current`). The proof re-evaluates `derived_mission().0` end-of-tick and compares
  families to measure churn — a logged metric, not an equality assert.

### Data Flow

```
advance_tick (top):  apply_commands → flush_pending_delete → object_ai_stage
                                                              └─ techno_ai_shell(Unit)
                                                                 └─ unit_mission_dispatch_shadow
                                                                    [read-only: derived_mission() FRESH → family → record]
...                  ground move → air → aircraft → combat/pursuit → ... → production(miner) → late
advance_tick (tail): refresh_mission_shadow (mission.current ← derived_mission) → state_hash
                     debug_assert_s1_shadow
                     debug_assert_unit_dispatch_shadow(trace)   [NEW: invariant asserts + churn log]
```

The host trace is a per-tick scratch `Vec` (debug-only; release never allocates, mirroring
the S0 `record` flag). It is threaded from `object_ai_stage` to the end-of-tick proof.

### Error Handling

No fallible paths. `dispatch_slot_offset`/`unit_dispatch_family` are total. The proof uses
`debug_assert!` (stripped in release) for the hard invariants and `log`-style surfacing for
the churn metric. An absent entity / dying Unit is skipped (inherits the S0 walk guards).

### Testing Strategy

Debug-only proof + a replay golden — same shape as S0/S1. Note the agreement assert is
**not** an equality of `mission.current` with `derived_mission()` (that is tautological after
`refresh_mission_shadow`); the non-vacuous invariants are the pure-function tables, the
live-set coverage, and the skip rules.

- `unit_dispatch_host_is_hash_neutral` — full replay over a fixed seed; `state_hash`
  per-tick bit-identical to pre-slice (read-only shadow).
- `unit_dispatch_slot_offset_matches_gamemd` — `dispatch_slot_offset` matches the §3(e)
  case→slot **offset** table for all 32 missions + `None`: `QMove→+0x204`, `AttackMove→None`,
  `Ambush→+0x20c`, `Guard==Sticky→+0x21c`, `Capture==Sabotage→+0x214`, `None→+0x204`.
- `unit_dispatch_family_reachable_missions_route_live` — `unit_dispatch_family` maps
  Move/Attack/Enter/Harvest/Guard/None each to a **live (non-`Skip`/non-`OtherInert`)**
  family; `AttackMove→Skip`; Ambush/Capture/…→`OtherInert`.
- `unit_dispatch_family_agrees_with_fresh_derived_mission_at_proof` — at end-of-tick,
  re-deriving each recorded Unit's `derived_mission().0` and routing it yields a family; any
  difference from the recorded host-time family is **logged with tick+id** (churn metric),
  asserted only to be a *recognized* divergence (machine actually changed), never silently
  equalized. The headline metric: divergence count is reported, not asserted to zero.
- `unit_dispatch_live_set_covers_legacy_touched_units` — every Unit whose machines would
  make a legacy dispatch phase touch it **after that phase's own guards** (see T5 triage
  rule below) is present in the host's `live_object_order_snapshot` set; any that is **not**
  is logged as surfaced drift (Q2 decision), never equalized.
- `unit_dispatch_skips_miner_and_nonunit` — miner Units and non-Unit categories are skipped
  this slice.
- `unit_dispatch_attackmove_unreachable_for_units` — assert `derived_mission` never yields
  `AttackMove` for a Unit (so the `Skip` family is unreachable from the live host this slice).

**T5 triage rule (resolves [P3]).** The legacy dispatch phases iterate `keys_sorted()` (all
entities) but apply their own skip-filters; the host iterates `live_object_order_snapshot()`
(LogicVector). T5's "expected-touched" set is therefore Units that carry a dispatch machine
**and pass the corresponding legacy phase's guards** — i.e. mirroring `tick_attack_pursuit`'s
skips (`!dying`, not `Structure`, `aircraft_mission.is_none()`, `!is_deployed()`,
`!passenger_role.is_inside_transport()`) and the analogous guards for movement/dock/order
phases. With those guards applied the expected set is **expected-empty outside live order**
in normal play (a passenger/limbo/deployed Unit is both excluded from live order and skipped
by the legacy phase). T5 therefore **logs** any residual member with `tick+id` for triage
rather than hard-asserting — a genuine member is a real Rust drift to investigate before S2,
not a test bug.

## Tiny-Detail Ledger

Parity constraints the **router** must carry (the handler **bodies** are not moving this
slice). Each cites its source; nothing invented.

- **Switch→slot table** (VERIFIED in `/review-plan`, `decompile 0x005B3060`) — `QMove(3)` AND
  **`AttackMove(29)` BOTH hit `default` → Sleep `+0x204` WITH a timer rewrite** (`+0xC8=frame`,
  `+0xD0=handler_return`). There is **no dispatcher skip for 29**; it is never a committed
  CurrentMission (assign-side prevents it). The router models 29 with a defensive `Skip`
  (never reached). `Ambush(14)`→`+0x20c` inert stub (`return 0x1C2`); `Guard(5)`&`Sticky(6)`→
  `+0x21c`; `Capture(8)`&`Sabotage(0x11)`→`+0x214`. **Corrects the source-doc claim
  "AttackMove falls off the switch, no dispatch, no timer rewrite" (WRONG).** `[decompile
  0x005B3060 this session; supersedes TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md
  §2.7/§3(e)/§7.6]`
- **Dispatch gate order** (recorded for S2, **not enforced in this read-only shadow**):
  `IsActive(+0x90)` → frame-anchored timer due (`binary_frame − +0xC8 ≥ +0xD0`) →
  `Health(+0x6C)>0` → switch. `[doc §2.2]`
- **Mission ids are the TS-shifted numbering** (Eaten=9, Harvest=10, AreaGuard=11,
  Ambush=14) — already correct in `MissionType`. `[doc §2.6; src/sim/mission/mod.rs:30]`
- **Routing consistent with `derived_mission` priority** miner→aircraft→dock→attack→move→
  idle. `[src/sim/game_entity.rs:523]`
- **Host routes by `derived_mission()` evaluated FRESH at host time**, NOT by
  `mission.current` (which at host time, mod.rs:1928, is stale: it reflects end-of-(N−1)
  and excludes this tick's commands applied at 1904; it is re-projected only at the 2575
  tail). gamemd routes by post-command `CurrentMission`; fresh re-derivation is the faithful
  stand-in until the verb-API authority flip (S5). `[src/sim/world/mod.rs:1904/1928/2575;
  doc §2.2]`
- **Reachable Unit missions** = `{Move, Attack, Enter, Harvest, Guard/None}`; **AttackMove
  never reachable for a Unit** → assert. `[src/sim/game_entity.rs:523]`
- **`None` (idle sentinel) routes to the Sleep family**, not Guard. `[src/sim/game_entity.rs:550]`
- **Hash neutrality:** shadow consumes **no RNG**, mutates no machine/mission/timer, does
  **not** touch `tick_counter` (the `+0xC4` per-object increment relocation is S2). `[doc §7.5]`
- **Iteration set = `live_object_order_snapshot()` (LogicVector)** — gamemd-correct
  dispatch set; frozen snapshot is safe because the pass mutates nothing. The native
  same-pass re-read (`for_each_live_object`) is only required once the host mutates
  membership (S2+). `[doc §7.2; Q2 decision]`
- **Units-only this slice;** Structure/Infantry/Aircraft arms stay `{}`; miner Units
  excluded (the miner session's L5 owns Harvest). `[Q3 decision]`

## Architectural Decisions

- **Follows existing patterns:** the pure-function module mirrors `mission/verb.rs`
  (pure `(MissionCom, …)` functions); the debug-only end-of-tick proof mirrors
  `debug_assert_s1_shadow`; the surface-divergence-never-equalize discipline mirrors the
  miner L5 `harvest_mission_step` and S1. No new pattern introduced.
- **`match category` + `match mission`, no `dyn`/vtable** (invariant #2). `DispatchSlot` is
  data, not a trait object. The router is the Rust stand-in for the MissionClass vtable
  switch; the per-leaf handler "overrides" become later `match category` arms.
- **Single owner preserved:** the host mutates nothing; all behavior stays in the legacy
  phases / substrate API. No second owner of presence/mission/active-vector state.
- **Tech debt:** the router's slot table duplicates knowledge that will later live in the
  authoritative handlers; acceptable because `dispatch_slot_offset` is the testable contract
  S2+ build on, and it is asserted against the gamemd table. The Harvest arm is a deliberate
  no-op-skip until a later merge slice wires it to the miner session's `harvest_mission_step`.

## Alternatives Considered

- **Inline the router + proof in `techno_ai.rs`** (no `mission/dispatch.rs`). Rejected:
  pushes `techno_ai.rs` past the file-size convention, hides the dispatch contract from the
  other category arms, and doesn't reflect gamemd's shared-MissionClass shape. The S5
  all-category flip would have to re-extract it.
- **Router + read-only shadow handlers that recompute each mission's would-be output**
  (Move/Attack/Guard/Stop) and assert against legacy phase outputs. Rejected for this slice
  (user chose router-skeleton depth): front-loads recompute work that belongs to S2/S3
  where those bodies actually move, and risks coupling the shadow to legacy-phase internals
  that are themselves slated for retirement.
- **Iterate `keys_sorted()` (legacy-union set) first** to mirror the exact legacy SET for a
  strictly identical shadow. Rejected (user chose live-order): it bakes in the Rust
  substitution the host is supposed to retire and would need a second migration; live-order
  is gamemd-correct and the host is hash-neutral either way, so surfacing legacy
  over-dispatch as drift now is the correct burden-of-proof posture.
- **Host reads the stale `mission.current` (no fresh re-derivation).** Rejected (design
  review [P1]): at host time `mission.current` excludes this tick's commands, so the host
  would route by a one-tick-lagged mission and the end-of-tick agreement check would be
  tautological (`mission.current == derived_mission()` by construction after the 2575
  refresh — it proves nothing). Fresh re-derivation matches gamemd's post-command routing
  and makes the churn proof non-vacuous.
- **Delegate the Harvest arm to `harvest_mission_step` now.** Rejected (user chose
  exclude): the miner session is actively editing `src/sim/miner/*`; excluding Harvest
  avoids file contention and is not a parity hole (L5 proves Harvest classification
  independently). Wired in a later merge slice.

## Sequencing & Handoff

1. This slice (S1.5, the **S2 prerequisite**): land the router + Unit-arm shadow + proof.
   Hash-neutral; no `SNAPSHOT_VERSION` bump.
2. **S2** (separate, hash-affecting): run locomotor `Process` after the host for scoped
   Units + relocate the `+0xC4`/`tick_counter` increment to per-object-before-dispatch.
   Carries the `SNAPSHOT_VERSION` bump + gamemd-cited golden re-baseline.
3. **Later merge slice:** wire the host's Harvest arm to the miner session's
   `harvest_mission_step` once both have landed.

## Stale-Doc Follow-ups (documentation only)

- `src/sim/game_entity.rs:490-497` (`mission` field doc) says "NOT folded into world_hash
  yet" — **stale**; it IS hashed as of Slice 8.
- `src/sim/game_entity.rs:518-523` (`derived_mission` doc) says a later slice makes mission
  authoritative and "this becomes the cross-check" — **partially stale**: the hashing
  happened; `derived_mission` is still the *writer* of `current`/`substate`, not yet the
  cross-check.
- `TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §7.2 places `object_ai_stage`
  "between docks/production and defeat" — **stale**: code runs it at the top of the tick
  before ground movement (mod.rs:1928).
- Auto-memory entries calling MissionCom a "shadow-mode Slice 3" are stale (Slice 8 made it
  hashed/authoritative).
- `TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §2.7/§3(e)/§7.6 claim "AttackMove 29
  falls off the switch (no dispatch, no timer rewrite)" — **WRONG** per `decompile 0x005B3060`
  (`/review-plan`): 29 hits `default` → Sleep `+0x204` + timer rewrite, identical to QMove. The
  accurate statement is "29 is never a committed CurrentMission (assign-side prevents it); the
  dispatcher has no special skip." Correct via `/audit` on that doc (cite `0x005B3060` inline).

## Open Items (carried, not asserted as settled)

- The frame-anchored dispatch **gate** (IsActive → timer → Health) is recorded but not
  enforced until the host executes handlers (S2+). Do not let the slot table imply the gate
  is live this slice.
- The per-object `+0xC4` increment-before-dispatch is an **S2** change; `tick_counter` stays
  written at the tail this slice.
- The §3(e) slot **offsets** are doc-sourced (`decompile 0x005B3060`), not re-verified this
  session. They do not affect behavior this slice (no handler executes), but if
  `dispatch_slot_offset` is to be load-bearing for S2, spot-verify the offsets against the
  binary before baselining the table test.
- The churn-divergence count (host-time vs tail-time fresh derivation) is the slice's S2
  go/no-go metric; its expected magnitude is unknown until measured. A high count means S2's
  authoritative host must re-derive at host time (already the chosen design), not read a
  cached selector.

## Design-Review Resolution (2026-06-04)

`/design-review` returned **REVISE**; all findings are now folded in:

- **[P1] Tautological agreement test** → resolved: the host routes by **fresh
  `derived_mission()` at host time**, and the proof **logs churn** (host-time vs tail-time
  family) instead of asserting a value equal to itself. The read-stale alternative is
  recorded as rejected. (Decision subsection + Testing Strategy + Alternatives.)
- **[P2] `DispatchSlot` can't satisfy the full 32-case table** → resolved: split into
  `dispatch_slot_offset(mission) -> Option<u16>` (gamemd §3(e) table, tested for all 32) and
  the coarse `unit_dispatch_family` / `DispatchSlot` Unit routing with explicit
  `Skip`/`OtherInert` families. (Components + Testing Strategy.)
- **[P3] T5 may fire on legitimate states** → resolved: T5's expected-touched set mirrors
  each legacy phase's own guards and **logs** residual members for triage rather than
  hard-asserting. (T5 triage rule.)
