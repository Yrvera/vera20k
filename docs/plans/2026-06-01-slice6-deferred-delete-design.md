# ObjectSubstrate Slice 6 — Deferred-Delete Queue + Dying Window Design (Fork B, revised)

> **REVISION NOTE (2026-06-01):** The original design assumed `Simulation::uninit` frees an
> entity immediately, so a same-tick reference resolves to `None`. **That premise was found
> CONTRADICTED in the port** during reconciliation (see §0). The user selected **Fork B**:
> keep the port's existing animation-length lingering window for animated deaths, give the
> immediate (structure/voxel) path gamemd's end-of-tick window via a real `pending_delete`
> queue, unify the lifecycle, and fix genuine gating gaps. This document is rewritten around
> that reality. Earlier "free-at-end-of-current-tick uniformly / resolves-to-None" text is
> superseded.

## 0. Reconciliation — discovered reality (supersedes the original premise)

The port does **not** uniformly free on `uninit`. There are **two distinct death-exit paths**:

1. **Animated deaths (infantry / SHP, `combat/mod.rs:986-1007`).** When `entity.animation.is_some()`,
   combat sets `dying=true`, clears targets/selection, switches the sprite to the death
   sequence, and **leaves the entity in the store** as its own corpse sprite. It is despawned
   only when the death animation finishes — and that despawn runs in the **app layer**
   (`app_sim_tick.rs:293-308`), *after* `advance_tick` returns (so after that tick's
   `state_hash`), via `sim.uninit()`. The corpse therefore lingers **multiple ticks** —
   a *longer* window than gamemd, which frees the unit at the death tick's end and renders
   the corpse as a separate `AnimClass`.

2. **Immediate deaths (structure / voxel vehicle, `combat/mod.rs:1008-1020`).** When
   `entity.animation.is_none()`, combat sets `dying=true` and pushes the id to
   `immediate_uninit_ids`. The world layer drains that list at **Phase 5** inside
   `advance_tick` (`world/mod.rs:1963-1965`, `for &dead_id in &combat_result.immediate_uninit_ids { self.uninit(dead_id); }`),
   freeing the slot the **same tick** the unit died. *This is the only path where the
   original "resolves to None" premise actually holds* — and it is the real parity gap:
   gamemd keeps the dead object resolvable-but-`IsAlive=0` through the rest of the tick,
   the port erases it mid-tick.

`uninit` itself (`world/mod.rs:939-969`) already does **everything synchronously except the
final free**: decrement owned counts (if `!dying`) → `remove_entity_occupancy` →
`clear_radio_contacts_for` → `conceal` (leave the logic vector) → set `presence=Dying` →
**`remove()` immediately**. The only thing this slice defers is that last `remove()`.

**Port-side audit conclusion carried in:** the dying-entity "gating bugs" the original design
worried about are largely **false positives** — live systems already gate on
`health.current==0`/`dying`, and only infantry/SHP linger, so building/power/AI/dock scans
never see a lingering dying *structure* today. One real consistency tidy already shipped:
commit `8f7b599` made `aircraft_dock`'s `cleanup_dead` alive-set filter `!dying` to match
`building_dock`/`miner`. **The remaining audit (§5) is narrower than the original doc's**:
because `uninit` *already* conceals + unmarks occupancy + decrements counts synchronously,
the only consumers that can observe the deferral are those that read **raw store membership**
(not logic-order, not occupancy, not owned-counts, not an `!dying` alive-set) during Phases
5.5→8.5. That set must be enumerated and proven to gate on `dying`.

**Out of scope (confirmed):**
- **Mind-control reversion** — gamemd reverts controlled units on the controller's death
  (`CaptureManagerClass::FreeAll`, evidence report §8). The port has only a `mind_controlled`
  flag, **no controller→slave mapping**, so there is nothing to revert yet. Deferred.
- **Transport-passenger despawn** — `combat/mod.rs:935-944` kills riders (sets `dying`) but the
  full passenger despawn is a known gap (`combat/mod.rs:904`). Not introduced or fixed here.
- **Fork C** (decouple death anims into separate `AnimClass`-style objects, free all units
  uniformly) — rejected: it is an internal-mechanism change with no observable benefit (the
  lingering-entity corpse already produces the identical visible animation), and CLAUDE.md
  explicitly says not to port the `ObjectClass`/`AnimClass` tree literally. Recorded as a
  possible future slice only if a concrete observable gap appears.

## 1. Goal (Fork B)

Unify the two death-exit paths through one deferred-delete queue so that **every** death is a
two-phase operation — synchronous detach/conceal/unmark/`dying`, then a deferred slot-free —
matching gamemd's `UnInit` → `ProcessPendingDelete` model:

- **Immediate path (structure/voxel):** enqueue at Phase 5 instead of removing; drain at the
  Phase 9 "building anims + cleanup" point, **before** the debug asserts + `state_hash`. The
  dead structure stays resolvable-but-`Dying` (off occupancy, off logic, counts already
  decremented) through Phases 5.5→8.5 — gamemd's end-of-tick window.
- **Animated path (infantry/SHP):** unchanged observable behavior — keep the
  animation-length lingering corpse. Its `uninit` (app layer, at anim end) routes through the
  same enqueue, immediately followed by an app-layer flush so the corpse frees at exactly the
  same point it does today (no extra tick of linger).

Hash/version: see §6 — **contingent on the gating audit**; do not assert a hash change as
fact before the audit + suite run prove it.

## 2. Architecture context (corrected)

- **`Presence` FSM** (`Limbo | InCell | Dying`, `game_entity.rs:139-149`) — serde-skip, **not
  hashed**, rebuilt on load. `Dying` is documented as transient "set after conceal, before the
  slot is freed; becomes a persistent, observable state only once deferred-delete lands." This
  slice makes it observable for the duration of the deferred window.
- **`dying: bool`** (`game_entity.rs:359`) — the hashed IsAlive-equivalent. Combat sets it (and
  decrements owned-counts) when HP hits 0; `uninit` does physical removal later.
- **`debug_assert_presence_consistent`** (`world/mod.rs:838-851`) runs at `advance_tick` tail
  (line 2261), **after** Phase 9. Its doc comment (`world/mod.rs:836-837`) currently claims "no
  in-store entity is ever `Dying` at a tick boundary in this slice." Under Fork B, `Dying`
  entities exist **mid-tick** (Phase 5→9) but are flushed before line 2261 → the assert still
  holds at its call point; **the comment must be updated** to "Dying entities exist between
  enqueue and the Phase 9 flush; the flush precedes this assert so none remain here."
  `derived_presence()` must also be checked: it must not be invoked on a `Dying` in-store
  entity by any path that runs before the flush (it isn't today — only the tail assert calls
  it). [verify in plan: `game_entity.rs` `derived_presence`]
- **Tick order in `advance_tick`** (verified, `world/mod.rs`): P1 ground movement (1659) → P2
  air/special (1696) → P2.5 rocking (1770) → P3 vision (1841) → P4 power (1855) → P4.5
  superweapons (1865) → P4.6 deploy (1872) → **P5 combat + death processing incl.
  `immediate_uninit` (1883/1963)** → P5.5 particles (2129) → P6 retaliation+passengers (2133)
  → P7 production/repairs/docks/ore (2139) → [`run_late_region`] P8 AI (1548) → P8.5 defeat
  (1581) → **P9 building anims + cleanup (1588)** → asserts (2259-2261) → `state_hash` (2262).
  *Vision (P3) and power (P4) run **before** combat (P5)* — they already saw the structure
  alive this tick; the deferral cannot affect them. The deferred-window consumers are
  **P5.5–P8.5 only**.
- **`SNAPSHOT_VERSION = 16`** (`snapshot.rs:22`). `pending_delete` is serde-skip → **no
  serialized-layout change**.
- **`cleanup_dead(&alive)` consumers** (aircraft_dock / building_dock / miner_dock /
  dock_reservations) self-heal stale ids each tick from an `alive` set; alive must be
  `present && !dying`.

## 3. Chosen approach — Fork B

### 3.1 The queue
- **`ObjectSubstrate.pending_delete: Vec<u64>`** (`substrate.rs`; `#[serde(skip)]`, transient).
  Empty at every tick/save boundary → not hashed, not serialized.
- **`Simulation::flush_pending_delete(&mut self)`** (`pub(crate)`) — drains the queue in
  insertion (= death) order, calling `self.substrate.entities.remove(id)` (or the existing
  remove path) for each, then clears it. Idempotent re-`remove` of an absent id is a no-op.

### 3.2 `uninit` becomes enqueue-not-free
`uninit` keeps its current synchronous order — decrement counts (if `!dying`) →
`remove_entity_occupancy` → `clear_radio_contacts_for` → `conceal` → set `presence=Dying` —
and additionally **sets `dying=true`** (idempotent; the count-decrement still reads the
*original* `dying`, so counts are still decremented exactly once). It then **pushes the id to
`pending_delete` instead of calling `remove()`**. The entity stays in the store, `dying`, off
occupancy, off logic, until a drain.

### 3.3 Drain points — reconciling the two paths
- **Inside `advance_tick`:** call `flush_pending_delete()` at the **end of Phase 9** (in
  `run_late_region`, after `tick_building_down`, before the frame-counter commit at
  `world/mod.rs:1631-1633`), so the drain precedes the asserts (2259-2261) and `state_hash`
  (2262). This drains every `uninit` called *within* `advance_tick`: the immediate
  structure/voxel path (P5), `tick_building_down`'s undeploy free (`world/mod.rs:1464`, P9),
  sell (`production_sell.rs:716`), slave_miner (`slave_miner.rs:473/555`), bridge/wall
  occupant kills (P5 post), etc.
- **App layer (anim-end despawn):** the death-anim despawn loop in `app_sim_tick.rs:300-308`
  runs *after* `advance_tick`. It already calls `sim.uninit(id)` per finished corpse. Add
  **`sim.flush_pending_delete()` once after that loop** so animated corpses free at exactly
  the same point as today (no extra tick of linger). [verify in plan: the loop's pre-`uninit`
  manual `occupancy_mut().remove` at `app_sim_tick.rs:302-306` — is it redundant with
  `uninit`'s `remove_entity_occupancy`? If so, note/leave; do not change behavior in this
  slice unless it's a proven double-op.]
- A start-of-`advance_tick` backstop flush is **not** added (would re-introduce a 1-tick
  linger for the animated path). The two explicit drains above cover all callers.

### 3.4 Detach — radio only; 1:1 fields gate-at-use (no proactive null)
The evidence report (§3.2, §8 RESOLVED) proved gamemd's `Detach_From_All_Lists` is an
**observer-notify / registry-removal** dispatch, **not** a fixed 1:1-field-null routine. The
1:1 cross-ref fields (`last_attacker_id`, `capture_target`, `bunker_occupant`,
`garrison_original_owner`) are **gated-at-use on `IsAlive`**, never nulled on death. The two
genuine membership links gamemd actively tears down are **radio** (already done via
`clear_radio_contacts_for` in `uninit`) and **mind-control** (out of scope — no port mapping).
**Therefore this slice adds NO new proactive cross-ref nulling** — doing so would be DRIFT and
would break the "resolves-to-`Dying`" acceptance. The port mirrors gamemd via `dying`-gating
during the window + by-id `None`-degradation after free. (No separate `detach_all_links` pass
is introduced; the original doc's §7.6 unified-null deviation is **not** taken.)

## 4. Components & interfaces

- `ObjectSubstrate.pending_delete: Vec<u64>` — transient deferred-delete queue (`substrate.rs`).
- `Simulation::flush_pending_delete(&mut self)` — `pub(crate)` drain.
- `Simulation::uninit` — enqueue + `dying=true` instead of immediate `remove()`.
- **Behavior contract change:** after `uninit(id)` and until the next drain, `store.get(id)`
  returns `Some` (a `Dying` entity) instead of `None`. Same-window consumers MUST gate on
  `dying`. No new public types; no serialized-layout change (serde-skip).

## 5. Gating audit (the load-bearing verification — for /write-plan, evidence required)

Because `uninit` synchronously conceals (off logic vector), unmarks occupancy, clears radio,
and decrements owned-counts, the **only** way a P5.5–P8.5 consumer can behave differently with
a `Dying` structure present vs absent is if it reads **raw store membership / iterates the
whole store** without a `dying` gate. Enumerate and prove each:

| Phase / consumer | Reads via | Gate today | Action |
|---|---|---|---|
| P5.5 ParticleSystems (`particles::system_ai`) | ? | ? | verify it doesn't attach to a dying structure id |
| P6 Retaliation (`combat::tick_retaliation`, logic_order) | logic-order | concealed off logic ⇒ never visited | confirm — likely no-op |
| P6 Passengers | cargo chain | — | confirm transport-pax path unaffected |
| P7 Production (`production::tick_production…`) | factory/queue state | ? | verify a dying factory/producer is excluded |
| P7 Repairs (`tick_repairs`) | building scan | ? | verify gates on `dying`/health |
| P7 Docks (`building_dock`/`miner`/`aircraft_dock`) | `cleanup_dead(&alive)` | `!dying` (8f7b599) | confirm all three alive-sets `!dying` |
| P7 Ore growth (`ore_growth`, `with_live_object_context`) | occupancy | occupancy unmarked synchronously | confirm occupancy-based, not store-membership |
| P8 AI (`ai::tick_ai`) | game-state scans | ? | verify AI target/threat scans gate on `dying` |
| P8.5 Defeat (`check_defeat`) | owned-counts | counts decremented at P5 | confirm — same as current |

**Outcome of the audit decides §6.** If every consumer already excludes the dying structure
(via logic-order/occupancy/alive-set/owned-counts), the **state hash is unchanged**. If any
consumer reads raw store membership and would now include the `Dying` entity, that is the
intended gamemd-faithful delta (and any such consumer that should *not* see it is a gating fix).

## 6. Hash / version decision (corrected — not assumed)

Per CLAUDE.md burden-of-proof, the hash change is **not asserted as fact**; it is determined
empirically:

1. Implement Fork B + the gating fixes (if any) from §5.
2. Run the full lib suite. All determinism/saveload/world_hash tests are **A-vs-B** comparisons
   (`assert_eq!(h1, h2)` with identical seeds, e.g. `world_tests.rs:1564/1604`, `snapshot.rs`)
   — **no hardcoded golden-hash constant exists in `sim/`** (verified by search). They stay
   green regardless of the absolute value, *provided determinism holds*.
3. Add a **new behavior test** exercising the immediate path (kill a structure that a later-
   same-tick consumer references; assert it resolves as `Dying`, not `None`, during the window
   and is `None` after the Phase 9 flush; assert `store.len()` correct).
4. **Version bump rule:**
   - If the audit/suite shows the **absolute state hash changes** for a representative scenario
     (a real observable delta), bump `SNAPSHOT_VERSION` 16→17 — the evidence artifact satisfies
     critic #4. This is the task's expected outcome and the most likely one if any P5.5–P8.5
     consumer reads raw store membership.
   - If the audit **proves no consumer observes the deferral** (hash identical *and* layout
     unchanged because `pending_delete` is serde-skip), **do not bump** — a gratuitous bump
     would reject old saves for a no-op. Instead document that the deferred window is latent
     until a consumer reads it, and **surface this to the user** before finalizing.

   **Default expectation (per burden-of-proof): the hash changes → bump to 17.** Confirm with
   evidence, do not paper over a no-change result.

## 7. Tiny-detail ledger

- Two death-exit paths; both route through `uninit`→enqueue; **two** drain points (Phase 9
  inside `advance_tick`; app-layer after the anim-end despawn loop). [§3.3]
- Immediate (structure/voxel) path: enqueue at P5, drain at P9 → resolvable-but-`Dying` through
  P5.5–P8.5. [code: `world/mod.rs:1963`; doc: SLICE6 §3.5]
- Animated (infantry/SHP) path: unchanged window (anim length); app-layer flush keeps the
  free-at-anim-end timing. [code: `combat/mod.rs:986-1007`, `app_sim_tick.rs:300-308`]
- `uninit` order unchanged except: set `dying=true`, push to `pending_delete` instead of
  `remove()`. Count-decrement still reads original `dying` ⇒ decremented exactly once. [§3.2]
- Detach = radio clear only (already present). 1:1 cross-ref fields gate-at-use, **not nulled**.
  [doc: SLICE6 §8 RESOLVED]
- Mind-control reversion + transport-pax despawn: **out of scope** (no port mapping / known
  gap). [task; SLICE6 §8]
- `pending_delete` transient: empty at every tick/save boundary; not hashed, not serialized.
- `debug_assert_presence_consistent` comment updated; assert still holds (flush precedes it).
  [code: `world/mod.rs:836-851`]
- `SNAPSHOT_VERSION` bump is **conditional on evidence** (§6); default expectation 16→17.
- No hardcoded golden-hash constant in `sim/`; determinism tests are A-vs-B. [verified]
- Mutual same-tick death (both structures): both enqueued in death order, both `Dying`-
  resolvable until the P9 drain; deterministic across replay. [doc: SLICE6 §3.6]

## 8. Testing strategy

- **Immediate-path window:** kill a structure in combat; within the same tick (a P6/P7
  consumer or a direct test harness) `store.get(id)` = `Some`+`dying`, absent from occupancy +
  logic; after the P9 drain `store.get(id)` = `None`, `store.len()` correct.
- **Animated-path unchanged:** an infantry death still lingers for its death animation and
  frees at anim end (no extra tick); existing animation/despawn tests stay green.
- **Same-tick cross-ref:** kill a structure another entity references; the reference resolves
  to `Dying` (gated-and-skipped), not `None`, during the window.
- **Mutual same-tick death:** two structures kill each other; each resolves the other as
  `Dying`; replay hash stable across two identical-seed runs.
- **Determinism / saveload:** full lib suite green; presence-consistency + logic-membership
  asserts hold (drain precedes them); snapshot round-trips (queue empty at save boundary).
- **Hash evidence:** record whether the absolute hash changes for the immediate-path scenario;
  drives the §6 version decision.

## 9. Determinism

The queue is drained in death order before the hash; empty at the boundary → not hashed, not
serialized. Determinism of the new window depends on: (a) deterministic enqueue order (death
order = `dead_entities` iteration order, already deterministic), (b) deterministic drain
(in-order `Vec` drain). The intended hash delta (if any, per §6) is same-tick references
resolving to `Dying` — a new deterministic golden is baselined only if the suite confirms a
real change; justified by the cited gamemd evidence (critic #4).

## 10. Architectural decisions

- **Follows** the substrate-owns-cross-cutting-state pattern (queue beside logic/occupancy/
  counters) and the Slice 2 `Presence`/`dying` shadow.
- **Does NOT** introduce the parent design §7.6 unified `detach_all_links` 1:1-field nulling —
  the RE showed gamemd gates those at-use (§3.4). Only the radio clear (already present) and
  the `dying`-gate + `None`-degradation are used.
- **Does NOT** take Fork C (separate `AnimClass` corpse objects) — internal-only, no observable
  benefit, contrary to CLAUDE.md's "don't port the class tree literally."
- **No tech debt created.** Mind-control reversion and transport-pax despawn are pre-existing
  gaps, recorded, not introduced here. The `vtable+0x44` ≥1-tick lingering predicate
  (`ObjectClass::IsDead` = `IsAlive==0`, always true post-UnInit ⇒ free same tick) confirms the
  end-of-tick window is the common-case parity, no multi-tick lingering to model.

## 11. Sources & references

- **Evidence artifact:** `docs/research/SLICE6_DEFERRED_DELETE_DYING_WINDOW_GHIDRA_REPORT.md`
  (HIGH; `UnInit 0x005F65F0`, `Detach_From_All_Lists 0x007258D0`, `ProcessPendingDelete
  0x00725C70`, `Main_Tick 0x0055D360`, drain predicate `IsDead 0x005F6690`).
- **Parent design:** `docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md` §8 Slice 6,
  critics #4/#6/#9.
- **Current code (verified this session):** `combat/mod.rs:840-1022` (two death paths),
  `world/mod.rs:939-975` (`uninit`/`despawn_entity`), `world/mod.rs:1463-1478`
  (`tick_building_down` uninit), `world/mod.rs:1539-1634` (`run_late_region`/Phase 9),
  `world/mod.rs:1659-2273` (tick order + Phase 5 immediate drain at 1963 + tail 2258-2262),
  `world/mod.rs:836-851` (presence assert), `app_sim_tick.rs:284-313` (post-`advance_tick`
  anim tick + anim-end despawn), `substrate.rs:45-83` (`ObjectSubstrate`),
  `game_entity.rs:130-149` (`Presence`), `snapshot.rs:15-22` (`SNAPSHOT_VERSION`).
- **Prior slices (dev):** Slice 5 EnterOrderCounter; Slice 4 incremental `by_owner`; commit
  `8f7b599` (aircraft_dock alive-set `!dying`).
- **INI keys:** none.
