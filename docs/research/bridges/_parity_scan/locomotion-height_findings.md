# Parity Scan — Locomotion-Height (on-bridge vs under-bridge locomotion, height, render/Z offsets)

**Facet:** On-bridge vs under-bridge locomotion, height, render/Z offsets.
**Rust sources audited:** `src/sim/movement/movement_bridge.rs`, `src/sim/occupancy.rs`, `src/sim/movement/drive_locomotion.rs`, plus the load-bearing consumers `src/sim/pathfinding/core.rs` (PathCell, CheckBridgeTraversal, height math), `src/sim/movement/movement_occupancy.rs` (`runtime_current_effective_height`), `src/sim/movement/movement_step.rs` (per-step transition + cliff gate), `src/sim/components.rs` (Position.z type).
**Binary anchors verified this session** (all resolved via `get_function_by_address` / `decompile_function`):
- `FootClass__Set_Height_On_Bridge @ 0x005f5fa0`
- `FootClass__ShouldBeOnBridge @ 0x004ddc40`
- `ObjectClass__ShouldBeOnBridge @ 0x005f6a70`
- `DriveLocomotionClass__ComputeBridgeRenderOffset @ 0x004af470`
- `DriveLocomotionClass__ComputeBridgeZOffset @ 0x004af4a0`
- `ShipLocomotionClass__Compute_BridgeZOffset @ 0x0069ebb0`
- `IsOnBridgeRamp @ 0x00578d80`
- `CellClass__IsOnBridgeSurface @ 0x00485060`
- `CheckBridgeTraversal @ 0x004d9c60`
- `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073b140`
- globals `0x00ac13c8` (LeptonsPerLevel), `0x00ac13bc` (Set_Height bridge add), xrefs of `0x00ac13c8`.

> Default verdict is DRIFT. Downgrades to PARITY-CONFIRMED only with algebraic/bit-identical/exhaustive-caller proof.

---

## Cross-cutting context: the unit-of-Z mismatch

The single most load-bearing fact for this whole facet:

- **gamemd**: all bridge Z math is in **leptons**. The bridge deck sits `g_BridgeZOffset_Drive = ftol(g_DriveHeightStep * 4 + 0.5)` leptons above ground, where `g_DriveHeightStep` is the per-height-level lepton displacement under the isometric tilt. `CellClass__GetGroundHeight` returns leptons; `unit.Z` (TechnoClass +0xA4) is leptons. The cell `+0x11B` (`Level`) byte is in height-LEVEL units (0-15). Set_Destination does `dest.Z = GetGroundHeight(cell) + g_BridgeZOffset_Drive` (leptons). `ObjectClass__ShouldBeOnBridge` and `Teleport::Process` use `LeptonsPerLevel * 3` (leptons) thresholds. Verified: `DriveLocomotionClass__ComputeBridgeZOffset @ 0x004af4a0` = `g_BridgeZOffset_Drive = Math__ftol(g_DriveHeightStep * 4)`; `ObjectClass__ShouldBeOnBridge @ 0x005f6a70` = `DAT_00ac13c8 * 3 < (ground_dest - ground_cur)` where `DAT_00ac13c8` = LeptonsPerLevel.
- **Rust**: `Position.z` is `u8` (`src/sim/components.rs:37`) — a height-LEVEL value (0-15), NOT leptons. The deck is `ground_level + 4` LEVELS (`movement_occupancy.rs:39 BRIDGE_DECK_LEVEL_DELTA: i16 = 4`; `movement_bridge.rs:91` uses `wrapping_sub(4)`; `pathfinding/core.rs:1997` `ground_level.saturating_add(4)`).

This is internally consistent on the Rust side ONLY IF the entire Z pipeline (sim + render) is height-levels and `+4 levels` is the real deck height. It is NOT a 1:1 port of any gamemd constant — gamemd's "4" is "4 height-LEVELS expressed in leptons via height_step", and the on-bridge predicate threshold is `3 levels` (in leptons), not 4. The Rust transition predicate keys on exactly `== -4 levels` (D1) and the runtime/pathfinding gates use thresholds of 2 levels (D5). Each is a separate translation choice that must be checked against the matching gamemd site, NOT assumed equal because both are "level math."

---

### D1: on_bridge transition predicate uses height-only `== -4`; gamemd uses cell-flag-and-level state machine with a distinct cliff-jump case

- **Rust now:** `compute_bridge_transition` (`movement_bridge.rs:84-103`). Enter fires iff `dst_h == src_h.wrapping_sub(4) && dst.has_structural_bridge()`. Exit fires iff `!dst.has_structural_bridge() && src.has_structural_bridge()`. Enter and Exit are the only two state changes; everything else is NoChange. This is keyed on `bridge_structural` (a Rust-synthesized flag ≈ cell.flags & 0x100 minus ramp/transition cells) and an exact `-4` level diff.
- **gamemd:** `Process_Drive_Track` sites 1-3 (DRIVE_SHIP doc §4.1-4.2, raw-asm verified at 0x4B181E/0x4B1830/0x4B184A) implement a 6-row table on `(new.Level - old.Level)`, `new.flags & 0x100`, `old.flags & 0x100`:
  - `diff == -4` AND new IS bridge → set on_bridge=1 (cliff-top → deck; fires REGARDLESS of old flag, and does NOT then run the clear check).
  - `diff == -4` AND new NOT bridge AND old WAS bridge → clear.
  - `diff != -4` AND new NOT bridge AND old WAS bridge → clear (the common "step off bridge" path).
  - all other rows → unchanged.
  The set-to-1 path uses the level diff `== -4` (the cliff-jump-onto-deck case); the clear path fires for `old IS bridge && new NOT bridge` at ANY diff, not only `-4`.
- **Fixture:** Unit on a high-bridge body cell (old: structural bridge, level 0, on deck) steps to an adjacent ground cell at the SAME ground level 0 that is NOT bridge (new: ground level 0). gamemd: `diff = 0 != -4`, new NOT bridge, old WAS bridge → clear on_bridge=0 (correct — unit walked off the side of a flush bridgehead). Rust `compute_bridge_transition`: src `has_structural_bridge`=true, dst `has_structural_bridge`=false → Exit fires → Clear. Match here. BUT take the cliff-jump fixture: old = a cliff/ground cell at level 4 (NOT structural bridge), new = bridge body cell at level 0 (structural bridge), diff `0 - 4 = -4`. gamemd: `diff == -4` AND new IS bridge → set on_bridge=1. Rust Enter: `dst_h(0) == src_h(4).wrapping_sub(4)=0` AND `dst.has_structural_bridge()` → Enter fires. Match. The divergence surface is the CLEAR side: gamemd clears whenever `old bridge & new not-bridge` at any diff; Rust Exit requires the same condition but is also gated by the structural-vs-ramp classification (`has_structural_bridge`), so a body→ramp step (ramp not structural) is NoChange in Rust (intentional, see ramp tests) whereas gamemd keys purely on `flags & 0x100` — if the ramp tile carries `flags & 0x100` (high-bridge ramp cells DO carry 0x100 per CheckBridgeTraversal `+0x11b + 4` deck math), gamemd would NOT clear (new IS bridge) but Rust also NoChange. The cases align on retail data, but the Rust predicate is a re-derivation, not the gamemd table, and the `bridge_structural` classification is the load-bearing assumption.
- **Player sees:** on_bridge flag drives object-list layer (ground vs bridge occupancy) and runtime height. A mis-timed flag means a unit crossing a bridge edge briefly occupies/collides on the wrong layer, or renders at the wrong elevation for 1 tick. Triggers every time a vehicle enters/leaves a high bridge — common on any map with a bridge.
- **Severity:** MED (visible 1-tick layer/elevation glitch at bridge edges; bridge crossings are common but the predicate matches on retail data for the tested fixtures).
- **Confidence:** LIKELY-DRIFT — the Rust predicate is a clean re-derivation that depends on `bridge_structural` being an exact stand-in for `cell.flags & 0x100` at ramp cells; not proven bit-identical across all ramp/body/bridgehead tile combinations, and the clear-on-any-diff vs Exit-requires-structural distinction is unproven equivalent.
- **Verify-call:** `decompile_function 0x004d9c60` (CheckBridgeTraversal — shows ramp/bridgehead use `+0x11b + 4` and `flags & 0x100/0x200`); DRIVE_SHIP doc §4.1-4.2 raw-asm at 0x4B181E/30/4A.

---

### D2: Set_Destination bridge Z-bump is entirely ABSENT in the Rust Drive/Ship locomotor

- **Rust now:** `drive_locomotion.rs` has NO bridge Z math at all (the whole file is the speed-fraction scaffold; only the test fixture mentions `bridge_deck_level: 0` at line 190). The Z that a unit ends up at is written by `resolve_cell_transition_bridge_state` (`movement_bridge.rs:119-148`), which sets `position.z = deck_level` on Enter / `dst.ground_level` on Exit / `dst.effective_cell_z_for_layer(next_layer)` on NoChange. There is no per-Set_Destination "add bridge offset to destination Z" step, and no `g_BridgeZOffset_Drive`/`g_BridgeZ_Offset_Ship` equivalent.
- **gamemd:** `DriveLocomotionClass::Set_Destination @ 0x4AFD40` (and Ship mirror `0x69F450`): after storing dest X/Y/Z, if dest is not the NullCoord sentinel, `iVar2 = Get_Cell_At(&dest); if (cell.flags & 0x100) dest.Z += g_BridgeZOffset_Drive;` — UNCONDITIONAL bump whenever the dest cell has `0x100` (body OR bridgehead; bit 0x200 is not consulted). The bump is re-applied even if the unit is already on the bridge (no height-diff guard). DRIVE_SHIP doc §3.2/§3.5, raw-asm at 0x4AFDE2.
- **Fixture:** Vehicle ordered to a bridge body cell at ground level 0, deck at +4 (levels) / `GetGroundHeight + g_BridgeZOffset_Drive` (leptons). gamemd: dest.Z = ground + offset (deck). Rust: the destination Z is never set in a Set_Destination step; the unit's `position.z` only changes at the cell-boundary transition, and the path layer (A*) decides whether it walked the bridge layer. Because Rust models Z in levels and resolves it at the boundary, the observable end-state (unit at deck level when on the deck) is reproduced via a DIFFERENT mechanism. The divergence is the deceleration/approach math: gamemd recomputes dest Z (`0x4B0FE7`, branchless `bridge ? offset : 0`) for the distance-to-goal Sqrt / deceleration ramp; Rust's `update_drive_speed_fraction` (`drive_locomotion.rs:102`) uses a flat `distance_to_goal` with no Z component, so the brake distance is the same on and off a bridge.
- **Player sees:** because the deck is only +4 levels (~tens of leptons), the dest-Z contribution to a 3D distance is small; the observable effect is at most a 1-frame difference in when a vehicle begins braking as it climbs onto a bridge. Low visibility but it fires on every bridge approach.
- **Severity:** LOW (sub-frame brake-timing on bridge approaches; the end elevation IS reproduced).
- **Confidence:** PROVEN-DRIFT (the Set_Destination Z-bump and the approach-Z recompute simply do not exist in the Rust code; mechanism is absent).
- **Verify-call:** `decompile_function 0x004af4a0` and `0x0069ebb0` (the two ComputeBridgeZOffset inits) — both present in binary, no Rust counterpart.

---

### D3: ShouldBeOnBridge threshold is `LeptonsPerLevel * 3` (3 levels), not 4 — and the predicate itself is unimplemented

- **Rust now:** No `ShouldBeOnBridge` equivalent. The closest Rust gate is `is_at_bridge_level` (`pathfinding/core.rs:410-412`) which uses `abs_diff(ground_level) >= 2` (BRIDGE_HEIGHT_THRESHOLD=2), and the runtime `evaluate_runtime_can_enter_cell` (`movement_occupancy.rs:152`) uses `>= 2`. The on_bridge transition (D1) keys on exactly `-4`.
- **gamemd:** `ObjectClass__ShouldBeOnBridge @ 0x005f6a70` (verified this session): two branches both gated by `DAT_00ac13c8 * 3 < |ground_dest - ground_cur|` where `DAT_00ac13c8` is LeptonsPerLevel. I.e. the "should be on bridge" / "should drop off bridge" decision uses a **3-height-level** lepton threshold. `FootClass__ShouldBeOnBridge @ 0x004ddc40` gates on `FootClass+0x684` sign then delegates to ObjectClass. `Teleport::Process` (WALK_DROPPOD_TELEPORT doc §4.5b) independently uses `unit.Z > ground + DAT_00B0EC38 * 3` — the SAME 3-level criterion for on-bridge detection. So gamemd's "am I physically high enough to be on the deck" test is `> 3 levels above ground`, accepting units whose anchor Z is below the exact deck (4 levels) because the anchor sits mid-body.
- **Fixture:** A unit's anchor Z sits 3.5 levels above ground while standing on a deck whose top is at +4. gamemd ShouldBeOnBridge: `3 * LeptonsPerLevel < 3.5 * LeptonsPerLevel` → true → on bridge. A Rust gate that demanded exactly `== 4` (D1 Enter) or `>= 4` would reject this mid-transition state; Rust's runtime layer gate uses `>= 2` (D5) which is even looser, so the boundary behaviors differ on both sides. The exact boundary (`> 3` strict, leptons) is unreproduced.
- **Player sees:** edge-case at the moment a unit is partway up/down a ramp — whether it is classified on-deck. Affects which occupancy layer it collides on and its render elevation for the in-between frames. Fires on every ramp traversal.
- **Severity:** MED (ramp-traversal layer classification; ramps are crossed on every bridge use).
- **Confidence:** PROVEN-DRIFT (gamemd uses a `*3` lepton threshold verified live; Rust has no matching predicate and its nearest gates use 2-level and exact-4-level criteria).
- **Verify-call:** `decompile_function 0x005f6a70` (shows `DAT_00ac13c8 * 3 < iVar3 - iVar2` twice); `get_xrefs_to 0x00ac13c8` (confirms LeptonsPerLevel read by ShouldBeOnBridge, IsHighFlying, IsLowFlying).

---

### D4: Bridge Z-offset rounding (round-half-UP for Drive/Ship, round-half-DOWN for Walk) is not modeled — Rust uses a single exact `+4` levels

- **Rust now:** deck height is exactly `ground + 4` levels everywhere (`movement_occupancy.rs:39`, `movement_bridge.rs:96`, `pathfinding/core.rs:1997`). No rounding, no per-locomotor offset.
- **gamemd:** Drive `g_BridgeZOffset_Drive = ftol(g_DriveHeightStep*4 + 0.5)` = round-half-UP (DRIVE_SHIP doc §2.2, the `+0.5` at 0x007E1738 is hidden by the decompiler but present in raw asm `FADD double ptr [0x007e1738]`). Ship: identical pattern, separate global `g_BridgeZ_Offset` from `g_ShipHeightStep`. Walk: `FUN_006D2120(60) = ftol((60 - 0.5) * DAT_00B0CDD8)` = round-half-DOWN (WALK doc §2.3, `FSUB double ptr [0x007e1738]`). Teleport: separate `g_BridgeZOffset_Teleport`. Each is a separately-initialized runtime constant; on a `.5` lepton boundary Drive and Walk differ by 1 lepton, and each locomotor can differ from the others.
- **Fixture:** If `g_DriveHeightStep * 4` lands on `N.5` leptons, Drive deck offset = `N+1`, Walk's `(60-0.5)*scale` could land on `M.5` giving `M` (truncate-toward-zero after the `-0.5`). A vehicle and an infantry standing on the same deck cell would have anchor Z differing by 1 lepton. In Rust, both are exactly `ground + 4` levels — identical, no rounding, and not per-locomotor.
- **Player sees:** sub-pixel (1 lepton) vertical offset difference between a vehicle and infantry on the same deck; only observable if the render pipeline converts leptons→pixels and that 1 lepton crosses a pixel boundary. Fires whenever both unit classes share a deck.
- **Severity:** LOW (1-lepton, only at `.5` boundaries; but it IS a determinism/parity drift in the Z value if any sim logic compares unit Z to a lepton threshold).
- **Confidence:** PROVEN-DRIFT (the four distinct rounding constants and the round-up/round-down split are raw-asm verified; Rust collapses all to exact `+4` levels).
- **Verify-call:** `decompile_function 0x004af4a0` (Drive, `*4`), `0x0069ebb0` (Ship), and WALK doc §2.3 raw-asm of FUN_006D2120 (`FSUB` = round-down). The shared `0.5` constant is at `0x007E1738`.

---

### D5: Runtime ground-vs-bridge layer selection threshold is `>= 2` levels in Rust, but gamemd's runtime per-step pick uses `Z >= ground + g_BridgeZOffset_Drive` (a `>= 4`-levels lepton compare) AND the scatter pick uses `>= 3`

- **Rust now:** `evaluate_runtime_can_enter_cell` object-list-layer pick (`movement_occupancy.rs:151-157`): `candidate.has_structural_bridge() && (height == -1 || |height - candidate.signed_level()| >= 2)` → Bridge layer. `is_at_bridge_level` also `>= 2` (`core.rs:411`). The cliff gate uses `>= 3` levels (`mod.rs:98 CLIFF_HEIGHT_THRESHOLD=3`, applied at `movement_step.rs:1099`).
- **gamemd:** three DISTINCT site-specific thresholds (DRIVE_SHIP doc §4.5, raw-asm verified):
  - A* closed-list layer decision (Phase-1): `abs(path_height - cell.Level) >= 2` (the `< 2` at 0x429e7d). → Rust `>= 2` matches THIS site.
  - TooBigToFitUnderBridge runtime crush-layer pick: `unit.Z >= ground + g_BridgeZOffset_Drive` (signed `JL` at 0x4B18CC) → bridge list, else ground list. This is a `>= 4-levels` (leptons) compare, NOT `>= 2`.
  - Scatter case-6 layer pick: `abs(unit.Z / g_DriveHeightStep - cell.Level) > 2` i.e. `>= 3 levels` (raw-asm `CMP EAX, 2 / JG` at 0x4B1F11). Rust scatter has no per-layer height pick at this threshold.
- **Fixture:** A TooBigToFitUnderBridge vehicle (Apocalypse) under a high bridge with `unit.Z` at 2 levels above ground (e.g. on a half-ramp). gamemd crush pick: `2 < 4` → GROUND list (it crushes ground-layer occupants under the bridge). Rust `is_at_bridge_level`/`evaluate_runtime_can_enter_cell`: `|2 - 0| = 2 >= 2` → BRIDGE layer → would treat it as on the deck. Opposite layer selected at the `2 ≤ diff < 4` band.
- **Player sees:** for the A* layer pick the thresholds match. For the runtime crush/scatter picks they diverge in the 2-3-level band — a tall unit on a ramp could be assigned to the wrong occupancy layer, crushing or colliding with the wrong set of units. Fires when oversized units traverse bridge ramps (Apocalypse/Mammoth on bridges).
- **Severity:** MED (wrong-layer crush/collision for oversized units on ramps; oversized units on bridges are a real but not constant occurrence).
- **Confidence:** PROVEN-DRIFT (three distinct gamemd thresholds raw-asm verified; Rust uses a single `>= 2` for layer selection and lacks the `>= 4`-lepton crush pick and `>= 3` scatter pick entirely — see D6).
- **Verify-call:** DRIVE_SHIP doc §4.3/§4.5 (raw-asm at 0x4B18CC `ADD EAX,[0x008a07c4] / JL` and 0x4B1F11 `CMP EAX, 2 / JG`); A* `< 2` site noted at core.rs:400 comment 0x429e7d.

---

### D6: TooBigToFitUnderBridge runtime crush-layer behavior and self-damage are missing

- **Rust now:** `too_big_to_fit_under_bridge` is parsed and carried on the snapshot (`mod.rs:145`, `game_entity.rs:310`) and `movement_path.rs:46` explicitly comments "gamemd does not gate movement on TooBigToFitUnderBridge." It is threaded into `handle_blocked_tick`/bump_crush but there is NO runtime "iterate the conflicting occupancy layer and apply 10000 damage to each occupant, plus 20 self-damage" behavior. `bump_crush.rs` uses it only for neighbor-count/blocker logic, not the layer-aware deck-crush.
- **gamemd:** `Process_Drive_Track` site 4 (DRIVE_SHIP doc §4.3, §8.4): when `TooBigToFitUnderBridge != 0 && FootClass+0x6D0 == 0`, select the conflicting layer list (`on_bridge ? bridge(+0xE8) : Z>=ground+offset ? bridge : ground(+0xE4)`), walk it, apply **10000 damage** (warhead `RulesClass+0xFA8`) to each occupant, and apply **20 damage** to the crusher itself. Ship mirrors this exactly (doc §13.2). `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073b140` (verified this session) is the rendering counterpart — a split-blit (upper full + lower 16×16 strip at priority -5/mode-2) when the TooBig unit is on a bridge edge with zero bridge-piece neighbors.
- **Fixture:** Apocalypse drives onto a high-bridge deck cell that has a Grizzly parked on the ground below it (ground layer). gamemd: the deck occupant on the bridge layer takes 10000 dmg (destroyed), Apocalypse takes 20 dmg. Rust: no crush occurs; both units coexist.
- **Player sees:** the signature "Mammoth/Apocalypse crushes everything on the bridge deck it runs onto" is absent; oversized units don't destroy deck occupants. Plus the render BridgeFudge split-blit (so the deck draws over the lower half of the tank when it pokes through the bridge) is absent in render. Fires whenever a TooBig unit shares a bridge-overlap cell with occupants.
- **Severity:** MED (distinctive missing crush mechanic for the biggest units; conditional on a TooBig unit meeting deck/under occupants, but unmistakable when it should happen).
- **Confidence:** PROVEN-DRIFT (no runtime layer-crush or self-damage in Rust; mechanism absent).
- **Verify-call:** `decompile_function 0x0073b140` (BridgeFudge split-blit logic, confirms the TooBig+IsOnBridge_ForFiring+neighbor-count==0 trigger); DRIVE_SHIP doc §4.3/§8.4 for the 10000/20 damage and layer pick.

---

### D7: CheckBridgeTraversal — Rust port needs spot-check against the live `+0x11b + 4` deck-level and bidirectional ramp rules

- **Rust now:** `check_bridge_traversal` in `pathfinding` (called from `movement_occupancy.rs:159`, returns `allowed` / `force_bridge_list` / `path_height`). `compute_neighbor_height` (`core.rs:417-445`) implements the Ground→Bridge entry as "diff EXACTLY 4 AND `neighbor_cell.transition`" → deck, otherwise ground/under.
- **gamemd:** `CheckBridgeTraversal @ 0x004d9c60` (decompiled this session). Key rules confirmed live: when `*param_3 == -1` and the OTHER cell has `0x100`, seed `*param_3 = other.Level + 4` (deck height). Return 7 (blocked) when entering a `0x100` cell whose source lacks the bridgehead `0x200`. `abs(height_diff) == 1` → ramp passability gated on `cell+0x11C` (slope index) of the lower cell (direction-dependent: `uVar3 < 1` checks `param_5+0x11c`, else `param_1+0x11c`). `abs(height_diff) == 4` → bridge entry/exit: sets `*param_4 = 1` (force-bridge-list / bridge-entered) only when `param_1` has BOTH `0x100` and `0x200`; otherwise return 7. Any other diff → return 7.
- **Fixture:** Ground cell (level 0, has 0x100+0x200 bridgehead) → bridge body neighbor (level 0, `+0x11b`=0 so deck=4), unit at path_height 4 descending. gamemd: `iVar5=0` (param_1.Level), neighbor is bridge so `iVar2=neighbor.Level=0`, `uVar3=0-0=0`, `abs=0` → first branch (`(uVar3 ^ uVar4)==uVar4`, abs==0): if param_1 lacks 0x100/0x200 OR neighbor not bridge, AND `*param_3 != -1 && != iVar5` → 7. Walking the exact branch requires the live `+0x11C` and `+0x11B` byte values. Rust's `compute_neighbor_height` collapses this to "diff==4 && transition → deck" plus a separate ramp-diff-1 path; the `*param_4=1` force-bridge-list is mapped to `bridge_traversal.force_bridge_list`. The mapping is plausible but the bidirectional `+0x11C` slope check and the `+0x11b + 4` seed are not obviously 1:1.
- **Player sees:** whether a unit is allowed onto/off a bridge at a given ramp, and which layer it lands on. A wrong gate would block a legal bridge entry or allow an illegal one. Fires on every bridge entry/exit pathfind.
- **Severity:** MED (pathing legality at bridge entries; very common, but the Rust port has dedicated tests and matches on retail tile data).
- **Confidence:** UNCHECKED — I verified the gamemd function live and read the Rust `compute_neighbor_height`, but did NOT walk every branch of both with concrete `+0x11B`/`+0x11C` byte fixtures to prove equivalence. The two are structurally different (Rust pre-classifies cells into `bridge_structural`/`transition`/`slope_type`; gamemd reads raw `+0x140`/`+0x11B`/`+0x11C`). Equivalence is plausible but unproven.
- **Verify-call:** `decompile_function 0x004d9c60`.

---

### D8: `IsOnBridgeRamp` / `IsOnBridgeSurface` are tile-index-range predicates; Rust uses synthesized `transition`/`slope_type` flags

- **Rust now:** Rust classifies ramp/surface via `PathCell.transition`, `bridge_structural`, `bridge_marker_0x80`, `slope_type` (set at terrain-resolve time from TMP tile data). `is_bridge_transition_cell()` = `self.transition` (`core.rs:1483`); `can_enter_bridge_layer_from_ground` = `bridge_walkable && transition` (`core.rs:1521`).
- **gamemd:** `IsOnBridgeRamp @ 0x00578d80` (decompiled live): tests `cell.IsoTileTypeIndex (+0x38)` against ranges of bridge-tile-type globals (`DAT_00aa1020..+0x28`, `DAT_00aa073c..+4`, `DAT_00abb110..+4`, `DAT_00aa1050..+4`, `DAT_00aa10a0..+4`, `DAT_00abbebc..+0x14`) with direction-specific (`param_2` = facing 0/1/2/3/4) sub-checks for the endpoint tiles. `IsOnBridgeSurface @ 0x00485060` (live): `DAT_00aa0738 <= cell+0x38 < DAT_00aa0738 + 0xe` — a 14-tile-index window on the wood-bridge tileset. These are TILE-INDEX membership tests, set at map load by the bridge tile-type registry.
- **Fixture:** A wood low-bridge cell whose `+0x38` IsoTileTypeIndex is `DAT_00aa0738 + 7`. gamemd IsOnBridgeSurface → 1 (it is a bridge surface). Rust has no tile-index window; it relies on `bridge_walkable`/`transition` having been set correctly during terrain resolution for that tile. If the resolver's tile→flag mapping omits one of the 14 surface tiles or one of the directional ramp endpoint tiles, the Rust classification silently diverges.
- **Player sees:** correctness of which cells are ramps/surfaces — drives the whole bridge layer system. Any omitted tile index = a cell that should be a bridge surface but isn't (unit falls through to ground layer there). Fires at map load; effect is per-cell.
- **Severity:** MED (foundational classification; if the terrain resolver's tile tables are complete it's fine, but the gamemd predicate is range/direction-specific and the Rust equivalent is an upstream resolver concern not visible in these files).
- **Confidence:** UNCHECKED — the gamemd predicates are verified live, but whether the Rust terrain resolver reproduces the exact tile-index ranges (incl. the directional endpoint sub-checks in IsOnBridgeRamp where `param_2 ∈ {0,1,3,4}` matters) is outside the audited files and not proven.
- **Verify-call:** `decompile_function 0x00578d80` and `0x00485060`.

---

### D9: Set_Height_On_Bridge adds a separate deck constant and recenters Fly objects via exact cell ground height

- **Rust now:** `src/sim/world/lifecycle.rs::sync_fly_object_height` stores exact Fly Object Z as exact sloped ground + the `OnBridge` 416-lepton deck offset + signed locomotor altitude before the final Put. Object `0x40` occupation then compares that exact Z with exact ground. Jumpjet is deliberately excluded because its active coordinate updater is a different state machine.
- **gamemd:** `FootClass__Set_Height_On_Bridge @ 0x005F5FA0` adds `g_nFootOnBridgeDeckOffsetLeptons` when Object+0x8C `OnBridge` is set, then writes `Object+0xA4 = CellClass::GetGroundHeight(Object.XY) + signed_height`. When Object+0x74 is marked it brackets the write with vtable+0x124 Remove/Put. `FlyLocomotionClass__Process @ 0x004CD600` reaches this virtual at active callsites `0x004CDE9D` and `0x004CDFB6`; the old "AnimClass-only" conclusion came from direct-xref enumeration and was wrong. Jumpjet instead calls `JumpjetLocomotionClass__Update_Coordinates_And_Altitude @ 0x0054D0F0`, which uses its own bridge/controller state and commits a full coordinate via vtable+0x1B4.
- **Fixture:** A Fly aircraft over a sloped live bridge cell is removed, advances altitude, commits exact absolute Z, and is put back on the plane selected by the inclusive exact-ground + 416 threshold. Missing/out-of-map ground lookup uses the native zero-height dummy cell before the optional deck offset.
- **Player sees:** Fly takeoff/landing and bridge-plane occupation remain consistent with the aircraft's actual altitude; Jumpjet behavior is not approximated through the Fly formula.
- **Severity:** CLOSED for the bounded Fly/Object-height seam; Jumpjet exact-coordinate parity remains a separate active-mechanism residual.
- **Confidence:** VERIFIED for Fly/Object height via live bodies, vtable callsites, and the focused `gsi_05_05_*` Rust tests. Jumpjet remains intentionally unclaimed beyond its separately verified native updater.
- **Verify-calls:** `decompile_function 0x005F5FA0`, `decompile_function 0x004CD600`, assembly context at `0x004CDE9D` / `0x004CDFB6`, and `decompile_function 0x0054D0F0` / caller `0x0054AEC0` (2026-08-13).

---

### D10: Ship under-bridge Z clearance constant differs from gamemd's `g_BridgeZ_Offset_Ship` (separate runtime global)

- **Rust now:** `movement_bridge.rs:74` `BRIDGE_Z_OFFSET: SimFixed = lit("360")` with the comment "360 == 90 * 4 — the Z distance from water surface to bridge deck. Added to braking distance when a ship passes under a bridge cell." This is a hardcoded `90 leptons/level * 4` value used in ship braking-distance math.
- **gamemd:** Ship uses `g_BridgeZ_Offset` (its OWN global at `0x00B0782C`), initialized by `ShipLocomotionClass__Compute_BridgeZOffset @ 0x0069ebb0` (verified live) = `ftol(g_ShipHeightStep * 4)`. `g_ShipHeightStep` is a runtime value from the isometric `Sin_Lookup` math — NOT necessarily `90`. Drive's is a DIFFERENT global (`g_BridgeZOffset_Drive @ 0x008A07C4` from `g_DriveHeightStep`). The two are separately computed and may hold different runtime values (DRIVE_SHIP doc §1: "Each locomotor owns its own global. None are shared.").
- **Fixture:** If `g_ShipHeightStep` resolves to, say, 91 leptons at the standard view angle, gamemd's ship bridge offset is `ftol(91*4 + 0.5) = 364`, not 360. Rust's hardcoded 360 would be 4 leptons short in the ship's under-bridge braking-distance calc.
- **Player sees:** ship deceleration timing when sailing under a high bridge differs by the lepton delta; sub-frame brake timing. Fires when a ship sails under a bridge — common on naval maps with bridges.
- **Severity:** LOW (sub-frame ship brake timing; the `90*4` is an assumed height_step, not the verified runtime `g_ShipHeightStep*4`).
- **Confidence:** LIKELY-DRIFT — the Rust 360 is a hardcoded `90*4`; gamemd computes `ftol(g_ShipHeightStep*4 + 0.5)` at runtime. `g_ShipHeightStep`'s runtime value is BSS/cold-dump-zero in the static binary (verified `read_memory 0x00B07838`-region zeros), so I cannot prove `90` is right or wrong without a runtime value — but it is an unverified assumption, hence DRIFT by default.
- **Verify-call:** `decompile_function 0x0069ebb0` (Ship `g_BridgeZ_Offset = ftol(g_ShipHeightStep * 4)`); cf. `0x004af4a0` for Drive's separate global.

---

### D11: Render does not consume `on_bridge` / `bridge_occupancy` / deck `position.z` for unit sprite elevation, and the BridgeFudge split-blit is absent

- **Rust now:** Grep of `src/render/` for `on_bridge`, `bridge_occupancy`, `deck_level`, `BridgeFudge`, split-blit, ZFudge returned NO matches. The sim maintains `entity.on_bridge`, `entity.bridge_occupancy.deck_level`, and `position.z`, but the render layer has no bridge-aware sprite lift or the TooBig split-blit.
- **gamemd:** `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073b140` (verified live) splits a TooBig unit's sprite into an upper full pass (priority -5) and a lower 16×16 strip (mode 2) when `IsOnBridge_ForFiring && bridge_piece_neighbor_count==0`, so the deck occludes the lower part of the unit. Plus the `ZFudgeBridge`/`ZFudgeCliff`/`ZFudgeTunnel`/`ZFudgeColumn` per-TechnoType render Z-fudge family (HIGH_BRIDGE doc §13.9, TechnoType +0xDC0..+0xDCC). And the deck Z lift (4 levels in leptons) raises the unit sprite when on the deck.
- **Fixture:** Tank on a high-bridge deck with another unit on the ground below. gamemd: tank renders lifted to deck height, lower half occluded by the deck rail via the split-blit; ZFudgeBridge tweaks its depth-sort. Rust: tank renders at its ground screen position with whatever `position.z` maps to (and z is in levels), no split-blit, no ZFudge family.
- **Player sees:** units on bridge decks may render at the wrong screen height and without the deck-edge occlusion; the signature look of a tank poking up through a bridge rail is wrong. Fires whenever units occupy bridge decks.
- **Severity:** MED — but this is RENDER, which is outside this facet's strict sim scope; flagged here because the deck `position.z` produced by this facet's code is the INPUT the render layer should consume and currently doesn't.
- **Confidence:** PROVEN-DRIFT (render has no bridge-aware consumption of these fields; verified by grep). Note: per CLAUDE.md this facet covers the sim-side Z; the render gap is logged for the render-presentation facet owner.
- **Verify-call:** `decompile_function 0x0073b140`; HIGH_BRIDGE doc §13.9 (ZFudge family).

---

## PARITY-CONFIRMED (checked, found matching or correctly justified)

- **`compute_bridge_transition` Enter on cliff-jump (-4) and the signed-i8 wrapping arithmetic** — `movement_bridge.rs:88-91` uses `i8`/`wrapping_sub(4)`; matches gamemd's `MOVSX ... SUB EAX,0x4` signed level diff (DRIVE_SHIP §4.1 `MOVSX EAX, byte [ESI+0x11b]`). The `-4` level diff for the descending-onto-deck case is correct. (D1 covers the residual classification concern.)
- **A* closed-list ground-vs-bridge layer threshold `>= 2`** — `core.rs:411 is_at_bridge_level` and `movement_occupancy.rs:152` both use `>= 2`, matching gamemd's A* `< 2` site at 0x429e7d (documented at core.rs:400). This ONE of the three gamemd thresholds matches (the other two diverge — see D5).
- **CLIFF_HEIGHT_THRESHOLD `>= 3` with bridge-ramp exemption** — `mod.rs:98` + `movement_step.rs:1097-1099` treats `diff >= 3 && !is_bridge_ramp` as cliff; consistent with gamemd's `>= 3`-level cliff handling and the ramp exemption (CheckBridgeTraversal allows the diff-1 ramp and diff-4 entry).
- **Bridge-deck-level = ground + 4 (levels) as the load-bearing invariant** — `BRIDGE_DECK_LEVEL_DELTA = 4` matches gamemd's "bridges sit exactly 4 height-levels above anchor ground" invariant (DriveComputeBridgeZOffset doc-comment, verified the `* 4`). The LEVEL count is right; the UNIT and rounding are the drifts (D4).
- **DropPod and Tunnel locomotors NOT implemented** — correct per WALK_DROPPOD_TELEPORT doc §3 (zero INI bindings to CLSID 4A582745 / 4A582743). TS-dead; not a Rust gap.
- **Occupancy is layer-tagged Ground/Bridge with independent per-layer ordering** — `occupancy.rs:54-90` (`iter_layer`, `blockers`, `infantry`, independent order via Prepend/Append) correctly mirrors gamemd's `cell.FirstObject(+0xE4)` (ground) vs `cell.AltObject(+0xE8)` (bridge) dual lists. `movement_step.rs:1139,1187` selects the layer from `projected_on_bridge_state`. The Ground/Bridge tagging requirement is met. (JumpJet's own list +0xE0 is air-layer, separate; not in scope here.)
- **JumpJet does NOT join the bridge occupancy list and does NOT add a bridge Z-offset** — AIR_HOVER doc §2.5; Rust JumpJet (`jumpjet_movement.rs`) keeps altitude independent of `position.z`. Correct behavior (the missing layer-SORT tweak is a render-sort concern, AIR_HOVER §6).
- **Hover does NOT add a bridge Z-offset to its destination** — AIR_HOVER doc §3.5; Rust hover has no Z-bump. Correct.

---

## UNCHECKED (could not resolve, with reason)

- **D7 CheckBridgeTraversal branch-equivalence** — verified the gamemd function live and read Rust `compute_neighbor_height`/`check_bridge_traversal`, but did not walk every `+0x11B`/`+0x11C` byte branch on both sides with concrete fixtures to PROVE the Rust pre-classified `transition`/`slope_type` model reproduces the raw-byte gamemd predicate. Structurally different; equivalence plausible but unproven.
- **D8 IsOnBridgeRamp/IsOnBridgeSurface tile-index ranges** — the gamemd predicates are tile-index membership tests over bridge-tile-type globals set at map load. Whether the Rust terrain resolver (outside the audited files) reproduces the exact ranges, including IsOnBridgeRamp's direction-specific (`param_2 ∈ {0,1,3,4}`) endpoint sub-checks, was not traced.
- **D10 ship `90`-lepton assumption** — `g_ShipHeightStep` (and `g_DriveHeightStep`) are BSS/runtime-initialized; the static binary cold-dumps zero (verified). Could not obtain the runtime value to confirm whether Rust's hardcoded `90*4 = 360` matches `ftol(g_ShipHeightStep*4 + 0.5)`. Would need a live debugger session post-map-load.
- **LeptonsPerLevel (`DAT_00ac13c8`) and Set_Height add (`DAT_00ac13bc`) runtime values** — both cold-dump zero (verified `read_memory`); the `*3` and `+=` semantics are verified but the numeric lepton values are runtime-init only. RA2 convention is 256 leptons/level but not confirmed at a binary site this session.
- **Whether the Rust per-tick Z (levels) → render screen lift is exactly 4-levels-worth of pixels** — render screen mapping of `position.z` was not in scope; D11 notes render does not consume these fields at all.
