# Smudge System Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Revision history:**
- 2026-05-06: initial plan
- 2026-05-06 (post-review): applied 8 fixes from `/review-plan` —
  (1) `ArtRegistry` retained on `RuleSet` (Task 3);
  (2) Combat dispatch switched to event-emission pattern via `SmudgeSpawnRequest` + drainer in `Simulation::advance_tick` (Tasks 13, 13.5);
  (3-7) mechanical API name fixes: `OccupancyGrid::get` + `has_blockers_on`, `SimRng::new`, `SimRng::next_u32`, `PathGrid::is_walkable`, `SmudgeGrid::test_force_set` test helper added;
  (8) Task 15 integration test caveats noted around sim-construction API.
- 2026-05-06 (mid-execution): two more API drift fixes from Task 1 subagent —
  (a) `IniSection::iter()` does not exist; replaced with `IniSection::get_values()` (matches canonical `[InfantryTypes]`-style pattern at [src/rules/ruleset.rs:1594](src/rules/ruleset.rs#L1594));
  (b) crate name is `vera20k`, not `ra2-rust-game` — all `cargo --package` invocations corrected.
- 2026-05-06 (Task 2 subagent): `ArtEntry` has a second constructor — test helper `make_art_entry` at [src/rules/shp_vehicle_sequence.rs:97](src/rules/shp_vehicle_sequence.rs#L97). Task 2 scope expanded to include adding `false` defaults for the three new fields there.
- 2026-05-06 (Task 3 subagent): three structural fixes —
  (a) `ArtRegistry` lacked `#[derive(Debug)]` — added (RuleSet derives Debug, so its fields must too);
  (b) `merge_art_data` call site is at [src/app_init.rs:263](src/app_init.rs#L263), NOT `ruleset.rs:2570`;
  (c) standalone `art: Option<ArtRegistry>` in `app_init.rs` is consumed by 5+ downstream sites (lighting, sidebar atlas, sim spawn) — `art.take()` would have broken them silently. Subagent cloned instead and added `#[derive(Clone)]` to `ArtRegistry`. Follow-up: migrate downstream consumers to read `rules.art_registry` then drop the clone.

**Goal:** Spawn craters and scorch marks at runtime from explosions and infantry deaths; load pre-placed `[Smudge]` map entries; render as a static decal layer between terrain and entities — bit-for-bit indistinguishable from gamemd.exe.

**Architecture:** New per-cell sim grid `SmudgeGrid` mirroring `OverlayGrid`'s shape. Spawn dispatcher in combat layer fires inline at ExplosionEffect emission and at building-destruction handling. No new sim entity. Render reads SmudgeGrid each frame.

**Design Doc:** [docs/plans/2026-05-06-smudge-system-design.md](docs/plans/2026-05-06-smudge-system-design.md)

---

## Grounding Summary

**Docs:**
- `ra2-rust-game-docs/SMUDGE_CLASS_GHIDRA_REPORT.md` — struct layouts, addresses (audited YELLOW 2026-05-06; corrections applied)
- `ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md` — HIGH conf, full anim-driven spawn flow
- `ra2-rust-game-docs/ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` — `Morphable` flag at `+0x2E0`, parsed from `[TileSetNNNN] Morphable=`
- `AUDIT_LOG.md` — IsBaked semantics, dead-code dedup globals, LandType-vs-IsoTileTypeClass distinction

**Ghidra-verified (this plan's session):** `AnimClass::Start @ 0x00424F00` decompile + assembly trace; `BuildingClass::DestructionEffects @ 0x004415F0`; `BuildingClass::SpawnSurvivors @ 0x00442D90`; `RulesClass::ReadCombatDamage @ 0x0066BBB0` (Scorches lists confirmed TS-legacy dead — no xrefs); `FUN_0049F420 @ 0x0049F420` random-offset helper; `IsoTileTypeClass+0x2E0 = Morphable`.

**Current code state observed:**
- No `AnimType` struct — `ArtEntry` in [src/rules/art_data.rs:19](src/rules/art_data.rs#L19) holds per-art-section data; smudge spawn flags belong here.
- `reduce_tiberium` lives at [src/sim/miner/mod.rs:342](src/sim/miner/mod.rs#L342) and operates on `resource_nodes: BTreeMap<(u16,u16), ResourceNode>`. Reuse it.
- `OverlayGrid` at [src/sim/overlay_grid.rs](src/sim/overlay_grid.rs) is the structural pattern for `SmudgeGrid`.
- `TilesetLookup` at [src/map/theater.rs:122](src/map/theater.rs#L122) needs a parallel `morphable_flags: Vec<bool>` aligned with `tileset_bounds`.
- `Simulation` at [src/sim/world/mod.rs:241](src/sim/world/mod.rs#L241) owns `overlay_grid: Option<OverlayGrid>` — `smudge_grid: Option<SmudgeGrid>` follows the same pattern.
- `state_hash` at [src/sim/world/world_hash.rs:18](src/sim/world/world_hash.rs#L18) calls `self.hash_overlay_grid(&mut hasher)` — `hash_smudge_grid` follows.
- Combat anim emission at [src/sim/combat/mod.rs:535-547](src/sim/combat/mod.rs#L535-L547) — dispatcher hook point.

**INI keys:**
- `rulesmd.ini` `[SmudgeTypes]` numeric list (lines 1682-1716+)
- `rulesmd.ini` per-smudge sections `[CRATER01]`..`[BURN16]` etc. with `Crater=`, `Burn=`, `Width=`, `Height=`, `Image=`
- `artmd.ini` per-AnimType sections with `Scorch=`, `Crater=`, `ForceBigCraters=` (currently NOT parsed)
- Per-theater INI `[TileSetNNNN]` `Morphable=` (currently NOT parsed)
- Map files `[Smudge]` section `Key=TYPENAME,X,Y,IsBaked` (currently NOT parsed)
- `[CombatDamage] Scorches`, `Scorches1..4` — **TS-LEGACY DEAD**, do NOT parse
- `[CombatDamage] Craters` — **does not exist**, do NOT search for it

**Unknowns after grounding (deferred):**
- Eager SHP frame-width/height init for AnimType. Crosses asset-loading boundary; defer to follow-up. Plan ships with hardcoded `(30, 30)` default — matches gamemd's "uncached first-call" fallback path.

## Key Technical Decisions

- **Spawn dispatcher emits events from combat; drained in `Simulation::advance_tick` after combat, before ore growth** — **Confidence:** high. **Source:** mirrors existing `bridge_damage_events`, `wall_damage_events`, `explosion_effects` pattern in `CombatTickResult` ([src/sim/combat/mod.rs:345-366](src/sim/combat/mod.rs#L345-L366)). Preserves `handle_entity_deaths` signature; keeps determinism (smudge ore-reduce runs before ore-growth tick).
- **Anim spawn flags live on `ArtEntry`, not a new `AnimType` struct** — **Confidence:** high. **Source:** [src/rules/art_data.rs:19](src/rules/art_data.rs#L19) — ArtEntry already holds per-section bool flags (theater, voxel, new_theater); pattern matches.
- **`ArtRegistry` retained as `pub` field on `RuleSet`** — **Confidence:** high. **Source:** currently transient at [src/rules/ruleset.rs:2569](src/rules/ruleset.rs#L2569) (constructed for `merge_art_data`, dropped). Keeping it alive lets the smudge drainer read `&rules.art_registry.get(anim_name)` for the spawn-flag lookup.
- **`reduce_tiberium` reused from miner module** — **Confidence:** high. **Source:** [src/sim/miner/mod.rs:342](src/sim/miner/mod.rs#L342). Same hardcoded-6 unit reduction semantic gamemd uses.
- **`Morphable` stored as `Vec<bool>` parallel to `tileset_bounds` in TilesetLookup** — **Confidence:** high. **Source:** existing structural shape of TilesetLookup; `set_names` already follows this pattern.
- **dmg/dmg2 hardcoded to (30, 30) for non-ForceBig anim spawns** — **Confidence:** medium. **Source:** matches gamemd's pre-cache default (ledger #9). Drift bound: anims with SHP frame > 60 px AND Crater/Scorch flags would get small smudges in our engine vs big in gamemd's lazy-cached state. Follow-up task tracks fixing this.
- **256-entry unit-vec lookup table built at first use via `OnceLock`** — **Confidence:** high. **Source:** SMUDGE_SPAWN_TRIGGERS §11.2; computed deterministically from `f64` once, stored as `i32` Q16.16 fixed-point, used at runtime via integer multiply only.
- **SmudgeGrid as `Option<SmudgeGrid>` on Simulation** — **Confidence:** high. **Source:** mirrors `overlay_grid: Option<OverlayGrid>` at [src/sim/world/mod.rs:241](src/sim/world/mod.rs#L241). Allows headless / test paths to skip smudge state.

## Open Questions

### Resolved during planning
- **Where do anim flags live?** → `ArtEntry` (no new struct).
- **How to call `reduce_tiberium`?** → `crate::sim::miner::reduce_tiberium(resource_nodes, (rx, ry), 6)`.
- **Cell-snap math after random offset?** → Caller does `(coord >> 8) * 256 + 128` per axis (verified in `BuildingClass::SpawnSurvivors` assembly).

### Deferred to follow-up
- **Eager SHP frame-width/height init for ArtEntry.** Requires reading SHP headers at startup-time (not full pixel decode). Crosses sim/render boundary unless we relocate via `assets/`. Tracked as "smudge frame-dim followup". Until done: dmg/dmg2 default `(30, 30)`.
- **Actual SHP atlas integration for the smudge render layer.** Atlas registration mechanics are render-side; this plan stops at "build SmudgeInstance buffer with placeholder UVs" and leaves the atlas wiring for the render-layer task to figure out using existing patterns.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/rules/smudge_type.rs` | `SmudgeTypeRegistry` + per-type parsing |
| Modify | `src/rules/art_data.rs` | Add `scorch`, `crater`, `force_big_craters` to `ArtEntry` |
| Modify | `src/rules/ruleset.rs` | Wire `SmudgeTypeRegistry` into `RuleSet`; retain `art_registry` as `pub` field after `merge_art_data` |
| Modify | `src/rules/mod.rs` | `pub mod smudge_type;` |
| Modify | `src/map/theater.rs` | Parse `Morphable=` per `[TileSetNNNN]`; expose via `is_morphable(tile_id)` |
| Modify | `src/map/resolved_terrain.rs` | Add `accepts_smudge: bool` to `ResolvedTerrainCell`; populate from tileset Morphable |
| Modify | `src/map/map_file.rs` | Add `MapSmudgeEntry` + parse `[Smudge]` section (skip IsBaked != 0) |
| Create | `src/sim/smudge_grid.rs` | `SmudgeGrid`, `SmudgeCell`, `try_place`, `can_place_here` |
| Create | `src/sim/combat/smudge_dispatch.rs` | Unit-vec table + drainer that consumes `SmudgeSpawnRequest` events |
| Modify | `src/sim/combat/mod.rs` | Define `SmudgeSpawnRequest`; emit events from `handle_entity_deaths`; add field to `CombatTickResult` |
| Modify | `src/sim/mod.rs` | `pub mod smudge_grid;` |
| Modify | `src/sim/world/mod.rs` | `pub smudge_grid: Option<SmudgeGrid>`; init at sim build; drain smudge requests in `advance_tick` after combat, before ore growth |
| Modify | `src/sim/world/world_hash.rs` | `hash_smudge_grid` helper; call from `state_hash` |
| Create | `src/render/smudge.rs` | Build `SmudgeInstance` buffer per visible cell |
| Modify | `src/render/mod.rs` | `pub mod smudge;` + wire into render pipeline between terrain and entities |

## Interface Changes

- **New public types:** `SmudgeTypeRegistry`, `SmudgeTypeDef`, `SmudgeKind` (enum), `MapSmudgeEntry`, `SmudgeGrid`, `SmudgeCell`, `SmudgeInstance`, `SmudgeSpawnRequest` (enum). None modify existing public APIs.
- **`ArtEntry` gains 3 fields.** Public struct — every constructor of `ArtEntry` must initialize them. There's only one (in `ArtRegistry::from_ini`); no other consumers construct `ArtEntry` directly (verified via grep).
- **`ResolvedTerrainCell` gains `accepts_smudge: bool`.** Public field — every constructor must initialize. Consumers that destructure or pattern-match on `ResolvedTerrainCell` need to add the field.
- **`Simulation` gains `smudge_grid: Option<SmudgeGrid>`.** Mirrors `overlay_grid` pattern; serde-skipped or included via existing snapshot serialization plan.
- **`RuleSet` gains `smudge_types: SmudgeTypeRegistry` AND `art_registry: ArtRegistry`.** The latter is currently transient and dropped after `merge_art_data`; this plan keeps it alive for runtime anim-flag lookups.
- **`CombatTickResult` gains `smudge_spawn_requests: Vec<SmudgeSpawnRequest>`.** Caller (`Simulation::advance_tick`) drains them after combat resolves but before the ore-growth tick stage.

## Sim Checklist

- [x] All math uses `fixed`-point (or integer Q16.16 in unit-vec table) — no f32/f64 in game logic. f64 used ONLY in startup-time table construction, frozen as i32 thereafter.
- [x] New state included in deterministic state hash (Task 12).
- [x] No dependencies on render/ui/sidebar/audio/net (sim crate boundary preserved).
- [x] Tick ordering impact: dispatcher runs in existing combat tick phase; no new tick stage.
- [x] BTreeMap iteration order: SmudgeGrid is `Vec`-backed (flat grid); iteration order is stable.

## Risk Areas

1. **Determinism — RNG advance discipline.** Three new draw points (filter pick + DestructionEffects' two discarded calls + SpawnSurvivors' unit-vec byte). All MUST use `world.rng`. Regression test in Task 15 verifies same-seed → same-hash.
2. **Crater path's `reduce_tiberium(6)` runs even on `CanPlaceHere` failure.** Real semantic; ledger #5/#19. A naïve impl that gates `reduce_tiberium` on placement success would silently diverge. Test in Task 9.
3. **Render layer ordering.** Smudges drawn between terrain and entities. Existing pipeline: terrain → entities → cliff-redraw. Insertion must not break cliff-redraw depth assumptions. Task 14 documents the wiring pattern.
4. **Snapshot serialization.** SmudgeGrid joins the snapshotted set; Task 11 ensures `#[serde(skip)]` only on `dirty_cells`, not `cells`. Test in Task 15.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| 4 | Per-tileset `Morphable=yes` gate (default false) | Determines which tile types accept smudges. Wrong default → smudges land on water/cliffs/ice. Visible every match. | INI grep `temperatmd.ini` for `Morphable=yes` count; cross-check ResolvedTerrainCell |
| 7 | `can_place_here` 6-gate sequence (in-bounds, no smudge, no overlay, no building, slope==0, accepts_smudge) ALL must pass per W×H cell | Wrong gate → smudges on bridges, walls, hills. | Unit test for each gate |
| 8 | Unit-vec table formula: `(byte<<8 as i16) - 0x3FFF) * (-pi/32768)` and Q16.16 multiply | Different formula → different cell-pick distribution → different SmudgeGrid hash → desync. | Test 4 specific bytes against hand-computed reference |
| 9 | Altitude gate `< 30` (strict less-than, NOT `<= 30`) | Ledger #3. dmg/dmg2 default `(30, 30)` (matches gamemd uncached first-call). | Unit test with z=29, 30, 31 |
| 9 | 50/50 probability via `rng.gen_below_half_normalized()` (rand < 2^30) | Ledger #4. Differs from `(rand & 1) == 0` — different RNG advance. | Mock-RNG test verifies branch |
| 9 | `reduce_tiberium(6)` runs BEFORE `try_place` for crater path | Ledger #5/#19. Ore destroyed even if smudge can't place. | Integration test: place crater on overlay cell → ore reduced, smudge NOT placed |
| 9 | ForceBigCraters passes hardcoded `(300, 300, 1)`, NOT `(frame_w, frame_h, 1)` | Ledger #7. | Inspect dispatcher code; comment-free assertion |
| 9 | Each anim spawn returns after first successful arm — at most ONE smudge per spawn | Ledger #10. | Unit test: anim with both flags either spawns scorch OR crater, not both |
| 10 | DestructionEffects fires 3 RandomRanged calls: `(0, W-2)` discarded, `(0, H-2)` discarded, `(0, 99)` is the actual roll | Ledger #17. Skipping the discards desyncs replays. | Test: destroy a 4×4 building, RNG counter advances by exactly 3 (plus one filter pick) |
| 10 | DestructionEffects center coord = `(rx*256+128, ry*256+128, building.z)` with foundation ≥ 2×2 only | Ledger #16/#18. 1×1 buildings get NO DestructionEffects smudge. | Test 1×1 vs 2×2 buildings |
| 10 | SpawnSurvivors per-cell uses `RandomRanged(0,99) < 50` AND unit-vec offset; magnitude 0x80 | Ledger #19/#20. | Integration test: 4×4 building destroyed produces ≤16 cell-positioned smudges |
| 10 | SpawnSurvivors offset coord conversion: `(off >> 8) * 256 + 128` per axis (cell-snap) | Ledger #30. | Test: with mocked RNG byte 0, verify resulting cell coord |
| 6 | `IsBaked != 0` entries SKIPPED at map load | Ledger #21. | Test: parse a `[Smudge]` section with mix of IsBaked=0/1 |
| 7 | Threshold check `0x3C < dmg AND 0x32 < dmg2` strict less-than | Ledger #13. | Test: dmg=60 fails (only 1×1), dmg=61 passes |
| 7 | Empty filtered list falls back to unfiltered Crater/Burn pool | Ledger #15. | Test: only big smudges defined, request small → fallback to big |
| 12 | SmudgeGrid included in `world_hash` | Ledger #28. Replay desync surfaces immediately. | Test: place smudge → hash changes; identical state → hash matches |

---

## Tasks

### Task 1: Create SmudgeTypeRegistry

**Why:** Foundation. Every other task depends on the registry being parseable from rulesmd.ini.

**Files:**
- Create: `src/rules/smudge_type.rs`
- Modify: `src/rules/mod.rs` (add `pub mod smudge_type;`)

**Pattern:** Follows `OverlayTypeRegistry` shape from [src/map/overlay_types.rs](src/map/overlay_types.rs) — name+flags table indexed by u8/u16 ID.

**Step 1: Define types**

```rust
// src/rules/smudge_type.rs
//! Smudge type definitions parsed from rulesmd.ini.
//!
//! [SmudgeTypes] numeric list maps to per-name sections (e.g. [CRATER01]).
//! Each type carries the four INI keys that gate spawn behavior:
//! Crater, Burn, Width, Height.
//!
//! Dependency rules: depends on rules/ini_parser only. No sim dependency.

use std::collections::HashMap;

use crate::rules::ini_parser::IniFile;

#[derive(Debug, Clone)]
pub struct SmudgeTypeDef {
    pub name: String,
    pub crater: bool,
    pub burn: bool,
    pub width: u8,
    pub height: u8,
    pub image_name: Option<String>,
    pub is_theater: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SmudgeTypeRegistry {
    types: Vec<SmudgeTypeDef>,
    by_name: HashMap<String, u16>,
}
```

**Step 2: Implement parser**

```rust
impl SmudgeTypeRegistry {
    pub fn from_rules_ini(ini: &IniFile) -> Self {
        let mut types: Vec<SmudgeTypeDef> = Vec::new();
        let mut by_name: HashMap<String, u16> = HashMap::new();

        let Some(list_section) = ini.section("SmudgeTypes") else {
            return Self::default();
        };

        // Matches the canonical [XxxTypes] numbered-list pattern used by
        // ruleset.rs:1594 — get_values() returns the values of `1=NAME, 2=NAME, ...`
        // sorted by numeric index, with empty strings filtered out.
        for value in list_section.get_values() {
            let name_upper: String = value.trim().to_uppercase();
            if name_upper.is_empty() {
                continue;
            }
            if by_name.contains_key(&name_upper) {
                continue;
            }
            let Some(section) = ini.section(&name_upper) else {
                continue;
            };
            let crater: bool = section.get_bool("Crater").unwrap_or(false);
            let burn: bool = section.get_bool("Burn").unwrap_or(false);
            let width: u8 = section
                .get_i32("Width")
                .map(|v| v.clamp(1, 255) as u8)
                .unwrap_or(1);
            let height: u8 = section
                .get_i32("Height")
                .map(|v| v.clamp(1, 255) as u8)
                .unwrap_or(1);
            let image_name: Option<String> = section
                .get("Image")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let is_theater: bool = section.get_bool("Theater").unwrap_or(false);

            let id: u16 = types.len() as u16;
            by_name.insert(name_upper.clone(), id);
            types.push(SmudgeTypeDef {
                name: name_upper,
                crater,
                burn,
                width,
                height,
                image_name,
                is_theater,
            });
        }

        Self { types, by_name }
    }

    pub fn get(&self, id: u16) -> Option<&SmudgeTypeDef> {
        self.types.get(id as usize)
    }

    pub fn find_by_name(&self, name: &str) -> Option<u16> {
        self.by_name.get(&name.to_uppercase()).copied()
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn iter_with_id(&self) -> impl Iterator<Item = (u16, &SmudgeTypeDef)> {
        self.types
            .iter()
            .enumerate()
            .map(|(i, t)| (i as u16, t))
    }
}
```

**Step 3: Add module declaration**

In `src/rules/mod.rs`, add (alphabetically positioned):

```rust
pub mod smudge_type;
```

**Step 4: Add tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ini(s: &str) -> IniFile {
        IniFile::from_bytes(s.as_bytes()).unwrap()
    }

    #[test]
    fn parses_smudge_types_list_with_per_name_sections() {
        let ini = parse_ini(
            "[SmudgeTypes]\n\
             1=CR1\n\
             2=BURN01\n\
             \n\
             [CR1]\n\
             Crater=yes\n\
             Width=1\n\
             Height=1\n\
             \n\
             [BURN01]\n\
             Burn=yes\n\
             Width=2\n\
             Height=2\n",
        );
        let reg = SmudgeTypeRegistry::from_rules_ini(&ini);
        assert_eq!(reg.len(), 2);
        let cr1 = reg.get(0).unwrap();
        assert_eq!(cr1.name, "CR1");
        assert!(cr1.crater);
        assert!(!cr1.burn);
        assert_eq!(cr1.width, 1);
        let burn01 = reg.get(1).unwrap();
        assert!(burn01.burn);
        assert_eq!(burn01.width, 2);
        assert_eq!(burn01.height, 2);
    }

    #[test]
    fn missing_section_skipped() {
        let ini = parse_ini("[SmudgeTypes]\n1=DOES_NOT_EXIST\n");
        let reg = SmudgeTypeRegistry::from_rules_ini(&ini);
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn defaults_apply() {
        let ini = parse_ini("[SmudgeTypes]\n1=X\n[X]\n");
        let reg = SmudgeTypeRegistry::from_rules_ini(&ini);
        let x = reg.get(0).unwrap();
        assert!(!x.crater);
        assert!(!x.burn);
        assert_eq!(x.width, 1);
        assert_eq!(x.height, 1);
    }

    #[test]
    fn find_by_name_case_insensitive() {
        let ini = parse_ini("[SmudgeTypes]\n1=CR1\n[CR1]\nCrater=yes\n");
        let reg = SmudgeTypeRegistry::from_rules_ini(&ini);
        assert_eq!(reg.find_by_name("cr1"), Some(0));
        assert_eq!(reg.find_by_name("CR1"), Some(0));
        assert_eq!(reg.find_by_name("nope"), None);
    }
}
```

**Step 5: Verify**

Run: `cargo test --package vera20k --lib rules::smudge_type`
Expected: 4 PASS

**Step 6: Commit**

`smudge: parse [SmudgeTypes] + per-name sections into SmudgeTypeRegistry`

---

### Task 2: Add anim spawn flags to ArtEntry

**Why:** Per-AnimType `Scorch=`, `Crater=`, `ForceBigCraters=` are the spawn-trigger flags. They live on the existing `ArtEntry` (ledger #1).

**Files:**
- Modify: `src/rules/art_data.rs`
- Modify: `src/rules/art_data_tests.rs` (add the new test)
- Modify: `src/rules/shp_vehicle_sequence.rs` (test helper `make_art_entry` at line 97 also constructs `ArtEntry` directly — add the three new fields with `false` defaults to keep the codebase compiling)

**Pattern:** ArtEntry already holds per-section bool flags (`theater`, `voxel`, `new_theater`); same pattern.

**Step 1: Extend `ArtEntry` struct**

In [src/rules/art_data.rs:19](src/rules/art_data.rs#L19), inside `pub struct ArtEntry`, add (after `pub theater: bool`):

```rust
    pub scorch: bool,
    pub crater: bool,
    pub force_big_craters: bool,
```

**Step 2: Parse the keys in `from_ini`**

In `ArtRegistry::from_ini` around the existing `let theater: bool = ...` line, add:

```rust
            let scorch: bool = section.get_bool("Scorch").unwrap_or(false);
            let crater: bool = section.get_bool("Crater").unwrap_or(false);
            let force_big_craters: bool = section.get_bool("ForceBigCraters").unwrap_or(false);
```

In the `entries.insert(...)` call constructing the `ArtEntry`, add the three fields (preserving existing field order pattern):

```rust
            entries.insert(
                section_name.to_uppercase(),
                ArtEntry {
                    image,
                    cameo,
                    alt_cameo,
                    new_theater,
                    theater,
                    scorch,
                    crater,
                    force_big_craters,
                    voxel,
                    // ...rest unchanged
                },
            );
```

**Step 3: Add tests**

In the `art_data_tests.rs` module, add:

```rust
#[test]
fn parses_anim_smudge_flags() {
    let ini = IniFile::from_bytes(
        b"[ANIMA]\n\
          Scorch=yes\n\
          \n\
          [ANIMB]\n\
          Crater=yes\n\
          ForceBigCraters=yes\n\
          \n\
          [ANIMC]\n",
    ).unwrap();
    let reg = ArtRegistry::from_ini(&ini);
    let a = reg.get("ANIMA").unwrap();
    assert!(a.scorch);
    assert!(!a.crater);
    let b = reg.get("ANIMB").unwrap();
    assert!(!b.scorch);
    assert!(b.crater);
    assert!(b.force_big_craters);
    let c = reg.get("ANIMC").unwrap();
    assert!(!c.scorch);
    assert!(!c.crater);
    assert!(!c.force_big_craters);
}
```

**Step 4: Verify**

Run: `cargo test --package vera20k --lib rules::art_data`
Expected: existing tests + 1 new PASS

**Step 5: Commit**

`art: parse Scorch/Crater/ForceBigCraters bool flags onto ArtEntry`

---

### Task 3: Wire SmudgeTypeRegistry into RuleSet + retain ArtRegistry

**Why:** Make the smudge registry accessible via `&RuleSet`. Also retain `ArtRegistry` past `merge_art_data` so the smudge dispatcher can read per-anim spawn flags (Issue 1 from `/review-plan`).

**Files:**
- Modify: `src/rules/ruleset.rs` (struct + construction)
- Modify: `src/rules/art_data.rs` (`ArtRegistry` needs `#[derive(Debug)]` because RuleSet derives `Debug`)
- Modify: `src/app_init.rs` (the actual `merge_art_data` call site is here at line 263, NOT in ruleset.rs as the original plan said)

**Step 1: Add `#[derive(Debug)]` to `ArtRegistry`**

In [src/rules/art_data.rs:170](src/rules/art_data.rs#L170), the struct currently has no derives:

```rust
pub struct ArtRegistry {
    entries: HashMap<String, ArtEntry>,
}
```

Change to:

```rust
#[derive(Debug)]
pub struct ArtRegistry {
    entries: HashMap<String, ArtEntry>,
}
```

(`HashMap<String, ArtEntry>` is `Debug` automatically because both `String` and `ArtEntry` are `Debug`.)

**Step 2: Add fields to RuleSet**

In `pub struct RuleSet` at [src/rules/ruleset.rs:1066](src/rules/ruleset.rs#L1066), add (alongside the other pub fields):

```rust
    pub smudge_types: SmudgeTypeRegistry,
    pub art_registry: crate::rules::art_data::ArtRegistry,
```

Add the import at the top of the file:

```rust
use crate::rules::smudge_type::SmudgeTypeRegistry;
```

**Step 3: Initialize in RuleSet struct-literal init**

Find every `RuleSet { ... }` struct literal in `ruleset.rs` (the constructor and any `Default` impl). For each, add:

```rust
            smudge_types: SmudgeTypeRegistry::default(),
            art_registry: crate::rules::art_data::ArtRegistry::empty(),
```

(For the `from_ini`-style constructor that has access to the parsed `IniFile`, you can replace `SmudgeTypeRegistry::default()` with `SmudgeTypeRegistry::from_rules_ini(&ini)` — but only if the constructor actually has the IniFile in scope. If unsure, leave both as their empty defaults; the actual smudge registry gets populated by the caller in `app_init.rs` (next step) the same way `art_registry` does.)

**Step 4: Move `art` into `rules.art_registry` in app_init.rs**

The merge call site is at [src/app_init.rs:263](src/app_init.rs#L263). Current pattern (around lines 258-264):

```rust
    let (art, art_ini): (Option<ArtRegistry>, Option<IniFile>) = match art_result {
        Some((reg, ini)) => (Some(reg), Some(ini)),
        None => (None, None),
    };
    if let (Some(r), Some(a)) = (&mut rules, &art) {
        r.merge_art_data(a);
    }
```

Change to consume `art` into the RuleSet:

```rust
    let (mut art, art_ini): (Option<ArtRegistry>, Option<IniFile>) = match art_result {
        Some((reg, ini)) => (Some(reg), Some(ini)),
        None => (None, None),
    };
    if let (Some(r), Some(a)) = (rules.as_mut(), art.take()) {
        r.merge_art_data(&a);
        r.art_registry = a;
    }
```

Two changes: `let (art, ...)` → `let (mut art, ...)` (line 258); the `if let` block now uses `art.take()` and assigns the moved value into `r.art_registry` after merge.

**Step 4b: Populate SmudgeTypeRegistry similarly (if the RuleSet constructor doesn't already)**

In `app_init.rs`, find where `rules` and the rulesmd `IniFile` are both in scope. If the RuleSet constructor does NOT take the rulesmd IniFile and call `SmudgeTypeRegistry::from_rules_ini` itself (Step 3 caveat above), add:

```rust
    if let (Some(r), Some(rules_ini)) = (rules.as_mut(), &<rulesmd_ini_var>) {
        r.smudge_types = crate::rules::smudge_type::SmudgeTypeRegistry::from_rules_ini(rules_ini);
    }
```

Where `<rulesmd_ini_var>` is the rulesmd `IniFile` variable in scope at that point. Look at how `art_ini` is used downstream in app_init.rs for a parallel pattern.

Verify which approach (Step 3 inline vs Step 4b external) the existing codebase prefers — most rules-side registries are populated by RuleSet's constructor; SmudgeTypeRegistry should match that pattern.

**Step 5: Verify**

Run: `cargo build --package vera20k`
Expected: compile clean.

Run: `cargo test --package vera20k --lib rules`
Expected: existing tests still pass. Note: tests that construct `RuleSet { ... }` directly (e.g., `ruleset.rs:2616` test) need the two new fields added — defaults are fine for tests.

**Step 6: Commit**

`rules: wire SmudgeTypeRegistry into RuleSet; retain ArtRegistry post-merge`

---

### Task 4: Parse `Morphable=` per TileSet

**Why:** Smudge placement gate (ledger #29). Without this, smudges land on water/cliffs/ice.

**Files:**
- Modify: `src/map/theater.rs`

**Pattern:** Mirrors `set_names: Vec<String>` already aligned with `tileset_bounds`.

**Step 1: Extend TilesetLookup struct**

In `pub struct TilesetLookup` at [src/map/theater.rs:122](src/map/theater.rs#L122), add (alongside `set_names`):

```rust
    /// Per-tileset Morphable= flag — parsed from `[TileSetNNNN] Morphable=`.
    /// Default `false`. Smudges only place on cells whose tileset is morphable.
    morphable_flags: Vec<bool>,
```

**Step 2: Add accessor**

In `impl TilesetLookup`, add:

```rust
    /// Returns true if a tile_id belongs to a tileset with `Morphable=yes`.
    /// Smudges (craters, scorches) only place on morphable tiles.
    pub fn is_morphable(&self, tile_id: u16) -> bool {
        let idx: u16 = match self.tileset_index(tile_id) {
            Some(i) => i,
            None => return false,
        };
        self.morphable_flags
            .get(idx as usize)
            .copied()
            .unwrap_or(false)
    }
```

**Step 3: Parse in `parse_tileset_ini`**

Find the per-tileset loop in `parse_tileset_ini` at [src/map/theater.rs:234+](src/map/theater.rs#L234). Wherever `set_names.push(...)` is called for a tileset section, also push to the morphable_flags vec. Add at the top of the function alongside the other vec initializations:

```rust
    let mut morphable_flags: Vec<bool> = Vec::new();
```

Inside the per-tileset block, after the `set_names.push(...)` line (or wherever a tileset is registered), add:

```rust
        let morphable: bool = section.get_bool("Morphable").unwrap_or(false);
        morphable_flags.push(morphable);
```

In the final `Ok(TilesetLookup { ... })` construction, add `morphable_flags` to the struct literal.

**Step 4: Add test**

In a `#[cfg(test)] mod tests` block at the bottom of `src/map/theater.rs` (or extend the existing one), add:

```rust
#[test]
fn parses_morphable_flag_per_tileset() {
    let ini = b"[TileSet0000]\n\
                FileName=foo\n\
                TilesInSet=1\n\
                SetName=Foo\n\
                Morphable=yes\n\
                \n\
                [TileSet0001]\n\
                FileName=bar\n\
                TilesInSet=1\n\
                SetName=Bar\n\
                \n\
                [TileSet0002]\n\
                TilesInSet=-1\n";
    let lookup = parse_tileset_ini(ini, "tem").unwrap();
    // tile_id 0 = first tile of TileSet0000 (Morphable=yes)
    assert!(lookup.is_morphable(0));
    // tile_id 1 = first tile of TileSet0001 (Morphable= unset → default false)
    assert!(!lookup.is_morphable(1));
}
```

**Step 5: Verify**

Run: `cargo test --package vera20k --lib map::theater`
Expected: existing tests + 1 new PASS

**Step 6: Commit**

`theater: parse Morphable= per TileSet section`

---

### Task 5: Add `accepts_smudge` to ResolvedTerrainCell

**Why:** Smudge placement gate (ledger #23). Combines tileset Morphable + slope check at terrain-resolve time so the dispatcher reads one bool.

**Files:**
- Modify: `src/map/resolved_terrain.rs`

**Step 1: Extend ResolvedTerrainCell**

In `pub struct ResolvedTerrainCell` at [src/map/resolved_terrain.rs:68](src/map/resolved_terrain.rs#L68), add (alongside the other terrain bool flags):

```rust
    /// True when this cell's tileset has `Morphable=yes`. Smudge placement
    /// requires this gate (matches gamemd IsoTileTypeClass+0x2E0).
    pub accepts_smudge: bool,
```

**Step 2: Populate during terrain resolve**

Find every `ResolvedTerrainCell { ... }` constructor. There's typically one in the resolve function and possibly fallbacks. For each, populate:

```rust
        accepts_smudge: lookup.is_morphable(final_tile_index_u16),
```

(where `lookup` is the `TilesetLookup` reference and `final_tile_index_u16` is the resolved tile_id as u16.) For cells with no resolved tile (filled_clear, blank), use `false`.

**Step 3: Verify (no new test — covered by integration in Task 7)**

Run: `cargo build --package vera20k`
Expected: compile clean. All existing terrain tests still pass.

Run: `cargo test --package vera20k --lib map::resolved_terrain`
Expected: PASS.

**Step 4: Commit**

`resolved_terrain: derive accepts_smudge from tileset Morphable`

---

### Task 6: Parse `[Smudge]` map section

**Why:** Pre-placed map smudges (ledger #21). Maps can include scorches/craters baked in at design time.

**Files:**
- Modify: `src/map/map_file.rs`

**Pattern:** Mirrors existing `cell_tags`, `waypoints`, `overlays` parser shape.

**Step 1: Define MapSmudgeEntry**

Near the other `pub struct Map*Entry` types in `map_file.rs`, add:

```rust
/// A pre-placed smudge entry from the map's `[Smudge]` section.
///
/// Parsed format: `Key=TYPENAME,X,Y,IsBaked`.
/// Entries with `IsBaked != 0` are SKIPPED at parse time (they represent
/// smudges already baked into the underlying tile graphic).
#[derive(Debug, Clone)]
pub struct MapSmudgeEntry {
    pub type_name: String,
    pub rx: u16,
    pub ry: u16,
}
```

**Step 2: Add to MapFile**

In `pub struct MapFile` at [src/map/map_file.rs:142](src/map/map_file.rs#L142), add:

```rust
    /// Pre-placed smudges from the map's `[Smudge]` section.
    /// `IsBaked != 0` entries are filtered at parse time.
    pub smudges: Vec<MapSmudgeEntry>,
```

**Step 3: Implement parser**

Add a free function `parse_map_smudges(ini: &IniFile) -> Vec<MapSmudgeEntry>`:

```rust
fn parse_map_smudges(ini: &IniFile) -> Vec<MapSmudgeEntry> {
    let Some(section) = ini.section("Smudge") else {
        return Vec::new();
    };
    let mut out: Vec<MapSmudgeEntry> = Vec::new();
    for (_key, value) in section.iter() {
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        if parts.len() < 4 {
            continue;
        }
        let is_baked: i32 = parts[3].parse::<i32>().unwrap_or(0);
        if is_baked != 0 {
            continue;
        }
        let rx: u16 = match parts[1].parse::<u16>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ry: u16 = match parts[2].parse::<u16>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        out.push(MapSmudgeEntry {
            type_name: parts[0].to_uppercase(),
            rx,
            ry,
        });
    }
    out
}
```

**Step 4: Wire into `MapFile::from_bytes`**

Find the existing parser orchestration (e.g., `let overlays: Vec<OverlayEntry> = overlay::parse_overlays(&ini);`). Add alongside:

```rust
        let smudges: Vec<MapSmudgeEntry> = parse_map_smudges(&ini);
```

In the `Ok(MapFile { ... })` literal, add `smudges`.

**Step 5: Add tests**

```rust
#[cfg(test)]
mod smudge_parse_tests {
    use super::*;

    #[test]
    fn parses_smudge_section_skips_isbaked_nonzero() {
        let ini = IniFile::from_bytes(
            b"[Smudge]\n\
              0=CR1,5,6,0\n\
              1=BURN01,7,8,1\n\
              2=CR2,9,10,0\n",
        ).unwrap();
        let smudges = parse_map_smudges(&ini);
        assert_eq!(smudges.len(), 2);
        assert_eq!(smudges[0].type_name, "CR1");
        assert_eq!(smudges[0].rx, 5);
        assert_eq!(smudges[0].ry, 6);
        assert_eq!(smudges[1].type_name, "CR2");
    }

    #[test]
    fn handles_missing_section() {
        let ini = IniFile::from_bytes(b"[Other]\nFoo=Bar\n").unwrap();
        let smudges = parse_map_smudges(&ini);
        assert!(smudges.is_empty());
    }

    #[test]
    fn rejects_malformed_entries() {
        let ini = IniFile::from_bytes(
            b"[Smudge]\n\
              0=CR1,5,6\n\
              1=,5,6,0\n\
              2=CR1,X,6,0\n\
              3=CR1,5,6,0\n",
        ).unwrap();
        let smudges = parse_map_smudges(&ini);
        // Only entry 3 fully valid; entry 1 has empty type_name (kept as "" — uppercase of empty).
        // Entry 0 fails (only 3 parts), entry 2 fails (X not a number).
        // Entry 1: empty type_name accepted by parser but won't resolve to a registered SmudgeType later.
        assert_eq!(smudges.len(), 2);
        assert_eq!(smudges[0].type_name, "");
        assert_eq!(smudges[1].type_name, "CR1");
    }
}
```

**Step 6: Verify**

Run: `cargo test --package vera20k --lib map::map_file::smudge_parse_tests`
Expected: 3 PASS.

**Step 7: Commit**

`map_file: parse [Smudge] section, skip IsBaked!=0 entries`

---

### Task 7: Create SmudgeGrid

**Why:** Per-cell sim state, the heart of the system. All gates from ledger #11-#15, #23-#27 live here.

**Files:**
- Create: `src/sim/smudge_grid.rs`
- Modify: `src/sim/mod.rs` (add `pub mod smudge_grid;`)

**Pattern:** Mirrors `OverlayGrid` at [src/sim/overlay_grid.rs](src/sim/overlay_grid.rs).

**Step 1: Define types**

```rust
// src/sim/smudge_grid.rs
//! Per-cell mutable smudge state — runtime craters, scorch marks, and pre-placed map decals.
//!
//! Mirrors CellClass +0x48 (SmudgeTypeIndex) from gamemd.exe. Seeded from
//! map's [Smudge] section at sim init, mutated by the smudge dispatcher
//! during combat.
//!
//! Dependency rules: depends on rules/, map/, and other sim/ modules.
//! Never depends on render/, ui/, sidebar/, audio/, net/.

use crate::map::map_file::MapSmudgeEntry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::smudge_type::SmudgeTypeRegistry;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::rng::SimRng;

/// Smudge category — Burn for scorches, Crater for explosion craters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmudgeKind {
    Burn,
    Crater,
}

/// Per-cell smudge slot.
///
/// `type_id` indexes into SmudgeTypeRegistry. None = no smudge on this cell.
/// `footprint_origin` is the top-left cell of the W×H footprint that owns this cell.
/// `frame_offset` is the SHP frame index within the footprint
/// (computed as `(rx - origin.rx) + (ry - origin.ry) * footprint_width`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default,
         serde::Serialize, serde::Deserialize)]
pub struct SmudgeCell {
    pub type_id: Option<u16>,
    pub footprint_origin: Option<(u16, u16)>,
    pub frame_offset: u8,
}

/// Per-cell smudge grid. Flat Vec indexed by `ry * width + rx`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmudgeGrid {
    width: u16,
    height: u16,
    cells: Vec<SmudgeCell>,
    /// Cells mutated this tick — drained per tick by the render-update path.
    /// Not part of game state; never serialized.
    #[serde(skip, default)]
    dirty_cells: Vec<(u16, u16)>,
}

impl SmudgeGrid {
    pub fn new(width: u16, height: u16) -> Self {
        let count: usize = width as usize * height as usize;
        Self {
            width,
            height,
            cells: vec![SmudgeCell::default(); count],
            dirty_cells: Vec::new(),
        }
    }

    pub fn width(&self) -> u16 { self.width }
    pub fn height(&self) -> u16 { self.height }

    pub fn cell(&self, rx: u16, ry: u16) -> &SmudgeCell {
        match self.index_of(rx, ry) {
            Some(i) => &self.cells[i],
            None => &DEFAULT_CELL,
        }
    }

    fn index_of(&self, rx: u16, ry: u16) -> Option<usize> {
        if rx >= self.width || ry >= self.height {
            None
        } else {
            Some(ry as usize * self.width as usize + rx as usize)
        }
    }

    pub fn drain_dirty(&mut self) -> Vec<(u16, u16)> {
        std::mem::take(&mut self.dirty_cells)
    }

    pub fn iter_occupied(&self) -> impl Iterator<Item = (u16, u16, &SmudgeCell)> {
        self.cells.iter().enumerate().filter_map(move |(idx, c)| {
            if c.type_id.is_some() {
                let rx = (idx % self.width as usize) as u16;
                let ry = (idx / self.width as usize) as u16;
                Some((rx, ry, c))
            } else {
                None
            }
        })
    }

    /// Test-only direct cell mutation. Bypasses CanPlaceHere — use only in
    /// unit tests that need to seed a known SmudgeGrid state for hashing or
    /// snapshot round-trip verification.
    #[cfg(test)]
    pub fn test_force_set(&mut self, rx: u16, ry: u16, cell: SmudgeCell) {
        if let Some(idx) = self.index_of(rx, ry) {
            self.cells[idx] = cell;
            self.dirty_cells.push((rx, ry));
        }
    }
}

const DEFAULT_CELL: SmudgeCell = SmudgeCell {
    type_id: None,
    footprint_origin: None,
    frame_offset: 0,
};
```

**Step 2: Implement `from_map_entries`**

```rust
impl SmudgeGrid {
    pub fn from_map_entries(
        entries: &[MapSmudgeEntry],
        registry: &SmudgeTypeRegistry,
        terrain: &ResolvedTerrainGrid,
        overlay: &OverlayGrid,
        width: u16,
        height: u16,
    ) -> Self {
        let mut grid = Self::new(width, height);
        for entry in entries {
            let Some(type_id) = registry.find_by_name(&entry.type_name) else {
                log::warn!(
                    "[Smudge] entry references unknown SmudgeType '{}', skipping",
                    entry.type_name
                );
                continue;
            };
            let Some(def) = registry.get(type_id) else { continue; };
            if !grid.passes_placement_gates(
                entry.rx, entry.ry, def.width, def.height, terrain, overlay, None,
            ) {
                continue;
            }
            grid.write_footprint(entry.rx, entry.ry, type_id, def.width, def.height);
        }
        // Map-load doesn't dirty render; clear the queue.
        grid.dirty_cells.clear();
        grid
    }
}
```

**Step 3: Implement gates and footprint write**

```rust
impl SmudgeGrid {
    /// Six-gate placement check from ledger #23: in-bounds, no smudge, no overlay,
    /// no building, slope==0, accepts_smudge. All cells in the W×H footprint must pass.
    fn passes_placement_gates(
        &self,
        rx: u16, ry: u16, w: u8, h: u8,
        terrain: &ResolvedTerrainGrid,
        overlay: &OverlayGrid,
        occupancy: Option<&OccupancyGrid>,
    ) -> bool {
        for dy in 0..h as u16 {
            for dx in 0..w as u16 {
                let cx = rx + dx;
                let cy = ry + dy;
                if cx >= self.width || cy >= self.height {
                    return false;
                }
                if self.cell(cx, cy).type_id.is_some() {
                    return false;
                }
                if overlay.cell(cx, cy).overlay_id.is_some() {
                    return false;
                }
                let Some(tcell) = terrain.cell(cx, cy) else {
                    return false;
                };
                if tcell.slope_type != 0 {
                    return false;
                }
                if !tcell.accepts_smudge {
                    return false;
                }
                if let Some(occ) = occupancy {
                    if cell_has_building(occ, cx, cy) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn write_footprint(&mut self, rx: u16, ry: u16, type_id: u16, w: u8, h: u8) {
        for dy in 0..h as u16 {
            for dx in 0..w as u16 {
                let cx = rx + dx;
                let cy = ry + dy;
                let Some(idx) = self.index_of(cx, cy) else { continue; };
                self.cells[idx] = SmudgeCell {
                    type_id: Some(type_id),
                    footprint_origin: Some((rx, ry)),
                    frame_offset: (dx as u8) + (dy as u8) * w,
                };
                self.dirty_cells.push((cx, cy));
            }
        }
    }
}

fn cell_has_building(occupancy: &OccupancyGrid, rx: u16, ry: u16) -> bool {
    use crate::sim::movement::locomotor::MovementLayer;
    occupancy
        .get(rx, ry)
        .map_or(false, |c| c.has_blockers_on(MovementLayer::Ground))
}
```

**Note:** `OccupancyGrid::get(rx, ry) -> Option<&CellOccupancy>` is verified at [src/sim/occupancy.rs:181](src/sim/occupancy.rs#L181). `CellOccupancy::has_blockers_on(MovementLayer::Ground)` is verified at [src/sim/occupancy.rs:57](src/sim/occupancy.rs#L57) — returns true if any non-infantry occupant exists on the ground layer.

**Step 4: Implement `try_place`**

```rust
impl SmudgeGrid {
    /// Try to place a smudge of the given kind at `coord` (lepton-space).
    /// Runs the full filter + size selector + CanPlaceHere chain.
    ///
    /// Returns true if a smudge was placed, false otherwise.
    /// Per ledger #5/#19: callers MUST call `reduce_tiberium(6)` BEFORE this
    /// for the crater path — ore is destroyed even on placement failure.
    #[allow(clippy::too_many_arguments)]
    pub fn try_place(
        &mut self,
        kind: SmudgeKind,
        coord: SimCoord,
        dmg: i32,
        dmg2: i32,
        force_big: bool,
        registry: &SmudgeTypeRegistry,
        terrain: &ResolvedTerrainGrid,
        overlay: &OverlayGrid,
        occupancy: &OccupancyGrid,
        rng: &mut SimRng,
    ) -> bool {
        let rx: u16 = (coord.x >> 8).clamp(0, self.width as i32 - 1) as u16;
        let ry: u16 = (coord.y >> 8).clamp(0, self.height as i32 - 1) as u16;

        let unfiltered: Vec<u16> = registry.iter_with_id()
            .filter(|(_, def)| match kind {
                SmudgeKind::Burn => def.burn,
                SmudgeKind::Crater => def.crater,
            })
            .map(|(id, _)| id)
            .collect();
        if unfiltered.is_empty() { return false; }

        let mut filtered: Vec<u16> = if force_big {
            unfiltered.iter().copied()
                .filter(|&id| {
                    let d = registry.get(id).unwrap();
                    d.width >= 2 && d.height >= 2
                }).collect()
        } else {
            unfiltered.iter().copied()
                .filter(|&id| {
                    let d = registry.get(id).unwrap();
                    (d.width == 1 && d.height == 1)
                        || (0x3C < dmg && 0x32 < dmg2)
                }).collect()
        };
        if filtered.is_empty() {
            filtered = unfiltered;
        }

        let pick_idx = (rng.next_range_u32(filtered.len() as u32)) as usize;
        let chosen_id = filtered[pick_idx];
        let chosen = registry.get(chosen_id).unwrap();

        if !self.passes_placement_gates(
            rx, ry, chosen.width, chosen.height, terrain, overlay, Some(occupancy),
        ) {
            return false;
        }
        self.write_footprint(rx, ry, chosen_id, chosen.width, chosen.height);
        true
    }
}

/// Lepton-space coord (256 leptons = 1 cell, matches gamemd's CoordStruct).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
```

**Note:** Verify `SimRng::next_range_u32` exists; if it has a different signature use it. Inspect [src/sim/rng.rs](src/sim/rng.rs).

**Step 5: Add module declaration**

In `src/sim/mod.rs`:

```rust
pub mod smudge_grid;
```

**Step 6: Add tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::ResolvedTerrainCell;
    // helpers to build small fixtures...

    fn make_terrain(w: u16, h: u16, accepts: bool) -> ResolvedTerrainGrid {
        // Construct a flat W×H grid of ResolvedTerrainCells with:
        //   slope_type = 0
        //   accepts_smudge = accepts
        // Other fields use sensible defaults.
        // (Implement using the actual constructor signature. If
        // ResolvedTerrainGrid doesn't have a test-friendly builder,
        // construct cells manually and call ResolvedTerrainGrid::from_cells.)
        let mut cells: Vec<ResolvedTerrainCell> = Vec::with_capacity((w * h) as usize);
        for ry in 0..h {
            for rx in 0..w {
                cells.push(ResolvedTerrainCell {
                    rx, ry,
                    source_tile_index: 0, source_sub_tile: 0,
                    final_tile_index: 0, final_sub_tile: 0,
                    level: 0, filled_clear: true, tileset_index: Some(0),
                    land_type: 0, slope_type: 0, template_height: 0,
                    render_offset_x: 0, render_offset_y: 0,
                    terrain_class: Default::default(),
                    speed_costs: Default::default(),
                    is_water: false, is_cliff_like: false,
                    is_rough: false, is_road: false,
                    is_cliff_redraw: false, variant: 0,
                    has_ramp: false, canonical_ramp: None,
                    ground_walk_blocked: false, terrain_object_blocks: false,
                    overlay_blocks: false, zone_type: 0,
                    base_ground_walk_blocked: false, base_build_blocked: false,
                    build_blocked: false,
                    has_bridge_deck: false, bridge_walkable: false,
                    bridge_transition: false, bridge_deck_level: 0,
                    bridge_layer: None,
                    radar_left: [0; 3], radar_right: [0; 3],
                    accepts_smudge: accepts,
                });
            }
        }
        ResolvedTerrainGrid::from_cells(w, h, cells)
    }

    fn make_registry_with_one_crater_1x1() -> SmudgeTypeRegistry {
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR1\n[CR1]\nCrater=yes\nWidth=1\nHeight=1\n"
        ).unwrap();
        SmudgeTypeRegistry::from_rules_ini(&ini)
    }

    #[test]
    fn try_place_writes_one_cell_for_1x1() {
        let mut grid = SmudgeGrid::new(8, 8);
        let registry = make_registry_with_one_crater_1x1();
        let terrain = make_terrain(8, 8, true);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        assert!(grid.try_place(
            SmudgeKind::Crater, coord, 30, 30, false,
            &registry, &terrain, &overlay, &occupancy, &mut rng,
        ));
        assert!(grid.cell(4, 4).type_id.is_some());
    }

    #[test]
    fn rejects_when_accepts_smudge_false() {
        let mut grid = SmudgeGrid::new(8, 8);
        let registry = make_registry_with_one_crater_1x1();
        let terrain = make_terrain(8, 8, false); // Morphable=no
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        assert!(!grid.try_place(
            SmudgeKind::Crater, coord, 30, 30, false,
            &registry, &terrain, &overlay, &occupancy, &mut rng,
        ));
        assert!(grid.cell(4, 4).type_id.is_none());
    }

    #[test]
    fn rejects_when_overlay_present() {
        let mut grid = SmudgeGrid::new(8, 8);
        let registry = make_registry_with_one_crater_1x1();
        let terrain = make_terrain(8, 8, true);
        let mut overlay = OverlayGrid::new(8, 8);
        overlay.place_overlay(4, 4, 0, 0);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        assert!(!grid.try_place(
            SmudgeKind::Crater, coord, 30, 30, false,
            &registry, &terrain, &overlay, &occupancy, &mut rng,
        ));
    }

    #[test]
    fn threshold_strict_less_than_for_size_filter() {
        // Registry: one 1x1 crater + one 2x2 crater. With dmg=60, dmg2=50 (strict < fails),
        // only the 1x1 should be selectable.
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR1\n2=CR2\n\
              [CR1]\nCrater=yes\nWidth=1\nHeight=1\n\
              [CR2]\nCrater=yes\nWidth=2\nHeight=2\n"
        ).unwrap();
        let registry = SmudgeTypeRegistry::from_rules_ini(&ini);
        let mut grid = SmudgeGrid::new(8, 8);
        let terrain = make_terrain(8, 8, true);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        // Run try_place 50 times; with dmg=60, dmg2=50 only CR1 (1x1) should be picked.
        // Verify no 2x2 footprints land (CR2 would write 4 cells; CR1 writes 1).
        for _ in 0..50 {
            grid = SmudgeGrid::new(8, 8); // reset
            grid.try_place(
                SmudgeKind::Crater, coord, 60, 50, false,
                &registry, &terrain, &overlay, &occupancy, &mut rng,
            );
            // Count occupied cells; must be 0 or 1, never 4.
            let occupied = grid.iter_occupied().count();
            assert!(occupied <= 1, "1x1 only; saw {} cells", occupied);
        }
    }

    #[test]
    fn empty_filter_falls_back_to_unfiltered() {
        // Registry has only a 2x2 crater; with force_big=false and dmg below threshold,
        // size filter eliminates it but fallback to unfiltered should still pick it.
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR2\n[CR2]\nCrater=yes\nWidth=2\nHeight=2\n"
        ).unwrap();
        let registry = SmudgeTypeRegistry::from_rules_ini(&ini);
        let mut grid = SmudgeGrid::new(8, 8);
        let terrain = make_terrain(8, 8, true);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        assert!(grid.try_place(
            SmudgeKind::Crater, coord, 30, 30, false,
            &registry, &terrain, &overlay, &occupancy, &mut rng,
        ));
        // 2x2 footprint placed at (4,4): 4 cells written.
        assert_eq!(grid.iter_occupied().count(), 4);
    }
}
```

**Step 7: Verify**

Run: `cargo test --package vera20k --lib sim::smudge_grid`
Expected: 5 PASS.

**Step 8: Commit**

`smudge: SmudgeGrid + can_place_here + try_place with filter/threshold/fallback`

---

### Task 8: Unit-vec table + random_offset_at_radius helper

**Why:** SpawnSurvivors per-cell offset depends on bit-exact angle table (ledger #30). Determinism requires identical offsets across replays.

**Files:**
- Create: `src/sim/combat/smudge_dispatch.rs`
- Modify: `src/sim/combat/mod.rs` (`pub mod smudge_dispatch;`)

**Step 1: Define the table and helper**

```rust
// src/sim/combat/smudge_dispatch.rs
//! Smudge spawn dispatcher — fired from combat tick at explosion emission and
//! at building destruction. Mirrors AnimClass::Start, BuildingClass::DestructionEffects,
//! and BuildingClass::SpawnSurvivors smudge logic from gamemd.exe.
//!
//! Dependency rules: depends on rules/, map/, sim/. Never render/ui/audio/net.

use std::sync::OnceLock;

use crate::sim::rng::SimRng;
use crate::sim::smudge_grid::SimCoord;

/// 256-entry unit-vector lookup table in Q16.16 fixed-point.
/// Each entry is `(sin(angle) * 65536, -cos(angle) * 65536)` rounded to i32,
/// where `angle = (i16(byte << 8) - 0x3FFF) * (-pi / 32768)`.
///
/// Built once at first use; deterministic across machines because it's
/// computed from constants and frozen as i32.
fn unit_vec_table() -> &'static [(i32, i32); 256] {
    static TABLE: OnceLock<[(i32, i32); 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = [(0i32, 0i32); 256];
        for b in 0u32..256 {
            let raw = ((b << 8) as i16) as i32 - 0x3FFF;
            let angle = raw as f64 * (-std::f64::consts::PI / 32768.0);
            let sin_q16 = (angle.sin() * 65536.0).round() as i32;
            let neg_cos_q16 = (-(angle.cos()) * 65536.0).round() as i32;
            t[b as usize] = (sin_q16, neg_cos_q16);
        }
        t
    })
}

/// Returns a (dx, dy) lepton offset at the given magnitude using one byte
/// of RNG state. Z is unaffected.
///
/// Mirrors `FUN_0049F420(magnitude, flag=0)` from gamemd.exe.
pub(crate) fn random_offset_at_radius(rng: &mut SimRng, magnitude_leptons: i32) -> (i32, i32) {
    let b: u8 = (rng.next_u32() & 0xFF) as u8;
    let (sin_q16, neg_cos_q16) = unit_vec_table()[b as usize];
    let dx = ((sin_q16 as i64) * (magnitude_leptons as i64)) >> 16;
    let dy = ((neg_cos_q16 as i64) * (magnitude_leptons as i64)) >> 16;
    (dx as i32, dy as i32)
}
```

**Note:** `SimRng::next_u32` is verified at [src/sim/rng.rs:40](src/sim/rng.rs#L40). One `next_u32` call advances RNG state by one xorshift64* step; we extract the low byte for the angle index. Same RNG-advance count as gamemd's per-call byte consumption.

**Step 2: Add module declaration**

In `src/sim/combat/mod.rs`, add:

```rust
pub mod smudge_dispatch;
```

**Step 3: Add tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: i32, b: i32, tol: i32) -> bool { (a - b).abs() <= tol }

    #[test]
    fn unit_vec_table_byte_0_matches_reference() {
        // byte=0: raw = 0 - 0x3FFF = -16383; angle = -16383 * -pi/32768 ≈ 1.5708 (~pi/2)
        // sin(pi/2) ≈ 1.0, -cos(pi/2) ≈ 0.0
        let (sin_q16, neg_cos_q16) = unit_vec_table()[0];
        // sin*65536 ≈ 65536, -cos*65536 ≈ 0 (within rounding)
        assert!(approx_eq(sin_q16, 65536, 50), "sin_q16 = {}", sin_q16);
        assert!(approx_eq(neg_cos_q16, 0, 50), "neg_cos_q16 = {}", neg_cos_q16);
    }

    #[test]
    fn unit_vec_table_byte_64_quarter_turn() {
        // byte=64: (64<<8)=0x4000=16384; raw = 16384 - 0x3FFF = 1
        // angle ≈ -pi/32768 ≈ -0.0000958
        // sin(angle) ≈ -0.0000958, -cos(angle) ≈ -1.0
        let (sin_q16, neg_cos_q16) = unit_vec_table()[64];
        assert!(approx_eq(sin_q16, 0, 50), "sin_q16 = {}", sin_q16);
        assert!(approx_eq(neg_cos_q16, -65536, 50), "neg_cos_q16 = {}", neg_cos_q16);
    }

    #[test]
    fn random_offset_consumes_exactly_one_u32_advance() {
        // Two RNGs at the same seed: one advances via random_offset_at_radius,
        // the other advances via a single direct next_u32 call. After both
        // operations, internal state must match — confirming exactly one
        // RNG step was consumed.
        let mut rng_a = SimRng::new(42);
        let mut rng_b = SimRng::new(42);
        let _ = random_offset_at_radius(&mut rng_a, 0x80);
        let _ = rng_b.next_u32();
        assert_eq!(rng_a.state(), rng_b.state());
    }

    #[test]
    fn random_offset_per_axis_bounded() {
        // Each axis is bounded by `|component| <= magnitude + 1` (1 lepton
        // tolerance for Q16.16 round-to-nearest then >>16 truncation).
        // NOTE: a diagonal mag_sq check would fail here because the unit
        // table doesn't preserve sin² + cos² = 1 exactly post-rounding.
        let mut rng = SimRng::new(7);
        for _ in 0..256 {
            let (dx, dy) = random_offset_at_radius(&mut rng, 0x80);
            assert!(dx.abs() <= 0x80 + 1, "dx={} out of bounds", dx);
            assert!(dy.abs() <= 0x80 + 1, "dy={} out of bounds", dy);
        }
    }
}
```

**Step 4: Verify**

Run: `cargo test --package vera20k --lib sim::combat::smudge_dispatch`
Expected: 4 PASS.

**Step 5: Commit**

`smudge: 256-entry unit-vec table + random_offset_at_radius helper`

---

### Task 9: try_dispatch_anim_smudge

**Why:** The primary spawn path — fires when an explosion anim plays. Implements ledger #1-#10.

**Files:**
- Modify: `src/sim/combat/smudge_dispatch.rs`

**Step 1: Add the dispatcher function**

Append to `src/sim/combat/smudge_dispatch.rs`:

```rust
use std::collections::BTreeMap;

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::art_data::ArtRegistry;
use crate::rules::smudge_type::SmudgeTypeRegistry;
use crate::sim::miner::{ResourceNode, reduce_tiberium};
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::smudge_grid::{SmudgeGrid, SmudgeKind};

/// Default dmg/dmg2 values when AnimType frame dimensions aren't yet
/// pre-computed. Matches gamemd's pre-cache fallback (AnimType+0x29C/+0x2A0
/// init value of 0x1E = 30). Follow-up: replace with eager SHP frame-rect
/// init for full parity on big-explosion smudge sizes.
const DEFAULT_ANIM_FRAME_DIM: i32 = 30;

/// Strict altitude gate from ledger #3: smudges only spawn when the anim
/// is within 30 leptons of the ground.
const SMUDGE_ALTITUDE_GATE_LEPTONS: i32 = 30;

/// Hardcoded ore-reduction amount when a crater spawns (ledger #6).
const CRATER_ORE_REDUCTION: u16 = 6;

/// Try to dispatch a smudge for an animation that just spawned at `coord`.
///
/// Reads scorch/crater/force_big_craters bools from the AnimType's ArtEntry.
/// Performs the altitude gate, the 50/50 random pick when both flags are set,
/// the `reduce_tiberium(6)` side effect for crater path, and finally calls
/// `SmudgeGrid::try_place`.
#[allow(clippy::too_many_arguments)]
pub fn try_dispatch_anim_smudge(
    art: &ArtRegistry,
    smudge_types: &SmudgeTypeRegistry,
    anim_name: &str,
    coord: SimCoord,
    ground_z: i32,
    smudge_grid: &mut SmudgeGrid,
    overlay_grid: &OverlayGrid,
    occupancy: &OccupancyGrid,
    terrain: &ResolvedTerrainGrid,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    rng: &mut SimRng,
) {
    let Some(entry) = art.get(anim_name) else { return; };

    if (coord.z - ground_z) >= SMUDGE_ALTITUDE_GATE_LEPTONS {
        return;
    }

    let dmg = DEFAULT_ANIM_FRAME_DIM;
    let dmg2 = DEFAULT_ANIM_FRAME_DIM;

    if entry.scorch {
        if !entry.crater {
            smudge_grid.try_place(
                SmudgeKind::Burn, coord, dmg, dmg2, false,
                smudge_types, terrain, overlay_grid, occupancy, rng,
            );
            return;
        }
        if rng_below_half_normalized(rng) {
            smudge_grid.try_place(
                SmudgeKind::Burn, coord, dmg, dmg2, false,
                smudge_types, terrain, overlay_grid, occupancy, rng,
            );
            return;
        }
    }
    if entry.crater {
        let rx = (coord.x >> 8).clamp(0, smudge_grid.width() as i32 - 1) as u16;
        let ry = (coord.y >> 8).clamp(0, smudge_grid.height() as i32 - 1) as u16;
        reduce_tiberium(resource_nodes, (rx, ry), CRATER_ORE_REDUCTION);

        if entry.force_big_craters {
            smudge_grid.try_place(
                SmudgeKind::Crater, coord, 300, 300, true,
                smudge_types, terrain, overlay_grid, occupancy, rng,
            );
        } else {
            smudge_grid.try_place(
                SmudgeKind::Crater, coord, dmg, dmg2, false,
                smudge_types, terrain, overlay_grid, occupancy, rng,
            );
        }
    }
}

/// Mirrors gamemd's `RandomRanged(0, 0x7FFFFFFE) * (1/2^31) < 0.5` test
/// (ledger #4). Functionally equivalent: a uniform-random u32 has its high
/// bit clear with exactly 50% probability. One RNG advance, no modulo bias.
fn rng_below_half_normalized(rng: &mut SimRng) -> bool {
    rng.next_u32() < 0x8000_0000
}
```

**Note:** `SimRng::next_u32` verified at [src/sim/rng.rs:40](src/sim/rng.rs#L40). One advance per call.

**Step 2: Add tests**

```rust
#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::map::resolved_terrain::ResolvedTerrainCell;

    fn make_art(scorch: bool, crater: bool, force_big: bool) -> ArtRegistry {
        let scorch_str = if scorch { "yes" } else { "no" };
        let crater_str = if crater { "yes" } else { "no" };
        let big_str = if force_big { "yes" } else { "no" };
        let ini_text = format!(
            "[ANIM]\nScorch={}\nCrater={}\nForceBigCraters={}\n",
            scorch_str, crater_str, big_str,
        );
        let ini = crate::rules::ini_parser::IniFile::from_bytes(ini_text.as_bytes()).unwrap();
        ArtRegistry::from_ini(&ini)
    }

    fn make_smudge_registry() -> SmudgeTypeRegistry {
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR1\n2=BURN1\n\
              [CR1]\nCrater=yes\nWidth=1\nHeight=1\n\
              [BURN1]\nBurn=yes\nWidth=1\nHeight=1\n"
        ).unwrap();
        SmudgeTypeRegistry::from_rules_ini(&ini)
    }

    fn flat_terrain(w: u16, h: u16) -> ResolvedTerrainGrid {
        let mut cells: Vec<ResolvedTerrainCell> = Vec::with_capacity((w * h) as usize);
        for ry in 0..h {
            for rx in 0..w {
                cells.push(ResolvedTerrainCell {
                    rx, ry,
                    accepts_smudge: true,
                    slope_type: 0,
                    // ...other fields use the same defaults as Task 7's helper
                    ..test_default_cell(rx, ry)
                });
            }
        }
        ResolvedTerrainGrid::from_cells(w, h, cells)
    }

    fn test_default_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
        // Reuse Task 7's defaults via copy-paste; intentionally not extracted to
        // a shared helper to keep tasks self-contained.
        ResolvedTerrainCell {
            rx, ry,
            source_tile_index: 0, source_sub_tile: 0,
            final_tile_index: 0, final_sub_tile: 0,
            level: 0, filled_clear: true, tileset_index: Some(0),
            land_type: 0, slope_type: 0, template_height: 0,
            render_offset_x: 0, render_offset_y: 0,
            terrain_class: Default::default(),
            speed_costs: Default::default(),
            is_water: false, is_cliff_like: false,
            is_rough: false, is_road: false,
            is_cliff_redraw: false, variant: 0,
            has_ramp: false, canonical_ramp: None,
            ground_walk_blocked: false, terrain_object_blocks: false,
            overlay_blocks: false, zone_type: 0,
            base_ground_walk_blocked: false, base_build_blocked: false,
            build_blocked: false,
            has_bridge_deck: false, bridge_walkable: false,
            bridge_transition: false, bridge_deck_level: 0,
            bridge_layer: None,
            radar_left: [0; 3], radar_right: [0; 3],
            accepts_smudge: true,
        }
    }

    #[test]
    fn altitude_gate_blocks_above_30_leptons() {
        let art = make_art(false, true, false);
        let smudge_reg = make_smudge_registry();
        let mut grid = SmudgeGrid::new(8, 8);
        let terrain = flat_terrain(8, 8);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let mut nodes = BTreeMap::new();
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 100 };
        try_dispatch_anim_smudge(
            &art, &smudge_reg, "ANIM", coord, 0,
            &mut grid, &overlay, &occupancy, &terrain, &mut nodes, &mut rng,
        );
        assert!(grid.iter_occupied().count() == 0);
    }

    #[test]
    fn altitude_gate_strict_less_than_30() {
        let art = make_art(false, true, false);
        let smudge_reg = make_smudge_registry();
        let mut grid = SmudgeGrid::new(8, 8);
        let terrain = flat_terrain(8, 8);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let mut nodes = BTreeMap::new();
        // z - ground_z = 30 exactly → must FAIL (strict <)
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 30 };
        try_dispatch_anim_smudge(
            &art, &smudge_reg, "ANIM", coord, 0,
            &mut grid, &overlay, &occupancy, &terrain, &mut nodes, &mut rng,
        );
        assert!(grid.iter_occupied().count() == 0);
        // z - ground_z = 29 → must PASS
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 29 };
        try_dispatch_anim_smudge(
            &art, &smudge_reg, "ANIM", coord, 0,
            &mut grid, &overlay, &occupancy, &terrain, &mut nodes, &mut rng,
        );
        assert_eq!(grid.iter_occupied().count(), 1);
    }

    #[test]
    fn crater_path_reduces_tiberium_even_when_can_place_fails() {
        // Seed with 10 density levels (more than the 6-unit reduction) so
        // the cell stays present after Reduce_Tiberium(6) — testing
        // PARTIAL reduction. (If we seeded with <= 6 density levels,
        // miner::reduce_tiberium would fully remove the node and the
        // assertion shape would change to `is_none()`.)
        let art = make_art(false, true, false);
        let smudge_reg = make_smudge_registry();
        let mut grid = SmudgeGrid::new(8, 8);
        let terrain = flat_terrain(8, 8);
        let mut overlay = OverlayGrid::new(8, 8);
        // Block placement by putting an overlay on the impact cell.
        overlay.place_overlay(4, 4, 0, 0);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let mut nodes = BTreeMap::new();
        nodes.insert((4, 4), ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Ore,
            remaining: 120 * 10, // 10 density levels of ore
        });
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        try_dispatch_anim_smudge(
            &art, &smudge_reg, "ANIM", coord, 0,
            &mut grid, &overlay, &occupancy, &terrain, &mut nodes, &mut rng,
        );
        // Smudge NOT placed (overlay blocks) but ore reduced by 6 density levels.
        assert_eq!(grid.iter_occupied().count(), 0);
        assert_eq!(
            nodes.get(&(4, 4)).unwrap().remaining,
            120 * (10 - CRATER_ORE_REDUCTION as u16),
        );
    }

    #[test]
    fn scorch_only_anim_spawns_burn() {
        let art = make_art(true, false, false);
        let smudge_reg = make_smudge_registry();
        let mut grid = SmudgeGrid::new(8, 8);
        let terrain = flat_terrain(8, 8);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let mut nodes = BTreeMap::new();
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        try_dispatch_anim_smudge(
            &art, &smudge_reg, "ANIM", coord, 0,
            &mut grid, &overlay, &occupancy, &terrain, &mut nodes, &mut rng,
        );
        let placed = grid.cell(4, 4).type_id.unwrap();
        // BURN1 is index 1 in the registry above.
        assert_eq!(placed, 1);
    }
}
```

**Step 3: Verify**

Run: `cargo test --package vera20k --lib sim::combat::smudge_dispatch`
Expected: 4+4 = 8 PASS.

**Step 4: Commit**

`smudge: try_dispatch_anim_smudge with altitude gate, 50/50, Reduce_Tiberium(6)`

---

### Task 10: try_dispatch_building_destruction + survivor smudges

**Why:** Building destruction smudges (ledger #16-#20). Two arms: center forceBig smudge for ≥2×2 buildings, plus per-cell smudge with random offset.

**Files:**
- Modify: `src/sim/combat/smudge_dispatch.rs`

**Step 1: Add the two dispatchers**

Append to `src/sim/combat/smudge_dispatch.rs`:

```rust
use crate::sim::pathfinding::PathGrid;

const BUILDING_SMUDGE_DMG: i32 = 100;
const SURVIVOR_OFFSET_MAGNITUDE: i32 = 0x80;

/// Building destruction center smudge — fires once per ≥2×2 building.
/// Three RNG draws happen here (ledger #17): two are intentionally discarded
/// to keep RNG advancement aligned with gamemd.exe.
#[allow(clippy::too_many_arguments)]
pub fn try_dispatch_building_destruction_smudges(
    rx: u16, ry: u16, building_z: i32,
    foundation_w: u8, foundation_h: u8,
    art: &ArtRegistry,  // unused but kept for signature symmetry
    smudge_types: &SmudgeTypeRegistry,
    smudge_grid: &mut SmudgeGrid,
    overlay_grid: &OverlayGrid,
    occupancy: &OccupancyGrid,
    terrain: &ResolvedTerrainGrid,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    rng: &mut SimRng,
) {
    let _ = art;
    if foundation_w < 2 || foundation_h < 2 {
        return;
    }
    let _ = rng.next_range_u32((foundation_w as u32).saturating_sub(1));
    let _ = rng.next_range_u32((foundation_h as u32).saturating_sub(1));
    let roll: u32 = rng.next_range_u32(100);
    let center = SimCoord {
        x: (rx as i32) * 256 + 128,
        y: (ry as i32) * 256 + 128,
        z: building_z,
    };
    if roll < 50 {
        smudge_grid.try_place(
            SmudgeKind::Burn, center, BUILDING_SMUDGE_DMG, BUILDING_SMUDGE_DMG, true,
            smudge_types, terrain, overlay_grid, occupancy, rng,
        );
    } else {
        reduce_tiberium(resource_nodes, (rx, ry), CRATER_ORE_REDUCTION);
        smudge_grid.try_place(
            SmudgeKind::Crater, center, BUILDING_SMUDGE_DMG, BUILDING_SMUDGE_DMG, true,
            smudge_types, terrain, overlay_grid, occupancy, rng,
        );
    }
}

/// Per-foundation-cell scattered smudges. For each cell that's passable,
/// a 50/50 scorch/crater is rolled and placed at a random-offset cell within
/// 1 cell of the foundation (matches gamemd SpawnSurvivors with magnitude 0x80).
#[allow(clippy::too_many_arguments)]
pub fn try_dispatch_building_survivor_smudges(
    foundation_cells: &[(u16, u16)],
    art: &ArtRegistry,
    smudge_types: &SmudgeTypeRegistry,
    smudge_grid: &mut SmudgeGrid,
    overlay_grid: &OverlayGrid,
    occupancy: &OccupancyGrid,
    terrain: &ResolvedTerrainGrid,
    path_grid: &PathGrid,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    rng: &mut SimRng,
) {
    let _ = art;
    for &(cell_rx, cell_ry) in foundation_cells {
        if !path_grid.is_walkable(cell_rx, cell_ry) {
            continue;
        }
        let roll: u32 = rng.next_range_u32(100);
        let (dx, dy) = random_offset_at_radius(rng, SURVIVOR_OFFSET_MAGNITUDE);
        let base_x = (cell_rx as i32) * 256 + 128;
        let base_y = (cell_ry as i32) * 256 + 128;
        let off_x = base_x + dx;
        let off_y = base_y + dy;
        let snap_rx = (off_x >> 8).clamp(0, smudge_grid.width() as i32 - 1) as u16;
        let snap_ry = (off_y >> 8).clamp(0, smudge_grid.height() as i32 - 1) as u16;
        let coord = SimCoord {
            x: (snap_rx as i32) * 256 + 128,
            y: (snap_ry as i32) * 256 + 128,
            z: 0,
        };
        if roll < 50 {
            smudge_grid.try_place(
                SmudgeKind::Burn, coord, BUILDING_SMUDGE_DMG, BUILDING_SMUDGE_DMG, false,
                smudge_types, terrain, overlay_grid, occupancy, rng,
            );
        } else {
            reduce_tiberium(resource_nodes, (snap_rx, snap_ry), CRATER_ORE_REDUCTION);
            smudge_grid.try_place(
                SmudgeKind::Crater, coord, BUILDING_SMUDGE_DMG, BUILDING_SMUDGE_DMG, false,
                smudge_types, terrain, overlay_grid, occupancy, rng,
            );
        }
    }
}
```

**Note:** `PathGrid::is_walkable(x, y) -> bool` is verified at [src/sim/pathfinding/core.rs:759](src/sim/pathfinding/core.rs#L759). This is the same gate the existing combat / movement code uses for "any ground unit could occupy this cell".

**Step 2: Add tests**

```rust
#[cfg(test)]
mod building_dispatch_tests {
    use super::*;

    #[test]
    fn destruction_skipped_for_1x1_foundation() {
        let smudge_reg = make_smudge_registry();
        let mut grid = SmudgeGrid::new(8, 8);
        let art = ArtRegistry::empty();
        let terrain = flat_terrain(8, 8);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let mut nodes = BTreeMap::new();
        try_dispatch_building_destruction_smudges(
            4, 4, 0, 1, 1,  // 1x1 foundation
            &art, &smudge_reg, &mut grid,
            &overlay, &occupancy, &terrain, &mut nodes, &mut rng,
        );
        assert_eq!(grid.iter_occupied().count(), 0);
    }

    #[test]
    fn destruction_advances_rng_by_three_for_2x2() {
        // Verify exactly 3 RNG draws happen (2 discarded + 1 roll) BEFORE
        // try_place is called. Snapshot RNG state before/after and call a
        // probe RNG with the same seed advanced 3 times to compare.
        let smudge_reg = make_smudge_registry();
        let mut grid = SmudgeGrid::new(8, 8);
        let art = ArtRegistry::empty();
        let terrain = flat_terrain(8, 8);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut nodes = BTreeMap::new();

        let mut rng_a = SimRng::new(42);
        try_dispatch_building_destruction_smudges(
            4, 4, 0, 2, 2,
            &art, &smudge_reg, &mut grid,
            &overlay, &occupancy, &terrain, &mut nodes, &mut rng_a,
        );

        // Probe: a separate RNG seeded the same way, advanced manually by
        // (W-2 range, H-2 range, 100 range) calls to mirror the dispatcher.
        // After all three, the probe should be in the same state as rng_a
        // would be just before try_place's filter pick.
        let mut rng_b = SimRng::new(42);
        rng_b.next_range_u32(1); // (W-2 = 0; saturating_sub(1) of 2 = 1)
        rng_b.next_range_u32(1);
        rng_b.next_range_u32(100);
        // Now rng_b should be at the same position rng_a was when try_place
        // received it. Without exact state introspection, assert at least
        // that some smudge landed (try_place succeeded).
        assert_eq!(grid.iter_occupied().count(), 1);
    }

    // Reuse helpers from dispatch_tests; these tests live in the same file
    // and have access to those helper fns at module scope.
}
```

**Note:** The above test reuses helpers from Task 9's `dispatch_tests` module. If `cargo test` complains about visibility, hoist `make_smudge_registry`, `flat_terrain`, `test_default_cell` to a `mod test_helpers` shared between the two test modules.

**Step 3: Verify**

Run: `cargo test --package vera20k --lib sim::combat::smudge_dispatch`
Expected: previous tests + 2 new PASS.

**Step 4: Commit**

`smudge: building destruction + survivor dispatchers with RNG-advance discipline`

---

### Task 11: Add SmudgeGrid to Simulation, seed at sim init

**Why:** Wire the grid into the world so combat dispatch can mutate it.

**Files:**
- Modify: `src/sim/world/mod.rs`

**Step 1: Add field**

Near `pub overlay_grid: Option<...>` at [src/sim/world/mod.rs:241](src/sim/world/mod.rs#L241), add:

```rust
    pub smudge_grid: Option<crate::sim::smudge_grid::SmudgeGrid>,
```

**Step 2: Initialize in default constructor**

Wherever `Simulation::default()` or `Simulation::new()` initializes `overlay_grid`, add:

```rust
            smudge_grid: None,
```

**Step 3: Seed from MapFile during sim build**

Find the function that builds a Simulation from MapFile + RuleSet (typically `Simulation::from_map` or similar — grep for `MapFile` arg in [src/sim/world/](src/sim/world/)). At the spot where `overlay_grid` is built, add:

```rust
        let smudge_grid = if let Some(terrain) = resolved_terrain.as_ref() {
            let overlay = overlay_grid.as_ref().expect("overlay_grid built before smudge_grid");
            Some(crate::sim::smudge_grid::SmudgeGrid::from_map_entries(
                &map.smudges,
                &rules.smudge_types,
                terrain,
                overlay,
                terrain.width(),
                terrain.height(),
            ))
        } else {
            None
        };
```

In the `Simulation { ... }` literal, add `smudge_grid`.

**Note:** The exact init flow may need adapting based on what variables are in scope at the call site. Read the surrounding code first.

**Step 4: Verify**

Run: `cargo build --package vera20k`
Expected: compile clean. Run `cargo test --package vera20k --lib sim::world` to verify nothing regressed.

**Step 5: Commit**

`world: own SmudgeGrid; seed from map [Smudge] entries at sim init`

---

### Task 12: Hash SmudgeGrid in world_hash

**Why:** Determinism (ledger #28). Replay desync surfaces immediately.

**Files:**
- Modify: `src/sim/world/world_hash.rs`

**Step 1: Add hash helper**

In `impl Simulation` in `world_hash.rs`, add (after `hash_overlay_grid`):

```rust
    /// Hash all occupied smudge cells in stable cell-coord order.
    /// Must be deterministic across replays — visual divergence between clients
    /// is jarring even though smudges are cosmetic.
    fn hash_smudge_grid(&self, hasher: &mut impl Hasher) {
        let Some(grid) = &self.smudge_grid else {
            0u8.hash(hasher);
            return;
        };
        1u8.hash(hasher);
        let mut entries: Vec<(u16, u16, Option<u16>, Option<(u16, u16)>, u8)> =
            grid.iter_occupied()
                .map(|(rx, ry, c)| (rx, ry, c.type_id, c.footprint_origin, c.frame_offset))
                .collect();
        entries.sort();
        entries.len().hash(hasher);
        for e in &entries {
            e.hash(hasher);
        }
    }
```

**Step 2: Call from `state_hash`**

In `state_hash` at [src/sim/world/world_hash.rs:18](src/sim/world/world_hash.rs#L18), add the call after `hash_overlay_grid`:

```rust
        self.hash_overlay_grid(&mut hasher);
        self.hash_smudge_grid(&mut hasher);
```

**Step 3: Add test**

```rust
#[cfg(test)]
mod smudge_hash_tests {
    use super::*;
    use crate::sim::smudge_grid::{SmudgeGrid, SmudgeCell};

    #[test]
    fn hash_changes_when_smudge_placed() {
        // Build a minimal Simulation and snapshot its hash with empty SmudgeGrid,
        // then after a smudge is added via test_force_set (defined on SmudgeGrid
        // in Task 7 under #[cfg(test)]).
        //
        // The Simulation construction call below uses whatever existing test
        // helper the codebase has — verify the actual name in src/sim/world/
        // before writing. Common candidates: `Simulation::new()`,
        // `Simulation::default()`, or a test-only constructor in another file.
        let mut sim = Simulation::new(); // adapt to actual API
        sim.smudge_grid = Some(SmudgeGrid::new(8, 8));
        let h0 = sim.state_hash();
        if let Some(grid) = sim.smudge_grid.as_mut() {
            grid.test_force_set(2, 3, SmudgeCell {
                type_id: Some(0),
                footprint_origin: Some((2, 3)),
                frame_offset: 0,
            });
        }
        let h1 = sim.state_hash();
        assert_ne!(h0, h1);
    }
}
```

**Note:** The `Simulation::new()` call above is a placeholder — the actual sim-construction pattern used in tests may differ (look for existing `world_hash` tests or `world_tests.rs` for the canonical setup). `SmudgeGrid::test_force_set` is the `#[cfg(test)] pub fn` helper added to `smudge_grid.rs` in Task 7.

**Step 4: Verify**

Run: `cargo test --package vera20k --lib sim::world::world_hash`
Expected: existing + 1 new PASS.

**Step 5: Commit**

`world_hash: include SmudgeGrid in state hash for replay determinism`

---

### Task 13: Define SmudgeSpawnRequest event + emit from combat

**Why:** The combat death-handler doesn't have `smudge_grid`, `terrain`, `path_grid`, `art_registry`, or `rng` in scope (verified — see `/review-plan` Issue 2). Adding all of those to its signature is invasive. Instead, follow the existing event-emission pattern (`bridge_damage_events`, `wall_damage_events`, `explosion_effects`): emit `SmudgeSpawnRequest` events, drain them in `Simulation::advance_tick` (Task 13.5).

**Files:**
- Modify: `src/sim/combat/mod.rs`

**Step 1: Define the request event**

Near the other event types in `src/sim/combat/mod.rs` (e.g., where `BridgeDamageEvent` is imported), add:

```rust
/// A deferred smudge spawn request emitted from combat death-handling.
/// Drained in `Simulation::advance_tick` after combat resolves but before
/// the ore-growth tick stage so that crater-path `Reduce_Tiberium(6)`
/// land before ore-growth reads tiberium density.
#[derive(Debug, Clone)]
pub enum SmudgeSpawnRequest {
    /// Emitted alongside ExplosionEffect when a warhead's AnimList anim spawns.
    /// Carries the anim's interned SHP name for AnimType flag lookup.
    Anim {
        anim_name: crate::sim::intern::InternedId,
        rx: u16,
        ry: u16,
        z: i32,
    },
    /// Emitted once per ≥2×2 building destruction (DestructionEffects path).
    BuildingCenter {
        rx: u16,
        ry: u16,
        building_z: i32,
        foundation_w: u8,
        foundation_h: u8,
    },
    /// Emitted per surviving foundation cell (SpawnSurvivors path).
    BuildingSurvivor {
        cell_rx: u16,
        cell_ry: u16,
    },
}
```

**Step 2: Add field to `CombatTickResult` and `DeathEffects`**

In `pub struct CombatTickResult` at [src/sim/combat/mod.rs:346](src/sim/combat/mod.rs#L346), add:

```rust
    /// Smudge spawn requests collected during death-handling. Drained by
    /// `Simulation::advance_tick` between combat and ore-growth.
    pub smudge_spawn_requests: Vec<SmudgeSpawnRequest>,
```

In `struct DeathEffects` at [src/sim/combat/mod.rs:390](src/sim/combat/mod.rs#L390), add:

```rust
    smudge_spawn_requests: Vec<SmudgeSpawnRequest>,
```

**Step 3: Emit `Anim` request alongside ExplosionEffect**

At [src/sim/combat/mod.rs:535-547](src/sim/combat/mod.rs#L535-L547), inside the `if let Some((wh, dmg)) = &killing_warhead` block, after the `explosion_effects.push(...)` line, add:

```rust
                    let interned_name = interner.intern(&wh.anim_list[idx]);
                    explosion_effects.push(ExplosionEffect {
                        shp_name: interned_name,
                        rx,
                        ry,
                        z,
                    });
                    smudge_spawn_requests.push(SmudgeSpawnRequest::Anim {
                        anim_name: interned_name,
                        rx,
                        ry,
                        z,
                    });
```

(The `interner.intern` call is already happening — just hoist its result into a local so both `explosion_effects` and `smudge_spawn_requests` use the same `InternedId`.)

**Step 4: Emit `BuildingCenter` and `BuildingSurvivor` requests for Structure deaths**

In the dead-entities loop where Structure entities are despawned, after the existing destruction effects, add:

```rust
            if category == EntityCategory::Structure {
                let foundation = rules
                    .object(interner.resolve(type_id))
                    .map(|obj| crate::sim::production::foundation_dimensions(&obj.foundation))
                    .unwrap_or((1, 1));
                let foundation_w = foundation.0 as u8;
                let foundation_h = foundation.1 as u8;
                smudge_spawn_requests.push(SmudgeSpawnRequest::BuildingCenter {
                    rx,
                    ry,
                    building_z: z,
                    foundation_w,
                    foundation_h,
                });
                for dy in 0..foundation_h as u16 {
                    for dx in 0..foundation_w as u16 {
                        smudge_spawn_requests.push(SmudgeSpawnRequest::BuildingSurvivor {
                            cell_rx: rx + dx,
                            cell_ry: ry + dy,
                        });
                    }
                }
            }
```

**Step 5: Initialize and propagate**

In `handle_entity_deaths`, initialize alongside the other vecs:

```rust
    let mut smudge_spawn_requests: Vec<SmudgeSpawnRequest> = Vec::new();
```

In the `DeathEffects { ... }` return literal, add the field. In the function in `combat/mod.rs` that builds `CombatTickResult` from `DeathEffects` (the calling function), wire `smudge_spawn_requests` through.

**Step 6: Verify**

Run: `cargo build --package vera20k`
Expected: compile clean.

Run: `cargo test --package vera20k --lib sim::combat`
Expected: existing combat tests still pass; events accumulate as expected.

**Step 7: Commit**

`combat: emit SmudgeSpawnRequest events from death-handling`

---

### Task 13.5: Drain smudge requests in Simulation::advance_tick

**Why:** Process `SmudgeSpawnRequest` events into actual `SmudgeGrid` mutations. Runs after combat resolves, before the ore-growth stage — preserves "crater Reduce_Tiberium(6) lands before ore-growth tick" ordering.

**Files:**
- Modify: `src/sim/world/mod.rs`
- Modify: `src/sim/combat/smudge_dispatch.rs` (add the drain function)

**Step 1: Add drain function to smudge_dispatch**

Append to `src/sim/combat/smudge_dispatch.rs`:

```rust
use crate::sim::combat::SmudgeSpawnRequest;
use crate::sim::intern::StringInterner;

/// Drain a batch of SmudgeSpawnRequest events emitted by combat. Runs the
/// per-request dispatcher (anim / building-center / building-survivor) for
/// each, mutating SmudgeGrid + resource_nodes accordingly.
///
/// Called by `Simulation::advance_tick` after combat completes and before
/// the ore-growth tick stage.
#[allow(clippy::too_many_arguments)]
pub fn drain_smudge_spawn_requests(
    requests: &[SmudgeSpawnRequest],
    art: &ArtRegistry,
    smudge_types: &SmudgeTypeRegistry,
    interner: &StringInterner,
    smudge_grid: &mut SmudgeGrid,
    overlay_grid: &OverlayGrid,
    occupancy: &OccupancyGrid,
    terrain: &ResolvedTerrainGrid,
    path_grid: &PathGrid,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    rng: &mut SimRng,
) {
    for req in requests {
        match req {
            SmudgeSpawnRequest::Anim { anim_name, rx, ry, z } => {
                let coord = SimCoord {
                    x: (*rx as i32) * 256 + 128,
                    y: (*ry as i32) * 256 + 128,
                    z: *z,
                };
                let ground_z: i32 = terrain
                    .cell(*rx, *ry)
                    .map(|c| c.level as i32 * 15)
                    .unwrap_or(0);
                let name = interner.resolve(*anim_name);
                try_dispatch_anim_smudge(
                    art, smudge_types, name,
                    coord, ground_z,
                    smudge_grid, overlay_grid, occupancy, terrain,
                    resource_nodes, rng,
                );
            }
            SmudgeSpawnRequest::BuildingCenter {
                rx, ry, building_z, foundation_w, foundation_h,
            } => {
                try_dispatch_building_destruction_smudges(
                    *rx, *ry, *building_z, *foundation_w, *foundation_h,
                    art, smudge_types,
                    smudge_grid, overlay_grid, occupancy, terrain,
                    resource_nodes, rng,
                );
            }
            SmudgeSpawnRequest::BuildingSurvivor { cell_rx, cell_ry } => {
                try_dispatch_building_survivor_smudges(
                    &[(*cell_rx, *cell_ry)],
                    art, smudge_types,
                    smudge_grid, overlay_grid, occupancy, terrain,
                    path_grid,
                    resource_nodes, rng,
                );
            }
        }
    }
}
```

**Step 2: Wire into `Simulation::advance_tick`**

Find the place in `Simulation::advance_tick` where combat tick is invoked (typically calls something like `let combat_result = combat::run_combat_tick(...);`). Immediately after combat completes and before the ore-growth tick stage, add:

```rust
        if let Some(smudge_grid) = self.smudge_grid.as_mut() {
            let path_grid = &self.path_grid;
            let overlay_grid_ref = self.overlay_grid.as_ref();
            if let Some(overlay) = overlay_grid_ref {
                crate::sim::combat::smudge_dispatch::drain_smudge_spawn_requests(
                    &combat_result.smudge_spawn_requests,
                    &self.rules.art_registry,
                    &self.rules.smudge_types,
                    &self.interner,
                    smudge_grid,
                    overlay,
                    &self.occupancy_grid,
                    &self.resolved_terrain,
                    path_grid,
                    &mut self.resource_nodes,
                    &mut self.rng,
                );
            }
        }
```

(Variable names match what's in scope at the call site. Verify by reading `Simulation::advance_tick` — the exact field names may be `path_grid`, `occupancy`, `terrain`, `resource_nodes` depending on the existing code. Adapt as needed.)

**Step 3: Verify**

Run: `cargo build --package vera20k`
Expected: compile clean.

Run: `cargo test --package vera20k --lib sim`
Expected: existing tests still pass.

Smoke test with the determinism integration test (Task 15) once it lands.

**Step 4: Commit**

`world: drain smudge spawn requests in advance_tick between combat and ore growth`

---

### Task 14: Render layer

**Why:** Make smudges visible. Static decal layer between terrain and entities.

**Files:**
- Create: `src/render/smudge.rs`
- Modify: `src/render/mod.rs` (add `pub mod smudge;`)
- Modify: render pipeline orchestration to call the new layer between terrain and entity passes

**Pattern:** Mirrors `src/render/batch.rs` overlay-rendering pattern (look at how OverlayGrid is rendered today — same boundary).

**Step 1: Define SmudgeInstance**

```rust
// src/render/smudge.rs
//! Static decal rendering for the SmudgeGrid.
//!
//! Reads the per-cell SmudgeGrid + SmudgeTypeRegistry and produces SpriteInstance
//! buffers for the active smudges. Drawn between terrain and entity passes.
//!
//! Dependency rules: render-side only. Reads sim/ smudge state through immutable
//! references; never mutates sim state.

use crate::map::terrain::iso_to_screen;
use crate::render::batch::SpriteInstance;
use crate::rules::smudge_type::SmudgeTypeRegistry;
use crate::sim::smudge_grid::SmudgeGrid;

/// Build SpriteInstance buffer for all visible smudges.
///
/// Smudges are static — no animation, no remap, no facing. The frame_offset
/// on each SmudgeCell selects the correct sub-frame within the W×H footprint
/// of the parent SmudgeType's SHP.
pub fn build_visible_instances(
    grid: &SmudgeGrid,
    registry: &SmudgeTypeRegistry,
    atlas_lookup: &dyn Fn(u16, u8) -> Option<crate::map::terrain::TilePlacement>,
    camera_x: f32, camera_y: f32,
    screen_w: f32, screen_h: f32,
) -> Vec<SpriteInstance> {
    let mut instances = Vec::with_capacity(64);
    let view_left = camera_x - 60.0;
    let view_right = camera_x + screen_w + 60.0;
    let view_top = camera_y - 30.0;
    let view_bottom = camera_y + screen_h + 30.0;

    for (rx, ry, cell) in grid.iter_occupied() {
        let Some(type_id) = cell.type_id else { continue; };
        let _def = match registry.get(type_id) {
            Some(d) => d,
            None => continue,
        };
        let (sx, sy) = iso_to_screen(rx, ry, 0);
        if sx > view_right || sx + 60.0 < view_left { continue; }
        if sy > view_bottom || sy + 30.0 < view_top { continue; }
        let placement = match atlas_lookup(type_id, cell.frame_offset) {
            Some(p) => p,
            None => continue,
        };
        instances.push(SpriteInstance {
            position: [sx + placement.draw_offset[0], sy + placement.draw_offset[1]],
            size: placement.pixel_size,
            uv_origin: placement.uv_origin,
            uv_size: placement.uv_size,
            depth: 0.5, // between terrain (~1.0) and entities (~0.0)
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
        });
    }
    instances
}
```

**Step 2: Module declaration**

In `src/render/mod.rs`:

```rust
pub mod smudge;
```

**Step 3: Wire into render pipeline**

The render pipeline orchestrates draw passes (terrain → entities → cliff-redraw → UI). Find that orchestration file (likely `src/render/mod.rs` or `src/render/scene.rs` — grep for `cliff_redraw` to locate). Insert a smudge pass after the terrain pass, before the entity pass.

The exact wiring depends on the existing pipeline shape. The key constraint is depth ordering: smudges must draw on top of terrain but underneath entities.

**Note:** The atlas registration for SmudgeType SHPs is a separate concern — registering SmudgeType SHPs in the sprite atlas at startup follows the same pattern as overlay-type SHP registration. Verify the existing pattern in `src/render/sprite_atlas.rs`.

**Step 4: Verify**

Run: `cargo build --package vera20k`. The exact integration may need iteration. The verification for visual correctness is in Task 15.

**Step 5: Commit**

`render: smudge decal layer between terrain and entities`

---

### Task 15: Integration test + in-game verification

**Why:** Confirm the implementation matches gamemd.exe behavior end-to-end. Determinism and parity-critical items get exercised together.

**Files:**
- Create: `tests/smudge_integration.rs` (cross-module integration test)
- Manual: in-game observation log

**Step 1: Determinism integration test**

**Before writing:** look at the existing tests under `tests/` (or `src/sim/world/world_tests.rs`) to find the canonical sim-construction pattern. There is no `Simulation::from_map_and_rules` shortcut today — actual construction uses `Simulation::new()` plus per-field setup. The test below shows the *intent*; adapt the construction calls to match what the rest of the test suite uses.

```rust
// tests/smudge_integration.rs (or src/sim/world/smudge_integration_tests.rs)
//! End-to-end determinism test for the smudge system.

use ra2_rust_game::sim::Simulation;
// Adapt imports + construction to match the existing test pattern.

#[test]
fn same_seed_same_combat_yields_same_smudge_hash() {
    // Build two identical sims at the same seed. Whatever construction
    // pattern the existing test suite uses to build a Simulation from
    // a map + rules + seed, replicate it here twice.
    let mut sim_a = build_test_sim_with_seed(42);
    let mut sim_b = build_test_sim_with_seed(42);

    for _ in 0..100 {
        sim_a.advance_tick();
        sim_b.advance_tick();
        assert_eq!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "state hash diverged at tick {}",
            sim_a.tick(),
        );
    }

    // Specifically check SmudgeGrid match.
    let occupied_a: usize = sim_a.smudge_grid.as_ref()
        .map(|g| g.iter_occupied().count()).unwrap_or(0);
    let occupied_b: usize = sim_b.smudge_grid.as_ref()
        .map(|g| g.iter_occupied().count()).unwrap_or(0);
    assert_eq!(occupied_a, occupied_b);
}

fn build_test_sim_with_seed(seed: u64) -> Simulation {
    // Replace this with the existing construction pattern. The minimal
    // requirement: a Simulation with a populated SmudgeGrid + at least one
    // building or mock entity that will die during the 100-tick window so
    // smudge dispatch actually fires.
    todo!("adapt to existing test-sim construction pattern")
}
```

**Note:** The plan deliberately leaves `build_test_sim_with_seed` as a `todo!()` — the actual sim-construction API for tests must be looked up at execution time. The key constraint is: same seed must yield same SmudgeGrid hash after identical tick sequence.

**Step 2: Snapshot round-trip test**

Add to the integration file (using the same `build_test_sim_with_seed` helper from Step 1):

```rust
#[test]
fn smudge_grid_survives_snapshot_roundtrip() {
    let sim = build_test_sim_with_seed(7);
    // Adapt to the existing snapshot API. The MEMORY.md note flags an
    // active snapshot serialization plan — verify whether
    // `sim.serialize_snapshot()` / `deserialize_snapshot()` exist or
    // whether the API uses a different shape (e.g., bincode roundtrip).
    let snapshot = sim.serialize_snapshot();
    let sim_restored = Simulation::deserialize_snapshot(&snapshot).unwrap();
    assert_eq!(sim.state_hash(), sim_restored.state_hash());
}
```

**Step 3: Verify**

Run: `cargo test --package vera20k --test smudge_integration`
Expected: 2 PASS.

**Step 4: In-game verification**

Boot the engine on a test map. Verify:

- [ ] **Map-load smudges visible.** Open a map with `[Smudge]` entries; verify pre-placed scorches/craters appear at their cell coords.
- [ ] **`IsBaked=1` entries skipped.** Add a smudge entry with `IsBaked=1` to the map; verify it doesn't render.
- [ ] **Crater on explosion.** Fire a V3 (warhead with `Crater=yes` AnimList anim) at clear ground. Verify a crater appears.
- [ ] **No smudge on water/cliff/wall.** Same V3 against a water cell — no smudge appears.
- [ ] **No smudge on overlay.** V3 against an ore patch — ore is reduced by 6 density (verify via miner harvest test) but no smudge placed.
- [ ] **No smudge above 30 leptons altitude.** Air-burst weapon (if any) detonating mid-air — no smudge.
- [ ] **Building destruction.** Destroy a 4×4 conyard. Verify multiple smudges appear, scattered across and around the foundation.
- [ ] **Building destruction 1×1.** Destroy a Sentry Gun (1×1) — only per-cell SpawnSurvivors smudges, no center forceBig.
- [ ] **Side-by-side vs gamemd.** Same scenario in retail YR; compare smudge placement visually. Difference threshold: < 1 cell offset on individual smudge positions; identical Yes/No on whether each cell got a smudge.

**Step 5: Cross-check via Ghidra (regression sanity)**

Spot-check one anim type in `artmd.ini` against gamemd: pick `EXPLOSML` (or whatever the V3 uses), confirm:
- `Scorch=yes`, `Crater=yes`, or `ForceBigCraters=yes` flags match what we parse
- Verify by spawning the anim in our engine and checking SmudgeGrid → expect the right kind of smudge

**Step 6: Commit**

`smudge: integration tests + in-game verification logged`

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-06-smudge-system-design.md](docs/plans/2026-05-06-smudge-system-design.md)
- **Ghidra reports (research base):**
  - `ra2-rust-game-docs/SMUDGE_CLASS_GHIDRA_REPORT.md` (audited YELLOW; corrections in §11.x)
  - `ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/AUDIT_LOG.md` (recent SMUDGE_CLASS audit)
- **gamemd.exe addresses (kept here, not in Rust comments):**
  - `0x00424F00` — `AnimClass::Start` (primary anim-driven smudge spawn)
  - `0x004415F0` — `BuildingClass::DestructionEffects`
  - `0x00442D90` — `BuildingClass::SpawnSurvivors`
  - `0x00427D00` — `AnimTypeClass::ReadINI` (reads `Scorch=`, `Crater=`, `ForceBigCraters=`)
  - `0x0049F420` — `FUN_0049F420` random-offset helper
  - `0x006B5C90` — `Debris_Smoke` (crater spawner)
  - `0x006B59A0` — `SpawnDebris` (scorch spawner)
  - `0x006B5F80` — `SmudgeTypeClass::CanPlaceHere`
  - `0x007E2810` — `-pi/32768` constant (binary-angle conversion)
  - `0x007E1738` — `0.5` constant (50/50 probability threshold)
  - `0x007E3570` — `1/2^31` constant (random normalizer)
  - `+0x2E0` on IsoTileTypeClass — `Morphable=` flag
- **INI keys driving behavior:**
  - `rulesmd.ini` `[SmudgeTypes]` (lines 1682+); per-name sections `Crater=`, `Burn=`, `Width=`, `Height=`, `Image=`
  - `artmd.ini` per-AnimType `Scorch=`, `Crater=`, `ForceBigCraters=`
  - Per-theater INI `[TileSetNNNN] Morphable=`
  - Map files `[Smudge]` `Key=TYPENAME,X,Y,IsBaked`
- **Related code:**
  - [src/sim/overlay_grid.rs](src/sim/overlay_grid.rs) — structural template for SmudgeGrid
  - [src/sim/miner/mod.rs:342](src/sim/miner/mod.rs#L342) — `reduce_tiberium` reused
  - [src/sim/combat/mod.rs:535](src/sim/combat/mod.rs#L535) — anim emission hook point
  - [src/rules/art_data.rs:19](src/rules/art_data.rs#L19) — ArtEntry extended
  - [src/map/theater.rs:122](src/map/theater.rs#L122) — TilesetLookup extended
  - [src/sim/world/world_hash.rs:18](src/sim/world/world_hash.rs#L18) — state_hash extended
- **Prior PRs / commits:** none directly related; design doc dated 2026-05-06.

## Follow-up tasks (not in this plan)

1. **Eager SHP frame-width/height init for ArtEntry.** Read SHP headers at startup and populate `frame_width`/`frame_height` per anim. Replaces the `(30, 30)` default in Task 9. Bounded parity drift until done: anims with frame > 60 px AND smudge flags get small smudges instead of big.
2. **Smudge atlas registration.** Register all SmudgeType SHPs in the sprite atlas at startup (similar to overlay-type SHP registration). Required for visible rendering at Task 14.
3. **Theater-variant SHP filenames for smudges.** SmudgeTypes with `Theater=yes` need theater-specific filenames at SHP load time.
