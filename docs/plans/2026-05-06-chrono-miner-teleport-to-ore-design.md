# Chrono Miner Teleport-to-Ore Design

## Goal
Make the Chrono Miner teleport to ore on every harvest cycle after its first refinery dock, matching gamemd.exe.

## Architecture Context

The miner state machine (`src/sim/miner/`) drives a chrono/war miner through
`SearchOre → MoveToOre → Harvest → Return → Dock → Unloading → ExitPad → SearchOre`.
Top-level FSM is `MinerState` in `mod.rs`; the `Dock`-state internals are a sub-FSM
in `miner_dock_sequence.rs` (`DockPhase: Approach | Pre-Dock | Docking |
Unloading | ExitPad`).

Movement primitives in `src/sim/movement/`:
- `issue_direct_move` — drive, bypass A* (used by undock exit)
- `issue_move_if_idle` — drive, A* (used by ore search and refinery return)
- `issue_teleport_command` — chrono warp with delay clamping (used by chrono
  return-to-refinery in three sites: `handle_return`, `handle_forced_return`,
  `begin_return`)

Today, `handle_move_to_ore` always issues a drive for both miner kinds. The
chrono miner has correct teleport behavior on the return leg (gated by
`ChronoHarvTooFarDistance`) but always drives to ore — wrong vs retail.

## Impact Analysis

**Touched files:**
- `src/sim/miner/mod.rs` — add `LastDestKind` enum, add field to `Miner`
- `src/sim/miner/miner_system.rs` — gate in `handle_move_to_ore`, updates at
  6-7 movement-command issue sites
- `src/sim/miner/miner_dock_sequence.rs` — no changes (undock drive deliberately
  excluded from updates)

**Risk areas:**
- The "intent-level only" update rule must be applied consistently. New code
  paths that issue movement without updating the field will silently break the
  gate. Mitigation: keep updates in 6-7 well-named call sites, all in
  `miner_system.rs`.
- Snapshot serialization will need to include the new field — trivial since
  it's a small enum.

**Determinism:** field is plain enum data, deterministically updated at
fixed call sites within the existing tick order. State hash includes it via
the Miner hashing path.

**Blast radius:** miner module only. No cross-module coupling, no public API
changes.

## Chosen Approach

**Approach 1 (Miner-local field, intent-level updates).**

Add `last_destination_kind: LastDestKind` to `Miner`, where `LastDestKind` is
`{ None, Cell, Building }`. Updated at FSM-intent-level movement command sites
(picking ore, picking refinery). NOT updated by sub-FSM movement (dock
approach steps, undock exit drive).

Gate in `handle_move_to_ore` for `MinerKind::Chrono`:
- If `last_destination_kind == Building`: `issue_teleport_command(ore_cell)`
- Else: `issue_move_if_idle(ore_cell)` (existing drive)

Cycle-0 (fresh spawn): `last_destination_kind = None` → drives. After first
dock cycle, the dock-approach issue site sets it to `Building`, persisting
through unload + ExitPad (since exit drive is sub-FSM and does not update).
On state 0 entry of cycle 1, `Building` is still set → teleport to ore.

This faithfully reproduces gamemd's observable behavior: drive cycle 0,
teleport from cycle 1 onward, matching the binary's `current_dest` state at
the moment `Set_Destination(ore_cell)` evaluates the Teleporter block at
`0x7423CD`.

## Tiny-Detail Ledger

Parity-relevant details the implementation must preserve. Each cites source.

- **Vanilla to-ore behavior**: gamemd has NO distance gate on the to-ore
  direction. Once gates align (current_dest=Building, new_dest=empty Cell,
  base loco = Teleport), teleport fires regardless of distance.
  `[GHIDRA Set_Destination@0x741970, gates @0x742472–0x7424FA]`
- **Cycle-0 fresh-spawn drives**: gamemd's gate fails because current_dest is
  null on a fresh-spawn unit. `[GHIDRA gate 1 @ 0x74246C]`
  Our model: `LastDestKind::None` → fail Building gate → drive.
- **Free-spawn cycle-0**: `[UNKNOWN — needs RE]` whether gamemd's free
  harvester from a refinery initializes nav_target to its parent. Likely no.
  Accept as known parity drift if it turns out gamemd teleports here.
- **Teleport delay formula**:
  `delay_frames = clamp(distance_in_leptons / ChronoDistanceFactor, ChronoMinimumDelay, ChronoMaximumDelay)`
  `[doc: CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md §3]`. Already implemented in
  `issue_teleport_command`.
- **50% translucency at destination during delay**:
  `[doc: CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md §5]`. Already in
  `TeleportPhase` warp-in.
- **WarpOut at source / WarpAway at destination sprite split**: fixed in
  commit 5ebf6f3.
- **ChronoInSound = ChronoOutSound = "ChronoMinerTeleport"**:
  `[ini: rulesmd.ini CMIN]`. Same sound both ends, already wired in
  `issue_teleport_command`.
- **Speed during return drive (within ChronoHarvTooFarDistance)**:
  Speed=4 normal Drive locomotion. Same as War Miner road speed.
  `[ini: rulesmd.ini CMIN]`. Existing path, no change.
- **Ore-cell becomes invalid during chrono delay**: gamemd behavior
  `[UNKNOWN — needs RE]` but likely warps anyway and re-scans next state-0
  tick. Our existing teleport machinery does the same. ✓
- **Refinery TypeClass+0x16b3 flag** `[GHIDRA 0x7424BD]`: gates the to-ore
  teleport on the building's TypeClass. Likely `Refinery=yes` or `Dock=yes`.
  We don't model this flag because chrono miner only docks at refineries
  in our scope. **Defer until a second Teleporter+Dock combination exists.**
- **Return leg uses different mechanism**:
  `[GHIDRA UnitClass::Mission_Harvest@0x73E5E0 case 2]`. State 2 has its own
  distance check against `Rules+0xD7C` (= `ChronoHarvTooFarDistance` cells) and
  uses a separate code path. Our existing return-leg gate already mirrors
  this; no change.
- **Cycle-2 entry mechanism** `[GHIDRA Mission_Harvest case 0 + Search_For_Tiberium_Short_And_Move@0x4DDB90]`:
  state 0 first calls `Set_Destination(nav_target=refinery)` if nav_target
  persists from previous cycle, then calls
  `Search_For_Tiberium_Short_And_Move` which itself calls
  `Set_Destination(ore_cell)`. Our equivalent: `last_destination_kind` is
  preserved through unload + ExitPad as `Building`, and `MoveToOre`'s gate
  reads it directly — captures the same effect without the dance.

## Design

### Components

```rust
// src/sim/miner/mod.rs

/// What kind of destination the miner most recently committed to at the
/// FSM-intent level. Mirrors gamemd's current_dest type for the Set_Destination
/// teleport gate. Sub-FSM movements (dock approach, undock exit drive) do NOT
/// update this — only top-level intent transitions do.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum LastDestKind {
    None,      // fresh spawn, no destination ever committed
    Cell,      // last intent was a map cell (ore search, attack-move, etc.)
    Building,  // last intent was a building (refinery dock approach, etc.)
}

pub struct Miner {
    // ... existing fields ...
    pub last_destination_kind: LastDestKind,
}

impl Miner {
    pub fn new(/* ... */) -> Self {
        Self {
            // ...
            last_destination_kind: LastDestKind::None,
        }
    }
}
```

### Interfaces / Contracts

**Public surface:** none changed. `LastDestKind` is `pub` only because it's
a field type on `pub struct Miner`; nothing outside the miner module reads or
writes the field.

**Update invariant:**
> `last_destination_kind` reflects the kind of the most recent FSM-intent-level
> destination committed by the miner system. It is updated *exactly* at the
> sites where `handle_*` functions issue movement commands for top-level
> FSM transitions (move-to-ore, return-to-refinery, dock approach). Sub-FSM
> movement (dock-sequence internals, exit drive) does not update it.

### Data Flow

```
[fresh spawn]
  Miner.last_destination_kind = None

[state 0 SearchOre picks ore cell]
  no field update yet; transition to MoveToOre

[handle_move_to_ore for MinerKind::Chrono]
  match last_destination_kind:
    Building → issue_teleport_command(ore_cell)
    _        → issue_move_if_idle(ore_cell)
  AFTER successful issue: last_destination_kind = Cell

[handle_move_to_ore for MinerKind::War]
  always issue_move_if_idle (existing behavior)
  AFTER issue: last_destination_kind = Cell  // still tracked for consistency

[harvest completes, full → state 2]
  begin_return chooses teleport or drive based on existing distance gate
  AFTER issue: last_destination_kind = Building

[arrival at refinery, dock_phase = Approach → Pre-Dock → Docking]
  dock-approach issue sites set last_destination_kind = Building
  (already-Building from begin_return, but explicitly set for safety)

[Unloading state]
  no movement issued; last_destination_kind unchanged (Building)

[ExitPad: undock drives to exit cell via issue_direct_move]
  NO update to last_destination_kind — sub-FSM movement
  (this is the critical case that preserves Building through to next cycle)

[arrival at exit cell → state = SearchOre]
  last_destination_kind is still Building from dock approach

[next state 0 SearchOre → MoveToOre]
  chrono branch sees Building → teleport
```

### Update Sites (concrete locations)

In `miner_system.rs`:

1. `handle_move_to_ore` line ~378 (drive issue): after `issue_move_if_idle`,
   set `Cell`
2. `handle_move_to_ore` new chrono teleport branch: after
   `issue_teleport_command`, set `Cell`
3. `handle_return` line ~476 (lost-reservation rebuild teleport): set `Building`
4. `handle_forced_return` line ~610 (teleport): set `Building`
5. `begin_return` line ~710 (teleport): set `Building`
6. `begin_return` drive path (when within distance gate): set `Building`
7. Dock-approach drive sites (in `handle_return` or `handle_dock_*`): set
   `Building`

The exact line numbers are anchors to verify in `/write-plan`; the rule is
"every place that issues a movement command targeting an FSM-intent-level
destination updates the field with the destination kind."

### Error Handling

- Default `None` is safe — failing the Building gate routes to drive, the
  conservative path.
- Refinery destroyed mid-cycle while `Building` is set: field stays `Building`
  until next intent-level command. Next ore search teleports, which is fine
  — the gate controls *how* not *whether* to move. Matches gamemd's harmless
  dangling current_dest pointer.
- Snapshot load with old saves (no field): default to `None` on
  deserialization. Worst-case effect: one drive instead of teleport on the
  first post-load cycle. Acceptable.

### Testing Strategy

Unit tests added to `miner_system.rs` (or new `miner_chrono_teleport_test.rs`
if file growth pushes past the ~600-line guideline):

1. **`chrono_cycle_zero_drives`**: fresh chrono miner, `last_destination_kind
   = None`. Run state machine through MoveToOre. Assert: a drive command was
   issued, NOT a teleport.
2. **`chrono_cycle_one_teleports`**: chrono miner with
   `last_destination_kind = Building`. MoveToOre tick. Assert: teleport
   command issued with correct chrono delay (function of ore-cell distance).
3. **`war_miner_unaffected_by_field`**: war miner with
   `last_destination_kind = Building`. MoveToOre tick. Assert: drive command
   issued (gate is `MinerKind::Chrono`-only).
4. **`field_updates_through_full_cycle`**: simulate ore → harvest → return →
   dock → unload → next ore. Assert field transitions at expected ticks:
   `None → Cell → Building → Building (preserved through undock) → Cell`.
5. **`undock_exit_drive_preserves_building`**: from Dock state with
   `Building`, run through Unloading → ExitPad → drive-to-exit → arrival →
   SearchOre transition. Assert: `last_destination_kind` is still `Building`
   throughout, despite `issue_direct_move` being called for the exit drive.

### Determinism

`LastDestKind` is `Copy + Eq + Hash` plain enum. Updates are deterministic
inline writes at known call sites. No RNG, no float math, no async, no
ordering-sensitive interactions with other systems. State hash includes the
field via the Miner hashing path (will need a one-line addition).

## Architectural Decisions

**Patterns followed:**
- Plain field on the existing `Miner` struct — matches how other miner state
  is represented (`MinerState`, `dock_phase`, `target_ore_cell`).
- Updates at issue sites — matches how `target_ore_cell`, `home_refinery`,
  `last_harvest_cell` are updated (set inline at the moment the FSM commits to
  a new value).
- No new module, no new component, no new abstraction.

**Patterns deviated from:** none.

**Tech debt introduced:**
- If we add Chrono Legionnaire / Chrono Commando / any other Teleporter=yes
  unit, we'd duplicate the pattern on `InfantryClass` or `UnitClass` and
  eventually want to refactor to a shared component (Approach 2 from
  brainstorm). Estimated cost: 2-3 hours when that day comes.

## Alternatives Considered

**Approach 2: Entity-level component, command-site updates with undock dance.**
Rejected: more surface area for an observable behavior already captured by
Approach 1. Generalizes to future Teleporter units, but those units aren't
on the roadmap; the refactor cost when they arrive is small.

**Approach B from brainstorm: always teleport for MinerKind::Chrono.**
Rejected: introduces a one-extra-teleport drift on cycle 0 of every chrono
miner's lifetime. User chose the most-faithful path.

**Approach A from brainstorm: one-shot `has_docked: bool` flag.** Rejected:
captures the right *effect* but not the right *mechanism* — would miss edge
cases like manual nav_target shifts (player attack-moving the chrono miner
to a building, then ordering ore harvest).
