# Unit Mission_Dispatch Host (shadow) Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Fill the no-op `EntityCategory::Unit` arm of the object-AI shell with one
per-object Mission_Dispatch router (read-only, hash-neutral) that routes each live Unit by
its fresh-at-host-time mission, proven (debug-only) against the scattered legacy dispatch —
the prerequisite that gates the hash-affecting S2 flip.

**Architecture:** A pure router in a new `src/sim/mission/dispatch.rs` (mirrors the
`mission/verb.rs` pure-function pattern) plus a debug-only shadow pass + end-of-tick proof
in `src/sim/world/techno_ai.rs` (mirrors the landed S1 `unit_ai_shadow_step` /
`debug_assert_s1_shadow`). No handler bodies move; the host mutates no hashed state.

**Design Doc:** [docs/plans/2026-06-04-unit-mission-dispatch-host-design.md](2026-06-04-unit-mission-dispatch-host-design.md)

---

## Grounding Summary

- **Docs:** `TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §3(e) (the 32-case Mission
  switch→slot table, `decompile 0x005B3060`), §2.2 (dispatch gate order IsActive→timer→
  Health), §7.2/§9 S0–S2 ladder; `TECHNOCLASS_AI_MIGRATION_BOUNDARY_GHIDRA_REPORT.md`
  (Unit→Foot→AI_Update→Mission_Dispatch→locomotor order, "verified"). These establish the
  mission grouping and the dispatch-before-locomotor ordering S2 needs.
- **Current Rust (re-verified this session, post-P5a `dc7a34d9`):** `object_ai_stage` runs
  at the TOP of `advance_tick` (mod.rs:2024, after commands 2001 + flush 2010, before ground
  movement); `EntityCategory::Unit => {}` is the no-op arm (techno_ai.rs:109); `MissionCom`
  is hashed (world_hash.rs:656, Slice 8) but `current`/`substate` are a tail projection of
  the legacy machines via `derived_mission()` (game_entity.rs:523) written by
  `refresh_mission_shadow` (mod.rs:910, called at 2671); the S1 proof runs at mod.rs:2683.
  The miner session's `harvest_mission_step` (miner/harvest_mission.rs:46) owns Harvest.
- **Repo pattern mirrored:** S1's parallel debug shadow (`unit_ai_shadow_step` +
  `debug_assert_s1_shadow`, techno_ai.rs:166/197) and the L5 surface-divergence-never-
  equalize discipline (harvest_mission.rs:54-74). The router mirrors `mission/verb.rs`.
- **INI keys:** none — this slice routes existing in-memory mission state; no new INI parse.
- **Resolved during /review-plan (binary `0x005B3060` decompiled):** the switch routes QMove(3)
  AND AttackMove(29) BOTH via `default` to the Sleep handler (`+0x204`) WITH a timer rewrite —
  there is no dispatcher-side skip for AttackMove. The source-doc claim "AttackMove falls off
  the switch, no dispatch, no timer rewrite" is WRONG; the real invariant is that 29 is never a
  committed CurrentMission (assign-side prevents it). The router models this with a defensive
  `Skip` (never reached). Source doc §2.7/§3(e)/§7.6 flagged for correction (Task 6).
- **Unknown after grounding:** churn-divergence magnitude (host-time vs tail-time mission) is
  unknown until measured — it is the slice's S2 go/no-go metric.

## Key Technical Decisions

- **Host routes by `derived_mission()` evaluated FRESH at host time, not stale
  `mission.current`.** gamemd dispatches by post-command CurrentMission; at host time
  (mod.rs:2024) `mission.current` excludes this tick's commands (projected only at the 2671
  tail). **Confidence:** high — **Source:** design review [P1]; mod.rs:2001/2024/2671;
  doc §2.2.
- **Router shipped at FAMILY granularity (`unit_dispatch_family`), full per-slot table
  deferred to S5.** This slice is Unit-only; the parity-relevant content is the Unit
  grouping (Move/Attack/Enter/Harvest/Guard, QMove→Sleep, AttackMove→no-dispatch). Encoding
  gamemd's raw vtable offsets (`+0x204…`) as literals in sim code is avoided (engine-internal
  with no Rust-port meaning). **Confidence:** high — **Source:** doc §3(e) grouping; design
  review [P2] (narrow-the-claim is a listed valid resolution); refines the design doc's
  `dispatch_slot_offset → Option<u16>` to a family enum.
- **The shadow is a parallel debug pass, not code inside the `Unit => {}` match arm** — same
  shape as the landed S1 shadow (which also leaves `Unit => {}` a no-op and runs
  `unit_ai_shadow_step` alongside). **Confidence:** high — **Source:** techno_ai.rs:109/166.
- **`object_ai_stage` returns the host-time dispatch trace; release builds never allocate**
  (records are pushed only under `cfg(any(test, debug_assertions))`; `Vec::new()` is lazy, so
  an all-skip release walk never allocates). **Confidence:** high — **Source:** hot-path
  no-alloc rule; existing `object_ai_walk` returns a debug-only `Vec`.
- **Iteration set = `live_object_order_snapshot()` (LogicVector).** Gamemd-correct dispatch
  set; the proof surfaces any legacy-touched Unit outside it as drift. **Confidence:** high —
  **Source:** doc §7.2; Q2 user decision.

## Open Questions

### Resolved During Planning

- *Where does the host-time record live — match arm or parallel pass?* → Parallel debug pass,
  mirroring S1 (the `Unit => {}` arm stays a no-op, exactly as it is after S1 shipped).
- *u16 offset table vs named slots vs family?* → Family granularity this slice (Unit-only);
  full per-slot table is S5. Avoids vtable-offset literals in sim code.
- *Does the agreement proof need a host-time trace?* → Yes, for the churn metric (host-time
  vs tail-time family). The structural asserts (reachable→live, AttackMove-unreachable,
  skip rules, live-set coverage) are end-of-tick properties and need no trace, but the
  churn metric — the slice's S2 signal — does.
- *Switch structure for QMove(3) vs AttackMove(29)?* → **Resolved in /review-plan** via
  `decompile 0x005B3060`: both hit `default` → Sleep `+0x204` + timer rewrite. AttackMove is
  not a dispatcher skip; it is simply never committed. Router uses a defensive `Skip`.

### Deferred to Implementation

- The churn-divergence count's expected magnitude (how often a Unit's mission changes between
  host-time 2024 and tail 2671) is unknown until measured in a replay; it is reported, not
  asserted-to-zero.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sim/mission/dispatch.rs` | Pure router: `DispatchSlot` + `unit_dispatch_family(MissionType) -> DispatchSlot` (total over 32 + None) |
| Modify | `src/sim/mission/mod.rs` | `pub mod dispatch;` |
| Modify | `src/sim/world/techno_ai.rs` | `UnitDispatchRecord`; debug-only `unit_dispatch_record_pass`; `object_ai_stage` returns the trace; `debug_assert_unit_dispatch_shadow(&self, &trace)` proof |
| Modify | `src/sim/world/mod.rs` | Bind the trace at the `object_ai_stage()` call (2024); call the new proof after `debug_assert_s1_shadow` (2683) |

## Interface Changes

- **`Simulation::object_ai_stage`** return type changes `() -> Vec<UnitDispatchRecord>`
  (alias `UnitDispatchTrace`). Callers: `advance_tick` (mod.rs:2024) binds it; the 3 test
  call sites in techno_ai.rs (`sim.object_ai_stage();`) drop the return (Vec is not
  `must_use`) — no edit required there.
- **New pub(crate) items** in `mission::dispatch`: `enum DispatchSlot`,
  `fn unit_dispatch_family`. No existing API changes.
- No public-facing (crate-external) interface changes.

## Sim Checklist

- [x] All math fixed-point — N/A (no arithmetic; enum routing only).
- [x] New state in deterministic hash — **none added**; the trace is debug-only, never
  serialized, never hashed (hash-neutral is an acceptance test).
- [x] No dependencies on render/ui/sidebar/audio/net — `mission/dispatch.rs` imports only
  `MissionType`/`EntityCategory`; the shadow reads `Simulation` sim-state only.
- [x] Tick ordering impact — **none**; no phase added or reordered. The proof slots in
  beside the existing end-of-tick `debug_assert_s1_shadow`.
- [x] BTreeMap iteration order — the host uses `live_object_order_snapshot()` (LogicVector),
  not BTreeMap order; deliberate (gamemd dispatch set).

## Risk Areas

- **Tautological proof (the trap design-review [P1] caught):** if the proof compared
  `mission.current` to `derived_mission()` at end-of-tick it would be vacuous (equal by
  construction after the 2671 refresh). Mitigated: the proof compares the **host-time
  recorded** family (captured at 2024) to a **fresh tail re-derivation** (churn), and runs
  structural asserts that don't depend on the tautology.
- **Release hot-path allocation:** the trace must not allocate in release. Mitigated: records
  pushed only under `cfg(any(test, debug_assertions))`; lazy `Vec::new()`.
- **Line-anchor drift (parallel sessions):** P5a already moved anchors this session. Re-grep
  `object_ai_stage();` / `debug_assert_s1_shadow();` immediately before editing mod.rs.
- **Miner-file contention:** none — this plan touches no `src/sim/miner/*` file (Q3).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | Mission grouping: `Sticky→Guard`, `Capture==Sabotage`, `QMove→default Sleep` (+ timer rewrite), `None→Sleep`; `AttackMove` never committed (assign-side), NOT a dispatcher skip | The dispatch routing S2 flips behavior on must match gamemd's switch grouping | **VERIFIED in binary `0x005B3060` during /review-plan** — see Sources |
| 2 | Host routes by `derived_mission()` **fresh at host time** (post-command), not stale `mission.current` | gamemd routes by post-command CurrentMission; a stale read bakes a 1-tick command lag into S2 ordering | Code review of the record pass; churn metric in Task 3 |
| 2 | Iteration set = `live_object_order_snapshot()` (LogicVector), miners + non-Units skipped | gamemd dispatches LogicVector members only; miner/Harvest is the other session's L5 | Task 2 skip test; Task 4 live-set-coverage proof |
| 3 | Dispatch gate order IsActive→timer-due→Health>0 (doc §2.2) is **recorded, not enforced** this slice | Premature enforcement would change behavior; S2 enforces it | Comment + no gate logic this slice; asserted absent by hash-neutral replay |
| 3,5 | Hash neutrality: no RNG, no machine/mission/timer mutation, no `tick_counter` touch | Lockstep — any mutation desyncs the shared stream / golden | `unit_dispatch_host_is_hash_neutral` full-replay test |

---

## Tasks

### Task 1: Pure router module `mission/dispatch.rs`

**Why:** The router is the contract every later task consumes; define it first, fully unit
tested in isolation, with no `sim` access.

**Files:**
- Create: `src/sim/mission/dispatch.rs`
- Modify: `src/sim/mission/mod.rs` (add `pub mod dispatch;` beside the existing `pub mod
  verb;` at mod.rs:13)

**Pattern:** mirrors `src/sim/mission/verb.rs` — a pure-function module over `MissionType`.

**Step 1: Register the module.** In `src/sim/mission/mod.rs`, add after `pub mod control;`
(line 10) line group:
```rust
pub mod dispatch;
```

**Step 2: Define the family enum + router.** Create `src/sim/mission/dispatch.rs`:
```rust
//! Per-object mission dispatch router — the Rust-native stand-in for the common
//! mission-dispatch switch (`match mission`), at Unit handler-family granularity.
//!
//! THIS SLICE the router is a read-only classifier: it maps a `MissionType` to the
//! coarse handler *family* a Unit's behaviour uses, so the per-object AI shell can route
//! each live Unit without executing or moving any handler body. The full per-handler slot
//! identity (every distinct dispatched-handler) is deferred to the all-category slice.
//!
//! Depends on `mission` (MissionType) + `map/entities` (EntityCategory). `sim/` only —
//! never render/ui/sidebar/audio/net. No `dyn`/vtable — data, not trait objects.

use super::MissionType;

/// The coarse Unit handler family a mission routes to. NOT a 1:1 of every distinct
/// dispatched handler — only the families a Unit's behaviour actually uses, plus the two
/// inert buckets. The full per-handler slot table is the all-category slice's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchSlot {
    /// Idle / Sleep family (also the `QMove` default and the `None` idle sentinel).
    Sleep,
    Move,
    Attack,
    /// Guard family — `Guard` and `Sticky` share it.
    Guard,
    /// Dock/enter family.
    Enter,
    /// Harvest family (reachable only for miners, which the live host skips this slice).
    Harvest,
    /// `AttackMove`: an assign-side selector that is NEVER a committed current mission —
    /// the assign side prevents it (and `derived_mission` never yields it for a Unit). The
    /// dispatch switch has NO special skip for it: if 29 ever reached dispatch the binary
    /// would route it via `default` to the Sleep handler WITH a timer rewrite (same as
    /// QMove). `Skip` therefore models the should-never-reach-here invariant — it does NOT
    /// mirror a binary "skip". Kept as a distinct, defensive variant so the unreachability
    /// is asserted, not silently folded into `Sleep`.
    Skip,
    /// Any mission with no Unit handler family this slice (Capture/Eaten/AreaGuard/Return/
    /// Stop/Ambush/Hunt/Unload/Sabotage/Construction/Selling/Repair/Rescue/Missile/Harmless/
    /// Open/Patrol/Paradrop/Deliberate/Spyplane/Retreat) — represented but inert for Units.
    OtherInert,
}

/// Route a mission to its Unit handler family. Total over all 32 dispatched missions plus
/// the `None` idle sentinel; pure; no panics. The reachable-Unit set
/// `{Move, Attack, Enter, Harvest, Guard, None}` maps to live families; everything else is
/// `Skip` (AttackMove) or `OtherInert`.
#[inline]
pub fn unit_dispatch_family(mission: MissionType) -> DispatchSlot {
    use MissionType as M;
    match mission {
        M::Move => DispatchSlot::Move,
        M::Attack => DispatchSlot::Attack,
        M::Enter => DispatchSlot::Enter,
        M::Harvest => DispatchSlot::Harvest,
        // Guard family: Guard + Sticky share the slot.
        M::Guard | M::Sticky => DispatchSlot::Guard,
        // Sleep family: explicit Sleep, the QMove default, and the idle sentinel.
        M::Sleep | M::QMove | M::None => DispatchSlot::Sleep,
        // AttackMove is never a committed current mission (assign-side prevents it); the
        // dispatcher has no skip for it. `Skip` models the should-never-reach-here case.
        M::AttackMove => DispatchSlot::Skip,
        // Everything else has no Unit handler family this slice.
        _ => DispatchSlot::OtherInert,
    }
}
```

**Step 3: Tests.** Append to `src/sim/mission/dispatch.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::mission::MissionType;

    #[test]
    fn reachable_unit_missions_route_to_live_families() {
        assert_eq!(unit_dispatch_family(MissionType::Move), DispatchSlot::Move);
        assert_eq!(unit_dispatch_family(MissionType::Attack), DispatchSlot::Attack);
        assert_eq!(unit_dispatch_family(MissionType::Enter), DispatchSlot::Enter);
        assert_eq!(unit_dispatch_family(MissionType::Harvest), DispatchSlot::Harvest);
        assert_eq!(unit_dispatch_family(MissionType::Guard), DispatchSlot::Guard);
        assert_eq!(unit_dispatch_family(MissionType::None), DispatchSlot::Sleep);
        for slot in [
            unit_dispatch_family(MissionType::Move),
            unit_dispatch_family(MissionType::Attack),
            unit_dispatch_family(MissionType::Enter),
            unit_dispatch_family(MissionType::Guard),
            unit_dispatch_family(MissionType::None),
        ] {
            assert!(
                !matches!(slot, DispatchSlot::Skip | DispatchSlot::OtherInert),
                "non-miner reachable Unit missions must route to a live family"
            );
        }
    }

    #[test]
    fn documented_groupings_and_specials() {
        // Sticky shares the Guard family.
        assert_eq!(
            unit_dispatch_family(MissionType::Sticky),
            unit_dispatch_family(MissionType::Guard)
        );
        // QMove defaults to the Sleep family.
        assert_eq!(
            unit_dispatch_family(MissionType::QMove),
            unit_dispatch_family(MissionType::Sleep)
        );
        // AttackMove is the defensive skip bucket — never a committed Unit mission.
        assert_eq!(unit_dispatch_family(MissionType::AttackMove), DispatchSlot::Skip);
        // A representative TS-legacy / non-Unit mission is inert.
        assert_eq!(unit_dispatch_family(MissionType::Ambush), DispatchSlot::OtherInert);
        assert_eq!(unit_dispatch_family(MissionType::Capture), DispatchSlot::OtherInert);
    }

    #[test]
    fn router_is_total_over_all_missions() {
        // Every dispatched id (0..=31) plus None routes without panic.
        for m in MissionType::all() {
            let _ = unit_dispatch_family(m);
        }
        let _ = unit_dispatch_family(MissionType::None);
    }
}
```

**Step 4: Verify.** Run: `cargo test -p vera20k dispatch:: -- --nocapture`
Expected: 3 tests PASS. (Confirm the `vera20k` package name; a wrong `-p` exits 101 without
running — read the literal `test result:` line.)

**Step 5: Commit.** `sim/mission: per-object dispatch router (Unit family granularity)`

---

### Task 2: Host-time dispatch shadow pass in `techno_ai.rs`

**Why:** This is the host that fills the Unit dispatch role — a debug-only pass that records
each live non-miner Unit's fresh-at-host-time routing, mirroring the S1 shadow. It produces
the trace the proof (Task 3) consumes.

**Files:**
- Modify: `src/sim/world/techno_ai.rs` (add the record type + pass; change
  `object_ai_stage` to return the trace)

**Pattern:** mirrors the S1 `unit_ai_shadow_step` parallel-debug method (techno_ai.rs:166)
— the `EntityCategory::Unit => {}` arm in `techno_ai_shell` stays a no-op.

**Step 1: Add the import + the record type.** Near the top of `src/sim/world/techno_ai.rs`,
add a (non-cfg) `use` for `DispatchSlot` and define the record. The record type is
always-defined (it uses only always-available types: `u64`, `MissionType`, `DispatchSlot`);
only its *population* is gated, so the release trace is an empty, non-allocating
`Vec<UnitDispatchRecord>` — no awkward `Vec<()>` alias:
```rust
use crate::sim::mission::dispatch::DispatchSlot;
// `unit_dispatch_family` is used only by the gated pass + proof:
#[cfg(any(test, debug_assertions))]
use crate::sim::mission::dispatch::unit_dispatch_family;

/// One live Unit's host-time dispatch routing, recorded at `object_ai_stage` time (top of
/// tick, after commands) for the end-of-tick churn proof. Copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnitDispatchRecord {
    pub id: u64,
    /// `derived_mission().0` evaluated fresh at host time (NOT the stale `mission.current`).
    pub host_mission: crate::sim::mission::MissionType,
    pub family: DispatchSlot,
}

/// The per-tick host-time dispatch trace. Populated only in debug/test builds; release
/// returns an empty `Vec` (lazy `Vec::new()` → no allocation on the hot path).
pub(crate) type UnitDispatchTrace = Vec<UnitDispatchRecord>;
```

**Step 2: Add the record pass.** Inside `impl Simulation` in techno_ai.rs (beside
`object_ai_stage`), add:
```rust
/// Host-time Unit dispatch shadow pass (debug/test only). Walks the live-object order
/// (the gamemd dispatch set), and for each live, non-dying, NON-miner Unit records its
/// fresh-at-host-time mission (`derived_mission().0` — NOT the stale `mission.current`,
/// which excludes this tick's commands) and the family it routes to. Read-only: mutates
/// no entity, no occupancy, no hash. Miners are skipped — the miner session's Harvest seam
/// owns that path (Q3).
#[cfg(any(test, debug_assertions))]
fn unit_dispatch_record_pass(&self) -> UnitDispatchTrace {
    let mut trace: UnitDispatchTrace = Vec::new();
    for id in self.live_object_order_snapshot() {
        let Some(e) = self.substrate.entities.get(id) else {
            continue;
        };
        if e.dying || e.category != EntityCategory::Unit || e.miner.is_some() {
            continue;
        }
        let (host_mission, _substate) = e.derived_mission();
        trace.push(UnitDispatchRecord {
            id,
            host_mission,
            family: unit_dispatch_family(host_mission),
        });
    }
    trace
}

/// Release stub: the host-time trace is empty and never allocates.
#[cfg(not(any(test, debug_assertions)))]
fn unit_dispatch_record_pass(&self) -> UnitDispatchTrace {
    Vec::new()
}
```

**Step 3: Return the trace from `object_ai_stage`.** Change its signature and tail. The
current body (techno_ai.rs:45-57) ends after the membership `debug_assert_eq!`. Replace:
```rust
    pub(crate) fn object_ai_stage(&mut self) -> UnitDispatchTrace {
        let visited = self.object_ai_walk(cfg!(debug_assertions));

        #[cfg(debug_assertions)]
        debug_assert_eq!(
            visited,
            self.object_ai_live_order_filtered(),
            "object_ai_stage visit order diverged from live LogicVector order",
        );

        #[cfg(not(debug_assertions))]
        let _ = visited;

        // Unit dispatch shadow: record host-time routing (debug/test only; empty in
        // release). The `Unit => {}` arm of `techno_ai_shell` stays a no-op — the shadow
        // is a parallel pass, exactly like the S1 shadow.
        self.unit_dispatch_record_pass()
    }
```

**Step 4: Tests (skip rules).** Add to the `tests` module in techno_ai.rs:
```rust
#[test]
fn unit_dispatch_record_pass_skips_miner_and_nonunit() {
    let mut sim = Simulation::new();
    // A plain moving Unit — recorded.
    sim.substrate.entities.insert(scoped_move_unit(1));
    // A miner Unit — skipped (Q3).
    let mut miner = scoped_move_unit(2);
    miner.miner = Some(Miner::new(MinerKind::War, &MinerConfig::default(), 0));
    sim.substrate.entities.insert(miner);
    // A non-Unit — skipped by category.
    sim.substrate.entities.insert(entity_of(3, EntityCategory::Structure));
    sim.set_logic_order_for_test(vec![1, 2, 3]);

    let trace = sim.unit_dispatch_record_pass();
    assert_eq!(trace.len(), 1, "only the non-miner Unit is recorded");
    assert_eq!(trace[0].id, 1);
    assert_eq!(trace[0].host_mission, MissionType::Move);
    assert_eq!(trace[0].family, DispatchSlot::Move);
}
```

**Step 5: Verify.** Run: `cargo test -p vera20k techno_ai:: -- --nocapture`
Expected: existing techno_ai tests + the new skip test PASS. (The 3 existing
`sim.object_ai_stage();` call sites compile unchanged — the returned Vec drops.)

**Step 6: Commit.** `sim/world: host-time Unit dispatch shadow pass (debug-only, hash-neutral)`

---

### Task 3: End-of-tick dispatch proof + wire into `advance_tick`

**Why:** The proof turns the host-time trace into the slice's value — non-vacuous invariant
asserts plus the churn metric — and wires it beside the existing S1 proof.

**Files:**
- Modify: `src/sim/world/techno_ai.rs` (the proof method)
- Modify: `src/sim/world/mod.rs` (bind the trace at 2024; call the proof after 2683)

**Pattern:** mirrors `debug_assert_s1_shadow` (techno_ai.rs:197) — read-only, debug-only,
surfaces divergence with tick+id, never equalizes.

**Step 1: Add the proof method.** In `impl Simulation` in techno_ai.rs:
```rust
/// End-of-tick Unit dispatch proof (debug/test only). Runs after `refresh_mission_shadow`,
/// beside `debug_assert_s1_shadow`. For each host-time record it:
///   1. asserts the routed family is correct for the recorded mission (router determinism),
///   2. asserts a non-miner Unit never routes to `Skip`/`OtherInert` from a reachable
///      mission, and that `AttackMove` is never the host mission of a Unit (unreachable),
///   3. re-derives the Unit's mission FRESH now (tail) and, if the family differs from the
///      host-time family, LOGS the churn with tick+id+both missions — it does NOT assert
///      equality (host-time and tail derivations legitimately differ when a Unit's machines
///      change mid-tick). Read-only; never hashed; never silently equalized.
#[cfg(any(test, debug_assertions))]
pub(crate) fn debug_assert_unit_dispatch_shadow(&self, trace: &UnitDispatchTrace) {
    for rec in trace {
        // (1) router determinism: the recorded family is exactly the router's output.
        debug_assert_eq!(
            rec.family,
            unit_dispatch_family(rec.host_mission),
            "dispatch: tick {} unit {}: recorded family must equal the router output",
            self.tick,
            rec.id,
        );
        // (2) a Unit is never on AttackMove (derived_mission cannot yield it).
        debug_assert_ne!(
            rec.host_mission,
            crate::sim::mission::MissionType::AttackMove,
            "dispatch: tick {} unit {}: a Unit must never derive AttackMove",
            self.tick,
            rec.id,
        );
        debug_assert!(
            !matches!(rec.family, DispatchSlot::Skip),
            "dispatch: tick {} unit {}: a live Unit must never route to Skip",
            self.tick,
            rec.id,
        );
        // (3) churn metric: compare host-time family to a fresh tail re-derivation.
        if let Some(e) = self.substrate.entities.get(rec.id) {
            if !e.dying && e.miner.is_none() {
                let (tail_mission, _) = e.derived_mission();
                let tail_family = unit_dispatch_family(tail_mission);
                if tail_family != rec.family {
                    // Surfaced, never equalized — the S2 go/no-go churn signal.
                    log::debug!(
                        "dispatch churn: tick {} unit {}: host {:?} -> tail {:?}",
                        self.tick,
                        rec.id,
                        rec.host_mission,
                        tail_mission,
                    );
                }
            }
        }
    }
}
```
(`log::debug!` is the prevailing mechanism in `sim/` — already used in
`src/sim/miner/miner_system.rs:149` and `src/sim/world/mod.rs:1490` — so it is the correct
choice here. Do NOT add a new logging crate.)

**Step 2: Bind the trace in `advance_tick`.** In `src/sim/world/mod.rs`, re-grep
`self.object_ai_stage();` (currently line 2024) and change it to bind the return:
```rust
        let dispatch_trace = self.object_ai_stage();
```

**Step 3: Call the proof at the tail.** Re-grep `self.debug_assert_s1_shadow();` (currently
2683) and add immediately after it:
```rust
        #[cfg(any(test, debug_assertions))]
        self.debug_assert_unit_dispatch_shadow(&dispatch_trace);
        #[cfg(not(any(test, debug_assertions)))]
        let _ = dispatch_trace;
```

**Step 4: Test (router determinism + AttackMove-unreachable).** Add to techno_ai.rs tests:
```rust
#[test]
fn unit_dispatch_proof_passes_on_scoped_units() {
    let mut sim = Simulation::new();
    sim.substrate.entities.insert(scoped_move_unit(1));   // Move
    let mut idle = scoped_move_unit(2);
    idle.movement_target = None;                          // None -> Sleep family
    sim.substrate.entities.insert(idle);
    sim.set_logic_order_for_test(vec![1, 2]);

    let trace = sim.unit_dispatch_record_pass();
    sim.debug_assert_unit_dispatch_shadow(&trace); // no panic
    assert_eq!(trace[0].family, DispatchSlot::Move);
    assert_eq!(trace[1].family, DispatchSlot::Sleep);
}

#[test]
fn unit_dispatch_attackmove_unreachable_for_units() {
    // derived_mission never yields AttackMove for any machine combination.
    let mut e = GameEntity::test_default(1, "TEST", "Americans", 5, 5);
    e.movement_target = Some(MovementTarget::default());
    e.attack_target = Some(AttackTarget {
        target: TargetKind::Entity(99),
        cooldown_ticks: 0,
        burst_remaining: 1,
        burst_delay_ticks: 0,
        pending_infantry_fire: None,
    });
    assert_ne!(e.derived_mission().0, MissionType::AttackMove);
}
```

**Step 5: Verify.** Run: `cargo test -p vera20k techno_ai:: -- --nocapture`
Expected: all techno_ai tests PASS.

**Step 6: Commit.** `sim/world: end-of-tick Unit dispatch proof + churn metric (debug-only)`

---

### Task 4: Live-set coverage proof (T5) with triage logging

**Why:** Surfaces any Unit a legacy dispatch phase would touch that is NOT in the host's
LogicVector set — the Q2 burden-of-proof check that S2 must not silently drop a unit.

**Files:**
- Modify: `src/sim/world/techno_ai.rs` (extend the proof or add a sibling method)

**Pattern:** the same surface-as-drift discipline; mirrors the legacy phases' own guards.

**Step 1: Add the coverage check.** Add to techno_ai.rs (called from the same end-of-tick
site, or fold into `debug_assert_unit_dispatch_shadow`):
```rust
/// Live-set coverage (T5): every Unit that a legacy dispatch phase would touch — i.e. it
/// carries a dispatch machine AND passes that phase's own guards — must be in the host's
/// live-object set. The legacy phases iterate `keys_sorted()` (all entities); the host
/// iterates the LogicVector. With the legacy guards applied (mirroring `tick_attack_pursuit`:
/// not dying, not Structure, no aircraft mission, not deployed, not a transport passenger)
/// the residual set is expected-empty in normal play. A residual member is a real Rust drift
/// to investigate before S2 — LOGGED with tick+id, never hard-asserted.
#[cfg(any(test, debug_assertions))]
pub(crate) fn debug_check_dispatch_live_set_coverage(&self) {
    use std::collections::BTreeSet;
    let live: BTreeSet<u64> = self.live_object_order_snapshot().into_iter().collect();
    // `iter_sorted()` yields `(u64, &GameEntity)` in ascending-id order (deterministic).
    for (id, e) in self.substrate.entities.iter_sorted() {
        if e.dying
            || e.category == EntityCategory::Structure
            || e.aircraft_mission.is_some()
            || e.is_deployed()
            || e.passenger_role.is_inside_transport()
        {
            continue;
        }
        // A Unit a legacy dispatch phase would act on: has a movement/attack/dock machine.
        let touched = e.movement_target.is_some()
            || e.attack_target.is_some()
            || e.dock_state.is_some()
            || e.order_intent.is_some();
        if touched && !live.contains(&id) {
            log::debug!(
                "dispatch coverage drift: tick {} unit {} touched by a legacy phase but \
                 absent from live order",
                self.tick,
                id,
            );
        }
    }
}
```
Wire it beside the Task-3 proof call in mod.rs:
```rust
        #[cfg(any(test, debug_assertions))]
        self.debug_check_dispatch_live_set_coverage();
```

**Step 2: Test (expected-empty in normal fixtures).** Add to techno_ai.rs tests:
```rust
#[test]
fn dispatch_live_set_covers_moving_units() {
    let mut sim = Simulation::new();
    sim.substrate.entities.insert(scoped_move_unit(1)); // movement_target set, in live order
    sim.set_logic_order_for_test(vec![1]);
    // Must not log/panic: the moving Unit is in the live set.
    sim.debug_check_dispatch_live_set_coverage();
}
```

**Step 3: Verify.** Run: `cargo test -p vera20k techno_ai:: -- --nocapture` — PASS.

**Step 4: Commit.** `sim/world: dispatch live-set coverage proof (T5, surface-as-drift)`

---

### Task 5: Hash-neutrality + phase-order regression tests

**Why:** The headline guarantee — the slice is read-only and must not move the lockstep hash
or perturb the tick phase order. This is the gate for the whole slice.

**Files:**
- Modify: `src/sim/world/techno_ai.rs` (tests)

**Pattern:** mirrors `s1_no_hash_change_shadow` (techno_ai.rs:571) and
`s1_shadow_preserves_advance_tick_phase_order` (629).

**Step 1: Hash-neutrality test.**
```rust
#[test]
fn unit_dispatch_host_is_hash_neutral() {
    let mut sim = Simulation::new();
    sim.substrate.entities.insert(scoped_move_unit(1));
    sim.set_logic_order_for_test(vec![1]);
    sim.refresh_mission_shadow();

    let before = sim.state_hash();
    let trace = sim.object_ai_stage();              // host pass (returns the trace)
    sim.debug_assert_unit_dispatch_shadow(&trace);  // read-only proof
    sim.debug_check_dispatch_live_set_coverage();   // read-only coverage
    let after = sim.state_hash();
    assert_eq!(before, after, "the Unit dispatch host + proofs must not perturb the hash");
}
```

**Step 2: Determinism / phase-order test.**
```rust
#[test]
fn unit_dispatch_preserves_advance_tick_phase_order() {
    fn run() -> Vec<u64> {
        let mut sim = Simulation::new();
        let heights = std::collections::BTreeMap::new();
        (0..5)
            .map(|_| {
                sim.advance_tick(&[], None, &heights, None, None, 67);
                sim.state_hash()
            })
            .collect()
    }
    assert_eq!(run(), run(), "advance_tick with the dispatch host stays deterministic");
}
```

**Step 3: Verify.** Run: `cargo test -p vera20k techno_ai:: -- --nocapture` — PASS.

**Step 4: Full focused check.** Run `cargo test -p vera20k mission:: techno_ai::` and
`cargo check -p vera20k`. Read the literal `test result:` line; confirm zero failures in
files this plan touched (ignore unrelated parallel-session breakage per CLAUDE.md).

**Step 5: Commit.** `sim/world: Unit dispatch host hash-neutrality + phase-order tests`

---

### Task 6: gamemd grouping verification + source-doc correction

**Why:** The router grouping was verified against the binary during `/review-plan`; this task
records the result and corrects the source doc that carried a wrong AttackMove claim.

**Done (in `/review-plan`, `decompile 0x005B3060`):**
- `Guard(5)`==`Sticky(6)`→`+0x21c`; `Capture(8)`==`Sabotage(0x11)`→`+0x214` — **CONFIRMED**.
- `QMove(3)` AND `AttackMove(29)` BOTH hit `default` → Sleep `+0x204` **with a timer rewrite**
  (`+0xC8 = frame`, `+0xD0 = handler_return`) — **CONFIRMED**. There is no dispatcher skip for
  29; it is simply never a committed CurrentMission (assign-side prevents it).
- Gate order IsActive(`+0x90`) → frame-anchored timer → Health(`+0x6C`)>0 → switch(`+0xAC`) —
  **CONFIRMED** bit-for-bit. `unit_dispatch_family`'s groupings match the binary.

**Step 1 (docs follow-up, non-code):** Correct the stale AttackMove characterization in
`docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §2.7, §3(e), §7.6 — they say
"AttackMove 29 falls off the switch (no dispatch, no timer rewrite)"; the binary shows 29 →
`default` → Sleep + timer rewrite, identical to QMove. The accurate statement is "29 is never a
committed CurrentMission (assign-side prevents it); the dispatcher has no special skip." Cite
`decompile 0x005B3060` inline per CLAUDE.md. (Run `/audit` on that doc, or patch in place.)

**No commit for the code path** (verification only). The doc correction is a `docs/` edit
(gitignored, local-only — no commit step).

## Sources & References

- **Design doc:** docs/plans/2026-06-04-unit-mission-dispatch-host-design.md
- **Ghidra reports:** docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md (§2.2,
  §3(e), §7.2, §9); docs/research/TECHNOCLASS_AI_MIGRATION_BOUNDARY_GHIDRA_REPORT.md
- **gamemd.exe addresses (kept here, not in code):** `Mission_Dispatch 0x005B3060` —
  **decompiled in /review-plan**: switch over `+0xAC` CurrentMission; gate IsActive(`+0x90`)→
  timer→Health(`+0x6C`)>0→switch; `default` (catches QMove 3 + AttackMove 29) dispatches Sleep
  `+0x204` + timer rewrite; `Guard`/`Sticky`→`+0x21c`, `Capture`/`Sabotage`→`+0x214`.
  `TechnoClass::AI_Update 0x006F9E50`; `FootClass::AI 0x004DA530`
- **INI keys:** none (routes in-memory mission state)
- **Related code:** src/sim/mission/verb.rs (pure-fn pattern), src/sim/mission/mod.rs
  (MissionType), src/sim/world/techno_ai.rs (S1 shadow precedent, lines 109/166/197/571/629),
  src/sim/world/mod.rs (advance_tick anchors 2001/2010/2024/2671/2683),
  src/sim/game_entity.rs:523 (derived_mission), src/sim/miner/harvest_mission.rs:46 (L5 seam)
- **Prior commits:** S0/S1 + L2 facing (`1c52a01b`); Slice 8 mission-hash; P5a (`dc7a34d9`,
  shifted advance_tick anchors this session)
