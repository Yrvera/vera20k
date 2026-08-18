# Force-Fire (Ctrl-Click) Parity — Design

**Date:** 2026-05-07
**Status:** Approved (brainstorm complete)
**Scope:** "(b+)" — input-side force-fire parity excluding disguise interaction

## Goal

Bring Ctrl-click force-fire to gamemd.exe parity: armed units fire at the clicked
cell (including empty terrain), unarmed units fall through to Move, Alt+Ctrl
routes to attack-move (not force-fire), and the cursor reflects force-fire state
live as Ctrl is held.

## Architecture Context

### Current state of the click → command pipeline

Mouse input enters at `app_input::handle_mouse_input` ([src/app_input.rs:34](../../src/app_input.rs#L34)),
which dispatches `MouseButton::Left` orders to
`app_context_order::try_queue_context_order_at_screen_point`
([src/app_context_order.rs:33](../../src/app_context_order.rs#L33)).

Order resolution polls Ctrl/Shift state inline (`is_ctrl_held`, `is_shift_held`
in [src/app_input.rs:628](../../src/app_input.rs#L628)) — modifier state lives
in the app/render layer, never crossing into sim. Per-unit commands are emitted
in a loop ([src/app_context_order.rs:501](../../src/app_context_order.rs#L501))
and queued onto `sim.pending_commands` as `CommandEnvelope`. Each tick,
`World::advance_tick` drains due commands and dispatches via
`world_commands::apply_command` ([src/sim/world/world_commands.rs:94](../../src/sim/world/world_commands.rs#L94)).

### What's already in place

- `Command::ForceAttack { attacker_id, target_id }` exists as a distinct variant
  in [src/sim/command.rs:45](../../src/sim/command.rs#L45). Bypasses friendship
  filter at dispatch time.
- `pick_any_target_stable_id` in [src/app_entity_pick.rs:61](../../src/app_entity_pick.rs#L61)
  picks any entity (including friendlies) under cursor when force-fire is
  active. Filters out shrouded enemies.
- Order-resolution branches on `force_fire` to suppress refinery-return,
  ore-harvest, and friendly-select fall-through behaviors. Emits
  `Command::ForceAttack` when an entity is hit
  ([src/app_context_order.rs:504](../../src/app_context_order.rs#L504)).

### What's missing

- **Force-fire on empty terrain** — `ForceAttack` only carries `target_id: u64`.
  Ctrl-click on empty ground falls through to `Move`.
- **Alt+Ctrl override** — Alt held is supposed to *clear* the force-fire flag
  (Alt+Ctrl = attack-move, not force-fire). We don't poll Alt at all.
- **Cursor swap when Ctrl held** — needs verification/extension in
  [src/app_cursor.rs](../../src/app_cursor.rs).
- **Unarmed unit handling** — Engineer/Harvester/MCV ctrl-clicked currently
  queue a doomed force-attack; gamemd falls them through to Move.
- **Shrouded-cell rejection** — no client-side check on click.
- **Disguise piercing** — *deferred, blocked on disguise system not implemented*.

## Impact Analysis

| Layer | File | Reason |
|---|---|---|
| sim/command | [src/sim/command.rs](../../src/sim/command.rs) | New `ForceAttackCell { attacker_id, target_rx, target_ry }` variant (appended for snapshot back-compat) |
| sim/dispatch | [src/sim/world/world_commands.rs](../../src/sim/world/world_commands.rs) | Dispatch arm for the new variant |
| sim/components | wherever `attack_target` lives | `Option<u64>` → `Option<AttackTarget>` |
| sim/combat | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs), `combat_targeting.rs` | Resolve weapon target coords from `AttackTarget::Cell` directly |
| app/input | [src/app_input.rs](../../src/app_input.rs) | New `is_alt_held` helper |
| app/order | [src/app_context_order.rs](../../src/app_context_order.rs) | Alt+Ctrl detection, ForceAttackCell emission, unarmed fall-through, shroud rejection |
| app/cursor | [src/app_cursor.rs](../../src/app_cursor.rs) | Force-fire cursor swap |
| Tests | sim integration + serde tests | Snapshot/replay coverage of new command + enum |

### Dependencies on what we're changing
- **Snapshot serialization**: `AttackTarget` enum widens an existing field. Bump snapshot version.
- **Replay format**: appending a new `Command` variant is back-compat for serde with explicit indices; never reorder existing variants.
- **Network lockstep**: command flows over the wire identically; no new message type needed.
- **Auto-acquire / passive targeting**: stays entity-only; cell targets are only set by explicit player command, never by acquisition logic.

### Determinism / state-hash
- `AttackTarget` enum participates in entity hashing via existing serde derive.
- All math fixed-point: cell-center leptons = `(rx as i32 * 256 + 128, ry as i32 * 256 + 128)`.
- No tick-order changes; no new global state.

### Blast radius
- Risk to existing entity-target attacks: **low** — additive variants and branches.
- Risk to replay/snapshot back-compat: **medium** — snapshot version bump required.
- Risk to UI: **low** — cursor swap is a localized read of modifier state.

## Chosen Approach

**Approach A — `ForceAttackCell` command + `AttackTarget` enum on entity.**

Add a new command variant (`ForceAttackCell`) and widen the entity's combat
target field from `Option<u64>` to `Option<AttackTarget>` where
`AttackTarget = Entity(u64) | Cell(u16, u16)`. Each selected unit emits its own
command at order-issue time; mixed selections naturally split (armed →
`ForceAttackCell`, unarmed → `Move`). Combat resolves target coords from the
enum at fire time.

This matches gamemd's per-unit polymorphic dispatch (verified in
`Selection__DispatchMultiUnitOrder` at `0x4ae750`) and keeps the change
additive within existing patterns.

## Tiny-Detail Ledger

Sourced from `MouseClass_research.md`,
`DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md`, `HOTKEY_SYSTEM_GHIDRA_REPORT.md`,
and direct decompiles of `TechnoClass::What_Action_OnCell` (`0x00700600`),
`DisplayClass::BandBox_LeftUp` (`0x4AB9B0`), and
`Selection::DispatchMultiUnitOrder` (`0x4AE750`).

### Modifier key resolution
- **Ctrl alone (no Alt) = force-fire.** Polled via `GetAsyncKeyState` on the
  Ctrl global pair `DAT_00a8ec00 / DAT_00a8ec04`.
  [doc: HOTKEY_SYSTEM_GHIDRA_REPORT.md §15]
- **Alt+Ctrl = attack-move**, NOT force-fire. Alt held *clears* the Ctrl
  force-fire flag — `if (alt_flag && bVar4) bVar4 = false` at `0x700706`.
  [GHIDRA 0x00700600]
- **Alt alone = force-move** (out of scope here; modifier check must still
  recognize so Alt-clicks don't accidentally trigger force-fire).
  [doc: HOTKEY_SYSTEM_GHIDRA_REPORT.md §15]
- **Shift orthogonal**: queue waypoint. Combines with force-fire (queued
  force-fire is legal). [doc: HOTKEY_SYSTEM_GHIDRA_REPORT.md §15]
- **Modifiers polled at decision time, not captured at click time.** gamemd
  re-polls inside `What_Action_OnCell`. Our existing per-click polling matches.
  [GHIDRA 0x00700600]

### Action selection on force-fire-on-cell
- **Armed unit + empty cell + Ctrl held** → action 1 (Attack). Unit walks into
  weapon range, fires at cell coords. [GHIDRA 0x00700600 — final branch
  returns `1` when `vtable[0xa0]()` (has-weapon) is true and `param_3`
  (force-fire) is set]
- **Unarmed unit + empty cell + Ctrl held** → action 2 (Move). No fire attempt.
  [GHIDRA 0x00700600 — falls through to `return 2` when `vtable[0xa0]()`
  returns false]
- **Mixed selection** (e.g. [Engineer + Grizzly]): cursor reflects the *best*
  unit (`SelectBestObjectForAction` priority: armed mobile = 5 beats unarmed
  mobile = 4), but each unit independently resolves its own action — Grizzly
  fires, Engineer walks. [doc: DETERMINE_ACTION_DOWNSTREAM §2]
- **Harvester explicitly excluded from being given attack orders directly** via
  `TechnoTypeClass +0xE13`. Falls through to Move.
  [doc: DETERMINE_ACTION_DOWNSTREAM §5]
- **Per-unit dispatch**: each selected unit's `vtable[0x70]` (=
  `What_Action_OnCell`) runs independently; each unit's command is built from
  that unit's resolved action. [GHIDRA 0x4ae750]
- **Action 1 ≡ 0x33 in dispatch**: `if ((param_3 == 1) || (param_3 == 0x33))`
  — same code path. 0x33 is purely a cursor distinction. One command variant
  (`ForceAttackCell`) covers both. [GHIDRA 0x4ae750]

### Multi-unit handling
- **NO group-pathing, NO leader, NO formation movement** for force-fire-on-cell.
  Each selected armed unit gets the same `(rx, ry)` cell coord and resolves its
  own path from its own position. Skip `group_destinations` distribution
  (currently used for plain Move). [GHIDRA 0x4ae750 — direct doc-comment]
- **Voice suppression**: `g_SelectionVoice_Enable = 0` during the dispatch
  loop, restored after. Only one voice cue per multi-unit force-fire batch.
  [GHIDRA 0x4ae750]

### Shroud
- **Force-fire on a shrouded cell**: cell isn't visible, no order issued.
  Cursor falls through to default. [GHIDRA 0x00700600 — `FUN_005023b0(cell)`
  shroud check gates the action-0x33 cursor branch]
- **Force-fire on a previously-revealed-but-unobserved cell** (post-fog): in
  standard YR (`FogOfWar=false`), once revealed always revealed, so this is the
  same as visible — works normally. [CLAUDE.md TS-legacy section]

### Cursor
- **Action 0x33** is the cursor code for "force-attack on cell with targets
  present" — display-only. Actual fire-at-cell uses action 1 downstream.
  [doc: DETERMINE_ACTION_DOWNSTREAM §4 + §8]
- **Force-fire cursor mapping**: when `param_3 != 0` in `SetCursorFromAction`,
  action 1/5 → cursor `0x12` (attack-target), action 2 → `0x13` or `0x12`
  (infantry-Building flag dependent). [doc: MouseClass_research.md §12.9]
- **Cursor must update live as Ctrl is pressed/released** — gamemd recomputes
  every hover tick. [doc: MouseClass_research.md §11–12]
- **Exact mouse SHP frame for action 0x33**: `UNKNOWN — needs RE`. Use
  existing attack cursor for now.

### Friend / foe / targeting
- Force-fire bypasses friendship check at command-issue time. Existing
  `Command::ForceAttack` path already does this.
  [src/app_entity_pick.rs:61, src/sim/world/world_commands.rs:319]
- **Disguise piercing** (Mirage Tank, Spy in disguise): **DEFERRED — blocked
  on disguise system not being implemented in Rust.**

### Range / can-hit verification
- Force-fire respects weapon range — out-of-range force-fire shows
  `AttackOutOfRange` cursor and the unit moves into range first. Existing
  combat pipeline handles this; cell-target reuses it.
  [doc: MouseClass_research.md §12.8]
- Force-fire bypasses `Verses=` at *target acquisition* but not at damage
  application — projectile lands and damage-calc produces zero against immune
  armor. [Inferred from on-entity force-fire current behavior]

## Design

### Components

| Component | Lives in | Role |
|---|---|---|
| `Command::ForceAttackCell { attacker_id, target_rx, target_ry }` | [src/sim/command.rs](../../src/sim/command.rs) | New command variant. Appended to enum (snapshot back-compat). |
| `AttackTarget` enum | [src/sim/components/](../../src/sim/components/) | Replaces `Option<u64>` with `Option<AttackTarget>` where `AttackTarget = Entity(u64) \| Cell(u16, u16)` |
| `apply_force_attack_cell` arm | [src/sim/world/world_commands.rs](../../src/sim/world/world_commands.rs) | Sets attacker's `attack_target = Cell(rx, ry)`; rejects unarmed attacker with warn-log |
| `fire_at_cell` branch | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) | Resolves coords from `Cell` variant; reuses projectile spawn + damage pipeline |
| `is_alt_held` helper | [src/app_input.rs](../../src/app_input.rs) | Mirror of `is_ctrl_held` |
| Force-fire branch in order resolution | [src/app_context_order.rs](../../src/app_context_order.rs) | (a) `force_fire = is_ctrl_held && !is_alt_held`. (b) Per-unit: armed → `ForceAttackCell`, unarmed → `Move`. (c) Skip `group_destinations`. (d) Reject shrouded cell. |
| Cursor swap on Ctrl held | [src/app_cursor.rs](../../src/app_cursor.rs) | Override hover-target cursor to attack-cursor when force-fire active |

No new modules. Everything is additive within existing files. Sim/render
dependency rules preserved (modifier state never enters sim).

### Interfaces / Contracts

```rust
// src/sim/components/...
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackTarget {
    Entity(u64),
    Cell(u16, u16),
}

// src/sim/command.rs (appended to enum Command)
ForceAttackCell {
    attacker_id: u64,
    target_rx: u16,
    target_ry: u16,
},
```

`Command::ForceAttack` stays unchanged — it remains the entity-target form
(Ctrl-click on a unit/building). The new variant covers Ctrl-click on empty
terrain.

No public API changes outside sim/. Render and UI consume the same
`EntityStore` shape; only `attack_target`'s interpretation widens.

### Data Flow

```
[Ctrl held] + [left-click on (x,y)]
  │
  ▼
app_input::handle_mouse_input
  │
  ▼
app_context_order::try_queue_context_order_at_screen_point
  │  reads: is_ctrl_held() && !is_alt_held() → force_fire
  │  reads: hover_target_at_point(...) → maybe entity
  │  reads: shroud check on cell → skip if shrouded
  │
  ├─ force_fire && entity hit         → Command::ForceAttack { attacker, target_id }   [unchanged]
  ├─ force_fire && no entity, armed   → Command::ForceAttackCell { attacker, rx, ry }   [NEW]
  ├─ force_fire && no entity, unarmed → Command::Move { ... }                           [fall-through]
  ├─ alt+ctrl                         → Command::AttackMove { ... }                     [fall-through]
  └─ neither                          → existing logic
  │
  ▼ (per selected unit; emit one command each)
sim.pending_commands.push(envelope)
  │
  ▼ (next tick, after input_delay_ticks)
world_commands::apply_command
  │
  ├─ ForceAttack    → entity.attack_target = Some(Entity(target_id))      [unchanged]
  └─ ForceAttackCell→ entity.attack_target = Some(Cell(rx, ry))           [NEW]
  │
  ▼ (combat tick)
combat::fire_weapon
  │
  ├─ AttackTarget::Entity(id) → resolve target position via entities[id].position
  └─ AttackTarget::Cell(x,y)  → use (x*256+128, y*256+128) directly (cell-center leptons)
  │
  ▼
projectile spawn → existing damage application pipeline (unchanged)
```

Invariants:
- Per-unit command emission preserved; mixed selections naturally split.
- No group-destination distribution for force-fire-cell.
- Modifier state never crosses sim/ boundary.
- Cell coords are `u16` (project's existing cell coord type).

### Error Handling

- **Invalid attacker** (despawned between click and apply): existing dispatch
  drops commands silently when entity is gone. Same path applies.
- **Shrouded cell at command-issue time**: rejected in
  `try_queue_context_order_at_screen_point` — no command issued.
- **Cell out of map bounds**: bounds check at order resolution; reject
  silently. `debug_assert!` in the combat fire-at-cell branch as belt-and-braces.
- **Unarmed unit somehow receives `ForceAttackCell`** (shouldn't happen given
  client-side filter, but desync-safe): `apply_force_attack_cell` re-checks
  for weapon; if none, no-op and warn-log once per attacker. (Per
  feedback_silent_render_failures memory: never silent skip — log the case.)

### Testing Strategy

**Unit tests (sim):**
1. `Command::ForceAttackCell` round-trips through serde (snapshot stability).
2. `AttackTarget` enum serializes deterministically; existing snapshots load
   with `Entity(...)` migration.
3. `apply_force_attack_cell` sets `attack_target = Some(Cell(rx,ry))` and
   queues movement-into-range.
4. Combat tick fires a projectile at correct world-leptons coords for `Cell`
   target.
5. Splash damage at the cell affects nearby entities (parity with on-entity
   force-fire splash).
6. Unarmed unit receiving `ForceAttackCell` logs warn and no-ops.

**Integration tests:**
7. Replay determinism: a recorded session with mixed entity/cell force-fires
   produces identical state hash on replay.
8. Snapshot round-trip: save/load preserves `Cell` target intact.
9. Multi-unit force-fire: 4 Grizzlies + 1 Engineer at same cell → all 4
   Grizzlies queue `ForceAttackCell`, Engineer queues `Move`. Verify per-unit
   dispatch.

**Order-resolution unit tests (app layer):**
10. `force_fire = ctrl && !alt` truth table — Ctrl alone → true; Ctrl+Alt →
    false; Alt alone → false; nothing → false.
11. Force-fire on cell with no `group_destinations` distribution applied.
12. Shrouded-cell click suppresses command emission.

**Manual / visual:**
13. Ctrl-hold cursor swaps to attack-cursor over allies, own units, and empty
    terrain (no click required).
14. Ctrl+left-click empty cell → unit walks into range and fires; splash
    kills hidden units in radius.

### Determinism Considerations

- `AttackTarget` enum has `#[derive(Serialize, Deserialize)]` — serde adds a
  tag byte vs the `Option<u64>` it replaces. Replay hash already includes
  entity components.
- All sim math stays fixed-point: `(rx as i32 * 256 + 128, ry as i32 * 256 + 128)`.
- Per-unit command emission ordered by `selected_units.sort_unstable()`
  (already done). Replay-stable.
- No new global state, no tick-order changes.

## Architectural Decisions

- **Pattern followed**: one `Command` variant per distinct order kind
  (existing convention).
- **Pattern followed**: per-unit command emission in the order-resolution loop
  (existing convention).
- **Pattern deviation**: `Option<u64>` → `Option<AttackTarget>` widens an
  existing field. Rationale: the target *is* fundamentally one-of-two; a u64
  sentinel for "this is actually a packed cell" would lose type safety and
  force consumers to remember the convention.
- **Tech debt introduced**: `Command::Attack`, `Command::ForceAttack`, and the
  new `Command::ForceAttackCell` are three variants. The binary models this
  as a single Attack command with a force-fire flag and a target enum. We
  accept the current 3-variant shape; a future refactor can collapse them
  once cell-target is proven out.

## Alternatives Considered

- **Synthetic ground-target entity** — spawn a hidden non-rendered entity at
  the cell and force-attack against it. Rejected: invents a structure gamemd
  doesn't have, pollutes `EntityStore`, every consumer that iterates entities
  has to know to skip ground markers. Anti-pattern: "new pattern for no
  reason."
- **Collapse `Attack` / `ForceAttack` / `ForceAttackCell` into one variant
  with target enum + flag** — closer to gamemd's actual model, but bundles a
  refactor with this feature. Breaks replay back-compat (variant signatures
  change, not just appended). Defer until cell-target is proven.

## Out-of-Scope / Deferred

- **Disguise piercing** (Mirage Tank, Spy in disguise, chameleon spy) —
  blocked on disguise system not being implemented in Rust. Revisit when
  disguise lands.
- **Exact mouse SHP frame for action 0x33** — `UNKNOWN — needs RE`. Use
  existing attack cursor for now; cosmetic-only.
- **Force-fire-on-water for non-amphibious units** — existing invalid-cell
  cursor logic already handles this; no special force-fire handling needed.
