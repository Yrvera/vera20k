# GRIZZLY_OPPORTUNITYFIRE_VISIBILITY_CLOAK_BRIDGE_FILTERS_GHIDRA_REPORT

## Working Notes

- Target question: For stock MTNK `OpportunityFire=yes` passive acquisition, which exact target-rejection filters apply for visibility, cloaking, bridge layer, hostility/allies, dead/limbo/hidden state, and weapon legality?
- Non-goals: Do not redo target scoring, scanner vtable resolution, OpportunityFire timing, or MTNK weapon range except where needed to anchor filter liveness.
- Evidence needed to mark COMPLETE: YR-live caller path into `TechnoClass__Evaluate_Candidate`, decompile plus assembly ranges for each scoped filter, and Rust-facing acceptance tests.
- Stop conditions: Stop if the filter depends on a broad unknown scoring formula, if a path is TS/fog legacy rather than standard YR, or if Ghidra requires mutating function boundaries.

## Scope Result

COMPLETE for stock MTNK passive-acquire candidate rejection filters in the already-settled scanner chain. The load-bearing filter body is `TechnoClass__Evaluate_Candidate @ 0x006F7CA0`, reached from both `TechnoClass__Greatest_Threat @ 0x006F8DF0` and `TechnoClass__Scan_Cell_For_Target @ 0x006F8960`.

Settled context used without re-investigation: stock MTNK uses `OpportunityFire=yes`; passive scan reaches `Greatest_Threat`; selected weapon range is `[105mm] Range=5`; ranking is threat-score based. This report only isolates acceptance/rejection gates.

## Verified Binary Findings

### 1. YR-live passive scan reaches the candidate filter

Active in YR: Yes.

Evidence:
- `FUN_00743190 @ 0x00743190` ORs selected weapon target flags, then calls `FUN_004D9920` at assembly `0x0074324F..0x0074325C`.
- `FUN_004D9920 @ 0x004D9920` calls `TechnoClass__Greatest_Threat` after optionally folding a local bit into the threat flags.
- `TechnoClass__Greatest_Threat @ 0x006F8DF0` calls `TechnoClass__Evaluate_Candidate` from the all-techno path and cell-scan path. Xrefs to `Evaluate_Candidate`: `0x006F92AC`, `0x006F9C2B`, `0x006F9D76`.
- `TechnoClass__Scan_Cell_For_Target @ 0x006F8960` calls `Evaluate_Candidate` at `0x006F8C00` after walking the cell's object list.

Rust implication: stock MTNK passive acquisition should use the same candidate filter surface as combat target acquisition, not a special Grizzly-only shortcut.

### 2. Dead, null, limbo, and off-map/visibility-state candidates are rejected before scoring

Active in YR: Yes.

Evidence:
- `Evaluate_Candidate` rejects failed weapon-fire precheck result `5` at `0x006F7CDB..0x006F7CF1`.
- Candidate null or `target+0x81 != 0` rejects at `0x006F7D88..0x006F7D98`; sibling docs identify `+0x81` as `InLimbo`.
- `target+0x6C == 0` rejects at `0x006F7D9E..0x006F7DA3`; this is the zero-health/dead object gate in this function.
- `target+0x3D5 == 0` rejects at `0x006F7DF1..0x006F7DF9`; if set, `MissionClass__GetMissionTimerEntry` and `vtable+0x1C8` add further reject branches at `0x006F7DFF..0x006F7E1E`.

Rust implication: existing Rust already skips `health.current == 0`, `dying`, and `passenger_role.is_inside_transport()` in `src/sim/combat/combat_targeting.rs`, but the binary also has explicit `InLimbo`/map-state gates before threat scoring. Passenger-hidden is covered in Rust by `PassengerRole::Inside`, but the exact binary passenger/limbo mapping is inherited from the broader limbo/passenger docs rather than rederived here.

### 3. Fully cloaked enemies require sensor coverage unless they share owner

Active in YR: Yes.

Evidence:
- `Evaluate_Candidate` tests `target+CloakState(0x220) == 2` at `0x006F7DA9..0x006F7DAF`.
- It gets the scanner owner's house index from `attacker->Owner+0x30`, resolves the target cell, calls `CellClass__SensorCountForHouse @ 0x004870D0`, and rejects at `0x006F7DB1..0x006F7DEB` when sensor count is false and target owner differs from attacker owner.
- `CellClass__SensorCountForHouse @ 0x004870D0` returns `cell+0x7C[house] > 0`, confirming this is sensor detection, not ordinary shroud visibility.

Rust implication: fog visibility is not enough for cloaked targets. A cloaked enemy in an otherwise visible cell must be rejected unless that cell has a positive sensor count for the attacking house, with same-owner exception.

### 4. Local-human visibility/discovery gates are separate from standard YR fog

Active in YR: Conditional.

Evidence:
- `Evaluate_Candidate` calls `HouseClass__IsHumanPlayer` for the attacker's owner at `0x006F81BE..0x006F81C9`.
- If that house is human and candidate bytes `+0x41A` and `+0x41B` are both zero, the standard single-player branch rejects non-RTTI-2 targets at `0x006F81CD..0x006F81F4`.
- `CellClass__IsVisibleToHouse @ 0x004870B0` exists and reads `cell+0x78`, but it is not directly called by this `Evaluate_Candidate` branch. The target gate here is object-level visibility/discovery state.
- AGENTS caution applies: standard YR does not use TS-style fog of war by default; explored cells remain visible unless fog is explicitly enabled.

Rust implication: current `FogState::is_cell_visible` in `src/sim/combat/combat_targeting.rs` is a partial approximation. Do not use TS fog semantics as the default Grizzly passive-acquire gate; model local-human object visibility/discovery separately when that system exists.

### 5. Bridge-layer mismatch is a candidate rejection only when both cells are structural bridge cells

Active in YR: Yes.

Evidence:
- `Evaluate_Candidate` resolves attacker and target cells from coordinates at `0x006F8682..0x006F86D5`.
- It tests attacker cell `CellClass+0x140 & 0x100` at `0x006F86DA..0x006F86E0`.
- Only if the target cell also has `0x100` at `0x006F86E2..0x006F86E8` does it compare attacker `OnBridge` (`+0x8C`) with target `OnBridge` (`+0x8C`) at `0x006F86EA..0x006F86F8`; mismatch jumps to reject.

Rust implication: do not make bridge mismatch a generic Z or deck-level filter for all targets. The binary's scoped gate is: both current cells are structural bridge cells and `OnBridge` differs.

## Additional Filter Notes

- Hostility/allies: `Greatest_Threat` and `Scan_Cell_For_Target` both prefilter via `HouseClass__Is_Ally_ByObject`, then `Evaluate_Candidate` repeats ally/special-case checks around `0x006F7F22..0x006F8001` and `0x006F85AB..0x006F866D`. For stock MTNK passive attack acquisition, ordinary allies are rejected; special repair/capture/heal-style ally branches are not stock Grizzly weapon behavior.
- Weapon legality: the first hard gate is `vtable+0x3BC(target, weapon)` returning fire-error `5` at `0x006F7CDB..0x006F7CF1`; later range and `vtable+0x3A8` checks appear at `0x006F8049..0x006F81B8`. This supports routing through generic weapon-selection/verses/range code rather than a Grizzly branch.
- Cell-list scan: `TechnoClass__Scan_Cell_For_Target @ 0x006F8960` reads `cell+0xE8` first and falls back to `cell+0xE4` when empty. The handoff-relevant layer rejection remains the later `Evaluate_Candidate` `OnBridge` check above.

## Implementation Handoff

1. Verified behavior: cloaked target acceptance requires `CloakState == 2` plus positive `CellClass::SensorCountForHouse(attacker_house)` unless same owner -> Rust delta: current targeting has fog visibility but no sensor/cloak target gate -> affected surface: `src/sim/combat/combat_targeting.rs`, future cloak/sensor components, `FogState`/vision surfaces -> acceptance scenario: a stock MTNK moving by a cloaked enemy in visible cell but with no friendly sensor does not acquire; adding a sensor count for the attacker house allows acquisition -> proposed test name: `grizzly_passive_scan_rejects_unsensed_cloaked_enemy` -> risk: using ordinary visibility for cloaked targets makes stealth units visible to Grizzly OpportunityFire.

2. Verified behavior: local-human target discovery uses object-level bytes `+0x41A/+0x41B` and is not a direct `CellClass__IsVisibleToHouse` call in `Evaluate_Candidate` -> Rust delta: current `FogState::is_cell_visible` likely over-couples targeting to cell visibility and TS fog state -> affected surface: `src/sim/combat/combat_targeting.rs`, `src/sim/world/mod.rs` fog/vision update, future per-object discovered/visible flags -> acceptance scenario: standard-YR explored-but-not-currently-visible target should follow RA2 object discovery semantics, not TS fog blocking, while undiscovered local-human object remains rejected -> proposed test name: `grizzly_passive_scan_uses_object_discovery_not_ts_fog_by_default` -> risk: turning default YR into TS fog changes normal play acquisition.

3. Verified behavior: bridge mismatch rejection requires both attacker and target cells to have structural bridge bit `0x100`, then compares `OnBridge` -> Rust delta: current target acquisition has range bridge LOS checks but no candidate-layer reject in `combat_targeting.rs` -> affected surface: `src/sim/combat/combat_targeting.rs`, `src/sim/game_entity.rs::on_bridge`, `ResolvedTerrainGrid` bridge facts -> acceptance scenario: two Grizzlies in the same bridge cell family with one on deck and one under/ground are not passively acquired when both cells are bridge cells; non-bridge cells do not use this shortcut -> proposed test name: `grizzly_passive_scan_rejects_bridge_cell_onbridge_mismatch` -> risk: rejecting all different-Z targets will over-block non-bridge combat.

## Focused Rust Scan

- `acquire_best_target` -> `src/sim/combat/combat_targeting.rs:167` -> existing filters: alive, `dying`, `PassengerRole::Inside`, hostile/friendly, `FogState::is_cell_visible`, weapon compatibility, range, nearest/class/stable-id ranking -> likely ownership: `sim/combat`.
- Fog/vision threading -> `src/sim/world/mod.rs:852`, `src/sim/world/mod.rs:1224`, `src/sim/combat/mod.rs:1138` -> current targeting gets `Some(&self.fog)` each tick -> likely ownership: `sim/world` scheduling plus `sim/combat`.
- Bridge state data -> `src/sim/game_entity.rs:139`, `src/sim/game_entity.rs:144`, `src/sim/game_entity.rs:480` -> `on_bridge` exists; targeting does not yet use it as a candidate filter -> likely ownership: `sim/combat` with terrain query.
- Existing adjacent tests -> `src/sim/combat/combat_tests.rs` has fog visibility tests around `tick_combat_with_fog`; `src/sim/combat/in_range.rs` has bridge LOS gate tests; `src/sim/combat/combat_aoe.rs` has bridge layer damage tests. No targeted OpportunityFire cloak/bridge candidate tests found.

## Negative Facts / Do Not Do

- Do not treat ordinary `FogState::is_cell_visible` as a substitute for sensor detection of fully cloaked enemies. Evidence: cloaked-target branch calls `CellClass__SensorCountForHouse @ 0x004870D0`; Active in YR: Yes.
- Do not apply TS fog-of-war as the default Grizzly passive-acquire blocker. Evidence: `Evaluate_Candidate` local-human branch uses object bytes `+0x41A/+0x41B`, and AGENTS notes standard YR fog is off by default; Active in YR: Conditional.
- Do not reject bridge targets solely because Z/deck levels differ. Evidence: bridge reject requires both cells `+0x140 & 0x100` and then `OnBridge` mismatch at `0x006F86DA..0x006F86F8`; Active in YR: Yes.
- Do not allow passengers/limbo objects just because they remain in an entity map. Evidence: `target+0x81 != 0` rejects at `0x006F7D88..0x006F7D98`; Active in YR: Yes.
- Do not bypass generic weapon legality for MTNK. Evidence: `Evaluate_Candidate` hard-rejects `vtable+0x3BC` fire-error `5` before scoring at `0x006F7CDB..0x006F7CF1`; Active in YR: Yes.

## Remaining Uncertainty

- Exact lifecycle and best semantic name of target `+0x3D5` in this filter was not re-investigated. The branch is verified as a rejection gate, but sibling docs disagree between `HasSight`, in-playfield, and visibility-related meanings.
- The full local-human `+0x41A/+0x41B` lifecycle was not traced here; this report only verifies their use in `Evaluate_Candidate`.
- Special allied repair/heal/capture branches inside `Evaluate_Candidate` were not expanded because they are not stock MTNK attack behavior.

## Stale-Doc Replacement Wording

Suggested replacement for `docs/research/TARGET_ACQUISITION_GHIDRA_REPORT.md` lines around the old bullet 8:

> `Evaluate_Candidate @ 0x006F7CA0` rejects candidates with target byte `+0x3D5 == 0`, and then applies a mission-timer / `vtable+0x1C8` gate before continuing. This scoped evidence does not prove the old "underground probabilistic detection" wording. Treat the exact lifecycle/name of `+0x3D5` as unresolved here; do not implement it as an underground random detection rule without a dedicated verification.

Suggested replacement for the bridge bullet:

> The bridge-layer target filter resolves attacker and target cells, requires both cells to have `CellClass+0x140 & 0x100`, and only then rejects when attacker `OnBridge` differs from target `OnBridge`. It is not a generic "same bridge" or all-Z-level mismatch test.

## Status

COMPLETE.
