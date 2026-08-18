# Garrison Lifecycle Implementation Design

## Goal

Bring civilian `CanBeOccupied` garrison lifecycle closer to active Yuri's Revenge `gamemd.exe` behavior using the verified 2026-05-27 swarm reports as the implementation constraint set.

## Architecture Context

Civilian garrison behavior is currently split across simulation placement, passenger ownership, combat/ejection, and app-layer rendering.

- `src/sim/passenger.rs` owns boarding and unloading. Today `tick_boarding` appends the passenger and immediately transfers neutral/special buildings to the infantry owner, while `tick_unloading` reverts to `garrison_original_owner` or Neutral.
- `src/sim/production/production_sell.rs` owns sell/destruction garrison ejection. Today `sell_survivor_positions` builds a sorted outside perimeter list, and `eject_garrison_passengers_at_edges` chooses fresh cells per occupant and scatters with `next_u32() % 8`.
- `src/sim/world/mod.rs` reports `ownership_changed` from the passenger phase and refreshes fog/control state from owner changes.
- `src/app_instances/shp.rs` owns building body frame and active anim emission. Today `building_frame_index` applies the occupied-frame formula for every `CanBeOccupied` structure, and `ActiveAnimGarrisoned` is emitted as a continuous garrisoned overlay.
- `src/app_building_anim.rs` owns garrison muzzle flash instances from shot events. Today `tick_garrison_muzzle_flashes` is shot-triggered, which is correct in shape, but uses hardcoded frame timing rather than native `AnimClass` metadata.

The design must keep the existing project boundary: `sim/` owns deterministic gameplay state and events; render/app code consumes sim state and rules/art metadata. `sim/` must not depend on render, UI, audio, sidebar, or net.

## Impact Analysis

Primary files likely touched by implementation:

- `src/sim/passenger.rs`: remove immediate garrison owner mutation from boarding/unloading; keep passenger vector mutation; add or call building reconciliation.
- `src/sim/world/mod.rs`: schedule garrison building reconciliation in native-like object order and report ownership changes from that reconciliation, not from boarding.
- `src/sim/production/production_sell.rs`: replace garrison ejection scan/reuse/fallback/scatter approximation.
- `src/sim/movement/*` or a new sim-local scatter helper: encode reusable infantry scatter semantics needed by ejected garrisons.
- `src/app_instances/shp.rs`: gate occupied building frame formula by native BState-like state and constrain garrisoned anim slot replacement.
- `src/app_building_anim.rs` plus `src/rules/art_data.rs`: use `AnimType` timing/lifecycle metadata for shot-triggered `OccupantAnim`.
- Tests near the above modules: update stale tests that currently assert body frame `2` for healthy occupied civilian buildings or distinct ejection cells per occupant.

Risk areas:

- Tick ordering and determinism: owner timing is not "always immediate" or "always next tick"; it depends on the next building reconciliation turn after occupant append.
- Fog/control visibility: moving owner changes from boarding to reconciliation changes when vision and ownership-visible UI update.
- RNG consumption: ejection placement must not consume scatter RNG directly; scatter RNG is conditional and uses `RandomRanged(0,4)`.
- Rendering parity: body-frame and `ActiveAnimGarrisoned` fixes remove currently visible effects that were previously treated as expected by tests.
- Existing research docs/tests are partially stale. The 2026-05-27 reports supersede older wording about per-occupant edge cells, continuous muzzle flashes, and generic occupied overlays.

## Chosen Approach

Use a verified civilian `CanBeOccupied` lifecycle pass with explicit sub-scope boundaries:

1. Keep boarding as occupant vector mutation, not owner transfer.
2. Add a persistent live-object-order surrogate for the garrison slice, separate from `EntityStore` sorted IDs, and run boarding/reconciliation against that order so same-frame vs next-frame owner timing is explicit.
3. Replace garrison sell/destruction ejection with a `SellBuilding`-specific mode that chooses one exit coordinate, reuses it for reverse occupants, and preserves caller-specific no-exit behavior.
4. Introduce a reusable sim-local infantry scatter helper with the verified ejected-infantry ordering and RNG contract.
5. Fix app-layer occupied building visuals for the verified healthy no-BState case and constrain garrisoned active anims to native live-slot semantics. Full damaged/yellow/red BState writer parity remains a named follow-up unless implemented in the same patch.
6. Keep muzzle flashes shot-triggered, but drive their timing/lifetime from art `AnimType` data rather than hardcoded `67ms`.

This is the recommended path because it closes the verified parity holes without forcing unrelated tank bunker work or a full engine-wide `LogicClass` scheduler migration in the same change.

## Tiny-Detail Ledger

- `SellBuilding` resets current garrison fire index `Building+0x69C` at entry. Source: `GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`, `0x00457DEB`.
- `SellBuilding` increments `g_MapEditorMode` before the occupant loop and decrements it after the loop. Source: `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`, section 4.1. Rust has no current equivalent; implementation must either model the side effect or document why no Rust systems read the equivalent state before claiming full ejection parity.
- Exit scan uses the first occupant's cell-entry predicate for all candidates, with args `CellClass*, -1, -1, 0, 1`, and accepts return `0`. Source: `GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`, `0x00457E77..0x00457E99`.
- Exit scan order is east column from SE to NE, south row from SE to SW, north row from `(ox, oy-1)` to NE, then west row from `(ox-1, oy)` to SW. SE/NE/SW can be tested twice; NW outside corner is skipped. Source: same report, section 3.3.
- One accepted coordinate is selected once before the occupant loop and reused for every occupant in reverse vector order. Source: same report, `0x00458060..0x0045819E`.
- If one occupant's `Unlimbo` at the chosen coordinate fails, that occupant is destroyed/removed and earlier occupants are still attempted. Source: same report, `0x004580BD..0x0045819E`.
- No-exit fallback is caller-mode dependent: destruction/red-HP zero-argument callers take `SpawnUnitsWithParachute(0)` null removal; normal player sell uses inside-foundation fallback `(ox+W-1, oy+H-1)`. Source: same report, section 3.4.
- Successful ejection order is `Unlimbo` succeeds, archive target clear `+0x3C8(0)`, direct scatter virtual `+0x174(building center, true, true)`, optional parent cleanup, then only if gated, mission `0xF`. Source: `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`, section 4.1; first-argument gate clarified by `GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`.
- Ejected infantry scatter may return before RNG due to locomotor/timer/type/table gates, including `DAT_007EAF7C[index*4] == 0` for non-exempt sequence states. Source: `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`, section 4.2.
- Ejected infantry directional scatter uses scenario `RandomRanged(0,4)`, inclusive, after gates. It is not raw `% 8` and not `RandomRanged(0,7)`. Source: same report, sections 4.2 and 4.3.
- Scatter queues mission `2` before setting destination. A later `SellBuilding` mission `0xF` is first-argument gated; the direct player sell/destruction/red-HP callers verified by the exit-scan report pass first argument `0`, so this design must not queue `0xF` for those modes unless a live nonzero caller is separately proven. Source: `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`, `0x0051D6BE..0x0051D6E0`; `GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`, section 3.5.
- Ownership transfer is not inside `AddGarrisonOccupant`; it is inside `BuildingClass::Update -> CheckAutoSellOrCivilian`. Source: `CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER_GHIDRA_REPORT.md`, findings 2-5.
- Same-frame vs next-frame owner transfer depends on live forward object-vector order: if infantry entry occurs before the building update in the same pass, transfer can be same global frame; otherwise it waits until the next building update. Source: same report, lifecycle answer.
- Occupied transfer uses first occupant slot owner; empty revert resolves the Civilian-side house and does not use a per-building original-owner field. Source: same report, findings 5-6.
- `GetCurrentFrame` only applies the `CanBeOccupied` occupied body-frame formula when `Building+0x534 != 0`; healthy occupied static civilian garrisons normally render body frame `0`. Source: `GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md`, finding 1.
- Under the BState-gated formula, occupied civilian yellow remains frame `2`, and occupied civilian red computes `3` then collapses to frame `1` when `TechLevel == -1`. Source: same report, finding 2.
- `FUN_00458330` swaps existing live anim slots to empty/occupied/damaged variants; it does not create a generic occupied overlay when the slot pointer is null. Source: same report, finding 4.
- Stock static garrisons such as `CAGAS01`, `CABARN02`, and `CABUNK01` have no active generic occupied overlay slot; stock `CAWASH19 ActiveAnimGarrisoned` is inactive in standard YR because its `CanBeOccupied` flags are commented out. Source: same report, finding 5.
- Ordinary occupied garrisons do not have a continuous `BuildingClass::Update` muzzle flash. The 24-frame branch is chrono/temporal sparkle gated by warp flags and uses `[General] ChronoSparkle1`. Source: `CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md`, sections 1-3.
- Actual occupied shot flashes are created by `TechnoClass::Fire_At` with `WeaponType+0x110 OccupantAnim`, not by the chrono sparkle branch. Source: same report, section 3.4.
- Shot-triggered `OccupantAnim` timing and deletion follow generic `AnimTypeClass`/`AnimClass`: `Rate=` maps to `900 / Rate`, defaults apply when stock UC anims omit timing keys, and `End/Loop/Next/Shadow` metadata matters. Source: same report, section 3.5.
- `CanDock` entry gates remain part of the broader civilian garrison lifecycle: native uses `CanBeOccupied`, mission/building-state gates, same owner or `MultiplayPassive`, exact count equality against `MaxNumberOccupants`, red HP, and mind-control checks. Current Rust owner-name shortcuts and missing gates are drift. Source: `GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`, sections 3 and 9. This implementation pass may defer entry gates only if it names the resulting active drift.

## Design

### Components

#### 1. Civilian garrison reconciliation

Add a sim-local reconciliation step for `CanBeOccupied` civilian buildings that mirrors `CheckAutoSellOrCivilian` for the scoped owner behavior.

The reconciliation should run as part of the building's update turn, not as part of the infantry boarding commit. It must not use `EntityStore::keys_sorted()` or a per-phase passenger snapshot as if that were native order. The native scheduler is a live forward object vector; the scoped Rust design therefore introduces a persistent `logic_order`/`object_update_order` vector owned by `Simulation`:

- Append new entity ids to `logic_order` at the same spawn sites that insert entities into `EntityStore`.
- Remove or skip dead/despawned ids during the live walk; reload the vector length each loop iteration so newly spawned entities can be observed in the same style as native live-count reloads where the scoped pass needs it.
- Tests may seed `logic_order` directly to create "infantry before building" and "building before infantry" fixtures.
- During the garrison lifecycle walk, when an infantry boarding commit appends an occupant, do not change building owner immediately.
- During the same walk, when the target building's update/reconciliation turn is reached, inspect its current occupant count.
- If the building is the resolved Civilian-side owner and occupant count is positive, transfer to owner of passenger slot `0`.
- If occupant count is zero and the building is not the resolved Civilian-side owner, revert to resolved Civilian-side owner.

This preserves the scoped native rule "next building reconciliation pass after occupant mutation" without claiming that the rest of Rust's phase scheduler is already a full `LogicClass::PerTickUpdate` clone. Save/load reconstruction and exact native insertion indices remain scheduler-level follow-ups; the garrison tests must assert only the relative order contract this design owns.

Implementation should remove `garrison_original_owner` from this civilian-garrison path. If the field is still used by unrelated behavior, leave the field in place but stop reading/writing it for civilian `CanBeOccupied` ownership.

#### 2. SellBuilding ejection mode

Replace the current distance-sorted helper for garrison occupants with a `SellBuilding`-specific scan:

- Build the exact candidate sequence described in the ledger, including duplicate SE/NE/SW corner tests and skipped NW corner.
- Test candidates through a named garrison-exit cell predicate, for example `can_garrison_exit_unlimbo(first_occupant_id, candidate_cell, rules, path_grid, occupancy, entities) -> bool`, which must be documented as the Rust binding for first occupant `Can_Enter_Cell(CellClass, -1, -1, 0, 1) == 0`.
- The initial implementation must not silently reduce that predicate to occupancy-only. If Rust lacks required inputs, the helper must list the missing gates in its doc comment and tests must mark those cases `UNCHECKED`; otherwise this design is not implementation-ready for that sub-slice.
- Select one coordinate before the occupant loop.
- Iterate occupants in reverse vector order.
- Attempt to place every occupant at that same coordinate.
- On individual placement failure, remove/destroy that occupant and continue.
- Clear the building cargo/fire index after native-equivalent cleanup.
- Preserve or explicitly dispose of the `g_MapEditorMode` increment/decrement side effect. If no Rust subsystem reads an equivalent flag, document this as "mode side effect has no modeled consumers" in the helper; do not omit it silently.

The helper should take an explicit caller mode, for example:

- `PlayerSell { queue_0f: false }`: no accepted scan cell falls back to `(ox+W-1, oy+H-1)`.
- `DestructionOrRedHp { queue_0f: false }`: no accepted scan cell removes occupants through the `SpawnUnitsWithParachute(0)` null behavior. If the parachute system is not modeled yet, implement the proven null branch result only, and name it that way.

The `queue_0f` field exists only for future live callers that pass first argument nonzero. It must default false for every direct caller verified by the 2026-05-27 reports.

Avoid reusing generic survivor placement helpers for this path. The native garrison path has different ordering, reuse, and fallback semantics.

#### 3. Reusable infantry scatter helper

Introduce a sim-local infantry scatter operation that can be called by garrison ejection and later reused by other scatter call sites. It should not be render-facing.

For the ejected-garrison use case, the public contract should preserve:

- input: infantry id, threat/source coordinate, two boolean flags equivalent to the direct scatter call's true/true args;
- pre-RNG gates, including sequence/table gate behavior where the necessary state exists;
- directional base from target-current coordinate;
- one scenario RNG `RandomRanged(0,4)` draw only if gates pass;
- mission `2` queued before destination write when a candidate is found;
- no RNG and no destination write if gates return early;
- return value describing whether scatter produced a destination, while allowing caller to apply the later `SellBuilding` mission gate only when the caller mode has `queue_0f=true`.

If some native infantry sequence fields or mission queue internals are not yet represented, the helper should make those gaps explicit in tests and names rather than hiding them behind `issue_direct_move`.

#### 4. Occupied building visual state

Update body frame selection so the occupied `CanBeOccupied` formula is not applied to every healthy occupied civilian structure.

The app-layer body frame decision should require a native BState-like condition before using the occupied formula. Until the full BState writer lifecycle is represented, the immediate parity fix is:

- healthy occupied static civilian garrison with no active BState equivalent renders base frame `0`;
- yellow/red occupied frame tests are blocked until a BState writer/equivalent state is represented, because the 2026-05-27 visual report did not re-trace all writers for `Building+0x534`;
- when a later BState implementation exists, keep the existing formula shape with the civilian red collapse to frame `1`;
- update tests to distinguish formula behavior from rendered healthy output.

For garrisoned active anim variants:

- Stop treating `ActiveAnimGarrisoned` as a generic continuous overlay for any garrisoned building.
- Model native live-slot replacement: a garrisoned variant can replace an already-live slot; it is not created from nothing for static stock civilian buildings.
- Keep stock `CAWASH19 ActiveAnimGarrisoned` inactive in standard YR unless the rules data actually makes the building `CanBeOccupied`.

#### 5. Shot-triggered OccupantAnim timing

Keep the current event-driven shape for garrison muzzle flashes. Do not add a continuous occupied-garrison muzzle system.

Replace hardcoded flash cadence/lifetime with generic `AnimType` timing:

- Resolve the fire event's `occupant_anim` to art data.
- Use native-style `Rate=` conversion and loop/end/default metadata for frame advancement and deletion.
- Preserve shot-triggered creation only on actual fire events.
- Keep chrono/temporal building sparkle rendering separate as a future chrono visual task, not a garrison muzzle task.

### Interfaces / Contracts

- Boarding contract: `tick_boarding` mutates passenger/cargo state and may emit first-occupant events, but does not change building owner for civilian garrisons.
- Reconciliation contract: building update/reconciliation in the persistent `logic_order` walk is the only civilian garrison owner transfer/revert surface for this path.
- Ejection contract: caller mode decides no-exit behavior and `queue_0f`; the accepted exit coordinate is chosen once; occupant loop is reverse order; candidate acceptance is delegated to the named garrison-exit cell predicate.
- Scatter contract: garrison ejection calls scatter through a reusable infantry scatter helper, not direct movement and not raw RNG. The later `0xF` mission is not queued for verified direct callers.
- Render contract: app layer may compute visual frames from sim/rules/art state, but `sim/` remains unaware of render concerns.
- Entry-gate contract: `CanDock` entry validation is a known adjacent lifecycle drift. If not included in the first implementation swarm, it must be split into a separate verified-fix task before claiming the full civilian garrison lifecycle closed.

### Data Flow

```text
garrison lifecycle walk over persistent logic_order
  -> infantry reaches target cell:
       passenger boarding appends occupant and limbos infantry
       no building owner change here
  -> when building update/reconciliation turn runs:
       if occupied Civilian building: ChangeOwner(first occupant owner)
       if empty non-Civilian civilian garrison: ChangeOwner(resolved Civilian)
  -> ownership_changed reports from this step feed fog/control refresh

player sell / destruction / red-HP calls garrison ejection
  -> reset fire index
  -> choose one native SellBuilding exit coordinate or caller-mode fallback
  -> reverse occupant loop
       -> unlimbo/place at same coordinate
       -> on success, clear target and call infantry scatter helper
       -> do not queue mission 0xF for verified direct caller modes
       -> only a future proven nonzero caller mode may queue mission 0xF
       -> on failure, remove occupant
  -> clear cargo

render frame build
  -> body frame uses BState-gated occupied formula only when native gate is active
  -> active anim variants replace live slots only
  -> shot fire events spawn OccupantAnim with AnimType timing
```

### Error Handling

- Missing passenger entity during ejection: skip or remove the stale cargo entry during cleanup; do not panic in release behavior.
- Missing first occupant for scan predicate: no occupant count should have reached ejection; treat as empty and clear cargo/fire index.
- No accepted exit coordinate:
  - player sell uses inside-foundation fallback coordinate;
  - destruction/red-HP removes occupants through the null fallback result.
- Missing `OccupantAnim` art data: no shot flash, matching native null anim behavior.
- Missing BState representation: tests should cover the immediate stock healthy case only. Full yellow/red BState render parity remains blocked until `Building+0x534` writer/equivalent state is implemented.

### Testing Strategy

Simulation tests:

- `garrison_owner_not_changed_during_boarding_call`
- `garrison_owner_transfers_same_frame_when_building_update_after_entry`
- `garrison_owner_waits_next_frame_when_building_update_before_entry`
- `garrison_reconciliation_uses_first_occupant_owner`
- `empty_captured_garrison_reverts_to_civilian_house_not_original_owner`
- `garrison_sellbuilding_scan_order_matches_gamemd_edges_2x2`
- `garrison_sellbuilding_reuses_single_exit_coord_for_all_lifo_occupants`
- `garrison_player_sell_no_exit_uses_inside_foundation_fallback`
- `garrison_destruction_no_exit_uses_null_parachute_removal`
- `garrison_sellbuilding_verified_direct_callers_do_not_queue_0f`
- `garrison_exit_scan_uses_first_occupant_cell_entry_predicate`
- `garrison_sellbuilding_map_editor_mode_side_effect_documented_or_modeled`
- `garrison_ejection_scatter_uses_random_ranged_0_4_after_gates`
- `garrison_ejection_scatter_table_gate_skips_rng_for_verified_direct_callers`
- `garrison_candock_entry_gates_follow_multiplaypassive_and_red_hp` if entry gates are included in this swarm; otherwise this is a required follow-up test.

Rendering/app tests:

- `healthy_occupied_static_civilian_garrison_body_frame_stays_zero_without_bstate`
- `occupied_civilian_garrison_bstate_yellow_uses_frame_two` only after BState writer/equivalent state lands
- `occupied_civilian_garrison_bstate_red_collapses_to_frame_one` only after BState writer/equivalent state lands
- `active_anim_garrisoned_requires_native_live_slot_and_garrisonable_type`
- `garrison_no_ambient_update_muzzle_flash_between_shots`
- `garrison_fire_uses_weapon_occupant_anim_not_chronosparkle1`
- `garrison_occupant_anim_uses_art_rate_and_loop_metadata`

Verification order:

1. Focused unit tests for ejection scan/fallback/reuse.
2. Focused passenger ownership/reconciliation tests.
3. Focused scatter RNG ordering tests.
4. App-layer visual tests for frame/anim timing.
5. One final `cargo check -q` after checking no other cargo/rustc process is active.

## Architectural Decisions

- Civilian `CanBeOccupied` garrisons are the scope. Tank bunkers are excluded because their occupant model is adjacent but distinct and was not part of the 2026-05-27 lifecycle swarm scope.
- The implementation should add only the minimal persistent live-object-order hook needed for this behavior. It must be separate from `EntityStore` sorted iteration and phase snapshots. A full native `LogicClass` scheduler migration remains a broader engine task.
- Garrison ejection gets a dedicated `SellBuilding` path instead of overloading generic survivor placement. The native behavior is too different for a mode-hidden generic helper.
- Infantry scatter should become reusable sim logic, because direct-move ejection is proven wrong and scatter has other native callers.
- Visual fixes stay in app/render-facing modules and consume state; no sim-to-render dependency is introduced.
- Chrono sparkle is explicitly not part of this garrison lifecycle implementation, except as a negative constraint: do not implement it as garrison muzzle fire.
- `CanDock` entry gates and full BState writer lifecycle are now named sub-scopes. The implementation swarm should either include them as separate workers or leave them as explicit active drift, not silently claim the lifecycle is complete.

## Alternatives Considered

### Alternative A: Patch only the highest-visible Rust mismatches

This would fix immediate owner transfer and body frame `2`, but leave ejection scan, scatter RNG/order, no-exit modes, and anim timing for later.

Rejected. The 2026-05-27 reports prove those are active gamemd behaviors, not optional polish. Cutting them would leave named parity drift in normal garrison sell/destruction/fire scenarios.

### Alternative B: Full native `LogicClass` scheduler migration first

This would model all object updates as one live forward vector walk before implementing garrison lifecycle.

Rejected for this pass. It is architecturally attractive long term, but garrison ownership can be implemented with a smaller explicit building reconciliation hook and object-order tests. Blocking all garrison fixes on scheduler migration would leave verified high-visibility drift in place.

### Alternative C: Recommended verified civilian garrison pass

This design. It keeps scope to civilian `CanBeOccupied` lifecycle, includes every verified active-gamemd detail from the 2026-05-27 swarm reports that the chosen implementation owns, and names `CanDock` entry gates plus full BState writer lifecycle as explicit sub-scope decisions rather than hidden omissions.

Accepted by user direction on 2026-05-27.

### Remaining Required Decisions Before Implementation

- Decide whether the first verified-fix swarm includes `CanDock` entry gates. If not, the implementation result must be described as "boarding/ejection/render lifecycle fixes with entry-gate drift remaining."
- Decide whether the first verified-fix swarm includes BState writer/equivalent state. If not, implement only the healthy occupied no-BState frame-0 visual fix and defer yellow/red occupied BState frames.
- Before coding ejection, bind the garrison-exit predicate to a concrete Rust function and list any missing `Can_Enter_Cell` inputs as `UNCHECKED`.
