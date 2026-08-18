# AI Shell Migration — Plan 2: UnitClass-core absorbs

**Status:** DRAFTED — not approved
**Date:** 2026-06-02
**Rule:** Rust-native structure, gamemd-native semantics — translate the verified behavior contract (ordering, lifecycle, RNG consumption, timer visibility, registration/removal) into idiomatic Rust; never port the C++ class tree, dyn/vtable/COM plumbing, or global-singleton mutation literally.
**Companions:** `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` (design source of truth — §7.1–§7.5, §8, §9 slice ladder S0–S8) + the mission/radio substrate plan (Slices 0–6 + the still-unlanded MissionCom-authority flip / shell S5).

---

## Overview

This plan migrates the three highest-leverage **mobile-leaf** behaviors — movement/locomotor, combat fire + turret facing, and miner/dock harvest — to dispatch through the per-object Techno-AI shell stood up by Plan 1, reproducing gamemd's verified `<Leaf>::AI → FootClass::AI → TechnoClass::AI_Update → Mission_Dispatch → locomotor Process` spine while keeping the engine's clean Rust-native structure (no class tree, `match category` + `CapabilityFlags` + `Option<T>` dispatch). Each slice lands **shadow-first** (new authority is `serde-skip`, unhashed, `debug_assert`-agreement against the existing global sweeps) and only flips to authoritative behind a NAMED acceptance test pinning gamemd order, a `SNAPSHOT_VERSION` bump, and a rebaselined golden. The slices are ordered by parity risk: **L1 movement** (the 1-tick movement-start slip), then **L2 combat + turret** (which carries the RNG-order + damage-math parity risk and a blocking Ghidra batch-vs-inline verdict), then **L5 miner/dock harvest** (a pure routing seam with no authority flip). The dominant sequencing constraint across all three is that the authoritative flips depend on two still-unlanded prerequisites — a per-object `mission::dispatch` host and the MissionCom-authority flip — so most of what this plan *lands now* is shadow scaffolding plus the acceptance tests that gate the later flips.

## Dependency order & gating

**Slice order (this plan): L1 movement/locomotor → L2 combat + turret → L5 miner/dock harvest.**

- **Depends on Plan 1** — the per-object Techno-AI shell (`object_ai_stage` / `techno_ai_step`, the substrate's `LogicVector` + `for_each_live_object`/`live_object_order_snapshot` active-order) must exist first. L1.0 is itself the harness step, so L1 and Plan 1 are tightly coupled; L2 and L5 stand up their own minimal hosts where the full shell is not yet required.
- **L1 before L2 before L5** is a *parity-risk* ordering, not a hard compile dependency: L2's per-object fire+facing host is shaped to be order-compatible with the movement shell, and L5's harvest seam is independent of both but rides on the same substrate active-order.
- **The two cross-cutting blockers** gate every *authoritative flip* (not the shadow landings):
  1. **A per-object `mission::dispatch(sim, id)` host must exist** — it does NOT today (`mission/` has verbs/timer/control/retask but no per-object dispatch stage; `MissionType::dispatch_id` at `mission/mod.rs:87` is only a byte accessor).
  2. **MissionCom must become authoritative** — today `current`/`substate` are *derived from* the `Option<T>` machines (`derived_mission` @ `game_entity.rs:482`) and unhashed; the authority flip is a still-unlanded later slice (shell **S5** ↔ mission/radio **Slice 6**).
- **L1.2+ (movement flips), L2 Task 3 (combat flip)** are blocked on BOTH prerequisites. **L5 flips nothing** — it lands its routing seam + pinning tests now and explicitly defers its two relevant flips (substate-authority = shell S5 ↔ mission/radio Slice 6; bus-authority = the registry-retire slice the live code labels **"Slice 8"**).
- **L2 Task 0** adds a third, slice-local hard blocker: a Ghidra batch-vs-inline damage-timing verdict that must land before L2 Task 3 chooses its damage-application shape.

---

## Slice L1 — Absorb ground/air movement (locomotor Process) under the per-object shell

> **Review notes — what I corrected in the draft (all re-verified against the live tree this session):**
> 1. **The Slice-6 blocker was mischaracterized.** The draft repeatedly says "mission/radio Slice 6 (MissionCom authority — NOT landed)." Slice 6 **HAS landed** as a *verb API + parallel-write* slice (`src/sim/mission/verb.rs`, `retask.rs`, `slice6_retask_tests.rs` with `replay_hash_stable_through_slice6`; `MissionCom` written in parallel at `game_entity.rs:456`). What is **NOT landed** is **MissionCom becoming authoritative** — `current`/`substate` are still *refreshed from* the `Option<T>` machines via `derived_mission` (`game_entity.rs:482`), and the field is unhashed (`#[serde(default)]`, not folded into `world_hash`). The real blocker for L1.2 is the **MissionCom-authority flip (a still-unlanded later slice), not "Slice 6."** Renamed throughout to **"the MissionCom-authority flip"** to avoid the stale label.
> 2. **`mission::dispatch(sim, id)` does not exist.** There is no per-object mission-dispatch stage in the tree — only `MissionType::dispatch_id()` (a byte accessor, `mission/mod.rs:87`). The draft's pseudocode invents a `mission::dispatch` call. The per-object dispatch host is **itself unlanded greenfield** and is a prerequisite the L1.2 flip must either build or wait on — corrected the dependency graph to make this explicit (it is a *second* unlanded prerequisite alongside the authority flip).
> 3. **Line-number fixes:** `refresh_mission_shadow` def @ **895** / called @ **2391** (draft said :859/:929/:2355 — all stale); `tick_deploy_state` call @ **1992** (draft :1956); `tick_fear_for_entities` call @ **1996** (draft :1960); scatter disabled block @ **2271** (draft's :2271-2279 was right, but the FACTS block's :2235-2243 was stale). `tick_aircraft_missions` def @ **`aircraft/mod.rs:154`** / called @ **1902** (FACTS said :152/:1866 — stale; draft already fixed to :154/:1902, kept).
> 4. **Confirmed-correct in the draft (kept):** `advance_tick`@1742, Phase 1@1777-1778, Phase 2@1814-1883, tunnel@1847, `flush_pending_delete` drains@1719/1770, `live_object_order_snapshot`@929, `for_each_live_object`@947, `process_drive_locomotion_shell` near-stub@`drive_locomotion.rs:27`, per-id call@`movement_tick.rs:906`, `pending_arrival_clear`@907, grid pre-build@845/882/899, no `sim/ai/` dir. Approach choice (seam-reuse over clean rewrite) is sound and well-justified.
> 5. **Acceptance tests:** kept the named set; demoted the L1.2 tests' gate-condition to "lands only after BOTH the dispatch host AND the authority flip" and tightened `per_cell_arrival_callback_fires_after_mission_retry` (the spec-mandated ordering test) to explicitly assert against a *named* shadow trace rather than implying a live flip.

---

#### 0. Critical pre-flight (read before anything)

**Two distinct things are unlanded greenfield that L1 depends on, and the draft conflated them with a landed slice:**

- **(a) The per-object mission-dispatch host does not exist.** There is no `mission::dispatch(sim, id)`, no `object_ai_stage`, no `techno_ai_step`. The mission module today is *vocabulary + verb-write + timer + control table* (`control.rs`, `retask.rs`, `timer.rs`, `verb.rs`, `mod.rs`). The verbs **write** MissionCom; nothing **dispatches off** it per object per tick. L1.0 must build the host; L1.2's "dispatch then process" cannot reference a dispatch entry point until one exists.
- **(b) MissionCom is not authoritative.** `current`/`substate` are derived from the still-authoritative `Option<T>` machines each tick (`derived_mission` @ `game_entity.rs:482`) and the field is unhashed. "Run the locomotor *after mission dispatch*" is only meaningful once dispatch is the authority for *what the unit decided to do this tick*; until the authority flip, the shell's post-dispatch slot can only be a **shadow** asserting agreement with the current movement-then-mission phase split.

**Consequence:** L1 lands as **shadow only (L1.0 + L1.1)** now. The flip sub-slices (L1.2 → L1.4) are authored here but **blocked from landing** on *both* (a) a per-object dispatch host existing AND (b) the MissionCom-authority flip. This is the single largest sequencing constraint in L1 and is stated unconditionally in §8.

There is no `src/sim/ai/` directory; `world/techno_ai.rs` is **owned by Plan 1 Slice S0** (greenfield only until S0 lands — L1.0 consumes it, see §4 L1.0). Design-§9 Slices S0/S1 (the harness + first shadow) are **not** what landed under "mission/radio Slices 0–3" — those were the mission/radio substrate, a different track.

**Re-verified live-tree anchors (this session):**

| Symbol | Live line |
|---|---|
| `advance_tick` fn | `mod.rs:1742` |
| Phase 1 snapshot / `tick_movement_with_grids` call | `:1777` / `:1778` |
| Phase 2 air/special snapshot (`special_movement_order`) | `:1814` |
| `tick_air_movement` call / def | `:1815` / `air_movement.rs:191` (air-leaf filter `layer==Air && kind!=Rocket` @ `:215-221`) |
| `tick_tunnel_movement` (TS) | `:1847` |
| teleport/rocket/homing/droppod/parachute/piggyback calls | `:1829-1883` |
| `tick_aircraft_missions` call / def | `:1902` / `aircraft/mod.rs:154` |
| `tick_deploy_state` call | `:1992` |
| `tick_fear_for_entities` call | `:1996` |
| scatter disabled block (commented) | `:2271+` |
| `refresh_mission_shadow` def / call | `:895` / `:2391` (pre `state_hash`) |
| `for_each_live_object` / `live_object_order_snapshot` | `:947` / `:929` |
| `flush_pending_delete` drains | `:1719`, `:1770` |
| `tick_movement_with_grids` def | `movement_tick.rs:820` (grid pre-build `:845/:882/:899`; per-id shell call `:906`; `pending_arrival_clear` `:907`) |
| `process_drive_locomotion_shell` (near-stub) | `drive_locomotion.rs:27` |
| `MissionCom` field / `derived_mission` | `game_entity.rs:456` / `:482` |
| `MissionType::dispatch_id` (NOT a per-object dispatch) | `mission/mod.rs:87` |

---

#### 1. Approach (brainstorm step)

**Chosen: incremental seam-reuse** — stand up `object_ai_stage` as a new per-object AI sub-stage that walks substrate-owned active order and calls a per-leaf shell; the shell's ground locomotor entry re-hosts the *existing* per-entity mover body that today lives inline in `tick_movement_with_grids` (`movement_tick.rs:902-922`), behind a shadow assertion, before retiring the global phase.

**Rejected: clean per-object rewrite** of movement (lift drive-track/cell-crossing math into a standalone stepper and delete the global phase in one move). The cell-crossing cadence — forced-drive tracks (`movement_tick.rs:899`), the per-mover collection loop (`:902-922`), `pending_arrival_clear` (`:907`), repath/occupancy/crush ordering — is interleaved with **cross-entity grids** (occupancy, blocker-neighbor counts `:845`, friendly-passable block-sets `:926`) that are *not* per-object. A clean rewrite would re-derive that cross-cutting structure at once and flip both authority *and* cadence in a single unverifiable jump — the "two 5%-off systems compound to 10%" risk. Seam-reuse keeps cadence bit-identical (the shell *calls the same code*), lands the ordering change as a shadow first (invariant #4), and retires the global phase only after a named per-cell-arrival-ordering test passes — matching S0→S1→S2 discipline and the "read the full loop" rule. The cost (temporary double-iteration during shadow) compiles out at flip.

---

#### 2. Goal

Move the locomotor/movement step for all **mobile leaves** (Unit/Infantry drive+walk, Aircraft fly/jumpjet, and the active special movers: teleport/rocket/homing/droppod/parachute + piggyback-restore) to run **inside the per-object shell, AFTER per-object mission dispatch**, reproducing the verified spine `<Leaf>::AI → FootClass::AI → TechnoClass::AI_Update → Mission_Dispatch → locomotor ILocomotion::Process`. Retire `tick_movement_with_grids` (Phase 1, `mod.rs:1778`) and the air/special movers (Phase 2, `:1815-1883`) as **leading global phases**. Preserve drive-track/cell-crossing cadence exactly. **Tunnel locomotor (`tick_tunnel_movement` `:1847`) is DORMANT/TS — keep its phase slot inert; do NOT absorb it as live.** Shadow-first, then flip per leaf, each flip gated on a NAMED per-cell-arrival-callback-vs-mission-retry ordering test **and** on the dispatch host + MissionCom-authority prerequisites.

---

#### 3. Files / surfaces (exact file:line)

**OWNED BY PLAN 1 SLICE S0 (consume, do not re-author):**
- `src/sim/world/techno_ai.rs` — the shell module, `Simulation::object_ai_stage(&mut self)` (the per-`EntityCategory` `match` no-op shell walked over live order), and the three no-op tests (`techno_ai_shell_is_passthrough_no_hash_change` / `_membership_matches_phase_snapshot` / `_preserves_advance_tick_phase_order`) are **authored and owned by Plan 1 Slice S0**. **L1.0 consumes S0; it does NOT re-create the module, the stage, or those tests** (see §4 L1.0). L1.1+ EXTENDS S0's `object_ai_stage` / per-category arms (e.g. fills the `Unit` arm with the locomotor step). The richer §7.1 `sim/ai/` tree is the eventual home but out of scope for L1: stay in `world/techno_ai.rs`; split into `sim/ai/` only when it crosses ~600 lines. **Do NOT create both, and do NOT author a second `object_ai_stage` signature — S0's `&mut self` (no `ctx`) is canonical.**

**MODIFY (flip sub-slices only; L1.0/L1.1 add the call but move no phase):**
- `src/sim/world/mod.rs`:
  - `advance_tick` `:1742` — S0 already wires `self.object_ai_stage()` just before `refresh_mission_shadow()` (`:2391`). **L1.0–L1.1 reuse S0's call site (shadow only, Phase 1/2 untouched).** Relocating the stage to the movement-slot region (after per-object mission dispatch, before vision) is part of the **L1.2+ flip** (which moves phases under a `SNAPSHOT_VERSION` bump); L1.2+ then routes scoped movers out of Phase 1 (`:1778`) / Phase 2 (`:1815`).
- `src/sim/movement/movement_tick.rs:820` (`tick_movement_with_grids`) — extract a per-entity locomotor entry from the **per-mover position-stepping loop `for entity_id in movers` at `:978`** (the `:904` / `:963` passes are the drive-shell presence check + `pending_arrival_clear`, not the position advance); keep the cross-entity grid work as a KEEP-AS-GLOBAL-SERVICE pre-pass.
- `src/sim/movement/drive_locomotion.rs:27` — `process_drive_locomotion_shell` is a **near-stub** (only checks `drive_locomotion.is_none()`); the real per-mover drive stepping is inline in `tick_movement_with_grids`. This is the seam to fill (L1.1).
- `src/sim/movement/air_movement.rs:191` — expose `air_locomotor_step(entity, …)` per-entity entry (extract from the `air_entity_ids` loop `:227+`; the leaf gate is the existing filter `:215-221`).
- `src/sim/movement/{teleport_movement,rocket_movement,homing_movement,droppod_movement,parachute_descent}.rs` + `movement::tick_locomotor_piggyback_restore` — same per-entity-entry exposure (L1.3). **`tunnel_movement.rs` excluded.**
- **Prerequisite (NOT authored in L1, but L1.2 depends on it):** a per-object `mission::dispatch(sim, id)` entry in `src/sim/mission/`. It does not exist; L1 does not build it (that is the dispatch-host slice). L1.2's pseudocode is written *against* it as a forward reference.

**READ-ONLY (consumed, not modified):** `mod.rs:947` `for_each_live_object`; `:929` `live_object_order_snapshot`; `:1060`/drains `:1719`/`:1770` `flush_pending_delete` (must NOT be called inside the AI stage); `logic_vector.rs` (`len`/`as_slice`/`snapshot`, no sort); `game_entity.rs:456` `MissionCom` + `:482` `derived_mission`.

---

#### 4. Step-by-step tasks

##### L1.0 — Consume Plan 1 Slice S0 (instrumented no-op shell harness)

**L1.0 IS design Slice S0, and Plan 1 Slice S0 OWNS it** — the module `src/sim/world/techno_ai.rs`, `Simulation::object_ai_stage(&mut self)` (the per-`EntityCategory` `match` no-op shell walked over live order via `for_each_live_object`), and the three no-op tests (`techno_ai_shell_is_passthrough_no_hash_change`, `techno_ai_shell_membership_matches_phase_snapshot`, `techno_ai_shell_preserves_advance_tick_phase_order`). L1.0 does **not** re-author any of this.

- **If S0 has already landed:** L1.0 is a pure *consume* step — do **NOT** re-create the module, the stage, or those three tests. L1.1 EXTENDS the existing `object_ai_stage` (fills the `Unit` arm with the locomotor step); it never redefines the shell.
- **If S0 has NOT landed when L1 starts:** then *this step is S0* — author it per **Plan 1 Slice S0** (its §4 Task 1–3 + §6 tests), not a divergent skeleton drafted here.

**Signature decision (one only): `object_ai_stage(&mut self)` — NO `ctx` parameter** (S0's signature is canonical; the earlier L1.0 draft's `object_ai_stage(&mut self, ctx: &ObjectAiCtx)` is **superseded**). The borrowed locomotor grids the per-leaf step needs at L1.1+ (`path_grid`, `tick_ms`, `sim_tick`, terrain/occupancy handles) are threaded by **extending** the shell at the first flip slice that consumes them — pass them into the per-leaf step (`foot_locomotor_step_ground(...)`) or add a parameter then — **not** by re-declaring `object_ai_stage` with a `ctx` at L1.0. Never ship two `object_ai_stage` signatures.

**Inherited from S0:** the stage walks live order via `for_each_live_object` (`mod.rs:947`, re-reads `logic.len()` each iteration — a unit revealed mid-stage acts this stage). **Do NOT call `flush_pending_delete` inside the stage** (flush stays at cleanup `:1719`). **Dispatch is `match category` + `Option<T>` — no trait/`dyn`/vtable** (invariant #2). The stage is wired just before `refresh_mission_shadow()` (`:2391`); L1.0 reuses that call site and moves no phase.

**Becomes authoritative:** nothing. **Verify:** the S0 tests (`cargo test -p vera20k techno_ai`) + full-replay golden unmoved — owned by S0; if S0 has landed, L1.0 adds nothing new to verify.

##### L1.1 — Locomotor-after-dispatch, one UnitClass scenario, SHADOW (design S1)

Land the verified ordering (per-object dispatch, *then* locomotor Process, same pass) for **one narrow scenario** (a single moving Unit on `Mission_Move`/`Mission_Guard`, no combat, no docking) as a **shadow** that `debug_assert`s agreement with the current phase-split movement-then-mission ordering. Not hashed. No phase moved.

**Task 1.1 — extract per-entity ground locomotor entry** from `tick_movement_with_grids`. Lift the inline per-mover body (`movement_tick.rs:902-922`: `process_drive_locomotion_shell` call `:906`, `pending_arrival_clear` `:907`, forced-drive filter `:910`, layer gate `:916-918`) into:

```rust
// movement_tick.rs — per-entity entry the shell calls AFTER dispatch.
pub(crate) fn foot_locomotor_step_ground(
    entity_id: u64,
    entities: &mut EntityStore,
    grids: &mut GroundLocomotorGrids<'_>,   // occupancy, next_occupancy_enter_order,
                                            // path_grid, terrain_costs, alliances, zone, resolved
    cfg: &MovementConfig,
    rng: &mut SimRng,
    dt: SimFixed,
    sim_tick: u64,
    interner: &mut StringInterner,
    rules: Option<&RuleSet>,
    sound_events: &mut Vec<SimSoundEvent>,
) -> PerMoverOutcome;
```

`GroundLocomotorGrids` bundles the **cross-entity KEEP-AS-GLOBAL-SERVICE** state (occupancy, blocker-neighbor counts, friendly-passable block-sets) built **once per stage** in a pre-pass, NOT per object. **Load-bearing cadence boundary:** the per-mover loop body moves into `foot_locomotor_step_ground`; the grid pre-build (`build_blocker_neighbor_counts` `:845`, drive-reaim `:882-894`, `tick_forced_drive_tracks` `:899`) stays a global pre-pass run before the per-object walk. **`process_drive_locomotion_shell` (`drive_locomotion.rs:27`) is filled out here** to actually perform the drive step it currently stubs — reproducing the inline behavior exactly, not re-deriving it.

**Task 1.2 — shadow assertion.** For the in-scope scenario only, `unit_ai(sim, id)` computes the would-be post-dispatch locomotor result via `foot_locomotor_step_ground` against a cloned/scratch state and `debug_assert!`s it equals the current tick's actual Phase-1 output for that id; on divergence it logs `(tick, id, expected, got)` and **never silently equalizes** (surface, don't triage). Live authority stays Phase 1; the shell is read-only. **Because no per-object `mission::dispatch` exists yet, L1.1's "after dispatch" is a shadow against the current movement-then-mission split — not a real dispatch call.**

**Becomes authoritative:** nothing (shadow, not hashed). **Verify:** `cargo test -p vera20k unit_move` + `s1_no_hash_change_shadow`.

##### L1.2 — Flip UnitClass dispatch→process authoritative (scoped) — DOUBLE-BLOCKED

Promote the L1.1 ordering to authoritative for scoped UnitClass. `SNAPSHOT_VERSION` bump + fresh golden.

**Function-move mapping (precise; bodies = the extracted code, not invented):**

| Source (current global) | Destination (shell slot) | Becomes |
|---|---|---|
| per-mover body `movement_tick.rs:902-922` | `foot_locomotor_step_ground` called from `unit_ai` AFTER dispatch | authoritative for scoped Units |
| grid pre-build `:845/:882/:899` | global pre-pass `ground_locomotor_prepass(sim)` run once before `object_ai_stage` | KEEP-AS-GLOBAL-SERVICE |
| `tick_movement_with_grids` outer `:820` | retained for out-of-scope categories; scoped ids skipped | partially retired |

**Per-object pass shape** (skeleton, not a full body):
```
fn unit_ai(sim, id, ctx):
    techno_common_pre(sim, id)            // deferred substrate slice; L1.2 = no-op
    sim.entity_mut(id).ai_tick += 1       // +0xC4 increment BEFORE dispatch
    mission::dispatch(sim, id)            // PREREQUISITE — does not exist yet (see blocker)
    if !sim.is_alive(id) { return }       // early-return death point
    foot_locomotor_step_ground(id, …)     // ILocomotion::Process AFTER dispatch
```

**BLOCKER (unconditional — do NOT land L1.2 until BOTH clear):**
1. **A per-object `mission::dispatch(sim, id)` must exist.** It does not today (`mission/` has verbs/timer/control but no per-object dispatch stage). This is a separate unlanded dispatch-host slice.
2. **MissionCom must be authoritative.** Today `current`/`substate` are *derived from* the `Option<T>` machines (`derived_mission` `game_entity.rs:482`) and unhashed; the authority flip is a still-unlanded later slice. Until it flips, "after mission dispatch" is only a shadow assertion (L1.1).

**Becomes authoritative:** per-object dispatch→process ordering + `+0xC4` increment-before-dispatch for scoped UnitClass.

##### L1.3 — Generalize to all mobile leaves (air + active special movers). Tunnel excluded.

| Source (current Phase 2) | Destination | Notes |
|---|---|---|
| `tick_air_movement` `:1815` (loop `air_movement.rs:227+`) | `aircraft_ai` → `air_locomotor_step(entity, dt, sim_tick)` | leaf gate = existing filter `:215-221` |
| `tick_teleport_movement` `:1829/:1838` | per-leaf teleport step, same Process slot | needs `&mut occupancy` + optional `TeleportVisuals` (the visuals struct is *passed in* by the caller assembling `world_effects`; the shell takes a borrowed handle — NOT a sim→render dep) |
| `tick_rocket_movement` `:1854` | per-leaf | `kind==Rocket` |
| `tick_homing_movement` `:1863` | per-leaf | detonation list currently unused — preserve |
| `tick_droppod_movement` `:1869` | per-leaf | |
| `tick_parachute_descent` `:1876` | per-leaf | |
| `tick_locomotor_piggyback_restore` `:1883` | per-leaf tail restore | |
| `tick_tunnel_movement` `:1847` | **NOT ABSORBED** — inert phase slot | TS_LEGACY; gate never fires (empty tube array) |

**Aircraft-mission snapshot wrinkle (latent parity item, NOT touched here):** `tick_aircraft_missions` (`aircraft/mod.rs:154`, called `:1902`) iterates `entities.values()` = **BTreeMap id-ascending**, NOT logic order. The shell uses logic order. **L1.3 absorbs only the aircraft *locomotor* (movement) step, NOT the aircraft mission state machines** — flag the BTreeMap-vs-logic-order divergence as a watch item inherited by the future aircraft-mission absorb slice; do **not** fix it here. Each leaf lands behind its own shadow→flip, each flip carrying the same double-blocker as L1.2.

##### L1.4 — Retire Phase 1/2 globals as leading phases

Once all mobile leaves run their locomotor under the shell, remove `tick_movement_with_grids` (`:1778`) and the air/special movers (`:1815-1883`) as **leading global phases**. The cross-entity grid pre-pass (`ground_locomotor_prepass`) stays global. Tunnel slot stays inert. `SNAPSHOT_VERSION` bump + golden.

**Hard constraint (invariant #3):** shadow slices (L1.0/L1.1) MUST NOT move Phase 1/2. Only flip sub-slices (L1.2+) move them, each with a `SNAPSHOT_VERSION` bump + new golden, justified by cited gamemd evidence (`FootClass::AI` runs Process after `Mission_Dispatch`; `+0xC4` increment before dispatch). Retire is the **last** step, after every mobile leaf is flipped.

---

#### 5. Shadow vs authoritative

| Slice | Shadow (debug_assert, not hashed) | Authoritative |
|---|---|---|
| L1.0 | shell pass-through (visits + membership assert) | nothing |
| L1.1 | per-object dispatch→process for one Unit scenario | nothing (Phase 1 still authority) |
| L1.2 | — | scoped UnitClass dispatch→process + `+0xC4` (DOUBLE-BLOCKED: dispatch host + authority flip) |
| L1.3 | per-leaf air/special (each shadow first) | scoped air/special locomotor (after own flip; same double-blocker) |
| L1.4 | — | Phase 1/2 removed as leading globals; grid pre-pass stays global |

The grid/occupancy/blocker-neighbor cross-entity work is **always KEEP-AS-GLOBAL-SERVICE** (a pre-pass), never per-object. Tunnel stays an inert phase slot throughout.

---

#### 6. NAMED acceptance tests

**L1.0 (OWNED BY PLAN 1 S0 — do not re-author):** `techno_ai_shell_is_passthrough_no_hash_change`, `techno_ai_shell_membership_matches_phase_snapshot`, and `techno_ai_shell_preserves_advance_tick_phase_order` are authored and owned by Plan 1 Slice S0. L1.0 reuses them as-is; if S0 has landed they already pass and L1.0 adds no new test.

**L1.1 (the spec's core ask — per-cell arrival ordering vs mission retry, all SHADOW):**
- `unit_ai_mission_dispatch_precedes_locomotor_process` — in the shadow trace, dispatch is observed before locomotor Process for the scoped Unit.
- `unit_move_dispatch_then_process_shadow_agrees` — shadow post-dispatch movement matches the live phase-split output every tick (zero `debug_assert` failures); divergence logged with tick+id, not equalized.
- `per_cell_arrival_callback_fires_after_mission_retry` — **spec-mandated ordering test, asserted against the named shadow trace:** on the tick a drive mover crosses a cell boundary, the `pending_arrival_clear` callback (`movement_tick.rs:907`) fires at the point relative to the unit's mission retry that gamemd produces (dispatch → locomotor advances the cell → arrival callback), not the current inverse (movement-then-mission). Asserted in the L1.1 shadow, before any live flip.
- `s1_no_hash_change_shadow` — `state_hash` unmoved.

**L1.2 (flip, scoped Unit) — land only after BOTH the dispatch host AND the authority flip:**
- `unit_move_start_slip_matches_dispatch_then_process` — a freshly-ordered Move advances on the tick predicted by dispatch-then-process, not the prior phase-split tick.
- `unit_c4_counter_increments_before_dispatch`.
- `scoped_vs_unscoped_unit_cell_contention_deterministic` — a scoped + an unscoped unit racing for one cell resolve deterministically and identically across replays (pins RNG-stream order).
- `drive_track_cell_crossing_cadence_unchanged_after_flip` — drive-track/cell-crossing cadence (forced-drive tracks, residual-budget stepping) bit-identical pre/post flip for the scoped unit (the "preserve cadence exactly" gate).
- `l2_snapshot_version_bumped_golden_rebaselined`.

**L1.3:** `aircraft_fly_locomotor_runs_after_dispatch_shadow_agrees`; `teleport_special_movers_absorbed_no_drift` (teleport/rocket/homing/droppod/parachute per-leaf output bit-identical to the Phase-2 sweep for scoped entities); `tunnel_movement_remains_inert_not_absorbed`; `aircraft_mission_snapshot_order_unchanged_in_l1` (aircraft *mission* iteration stays BTreeMap-order — the wrinkle is NOT touched by the movement absorb).

**L1.4:** `phase1_ground_move_retired_no_drift`; `ground_locomotor_prepass_stays_global` (cross-entity grid pre-pass runs once before the object stage, not per object); `l4_snapshot_version_bumped_golden_rebaselined`.

---

#### 7. Determinism / hash notes

- **Shadow slices (L1.0, L1.1) MUST NOT move `state_hash`** — prove with the `*_no_hash_change` tests before any flip. New shadow fields (scratch locomotor result, `ai_tick` while shadowed) are `#[serde(skip)]` and not folded into the hash until their flip.
- **RNG position is load-bearing.** The ground mover consumes `&mut self.scenario_rng` for bump/scatter + sub-cell (passed at `mod.rs:1789`, used inside `movement_tick.rs:828`'s `rng` param). When the per-mover body moves into `foot_locomotor_step_ground`, it must consume RNG at the **same per-object position in active-vector (logic) order** as Phase 1 does today — Phase 1 already iterates `live_object_order_snapshot()` (no sort), so the shell using the same logic order preserves the draw sequence. **Any reordering of the mover walk changes the RNG stream → desync.** `scoped_vs_unscoped_unit_cell_contention_deterministic` pins this.
- **Frame-anchored timers never decrement** (invariant #5): the `+0xC4` counter is a per-object increment (not a timer); MissionTimer re-arm writes `start_frame`+`duration` and is the authority-flip slice's territory, not L1.
- **Single time base = `binary_frame`**, committed late (`mod.rs:1738`). Idle-scatter and any frame-phased work the shell later hosts use `binary_frame`, not the per-object counter (scatter is a separate row, currently DISABLED at `:2271+` — not part of L1).
- **Each authority flip = `SNAPSHOT_VERSION` bump + rebaselined golden** justified by cited gamemd evidence; the state hash is a self-replay determinism oracle, not a gamemd-parity oracle (invariant #8).
- **Deferred death** (invariant #6): any death inside `foot_locomotor_step_ground` (crush kill — `PendingCrushKill` at `movement_tick.rs:877`) routes through `substrate.uninit` → enqueue `pending_delete`; the shell never frees synchronously and never calls `flush_pending_delete` inside the AI stage (flush stays at cleanup `:1719`).

---

#### 8. Dependencies + risk + do-not-do

**Dependencies (ordered):**
1. **L1.0 (S0 harness)** depends on object-substrate Slices 1–2 (landed: `LogicVector`, `for_each_live_object` `:947`, `ObjectSubstrate`).
2. **L1.1 (S1 shadow)** depends on L1.0.
3. **L1.2 (flip, scoped Unit)** depends on L1.1 **AND two still-unlanded prerequisites: (a) a per-object `mission::dispatch` host, (b) the MissionCom-authority flip.** Until both land, "locomotor after dispatch" is shadow only (L1.1). **L1 lands as shadow (L1.0 + L1.1) now; the flip sub-slices are authored but blocked.** This is the largest sequencing constraint in L1.
4. **L1.3 (air/special)** depends on L1.2's pattern (and inherits the same double-blocker per leaf).
5. **L1.4 (retire)** depends on every mobile leaf being flipped.

**Risk:**
- **Highest-leverage ordering in the migration** (design §9 S1): flipping movement-after-dispatch changes *when* a freshly-dispatched `Mission_Move` first advances — a 1-tick movement-start slip that is player-visible. Shadow-first surfaces the divergence count before committing.
- **Cadence drift:** the extraction (L1.1 Task 1.1) is where a 1-tick/1-cell slip could enter. `drive_track_cell_crossing_cadence_unchanged_after_flip` is the gate; seam-reuse mitigates by *calling the same code*.
- **`process_drive_locomotion_shell` is a near-stub** (`drive_locomotion.rs:27`) — the real cadence is inline in Phase 1; filling the stub must reproduce inline behavior exactly, not re-derive it.

**DO-NOT-DO:**
- **Do NOT absorb tunnel/subterranean** (`tick_tunnel_movement` `:1847`) — TS_LEGACY/DORMANT; keep its slot inert.
- **Do NOT move any phase in L1.0/L1.1** (invariant #3) — shadow only; only flip sub-slices move Phase 1/2, each with a version bump.
- **Do NOT introduce a per-object owner of occupancy/grids** — the cross-entity grid pre-pass stays KEEP-AS-GLOBAL-SERVICE.
- **Do NOT add a trait/`dyn`/vtable** for per-leaf dispatch — `match category` + `Option<T>` + `CapabilityFlags` only (invariant #2).
- **Do NOT call `flush_pending_delete` inside `object_ai_stage`** — flush stays at cleanup (`:1719`).
- **Do NOT fix the aircraft-mission BTreeMap-vs-logic-order divergence in L1** — it belongs to the aircraft-*mission* absorb, not the movement absorb; flag it as inherited.
- **Do NOT flip L1.2+ before BOTH the per-object dispatch host AND the MissionCom-authority flip land** — and do NOT reference a `mission::dispatch` call as if it exists; it does not.
- **Do NOT create both `world/techno_ai.rs` and `sim/ai/`** — pick `world/techno_ai.rs` for L1.

**Relevant files (absolute):** `src/sim/world/techno_ai.rs` (new), `...\src\sim\world\mod.rs`, `...\src\sim\movement\movement_tick.rs`, `...\src\sim\movement\drive_locomotion.rs`, `...\src\sim\movement\air_movement.rs`, the special-mover siblings under `...\src\sim\movement\`, `...\src\sim\world\logic_vector.rs`, `...\src\sim\game_entity.rs`, `...\src\sim\mission\` (dispatch-host prerequisite — not built in L1). Design doc: `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` (§7.1–§7.5, §8 rows 1–2, §9 S0–S2).

---

## Slice L2 — Absorb UnitClass Fire + Facing into `unit_post` (retire the split combat/turret sweeps)

> **Review notes — what I corrected in the draft (all verified against the live tree this session):**
> 1. **Combat-internal line citations re-verified and corrected.** The draft's `combat/mod.rs` cites were close but several were off: snapshot-sort is at **`:1533-1538`** (draft said `:1528`); the `barrel.current` alignment gate body spans **`:1863-1892`** with the read at **`:1879`** (draft said `:1864-1892`); the damage-event push is at **`:1946`**; the batched damage-apply is **Phase 4 `:2190-2220`** and death handling is **Phase 6 `:2231-2250`** (`handle_entity_deaths` at `:2232`). Verified `:1376` snapshot-vec, `:1392-1393` cooldown decrement, `:1494` push are correct.
> 2. **The batch model is bigger than "extract the loop body."** Confirmed by reading `:2050-2287`: `tick_combat_with_fog` is a 6-phase batch (P2 collect `damage_events` for *all* attackers at pre-damage HP → P3 apply retarget/burst/ammo/garrison → P4 apply damage → P5 clear attack_target → P6 `handle_entity_deaths`). **Phases 4 and 6 operate on the aggregated cross-attacker `damage_events`/`dead_entities` vectors, not per-attacker.** So Task 1's "extract one attacker's fire body" only cleanly extracts **P2** (the fire-decision + damage-event emission); P4/P6 stay batched. This makes Task 0's inline-vs-batch question the true crux: if gamemd is inline, the per-object walk must thread damage-apply + death **into** the per-object step, which is a much deeper change than the draft's "fold the sweeps" framing. Raised this explicitly in §8.
> 3. **`for_each_live_object` (same-pass re-read) must NOT be used in L2 — corrected to mandate `live_object_order_snapshot` (point-in-time).** Current combat uses the point-in-time snapshot (`:2026`). The draft left the iterator choice open ("§7.2 uses re-read"); switching to re-read in L2 would be a latent, unannounced behavior change (mid-pass membership visibility) that combat does not have today. L2 must match the current point-in-time semantics exactly. The future full shell (S4/S5) can adopt re-read when it owns spawns.
> 4. **`tick_turret_rotation` iterates id-ascending (`keys_sorted()` `:95`), not live-LOGIC order** — `unit_post`'s Facing step will move it to live-LOGIC order. Added a proof obligation that this reorder is output-neutral (each entity's desired-facing depends only on its own + target state; `barrel.set` is idempotent), pinned by `turret_sweep_retired_for_scoped_units_no_drift`.
> 5. **`refresh_mission_shadow` is at `:895`/called `:2391`** (FACTS said def `:859`/called `:2355`; combat call site `:2027`, turret `:2044`, snapshot `:2026`, smudge drain `:2226` all confirmed). Test names confirmed but lines are `:62`/`:149`/`:91`/`:174` (draft's `:61/148` were off by one). S0-S2 scaffolding absence (`src/sim/ai/`, `techno_ai.rs`, `unit_post.rs`) confirmed by Glob.
> 6. **Confirmed zero RNG in `combat/mod.rs`** (Grep: no matches) — the `unit_post_consumes_no_rng` negative criterion and the "smudge RNG is S4, not L2" framing hold. Smudge drain at `:2226` consumes `combat_result.smudge_spawn_requests` via `scenario_rng` in emission order, after the P4.5 superweapon drain at `:2212` — emission-order preservation is lockstep-critical, as the draft states.
> 7. **Slice mapping clarified:** the draft scopes L2 to **Fire+Facing only**, which is a deliberate *narrower* cut than the design doc's S3 (S3 = Fire→Facing→**HarvestBrain→Anim/Ammo→Spawn**). Kept the narrowing but flagged that the doc's S3 tests `harvest_brain_between_facing_and_ammo` and the Anim/Ammo/Spawn ordering are explicitly **deferred to a follow-up sub-slice**, not satisfied by L2.

---

#### 1. Approach choice

**Verdict: hybrid — stand up a minimal *fire+facing-only* `unit_post` host inside L2** (not gated behind the unlanded S0-S2 `object_ai_stage`/`techno_ai.rs` scaffolding, which Glob confirms is absent). Rationale unchanged from the draft and endorsed by the design doc §7.3 line 602: *"UnitClass is the safest first leaf slice … Rust already has separable movement/turret/combat phases to fold under it."* Build the narrowest host that makes the fire→facing coupling per-object, leave the global sweeps in place for out-of-scope categories (Infantry garrison fire, Aircraft, Building turrets), and shape `unit_post(sim, id)`'s signature so S4/S5 prepend the common-body steps with zero re-plumbing. Reject `dyn UnitLeaf` (invariant #2): dispatch is a `match category == Unit` site.

**The gating unknown (UNVERIFIED):** does gamemd apply hitscan damage **inline** inside `Fire_At_Target` (early kill removes a later attacker's target this tick) or **deferred** (projectile resolves later)? The current Rust is **batched** (P2 collects all damage at pre-damage HP, P4 applies). If gamemd is inline, the batch model is itself latent DRIFT and L2's absorb must thread damage-apply per-object — a deeper change. **Task 0 settles this before any flip.**

#### 2. Goal

Move UnitClass fire and turret-facing into a per-object `unit_post` step, executed in native order **`Fire_At_Target → Facing_Update`** (fire reads previous-tick facing), walking **live-LOGIC object order via `live_object_order_snapshot` (point-in-time, NO mid-pass re-read — matching current combat)**. Retire the global attacker-snapshot machinery in `tick_combat_with_fog` and the separate `tick_turret_rotation` sweep **for Unit-category entities only**. Keep the AoE/single-target/wall/bridge/terrain damage helpers as plain functions. Land **shadowed** (debug_assert agreement vs the current sweeps) before flipping, then flip with a `SNAPSHOT_VERSION` bump and a gamemd-evidence-backed golden.

#### 3. Files / surfaces (re-verified `file:line` this session)

| Surface | Path:line | Role in L2 |
|---|---|---|
| Combat sweep entry | `src/sim/combat/mod.rs:1183` `tick_combat_with_fog` | Source of the per-object **Phase-2 fire body** to extract |
| Attacker-snapshot vec build | `src/sim/combat/mod.rs:1376` | RETIRE for Unit (the batched snapshot is the substituted ordering) |
| `AttackerSnapshot` push | `src/sim/combat/mod.rs:1494` | RETIRE for Unit |
| Live-order re-sort of snapshots | `src/sim/combat/mod.rs:1533-1538` (tiebreaker `s.stable_id`, absent→`usize::MAX`) | The order the per-object walk reproduces by construction; PRESERVE the absent-from-order sink semantics |
| Cooldown/burst-delay decrement (all attackers) | `src/sim/combat/mod.rs:1392-1393` | Keep firing for ALL attackers; per-entity `saturating_sub(1)`, order-independent (prove it) |
| Turret-alignment fire gate (`barrel.current(binary_frame) == desired`) | `src/sim/combat/mod.rs:1863-1892` (read at `:1879`) | The fire-reads-previous-tick-facing gate — move into per-object fire |
| Fire body (AoE/single damage emit, FX, fire event, burst/ROF/ammo) | `src/sim/combat/mod.rs:1915-2131` | KEEP helpers; this is the **Phase-2 emission** to extract |
| **Batched damage application (Phase 4)** | `src/sim/combat/mod.rs:2190-2220` | **The batch-vs-inline decision point** (§8) — operates on aggregated `damage_events` |
| **Death handling (Phase 6, `handle_entity_deaths`)** | `src/sim/combat/mod.rs:2231-2250` | Operates on aggregated `dead_entities`; produces `despawned_ids`/`immediate_uninit_ids` consumed by deferred-delete |
| `tick_turret_rotation` | `src/sim/movement/turret.rs:82` (iterates `keys_sorted()` id-ascending `:95`; idempotent `barrel.set` `:169`) | RETIRE for Unit; absorb desired-facing read + `barrel.set_rot`/`set` into `unit_post` Facing step |
| `FacingClass` primitive | `src/sim/movement/facing_class.rs` | UNCHANGED — frame-anchored, never decrements (invariant #5) |
| `live_object_order_snapshot` (point-in-time, NO sort) | `src/sim/world/mod.rs:929` | **The order the new walk iterates** |
| `for_each_live_object` (same-pass re-read) | `src/sim/world/mod.rs:947` | **Do NOT use in L2** — would add mid-pass membership visibility combat lacks today |
| Combat call site (Phase 5) | `src/sim/world/mod.rs:2027` | Where `tick_combat_with_fog` is invoked |
| Turret call site (Phase 5, AFTER combat) | `src/sim/world/mod.rs:2044` | Where `tick_turret_rotation` is invoked |
| Combat-result smudge drain (scenario_rng, emission order) | `src/sim/world/mod.rs:2226-2239` | RNG cursor — emission order is lockstep-critical |
| Death → deferred-delete path | `src/sim/world/mod.rs:2065-2080` (`unregister_live_object` / `uninit`) | Unit kills route here — synchronous conceal/unmark, deferred slot-free (invariant #6) |
| End-of-Phase-5 flush | `src/sim/world/mod.rs:2254` `flush_pending_delete` | Slot-free boundary; do NOT free inline inside `unit_post` |
| Existing test harness | `src/sim/combat/combat_turret_facing_tests.rs:62/149/91/174` | Pin the behavior being reorganized — stay green or re-baseline with cited evidence |
| New host file | `src/sim/world/unit_post.rs` (NEW) | Hosts `unit_post(sim, id, …)` + the extracted Phase-2 fire helper |

**Mandatory pre-implementation step:** re-Read all `world/mod.rs` / `combat/mod.rs` line numbers immediately before editing — the FACTS block and design doc are stale (FACTS `~+36` low on `world/mod.rs`; design doc cites `world/mod.rs:1760..1783`/`:1778` which do NOT match the live tree).

#### 4. Step-by-step tasks

**Task 0 — Verify batch-vs-inline damage timing in Ghidra (BLOCKING gate).**
Decompile `UnitClass::Fire_At_Target` and the `TechnoClass`/`BulletClass` damage-application chain. Answer with evidence: **does the shot's damage land on target HP synchronously inside `Fire_At_Target` (inline), or via a `BulletClass` that resolves on a later tick (deferred-projectile)?**
- Load schemas: `ToolSearch "select:mcp__ghidra-mcp__decompile_function,mcp__ghidra-mcp__get_function_callers,mcp__ghidra-mcp__get_function_callees"`.
- Decompile the verified `UnitClass::Fire_At_Target` address (confirm the address live before dispatch — do NOT trust a hardcoded address from a stale doc); trace `Fire_At` → bullet → `ReceiveDamage`. If deferred-projectile, the current same-tick-batch is already a simplification — preserve it, do NOT "fix" projectile timing in L2. If inline-hitscan, record that an early kill removes a later attacker's target this tick, and Task 3 must thread damage-apply per-object.
- **Output:** a one-paragraph VERIFIED verdict (inline | deferred-projectile | mixed-per-weapon) cited by the `decompile_function` call. **Do not proceed past Task 2 without it.** Default verdict until proven: **DRIFT** (treat any divergence from the batch model as a behavior change requiring evidence).

**Task 1 — Extract the per-object Phase-2 fire-emission body (no behavior change).**
Refactor the `tick_combat_with_fog` **Phase-2 loop body** (`combat/mod.rs` ~`:1559-2131`) into a free function with an explicit per-attacker signature, leaving the current sweep calling it in a loop so behavior is **bit-identical**. **Scope note (corrected):** this cleanly extracts only the *fire-decision + damage-event/FX emission* (Phase 2). Phases 3 (retarget/burst/ammo/garrison apply `:2133-2188`), 4 (damage apply `:2190`), 5 (clear attack_target `:2222`), 6 (deaths `:2231`) operate on **aggregated cross-attacker** vectors and stay batched in this task. Proposed signature (translate exact types from the live `AttackerSnapshot` at `:1494`):

```rust
/// Resolve one attacker's Phase-2 fire decision + emission for the current tick.
/// Pure w.r.t. iteration order: reads target/occupancy/rules, emits events into `out`.
/// Does NOT apply damage or handle death — that stays in the batched P4/P6.
fn resolve_attacker_fire(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    fog: Option<&FogState>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&ResolvedTerrainGrid>,
    snap: &AttackerSnapshot,
    binary_frame: u32,
    out: &mut CombatEmit,   // damage_events, fire_events, retarget_events, anim/burst/ammo, smudge_spawn_requests, remove_attack, …
) -> FireOutcome
```
`CombatEmit` bags the existing per-attacker output vecs. The KEEP-as-functions helpers (`combat_aoe::apply_aoe_damage`, single-target, wall/bridge/terrain) are called inside unchanged.
- **Verify:** `cargo test -p vera20k combat` — all existing combat tests green (pure extraction). Safety net for Task 2.

**Task 2 — Stand up the minimal `unit_post` host (SHADOW, behind a flag).**
Add `unit_post(sim, id, rules, binary_frame, out)` (new `src/sim/world/unit_post.rs`). Fire + facing only, native order:
```rust
/// Per-object UnitClass post-Foot step: Fire_At_Target → Facing_Update.
/// Fire reads PREVIOUS-tick barrel facing (barrel.current(binary_frame));
/// Facing_Update then rotates the barrel toward the target for next tick.
fn unit_post(sim: &mut Simulation, id: u64, rules: &RuleSet, binary_frame: u32, out: &mut CombatEmit) {
    // 1. FIRE — reuse Task-1 helper with this object's snapshot. Cooldown/burst
    //    decrement here (Task 2a). Emits damage_events into `out` (NOT applied here in shadow).
    // 2. FACING_UPDATE — the tick_turret_rotation per-entity work:
    //    desired = facing_toward_lepton(target) | body_facing_to_turret(idle);
    //    barrel.set_rot(rot); barrel.set(desired, binary_frame);  // idempotent
}
```
Drive it from a new shadow walk inside Phase 5, gated on `const L2_UNIT_POST_AUTHORITATIVE: bool = false;`. In shadow mode the host iterates **`sim.live_object_order_snapshot()`** (point-in-time — NOT `for_each_live_object`), computes fire+facing into a **scratch** `CombatEmit`, and `debug_assert`s the per-Unit `damage_events` and `barrel` destination match the legacy sweeps' output. Legacy `tick_combat_with_fog` + `tick_turret_rotation` remain authoritative.

**Task 2a — Cooldown/burst-delay decrement placement.** Legacy decrement (`:1392-1393`) ticks all attackers in snapshot order; it is per-entity `saturating_sub(1)` with no cross-entity dependency. Decrement inside `unit_post` per object (locality), and **prove** order-independence in `unit_cooldown_decrement_order_independent` before flipping.

- **Verify:** `cargo check -p vera20k`; run a `debug_assertions` skirmish replay test — the shadow agreement assert must not fire. If it fires on a non-interleave case, log the attacker/tick and decide intended-interleave vs bug.

**Task 3 — Flip authority for Unit (retire the split sweeps for Unit only).**
After Task 0's verdict and Task 2 shadow agreement:
- In `tick_combat_with_fog`: skip Unit-category attackers in the snapshot build (`:1377` loop) so the global sweep no longer fires Units. Infantry/Aircraft/Building/garrison stay on the global path until their slices.
- In `tick_turret_rotation`: skip Unit-category turreted entities (`:96` loop), leaving Aircraft/Building turrets on the sweep.
- Promote `unit_post` to authoritative: it emits the real `CombatEmit` for Units, **merged into `combat_result`** so the existing P4 damage-apply / P6 death / smudge drain consume Unit fire events too.
- **Damage application (per Task 0):** if **deferred-projectile/batched matches gamemd**, route Unit `damage_events` into the existing aggregated P4 batch unchanged (simplest, no model change). If **inline**, `unit_post` must apply HP damage at fire time and enqueue deaths via the deferred-delete path — a deeper change; do **not** do this without Task 0 evidence.
- **Emission order:** Unit fire events + their `smudge_spawn_requests` emit in live-LOGIC order so the `scenario_rng` smudge drain at `:2226` advances identically. The legacy path sorts snapshots into live order (`:1533`); the per-object walk *is* live order — preserved by construction, but assert it (`smudge_emission_order_unchanged`).
- **Verify:** `cargo test -p vera20k` (full sim) + the named tests in §6.

**Task 4 — `SNAPSHOT_VERSION` bump + golden re-baseline.**
The flip changes the turret-vs-fire interleave on the first-acquisition tick (doc §S3 line 767: player-visible, hash-affecting). Bump `SNAPSHOT_VERSION`, regenerate the replay golden, cite Task 0's gamemd evidence in the commit/plan. `FacingClass` is already hashed (`world_hash.rs`); L2 adds no new hashed field.

#### 5. Authoritative / shadow state

| State | Before L2 | During L2 (Task 2) | After flip (Task 3) |
|---|---|---|---|
| Unit fire decision + damage emission | Authoritative via `tick_combat_with_fog` (batched P2 emit, id-ascending cooldown / live-order P2) | Shadow `unit_post` (debug_assert vs sweep) | **Authoritative** via `unit_post` per-object live-order P2; **P4/P6 damage-apply/death stay batched unless Task 0 says inline** |
| Unit turret facing | Authoritative via `tick_turret_rotation` (id-ascending sweep) | Shadow `unit_post` Facing step | **Authoritative** via `unit_post` (after fire, same pass, **now live-LOGIC order** — output-neutral, proven) |
| Infantry/Aircraft/Building fire + facing | Authoritative via global sweeps | Unchanged | **Unchanged** (out of scope — sweeps retained) |
| AoE/single/wall/bridge/terrain damage helpers | Functions called by sweep | Functions called by `unit_post` (shadow) | Functions called by `unit_post` — KEEP |
| `FacingClass` primitive | Hashed, frame-anchored | Unchanged | Unchanged |
| HarvestBrain / Anim-Ammo / Spawn ordering | Existing paths | Unchanged | **Unchanged — deferred to a follow-up sub-slice of doc S3** |
| Per-object AI shell common body | Does not exist | Does not exist | Does not exist — `unit_post` is fire+facing only, order-compatible with future S4 prepend |

#### 6. Named acceptance tests (live in `src/sim/combat/combat_turret_facing_tests.rs`)

- **`unit_ai_fire_then_facing_update_order`** (slice-spec name; = doc's `fire_then_facing_then_ammo_order`, narrowed to fire+facing) — a scoped Unit's `unit_post` runs Fire before Facing in the same pass: assert the fire decision reads `barrel.current(binary_frame)` (previous-tick value) and `barrel.set` (rotation begin) happens *after* fire resolution within the same `unit_post` call.
- **`unit_fire_reads_previous_tick_facing`** — on the tick a target is first assigned, the Unit fires using last-tick facing and rotation only *begins* this tick; no same-tick rotate-and-fire. (Descendant of `one_tick_acquisition_latency_first_tick_no_fire` — keep that green.)
- **`unit_cooldown_decrement_order_independent`** — two Units at different live-order positions decrement cooldown/burst-delay to identical values regardless of walk order (pins Task 2a).
- **`turret_sweep_retired_for_scoped_units_no_drift`** — removing the global turret sweep for Units leaves Aircraft/Building turret behavior bit-identical AND proves the id-ascending→live-LOGIC order change is output-neutral for Unit facing (mixed Unit+Aircraft scene; Aircraft barrel destination unchanged).
- **`combat_snapshot_retired_for_units_other_categories_unchanged`** — a Unit + garrisoned Building + Aircraft attacker scene: Building garrison fire and Aircraft fire produce bit-identical `damage_events` vs pre-L2 (only Unit fire moved).
- **`smudge_emission_order_unchanged`** — the `scenario_rng` cursor after the combat-result smudge drain is identical pre/post-flip for a multi-Unit destruction scene (pins the RNG-cursor / emission-order invariant).
- **`unit_post_consumes_no_rng`** — L2 draws zero RNG: assert `scenario_rng` and main-rng positions are unchanged by `unit_post` execution for a firing Unit (negative criterion — L2 must not add the S4 damage-particle draws).
- **Keep-green (re-baseline only with cited gamemd evidence):** `one_tick_acquisition_latency_first_tick_no_fire` (`:62`), `idle_turret_returns_to_body_facing` (`:149`), `slow_rot_takes_more_frames_to_align_than_fast_rot` (`:91`), `mid_rotation_retarget_snapshots_into_prev` (`:174`).

#### 7. Determinism / hash notes

- **RNG (invariant #7):** the combat fire path consumes **zero RNG today** (Grep-confirmed: no `rng`/`RandomRanged`/`scenario_rng` in `combat/mod.rs`). L2 keeps it that way — the damage-fire particle RNG (×2 `RandomRanged`, gated on ConditionYellow + `DamageParticleSystems`) is **design Slice S4, NOT L2**. Acceptance: the negative `unit_post_consumes_no_rng`.
- **Smudge/FX cursor:** `scenario_rng` drains AFTER combat from `combat_result.smudge_spawn_requests` in **emission order** (`world/mod.rs:2226-2239`, after the P4.5 superweapon drain `:2212`). The per-object live-order walk preserves emission order by construction (legacy sorts into the same live order at `:1533`); `smudge_emission_order_unchanged` proves it. Any reorder → DRIFT → re-baseline with evidence, do not paper over.
- **Hash:** `FacingClass` is already hashed (`world_hash.rs`); L2 adds no hashed field. The Task-3 interleave change *does* move hashed state (barrel destination / HP on the first-acquisition tick) → `SNAPSHOT_VERSION` bump + golden (Task 4).
- **Live-order tiebreaker:** legacy sorts absent-from-`live_order` attackers to `usize::MAX` then by `stable_id` (`:1535`). The per-object walk iterates `live_object_order_snapshot()` directly, so limbo/absent objects simply are not in the snapshot and do not fire — matching the legacy sink.
- **Order-change audit (corrected):** `tick_turret_rotation` currently runs **id-ascending** (`keys_sorted()` `:95`); `unit_post` runs **live-LOGIC**. This is output-neutral for facing because each entity's desired-facing reads only its own + its target's position and `barrel.set` is idempotent — but it is a real order change, so it must be **proven** by `turret_sweep_retired_for_scoped_units_no_drift`, not assumed.
- **Timers (invariant #5):** `FacingClass` is frame-anchored and never decrements; `unit_post` only calls `set`/`set_rot`.
- **Phase order (invariant #3):** Phase 5 stays put; `unit_post` runs *within* Phase 5 in place of the two sweeps for Units. No other phase moves. The full `object_ai_stage` AI-phase relocation is S0/future, not L2.
- **Deferred death (invariant #6):** Unit kills route through `combat_result.despawned_ids` → `unregister_live_object`/`uninit` (`world/mod.rs:2065-2080`), slot-freed at `flush_pending_delete` (`:2254`) — synchronous conceal/unmark, deferred slot-free. `unit_post` must NOT free slots inline.

#### 8. Dependencies + risk / do-not-do

**Dependencies.**
- **Task 0 (Ghidra batch-vs-inline verdict) is a hard blocker** for Task 3's damage-application shape. The current Rust is **batched** (verified: P2 collects all `damage_events` at pre-damage HP `:1946`, P4 applies `:2203`, P6 kills `:2232`). Do not flip without the verdict.
- **No dependency on S0-S2 scaffolding** (intentionally — L2 stands up its own minimal fire+facing host; `src/sim/ai/`/`techno_ai.rs`/`unit_post.rs` confirmed absent). `unit_post`'s signature is shaped so S4/S5 prepend common-body steps without re-plumbing.
- Depends on the live `AttackerSnapshot` field set (`:1494`) and `FacingClass` API — both verified present.

**Highest risks (flagged per slice-spec).**
1. **Batch-vs-inline damage timing (UNVERIFIED, dominant risk — corrected to note it's deeper than the draft implied).** The combat result is a 6-phase batch where P4/P6 operate on aggregated cross-attacker vectors. A per-object `fire→apply→fire` loop lets an early kill remove a later attacker's target this tick. The doc asserts *ordering* only, never batch-vs-inline. If gamemd is inline, L2 must thread damage-apply + death into the per-object step (not just "fold the sweeps"). **Default verdict: DRIFT until proven.** Task 0 settles it.
2. **RNG-order (lockstep desync, not cosmetic).** L2 adds no RNG, but if fire-event / smudge-request emission order changes, the post-combat `scenario_rng` smudge cursor shifts and desyncs a multiplayer match. Preserve emission order (live-LOGIC) or re-baseline with evidence.
3. **First-acquisition interleave (player-visible, hash-affecting).** Coupling fire-before-facing per-object changes turret lag / first-shot timing on the acquisition tick vs the two-sweep model. This is the intended DRIFT-fix; requires the `SNAPSHOT_VERSION` bump + golden (Task 4). Frequency: fires every time any turreted Unit acquires a new target — i.e. constantly in any skirmish, so this is the headline hash change, fix-and-rebaseline first.

**Do NOT do.**
- Do NOT retire the global sweeps for Infantry / Aircraft / Building — only Unit is in scope. Garrison building fire and Aircraft fire stay on `tick_combat_with_fog`; Aircraft/Building turrets stay on `tick_turret_rotation`.
- Do NOT use `for_each_live_object` (same-pass re-read) in L2 — combat uses the point-in-time `live_object_order_snapshot` today; switching is a latent behavior change. Defer re-read to the spawn-owning shell.
- Do NOT add the S4 damage-particle RNG draws, HarvestBrain/Anim-Ammo/Spawn ordering, passive/opportunity acquisition, or any common-body pre/post-mission step — those are S4/S5 (and the HarvestBrain/Anim/Ammo/Spawn tail of doc S3).
- Do NOT change projectile/hitscan timing or the AoE/single-target damage math — KEEP those helpers and call them from the per-object path.
- Do NOT trust the FACTS-block or design-doc `world/mod.rs` line numbers (FACTS ~+36 low; doc cites the wrong `:1760..1783`/`:1778`); re-Read before editing.
- Do NOT free entity slots inline inside `unit_post`; route deaths through the deferred-delete path.
- Do NOT introduce `dyn`/trait-object dispatch; `unit_post` is called from a `match category == Unit` site (invariant #2).

**Verification commands:** `cargo check -p vera20k`, then `cargo test -p vera20k combat` (targeted) and `cargo test -p vera20k` (full sim) as a **separate bounded pass** — read the literal `test result:` line before reporting. Confirm `-p vera20k` (wrong `-p` exits 101 without running).

**Files the implementation will touch (absolute):** `src/sim/combat/mod.rs`, `...\src\sim\combat\combat_turret_facing_tests.rs`, `...\src\sim\movement\turret.rs`, `...\src\sim\world\mod.rs`, `...\src\sim\world\unit_post.rs` (NEW), and the `SNAPSHOT_VERSION` / golden definition site. `FacingClass` (`...\src\sim\movement\facing_class.rs`) is read-only reference. Design doc: `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` (§7.2/§7.3/§S3, lines 575-602 / 759-773).

---

## Slice L5 — Miner/Dock Harvest under the Harvest Mission Handler

> **Review notes (what I corrected against the live tree + design §9):**
> 1. **Slice attribution was wrong in three places.** The draft repeatedly said the RadioBus shadow→authoritative flip is "mission/radio Slice 4." It is not. The live code names it **Slice 8** (`miner_dock_sequence.rs:149`: "the bus does not independently admit during this transitional slice — Slice 8 flips that") and "a later slice" (`world_hash.rs:232`). Radio-program **Slice 4** is where the FIFO `waiting_retry_queue` was *removed* (`world_hash.rs:229`), a different change. Corrected throughout to: **bus-authority flip = the registry-retire slice the code labels "Slice 8"; design §9 maps the registry→bus handoff into shell S5 ↔ mission/radio Slice 6 surfaces (`receive.rs` listed at design:822).**
> 2. **MissionCom/substate-authority flip is shell S5 (depends on mission/radio Slice 6)** — design lines 801–824, summary table line 909 (`S5 … MissionCom + ReadyToCommence … depends on S4 + mission/radio Slice 6`). Draft's loose "mission/radio S5" corrected.
> 3. **Dock-registry field/type distinction (corrected).** The two registry fields are **distinct types** (verified `production_types.rs:206/235`): `sim.production.dock_reservations` is type **`RefineryDockContacts`** (the miner refinery registry, the FIFO-free V3 store this slice rides on — `miner_dock.rs:36`), while only `sim.production.depot_dock_reservations` is type **`DockReservations`** (depot/building dock path — `miner_dock.rs:175`; `building_dock.rs:146/234/255/325`). Neither is orphaned; L5 touches the lifecycle of neither. (An earlier draft wrongly said the `DockReservations` *type* backs both fields — it does not; the miner field is `RefineryDockContacts`.)
> 4. **Cadence-gate wording.** The gate at `miner_dock_sequence.rs:1108` is `unload_accumulator.saturating_mul(10) < config.unload_tick_interval` (default 144). Draft's shorthand "`acc*10 < 144`" is only true at the default; corrected to cite the config field so a non-default `unload_tick_interval` is not silently assumed.
> 5. **Verified all keep-green test line numbers** — every one is exact (`:764/2849/3224/3444/3496/4245/4609/5641/5722`). `receive.rs` handshake tests span `:213–271` (draft said `:213-270`; corrected). `derived_mission` priority and the `(MissionType::Harvest, miner.state as u8)` mapping confirmed at `game_entity.rs:482-509`.
>
> Everything else in the draft (thin-wrapper approach, shadow-first shape, hash/RNG/timer invariants, the §0.2 call-path correction) verified correct against the live tree and kept.

---

**Status:** AUTHORED PLAN — read-only research; no Rust written this session. Source of truth: `TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §9 (slice ladder), §5 (radio invariants R1/R2), §8 (absorb table) + the live Rust tree (every `file:line` re-verified this session). Default verdict on unproven equivalence is DRIFT.

**Scope.** Route the existing miner FSM + refinery-dock choreography so they dispatch as the **Harvest mission handler under the Techno-AI shell**, consuming the already-landed `RadioBus` (`radio/receive.rs`) and `MissionCom` shadow. Preserve the unload-accumulator ordering, the 14.4-tick deposit cadence, the accepted-cell offset, the Ore→Gem slot order, and the refinery-owner credit identity. **No authority flips this slice** — the bus stays the hashed lockstep shadow it already is; the registry stays the admission decision source.

---

##### 0. Critical corrections to the grounding notes before any work starts

1. **Both dock registries are live for L5 — do NOT delete or repurpose either.** They are **distinct types** (`production_types.rs:206/235`): `sim.production.dock_reservations` is type **`RefineryDockContacts`** (the refinery registry the miner FSM admits through — V3, FIFO-free), and `sim.production.depot_dock_reservations` is type **`DockReservations`** (the depot/building dock path, `building_dock.rs:146/234/255/325`). The §8 "retire FIFO" row refers to the non-native FIFO that was already removed (`waiting_retry_queue`, gone in radio Slice 4 per `world_hash.rs:229`) and to `AirfieldDocks.queues` — **not** to either registry field. **L5 touches the lifecycle of neither `dock_reservations` (`RefineryDockContacts`) nor `depot_dock_reservations` (`DockReservations`) nor `AirfieldDocks`.**

2. **The miner tick is not called from `world/mod.rs` directly.** Verified chain: `advance_tick` → `tick_resource_economy` (`production_economy.rs`, calls `tick_miners_with_overlay_registry` at `:21`) → `process_miner` (`miner_system.rs:234`) → `match snap.miner.state` → `handle_dock_sequence` (`miner_dock_sequence.rs:714`). Any "route the Harvest handler from an AI phase at `world/mod.rs:NNNN`" instruction is stale. L5 lands inside the existing "scatter + production + repairs + **docks** + ore" phase bracket, at the `tick_miners` call site — **it does not move a phase.**

---

##### 1. Approach (brainstorm step)

**Chosen: a thin dispatch-shell wrapper that re-expresses the existing `match miner.state` as the Harvest mission handler, leaving the FSM bodies in place.** The miner FSM already iterates `live_object_order_snapshot()` (no sort, `miner_system.rs:106`) — the verified native LogicClass order — and `derived_mission()` already maps the whole loop to `(MissionType::Harvest, miner.state as u8)` (`game_entity.rs:482-485`). The harvest behaviour is *already* the Harvest mission in everything but name; L5 adds the **dispatch seam**: a `harvest_mission_step(...)` entry the Techno-AI shell calls, which internally runs the existing `process_miner` (and thus `handle_dock_sequence`). Function bodies move by reference, not by rewrite.

**Rejected: a full rewrite of the dock FSM into a `MissionCom.substate`-driven state machine.** That is the eventual end-state (design §7.4 / shell S5 ↔ mission/radio Slice 6), but doing it in L5 conflates two flips — (a) routing under the shell and (b) making `MissionCom.substate` the authoritative cursor — into one hash-affecting change across ~30 dock tests. Invariant #4 (shadow-first) requires the routing seam land first, proven behavior-identical, before any cursor-authority flip. The thin wrapper lands the seam with **zero observable change**; the substate flip is a clean follow-up gated by its own golden.

---

##### 2. Goal

Make the miner harvest + refinery dock choreography **dispatch through the Harvest mission handler seam** under the Techno-AI shell, consuming `MissionCom` (shadow) and the `RadioBus` (shadow), with **no observable change** to dock handshake, deposit cadence, slot order, credit identity, RNG, or hash. Establish named acceptance tests pinning the dock handshake and the 14.4-tick deposit cadence so the later substate-authority (shell S5) and bus-authority (code-labeled "Slice 8") flips have a baseline. Flag the bus-authority flip as a gated cross-slice dependency.

---

##### 3. Files / surfaces (exact file:line, re-verified this session)

| Surface | file:line | Role in L5 |
|---|---|---|
| `tick_miners_with_overlay_registry` | `miner_system.rs:98` | Snapshot pass; iterates `live_object_order_snapshot()` (`:106`), empty-order fallback to `keys_sorted()` (`:108-115`). The dispatch-shell call site. |
| `process_miner` + `match snap.miner.state` | `miner_system.rs:234`; Dock arm `:258-260` calls `handle_dock_sequence` | Becomes the body reached *through* `harvest_mission_step`. |
| Phase-2 loop | `miner_system.rs:171-173` | Call site changes from `process_miner(...)` → `harvest_mission_step(...)`. |
| `handle_dock_sequence` | `miner_dock_sequence.rs:714`; `match dock_phase` `:731-800`; `tick_unload_accumulator` `:802` (after `phase_unloading` `:792`) | The dock sub-FSM. PRESERVE this ordering. |
| `tick_unload_accumulator` | `miner_dock_sequence.rs:194` | Accumulator increment AFTER phase sample (sample-before-increment). PRESERVE. |
| `phase_unloading` cadence gate | `miner_dock_sequence.rs:1108` (`unload_accumulator.saturating_mul(10) < config.unload_tick_interval`) | The 14.4-tick gate. PRESERVE byte-for-byte. |
| `SLOT_ORDER` Ore→Gem; per-slot atomic drain; refinery-owner credit; purifier bonus | `miner_dock_sequence.rs:1118` / `:1131-1138` / `:1147` / `:1162-1173` | PRESERVE. |
| `unload_tick_interval` (default 144 = 14.4 ticks) | `miner/mod.rs:175` (field), `:209` (default), `:241` (constructed) | PRESERVE the constant and the `*10`/`< unload_tick_interval` math. |
| Registry (authoritative) | `RefineryDockContacts` / `hello_or_wait` (`miner_dock.rs`); `sim.production.dock_reservations.hello_or_wait` | FSM admission decision source TODAY. |
| Registry FSM sites | `miner_dock_sequence.rs:824-840` (Approach), `:871-928` (MissionEnter `mark_contact_entered` `:925`) | Where `hello_or_wait` is consulted and contacts marked. |
| Bus shadow helpers | `bus_hello` `miner_dock_sequence.rs:150`; `bus_enter_dock` `:172`; `bus_break` `:184` | Maintain `radio_contacts`/`dock_entered_with` in lockstep, gated on the registry decision (`:828`, `:875`). |
| Bus receiver | `refinery_receive` `radio/receive.rs:56`; `refinery_hello` `:116` (owner-equality + capacity + idempotent + no-evict) | Full gamemd admission; ready to be authoritative when the registry-retire slice un-gates it. |
| `MissionCom` shadow + `derived_mission` | `game_entity.rs:482-509` (priority miner→aircraft→dock→attack→move→idle; harvest→`(Harvest, state)` at `:485`) | Already maps harvest. KEEP consistent. |
| Hash folds — registry | `world_hash.rs:223` (`contacts`), `:233` (`contact_entered`), `:237` (`on_pad`) | Folded; transitional mirror, retired in the registry-retire slice (NOT here). |
| Hash folds — bus | `world_hash.rs:483` (`radio_contacts.hash_fold`) / `:486` (`dock_entered_with`) | Bus state ALSO folded. Both stores hashed today (intentional lockstep duplicate). |
| New seam file | `src/sim/miner/harvest_mission.rs` (NEW) | Houses `harvest_mission_step` + the shadow MissionCom-substate `debug_assert`. |

---

##### 4. Step-by-step tasks

###### Task 1 — Introduce the Harvest mission handler seam (`harvest_mission_step`)

**File:** new `src/sim/miner/harvest_mission.rs`; declared `mod harvest_mission;` in `miner/mod.rs`.

**This is a function-move-by-reference, not a rewrite.** Add one dispatch entry the Techno-AI shell (current or future) calls per snapshot, mirroring the `process_miner` shape so the move is mechanical:

```
pub(super) fn harvest_mission_step(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    snap: &mut MinerSnapshot,
)
```

Body: call the existing `process_miner(...)` unchanged. **Do not inline or reorder the `match snap.miner.state` arms.** The seam's only job this slice is to be the named entry point the shell dispatches to for `MissionType::Harvest`, so `process_miner` is reached *through the mission seam*.

**Wiring:** at `miner_system.rs:171-173`, the Phase-2 loop calls `harvest_mission_step` (which calls `process_miner`). No order change — same per-snapshot loop over `live_object_order_snapshot()`.

**Why shadow:** the seam adds indirection only; `derived_mission()` already classifies these entities Harvest; `miner.state` remains the FSM cursor; no new authority. `cargo check` proves it compiles; the existing miner suite proves zero behavior change.

###### Task 2 — Assert the MissionCom Harvest mapping at the seam (shadow agreement)

**File:** `harvest_mission.rs`.

Inside `harvest_mission_step`, add a **`debug_assert`-gated** consistency check (compiled out in release, never hashed) that the entity's `derived_mission()` equals `(MissionType::Harvest, snap.miner.state as u8)`. This pins the shadow MissionCom substate to the FSM cursor so the *later* substate-authority flip (shell S5 ↔ mission/radio Slice 6) has a proven invariant. Reads existing fields only; writes nothing.

> Do NOT make `MissionCom.substate` the FSM cursor in L5, and do NOT fold `MissionCom` into the hash. That is shell **S5** (depends on mission/radio Slice 6; design lines 801–824, summary table line 909). MissionCom is unhashed today (`game_entity.rs`, serde-skip). L5 only *asserts* the mapping holds.

###### Task 3 — Preserve the unload-accumulator ordering (no code change; pinning test)

`miner_dock_sequence.rs:792` (`phase_unloading`) then `:802` (`tick_unload_accumulator`) is already native (sample-before-increment). **No code edit.** Task 6 adds the named test so the seam refactor cannot silently reorder it.

###### Task 4 — Confirm cadence/slot/credit invariants untouched (no code change)

The 14.4-tick gate (`:1108`, `*10 < config.unload_tick_interval`), `SLOT_ORDER = [Ore, Gem]` (`:1118`), per-slot atomic drain (`:1131-1138`), refinery-owner credit (`:1147`), and per-slot purifier bonus (`:1162-1173`) are preserved verbatim. **No code edit.** Task 6 pins them.

###### Task 5 — Document the bus-authority dependency (no behavior change)

**File:** `harvest_mission.rs` `//!` header + an inline comment at the `bus_hello` gate sites (`miner_dock_sequence.rs:828` Approach, `:875` MissionEnter).

Record, in `//!` and at the gates, that the registry (`dock_reservations`) remains the admission decision source in L5; the bus (`radio_contacts`/`dock_entered_with`) is the lockstep shadow; and the authority flip (un-gating `bus_hello` so `refinery_hello` at `receive.rs:116` independently admits, then retiring the registry mirror + its hash folds at `world_hash.rs:223/233`) is the **registry-retire slice the code labels "Slice 8"** (`miner_dock_sequence.rs:149`; design §9 lists `receive.rs` as a shell-S5/mission-radio-Slice-6 surface at design:822). Comment-only; does not move the flip into L5.

---

##### 5. What becomes authoritative vs shadow

| State | Before L5 | After L5 | When it flips authoritative |
|---|---|---|---|
| Miner FSM cursor (`miner.state` / `dock_phase`) | authoritative free-FSM | **authoritative, now reached via the Harvest seam** | stays `miner.state`; substate-authority is shell S5 |
| `MissionCom` (`current`/`substate`) | shadow (unhashed, re-derived) | shadow (now `debug_assert`-consistent at the seam) | shell **S5** ↔ mission/radio Slice 6 |
| Registry `dock_reservations` (`contacts`/`contact_entered`/`on_pad`) | **authoritative** admission/entered/pad source | **still authoritative** (unchanged) | registry-retire slice (code: "Slice 8") |
| `RadioBus` (`radio_contacts`/`dock_entered_with`) | shadow, lockstep mirror, hashed | shadow, lockstep mirror, hashed (unchanged) | registry-retire slice (un-gate `bus_hello`) |

**Net: L5 flips nothing to authoritative.** It is a pure routing seam + pinning tests — the correct shape for a shadow-first absorb slice.

---

##### 6. Named acceptance tests (exact `fn` names)

Add to `miner_tests.rs` (existing harness with registry/bus fixtures):

1. **`harvest_seam_dispatch_matches_direct_process_miner`** — run a full harvest→dock→unload→depart cycle through `harvest_mission_step` and assert per-tick `miner.state`, `dock_phase`, credits, and `radio_contacts`/`dock_entered_with` are bit-identical to a recording made by calling `process_miner` directly (the seam is observably a no-op).
2. **`harvest_seam_derived_mission_is_harvest_each_tick`** — across the cycle, `entity.derived_mission() == (MissionType::Harvest, miner.state as u8)` every tick (the Task-2 invariant as a real test).
3. **`dock_handshake_hello_enter_over_seam`** — through the seam: HELLO admission → `MissionEnter` → accepted-cell move → `mark_contact_entered` (`:925`) + `dock_entered_with == Some(ref)` follow the verified phase order; a capacity-1 second miner gets `Waiting`/NEGATORY with no eviction and no FIFO (pins the §8 V3 no-wait-queue contract).
4. **`deposit_cadence_14_4_ticks_over_seam`** — pins `unload_accumulator.saturating_mul(10) < config.unload_tick_interval`: first ore-slot drain lands at the 15–16 tick window (mirrors `dock_first_slot_drain_waits_one_unload_interval` `:4609`), unchanged after routing through the seam.
5. **`unload_accumulator_sample_before_increment`** — pins `tick_unload_accumulator` (`:802`) running AFTER `phase_unloading` samples the accumulator (`:1108`): enter `Unloading` at a known frame, single-step, assert the value the unload phase reads is the *pre-increment* value (resolves the §8 NEEDS-PROOF on this ordering).
6. **`deposit_slot_order_ore_then_gem_over_seam`** — mixed cargo drains Ore first, then Gem; one `BaleDepositEvent` per slot; credit to the refinery owner — unchanged through the seam.
7. **`harvest_seam_preserves_bus_registry_lockstep`** — extends `refinery_cycle_over_radio_bus_matches_registry_cadence` (`:5641`): through the seam, `radio_contacts` mirrors `dock_reservations.has_contact` and `dock_entered_with` mirrors `has_contact_entered` every tick (routing did not desync the two stores).

**Keep-green (must still pass unchanged):** `dock_first_slot_drain_waits_one_unload_interval` (`:4609`), `credits_arrive_per_slot_during_unload` (`:764`), `unloading_emits_one_event_per_slot_drain` (`:3496`), `accepted_cell_arrival_sets_contact_entered_then_0x15_starts_unload_fsm` (`:3444`), `hello_before_mission_enter_then_can_dock_move` (`:2849`), `two_miners_waiter_after_releaser_approach_hello_only` (`:3224`), `full_dock_cycle_war_miner` (`:4245`), `full_unload_credits_unchanged_over_bus` (`:5722`), and the `receive.rs` handshake tests (`:213-271`).

---

##### 7. Determinism / hash notes

- **Iteration order unchanged:** the seam keeps the existing `live_object_order_snapshot()` pass (`miner_system.rs:106`, no sort) = verified native LogicClass order; the empty-order `keys_sorted()` fallback (`:108-115`) is preserved for direct-setup unit tests.
- **Hash unchanged:** L5 adds/removes no hashed field. Both the registry folds (`world_hash.rs:223/233/237`) and the bus folds (`:483/486`) stay as-is — the lockstep-duplicate fold is intentional and is retired only by the registry-retire slice ("Slice 8"), **not here**. **No `SNAPSHOT_VERSION` bump in L5** (it is a shadow slice; design line 914 reserves the bump for authority flips).
- **RNG unchanged:** the seam adds no RNG draw and reorders none. The dock path's only RNG draws — `schedule_enter_retry` jitter via `miner_jitter_rng()` (`miner_dock_sequence.rs:82-86`) and the mission-deploy/unload jitter — are consumed at the same per-object position because the per-snapshot dispatch order and arm bodies are byte-identical.
- **Timers:** `unload_cluster_timer.arm(sim.binary_frame, …)` (`:216`), `dock_enter_retry.arm` (`:86`), and `mission_deploy_timer.arm` (`:98`) remain frame-anchored; no decrement introduced. The 14.4-tick gate is a frame-anchored accumulator compare, untouched.
- **Deferred death:** the `cleanup_dead` release of dock reservations (`miner_system.rs:168`, keyed on `!dying` stable_ids) runs before the dispatch pass, unchanged; the seam adds no synchronous free.

---

##### 8. Dependencies, risk, and do-not-do

**Cross-slice dependency — the bus-authority flip (gated, NOT in L5).** Verdict: the RadioBus shadow→authoritative flip is **not** owned by L5; it is the registry-retire slice the live code labels **"Slice 8"** (`miner_dock_sequence.rs:149`) / "a later slice" (`world_hash.rs:232`), and design §9 routes its surfaces (`receive.rs`) through shell **S5 ↔ mission/radio Slice 6** (design:822). Evidence: the bus is landed-but-uncommitted (`receive.rs` untracked; `radio/mod.rs`/`game_entity.rs`/`miner_dock_sequence.rs` modified-uncommitted); the FACTS block states "bus-authority … are NOT landed." Performing the bus flip inside L5 would conflate two hash-affecting changes. **L5 depends on that slice, does not perform it.** Concretely:
- **If the registry-retire slice lands first:** L5's seam reads whichever store is authoritative — no code change to the seam (the FSM sites call `hello_or_wait`; the retire slice changes those, not the seam).
- **If it lands after (the default in this plan):** L5 keeps the existing lockstep shadow. Test 7 guarantees the shadow stays consistent so the later flip is observably a no-op.
- **User decision required** only if the user wants to merge the two flips into L5; default is to keep them separate (shadow-first, invariant #4).

> Correction note: the draft's "mission/radio Slice 4" attribution for this flip is unsupported — radio Slice 4 *removed* the FIFO `waiting_retry_queue` (`world_hash.rs:229`), it did not retire the registry mirror.

**Risk: low.** The seam is an indirection over an already-Harvest-classified, already-native-ordered loop. Blast radius is the ~30 dock tests, all keep-green, all exercising the unchanged FSM bodies.

**Do NOT do in L5:**
- Do NOT delete or repurpose either registry — `dock_reservations` (type `RefineryDockContacts`) or `depot_dock_reservations` (type `DockReservations`); both live — §0.1.
- Do NOT touch `AirfieldDocks` (separate, already FIFO-free).
- Do NOT un-gate `bus_hello` / retire the registry / drop the registry hash folds (registry-retire "Slice 8").
- Do NOT make `MissionCom.substate` the FSM cursor or fold `MissionCom` into the hash (shell S5 ↔ mission/radio Slice 6).
- Do NOT move any `advance_tick` phase, reorder the `match dock_phase` arms, or move `tick_unload_accumulator` relative to `phase_unloading`.
- Do NOT change `unload_tick_interval`, the `*10`/`< unload_tick_interval` gate, `SLOT_ORDER`, the per-slot atomic drain, or the refinery-owner credit identity.

**Invariant compliance:** (1) sim-only, no render/ui/audio/net deps; (2) dispatch is the existing `match miner.state`/`match dock_phase` + `Option<Miner>` — no trait/dyn/vtable/COM added; (3) `advance_tick` phase order preserved; (4) shadow-first — L5 flips nothing authoritative, adds `debug_assert` agreement only, no hash movement, no `SNAPSHOT_VERSION` bump; (5) timers stay frame-anchored (`.arm(binary_frame, …)`), never decrement; (6) deferred death untouched; (7) RNG positions unchanged; (8) named tests (§6) pin gamemd dock order + cadence before any later flip.

---

**Files the implementation will touch (absolute):**
- `src/sim/miner/harvest_mission.rs` (NEW)
- `src/sim/miner/mod.rs` (add `mod harvest_mission;`)
- `src/sim/miner/miner_system.rs` (Phase-2 call site `:171-173`)
- `src/sim/miner/miner_dock_sequence.rs` (comment-only at `:149`/`:828`/`:875`)
- `src/sim/miner/miner_tests.rs` (new acceptance tests §6)

**Read-only references (not edited in L5):** `...\src\sim\radio\receive.rs`, `...\src\sim\radio\mod.rs`, `...\src\sim\game_entity.rs`, `...\src\sim\world\world_hash.rs`, `...\src\sim\miner\miner_dock.rs`, `...\src\sim\production\production_economy.rs`. Design doc: `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md`.

---

## Acceptance test index (consolidated)

Every NAMED test across the three slices, in landing order. Shadow tests gate their slice's landing; flip tests gate the authority flip and require a `SNAPSHOT_VERSION` bump + rebaselined golden.

**L1 — movement/locomotor**

| Test | Slice | Type | Gates |
|---|---|---|---|
| `techno_ai_shell_is_passthrough_no_hash_change` | L1.0 | shadow | `state_hash` bit-identical to pre-L1.0 |
| `techno_ai_shell_membership_matches_phase_snapshot` | L1.0 | shadow | visited ids == movement-phase visited set, same order |
| `techno_ai_shell_preserves_advance_tick_phase_order` | L1.0 | shadow | phase order unchanged (invariant #3) |
| `unit_ai_mission_dispatch_precedes_locomotor_process` | L1.1 | shadow | dispatch-before-Process observed in trace |
| `unit_move_dispatch_then_process_shadow_agrees` | L1.1 | shadow | shadow matches live phase-split every tick |
| `per_cell_arrival_callback_fires_after_mission_retry` | L1.1 | shadow | spec-mandated arrival-vs-retry ordering, asserted vs named shadow trace |
| `s1_no_hash_change_shadow` | L1.1 | shadow | `state_hash` unmoved |
| `unit_move_start_slip_matches_dispatch_then_process` | L1.2 | flip | 1-tick movement-start slip matches dispatch→process |
| `unit_c4_counter_increments_before_dispatch` | L1.2 | flip | `+0xC4` increment ordering |
| `scoped_vs_unscoped_unit_cell_contention_deterministic` | L1.2 | flip | RNG-stream order across replays |
| `drive_track_cell_crossing_cadence_unchanged_after_flip` | L1.2 | flip | cadence bit-identical pre/post flip |
| `l2_snapshot_version_bumped_golden_rebaselined` | L1.2 | flip | version bump + golden (note: name is from the slice spec; it gates the L1.2 flip) |
| `aircraft_fly_locomotor_runs_after_dispatch_shadow_agrees` | L1.3 | shadow | aircraft locomotor after dispatch |
| `teleport_special_movers_absorbed_no_drift` | L1.3 | flip | teleport/rocket/homing/droppod/parachute bit-identical |
| `tunnel_movement_remains_inert_not_absorbed` | L1.3 | guard | tunnel slot stays inert (TS legacy) |
| `aircraft_mission_snapshot_order_unchanged_in_l1` | L1.3 | guard | aircraft *mission* iteration stays BTreeMap-order |
| `phase1_ground_move_retired_no_drift` | L1.4 | flip | Phase-1 ground move retired, no drift |
| `ground_locomotor_prepass_stays_global` | L1.4 | flip | grid pre-pass runs once before object stage |
| `l4_snapshot_version_bumped_golden_rebaselined` | L1.4 | flip | version bump + golden |

**L2 — combat + turret**

| Test | Type | Gates |
|---|---|---|
| `unit_ai_fire_then_facing_update_order` | flip | Fire-before-Facing within one `unit_post` |
| `unit_fire_reads_previous_tick_facing` | flip | fire reads last-tick facing; rotation only begins |
| `unit_cooldown_decrement_order_independent` | shadow→flip | cooldown decrement order-independent (Task 2a) |
| `turret_sweep_retired_for_scoped_units_no_drift` | flip | id-ascending→live-LOGIC reorder output-neutral; Aircraft/Building unchanged |
| `combat_snapshot_retired_for_units_other_categories_unchanged` | flip | only Unit fire moved; garrison/Aircraft bit-identical |
| `smudge_emission_order_unchanged` | flip | `scenario_rng` smudge cursor identical pre/post (RNG-order invariant) |
| `unit_post_consumes_no_rng` | shadow | negative criterion — L2 draws zero RNG |
| `one_tick_acquisition_latency_first_tick_no_fire` (`:62`) | keep-green | re-baseline only with cited evidence |
| `idle_turret_returns_to_body_facing` (`:149`) | keep-green | " |
| `slow_rot_takes_more_frames_to_align_than_fast_rot` (`:91`) | keep-green | " |
| `mid_rotation_retarget_snapshots_into_prev` (`:174`) | keep-green | " |

**L5 — miner/dock harvest** (all in `miner_tests.rs`; L5 flips nothing — these pin the seam as a no-op and baseline the later flips)

| Test | Type | Gates |
|---|---|---|
| `harvest_seam_dispatch_matches_direct_process_miner` | shadow | seam observably a no-op vs direct `process_miner` |
| `harvest_seam_derived_mission_is_harvest_each_tick` | shadow | Task-2 invariant `(Harvest, state)` every tick |
| `dock_handshake_hello_enter_over_seam` | shadow | HELLO→Enter→accepted-cell→entered order; capacity-1 NEGATORY, no FIFO |
| `deposit_cadence_14_4_ticks_over_seam` | shadow | first ore drain at the 15–16 tick window through the seam |
| `unload_accumulator_sample_before_increment` | shadow | accumulator sampled before increment |
| `deposit_slot_order_ore_then_gem_over_seam` | shadow | Ore-then-Gem; one event per slot; refinery-owner credit |
| `harvest_seam_preserves_bus_registry_lockstep` | shadow | bus mirrors registry every tick through the seam |
| `dock_first_slot_drain_waits_one_unload_interval` (`:4609`) | keep-green | unchanged |
| `credits_arrive_per_slot_during_unload` (`:764`) | keep-green | unchanged |
| `unloading_emits_one_event_per_slot_drain` (`:3496`) | keep-green | unchanged |
| `accepted_cell_arrival_sets_contact_entered_then_0x15_starts_unload_fsm` (`:3444`) | keep-green | unchanged |
| `hello_before_mission_enter_then_can_dock_move` (`:2849`) | keep-green | unchanged |
| `two_miners_waiter_after_releaser_approach_hello_only` (`:3224`) | keep-green | unchanged |
| `full_dock_cycle_war_miner` (`:4245`) | keep-green | unchanged |
| `full_unload_credits_unchanged_over_bus` (`:5722`) | keep-green | unchanged |
| `receive.rs` handshake tests (`:213-271`) | keep-green | unchanged |

---

## Cross-cutting invariants (recap)

The 8 hard invariants every slice in this plan respects. Each slice's §7/§8 cites how it complies; this is the consolidated checklist.

1. **sim/ has no upward deps** — `sim/` never depends on `render/`/`ui/`/`sidebar/`/`audio/`/`net/`. (L1's `TeleportVisuals` is a *borrowed handle passed in by the caller*, not a sim→render dependency.)
2. **No C++ class tree / no dyn/vtable/COM** — per-leaf dispatch is `match category` + `CapabilityFlags` + `Option<T>` only. L2 dispatches from a `match category == Unit` site; L5 keeps the existing `match miner.state`/`match dock_phase`.
3. **`advance_tick` phase order preserved** until a slice explicitly flips it. Shadow slices (L1.0/L1.1, all of L5) move NO phase; only flip sub-slices (L1.2+, L2 Task 3) move phases, each with a `SNAPSHOT_VERSION` bump + golden.
4. **Shadow-first** — new authority lands `serde-skip`, not hashed, with `debug_assert` agreement, before the authority flips. L1.0/L1.1 and all of L5 are shadow; L2 Task 2 is shadow before Task 3 flips.
5. **Frame-anchored timers never decrement** — `MissionTimer`/`FacingClass` re-arm with `start_frame`+`duration`; the `+0xC4` per-object counter is an increment, not a timer; the 14.4-tick gate is a frame-anchored accumulator compare.
6. **Deferred death** — enqueue `pending_delete`, synchronous conceal/unmark/detach, deferred slot-free. No slice frees slots inline inside its per-object step; `flush_pending_delete` stays at cleanup (L1 `:1719`, L2 `:2254`).
7. **RNG consumed at the same per-object position/gate** — the per-object walk uses the same live-LOGIC order as today so the draw sequence is preserved; emission order of FX/smudge requests (L2) is preserved so the `scenario_rng` cursor advances identically. Reordering the walk = desync = DRIFT.
8. **Every behavior-moving slice needs a NAMED acceptance test pinning gamemd order before it flips** — see the Acceptance test index. The `state_hash` is a self-replay determinism oracle, not a gamemd-parity oracle; each flip's golden is justified by cited gamemd evidence.
