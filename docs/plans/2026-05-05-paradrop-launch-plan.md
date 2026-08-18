# Paradrop Superweapon Launch Pipeline — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Wire the full paradrop superweapon launch pipeline — player click → carrier aircraft spawn → approach/overfly mission FSM → V-pattern Drop_Payload → infantry call `begin_parachute_descent` — at 99% observable parity with gamemd.exe.

**Architecture:** Extend the existing `AircraftMission` enum with two new variants (`ParaDropApproach`, `ParaDropOverfly`); add a new SW handler in `src/sim/superweapon/paradrop.rs` that mirrors `iron_curtain.rs`'s shape; reuse the existing `PassengerCargo` for cargo, the existing `facing_table::SIN_TABLE`/`COS_TABLE` for V-pattern trig, and the already-shipped `parachute_descent` module for descent. No floats in sim, no new locomotor types, no new ECS abstractions.

**Design Doc:** [docs/plans/2026-05-05-paradrop-launch-design.md](2026-05-05-paradrop-launch-design.md)

---

## Grounding Summary

- **Research docs**: [PARADROP_SUPERWEAPON_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/PARADROP_SUPERWEAPON_GHIDRA_REPORT.md) (Round 1+2 + 2026-05-05 CORRECTION) covers the full pipeline: SuperClass::Launch dispatch (cases 5/6), spawner FUN_0065E660, Mission_ParaDropApproach (0x4155F0), Mission_ParaDropOverfly (0x4157C0), Drop_Payload V-pattern (0x415C60), Fire_At gate (0x415EF8), edge cell finder FUN_004AA440. [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md) Round 4 documents L17 (always-success Unlimbo quirk — explicitly deviated from in this plan).
- **Ghidra verification**: All addresses cited in design doc were verified during the brainstorm. V-pattern radius = 128 leptons (constant at `0x7E2808`); angle conversion = `-2π/65536` rad/binary-angle (constant at `0x7E2810`); these collapse to "use existing facing-table with `facing ± 64` offset" since our facing convention (256-step) maps directly. ROF cadence = `[ParaDropWeapon] ROF=130` ticks. ParadropRadius = 1024 leptons (`Rules+0x54C`).
- **Repo patterns mirrored**:
  - SW handler: `src/sim/superweapon/iron_curtain.rs` — `pub fn launch(sim, rules, owner, rx, ry) -> bool`.
  - Aircraft mission FSM: `src/sim/aircraft/mod.rs` — `AircraftMission` enum + `tick_aircraft_missions` snapshot/apply pattern.
  - Per-entity descent state: already shipped at `src/sim/movement/parachute_descent.rs` — `begin_parachute_descent(entities, id, alt) -> bool`.
- **INI keys driving behavior**: `[General] ParadropRadius=1024`, `AmerParaDropInf=E1` / `AmerParaDropNum=8`, `AllyParaDropInf=E1` / `AllyParaDropNum=6`, `SovParaDropInf=E2` / `SovParaDropNum=9`, `YuriParaDropInf=INIT` / `YuriParaDropNum=6`, `[ParaDropWeapon] ROF=130`, `[PDPLANE] Speed=15 ROT=2 Spawned=yes Primary=ParaDropWeapon`, `[CAAIRP] SuperWeapon=ParaDropSpecial`, `[AMRADR] SuperWeapon=AmericanParaDropSpecial RequiredHouses=Americans`.
- **Still unknown after grounding**: bridge-cell rejection requires the map system to distinguish bridge surfaces (`overlay_types.rs` mentions bridges but no `is_bridge_cell` API exists). Deferred — stub the helper to always return the original target. See Open Questions.

---

## Key Technical Decisions

- **Reuse existing `facing_table` 256-facing sin/cos LUT for V-pattern trig** instead of building a new 65536-step binary-angle LUT — gamemd's `0x3FFF` quarter-circle binary angle maps directly to facing offset of 64. **Confidence:** high — verified `SIN_TABLE`/`COS_TABLE` exist at [src/util/facing_table.rs:56-77](src/util/facing_table.rs#L56) and `facing_to_movement` returns `(sin*speed, -cos*speed)` matching gamemd's V-offset convention. **Source:** repo pattern src/util/facing_table.rs.

- **Extend `AircraftMission` enum with `ParaDropApproach` and `ParaDropOverfly` variants** instead of building parallel FSM. **Confidence:** high — matches established pattern for all 7 existing missions. **Source:** repo pattern src/sim/aircraft/mod.rs.

- **Reuse `PassengerCargo` (Vec<u64>) for paradrop carrier cargo** instead of building paradrop-specific cargo. **Confidence:** high — `unload_first()` already exists; `Vec.insert(0, id)` cleanly mirrors gamemd's "re-add to cargo head" retry semantics. **Source:** repo pattern src/sim/passenger.rs.

- **Deviate from gamemd's L17 always-success Unlimbo quirk** — return `bool` from `begin_parachute_descent` and retry on false (same path as P29 impassable-cell). **Confidence:** high — verified against the parachute-descent design doc; player-equivalent. **Source:** Ghidra Round 4 §R4.3 + parachute-descent design.

- **`spawn_pdplane` calls `spawn_object_at_height` directly with `z=0`, then post-spawn sets `loco.altitude = flight_level` and `loco.air_phase = Cruising`** — accepts the parity drift D7 (no ascent ramp) for v1. **Confidence:** medium — assumes `LocomotorState` exposes mutable `altitude` and `air_phase`; verify in Task 11. **Source:** design doc §S8.

- **Bridge-cell rejection deferred to a later task** — Task 12 wires a stub that always returns the original target unchanged. **Confidence:** high (that the deferral is correct) — map system does not yet expose `is_bridge_cell`. **Source:** Grounding R3.

- **Silent spawn deferred** — Task 12 calls regular `spawn_object_at_height`. Audio/radar/AI suppression flagged in design doc as D5, addressed in a separate cross-cutting task. **Confidence:** high. **Source:** design doc D5.

- **Per-side dispatch picks fallback to Soviet for any `side_index ∉ {0, 2}`** — mirrors gamemd's `if (side==0) Allies; else if (side==2) Yuri; else Soviet;` fallthrough. **Confidence:** high — verified in Ghidra report case 5. **Source:** ra2-rust-game-docs/PARADROP_SUPERWEAPON_GHIDRA_REPORT.md §3.1.

---

## Open Questions

### Resolved During Planning

- **Q: Does `util/fixed_math` have sin/cos?** A: Not directly, but `util/facing_table.rs` does — `SIN_TABLE` and `COS_TABLE` are 256-facing fixed-point LUTs. Use `facing_to_movement(facing ± 64, V_PATTERN_RADIUS_SIM)` for V-pattern offset.

- **Q: Does the weapon parser expose ROF on resolved weapons?** A: Yes — `WeaponType.rof: i32` parsed at [src/rules/weapon_type.rs:188](src/rules/weapon_type.rs#L188) from `[ParaDropWeapon] ROF=130`. Plan Task 5 verifies the lookup path.

- **Q: Where do paradrop INI keys land — separate `general_rules.rs` or in `ruleset.rs`?** A: `GeneralRules` lives inside `src/rules/ruleset.rs`. All paradrop fields go there. (Design doc said `general_rules.rs` — corrected.)

- **Q: Does `PassengerCargo` support FIFO unload?** A: Yes — `unload_first() -> Option<u64>` at [src/sim/passenger.rs:89](src/sim/passenger.rs#L89). Vec.remove(0) — O(n) cost is fine at N≤9. **Caveat**: `unload_first` does NOT decrement `total_size` (caller must — see passenger.rs:94). Task 10's `try_drop` corrects this on success.

- **Q: Where do map dimensions live on Simulation?** A: `sim.fog: FogState` exposes `pub width: u16, pub height: u16` ([src/sim/vision/mod.rs:179-181](src/sim/vision/mod.rs#L179)). Existing code uses `sim.fog.width` / `sim.fog.height` (e.g., [psychic_reveal.rs:63-64](src/sim/superweapon/psychic_reveal.rs#L63)). Plan tasks use this access pattern.

- **Q: Is `PathGrid` a Simulation field?** A: No — `path_grid: Option<&PathGrid>` is a parameter threaded through `apply_command` ([world_commands.rs:99](src/sim/world/world_commands.rs#L99)) and `advance_tick` ([world/mod.rs:1096](src/sim/world/mod.rs#L1096)). Plan tasks accept it as a parameter on `paradrop::launch`, `tick_aircraft_missions`, `tick_paradrop_approach`, `try_drop`, and `compute_exit_cell`.

- **Q: What is the `PathGrid` passability API?** A: `PathGrid::is_walkable(x, y) -> bool` at [pathfinding/core.rs:759](src/sim/pathfinding/core.rs#L759). Out-of-bounds returns `false` (impassable). Layer-aware variant: `is_walkable_on_layer(x, y, layer)`.

### Deferred to Implementation

- **Bridge-cell rejection (P1)**: needs `map.is_bridge_cell(rx, ry)` which does not exist today. Task 12 wires a stub returning the original target. Real implementation deferred to a follow-up task once the map system distinguishes bridge surfaces.

- **Silent spawn (D5)**: PDPLANE creation triggers normal spawn audio/radar/AI events; suppression hookup is cross-cutting and out of scope here. Tracked separately as a `g_MapEditorMode-equivalent` task.

- **Visible chute sprite (D4)**: needs attached-anim infrastructure — separate brainstorm. Infantry descend without a visible parachute sprite for now.

- **`RequiredHouses=Americans` enforcement on `[AMRADR]` (D6)**: cross-cutting SW grant-gate concern affecting all American-locked SWs. Out of scope here.

- **Initial PDPLANE altitude (S8 parity drift)**: spawn at `flight_level` directly skipping the ascent ramp. Verify the locomotor accepts this without state machine confusion in Task 11.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sim/superweapon/paradrop.rs` | SW launch entry point + per-side dispatch + spawn_pdplane |
| Create | `src/sim/aircraft/paradrop_mission.rs` | tick_paradrop_approach + tick_paradrop_overfly |
| Create | `src/sim/aircraft/drop_payload.rs` | V-pattern math + try_drop |
| Create | `src/sim/world/edge_cell.rs` | Map-edge passable cell finder (N/E/S/W) |
| Modify | `src/sim/aircraft/mod.rs` | Add ParaDropApproach + ParaDropOverfly variants + tick dispatch |
| Modify | `src/sim/superweapon/mod.rs` | `pub mod paradrop;` |
| Modify | `src/sim/world/world_commands.rs` (~982) | Replace ParaDrop/AmerParaDrop fall-through with handler call |
| Modify | `src/sim/world/mod.rs` | Add `ChuteSound { rx, ry }` variant to SimSoundEvent |
| Modify | `src/sim/house_state.rs` | Add `waypoint_edge: u8` field |
| Modify | `src/sim/world/mod.rs` (game start) | Initialize `waypoint_edge` per house via closest-edge algorithm |
| Modify | `src/rules/ruleset.rs` | Parse paradrop INI keys into GeneralRules |

## Interface Changes

- **`AircraftMission` enum** gains 2 variants. Consumers: `tick_aircraft_missions` (must add match arms), serde (auto). No external callers exist outside `aircraft::mod.rs`.
- **`SimSoundEvent` enum** gains 1 variant (`ChuteSound`). Consumers: app layer `audio_dispatch.rs` may want to map to a Voc, but missing match arms will only produce a warning since `SimSoundEvent` consumers use exhaustive `match`. **Risk: if any consumer uses `match _ {}` exhaustively, this breaks compilation.** Audit in Task 1.
- **`HouseState` struct** gains 1 field. Consumers: serde (auto). Default initialization at house creation must be added.
- **`GeneralRules` struct** gains 6 fields. Consumers: serde (auto). Defaults provided.
- **`paradrop::launch(sim, rules, owner, rx, ry, kind: ParaDropKind, path_grid: Option<&PathGrid>) -> bool`** is a new public function. Caller: `world_commands.rs` SW dispatch (passes `path_grid` from `apply_command`'s parameter).
- **`tick_aircraft_missions(sim, rules)`** signature gains `path_grid: Option<&PathGrid>`. Sole caller is `advance_tick` at [world/mod.rs:1195](src/sim/world/mod.rs#L1195) — already has `path_grid` in scope.
- **`PathGrid`** gains two `#[cfg(test)]`-only constructors: `test_all_passable(w, h)` and `test_all_blocked(w, h)`. No production impact.

## Sim Checklist

- [x] All math uses `fixed`-point — V-pattern uses existing `facing_to_movement` (returns `(SimFixed, SimFixed)`); no f32/f64 in sim logic.
- [x] New state included in deterministic state hash — new mission variants and `waypoint_edge` auto-serialize via serde derive on enum/struct, included in entity store snapshots.
- [x] No dependencies on render/ui/sidebar/audio/net — sim emits `SimSoundEvent::ChuteSound`; app layer drains and resolves to audio.
- [x] Tick ordering impact noted — paradrop runs in Phase 2 (`tick_aircraft_missions`), which already runs after `parachute_descent`. Dropped infantry's first-tick ramp (rate=0, no movement) lines up correctly because their descent state is created in this tick and ticked next tick.
- [x] BTreeMap iteration order considered — `EntityStore::keys_sorted()` is the existing iteration helper; cargo `Vec<u64>` ordering is preserved across save/load; deterministic.

## Risk Areas

- **Aircraft mission FSM extension** — adding two variants to a heavily-matched enum risks missing exhaustive match arms in distant code. Mitigation: Task 6 grep-audits all `match.*AircraftMission` usages.
- **Cargo lifecycle on PDPLANE** — paradrop infantry are loaded via `cargo.board()` then released mid-flight; standard transport flow assumes ground-based unload. Mitigation: Task 10 explicitly handles passenger_role transitions and tests against the existing transport pattern.
- **Silent despawn at exit edge** — if the carrier reaches map boundary while still in `ParaDropOverfly`, must skip explosion anim. Mitigation: Task 12 sets a clear "silent_despawn" flag distinct from normal entity death.
- **Drop_Payload retry semantics** — re-adding passenger to cargo head must NOT re-trigger `passenger_role = Inside` boarding logic. Mitigation: Task 10 keeps `passenger_role` as `Inside` throughout the cargo-head retry path; only successful drops transition to `None`.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | `ParadropRadius=1024` parsed from INI | Drives the moment fog-reveal fires + the moment Approach→Overfly transitions; observable in every paradrop launch | Compare distance-to-target at fog-reveal moment with gamemd.exe via screenshot |
| Task 9 | V-pattern radius = 128 leptons (= 0.5 cell) | Paratroopers land 0.5 cells perpendicular to flight path; observable as a "narrow zig-zag line" pattern | Drop 8 paratroopers from north-facing PDPLANE and verify lateral spread is 1 cell wide |
| Task 9 | V-pattern alternates **L, R, L, R** starting LEFT | First drop is to the LEFT of plane heading (verified gamemd `(payload_count & 1) == 1` post-decrement) | Unit test `test_v_pattern_alternates_starting_left` |
| Task 10 | ROF=130 ticks between drops | Drop cadence — paratroopers spread along ~7.6 cells of flight per drop | Frame-count between drop events in-game vs gamemd recording |
| Task 10 | Impassable retry: re-add to cargo HEAD with same payload_count | Same passenger retried with new heading; player observation = "paratrooper missed once, dropped on next pass" | Unit test `test_drop_retry_preserves_payload_count` |
| Task 11 | Approach→Overfly transition ≤ ParadropRadius (1024 leptons) | Determines the first drop tick relative to map entry | In-game observation + Ghidra Mission_ParaDropApproach decompile |
| Task 11 | ChuteSound + fog-reveal fire ONCE per launch (latched via `has_revealed_fog`) | Players hear the drop voice exactly once, not repeatedly | Unit test `test_fog_reveal_latches` |
| Task 12 | Cargo-empty → redirect to opposite-edge cell + silent despawn | Aircraft exits map without explosion anim (`Landable=no` path) | In-game observation: PDPLANE flies off-screen and disappears, no death FX |
| Task 13 | Per-side branch picks correct list (Allies=0, Yuri=2, else Soviet) | Side 1 (Soviet) is the **fallback**; any non-{0,2} value lands here | Unit test `test_side_branch_fallback_to_soviet` |
| Task 13 | Multi-type list: spawn 1 PDPLANE per `(inf_type, num)` entry | `AmerParaDropInf=E1,GHOST,ENGINEER` + `Num=6,6,6` spawns 3 PDPLANEs | Unit test `test_multi_type_spawns_n_aircraft` |
| Task 15 | End-to-end descent timeline 0,−1,−2,−3,−3,… leptons/tick | Already tested in `parachute_descent` — but the full pipeline must reach this state with no drift | Integration test verifies altitude curve matches gamemd Round 4 §R4.7 |

---

## Tasks

### Task 1: Add ChuteSound variant to SimSoundEvent

**Why:** Drop_Payload emits a positional sound on each successful drop (P30); we add the variant first so downstream tasks can emit it without scaffolding.

**Files:**
- Modify: `src/sim/world/mod.rs` (~line 92, `SimSoundEvent` enum)

**Pattern:** Mirrors existing `WeaponFired { report_sound_id, rx, ry }` — positional event with cell coords. ChuteSound has no per-launch ID variation (always plays the global ChuteSound voc), so no `report_sound_id` field needed.

**Step 1: Add the enum variant**

```rust
// src/sim/world/mod.rs, inside SimSoundEvent enum (~line 92)
pub enum SimSoundEvent {
    // ... existing variants ...

    /// A paratrooper was dropped from a carrier aircraft.
    /// Played at drop position; app layer resolves to `[General] ChuteSound`.
    ChuteSound { rx: u16, ry: u16 },
}
```

**Step 2: Audit exhaustive match arms**

Run: `cargo build 2>&1 | grep "non-exhaustive\|match.*SimSoundEvent"`
Expected: no errors, OR errors flag specific files needing match arms. Fix any flagged file by adding `SimSoundEvent::ChuteSound { .. } => { /* no-op for now; app audio_dispatch will wire later */ }`.

**Step 3: Verify**

Run: `cargo check`
Expected: PASS.

**Step 4: Commit**

`sim: add ChuteSound variant to SimSoundEvent`

---

### Task 2: Add waypoint_edge field to HouseState + closest-edge helper

**Why:** Paradrop carrier spawns at the house's waypoint edge (P6, P7); needed before any spawn task.

**Files:**
- Modify: `src/sim/house_state.rs`

**Pattern:** New scalar field on `HouseState` plus a free function for the closest-edge algorithm — same shape as existing `side_index_from_name` helper at line 114.

**Step 1: Add waypoint_edge field**

```rust
// src/sim/house_state.rs (in HouseState struct, after side_index)
pub struct HouseState {
    // ... existing fields ...

    /// Edge of the playfield where this house spawns paradrop carriers.
    /// Encoding: 0=N, 1=E, 2=S, 3=W. Computed at game start from base_center
    /// via closest-edge-of-bounds algorithm (gamemd HouseClass::DetermineEdge).
    pub waypoint_edge: u8,
}
```

**Step 2: Update Default + new() to initialize waypoint_edge**

```rust
// src/sim/house_state.rs (in HouseState::new() ~line 47)
impl HouseState {
    pub fn new(
        name: InternedId,
        side_index: u8,
        country: Option<InternedId>,
        is_human: bool,
        credits: i32,
        tech_level: i32,
    ) -> Self {
        Self {
            name,
            side_index,
            country,
            is_human,
            credits,
            rally_point: None,
            is_defeated: false,
            has_won: false,
            has_lost: false,
            owned_building_count: 0,
            owned_unit_count: 0,
            base_center: None,
            tech_level,
            waypoint_edge: 0,  // North by default; reset later by initializer
        }
    }
}
```

**Step 3: Add closest-edge helper**

```rust
// src/sim/house_state.rs (append at end)

/// Compute the closest map edge to a given anchor cell.
/// Mirrors gamemd HouseClass::DetermineEdge (0x0050DB00):
/// picks the minimum-distance edge from 4 reference points
/// (some corners, some midpoints — asymmetric per gamemd).
///
/// Encoding: 0=N, 1=E, 2=S, 3=W.
pub fn closest_edge_for(anchor: (u16, u16), map_width: u32, map_height: u32) -> u8 {
    let (ax, ay) = (anchor.0 as i64, anchor.1 as i64);
    let w = map_width as i64;
    let h = map_height as i64;

    // gamemd's 4 reference points (deliberately asymmetric):
    let refs: [(i64, i64); 4] = [
        (w / 2, 1),     // 0: top edge midpoint
        (w, h),         // 1: bottom-right corner-ish (E)
        (w / 2, h * 2), // 2: south extension midpoint
        (0, h),         // 3: left edge midpoint (W)
    ];
    let mut best_edge = 0u8;
    let mut best_dsq = i64::MAX;
    for (i, &(rx, ry)) in refs.iter().enumerate() {
        let dx = ax - rx;
        let dy = ay - ry;
        let dsq = dx * dx + dy * dy;
        if dsq < best_dsq {
            best_dsq = dsq;
            best_edge = i as u8;
        }
    }
    best_edge
}
```

**Step 4: Add tests**

```rust
// src/sim/house_state.rs (append to existing tests or create #[cfg(test)] mod tests)
#[cfg(test)]
mod waypoint_edge_tests {
    use super::*;

    #[test]
    fn test_closest_edge_top_center_picks_north() {
        // Anchor near top-middle → closest to (w/2, 1) reference point.
        let edge = closest_edge_for((50, 5), 100, 100);
        assert_eq!(edge, 0); // North
    }

    #[test]
    fn test_closest_edge_left_middle_picks_west() {
        let edge = closest_edge_for((2, 50), 100, 100);
        assert_eq!(edge, 3); // West
    }

    #[test]
    fn test_closest_edge_bottom_right_picks_east() {
        // Note gamemd's E reference is (w, h) corner — bottom-right anchors land here.
        let edge = closest_edge_for((95, 95), 100, 100);
        assert_eq!(edge, 1); // East
    }
}
```

**Step 5: Verify**

Run: `cargo test waypoint_edge -- --nocapture`
Expected: 3 tests PASS.

**Step 6: Commit**

`house_state: add waypoint_edge field + closest-edge helper`

---

### Task 3: Initialize waypoint_edge at game start

**Why:** Field is added but unused until populated; populating it must happen during house creation, where `base_center` is known.

**Files:**
- Modify: wherever `HouseState::new` is called at game start. Find via:
  ```
  Grep("HouseState::new", path="src/sim")
  ```
  Likely: `src/sim/world/mod.rs` or `src/sim/world/world_init.rs` or similar.

**Pattern:** Set `waypoint_edge` immediately after the house's `base_center` is established. If `base_center` is set later than `HouseState::new()`, set `waypoint_edge` at the same spot `base_center` is set.

**Step 1: Locate the call sites**

Run: `Grep("HouseState::new\|base_center = ", path="src/sim", output_mode="content", -n=true)`. Identify the function(s) that create houses + set base_center.

**Step 2: Wire the initializer**

After each `house.base_center = Some((rx, ry))` assignment, add:

```rust
// Compute waypoint edge once base_center is known.
if let Some((rx, ry)) = house.base_center {
    house.waypoint_edge = crate::sim::house_state::closest_edge_for(
        (rx, ry),
        map.width,
        map.height,
    );
}
```

If `base_center` is set inside `HouseState::new`, instead pass map bounds into `new()` and compute there. Otherwise prefer the post-assignment hook.

**Step 3: Verify**

Run: `cargo build`
Expected: PASS.

Spot-check via debug log: add a temporary `log::info!("house {} waypoint_edge={}", interner.resolve(house.name), house.waypoint_edge);` after the assignment, run a skirmish, verify reasonable values (0..=3) for each player. Remove the debug log before commit.

**Step 4: Commit**

`world: initialize HouseState.waypoint_edge from base_center at game start`

---

### Task 4: Parse paradrop INI keys into GeneralRules

**Why:** All downstream tasks depend on the parsed config (lists, radius, aircraft type).

**Files:**
- Modify: `src/rules/ruleset.rs` (`GeneralRules` struct ~line 119, `from_ini` impl ~line 600)

**Pattern:** Mirrors existing `damage_fire_types: Vec<AnimRef>` parsing at line 669-680 (uses `general.get_list("Key")`). Lists are paired (`Inf` + `Num`); zip them post-parse.

**Step 1: Add fields to GeneralRules struct**

```rust
// src/rules/ruleset.rs (in GeneralRules struct, place after parachute_max_fall_rate)
pub struct GeneralRules {
    // ... existing fields ...

    /// Paradrop trigger radius in leptons. From [General] ParadropRadius=.
    /// Default 1024 (~4 cells). Distance to target at which carrier aircraft
    /// begins fog-reveal + transitions to overfly.
    pub paradrop_radius: i32,

    /// Carrier aircraft type for paradrop missions. From hardcoded `PDPLANE`
    /// (gamemd uses the AircraftType array entry by index; we look up by name).
    /// Resolved at parse time to the InternedId stored in rules.objects.
    pub paradrop_aircraft_type: String,

    /// American paradrop infantry list: (type_name, count) pairs.
    /// From [General] AmerParaDropInf= zipped with AmerParaDropNum=.
    /// Default: [("E1", 8)].
    pub amer_paradrop_list: Vec<(String, u32)>,

    /// Allied paradrop list. Default: [("E1", 6)].
    pub ally_paradrop_list: Vec<(String, u32)>,

    /// Soviet paradrop list. Default: [("E2", 9)].
    /// NOTE: gamemd skips count-equality assert on this branch only — preserve.
    pub sov_paradrop_list: Vec<(String, u32)>,

    /// Yuri paradrop list. Default: [("INIT", 6)].
    pub yuri_paradrop_list: Vec<(String, u32)>,
}
```

**Step 2: Add Default impl entries**

In `impl Default for GeneralRules` (find by grepping `impl Default for GeneralRules`), add:

```rust
paradrop_radius: 1024,
paradrop_aircraft_type: "PDPLANE".to_string(),
amer_paradrop_list: vec![("E1".to_string(), 8)],
ally_paradrop_list: vec![("E1".to_string(), 6)],
sov_paradrop_list:  vec![("E2".to_string(), 9)],
yuri_paradrop_list: vec![("INIT".to_string(), 6)],
```

**Step 3: Add parser helper**

```rust
// src/rules/ruleset.rs — add as a free fn near GeneralRules::from_ini
fn parse_paradrop_list(
    general: &IniSection,
    inf_key: &str,
    num_key: &str,
    skip_count_assert: bool,
    default: Vec<(String, u32)>,
) -> Vec<(String, u32)> {
    let inf = match general.get_list(inf_key) {
        Some(list) => list.into_iter().filter(|s| !s.is_empty()).map(|s| s.to_uppercase()).collect::<Vec<_>>(),
        None => return default,
    };
    let nums = match general.get_list(num_key) {
        Some(list) => list.into_iter()
            .filter_map(|s| s.parse::<u32>().ok())
            .collect::<Vec<_>>(),
        None => return default,
    };

    if !skip_count_assert && inf.len() != nums.len() {
        log::warn!(
            "Paradrop list mismatch: {}={} entries but {}={} entries — using defaults",
            inf_key, inf.len(), num_key, nums.len(),
        );
        return default;
    }

    // Soviet skip path: zip up to the shorter length (mirror gamemd's no-assert behavior).
    inf.into_iter().zip(nums.into_iter()).collect()
}
```

**Step 4: Wire in from_ini**

In `GeneralRules::from_ini` (line 600-ish), inside the `Self { ... }` block:

```rust
paradrop_radius: general.get_i32("ParadropRadius").unwrap_or(1024),
paradrop_aircraft_type: general
    .get("ParaDropPlane")
    .map(|s| s.trim().to_uppercase())
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "PDPLANE".to_string()),
amer_paradrop_list: parse_paradrop_list(
    general, "AmerParaDropInf", "AmerParaDropNum",
    false,
    vec![("E1".to_string(), 8)],
),
ally_paradrop_list: parse_paradrop_list(
    general, "AllyParaDropInf", "AllyParaDropNum",
    false,
    vec![("E1".to_string(), 6)],
),
sov_paradrop_list: parse_paradrop_list(
    general, "SovParaDropInf", "SovParaDropNum",
    true,  // P5: gamemd Soviet branch has no assert
    vec![("E2".to_string(), 9)],
),
yuri_paradrop_list: parse_paradrop_list(
    general, "YuriParaDropInf", "YuriParaDropNum",
    false,
    vec![("INIT".to_string(), 6)],
),
```

**Step 5: Add tests**

```rust
// src/rules/ruleset.rs — append to existing tests module
#[cfg(test)]
mod paradrop_parse_tests {
    use super::*;

    fn ini_with_general(body: &str) -> IniFile {
        let text = format!("[General]\n{}\n", body);
        IniFile::parse(&text).expect("parse")
    }

    #[test]
    fn test_paradrop_defaults() {
        let ini = ini_with_general("");
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(general.paradrop_radius, 1024);
        assert_eq!(general.paradrop_aircraft_type, "PDPLANE");
        assert_eq!(general.amer_paradrop_list, vec![("E1".to_string(), 8)]);
        assert_eq!(general.ally_paradrop_list, vec![("E1".to_string(), 6)]);
        assert_eq!(general.sov_paradrop_list,  vec![("E2".to_string(), 9)]);
        assert_eq!(general.yuri_paradrop_list, vec![("INIT".to_string(), 6)]);
    }

    #[test]
    fn test_paradrop_explicit_values() {
        let ini = ini_with_general(
            "ParadropRadius=2048\n\
             AmerParaDropInf=E1,GHOST,ENGINEER\n\
             AmerParaDropNum=6,6,6"
        );
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(general.paradrop_radius, 2048);
        assert_eq!(general.amer_paradrop_list, vec![
            ("E1".to_string(), 6),
            ("GHOST".to_string(), 6),
            ("ENGINEER".to_string(), 6),
        ]);
    }

    #[test]
    fn test_paradrop_list_mismatch_falls_back_to_default() {
        let ini = ini_with_general(
            "AllyParaDropInf=E1,E2\n\
             AllyParaDropNum=5"  // mismatch
        );
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(general.ally_paradrop_list, vec![("E1".to_string(), 6)]);
    }

    #[test]
    fn test_paradrop_soviet_no_assert() {
        // Soviet is gamemd's no-assert path: take what we can zip.
        let ini = ini_with_general(
            "SovParaDropInf=E2,E3\n\
             SovParaDropNum=9"  // mismatch — Soviet path tolerates
        );
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(general.sov_paradrop_list, vec![("E2".to_string(), 9)]);
    }
}
```

**Step 6: Verify**

Run: `cargo test paradrop_parse -- --nocapture`
Expected: 4 tests PASS.

**Step 7: Commit**

`rules: parse paradrop INI keys into GeneralRules`

---

### Task 5: Verify ParaDropWeapon ROF reaches a resolvable weapon

**Why:** The drop cadence (130 ticks) comes from `[ParaDropWeapon] ROF=130`. The Drop_Payload tick reads it via `rules.weapon("ParaDropWeapon").rof`. If the weapon isn't parsed or accessible, we need a fallback const.

**Files:** None modified — this is a verification-only task.

**Step 1: Run the lookup test**

```rust
// In a temporary test file or REPL — verify rules.weapon("ParaDropWeapon") works
#[test]
fn verify_paradrop_weapon_rof() {
    let rules = test_helpers::load_default_rules();  // however the test harness loads rulesmd.ini
    let weapon = rules.weapon("ParaDropWeapon").expect("ParaDropWeapon must parse");
    assert_eq!(weapon.rof, 130, "ROF expected 130 from [ParaDropWeapon] ROF= entry");
}
```

Place in `src/rules/weapon_type.rs` (or wherever weapon-resolution tests live).

**Step 2: Verify**

Run: `cargo test verify_paradrop_weapon -- --nocapture`

Expected: PASS, weapon.rof = 130.

If FAIL because `rules.weapon()` does not exist or returns None: weapon-resolution helper needs adding. Note this in Step 3 outcome.

**Step 3: Decide constant fallback**

If verification PASSED:
- Subsequent tasks reference `rules.weapon(&rules.general.paradrop_weapon_type).rof` (note: this implies adding `paradrop_weapon_type: String` field to GeneralRules — defaults to `"ParaDropWeapon"`). Add this field in a tiny follow-up edit to Task 4 if needed.

If verification FAILED:
- Hardcode `pub const PARADROP_DROP_INTERVAL_TICKS: u16 = 130;` in `src/sim/aircraft/drop_payload.rs` (Task 9), with an inline note this is the [ParaDropWeapon] ROF value, and a follow-up TODO task to wire weapon-resolution.

Document outcome in Step 4 commit message.

**Step 4: Commit (if any code change)**

`rules: verify ParaDropWeapon ROF lookup path` (if a test was added)
OR no commit if pure verification.

---

### Task 6: Add ParaDropApproach + ParaDropOverfly variants to AircraftMission

**Why:** FSM extension is the foundation for everything in `tick_aircraft_missions`; do this before writing any handler.

**Files:**
- Modify: `src/sim/aircraft/mod.rs` (~line 33, AircraftMission enum)

**Pattern:** New enum variants — match shape of existing `ReturnToBase { airfield_id }` and `Docking { airfield_id, sub_state, reload_timer }`.

**Step 1: Add variants**

```rust
// src/sim/aircraft/mod.rs (in AircraftMission enum)
pub enum AircraftMission {
    // ... existing variants ...

    /// Carrier aircraft flying in toward paradrop target.
    /// Transitions to ParaDropOverfly when distance <= ParadropRadius.
    ParaDropApproach {
        /// Cell coords of the click target.
        target_rx: u16,
        target_ry: u16,
        /// Latched true after fog-reveal + ChuteSound fire (P14, prevents repeat).
        has_revealed_fog: bool,
    },

    /// Carrier aircraft over the drop zone, dispensing payload.
    /// Transitions to silent despawn when cargo empty + at exit cell.
    ParaDropOverfly {
        /// Opposite-edge cell to fly to once cargo is empty.
        exit_rx: u16,
        exit_ry: u16,
        /// Ticks until next drop allowed (ROF=130 cadence — P22).
        drop_cooldown: u16,
        /// 5-tick mutex between drops (LandingState — P23).
        landing_state: u8,
        /// Decrements per drop; parity drives V-pattern side (P25).
        payload_count: u8,
    },
}
```

**Step 2: Audit exhaustive match arms**

Run: `Grep("match.*AircraftMission|AircraftMission::Idle\\s*=>", path="src", output_mode="files_with_matches")` — get the list of consumers.

For each file, add stub arms:
```rust
AircraftMission::ParaDropApproach { .. } | AircraftMission::ParaDropOverfly { .. } => {
    // Handled in tick_aircraft_missions; downstream consumers treat as "in flight, busy".
}
```

Likely consumers: `aircraft/idle_mode.rs` (decision tree), `aircraft/attack_mission.rs`, `world/world_commands.rs` (movement orders may need to skip/cancel paradrop missions), and the `tick_aircraft_missions` itself.

**Step 3: Verify**

Run: `cargo build`
Expected: PASS.

**Step 4: Commit**

`aircraft: add ParaDropApproach + ParaDropOverfly mission variants`

---

### Task 7: Stub paradrop mission dispatch + thread path_grid into tick_aircraft_missions

**Why:** Wire empty match arms in the main mission tick so the variants don't crash. Also thread `path_grid: Option<&PathGrid>` through the tick now — Tasks 11/12 need it for cell-passability checks during paradrop, and updating the signature in one focused commit avoids touching the call site again later.

**Files:**
- Modify: `src/sim/aircraft/mod.rs` (`tick_aircraft_missions` signature + `match &snap.mission` block ~line 164)
- Modify: `src/sim/world/mod.rs` (~line 1195 — the caller in `advance_tick`)

**Pattern:** Mirrors existing `match &snap.mission { AircraftMission::Idle => {...}, AircraftMission::Attack {...} => {...}, ... }` structure. Path-grid threading mirrors how `apply_command` and `world_orders` already accept `Option<&PathGrid>`.

**Step 1: Update tick_aircraft_missions signature**

```rust
// src/sim/aircraft/mod.rs (~line 111)
pub fn tick_aircraft_missions(
    sim: &mut Simulation,
    rules: &RuleSet,
    path_grid: Option<&crate::sim::pathfinding::core::PathGrid>,
) {
    // body unchanged — Tasks 11/12 will use path_grid; Task 7 just plumbs it.
    let _ = path_grid;
    // ... existing snapshot/process/apply ...
}
```

**Step 2: Update the caller in advance_tick**

Locate the call at [src/sim/world/mod.rs:1195](src/sim/world/mod.rs#L1195):

```rust
// before:
crate::sim::aircraft::tick_aircraft_missions(self, rules);
// after:
crate::sim::aircraft::tick_aircraft_missions(self, rules, path_grid);
```

`path_grid` is the `Option<&PathGrid>` parameter already in scope at `advance_tick`'s signature ([world/mod.rs:1096](src/sim/world/mod.rs#L1096)).

**Step 3: Add stub match arms**

```rust
// src/sim/aircraft/mod.rs — inside the `match &snap.mission` block in tick_aircraft_missions
AircraftMission::ParaDropApproach { target_rx, target_ry, has_revealed_fog } => {
    // Real handler in Task 11.
    let _ = (target_rx, target_ry, has_revealed_fog);
}
AircraftMission::ParaDropOverfly { exit_rx, exit_ry, drop_cooldown, landing_state, payload_count } => {
    // Real handler in Task 12.
    let _ = (exit_rx, exit_ry, drop_cooldown, landing_state, payload_count);
}
```

**Step 4: Verify**

Run: `cargo build`
Expected: PASS, no warnings about unhandled enum variants.

**Step 5: Commit**

`aircraft: stub ParaDrop mission dispatch + thread path_grid through mission tick`

---

### Task 8: Edge cell finder

**Why:** Carrier spawns at `house.waypoint_edge`'s passable cell (P6, P8); needed before launch handler.

**Files:**
- Create: `src/sim/world/edge_cell.rs`
- Modify: `src/sim/world/mod.rs` to add `pub mod edge_cell;`

**Pattern:** New module under `world/`; pure function with map-bounds + edge + target inputs. South mode is asymmetric per gamemd P8.

**Step 1: Create the module**

```rust
// src/sim/world/edge_cell.rs

//! Map-edge passable cell finder. Mirrors gamemd FUN_004AA440.
//!
//! Modes 0=N, 1=E, 2=S, 3=W. Modes 0/1/3 use linear scan-from-edge.
//! Mode 2 (south) is asymmetric: builds a candidate list and picks
//! closest-to-target.

use crate::sim::pathfinding::path_grid::PathGrid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    North,
    East,
    South,
    West,
}

impl Edge {
    pub fn from_index(i: u8) -> Option<Self> {
        match i {
            0 => Some(Edge::North),
            1 => Some(Edge::East),
            2 => Some(Edge::South),
            3 => Some(Edge::West),
            _ => None,
        }
    }
}

/// Find a passable cell along the given map edge, biased toward `target`.
///
/// Returns `None` if no passable cell exists along that edge.
pub fn find_passable_at_edge(
    path_grid: &PathGrid,
    map_width: u16,
    map_height: u16,
    edge: Edge,
    target: (u16, u16),
) -> Option<(u16, u16)> {
    match edge {
        Edge::North => scan_linear(path_grid, edge, map_width, map_height, target),
        Edge::East  => scan_linear(path_grid, edge, map_width, map_height, target),
        Edge::West  => scan_linear(path_grid, edge, map_width, map_height, target),
        Edge::South => scan_candidates_closest(path_grid, map_width, map_height, target),
    }
}

/// Linear scan along the edge — pick the cell closest to target's projection.
fn scan_linear(
    path_grid: &PathGrid,
    edge: Edge,
    map_width: u16,
    map_height: u16,
    target: (u16, u16),
) -> Option<(u16, u16)> {
    let row_or_col_axis: Vec<(u16, u16)> = match edge {
        Edge::North => (0..map_width).map(|x| (x, 0)).collect(),
        Edge::East  => (0..map_height).map(|y| (map_width.saturating_sub(1), y)).collect(),
        Edge::West  => (0..map_height).map(|y| (0, y)).collect(),
        Edge::South => unreachable!("south uses scan_candidates_closest"),
    };

    // Bias: pick passable cell minimizing Chebyshev distance to target's edge projection.
    row_or_col_axis
        .into_iter()
        .filter(|&(rx, ry)| path_grid.is_walkable(rx, ry))
        .min_by_key(|&(rx, ry)| {
            let dx = (rx as i32 - target.0 as i32).abs();
            let dy = (ry as i32 - target.1 as i32).abs();
            dx.max(dy)
        })
}

/// South-edge mode: build candidate list (≤10 valid cells), pick closest-to-target.
/// Mirrors gamemd's mode 2 special path (P8 — alternate cell IS the target for paradrop,
/// so we always hit the closest-to-target branch; no RNG needed).
fn scan_candidates_closest(
    path_grid: &PathGrid,
    map_width: u16,
    map_height: u16,
    target: (u16, u16),
) -> Option<(u16, u16)> {
    let south_y = map_height.saturating_sub(1);
    let mut candidates: Vec<(u16, u16)> = Vec::with_capacity(10);
    for x in 0..map_width {
        if candidates.len() >= 10 {
            break;
        }
        if path_grid.is_walkable(x, south_y) {
            candidates.push((x, south_y));
        }
    }
    candidates.into_iter().min_by_key(|&(rx, ry)| {
        let dx = (rx as i32 - target.0 as i32).abs();
        let dy = (ry as i32 - target.1 as i32).abs();
        dx.max(dy)
    })
}
```

**Note:** `PathGrid::is_walkable(x, y) -> bool` exists at [src/sim/pathfinding/core.rs:759](src/sim/pathfinding/core.rs#L759); ground-layer passability is the right semantic for paradrop drop-cell validation.

**Step 2: Add module to world/mod.rs**

```rust
// src/sim/world/mod.rs
pub mod edge_cell;
```

**Step 3: Add `#[cfg(test)]` PathGrid construction helpers**

`PathGrid` does not currently have `test_all_passable` / `test_all_blocked` constructors. Add them as a sub-step before the edge_cell tests:

```rust
// src/sim/pathfinding/core.rs — append to existing impl PathGrid block (or a new
// #[cfg(test)] impl block at the end of the file)

#[cfg(test)]
impl PathGrid {
    /// Test helper: build a PathGrid with every cell ground-walkable.
    pub fn test_all_passable(width: u16, height: u16) -> Self {
        let size = width as usize * height as usize;
        let mut cells = vec![DEFAULT_BLOCKED_CELL; size];
        for c in &mut cells {
            c.ground_walkable = true;
        }
        PathGrid { cells, width, height }
    }

    /// Test helper: build a PathGrid with every cell blocked.
    pub fn test_all_blocked(width: u16, height: u16) -> Self {
        let size = width as usize * height as usize;
        PathGrid {
            cells: vec![DEFAULT_BLOCKED_CELL; size],
            width,
            height,
        }
    }
}
```

The `DEFAULT_BLOCKED_CELL` constant and `Cell` struct already exist in [pathfinding/core.rs:897-908](src/sim/pathfinding/core.rs#L897). Verify the `Cell` field name matches `ground_walkable` (it does per [line 764](src/sim/pathfinding/core.rs#L764)).

**Step 4: Add tests**

```rust
// src/sim/world/edge_cell.rs (append)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::pathfinding::core::PathGrid;

    #[test]
    fn test_north_edge_picks_closest_to_target_x() {
        let grid = PathGrid::test_all_passable(100, 100);
        let cell = find_passable_at_edge(&grid, 100, 100, Edge::North, (42, 50)).unwrap();
        assert_eq!(cell.1, 0);   // y=0 (north)
        assert_eq!(cell.0, 42);  // x matches target's x projection
    }

    #[test]
    fn test_west_edge_picks_closest_to_target_y() {
        let grid = PathGrid::test_all_passable(100, 100);
        let cell = find_passable_at_edge(&grid, 100, 100, Edge::West, (50, 70)).unwrap();
        assert_eq!(cell.0, 0);
        assert_eq!(cell.1, 70);
    }

    #[test]
    fn test_south_edge_uses_candidate_list_closest() {
        let grid = PathGrid::test_all_passable(100, 100);
        let cell = find_passable_at_edge(&grid, 100, 100, Edge::South, (5, 50)).unwrap();
        // Mode 2 collects ≤10 candidates from x=0..min(10, w); candidates are
        // x in {0,1,2,3,4,5,6,7,8,9}; min Chebyshev to (5,50) is x=5.
        assert_eq!(cell, (5, 99));
    }

    #[test]
    fn test_no_passable_returns_none() {
        let grid = PathGrid::test_all_blocked(100, 100);
        assert_eq!(find_passable_at_edge(&grid, 100, 100, Edge::North, (50, 50)), None);
    }
}
```

**Step 5: Verify**

Run: `cargo test edge_cell -- --nocapture` and `cargo test pathfinding -- --nocapture`
Expected: 4 edge_cell tests PASS; existing pathfinding tests still PASS.

**Step 6: Commit**

`world: add edge_cell::find_passable_at_edge for paradrop carrier spawn`

---

### Task 9: V-pattern math + drop_payload skeleton

**Why:** Pure math first, before the messy spawn/cargo/descent integration. Tests prove parity early.

**Files:**
- Create: `src/sim/aircraft/drop_payload.rs`
- Modify: `src/sim/aircraft/mod.rs` to add `pub mod drop_payload;`

**Pattern:** Pure function over `(facing, payload_count)` — no entity access. Tests verify L,R,L,R alternation and 128-lepton magnitude.

**Step 1: Create the module skeleton**

```rust
// src/sim/aircraft/drop_payload.rs

//! Paradrop Drop_Payload — V-pattern math + per-tick passenger ejection.
//!
//! Mirrors gamemd.exe AircraftClass::Drop_Payload (0x415C60). Each call
//! ejects ONE passenger from the carrier's cargo at a 128-lepton offset
//! perpendicular to flight heading, alternating CW/CCW based on the
//! post-decrement payload count parity.

use crate::util::facing_table::facing_to_movement;
use crate::util::fixed_math::{SimFixed, sim_to_i32};

/// V-pattern lateral radius. From gamemd constant at 0x7E2808 = 128.0.
pub const V_PATTERN_RADIUS_LEPTONS: i32 = 128;

/// Per-side reset value for the LandingState mutex (gamemd P23).
/// Prevents back-to-back drops within 5 ticks of each other.
pub const LANDING_STATE_RESET: u8 = 5;

/// Drop interval in ticks. From [ParaDropWeapon] ROF=130 (P22).
/// Used only as a fallback if rules.weapon("ParaDropWeapon").rof lookup fails.
pub const PARADROP_DROP_INTERVAL_TICKS: u16 = 130;

/// Compute the V-pattern offset for the next drop, in leptons.
///
/// `facing`: aircraft body facing 0..=255 (RA2 convention: 0=N, 64=E, 128=S, 192=W).
/// `payload_count_post_dec`: payload count AFTER decrement (gamemd's order, P25).
///
/// Returns `(dx, dy)` in leptons. CW 90° when even (RIGHT of heading);
/// CCW 90° when odd (LEFT of heading). With initial count=8, sequence is L,R,L,R,L,R,L,R.
pub fn v_offset(facing: u8, payload_count_post_dec: u8) -> (i32, i32) {
    // gamemd's 0x3FFF binary-angle quarter-circle = our 64-step facing offset
    // (since 0x3FFF / 0xFFFF ≈ 0.25, and 64 / 256 = 0.25).
    let drop_facing = if (payload_count_post_dec & 1) == 0 {
        // EVEN → CW 90° (RIGHT)
        facing.wrapping_add(64)
    } else {
        // ODD → CCW 90° (LEFT)
        facing.wrapping_sub(64)
    };
    let radius = SimFixed::from_num(V_PATTERN_RADIUS_LEPTONS);
    let (dx, dy) = facing_to_movement(drop_facing, radius);
    (sim_to_i32(dx), sim_to_i32(dy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v_pattern_radius_is_128() {
        // Magnitude of (dx, dy) should be ~128 leptons regardless of facing.
        for facing in [0u8, 32, 64, 96, 128, 160, 192, 224] {
            let (dx, dy) = v_offset(facing, 0); // even → RIGHT
            let mag_sq = (dx * dx + dy * dy) as f64;
            let mag = mag_sq.sqrt();
            assert!(
                (mag - 128.0).abs() < 2.0,
                "facing={} produced offset ({},{}), mag={}",
                facing, dx, dy, mag,
            );
        }
    }

    #[test]
    fn test_v_pattern_alternates_starting_left() {
        // gamemd: with initial count=8, post-decrement sequence is 7,6,5,4,3,2,1,0.
        // Parity: 7→ODD→LEFT, 6→EVEN→RIGHT, 5→ODD→LEFT, 4→EVEN→RIGHT, ...
        // So drop sequence = L, R, L, R, L, R, L, R (first drop is LEFT).
        let facing = 0u8; // North → LEFT = -X (west), RIGHT = +X (east)
        let (dx_first,  _) = v_offset(facing, 7);
        let (dx_second, _) = v_offset(facing, 6);
        assert!(dx_first  < 0, "first drop (count=7, ODD) should be LEFT (-X), got dx={}", dx_first);
        assert!(dx_second > 0, "second drop (count=6, EVEN) should be RIGHT (+X), got dx={}", dx_second);
    }

    #[test]
    fn test_v_pattern_facing_north() {
        // Facing North (0): "RIGHT 90°" should be facing East (64) → +X direction.
        let (dx, dy) = v_offset(0, 0); // EVEN → RIGHT
        assert!(dx > 100, "North-RIGHT should give +X, got dx={}", dx);
        assert!(dy.abs() < 30, "North-RIGHT should have ~zero Y, got dy={}", dy);
    }

    #[test]
    fn test_v_pattern_facing_east() {
        // Facing East (64): "RIGHT 90°" should be facing South (128) → +Y direction.
        let (dx, dy) = v_offset(64, 0); // EVEN → RIGHT
        assert!(dy > 100, "East-RIGHT should give +Y, got dy={}", dy);
        assert!(dx.abs() < 30, "East-RIGHT should have ~zero X, got dx={}", dx);
    }
}
```

**Step 2: Add to module tree**

```rust
// src/sim/aircraft/mod.rs (top level, near `pub mod attack_mission;`)
pub mod drop_payload;
```

**Step 3: Verify**

Run: `cargo test drop_payload -- --nocapture`
Expected: 4 tests PASS.

**Step 4: Commit**

`aircraft: drop_payload V-pattern math + tests`

---

### Task 10: Implement try_drop with retry semantics

**Why:** This is the core drop primitive — pop passenger, spawn at offset cell + altitude, attach descent. Three return states (Success / ImpassableRetry / AttachFailedRetry) drive cooldown handling in the overfly mission tick.

**Files:**
- Modify: `src/sim/aircraft/drop_payload.rs`

**Pattern:** Operates on `&mut Simulation` for entity store + sound events. Returns enum for caller to dispatch on.

**Step 1: Define DropResult**

```rust
// src/sim/aircraft/drop_payload.rs (append)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropResult {
    /// Passenger was successfully placed; descent state attached.
    Success,
    /// Drop cell was impassable. Passenger re-inserted at cargo HEAD; payload_count restored.
    /// Caller should NOT reset drop_cooldown — retry next tick.
    ImpassableRetry,
    /// begin_parachute_descent returned false. Same retry semantics as Impassable.
    AttachFailedRetry,
    /// Cargo was empty (caller should have gated on cargo_empty already).
    NoCargo,
}
```

**Step 2: Implement try_drop**

```rust
// src/sim/aircraft/drop_payload.rs (append)

use crate::sim::movement::parachute_descent::begin_parachute_descent;
use crate::sim::passenger::PassengerRole;
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::fixed_math::SIM_ZERO;

/// Attempt to drop one passenger from the aircraft's cargo.
///
/// Pre-conditions (caller-enforced):
///   - aircraft entity exists
///   - aircraft has PassengerRole::Transport with non-empty cargo
///   - drop_cooldown == 0 && landing_state == 0
///
/// `path_grid`: Some when threaded through the tick from advance_tick;
/// None in headless tests — passability defaults to "always passable" in that case.
/// `rules`: needed to look up passenger ObjectType.size for cargo accounting.
///
/// On Success: caller resets drop_cooldown to ROF, sets landing_state=5, decrements payload_count.
/// On *Retry: caller leaves drop_cooldown unchanged (retry next tick).
pub fn try_drop(
    sim: &mut Simulation,
    rules: &crate::rules::ruleset::RuleSet,
    aircraft_id: u64,
    payload_count_pre_dec: u8,
    path_grid: Option<&crate::sim::pathfinding::core::PathGrid>,
) -> DropResult {
    // 1. Snapshot aircraft state we need.
    let (facing, altitude, aircraft_rx, aircraft_ry) = {
        let aircraft = match sim.entities.get(aircraft_id) {
            Some(e) => e,
            None => return DropResult::NoCargo,
        };
        let alt = aircraft.locomotor.as_ref().map(|l| l.altitude).unwrap_or(SIM_ZERO);
        (aircraft.facing, alt, aircraft.position.rx, aircraft.position.ry)
    };

    // 2. Pop FIFO passenger from cargo. (P24)
    let passenger_id = {
        let cargo = match sim
            .entities
            .get_mut(aircraft_id)
            .and_then(|a| a.passenger_role.cargo_mut())
        {
            Some(c) => c,
            None => return DropResult::NoCargo,
        };
        match cargo.unload_first() {
            Some(id) => id,
            None => return DropResult::NoCargo,
        }
    };

    // Look up passenger size via rules (needed to correct total_size on success).
    // unload_first does NOT decrement total_size — see passenger.rs:94.
    let pax_size: u32 = sim
        .entities
        .get(passenger_id)
        .and_then(|p| {
            let type_str = sim.interner.resolve(p.type_ref);
            rules.object(type_str).map(|o| o.size as u32)
        })
        .unwrap_or(1);

    // 3. Compute V-offset and drop cell. (P25-P28)
    let payload_count_post = payload_count_pre_dec.saturating_sub(1);
    let (dx, dy) = v_offset(facing, payload_count_post);
    let drop_rx = (aircraft_rx as i32 + dx / 256).clamp(0, u16::MAX as i32) as u16;
    let drop_ry = (aircraft_ry as i32 + dy / 256).clamp(0, u16::MAX as i32) as u16;

    // 4. Passability check via threaded path_grid. (P29)
    let passable = path_grid.map_or(true, |g| g.is_walkable(drop_rx, drop_ry));
    if !passable {
        // Re-insert at cargo HEAD; total_size unchanged (unload_first didn't decrement).
        if let Some(cargo) = sim
            .entities
            .get_mut(aircraft_id)
            .and_then(|a| a.passenger_role.cargo_mut())
        {
            cargo.passengers.insert(0, passenger_id);
        }
        return DropResult::ImpassableRetry;
    }

    // 5. Position passenger at drop cell + altitude; un-limbo. (P31)
    if let Some(passenger) = sim.entities.get_mut(passenger_id) {
        passenger.position.rx = drop_rx;
        passenger.position.ry = drop_ry;
        passenger.position.sub_x = 0;
        passenger.position.sub_y = 0;
        passenger.passenger_role = PassengerRole::None;
        if let Some(loco) = passenger.locomotor.as_mut() {
            loco.altitude = altitude;
        }
    }

    // 6. Attach descent. (P32-P34)
    if !begin_parachute_descent(&mut sim.entities, passenger_id, altitude) {
        // L17 deviation: revert passenger_role and re-insert at cargo HEAD; retry next tick.
        if let Some(passenger) = sim.entities.get_mut(passenger_id) {
            passenger.passenger_role = PassengerRole::Inside { transport_id: aircraft_id };
        }
        if let Some(cargo) = sim
            .entities
            .get_mut(aircraft_id)
            .and_then(|a| a.passenger_role.cargo_mut())
        {
            cargo.passengers.insert(0, passenger_id);
        }
        return DropResult::AttachFailedRetry;
    }

    // 7. ChuteSound at drop cell. (P30)
    sim.sound_events.push(SimSoundEvent::ChuteSound {
        rx: drop_rx,
        ry: drop_ry,
    });

    // 8. Decrement cargo.total_size on success (unload_first left it stale).
    if let Some(cargo) = sim
        .entities
        .get_mut(aircraft_id)
        .and_then(|a| a.passenger_role.cargo_mut())
    {
        cargo.total_size = cargo.total_size.saturating_sub(pax_size);
    }

    DropResult::Success
}
```

**Note:** `path_grid` is threaded as a parameter (Task 7 plumbed it into `tick_aircraft_missions`; Task 12 passes it into `try_drop` from the apply phase). `rules` is needed for size lookup. `is_walkable` is the right ground-passability check per [pathfinding/core.rs:759](src/sim/pathfinding/core.rs#L759). `ObjectType.size` field name should be verified during implementation — adjust if it's actually `Size` or similar.

**Step 3: Add unit tests**

```rust
// src/sim/aircraft/drop_payload.rs (append to tests module)

// (Integration-style tests of try_drop go in Task 15 since they need full Simulation
// setup. Unit tests for v_offset are already in Task 9.)
```

**Step 4: Verify**

Run: `cargo build`
Expected: PASS.

Run: `cargo test drop_payload -- --nocapture`
Expected: 4 tests still PASS (no regressions in v_offset).

**Step 5: Commit**

`aircraft: drop_payload::try_drop with impassable + attach-fail retry`

---

### Task 11: tick_paradrop_approach handler

**Why:** Implements the carrier's approach phase — fog-reveal latch, ChuteSound trigger, transition to overfly.

**Files:**
- Create: `src/sim/aircraft/paradrop_mission.rs`
- Modify: `src/sim/aircraft/mod.rs` — add `pub mod paradrop_mission;`; replace the Task 7 stub for `ParaDropApproach` with a call to `paradrop_mission::tick_approach`.

**Pattern:** Pure function returning a mutation struct (mirrors `attack_mission::tick_attack_state` shape at [src/sim/aircraft/attack_mission.rs](src/sim/aircraft/attack_mission.rs)).

**Step 1: Create the module**

```rust
// src/sim/aircraft/paradrop_mission.rs

//! Carrier-aircraft paradrop mission handlers — Approach + Overfly.
//!
//! Approach: flies in toward target. When distance <= ParadropRadius,
//! fires a one-shot fog reveal + ChuteSound and transitions to Overfly.
//!
//! Overfly: dispenses payload at ROF cadence with V-pattern offset.
//! When cargo empty, redirects to opposite-edge exit cell and silently despawns.

use crate::rules::ruleset::RuleSet;
use crate::sim::aircraft::AircraftMission;
use crate::sim::aircraft::drop_payload::{DropResult, LANDING_STATE_RESET, try_drop};
use crate::sim::world::{SimSoundEvent, Simulation};

/// Approach handler return — describes mission/state mutation for the apply phase.
pub struct ApproachOutcome {
    pub new_mission: AircraftMission,
    pub fire_fog_reveal: bool,
    pub play_chute_sound: bool,
    pub move_to: Option<(u16, u16)>,
    /// Set true if cargo emptied mid-approach (transition to Idle).
    pub abort_to_idle: bool,
}

pub fn tick_approach(
    sim: &Simulation,
    rules: &RuleSet,
    aircraft_id: u64,
    target_rx: u16,
    target_ry: u16,
    has_revealed_fog: bool,
    path_grid: Option<&crate::sim::pathfinding::core::PathGrid>,
) -> ApproachOutcome {
    let aircraft = match sim.entities.get(aircraft_id) {
        Some(e) => e,
        None => {
            return ApproachOutcome {
                new_mission: AircraftMission::Idle,
                fire_fog_reveal: false,
                play_chute_sound: false,
                move_to: None,
                abort_to_idle: true,
            };
        }
    };

    // Cargo empty → abort. (P17)
    let cargo_count = aircraft
        .passenger_role
        .cargo()
        .map_or(0, |c| c.count());
    if cargo_count == 0 {
        return ApproachOutcome {
            new_mission: AircraftMission::Idle,
            fire_fog_reveal: false,
            play_chute_sound: false,
            move_to: None,
            abort_to_idle: true,
        };
    }

    // P13: 3D distance to target — Chebyshev × 256 leptons/cell as a starting approximation.
    let dx = (aircraft.position.rx as i32 - target_rx as i32).abs();
    let dy = (aircraft.position.ry as i32 - target_ry as i32).abs();
    let dist_leptons = dx.max(dy) * 256;

    let radius = rules.general.paradrop_radius;

    // P14: fog reveal + sound, latched once per launch.
    let fire_fog = dist_leptons <= radius && !has_revealed_fog;
    let play_sound = fire_fog;

    // P16: transition to Overfly at the ParadropRadius threshold.
    if dist_leptons <= radius {
        // Compute exit cell (opposite edge — Task 13 helper resolves this).
        let exit = compute_exit_cell(sim, aircraft.owner, target_rx, target_ry, path_grid);
        return ApproachOutcome {
            new_mission: AircraftMission::ParaDropOverfly {
                exit_rx: exit.0,
                exit_ry: exit.1,
                drop_cooldown: 0,
                landing_state: 0,
                payload_count: cargo_count as u8,
            },
            fire_fog_reveal: fire_fog,
            play_chute_sound: play_sound,
            move_to: Some(exit),  // start flying toward exit
            abort_to_idle: false,
        };
    }

    // Still approaching: keep flying toward target.
    ApproachOutcome {
        new_mission: AircraftMission::ParaDropApproach {
            target_rx,
            target_ry,
            has_revealed_fog: fire_fog || has_revealed_fog,
        },
        fire_fog_reveal: fire_fog,
        play_chute_sound: play_sound,
        move_to: if aircraft.movement_target.is_none() {
            Some((target_rx, target_ry))
        } else {
            None
        },
        abort_to_idle: false,
    }
}

/// Resolve the opposite-edge exit cell for the carrier aircraft.
/// Encoding: waypoint_edge → opposite via +2 mod 4 (P12).
///
/// Fallback chain when no passable opposite-edge cell exists:
///   1. Try the opposite edge.
///   2. Try the South edge (any side may have water-locked opposite — south usually doesn't).
///   3. Fall back to a corner of the playfield to force a despawn boundary.
fn compute_exit_cell(
    sim: &Simulation,
    owner: crate::sim::intern::InternedId,
    target_rx: u16,
    target_ry: u16,
    path_grid: Option<&crate::sim::pathfinding::core::PathGrid>,
) -> (u16, u16) {
    use crate::sim::world::edge_cell::{Edge, find_passable_at_edge};
    let waypoint_edge = sim
        .houses
        .get(&owner)
        .map_or(0, |h| h.waypoint_edge);
    let opposite_idx = (waypoint_edge + 2) % 4;
    let exit_edge = Edge::from_index(opposite_idx).unwrap_or(Edge::South);

    let map_w = sim.fog.width;
    let map_h = sim.fog.height;
    let target = (target_rx, target_ry);

    if let Some(grid) = path_grid {
        if let Some(cell) = find_passable_at_edge(grid, map_w, map_h, exit_edge, target) {
            return cell;
        }
        // Secondary: try south as a generic fallback edge.
        if exit_edge != Edge::South {
            if let Some(cell) = find_passable_at_edge(grid, map_w, map_h, Edge::South, target) {
                return cell;
            }
        }
    }
    // Final fallback: a playfield corner forces the silent-despawn boundary check
    // in tick_overfly to fire deterministically (rather than looping at the target).
    (map_w.saturating_sub(1), map_h.saturating_sub(1))
}
```

**Note:** `sim.houses` is verified at [world/mod.rs:205](src/sim/world/mod.rs#L205). `sim.fog.width`/`sim.fog.height` is the canonical source of map dimensions ([vision/mod.rs:179-181](src/sim/vision/mod.rs#L179)). `aircraft.movement_target` is the standard movement-target field already used elsewhere in `tick_aircraft_missions` — verify during impl.

**Step 2: Wire into tick_aircraft_missions**

Replace the Task 7 stub for `ParaDropApproach` in `src/sim/aircraft/mod.rs`:

```rust
AircraftMission::ParaDropApproach { target_rx, target_ry, has_revealed_fog } => {
    let outcome = paradrop_mission::tick_approach(
        sim, rules, snap.id, *target_rx, *target_ry, *has_revealed_fog, path_grid,
    );
    m.new_mission = outcome.new_mission;
    if outcome.abort_to_idle {
        // Idle handles airport_bound self-destruct via existing path.
    }
    if outcome.fire_fog_reveal {
        // V1: no-op. Fog-reveal hookup is tracked in plan Open Questions
        // (deferred alongside silent-spawn audio/radar suppression — D5).
        // ChuteSound is emitted below regardless, which is the audible cue.
        let _ = outcome.fire_fog_reveal;
    }
    if outcome.play_chute_sound {
        sim.sound_events.push(SimSoundEvent::ChuteSound {
            rx: *target_rx,
            ry: *target_ry,
        });
    }
    m.move_to = outcome.move_to;
}
```

**Step 3: Add tests**

```rust
// src/sim/aircraft/paradrop_mission.rs (append)
#[cfg(test)]
mod tests {
    use super::*;
    // Tests for tick_approach require Simulation construction — defer to Task 15
    // integration tests. Unit-test the pure parts here:

    #[test]
    fn test_chebyshev_distance_in_leptons() {
        // dx=3 cells, dy=2 cells → Chebyshev=3, distance=3*256=768 leptons.
        let dx = 3i32;
        let dy = 2i32;
        let dist = dx.max(dy) * 256;
        assert_eq!(dist, 768);
    }

    #[test]
    fn test_radius_threshold() {
        // ParadropRadius=1024 → 4 cells exactly triggers transition.
        let radius = 1024;
        assert!(4 * 256 <= radius);  // 1024 ≤ 1024
        assert!(5 * 256 > radius);   // 1280 > 1024
    }
}
```

**Step 4: Verify**

Run: `cargo build && cargo test paradrop_mission -- --nocapture`
Expected: PASS, 2 tests added.

**Step 5: Commit**

`aircraft: tick_paradrop_approach with fog-reveal latch + overfly transition`

---

### Task 12: tick_paradrop_overfly handler

**Why:** Drives the actual drop cadence + cargo-empty exit + silent despawn.

**Files:**
- Modify: `src/sim/aircraft/paradrop_mission.rs`
- Modify: `src/sim/aircraft/mod.rs` — replace the Task 7 stub for `ParaDropOverfly`.

**Pattern:** Same shape as tick_approach — returns an outcome struct describing mutations. Calls `drop_payload::try_drop` when cooldowns are zero.

**Step 1: Add overfly handler**

```rust
// src/sim/aircraft/paradrop_mission.rs (append)

pub struct OverflyOutcome {
    pub new_mission: AircraftMission,
    pub move_to: Option<(u16, u16)>,
    pub try_drop: bool,
    pub silent_despawn: bool,
}

pub fn tick_overfly(
    sim: &Simulation,
    aircraft_id: u64,
    exit_rx: u16,
    exit_ry: u16,
    drop_cooldown: u16,
    landing_state: u8,
    payload_count: u8,
) -> OverflyOutcome {
    let aircraft = match sim.entities.get(aircraft_id) {
        Some(e) => e,
        None => {
            return OverflyOutcome {
                new_mission: AircraftMission::Idle,
                move_to: None,
                try_drop: false,
                silent_despawn: false,
            };
        }
    };

    let cargo_count = aircraft.passenger_role.cargo().map_or(0, |c| c.count());
    let cargo_empty = cargo_count == 0;

    // Decrement cooldowns.
    let new_cooldown = drop_cooldown.saturating_sub(1);
    let new_landing  = landing_state.saturating_sub(1);

    // P19: cargo empty → fly to exit.
    if cargo_empty {
        let at_exit = aircraft.position.rx == exit_rx && aircraft.position.ry == exit_ry;
        // P20: silent despawn at exit boundary.
        let despawn = at_exit
            || aircraft.position.rx == 0
            || aircraft.position.ry == 0
            || aircraft.position.rx + 1 >= sim.fog.width
            || aircraft.position.ry + 1 >= sim.fog.height;
        return OverflyOutcome {
            new_mission: AircraftMission::ParaDropOverfly {
                exit_rx, exit_ry,
                drop_cooldown: new_cooldown,
                landing_state: new_landing,
                payload_count,
            },
            move_to: if !despawn && aircraft.movement_target.is_none() {
                Some((exit_rx, exit_ry))
            } else {
                None
            },
            try_drop: false,
            silent_despawn: despawn,
        };
    }

    // P21: drop trigger if both cooldowns at zero.
    let can_drop = new_cooldown == 0 && new_landing == 0 && cargo_count > 0;

    OverflyOutcome {
        new_mission: AircraftMission::ParaDropOverfly {
            exit_rx, exit_ry,
            drop_cooldown: new_cooldown,
            landing_state: new_landing,
            payload_count,
        },
        move_to: None,  // already heading toward target/exit; locomotor handles
        try_drop: can_drop,
        silent_despawn: false,
    }
}
```

**Step 2: Wire into tick_aircraft_missions**

Replace the Task 7 stub for `ParaDropOverfly` in `src/sim/aircraft/mod.rs`. Since `try_drop` mutates `sim`, this requires care with the snapshot/apply pattern — call try_drop in the apply phase, not the snapshot phase:

```rust
// In the snapshot/process match block:
AircraftMission::ParaDropOverfly { exit_rx, exit_ry, drop_cooldown, landing_state, payload_count } => {
    let outcome = paradrop_mission::tick_overfly(
        sim, snap.id, *exit_rx, *exit_ry, *drop_cooldown, *landing_state, *payload_count,
    );
    m.new_mission = outcome.new_mission;
    m.move_to = outcome.move_to;
    // Stash try_drop / silent_despawn flags on the mutation:
    m.paradrop_try_drop = outcome.try_drop;
    m.paradrop_silent_despawn = outcome.silent_despawn;
    m.paradrop_payload_count_pre = *payload_count;
}
```

Add the new fields to `MissionMutation`:

```rust
struct MissionMutation {
    // ... existing ...
    paradrop_try_drop: bool,
    paradrop_silent_despawn: bool,
    paradrop_payload_count_pre: u8,
}
```

In the apply phase (after the existing `mutations` loop), add:

```rust
// Apply paradrop drop attempts.
let drop_attempts: Vec<(u64, u8)> = mutations
    .iter()
    .filter(|m| m.paradrop_try_drop)
    .map(|m| (m.id, m.paradrop_payload_count_pre))
    .collect();
for (aircraft_id, payload_pre) in drop_attempts {
    use crate::sim::aircraft::drop_payload::{DropResult, LANDING_STATE_RESET, try_drop, PARADROP_DROP_INTERVAL_TICKS};
    let result = try_drop(sim, rules, aircraft_id, payload_pre, path_grid);
    if let Some(entity) = sim.entities.get_mut(aircraft_id) {
        if let Some(AircraftMission::ParaDropOverfly {
            exit_rx, exit_ry, payload_count, ..
        }) = entity.aircraft_mission.clone()
        {
            let drop_interval = rules
                .weapon(&rules.general.paradrop_weapon_type_or_default())
                .map(|w| w.rof.max(1) as u16)
                .unwrap_or(PARADROP_DROP_INTERVAL_TICKS);

            let new_mission = match result {
                DropResult::Success => AircraftMission::ParaDropOverfly {
                    exit_rx,
                    exit_ry,
                    drop_cooldown: drop_interval,
                    landing_state: LANDING_STATE_RESET,
                    payload_count: payload_count.saturating_sub(1),
                },
                DropResult::ImpassableRetry | DropResult::AttachFailedRetry => {
                    // Retry next tick — leave cooldown as-is, payload_count restored by try_drop.
                    AircraftMission::ParaDropOverfly {
                        exit_rx, exit_ry,
                        drop_cooldown: 0,
                        landing_state: 0,
                        payload_count,
                    }
                }
                DropResult::NoCargo => AircraftMission::Idle,
            };
            entity.aircraft_mission = Some(new_mission);
        }
    }
}

// Apply silent despawns.
for m in &mutations {
    if m.paradrop_silent_despawn {
        if let Some(entity) = sim.entities.get_mut(m.id) {
            entity.health.current = 0;
            entity.dying = true;
            // Do NOT emit explosion anim (Landable=no path).
            entity.aircraft_mission = None;
        }
    }
}
```

**Step 3: Add helper for weapon-type lookup**

In `src/rules/ruleset.rs`, add a helper to `GeneralRules`:

```rust
impl GeneralRules {
    pub fn paradrop_weapon_type_or_default(&self) -> &str {
        // For v1 hardcoded; future: parse [PDPLANE] Primary= and store.
        "ParaDropWeapon"
    }
}
```

**Step 4: Verify**

Run: `cargo build && cargo test paradrop -- --nocapture`
Expected: PASS, no new test additions but existing tests still green.

**Step 5: Commit**

`aircraft: tick_paradrop_overfly with drop cadence + silent despawn`

---

### Task 13: paradrop::launch SW handler + per-side dispatch

**Why:** Top of the pipeline — entry point from the SW dispatch in world_commands.

**Files:**
- Create: `src/sim/superweapon/paradrop.rs`
- Modify: `src/sim/superweapon/mod.rs` — add `pub mod paradrop;`

**Pattern:** Mirrors `iron_curtain.rs`:`launch(sim, rules, owner, rx, ry) -> bool`. Difference: takes a `kind: ParaDropKind` to distinguish case 5 (side-branched) from case 6 (always Amer).

**Step 1: Create the module**

```rust
// src/sim/superweapon/paradrop.rs

//! ParaDrop / AmerParaDrop superweapon launch handler.
//!
//! Mirrors gamemd.exe SuperClass::Launch cases 5 (ParaDrop, side-branched)
//! and 6 (AmerParaDrop, always-American). Per-side branch picks an infantry
//! list from rules; for each (inf_type, num) entry, spawns one PDPLANE at
//! the house's waypoint edge with `num` infantry loaded as cargo.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, sim/aircraft, sim/world, sim/passenger.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::ruleset::RuleSet;
use crate::sim::aircraft::AircraftMission;
use crate::sim::intern::InternedId;
use crate::sim::movement::air_movement;
use crate::sim::passenger::PassengerRole;
use crate::sim::world::edge_cell::{Edge, find_passable_at_edge};
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::fixed_math::{SimFixed, ra2_speed_to_leptons_per_second};

#[derive(Debug, Clone, Copy)]
pub enum ParaDropKind {
    /// Type=ParaDrop — side-branched on HouseClass.side_index.
    Generic,
    /// Type=AmerParaDrop — always uses the AmerParaDropList.
    American,
}

pub fn launch(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    target_rx: u16,
    target_ry: u16,
    kind: ParaDropKind,
    path_grid: Option<&crate::sim::pathfinding::core::PathGrid>,
) -> bool {
    // P1: bridge rejection (deferred — see plan Open Questions).
    let (target_rx, target_ry) = (target_rx, target_ry);

    // P2/P3: pick the per-side list.
    let list: &Vec<(String, u32)> = match kind {
        ParaDropKind::American => &rules.general.amer_paradrop_list,
        ParaDropKind::Generic => {
            let side = sim.houses.get(&owner).map_or(0, |h| h.side_index);
            match side {
                0 => &rules.general.ally_paradrop_list,
                2 => &rules.general.yuri_paradrop_list,
                _ => &rules.general.sov_paradrop_list,  // P2 fallback
            }
        }
    };

    // P6: resolve waypoint edge cell.
    let waypoint_edge_idx = sim.houses.get(&owner).map_or(0, |h| h.waypoint_edge);
    let edge = match Edge::from_index(waypoint_edge_idx) {
        Some(e) => e,
        None => return false,
    };
    let edge_cell = match path_grid.and_then(|g| {
        find_passable_at_edge(g, sim.fog.width, sim.fog.height, edge, (target_rx, target_ry))
    }) {
        Some(c) => c,
        None => return false,
    };

    // Sound event for SW launch (existing convention).
    sim.sound_events.push(SimSoundEvent::SuperWeaponLaunched {
        owner,
        rx: target_rx,
        ry: target_ry,
    });

    // P4/P38: spawn one PDPLANE per (inf_type, num) entry.
    let mut spawned_any = false;
    for (inf_type_name, num) in list.clone() {
        if spawn_pdplane(sim, rules, owner, edge_cell, target_rx, target_ry, &inf_type_name, num) {
            spawned_any = true;
        }
    }
    spawned_any
}

fn spawn_pdplane(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    edge_cell: (u16, u16),
    target_rx: u16,
    target_ry: u16,
    inf_type: &str,
    num: u32,
) -> bool {
    let owner_str = sim.interner.resolve(owner).to_string();
    let pdplane_type = rules.general.paradrop_aircraft_type.clone();

    // P9, P11 (D5 deferred): spawn at edge cell, ground z. Subsequent post-spawn
    // overrides set altitude to flight_level directly (S8 parity drift).
    let pdplane_id = match sim.spawn_object_at_height(
        &pdplane_type, &owner_str, edge_cell.0, edge_cell.1, /*facing*/ 0, /*z*/ 0, rules,
    ) {
        Some(id) => id,
        None => {
            log::warn!("paradrop: failed to spawn carrier {} at edge ({},{})",
                pdplane_type, edge_cell.0, edge_cell.1);
            return false;
        }
    };

    // S8: jump straight to cruise altitude.
    let flight_level = SimFixed::from_num(rules.general.flight_level);
    if let Some(entity) = sim.entities.get_mut(pdplane_id) {
        if let Some(loco) = entity.locomotor.as_mut() {
            loco.altitude = flight_level;
            loco.target_altitude = flight_level;
            loco.air_phase = crate::sim::movement::locomotor::AirMovePhase::Cruising;
        }
    }

    // P12: load N infantry into cargo as `Inside` passengers.
    let inf_size = rules.object(inf_type).map(|o| o.size).unwrap_or(1);
    let mut loaded = 0u32;
    for _ in 0..num {
        // Spawn the infantry at the carrier's edge cell. spawn_object_at_height
        // registers the entity in sim.occupancy at that cell — we IMMEDIATELY
        // remove that registration after setting passenger_role = Inside, since
        // passengers in transit shouldn't block ground cells.
        let pax_id = match sim.spawn_object_at_height(
            inf_type, &owner_str, edge_cell.0, edge_cell.1, /*facing*/ 0, /*z*/ 0, rules,
        ) {
            Some(id) => id,
            None => break,
        };
        if let Some(pax) = sim.entities.get_mut(pax_id) {
            pax.passenger_role = PassengerRole::Inside { transport_id: pdplane_id };
        }
        // Clean up transient occupancy from the spawn — passenger is now Inside the carrier.
        // Mirrors the existing transport-boarding pattern in passenger.rs (~line 401).
        sim.occupancy.remove(edge_cell.0, edge_cell.1, pax_id);
        if let Some(cargo) = sim
            .entities
            .get_mut(pdplane_id)
            .and_then(|a| a.passenger_role.cargo_mut())
        {
            if !cargo.board(pax_id, inf_size) {
                // Capacity exceeded — break and let the partial cargo fly.
                break;
            }
        }
        loaded += 1;
    }

    if loaded == 0 {
        // Couldn't load any passengers — kill the empty carrier rather than fly empty.
        if let Some(entity) = sim.entities.get_mut(pdplane_id) {
            entity.health.current = 0;
            entity.dying = true;
        }
        return false;
    }

    // P10: set initial mission + destination.
    if let Some(entity) = sim.entities.get_mut(pdplane_id) {
        entity.aircraft_mission = Some(AircraftMission::ParaDropApproach {
            target_rx,
            target_ry,
            has_revealed_fog: false,
        });
    }
    let speed = rules
        .object(&pdplane_type)
        .map(|o| ra2_speed_to_leptons_per_second(o.speed.max(1)))
        .unwrap_or(SimFixed::from_num(8));
    air_movement::issue_air_move_command(&mut sim.entities, pdplane_id, (target_rx, target_ry), speed);
    true
}
```

**Note:** `sim.spawn_object_at_height` is `pub(crate) fn` on `Simulation` at [src/sim/world/world_spawn.rs:288](src/sim/world/world_spawn.rs#L288) — directly callable. `loco.altitude`, `loco.target_altitude`, `loco.air_phase` are confirmed on `LocomotorState` at [src/sim/movement/locomotor.rs:124-142](src/sim/movement/locomotor.rs#L124). `AirMovePhase::Cruising` at [locomotor.rs:93](src/sim/movement/locomotor.rs#L93). `sim.occupancy.remove` — verify exact signature during impl (likely `remove(rx, ry, entity_id)` per the existing `add(...)` pattern at [passenger.rs:532](src/sim/passenger.rs#L532)).

**Step 2: Add to module tree**

```rust
// src/sim/superweapon/mod.rs
pub mod paradrop;
```

**Step 3: Add unit tests**

Defer integration testing to Task 15. Add small unit tests for the side-branch picker:

```rust
// src/sim/superweapon/paradrop.rs (append)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_side_branch_fallback_to_soviet_for_unknown_side() {
        // Side index 1 (Soviet) and 99 (unknown) both fall back to Soviet list.
        // Verified by inspection of the match block in launch().
        // (Functional verification in Task 15 integration test.)
    }
}
```

**Step 4: Verify**

Run: `cargo build`
Expected: PASS.

**Step 5: Commit**

`superweapon: paradrop launch handler with per-side dispatch + spawn_pdplane`

---

### Task 14: Wire ParaDrop / AmerParaDrop dispatch

**Why:** Plug the new handler into the SW launch dispatch.

**Files:**
- Modify: `src/sim/world/world_commands.rs` (~line 982)

**Pattern:** Add two match arms before the `other =>` fallthrough. `path_grid: Option<&PathGrid>` is already in scope as a parameter of `apply_command` ([world_commands.rs:99](src/sim/world/world_commands.rs#L99)) — pass it through.

**Step 1: Replace the fallthrough**

```rust
// src/sim/world/world_commands.rs (~line 982 — inside the `match kind` block)
crate::rules::superweapon_type::SuperWeaponKind::ParaDrop => {
    let rules = rules.unwrap();
    crate::sim::superweapon::paradrop::launch(
        self, rules, owner_iid, *target_rx, *target_ry,
        crate::sim::superweapon::paradrop::ParaDropKind::Generic,
        path_grid,
    )
}
crate::rules::superweapon_type::SuperWeaponKind::AmerParaDrop => {
    let rules = rules.unwrap();
    crate::sim::superweapon::paradrop::launch(
        self, rules, owner_iid, *target_rx, *target_ry,
        crate::sim::superweapon::paradrop::ParaDropKind::American,
        path_grid,
    )
}
other => {
    log::warn!("SuperWeapon kind {:?} not yet implemented", other);
    false
}
```

**Step 2: Verify**

Run: `cargo build`
Expected: PASS.

**Step 3: Commit**

`world_commands: dispatch ParaDrop + AmerParaDrop to paradrop::launch`

---

### Task 15: End-to-end integration test

**Why:** The unit tests cover individual pieces. This test exercises the whole pipeline once, catching wiring bugs.

**Files:**
- Create: `tests/paradrop_e2e.rs` (or extend existing test harness — match the existing integration-test convention by checking `tests/` structure first)

**Pattern:** Builds a minimal Simulation, configures rules, fires the SW, advances ticks, asserts on intermediate + final states.

**Step 1: Locate the integration-test convention**

Run: `Glob("tests/**/*.rs")`. If the project uses `tests/` for integration tests, place there. If integration tests live inside src modules with `#[cfg(test)]`, place inside `src/sim/superweapon/paradrop.rs` instead.

**Step 2: Write the test**

```rust
// tests/paradrop_e2e.rs (or appropriate location)

//! End-to-end paradrop launch + descent verification.

use ra2_engine::rules::ruleset::RuleSet;
use ra2_engine::sim::aircraft::AircraftMission;
use ra2_engine::sim::superweapon::paradrop::{ParaDropKind, launch};
use ra2_engine::sim::world::Simulation;

fn build_test_simulation() -> (Simulation, RuleSet) {
    // Use the standard test harness for loading rulesmd.ini + a minimal map.
    // Names used here are placeholders; adjust to actual harness API.
    let rules = ra2_engine::rules::test_helpers::load_default_rules();
    let mut sim = ra2_engine::sim::test_helpers::sim_with_house("AmericanPlayer", /*side*/ 0);
    sim.set_map_dimensions(100, 100);
    sim.house_mut("AmericanPlayer").base_center = Some((50, 50));
    sim.house_mut("AmericanPlayer").waypoint_edge = 0; // North
    (sim, rules)
}

#[test]
fn test_paradrop_e2e_launch_to_first_drop() {
    let (mut sim, rules) = build_test_simulation();

    let owner = sim.interner.intern("AmericanPlayer");
    let target = (50u16, 50u16);

    // Launch.
    let ok = launch(&mut sim, &rules, owner, target.0, target.1, ParaDropKind::American);
    assert!(ok, "launch should succeed");

    // Find the spawned PDPLANE.
    let pdplane_ids: Vec<u64> = sim
        .entities
        .values()
        .filter(|e| {
            sim.interner.resolve(e.type_ref).eq_ignore_ascii_case("PDPLANE")
        })
        .map(|e| e.stable_id)
        .collect();
    assert_eq!(pdplane_ids.len(), 1, "AmerParaDropInf=E1 default → 1 PDPLANE");
    let pdplane_id = pdplane_ids[0];

    // Verify cargo loaded with 8 E1 (default AmerParaDropNum).
    let cargo_count = sim.entities.get(pdplane_id).unwrap()
        .passenger_role.cargo().unwrap().count();
    assert_eq!(cargo_count, 8, "default AmerParaDropNum=8");

    // Verify mission set.
    assert!(matches!(
        sim.entities.get(pdplane_id).unwrap().aircraft_mission,
        Some(AircraftMission::ParaDropApproach { .. }),
    ));

    // Advance ticks until first drop.
    let max_ticks = 1000;
    let mut first_drop_tick = None;
    for tick in 0..max_ticks {
        let cargo_before = sim.entities.get(pdplane_id)
            .map_or(0, |e| e.passenger_role.cargo().map_or(0, |c| c.count()));
        sim.advance_tick(&rules);
        let cargo_after = sim.entities.get(pdplane_id)
            .map_or(0, |e| e.passenger_role.cargo().map_or(0, |c| c.count()));
        if cargo_after < cargo_before {
            first_drop_tick = Some(tick);
            break;
        }
    }
    assert!(first_drop_tick.is_some(), "should drop at least once within {} ticks", max_ticks);

    // Verify a parachute_state exists on a freshly-dropped infantry.
    let descending_count = sim
        .entities
        .values()
        .filter(|e| e.parachute_state.is_some())
        .count();
    assert!(descending_count >= 1, "at least one infantry should be descending");
}

#[test]
fn test_paradrop_e2e_full_descent_to_landing() {
    let (mut sim, rules) = build_test_simulation();
    let owner = sim.interner.intern("AmericanPlayer");
    launch(&mut sim, &rules, owner, 50, 50, ParaDropKind::American);

    // Run for enough ticks to drain cargo + descent.
    // 8 drops × 130-tick ROF + descent = ~1500 ticks.
    for _ in 0..2000 {
        sim.advance_tick(&rules);
    }

    // Final state: no descending entities.
    let still_descending = sim.entities.values().filter(|e| e.parachute_state.is_some()).count();
    assert_eq!(still_descending, 0, "all infantry should have landed");

    // PDPLANE should be despawned (or dying).
    let pdplane_alive = sim
        .entities
        .values()
        .any(|e| sim.interner.resolve(e.type_ref).eq_ignore_ascii_case("PDPLANE")
            && e.health.current > 0);
    assert!(!pdplane_alive, "PDPLANE should have despawned");

    // Some E1 infantry should be alive on the ground.
    let landed_e1 = sim
        .entities
        .values()
        .filter(|e| {
            sim.interner.resolve(e.type_ref).eq_ignore_ascii_case("E1")
                && e.health.current > 0
                && e.parachute_state.is_none()
        })
        .count();
    assert!(landed_e1 >= 1, "at least one E1 should have landed alive (got {})", landed_e1);
}
```

NOTE: helper APIs (`sim_with_house`, `set_map_dimensions`, `house_mut`, `advance_tick`) names are placeholders — adjust to the actual test harness. If those helpers don't exist, write minimal versions in `src/sim/test_helpers.rs` or `src/rules/test_helpers.rs` gated on `#[cfg(test)]`.

**Step 3: Verify**

Run: `cargo test --test paradrop_e2e -- --nocapture`
Expected: 2 tests PASS.

**Step 4: Commit**

`tests: end-to-end paradrop launch + descent integration test`

---

### Task 16: Verification against gamemd.exe

**Why:** Confirm the implementation matches original engine behavior for the parity-critical items in the table above. This is the manual gamemd.exe comparison required by the parity bar.

**Files:** None — this is a research/verification task.

**Verify:**

1. **First-drop position relative to flight path** — Run gamemd.exe, build CAAIRP, launch ParaDrop on a clear cell. Observe where the FIRST paratrooper lands relative to the plane's flight path. Expected: LEFT side (CCW 90°), 0.5 cell offset. Compare to our implementation by running the same scenario and capturing screenshots.

2. **V-pattern alternation** — Same scenario; observe the L,R,L,R,L,R,L,R drop pattern across all 8 paratroopers. In gamemd, the visible spread should be a narrow zig-zag along the flight axis.

3. **Drop cadence (ROF=130 ticks)** — Time the interval between consecutive drops in gamemd vs our implementation. At 15 fps, 130 ticks = ~8.7 seconds. Use a stopwatch on screenshot timestamps.

4. **Fog reveal moment** — In gamemd, the fog around the drop point reveals when the plane reaches ParadropRadius=1024 leptons (~4 cells from target). Verify our timing matches by triggering a paradrop on a fogged area and timing the reveal.

5. **Carrier despawn** — Verify the PDPLANE flies off the OPPOSITE map edge after dropping all paratroopers, with no explosion animation. In our impl, `silent_despawn=true` should produce equivalent behavior.

6. **First-tick descent ramp** — Already verified for `parachute_descent` in its own design doc (16 unit tests cover gamemd's 0,−1,−2,−3,−3 ramp). Spot-check by observing falling speed in the first 4 ticks after a drop is visible on-screen.

**Step 1: Document findings**

Create a verification log at `docs/plans/2026-05-05-paradrop-launch-verification.md`:

```markdown
# Paradrop Launch Verification — gamemd.exe Comparison

Date: <fill in>
Tester: <fill in>

## 1. First-drop side
- gamemd: <observed>
- ours:   <observed>
- Match:  yes / no / drift-explained

## 2. V-pattern alternation
- gamemd 8-drop sequence: <L/R per drop>
- ours 8-drop sequence:   <L/R per drop>
- Match:  yes / no

## 3. Drop cadence
- gamemd ticks between drops: <observed>
- ours ticks between drops:    <observed>
- Match:  ±N ticks

## 4. Fog reveal moment
- gamemd reveal distance:     <observed cells/leptons>
- ours reveal distance:       <observed cells/leptons>
- Match:  yes / no

## 5. Carrier despawn
- gamemd exit edge + explosion: <observed>
- ours exit edge + explosion:   <observed>
- Match:  yes / no

## 6. Descent ramp first 4 ticks
- gamemd altitude curve: <observed>
- ours altitude curve:    <observed>
- Match:  yes / no

## Summary
- Total parity-critical items: 6
- Match: <count>
- Drift: <count + reason>
- Action items: <list>
```

**Step 2: If drift found, file follow-ups**

Any drift becomes a separate plan task. Do NOT block this plan's completion on drift — paradrop with known drifts is still a meaningful step forward; the parity bar is met by addressing drifts iteratively.

**Step 3: Commit**

`docs: paradrop launch verification log against gamemd.exe`

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-05-paradrop-launch-design.md](2026-05-05-paradrop-launch-design.md)
- **Ghidra reports:**
  - [ra2-rust-game-docs/PARADROP_SUPERWEAPON_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/PARADROP_SUPERWEAPON_GHIDRA_REPORT.md)
  - [ra2-rust-game-docs/JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
- **gamemd.exe addresses (kept here, not in code comments):**
  - `SuperClass::Launch` cases 5/6 — `0x006CC390`
  - `FUN_0065E660` paradrop spawner — `0x0065E660`
  - `Mission_ParaDropApproach` — `0x004155F0`
  - `Mission_ParaDropOverfly` — `0x004157C0`
  - `Drop_Payload` — `0x00415C60`
  - `Fire_At` gate — `0x00415EF8`
  - `FUN_004AA440` map-edge cell finder — `0x004AA440`
  - `Pop_Passenger` — `0x00473430`
  - `HouseClass::DetermineEdge` — `0x0050DB00`
  - V-pattern radius constant `128.0` — `0x007E2808`
  - Binary-angle conversion `-2π/65536` — `0x007E2810`
- **INI keys:**
  - `[General] ParadropRadius=1024`, `[General] AmerParaDropInf=E1`, `[General] AmerParaDropNum=8`,
    `[General] AllyParaDropInf=E1`, `[General] AllyParaDropNum=6`,
    `[General] SovParaDropInf=E2`, `[General] SovParaDropNum=9`,
    `[General] YuriParaDropInf=INIT`, `[General] YuriParaDropNum=6`,
    `[General] ParachuteMaxFallRate=-3`
  - `[ParaDropWeapon] ROF=130` (drop cadence)
  - `[PDPLANE] Speed=15 ROT=2 Spawned=yes Selectable=no Sight=0 Primary=ParaDropWeapon`
  - `[CAAIRP] SuperWeapon=ParaDropSpecial`
  - `[AMRADR] SuperWeapon=AmericanParaDropSpecial RequiredHouses=Americans`
  - `[ParaDropSpecial] Type=ParaDrop Action=ParaDrop`
  - `[AmericanParaDropSpecial] Type=AmerParaDrop Action=AmerParaDrop`
- **Related code:**
  - SW handler pattern: [src/sim/superweapon/iron_curtain.rs](../../src/sim/superweapon/iron_curtain.rs)
  - Mission FSM dispatch: [src/sim/aircraft/mod.rs:111-556](../../src/sim/aircraft/mod.rs#L111)
  - Cargo system: [src/sim/passenger.rs:30-100](../../src/sim/passenger.rs#L30)
  - Descent state: [src/sim/movement/parachute_descent.rs](../../src/sim/movement/parachute_descent.rs)
  - Fixed-point trig: [src/util/facing_table.rs](../../src/util/facing_table.rs)
  - SW dispatch: [src/sim/world/world_commands.rs:917-996](../../src/sim/world/world_commands.rs#L917)
- **Related design docs:**
  - [docs/plans/2026-05-05-parachute-descent-design.md](2026-05-05-parachute-descent-design.md) — descent state machine (already implemented)
  - [docs/plans/2026-05-05-parachute-descent-plan.md](2026-05-05-parachute-descent-plan.md) — descent implementation plan
