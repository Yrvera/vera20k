# Force-Fire (Ctrl-Click) Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Bring Ctrl-click force-fire to gamemd.exe parity — armed units fire at the
clicked cell (including empty terrain), unarmed units fall through to Move, Alt+Ctrl
routes to attack-move, and the cursor reflects force-fire state live as Ctrl is held.

**Architecture:** Adds one new `Command::ForceAttackCell` variant and widens the
existing `AttackTarget.target` field from `u64` to a `TargetKind` enum (Entity vs
Cell). All sim math stays fixed-point. Modifier-key state remains in the app/render
layer; sim only sees the resolved command.

**Design Doc:** [docs/plans/2026-05-07-force-fire-ctrl-click-design.md](2026-05-07-force-fire-ctrl-click-design.md)

---

## Grounding Summary

**Docs (R1):** Three reports cover this end-to-end:
- `MouseClass_research.md` — `What_Action_OnCell` semantics, force-fire cursor mapping
  (§12.9), action-code table.
- `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` — `SelectBestObjectForAction`
  priority ladder, mixed-selection per-unit dispatch, action 0x33 / action 1
  distinction.
- `HOTKEY_SYSTEM_GHIDRA_REPORT.md §15` — modifier-key polling globals, Alt overrides
  Ctrl rule, attack-move = Alt+Ctrl.

**Ghidra (R2):** Three functions decompiled live during brainstorm:
- `0x00700600` `TechnoClass::What_Action_OnCell` — armed → action 1, unarmed →
  action 2; Alt clears Ctrl flag.
- `0x004AB9B0` `DisplayClass::BandBox_LeftUp` — order dispatch, calls
  `Selection::DispatchMultiUnitOrder` with `(target_obj, cell_coord, action)`.
- `0x004AE750` `Selection::DispatchMultiUnitOrder` — **direct doc-comment**:
  *"NO group-pathing, NO leader, NO formation movement — each unit resolves its
  own path from its own position."* Action 1 ≡ 0x33 in dispatch.

**Repo pattern (R3):** Existing `Command::ForceAttack { attacker_id, target_id }`
([src/sim/command.rs:45](../../src/sim/command.rs#L45)) and dispatch arm in
[src/sim/world/world_commands.rs:319](../../src/sim/world/world_commands.rs#L319)
are the template — new `ForceAttackCell` variant follows the same shape.

**INI (R4):** No INI keys drive force-fire — input-side feature only. Confirmed
empty grep for `MouseControl|ForceFire|MouseClass` across `ini/`.

**Discrepancy with design doc:** The design described `attack_target: Option<u64>`
widening to `Option<AttackTarget>`. Reality (verified during grounding):
`attack_target` is already `Option<AttackTarget>` where `AttackTarget` is a struct
holding `target: u64` plus combat state (cooldown_ticks, burst_remaining,
burst_delay_ticks) at [src/sim/combat/mod.rs:138](../../src/sim/combat/mod.rs#L138).
The design's chosen approach is still correct (cell-target as a first-class enum
variant); the implementation lives one level deeper — widening
`AttackTarget.target` from `u64` to a `TargetKind` enum, not the outer
`Option<…>`. Plan reflects the corrected understanding.

**Still unknown (deferred):**
- Exact mouse SHP cursor frame for action 0x33 vs action 1 force-fire — cosmetic
  only, use existing attack cursor.
- Disguise piercing (Mirage / Spy / chameleon) — blocked on disguise system not
  being implemented in Rust. Out-of-scope.

---

## Key Technical Decisions

- **Widen `AttackTarget.target` from `u64` to `TargetKind` enum** instead of adding
  a parallel `cell_target: Option<(u16,u16)>` field. — Type safety; matches gamemd's
  conceptual model. **Confidence: high.** **Source:** Approach A in design doc;
  inspection of [src/sim/combat/mod.rs:138](../../src/sim/combat/mod.rs#L138).

- **`Selection::DispatchMultiUnitOrder` skips group-destination spread** for
  force-fire-cell — every selected armed unit gets the same `(rx, ry)`. **Confidence:
  high.** **Source:** Ghidra `0x004AE750` direct doc-comment.

- **Alt+Ctrl = attack-move (NOT force-fire)**: detected at order resolution with
  `force_fire = is_ctrl_held && !is_alt_held`. **Confidence: high.** **Source:**
  Ghidra `0x00700600` line 0x700706.

- **Unarmed unit + force-fire-cell falls through to Move at order-resolution time**
  (client-side filter), with a sim-side defensive check that warn-logs and no-ops
  if a `ForceAttackCell` somehow reaches an unarmed unit. **Confidence: high.**
  **Source:** Ghidra `0x00700600` final branch + per-unit dispatch in `0x004AE750`.

- **Snapshot version bump 4 → 5**: `AttackTarget` field-shape change is a
  serialization break. **Confidence: high.** **Source:**
  [src/sim/snapshot.rs:16](../../src/sim/snapshot.rs#L16).

- **Aircraft handle Cell targets** by reusing `target_coords()` for position
  resolution. **Confidence: medium.** **Source:** repo inspection of
  [src/sim/aircraft/attack_mission.rs:49](../../src/sim/aircraft/attack_mission.rs#L49)
  and [src/sim/aircraft/mod.rs:302](../../src/sim/aircraft/mod.rs#L302) — both
  read `at.target` directly to look up target entity. Needs `TargetKind` match.
  Flag for `/review-plan` to verify aircraft mission flow handles Cell targets
  cleanly before Task 5 lands.

## Open Questions

### Resolved During Planning

- **What field-shape does `attack_target` actually have?** — `Option<AttackTarget>`
  where `AttackTarget` is a struct (not the bare `u64` the design assumed).
  Source: [src/sim/combat/mod.rs:138](../../src/sim/combat/mod.rs#L138).
- **Multi-unit cell distribution behavior** — gamemd issues identical cell coord
  to every selected unit; no spread. Source: Ghidra `0x004AE750`.
- **Action 1 vs 0x33 dispatch** — identical code path. One Rust command variant
  suffices. Source: Ghidra `0x004AE750` `if ((param_3 == 1) || (param_3 == 0x33))`.
- **Unarmed-unit force-fire result** — falls through to action 2 (Move). Source:
  Ghidra `0x00700600`.
- **Modifier polling timing** — gamemd re-polls during action determination;
  matches our existing per-click polling. Source: Ghidra `0x00700600`.

### Deferred to Implementation

- **Exact aircraft fire-at-cell behavior** — does the Harrier circle the cell
  until ammo runs out, or fire once and RTB? Existing aircraft attack mission
  loop dictates this; observe in-game during Task 16 verification.
- **Cursor frame selection for force-fire-on-empty-cell vs force-fire-on-target**
  — gamemd uses different SHP frames; we initially use the existing attack cursor
  for both. Track as cosmetic follow-up.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) | Widen `AttackTarget.target: u64` → `TargetKind` enum; update `target_coords` and `issue_attack_command` |
| Modify | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) | Add `issue_attack_cell_command` |
| Modify | [src/sim/aircraft/attack_mission.rs](../../src/sim/aircraft/attack_mission.rs) | Match `TargetKind` when reading target; reuse `target_coords` |
| Modify | [src/sim/aircraft/mod.rs](../../src/sim/aircraft/mod.rs) | Match `TargetKind` at line 302 lookup site |
| Modify | [src/sim/combat/combat_tests.rs](../../src/sim/combat/combat_tests.rs) | Test `at.target` reads → `TargetKind` matches |
| Modify | [src/sim/snapshot.rs](../../src/sim/snapshot.rs) | `SNAPSHOT_VERSION: 4 → 5` |
| Modify | [src/sim/command.rs](../../src/sim/command.rs) | Append `Command::ForceAttackCell` variant |
| Modify | [src/sim/world/world_commands.rs](../../src/sim/world/world_commands.rs) | Dispatch arm for `ForceAttackCell` |
| Modify | [src/app_input.rs](../../src/app_input.rs) | Add `is_alt_held` helper |
| Modify | [src/app_context_order.rs](../../src/app_context_order.rs) | Alt+Ctrl detection; `ForceAttackCell` emission; unarmed fall-through; shroud rejection; skip `group_destinations` for force-fire-cell |
| Modify | [src/app_cursor.rs](../../src/app_cursor.rs) | Force-fire cursor swap when Ctrl-held over allies/own/empty |
| Create | [src/sim/combat/combat_force_fire_cell_tests.rs](../../src/sim/combat/) | New unit-test file for cell-target attacks |
| Modify | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) | `mod combat_force_fire_cell_tests;` |

## Interface Changes

- **`AttackTarget.target` type changes from `u64` → `TargetKind`.** Public field;
  consumers in `aircraft/attack_mission.rs:49`, `aircraft/mod.rs:302`,
  `combat_tests.rs:91` must update reads.
- **`AttackTarget::new(u64)` constructor stays** for entity targets (most callers
  don't change). Add **`AttackTarget::for_cell(u16, u16)`**.
- **`Command::ForceAttackCell { attacker_id, target_rx, target_ry }`** appended
  to `Command` enum (variant order preserved for serde back-compat among existing
  variants; new variant is last).
- **`is_alt_held(state) -> bool`** added to `app_input.rs` mirror of `is_ctrl_held`.

## Sim Checklist

- [x] All math uses `fixed`-point — cell-center leptons computed as `i32 * 256 + 128`,
      converted to `SimFixed` for projectile spawn (existing `target_coords` pattern).
- [x] New state included in deterministic state hash — `TargetKind` is part of
      `AttackTarget` which is part of `GameEntity`, already serialized into hash.
- [x] No dependencies on render/ui/sidebar/audio/net introduced.
- [x] No tick ordering changes — `apply_command` runs in the same phase as today.
- [x] BTreeMap iteration order preserved — `keys_sorted()` iteration in combat tick
      unchanged.

## Risk Areas

- **Snapshot back-compat (medium):** `AttackTarget` shape change breaks load of
  pre-bump snapshots. Version bump and explicit error are mandatory; existing
  snapshots in test fixtures may need regeneration.
- **Aircraft Cell-target handling (high):** aircraft attack-mission state
  machine is deeply entity-coupled — 9 separate target reads across states
  0/3/4/5/6/7/8/9/10, plus `AttackTickResult.fire_at: Option<u64>`. Task 5
  is split into 5.1–5.5 to extract a `aircraft_target_status` helper, widen
  the fire signal, and thread both through every state. Risk that a state
  is missed during refactor — `cargo check` after Task 5.3 must surface zero
  remaining `target.position` / `entities.get(tid)` calls in attack_mission.rs.
  Task 5 + Task 15 + Task 16 verification cover this.
- **Auto-acquire entanglement (low):** passive target acquisition
  ([combat_targeting.rs:329](../../src/sim/combat/combat_targeting.rs#L329))
  always builds `Entity` variants. Confirm it never produces `Cell` variants
  (verified — it only fires for hostile-entity scans).
- **Per-unit voice cue (low):** ensure only one cue plays per multi-unit
  `ForceAttackCell` batch; matches existing `attack_voice` flag at
  [app_context_order.rs:568](../../src/app_context_order.rs#L568).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| Task 1 | `TargetKind::Cell(rx, ry)` carries cell coord directly, not via synthetic entity | Matches gamemd model — coord pair is the target, not a phantom object | Inspection vs Ghidra `0x004AE750` |
| Task 3 | `target_coords` returns cell-center leptons `(rx*256+128, ry*256+128)` for `Cell` variant | Projectile must spawn at the cell center, not the NW corner | Compute matches gamemd cell-center convention (RA2 standard) |
| Task 5.1–5.5 | Aircraft fly toward cell-center coords on `Cell` target via `aircraft_target_status` helper threaded through state machine + widened `fire_at: Option<TargetKind>` | Harrier ctrl-click cell must work; aircraft are score-5 (best-unit cursor source) — every mixed selection with a Harrier hits this | In-game test (Task 16) + aircraft regression test (Task 15) |
| Task 7 | `Command::ForceAttackCell` is a NEW variant, not a flag on `Attack` | Action 1 force-fire-on-cell must be a distinct dispatch path; per-unit dispatch in `0x004AE750` | Replay-determinism test (Task 14) |
| Task 10 | `force_fire = is_ctrl_held && !is_alt_held` | Alt+Ctrl = attack-move, NOT force-fire — every Alt+Ctrl press is observable | Truth-table test (Task 13); Ghidra `0x00700600` |
| Task 10 | Skip `group_destinations` distribution for `ForceAttackCell` | All selected units share the same cell coord; gamemd has NO group spread | Multi-unit test (Task 14); Ghidra `0x004AE750` |
| Task 10 | Unarmed unit (Engineer/Harvester/MCV/`+0xE13`) + Ctrl + cell click → emit `Move`, not `ForceAttackCell` | Common in mixed selections; gamemd falls through every time | Mixed-selection test (Task 14) |
| Task 10 | Shrouded-cell click suppresses command (no order issued) | Player Ctrl-clicks into shroud while scouting; gamemd does nothing | Shroud test (Task 14) |
| Task 11 | Cursor swaps to attack-cursor live as Ctrl is held over allies/own/empty | Visible every Ctrl-hold — the player needs feedback that force-fire is armed | Manual visual verification (Task 16) |
| Task 16 | One voice cue per multi-unit force-fire batch | gamemd's `g_SelectionVoice_Enable = 0` during dispatch — multiple cues would be a regression vs current Attack | Aural verification (Task 16) |

---

## Tasks

### Task 1: Define `TargetKind` enum

**Why:** Foundation type for all later work. Cell-target representation has to exist
before any consumer can reference it.

**Files:**
- Modify: [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) (around line 137,
  immediately above the existing `AttackTarget` struct).

**Pattern:** Plain enum with serde derives, mirrors the project convention from
e.g. `MovementLayer` and other small kind-enums in sim/.

**Step 1: Define type**

Insert above the existing `pub struct AttackTarget` (currently at line 138):

```rust
/// What an `AttackTarget` is pointing at — an entity or a ground cell.
///
/// Force-fire on empty terrain (`Ctrl + click cell`) sets the `Cell` variant.
/// Auto-acquired and explicit attack-on-unit orders set `Entity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TargetKind {
    /// Entity-targeted attack (normal Attack / ForceAttack on a unit/building).
    Entity(u64),
    /// Ground-targeted attack (force-fire on a cell). Cell coord in map space.
    Cell(u16, u16),
}
```

**Step 2: Verify compile**

Run: `cargo check -p ra2-rust-game`
Expected: PASS — type is defined but not yet referenced anywhere.

**Step 3: Commit**

`feat(combat): introduce TargetKind enum for entity vs cell targeting`

---

### Task 2: Replace `AttackTarget.target: u64` with `TargetKind`

**Why:** The struct is the single source of truth for "what's this unit firing at."
Widening the field is the smallest change that lets cell-targets flow through the
existing combat tick.

**Files:**
- Modify: [src/sim/combat/mod.rs:140](../../src/sim/combat/mod.rs#L140)

**Pattern:** Direct field-type replacement, with constructor pair
(`new` for entity, `for_cell` for cell).

**Step 1: Modify struct field**

Change [src/sim/combat/mod.rs:138-147](../../src/sim/combat/mod.rs#L138):

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttackTarget {
    /// What this attacker is firing at: an entity or a ground cell (force-fire).
    pub target: TargetKind,
    /// Simulation ticks remaining before the next shot (ROF cooldown).
    pub cooldown_ticks: u16,
    /// Shots remaining in the current burst. When this reaches 0, ROF cooldown starts.
    pub burst_remaining: u8,
    /// Ticks between individual burst shots (short inter-shot delay).
    pub burst_delay_ticks: u8,
}
```

**Step 2: Update constructor + add `for_cell`**

Replace [src/sim/combat/mod.rs:153-163](../../src/sim/combat/mod.rs#L153) with:

```rust
impl AttackTarget {
    /// Entity-targeted attack: fire at a specific entity by stable ID.
    pub fn new(target_stable_id: u64) -> Self {
        Self {
            target: TargetKind::Entity(target_stable_id),
            cooldown_ticks: 0,
            burst_remaining: 0,
            burst_delay_ticks: 0,
        }
    }

    /// Ground-targeted attack: fire at a specific cell coord (force-fire on terrain).
    pub fn for_cell(rx: u16, ry: u16) -> Self {
        Self {
            target: TargetKind::Cell(rx, ry),
            cooldown_ticks: 0,
            burst_remaining: 0,
            burst_delay_ticks: 0,
        }
    }
}
```

**Step 3: Verify compile breakage scope**

Run: `cargo check -p ra2-rust-game 2>&1 | head -60`
Expected: Compile errors at the consumer sites. Confirm the failing locations
are exactly:
- `src/sim/aircraft/attack_mission.rs:49`
- `src/sim/aircraft/mod.rs:302`
- `src/sim/combat/combat_tests.rs:91`
- (any other unexpected sites — investigate before continuing)

If new failure sites appear that aren't listed above, STOP and re-scope.

**Step 4: Commit (intermediate; compile broken — that's expected, fixed in tasks 3–5)**

Use a `wip:` prefix to make the broken-compile state explicit:

`wip(combat): widen AttackTarget.target to TargetKind enum`

---

### Task 3: Update `target_coords` to handle `Cell` variant

**Why:** `target_coords` is the single function that resolves a target's
(rx, ry, sub_x, sub_y) for projectile spawn and facing. Teaching it to handle
`Cell` keeps the rest of the combat pipeline ignorant of the difference.

**Files:**
- Modify: [src/sim/combat/mod.rs:175-204](../../src/sim/combat/mod.rs#L175)

**Pattern:** Existing function takes `entity: &GameEntity` and computes
foundation-center coords for buildings. Extend to compute cell-center coords
when the caller passes a `Cell` target instead of an entity.

**Step 1: Add helper for cell-center coords**

Insert immediately after `target_coords` (around line 205):

```rust
/// Compute lepton-precise coordinates for a cell target (force-fire on terrain).
///
/// Cell-center convention: leptons = `cell_index * 256 + 128`. Returns the
/// shape `target_coords` returns for entities (rx, ry, sub_x, sub_y) so callers
/// can branch on `TargetKind` and feed the result into the same pipeline.
fn cell_center_coords(rx: u16, ry: u16) -> (u16, u16, SimFixed, SimFixed) {
    (rx, ry, SimFixed::from_num(128), SimFixed::from_num(128))
}
```

**Step 2: Add a `TargetKind`-aware coordinate resolver**

Insert after `cell_center_coords`:

```rust
/// Resolve target coords from a `TargetKind`, looking up entity position when
/// needed and using cell-center for `Cell` targets.
///
/// Returns `None` if the target is `Entity(id)` and the entity no longer exists
/// (despawned / destroyed). `Cell` targets always resolve.
fn resolve_target_coords(
    target: &TargetKind,
    entities: &EntityStore,
    rules: Option<&RuleSet>,
    interner: &StringInterner,
) -> Option<(u16, u16, SimFixed, SimFixed)> {
    match *target {
        TargetKind::Entity(id) => entities.get(id).map(|t| target_coords(t, rules, interner)),
        TargetKind::Cell(rx, ry) => Some(cell_center_coords(rx, ry)),
    }
}
```

**Step 3: Verify compile**

Run: `cargo check -p ra2-rust-game 2>&1 | head -30`
Expected: same compile errors as after Task 2; no new errors introduced.

**Step 4: Commit**

`feat(combat): resolve_target_coords supports Cell targets`

---

### Task 4: Add `issue_attack_cell_command`

**Why:** Sim-side entry point for `Command::ForceAttackCell`. Mirrors the
existing `issue_attack_command` shape so dispatch in `world_commands.rs` follows
the existing pattern.

**Files:**
- Modify: [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) (after
  `issue_attack_command`, around line 268).

**Pattern:** Mirror of `issue_attack_command` at lines 210-267 — facing update,
clear movement target, set `attack_target`. The cell-target case skips the
"target entity exists" check (cells always exist) and uses `cell_center_coords`
instead of `target_coords`.

**Step 1: Add function**

Insert after the existing `issue_attack_command` closing brace (around line 268):

```rust
/// Issue a force-fire-on-cell command: make `attacker` fire at a ground cell.
///
/// Used by `Command::ForceAttackCell` (Ctrl + left-click on empty terrain).
/// Aborts (returns `false`) if the attacker has no weapon — caller is expected
/// to filter unarmed units client-side, but this defensive check keeps a stray
/// command from corrupting state.
pub fn issue_attack_cell_command(
    entities: &mut EntityStore,
    attacker_id: u64,
    target_rx: u16,
    target_ry: u16,
    rules: Option<&RuleSet>,
    interner: &StringInterner,
) -> bool {
    // Read attacker position and weapon presence before mutable borrow.
    let attacker_info = entities.get(attacker_id).map(|a| {
        let type_str = interner.resolve(a.type_ref);
        let has_weapon = rules
            .and_then(|r| r.object(type_str))
            .is_some_and(|obj| obj.primary.is_some() || obj.secondary.is_some());
        (
            a.position.rx,
            a.position.ry,
            a.position.sub_x,
            a.position.sub_y,
            a.turret_facing.is_some(),
            has_weapon,
        )
    });
    let (arx, ary, asx, asy, has_turret, has_weapon) = match attacker_info {
        Some(info) => info,
        None => return false,
    };

    if !has_weapon {
        // Defensive: client-side filter should have routed this to Move.
        // Log once per attacker rather than silently dropping.
        log::warn!(
            "ForceAttackCell rejected for unarmed attacker {} (target cell {},{})",
            attacker_id, target_rx, target_ry
        );
        return false;
    }

    let (trx, try_, tsx, tsy) = cell_center_coords(target_rx, target_ry);

    let attacker = match entities.get_mut(attacker_id) {
        Some(a) => a,
        None => return false,
    };

    if has_turret {
        let desired_u16 = crate::sim::movement::turret::facing_toward_lepton(
            arx, ary, asx, asy, trx, try_, tsx, tsy,
        );
        attacker.turret_facing = Some(desired_u16);
    } else {
        let dx: i32 = trx as i32 - arx as i32;
        let dy: i32 = try_ as i32 - ary as i32;
        attacker.facing = crate::sim::movement::facing_from_delta(dx, dy);
    }

    attacker.movement_target = None;
    attacker.attack_target = Some(AttackTarget::for_cell(target_rx, target_ry));
    true
}
```

**Step 2: Verify compile**

Run: `cargo check -p ra2-rust-game 2>&1 | head -30`
Expected: same compile errors as before (consumer sites still failing).

**Step 3: Commit**

`feat(combat): add issue_attack_cell_command for ground-targeted force-fire`

---

### Task 5: Aircraft Cell-target support (split into 5.1–5.5)

**Why:** The aircraft attack-mission state machine
([src/sim/aircraft/attack_mission.rs:35-320](../../src/sim/aircraft/attack_mission.rs#L35))
re-reads target as an entity in 9 places across states 0/3/4/5/6/7/8/9/10
(`entities.get(tid)` calls + `target.dying` / `target.health.current` /
`target.position.rx` reads). The fire signal `AttackTickResult.fire_at` is
also `Option<u64>` ([src/sim/aircraft/mod.rs:176](../../src/sim/aircraft/mod.rs#L176)),
so the projectile-spawn pipeline must be widened too. This is **larger than
the standard 2-5 min task** — acknowledged. Split into substeps below.

**Why bother (vs. defer aircraft):** Per the parity bar — aircraft are armed
mobile (score 5, the priority class for `SelectBestObjectForAction`). Mixed
selections like [Harrier + Grizzly] Ctrl-clicking a cell are common; if the
Harrier no-ops while Grizzly fires, that's a visible parity hole every match.
Aircraft Cell support is required, not optional, for (b+) scope.

**Files:**
- Modify: [src/sim/aircraft/attack_mission.rs](../../src/sim/aircraft/attack_mission.rs)
- Modify: [src/sim/aircraft/mod.rs](../../src/sim/aircraft/mod.rs)
- Modify: [src/sim/combat/combat_tests.rs](../../src/sim/combat/combat_tests.rs)
- Modify: any callsite that reads `AttackTickResult.fire_at` and dispatches
  to projectile spawn (search `fire_at` in `src/sim/aircraft/`).

**Pattern:** Extract a small abstraction over Entity vs Cell target lookup,
thread it through every state. Widen the fire signal to carry `TargetKind`.

---

#### Task 5.1: Add `aircraft_target_status` helper

**Files:**
- Modify: [src/sim/aircraft/attack_mission.rs](../../src/sim/aircraft/attack_mission.rs)
  (add at top of file, below the `const` declarations).

**Step 1: Add helper struct + function**

```rust
use crate::sim::combat::{AttackTarget, TargetKind};

/// Resolved status of an aircraft's current attack target — abstracts over
/// Entity vs Cell so the state machine doesn't care which kind it is.
#[derive(Debug, Clone, Copy)]
struct AircraftTargetStatus {
    /// Target cell coord (entity position for Entity, cell coord for Cell).
    rx: u16,
    ry: u16,
    /// True if the target is engageable (entity alive, or always-true for cells).
    alive: bool,
    /// Entity ID for Entity targets; None for Cell targets. Used by the fire
    /// signal to know whether the projectile has an entity destination or
    /// just coords.
    entity_id: Option<u64>,
}

/// Look up the aircraft's current target status.
///
/// Returns `None` if `attack_target` is `None`. For Entity targets, returns
/// `None` if the entity has despawned. For Cell targets, always returns
/// `Some` (cells "always exist").
fn aircraft_target_status(
    at: Option<&AttackTarget>,
    entities: &EntityStore,
) -> Option<AircraftTargetStatus> {
    let at = at?;
    match at.target {
        TargetKind::Entity(id) => {
            let target = entities.get(id)?;
            Some(AircraftTargetStatus {
                rx: target.position.rx,
                ry: target.position.ry,
                alive: !target.dying && target.health.current > 0,
                entity_id: Some(id),
            })
        }
        TargetKind::Cell(rx, ry) => Some(AircraftTargetStatus {
            rx,
            ry,
            alive: true,
            entity_id: None,
        }),
    }
}
```

**Step 2: Verify compile**

Run: `cargo check -p ra2-rust-game 2>&1 | head -40`
Expected: same compile errors as after Task 4 (consumers still failing) — no
new errors from the helper.

**Step 3: Commit**

`feat(aircraft): add aircraft_target_status helper for entity/cell abstraction`

---

#### Task 5.2: Widen `AttackTickResult.fire_at` to `Option<TargetKind>`

**Why:** The fire signal must carry enough info for the projectile-spawn
pipeline to spawn either at an entity or at a cell coord.

**Files:**
- Modify: [src/sim/aircraft/mod.rs:176](../../src/sim/aircraft/mod.rs#L176)
  (the `AttackTickResult` struct definition).
- Modify: [src/sim/aircraft/attack_mission.rs](../../src/sim/aircraft/attack_mission.rs)
  (all `AttackTickResult::fire(mission, tid)` call sites — there are several).

**Step 1: Change struct field type**

Find the `AttackTickResult` struct (around aircraft/mod.rs:176) and change:

```rust
fire_at: Option<u64>,
```

to:

```rust
fire_at: Option<TargetKind>,
```

Add `use crate::sim::combat::TargetKind;` at top if not already imported.

**Step 2: Change `AttackTickResult::fire` signature**

Find the `fn fire(...)` constructor for `AttackTickResult` (search for
`fn fire(` in aircraft/mod.rs and attack_mission.rs). Change parameter
from `tid: u64` to `tk: TargetKind`. Update the body's `fire_at: Some(tid)`
to `fire_at: Some(tk)`.

**Step 3: Verify compile**

Run: `cargo check -p ra2-rust-game 2>&1 | head -40`
Expected: many new errors at `AttackTickResult::fire(mission, tid)` call
sites (states 4, 5, 6/7/8, 9 in attack_mission.rs). Confirm those are the
only new errors — nothing else.

**Step 4: Commit (intermediate; compile broken)**

`wip(aircraft): widen AttackTickResult.fire_at to TargetKind`

---

#### Task 5.3: Refactor attack_mission states to use the helper + new fire signal

**Files:**
- Modify: [src/sim/aircraft/attack_mission.rs](../../src/sim/aircraft/attack_mission.rs)
  (states 0, 3, 4, 5, 6/7/8, 9, 10 — every state in the `match sub_state`).

**Step 1: Replace `target_id` extract at line 49**

Replace:

```rust
let target_id = entity.attack_target.as_ref().map(|at| at.target);
```

with:

```rust
let target_status = aircraft_target_status(entity.attack_target.as_ref(), entities);
```

**Step 2: Replace all per-state target reads**

Per state, rewrite each of these patterns:

| Old pattern | New pattern |
|---|---|
| `if target_id.is_none() \|\| target_id.and_then(\|tid\| entities.get(tid)).is_none() {` (state 0) | `if target_status.map_or(true, \|s\| !s.alive) {` |
| `let Some(tid) = target_id else { ... };` followed by `let Some(target) = entities.get(tid) else { ... };` (states 3, 4, 6/7/8) | `let Some(status) = target_status else { ... }; if !status.alive { return RTB-transition; }` |
| `target.position.rx` / `target.position.ry` reads | `status.rx` / `status.ry` |
| `target.dying \|\| target.health.current == 0` | `!status.alive` |
| `AttackTickResult::fire(mission, tid)` (states 4, 5, 6/7/8, 9) | `AttackTickResult::fire(mission, status.entity_id.map_or(TargetKind::Cell(status.rx, status.ry), TargetKind::Entity))` |
| State 10's re-engage check (lines 305-308) | `let can_reengage = ammo_current + result_ammo_delta > 0 && target_status.is_some_and(\|s\| s.alive);` |

(Read each state in full first; the existing variable names like `tid` and
`target` are referenced multiple lines after the initial extract. Replace
all of them in lockstep.)

**Step 3: Verify compile + tests**

Run: `cargo check -p ra2-rust-game`
Expected: PASS.

Run: `cargo test -p ra2-rust-game --lib aircraft`
Expected: existing aircraft tests still pass (no regressions on entity-target
behavior).

**Step 4: Commit**

`feat(aircraft): thread TargetKind through attack_mission state machine`

---

#### Task 5.4: Update aircraft/mod.rs:302 distance-tier branch

**Files:**
- Modify: [src/sim/aircraft/mod.rs:300-323](../../src/sim/aircraft/mod.rs#L300)
  (the speed-fraction-by-distance branch in the Attack mission tick).

**Step 1: Replace target lookup**

Replace lines 301-323 (the `if let Some(tid) = entity.attack_target.as_ref()
.map(|at| at.target) { … }` block) with:

```rust
if let Some(status) = aircraft_target_status(entity.attack_target.as_ref(), &sim.entities) {
    let dx = (entity.position.rx as i32 - status.rx as i32).abs();
    let dy = (entity.position.ry as i32 - status.ry as i32).abs();
    let dist_cells = dx.max(dy);

    let speed_frac = if dist_cells < 1 {
        SIM_ZERO
    } else if dist_cells < 2 {
        SimFixed::lit("0.5")
    } else if dist_cells < 3 {
        SimFixed::lit("0.75")
    } else {
        SIM_ONE
    };
    m.set_speed_fraction = Some(speed_frac);
}
```

(Move `aircraft_target_status` to a `pub(crate)` visibility in
attack_mission.rs if the cross-module call requires it. Add
`use crate::sim::aircraft::attack_mission::aircraft_target_status;` import.)

**Step 2: Verify**

Run: `cargo check -p ra2-rust-game`
Expected: PASS.

**Step 3: Commit**

`feat(aircraft): distance-tier branch supports cell targets via helper`

---

#### Task 5.5: Update combat_tests.rs:91 + downstream fire-pipeline consumers

**Files:**
- Modify: [src/sim/combat/combat_tests.rs:91-93](../../src/sim/combat/combat_tests.rs#L91)
- Modify: any code that consumes `AttackTickResult.fire_at` to spawn
  projectiles (grep `fire_at` across aircraft/ and combat/ to find all sites).

**Step 1: combat_tests.rs**

Replace [combat_tests.rs:91-93](../../src/sim/combat/combat_tests.rs#L91):

```rust
let attack = store.get(1).unwrap().attack_target.as_ref().unwrap();
assert_eq!(attack.target, 2);
assert_eq!(attack.cooldown_ticks, 0, "Initial cooldown should be 0");
```

with:

```rust
let attack = store.get(1).unwrap().attack_target.as_ref().unwrap();
assert!(matches!(attack.target, TargetKind::Entity(2)));
assert_eq!(attack.cooldown_ticks, 0, "Initial cooldown should be 0");
```

Add `use crate::sim::combat::TargetKind;` to the test module's imports.

**Step 2: Update fire-pipeline consumers**

Run `grep -n 'fire_at' src/sim/aircraft/ src/sim/combat/` to find every
consumer of the widened `fire_at: Option<TargetKind>` field. For each:
- If it reads `fire_at` to look up an entity, branch on `TargetKind::Entity(id)
  → entity lookup` vs `TargetKind::Cell(rx, ry) → cell-center coords`.
- Projectile spawn that needs target coords: use cell-center directly for
  Cell, or look up entity position for Entity (existing path).

**Step 3: Verify**

Run: `cargo test -p ra2-rust-game --lib`
Expected: PASS — all existing tests unchanged for entity targets; aircraft
Cell-target firing now works.

**Step 4: Commit**

`feat(aircraft): wire TargetKind through fire pipeline + update tests`

---

### Task 6: Bump `SNAPSHOT_VERSION`

**Why:** `AttackTarget` field-shape change is a breaking serialization change.
Pre-bump snapshots must fail to load with a clear error.

**Files:**
- Modify: [src/sim/snapshot.rs:16](../../src/sim/snapshot.rs#L16)

**Step 1: Bump version**

Change [src/sim/snapshot.rs:16](../../src/sim/snapshot.rs#L16):

```rust
const SNAPSHOT_VERSION: u32 = 5;
```

**Step 2: Verify**

Run: `cargo test -p ra2-rust-game --lib snapshot`
Expected: PASS. Any tests that had hardcoded version 4 must already be using
the constant — if not, they'll fail and need to reference `SNAPSHOT_VERSION`.

**Step 3: Commit**

`chore(snapshot): bump SNAPSHOT_VERSION 4 -> 5 (AttackTarget field shape)`

---

### Task 7: Append `Command::ForceAttackCell` variant

**Why:** Sim-side entry point for the new command. Variant goes at the **end**
of the enum so existing variant ordinal indices stay stable across this change
(serde back-compat for existing replays/snapshots).

**Files:**
- Modify: [src/sim/command.rs](../../src/sim/command.rs) (append before the
  closing `}` of the enum at line 144).

**Pattern:** Mirror of existing `Command::ForceAttack { attacker_id, target_id }`
at line 45.

**Step 1: Add variant**

Insert before the enum's closing brace (after the `ToggleInfantryDeploy`
variant at line 143):

```rust
    /// Force-attack on a ground cell (Ctrl + left-click on empty terrain).
    ///
    /// Bypasses friendship check and entity-targeting — fires the attacker's
    /// weapon at the cell's center. Unarmed units must NOT receive this
    /// command; the order-resolution layer routes them to `Move` instead.
    /// Defensive sim-side check in `issue_attack_cell_command` warn-logs and
    /// no-ops if a stray `ForceAttackCell` reaches an unarmed unit.
    ForceAttackCell {
        attacker_id: u64,
        target_rx: u16,
        target_ry: u16,
    },
```

**Step 2: Verify compile**

Run: `cargo check -p ra2-rust-game 2>&1 | head -30`
Expected: PASS — variant defined but not yet dispatched (added in Task 8).
Some `match Command { … }` exhaustiveness warnings are fine; resolved by
Task 8.

**Step 3: Commit**

`feat(command): append ForceAttackCell variant`

---

### Task 8: Add `ForceAttackCell` dispatch arm

**Why:** Plumbing — the Command must reach the sim's combat code. Mirror the
existing `Command::ForceAttack` arm.

**Files:**
- Modify: [src/sim/world/world_commands.rs:319-341](../../src/sim/world/world_commands.rs#L319)
  (insert a new match arm immediately after `Command::ForceAttack`).

**Pattern:** Mirror of `Command::ForceAttack` arm (existing).

**Step 1: Add match arm**

Insert immediately after the closing brace of the `Command::ForceAttack` arm
(currently at line 341, before `Command::AttackMove`):

```rust
            Command::ForceAttackCell {
                attacker_id,
                target_rx,
                target_ry,
            } => {
                if !self.entity_owned_by_id(command_owner, *attacker_id) {
                    return false;
                }
                // No target-entity existence check — cells always "exist".
                self.release_docked_idle(*attacker_id);
                if let Some(e) = self.entities.get_mut(*attacker_id) {
                    e.order_intent = None;
                    Self::clear_aircraft_dock_phase(e);
                }
                combat::issue_attack_cell_command(
                    &mut self.entities,
                    *attacker_id,
                    *target_rx,
                    *target_ry,
                    rules,
                    &self.interner,
                )
            }
```

**Step 2: Verify compile + tests**

Run: `cargo test -p ra2-rust-game --lib`
Expected: PASS — exhaustiveness satisfied; existing tests unchanged.

**Step 3: Commit**

`feat(world): dispatch ForceAttackCell to issue_attack_cell_command`

---

### Task 9: Add `is_alt_held` helper

**Why:** Foundation for the Alt+Ctrl=attack-move detection in Task 10.

**Files:**
- Modify: [src/app_input.rs](../../src/app_input.rs) (immediately below the
  existing `is_ctrl_held` and `is_shift_held` definitions around line 628).

**Pattern:** Direct mirror of [is_ctrl_held](../../src/app_input.rs#L628) — read
`AppState.keys_held: HashSet<KeyCode>` and check for `AltLeft | AltRight`.

**Step 1: Add helper**

Insert immediately below `is_ctrl_held`:

```rust
/// Return `true` if either Alt key is currently held.
///
/// Used in order resolution to detect Alt+Ctrl = attack-move (NOT force-fire).
/// Mirrors the gamemd modifier-poll for `DAT_00a8ec08 / DAT_00a8ec0c`.
pub(crate) fn is_alt_held(state: &AppState) -> bool {
    use winit::keyboard::KeyCode;
    state.keys_held.contains(&KeyCode::AltLeft) || state.keys_held.contains(&KeyCode::AltRight)
}
```

**Step 2: Verify compile**

Run: `cargo check -p ra2-rust-game`
Expected: PASS.

**Step 3: Commit**

`feat(input): add is_alt_held helper for Alt+Ctrl modifier detection`

---

### Task 10: Wire force-fire-cell into order resolution

**Why:** Single biggest behavior change in this plan. Routes Ctrl+click on
empty cell → `ForceAttackCell` (armed) or `Move` (unarmed); routes Alt+Ctrl →
attack-move; rejects shrouded-cell clicks; suppresses group-destination spread
for the new command. This is where parity is won or lost.

**Files:**
- Modify: [src/app_context_order.rs](../../src/app_context_order.rs) — major
  changes around lines 43, 113, 428-490, 519-558.

**Pattern:** Existing `force_fire` flag pattern at line 43. Existing per-unit
command emission loop at line 501. Existing `clicked_ore` suppression at
line 113.

**Step 1: Tighten `force_fire` to exclude Alt+Ctrl**

Change [src/app_context_order.rs:43](../../src/app_context_order.rs#L43):

```rust
let force_fire: bool = is_ctrl_held(state) && !is_alt_held(state);
```

(Imports: add `is_alt_held` to the existing import line at line 19.)

**Step 2: Add cell-shroud rejection helper**

Just before the per-unit emission loop (around line 500, before the line that
reads `let attack_target: Option<u64> = if force_fire { ... }`), add:

```rust
// Force-fire on a shrouded cell is rejected — gamemd can't target what it
// can't see. Computed once outside the per-unit loop.
let cell_is_shrouded: bool = if force_fire && !state.sandbox_full_visibility {
    let owner_id_for_fog = sim.interner.get(&owner).unwrap_or_default();
    !sim.fog.is_cell_revealed(owner_id_for_fog, target_rx, target_ry)
        || sim.fog.is_cell_gap_covered(owner_id_for_fog, target_rx, target_ry)
} else {
    false
};
```

**Step 3: Branch the per-unit emission loop on armed-vs-unarmed**

In the loop starting at [line 501](../../src/app_context_order.rs#L501), modify
the `payload = if let Some(target_id) = attack_target { ... } else { ... }`
branch to handle the force-fire-no-entity-hit case explicitly. Replace the
entire `if let Some(target_id) = attack_target { ... }` else-branch with:

```rust
            let payload = if let Some(target_id) = attack_target {
                if force_fire {
                    Command::ForceAttack {
                        attacker_id: stable_id,
                        target_id,
                    }
                } else if order_mode != OrderMode::Guard {
                    Command::Attack {
                        attacker_id: stable_id,
                        target_id,
                    }
                } else {
                    Command::Guard {
                        entity_id: stable_id,
                        target_id: Some(target_id),
                    }
                }
            } else if force_fire && !cell_is_shrouded {
                // Force-fire on empty terrain: per-unit dispatch.
                // Armed units → ForceAttackCell. Unarmed → Move (gamemd
                // What_Action_OnCell falls unarmed units through to action 2).
                let unit_armed = sim
                    .entities
                    .get(stable_id)
                    .and_then(|e| {
                        let type_str = sim.interner.resolve(e.type_ref);
                        state
                            .rules
                            .as_ref()
                            .and_then(|r| r.object(type_str))
                            .map(|obj| obj.primary.is_some() || obj.secondary.is_some())
                    })
                    .unwrap_or(false);

                // Harvesters explicitly excluded from direct attack orders
                // (gamemd TechnoTypeClass +0xE13).
                let is_harvester = sim
                    .entities
                    .get(stable_id)
                    .is_some_and(|e| e.miner.is_some());

                if unit_armed && !is_harvester {
                    Command::ForceAttackCell {
                        attacker_id: stable_id,
                        target_rx,
                        target_ry,
                    }
                } else {
                    // Fall through to plain Move. Reuse the SAME walkability
                    // fallback the regular Move path uses (lines 525-540) — if
                    // the cell is not walkable, route to nearest walkable cell.
                    // Otherwise an Engineer Ctrl-clicking water silently stalls.
                    let goal: (u16, u16) = {
                        let mut g = (target_rx, target_ry);
                        if let Some(grid) = state.path_grid.as_ref() {
                            if !crate::app_sim_tick::is_any_layer_walkable(grid, g.0, g.1) {
                                if let Some(nearest) =
                                    crate::app_sim_tick::nearest_walkable_cell_layered(grid, g, 12)
                                {
                                    g = nearest;
                                }
                            }
                        }
                        g
                    };
                    Command::Move {
                        entity_id: stable_id,
                        target_rx: goal.0,
                        target_ry: goal.1,
                        queue: queue_mode,
                        group_id: None, // No group spread for force-fire fall-through.
                    }
                }
            } else {
                match order_mode {
                    // ... unchanged from existing match (lines 520-563) ...
                }
            };
```

(Preserve the existing `match order_mode { ... }` body at lines 520-563
unchanged in the final `else` branch.)

**Step 4: Suppress voice spam — only one cue per batch**

The existing `attack_voice = attack_target.is_some();` at
[line 568](../../src/app_context_order.rs#L568) covers the entity-target case.
Extend it to the cell-target case:

```rust
attack_voice = attack_target.is_some() || (force_fire && !cell_is_shrouded);
```

**Step 5: Verify compile**

Run: `cargo check -p ra2-rust-game`
Expected: PASS.

**Step 6: Run existing app-layer tests**

Run: `cargo test -p ra2-rust-game --lib app_context_order`
Expected: all pass; if any fail, they're testing old force-fire behavior and
need updates in Task 13.

**Step 7: Commit**

`feat(input): wire ForceAttackCell + Alt+Ctrl override + unarmed fall-through`

---

### Task 11: Force-fire cursor swap

**Why:** Live cursor feedback is one of the parity-critical visible details —
the player needs to see "force-fire is armed" the moment Ctrl is held, not just
on click.

**Files:**
- Modify: [src/app_cursor.rs](../../src/app_cursor.rs) — `current_cursor_feedback_kind`
  at line 17.

**Pattern:** Existing cursor-decision cascade. The function ALREADY rejects
shrouded cells at lines 69-85 via `cell_visibility_for_local_owner` — the
force-fire override goes AFTER that check, not before, so over-shroud Ctrl-hold
falls through to the existing shroud-rejection path (queued_order_mode cursor)
rather than incorrectly showing Attack.

**Step 1: Add force-fire override branch (correct placement)**

Insert the new branch between the existing shroud-rejection block (ending at
line 85) and the hover-target lookup (starting at line 86). At this point:
- We know the selection is non-empty (line 60 short-circuit).
- We know the cell IS visible (lines 69-85 returned for non-Visible cells).
- We have `world_x, world_y, hover_rx, hover_ry, owner, owner_id` already
  computed at lines 63-68.

Add immediately after line 85's closing brace:

```rust
// Force-fire override: when Ctrl is held (and Alt isn't), show attack cursor
// over allies, own units, and empty cells. Only fires if the selection has at
// least one armed unit (gamemd SelectBestObjectForAction priority: armed
// mobile = 5 wins the cursor source). Placed AFTER the shroud check above
// so over-shroud Ctrl-hold falls through to the queued_order_mode cursor
// already chosen at lines 79-84 — that branch already returned by here.
if crate::app_input::is_ctrl_held(state) && !crate::app_input::is_alt_held(state) {
    let selection_has_armed_unit = sim
        .entities
        .values()
        .filter(|e| e.selected)
        .any(|e| {
            let type_str = sim.interner.resolve(e.type_ref);
            state
                .rules
                .as_ref()
                .and_then(|r| r.object(type_str))
                .is_some_and(|obj| obj.primary.is_some() || obj.secondary.is_some())
        });
    if selection_has_armed_unit {
        return Some(CursorFeedbackKind::Attack);
    }
}
```

(No invented `cell_under_cursor_is_shrouded` helper — the existing shroud
check at lines 69-85 already handled that case before we got here.)

**Step 2: Verify compile**

Run: `cargo check -p ra2-rust-game`
Expected: PASS.

**Step 3: Commit**

`feat(cursor): force-fire cursor swap when Ctrl held over allies/own/empty`

---

### Task 12: Unit tests — `issue_attack_cell_command`

**Why:** Verify the sim-side entry point sets `attack_target = Some(Cell(rx,ry))`,
clears movement, updates facing, and rejects unarmed attackers.

**Files:**
- Create: [src/sim/combat/combat_force_fire_cell_tests.rs](../../src/sim/combat/)
- Modify: [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) — add
  `#[cfg(test)] mod combat_force_fire_cell_tests;` near the existing test mod
  declarations (search for `mod combat_tests;`).

**Pattern:** Mirror of existing tests in `combat_tests.rs`.

**Step 1: Create test file**

Use the existing `make_entity(stable_id, type_str, rx, ry, hp)` helper and
`test_interner()` from `combat_tests.rs`. Re-read
[src/sim/combat/combat_tests.rs:82-103](../../src/sim/combat/combat_tests.rs#L82)
first for the exact pattern (`test_issue_attack_command` is the closest
template). Note that `combat_tests.rs:88` calls `issue_attack_command` with
`None` for rules — but our `issue_attack_cell_command` reads `obj.primary` /
`obj.secondary` to check for a weapon, so we MUST construct a minimal
`RuleSet` with the relevant `[GRIZZLY]` and `[ENGINEER]` entries (or use an
existing helper if one exists — grep `RuleSet::default` and
`fn test_ruleset` in `src/` to find one).

```rust
//! Force-fire-on-cell unit tests for `issue_attack_cell_command`.

use super::{AttackTarget, TargetKind, issue_attack_cell_command};
use crate::sim::combat::combat_tests::{make_entity, test_interner};
use crate::sim::entity_store::EntityStore;
// (Adapt RuleSet import once you've located the test helper. If none exists,
// build a minimal RuleSet inline — see Step 1.5 below.)

#[test]
fn issue_attack_cell_sets_cell_target_for_armed_unit() {
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "GRIZZLY", 5, 5, 300));
    let interner = test_interner();
    let rules = test_ruleset_with_armed_grizzly();  // see Step 1.5

    let ok = issue_attack_cell_command(
        &mut store,
        1,         // attacker_id
        50, 50,    // target cell
        Some(&rules),
        &interner,
    );

    assert!(ok, "issue_attack_cell_command should succeed for armed unit");
    let attack = store.get(1).unwrap().attack_target.as_ref().unwrap();
    assert!(matches!(attack.target, TargetKind::Cell(50, 50)));
    assert_eq!(attack.cooldown_ticks, 0);
    assert_eq!(attack.burst_remaining, 0);
}

#[test]
fn issue_attack_cell_clears_movement_target() {
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "GRIZZLY", 5, 5, 300));
    let interner = test_interner();
    let rules = test_ruleset_with_armed_grizzly();
    // Pre-set a movement target. Inspect `MovementTarget` in
    // src/sim/components/ for the exact constructor — likely
    // `MovementTarget::new(50, 50)` or similar.
    store.get_mut(1).unwrap().movement_target = Some(/* MovementTarget for (50, 50) */);

    let ok = issue_attack_cell_command(&mut store, 1, 50, 50, Some(&rules), &interner);
    assert!(ok);
    assert!(store.get(1).unwrap().movement_target.is_none());
}

#[test]
fn issue_attack_cell_rejects_unarmed_attacker() {
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "ENGINEER", 5, 5, 75));
    let interner = test_interner();
    let rules = test_ruleset_with_unarmed_engineer();  // see Step 1.5

    let ok = issue_attack_cell_command(&mut store, 1, 50, 50, Some(&rules), &interner);

    assert!(!ok, "ForceAttackCell on unarmed unit must return false");
    assert!(store.get(1).unwrap().attack_target.is_none());
}

// (Add a test for facing update — model it on whatever facing assertion
// `test_issue_attack_command` style tests use in combat_tests.rs.)
```

**Step 1.5: Construct minimal `RuleSet` helpers**

In the same test file, add:

```rust
fn test_ruleset_with_armed_grizzly() -> crate::rules::ruleset::RuleSet {
    // Minimal RuleSet with one ObjectType entry:
    //   [GRIZZLY] Primary=120mm  +  a [120mm] weapon entry
    // Inspect src/rules/ruleset.rs for the construction pattern. If a builder
    // pattern exists, use it; otherwise construct the structs directly.
    todo!("inline build — see src/rules/ruleset.rs for shape")
}

fn test_ruleset_with_unarmed_engineer() -> crate::rules::ruleset::RuleSet {
    // [ENGINEER] (no Primary, no Secondary) — Engineer is the canonical
    // unarmed unit.
    todo!("inline build — see src/rules/ruleset.rs for shape")
}
```

Replace the `todo!()` bodies with the actual construction once you've read
[src/rules/ruleset.rs](../../src/rules/ruleset.rs). If the construction is
non-trivial (more than ~15 lines), the cleanest path may be a separate
test-fixture module; check whether `src/rules/` already has a `test_helpers`
module before inventing one.

**Step 2: Wire the module**

In [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs), find the existing test
module declarations (likely at the bottom of the file: `#[cfg(test)] mod
combat_tests;`) and add:

```rust
#[cfg(test)]
mod combat_force_fire_cell_tests;
```

**Step 3: Verify**

Run: `cargo test -p ra2-rust-game --lib combat_force_fire_cell`
Expected: all tests pass.

**Step 4: Commit**

`test(combat): unit tests for issue_attack_cell_command`

---

### Task 13: Order-resolution unit tests

**Why:** The Alt+Ctrl truth table and the unarmed fall-through are the most
likely sites for parity drift; these tests pin the behavior.

**Files:**
- Modify or create test mod for [src/app_context_order.rs](../../src/app_context_order.rs)
  — search for an existing `#[cfg(test)] mod` in the file. If none, add one
  at the bottom.

**Step 1: Add tests**

```rust
#[cfg(test)]
mod force_fire_tests {
    // (Adapt to actual existing test infrastructure. The order-resolution
    // function takes an AppState — these tests will need a minimal AppState
    // fixture. If no fixture exists, the cleanest path is integration-level
    // tests in a `tests/` directory; document that decision and adjust.)

    #[test]
    fn force_fire_truth_table() {
        // ctrl_only        → force_fire = true
        // alt_only         → force_fire = false
        // ctrl_and_alt     → force_fire = false (Alt overrides Ctrl)
        // shift_only       → force_fire = false
        // ctrl_and_shift   → force_fire = true (Shift orthogonal)
        // none             → force_fire = false
    }

    #[test]
    fn force_fire_on_cell_emits_force_attack_cell_for_armed() {
        // [Grizzly] selected, ctrl-click empty cell → 1× Command::ForceAttackCell.
    }

    #[test]
    fn force_fire_on_cell_falls_through_to_move_for_unarmed() {
        // [Engineer] selected, ctrl-click empty cell → 1× Command::Move.
    }

    #[test]
    fn force_fire_mixed_selection_splits_per_unit() {
        // [Grizzly + Engineer] ctrl-click empty cell:
        //   - Grizzly  → Command::ForceAttackCell { rx, ry }
        //   - Engineer → Command::Move { rx, ry }
        // Verify both commands target the SAME cell coord (no group spread).
    }

    #[test]
    fn force_fire_shrouded_cell_emits_no_command() {
        // [Grizzly] selected, ctrl-click cell that's shrouded for local player
        // → no commands queued, return value indicates click NOT consumed
        // (or matches the "shrouded fall-through" convention; check existing
        // shroud-rejection tests for the convention).
    }

    #[test]
    fn force_fire_skips_group_destinations_distribution() {
        // [4× Grizzly] ctrl-click empty cell → 4× ForceAttackCell, all with
        // identical (target_rx, target_ry). Verify NO radial spread applied.
    }
}
```

**Step 2: Verify**

Run: `cargo test -p ra2-rust-game --lib force_fire_tests`
Expected: all pass.

**Step 3: Commit**

`test(input): order-resolution truth table + per-unit dispatch + shroud rejection`

---

### Task 14: Replay determinism + snapshot round-trip

**Why:** New command variant + struct field shape change must survive
serialize → deserialize → re-hash unchanged. This is the single test that
proves we haven't broken lockstep correctness.

**Files:**
- Modify: existing replay/snapshot integration tests (search
  `tests/snapshot*.rs`, `tests/determinism*.rs`, or in
  [src/sim/world/world_tests.rs](../../src/sim/world/world_tests.rs) and
  [src/sim/snapshot.rs](../../src/sim/snapshot.rs) test mods).

**Step 1: Add snapshot round-trip test for cell-target attack**

```rust
#[test]
fn snapshot_round_trip_preserves_cell_attack_target() {
    let mut sim = build_test_simulation();
    spawn_grizzly_at(&mut sim, /* ... */);
    let attacker_id = /* ... */;

    // Apply a ForceAttackCell.
    sim.pending_commands.push(CommandEnvelope::new(
        owner_id,
        sim.tick,
        Command::ForceAttackCell { attacker_id, target_rx: 50, target_ry: 50 },
    ));
    sim.advance_tick(/* ... */);

    // Confirm cell target attached.
    let attack = sim.entities.get(attacker_id).unwrap().attack_target.as_ref().unwrap();
    assert!(matches!(attack.target, TargetKind::Cell(50, 50)));

    // Round-trip through snapshot.
    let snap = sim.snapshot().expect("snapshot");
    let bytes = bincode::serialize(&snap).expect("serialize");
    let restored: SimulationSnapshot = bincode::deserialize(&bytes).expect("deserialize");
    let mut sim2 = Simulation::from_snapshot(restored).expect("restore");

    let attack2 = sim2.entities.get(attacker_id).unwrap().attack_target.as_ref().unwrap();
    assert!(matches!(attack2.target, TargetKind::Cell(50, 50)));
    assert_eq!(sim.world_hash(), sim2.world_hash(),
        "state hash must be identical after round-trip");
}
```

(Adapt to the actual snapshot API. If `sim.snapshot()` / `from_snapshot()` /
`bincode::serialize` aren't the right calls, read `src/sim/snapshot.rs` for
the real API.)

**Step 2: Add replay determinism test for mixed-target session**

```rust
#[test]
fn replay_determinism_mixed_entity_and_cell_targets() {
    let mut sim_a = build_test_simulation();
    let mut sim_b = build_test_simulation();

    let commands = vec![
        Command::Attack { attacker_id: 1, target_id: 2 },
        Command::ForceAttack { attacker_id: 3, target_id: 4 },
        Command::ForceAttackCell { attacker_id: 5, target_rx: 50, target_ry: 50 },
    ];

    // Apply identical sequence to both sims, advance N ticks.
    for cmd in &commands {
        sim_a.pending_commands.push(CommandEnvelope::new(owner, sim_a.tick, cmd.clone()));
        sim_b.pending_commands.push(CommandEnvelope::new(owner, sim_b.tick, cmd.clone()));
    }
    for _ in 0..30 {
        sim_a.advance_tick(/* ... */);
        sim_b.advance_tick(/* ... */);
    }

    assert_eq!(sim_a.world_hash(), sim_b.world_hash(),
        "two sims with identical command sequence must hash-match");
}
```

**Step 3: Verify**

Run: `cargo test -p ra2-rust-game snapshot_round_trip_preserves_cell_attack_target`
Run: `cargo test -p ra2-rust-game replay_determinism_mixed_entity_and_cell_targets`
Expected: both pass.

**Step 4: Commit**

`test(replay): cell-target attack round-trips and replays deterministically`

---

### Task 15: Aircraft Cell-target verification (manual + assertion)

**Why:** Aircraft handle the `TargetKind` match in Task 5, but the actual
fly-toward-cell + fire behavior wasn't directly tested. Add a small test plus
a manual check.

**Files:**
- Modify: [src/sim/aircraft/attack_mission.rs](../../src/sim/aircraft/attack_mission.rs)
  test mod (existing tests at lines ~431, 454, 480, 505, 525).

**Step 1: Add a cell-target aircraft test**

Mirror one of the existing tests:

```rust
#[test]
fn harrier_with_cell_target_advances_toward_cell() {
    let mut entities = /* test fixture with a Harrier-type aircraft at (10, 10) */;
    let harrier_id = 1u64;
    entities.get_mut(harrier_id).unwrap().attack_target =
        Some(AttackTarget::for_cell(50, 50));

    // Tick aircraft mission.
    tick_aircraft_attack_mission(&mut entities, /* ... */);

    // Verify the aircraft is now heading toward cell (50, 50) — exact criterion
    // depends on the aircraft AI (movement_target set, facing updated, etc.).
    let h = entities.get(harrier_id).unwrap();
    // assert that movement target or facing is now toward (50, 50).
}
```

**Step 2: Verify**

Run: `cargo test -p ra2-rust-game aircraft attack_mission`
Expected: PASS, including the new test.

**Step 3: Commit**

`test(aircraft): Harrier with Cell target advances toward cell`

---

### Task 16: In-game manual verification

**Why:** Final parity check against gamemd.exe — confirms the implementation
matches what the player sees in the original game.

**Steps to verify in-game:**

1. **Cursor live-update**: Hold Ctrl with a Grizzly selected. Move cursor over
   an ally, an own unit, an empty cell. Cursor must show attack-cursor in all
   three cases. Release Ctrl — cursor must revert to normal in all three.
   Expected: matches gamemd.exe.

2. **Force-fire on empty cell**: Select 1 Grizzly, Ctrl+click an empty cell.
   Grizzly walks into range and fires shells at the cell. Splash damage hits
   anything nearby. Expected: matches gamemd.exe.

3. **Force-fire on ally**: Select 1 Grizzly, Ctrl+click an allied unit.
   Grizzly fires on the ally. Expected: matches gamemd.exe.

4. **Force-fire on own MCV**: Select 1 Grizzly, Ctrl+click your own MCV.
   Grizzly fires on the MCV. Expected: matches gamemd.exe.

5. **Mixed selection**: Select [Engineer + Grizzly], Ctrl+click empty cell.
   Engineer walks to the cell; Grizzly walks into range and fires at the cell.
   Expected: matches gamemd.exe (no attack from Engineer).

6. **Multi-Grizzly cell-fire**: Select [4× Grizzly], Ctrl+click empty cell.
   All 4 Grizzlies target the same cell — NO radial spread. Each walks into
   its own weapon range from its current position and fires.
   Expected: matches gamemd.exe.

7. **Alt+Ctrl = attack-move (NOT force-fire)**: Hold Alt+Ctrl, click empty
   cell. Selected units attack-move toward the cell, NOT force-fire at it.
   Expected: matches gamemd.exe.

8. **Alt alone = force-move**: existing behavior; verify still works
   (regression check).

9. **Shrouded-cell click**: With `FogOfWar=false` (YR default), all explored
   cells stay visible. Click a never-explored shrouded cell with Ctrl held —
   no command is issued (no walk, no fire). Expected: matches gamemd.exe.

10. **Voice cue**: Multi-unit force-fire-cell — only ONE acknowledge voice
    cue plays, not one per unit. Expected: matches gamemd.exe.

11. **Out-of-range force-fire**: Ctrl+click a cell far away. Unit walks toward
    the cell until it enters weapon range, then fires. Expected: matches
    gamemd.exe.

If any step diverges from gamemd, **STOP** and re-read the design doc / open
a follow-up issue. Do not "fix" by ad-hoc patches.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-07-force-fire-ctrl-click-design.md](2026-05-07-force-fire-ctrl-click-design.md)
- **Ghidra reports:**
  - `ra2-rust-game-docs/MouseClass_research.md` — `What_Action_OnCell` semantics, force-fire cursor mapping (§12.9)
  - `ra2-rust-game-docs/DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` — action codes 0x33 / 0x08; per-unit dispatch (§2)
  - `ra2-rust-game-docs/HOTKEY_SYSTEM_GHIDRA_REPORT.md §15` — modifier-key polling globals
- **gamemd.exe addresses:**
  - `0x00700600` `TechnoClass::What_Action_OnCell` — force-fire dispatch on cell click
  - `0x004AB9B0` `DisplayClass::BandBox_LeftUp` — order issuance
  - `0x004AE750` `Selection::DispatchMultiUnitOrder` — per-unit dispatch, no group-spread doc-comment
- **INI keys:** None (force-fire is purely input-side)
- **Related code:**
  - [src/sim/command.rs:45](../../src/sim/command.rs#L45) — existing `Command::ForceAttack`
  - [src/sim/combat/mod.rs:138](../../src/sim/combat/mod.rs#L138) — existing `AttackTarget`
  - [src/sim/world/world_commands.rs:319](../../src/sim/world/world_commands.rs#L319) — existing `ForceAttack` dispatch
  - [src/app_context_order.rs:43](../../src/app_context_order.rs#L43) — existing `force_fire` flag
  - [src/app_entity_pick.rs:61](../../src/app_entity_pick.rs#L61) — existing `pick_any_target_stable_id`
- **Snapshot:** [src/sim/snapshot.rs:16](../../src/sim/snapshot.rs#L16) — `SNAPSHOT_VERSION = 4 → 5`
