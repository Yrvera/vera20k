# Diagnosis — SEAL/Tanya C4 on Civilian Bridge Hut (CABHUT) does nothing

**Date**: 2026-05-24
**Bug**: Right-clicking CABHUT with SEAL/Tanya selected produces no observable
action (no cursor change to Demolish, no movement, no plant, no detonation).
**Reference (refuted hypothesis)**: 2026-05-12 gamemd-side investigation
established there is NO upstream `Immune=` gate in `gamemd.exe` for C4-on-
CABHUT. Bug lives in the Rust port (per `project_c4_bridge_hut_followup.md`).

---

## Verdict

**Root cause: most likely no CABHUT entity exists for the cursor/order
machinery to target.** The map's `[Structures]` section is the only source of
`EntityCategory::Structure` entities in the Rust port
(`src/map/entities.rs:73-74, 188-237`). Stock retail RA2 bridge huts on
WAE-edited and retail maps are typically NOT listed in `[Structures]` —
`gamemd.exe` instantiates them automatically as a side effect of bridge-tile
placement. The Rust port has no equivalent auto-spawn: a `Grep` for
`spawn_building`/`bridge_repair_hut`/`CABHUT` under `src/map/` and
`src/sim/world/world_spawn.rs` returns no automatic instantiation path.
Without a `GameEntity` at the hut's cell, `hover_target_at_point`
(`src/app_entity_pick.rs:87-165`) returns `None` for clicks on the hut sprite
— the click then falls through every cursor and order branch that requires
an entity, including the C4 dispatch at `src/app_context_order.rs:319-375`.
The click ends up as a plain Move on the underlying terrain cell (which is a
blocked bridge cell, so the move is also a no-op for the user). Net effect:
nothing visible happens.

This explains **all three observable signs of the bug**:

1. No Demolish cursor (cursor branch at `src/app_cursor.rs:250-261` requires
   `hover.kind == EnemyStructure` — needs an entity to classify)
2. No `PlantC4` command issued (dispatch branch requires `hover.stable_id`)
3. No movement to the hut (the right-click resolves to a click on the
   underlying bridge cell, which the move pathfinder rejects/redirects to a
   nearest land cell — visible as "nothing happens" if the SEAL is already
   close to the hut)

The 2026-05-12 gamemd finding ("`Immune` is not the gate") is consistent —
gamemd HAS the hut entity at the cell, so its cursor + dispatch fire
correctly. The Rust port is missing the entity upstream.

### Which of the user's hypotheses won

**None of the three.** All three assume a CABHUT entity exists and the
problem is in the cursor/order/path pipeline that consumes it. The real
problem is one layer earlier — no entity is ever spawned.

---

## How to falsify or confirm in 60 seconds

The user already added a diag log at `src/app_cursor.rs:209-221`
(`c4-cursor: best_id={} sel_type={:?} obj_lookup={}`) AND an inner log at
`src/app_cursor.rs:236-249` (`c4-cursor: sel.c4={} hover.kind={:?}
hover_type={:?} can_c4={:?} invis={:?} invuln={}`).

- **If the inner log NEVER fires when hovering CABHUT**, the outer log only
  shows `obj_lookup=true` (the selected SEAL), and `hover_type=None` — that
  is the confirmation: `hover_target_at_point` returned `None`, so no
  hovered entity exists. → Root cause = no CABHUT entity. (This diagnosis.)
- **If the inner log fires with `hover_type=Some("CABHUT")` and
  `can_c4=Some(true)` and `invuln=false`**, the cursor SHOULD show Demolish
  and the dispatch SHOULD fire. If it still doesn't, the cause is downstream
  (one of the user's three hypotheses) and this diagnosis is wrong.

Run the failing case once and read `c4-cursor:` lines from stderr to
discriminate.

---

## Full dispatch chain (traced for the case where a CABHUT entity DOES exist)

This is for completeness, since the user requested the dispatch chain and to
document what would happen if the upstream spawn issue were fixed.

### 1. Mouse hover → `hover_target_at_point`

`src/app_entity_pick.rs:87-165`

Iterates every entity. For `EntityCategory::Structure`, calls
`click_hits_foundation` (`:353-370`) — strict cell-containment test against
the foundation rectangle `(rx..rx+fw, ry..ry+fh)`. CABHUT foundation is `1x1`
(`ini/rulesmd.ini:16336+` has no `Foundation=` line → default; resolved by
`ObjectType` defaulting `foundation` to `1x1`).

Returns `HoverTargetKind`:
- `EnemyStructure` (`:154-156`) when the entity is a structure and
  `!fog.is_friendly(local_owner, owner_str)`.

CABHUT owner is `Neutral` on stock maps. `is_friendly("Americans", "Neutral")`
(`src/map/houses.rs:89-101`) returns false unless the map's
`[Americans] Allies=` list contains `Neutral` (never the case in stock maps).
→ `EnemyStructure`, with `stable_id = CABHUT's entity id`.

### 2. Cursor — `app_cursor.rs:222-261`

C4 cursor branch (`:250-261`):

```
if sel_obj.c4
    && matches!(hover.kind, HoverTargetKind::EnemyStructure)
    && hovered_obj.map_or(false, |o| o.can_c4 && !o.invisible_in_game)
    && !is_invulnerable(...)
{
    return CursorFeedbackKind::Demolish;
}
```

For CABHUT:
- `sel_obj.c4` = `true` (GHOST/SEAL/TANY have `C4=yes`)
- `hover.kind == EnemyStructure` = `true`
- `hovered_obj.can_c4` = `true` — **default for buildings** when not
  overridden (`src/rules/object_type.rs:1085-1087`,
  `can_c4_defaults_to_true_for_buildings`). CABHUT does NOT set `CanC4=no` in
  `ini/rulesmd.ini:16336-16352`, so it inherits `true`.
- `hovered_obj.invisible_in_game` = `false` (CABHUT has no `InvisibleInGame=`)
- `is_invulnerable` = `false` (CABHUT has no active iron-curtain)

→ Cursor IS `Demolish`. (Would be, if the entity exists.)

### 3. Right-click → `try_queue_context_order_at_screen_point`

`src/app_context_order.rs:35-208`

- Selection is one or more SEAL/Tanya, no structures → `structure_owner = None`.
  `else` branch at `:252` enters.
- Garrison entry at `:257-314`: CABHUT has no `CanBeOccupied=yes` → branch
  rejects.
- **C4 plant branch at `:319-375`**:

```
let c4_target = hover.as_ref().and_then(|target| {
    if !matches!(target.kind, HoverTargetKind::EnemyStructure) { return None; }
    let obj = rules.object(...)?;
    if !obj.can_c4 || obj.invisible_in_game { return None; }
    if is_invulnerable(...) { return None; }
    Some(target.stable_id)
});
```

For CABHUT, the same conditions as the cursor pass. `c4_target = Some(CABHUT
entity id)`. Then filters selected units for `C4=yes` infantry, queues
`Command::PlantC4 { attacker_id, target_building_id }` for each, returns
`true` (click consumed).

### 4. Command handler — `src/sim/world/world_commands.rs:947-1049`

Validates:
- `entity_owned_by_id(command_owner, attacker_id)` (`:952-954`) — passes
- attacker is not deployed (`:955-961`) — passes for standing SEAL
- attacker has `C4=yes` (`:962-969`) — passes
- target is a `Structure`, not dying, has `can_c4`, not `invisible_in_game`,
  not iron-curtained (`:972-994`) — all pass for CABHUT
- target owner is enemy of `command_owner` via `are_houses_friendly`
  (`:995-1002`) — passes (player vs Neutral)

Then sets `attacker.c4_plant = Some(C4PlantState { target_building_id })`
(`:1004-1012`) and calls `issue_move_command_with_layered` toward
`(trx, try_)` = CABHUT's NW cell (`:1013-1047`).

### 5. Pathfinding — `src/sim/movement/movement_commands.rs:155-217`

`resolve_requested_move_goal` (`src/sim/movement/movement_path.rs:120-142`)
sees the goal cell is in `merged_entity_blocks` (CABHUT cell is in
`ground_blocked` per `bump_crush.rs:141-169` — buildings always block their
foundation) and redirects to nearest walkable adjacent cell. A* plans a path
to that adjacent cell. SEAL walks.

### 6. Per-tick: `tick_c4_plants` — `src/sim/world/world_orders.rs:423-606`

**Phase 1 (walk-up + claim, `:428-525`)**:
- Each tick scans entities with `c4_plant`.
- Reads attacker's current cell.
- `target_footprint = building_entry_target_footprint` (`:608-622`) = list of
  CABHUT's foundation cells (1 cell for CABHUT).
- If attacker cell IS in footprint → claim plant.
- If attacker is adjacent (Chebyshev-1) AND has no active `movement_target`
  → call `issue_building_enter_target_cell` (`:642-671`) which issues a
  bypass-grid 1-cell move into the CABHUT cell.
- Else continue walking.

`issue_building_enter_target_cell` calls `issue_direct_move` and sets
`bypass_grid = true` on the resulting MovementTarget so the SEAL can step
into the otherwise-blocked CABHUT cell.

### 7. Phase 1 claim — `:492-525`

When attacker cell == CABHUT cell:
- Sets `building.pending_c4_detonation = Some(PendingC4Detonation {...})`.
- Switches SEAL animation to `Attack` sequence.
- Emits `SimSoundEvent::C4Planted` ([SealPlaceBomb] sound).

### 8. Phase 2 detonation — `:530-605` calling `apply_c4_damage_to_building`

`apply_c4_damage_to_building` (`:752-793`) detects
`obj.bridge_repair_hut == true` for CABHUT and **reroutes** to
`bridge_orchestrator::dispatch_bridge_collapse_from_hut`. The hut survives
(no HP damage applied), `consumed_pending_marker = true` clears
`pending_c4_detonation`, and the bridge segment collapses.

**Conclusion of trace**: If a CABHUT `GameEntity` exists in `sim.entities`,
the entire C4-on-CABHUT chain functions end-to-end. The Rust port has all the
necessary code: cursor gating, dispatch gating, target validation, walk-up,
enter-cell bypass, claim, delayed detonation, and bridge-collapse rerouting.
The behavior **fails because no CABHUT entity reaches the entity store**.

---

## Why the entity is missing

`src/map/entities.rs` parses entity placements only from these INI sections
(`:60-78`): `[Units]`, `[Infantry]`, `[Structures]`, `[Aircraft]`. There is
no code that synthesizes CABHUTs from bridge tile placements.

`gamemd.exe` spawns CABHUTs from bridge tiles automatically: the bridge tile
placement code reads bridge-tile metadata (BridgeSet/WoodBridgeSet) and
places a `BridgeRepairHut` building at the appropriate endpoint cell. This
is a side effect of map load, not a `[Structures]` line in the .map file.

The Rust port relies entirely on the map's `[Structures]` section. For stock
maps that DON'T list CABHUTs in `[Structures]`, the port has no CABHUT
entities → no hover target → no cursor → no dispatch → no C4.

Two possible mitigations (each requires changes to multiple subsystems —
not proposed here):

- **Option A**: Implement automatic CABHUT spawn during map load by scanning
  `[OverlayPack]` / bridge-tile metadata for bridge endpoints and inserting a
  `Structure` entity at the appropriate cell. Subsystems touched: `src/map/`
  (tile/overlay parser), `src/sim/world/world_spawn.rs` (entity insertion),
  `src/map/entities.rs` (perhaps as a synthetic post-pass on parsed
  entities).
- **Option B**: Augment maps known to have CABHUTs with explicit
  `[Structures]` lines. Workable for test maps, not for retail.

Option A is the parity-faithful path per CLAUDE.md "model the gamemd
primitive, not approximate it."

---

## Falsification & follow-ups

- **If the diag log shows `hover_type=Some("CABHUT")`** (an entity does
  exist for the user's test map), then the actual cause is elsewhere. The
  next-most-likely candidates would be:
  - Hover hit-test foundation mismatch — CABHUT visual sprite extends
    beyond its 1×1 foundation; the user might be clicking the visible roof
    in an adjacent cell. (`src/app_entity_pick.rs:353-370` is strict
    cell-containment.) **Test**: click the SW corner of the hut sprite where
    the foundation cell actually is.
  - `select_best_for_action` (`src/app_cursor.rs:464-510`) returning a
    different selected entity than the SEAL, with `sel_obj.c4 = false`.
    Unlikely if only SEAL is selected.

- **The user's three named hypotheses are FALSE under current code**:
  - **#1 Footprint impassability**: dispatch handler does NOT validate
    pathability to the target. It sets `c4_plant` and issues the move
    regardless; the SEAL would walk to an adjacent cell and use bypass-grid
    to step in. No impassability check rejects the C4 dispatch.
  - **#2 Cell→building lookup**: hover lookup iterates `sim.entities` (not a
    separate overlay map). CABHUT lives in the same store as every other
    structure. The lookup ISN'T broken; it returns None because there's no
    entity to find.
  - **#3 Pathfinding rejects routes ending on CABHUT**:
    `resolve_requested_move_goal` redirects blocked goals to the nearest
    walkable cell within radius 10 (`src/sim/movement/movement_path.rs:120-142,
    78-117`). It does NOT reject; it falls back. Then `tick_c4_plants`
    Phase 1 takes over with bypass-grid arrival.

- **Open question this diagnosis can't answer without an in-game test**:
  Does the user's test map have a CABHUT in `[Structures]`, or is it relying
  on bridge-tile auto-spawn? Need to inspect the actual `.map` file used in
  the failing reproduction. If the map DOES list the CABHUT, then this
  diagnosis is wrong and the bug is downstream (one of the three named
  hypotheses, or an unconsidered fourth).

---

## Subsystems requiring changes (for a parity-faithful fix)

1. **`src/map/` (or new module)**: Logic to identify bridge endpoints from
   parsed tile/overlay data and emit synthetic CABHUT `MapEntity` entries.
2. **`src/map/entities.rs`**: Accept the synthesized entries alongside
   `[Structures]`-parsed ones.
3. **`src/sim/world/world_spawn.rs`**: Owner defaults — synthetic CABHUTs
   should be assigned `Neutral` owner (matching gamemd) when no map override
   exists.
4. **Tests**: Add a `world_orders_c4_tests.rs` case for a `BridgeRepairHut=yes`
   target verifying the bridge-collapse rerouting in
   `apply_c4_damage_to_building` (lines `:760-793`) actually fires — this
   currently has no integration test coverage despite the code path existing.

---

## Cited Rust file:line evidence

- Map entity parser (only [Structures] is read): `src/map/entities.rs:60-78,
  188-237`
- Hover entity classification: `src/app_entity_pick.rs:87-165`
- Hover foundation hit-test (strict cell containment): `src/app_entity_pick.rs:353-370`
- `HoverTargetKind` enum: `src/app_types.rs:228-234`
- `is_friendly` (alliance map driven, no special Neutral handling):
  `src/sim/vision/mod.rs:276-278`, `src/map/houses.rs:89-101`
- `can_c4` default = true for buildings: `src/rules/object_type.rs:1085-1087,
  1751-1756`
- C4 cursor branch: `src/app_cursor.rs:250-261`
- C4 dispatch branch: `src/app_context_order.rs:319-375`
- C4 command handler: `src/sim/world/world_commands.rs:947-1049`
- Move command + goal redirect: `src/sim/movement/movement_commands.rs:155-217`,
  `src/sim/movement/movement_path.rs:120-142`
- Structure cells always block: `src/sim/movement/bump_crush.rs:141-169`
- C4 walk-up, adjacency, claim: `src/sim/world/world_orders.rs:423-525`
- Enter blocked footprint via bypass-grid:
  `src/sim/world/world_orders.rs:642-671`
- C4 damage with CABHUT bridge-collapse rerouting:
  `src/sim/world/world_orders.rs:752-793`
- Bridge collapse dispatcher (called from CABHUT path):
  `src/sim/world/bridge_orchestrator.rs:140-217`
- Existing diag logs added by user:
  `src/app_cursor.rs:209-221` (outer) and `src/app_cursor.rs:236-249` (inner)
- CABHUT INI definition (no `CanC4=` override, no `CanBeOccupied`,
  no `Capturable`): `ini/rulesmd.ini:16336-16352`
