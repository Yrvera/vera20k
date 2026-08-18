# Bridges Tier 2 — Phase F Orchestrator Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Cite the design doc and verification doc by section (e.g., "design ledger
> #7" or "verification doc Finding 3") in commit messages — keep binary
> addresses out of Rust code comments.

**Goal:** Wire combat damage end-to-end through the new bridge state machine
+ a new overlay-direct walker, replacing the legacy single-shot `apply_damage`
flow, so bridges destruct in gamemd-parity-correct fashion: 4-path dispatcher,
per-path BridgeStrength RNG gate, IonCannon-only retry, body/bridgehead state-
machine drivers, `DestroyBridge_*` walker drivers (new), full BlowUpBridge
cascade with two corrections (DropIn-no-despawn, debris-shape).

**Architecture:** World-layer orchestrator (`world/bridge_orchestrator.rs`)
mirrors the existing `apply_wall_damage_events` pattern. State-machine and
walker drivers stay as methods on `BridgeRuntimeState` (split into a
directory with `walker.rs` for the new walker driver). Cascade consumers
(ground kill, deck DropIn, debris, rim refresh, zone rebuild) live in the
orchestrator file alongside the dispatcher.

**Design Doc:** [docs/plans/2026-05-07-bridges-tier2-phase-f-orchestrator-design.md](2026-05-07-bridges-tier2-phase-f-orchestrator-design.md)

---

## Grounding Summary

**Docs:** Primary source is `ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` (sections §3.1, §4, §11.2, §11.3, §11.4, §11.9, §12.5, §12.6, §12.7, §12.8, §12.9 cited extensively). New verification doc `ra2-rust-game-docs/PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md` (this session) corrects/extends §4 and §3.1 — four findings change scope: direct-overlay paths reachable in normal play, no IonCannon retry on direct-overlay, Z-height gate is state-machine-only, `ApplyDamageToCell` internal dispatch is overlay-direct first.

**Ghidra (live, this session):** `Apply_area_damage @ 0x00489280`, `ApplyDamageToCell @ 0x00587180`, `DestroyBridge_High @ 0x0057CCF0` decompiled. Sub-walker addresses (`DestroyBridgeWalker_NS_High @ 0x0057CF60`, `_EW_High @ 0x0057D530`, `ApplyBridgeDestruction_NS_High @ 0x0057E7A0`, `_EW_High @ 0x0057ED00`) are documented in HIGH §5 + verification doc §4 — verified by xref but not re-decompiled this session; will re-verify when implementing the walker (Task 6/7).

**Repo pattern:** `Simulation::apply_wall_damage_events` at [world/mod.rs:701](../../src/sim/world/mod.rs#L701) is the closest existing pattern — same shape `(events, rules, registry)` → per-cell mutation + cascade in one pass. Plan mirrors this for bridges. Confirmed `bridge_explosions: Vec<InternedId>` pre-intern pattern at [app_init_helpers.rs:361-369](../../src/app_init_helpers.rs#L361-L369) — `metallic_debris` follows the same shape.

**INI keys (all parsed already):** `[CombatDamage] BridgeStrength=` ([ruleset.rs](../../src/rules/ruleset.rs)), `[CombatDamage] IonCannonWarhead=` + `[CombatDamage] C4Warhead=` ([bridge_warheads.rs](../../src/rules/bridge_warheads.rs)), `[General] BridgeExplosions=` (already pre-interned to `Simulation.bridge_explosions`), `[General] MetallicDebris=` ([ruleset.rs:435](../../src/rules/ruleset.rs#L435)), `[General] BridgeVoxelMax=` ([ruleset.rs:676](../../src/rules/ruleset.rs#L676), default 3), warhead `Wall=` (already on `Warhead.wall: bool`).

**Unknown after grounding:**
- Whether `update_adjacent_bridges` rim refresh is observable in our renderer or whether the renderer already neighbor-aware. Resolves at Task 12 implementation by reading `src/render/`.
- Exact RNG-draw count for `apply_bridge_destruction_*_high` per-cell scatter (HIGH §5 lists it as "Per-cell destruction effect (unit damage / scatter)" — precise draw count needs Ghidra spot-check at Task 7).

---

## Key Technical Decisions

- **Orchestrator lives in world layer** (`world/bridge_orchestrator.rs`), not on `BridgeRuntimeState`. **Confidence:** high. **Source:** design doc §"Chosen Approach"; mirrors `apply_wall_damage_events` pattern at [world/mod.rs:701](../../src/sim/world/mod.rs#L701).
- **`StateOutcome` flows end-to-end; `BridgeStateChange` deleted.** **Confidence:** high. **Source:** design doc §2 + ledger #21. Walker driver populates the same `StateOutcome::Collapsed` shape as state-machine drivers — cascade consumers iterate uniformly.
- **`impact_z: i32` on `BridgeDamageEvent`** carries explosion z in tile-step level units (signed for safety; entity `position.z: u8` cast to i32 at emit). **Confidence:** high. **Source:** verification doc Finding 2; arithmetic mirrors binary's `(level-2)*step+base < impact.z <= (level+1)*step+base` after dividing by step.
- **`bridge_state.rs` → `bridge_state/` directory.** **Confidence:** high. **Source:** file is 1947 LOC; adding ~350 LOC walker drivers + ~80 LOC dispatch helpers without splitting violates the 600-line guideline.
- **Direct-overlay path uses single-shot, NOT IonCannon retry.** **Confidence:** high (verified live this session). **Source:** verification doc Finding 1.
- **`path_matches_cell(HighStateMachine)` requires `cell.overlay_byte ∉ [0xCD..0xE6]`** to enforce raw-overlay routing through the walker. **Confidence:** high (verified live this session). **Source:** verification doc Finding 3.
- **Z-height gate range is `impact_z ∈ [cell.level - 1, cell.level + 1]`** (3-level window). **Confidence:** high (derived from binary's tile-step constants by dividing both sides). **Source:** verification doc Finding 2.
- **DropIn correction: NEVER despawn bridge-deck entities on collapse.** **Confidence:** high. **Source:** HIGH §12.7, §12.9 (vanilla has no drown / fall damage / EVA).
- **Debris correction: 50% MetallicDebris no-delay + 1 always BridgeExplosion delayed 1-5 frames.** **Confidence:** high. **Source:** HIGH §11.4 step 4.
- **`update_adjacent_bridges` may stay a stub** if the renderer is neighbor-aware. **Confidence:** medium — resolves at Task 12 by reading `src/render/`. **Source:** HIGH §11.9 + design doc §3 open sub-question. **Flagged for /review-plan.**

---

## Open Questions

### Resolved During Planning

- **Where does `apply_area_damage` live?** Resolved: world layer (`world/bridge_orchestrator.rs`) per Approach 2. Source: design doc.
- **Are direct-overlay paths reachable in normal play?** Resolved: yes. Source: verification doc Finding 4.
- **Should `BridgeStateChange` survive?** Resolved: deleted. `StateOutcome` flows end-to-end. Source: design doc §"Architectural Decisions".
- **Walker outcome shape?** Resolved: reuse `StateOutcome::Collapsed { destroyed_cells, set_bridge_direction, adjacent_bridges_dirty, zones_dirty }`. Walker fills same shape as state-machine drivers. Source: design doc §"Components" + ledger #17.

### Resolved During /review-plan

- **Walker algorithm shape?** Resolved: 3-cell length-axis triple-write (NOT linear axis walk) + perpendicular sibling cascade via `apply_bridge_destruction_*_high`. Source: Ghidra `DestroyBridgeWalker_NS_High @ 0x0057CF60` + `_EW_High @ 0x0057D530` re-decompiled during /review-plan. Plan rewritten in Task 7.
- **`apply_bridge_destruction_*_high` RNG draw count?** Resolved: zero. The function is pure overlay-table-lookup + 3-cell write + DirtyScreenRect + RecalcAttributes — no `RandomRanged` calls. Source: Ghidra `0x0057E7A0` decompile during /review-plan. Earlier "deferred to Task 7" entry was unnecessary.
- **LOW walker NS/EW split?** Resolved: yes, LOW has axis split (NOT uniform). Source: Ghidra `DestroyBridge_Low @ 0x0057BAA0` decompile. Plan's Task 8 rewritten.

### Deferred to Implementation

- **`update_adjacent_bridges` active vs stub** — resolved at Task 13 by reading `src/render/` for neighbor-aware tile selection.
- **`update_adjacent_bridges` step count** — HIGH §11.9 says "walks 8 directions" but binary `MapClass__UpdateAdjacentBridges_High @ 0x00576770` not re-decompiled this session. Resolves at Task 13.
- **LOW walker exact case-mapping** — Step 0 of Task 8: decompile `DestroyBridgeWalker_NS_Low` + `_EW_Low` (addresses pending — search via xref from `0x0057BAA0`) to confirm the initial-stage and final-eligible overlay ranges. Plan's Task 8 has placeholder values flagged with `TODO`.
- **`check_bridge_neighbors_ns_high` exact overlay sets** — Task 7 Step 1 uses an EW-mirrored bit pattern; the precise overlay sets for the NS classifier need verification at `MapClass__CheckBridgeNeighbors_NS_High @ 0x0057CBE0` during implementation.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Move | `src/sim/bridge_state.rs` → `src/sim/bridge_state/mod.rs` | unchanged content; directory split |
| Create | `src/sim/bridge_state/walker.rs` | overlay-direct walker drivers (`destroy_bridge_high`, `_low`) + per-axis walker + per-cell scatter |
| Modify | `src/sim/bridge_state/mod.rs` | delete `apply_damage` + `group_hitpoints` + `strength_per_group` + `BridgeStateChange`; add `path_matches_cell` + `bridge_strength()` + `is_destroyable()` getters; expose `walker` submodule |
| Create | `src/sim/world/bridge_orchestrator.rs` | 4-path dispatcher + cascade consumers (kill ground, DropIn, debris, rim, trigger 31, zone rebuild) |
| Modify | `src/sim/world/mod.rs` | delete `apply_bridge_damage_events`, `resolve_bridge_state_changes`, `spawn_bridge_explosions`; add `metallic_debris: Vec<InternedId>` field; expose `bridge_orchestrator` submodule; replace [world/mod.rs:1338-1340](../../src/sim/world/mod.rs#L1338-L1340) calls with single orchestrator call |
| Modify | `src/sim/combat/mod.rs` | extend `BridgeDamageEvent` with `warhead_ref`, `is_ion_cannon`, `impact_z`; update 3 emit sites at lines 798, 1476, 1511 |
| Modify | `src/rules/ruleset.rs` | add `ion_cannon_warhead_id`, `c4_warhead_id` resolved fields; add `resolve_bridge_warheads()` + accessors |
| Modify | `src/app_init_helpers.rs` | call `resolve_bridge_warheads` before `BridgeRuntimeState::from_resolved_terrain`; pre-intern `metallic_debris` to `sim.metallic_debris` |
| Modify | `src/sim/bridge_specs.rs:454` | update doc-comment reference (`world::resolve_bridge_state_changes` → `world::bridge_orchestrator::apply_bridge_damage_events`) |
| Modify | `src/sim/world/world_tests.rs` | migrate 6 bridge fixtures (lines 413, 455, 500, 539, 578, 617) to new event shape + new orchestrator return type |
| Modify | `src/sim/world/world_hash.rs` | (no change — `hash_bridge_state` does not hash deleted fields, verified) |
| Modify | `src/sim/bridge_state/mod.rs` (internal tests) | migrate 3 test sites at lines ~1205, ~1227, ~1252 from `state.apply_damage(...)` to direct `cell_mut` mutation |
| Modify | `src/sim/pathfinding/core_tests.rs:588` | migrate 1 test site to direct mutation |
| Modify | `src/sim/production/production_placement_tests.rs:664` | migrate 1 test site to direct mutation |

---

## Interface Changes

**Extended public types:**
- `BridgeDamageEvent` gains 3 fields (`warhead_ref: InternedId`, `is_ion_cannon: bool`, `impact_z: i32`). All consumers mechanically updated in Task 3.
- `RuleSet` gains 2 private fields + 3 public methods (`resolve_bridge_warheads`, `ion_cannon_warhead_id`, `c4_warhead_id`).
- `Simulation` gains `pub metallic_debris: Vec<InternedId>`.

**Deleted public/crate-public types:**
- `BridgeStateChange` struct.
- `BridgeRuntimeState::apply_damage` method.
- `Simulation::apply_bridge_damage_events` (old shape) replaced by free function in `bridge_orchestrator`.
- `Simulation::resolve_bridge_state_changes` (old shape) folded into the new orchestrator.
- `Simulation::spawn_bridge_explosions` (wrong-shape) replaced by `spawn_bridge_debris` in orchestrator.

**New public methods on `BridgeRuntimeState`:**
- `width() -> u16`, `height() -> u16` (needed by walker submodule per Rust privacy)
- `bridge_strength() -> u16`
- `is_destroyable() -> bool`
- `path_matches_cell(path, rx, ry, ctx, terrain) -> bool` (`pub(crate)`)
- `destroy_bridge_high(rx, ry, terrain) -> StateOutcome`
- `destroy_bridge_low(rx, ry, terrain) -> StateOutcome`

**New `pub(super)` methods on `BridgeRuntimeState` (used by walker submodule only):**
- `destroy_bridge_walker_ns_high`, `_ew_high`, `_ns_low`, `_ew_low`
- `check_bridge_neighbors_ew_high`, `_ns_high`, `_ew_low`, `_ns_low`

**Callsite changes:**
- [world/mod.rs:1338-1340](../../src/sim/world/mod.rs#L1338-L1340) collapses from 2 calls to 1 call into `bridge_orchestrator::apply_bridge_damage_events(self, rules, &combat_result.bridge_damage_events)`.

---

## Sim Checklist

- [x] All math integer (`u8`/`u16`/`i32`/`u32`) — no f32/f64 in any new bridge logic
- [x] New state hashed — `damage_state`/`overlay_byte`/`anchor_span_id` already hashed in [world_hash.rs:218-228](../../src/sim/world/world_hash.rs#L218-L228); deleted fields (`group_hitpoints`/`strength_per_group`) NOT hashed (verified)
- [x] No render/ui/sidebar/audio/net dependencies in any new sim file
- [x] Tick ordering unchanged — orchestrator runs at the existing combat-stage callsite
- [x] BTreeMap iteration order preserved — `destroyed_cells` aggregated into `BTreeSet` before cascade
- [x] RNG draws use `SimRng::next_range_u32` / `next_range_u32_inclusive` — no thread-local randomness

---

## Risk Areas

- **Determinism (high):** 4 path-gate RNG draws in fixed dispatch order (HighSM → LowSM → LowDirect → HighDirect). Wrong order → desync. Mitigated by Task 16's RNG draw-count regression test.
- **Test migration (medium):** 11 callsites construct `BridgeDamageEvent` literals; all need new fields. Atomic update in Task 3 prevents broken builds.
- **DropIn correction breaks existing test assertions (medium):** Existing fixtures asserting "deck entity despawns when ground unwalkable" will fail. Task 14 rewrites them to assert "deck entity snaps to ground level + survives."
- **Walker correctness (medium):** New code mirroring `DestroyBridgeWalker_*_High` — verified against existing `pick_destruction_overlay` table for overlay transitions, but per-cell scatter (`apply_bridge_destruction_*_high`) RNG-draw shape needs Ghidra spot-check at Task 7.
- **Cascade ordering (low):** ground kill → deck DropIn → debris → rim → trigger 31 → zone rebuild. Wrong order produces visible glitches. Single-function orchestrator makes ordering explicit.

---

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 3 | `is_ion_cannon` pre-resolved at combat boundary | Ion Cannon must reliably destroy bridges — gate bypass + 3-retry are the parity guarantee | RNG-bypass + retry-loop both fire as in HIGH §4 |
| Task 5 | Z-height gate `impact_z ∈ [cell.level-1, cell.level+1]` for state-machine paths | Ground-level explosions don't damage elevated bridges via state machine; only deck-level + nearby do (visible in normal play) | Verification doc Finding 2 |
| Task 5 | `path_matches_cell(HighStateMachine)` rejects raw `[0xCD..0xE6]` overlay | Fresh body cells route through the walker (instant collapse) — fundamental difference from state-machine 2-hit progression | Verification doc Finding 3 |
| Task 6 | Walker NS/EW classification by overlay byte | Wrong axis pick collapses the wrong row of cells (visible) | HIGH §4 + Ghidra `DestroyBridge_High @ 0x0057CCF0` |
| Task 7 | Walker per-cell scatter via `apply_bridge_destruction_*_high` | Per-cell unit damage / scatter during bridge collapse — affects what units survive on the falling deck | HIGH §5 table + Ghidra spot-check `0x0057E7A0` |
| Task 9 | C4Warhead force-kill on ground occupants | InfDeath selection comes from C4Warhead — wrong warhead → wrong death animation (visible every collapse) | HIGH §11.4 step 1 + §12.5 |
| Task 10 | DropIn semantics: snap to ground + clear OnBridge + NEVER despawn | Vanilla units survive bridge collapse stranded; we currently despawn → wrong behavior every collapse | HIGH §12.7, §12.9 |
| Task 11 | Debris: 50% MetallicDebris no-delay + 1 always BridgeExplosion delay 1-5 frames | Wrong debris shape is visible every collapse (current code has wrong shape) | HIGH §11.4 step 4 |
| Task 11 | Debris RNG-draw order parity | 5-7 draws per cell in exact order: outer-gate, jitter×2, metallic-50%, optional metallic-slot, explosion-delay, explosion-slot | HIGH §11.4 + lockstep RNG (HIGH §12.10) |
| Task 13 | 4-path RNG draw order: HighSM → LowSM → LowDirect → HighDirect | Determinism contract; wrong order → desync | Verification doc §1 |
| Task 13 | Direct-overlay paths single-shot (NO IonCannon retry) | Verification doc Finding 1 — corrects master plan's stub | Verification doc Finding 1 |
| Task 13 | Cascade order: ground kill → deck DropIn → debris → rim → trigger 31 → zone rebuild | Wrong sub-tick order produces visible glitches (entity drops onto cell where another just died, etc.) | HIGH §11.4, §12.8 |

---

## Tasks

### Phase E — Combat boundary + warhead pre-resolution + interner pre-fill

### Task 1: `RuleSet::resolve_bridge_warheads` + accessors

**Why:** Combat reads pre-resolved interned IDs at emit. RuleSet stores warhead names from `[CombatDamage]` — world layer interns them once at sim init. Additive change; no consumers yet break.

**Files:**
- Modify: `src/rules/ruleset.rs`
- Modify: `src/app_init_helpers.rs`

**Pattern:** mirrors `Simulation.bridge_explosions` interning pattern at [app_init_helpers.rs:361-369](../../src/app_init_helpers.rs#L361-L369).

**Step 1: Add private fields + accessors to `RuleSet`**

Locate the `RuleSet` struct definition (search for `pub struct RuleSet` — around line 1100-1200). Add fields at the end of the struct body:

```rust
    // ----- Phase F: pre-resolved warhead IDs for bridge damage path. -----
    // Set at sim init via `resolve_bridge_warheads`; combat reads via accessors.
    #[serde(skip)]
    ion_cannon_warhead_id: Option<crate::sim::intern::InternedId>,
    #[serde(skip)]
    c4_warhead_id: Option<crate::sim::intern::InternedId>,
```

(If `RuleSet` does not derive serde, drop the `#[serde(skip)]` attribute. Search for `#[derive(...Serialize...)]` near `pub struct RuleSet` to confirm.)

Add accessor methods inside `impl RuleSet { ... }`:

```rust
    /// Resolve `[CombatDamage] IonCannonWarhead=` and `C4Warhead=` against
    /// the simulation interner. Call once at sim init after the warhead
    /// registry is populated and before any combat tick.
    pub fn resolve_bridge_warheads(
        &mut self,
        interner: &mut crate::sim::intern::StringInterner,
    ) {
        self.ion_cannon_warhead_id =
            Some(interner.intern(&self.bridge_warheads.ion_cannon_name));
        self.c4_warhead_id = Some(interner.intern(&self.bridge_warheads.c4_name));
    }

    /// Pre-resolved IonCannonWarhead InternedId. Panics if
    /// `resolve_bridge_warheads` was not called.
    pub fn ion_cannon_warhead_id(&self) -> crate::sim::intern::InternedId {
        self.ion_cannon_warhead_id.expect(
            "RuleSet::resolve_bridge_warheads must be called at sim init \
             before combat reads warhead IDs",
        )
    }

    /// Pre-resolved C4Warhead InternedId. Panics if `resolve_bridge_warheads`
    /// was not called.
    pub fn c4_warhead_id(&self) -> crate::sim::intern::InternedId {
        self.c4_warhead_id.expect(
            "RuleSet::resolve_bridge_warheads must be called at sim init \
             before bridge cascade fires",
        )
    }
```

**Step 2: Wire into sim init**

In `src/app_init_helpers.rs` near [line 354](../../src/app_init_helpers.rs#L354) (the `BridgeRuntimeState::from_resolved_terrain` call), insert BEFORE that call:

```rust
    if let Some(rules) = rules {
        // Pre-resolve `[CombatDamage]` warhead IDs for the bridge damage
        // pipeline. Must happen before any combat tick.
        // Note: `rules` comes through this function as `Option<&RuleSet>`;
        // resolution requires `&mut RuleSet` so the caller's RuleSet must
        // be mutable. If app_init_helpers receives `&RuleSet` only, this
        // step lives one level up (wherever the RuleSet is initially
        // populated and still mutable).
        let _ = rules; // see note above — adjust if signature is &RuleSet
    }
```

The actual insertion site depends on the existing `rules: Option<&RuleSet>` vs `Option<&mut RuleSet>` signature. Read [app_init_helpers.rs:340-360](../../src/app_init_helpers.rs#L340-L360) and choose the right insertion: if rules is `&mut`, call `rules.resolve_bridge_warheads(&mut sim.interner)` directly. If `&`, the resolver call must move up the stack to where the `RuleSet` is constructed. **Open the file first; do not assume.**

**Step 3: Add unit test in `src/rules/ruleset.rs`**

```rust
    #[test]
    fn resolve_bridge_warheads_populates_ids() {
        use crate::sim::intern::StringInterner;
        let mut rules = RuleSet::default();
        let mut interner = StringInterner::default();
        rules.resolve_bridge_warheads(&mut interner);
        let ion_id = rules.ion_cannon_warhead_id();
        let c4_id = rules.c4_warhead_id();
        // Defaults match retail rulesmd.ini ("IonCannonWH" + "Super").
        assert_eq!(interner.resolve(ion_id), "IonCannonWH");
        assert_eq!(interner.resolve(c4_id), "Super");
    }

    #[test]
    #[should_panic(expected = "resolve_bridge_warheads")]
    fn ion_cannon_warhead_id_panics_before_resolve() {
        let rules = RuleSet::default();
        let _ = rules.ion_cannon_warhead_id();
    }
```

**Step 4: Verify**

```
cargo test --lib resolve_bridge_warheads -- --nocapture
cargo build
```
Expected: tests pass, build green.

**Step 5: Commit**

```
git commit -m "rules: pre-resolve IonCannonWarhead + C4Warhead interned IDs at sim init (Phase F precondition)"
```

---

### Task 2: Pre-intern `Simulation.metallic_debris: Vec<InternedId>`

**Why:** Phase F debris cascade reads `metallic_debris` per-cell; it must be pre-interned (matching `bridge_explosions` pattern) so the per-tick path doesn't allocate. Additive; no breaks.

**Files:**
- Modify: `src/sim/world/mod.rs`
- Modify: `src/app_init_helpers.rs`

**Pattern:** identical shape to `bridge_explosions` field at [world/mod.rs:254-256](../../src/sim/world/mod.rs#L254-L256) and init at [app_init_helpers.rs:361-369](../../src/app_init_helpers.rs#L361-L369).

**Step 1: Add field to `Simulation` struct**

In `src/sim/world/mod.rs` immediately after the `bridge_explosions` declaration (around line 256), add:

```rust
    /// SHP interned IDs for bridge metallic-debris animations
    /// (from `[General] MetallicDebris=`). Pre-interned at sim init so the
    /// per-cell debris cascade in `bridge_orchestrator::spawn_bridge_debris`
    /// runs allocation-free.
    #[serde(skip)]
    pub metallic_debris: Vec<InternedId>,
```

Add to the `Simulation::new` initializer (around line 360 where `bridge_explosions: Vec::new(),` lives):

```rust
            metallic_debris: Vec::new(),
```

**Step 2: Update `rebuild_caches_after_load`**

At [world/mod.rs:573-586](../../src/sim/world/mod.rs#L573-L586), add `metallic_debris: Vec<InternedId>` parameter:

```rust
    pub fn rebuild_caches_after_load(
        &mut self,
        resolved_terrain: ResolvedTerrainGrid,
        terrain_speed_config: terrain_speed::TerrainSpeedConfig,
        bridge_explosions: Vec<InternedId>,
        metallic_debris: Vec<InternedId>,    // NEW
        effect_frame_counts: BTreeMap<InternedId, u16>,
        terrain_costs: BTreeMap<SpeedType, TerrainCostGrid>,
    ) {
        self.resolved_terrain = Some(resolved_terrain);
        self.terrain_speed_config = terrain_speed_config;
        self.bridge_explosions = bridge_explosions;
        self.metallic_debris = metallic_debris;     // NEW
        self.effect_frame_counts = effect_frame_counts;
        self.terrain_costs = terrain_costs;
        // ... rest unchanged ...
    }
```

Update the callsite of `rebuild_caches_after_load` (search for it: `grep "rebuild_caches_after_load(" -r src/`) to pass the new parameter. The caller likely lives in `app_input.rs` or a snapshot-load helper — pass `Vec::new()` for now if the snapshot path doesn't track it (post-Phase F, snapshots will rebuild it from rules at load).

**Step 3: Pre-intern at sim init**

In `src/app_init_helpers.rs` immediately after the existing `sim.bridge_explosions = ...;` block at [line 361-369](../../src/app_init_helpers.rs#L361-L369):

```rust
    sim.metallic_debris = rules
        .map(|r| {
            r.general
                .metallic_debris
                .iter()
                .map(|s| sim.interner.intern(s))
                .collect()
        })
        .unwrap_or_default();
```

(Confirm the exact location of `metallic_debris` on `RuleSet` — it's at `rules.general.metallic_debris` per [ruleset.rs:435](../../src/rules/ruleset.rs#L435).)

**Step 4: Verify**

```
cargo build
cargo test --lib world -- --nocapture
```
Expected: build green; existing world tests still pass (additive change).

**Step 5: Commit**

```
git commit -m "sim/world: pre-intern Simulation.metallic_debris from [General] MetallicDebris= (Phase F precondition)"
```

---

### Task 3: Extend `BridgeDamageEvent` shape + update all consumer sites

**Why:** Combat boundary needs to pre-resolve the warhead identity, IonCannon flag, and impact z so the world orchestrator can do gate + retry + Z-height gate without re-fetching warhead context. Mechanical update across 11 sites — atomic to avoid broken builds.

**Files:**
- Modify: `src/sim/bridge_state.rs` (struct definition near [line 222](../../src/sim/bridge_state.rs#L222))
- Modify: `src/sim/combat/mod.rs` (3 emit sites at lines 798, 1476, 1511)
- Modify: `src/sim/world/world_tests.rs` (6 fixture call sites at lines 413, 455, 500, 539, 578, 617)
- Modify: `src/sim/bridge_state.rs` internal tests (lines ~1205, ~1227, ~1252)
- Modify: `src/sim/pathfinding/core_tests.rs:588`
- Modify: `src/sim/production/production_placement_tests.rs:664`

**Pattern:** struct extension; mechanical addition of 3 fields at every literal site.

**Step 1: Extend the struct**

In `src/sim/bridge_state.rs` near [line 222](../../src/sim/bridge_state.rs#L222):

```rust
/// Per-cell bridge damage event emitted by combat. World drains via the
/// `bridge_orchestrator` 4-path dispatcher. Apply_area_damage gate + retry
/// happen in the world orchestrator, not in combat — so RNG draw order
/// matches the binary's dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BridgeDamageEvent {
    pub rx: u16,
    pub ry: u16,
    pub damage: u16,
    /// Interned warhead ID — used for IonCannon identity check (combat
    /// boundary pre-resolves) and for InfDeath selection in the future
    /// C4-warhead ground-kill path.
    pub warhead_ref: crate::sim::intern::InternedId,
    /// Pre-resolved at combat: `warhead_ref == rules.ion_cannon_warhead_id()`.
    /// Bypasses BridgeStrength RNG gate; enables 3-retry loop on state-machine
    /// paths only (direct-overlay paths are single-shot per verification doc Finding 1).
    pub is_ion_cannon: bool,
    /// Explosion z in tile-step level units (signed). Used by the state-machine
    /// Z-height gate per verification doc Finding 2: state-machine paths fire
    /// only when `impact_z ∈ [cell.level - 1, cell.level + 1]`. Direct-overlay
    /// paths skip this gate.
    pub impact_z: i32,
}
```

**Step 2: Update 3 combat emit sites**

**Site 1 — death AoE in `apply_death_effects`** at [combat/mod.rs:798](../../src/sim/combat/mod.rs#L798):

Look at the surrounding loop (`for (rx, ry, dmg, wh_id, owner_id) in &death_aoe`). The current `death_aoe` tuple is `(u16, u16, i32, InternedId, InternedId)` per [line 577](../../src/sim/combat/mod.rs#L577) — no z. We need to extend the tuple to carry z.

Update the `death_aoe` declaration at [combat/mod.rs:577](../../src/sim/combat/mod.rs#L577):

```rust
    let mut death_aoe: Vec<(u16, u16, u8, i32, InternedId, InternedId)> = Vec::new();
    //                          ^^ z (entity tile-level)
```

Update the push at [combat/mod.rs:612](../../src/sim/combat/mod.rs#L612):

```rust
                    death_aoe.push((rx, ry, z, dmg, wh_id, owner));
```

(`z` is already in scope at this push site — see [combat/mod.rs:605](../../src/sim/combat/mod.rs#L605) where `dead_info` is destructured to `(type_id, rx, ry, z, owner, ...)`).

Now update the consumer loop pattern (search for `for (rx, ry, dmg, wh_id, owner_id) in &death_aoe` near line 780):

```rust
    for (rx, ry, z, dmg, wh_id, owner_id) in &death_aoe {
```

And the bridge-event push at [combat/mod.rs:798-803](../../src/sim/combat/mod.rs#L798-L803):

```rust
                } else {
                    let wh_iid = *wh_id;
                    bridge_damage_events.push(BridgeDamageEvent {
                        rx: *rx,
                        ry: *ry,
                        damage: damage_u16,
                        warhead_ref: wh_iid,
                        is_ion_cannon: wh_iid == rules.ion_cannon_warhead_id(),
                        impact_z: *z as i32,
                    });
                }
```

**Site 2 — primary attack AoE branch** at [combat/mod.rs:1476](../../src/sim/combat/mod.rs#L1476):

The `target_rx`, `target_ry` are in scope, plus `warhead` (the WarheadType the weapon uses). For impact_z, the target's z: if `TargetKind::Entity(target_id)`, look up the entity's z; if `TargetKind::Cell(rx,ry)`, use `terrain.cell(rx, ry).level`.

Replace the push at [combat/mod.rs:1476-1481](../../src/sim/combat/mod.rs#L1476-L1481):

```rust
                } else {
                    let wh_iid = interner.intern(&warhead.id);
                    let impact_z = combat_target_z(snap, target_rx, target_ry, entities)
                        .unwrap_or(0);
                    bridge_damage_events.push(BridgeDamageEvent {
                        rx: target_rx,
                        ry: target_ry,
                        damage: damage_u16,
                        warhead_ref: wh_iid,
                        is_ion_cannon: wh_iid == rules.ion_cannon_warhead_id(),
                        impact_z,
                    });
                }
```

Add a helper at the top of `combat/mod.rs` (above `apply_death_effects`):

```rust
/// Resolve impact-z (tile-step level units, signed) for a combat snapshot.
/// For Entity targets returns the target's z. For Cell targets returns 0
/// (terrain-cell level not in scope here without a terrain ref); callers
/// pass impact_z=0 and the orchestrator's Z-gate clamps appropriately.
fn combat_target_z(
    snap: &CombatSnapshot,
    target_rx: u16,
    target_ry: u16,
    entities: &EntityStore,
) -> Option<i32> {
    let _ = (target_rx, target_ry);
    if let TargetKind::Entity(eid) = snap.target {
        return entities.get(eid).map(|e| e.position.z as i32);
    }
    None
}
```

**Site 3 — primary attack non-AoE branch** at [combat/mod.rs:1511](../../src/sim/combat/mod.rs#L1511): same shape as Site 2:

```rust
                } else {
                    let wh_iid = interner.intern(&warhead.id);
                    let impact_z = combat_target_z(snap, target_rx, target_ry, entities)
                        .unwrap_or(0);
                    bridge_damage_events.push(BridgeDamageEvent {
                        rx: target_rx,
                        ry: target_ry,
                        damage: damage_u16,
                        warhead_ref: wh_iid,
                        is_ion_cannon: wh_iid == rules.ion_cannon_warhead_id(),
                        impact_z,
                    });
                }
```

(Confirm `rules: &RuleSet` is in scope at the call site — search for the function signature containing line 1476/1511.)

**Step 3: Update 11 consumer literal sites**

Each existing `BridgeDamageEvent { rx, ry, damage }` literal must add the 3 new fields. For test sites that aren't testing Ion-Cannon-specific behavior, use `is_ion_cannon: true, impact_z: 4` (level 4 = typical high-bridge deck level; IonCannon=true bypasses RNG gate and gives single-shot semantics that mimic the legacy apply_damage behavior tests expect).

Files to update:
- `src/sim/world/world_tests.rs` lines 413, 455, 500, 539, 578, 617 (6 sites)
- `src/sim/bridge_state.rs` internal tests (search for `BridgeDamageEvent {` in the test module — 3 sites near 1205, 1227, 1252)
- `src/sim/pathfinding/core_tests.rs:588`
- `src/sim/production/production_placement_tests.rs:664`

Add a tiny helper in test scope (or use directly):

```rust
    // Helper to build a single-shot Ion-Cannon-flavored bridge event.
    fn ion_event(rx: u16, ry: u16, damage: u16) -> BridgeDamageEvent {
        BridgeDamageEvent {
            rx, ry, damage,
            warhead_ref: crate::sim::intern::InternedId::default(), // tests don't care
            is_ion_cannon: true,
            impact_z: 4,
        }
    }
```

(Confirm `InternedId::default()` exists; if not, use a dummy intern via the test's interner.)

**Step 4: Verify**

```
cargo build
cargo test --lib -- --nocapture
```
Expected: full build green; all existing tests pass (still using legacy `apply_damage` internally; only struct shape changed).

**Step 5: Commit**

```
git commit -m "combat,bridge_state: extend BridgeDamageEvent with warhead_ref + is_ion_cannon + impact_z (Phase F precondition; 11 sites mechanically updated)"
```

---

### --- Phase E END — Combat boundary + warhead resolution + interner pre-fill complete. Build green. ---

---

### Phase F1 — Structural

### Task 4: Split `bridge_state.rs` → `bridge_state/` directory

**Why:** Adding ~350 LOC walker drivers + ~80 LOC dispatch helpers to the current 1947-line `bridge_state.rs` would violate the 600-line guideline. Cosmetic split first; no behavior change.

**Files:**
- Move: `src/sim/bridge_state.rs` → `src/sim/bridge_state/mod.rs`
- Create: `src/sim/bridge_state/` directory

**Pattern:** repo-wide module-directory pattern (e.g., `src/sim/world/` already a directory).

**Step 1: Create the directory + move the file**

```
git mv src/sim/bridge_state.rs src/sim/bridge_state/mod.rs
```

(If `git mv` is unavailable on the platform, use `mkdir src/sim/bridge_state && mv src/sim/bridge_state.rs src/sim/bridge_state/mod.rs && git add -u`.)

**Step 2: Verify imports still resolve**

Open `src/sim/mod.rs` and look for `pub mod bridge_state;` — this declaration is unchanged because Rust supports both `bridge_state.rs` and `bridge_state/mod.rs` for the same module path.

**Step 3: Verify**

```
cargo build
cargo test --lib bridge_state -- --nocapture
```
Expected: builds green; all bridge_state tests still pass (no content changes).

**Step 4: Commit**

```
git commit -m "sim/bridge_state: split into directory module (Phase F structural — preparing for walker.rs)"
```

---

### Phase F2 — New drivers + classifier

### Task 5: `DispatchPath` enum + `path_matches_cell` classifier + `bridge_strength()` / `is_destroyable()` getters

**Why:** Orchestrator needs a single classifier for all 4 paths. Encodes verification-doc Findings 1/2/3 (single-shot direct-overlay, Z-height gate, raw-overlay routing). Additive; old `apply_damage` still works.

**Files:**
- Modify: `src/sim/bridge_state/mod.rs`

**Pattern:** new method on `BridgeRuntimeState`; pure-function classifier with no mutation.

**Step 1: Add public getters near other accessors**

Locate `BridgeRuntimeState::cell` and `cell_mut` methods (~line 500-515 in the original file, now in `mod.rs`). Add after them:

```rust
    /// Map width in cells. Needed by walker code in the `walker` submodule
    /// (Rust privacy: child modules can't access parent's private fields
    /// without `pub(super)` or a getter).
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Map height in cells. See `width()` rationale.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// `[CombatDamage] BridgeStrength=` value used by `apply_area_damage`'s
    /// per-path RNG gate. Read-only; set at construction.
    pub fn bridge_strength(&self) -> u16 {
        self.bridge_strength
    }

    /// Whether the global `SpecialFlags::DestroyableBridges` (verified mirror
    /// of binary's `g_ScenarioClass+0x?? & 0x8000`) is set. Outer gate of
    /// `apply_area_damage`; if false, bridges are immune.
    pub fn is_destroyable(&self) -> bool {
        self.bridge_destroyable_flag
    }
```

**Step 2: Add `DispatchPath` enum + `BridgeDamageContext` near the top-level types (after `BridgeDamageEvent` declaration around line 222)**

```rust
/// Per-event context passed from world orchestrator to `BridgeRuntimeState`
/// for the 4-path dispatcher. Carries the pre-resolved IonCannon flag, the
/// impact z (for state-machine Z-height gate), and the interner-resolved
/// warhead reference. The orchestrator owns the `&mut SimRng` and does the
/// actual RNG draws — drivers themselves are pure of RNG.
#[derive(Debug, Clone, Copy)]
pub struct BridgeDamageContext {
    pub damage: u16,
    pub warhead_ref: crate::sim::intern::InternedId,
    pub is_ion_cannon: bool,
    pub bridge_strength: u16,
    /// Tile-step level units (signed for safety). State-machine Z-gate fires
    /// when `impact_z ∈ [cell.level - 1, cell.level + 1]` (3-level window).
    pub impact_z: i32,
}

/// Path discriminator for `apply_area_damage` 4-path dispatcher.
/// Order matches binary `0x00489280` evaluation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchPath {
    /// HIGH state-machine: `flags & 0x100` + anchor.+0x44 ∈ {0x18, 0x19}
    /// OR bridgehead tile-class match. Includes Z-height range gate.
    HighStateMachine,
    /// LOW state-machine: anchor.+0x44 ∈ {0xED, 0xEE} OR low-bridgehead
    /// tile-class match. Includes Z-height range gate.
    LowStateMachine,
    /// LOW direct-overlay: `cell.OverlayIndex ∈ [0x4A..0x63]`. Single-shot;
    /// no Z-gate.
    LowDirect,
    /// HIGH direct-overlay: `cell.OverlayIndex ∈ [0xCD..0xE6]`. Single-shot;
    /// no Z-gate.
    HighDirect,
}

impl DispatchPath {
    /// State-machine paths support the IonCannon 3-retry loop. Direct-overlay
    /// paths are single-shot regardless of warhead (verification doc Finding 1).
    pub fn is_state_machine(self) -> bool {
        matches!(self, DispatchPath::HighStateMachine | DispatchPath::LowStateMachine)
    }
}
```

**Step 3: Add `path_matches_cell` classifier on `BridgeRuntimeState`**

Add near the new getters:

```rust
    /// Per-path entry-condition classifier for the orchestrator. Pure
    /// function; no mutation. Returns true iff the cell at (rx, ry) matches
    /// the entry conditions for `path` under `ctx`.
    ///
    /// Mirrors the binary's per-path entry checks:
    /// - HighStateMachine: cell has structural bit + anchor's overlay class
    ///   matches HIGH anchor IDs OR bridgehead-tile-class match. Z-gate
    ///   restricts impact_z to `[level - 1, level + 1]`. Per verification
    ///   doc Finding 3, this path additionally REJECTS cells whose
    ///   `overlay_byte` is still in `[0xCD..0xE6]` — those route through
    ///   HighDirect (the walker), not the state machine.
    /// - LowStateMachine: same shape with LOW anchor IDs.
    /// - HighDirect: `overlay_byte ∈ [0xCD..0xE6]`. Single-shot, no Z-gate.
    /// - LowDirect:  `overlay_byte ∈ [0x4A..0x63]`. Single-shot, no Z-gate.
    pub(crate) fn path_matches_cell(
        &self,
        path: DispatchPath,
        rx: u16,
        ry: u16,
        ctx: &BridgeDamageContext,
        terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    ) -> bool {
        let Some(cell) = self.cell(rx, ry) else { return false; };
        match path {
            DispatchPath::HighDirect => (0xCD..=0xE6).contains(&cell.overlay_byte),
            DispatchPath::LowDirect => (0x4A..=0x63).contains(&cell.overlay_byte),
            DispatchPath::HighStateMachine | DispatchPath::LowStateMachine => {
                // Verification doc Finding 3: raw-overlay cells route to the
                // walker, NOT the state machine. State-machine fires only
                // after overlay has been transitioned out of the body range.
                if matches!(path, DispatchPath::HighStateMachine)
                    && (0xCD..=0xE6).contains(&cell.overlay_byte)
                {
                    return false;
                }
                if matches!(path, DispatchPath::LowStateMachine)
                    && (0x4A..=0x63).contains(&cell.overlay_byte)
                {
                    return false;
                }
                // Role check: must be a bridge-structural cell.
                if !matches!(
                    cell.role,
                    BridgeCellRole::Anchor
                        | BridgeCellRole::Body
                        | BridgeCellRole::Tail
                        | BridgeCellRole::Bridgehead
                ) {
                    return false;
                }
                // High vs Low discriminator: deck_level >= 4 → high, else low.
                // (Bridgehead cells share the same axis classification.)
                let is_high = cell.deck_level >= 4;
                let want_high = matches!(path, DispatchPath::HighStateMachine);
                if is_high != want_high {
                    return false;
                }
                // Z-height range gate per verification doc Finding 2.
                // Pass when impact_z is in [level - 1, level + 1]; skip otherwise.
                let level_i32 = terrain
                    .cell(rx, ry)
                    .map(|c| c.level as i32)
                    .unwrap_or(cell.deck_level as i32);
                if ctx.impact_z < level_i32 - 1 || ctx.impact_z > level_i32 + 1 {
                    return false;
                }
                true
            }
        }
    }
```

**Step 4: Unit tests (in the existing `mod tests` of `bridge_state/mod.rs`)**

```rust
    #[test]
    fn path_matches_high_direct_for_raw_body_overlay() {
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(2, 0, BridgeRuntimeCell {
            deck_present: true, destroyable: true, deck_level: 5,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::EW),
            role: BridgeCellRole::Body,
            anchor_span_id: Some(1),
            overlay_byte: 0xD0, // in [0xCD..0xE6]
        });
        let terrain = make_test_terrain(); // existing test helper
        let ctx = test_ctx(/* damage */ 100, /* impact_z */ 5);
        assert!(state.path_matches_cell(DispatchPath::HighDirect, 2, 0, &ctx, &terrain));
        assert!(!state.path_matches_cell(DispatchPath::HighStateMachine, 2, 0, &ctx, &terrain),
            "raw-overlay cell must NOT match HighStateMachine (verification Finding 3)");
    }

    #[test]
    fn path_matches_high_sm_z_gate_excludes_far_explosions() {
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(2, 0, BridgeRuntimeCell {
            deck_present: true, destroyable: true, deck_level: 5,
            bridge_group_id: Some(1),
            damage_state: DamageState::Damaged, // overlay-transitioned
            axis: Some(Axis::EW),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x6, // out of body range — sm-eligible
        });
        let terrain = make_test_terrain_at_level(2, 0, 5);
        // impact_z=8 is 3 above level 5 — outside [4, 6] window
        let ctx = test_ctx(100, 8);
        assert!(!state.path_matches_cell(DispatchPath::HighStateMachine, 2, 0, &ctx, &terrain));
        // impact_z=5 is at level — passes
        let ctx = test_ctx(100, 5);
        assert!(state.path_matches_cell(DispatchPath::HighStateMachine, 2, 0, &ctx, &terrain));
        // impact_z=6 is +1 — boundary inclusive
        let ctx = test_ctx(100, 6);
        assert!(state.path_matches_cell(DispatchPath::HighStateMachine, 2, 0, &ctx, &terrain));
        // impact_z=4 is -1 — boundary inclusive
        let ctx = test_ctx(100, 4);
        assert!(state.path_matches_cell(DispatchPath::HighStateMachine, 2, 0, &ctx, &terrain));
    }
```

(Add `test_ctx`, `make_test_terrain_at_level` helpers — the test module already has `make_bridge_state_with_overlay` style helpers per [world_hash.rs:549](../../src/sim/world/world_hash.rs#L549); follow the same pattern.)

**Step 5: Verify**

```
cargo test --lib bridge_state::tests::path_matches -- --nocapture
cargo build
```
Expected: tests pass; build green.

**Step 6: Commit**

```
git commit -m "sim/bridge_state: add DispatchPath + BridgeDamageContext + path_matches_cell classifier (Phase F2; encodes verification-doc Findings 1/2/3)"
```

---

### Task 6: `destroy_bridge_high` walker driver entry

**Why:** New driver for the overlay-direct path (paths 3 & 4 in the dispatcher). Mirrors binary `DestroyBridge_High @ 0x0057CCF0`. Classifies axis from overlay byte, dispatches to NS or EW walker. Walker proper lands in Task 7.

**Files:**
- Create: `src/sim/bridge_state/walker.rs`
- Modify: `src/sim/bridge_state/mod.rs` (add `pub mod walker;` near top)

**Pattern:** new method on `BridgeRuntimeState`, lives in submodule. Public method.

**Step 1: Wire up the submodule**

In `src/sim/bridge_state/mod.rs` near the top (after the `//!` doc comment, before the first `use`):

```rust
pub mod walker;
```

**Step 2: Create `src/sim/bridge_state/walker.rs`**

```rust
//! Overlay-direct bridge destruction walker. Mirror of binary
//! `DestroyBridge_High @ 0x0057CCF0` and `DestroyBridge_Low` (low equivalent).
//!
//! Drives full-bridge collapse from a single hit on a cell whose overlay byte
//! is still in the raw body range (verification doc Finding 3). Distinct from
//! the state-machine drivers in `bridge_state/mod.rs`, which handle the
//! late-stage progression after overlays have been transitioned.
//!
//! ## Dependency rules
//! Same as sim/: depends on rules/ + map/; never render/ui/audio/net.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::sim::bridge_state::{
    Axis, BridgeRuntimeState, StateOutcome,
};

impl BridgeRuntimeState {
    /// Overlay-direct HIGH walker entry. Mirror of binary
    /// `DestroyBridge_High @ 0x0057CCF0`. Three responsibilities:
    /// 1. Classifies cell.overlay_byte to pick NS or EW walker
    /// 2. Performs the **start-cell shift**: reads neighbor 1 + neighbor 2
    ///    along the body axis to find a "stable mid" before walking. The
    ///    binary does this so multiple hits on different cells of the same
    ///    bridge converge to the same walker start.
    /// 3. Forwards the shifted coord to the appropriate walker.
    ///
    /// Returns `Collapsed` (populated by walker recursion), `NoChange` when
    /// overlay is not in HIGH body range.
    ///
    /// Start-shift logic (NS axis, mirror for EW with x±1, x±2):
    /// - if `(rx, ry-1)` overlay NOT in `[0xCD..=0xE8]` (off-bridge north)
    ///   → walker starts at `(rx, ry+1)`
    /// - else if `(rx, ry-2)` overlay IS in `[0xCD..=0xE8]` → walker starts
    ///   at `(rx, ry-1)`
    /// - else → walker starts at input
    pub fn destroy_bridge_high(
        &mut self,
        rx: u16,
        ry: u16,
        terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        let Some(cell) = self.cell(rx, ry).copied() else { return StateOutcome::NoChange; };
        let overlay = cell.overlay_byte;

        // HIGH body overlay range: [0xCD..0xE6].
        // NS axis sub-range: [0xCD..=0xD5] ∪ [0xDF..=0xE2] ∪ {0xE7}
        // EW axis sub-range: [0xD6..=0xDE] ∪ [0xE3..=0xE6] ∪ {0xE8}
        if Self::is_ns_walker_overlay(overlay) {
            let start = self.find_walker_start_high_ns(rx, ry);
            return self.destroy_bridge_walker_ns_high(start.0, start.1, terrain);
        }
        if Self::is_ew_walker_overlay(overlay) {
            let start = self.find_walker_start_high_ew(rx, ry);
            return self.destroy_bridge_walker_ew_high(start.0, start.1, terrain);
        }
        StateOutcome::NoChange
    }

    /// Pre-walk start-cell shift for NS axis. Mirror of binary's
    /// 3-case neighbor check at `DestroyBridge_High @ 0x0057CCF0`.
    fn find_walker_start_high_ns(&self, rx: u16, ry: u16) -> (u16, u16) {
        let in_range = |o: u8| (0xCD..=0xE8).contains(&o);
        // North-1: (rx, ry-1)
        if ry == 0 || self.cell(rx, ry - 1).map(|c| !in_range(c.overlay_byte)).unwrap_or(true) {
            // North not on bridge → walker starts at (rx, ry+1)
            return (rx, ry.saturating_add(1));
        }
        // North-2: (rx, ry-2)
        if ry >= 2 && self.cell(rx, ry - 2).map(|c| in_range(c.overlay_byte)).unwrap_or(false) {
            // 2-north IS on bridge → walker starts at (rx, ry-1)
            return (rx, ry - 1);
        }
        (rx, ry)
    }

    /// Pre-walk start-cell shift for EW axis. Mirror of binary's neighbor
    /// check at `0x0057CCF0`.
    fn find_walker_start_high_ew(&self, rx: u16, ry: u16) -> (u16, u16) {
        let in_range = |o: u8| (0xCD..=0xE8).contains(&o);
        if rx == 0 || self.cell(rx - 1, ry).map(|c| !in_range(c.overlay_byte)).unwrap_or(true) {
            return (rx.saturating_add(1), ry);
        }
        if rx >= 2 && self.cell(rx - 2, ry).map(|c| in_range(c.overlay_byte)).unwrap_or(false) {
            return (rx - 1, ry);
        }
        (rx, ry)
    }

    /// Overlay-direct LOW walker entry. Mirror of binary
    /// `DestroyBridge_Low @ 0x0057BAA0`. Same shape as `destroy_bridge_high`:
    /// classify axis, start-cell shift, forward to NS or EW walker.
    /// LOW range: `[0x4A..=0x65]`.
    /// - LOW NS sub-range: `[0x4A..=0x52] ∪ [0x5C..=0x5F] ∪ {0x64}`
    /// - LOW EW sub-range: `[0x53..=0x5B] ∪ [0x60..=0x63] ∪ {0x65}`
    pub fn destroy_bridge_low(
        &mut self,
        rx: u16,
        ry: u16,
        terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        let Some(cell) = self.cell(rx, ry).copied() else { return StateOutcome::NoChange; };
        let overlay = cell.overlay_byte;
        if Self::is_ns_walker_overlay_low(overlay) {
            let start = self.find_walker_start_low_ns(rx, ry);
            return self.destroy_bridge_walker_ns_low(start.0, start.1, terrain);
        }
        if Self::is_ew_walker_overlay_low(overlay) {
            let start = self.find_walker_start_low_ew(rx, ry);
            return self.destroy_bridge_walker_ew_low(start.0, start.1, terrain);
        }
        StateOutcome::NoChange
    }

    fn find_walker_start_low_ns(&self, rx: u16, ry: u16) -> (u16, u16) {
        let in_range = |o: u8| (0x4A..=0x65).contains(&o);
        if ry == 0 || self.cell(rx, ry - 1).map(|c| !in_range(c.overlay_byte)).unwrap_or(true) {
            return (rx, ry.saturating_add(1));
        }
        if ry >= 2 && self.cell(rx, ry - 2).map(|c| in_range(c.overlay_byte)).unwrap_or(false) {
            return (rx, ry - 1);
        }
        (rx, ry)
    }

    fn find_walker_start_low_ew(&self, rx: u16, ry: u16) -> (u16, u16) {
        let in_range = |o: u8| (0x4A..=0x65).contains(&o);
        if rx == 0 || self.cell(rx - 1, ry).map(|c| !in_range(c.overlay_byte)).unwrap_or(true) {
            return (rx.saturating_add(1), ry);
        }
        if rx >= 2 && self.cell(rx - 2, ry).map(|c| in_range(c.overlay_byte)).unwrap_or(false) {
            return (rx - 1, ry);
        }
        (rx, ry)
    }

    fn is_ns_walker_overlay(overlay: u8) -> bool {
        // HIGH NS axis (per Ghidra `0x0057CCF0`):
        // [0xCD..=0xD5] ∪ [0xDF..=0xE2] ∪ {0xE7}
        (0xCD..=0xD5).contains(&overlay)
            || (0xDF..=0xE2).contains(&overlay)
            || overlay == 0xE7
    }

    fn is_ew_walker_overlay(overlay: u8) -> bool {
        // HIGH EW axis: [0xD6..=0xDE] ∪ [0xE3..=0xE6] ∪ {0xE8}
        (0xD6..=0xDE).contains(&overlay)
            || (0xE3..=0xE6).contains(&overlay)
            || overlay == 0xE8
    }

    fn is_ns_walker_overlay_low(overlay: u8) -> bool {
        // LOW NS axis (per Ghidra `0x0057BAA0`):
        // [0x4A..=0x52] ∪ [0x5C..=0x5F] ∪ {0x64}
        (0x4A..=0x52).contains(&overlay)
            || (0x5C..=0x5F).contains(&overlay)
            || overlay == 0x64
    }

    fn is_ew_walker_overlay_low(overlay: u8) -> bool {
        // LOW EW axis: [0x53..=0x5B] ∪ [0x60..=0x63] ∪ {0x65}
        (0x53..=0x5B).contains(&overlay)
            || (0x60..=0x63).contains(&overlay)
            || overlay == 0x65
    }

    /// Stub — implemented in Task 7. Returns NoChange for now.
    pub(super) fn destroy_bridge_walker_ns_high(
        &mut self,
        _rx: u16,
        _ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        StateOutcome::NoChange
    }

    /// Stub — implemented in Task 7. Returns NoChange for now.
    pub(super) fn destroy_bridge_walker_ew_high(
        &mut self,
        _rx: u16,
        _ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        StateOutcome::NoChange
    }

    /// Stub — implemented in Task 8. Returns NoChange for now.
    pub(super) fn destroy_bridge_walker_ns_low(
        &mut self,
        _rx: u16,
        _ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        StateOutcome::NoChange
    }

    /// Stub — implemented in Task 8. Returns NoChange for now.
    pub(super) fn destroy_bridge_walker_ew_low(
        &mut self,
        _rx: u16,
        _ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        StateOutcome::NoChange
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::bridge_state::{
        BridgeCellRole, BridgeRuntimeCell, DamageState,
    };

    #[test]
    fn destroy_bridge_high_classifies_ns_axis() {
        // overlay 0xD0 ∈ NS sub-range
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(2, 0, BridgeRuntimeCell {
            deck_present: true, destroyable: true, deck_level: 5,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Body,
            anchor_span_id: Some(1),
            overlay_byte: 0xD0,
        });
        // Walker is stubbed; expectation = the entry function classified
        // axis without panicking and forwarded to the (stubbed) NS walker.
        // After Task 7 lands, this becomes a real outcome assertion.
        let terrain = ResolvedTerrainGrid::default();
        let outcome = state.destroy_bridge_high(2, 0, &terrain);
        assert!(matches!(outcome, StateOutcome::NoChange),
            "Task 6 ships with NS/EW walkers stubbed");
    }

    #[test]
    fn destroy_bridge_high_returns_nochange_for_non_high_overlay() {
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(0, 0, BridgeRuntimeCell {
            deck_present: true, destroyable: true, deck_level: 2,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Body,
            anchor_span_id: Some(1),
            overlay_byte: 0x5A, // low range, not high
        });
        let terrain = ResolvedTerrainGrid::default();
        assert!(matches!(state.destroy_bridge_high(0, 0, &terrain), StateOutcome::NoChange));
    }
}
```

**Step 3: Verify**

```
cargo build
cargo test --lib bridge_state::walker -- --nocapture
```
Expected: build green; classification tests pass.

**Step 4: Commit**

```
git commit -m "sim/bridge_state/walker: add destroy_bridge_high/_low entry — overlay-byte axis classification (walker bodies stubbed for Task 7/8)"
```

---

### Task 7: HIGH walker bodies + sibling-cascade helpers

**Why:** The walker is NOT a linear axis walk. Per Ghidra `DestroyBridgeWalker_NS_High @ 0x0057CF60` and `_EW_High @ 0x0057D530` (re-verified during /review-plan), the walker is a **3-cell length-axis triple-write** with case-based overlay transitions and recursion to perpendicular siblings via `apply_bridge_destruction_*_high`. Each call mutates up to 9 cells (3 from walker + 3 per sibling), and final-stage transitions (0xE7/0xE8) trigger zone refresh via `FindBridgeEndpoints_*_High`.

**Files:**
- Modify: `src/sim/bridge_state/walker.rs` (replace stubs from Task 6)

**Pattern:** Walker writes 3 length-axis cells; cascades to perpendicular siblings via `apply_bridge_destruction_*` (which itself writes 3 cells using `pick_destruction_overlay` table). No further recursion.

**Step 1: Implement `check_bridge_neighbors_ew_high` + `_ns_high` classifiers**

These compute the 0..=15 index that `pick_destruction_overlay` (already shipped, [bridge_specs.rs:399](../../src/sim/bridge_specs.rs#L399)) takes. Mirrors of binary `MapClass__CheckBridgeNeighbors_EW_High @ 0x0057CAB0` / `_NS_High @ 0x0057CBE0` (re-verified during /review-plan).

```rust
impl BridgeRuntimeState {
    /// Compute neighbor-pattern index for `pick_destruction_overlay(_, NS, _)`.
    /// Used inside `apply_bridge_destruction_ns_high` (NS body cell). Reads
    /// the perpendicular EW neighbors (west and east) of `(rx, ry)`.
    /// Result bits:
    ///   bit 0 (val 1): east in {0xD1, 0xD3, 0xD5, 0xE0}
    ///   bit 1 (val 2): east in {0xD4, 0xE7}
    ///   bit 2 (val 4): west in {0xD2, 0xD3, 0xD4, 0xE2}
    ///   bit 3 (val 8): west in {0xD5, 0xE7}
    /// Reachable indices: 0,1,2,4,5,6,8,9,10 (others unreachable due to
    /// switch mutual exclusion).
    pub(super) fn check_bridge_neighbors_ew_high(&self, rx: u16, ry: u16) -> u8 {
        let east = if rx as u32 + 1 < u16::MAX as u32 {
            self.cell(rx + 1, ry).map(|c| c.overlay_byte).unwrap_or(0)
        } else { 0 };
        let west = if rx > 0 {
            self.cell(rx - 1, ry).map(|c| c.overlay_byte).unwrap_or(0)
        } else { 0 };
        let mut idx = 0u8;
        match east {
            0xD1 | 0xD3 | 0xD5 | 0xE0 => idx |= 1,
            0xD4 | 0xE7 => idx |= 2,
            _ => {}
        }
        match west {
            0xD2 | 0xD3 | 0xD4 | 0xE2 => idx |= 4,
            0xD5 | 0xE7 => idx |= 8,
            _ => {}
        }
        idx
    }

    /// Compute neighbor-pattern index for `pick_destruction_overlay(_, EW, _)`.
    /// Used inside `apply_bridge_destruction_ew_high`. Reads the perpendicular
    /// NS neighbors (north and south). Mirror of `check_bridge_neighbors_ew_high`
    /// with the same bit-encoding scheme but EW overlays:
    ///   bit 0 (val 1): south in {0xDA, 0xDC, 0xDE, 0xE6}
    ///   bit 1 (val 2): south in {0xDD, 0xE8}
    ///   bit 2 (val 4): north in {0xDB, 0xDC, 0xDD, 0xE4}
    ///   bit 3 (val 8): north in {0xDE, 0xE8}
    pub(super) fn check_bridge_neighbors_ns_high(&self, rx: u16, ry: u16) -> u8 {
        let south = self.cell(rx, ry.saturating_add(1)).map(|c| c.overlay_byte).unwrap_or(0);
        let north = if ry > 0 {
            self.cell(rx, ry - 1).map(|c| c.overlay_byte).unwrap_or(0)
        } else { 0 };
        let mut idx = 0u8;
        match south {
            0xDA | 0xDC | 0xDE | 0xE6 => idx |= 1,
            0xDD | 0xE8 => idx |= 2,
            _ => {}
        }
        match north {
            0xDB | 0xDC | 0xDD | 0xE4 => idx |= 4,
            0xDE | 0xE8 => idx |= 8,
            _ => {}
        }
        idx
    }
}
```

(EW north/south overlay sets are mirrored from the binary's EW classifier at `0x0057CBE0` — re-verify these specific overlay values when implementing by decompiling that address. The bit-encoding pattern matches NS exactly.)

**Step 2: Implement `apply_bridge_destruction_ns_high` (sibling-cascade leaf)**

Mirror of binary `MapClass__ApplyBridgeDestruction_NS_High @ 0x0057E7A0` (re-decompiled during /review-plan). Writes the (this, north, south) triple at the sibling cell using `pick_destruction_overlay` table-lookup, with overrides for 0xDF and 0xE1 mid-states.

**Returns the list of cells whose `damage_state` transitioned to `Destroyed` (final state 0xE7 only).** The walker accumulates these into the StateOutcome.

```rust
    /// Sibling-cascade leaf for the NS body axis. Mirror of binary
    /// `MapClass__ApplyBridgeDestruction_NS_High @ 0x0057E7A0`.
    ///
    /// Validates `(rx, ry)` is within HIGH range, computes neighbor index
    /// via `check_bridge_neighbors_ew_high`, looks up next-overlay via
    /// `pick_destruction_overlay`, writes the (this, north, south) triple.
    /// Returns the cells that hit final-collapse (overlay 0xE7) — caller
    /// adds them to `destroyed_cells` and `BlowUpBridge` actions.
    fn apply_bridge_destruction_ns_high(
        &mut self,
        rx: u16,
        ry: u16,
    ) -> Vec<(u16, u16)> {
        use crate::sim::bridge_specs::pick_destruction_overlay;
        use crate::sim::bridge_state::DamageState;

        let mut final_cells = Vec::new();
        let Some(cell) = self.cell(rx, ry).copied() else { return final_cells; };
        let cur = cell.overlay_byte;
        // Outer gate: HIGH range.
        if !(0xCD..=0xE8).contains(&cur) { return final_cells; }

        let idx = self.check_bridge_neighbors_ew_high(rx, ry);
        if idx == 0 { return final_cells; } // no perpendicular pattern

        let next = if cur < 0xDF {
            // Table lookup; if same as current, no-op.
            match pick_destruction_overlay(idx, Axis::NS, true) {
                Some(n) if n != cur => n,
                _ => return final_cells,
            }
        } else if cur == 0xDF {
            0xE0
        } else if cur == 0xE1 {
            0xE2
        } else {
            // 0xE0, 0xE2, 0xE7+ — no further transition at this cell.
            return final_cells;
        };

        // Write triple: (this, north, south)
        for cell_pos in Self::ns_triple(rx, ry) {
            if let Some(c) = self.cell_mut(cell_pos.0, cell_pos.1) {
                c.overlay_byte = next;
                if next == 0xE7 {
                    c.damage_state = DamageState::Destroyed;
                    final_cells.push(cell_pos);
                } else {
                    c.damage_state = DamageState::Damaged;
                }
            }
        }
        final_cells
    }

    /// Sibling-cascade leaf for the EW body axis. Mirror.
    fn apply_bridge_destruction_ew_high(
        &mut self,
        rx: u16,
        ry: u16,
    ) -> Vec<(u16, u16)> {
        use crate::sim::bridge_specs::pick_destruction_overlay;
        use crate::sim::bridge_state::DamageState;

        let mut final_cells = Vec::new();
        let Some(cell) = self.cell(rx, ry).copied() else { return final_cells; };
        let cur = cell.overlay_byte;
        if !(0xCD..=0xE8).contains(&cur) { return final_cells; }

        let idx = self.check_bridge_neighbors_ns_high(rx, ry);
        if idx == 0 { return final_cells; }

        let next = if cur < 0xE3 {
            match pick_destruction_overlay(idx, Axis::EW, true) {
                Some(n) if n != cur => n,
                _ => return final_cells,
            }
        } else if cur == 0xE3 {
            0xE4
        } else if cur == 0xE5 {
            0xE6
        } else {
            return final_cells;
        };

        for cell_pos in Self::ew_triple(rx, ry) {
            if let Some(c) = self.cell_mut(cell_pos.0, cell_pos.1) {
                c.overlay_byte = next;
                if next == 0xE8 {
                    c.damage_state = DamageState::Destroyed;
                    final_cells.push(cell_pos);
                } else {
                    c.damage_state = DamageState::Damaged;
                }
            }
        }
        final_cells
    }

    /// (this, north, south) cell triple for NS-axis triple-write. Skips
    /// off-map cells.
    fn ns_triple(rx: u16, ry: u16) -> impl Iterator<Item = (u16, u16)> {
        let north = if ry > 0 { Some((rx, ry - 1)) } else { None };
        let south = Some((rx, ry.saturating_add(1)));
        [Some((rx, ry)), north, south].into_iter().flatten()
    }

    /// (this, west, east) cell triple for EW-axis triple-write. Skips
    /// off-map cells.
    fn ew_triple(rx: u16, ry: u16) -> impl Iterator<Item = (u16, u16)> {
        let west = if rx > 0 { Some((rx - 1, ry)) } else { None };
        let east = Some((rx.saturating_add(1), ry));
        [Some((rx, ry)), west, east].into_iter().flatten()
    }
```

**Step 3: Implement `destroy_bridge_walker_ns_high`** (replaces Task 6 stub):

```rust
    /// NS-axis walker. Mirror of binary
    /// `MapClass__DestroyBridgeWalker_NS_High @ 0x0057CF60`.
    ///
    /// Reads input cell's overlay, picks one of 5 cases:
    /// - 0xDF → write 0xE0 to (this, north, south); cascade at (rx-1, ry)
    /// - 0xE1 → write 0xE2 to (this, north, south); cascade at (rx+1, ry)
    /// - input < 0xD3 (0xCD..=0xD2) → write 0xD3 to triple; cascade at BOTH (rx±1, ry)
    /// - input ∈ [0xD3..=0xD5] → write 0xE7 to triple (FINAL collapse);
    ///       cascade at BOTH (rx±1, ry); call FindBridgeEndpoints; zones_dirty=true
    /// - else → no-op
    ///
    /// Writes to (this, north, south) cell triple; recurses ONCE to
    /// perpendicular siblings via `apply_bridge_destruction_ns_high`.
    /// Total: ~3-9 cells touched per call.
    pub(super) fn destroy_bridge_walker_ns_high(
        &mut self,
        rx: u16,
        ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        use crate::sim::bridge_specs::{CellAction, SetBridgeDirectionResult};
        use crate::sim::bridge_state::{compute_adjacent_bridges_dirty, DamageState};

        let Some(cell) = self.cell(rx, ry).copied() else { return StateOutcome::NoChange; };
        let cur = cell.overlay_byte;

        // Pick case + sibling-cascade plan.
        let (next, siblings, is_final) = if cur == 0xDF {
            (0xE0u8, vec![(rx.wrapping_sub(1), ry)], false)
        } else if cur == 0xE1 {
            (0xE2u8, vec![(rx.saturating_add(1), ry)], false)
        } else if cur < 0xD3 {
            (0xD3u8, vec![(rx.wrapping_sub(1), ry), (rx.saturating_add(1), ry)], false)
        } else if (0xD3..=0xD5).contains(&cur) {
            (0xE7u8, vec![(rx.wrapping_sub(1), ry), (rx.saturating_add(1), ry)], true)
        } else {
            return StateOutcome::NoChange;
        };

        let mut destroyed: Vec<(u16, u16)> = Vec::new();
        let mut actions: Vec<((u16, u16), usize, CellAction)> = Vec::new();

        // Write the (this, north, south) length-axis triple.
        for (slot, pos) in Self::ns_triple(rx, ry).enumerate() {
            if let Some(c) = self.cell_mut(pos.0, pos.1) {
                c.overlay_byte = next;
                if is_final {
                    c.damage_state = DamageState::Destroyed;
                    destroyed.push(pos);
                    actions.push((pos, slot, CellAction::BlowUpBridge));
                } else {
                    c.damage_state = DamageState::Damaged;
                }
            }
        }

        // Cascade to perpendicular siblings.
        for (sx, sy) in siblings {
            if sx == u16::MAX { continue; } // wrapping_sub overflow: x=0 west neighbor
            let mut sibling_finals = self.apply_bridge_destruction_ns_high(sx, sy);
            for pos in sibling_finals.drain(..) {
                if !destroyed.contains(&pos) {
                    destroyed.push(pos);
                    actions.push((pos, 0, CellAction::BlowUpBridge));
                }
            }
        }

        if destroyed.is_empty() && !is_final {
            // Intermediate transition only — overlay/damage_state changed but
            // no cell hit final. Nothing to cascade. Per binary behavior we
            // don't return Collapsed in this case.
            return StateOutcome::Absorbed;
        }

        let adj = compute_adjacent_bridges_dirty(rx, ry, Axis::NS);
        StateOutcome::Collapsed {
            destroyed_cells: destroyed,
            set_bridge_direction: SetBridgeDirectionResult { actions },
            adjacent_bridges_dirty: adj,
            zones_dirty: is_final, // only final-stage transitions invalidate zones
        }
    }
```

**Step 4: Implement `destroy_bridge_walker_ew_high`** as a mirror of NS with x±1 perpendicular siblings instead of y±1:

```rust
    /// EW-axis walker. Mirror of `DestroyBridgeWalker_EW_High @ 0x0057D530`.
    /// Cases:
    /// - 0xE3 → write 0xE4 to (this, west, east); cascade at (rx, ry+1)
    /// - 0xE5 → write 0xE6 to triple; cascade at (rx, ry-1)
    /// - input < 0xDC → write 0xDC to triple; cascade at BOTH (rx, ry±1)
    /// - input ∈ [0xDC..=0xDE] → write 0xE8 to triple (FINAL); cascade at
    ///       BOTH (rx, ry±1); FindBridgeEndpoints_EW_High; zones_dirty
    /// - else → no-op
    pub(super) fn destroy_bridge_walker_ew_high(
        &mut self,
        rx: u16,
        ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        use crate::sim::bridge_specs::{CellAction, SetBridgeDirectionResult};
        use crate::sim::bridge_state::{compute_adjacent_bridges_dirty, DamageState};

        let Some(cell) = self.cell(rx, ry).copied() else { return StateOutcome::NoChange; };
        let cur = cell.overlay_byte;

        let (next, siblings, is_final) = if cur == 0xE3 {
            (0xE4u8, vec![(rx, ry.saturating_add(1))], false)
        } else if cur == 0xE5 {
            (0xE6u8, vec![(rx, ry.wrapping_sub(1))], false)
        } else if cur < 0xDC {
            (0xDCu8, vec![(rx, ry.wrapping_sub(1)), (rx, ry.saturating_add(1))], false)
        } else if (0xDC..=0xDE).contains(&cur) {
            (0xE8u8, vec![(rx, ry.wrapping_sub(1)), (rx, ry.saturating_add(1))], true)
        } else {
            return StateOutcome::NoChange;
        };

        let mut destroyed: Vec<(u16, u16)> = Vec::new();
        let mut actions: Vec<((u16, u16), usize, CellAction)> = Vec::new();

        for (slot, pos) in Self::ew_triple(rx, ry).enumerate() {
            if let Some(c) = self.cell_mut(pos.0, pos.1) {
                c.overlay_byte = next;
                if is_final {
                    c.damage_state = DamageState::Destroyed;
                    destroyed.push(pos);
                    actions.push((pos, slot, CellAction::BlowUpBridge));
                } else {
                    c.damage_state = DamageState::Damaged;
                }
            }
        }

        for (sx, sy) in siblings {
            if sy == u16::MAX { continue; }
            let mut sibling_finals = self.apply_bridge_destruction_ew_high(sx, sy);
            for pos in sibling_finals.drain(..) {
                if !destroyed.contains(&pos) {
                    destroyed.push(pos);
                    actions.push((pos, 0, CellAction::BlowUpBridge));
                }
            }
        }

        if destroyed.is_empty() && !is_final {
            return StateOutcome::Absorbed;
        }

        let adj = compute_adjacent_bridges_dirty(rx, ry, Axis::EW);
        StateOutcome::Collapsed {
            destroyed_cells: destroyed,
            set_bridge_direction: SetBridgeDirectionResult { actions },
            adjacent_bridges_dirty: adj,
            zones_dirty: is_final,
        }
    }
```

**Step 5: Tests in walker.rs `mod tests`**

Tests must reflect the **3-cell perpendicular-triple** behavior, not 4-step linear walk:

```rust
    #[test]
    fn ns_walker_initial_writes_0xd3_to_triple() {
        // Setup: 3 NS body cells at (2, 0), (2, 1), (2, 2), all overlay 0xD0
        // (initial body, < 0xD3). Hit (2, 1) — expect (2, 0), (2, 1), (2, 2)
        // all → overlay 0xD3, damage_state = Damaged (NOT Destroyed; intermediate).
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            state.test_seed_cell(2, y, BridgeRuntimeCell {
                deck_present: true, destroyable: true, deck_level: 5,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: 0xD0,
            });
        }
        let terrain = ResolvedTerrainGrid::default();
        let outcome = state.destroy_bridge_walker_ns_high(2, 1, &terrain);
        assert!(matches!(outcome, StateOutcome::Absorbed | StateOutcome::Collapsed { zones_dirty: false, .. }));
        for y in 0..3 {
            let c = state.cell(2, y).unwrap();
            assert_eq!(c.overlay_byte, 0xD3, "y={} should transition to 0xD3", y);
            assert_eq!(c.damage_state, DamageState::Damaged);
        }
    }

    #[test]
    fn ns_walker_final_writes_0xe7_marks_destroyed_zones_dirty() {
        // Setup: cell (2, 1) at overlay 0xD4 (final-eligible range
        // [0xD3..=0xD5]). Hit it. Expect 0xE7 + Destroyed + zones_dirty.
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            state.test_seed_cell(2, y, BridgeRuntimeCell {
                deck_present: true, destroyable: true, deck_level: 5,
                bridge_group_id: Some(1),
                damage_state: DamageState::Damaged,
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: 0xD4,
            });
        }
        let terrain = ResolvedTerrainGrid::default();
        let outcome = state.destroy_bridge_walker_ns_high(2, 1, &terrain);
        match outcome {
            StateOutcome::Collapsed { destroyed_cells, zones_dirty, .. } => {
                assert!(zones_dirty);
                for y in 0..3 {
                    let c = state.cell(2, y).unwrap();
                    assert_eq!(c.overlay_byte, 0xE7);
                    assert_eq!(c.damage_state, DamageState::Destroyed);
                    assert!(destroyed_cells.contains(&(2, y)));
                }
            }
            _ => panic!("expected Collapsed, got {:?}", outcome),
        }
    }

    #[test]
    fn ns_walker_0xdf_special_case_writes_0xe0() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            state.test_seed_cell(2, y, BridgeRuntimeCell {
                deck_present: true, destroyable: true, deck_level: 5,
                bridge_group_id: Some(1),
                damage_state: DamageState::Damaged,
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: 0xDF,
            });
        }
        let terrain = ResolvedTerrainGrid::default();
        let _ = state.destroy_bridge_walker_ns_high(2, 1, &terrain);
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0xE0);
        }
    }
```

**Step 6: Re-verify tables in `bridge_specs.rs`**

`pick_destruction_overlay` already has tables for HIGH NS/EW. Confirm no extension needed; if `apply_bridge_destruction_*` paths require a 0xE7/0xE8 entry that's not in the existing table, extend per HIGH §11.2.

**Step 7: Verify**

```
cargo test --lib bridge_state::walker -- --nocapture
```

**Step 8: Commit**

```
git commit -m "sim/bridge_state/walker: HIGH walkers (3-cell length-axis triple + perpendicular sibling cascade) + apply_bridge_destruction helpers + check_bridge_neighbors classifiers (mirror DestroyBridgeWalker/ApplyBridgeDestruction/CheckBridgeNeighbors at 0x0057CF60/0x0057D530/0x0057E7A0/0x0057ED00/0x0057CAB0/0x0057CBE0)"
```

---

### Task 8: LOW walker bodies (NS + EW) + LOW sibling-cascade helpers

**Why:** Mirror Task 7 for LOW bridges. Per Ghidra `DestroyBridge_Low @ 0x0057BAA0`, LOW has axis split (NS / EW walkers, NOT uniform) — same shape as HIGH, just shifted overlay ranges. LOW NS overlay range `[0x4A..=0x52] ∪ [0x5C..=0x5F] ∪ {0x64}`; LOW EW range `[0x53..=0x5B] ∪ [0x60..=0x63] ∪ {0x65}`. Existing `DESTRUCTION_OVERLAY_LOW_NS` / `_LOW_EW` tables in [bridge_specs.rs:439-451](../../src/sim/bridge_specs.rs#L439-L451) already shipped — reuse via `pick_destruction_overlay`.

**Files:**
- Modify: `src/sim/bridge_state/walker.rs`

**Step 0: Decompile LOW walker case-mapping**

Before implementing, decompile in Ghidra to capture exact case values:
- `MapClass__DestroyBridgeWalker_NS_Low @ ?` (search via xref from `DestroyBridge_Low @ 0x0057BAA0`)
- `MapClass__DestroyBridgeWalker_EW_Low @ ?`
- `MapClass__ApplyBridgeDestruction_NS_Low @ 0x0057DD50` (per existing comment in bridge_specs.rs)
- `MapClass__ApplyBridgeDestruction_EW_Low @ 0x0057E2A0`
- `MapClass__CheckBridgeNeighbors_EW_Low @ 0x0057B870`
- `MapClass__CheckBridgeNeighbors_NS_Low @ 0x0057B990`

Capture for each LOW walker:
- The 4 case values (mid×2, initial, final-eligible) and their target overlays
- Exact perpendicular sibling offsets (likely same shape as HIGH: NS walker recurses to (rx±1, ry); EW walker recurses to (rx, ry±1))

Per existing bridge_specs.rs:438-450 commentary:
- LOW NS: progressive intermediates `0x5C → 0x5D`, `0x5E → 0x5F`; final = `0x64`
- LOW EW: progressive intermediates `0x60 → 0x61`, `0x62 → 0x63`; final = `0x65`

So expected case structure (verify via decompile):
- LOW NS walker: `0x5C → 0x5D`; `0x5E → 0x5F`; `< 0x?? → 0x??` (initial); `[0x??..=0x??] → 0x64` (final)
- LOW EW walker: `0x60 → 0x61`; `0x62 → 0x63`; `< 0x?? → 0x??` (initial); `[0x??..=0x??] → 0x65` (final)

**Step 1: Implement LOW neighbor classifiers**

```rust
    /// LOW NS perpendicular neighbor classifier. Mirror of binary
    /// `MapClass__CheckBridgeNeighbors_EW_Low @ 0x0057B870`.
    /// Bit-encoding mirrors HIGH classifier; overlay values shifted to
    /// LOW range (0x4A..=0x65). Exact overlay sets per Ghidra spot-check
    /// at Step 0.
    pub(super) fn check_bridge_neighbors_ew_low(&self, rx: u16, ry: u16) -> u8 {
        // TODO: populate switch arms from Step 0 decompile.
        // Pattern: bits 0,1 from east; bits 2,3 from west.
        let _ = (rx, ry);
        0
    }

    pub(super) fn check_bridge_neighbors_ns_low(&self, rx: u16, ry: u16) -> u8 {
        // TODO: populate switch arms from Step 0 decompile.
        let _ = (rx, ry);
        0
    }
```

**Step 2: Implement LOW sibling-cascade helpers**

```rust
    fn apply_bridge_destruction_ns_low(&mut self, rx: u16, ry: u16) -> Vec<(u16, u16)> {
        use crate::sim::bridge_specs::pick_destruction_overlay;
        use crate::sim::bridge_state::DamageState;
        let mut final_cells = Vec::new();
        let Some(cell) = self.cell(rx, ry).copied() else { return final_cells; };
        let cur = cell.overlay_byte;
        if !(0x4A..=0x65).contains(&cur) { return final_cells; }
        let idx = self.check_bridge_neighbors_ew_low(rx, ry);
        if idx == 0 { return final_cells; }
        // Same dual-mode as HIGH:
        // - cur < intermediate-zone → table lookup
        // - cur in intermediate zone → fixed advance
        // - cur ≥ final → no-op
        let next = if cur < 0x5C {
            match pick_destruction_overlay(idx, Axis::NS, false) {
                Some(n) if n != cur => n,
                _ => return final_cells,
            }
        } else if cur == 0x5C { 0x5D }
        else if cur == 0x5E { 0x5F }
        else { return final_cells; };
        for pos in Self::ns_triple(rx, ry) {
            if let Some(c) = self.cell_mut(pos.0, pos.1) {
                c.overlay_byte = next;
                if next == 0x64 {
                    c.damage_state = DamageState::Destroyed;
                    final_cells.push(pos);
                } else {
                    c.damage_state = DamageState::Damaged;
                }
            }
        }
        final_cells
    }

    fn apply_bridge_destruction_ew_low(&mut self, rx: u16, ry: u16) -> Vec<(u16, u16)> {
        // Mirror of _ns_low with EW triple, EW table, EW intermediates 0x60/0x62.
        use crate::sim::bridge_specs::pick_destruction_overlay;
        use crate::sim::bridge_state::DamageState;
        let mut final_cells = Vec::new();
        let Some(cell) = self.cell(rx, ry).copied() else { return final_cells; };
        let cur = cell.overlay_byte;
        if !(0x4A..=0x65).contains(&cur) { return final_cells; }
        let idx = self.check_bridge_neighbors_ns_low(rx, ry);
        if idx == 0 { return final_cells; }
        let next = if cur < 0x60 {
            match pick_destruction_overlay(idx, Axis::EW, false) {
                Some(n) if n != cur => n,
                _ => return final_cells,
            }
        } else if cur == 0x60 { 0x61 }
        else if cur == 0x62 { 0x63 }
        else { return final_cells; };
        for pos in Self::ew_triple(rx, ry) {
            if let Some(c) = self.cell_mut(pos.0, pos.1) {
                c.overlay_byte = next;
                if next == 0x65 {
                    c.damage_state = DamageState::Destroyed;
                    final_cells.push(pos);
                } else {
                    c.damage_state = DamageState::Damaged;
                }
            }
        }
        final_cells
    }
```

**Step 3: Implement LOW walkers**

`destroy_bridge_walker_ns_low` and `_ew_low` — same shape as their HIGH counterparts (Task 7 Steps 3 + 4) but with LOW case values from Step 0 decompile. Replace Task 6 stubs.

```rust
    pub(super) fn destroy_bridge_walker_ns_low(
        &mut self,
        rx: u16,
        ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        use crate::sim::bridge_specs::{CellAction, SetBridgeDirectionResult};
        use crate::sim::bridge_state::{compute_adjacent_bridges_dirty, DamageState};

        let Some(cell) = self.cell(rx, ry).copied() else { return StateOutcome::NoChange; };
        let cur = cell.overlay_byte;

        // Case structure verified at Step 0 — fill in from decompile.
        // Expected pattern (parallels HIGH NS):
        //   cur == 0x5C → 0x5D, sibling cascade at (rx-1, ry)
        //   cur == 0x5E → 0x5F, sibling cascade at (rx+1, ry)
        //   cur < 0x?? → 0x?? (initial → first stage), siblings BOTH
        //   cur ∈ [0x??..=0x??] → 0x64 (FINAL), siblings BOTH, FindBridgeEndpoints
        let (next, siblings, is_final): (u8, Vec<(u16, u16)>, bool) = match cur {
            0x5C => (0x5D, vec![(rx.wrapping_sub(1), ry)], false),
            0x5E => (0x5F, vec![(rx.saturating_add(1), ry)], false),
            // TODO: confirm initial / final ranges from Step 0 decompile.
            // Placeholder values — VERIFY before merging:
            v if v < 0x4F => (0x4F, vec![(rx.wrapping_sub(1), ry), (rx.saturating_add(1), ry)], false),
            v if (0x4F..=0x52).contains(&v) => (0x64, vec![(rx.wrapping_sub(1), ry), (rx.saturating_add(1), ry)], true),
            _ => return StateOutcome::NoChange,
        };

        let mut destroyed: Vec<(u16, u16)> = Vec::new();
        let mut actions: Vec<((u16, u16), usize, CellAction)> = Vec::new();

        for (slot, pos) in Self::ns_triple(rx, ry).enumerate() {
            if let Some(c) = self.cell_mut(pos.0, pos.1) {
                c.overlay_byte = next;
                if is_final {
                    c.damage_state = DamageState::Destroyed;
                    destroyed.push(pos);
                    actions.push((pos, slot, CellAction::BlowUpBridge));
                } else {
                    c.damage_state = DamageState::Damaged;
                }
            }
        }

        for (sx, sy) in siblings {
            if sx == u16::MAX { continue; }
            let mut sibling_finals = self.apply_bridge_destruction_ns_low(sx, sy);
            for pos in sibling_finals.drain(..) {
                if !destroyed.contains(&pos) {
                    destroyed.push(pos);
                    actions.push((pos, 0, CellAction::BlowUpBridge));
                }
            }
        }

        if destroyed.is_empty() && !is_final {
            return StateOutcome::Absorbed;
        }
        let adj = compute_adjacent_bridges_dirty(rx, ry, Axis::NS);
        StateOutcome::Collapsed {
            destroyed_cells: destroyed,
            set_bridge_direction: SetBridgeDirectionResult { actions },
            adjacent_bridges_dirty: adj,
            zones_dirty: is_final,
        }
    }

    pub(super) fn destroy_bridge_walker_ew_low(
        &mut self,
        rx: u16,
        ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        // Mirror of `_ns_low` with EW intermediates (0x60, 0x62), EW final (0x65),
        // EW siblings (rx, ry±1). Fill case mapping from Step 0 decompile.
        // ... (parallel structure) ...
        let _ = (rx, ry);
        StateOutcome::NoChange
    }
```

**Step 4: Test**

```rust
    #[test]
    fn low_ns_walker_intermediate_5c_writes_5d() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            state.test_seed_cell(2, y, BridgeRuntimeCell {
                deck_present: true, destroyable: true, deck_level: 2,
                bridge_group_id: Some(2),
                damage_state: DamageState::Damaged,
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(2),
                overlay_byte: 0x5C,
            });
        }
        let terrain = ResolvedTerrainGrid::default();
        let _ = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0x5D);
        }
    }
```

**Step 5: Verify + commit**

```
cargo test --lib bridge_state::walker::tests::low -- --nocapture
git commit -m "sim/bridge_state/walker: LOW walkers (NS+EW) + sibling-cascade helpers + neighbor classifiers (mirror DestroyBridgeWalker_*_Low / ApplyBridgeDestruction_*_Low / CheckBridgeNeighbors_*_Low)"
```

---

### --- Phase F2 END — Drivers + classifier ready. Build green. ---

---

### Phase F3 — Orchestrator + cascade + wire-up + legacy deletion

### Task 9: `bridge_orchestrator.rs` scaffolding + 4-path dispatcher entry

**Why:** Lands the new orchestrator file with the 4-path dispatcher loop, RNG-gate logic, IonCannon retry loop, but leaves cascade consumers stubbed for Tasks 10-13. Outer hook to world layer wired only at Task 14 — additive until then.

**Files:**
- Create: `src/sim/world/bridge_orchestrator.rs`
- Modify: `src/sim/world/mod.rs` (add `pub(crate) mod bridge_orchestrator;` near top)

**Pattern:** mirror `apply_wall_damage_events` shape at [world/mod.rs:701](../../src/sim/world/mod.rs#L701).

**Step 1: Wire the submodule**

In `src/sim/world/mod.rs` near other `mod` declarations:

```rust
pub(crate) mod bridge_orchestrator;
```

**Step 2: Create `src/sim/world/bridge_orchestrator.rs`**

```rust
//! Bridge damage orchestrator — 4-path dispatcher + cascade consumers.
//!
//! Mirror of binary `Apply_area_damage @ 0x00489280` for the bridge subset.
//! Owns the per-event dispatch (HighSM → LowSM → LowDirect → HighDirect),
//! per-path BridgeStrength RNG gate, IonCannon retry loop (state-machine
//! paths only), and the BlowUpBridge cascade (ground kill, deck DropIn,
//! debris spawn, rim refresh, trigger 31, zone rebuild).
//!
//! ## Dependency rules
//! Same as sim/world: depends on sim/bridge_state, sim/rng, rules/, map/;
//! never render/ui/audio/net.

use std::collections::BTreeSet;

use crate::rules::ruleset::RuleSet;
use crate::sim::bridge_state::{
    BridgeDamageContext, BridgeDamageEvent, DispatchPath, StateOutcome,
};
use crate::sim::world::Simulation;

/// Drain a batch of `BridgeDamageEvent`s through the 4-path dispatcher.
/// Replaces the legacy `Simulation::apply_bridge_damage_events` AND
/// `Simulation::resolve_bridge_state_changes` — orchestrator owns dispatch
/// + cascade in one pass.
///
/// Returns the list of despawned entity IDs. Per the DropIn correction
/// (HIGH §12.7), this is typically empty — bridge-deck entities survive
/// stranded rather than despawning when the destination is not walkable.
pub(crate) fn apply_bridge_damage_events(
    sim: &mut Simulation,
    rules: &RuleSet,
    events: &[BridgeDamageEvent],
) -> Vec<u64> {
    let mut despawned_ids: Vec<u64> = Vec::new();
    if events.is_empty() {
        return despawned_ids;
    }

    // Take terrain by reference (immutable; outer borrow allows &mut bridge_state).
    let Some(terrain) = sim.resolved_terrain.as_ref() else { return despawned_ids; };

    // Outer gate: SpecialFlags::DestroyableBridges. If clear, bridges immune.
    let bridge_state = match sim.bridge_state.as_ref() {
        Some(bs) if bs.is_destroyable() => bs,
        _ => return despawned_ids,
    };
    let bridge_strength = bridge_state.bridge_strength();
    let _c4_id = rules.c4_warhead_id();

    // Phase F-3 stub — actual outcome aggregation lands in Task 14 wire-up.
    // For Task 9, dispatcher loop runs without cascade application.
    let _outcomes: Vec<StateOutcome> = run_dispatch_loop(sim, rules, events, terrain, bridge_strength);

    despawned_ids
}

/// Inner dispatch loop. Pure orchestration — reads events, runs the 4-path
/// dispatcher with RNG gate + IonCannon retry per path, collects
/// `StateOutcome`s for the caller to drain.
fn run_dispatch_loop(
    sim: &mut Simulation,
    _rules: &RuleSet,
    events: &[BridgeDamageEvent],
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    bridge_strength: u16,
) -> Vec<StateOutcome> {
    let mut outcomes = Vec::new();
    let bridge_state = sim.bridge_state.as_mut().expect("destroyable_flag check above");

    for event in events {
        let ctx = BridgeDamageContext {
            damage: event.damage,
            warhead_ref: event.warhead_ref,
            is_ion_cannon: event.is_ion_cannon,
            bridge_strength,
            impact_z: event.impact_z,
        };

        // 4 paths in fixed order — verification doc §1.
        for path in [
            DispatchPath::HighStateMachine,
            DispatchPath::LowStateMachine,
            DispatchPath::LowDirect,
            DispatchPath::HighDirect,
        ] {
            if !bridge_state.path_matches_cell(path, event.rx, event.ry, &ctx, terrain) {
                continue;
            }

            // RNG gate. IonCannon bypasses.
            if !ctx.is_ion_cannon {
                let roll = sim.rng.next_range_u32_inclusive(1, ctx.bridge_strength as u32);
                if !(roll < ctx.damage as u32) {
                    continue;
                }
            }

            // Retry: state-machine paths get 3 retries on IonCannon; direct-
            // overlay paths are single-shot regardless (verification Finding 1).
            let max_attempts = if ctx.is_ion_cannon && path.is_state_machine() { 4 } else { 1 };
            for _attempt in 0..max_attempts {
                let outcome = match path {
                    DispatchPath::HighStateMachine => {
                        // Anchor / Body / Tail → body driver; Bridgehead → bridgehead driver.
                        // We don't know the role without re-reading; classifier already
                        // confirmed "body-or-bridgehead high cell" — dispatch by role.
                        match bridge_state.cell(event.rx, event.ry).map(|c| c.role) {
                            Some(crate::sim::bridge_state::BridgeCellRole::Bridgehead) => {
                                bridge_state.bridgehead_advance_state(event.rx, event.ry, true, terrain)
                            }
                            _ => bridge_state.body_cell_advance_state(event.rx, event.ry, true),
                        }
                    }
                    DispatchPath::LowStateMachine => {
                        match bridge_state.cell(event.rx, event.ry).map(|c| c.role) {
                            Some(crate::sim::bridge_state::BridgeCellRole::Bridgehead) => {
                                bridge_state.bridgehead_advance_state(event.rx, event.ry, false, terrain)
                            }
                            _ => bridge_state.body_cell_advance_state(event.rx, event.ry, false),
                        }
                    }
                    DispatchPath::HighDirect => bridge_state.destroy_bridge_high(event.rx, event.ry, terrain),
                    DispatchPath::LowDirect => bridge_state.destroy_bridge_low(event.rx, event.ry, terrain),
                };
                if !matches!(outcome, StateOutcome::NoChange) {
                    outcomes.push(outcome);
                    break;
                }
            }
        }
    }

    outcomes
}

#[cfg(test)]
mod tests {
    // Tests for the dispatcher live here; they construct Simulation fixtures
    // and exercise individual path-matches with seeded RNG. Cascade-consumer
    // tests live in world_tests.rs (integration-flavored).
}
```

**Step 3: Verify**

```
cargo build
cargo test --lib bridge_orchestrator -- --nocapture
```
Expected: builds green; module loads; no callers yet.

**Step 4: Commit**

```
git commit -m "sim/world/bridge_orchestrator: scaffolding + 4-path dispatcher loop with RNG gate + IonCannon retry (Phase F3 step 1; cascade consumers stubbed)"
```

---

### Task 10: `kill_ground_occupants_at` cascade helper

**Why:** Per HIGH §11.4 step 1: BlowUpBridge walks `+0xE4` (ground occupants) on each destroyed cell and force-kills via `ReceiveDamage(damage=0, C4Warhead, force_kill=1)`. InfDeath for the kill animation comes from C4Warhead. Currently absent from our Rust runtime — units below a collapsing bridge survive, which is a parity bug.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** entity-store iteration + force-kill via existing death pipeline.

**Step 1: Add helper**

```rust
/// Kill ground-layer entities at `(rx, ry)` with C4Warhead force-kill semantics.
/// Mirror of binary `BlowUpBridge` step 1 (HIGH §11.4): walks `+0xE4` ground
/// occupants and calls `ReceiveDamage(damage=0, C4Warhead, force_kill=1)`.
/// InfDeath is selected from C4Warhead's `InfDeath=` for animation.
///
/// Bridge-layer entities (those with `is_on_bridge_layer() == true`) are
/// untouched here — they go through `drop_in_bridge_deck_entities` per HIGH §12.7.
fn kill_ground_occupants_at(
    sim: &mut Simulation,
    rx: u16,
    ry: u16,
    c4_warhead_id: crate::sim::intern::InternedId,
) {
    let _ = c4_warhead_id; // reserved for InfDeath lookup once death-pipeline accepts warhead-as-killing-warhead
    let victims: Vec<u64> = sim
        .entities
        .iter_sorted()
        .filter(|(_, e)| {
            e.position.rx == rx
                && e.position.ry == ry
                && !e.is_on_bridge_layer()
                && e.health.current > 0
        })
        .map(|(id, _)| id)
        .collect();
    for id in victims {
        if let Some(entity) = sim.entities.get_mut(id) {
            entity.health.current = 0;
            entity.dying = true;
            entity.attack_target = None;
            entity.movement_target = None;
            entity.selected = false;
        }
    }
}
```

**Step 2: Wire into orchestrator (replace dispatch-only loop)**

Update `apply_bridge_damage_events` so that after `run_dispatch_loop` returns, the cascade consumes `outcomes`. For Task 10, only the ground-kill step is wired:

```rust
pub(crate) fn apply_bridge_damage_events(
    sim: &mut Simulation,
    rules: &RuleSet,
    events: &[BridgeDamageEvent],
) -> Vec<u64> {
    let mut despawned_ids: Vec<u64> = Vec::new();
    if events.is_empty() { return despawned_ids; }
    let Some(_terrain) = sim.resolved_terrain.as_ref() else { return despawned_ids; };
    let bridge_state_ok = sim.bridge_state.as_ref().is_some_and(|bs| bs.is_destroyable());
    if !bridge_state_ok { return despawned_ids; }
    let bridge_strength = sim.bridge_state.as_ref().unwrap().bridge_strength();
    let c4_id = rules.c4_warhead_id();

    // Borrow gymnastics: dispatch loop needs &mut sim; cascade also needs &mut sim.
    // Drop the terrain ref by re-fetching inside the loop (cheap pointer copy).
    let outcomes = {
        // shadow re-bind for the subscope:
        let terrain_ref: *const _ = sim.resolved_terrain.as_ref().unwrap();
        // SAFETY: terrain is &'a inside Simulation; we hold a &mut to sim only
        // for the duration of run_dispatch_loop, which itself only reads
        // terrain. No aliasing — terrain is not mutated by the dispatcher.
        let terrain = unsafe { &*terrain_ref };
        run_dispatch_loop(sim, rules, events, terrain, bridge_strength)
    };

    // Aggregate destroyed cells + BlowUpBridge cells from outcomes.
    let mut destroyed_set: BTreeSet<(u16, u16)> = BTreeSet::new();
    let mut blow_up_cells: BTreeSet<(u16, u16)> = BTreeSet::new();
    for outcome in &outcomes {
        if let StateOutcome::Collapsed { destroyed_cells, set_bridge_direction, .. } = outcome {
            destroyed_set.extend(destroyed_cells.iter().copied());
            for (cell, _slot, action) in &set_bridge_direction.actions {
                if matches!(action, crate::sim::bridge_specs::CellAction::BlowUpBridge) {
                    blow_up_cells.insert(*cell);
                    destroyed_set.insert(*cell);
                }
            }
        }
    }

    // Cascade Step 1: ground-occupant kill (BlowUpBridge step 1, HIGH §11.4).
    for &(rx, ry) in &blow_up_cells {
        kill_ground_occupants_at(sim, rx, ry, c4_id);
    }

    // Cascade Steps 2-6 land in Tasks 11-13.

    despawned_ids
}
```

**Note on the SAFETY block:** the unsafe ptr-cast is a pragmatic workaround for the
`&mut sim` + `&sim.resolved_terrain` borrow conflict. An alternative is to
restructure so the dispatch loop takes `&mut bridge_state` + `&terrain` directly
and the orchestrator threads them in — but that requires splitting the borrow
of `sim` into projections. Pick the cleanest pattern at implementation time:
pass `&mut sim.bridge_state.unwrap()` + `&sim.resolved_terrain.unwrap()` +
`&mut sim.rng` separately to `run_dispatch_loop`, eliminating the unsafe.

**Step 3: Test**

Add to the orchestrator's `mod tests` (or extend world_tests):

```rust
    #[test]
    fn cascade_kills_ground_occupants_under_destroyed_cell() {
        // Build a Simulation with a bridge cell (5, 5) that's destroyed,
        // and a ground-layer entity at (5, 5). After cascade, entity must
        // be dying.
        // ... fixture setup ...
        // Assert: entity.health.current == 0 && entity.dying == true.
    }
```

**Step 4: Verify + commit**

```
cargo build
cargo test --lib bridge_orchestrator -- --nocapture
git commit -m "sim/world/bridge_orchestrator: kill_ground_occupants_at cascade — C4Warhead force-kill mirroring BlowUpBridge step 1"
```

---

### Task 11: `drop_in_bridge_deck_entities` cascade helper (DropIn correction)

**Why:** Per HIGH §12.7, bridge-deck entities go through `DropIn`: snap to ground level, clear OnBridge, NO damage, NO despawn. Our current `resolve_bridge_state_changes` despawns deck entities when the destination is not walkable — vanilla units survive stranded (HIGH §12.9: no drown / fall damage). This is a parity bug fix.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Step 1: Add helper**

```rust
/// Snap bridge-deck entities to ground level. Mirror of binary
/// `BlowUpBridge` step 2 (HIGH §11.4 + §12.7): walks `+0xE8` and calls
/// `DropIn` on each. Per HIGH §12.7 / §12.9: NO damage, NO despawn —
/// units survive stranded even when the destination is not walkable.
/// Vanilla has no drown mechanism.
fn drop_in_bridge_deck_entities(sim: &mut Simulation, rx: u16, ry: u16) {
    use crate::sim::components::{GroundMovePhase, MovementLayer};

    let ground_level = sim
        .resolved_terrain
        .as_ref()
        .and_then(|t| t.cell(rx, ry))
        .map(|c| c.level)
        .unwrap_or(0);

    let to_snap: Vec<u64> = sim
        .entities
        .iter_sorted()
        .filter(|(_, e)| {
            e.position.rx == rx && e.position.ry == ry && e.is_on_bridge_layer()
        })
        .map(|(id, _)| id)
        .collect();

    for id in to_snap {
        if let Some(entity) = sim.entities.get_mut(id) {
            entity.bridge_occupancy = None;
            entity.on_bridge = false;
            entity.position.z = ground_level;
            entity.position.refresh_screen_coords();
            entity.movement_target = None; // stop in place
            if let Some(ref mut loco) = entity.locomotor {
                loco.layer = MovementLayer::Ground;
                loco.phase = GroundMovePhase::Idle;
            }
        }
    }
}
```

**Step 2: Wire into orchestrator after Step 1**

In `apply_bridge_damage_events`, after `kill_ground_occupants_at`:

```rust
    // Cascade Step 2: bridge-deck DropIn (HIGH §12.7 — NEVER despawn).
    for &(rx, ry) in &destroyed_set {
        drop_in_bridge_deck_entities(sim, rx, ry);
    }
```

**Step 3: Test (in world_tests.rs)**

```rust
    #[test]
    fn cascade_drops_in_deck_entity_no_despawn_even_when_water_below() {
        // Fixture: bridge cell over water (ground-cell unwalkable).
        // Spawn an MTNK on the deck. Trigger collapse.
        // Assert: entity still in EntityStore (NOT despawned), entity.on_bridge == false,
        // entity.position.z == ground_level.
    }
```

**Step 4: Verify + commit**

```
cargo test --lib world_tests::cascade_drops_in -- --nocapture
git commit -m "sim/world/bridge_orchestrator: drop_in_bridge_deck_entities — snap-and-survive (CORRECTION; vanilla never despawns per HIGH §12.7)"
```

---

### Task 12: `spawn_bridge_debris` cascade helper (debris correction)

**Why:** Per HIGH §11.4 step 4: per-cell 95% outer gate, 2 jitter draws (RNG-order parity), 50% MetallicDebris (no delay) gated by `voxel_max > 0`, 1 always BridgeExplosion (delay 1-5 frames). Current `spawn_bridge_explosions` has wrong shape (1 immediate BridgeExplosion + 50% delayed BridgeExplosion). This is a parity bug fix.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Step 1: Add helper**

```rust
/// Per-cell debris spawn. Mirror of binary `BlowUpBridge` step 4 (HIGH §11.4).
/// RNG draw order is parity-critical:
/// 1. Outer 95% gate (`next_range_u32(20)`)
/// 2. 2 jitter draws (`next_range_u32(0xFFFF)` × 2) — values discarded but
///    consumed for RNG-order parity with binary
/// 3. MetallicDebris 50% gate (`next_range_u32(2)`)
/// 4. Optional MetallicDebris slot (`next_range_u32(metallic_count)`) — only if 50% pass
///    AND `voxel_max > 0` AND `metallic_count > 0`
/// 5. Explosion delay (`next_range_u32_inclusive(1, 5)`)
/// 6. Explosion slot (`next_range_u32(explosion_count)`)
fn spawn_bridge_debris(
    sim: &mut Simulation,
    rules: &RuleSet,
    cells: &BTreeSet<(u16, u16)>,
) {
    use crate::sim::components::WorldEffect;

    let explosion_count = sim.bridge_explosions.len() as u32;
    let metallic_count = sim.metallic_debris.len() as u32;
    let voxel_max = rules.bridge_rules.voxel_max as u32;

    if explosion_count == 0 && metallic_count == 0 { return; }

    for &(rx, ry) in cells {
        // Step 1: outer 95% gate.
        if sim.rng.next_range_u32(20) == 0 { continue; }

        // Step 2: 2 jitter draws (consumed for RNG-order parity).
        let _jitter_x = sim.rng.next_range_u32(0xFFFF);
        let _jitter_y = sim.rng.next_range_u32(0xFFFF);

        let deck_level = sim
            .resolved_terrain
            .as_ref()
            .and_then(|t| t.cell(rx, ry))
            .map(|c| c.bridge_deck_level_if_any().unwrap_or(c.level))
            .unwrap_or(0);

        // Step 3: MetallicDebris 50% gate.
        let metallic_pass = sim.rng.next_range_u32(2) == 0;
        if metallic_pass && voxel_max > 0 && metallic_count > 0 {
            // Step 4: MetallicDebris slot pick + spawn (no delay).
            let idx = sim.rng.next_range_u32(metallic_count) as usize;
            let anim_id = sim.metallic_debris[idx];
            let frames = sim.effect_frame_counts.get(&anim_id).copied().unwrap_or(20);
            sim.world_effects.push(WorldEffect {
                shp_name: anim_id,
                rx, ry, z: deck_level,
                frame: 0, total_frames: frames,
                rate_ms: 67, elapsed_ms: 0,
                translucent: true, delay_ms: 0,
            });
        }

        // Step 5 + 6: BridgeExplosion (always — when explosion list non-empty).
        if explosion_count > 0 {
            let delay_frames = sim.rng.next_range_u32_inclusive(1, 5);
            let idx = sim.rng.next_range_u32(explosion_count) as usize;
            let anim_id = sim.bridge_explosions[idx];
            let frames = sim.effect_frame_counts.get(&anim_id).copied().unwrap_or(20);
            sim.world_effects.push(WorldEffect {
                shp_name: anim_id,
                rx, ry, z: deck_level,
                frame: 0, total_frames: frames,
                rate_ms: 67, elapsed_ms: 0,
                translucent: true,
                delay_ms: delay_frames * 67,
            });
        }
    }
}
```

**Step 2: Wire into orchestrator after Step 2**

```rust
    // Cascade Step 3: debris spawn (HIGH §11.4 step 4).
    spawn_bridge_debris(sim, rules, &destroyed_set);
```

**Step 3: Test**

```rust
    #[test]
    fn debris_consumes_correct_rng_count_per_cell() {
        // Seeded RNG; 1 destroyed cell; force 95% gate to pass via known seed.
        // Assert RNG state-after matches expected after exactly:
        //   1 (outer gate) + 2 (jitter) + 1 (metallic 50%) +
        //   {0 or 1} (metallic slot) + 1 (delay) + 1 (slot) = 6 or 7 draws.
    }

    #[test]
    fn debris_skipped_when_voxel_max_zero() {
        // BridgeRules.voxel_max = 0 → no MetallicDebris spawn even on 50% pass.
    }
```

**Step 4: Verify + commit**

```
cargo test --lib world_tests::debris -- --nocapture
git commit -m "sim/world/bridge_orchestrator: spawn_bridge_debris (CORRECTION — 50% MetallicDebris no-delay + 1 always BridgeExplosion delayed; replaces wrong-shape spawn_bridge_explosions)"
```

---

### Task 13: Rim refresh + trigger 31 + zone rebuild stubs

**Why:** Wire the remaining cascade steps. `update_adjacent_bridges` may stay a stub if renderer is neighbor-aware (resolve at impl time by reading `src/render/`). `notify_bridge_span_collapse` is a no-op stub for trigger system (HIGH §11.3 — no-op on skirmish). `refresh_bridge_zones_if_dirty` reuses existing `Simulation::rebuild_zone_grid`.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Step 1: Investigate renderer for `update_adjacent_bridges` decision**

```
grep -rn "BridgeRuntimeCell\|damage_state\|bridge_layer" src/render/ | head -50
```

Look for whether the bridge renderer queries a cell's neighbors or only the cell itself. If neighbor-aware → stub is correct. If per-cell → needs a `rim_dirty` flag the renderer reads.

Document the finding inline in the helper's docstring.

**Step 2: Add helpers**

```rust
/// Rim refresh hook. Mirror of binary `MapClass::UpdateAdjacentBridges_High @ 0x00576770`.
/// Walks 8 neighbors of changed cells; re-evaluates bridge-edge tile
/// classification (height 5/7/8/12) per HIGH §11.9.
///
/// Tier 2: stub iff our renderer is neighbor-aware (resolved at impl time
/// by reading src/render/). If renderer is per-cell-only, this helper
/// must mutate a `rim_dirty` flag for re-evaluation downstream.
fn update_adjacent_bridges(sim: &mut Simulation, rim_cells: &BTreeSet<(u16, u16)>) {
    let _ = (sim, rim_cells); // resolved at implementation per Step 1 finding
}

/// TriggerEvent 31 broadcast. Mirror of binary
/// `MapClass::RepairBridgeSegment @ 0x00575EE0` (misnamed in binary —
/// actually `NotifyBridgeSpanCollapse`, HIGH §11.3 + §12.6).
/// No-op on skirmish maps (no triggers bound to event 31). Wired as a hook
/// for future campaign / map-trigger support.
fn notify_bridge_span_collapse(sim: &mut Simulation, cells: &BTreeSet<(u16, u16)>) {
    let _ = (sim, cells); // hook stub — see HIGH §11.3
}

/// Zone refresh. Per HIGH §12.8: `InvalidateBridgeZones` toggles
/// `BridgeEndpointRecord.active`; if any flipped, `UpdateBridgeZonesHelper`
/// rebuilds the full passability matrix. Reuses
/// `Simulation::rebuild_zone_grid` ([world/mod.rs:618](../../src/sim/world/mod.rs#L618))
/// which takes a `&PathGrid`. Build the path grid from post-collapse bridge
/// state (mirror of existing `fallout_ground_grid` construction at
/// [world/mod.rs:787](../../src/sim/world/mod.rs#L787)).
fn refresh_bridge_zones_if_dirty(sim: &mut Simulation, any_zones_dirty: bool) {
    if !any_zones_dirty { return; }
    use crate::sim::pathfinding::PathGrid;
    let Some(terrain) = sim.resolved_terrain.as_ref() else { return; };
    let path_grid =
        PathGrid::from_resolved_terrain_with_bridges(terrain, sim.bridge_state.as_ref());
    sim.rebuild_zone_grid(&path_grid);
}
```

**Step 3: Wire all into orchestrator**

In `apply_bridge_damage_events`, after Step 3 (debris):

```rust
    // Aggregate rim cells + zones-dirty flag from outcomes
    let mut rim_cells: BTreeSet<(u16, u16)> = BTreeSet::new();
    let mut any_zones_dirty = false;
    for outcome in &outcomes {
        if let StateOutcome::Collapsed { adjacent_bridges_dirty, zones_dirty, .. } = outcome {
            rim_cells.extend(adjacent_bridges_dirty.iter().copied());
            any_zones_dirty |= *zones_dirty;
        }
    }

    // Cascade Step 4: rim refresh (HIGH §11.9).
    update_adjacent_bridges(sim, &rim_cells);

    // Cascade Step 5: TriggerEvent 31 broadcast (HIGH §11.3 — no-op on skirmish).
    notify_bridge_span_collapse(sim, &destroyed_set);

    // Cascade Step 6: zone graph rebuild (HIGH §12.8).
    refresh_bridge_zones_if_dirty(sim, any_zones_dirty);
```

**Step 4: Verify + commit**

```
cargo build
git commit -m "sim/world/bridge_orchestrator: rim refresh + trigger 31 + zone rebuild cascade hooks (HIGH §11.3, §11.9, §12.8)"
```

---

### Task 14: Wire orchestrator into world tick + delete legacy

**Why:** Single switchover commit. Replaces world/mod.rs:1338-1340 calls with the new orchestrator entry. Deletes legacy `apply_bridge_damage_events`, `resolve_bridge_state_changes`, `spawn_bridge_explosions` from `Simulation`. Deletes `BridgeRuntimeState::apply_damage` + `group_hitpoints` + `strength_per_group` + `BridgeStateChange` struct. Migrates the 5 remaining test sites from `state.apply_damage(...)` to direct `cell_mut` mutation. **Atomic — leaves the build green.**

**Files:**
- Modify: `src/sim/world/mod.rs`
- Modify: `src/sim/bridge_state/mod.rs`
- Modify: `src/sim/bridge_specs.rs:454` (doc-comment update)
- Modify: `src/sim/bridge_state/mod.rs` internal tests (3 sites)
- Modify: `src/sim/pathfinding/core_tests.rs:588`
- Modify: `src/sim/production/production_placement_tests.rs:664`
- Modify: `src/sim/world/world_tests.rs` (6 fixtures — see Task 15 for test rewrites; minimum here = compile)

**Step 1: Replace the call site in `world/mod.rs`**

At [world/mod.rs:1337-1340](../../src/sim/world/mod.rs#L1337-L1340), replace:

```rust
            let bridge_changes =
                self.apply_bridge_damage_events(&combat_result.bridge_damage_events);
            // resolve_bridge_state_changes calls despawn_entity() internally.
            let _bridge_fallout_ids = self.resolve_bridge_state_changes(&bridge_changes);
```

with:

```rust
            let _bridge_fallout_ids = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
                self,
                rules,
                &combat_result.bridge_damage_events,
            );
```

**Step 2: Delete legacy methods on `Simulation`**

Delete:
- `apply_bridge_damage_events` at [world/mod.rs:677-691](../../src/sim/world/mod.rs#L677-L691)
- `resolve_bridge_state_changes` at [world/mod.rs:773-848](../../src/sim/world/mod.rs#L773-L848)
- `spawn_bridge_explosions` at [world/mod.rs:855-924](../../src/sim/world/mod.rs#L855-L924)

Update the `use` import at [world/mod.rs:32](../../src/sim/world/mod.rs#L32):

```rust
    BridgeDamageEvent, BridgeRuntimeState, DamageState,  // removed BridgeStateChange
```

**Step 3: Delete legacy on `BridgeRuntimeState`**

In `src/sim/bridge_state/mod.rs`:
- Delete `apply_damage` method (lines ~552-590)
- Delete `pub struct BridgeStateChange` (around line 229)
- Delete fields `group_hitpoints: BTreeMap<u16, u16>` and `strength_per_group: u16` from struct (lines ~318-319) and from constructor init block (lines ~469-482)

**Step 4: Update `bridge_specs.rs` doc comment**

At [bridge_specs.rs:454](../../src/sim/bridge_specs.rs#L454), update:

```rust
/// in `world::bridge_orchestrator::apply_bridge_damage_events` consumes these and dispatches.
```

**Step 5: Migrate 5 internal `state.apply_damage` test sites**

For each of:
- `bridge_state/mod.rs:1205, 1227, 1252` (3 sites in internal tests)
- `pathfinding/core_tests.rs:588`
- `production/production_placement_tests.rs:664`

Replace `state.apply_damage(BridgeDamageEvent { ... });` with direct mutation:

```rust
    if let Some(c) = state.cell_mut(rx, ry) {
        c.damage_state = crate::sim::bridge_state::DamageState::Destroyed;
    }
    // Mark associated endpoint records inactive if test depends on
    // pathfinding seeing the bridge as severed:
    // (search for BridgeEndpointRecord usage in the test fixture)
```

(Read each test to confirm what it asserts — most assert "pathfinding sees bridge as unwalkable" which `damage_state = Destroyed` already covers via `is_bridge_walkable`. If a test depends on group HP draining, simplify to "directly set damage_state".)

**Step 6: Update 6 world_tests fixtures (minimum to compile)**

In each of `world_tests.rs:413, 455, 500, 539, 578, 617`, replace:

```rust
    let changes = sim.apply_bridge_damage_events(&[BridgeDamageEvent { ... }]);
    let _ = sim.resolve_bridge_state_changes(&changes);
```

with:

```rust
    let _despawned = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent { ... is_ion_cannon: true ... }],
    );
```

The fixtures may need a `rules: RuleSet` in scope — search for existing `RuleSet::default()` patterns in the same file to construct a minimal one. Test-logic rewrites (assertion shape changes) land in Task 15.

**Step 7: Verify + commit**

```
cargo build
cargo test --lib -- --nocapture
```
Expected: full build green; tests pass (some assertions may need Task 15 to fully pass — for Task 14 the bar is "compiles and existing assertions hold for the simplest cases").

```
git commit -m "sim: switchover to bridge_orchestrator + delete legacy apply_damage / resolve_bridge_state_changes / spawn_bridge_explosions / BridgeStateChange / group_hitpoints (Phase F end of structural changes)"
```

---

### --- Phase F END — Orchestrator wired. Build green. ---

---

### Phase G — Tests

### Task 15: Migrate 6 `world_tests.rs` bridge fixtures to new orchestrator semantics

**Why:** Fixtures currently assert legacy single-shot collapse semantics (per-group HP). New orchestrator runs through state-machine drivers + walker + cascade; assertions need updating. Each fixture either uses `is_ion_cannon: true` (single-shot semantics, mirrors legacy behavior) or constructs the cell state to be in `DamageState::Damaged` so a single hit collapses.

**Files:**
- Modify: `src/sim/world/world_tests.rs:413-650` (the 6 bridge tests)

**Step 1: Read each fixture, rewrite assertions**

Each fixture follows the pattern:
```rust
    let changes = sim.apply_bridge_damage_events(&[...]);
    assert_eq!(changes.len(), 1);
    let _ = sim.resolve_bridge_state_changes(&changes);
    // ... assert pathfinding effect
```

Migrate to:
```rust
    // Pre-condition: place the test cell in DamageState::Damaged so a single
    // non-Ion-Cannon hit triggers collapse via the body-cell driver. OR use
    // is_ion_cannon: true to bypass the RNG gate and get single-shot semantics.
    let _despawned = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent { /* fields */, is_ion_cannon: true, impact_z: 5 }],
    );
    // Assert: bridge cell.damage_state is Destroyed, NOT count of changes.
    assert!(matches!(
        sim.bridge_state.as_ref().unwrap().cell(rx, ry).map(|c| c.damage_state),
        Some(DamageState::Destroyed)
    ));
```

**Step 2: Verify + commit**

```
cargo test --lib world_tests -- --nocapture
git commit -m "sim/world/world_tests: migrate 6 bridge fixtures to bridge_orchestrator (assertions on damage_state, not BridgeStateChange count)"
```

---

### Task 16: New integration tests + RNG draw-count parity test

**Why:** Cover the parity-critical behaviors that Tasks 10-13 introduce: ground-kill, DropIn-no-despawn, debris RNG-draw count, walker full-span collapse, multi-path mutual exclusion via overlay invariant.

**Files:**
- Modify: `src/sim/world/world_tests.rs`

**Step 1: Add tests**

```rust
    #[test]
    fn cascade_kills_ground_unit_under_destroyed_cell() {
        // Setup: bridge cell at (5, 5) with ground entity present.
        // Trigger: collapse via Ion-Cannon-flavored event.
        // Assert: ground entity health=0, dying=true.
    }

    #[test]
    fn cascade_drops_in_deck_unit_no_despawn_on_water_below() {
        // Setup: bridge cell over water, MTNK on deck.
        // Trigger: collapse.
        // Assert: entity still in EntityStore, on_bridge=false, position.z=ground_level.
    }

    #[test]
    fn debris_consumes_5_to_7_rng_draws_per_cell() {
        // Seeded RNG; force 95% gate pass via known seed; force 50% gate
        // path via known second draw.
        // Assert: RNG state-after matches expected for exactly N draws.
    }

    #[test]
    fn walker_collapses_full_high_span() {
        // Seed 4 raw-overlay (0xD0) NS body cells.
        // Trigger: hit cell 0 with non-IonCannon damage > BridgeStrength
        //          (so RNG gate passes deterministically).
        // Assert: all 4 cells damage_state = Destroyed.
    }

    #[test]
    fn multi_path_mutual_exclusion_via_overlay_invariant() {
        // Cell with overlay_byte = 0x6 (transitioned out of body range)
        // and damage_state = Damaged. HighSM matches; HighDirect does NOT.
        // Verify path_matches_cell returns true for HighSM, false for HighDirect.
    }
```

**Step 2: Verify + commit**

```
cargo test --lib world_tests::cascade -- --nocapture
git commit -m "sim/world/world_tests: integration tests for cascade (ground kill / DropIn / debris RNG / walker / multi-path)"
```

---

### Task 17: Determinism + snapshot regression

**Why:** Lock down the lockstep contract. State hash before + after collapse must match for identical inputs. Snapshot round-trip preserves all bridge state.

**Files:**
- Modify: `src/sim/world/world_tests.rs` (or `src/sim/snapshot/` tests if separate)

**Step 1: Add tests**

```rust
    #[test]
    fn bridge_collapse_is_deterministic_under_replay() {
        // Build two identical sims, identical seeds, identical events.
        // Run apply_bridge_damage_events on both.
        // Assert: state_hash() matches.
    }

    #[test]
    fn snapshot_roundtrip_preserves_bridge_state_after_collapse() {
        // Trigger collapse. Snapshot (serialize). Deserialize. Restore caches.
        // Assert: state_hash() matches; bridge cell damage_state preserved.
    }

    #[test]
    fn rng_draw_count_per_event_matches_dispatch_table() {
        // Cell matches HighSM only (transitioned overlay, in Z range, not raw).
        // Non-IonCannon: 1 RNG draw before dispatch. After cascade: + per-cell debris draws.
        // Cell matches HighSM + HighDirect via overlay-byte transition during dispatch
        // (raw → state-machine routes via walker). Assert RNG total matches.
    }
```

**Step 2: Verify + commit**

```
cargo test --lib world_tests::determinism -- --nocapture
cargo test
git commit -m "sim/world/world_tests: determinism + snapshot regression tests for bridge cascade"
```

---

### --- Phase G END — Tests green. Tier 2 Phase F lands. ---

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-07-bridges-tier2-phase-f-orchestrator-design.md](2026-05-07-bridges-tier2-phase-f-orchestrator-design.md)
- **Verification doc:** ra2-rust-game-docs/PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md (this session, 2026-05-07)
- **Master research:** ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md (sections §3.1, §4, §11.2, §11.3, §11.4, §11.6, §11.9, §12.5, §12.6, §12.7, §12.8, §12.9, §12.10, §14.17)
- **Prior research:** ra2-rust-game-docs/BRIDGE_RENDERING_GHIDRA_REPORT.md, ra2-rust-game-docs/BRIDGE_SYSTEM.md, ra2-rust-game-docs/CELLCLASS_ZONES_SPEED_BRIDGES.md (referenced for cell-flag semantics)
- **gamemd.exe addresses (kept here, not in Rust comments):**
  - `Apply_area_damage @ 0x00489280` — 4-path outer dispatcher
  - `ApplyDamageToCell @ 0x00587180` — inner dispatcher
  - `DestroyBridge_High @ 0x0057CCF0` — overlay-direct HIGH walker entry
  - `DestroyBridge_Low @ 0x0057BAA0` — overlay-direct LOW walker entry
  - `DestroyBridgeWalker_NS_High @ 0x0057CF60`, `_EW_High @ 0x0057D530`
  - `DestroyBridgeWalker_NS_Low @ ?`, `_EW_Low @ ?` (xrefs from `0x0057BAA0` — capture at Task 8 Step 0)
  - `ApplyBridgeDestruction_NS_High @ 0x0057E7A0`, `_EW_High @ 0x0057ED00`
  - `ApplyBridgeDestruction_NS_Low @ 0x0057DD50`, `_EW_Low @ 0x0057E2A0` (per existing comment in bridge_specs.rs)
  - `CheckBridgeNeighbors_EW_High @ 0x0057CAB0`, `_NS_High @ 0x0057CBE0`
  - `CheckBridgeNeighbors_EW_Low @ 0x0057B870`, `_NS_Low @ 0x0057B990`
  - `FindBridgeEndpoints_NS_High @ 0x0057DC20`, `_EW_High @ 0x0057DAF0`
  - `FindBridgeEndpoints_NS_Low @ 0x0057C990`, `_EW_Low @ 0x0057C870`
  - `ProcessBridgeDamageStateMachine_High @ 0x00576BA0` (mirrored by shipped Rust drivers)
  - `CellClass::BlowUpBridge @ 0x0047DD70` — kill ground + DropIn deck + debris
  - `MapClass::UpdateAdjacentBridges_High @ 0x00576770` — rim refresh
  - `MapClass::RepairBridgeSegment @ 0x00575EE0` — actually NotifyBridgeSpanCollapse
  - `Random__RandomRanged @ 0x0065C7E0` — lockstep RNG (mirrored by `SimRng`)
- **INI keys:**
  - `[CombatDamage] BridgeStrength=` (default 1500)
  - `[CombatDamage] IonCannonWarhead=` (default `IonCannonWH`)
  - `[CombatDamage] C4Warhead=` (default `Super`)
  - `[General] BridgeExplosions=` (default 7-anim list)
  - `[General] MetallicDebris=` (default 20-anim list)
  - `[General] BridgeVoxelMax=` (default 3) — gates MetallicDebris spawn
  - Warhead `Wall=` boolean — outer gate at combat boundary
- **Repo patterns mirrored:**
  - `Simulation::apply_wall_damage_events` at [src/sim/world/mod.rs:701](../../src/sim/world/mod.rs#L701) — orchestrator shape
  - `Simulation.bridge_explosions: Vec<InternedId>` pre-intern at [src/app_init_helpers.rs:361-369](../../src/app_init_helpers.rs#L361-L369)
- **Parent master plan:** [docs/plans/2026-05-07-bridges-tier2-damage-state-machine-plan.md](2026-05-07-bridges-tier2-damage-state-machine-plan.md) Tasks 18-26 (this plan supersedes them with corrections from the verification doc)
