# Navy SEAL / Tanya C4 Building Demolition — Design

## Goal

Implement the player-issued C4 plant for SEAL / Tanya / Psi-Corp Trooper:
right-click an enemy structure, walk up, plant, detonate after `C4Delay`.
Internals are ours; observable output must match `gamemd.exe`.

## Source Research

- `ra2-rust-game-docs/NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` — full pipeline.
- Open Question 2 (cleanup of `+0x6df` on attacker death) **resolved live**
  via `BuildingClass::Update @ 0x0043fb20` decompile during this brainstorm:
  the marker is **never cleared** in the C4 path. The detonation timer is
  building-side, fires through attacker death and through Iron Curtain.
  This finding simplifies the design substantially.

## Architecture Context

The closest existing analog is **engineer capture**, which already implements
the full walk-up-and-mutate-target pattern end-to-end. Reusable pieces:

| Stage | Engineer capture site | C4 reuse |
|---|---|---|
| Click resolution | [app_context_order.rs:311-367](../../src/app_context_order.rs#L311-L367) | Add a parallel branch for C4 |
| Command | `Command::CaptureBuilding` ([command.rs:120-126](../../src/sim/command.rs#L120-L126)) | New `Command::PlantC4` |
| Per-attacker state | `capture_target: Option<u64>` ([game_entity.rs:199-201](../../src/sim/game_entity.rs#L199-L201)) | New `c4_plant: Option<C4PlantState>` |
| Walk-up handler | `tick_capture_orders` ([world_orders.rs:151-209](../../src/sim/world/world_orders.rs#L151-L209)) | New `tick_c4_plants` next to it |
| Tick slot | Phase 5 entry ([world/mod.rs:1174](../../src/sim/world/mod.rs#L1174)) | Same phase, immediately after |
| AoE damage primitive | `apply_aoe_damage` at [combat_aoe.rs:33](../../src/sim/combat/combat_aoe.rs#L33) | Reused at detonation |
| C4Warhead lookup | `RuleSet::c4_warhead_id()` ([ruleset.rs:1444](../../src/rules/ruleset.rs#L1444)) | Already wired |

The novel piece is the **building-side detonation timer** — a state lane on
the building, not the attacker. This was unambiguous in the binary
(`BuildingClass::Update` field reads `field_0x528 / 0x530 / 0x540 / 0x6df`).

## Impact Analysis

**Touched files (≈400 lines + tests):**

- `src/rules/object_type.rs` — three new fields: `c4: bool`, `can_c4: bool`
  (default `true` for buildings, `false` for non-buildings),
  `invisible_in_game: bool` (default `false`). Plus parsers.
- `src/rules/ruleset.rs` — `c4_delay_ticks: u32` parsed from
  `[CombatDamage] C4Delay=` (double, minutes → frames at 15 fps,
  default `0.03 × 60 × 15 = 27` ticks).
- `src/sim/command.rs` — new `Command::PlantC4 { attacker_id, target_building_id }`.
- `src/sim/components.rs` — new `C4PlantState`, new `PendingC4Detonation`.
- `src/sim/game_entity.rs` — two new optional fields: `c4_plant` (on attacker)
  and `pending_c4_detonation` (on building).
- `src/sim/world/world_commands.rs` — dispatch for `Command::PlantC4`
  (mirrors `CaptureBuilding` block at lines 861-915).
- `src/sim/world/world_orders.rs` — new `tick_c4_plants` (~120 lines).
- `src/sim/world/mod.rs` — single call-site insertion in `advance_tick`.
- `src/sim/world/world_hash.rs` — hash both new fields for lockstep determinism.
- `src/app_context_order.rs` — new branch to emit `Command::PlantC4`.
- `src/app_cursor.rs` — gate the `sabotage_cursor` branch on
  `c4 && can_c4 && !invisible_in_game`; also re-route to the new
  `Demolish` cursor variant.
- `src/app_types.rs` — new `CursorId::Demolish` + `CursorFeedbackKind::Demolish`.
- `src/render/cursor_atlas.rs` — load demolish frames from `mouse.shp`.

**Determinism risk areas:**

- New fields must be in `world_hash` and serde-serialized.
- Detonation timing keys off integer compare `tick - plant_start_tick >=
  c4_delay_ticks` — lockstep-safe.
- Iteration over `pending_c4_detonation`-bearing entities uses
  `keys_sorted()` like the rest of `tick_*` handlers.

**No risk areas:**

- Pathfinding: walk-up reuses `issue_move_command_with_layered` exactly like
  `CaptureBuilding`.
- Damage: routed through existing combat damage path, which already handles
  `is_invulnerable` (Iron Curtain) correctly.

## Chosen Approach

**Approach A — Engineer-Capture-Mirror with split state lanes.**

Two state lanes, on different entities:

```rust
// On the SEAL/Tanya entity (mirrors capture_target)
pub c4_plant: Option<C4PlantState>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct C4PlantState {
    pub target_building_id: u64,
}

// On the BUILDING entity (the actual detonation timer)
pub pending_c4_detonation: Option<PendingC4Detonation>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PendingC4Detonation {
    pub plant_start_tick: u64,
    /// Original attacker for kill-credit. May refer to a despawned entity by
    /// the time detonation fires — this is intentional gamemd parity (the
    /// binary's `field_0x540` is never re-validated and may be a dangling
    /// pointer at detonation time; we just attribute kill-credit to None
    /// when the entity is gone).
    pub attacker_id: u64,
}
```

Why split? Because the binary does it that way: the marker, start frame,
delay, and attacker ptr all live on the building (verified offsets
`+0x528/+0x530/+0x540/+0x6df`). Modeling them as building-side state
preserves these exact behaviors **for free**:

- Attacker death does not abort detonation.
- Iron Curtain on the building doesn't abort either — damage fires every
  tick once the timer elapses, gets nullified by IC, fires again next tick.
- A second SEAL arriving on the cell sees `pending_c4_detonation == Some`
  and idles (matches the `if (*+0x6df != 0) return` early-out in Mission_Enter).
- The marker is never cleared — when the building dies (which it will once
  damage gets through), the entity is despawned and the field goes with it.

Cleaner than fighting the binary's design and trying to put the timer on
the attacker.

## Tiny-Detail Ledger

Each item cites its source. Items marked **VERIFIED OQ2** were locked down
during this brainstorm via direct decompile of `BuildingClass::Update`.

### Issue-time gates

1. Click qualifies as plant only if attacker has `C4=yes`, target is Building,
   `target.CanC4=yes`, `target.InvisibleInGame=no`, target not iron-curtained,
   target not in fog/shroud. `[doc: §7]`
2. Force-fire (Ctrl-click) on non-CanC4 building falls through to normal
   weapon fire (Sapper, Mechanical warhead, 0% vs buildings = no-op except
   sound). **Already works through existing combat path; no C4-specific code.** `[doc: §11]`

### Plant timing

3. **C4Delay default = `0.03` minutes = `27` ticks @ 15 fps.** Parse as f64
   minutes from `[CombatDamage] C4Delay=`, convert to integer frame count
   at load time. `[doc: §5]` `[ini: rulesmd.ini [CombatDamage] C4Delay=]`
4. **Plant timer starts when SEAL stands on the target's center cell.**
   Not Chebyshev-≤-1 like engineer capture — it's "SEAL.cell ==
   target.cell". `[GHIDRA 0x005196a0: Mission_Enter]` Use the building's
   `position.rx/ry` directly; building footprints are multi-cell but the
   click resolves to the building's anchor cell.
5. **`plant_start_tick = current_tick`** captured on the first tick the
   SEAL is on the target cell. `[GHIDRA: piVar8[0x14a] = g_CurrentFrameCounter]`

### Detonation

6. **Damage value at detonation = building's CURRENT health (not the
   warhead's `Damage=` field).** This guarantees a one-shot kill regardless
   of `[Super]` Verses: `dmg = current_hp × verses[armor]`, with Verses=100%
   on all armors → exact-HP kill. **VERIFIED OQ2:**
   `[GHIDRA 0x0043fb20: iStack_28 = this->Health; vtable[+0x16c](&iStack_28, ...)]`
7. **Damage fires from BuildingClass::Update**, not from SEAL's tick. Once
   the timer elapses, it fires every building update tick until the building
   dies. **VERIFIED OQ2.**
8. **C4Warhead default = `Super`.** InfDeath=2 (gibbed). `[doc: §5, §10.D]`
9. **Marker (`+0x6df`) is NEVER CLEARED in the C4 path** — only in the
   BridgeRepairHut branch (Engineer destroying a bridge hut). Permanent until
   the building dies. **VERIFIED OQ2.**
10. **Cell-AoE on detonation:** the binary calls `Apply_area_damage` THREE
    times in the SEAL's tick (after the building has died from its own
    update tick): one with the SEAL as source, two with `source=NULL`. The
    2nd/3rd handle destructible-overlay chain reaction (sandbags/barrels).
    Per user choice: route the final damage through `apply_aoe_damage` at
    [combat_aoe.rs:33](../../src/sim/combat/combat_aoe.rs#L33) which iterates
    cell occupants. Overlay chain comes from whatever overlay handling that
    primitive already has (parity-relevant only when destructible overlays
    sit on building footprint cells — rare but possible on modder maps).
    `[doc: §1, §8]`

### Cancellation behavior (VERIFIED OQ2)

11. **SEAL killed mid-plant**: detonation **still fires** at scheduled tick.
    The building's `pending_c4_detonation.attacker_id` may refer to a
    despawned entity — we resolve this gracefully (kill-credit goes to None
    if the attacker is gone). Building dies. `[GHIDRA 0x0043fb20]`
12. **Target killed by another source mid-plant**: building entity despawned
    → `pending_c4_detonation` despawned with it. SEAL's `c4_plant` becomes
    dangling; on next tick the handler detects target gone, clears
    `c4_plant`, and the SEAL falls through to Guard/Idle.
13. **Iron Curtain on target after plant starts**: damage fires every tick
    while marker set. `is_invulnerable` nullifies. Marker stays. Once IC
    expires, next damage tick kills the building. **VERIFIED OQ2.**
14. **Two SEALs same target**: first SEAL claims plant by setting
    `target.pending_c4_detonation = Some(...)`. Second SEAL on arrival sees
    `Some(...)` already set, does NOT overwrite it, just hovers (no plant
    timer of its own). `[GHIDRA 0x005196a0: marker-set early-return branch]`
15. **Player issues Stop/Move on planting SEAL**: SEAL's `c4_plant` is
    cleared by the new command. Building's `pending_c4_detonation` is
    **NOT cleared** — it's owned by the building, not the SEAL, and the
    binary doesn't unwind it on attacker re-tasking either. **The plant
    detonates regardless.** This matches gamemd: once the SEAL is on the
    cell long enough to claim the plant, the damage is committed even if
    the player retasks the SEAL. (Confirm: in stock YR, ordering a planted
    SEAL away does NOT cancel the explosion.)

### Player-observable details

16. **Cursor 0x10 (DEMOLISH) is visually distinct from cursor 9 (ENTER)**.
    Per parity bar: player notices. Add `CursorId::Demolish` and load the
    correct mouse.shp frames. `[doc: §7, MouseClass_research.md]`
17. **Plant animation**: SEAL plays `[SealSequence] FireUp=164,6,6` (frames
    164-169 of own SHP). No CHARGE.SHP overlay. `[doc: §6.2]` `[ini: artmd.ini]`
18. **Audio cue**: `[SealPlaceBomb]` (Sounds=`icraatta`, Volume=60) plays at
    SEAL's position on the "fire" frame match (frame 0 since per-Type firing
    frame ints default 0). `[doc: §6.1, §10.F]`
19. **EVA voice**: `VoiceSpecialAttack=SealSpecialAttack` (global) plays at
    command-issue time on `[GHOST]`. `[doc: §10.F]`
20. **Post-detonation scatter**: `(tick >> 12 + 1) >> 1 & 7` → 1 of 8
    directions, deterministic from frame counter. SEAL walks 1 cell in
    that direction post-detonation. **Player-observable.** `[doc: §1]`
21. **SEAL survives the plant.** No engine self-destruct. SEAL transitions
    `Mission` from Enter (0x11) to Move (2) when its update tick finds
    `Look_up_building_in_cell()` no longer returns the original target
    (i.e., the building has died). `[doc: §9]` `[GHIDRA 0x005196a0]`

### Per-BuildingType data

22. **`CanC4` default = true.** Stock buildings opting out: `CAMISC01`
    (Oil Derrick), `CAMISC02` (Barrel), `CAMSC09`/`CAMSC10` (McBurger Kong).
    `[doc: §10.B]` `[ini: rulesmd.ini]`
23. **`InvisibleInGame` default = false.** No stock targetable building
    sets it. Defensive gate. `[doc: §4]`

### Active-in-YR

24. **C4 plant is always active in YR.** No SpecialFlags gate. `[doc: §13, §14 OQ9]`

### Consciously deferred

- **Selling-in-progress building exclusion** (mission==0x13 in
  `Mission_Enter`): research Open Q 6, semantics not nailed down. Single
  edge case; defer pending /re-investigate of building Mission state map.
  Add a TODO marker in code.
- **Per-Type firing-frame ints** (`FireUp/FireProne/SecondaryFire/SecondaryProne`
  on InfantryType): default 0 in stock SEAL/Tanya/PTROOP INI, so the
  audio cue fires on animation frame 0. We can hardcode frame 0 today; if
  modders set non-zero values we'll need to honor those, but no stock unit
  needs it.

## Design

### Components

```rust
// src/sim/components.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct C4PlantState {
    pub target_building_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct PendingC4Detonation {
    pub plant_start_tick: u64,
    pub attacker_id: u64,
}
```

Two new optional fields on `GameEntity` ([game_entity.rs](../../src/sim/game_entity.rs)):

```rust
pub c4_plant: Option<C4PlantState>,
pub pending_c4_detonation: Option<PendingC4Detonation>,
```

Both default to `None` in `GameEntity::new`.

### Object Type / Building Type fields

```rust
// src/rules/object_type.rs

pub struct ObjectType {
    // ...
    /// `C4=yes` on InfantryType. Gates the C4 plant mission path.
    pub c4: bool,
    /// `CanC4=yes` on BuildingType. Default `true` for buildings,
    /// `false` for non-building types.
    pub can_c4: bool,
    /// `InvisibleInGame=yes` on BuildingType. Excludes the building
    /// from C4 (and other interaction) cursors. Default `false`.
    pub invisible_in_game: bool,
    // ...
}
```

Parsers:

```rust
c4: section.get_bool("C4").unwrap_or(false),
can_c4: section.get_bool("CanC4").unwrap_or(matches!(kind, ObjectKind::Building)),
invisible_in_game: section.get_bool("InvisibleInGame").unwrap_or(false),
```

### RuleSet fields

```rust
// src/rules/ruleset.rs

pub struct RuleSet {
    // ...
    /// `[CombatDamage] C4Delay=`. Default 0.03 minutes = 27 ticks @ 15fps.
    pub c4_delay_ticks: u32,
    // ...
}
```

Parser converts `f64 minutes → u32 ticks` at load time using the
sim's tick rate constant.

### Command

```rust
// src/sim/command.rs

pub enum Command {
    // ...
    /// Order a C4-capable infantry to plant on an enemy building.
    /// Walks to the building's cell, claims the plant on arrival,
    /// and the building's update tick fires the detonation after C4Delay.
    PlantC4 {
        attacker_id: u64,
        target_building_id: u64,
    },
    // ...
}
```

### Command dispatch

`src/sim/world/world_commands.rs` — mirrors the `CaptureBuilding` block:

1. Validate ownership of attacker.
2. Validate attacker has `c4` flag.
3. Validate target is a building, not dying, has `can_c4`, not
   `invisible_in_game`, not iron-curtained, owned by an enemy.
4. Clear conflicting state on attacker (`attack_target = None`,
   `order_intent = None`, `dock_state = None`, `capture_target = None`).
5. Set `attacker.c4_plant = Some(C4PlantState { target_building_id })`.
6. Issue movement to target's `position.rx/ry` via
   `issue_move_command_with_layered`.

### Tick handler

New function in `src/sim/world/world_orders.rs`, called from `advance_tick`
**immediately after** `tick_capture_orders` ([world/mod.rs:1174](../../src/sim/world/mod.rs#L1174)):

```rust
pub(crate) fn tick_c4_plants(&mut self, rules: &RuleSet) -> bool {
    let mut destroyed_structure = false;

    // Phase 1 — walk-up: SEALs with c4_plant.
    let walkup_keys: Vec<u64> = /* keys_sorted with c4_plant.is_some() && !dying */;
    for seal_id in walkup_keys {
        let plant = self.entities.get(seal_id).and_then(|e| e.c4_plant);
        let Some(plant) = plant else { continue };

        // Target gone? Clear c4_plant.
        let Some(target) = self.entities.get(plant.target_building_id) else {
            self.entities.get_mut(seal_id).unwrap().c4_plant = None;
            continue;
        };
        if target.dying { /* same: clear and continue */ }

        // SEAL on target cell?
        let on_cell = /* compare seal.position cell vs target.position cell */;
        if !on_cell { continue; } // pathfinding handles walk-up

        // Already claimed (this SEAL or another)?
        if target.pending_c4_detonation.is_some() {
            // Second SEAL or self on subsequent tick — hover, no-op.
            continue;
        }

        // Claim the plant.
        let target_id = plant.target_building_id;
        let attacker_id = seal_id;
        let start_tick = self.tick;
        if let Some(b) = self.entities.get_mut(target_id) {
            b.pending_c4_detonation = Some(PendingC4Detonation {
                plant_start_tick: start_tick,
                attacker_id,
            });
        }
        // Queue the audio cue + animation here:
        //   sound_events.push(seal_place_bomb at seal.position)
        //   seal.animation = FireUp sequence (artmd [SealSequence])
    }

    // Phase 2 — detonation: buildings with pending_c4_detonation.
    let det_keys: Vec<u64> = /* keys_sorted with pending_c4_detonation.is_some() */;
    let c4_warhead_id = rules.c4_warhead_id();
    let delay = rules.c4_delay_ticks as u64;
    for building_id in det_keys {
        let pending = self.entities.get(building_id)
            .and_then(|e| e.pending_c4_detonation);
        let Some(pending) = pending else { continue };
        if self.tick - pending.plant_start_tick < delay { continue }

        // Timer elapsed — fire damage. Don't clear pending; if IC nullifies,
        // it'll fire again next tick (matches gamemd binary).
        let bld_pos = self.entities.get(building_id).map(|e| (e.position.rx, e.position.ry));
        let Some((rx, ry)) = bld_pos else { continue };
        let attacker_id = if self.entities.get(pending.attacker_id).is_some() {
            Some(pending.attacker_id)
        } else {
            None  // attacker despawned; kill-credit unattributed (matches gamemd dangling-ptr)
        };

        let result = combat::apply_aoe_damage(
            &mut self.entities,
            &mut self.occupancy,
            (rx, ry),
            c4_warhead_id,
            attacker_id,
            /* damage = current_hp of building */,
            rules,
            // ... other args from existing apply_aoe_damage signature
        );
        destroyed_structure |= result.structure_destroyed;
        // pending_c4_detonation NOT cleared on purpose.
    }

    destroyed_structure
}
```

(Exact signature of `apply_aoe_damage` to be matched at implementation
time; the function exists and handles cell iteration + warhead Verses.)

### Cursor

`src/app_types.rs`:

```rust
pub enum CursorId {
    // ...
    Demolish,  // mouse.shp frames for action 0x10
}

pub enum CursorFeedbackKind {
    // ...
    Demolish,
}
```

`src/app_cursor.rs:214-218` — change the SabotageCursor branch:

```rust
// Before:
if sel_obj.sabotage_cursor {
    if matches!(hover.kind, HoverTargetKind::EnemyStructure) {
        return CursorFeedbackKind::Enter;
    }
}

// After:
if sel_obj.c4 {
    if matches!(hover.kind, HoverTargetKind::EnemyStructure) {
        if hovered_obj.map_or(false, |o| o.can_c4 && !o.invisible_in_game) {
            // TODO: also check IC (target.invulnerability is_some).
            return CursorFeedbackKind::Demolish;
        }
    }
}
```

The `sabotage_cursor` flag is no longer the trigger — `c4` is. SabotageCursor
remains as a parsed flag (Sapper/FakeC4 weapons set it for purely visual
modder use), but it doesn't drive cursor logic anymore. Document in the
field comment.

`src/app_cursor.rs:457` — extend `cursor_id_for_feedback`:

```rust
CursorFeedbackKind::Demolish => Some(CursorId::Demolish),
```

`src/render/cursor_atlas.rs` — load the demolish frames from `mouse.shp`
under the same pattern as Enter / EngineerRepair.

### Click-to-Command

`src/app_context_order.rs` — new branch BEFORE the engineer branch (line 311),
since C4 supersedes capture if both `c4` and `engineer` somehow held (not
stock-possible, but safe ordering):

```rust
// C4 plant: SEAL/Tanya clicking a CanC4 enemy structure.
if !force_fire {
    let c4_target = hover.as_ref().and_then(|target| {
        if !matches!(target.kind, HoverTargetKind::EnemyStructure) { return None; }
        let rules = state.rules.as_ref()?;
        let building = sim.entities.get(target.stable_id)?;
        let obj = rules.object(sim.interner.resolve(building.type_ref))?;
        if !obj.can_c4 || obj.invisible_in_game { return None; }
        Some(target.stable_id)
    });
    if let Some(building_id) = c4_target {
        let c4_units: Vec<u64> = selected_units.iter().copied()
            .filter(|&sid| sim.entities.get(sid).is_some_and(|e| {
                e.category == EntityCategory::Infantry &&
                state.rules.as_ref().and_then(|r| r.object(sim.interner.resolve(e.type_ref)))
                    .map_or(false, |o| o.c4)
            }))
            .collect();
        if !c4_units.is_empty() {
            for attacker_id in c4_units {
                queued.push(CommandEnvelope::new(
                    owner_id, execute_tick,
                    Command::PlantC4 { attacker_id, target_building_id: building_id }));
            }
            for cmd in queued { sim.pending_commands.push(cmd); }
            emit_order_voice(state, "VoiceSpecialAttack");  // §10.F EVA cue
            return true;
        }
    }
}
```

### Determinism

- Both new fields included in `world_hash` ([world_hash.rs](../../src/sim/world/world_hash.rs)).
- Both fields `serde::Serialize + Deserialize` for replay/save/load.
- Tick handler uses `keys_sorted()` for deterministic iteration order.
- All math integer (no float in sim path).

### Testing Strategy

Unit tests in a new `world_orders_c4_tests.rs` (or extend
`world_tests.rs`):

1. **Happy path**: SEAL plants, 27 ticks elapse, building dies. Assert
   damage attribution to SEAL, building despawned, SEAL's `c4_plant`
   cleared on next tick after target despawn.
2. **Attacker dies mid-plant** (parity-critical, OQ2): plant SEAL,
   advance 10 ticks, kill SEAL. Advance 17 more ticks. Assert building
   dies anyway, attribution = None.
3. **IC during plant** (parity-critical): plant SEAL, IC the building
   on tick 25 (2 ticks before C4Delay). Advance through IC duration.
   Assert building survives until IC expires, then dies on next tick.
4. **Two SEALs same target**: SEAL A arrives tick 10, SEAL B arrives
   tick 15. Assert only A's plant timer is registered; B hovers.
5. **Target dies before timer**: plant SEAL, kill building via different
   weapon at tick 15. Assert SEAL's `c4_plant` clears, no detonation.
6. **`CanC4=no` building rejected at issue time**: assert `Command::PlantC4`
   on a CAMISC01 returns false (rejected).
7. **`InvisibleInGame=yes` rejected at issue time**.
8. **Non-C4 unit clicked on enemy building**: assert no `Command::PlantC4`
   issued (engineer branch / normal attack instead).
9. **Player Stop/Move during plant** (after SEAL is on cell with timer
   running): assert SEAL's `c4_plant` clears, but
   `target.pending_c4_detonation` stays set, detonation still fires.
10. **Determinism**: hash before / after / replay path with C4 plant in
    play matches across two simulator instances.

## Architectural Decisions

- **Pattern followed**: engineer-capture's dedicated-component-field +
  Phase-5-pre-combat-tick-handler. Same shape, parallel code.
- **Pattern deviated**: split state across two entities (attacker
  has `c4_plant`, target has `pending_c4_detonation`). Engineer capture
  uses a single field on the attacker. **Reason**: gamemd's binary keeps
  the timer state on the building (`field_0x528/0x530/0x540/0x6df`) and
  this is load-bearing — it's why attacker death doesn't abort and why
  IC produces the correct behavior for free.
- **No new abstraction**: did not generalize to a shared `EnterTarget` enum
  (premature with N=2). Per CLAUDE.md "Three similar lines is better than
  a premature abstraction."
- **Tech debt**: `BridgeWarheads` struct in `src/rules/bridge_warheads.rs`
  is misnamed — it's now used for any C4 detonation, not just bridges. Do
  NOT rename in this work (would touch many call sites for cosmetic
  reasons); add a single doc-comment update noting "also consumed by
  player-issued C4 plant on buildings."

## Alternatives Considered

- **Approach B — single `OrderIntent::PlantC4` variant on attacker only**:
  rejected because keeping the timer state on the attacker (a) diverges
  from the engineer-capture precedent and (b) makes the cancellation
  semantics harder to reproduce — attacker death would naturally drop
  the timer, but gamemd does NOT drop the timer on attacker death.
  Forcing parity through this shape requires hacks.
- **Approach C — generalized `EnterTarget` enum**: premature abstraction.
- **Single damage instance on building (no cell iteration)**: rejected
  per user choice. Full `apply_aoe_damage` cell iteration matches gamemd's
  three-call chain at low extra cost (existing primitive).
- **Clear marker on attacker death**: rejected per user choice + verified
  binary behavior. The marker is never cleared in gamemd's C4 path.

## Known parity drift / open follow-ups

- **Selling-in-progress building exclusion** (Mission==0x13 in gamemd): not
  modeled. Add a TODO. Edge-case trigger only when player tries to C4 a
  selling building. Defer.
- **Modder INI: per-Type firing-frame ints**: hardcoded to frame 0 today.
  Stock units don't override; modders that do would see the wrong frame
  for the audio cue. Defer; document in code comment.
- **Animation sequence**: needs to play SEAL's `FireUp=164,6,6` sequence
  during the 27-tick plant. Wiring depends on existing infantry-animation
  plumbing — confirm at implementation time that the `Animation` component
  can be driven externally to play a specific sequence range. If yes, add
  to Phase 1 of `tick_c4_plants`.
