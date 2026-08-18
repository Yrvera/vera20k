# TIBTRE Ore Spawning — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Implement TIBTRE (Tiberium Tree) periodic ore spawning so YR maps' starting-ore patches replenish over a long match, matching gamemd.exe's `TerrainClass::AI` → `CellClass::SpreadTiberium(force=true)` path.

**Architecture:** Sim-side flat `BTreeMap<(u16, u16), TerrainSpawnerState>` keyed by cell, populated at map load from `app.terrain_objects`, ticked once per sim tick in Phase 7 right after `tick_ore_growth`. Mirrors the `production.ore_growth_state` pattern. NOT entity-shaped — TIBTRE is `Immune=yes` and has no movement/combat/death lifecycle to justify entityhood.

**Design:** No separate design doc — small enough to skip per user direction. Brainstorm record in conversation history; this plan is the spec.

---

## Grounding Summary

**Docs (R1):** [TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md) (HIGH confidence, 2026-03-31). Documents `TerrainClass::AI` (0x71C730), `CellClass::SpreadTiberium` (0x483780) with `force=true` semantics, `CellClass::CanAcceptTiberium` (0x4838E0) checks, and TerrainTypeClass field offsets (0x2A0 AnimationRate, 0x2A4 AnimationProbability, 0x2B1 SpawnsTiberium, 0x2B3 IsAnimated). param_1 in TerrainTypeClass functions is `int*`, indices direct byte (no ×4 multiplication).

**Ghidra (R2):** Verified `TerrainClass::AI` (0x71C730) has no SpecialFlags gate, no game-mode check — only internal gates are `+0x2B1` and `+0x2B3` flags. Single xref to a vtable slot at 0x7F5288 (TerrainClass vtable +0x5C, slot 23 = ObjectClass `AI()`) — invoked via standard per-tick virtual dispatch. `CellClass::SpreadTiberium` has exactly 2 callers: `TerrainClass::AI` (TIBTRE path) and `TiberiumClass::SpreadProcessor` (global ore spread). force=true bypasses the `TiberiumSpreads` SpecialFlags check.

**Repo pattern (R3):** Mirrors `tick_ore_growth` ([src/sim/ore_growth.rs:156](../../src/sim/ore_growth.rs#L156)). State on `ProductionState` ([src/sim/production/production_types.rs:195-216](../../src/sim/production/production_types.rs#L195)). Hook in Phase 7 of `advance_tick` ([src/sim/world/mod.rs:1320](../../src/sim/world/mod.rs#L1320)). Map-load seeding mirrors `seed_resource_nodes_from_overlays` ([src/sim/production/production_queue.rs:125](../../src/sim/production/production_queue.rs#L125)), called from [src/app_init.rs:553-554](../../src/app_init.rs#L553).

**INI (R4):** Repo's `ini/rulesmd.ini` has `[TIBTRE01]/[TIBTRE02]/[TIBTRE03]` ([lines 28109-28152](../../ini/rulesmd.ini#L28109)) with `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`, `Immune=yes`. Repo INI matches retail behavior. Detection by name pattern would be brittle — detect via parsed `spawns_tiberium` flag instead.

**Recent git activity:** `git log --oneline -10` against all touched files shows no commits invalidating the design (last touch on `ore_growth.rs` predates 2026-04). Recent activity (Phase 2 parachute_descent at 618f4f4, Phase 4.6 deploy_state at 768d6a6, Phase 5.5 particle_systems at 986533b) added phases between but did not modify Phase 7's ore_growth section. Plan's hook line offset may shift — anchor by surrounding code, not line number.

**Unknowns:** the binary's `CanAcceptTiberium` check "no other SpawnsTiberium TerrainClass on cell" — we verify "no other tibtre at the same cell as candidate" by checking our own `terrain_spawners` map for the candidate cell.

**Resolved during plan revision (post-grounding):**
- `[TerrainTypes]` is a numbered registry (`1=NAME`, `2=NAME`, ...) — reuse existing `parse_registry` helper at [src/rules/ruleset.rs:1363](../../src/rules/ruleset.rs#L1363).
- `rules` at app_init.rs:553 is `Option<RuleSet>` — must handle `None` (skip seeding gracefully, match `tick_ore_growth`'s pattern of taking pre-baked config).
- `tick_terrain_spawners` does NOT take `rules` at the tick site — instead, cache `animation_probability_micros` into `TerrainSpawnerState` at seed time (mirrors how `OreGrowthConfig` bakes config from rules at map load). Eliminates per-tick rules lookup.
- `Simulation::state_hash()` confirmed at [src/sim/world/world_hash.rs:18](../../src/sim/world/world_hash.rs#L18).

---

## Key Technical Decisions

- **Sim resource (flat BTreeMap), not entity-shaped** — `Immune=yes` removes the case for entityhood. **Confidence:** high. **Source:** brainstorm conversation; INI flag at rulesmd.ini:28121.
- **Single-phase animation collapse** — roll succeeds → spawn immediately, skipping the binary's animation-midpoint countdown. Visually identical to player (anim is render-only). **Confidence:** high (player-observable behavior is the spawn cadence, which is preserved on average). **Source:** decompile of `TerrainClass::AI` shows the midpoint check is purely an animation-timing gate; spawn rate average is unchanged.
- **Probability comparison via integer micros (× 1_000_000)** — match binary's `random % 1_000_000 < probability * 1_000_000` pattern; avoid f32 in sim hot path. **Confidence:** high. **Source:** Ghidra decompile of TerrainClass::AI (0x71C730) shows `random % 1_000_000` against AnimationProbability scaled by `_DAT_007ef918 = 1.0e-6`.
- **New TerrainObjectType struct** (separate from `ObjectType`) — TIBTRE has no weapons/movement/factory category, doesn't fit the unit/building/infantry shape. Naming: avoid `TerrainType` (collides with the existing `terrain_rules::TerrainClass`/`TerrainRules` for land semantics). **Confidence:** high.
- **`default_ore_overlay_id`** computed at map load from `overlay_names` (first name where `name.to_uppercase().starts_with("TIB")`), stored on `ProductionState`. Used as the visual overlay_id when TIBTRE spawns ore on an empty cell (the binary uses `TiberiumClass::OverlayType->ArrayIndex`; we don't have that registry yet, so we pick the first ore overlay we know about — produces visually correct ore on the map). **Confidence:** medium. The visual overlay_id only matters for rendering; the resource_node remains correct regardless. **Source:** repo pattern from `seed_resource_nodes_from_overlays` (production_queue.rs:142-147) detecting "TIB" prefix.

---

## Open Questions

### Resolved During Planning

- **Where does TIBTRE per-instance state live?** → `production.terrain_spawners: BTreeMap<(u16,u16), TerrainSpawnerState>`. Resolved via brainstorm.
- **Is TerrainType already parsed?** → No. `src/rules/terrain_rules.rs` parses LAND types (Clear/Rough/Water), not terrain-object types (TIBTRE). New parser surface needed.
- **How is the tick function hooked?** → New `tick_terrain_spawners` call in `advance_tick` Phase 7, immediately after the `tick_ore_growth` block at world/mod.rs:1320.
- **What overlay_id does spawned ore use?** → `default_ore_overlay_id` resolved at map load by scanning `overlay_names` for first "TIB"-prefixed entry. See Key Technical Decisions.
- **Where is map-load wiring?** → [src/app_init.rs:552-575](../../src/app_init.rs#L552) — the `if let Some(sim) = &mut simulation` block, immediately after the existing `seed_resource_nodes_from_overlays` call.

### Deferred to Implementation

- **What's the exact spawn cadence in-game?** Math says 0.003 per 15Hz tick → ~333 ticks ≈ 22.2s expected interval. Confirm empirically by running the determinism test for 60 sim seconds and counting spawns; should be ~3 per tree.
- **Does our `PathGrid::is_walkable` match `CanAcceptTiberium`'s land-type check?** Approximate match — both reject buildings/cliffs/water. Exact equivalence to the binary's `RTTI 6 with health > 0` check requires verifying at the call site; if drift surfaces (ore spawning on top of buildings) it's a one-line fix.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/rules/terrain_object_type.rs` | Parse `[TIBTRE*]`-style sections: `spawns_tiberium`, `is_animated`, `animation_rate`, `animation_probability_micros` |
| Modify | `src/rules/mod.rs` | Declare new module |
| Modify | `src/rules/ruleset.rs` | Add `terrain_object_types: BTreeMap<String, TerrainObjectType>` and lookup helper |
| Create | `src/sim/terrain_spawn.rs` | `TerrainSpawnerState` struct + `tick_terrain_spawners` + `seed_terrain_spawners` |
| Modify | `src/sim/mod.rs` | Declare new module |
| Modify | `src/sim/production/production_types.rs:195-234` | Add `terrain_spawners` and `default_ore_overlay_id` fields to `ProductionState` + Default |
| Modify | `src/sim/world/mod.rs:1320` | Hook `tick_terrain_spawners` into Phase 7 after `tick_ore_growth` |
| Modify | `src/sim/world/world_hash.rs:154-159` | Extend hash to cover `terrain_spawners` and `default_ore_overlay_id` |
| Modify | `src/app_init.rs:552-575` | Call `seed_terrain_spawners` after `seed_resource_nodes_from_overlays`; resolve `default_ore_overlay_id` from `overlay_names` |

---

## Interface Changes

- **New public types:** `crate::rules::terrain_object_type::TerrainObjectType` (parsed rules data), `crate::sim::terrain_spawn::TerrainSpawnerState` (per-cell sim state).
- **`ProductionState` adds two fields:** `terrain_spawners: BTreeMap<(u16,u16), TerrainSpawnerState>` and `default_ore_overlay_id: Option<u8>`. Both `Default::default()` to empty/None — existing tests that build `ProductionState::default()` continue to compile and run unchanged. Snapshot/replay state shape changes — extend serde derives.
- **`RuleSet` adds:** `terrain_object_types: BTreeMap<String, TerrainObjectType>` (sorted by name for determinism) + `terrain_object_type_case_insensitive(name: &str) -> Option<&TerrainObjectType>` (mirrors `object_case_insensitive`).
- **No public API on existing types changes.** All additions are new fields + new functions.

---

## Sim Checklist

- [x] All math uses fixed-point or integer — probability stored as `u32` micros, not `f32`. Compared via `rng.next_range_u32(1_000_000)`.
- [x] New state included in deterministic state hash — Task 8 extends world_hash.rs.
- [x] No dependencies on render/ui/sidebar/audio/net — `terrain_spawn.rs` imports only `rules/`, `sim/production`, `sim/overlay_grid`, `sim/pathfinding`, `sim/rng`. Data flow: rules → sim. No upward deps.
- [x] Tick ordering impact noted — runs in Phase 7 after `tick_ore_growth`. Reads/writes the same `resource_nodes` and `overlay_grid` ore_growth touches; sequential is required (not concurrent). Adding after ore_growth means a TIBTRE spawn on tick T cannot influence that tick's growth/spread but will be visible to tick T+1 — matches binary semantics (terrain AI runs once per tick after the per-tick world updates).
- [x] BTreeMap iteration order considered — `terrain_spawners` keyed by (rx, ry) gives deterministic sorted iteration; one RNG draw per spawner per tick maintains lockstep order.

---

## Risk Areas

- **Determinism state hash** — adding fields without extending `world_hash.rs` would silently corrupt replay/lockstep. Task 8 is mandatory before any merge.
- **Map-load ordering** — `seed_terrain_spawners` must run after `OverlayGrid` is initialized (so we can resolve `default_ore_overlay_id` from `overlay_names`). Wire AFTER the existing `seed_resource_nodes_from_overlays` call which already has `overlay_names` in scope.
- **Naming collision** — repo already has `TerrainClass` (in `terrain_rules.rs`, an enum for land semantics) and `TerrainRules`. New struct named `TerrainObjectType` to avoid both, and to mirror `ObjectType` shape.
- **f32 in INI parse path is OK** — `AnimationProbability=.003` is a float in the INI. Convert to integer `animation_probability_micros: u32` at parse time so the sim hot path never sees f32.

---

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | `animation_probability_micros = (probability * 1_000_000.0).round() as u32` | Probability comparison must match binary's `random % 1_000_000 < prob*1e6` pattern. Float arithmetic order matters: round at INI parse, not at compare time. | Unit test in Task 1: `[TIBTRE01]\nAnimationProbability=.003\n` parses to `animation_probability_micros == 3000` |
| Task 4 | Density on spawn = 3 (not 1) | Binary calls `PlaceTiberium(tib_type, 3)`. Our existing ore_growth uses density 1 for spread. Drift here halves TIBTRE's ore-replenishment rate. | Unit test: spawn on empty cell creates `ResourceNode { remaining: 3 * 120 = 360 }` |
| Task 4 | Place is additive on existing ore | Binary doc §3: "Increases density by 3 if ore already exists". Our `can_germinate` rejects existing nodes — drift would make TIBTRE useless once a cell has any ore. | Unit test: spawn on cell with 240 ore (2 levels) → cell has 240 + 360 = 600 ore (capped at MAX_ORE_REMAINING) |
| Task 4 | 8-direction random-start iteration | Binary picks random start direction from 8 facings, iterates all 8. Same shape as `try_spread_ore` in ore_growth.rs:296. | Unit test: with seeded RNG, spawn directions over many trials cover all 8 cells |
| Task 4 | Gem-cell rejection happens in `can_accept_tiberium`, NOT `place_tiberium_additive` | Bug caught in `/review-plan`: if rejection were inside the place fn, hitting a gem on the first acceptable direction would consume the single placement chance silently. Putting it in the accept-check lets the 8-direction loop continue past gems to find an empty cell. | Unit test `spawn_skips_gem_neighbors_and_picks_empty_cell`: 7 gem neighbors + 1 empty SE cell → ore lands on the empty cell, gems untouched |
| Task 4 | force=true bypasses TiberiumSpreads | Binary's `force=1` skips the global `TiberiumSpreads` check. We must NOT gate `tick_terrain_spawners` on `OreGrowthConfig.spreads`. | Unit test: ore_growth_config has `spreads=false`, TIBTRE still spawns |
| Task 4 | Tib type defaults to `ResourceType::Ore` (Riparius) | Binary defaults to type 0 when TIBTRE cell has no overlay. We have only Ore/Gem in our enum; Ore is the right map. | Unit test: spawned node has `resource_type == ResourceType::Ore` |
| Task 7 | Hook order: AFTER `tick_ore_growth`, BEFORE Phase 8 (AI) | Spawning before ore_growth would let new ore "grow" within the same tick — drift. Spawning after `if spawned_entities { refresh_fog }` is fine because TIBTRE doesn't spawn entities. | Visual: insert call between `ore_growth::tick_ore_growth(...)` and the `if spawned_entities` block |
| Task 8 | Hash both `terrain_spawners` and `default_ore_overlay_id` | Without this, replay/lockstep desyncs after the first TIBTRE spawn. | Determinism test: same seed, 1000 ticks, two `Simulation` instances produce identical state hash |

---

## Tasks

### Task 1: Define `TerrainObjectType` parser

**Why:** Parser surface for `[TIBTRE01/02/03]` and any other `SpawnsTiberium=yes` terrain object types in rules.ini. New surface — no precedent for terrain-object-type INI parsing in the codebase.

**Files:**
- Create: `src/rules/terrain_object_type.rs`
- Modify: `src/rules/mod.rs` (add `pub mod terrain_object_type;`)

**Pattern:** Mirror the shape of `crate::rules::object_type::ObjectType::from_ini_section` ([src/rules/object_type.rs:781-784](../../src/rules/object_type.rs#L781) for SlavesNumber-style int parsing). Use the existing `IniSection::get_bool`, `IniSection::get_i32`, `IniSection::get` helpers.

**Step 1: Define the struct**
```rust
//! Parsing for `[TIBTRE*]`-style terrain object types (rules.ini sections).
//!
//! Distinct from `terrain_rules` (which parses LAND types like Clear/Rough/Water).
//! These are per-object-type definitions for terrain decorations — currently
//! only TIBTRE (Tiberium Tree) is consumed by sim. Other terrain objects
//! (TREE01, ROCK01, etc.) parse to the same struct but have all-default flags
//! and are ignored by the spawner system.

use crate::rules::ini_parser::IniSection;

/// Type-class data for a terrain object (e.g. `[TIBTRE01]`).
///
/// Only the fields the sim needs; render-only fields (LightVisibility, tints,
/// IsFlammable) are intentionally not parsed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TerrainObjectType {
    /// Section name, e.g. "TIBTRE01".
    pub name: String,
    /// `SpawnsTiberium=yes` — periodically spawns ore in adjacent cells.
    pub spawns_tiberium: bool,
    /// `IsAnimated=yes` — required gate for SpawnsTiberium logic in gamemd.exe.
    pub is_animated: bool,
    /// `AnimationRate=` (frames per anim step). Currently parsed but unused
    /// in sim — animation timing is collapsed to single-phase. Kept for
    /// future render-side use and to surface mod-tuning differences.
    pub animation_rate: u8,
    /// `AnimationProbability=` × 1_000_000, stored as integer micros.
    /// Used directly in the sim tick: `rng.next_range_u32(1_000_000) < this`.
    /// Avoids f32 in the hot path.
    pub animation_probability_micros: u32,
}

impl TerrainObjectType {
    pub fn from_ini_section(name: &str, section: &IniSection) -> Self {
        let probability_f = section
            .get("AnimationProbability")
            .and_then(|s| s.trim().parse::<f32>().ok())
            .unwrap_or(0.0);
        let animation_probability_micros: u32 =
            (probability_f.clamp(0.0, 1.0) * 1_000_000.0).round() as u32;

        Self {
            name: name.to_string(),
            spawns_tiberium: section.get_bool("SpawnsTiberium").unwrap_or(false),
            is_animated: section.get_bool("IsAnimated").unwrap_or(false),
            animation_rate: section
                .get_i32("AnimationRate")
                .unwrap_or(0)
                .clamp(0, 255) as u8,
            animation_probability_micros,
        }
    }
}
```

**Step 2: Add tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn parse_tibtre_section_defaults() {
        let ini = IniFile::from_str(
            "[TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\n\
             AnimationRate=3\nAnimationProbability=.003\n",
        );
        let section = ini.section("TIBTRE01").expect("section");
        let t = TerrainObjectType::from_ini_section("TIBTRE01", section);
        assert_eq!(t.name, "TIBTRE01");
        assert!(t.spawns_tiberium);
        assert!(t.is_animated);
        assert_eq!(t.animation_rate, 3);
        // 0.003 * 1_000_000 = 3000.0
        assert_eq!(t.animation_probability_micros, 3000);
    }

    #[test]
    fn parse_non_spawning_terrain_section() {
        let ini = IniFile::from_str("[TREE01]\nIsAnimated=no\n");
        let section = ini.section("TREE01").expect("section");
        let t = TerrainObjectType::from_ini_section("TREE01", section);
        assert!(!t.spawns_tiberium);
        assert!(!t.is_animated);
        assert_eq!(t.animation_probability_micros, 0);
    }

    #[test]
    fn animation_probability_clamps_above_one() {
        let ini = IniFile::from_str("[X]\nAnimationProbability=2.5\n");
        let section = ini.section("X").expect("section");
        let t = TerrainObjectType::from_ini_section("X", section);
        assert_eq!(t.animation_probability_micros, 1_000_000);
    }
}
```

**Step 3: Wire module declaration**

Add to `src/rules/mod.rs`:
```rust
pub mod terrain_object_type;
```
(Alongside the other `pub mod` lines.)

**Step 4: Verify**
Run: `cargo test --lib rules::terrain_object_type`
Expected: 3 tests pass.

**Step 5: Commit** — `rules: parse TerrainObjectType for TIBTRE-style sections`

---

### Task 2: Wire `TerrainObjectType` lookup into `RuleSet`

**Why:** `tick_terrain_spawners` and the map-load seeding need to look up a TerrainObjectType by name. Mirrors the existing `object_case_insensitive` helper.

**Files:**
- Modify: `src/rules/ruleset.rs`

**Pattern:** Mirror existing `objects: BTreeMap<String, ObjectType>` field and `object_case_insensitive` lookup.

**Step 1: Add field to `RuleSet`**

Find the `RuleSet` struct definition. Add field alongside `objects`:
```rust
    /// Terrain object type definitions (TIBTRE*, TREE*, ROCK*, etc.) keyed by
    /// uppercase section name for deterministic iteration.
    pub terrain_object_types: BTreeMap<String, TerrainObjectType>,
```

Add import at the top of the file:
```rust
use crate::rules::terrain_object_type::TerrainObjectType;
```

**Step 2: Populate during `from_ini`**

The `[TerrainTypes]` section is a numbered registry (`1=BOXES01`, `2=BOXES02`, ..., `108=TIBTRE01`, etc. per [ini/rulesmd.ini:1598](../../ini/rulesmd.ini#L1598)). Reuse the existing `parse_registry` helper at [src/rules/ruleset.rs:1363](../../src/rules/ruleset.rs#L1363) — same shape as `[InfantryTypes]/[VehicleTypes]/[BuildingTypes]`.

Find `RuleSet::from_ini` (currently builds `objects` via `parse_registry` per category). After the `objects` BTreeMap construction, add:
```rust
    let mut terrain_object_types: BTreeMap<String, TerrainObjectType> = BTreeMap::new();
    let terrain_names: Vec<String> = parse_registry(ini, "TerrainTypes");
    for name in &terrain_names {
        if let Some(type_section) = ini.section(name) {
            terrain_object_types.insert(
                name.to_ascii_uppercase(),
                TerrainObjectType::from_ini_section(name, type_section),
            );
        }
    }
```

Add to the `Self { ... }` constructor block:
```rust
            terrain_object_types,
```

Default value (in `RuleSet::default` or wherever the empty case is built):
```rust
            terrain_object_types: BTreeMap::new(),
```

**Step 3: Add lookup helper**

Below `object_case_insensitive` in the `impl RuleSet` block:
```rust
    /// Look up a TerrainObjectType by section name, case-insensitive.
    pub fn terrain_object_type_case_insensitive(&self, name: &str) -> Option<&TerrainObjectType> {
        self.terrain_object_types.get(&name.to_ascii_uppercase())
    }
```

**Step 4: Add test**

In the existing `#[cfg(test)] mod tests` block of `ruleset.rs`:
```rust
    #[test]
    fn from_ini_loads_tibtre_terrain_object_types() {
        let ini = IniFile::from_str(
            "[TerrainTypes]\n1=TIBTRE01\n\
             [TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\n\
             AnimationRate=3\nAnimationProbability=.003\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let t = rules
            .terrain_object_type_case_insensitive("tibtre01")
            .expect("TIBTRE01 should be parsed");
        assert!(t.spawns_tiberium);
        assert_eq!(t.animation_probability_micros, 3000);
    }
```

**Step 5: Verify**
Run: `cargo test --lib rules::ruleset`
Expected: existing tests still pass, new test passes.

**Step 6: Commit** — `rules: load TerrainObjectType list from rulesmd.ini`

---

### Task 3: Add `TerrainSpawnerState` + `ProductionState` fields

**Why:** Storage for per-cell tick state. Must land before the tick function so types are in scope.

**Files:**
- Create stub: `src/sim/terrain_spawn.rs` (just struct + module skeleton — tick fn lands in Task 4)
- Modify: `src/sim/mod.rs` (add `pub mod terrain_spawn;`)
- Modify: `src/sim/production/production_types.rs:195-234`

**Pattern:** Mirror `OreGrowthState` shape (serde-derived, keyed by cell, lives on ProductionState).

**Step 1: Create stub module**

`src/sim/terrain_spawn.rs`:
```rust
//! TIBTRE-style terrain object ore spawning.
//!
//! Per-cell sim state for terrain objects with `SpawnsTiberium=yes`. Each tick,
//! every spawner rolls its `AnimationProbability`; on success it places ore
//! in a random adjacent walkable cell at density 3, additive on existing ore.
//!
//! Mirrors gamemd.exe's `TerrainClass::AI` (0x71C730) calling
//! `CellClass::SpreadTiberium(force=true)` (0x483780), which bypasses the
//! global `TiberiumSpreads` SpecialFlag.
//!
//! ## Animation model
//! Single-phase: roll succeeds → spawn immediately. The binary's two-phase
//! (roll → animation midpoint countdown → spawn) is collapsed because the
//! animation visual is render-only and the spawn-rate average is identical.
//! See plan doc 2026-05-05-tibtre-ore-spawning-plan.md for the rationale.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/overlay_grid, sim/pathfinding,
//!   sim/rng, sim/miner (ResourceNode/ResourceType).
//! - The tick function does NOT depend on rules/ — config is baked into
//!   TerrainSpawnerState at seed time (mirrors OreGrowthConfig pattern).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::intern::InternedId;

/// Per-instance state for one TIBTRE-style spawner placed on the map.
///
/// Keyed by cell in `ProductionState::terrain_spawners`. The spawner doesn't
/// move and isn't destroyable (`Immune=yes` on TIBTRE in YR), so the only
/// lifecycle is "exists from map load to game end".
///
/// `animation_probability_micros` is cached at seed time so the tick function
/// doesn't need to look up rules — same pattern as `OreGrowthConfig` baking
/// `growth_rate_seconds` from rules at map load.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TerrainSpawnerState {
    /// Interned name of the TerrainObjectType (e.g. "TIBTRE01"). Kept for
    /// debug logging and future render-side visual lookup; NOT used by the
    /// tick function.
    pub type_ref: InternedId,
    /// Cached `AnimationProbability * 1_000_000` from rules at seed time.
    /// The tick rolls `rng.next_range_u32(1_000_000) < this` directly.
    /// 0 = never spawns (e.g. type was found but `SpawnsTiberium=no` —
    /// in that case the seed function wouldn't insert a spawner, so this
    /// case is mostly defensive).
    pub animation_probability_micros: u32,
}
```

**Step 2: Wire module declaration**

Add to `src/sim/mod.rs` (alongside `pub mod ore_growth;`):
```rust
pub mod terrain_spawn;
```

**Step 3: Extend `ProductionState`**

In `src/sim/production/production_types.rs` around line 209 (after `slave_bindings`):
```rust
    /// TIBTRE-style ore-spawning terrain objects, keyed by map cell.
    /// Populated at map load from `app.terrain_objects` filtered by
    /// `SpawnsTiberium=yes` on the matching TerrainObjectType.
    pub terrain_spawners:
        std::collections::BTreeMap<(u16, u16), crate::sim::terrain_spawn::TerrainSpawnerState>,
    /// Default overlay_id used for new ore cells spawned by terrain_spawners.
    /// Resolved at map load by scanning the overlay_names registry for the
    /// first "TIB"-prefixed entry. None if no ore overlay is registered
    /// (TIBTRE will still update resource_nodes; only the visual sprite is missing).
    pub default_ore_overlay_id: Option<u8>,
```

In `ProductionState::default`:
```rust
            terrain_spawners: BTreeMap::new(),
            default_ore_overlay_id: None,
```

**Step 4: Verify**
Run: `cargo build --lib`
Expected: compiles. No tests yet — those come with Task 4.

**Step 5: Commit** — `sim: add TerrainSpawnerState + ProductionState terrain_spawners`

---

### Task 4: Implement `tick_terrain_spawners`

**Why:** Core spawn logic. Runs per tick, rolls probability, places ore on hit.

**Files:**
- Modify: `src/sim/terrain_spawn.rs` (append to the stub from Task 3)

**Pattern:** Mirror `try_spread_ore` ([src/sim/ore_growth.rs:296-338](../../src/sim/ore_growth.rs#L296)) for direction iteration. Place primitive is **additive** (different from `can_germinate`), so do not call `can_germinate` directly — write a small variant inline.

**Step 1: Add tick function**

Append to `src/sim/terrain_spawn.rs`:
```rust
use std::collections::BTreeMap;

use crate::sim::miner::{ResourceNode, ResourceType};
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::rng::SimRng;

/// 8 adjacent directions (matches ore_growth::ADJACENT_OFFSETS layout):
/// N, NE, E, SE, S, SW, W, NW.
const ADJACENT_OFFSETS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// Base ore stock per density level. Matches ore_growth::ORE_BASE_PER_LEVEL
/// and seed_resource_nodes_from_overlays (production_queue.rs).
const ORE_BASE_PER_LEVEL: u16 = 120;
/// Maximum ore stock (12 levels × 120). Matches ore_growth::MAX_ORE_REMAINING.
const MAX_ORE_REMAINING: u16 = ORE_BASE_PER_LEVEL * 12;
/// Density levels added per TIBTRE spawn. Binary calls `PlaceTiberium(tib_type, 3)`.
const SPAWN_DENSITY_LEVELS: u16 = 3;
/// Probability roll denominator. Binary uses `random % 1_000_000` against
/// AnimationProbability scaled by 1.0e-6 (DAT_007ef918).
const PROBABILITY_DENOMINATOR: u32 = 1_000_000;

/// Tick all terrain spawners.
///
/// Called once per sim tick from `World::advance_tick` (Phase 7), AFTER
/// `tick_ore_growth` so a TIBTRE spawn this tick can't be grown/spread by
/// `tick_ore_growth` until the next tick — matches gamemd.exe ordering.
///
/// **Determinism contract:**
/// - BTreeMap iteration is deterministic (sorted by cell).
/// - One `rng.next_range_u32(PROBABILITY_DENOMINATOR)` per spawner per tick.
/// - On hit, one `rng.next_range_u32(8)` for direction start.
/// - Same seed + same map → identical spawn pattern across runs.
pub fn tick_terrain_spawners(
    spawners: &BTreeMap<(u16, u16), TerrainSpawnerState>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    default_ore_overlay_id: Option<u8>,
    path_grid: Option<&PathGrid>,
    rng: &mut SimRng,
) {
    if spawners.is_empty() {
        return;
    }

    // To avoid borrowing overlay_grid through the loop, take ownership of the
    // mutable reference once and reborrow per-call. Using Option<&mut ...> here
    // lets the helper take an Option<&mut ...> downstream.
    let mut overlay_grid = overlay_grid;

    for (&(rx, ry), spawner) in spawners {
        // Probability gate — defensive against zero-probability entries
        // (seed function shouldn't insert these, but handle them safely).
        if spawner.animation_probability_micros == 0 {
            continue;
        }

        // Probability roll — integer comparison, parity with binary's
        // `random % 1_000_000 < probability * 1_000_000` pattern.
        let roll = rng.next_range_u32(PROBABILITY_DENOMINATOR);
        if roll >= spawner.animation_probability_micros {
            continue;
        }

        // On hit, try to spawn in a random adjacent cell.
        try_spawn_ore(
            (rx, ry),
            resource_nodes,
            overlay_grid.as_deref_mut(),
            default_ore_overlay_id,
            path_grid,
            spawners,
            rng,
        );
    }
}

/// Try to place ore in a random adjacent cell. Mirrors the 8-direction
/// random-start iteration from `ore_growth::try_spread_ore`, but uses
/// the additive density-3 place primitive (matches binary's
/// `PlaceTiberium(tib_type, 3)` semantics).
fn try_spawn_ore(
    source: (u16, u16),
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    default_ore_overlay_id: Option<u8>,
    path_grid: Option<&PathGrid>,
    spawners: &BTreeMap<(u16, u16), TerrainSpawnerState>,
    rng: &mut SimRng,
) {
    let start_dir = rng.next_range_u32(8) as usize;

    for i in 0..8 {
        let dir = (start_dir + i) % 8;
        let (dx, dy) = ADJACENT_OFFSETS[dir];
        let nx = source.0 as i32 + dx;
        let ny = source.1 as i32 + dy;
        if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
            continue;
        }
        let cell = (nx as u16, ny as u16);

        if !can_accept_tiberium(cell, resource_nodes, path_grid, spawners) {
            continue;
        }

        // Place — additive on existing ore, create at density 3 if empty.
        place_tiberium_additive(
            cell,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            default_ore_overlay_id,
        );
        return;
    }
}

/// Whether a cell can receive new ore from a terrain spawner.
///
/// Maps onto binary's `CanAcceptTiberium` (0x4838E0) checks:
/// - Cell walkable (rejects buildings/cliffs/water — approximate match for
///   "no living building" + "passable land type")
/// - No other SpawnsTiberium TerrainClass on the cell
/// - Cell does not already hold a non-ore resource (gems) — TIBTRE always
///   spawns Riparius (ore type 0) per binary, and we don't overwrite gems.
///   Rejecting here (rather than silently no-op'ing inside the place fn)
///   lets `try_spawn_ore` continue iterating through the remaining 7
///   neighbors instead of consuming its single placement chance on a gem.
///
/// Note: existing ORE on the cell is NOT a rejection reason — place is
/// additive in that case (matches binary's `PlaceTiberium(tib, 3)` semantics).
fn can_accept_tiberium(
    cell: (u16, u16),
    resource_nodes: &BTreeMap<(u16, u16), ResourceNode>,
    path_grid: Option<&PathGrid>,
    spawners: &BTreeMap<(u16, u16), TerrainSpawnerState>,
) -> bool {
    if let Some(grid) = path_grid {
        if !grid.is_walkable(cell.0, cell.1) {
            return false;
        }
    }
    // Don't spawn directly under another tibtre.
    if spawners.contains_key(&cell) {
        return false;
    }
    // Reject cells already holding a non-ore resource (gems). Existing ore
    // is fine — place_tiberium_additive will increment density there.
    if let Some(existing) = resource_nodes.get(&cell) {
        if existing.resource_type != ResourceType::Ore {
            return false;
        }
    }
    true
}

/// Place ore at `cell` with density `SPAWN_DENSITY_LEVELS`, additive on existing.
///
/// Caller (`try_spawn_ore`) must have already checked `can_accept_tiberium`,
/// which guarantees the cell is either empty or holds ore (not gems).
fn place_tiberium_additive(
    cell: (u16, u16),
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    default_ore_overlay_id: Option<u8>,
) {
    let density_stock: u16 = ORE_BASE_PER_LEVEL * SPAWN_DENSITY_LEVELS;

    let (new_remaining, was_empty) = match resource_nodes.get(&cell) {
        Some(existing) => {
            // can_accept_tiberium guarantees this is ore (not gems).
            debug_assert_eq!(existing.resource_type, ResourceType::Ore);
            let r = existing.remaining.saturating_add(density_stock);
            (r.min(MAX_ORE_REMAINING), false)
        }
        None => (density_stock, true),
    };

    resource_nodes.insert(
        cell,
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: new_remaining,
        },
    );

    // Sync the overlay grid for visual rendering. Use the source overlay_id
    // for the new cell if there is one, otherwise fall back to default_ore_overlay_id.
    if let Some(grid) = overlay_grid {
        let target_frame: u8 = (new_remaining / ORE_BASE_PER_LEVEL)
            .saturating_sub(1)
            .min(11) as u8;
        if was_empty {
            if let Some(id) = default_ore_overlay_id {
                grid.place_overlay(cell.0, cell.1, id, target_frame);
            }
        } else {
            grid.set_overlay_data(cell.0, cell.1, target_frame);
        }
    }
}
```

**Step 2: Add tests**

Append to `src/sim/terrain_spawn.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::intern::StringInterner;

    fn spawner(interner: &mut StringInterner, name: &str, prob_micros: u32) -> TerrainSpawnerState {
        TerrainSpawnerState {
            type_ref: interner.intern(name),
            animation_probability_micros: prob_micros,
        }
    }

    #[test]
    fn probability_one_always_spawns_within_one_tick() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);

        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        // Exactly one new ore node, in one of 8 adjacent cells.
        assert_eq!(resource_nodes.len(), 1);
        let &cell = resource_nodes.keys().next().unwrap();
        let dx = (cell.0 as i32 - 10).abs();
        let dy = (cell.1 as i32 - 10).abs();
        assert!(dx <= 1 && dy <= 1 && (dx + dy) > 0);
    }

    #[test]
    fn probability_zero_never_spawns() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE_NEVER", 0));
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);

        for _ in 0..1000 {
            tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        }
        assert!(resource_nodes.is_empty());
    }

    #[test]
    fn spawn_on_empty_cell_creates_density_3_ore() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);

        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        let node = resource_nodes.values().next().unwrap();
        assert_eq!(node.resource_type, ResourceType::Ore);
        // density 3 * base 120 = 360
        assert_eq!(node.remaining, 360);
    }

    #[test]
    fn spawn_is_additive_on_existing_ore() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        // Surround with 8 ore cells at density 2 (240 each) so the spawn
        // can't pick a fresh empty cell — has to add to one of them.
        let mut resource_nodes = BTreeMap::new();
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let cell = ((10 + dx) as u16, (10 + dy) as u16);
            resource_nodes.insert(
                cell,
                ResourceNode {
                    resource_type: ResourceType::Ore,
                    remaining: 240,
                },
            );
        }
        let mut rng = SimRng::new(7);

        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        // One neighbor must now be at 240 + 360 = 600.
        let added = resource_nodes
            .values()
            .filter(|n| n.remaining == 600)
            .count();
        assert_eq!(added, 1, "exactly one neighbor should grow by 360 stock");
    }

    #[test]
    fn spawn_never_overwrites_gem_cells() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        // All 8 neighbors are gems — TIBTRE must NOT overwrite.
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let cell = ((10 + dx) as u16, (10 + dy) as u16);
            resource_nodes.insert(
                cell,
                ResourceNode {
                    resource_type: ResourceType::Gem,
                    remaining: 360,
                },
            );
        }
        let mut rng = SimRng::new(7);
        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        // All gems unchanged.
        for n in resource_nodes.values() {
            assert_eq!(n.resource_type, ResourceType::Gem);
            assert_eq!(n.remaining, 360);
        }
    }

    #[test]
    fn spawn_skips_gem_neighbors_and_picks_empty_cell() {
        // Regression test for the gem-cell asymmetry caught in /review-plan:
        // when the random-start direction lands on a gem cell, try_spawn_ore
        // must continue iterating to find an empty neighbor, not consume its
        // single placement chance silently.
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        // 7 of 8 neighbors are gems; one cell — picked deliberately to NOT
        // be the start direction for seed 7 — is empty. Whichever of the 8
        // directions the RNG picks first, the loop must iterate past gems
        // until it finds the empty cell at offset (1, 1) (SE).
        for &(dx, dy) in &ADJACENT_OFFSETS {
            if (dx, dy) == (1, 1) {
                continue; // leave SE empty
            }
            let cell = ((10 + dx) as u16, (10 + dy) as u16);
            resource_nodes.insert(
                cell,
                ResourceNode {
                    resource_type: ResourceType::Gem,
                    remaining: 360,
                },
            );
        }
        let mut rng = SimRng::new(7);
        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        // The SE neighbor must now hold ore at density 3 = 360.
        let se = resource_nodes
            .get(&(11, 11))
            .expect("SE neighbor should exist after spawn");
        assert_eq!(se.resource_type, ResourceType::Ore);
        assert_eq!(se.remaining, 360);
        // Every gem cell must remain unchanged.
        for &(dx, dy) in &ADJACENT_OFFSETS {
            if (dx, dy) == (1, 1) {
                continue;
            }
            let cell = ((10 + dx) as u16, (10 + dy) as u16);
            let n = resource_nodes.get(&cell).expect("gem still present");
            assert_eq!(n.resource_type, ResourceType::Gem);
            assert_eq!(n.remaining, 360);
        }
    }

    #[test]
    fn spawn_caps_at_max_remaining() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        // Fill all 8 neighbors near max ore (12 × 120 = 1440).
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let cell = ((10 + dx) as u16, (10 + dy) as u16);
            resource_nodes.insert(
                cell,
                ResourceNode {
                    resource_type: ResourceType::Ore,
                    remaining: 1320, // 11 levels
                },
            );
        }
        let mut rng = SimRng::new(7);
        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        // The chosen neighbor should be capped at MAX_ORE_REMAINING = 1440.
        let capped = resource_nodes
            .values()
            .filter(|n| n.remaining == MAX_ORE_REMAINING)
            .count();
        assert_eq!(capped, 1);
    }

    #[test]
    fn deterministic_same_seed_same_pattern() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        // Use a probability between 0 and 1 to actually exercise the RNG path.
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE_HALF", 500_000));

        fn run(
            spawners: &BTreeMap<(u16, u16), TerrainSpawnerState>,
            seed: u64,
        ) -> BTreeMap<(u16, u16), ResourceNode> {
            let mut nodes = BTreeMap::new();
            let mut rng = SimRng::new(seed);
            for _ in 0..200 {
                tick_terrain_spawners(spawners, &mut nodes, None, None, None, &mut rng);
            }
            nodes
        }

        let a = run(&spawners, 42);
        let b = run(&spawners, 42);
        assert_eq!(a, b, "same seed must produce identical state");
    }
}
```

**Step 3: Verify**
Run: `cargo test --lib sim::terrain_spawn`
Expected: 8 tests pass (`probability_one_*`, `probability_zero_*`, `spawn_on_empty_cell_*`, `spawn_is_additive_*`, `spawn_never_overwrites_gem_cells`, `spawn_skips_gem_neighbors_and_picks_empty_cell`, `spawn_caps_at_max_remaining`, `deterministic_same_seed_*`).

**Step 4: Commit** — `sim: implement tick_terrain_spawners (single-phase, additive density-3)`

---

### Task 5: Map-load seeding helper

**Why:** Walk `terrain_objects` after parse, build the `terrain_spawners` map.

**Files:**
- Modify: `src/sim/terrain_spawn.rs` (add `seed_terrain_spawners` function)

**Pattern:** Mirror `seed_resource_nodes_from_overlays` ([src/sim/production/production_queue.rs:125](../../src/sim/production/production_queue.rs#L125)) — same shape, different input source.

**Step 1: Add seed function**

Append to `src/sim/terrain_spawn.rs` (above the `#[cfg(test)] mod tests` block):
```rust
use crate::map::overlay::TerrainObject;
use crate::rules::ruleset::RuleSet;
use crate::sim::world::Simulation;

/// Populate `production.terrain_spawners` from the map's terrain objects.
///
/// For each `TerrainObject` whose name matches a TerrainObjectType with
/// `spawns_tiberium = true && is_animated = true`, insert a spawner state
/// keyed by cell with `animation_probability_micros` cached from rules.
/// Returns the count seeded.
///
/// Also resolves `production.default_ore_overlay_id` from `overlay_names`
/// (first entry whose uppercase name starts with "TIB"). Used as the fallback
/// overlay_id when TIBTRE spawns ore on a previously empty cell.
///
/// The tick function does NOT consult rules — all per-spawner config is
/// baked here at seed time, mirroring the OreGrowthConfig pattern.
pub fn seed_terrain_spawners(
    sim: &mut Simulation,
    terrain_objects: &[TerrainObject],
    rules: &RuleSet,
    overlay_names: &BTreeMap<u8, String>,
) -> usize {
    // Resolve default_ore_overlay_id once.
    sim.production.default_ore_overlay_id = overlay_names
        .iter()
        .find(|(_id, name)| name.to_ascii_uppercase().starts_with("TIB"))
        .map(|(id, _)| *id);

    let mut seeded = 0usize;
    for obj in terrain_objects {
        let Some(t) = rules.terrain_object_type_case_insensitive(&obj.name) else {
            continue;
        };
        if !t.spawns_tiberium || !t.is_animated {
            continue;
        }
        let type_ref = sim.interner.intern(&obj.name);
        sim.production.terrain_spawners.insert(
            (obj.rx, obj.ry),
            TerrainSpawnerState {
                type_ref,
                animation_probability_micros: t.animation_probability_micros,
            },
        );
        seeded += 1;
    }
    seeded
}
```

**Step 2: Add test**

Append a test inside the existing `mod tests` block:
```rust
    #[test]
    fn seed_filters_to_spawning_animated_types_and_caches_probability() {
        use crate::map::overlay::TerrainObject;
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;

        let ini = IniFile::from_str(
            "[TerrainTypes]\n1=TIBTRE01\n2=TREE01\n\
             [TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\nAnimationProbability=.003\n\
             [TREE01]\nSpawnsTiberium=no\nIsAnimated=yes\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules");
        let mut sim = Simulation::new();
        let mut overlay_names = BTreeMap::new();
        overlay_names.insert(2u8, "TIB1".to_string());
        overlay_names.insert(7u8, "RUBBLE".to_string());

        let objs = vec![
            TerrainObject { rx: 5, ry: 6, name: "TIBTRE01".to_string() },
            TerrainObject { rx: 8, ry: 9, name: "TREE01".to_string() },
            TerrainObject { rx: 1, ry: 2, name: "UNKNOWN".to_string() },
        ];
        let seeded = seed_terrain_spawners(&mut sim, &objs, &rules, &overlay_names);
        assert_eq!(seeded, 1);
        let placed = sim
            .production
            .terrain_spawners
            .get(&(5, 6))
            .expect("TIBTRE01 seeded at (5,6)");
        // Cached probability matches the rules entry (0.003 * 1_000_000 = 3000).
        assert_eq!(placed.animation_probability_micros, 3000);
        assert_eq!(sim.production.default_ore_overlay_id, Some(2));
    }
```

**Step 3: Verify**
Run: `cargo test --lib sim::terrain_spawn::tests::seed`
Expected: pass.

**Step 4: Commit** — `sim: seed terrain_spawners from map terrain_objects`

---

### Task 6: Wire seeding into map-load

**Why:** Plug the new seed function into the existing app_init flow so spawners exist by the time the first tick runs.

**Files:**
- Modify: `src/app_init.rs:552-575` — extend the existing `if let Some(sim) = &mut simulation { ... }` block.

**Pattern:** Mirror the existing `seed_resource_nodes_from_overlays` call site.

**Step 1: Add the seed call**

Just after the existing block ([src/app_init.rs:553-557](../../src/app_init.rs#L553)):
```rust
        let seeded =
            production::seed_resource_nodes_from_overlays(sim, &map_data.overlays, &overlay_names);
        if seeded > 0 {
            log::info!("Seeded {} resource node cells for economy loop", seeded);
        }
```

Append (mirrors the existing `rules.as_ref().map_or(...)` pattern from line 578):
```rust
        // Seed TIBTRE-style ore-spawning terrain objects.
        // `rules` is Option<RuleSet> in this scope — skip seeding if rules
        // failed to load (matches the graceful-degradation pattern of the
        // ore_growth_config initialization a few lines below).
        if let Some(rules_for_terrain) = rules.as_ref() {
            let seeded_terrain = crate::sim::terrain_spawn::seed_terrain_spawners(
                sim,
                &map_data.terrain_objects,
                rules_for_terrain,
                &overlay_names,
            );
            if seeded_terrain > 0 {
                log::info!(
                    "Seeded {} ore-spawning terrain objects (TIBTRE)",
                    seeded_terrain,
                );
            }
        } else {
            log::warn!("No rules loaded — skipping terrain spawner seeding");
        }
```

**Step 2: Verify**
Run: `cargo build`
Expected: compiles.
Run: `cargo test --lib`
Expected: existing tests still pass.

**Step 3: Commit** — `app: seed terrain spawners after map load`

---

### Task 7: Hook tick into Phase 7

**Why:** Without this, the spawner list exists but never ticks.

**Files:**
- Modify: `src/sim/world/mod.rs:~1320` (line may have shifted; anchor by `ore_growth::tick_ore_growth`)

**Pattern:** Insert immediately after the existing `tick_ore_growth` block, before `if spawned_entities { refresh_fog }`.

**Step 1: Find the anchor**

Locate the block:
```rust
            ore_growth::tick_ore_growth(
                &self.production.ore_growth_config,
                &mut self.production.ore_growth_state,
                &mut self.production.resource_nodes,
                path_grid,
                self.overlay_grid.as_mut(),
                &mut self.rng,
            );
```

**Step 2: Insert tick call**

`tick_ore_growth` does NOT take `rules` — it operates entirely on baked config in `production`. `tick_terrain_spawners` is built the same way (probability cached into `TerrainSpawnerState` at seed time per Task 3), so it also doesn't need rules at the tick site.

Immediately after the closing `);` of `tick_ore_growth`:
```rust
            // TIBTRE ore spawning runs AFTER ore_growth so a spawn this tick
            // can't be grown/spread until next tick — matches gamemd.exe ordering.
            crate::sim::terrain_spawn::tick_terrain_spawners(
                &self.production.terrain_spawners,
                &mut self.production.resource_nodes,
                self.overlay_grid.as_mut(),
                self.production.default_ore_overlay_id,
                path_grid,
                &mut self.rng,
            );
```

**Step 3: Verify**
Run: `cargo build`
Expected: compiles.
Run: `cargo test --lib sim::world::world_tests`
Expected: existing world tests pass (no spawners in default sims → no change in behavior).

**Step 4: Commit** — `world: tick terrain spawners after ore_growth in Phase 7`

---

### Task 8: Extend determinism state hash

**Why:** Without this, two sims with the same seed but TIBTRE active will desync after the first spawn (the resource_node hash will diverge but the per-tick spawner-state inputs are NOT cross-checked, hiding the bug source). Required for replay/lockstep correctness.

**Files:**
- Modify: `src/sim/world/world_hash.rs:~159` (after `resource_nodes` block)

**Pattern:** Mirror the existing `resource_nodes` hash block (lines 154-159).

**Step 1: Extend hash**

Find the `resource_nodes` hash block:
```rust
        for (&(rx, ry), node) in &self.production.resource_nodes {
            rx.hash(hasher);
            ry.hash(hasher);
            (node.resource_type as u8).hash(hasher);
            node.remaining.hash(hasher);
        }
```

Insert immediately after:
```rust
        // Hash terrain spawners (TIBTRE-style ore generators).
        for (&(rx, ry), spawner) in &self.production.terrain_spawners {
            rx.hash(hasher);
            ry.hash(hasher);
            spawner.type_ref.hash(hasher);
            spawner.animation_probability_micros.hash(hasher);
        }
        self.production.default_ore_overlay_id.hash(hasher);
```

**Step 2: Add determinism test**

In `src/sim/world/world_tests.rs` (or wherever existing determinism tests live — search for "state_hash" usage):
```rust
    #[test]
    fn terrain_spawners_included_in_state_hash() {
        use crate::sim::terrain_spawn::TerrainSpawnerState;

        let mut sim_a = Simulation::new();
        let sim_b = Simulation::new();
        // sim_a has a TIBTRE; sim_b doesn't.
        let type_ref = sim_a.interner.intern("TIBTRE01");
        sim_a.production.terrain_spawners.insert(
            (10, 10),
            TerrainSpawnerState {
                type_ref,
                animation_probability_micros: 3000,
            },
        );

        assert_ne!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "terrain_spawners must affect state hash",
        );
    }
```
`state_hash()` confirmed at [src/sim/world/world_hash.rs:18](../../src/sim/world/world_hash.rs#L18) as `pub fn state_hash(&self) -> u64` on `Simulation`.

**Step 3: Verify**
Run: `cargo test --lib sim::world::world_tests::terrain_spawners_included_in_state_hash`
Expected: pass.

**Step 4: Commit** — `sim: include terrain_spawners in determinism state hash`

---

### Task 9: End-to-end integration test

**Why:** Verifies the full pipeline (rules parse → seed → tick → resource_nodes update → hash) works together. Catches integration bugs that unit tests miss.

**Files:**
- Modify: `src/sim/terrain_spawn.rs` (new test) OR new test file `src/sim/terrain_spawn_tests.rs`. Match the file split pattern of nearby modules (e.g. `miner_tests.rs`).

**Pattern:** Mirror miner integration tests in [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) for advance_tick driving.

**Step 1: Add integration test**

Append to `src/sim/terrain_spawn.rs` `mod tests`:
```rust
    #[test]
    fn full_pipeline_seeds_then_ticks_until_spawn() {
        use crate::map::overlay::TerrainObject;
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;

        let ini = IniFile::from_str(
            "[TerrainTypes]\n1=TIBTRE01\n\
             [TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\n\
             AnimationRate=3\nAnimationProbability=.5\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules");
        let mut sim = Simulation::new();
        // Seed.
        let objs = vec![TerrainObject { rx: 20, ry: 20, name: "TIBTRE01".into() }];
        let mut overlay_names = BTreeMap::new();
        overlay_names.insert(102u8, "TIB1".to_string());
        seed_terrain_spawners(&mut sim, &objs, &rules, &overlay_names);
        assert_eq!(sim.production.terrain_spawners.len(), 1);
        // Probability got baked into the spawner state (0.5 → 500_000).
        let placed = sim.production.terrain_spawners.get(&(20, 20)).unwrap();
        assert_eq!(placed.animation_probability_micros, 500_000);

        // Tick directly (bypass advance_tick to avoid setting up full world state).
        let mut rng = SimRng::new(99);
        let mut spawned = false;
        for _ in 0..50 {
            tick_terrain_spawners(
                &sim.production.terrain_spawners,
                &mut sim.production.resource_nodes,
                None,
                sim.production.default_ore_overlay_id,
                None,
                &mut rng,
            );
            if !sim.production.resource_nodes.is_empty() {
                spawned = true;
                break;
            }
        }
        assert!(spawned, "TIBTRE should spawn within 50 ticks at p=0.5");
    }
```

**Step 2: Verify**
Run: `cargo test --lib sim::terrain_spawn`
Expected: all tests pass (includes new integration test).

**Step 3: Final regression**
Run: `cargo test --lib`
Expected: full test suite passes.
Run: `cargo build --release`
Expected: clean build.

**Step 4: Commit** — `sim: integration test for TIBTRE seed-then-tick pipeline`

---

## Sources & References

- **Brainstorm:** in conversation; no separate design doc per user direction (system small enough to skip)
- **Ghidra report:** [TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md) (HIGH confidence, 2026-03-31)
- **Verification of non-TS-ghost status:** Ghidra MCP decompile of `TerrainClass::AI` (0x0071C730) — no SpecialFlags gate, single xref via TerrainClass vtable slot 23 (offset +0x5C from vtable base 0x007F522C)
- **gamemd.exe addresses:**
  - `TerrainClass::AI` = 0x0071C730
  - `CellClass::SpreadTiberium` = 0x00483780 (force=true bypasses TiberiumSpreads)
  - `CellClass::CanAcceptTiberium` = 0x004838E0
  - `TerrainTypeClass::ReadINI` = 0x0071DEA0
  - `1.0e-6` normalizer constant = 0x007EF918
- **TerrainTypeClass field offsets** (param_1 is `int*`, indices direct byte):
  - 0x2A0 AnimationRate (int)
  - 0x2A4 AnimationProbability (float)
  - 0x2B1 SpawnsTiberium (bool)
  - 0x2B3 IsAnimated (bool)
- **INI:** `ini/rulesmd.ini` lines 28109-28152 (`[TIBTRE01/02/03]` sections)
- **Repo patterns mirrored:**
  - `src/sim/ore_growth.rs:156` (tick function shape)
  - `src/sim/ore_growth.rs:296` (try_spread_ore — 8-direction iteration)
  - `src/sim/production/production_queue.rs:125` (seed-from-input shape)
  - `src/sim/production/production_types.rs:195-216` (ProductionState field placement)
  - `src/sim/world/world_hash.rs:154-159` (hash extension pattern)
  - `src/rules/object_type.rs:781-784` (INI field parsing)
  - `src/rules/ruleset.rs:object_case_insensitive` (case-insensitive lookup helper)
- **Related plan:** Gap-scan [docs/gap-scans/2026-05-05-gap-scan-miner.md](../gap-scans/2026-05-05-gap-scan-miner.md) Candidate #1
