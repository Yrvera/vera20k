# Bridge deck height — diagnosis

**Symptom (ground truth, user, in game):** a tank driving over an **intact** bridge falls to the
bottom, or through it. Normal driving, no collapse.

**Evidence weighting used here.** The `gamemd` lane read live Ghidra disassembly of the shipped
Yuri's Revenge binary and outranks everything else; its claims are marked VERIFIED with an address.
The three VERA lanes read current Rust; their `file:line` claims that are load-bearing below were
re-read directly in this worktree and are marked VERIFIED-in-repo. The `reference` lane read an
external reconstruction of the predecessor engine — it is a **shape guide only** and settles
nothing about YR; agreement with it is not evidence. All five lane reports were present and read.

---

## 1. VERDICT

VERA computes a unit's height on a bridge from the wrong inputs. Yuri's Revenge derives it from
one thing — the unit's own "I am on a bridge" flag — and adds a fixed four-level offset to the
ground under the unit's own cell, redoing that arithmetic on every movement step. VERA instead
looks up a **stored per-cell deck height**, reads it through an **`Option` that a separate
"walkable" permission bit can empty**, and selects between deck and ground using the **A\*
path-planning layer** rather than the unit's flag. Neither of those two extra inputs exists in
gamemd. When either one says "ground" — a planner layer that came back `Ground`, or a cell whose
walkable bit is clear — the mover's `position.z` is silently set to the riverbed while `on_bridge`
and `bridge_occupancy` stay set, and the renderer faithfully draws the tank sixty pixels down,
under the deck. Both halves of the fallback live on two lines: `movement_bridge.rs:206` and
`core.rs:1783`. The fix is a **removal**: stop the mover's Z from consulting either the stored
optional deck value or the path layer, and compute it the way gamemd does — from the unit's own
`on_bridge` flag over its own cell's ground level.

This is a single root cause, not a set. It is stated with confidence because the gamemd side is
VERIFIED disassembly, not inference, and because the correct model **already exists in this
codebase** in the same module tree — `runtime_current_effective_height`,
`src/sim/movement/movement_occupancy.rs:95-111`, computes `cell.signed_level() + if on_bridge { 4 }
else { 0 }`. Two contradictory height models are live at once; one of them is right.

What is *not* settled: **which** of the two invented inputs fires on the map the user played. See
§4 — that changes which of C1/C2 leads, not the fix, because the fix deletes both.

---

## 2. THE TWO READINGS

**"Falls to the bottom" and "through it" are one failure, not two.** They are simultaneous
consequences of a single wrong `position.z`, and the render lane proved this with arithmetic
rather than assertion.

- **Placement.** For a ground vehicle the *only* vertical term in the entire render path is
  `position.z` — `screen_position` (`src/render/locomotor_visual.rs:178-185`) →
  `lepton_to_screen` (`src/util/lepton.rs:352-360`), and `height_lift_px` contributes exactly zero
  for `HeightSource::Ground` (`locomotor_visual.rs:80-96, 121-140`). Four levels = 416 leptons = 60
  px. A tank carrying riverbed `z` is drawn 60 px low. **VERIFIED-in-repo (render lane §1).**
- **Occlusion.** The deck body is the only sprite pass that writes depth
  (`src/app/presentation/render/draw_passes.rs:111-118`, `zdepth_pipeline`, write ON, `Less`).
  With the correct deck `z`, `compute_sprite_depth_params`' `+ z*15` term
  (`src/app/presentation/instances/helpers.rs:362-372`) cancels the 60 px lift and the tank passes
  the `LessEqual` test over every deck pixel at or above its own sprite bottom minus 2 px. With
  `z` left at riverbed the `+z*15` bonus is gone, `z_bias` is `0` instead of `4e-4`, the deck still
  carries its `−2/wh` — and the tank **fails `LessEqual` wherever the deck art covers it**.
  **VERIFIED-in-repo (render lane §4, case B).**

So the player sees the tank at water level, partly erased by the deck it should be standing on.
One `z`, both sentences.

**The "right height, wrong draw order" reading is REFUTED.** Suppose `position.z` were correct and
only `bridge_occupancy` were missing, mis-routing the unit into the under-bridge stream (step 4)
with `apply_bridge_depth_bias`'s `+7 × 0.00002 = +0.00014`
(`helpers.rs:477-494`, `src/rules/object_type.rs:1429`). On a world height of a few thousand pixels
that bias is worth **0.5–1.2 px** of depth. The tank still draws over the deck. Steps 4 and 5 both
run before railings and neither writes depth, so the worst outcome is a subtle sorting wobble.
Mis-routing alone **cannot** bury a tank. **REFUTED (render lane §4).**

Corollary worth keeping: `entity.bridge_occupancy.deck_level` is read by **zero** draw paths. The
renderer has no second channel for deck height. If `position.z` is wrong, nothing downstream can
rescue it. **VERIFIED-in-repo (render lane §2).**

---

## 3. THE MECHANISM GAP

| | Yuri's Revenge (VERIFIED, Ghidra) | VERA (VERIFIED-in-repo) |
|---|---|---|
| Where deck height comes from | **Computed**: `Location.Z = GroundHeight(own X,Y) + heightArg + (OnBridge ? DeckOffset : 0)` — `FootClass::Set_Height_On_Bridge` @ `0x005F5FA0`, store at `0x005F5FFF` | **Stored per cell**: `PathCell.bridge_deck_level`, written at load as `cell.level.saturating_add(4)` — `src/map/resolved_terrain.rs:2415` |
| The deck offset | One process-wide constant `g_nFootOnBridgeDeckOffsetLeptons` @ `[0x00AC13BC]`, initialized once as `round(4 × LevelStep)` — `LEA ECX,[EAX*4]` @ `0x005F3866`, store @ `0x005F3880`. Exactly one writer, four readers. | Recomputed per cell at load, and re-derivable a second, different way in `movement_occupancy.rs:105` |
| What decides deck vs ground | **The object's own `OnBridge` byte at `+0x8C`, and nothing else.** Single `ADD EDI,[0x00AC13BC]` gated on it @ `0x005F5FB5`. No cell lookup on the height path. | **Two other things**: the destination cell's `bridge_walkable` bit (`core.rs:1762`) *and* the A\* path layer `next_layer` (`movement_bridge.rs:256`). The unit's own `on_bridge` is **not consulted** on the Z path at all. |
| Can the deck value be absent? | **No.** `CellClass::GetGroundHeight` @ `0x00578080` → `ComputeGroundHeightAtCoord` @ `0x0047B3A0` reads only `Cell+0x11B` (level) and `Cell+0x11C` (slope). It never reads a bridge flag and never reads a deck field. | **Yes.** `bridge_deck_level_if_any()` = `self.bridge_walkable.then_some(self.bridge_deck_level)` — `src/sim/pathfinding/core.rs:1761-1763`. `None` is then `unwrap_or`'d to **ground level** at `core.rs:1783` and `movement_bridge.rs:206`. |
| The walkable/permission bit | `Cell+0x140 & 0x200` gates `CheckBridgeTraversal` @ `0x004D9C60` (can-enter only). **It appears in no height computation anywhere in the binary.** | `bridge_walkable` gates the height. Same bit family, wired into the wrong question. |
| When Z is written | **Every movement step.** `SetHeight(0)` is vtable slot `+0x1CC`; `Process_Drive_Track` calls it on the cell-crossing arm (`0x004B208A`) **and** on the same-cell sub-step arm (`0x004B20BD`); walk likewise at `0x0075C1A1` / `0x0075C21C`. | **Only at cell boundaries**, inside `resolve_cell_transition_bridge_state` (`movement_bridge.rs:229-261`), reached from `movement_step.rs:2126` and `movement_tick.rs:2285`. |
| Flag vs height ownership | **Split.** The cell-flag block (`0x004B2561-0x004B259B`) sets `OnBridge` and *nothing else*; `Set_Height_On_Bridge` owns Z. | **Merged.** `compute_bridge_transition` returns `deck_level` inside the same value the flag decision produces (`movement_bridge.rs:204-208`), so one bad flag corrupts both at once. |
| Inverse invariant | `ObjectClass::GetHeight` @ `0x005F5F30`: `Z − GroundHeight(XY) − (OnBridge ? DeckOffset : 0)`. Exact algebraic inverse — the invariant is engine-wide. | No such invariant holds; nothing enforces `z == ground + 4` while `on_bridge` is true. |

**Stated plainly, as asked:** VERA invented a per-cell optional deck value where gamemd computes a
constant offset. The consequence is a state gamemd's representation cannot express — *marked
on-bridge, standing at ground level* — and VERA reaches it silently, through an `unwrap_or`, with
`bridge_occupancy.deck_level` poisoned to the same wrong value so the repo's own BRIDGE_DIAG check
(`movement_tick.rs:2588-2602`, `z + 2 < deck_level`) cannot fire.

**Provenance note.** The `4 ×` relation is VERIFIED from the literal `LEA ECX,[EAX*4]`. The
numerals `LevelStep = 104` / `DeckOffset = 416` are **UNCHECKED at runtime** — `[0x00AC13C8]` is
initialized from a float chain at `0x005F37C0` the gamemd lane could not read statically. VERA's
`BRIDGE_DECK_HEIGHT_LEPTONS = 416` (`src/sim/map/bridge_topology.rs:61`) is consistent with the
verified structure but is not independently proven.

---

## 4. RANKED CAUSES

Ranked by how well each explains *a tank falling through an intact span in ordinary driving*.

### C1 — Mid-span Z is keyed off the A\* path layer, not the unit's `on_bridge` flag
`src/sim/movement/movement_bridge.rs:255-259` — `BridgeTransition::NoChange => position.z =
dst_cell.effective_cell_z_for_layer(next_layer)`, with `next_layer = target.layer_at(...)`
(`movement_step.rs:1852`, `movement_tick.rs:2283`).

- **For.** `NoChange` fires on **every deck-to-deck cell**, i.e. the whole span except the two ends,
  and it *rewrites* Z each time rather than carrying the Enter value. `NoChange` also maps to
  `BridgeStateUpdate::Unchanged`, which leaves `on_bridge` and `bridge_occupancy` untouched
  (`movement_bridge.rs:331-334`) — producing exactly the marked-on-bridge-at-ground-level state.
  gamemd's equivalent consults no planner artifact whatsoever (`0x005F5FB5`, VERIFIED). Four
  production routes make `next_layer` come back `Ground`: `layer_at`'s
  `unwrap_or(MovementLayer::Ground)` past the end of `path_layers` (`components.rs:549-552`); the
  flat-path fallback labelling an entire path `Ground` (`movement_path.rs:544-546`); the direct
  two-node move target hardcoding the current layer (`movement_commands.rs:276-279`); and the
  layered A\* itself, whose closed-list layer selection reads `bridge_walkable`
  (`is_at_bridge_level`, `core.rs:464-465`). All VERIFIED-in-repo.
- **Against.** For a `Drive` locomotor on a stock map the layered branch is taken
  (`supports_layered_bridge_pathing`, `movement_path.rs:83-95`) and the layered A\* *should* carry
  `Bridge` across the span: I traced Bay of Pigs by hand — ramp level 5 → deck level 1,
  `is_at_bridge_level(5, deck)` true, `compute_neighbor_height` case 3 returns `bridge_deck_level`
  = 5, then case 2 holds it at 5 for every subsequent deck cell (`core.rs:471-508`,
  `core.rs:1129`). So on that map, on that route, C1 should not fire. The flat fallback fires
  unconditionally for Hover / Ship / Amphibious, and `movement_commands.rs:276` is a
  building-footprint path, not an ordinary right-click.
- **Cheapest kill/confirm.** One log line at `movement_bridge.rs:256` printing
  `(next_layer, dst_cell.bridge_walkable, dst_cell.bridge_deck_level, position.z, on_bridge)`,
  then one release run driving a tank across a span. If `next_layer == Ground` mid-span, confirmed
  in one crossing.

### C2 — The `unwrap_or(ground_level)` deck fallback, gated on `bridge_walkable`
`src/sim/movement/movement_bridge.rs:206` (Enter) and `src/sim/pathfinding/core.rs:1783`
(`effective_cell_z_for_layer`), both reaching `core.rs:1761-1763`.

- **For.** gamemd has **no branch anywhere that yields ground height while `OnBridge` is set** —
  the `+DeckOffset` add is unconditional on that byte and consults nothing else (VERIFIED,
  `0x005F5FB5`). The equivalent permission bit `0x200` is read only by `CheckBridgeTraversal` @
  `0x004D9C60` and appears in **no** height expression in the binary (VERIFIED, gamemd lane §5.3).
  This is precisely a gate/fallback in `sim/` that gamemd lacks. The Enter arm reads
  `bridge_structural` for the *predicate* and `bridge_walkable` for the *height* — two different
  flags in one arm (`movement_bridge.rs:201` vs `:206`). And when it fires it also poisons
  `BridgeOccupancy.deck_level` (`movement_bridge.rs:325`), hiding itself from BRIDGE_DIAG. The two
  flags are structurally incapable of agreeing: `bridge_structural` comes from the stamp alone,
  `bridge_walkable` additionally requires `!terrain_object_blocks && !overlay_blocks`
  (`resolved_terrain.rs:2412-2415`) **and** live runtime intactness (`core.rs:2215-2263`).
- **Against.** The mapdata lane decoded the real `XBayOPigs.MAP` packs and found **all 136**
  structural deck cells come out `bridge_walkable: true`, `has_bridge_deck: true`,
  `bridge_deck_level = level + 4 = 5`, `is_bridge_walkable() == true`, with **zero** `[Terrain]`
  objects anywhere in the bridge footprint. So on that map, at load, the fallback does not fire.
  Whether the user's map differs is **UNCHECKED**.
- **Cheapest kill/confirm.** A `log::error!` in `bridge_deck_level_if_any()` (`core.rs:1761`) when
  `bridge_structural && !bridge_walkable`, plus one at `movement_bridge.rs:206` when the
  `unwrap_or` actually substitutes. Silent for a whole match ⇒ C2 is not the live trigger on that
  map; it remains a DRIFT to remove regardless.

### C3 — Z is written only at cell boundaries; gamemd re-asserts it every step
`resolve_cell_transition_bridge_state` is the only mover Z writer on the crossing path
(`movement_bridge.rs:246/251/256`), reached at boundaries only.

- **For.** VERIFIED disassembly: `Process_Drive_Track` calls `SetHeight(0)` on **both** arms of the
  did-the-cell-change branch — `0x004B208A` (changed) and `0x004B20BD` (same cell, sub-cell step)
  — and `WalkLocomotionClass::ProcessMovement` does the same at `0x0075C1A1` / `0x0075C21C`.
- **Against.** This does not *initiate* the symptom; it removes gamemd's self-healing. In gamemd a
  unit that lost its deck Z for any reason regains it on the next sub-step. In VERA it stays lost
  until the next cell boundary, and possibly for the rest of the span.
- **Cheapest kill/confirm.** Nothing to check — the disassembly settles it. Classify as an
  amplifier and a recorded DRIFT, not a candidate initiator.

### C4 — The drive-track terminal commit bypasses the resolver entirely
`src/sim/movement/movement_tick.rs:202-209` — `position.z = head.z` with no bridge predicate,
where `head.z` came from `resolved_track_endpoint` (`movement_step.rs:68-80`), which itself calls
`effective_cell_z_for_layer(layer)`.

- **For.** A second, independent Z writer carrying the same layer dependence as C1 and none of the
  transition logic. `u8::try_from(head.z)` failing **silently skips the write** (`:207`), leaving
  stale Z. The surrounding occupancy calls hardcode `MovementLayer::Ground` (`:192, :198, :217,
  :228`), so a mover finishing a track on the deck is inserted into the **ground** object list.
  All VERIFIED-in-repo.
- **Against.** This is the terminal cell of a track — a path end, not a mid-span crossing. gamemd
  has an analogous no-`SetHeight` tail at `0x004B24A0`, but there the Z arrives pre-computed with
  `g_BridgeZOffset_Drive` (`0x004AF4A0` → `[0x008A07C4]`, also `4 × LevelStep`) already folded in
  at `0x004B2196` — so gamemd's tail is deck-correct by construction and VERA's is not.
- **Cheapest kill/confirm.** Log at `:207` when `entity.on_bridge && head.z != signed_level + 4`.

### C5 — `apply_pending_bridge_render_state` is skipped on stuck/deferred ticks
`src/sim/movement/movement_tick.rs:2567-2578`.

- **For.** `position.z` was already committed inside the crossing loop (`movement_step.rs:2126`)
  but the flags were not, so Z and `on_bridge`/`bridge_occupancy` can disagree for one or more
  ticks. The comment immediately above (`:2562-2566`) records that an earlier version of exactly
  this ordering problem caused the unit to "briefly dip to water level" — independent corroboration
  that the symptom class is real and Z-driven.
- **Against.** Produces a *transient* split, not a persistent ground-level Z across a span. Only
  fires when the tick aborted for stuck or deferred a vehicle cell check.
- **Cheapest kill/confirm.** Log when the skip fires with a non-`Unchanged` `pending_bridge_update`.

### C6 — `DEFAULT_BLOCKED_CELL` seeds an out-of-bounds lookup at height 0
`core.rs:976, 982` with `core.rs:1814-1825` (`ground_level: 0, bridge_deck_level: 0`).

- **For.** An OOB start/goal silently begins the search at height 0.
- **Against.** Requires an OOB cell; no evidence any lane found one on a real crossing.
- **Cheapest kill/confirm.** Count `unwrap_or(&DEFAULT_BLOCKED_CELL)` hits for one match.

### REFUTED — R1: draw-order / stream mis-routing as the cause
Killed by arithmetic, §2 above: the under-bridge depth bias is worth 0.5–1.2 px against a deck
occlusion that needs ~60 px. Render lane §4.

### REFUTED — R2: the renderer recomputes unit height from the terrain cell
Killed: the only terrain-cell reads on the unit draw path are `slope_type` for voxel tilt
(`units.rs:130-144`) and lighting tint (`units.rs:47-72`). Neither touches Z. Render lane §1d.

### REFUTED — R3: a structural cell missing from `BridgeRuntimeState`
Killed: the registration predicate `resolved_cell_has_runtime_deck`
(`src/sim/bridge_state/mod.rs:2117-2123`) is a strict **superset** of `PathCell.bridge_structural`
(`core.rs:2215-2221`), and pass 1 iterates every terrain cell. sim-z lane, candidate 3.

### REFUTED — R4: an intact bridge marked destroyed at load
Killed twice: `DamageState::from_state_byte` (`bridge_state/mod.rs:144-156`) **never** returns
`Destroyed` for any byte, and the caller `unwrap_or`s to `Healthy { variant: 0 }`
(`:2136`). Independently, Bay of Pigs' `[OverlayDataPack]` byte is `9` on all four lanes → healthy.
sim-z lane candidate 4, mapdata lane §B3.

### REFUTED — R5: the whole-deck `0x200` transition bit is the fall
Killed on the gamemd side: `CellClass::SetBridgeDirection_NESW` @ `0x0047E040` sets `0x200`
together with `0x100` on the anchor, the 2nd stepped cell and the opposite cell, omitting it only
on the 3rd stepped cell — which **matches** VERA's stamp (`src/map/bridge_facts.rs:285-316`), and
`0x200` touches no height expression anywhere in the binary (VERIFIED). The mapdata lane's finding
that `0x200` covers 3 of 4 lanes for the whole span is therefore native-shaped, not a defect, and
it is currently **load-bearing** for deck-to-deck traversal (`core.rs:639-645`). Do not "fix" it.
Carried to §6 as a residual.

### REFUTED — R6: bridge collapse / damage
Excluded by the user's own report: intact bridge, normal driving. Noted only because collapse *is*
the one production route where `has_bridge_deck` stays true while `bridge_walkable` goes false
(mapdata §B2) — the same state C2 describes, reached legitimately.

---

## 5. FIX PLAN

Smallest change that makes a tank sit on the deck. Every edit **removes** an invented input; none
adds a gate, clamp, sentinel or special case. All arithmetic is integer level-index arithmetic
(`u8`/`i16`) — no `f32`/`f64` anywhere in `sim/`; the lepton twin, where needed, is the existing
`BRIDGE_DECK_HEIGHT_LEPTONS` (`src/sim/map/bridge_topology.rs:61`).

**gamemd source for the whole slice:** `FootClass::Set_Height_On_Bridge` @ `0x005F5FA0` —
`ADD EDI,[0x00AC13BC]` gated on `this->OnBridge` (`+0x8C`) at `0x005F5FB5`, ground term from
`CellClass::GetGroundHeight` @ `0x00578080` → `CellClass::ComputeGroundHeightAtCoord` @
`0x0047B3A0`, store at `0x005F5FFF`; deck constant initialized as `4 × LevelStep` at `0x005F3860`
(`LEA ECX,[EAX*4]` @ `0x005F3866`); inverse invariant `ObjectClass::GetHeight` @ `0x005F5F30`.

### Edit 1 — make the resolver compute Z from `on_bridge`, not from the layer or the Option
**Target:** `src/sim/movement/movement_bridge.rs:229-261` (`resolve_cell_transition_bridge_state`).

- Add an `on_bridge_before: bool` parameter. Both callsites already hold it:
  `src/sim/movement/movement_step.rs:2126` has `projected_on_bridge_state` (computed one line
  earlier at `:2121`), and `src/sim/movement/movement_tick.rs:2285` has `entity.on_bridge`
  (`:2272`).
- Keep `compute_bridge_transition` **unchanged** — its predicate is VERIFIED verbatim against
  `0x004B2561-0x004B259B` (gamemd lane §4, both instruction-identical copies traced).
- Replace all three arms' Z assignment with one post-transition computation, mirroring gamemd's
  split of responsibility (flag block owns the flag; height function owns Z):

  ```rust
  let update = /* Enter -> Set(..), Exit -> Clear, NoChange -> Unchanged */;
  let on_bridge_after = projected_on_bridge(on_bridge_before, update);
  // FootClass::Set_Height_On_Bridge @ 0x005F5FA0: Z = GroundHeight(own cell)
  // + (OnBridge ? 4 levels : 0). Signed, matching MOVSX byte [cell+0x11B].
  position.z =
      (dst_cell.signed_level() + if on_bridge_after { BRIDGE_DECK_LEVEL_DELTA } else { 0 }) as u8;
  position.exact_z_leptons = None;
  ```

- `BridgeStateUpdate::Set` should carry the same computed value so `BridgeOccupancy.deck_level`
  stops being a second, independently-wrong number (`movement_bridge.rs:325`) and BRIDGE_DIAG at
  `movement_tick.rs:2588-2602` regains the ability to fire.
- `next_layer` becomes unused by the Z path. Keep the parameter only if the caller still needs it
  for `active_layer` (`movement_step.rs:2176`, `movement_tick.rs:2302`); otherwise drop it.
- **What this deletes:** the `unwrap_or(dst.ground_level)` at `:206`, the
  `effective_cell_z_for_layer(next_layer)` at `:256`, and every dependence of the mover's Z on
  `bridge_walkable` and on the planner.

**Constant.** Reuse `BRIDGE_DECK_LEVEL_DELTA` (`src/sim/movement/movement_occupancy.rs:39`,
currently module-private — widen to `pub(super)`). Do **not** introduce a new `4`. Note that
`movement_occupancy.rs:95-111` is already the gamemd-shaped model; after this edit the two agree
by construction instead of by coincidence.

### Edit 2 — remove the permission bit from the height query
**Target:** `src/sim/pathfinding/core.rs:1761-1763`, `:1781-1788`.

After Edit 1 nothing on the mover Z path reads `effective_cell_z_for_layer`. Leave
`bridge_deck_level_if_any()` in place for its remaining *non-height* consumers
(`is_elevated_bridge_cell` `:1756-1759`, terrain cost) but add a provenance comment recording that
gamemd's `0x200` equivalent is a `CheckBridgeTraversal` @ `0x004D9C60` input only and must never
reach a coordinate. If a later slice removes the last height consumer of
`effective_cell_z_for_layer`, delete the function rather than keeping the `unwrap_or`.

### Edit 3 — the drive-track endpoint takes the same model
**Targets:** `src/sim/movement/movement_step.rs:68-80` (`resolved_track_endpoint`) and
`src/sim/movement/movement_tick.rs:202-209` (terminal commit).

Replace `path_cell.effective_cell_z_for_layer(layer)` with
`path_cell.signed_level() + if on_bridge { BRIDGE_DECK_LEVEL_DELTA } else { 0 }`, threading the
mover's `on_bridge` in. gamemd source: `DriveLocomotionClass` tail at `0x004B24A0` commits a Z that
already folded `g_BridgeZOffset_Drive` in at `0x004B2196`; the constant is initialized as
`4 × LevelStep` at `0x004AF4A0` → `[0x008A07C4]`.

### Deliberately NOT in this slice — name them, do not do them
- **Per-step Z re-assert (C3).** gamemd calls `SetHeight(0)` on the same-cell sub-step too
  (`0x004B20BD`, `0x0075C21C`). Changing VERA's write cadence on a hot path is its own slice with
  its own fixture.
- **Hardcoded `MovementLayer::Ground` occupancy at `movement_tick.rs:192, 198, 217, 228, 2954`.**
  Real (a mover on the deck is inserted into the ground object list) but it is an occupancy-plane
  bug, not a height bug. Separate slice.
- **`bridge_walkable` vs `bridge_structural` in the A\* expansion** (`core.rs:464-465, 477, 482`).
  The repo already records this as a DRIFT in a provenance comment at `core.rs:447-463`
  ("Frequency: every bridge approach on every bridge map"). It moves every bridge-adjacent
  expansion at once — needs its own slice. Edit 1 makes the *height* immune to it in the meantime,
  which is the point.

### The test that pins it — a full drive across a span, not isolated cells

No existing test drives a unit across a bridge. The nearest,
`test_layered_path_transitions_onto_bridge_and_stays_on_deck`
(`src/sim/pathfinding/core_tests.rs:1028-1091`), asserts only the path's first and last cell — not
the layer or the carried height of any intermediate step.

Write `tank_drives_full_span_and_never_leaves_the_deck`:

1. Build a `PathGrid` with real span geometry: ramp at level 5 → **at least 8** deck cells at
   ground level 1 with `bridge_deck_level 5` → ramp at level 5. Multi-cell is the whole point;
   a two-cell fixture passes on the Enter arm alone and proves nothing.
2. Drive a `Drive` entity end to end and assert, **after every tick**, the gamemd invariant proven
   by `ObjectClass::GetHeight` @ `0x005F5F30`:
   `position.z == cell.signed_level() + if on_bridge { 4 } else { 0 }`.
   Assert it on the ramps too, not only on the deck.
3. **Variant A — hostile path layer.** Force `path_layers` to `Ground` for every mid-span node.
   Pre-fix this drops the tank to `z == 1` at cell 2; post-fix `z` must stay `5` for the whole span.
   This is the regression that C1 needs and that nothing currently covers.
4. **Variant B — decoupled flags.** One mid-span cell with `bridge_structural: true,
   bridge_walkable: false`. **This requires a new fixture builder**: both existing helpers tie the
   flags together — `PathGrid::set_cell_for_test` sets `bridge_structural: bridge_walkable`
   (`core.rs:2400-2427`) and `bridge_test_cell` derives both from one bool
   (`core_tests.rs:17-30`), so **the entire A\* and traversal suite is structurally incapable of
   producing the state C2 describes.** Building that fixture is part of this slice.
5. Assert `bridge_occupancy.deck_level == position.z` throughout, so BRIDGE_DIAG stays meaningful.

Run tier: `cargo test -p vera20k --lib sim::movement::movement_bridge::` while working; one full
`cargo test -p vera20k --lib` before the PR. Then a live release run driving a tank across a real
span — the decompile is the spec, the production experience is how you check it landed.

---

## 6. RESIDUALS

Each with trigger, player effect and ordinary-play frequency.

1. **Whole-deck `0x200` is load-bearing for deck-to-deck steps.** Trigger: every deck-to-deck step.
   Effect: none visible today — but `check_bridge_traversal`'s `diff == 0` arm (`core.rs:639-645`)
   *requires* `has_bridgehead_transition()`, so clearing `0x200` from body cells without changing
   that arm would make **every** deck step illegal. Frequency: every bridge crossing. gamemd's
   placer at `0x0047E040` matches VERA's stamp, so the flag is right and the *consumer* is the
   suspect. **UNCHECKED:** whether gamemd's deck-to-deck step consults `0x200` at all.
2. **Two contradictory height models coexist.** `movement_occupancy.rs:95-111`
   (`signed_level() + 4`, gamemd-shaped) vs the optional per-cell `bridge_deck_level` on the mover
   Z path. They agree only where `bridge_deck_level == ground_level + 4`, which
   `resolved_terrain.rs:2415` guarantees for stamp-structural cells but **not** for
   `has_bridge_deck && family == None` cells, whose deck level comes from
   `bridge_layer.deck_level` with `unwrap_or(level)` (`resolved_terrain.rs:2330-2334`). Trigger:
   any such cell on a real map. Frequency: **UNCHECKED** — no lane read `overlay_effects` to see
   whether that combination occurs. Edit 1 removes the second model from the Z path but leaves the
   stored value for other consumers.
3. **`BridgeRuntimeState` pass 4 is dead code in production.** `bridge_state/mod.rs:779-818`; its
   guard needs `bridge_walkable && !has_bridge_deck`, but `bridge_walkable ⇒ has_bridge_deck` holds
   at every write site in `resolved_terrain.rs`. Its own comment cites a "bridgehead pass" that no
   longer exists. Trigger: never in production; two tests are green on a state the pipeline cannot
   emit. Frequency: zero. Effect: false confidence.
4. **Under-bridge SHP pass writes depth; the ground SHP pass does not.**
   `merge_passes.rs:215` → `overlay_pipeline` (`LessEqual`, write ON, `batch.rs:655-658`) vs
   `draw_passthrough_range` (`Always`, no write). Trigger: any infantry standing in a structural
   bridge cell or an axis-matched orthogonal neighbour. Effect: that infantryman stamps the shared
   depth buffer and can clip later voxel units; the same man one cell away does not. Frequency:
   uncommon in ordinary skirmish, but gated on nothing rare.
5. **Deck art height is not driven by the deck-height map.** `instances/bridges.rs:162-165` anchors
   the body at the **ground** height map plus fixed `-16`/`-31` px art offsets; the 4-level lift
   enters only the *depth* term via `BRIDGE_HEIGHT_BONUS`. `bridge_height_map` is read only by
   click-pick, target lines and the debug overlay — **no draw path uses it**. Trigger: a map whose
   deck is not exactly ground + 4. Effect: tank and deck art disagree by `(deck − ground − 4) × 15`
   px. Frequency: zero on any standard bridge; unbounded on a non-standard one.
6. **Unsigned `z as f32` in the depth key.** `helpers.rs:368`, while `terrain.rs:316`,
   `lepton.rs:358` and `locomotor_visual.rs:113` all sign-extend via `as i8`. Trigger: a level byte
   ≥ 0x80. Effect: depth key off by up to 255 × 15 px. Frequency: zero on retail maps (levels 0-15).
   Same signedness asymmetry appears in `resolved_terrain.rs:2415`'s unsigned `saturating_add(4)`
   against `PathCell::signed_level()` (`core.rs:1777-1779`).
7. **Head-to mark released on the wrong plane.** `movement_tick.rs:2954-2956` —
   `occupancy_list_layer().unwrap_or(MovementLayer::Ground)`. Trigger: every drive finalization on
   a deck cell. Effect: occupancy-plane leak. Frequency: every crossing that ends on the deck.
8. **`u8::try_from(head.z)` silently skips the Z write.** `movement_tick.rs:207`. Trigger: a
   negative head Z. Effect: stale Z survives the terminal commit. Frequency: **UNCHECKED**.
9. **UNCHECKED gamemd items carried forward.** The numeral `LevelStep = 104` / `DeckOffset = 416`
   (only the `× 4` relation is VERIFIED); `object + 0x90`, the gate on the `0x004B1A96` SetHeight
   callsite; the roles of `Cell+0x140` bits `0x400`, `0x1000`, `0x10000`; whether gamemd's own
   `0x200` write set matches VERA's `stamp_intact` (the `0x200/0x400/0x800/0x10000` writes in
   `src/map/bridge_facts.rs:281-323` carry **no Ghidra address**).
10. **TS-legacy check, all lanes: clean.** Nothing in this diagnosis touches fog of war,
    subterranean locomotion, veins, rail bridges, drop pods or ion storm. Low-bridge `TubeClass`
    movement (`src/sim/movement/tube_movement.rs`) is a genuinely separate active YR system and
    shares no deck-height flag — do not conflate it with the `0x100` high-bridge path.
