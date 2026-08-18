# Slice S3 — UnitClass post-Foot ordering Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained, ends green
> (`cargo test -p vera20k`), and commits on `dev`. Tasks 1–3 are hash-neutral; Tasks 4–5 are
> the two hash-affecting flips (each re-baselines the golden in-task with a documented reason);
> Task 6 bumps `SNAPSHOT_VERSION`. If a supposedly-neutral task moves the golden, STOP and
> diagnose — do not re-baseline.

**Goal:** Gamemd-faithful kill-tick Fire→Facing coupling (barrel holds the dying target's
facing on the kill tick) + idle Units hash mission `Guard(5)` instead of `None` + the named
post-Foot slot scaffold.

**Architecture:** Read point of Unit barrel destinations moves into the per-object combat
window (pre-death batch); write point stays at the existing post-batch site. `derived_mission`
fall-through becomes category-gated. No phase moves, no new deps, `match category` only.

**Design Doc:** `docs/plans/2026-06-10-s3-unit-postfoot-ordering-design.md` (v2, review-resolved)

---

## Grounding Summary

- **Order + kill-tick evidence:** `UNITCLASS_GHIDRA_REPORT.md` §3/§10 (post-Foot order, steps
  3m–3t); `GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT_GHIDRA_REPORT.md` §4 (asm `0x007365E1` fire →
  `0x007365E8` facing; re-confirmed live in the L2-T3 session); `L2_FIRE_DAMAGE_TIMING_VERDICT`
  (munition-deferred damage, `0x006fdd50` → `0x00468d80`); LogicClass re-read contract
  (`0x0055AFB0`); PointerExpired clears TarCom at death (`DETACH_FROM_ALL_LISTS_*`).
- **Idle mission:** Guard=5/Move=2 (`FOOTCLASS_MISSION_MOVE` §3.3/§6; `[Guard] Rate=.030`);
  arrival tick stays Move, →Guard after the post-arrival dispatch (S2 design, decompile
  `0x004D4200` 2026-06-10); passive-acquire gate {2,10,5} (`decompile 0x006F9E50`).
- **Code state (verified this session):** kill-tick clears at `combat/mod.rs:1654-1661` (P5
  finished-attacker clear) and `:985→:1128` (P6 dead-target sweep) run before the facing pass
  at `world/mod.rs:2287`; `CombatEmit` `:1182` carries `retarget_events`/`remove_attack`;
  acquire sites `:1873/:1953/:1970` push-and-return (no fire that resolution);
  `FacingClass::set/current` pure in `(state, binary_frame)` (`facing_class.rs:85/123`);
  no Unit-barrel writer between the P2 window and the apply site (spawn paths = other phases;
  `turret.rs:149` sweep skips Units). `derived_mission` fall-through `(None,0)` at
  `game_entity.rs:558`. `SNAPSHOT_VERSION = 20` (`snapshot.rs:41`, self-test `:392`). Golden:
  `GLOBAL_HARNESS_FINAL_HASH` (`global_parity_harness_tests.rs:44`, test `:136`).
- **INI:** no new keys. ROT continues to come from `turret_rot` at apply (unchanged).
- **Unknown after grounding:** exact ground-unit idle→Guard assigner identity (G2, YELLOW —
  value multiply corroborated); §3o/§3q/§3r slot internals (named markers only).

## Key Technical Decisions

- **Compute-per-object / apply-at-site** (read moves, write stays): equivalence rests on
  `FacingClass` binary-frame purity + the no-intervening-writes audit — **high**;
  source: `facing_class.rs:85-163`, writer grep this session.
- **Residual pass iterates `keys_sorted()`** (not live order): coverage must stay bit-identical
  to today's `tick_unit_facing` (which includes limbo/in-transport/dying Units) so the only
  output delta is the per-object read window — **high**; source: `unit_post.rs:46-60`.
- **Own-retarget → aim new target same tick; own-remove → body same tick** — matches both
  today's output and gamemd (upstream same-pass validation/acquisition precedes fire/facing) —
  **high** for remove (today-equivalent), **medium** for retarget-aim timing (gamemd analog is
  the upstream acquisition; flagged for /review-plan).
- **D2 uniform across dying** (no dying gate in the tail projection — value change only, no
  new refresh logic): **medium**, G5 pinned by test; corpse-mission freeze deferred to the
  deferred-delete substrate.
- **One `SNAPSHOT_VERSION` bump (20→21) after both flips** — mirrors S2 (flip first, bump in
  its own task) — **high**; source: S2 commits `91cd1f61`/`af87002d`.

## Open Questions

### Resolved During Planning
- *Where do same-tick attack-target clears happen?* `combat/mod.rs:1654-1661` (P5) and
  `:985` (P6) — both inside `tick_combat_with_fog`, before today's facing pass.
- *Does the snapshot set cover all target-holders?* Yes except in-transport (`:1424`) — those
  fall to the residual pass.
- *Is `DispatchSlot::Guard` live?* Yes (`dispatch.rs:24/56`) — D2 cannot route to `Skip`.

### Deferred to Implementation
- Whether the committed golden scenario contains Unit kill ticks (D1 may leave the baseline
  unshifted, like S2) — observed when Task 4 runs the harness.
- `tick_ms == 0` early-return now also skips facing application (degenerate non-production
  path; today's pass was idempotent there) — accepted, noted in the flip commit.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/combat/mod.rs` | `unit_facing` emit + per-attacker/residual compute (P2 window) |
| Modify | `src/sim/world/unit_post.rs` | `apply_unit_facing` (write point); retire compute phase; slot-scaffold doc |
| Modify | `src/sim/world/mod.rs` | apply call site consumes `combat_result.unit_facing` |
| Modify | `src/sim/game_entity.rs` | D2 Guard fall-through + test updates |
| Modify | `src/sim/world/techno_ai.rs` | S2 arrival-test expectation None→Guard |
| Modify | `src/sim/mission/dispatch.rs` | stale comment (idle fall-through) |
| Modify | `src/sim/snapshot.rs` | `SNAPSHOT_VERSION` 20→21 |
| Modify | `src/sim/world/global_parity_harness_tests.rs` | golden re-baseline (documented) |
| Modify | `src/sim/combat/combat_turret_facing_tests.rs` | new S3 facing tests |

## Interface Changes

- `CombatEmit` + `CombatTickResult` gain `unit_facing: Vec<(u64, u16)>` (crate-internal;
  consumers: `world/mod.rs` Phase 5 only).
- `unit_post::tick_unit_facing` removed; `unit_post::apply_unit_facing` added (called from
  `world/mod.rs`; same math).
- `GameEntity::derived_mission` Unit fall-through value changes (consumers verified:
  tail projection, S2 in-loop commit [movement_target ⇒ Move, unaffected], host record pass,
  miner seam assert [miner ⇒ Harvest, unaffected]).

## Sim Checklist

- [x] No f32/f64 — facing math is existing integer/FacingClass code.
- [x] No new hashed state — `unit_facing` is transient emit (never stored/serialized/hashed);
      `barrel_facing` and `mission.current` were already hashed.
- [x] No render/ui/sidebar/audio/net deps.
- [x] Tick ordering unchanged (read point moves within Phase 5; write point fixed).
- [x] Determinism: per-attacker order = live order (existing); residual = `keys_sorted()`;
      dedup via sorted Vec + binary_search.

## Risk Areas

- Borrow structure in the P2 loop (immutable reborrow after `resolve_attacker_fire`) — the
  loop already releases `&mut` between iterations; compiler is the oracle.
- The S2 suite (`techno_ai.rs` tests, `s2_tick_counter_increments_exactly_once`,
  `save_load_round_trip_on_arrival_tick`) must stay green through D2 — they exercise the tail
  projection paths D2 touches.
- Golden moves twice (T4 possibly, T5 certainly) — each re-baseline documented in-commit.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 2/4 | Kill-tick barrel holds the dying target's facing; idle-return starts next tick | Fires on **every kill**; turret snap-back timing is player-visible | Composition of `0x006fdd50→0x00468d80`, `0x0055AFB0` re-read, PointerExpired; test `kill_tick_barrel_holds_target_facing` |
| 2 | Own-remove → body facing same tick | Matches today + gamemd upstream-validation timing | Test `removed_attacker_returns_to_body_same_tick` |
| 2 | Own-retarget → aim new target same tick | gamemd upstream acquisition aims same pass | Test `retargeted_attacker_aims_new_target_same_tick` |
| 4 | Fire keeps reading last-tick facing (P1 snapshot) | "No same-tick rotate-and-fire" (`0x007365E1→E8`) | Existing fire-gate tests stay green |
| 5 | Idle Unit mission = Guard(5), substate 0; arrival tick stays Move | Hashed selector must match gamemd's idle value; S4 passive-acquire gate {2,10,5} depends on it | Tests `idle_unit_derives_guard`, updated arrival test |
| 5 | In-transport passengers stay `None` (named placeholder) | Do-not-invent: gamemd passenger mission untraced | Test `passenger_derive_unchanged_placeholder` |
| 4 | Non-Unit categories bit-identical | Aircraft/Building barrels stay on the legacy sweep until S7/S8 | Test `non_unit_facing_unchanged` + golden determinism |

---

## Tasks

### Task 1: `unit_facing` plumbing (types only, hash-neutral)

**Why:** Land the emit/result field both flips ride on; provably inert.

**Files:** Modify `src/sim/combat/mod.rs` (`CombatEmit` `:1182`; `CombatTickResult` `:724`;
both constructors `:1231` early-return and `:1703` tail).

**Step 1:** Add to `CombatEmit`:
```rust
    /// (unit_id, desired 16-bit barrel destination) — computed in the Phase-2
    /// per-object window (pre-death state; own-retarget visible), applied
    /// post-batch by `unit_post::apply_unit_facing`. Transient — never stored,
    /// serialized, or hashed.
    pub(crate) unit_facing: Vec<(u64, u16)>,
```
**Step 2:** Add `pub unit_facing: Vec<(u64, u16)>,` to `CombatTickResult`; populate
`Vec::new()` in the `tick_ms == 0` early return and `unit_facing` (from the destructure) in
the tail constructor. Add `unit_facing,` to the `let CombatEmit { ... } = emit;` destructure.

**Step 3:** `cargo test -p vera20k combat` → expect all green (field unused).

**Step 4:** Commit: `sim/combat: S3 T1 — unit_facing emit/result plumbing (inert, hash-neutral)`

### Task 2: Per-attacker + residual facing compute (emitted, not yet consumed)

**Why:** The per-object read window — fills `unit_facing` while `tick_unit_facing` stays
authoritative, so this is hash-neutral and testable in isolation.

**Files:** Modify `src/sim/combat/mod.rs` (P2 loop `:1523-1547` and just before the
destructure `:1549`).

**Step 1:** In the P2 loop, wrap the existing call:
```rust
    for snap in &snapshots {
        let n_retarget = emit.retarget_events.len();
        let n_remove = emit.remove_attack.len();
        resolve_attacker_fire(/* existing args unchanged */);
        // S3: per-object barrel destination for Unit attackers, read in the
        // per-object window — deaths/clears (Phases 3-6) are not yet applied,
        // so a unit whose target dies this tick still aims at it (idle-return
        // begins next tick); a unit whose own resolution retargeted aims at
        // the new target now; one whose own resolution cleared returns to body.
        let Some(e) = entities
            .get(snap.stable_id)
            .filter(|e| e.category == EntityCategory::Unit && e.barrel_facing.is_some())
        else {
            continue;
        };
        let own_retarget = emit.retarget_events[n_retarget..]
            .iter()
            .find(|&&(aid, _)| aid == snap.stable_id)
            .map(|&(_, tid)| tid);
        let own_removed = emit.remove_attack[n_remove..].contains(&snap.stable_id);
        let desired: u16 = if let Some(tid) = own_retarget {
            match entities.get(tid) {
                Some(t) => crate::sim::movement::turret::facing_toward_lepton(
                    e.position.rx, e.position.ry, e.position.sub_x, e.position.sub_y,
                    t.position.rx, t.position.ry, t.position.sub_x, t.position.sub_y,
                ),
                None => crate::sim::movement::turret::body_facing_to_turret(e.facing),
            }
        } else if own_removed {
            crate::sim::movement::turret::body_facing_to_turret(e.facing)
        } else {
            match crate::sim::movement::turret::desired_turret_facing(e, entities) {
                Some(d) => d,
                None => continue, // unreachable: barrel_facing checked above
            }
        };
        emit.unit_facing.push((snap.stable_id, desired));
    }
```
(If the loop body is not a plain call, insert the `n_retarget`/`n_remove` captures
immediately before the call and the block immediately after it; `continue` semantics match
the loop.)

**Step 2:** Residual pass, inserted AFTER the loop and BEFORE `let CombatEmit { ... } = emit;`
— placement is semantic (Phases 3-6 clear `attack_target`; the read must be pre-death):
```rust
    // S3 residual: every Unit not in the attacker snapshot set (target-less,
    // or in-transport holders excluded at the snapshot build). Iterates the
    // SAME keys_sorted() coverage the legacy tick_unit_facing pass had —
    // including limbo/dying Units — so the only output delta vs. the legacy
    // pass is the pre-death read window. Per-entity independent → id order OK.
    let mut computed: Vec<u64> = emit.unit_facing.iter().map(|&(id, _)| id).collect();
    computed.sort_unstable();
    for &id in &keys {
        if computed.binary_search(&id).is_ok() {
            continue;
        }
        let Some(e) = entities.get(id) else { continue };
        if e.category != EntityCategory::Unit {
            continue;
        }
        let Some(desired) = crate::sim::movement::turret::desired_turret_facing(e, entities)
        else {
            continue;
        };
        emit.unit_facing.push((id, desired));
    }
```
(`keys` is the existing `keys_sorted()` vec from the P1 build; reuse it.)

**Step 3:** Tests in `src/sim/combat/combat_turret_facing_tests.rs` (follow the file's
existing fixture helpers — `FacingClass::new`, `body_facing_to_turret`):
- `s3_unit_facing_emitted_for_attacker_and_idle` — one attacker Unit + one idle turreted
  Unit; run `tick_combat_with_fog`; assert `result.unit_facing` contains both ids, attacker's
  destination = toward target, idle's = body.
- `removed_attacker_returns_to_body_same_tick` — attacker whose resolution removes the
  attack (e.g. no weapon selectable for the target) → destination = body facing.
- `retargeted_attacker_aims_new_target_same_tick` — attacker whose target is friendly-flipped
  or dead-at-resolve (hp 0 fixture) with a hostile in range → destination = toward the new
  target id from `retarget_events`.
- `kill_tick_unit_facing_holds_target` — attacker kills target this tick (lethal damage);
  assert `unit_facing` destination = toward the (now dead) target, while the entity's
  `attack_target` was cleared by the batch (proves the pre-death read).

**Step 4:** `cargo test -p vera20k combat` green; `cargo test -p vera20k global_parity` green
(unconsumed emit must not move the golden).

**Step 5:** Commit: `sim/combat: S3 T2 — per-object Unit facing destinations computed in the
P2 window (emitted, unconsumed; hash-neutral)`

### Task 3: `apply_unit_facing` extraction (pure refactor)

**Why:** Separate the write point so the flip swaps only the destination source.

**Files:** Modify `src/sim/world/unit_post.rs`.

**Step 1:** Add:
```rust
/// Apply precomputed Unit barrel destinations (the write half of the post-Foot
/// Facing slot). `FacingClass::set` is pure in `(state, binary_frame)` and no
/// system writes Unit barrels between the Phase-2 read window and this site,
/// so the apply point within Phase 5 does not affect the resulting state.
pub(crate) fn apply_unit_facing(
    entities: &mut EntityStore,
    updates: &[(u64, u16)],
    rules: &RuleSet,
    interner: &StringInterner,
    binary_frame: u32,
) {
    for &(id, desired) in updates {
        let rot_byte: u8 = rules
            .object(interner.resolve(entities.get(id).map(|e| e.type_ref).unwrap_or_default()))
            .map(|obj| obj.turret_rot.clamp(0, 0xFF) as u8)
            .unwrap_or(5);
        if let Some(entity) = entities.get_mut(id) {
            if let Some(ref mut barrel) = entity.barrel_facing {
                barrel.set_rot(rot_byte);
                barrel.set(desired, binary_frame);
            }
        }
    }
}
```
**Step 2:** Rewrite `tick_unit_facing` to: compute `updates` exactly as today (Phase-1 body
unchanged), then `apply_unit_facing(entities, &updates, rules, interner, binary_frame)`.
Delete the now-duplicated Phase-2 body.

**Step 3:** `cargo test -p vera20k` green; golden unmoved (pure refactor).

**Step 4:** Commit: `sim: S3 T3 — extract unit_post::apply_unit_facing write half (pure
refactor, hash-neutral)`

### Task 4: FLIP — facing destinations come from the P2 window

**Why:** D1. The only output change is the per-object pre-death read (kill/retarget ticks).

**Files:** Modify `src/sim/world/mod.rs:2284-2292`, `src/sim/world/unit_post.rs`.

**Step 1:** Replace the `tick_unit_facing` call with:
```rust
            // S3: Unit barrel destinations were computed per-object in the
            // combat Phase-2 window (pre-death state — a unit whose target died
            // this tick keeps aiming at it this tick, idle-return next tick);
            // this is the unchanged write point.
            crate::sim::world::unit_post::apply_unit_facing(
                &mut self.substrate.entities,
                &combat_result.unit_facing,
                rules,
                &self.interner,
                self.binary_frame,
            );
```
**Step 2:** Delete `tick_unit_facing` from `unit_post.rs` (its compute half is now dead);
update the module doc (write half stays here; read half lives in the combat P2 window).

**Step 3:** Sim-level tests (place beside the existing S2 tests in `techno_ai.rs` or in
`combat_turret_facing_tests.rs`, whichever already has full-`advance_tick` fixtures):
- `kill_tick_barrel_holds_target_facing` — full `advance_tick`: A kills T on tick N → A's
  `barrel_facing` destination on tick N is toward T; on tick N+1 it is `body<<8`.
- `co_attacker_facing_matches_killer` — A and B both target T; T dies to A on tick N → B's
  destination on tick N is toward T.
- `facing_apply_point_equivalence_no_kill` — a no-death, no-retarget combat scenario: per-tick
  `barrel_facing` (prev/dest/start/duration) sequence identical to a pre-flip capture (assert
  against values captured by running the same fixture before Step 1 — hardcode the expected
  sequence in the test with a comment naming the capture commit).
- `non_unit_facing_unchanged` — Aircraft/Building barrel state sequences identical pre/post
  flip (fixture with a turreted building and an aircraft attacker).

**Step 4:** `cargo test -p vera20k` → the global harness may shift ONLY if the committed
scenario contains Unit kill/retarget ticks. If unshifted: document on
`GLOBAL_HARNESS_FINAL_HASH` ("S3 facing flip verified unshifted — no Unit kill ticks in the
committed scenario"). If shifted: re-baseline once with the reason ("S3: per-object pre-death
facing read — kill-tick barrel hold, gamemd-cited") and verify determinism (replay == record)
still passes.

**Step 5:** Commit: `sim: S3 T4 — flip Unit facing to per-object P2-window destinations
(kill-tick barrel hold; golden <unshifted|re-baselined with reason>)`

### Task 5: FLIP — idle Units derive Guard(5)

**Why:** D2. gamemd has no `None` mission; idle ground vehicles sit in Guard. Unblocks S4's
{2,10,5} passive-acquire gate.

**Files:** Modify `src/sim/game_entity.rs:555-558`, `src/sim/world/techno_ai.rs:975-992`,
`src/sim/game_entity.rs:1079-1082`, `src/sim/mission/dispatch.rs:45` (comment).

**Step 1:** In `derived_mission`, replace the final fall-through:
```rust
        if self.movement_target.is_some() {
            return (MissionType::Move, 0);
        }
        // gamemd has no "None" mission: an idle ground vehicle sits in Guard(5)
        // (dispatched at [Guard] Rate; the passive-acquire gate covers missions
        // {Move, Harvest, Guard} only). Units only this slice — infantry (S6),
        // aircraft (S7, already mapped via aircraft_mission), buildings (S8).
        // In-transport passengers keep the legacy placeholder until the
        // enter-transport mission commit is traced from the binary.
        if self.category == EntityCategory::Unit && !self.passenger_role.is_inside_transport() {
            return (MissionType::Guard, 0);
        }
        (MissionType::None, 0)
```
**Step 2:** Update tests:
- `derived_mission_idle_when_no_machine_active` (`game_entity.rs:1079`): the default fixture
  is a Unit → expect `(MissionType::Guard, 0)`; add a Structure-category assertion expecting
  `(MissionType::None, 0)`.
- `arrival_tick_mission_is_move_not_sleep` (`techno_ai.rs:977-992`): post-arrival tick now
  expects `MissionType::Guard` (comment: "idle→Guard landed in S3; arrival tick still hashes
  dispatch-time Move").
- New `idle_unit_derives_guard` — machine-less Unit → `(Guard, 0)`; same entity as Infantry
  category → `(None, 0)`.
- New `passenger_derive_unchanged_placeholder` — in-transport Unit → `(None, 0)` with the
  do-not-invent comment.
- New `dying_unit_projection_uniform` (G5 pin) — a dying machine-less Unit projects Guard via
  `refresh_mission_shadow` (uniform treatment; corpse-mission freeze deferred to the
  deferred-delete substrate — 1-tick window).
**Step 3:** Fix stale comments: `dispatch.rs:45` (None → "non-Unit idle fall-through");
any S2 comment near `movement_tick.rs:1047` describing the idle derivation.

**Step 4:** `cargo test -p vera20k` → S2 suite (`s2_tick_counter_increments_exactly_once`,
`save_load_round_trip_on_arrival_tick`, dispatch shadow asserts) must stay green. The global
harness WILL shift (idle Units exist in the scenario) → re-baseline once: "S3: idle Unit
mission Guard(5) — gamemd-faithful idle selector (no None mission in gamemd)". Verify
determinism (replay == record).

**Step 5:** Commit: `sim: S3 T5 — machine-less idle Units derive Guard(5) (gamemd-faithful
idle selector; golden re-baselined with reason)`

### Task 6: `SNAPSHOT_VERSION` 20→21

**Why:** Two hashed-representation changes landed (barrel kill-tick values, mission idle
value); saves from ≤20 are not hash-compatible.

**Files:** Modify `src/sim/snapshot.rs:41` (+ self-test `:392`).

**Step 1:** `const SNAPSHOT_VERSION: u32 = 21;` with a one-line comment
(`// 21: S3 — per-object pre-death facing read + idle Unit Guard(5)`); update the `:392`
assertion to 21.

**Step 2:** `cargo test -p vera20k snapshot` green; the S2 round-trip test
(`save_load_round_trip_on_arrival_tick`) green.

**Step 3:** Commit: `sim: S3 T6 — SNAPSHOT_VERSION 20->21 (post-Foot facing + idle-Guard authority)`

### Task 7: Full verification + slot scaffold doc

**Why:** End-to-end gate + the named post-Foot slot order for later slices.

**Step 1:** `unit_post.rs` module doc gains the slot scaffold:
```
//! Post-Foot UnitClass slot order (gamemd `UnitClass::AI` steps 3m–3r):
//!   1. Fire        — per-attacker in combat Phase 2, live order        [LANDED L2/S3]
//!   2. Facing      — destinations read in the P2 window, applied here  [LANDED S3]
//!   3. GuardTerrain— Guard(5) + invalid terrain + sight → self-destroy [SLOT — UNCHECKED, needs RE]
//!   4. HarvestBrain— idle Harvester/Weeder → Harvest decision          [SLOT — miner substrate owns]
//!   5. Anim/Ammo   — vtable+0x424 wrapper                              [SLOT — target unresolved, needs RE]
//!   6. SpawnManager— Carrier/Dreadnought spawn dispatch                [SLOT — feature absent, named gap]
//! Pre-fire TurretAI idle scan + AI auto-hunt/stuck-rescue: S4 / AI-deferred.
```
**Step 2:** Run the full suite once: `cargo test -p vera20k` — read the literal
`test result:` lines; every binary green. Run clippy: `cargo clippy -p vera20k` — no new
warnings in touched files.

**Step 3:** Update the design doc status header to IMPLEMENTED with the commit list; note
golden outcomes (T4 shifted or not, T5 reason line).

**Step 4:** Commit: `sim: S3 T7 — post-Foot slot scaffold doc + full-suite verification`

## Sources & References

- **Design doc:** `docs/plans/2026-06-10-s3-unit-postfoot-ordering-design.md` (v2)
- **Ghidra reports:** `UNITCLASS_GHIDRA_REPORT.md` §3/§10/§11;
  `GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT_GHIDRA_REPORT.md`;
  `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` §6/§7/§9;
  `L2_FIRE_DAMAGE_TIMING_VERDICT_GHIDRA_REPORT.md`; `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`
  §3/§6; `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §5; `NAVCOM_ONARRIVAL_TAIL_HOOKS_GHIDRA_REPORT.md`;
  `S2_MISSION_DISPATCH_VS_PASSIVE_ACQUIRE_ORDERING.md`; `DETACH_FROM_ALL_LISTS_*`
- **Addresses (docs-side only):** `0x007360C0` (UnitClass::AI), `0x00736DF0`/`0x00736990`
  (fire/facing), `0x007365E1→0x007365E8` (order), `0x006fdd50→0x00468d80` (deferred damage),
  `0x0055AFB0` (logic loop), `0x004D4200` (Mission_Move), `0x006F9E50` (AI_Update/passive gate)
- **INI:** `rulesmd.ini` `[Guard] Rate=.030`, `[Move] Rate=.016` (context; not newly parsed)
- **Related code:** `src/sim/combat/mod.rs`, `src/sim/world/unit_post.rs`,
  `src/sim/movement/turret.rs`, `src/sim/movement/facing_class.rs`, `src/sim/game_entity.rs`,
  `src/sim/world/techno_ai.rs`, `src/sim/snapshot.rs`,
  `src/sim/world/global_parity_harness_tests.rs`
- **Prior commits:** S2 `32f9ef36..7b79a186`; L2 flip (see `2026-06-04-l2-task3-unit-post-flip-plan.md`)
