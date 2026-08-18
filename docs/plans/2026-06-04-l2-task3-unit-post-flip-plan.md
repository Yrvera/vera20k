# L2 Task 3 — Flip `unit_post` to authoritative for Unit fire+facing (Approach B)

> **For Claude:** Execute task-by-task. Each task is self-contained. This slice makes
> the per-object `unit_post` host the **authoritative owner of Unit-category fire AND
> facing** (Approach B — full ownership): the combat Phase-2 loop dispatches Unit
> attackers to `unit_post` (interleaved in live order, so emission order is preserved),
> and the legacy `tick_turret_rotation` Unit arm is retired. **The flip is expected to be
> hash-neutral** (the L2-Task-2 shadow proved per-object output == the legacy sweeps), so
> the gate is the replay/global-parity golden staying **UNMOVED** — NOT a `SNAPSHOT_VERSION`
> bump. If a golden moves, STOP and diagnose; do not blindly re-baseline.

**Goal:** Make `unit_post` (`src/sim/world/unit_post.rs`) the authoritative owner of
Unit-category fire+facing in Phase 5 — combat dispatches Unit fire through `unit_post`
and `unit_post` drives Unit barrel facing — retiring the Unit arm of the global
`tick_turret_rotation` sweep, with the per-tick `state_hash` unchanged.

**Architecture:** The L2-Task-2 shadow (commits `a592bfb6`, `ebafb93a`, `443bdb4b`,
`9e6cb7d3` on `dev`) computes Unit fire+facing per-object and `debug_assert`s agreement
with the legacy sweeps every debug tick — 3615/0, global parity golden green. This slice
promotes that proven-equivalent per-object path to authoritative. Fire stays in the single
live-order Phase-2 pass (dispatched to `unit_post` per Unit); facing moves from the id-order
sweep to a per-object live-order pass owned by `unit_post`. Stays inside Phase 5 (invariant
#3: `object_ai_stage` untouched; the S4 relocation is a later slice).

**Approach decision (user-confirmed):** **B — full fire+facing ownership** via interleaved
category dispatch in the Phase-2 loop. (Approach A — facing-only — was rejected; it didn't
meet the fire-ownership goal.)

**Design Doc:** `docs/plans/2026-06-02-ai-shell-unitclass-core-plan.md` Slice L2 §4 Task 3/2a,
§5, §7, §8. **Gating verdict:** `docs/research/L2_FIRE_DAMAGE_TIMING_VERDICT_GHIDRA_REPORT.md`
(deferred-projectile → keep batched P4/P6 — settled).

---

## Grounding Summary

- **Not blocked.** The design doc's cross-cutting line "L2 Task 3 (combat flip) blocked on
  BOTH prerequisites" (MissionCom-authority + dispatch host) is an over-generalization for
  the *movement* flips. The authoritative L2 §8 dependencies (design `:428-431`) list only
  Task 0 (done) + the live `AttackerSnapshot`/`FacingClass` API, and `:430` states verbatim
  *"No dependency on S0–S2 scaffolding."* The combat flip is self-contained in Phase 5.
- **Damage stays batched (Task 0, VERIFIED).** `UnitClass::Fire_At_Target @ 0x00736df0` →
  `TechnoClass::Fire_At @ 0x006fdd50` launches a munition; **no HP applied in the fire pass**
  (HP lands later via `BulletClass::BulletDetonation @ 0x00468d80`). Route Unit `damage_events`
  through the existing batch unchanged. (`L2_FIRE_DAMAGE_TIMING_VERDICT_GHIDRA_REPORT.md`.)
- **Fire-before-Facing, per object (VERIFIED).** `UnitClass::AI @ 0x007360c0` calls
  `Fire_At_Target` (`0x007365e1`) then `Facing_Update` (`0x007365e8`); fire reads the
  previous-tick barrel facing. `Fire_At_Target` operates on the *current* `Target` and does
  NOT re-acquire a dead target inline (the switch on the fire-error code only aims — case 2
  `RateTimer__Set` on BarrelFacing — or fires — case 0 `vtable+0x3cc`); re-acquisition is the
  upstream targeting layer. (`GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT_GHIDRA_REPORT.md` §4 + live
  `decompile_function 0x00736df0` this session.)
- **Deaths are deferred in Rust (VERIFIED live).** `handle_entity_deaths` (`combat/mod.rs:812`)
  sets `dying = true` + pushes `despawned_ids` (`:995/:1006/:1012/:1018`); no `entities.remove()`.
  Removal is `flush_pending_delete` (end of Phase 5), AFTER both the fire pass and the facing
  pass. So a killed target stays in the store at facing time → facing toward its position is
  identical pre/post-combat.
- **Repo pattern:** the shadow path itself (`world/unit_post.rs`, `turret::desired_turret_facing`,
  `combat::build_attacker_snapshot`/`resolve_attacker_fire`). The flag
  `L2_UNIT_POST_AUTHORITATIVE` is the seam.
- **INI:** none — pure plumbing.
- **Unknown after grounding (→ Open Questions):** (a) the empirical golden result (expected
  unmoved); (b) the two YELLOW verdict items (vtable slot read; estimated-health write) —
  both fire-time *target selection*, neither modeled in our combat today, so non-blocking.

## Key Technical Decisions

- **The flip is hash-neutral; the gate is "golden UNMOVED," not a version bump.** The
  L2-Task-2 shadow asserts every debug tick (3615/0, global parity golden green) that
  per-object Unit fire (`fire_events` → `(attacker_id, weapon_id)`) and post-combat Unit
  facing (`desired_turret_facing == sweep destination`) equal the legacy sweeps. Promoting
  the proven-equivalent path to authoritative changes no output. The design doc's Task 4
  (`SNAPSHOT_VERSION` 17→18 + re-baseline) assumed a fire-emission reorder this plan avoids.
  **Confidence:** medium → **flag for /review-plan** (the empirical golden run is the proof).
  **Source:** L2-Task-2 shadow agreement + deferred-death verification (`combat/mod.rs:812-1018`).
- **Fire ownership = dispatch Unit snapshots to `unit_post` inside the existing live-order
  Phase-2 loop — do NOT add a separate Unit fire pass.** Units already fire via
  `resolve_attacker_fire` in live-LOGIC order in `tick_combat_with_fog` Phase 2; that order is
  what the post-combat `scenario_rng` smudge drain depends on. Keeping the dispatch *inside*
  the same loop (Unit → `unit_post`, else → `resolve_attacker_fire`) preserves emission order
  exactly while making `unit_post` the Unit fire entry point. A separate pre/post pass would
  batch Unit smudges apart from non-Unit → RNG-cursor shift → hash move. **Confidence:** high —
  **Source:** design §7 RNG/smudge invariant + `combat/mod.rs` Phase-2 live-order sort `:1562`.
- **`unit_post` does Fire then Facing per object, facing toward the post-fire target.**
  After `resolve_attacker_fire` emits into `emit`, `unit_post` reads this unit's
  `emit.retarget_events` / `emit.remove_attack` to determine the post-fire target (retarget →
  new entity target; remove → no target/body facing; else the snapshot's target), then
  `barrel.set_rot` + `barrel.set` toward it. This matches gamemd (Fire→Facing, facing the
  re-acquired target) and the legacy sweep (which faces the post-Phase-3-retarget target).
  **Confidence:** medium → **flag for /review-plan.** **Source:** Fire→Facing decompile +
  legacy Phase-3 retarget apply (`combat/mod.rs:1613`).
- **Cooldown/burst-delay decrement stays in combat Phase-1 this slice (shared, order-independent).**
  The Phase-1 snapshot build (incl. the `saturating_sub(1)` decrement, `combat/mod.rs:1421-1422`)
  still builds Unit snapshots that feed the Phase-2 dispatch; `unit_post` consumes the
  already-decremented snapshot (no double-decrement). Moving the decrement into `unit_post` is
  order-independent (pinned by `unit_cooldown_decrement_order_independent`) and hash-neutral,
  but is deferred to keep this slice's hash-critical surface minimal — absorbing the Phase-1
  build into `unit_post` is the S4 walk's job. **Confidence:** high — **Source:** design Task 2a
  + order-independence test (already green).
- **The shadow is removed by this slice.** Once `unit_post` is the authoritative Unit fire
  entry, the L2-Task-2 `l2_unit_post_fire_shadow`/`l2_unit_post_assert` (which compared a
  scratch per-object computation against the legacy Unit arm) no longer have a legacy arm to
  compare against. Delete the shadow compute + assert wiring in Phase 5; the named acceptance
  tests (Task 4) replace its coverage. **Confidence:** high — **Source:** the shadow's contract
  (`world/unit_post.rs` doc comments).
- **Two YELLOW verdict items are non-blocking.** vtable+0x3cc→0x6fdd50 read and the
  estimated-health write are about fire-time target *selection*; our combat models neither,
  and `unit_post` calls the same `resolve_attacker_fire`, so the flip preserves today's
  behavior. **Confidence:** medium — **Source:** verdict doc §"Unverified" + YELLOW box.

## Open Questions

### Resolved During Planning
- *Is the combat flip blocked on MissionCom-authority / a dispatch host?* — No (L2 §8 `:430`).
- *Inline-kill vs deferred-projectile?* — Deferred (Task 0). Keep batched P4/P6.
- *Does `Fire_At_Target` re-acquire a dead target inline before facing?* — No; our analog is
  `resolve_attacker_fire`'s `retarget_events`, which `unit_post` reads for the facing target.
- *A vs B?* — **B** (user-confirmed): full fire+facing ownership.

### Deferred to Implementation
- **Does the replay/global-parity golden stay unmoved?** Expected yes (shadow-proven +
  deferred deaths). Task 6's golden run is the proof. If it moves: STOP, diagnose which
  sub-change moved it (emission order? a despawn-mid-combat facing edge? the dispatch wrapper?),
  fix to neutral — do NOT re-baseline without proving a real, gamemd-justified behavior change
  and surfacing it to the user first.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/world/unit_post.rs` | Fill authoritative `unit_post` (Fire via shared body → post-fire Facing); flip `L2_UNIT_POST_AUTHORITATIVE`→`true`; remove the shadow methods |
| Modify | `src/sim/combat/mod.rs` | Phase-2 loop dispatches `category == Unit` snapshots to `unit_post`, else `resolve_attacker_fire` (gated on the flag) |
| Modify | `src/sim/movement/turret.rs` | `tick_turret_rotation` skips Unit-category entities when authoritative |
| Modify | `src/sim/world/mod.rs` | Phase 5: remove the debug shadow compute + assert; combat now owns Unit fire+facing via the dispatch |

## Interface Changes

- `L2_UNIT_POST_AUTHORITATIVE`: `false`→`true`.
- `unit_post(...)`: body filled. Signature takes the disjoint refs the dispatch needs
  (`&mut EntityStore`, occupancy, fog, overlay grid/registry, terrain, `&mut interner`, snap,
  binary_frame, tick_ms, `&mut CombatEmit`) — finalize against `resolve_attacker_fire`'s
  signature + a barrel `get_mut`. Still `pub(crate)`.
- `l2_unit_post_fire_shadow` / `l2_unit_post_assert` / `unit_post_shadow_fire_step`: **removed**.
- `tick_combat_with_fog` / `tick_turret_rotation` public signatures: **unchanged** (gated skips/dispatch internal).
- `SNAPSHOT_VERSION`: **unchanged at 17** unless Task 6's golden proves a real move.

## Sim Checklist

- [x] No new f32/f64.
- [x] No new hashed state (`FacingClass` already hashed).
- [x] No render/ui/sidebar/audio/net dependency.
- [x] Tick ordering: stays inside Phase 5; `object_ai_stage` untouched (invariant #3).
- [x] BTreeMap / live-order: fire dispatched inside the live-order Phase-2 loop; facing walks live order.
- [x] RNG: zero added; emission order preserved (smudge cursor unmoved).

## Risk Areas

- **`combat/mod.rs` Phase-2 loop is hash-critical (highest).** Dispatching Units to
  `unit_post` must call the SAME `resolve_attacker_fire` with the SAME snapshot at the SAME
  live-order position — any deviation reorders emission → smudge RNG cursor → desync.
  Mitigation: dispatch is a thin `match snap.category` in-loop; `smudge_emission_order_unchanged`
  + global parity golden.
- **Per-object facing toward the post-fire target.** Reading `emit.retarget_events`/`remove_attack`
  to pick the facing target must match what Phase 3 applies and what the legacy sweep faced.
  Mitigation: the post-retarget facing test + golden.
- **Despawn-mid-combat facing edge** (a Unit that fires and is killed same tick) — unverified by
  the shadow's pre-combat Unit set. Mitigation: golden gate.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 2 | **Unit fire dispatched at its live-LOGIC position in the single Phase-2 pass** | The post-combat `scenario_rng` smudge drain consumes `smudge_spawn_requests` in emission order; any reorder desyncs lockstep. | `smudge_emission_order_unchanged` + global parity golden unmoved. |
| 2 | **`unit_post` faces the post-fire target (retarget/remove honored)** | gamemd faces the re-acquired target; facing a stale target on a retarget tick is a visible turret-aim regression. | `unit_post_faces_reacquired_target_on_retarget_tick` + golden unmoved. |
| 3 | **`tick_turret_rotation` Unit retire is output-neutral** | id-order→live-order Unit facing must not change any barrel destination; Aircraft/Building turrets must stay bit-identical. | `turret_sweep_retired_for_scoped_units_no_drift`. |
| 5,6 | **state_hash unmoved (flip is output-neutral)** | The slice's premise; a moved golden = an unintended change to diagnose, not re-baseline. | global parity + replay goldens unmoved; `SNAPSHOT_VERSION` stays 17. |

---

## Tasks

### Task 1: `tick_turret_rotation` skips Units when authoritative (flag still `false`)

**Why:** Land the turret-sweep skip first, flag still `false`, so the build/tests are
unchanged and the authority-flip diff (Task 3) is small.

**Files:** Modify `src/sim/movement/turret.rs`. Re-Read `tick_turret_rotation` (`:82-149`)
before editing.

**Step 1:** In the Phase-1 read loop, after `entities.get(id)` and before `desired_turret_facing`:
```rust
// Unit turrets are driven per-object by unit_post once authoritative.
if crate::sim::world::unit_post::L2_UNIT_POST_AUTHORITATIVE
    && entity.category == crate::map::entities::EntityCategory::Unit
{
    continue;
}
```
**Step 2 — Verify.** Pre-flight `tasklist | grep -iE "cargo|rustc"`; `cargo check -p vera20k -q`
(exit 0). `cargo test -p vera20k combat` → **146 passed; 0 failed** (flag `false`, unchanged).
**Step 3 — Commit:** `git add src/sim/movement/turret.rs && git commit -m "sim/movement: gate Unit turret sweep behind L2_UNIT_POST_AUTHORITATIVE (still false) (L2 Task 3 prep)"`

---

### Task 2: Fill the authoritative `unit_post` body (Fire → post-fire Facing)

**Why:** Define the per-object behavior the dispatch will invoke. Flag still `false`, so this
is dead code until Task 3 — keep the build green.

**Files:** Modify `src/sim/world/unit_post.rs`; add a test in
`src/sim/combat/combat_turret_facing_tests.rs`. Re-Read `combat/mod.rs` `resolve_attacker_fire`
+ Phase-3 retarget apply (`:1613`), and `turret.rs` `facing_toward_lepton`/`body_facing_to_turret`.

**Step 1 — Fill `unit_post`.** It receives this Unit's already-built snapshot (Phase-1 built,
decremented) plus the disjoint store refs and `&mut emit`. It:
1. **FIRE** — `combat::resolve_attacker_fire(snap, &*entities, .., emit)` (immutable entities
   reborrow), emitting into `emit` exactly as the legacy loop would.
2. **FACING** — determine the post-fire target: if `emit.retarget_events` gained an entry for
   `snap.stable_id`, use that new `TargetKind::Entity(new)`; else if `emit.remove_attack`
   contains `snap.stable_id`, no target (body facing); else `snap.target`. Compute the desired
   16-bit facing toward that target (look up the entity target's lepton position via
   `facing_toward_lepton`, the cell-center for a `Cell` target, or `body_facing_to_turret(facing)`
   when no target), then `entities.get_mut(snap.stable_id)` → `barrel.set_rot(rot)` +
   `barrel.set(desired, binary_frame)` (skip if not turreted). `rot` is the unit's
   `turret_rot` from rules, same as `tick_turret_rotation` reads.

> **Reconcile against live code (flag for /review-plan):**
> - Borrow order inside `unit_post`: `resolve_attacker_fire` holds `&entities` for its call;
>   it returns before the `get_mut` for `barrel.set` — sequential, no conflict. Confirm against
>   the live `resolve_attacker_fire` signature.
> - The post-fire target read: `resolve_attacker_fire` pushes at most one retarget per attacker;
>   find this id's entry in `emit.retarget_events` (search from the end for this tick's push).
>   Confirm the retarget carries the new entity stable id.
> - Facing toward an *explicit* target (not `entity.attack_target`) needs the lepton lookup
>   inline (mirror `desired_turret_facing`'s entity/cell branches), since the entity's
>   `attack_target` is not yet updated (Phase-3 applies it). Keep it byte-identical to
>   `desired_turret_facing` for the no-retarget case.
> - `tick_ms == 0`: mirror legacy — the Phase-1 build already skips (no snapshot when combat
>   early-returns); `unit_post` is only called with a built snapshot, so no extra guard needed.

**Step 2 — Add `unit_post_faces_reacquired_target_on_retarget_tick`** to
`combat_turret_facing_tests.rs`: construct a turreted Unit whose current target is dead/removed
and a valid new target B in range; drive one authoritative tick (toggle the flag for the test
scope, or call `unit_post` directly on the scene) and assert the barrel destination points at B.

**Step 3 — Verify.** `cargo check -p vera20k -q` (exit 0; `unit_post` body live but unused while
flag `false`). `cargo test -p vera20k combat` → 146 + new test pass.
**Step 4 — Commit:** `git add src/sim/world/unit_post.rs src/sim/combat/combat_turret_facing_tests.rs && git commit -m "sim/world: implement authoritative unit_post Fire→Facing body + post-retarget facing test (L2 Task 3)"`

---

### Task 3: Flip the flag — dispatch Unit fire to `unit_post`; remove the shadow

**Why:** The authority transfer. Keep emission order by dispatching inside the live-order
Phase-2 loop; retire the now-redundant shadow.

**Files:** Modify `src/sim/combat/mod.rs` (Phase-2 dispatch), `src/sim/world/unit_post.rs`
(flag→`true`; delete shadow methods), `src/sim/world/mod.rs` (delete shadow wiring in Phase 5).
Re-Read the Phase-2 loop (`combat/mod.rs:1574-1589`) and the Phase-5 shadow block (added in
L2 Task 2) before editing.

**Step 1 — Phase-2 dispatch (`combat/mod.rs`).** Read `unit_post_authoritative` once at the top
of `tick_combat_with_fog`. In the Phase-2 loop, replace the unconditional `resolve_attacker_fire`
call with:
```rust
for snap in &snapshots {
    if unit_post_authoritative && snap.category == EntityCategory::Unit {
        crate::sim::world::unit_post::unit_post(snap, entities, occupancy, rules, interner,
            fog, overlay_grid, overlay_registry, terrain, binary_frame, tick_ms, &mut emit);
    } else {
        resolve_attacker_fire(snap, entities, rules, interner, fog, occupancy,
            overlay_grid, overlay_registry, terrain, binary_frame, tick_ms, &mut emit);
    }
}
```
(Adjust arg order/borrows to the live signatures. `entities` is `&mut EntityStore` here, so
`unit_post` can `get_mut` for the barrel after its internal immutable fire reborrow.)

**Step 2 — Flip `L2_UNIT_POST_AUTHORITATIVE` to `true`** in `unit_post.rs`.

**Step 3 — Remove the shadow.** Delete `l2_unit_post_fire_shadow`, `l2_unit_post_assert`,
`unit_post_shadow_fire_step` (`unit_post.rs`) and their `#[cfg(debug_assertions)]` call sites in
`world/mod.rs` Phase 5 (the pre-combat compute and post-sweep assert added in L2 Task 2).

**Step 4 — Verify.** `cargo check -p vera20k -q`; `cargo test -p vera20k combat` → 146(+new), 0 failed.
**Step 5 — Commit:** `git add src/sim/combat/mod.rs src/sim/world/unit_post.rs src/sim/world/mod.rs && git commit -m "sim: flip Unit fire+facing to authoritative unit_post; retire legacy Unit arm + shadow (L2 Task 3)"`

---

### Task 4: Emission-order + turret-retire parity tests

**Why:** Pin the invariants the flip rides on.

**Files:** Modify `src/sim/combat/combat_turret_facing_tests.rs`.

**Step 1 — `smudge_emission_order_unchanged`:** multi-Unit destruction scene; assert the
`scenario_rng` cursor after the combat smudge drain is identical to a captured baseline (or two
identical runs of the scene, flag on).
**Step 2 — `turret_sweep_retired_for_scoped_units_no_drift`:** mixed Unit + Aircraft turreted
scene; advance a tick; assert Aircraft barrel destinations are byte-identical to a sweep-only
baseline and each Unit destination equals the per-object facing.
**Step 3 — Verify:** `cargo test -p vera20k combat` all green.
**Step 4 — Commit:** `git commit -m "test(sim/combat): pin emission-order + turret-retire parity for L2 Task 3"`

---

### Task 5: Verify against gamemd (Fire→Facing order + the two YELLOW items)

**Why:** Confirm authoritative behavior matches gamemd; clear the verdict's YELLOW items
(non-blocking, cheap now).

**Verify:**
- **Fire→Facing order:** `UnitClass::AI @ 0x007360c0` calls `Fire_At_Target 0x007365e1` before
  `Facing_Update 0x007365e8` (spot-recheck the two call addresses live). `unit_post` fires then
  faces in the same per-object step — matches.
- **YELLOW 1 — vtable+0x3cc binding:** one `read_memory` of the UnitClass vtable slot → confirm
  `0x006fdd50`. Non-blocking.
- **YELLOW 2 — estimated-health write site:** confirm the `Fire_At` fire-time estimated-health
  write offset; record as a known UNMODELED fire-time target-selection write (unchanged by the flip).
- **Output:** append a short VERIFIED/UNMODELED note to the verdict doc if anything new is learned
  (doc is gitignored — do NOT `git add`).

---

### Task 6: Full-suite verification — golden MUST stay unmoved (separate bounded pass)

**Why:** Prove the flip is output-neutral. The slice's load-bearing gate.

**Step 1:** Pre-flight `tasklist | grep -iE "cargo|rustc"`; `cargo check -p vera20k -q` (exit 0).
**Step 2:** `cargo test -p vera20k combat` — read the literal `test result:` line.
**Step 3:** `cargo test -p vera20k` (full sim) — read the literal `test result:` lines. **Confirm
the global parity + replay/`state_hash` goldens are UNMOVED** (e.g.
`global_skirmish_replay_is_deterministic_and_baseline_stable`, `replay_hash_stable_through_slice6`,
`round_trip_preserves_state_hash`). `SNAPSHOT_VERSION` stays **17**.
**Step 4 — If any golden moved:** STOP. Diagnose (emission order? despawn-mid-combat facing edge?
the dispatch wrapper?), fix to neutral. Do **NOT** bump `SNAPSHOT_VERSION` / re-baseline unless you
can prove (cited gamemd evidence) the move is a real, intended correction — and surface it to the
user first; that is a scope change from this hash-neutral slice.

---

## Out of scope (explicitly deferred)

- Absorbing the Phase-1 snapshot build + cooldown decrement into `unit_post` (order-independent
  cleanup; the S4 walk's job).
- Retiring the sweeps for Infantry / Aircraft / Building (their slices).
- Wiring `unit_post` into the early `object_ai_stage` (future S4 relocation).
- S4 damage-particle RNG, HarvestBrain / Anim-Ammo / Spawn ordering, projectile-flight timing.
- MissionCom-authority / movement flips (the actually-blocked slices).

## Sources & References

- **Design doc:** `docs/plans/2026-06-02-ai-shell-unitclass-core-plan.md` Slice L2 §4 Task 3/2a,
  §5, §7, §8 (deps `:428-431` — "No dependency on S0–S2 scaffolding").
- **Gating verdict:** `docs/research/L2_FIRE_DAMAGE_TIMING_VERDICT_GHIDRA_REPORT.md`
  (deferred-projectile; keep batched P4/P6; YELLOW: vtable slot read, estimated-health write).
- **gamemd:** `UnitClass::AI @ 0x007360c0` (Fire `0x007365e1` → Facing `0x007365e8`),
  `UnitClass::Fire_At_Target @ 0x00736df0` (no inline re-acquire — `decompile_function 0x00736df0`
  this session), `TechnoClass::Fire_At @ 0x006fdd50`, `BulletClass::BulletDetonation @ 0x00468d80`.
  `GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT_GHIDRA_REPORT.md` §4,
  `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` §7.
- **Live code (re-verified this session):** `combat/mod.rs` (`resolve_attacker_fire` `:1772`,
  Phase-2 loop `:1574-1589`, snapshot live-order sort `:1562`, cooldown decrement `:1421-1422`,
  `fire_blocked` `:1248`, Phase-3 retarget `:1613`, `handle_entity_deaths` `:812` sets
  `dying`/`despawned_ids` — no `entities.remove`), `turret.rs` (`tick_turret_rotation` `:82`,
  `desired_turret_facing`, `facing_toward_lepton`, `body_facing_to_turret`, `barrel.set_rot`/`set`
  `:168-169`), `world/unit_post.rs` (shadow + `L2_UNIT_POST_AUTHORITATIVE`), `world/mod.rs` Phase 5
  (combat, turret sweep, shadow wiring), `snapshot.rs:24` (`SNAPSHOT_VERSION=17`).
- **Prior commits (this session, `dev`):** `a592bfb6`, `ebafb93a`, `443bdb4b`, `9e6cb7d3`
  (L2 Task 2 shadow — the proven-equivalent per-object path this slice promotes).
