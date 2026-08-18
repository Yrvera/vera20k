# Chrono Miner Player-Command Teleport (FSM Finding 4) Design

## Goal

When the player right-clicks an empty cell with a chrono miner, the miner
warps if its dispatch locomotor is currently Teleport (i.e., it just
finished a drive) and drives otherwise (it just finished a warp). Match
gamemd's IPiggyback dispatch dance for player commands.

## Architecture Context

The chrono miner in gamemd has two locomotors connected via the
IPiggyback COM interface. At any moment one is "active" (dispatches
`Set_Destination`) and the other is stored underneath. After each
locomotor completes its job, `FootClass::AI` swaps them.

- After a drive completes → Teleport becomes active (next click warps).
- After a warp completes → Drive becomes active (next click drives).

`TechnoClass::Set_Destination` at 0x741970 decides per command, based on
the active CLSID and whether the destination cell holds a building:
- Cell has a building → always force Drive piggyback.
- Cell is empty + active is Teleport → skip piggyback, warp.
- Cell is empty + active is Drive → install Drive piggyback (no-op),
  drive.

Today's Rust port hard-codes chrono miner player commands to drive via a
`!info.is_harvester` filter at [world_commands.rs:154-158](../../src/sim/world/world_commands.rs#L154-L158)
and the identical filter at line 405-407 (`AttackMove`). The Rust
miner system already issues teleport on FSM-internal returns
(`begin_return`, `handle_forced_return`) via `issue_teleport_command`
with `is_harvester=true` (instant warp, no chrono lock).

We add persistent mode tracking on `LocomotorState` so the player-command
path can dispatch warp-vs-drive correctly.

## Impact Analysis

**Files changed:**
- [src/sim/movement/locomotor.rs](../../src/sim/movement/locomotor.rs) — new `PiggybackMode` enum + 3 fields on `LocomotorState`.
- [src/sim/movement/piggyback_mode.rs](../../src/sim/movement/piggyback_mode.rs) — new module, mode-flip tick phase.
- [src/sim/movement/teleport_movement.rs](../../src/sim/movement/teleport_movement.rs) — host `pub fn spawn_warp_effects`, moved from miner_system.rs.
- [src/sim/movement/mod.rs](../../src/sim/movement/mod.rs) — `pub mod piggyback_mode;`.
- [src/sim/world/world_commands.rs](../../src/sim/world/world_commands.rs) — `cell_has_structure` helper + new dispatch in `Move` and `AttackMove`; call `spawn_warp_effects` for player-command teleports (fixes the existing Chrono Legionnaire silent-warp drift as a free bonus).
- [src/sim/world/mod.rs](../../src/sim/world/mod.rs) — schedule `tick_piggyback_mode` at end of Phase 2.
- [src/sim/miner/miner_system.rs](../../src/sim/miner/miner_system.rs) — remove the private `spawn_warp_effects` (calls now point at the moved version).
- [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) — new tests.

**Blast radius:**
- `LocomotorState` is serde-serialized — adding fields needs a save format
  migration. We assume no shipped save files, so no migration shim.
- Mode-flip phase runs once per tick over all entities — O(N) per tick,
  negligible.
- Determinism: piggyback_mode mutations are deterministic
  (`keys_sorted()` iteration, single mutation per entity per tick).
  Mode field hashes into state hash via locomotor serialization.

**Risk:**
- Wrong initial mode → first player click misbehaves once. Mitigated by
  test 1 (initial-mode assertion).
- Phase ordering wrong → mode flip lags by one tick. Phase 2 placement
  (after teleport + after ground move) is the correct order; tests 5/6
  verify flip happens within one tick of the completion.
- Building-at-cell scan is O(N entities) per command — commands are rare,
  fine.

## Chosen Approach

**LocomotorState owns `piggyback_mode`. A dedicated post-movement phase
flips it. world_commands reads it for the dispatch decision. Building
check forces drive regardless of mode. Player-command warps use the
full chrono delay and spawn warp effects (animation + sounds).**

Rejected alternatives:
- Putting mode on the `Miner` component — narrower, but mode is
  semantically a locomotor concept, not a harvester one. Future hybrid
  Drive+Teleport units would need duplicated logic.
- Lazy mode inference from current entity state — can't distinguish
  "just-completed drive" from "currently driving."

## Tiny-Detail Ledger

1. **Flip after warp completes** → `DriveActive`. `[GHIDRA 0x4DA530, 0x719F30]`
2. **Flip after drive completes** → `TeleportActive`. `[GHIDRA 0x4DA530, 0x4AF970]`
3. **Initial mode for `Teleporter=yes && base != Teleport`** = `TeleportActive`. `[doc: CHRONO_MINER_SYSTEM_OVERVIEW §2]`
4. **Set_Destination dispatch** `[GHIDRA 0x7424CD-0x7424FA]`:
   - Building cell → always drive.
   - Empty cell + TeleportActive → warp.
   - Empty cell + DriveActive → drive.
5. **Building check is owner-agnostic** — `CellClass::FindFirstBuilding` returns any structure. `[GHIDRA 0x47EBA0]`
6. **War miners** are gated out at `0x7423D3` (TypeClass+0xCD4 == 0). Default `DriveActive`, mode never read.
7. **Chrono delay for player-command warp**: `delay = distance / ChronoDistanceFactor`, clamped to `≥ ChronoMinimumDelay`. Passed via `is_harvester=false`. `[GHIDRA Phase 0, ChronoMinerTeleport report §5]`
8. **Pre-existing drift, intentionally untouched**: `begin_return` / `handle_forced_return` keep `is_harvester=true` (instant warp). Player-command path uses full delay.
9. **WarpOut anim + ChronoIn/OutSound** emitted at both endpoints. Shared `spawn_warp_effects` in `teleport_movement.rs`. `[GHIDRA Phase 0 steps 3, 9, 12]`
10. **AttackMove command** at world_commands.rs:405-407 — same fix applied.
11. **Mid-warp player command** — pre-existing behavior preserved (new movement_target stacks atop in-flight teleport_state; the warp completes first). Not in scope.
12. **Determinism** — `piggyback_mode` + the two booleans serde-serialize into LocomotorState → present in state hash. Mode-flip phase iterates `keys_sorted()`.
13. **War miner default** — `DriveActive` (never read for non-teleporters, cheap to store).

## Design

### Components

**A. New types in [src/sim/movement/locomotor.rs](../../src/sim/movement/locomotor.rs):**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PiggybackMode {
    /// Drive locomotor is currently the dispatch — Set_Destination drives.
    DriveActive,
    /// Teleport locomotor is currently the dispatch — Set_Destination warps.
    TeleportActive,
}
```

Three new fields on `LocomotorState`:
```rust
pub piggyback_mode: PiggybackMode,
pub had_teleport_state: bool,
pub had_movement_target: bool,
```

`LocomotorState::from_object_type`:
```rust
let piggyback_mode = if obj.teleporter && obj.locomotor != LocomotorKind::Teleport {
    PiggybackMode::TeleportActive
} else {
    PiggybackMode::DriveActive
};
```

**B. New module [src/sim/movement/piggyback_mode.rs](../../src/sim/movement/piggyback_mode.rs):**

```rust
pub fn tick_piggyback_mode(entities: &mut EntityStore) {
    for &id in &entities.keys_sorted() {
        let Some(entity) = entities.get_mut(id) else { continue };
        let cur_teleport = entity.teleport_state.is_some();
        let cur_movement = entity.movement_target.is_some();
        let Some(ref mut loco) = entity.locomotor else { continue };

        if loco.had_teleport_state && !cur_teleport {
            loco.piggyback_mode = PiggybackMode::DriveActive;
        } else if loco.had_movement_target && !cur_movement {
            loco.piggyback_mode = PiggybackMode::TeleportActive;
        }
        loco.had_teleport_state = cur_teleport;
        loco.had_movement_target = cur_movement;
    }
}
```

Scheduled in [world/mod.rs](../../src/sim/world/mod.rs) immediately after
`tick_teleport_movement` (line ~1078). Both ground move (Phase 1) and
teleport (Phase 2) have already completed by then, so transitions are
visible on the same tick they happen.

**C. `spawn_warp_effects` moved to `teleport_movement.rs`:**

Promoted from `pub(crate)` in [miner_system.rs](../../src/sim/miner/miner_system.rs)
to a `pub(crate) fn` in `teleport_movement.rs`. Signature unchanged:
takes `&mut Simulation`, `&RuleSet`, `type_id`, `depart`, `arrive`.
Miner system import path updated.

**D. `cell_has_structure` helper in `world_commands.rs`:**

```rust
fn cell_has_structure(
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &Interner,
    cell: (u16, u16),
) -> bool {
    for e in entities.values() {
        if e.category != EntityCategory::Structure { continue }
        let Some(obj) = rules.object(interner.resolve(e.type_ref)) else { continue };
        let (w, h) = foundation_dimensions(&obj.foundation);
        if cell.0 >= e.position.rx && cell.0 < e.position.rx + w
            && cell.1 >= e.position.ry && cell.1 < e.position.ry + h
        {
            return true;
        }
    }
    false
}
```

O(N structures) per command. Owner-agnostic (matches gamemd).

**E. Dispatch in Move and AttackMove handlers:**

Replace existing `use_teleport_move = !info.is_harvester && (...)` with:

```rust
let dest_has_building = match rules {
    Some(r) => cell_has_structure(&self.entities, r, &self.interner,
                                   (*target_rx, *target_ry)),
    None => false,
};
let mode_warps = self.entities.get(*entity_id)
    .and_then(|e| e.locomotor.as_ref())
    .map(|l| l.piggyback_mode == PiggybackMode::TeleportActive)
    .unwrap_or(false);
let use_teleport_move = !dest_has_building
    && (info.loco_kind == Some(LocomotorKind::Teleport)
        || (info.is_teleporter && mode_warps));
```

The first disjunct (`loco_kind == Teleport`) keeps Chrono Legionnaire
warping always — its base locomotor is Teleport so it has no Drive
mode to swap into.

When `use_teleport_move`, call `spawn_warp_effects` BEFORE
`issue_teleport_command(is_harvester=false)`. Argument:
`(self.entities.get(entity_id).map(|e| (e.position.rx, e.position.ry, e.position.z))`
unwrapped, target, type_id).

### Data Flow

```
Per sim tick:
  Phase 0 commands
    apply_command Move/AttackMove on chrono miner:
      → cell_has_structure(target_cell)
      → read locomotor.piggyback_mode
      → either spawn_warp_effects + issue_teleport_command,
        or issue_move_command_with_layered
  Phase 1 ground movement
    → may clear movement_target on path completion
  Phase 2 special movement
    teleport_movement::tick_teleport_movement
      → may clear teleport_state on warp completion
    piggyback_mode::tick_piggyback_mode (NEW)
      → detect transitions, flip mode, update last-tick cache
  ... (later phases unchanged)
```

### Error Handling

- Entity missing locomotor → skip (existing pattern).
- Missing rules at command time → fall back to drive (cell_has_building defaults to false; no mode-warp without rules).
- Mid-warp player command → existing behavior preserved (no extra handling).

### Testing Strategy

All in [miner_tests.rs](../../src/sim/miner/miner_tests.rs); reuses the
existing `tick_miners_n` helper which orders teleport → miners → ground
move (the phase ordering matters for the flip detection).

1. **`chrono_miner_initial_mode_is_teleport_active`** — fresh CMIN spawn → `piggyback_mode == TeleportActive`.
2. **`war_miner_initial_mode_is_drive_active`** — HARV spawn → `DriveActive`.
3. **`chrono_miner_player_move_warps_on_empty_cell_when_teleport_active`** — set TeleportActive, issue `Command::Move` to empty cell, assert `teleport_state.is_some()`.
4. **`chrono_miner_player_move_drives_on_empty_cell_when_drive_active`** — set DriveActive, issue Move, assert `movement_target.is_some()` and `teleport_state.is_none()`.
5. **`chrono_miner_player_move_drives_to_building_cell_regardless_of_mode`** — TeleportActive, target = refinery cell → drive issued.
6. **`mode_flips_to_drive_active_after_warp_completes`** — issue_teleport_command directly, run tick_teleport_movement + tick_piggyback_mode → mode becomes DriveActive.
7. **`mode_flips_to_teleport_active_after_drive_completes`** — issue drive on a one-cell path, tick movement + piggyback → mode becomes TeleportActive.
8. **`war_miner_player_move_always_drives`** — HARV with default DriveActive, click empty cell → drives (proves mode field is harmless for non-teleporters).
9. **`harvest_cycle_mode_transitions`** — end-to-end: chrono miner cycle warp-back-from-ore → undock-drive → drive-to-ore → harvest → warp-return. Assert mode at each named stage.
10. (Regression) Existing `chrono_miner_teleports_to_refinery_on_return`, `forced_return_chrono_teleports`, and dock-sequence tests still pass.

## Architectural Decisions

- **Field placement on LocomotorState** (not Miner) — mode is a locomotor concept; future hybrid units don't need duplicated state. The flip-detection logic is generic over any entity, not coupled to the miner FSM.
- **Mode-flip phase, not callbacks** — observing state transitions in a single pass is simpler than instrumenting every issue/complete site, and order-independent within the tick.
- **Separate phase after Phase 2** — keeps the flip detection cohesive with the movement subsystem it observes.
- **Shared `spawn_warp_effects`** — fixes the pre-existing Chrono Legionnaire silent-warp drift as a side benefit, consolidates the WarpOut/sound logic in one place.
- **Pre-existing FSM-internal `is_harvester=true` shortcut kept** — documented as a separate parity drift; out of scope here.

## Alternatives Considered

- **Mode on `Miner` component.** Smaller change but couples a locomotor concept to a single unit type. Rejected per user choice.
- **Lazy mode inference at command time** (no stored state, infer from current movement_target/teleport_state). Doesn't work: "currently driving" ≠ "just finished a drive."
- **Skip building check, only model empty-cell rule.** Cheaper, but player-visible parity drift when clicking buildings. Rejected per user choice.
- **Instant warp for player commands (`is_harvester=true`).** Consistent with FSM-internal but player-visible drift vs gamemd. Rejected per user choice.
- **Skip warp effects for player command.** Consistent with current Chrono Legionnaire path but parity drift. Rejected per user choice.
